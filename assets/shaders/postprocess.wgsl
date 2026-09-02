@group(0) @binding(0)
var bloom_texture: texture_2d<f32>;

@group(0) @binding(1)
var bloom_sampler: sampler;

struct PostOut {
    @builtin(position) position: vec4<f32>,
    @location(0) uv: vec2<f32>,
};

@vertex
fn vs_main(@builtin(vertex_index) vertex_index: u32) -> PostOut {
    let positions = array<vec2<f32>, 3>(
        vec2<f32>(-1.0, -1.0),
        vec2<f32>(3.0, -1.0),
        vec2<f32>(-1.0, 3.0),
    );
    var output: PostOut;
    output.position = vec4<f32>(positions[vertex_index], 0.0, 1.0);
    output.uv = positions[vertex_index] * 0.5 + 0.5;
    return output;
}

@fragment
fn fs_main(input: PostOut) -> @location(0) vec4<f32> {
    let dimensions = vec2<f32>(textureDimensions(bloom_texture));
    let texel = 1.0 / dimensions;
    var glow = textureSample(bloom_texture, bloom_sampler, input.uv) * 0.36;
    glow += textureSample(bloom_texture, bloom_sampler, input.uv + vec2<f32>(texel.x, 0.0)) * 0.16;
    glow += textureSample(bloom_texture, bloom_sampler, input.uv - vec2<f32>(texel.x, 0.0)) * 0.16;
    glow += textureSample(bloom_texture, bloom_sampler, input.uv + vec2<f32>(0.0, texel.y)) * 0.16;
    glow += textureSample(bloom_texture, bloom_sampler, input.uv - vec2<f32>(0.0, texel.y)) * 0.16;
    return vec4<f32>(glow.rgb * 0.42, glow.a * 0.24);
}
