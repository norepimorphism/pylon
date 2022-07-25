struct Output {
    @builtin(position) pos: vec4<f32>,
}

@vertex
fn main(@location(0) pos: vec3<f32>) -> Output {
    var output: Output;
    output.pos = vec4<f32>(pos.xyz, 1.0);

    return output;
}
