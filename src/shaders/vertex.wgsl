/// Object attributes from which an object transformation matrix may be generated.
struct ObjectTransforms {
    /// The position of the object's mesh in world space.
    position: vec3<f32>,
    rotation: vec3<f32>,
    /// The scale factor of the object's mesh.
    scale: f32,
}

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

/// The transforms for the object that the current vertex belongs to.
@group(1) @binding(0)
var<uniform> object_transforms: ObjectTransforms;

/// The 4x4 identity matrix.
fn identity_matrix() -> mat4x4<f32> {
    return mat4x4(
        1., 0., 0., 0.,
        0., 1., 0., 0.,
        0., 0., 1., 0.,
        0., 0., 0., 1.,
    );
}

struct SinCos {
    sin: f32,
    cos: f32,
}

fn calc_sin_cos(radians: f32) -> SinCos {
    var output: SinCos;
    output.sin = sin(radians);
    output.cos = cos(radians);

    return output;
}

fn object_x_rotation_matrix() -> mat4x4<f32> {
    let sc = calc_sin_cos(object_transforms.rotation.x);
    let s = sc.sin;
    let c = sc.cos;

    return mat4x4(
        1., 0., 0., 0.,
        0.,  c, -s, 0.,
        0.,  s,  c, 0.,
        0., 0., 0., 1.,
    );
}

fn object_y_rotation_matrix() -> mat4x4<f32> {
    let sc = calc_sin_cos(object_transforms.rotation.y);
    let s = sc.sin;
    let c = sc.cos;

    return mat4x4(
         c, 0.,  s, 0.,
        0., 1., 0., 0.,
        -s, 0.,  c, 0.,
        0., 0., 0., 1.,
    );
}

fn object_z_rotation_matrix() -> mat4x4<f32> {
    let sc = calc_sin_cos(object_transforms.rotation.z);
    let s = sc.sin;
    let c = sc.cos;

    return mat4x4(
         c, -s, 0., 0.,
         s,  c, 0., 0.,
        0., 0., 1., 0.,
        0., 0., 0., 1.,
    );
}

fn object_rotation_matrix() -> mat4x4<f32> {
    return object_x_rotation_matrix() * object_y_rotation_matrix() * object_z_rotation_matrix();
}

/// The transformation matrix for the scale transform of the current object.
///
/// This is applied first.
fn object_scale_matrix() -> mat4x4<f32> {
    let f = object_transforms.scale;

    return mat4x4(
         f, 0., 0., 0.,
        0.,  f, 0., 0.,
        0., 0.,  f, 0.,
        0., 0., 0., 1.,
    );
}

/// The transformation matrix for the position transform of the current object.
///
/// This is applied after the scale matrix.
fn object_position_matrix() -> mat4x4<f32> {
    var m = identity_matrix();
    m[3] += vec4<f32>(object_transforms.position, 0.);

    return m;
}

/// The transformation matrix for the current object.
///
/// This is the product of all object transformation matrices.
fn object_transformation_matrix() -> mat4x4<f32> {
    // Because we're using pre-multiplication, the order here is reversed. The true order is:
    // 1. Scale.
    // 2. Rotate.
    // 3. Translate.
    return object_position_matrix() * object_rotation_matrix() * object_scale_matrix();
}

/// The transformation matrix to be applied to the current vertex.
///
/// This is a combination of the object and camera transformation matrices.
fn vertex_transformation_matrix() -> mat4x4<f32> {
    return object_transformation_matrix() * camera_transformation_matrix;
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
