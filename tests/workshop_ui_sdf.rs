mod common;
use common::*;

use glam::Vec2;
use nyon::engine::backend::BackendKind;
use nyon::ui::AtlasMetrics;
use nyon::ui::UiBatchError;
use nyon::ui::UiIcon;
use nyon::ui::accessibility::SemanticRole;
use nyon::ui::creator::CreatorDraft;
use nyon::ui::platform::build_workshop_platform_frame;
use nyon::ui::platform::build_workshop_platform_frame_for_view;
use nyon::ui::platform_sdf::PlatformPanelRole;
use nyon::ui::platform_sdf::PlatformTextOverflow;
use nyon::ui::platform_sdf::PlatformTextRole;
use nyon::ui::platform_sdf::build_platform_ui_batch;
use nyon::ui::workshop::CreatorTool;
use nyon::ui::workshop::WorkshopUiContext;
use nyon::ui::workshop::WorkshopUiIntent;
use nyon::ui::workshop::WorkshopUiModel;
use nyon::ui::workshop::default_creator_batch;
use nyon::ui::workshop_layout::WorkshopLayout;
use nyon::ui::workshop_view::WorkshopDrawer;
use nyon::ui::workshop_view::WorkshopViewAction;
use nyon::ui::workshop_view::WorkshopViewState;
use nyon::workshop::WorkshopStateV1;
use nyon::workshop::WorkshopTick;
use nyon::workshop::session::WorkshopDiagnostic;
use nyon::workshop::session::WorkshopDiagnosticCode;
use nyon::workshop::session::WorkshopSpeed;
use nyon::workshop::store::SlotId;
use std::collections::BTreeSet;

#[test]
fn platform_sdf_uses_source_records_for_semantic_text_and_icons() {
    let model = WorkshopUiModel::build(&snapshot(), WorkshopUiContext::default());
    let mut frame = build_workshop_platform_frame(&model, Vec2::new(1_280.0, 720.0), None);
    let save_id = frame
        .controls
        .iter()
        .find(|control| control.label == "Save Workshop")
        .unwrap()
        .action_id
        .clone();
    frame.append_status_line("Waiting for durable save");
    frame.reconcile_focused(Some(&save_id));
    let metrics = AtlasMetrics::embedded().unwrap();
    let output = build_platform_ui_batch(&frame, &metrics).unwrap();
    for run in &output.text_runs {
        assert_eq!(
            run.logical_advance,
            metrics
                .measure_text(run.style.font_size, run.style.weight, &run.exact_text)
                .unwrap()
                .advance
        );
        assert!(frame.visible_nodes.iter().any(|record| {
            record.semantic_id == run.semantic_id && record.bounds.contains_rect(run.clip)
        }));
    }
    for run in &output.icon_runs {
        let owner = frame
            .visible_nodes
            .iter()
            .find(|record| record.semantic_id == run.semantic_id)
            .unwrap();
        assert_eq!(owner.icon, Some(run.icon));
        assert!(owner.bounds.contains_rect(run.bounds));
        assert!(run.clip.contains_rect(run.bounds));
    }
    for run in &output.text_runs {
        for glyph in &output.batch.glyphs()[run.emitted_glyph_range[0]..run.emitted_glyph_range[1]]
        {
            let ink = nyon::ui::platform::PlatformRect::from_xywh(
                glyph.rect[0],
                glyph.rect[1],
                glyph.rect[2],
                glyph.rect[3],
            );
            assert!(run.clip.contains_rect(ink));
        }
    }
    for run in &output.icon_runs {
        for glyph in &output.batch.glyphs()[run.emitted_glyph_range[0]..run.emitted_glyph_range[1]]
        {
            let ink = nyon::ui::platform::PlatformRect::from_xywh(
                glyph.rect[0],
                glyph.rect[1],
                glyph.rect[2],
                glyph.rect[3],
            );
            assert!(run.clip.contains_rect(ink));
            assert!(run.bounds.contains_rect(ink));
        }
    }
    for node in frame
        .semantics
        .nodes_depth_first()
        .into_iter()
        .filter(|node| {
            node.visible
                && matches!(
                    node.role,
                    SemanticRole::Button
                        | SemanticRole::TreeItem
                        | SemanticRole::Heading
                        | SemanticRole::Text
                        | SemanticRole::Status
                        | SemanticRole::Alert
                        | SemanticRole::ListItem
                        | SemanticRole::Option
                        | SemanticRole::Checkbox
                        | SemanticRole::TextInput
                        | SemanticRole::SpinButton
                )
                && (!node.name.is_empty() || !node.description.is_empty() || node.value.is_some())
        })
    {
        let record = frame
            .visible_nodes
            .iter()
            .find(|record| record.semantic_id == node.id)
            .unwrap_or_else(|| panic!("semantic node {} lacks a visible record", node.id));
        assert_eq!(record.semantic_name, node.name);
        assert_eq!(record.semantic_description, node.description);
        assert_eq!(record.semantic_value, node.value);
        assert!(
            output
                .text_runs
                .iter()
                .any(|run| run.semantic_id == node.id)
                || output
                    .icon_runs
                    .iter()
                    .any(|run| run.semantic_id == node.id),
            "semantic node {} lacks SDF output",
            node.id
        );
    }
    assert_action_witnesses(&model, &frame);
    assert!(!frame.visible_nodes.is_empty());
    assert!(frame.visible_nodes.iter().any(|record| {
        record.role == PlatformTextRole::Status
            && record.overflow == PlatformTextOverflow::Wrap
            && record.semantic_id.as_str() == "save.status"
    }));
    assert!(frame.visible_nodes.iter().any(|record| {
        record.role == PlatformTextRole::Status && record.display_text == "Waiting for durable save"
    }));
    for record in frame
        .visible_nodes
        .iter()
        .filter(|record| record.informative())
    {
        assert!(
            output
                .text_runs
                .iter()
                .any(|run| { run.semantic_id == record.semantic_id && !run.exact_text.is_empty() })
                || output
                    .icon_runs
                    .iter()
                    .any(|run| run.semantic_id == record.semantic_id)
        );
    }
    let save = frame
        .controls
        .iter()
        .find(|control| control.label == "Save Workshop")
        .unwrap();
    assert_eq!(save.icon, Some(UiIcon::Save));
    assert!(save.focused);
    assert!(
        output
            .icon_runs
            .iter()
            .any(|run| run.semantic_id == save.semantic_id && run.icon == UiIcon::Save)
    );
}

