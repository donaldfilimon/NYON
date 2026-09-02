struct SceneGlobals {
    view_projection: mat4x4<f32>,
    camera_position: vec4<f32>,
    visual: vec4<f32>,
    effects: vec4<f32>,
};

@group(0) @binding(0)
var<uniform> globals: SceneGlobals;

struct RouteInput {
    @location(0) start: vec4<f32>,
    @location(1) control: vec4<f32>,
    @location(2) end: vec4<f32>,
    @location(3) color: vec4<f32>,
};

struct TransparentOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) color: vec4<f32>,
    @location(1) local: vec2<f32>,
};

fn curve(start: vec3<f32>, control: vec3<f32>, end: vec3<f32>, t: f32) -> vec3<f32> {
    let one_minus = 1.0 - t;
    return start * one_minus * one_minus + control * (2.0 * one_minus * t) + end * t * t;
}

@vertex
fn route_vs(input: RouteInput, @builtin(vertex_index) vertex_index: u32) -> TransparentOutput {
    let segment = vertex_index / 2u;
    let endpoint = vertex_index % 2u;
    let t = f32(segment + endpoint) / 32.0;
    var output: TransparentOutput;
    output.position = globals.view_projection * vec4<f32>(curve(input.start.xyz, input.control.xyz, input.end.xyz, t), 1.0);
    let dash = select(0.34, 0.78, (segment % 3u) != 1u);
    output.color = vec4<f32>(input.color.rgb, input.color.a * dash);
    output.local = vec2<f32>(0.0);
    return output;
}

struct FleetInput {
    @location(0) position_size: vec4<f32>,
    @location(1) color: vec4<f32>,
};

@vertex
fn fleet_vs(input: FleetInput, @builtin(vertex_index) vertex_index: u32) -> TransparentOutput {
    let corners = array<vec2<f32>, 6>(
        vec2<f32>(-1.0, -1.0), vec2<f32>(1.0, -1.0), vec2<f32>(1.0, 1.0),
        vec2<f32>(-1.0, -1.0), vec2<f32>(1.0, 1.0), vec2<f32>(-1.0, 1.0),
    );
    let corner = corners[vertex_index];
    var clip = globals.view_projection * vec4<f32>(input.position_size.xyz, 1.0);
    var offset = corner * input.position_size.w * clip.w;
    offset.x *= globals.effects.x;
    clip.x += offset.x;
    clip.y += offset.y;
    var output: TransparentOutput;
    output.position = clip;
    output.color = input.color;
    output.local = corner;
    return output;
}

struct HaloInput {
    @location(0) position_radius: vec4<f32>,
    @location(1) color: vec4<f32>,
};

@vertex
fn halo_vs(input: HaloInput, @builtin(vertex_index) vertex_index: u32) -> TransparentOutput {
    let corners = array<vec2<f32>, 6>(
        vec2<f32>(-1.0, -1.0), vec2<f32>(1.0, -1.0), vec2<f32>(1.0, 1.0),
        vec2<f32>(-1.0, -1.0), vec2<f32>(1.0, 1.0), vec2<f32>(-1.0, 1.0),
    );
    let corner = corners[vertex_index];
    var clip = globals.view_projection * vec4<f32>(input.position_radius.xyz, 1.0);
    var offset = corner * input.position_radius.w * clip.w;
    offset.x *= globals.effects.x;
    clip.x += offset.x;
    clip.y += offset.y;
    var output: TransparentOutput;
    output.position = clip;
    output.color = input.color;
    output.local = corner;
    return output;
}

struct ParticleInput {
    @location(0) position_size: vec4<f32>,
    @location(1) color: vec4<f32>,
};

@vertex
fn particle_vs(input: ParticleInput, @builtin(vertex_index) vertex_index: u32) -> TransparentOutput {
    let corners = array<vec2<f32>, 6>(
        vec2<f32>(-1.0, -1.0), vec2<f32>(1.0, -1.0), vec2<f32>(1.0, 1.0),
        vec2<f32>(-1.0, -1.0), vec2<f32>(1.0, 1.0), vec2<f32>(-1.0, 1.0),
    );
    let corner = corners[vertex_index];
    var clip = globals.view_projection * vec4<f32>(input.position_size.xyz, 1.0);
    var offset = corner * input.position_size.w * clip.w;
    offset.x *= globals.effects.x;
    clip.x += offset.x;
    clip.y += offset.y;
    var output: TransparentOutput;
    output.position = clip;
    output.color = input.color;
    output.local = corner;
    return output;
}

@fragment
fn route_fs(input: TransparentOutput) -> @location(0) vec4<f32> {
    return input.color;
}

@fragment
fn fleet_fs(input: TransparentOutput) -> @location(0) vec4<f32> {
    let radius = length(input.local);
    let alpha = smoothstep(1.0, 0.2, radius) * input.color.a;
    let core = smoothstep(0.45, 0.0, radius);
    return vec4<f32>(input.color.rgb * (0.8 + core * 1.8), alpha);
}

@fragment
fn halo_fs(input: TransparentOutput) -> @location(0) vec4<f32> {
    let radius = length(input.local);
    let outer = smoothstep(1.0, 0.84, radius);
    let inner = smoothstep(0.70, 0.82, radius);
    let alpha = outer * inner * input.color.a;
    return vec4<f32>(input.color.rgb * 1.25 * alpha, alpha);
}

@fragment
fn particle_fs(input: TransparentOutput) -> @location(0) vec4<f32> {
    let radius = length(input.local);
    let alpha = smoothstep(1.0, 0.0, radius) * input.color.a;
    return vec4<f32>(input.color.rgb * 1.65 * alpha, alpha);
}
