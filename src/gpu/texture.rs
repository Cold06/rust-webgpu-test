use bytemuck::NoUninit;
use crate::gpu::GPUCtx;

pub struct GPUTexture {
    texture: wgpu::Texture,
    view: wgpu::TextureView,
    size: wgpu::Extent3d,
    format: wgpu::TextureFormat,
    usage: wgpu::TextureUsages,
}

fn size_of<T>() -> u32 {
    std::mem::size_of::<T>() as u32
}

fn get_bytes_per_row(format: wgpu::TextureFormat, width: u32) -> Option<u32> {
    let bytes = match format {
        wgpu::TextureFormat::Depth32Float => size_of::<f32>() * width,
        wgpu::TextureFormat::R8Unorm => size_of::<u8>() * width,
        wgpu::TextureFormat::Rgba8Unorm => 4 * size_of::<u8>() * width,
        _ => {
            panic!("Unknown format {:?}", format);
        }
    };

    Some(bytes)
}

impl GPUTexture {
    pub const fn get_layout(binding: u32) -> wgpu::BindGroupLayoutEntry {
        wgpu::BindGroupLayoutEntry {
            binding,
            visibility: wgpu::ShaderStages::FRAGMENT,
            ty: wgpu::BindingType::Texture {
                multisampled: false,
                sample_type: wgpu::TextureSampleType::Float {
                    filterable: true
                },
                view_dimension: wgpu::TextureViewDimension::D2,
            },
            count: None,
        }
    }

    pub fn create<T: NoUninit>(ctx: &GPUCtx, width: u32, height: u32, format: wgpu::TextureFormat, filler: T, usage: wgpu::TextureUsages) -> Self {
        let size = wgpu::Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        };

        let texture = ctx.device.create_texture(&wgpu::TextureDescriptor {
            label: None,
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            size,
            format,
            usage,
            view_formats: &[format],
        });

        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());

        ctx.queue.write_texture(
            texture.as_image_copy(),
            bytemuck::cast_slice(&vec![filler; (width * height) as usize]),
            wgpu::ImageDataLayout {
                offset: 0,
                bytes_per_row: get_bytes_per_row(format, size.width),
                rows_per_image: None,
            },
            size,
        );

        Self {
            size,
            view,
            texture,
            format,
            usage,
        }
    }

    pub fn resize<T: NoUninit>(&mut self, ctx: &GPUCtx, width: u32, height: u32, filler: T) {
        if self.size.width == width && self.size.height == height {
            return;
        }

        *self = Self::create(&ctx, width, height, self.format, filler, self.usage);
    }
    pub fn update<T: NoUninit>(&self, ctx: &GPUCtx, data: &[T]) {
        ctx.queue.write_texture(
            self.texture.as_image_copy(),
            bytemuck::cast_slice(data),
            wgpu::ImageDataLayout {
                offset: 0,
                bytes_per_row: get_bytes_per_row(self.format, self.size.width),
                rows_per_image: None,
            },
            self.size,
        );
    }

    pub fn get_binding(&self, binding: u32) -> wgpu::BindGroupEntry {
        wgpu::BindGroupEntry {
            binding,
            resource: wgpu::BindingResource::TextureView(&self.view),
        }
    }
}
