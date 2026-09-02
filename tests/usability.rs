use std::time::Duration;

use glam::Vec2;
use intergalactic_warfare::{
    app::{AppCore, AppMode, GameSpeed, onboarding::OnboardingStep, settings::SettingsAction},
    engine::input::Action,
    game::{
        model::{Faction, FieldKind, GameCommand, WorldId},
        simulation::Simulation,
    },
    preferences::{UiScale, UserPreferencesV1, encode, store::MemoryPreferencesStore},
    presentation::{
        GestureAction, GraphicsQuality, InteractionController, InteractionHit, MotionPreference,
        PointerButton, PointerSource,
        ui::{
            CommandTrayState, MIN_CONTROL_EXTENT, UiAction, UiBuildState, UiScreen, build_ui_batch,
            build_ui_frame,
        },
    },
    scenario::{ScenarioDraft, store::MemoryScenarioStore},
    ui::{AtlasMetrics, UiBatch},
};

type Core = AppCore<MemoryScenarioStore, MemoryPreferencesStore>;

fn core_with_preferences(preferences: UserPreferencesV1) -> Core {
    core_with_draft_and_preferences(ScenarioDraft::factory_default(), preferences)
}

fn core_with_draft_and_preferences(draft: ScenarioDraft, preferences: UserPreferencesV1) -> Core {
    AppCore::new_with_preferences(
        draft.validated().unwrap(),
        MemoryScenarioStore::default(),
        MemoryPreferencesStore::with_slot(encode(&preferences).unwrap()),
    )
}

fn ready_core() -> Core {
    core_with_preferences(UserPreferencesV1 {
        onboarding_completed: true,
        ..UserPreferencesV1::default()
    })
}

fn projected_world(core: &Core, id: WorldId) -> Vec2 {
    let viewport = core.viewport();
    let world = core
        .scene_frame()
        .worlds
        .into_iter()
        .find(|world| world.id == id)
        .unwrap();
    let clip = core.camera().current_matrices.view_projection * world.position.extend(1.0);
    let ndc = clip.truncate() / clip.w;
    Vec2::new(
        (ndc.x + 1.0) * viewport.x * 0.5,
        (1.0 - ndc.y) * viewport.y * 0.5,
    )
}

#[test]
fn speed_multipliers_match_direct_canonical_stepping_and_clear_residue() {
    let scenario = ScenarioDraft::factory_default().validated().unwrap();
    for (speed, expected_steps) in [
        (GameSpeed::Paused, 0),
        (GameSpeed::Normal, 6),
        (GameSpeed::Fast, 12),
        (GameSpeed::VeryFast, 24),
    ] {
        let mut core = ready_core();
        core.set_game_speed(speed);
        core.advance(Duration::from_millis(100));
        assert_eq!(core.simulation().state().next_tick.0, expected_steps);

        let mut direct = Simulation::from_scenario(&scenario);
        for _ in 0..expected_steps {
            direct.step();
        }
        assert_eq!(
            core.simulation().canonical_fingerprint(),
            direct.canonical_fingerprint()
        );
    }

    let mut core = ready_core();
    core.advance(Duration::from_millis(1));
    assert!(core.clock_alpha() > 0.0);
    core.set_game_speed(GameSpeed::Fast);
    assert_eq!(core.clock_alpha(), 0.0);
}

#[test]
fn onboarding_pauses_without_catch_up_and_advances_from_real_session_actions() {
    let mut core = core_with_preferences(UserPreferencesV1::default());
    assert_eq!(
        core.onboarding_step(),
        Some(OnboardingStep::SelectUnionWorld)
    );
    assert_eq!(core.game_speed(), GameSpeed::Paused);
    core.advance(Duration::from_secs(10));
    assert_eq!(core.simulation().state().next_tick.0, 0);

    let source = projected_world(&core, WorldId(0));
    let target = projected_world(&core, WorldId(3));
    assert!(core.handle_scene_tap(source, true));
    assert_eq!(core.onboarding_step(), Some(OnboardingStep::PreviewLaunch));
    assert!(core.handle_scene_tap(target, true));
    assert_eq!(core.onboarding_step(), Some(OnboardingStep::LaunchFleet));
    assert_eq!(core.simulation().pending_command_count(), 0);

    assert!(core.handle_ui_action(UiAction::Launch));
    assert_eq!(core.onboarding_step(), Some(OnboardingStep::TuneField));
    assert!(core.handle_action(Action::Increase));
    assert_eq!(core.onboarding_step(), Some(OnboardingStep::ReadAdvisory));
    assert!(core.handle_ui_action(UiAction::AcknowledgeAdvisory));
    assert_eq!(
        core.onboarding_step(),
        Some(OnboardingStep::OpenScenarioEditor)
    );
    assert!(core.handle_action(Action::Scenario));
    assert_eq!(core.mode(), AppMode::EditingScenario);
    assert!(!core.onboarding().active());
    assert!(core.preferences().onboarding_completed);

    core.advance(Duration::from_secs(1));
    assert_eq!(core.simulation().state().next_tick.0, 0);
}

