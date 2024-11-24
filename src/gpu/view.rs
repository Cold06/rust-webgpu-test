use crate::gpu::GPUCtx;
use crate::gpu_utils::build_depth_texture;
use egui_wgpu::wgpu;

pub struct ViewTarget {
    pub depth_stencil: wgpu::Texture,
}

impl ViewTarget {
    pub fn create(ctx: &GPUCtx, width: u32, height: u32) -> Self {
        Self {
            depth_stencil: build_depth_texture(ctx, (width, height)),
        }
    }

    pub fn resize(&mut self, ctx: &GPUCtx, width: u32, height: u32) {
        self.depth_stencil = build_depth_texture(ctx, (width, height));
    }
}

pub struct SView<'a> {
    color_view: &'a wgpu::TextureView,
    depth_view: &'a wgpu::TextureView,
}

impl<'a> SView<'a> {
    pub fn new(color_view: &'a wgpu::TextureView, depth_view: &'a wgpu::TextureView) -> SView<'a> {
        Self {
            color_view,
            depth_view,
        }
    }

    pub fn render_pass<'pass>(
        &'pass self,
        encoder: &'pass mut wgpu::CommandEncoder,
    ) -> wgpu::RenderPass<'pass> {
        encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: None,
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: &self.color_view,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(wgpu::Color {
                        r: 0.0,
                        g: 0.0,
                        b: 0.0,
                        a: 1.0,
                    }),
                    store: wgpu::StoreOp::Store,
                },
            })],
            depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                view: &self.depth_view,
                depth_ops: Some(wgpu::Operations {
                    load: wgpu::LoadOp::Clear(1.0),
                    store: wgpu::StoreOp::Store,
                }),
                stencil_ops: None,
            }),
            timestamp_writes: None,
            occlusion_query_set: None,
        })
    }
}