#[test]
fn typed_icons_freeze_allowed_and_forbidden_meanings() {
    use nyon::{
        app::client_runtime::MainMenuRoute,
        ui::{
            guide::{GuideAction, GuideLocation},
            platform::{PlatformIconAction as Source, ShellUiAction, icon_for_action},
        },
        workshop::session::WorkshopAction,
    };
    for (action, expected) in [
        (WorkshopViewAction::OpenCreator, Some(UiIcon::Crosshair)),
        (WorkshopViewAction::OpenNavigator, Some(UiIcon::Zoom)),
        (WorkshopViewAction::CloseDrawer, Some(UiIcon::Close)),
        (WorkshopViewAction::ShowInspector, Some(UiIcon::Zoom)),
        (WorkshopViewAction::ScrollPrevious, None),
        (WorkshopViewAction::ScrollNext, None),
        (WorkshopViewAction::ScrollInspectorPrevious, None),
        (WorkshopViewAction::ScrollInspectorNext, None),
        (WorkshopViewAction::ShowHierarchy, None),
        (WorkshopViewAction::ShowBranches, None),
    ] {
        assert_eq!(icon_for_action(Source::WorkshopView(action)), expected);
    }
    for (action, expected) in [
        (
            ShellUiAction::Menu(MainMenuRoute::Continue),
            Some(UiIcon::Load),
        ),
        (
            ShellUiAction::Menu(MainMenuRoute::Settings),
            Some(UiIcon::Settings),
        ),
        (ShellUiAction::Menu(MainMenuRoute::NewWorkshop), None),
        (ShellUiAction::Menu(MainMenuRoute::ClassicSector), None),
        (ShellUiAction::Menu(MainMenuRoute::Credits), None),
        (ShellUiAction::CloseSettings, Some(UiIcon::Close)),
        (ShellUiAction::DismissCredits, Some(UiIcon::Close)),
        (ShellUiAction::DismissRecovery, Some(UiIcon::Close)),
        (ShellUiAction::ContinueRecovery, Some(UiIcon::Check)),
        (ShellUiAction::CycleUiScale, Some(UiIcon::Settings)),
        (ShellUiAction::ToggleReducedMotion, Some(UiIcon::Settings)),
        (ShellUiAction::ToggleHighContrast, Some(UiIcon::Settings)),
        (ShellUiAction::ReturnToMainMenu, None),
    ] {
        assert_eq!(icon_for_action(Source::Shell(action)), expected);
    }
    for (action, expected) in [
        (GuideAction::Open, Some(UiIcon::Help)),
        (GuideAction::Close, Some(UiIcon::Close)),
        (GuideAction::MainMenu, None),
        (
            GuideAction::Show(GuideLocation::for_screen(
                nyon::app::client_runtime::ClientScreen::GalaxyWorkshop,
            )),
            None,
        ),
    ] {
        assert_eq!(icon_for_action(Source::Guide(action)), expected);
    }
    for (intent, expected) in [
        (
            WorkshopUiIntent::Dispatch(WorkshopAction::Pause),
            Some(UiIcon::Pause),
        ),
        (
            WorkshopUiIntent::Dispatch(WorkshopAction::Resume(WorkshopSpeed::One)),
            Some(UiIcon::Speed),
        ),
        (
            WorkshopUiIntent::Dispatch(WorkshopAction::StepOnce),
            Some(UiIcon::Speed),
        ),
        (
            WorkshopUiIntent::Dispatch(WorkshopAction::RequestSave),
            Some(UiIcon::Save),
        ),
        (WorkshopUiIntent::Dispatch(WorkshopAction::Undo), None),
        (
            WorkshopUiIntent::Dispatch(WorkshopAction::Redo(revision(2))),
            None,
        ),
        (
            WorkshopUiIntent::Dispatch(WorkshopAction::SelectBranch(branch(1))),
            None,
        ),
        (
            WorkshopUiIntent::Dispatch(WorkshopAction::RequestExport),
            None,
        ),
        (
            WorkshopUiIntent::Dispatch(WorkshopAction::RequestImport(Box::new([]))),
            None,
        ),
        (
            WorkshopUiIntent::Dispatch(WorkshopAction::RequestLoad(SlotId(4))),
            Some(UiIcon::Load),
        ),
        (
            WorkshopUiIntent::Dispatch(WorkshopAction::Submit(
                default_creator_batch(&snapshot(), None, CreatorTool::CreateSystem).unwrap(),
            )),
            Some(UiIcon::Check),
        ),
        (
            WorkshopUiIntent::OpenCreatorForm {
                tool: CreatorTool::CreateSystem,
                subject: None,
            },
            Some(UiIcon::Crosshair),
        ),
        (WorkshopUiIntent::CloseCreatorForm, Some(UiIcon::Close)),
        (
            WorkshopUiIntent::CloseRemovalConfirmation,
            Some(UiIcon::Close),
        ),
        (WorkshopUiIntent::ReturnToMainMenu, None),
        (
            WorkshopUiIntent::SelectEntity(entity(2)),
            Some(UiIcon::Crosshair),
        ),
        (
            WorkshopUiIntent::SetReducedMotion(true),
            Some(UiIcon::Settings),
        ),
        (
            WorkshopUiIntent::SetHighContrast(true),
            Some(UiIcon::Settings),
        ),
        (WorkshopUiIntent::OpenRemovalConfirmation(entity(2)), None),
        (
            WorkshopUiIntent::EditCreatorField {
                field_id: "name".to_owned(),
                cycle: false,
            },
            None,
        ),
    ] {
        assert_eq!(icon_for_action(Source::Workshop(&intent)), expected);
    }
    #[cfg(not(target_arch = "wasm32"))]
    assert_eq!(
        icon_for_action(Source::Shell(ShellUiAction::Menu(MainMenuRoute::Quit))),
        None
    );
}

