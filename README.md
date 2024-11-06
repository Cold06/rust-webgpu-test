# WebGPU Testing

Stack:
* WebGPU
* Skia
* Imgui
* Javascript/Wasm

# Supported Toolchain
 * Typescript
 * C#
 * Rust
 * WebAssembly

> How Does `live-compositor` gets a video from FFMPEG to a WebGPU Texture?

```rs
let weak_pipeline = Arc::downgrade(pipeline);
thread::spawn(move || run_renderer_thread(weak_pipeline, video_receiver));

let weak_pipeline = Arc::downgrade(pipeline);
thread::spawn(move || run_audio_mixer_thread(weak_pipeline, audio_receiver));

GOES TO

populate_inputs(scene, FrameSet<InputId>)

    GOES TO

    for (node_texture, input_textures) in scene.inputs.values_mut() {
        input_textures.convert_to_node_texture(ctx.wgpu_ctx, node_texture);
        
        GOES TO
        
            InputTextureState::PlanarYuvTextures {
                textures,
                bind_group,
            } => ctx.format.convert_planar_yuv_to_rgba(
                ctx,
                (textures, bind_group),
                dest_state.rgba_texture(),
                
                pub fn convert(
                    &self,
                    ctx: &WgpuCtx,
                    src: (&PlanarYuvTextures, &wgpu::BindGroup),
                    dst: &RGBATexture,
                ) {
                    let mut encoder = ctx
                        .device
                        .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                            label: Some("Planar YUV 4:2:0 to RGBA color converter encoder"),
                        });
            
                    {
                        let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                            label: Some("Planar YUV 4:2:0 to RGBA color converter render pass"),
                            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                                ops: wgpu::Operations {
                                    load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                                    store: wgpu::StoreOp::Store,
                                },
                                view: &dst.texture().view,
                                resolve_target: None,
                            })],
                            depth_stencil_attachment: None,
                            timestamp_writes: None,
                            occlusion_query_set: None,
                        });
            
                        render_pass.set_pipeline(&self.pipeline);
                        render_pass.set_bind_group(0, src.1, &[]);
                        render_pass.set_bind_group(1, &self.sampler.bind_group, &[]);
                        render_pass.set_push_constants(
                            ShaderStages::VERTEX_FRAGMENT,
                            0,
                            YUVToRGBAPushConstants::new(src.0.variant()).push_constant(),
                        );
            
                        ctx.plane.draw(&mut render_pass);
                    }
            
                    ctx.queue.submit(Some(encoder.finish()));
                }
                
                
            ),
        
    }
```

// ABCDEFG
// Efg Abc Def Gab Bcd E 
// 


There is also: `/Users/cold/w/live-compositor/compositor_pipeline/src/pipeline/decoder/video/ffmpeg_h264.rs` which gets the raw h264 frame.

Which then goes to a layer of indirection which only god knows where gets intercepted

```rs
if frame_sender.send(PipelineEvent::Data(frame)).is_err() {
                debug!("Failed to send frame from H264 decoder. Channel closed.");
                return;
}
```
But there is a `frame_from_av` which i remmember is important

```
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
        Pixel::YUVJ420P => FrameData::PlanarYuvJ420(YuvPlanes {
            y_plane: copy_plane_from_av(decoded, 0),
            u_plane: copy_plane_from_av(decoded, 1),
            v_plane: copy_plane_from_av(decoded, 2),
        }),
        Pixel::UYVY422 => FrameData::InterleavedYuv422(copy_plane_from_av(decoded, 0)),
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
```



builds that `FrameData` type which is used in the webgpu code.

But that basically maps `Pixel::*` formats to `FrameData::*` formats (which are the ones they have the GLSL code to render with)

Another important type they have is the `FrameSet`

```
#[derive(Debug)]
pub struct FrameSet<Id>
where
    Id: From<Arc<str>>,
{
    pub frames: HashMap<Id, Frame>,
    pub pts: Duration,
}
```

Which could basically be a "Video Chunk"

Will be funny to implement video frame scheduling, but thats what we will have to deal with.

# Theory:

`run_decoder_thread` -> `frame_from_av` 
    `T = Frame`

    `PipelineEvent::Data(frame)` 

    SOMEHOW

    `FrameSet`

    `upload(ctx: WgpuCtx, frame: Frame)`

    Wgpu Render 

