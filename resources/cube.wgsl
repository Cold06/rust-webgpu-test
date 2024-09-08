struct VertexOutput {
    @location(0) tex_coord: vec2<f32>,
    @location(1) normal: vec3<f32>,
    @builtin(position) position: vec4<f32>,
};

struct PerInstance {
    position: vec4<f32>,
};


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
    @location(1) normal: vec4<f32>,
    @location(2) tex_coord: vec2<f32>,
) -> VertexOutput {
    var out: VertexOutput;
    out.tex_coord = tex_coord;
    out.position = locals.transform * vec4<f32>(position.xyz + perInstance.position.xyz, 1.0);
    out.normal = normal.xyz;
    return out;
}

@group(0) @binding(1)
var r_color: texture_2d<f32>;

@group(0)
@binding(2)
var r_sampler: sampler;

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {

    var norm: vec3<f32> = normalize(in.normal);
    var lightDir: vec3<f32> = normalize(vec3(0.3, 0.9, 0.3));
    var diffuse: f32 = max(dot(norm, lightDir), 0.0);
    var brightness: f32 = max(0.5, diffuse);

    return textureSample(r_color, r_sampler, in.tex_coord) * brightness;
}
