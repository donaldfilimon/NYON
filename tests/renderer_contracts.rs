use std::mem::{offset_of, size_of};

use naga::{
    AddressSpace, Binding, ImageClass, ImageDimension, ScalarKind, ShaderStage, TypeInner,
    VectorSize,
};

use nyon::{
    engine::{
        primitives::{PrimitiveBatch, Vertex},
        quality::resolve_quality,
        render_frame::RenderFrame,
        resources::{
            FleetInstance, HaloInstance, MeshVertex, ParticleInstance, RouteInstance,
            SPHERE_INDEX_COUNT, SPHERE_LATITUDE_SEGMENTS, SPHERE_LONGITUDE_SEGMENTS,
            SPHERE_VERTEX_COUNT, WorldInstance, generate_sphere_mesh,
        },
        scene_renderer::{
            BLOOM_SAMPLER_BINDING, BLOOM_SAMPLER_BINDING_TYPE, BLOOM_TEXTURE_BINDING,
            BLOOM_TEXTURE_SAMPLE_TYPE, BLOOM_TEXTURE_VIEW_DIMENSION, SCENE_GLOBALS_BINDING,
            SCENE_GLOBALS_SIZE,
        },
        shader::{POSTPROCESS_WGSL, ROUTES_WGSL, SPACE_WGSL, WORLDS_WGSL, validate_wgsl},
    },
    presentation::GraphicsQuality,
    ui::{UI_ATLAS_HEIGHT, UI_ATLAS_WIDTH, UiBatch},
};

#[test]
fn renderer_abis_are_separate_and_the_primitive_contract_stays_frozen() {
    assert_eq!(size_of::<Vertex>(), 36);
    assert_eq!(size_of::<MeshVertex>(), 24);
    assert_eq!(offset_of!(MeshVertex, normal), 12);
    assert_eq!(size_of::<WorldInstance>(), 128);
    assert_eq!(offset_of!(WorldInstance, color), 64);
    assert_eq!(offset_of!(WorldInstance, metadata), 112);
    assert_eq!(size_of::<RouteInstance>(), 64);
    assert_eq!(size_of::<FleetInstance>(), 32);
    assert_eq!(size_of::<HaloInstance>(), 32);
    assert_eq!(size_of::<ParticleInstance>(), 32);
}

#[test]
fn device_epoch_sphere_uses_the_accepted_fixed_tessellation() {
    assert_eq!(SPHERE_LATITUDE_SEGMENTS, 24);
    assert_eq!(SPHERE_LONGITUDE_SEGMENTS, 48);
    let sphere = generate_sphere_mesh();
    assert_eq!(sphere.vertices.len(), SPHERE_VERTEX_COUNT);
    assert_eq!(sphere.indices.len(), SPHERE_INDEX_COUNT);
}

#[test]
fn graphics_quality_fallback_is_monotonic() {
    let high = resolve_quality(GraphicsQuality::Auto, true);
    assert_eq!(high.effective, GraphicsQuality::High);
    assert_eq!(high.sample_count, 4);
    assert!(high.bloom);

    for requested in [
        GraphicsQuality::Auto,
        GraphicsQuality::High,
        GraphicsQuality::Low,
    ] {
        let low = resolve_quality(requested, false);
        assert_eq!(low.effective, GraphicsQuality::Low);
        assert_eq!(low.sample_count, 1);
        assert!(!low.bloom);
    }
}

#[test]
fn zero_extent_render_frames_are_not_presentable() {
    let underlay = PrimitiveBatch::default();
    let overlay = PrimitiveBatch::default();
    let ui = UiBatch::default();
    for physical_target in [[0, 900], [1440, 0], [0, 0]] {
        let frame = RenderFrame {
            logical_viewport: [1440.0, 900.0],
            physical_target,
            quality: GraphicsQuality::Auto,
            scene: None,
            ui: Some(&ui),
            primitive_underlay: &underlay,
            primitive_overlay: &overlay,
        };
        assert!(!frame.has_presentable_extent());
    }
}

#[test]
fn sdf_ui_is_composed_between_legacy_chrome_and_the_frozen_overlay() {
    assert_eq!([UI_ATLAS_WIDTH, UI_ATLAS_HEIGHT], [1024, 1024]);

    let renderer = include_str!("../src/engine/render.rs");
    let constructor = renderer
        .find("let ui = UiRenderer::new(gpu)?")
        .expect("Renderer must rebuild SDF resources for each device epoch");
    let scene = renderer
        .find("self.scene.render(")
        .expect("scene stage must be present");
    let underlay = renderer
        .find("\"primitive HUD underlay\"")
        .expect("legacy chrome underlay stage must be present");
    let sdf = renderer
        .find("self.ui.encode(&mut encoder")
        .expect("SDF stage must be present");
    let primitive = renderer
        .find("\"primitive focus and diagnostic overlay\"")
        .expect("primitive compatibility stage must be present");

    assert!(constructor < scene);
    assert!(scene < underlay);
    assert!(underlay < sdf);
    assert!(sdf < primitive);
}

