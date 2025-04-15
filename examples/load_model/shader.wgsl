struct CameraUniform {
    view_proj: mat4x4f,
};

struct VertexInput {
    @location(4) position: vec3f,
    @location(5) tex_coords: vec2f,
}

struct InstanceInput {
    @location(0) model_matrix_0: vec4f,
    @location(1) model_matrix_1: vec4f,
    @location(2) model_matrix_2: vec4f,
    @location(3) model_matrix_3: vec4f,
};

struct VertexOutput {
    @builtin(position) clip_position: vec4f,
    @location(0) tex_coords: vec2f,
}

@group(0) @binding(0) // 1.
var<uniform> camera: CameraUniform;

@vertex
fn vs_main(
    model: VertexInput,
    instance: InstanceInput,
) -> VertexOutput {
    var out: VertexOutput;
    let model_matrix = mat4x4f(
        instance.model_matrix_0,
        instance.model_matrix_1,
        instance.model_matrix_2,
        instance.model_matrix_3,
    );
    out.tex_coords = model.tex_coords;
    out.clip_position = camera.view_proj * model_matrix * vec4f(model.position, 1.0); // 2.
    return out;
}

@group(1) @binding(0)
var t_diffuse: texture_2d<f32>;
@group(1) @binding(1)
var s_diffuse: sampler;

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4f {
    return textureSample(t_diffuse, s_diffuse, in.tex_coords);
}