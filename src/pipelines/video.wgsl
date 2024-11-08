struct Vertex {
    @location(0) position: vec4<f32>,
    @location(1) tex_coord: vec2<f32>,
}
struct PerInstance {
    position: vec4<f32>,
};

struct Locals {
    transform: mat4x4<f32>,
};

@group(0) @binding(0)
var<uniform> locals: Locals;

@group(0) @binding(1)
var r_color: texture_2d<f32>;

@group(0)
@binding(2)
var r_sampler: sampler;

@group(1) @binding(0)
var<uniform> perInstance: PerInstance;

@vertex
fn vs_main(vertex: Vertex) -> VertexOutput {
    var out: VertexOutput;
    out.tex_coord = vertex.tex_coord;
    out.position = locals.transform * vec4<f32>(vertex.position.xyz + perInstance.position.xyz, 1.0);
    return out;
}

struct VertexOutput {
    @location(0) tex_coord: vec2<f32>,
    @builtin(position) position: vec4<f32>,
};

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    var r = textureSample(r_color, r_sampler, in.tex_coord).r;

    return vec4<f32>(vec3(r), 1.0);
}
