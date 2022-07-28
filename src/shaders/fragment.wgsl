@fragment
fn main(@builtin(position) position: vec4<f32>) -> @location(0) vec4<f32> {
    // TODO
    return vec4<f32>(
        position.x / 512.0,
        position.y / 512.0,
        position.z,
        1.0,
    );
}
