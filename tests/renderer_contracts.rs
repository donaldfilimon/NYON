use std::mem::{offset_of, size_of};

fn installed_shell(
    screen: nyon::app::client_runtime::ClientScreen,
) -> nyon::ui::platform::PlatformUiFrame {
    use nyon::ui::platform::*;
    build_shell_platform_frame(ShellPlatformInput {
        screen,
        capabilities: &[],
        credits_visible: false,
        recovery_message: None,
        continue_available: false,
        backend: None,
        preferences: Default::default(),
        viewport: glam::Vec2::new(1280.0, 720.0),
        focused: None,
    })
}

#[test]
fn guide_composition_keeps_only_bounded_fallback_when_chrome_installation_fails() {
    use nyon::{
        app::client_runtime::ClientScreen,
        ui::{AtlasMetrics, guide::*, platform::*},
    };
    let guide = build_guide_frame(
        GuideLocation::for_screen(ClientScreen::GalaxyWorkshop),
        glam::Vec2::new(1280.0, 720.0),
        false,
        None,
    );
    let frame_before = guide.platform.clone();
    let mut metrics = AtlasMetrics::embedded().unwrap();
    let mut ui = UiBatch::default();
    let mut overlay = PrimitiveBatch::default();
    install_guide_batches(&guide, &metrics, &mut ui, &mut overlay).unwrap();
    assert!(!ui.glyphs().is_empty() && !ui.panels().is_empty());
    assert!(!overlay.vertices().is_empty());
    metrics.entries.clear();
    let result = install_guide_batches(&guide, &metrics, &mut ui, &mut overlay);
    assert!(result.is_err());
    assert!(ui.glyphs().is_empty() && ui.panels().is_empty());
    let mut fallback = PrimitiveBatch::default();
    fallback.text(
        glam::Vec2::splat(8.0),
        1.0,
        [1.0; 4],
        PlatformFallbackCode::Atlas.text(),
    );
    assert!(
        overlay.vertices() == fallback.vertices(),
        "Guide failure overlay must contain only {} fallback vertices, got {} (body appended)",
        fallback.vertices().len(),
        overlay.vertices().len()
    );
    assert_eq!(guide.platform, frame_before);
    assert!(!guide.platform.focus_order().is_empty());
    for action in guide.platform.focus_order() {
        assert!(guide.platform.action(&action).is_some());
    }
}

#[test]
fn actual_platform_installation_replaces_layers_and_preserves_semantics_on_both_failures() {
    use nyon::{
        app::client_runtime::ClientScreen,
        ui::{AtlasMetrics, UiBatchError, platform::*},
    };
    let mut frame = installed_shell(ClientScreen::Settings);
    let action = frame.focus_order()[0].clone();
    frame.append_status_line("Waiting for Workshop save");
    frame.reconcile_focused(Some(&action));
    let metrics = AtlasMetrics::embedded().unwrap();
    let mut ui = UiBatch::default();
    let mut overlay = PrimitiveBatch::default();
    let witnesses = install_platform_batches(&frame, &metrics, &mut ui, &mut overlay).unwrap();
    assert!(!ui.glyphs().is_empty() && !ui.panels().is_empty());
    assert_eq!(witnesses.len(), 1);
    assert_eq!(witnesses[0].kind, PlatformDecorationKind::Focus);
    assert_eq!(witnesses[0].vertex_span, [0, 24]);
    let sdf = nyon::ui::platform_sdf::build_platform_ui_batch(&frame, &metrics).unwrap();
    assert!(
        sdf.text_runs.iter().any(|run| run
            .exact_text
            .to_ascii_lowercase()
            .contains("waiting for workshop save")),
        "{:?}",
        sdf.text_runs
    );
    let before = frame.clone();
    let mut missing_metrics = metrics.clone();
    missing_metrics.entries.clear();
    assert!(install_platform_batches(&frame, &missing_metrics, &mut ui, &mut overlay).is_err());
    assert!(ui.glyphs().is_empty() && ui.panels().is_empty());
    let mut expected = PrimitiveBatch::default();
    expected.text(
        glam::Vec2::splat(8.0),
        1.0,
        [1.0; 4],
        PlatformFallbackCode::Atlas.text(),
    );
    assert_eq!(overlay.vertices(), expected.vertices());
    assert_eq!(frame, before);
    assert!(frame.action(&action).is_some());
    install_platform_batches(&frame, &metrics, &mut ui, &mut overlay).unwrap();
    // SDF still succeeds, but an owner outside its accepted viewport fails overlay validation.
    frame
        .controls
        .iter_mut()
        .find(|control| control.action_id == action)
        .unwrap()
        .bounds
        .min
        .x = -1.0;
    assert!(nyon::ui::platform_sdf::build_platform_ui_batch(&frame, &metrics).is_ok());
    assert_eq!(
        install_platform_batches(&frame, &metrics, &mut ui, &mut overlay),
        Err(UiBatchError::NonFiniteGeometry)
    );
    assert!(ui.glyphs().is_empty() && ui.panels().is_empty());
    expected.clear();
    expected.text(
        glam::Vec2::splat(8.0),
        1.0,
        [1.0; 4],
        PlatformFallbackCode::Geometry.text(),
    );
    assert_eq!(overlay.vertices(), expected.vertices());
    assert!(frame.action(&action).is_some());
}

