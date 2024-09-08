use crate::camera_controller::CameraController;
use winit::event::{DeviceEvent, Event, WindowEvent};

pub fn process_camera_input(
    focused: bool,
    event: Event<()>,
    camera_controller: &mut CameraController,
) {
    if !focused {
        return;
    }

    match event {
        Event::WindowEvent { event, .. } => match event {
            WindowEvent::KeyboardInput { event, .. } => {
                camera_controller.process_keyboard(event.physical_key, event.state);
            }
            _ => {}
        },
        Event::DeviceEvent { event, .. } => match event {
            DeviceEvent::MouseMotion { delta } => {
                camera_controller.process_mouse(delta.0, delta.1);
            }
            _ => {}
        },
        _ => {}
    }
}
