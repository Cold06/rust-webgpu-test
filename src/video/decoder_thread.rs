use crossbeam_channel::{Receiver, Sender};
use ffmpeg_next::{format, frame, Rational};
use std::{
    path::PathBuf,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    time::Duration,
};

use super::{InitData, SeekCommand};

#[derive(Debug)]
pub enum PipelineEvent<T> {
    Data(T),
    SeekAck,
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

fn frame_from_ffmpeg(
    decoded: &mut frame::Video,
    time_base: Rational,
) -> Result<Frame, DecoderFrameConversionError> {
    let data = match decoded.format() {
        format::Pixel::YUV420P => FrameData::PlanarYuv420(YuvPlanes {
            y_plane: copy_plane_from_av(decoded, 0),
            u_plane: copy_plane_from_av(decoded, 1),
            v_plane: copy_plane_from_av(decoded, 2),
        }),
        fmt => return Err(DecoderFrameConversionError::UnsupportedPixelFormat(fmt)),
    };

    let pts = decoded.pts().unwrap();

    let pts: Duration = if pts != 0 {
        let pts_in_time_base = pts as f64 / time_base.denominator() as f64;
        Duration::from_secs_f64(pts_in_time_base)
    } else {
        Duration::from_secs_f64(0.0)
    };

    Ok(Frame {
        data,
        resolution: Resolution {
            width: decoded.width().try_into().unwrap(),
            height: decoded.height().try_into().unwrap(),
        },
        pts,
    })
}

pub fn run_decoder_thread(
    file: PathBuf,
    init_result_sender: Sender<Result<InitData, InputInitError>>,
    frame_sender: Sender<PipelineEvent<Frame>>,
    close_thread: Arc<AtomicBool>,
    command_receiver: Receiver<SeekCommand>,
) {
    if let Ok(mut ictx) = ffmpeg_next::format::input(&file) {
        let (params, video_stream_index, avg_frame_rate, total_duration) = {
            let input = ictx
                .streams()
                .best(ffmpeg_next::media::Type::Video)
                .ok_or(ffmpeg_next::Error::StreamNotFound)
                .expect("No stream found");

            let video_stream_index = input.index();

            let time_sabe = input.time_base();
            let time_base_den = time_sabe.denominator() as f64 / time_sabe.numerator() as f64;
            let total_duration = input.duration() as f64 / time_base_den;

            let params = input.parameters();

            let avg_frame_rate = input.avg_frame_rate();

            (params, video_stream_index, avg_frame_rate, total_duration)
        };

        init_result_sender
            .send(Ok(InitData {
                fps: avg_frame_rate.into(),
                total_duration: Duration::from_secs_f64(total_duration),
            }))
            .unwrap();

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

            let mut seek_to: Option<i64> = None;

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

                        let time_sabe = stream.time_base();

                        let frame = match frame_from_ffmpeg(&mut decoded, time_sabe) {
                            Ok(frame) => frame,
                            Err(_) => {
                                continue;
                            }
                        };

                        if frame_sender.send(PipelineEvent::Data(frame)).is_err() {
                            return;
                        }

                        if let Ok(command) = command_receiver.try_recv() {
                            match command {
                                SeekCommand::Seek(value) => {
                                    if frame_sender.send(PipelineEvent::SeekAck).is_err() {
                                        return;
                                    }

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
                    }

                    if let Some(seek_to) = seek_to.take() {
                        let seek_range = ..seek_to;
                        ictx.seek(seek_to, seek_range).unwrap();
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
