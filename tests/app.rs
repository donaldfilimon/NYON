use std::time::Duration;

use glam::Vec2;
use intergalactic_warfare::{
    app::{
        AppCore, AppMode, PointerIntent, SurfaceDecision, SurfaceState, game_command_for_action,
        pointer_command, surface_decision,
    },
    editor::{EditorAction, EditorSection},
    engine::input::Action,
    game::{
        model::{FieldAdjustment, FieldKind, GameCommand, WorldId},
        view::GameLayout,
    },
    preferences::{UserPreferencesV1, encode, store::MemoryPreferencesStore},
    scenario::{ScenarioDraft, store::MemoryScenarioStore},
};

fn core() -> AppCore<MemoryScenarioStore> {
    AppCore::new_with_preferences(
        ScenarioDraft::factory_default().validated().unwrap(),
        MemoryScenarioStore::default(),
        MemoryPreferencesStore::with_slot(
            encode(&UserPreferencesV1 {
                onboarding_completed: true,
                ..UserPreferencesV1::default()
            })
            .unwrap(),
        ),
    )
}

#[test]
fn pointer_and_keyboard_mapping_use_explicit_world_ids() {
    let core = core();
    let layout = GameLayout::new(Vec2::new(1440.0, 900.0));
    let target = layout.world_to_screen(core.simulation().state().worlds[2].position);
    assert_eq!(
        pointer_command(
            core.simulation().state(),
            &layout,
            Some(WorldId(0)),
            target,
            PointerIntent::Launch,
        ),
        Some(GameCommand::Launch {
            source: WorldId(0),
            destination: WorldId(2),
        })
    );
    assert_eq!(
        game_command_for_action(
            Action::Increase,
            Some(WorldId(0)),
            Some(WorldId(2)),
            FieldKind::Topology,
        ),
        Some(GameCommand::TuneField {
            world: WorldId(0),
            field: FieldKind::Topology,
            adjustment: FieldAdjustment::Increase,
        })
    );
}

#[test]
fn pause_open_cancel_suspend_and_resume_clear_clock_without_mutating_campaign() {
    let mut core = core();
    core.advance(Duration::from_millis(1));
    assert!(core.clock_alpha() > 0.0);
    core.handle_action(Action::Pause);
    assert!(core.paused());
    assert_eq!(core.clock_alpha(), 0.0);

    let before = core.simulation().canonical_fingerprint();
    core.handle_action(Action::Scenario);
    assert_eq!(core.mode(), AppMode::EditingScenario);
    assert_eq!(core.clock_alpha(), 0.0);
    assert!(!core.queue_game_command(GameCommand::Launch {
        source: WorldId(0),
        destination: WorldId(1),
    }));
    assert_eq!(core.simulation().pending_command_count(), 0);
    core.handle_editor_action(EditorAction::Cancel).unwrap();
    assert_eq!(core.mode(), AppMode::Playing);
    assert_eq!(core.simulation().canonical_fingerprint(), before);

    core.on_suspend();
    core.on_resume();
    assert_eq!(core.clock_alpha(), 0.0);
    assert_eq!(core.simulation().canonical_fingerprint(), before);
}

#[test]
fn validated_editor_apply_is_atomic_and_restart_uses_the_active_scenario() {
    let mut core = core();
    core.handle_action(Action::Scenario);
    core.editor_mut()
        .unwrap()
        .select_section(EditorSection::Rules);
    core.editor_mut()
        .unwrap()
        .focus_widget("scenario.seed")
        .unwrap();
    core.editor_mut()
        .unwrap()
        .replace_focused_text("0000000000000001");
    core.handle_editor_action(EditorAction::ApplyAndRestart)
        .unwrap();
    core.handle_editor_action(EditorAction::Confirm).unwrap();
    assert_eq!(core.mode(), AppMode::Playing);
    assert_eq!(core.active_scenario().seed(), 1);
    assert_eq!(core.simulation().state().next_tick.0, 0);
    assert_eq!(core.next_sequence(), 0);

    core.advance(Duration::from_millis(17));
    assert_eq!(core.simulation().state().next_tick.0, 1);
    core.handle_action(Action::Restart);
    assert_eq!(core.active_scenario().seed(), 1);
    assert_eq!(core.simulation().state().next_tick.0, 0);
    assert_eq!(core.next_sequence(), 0);
}

#[test]
fn only_an_accepted_command_marks_advisory_materially_dirty() {
    let mut core = core();
    let initial_request = core.advisory_snapshot().unwrap().metadata.request_id;
    assert!(core.queue_game_command(GameCommand::Launch {
        source: WorldId(0),
        destination: WorldId(3),
    }));
    assert_eq!(core.next_sequence(), 1);
    assert_eq!(
        core.advisory_snapshot().unwrap().metadata.request_id,
        initial_request
    );
    core.advance(Duration::from_millis(17));
    assert!(core.advisory_snapshot().unwrap().metadata.request_id > initial_request);

    core.handle_action(Action::Restart);
    assert_eq!(core.next_sequence(), 0);
}

#[test]
fn every_wgpu_surface_state_has_an_explicit_decision() {
    assert_eq!(
        surface_decision(SurfaceState::Success),
        SurfaceDecision::Render
    );
    assert_eq!(
        surface_decision(SurfaceState::Suboptimal),
        SurfaceDecision::RenderThenReconfigure
    );
    assert_eq!(
        surface_decision(SurfaceState::Timeout),
        SurfaceDecision::Skip
    );
    assert_eq!(
        surface_decision(SurfaceState::Occluded),
        SurfaceDecision::Skip
    );
    assert_eq!(
        surface_decision(SurfaceState::Outdated),
        SurfaceDecision::Reconfigure
    );
    assert_eq!(
        surface_decision(SurfaceState::Lost),
        SurfaceDecision::Recreate
    );
    assert_eq!(
        surface_decision(SurfaceState::Validation),
        SurfaceDecision::Fatal
    );
}

#[test]
fn scene_picking_camera_and_preview_remain_session_only() {
    let mut core = core();
    let before = core.simulation().canonical_fingerprint();
    let world = core.scene_frame().worlds[0];
    let viewport = core.viewport();
    let clip = core.camera().current_matrices.view_projection * world.position.extend(1.0);
    let ndc = clip.truncate() / clip.w;
    let pointer = Vec2::new(
        (ndc.x + 1.0) * viewport.x * 0.5,
        (1.0 - ndc.y) * viewport.y * 0.5,
    );

    assert!(!core.select_scene_world(pointer, true));
    assert_eq!(core.selected_world(), None);
    assert!(core.select_scene_world(pointer, false));
    assert_eq!(core.selected_world(), Some(WorldId(0)));
    core.hover_scene_world(pointer, false);
    assert_eq!(
        core.preview_contextual_launch(),
        Some(Err(
            intergalactic_warfare::game::simulation::CommandRejection::SameSourceAndTarget
        ))
    );

    let yaw = core.camera().target_yaw;
    core.camera_mut().orbit(-0.2, 0.1);
    assert_ne!(core.camera().target_yaw, yaw);
    core.reset_camera();
    core.zoom_camera(120.0);

    assert_eq!(core.simulation().canonical_fingerprint(), before);
    assert_eq!(core.simulation().pending_command_count(), 0);
}
