use glam::Vec2;
use intergalactic_warfare::editor::{
    ConfirmationAction, ConfirmedEditorAction, EditorAction, EditorLayoutKind, EditorSection,
    EditorState, WidgetKind, build_frame,
};
use intergalactic_warfare::engine::input::NavigationAction;
use intergalactic_warfare::engine::primitives::PrimitiveBatch;
use intergalactic_warfare::scenario::ScenarioDraft;
use intergalactic_warfare::scenario::store::{MemoryScenarioStore, ScenarioStore};

fn edit_first_world_name(editor: &mut EditorState, value: &str) {
    editor.select_world(0);
    editor.focus_widget("world.0.name").unwrap();
    editor.replace_focused_text(value);
}

#[test]
fn wide_compact_and_tiny_layouts_are_bounded_and_touch_sized() {
    let mut editor = EditorState::new(ScenarioDraft::factory_default());
    for (viewport, expected) in [
        (Vec2::new(1440.0, 900.0), EditorLayoutKind::Wide),
        (Vec2::new(960.0, 600.0), EditorLayoutKind::Compact),
        (Vec2::new(320.0, 220.0), EditorLayoutKind::Compact),
    ] {
        editor.rebuild_widgets(viewport);
        assert_eq!(editor.layout_kind(), expected);
        assert!(editor.widgets().iter().all(|widget| {
            widget.rect.width >= 0.0
                && widget.rect.height >= 0.0
                && (!widget.focusable
                    || widget.rect.width >= 44.0_f32.min(viewport.x)
                        && widget.rect.height >= 44.0_f32.min(viewport.y))
        }));
    }
}

#[test]
fn wide_layout_has_navigation_preview_form_and_footer_while_compact_uses_tabs() {
    let mut editor = EditorState::new(ScenarioDraft::factory_default());
    editor.rebuild_widgets(Vec2::new(1440.0, 900.0));
    assert_eq!(editor.layout_kind(), EditorLayoutKind::Wide);
    assert!(
        editor
            .widgets()
            .iter()
            .any(|widget| widget.id == "section.worlds")
    );
    assert!(
        editor
            .widgets()
            .iter()
            .any(|widget| widget.id == "world.select.6")
    );
    let preview = editor
        .widgets()
        .iter()
        .find(|widget| widget.id == "preview.world")
        .unwrap();
    let form = editor
        .widgets()
        .iter()
        .find(|widget| widget.id == "world.0.name")
        .unwrap();
    assert!(preview.rect.x >= 224.0);
    assert!(preview.rect.x + preview.rect.width < form.rect.x);
    assert!((400.0..=440.0).contains(&form.rect.width));
    assert!(editor.widgets().iter().any(|widget| {
        widget.kind == WidgetKind::Action(EditorAction::ApplyAndRestart)
            && widget.rect.y + widget.rect.height <= 900.0
    }));
    for section in [EditorSection::Rules, EditorSection::Validation] {
        editor.select_section(section);
        assert!(
            editor
                .widgets()
                .iter()
                .any(|widget| { widget.id == "preview.world" && widget.rect.width > 0.0 })
        );
    }

    editor.select_section(EditorSection::Worlds);
    editor.rebuild_widgets(Vec2::new(960.0, 600.0));
    assert_eq!(editor.layout_kind(), EditorLayoutKind::Compact);
    for section in [
        EditorSection::Worlds,
        EditorSection::Rules,
        EditorSection::Validation,
    ] {
        assert!(editor.widgets().iter().any(|widget| {
            widget.kind == WidgetKind::Action(EditorAction::SelectSection(section))
        }));
    }
    let compact_form = editor
        .widgets()
        .iter()
        .find(|widget| widget.id == "world.0.name")
        .unwrap();
    assert!(compact_form.rect.width <= 928.0);
}

