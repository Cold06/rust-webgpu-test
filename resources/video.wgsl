// Vertex Layout
struct VertexOutput {
    @location(0) tex_coord: vec2<f32>,
    @builtin(position) position: vec4<f32>,
};

// Uniforms Like
struct PerInstance {
    position: vec4<f32>,
};

// More like... globals i am right?
struct Locals {
    transform: mat4x4<f32>,
};

@group(0) @binding(0)
var<uniform> locals: Locals;

@group(1) @binding(0)
var<uniform> perInstance: PerInstance;

@vertex
fn vs_main(
    @location(0) position: vec4<f32>,
    @location(1) tex_coord: vec2<f32>,
) -> VertexOutput {
    var out: VertexOutput;
    out.tex_coord = tex_coord;
    out.position = locals.transform * vec4<f32>(position.xyz + perInstance.position.xyz, 1.0);
    return out;
}

@group(0) @binding(1)
var r_color: texture_2d<f32>;

@group(0)
@binding(2)
var r_sampler: sampler;

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    return vec4<f32>(textureSample(r_color, r_sampler, in.tex_coord).rgb, 1.0);
}
