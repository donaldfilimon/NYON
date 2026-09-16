//! Library platform frame: route-design task 8, slices 1 to 3.
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
//! no room for a single row. Compact places no actions or transfer controls
//! while the list shows: its `drawer_sheet` is the whole canvas at the floor,
//! so an always-open sheet would hide every row.
//!
//! **The Compact sheet (slice 2c).** Selecting a row opens the sheet on its
//! actions page; the bottom bar's third slot opens it on the transfer page,
//! which is the only route to Import in an empty Library. While the sheet is
//! open that slot is Back, so the page gets the sheet's full height. Whatever
//! the sheet covers is removed for the frame: no pointer target, no focus
//! slot, no drawn text. Page columns are sized by the widest word, because a
//! label wraps between words and five one-line labels do not fit at the floor.
//!
//! **Section headings travel with their first row.** Each section's heading is
//! drawn directly above its first placed row, and only when that row fits too;
//! a section with no placed row draws no heading and its node is hidden.
//!
//! **Origins are whole logical pixels.** A 44-pixel control placed below
//! wrapped text at a fractional offset measured 43.99998 tall, which is both
//! under the minimum and blurry; snapping the origin makes `min + 44` exact.
//!
//! **Rows that do not fit are reached by paging (slice 2b).** When rows
//! overflow, Previous and Next sit on the first placed row's line, to the right
//! of a narrower row column, so paging costs no height: at the 320 floor and
//! the largest scale the canvas had 13 pixels left under its only row. The
//! Workshop outliner instead keeps off-screen rows in the focus order and
//! scrolls to the focused one; that would give an unplaced row a focus slot,
//! which the next paragraph rules out. Both pager controls are always enabled
//! and do nothing at the ends, as the Workshop's Previous and Next do.
//!
//! An unplaced control is **not** a platform control. It stays in the semantic
//! tree with no bounds, is invisible, and gets no focus slot, so pointer and
//! keyboard agree that it cannot be reached rather than disagreeing about a
//! control that has a focus slot and nowhere to draw it.

use super::{
    PlatformBackground, PlatformControl, PlatformRect, PlatformUiAction, PlatformUiFrame,
    apply_platform_control_geometry,
};
use crate::ui::accessibility::{SemanticActionId, SemanticNode, SemanticNodeId, SemanticRole};
use crate::ui::library::{LibraryControl, LibrarySection, LibraryUiModel};
use crate::ui::platform_projection::{
    materialize_modal, materialize_text_block, modal_action_ids, modal_footer_bounds,
    modal_presentation,
};
use crate::ui::workshop_layout::{WorkshopLayout, WorkshopLayoutMode};

/// Logical gap between controls and from region edges.
const GAP: f32 = 8.0;

/// Every Library control is at least 44 logical pixels on both axes, at every
/// UI scale. `44 * scale` alone would be 37.4 at the 0.85 floor.
fn control_height(scale: f32) -> f32 {
    (48.0 * scale).max(44.0)
}

/// The view-only Library actions. They move the row window or the Compact
/// sheet and touch neither the model nor the store.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum LibraryViewAction {
    PreviousRows,
    NextRows,
    /// Compact only: opens the sheet on its transfer page. The one route to
    /// Import in an empty Library, where there is no row to select.
    OpenTransfer,
    /// Compact only: closes the sheet, back to the list.
    CloseSheet,
}

impl LibraryViewAction {
    pub fn action_id(self) -> SemanticActionId {
        SemanticActionId::new(match self {
            Self::PreviousRows => "library.rows.previous",
            Self::NextRows => "library.rows.next",
            Self::OpenTransfer => "library.sheet.transfer",
            Self::CloseSheet => "library.sheet.close",
        })
    }

    const fn label(self) -> (&'static str, &'static str) {
        match self {
            Self::PreviousRows => ("Previous", "Show the previous saved galaxies."),
            Self::NextRows => ("Next", "Show the next saved galaxies."),
            Self::OpenTransfer => ("Transfer", "Import or export portable files."),
            Self::CloseSheet => ("Back", "Return to the saved galaxy list."),
        }
    }
}