#[test]
fn sdf_required_matrix_crosses_drawers_modal_validation_and_contrast() {
    let mut glyph_maximum = (0, String::new());
    let mut panel_maximum = (0, String::new());
    let current = snapshot();
    let catalog = catalog_with_maximum_industry_description();
    let defaults = default_creator_batch(&current, None, CreatorTool::CreateSystem).unwrap();
    let mut invalid = CreatorDraft::from_batch(&current.state, &defaults).unwrap();
    invalid.set_text("name", "").unwrap();
    for (width, height, scale) in SDF_QUALIFICATION_MATRIX {
        let layout = WorkshopLayout::resolve(Vec2::new(width, height), scale).unwrap();
        for high_contrast in [false, true] {
            for state in 0..6 {
                let context = WorkshopUiContext {
                    catalog: Some(&catalog),
                    selected_entity: Some(entity(6)),
                    high_contrast,
                    creator_form: (state == 2 || state == 3).then_some(CreatorTool::CreateSystem),
                    creator_draft: (state == 3).then_some(&invalid),
                    pending_removal: (state == 1).then_some(entity(2)),
                    backend: (state == 5).then_some(BackendKind::WebGl2Low),
                    ..WorkshopUiContext::default()
                };
                let mut state_snapshot = current.clone();
                if state == 4 {
                    state_snapshot.state = WorkshopStateV1::default();
                    state_snapshot.state_digest = state_snapshot.state.digest();
                    state_snapshot.active_view.digest = state_snapshot.state_digest;
                    state_snapshot.active_view.tick = WorkshopTick(0);
                }
                if state == 5 {
                    state_snapshot.store.commit_pending = true;
                    state_snapshot.diagnostics.push(WorkshopDiagnostic {
                        code: WorkshopDiagnosticCode::ArchiveRejected,
                        message: "Untrusted imported error".repeat(30),
                    });
                }
                let model = WorkshopUiModel::build(&state_snapshot, context);
                for open_drawer in [
                    None,
                    Some(WorkshopDrawer::Navigator),
                    Some(WorkshopDrawer::Creator),
                ] {
                    let mut view = WorkshopViewState::default();
                    view.open_drawer = open_drawer;
                    let mut frame =
                        build_workshop_platform_frame_for_view(&model, layout.clone(), &view, None);
                    let focus = frame
                        .controls
                        .iter()
                        .find(|control| {
                            control.enabled
                                && frame
                                    .semantics
                                    .node(control.semantic_id.as_str())
                                    .is_some_and(|node| node.visible)
                        })
                        .map(|control| control.action_id.clone());
                    frame.reconcile_focused(focus.as_ref());
                    qualify_sdf_frame(&model, &frame);
                    let output =
                        build_platform_ui_batch(&frame, &AtlasMetrics::embedded().unwrap())
                            .unwrap();
                    let state = format!(
                        "{width}x{height}@{scale} contrast={high_contrast} state={state} drawer={open_drawer:?}"
                    );
                    if output.batch.glyphs().len() > glyph_maximum.0 {
                        glyph_maximum = (output.batch.glyphs().len(), state.clone());
                    }
                    if output.batch.panels().len() > panel_maximum.0 {
                        panel_maximum = (output.batch.panels().len(), state);
                    }
                }
            }
        }
    }
    eprintln!("MATRIX MAX glyphs={glyph_maximum:?} panels={panel_maximum:?}");
}

