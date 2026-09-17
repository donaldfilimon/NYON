//! The Library platform frame: route-design task 8 and the task 9 dialogs.
//!
//! `workshop_ui_library` pins the model. This suite pins what the platform
//! layer does with it: that every frame survives the real SDF batch, that every
//! placed control is at least 44 by 44 and reachable by both pointer and
//! keyboard, that the Compact sheet, row paging and the docked strip place what
//! they must, and that a confirmation dialog fits, traps focus and disables
//! everything behind it.

mod common;

use std::collections::BTreeSet;

use glam::Vec2;
use nyon::{
    app::{
        client_runtime::{ClientDiagnosticCode, ExportSource, LibrarySlotsStatus, SlotRequestKind},
        transfer::{HandoffOutcome, TransferFailureCode},
    },
    engine::primitives::PrimitiveBatch,
    ui::{
        AtlasMetrics, UiBatch,
        accessibility::SemanticActionId,
        library::{
            LibraryConfirmationKind, LibraryConfirmationRequest, LibrarySection, LibraryUiContext,
            LibraryUiModel, library_confirmation_order,
        },
        platform::*,
        workshop_layout::{WorkshopLayout, WorkshopLayoutMode},
    },
    workshop::store::{SaveGeneration, SlotId, SlotList, SlotName, SlotSummary},
};

const ACTION_IDS: [&str; 5] = [
    "library.action.open",
    "library.action.rename",
    "library.action.archive",
    "library.action.use-for-continue",
    "library.action.export",
];

const TRANSFER_IDS: [&str; 4] = [
    "library.transfer.import-archive",
    "library.transfer.import-pack",
    "library.transfer.export-active-archive",
    "library.transfer.export-active-pack",
];

/// Sixteen slots, the bounded capacity, every name at the 64-byte maximum and
/// distinct, a third of them archived.
fn crowded_list() -> SlotList {
    let slots = (0..16u64)
        .map(|id| SlotSummary {
            id: SlotId(id),
            name: SlotName::new(format!("{id:02}{}", "W".repeat(62))).unwrap(),
            generation: SaveGeneration(id + 1),
            archived: id % 3 == 2,
            has_previous_generation: id % 2 == 0,
            selected_for_continue: id == 0,
        })
        .collect();
    SlotList {
        slots,
        selected_continue: Some(SlotId(0)),
    }
}

/// The busiest shape the model produces: a selected row, a resident slot, an
/// active Workshop, and a failed request so Retry and Cancel both appear.
fn crowded_model(list: &SlotList) -> LibraryUiModel {
    LibraryUiModel::build(crowded_context(list))
}

fn crowded_context(list: &SlotList) -> LibraryUiContext<'_> {
    LibraryUiContext {
        slots: Some(list),
        status: LibrarySlotsStatus::Failed {
            kind: SlotRequestKind::Rename,
            slot: Some(SlotId(1)),
            code: ClientDiagnosticCode::Store,
        },
        selected_slot: Some(SlotId(1)),
        resident_slot: Some(SlotId(4)),
        workshop_active: true,
        ..LibraryUiContext::default()
    }
}

/// The SDF qualification matrix plus the 320 floor and 1440x900, each of the
/// latter at the three UI scales the layout accepts.
fn viewports() -> Vec<(Vec2, f32)> {
    let mut cases: Vec<(Vec2, f32)> = common::SDF_QUALIFICATION_MATRIX
        .iter()
        .map(|&(width, height, scale)| (Vec2::new(width, height), scale))
        .collect();
    for scale in [0.85, 1.0, 1.3] {
        // 300 of canvas plus both bars at the largest scale.
        cases.push((Vec2::new(320.0, 460.0), scale));
        cases.push((Vec2::new(1440.0, 900.0), scale));
    }
    cases
}

fn frame_for(model: &LibraryUiModel, viewport: Vec2, scale: f32) -> PlatformUiFrame {
    frame_at(model, viewport, scale, 0)
}

fn frame_at(
    model: &LibraryUiModel,
    viewport: Vec2,
    scale: f32,
    row_start: usize,
) -> PlatformUiFrame {
    frame_with(
        model,
        viewport,
        scale,
        LibraryView {
            row_start,
            sheet: None,
        },
    )
}

fn frame_with(
    model: &LibraryUiModel,
    viewport: Vec2,
    scale: f32,
    view: LibraryView,
) -> PlatformUiFrame {
    let layout = WorkshopLayout::resolve(viewport, scale).expect("test viewport resolves");
    build_library_platform_frame(model, layout, view, None, false)
}

