use glam::{Vec2, Vec3, Vec4};
use intergalactic_warfare::{
    game::{
        model::{Campaign, DEFAULT_SEED, FieldKind, RulesV1, WORLD_COUNT, WorldId},
        simulation::Simulation,
        view::{GameLayout, ViewState},
    },
    presentation::{
        CameraState, GraphicsQuality, MAX_DISTANCE, MAX_PITCH_RADIANS, MAX_SCENE_FLEETS,
        MAX_YAW_RADIANS, MIN_DISTANCE, MIN_PITCH_RADIANS, MIN_YAW_RADIANS, MotionPreference,
        PresentationPreferences, SceneRay, SceneWorld, build_scene_frame, pick_world,
        pick_world_after_hud, sector_to_command_plane, visual_seed,
    },
    scenario::ScenarioDraft,
};

fn view_state() -> ViewState {
    ViewState {
        selected_world: Some(WorldId(0)),
        hovered_world: Some(WorldId(1)),
        selected_field: FieldKind::Atmosphere,
        paused: false,
        speed_multiplier: 1,
        help_visible: false,
    }
}

fn project_to_logical(camera: &CameraState, world: Vec3) -> Vec2 {
    let clip = camera.current_matrices.view_projection * world.extend(1.0);
    let ndc = clip.truncate() / clip.w;
    Vec2::new(
        (ndc.x + 1.0) * 0.5 * camera.logical_viewport.x,
        (1.0 - ndc.y) * 0.5 * camera.logical_viewport.y,
    )
}

fn scene_world(id: u8, position: Vec3, radius: f32) -> SceneWorld {
    SceneWorld {
        id: WorldId(id),
        position,
        radius,
        ownership_color: [1.0; 4],
        ownership_pattern: 0,
        fields: [0.5; 3],
        energy: 0.5,
        defense: 0.5,
        hovered: false,
        selected: false,
        hazard: None,
        hazard_field: None,
    }
}

#[test]
fn camera_matrices_remain_finite_at_every_clamped_limit() {
    for viewport in [
        Vec2::new(1.0, 1.0),
        Vec2::new(960.0, 600.0),
        Vec2::new(7680.0, 4320.0),
    ] {
        let mut camera = CameraState::new(viewport);
        for (yaw, pitch, distance) in [
            (MIN_YAW_RADIANS, MIN_PITCH_RADIANS, MIN_DISTANCE),
            (MAX_YAW_RADIANS, MAX_PITCH_RADIANS, MAX_DISTANCE),
        ] {
            camera.target_yaw = yaw;
            camera.target_pitch = pitch;
            camera.target_distance = distance;
            camera.set_motion(MotionPreference::Reduced);
            assert!(camera.current_matrices.is_finite());
            assert!(camera.target_matrices.is_finite());
            let product = camera.current_matrices.view_projection
                * camera.current_matrices.inverse_view_projection;
            for (actual, expected) in product
                .to_cols_array()
                .into_iter()
                .zip(glam::Mat4::IDENTITY.to_cols_array())
            {
                assert!((actual - expected).abs() <= 0.0001);
            }
        }
    }
}

#[test]
fn default_camera_frames_every_world_inside_the_gameplay_safe_area() {
    let viewport = Vec2::new(1440.0, 900.0);
    let layout = GameLayout::new(viewport);
    let camera = CameraState::new(viewport);
    let campaign = Campaign::new(DEFAULT_SEED, RulesV1::default());
    for world in &campaign.worlds {
        let projected = project_to_logical(
            &camera,
            sector_to_command_plane(world.position.x, world.position.y),
        );
        assert!(
            projected.cmpge(layout.playfield_min).all(),
            "{world:?} at {projected}"
        );
        assert!(
            projected.cmple(layout.playfield_max).all(),
            "{world:?} at {projected}"
        );
    }
}

