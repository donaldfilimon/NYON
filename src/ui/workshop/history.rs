//! Timeline, branch, and save-status models for the Workshop history surface,
//! split out of `workshop.rs`.

use nyon_workshop_core::RevisionId;

use crate::workshop::session::{WorkshopAction, WorkshopSessionSnapshot, WorkshopSpeed};

use super::{
    BranchChoice, SaveStatusModel, TimelineModel, WorkshopControl, WorkshopUiIntent, fixed_hex,
    safe_store_status, short_hex,
};

pub(super) fn build_timeline(
    snapshot: &WorkshopSessionSnapshot,
    redo_children: &[RevisionId],
    paused: bool,
    replacement_active: bool,
) -> TimelineModel {
    let mut controls = vec![
        WorkshopControl::new(
            "timeline.pause",
            "Pause",
            "Pause authoritative Workshop stepping.",
            !paused,
            paused,
            WorkshopUiIntent::Dispatch(WorkshopAction::Pause),
        ),
        WorkshopControl::new(
            "timeline.step",
            "Step once",
            "Advance exactly one authoritative tick while paused.",
            paused && !replacement_active,
            false,
            WorkshopUiIntent::Dispatch(WorkshopAction::StepOnce),
        ),
        speed_control("1x", WorkshopSpeed::One, snapshot, replacement_active),
        speed_control("4x", WorkshopSpeed::Four, snapshot, replacement_active),
        speed_control("20x", WorkshopSpeed::Twenty, snapshot, replacement_active),
        WorkshopControl::new(
            "timeline.undo",
            "Undo",
            "Browse the immutable parent revision at the current tick.",
            paused && !replacement_active && snapshot.active_view.view_cursor.is_some(),
            false,
            WorkshopUiIntent::Dispatch(WorkshopAction::Undo),
        ),
    ];

    for revision in redo_children {
        controls.push(WorkshopControl::new(
            format!("timeline.redo.{}", fixed_hex(&revision.0)),
            format!("Redo {}", short_hex(&revision.0)),
            "Select this existing child revision without rewriting history.",
            paused && !replacement_active,
            false,
            WorkshopUiIntent::Dispatch(WorkshopAction::Redo(*revision)),
        ));
    }

    TimelineModel {
        tick: snapshot.state.tick.0,
        speed: snapshot.speed,
        controls,
        redo_children: redo_children.to_vec(),
    }
}

fn speed_control(
    label: &'static str,
    speed: WorkshopSpeed,
    snapshot: &WorkshopSessionSnapshot,
    replacement_active: bool,
) -> WorkshopControl {
    WorkshopControl::new(
        format!("timeline.speed.{}", label.to_ascii_lowercase()),
        label,
        format!("Run the Workshop simulation at {label}."),
        !snapshot.store.load_pending && !replacement_active,
        snapshot.speed == speed,
        WorkshopUiIntent::Dispatch(WorkshopAction::Resume(speed)),
    )
}

pub(super) fn build_branches(
    snapshot: &WorkshopSessionSnapshot,
    paused: bool,
    replacement_active: bool,
) -> Vec<BranchChoice> {
    snapshot
        .branches
        .iter()
        .map(|branch| {
            let selected = branch.id == snapshot.active_view.selected_branch;
            let cursor = selected
                .then_some(snapshot.active_view.view_cursor)
                .flatten();
            let browsing_history = selected && branch.head != snapshot.active_view.view_cursor;
            let description = if browsing_history {
                format!(
                    "Selected branch; cursor {} is behind head {}.",
                    optional_revision(cursor),
                    optional_revision(branch.head)
                )
            } else {
                format!(
                    "Branch head {}; last authoritative tick {}.",
                    optional_revision(branch.head),
                    branch.last_tick.0
                )
            };
            BranchChoice {
                branch: branch.id,
                name: branch.name.clone(),
                head: branch.head,
                selected_cursor: cursor,
                last_tick: branch.last_tick.0,
                selected,
                browsing_history,
                control: WorkshopControl::new(
                    format!("branch.{}", fixed_hex(&branch.id.0)),
                    branch.name.clone(),
                    description,
                    paused && !replacement_active,
                    selected,
                    WorkshopUiIntent::Dispatch(WorkshopAction::SelectBranch(branch.id)),
                ),
            }
        })
        .collect()
}

/// The in-Workshop Library entry's action id. The app returns focus here when
/// the Library closes.
pub(crate) const WORKSHOP_LIBRARY_ACTION: &str = "save.library";

pub(super) fn build_save_status(snapshot: &WorkshopSessionSnapshot) -> SaveStatusModel {
    SaveStatusModel {
        slot: snapshot.store.slot.map(|slot| slot.0),
        generation: snapshot.store.generation.map(|generation| generation.0),
        dirty: snapshot.store.dirty,
        commit_pending: snapshot.store.commit_pending,
        load_pending: snapshot.store.load_pending,
        status: safe_store_status(&snapshot.store.status).to_owned(),
        save_control: WorkshopControl::new(
            "save.commit",
            "Save Workshop",
            "Commit the current archive using generation compare-and-swap.",
            !snapshot.store.commit_pending && !snapshot.store.load_pending,
            false,
            WorkshopUiIntent::Dispatch(WorkshopAction::RequestSave),
        ),
        library_control: WorkshopControl::new(
            WORKSHOP_LIBRARY_ACTION,
            "Library",
            "Manage saved galaxies. The Workshop stays open and unchanged.",
            true,
            false,
            WorkshopUiIntent::OpenLibrary,
        ),
    }
}

fn optional_revision(revision: Option<RevisionId>) -> String {
    revision.map_or_else(|| "genesis".to_owned(), |id| short_hex(&id.0))
}
