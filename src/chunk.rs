use crate::cube::generate_full_mesh;
use crate::multimath::Vec4;
use wgpu::util::DeviceExt;
use wgpu::Device;
use crate::pipelines::{BindGroup1, VertexFormat};

pub struct Chunk {
    pub vertex_format: VertexFormat,
    position_buffer: wgpu::Buffer,
    pub bind_group: BindGroup1,
    pub index_count: usize,
}

impl Chunk {

    pub fn new(device: &Device, x: i32, y: i32, z: i32) -> Self {
        let model = generate_full_mesh(x * 16, y * 16, z * 16);

        let vertex_format = VertexFormat::create(device, &model);

        let pos = (x as f32 * 32.0, y as f32 * 32.0, z as f32 * 32.0, 0.0);

        let position_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Position"),
            contents: bytemuck::bytes_of(&Vec4::from_components(pos.0, pos.1, pos.2, pos.3)),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        let bind_group = BindGroup1::create(device, &position_buffer);

        Chunk {
            position_buffer,
            vertex_format,
            index_count: model.index_data.len(),
            bind_group,
        }
    }
}
