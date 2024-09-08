use crate::multimath::{to_rad, Mat4, Vec2, Vec3};

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
            matrix: Mat4::new(),
        }
    }

    pub fn resize(&mut self, width: f32, height: f32) {
        self.aspect = width / height;
    }

    pub fn compute(&mut self) {
        self.matrix
            .as_projection_matrix(self.z_near, self.z_far, self.fov_y, self.aspect, 1.0);
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
            center: Vec3::new(),
            matrix: Mat4::new(),
        }
    }

    fn compute(&mut self) {
        self.center.as_sphere_coord(&self.yaw_pitch);
        self.center.normalize_mut();

        self.center.add_mut(&self.position);

        self.matrix.look_at(&self.position, &self.center, &WORLD_UP);
    }
}

pub struct Camera {
    pub matrix: Mat4,
    pub view: ViewMatrix,
    pub projection: ProjectionMatrix,
}

const WORLD_UP: Vec3 = Vec3 {
    x: 0.0,
    y: 1.0,
    z: 0.0,
};

impl Camera {
    pub fn resize(&mut self, w: f32, h: f32) {
        self.projection.resize(w, h);
    }

    pub fn new(position: Vec3, yaw_pitch: Vec2, width: f32, height: f32) -> Self {
        let mut camera = Self {
            matrix: Mat4::new(),
            view: ViewMatrix::new(position, yaw_pitch),
            projection: ProjectionMatrix::new(width, height, to_rad(90.0), 0.1, 100.0),
        };

        camera.compute();

        camera
    }

    pub fn compute(&mut self) {
        self.view.compute();
        self.projection.compute();
        self.matrix
            .multiply_from(&self.projection.matrix, &self.view.matrix);
    }
}