#[test]
fn settings_ui_scale_control_is_exact_semantic_and_ink_qualified_on_short_viewports() {
    use nyon::{
        app::client_runtime::{ClientScreen, MainMenuCapability},
        preferences::{UiScale, UserPreferencesV1},
        ui::{
            accessibility::SemanticActionId,
            platform::{
                PlatformUiAction, ShellPlatformInput, ShellUiAction, build_shell_platform_frame,
            },
        },
    };

    let model = WorkshopUiModel::build(&snapshot(), WorkshopUiContext::default());
    let action_id = SemanticActionId::new("shell.settings.scale");
    for viewport in [Vec2::new(1280.0, 480.0), Vec2::new(723.0, 802.0)] {
        let frame = build_shell_platform_frame(ShellPlatformInput {
            screen: ClientScreen::Settings,
            capabilities: &[] as &[MainMenuCapability],
            credits_visible: false,
            recovery_message: None,
            continue_available: false,
            backend: Some(BackendKind::WebGl2Low),
            preferences: UserPreferencesV1 {
                ui_scale: UiScale::Percent130,
                ..UserPreferencesV1::default()
            },
            viewport,
            focused: Some(&action_id),
        });
        let control = frame
            .controls
            .iter()
            .find(|control| control.action_id == action_id)
            .unwrap_or_else(|| panic!("missing rendered UI scale action at {viewport:?}"));
        assert_eq!(control.label, "UI scale: 130%");
        assert_eq!(
            control.action,
            PlatformUiAction::Shell(ShellUiAction::CycleUiScale)
        );
        assert_eq!(
            control.description,
            "Cycle UI scale through 85%, 100%, 115%, and 130%."
        );
        assert!(control.focused);
        assert_eq!(
            frame
                .hit_test(control.bounds.center())
                .map(|hit| &hit.action_id),
            Some(&action_id)
        );
        let semantic = frame
            .semantics
            .nodes_depth_first()
            .into_iter()
            .find(|node| node.action_id.as_ref() == Some(&action_id))
            .expect("UI scale action must have a semantic control");
        assert_eq!(semantic.name, control.label);
        assert_eq!(semantic.description, control.description);
        assert_eq!(semantic.bounds, Some(control.bounds.into()));
        qualify_sdf_frame(&model, &frame);
        let output = build_platform_ui_batch(&frame, &AtlasMetrics::embedded().unwrap()).unwrap();
        let exact_ink = output
            .text_runs
            .iter()
            .filter(|run| run.semantic_id == control.semantic_id)
            .map(|run| run.exact_text.as_str())
            .collect::<String>();
        assert_eq!(exact_ink, control.label);
    }
}

