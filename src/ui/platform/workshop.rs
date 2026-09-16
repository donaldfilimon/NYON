//! Workshop platform frame building and scene drawing, split out of `platform.rs`.

use super::{
    PlatformBackground, PlatformControl, PlatformIconAction, PlatformRect, PlatformUiAction,
    PlatformUiFrame, apply_platform_control_geometry, icon_for_action, push_guide_control,
    safe_viewport,
};
use crate::engine::primitives::PrimitiveBatch;
use crate::presentation::workshop::WorkshopSceneFrame;
use crate::ui::accessibility::SemanticActionId;
use crate::ui::accessibility::SemanticNode;
use crate::ui::accessibility::SemanticNodeId;
use crate::ui::accessibility::SemanticRole;
use crate::ui::platform_inspector::apply_platform_sighted_text_geometry;
use crate::ui::platform_inspector::build_inspector_sighted_text;
use crate::ui::platform_projection::build_source_records;
use crate::ui::platform_projection::{modal_action_ids, modal_presentation};
use crate::ui::workshop::WorkshopUiModel;
use crate::ui::workshop::{CREATOR_SUBMIT_ACTION, REMOVAL_CONFIRM_ACTION, WorkshopControl};
use crate::ui::workshop_layout::WorkshopLayout;
use crate::ui::workshop_layout::WorkshopLayoutMode;
use crate::ui::workshop_view::NavigatorSection;
use crate::ui::workshop_view::WorkshopDrawer;
use crate::ui::workshop_view::WorkshopViewAction;
use crate::ui::workshop_view::WorkshopViewState;
use glam::Vec2;

pub fn build_workshop_platform_frame(
    model: &WorkshopUiModel,
    viewport: Vec2,
    focused: Option<&SemanticActionId>,
) -> PlatformUiFrame {
    let viewport = safe_viewport(viewport);
    let layout =
        WorkshopLayout::resolve(viewport, 1.0).expect("safe Workshop viewport must resolve");
    build_workshop_platform_frame_with_layout(model, layout, focused)
}

pub fn build_workshop_platform_frame_with_layout(
    model: &WorkshopUiModel,
    layout: WorkshopLayout,
    focused: Option<&SemanticActionId>,
) -> PlatformUiFrame {
    build_workshop_platform_frame_for_view(model, layout, &WorkshopViewState::default(), focused)
}

