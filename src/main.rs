mod camera;
mod camera_controller;
mod canvas;
mod multimath;

use crate::camera::Camera;
use crate::camera_controller::CameraController;
use crate::canvas::Canvas;
use crate::multimath::{Mat4, Vec2, Vec3};
use bitflags::bitflags;
use bytemuck::{Pod, Zeroable};
use cgmath;
use imgui::*;
use imgui_wgpu;
use pollster::block_on;
use std::error::Error;
use std::time::Instant;
use wgpu::util::DeviceExt;
use winit::{
    dpi::{LogicalSize, PhysicalSize},
    event::{DeviceEvent, ElementState, Event, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
    keyboard::{Key, NamedKey},
    window::{CursorGrabMode, WindowBuilder},
};

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct Vertex {
    pos: [f32; 4],
    tex_coord: [f32; 2],
}

fn vertex(pos: [i8; 3], tc: [i8; 2]) -> Vertex {
    Vertex {
        pos: [pos[0] as f32, pos[1] as f32, pos[2] as f32, 1.0],
        tex_coord: [tc[0] as f32, tc[1] as f32],
    }
}

struct ModelBundle {
    vertex_data: Vec<Vertex>,
    index_data: Vec<u16>,
}

#[rustfmt::skip]
bitflags! {
    struct BlockFaces: u32 {
        const None    = 0b000000;
        const Top     = 0b000001;
        const Bottom  = 0b000010;
        const Left    = 0b000100;
        const Right   = 0b001000;
        const Front   = 0b010000;
        const Back    = 0b100000;
        const All     = Self::Top.bits()
                        | Self::Bottom.bits()
                        | Self::Left.bits()
                        | Self::Right.bits()
                        | Self::Front.bits()
                        | Self::Back.bits();
    }
}

fn create_vertices(faces: BlockFaces) -> ModelBundle {
    let face_count = faces.bits().count_ones() as usize;

    let mut bundle = ModelBundle {
        vertex_data: Vec::with_capacity(face_count * 4),
        index_data: Vec::with_capacity(face_count * 4),
    };

    let mut i_stack: u16 = 0;

    let mut push_quad = || {
        bundle.index_data.extend([
            i_stack + 0,
            i_stack + 1,
            i_stack + 2,
            i_stack + 2,
            i_stack + 3,
            i_stack + 0,
        ]);

        i_stack += 4;
    };

    if faces.contains(BlockFaces::Right) {
        bundle.vertex_data.extend([
            vertex([-1, -1, 1], [0, 0]),
            vertex([1, -1, 1], [1, 0]),
            vertex([1, 1, 1], [1, 1]),
            vertex([-1, 1, 1], [0, 1]),
        ]);

        push_quad();
    }

    if faces.contains(BlockFaces::Left) {
        bundle.vertex_data.extend([
            vertex([-1, 1, -1], [1, 0]),
            vertex([1, 1, -1], [0, 0]),
            vertex([1, -1, -1], [0, 1]),
            vertex([-1, -1, -1], [1, 1]),
        ]);

        push_quad();
    }

    if faces.contains(BlockFaces::Back) {
        bundle.vertex_data.extend([
            vertex([1, -1, -1], [0, 0]),
            vertex([1, 1, -1], [1, 0]),
            vertex([1, 1, 1], [1, 1]),
            vertex([1, -1, 1], [0, 1]),
        ]);

        push_quad();
    }

    if faces.contains(BlockFaces::Front) {
        bundle.vertex_data.extend([
            vertex([-1, -1, 1], [1, 0]),
            vertex([-1, 1, 1], [0, 0]),
            vertex([-1, 1, -1], [0, 1]),
            vertex([-1, -1, -1], [1, 1]),
        ]);

        push_quad();
    }

    if faces.contains(BlockFaces::Top) {
        bundle.vertex_data.extend([
            vertex([1, 1, -1], [1, 0]),
            vertex([-1, 1, -1], [0, 0]),
            vertex([-1, 1, 1], [0, 1]),
            vertex([1, 1, 1], [1, 1]),
        ]);

        push_quad();
    }

    if faces.contains(BlockFaces::Bottom) {
        bundle.vertex_data.extend([
            vertex([1, -1, 1], [0, 0]),
            vertex([-1, -1, 1], [1, 0]),
            vertex([-1, -1, -1], [1, 1]),
            vertex([1, -1, -1], [0, 1]),
        ]);

        push_quad();
    }

    bundle
}

fn create_texels(size: usize) -> Vec<u8> {
    let mut canvas = Canvas::new(size as i32, size as i32);
    canvas.scale(1.2, 1.2);
    canvas.move_to(36.0, 48.0);
    canvas.quad_to(660.0, 880.0, 100.0, 360.0);
    canvas.translate(10.0, 10.0);
    canvas.set_line_width(20.0);
    canvas.stroke();
    canvas.save();
    canvas.move_to(30.0, 90.0);
    canvas.line_to(110.0, 20.0);
    canvas.line_to(240.0, 130.0);
    canvas.line_to(60.0, 130.0);
    canvas.line_to(190.0, 20.0);
    canvas.line_to(270.0, 90.0);
    canvas.fill();
    canvas.as_bytes().unwrap()
}

struct Example {
    vertex_buffer: wgpu::Buffer,
    index_buffer: wgpu::Buffer,
    index_count: usize,
    bind_group: wgpu::BindGroup,
    uniform_buffer: wgpu::Buffer,
    pipeline: wgpu::RenderPipeline,
    depth_texture_main: wgpu::Texture,
    depth_texture_secondary: wgpu::Texture,
    time: f32,
    camera_controller: CameraController,
    camera_controller_debug: CameraController,
    camera: Camera,
    camera_debug: Camera,
}

fn build_depth_texture(device: &wgpu::Device, size: (u32, u32)) -> wgpu::Texture {
    device.create_texture(&wgpu::TextureDescriptor {
        label: Some("Main depth texture"),
        size: wgpu::Extent3d {
            width: size.0,
            height: size.1,
            depth_or_array_layers: 1,
        },
        format: wgpu::TextureFormat::Depth32Float,
        dimension: wgpu::TextureDimension::D2,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
        mip_level_count: 1,
        sample_count: 1,
        view_formats: &[wgpu::TextureFormat::Depth32Float],
    })
}

impl Example {
    fn init(
        config: &wgpu::SurfaceConfiguration,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        size: (f32, f32),
    ) -> Self {
        let camera = Camera::new(Vec3::new(), Vec2::new(), size.0, size.1);
        let camera_debug = Camera::new(Vec3::new(), Vec2::new(), size.0, size.1);

        let camera_controller = CameraController::new(4.0, 0.004);
        let camera_controller_debug = CameraController::new(4.0, 0.004);

        let (vertex_buf, index_buf, index_count) = {
            let prebuilt = create_vertices(BlockFaces::All);

            let vertex_buf = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("Vertex Buffer"),
                contents: bytemuck::cast_slice(&prebuilt.vertex_data),
                usage: wgpu::BufferUsages::VERTEX,
            });

            let index_buf = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("Index Buffer"),
                contents: bytemuck::cast_slice(&prebuilt.index_data),
                usage: wgpu::BufferUsages::INDEX,
            });

            (vertex_buf, index_buf, prebuilt.index_data.len())
        };

        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: None,
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::VERTEX,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: wgpu::BufferSize::new(64),
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        multisampled: false,
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        view_dimension: wgpu::TextureViewDimension::D2,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 2,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
            ],
        });

        let texture_view = {
            let fractal_size = 256u32;
            let texels = create_texels(fractal_size as usize);

            let texture_extent = wgpu::Extent3d {
                width: fractal_size,
                height: fractal_size,
                depth_or_array_layers: 1,
            };

            let texture = device.create_texture(&wgpu::TextureDescriptor {
                label: None,
                size: texture_extent,
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format: wgpu::TextureFormat::Rgba8Unorm,
                usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
                view_formats: &[wgpu::TextureFormat::Rgba8Unorm],
            });

            queue.write_texture(
                texture.as_image_copy(),
                &texels,
                wgpu::ImageDataLayout {
                    offset: 0,
                    bytes_per_row: Some(fractal_size * 4),
                    rows_per_image: None,
                },
                texture_extent,
            );

            texture.create_view(&wgpu::TextureViewDescriptor::default())
        };

        let uniform_buf = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Uniform Buffer"),
            contents: bytemuck::bytes_of(&Mat4::new()),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("Texture sampler"),
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            mipmap_filter: wgpu::FilterMode::Linear,
            ..Default::default()
        });

        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: uniform_buf.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::TextureView(&texture_view),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: wgpu::BindingResource::Sampler(&sampler),
                },
            ],
            label: None,
        });

        let pipeline = {
            let shader = device.create_shader_module(wgpu::include_wgsl!("../resources/cube.wgsl"));

            let vertex_buffers = [wgpu::VertexBufferLayout {
                array_stride: size_of::<Vertex>() as wgpu::BufferAddress,
                step_mode: wgpu::VertexStepMode::Vertex,
                attributes: &[
                    wgpu::VertexAttribute {
                        format: wgpu::VertexFormat::Float32x4,
                        offset: 0,
                        shader_location: 0,
                    },
                    wgpu::VertexAttribute {
                        format: wgpu::VertexFormat::Float32x2,
                        offset: 4 * 4,
                        shader_location: 1,
                    },
                ],
            }];

            let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: None,
                bind_group_layouts: &[&bind_group_layout],
                push_constant_ranges: &[],
            });

            device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: None,
                layout: Some(&pipeline_layout),
                vertex: wgpu::VertexState {
                    module: &shader,
                    entry_point: "vs_main",
                    buffers: &vertex_buffers,
                },
                fragment: Some(wgpu::FragmentState {
                    module: &shader,
                    entry_point: "fs_main",
                    targets: &[Some(config.format.into())],
                }),
                primitive: wgpu::PrimitiveState {
                    cull_mode: None,
                    ..Default::default()
                },
                depth_stencil: Some(wgpu::DepthStencilState {
                    format: wgpu::TextureFormat::Depth32Float,
                    stencil: wgpu::StencilState::default(),
                    bias: wgpu::DepthBiasState::default(),
                    depth_write_enabled: true,
                    depth_compare: wgpu::CompareFunction::Less,
                }),
                multisample: wgpu::MultisampleState::default(),
                multiview: None,
            })
        };

        let depth_texture_0 = build_depth_texture(device, (size.0 as u32, size.1 as u32));

        let depth_texture_1 = build_depth_texture(device, (512u32, 512u32));

        Example {
            vertex_buffer: vertex_buf,
            index_buffer: index_buf,
            index_count,
            bind_group,
            uniform_buffer: uniform_buf,
            pipeline,
            time: 0.0,
            camera,
            camera_debug,
            camera_controller,
            camera_controller_debug,
            depth_texture_main: depth_texture_0,
            depth_texture_secondary: depth_texture_1,
        }
    }

    fn update(&mut self, delta_time: f32) {
        self.time += delta_time;
    }

    fn setup_dynamic_camera(&self, queue: &wgpu::Queue, camera: &Camera) {
        queue.write_buffer(&self.uniform_buffer, 0, bytemuck::bytes_of(&camera.matrix));
    }

    fn render(
        &mut self,
        color_view: &wgpu::TextureView,
        depth_view: &wgpu::TextureView,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
    ) {
        let mut encoder =
            device.create_command_encoder(&wgpu::CommandEncoderDescriptor { label: None });

        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: None,
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: color_view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color {
                            r: 0.1,
                            g: 0.2,
                            b: 0.3,
                            a: 1.0,
                        }),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: depth_view,
                    depth_ops: Some(wgpu::Operations {
                        load: wgpu::LoadOp::Clear(1.0),
                        store: wgpu::StoreOp::Store,
                    }),
                    stencil_ops: None,
                }),
                timestamp_writes: None,
                occlusion_query_set: None,
            });

            pass.push_debug_group("Prepare data for draw.");
            {
                pass.set_pipeline(&self.pipeline);
                pass.set_bind_group(0, &self.bind_group, &[]);
                pass.set_index_buffer(self.index_buffer.slice(..), wgpu::IndexFormat::Uint16);
                pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
            }
            pass.pop_debug_group();

            pass.insert_debug_marker("Draw!");
            pass.draw_indexed(0..self.index_count as u32, 0, 0..1);
        }

        queue.submit(Some(encoder.finish()));
    }
}

