mod common;
use common::*;

use glam::Vec2;
use nyon::engine::backend::BackendKind;
use nyon::ui::AtlasMetrics;
use nyon::ui::accessibility::AnnouncementKind;
use nyon::ui::accessibility::FocusManager;
use nyon::ui::accessibility::InputModality;
use nyon::ui::accessibility::SemanticRole;
use nyon::ui::creator::CreatorDraft;
use nyon::ui::creator::CreatorFieldKind;
use nyon::ui::platform::build_workshop_platform_frame;
use nyon::ui::platform::build_workshop_platform_frame_for_view;
use nyon::ui::platform_sdf::build_platform_ui_batch;
use nyon::ui::workshop::CreatorTool;
use nyon::ui::workshop::WorkshopUiContext;
use nyon::ui::workshop::WorkshopUiIntent;
use nyon::ui::workshop::WorkshopUiModel;
use nyon::ui::workshop::default_creator_batch;
use nyon::ui::workshop::removal_blockers;
use nyon::ui::workshop_layout::WorkshopLayout;
use nyon::ui::workshop_view::WorkshopViewState;
use nyon::workshop::BatchLocalId;
use nyon::workshop::CatalogHash;
use nyon::workshop::CreatorOpV1;
use nyon::workshop::GalaxyPointV1;
use nyon::workshop::ObjectRefV1;
use nyon::workshop::WorkshopTick;
use nyon::workshop::session::WorkshopDiagnostic;
use nyon::workshop::session::WorkshopDiagnosticCode;
use nyon::workshop::session::WorkshopSpeed;
use nyon_workshop_core::model::SystemV1;
use std::collections::BTreeMap;
use std::collections::BTreeSet;

#[test]
fn accepted_branch_history_and_overflow_timeline_actions_are_reachable() {
    use nyon_workshop_core::{CreatorBatchV1, WorkshopHistory};
    let mut history = WorkshopHistory::from_seed_u64(core_catalog(), 46);
    for index in 0..24 {
        history
            .submit(CreatorBatchV1 {
                expected_cursor: None,
                expected_tick: WorkshopTick(0),
                operations: vec![CreatorOpV1::CreateSystem {
                    local: BatchLocalId(0),
                    name: name(&format!("Alternative {index:02}")),
                    position: GalaxyPointV1::new(0, 0).unwrap(),
                }],
            })
            .unwrap();
        history.undo().unwrap();
    }
    let redo = history.redo_choices();
    assert_eq!(redo.len(), 24);
    let session = nyon::workshop::session::WorkshopSession::new(history);
    let model = WorkshopUiModel::build(
        session.snapshot(),
        WorkshopUiContext {
            redo_children: &redo,
            ..WorkshopUiContext::default()
        },
    );
    assert!(model.branches.len() >= 24);
    for (width, height, scale) in SDF_QUALIFICATION_MATRIX {
        let layout = WorkshopLayout::resolve(Vec2::new(width, height), scale).unwrap();
        let mut view = WorkshopViewState::default();
        for target in model
            .branches
            .iter()
            .map(|branch| &branch.control)
            .chain(&model.timeline.controls)
        {
            assert!(view.reveal_action(&model, &layout, &target.action_id));
            let frame = build_workshop_platform_frame_for_view(
                &model,
                layout.clone(),
                &view,
                Some(&target.action_id),
            );
            qualify_sdf_frame(&model, &frame);
            let control = frame
                .controls
                .iter()
                .find(|control| control.action_id == target.action_id)
                .unwrap_or_else(|| {
                    panic!(
                        "{} not revealed at {width}x{height}@{scale}",
                        target.action_id
                    )
                });
            if control.enabled {
                assert_eq!(
                    frame.hit_test(control.bounds.center()).unwrap().action_id,
                    target.action_id
                );
                assert_eq!(
                    model.activate(&target.action_id, InputModality::Keyboard),
                    model.activate(&target.action_id, InputModality::Pointer)
                );
            }
        }
    }
}