Ok, so we know how to make this go to the GPU.

The only questions now are:
    Best practices for low level video rendering.
    How to create a scrubber.
    How to render audio (optional).
    How costly is to get a video frame from a video file.

But first:
 * Get some frames from ffmpeg, and upload them as textures.
 * Show them in a quad.
 * Allow them to be used as skia RGBA images (optional)
   * maybe a simple API like `image_from_frame(ctx, Duration::AsMills(10000))`
 * See how fast a decoder thread produce frames, and how to halt frame generation.
    
Feature suggestions:
 * Render episode by average color.

# SO:

Build a video renderer, with playback/scrubbing/buffering

Then you will have the data needed to start matching the frames to a control rig.

note: unfortunately it looks like we are going to have issues related to control rigs being positioned in 3D space while the characters live in 2D

Maybe we are going to have to built a initial render so we can normalize the 2D rendered output, so we can render at the same time the normalized eye control rig result,the 3D control rig result, the 2D exact expected rendered result due to the pose, and the error %, since we have pixel data we can even calculate the error the Bézier curves are making. 


# Video Frame Type:

```rust

// Q: How does one frame is created and how does it get sent to WGPU?

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Resolution {
    pub width: usize,
    pub height: usize,
}

#[derive(Clone)]
pub struct YuvPlanes {
    pub y_plane: bytes::Bytes,
    pub u_plane: bytes::Bytes,
    pub v_plane: bytes::Bytes,
}

#[derive(Debug, Clone)]
pub enum FrameData {
    PlanarYuv420(YuvPlanes),
    PlanarYuvJ420(YuvPlanes),
    InterleavedYuv422(bytes::Bytes),
    Rgba8UnormWgpuTexture(Arc<wgpu::Texture>),
    Nv12WgpuTexture(Arc<wgpu::Texture>),
}

#[derive(Debug, Clone)]
pub struct Frame {
    pub data: FrameData,
    pub resolution: Resolution,
    pub pts: Duration,
}

```


# Video Queueing/Buffering

`compositor_pipeline/src/queue.rs`


```rust
pub const DEFAULT_BUFFER_DURATION: Duration = Duration::from_millis(16 * 5);

/// Queue is responsible for consuming frames from different inputs and producing
/// sets of frames from all inputs in a single batch.
///
/// - PTS of inputs streams can be in any frame of reference.
/// - PTS of frames stored in queue are in a frame of reference where PTS=0 represents.
///   first frame/packet.
/// - PTS of output frames is in a frame of reference where PTS=0 represents
///   start request.
pub struct Queue {
    video_queue: Mutex<VideoQueue>,
    audio_queue: Mutex<AudioQueue>,

    output_framerate: Framerate,

    /// Duration of queue output samples set.
    audio_chunk_duration: Duration,

    /// Define if queue should process frames if all inputs are ready.
    ahead_of_time_processing: bool,
    /// If true do not drop output frames even if queue is behind the
    /// real time clock.
    never_drop_output_frames: bool,

    /// Defines behavior when event is scheduled too late:
    /// true - Event will be executed immediately.
    /// false - Event will be discarded.
    run_late_scheduled_events: bool,

    default_buffer_duration: Duration,

    start_sender: Mutex<Option<Sender<QueueStartEvent>>>,
    scheduled_event_sender: Sender<ScheduledEvent>,

    clock: Clock,

    should_close: AtomicBool,
}
```

# Video Queue

