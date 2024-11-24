use crossbeam_channel::{Receiver, Sender};
use decoder::{run_decoder_thread, Frame};
use std::path::PathBuf;

mod decoder;

pub use decoder::{FrameData, MP4Command, PipelineEvent, Resolution};

pub fn start_video_decoding(file: PathBuf) -> (Receiver<PipelineEvent<Frame>>, Sender<MP4Command>) {
    let (command_sender, command_receiver) = crossbeam_channel::bounded::<MP4Command>(1);

    let (yuv_frame_sender, yuv_frame_receiver) = crossbeam_channel::bounded(1);

    let (init_result_sender, init_result_receiver) = crossbeam_channel::bounded(0);

    std::thread::Builder::new()
        .name(format!("h264 ffmpeg decoder {}", 0))
        .spawn(move || {
            println!("Starting FFMPEG decoder thread");

            run_decoder_thread(file, init_result_sender, yuv_frame_sender)
        })
        .unwrap();

    init_result_receiver.recv().unwrap().unwrap();

    (yuv_frame_receiver, command_sender)
}
