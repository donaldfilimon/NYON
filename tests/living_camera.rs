use glam::Vec2;
use nyon::{
    living_camera::{LivingCameraLevel, LivingCameraState},
    living_content::LivingContent,
    sandbox_session::{SandboxSession, SandboxUiAction},
    ui::platform::PlatformRect,
};

#[test]
fn finite_camera_inputs_are_bounded_and_rejected_inputs_are_atomic() {
    let mut camera = LivingCameraState::default();
    assert!(camera.pan(Vec2::splat(f32::MAX)));
    assert_eq!(camera.pan, Vec2::splat(4096.0));
    assert!(camera.orbit(Vec2::splat(f32::MAX)));
    assert_eq!((camera.yaw_degrees, camera.pitch_degrees), (45.0, 70.0));
    assert!(camera.zoom_at(f32::MAX, Vec2::ZERO));
    assert_eq!(camera.zoom, 8.0);
    let before = camera;
    assert!(!camera.pan(Vec2::new(f32::NAN, 0.0)));
    assert!(!camera.orbit(Vec2::new(0.0, f32::INFINITY)));
    assert!(!camera.zoom_at(0.0, Vec2::ZERO));
    assert!(!camera.zoom_at(1.0, Vec2::splat(f32::NAN)));
    assert_eq!(camera, before);
}

#[test]
fn camera_scope_memory_fit_and_pointer_zoom_preserve_authority() {
    let mut session = SandboxSession::from_authority(LivingContent::ValeConfluence.open().unwrap());
    let bookmark = session.bookmark();
    let dirty = session.dirty();
    let generation = session.notebook_generation();
    let bounds = PlatformRect::from_xywh(0.0, 0.0, 1280.0, 720.0);
    assert!(session.camera_pan(Vec2::new(30.0, -10.0)));
    assert!(session.camera_orbit(Vec2::new(10.0, -5.0)));
    let frame = session.scene_frame(bounds);
    let marker = frame
        .entities
        .iter()
        .find(|marker| marker.visible && marker.position.distance(Vec2::new(640.0, 360.0)) > 30.0)
        .unwrap();
    let selection = marker.selection;
    let anchor = marker.position;
    assert!(session.camera_zoom(1.25, anchor, bounds));
    let next = session.scene_frame(bounds);
    let projected = next
        .entities
        .iter()
        .find(|marker| marker.selection == selection)
        .unwrap();
    assert!(projected.position.distance(anchor) < 0.01);
    let galaxy = session.camera_state();
    session
        .apply_action(SandboxUiAction::NavigateSystem)
        .unwrap();
    assert_eq!(session.camera_scope().level, LivingCameraLevel::System);
    assert_eq!(session.camera_state(), LivingCameraState::default());
    session.camera_pan(Vec2::new(60.0, 20.0));
    session.fit_camera_scope();
    assert_ne!(session.camera_state(), LivingCameraState::default());
    assert!(session.camera_state().zoom.is_finite());
    session
        .apply_action(SandboxUiAction::NavigateGalaxy)
        .unwrap();
    assert_eq!(session.camera_state(), galaxy);
    assert!(!session.camera_zoom(f32::NAN, anchor, bounds));
    assert_eq!(session.camera_state(), galaxy);
    assert_eq!(session.bookmark(), bookmark);
    assert_eq!(session.dirty(), dirty);
    assert_eq!(session.notebook_generation(), generation);
}

#[test]
fn imported_candidate_requires_explicit_resolution_before_camera_modal() {
    let mut session = SandboxSession::open().unwrap();
    session.set_import_preview(Some("Validated candidate"));
    for action in [
        SandboxUiAction::CameraPicker,
        SandboxUiAction::NavigatorPicker,
        SandboxUiAction::InspectorPicker,
        SandboxUiAction::RevealSelection,
    ] {
        assert!(session.apply_action(action).is_err());
        assert!(session.creator_modal_open());
    }
}