pub fn build_workshop_platform_frame_for_view(
    model: &WorkshopUiModel,
    layout: WorkshopLayout,
    view: &WorkshopViewState,
    focused: Option<&SemanticActionId>,
) -> PlatformUiFrame {
    let viewport = layout.viewport.max;
    let scale = layout.ui_scale;
    let mut controls = Vec::new();
    push_workshop_control(
        &mut controls,
        &model.menu_control,
        PlatformRect::from_xywh(
            (layout.top_bar.max.x - 154.0 * scale).max(layout.top_bar.min.x + 8.0),
            layout.top_bar.min.y + (layout.top_bar.height() - 44.0) * 0.5,
            146.0 * scale,
            44.0,
        ),
        focused,
    );

    if crate::ui::workshop_view::wide_navigation_fits(&layout) {
        let left = layout
            .left_panel
            .expect("wide Workshop has a navigator panel");
        let gap = 4.0;
        let tool_width = ((left.width() - 16.0 - gap) * 0.5).max(44.0);
        let tool_height = 44.0;
        let tools_top = left.min.y + 318.0;
        for (visible_index, item) in model.tool_palette.iter().enumerate() {
            let column = visible_index % 2;
            let row = visible_index / 2;
            push_workshop_control(
                &mut controls,
                &item.control,
                PlatformRect::from_xywh(
                    left.min.x + 8.0 + column as f32 * (tool_width + gap),
                    tools_top + row as f32 * 48.0,
                    tool_width,
                    tool_height,
                ),
                focused,
            );
        }
        let projection =
            crate::ui::workshop_view::wide_outliner_projection(&layout, model.tool_palette.len())
                .expect("roomy Wide navigation has a resolved outliner projection");
        let window = view.outliner_window(model, &layout);
        for (visible_index, index) in window.range().enumerate() {
            let entry = &model.outliner[index];
            let indent = entry.depth as f32 * 12.0;
            push_workshop_control(
                &mut controls,
                &entry.control,
                PlatformRect::from_xywh(
                    left.min.x + 8.0 + indent,
                    projection.top + visible_index as f32 * projection.row_stride,
                    (left.width() - 16.0 - indent).max(44.0),
                    44.0,
                ),
                focused,
            );
        }
    } else {
        let button_height = 44.0;
        let button_y = if layout.mode == WorkshopLayoutMode::Wide {
            layout.left_panel.unwrap().min.y + 8.0
        } else {
            layout.top_bar.min.y + (layout.top_bar.height() - button_height) * 0.5
        };
        push_view_control(
            &mut controls,
            WorkshopViewAction::OpenCreator,
            "Create",
            PlatformRect::from_xywh(8.0, button_y, 108.0, button_height),
            focused,
            view.open_drawer == Some(WorkshopDrawer::Creator),
        );
        push_view_control(
            &mut controls,
            WorkshopViewAction::OpenNavigator,
            "Navigator",
            if layout.mode == WorkshopLayoutMode::Medium {
                let rail = layout
                    .left_panel
                    .expect("medium Workshop has a navigator rail");
                PlatformRect::from_xywh(rail.center().x - 22.0, rail.min.y + 8.0, 44.0, 44.0)
            } else {
                PlatformRect::from_xywh(122.0, button_y, 118.0, button_height)
            },
            focused,
            view.open_drawer == Some(WorkshopDrawer::Navigator),
        );
    }

    let mut timeline_x = layout.bottom_bar.min.x + 8.0;
    let save_width = if layout.mode == WorkshopLayoutMode::Compact {
        112.0
    } else {
        0.0
    };
    let timeline_limit = layout.bottom_bar.max.x - 8.0 - save_width;
    let timeline_control_width = |control: &crate::ui::workshop::WorkshopControl| {
        if control.action_id.as_str().contains("redo") {
            112.0 * scale
        } else {
            82.0 * scale
        }
    };
    // Probe first: the scroll pair only earns its space when the strip cannot
    // show everything from the current start, or when the user has already
    // scrolled and needs a way back.
    let remaining = model
        .timeline
        .controls
        .len()
        .saturating_sub(view.timeline_start);
    let mut probe_x = timeline_x;
    let mut fitted = 0_usize;
    for control in model.timeline.controls.iter().skip(view.timeline_start) {
        let width = timeline_control_width(control);
        if probe_x + width > timeline_limit {
            break;
        }
        probe_x += width + 4.0 * scale;
        fitted += 1;
    }
    let timeline_overflows = view.timeline_start > 0 || fitted < remaining;
    // Wide enough for the fixed label: the SDF batch refuses a control whose
    // label cannot be laid out inside its box, so a 44-square button here is
    // rejected rather than truncated.
    let scroll_width = 82.0 * scale;
    let scroll_height = 44.0;
    let scroll_gap = 4.0;
    let strip_limit = if timeline_overflows {
        timeline_limit - (scroll_width * 2.0 + scroll_gap + 8.0)
    } else {
        timeline_limit
    };

    for control in model.timeline.controls.iter().skip(view.timeline_start) {
        let width = timeline_control_width(control);
        if timeline_x + width > strip_limit {
            break;
        }
        push_workshop_control(
            &mut controls,
            control,
            PlatformRect::from_xywh(
                timeline_x,
                layout.bottom_bar.min.y + (layout.bottom_bar.height() - 44.0) * 0.5,
                width,
                44.0,
            ),
            focused,
        );
        timeline_x += width + 4.0 * scale;
    }

    if timeline_overflows {
        let scroll_y = layout.bottom_bar.min.y + (layout.bottom_bar.height() - scroll_height) * 0.5;
        for (index, (action, label)) in [
            (WorkshopViewAction::ScrollTimelinePrevious, "Older"),
            (WorkshopViewAction::ScrollTimelineNext, "Newer"),
        ]
        .into_iter()
        .enumerate()
        {
            push_view_control(
                &mut controls,
                action,
                label,
                PlatformRect::from_xywh(
                    timeline_limit - scroll_width * 2.0 - scroll_gap
                        + index as f32 * (scroll_width + scroll_gap),
                    scroll_y,
                    scroll_width,
                    scroll_height,
                ),
                focused,
                false,
            );
        }
    }

    if let Some(right) = layout.right_panel {
        let right_x = right.min.x + 8.0;
        let branch_gap = 6.0;
        let branch_width = ((right.width() - 16.0 - branch_gap) * 0.5).max(44.0);
        let branch_capacity = crate::ui::workshop_view::docked_branch_capacity(&layout);
        let branch_window = crate::ui::virtual_list::VisibleWindow::new(
            model.branches.len(),
            branch_capacity,
            view.branch_start,
        );
        for (visible_index, index) in branch_window.range().enumerate() {
            let branch = &model.branches[index];
            push_workshop_control(
                &mut controls,
                &branch.control,
                PlatformRect::from_xywh(
                    right_x,
                    right.min.y + 12.0 + visible_index as f32 * (44.0 + 4.0),
                    branch_width,
                    44.0,
                ),
                focused,
            );
        }
        push_workshop_control(
            &mut controls,
            &model.save.save_control,
            PlatformRect::from_xywh(
                right_x + branch_width + branch_gap,
                right.min.y + 12.0,
                branch_width,
                44.0,
            ),
            focused,
        );
        if view.open_drawer.is_none() {
            for (index, (action, label)) in [
                (
                    WorkshopViewAction::ScrollInspectorPrevious,
                    "Inspector previous",
                ),
                (WorkshopViewAction::ScrollInspectorNext, "Inspector next"),
            ]
            .into_iter()
            .enumerate()
            {
                let gap = 4.0;
                let width = (right.width() - 16.0 - gap) * 0.5;
                push_view_control(
                    &mut controls,
                    action,
                    label,
                    PlatformRect::from_xywh(
                        right_x + index as f32 * (width + gap),
                        right.max.y - 148.0,
                        width,
                        44.0,
                    ),
                    focused,
                    false,
                );
            }
        }
        if let Some(remove) = &model.inspector.remove_control {
            push_workshop_control(
                &mut controls,
                remove,
                PlatformRect::from_xywh(
                    right_x,
                    right.max.y - 52.0,
                    (right.width() - 16.0).max(44.0),
                    44.0,
                ),
                focused,
            );
        }
        for (index, preference) in [
            &model.preferences.reduced_motion_control,
            &model.preferences.high_contrast_control,
        ]
        .into_iter()
        .enumerate()
        {
            push_workshop_control(
                &mut controls,
                preference,
                PlatformRect::from_xywh(
                    right_x + index as f32 * (((right.width() - 20.0) * 0.5).max(44.0) + 4.0),
                    right.max.y - 100.0,
                    ((right.width() - 20.0) * 0.5).max(44.0),
                    44.0,
                ),
                focused,
            );
        }
    } else {
        push_workshop_control(
            &mut controls,
            &model.save.save_control,
            PlatformRect::from_xywh(
                layout.bottom_bar.max.x - save_width - 8.0,
                layout.bottom_bar.min.y + (layout.bottom_bar.height() - 44.0) * 0.5,
                save_width,
                44.0,
            ),
            focused,
        );
    }

    let drawer = (!crate::ui::workshop_view::wide_navigation_fits(&layout)
        || view.navigator_section == NavigatorSection::Status)
        .then_some(view.open_drawer)
        .flatten()
        .map(|_| layout.drawer_sheet);
    if let (Some(drawer), Some(open_drawer)) = (drawer, view.open_drawer) {
        controls.retain(|control| !drawer.overlaps(control.bounds));
        push_view_control(
            &mut controls,
            WorkshopViewAction::CloseDrawer,
            "Close",
            PlatformRect::from_xywh(drawer.max.x - 108.0, drawer.min.y + 8.0, 100.0, 44.0),
            focused,
            false,
        );
        let body_top = drawer.min.y + 60.0;
        if open_drawer == WorkshopDrawer::Navigator {
            let tab_gap = 4.0;
            let columns = if drawer.width() < 480.0 { 2 } else { 4 };
            let tab_width =
                (drawer.width() - 24.0 - tab_gap * (columns - 1) as f32) / columns as f32;
            for (index, (action, label, selected)) in [
                (
                    WorkshopViewAction::ShowHierarchy,
                    "Hierarchy",
                    view.navigator_section == NavigatorSection::Hierarchy,
                ),
                (
                    WorkshopViewAction::ShowBranches,
                    "Branches",
                    view.navigator_section == NavigatorSection::Branches,
                ),
                (
                    WorkshopViewAction::ShowInspector,
                    "Inspector",
                    view.navigator_section == NavigatorSection::Inspector,
                ),
                (
                    WorkshopViewAction::ShowStatus,
                    "Status",
                    view.navigator_section == NavigatorSection::Status,
                ),
            ]
            .into_iter()
            .enumerate()
            {
                push_view_control(
                    &mut controls,
                    action,
                    label,
                    PlatformRect::from_xywh(
                        drawer.min.x + 8.0 + (index % columns) as f32 * (tab_width + tab_gap),
                        body_top + (index / columns) as f32 * 52.0,
                        tab_width,
                        44.0,
                    ),
                    focused,
                    selected,
                );
            }
        }
        let body_top = if open_drawer == WorkshopDrawer::Navigator {
            drawer.min.y + crate::ui::workshop_view::navigator_header_height(&layout)
        } else {
            body_top
        };
        let body_bottom = drawer.max.y - 56.0;
        match open_drawer {
            WorkshopDrawer::Creator => {
                let window = view.creator_window(model, &layout);
                let gap = 4.0;
                let width = (drawer.width() - 24.0 - gap) * 0.5;
                for (visible_index, index) in window.range().enumerate() {
                    let column = visible_index % 2;
                    let row = visible_index / 2;
                    let y = body_top + row as f32 * 48.0;
                    if y + 44.0 > body_bottom {
                        break;
                    }
                    push_workshop_control(
                        &mut controls,
                        &model.tool_palette[index].control,
                        PlatformRect::from_xywh(
                            drawer.min.x + 8.0 + column as f32 * (width + gap),
                            y,
                            width,
                            44.0,
                        ),
                        focused,
                    );
                }
            }
            WorkshopDrawer::Navigator => match view.navigator_section {
                NavigatorSection::Status => {}
                NavigatorSection::Hierarchy => {
                    let window = view.outliner_window(model, &layout);
                    for (visible_index, index) in window.range().enumerate() {
                        let entry = &model.outliner[index];
                        let indent = entry.depth as f32 * 12.0;
                        let y = body_top + visible_index as f32 * 48.0;
                        if y + 44.0 > body_bottom {
                            break;
                        }
                        push_workshop_control(
                            &mut controls,
                            &entry.control,
                            PlatformRect::from_xywh(
                                drawer.min.x + 8.0 + indent,
                                y,
                                (drawer.width() - 16.0 - indent).max(44.0),
                                44.0,
                            ),
                            focused,
                        );
                    }
                }
                NavigatorSection::Branches => {
                    let window = view.branch_window(model, &layout);
                    for (visible_index, index) in window.range().enumerate() {
                        let y = body_top + visible_index as f32 * 48.0;
                        if y + 44.0 > body_bottom {
                            break;
                        }
                        push_workshop_control(
                            &mut controls,
                            &model.branches[index].control,
                            PlatformRect::from_xywh(
                                drawer.min.x + 8.0,
                                y,
                                drawer.width() - 16.0,
                                44.0,
                            ),
                            focused,
                        );
                    }
                }
                NavigatorSection::Inspector => {
                    for (index, preference) in [
                        &model.preferences.reduced_motion_control,
                        &model.preferences.high_contrast_control,
                    ]
                    .into_iter()
                    .enumerate()
                    {
                        push_workshop_control(
                            &mut controls,
                            preference,
                            PlatformRect::from_xywh(
                                drawer.min.x
                                    + 8.0
                                    + index as f32 * ((drawer.width() - 20.0) * 0.5 + 4.0),
                                drawer.max.y - 148.0,
                                (drawer.width() - 20.0) * 0.5,
                                44.0,
                            ),
                            focused,
                        );
                    }
                    if let Some(remove) = &model.inspector.remove_control {
                        push_workshop_control(
                            &mut controls,
                            remove,
                            PlatformRect::from_xywh(
                                drawer.min.x + 8.0,
                                drawer.max.y - 100.0,
                                drawer.width() - 16.0,
                                44.0,
                            ),
                            focused,
                        );
                    }
                }
            },
        }
        push_view_control(
            &mut controls,
            WorkshopViewAction::ScrollPrevious,
            "Previous",
            PlatformRect::from_xywh(drawer.min.x + 8.0, drawer.max.y - 48.0, 108.0, 44.0),
            focused,
            false,
        );
        push_view_control(
            &mut controls,
            WorkshopViewAction::ScrollNext,
            "Next",
            PlatformRect::from_xywh(drawer.max.x - 116.0, drawer.max.y - 48.0, 108.0, 44.0),
            focused,
            false,
        );
    }
    if let Some(dialog) = &model.removal_confirmation {
        let center = layout.canvas.center();
        let y = center.y + 64.0;
        push_workshop_control(
            &mut controls,
            &dialog.cancel_control,
            PlatformRect::from_xywh(center.x - 154.0, y, 148.0, 40.0),
            focused,
        );
        push_workshop_control(
            &mut controls,
            &dialog.confirm_control,
            PlatformRect::from_xywh(center.x + 6.0, y, 148.0, 40.0),
            focused,
        );
    }
    if let Some(dialog) = &model.creator_form {
        let center = layout.canvas.center();
        let start_y =
            (center.y - dialog.fields.len() as f32 * 17.0 - 48.0).max(layout.canvas.min.y + 16.0);
        for (index, field) in dialog.fields.iter().enumerate() {
            push_workshop_control(
                &mut controls,
                &field.control,
                PlatformRect::from_xywh(
                    center.x - 210.0,
                    start_y + index as f32 * 34.0,
                    420.0,
                    30.0,
                ),
                focused,
            );
        }
        let y = start_y + dialog.fields.len() as f32 * 34.0 + 10.0;
        push_workshop_control(
            &mut controls,
            &dialog.cancel_control,
            PlatformRect::from_xywh(center.x - 154.0, y, 100.0, 40.0),
            focused,
        );
        push_workshop_control(
            &mut controls,
            &dialog.submit_control,
            PlatformRect::from_xywh(center.x - 42.0, y, 196.0, 40.0),
            focused,
        );
    }

    let modal_content = modal_presentation(&model.semantics, &layout);
    // Derived from the same dialog `modal_presentation` renders, not from named model
    // fields — see `modal_dialog`. Building this list by hand meant the focus trap could
    // describe a different dialog than the one on screen whenever both modals were open.
    let mut modal_actions = modal_action_ids(&model.semantics);
    let modal = modal_content.as_ref().map(|content| content.bounds);
    if let Some(modal_actions) = modal_actions.as_ref() {
        for control in &mut controls {
            if !modal_actions.contains(&control.action_id) {
                control.enabled = false;
            }
        }
    }

    let sighted_text = build_inspector_sighted_text(model, &layout, view, &controls);
    let mut semantics = model.semantics.clone();
    if let Some(content) = &modal_content {
        let actions = modal_actions.as_mut().unwrap();
        let visible = content
            .visible_rows(view.modal_start(model))
            .collect::<Vec<_>>();
        controls.retain(|control| {
            !content
                .rows
                .iter()
                .any(|row| row.action.as_ref() == Some(&control.action_id))
                || visible
                    .iter()
                    .any(|(row, _)| row.action.as_ref() == Some(&control.action_id))
        });
        for control in &mut controls {
            if let Some((_, bounds)) = visible
                .iter()
                .find(|(row, _)| row.action.as_ref() == Some(&control.action_id))
            {
                control.bounds = *bounds;
            } else if actions.contains(&control.action_id) {
                // Which button sits on the right is a *list*, not a derivation: the
                // affirmative action of each known dialog. It reads the owners'
                // constants so a rename cannot strand it, but it does not generalize —
                // a third dialog must be added here to get its affirmative button
                // right-aligned. Left as a list deliberately rather than inventing a
                // "last action is primary" convention nothing else declares.
                let right = matches!(
                    control.action_id.as_str(),
                    CREATOR_SUBMIT_ACTION | REMOVAL_CONFIRM_ACTION
                );
                let width = (content.bounds.width() - 32.0) * 0.5;
                control.bounds = PlatformRect::from_xywh(
                    content.bounds.min.x + 12.0 + if right { width + 8.0 } else { 0.0 },
                    content.bounds.max.y - content.footer_height - 8.0,
                    width,
                    content.footer_height,
                );
            }
        }
        let dialog = semantics
            .root
            .children
            .iter_mut()
            .find(|node| node.role == SemanticRole::Dialog)
            .unwrap();
        dialog.children.push(SemanticNode::text(
            content.title_id.as_str(),
            SemanticRole::Heading,
            &content.title,
            "",
        ));
        if content.scrolling {
            // Existing drawer scroll actions are owned by this dialog while it is active.
            controls.retain(|control| {
                !matches!(
                    control.action_id.as_str(),
                    "view.scroll-previous" | "view.scroll-next"
                )
            });
            for (action, label, x) in [
                (
                    WorkshopViewAction::ScrollPrevious,
                    "Previous",
                    content.bounds.min.x + 12.0,
                ),
                (
                    WorkshopViewAction::ScrollNext,
                    "Next",
                    content.bounds.max.x - 120.0,
                ),
            ] {
                push_view_control(
                    &mut controls,
                    action,
                    label,
                    PlatformRect::from_xywh(
                        x,
                        content.bounds.max.y - content.footer_height - 56.0,
                        108.0,
                        40.0,
                    ),
                    focused,
                    false,
                );
                let id = action.action_id();
                actions.push(id.clone());
                dialog.children.push(SemanticNode::control(
                    format!("platform.{id}"),
                    SemanticRole::Button,
                    label,
                    "Scroll modal content",
                    true,
                    false,
                    id,
                ));
            }
        }
    }
    if model.creator_form.is_none() && model.removal_confirmation.is_none() {
        push_guide_control(
            &mut controls,
            &mut semantics.root.children,
            PlatformRect::from_xywh(
                (layout.top_bar.max.x - 304.0 * scale).max(layout.top_bar.min.x + 8.0),
                layout.top_bar.min.y + (layout.top_bar.height() - 44.0) * 0.5,
                142.0 * scale,
                44.0,
            ),
            focused,
        );
    }

    let mut status_lines = vec![
        format!(
            "Tick {} // {} // {} revisions",
            model.diagnostics.tick, model.diagnostics.backend, model.diagnostics.revision_count
        ),
        format!(
            "Save: {}{} // generation {}",
            model.save.status,
            if model.save.dirty { " // DIRTY" } else { "" },
            model
                .save
                .generation
                .map_or_else(|| "none".to_owned(), |value| value.to_string())
        ),
    ];
    if let Some(dialog) = &model.creator_form {
        status_lines.push(format!("Creator: {}", dialog.title));
        status_lines.extend(dialog.preview.iter().take(1).cloned());
        if let Some(message) = &dialog.validation_message {
            status_lines.push(format!("Needs attention: {message}"));
        }
    } else if let Some(dialog) = &model.removal_confirmation {
        status_lines.push(if dialog.blockers.is_empty() {
            format!("Confirm removal of {}", dialog.target_label)
        } else {
            format!(
                "Removal blocked by {} dependent objects",
                dialog.blockers.len()
            )
        });
    } else if !model.inspector.title.is_empty() {
        status_lines.push(format!("Selected: {}", model.inspector.title));
        if let Some(inventory) = model
            .inspector
            .sections
            .iter()
            .find(|section| section.heading == "Inventory" && !section.rows.is_empty())
        {
            status_lines.push(format!(
                "Inventory: {}",
                inventory
                    .rows
                    .iter()
                    .take(3)
                    .map(|row| format!("{} {}", row.value, row.label))
                    .collect::<Vec<_>>()
                    .join(" / ")
            ));
        } else if model.outliner.is_empty() {
            status_lines.push("Start: Create system > star > world. Player guide: F1".to_owned());
        }
    }

    apply_platform_control_geometry(&mut semantics, &mut controls);
    apply_platform_sighted_text_geometry(&mut semantics, &sighted_text, &layout, view);
    let source_records = build_source_records(
        &mut semantics,
        &layout,
        view,
        modal_content
            .as_ref()
            .map(|content| (content, view.modal_start(model))),
    );
    let mut logical_focus_order = modal_actions.clone().unwrap_or_else(|| {
        controls
            .iter()
            .filter(|control| matches!(control.action, PlatformUiAction::WorkshopView(_)))
            .map(|control| control.action_id.clone())
            .chain(model.focus_order().iter().cloned())
            .collect::<Vec<_>>()
    });
    if controls
        .iter()
        .any(|control| control.action_id.as_str() == "guide.open")
    {
        logical_focus_order.push(SemanticActionId::new("guide.open"));
    }
    let mut seen = std::collections::BTreeSet::new();
    logical_focus_order.retain(|action| seen.insert(action.clone()));
    let mut frame = PlatformUiFrame {
        viewport,
        layout,
        background: PlatformBackground::ChromeOnly,
        drawer,
        modal,
        controls,
        logical_focus_order,
        semantics,
        title: "NYON // Galaxy Workshop".to_owned(),
        status_lines,
        sighted_text,
        visible_nodes: Vec::new(),
        source_records,
        operational_status: None,
        high_contrast: model.preferences.high_contrast,
    };
    frame.rebuild_visible_nodes();
    frame
}

