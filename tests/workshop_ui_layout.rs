mod common;
use common::*;

use glam::Vec2;
use nyon::engine::backend::BackendKind;
use nyon::engine::primitives::PrimitiveBatch;
use nyon::ui::AtlasMetrics;
use nyon::ui::UiBatchError;
use nyon::ui::accessibility::InputModality;
use nyon::ui::platform::PlatformBackground;
use nyon::ui::platform::WorkshopMarkerLayer;
use nyon::ui::platform::build_workshop_platform_frame;
use nyon::ui::platform::build_workshop_platform_frame_for_view;
use nyon::ui::platform::build_workshop_platform_frame_with_layout;
use nyon::ui::platform::draw_workshop_scene;
use nyon::ui::platform_sdf::PlatformPanelRole;
use nyon::ui::platform_sdf::PlatformTextOverflow;
use nyon::ui::platform_sdf::PlatformTextRole;
use nyon::ui::platform_sdf::build_platform_ui_batch;
use nyon::ui::workshop::CreatorTool;
use nyon::ui::workshop::WorkshopUiContext;
use nyon::ui::workshop::WorkshopUiModel;
use nyon::ui::workshop_layout::WorkshopLayout;
use nyon::ui::workshop_layout::WorkshopLayoutMode;
use nyon::ui::workshop_view::WorkshopDrawer;
use nyon::ui::workshop_view::WorkshopViewAction;
use nyon::ui::workshop_view::WorkshopViewState;
use std::collections::BTreeMap;
use std::collections::BTreeSet;

#[test]
fn fixed_action_labels_supplement_icons_across_responsive_presentations() {
    for (viewport, scale) in [
        Vec2::new(1600.0, 900.0),
        Vec2::new(1280.0, 720.0),
        Vec2::new(1024.0, 768.0),
        Vec2::new(723.0, 802.0),
        Vec2::new(640.0, 480.0),
        Vec2::new(320.0, 450.0),
    ]
    .into_iter()
    .flat_map(|viewport| [0.85, 1.0, 1.15, 1.3].map(|scale| (viewport, scale)))
    {
        let layout = WorkshopLayout::resolve(viewport, scale).unwrap();
        for context in [
            WorkshopUiContext::default(),
            WorkshopUiContext {
                pending_removal: Some(entity(2)),
                ..WorkshopUiContext::default()
            },
        ]
        .into_iter()
        .chain(CreatorTool::ALL.map(|tool| WorkshopUiContext {
            creator_form: Some(tool),
            ..WorkshopUiContext::default()
        })) {
            let model = WorkshopUiModel::build(&snapshot(), context);
            for open_drawer in [
                None,
                Some(WorkshopDrawer::Creator),
                Some(WorkshopDrawer::Navigator),
            ] {
                let mut view = WorkshopViewState::default();
                view.open_drawer = open_drawer;
                let frame =
                    build_workshop_platform_frame_for_view(&model, layout.clone(), &view, None);
                if let Some(form) = &model.creator_form {
                    let cancel = frame
                        .controls
                        .iter()
                        .find(|control| control.action_id == form.cancel_control.action_id)
                        .unwrap();
                    let submit = frame
                        .controls
                        .iter()
                        .find(|control| control.action_id == form.submit_control.action_id)
                        .unwrap();
                    assert!(!cancel.bounds.overlaps(submit.bounds));
                    assert!(cancel.bounds.width() >= 44.0 && cancel.bounds.height() >= 39.99);
                    assert!(submit.bounds.width() >= 44.0 && submit.bounds.height() >= 39.99);
                }
                assert!(
                    build_platform_ui_batch(&frame, &AtlasMetrics::embedded().unwrap()).is_ok(),
                    "fixed layout failed at {viewport:?}, scale {scale}, form {:?}, drawer {open_drawer:?}",
                    model.creator_form.as_ref().map(|form| form.tool)
                );
                assert_action_witnesses(&model, &frame);
            }
        }
    }
}

#[test]
fn fixed_action_layout_rejects_boxes_that_cannot_fit_readable_lines() {
    let model = WorkshopUiModel::build(&snapshot(), WorkshopUiContext::default());
    let mut frame = build_workshop_platform_frame(&model, Vec2::new(1280.0, 720.0), None);
    let record = frame
        .visible_nodes
        .iter_mut()
        .find(|record| {
            record.role == PlatformTextRole::Control
                && record.overflow == PlatformTextOverflow::Wrap
        })
        .unwrap();
    record.bounds.max = record.bounds.min + Vec2::new(40.0, 12.0);
    record.clip = Some(record.bounds);
    assert!(matches!(
        build_platform_ui_batch(&frame, &AtlasMetrics::embedded().unwrap()),
        Err(UiBatchError::NonFiniteGeometry),
    ));
}

