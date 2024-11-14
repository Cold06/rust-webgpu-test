use std::path::PathBuf;
use crossbeam_channel::{Receiver, Sender};
use crate::video::decoder::{start_video_decoder_thread, Frame};
use crate::video::reader::{create_mp4_reader_thread, Mp4FileReader, VideoDecoderOptions, VideoInputReceiver};

mod decoder;
mod reader;

pub use decoder::{Resolution, FrameData};
pub use reader::{MP4Command, PipelineEvent};

pub fn start(
    file: PathBuf,
) -> (
    Mp4FileReader<VideoDecoderOptions>,
    Receiver<PipelineEvent<Frame>>,
    Sender<MP4Command>,
) {
    let (command_sender, command_receiver) = crossbeam_channel::bounded::<MP4Command>(1);

    let (file_reader, video_receiver) = create_mp4_reader_thread(file, command_receiver);

    let yuv_frame_receiver = match video_receiver {
        VideoInputReceiver::Encoded {
            decoder_options,
            chunk_receiver,
        } => {
            let (sender, receiver) = crossbeam_channel::bounded(10);
            start_video_decoder_thread(decoder_options, chunk_receiver, sender);
            receiver
        }
    };

    (file_reader, yuv_frame_receiver, command_sender)
}

