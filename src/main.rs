#![feature(const_option)]

mod camera;
mod camera_controller;
mod camera_utils;
mod canvas;
mod chunk;
mod cube;
mod demos;
mod egui_tools;
mod frontend;
mod fs_utils;
mod gizmo_example;
mod gpu;
mod gpu_utils;
mod js;
mod multimath;
mod paint_utils;
mod pipelines;
mod shared;
mod video;
mod window;

use crate::camera::Camera;
use crate::camera_controller::CameraController;
use crate::camera_utils::process_camera_input;
use crate::canvas::{render_svg, Canvas};
use crate::demos::{ChunksDemo, VideoDemo};
use crate::egui_tools::EguiRenderer;
use crate::frontend::{HandleList, TabHandle};
use crate::gpu::{GPUCtx, GPUTexture, SView, ViewTarget};
use crate::js::VM;
use crate::video::{start, FrameData, PipelineEvent};
use bytemuck::{Pod, Zeroable};
use egui::load::SizedTexture;
use egui::ImageSource;
use egui_dock::{DockArea, DockState, NodeIndex, Style, SurfaceIndex};
use egui_wgpu::wgpu::FilterMode;
use egui_wgpu::{wgpu, ScreenDescriptor};
use frontend::{TabView, WorldView};
use glam::*;
use shared::{Shared, WeakShared};
use std::error::Error;
use std::path::PathBuf;
use std::time::{Duration, Instant};
use video::MP4Command;
use winit::{
    event::{ElementState, Event, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
    keyboard::{Key, NamedKey},
    window::CursorGrabMode,
};

#[repr(C)]
#[derive(Pod, Copy, Clone, Zeroable)]
struct Filler0(u8, u8, u8, u8);

fn fps(frame_duration: Duration) -> f64 {
    let seconds = frame_duration.as_secs_f64();
    if seconds > 0.0 {
        1.0 / seconds
    } else {
        f64::INFINITY
    }
}

fn main() -> Result<(), Box<dyn Error>> {
    env_logger::init();

    let mut vm = Shared::new(VM::new());

    // Choose a video
    // let video_path = get_random_file_from_directory("/Volumes/dev/Shared/mp4")
    //     .or_else(|| Some(PathBuf::from("/Users/cold/Desktop/YP-1R-05x13.mp4")))
    //     .unwrap();

    let video_path = PathBuf::from("/Users/cold/Desktop/YP-1R-05x13.mp4");

    println!("Now playing: {:?}", video_path);

    let (video_thread, video_receiver, command_sender) = start(video_path);

    let event_loop = EventLoop::new()?;

    let (ctx, mut os_window) = GPUCtx::new(&event_loop);
    let window_size = os_window.window.inner_size();
    let high_dpi_factor = 2.0 * os_window.window.scale_factor() as f32;

    os_window.window.set_maximized(true);

    let mut main_render_target_depth =
        ViewTarget::create(&ctx, window_size.width, window_size.height);

    let mut egui_renderer = EguiRenderer::new(
        &ctx.device,
        os_window.surface_configuration.format,
        None,
        1,
        &os_window.window,
    );

    let chunks_demo = Shared::new(ChunksDemo::create(&os_window.surface_configuration, &ctx));
    let mut video_demo = VideoDemo::create(&ctx, &os_window.surface_configuration);

    let canvas_size = [1000.0 * high_dpi_factor, 1000.0 * high_dpi_factor];
    let skia_canvas = Shared::new(Canvas::new(
        canvas_size[0] as u32,
        canvas_size[1] as u32,
        high_dpi_factor,
    ));

    {
        let code = String::from(include_str!(
            "/Users/cold/w/pony-render/svg/tier-5/complex-drawing.svg"
        ));
        render_svg(code, &mut skia_canvas.borrow_mut());
    }

    let canvas_data = skia_canvas.borrow_mut().as_bytes()?;
    let skia_gpu_texture = Shared::new(GPUTexture::create(
        &ctx,
        canvas_size[0] as u32,
        canvas_size[1] as u32,
        wgpu::TextureFormat::Rgba8UnormSrgb,
        Filler0(0, 0, 0, 255),
        wgpu::TextureUsages::COPY_DST | wgpu::TextureUsages::TEXTURE_BINDING,
    ));

    let canvas_texture_id = skia_gpu_texture.with(|t| {
        t.update(&ctx, &canvas_data);

        egui_renderer
            .renderer
            .register_native_texture(&ctx.device, &t.view, FilterMode::Linear)
    });

    let mut main_camera = Camera::new(
        Vec3::new(-137.0, 0.0, 0.0),
        Vec2::ZERO,
        window_size.width as f32,
        window_size.height as f32,
    );
    let mut camera_controller = CameraController::new(20.0, 0.004);
    camera_controller.copy_camera_rotation(&main_camera);

    let mut square_dist = 1.0;
    let scale_factor = 1.0;
    let mut focused = false;
    let mut last_frame = Instant::now();
    let mut use_secondary_camera = false;


    let mut render_passes: Vec<WeakShared<WorldView>> = vec![];
    let mut egui_passes: Vec<WeakShared<WorldView>> = vec![];

    let world_view1 = frontend::WorldView::new(
        &ctx,
        &mut egui_renderer,
        &mut egui_passes,
        &mut render_passes,
    );
    let mut stats_view = frontend::QuickView::new();
    let mut canvas_example_view = frontend::QuickView::new();
    let mut chunk_manager_view = frontend::QuickView::new();
    let mut video_view = frontend::QuickView::new();
    let code_editor_view = frontend::CodeView::new();

    let mut dock_state = DockState::new(vec![
        canvas_example_view.as_tab_handle(SurfaceIndex::main(), NodeIndex(1))
    ]);

    {
        let [a, b] = dock_state.main_surface_mut().split_left(
            NodeIndex::root(),
            0.5,
            vec![
                code_editor_view.as_tab_handle(SurfaceIndex::main(), NodeIndex(6)),
                chunk_manager_view.as_tab_handle(SurfaceIndex::main(), NodeIndex(3)),
            ],
        );
        let [_, _] = dock_state.main_surface_mut().split_below(
            a,
            0.5,
            vec![video_view.as_tab_handle(SurfaceIndex::main(), NodeIndex(4))],
        );
        let [_, _] = dock_state.main_surface_mut().split_below(
            b,
            0.5,
            vec![
                world_view1.as_tab_handle(SurfaceIndex::main(), NodeIndex(5)),
                stats_view.as_tab_handle(SurfaceIndex::main(), NodeIndex(8)),
            ],
        );
    }

    let mut counter: usize = 9;

    event_loop.run(|event, window_target| {
        window_target.set_control_flow(ControlFlow::Poll);

        // TODO: focus management to know which view should process the inputs

        if use_secondary_camera {
            world_view1.with(|view| {
                process_camera_input(
                    focused,
                    event.clone(),
                    &mut view.secondary_camera_controller,
                );
            });
        } else {
            process_camera_input(focused, event.clone(), &mut camera_controller);
        }

        match event {
            Event::AboutToWait => {
                os_window.window.request_redraw();
            }
            Event::WindowEvent { ref event, .. } => {
                egui_renderer.handle_input(&os_window.window, &event);

                match event {
                    WindowEvent::Resized(size) => {
                        main_render_target_depth.resize(&ctx, size.width, size.height);
                        os_window.re_configure(&ctx);
                        main_camera.resize(size.width as f32, size.height as f32);
                    }
                    WindowEvent::CloseRequested => {
                        window_target.exit();
                    }
                    WindowEvent::KeyboardInput { event, .. } => {
                        if let Key::Named(NamedKey::Escape) = event.logical_key {
                            if event.state.is_pressed() {
                                os_window
                                    .window
                                    .set_cursor_grab(CursorGrabMode::None)
                                    .unwrap();
                                os_window.window.set_cursor_visible(true);
                                focused = false;
                            }
                        }

                        if let Key::Named(NamedKey::Tab) = event.logical_key {
                            if event.state.is_pressed() {
                                use_secondary_camera = !use_secondary_camera;
                            }
                        }
                    }
                    WindowEvent::MouseInput {
                        state: ElementState::Pressed,
                        button: winit::event::MouseButton::Left,
                        ..
                    } => {
                        // if !egui_renderer.state.egui_ctx().is_pointer_over_area() {
                        //     os_window
                        //         .window
                        //         .set_cursor_grab(CursorGrabMode::Locked)
                        //         .unwrap();
                        //     os_window.window.set_cursor_visible(false);
                        //     focused = true;
                        // }
                    }
                    WindowEvent::RedrawRequested => {
                        // Compute Delta
                        let now = Instant::now();
                        let delta = now - last_frame;
                        last_frame = now;

                        // Update Cameras
                        camera_controller.update_camera(&mut main_camera, delta);
                        main_camera.compute();

                        // TODO: This is also TRASH we should not
                        // need to match against the enum

                        world_view1.with(|view| {
                            let transform = view.get_transform(delta);

                            video_demo.update_location(&ctx, transform);
                        });

                        // Get Surface Texture
                        let frame = match os_window.surface.get_current_texture() {
                            Ok(frame) => frame,
                            Err(e) => {
                                eprintln!("dropped frame: {e:?}");
                                return;
                            }
                        };

                        // Try to update video texture
                        if let Ok(event) = video_receiver.try_recv() {
                            match event {
                                PipelineEvent::Data(frame) => {
                                    video_demo.check_resize(&ctx, frame.resolution);

                                    match frame.data {
                                        FrameData::PlanarYuv420(planes) => {
                                            video_demo.update_texture(
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

                        let frame_view = &frame.texture.create_view(&Default::default());

                        let mut render_pass =
                            |camera: &Camera,
                             color_view: &wgpu::TextureView,
                             depth_view: &wgpu::TextureView| {
                                let mut encoder =
                                    ctx.device.create_command_encoder(&Default::default());
                                {
                                    let view = SView::new(color_view, depth_view);
                                    let mut pass = view.render_pass(&mut encoder).forget_lifetime();

                                    chunks_demo.with(|u| {
                                        u.setup_dynamic_camera(&ctx, &camera);
                                        u.render_static(&mut pass);
                                    });

                                    video_demo.setup_dynamic_camera(&ctx, &camera);
                                    video_demo.render_static(&mut pass);
                                }

                                ctx.queue.submit([encoder.finish()]);
                            };

                        {
                            for item in &egui_passes {
                                if let Some(rc) = item.upgrade() {
                                    rc.borrow_mut().on_egui(&mut egui_renderer);
                                }
                            }
                            egui_passes.retain(|weak| weak.upgrade().is_some());
                        }

                        {
                            for item in &render_passes {
                                if let Some(rc) = item.upgrade() {
                                    rc.borrow_mut().render_to(&mut render_pass);
                                }
                            }
                            render_passes.retain(|weak| weak.upgrade().is_some());
                        }

                        let v = dock_state.focused_leaf();

                        stats_view.ui(move |ui| {
                            ui.label(format!("FPS: {}", fps(delta)));

                            if let Some((a, b)) = v {
                                ui.label(format!("Focused surface {} on node {}", a.0, b.0));
                            } else {
                                ui.label(format!("No focused tab"));
                            }
                        });

                        {
                            let mut encoder =
                                ctx.device.create_command_encoder(&Default::default());

                            egui_renderer.begin_frame(&os_window.window);

                            let mut added_nodes = Vec::new();

                            let mut handle_list = HandleList::new(&mut added_nodes);

                            DockArea::new(&mut dock_state)
                                .show_add_buttons(true)
                                .show_add_popup(true)
                                .style(Style::from_egui(egui_renderer.context().style().as_ref()))
                                .show(egui_renderer.context(), &mut handle_list);

                            handle_list.build_tabs(
                                &ctx,
                                &mut egui_renderer,
                                &mut egui_passes,
                                &mut render_passes,
                            );

                            added_nodes.drain(..).for_each(|node| {
                                dock_state.set_focused_node_and_surface((node.surface, node.node));
                                dock_state.push_to_focused_leaf(TabHandle {
                                    kind: node.kind,
                                    surface: node.surface,
                                    node: NodeIndex(counter),
                                });
                                counter += 1;
                            });

                            let inner_size = os_window.window.inner_size();
                            let outer_size = os_window.window.outer_size();

                            let inner = skia_canvas.clone();
                            let inner_text = skia_gpu_texture.clone();
                            let inner_ctx = ctx.clone();

                            canvas_example_view.ui(move |ui| {
                                ui.label(format!("Available Size {}", ui.available_size()));
                                ui.label(format!("Inner Size {:#?}", inner_size));
                                ui.label(format!("Size {:#?}", outer_size));

                                ui.image(ImageSource::Texture(SizedTexture::new(
                                    canvas_texture_id,
                                    (
                                        canvas_size[0] / high_dpi_factor,
                                        canvas_size[0] / high_dpi_factor,
                                    ),
                                )));

                                if ui.button("Add Square").clicked() {
                                    square_dist += 50.0;

                                    inner.borrow_mut().fill_rect(
                                        square_dist,
                                        square_dist,
                                        100.0 + square_dist,
                                        100.0 + square_dist,
                                    );

                                    let example_data = inner.borrow_mut().as_bytes().unwrap();

                                    inner_text.with(|u: &mut GPUTexture| {
                                        u.update(&inner_ctx, &example_data);
                                    });
                                }
                            });

                            let inner_ctx = ctx.clone();

                            let inner_c_demo = chunks_demo.clone();

                            chunk_manager_view.ui(move |ui| {
                                if ui.button("Spawn Chunk").clicked() {
                                    inner_c_demo.with(|u| u.spawn_chunk(&inner_ctx));
                                }
                            });

                            let value = command_sender.clone();

                            video_view.ui(move |ui: &mut egui::Ui| {
                                if ui.button("Pause").clicked() {
                                    let _ = value.try_send(MP4Command::Pause);
                                }
                                if ui.button("Play").clicked() {
                                    let _ = value.try_send(MP4Command::Play);
                                }
                                if ui.button("Stop").clicked() {
                                    let _ = value.try_send(MP4Command::Stop);
                                }
                                if ui.button("SkipForward").clicked() {
                                    let _ = value.try_send(MP4Command::SkipForward);
                                }
                                if ui.button("SkipBackward").clicked() {
                                    let _ = value.try_send(MP4Command::SkipBackward);
                                }
                                if ui.button("Seek(Duration)").clicked() {
                                    let _ =
                                        value.try_send(MP4Command::Seek(Duration::from_millis(0)));
                                }
                            });

                            egui_renderer.end_frame_and_draw(
                                &ctx.device,
                                &ctx.queue,
                                &mut encoder,
                                &os_window.window,
                                &frame_view,
                                ScreenDescriptor {
                                    size_in_pixels: [
                                        os_window.surface_configuration.width,
                                        os_window.surface_configuration.height,
                                    ],
                                    pixels_per_point: os_window.window.scale_factor() as f32
                                        * scale_factor,
                                },
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

    drop(video_thread);

    Ok(())
}