#[test]
fn short_modal_pages_reveal_every_field_and_maximum_reference_blockers() {
    let current = maximum_reference_snapshot();
    let system = *current.state.systems.keys().next().unwrap();
    for scale in [1.0, 1.3] {
        let layout = WorkshopLayout::resolve(Vec2::new(1280.0, 480.0), scale).unwrap();
        for high_contrast in [false, true] {
            let model = WorkshopUiModel::build(
                &current,
                WorkshopUiContext {
                    pending_removal: Some(system),
                    high_contrast,
                    ..WorkshopUiContext::default()
                },
            );
            let mut view = WorkshopViewState::default();
            let mut seen = BTreeSet::new();
            let expected = model
                .semantics
                .nodes_depth_first()
                .into_iter()
                .filter(|node| node.id.as_str().starts_with("removal.blocker."))
                .map(|node| {
                    (
                        node.id.clone(),
                        format!("{} - {}", node.name, node.description),
                    )
                })
                .collect::<BTreeMap<_, _>>();
            let mut rendered = BTreeMap::<_, String>::new();
            let mut max_glyphs = 0;
            let mut max_panels = 0;
            let mut first = None;
            let mut forward_pages = Vec::new();
            for _ in 0..4000 {
                let frame =
                    build_workshop_platform_frame_for_view(&model, layout.clone(), &view, None);
                let output =
                    build_platform_ui_batch(&frame, &AtlasMetrics::embedded().unwrap()).unwrap();
                forward_pages.push(frame.visible_nodes.clone());
                max_glyphs = max_glyphs.max(output.batch.glyphs().len());
                max_panels = max_panels.max(output.batch.panels().len());
                let ids = output
                    .text_runs
                    .iter()
                    .filter(|run| run.semantic_id.as_str().starts_with("removal.blocker."))
                    .map(|run| run.semantic_id.clone())
                    .collect::<BTreeSet<_>>();
                seen.extend(ids);
                for run in output
                    .text_runs
                    .iter()
                    .filter(|run| run.semantic_id.as_str().starts_with("removal.blocker."))
                {
                    rendered
                        .entry(run.semantic_id.clone())
                        .or_default()
                        .push_str(&run.exact_text);
                }
                for (id, text) in &rendered {
                    assert!(
                        expected[id].starts_with(text),
                        "paging repeated or skipped {id}"
                    );
                }
                if first.is_none() {
                    first = Some(frame.clone());
                }
                for id in [
                    "remove.cancel",
                    "remove.confirm",
                    "view.scroll-previous",
                    "view.scroll-next",
                ] {
                    let control = frame
                        .controls
                        .iter()
                        .find(|control| control.action_id.as_str() == id)
                        .unwrap();
                    assert!(frame.modal.unwrap().contains_rect(control.bounds));
                    if control.enabled {
                        assert_eq!(
                            frame.hit_test(control.bounds.center()).unwrap().action,
                            control.action
                        );
                    }
                }
                if rendered == expected {
                    qualify_sdf_frame(&model, &frame);
                    break;
                }
                view.apply(WorkshopViewAction::ScrollNext, &model, &layout);
            }
            assert_eq!(seen.len(), 640);
            assert_eq!(rendered, expected);
            for expected_page in forward_pages.iter().rev().skip(1) {
                view.apply(WorkshopViewAction::ScrollPrevious, &model, &layout);
                let reverse =
                    build_workshop_platform_frame_for_view(&model, layout.clone(), &view, None);
                assert_eq!(
                    &reverse.visible_nodes, expected_page,
                    "Previous must traverse one complete measured page without overlap or skips"
                );
                qualify_sdf_frame(&model, &reverse);
            }
            assert!(max_glyphs < 8192 && max_panels < 512);
            eprintln!(
                "CAPACITY removal640 @ {scale} contrast={high_contrast}: glyphs={max_glyphs} panels={max_panels}"
            );
            let first = first.unwrap();
            qualify_sdf_frame(&model, &first);
            let first_id = nyon::ui::accessibility::SemanticNodeId::new("removal.blocker.0");
            assert!(view.reveal_semantic(&model, &layout, &first_id));
            let revealed =
                build_workshop_platform_frame_for_view(&model, layout.clone(), &view, None);
            assert_eq!(first.visible_nodes, revealed.visible_nodes);
            let creator = WorkshopUiModel::build(
                &current,
                WorkshopUiContext {
                    creator_form: Some(CreatorTool::CreateWorld),
                    high_contrast,
                    ..WorkshopUiContext::default()
                },
            );
            let form = creator.creator_form.as_ref().unwrap();
            for field in &form.fields {
                assert!(view.reveal_action(&creator, &layout, &field.control.action_id));
                let frame = build_workshop_platform_frame_for_view(
                    &creator,
                    layout.clone(),
                    &view,
                    Some(&field.control.action_id),
                );
                qualify_sdf_frame(&creator, &frame);
                let control = frame
                    .controls
                    .iter()
                    .find(|control| control.action_id == field.control.action_id)
                    .unwrap();
                assert_eq!(
                    frame.hit_test(control.bounds.center()).unwrap().action_id,
                    field.control.action_id
                );
                assert_eq!(
                    frame
                        .semantics
                        .node(control.semantic_id.as_str())
                        .unwrap()
                        .value
                        .as_deref(),
                    Some(field.value.as_str())
                );
                assert_eq!(
                    creator.activate(&field.control.action_id, InputModality::Keyboard),
                    creator.activate(&field.control.action_id, InputModality::Pointer)
                );
            }
        }
    }
}