#[test]
fn shader_interfaces_match_the_host_layouts_and_postprocess_bindings() {
    assert_layout(
        &MeshVertex::LAYOUT,
        24,
        wgpu::VertexStepMode::Vertex,
        &[
            (0, 0, wgpu::VertexFormat::Float32x3),
            (1, 12, wgpu::VertexFormat::Float32x3),
        ],
    );
    assert_layout(
        &WorldInstance::LAYOUT,
        128,
        wgpu::VertexStepMode::Instance,
        &[
            (2, 0, wgpu::VertexFormat::Float32x4),
            (3, 16, wgpu::VertexFormat::Float32x4),
            (4, 32, wgpu::VertexFormat::Float32x4),
            (5, 48, wgpu::VertexFormat::Float32x4),
            (6, 64, wgpu::VertexFormat::Float32x4),
            (7, 80, wgpu::VertexFormat::Float32x4),
            (8, 96, wgpu::VertexFormat::Float32x4),
            (9, 112, wgpu::VertexFormat::Uint32x4),
        ],
    );
    assert_layout(
        &RouteInstance::LAYOUT,
        64,
        wgpu::VertexStepMode::Instance,
        &[
            (0, 0, wgpu::VertexFormat::Float32x4),
            (1, 16, wgpu::VertexFormat::Float32x4),
            (2, 32, wgpu::VertexFormat::Float32x4),
            (3, 48, wgpu::VertexFormat::Float32x4),
        ],
    );
    for layout in [
        &FleetInstance::LAYOUT,
        &HaloInstance::LAYOUT,
        &ParticleInstance::LAYOUT,
    ] {
        assert_layout(
            layout,
            32,
            wgpu::VertexStepMode::Instance,
            &[
                (0, 0, wgpu::VertexFormat::Float32x4),
                (1, 16, wgpu::VertexFormat::Float32x4),
            ],
        );
    }

    let worlds = validate_wgsl("worlds.wgsl", WORLDS_WGSL).unwrap();
    assert_eq!(
        vertex_inputs(&worlds, "vs_main"),
        vec![
            (0, NumericType::Float3),
            (1, NumericType::Float3),
            (2, NumericType::Float4),
            (3, NumericType::Float4),
            (4, NumericType::Float4),
            (5, NumericType::Float4),
            (6, NumericType::Float4),
            (7, NumericType::Float4),
            (8, NumericType::Float4),
            (9, NumericType::Uint4),
        ]
    );
    let routes = validate_wgsl("routes.wgsl", ROUTES_WGSL).unwrap();
    assert_eq!(
        vertex_inputs(&routes, "route_vs"),
        vec![
            (0, NumericType::Float4),
            (1, NumericType::Float4),
            (2, NumericType::Float4),
            (3, NumericType::Float4),
        ]
    );
    for entry in ["fleet_vs", "halo_vs", "particle_vs"] {
        assert_eq!(
            vertex_inputs(&routes, entry),
            vec![(0, NumericType::Float4), (1, NumericType::Float4)]
        );
    }

    for (label, source) in [
        ("space.wgsl", SPACE_WGSL),
        ("worlds.wgsl", WORLDS_WGSL),
        ("routes.wgsl", ROUTES_WGSL),
    ] {
        let module = validate_wgsl(label, source).unwrap();
        assert_eq!(
            scene_uniform_contract(&module),
            Some((0, SCENE_GLOBALS_BINDING, SCENE_GLOBALS_SIZE as u32))
        );
    }

    let postprocess = validate_wgsl("postprocess.wgsl", POSTPROCESS_WGSL).unwrap();
    assert_eq!(
        postprocess_resources(&postprocess),
        vec![
            (
                0,
                BLOOM_TEXTURE_BINDING,
                PostprocessResource::FilterableTexture2d
            ),
            (
                0,
                BLOOM_SAMPLER_BINDING,
                PostprocessResource::FilteringSampler
            )
        ]
    );
    assert_eq!(
        BLOOM_TEXTURE_SAMPLE_TYPE,
        wgpu::TextureSampleType::Float { filterable: true }
    );
    assert_eq!(BLOOM_TEXTURE_VIEW_DIMENSION, wgpu::TextureViewDimension::D2);
    assert_eq!(
        BLOOM_SAMPLER_BINDING_TYPE,
        wgpu::SamplerBindingType::Filtering
    );
}