#[test]
fn platform_sdf_preserves_semantic_sources_layers_and_measured_text_contracts() {
    let model = WorkshopUiModel::build(&snapshot(), WorkshopUiContext::default());
    let mut frame = build_workshop_platform_frame(&model, Vec2::new(1_280.0, 720.0), None);
    let metrics = AtlasMetrics::embedded().unwrap();

    for id in [
        "save.status",
        "diagnostics.backend",
        "diagnostics.catalog",
        "diagnostics.digest",
        "diagnostics.tick",
    ] {
        let node = frame.semantics.node(id).unwrap();
        let record = frame
            .visible_nodes
            .iter()
            .find(|record| record.semantic_id == node.id)
            .unwrap_or_else(|| panic!("missing visible record for {id}"));
        assert_eq!(record.semantic_name, node.name);
        assert_eq!(record.semantic_description, node.description);
        assert_eq!(record.semantic_value, node.value);
    }

    let inspector_id = model.inspector.text_records()[1].semantic_id.clone();
    let output = build_platform_ui_batch(&frame, &metrics).unwrap();
    assert!(output.panel_witnesses.iter().any(|witness| {
        witness.role == PlatformPanelRole::InspectorRowFill
            && witness.owning_node.as_ref() == Some(&inspector_id)
    }));

    frame.visible_nodes[0].display_text = "A  B".to_owned();
    frame.visible_nodes[0].overflow = PlatformTextOverflow::Wrap;
    frame.visible_nodes[0].bounds =
        nyon::ui::platform::PlatformRect::from_xywh(0.0, 0.0, 200.0, 80.0);
    frame.visible_nodes[0].clip = Some(frame.visible_nodes[0].bounds);
    frame.visible_nodes[0].prewrapped_lines = None;
    let whitespace = build_platform_ui_batch(&frame, &metrics).unwrap();
    let exact = whitespace
        .text_runs
        .iter()
        .filter(|run| run.semantic_id == frame.visible_nodes[0].semantic_id)
        .map(|run| run.exact_text.as_str())
        .collect::<String>();
    assert_eq!(exact, "A  B");

    frame.visible_nodes[0].overflow = PlatformTextOverflow::SingleLineEllipsis;
    frame.visible_nodes[0].display_text = "A very long measured platform title".to_owned();
    frame.visible_nodes[0].bounds =
        nyon::ui::platform::PlatformRect::from_xywh(0.0, 0.0, 60.0, 40.0);
    frame.visible_nodes[0].clip = Some(frame.visible_nodes[0].bounds);
    let ellipsized = build_platform_ui_batch(&frame, &metrics).unwrap();
    let run = ellipsized
        .text_runs
        .iter()
        .find(|run| run.semantic_id == frame.visible_nodes[0].semantic_id)
        .unwrap();
    assert!(run.exact_text.ends_with('.'));
    assert!(run.logical_advance <= run.bounds.width());

    frame.visible_nodes[0].bounds =
        nyon::ui::platform::PlatformRect::from_xywh(0.0, 0.0, 1.0, 40.0);
    frame.visible_nodes[0].clip = Some(frame.visible_nodes[0].bounds);
    let narrow = build_platform_ui_batch(&frame, &metrics).unwrap();
    let run = narrow
        .text_runs
        .iter()
        .find(|run| run.semantic_id == frame.visible_nodes[0].semantic_id)
        .unwrap();
    assert!(run.logical_advance <= run.bounds.width());
}

#[test]
fn platform_sdf_modal_suppresses_nonmodal_records_and_full_surface_is_typed() {
    let model = WorkshopUiModel::build(
        &snapshot(),
        WorkshopUiContext {
            pending_removal: Some(entity(2)),
            ..WorkshopUiContext::default()
        },
    );
    let frame = build_workshop_platform_frame(&model, Vec2::new(1_280.0, 720.0), None);
    let output = build_platform_ui_batch(&frame, &AtlasMetrics::embedded().unwrap()).unwrap();
    for node in frame
        .semantics
        .nodes_depth_first()
        .into_iter()
        .filter(|node| node.id.as_str().starts_with("removal.blocker."))
    {
        assert!(
            frame
                .visible_nodes
                .iter()
                .any(|record| record.semantic_id == node.id)
        );
        assert!(
            output
                .text_runs
                .iter()
                .any(|run| run.semantic_id == node.id)
        );
    }
    assert!(
        output
            .panel_witnesses
            .iter()
            .all(|witness| witness.role != PlatformPanelRole::FullSurface)
    );
    let modal = frame.modal.unwrap();
    let dialog = frame
        .semantics
        .nodes_depth_first()
        .into_iter()
        .find(|node| node.role == SemanticRole::Dialog)
        .unwrap();
    let mut dialog_ids = BTreeSet::new();
    let mut pending = vec![dialog];
    while let Some(node) = pending.pop() {
        dialog_ids.insert(node.id.clone());
        pending.extend(&node.children);
    }
    let records = frame
        .visible_nodes
        .iter()
        .map(|record| record.semantic_id.clone())
        .collect::<BTreeSet<_>>();
    assert!(records.is_subset(&dialog_ids));
    let rendered = output
        .text_runs
        .iter()
        .map(|run| run.semantic_id.clone())
        .chain(output.icon_runs.iter().map(|run| run.semantic_id.clone()))
        .collect::<BTreeSet<_>>();
    assert_eq!(records, rendered);
    let mut rebuilt = frame.clone();
    rebuilt.append_status_line("Saving before exit");
    rebuilt.reconcile_focused(frame.focus_order().first());
    assert_eq!(
        records,
        rebuilt
            .visible_nodes
            .iter()
            .map(|record| record.semantic_id.clone())
            .collect::<BTreeSet<_>>()
    );
    assert_action_witnesses(&model, &rebuilt);
    assert!(frame.semantics.nodes_depth_first().into_iter().all(|node| {
        node.role == SemanticRole::Application
            || dialog_ids.contains(&node.id)
            || (!node.visible && !node.enabled)
    }));
    assert!(
        output
            .text_runs
            .iter()
            .all(|run| modal.contains_rect(run.bounds))
    );
    assert!(
        output
            .icon_runs
            .iter()
            .all(|run| modal.contains_rect(run.bounds))
    );
    let modal_panel_end = output
        .panel_witnesses
        .iter()
        .find(|witness| witness.role == PlatformPanelRole::Modal)
        .unwrap()
        .emitted_panel_range[1];
    assert!(output.panel_witnesses.iter().all(|witness| {
        witness.role != PlatformPanelRole::ControlFill
            || (modal.contains_rect(witness.bounds)
                && witness.emitted_panel_range[0] >= modal_panel_end)
    }));

    let guide = nyon::ui::guide::build_guide_frame(
        nyon::ui::guide::GuideLocation::for_screen(
            nyon::app::client_runtime::ClientScreen::GalaxyWorkshop,
        ),
        Vec2::new(800.0, 600.0),
        false,
        None,
    );
    let guide_output =
        build_platform_ui_batch(&guide.platform, &AtlasMetrics::embedded().unwrap()).unwrap();
    assert_action_witnesses(&model, &guide.platform);
    assert!(guide_output.panel_witnesses.iter().any(|witness| {
        witness.role == PlatformPanelRole::FullSurface
            && witness.bounds == guide.platform.layout.viewport
    }));
}