const PAGER_IDS: [&str; 2] = ["library.rows.previous", "library.rows.next"];
const VIEW_IDS: [&str; 4] = [
    "library.rows.previous",
    "library.rows.next",
    "library.sheet.transfer",
    "library.sheet.close",
];
const SHEETS: [Option<LibrarySheet>; 3] = [
    None,
    Some(LibrarySheet::Actions),
    Some(LibrarySheet::Transfer),
];

fn sheet_cases() -> Vec<((Vec2, f32), Option<LibrarySheet>)> {
    viewports()
        .into_iter()
        .flat_map(|case| SHEETS.map(move |sheet| (case, sheet)))
        .collect()
}

fn view(sheet: Option<LibrarySheet>) -> LibraryView {
    LibraryView {
        row_start: 0,
        sheet,
    }
}

fn placed_rows(model: &LibraryUiModel, frame: &PlatformUiFrame) -> Vec<usize> {
    let placed = placed_ids(frame);
    model
        .rows
        .iter()
        .enumerate()
        .filter(|(_, row)| placed.contains(row.control.action_id.as_str()))
        .map(|(index, _)| index)
        .collect()
}

fn install(frame: &PlatformUiFrame) -> Result<(), String> {
    let metrics = AtlasMetrics::embedded().unwrap();
    let mut ui = UiBatch::default();
    let mut overlay = PrimitiveBatch::default();
    install_platform_batches(frame, &metrics, &mut ui, &mut overlay)
        .map_err(|error| format!("{error:?}"))?;
    // An `Ok` that drew nothing would pass every caller. `renderer_contracts`
    // guards its installs the same way, for the same reason.
    if ui.glyphs().is_empty() || ui.panels().is_empty() {
        return Err(format!(
            "installed but drew {} glyphs and {} panels",
            ui.glyphs().len(),
            ui.panels().len()
        ));
    }
    Ok(())
}

fn placed_ids(frame: &PlatformUiFrame) -> BTreeSet<String> {
    frame
        .controls
        .iter()
        .map(|control| control.action_id.as_str().to_owned())
        .collect()
}

#[test]
fn every_library_frame_installs_through_the_real_sdf_batch() {
    // Sixteen 64-byte names is the case the design names: a slot name is the
    // only user-authored string on screen, and anything but the `Option` role
    // would reject the entire frame rather than ellipsize it.
    let list = crowded_list();
    let model = crowded_model(&list);
    for (viewport, scale) in viewports() {
        for sheet in SHEETS {
            let frame = frame_with(&model, viewport, scale, view(sheet));
            if let Err(error) = install(&frame) {
                panic!("{viewport} at {scale} {sheet:?}: the SDF batch refused the frame: {error}");
            }
        }
        let empty = LibraryUiModel::build(LibraryUiContext::default());
        install(&frame_for(&empty, viewport, scale)).unwrap_or_else(|error| {
            panic!("{viewport} at {scale}: the resting Library frame was refused: {error}")
        });
    }
}

#[test]
fn every_placed_control_is_at_least_44_square_and_inside_the_viewport() {
    let list = crowded_list();
    let model = crowded_model(&list);
    for ((viewport, scale), sheet) in sheet_cases() {
        let frame = frame_with(&model, viewport, scale, view(sheet));
        let screen = PlatformRect::from_xywh(0.0, 0.0, viewport.x, viewport.y);
        assert!(!frame.controls.is_empty());
        for control in &frame.controls {
            let id = control.action_id.as_str();
            assert!(
                control.bounds.width() >= 44.0 && control.bounds.height() >= 44.0,
                "{viewport} at {scale}: {id} is {}x{}",
                control.bounds.width(),
                control.bounds.height()
            );
            assert!(
                screen.contains_rect(control.bounds),
                "{viewport} at {scale}: {id} leaves the viewport"
            );
        }
        for (index, left) in frame.controls.iter().enumerate() {
            for right in &frame.controls[index + 1..] {
                assert!(
                    !left.bounds.overlaps(right.bounds),
                    "{viewport} at {scale}: {} overlaps {}",
                    left.action_id.as_str(),
                    right.action_id.as_str()
                );
            }
        }
    }
}