#[test]
fn every_creator_operation_has_a_non_spatial_semantic_tool() {
    let model = WorkshopUiModel::build(&snapshot(), WorkshopUiContext::default());
    let mapped = operation_examples()
        .iter()
        .map(CreatorTool::for_operation)
        .collect::<BTreeSet<_>>();

    assert_eq!(mapped, CreatorTool::ALL.into_iter().collect());
    assert_eq!(model.tool_palette.len(), CreatorTool::ALL.len());
    for tool in CreatorTool::ALL {
        let control = &model
            .tool_palette
            .iter()
            .find(|candidate| candidate.tool == tool)
            .unwrap()
            .control;
        assert!(control.enabled);
        assert!(model.semantics.nodes_depth_first().iter().any(|node| {
            node.role == SemanticRole::Button && node.action_id.as_ref() == Some(&control.action_id)
        }));
        assert_eq!(
            model.activate(&control.action_id, InputModality::Keyboard),
            Some(WorkshopUiIntent::OpenCreatorForm {
                tool,
                subject: None,
            })
        );
    }
}

#[test]
fn pointer_and_keyboard_activation_produce_identical_typed_intents() {
    let model = WorkshopUiModel::build(
        &snapshot(),
        WorkshopUiContext {
            selected_entity: Some(entity(4)),
            redo_children: &[revision(9)],
            pending_removal: Some(entity(12)),
            ..WorkshopUiContext::default()
        },
    );

    for control in model.controls() {
        assert_eq!(
            model.activate(&control.action_id, InputModality::Pointer),
            model.activate(&control.action_id, InputModality::Keyboard),
            "input paths diverged for {}",
            control.action_id
        );
    }
    let focus = FocusManager::new(model.focus_order().iter().cloned());
    assert_eq!(focus.base_order(), model.focus_order());
    assert_eq!(
        model.focus_order().iter().collect::<BTreeSet<_>>().len(),
        model.focus_order().len()
    );
}

#[test]
fn every_creator_form_exposes_a_concrete_typed_batch_or_fixed_prerequisite() {
    let mut current = snapshot();
    current.state.systems.insert(
        entity(14),
        SystemV1 {
            name: name("Quench"),
            position: GalaxyPointV1::new(2_048, 1_024).unwrap(),
        },
    );
    current.state_digest = current.state.digest();
    current.active_view.digest = current.state_digest;

    let selections = BTreeMap::from([
        (CreatorTool::CreateStar, entity(2)),
        (CreatorTool::CreateWorld, entity(3)),
        (CreatorTool::ConnectLane, entity(14)),
        (CreatorTool::CreateDeposit, entity(10)),
        (CreatorTool::SetOwner, entity(10)),
        (CreatorTool::PlaceIndustry, entity(10)),
        (CreatorTool::ConnectRoute, entity(4)),
        (CreatorTool::SetIndustryEnabled, entity(6)),
        (CreatorTool::RenameObject, entity(4)),
        (CreatorTool::RemoveObject, entity(12)),
        (CreatorTool::ScheduleHazard, entity(7)),
        (CreatorTool::CancelHazard, entity(12)),
    ]);

    for tool in CreatorTool::ALL {
        let selected = selections.get(&tool).copied();
        let batch = default_creator_batch(&current, selected, tool)
            .unwrap_or_else(|error| panic!("{tool:?} did not produce a batch: {error}"));
        assert_eq!(batch.expected_cursor, current.active_view.view_cursor);
        assert_eq!(batch.expected_tick, current.active_view.tick);
        assert_eq!(batch.operations.len(), 1);
        assert_eq!(CreatorTool::for_operation(&batch.operations[0]), tool);

        let model = WorkshopUiModel::build(
            &current,
            WorkshopUiContext {
                selected_entity: selected,
                creator_form: Some(tool),
                ..WorkshopUiContext::default()
            },
        );
        let form = model.creator_form.as_ref().unwrap();
        assert!(form.submit_control.enabled, "{tool:?}");
        let keyboard = model.activate(&form.submit_control.action_id, InputModality::Keyboard);
        let pointer = model.activate(&form.submit_control.action_id, InputModality::Pointer);
        assert_eq!(keyboard, pointer);
        let Some(WorkshopUiIntent::Dispatch(nyon::workshop::session::WorkshopAction::Submit(
            submitted,
        ))) = keyboard
        else {
            panic!("{tool:?} form did not submit a typed creator batch");
        };
        assert_eq!(submitted, batch);
        assert!(model.semantics.node("workshop.creator-dialog").is_some());
    }
}