#[test]
fn widgets_are_the_shared_focus_hit_test_and_culling_source() {
    let mut editor = EditorState::new(ScenarioDraft::factory_default());
    editor.rebuild_widgets(Vec2::new(960.0, 600.0));
    assert!(editor.widgets().iter().any(|widget| !widget.visible));

    editor.focus_first();
    let first = editor.focused_widget_id().unwrap().to_owned();
    editor.focus_next(false);
    let second = editor.focused_widget_id().unwrap().to_owned();
    assert_ne!(first, second);
    editor.focus_next(true);
    assert_eq!(editor.focused_widget_id(), Some(first.as_str()));

    let widget = editor
        .widgets()
        .iter()
        .find(|widget| widget.visible && widget.focusable)
        .unwrap();
    let center = widget.rect.center();
    assert_eq!(editor.hit_test(center), Some(widget.id.clone()));
}

#[test]
fn text_editing_retains_invalid_numeric_input_and_disables_apply() {
    let mut editor = EditorState::new(ScenarioDraft::factory_default());
    editor.rebuild_widgets(Vec2::new(1440.0, 900.0));
    editor.select_section(EditorSection::Rules);
    editor.focus_widget("rule.maximum_energy").unwrap();
    editor.replace_focused_text("NOT A NUMBER");
    editor.commit_ime("7");
    editor.set_ime_preedit("50");
    assert_eq!(editor.ime_preedit(), "50");
    assert_eq!(editor.focused_text(), Some("NOT A NUMBER7"));
    assert!(!editor.can_apply());
    assert!(
        editor
            .request_save(&mut MemoryScenarioStore::default())
            .is_err()
    );
    assert!(editor.widgets().iter().any(|widget| {
        widget.kind == WidgetKind::Action(EditorAction::ApplyAndRestart) && !widget.enabled
    }));
}

#[test]
fn focused_text_widgets_accept_spaces_before_control_activation() {
    let mut editor = EditorState::new(ScenarioDraft::factory_default());
    editor.rebuild_widgets(Vec2::new(1440.0, 900.0));
    editor.focus_widget("world.0.name").unwrap();
    assert!(editor.focused_accepts_text());

    editor.replace_focused_text("ASTER");
    editor.insert_focused_text(" ");
    editor.insert_focused_text("PRIME");

    assert_eq!(editor.draft().worlds[0].name, "ASTER PRIME");
    assert!(editor.validation_report().is_valid());
}

#[test]
fn pointer_and_keyboard_activation_produce_the_same_editor_action() {
    let mut editor = EditorState::new(ScenarioDraft::factory_default());
    editor.rebuild_widgets(Vec2::new(1440.0, 900.0));
    let save = editor
        .widgets()
        .iter()
        .find(|widget| widget.kind == WidgetKind::Action(EditorAction::Save))
        .unwrap()
        .clone();
    editor.focus_widget(&save.id).unwrap();
    assert_eq!(editor.activate_focused(), Some(EditorAction::Save));
    assert_eq!(
        editor.activate_at(save.rect.center()),
        Some(EditorAction::Save)
    );
}

#[test]
fn occupied_save_requires_confirmation_and_cancel_preserves_everything() {
    let mut editor = EditorState::new(ScenarioDraft::factory_default());
    edit_first_world_name(&mut editor, "ASTER PRIME");
    let before = editor.draft().clone();
    let mut store = MemoryScenarioStore::with_slot("old payload");

    assert!(!editor.request_save(&mut store).unwrap());
    assert_eq!(
        editor.confirmation(),
        Some(ConfirmationAction::OverwriteSave)
    );
    editor.cancel_confirmation();
    assert_eq!(store.load().unwrap().as_deref(), Some("old payload"));
    assert_eq!(editor.draft(), &before);

    assert!(!editor.request_save(&mut store).unwrap());
    let _ = editor.confirm(&mut store).unwrap();
    assert_ne!(store.load().unwrap().as_deref(), Some("old payload"));
}