#[test]
fn pointer_and_keyboard_reach_exactly_the_same_controls() {
    // A focus slot with no pointer target, or a pointer target with no focus
    // slot, is the disagreement slice 1 is built to avoid: an unplaced control
    // is not a platform control at all.
    let list = crowded_list();
    let model = crowded_model(&list);
    for ((viewport, scale), sheet) in sheet_cases() {
        let frame = frame_with(&model, viewport, scale, view(sheet));
        let focus: Vec<SemanticActionId> = frame.focus_order();
        let focus_set: BTreeSet<String> = focus.iter().map(|id| id.as_str().to_owned()).collect();
        assert_eq!(focus.len(), focus_set.len(), "a focus slot repeats");
        assert_eq!(
            focus_set,
            placed_ids(&frame),
            "{viewport} at {scale}: focus slots and placed controls disagree"
        );
        // The model's order is kept, only filtered.
        let model_order: Vec<&str> = model
            .focus_order()
            .iter()
            .map(SemanticActionId::as_str)
            .filter(|id| focus_set.contains(*id))
            .collect();
        let frame_order: Vec<&str> = focus
            .iter()
            .map(SemanticActionId::as_str)
            .filter(|id| !VIEW_IDS.contains(id))
            .collect();
        assert_eq!(frame_order, model_order, "{viewport} at {scale}");
        for control in &frame.controls {
            let hit = frame.hit_test(control.bounds.center());
            if !control.enabled {
                // `hit_test` ignores disabled controls by design, so a disabled
                // box must swallow the click rather than pass it to a neighbour.
                assert!(
                    hit.is_none(),
                    "a click on disabled {} reached {:?}",
                    control.action_id.as_str(),
                    hit.map(|hit| hit.action_id.as_str())
                );
                continue;
            }
            let hit = hit
                .unwrap_or_else(|| panic!("{} has no pointer target", control.action_id.as_str()));
            assert_eq!(hit.action_id, control.action_id);
            let expected = match control.action_id.as_str() {
                "library.rows.previous" => {
                    PlatformUiAction::LibraryView(LibraryViewAction::PreviousRows)
                }
                "library.rows.next" => PlatformUiAction::LibraryView(LibraryViewAction::NextRows),
                "library.sheet.transfer" => {
                    PlatformUiAction::LibraryView(LibraryViewAction::OpenTransfer)
                }
                "library.sheet.close" => {
                    PlatformUiAction::LibraryView(LibraryViewAction::CloseSheet)
                }
                id => PlatformUiAction::Library(SemanticActionId::new(id)),
            };
            assert_eq!(
                hit.action, expected,
                "a Library control must resolve against the Library model"
            );
        }
    }
}

#[test]
fn slice_one_leaves_exactly_the_compact_panels_and_the_overflowing_rows_unplaced() {
    let list = crowded_list();
    let model = crowded_model(&list);
    let deferred: BTreeSet<String> = ACTION_IDS
        .iter()
        .chain(TRANSFER_IDS.iter())
        .map(|id| (*id).to_owned())
        .collect();
    for (viewport, scale) in viewports() {
        let frame = frame_for(&model, viewport, scale);
        let placed = placed_ids(&frame);
        for id in [
            "library.close",
            "library.refresh",
            "library.request.retry",
            "library.request.cancel",
        ] {
            assert!(
                placed.contains(id),
                "{viewport} at {scale}: header control {id} missing"
            );
        }
        if frame.layout.mode == WorkshopLayoutMode::Compact {
            assert!(
                placed.contains("library.sheet.transfer"),
                "{viewport} at {scale}: Compact has no route to transfer"
            );
            assert!(
                frame.drawer.is_none(),
                "{viewport} at {scale}: a closed sheet is drawn"
            );
            assert!(
                placed.is_disjoint(&deferred),
                "{viewport} at {scale}: Compact placed a slice-2 control"
            );
            for id in &deferred {
                let node = frame
                    .semantics
                    .nodes_depth_first()
                    .into_iter()
                    .find(|node| node.action_id.as_ref().map(SemanticActionId::as_str) == Some(id))
                    .unwrap_or_else(|| panic!("{id} vanished from the semantic tree"));
                assert!(
                    node.bounds.is_none() && !node.visible,
                    "{id} is unplaced but visible"
                );
            }
        } else {
            assert!(!placed.contains("library.sheet.transfer"));
            assert!(
                deferred.is_subset(&placed),
                "{viewport} at {scale}: missing {:?}",
                deferred.difference(&placed).collect::<Vec<_>>()
            );
        }

        // Rows are placed as a prefix, never with a gap: slice 2's reveal can
        // then extend the prefix without reordering anything.
        let row_ids: Vec<&str> = model
            .rows
            .iter()
            .map(|row| row.control.action_id.as_str())
            .collect();
        let placed_rows = row_ids
            .iter()
            .take_while(|id| placed.contains(**id))
            .count();
        assert!(
            placed_rows >= 1,
            "{viewport} at {scale}: not even one row fits"
        );
        assert!(
            row_ids[placed_rows..]
                .iter()
                .all(|id| !placed.contains(*id)),
            "{viewport} at {scale}: rows were placed with a gap"
        );
    }
}

