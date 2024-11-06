use std::io::{Read, Seek, SeekFrom};
use std::os::unix::fs::MetadataExt;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::AtomicBool;
use std::time::Duration;
use bytes::{Buf, Bytes, BytesMut};
use crossbeam_channel::{bounded, Receiver, Sender};
use ffmpeg_next::codec::{Context, Id};
use ffmpeg_next::format::Pixel;
use ffmpeg_next::frame::Video;
use ffmpeg_next::media::Type;
use ffmpeg_next::Rational;
use mp4::Mp4Reader;
use tracing::{debug, error, span, trace, warn};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VideoDecoder {
    FFmpegH264,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VideoDecoderOptions {
    pub decoder: VideoDecoder,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VideoCodec {
    H264,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EncodedChunkKind {
    Video(VideoCodec),
}


struct TrackInfo<DecoderOptions, SampleUnpacker: FnMut(mp4::Mp4Sample) -> Bytes> {
    sample_count: u32,
    timescale: u32,
    track_id: u32,
    decoder_options: DecoderOptions,
    sample_unpacker: SampleUnpacker,
    chunk_kind: EncodedChunkKind,
}

fn find_h264_info<Reader: Read + Seek + Send + 'static>(
    reader: &mp4::Mp4Reader<Reader>,
) -> Option<TrackInfo<VideoDecoderOptions, impl FnMut(mp4::Mp4Sample) -> Bytes>> {
    let (&track_id, track, avc) = reader.tracks().iter().find_map(|(id, track)| {
        let track_type = track.track_type().ok()?;
        let media_type = track.media_type().ok()?;
        let avc = track.avc1_or_3_inner();

        if track_type != mp4::TrackType::Video
            || media_type != mp4::MediaType::H264
            || avc.is_none()
        {
            return None;
        }

        avc.map(|avc| (id, track, avc))
    })?;

    // sps and pps have to be extracted from the container, interleaved with [0, 0, 0, 1],
    // concatenated and prepended to the first frame.
    let sps = avc
        .avcc
        .sequence_parameter_sets
        .iter()
        .flat_map(|s| [0, 0, 0, 1].iter().chain(s.bytes.iter()));

    let pps = avc
        .avcc
        .picture_parameter_sets
        .iter()
        .flat_map(|s| [0, 0, 0, 1].iter().chain(s.bytes.iter()));

    let mut sps_and_pps_payload = Some(sps.chain(pps).copied().collect::<Bytes>());

    let length_size = avc.avcc.length_size_minus_one + 1;

    let sample_unpacker = move |sample: mp4::Mp4Sample| {
        let mut sample_data = sample.bytes.reader();
        let mut data: BytesMut = Default::default();

        if let Some(first_nal) = sps_and_pps_payload.take() {
            data.extend_from_slice(&first_nal);
        }

        // the mp4 sample contains one h264 access unit (possibly more than one NAL).
        // the NALs are stored as: <length_size bytes long big endian encoded length><the NAL>.
        // we need to convert this into Annex B, in which NALs are separated by
        // [0, 0, 0, 1]. `length_size` is at most 4 bytes long.
        loop {
            let mut len = [0u8; 4];

            if sample_data
                .read_exact(&mut len[4 - length_size as usize..])
                .is_err()
            {
                break;
            }

            let len = u32::from_be_bytes(len);

            let mut nalu = bytes::BytesMut::zeroed(len as usize);
            sample_data.read_exact(&mut nalu).unwrap();

            data.extend_from_slice(&[0, 0, 0, 1]);
            data.extend_from_slice(&nalu);
        }

        data.freeze()
    };

    let decoder_options = VideoDecoderOptions {
        decoder: VideoDecoder::FFmpegH264,
    };

    Some(TrackInfo {
        sample_count: track.sample_count(),
        timescale: track.timescale(),
        decoder_options,
        track_id,
        sample_unpacker,
        chunk_kind: EncodedChunkKind::Video(VideoCodec::H264),
    })
}

#[derive(Debug)]
pub enum PipelineEvent<T> {
    Data(T),
    EOS,
}

#[derive(Debug, thiserror::Error)]
pub enum Mp4Error {
    #[error("Error while doing file operations.")]
    IoError(#[from] std::io::Error),

    #[error("Mp4 reader error.")]
    Mp4ReaderError(#[from] mp4::Error),

    #[error("No suitable track in the mp4 file")]
    NoTrack,
}

pub struct Mp4FileReader<DecoderOptions> {
    stop_thread: Arc<AtomicBool>,
    fragment_sender: Option<Sender<PipelineEvent<Bytes>>>,
    decoder_options: DecoderOptions,
}

impl<DecoderOptions: Clone + Send + 'static> Mp4FileReader<DecoderOptions> {
    pub(crate) fn decoder_options(&self) -> DecoderOptions {
        self.decoder_options.clone()
    }
}

pub struct EncodedChunk {
    pub data: Bytes,
    pub pts: Duration,
    pub dts: Option<Duration>,
    pub kind: EncodedChunkKind,
}

type ChunkReceiver = Receiver<PipelineEvent<EncodedChunk>>;

enum Mp4ReaderOptions {
    NonFragmented {
        file: PathBuf,
        should_loop: bool,
    },
}




fn run_reader_thread<Reader: Read + Seek, DecoderOptions>(
    mut reader: Mp4Reader<Reader>,
    sender: Sender<PipelineEvent<EncodedChunk>>,
    stop_thread: Arc<AtomicBool>,
    _fragment_receiver: Option<Receiver<PipelineEvent<Bytes>>>,
    track_info: TrackInfo<DecoderOptions, impl FnMut(mp4::Mp4Sample) -> Bytes>,
    should_loop: bool,
) {
    // Grab our registered sample unpacker
    let mut sample_unpacker = track_info.sample_unpacker;

    // State variable to keep track of current loop
    let mut loop_offset = Duration::ZERO;

    loop {
        let mut last_end_pts = Duration::ZERO;

        // track_info.sample_count is gotten from the Mp4 metadata
        for i in 1..track_info.sample_count {

            // Control variable to be able to force a stop
            if stop_thread.load(std::sync::atomic::Ordering::Relaxed) {
                return;
            }

            // You can probably read many samples here
            // But here we just read one track per sample_count
            // QUESTION: how to seek the file

            match reader.read_sample(track_info.track_id, i) {
                Ok(Some(sample)) => {
                    let rendering_offset = sample.rendering_offset;
                    let start_time = sample.start_time;

                    let sample_duration = Duration::from_secs_f64(
                        sample.duration as f64 / track_info.timescale as f64,
                    );

                    let dts =
                        Duration::from_secs_f64(start_time as f64 / track_info.timescale as f64)
                            + loop_offset;

                    let pts = Duration::from_secs_f64(
                        (start_time as f64 + rendering_offset as f64) / track_info.timescale as f64,
                    ) + loop_offset;

                    last_end_pts = pts + sample_duration;

                    let data = sample_unpacker(sample);

                    let chunk = EncodedChunk {
                        data,
                        pts,
                        dts: Some(dts),
                        kind: track_info.chunk_kind,
                    };

                    match sender.send(PipelineEvent::Data(chunk)) {
                        Ok(_) => {}
                        Err(_) => {
                            println!("Failed to send MP4 chunk. Channel closed.");
                            return;
                        }
                    }
                }
                Err(e) => {
                    println!("Error while reading MP4 video sample: {:?}", e);
                }
                _ => {}
            }
        }
        loop_offset = last_end_pts;
        if !should_loop {
            break;
        }
    }
    if let Err(_err) = sender.send(PipelineEvent::EOS) {
        println!("Failed to send EOS from MP4 video reader. Channel closed.");
    }
}


impl<DecoderOptions: Clone + Send + 'static> Mp4FileReader<DecoderOptions> {
    fn new<
        TReader: Read + Seek + Send + 'static,
        TUnpacker: FnMut(mp4::Mp4Sample) -> Bytes + Send + 'static,
    >(
        reader: TReader,
        size: u64,
        track_info_reader: impl Fn(
            &mp4::Mp4Reader<TReader>,
        ) -> Option<TrackInfo<DecoderOptions, TUnpacker>>,
        fragment_receiver: Option<Receiver<PipelineEvent<Bytes>>>,
        stop_thread: Arc<AtomicBool>,
        should_loop: bool,
    ) -> Result<Option<(Self, ChunkReceiver)>, Mp4Error> {
        let reader = mp4::Mp4Reader::read_header(reader, size)?;

        let Some(track_info) = track_info_reader(&reader) else {
            return Ok(None);
        };

        let (sender, receiver) = crossbeam_channel::bounded(10);

        let stop_thread_clone = stop_thread.clone();
        let decoder_options = track_info.decoder_options.clone();

        std::thread::Builder::new()
            .name("mp4 reader".to_string())
            .spawn(move || {
                println!("Starting MP4 Reader Thread");
                run_reader_thread(
                    reader,
                    sender,
                    stop_thread_clone,
                    fragment_receiver,
                    track_info,
                    should_loop,
                );
            })
            .unwrap();

        Ok(Some((
            Mp4FileReader {
                stop_thread,
                fragment_sender: None,
                decoder_options,
            },
            receiver,
        )))
    }
}

impl Mp4FileReader<VideoDecoderOptions> {


    fn new_video(options: Mp4ReaderOptions) -> Result<Option<(Mp4FileReader<VideoDecoderOptions>, ChunkReceiver)>, Mp4Error> {
        let stop_thread = Arc::new(AtomicBool::new(false));

        match options {
            Mp4ReaderOptions::NonFragmented { file, should_loop } => {
                let input_file = std::fs::File::open(file)?;
                let size = input_file.metadata()?.size();

                Self::new(input_file, size, find_h264_info, None, stop_thread, should_loop)
            }
        }
    }
}

enum VideoInputReceiver {
    Encoded {
        chunk_receiver: Receiver<PipelineEvent<EncodedChunk>>,
        decoder_options: VideoDecoderOptions,
    },
}


fn middle(file: PathBuf) -> (Mp4FileReader<VideoDecoderOptions>, VideoInputReceiver) {
    let video = Mp4FileReader::new_video(Mp4ReaderOptions::NonFragmented {
        file,
        should_loop: false,
    }).unwrap();

    let (video_reader, video_receiver) = match video {
        Some((reader, receiver)) => {
            let input_receiver = VideoInputReceiver::Encoded {
                chunk_receiver: receiver,
                decoder_options: reader.decoder_options(),
            };
            (Some(reader), Some(input_receiver))
        }
        None => (None, None),
    };

    (video_reader.unwrap(), video_receiver.unwrap())
}

pub fn start_video_decoder_thread(
    options: VideoDecoderOptions,
    chunks_receiver: Receiver<PipelineEvent<EncodedChunk>>,
    frame_sender: Sender<PipelineEvent<Frame>>,
) {
    match options.decoder {
        VideoDecoder::FFmpegH264 => start_ffmpeg_decoder_thread(
            chunks_receiver,
            frame_sender,
        ).unwrap(),
    };
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Resolution {
    pub width: usize,
    pub height: usize,
}

pub const MAX_NODE_RESOLUTION: Resolution = Resolution {
    width: 7682,
    height: 4320,
};

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
    pub pts: Duration,
}


pub fn start(file: PathBuf) -> (Mp4FileReader<VideoDecoderOptions>, Receiver<PipelineEvent<Frame>>)  {
    let (file_reader, video_receiver) = middle(file);

    let yuv_frame_receiver = match video_receiver {
        VideoInputReceiver::Encoded {
            decoder_options,
            chunk_receiver,
        } => {
            let (sender, receiver) = bounded(10);
            start_video_decoder_thread(
                decoder_options,
                chunk_receiver,
                sender,
            );
            receiver
        }
    };

    (file_reader, yuv_frame_receiver)
}

#[derive(Debug, thiserror::Error)]
pub enum InputInitError {
    #[error(transparent)]
    FfmpegError(#[from] ffmpeg_next::Error),

    #[error("Couldn't read decoder init result.")]
    CannotReadInitResult,
}

#[derive(Debug, thiserror::Error)]
enum DecoderChunkConversionError {
    #[error(
        "Cannot send a chunk of kind {0:?} to the decoder. The decoder only handles H264-encoded video."
    )]
    BadPayloadType(EncodedChunkKind),
}


fn chunk_to_av(chunk: EncodedChunk) -> Result<ffmpeg_next::Packet, DecoderChunkConversionError> {
    if chunk.kind != EncodedChunkKind::Video(VideoCodec::H264) {
        return Err(DecoderChunkConversionError::BadPayloadType(chunk.kind));
    }

    let mut packet = ffmpeg_next::Packet::new(chunk.data.len());

    packet.data_mut().unwrap().copy_from_slice(&chunk.data);
    packet.set_pts(Some(chunk.pts.as_micros() as i64));
    packet.set_dts(chunk.dts.map(|dts| dts.as_micros() as i64));

    Ok(packet)
}

#[derive(Debug, thiserror::Error)]
enum DecoderFrameConversionError {
    #[error("Error converting frame: {0}")]
    FrameConversionError(String),
    #[error("Unsupported pixel format: {0:?}")]
    UnsupportedPixelFormat(ffmpeg_next::format::pixel::Pixel),
}

fn copy_plane_from_av(decoded: &Video, plane: usize) -> bytes::Bytes {
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
    decoded: &mut Video,
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
        Pixel::YUV420P => FrameData::PlanarYuv420(YuvPlanes {
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
    let decoder = Context::from_parameters(parameters.clone())
        .map_err(InputInitError::FfmpegError)
        .and_then(|mut decoder| {
            unsafe {
                // This is because we use microseconds as pts and dts in the packets.
                // See `chunk_to_av` and `frame_from_av`.
                (*decoder.as_mut_ptr()).pkt_timebase = Rational::new(1, 1_000_000).into();
            }

            let decoder = decoder.decoder();
            decoder
                .open_as(Into::<Id>::into(parameters.id()))
                .map_err(InputInitError::FfmpegError)
        });

    let mut decoder = match decoder {
        Ok(decoder) => {
            init_result_sender.send(Ok(())).unwrap();
            decoder
        }
        Err(err) => {
            init_result_sender.send(Err(err)).unwrap();
            return;
        }
    };

    let mut decoded_frame = ffmpeg_next::frame::Video::empty();
    let mut pts_offset = None;

    for chunk in chunks_receiver {
        let chunk = match chunk {
            PipelineEvent::Data(chunk) => chunk,
            PipelineEvent::EOS => {
                break;
            }
        };
        if chunk.kind != EncodedChunkKind::Video(VideoCodec::H264) {
            println!(
                "H264 decoder received chunk of wrong kind: {:?}",
                chunk.kind
            );
            continue;
        }

        let av_packet: ffmpeg_next::Packet = match chunk_to_av(chunk) {
            Ok(packet) => packet,
            Err(err) => {
                warn!("Dropping frame: {}", err);
                continue;
            }
        };

        match decoder.send_packet(&av_packet) {
            Ok(()) => {}
            Err(e) => {
                warn!("Failed to send a packet to decoder: {}", e);
                continue;
            }
        }

        while decoder.receive_frame(&mut decoded_frame).is_ok() {
            let frame = match frame_from_av(&mut decoded_frame, &mut pts_offset) {
                Ok(frame) => frame,
                Err(err) => {
                    warn!("Dropping frame: {}", err);
                    continue;
                }
            };

            trace!(pts=?frame.pts, "H264 decoder produced a frame.");
            if frame_sender.send(PipelineEvent::Data(frame)).is_err() {
                debug!("Failed to send frame from H264 decoder. Channel closed.");
                return;
            }
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

        parameters.codec_type = Type::Video.into();
        parameters.codec_id = Id::H264.into();
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
