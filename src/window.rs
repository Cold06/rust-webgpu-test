use crate::gpu_utils::get_surface_configuration;
use std::sync::Arc;
use egui_wgpu::wgpu;
use wgpu::{Instance, Surface, SurfaceConfiguration};
use winit::dpi::Size;
use winit::event_loop::EventLoop;
use winit::window::{Window, WindowAttributes};
use crate::gpu::GPUCtx;

pub struct OSWindow {
    pub window: Arc<Window>,
    pub surface: Surface<'static>,
    pub surface_configuration: SurfaceConfiguration,
}

impl OSWindow {
    pub fn new<S: Into<Size>>(event_loop: &EventLoop<()>, instance: &Instance, size: S) -> Self {
        let window_attrs = WindowAttributes::new()
            .with_inner_size(size)
            .with_title(&"imgui-wgpu".to_string());

        let window = event_loop.create_window(window_attrs).expect("Failed to create window");

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

    pub fn init_configuration(&self, ctx: &GPUCtx) {
        self.surface.configure(&ctx.device, &self.surface_configuration);
    }

    pub fn re_configure(&mut self, ctx: &GPUCtx) {
        let size = self.window.inner_size();
        self.surface_configuration = get_surface_configuration(size);
        self.init_configuration(ctx);
    }
}