#[test]
fn touch_destination_only_previews_until_the_explicit_launch_control() {
    let mut core = ready_core();
    let source = projected_world(&core, WorldId(0));
    let target = projected_world(&core, WorldId(3));
    core.handle_scene_tap(source, true);
    core.handle_scene_tap(target, true);

    assert_eq!(core.selected_world(), Some(WorldId(0)));
    assert_eq!(core.hovered_world(), Some(WorldId(3)));
    assert!(core.preview_contextual_launch().unwrap().is_ok());
    assert_eq!(core.simulation().pending_command_count(), 0);
    assert!(core.handle_ui_action(UiAction::Launch));
    assert_eq!(core.simulation().pending_command_count(), 1);
}

#[test]
fn mouse_destination_does_not_survive_when_hover_leaves_the_scene() {
    let mut core = ready_core();
    let source = projected_world(&core, WorldId(0));
    let target = projected_world(&core, WorldId(3));
    core.handle_scene_tap(source, false);
    core.set_scene_cursor(target, false);
    assert_eq!(core.command_target(), Some(WorldId(3)));
    assert!(core.preview_contextual_launch().is_some());

    core.set_scene_cursor(Vec2::new(480.0, 580.0), true);
    assert_eq!(core.hovered_world(), None);
    assert_eq!(core.command_target(), None);
    assert!(core.preview_contextual_launch().is_none());
}

#[test]
fn ray_picking_drives_hover_selection_and_right_click_launch_ids() {
    let mut core = ready_core();
    let source = projected_world(&core, WorldId(0));
    let target = projected_world(&core, WorldId(3));

    core.set_scene_cursor(source, false);
    assert_eq!(core.hovered_world(), Some(WorldId(0)));
    core.handle_scene_tap(source, false);
    assert_eq!(core.selected_world(), Some(WorldId(0)));
    assert!(core.handle_scene_launch_pointer(target));
    assert_eq!(core.hovered_world(), Some(WorldId(3)));
    assert_eq!(core.simulation().pending_command_count(), 1);
}

#[test]
fn touch_can_replace_and_clear_its_source_without_keyboard_input() {
    let mut invalid_source = ready_core();
    let enemy = projected_world(&invalid_source, WorldId(1));
    invalid_source.handle_scene_tap(enemy, true);
    assert_eq!(invalid_source.selected_world(), None);

    let mut draft = ScenarioDraft::factory_default();
    draft.worlds[1].owner = Some(Faction::Union);
    let mut core = core_with_draft_and_preferences(
        draft,
        UserPreferencesV1 {
            onboarding_completed: true,
            ..UserPreferencesV1::default()
        },
    );
    let first_source = projected_world(&core, WorldId(0));
    let second_source = projected_world(&core, WorldId(1));
    let empty = Vec2::new(core.viewport().x * 0.5, core.viewport().y - 8.0);

    core.handle_scene_tap(first_source, true);
    assert_eq!(core.selected_world(), Some(WorldId(0)));
    core.handle_scene_tap(second_source, true);
    assert_eq!(core.selected_world(), Some(WorldId(1)));
    core.handle_scene_tap(empty, true);
    assert_eq!(core.selected_world(), None);
    assert_eq!(core.command_target(), None);
}

