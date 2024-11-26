#![feature(duration_millis_float)]
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
mod thread_utils;
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
use crate::video::FrameData;
use bytemuck::{Pod, Zeroable};
use egui::load::SizedTexture;
use egui::{ImageSource, ProgressBar, Slider};
use egui_dock::{DockArea, DockState, NodeIndex, Style, SurfaceIndex};
use egui_wgpu::wgpu::FilterMode;
use egui_wgpu::{wgpu, ScreenDescriptor};
use fancy_duration::FancyDuration;
use frontend::{TabView, WorldView};
use fs_utils::get_random_file_from_directory;
use winit::application::ApplicationHandler;
use winit::event_loop::ActiveEventLoop;

use glam::*;
use shared::{Shared, WeakShared};
use std::error::Error;
use std::path::PathBuf;
use std::time::{Duration, Instant};
use video::VideoHandle;
use window::OSWindow;
use winit::window::WindowId;
use winit::{
    event::{ElementState, WindowEvent},
    event_loop::EventLoop,
    keyboard::{Key, NamedKey},
    window::CursorGrabMode,
};

pub const BENCHMARK_MODE: bool = true;

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

struct ApplicationHack {
    os_window: Shared<OSWindow>,
    boxed_fn: Box<dyn FnMut(&ActiveEventLoop, WindowId, WindowEvent)>,
}

