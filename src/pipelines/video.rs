use crate::camera::Camera;
use crate::gpu::{GPUCtx, GPUSampler, GPUTexture};
use crate::multimath::{Mat4Bytes, Vec4Bytes};
use bytemuck::{NoUninit, Pod, Zeroable};
use egui_wgpu::wgpu;
use glam::*;
use std::mem::offset_of;
use wgpu::util::DeviceExt;

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable, Default)]
pub struct Vertex {
    pub pos: [f32; 4],
    pub tex_coord: [f32; 2],
}

impl Vertex {
    pub fn new(pos: [f32; 3], tc: [i8; 2]) -> Vertex {
        Vertex {
            pos: [pos[0], pos[1], pos[2], 1.0],
            tex_coord: [tc[0] as f32, tc[1] as f32],
        }
    }
}

pub struct ModelBundle {
    pub vertex_data: Vec<Vertex>,
    pub index_data: Vec<u32>,
}

pub struct VertexFormat {
    pub vertex_buffer: wgpu::Buffer,
    pub index_buffer: wgpu::Buffer,
    pub index_count: u32,
}

impl VertexFormat {
    pub const LAYOUT: [wgpu::VertexBufferLayout<'static>; 1] = [wgpu::VertexBufferLayout {
        array_stride: size_of::<Vertex>() as wgpu::BufferAddress,
        step_mode: wgpu::VertexStepMode::Vertex,
        attributes: &[
            wgpu::VertexAttribute {
                format: wgpu::VertexFormat::Float32x4,
                offset: offset_of![Vertex, pos] as u64,
                shader_location: 0,
            },
            wgpu::VertexAttribute {
                format: wgpu::VertexFormat::Float32x2,
                offset: offset_of!(Vertex, tex_coord) as u64,
                shader_location: 1,
            },
        ],
    }];

    pub fn create(ctx: &GPUCtx, model_bundle: &ModelBundle) -> Self {
        let vertex_buffer = ctx
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: None,
                contents: bytemuck::cast_slice(&model_bundle.vertex_data),
                usage: wgpu::BufferUsages::VERTEX,
            });

        let index_buffer = ctx
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: None,
                contents: bytemuck::cast_slice(&model_bundle.index_data),
                usage: wgpu::BufferUsages::INDEX,
            });

        Self {
            index_buffer,
            vertex_buffer,
            index_count: model_bundle.index_data.len() as u32,
        }
    }
}

pub struct BindGroup0 {
    pub bind_group: wgpu::BindGroup,
    uniform_buffer: wgpu::Buffer,

    y_texture: GPUTexture,
    u_texture: GPUTexture,
    v_texture: GPUTexture,

    sampler: GPUSampler,
}

impl BindGroup0 {
    pub fn update_texture<T: NoUninit>(
        &self,
        ctx: &GPUCtx,
        y_data: &[T],
        u_data: &[T],
        v_data: &[T],
    ) {
        self.y_texture.update(&ctx, y_data);
        self.u_texture.update(&ctx, u_data);
        self.v_texture.update(&ctx, v_data);
    }
    pub(crate) fn resize_textures(&mut self, ctx: &GPUCtx, width: u32, height: u32) {
        let y_width = width;
        let y_height = height;
        let uv_width = width / 2;
        let uv_height = height / 2;

        self.y_texture.resize(&ctx, y_width, y_height, 0u8);
        self.u_texture.resize(&ctx, uv_width, uv_height, 0u8);
        self.v_texture.resize(&ctx, uv_width, uv_height, 0u8);

        self.bind_group = Self::generate_bind_group(
            ctx,
            &self.uniform_buffer,
            &self.sampler,
            &self.y_texture,
            &self.u_texture,
            &self.v_texture,
        );
    }
}

