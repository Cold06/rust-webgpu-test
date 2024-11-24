use crossbeam_channel::{Receiver, Sender};
use ffmpeg_next::{codec, format, frame, media};
use std::time::Duration;
use tracing::{debug, error, warn};

use crate::video::reader::{EncodedChunk, PipelineEvent};

pub fn start_video_decoder_thread(
    chunks_receiver: Receiver<PipelineEvent<EncodedChunk>>,
    frame_sender: Sender<PipelineEvent<Frame>>,
) {
    start_ffmpeg_decoder_thread(chunks_receiver, frame_sender).unwrap()
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

fn chunk_to_av(chunk: EncodedChunk) -> ffmpeg_next::Packet {
    let mut packet = ffmpeg_next::Packet::new(chunk.data.len());

    packet.data_mut().unwrap().copy_from_slice(&chunk.data);
    packet.set_pts(Some(chunk.pts.as_micros() as i64));
    packet.set_dts(chunk.dts.map(|dts| dts.as_micros() as i64));

    packet
}

#[derive(Debug, thiserror::Error)]
enum DecoderFrameConversionError {
    #[error("Error converting frame: {0}")]
    FrameConversionError(String),
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

fn frame_from_av(
    decoded: &mut frame::Video,
    pts_offset: &mut Option<i64>,
) -> Result<Frame, DecoderFrameConversionError> {
    let original_pts = decoded.pts();
    if let (Some(pts), None) = (decoded.pts(), &pts_offset) {
        *pts_offset = Some(-pts)
    }
    let pts = original_pts
        .map(|original_pts| original_pts + pts_offset.unwrap_or(0))
        .ok_or_else(|| {
            DecoderFrameConversionError::FrameConversionError("missing pts".to_owned())
        })?;
    if pts < 0 {
        error!(pts, pts_offset, "Received negative PTS. PTS values of the decoder output are not monotonically increasing.")
    }
    let pts = Duration::from_micros(i64::max(pts, 0) as u64);
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
        pts,
    })
}

fn run_decoder_thread(
    parameters: ffmpeg_next::codec::Parameters,
    init_result_sender: Sender<Result<(), InputInitError>>,
    chunks_receiver: Receiver<PipelineEvent<EncodedChunk>>,
    frame_sender: Sender<PipelineEvent<Frame>>,
) {
    let ffmpeg_decoder = codec::Context::from_parameters(parameters.clone())
        .map_err(InputInitError::FfmpegError)
        .and_then(|mut decoder| {
            unsafe {
                // This is because we use microseconds as pts and dts in the packets.
                // See `chunk_to_av` and `frame_from_av`.
                (*decoder.as_mut_ptr()).pkt_timebase = ffmpeg_next::Rational::new(1, 1_000_000).into();
            }

            let decoder = decoder.decoder();
            decoder
                .open_as(Into::<codec::Id>::into(parameters.id()))
                .map_err(InputInitError::FfmpegError)
        });

    let mut ffmpeg_context = match ffmpeg_decoder {
        Ok(decoder) => {
            init_result_sender.send(Ok(())).unwrap();
            decoder
        }
        Err(err) => {
            init_result_sender.send(Err(err)).unwrap();
            return;
        }
    };

    let mut decoded_frame = frame::Video::empty();
    let mut pts_offset = None;

    for chunk in chunks_receiver {
        let chunk = match chunk {
            PipelineEvent::Data(chunk) => chunk,
            PipelineEvent::EOS => {
                break;
            }
        };

        let av_packet = chunk_to_av(chunk);

        

        match ffmpeg_context.send_packet(&av_packet) {
            Ok(()) => {}
            Err(e) => {
                warn!("Failed to send a packet to decoder: {}", e);
                continue;
            }
        }


        while ffmpeg_context.receive_frame(&mut decoded_frame).is_ok() {
            let frame = match frame_from_av(&mut decoded_frame, &mut pts_offset) {
                Ok(frame) => frame,
                Err(_) => {
                    continue;
                }
            };

            if frame_sender.send(PipelineEvent::Data(frame)).is_err() {
                return;
            }
            std::thread::sleep(Duration::from_millis(16));
        }
    }
    if frame_sender.send(PipelineEvent::EOS).is_err() {
        debug!("Failed to send EOS from H264 decoder. Channel closed.")
    }
}

pub fn start_ffmpeg_decoder_thread(
    chunks_receiver: Receiver<PipelineEvent<EncodedChunk>>,
    frame_sender: Sender<PipelineEvent<Frame>>,
) -> Result<(), InputInitError> {
    let (init_result_sender, init_result_receiver) = crossbeam_channel::bounded(0);

    let mut parameters = ffmpeg_next::codec::Parameters::new();

    unsafe {
        let parameters = &mut *parameters.as_mut_ptr();

        parameters.codec_type = media::Type::Video.into();
        parameters.codec_id = codec::Id::H264.into();
    };

    std::thread::Builder::new()
        .name(format!("h264 ffmpeg decoder {}", 0))
        .spawn(move || {
            println!("Starting FFMPEG decoder thread");

            run_decoder_thread(
                parameters,
                init_result_sender,
                chunks_receiver,
                frame_sender,
            )
        })
        .unwrap();

    init_result_receiver.recv().unwrap()?;

    Ok(())
}
