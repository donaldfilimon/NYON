//! The Library platform frame: route-design task 8, slice 1.
//!
//! `workshop_ui_library` pins the model. This suite pins what the platform
//! layer does with it: that every frame survives the real SDF batch, that every
//! placed control is at least 44 by 44 and reachable by both pointer and
//! keyboard, and that the two deliberate slice-1 gaps -- Compact's unplaced
//! actions and transfer controls, and rows past the canvas -- are exactly what
//! is missing and nothing more.

mod common;

use std::collections::BTreeSet;

use glam::Vec2;
use nyon::{
    app::client_runtime::{ClientDiagnosticCode, LibrarySlotsStatus, SlotRequestKind},
    engine::primitives::PrimitiveBatch,
    ui::{
        AtlasMetrics, UiBatch,
        accessibility::SemanticActionId,
        library::{LibraryUiContext, LibraryUiModel},
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
    LibraryUiModel::build(LibraryUiContext {
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
    })
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
    let layout = WorkshopLayout::resolve(viewport, scale).expect("test viewport resolves");
    build_library_platform_frame(model, layout, None, false)
}

fn install(frame: &PlatformUiFrame) -> Result<(), String> {
    let metrics = AtlasMetrics::embedded().unwrap();
    let mut ui = UiBatch::default();
    let mut overlay = PrimitiveBatch::default();
    install_platform_batches(frame, &metrics, &mut ui, &mut overlay)
        .map(|_| ())
        .map_err(|error| format!("{error:?}"))
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
        let frame = frame_for(&model, viewport, scale);
        if let Err(error) = install(&frame) {
            panic!("{viewport} at {scale}: the SDF batch refused the Library frame: {error}");
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
    for (viewport, scale) in viewports() {
        let frame = frame_for(&model, viewport, scale);
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
    for (viewport, scale) in viewports() {
        let frame = frame_for(&model, viewport, scale);
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
        let frame_order: Vec<&str> = focus.iter().map(SemanticActionId::as_str).collect();
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
            assert_eq!(
                hit.action,
                PlatformUiAction::Library(control.action_id.clone()),
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
