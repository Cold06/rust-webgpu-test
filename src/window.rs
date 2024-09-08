use crate::gpu_utils::get_surface_configuration;
use std::sync::Arc;
use wgpu::{Device, Instance, Surface, SurfaceConfiguration};
use winit::dpi::Size;
use winit::event_loop::EventLoop;
use winit::window::{Window, WindowBuilder};

pub struct OSWindow {
    pub window: Arc<Window>,
    pub surface: Surface<'static>,
    pub surface_configuration: SurfaceConfiguration,
}

impl OSWindow {
    pub fn new<S: Into<Size>>(event_loop: &EventLoop<()>, instance: &Instance, size: S) -> Self {
        let window = WindowBuilder::new()
            .with_inner_size(size)
            .with_title(&"imgui-wgpu".to_string())
            .build(&event_loop)
            .unwrap();

        let window = Arc::new(window);
        let surface = instance.create_surface(window.clone()).unwrap();

        let size = window.inner_size();
        let surface_configuration = get_surface_configuration(size);

        Self {
            surface,
            window,
            surface_configuration,
        }
    }

    pub fn init_configuration(&self, device: &Device) {
        self.surface.configure(device, &self.surface_configuration);
    }

    pub fn re_configure(&mut self, device: &Device) {
        let size = self.window.inner_size();
        self.surface_configuration = get_surface_configuration(size);
        self.init_configuration(device);
    }
}