fn get_surface_configuration(size: PhysicalSize<u32>) -> wgpu::SurfaceConfiguration {
    wgpu::SurfaceConfiguration {
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
        format: wgpu::TextureFormat::Bgra8UnormSrgb,
        width: size.width,
        height: size.height,
        present_mode: wgpu::PresentMode::Fifo,
        desired_maximum_frame_latency: 2,
        alpha_mode: wgpu::CompositeAlphaMode::Auto,
        view_formats: vec![wgpu::TextureFormat::Bgra8Unorm],
    }
}

fn process_camera_input(focused: bool, event: Event<()>, camera_controller: &mut CameraController) {
    if !focused {
        return;
    }

    match event {
        Event::WindowEvent { event, .. } => match event {
            WindowEvent::KeyboardInput { event, .. } => {
                camera_controller.process_keyboard(event.physical_key, event.state);
            }
            _ => {}
        },
        Event::DeviceEvent { event, .. } => match event {
            DeviceEvent::MouseMotion { delta } => {
                camera_controller.process_mouse(delta.0, delta.1);
            }
            _ => {}
        },
        _ => {}
    }
}

fn main() -> Result<(), Box<dyn Error>> {
    env_logger::init();

    // Set up window and GPU
    let event_loop = EventLoop::new()?;

    let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
        backends: wgpu::Backends::PRIMARY,
        ..Default::default()
    });

    let window = {
        let size = LogicalSize::new(1280.0, 720.0);

        WindowBuilder::new()
            .with_inner_size(size)
            .with_title(&"imgui-wgpu".to_string())
            .build(&event_loop)?
    };

    let surface = instance.create_surface(&window)?;

    let adapter = block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
        power_preference: wgpu::PowerPreference::HighPerformance,
        compatible_surface: Some(&surface),
        force_fallback_adapter: false,
    }))
    .unwrap();

    let (device, queue) =
        block_on(adapter.request_device(&wgpu::DeviceDescriptor::default(), None)).unwrap();

    let size = window.inner_size();
    let surface_configuration = get_surface_configuration(size);
    surface.configure(&device, &surface_configuration);

    let mut imgui = Context::create();
    imgui.set_ini_filename(None);
    let mut platform = imgui_winit_support::WinitPlatform::init(&mut imgui);
    platform.attach_window(
        imgui.io_mut(),
        &window,
        imgui_winit_support::HiDpiMode::Default,
    );

    {
        let hidpi_factor = window.scale_factor();
        let font_size = (13.0 * hidpi_factor) as f32;

        imgui.io_mut().font_global_scale = (1.0 / hidpi_factor) as f32;

        imgui.fonts().add_font(&[FontSource::DefaultFontData {
            config: Some(FontConfig {
                oversample_h: 1,
                pixel_snap_h: true,
                size_pixels: font_size,
                ..Default::default()
            }),
        }]);
    }

    // Requires font setup
    let mut renderer = imgui_wgpu::Renderer::new(
        &mut imgui,
        &device,
        &queue,
        imgui_wgpu::RendererConfig {
            texture_format: surface_configuration.format,
            ..Default::default()
        },
    );

    let mut last_frame = Instant::now();
    let mut example_size: [f32; 2] = [640.0, 480.0];
    let size = window.inner_size();
    let mut example = Example::init(
        &surface_configuration,
        &device,
        &queue,
        (size.width as f32, size.height as f32),
    );

    let example_texture_id = {
        let texture_config = imgui_wgpu::TextureConfig {
            size: wgpu::Extent3d {
                width: example_size[0] as u32,
                height: example_size[1] as u32,
                ..Default::default()
            },
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
            ..Default::default()
        };
        let texture = imgui_wgpu::Texture::new(&device, &renderer, texture_config);
        renderer.textures.insert(texture)
    };

    let mut focused = false;

    let mut use_debug_camera = false;

    event_loop.run(|event, window_target| {
        window_target.set_control_flow(ControlFlow::Poll);

        let imgui_io = imgui.io();
        let imgui_wants_mouse = imgui_io.want_capture_mouse;

        if use_debug_camera {
            process_camera_input(focused, event.clone(), &mut example.camera_controller_debug);
        } else {
            process_camera_input(focused, event.clone(), &mut example.camera_controller);
        }

        match event {
            Event::AboutToWait => {
                window.request_redraw();
            }
            Event::WindowEvent { ref event, .. } => match event {
                WindowEvent::Resized(size) => {
                    let surface_desc = get_surface_configuration(*size);

                    example.depth_texture_main =
                        build_depth_texture(&device, (size.width, size.height));

                    surface.configure(&device, &surface_desc);

                    example.camera.resize(size.width as f32, size.height as f32);
                }
                WindowEvent::CloseRequested => {
                    window_target.exit();
                }
                WindowEvent::KeyboardInput { event, .. } => {
                    if let Key::Named(NamedKey::Escape) = event.logical_key {
                        if event.state.is_pressed() {
                            window.set_cursor_grab(CursorGrabMode::None).unwrap();
                            window.set_cursor_visible(true);
                            focused = false;
                        }
                    }

                    if let Key::Named(NamedKey::Tab) = event.logical_key {
                        if event.state.is_pressed() {
                            use_debug_camera = !use_debug_camera;
                        }
                    }
                }
                WindowEvent::MouseInput {
                    state: ElementState::Pressed,
                    button: winit::event::MouseButton::Left,
                    ..
                } => {
                    if !imgui_wants_mouse {
                        window.set_cursor_grab(CursorGrabMode::Locked).unwrap();
                        window.set_cursor_visible(false);
                        focused = true;
                    }
                }
                WindowEvent::RedrawRequested => {
                    let now = Instant::now();
                    let delta = now - last_frame;
                    last_frame = now;

                    imgui.io_mut().update_delta_time(delta);

                    example
                        .camera_controller
                        .update_camera(&mut example.camera, delta);

                    example.camera.compute();

                    platform
                        .prepare_frame(imgui.io_mut(), &window)
                        .expect("Failed to prepare frame");

                    let frame = match surface.get_current_texture() {
                        Ok(frame) => frame,
                        Err(e) => {
                            eprintln!("dropped frame: {e:?}");
                            return;
                        }
                    };

                    let view = frame
                        .texture
                        .create_view(&wgpu::TextureViewDescriptor::default());

                    let main_depth_view = example
                        .depth_texture_main
                        .create_view(&wgpu::TextureViewDescriptor::default());

                    let ui = imgui.frame();
                    example.update(ui.io().delta_time);
                    example.setup_dynamic_camera(&queue, &example.camera);
                    example.render(&view, &main_depth_view, &device, &queue);

                    let mut new_example_size: Option<[f32; 2]> = None;

                    ui.window("Cube")
                        .size([512.0, 512.0], Condition::FirstUseEver)
                        .build(|| {
                            new_example_size = Some(ui.content_region_avail());
                            Image::new(example_texture_id, new_example_size.unwrap()).build(ui);
                        });

                    ui.window("Camera")
                        .size([170.0, 260.0], Condition::FirstUseEver)
                        .position([1070.0, 12.0], Condition::FirstUseEver)
                        .build(|| {
                            ui.text(format!(
                                "Main Camera {}",
                                if !use_debug_camera { "(active)" } else { "" }
                            ));
                            ui.text(format!("   X {}", example.camera.view.position.x));
                            ui.text(format!("   Y {}", example.camera.view.position.y));
                            ui.text(format!("   Z {}", example.camera.view.position.z));
                            ui.text(format!("   Pitch {}", example.camera.view.yaw_pitch.x));
                            ui.text(format!("   Yaw {}", example.camera.view.yaw_pitch.y));
                            ui.text(format!(
                                "Debug Camera {}",
                                if use_debug_camera { "(active)" } else { "" }
                            ));
                            ui.text(format!("   X {}", example.camera_debug.view.position.x));
                            ui.text(format!("   Y {}", example.camera_debug.view.position.y));
                            ui.text(format!("   Z {}", example.camera_debug.view.position.z));
                            ui.text(format!(
                                "   Pitch {}",
                                example.camera_debug.view.yaw_pitch.x
                            ));
                            ui.text(format!("   Yaw {}", example.camera_debug.view.yaw_pitch.y));
                        });

                    if let Some(size) = new_example_size {
                        if size != example_size && size[0] >= 1.0 && size[1] >= 1.0 {
                            example_size = size;
                            let scale = &ui.io().display_framebuffer_scale;
                            let texture_config = imgui_wgpu::TextureConfig {
                                size: wgpu::Extent3d {
                                    width: (example_size[0] * scale[0]) as u32,
                                    height: (example_size[1] * scale[1]) as u32,
                                    ..Default::default()
                                },
                                usage: wgpu::TextureUsages::RENDER_ATTACHMENT
                                    | wgpu::TextureUsages::TEXTURE_BINDING,
                                ..Default::default()
                            };
                            renderer.textures.replace(
                                example_texture_id,
                                imgui_wgpu::Texture::new(&device, &renderer, texture_config),
                            );

                            example
                                .camera_debug
                                .resize(example_size[0] * scale[0], example_size[1] * scale[1]);

                            example.depth_texture_secondary = build_depth_texture(
                                &device,
                                (
                                    (example_size[0] * scale[0]) as u32,
                                    (example_size[1] * scale[1]) as u32,
                                ),
                            );
                        }

                        example
                            .camera_controller_debug
                            .update_camera(&mut example.camera_debug, delta);
                        example.camera_debug.compute();
                        example.setup_dynamic_camera(&queue, &example.camera_debug);

                        example.render(
                            renderer.textures.get(example_texture_id).unwrap().view(),
                            &example
                                .depth_texture_secondary
                                .create_view(&wgpu::TextureViewDescriptor::default()),
                            &device,
                            &queue,
                        );
                    }

                    let mut encoder: wgpu::CommandEncoder = device
                        .create_command_encoder(&wgpu::CommandEncoderDescriptor { label: None });

                    platform.prepare_render(ui, &window);

                    let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                        label: None,
                        color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                            view: &view,
                            resolve_target: None,
                            ops: wgpu::Operations {
                                load: wgpu::LoadOp::Load,
                                store: wgpu::StoreOp::Store,
                            },
                        })],
                        depth_stencil_attachment: None,
                        timestamp_writes: None,
                        occlusion_query_set: None,
                    });

                    renderer
                        .render(imgui.render(), &queue, &device, &mut pass)
                        .expect("Rendering failed");

                    drop(pass);

                    queue.submit(Some(encoder.finish()));
                    frame.present();
                }
                _ => {}
            },
            _ => {}
        }

        platform.handle_event(imgui.io_mut(), &window, &event);
    })?;

    Ok(())
}
