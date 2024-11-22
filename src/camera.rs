use crate::multimath::{as_sphere_coord, to_rad};
use glam::*;

pub struct ProjectionMatrix {
    pub aspect: f32,
    pub fov_y: f32,
    pub z_near: f32,
    pub z_far: f32,
    pub matrix: Mat4,
}

impl ProjectionMatrix {
    pub fn new(width: f32, height: f32, fov_y: f32, z_near: f32, z_far: f32) -> Self {
        Self {
            aspect: width / height,
            fov_y,
            z_near,
            z_far,
            matrix: Mat4::IDENTITY,
        }
    }

    pub fn resize(&mut self, width: f32, height: f32) {
        self.aspect = width / height;
    }

    pub fn compute(&mut self) {
        self.matrix = Mat4::perspective_rh(self.fov_y, self.aspect, self.z_near, self.z_far);
    }
}

pub struct ViewMatrix {
    pub position: Vec3,
    pub yaw_pitch: Vec2,
    pub matrix: Mat4,
    pub center: Vec3,
}

impl ViewMatrix {
    pub fn new(position: Vec3, yaw_pitch: Vec2) -> Self {
        Self {
            position,
            yaw_pitch,
            center: Vec3::ZERO,
            matrix: Mat4::IDENTITY,
        }
    }

    fn compute(&mut self) {
        self.center = as_sphere_coord(self.yaw_pitch);

        self.center = self.center.normalize();

        self.center += self.position;

        self.matrix = Mat4::look_at_rh(self.position, self.center, Vec3::Y);
    }
}

pub struct Camera {
    pub matrix: Mat4,
    pub view: ViewMatrix,
    pub projection: ProjectionMatrix,
    pub width: f32,
    pub height: f32,
}

impl Camera {
    pub fn resize(&mut self, w: f32, h: f32) {
        self.projection.resize(w, h);
    }

    pub fn new(position: Vec3, yaw_pitch: Vec2, width: f32, height: f32) -> Self {
        let mut camera = Self {
            matrix: Mat4::IDENTITY,
            view: ViewMatrix::new(position, yaw_pitch),
            projection: ProjectionMatrix::new(width, height, std::f32::consts::PI / 4.0 , 0.1, 10000.0),
            width,
            height,
        };

        camera.compute();

        camera
    }

    pub fn check_resize<F: FnOnce()>(&mut self, new_width: f32, new_height: f32, cb: F) {
        if self.width != new_width || self.height != new_height {
            self.width = new_width;
            self.height = new_height;

            self.resize(self.width, self.height);

            self.compute();

            cb();
        }
    }

    pub fn compute(&mut self) {
        self.view.compute();
        self.projection.compute();
        self.matrix = self.projection.matrix * self.view.matrix;
    }
}