/// Every confirmation fits and owns the frame, at every matrix point, with or
/// without the Compact sheet under it.
///
/// The body must show in full without paging: the Workshop's modal paging
/// controls are Workshop view actions, so a Library dialog that needed them
/// would have no way to reach its own text. The crowded fixture's 64-byte
/// names are the widest body this dialog can hold.
#[test]
fn every_library_dialog_fits_without_paging_and_owns_the_frame() {
    let list = crowded_list();
    // A full-length draft that is also invalid (trailing space): the field,
    // the notice and the rules sentence all at once.
    let draft = format!("{} ", "W".repeat(63));
    let dialogs = [
        // Slot 0 is the Continue save: the longest Archive sentence.
        (LibraryConfirmationKind::Archive, SlotId(0)),
        (LibraryConfirmationKind::Unarchive, SlotId(2)),
        (LibraryConfirmationKind::Rename, SlotId(0)),
    ];
    for (kind, slot) in dialogs {
        let order = library_confirmation_order(kind);
        let model = LibraryUiModel::build(LibraryUiContext {
            confirmation: Some(LibraryConfirmationRequest { kind, slot }),
            rename_draft: Some(&draft),
            ..crowded_context(&list)
        });
        if kind == LibraryConfirmationKind::Rename {
            assert!(model.confirmation.as_ref().unwrap().problem.is_some());
        }
        let body = model
            .confirmation
            .as_ref()
            .expect("dialog built")
            .body
            .clone();
        for ((viewport, scale), sheet) in sheet_cases() {
            let case = format!("{kind:?} at {viewport} {scale} {sheet:?}");
            let frame = frame_with(&model, viewport, scale, view(sheet));
            let modal = frame.modal.unwrap_or_else(|| panic!("{case}: no modal"));
            let screen = PlatformRect::from_xywh(0.0, 0.0, viewport.x, viewport.y);
            assert!(
                screen.contains_rect(modal),
                "{case}: modal leaves the screen"
            );
            install(&frame).unwrap_or_else(|error| panic!("{case}: refused: {error}"));
            assert_eq!(
                frame.logical_focus_order, order,
                "{case}: focus not trapped"
            );
            for control in &frame.controls {
                let id = control.action_id.as_str();
                if order.contains(&control.action_id) {
                    assert!(modal.contains_rect(control.bounds), "{case}: {id} outside");
                    assert!(
                        control.bounds.width() >= 44.0 && control.bounds.height() >= 44.0,
                        "{case}: {id} is {:?}",
                        control.bounds
                    );
                } else {
                    assert!(!control.enabled, "{case}: {id} is live behind the dialog");
                }
            }
            if kind == LibraryConfirmationKind::Rename {
                assert!(
                    frame.visible_nodes.iter().any(|record| {
                        record.semantic_id.as_str() == "library.confirm.problem"
                            && !record.display_text.is_empty()
                    }),
                    "{case}: the notice was not shown"
                );
                let field = frame
                    .controls
                    .iter()
                    .find(|control| control.action_id.as_str() == "library.confirm.name")
                    .unwrap_or_else(|| panic!("{case}: the field was not placed"));
                let submit = frame
                    .controls
                    .iter()
                    .find(|control| control.action_id.as_str() == "library.confirm.submit")
                    .unwrap();
                assert!(
                    field.bounds.max.y <= submit.bounds.min.y,
                    "{case}: the field is not above the footer"
                );
            }
            let shown: String = frame
                .visible_nodes
                .iter()
                .filter(|record| record.semantic_id.as_str() == "library.confirm.body")
                .flat_map(|record| record.prewrapped_lines.clone().unwrap_or_default())
                .collect();
            assert_eq!(shown, body, "{case}: the body was cut");
            assert!(
                frame.visible_nodes.iter().all(|record| record
                    .semantic_id
                    .as_str()
                    .starts_with("library.confirm")
                    || record
                        .semantic_id
                        .as_str()
                        .starts_with("platform.library.confirm")
                    || record
                        .action_id
                        .as_ref()
                        .is_some_and(|id| order.contains(id))),
                "{case}: text behind the dialog is still presented"
            );
        }
    }
}