#[test]
fn actual_installer_capacity_failure_clears_both_previous_layers() {
    use nyon::{
        app::client_runtime::ClientScreen,
        ui::{AtlasMetrics, UiBatchError, platform::*},
    };
    let mut frame = installed_shell(ClientScreen::Settings);
    frame.reconcile_focused(Some(&frame.focus_order()[0].clone()));
    let metrics = AtlasMetrics::embedded().unwrap();
    let mut ui = UiBatch::default();
    let mut overlay = PrimitiveBatch::default();
    install_platform_batches(&frame, &metrics, &mut ui, &mut overlay).unwrap();
    assert!(!ui.glyphs().is_empty() && !overlay.vertices().is_empty());
    frame.background = PlatformBackground::ChromeOnly;
    frame.layout.chrome = vec![frame.layout.top_bar; 513];
    let before = frame.clone();
    assert!(matches!(
        install_platform_batches(&frame, &metrics, &mut ui, &mut overlay),
        Err(UiBatchError::PanelCapacity { .. })
    ));
    assert!(ui.glyphs().is_empty() && ui.panels().is_empty());
    let mut expected = PrimitiveBatch::default();
    expected.text(
        glam::Vec2::splat(8.0),
        1.0,
        [1.0; 4],
        PlatformFallbackCode::Capacity.text(),
    );
    assert_eq!(overlay.vertices(), expected.vertices());
    assert_eq!(frame, before);
    assert!(frame.action(&frame.focus_order()[0]).is_some());
}

#[test]
fn actual_glyph_overflow_preserves_adapter_actions_and_replaces_installed_layers() {
    use nyon::{
        app::client_runtime::ClientScreen,
        ui::{AtlasMetrics, UiBatchError, platform::*},
    };
    let mut frame = installed_shell(ClientScreen::Settings);
    let focused = frame.focus_order()[0].clone();
    frame.reconcile_focused(Some(&focused));
    let metrics = AtlasMetrics::embedded().unwrap();
    let mut ui = UiBatch::default();
    let mut overlay = PrimitiveBatch::default();
    install_platform_batches(&frame, &metrics, &mut ui, &mut overlay).unwrap();
    assert!(!ui.glyphs().is_empty() && !ui.panels().is_empty() && !overlay.vertices().is_empty());
    // Fault injection at the immutable presentation boundary, not a legal Workshop capacity fixture.
    let template = frame.visible_nodes[0].clone();
    frame.visible_nodes.extend((0..8193).map(|_| {
        let mut record = template.clone();
        record.display_text = "X".to_owned();
        record.icon = None;
        record.prewrapped_lines = None;
        record
    }));
    let before = frame.clone();
    assert!(matches!(
        install_platform_batches(&frame, &metrics, &mut ui, &mut overlay),
        Err(UiBatchError::Capacity { requested: 8193 })
    ));
    assert!(ui.glyphs().is_empty() && ui.panels().is_empty());
    let mut expected = PrimitiveBatch::default();
    expected.text(
        glam::Vec2::splat(8.0),
        1.0,
        [1.0; 4],
        PlatformFallbackCode::Capacity.text(),
    );
    assert_eq!(overlay.vertices(), expected.vertices());
    assert_eq!(frame, before);
    #[cfg(not(target_arch = "wasm32"))]
    {
        let mut adapter = nyon::ui::platform_native::NativeSemanticAdapter::default();
        adapter.sync(&frame.semantics, Some(&focused));
        let update = adapter.full_tree_update();
        assert_eq!(update.focus, adapter.action_node(&focused).unwrap());
        for action in frame.focus_order() {
            let node = adapter.action_node(&action).unwrap();
            assert!(adapter.request_node_action(node));
            assert!(frame.action(&action).is_some());
        }
        assert_eq!(
            adapter.drain_actions().collect::<Vec<_>>(),
            frame.focus_order()
        );
    }
}