impl ApplicationHack {
    fn new(ctx: GPUCtx, os_window: Shared<OSWindow>) -> Self {
        let mut vm = Shared::new(VM::new());

        // let video_path = get_random_file_from_directory("/Volumes/dev/Shared/mp4")
        //     .or_else(|| Some(PathBuf::from("/Users/cold/Desktop/YP-1R-05x13.mp4")))
        //     .unwrap();

        let video_path = PathBuf::from("/Users/cold/Desktop/YP-1R-05x13.mp4");
        let video_handle = VideoHandle::create(video_path);

        let window_size = { os_window.borrow().window.inner_size() };
        let high_dpi_factor = 2.0 * { os_window.borrow().window.scale_factor() as f32 };

        let mut main_render_target_depth =
            ViewTarget::create(&ctx, window_size.width, window_size.height);

        let mut egui_renderer = EguiRenderer::new(
            &ctx.device,
            { os_window.borrow().surface_configuration.format },
            None,
            1,
            { &os_window.borrow().window },
        );

        let chunks_demo = Shared::new(ChunksDemo::create(
            &os_window.borrow().surface_configuration,
            &ctx,
        ));

        let mut video_demo = VideoDemo::create(&ctx, &os_window.borrow().surface_configuration);

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

        let canvas_data = skia_canvas
            .borrow_mut()
            .as_bytes()
            .expect("Failed to read from canvas");
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
            Vec3::new(-337.0, 0.0, 0.0),
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
        let mut profiler_view = frontend::QuickView::new();

        let mut dock_state = DockState::new(vec![
            profiler_view.as_tab_handle(SurfaceIndex::main(), NodeIndex(2)),
            stats_view.as_tab_handle(SurfaceIndex::main(), NodeIndex(1)), // canvas_example_view.as_tab_handle(SurfaceIndex::main(), NodeIndex(1)),
        ]);

        {
            let [a, b] = dock_state.main_surface_mut().split_above(
                NodeIndex::root(),
                0.5,
                vec![
                    video_view.as_tab_handle(SurfaceIndex::main(), NodeIndex(3)),
                    // code_editor_view.as_tab_handle(SurfaceIndex::main(), NodeIndex(6)),
                    // stats_view.as_tab_handle(SurfaceIndex::main(), NodeIndex(8)),
                ],
            );

            dock_state.main_surface_mut().split_left(
                b,
                0.50,
                vec![world_view1.as_tab_handle(SurfaceIndex::main(), NodeIndex(4))],
            );

            // let [_, _] = dock_state.main_surface_mut().split_below(
            //     a,
            //     0.5,
            //     vec![],
            // );
            // let [_, _] = dock_state.main_surface_mut().split_below(
            //     b,
            //     0.5,
            //     vec![
            //         // world_view1.as_tab_handle(SurfaceIndex::main(), NodeIndex(5)),

            //     ],
            // );
        }

        let mut counter: usize = 9;

        Self {
            os_window: os_window.clone(),
            boxed_fn: Box::new(move |event_loop, window_id, event| {
                puffin::profile_function!();

                video_handle.tick();

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

                egui_renderer.handle_input(&os_window.borrow().window, &event);

                match event {
                    WindowEvent::Resized(size) => {
                        main_render_target_depth.resize(&ctx, size.width, size.height);
                        os_window.borrow_mut().re_configure(&ctx);
                        main_camera.resize(size.width as f32, size.height as f32);
                    }
                    WindowEvent::CloseRequested => {
                        event_loop.exit();
                    }
                    WindowEvent::KeyboardInput { event, .. } => {
                        if let Key::Named(NamedKey::Escape) = event.logical_key {
                            if event.state.is_pressed() {
                                os_window
                                    .borrow_mut()
                                    .window
                                    .set_cursor_grab(CursorGrabMode::None)
                                    .unwrap();
                                os_window.borrow_mut().window.set_cursor_visible(true);
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
                        puffin::GlobalProfiler::lock().new_frame();

                        puffin::profile_scope!("Total Frame Time");

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

                        let frame = {
                            puffin::profile_scope!("Get current texture (Blocks due to VSync)");
                            // Get Surface Texture
                            match os_window.borrow().surface.get_current_texture() {
                                Ok(frame) => frame,
                                Err(e) => {
                                    eprintln!("dropped frame: {e:?}");
                                    return;
                                }
                            }
                        };

                        // Try to update video texture
                        if let Some(new_frame) = video_handle.get_current_frame() {
                            video_demo.check_resize(&ctx, new_frame.resolution);

                            match new_frame.data {
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

                        let frame_view = &frame.texture.create_view(&Default::default());

                        let mut render_pass =
                            |camera: &Camera,
                             color_view: &wgpu::TextureView,
                             depth_view: &wgpu::TextureView| {
                                puffin::profile_scope!("Render pass");
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

                            egui_renderer.begin_frame(&os_window.borrow().window);

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

                            let inner_size = os_window.borrow().window.inner_size();
                            let outer_size = os_window.borrow().window.outer_size();

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

                            let value = video_handle.clone();

                            video_view.ui(move |ui: &mut egui::Ui| {
                                value.with(|video| {
                                    ui.label(format!("Movie avg framerate: {:.2}FPS", video.fps));

                                    ui.label(format!(
                                        "Movie length: {}",
                                        FancyDuration(video.total_duration)
                                    ));
                                });

                                ui.horizontal(|ui| {
                                    if ui.button("Pause").clicked() {
                                        value.pause();
                                        // let _ = value.try_send(MP4Command::Pause);
                                    }
                                    if ui.button("Play").clicked() {
                                        value.play();
                                        // let _ = value.try_send(MP4Command::Play);
                                    }
                                    if ui.button("Stop").clicked() {
                                        value.stop();
                                        // let _ = value.try_send(MP4Command::Stop);
                                    }
                                    if ui.button("SkipForward").clicked() {
                                        // let _ = value.try_send(MP4Command::SkipForward);
                                    }
                                    if ui.button("SkipBackward").clicked() {
                                        // let _ = value.try_send(MP4Command::SkipBackward);
                                    }
                                    if ui.button("Seek(Duration)").clicked() {
                                        // let _ =
                                        //     value.try_send(MP4Command::Seek(Duration::from_millis(0)));
                                    }
                                });

                                let prog = value.get_realtime_progress();
                                let mut new_prog = prog;

                                ui.horizontal(|ui| {
                                    let available_width = ui.available_width();

                                    ui.spacing_mut().slider_width = available_width;

                                    ui.add(Slider::new(&mut new_prog, 0.0..=1.0).show_value(false))
                                });

                                value.with(|v| {
                                    ui.horizontal(|ui: &mut egui::Ui| {
                                        let available_width = ui.available_width();

                                        ui.spacing_mut().slider_width = available_width / 2.0;

                                        ui.add(
                                            Slider::new(&mut v.play_speed, 0.0..=10.0)
                                                .show_value(true),
                                        );
                                    });
                                });

                                let mut seek_target = None;

                                let current = value
                                    .get_next_pts()
                                    .map(|t| FancyDuration(t))
                                    .unwrap_or(FancyDuration(Duration::from_millis(0)));

                                ui.label(format!(
                                    "Frame PTS: {}",
                                    value
                                        .get_frame_pts()
                                        .map(|t| FancyDuration(t))
                                        .unwrap_or(FancyDuration(Duration::from_millis(0)))
                                ));

                                value.with(|video| {
                                    if new_prog == prog {
                                        ui.label(format!(
                                            "Current Time: {}",
                                            FancyDuration(video.total_duration.mul_f64(prog))
                                        ));

                                        ui.label(format!("Next Frame: {}", current));
                                    } else {
                                        let new_duration = video.total_duration.mul_f64(new_prog);

                                        ui.label(format!(
                                            "Seek Target: {}",
                                            FancyDuration(new_duration)
                                        ));

                                        ui.label(format!("Next Frame: None"));

                                        seek_target = Some(new_duration);
                                    }
                                });

                                if let Some(seek_target) = seek_target.take() {
                                    value.seek(seek_target);
                                }

                                ui.add_space(12.0);

                                ui.label(format!(
                                    "Last frame decode latency {}",
                                    value
                                        .get_last_decode_cost()
                                        .map(|t| FancyDuration(t))
                                        .unwrap_or(FancyDuration(Duration::from_millis(0)))
                                ));

                                ui.label(format!(
                                    "Realtime PTS {}",
                                    FancyDuration(value.get_pts())
                                ));

                                ui.label(format!(
                                    "Realtime PTS {}",
                                    FancyDuration(value.get_pts())
                                ));

                                ui.label(format!("Dropped Frames: {}", value.get_dropped_frames()));

                                let (len, cap) = value.get_buffer_size();
                                let health = value.get_buffer_health();

                                ui.label(format!("Buffer Health: {len}/{cap}"));

                                ui.add(ProgressBar::new(health as f32));
                            });

                            profiler_view.ui(|ui| {
                                puffin_egui::profiler_ui(ui);
                            });

                            egui_renderer.end_frame_and_draw(
                                &ctx.device,
                                &ctx.queue,
                                &mut encoder,
                                &os_window.borrow().window,
                                &frame_view,
                                ScreenDescriptor {
                                    size_in_pixels: [
                                        os_window.borrow().surface_configuration.width,
                                        os_window.borrow().surface_configuration.height,
                                    ],
                                    pixels_per_point: os_window.borrow().window.scale_factor()
                                        as f32
                                        * scale_factor,
                                },
                            );

                            ctx.queue.submit(Some(encoder.finish()));
                        }

                        frame.present();
                    }
                    _ => {}
                };
            }),
        }
    }
}

impl ApplicationHandler for ApplicationHack {
    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        window_id: WindowId,
        event: WindowEvent,
    ) {
        (self.boxed_fn)(event_loop, window_id, event);
    }

    fn about_to_wait(&mut self, event_loop: &ActiveEventLoop) {
        self.os_window.borrow_mut().window.request_redraw();
    }

    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        // if let Some(span) = self.resumed_span.take() {
        //     span.end();
        // }
    }
}

fn main() -> Result<(), Box<dyn Error>> {
    env_logger::init();

    ffmpeg_next::init().unwrap();

    puffin::set_scopes_on(true);

    let event_loop = EventLoop::new()?;

    let (ctx, os_window) = GPUCtx::new(&event_loop);

    {
        os_window.borrow_mut().window.set_maximized(true)
    };

    let mut app = ApplicationHack::new(ctx, os_window);

    event_loop.run_app(&mut app)?;

    Ok(())
}
