use crate::camera::Camera;
use crate::chunk::Chunk;
use crate::gpu::GPUCtx;
use crate::paint_utils::create_texels;
use crate::pipelines::quad_mesh;
use egui_wgpu::wgpu;

pub struct ChunksDemo {
    chunks: Vec<Chunk>,
    bind_group: quad_mesh::BindGroup0,
    pipeline: quad_mesh::Pipeline,
    pub last_spawn_x: i32,
}

impl ChunksDemo {
    pub fn init(
        config: &wgpu::SurfaceConfiguration,
        ctx: &GPUCtx,
    ) -> Self {
        let fractal_size = 256u32;
        let texels = create_texels(fractal_size as usize);

        let bind_group = quad_mesh::BindGroup0::create(ctx, fractal_size, texels);
        let pipeline = quad_mesh::Pipeline::create(ctx, config.format);

        ChunksDemo {
            last_spawn_x: 1,
            chunks: vec![],
            bind_group,
            pipeline,
        }
    }

    pub fn spawn_chunk(&mut self, ctx: &GPUCtx) {
        for x in 0..8 {
            for y in 0..6 {
                for z in 0..8 {
                    let chunk = Chunk::new(ctx, x, y, z);
                    self.chunks.push(chunk);
                }
            }
        }
    }

    pub fn setup_dynamic_camera(&self, ctx: &GPUCtx, camera: &Camera) {
        self.bind_group.update_globals(ctx, camera);
    }

    pub fn render<'a>(
        &'a mut self,
        pass: &mut wgpu::RenderPass<'a>,
    ) {
        for chunk in &self.chunks {
            pass.set_pipeline(&self.pipeline.pipeline);
            pass.set_bind_group(0, &self.bind_group.bind_group, &[]);
            pass.set_bind_group(1, &chunk.bind_group.bind_group, &[]);
            pass.set_index_buffer(
                chunk.vertex_format.index_buffer.slice(..),
                wgpu::IndexFormat::Uint32,
            );
            pass.set_vertex_buffer(0, chunk.vertex_format.vertex_buffer.slice(..));
            pass.draw_indexed(0..chunk.index_count as u32, 0, 0..1);
        }
    }
}
