#![feature(const_option)]

mod camera;
mod camera_controller;
mod camera_utils;
mod canvas;
mod chunk;
mod cube;
mod egui_tools;
mod example;
mod fs_utils;
mod gpu;
mod gpu_utils;
mod multimath;
mod paint_utils;
mod pipelines;
mod video;
mod video_example;
mod window;
mod gizmo_example;

use crate::camera::Camera;
use crate::camera_controller::CameraController;
use crate::camera_utils::process_camera_input;
use crate::canvas::Canvas;
use crate::egui_tools::EguiRenderer;
use crate::example::Example;
use crate::fs_utils::get_random_file_from_directory;
use crate::gpu::{GPUCtx, GPUTexture, SView, ViewTarget};
use crate::multimath::{Vec2, Vec3};
use crate::video::{start, FrameData, PipelineEvent};
use crate::video_example::VideoExample;
use bytemuck::{Pod, Zeroable};
use egui::load::SizedTexture;
use egui::ImageSource;
use egui_wgpu::wgpu::FilterMode;
use egui_wgpu::{wgpu, ScreenDescriptor};
use std::error::Error;
use std::time::Instant;
use winit::{
    event::{ElementState, Event, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
    keyboard::{Key, NamedKey},
    window::CursorGrabMode,
};
use crate::gizmo_example::GizmoExample;

#[repr(C)]
#[derive(Pod, Copy, Clone, Zeroable)]
struct Filler0(u8, u8, u8, u8);

fn main() -> Result<(), Box<dyn Error>> {
    env_logger::init();

    // Choose a video
    let video_path = get_random_file_from_directory("/Volumes/dev/Shared/mp4").unwrap();
    println!("Now playing: {:?}", video_path);

    // Start playing
    let (a, b) = start(video_path);

    // Create the event loop
    let event_loop = EventLoop::new()?;

    // Init graphics
    let (ctx, mut window) = GPUCtx::new(&event_loop);
    let size = window.window.inner_size();

    // Create depth textures
    let mut main_render_target_depth = ViewTarget::create(&ctx, size.width, size.height);

    // Create egui
    let mut egui_renderer = EguiRenderer::new(
        &ctx.device,
        window.surface_configuration.format,
        None,
        1,
        &window.window,
    );

    // Init Examples
    let mut example = Example::init(&window.surface_configuration, &ctx);
    let mut video_example = VideoExample::create(&ctx, &window.surface_configuration);

    // Init canvas
    let canvas_size = [512.0, 512.0];
    let mut example_canvas = Canvas::new(canvas_size[0] as u32, canvas_size[1] as u32);
    example_canvas.fill_rect(0.0, 0.0, 100.0, 100.0);
    let example_data = example_canvas.as_bytes()?;
    let example_texture = GPUTexture::create(
        &ctx,
        canvas_size[0] as u32,
        canvas_size[1] as u32,
        wgpu::TextureFormat::Rgba8Unorm,
        Filler0(0, 0, 0, 255),
        wgpu::TextureUsages::COPY_DST | wgpu::TextureUsages::TEXTURE_BINDING,
    );
    example_texture.update(&ctx, &example_data);
    let canvas_texture_id = egui_renderer.renderer.register_native_texture(
        &ctx.device,
        &example_texture.view,
        FilterMode::Linear,
    );

    // Secondary view render attachments
    let mut secondary_render_target_depth = ViewTarget::create(&ctx, size.width, size.height);
    let imgui_color_texture = GPUTexture::create(
        &ctx,
        size.width,
        size.height,
        wgpu::TextureFormat::Bgra8UnormSrgb,
        Filler0(0, 0, 0, 255),
        wgpu::TextureUsages::COPY_DST
            | wgpu::TextureUsages::RENDER_ATTACHMENT
            | wgpu::TextureUsages::TEXTURE_BINDING,
    );
    let example_texture_id = egui_renderer.renderer.register_native_texture(
        &ctx.device,
        &imgui_color_texture.view,
        FilterMode::Linear,
    );

    // Main camera
    let mut camera = Camera::new(
        Vec3::from_components(-50.0, 0.0, 0.0),
        Vec2::new(),
        size.width as f32,
        size.height as f32,
    );
    let mut camera_controller = CameraController::new(20.0, 0.004);
    camera_controller.copy_camera_rotation(&camera);

    // Secondary camera
    let mut camera_debug = Camera::new(
        Vec3::from_components(-50.0, 0.0, 0.0),
        Vec2::new(),
        size.width as f32,
        size.height as f32,
    );
    let mut camera_controller_debug = CameraController::new(200.0, 0.004);
    camera_controller.copy_camera_rotation(&camera_debug);

    // State
    let mut square_dist = 1.0;
    let scale_factor = 1.0;
    let mut focused = false;
    let mut last_frame = Instant::now();
    let mut use_debug_camera = false;

    let mut gizmo_example = GizmoExample::new();

    event_loop.run(|event, window_target| {
        window_target.set_control_flow(ControlFlow::Poll);

        if use_debug_camera {
            process_camera_input(focused, event.clone(), &mut camera_controller_debug);
        } else {
            process_camera_input(focused, event.clone(), &mut camera_controller);
        }

        match event {
            Event::AboutToWait => {
                window.window.request_redraw();
            }
            Event::WindowEvent { ref event, .. } => {
                egui_renderer.handle_input(&window.window, &event);

                match event {
                    WindowEvent::Resized(size) => {
                        main_render_target_depth.resize(&ctx, size.width, size.height);
                        window.re_configure(&ctx);
                        camera.resize(size.width as f32, size.height as f32);
                    }
                    WindowEvent::CloseRequested => {
                        window_target.exit();
                    }
                    WindowEvent::KeyboardInput { event, .. } => {
                        if let Key::Named(NamedKey::Escape) = event.logical_key {
                            if event.state.is_pressed() {
                                window.window.set_cursor_grab(CursorGrabMode::None).unwrap();
                                window.window.set_cursor_visible(true);
                                focused = false;
                            }
                        }

                        if let Key::Named(NamedKey::Tab) = event.logical_key {
                            if event.state.is_pressed() {
                                use_debug_camera = !use_debug_camera;
                            }
                        }
                    }
                    WindowEvent::MouseInput {
                        state: ElementState::Pressed,
                        button: winit::event::MouseButton::Left,
                        ..
                    } => {
                        if !egui_renderer.state.egui_ctx().is_pointer_over_area() {
                            window
                                .window
                                .set_cursor_grab(CursorGrabMode::Locked)
                                .unwrap();
                            window.window.set_cursor_visible(false);
                            focused = true;
                        }
                    }
                    WindowEvent::RedrawRequested => {
                        // Compute Delta
                        let now = Instant::now();
                        let delta = now - last_frame;
                        last_frame = now;

                        // Update Cameras
                        camera_controller.update_camera(&mut camera, delta);
                        camera.compute();

                        camera_controller_debug.update_camera(&mut camera_debug, delta);
                        camera_debug.compute();

                        // Get Surface Texture
                        let frame = match window.surface.get_current_texture() {
                            Ok(frame) => frame,
                            Err(e) => {
                                eprintln!("dropped frame: {e:?}");
                                return;
                            }
                        };

                        // Try to update video texture
                        if let Ok(event) = b.try_recv() {
                            match event {
                                PipelineEvent::Data(frame) => {
                                    video_example.check_resize(&ctx, frame.resolution);

                                    match frame.data {
                                        FrameData::PlanarYuv420(planes) => {
                                            video_example.update_texture(
                                                &ctx,
                                                &planes.y_plane,
                                                &planes.u_plane,
                                                &planes.v_plane,
                                            );
                                        }
                                    }
                                }
                                PipelineEvent::EOS => {
                                    println!("Got end of stream");
                                }
                            }
                        }

                        // This will be used for both main render pass and egui render pass
                        let frame_view = &frame.texture.create_view(&Default::default());

                        let mut render_pass = |camera: &Camera, color_view: &wgpu::TextureView, depth_view: &wgpu::TextureView| {
                            let mut encoder =
                                ctx.device.create_command_encoder(&Default::default());
                            {
                                let mut view = SView::new(color_view, depth_view);
                                let mut pass = view.render_pass(&mut encoder);
                                example.setup_dynamic_camera(&ctx, &camera);
                                video_example.setup_dynamic_camera(&ctx, &camera);
                                example.render(&mut pass);
                                video_example.render(&mut pass);
                            }

                            ctx.queue.submit([encoder.finish()]);
                        };

                        // Render main pass
                        {
                            let color_view = frame_view;
                            let depth_view =
                                &main_render_target_depth.depth_stencil.create_view(&Default::default());

                            render_pass(&camera, color_view, depth_view);
                        }

                        // Render Second Pass
                        {
                            let color_view = &imgui_color_texture.view;
                            let depth_view = &secondary_render_target_depth
                                .depth_stencil
                                .create_view(&wgpu::TextureViewDescriptor::default());

                            render_pass(&camera_debug, color_view, depth_view);
                        }

                        // Render ImGUI
                        {
                            let mut encoder =
                                ctx.device.create_command_encoder(&Default::default());

                            egui_renderer.begin_frame(&window.window);

                            let screen_descriptor = ScreenDescriptor {
                                size_in_pixels: [
                                    window.surface_configuration.width,
                                    window.surface_configuration.height,
                                ],
                                pixels_per_point: window.window.scale_factor() as f32
                                    * scale_factor,
                            };

                            egui::SidePanel::left("options_panel").show(egui_renderer.context(), |ui| {
                                gizmo_example.draw_options(ui);
                            });

                            egui::Window::new("Canvas Example")
                                .default_size([512.0, 512.0])
                                .resizable(true)
                                .show(egui_renderer.context(), |ui| {
                                    ui.image(ImageSource::Texture(SizedTexture::new(
                                        canvas_texture_id,
                                        canvas_size,
                                    )));

                                    if ui.button("Add Square").clicked() {
                                        square_dist += 50.0;

                                        example_canvas.fill_rect(
                                            square_dist,
                                            square_dist,
                                            100.0 + square_dist,
                                            100.0 + square_dist,
                                        );

                                        let example_data = example_canvas.as_bytes().unwrap();
                                        example_texture.update(&ctx, &example_data);
                                    }
                                });

                            egui::Window::new("Camera")
                                .default_size([170.0, 260.0])
                                .resizable(true)
                                .show(egui_renderer.context(), |ui| {
                                    ui.label(format!(
                                        "Main Camera {}",
                                        if !use_debug_camera { "(active)" } else { "" }
                                    ));
                                    ui.label(format!("   X {}", camera.view.position.x));
                                    ui.label(format!("   Y {}", camera.view.position.y));
                                    ui.label(format!("   Z {}", camera.view.position.z));
                                    ui.label(format!("   Pitch {}", camera.view.yaw_pitch.x));
                                    ui.label(format!("   Yaw {}", camera.view.yaw_pitch.y));
                                    ui.label(format!(
                                        "Debug Camera {}",
                                        if use_debug_camera { "(active)" } else { "" }
                                    ));
                                    ui.label(format!("   X {}", camera_debug.view.position.x));
                                    ui.label(format!("   Y {}", camera_debug.view.position.y));
                                    ui.label(format!("   Z {}", camera_debug.view.position.z));
                                    ui.label(format!("   Pitch {}", camera_debug.view.yaw_pitch.x));
                                    ui.label(format!("   Yaw {}", camera_debug.view.yaw_pitch.y));
                                });

                            egui::Window::new("Renderer Example")
                                .default_size([512.0, 512.0])
                                .resizable(true)
                                .show(egui_renderer.context(), |ui| {
                                    ui.image(ImageSource::Texture(SizedTexture::new(
                                        example_texture_id,
                                        [(size.width as f32) / 6.0, (size.height as f32) / 6.0],
                                    )));
                                    gizmo_example.draw_gizmo(ui);
                                });

                            egui::Window::new("Chunk Manager")
                                .default_size([170.0, 260.0])
                                .resizable(true)
                                .show(egui_renderer.context(), |ui| {
                                    if ui.button("Spawn Chunk").clicked() {
                                        example.spawn_chunk(&ctx);
                                    }
                                });

                            egui_renderer.end_frame_and_draw(
                                &ctx.device,
                                &ctx.queue,
                                &mut encoder,
                                &window.window,
                                &frame_view,
                                screen_descriptor,
                            );

                            ctx.queue.submit(Some(encoder.finish()));
                        }

                        frame.present();
                    }
                    _ => {}
                }
            }
            _ => {}
        }
    })?;

    drop(a);

    Ok(())
}
