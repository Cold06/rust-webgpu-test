use imgui::{Context, FontConfig, FontSource};
use imgui_wgpu::Renderer;
use imgui_winit_support::WinitPlatform;
use wgpu::{TextureFormat, TextureView};
use winit::window::Window;
use crate::gpu::GPUCtx;

pub struct Gui {
    pub imgui: Context,
    pub renderer: Renderer,
    pub platform: WinitPlatform,
}

impl Gui {
    pub fn new(
        window: &Window,
        ctx: &GPUCtx,
        frame_buffer_format: TextureFormat,
    ) -> Self {
        let mut imgui = Context::create();
        imgui.set_ini_filename(None);

        let mut platform = WinitPlatform::init(&mut imgui);
        platform.attach_window(
            imgui.io_mut(),
            &window,
            imgui_winit_support::HiDpiMode::Default,
        );

        let hidpi_factor = window.scale_factor();
        let font_size = (13.0 * hidpi_factor) as f32;

        imgui.io_mut().font_global_scale = (1.0 / hidpi_factor) as f32;

        imgui.fonts().add_font(&[FontSource::DefaultFontData {
            config: Some(FontConfig {
                oversample_h: 1,
                pixel_snap_h: true,
                size_pixels: font_size,
                ..Default::default()
            }),
        }]);

        let renderer = Renderer::new(
            &mut imgui,
            &ctx.device,
            &ctx.queue,
            imgui_wgpu::RendererConfig {
                texture_format: frame_buffer_format,
                ..Default::default()
            },
        );

        Self {
            imgui,
            platform,
            renderer,
        }
    }

    pub fn render(&mut self, ctx: &GPUCtx, main_view: &TextureView) {
        {
            let mut encoder: wgpu::CommandEncoder =
                ctx.device.create_command_encoder(&wgpu::CommandEncoderDescriptor { label: None });

            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: None,
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &main_view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Load,
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
            });

            self.renderer
                .render(self.imgui.render(), &ctx.queue, &ctx.device, &mut pass)
                .expect("Rendering failed");

            drop(pass);

            ctx.queue.submit(Some(encoder.finish()));
        }
    }
}
