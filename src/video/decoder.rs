use crossbeam_channel::Sender;
use ffmpeg_next::format::input;
use ffmpeg_next::{format, frame};
use std::{path::PathBuf, time::Duration};
use tracing::{debug, error};

pub enum MP4Command {
    // Pause (if possible)
    Pause,
    // Play (if possible)
    Play,
    // Pause + Go to star
    Stop,
    // Go +10 seconds
    SkipForward,
    // Go -10 seconds
    SkipBackward,
    // Go to a specific time (convert duration to sample_id)
    Seek(Duration),
}

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
    init_result_sender: Sender<Result<(), InputInitError>>,
    frame_sender: Sender<PipelineEvent<Frame>>,
) {
    if let Ok(mut ictx) = ffmpeg_next::format::input(&file) {
        let (params, video_stream_index, time_base, avg_frame_rate, start_time) = {
            let input = ictx
                .streams()
                .best(ffmpeg_next::media::Type::Video)
                .ok_or(ffmpeg_next::Error::StreamNotFound)
                .expect("No stream found");
            let video_stream_index = input.index();

            let start_time = input.start_time();

            let params = input.parameters();

            let time_base = i64::from(input.time_base().denominator());

            let avg_frame_rate = input.avg_frame_rate();

            (
                params,
                video_stream_index,
                time_base,
                avg_frame_rate,
                start_time,
            )
        };

        init_result_sender.send(Ok(())).unwrap();

        println!("Start Time {}", start_time);

        let context_decoder = ffmpeg_next::codec::context::Context::from_parameters(params.clone())
            .expect("Failed to build context");

        let mut decoder = context_decoder
            .decoder()
            .video()
            .expect("Failed to create video decoder");

        let mut frame_idx = 0;

        loop {
            let mut iter = ictx.packets();

            if let Some((stream, packet)) = iter.next() {
                if stream.index() == video_stream_index {
                    decoder
                        .send_packet(&packet)
                        .expect("Could not send the packet for some reason");

                    let mut decoded = ffmpeg_next::util::frame::video::Video::empty();

                    while decoder.receive_frame(&mut decoded).is_ok() {
                        let frame = match frame_from_ffmpeg(&mut decoded) {
                            Ok(frame) => frame,
                            Err(_) => {
                                continue;
                            }
                        };

                        if frame_sender.send(PipelineEvent::Data(frame)).is_err() {
                            return;
                        }

                        let time_sabe = stream.time_base();
                        let time_base_den =
                            time_sabe.denominator() as f64 / time_sabe.numerator() as f64;

                        let frame_duration = decoded.packet().duration;

                        let frame_duration_ms = time_base_den as f64 / frame_duration as f64;

                        std::thread::sleep(Duration::from_secs_f64(frame_duration_ms / 1000.0));
                        frame_idx += 1;
                        if frame_idx >= 100 {
                            let seek_to = 6000_0000;
                            frame_idx = 0;
                            println!("Seeking to {seek_to}");
                            ictx.seek(seek_to, ..seek_to).unwrap();

                            break;
                        }
                    }
                }
            } else {
                break;
            }
        }

        decoder.send_eof().expect("EOF sending failed");
    }

    if frame_sender.send(PipelineEvent::EOS).is_err() {
        debug!("Failed to send EOS from H264 decoder. Channel closed.")
    }
}
