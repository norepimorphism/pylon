struct WorldMatrixInput {
    position: vec3<f32>,
    scale: f32,
}

struct Output {
    @builtin(position) pos: vec4<f32>,
}

@group(0) @binding(0)
var<uniform> world_matrix_input: WorldMatrixInput;

fn identity_matrix() -> mat4x4<f32> {
    return mat4x4(
        1., 0., 0., 0.,
        0., 1., 0., 0.,
        0., 0., 1., 0.,
        0., 0., 0., 1.,
    );
}

fn scale_matrix() -> mat4x4<f32> {
    return world_matrix_input.scale * identity_matrix();
}

fn position_matrix() -> mat4x4<f32> {
    return identity_matrix();
}

fn world_matrix() -> mat4x4<f32> {
    return scale_matrix() * position_matrix();
}

fn transform_position(pos: vec3<f32>) -> vec3<f32> {
    return (world_matrix() * vec4<f32>(pos.xyz, 1.0)).xyz;
}

@vertex
fn main(@location(0) pos: vec3<f32>) -> Output {
    var output: Output;
    output.pos = vec4<f32>(transform_position(pos), 1.0);

    return output;
}
