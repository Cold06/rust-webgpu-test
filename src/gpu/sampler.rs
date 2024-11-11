use egui_wgpu::wgpu;
use crate::gpu::GPUCtx;

pub struct GPUSampler {
    sampler: wgpu::Sampler,
}

impl GPUSampler {
    pub const fn get_layout(binding: u32) -> wgpu::BindGroupLayoutEntry {
        wgpu::BindGroupLayoutEntry {
            binding,
            visibility: wgpu::ShaderStages::FRAGMENT,
            ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
            count: None,
        }
    }

    pub fn create(ctx: &GPUCtx, filter: wgpu::FilterMode, address_mode: wgpu::AddressMode) -> Self {
        let sampler = ctx.device.create_sampler(&wgpu::SamplerDescriptor {
            label: None,
            address_mode_u: address_mode,
            address_mode_v: address_mode,
            address_mode_w: address_mode,
            min_filter: filter,
            mag_filter: filter,
            mipmap_filter: filter,
            ..Default::default()
        });

        Self {
            sampler
        }
    }

    pub fn get_binding(&self, binding: u32) -> wgpu::BindGroupEntry {
        wgpu::BindGroupEntry {
            binding,
            resource: wgpu::BindingResource::Sampler(&self.sampler),
        }
    }
}