#[test]
fn creator_form_focus_controls_are_unique_and_menu_exit_is_typed() {
    let model = WorkshopUiModel::build(
        &snapshot(),
        WorkshopUiContext {
            selected_entity: Some(entity(4)),
            creator_form: Some(CreatorTool::CreateDeposit),
            ..WorkshopUiContext::default()
        },
    );
    assert_eq!(
        model.focus_order().iter().collect::<BTreeSet<_>>().len(),
        model.focus_order().len()
    );
    assert!(model.focus_order().contains(&model.menu_control.action_id));
    assert!(
        model.focus_order().contains(
            &model
                .creator_form
                .as_ref()
                .unwrap()
                .cancel_control
                .action_id
        )
    );
    assert!(
        model.focus_order().contains(
            &model
                .creator_form
                .as_ref()
                .unwrap()
                .submit_control
                .action_id
        )
    );
    assert_eq!(
        model.activate(&model.menu_control.action_id, InputModality::Keyboard),
        Some(WorkshopUiIntent::ReturnToMainMenu)
    );
}

#[test]
fn undo_and_redo_disable_while_running_or_during_store_replacement() {
    let mut running = snapshot();
    running.speed = WorkshopSpeed::Four;
    let running_model = WorkshopUiModel::build(
        &running,
        WorkshopUiContext {
            redo_children: &[revision(9)],
            ..WorkshopUiContext::default()
        },
    );
    assert!(!control(&running_model, "timeline.undo").enabled);
    assert!(
        !control(
            &running_model,
            &format!("timeline.redo.{}", "09".repeat(32))
        )
        .enabled
    );

    let mut replacing = snapshot();
    replacing.store.commit_pending = true;
    let replacing_model = WorkshopUiModel::build(
        &replacing,
        WorkshopUiContext {
            redo_children: &[revision(9)],
            ..WorkshopUiContext::default()
        },
    );
    assert!(!control(&replacing_model, "timeline.undo").enabled);
    assert!(
        !control(
            &replacing_model,
            &format!("timeline.redo.{}", "09".repeat(32))
        )
        .enabled
    );
    assert_eq!(
        replacing_model.activate(
            &control(&replacing_model, "timeline.undo").action_id,
            InputModality::Keyboard,
        ),
        None
    );
}

#[test]
fn branch_chooser_distinguishes_the_immutable_head_from_the_view_cursor() {
    let mut snapshot = snapshot();
    snapshot.active_view.view_cursor = Some(revision(1));
    let model = WorkshopUiModel::build(
        &snapshot,
        WorkshopUiContext {
            redo_children: &[revision(2)],
            ..WorkshopUiContext::default()
        },
    );
    let main = model
        .branches
        .iter()
        .find(|choice| choice.selected)
        .unwrap();

    assert_eq!(main.head, Some(revision(2)));
    assert_eq!(main.selected_cursor, Some(revision(1)));
    assert!(main.browsing_history);
    assert!(main.control.description.contains("behind head"));
    assert_eq!(model.timeline.redo_children, vec![revision(2)]);
}

