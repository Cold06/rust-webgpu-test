use crate::camera::Camera;
use crate::multimath::{Vec2, Vec3};
use cgmath::num_traits::clamp;
use std::f32::consts::FRAC_PI_2;
use std::time::Duration;
use winit::event::*;
use winit::keyboard::{KeyCode, PhysicalKey};

const SAFE_FRAC_PI_2: f32 = FRAC_PI_2 - 0.0001;

#[derive(Debug)]
pub struct CameraController {
    amount_left: f32,
    amount_right: f32,
    amount_forward: f32,
    amount_backward: f32,
    amount_up: f32,
    amount_down: f32,
    speed: f32,
    sensitivity: f32,
    yaw_pitch: Vec2,
}

impl CameraController {
    pub fn new(speed: f32, sensitivity: f32) -> Self {
        Self {
            amount_left: 0.0,
            amount_right: 0.0,
            amount_forward: 0.0,
            amount_backward: 0.0,
            amount_up: 0.0,
            amount_down: 0.0,
            speed,
            sensitivity,
            yaw_pitch: Vec2 { x: 0.0, y: 0.0 },
        }
    }

    pub fn copy_camera_rotation(&mut self, camera: &Camera) {
        self.yaw_pitch = camera.view.yaw_pitch;
    }

    pub fn process_keyboard(&mut self, key: PhysicalKey, state: ElementState) -> bool {
        let amount = if state == ElementState::Pressed {
            1.0
        } else {
            0.0
        };
        match key {
            PhysicalKey::Code(KeyCode::KeyW) | PhysicalKey::Code(KeyCode::ArrowUp) => {
                self.amount_forward = amount;
                true
            }
            PhysicalKey::Code(KeyCode::KeyS) | PhysicalKey::Code(KeyCode::ArrowDown) => {
                self.amount_backward = amount;
                true
            }
            PhysicalKey::Code(KeyCode::KeyA) | PhysicalKey::Code(KeyCode::ArrowLeft) => {
                self.amount_left = amount;
                true
            }
            PhysicalKey::Code(KeyCode::KeyD) | PhysicalKey::Code(KeyCode::ArrowRight) => {
                self.amount_right = amount;
                true
            }
            PhysicalKey::Code(KeyCode::Space) => {
                self.amount_up = amount;
                true
            }
            PhysicalKey::Code(KeyCode::ShiftLeft) => {
                self.amount_down = amount;
                true
            }
            _ => false,
        }
    }

    pub fn process_mouse(&mut self, mouse_dx: f64, mouse_dy: f64) {
        self.yaw_pitch.x += mouse_dx as f32 * self.sensitivity;
        self.yaw_pitch.y += -mouse_dy as f32 * self.sensitivity;
        self.yaw_pitch.y = clamp(self.yaw_pitch.y, -SAFE_FRAC_PI_2, SAFE_FRAC_PI_2);
    }

    pub fn update_camera(&mut self, camera: &mut Camera, dt: Duration) {
        camera.view.yaw_pitch.x = self.yaw_pitch.x;
        camera.view.yaw_pitch.y = self.yaw_pitch.y;
        camera.view.yaw_pitch.y = clamp(camera.view.yaw_pitch.y, -SAFE_FRAC_PI_2, SAFE_FRAC_PI_2);

        let dt = dt.as_secs_f32();

        let (yaw_sin, yaw_cos) = camera.view.yaw_pitch.x.sin_cos();
        let mut forward = Vec3::from_components(yaw_cos, 0.0, yaw_sin);
        forward.normalize_mut();

        let mut right = Vec3::from_components(-yaw_sin, 0.0, yaw_cos);
        right.normalize_mut();

        let amount_forward = (self.amount_forward - self.amount_backward) * self.speed * dt;
        let amount_right = (self.amount_right - self.amount_left) * self.speed * dt;

        forward.multiply_scalar(amount_forward);
        right.multiply_scalar(amount_right);

        camera.view.position.add_mut(&forward);
        camera.view.position.add_mut(&right);

        camera.view.position.y += (self.amount_up - self.amount_down) * self.speed * dt;
    }
}
