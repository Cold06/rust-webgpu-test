use crate::camera::Camera;
use crate::multimath::{Mat4, Vec4};
use bytemuck::{NoUninit, Pod, Zeroable};
use std::mem::offset_of;
use wgpu::util::DeviceExt;

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable, Default)]
pub struct Vertex {
    pub pos: [f32; 4],
    pub tex_coord: [f32; 2],
}

impl Vertex {
    pub fn new(pos: [f32; 3], tc: [i8; 2]) -> Vertex {
        Vertex {
            pos: [pos[0], pos[1], pos[2], 1.0],
            tex_coord: [tc[0] as f32, tc[1] as f32],
        }
    }
}

pub struct ModelBundle {
    pub vertex_data: Vec<Vertex>,
    pub index_data: Vec<u32>,
}

pub struct VertexFormat {
    pub vertex_buffer: wgpu::Buffer,
    pub index_buffer: wgpu::Buffer,
    pub index_count: u32,
}

impl VertexFormat {
    pub const LAYOUT: [wgpu::VertexBufferLayout<'static>; 1] = [wgpu::VertexBufferLayout {
        array_stride: size_of::<Vertex>() as wgpu::BufferAddress,
        step_mode: wgpu::VertexStepMode::Vertex,
        attributes: &[
            wgpu::VertexAttribute {
                format: wgpu::VertexFormat::Float32x4,
                offset: offset_of![Vertex, pos] as u64,
                shader_location: 0,
            },
            wgpu::VertexAttribute {
                format: wgpu::VertexFormat::Float32x2,
                offset: offset_of!(Vertex, tex_coord) as u64,
                shader_location: 1,
            },
        ],
    }];

    pub fn create(device: &wgpu::Device, model_bundle: &ModelBundle) -> Self {
        let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: None,
            contents: bytemuck::cast_slice(&model_bundle.vertex_data),
            usage: wgpu::BufferUsages::VERTEX,
        });

        let index_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: None,
            contents: bytemuck::cast_slice(&model_bundle.index_data),
            usage: wgpu::BufferUsages::INDEX,
        });

        Self {
            index_buffer,
            vertex_buffer,
            index_count: model_bundle.index_data.len() as u32,
        }
    }
}

pub struct BindGroup0 {
    pub bind_group: wgpu::BindGroup,
    uniform_buffer: wgpu::Buffer,

    y_texture_view: wgpu::TextureView,
    y_texture: wgpu::Texture,

    u_texture_view: wgpu::TextureView,
    u_texture: wgpu::Texture,

    v_texture_view: wgpu::TextureView,
    v_texture: wgpu::Texture,

    sampler: wgpu::Sampler,

    y_width: u32,
    y_height: u32,

    uv_width: u32,
    uv_height: u32,
}