#[test]
fn typed_overlay_rejects_unowned_overlapping_out_of_range_and_forged_geometry() {
    use nyon::{app::client_runtime::ClientScreen, ui::platform::*};
    let mut frame = installed_shell(ClientScreen::Settings);
    frame.reconcile_focused(Some(&frame.focus_order()[0].clone()));
    let make = || {
        let mut overlay = PlatformPrimitiveOverlay::default();
        frame.append_primitive_overlay(&mut overlay).unwrap();
        overlay
    };
    let mut overlay = make();
    overlay.witnesses.clear();
    assert!(overlay.validate(&frame).is_err());
    let mut overlay = make();
    overlay.witnesses.push(overlay.witnesses[0].clone());
    assert!(overlay.validate(&frame).is_err());
    let mut overlay = make();
    overlay.witnesses[0].vertex_span[1] += 6;
    assert!(overlay.validate(&frame).is_err());
    let mut overlay = make();
    overlay.batch.clear();
    let bounds = overlay.witnesses[0].bounds;
    for _ in 0..4 {
        overlay
            .batch
            .quad(bounds.center(), (bounds.max - bounds.min) * 0.5, [1.0; 4]);
    }
    assert!(
        overlay.validate(&frame).is_err(),
        "ordinary control quads cannot masquerade as focus"
    );
    let mut overlay = make();
    overlay.batch.clear();
    for _ in 0..4 {
        overlay
            .batch
            .quad(glam::Vec2::ZERO, glam::Vec2::splat(500.0), [1.0; 4]);
    }
    assert!(
        overlay.validate(&frame).is_err(),
        "declared bounds cannot conceal escaped vertices"
    );
    let mut overlay = make();
    overlay.witnesses[0].bounds.max.x = f32::NAN;
    assert!(overlay.validate(&frame).is_err());
}

