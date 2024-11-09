use crate::canvas::Canvas;
use imgui::TextureId;
use imgui_wgpu::{Renderer, Texture};
use crate::gpu::GPUCtx;

pub struct GUICanvas {
    pub texture_id: TextureId,
    size: [u32; 2],
    dirty: bool,
    canvas: Canvas,
}

impl GUICanvas {
    pub fn new(renderer: &mut Renderer, ctx: &GPUCtx, size: [u32; 2]) -> Self {
        let texture_config = imgui_wgpu::TextureConfig {
            size: wgpu::Extent3d {
                width: size[0],
                height: size[1],
                ..Default::default()
            },
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            ..Default::default()
        };
        let texture = Texture::new(&ctx.device, &renderer, texture_config);
        let texture_id = renderer.textures.insert(texture);

        Self {
            texture_id,
            size,
            dirty: false,
            canvas: Canvas::new(size[0] as i32, size[1] as i32),
        }
    }

    pub fn with(&mut self, implementation: impl FnOnce(&mut Canvas)) {
        self.dirty = true;
        implementation(&mut self.canvas);
    }

    pub fn update(&mut self, renderer: &mut Renderer, ctx: &GPUCtx) {
        if self.dirty {
            self.dirty = false;
            let buffer = self.canvas.as_bytes();

            if let Ok(buffer) = buffer {
                if let Some(texture) = renderer.textures.get_mut(self.texture_id) {
                    texture.write(&ctx.queue, buffer.as_slice(), self.size[0], self.size[1]);
                } else {
                    eprintln!("Failed to find texture with ID {:?}", self.texture_id);
                }
            } else {
                eprintln!("Failed to read RGBA data");
            }
        }
    }
}