#[test]
fn short_height_maximum_description_and_navigation_remain_reachable_in_both_motion_modes() {
    let catalog = catalog_with_maximum_industry_description();
    for scale in [1.0, 1.3] {
        let layout = WorkshopLayout::resolve(Vec2::new(1280.0, 480.0), scale).unwrap();
        for high_contrast in [false, true] {
            let current = snapshot();
            let models = [false, true].map(|reduced_motion| {
                WorkshopUiModel::build(
                    &current,
                    WorkshopUiContext {
                        catalog: Some(&catalog),
                        selected_entity: Some(entity(6)),
                        high_contrast,
                        reduced_motion,
                        ..WorkshopUiContext::default()
                    },
                )
            });
            let detail = models[0]
                .inspector
                .rows()
                .find(|row| row.label == "Catalog detail")
                .unwrap();
            assert_eq!(detail.value.len(), 512);
            assert!(detail.value.starts_with("FIRST") && detail.value.ends_with("LAST"));
            let mut view = WorkshopViewState::default();
            assert!(view.reveal_semantic(&models[0], &layout, &detail.semantic_id));
            let mut saw_first = false;
            let mut saw_last = false;
            for _ in 0..80 {
                let frames = models.each_ref().map(|model| {
                    build_workshop_platform_frame_for_view(model, layout.clone(), &view, None)
                });
                let outputs = frames.each_ref().map(|frame| {
                    build_platform_ui_batch(frame, &AtlasMetrics::embedded().unwrap()).unwrap()
                });
                assert_eq!(outputs[0].text_runs, outputs[1].text_runs);
                assert_eq!(outputs[0].icon_runs, outputs[1].icon_runs);
                assert_eq!(outputs[0].batch.glyphs(), outputs[1].batch.glyphs());
                let reduced_motion_id = frames[1]
                    .controls
                    .iter()
                    .find(|control| {
                        control.action_id == models[1].preferences.reduced_motion_control.action_id
                    })
                    .map(|control| control.semantic_id.clone())
                    .unwrap();
                let reduced_motion_outline = outputs[1].panel_witnesses.iter().find(|witness| {
                    witness.role == PlatformPanelRole::ContrastBorder
                        && witness.owning_node.as_ref() == Some(&reduced_motion_id)
                });
                assert_eq!(reduced_motion_outline.is_some(), high_contrast);
                assert!(outputs[0].panel_witnesses.iter().all(|witness| {
                    witness.role != PlatformPanelRole::ContrastBorder
                        || witness.owning_node.as_ref() != Some(&reduced_motion_id)
                }));
                let selection_range = reduced_motion_outline
                    .map(|witness| witness.emitted_panel_range)
                    .unwrap_or([0, 0]);
                assert_eq!(
                    outputs[0]
                        .batch
                        .panels()
                        .iter()
                        .map(|panel| panel.rect)
                        .collect::<Vec<_>>(),
                    outputs[1]
                        .batch
                        .panels()
                        .iter()
                        .enumerate()
                        .filter(|(index, _)| {
                            *index < selection_range[0] || *index >= selection_range[1]
                        })
                        .map(|(_, panel)| panel.rect)
                        .collect::<Vec<_>>()
                );
                let text = outputs[0]
                    .text_runs
                    .iter()
                    .filter(|run| run.semantic_id == detail.semantic_id)
                    .map(|run| run.exact_text.as_str())
                    .collect::<String>();
                saw_first |= text.contains("FIRST");
                saw_last |= text.contains("LAST");
                assert!(
                    frames[0]
                        .sighted_text
                        .iter()
                        .any(|text| text.kind == nyon::ui::workshop::InspectorTextKind::Title)
                );
                for action in [
                    &models[0].preferences.reduced_motion_control.action_id,
                    &models[0].preferences.high_contrast_control.action_id,
                    &models[0]
                        .inspector
                        .remove_control
                        .as_ref()
                        .unwrap()
                        .action_id,
                    &WorkshopViewAction::ScrollInspectorNext.action_id(),
                ] {
                    assert!(
                        frames[0]
                            .controls
                            .iter()
                            .any(|control| &control.action_id == action),
                        "missing persistent footer {action}"
                    );
                }
                qualify_sdf_frame(&models[0], &frames[0]);
                if saw_last {
                    break;
                }
                view.apply(WorkshopViewAction::ScrollInspectorNext, &models[0], &layout);
            }
            assert!(saw_first && saw_last);
            for model in &models {
                let mut view = WorkshopViewState::default();
                let default =
                    build_workshop_platform_frame_for_view(model, layout.clone(), &view, None);
                assert!(default.controls.iter().any(
                    |control| control.action_id == WorkshopViewAction::OpenCreator.action_id()
                ));
                for tool in &model.tool_palette {
                    assert!(view.reveal_action(model, &layout, &tool.control.action_id));
                    let frame = build_workshop_platform_frame_for_view(
                        model,
                        layout.clone(),
                        &view,
                        Some(&tool.control.action_id),
                    );
                    qualify_sdf_frame(model, &frame);
                    let control = frame
                        .controls
                        .iter()
                        .find(|control| control.action_id == tool.control.action_id)
                        .unwrap();
                    assert_eq!(
                        frame.hit_test(control.bounds.center()).unwrap().action_id,
                        control.action_id
                    );
                }
            }
        }
    }
}

#[test]
fn validated_capacity_navigation_frames_stay_below_renderer_limits() {
    let current = maximum_reference_snapshot();
    let mut maxima = (0, 0, String::new(), String::new());
    for (width, height, scale) in SDF_QUALIFICATION_MATRIX {
        let layout = WorkshopLayout::resolve(Vec2::new(width, height), scale).unwrap();
        for high_contrast in [false, true] {
            let model = WorkshopUiModel::build(
                &current,
                WorkshopUiContext {
                    high_contrast,
                    selected_entity: current.state.worlds.keys().next().copied(),
                    ..WorkshopUiContext::default()
                },
            );
            for action in [
                None,
                Some(WorkshopViewAction::OpenCreator),
                Some(WorkshopViewAction::OpenNavigator),
                Some(WorkshopViewAction::ShowStatus),
            ] {
                let mut view = WorkshopViewState::default();
                if let Some(action) = action {
                    view.apply(action, &model, &layout);
                }
                let frame =
                    build_workshop_platform_frame_for_view(&model, layout.clone(), &view, None);
                qualify_sdf_frame(&model, &frame);
                let output =
                    build_platform_ui_batch(&frame, &AtlasMetrics::embedded().unwrap()).unwrap();
                let state = format!(
                    "64 systems/128 stars/512 worlds,64char names,{width}x{height}@{scale},contrast={high_contrast},action={action:?}"
                );
                if output.batch.glyphs().len() > maxima.0 {
                    maxima.0 = output.batch.glyphs().len();
                    maxima.2.clone_from(&state);
                }
                if output.batch.panels().len() > maxima.1 {
                    maxima.1 = output.batch.panels().len();
                    maxima.3 = state;
                }
            }
        }
    }
    eprintln!("VALIDATED CAPACITY MAX {maxima:?}");
}

