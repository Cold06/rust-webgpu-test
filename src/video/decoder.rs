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
    if let Ok(mut ictx) = input(&file) {
        let input = ictx
            .streams()
            .best(ffmpeg_next::media::Type::Video)
            .ok_or(ffmpeg_next::Error::StreamNotFound)
            .expect("No stream found");
        let video_stream_index = input.index();

        let context_decoder =
            ffmpeg_next::codec::context::Context::from_parameters(input.parameters())
                .expect("Failed to build context");

        let mut decoder = context_decoder
            .decoder()
            .video()
            .expect("Failed to create video decoder");

        init_result_sender.send(Ok(())).unwrap();

        let receive_and_process_decoded_frames =
            |decoder: &mut ffmpeg_next::decoder::Video| -> Result<(), ffmpeg_next::Error> {
                let mut decoded = ffmpeg_next::util::frame::video::Video::empty();
                while decoder.receive_frame(&mut decoded).is_ok() {
                    let frame = match frame_from_ffmpeg(&mut decoded) {
                        Ok(frame) => frame,
                        Err(_) => {
                            continue;
                        }
                    };

                    if frame_sender.send(PipelineEvent::Data(frame)).is_err() {
                        return Ok(());
                    }

                    // std::thread::sleep(Duration::from_millis(16));
                }
                Ok(())
            };

        for (stream, packet) in ictx.packets() {
            if stream.index() == video_stream_index {
                decoder
                    .send_packet(&packet)
                    .expect("Could not send the packet for some reason");
                receive_and_process_decoded_frames(&mut decoder)
                    .expect("Could not process decoded frames");
            }
        }
        decoder.send_eof().expect("EOF sending failed");
        receive_and_process_decoded_frames(&mut decoder).expect("Failed to process the last frame");
    }

    if frame_sender.send(PipelineEvent::EOS).is_err() {
        debug!("Failed to send EOS from H264 decoder. Channel closed.")
    }
}