pub(super) fn push_view_control(
    controls: &mut Vec<PlatformControl>,
    action: WorkshopViewAction,
    label: &str,
    bounds: PlatformRect,
    focused: Option<&SemanticActionId>,
    selected: bool,
) {
    let action_id = action.action_id();
    controls.push(PlatformControl {
        semantic_id: SemanticNodeId::new(format!("platform.{}", action_id.as_str())),
        action_id: action_id.clone(),
        action: PlatformUiAction::WorkshopView(action),
        bounds,
        label: label.to_owned(),
        description: format!("{label} Workshop drawer"),
        enabled: true,
        selected,
        focused: focused == Some(&action_id),
        icon: icon_for_action(PlatformIconAction::WorkshopView(action)),
    });
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum WorkshopMarkerLayer {
    SystemRing,
    StarHalo,
    StarCore,
    StarSelectionRing,
    World,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct WorkshopMarkerWitness {
    pub layer: WorkshopMarkerLayer,
    pub source_index: usize,
    pub center: Vec2,
    pub radius: f32,
    pub color: [f32; 4],
    pub ring_thickness: Option<f32>,
    pub packed_shape: u32,
    pub vertex_span: [usize; 2],
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct WorkshopSceneCoverage {
    pub system_markers: Vec<Vec2>,
    pub star_markers: Vec<Vec2>,
    pub world_markers: Vec<Vec2>,
    pub marker_witnesses: Vec<WorkshopMarkerWitness>,
}

pub fn draw_workshop_scene(
    scene: &WorkshopSceneFrame,
    layout: &WorkshopLayout,
    batch: &mut PrimitiveBatch,
    high_contrast: bool,
) -> WorkshopSceneCoverage {
    let canvas_min = layout.canvas.min;
    let canvas_max = layout.canvas.max;
    let points = scene
        .systems
        .iter()
        .map(|system| Vec2::new(system.position_radius[0], system.position_radius[1]))
        .collect::<Vec<_>>();
    let (center, scale) = fit_points(&points, canvas_min, canvas_max);
    let project = |point: Vec2| (point - center) * scale + (canvas_min + canvas_max) * 0.5;

    for lane in &scene.lanes {
        let from = project(Vec2::new(lane.from_to_x[0], lane.from_to_y[0]));
        let to = project(Vec2::new(lane.from_to_x[1], lane.from_to_y[1]));
        batch.line(
            from,
            to,
            lane.color_width[3].max(1.0),
            [
                lane.color_width[0],
                lane.color_width[1],
                lane.color_width[2],
                0.9,
            ],
        );
    }
    let mut system_markers = Vec::with_capacity(scene.systems.len());
    let mut star_markers = Vec::with_capacity(scene.stars.len());
    let mut marker_witnesses =
        Vec::with_capacity(scene.systems.len() + scene.stars.len() * 3 + scene.worlds.len());
    for (source_index, system) in scene.systems.iter().enumerate() {
        let point = project(Vec2::new(
            system.position_radius[0],
            system.position_radius[1],
        ));
        let radius = marker_radius(
            system.position_radius[3],
            0.12,
            if high_contrast { 18.0 } else { 16.0 },
        );
        system_markers.push(point);
        let ring_thickness = if high_contrast { 3.0 } else { 2.0 };
        let vertex_start = batch.vertices().len();
        batch.ring(point, radius, ring_thickness, system.color);
        marker_witnesses.push(WorkshopMarkerWitness {
            layer: WorkshopMarkerLayer::SystemRing,
            source_index,
            center: point,
            radius,
            color: system.color,
            ring_thickness: Some(ring_thickness),
            packed_shape: batch.vertices()[vertex_start].shape,
            vertex_span: [vertex_start, batch.vertices().len()],
        });
    }

    let projected_stars = scene
        .stars
        .iter()
        .enumerate()
        .map(|(source_index, star)| {
            let point = project(Vec2::new(star.position_radius[0], star.position_radius[1]));
            let visual_scale = (star.position_radius[3] / 0.075).clamp(0.5, 2.0);
            (source_index, star, point, visual_scale)
        })
        .collect::<Vec<_>>();
    for (source_index, star, point, visual_scale) in &projected_stars {
        let radius = if high_contrast { 10.5 } else { 9.5 } * visual_scale;
        let color = [star.color[0], star.color[1], star.color[2], 0.28];
        star_markers.push(*point);
        let vertex_start = batch.vertices().len();
        batch.disc(*point, radius, color);
        marker_witnesses.push(WorkshopMarkerWitness {
            layer: WorkshopMarkerLayer::StarHalo,
            source_index: *source_index,
            center: *point,
            radius,
            color,
            ring_thickness: Some(radius),
            packed_shape: batch.vertices()[vertex_start].shape,
            vertex_span: [vertex_start, batch.vertices().len()],
        });
    }
    for (source_index, star, point, visual_scale) in &projected_stars {
        let radius = if high_contrast { 5.5 } else { 4.75 } * visual_scale;
        let vertex_start = batch.vertices().len();
        batch.disc(*point, radius, star.color);
        marker_witnesses.push(WorkshopMarkerWitness {
            layer: WorkshopMarkerLayer::StarCore,
            source_index: *source_index,
            center: *point,
            radius,
            color: star.color,
            ring_thickness: Some(radius),
            packed_shape: batch.vertices()[vertex_start].shape,
            vertex_span: [vertex_start, batch.vertices().len()],
        });
    }
    for (source_index, star, point, visual_scale) in &projected_stars {
        if star.flags[0] == 0 {
            continue;
        }
        let radius = if high_contrast { 14.0 } else { 12.75 } * visual_scale;
        let color = if high_contrast {
            [1.0, 1.0, 1.0, 1.0]
        } else {
            [1.0, 0.92, 0.32, 1.0]
        };
        let ring_thickness = if high_contrast { 3.0 } else { 2.25 };
        let vertex_start = batch.vertices().len();
        batch.ring(*point, radius, ring_thickness, color);
        marker_witnesses.push(WorkshopMarkerWitness {
            layer: WorkshopMarkerLayer::StarSelectionRing,
            source_index: *source_index,
            center: *point,
            radius,
            color,
            ring_thickness: Some(ring_thickness),
            packed_shape: batch.vertices()[vertex_start].shape,
            vertex_span: [vertex_start, batch.vertices().len()],
        });
    }
    let mut world_markers = Vec::with_capacity(scene.worlds.len());
    for (source_index, world) in scene.worlds.iter().enumerate() {
        let point = project(Vec2::new(
            world.position_radius[0],
            world.position_radius[1],
        ));
        let radius = marker_radius(world.position_radius[3], 0.05, 3.5);
        world_markers.push(point);
        let vertex_start = batch.vertices().len();
        batch.disc(point, radius, world.color);
        marker_witnesses.push(WorkshopMarkerWitness {
            layer: WorkshopMarkerLayer::World,
            source_index,
            center: point,
            radius,
            color: world.color,
            ring_thickness: Some(radius),
            packed_shape: batch.vertices()[vertex_start].shape,
            vertex_span: [vertex_start, batch.vertices().len()],
        });
    }
    for shipment in &scene.shipments {
        let point = project(Vec2::new(
            shipment.position_units[0],
            shipment.position_units[1],
        ));
        batch.disc(point, 3.0, shipment.color);
    }
    for entry in scene.semantic_entries.iter().filter(|entry| {
        matches!(
            entry.role,
            crate::presentation::workshop::WorkshopSemanticRole::System
        )
    }) {
        let point = project(Vec2::new(entry.bounds[0], entry.bounds[1]));
        batch.text(
            point + Vec2::new(10.0, -16.0),
            1.0,
            if high_contrast {
                [1.0, 1.0, 1.0, 1.0]
            } else {
                [0.58, 0.82, 1.0, 1.0]
            },
            &truncate_ascii(&entry.label.to_ascii_uppercase(), 18),
        );
    }
    WorkshopSceneCoverage {
        system_markers,
        star_markers,
        world_markers,
        marker_witnesses,
    }
}

pub(super) fn marker_radius(source_radius: f32, reference_radius: f32, pixel_radius: f32) -> f32 {
    let scale = if source_radius.is_finite() && source_radius > 0.0 {
        (source_radius / reference_radius).clamp(0.5, 2.0)
    } else {
        1.0
    };
    pixel_radius * scale
}

pub(super) fn push_workshop_control(
    controls: &mut Vec<PlatformControl>,
    control: &WorkshopControl,
    bounds: PlatformRect,
    focused: Option<&SemanticActionId>,
) {
    controls.push(PlatformControl {
        semantic_id: SemanticNodeId::new(format!("platform.{}", control.action_id.as_str())),
        action_id: control.action_id.clone(),
        action: PlatformUiAction::Workshop(control.action_id.clone()),
        bounds,
        label: control.label.clone(),
        description: control.description.clone(),
        enabled: control.enabled,
        selected: control.selected,
        focused: focused == Some(&control.action_id),
        icon: icon_for_action(PlatformIconAction::Workshop(&control.intent)),
    });
}

pub(super) fn fit_points(points: &[Vec2], min: Vec2, max: Vec2) -> (Vec2, f32) {
    if points.is_empty() {
        return (Vec2::ZERO, 1.0);
    }
    let mut low = points[0];
    let mut high = points[0];
    for point in points.iter().copied().filter(|point| point.is_finite()) {
        low = low.min(point);
        high = high.max(point);
    }
    let span = (high - low).max(Vec2::splat(1.0));
    let available = (max - min - Vec2::splat(80.0)).max(Vec2::splat(1.0));
    (
        (low + high) * 0.5,
        (available.x / span.x).min(available.y / span.y),
    )
}

pub(super) fn truncate_ascii(value: &str, maximum: usize) -> String {
    value.chars().take(maximum).collect()
}