#[test]
fn scene_interaction_releases_stable_hud_focus() {
    let mut core = ready_core();
    let settings_index = core
        .ui_frame()
        .controls
        .iter()
        .position(|control| control.action == UiAction::OpenSettings)
        .unwrap();
    core.focus_ui(settings_index);
    assert!(core.ui_focus_index().is_some());

    core.handle_action(Action::Help);
    assert_eq!(
        core.ui_frame()
            .controls
            .iter()
            .find(|control| control.focused)
            .map(|control| control.action),
        Some(UiAction::OpenSettings)
    );

    core.handle_scene_tap(projected_world(&core, WorldId(0)), false);
    assert_eq!(core.ui_focus_index(), None);
}

#[test]
fn settings_share_one_action_path_and_layout_has_visible_focus_and_large_targets() {
    let mut core = ready_core();
    assert!(core.handle_action(Action::Settings));
    assert_eq!(core.mode(), AppMode::Settings);
    assert!(core.handle_settings_action(SettingsAction::SetUiScale(UiScale::Percent130)));
    assert!(core.handle_settings_action(SettingsAction::SetMotion(MotionPreference::Reduced)));
    assert!(core.handle_settings_action(SettingsAction::SetHighContrast(true)));
    assert!(core.handle_settings_action(SettingsAction::SetGraphicsQuality(GraphicsQuality::Low)));
    let preferences = core.preferences();
    assert_eq!(preferences.ui_scale, UiScale::Percent130);
    assert_eq!(preferences.motion, MotionPreference::Reduced);
    assert!(preferences.high_contrast);
    assert_eq!(preferences.graphics_quality, GraphicsQuality::Low);
    assert_eq!(core.camera().motion, MotionPreference::Reduced);

    assert!(core.handle_settings_action(SettingsAction::Close));
    core.focus_ui_next(false);

    for viewport in [Vec2::new(960.0, 600.0), Vec2::new(1440.0, 900.0)] {
        core.set_viewport(viewport);
        let frame = core.ui_frame();
        assert_eq!(frame.wide, viewport.x >= 1_100.0);
        assert!(frame.controls.iter().any(|control| control.focused));
        assert!(frame.controls.iter().all(|control| {
            control.bounds.size().x >= MIN_CONTROL_EXTENT
                && control.bounds.size().y >= MIN_CONTROL_EXTENT
        }));
        for (index, control) in frame.controls.iter().enumerate() {
            assert!(control.bounds.min.cmpge(Vec2::ZERO).all());
            assert!(control.bounds.max.cmple(viewport).all());
            for other in frame.controls.iter().skip(index + 1) {
                let separated = control.bounds.max.x <= other.bounds.min.x
                    || other.bounds.max.x <= control.bounds.min.x
                    || control.bounds.max.y <= other.bounds.min.y
                    || other.bounds.max.y <= control.bounds.min.y;
                assert!(separated, "overlapping controls: {control:?} and {other:?}");
            }
        }

        let mut batch = UiBatch::default();
        build_ui_batch(&frame, &AtlasMetrics::embedded().unwrap(), &mut batch).unwrap();
        assert!(!batch.glyphs().is_empty());
        assert!(batch.glyphs().iter().all(|glyph| {
            let [x, y, width, height] = glyph.rect;
            x >= 0.0 && y >= 0.0 && x + width <= viewport.x && y + height <= viewport.y
        }));
    }
}

#[test]
fn every_ui_scale_changes_compact_and_wide_layout_without_shrinking_touch_targets() {
    for viewport in [Vec2::new(960.0, 600.0), Vec2::new(1440.0, 900.0)] {
        let mut previous_scale = 0.0;
        for ui_scale in [
            UiScale::Percent85,
            UiScale::Percent100,
            UiScale::Percent115,
            UiScale::Percent130,
        ] {
            let mut core = core_with_preferences(UserPreferencesV1 {
                ui_scale,
                onboarding_completed: true,
                ..UserPreferencesV1::default()
            });
            core.set_viewport(viewport);
            let frame = core.ui_frame();
            assert!(frame.scale > previous_scale);
            for control in &frame.controls {
                assert!(
                    control.bounds.size().x >= MIN_CONTROL_EXTENT - 0.001
                        && control.bounds.size().y >= MIN_CONTROL_EXTENT - 0.001,
                    "{ui_scale:?} at {viewport:?} produced an undersized control: {control:?}"
                );
            }
            previous_scale = frame.scale;
        }
    }
}

