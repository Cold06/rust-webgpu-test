use winit::dpi::PhysicalSize;

pub fn build_depth_texture(device: &wgpu::Device, size: (u32, u32)) -> wgpu::Texture {
    device.create_texture(&wgpu::TextureDescriptor {
        label: Some("Main depth texture"),
        size: wgpu::Extent3d {
            width: size.0,
            height: size.1,
            depth_or_array_layers: 1,
        },
        format: wgpu::TextureFormat::Depth32Float,
        dimension: wgpu::TextureDimension::D2,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
        mip_level_count: 1,
        sample_count: 1,
        view_formats: &[wgpu::TextureFormat::Depth32Float],
    })
}

pub fn get_surface_configuration(size: PhysicalSize<u32>) -> wgpu::SurfaceConfiguration {
    wgpu::SurfaceConfiguration {
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
        format: wgpu::TextureFormat::Bgra8UnormSrgb,
        width: size.width,
        height: size.height,
        present_mode: wgpu::PresentMode::Fifo,
        desired_maximum_frame_latency: 2,
        alpha_mode: wgpu::CompositeAlphaMode::Auto,
        view_formats: vec![wgpu::TextureFormat::Bgra8Unorm],
    }
}
