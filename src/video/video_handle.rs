use crossbeam_channel::Receiver;

use std::{
    path::PathBuf,
    sync::{atomic::AtomicBool, Arc},
    time::{Duration, Instant},
};

use crate::{
    shared::Shared,
    thread_utils::custom_beams::{self, LooseSender},
    video::{run_decoder_thread, InputInitError},
    BENCHMARK_MODE,
};

use super::{Frame, PipelineEvent};

const MAX_PLAY_SPEED_SCALAR: f32 = 10.0;

pub enum SeekCommand {
    Stop,
    // Go +10 seconds
    SkipForward,
    // Go -10 seconds
    SkipBackward,
    // Go to a specific time (convert duration to sample_id)
    Seek(f64),
}

#[derive(PartialEq)]
pub enum PlayState {
    // Just created or stopped back to start
    Stopped,
    // Currently playing
    Playing,
    // Currently paused
    Paused,
    // Received EOS
    Completed,
}

#[derive(PartialEq)]
pub enum PlaySpeed {
    /// User needs to press for next frame
    Stopped,
    /// x0.25
    Slower,
    /// x0.5
    Slow,
    /// x1.0
    Normal,
    /// x1.5
    Fast,
    /// x2.0
    Faster,
    /// The fastes the CPU can handle
    Fastest,
}

pub struct InitData {
    pub fps: f64,
    pub total_duration: Duration,
}

pub struct VideoHandle {
    pub fps: f64,
    close_thread: Arc<AtomicBool>,
    pub total_duration: Duration,
    pub progress: f64,
    pub play_speed: f32,
    pub dropped_frames: u64,
    play_state: PlayState,
    command_sender: LooseSender<SeekCommand>,
    yuv_frame_receiver: Receiver<PipelineEvent<Frame>>,
    next_frame: Option<Frame>,
    queued_frame: Option<Frame>,
    last_update: Instant,
    current_timestamp: Duration,
    last_decode_cost: Option<Duration>,
    frame_timestamp: Option<Duration>,
    eos: bool,
}

impl Drop for VideoHandle {
    fn drop(&mut self) {
        self.close_thread
            .store(true, std::sync::atomic::Ordering::Relaxed);
    }
}

fn scale_duration(duration: Duration, scale: f32) -> Duration {
    let nanos = duration.as_secs_f32() * scale;

    Duration::from_secs_f32(nanos)
}

impl VideoHandle {
    pub fn create(file: PathBuf) -> Shared<VideoHandle> {
        let (command_sender, command_receiver) = custom_beams::loose::<SeekCommand>(1);

        let (yuv_frame_sender, yuv_frame_receiver) =
            crossbeam_channel::bounded(if BENCHMARK_MODE { 512 } else { 16 });

        let (init_result_sender, init_result_receiver) =
            crossbeam_channel::bounded::<Result<InitData, InputInitError>>(0);

        let close_thread = Arc::new(AtomicBool::new(false));

        let close_thread_clone = close_thread.clone();

        std::thread::Builder::new()
            .spawn(move || {
                run_decoder_thread(
                    file,
                    init_result_sender,
                    yuv_frame_sender,
                    close_thread_clone,
                    command_receiver,
                )
            })
            .unwrap();

        if let Ok(data) = init_result_receiver
            .recv()
            .expect("Failed to read from ffmpeg thread")
        {
            return Shared::new(VideoHandle {
                close_thread,
                fps: data.fps,
                total_duration: data.total_duration,
                progress: 0.0,
                play_state: PlayState::Playing,
                play_speed: 1.0,
                command_sender,
                last_update: Instant::now(),
                yuv_frame_receiver,
                next_frame: None,
                dropped_frames: 0,
                queued_frame: None,
                frame_timestamp: None,
                current_timestamp: Duration::from_millis(0),
                last_decode_cost: None,
                eos: false,
            });
        }

        panic!("Failed to create FFMPEG thred")
    }
}

impl Shared<VideoHandle> {
    pub fn play(&self) {
        self.with(|this| {
            this.play_state = PlayState::Playing;
        })
    }
    pub fn pause(&self) {
        self.with(|this| {
            this.play_state = PlayState::Paused;
        })
    }
    pub fn stop(&self) {}

    pub fn get_pts(&self) -> Duration {
        self.with(|this| this.current_timestamp)
    }

    pub fn get_frame_pts(&self) -> Option<Duration> {
        self.with(|this| this.frame_timestamp)
    }

    pub fn get_dropped_frames(&self) -> u64 {
        self.with(|this| this.dropped_frames)
    }

    pub fn get_next_pts(&self) -> Option<Duration> {
        self.with_ref(|this| {
            if let Some(ref frame) = this.next_frame {
                return Some(frame.pts);
            }
            None
        })
    }