impl BindGroup0 {
    pub fn update_texture<T: NoUninit>(&self, queue: &wgpu::Queue, y_data: &[T], u_data: &[T], v_data: &[T]) {
        let y_texture_extent = wgpu::Extent3d {
            width: self.y_width,
            height: self.y_height,
            depth_or_array_layers: 1,
        };

        queue.write_texture(
            self.y_texture.as_image_copy(),
            bytemuck::cast_slice(y_data),
            wgpu::ImageDataLayout {
                offset: 0,
                bytes_per_row: Some(self.y_width),
                rows_per_image: Some(self.y_height),
            },
            y_texture_extent,
        );

        let uv_textures_extent = wgpu::Extent3d {
            width: self.uv_width,
            height: self.uv_height,
            depth_or_array_layers: 1,
        };

        queue.write_texture(
            self.u_texture.as_image_copy(),
            bytemuck::cast_slice(u_data),
            wgpu::ImageDataLayout {
                offset: 0,
                bytes_per_row: Some(self.uv_width),
                rows_per_image: Some(self.uv_height),
            },
            uv_textures_extent,
        );

        queue.write_texture(
            self.v_texture.as_image_copy(),
            bytemuck::cast_slice(v_data),
            wgpu::ImageDataLayout {
                offset: 0,
                bytes_per_row: Some(self.uv_width),
                rows_per_image: Some(self.uv_height),
            },
            uv_textures_extent,
        );
    }
    pub(crate) fn resize_textures(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        width: u32,
        height: u32,
    ) {
        let y_width = width;
        let y_height = height;
        self.y_width = width;
        self.y_height = height;

        let uv_width = width / 2;
        let uv_height = height / 2;
        self.uv_width = uv_width;
        self.uv_height = uv_height;

        let y_texture_extent = wgpu::Extent3d {
            width: y_width,
            height: y_height,
            depth_or_array_layers: 1,
        };

        let uv_textures_extent = wgpu::Extent3d {
            width: uv_width,
            height: uv_height,
            depth_or_array_layers: 1,
        };

        self.y_texture = device.create_texture(&wgpu::TextureDescriptor {
            label: None,
            size: y_texture_extent,
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::R8Unorm,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT
                | wgpu::TextureUsages::COPY_DST
                | wgpu::TextureUsages::COPY_SRC
                | wgpu::TextureUsages::TEXTURE_BINDING,
            view_formats: &[wgpu::TextureFormat::R8Unorm],
        });

        self.y_texture_view = self
            .y_texture
            .create_view(&wgpu::TextureViewDescriptor::default());

        queue.write_texture(
            self.y_texture.as_image_copy(),
            bytemuck::cast_slice(&vec![0u32; (y_width * y_height) as usize]),
            wgpu::ImageDataLayout {
                offset: 0,
                bytes_per_row: Some(y_width),
                rows_per_image: None,
            },
            y_texture_extent,
        );

        self.u_texture = device.create_texture(&wgpu::TextureDescriptor {
            label: None,
            size: uv_textures_extent,
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::R8Unorm,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT
                | wgpu::TextureUsages::COPY_DST
                | wgpu::TextureUsages::COPY_SRC
                | wgpu::TextureUsages::TEXTURE_BINDING,
            view_formats: &[wgpu::TextureFormat::R8Unorm],
        });

        self.u_texture_view = self
            .u_texture
            .create_view(&wgpu::TextureViewDescriptor::default());

        queue.write_texture(
            self.u_texture.as_image_copy(),
            bytemuck::cast_slice(&vec![0u32; (uv_width * uv_height) as usize]),
            wgpu::ImageDataLayout {
                offset: 0,
                bytes_per_row: Some(uv_width),
                rows_per_image: None,
            },
            uv_textures_extent,
        );


        self.v_texture = device.create_texture(&wgpu::TextureDescriptor {
            label: None,
            size: uv_textures_extent,
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::R8Unorm,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT
                | wgpu::TextureUsages::COPY_DST
                | wgpu::TextureUsages::COPY_SRC
                | wgpu::TextureUsages::TEXTURE_BINDING,
            view_formats: &[wgpu::TextureFormat::R8Unorm],
        });

        self.v_texture_view = self
            .v_texture
            .create_view(&wgpu::TextureViewDescriptor::default());

        queue.write_texture(
            self.v_texture.as_image_copy(),
            bytemuck::cast_slice(&vec![0u32; (uv_width * uv_height) as usize]),
            wgpu::ImageDataLayout {
                offset: 0,
                bytes_per_row: Some(uv_width),
                rows_per_image: None,
            },
            uv_textures_extent
        );


        self.bind_group = Self::generate_bind_group(
            &device,
            &self.uniform_buffer,
            &self.sampler,
            &self.y_texture_view,
            &self.u_texture_view,
            &self.v_texture_view,
        );
    }
}

