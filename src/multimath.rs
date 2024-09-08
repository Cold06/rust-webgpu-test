use std::fmt::{Debug, Formatter, Result};

pub const PI: f32 = std::f32::consts::PI;

pub const DEG_2_RAD: f32 = PI / 180.0;

#[repr(C)]
#[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub struct Mat4(pub [f32; 16]);

pub fn to_rad(deg: f32) -> f32 {
    deg * DEG_2_RAD
}

impl Debug for Mat4 {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        write!(
            f,
            "Mat4[{:.3?}, {:.3?}, {:.3?}, {:.3?}\n{:.3?}, {:.3?}, {:.3?}, {:.3?}\n{:.3?}, {:.3?}, {:.3?}, {:.3?}\n{:.3?}, {:.3?}, {:.3?}, {:.3?}]",
            self.0[0], self.0[1], self.0[2], self.0[3],
            self.0[4], self.0[5], self.0[6], self.0[7],
            self.0[8], self.0[9], self.0[10], self.0[11],
            self.0[12], self.0[13], self.0[14], self.0[15],
        )
    }
}

impl Mat4 {
    #[rustfmt::skip]
    pub fn new() -> Self {
        Self([
            1.0, 0.0, 0.0, 0.0,
            0.0, 1.0, 0.0, 0.0,
            0.0, 0.0, 1.0, 0.0,
            0.0, 0.0, 0.0, 1.0,
        ])
    }

    pub fn as_perspective(
        &mut self,
        left: f32,
        right: f32,
        top: f32,
        bottom: f32,
        near: f32,
        far: f32,
    ) {
        let x = 2.0 * near / (right - left);
        let y = 2.0 * near / (top - bottom);

        let a = (right + left) / (right - left);
        let b = (top + bottom) / (top - bottom);
        let c = -(far + near) / (far - near);
        let d = -2.0 * far * near / (far - near);

        self.0[0] = x;
        self.0[4] = 0.0;
        self.0[8] = a;
        self.0[12] = 0.0;

        self.0[1] = 0.0;
        self.0[5] = y;
        self.0[9] = b;
        self.0[13] = 0.0;

        self.0[2] = 0.0;
        self.0[6] = 0.0;
        self.0[10] = c;
        self.0[14] = d;

        self.0[3] = 0.0;
        self.0[7] = 0.0;
        self.0[11] = -1.0;
        self.0[15] = 0.0;
    }

    pub fn as_projection_matrix(&mut self, near: f32, far: f32, fov: f32, aspect: f32, zoom: f32) {
        let top = near * (0.5 * fov).tan() / zoom;

        let height = 2.0 * top;
        let width = aspect * height;
        let left = -0.5 * width;

        self.as_perspective(left, left + width, top, top - height, near, far);
    }

    pub fn look_at(&mut self, eye: &Vec3, center: &Vec3, up: &Vec3) {
        let mut dir = Vec3::from_sub_vectors(center, eye);
        dir.normalize_mut();

        let f = Vec3::from(&dir);

        let mut s = Vec3::from_cross(&f, &up);
        s.normalize_mut();

        let u = Vec3::from_cross(&s, &f);

        #[rustfmt::skip]
        self.set_common(
            s.x, u.x, -f.x,
            s.y, u.y, -f.y,
            s.z, u.z, -f.z,
            -eye.dot(&s),
            -eye.dot(&u),
            eye.dot(&f),
        );
    }

    pub fn set_common(
        &mut self,
        r0c0: f32,
        r0c1: f32,
        r0c2: f32,

        r1c0: f32,
        r1c1: f32,
        r1c2: f32,

        r2c0: f32,
        r2c1: f32,
        r2c2: f32,

        r3c0: f32,
        r3c1: f32,
        r3c2: f32,
    ) {
        self.0[0] = r0c0;
        self.0[1] = r0c1;
        self.0[2] = r0c2;

        self.0[4] = r1c0;
        self.0[5] = r1c1;
        self.0[6] = r1c2;

        self.0[8] = r2c0;
        self.0[9] = r2c1;
        self.0[10] = r2c2;

        self.0[12] = r3c0;
        self.0[13] = r3c1;
        self.0[14] = r3c2;
    }

