use crossbeam_channel::{Receiver, Sender};
use ffmpeg_next::{format, frame};
use std::{
    path::PathBuf,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    time::{Duration, Instant},
};

use crate::{thread_utils::custom_beams::LooseSender, video::PlaySpeed};

use super::{InitData, MP4Command, VideoUpdateInfo};

#[derive(Debug)]
pub enum PipelineEvent<T> {
    Data(T),
    EOS,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Resolution {
    pub width: usize,
    pub height: usize,
}

#[derive(Clone, Debug)]
pub struct YuvPlanes {
    pub y_plane: bytes::Bytes,
    pub u_plane: bytes::Bytes,
    pub v_plane: bytes::Bytes,
}

#[derive(Debug, Clone)]
pub enum FrameData {
    PlanarYuv420(YuvPlanes),
}

#[derive(Debug, Clone)]
pub struct Frame {
    pub data: FrameData,
    pub resolution: Resolution,
    #[allow(unused)]
    pub pts: Duration,
}

#[derive(Debug, thiserror::Error)]
pub enum InputInitError {
    #[error(transparent)]
    FfmpegError(#[from] ffmpeg_next::Error),
}

#[derive(Debug, thiserror::Error)]
enum DecoderFrameConversionError {
    #[error("Unsupported pixel format: {0:?}")]
    UnsupportedPixelFormat(format::pixel::Pixel),
}

fn copy_plane_from_av(decoded: &frame::Video, plane: usize) -> bytes::Bytes {
    let mut output_buffer = bytes::BytesMut::with_capacity(
        decoded.plane_width(plane) as usize * decoded.plane_height(plane) as usize,
    );

    decoded
        .data(plane)
        .chunks(decoded.stride(plane))
        .map(|chunk| &chunk[..decoded.plane_width(plane) as usize])
        .for_each(|chunk| output_buffer.extend_from_slice(chunk));

    output_buffer.freeze()
}

fn frame_from_ffmpeg(decoded: &mut frame::Video) -> Result<Frame, DecoderFrameConversionError> {
    let data = match decoded.format() {
        format::Pixel::YUV420P => FrameData::PlanarYuv420(YuvPlanes {
            y_plane: copy_plane_from_av(decoded, 0),
            u_plane: copy_plane_from_av(decoded, 1),
            v_plane: copy_plane_from_av(decoded, 2),
        }),
        fmt => return Err(DecoderFrameConversionError::UnsupportedPixelFormat(fmt)),
    };
    Ok(Frame {
        data,
        resolution: Resolution {
            width: decoded.width().try_into().unwrap(),
            height: decoded.height().try_into().unwrap(),
        },
        pts: Duration::from_millis(0),
    })
}

pub fn run_decoder_thread(
    file: PathBuf,
    init_result_sender: Sender<Result<InitData, InputInitError>>,
    frame_sender: LooseSender<PipelineEvent<Frame>>,
    close_thread: Arc<AtomicBool>,
    command_receiver: Receiver<MP4Command>,
    update_sender: LooseSender<VideoUpdateInfo>,
) {
    if let Ok(mut ictx) = ffmpeg_next::format::input(&file) {
        let (
            time_base_as_f64,
            total_duration,
            params,
            video_stream_index,
            avg_frame_rate,
            start_time,
            duration,
        ) = {
            let input = ictx
                .streams()
                .best(ffmpeg_next::media::Type::Video)
                .ok_or(ffmpeg_next::Error::StreamNotFound)
                .expect("No stream found");
            let video_stream_index = input.index();

            let time_sabe = input.time_base();
            let time_base_as_f64: f64 = time_sabe.into();
            let time_base_den = time_sabe.denominator() as f64 / time_sabe.numerator() as f64;

            let start_time = input.start_time();

            let duration = input.duration() as f64 / time_base_den;

            let params = input.parameters();

            let avg_frame_rate = input.avg_frame_rate();

            (
                time_base_as_f64,
                input.duration(),
                params,
                video_stream_index,
                avg_frame_rate,
                start_time,
                duration,
            )
        };

        init_result_sender
            .send(Ok(InitData {
                fps: avg_frame_rate.into(),
                total_duration: duration,
            }))
            .unwrap();

        println!("Start Time {}", start_time);

        let context_decoder = ffmpeg_next::codec::context::Context::from_parameters(params.clone())
            .expect("Failed to build context");

        let mut decoder = context_decoder
            .decoder()
            .video()
            .expect("Failed to create video decoder");

        let should_stop = || close_thread.load(Ordering::Relaxed);

        loop {
            puffin::profile_scope!("Video Packet Processing");

            if should_stop() {
                return;
            }

            let mut iter = ictx.packets();

            let speed_scalar = 1.0;

            let mut seek_to: Option<i64> = None;
            let mut seek_target: Option<f64> = None;

            let speed = PlaySpeed::Normal;

            let mut last = Instant::now();

            if let Some((stream, packet)) = iter.next() {
                if stream.index() == video_stream_index {
                    decoder
                        .send_packet(&packet)
                        .expect("Could not send the packet for some reason");

                    let mut decoded = ffmpeg_next::util::frame::video::Video::empty();

                    while decoder.receive_frame(&mut decoded).is_ok() {
                        puffin::profile_scope!("Frame Receive");
                        if should_stop() {
                            break;
                        }

                        let frame = match frame_from_ffmpeg(&mut decoded) {
                            Ok(frame) => frame,
                            Err(_) => {
                                continue;
                            }
                        };

                        let time_sabe = stream.time_base();
                        let time_base_den =
                            time_sabe.denominator() as f64 / time_sabe.numerator() as f64;

                        // println!(
                        //     "PTS {} Dur {:.2}",
                        //     decoded.pts().unwrap(),
                        //     decoded.pts().unwrap() as f64 / time_base_den
                        // );

                        drop(update_sender.loosely_send(VideoUpdateInfo::Frame(
                            decoded.pts().unwrap() as f64 / time_base_den,
                        )));

                        if frame_sender.loosely_send(PipelineEvent::Data(frame)).is_err() {
                            return;
                        }

                        let frame_duration = decoded.packet().duration;

                        let frame_duration_ms = time_base_den as f64 / frame_duration as f64;

                        std::thread::sleep(Duration::from_secs_f64(frame_duration_ms / 1000.0));

                        if let Ok(command) = command_receiver.try_recv() {
                            match command {
                                MP4Command::Seek(value) => {
                                    let dur = stream.duration();
                                    let num = stream.frames();
                                    let fps: f64 = stream.avg_frame_rate().into();
                                    let time_base = stream.time_base();
                                    let start_time = stream.start_time();

                                    // println!("\t duration: {}", dur);
                                    // println!("\t num: {}", num);
                                    // println!("\t fps: {}", fps);
                                    // println!("\t time_base: {}", time_base);
                                    // println!("\t start_time: {}", start_time);

                                    let target = (num as f64 / 2.0).ceil() as i64;
                                    let target_sec = target as f64 / fps;
                                    let target_timestamp =
                                        (target_sec / f64::from(time_base)) as i64 + start_time;

                                    // println!("\t target: {}", target);
                                    // println!("\t target_sec: {}", target_sec);
                                    // println!("\t target_timestamp: {}", target_timestamp);

                                    seek_to = Some(
                                        (stream.duration() as f64 * 62.29626645645534 * value)
                                            .round() as i64,
                                    );

                                    decoder.flush();
                                    break;
                                }
                                _ => {}
                            }
                        }

                        // frame_idx += 1;
                        // if frame_idx >= 100 {
                        //     let seek_to = 6000_0000;
                        //     frame_idx = 0;
                        //     println!("Seeking to {seek_to}");
                        //     ictx.seek(seek_to, ..seek_to).unwrap();

                        //     break;
                        // }
                    }

                    if let Some(seek_to) = seek_to.take() {
                        println!("Seeking to {seek_to}");
                        ictx.seek(seek_to, (seek_to - 1000000)..(seek_to + 1000000))
                            .unwrap();
                    }

                    if should_stop() {
                        break;
                    }
                }
            } else {
                break;
            }
        }

        decoder.send_eof().expect("EOF sending failed");
    }

    if frame_sender.send(PipelineEvent::EOS).is_err() {
        println!("Failed to send EOS from H264 decoder. Channel closed.")
    }
}
