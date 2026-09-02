struct SceneGlobals {
    view_projection: mat4x4<f32>,
    camera_position: vec4<f32>,
    visual: vec4<f32>,
    effects: vec4<f32>,
};

@group(0) @binding(0)
var<uniform> globals: SceneGlobals;

struct BackgroundOut {
    @builtin(position) position: vec4<f32>,
    @location(0) uv: vec2<f32>,
};

fn hash21(point: vec2<f32>) -> f32 {
    let seed = globals.visual.yz * vec2<f32>(0.000013, 0.000019);
    return fract(sin(dot(point + seed, vec2<f32>(127.1, 311.7))) * 43758.5453);
}

@vertex
fn vs_main(@builtin(vertex_index) vertex_index: u32) -> BackgroundOut {
    let positions = array<vec2<f32>, 3>(
        vec2<f32>(-1.0, -1.0),
        vec2<f32>(3.0, -1.0),
        vec2<f32>(-1.0, 3.0),
    );
    var output: BackgroundOut;
    output.position = vec4<f32>(positions[vertex_index], 0.999, 1.0);
    output.uv = positions[vertex_index] * 0.5 + 0.5;
    return output;
}

@fragment
fn fs_main(input: BackgroundOut) -> @location(0) vec4<f32> {
    let parallax = globals.visual.zw * 0.35;
    let uv = input.uv + parallax;
    let cell = floor(uv * 180.0);
    let star_hash = hash21(cell);
    let local = fract(uv * 180.0) - 0.5;
    let star = select(0.0, smoothstep(0.055, 0.0, length(local)), star_hash > 0.985);
    let nebula_a = sin(uv.x * 5.3 + sin(uv.y * 7.1) + globals.visual.x * 0.015);
    let nebula_b = sin(uv.y * 4.7 - uv.x * 2.9);
    let nebula = smoothstep(0.25, 1.35, nebula_a + nebula_b) * mix(0.022, 0.055, globals.effects.y);
    let base = vec3<f32>(0.006, 0.012, 0.035);
    let haze = vec3<f32>(0.06, 0.16, 0.27) * nebula;
    let starlight = vec3<f32>(0.54, 0.86, 1.0) * star * (0.3 + star_hash * 0.7);
    return vec4<f32>(base + haze + starlight, 1.0);
}