#[test]
fn shell_chrome_qualifies_at_every_required_viewport_and_scale() {
    use nyon::{
        app::client_runtime::{ClientScreen, MainMenuCapability, MainMenuRoute},
        preferences::{UiScale, UserPreferencesV1},
        ui::platform::{ShellPlatformInput, build_shell_platform_frame},
    };
    let model = WorkshopUiModel::build(&snapshot(), WorkshopUiContext::default());
    let capabilities = [
        MainMenuRoute::NewWorkshop,
        MainMenuRoute::Continue,
        MainMenuRoute::ClassicSector,
        MainMenuRoute::Settings,
        MainMenuRoute::Credits,
    ]
    .map(|route| MainMenuCapability {
        route,
        enabled: true,
    });
    for (width, height, scale) in SDF_QUALIFICATION_MATRIX {
        for high_contrast in [false, true] {
            for (screen, credits_visible) in [
                (ClientScreen::MainMenu, false),
                (ClientScreen::Settings, false),
                (ClientScreen::MainMenu, true),
                (ClientScreen::RecoverableError, false),
            ] {
                let frame = build_shell_platform_frame(ShellPlatformInput {
                    screen,
                    capabilities: &capabilities,
                    credits_visible,
                    recovery_message: Some("Recovery available"),
                    continue_available: true,
                    backend: Some(BackendKind::WebGl2Low),
                    preferences: UserPreferencesV1 {
                        ui_scale: UiScale::try_from((scale * 100.0).round() as u16).unwrap(),
                        high_contrast,
                        ..UserPreferencesV1::default()
                    },
                    viewport: Vec2::new(width, height),
                    focused: None,
                });
                qualify_sdf_frame(&model, &frame);
            }
        }
    }
}

#[test]
fn wide_outliner_reveal_materializes_actual_first_middle_and_last_systems() {
    let current = sixty_four_system_snapshot();
    let model = WorkshopUiModel::build(&current, WorkshopUiContext::default());
    assert_eq!(model.outliner.len(), 64);
    let metrics = AtlasMetrics::embedded().unwrap();

    for scale in [1.0, 1.3] {
        let layout = WorkshopLayout::resolve(Vec2::new(1920.0, 1080.0), scale).unwrap();
        for index in [0, model.outliner.len() / 2, model.outliner.len() - 1] {
            let target = &model.outliner[index].control.action_id;
            let mut view = WorkshopViewState::default();
            assert!(view.reveal_action(&model, &layout, target));

            for rebuild in 0..2 {
                assert!(view.reveal_action(&model, &layout, target));
                let frame = build_workshop_platform_frame_for_view(
                    &model,
                    layout.clone(),
                    &view,
                    Some(target),
                );
                let control = frame
                    .controls
                    .iter()
                    .find(|control| &control.action_id == target)
                    .unwrap_or_else(|| {
                        panic!(
                            "wide outliner index {index} disappeared at {scale} on rebuild {rebuild}; start={}",
                            view.outliner_start
                        )
                    });
                assert!(frame.action(target).is_some());
                assert_eq!(
                    frame
                        .hit_test(control.bounds.center())
                        .map(|hit| &hit.action_id),
                    Some(target)
                );
                assert!(
                    frame
                        .semantics
                        .node(control.semantic_id.as_str())
                        .is_some_and(|node| node.visible
                            && node.enabled
                            && node.action_id.as_ref() == Some(target))
                );
                let output = build_platform_ui_batch(&frame, &metrics).unwrap();
                assert!(output.text_runs.iter().any(|run| {
                    run.semantic_id == control.semantic_id
                        && run.emitted_glyph_range[1] > run.emitted_glyph_range[0]
                }));
            }
        }
    }
}

#[test]
fn shell_actions_use_the_same_exact_label_and_typed_icon_contract() {
    use nyon::{
        app::client_runtime::{ClientScreen, MainMenuCapability, MainMenuRoute},
        preferences::UserPreferencesV1,
        ui::platform::{ShellPlatformInput, build_shell_platform_frame},
    };
    let model = WorkshopUiModel::build(&snapshot(), WorkshopUiContext::default());
    let capabilities = [
        MainMenuRoute::NewWorkshop,
        MainMenuRoute::Continue,
        MainMenuRoute::ClassicSector,
        MainMenuRoute::Settings,
        MainMenuRoute::Credits,
    ]
    .map(|route| MainMenuCapability {
        route,
        enabled: true,
    });
    for screen in [
        ClientScreen::MainMenu,
        ClientScreen::Settings,
        ClientScreen::RecoverableError,
    ] {
        for credits_visible in [false, true] {
            let frame = build_shell_platform_frame(ShellPlatformInput {
                screen,
                capabilities: &capabilities,
                credits_visible,
                recovery_message: Some("Recovery available"),
                continue_available: true,
                backend: None,
                preferences: UserPreferencesV1::default(),
                viewport: Vec2::new(800.0, 600.0),
                focused: None,
            });
            assert_action_witnesses(&model, &frame);
        }
    }
}

#[test]
fn workshop_layout_preserves_a_real_canvas_at_supported_sizes() {
    for (viewport, scale, expected) in [
        (Vec2::new(723.0, 802.0), 1.15, WorkshopLayoutMode::Compact),
        (Vec2::new(1280.0, 720.0), 1.30, WorkshopLayoutMode::Medium),
        (Vec2::new(1280.0, 720.0), 1.00, WorkshopLayoutMode::Wide),
        (Vec2::new(1440.0, 900.0), 1.00, WorkshopLayoutMode::Wide),
    ] {
        let layout = WorkshopLayout::resolve(viewport, scale).unwrap();
        assert_eq!(layout.mode, expected);
        assert!(layout.canvas.width() >= 320.0, "{layout:?}");
        assert!(layout.canvas.height() >= 300.0, "{layout:?}");
        assert!(
            layout
                .chrome
                .iter()
                .all(|rect| !rect.overlaps(layout.canvas)),
            "{layout:?}"
        );
        assert!(
            layout
                .chrome
                .iter()
                .all(|rect| layout.viewport.contains_rect(*rect)),
            "{layout:?}"
        );
    }
}

