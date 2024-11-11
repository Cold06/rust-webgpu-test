use bytemuck::{NoUninit, Pod};
use egui_wgpu::wgpu;
use egui_wgpu::wgpu::util::DeviceExt;
use crate::gpu::GPUCtx;

pub struct GPUBuffer {
    buffer: wgpu::Buffer,
    usage: wgpu::BufferUsages,
}

#[derive(Clone, Copy, Debug)]
pub enum BufferType {
    Vertex,
    Index,
    Uniform,
    StorageRO,
    StorageRW
}

const fn get_visibility(ty: BufferType) -> wgpu::ShaderStages {
    match ty {
        BufferType::Vertex => wgpu::ShaderStages::VERTEX,
        BufferType::Index => wgpu::ShaderStages::VERTEX,
        BufferType::Uniform => wgpu::ShaderStages::VERTEX_FRAGMENT,
        BufferType::StorageRO | BufferType::StorageRW => {
            let vertex = 1 << 0;
            let fragment = 1 << 1;
            let compute = 1 << 2;
            wgpu::ShaderStages::from_bits(vertex | fragment | compute).unwrap()
        },
    }
}

const fn get_usage(ty: BufferType) -> wgpu::BufferUsages {
    match ty {
        BufferType::Vertex => wgpu::BufferUsages::VERTEX,
        BufferType::Index => wgpu::BufferUsages::INDEX,
        BufferType::Uniform => {
            let uniform = 1 << 6;
            let copy_dst = 1 << 3;
            wgpu::BufferUsages::from_bits(uniform | copy_dst).unwrap()
        }
        BufferType::StorageRO => {
            let storage = 1 << 7;
            let copy_src = 1 << 2;
            wgpu::BufferUsages::from_bits(storage | copy_src).unwrap()
        }
        BufferType::StorageRW => {
            let storage = 1 << 7;
            let copy_src = 1 << 2;
            let copy_dst = 1 << 3;
            wgpu::BufferUsages::from_bits(storage | copy_src | copy_dst).unwrap()
        },
    }
}

const fn get_binding_type(ty: BufferType) -> wgpu::BufferBindingType {
    match ty {
        BufferType::Vertex => panic!("Error: BufferType::Vertex cannot be used as binding resource (they are used in pipeline instead)"),
        BufferType::Index => panic!("Error: BufferType::Index cannot be used as binding resource (they are used in pipeline instead)"),
        BufferType::Uniform => wgpu::BufferBindingType::Uniform,
        BufferType::StorageRO => wgpu::BufferBindingType::Storage { read_only: true },
        BufferType::StorageRW => wgpu::BufferBindingType::Storage { read_only: false },
    }
}

impl GPUBuffer {
    pub const fn get_layout<T>(binding: u32, ty: BufferType) -> wgpu::BindGroupLayoutEntry {
        wgpu::BindGroupLayoutEntry {
            count: None,
            binding,
            visibility: get_visibility(ty),
            ty: wgpu::BindingType::Buffer {
                ty: get_binding_type(ty),
                has_dynamic_offset: false,
                min_binding_size: wgpu::BufferSize::new(size_of::<T>() as u64),
            },
        }
    }

    pub fn create_init<T: NoUninit + Pod>(ctx: &GPUCtx, usage: wgpu::BufferUsages, init: &T) -> Self {
        let buffer = ctx.device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: None,
            contents: bytemuck::bytes_of(init),
            usage,
        });

        Self {
            buffer,
            usage,
        }
    }

    pub fn update<T: NoUninit + Pod>(&self, ctx: &GPUCtx, data: &T) {
        ctx.queue.write_buffer(&self.buffer, 0, bytemuck::bytes_of(data));
    }
}