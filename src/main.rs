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
use egui_dock::{DockArea, DockState, NodeIndex, Style, SurfaceIndex};
use egui_wgpu::wgpu::FilterMode;
use egui_wgpu::{wgpu, ScreenDescriptor};
use frontend::TabView;
use glam::*;
use std::cell::RefCell;
use std::error::Error;
use std::path::PathBuf;
use std::rc::Rc;
use std::time::{Duration, Instant};
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

    let mut vm = VM::new();

    // Choose a video
    // let video_path = get_random_file_from_directory("/Volumes/dev/Shared/mp4")
    //     .or_else(|| Some(PathBuf::from("/Users/cold/Desktop/YP-1R-05x13.mp4")))
    //     .unwrap();

    let video_path = PathBuf::from("/Users/cold/Desktop/YP-1R-05x13.mp4");

    println!("Now playing: {:?}", video_path);

    // Start playing
    let (video_thread, video_receiver, command_sender) = start(video_path);

    // Create the event loop
    let event_loop = EventLoop::new()?;

    // Init graphics
    let (ctx, mut os_window) = GPUCtx::new(&event_loop);
    let window_size = os_window.window.inner_size();
    let high_dpi_factor = 2.0 * os_window.window.scale_factor() as f32;

    os_window.window.set_maximized(true);

    // Create depth textures
    let mut main_render_target_depth =
        ViewTarget::create(&ctx, window_size.width, window_size.height);

    // Create egui
    let mut egui_renderer = EguiRenderer::new(
        &ctx.device,
        os_window.surface_configuration.format,
        None,
        1,
        &os_window.window,
    );

    // Init Examples
    let mut chunks_demo = ChunksDemo::create(&os_window.surface_configuration, &ctx);
    let mut video_demo = VideoDemo::create(&ctx, &os_window.surface_configuration);

    // Init canvas
    let canvas_size = [1000.0 * high_dpi_factor, 1000.0 * high_dpi_factor];
    let mut skia_canvas = Rc::new(RefCell::new(Canvas::new(
        canvas_size[0] as u32,
        canvas_size[1] as u32,
        high_dpi_factor,
    )));

    {
        let code = String::from(include_str!(
            "/Users/cold/w/pony-render/svg/tier-5/complex-drawing.svg"
        ));
        render_svg(code, &mut skia_canvas.borrow_mut());
    }

    let canvas_data = skia_canvas.borrow_mut().as_bytes()?;
    let skia_gpu_texture = GPUTexture::create(
        &ctx,
        canvas_size[0] as u32,
        canvas_size[1] as u32,
        wgpu::TextureFormat::Rgba8UnormSrgb,
        Filler0(0, 0, 0, 255),
        wgpu::TextureUsages::COPY_DST | wgpu::TextureUsages::TEXTURE_BINDING,
    );
    skia_gpu_texture.update(&ctx, &canvas_data);
    let canvas_texture_id = egui_renderer.renderer.register_native_texture(
        &ctx.device,
        &skia_gpu_texture.view,
        FilterMode::Linear,
    );

    // Secondary view render attachments

    // Main camera
    let mut main_camera = Camera::new(
        Vec3::new(-137.0, 0.0, 0.0),
        Vec2::ZERO,
        window_size.width as f32,
        window_size.height as f32,
    );
    let mut camera_controller = CameraController::new(20.0, 0.004);
    camera_controller.copy_camera_rotation(&main_camera);

    // Secondary camera

    // State
    let mut square_dist = 1.0;
    let scale_factor = 1.0;
    let mut focused = false;
    let mut last_frame = Instant::now();
    let mut use_secondary_camera = false;

    let mut language = String::from("js");
    let mut code = String::from(include_str!("../js/outline.js"));

    let mut frame_count = 0;

    // TODO: The API is trash
    // The Tabs Should Be Rc<RefCell> by default

    let world_view = frontend::WorldView::new(&ctx, &mut egui_renderer);

    let tab1 = frontend::CustomView::new(|ui| {
        ui.label("GL DONT CARE");
    });
    let mut quick_tab = frontend::QuickView::new();
    let tab3 = frontend::RegularView::new();
    let tab4 = frontend::RegularView::new();

    let mut dock_state = DockState::new(vec![
        tab1.as_tab_handle(SurfaceIndex::main(), NodeIndex(1)),
        world_view.as_tab_handle(SurfaceIndex::main(), NodeIndex(2)),
    ]);

    {
        let [a, b] = dock_state.main_surface_mut().split_left(
            NodeIndex::root(),
            0.5,
            vec![quick_tab.as_tab_handle(SurfaceIndex::main(), NodeIndex(3))],
        );
        let [_, _] = dock_state.main_surface_mut().split_below(
            a,
            0.5,
            vec![tab3.as_tab_handle(SurfaceIndex::main(), NodeIndex(4))],
        );
        let [_, _] = dock_state.main_surface_mut().split_below(
            b,
            0.5,
            vec![tab4.as_tab_handle(SurfaceIndex::main(), NodeIndex(5))],
        );
    }

    let mut counter: usize = 9;

    event_loop.run(|event, window_target| {
        window_target.set_control_flow(ControlFlow::Poll);

        // TODO: focus management to know which view should process the inputs

        if use_secondary_camera {
            world_view.with(|view| {
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



                        world_view.with(|view| {
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
                        frame_count += 1;

                        // TODO: trash again

                        world_view.with(|view| {
                            view.on_egui(&mut egui_renderer);
                        });

                        // This will be used for both main render pass and egui render pass
                        let frame_view = &frame.texture.create_view(&Default::default());

                        let mut render_pass =
                            |camera: &Camera,
                             color_view: &wgpu::TextureView,
                             depth_view: &wgpu::TextureView| {
                                let mut encoder =
                                    ctx.device.create_command_encoder(&Default::default());
                                {
                                    let mut view = SView::new(color_view, depth_view);
                                    let mut pass = view.render_pass(&mut encoder);
                                    chunks_demo.setup_dynamic_camera(&ctx, &camera);
                                    video_demo.setup_dynamic_camera(&ctx, &camera);
                                    chunks_demo.render(&mut pass);
                                    video_demo.render(&mut pass);
                                }

                                ctx.queue.submit([encoder.finish()]);
                            };

                        // Render main pass
                        // {
                        //     let color_view = frame_view;
                        //     let depth_view = &main_render_target_depth
                        //         .depth_stencil
                        //         .create_view(&Default::default());
                        //
                        //     render_pass(&main_camera, color_view, depth_view);
                        // }

                        // Render Second Pass
                        // TODO: trash again

                        quick_tab.ui(move |ui| {
                            ui.label(format!("FPS: {}", fps(delta)));
                        });

                        world_view.with(|view| {
                            view.render_to(&mut render_pass);
                        });

                        // Render ImGUI
                        {
                            let mut encoder =
                                ctx.device.create_command_encoder(&Default::default());

                            egui_renderer.begin_frame(&os_window.window);

                            let screen_descriptor = ScreenDescriptor {
                                size_in_pixels: [
                                    os_window.surface_configuration.width,
                                    os_window.surface_configuration.height,
                                ],
                                pixels_per_point: os_window.window.scale_factor() as f32
                                    * scale_factor,
                            };

                            let mut added_nodes = Vec::new();

                            DockArea::new(&mut dock_state)
                                .show_add_buttons(true)
                                .show_add_popup(true)
                                .style(Style::from_egui(egui_renderer.context().style().as_ref()))
                                .show(
                                    egui_renderer.context(),
                                    &mut HandleList(&mut added_nodes),
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

                            // egui::SidePanel::left("options_panel").show(
                            //     egui_renderer.context(),
                            //     |ui| {
                            //         gizmo_example.draw_options(ui);
                            //     },
                            // );

                            // egui::Window::new("Canvas Example")
                            //     .default_size([512.0, 612.0])
                            //     .resizable(true)
                            //     .hscroll(true)
                            //     .vscroll(true)
                            //     .show(egui_renderer.context(), |ui| {
                            //         ui.label(format!("Available Size {}", ui.available_size()));
                            //         ui.label(format!(
                            //             "Inner Size {:#?}",
                            //             os_window.window.inner_size()
                            //         ));
                            //         ui.label(format!("Size {:#?}", os_window.window.outer_size()));
                            //
                            //         ui.image(ImageSource::Texture(SizedTexture::new(
                            //             canvas_texture_id,
                            //             (
                            //                 canvas_size[0] / high_dpi_factor,
                            //                 canvas_size[0] / high_dpi_factor,
                            //             ),
                            //         )));
                            //
                            //         if ui.button("Add Square").clicked() {
                            //             square_dist += 50.0;
                            //
                            //             skia_canvas.borrow_mut().fill_rect(
                            //                 square_dist,
                            //                 square_dist,
                            //                 100.0 + square_dist,
                            //                 100.0 + square_dist,
                            //             );
                            //
                            //             let example_data =
                            //                 skia_canvas.borrow_mut().as_bytes().unwrap();
                            //             skia_gpu_texture.update(&ctx, &example_data);
                            //         }
                            //     });

                            // egui::Window::new("Camera")
                            //     .default_size([170.0, 260.0])
                            //     .resizable(true)
                            //     .hscroll(true)
                            //     .vscroll(true)
                            //     .show(egui_renderer.context(), |ui| {
                            //         ui.label(format!(
                            //             "Main Camera {}",
                            //             if !use_secondary_camera {
                            //                 "(active)"
                            //             } else {
                            //                 ""
                            //             }
                            //         ));
                            //         ui.label(format!("   X {}", main_camera.view.position.x));
                            //         ui.label(format!("   Y {}", main_camera.view.position.y));
                            //         ui.label(format!("   Z {}", main_camera.view.position.z));
                            //         ui.label(format!("   Pitch {}", main_camera.view.yaw_pitch.x));
                            //         ui.label(format!("   Yaw {}", main_camera.view.yaw_pitch.y));
                            //         ui.label(format!(
                            //             "Debug Camera {}",
                            //             if use_secondary_camera { "(active)" } else { "" }
                            //         ));
                            //         ui.label(format!("   X {}", secondary_camera.view.position.x));
                            //         ui.label(format!("   Y {}", secondary_camera.view.position.y));
                            //         ui.label(format!("   Z {}", secondary_camera.view.position.z));
                            //         ui.label(format!(
                            //             "   Pitch {}",
                            //             secondary_camera.view.yaw_pitch.x
                            //         ));
                            //         ui.label(format!(
                            //             "   Yaw {}",
                            //             secondary_camera.view.yaw_pitch.y
                            //         ));
                            //     });

                            // egui::Window::new("Chunk Manager")
                            //     .default_size([170.0, 260.0])
                            //     .resizable(true)
                            //     .hscroll(true)
                            //     .vscroll(true)
                            //     .show(egui_renderer.context(), |ui| {
                            //         if ui.button("Spawn Chunk").clicked() {
                            //             chunks_demo.spawn_chunk(&ctx);
                            //         }
                            //     });

                            // egui::Window::new("Video")
                            //     .resizable(true)
                            //     .hscroll(true)
                            //     .vscroll(true)
                            //     .show(egui_renderer.context(), |ui| {
                            //         if ui.button("Pause").clicked() {
                            //             command_sender.try_send(MP4Command::Pause);
                            //         }
                            //         if ui.button("Play").clicked() {
                            //             command_sender.try_send(MP4Command::Play);
                            //         }
                            //         if ui.button("Stop").clicked() {
                            //             command_sender.try_send(MP4Command::Stop);
                            //         }
                            //         if ui.button("SkipForward").clicked() {
                            //             command_sender.try_send(MP4Command::SkipForward);
                            //         }
                            //         if ui.button("SkipBackward").clicked() {
                            //             command_sender.try_send(MP4Command::SkipBackward);
                            //         }
                            //         if ui.button("Seek(Duration)").clicked() {
                            //             command_sender.try_send(MP4Command::Seek(Duration::from_millis(0)));
                            //         }
                            //     });

                            // egui::Window::new("Code Editor")
                            //     .resizable(true)
                            //     .hscroll(true)
                            //     .vscroll(true)
                            //     .default_height(600.0)
                            //     .show(egui_renderer.context(), |ui| {
                            //         let mut theme =
                            //             egui_extras::syntax_highlighting::CodeTheme::from_memory(
                            //                 ui.ctx(),
                            //                 ui.style(),
                            //             );
                            //
                            //         ui.collapsing("Theme", |ui| {
                            //             ui.group(|ui| {
                            //                 theme.ui(ui);
                            //                 theme.clone().store_in_memory(ui.ctx());
                            //             });
                            //         });
                            //
                            //         let mut layouter =
                            //             |ui: &egui::Ui, string: &str, wrap_width: f32| {
                            //                 let mut layout_job =
                            //                     egui_extras::syntax_highlighting::highlight(
                            //                         ui.ctx(),
                            //                         ui.style(),
                            //                         &theme,
                            //                         string,
                            //                         &mut language,
                            //                     );
                            //                 layout_job.wrap.max_width = wrap_width;
                            //                 ui.fonts(|f| f.layout_job(layout_job))
                            //             };
                            //
                            //         if ui.button("Eval").clicked() {
                            //             {
                            //                 skia_canvas.borrow_mut().clear();
                            //             }
                            //
                            //             vm.eval_with_canvas(code.clone(), skia_canvas.clone());
                            //
                            //             let new_texture_sync =
                            //                 skia_canvas.borrow_mut().as_bytes().unwrap();
                            //             skia_gpu_texture.update(&ctx, &new_texture_sync);
                            //         }
                            //
                            //         egui::ScrollArea::vertical().show(ui, |ui| {
                            //             ui.add(
                            //                 egui::TextEdit::multiline(&mut code)
                            //                     .font(egui::TextStyle::Monospace) // for cursor height
                            //                     .code_editor()
                            //                     .desired_rows(10)
                            //                     .lock_focus(true)
                            //                     .desired_width(f32::INFINITY)
                            //                     .layouter(&mut layouter),
                            //             );
                            //         });
                            //     });

                            egui_renderer.end_frame_and_draw(
                                &ctx.device,
                                &ctx.queue,
                                &mut encoder,
                                &os_window.window,
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

    drop(video_thread);

    Ok(())
}
