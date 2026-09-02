use intergalactic_warfare::{
    advisory::gpu::ADVISORY_WGSL,
    engine::shader::{
        POSTPROCESS_WGSL, PRIMITIVES_WGSL, ROUTES_WGSL, SPACE_WGSL, ShaderError, WORLDS_WGSL,
        validate_wgsl,
    },
};

#[test]
fn shipped_primitive_shader_parses_and_validates() {
    validate_wgsl("assets/shaders/primitives.wgsl", PRIMITIVES_WGSL)
        .expect("the shipped primitive shader must pass Naga validation");
}

#[test]
fn tactical_scene_shaders_parse_and_validate() {
    for (label, source) in [
        ("space.wgsl", SPACE_WGSL),
        ("worlds.wgsl", WORLDS_WGSL),
        ("routes.wgsl", ROUTES_WGSL),
        ("postprocess.wgsl", POSTPROCESS_WGSL),
    ] {
        validate_wgsl(label, source).unwrap_or_else(|error| panic!("{error}"));
    }
}

#[test]
fn parse_diagnostic_includes_the_shader_label() {
    let label = "tests/fixtures/broken-parse.wgsl";
    let error = validate_wgsl(label, "this is not WGSL").expect_err("source must fail parsing");

    assert!(matches!(error, ShaderError::Parse { .. }));
    assert!(error.to_string().contains(label));
}

#[test]
fn parse_valid_interface_collision_reaches_validation_diagnostic() {
    let label = "tests/fixtures/interface-collision.wgsl";
    let source = r#"
struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) first: vec4<f32>,
    @location(0) second: vec4<f32>,
}

@vertex
fn vs_main() -> VertexOutput {
    var output: VertexOutput;
    output.position = vec4<f32>(0.0, 0.0, 0.0, 1.0);
    output.first = vec4<f32>(1.0);
    output.second = vec4<f32>(1.0);
    return output;
}
"#;

    let error = validate_wgsl(label, source).expect_err("location collision must fail validation");

    assert!(matches!(error, ShaderError::Validation { .. }));
    assert!(error.to_string().contains(label));
}

#[test]
fn primitive_shader_decodes_the_reviewed_packed_ring_word() {
    assert!(PRIMITIVES_WGSL.contains("input.shape & 0xffu"));
    assert!(PRIMITIVES_WGSL.contains("input.shape >> 8u"));
    assert!(PRIMITIVES_WGSL.contains("16777215.0"));
}

#[test]
fn primitive_shader_handles_packed_ring_width_endpoints_after_derivatives() {
    validate_wgsl("assets/shaders/primitives.wgsl", PRIMITIVES_WGSL)
        .expect("endpoint handling must remain Naga-valid");

    let derivative = PRIMITIVES_WGSL.find("fwidth(local_distance)").unwrap();
    let ring_branch = PRIMITIVES_WGSL.find("if kind == 2u").unwrap();
    assert!(derivative < ring_branch);
    assert!(PRIMITIVES_WGSL.contains("ring_width_bits == 0u"));
    assert!(PRIMITIVES_WGSL.contains("ring_width_bits == 0x00ffffffu"));
    assert!(PRIMITIVES_WGSL.contains("coverage = 0.0"));
    assert!(PRIMITIVES_WGSL.contains("coverage = outer_coverage"));
    assert!(PRIMITIVES_WGSL.contains("coverage = outer_coverage * inner_coverage"));
}

#[test]
fn additive_sprite_rgb_is_masked_by_its_analytic_alpha() {
    assert!(ROUTES_WGSL.contains("input.color.rgb * 1.25 * alpha"));
    assert!(ROUTES_WGSL.contains("input.color.rgb * 1.65 * alpha"));
}

#[test]
fn advisory_shader_parses_validates_and_guards_the_eighth_invocation() {
    validate_wgsl("assets/shaders/advisory.wgsl", ADVISORY_WGSL)
        .expect("the shipped advisory shader must pass Naga validation");
    assert!(ADVISORY_WGSL.contains("array<f32, 84>"));
    assert!(ADVISORY_WGSL.contains("array<f32, 57>"));
    assert!(ADVISORY_WGSL.contains("array<f32, 7>"));
    assert!(ADVISORY_WGSL.contains("@workgroup_size(8)"));
    assert!(ADVISORY_WGSL.contains("if (world_index >= 7u)"));
}
