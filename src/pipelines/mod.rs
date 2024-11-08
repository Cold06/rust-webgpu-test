use std::mem::offset_of;
use bytemuck::{Pod, Zeroable};
use wgpu::{BindGroupDescriptor, Buffer, Device, Sampler, TextureView};
use wgpu::util::DeviceExt;
use crate::multimath::Vec4;

pub mod quad_mesh;

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable, Default)]
pub struct Vertex {
    pub pos: [f32; 4],
    pub normal: [f32; 4],
    pub tex_coord: [f32; 2],
}

pub fn vertex(pos: [f32; 3], normal: [i8; 3], tc: [i8; 2]) -> Vertex {
    Vertex {
        pos: [pos[0] as f32, pos[1] as f32, pos[2] as f32, 1.0],
        normal: [normal[0] as f32, normal[1] as f32, normal[2] as f32, 0.0],
        tex_coord: [tc[0] as f32, tc[1] as f32],
    }
}

pub struct ModelBundle {
    pub vertex_data: Vec<Vertex>,
    pub index_data: Vec<u32>,
}


pub struct VertexFormat {
    pub vertex_buffer: Buffer,
    pub index_buffer: Buffer,
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
                format: wgpu::VertexFormat::Float32x4,
                offset: offset_of! {Vertex, normal} as u64,
                shader_location: 1,
            },
            wgpu::VertexAttribute {
                format: wgpu::VertexFormat::Float32x2,
                offset: offset_of!(Vertex, tex_coord) as u64,
                shader_location: 2,
            },
        ],
    }];

    pub fn create(device: &Device, model_bundle: &ModelBundle) -> Self {
        let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: None,
            contents: bytemuck::cast_slice(&model_bundle.vertex_data),
            usage: wgpu::BufferUsages::VERTEX,
        });

        let index_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: None,
            contents: bytemuck::cast_slice(&model_bundle.index_data),
            usage: wgpu::BufferUsages::INDEX,
        });

        Self {
            index_buffer,
            vertex_buffer,
        }
    }
}

pub struct BindGroup0 {
    pub bind_group: wgpu::BindGroup,
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
    };

    pub fn get_layout(device: &Device) -> wgpu::BindGroupLayout {
        device.create_bind_group_layout(&Self::LAYOUT)
    }

    pub fn create(
        device: &Device,
        uniform_buffer: &Buffer,
        texture_view: &TextureView,
        sampler: &Sampler,
    ) -> Self {
        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &Self::get_layout(device),
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: uniform_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::TextureView(texture_view),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: wgpu::BindingResource::Sampler(sampler),
                },
            ],
            label: None,
        });

        Self { bind_group }
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

    pub fn get_layout(device: &Device) -> wgpu::BindGroupLayout {
        device.create_bind_group_layout(&Self::LAYOUT)
    }

    pub fn create(device: &Device, position: Vec4) -> Self {
        let position_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: None,
            contents: bytemuck::bytes_of(&position),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        let bind_group = device.create_bind_group(&BindGroupDescriptor {
            layout: &BindGroup1::get_layout(device),
            label: None,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: position_buffer.as_entire_binding(),
            }],
        });

        Self { bind_group }
    }
}
