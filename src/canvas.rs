use std::mem;

use skia_safe::{surfaces, AlphaType, Color, Color4f, ColorType, Data, EncodedImageFormat, ImageInfo, Paint, PaintStyle, Path, Rect, Surface};

pub struct Canvas {
    width: u32,
    height: u32,
    surface: Surface,
    path: Path,
    paint: Paint,
}

impl Canvas {
    pub fn new(width: u32, height: u32) -> Canvas {
        let mut surface = surfaces::raster_n32_premul((width as i32, height as i32)).expect("surface");
        let path = Path::new();
        let mut paint = Paint::default();
        paint.set_color(Color::BLACK);
        paint.set_anti_alias(true);
        paint.set_stroke_width(1.0);
        surface.canvas().clear(Color::WHITE);
        Canvas {
            surface,
            path,
            paint,
            width,
            height,
        }
    }

    #[inline]
    pub fn save(&mut self) {
        self.canvas().save();
    }

    #[inline]
    pub fn translate(&mut self, dx: f32, dy: f32) {
        self.canvas().translate((dx, dy));
    }

    #[inline]
    pub fn scale(&mut self, sx: f32, sy: f32) {
        self.canvas().scale((sx, sy));
    }

    #[inline]
    pub fn move_to(&mut self, x: f32, y: f32) {
        self.begin_path();
        self.path.move_to((x, y));
    }

    #[inline]
    pub fn line_to(&mut self, x: f32, y: f32) {
        self.path.line_to((x, y));
    }

    #[inline]
    pub fn quad_to(&mut self, cpx: f32, cpy: f32, x: f32, y: f32) {
        self.path.quad_to((cpx, cpy), (x, y));
    }

    #[allow(dead_code)]
    #[inline]
    pub fn bezier_curve_to(&mut self, cp1x: f32, cp1y: f32, cp2x: f32, cp2y: f32, x: f32, y: f32) {
        self.path.cubic_to((cp1x, cp1y), (cp2x, cp2y), (x, y));
    }

    #[allow(dead_code)]
    #[inline]
    pub fn close_path(&mut self) {
        self.path.close();
    }

    #[inline]
    pub fn begin_path(&mut self) {
        let new_path = Path::new();
        self.surface.canvas().draw_path(&self.path, &self.paint);
        let _ = mem::replace(&mut self.path, new_path);
    }

    #[inline]
    pub fn stroke(&mut self) {
        self.paint.set_style(PaintStyle::Stroke);
        self.surface.canvas().draw_path(&self.path, &self.paint);
    }

    #[inline]
    pub fn set_fill_color(&mut self, color: Color) {
        self.paint.set_color(color);
    }

    #[inline]
    pub fn set_color(&mut self, color: Color4f) {
        self.paint.set_color(color.to_color());
    }

    #[inline]
    pub fn fill_rect(&mut self, left: f32, top: f32, right: f32, bottom: f32) {
        self.paint.set_style(PaintStyle::Fill);
        self.surface.canvas().draw_rect(
            Rect {
                left,
                bottom,
                right,
                top,
            },
            &self.paint,
        );
    }

    #[inline]
    pub fn stroke_rect(&mut self, left: f32, top: f32, right: f32, bottom: f32) {
        self.paint.set_style(PaintStyle::Stroke);
        self.surface.canvas().draw_rect(
            Rect {
                left,
                bottom,
                right,
                top,
            },
            &self.paint,
        );
    }

    #[inline]
    pub fn fill(&mut self) {
        self.paint.set_style(PaintStyle::Fill);
        self.surface.canvas().draw_path(&self.path, &self.paint);
    }

    #[inline]
    pub fn set_line_width(&mut self, width: f32) {
        self.paint.set_stroke_width(width);
    }

    #[allow(dead_code)]
    pub fn as_png_data(&mut self) -> Data {
        let image = self.surface.image_snapshot();
        let mut context = self.surface.direct_context();
        image
            .encode(context.as_mut(), EncodedImageFormat::PNG, None)
            .unwrap()
    }

    pub fn as_bytes(&mut self) -> Result<Vec<u8>, &'static str> {
        let (width, height) = (self.surface.width(), self.surface.height());
        let image_info = ImageInfo::new(
            (width, height),
            ColorType::RGBA8888,
            AlphaType::Premul,
            None,
        );
        let image_row_size = (width * 4) as usize;
        let image_size = (width * height * 4) as usize;
        let mut pixel_data = vec![0u8; image_size];
        let success = self.surface.read_pixels(
            &image_info,
            pixel_data.as_mut_slice(),
            image_row_size,
            (0, 0),
        );

        if !success {
            eprintln!("Failed to read pixels from the surface.");
            return Err("Failed to read pixels from the surface.");
        }

        Ok(pixel_data)
    }

    #[inline]
    fn canvas(&mut self) -> &skia_safe::Canvas {
        self.surface.canvas()
    }
}
