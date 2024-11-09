use std::rc::Rc;
use pollster::block_on;
use winit::dpi::LogicalSize;
use winit::event_loop::EventLoop;
use crate::window::OSWindow;

#[derive(Debug)]
pub struct GPUCtx {
    pub device: Rc<wgpu::Device>,
    pub queue: Rc<wgpu::Queue>,
}

impl GPUCtx {
    pub fn new(event_loop: &EventLoop<()>) -> (Self, OSWindow) {
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
            backends: wgpu::Backends::PRIMARY,
            ..Default::default()
        });

        let size = LogicalSize::new(1280.0, 720.0);
        let mut window = OSWindow::new(&event_loop, &instance, size);

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
}
