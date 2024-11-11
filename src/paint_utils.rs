use skia_safe::{Color};
use crate::canvas::Canvas;

fn map_range(value: f64) -> f64 {
    (value + 1.0) / 2.0
}

pub fn create_texels(size: usize) -> Vec<u8> {
    let mut canvas = Canvas::new(size as u32, size as u32);

    canvas.set_fill_color(Color::RED);
    canvas.fill_rect(0.0, 0.0, size as f32, size as f32);

    canvas.set_fill_color(Color::BLACK);
    canvas.fill_rect(0.0, 0.0, 128.0, 128.0);
    canvas.fill_rect(128.0, 128.0, 256.0, 256.0);

    canvas.set_fill_color(Color::WHITE);
    canvas.set_line_width(24.0);
    canvas.stroke_rect(0.0, 0.0, size as f32, size as f32);

    // let perlin = Perlin::default();
    // let ridged = RidgedMulti::<Perlin>::default();
    // let fbm = Fbm::<Perlin>::default();
    // let blend = Blend::new(perlin, ridged, fbm);
    //
    // for y in 0..(size as i32) {
    //     for x in 0..(size as i32) {
    //
    //         let noise_scale = 0.01;
    //
    //         let value = blend.get([(x as f64) * noise_scale, (y as f64) * noise_scale, 0.0 * noise_scale]);
    //         let value = map_range(value);
    //         canvas.set_color(Color4f::new(value as f32, value as f32, value as f32, 1.0));
    //         canvas.fill_rect(x as f32, y as f32, (x + 1) as f32, (y + 1) as f32);
    //     }
    // }


    canvas.as_bytes().unwrap()
}