#[test]
fn platform_sdf_capacity_failures_return_no_partial_output() {
    let model = WorkshopUiModel::build(&snapshot(), WorkshopUiContext::default());
    let metrics = AtlasMetrics::embedded().unwrap();
    let mut glyph_frame = build_workshop_platform_frame(&model, Vec2::new(1_280.0, 720.0), None);
    let template = glyph_frame.visible_nodes[0].clone();
    glyph_frame.controls.clear();
    glyph_frame.sighted_text.clear();
    glyph_frame.visible_nodes = (0..8_300)
        .map(|index| {
            let mut record = template.clone();
            record.semantic_id =
                nyon::ui::accessibility::SemanticNodeId::new(format!("capacity.glyph.{index}"));
            record.display_text = "X".to_owned();
            record.prewrapped_lines = None;
            record
        })
        .collect();
    assert!(matches!(
        build_platform_ui_batch(&glyph_frame, &metrics),
        Err(UiBatchError::Capacity { requested: 8_193 })
    ));

    let mut panel_frame = build_workshop_platform_frame(&model, Vec2::new(1_280.0, 720.0), None);
    panel_frame.visible_nodes.clear();
    let template = panel_frame.controls[0].clone();
    panel_frame.controls = (0..520).map(|_| template.clone()).collect();
    assert!(matches!(
        build_platform_ui_batch(&panel_frame, &metrics),
        Err(UiBatchError::PanelCapacity { .. })
    ));

    let mut exact_glyph_frame =
        build_workshop_platform_frame(&model, Vec2::new(1_280.0, 720.0), None);
    exact_glyph_frame.controls.clear();
    exact_glyph_frame.visible_nodes.truncate(1);
    exact_glyph_frame.visible_nodes[0].display_text = "X".repeat(8_192);
    exact_glyph_frame.visible_nodes[0].overflow = PlatformTextOverflow::Wrap;
    exact_glyph_frame.visible_nodes[0].bounds =
        nyon::ui::platform::PlatformRect::from_xywh(0.0, 0.0, 1_000_000.0, 40.0);
    exact_glyph_frame.visible_nodes[0].clip = Some(exact_glyph_frame.visible_nodes[0].bounds);
    exact_glyph_frame.viewport = Vec2::new(1_000_000.0, 720.0);
    let exact_glyphs = build_platform_ui_batch(&exact_glyph_frame, &metrics).unwrap();
    assert_eq!(exact_glyphs.batch.glyphs().len(), 8_192);

    let mut exact_panel_frame =
        build_workshop_platform_frame(&model, Vec2::new(1_280.0, 720.0), None);
    exact_panel_frame.visible_nodes.clear();
    let control = exact_panel_frame.controls[0].clone();
    exact_panel_frame.controls.clear();
    let base_panels = build_platform_ui_batch(&exact_panel_frame, &metrics)
        .unwrap()
        .batch
        .panels()
        .len();
    exact_panel_frame.controls = (base_panels..512).map(|_| control.clone()).collect();
    let exact_panels = build_platform_ui_batch(&exact_panel_frame, &metrics).unwrap();
    assert_eq!(exact_panels.batch.panels().len(), 512);
}