#[test]
fn workshop_layout_uses_the_exact_effective_width_boundaries() {
    for (effective_width, expected) in [
        (899.0, WorkshopLayoutMode::Compact),
        (900.0, WorkshopLayoutMode::Medium),
        (1199.0, WorkshopLayoutMode::Medium),
        (1200.0, WorkshopLayoutMode::Wide),
    ] {
        let layout = WorkshopLayout::resolve(Vec2::new(effective_width, 900.0), 1.0).unwrap();
        assert_eq!(layout.mode, expected, "effective width {effective_width}");
    }

    let compact = WorkshopLayout::resolve(Vec2::new(723.0, 802.0), 1.30).unwrap();
    assert_eq!(compact.canvas.min.x, 0.0);
    assert_eq!(compact.canvas.max.x, 723.0);
    assert_eq!(compact.left_panel, None);
    assert_eq!(compact.right_panel, None);

    let medium = WorkshopLayout::resolve(Vec2::new(1280.0, 720.0), 1.30).unwrap();
    assert_eq!(medium.mode, WorkshopLayoutMode::Medium);
    assert!(medium.left_panel.unwrap().width() >= 44.0);
    assert!(medium.left_panel.unwrap().width() < 100.0);
}

#[test]
fn medium_layout_uses_a_collapsed_rail_and_one_click_navigator_overlay() {
    let model = WorkshopUiModel::build(&snapshot(), WorkshopUiContext::default());
    let layout = WorkshopLayout::resolve(Vec2::new(1280.0, 720.0), 1.30).unwrap();
    assert_eq!(layout.mode, WorkshopLayoutMode::Medium);
    let rail = layout.left_panel.unwrap();

    let closed = build_workshop_platform_frame_for_view(
        &model,
        layout.clone(),
        &WorkshopViewState::default(),
        None,
    );
    assert_eq!(closed.drawer, None);
    let open = closed
        .controls
        .iter()
        .find(|control| control.action_id == WorkshopViewAction::OpenNavigator.action_id())
        .expect("medium navigator rail needs one persistent open action");
    assert!(rail.contains_rect(open.bounds));
    assert!(open.bounds.width() >= 44.0 && open.bounds.height() >= 44.0);
    let navigator_label = closed
        .visible_nodes
        .iter()
        .find(|record| {
            record.action_id.as_ref() == Some(&open.action_id)
                && record.icon.is_none()
                && record.display_text == "Navigator"
        })
        .expect("the icon rail needs a persistent sighted focus/help label");
    assert!(closed.layout.top_bar.contains_rect(navigator_label.bounds));
    assert!(navigator_label.bounds.width() >= 100.0);

    let mut view = WorkshopViewState::default();
    view.apply(WorkshopViewAction::OpenNavigator, &model, &layout);
    let expanded = build_workshop_platform_frame_for_view(&model, layout.clone(), &view, None);
    assert_eq!(expanded.drawer, Some(layout.drawer_sheet));
    assert!(expanded.drawer.unwrap().overlaps(layout.canvas));
    assert!(expanded.controls.iter().any(|control| {
        control.action_id == WorkshopViewAction::CloseDrawer.action_id()
            && control.bounds.width() >= 44.0
            && control.bounds.height() >= 44.0
    }));
}

#[test]
fn workshop_frame_never_draws_a_full_viewport_overlay() {
    for high_contrast in [false, true] {
        let mut model = WorkshopUiModel::build(
            &snapshot(),
            WorkshopUiContext {
                high_contrast,
                ..WorkshopUiContext::default()
            },
        );
        model.preferences.high_contrast = high_contrast;
        let viewport = Vec2::new(1280.0, 720.0);
        let layout = WorkshopLayout::resolve(viewport, 1.0).unwrap();
        let frame = build_workshop_platform_frame_with_layout(&model, layout, None);
        assert_eq!(frame.background, PlatformBackground::ChromeOnly);
        assert!(
            frame
                .layout
                .chrome
                .iter()
                .all(|rect| *rect != frame.layout.viewport && !rect.overlaps(frame.layout.canvas))
        );

        let mut batch = PrimitiveBatch::default();
        frame.draw(&mut batch);
        assert!(!contains_full_viewport_quad(&batch, viewport));
    }
}

#[test]
fn responsive_workshop_chrome_reserves_controls_from_sighted_text() {
    let model = WorkshopUiModel::build(&snapshot(), WorkshopUiContext::default());
    for (viewport, scale, expected_mode) in [
        (Vec2::new(723.0, 802.0), 1.15, WorkshopLayoutMode::Compact),
        (Vec2::new(723.0, 802.0), 1.30, WorkshopLayoutMode::Compact),
        (Vec2::new(816.0, 802.0), 1.00, WorkshopLayoutMode::Compact),
        (Vec2::new(1280.0, 720.0), 1.30, WorkshopLayoutMode::Medium),
    ] {
        let layout = WorkshopLayout::resolve(viewport, scale).unwrap();
        assert_eq!(layout.mode, expected_mode);
        let frame = build_workshop_platform_frame_for_view(
            &model,
            layout,
            &WorkshopViewState::default(),
            None,
        );
        let expected_actions = [
            WorkshopViewAction::OpenCreator.action_id(),
            WorkshopViewAction::OpenNavigator.action_id(),
            model.menu_control.action_id.clone(),
            nyon::ui::accessibility::SemanticActionId::new("guide.open"),
        ];
        let reserved_controls = expected_actions
            .iter()
            .map(|action| {
                frame
                    .controls
                    .iter()
                    .find(|control| &control.action_id == action)
                    .unwrap_or_else(|| panic!("missing responsive control {action}"))
            })
            .collect::<Vec<_>>();

        assert!(frame.title_bounds().is_none());
        if expected_mode == WorkshopLayoutMode::Compact {
            assert!(frame.sighted_text_bounds().is_empty());
        } else {
            let right = frame.layout.right_panel.unwrap();
            assert!(!frame.sighted_text.is_empty());
            assert!(
                frame
                    .sighted_text
                    .iter()
                    .all(|text| right.contains_rect(text.bounds))
            );
        }
        for (index, control) in reserved_controls.iter().enumerate() {
            assert!(frame.layout.viewport.contains_rect(control.bounds));
            assert!(
                reserved_controls[index + 1..]
                    .iter()
                    .all(|other| !control.bounds.overlaps(other.bounds))
            );
        }
    }

    let wide_layout = WorkshopLayout::resolve(Vec2::new(1280.0, 720.0), 1.0).unwrap();
    let wide = build_workshop_platform_frame_for_view(
        &model,
        wide_layout,
        &WorkshopViewState::default(),
        None,
    );
    let title = wide
        .title_bounds()
        .expect("Wide Workshop title remains sighted");
    assert!(title.width() > 0.0 && wide.layout.top_bar.contains_rect(title));
    assert!(
        wide.controls
            .iter()
            .filter(|control| {
                control.action_id == model.menu_control.action_id
                    || control.action_id.as_str() == "guide.open"
            })
            .all(|control| !title.overlaps(control.bounds))
    );
}

