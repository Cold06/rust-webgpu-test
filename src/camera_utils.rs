use crate::camera_controller::CameraController;
use winit::event::WindowEvent;

pub fn process_camera_input(
    focused: bool,
    event: WindowEvent,
    camera_controller: &mut CameraController,
    mouse_delta: Option<(f64, f64)>,
) {
    if !focused {
        return;
    }

    if let Some((x, y)) = mouse_delta {
        camera_controller.process_mouse(x, y);
    }

    match event {
        WindowEvent::KeyboardInput { event, .. } => {
            camera_controller.process_keyboard(event.physical_key, event.state);
        }
        _ => {}
    }
}
