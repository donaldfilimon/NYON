//! Library platform frame: route-design task 8, slice 1.
//!
//! The model in `crate::ui::library` owns the semantic tree, every label, every
//! disabled reason and the focus order. This layer only assigns geometry, so it
//! clones that tree rather than rebuilding it the way the shell does, which has
//! no model of its own.
//!
//! **What slice 1 places, and what it deliberately does not.** Every mode puts
//! the request and content text at the top of the canvas, Retry and Cancel
//! directly beneath the failure they answer, and the rows under those. Wide and
//! Medium add Close and Refresh to that grid, the selected-save actions in
//! `right_panel`, and the four transfer controls in `bottom_bar`. Compact moves
//! Close and Refresh into its otherwise-empty `bottom_bar` instead: at the 320
//! floor and the largest scale, a failure message plus a four-control grid left
//! no room for a single row. Compact places no actions or transfer controls:
//! its `drawer_sheet` is the whole canvas at the floor, so an always-open sheet
//! would hide every row, and "row selection opens the sheet" is slice 2. A row
//! that does not fit the canvas is likewise left for slice 2's reveal.
//!
//! **Section headings travel with their first row.** Each section's heading is
//! drawn directly above its first placed row, and only when that row fits too;
//! a section with no placed row draws no heading and its node is hidden.
//!
//! **Origins are whole logical pixels.** A 44-pixel control placed below
//! wrapped text at a fractional offset measured 43.99998 tall, which is both
//! under the minimum and blurry; snapping the origin makes `min + 44` exact.
//!
//! An unplaced control is **not** a platform control. It stays in the semantic
//! tree with no bounds, is invisible, and gets no focus slot, so pointer and
//! keyboard agree that it cannot be reached rather than disagreeing about a
//! control that has a focus slot and nowhere to draw it.

use super::{
    PlatformBackground, PlatformControl, PlatformRect, PlatformUiAction, PlatformUiFrame,
    apply_platform_control_geometry,
};
use crate::ui::accessibility::{SemanticActionId, SemanticNode, SemanticNodeId};
use crate::ui::library::{LibraryControl, LibrarySection, LibraryUiModel};
use crate::ui::platform_projection::materialize_text_block;
use crate::ui::workshop_layout::{WorkshopLayout, WorkshopLayoutMode};

/// Logical gap between controls and from region edges.
const GAP: f32 = 8.0;

/// Every Library control is at least 44 logical pixels on both axes, at every
/// UI scale. `44 * scale` alone would be 37.4 at the 0.85 floor.
fn control_height(scale: f32) -> f32 {
    (48.0 * scale).max(44.0)
}

pub fn build_library_platform_frame(
    model: &LibraryUiModel,
    layout: WorkshopLayout,
    focused: Option<&SemanticActionId>,
    high_contrast: bool,
) -> PlatformUiFrame {
    let scale = layout.ui_scale;
    let height = control_height(scale);
    let mut semantics = model.semantics.clone();
    let mut records = Vec::new();
    let mut placed: Vec<(&LibraryControl, PlatformRect)> = Vec::new();

    let canvas = inset(layout.canvas, GAP);
    let used = materialize_text_block(
        &mut semantics,
        &["library.request.status", "library.content"],
        // Bounded so a long message cannot push every control off the canvas.
        PlatformRect::from_xywh(
            canvas.min.x,
            canvas.min.y,
            canvas.width(),
            canvas.height() * 0.4,
        ),
        scale,
        &mut records,
    );
    let mut top = (canvas.min.y + used + GAP).ceil();
    let compact = layout.mode == WorkshopLayoutMode::Compact;

    let mut header = Vec::new();
    if !compact {
        header.extend([&model.close_control, &model.refresh_control]);
    }
    header.extend(&model.request.retry_control);
    header.extend(&model.request.cancel_control);
    let header_area = PlatformRect::from_xywh(
        canvas.min.x,
        top,
        canvas.width(),
        (canvas.max.y - top).max(0.0),
    );
    top += place_grid(&header, header_area, height, 110.0 * scale, &mut placed);

    let mut section = None;
    for row in &model.rows {
        let mut row_top = top;
        let mut heading = None;
        if section != Some(row.section) {
            // Measured into a scratch tree first: a heading is only drawn when
            // the row it introduces fits too, so no heading is ever stranded at
            // the bottom of the canvas with nothing under it.
            let id = section_heading_id(row.section);
            let mut probe = semantics.clone();
            let mut probe_records = Vec::new();
            let used = materialize_text_block(
                &mut probe,
                &[id],
                PlatformRect::from_xywh(canvas.min.x, top, canvas.width(), canvas.max.y - top),
                scale,
                &mut probe_records,
            );
            row_top = (top + used + GAP * 0.5).ceil();
            heading = Some((id, used));
        }
        let bounds = PlatformRect::from_xywh(canvas.min.x, row_top, canvas.width(), height);
        if !canvas.contains_rect(bounds) {
            break;
        }
        if let Some((id, used)) = heading {
            materialize_text_block(
                &mut semantics,
                &[id],
                PlatformRect::from_xywh(canvas.min.x, top, canvas.width(), used),
                scale,
                &mut records,
            );
            section = Some(row.section);
        }
        placed.push((&row.control, bounds));
        top = (row_top + height + GAP * 0.5).ceil();
    }

    // A heading whose section has no placed row is unplaced exactly as an
    // unplaced control is: in the tree, with no bounds, and not visible.
    for section in [LibrarySection::Active, LibrarySection::Archived] {
        if section_placed(&placed, model, section) {
            continue;
        }
        hide_node(&mut semantics.root, section_heading_id(section));
    }

    let bar = layout.bottom_bar;
    let strip = PlatformRect::from_xywh(
        bar.min.x + GAP,
        (bar.min.y + ((bar.height() - height) * 0.5).max(0.0)).floor(),
        bar.width() - GAP * 2.0,
        height,
    );
    if compact {
        let chrome = [&model.close_control, &model.refresh_control];
        place_grid(&chrome, strip, height, 110.0 * scale, &mut placed);
    } else {
        if let Some(panel) = layout.right_panel {
            let actions = model.actions.controls();
            place_grid(
                &actions,
                inset(panel, GAP),
                height,
                100.0 * scale,
                &mut placed,
            );
        }
        let transfer = model.transfer.controls();
        place_grid(&transfer, strip, height, 100.0 * scale, &mut placed);
    }

    let mut controls: Vec<PlatformControl> = placed
        .into_iter()
        .map(|(control, bounds)| PlatformControl {
            // Replaced from the tree by `apply_platform_control_geometry`, which
            // matches on the action identifier; the placeholder only has to be
            // unique until then.
            semantic_id: SemanticNodeId::new(format!(
                "library.platform.{}",
                control.action_id.as_str()
            )),
            action_id: control.action_id.clone(),
            action: PlatformUiAction::Library(control.action_id.clone()),
            bounds,
            label: control.label.clone(),
            description: control.description.clone(),
            enabled: control.enabled,
            selected: control.selected,
            focused: focused == Some(&control.action_id),
            // Slice 1 maps no Library icons; a label alone is the frozen
            // vocabulary's fallback and adding one is not a layout change.
            icon: None,
        })
        .collect();
    apply_platform_control_geometry(&mut semantics, &mut controls);

    // The model's order, restricted to what this frame placed. Order is
    // preserved, so slice 2 placing more controls only inserts slots.
    let logical_focus_order = model
        .focus_order()
        .iter()
        .filter(|id| controls.iter().any(|control| &control.action_id == *id))
        .cloned()
        .collect();

    let mut frame = PlatformUiFrame {
        viewport: glam::Vec2::new(layout.viewport.width(), layout.viewport.height()),
        layout,
        background: PlatformBackground::Full,
        drawer: None,
        modal: None,
        controls,
        logical_focus_order,
        semantics,
        title: "NYON // Library".to_owned(),
        status_lines: Vec::new(),
        sighted_text: Vec::new(),
        visible_nodes: Vec::new(),
        source_records: records,
        operational_status: None,
        high_contrast,
    };
    frame.rebuild_visible_nodes();
    frame
}

