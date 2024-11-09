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

@group(0)
@binding(1)
var main_sampler: sampler;

@group(0) @binding(2)
var y_color: texture_2d<f32>;

@group(0) @binding(3)
var u_color: texture_2d<f32>;

@group(0) @binding(4)
var v_color: texture_2d<f32>;

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
    var y = textureSample(y_color, main_sampler, in.tex_coord).x;
    var u = textureSample(u_color, main_sampler, in.tex_coord).x;
    var v = textureSample(v_color, main_sampler, in.tex_coord).x;

    let r = y + 1.40200 * (v - 128.0 / 255.0);
    let g = y - 0.34414 * (u - 128.0 / 255.0) - 0.71414 * (v - 128.0 / 255.0);
    let b = y + 1.77200 * (u - 128.0 / 255.0);

    let xlinear = vec4(clamp(r, 0.0, 1.0), clamp(g, 0.0, 1.0), clamp(b, 0.0, 1.0), 1.0);

    return vec4(srgb_to_linear(xlinear.xyz), 1.0);
}

fn srgb_to_linear(srgb: vec3<f32>) -> vec3<f32> {
    let threshold = vec3<f32>(0.04045);
    let inv_scale = vec3<f32>(1.0 / 12.92);
    let high_scale = vec3<f32>(1.0 / 1.055);
    let gamma = vec3<f32>(2.4);

    let below_threshold = srgb * inv_scale;
    let above_threshold = pow((srgb + vec3<f32>(0.055)) * high_scale, gamma);

    // Use smoothstep to blend between the two cases based on the threshold
    return mix(below_threshold, above_threshold, step(threshold, srgb));
}