#[test]
fn fit_uses_current_scope_extent_and_keeps_static_members_inside_across_aspects() {
    use nyon::living_navigation::{LivingEntityKind as K, LivingEntitySelection as S};
    for (width, height) in [(900.0, 600.0), (320.0, 600.0), (900.0, 320.0)] {
        let mut session = SandboxSession::open().unwrap();
        let bounds = PlatformRect::from_xywh(30.0, 40.0, width, height);
        let world = session.selected_world().unwrap();
        let system = session
            .state()
            .worlds
            .as_slice()
            .iter()
            .find(|row| row.id == world)
            .unwrap()
            .system;
        let digest = session.state().digest().unwrap();
        for level in [LivingCameraLevel::System, LivingCameraLevel::World] {
            session
                .apply_action(if level == LivingCameraLevel::System {
                    SandboxUiAction::NavigateSystem
                } else {
                    SandboxUiAction::NavigateWorld
                })
                .unwrap();
            session.scene_frame(bounds);
            session.camera_pan(Vec2::new(300.0, -200.0));
            session.camera_orbit(Vec2::new(15.0, 10.0));
            session.fit_camera_scope();
            let scene = session.scene_frame(bounds);
            let worlds = session
                .state()
                .worlds
                .as_slice()
                .iter()
                .filter(|row| {
                    if level == LivingCameraLevel::System {
                        row.system == system
                    } else {
                        row.id == world
                    }
                })
                .collect::<Vec<_>>();
            for world in &worlds {
                let marker = scene
                    .worlds
                    .iter()
                    .find(|marker| marker.id == world.id)
                    .expect("fitted world remains in canvas");
                assert!(bounds.contains(marker.position));
                for facility in session
                    .state()
                    .facilities
                    .as_slice()
                    .iter()
                    .filter(|row| row.world == world.id)
                {
                    let marker = scene
                        .entities
                        .iter()
                        .find(|marker| marker.selection == S::new(K::Facility, facility.id))
                        .expect("fitted finished industry remains in canvas");
                    assert!(bounds.contains(marker.position));
                }
            }
            if level == LivingCameraLevel::System {
                let star = session
                    .state()
                    .stars
                    .as_slice()
                    .iter()
                    .find(|row| row.system == system)
                    .unwrap();
                assert!(
                    scene
                        .entities
                        .iter()
                        .any(|marker| marker.selection == S::new(K::Star, star.id))
                );
                let points = scene
                    .entities
                    .iter()
                    .filter(|marker| {
                        marker.selection == S::new(K::Star, star.id)
                            || worlds
                                .iter()
                                .any(|world| marker.selection == S::new(K::World, world.id))
                    })
                    .map(|marker| marker.position)
                    .collect::<Vec<_>>();
                let first = points[0];
                let mut min = first;
                let mut max = first;
                for p in points {
                    min = min.min(p);
                    max = max.max(p);
                }
                // A centered scope can be asymmetric, so bbox occupancy is not
                // a fit criterion. Measure the limiting rendered static radius
                // against the fit margin independently of the camera helper.
                let rows = nyon::living_navigation::navigator_rows(session.state());
                let root = S::new(K::System, system);
                let members = rows
                    .iter()
                    .filter(|row| {
                        if matches!(
                            row.selection.kind,
                            K::Fleet
                                | K::Hull
                                | K::Shipment
                                | K::Civilization
                                | K::Lane
                                | K::Hazard
                        ) {
                            return false;
                        }
                        let mut current = Some(row.selection);
                        while let Some(selection) = current {
                            if selection == root {
                                return true;
                            }
                            current = rows
                                .iter()
                                .find(|row| row.selection == selection)
                                .and_then(|row| row.parent);
                        }
                        false
                    })
                    .map(|row| row.selection)
                    .collect::<std::collections::BTreeSet<_>>();
                let center = (bounds.min + bounds.max) * 0.5;
                let half_available = (bounds.max - bounds.min - Vec2::splat(112.0)) * 0.5;
                let mut utilization = 0.0f32;
                for marker in scene
                    .entities
                    .iter()
                    .filter(|marker| members.contains(&marker.selection))
                {
                    assert!(
                        bounds.contains(marker.position),
                        "static member clipped: {:?}",
                        marker.selection
                    );
                    utilization = utilization
                        .max(((marker.position - center).abs() / half_available).max_element());
                }
                if session.camera_state().zoom > 0.25 && session.camera_state().zoom < 8.0 {
                    assert!(
                        (0.99..=1.01).contains(&utilization),
                        "centered fit leaves unused limiting extent: {width}x{height} utilization={utilization} world span={:?}",
                        max - min
                    );
                }
            }
            assert_eq!(session.state().digest().unwrap(), digest);
        }
    }
}