fn section_placed(
    placed: &[(&LibraryControl, PlatformRect)],
    model: &LibraryUiModel,
    section: LibrarySection,
) -> bool {
    model
        .rows
        .iter()
        .filter(|row| row.section == section)
        .any(|row| {
            placed
                .iter()
                .any(|(control, _)| control.action_id == row.control.action_id)
        })
}

fn hide_node(node: &mut SemanticNode, id: &str) {
    if node.id.as_str() == id {
        node.visible = false;
        node.bounds = None;
    }
    for child in &mut node.children {
        hide_node(child, id);
    }
}

fn section_heading_id(section: LibrarySection) -> &'static str {
    match section {
        LibrarySection::Active => "library.section.active.heading",
        LibrarySection::Archived => "library.section.archived.heading",
    }
}

/// Places `controls` left to right, top to bottom, inside `area`.
///
/// Uses as many columns as keep each at least `min_width` wide, capped at the
/// number of controls, so a wide region gets one row and a narrow one wraps.
/// A control whose cell falls outside `area` is left unplaced rather than
/// clipped. Returns the height consumed, including the trailing gap.
fn place_grid<'a>(
    controls: &[&'a LibraryControl],
    area: PlatformRect,
    height: f32,
    min_width: f32,
    placed: &mut Vec<(&'a LibraryControl, PlatformRect)>,
) -> f32 {
    if controls.is_empty() || area.width() <= 0.0 {
        return 0.0;
    }
    let fits = ((area.width() + GAP) / (min_width + GAP)).floor() as usize;
    let columns = fits.clamp(1, controls.len());
    let width = ((area.width() - GAP * (columns - 1) as f32) / columns as f32).floor();
    let mut used: f32 = 0.0;
    for (index, control) in controls.iter().enumerate() {
        let row = (index / columns) as f32;
        let column = (index % columns) as f32;
        // `ceil`, not `round`: rounding can move a cell before the area's
        // origin when that origin is fractional, and the containment check
        // below would then drop the control without a trace. A mutation that
        // removed the caller's own snapping lost the Retry button exactly
        // that way.
        let bounds = PlatformRect::from_xywh(
            (area.min.x + column * (width + GAP)).ceil(),
            (area.min.y + row * (height + GAP)).ceil(),
            width,
            height,
        );
        if !area.contains_rect(bounds) {
            continue;
        }
        used = used.max(bounds.max.y - area.min.y + GAP);
        placed.push((control, bounds));
    }
    used
}

fn inset(rect: PlatformRect, by: f32) -> PlatformRect {
    let min_x = (rect.min.x + by).ceil();
    let min_y = (rect.min.y + by).ceil();
    PlatformRect::from_xywh(
        min_x,
        min_y,
        (rect.max.x - by - min_x).floor().max(0.0),
        (rect.max.y - by - min_y).floor().max(0.0),
    )
}