#[test]
fn projected_world_centers_pick_the_expected_id_and_hud_consumes_first() {
    let viewport = Vec2::new(1440.0, 900.0);
    let camera = CameraState::new(viewport);
    let campaign = Campaign::new(DEFAULT_SEED, RulesV1::default());
    let scenario = ScenarioDraft::factory_default().validated().unwrap();
    let frame = build_scene_frame(
        &campaign,
        scenario.fingerprint(),
        0.0,
        view_state(),
        None,
        0.0,
        &camera,
        PresentationPreferences::default(),
    );
    for world in &frame.worlds {
        let pointer = project_to_logical(&camera, world.position);
        let ray = SceneRay::from_logical_pointer(
            pointer,
            viewport,
            camera.current_matrices.inverse_view_projection,
        )
        .unwrap();
        assert_eq!(pick_world(ray, &frame.worlds).unwrap().world, world.id);
        assert_eq!(pick_world_after_hud(true, ray, &frame.worlds), None);
    }
}

#[test]
fn overlapping_hits_tie_by_nearest_positive_distance_then_world_id() {
    let ray = SceneRay {
        origin: Vec3::new(0.0, 0.0, 5.0),
        direction: -Vec3::Z,
    };
    let worlds = [
        scene_world(4, Vec3::ZERO, 1.0),
        scene_world(2, Vec3::ZERO, 1.0),
        scene_world(1, Vec3::new(0.0, 0.0, -3.0), 1.0),
    ];
    let hit = pick_world(ray, &worlds).unwrap();
    assert_eq!(hit.world, WorldId(2));
    assert!((hit.distance - 4.0).abs() <= f32::EPSILON);
}

#[test]
fn camera_zoom_and_reset_are_clamped() {
    let mut camera = CameraState::new(Vec2::new(960.0, 600.0));
    camera.set_motion(MotionPreference::Reduced);
    camera.orbit(0.25, 0.1);
    assert_ne!(camera.yaw, 0.0);
    camera.zoom(100_000.0);
    assert_eq!(camera.distance, MIN_DISTANCE);
    camera.reset();
    assert_eq!(camera.yaw, 0.0);
    assert_eq!(camera.pitch, 55.0_f32.to_radians());
}

#[test]
fn scene_extraction_is_bounded_immutable_and_preference_only() {
    let scenario = ScenarioDraft::factory_default().validated().unwrap();
    let campaign = Simulation::from_scenario(&scenario).state().clone();
    let before = campaign.clone();
    let camera = CameraState::new(Vec2::new(1440.0, 900.0));
    let regular = build_scene_frame(
        &campaign,
        scenario.fingerprint(),
        0.5,
        view_state(),
        None,
        12.0,
        &camera,
        PresentationPreferences::default(),
    );
    let reduced = build_scene_frame(
        &campaign,
        scenario.fingerprint(),
        0.5,
        view_state(),
        None,
        12.0,
        &camera,
        PresentationPreferences {
            graphics_quality: GraphicsQuality::Low,
            motion: MotionPreference::Reduced,
            high_contrast: true,
        },
    );
    assert_eq!(regular.worlds.len(), WORLD_COUNT);
    assert!(regular.fleets.len() <= MAX_SCENE_FLEETS);
    assert_eq!(reduced.visual_time, 0.0);
    assert_eq!(reduced.effects.particle_budget, 0);
    assert!(
        reduced
            .worlds
            .iter()
            .all(|world| world.ownership_pattern != 0)
    );
    assert_eq!(campaign, before);
    assert_eq!(
        visual_seed(scenario.fingerprint()),
        regular.background.visual_seed
    );
}

#[test]
fn invalid_pointer_inputs_do_not_construct_rays() {
    let inverse = glam::Mat4::IDENTITY;
    assert!(SceneRay::from_logical_pointer(Vec2::NAN, Vec2::ONE, inverse).is_none());
    assert!(SceneRay::from_logical_pointer(Vec2::ZERO, Vec2::ZERO, inverse).is_none());
    let zero_w = glam::Mat4::from_cols(Vec4::ZERO, Vec4::ZERO, Vec4::ZERO, Vec4::ZERO);
    assert!(SceneRay::from_logical_pointer(Vec2::ZERO, Vec2::ONE, zero_w).is_none());
}
