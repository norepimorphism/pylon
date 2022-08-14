/// The output of this vertex shader.
struct Output {
    /// The position of the current vertex in clip space.
    @builtin(position) position: vec4<f32>,
}

/// The precompiled camera transformation matrix supplied by the CPU.
///
/// This transformation matrix is to be applied after the object transformation matrix.
@group(0) @binding(0)
var<uniform> camera_transformation_matrix: mat4x4<f32>;

/// The precompiled transformation matrix for the object that the current vertex belongs to.
@group(1) @binding(0)
var<uniform> object_transformation_matrix: mat4x4<f32>;

/// The transformation matrix to be applied to the current vertex.
///
/// This is a combination of the object and camera transformation matrices.
fn vertex_transformation_matrix() -> mat4x4<f32> {
    return object_transformation_matrix * camera_transformation_matrix;
}

/// Transforms the given vertex according to the vertex transformation matrix.
fn transform_position(position: vec3<f32>) -> vec3<f32> {
    return (vertex_transformation_matrix() * vec4<f32>(position.xyz, 1.0)).xyz;
}

@vertex
fn main(@location(0) position: vec3<f32>) -> Output {
    var output: Output;
    output.position = vec4<f32>(transform_position(position), 1.0);
    output.position.y *= -1.0;

    return output;
}