#[test]
fn compact_drawers_reveal_every_creator_tool_and_populated_outliner_row() {
    let model = WorkshopUiModel::build(&snapshot(), WorkshopUiContext::default());
    for scale in [1.15, 1.30] {
        let layout = WorkshopLayout::resolve(Vec2::new(723.0, 802.0), scale).unwrap();
        assert_eq!(layout.mode, WorkshopLayoutMode::Compact);

        let closed = build_workshop_platform_frame_for_view(
            &model,
            layout.clone(),
            &WorkshopViewState::default(),
            None,
        );
        assert!(
            closed.controls.iter().all(|control| {
                control.bounds.width() >= 44.0 && control.bounds.height() >= 44.0
            })
        );
        for action in [
            WorkshopViewAction::OpenCreator,
            WorkshopViewAction::OpenNavigator,
        ] {
            let id = action.action_id();
            let control = closed
                .controls
                .iter()
                .find(|control| control.action_id == id)
                .unwrap_or_else(|| panic!("missing persistent action {id}"));
            assert!(control.bounds.width() >= 44.0);
            assert!(control.bounds.height() >= 44.0);
        }

        for expected in model
            .tool_palette
            .iter()
            .map(|tool| &tool.control.action_id)
        {
            let mut view = WorkshopViewState::default();
            assert!(view.reveal_action(&model, &layout, expected));
            assert_eq!(view.open_drawer, Some(WorkshopDrawer::Creator));
            let frame = build_workshop_platform_frame_for_view(
                &model,
                layout.clone(),
                &view,
                Some(expected),
            );
            let control = frame
                .controls
                .iter()
                .find(|control| &control.action_id == expected)
                .unwrap_or_else(|| panic!("creator action {expected} was not materialized"));
            assert!(control.bounds.width() >= 44.0);
            assert!(control.bounds.height() >= 44.0);
        }

        assert!(!model.outliner.is_empty());
        for expected in model.outliner.iter().map(|entry| &entry.control.action_id) {
            let mut view = WorkshopViewState::default();
            assert!(view.reveal_action(&model, &layout, expected));
            assert_eq!(view.open_drawer, Some(WorkshopDrawer::Navigator));
            let frame = build_workshop_platform_frame_for_view(
                &model,
                layout.clone(),
                &view,
                Some(expected),
            );
            assert!(
                frame
                    .controls
                    .iter()
                    .any(|control| &control.action_id == expected),
                "outliner action {expected} was not materialized"
            );
        }

        let mut view = WorkshopViewState::default();
        view.apply(WorkshopViewAction::OpenCreator, &model, &layout);
        let creator = build_workshop_platform_frame_for_view(&model, layout.clone(), &view, None);
        assert!(creator.drawer.is_some());
        assert!(creator.controls.iter().any(|control| {
            control.action_id == WorkshopViewAction::CloseDrawer.action_id()
                && control.bounds.width() >= 44.0
                && control.bounds.height() >= 44.0
        }));
        assert!(creator.controls.iter().any(|control| {
            control.action_id == WorkshopViewAction::OpenNavigator.action_id()
                && control.bounds.width() >= 44.0
                && control.bounds.height() >= 44.0
        }));
        view.apply(WorkshopViewAction::OpenNavigator, &model, &layout);
        assert_eq!(view.open_drawer, Some(WorkshopDrawer::Navigator));

        let navigator = build_workshop_platform_frame_for_view(&model, layout, &view, None);
        for control in &navigator.controls {
            assert!(
                navigator.layout.viewport.contains_rect(control.bounds),
                "{} is outside the viewport at {scale}: {:?}",
                control.action_id.as_str(),
                control.bounds
            );
        }
    }
}

#[test]
fn visible_controls_share_draw_hit_focus_and_semantic_geometry() {
    let model = WorkshopUiModel::build(&snapshot(), WorkshopUiContext::default());
    let layout = WorkshopLayout::resolve(Vec2::new(723.0, 802.0), 1.30).unwrap();
    let mut view = WorkshopViewState::default();
    view.apply(WorkshopViewAction::OpenNavigator, &model, &layout);
    let frame = build_workshop_platform_frame_for_view(&model, layout, &view, None);

    for control in &frame.controls {
        if control.enabled {
            assert_eq!(
                frame
                    .hit_test(control.bounds.center())
                    .map(|hit| &hit.action_id),
                Some(&control.action_id)
            );
        }
        assert!(frame.focus_order().contains(&control.action_id));
        let semantic = frame
            .semantics
            .nodes_depth_first()
            .into_iter()
            .find(|node| node.action_id.as_ref() == Some(&control.action_id))
            .unwrap_or_else(|| panic!("missing semantic action {}", control.action_id));
        assert!(semantic.visible);
        assert_eq!(semantic.bounds, Some(control.bounds.into()));
    }
    for expected in model.focus_order() {
        assert!(frame.focus_order().contains(expected));

        let mut revealed = WorkshopViewState::default();
        assert!(
            revealed.reveal_action(&model, &frame.layout, expected),
            "logical focus action {expected} has no reveal behavior"
        );
        let revealed_frame = build_workshop_platform_frame_for_view(
            &model,
            frame.layout.clone(),
            &revealed,
            Some(expected),
        );
        assert!(
            revealed_frame
                .controls
                .iter()
                .any(|control| &control.action_id == expected),
            "logical focus action {expected} was not materialized"
        );
    }
}