/// The design asks the docked transfer strip to degrade into a scrolling strip
/// rather than drop a control (baseline Finding 5). The strip spans the whole
/// bottom bar, and the narrowest docked window is 900 logical pixels, so four
/// controls never wrap and there is nothing to scroll. This sweep is what
/// makes that a checked fact instead of arithmetic: every docked width from
/// each mode's floor to 1920, at every scale and at the shortest and a tall
/// height, places all four transfer controls on one line inside the bar and
/// all five selected-save actions in the panel. If a label, a scale or a
/// control count ever changes that, this fails before a control goes missing.
#[test]
fn every_docked_width_places_every_transfer_control_and_action_on_screen() {
    let list = crowded_list();
    let model = crowded_model(&list);
    for scale in [0.85_f32, 1.0, 1.15, 1.3] {
        let medium_floor = (900.0 * scale).ceil() as u32;
        let wide_floor = (1200.0 * scale).ceil() as u32;
        // Every width near a mode floor, every seventh one elsewhere.
        let widths = (medium_floor..=1920).filter(|width| {
            width % 7 == 0
                || width.abs_diff(medium_floor) <= 3
                || width.abs_diff(wide_floor) <= 3
                || *width == 1920
        });
        let shortest = (300.0 + 114.0 * scale).ceil();
        for width in widths {
            for height in [shortest, 900.0] {
                let viewport = Vec2::new(width as f32, height);
                let case = format!("{viewport} at {scale}");
                let frame = frame_for(&model, viewport, scale);
                assert_ne!(frame.layout.mode, WorkshopLayoutMode::Compact, "{case}");
                let bar = frame.layout.bottom_bar;
                let panel = frame.layout.right_panel.expect("docked layouts have one");
                let bounds = |id: &str| {
                    frame
                        .controls
                        .iter()
                        .find(|control| control.action_id.as_str() == id)
                        .map(|control| control.bounds)
                        .unwrap_or_else(|| panic!("{case}: {id} was dropped"))
                };
                let strip: Vec<PlatformRect> = TRANSFER_IDS.iter().map(|id| bounds(id)).collect();
                for (id, rect) in TRANSFER_IDS.iter().zip(&strip) {
                    assert!(bar.contains_rect(*rect), "{case}: {id} leaves the bar");
                    assert_eq!(rect.min.y, strip[0].min.y, "{case}: {id} wrapped");
                }
                for id in ACTION_IDS {
                    assert!(
                        panel.contains_rect(bounds(id)),
                        "{case}: {id} leaves the panel"
                    );
                }
                if width.abs_diff(medium_floor) <= 3 || width.abs_diff(wide_floor) <= 3 {
                    install(&frame).unwrap_or_else(|error| {
                        panic!("{case}: the SDF batch refused the frame: {error}")
                    });
                }
            }
        }
    }
}

/// Where a frame places a section's rows, the section heading is drawn
/// directly above the first of them and nowhere else; a section with no
/// placed rows draws no heading.
fn assert_headings_lead_their_rows(model: &LibraryUiModel, frame: &PlatformUiFrame, case: &str) {
    for (section, heading) in [
        (LibrarySection::Active, "library.section.active.heading"),
        (LibrarySection::Archived, "library.section.archived.heading"),
    ] {
        let first_placed = model
            .rows
            .iter()
            .filter(|row| row.section == section)
            .find_map(|row| {
                frame
                    .controls
                    .iter()
                    .find(|control| control.action_id == row.control.action_id)
            });
        let node = frame.semantics.node(heading);
        let Some(first) = first_placed else {
            assert!(
                node.is_none_or(|node| !node.visible && node.bounds.is_none()),
                "{case}: {heading} is drawn with no row under it"
            );
            assert!(
                !frame
                    .visible_nodes
                    .iter()
                    .any(|record| record.semantic_id.as_str() == heading),
                "{case}: {heading} has drawn text with no row under it"
            );
            continue;
        };
        let node = node.unwrap_or_else(|| panic!("{case}: {heading} missing"));
        let bounds = node
            .bounds
            .unwrap_or_else(|| panic!("{case}: {heading} has rows but is not drawn"));
        assert!(node.visible, "{case}: {heading} not visible");
        let bounds = PlatformRect::from_xywh(
            bounds.min[0],
            bounds.min[1],
            bounds.max[0] - bounds.min[0],
            bounds.max[1] - bounds.min[1],
        );
        assert!(
            bounds.max.y <= first.bounds.min.y,
            "{case}: {heading} is not above its first row"
        );
        assert!(
            first.bounds.min.y - bounds.max.y <= 16.0,
            "{case}: {heading} is detached from its first row"
        );
        assert!(
            frame.layout.canvas.contains_rect(bounds),
            "{case}: {heading} leaves the canvas"
        );
        for control in &frame.controls {
            assert!(
                !control.bounds.overlaps(bounds),
                "{case}: {heading} overlaps {}",
                control.action_id.as_str()
            );
        }
        assert!(
            frame
                .visible_nodes
                .iter()
                .any(|record| record.semantic_id.as_str() == heading
                    && record.display_text.starts_with(&node.name)),
            "{case}: {heading} has no drawn text"
        );
    }
}

