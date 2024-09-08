use skia_safe::Color;
use crate::canvas::Canvas;

pub fn create_texels(size: usize) -> Vec<u8> {
    let mut canvas = Canvas::new(size as i32, size as i32);

    canvas.set_fill_color(Color::RED);
    canvas.fill_rect(0.0, 0.0, size as f32, size as f32);

    canvas.set_fill_color(Color::BLACK);
    canvas.fill_rect(0.0, 0.0, 128.0, 128.0);
    canvas.fill_rect(128.0, 128.0, 256.0, 256.0);

    canvas.set_fill_color(Color::WHITE);
    canvas.set_line_width(24.0);
    canvas.stroke_rect(0.0, 0.0, size as f32, size as f32);

    canvas.as_bytes().unwrap()
}
