use bytes::{Buf, Bytes, BytesMut};
use crossbeam_channel::{Receiver, Sender};
use mp4::Mp4Track;
use std::io::{Read, Seek};
use std::os::unix::fs::MetadataExt;
use std::path::PathBuf;
use std::sync::atomic::AtomicBool;
use std::sync::Arc;
use std::time::Duration;
use tracing::error;

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

struct TrackInfo<SampleUnpacker: FnMut(mp4::Mp4Sample) -> Bytes> {
    sample_count: u32,
    timescale: u32,
    track_id: u32,
    sample_unpacker: SampleUnpacker,
    frame_rate: f64,
    bitrate: u32,
    height: u16,
    width: u16,
    default_sample_duration: u32,
    duration: Duration,
}

fn find_h264_info<Reader: Read + Seek + Send + 'static>(
    reader: &mp4::Mp4Reader<Reader>,
) -> Option<TrackInfo<impl FnMut(mp4::Mp4Sample) -> Bytes>> {
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

    Some(TrackInfo {
        duration: track.duration(),
        default_sample_duration: track.default_sample_duration,
        width: track.width(),
        height: track.height(),
        bitrate: track.bitrate(),
        frame_rate: track.frame_rate(),
        sample_count: track.sample_count(),
        timescale: track.timescale(),
        track_id,
        sample_unpacker,
    })
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

pub struct Mp4FileReader {
    stop_thread: Arc<AtomicBool>,
    fragment_sender: Option<Sender<PipelineEvent<Bytes>>>,
}

pub struct EncodedChunk {
    pub data: Bytes,
    pub pts: Duration,
    pub dts: Option<Duration>,
}

type ChunkReceiver = Receiver<PipelineEvent<EncodedChunk>>;

struct VideoCursor {
    video_duration: Duration,
    avg_sample_duration: Duration,
    sample_count: u32,
    current_sample: u32,
    command_receiver: Receiver<MP4Command>,
}

impl VideoCursor {
    pub fn new(
        video_duration: Duration,
        sample_count: u32,
        command_receiver: Receiver<MP4Command>,
    ) -> Self {
        Self {
            sample_count,
            video_duration,
            current_sample: 1,
            avg_sample_duration: video_duration / sample_count,
            command_receiver,
        }
    }

    pub fn next_sample(&mut self, reader: &Option<&Mp4Track>) -> Option<u32> {
        let stss_box = if let Some(ref reader) = reader {
            if let Some(ref stss_box) = reader.trak.mdia.minf.stbl.stss {
                Some(stss_box)
            } else {
                None
            }
        } else {
            None
        };

        fn find_nearest_sample(target: u32, numbers: &Vec<u32>) -> Option<u32> {
            let pos = numbers.binary_search(&target).unwrap_or_else(|x| x);
            if pos == 0 {
                None
            } else {
                Some(numbers[pos - 1])
            }
        }

        // TODO:
        // Buffer 10 seconds worth of frames
        // Seek operations have fast path through the buffer
        // And slow path
        // A seek should always be successful
        // That means we probably need to invert the controls a bit
        //  View -> Decoder -> Reader
        // Video controls stays on the view
        // view asks the decoder for more frames
        // which asks the reader for more packets
        // during a seek, we are skipping frames
        // or maybe invalidating the whole buffer.
        //
        // both the decoder and the reader have to
        // cope with that by either showing the frame
        // or showing a loading state, but thats for later
        //
        // for now just skipping through keyframes is enough
        //
        // also keyframes are useful for determining animation sections


        match self.command_receiver.try_recv() {
            Ok(command) => match command {
                MP4Command::SkipBackward => {
                    let target_next_sample = self.current_sample.checked_sub(100).unwrap_or(0);

                    if let Some(stss) = stss_box {
                        if let Some(actual_next_sample) =
                            find_nearest_sample(target_next_sample, &stss.entries)
                        {
                            self.current_sample = actual_next_sample;
                        } else {
                            println!(
                                "No suitable STSS box for {} found, skip command will fail",
                                target_next_sample
                            );
                        }
                    } else {
                        println!("No STSS box found");
                    }
                }
                MP4Command::SkipForward => {
                    println!("Skip forward");
                    let target_next_sample = self
                        .current_sample
                        .checked_add(100)
                        .unwrap_or(self.sample_count);

                    if let Some(stss) = stss_box {
                        if let Some(actual_next_sample) =
                            find_nearest_sample(target_next_sample, &stss.entries)
                        {
                            self.current_sample = actual_next_sample;
                        } else {
                            println!(
                                "No suitable STSS box for {} found, skip command will fail",
                                target_next_sample
                            );
                        }
                    } else {
                        println!("No STSS box found");
                    }
                }
                MP4Command::Pause => {
                    println!("Pause");
                }
                MP4Command::Play => {
                    println!("Play");
                }
                MP4Command::Stop => {
                    println!("Stop");
                }
                MP4Command::Seek(duration) => {
                    println!("Seek to {:?}", duration);
                }
            },
            _ => {
                self.current_sample += 1;
            }
        }

        if self.current_sample <= self.sample_count {
            return Some(self.current_sample);
        }

        None
    }

    pub fn seek(to: Duration) {
        // Update current sample and
        // return it while to > 0 && to < duration
        // get the sample_id from duration, even if a little incorrect
    }

    pub fn skip_forward(seconds: u32) {
        // update current sample to
        // += number of samples in a second * seconds
    }
    pub fn skip_backward(seconds: u32) {
        // update current sample to
        // -= number of samples in a second * seconds
    }
}

fn run_reader_thread<Reader: Read + Seek>(
    mut reader: mp4::Mp4Reader<Reader>,
    sender: Sender<PipelineEvent<EncodedChunk>>,
    stop_thread: Arc<AtomicBool>,
    track_info: TrackInfo<impl FnMut(mp4::Mp4Sample) -> Bytes>,
    command_receiver: Receiver<MP4Command>,
) {
    let mut sample_unpacker = track_info.sample_unpacker;

    let mut video_cursor = VideoCursor::new(
        track_info.duration,
        track_info.sample_count,
        command_receiver,
    );

    loop {
        if stop_thread.load(std::sync::atomic::Ordering::Relaxed) {
            break;
        }

        if let Some(sample_id) =
            video_cursor.next_sample(&reader.tracks().get(&track_info.track_id))
        {
            match reader.read_sample(track_info.track_id, sample_id) {
                Ok(Some(sample)) => {
                    let rendering_offset = sample.rendering_offset;
                    let start_time = sample.start_time;

                    let sample_duration = Duration::from_secs_f64(
                        sample.duration as f64 / track_info.timescale as f64,
                    );

                    let dts =
                        Duration::from_secs_f64(start_time as f64 / track_info.timescale as f64);

                    let pts = Duration::from_secs_f64(
                        (start_time as f64 + rendering_offset as f64) / track_info.timescale as f64,
                    );

                    let data = sample_unpacker(sample);

                    let chunk = EncodedChunk {
                        data,
                        pts,
                        dts: Some(dts),
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
        } else {
            // TODO: instead of breaking pause and wait for seek/play commands
            break;
        }
    }

    if let Err(_err) = sender.send(PipelineEvent::EOS) {
        println!("Failed to send EOS from MP4 video reader. Channel closed.");
    }
}

impl Mp4FileReader {
    fn new_video(
        file: PathBuf,
        command_receiver: Receiver<MP4Command>,
    ) -> Result<Option<(Mp4FileReader, ChunkReceiver)>, Mp4Error> {
        let stop_thread = Arc::new(AtomicBool::new(false));

        let input_file = std::fs::File::open(file)?;
        let size = input_file.metadata()?.size();

        Self::new(
            input_file,
            size,
            find_h264_info,
            stop_thread,
            command_receiver,
        )
    }

    fn new<
        TReader: Read + Seek + Send + 'static,
        TUnpacker: FnMut(mp4::Mp4Sample) -> Bytes + Send + 'static,
    >(
        reader: TReader,
        size: u64,
        track_info_reader: impl Fn(&mp4::Mp4Reader<TReader>) -> Option<TrackInfo<TUnpacker>>,
        stop_thread: Arc<AtomicBool>,
        command_receiver: Receiver<MP4Command>,
    ) -> Result<Option<(Self, ChunkReceiver)>, Mp4Error> {
        let reader = mp4::Mp4Reader::read_header(reader, size)?;

        let Some(track_info) = track_info_reader(&reader) else {
            return Ok(None);
        };

        let (sender, receiver) = crossbeam_channel::bounded(10);

        let stop_thread_clone = stop_thread.clone();

        std::thread::Builder::new()
            .name("mp4 reader".to_string())
            .spawn(move || {
                println!("Starting MP4 Reader Thread");
                run_reader_thread(
                    reader,
                    sender,
                    stop_thread_clone,
                    track_info,
                    command_receiver,
                );
            })
            .unwrap();

        Ok(Some((
            Mp4FileReader {
                stop_thread,
                fragment_sender: None,
            },
            receiver,
        )))
    }
}

pub type VideoInputReceiver = Receiver<PipelineEvent<EncodedChunk>>;

pub fn create_mp4_reader_thread(
    file: PathBuf,
    command_receiver: Receiver<MP4Command>,
) -> (Mp4FileReader, VideoInputReceiver) {
    Mp4FileReader::new_video(file, command_receiver)
        .unwrap()
        .unwrap()
}