    pub fn get_last_decode_cost(&self) -> Option<Duration> {
        self.with_ref(|this| this.last_decode_cost)
    }

    pub fn get_frame_progress(&self) -> Option<f64> {
        self.with(|this| {
            if let Some(ref frame_timestamp) = this.frame_timestamp {
                let current = frame_timestamp.as_millis_f64();

                let total = this.total_duration.as_millis_f64();

                return Some(current / total);
            }
            None
        })
    }

    pub fn get_realtime_progress(&self) -> f64 {
        self.with(|this| {
            let current = this.current_timestamp.as_millis_f64();

            let total = this.total_duration.as_millis_f64();

            current / total
        })
    }

    pub fn get_buffer_size(&self) -> (u64, u64) {
        self.with(|this| {
            let len = this.yuv_frame_receiver.len();
            let capacity = this.yuv_frame_receiver.capacity().unwrap_or(0);

            (len as u64, capacity as u64)
        })
    }

    pub fn get_buffer_health(&self) -> f64 {
        let (len, cap) = self.get_buffer_size();

        len as f64 / cap as f64
    }

    pub fn seek(&self, to: Duration) {
        self.with(|this| {
            let total = this.total_duration.as_millis_f64();
            let target = to.as_millis_f64();

            // Due to incomprehensible problems
            // the seek thread accepts 0.0 .. 1.0 range
            // instead of a Duration
            let norm = target / total;

            this.current_timestamp = to;
            this.frame_timestamp = None;

            this.command_sender
                .loosely_send(SeekCommand::Seek(norm))
                .expect("Sending commands should always succed");

            if this.next_frame.is_some() {
                this.dropped_frames += 1;
                this.next_frame = None;
            }

            // Drain old frames until a SeekAck
            for item in this.yuv_frame_receiver.iter() {
                match item {
                    PipelineEvent::Data(_) => {
                        this.dropped_frames += 1;
                    }
                    PipelineEvent::SeekAck => {
                        break;
                    }
                    PipelineEvent::EOS => {
                        return;
                    }
                }
            }

            // Then drain frames from the keyframe
            // until the frame we want
            for item in this.yuv_frame_receiver.iter() {
                match item {
                    PipelineEvent::Data(frame) => {
                        if frame.pts >= to {
                            if this.queued_frame.is_some() {
                                this.dropped_frames += 1;
                            }

                            this.queued_frame = Some(frame);

                            break;
                        }
                        this.dropped_frames += 1;
                    }
                    PipelineEvent::SeekAck => {
                        panic!("Cannot have two seek acks in the same streams");
                    }
                    PipelineEvent::EOS => {
                        return;
                    }
                }
            }
        });
    }
    pub fn tick(&self) {
        self.with(|this| {
            if this.play_state != PlayState::Playing {
                this.last_update = Instant::now();
                return;
            }

            let has_frames = || !this.yuv_frame_receiver.is_empty();

            let take_one_frame = || {
                this.yuv_frame_receiver
                    .try_recv()
                    .map_or(None, |a| match a {
                        PipelineEvent::Data(frame) => Some(frame),
                        PipelineEvent::EOS => None,
                        PipelineEvent::SeekAck => None,
                    })
            };

            if this.next_frame.is_none() {
                if has_frames() {
                    this.next_frame = take_one_frame();
                    this.eos = this.next_frame.is_none();
                } else {
                    return;
                }
            }

            let now = Instant::now();
            let delta = now - this.last_update;
            this.last_update = now;
            this.current_timestamp += scale_duration(delta, this.play_speed);

            // Frame dropping is being expensive, move it to the decoder thread thread

            loop {
                // Next frame is in the future
                // we can grab it inext farme
                if let Some(ref frame) = this.next_frame {
                    if frame.pts > this.current_timestamp {
                        break;
                    }
                }

                // Otherwise, the current frame sould be enqueued
                // We will check it again next iteration, until we
                // find a valid frame

                let queued_frame = this.next_frame.take();

                // We took the frame, but there was already one equeued.
                // This happens in two cases:
                // * We are currently draining many old frames
                // * There was a FPS drop on the render thread
                // * And the PTS advanced too much, so many frames are
                // enqueed but not presented in this loop.
                if this.queued_frame.is_some() {
                    this.dropped_frames += 1;
                }
                this.queued_frame = queued_frame;

                // Update metadata
                if let Some(ref frame) = this.queued_frame {
                    this.frame_timestamp = Some(frame.pts);
                    this.last_decode_cost = Some(frame.decode_cost);
                }

                // Set next frame
                this.next_frame = take_one_frame();
                this.eos = this.next_frame.is_none();

                // There isn't a new frame, break
                // and leave processing for the next tick instead.
                if this.eos {
                    break;
                }
            }
        });
    }
    pub fn get_current_frame(&self) -> Option<Frame> {
        self.with(|this| this.queued_frame.take())
    }
}
