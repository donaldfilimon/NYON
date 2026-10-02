//! First-load fit is presentation initialization, never an authority command.
use glam::Vec2;
use nyon::{
    living_camera::LivingCameraState,
    living_content::LivingContent,
    presentation::living::LivingSceneFrame,
    sandbox_session::{SandboxSession, SandboxUiAction as A},
    ui::platform::PlatformRect,
};

#[test]
fn fresh_scopes_fit_once_and_remember_framing_across_resize_and_steps() {
    for content in [LivingContent::Commerce, LivingContent::ValeConfluence] {
        let mut session = SandboxSession::from_authority(content.open().unwrap());
        let bounds = PlatformRect::from_xywh(30.0, 40.0, 1100.0, 650.0);
        let initial = session.bookmark().unwrap();
        let dirty = session.dirty();
        let expected =
            LivingSceneFrame::fitted_camera(session.state(), bounds, session.camera_scope());
        assert_ne!(expected, LivingCameraState::default());
        let first = session.scene_frame(bounds);
        assert_eq!(session.camera_state(), expected);
        assert_eq!(session.scene_frame(bounds), first);
        assert_eq!(session.bookmark().unwrap(), initial);
        assert_eq!(session.dirty(), dirty);
        assert!(
            first
                .worlds
                .iter()
                .all(|world| bounds.contains(world.position))
        );
        session.camera_pan(Vec2::new(17.0, -9.0));
        let galaxy = session.camera_state();
        session.scene_frame(PlatformRect::from_xywh(0.0, 0.0, 800.0, 480.0));
        assert_eq!(
            session.camera_state(),
            galaxy,
            "resize must preserve framing"
        );
        session.step_once().unwrap();
        let stepped = session.bookmark().unwrap();
        session.scene_frame(bounds);
        assert_eq!(session.camera_state(), galaxy, "simulation must not re-fit");
        assert_eq!(session.bookmark().unwrap(), stepped);
        session.apply_action(A::NavigateSystem).unwrap();
        let system_fit =
            LivingSceneFrame::fitted_camera(session.state(), bounds, session.camera_scope());
        session.scene_frame(bounds);
        assert_eq!(session.camera_state(), system_fit);
        assert_ne!(system_fit, LivingCameraState::default());
        session.camera_pan(Vec2::new(-23.0, 11.0));
        let system = session.camera_state();
        session.apply_action(A::NavigateGalaxy).unwrap();
        session.scene_frame(bounds);
        assert_eq!(session.camera_state(), galaxy);
        session.apply_action(A::NavigateSystem).unwrap();
        session.scene_frame(bounds);
        assert_eq!(session.camera_state(), system);
        assert_eq!(session.bookmark().unwrap(), stepped);
    }
}

#[test]
fn first_pointer_camera_input_uses_fitted_base_before_any_scene_render() {
    let mut session = SandboxSession::from_authority(LivingContent::Commerce.open().unwrap());
    let before = session.bookmark().unwrap();
    let bounds = PlatformRect::from_xywh(0.0, 0.0, 1280.0, 720.0);
    let mut expected =
        LivingSceneFrame::fitted_camera(session.state(), bounds, session.camera_scope());
    assert_ne!(expected, LivingCameraState::default());
    let delta = Vec2::new(31.0, -14.0);
    expected.pan(delta);
    assert!(session.camera_pan(delta));
    assert_eq!(session.camera_state(), expected);
    session.scene_frame(bounds);
    assert_eq!(session.camera_state(), expected);
    assert_eq!(session.bookmark().unwrap(), before);
}

#[test]
fn scoped_initial_fit_keeps_target_centered_and_members_inside_asymmetric_extent() {
    use nyon::living_navigation::{LivingEntityKind as K, LivingEntitySelection as S};
    for (width, height) in [(1100.0, 650.0), (600.0, 500.0), (320.0, 600.0)] {
        let mut session = SandboxSession::open().unwrap();
        let bounds = PlatformRect::from_xywh(30.0, 40.0, width, height);
        let before = session.bookmark().unwrap();
        let world = session.selected_world().unwrap();
        for action in [A::NavigateSystem, A::NavigateWorld] {
            session.apply_action(action).unwrap();
            let scope = session.camera_scope();
            let target = S::new(
                if scope.level == nyon::living_camera::LivingCameraLevel::System {
                    K::System
                } else {
                    K::World
                },
                scope.entity.unwrap(),
            );
            let frame = session.scene_frame(bounds);
            let marker = frame
                .entities
                .iter()
                .find(|row| row.selection == target)
                .unwrap();
            assert!(marker.position.distance((bounds.min + bounds.max) * 0.5) < 0.001);
            assert_eq!(session.camera_state().pan, Vec2::ZERO);
            assert!(frame.worlds.iter().all(|row| bounds.contains(row.position)));
            if target.kind == K::World {
                assert_eq!(
                    frame.hit_world((bounds.min + bounds.max) * 0.5),
                    Some(world)
                );
            }
        }
        assert_eq!(session.bookmark().unwrap(), before);
    }
}

#[test]
fn removing_focused_entity_refits_surviving_scope_once_without_stale_focus_target() {
    assert_removed_focus_recovery(false);
}

#[test]
fn removed_focus_refits_even_after_selection_changes_to_surviving_world() {
    assert_removed_focus_recovery(true);
}

fn assert_removed_focus_recovery(change_selection: bool) {
    use nyon::{
        living_creator::LivingCreatorTool,
        living_navigation::{LivingEntityKind as K, LivingEntitySelection as S},
    };
    let mut session = SandboxSession::open().unwrap();
    let bounds = PlatformRect::from_xywh(30.0, 40.0, 900.0, 600.0);
    let facility = session.state().facilities.as_slice()[0].id;
    session
        .select_entity(S::new(K::Facility, facility))
        .unwrap();
    session.scene_frame(bounds);
    assert!(session.focus_camera_selection());
    session.camera_pan(Vec2::new(250.0, -150.0));
    if change_selection {
        let world = session.state().facilities.as_slice()[0].world;
        session.select_entity(S::new(K::World, world)).unwrap();
    }
    session
        .apply_action(A::OpenCreator(LivingCreatorTool::Remove))
        .unwrap();
    let id = facility
        .0
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect::<String>();
    session.edit_creator_field("0.target.id", &id).unwrap();
    session.apply_action(A::CreatorPreview).unwrap();
    session.apply_action(A::CreatorApply).unwrap();
    assert_ne!(
        session.selected_entity(),
        Some(S::new(K::Facility, facility))
    );
    let committed = session.bookmark().unwrap();
    let expected = LivingSceneFrame::fitted_camera(session.state(), bounds, session.camera_scope());
    session.scene_frame(bounds);
    assert_eq!(
        session.camera_state(),
        expected,
        "invalid focus must recover by fitting the surviving scope"
    );
    assert_eq!(session.camera_state().focus_target, None);
    session.camera_pan(Vec2::new(15.0, 10.0));
    let recovered = session.camera_state();
    session.scene_frame(bounds);
    assert_eq!(
        session.camera_state(),
        recovered,
        "recovery must fit only once"
    );
    assert_eq!(session.bookmark().unwrap(), committed);
}
