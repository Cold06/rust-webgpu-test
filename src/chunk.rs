use crate::cube::generate_full_mesh;
use crate::multimath::Vec4;
use wgpu::util::DeviceExt;
use wgpu::{BindGroupDescriptor, Device};

pub struct Chunk {
    pub vertex_buffer: wgpu::Buffer,
    pub index_buffer: wgpu::Buffer,
    position_buffer: wgpu::Buffer,
    pub bind_group: wgpu::BindGroup,
    pub index_count: usize,
}

impl Chunk {
    pub fn get_bind_group_layout(device: &Device) -> wgpu::BindGroupLayout {
        device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
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
        })
    }
    pub fn new(device: &Device, x: i32, y: i32, z: i32) -> Self {
        let model = generate_full_mesh(x * 16, y * 16, z * 16);

        let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Vertex Buffer"),
            contents: bytemuck::cast_slice(&model.vertex_data),
            usage: wgpu::BufferUsages::VERTEX,
        });

        let index_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Index Buffer"),
            contents: bytemuck::cast_slice(&model.index_data),
            usage: wgpu::BufferUsages::INDEX,
        });

        let pos = (x as f32 * 32.0, y as f32 * 32.0, z as f32 * 32.0, 0.0);

        let position_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Position"),
            contents: bytemuck::bytes_of(&Vec4::from_components(pos.0, pos.1, pos.2, pos.3)),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        let bind_group = device.create_bind_group(&BindGroupDescriptor {
            layout: &Chunk::get_bind_group_layout(device),
            label: Some("Instance Bind Group"),
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: position_buffer.as_entire_binding(),
            }],
        });

        Chunk {
            position_buffer,
            index_buffer,
            vertex_buffer,
            index_count: model.index_data.len(),
            bind_group,
        }
    }
}
