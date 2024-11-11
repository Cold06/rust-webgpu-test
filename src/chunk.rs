use crate::cube::generate_full_mesh;
use crate::gpu::GPUCtx;
use crate::pipelines::quad_mesh;
use glam::*;

pub struct Chunk {
    pub vertex_format: quad_mesh::VertexFormat,
    pub bind_group: quad_mesh::BindGroup1,
    pub index_count: usize,
}

impl Chunk {
    pub fn new(ctx: &GPUCtx, x: i32, y: i32, z: i32) -> Self {
        let model = generate_full_mesh(x * 16, y * 16, z * 16);

        let vertex_format = quad_mesh::VertexFormat::create(ctx, &model);

        let bind_group = quad_mesh::BindGroup1::create(
            ctx,
            Vec4::new(x as f32 * 32.0, y as f32 * 32.0, z as f32 * 32.0, 0.0),
        );

        Chunk {
            vertex_format,
            index_count: model.index_data.len(),
            bind_group,
        }
    }
}
