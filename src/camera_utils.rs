use crate::camera_controller::CameraController;
use winit::event::WindowEvent;

pub fn process_camera_input(
    focused: bool,
    event: WindowEvent,
    camera_controller: &mut CameraController,
) {
    if !focused {
        return;
    }

    match event {
        WindowEvent::KeyboardInput { event, .. } => {
            camera_controller.process_keyboard(event.physical_key, event.state);
        }

        WindowEvent::CursorMoved { position, .. } => {
            println!("Mouse delta {:?}", position);
            camera_controller.process_mouse(position.x, position.y);
        }
        _ => {}
    }
}
