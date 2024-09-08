use crate::canvas::Canvas;

pub fn create_texels(size: usize) -> Vec<u8> {
    let mut canvas = Canvas::new(size as i32, size as i32);
    canvas.scale(1.2, 1.2);
    canvas.move_to(36.0, 48.0);
    canvas.quad_to(660.0, 880.0, 100.0, 360.0);
    canvas.translate(10.0, 10.0);
    canvas.set_line_width(20.0);
    canvas.stroke();
    canvas.save();
    canvas.move_to(30.0, 90.0);
    canvas.line_to(110.0, 20.0);
    canvas.line_to(240.0, 130.0);
    canvas.line_to(60.0, 130.0);
    canvas.line_to(190.0, 20.0);
    canvas.line_to(270.0, 90.0);
    canvas.fill();
    canvas.as_bytes().unwrap()
}