#[test]
fn section_headings_are_drawn_above_their_first_placed_row() {
    let crowded = crowded_list();
    let crowded_model = crowded_model(&crowded);
    for (viewport, scale) in viewports() {
        let frame = frame_for(&crowded_model, viewport, scale);
        assert_headings_lead_their_rows(&crowded_model, &frame, &format!("{viewport} at {scale}"));
    }

    // A short list, so both sections are on screen together.
    let short = SlotList {
        slots: crowded_list().slots.into_iter().take(6).collect(),
        selected_continue: Some(SlotId(0)),
    };
    let short_model = LibraryUiModel::build(LibraryUiContext {
        slots: Some(&short),
        ..LibraryUiContext::default()
    });
    for (viewport, scale) in viewports() {
        let frame = frame_for(&short_model, viewport, scale);
        let case = format!("short {viewport} at {scale}");
        assert_headings_lead_their_rows(&short_model, &frame, &case);
        // Six rows and two headings need a tall canvas; the short windows
        // are covered by the ordering check above.
        if frame.layout.mode != WorkshopLayoutMode::Compact && viewport.y >= 900.0 {
            for heading in [
                "library.section.active.heading",
                "library.section.archived.heading",
            ] {
                assert!(
                    frame
                        .semantics
                        .node(heading)
                        .is_some_and(|node| node.visible),
                    "{case}: {heading} should fit"
                );
            }
        }
    }
}

#[test]
fn next_reaches_every_row_and_each_window_keeps_its_invariants() {
    // Slice 2b: a row that does not fit is reached by paging, through two
    // placed controls, so pointer and keyboard still agree about every slot.
    let list = crowded_list();
    let model = crowded_model(&list);
    for (viewport, scale) in viewports() {
        let mut seen = BTreeSet::new();
        let mut start = 0;
        for step in 0..=model.rows.len() {
            let case = format!("{viewport} at {scale}, start {start}");
            let frame = frame_at(&model, viewport, scale, start);
            install(&frame).unwrap_or_else(|error| panic!("{case}: {error}"));
            assert_headings_lead_their_rows(&model, &frame, &case);
            let rows = placed_rows(&model, &frame);
            assert!(!rows.is_empty(), "{case}: no row placed");
            assert_eq!(rows[0], start, "{case}: the window must start at row_start");
            assert!(
                rows.windows(2).all(|pair| pair[1] == pair[0] + 1),
                "{case}: rows were placed with a gap"
            );
            let placed = placed_ids(&frame);
            let paged = PAGER_IDS.iter().all(|id| placed.contains(*id));
            assert_eq!(
                paged,
                rows.len() < model.rows.len(),
                "{case}: the pager must appear exactly when rows overflow"
            );
            assert!(
                PAGER_IDS.iter().all(|id| placed.contains(*id))
                    || !PAGER_IDS.iter().any(|id| placed.contains(*id)),
                "{case}: only one pager control was placed"
            );
            seen.extend(rows.iter().copied());
            let next = rows[rows.len() - 1] + 1;
            if next >= model.rows.len() {
                break;
            }
            assert!(step < model.rows.len(), "{case}: paging did not terminate");
            start = next;
        }
        assert_eq!(
            seen.len(),
            model.rows.len(),
            "{viewport} at {scale}: some rows are unreachable"
        );
    }
}

#[test]
fn a_list_that_fits_has_no_pager_and_a_stale_start_is_clamped() {
    let short = SlotList {
        slots: crowded_list().slots.into_iter().take(3).collect(),
        selected_continue: Some(SlotId(0)),
    };
    let model = LibraryUiModel::build(LibraryUiContext {
        slots: Some(&short),
        ..LibraryUiContext::default()
    });
    let frame = frame_for(&model, Vec2::new(1440.0, 900.0), 1.0);
    let placed = placed_ids(&frame);
    assert!(PAGER_IDS.iter().all(|id| !placed.contains(*id)));
    assert_eq!(placed_rows(&model, &frame), vec![0, 1, 2]);

    // Refresh can shrink the list under a window that had moved on.
    let frame = frame_at(&model, Vec2::new(1440.0, 900.0), 1.0, 40);
    assert_eq!(placed_rows(&model, &frame), vec![2]);
}

