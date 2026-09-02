struct SceneGlobals {
    view_projection: mat4x4<f32>,
    camera_position: vec4<f32>,
    visual: vec4<f32>,
    effects: vec4<f32>,
};

@group(0) @binding(0)
var<uniform> globals: SceneGlobals;

struct WorldInput {
    @location(0) position: vec3<f32>,
    @location(1) normal: vec3<f32>,
    @location(2) model_0: vec4<f32>,
    @location(3) model_1: vec4<f32>,
    @location(4) model_2: vec4<f32>,
    @location(5) model_3: vec4<f32>,
    @location(6) color: vec4<f32>,
    @location(7) status: vec4<f32>,
    @location(8) fields: vec4<f32>,
    @location(9) metadata: vec4<u32>,
};

struct WorldOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) world_position: vec3<f32>,
    @location(1) normal: vec3<f32>,
    @location(2) color: vec4<f32>,
    @location(3) status: vec4<f32>,
    @location(4) fields: vec4<f32>,
    @location(5) @interpolate(flat) metadata: vec4<u32>,
};

@vertex
fn vs_main(input: WorldInput) -> WorldOutput {
    let model = mat4x4<f32>(input.model_0, input.model_1, input.model_2, input.model_3);
    let world = model * vec4<f32>(input.position, 1.0);
    var output: WorldOutput;
    output.position = globals.view_projection * world;
    output.world_position = world.xyz;
    output.normal = normalize((model * vec4<f32>(input.normal, 0.0)).xyz);
    output.color = input.color;
    output.status = input.status;
    output.fields = input.fields;
    output.metadata = input.metadata;
    return output;
}

@fragment
fn fs_main(input: WorldOutput) -> @location(0) vec4<f32> {
    let normal = normalize(input.normal);
    let to_camera = normalize(globals.camera_position.xyz - input.world_position);
    let key_light = normalize(vec3<f32>(-0.45, 0.82, 0.35));
    let diffuse = 0.18 + max(dot(normal, key_light), 0.0) * 0.72;
    let rim = pow(1.0 - max(dot(normal, to_camera), 0.0), 2.4);
    let band_axis = input.world_position.y * (12.0 + input.fields.z * 9.0);
    let rotation = globals.visual.x * (0.06 + input.fields.y * 0.05);
    let detailed_bands = 0.80 + 0.20 * sin(band_axis + rotation + f32(input.metadata.x));
    let bands = mix(1.0, detailed_bands, globals.effects.y);
    var pattern = 1.0;
    if input.metadata.y == 1u {
        pattern = select(0.64, 1.0, fract((input.world_position.x + input.world_position.z) * 7.0) > 0.45);
    } else if input.metadata.y == 2u {
        pattern = select(0.62, 1.0, fract(input.world_position.y * 12.0) > 0.5);
    } else if input.metadata.y == 3u {
        pattern = select(0.62, 1.0, fract(atan2(normal.z, normal.x) * 3.0) > 0.5);
    } else if input.metadata.y == 4u {
        pattern = select(0.66, 1.0, fract((input.world_position.x - input.world_position.z) * 9.0) > 0.55);
    }
    var hazard_tint = vec3<f32>(0.0);
    if input.metadata.z == 1u {
        hazard_tint = vec3<f32>(0.05, 0.34, 0.55) * (0.35 + input.fields.w * 0.65);
    } else if input.metadata.z == 2u {
        hazard_tint = vec3<f32>(0.52, 0.14, 0.04) * (0.35 + input.fields.w * 0.65);
    }
    let selected = input.status.z;
    let hovered = input.status.w;
    let atmosphere_detail = mix(0.34, 1.0, globals.effects.y);
    let atmosphere = rim * (0.18 + input.fields.x * 0.65) * atmosphere_detail;
    let energy_glow = input.color.rgb * input.status.x * 0.22;
    let selection = vec3<f32>(1.0, 0.82, 0.24) * selected * rim * 1.35;
    let hover = vec3<f32>(0.7, 0.95, 1.0) * hovered * rim * 0.65;
    let surface = input.color.rgb * diffuse * bands * pattern;
    return vec4<f32>(surface + energy_glow + atmosphere * input.color.rgb + hazard_tint + selection + hover, 1.0);
}