#[test]
fn save_failures_are_visible_and_preserve_the_detached_draft() {
    let mut editor = EditorState::new(ScenarioDraft::factory_default());
    edit_first_world_name(&mut editor, "ASTER PRIME");
    let before = editor.draft().clone();

    let mut empty_store = MemoryScenarioStore::default();
    empty_store.fail_next_save("empty slot failure");
    assert!(editor.request_save(&mut empty_store).is_err());
    assert_eq!(editor.draft(), &before);
    assert!(editor.widgets().iter().any(|widget| {
        widget.kind == WidgetKind::Status && widget.value.contains("EMPTY SLOT FAILURE")
    }));

    let mut occupied_store = MemoryScenarioStore::with_slot("old payload");
    assert!(!editor.request_save(&mut occupied_store).unwrap());
    occupied_store.fail_next_save("overwrite failure");
    assert!(editor.confirm(&mut occupied_store).is_err());
    assert_eq!(
        occupied_store.load().unwrap().as_deref(),
        Some("old payload")
    );
    assert_eq!(editor.draft(), &before);
    assert!(editor.widgets().iter().any(|widget| {
        widget.kind == WidgetKind::Status && widget.value.contains("OVERWRITE FAILURE")
    }));
}

#[test]
fn empty_slot_save_needs_no_confirmation_and_apply_stays_detached() {
    let active = ScenarioDraft::factory_default();
    let mut editor = EditorState::new(active.clone());
    edit_first_world_name(&mut editor, "ASTER PRIME");
    let mut store = MemoryScenarioStore::default();
    assert!(editor.request_save(&mut store).unwrap());
    assert_eq!(editor.confirmation(), None);
    assert_eq!(editor.original(), &active);
    assert_ne!(editor.draft(), editor.original());
    assert!(editor.request_apply_and_restart().is_ok());
    editor.cancel_confirmation();
}

#[test]
fn navigation_reveals_focus_and_scrolls_with_clamped_page_and_wheel_input() {
    let mut editor = EditorState::new(ScenarioDraft::factory_default());
    editor.rebuild_widgets(Vec2::new(960.0, 600.0));
    assert!(editor.maximum_scroll() > 0.0);
    editor.navigate(NavigationAction::End);
    assert_eq!(editor.scroll(), editor.maximum_scroll());
    editor.scroll_by(f32::INFINITY);
    assert_eq!(editor.scroll(), editor.maximum_scroll());
    editor.navigate(NavigationAction::Home);
    assert_eq!(editor.scroll(), 0.0);
    editor.select_world(6);
    editor.focus_widget("world.6.topology").unwrap();
    assert!(editor.scroll() > 0.0);
    assert!(
        editor
            .widgets()
            .iter()
            .find(|widget| widget.id == "world.6.topology")
            .unwrap()
            .visible
    );
}

#[test]
fn upward_focus_reveal_and_reverse_wrap_never_leave_focus_clipped() {
    let mut editor = EditorState::new(ScenarioDraft::factory_default());
    editor.rebuild_widgets(Vec2::new(960.0, 600.0));
    editor.navigate(NavigationAction::End);
    assert_eq!(editor.scroll(), editor.maximum_scroll());
    editor.focus_widget("world.0.name").unwrap();
    let first = editor
        .widgets()
        .iter()
        .find(|widget| widget.id == "world.0.name")
        .unwrap();
    assert!(first.visible);
    assert!(first.rect.y >= editor.content_rect().y);
    assert!(
        first.rect.y + first.rect.height <= editor.content_rect().y + editor.content_rect().height
    );

    editor.focus_first();
    editor.focus_next(true);
    let focused = editor.focused_widget_id().unwrap();
    let widget = editor
        .widgets()
        .iter()
        .find(|widget| widget.id == focused)
        .unwrap();
    assert!(widget.visible);
}

