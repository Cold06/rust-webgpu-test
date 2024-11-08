use crate::camera::Camera;
use crate::camera_controller::CameraController;
use crate::chunk::Chunk;
use crate::pipelines::Vertex;
use crate::gpu_utils::build_depth_texture;
use crate::multimath::{Mat4, Vec2, Vec3};
use crate::paint_utils::create_texels;
use crate::pipelines::{BindGroup0, BindGroup1, VertexFormat};
use std::mem::offset_of;
use wgpu::util::DeviceExt;
use wgpu::{Device, Face, PrimitiveTopology};

pub struct Example {
    chunks: Vec<Chunk>,
    bind_group: BindGroup0,
    uniform_buffer: wgpu::Buffer,
    pipeline: wgpu::RenderPipeline,
    time: f32,
    pub depth_texture_main: wgpu::Texture,
    pub depth_texture_secondary: wgpu::Texture,
    pub camera_controller: CameraController,
    pub camera_controller_debug: CameraController,
    pub camera: Camera,
    pub camera_debug: Camera,
    pub last_spawn_x: i32,
}

impl Example {
    pub fn init(
        config: &wgpu::SurfaceConfiguration,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        size: (f32, f32),
    ) -> Self {
        let camera = Camera::new(
            Vec3::from_components(27.0, 30.0, -11.0),
            Vec2::from_components(1.04, -0.58),
            size.0,
            size.1,
        );
        let camera_debug = Camera::new(Vec3::new(), Vec2::new(), size.0, size.1);

        let mut camera_controller = CameraController::new(20.0, 0.004);
        camera_controller.copy_camera_rotation(&camera);
        let mut camera_controller_debug = CameraController::new(200.0, 0.004);
        camera_controller.copy_camera_rotation(&camera_debug);

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
            mag_filter: wgpu::FilterMode::Nearest,
            min_filter: wgpu::FilterMode::Nearest,
            mipmap_filter: wgpu::FilterMode::Nearest,
            ..Default::default()
        });

        let bind_group = BindGroup0::create(device, &uniform_buf, &texture_view, &sampler);

        let pipeline = {
            let shader = device.create_shader_module(wgpu::include_wgsl!("../resources/cube.wgsl"));

            let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: None,
                bind_group_layouts: &[
                    &BindGroup0::get_layout(device),
                    &BindGroup1::get_layout(&device),
                ],
                push_constant_ranges: &[],
            });

            device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: None,
                layout: Some(&pipeline_layout),
                vertex: wgpu::VertexState {
                    module: &shader,
                    entry_point: "vs_main",
                    buffers: &VertexFormat::LAYOUT,
                },
                fragment: Some(wgpu::FragmentState {
                    module: &shader,
                    entry_point: "fs_main",
                    targets: &[Some(config.format.into())],
                }),
                primitive: wgpu::PrimitiveState {
                    cull_mode: Some(Face::Back),
                    topology: PrimitiveTopology::TriangleList,
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

        let mut e = Example {
            last_spawn_x: 1,
            chunks: vec![],
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
        };

        e.spawn_chunk(&device);

        e
    }

    pub fn spawn_chunk(&mut self, device: &Device) {
        for x in 0..8 {
            for y in 0..6 {
                for z in 0..8 {
                    let chunk = Chunk::new(device, x, y, z);
                    self.chunks.push(chunk);
                }
            }
        }
    }

    pub fn update(&mut self, delta_time: f32) {
        self.time += delta_time;
    }

    pub fn setup_dynamic_camera(&self, queue: &wgpu::Queue, camera: &Camera) {
        queue.write_buffer(&self.uniform_buffer, 0, bytemuck::bytes_of(&camera.matrix));
    }

    pub fn render(
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

            for chunk in &self.chunks {
                pass.push_debug_group("Prepare data for draw.");
                {
                    pass.set_pipeline(&self.pipeline);
                    pass.set_bind_group(0, &self.bind_group.bind_group, &[]);
                    pass.set_bind_group(1, &chunk.bind_group.bind_group, &[]);
                    pass.set_index_buffer(chunk.vertex_format.index_buffer.slice(..), wgpu::IndexFormat::Uint32);
                    pass.set_vertex_buffer(0, chunk.vertex_format.vertex_buffer.slice(..));
                }
                pass.pop_debug_group();
                pass.insert_debug_marker("Draw!");
                pass.draw_indexed(0..chunk.index_count as u32, 0, 0..1);
            }
        }

        queue.submit(Some(encoder.finish()));
    }
}
