use bitflags::bitflags;
use bytemuck::{Pod, Zeroable};
use noise::{Blend, Fbm, NoiseFn, Perlin, RidgedMulti, Seedable};
#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
pub struct Vertex {
    pub pos: [f32; 4],
    pub normal: [f32; 4],
    pub tex_coord: [f32; 2],
}

fn vertex(pos: [f32; 3], normal: [i8; 3], tc: [i8; 2]) -> Vertex {
    Vertex {
        pos: [pos[0] as f32, pos[1] as f32, pos[2] as f32, 1.0],
        normal: [normal[0] as f32, normal[1] as f32, normal[2] as f32, 0.0],
        tex_coord: [tc[0] as f32, tc[1] as f32],
    }
}

pub struct ModelBundle {
    pub vertex_data: Vec<Vertex>,
    pub index_data: Vec<u32>,
}

struct GenModel {
    pub vertex_data: Vec<Vertex>,
    pub index_data: Vec<u32>,
    pub top_stack: u32,
}

impl GenModel {
    fn new() -> Self {
        Self {
            top_stack: 0,
            index_data: Vec::with_capacity(10_000),
            vertex_data: Vec::with_capacity(10_000),
        }
    }
}

#[rustfmt::skip]
bitflags! {
    pub struct BlockFaces: u32 {
        const None    = 0b000000;
        const Top     = 0b000001;
        const Bottom  = 0b000010;
        const Left    = 0b000100;
        const Right   = 0b001000;
        const Front   = 0b010000;
        const Back    = 0b100000;
        const All     = Self::Top.bits()
                        | Self::Bottom.bits()
                        | Self::Left.bits()
                        | Self::Right.bits()
                        | Self::Front.bits()
                        | Self::Back.bits();
    }
}

#[rustfmt::skip]
pub fn add_faces(faces: BlockFaces, model: &mut GenModel, x: f32, y: f32, z: f32) {
    let mut push_quad = || {
        model.index_data.extend([
            model.top_stack + 0,
            model.top_stack + 1,
            model.top_stack + 2,
            model.top_stack + 2,
            model.top_stack + 3,
            model.top_stack + 0,
        ]);

        model.top_stack += 4;
    };

    if faces.contains(BlockFaces::Right) {
        model.vertex_data.extend([
            vertex([-1.0 + x, -1.0 + y,  1.0 + z], [0, 0, 1], [0, 0]),
            vertex([ 1.0 + x, -1.0 + y,  1.0 + z], [0, 0, 1], [1, 0]),
            vertex([ 1.0 + x,  1.0 + y,  1.0 + z], [0, 0, 1], [1, 1]),
            vertex([-1.0 + x,  1.0 + y,  1.0 + z], [0, 0, 1], [0, 1]),
        ]);

        push_quad();
    }

    if faces.contains(BlockFaces::Left) {
        model.vertex_data.extend([
            vertex([-1.0 + x,  1.0 + y, -1.0 + z], [0, 0, -1], [1, 0]),
            vertex([ 1.0 + x,  1.0 + y, -1.0 + z], [0, 0, -1], [0, 0]),
            vertex([ 1.0 + x, -1.0 + y, -1.0 + z], [0, 0, -1], [0, 1]),
            vertex([-1.0 + x, -1.0 + y, -1.0 + z], [0, 0, -1], [1, 1]),
        ]);

        push_quad();
    }

    if faces.contains(BlockFaces::Back) {
        model.vertex_data.extend([
            vertex([ 1.0 + x, -1.0 + y, -1.0 + z], [1, 0, 0], [0, 0]),
            vertex([ 1.0 + x,  1.0 + y, -1.0 + z], [1, 0, 0], [1, 0]),
            vertex([ 1.0 + x,  1.0 + y,  1.0 + z], [1, 0, 0], [1, 1]),
            vertex([ 1.0 + x, -1.0 + y,  1.0 + z], [1, 0, 0], [0, 1]),
        ]);

        push_quad();
    }

    if faces.contains(BlockFaces::Front) {
        model.vertex_data.extend([
            vertex([-1.0 + x, -1.0 + y,  1.0 + z], [-1, 0, 0], [1, 0]),
            vertex([-1.0 + x,  1.0 + y,  1.0 + z], [-1, 0, 0], [0, 0]),
            vertex([-1.0 + x,  1.0 + y, -1.0 + z], [-1, 0, 0], [0, 1]),
            vertex([-1.0 + x, -1.0 + y, -1.0 + z], [-1, 0, 0], [1, 1]),
        ]);

        push_quad();
    }

    if faces.contains(BlockFaces::Top) {
        model.vertex_data.extend([
            vertex([ 1.0 + x, 1.0 + y, -1.0 + z], [0, 1, 0], [1, 0]),
            vertex([-1.0 + x, 1.0 + y, -1.0 + z], [0, 1, 0], [0, 0]),
            vertex([-1.0 + x, 1.0 + y,  1.0 + z], [0, 1, 0], [0, 1]),
            vertex([ 1.0 + x, 1.0 + y,  1.0 + z], [0, 1, 0], [1, 1]),
        ]);

        push_quad();
    }

    if faces.contains(BlockFaces::Bottom) {
        model.vertex_data.extend([
            vertex([ 1.0 + x, -1.0 + y,  1.0 + z], [0, -1, 0], [0, 0]),
            vertex([-1.0 + x, -1.0 + y,  1.0 + z], [0, -1, 0], [1, 0]),
            vertex([-1.0 + x, -1.0 + y, -1.0 + z], [0, -1, 0], [1, 1]),
            vertex([ 1.0 + x, -1.0 + y, -1.0 + z], [0, -1, 0], [0, 1]),
        ]);

        push_quad();
    }
}

pub fn generate_full_mesh(x: i32, y: i32, z: i32) -> ModelBundle {
    let perlin = Perlin::default();
    let ridged = RidgedMulti::<Perlin>::default();
    let fbm = Fbm::<Perlin>::default();
    let blend = Blend::new(perlin, ridged, fbm);

    let c_x = x;
    let c_y = y;
    let c_z = z;

    let size = 16;

    let mut model = GenModel::new();

    for x in 0..size {
        for y in 0..size {
            for z in 0..size {

                let x = x + c_x;
                let y = y + c_y;
                let z = z + c_z;

                let noise_scale = 0.1;

                let get_at = |x, y, z| {
                    blend.get([
                        (x as f64) * noise_scale,
                        (y as f64) * noise_scale,
                        (z as f64) * noise_scale,
                    ]) > 0.2
                };

                let mut face = BlockFaces::None;

                let self_block = get_at(x + 0, y + 0, z + 0);

                if !self_block {
                    continue;
                }

                {
                    //                                           // Y
                    face.set(BlockFaces::Top, !get_at(x + 0, y + 1, z + 0));
                    face.set(BlockFaces::Bottom, !get_at(x + 0, y - 1, z + 0));

                    //                                                        Z
                    face.set(BlockFaces::Left, !get_at(x + 0, y + 0, z - 1));
                    face.set(BlockFaces::Right, !get_at(x + 0, y + 0, z + 1));

                    //                                  X
                    face.set(BlockFaces::Front, !get_at(x - 1, y + 0, z + 0));
                    face.set(BlockFaces::Back, !get_at(x + 1, y + 0, z + 0));
                }

                add_faces(
                    face,
                    &mut model,
                    (x as f32)  * 2.0,
                    (y as f32)  * 2.0,
                    (z as f32)  * 2.0,
                );
            }
        }
    }

    ModelBundle {
        vertex_data: model.vertex_data,
        index_data: model.index_data,
    }
}