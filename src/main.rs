mod camera;
mod camera_controller;
mod camera_utils;
mod canvas;
mod chunk;
mod cube;
mod example;
mod gpu;
mod gpu_utils;
mod gui;
mod gui_utils;
mod multimath;
mod paint_utils;
mod pipelines;
mod video;
mod window;

use crate::camera_utils::process_camera_input;
use crate::example::Example;
use crate::gpu::{SView, ViewTarget};
use crate::gpu_utils::build_depth_texture;
use crate::gui::Gui;
use crate::gui_utils::GUICanvas;
use crate::video::{start, FrameData, PipelineEvent};
use crate::window::OSWindow;
use imgui::*;
use imgui_wgpu;
use pollster::block_on;
use std::error::Error;
use std::path::Path;
use std::time::Instant;
use winit::{
    dpi::LogicalSize,
    event::{ElementState, Event, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
    keyboard::{Key, NamedKey},
    window::CursorGrabMode,
};
use crate::camera::Camera;
use crate::camera_controller::CameraController;
use crate::multimath::{Vec2, Vec3};

fn main() -> Result<(), Box<dyn Error>> {
    env_logger::init();

    let (a, b) = start(
        Path::new("/Users/cold/Desktop/YP-1R-05x13.mp4")
            .canonicalize()
            .unwrap(),
    );

    let event_loop = EventLoop::new()?;

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

    window.init_configuration(&device);

    let mut gui = Gui::new(
        &window.window,
        &device,
        &queue,
        window.surface_configuration.format,
    );

    let size = window.window.inner_size();
    let mut example = Example::init(
        &window.surface_configuration,
        &device,
        &queue,
    );

    let mut example_size: [f32; 2] = [640.0, 480.0];
    let example_texture_id = {
        let texture_config = imgui_wgpu::TextureConfig {
            size: wgpu::Extent3d {
                width: example_size[0] as u32,
                height: example_size[1] as u32,
                ..Default::default()
            },
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
            ..Default::default()
        };
        let texture = imgui_wgpu::Texture::new(&device, &gui.renderer, texture_config);
        gui.renderer.textures.insert(texture)
    };

    let mut example_canvas = GUICanvas::new(&mut gui.renderer, &device, [250, 250]);

    example_canvas.with(|ctx| ctx.fill_rect(0.0, 0.0, 100.0, 100.0));
    example_canvas.update(&mut gui.renderer, &queue);

    let mut focused = false;
    let mut last_frame = Instant::now();
    let mut use_debug_camera = false;
    let mut square_dist = 0.0;

    let mut surface_depth = ViewTarget::create(&device, size.width, size.height);

    let mut imgui_view_depth = ViewTarget::create(&device, 512u32, 512u32);

    let mut camera = Camera::new(Vec3::new(), Vec2::new(), size.width as f32, size.height as f32);
    let mut camera_controller = CameraController::new(20.0, 0.004);
    camera_controller.copy_camera_rotation(&camera);

    let mut camera_debug = Camera::new(Vec3::new(), Vec2::new(), size.width as f32, size.height as f32);
    let mut camera_controller_debug = CameraController::new(200.0, 0.004);
    camera_controller.copy_camera_rotation(&camera_debug);

    event_loop.run(|event, window_target| {
        if let Ok(frame) = b.try_recv() {
            match frame {
                PipelineEvent::Data(frame) => match frame.data {
                    FrameData::PlanarYuv420(planes) => {
                        println!("Got frame {:?} Y {}", frame.pts, planes.y_plane[0]);
                    }
                },
                PipelineEvent::EOS => {
                    println!("Got end of stream");
                }
            }
        }

        window_target.set_control_flow(ControlFlow::Poll);

        let imgui_io = gui.imgui.io();
        let imgui_wants_mouse = imgui_io.want_capture_mouse;

        if use_debug_camera {
            process_camera_input(focused, event.clone(), &mut camera_controller_debug);
        } else {
            process_camera_input(focused, event.clone(), &mut camera_controller);
        }

        match event {
            Event::AboutToWait => {
                window.window.request_redraw();
            }
            Event::WindowEvent { ref event, .. } => match event {
                WindowEvent::Resized(size) => {
                    surface_depth.resize(&device, size.width, size.height);

                    window.re_configure(&device);

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
                    if !imgui_wants_mouse {
                        window
                            .window
                            .set_cursor_grab(CursorGrabMode::Locked)
                            .unwrap();
                        window.window.set_cursor_visible(false);
                        focused = true;
                    }
                }
                WindowEvent::RedrawRequested => {
                    let now = Instant::now();
                    let delta = now - last_frame;
                    last_frame = now;

                    gui.imgui.io_mut().update_delta_time(delta);

                    camera_controller
                        .update_camera(&mut camera, delta);

                    camera.compute();

                    gui.platform
                        .prepare_frame(gui.imgui.io_mut(), &window.window)
                        .expect("Failed to prepare frame");

                    let frame = match window.surface.get_current_texture() {
                        Ok(frame) => frame,
                        Err(e) => {
                            eprintln!("dropped frame: {e:?}");
                            return;
                        }
                    };

                    let ui = gui.imgui.frame();
                    example.update(ui.io().delta_time);
                    example.setup_dynamic_camera(&queue, &camera);

                    let color_view = &frame
                        .texture
                        .create_view(&wgpu::TextureViewDescriptor::default());

                    let depth_view = &surface_depth.depth_stencil
                        .create_view(&wgpu::TextureViewDescriptor::default());

                    example.render(&device, &queue, &SView::new(color_view, depth_view));

                    let mut new_example_size: Option<[f32; 2]> = None;

                    ui.window("Cube")
                        .size([512.0, 512.0], Condition::FirstUseEver)
                        .build(|| {
                            new_example_size = Some(ui.content_region_avail());
                            Image::new(example_texture_id, new_example_size.unwrap()).build(ui);
                        });

                    ui.window("My Texture Example").build(|| {
                        let size = Some(ui.content_region_avail()).unwrap();
                        Image::new(example_canvas.texture_id, [size[0], size[1] - 40.0]).build(ui);

                        if ui.button("Add Square") {
                            square_dist += 50.0;

                            example_canvas.with(|ctx| {
                                ctx.fill_rect(
                                    square_dist,
                                    square_dist,
                                    100.0 + square_dist,
                                    100.0 + square_dist,
                                )
                            });
                            example_canvas.update(&mut gui.renderer, &queue);
                        }
                    });

                    ui.window("Chunk Manager")
                        .size([170.0, 260.0], Condition::FirstUseEver)
                        .build(|| {
                            if ui.button("Spawn Chunk") {
                                example.spawn_chunk(&device);
                            }
                        });

                    ui.window("Camera")
                        .size([170.0, 260.0], Condition::FirstUseEver)
                        .position([1070.0, 12.0], Condition::FirstUseEver)
                        .build(|| {
                            ui.text(format!(
                                "Main Camera {}",
                                if !use_debug_camera { "(active)" } else { "" }
                            ));
                            ui.text(format!("   X {}", camera.view.position.x));
                            ui.text(format!("   Y {}", camera.view.position.y));
                            ui.text(format!("   Z {}", camera.view.position.z));
                            ui.text(format!("   Pitch {}", camera.view.yaw_pitch.x));
                            ui.text(format!("   Yaw {}", camera.view.yaw_pitch.y));
                            ui.text(format!(
                                "Debug Camera {}",
                                if use_debug_camera { "(active)" } else { "" }
                            ));
                            ui.text(format!("   X {}", camera_debug.view.position.x));
                            ui.text(format!("   Y {}", camera_debug.view.position.y));
                            ui.text(format!("   Z {}", camera_debug.view.position.z));
                            ui.text(format!(
                                "   Pitch {}",
                                camera_debug.view.yaw_pitch.x
                            ));
                            ui.text(format!("   Yaw {}", camera_debug.view.yaw_pitch.y));
                        });

                    if let Some(size) = new_example_size {
                        if size != example_size && size[0] >= 1.0 && size[1] >= 1.0 {
                            example_size = size;
                            let scale = &ui.io().display_framebuffer_scale;
                            let texture_config = imgui_wgpu::TextureConfig {
                                size: wgpu::Extent3d {
                                    width: (example_size[0] * scale[0]) as u32,
                                    height: (example_size[1] * scale[1]) as u32,
                                    ..Default::default()
                                },
                                usage: wgpu::TextureUsages::RENDER_ATTACHMENT
                                    | wgpu::TextureUsages::TEXTURE_BINDING,
                                ..Default::default()
                            };
                            gui.renderer.textures.replace(
                                example_texture_id,
                                imgui_wgpu::Texture::new(&device, &gui.renderer, texture_config),
                            );

                            camera_debug
                                .resize(example_size[0] * scale[0], example_size[1] * scale[1]);

                            imgui_view_depth.resize(
                                &device,
                                (example_size[0] * scale[0]) as u32,
                                (example_size[1] * scale[1]) as u32,
                            );
                        }

                        camera_controller_debug
                            .update_camera(&mut camera_debug, delta);
                        camera_debug.compute();
                        example.setup_dynamic_camera(&queue, &camera_debug);

                        let color_view = gui
                            .renderer
                            .textures
                            .get(example_texture_id)
                            .unwrap()
                            .view();

                        let depth_view = &imgui_view_depth.depth_stencil
                            .create_view(&wgpu::TextureViewDescriptor::default());

                        example.render(&device, &queue, &SView::new(color_view, depth_view));
                    }

                    gui.platform.prepare_render(&ui, &window.window);
                    gui.render(&device, &queue, color_view);

                    frame.present();
                }
                _ => {}
            },
            _ => {}
        }

        gui.platform
            .handle_event(gui.imgui.io_mut(), &window.window, &event);
    })?;

    drop(a);

    Ok(())
}