#[test]
fn removal_confirmation_lists_ordered_blockers_and_never_cascades() {
    let state = populated_state();
    assert_eq!(
        removal_blockers(&state, entity(2)),
        vec![entity(3), entity(4), entity(7)]
    );
    let blocked = WorkshopUiModel::build(
        &snapshot(),
        WorkshopUiContext {
            selected_entity: Some(entity(2)),
            pending_removal: Some(entity(2)),
            ..WorkshopUiContext::default()
        },
    );
    let dialog = blocked.removal_confirmation.as_ref().unwrap();
    assert_eq!(
        dialog
            .blockers
            .iter()
            .map(|blocker| blocker.entity)
            .collect::<Vec<_>>(),
        vec![entity(3), entity(4), entity(7)]
    );
    assert!(!dialog.confirm_control.enabled);
    assert_eq!(
        blocked.activate(&dialog.confirm_control.action_id, InputModality::Keyboard),
        None
    );

    let removable = WorkshopUiModel::build(
        &snapshot(),
        WorkshopUiContext {
            selected_entity: Some(entity(12)),
            pending_removal: Some(entity(12)),
            ..WorkshopUiContext::default()
        },
    );
    let dialog = removable.removal_confirmation.as_ref().unwrap();
    assert!(dialog.blockers.is_empty());
    assert!(dialog.confirm_control.enabled);
    let Some(WorkshopUiIntent::Dispatch(nyon::workshop::session::WorkshopAction::Submit(batch))) =
        removable.activate(&dialog.confirm_control.action_id, InputModality::Keyboard)
    else {
        panic!("removal confirmation did not create a typed creator batch");
    };
    assert_eq!(batch.expected_cursor, Some(revision(2)));
    assert_eq!(batch.expected_tick, WorkshopTick(15));
    assert_eq!(
        batch.operations,
        vec![CreatorOpV1::RemoveObject {
            target: ObjectRefV1::Existing(entity(12)),
        }]
    );
}

#[test]
fn backend_catalog_save_and_accessibility_preferences_are_diagnostic_only() {
    let model = WorkshopUiModel::build(
        &snapshot(),
        WorkshopUiContext {
            backend: Some(BackendKind::WebGl2Low),
            catalog_hash: Some(CatalogHash([0xAB; 32])),
            reduced_motion: true,
            high_contrast: true,
            ..WorkshopUiContext::default()
        },
    );

    assert_eq!(model.diagnostics.backend, "WEBGL2 LOW");
    assert_eq!(model.diagnostics.catalog_hash, "ab".repeat(32));
    assert_eq!(model.save.slot, Some(4));
    assert_eq!(model.save.generation, Some(7));
    assert!(model.save.dirty);
    assert!(model.preferences.reduced_motion);
    assert!(model.preferences.high_contrast);
    assert!(model.preferences.reduced_motion_control.selected);
    assert!(model.preferences.high_contrast_control.selected);
    assert_eq!(
        model.activate(
            &model.preferences.reduced_motion_control.action_id,
            InputModality::Keyboard,
        ),
        Some(WorkshopUiIntent::SetReducedMotion(false))
    );
    assert_eq!(
        model
            .semantics
            .node("diagnostics.backend")
            .unwrap()
            .description,
        "WEBGL2 LOW"
    );
    assert_eq!(
        model
            .semantics
            .node("diagnostics.catalog")
            .unwrap()
            .description,
        "ab".repeat(32)
    );
}

#[test]
fn status_announcements_use_codes_without_echoing_untrusted_error_content() {
    const RAW: &str = "UNTRUSTED IMPORTED NAME <script>alert(1)</script>";
    let mut snapshot = snapshot();
    snapshot.diagnostics = vec![
        WorkshopDiagnostic {
            code: WorkshopDiagnosticCode::ArchiveRejected,
            message: RAW.to_owned(),
        },
        WorkshopDiagnostic {
            code: WorkshopDiagnosticCode::StoreRejected,
            message: RAW.to_owned(),
        },
    ];
    snapshot.store.status = RAW.to_owned();
    let model = WorkshopUiModel::build(&snapshot, WorkshopUiContext::default());

    assert!(
        model
            .semantics
            .announcements
            .iter()
            .all(|announcement| !announcement.message.contains(RAW))
    );
    assert!(model.semantics.announcements.iter().any(|announcement| {
        announcement.kind == AnnouncementKind::Error
            && announcement.code == "archive-rejected"
            && announcement.message == "The Workshop archive could not be validated."
    }));
    assert!(model.semantics.announcements.iter().any(|announcement| {
        announcement.kind == AnnouncementKind::Status
            && announcement.code == "workshop-store-status"
            && announcement.message == "Workshop storage status changed"
    }));
    assert_eq!(model.save.status, "Workshop storage status changed");
}

