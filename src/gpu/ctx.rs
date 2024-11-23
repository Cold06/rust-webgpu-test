use crate::window::OSWindow;
use egui_wgpu::wgpu;
use pollster::block_on;
use std::rc::Rc;
use winit::dpi::LogicalSize;
use winit::event_loop::EventLoop;

#[derive(Debug)]
pub struct GPUCtx {
    pub device: Rc<wgpu::Device>,
    pub queue: Rc<wgpu::Queue>,
}

impl Clone for GPUCtx {
    fn clone(&self) -> Self {
        Self {
            device: self.device.clone(),
            queue: self.queue.clone(),
        }
    }
}

impl GPUCtx {
    pub fn new(event_loop: &EventLoop<()>) -> (Self, OSWindow) {
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
            backends: wgpu::Backends::PRIMARY,
            ..Default::default()
        });

        let size = LogicalSize::new(1920, 1080);
        let window = OSWindow::new(&event_loop, &instance, size);

        let adapter = block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::HighPerformance,
            compatible_surface: Some(&window.surface),
            force_fallback_adapter: false,
        }))
        .unwrap();

        let (device, queue) =
            block_on(adapter.request_device(&wgpu::DeviceDescriptor::default(), None)).unwrap();

        let ctx = Self {
            queue: queue.into(),
            device: device.into(),
        };

        window.init_configuration(&ctx);

        (ctx, window)
    }

    #[allow(unused)]
    pub(crate) fn force_sync(&self) {
        self.queue.submit([]);
    }
}