#[test]
fn the_compact_sheet_holds_its_page_and_covers_the_list() {
    let list = crowded_list();
    let model = crowded_model(&list);
    for (viewport, scale) in viewports() {
        for (page, ids, other) in [
            (LibrarySheet::Actions, &ACTION_IDS[..], &TRANSFER_IDS[..]),
            (LibrarySheet::Transfer, &TRANSFER_IDS[..], &ACTION_IDS[..]),
        ] {
            let case = format!("{viewport} at {scale} {page:?}");
            let closed = frame_for(&model, viewport, scale);
            let frame = frame_with(&model, viewport, scale, view(Some(page)));
            if frame.layout.mode != WorkshopLayoutMode::Compact {
                // Wide and Medium dock both surfaces; the sheet does not exist.
                assert_eq!(frame, closed, "{case}: a docked layout drew a sheet");
                continue;
            }
            let sheet = frame
                .drawer
                .unwrap_or_else(|| panic!("{case}: no sheet drawn"));
            assert_eq!(sheet, frame.layout.drawer_sheet);
            let placed = placed_ids(&frame);
            for id in ids {
                let control = frame
                    .controls
                    .iter()
                    .find(|control| control.action_id.as_str() == *id)
                    .unwrap_or_else(|| panic!("{case}: {id} not placed"));
                assert!(
                    sheet.contains_rect(control.bounds),
                    "{case}: {id} outside the sheet"
                );
            }
            // Back takes Transfer's slot in the bottom bar.
            let back = frame
                .controls
                .iter()
                .find(|control| control.action_id.as_str() == "library.sheet.close")
                .unwrap_or_else(|| panic!("{case}: no way back"));
            assert!(
                frame.layout.bottom_bar.contains_rect(back.bounds),
                "{case}: Back is not in the bar"
            );
            assert!(
                !placed.contains("library.sheet.transfer"),
                "{case}: Transfer and Back both placed"
            );
            assert!(
                other.iter().all(|id| !placed.contains(*id)),
                "{case}: both pages placed"
            );
            // Nothing under the sheet can be pressed, focused or read.
            for control in &frame.controls {
                assert!(
                    sheet.contains_rect(control.bounds) || !control.bounds.overlaps(sheet),
                    "{case}: {} under the sheet",
                    control.action_id.as_str()
                );
            }
            for record in &frame.visible_nodes {
                let is_control = frame
                    .controls
                    .iter()
                    .any(|control| control.semantic_id == record.semantic_id);
                if !is_control && record.semantic_id.as_str().starts_with("library.") {
                    assert!(
                        !record.bounds.overlaps(sheet),
                        "{case}: {} is drawn under the sheet",
                        record.semantic_id.as_str()
                    );
                }
            }
            let order = frame.focus_order();
            let back = order
                .iter()
                .position(|id| id.as_str() == "library.sheet.close")
                .unwrap();
            assert_eq!(
                order[back + 1].as_str(),
                ids[0],
                "{case}: Back is not right before the page"
            );
        }
    }
}

#[test]
fn disabled_reasons_and_the_content_line_survive_into_the_frame() {
    // The model pins that every disabled node states its reason. A platform
    // layer that rebuilt the tree could strip that, so it is read again here.
    let list = crowded_list();
    let model = crowded_model(&list);
    let frame = frame_for(&model, Vec2::new(1440.0, 900.0), 1.0);
    let mut disabled = 0;
    for control in frame.controls.iter().filter(|control| !control.enabled) {
        disabled += 1;
        let node = frame
            .semantics
            .node(control.semantic_id.as_str())
            .expect("a placed control keeps its model node");
        assert!(
            node.value.as_deref().is_some_and(|value| !value.is_empty()),
            "{} lost its disabled reason",
            control.action_id.as_str()
        );
    }
    assert!(disabled > 0, "this shape must contain disabled controls");

    let content = frame
        .semantics
        .node("library.content")
        .expect("the content line is in the tree once");
    assert!(
        content.visible && content.bounds.is_some(),
        "the content line is not shown"
    );
    assert_eq!(
        frame
            .semantics
            .nodes_depth_first()
            .into_iter()
            .filter(|node| node.name == "Saved galaxies"
                && node.action_id.is_none()
                && node.children.is_empty())
            .count(),
        1,
        "the content line must not be copied into a second, status-line node"
    );
    assert!(frame.status_lines.is_empty());
}

/// Task 12a's request states carry the longest request labels ("Open
/// previous", "Refresh") and the longest status lines, so every one of them
/// must still install, keep 44-pixel controls, and place Cancel wherever the
/// crowded Rename failure places it.
#[test]
fn every_open_request_state_installs_and_keeps_its_decision_controls() {
    let list = crowded_list();
    let open_failed = |code| LibrarySlotsStatus::Failed {
        kind: SlotRequestKind::Open,
        slot: Some(SlotId(1)),
        code,
    };
    let statuses = [
        LibrarySlotsStatus::Working {
            kind: SlotRequestKind::Open,
            slot: Some(SlotId(1)),
        },
        open_failed(ClientDiagnosticCode::StaleSave),
        open_failed(ClientDiagnosticCode::Store),
        LibrarySlotsStatus::Held {
            slot: SlotId(1),
            recovered: true,
        },
        LibrarySlotsStatus::Held {
            slot: SlotId(1),
            recovered: false,
        },
    ];
    let reference = crowded_model(&list);
    for status in statuses {
        let model = LibraryUiModel::build(LibraryUiContext {
            status,
            ..crowded_context(&list)
        });
        for ((viewport, scale), sheet) in sheet_cases() {
            let case = format!("{status:?} at {viewport} x{scale} {sheet:?}");
            let frame = frame_with(&model, viewport, scale, view(sheet));
            install(&frame).unwrap_or_else(|error| panic!("{case}: refused: {error}"));
            for control in &frame.controls {
                assert!(
                    control.bounds.width() >= 44.0 && control.bounds.height() >= 44.0,
                    "{case}: {} is undersized",
                    control.action_id.as_str()
                );
            }
            let placed = placed_ids(&frame);
            let expected = placed_ids(&frame_with(&reference, viewport, scale, view(sheet)));
            for id in ["library.request.retry", "library.request.cancel"] {
                let offered = model.controls().any(|c| c.action_id.as_str() == id);
                if offered && expected.contains(id) {
                    assert!(placed.contains(id), "{case}: {id} was dropped");
                }
            }
        }
    }
}