#[test]
fn populated_workshop_markers_are_not_covered_by_opaque_chrome() {
    for high_contrast in [false, true] {
        let model = WorkshopUiModel::build(
            &snapshot(),
            WorkshopUiContext {
                high_contrast,
                ..WorkshopUiContext::default()
            },
        );
        let scene = nyon::presentation::workshop::build_workshop_scene_frame(
            &snapshot().state,
            None,
            high_contrast,
        );
        for (viewport, scale) in [
            (Vec2::new(1280.0, 720.0), 1.0),
            (Vec2::new(723.0, 802.0), 1.15),
            (Vec2::new(723.0, 802.0), 1.30),
        ] {
            let layout = WorkshopLayout::resolve(viewport, scale).unwrap();
            let mut underlay = PrimitiveBatch::default();
            let coverage = draw_workshop_scene(&scene, &layout, &mut underlay, high_contrast);
            assert!(!coverage.world_markers.is_empty());
            assert!(!underlay.vertices().is_empty());

            let frame = build_workshop_platform_frame_for_view(
                &model,
                layout,
                &WorkshopViewState::default(),
                None,
            );
            let mut overlay = PrimitiveBatch::default();
            frame.draw(&mut overlay);
            for marker in coverage.world_markers {
                assert!(
                    !opaque_quad_covers(&overlay, marker),
                    "opaque Workshop chrome covered world marker {marker:?} at {viewport:?}"
                );
            }
        }
    }
}

#[test]
fn workshop_scene_draws_distinct_system_star_world_layers_and_selected_star_ring() {
    let state = populated_state();
    let selected_star = entity(3);
    let mut contrast_witnesses = Vec::new();
    for high_contrast in [false, true] {
        let scene = nyon::presentation::workshop::build_workshop_scene_frame(
            &state,
            Some(selected_star),
            high_contrast,
        );
        let layout = WorkshopLayout::resolve(Vec2::new(1280.0, 720.0), 1.0).unwrap();
        let mut batch = PrimitiveBatch::default();
        let coverage = draw_workshop_scene(&scene, &layout, &mut batch, high_contrast);

        assert_eq!(coverage.system_markers.len(), state.systems.len());
        assert_eq!(coverage.star_markers.len(), state.stars.len());
        assert_eq!(coverage.world_markers.len(), state.worlds.len());
        assert!(
            coverage
                .marker_witnesses
                .windows(2)
                .all(|pair| marker_layer_rank(pair[0].layer) <= marker_layer_rank(pair[1].layer))
        );
        for witness in &coverage.marker_witnesses {
            assert_marker_witness(&batch, witness);
        }
        for witness in coverage.marker_witnesses.iter().filter(|witness| {
            matches!(
                witness.layer,
                WorkshopMarkerLayer::StarHalo
                    | WorkshopMarkerLayer::StarCore
                    | WorkshopMarkerLayer::World
            )
        }) {
            assert_eq!(witness.packed_shape, 0xffff_ff02);
            assert_eq!(witness.ring_thickness, Some(witness.radius));
        }

        for (source_index, star) in scene.stars.iter().enumerate() {
            let star_witnesses = coverage
                .marker_witnesses
                .iter()
                .filter(|witness| {
                    witness.source_index == source_index
                        && matches!(
                            witness.layer,
                            WorkshopMarkerLayer::StarHalo
                                | WorkshopMarkerLayer::StarCore
                                | WorkshopMarkerLayer::StarSelectionRing
                        )
                })
                .collect::<Vec<_>>();
            let expected_count = if star.flags[0] == 1 { 3 } else { 2 };
            assert_eq!(star_witnesses.len(), expected_count);
            assert_eq!(star_witnesses[0].layer, WorkshopMarkerLayer::StarHalo);
            assert_eq!(star_witnesses[1].layer, WorkshopMarkerLayer::StarCore);
            assert!(star_witnesses[0].radius > star_witnesses[1].radius);
            assert!(star_witnesses.iter().all(|witness| {
                witness.center.is_finite()
                    && witness.radius.is_finite()
                    && witness.radius > 0.0
                    && witness.color.iter().all(|value| value.is_finite())
            }));
            assert_eq!(star_witnesses[0].color[3], 0.28);
            assert_eq!(star_witnesses[1].color[3], 1.0);
            assert!(luminance(star_witnesses[1].color) > 0.20);
            if expected_count == 3 {
                assert_eq!(
                    star_witnesses[2].layer,
                    WorkshopMarkerLayer::StarSelectionRing
                );
                assert!(star_witnesses[2].radius > star_witnesses[0].radius);
            }
            let system_ring = coverage
                .marker_witnesses
                .iter()
                .find(|witness| {
                    witness.layer == WorkshopMarkerLayer::SystemRing
                        && witness.center == star_witnesses[0].center
                })
                .expect("each star must retain its enclosing system ring");
            assert!(
                system_ring.radius
                    > star_witnesses
                        .iter()
                        .map(|witness| witness.radius)
                        .fold(0.0_f32, f32::max)
            );
        }

        let selected_witnesses = coverage
            .marker_witnesses
            .iter()
            .filter(|witness| witness.layer == WorkshopMarkerLayer::StarSelectionRing)
            .count();
        assert_eq!(selected_witnesses, 1);
        assert!(
            batch
                .vertices()
                .iter()
                .all(|vertex| Vec2::from_array(vertex.position).is_finite())
        );
        contrast_witnesses.push(coverage.marker_witnesses);
    }

    let [normal_witnesses, high_contrast_witnesses] = contrast_witnesses.as_slice() else {
        panic!("both presentation modes must be retained for comparison");
    };
    for layer in [
        WorkshopMarkerLayer::StarHalo,
        WorkshopMarkerLayer::StarCore,
        WorkshopMarkerLayer::StarSelectionRing,
    ] {
        for normal in normal_witnesses
            .iter()
            .filter(|witness| witness.layer == layer)
        {
            let high_contrast = matching_witness(high_contrast_witnesses, normal);
            assert_eq!(high_contrast.center, normal.center);
            assert!(
                high_contrast.radius > normal.radius,
                "high contrast must enlarge {layer:?} geometry"
            );
        }
    }
    for layer in [
        WorkshopMarkerLayer::SystemRing,
        WorkshopMarkerLayer::StarSelectionRing,
    ] {
        for normal in normal_witnesses
            .iter()
            .filter(|witness| witness.layer == layer)
        {
            let high_contrast = matching_witness(high_contrast_witnesses, normal);
            assert!(high_contrast.ring_thickness.unwrap() > normal.ring_thickness.unwrap());
            assert_ne!(high_contrast.packed_shape, normal.packed_shape);
        }
    }

    let unselected = nyon::presentation::workshop::build_workshop_scene_frame(&state, None, false);
    let selected = nyon::presentation::workshop::build_workshop_scene_frame(
        &state,
        Some(selected_star),
        false,
    );
    let layout = WorkshopLayout::resolve(Vec2::new(1280.0, 720.0), 1.0).unwrap();
    let mut unselected_batch = PrimitiveBatch::default();
    draw_workshop_scene(&unselected, &layout, &mut unselected_batch, false);
    let mut selected_batch = PrimitiveBatch::default();
    draw_workshop_scene(&selected, &layout, &mut selected_batch, false);
    assert_eq!(
        selected_batch.vertices().len(),
        unselected_batch.vertices().len() + 6,
        "selection must add one geometric ring rather than recolor the star"
    );
}