#[test]
fn shared_installer_converts_shell_guide_and_workshop_without_primitive_chrome() {
    use nyon::{
        app::client_runtime::ClientScreen,
        ui::{AtlasMetrics, guide::*, platform::*, workshop::*},
    };
    let metrics = AtlasMetrics::embedded().unwrap();
    let mut frames = vec![
        installed_shell(ClientScreen::MainMenu),
        installed_shell(ClientScreen::Settings),
        installed_shell(ClientScreen::RecoverableError),
    ];
    frames.push(build_shell_platform_frame(ShellPlatformInput {
        screen: ClientScreen::MainMenu,
        capabilities: &[],
        credits_visible: true,
        recovery_message: Some("Recovery available"),
        continue_available: true,
        backend: None,
        preferences: Default::default(),
        viewport: glam::Vec2::new(1280.0, 720.0),
        focused: None,
    }));
    let guide = build_guide_frame(
        GuideLocation::for_screen(ClientScreen::GalaxyWorkshop),
        glam::Vec2::new(1280.0, 720.0),
        false,
        None,
    );
    frames.push(guide.platform.clone());
    let mut scaled_shell = installed_shell(ClientScreen::Settings);
    scaled_shell.layout.ui_scale = 1.3;
    scaled_shell.reconcile_focused(None);
    frames.push(scaled_shell);
    let catalog = nyon_workshop_core::decode_catalog_pack(include_bytes!(
        "../assets/workshop/core-pack-v1.json"
    ))
    .unwrap();
    let session = nyon::workshop::session::WorkshopSession::new(
        nyon::workshop::WorkshopHistory::from_seed_u64(catalog, 42),
    );
    let before = session.snapshot().clone();
    let model = WorkshopUiModel::build(session.snapshot(), WorkshopUiContext::default());
    for viewport in [
        glam::Vec2::new(1280.0, 720.0),
        glam::Vec2::new(723.0, 802.0),
    ] {
        let mut pending = build_workshop_platform_frame(&model, viewport, None);
        pending.high_contrast = true;
        let message =
            "Waiting for Workshop save and Continue selection; press Escape to cancel exit";
        pending.append_status_line(message);
        pending.reconcile_focused(Some(&pending.focus_order()[0].clone()));
        let mut ui = UiBatch::default();
        let mut overlay = PrimitiveBatch::default();
        let witnesses =
            install_platform_batches(&pending, &metrics, &mut ui, &mut overlay).unwrap();
        assert!(
            witnesses
                .iter()
                .all(|w| w.kind == PlatformDecorationKind::Focus)
        );
        let sdf = nyon::ui::platform_sdf::build_platform_ui_batch(&pending, &metrics).unwrap();
        let status: Vec<_> = sdf
            .text_runs
            .iter()
            .filter(|run| run.semantic_id.as_str().starts_with("platform.status."))
            .collect();
        assert_eq!(
            status
                .iter()
                .map(|run| run.exact_text.as_str())
                .collect::<Vec<_>>()
                .join(" "),
            message
        );
        assert!(status.iter().all(|run| run.clip.contains_rect(run.bounds)
            && run.emitted_glyph_range[1] > run.emitted_glyph_range[0]));
        assert!(
            sdf.panel_witnesses.iter().any(
                |panel| panel.role == nyon::ui::platform_sdf::PlatformPanelRole::ContrastBorder
            )
        );
    }
    frames.push(build_workshop_platform_frame(
        &model,
        glam::Vec2::new(1280.0, 720.0),
        None,
    ));
    for frame in frames {
        let mut ui = UiBatch::default();
        let mut overlay = PrimitiveBatch::default();
        let semantics = frame.semantics.clone();
        let witnesses = install_platform_batches(&frame, &metrics, &mut ui, &mut overlay).unwrap();
        assert!(!ui.glyphs().is_empty() && !ui.panels().is_empty());
        assert!(witnesses.is_empty() && overlay.vertices().is_empty());
        assert_eq!(frame.semantics, semantics);
        let sdf = nyon::ui::platform_sdf::build_platform_ui_batch(&frame, &metrics).unwrap();
        for run in sdf.text_runs.iter().filter(|run| {
            run.semantic_id.as_str() == "platform.title"
                || run.semantic_id.as_str().starts_with("platform.status.")
        }) {
            assert!(
                run.clip.contains_rect(run.bounds),
                "title/status line must fit: {run:?}"
            );
            assert!(run.emitted_glyph_range[1] > run.emitted_glyph_range[0]);
            assert!(
                frame
                    .controls
                    .iter()
                    .all(|control| !control.bounds.overlaps(run.bounds))
            );
        }
        let underlay = PrimitiveBatch::default();
        let render = RenderFrame {
            logical_viewport: frame.viewport.to_array(),
            physical_target: [1280, 720],
            quality: GraphicsQuality::Auto,
            scene: None,
            ui: Some(&ui),
            primitive_underlay: &underlay,
            primitive_overlay: &overlay,
        };
        assert!(std::ptr::eq(render.ui.unwrap(), &ui));
        assert!(std::ptr::eq(render.primitive_overlay, &overlay));
    }
    assert_eq!(session.snapshot(), &before);
    let mut ui = UiBatch::default();
    let mut overlay = PrimitiveBatch::default();
    install_platform_batches(&guide.platform, &metrics, &mut ui, &mut overlay).unwrap();
    guide.draw_body(&mut overlay);
    assert!(!guide.lines.is_empty() && !overlay.vertices().is_empty());
    for vertex in overlay.vertices() {
        assert!(vertex.position[1] >= 134.0 && vertex.position[1] < 720.0);
        assert!(guide.platform.controls.iter().all(|control| {
            !control
                .bounds
                .contains(glam::Vec2::from_array(vertex.position))
        }));
    }
}

#[test]
fn platform_primitive_slice_contains_no_ordinary_chrome() {
    use glam::Vec2;
    use nyon::{app::client_runtime::ClientScreen, ui::platform::*};
    let frame = build_shell_platform_frame(ShellPlatformInput {
        screen: ClientScreen::MainMenu,
        capabilities: &[],
        credits_visible: false,
        recovery_message: None,
        continue_available: false,
        backend: None,
        preferences: Default::default(),
        viewport: Vec2::new(1280.0, 720.0),
        focused: None,
    });
    let mut overlay = PrimitiveBatch::default();
    frame.draw(&mut overlay);
    assert!(
        overlay.vertices().is_empty(),
        "unfocused shell chrome must be SDF, with no ordinary primitive panels or text"
    );
}

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
fn classic_scene_abi_remains_the_only_scene_payload_during_layout_repair() {
    let source = include_str!("../src/engine/render_frame.rs");
    assert!(source.contains("pub scene: Option<&'a SceneFrame>"));
    assert!(!source.contains("RenderScene"));
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