    pub fn multiply_from(&mut self, a: &Mat4, b: &Mat4) {
        let ae = a.0;
        let be = b.0;

        let a11 = ae[0];
        let a12 = ae[4];
        let a13 = ae[8];
        let a14 = ae[12];

        let a21 = ae[1];
        let a22 = ae[5];
        let a23 = ae[9];
        let a24 = ae[13];

        let a31 = ae[2];
        let a32 = ae[6];
        let a33 = ae[10];
        let a34 = ae[14];

        let a41 = ae[3];
        let a42 = ae[7];
        let a43 = ae[11];
        let a44 = ae[15];

        let b11 = be[0];
        let b12 = be[4];
        let b13 = be[8];
        let b14 = be[12];

        let b21 = be[1];
        let b22 = be[5];
        let b23 = be[9];
        let b24 = be[13];

        let b31 = be[2];
        let b32 = be[6];
        let b33 = be[10];
        let b34 = be[14];

        let b41 = be[3];
        let b42 = be[7];
        let b43 = be[11];
        let b44 = be[15];

        self.0[0] = a11 * b11 + a12 * b21 + a13 * b31 + a14 * b41;
        self.0[4] = a11 * b12 + a12 * b22 + a13 * b32 + a14 * b42;
        self.0[8] = a11 * b13 + a12 * b23 + a13 * b33 + a14 * b43;
        self.0[12] = a11 * b14 + a12 * b24 + a13 * b34 + a14 * b44;

        self.0[1] = a21 * b11 + a22 * b21 + a23 * b31 + a24 * b41;
        self.0[5] = a21 * b12 + a22 * b22 + a23 * b32 + a24 * b42;
        self.0[9] = a21 * b13 + a22 * b23 + a23 * b33 + a24 * b43;
        self.0[13] = a21 * b14 + a22 * b24 + a23 * b34 + a24 * b44;

        self.0[2] = a31 * b11 + a32 * b21 + a33 * b31 + a34 * b41;
        self.0[6] = a31 * b12 + a32 * b22 + a33 * b32 + a34 * b42;
        self.0[10] = a31 * b13 + a32 * b23 + a33 * b33 + a34 * b43;
        self.0[14] = a31 * b14 + a32 * b24 + a33 * b34 + a34 * b44;

        self.0[3] = a41 * b11 + a42 * b21 + a43 * b31 + a44 * b41;
        self.0[7] = a41 * b12 + a42 * b22 + a43 * b32 + a44 * b42;
        self.0[11] = a41 * b13 + a42 * b23 + a43 * b33 + a44 * b43;
        self.0[15] = a41 * b14 + a42 * b24 + a43 * b34 + a44 * b44;
    }
}

#[inline]
pub fn one_when_zero(value: f32) -> f32 {
    if value == 0.0 {
        1.0
    } else {
        value
    }
}

#[repr(C)]
#[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub struct Vec2 {
    pub x: f32,
    pub y: f32,
}

impl Vec2 {
    pub fn new() -> Self {
        Self { x: 0.0, y: 0.0 }
    }
}

impl Debug for Vec2 {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        write!(f, "Vec2{{ x: {:.3?}, y: {:.3?}}}", self.x, self.y,)
    }
}

#[repr(C)]
#[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub struct Vec3 {
    pub x: f32,
    pub y: f32,
    pub z: f32,
}

impl Debug for Vec3 {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        write!(
            f,
            "Vec3{{ x: {:.3?}, y: {:.3?}, z: {:.3?}}}",
            self.x, self.y, self.z
        )
    }
}

impl Vec3 {
    pub fn add_mut(&mut self, vec: &Vec3) {
        self.x += vec.x;
        self.y += vec.y;
        self.z += vec.z;
    }

    pub fn new() -> Self {
        Self {
            x: 0.0,
            y: 0.0,
            z: 0.0,
        }
    }

    pub fn from_components(x: f32, y: f32, z: f32) -> Self {
        Self { x, y, z }
    }

    pub fn set(&mut self, x: f32, y: f32, z: f32) {
        self.x = x;
        self.y = y;
        self.z = z;
    }

    pub fn length_squared(&self) -> f32 {
        self.x * self.x + self.y * self.y + self.z * self.z
    }

    pub fn length(&self) -> f32 {
        self.length_squared().sqrt()
    }

    pub fn multiply_scalar(&mut self, scalar: f32) {
        self.x *= scalar;
        self.y *= scalar;
        self.z *= scalar;
    }

    pub fn divide_scalar(&mut self, scalar: f32) {
        self.multiply_scalar(1.0 / scalar);
    }

    pub fn normalize_mut(&mut self) {
        self.divide_scalar(one_when_zero(self.length()));
    }

    pub fn from_cross(a: &Vec3, b: &Vec3) -> Self {
        Self::from_components(
            a.y * b.z - a.z * b.y,
            a.z * b.x - a.x * b.z,
            a.x * b.y - a.y * b.x,
        )
    }

    pub fn as_sphere_coord(&mut self, v: &Vec2) {
        let (sy, cy) = v.y.sin_cos();
        let (sx, cx) = v.x.sin_cos();

        self.x = cy * cx;
        self.y = sy;
        self.z = cy * sx;
    }

    pub fn sub_vectors(&mut self, a: &Vec3, b: &Vec3) {
        self.set(a.x - b.x, a.y - b.y, a.z - b.z);
    }

    pub fn from_sub_vectors(a: &Vec3, b: &Vec3) -> Self {
        let mut vec = Self::new();
        vec.sub_vectors(&a, &b);
        vec
    }

    pub fn dot(&self, v: &Vec3) -> f32 {
        self.x * v.x + self.y * v.y + self.z * v.z
    }

    pub fn from(v: &Vec3) -> Self {
        Self::from_components(v.x.clone(), v.y.clone(), v.z.clone())
    }
}

#[repr(C)]
#[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub struct Vec4 {
    pub x: f32,
    pub y: f32,
    pub z: f32,
    pub w: f32,
}

impl Vec4 {
    pub fn new() -> Self {
        Self {
            x: 0.0,
            y: 0.0,
            z: 0.0,
            w: 0.0,
        }
    }

    pub fn from_components(x: f32, y: f32, z: f32, w: f32) -> Self {
        Self { x, y, z, w }
    }
}
