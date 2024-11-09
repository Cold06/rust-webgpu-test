use bytemuck::NoUninit;
use crate::camera::Camera;
use crate::multimath::Vec4;
use crate::pipelines::video;
use crate::video::Resolution;

pub struct VideoExample {
    bind_group_0: video::BindGroup0,
    bind_group_1: video::BindGroup1,
    pipeline: video::Pipeline,
    vertex_format: video::VertexFormat,
    width: u32,
    height: u32,
}

fn generate_quad() -> video::ModelBundle {
    let s = 0.05;
    let h = 1080.0 * s;
    let w = 1920.0 * s;

    let (x, y, z) = (3.0, 0.0, 0.0);

    video::ModelBundle {
        vertex_data: vec![
            video::Vertex::new([x, -h + y,  w + z],  [1, 1]),
            video::Vertex::new([x,  h + y,  w + z],  [1, 0]),
            video::Vertex::new([x,  h + y, -w + z],  [0, 0]),
            video::Vertex::new([x, -h + y, -w + z],  [0, 1]),
        ],
        index_data: vec![0, 1, 2, 2, 3, 0],
    }
}

impl VideoExample {
    pub fn create(
        config: &wgpu::SurfaceConfiguration,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
    ) -> Self {
        let bind_group_0 = video::BindGroup0::create(device, queue, 512, 521);
        let bind_group_1 = video::BindGroup1::create(device, Vec4::new());
        let pipeline = video::Pipeline::create(device, config.format);

        let vertex_format = video::VertexFormat::create(device, &generate_quad());

        Self {
            width: 512,
            height: 512,
            bind_group_0,
            bind_group_1,
            pipeline,
            vertex_format,
        }
    }

    pub fn update_texture<T: NoUninit>(&self, queue: &wgpu::Queue, y_data: &[T],u_data: &[T],v_data: &[T]) {
        self.bind_group_0.update_texture(queue, y_data, u_data, v_data);
    }

    pub(crate) fn check_resize(&mut self, device: &wgpu::Device, queue: &wgpu::Queue, resolution: Resolution) {
        if (resolution.width != self.width as usize) || (resolution.height != self.height as usize) {
            self.bind_group_0.resize_textures(device, queue, resolution.width as u32, resolution.height as u32);
            self.width = resolution.width as u32;
            self.height = resolution.height as u32;
        }
    }

    pub fn setup_dynamic_camera(&self, queue: &wgpu::Queue, camera: &Camera) {
        self.bind_group_0.update_globals(queue, camera);
    }

    pub fn render<'a>(&'a mut self, pass: &mut wgpu::RenderPass<'a>) {
        pass.set_pipeline(&self.pipeline.pipeline);
        pass.set_bind_group(0, &self.bind_group_0.bind_group, &[]);
        pass.set_bind_group(1, &self.bind_group_1.bind_group, &[]);
        pass.set_index_buffer(
            self.vertex_format.index_buffer.slice(..),
            wgpu::IndexFormat::Uint32,
        );
        pass.set_vertex_buffer(0, self.vertex_format.vertex_buffer.slice(..));
        pass.draw_indexed(0..self.vertex_format.index_count, 0, 0..1);
    }
}