#[test]
fn frame_visibly_emits_values_focus_errors_confirmations_and_store_status() {
    let mut editor = EditorState::new(ScenarioDraft::factory_default());
    editor.rebuild_widgets(Vec2::new(960.0, 600.0));
    editor.focus_widget("world.0.energy").unwrap();
    editor.replace_focused_text("INVALID");
    editor.set_ime_preedit("7");
    let mut batch = PrimitiveBatch::default();
    build_frame(&editor, &mut batch);
    assert!(!batch.vertices().is_empty());
    let invalid = editor
        .widgets()
        .iter()
        .find(|widget| widget.id == "world.0.energy")
        .unwrap();
    assert_eq!(invalid.value, "INVALID7");
    assert!(invalid.error.is_some());
    assert_eq!(editor.focused_widget_id(), Some("world.0.energy"));

    editor.request_cancel();
    assert!(editor.widgets().iter().any(|widget| {
        widget.kind == WidgetKind::Confirmation && widget.value.contains("DISCARD")
    }));

    let mut clean_editor = EditorState::new(ScenarioDraft::factory_default());
    let empty = MemoryScenarioStore::default();
    assert!(!clean_editor.request_load(&empty).unwrap());
    assert!(clean_editor.widgets().iter().any(|widget| {
        widget.kind == WidgetKind::Status && widget.value == "NO SAVED SCENARIO"
    }));
}

#[test]
fn invalid_text_and_edits_after_request_cannot_yield_a_stale_apply_payload() {
    let mut editor = EditorState::new(ScenarioDraft::factory_default());
    editor.rebuild_widgets(Vec2::new(960.0, 600.0));
    editor.focus_widget("world.0.energy").unwrap();
    editor.replace_focused_text("INVALID");
    assert!(editor.request_apply_and_restart().is_err());

    editor.replace_focused_text("80000");
    editor.request_apply_and_restart().unwrap();
    assert_eq!(
        editor.confirmation(),
        Some(ConfirmationAction::ApplyAndRestart)
    );
    editor.replace_focused_text("99999999999999999999");
    let mut store = MemoryScenarioStore::default();
    assert!(editor.confirm(&mut store).is_err());
    assert!(editor.confirmation().is_none());
}

#[test]
fn confirmed_apply_returns_the_validated_scenario_not_an_unguarded_action() {
    let mut editor = EditorState::new(ScenarioDraft::factory_default());
    editor.request_apply_and_restart().unwrap();
    let mut store = MemoryScenarioStore::default();
    let outcome = editor.confirm(&mut store).unwrap().unwrap();
    match outcome {
        ConfirmedEditorAction::ApplyAndRestart(scenario) => {
            assert_eq!(
                scenario.fingerprint(),
                editor.draft().validated().unwrap().fingerprint()
            );
        }
        other => panic!("unexpected confirmation outcome: {other:?}"),
    }
}

#[test]
fn destructive_controls_require_confirmation_and_failed_load_preserves_draft() {
    let mut editor = EditorState::new(ScenarioDraft::factory_default());
    edit_first_world_name(&mut editor, "ASTER PRIME");
    let before = editor.draft().clone();
    let mut corrupt = MemoryScenarioStore::with_slot("not json");
    assert!(!editor.request_load(&corrupt).unwrap());
    assert_eq!(editor.confirmation(), Some(ConfirmationAction::LoadDirty));
    assert!(editor.confirm(&mut corrupt).is_err());
    assert_eq!(editor.draft(), &before);

    editor.request_revert();
    assert_eq!(editor.confirmation(), Some(ConfirmationAction::Revert));
    editor.cancel_confirmation();
    editor.request_factory_defaults();
    assert_eq!(
        editor.confirmation(),
        Some(ConfirmationAction::FactoryDefaults)
    );
    editor.cancel_confirmation();
    editor.request_regenerate_from_seed();
    assert_eq!(
        editor.confirmation(),
        Some(ConfirmationAction::RegenerateFromSeed)
    );
    editor.cancel_confirmation();
    assert!(!editor.request_cancel());
    assert_eq!(editor.confirmation(), Some(ConfirmationAction::CancelDirty));
    editor.cancel_confirmation();
    editor.request_apply_and_restart().unwrap();
    assert_eq!(
        editor.confirmation(),
        Some(ConfirmationAction::ApplyAndRestart)
    );
    assert_eq!(
        editor.confirm(&mut corrupt).unwrap(),
        Some(ConfirmedEditorAction::ApplyAndRestart(Box::new(
            editor.draft().validated().unwrap()
        )))
    );
    assert_eq!(editor.draft(), &before);
}