#[test]
fn compact_chrome_consumes_pointer_gaps_and_tiny_layouts_remain_bounded() {
    let mut core = core_with_preferences(UserPreferencesV1 {
        ui_scale: UiScale::Percent85,
        onboarding_completed: false,
        ..UserPreferencesV1::default()
    });
    core.set_viewport(Vec2::new(960.0, 600.0));
    let compact = core.ui_frame();
    assert!(compact.consumes_pointer(Vec2::new(160.0, 300.0)));
    assert!(compact.consumes_pointer(Vec2::new(480.0, 549.0)));

    let tiny = build_ui_frame(UiBuildState {
        viewport: Vec2::new(120.0, 100.0),
        ui_scale: UiScale::Percent85,
        screen: UiScreen::Playing,
        focus_index: None,
        speed_multiplier: 0,
        help_visible: false,
        selected_field: FieldKind::Atmosphere,
        decrease_field: (false, "Unavailable"),
        increase_field: (false, "Unavailable"),
        pointer: None,
        onboarding: Some((OnboardingStep::SelectUnionWorld, 1, 6)),
        preferences: UserPreferencesV1::default(),
        command_tray: &CommandTrayState::default(),
    });
    let mut batch = intergalactic_warfare::ui::UiBatch::default();
    let metrics = intergalactic_warfare::ui::AtlasMetrics::embedded().unwrap();
    build_ui_batch(&tiny, &metrics, &mut batch).unwrap();
    assert!(batch.glyphs().iter().all(|glyph| {
        glyph.rect[0].is_finite()
            && glyph.rect[1].is_finite()
            && glyph.rect[2].is_finite()
            && glyph.rect[3].is_finite()
    }));
}

#[test]
fn field_controls_publish_the_simulation_preview_state() {
    let mut core = ready_core();
    let no_source = core.ui_frame();
    for action in [UiAction::DecreaseField, UiAction::IncreaseField] {
        let control = no_source
            .controls
            .iter()
            .find(|control| control.action == action)
            .unwrap();
        assert!(!control.enabled);
        assert_eq!(control.tooltip, "Select a Union world");
    }

    core.handle_scene_tap(projected_world(&core, WorldId(0)), false);
    let selected = core.ui_frame();
    assert!(
        selected
            .controls
            .iter()
            .filter(|control| matches!(
                control.action,
                UiAction::DecreaseField | UiAction::IncreaseField
            ))
            .any(|control| control.enabled)
    );

    let mut wrong_owner = ready_core();
    wrong_owner.handle_scene_tap(projected_world(&wrong_owner, WorldId(1)), false);
    for action in [UiAction::DecreaseField, UiAction::IncreaseField] {
        let control = wrong_owner
            .ui_frame()
            .controls
            .into_iter()
            .find(|control| control.action == action)
            .unwrap();
        assert!(!control.enabled);
        assert_eq!(control.tooltip, "REJECTED - SOURCE IS NOT UNION");
    }

    let mut at_limit = ScenarioDraft::factory_default();
    at_limit.worlds[0].atmosphere = 10;
    let mut at_limit = core_with_draft_and_preferences(
        at_limit,
        UserPreferencesV1 {
            onboarding_completed: true,
            ..UserPreferencesV1::default()
        },
    );
    at_limit.handle_scene_tap(projected_world(&at_limit, WorldId(0)), false);
    let increase = at_limit
        .ui_frame()
        .controls
        .into_iter()
        .find(|control| control.action == UiAction::IncreaseField)
        .unwrap();
    assert!(!increase.enabled);
    assert_eq!(increase.tooltip, "REJECTED - FIELD AT LIMIT");

    let mut insufficient = ScenarioDraft::factory_default();
    insufficient.worlds[0].energy = 0;
    let mut insufficient = core_with_draft_and_preferences(
        insufficient,
        UserPreferencesV1 {
            onboarding_completed: true,
            ..UserPreferencesV1::default()
        },
    );
    insufficient.handle_scene_tap(projected_world(&insufficient, WorldId(0)), false);
    let increase = insufficient
        .ui_frame()
        .controls
        .into_iter()
        .find(|control| control.action == UiAction::IncreaseField)
        .unwrap();
    assert!(!increase.enabled);
    assert_eq!(increase.tooltip, "REJECTED - INSUFFICIENT ENERGY");
}

