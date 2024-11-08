use crate::cube::generate_full_mesh;
use crate::multimath::Vec4;
use crate::pipelines::{BindGroup1, VertexFormat};
use wgpu::util::DeviceExt;
use wgpu::Device;

pub struct Chunk {
    pub vertex_format: VertexFormat,
    pub bind_group: BindGroup1,
    pub index_count: usize,
}

impl Chunk {
    pub fn new(device: &Device, x: i32, y: i32, z: i32) -> Self {
        let model = generate_full_mesh(x * 16, y * 16, z * 16);

        let vertex_format = VertexFormat::create(device, &model);

        let bind_group = BindGroup1::create(
            device,
            Vec4::from_components(x as f32 * 32.0, y as f32 * 32.0, z as f32 * 32.0, 0.0),
        );

        Chunk {
            vertex_format,
            index_count: model.index_data.len(),
            bind_group,
        }
    }
}
