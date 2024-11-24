use crossbeam_channel::{Receiver, Sender};
use decoder_thread::{run_decoder_thread, Frame, InputInitError};
mod decoder_thread;
use std::{
    path::PathBuf,
    sync::{atomic::AtomicBool, Arc},
};

pub use decoder_thread::{FrameData, PipelineEvent, Resolution};

use crate::{
    shared::Shared,
    thread_utils::custom_beams::{self, LooseSender},
};

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
    Seek(f64),

    // Go to middle
    GoToMiddle,
}

pub enum VideoUpdateInfo {
    Started,
    Frame(f64),
    EOS,
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
    fps: f64,
    total_duration: f64,
}

pub struct VideoHandle {
    pub fps: f64,
    close_thread: Arc<AtomicBool>,
    pub total_duration: f64,
    pub progress: f64,
    pub play_speed: PlaySpeed,
    play_state: PlayState,
    command_sender: LooseSender<MP4Command>,
    update_receiver: Receiver<VideoUpdateInfo>,
    yuv_frame_receiver: Receiver<PipelineEvent<Frame>>,
}

impl Drop for VideoHandle {
    fn drop(&mut self) {
        self.close_thread
            .store(true, std::sync::atomic::Ordering::Relaxed);
    }
}

impl VideoHandle {
    pub fn create(file: PathBuf) -> Shared<VideoHandle> {
        let (command_sender, command_receiver) = custom_beams::loose::<MP4Command>(1);

        let (update_sender, update_receiver) = custom_beams::loose::<VideoUpdateInfo>(1);

        let (yuv_frame_sender, yuv_frame_receiver) = custom_beams::loose(1);

        let (init_result_sender, init_result_receiver) =
            crossbeam_channel::bounded::<Result<InitData, InputInitError>>(0);

        let close_thread = Arc::new(AtomicBool::new(false));

        let close_thread_clone = close_thread.clone();

        std::thread::Builder::new()
            .spawn(move || {
                println!("Starting FFMPEG decoder thread");

                run_decoder_thread(
                    file,
                    init_result_sender,
                    yuv_frame_sender,
                    close_thread_clone,
                    command_receiver,
                    update_sender,
                )
            })
            .unwrap();

        // BLOCKING
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
                update_receiver,
                yuv_frame_receiver,
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
    pub fn stop(&self) {
        self.with(|this| {
            drop(this.command_sender.send(MP4Command::GoToMiddle));
        })
    }
    pub fn seek(&self, to: f64) {
        // Seek operations can be loosy
        // while everything else, cannot
        // BUG: needs a way to take a look at the channel
        // to know if the command we just dropped is a important
        // one like pause, so we gasp! say "my bad", and put it back
        self.with(|this| {
            this.command_sender
                .loosely_send(MP4Command::Seek(to))
                .expect("Sending commands should always succed")
        });
    }
    pub fn sync(&self) {
        self.with(|this| {
            if let Ok(update) = this.update_receiver.try_recv() {
                match update {
                    VideoUpdateInfo::Started => {
                        println!("Play started")
                    }
                    VideoUpdateInfo::Frame(f) => {
                        this.progress = f;
                    }
                    VideoUpdateInfo::EOS => {
                        println!("EOS")
                    }
                }
            }
        });
    }
    pub fn try_read_next_frame(&self) -> Option<PipelineEvent<Frame>> {
        self.with(|this| {
            // To pause, simply just don't read, this will
            // block the sending of more frames from ffmpeg thread
            if this.play_state != PlayState::Playing {
                return None;
            }

            this.yuv_frame_receiver.try_recv().map_or(None, |e| Some(e))
        })
    }
}