impl BindGroup0 {
    const LAYOUT: wgpu::BindGroupLayoutDescriptor<'static> = wgpu::BindGroupLayoutDescriptor {
        label: None,
        entries: &[
            wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::VERTEX,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: wgpu::BufferSize::new(64),
                },
                count: None,
            },
            wgpu::BindGroupLayoutEntry {
                binding: 1,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                count: None,
            },
            wgpu::BindGroupLayoutEntry {
                binding: 2,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Texture {
                    multisampled: false,
                    sample_type: wgpu::TextureSampleType::Float { filterable: true },
                    view_dimension: wgpu::TextureViewDimension::D2,
                },
                count: None,
            },
            wgpu::BindGroupLayoutEntry {
                binding: 3,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Texture {
                    multisampled: false,
                    sample_type: wgpu::TextureSampleType::Float { filterable: true },
                    view_dimension: wgpu::TextureViewDimension::D2,
                },
                count: None,
            },
            wgpu::BindGroupLayoutEntry {
                binding: 4,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Texture {
                    multisampled: false,
                    sample_type: wgpu::TextureSampleType::Float { filterable: true },
                    view_dimension: wgpu::TextureViewDimension::D2,
                },
                count: None,
            },
        ],
    };

    pub fn get_layout(device: &wgpu::Device) -> wgpu::BindGroupLayout {
        device.create_bind_group_layout(&Self::LAYOUT)
    }

    pub fn update_globals(&self, queue: &wgpu::Queue, camera: &Camera) {
        queue.write_buffer(&self.uniform_buffer, 0, bytemuck::bytes_of(&camera.matrix));
    }

    pub fn create(device: &wgpu::Device, queue: &wgpu::Queue, width: u32, height: u32) -> Self {
        let y_width = width;
        let y_height = height;

        let uv_width = width / 2;
        let uv_height = height / 2;

        let uv_textures_extent = wgpu::Extent3d {
            width: uv_width,
            height: uv_height,
            depth_or_array_layers: 1,
        };

        let (u_texture, u_texture_view) = {
            let u_texture = device.create_texture(&wgpu::TextureDescriptor {
                label: None,
                size: uv_textures_extent,
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format: wgpu::TextureFormat::R8Unorm,
                usage: wgpu::TextureUsages::RENDER_ATTACHMENT
                    | wgpu::TextureUsages::COPY_DST
                    | wgpu::TextureUsages::COPY_SRC
                    | wgpu::TextureUsages::TEXTURE_BINDING,
                view_formats: &[wgpu::TextureFormat::R8Unorm],
            });

            let u_texture_view = u_texture.create_view(&wgpu::TextureViewDescriptor::default());

            queue.write_texture(
                u_texture.as_image_copy(),
                bytemuck::cast_slice(&vec![0u32; (uv_width * uv_height) as usize]),
                wgpu::ImageDataLayout {
                    offset: 0,
                    bytes_per_row: Some(uv_width),
                    rows_per_image: None,
                },
                uv_textures_extent,
            );

            (u_texture, u_texture_view)
        };

        let (v_texture, v_texture_view) = {
            let v_texture = device.create_texture(&wgpu::TextureDescriptor {
                label: None,
                size: uv_textures_extent,
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format: wgpu::TextureFormat::R8Unorm,
                usage: wgpu::TextureUsages::RENDER_ATTACHMENT
                    | wgpu::TextureUsages::COPY_DST
                    | wgpu::TextureUsages::COPY_SRC
                    | wgpu::TextureUsages::TEXTURE_BINDING,
                view_formats: &[wgpu::TextureFormat::R8Unorm],
            });

            let v_texture_view = v_texture.create_view(&wgpu::TextureViewDescriptor::default());

            queue.write_texture(
                v_texture.as_image_copy(),
                bytemuck::cast_slice(&vec![0u32; (uv_width * uv_height) as usize]),
                wgpu::ImageDataLayout {
                    offset: 0,
                    bytes_per_row: Some(uv_width),
                    rows_per_image: None,
                },
                uv_textures_extent,
            );

            (v_texture, v_texture_view)
        };

        let y_texture_extent = wgpu::Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        };

        let y_texture = device.create_texture(&wgpu::TextureDescriptor {
            label: None,
            size: y_texture_extent,
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::R8Unorm,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT
                | wgpu::TextureUsages::COPY_DST
                | wgpu::TextureUsages::COPY_SRC
                | wgpu::TextureUsages::TEXTURE_BINDING,
            view_formats: &[wgpu::TextureFormat::R8Unorm],
        });

        let y_texture_view = y_texture.create_view(&wgpu::TextureViewDescriptor::default());

        queue.write_texture(
            y_texture.as_image_copy(),
            bytemuck::cast_slice(&vec![0u32; (y_width * y_height) as usize]),
            wgpu::ImageDataLayout {
                offset: 0,
                bytes_per_row: Some(width),
                rows_per_image: None,
            },
            y_texture_extent,
        );

        let uniform_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: None,
            contents: bytemuck::bytes_of(&Mat4::new()),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        // TODO: both needs to be at the same place
        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: None,
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            min_filter: wgpu::FilterMode::Linear,
            mag_filter: wgpu::FilterMode::Linear,
            mipmap_filter: wgpu::FilterMode::Linear,
            ..Default::default()
        });

        let bind_group = Self::generate_bind_group(
            device,
            &uniform_buffer,
            &sampler,
            &y_texture_view,
            &u_texture_view,
            &v_texture_view,
        );

        Self {
            bind_group,
            uniform_buffer,
            y_texture_view,
            y_texture,
            u_texture_view,
            u_texture,
            v_texture_view,
            v_texture,
            sampler,
            y_width,
            y_height,
            uv_width,
            uv_height,
        }
    }

    fn generate_bind_group(
        device: &wgpu::Device,
        uniform_buffer: &wgpu::Buffer,
        sampler: &wgpu::Sampler,
        y_texture_view: &wgpu::TextureView,
        u_texture_view: &wgpu::TextureView,
        v_texture_view: &wgpu::TextureView,
    ) -> wgpu::BindGroup {
        device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &Self::get_layout(device),
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: uniform_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&sampler),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: wgpu::BindingResource::TextureView(&y_texture_view),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: wgpu::BindingResource::TextureView(&u_texture_view),
                },
                wgpu::BindGroupEntry {
                    binding: 4,
                    resource: wgpu::BindingResource::TextureView(&v_texture_view),
                },
            ],
            label: None,
        })
    }
}

