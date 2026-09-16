//! Library confirmations and the rename field: route-design task 9.
//!
//! Addendum §3 requires lifecycle actions to use explicit confirmation text,
//! "including whether Archive will clear Continue", and Rename to validate
//! through the canonical `SlotName`. Rename, Archive and Unarchive therefore
//! do not reach the store themselves: they ask the dispatcher to open one of
//! these dialogs, and only the dialog submits the change.
//!
//! **The rename draft is client state and survives refusal.** A name the store
//! would reject disables the confirm control with [`LibraryDisabledReason::InvalidName`]
//! and shows a one-line notice, but the draft stays; only Cancel, Escape or a
//! submitted rename discard it.
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
    workshop::store::{MAX_SLOT_NAME_BYTES, SlotId, SlotName},
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

/// The rename dialog's text field.
pub const LIBRARY_RENAME_FIELD_ACTION: &str = "library.confirm.name";

/// The dialog's semantic node id.
pub const LIBRARY_CONFIRM_DIALOG: &str = "library.confirm-dialog";

/// The confirmation's focus order, for callers that must open the trap before
/// a frame exists. Rename starts in its field, so typing replaces the name.
pub fn library_confirmation_order(kind: LibraryConfirmationKind) -> Vec<SemanticActionId> {
    let mut order = Vec::with_capacity(3);
    if kind == LibraryConfirmationKind::Rename {
        order.push(SemanticActionId::new(LIBRARY_RENAME_FIELD_ACTION));
    }
    order.push(SemanticActionId::new(LIBRARY_CONFIRM_CANCEL_ACTION));
    order.push(SemanticActionId::new(LIBRARY_CONFIRM_SUBMIT_ACTION));
    order
}

/// Whether a keystroke or browser edit may put `value` into the rename field.
///
/// Printable ASCII up to the store's byte limit, the characters `SlotName`
/// accepts: the atlas draws nothing else, and a longer draft could only ever
/// be refused. Spacing rules are left to live validation, because a name is
/// typed through states (a trailing space before the next word) that are
/// invalid only as a final answer.
pub fn rename_draft_acceptable(value: &str) -> bool {
    value.len() <= MAX_SLOT_NAME_BYTES && value.bytes().all(|byte| (b' '..=b'~').contains(&byte))
}

/// Which change a confirmation guards.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum LibraryConfirmationKind {
    Rename,
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
    /// Rename only: the text field. Its label shows the draft on one line;
    /// activating it submits, like Enter in any single-line form.
    pub name_field: Option<LibraryControl>,
    /// Rename only: the draft as typed.
    pub name_value: Option<String>,
    /// Rename only: why the draft cannot be submitted, when it cannot.
    pub problem: Option<&'static str>,
    pub cancel_control: LibraryControl,
    pub submit_control: LibraryControl,
}

impl LibraryConfirmationModel {
    pub(crate) fn controls(&self) -> Vec<&LibraryControl> {
        self.name_field
            .iter()
            .chain([&self.cancel_control, &self.submit_control])
            .collect()
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
    rename_draft: Option<&str>,
) -> Option<LibraryConfirmationModel> {
    let row = rows.iter().find(|row| row.slot == request.slot)?;
    let name = &row.name;
    let mut name_field = None;
    let mut name_value = None;
    let mut problem = None;
    let (title, body, label, description, reasons) = match request.kind {
        LibraryConfirmationKind::Rename => {
            // A draft the field could never hold is not shown; the row's own
            // name is. The dispatcher filters edits, so this is a backstop.
            let draft = rename_draft
                .filter(|draft| rename_draft_acceptable(draft))
                .unwrap_or(name);
            let invalid = SlotName::new(draft).is_err();
            problem = invalid.then_some("This name cannot be used yet.");
            name_field = Some(LibraryControl::enabled(
                LIBRARY_RENAME_FIELD_ACTION,
                format!("Name: {draft}"),
                "The new name for this save. Enter renames it.",
                false,
                LibraryUiIntent::SubmitConfirmation(request),
            ));
            name_value = Some(draft.to_owned());
            (
                "Rename save",
                "Use 1 to 64 letters, digits or symbols, with single spaces between words."
                    .to_owned(),
                "Rename",
                "Rename this saved galaxy.",
                [invalid.then_some(LibraryDisabledReason::InvalidName), lane],
            )
        }
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
        name_field,
        name_value,
        problem,
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
    let mut children = vec![SemanticNode::text(
        "library.confirm.body",
        SemanticRole::Text,
        &confirmation.body,
        "",
    )];
    if let Some(field) = &confirmation.name_field {
        let mut node = semantic_control(field, SemanticRole::TextInput);
        node.value.clone_from(&confirmation.name_value);
        children.push(node);
    }
    if let Some(problem) = confirmation.problem {
        children.push(SemanticNode::text(
            "library.confirm.problem",
            SemanticRole::Alert,
            problem,
            "",
        ));
    }
    children.push(semantic_control(
        &confirmation.cancel_control,
        SemanticRole::Button,
    ));
    children.push(semantic_control(
        &confirmation.submit_control,
        SemanticRole::Button,
    ));
    SemanticNode::container(
        LIBRARY_CONFIRM_DIALOG,
        SemanticRole::Dialog,
        confirmation.title,
        children,
    )
}