```rust
pub struct VideoQueue {
    inputs: HashMap<InputId, VideoQueueInput>,
    event_emitter: Arc<EventEmitter>,
}

impl VideoQueue {
    /// Gets frames closest to buffer pts. It does not check whether input is ready
    /// or not. It should not be called before pipeline start.
    pub(super) fn get_frames_batch(
        &mut self,
        buffer_pts: Duration,
        queue_start: Instant,
    ) -> QueueVideoOutput {
        let frames = self
            .inputs
            .iter_mut()
            .filter_map(|(input_id, input)| {
                input
                    .get_frame(buffer_pts, queue_start)
                    .map(|frame| (input_id.clone(), frame))
            })
            .collect();

        QueueVideoOutput {
            frames,
            pts: buffer_pts,
        }
    }

    // Funny Concept
    pub(super) fn drop_old_frames_before_start(&mut self) {
        for input in self.inputs.values_mut() {
            input.drop_old_frames_before_start()
        }
    }

}

/// On that same file


pub struct VideoQueueInput {
    input_id: InputId,
    /// Frames are PTS ordered where PTS=0 represents beginning of the stream.
    queue: VecDeque<Frame>,
    /// Frames from the channel might have any PTS, they need to be processed
    /// before adding them to the `queue`.
    receiver: Receiver<PipelineEvent<Frame>>,
    /// Initial buffering + resets PTS to values starting with 0. All
    /// frames from receiver should be processed by this element.
    input_frames_processor: InputProcessor<Frame>,
    /// If stream is required the queue should wait for frames. For optional
    /// inputs a queue will wait only as long as a buffer allows.
    required: bool,
    /// Offset of the stream relative to the start. If set to `None`
    /// offset will be resolved automatically on the stream start.
    offset: Option<Duration>,

    eos_sent: bool,
    first_frame_sent: bool,

    event_emitter: Arc<EventEmitter>,
}



```

# `queue/utils.rs`

```rust
/// InputProcessor handles initial processing for frames/samples that are being
/// queued. For each received frame/sample batch, the `process_new_chunk`
/// method should be called and only elements returned should be used
/// in a queue.
///
/// 1. New input starts in `InputState::WaitingForStart`.
/// 2. When `process_new_chunk` is called for the first time it transitions to
///    the Buffering state.
/// 3. Each new call to the `process_new_chunk` is adding frames to the buffer
///    until it reaches a specific size/duration.
/// 4. After buffer reaches a certain size, calculate the offset and switch
///    to the `Ready` state.
/// 5. In `Ready` state `process_new_chunk` is immediately returning frame or sample
///    batch passed with arguments with modified pts.
#[derive(Debug)]
pub(super) struct InputProcessor<Payload: InputProcessorMediaExt> {
    input_id: InputId,

    buffer_duration: Duration,

    /// Moment where input transitioned to a ready state
    start_time: Option<Instant>,

    state: InputState<Payload>,

    clock: Clock,

    event_emitter: Arc<EventEmitter>,
}

```

# Clock impl if we need one:


```rust
#[derive(Debug, Clone)]
pub(super) struct Clock(Arc<AtomicI64>);

impl Clock {
    pub(super) fn new() -> Self {
        Self(Arc::new(AtomicI64::new(0)))
    }

    pub(super) fn update_delay(&self, start_time: Instant, current_pts: Duration) {
        let real_now = Instant::now();
        let queue_now = start_time + current_pts;
        let delay_ns = if queue_now > real_now {
            -(queue_now.duration_since(real_now).as_nanos() as i64)
        } else {
            real_now.duration_since(queue_now).as_nanos() as i64
        };
        self.0.store(delay_ns, Ordering::Relaxed)
    }

    fn now(&self) -> Instant {
        let delay_nanos = self.0.load(Ordering::Relaxed);
        if delay_nanos >= 0 {
            Instant::now() - Duration::from_nanos(delay_nanos as u64)
        } else {
            Instant::now() + Duration::from_nanos(-delay_nanos as u64)
        }
    }
}
```

# Singleton pattern in rust:

```rust
pub struct Emitter<E: Clone> {
    subscribers: RwLock<Vec<Sender<E>>>,
}

fn global_instance() -> &'static Emitter<Event> {
    static EMITTER: OnceLock<Emitter<Event>> = OnceLock::new();
    EMITTER.get_or_init(Emitter::new)
}

```

# Notes:
 * Video frames comes from the decoder thread
 * Gets sent to the video queue 
 * Which sends them to the frames processor
 * Which accumulates some of them
 * `check_ready_for_pts(next_buffer_pts: Duration, queue_start: Instant)` 
   * `input_start_time`  WILL block when trying to compute stuff  
 * Buffering implementation is at `InputProcessor::handle_data`
 * So, `InputProcessor` deals with buffering
 * Apparently `VideoQueueInput::get_frame` is the goat.
   * `check_ready_for_pts` enqueue frames.

`compositor_pipeline/src/pipeline.rs:425` 

Pipeline::start
Pipeline::register_raw_data_input