/// Task 12c's request states, and task 11's handoff states: the
/// export-recovery offer, the prepared copy, the handoff in flight, a failed
/// handoff (whose recovered status line is the longest the strip carries) and
/// the receipt. Each must install, keep 44-pixel controls, and place its strip
/// controls wherever the crowded failure places Retry and Cancel. The stage-2
/// handoff has its own identifier and must take Retry's place, never be
/// dropped, with or without an adapter.
#[test]
fn every_export_request_state_installs_and_keeps_its_strip_controls() {
    let list = crowded_list();
    let ready = |source| LibrarySlotsStatus::ExportReady { source };
    let head = ExportSource::Head {
        slot: SlotId(1),
        generation: SaveGeneration(2),
    };
    let predecessor = ExportSource::RecoveredPredecessor {
        slot: SlotId(1),
        generation: SaveGeneration(2),
    };
    let unsaved = ExportSource::ActiveWorkshop {
        continue_ready: false,
    };
    let statuses = [
        LibrarySlotsStatus::Working {
            kind: SlotRequestKind::Export,
            slot: Some(SlotId(1)),
        },
        LibrarySlotsStatus::Failed {
            kind: SlotRequestKind::Export,
            slot: Some(SlotId(1)),
            code: ClientDiagnosticCode::Store,
        },
        LibrarySlotsStatus::ExportRecoveryOffered { slot: SlotId(1) },
        ready(head),
        ready(predecessor),
        ready(unsaved),
        ready(ExportSource::ActiveWorkshop {
            continue_ready: true,
        }),
        ready(ExportSource::ActivePack {
            hash: nyon::workshop::CatalogHash([0xab; 32]),
        }),
        LibrarySlotsStatus::Working {
            kind: SlotRequestKind::HandOff,
            slot: Some(SlotId(1)),
        },
        LibrarySlotsStatus::HandOffFailed {
            source: ExportSource::RecoveredPredecessor {
                slot: SlotId(1),
                generation: SaveGeneration(2),
            },
            code: TransferFailureCode::Platform,
        },
        LibrarySlotsStatus::ExportHandedOff {
            source: ExportSource::RecoveredPredecessor {
                slot: SlotId(1),
                generation: SaveGeneration(2),
            },
            outcome: HandoffOutcome::HandedToSystem,
        },
        LibrarySlotsStatus::HandOffFailed {
            source: unsaved,
            code: TransferFailureCode::Platform,
        },
        LibrarySlotsStatus::ExportHandedOff {
            source: unsaved,
            outcome: HandoffOutcome::HandedToSystem,
        },
    ];
    let reference = crowded_model(&list);
    let mut handoffs_placed = 0;
    for status in statuses {
        for handoff_available in [false, true] {
            let model = LibraryUiModel::build(LibraryUiContext {
                status,
                handoff_available,
                ..crowded_context(&list)
            });
            for ((viewport, scale), sheet) in sheet_cases() {
                let case = format!(
                    "{status:?} adapter={handoff_available} at {viewport} x{scale} {sheet:?}"
                );
                let frame = frame_with(&model, viewport, scale, view(sheet));
                install(&frame).unwrap_or_else(|error| panic!("{case}: refused: {error}"));
                for control in &frame.controls {
                    assert!(
                        control.bounds.width() >= 44.0 && control.bounds.height() >= 44.0,
                        "{case}: {} is undersized",
                        control.action_id.as_str()
                    );
                }
                let placed = placed_ids(&frame);
                let expected = placed_ids(&frame_with(&reference, viewport, scale, view(sheet)));
                for (id, reference_id) in [
                    ("library.request.retry", "library.request.retry"),
                    ("library.request.handoff", "library.request.retry"),
                    ("library.request.cancel", "library.request.cancel"),
                ] {
                    let offered = model.controls().any(|c| c.action_id.as_str() == id);
                    if offered && expected.contains(reference_id) {
                        assert!(placed.contains(id), "{case}: {id} was dropped");
                        if id == "library.request.handoff" {
                            handoffs_placed += 1;
                        }
                    }
                }
            }
        }
    }
    assert!(handoffs_placed > 0, "no case placed the handoff at all");
}