/// Baseline review Finding 5, the timeline half: at the declared capacity the
/// tail redo choices had no pointer path in any layout.
///
/// `view.timeline_start` was written in exactly one place, inside
/// `reveal_action`, which is the keyboard model. No `WorkshopViewAction`
/// advanced it, so a pointer-only user could never reach an overflowing redo
/// choice. Branches were already reachable by pointer through
/// `OpenNavigator` then `ShowBranches` then `ScrollNext`; the timeline was not.
///
/// This drives the frame the way a pointer does: hit-test a control, read the
/// typed action off it, apply that, rebuild. Nothing here calls
/// `reveal_action`.
#[test]
fn overflowing_redo_choices_are_reachable_by_pointer_alone() {
    use nyon::ui::platform::PlatformUiAction;
    use nyon_workshop_core::ids::BatchLocalId;
    use nyon_workshop_core::ids::GalaxyPointV1;
    use nyon_workshop_core::ids::WorkshopTick;
    use nyon_workshop_core::{CreatorBatchV1, CreatorOpV1, WorkshopHistory};

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
    assert_eq!(redo.len(), 24, "the fixture must overflow the bottom bar");

    let session = nyon::workshop::session::WorkshopSession::new(history);
    let model = WorkshopUiModel::build(
        session.snapshot(),
        WorkshopUiContext {
            redo_children: &redo,
            ..WorkshopUiContext::default()
        },
    );
    let last = model
        .timeline
        .controls
        .last()
        .expect("the timeline always carries controls")
        .action_id
        .clone();

    for (width, height, scale) in SDF_QUALIFICATION_MATRIX {
        let layout = WorkshopLayout::resolve(Vec2::new(width, height), scale).unwrap();
        let mut view = WorkshopViewState::default();

        let visible = |view: &WorkshopViewState| {
            build_workshop_platform_frame_for_view(&model, layout.clone(), view, None)
                .controls
                .iter()
                .any(|control| control.action_id == last)
        };
        assert!(
            !visible(&view),
            "fixture assumes the tail redo choice starts off screen at {width}x{height}@{scale}"
        );

        // Advance only through controls the frame actually offers, activated
        // through their own hit-test rectangles, with a bound well above the
        // 24 steps a one-per-press scroll needs.
        let mut reached = false;
        for _ in 0..64 {
            let frame = build_workshop_platform_frame_for_view(&model, layout.clone(), &view, None);
            let Some(action) = frame
                .controls
                .iter()
                .find(|control| {
                    control.enabled
                        && control.action
                            == PlatformUiAction::WorkshopView(
                                WorkshopViewAction::ScrollTimelineNext,
                            )
                })
                .and_then(|control| frame.hit_test(control.bounds.center()))
                .map(|control| control.action.clone())
            else {
                break;
            };
            let PlatformUiAction::WorkshopView(action) = action else {
                unreachable!("the timeline scroll control carries a view action");
            };
            view.apply(action, &model, &layout);
            if visible(&view) {
                reached = true;
                break;
            }
        }
        assert!(
            reached,
            "no pointer path reaches the tail redo choice at {width}x{height}@{scale}"
        );

        // The reverse control must exist once scrolled, or the user is stranded.
        let frame = build_workshop_platform_frame_for_view(&model, layout.clone(), &view, None);
        assert!(
            frame.controls.iter().any(|control| {
                control.action
                    == PlatformUiAction::WorkshopView(WorkshopViewAction::ScrollTimelinePrevious)
            }),
            "no pointer path back at {width}x{height}@{scale}"
        );
    }
}