#[test]
fn reduced_motion_and_high_contrast_preserve_static_non_color_cues() {
    let core = core_with_preferences(UserPreferencesV1 {
        motion: MotionPreference::Reduced,
        high_contrast: true,
        onboarding_completed: true,
        ..UserPreferencesV1::default()
    });
    let scene = core.scene_frame();
    assert_eq!(core.camera().motion, MotionPreference::Reduced);
    assert_eq!(scene.visual_time, 0.0);
    assert!(scene.background.static_background);
    assert!(!scene.effects.animate_halos);
    assert_eq!(scene.effects.particle_budget, 0);
    assert!(
        scene
            .worlds
            .iter()
            .all(|world| world.ownership_pattern != 0)
    );
}

#[test]
fn gesture_ownership_prevents_drag_launch_and_supports_orbit_and_pinch() {
    let mut controller = InteractionController::default();
    controller.begin(
        1,
        Vec2::new(100.0, 100.0),
        PointerSource::Mouse,
        PointerButton::Secondary,
        InteractionHit::World,
    );
    controller.update(1, Vec2::new(130.0, 100.0));
    assert_eq!(controller.end(1, Vec2::new(130.0, 100.0), None), None);

    controller.begin(
        2,
        Vec2::new(100.0, 100.0),
        PointerSource::Mouse,
        PointerButton::Middle,
        InteractionHit::EmptyScene,
    );
    assert!(matches!(
        controller.update(2, Vec2::new(120.0, 110.0)),
        Some(GestureAction::Orbit { .. })
    ));
    controller.end(2, Vec2::new(120.0, 110.0), None);

    controller.begin(
        10,
        Vec2::new(100.0, 100.0),
        PointerSource::Touch,
        PointerButton::Primary,
        InteractionHit::EmptyScene,
    );
    controller.begin(
        11,
        Vec2::new(200.0, 100.0),
        PointerSource::Touch,
        PointerButton::Primary,
        InteractionHit::EmptyScene,
    );
    assert!(matches!(
        controller.update(11, Vec2::new(220.0, 100.0)),
        Some(GestureAction::Zoom { .. })
    ));
    assert_eq!(controller.end(10, Vec2::new(100.0, 100.0), None), None);
}

#[test]
fn preference_load_failure_is_recoverable_and_cannot_change_campaign_truth() {
    let scenario = ScenarioDraft::factory_default().validated().unwrap();
    let expected = Simulation::from_scenario(&scenario).canonical_fingerprint();
    let mut preferences = MemoryPreferencesStore::default();
    preferences.deny_load("injected denial");
    let core = AppCore::new_with_preferences(scenario, MemoryScenarioStore::default(), preferences);
    assert_eq!(core.preferences(), UserPreferencesV1::default());
    assert!(core.recoverable_message().is_some());
    assert_eq!(core.simulation().canonical_fingerprint(), expected);
    assert_eq!(core.simulation().pending_command_count(), 0);
}

#[test]
fn preference_write_failure_keeps_the_session_usable_and_scenario_unchanged() {
    let scenario = ScenarioDraft::factory_default().validated().unwrap();
    let expected = Simulation::from_scenario(&scenario).canonical_fingerprint();
    let mut preferences = MemoryPreferencesStore::default();
    preferences.fail_next_save("injected quota failure");
    let mut core =
        AppCore::new_with_preferences(scenario, MemoryScenarioStore::default(), preferences);

    core.skip_onboarding();
    assert!(!core.onboarding().active());
    assert!(core.preferences().onboarding_completed);
    assert!(
        core.recoverable_message()
            .is_some_and(|message| message.contains("injected quota failure"))
    );
    assert_eq!(core.simulation().canonical_fingerprint(), expected);
    assert_eq!(core.simulation().pending_command_count(), 0);
}

#[test]
fn editor_and_ui_preview_remain_command_isolated() {
    let mut core = ready_core();
    let before = core.simulation().canonical_fingerprint();
    let _ = core.ui_frame();
    core.open_editor();
    assert!(!core.queue_game_command(GameCommand::TuneField {
        world: WorldId(0),
        field: FieldKind::Atmosphere,
        adjustment: intergalactic_warfare::game::model::FieldAdjustment::Increase,
    }));
    assert_eq!(core.simulation().canonical_fingerprint(), before);
    assert_eq!(core.simulation().pending_command_count(), 0);
}
