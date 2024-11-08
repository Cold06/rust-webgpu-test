use crate::camera::Camera;
use crate::chunk::Chunk;
use crate::gpu::SView;
use crate::paint_utils::create_texels;
use crate::pipelines::quad_mesh;

pub struct Example {
    chunks: Vec<Chunk>,
    bind_group: quad_mesh::BindGroup0,
    pipeline: quad_mesh::Pipeline,
    time: f32,
    pub last_spawn_x: i32,
}

impl Example {
    pub fn init(
        config: &wgpu::SurfaceConfiguration,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
    ) -> Self {
        let fractal_size = 256u32;
        let texels = create_texels(fractal_size as usize);

        let bind_group = quad_mesh::BindGroup0::create(device, queue, fractal_size, texels);
        let pipeline = quad_mesh::Pipeline::create(device, config.format);

        let mut e = Example {
            last_spawn_x: 1,
            chunks: vec![],
            bind_group,
            pipeline,
            time: 0.0,
        };

        e.spawn_chunk(&device);

        e
    }

    pub fn spawn_chunk(&mut self, device: &wgpu::Device) {
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
        self.bind_group.update_globals(queue, camera);
    }

    pub fn render(&mut self, device: &wgpu::Device, queue: &wgpu::Queue, view: &SView) {
        let mut encoder =
            device.create_command_encoder(&wgpu::CommandEncoderDescriptor { label: None });

        {
            let mut pass = view.render_pass(&mut encoder);

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

        queue.submit(Some(encoder.finish()));
    }
}