#[test]
fn creator_dialog_disables_pointer_controls_outside_its_modal_scope() {
    let model = WorkshopUiModel::build(
        &snapshot(),
        WorkshopUiContext {
            creator_form: Some(CreatorTool::CreateSystem),
            ..WorkshopUiContext::default()
        },
    );
    let frame = build_workshop_platform_frame(&model, Vec2::new(1_280.0, 720.0), None);
    let enabled = frame
        .controls
        .iter()
        .filter(|control| control.enabled)
        .map(|control| control.action_id.as_str())
        .collect::<BTreeSet<_>>();
    assert_eq!(
        enabled,
        BTreeSet::from([
            "creator.cancel",
            "creator.field.name",
            "creator.field.x",
            "creator.field.y",
            "creator.submit",
        ])
    );

    let menu = frame
        .controls
        .iter()
        .find(|control| control.action_id.as_str() == "workshop.main-menu")
        .unwrap();
    assert!(!menu.enabled);
    assert!(
        frame
            .hit_test((menu.bounds.min + menu.bounds.max) * 0.5)
            .is_none()
    );
    assert_eq!(
        frame
            .focus_order()
            .iter()
            .map(|action| action.as_str())
            .collect::<Vec<_>>(),
        vec![
            "creator.field.name",
            "creator.field.x",
            "creator.field.y",
            "creator.cancel",
            "creator.submit",
        ]
    );
    assert!(frame.action(&menu.action_id).is_none());
    let dialog = frame
        .semantics
        .nodes_depth_first()
        .into_iter()
        .find(|node| node.role == SemanticRole::Dialog)
        .unwrap();
    fn subtree_ids(
        node: &nyon::ui::accessibility::SemanticNode,
        ids: &mut BTreeSet<nyon::ui::accessibility::SemanticNodeId>,
    ) {
        ids.insert(node.id.clone());
        for child in &node.children {
            subtree_ids(child, ids);
        }
    }
    let mut modal_ids = BTreeSet::new();
    subtree_ids(dialog, &mut modal_ids);
    let records = frame
        .visible_nodes
        .iter()
        .map(|record| record.semantic_id.clone())
        .collect::<BTreeSet<_>>();
    assert!(
        records.is_subset(&modal_ids),
        "nonmodal visible records: {:?}",
        records.difference(&modal_ids).collect::<Vec<_>>()
    );
    let visible = frame
        .semantics
        .nodes_depth_first()
        .into_iter()
        .filter(|node| {
            node.visible
                && node.role != SemanticRole::Application
                && node.role != SemanticRole::Group
                && node.role != SemanticRole::Dialog
        })
        .map(|node| node.id.clone())
        .collect::<BTreeSet<_>>();
    assert_eq!(records, visible);
    let output = build_platform_ui_batch(&frame, &AtlasMetrics::embedded().unwrap()).unwrap();
    let rendered = output
        .text_runs
        .iter()
        .map(|run| run.semantic_id.clone())
        .chain(output.icon_runs.iter().map(|run| run.semantic_id.clone()))
        .collect::<BTreeSet<_>>();
    assert_eq!(records, rendered);
    for control in frame.controls.iter().filter(|control| control.enabled) {
        assert!(records.contains(&control.semantic_id));
        assert!(modal_ids.contains(&control.semantic_id));
    }
}

