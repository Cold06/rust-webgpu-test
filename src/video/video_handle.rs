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
};

use super::{Frame, PipelineEvent};

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
    pub total_duration: f64,
}

pub struct VideoHandle {
    pub fps: f64,
    close_thread: Arc<AtomicBool>,
    pub total_duration: f64,
    pub progress: f64,
    pub play_speed: PlaySpeed,
    pub dropped_frames: u64,
    play_state: PlayState,
    command_sender: LooseSender<SeekCommand>,
    yuv_frame_receiver: Receiver<PipelineEvent<Frame>>,
    next_frame: Option<Frame>,
    queued_frame: Option<Frame>,
    last_update: Instant,
    current_timestamp: Duration,
    eos: bool,
}

impl Drop for VideoHandle {
    fn drop(&mut self) {
        self.close_thread
            .store(true, std::sync::atomic::Ordering::Relaxed);
    }
}

impl VideoHandle {
    pub fn create(file: PathBuf) -> Shared<VideoHandle> {
        let (command_sender, command_receiver) = custom_beams::loose::<SeekCommand>(1);

        let (yuv_frame_sender, yuv_frame_receiver) = crossbeam_channel::bounded(100);

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
                play_speed: PlaySpeed::Normal,
                command_sender,
                last_update: Instant::now(),
                yuv_frame_receiver,
                next_frame: None,
                dropped_frames: 0,
                queued_frame: None,
                current_timestamp: Duration::from_millis(0),
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

    pub fn seek(&self, to: f64) {
        self.with(|this| {
            this.command_sender
                .loosely_send(SeekCommand::Seek(to))
                .expect("Sending commands should always succed")
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
            this.current_timestamp += delta;

            loop {
                if let Some(ref frame) = this.next_frame {
                    if this.current_timestamp < frame.pts {
                        break;
                    }
                }

                let queued_frame = this.next_frame.take();

                if this.queued_frame.is_some() {
                    this.dropped_frames += 1;
                }

                this.queued_frame = queued_frame;

                this.next_frame = take_one_frame();
                this.eos = this.next_frame.is_none();
            }
        });
    }
    pub fn get_current_frame(&self) -> Option<Frame> {
        self.with(|this| this.queued_frame.take())
    }
}
