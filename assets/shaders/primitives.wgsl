struct Globals {
    viewport: vec2<f32>,
    _padding: vec2<f32>,
}

@group(0) @binding(0)
var<uniform> globals: Globals;

struct VertexInput {
    @location(0) position: vec2<f32>,
    @location(1) color: vec4<f32>,
    @location(2) local: vec2<f32>,
    @location(3) shape: u32,
}

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) color: vec4<f32>,
    @location(1) local: vec2<f32>,
    @location(2) @interpolate(flat) shape: u32,
}

@vertex
fn vs_main(input: VertexInput) -> VertexOutput {
    let safe_viewport = max(globals.viewport, vec2<f32>(1.0, 1.0));
    let clip = vec2<f32>(
        input.position.x / safe_viewport.x * 2.0 - 1.0,
        1.0 - input.position.y / safe_viewport.y * 2.0,
    );

    var output: VertexOutput;
    output.position = vec4<f32>(clip, 0.0, 1.0);
    output.color = input.color;
    output.local = input.local;
    output.shape = input.shape;
    return output;
}

@fragment
fn fs_main(input: VertexOutput) -> @location(0) vec4<f32> {
    let kind = input.shape & 0xffu;
    let ring_width_bits = input.shape >> 8u;
    let ring_width = f32(ring_width_bits) / 16777215.0;

    // Compute derivatives before shape-dependent control flow so every fragment
    // in a quad reaches the derivative operation.
    let local_distance = length(input.local);
    let edge_width = max(fwidth(local_distance), 0.0001);
    let outer_coverage = 1.0 - smoothstep(1.0 - edge_width, 1.0 + edge_width, local_distance);
    let inner_radius = 1.0 - ring_width;
    let inner_coverage = smoothstep(
        inner_radius - edge_width,
        inner_radius + edge_width,
        local_distance,
    );

    var coverage = 1.0;
    if kind == 1u {
        coverage = outer_coverage;
    } else if kind == 2u {
        if ring_width_bits == 0u {
            coverage = 0.0;
        } else if ring_width_bits == 0x00ffffffu {
            // Filled discs use this explicit full-width ring endpoint so
            // Metal, WebGPU, and WebGL2 execute the same analytic shape path.
            coverage = outer_coverage;
        } else {
            coverage = outer_coverage * inner_coverage;
        }
    }

    let alpha = input.color.a * coverage;
    if alpha <= 0.0 {
        discard;
    }

    // The pipeline uses straight-alpha blending, so RGB remains unpremultiplied.
    return vec4<f32>(input.color.rgb, alpha);
}