#[test]
fn creator_draft_edits_are_typed_client_state_until_submit() {
    let current = snapshot();
    let defaults = default_creator_batch(&current, None, CreatorTool::CreateSystem).unwrap();
    let mut draft = CreatorDraft::from_batch(&current.state, &defaults).unwrap();
    assert_eq!(draft.field("name").unwrap().kind, CreatorFieldKind::Text);
    assert_eq!(draft.field("x").unwrap().kind, CreatorFieldKind::Integer);

    draft.set_text("name", "Ember Reach").unwrap();
    draft.set_text("x", "32768").unwrap();
    draft.set_text("y", "-4096").unwrap();
    let submitted = draft.to_batch().unwrap();

    assert_eq!(
        current.state,
        snapshot().state,
        "draft editing mutated authority"
    );
    assert_eq!(submitted.expected_cursor, defaults.expected_cursor);
    assert_eq!(submitted.expected_tick, defaults.expected_tick);
    assert!(matches!(
        &submitted.operations[0],
        CreatorOpV1::CreateSystem { name, position, .. }
            if name.as_str() == "Ember Reach"
                && position.x.get() == 32_768
                && position.y.get() == -4_096
    ));

    let model = WorkshopUiModel::build(
        &current,
        WorkshopUiContext {
            creator_form: Some(CreatorTool::CreateSystem),
            creator_draft: Some(&draft),
            ..WorkshopUiContext::default()
        },
    );
    let form = model.creator_form.as_ref().unwrap();
    assert_eq!(form.fields.len(), 3);
    assert!(form.submit_control.enabled);
    assert_eq!(
        model
            .semantics
            .node("creator-field.control.creator.field.name")
            .unwrap()
            .role,
        SemanticRole::TextInput
    );
    let Some(WorkshopUiIntent::Dispatch(nyon::workshop::session::WorkshopAction::Submit(actual))) =
        model.activate(&form.submit_control.action_id, InputModality::Keyboard)
    else {
        panic!("edited draft did not produce a typed submit intent");
    };
    assert_eq!(actual, submitted);
}

#[test]
fn creator_draft_validation_keeps_invalid_text_out_of_the_mailbox() {
    let current = snapshot();
    let defaults = default_creator_batch(&current, None, CreatorTool::CreateSystem).unwrap();
    let mut draft = CreatorDraft::from_batch(&current.state, &defaults).unwrap();
    draft.set_text("name", "").unwrap();

    let model = WorkshopUiModel::build(
        &current,
        WorkshopUiContext {
            creator_form: Some(CreatorTool::CreateSystem),
            creator_draft: Some(&draft),
            ..WorkshopUiContext::default()
        },
    );
    let form = model.creator_form.as_ref().unwrap();
    assert!(!form.submit_control.enabled);
    assert!(form.validation_message.as_deref().unwrap().contains("Name"));
    assert_eq!(
        model.activate(&form.submit_control.action_id, InputModality::Pointer),
        None
    );
}

#[test]
fn creator_choice_fields_cycle_through_typed_available_objects() {
    let mut current = snapshot();
    current.state.systems.insert(
        entity(14),
        SystemV1 {
            name: name("Quench"),
            position: GalaxyPointV1::new(2_048, 1_024).unwrap(),
        },
    );
    current.state_digest = current.state.digest();
    current.active_view.digest = current.state_digest;
    let defaults =
        default_creator_batch(&current, Some(entity(14)), CreatorTool::ConnectLane).unwrap();
    let mut draft = CreatorDraft::from_batch(&current.state, &defaults).unwrap();
    let before = draft.field("a").unwrap().value.clone();
    assert_eq!(draft.field("a").unwrap().kind, CreatorFieldKind::Choice);
    assert!(draft.field("a").unwrap().choices.len() >= 3);
    draft.cycle("a", 1).unwrap();
    assert_ne!(draft.field("a").unwrap().value, before);
    assert_ne!(draft.to_batch().unwrap(), defaults);

    let model = WorkshopUiModel::build(
        &current,
        WorkshopUiContext {
            creator_form: Some(CreatorTool::ConnectLane),
            creator_draft: Some(&draft),
            ..WorkshopUiContext::default()
        },
    );
    let control = &model.creator_form.as_ref().unwrap().fields[0].control;
    let keyboard = model.activate(&control.action_id, InputModality::Keyboard);
    let pointer = model.activate(&control.action_id, InputModality::Pointer);
    assert_eq!(keyboard, pointer);
    assert!(matches!(
        keyboard,
        Some(WorkshopUiIntent::EditCreatorField { cycle: true, .. })
    ));
}