/// Which page of the Compact sheet is open. Wide and Medium dock both
/// surfaces and ignore it.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum LibrarySheet {
    /// Opened by selecting a row: the five actions for that save.
    Actions,
    /// Opened by the bottom bar's Transfer control.
    Transfer,
}

/// Client-side view state the Library frame is drawn for.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct LibraryView {
    /// First model row placed; moved only by the pager.
    pub row_start: usize,
    pub sheet: Option<LibrarySheet>,
}

/// Clamps a row window start to the rows the model holds, so a Refresh that
/// shrinks the list never leaves an empty window.
pub fn clamp_library_row_start(model: &LibraryUiModel, row_start: usize) -> usize {
    row_start.min(model.rows.len().saturating_sub(1))
}

pub fn build_library_platform_frame(
    model: &LibraryUiModel,
    layout: WorkshopLayout,
    view: LibraryView,
    focused: Option<&SemanticActionId>,
    high_contrast: bool,
) -> PlatformUiFrame {
    let row_start = clamp_library_row_start(model, view.row_start);
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
    let first_row = placed.len();
    for row in model.rows.iter().skip(row_start) {
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

    let rows_placed = placed.len() - first_row;
    let mut view_controls: Vec<(LibraryViewAction, PlatformRect)> = Vec::new();
    let mut pager = Vec::new();
    if rows_placed > 0 && rows_placed < model.rows.len() {
        let line = placed[first_row].1;
        let mut right = canvas.max.x;
        for action in [LibraryViewAction::NextRows, LibraryViewAction::PreviousRows] {
            let width = label_width(action.label().0, scale, height);
            let bounds = PlatformRect::from_xywh(right - width, line.min.y, width, height);
            right = bounds.min.x - GAP;
            pager.push((action, bounds));
        }
        pager.reverse();
        let row_width = (right - canvas.min.x).floor();
        for (_, bounds) in &mut placed[first_row..] {
            *bounds = PlatformRect::from_xywh(bounds.min.x, bounds.min.y, row_width, height);
        }
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
    let mut sheet = None;
    if compact {
        // Close, Refresh and the sheet's Transfer opener share the strip; the
        // column width is the widest label, so a label is never cut.
        let chrome = [&model.close_control, &model.refresh_control];
        let (opener_label, _) = LibraryViewAction::OpenTransfer.label();
        let min_width = chrome
            .iter()
            .map(|control| label_width(&control.label, scale, height))
            .fold(label_width(opener_label, scale, height), f32::max);
        let (cells, _) = grid_cells(3, strip, height, min_width);
        for (control, cell) in chrome.into_iter().zip(&cells) {
            if let Some(cell) = cell {
                placed.push((control, *cell));
            }
        }
        // The third slot is Transfer while the list shows and Back while the
        // sheet does, which leaves the sheet's full height to its page.
        if let Some(cell) = cells[2] {
            let slot = if view.sheet.is_some() {
                LibraryViewAction::CloseSheet
            } else {
                LibraryViewAction::OpenTransfer
            };
            view_controls.push((slot, cell));
        }

        if let Some(page) = view.sheet {
            let drawer = layout.drawer_sheet;
            sheet = Some(drawer);
            // What the sheet covers is gone for this frame: no pointer target,
            // no focus slot, no drawn text under it.
            placed.retain(|(_, bounds)| !bounds.overlaps(drawer));
            pager
                .retain(|(_, bounds): &(LibraryViewAction, PlatformRect)| !bounds.overlaps(drawer));
            let mut covered = Vec::new();
            records.retain(|record| {
                let keep = !record.bounds.overlaps(drawer);
                if !keep {
                    covered.push(record.semantic_id.clone());
                }
                keep
            });
            for id in covered {
                hide_node(&mut semantics.root, id.as_str());
            }

            let body = inset(drawer, GAP);
            let page_controls: Vec<&LibraryControl> = match page {
                LibrarySheet::Actions => model.actions.controls().to_vec(),
                LibrarySheet::Transfer => model.transfer.controls().to_vec(),
            };
            // Sized by the widest word, not the widest label: a control label
            // wraps between words, and "Use for Continue" on one line would
            // force a single column that cannot hold five controls at the floor.
            let min_width = page_controls
                .iter()
                .flat_map(|control| control.label.split(' '))
                .map(|word| label_width(word, scale, height))
                .fold(0.0, f32::max);
            place_grid(&page_controls, body, height, min_width, &mut placed);
        }
    } else {
        // Docked, nothing here is allowed to go missing (baseline Finding 5).
        // The bar spans the window and the narrowest docked window is 900
        // logical pixels, so the strip never wraps and needs no scroll pair;
        // `every_docked_width_places_every_transfer_control_and_action_on_screen`
        // sweeps that, and this makes any future drop loud.
        let before = placed.len();
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
        debug_assert_eq!(
            placed.len() - before,
            model.actions.controls().len() + transfer.len(),
            "a docked Library frame dropped an action or transfer control"
        );
    }

    // The confirmation (task 9) sits over whatever the frame placed. Its body
    // must fit without paging: the Workshop's paging controls are Workshop
    // view actions, and `every_library_dialog_fits_without_paging` holds the
    // Library to that across the matrix.
    let modal_content = modal_presentation(&model.semantics, &layout);
    if let (Some(content), Some(confirmation)) = (&modal_content, &model.confirmation) {
        debug_assert!(!content.scrolling, "a Library dialog needs paging");
        // The rename field is a body row; the two buttons are the footer.
        if let Some(field) = &confirmation.name_field {
            let row = content
                .visible_rows(0)
                .find(|(row, _)| row.action.as_ref() == Some(&field.action_id));
            debug_assert!(row.is_some(), "the rename field did not fit its dialog");
            if let Some((_, bounds)) = row {
                // Whole-pixel origin, as everywhere in this frame: the row's
                // fractional origin measured 43.99998 tall.
                let snapped = PlatformRect::from_xywh(
                    bounds.min.x.round(),
                    bounds.min.y.round(),
                    bounds.width().floor(),
                    bounds.height(),
                );
                placed.push((field, snapped));
            }
        }
        placed.push((
            &confirmation.cancel_control,
            modal_footer_bounds(content, false),
        ));
        placed.push((
            &confirmation.submit_control,
            modal_footer_bounds(content, true),
        ));
    }

    view_controls.extend(pager.iter().copied());
    let pager_controls: Vec<PlatformControl> = view_controls
        .iter()
        .map(|&(action, bounds)| {
            let action_id = action.action_id();
            let (label, description) = action.label();
            PlatformControl {
                semantic_id: SemanticNodeId::new(format!("platform.{}", action_id.as_str())),
                focused: focused == Some(&action_id),
                action: PlatformUiAction::LibraryView(action),
                action_id,
                bounds,
                label: label.to_owned(),
                description: description.to_owned(),
                enabled: true,
                selected: false,
                icon: None,
            }
        })
        .collect();
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
    let last_row = model
        .rows
        .iter()
        .rev()
        .find(|row| {
            controls
                .iter()
                .any(|control| control.action_id == row.control.action_id)
        })
        .map(|row| row.control.action_id.clone());
    controls.extend(pager_controls);
    let modal_actions = modal_action_ids(&model.semantics);
    if let Some(actions) = &modal_actions {
        for control in &mut controls {
            if !actions.contains(&control.action_id) {
                control.enabled = false;
            }
        }
    }
    apply_platform_control_geometry(&mut semantics, &mut controls);
    if let Some(content) = &modal_content {
        if let Some(dialog) = semantics
            .root
            .children
            .iter_mut()
            .find(|node| node.role == SemanticRole::Dialog)
        {
            dialog.children.push(SemanticNode::text(
                content.title_id.as_str(),
                SemanticRole::Heading,
                &content.title,
                "",
            ));
        }
        materialize_modal(&mut semantics, scale, content, 0, &mut records);
    }

    // The model's order, restricted to what this frame placed, with the
    // view controls inserted where a keyboard user expects them: Transfer
    // after Refresh, Back before the sheet's first control, Previous and Next
    // after the last placed row.
    let has = |action: LibraryViewAction| view_controls.iter().any(|(item, _)| *item == action);
    let page_first = view.sheet.and_then(|page| {
        let ids: Vec<&SemanticActionId> = match page {
            LibrarySheet::Actions => model
                .actions
                .controls()
                .iter()
                .map(|c| &c.action_id)
                .collect(),
            LibrarySheet::Transfer => model
                .transfer
                .controls()
                .iter()
                .map(|c| &c.action_id)
                .collect(),
        };
        model
            .focus_order()
            .iter()
            .find(|id| ids.contains(id) && controls.iter().any(|control| &control.action_id == *id))
            .cloned()
    });
    let mut logical_focus_order: Vec<SemanticActionId> = Vec::new();
    let mut back_pending = has(LibraryViewAction::CloseSheet);
    for id in model.focus_order() {
        if back_pending && page_first.as_ref() == Some(id) {
            logical_focus_order.push(LibraryViewAction::CloseSheet.action_id());
            back_pending = false;
        }
        if controls.iter().any(|control| &control.action_id == id) {
            logical_focus_order.push(id.clone());
        }
        if *id == model.refresh_control.action_id && has(LibraryViewAction::OpenTransfer) {
            logical_focus_order.push(LibraryViewAction::OpenTransfer.action_id());
        }
        if last_row.as_ref() == Some(id) {
            logical_focus_order.extend(pager.iter().map(|(action, _)| action.action_id()));
        }
    }
    if back_pending {
        logical_focus_order.push(LibraryViewAction::CloseSheet.action_id());
    }

    if let Some(actions) = modal_actions {
        logical_focus_order = actions;
    }

    let mut frame = PlatformUiFrame {
        viewport: glam::Vec2::new(layout.viewport.width(), layout.viewport.height()),
        layout,
        background: PlatformBackground::Full,
        drawer: sheet,
        modal: modal_content.as_ref().map(|content| content.bounds),
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

/// Wide enough for the label at the control font, never under the minimum.
fn label_width(label: &str, scale: f32, minimum: f32) -> f32 {
    let metrics = crate::ui::AtlasMetrics::embedded().expect("embedded atlas is valid");
    let advance = metrics
        .measure_text(13.0 * scale, crate::ui::FontWeight::SemiBold, label)
        .expect("pager labels are atlas text")
        .advance;
    (advance + 6.0 + 2.0 * crate::ui::platform_sdf::text_ink_padding(scale))
        .ceil()
        .max(minimum)
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
    let (cells, used) = grid_cells(controls.len(), area, height, min_width);
    for (control, cell) in controls.iter().zip(cells) {
        if let Some(cell) = cell {
            placed.push((control, cell));
        }
    }
    used
}

/// The cells [`place_grid`] would use for `count` controls; `None` for a cell
/// outside `area`.
fn grid_cells(
    count: usize,
    area: PlatformRect,
    height: f32,
    min_width: f32,
) -> (Vec<Option<PlatformRect>>, f32) {
    if count == 0 || area.width() <= 0.0 {
        return (vec![None; count], 0.0);
    }
    let fits = ((area.width() + GAP) / (min_width + GAP)).floor() as usize;
    let columns = fits.clamp(1, count);
    let width = ((area.width() - GAP * (columns - 1) as f32) / columns as f32).floor();
    let mut used: f32 = 0.0;
    let mut cells = Vec::with_capacity(count);
    for index in 0..count {
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
            cells.push(None);
            continue;
        }
        used = used.max(bounds.max.y - area.min.y + GAP);
        cells.push(Some(bounds));
    }
    (cells, used)
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