#[test]
fn parsed_contract_rejects_mutated_location_type_and_binding() {
    let wrong_location = WORLDS_WGSL.replacen("@location(9) metadata", "@location(10) metadata", 1);
    let module = validate_wgsl("wrong-world-location.wgsl", &wrong_location).unwrap();
    assert_ne!(
        vertex_inputs(&module, "vs_main").last(),
        Some(&(9, NumericType::Uint4))
    );

    let mut wrong_type = ROUTES_WGSL.replace(
        "struct HaloInput {\n    @location(0) position_radius: vec4<f32>,\n    @location(1) color: vec4<f32>,\n};",
        "struct HaloInput {\n    @location(0) position_radius: vec4<f32>,\n    @location(1) color: vec3<f32>,\n};",
    );
    let halo_start = wrong_type.find("fn halo_vs").unwrap();
    let relative_assignment = wrong_type[halo_start..]
        .find("output.color = input.color;")
        .unwrap();
    let assignment_start = halo_start + relative_assignment;
    wrong_type.replace_range(
        assignment_start..assignment_start + "output.color = input.color;".len(),
        "output.color = vec4<f32>(input.color, 1.0);",
    );
    let module = validate_wgsl("wrong-halo-type.wgsl", &wrong_type).unwrap();
    assert_ne!(
        vertex_inputs(&module, "halo_vs"),
        vec![(0, NumericType::Float4), (1, NumericType::Float4)]
    );

    let wrong_binding = POSTPROCESS_WGSL.replacen("@binding(1)", "@binding(2)", 1);
    let module = validate_wgsl("wrong-postprocess-binding.wgsl", &wrong_binding).unwrap();
    assert_ne!(
        postprocess_resources(&module),
        vec![
            (0, 0, PostprocessResource::FilterableTexture2d),
            (0, 1, PostprocessResource::FilteringSampler)
        ]
    );
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum NumericType {
    Float3,
    Float4,
    Uint4,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum PostprocessResource {
    FilterableTexture2d,
    FilteringSampler,
}

fn assert_layout(
    layout: &wgpu::VertexBufferLayout<'_>,
    stride: u64,
    step_mode: wgpu::VertexStepMode,
    attributes: &[(u32, u64, wgpu::VertexFormat)],
) {
    assert_eq!(layout.array_stride, stride);
    assert_eq!(layout.step_mode, step_mode);
    assert_eq!(layout.attributes.len(), attributes.len());
    for (actual, &(shader_location, offset, format)) in layout.attributes.iter().zip(attributes) {
        assert_eq!(actual.shader_location, shader_location);
        assert_eq!(actual.offset, offset);
        assert_eq!(actual.format, format);
    }
}

fn vertex_inputs(module: &naga::Module, entry_name: &str) -> Vec<(u32, NumericType)> {
    let entry = module
        .entry_points
        .iter()
        .find(|entry| entry.name == entry_name && entry.stage == ShaderStage::Vertex)
        .expect("vertex entry point must exist");
    let mut inputs = Vec::new();
    for argument in &entry.function.arguments {
        match &module.types[argument.ty].inner {
            TypeInner::Struct { members, .. } => {
                for member in members {
                    if let Some(location) = binding_location(member.binding.as_ref()) {
                        inputs.push((location, numeric_type(&module.types[member.ty].inner)));
                    }
                }
            }
            inner => {
                if let Some(location) = binding_location(argument.binding.as_ref()) {
                    inputs.push((location, numeric_type(inner)));
                }
            }
        }
    }
    inputs.sort_by_key(|input| input.0);
    inputs
}

fn binding_location(binding: Option<&Binding>) -> Option<u32> {
    match binding {
        Some(Binding::Location { location, .. }) => Some(*location),
        _ => None,
    }
}

fn numeric_type(inner: &TypeInner) -> NumericType {
    match inner {
        TypeInner::Vector {
            size: VectorSize::Tri,
            scalar,
        } if scalar.kind == ScalarKind::Float && scalar.width == 4 => NumericType::Float3,
        TypeInner::Vector {
            size: VectorSize::Quad,
            scalar,
        } if scalar.kind == ScalarKind::Float && scalar.width == 4 => NumericType::Float4,
        TypeInner::Vector {
            size: VectorSize::Quad,
            scalar,
        } if scalar.kind == ScalarKind::Uint && scalar.width == 4 => NumericType::Uint4,
        other => panic!("unexpected vertex input type: {other:?}"),
    }
}

fn scene_uniform_contract(module: &naga::Module) -> Option<(u32, u32, u32)> {
    module.global_variables.iter().find_map(|(_, variable)| {
        let binding = variable.binding.as_ref()?;
        if variable.space != AddressSpace::Uniform {
            return None;
        }
        let TypeInner::Struct { span, .. } = module.types[variable.ty].inner else {
            return None;
        };
        Some((binding.group, binding.binding, span))
    })
}

fn postprocess_resources(module: &naga::Module) -> Vec<(u32, u32, PostprocessResource)> {
    let mut resources = module
        .global_variables
        .iter()
        .filter_map(|(_, variable)| {
            let binding = variable.binding.as_ref()?;
            let resource = match module.types[variable.ty].inner {
                TypeInner::Image {
                    dim: ImageDimension::D2,
                    arrayed: false,
                    class:
                        ImageClass::Sampled {
                            kind: ScalarKind::Float,
                            multi: false,
                        },
                } => PostprocessResource::FilterableTexture2d,
                TypeInner::Sampler { comparison: false } => PostprocessResource::FilteringSampler,
                ref other => panic!("unexpected postprocess resource type: {other:?}"),
            };
            Some((binding.group, binding.binding, resource))
        })
        .collect::<Vec<_>>();
    resources.sort_by_key(|resource| (resource.0, resource.1));
    resources
}
