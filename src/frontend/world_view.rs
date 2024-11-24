use crate::camera::Camera;
use crate::camera_controller::CameraController;
use crate::egui_tools::EguiRenderer;
use crate::frontend::{TabView, TabHandle};
use crate::gizmo_example::GizmoExample;
use crate::gpu::{GPUCtx, GPUTexture, ViewTarget};
use bytemuck::{Pod, Zeroable};
use egui::load::SizedTexture;
use egui::{ImageSource, Rect, TextureId, Ui};
use egui_wgpu::wgpu;
use egui_wgpu::wgpu::{FilterMode, TextureView};
use glam::{Mat4, Vec2, Vec3};
use std::time::Duration;
use crate::shared::{Shared, WeakShared};

pub struct WorldView {
    pub gizmo_example: GizmoExample,
    pub secondary_render_target_depth: ViewTarget,
    pub secondary_rt_texture_id: TextureId,
    pub secondary_rt_gpu_texture: GPUTexture,
    pub secondary_camera: Camera,
    pub secondary_camera_controller: CameraController,
    pub ctx: GPUCtx,
    pub needs_texture_update: bool,
}

#[repr(C)]
#[derive(Pod, Copy, Clone, Zeroable)]
struct Filler0(u8, u8, u8, u8);

impl WorldView {
    pub fn get_transform(&mut self, delta: Duration) -> Mat4 {
        self.secondary_camera_controller
            .update_camera(&mut self.secondary_camera, delta);
        self.secondary_camera.compute();

        self.gizmo_example.transform
    }

    pub fn new(ctx: &GPUCtx, egui_renderer: &mut EguiRenderer, render_passes: &mut Vec<WeakShared<WorldView>>, egui_passes: &mut Vec<WeakShared<WorldView>>) -> Shared<Self> {
        let view_width = 700.0;
        let view_height = 400.0;
        let secondary_camera = Camera::new(
            Vec3::new(-140.0, 0.0, 0.0),
            Vec2::ZERO,
            view_width,
            view_height,
        );
        let mut secondary_camera_controller = CameraController::new(200.0, 0.004);
        secondary_camera_controller.copy_camera_rotation(&secondary_camera);

        let secondary_render_target_depth =
            ViewTarget::create(&ctx, view_width as u32, view_height as u32);

        let secondary_rt_gpu_texture = GPUTexture::create(
            &ctx,
            view_width as u32,
            view_height as u32,
            wgpu::TextureFormat::Bgra8UnormSrgb,
            Filler0(0, 0, 0, 255),
            wgpu::TextureUsages::COPY_DST
                | wgpu::TextureUsages::RENDER_ATTACHMENT
                | wgpu::TextureUsages::TEXTURE_BINDING,
        );

        let secondary_rt_texture_id = egui_renderer.renderer.register_native_texture(
            &ctx.device,
            &secondary_rt_gpu_texture.view,
            FilterMode::Linear,
        );

        let gizmo_example = GizmoExample::new();

        let data: Shared<Self> = Self {
            gizmo_example,
            secondary_render_target_depth,
            secondary_rt_texture_id,
            secondary_rt_gpu_texture,
            secondary_camera,
            secondary_camera_controller,
            ctx: ctx.clone(),
            needs_texture_update: false,
        }.into();

        render_passes.push(data.weak());
        egui_passes.push(data.weak());

        data
    }

    pub fn render_to<F: FnMut(&Camera, &TextureView, &TextureView)>(&self, render_pass: &mut F) {
        let color_view = &self.secondary_rt_gpu_texture.view;
        let depth_view = &self
            .secondary_render_target_depth
            .depth_stencil
            .create_view(&wgpu::TextureViewDescriptor::default());

        render_pass(&self.secondary_camera, color_view, depth_view);
    }

    pub fn on_egui(&self, egui_renderer: &mut EguiRenderer) {
        egui_renderer
            .renderer
            .update_egui_texture_from_wgpu_texture(
                &self.ctx.device,
                &self.secondary_rt_gpu_texture.view,
                FilterMode::Linear,
                self.secondary_rt_texture_id,
            );
    }
}

impl TabView for Shared<WorldView> {
    fn title(&self, _: &TabHandle) -> String {
        format!("View")
    }

    fn content(&mut self, ui: &mut Ui) {
        self.with(|view| {
            let rect = ui.response().interact_rect;

            if rect != Rect::NOTHING {
                let w = rect.width();
                let h = rect.height();

                ui.image(ImageSource::Texture(SizedTexture::new(
                    view.secondary_rt_texture_id,
                    [w, h],
                )));

                // view.gizmo_example.draw_gizmo(
                //     ui,
                //     &view.secondary_camera,
                //     w,
                //     h,
                // );

                view.secondary_camera.check_resize(w, h, || {
                    if w as u32 == 0 || h as u32 == 0 {
                        return;
                    }

                    view.secondary_rt_gpu_texture.resize(
                        &view.ctx,
                        w as u32,
                        h as u32,
                        Filler0(0, 0, 0, 255),
                    );

                    view.secondary_render_target_depth
                        .resize(&view.ctx, w as u32, h as u32);

                    view.needs_texture_update = true;
                });
            }
        });
    }

    fn as_tab_handle(&self, surface: egui_dock::SurfaceIndex, node: egui_dock::NodeIndex) -> TabHandle {
        TabHandle::new(self.clone().into(), surface, node)
    }
}
