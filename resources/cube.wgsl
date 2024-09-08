struct VertexOutput {
    @location(0) tex_coord: vec2<f32>,
    @builtin(position) position: vec4<f32>,
};

struct Locals {
    transform: mat4x4<f32>,
};

@group(0) @binding(0)
var<uniform> locals: Locals;

@vertex
fn vs_main(
    @location(0) position: vec4<f32>,
    @location(1) tex_coord: vec2<f32>,
) -> VertexOutput {
    var out: VertexOutput;
    out.tex_coord = tex_coord;
    out.position = locals.transform * position;
    return out;
}

@group(0) @binding(1)
var r_color: texture_2d<f32>;

@group(0)
@binding(2)
var r_sampler: sampler;

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    return textureSample(r_color, r_sampler, in.tex_coord);
}
