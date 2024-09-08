use bitflags::bitflags;
use bytemuck::{Pod, Zeroable};

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
pub struct Vertex {
    pub pos: [f32; 4],
    pub normal: [f32; 4],
    pub tex_coord: [f32; 2],
}

fn vertex(pos: [i8; 3], normal: [i8; 3], tc: [i8; 2]) -> Vertex {
    Vertex {
        pos: [pos[0] as f32, pos[1] as f32, pos[2] as f32, 1.0],
        normal: [normal[0] as f32, normal[1] as f32, normal[2] as f32, 0.0],
        tex_coord: [tc[0] as f32, tc[1] as f32],
    }
}

pub struct ModelBundle {
    pub vertex_data: Vec<Vertex>,
    pub index_data: Vec<u16>,
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

pub fn create_vertices(faces: BlockFaces) -> ModelBundle {
    let face_count = faces.bits().count_ones() as usize;

    let mut bundle = ModelBundle {
        vertex_data: Vec::with_capacity(face_count * 4),
        index_data: Vec::with_capacity(face_count * 4),
    };

    let mut i_stack: u16 = 0;

    let mut push_quad = || {
        bundle.index_data.extend([
            i_stack + 0,
            i_stack + 1,
            i_stack + 2,
            i_stack + 2,
            i_stack + 3,
            i_stack + 0,
        ]);

        i_stack += 4;
    };

    if faces.contains(BlockFaces::Right) {
        bundle.vertex_data.extend([
            vertex([-1, -1, 1], [0, 0, 1], [0, 0]),
            vertex([1, -1, 1], [0, 0, 1], [1, 0]),
            vertex([1, 1, 1], [0, 0, 1], [1, 1]),
            vertex([-1, 1, 1], [0, 0, 1], [0, 1]),
        ]);

        push_quad();
    }

    if faces.contains(BlockFaces::Left) {
        bundle.vertex_data.extend([
            vertex([-1, 1, -1], [0, 0, -1], [1, 0]),
            vertex([1, 1, -1], [0, 0, -1], [0, 0]),
            vertex([1, -1, -1], [0, 0, -1], [0, 1]),
            vertex([-1, -1, -1], [0, 0, -1], [1, 1]),
        ]);

        push_quad();
    }

    if faces.contains(BlockFaces::Back) {
        bundle.vertex_data.extend([
            vertex([1, -1, -1], [1, 0, 0], [0, 0]),
            vertex([1, 1, -1], [1, 0, 0], [1, 0]),
            vertex([1, 1, 1], [1, 0, 0], [1, 1]),
            vertex([1, -1, 1], [1, 0, 0], [0, 1]),
        ]);

        push_quad();
    }

    if faces.contains(BlockFaces::Front) {
        bundle.vertex_data.extend([
            vertex([-1, -1, 1], [-1, 0, 0], [1, 0]),
            vertex([-1, 1, 1], [-1, 0, 0], [0, 0]),
            vertex([-1, 1, -1], [-1, 0, 0], [0, 1]),
            vertex([-1, -1, -1], [-1, 0, 0], [1, 1]),
        ]);

        push_quad();
    }

    if faces.contains(BlockFaces::Top) {
        bundle.vertex_data.extend([
            vertex([1, 1, -1], [0, 1, 0], [1, 0]),
            vertex([-1, 1, -1], [0, 1, 0], [0, 0]),
            vertex([-1, 1, 1], [0, 1, 0], [0, 1]),
            vertex([1, 1, 1], [0, 1, 0], [1, 1]),
        ]);

        push_quad();
    }

    if faces.contains(BlockFaces::Bottom) {
        bundle.vertex_data.extend([
            vertex([1, -1, 1], [0, -1, 0], [0, 0]),
            vertex([-1, -1, 1], [0, -1, 0], [1, 0]),
            vertex([-1, -1, -1], [0, -1, 0], [1, 1]),
            vertex([1, -1, -1], [0, -1, 0], [0, 1]),
        ]);

        push_quad();
    }

    bundle
}
