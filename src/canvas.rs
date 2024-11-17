use std::mem;

use skia_safe::paint::{Cap, Join, Style};
use skia_safe::{surfaces, AlphaType, Color, Color4f, ColorType, Data, EncodedImageFormat, ImageInfo, Paint, PaintStyle, Path, Rect, Surface, M44};

pub struct Canvas {
    width: u32,
    height: u32,
    surface: Surface,
    path: Path,
    paint: Paint,

    fill_style: Paint,
    stroke_style: Paint,
    line_width: f64,
    line_cap: Cap,
    line_join: Join,
    miter_limit: f64,
}

impl Canvas {
    pub fn new(width: u32, height: u32, high_dpi_factor: f32) -> Canvas {
        let mut surface =
            surfaces::raster_n32_premul((width as i32, height as i32)).expect("surface");
        let path = Path::new();
        let mut paint = Paint::default();
        paint.set_color(Color::BLACK);
        paint.set_anti_alias(true);
        paint.set_stroke_width(1.0);
        surface.canvas().clear(Color::WHITE);
        surface.canvas().scale((high_dpi_factor, high_dpi_factor));

        let mut fill_style = Paint::default();
        fill_style.set_style(Style::Fill);
        fill_style.set_color(Color::BLACK);
        fill_style.set_anti_alias(true);

        let mut stroke_style = Paint::default();
        stroke_style.set_style(Style::Stroke);
        stroke_style.set_color(Color::BLACK);
        stroke_style.set_anti_alias(true);

        Canvas {
            surface,
            path,
            paint,
            width,
            height,
            fill_style: Paint::default(),
            stroke_style: Paint::default(),
            line_width: 1.0,
            line_cap: Cap::Butt,
            line_join: Join::Miter,
            miter_limit: 0.0,
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

    pub fn clear(&mut self) {
        self.surface.canvas().clear(Color::WHITE);
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

    // Optional setters
    // globalAlpha

    // Required setters
    // fillStyle
    // lineWidth
    // strokeStyle
    // lineCap
    // lineJoin
    // miterLimit

    // Required ops
    // save
    // scale
    // beginPath
    // rect
    // fill
    // moveTo
    // lineTo
    // bezierCurveTo
    // closePath
    // transform
    // stroke
    // restore

    pub fn js_set_fill_style(&mut self, rgb_color: String) {
        let parsed: skribble_color::Color = rgb_color.parse().unwrap();
        let rgb_v = parsed.into_rgb();
        let rgb = rgb_v.get_rgb().unwrap();

        let color = Color4f::new(
            rgb.red,
            rgb.green,
            rgb.blue,
            rgb.alpha,
        );

        self.fill_style.set_color(color.to_color());
    }

    pub fn js_set_line_width(&mut self, line_width: f64) {
        self.line_width = line_width;
        self.stroke_style.set_stroke_width(line_width as f32);
    }

    pub fn js_set_stroke_style(&mut self, stroke_style: String) {
        let parsed: skribble_color::Color = stroke_style.parse().unwrap();
        let rgb_v = parsed.into_rgb();
        let rgb = rgb_v.get_rgb().unwrap();

        let color = Color4f::new(
            rgb.red,
            rgb.green,
            rgb.blue,
            rgb.alpha,
        );

        self.stroke_style.set_color(color.to_color());
    }

    pub fn js_set_line_cap(&mut self, line_cap: String) {
        match line_cap.as_str() {
            "butt" => {
                self.line_cap = Cap::Butt;
                self.stroke_style.set_stroke_cap(Cap::Butt);
            }
            "round" => {
                self.line_cap = Cap::Round;
                self.stroke_style.set_stroke_cap(Cap::Round);
            }
            "square" => {
                self.line_cap = Cap::Square;
                self.stroke_style.set_stroke_cap(Cap::Square);
            }
            _ => {
                println!("Unknown line cap {}", line_cap);
            }
        }
    }

    pub fn js_set_line_join(&mut self, line_join: String) {
        match line_join.as_str() {
            "miter" => {
                self.line_join = Join::Miter;
                self.stroke_style.set_stroke_join(Join::Miter);
            }
            "round" => {
                self.line_join = Join::Round;
                self.stroke_style.set_stroke_join(Join::Round);
            }
            "bevel" => {
                self.line_join = Join::Bevel;
                self.stroke_style.set_stroke_join(Join::Bevel);
            }
            _ => {
                println!("Unknown line join {}", line_join);
            }
        }
    }

    pub fn js_set_miter_limit(&mut self, miter_limit: f64) {
        self.miter_limit = miter_limit;
        self.stroke_style.set_stroke_miter(miter_limit as f32);
    }

    pub fn js_call_save(&mut self) {
        self.surface.canvas().save();
    }

    pub fn js_call_scale(&mut self, x: f64, y: f64) {
        self.surface.canvas().scale((x as f32, y as f32));
    }

    pub fn js_call_begin_path(&mut self) {
        drop(mem::replace(&mut self.path, Path::new()));
    }

    pub fn js_call_rect(&mut self, x: f64, y: f64, width: f64, height: f64) {
        self.path.add_rect(Rect::new(x as f32, y as f32, (x as f32) + (width as f32), (y as f32) + (height as f32)), None);
        self.path.close();
    }

    pub fn js_call_fill(&mut self) {
        self.fill_style.set_style(PaintStyle::Fill);
        self.surface.canvas().draw_path(&self.path, &self.fill_style);
    }

    pub fn js_call_move_to(&mut self, x: f64, y: f64) {
        self.path.move_to((x as f32, y as f32));
    }

    pub fn js_call_arc(&mut self, x: f64, y: f64, radius: f64, start_angle: f64, end_angle: f64, counterclockwise: bool) {
        let r = radius as f32;

        self.path.add_arc(
            Rect {
                top: (y as f32) - r,
                left: (x as f32) - r,
                bottom: (y as f32) + r,
                right: (x as f32) + r,
            },
            start_angle as f32,
            end_angle as f32
        );
    }

    pub fn js_call_line_to(&mut self, x: f64, y: f64) {
        self.path.line_to((x as f32, y as f32));
    }

    pub fn js_call_bezier_curve_to(&mut self, p1x: f64, p1y: f64, p2x: f64, p2y: f64, px: f64, py: f64) {
        self.bezier_curve_to(
            p1x as f32, p1y as f32, p2x as f32, p2y as f32, px as f32, py as f32,
        );
    }

    pub fn js_call_close_path(&mut self) {
        self.close_path();
    }

    pub fn js_call_transform(&mut self, a: f64, b: f64, c: f64, d: f64, e: f64, f: f64) {
        let x = M44::new(
            a as f32, c as f32, e as f32, 0.0,
            b as f32, d as f32, f as f32, 0.0,
            0.0,     0.0,      1.0,       0.0,
            0.0,     0.0,      0.0,       1.0,
        );
        self.canvas().set_matrix(&x);
    }

    pub fn js_call_stroke(&mut self) {
        self.stroke_style.set_style(PaintStyle::Stroke);
        self.surface.canvas().draw_path(&self.path, &self.stroke_style);
    }

    pub fn js_call_restore(&mut self) {
        self.canvas().restore();
    }
}

impl Canvas {
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
