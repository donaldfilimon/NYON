//! Library confirmations: route-design task 9.
//!
//! Addendum §3 requires lifecycle actions to use explicit confirmation text,
//! "including whether Archive will clear Continue". The Archive and Unarchive
//! controls therefore do not reach the store themselves: they ask the
//! dispatcher to open one of these dialogs, and only the dialog's confirm
//! control submits the change.
//!
//! **The confirm control carries the request it was built for.** The
//! dialog's wording and its submitted intent are derived from the same
//! [`LibraryConfirmationRequest`] in one place, so a dialog that says
//! "Archive" cannot submit an unarchive (task 7 review F2 is the reason this
//! pairing is spelled out rather than trusted).
//!
//! **The slot name lives in the body, never in the title.** The title is one
//! unwrapped run; the body wraps. A 64-byte name in the title would make the
//! frame fit worse at the 320 floor, so the title is a fixed short phrase.
//!
//! **The body is as short as §3 allows.** At 320x460 and scale 1.3 a dialog
//! body holds about seven wrapped lines and a 64-byte name takes four, and the
//! Library has no dialog paging. Lengthening this text fails
//! `every_library_dialog_fits_without_paging_and_owns_the_frame`.

use crate::{
    ui::accessibility::{SemanticActionId, SemanticNode, SemanticRole},
    workshop::store::SlotId,
};

use super::{LibraryControl, LibraryDisabledReason, LibraryRow, LibraryUiIntent};

/// Action IDs of the confirmation's controls, in focus order.
///
/// `src/app.rs` opens the focus trap before the model, and therefore the
/// semantic tree, has been rebuilt, so it reads these rather than deriving
/// them the way `platform_projection::modal_action_ids` does. Same reason as
/// `removal_modal_order` in the Workshop.
pub const LIBRARY_CONFIRM_CANCEL_ACTION: &str = "library.confirm.cancel";
pub const LIBRARY_CONFIRM_SUBMIT_ACTION: &str = "library.confirm.submit";

/// The dialog's semantic node id.
pub const LIBRARY_CONFIRM_DIALOG: &str = "library.confirm-dialog";

/// The confirmation's focus order, for callers that must open the trap before
/// a frame exists.
pub fn library_confirmation_order() -> [SemanticActionId; 2] {
    [
        SemanticActionId::new(LIBRARY_CONFIRM_CANCEL_ACTION),
        SemanticActionId::new(LIBRARY_CONFIRM_SUBMIT_ACTION),
    ]
}

/// Which change a confirmation guards.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum LibraryConfirmationKind {
    Archive,
    Unarchive,
}

/// What the user asked to confirm: pure client state, owned by the app.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct LibraryConfirmationRequest {
    pub kind: LibraryConfirmationKind,
    pub slot: SlotId,
}

/// The open confirmation, as presented.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LibraryConfirmationModel {
    pub request: LibraryConfirmationRequest,
    /// A fixed short phrase; see the module note.
    pub title: &'static str,
    /// The explicit confirmation text §3 requires, naming the save.
    pub body: String,
    pub cancel_control: LibraryControl,
    pub submit_control: LibraryControl,
}

impl LibraryConfirmationModel {
    pub(crate) fn controls(&self) -> [&LibraryControl; 2] {
        [&self.cancel_control, &self.submit_control]
    }
}

/// Builds the dialog for `request` over the row it names.
///
/// Returns `None` when the row is not in the current list: a confirmation
/// never describes a save the screen cannot show, and the dispatcher closes a
/// request that no longer resolves.
///
/// `lane` is the occupancy of the one slot-request lane. The submit control is
/// gated on it, and on residency for Archive, so the dialog never offers a
/// confirm the runtime is certain to refuse.
pub(super) fn build_confirmation(
    request: LibraryConfirmationRequest,
    rows: &[LibraryRow],
    lane: Option<LibraryDisabledReason>,
) -> Option<LibraryConfirmationModel> {
    let row = rows.iter().find(|row| row.slot == request.slot)?;
    let name = &row.name;
    let (title, body, label, description, reasons) = match request.kind {
        LibraryConfirmationKind::Archive => (
            "Archive save",
            format!(
                "Archive \"{name}\"? Nothing is deleted. {}",
                if row.selected_for_continue {
                    "Continue will be cleared."
                } else {
                    "Continue is not affected."
                }
            ),
            "Archive",
            "Archive this saved galaxy.",
            [
                row.resident.then_some(LibraryDisabledReason::ResidentSlot),
                lane,
            ],
        ),
        LibraryConfirmationKind::Unarchive => (
            "Unarchive save",
            format!(
                "Unarchive \"{name}\"? It is not opened and does not become the \
                 Continue save."
            ),
            "Unarchive",
            "Return this saved galaxy to the active list.",
            [None, lane],
        ),
    };
    Some(LibraryConfirmationModel {
        request,
        title,
        body,
        cancel_control: LibraryControl::enabled(
            LIBRARY_CONFIRM_CANCEL_ACTION,
            "Cancel",
            "Close this confirmation without changing the save.",
            false,
            LibraryUiIntent::CancelConfirmation,
        ),
        submit_control: LibraryControl::gated(
            LIBRARY_CONFIRM_SUBMIT_ACTION,
            label,
            description,
            false,
            LibraryUiIntent::SubmitConfirmation(request),
            &reasons,
        ),
    })
}

pub(super) fn confirmation_node(
    confirmation: &LibraryConfirmationModel,
    semantic_control: impl Fn(&LibraryControl, SemanticRole) -> SemanticNode,
) -> SemanticNode {
    SemanticNode::container(
        LIBRARY_CONFIRM_DIALOG,
        SemanticRole::Dialog,
        confirmation.title,
        vec![
            SemanticNode::text(
                "library.confirm.body",
                SemanticRole::Text,
                &confirmation.body,
                "",
            ),
            semantic_control(&confirmation.cancel_control, SemanticRole::Button),
            semantic_control(&confirmation.submit_control, SemanticRole::Button),
        ],
    )
}