impl BindGroup0 {
    const LAYOUT: wgpu::BindGroupLayoutDescriptor<'static> = wgpu::BindGroupLayoutDescriptor {
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
            GPUSampler::get_layout(1),
            GPUTexture::get_layout(2),
            GPUTexture::get_layout(3),
            GPUTexture::get_layout(4),
        ],
    };

    pub fn get_layout(ctx: &GPUCtx) -> wgpu::BindGroupLayout {
        ctx.device.create_bind_group_layout(&Self::LAYOUT)
    }

    pub fn update_globals(&self, ctx: &GPUCtx, camera: &Camera) {
        ctx.queue
            .write_buffer(&self.uniform_buffer, 0, Mat4Bytes(camera.matrix).as_bytes());
    }

    pub fn create(ctx: &GPUCtx, width: u32, height: u32) -> Self {
        let uv_width = width / 2;
        let uv_height = height / 2;

        let usage = wgpu::TextureUsages::RENDER_ATTACHMENT
            | wgpu::TextureUsages::COPY_DST
            | wgpu::TextureUsages::COPY_SRC
            | wgpu::TextureUsages::TEXTURE_BINDING;

        let y_texture =
            GPUTexture::create(ctx, width, height, wgpu::TextureFormat::R8Unorm, 0u8, usage);
        let u_texture = GPUTexture::create(
            ctx,
            uv_width,
            uv_height,
            wgpu::TextureFormat::R8Unorm,
            0u8,
            usage,
        );
        let v_texture = GPUTexture::create(
            ctx,
            uv_width,
            uv_height,
            wgpu::TextureFormat::R8Unorm,
            0u8,
            usage,
        );

        let uniform_buffer = ctx
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: None,
                contents: Mat4Bytes(Mat4::IDENTITY).as_bytes(),
                usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            });

        let sampler = GPUSampler::create(
            &ctx,
            wgpu::FilterMode::Linear,
            wgpu::AddressMode::ClampToEdge,
        );

        let bind_group = Self::generate_bind_group(
            ctx,
            &uniform_buffer,
            &sampler,
            &y_texture,
            &u_texture,
            &v_texture,
        );

        Self {
            bind_group,
            uniform_buffer,
            y_texture,
            u_texture,
            v_texture,
            sampler,
        }
    }

    fn generate_bind_group(
        ctx: &GPUCtx,
        uniform_buffer: &wgpu::Buffer,
        sampler: &GPUSampler,
        y_texture_view: &GPUTexture,
        u_texture_view: &GPUTexture,
        v_texture_view: &GPUTexture,
    ) -> wgpu::BindGroup {
        ctx.device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &Self::get_layout(ctx),
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: uniform_buffer.as_entire_binding(),
                },
                sampler.get_binding(1),
                y_texture_view.get_binding(2),
                u_texture_view.get_binding(3),
                v_texture_view.get_binding(4),
            ],
            label: None,
        })
    }
}

pub struct BindGroup1 {
    pub bind_group: wgpu::BindGroup,
}

impl BindGroup1 {
    const LAYOUT: wgpu::BindGroupLayoutDescriptor<'static> = wgpu::BindGroupLayoutDescriptor {
        label: None,
        entries: &[wgpu::BindGroupLayoutEntry {
            binding: 0,
            visibility: wgpu::ShaderStages::VERTEX,
            ty: wgpu::BindingType::Buffer {
                ty: wgpu::BufferBindingType::Uniform,
                has_dynamic_offset: false,
                min_binding_size: wgpu::BufferSize::new(16),
            },
            count: None,
        }],
    };

    pub fn get_layout(ctx: &GPUCtx) -> wgpu::BindGroupLayout {
        ctx.device.create_bind_group_layout(&Self::LAYOUT)
    }

    pub fn create(ctx: &GPUCtx, position: Vec4) -> Self {
        let position_buffer = ctx
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: None,
                contents: Vec4Bytes(position).as_bytes(),
                usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            });

        let bind_group = ctx.device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &BindGroup1::get_layout(ctx),
            label: None,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: position_buffer.as_entire_binding(),
            }],
        });

        Self { bind_group }
    }
}

pub struct Pipeline {
    pub pipeline: wgpu::RenderPipeline,
}

impl Pipeline {
    pub fn create(ctx: &GPUCtx, format: wgpu::TextureFormat) -> Self {
        let shader = ctx
            .device
            .create_shader_module(wgpu::include_wgsl!("./video.wgsl"));

        let pipeline_layout = ctx
            .device
            .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: None,
                bind_group_layouts: &[&BindGroup0::get_layout(ctx), &BindGroup1::get_layout(ctx)],
                push_constant_ranges: &[],
            });

        let pipeline = ctx
            .device
            .create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: None,
                layout: Some(&pipeline_layout),
                cache: None,
                vertex: wgpu::VertexState {
                    module: &shader,
                    entry_point: "vs_main",
                    buffers: &VertexFormat::LAYOUT,
                    compilation_options: Default::default(),
                },
                fragment: Some(wgpu::FragmentState {
                    module: &shader,
                    entry_point: "fs_main",
                    targets: &[Some(format.into())],
                    compilation_options: Default::default(),
                }),
                primitive: wgpu::PrimitiveState {
                    cull_mode: Some(wgpu::Face::Back),
                    topology: wgpu::PrimitiveTopology::TriangleList,
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
            });

        Self { pipeline }
    }
}