pub struct BindGroup1 {
    pub bind_group: wgpu::BindGroup,
}

impl BindGroup1 {
    const LAYOUT: wgpu::BindGroupLayoutDescriptor<'static> = wgpu::BindGroupLayoutDescriptor {
        label: None,
        entries: &[wgpu::BindGroupLayoutEntry {
            binding: 0,
            visibility: wgpu::ShaderStages::VERTEX,
            ty: wgpu::BindingType::Buffer {
                ty: wgpu::BufferBindingType::Uniform,
                has_dynamic_offset: false,
                min_binding_size: wgpu::BufferSize::new(16),
            },
            count: None,
        }],
    };

    pub fn get_layout(device: &wgpu::Device) -> wgpu::BindGroupLayout {
        device.create_bind_group_layout(&Self::LAYOUT)
    }

    pub fn create(device: &wgpu::Device, position: Vec4) -> Self {
        let position_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: None,
            contents: bytemuck::bytes_of(&position),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &BindGroup1::get_layout(device),
            label: None,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: position_buffer.as_entire_binding(),
            }],
        });

        Self { bind_group }
    }
}

pub struct Pipeline {
    pub pipeline: wgpu::RenderPipeline,
}

impl Pipeline {
    pub fn create(device: &wgpu::Device, format: wgpu::TextureFormat) -> Self {
        let shader = device.create_shader_module(wgpu::include_wgsl!("./video.wgsl"));

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: None,
            bind_group_layouts: &[
                &BindGroup0::get_layout(device),
                &BindGroup1::get_layout(device),
            ],
            push_constant_ranges: &[],
        });

        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: None,
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: "vs_main",
                buffers: &VertexFormat::LAYOUT,
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: "fs_main",
                targets: &[Some(format.into())],
            }),
            primitive: wgpu::PrimitiveState {
                cull_mode: Some(wgpu::Face::Back),
                topology: wgpu::PrimitiveTopology::TriangleList,
                ..Default::default()
            },
            depth_stencil: Some(wgpu::DepthStencilState {
                format: wgpu::TextureFormat::Depth32Float,
                stencil: wgpu::StencilState::default(),
                bias: wgpu::DepthBiasState::default(),
                depth_write_enabled: true,
                depth_compare: wgpu::CompareFunction::Less,
            }),
            multisample: wgpu::MultisampleState::default(),
            multiview: None,
        });

        Self { pipeline }
    }
}
