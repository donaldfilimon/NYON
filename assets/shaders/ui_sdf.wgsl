struct UiGlobals {
    viewport: vec2<f32>,
    sdf_width: f32,
    _padding: f32,
}

struct VertexInput {
    @builtin(vertex_index) vertex_index: u32,
    @location(0) rect: vec4<f32>,
    @location(1) uv_rect: vec4<f32>,
    @location(2) color: vec4<f32>,
}

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) uv: vec2<f32>,
    @location(1) color: vec4<f32>,
}

struct PanelInput {
    @builtin(vertex_index) vertex_index: u32,
    @location(0) rect: vec4<f32>,
    @location(1) color: vec4<f32>,
}

struct PanelOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) color: vec4<f32>,
}

@group(0) @binding(0) var ui_atlas: texture_2d<f32>;
@group(0) @binding(1) var ui_sampler: sampler;
@group(0) @binding(2) var<uniform> globals: UiGlobals;

const QUAD_CORNERS: array<vec2<f32>, 6> = array<vec2<f32>, 6>(
    vec2<f32>(0.0, 0.0),
    vec2<f32>(1.0, 0.0),
    vec2<f32>(1.0, 1.0),
    vec2<f32>(0.0, 0.0),
    vec2<f32>(1.0, 1.0),
    vec2<f32>(0.0, 1.0),
);

fn clip_position(rect: vec4<f32>, corner: vec2<f32>) -> vec4<f32> {
    let logical_position = rect.xy + corner * rect.zw;
    let ndc = vec2<f32>(
        logical_position.x / globals.viewport.x * 2.0 - 1.0,
        1.0 - logical_position.y / globals.viewport.y * 2.0,
    );
    return vec4<f32>(ndc, 0.0, 1.0);
}

@vertex
fn panel_vs(input: PanelInput) -> PanelOutput {
    let corner = QUAD_CORNERS[input.vertex_index];
    var output: PanelOutput;
    output.position = clip_position(input.rect, corner);
    output.color = input.color;
    return output;
}

@fragment
fn panel_fs(input: PanelOutput) -> @location(0) vec4<f32> {
    return input.color;
}

@vertex
fn vs_main(input: VertexInput) -> VertexOutput {
    let corner = QUAD_CORNERS[input.vertex_index];

    var output: VertexOutput;
    output.position = clip_position(input.rect, corner);
    output.uv = input.uv_rect.xy + corner * input.uv_rect.zw;
    output.color = input.color;
    return output;
}

@fragment
fn fs_main(input: VertexOutput) -> @location(0) vec4<f32> {
    let distance = textureSample(ui_atlas, ui_sampler, input.uv).r;
    let coverage = smoothstep(0.5 - globals.sdf_width, 0.5 + globals.sdf_width, distance);
    return vec4<f32>(input.color.rgb, input.color.a * coverage);
}