#[test]
fn platform_sdf_identity_and_exact_text_survive_presentation_preferences() {
    let snapshot = snapshot();
    let normal = WorkshopUiModel::build(&snapshot, WorkshopUiContext::default());
    let altered = WorkshopUiModel::build(
        &snapshot,
        WorkshopUiContext {
            reduced_motion: true,
            high_contrast: true,
            ..WorkshopUiContext::default()
        },
    );
    let viewport = Vec2::new(1_280.0, 720.0);
    let normal_frame = build_workshop_platform_frame(&normal, viewport, None);
    let focus = altered.save.save_control.action_id.clone();
    let mut altered_frame = build_workshop_platform_frame(&altered, viewport, Some(&focus));
    altered_frame.reconcile_focused(Some(&focus));
    let normal_records = normal_frame
        .visible_nodes
        .iter()
        .map(|record| (record.semantic_id.clone(), record.display_text.clone()))
        .collect::<BTreeSet<_>>();
    let altered_records = altered_frame
        .visible_nodes
        .iter()
        .map(|record| (record.semantic_id.clone(), record.display_text.clone()))
        .collect::<BTreeSet<_>>();
    assert_eq!(normal_records, altered_records);

    let output =
        build_platform_ui_batch(&altered_frame, &AtlasMetrics::embedded().unwrap()).unwrap();
    assert!(output.panel_witnesses.iter().any(|witness| {
        witness.role == PlatformPanelRole::ContrastBorder
            && witness.owning_node.as_ref()
                == altered_frame
                    .controls
                    .iter()
                    .find(|control| control.action_id == focus)
                    .map(|control| &control.semantic_id)
    }));
    let border = output
        .panel_witnesses
        .iter()
        .find(|witness| witness.role == PlatformPanelRole::ContrastBorder)
        .unwrap();
    assert_eq!(
        border.emitted_panel_range[1] - border.emitted_panel_range[0],
        4
    );
    let strips =
        &output.batch.panels()[border.emitted_panel_range[0]..border.emitted_panel_range[1]];
    assert!(
        strips
            .iter()
            .all(|strip| strip.rect[2] <= 2.0 || strip.rect[3] <= 2.0)
    );

    let normal_output =
        build_platform_ui_batch(&normal_frame, &AtlasMetrics::embedded().unwrap()).unwrap();
    let normal_identity = normal_output
        .text_runs
        .iter()
        .map(|run| (run.semantic_id.clone(), run.exact_text.clone()))
        .chain(
            normal_output
                .icon_runs
                .iter()
                .map(|run| (run.semantic_id.clone(), format!("icon:{:?}", run.icon))),
        )
        .collect::<BTreeSet<_>>();
    let altered_identity = output
        .text_runs
        .iter()
        .map(|run| (run.semantic_id.clone(), run.exact_text.clone()))
        .chain(
            output
                .icon_runs
                .iter()
                .map(|run| (run.semantic_id.clone(), format!("icon:{:?}", run.icon))),
        )
        .collect::<BTreeSet<_>>();
    assert_eq!(normal_identity, altered_identity);
}

#[test]
fn high_contrast_selection_has_owner_linked_outline_with_and_without_focus() {
    let mut current = snapshot();
    current.speed = WorkshopSpeed::One;
    let selected_entity = current.state.systems.keys().next().copied();
    let model = WorkshopUiModel::build(
        &current,
        WorkshopUiContext {
            reduced_motion: true,
            high_contrast: true,
            selected_entity,
            ..WorkshopUiContext::default()
        },
    );
    let selected_actions = [
        model.preferences.reduced_motion_control.action_id.clone(),
        model
            .timeline
            .controls
            .iter()
            .find(|control| control.selected)
            .expect("active speed control")
            .action_id
            .clone(),
        model
            .outliner
            .iter()
            .find(|entry| entry.control.selected)
            .expect("selected outliner row")
            .control
            .action_id
            .clone(),
    ];
    let metrics = AtlasMetrics::embedded().unwrap();

    for scale in [1.0, 1.3] {
        let layout = WorkshopLayout::resolve(Vec2::new(1920.0, 1080.0), scale).unwrap();
        for action in &selected_actions {
            let mut view = WorkshopViewState::default();
            assert!(view.reveal_action(&model, &layout, action));
            for focused in [false, true] {
                let focus = focused.then_some(action);
                let frame =
                    build_workshop_platform_frame_for_view(&model, layout.clone(), &view, focus);
                let control = frame
                    .controls
                    .iter()
                    .find(|control| &control.action_id == action)
                    .unwrap();
                assert!(control.selected);
                assert!(control.enabled, "selected cue fixture disabled {action}");
                assert_eq!(control.focused, focused);
                let output = build_platform_ui_batch(&frame, &metrics).unwrap();
                let outline = output
                    .panel_witnesses
                    .iter()
                    .find(|witness| {
                        witness.role == PlatformPanelRole::ContrastBorder
                            && witness.owning_node.as_ref() == Some(&control.semantic_id)
                    })
                    .unwrap_or_else(|| {
                        panic!(
                            "selected control {action} has no non-color outline at {scale}, focused={focused}"
                        )
                    });
                assert_eq!(outline.bounds, control.bounds);
                assert_eq!(
                    outline.emitted_panel_range[1] - outline.emitted_panel_range[0],
                    4
                );

                let mut overlay = nyon::ui::platform::PlatformPrimitiveOverlay::default();
                frame.append_primitive_overlay(&mut overlay).unwrap();
                assert_eq!(
                    overlay.witnesses.iter().any(|witness| {
                        witness.kind == nyon::ui::platform::PlatformDecorationKind::Focus
                            && witness.owning_node.as_ref() == Some(&control.semantic_id)
                    }),
                    focused,
                    "focus remains a separate owned primitive cue for {action}: {:?}",
                    overlay.witnesses
                );
            }
        }
    }
}

#[test]
fn platform_sdf_materializes_empty_and_creator_diagnostic_semantics() {
    let mut empty_snapshot = snapshot();
    empty_snapshot.state = WorkshopStateV1::default();
    let empty = WorkshopUiModel::build(&empty_snapshot, WorkshopUiContext::default());
    let empty_frame = build_workshop_platform_frame(&empty, Vec2::new(1_280.0, 720.0), None);
    let empty_output =
        build_platform_ui_batch(&empty_frame, &AtlasMetrics::embedded().unwrap()).unwrap();
    let empty_node = empty_frame.semantics.node("outliner.empty").unwrap();
    let empty_record = empty_frame
        .visible_nodes
        .iter()
        .find(|record| record.semantic_id == empty_node.id)
        .unwrap();
    assert_eq!(empty_record.semantic_name, empty_node.name);
    assert_eq!(empty_record.semantic_description, empty_node.description);
    assert!(
        empty_output
            .text_runs
            .iter()
            .any(|run| run.semantic_id == empty_node.id)
    );

    let current = snapshot();
    let defaults = default_creator_batch(&current, None, CreatorTool::CreateSystem).unwrap();
    let mut draft = CreatorDraft::from_batch(&current.state, &defaults).unwrap();
    draft.set_text("name", "").unwrap();
    let creator = WorkshopUiModel::build(
        &current,
        WorkshopUiContext {
            creator_form: Some(CreatorTool::CreateSystem),
            creator_draft: Some(&draft),
            ..WorkshopUiContext::default()
        },
    );
    let creator_frame = build_workshop_platform_frame(&creator, Vec2::new(1_280.0, 720.0), None);
    let creator_output =
        build_platform_ui_batch(&creator_frame, &AtlasMetrics::embedded().unwrap()).unwrap();
    for node in creator_frame
        .semantics
        .nodes_depth_first()
        .into_iter()
        .filter(|node| {
            node.id.as_str().starts_with("creator.preview.")
                || node.id.as_str() == "creator.validation"
                || node.id.as_str().starts_with("creator-field.")
        })
    {
        let record = creator_frame
            .visible_nodes
            .iter()
            .find(|record| record.semantic_id == node.id)
            .unwrap_or_else(|| panic!("missing creator record {}", node.id));
        assert_eq!(record.semantic_name, node.name);
        assert_eq!(record.semantic_description, node.description);
        assert_eq!(record.semantic_value, node.value);
        assert!(
            creator_output
                .text_runs
                .iter()
                .any(|run| run.semantic_id == node.id)
                || creator_output
                    .icon_runs
                    .iter()
                    .any(|run| run.semantic_id == node.id)
        );
    }
}

#[test]
fn bundled_nonfixed_ascii_ink_fits_owned_records_at_required_scales() {
    let model = WorkshopUiModel::build(&snapshot(), WorkshopUiContext::default());
    let text = (b'!'..=b'~').map(char::from).collect::<String>();
    for (_, _, scale) in SDF_QUALIFICATION_MATRIX {
        let mut frame = build_workshop_platform_frame(&model, Vec2::new(1280.0, 720.0), None);
        frame.layout.ui_scale = scale;
        frame.visible_nodes.truncate(1);
        let record = &mut frame.visible_nodes[0];
        record.display_text = text.clone();
        record.prewrapped_lines = None;
        record.role = PlatformTextRole::Body;
        record.overflow = PlatformTextOverflow::Wrap;
        record.bounds = nyon::ui::platform::PlatformRect::from_xywh(20.0, 100.0, 240.0, 500.0);
        record.clip = Some(record.bounds);
        let owned_bounds = record.bounds;
        let output = build_platform_ui_batch(&frame, &AtlasMetrics::embedded().unwrap()).unwrap();
        assert_eq!(
            output
                .text_runs
                .iter()
                .map(|run| run.exact_text.as_str())
                .collect::<String>(),
            text
        );
        for run in &output.text_runs {
            assert_full_text_ink(run, &output.batch);
            assert!(run.clip.contains_rect(run.bounds));
            assert!(owned_bounds.contains_rect(run.clip));
        }
    }
}
