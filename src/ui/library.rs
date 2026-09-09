//! Immutable, render-platform-neutral UI model for the Workshop Library.
//!
//! This is the model half of the Library screen: the rows, the controls, the
//! typed intents they submit, and one deterministic focus order over all of
//! them. It builds no geometry and reads no layout. The docked panel, the
//! Compact sheet and the transfer strip are `src/ui/platform/library.rs`; the
//! confirmations and the rename field are the modal task after it.
//!
//! It deliberately does not live in `src/ui/workshop.rs`. That model belongs to
//! a resident Workshop session, and the Library exists precisely where there is
//! none — the main menu.
//!
//! # What the model must not get wrong
//!
//! Three runtime contracts are load-bearing here, and each one fails toward a
//! plausible-looking screen rather than a crash:
//!
//! 1. **`slots == None` with a listing request in flight is loading, not
//!    empty.** An empty store lists as `Some` with an empty `slots`, and every
//!    successful mutation drops the cached list for one frame before the
//!    automatic re-list lands. Rendering "no saves yet" there flashes a false
//!    empty on every rename. [`LibraryContent`] separates the four cases.
//! 2. **A preserved [`LibrarySlotsStatus::Failed`] is a pending decision.**
//!    `ClientRuntime::open_library` auto-lists only from `Idle`, so a failure
//!    left by a previous visit survives re-entry on purpose. The model always
//!    renders Retry and Cancel over it, cached rows or not.
//!    See [`LibraryRequestModel`].
//! 3. **A store failure never becomes the global recovery screen.** Addendum §6
//!    forbids collapsing Library failures into `RecoverableError`, so there is
//!    no intent in [`LibraryUiIntent`] that leaves this screen except
//!    [`LibraryUiIntent::Close`], which returns to the screen the Library was
//!    opened from.
//!
//! # Disabled, never omitted
//!
//! Controls whose backing capability has not shipped yet — the exact-catalog
//! open client, the transfer adapters — render **disabled with a visible
//! reason**, keeping their focus slot. Omitting them is baseline Finding 5
//! exactly: a control that exists logically and cannot be reached.
//! [`LibraryControl`] makes that structural, because `enabled` and
//! `disabled_reason` can only be set together.

use std::collections::BTreeMap;

use crate::{
    app::client_runtime::{ClientDiagnosticCode, LibrarySlotsStatus, SlotRequestKind},
    workshop::store::{SaveGeneration, SlotId, SlotList, SlotSummary},
};

use super::accessibility::{
    AnnouncementKind, InputModality, SemanticActionId, SemanticAnnouncement, SemanticNode,
    SemanticRole, SemanticTree,
};

/// Everything a Library control can ask the runtime to do.
///
/// Each variant is one user activation, not one store request. Whether Rename
/// opens a text field first, or Archive raises a confirmation, is the
/// dispatcher's decision and belongs to the modal task; interposing a dialog
/// there must not change this vocabulary.
///
/// Intents that name a generation carry the one observed in the activated row,
/// because addendum §4 requires the exact-catalog client to compare-and-swap
/// against it rather than silently accept a newer head. Rename, Archive and
/// Unarchive carry none: §3 makes them flag- and name-only mutations that
/// cannot open or replay a generation.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum LibraryUiIntent {
    /// Return to the exact screen the Library was opened from.
    Close,
    /// Re-list the bounded slots.
    RefreshSlots,
    /// Re-submit the retained failed slot request.
    RetrySlotRequest,
    /// Drop pending slot intent; never deletes anything already stored.
    CancelSlotRequest,
    /// Pure client selection. Identity is the [`SlotId`], never a row index.
    SelectSlot(SlotId),
    OpenSlot {
        slot: SlotId,
        generation: SaveGeneration,
    },
    RenameSlot {
        slot: SlotId,
    },
    ArchiveSlot {
        slot: SlotId,
    },
    UnarchiveSlot {
        slot: SlotId,
    },
    UseForContinue {
        slot: SlotId,
        generation: SaveGeneration,
    },
    ExportSlot {
        slot: SlotId,
        generation: SaveGeneration,
    },
    ImportArchive,
    ImportPack,
    ExportActiveArchive,
    ExportActivePack,
}

/// Why a control is present but not activatable.
///
/// Every reason is a bounded typed fact. The two that a client diagnostic
/// already describes report that code through
/// [`Self::diagnostic_code`] rather than restating the rule, so the screen
/// shows the runtime's own refusal text for the refusal the runtime performs.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum LibraryDisabledReason {
    /// No row is selected, so the action has no subject.
    NoSelection,
    /// The resident Workshop occupies this slot. §3.
    ResidentSlot,
    /// The row is archived and must be unarchived first. §3.
    ArchivedSlot,
    /// The row is already the selected Continue target.
    AlreadyContinue,
    /// The single slot-request lane is occupied by a request in flight.
    RequestInFlight,
    /// The lane holds a failure the user has not resolved yet.
    DecisionPending,
    /// The exact-catalog Library client is not wired to this screen yet.
    LibraryClientUnavailable,
    /// No portable transfer adapter is installed.
    TransferUnavailable,
    /// There is no active Workshop session to export.
    WorkshopInactive,
}

impl LibraryDisabledReason {
    /// The client diagnostic whose text already states this refusal, when one
    /// exists.
    ///
    /// Only [`Self::ResidentSlot`] has one today. The runtime refuses that case
    /// in `dispatch_slot_request` with exactly this code, so the presentation
    /// layer renders the same sentence for the pre-emptive disable and for a
    /// refusal that reached the store.
    pub const fn diagnostic_code(self) -> Option<ClientDiagnosticCode> {
        match self {
            Self::ResidentSlot => Some(ClientDiagnosticCode::ResidentSlot),
            _ => None,
        }
    }

    /// A bounded reason string for the semantic tree.
    ///
    /// Where a client diagnostic already states the refusal, this **delegates**
    /// to the runtime's own safe text rather than restating it. Copying that
    /// sentence would let the screen and the store drift into describing the
    /// same refusal two different ways, with nothing to catch it.
    pub const fn message(self) -> &'static str {
        match self {
            Self::NoSelection => "Select a save first.",
            Self::ResidentSlot => {
                crate::app::safe_client_diagnostic(ClientDiagnosticCode::ResidentSlot)
            }
            Self::ArchivedSlot => "Unarchive this save before opening or continuing it.",
            Self::AlreadyContinue => "This save is already the Continue target.",
            Self::RequestInFlight => "Another Library request is still running.",
            Self::DecisionPending => "Retry or cancel the failed Library request first.",
            Self::LibraryClientUnavailable => "Opening saved galaxies is not available yet.",
            Self::TransferUnavailable => "Portable file transfer is not available yet.",
            Self::WorkshopInactive => "Open a Workshop before exporting it.",
        }
    }
}

/// One activatable Library control.
///
/// `enabled` and `disabled_reason` are never set independently: an enabled
/// control has no reason and a disabled one always has exactly one. Both
/// constructors enforce it, so no caller can produce a disabled control with
/// nothing to display.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LibraryControl {
    pub action_id: SemanticActionId,
    pub label: String,
    pub description: String,
    pub enabled: bool,
    pub selected: bool,
    pub disabled_reason: Option<LibraryDisabledReason>,
    /// Private on purpose. A disabled action still needs a subject-shaped
    /// intent to exist so its box, label and focus slot survive, and with no
    /// selection that subject is a placeholder `SlotId(0)` — which is a **real
    /// allocatable slot identifier**, because the native store mints ids from
    /// zero upward. Reading this field directly on a disabled control would
    /// therefore name somebody's first save. [`Self::intent`] closes that by
    /// construction rather than by convention.
    intent: LibraryUiIntent,
}

impl LibraryControl {
    /// The intent this control submits, or `None` when it is disabled.
    ///
    /// The only sanctioned way to read an intent off a control. See the field's
    /// note for why a disabled control's intent must never be trusted.
    pub const fn intent(&self) -> Option<&LibraryUiIntent> {
        if self.enabled {
            Some(&self.intent)
        } else {
            None
        }
    }

    fn enabled(
        action_id: impl Into<String>,
        label: impl Into<String>,
        description: impl Into<String>,
        selected: bool,
        intent: LibraryUiIntent,
    ) -> Self {
        Self {
            action_id: SemanticActionId::new(action_id),
            label: label.into(),
            description: description.into(),
            enabled: true,
            selected,
            disabled_reason: None,
            intent,
        }
    }

    fn disabled(
        action_id: impl Into<String>,
        label: impl Into<String>,
        description: impl Into<String>,
        selected: bool,
        intent: LibraryUiIntent,
        reason: LibraryDisabledReason,
    ) -> Self {
        Self {
            action_id: SemanticActionId::new(action_id),
            label: label.into(),
            description: description.into(),
            enabled: false,
            selected,
            disabled_reason: Some(reason),
            intent,
        }
    }

    /// Builds an enabled control, or a disabled one carrying the first reason
    /// that applies.
    ///
    /// **Precedence, in one sentence:** no subject, then facts about the chosen
    /// row, then the occupancy of the one slot-request lane, then a capability
    /// that has not shipped — most specific to this row first, so a reason the
    /// user can act on now is never hidden behind a build-time deferral.
    fn gated(
        action_id: impl Into<String>,
        label: impl Into<String>,
        description: impl Into<String>,
        selected: bool,
        intent: LibraryUiIntent,
        reasons: &[Option<LibraryDisabledReason>],
    ) -> Self {
        match reasons.iter().flatten().next() {
            Some(reason) => {
                Self::disabled(action_id, label, description, selected, intent, *reason)
            }
            None => Self::enabled(action_id, label, description, selected, intent),
        }
    }
}

/// Which of the two list sections a row belongs to.
///
/// §2 requires archived saves to be a distinct section rather than an
/// inline flag, so grouping is part of the model and both the focus order and
/// the presentation read the same answer.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum LibrarySection {
    Active,
    Archived,
}

/// One slot row.
///
/// The row control takes [`SemanticRole::Option`] in the semantic tree, which
/// routes its label through the variable-label path to single-line ellipsis.
/// Slot names are the only user-authored string on this screen and may be 64
/// bytes; under any other role an oversized fixed label makes the SDF batch
/// reject the whole frame instead of truncating.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LibraryRow {
    pub slot: SlotId,
    pub name: String,
    pub generation: SaveGeneration,
    pub archived: bool,
    pub has_previous_generation: bool,
    pub selected_for_continue: bool,
    /// Whether the resident Workshop session occupies this slot.
    pub resident: bool,
    pub section: LibrarySection,
    pub control: LibraryControl,
}

/// What the row area is showing.
///
/// Derived from the cached list and the request status together, because
/// neither answers it alone: `None` means "there is no list to render", which
/// is a different fact from having listed an empty store.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum LibraryContent {
    /// A listing request is in flight and there is nothing cached to show.
    Loading,
    /// There is no list and nothing is fetching one. Refresh is the way out.
    NotListed,
    /// The store was listed and holds no saves.
    Empty,
    /// The store was listed and holds at least one save.
    Slots,
}

/// The one slot-request lane, as the user sees it.
///
/// `retry_control` exists only in [`LibrarySlotsStatus::Failed`];
/// `cancel_control` exists in both `Failed` and `Working`, because an in-flight
/// request occupies the Commit lane a resident Workshop needs to save.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LibraryRequestModel {
    pub status: LibrarySlotsStatus,
    /// The runtime's diagnostic for a failure, for presentation to render
    /// through the same safe-text mapping the rest of the product uses. The
    /// model never restates that sentence.
    pub failure_code: Option<ClientDiagnosticCode>,
    pub retry_control: Option<LibraryControl>,
    pub cancel_control: Option<LibraryControl>,
}

impl LibraryRequestModel {
    /// Whether the lane holds a decision the user has not resolved.
    pub const fn decision_pending(&self) -> bool {
        matches!(self.status, LibrarySlotsStatus::Failed { .. })
    }

    /// Whether a request currently occupies the lane.
    pub const fn in_flight(&self) -> bool {
        matches!(self.status, LibrarySlotsStatus::Working { .. })
    }
}

/// The five actions bound to the selected row.
///
/// They are panel controls rather than per-row buttons: rows live in the
/// canvas, the actions live in the docked panel or the Compact sheet, and
/// selecting a row is what aims them. With no selection every one of them is
/// present and disabled with [`LibraryDisabledReason::NoSelection`].
///
/// Archive and Unarchive are one slot in the panel and two distinct action
/// identifiers, so the intent submitted is never ambiguous about direction.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LibraryActionsModel {
    pub selected: Option<SlotId>,
    pub open: LibraryControl,
    pub rename: LibraryControl,
    /// Archive when the selected row is active, Unarchive when it is archived.
    pub archive: LibraryControl,
    pub use_for_continue: LibraryControl,
    pub export: LibraryControl,
}

impl LibraryActionsModel {
    fn controls(&self) -> [&LibraryControl; 5] {
        [
            &self.open,
            &self.rename,
            &self.archive,
            &self.use_for_continue,
            &self.export,
        ]
    }
}

/// The portable transfer strip.
///
/// Live with no adapter installed: every control keeps its label, its focus
/// slot and its disabled reason. §7 and §8 gate the byte handoff on a
/// compatibility spike, which is a reason to disable these, never to hide them.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LibraryTransferModel {
    pub import_archive: LibraryControl,
    pub import_pack: LibraryControl,
    pub export_active_archive: LibraryControl,
    pub export_active_pack: LibraryControl,
}

impl LibraryTransferModel {
    fn controls(&self) -> [&LibraryControl; 4] {
        [
            &self.import_archive,
            &self.import_pack,
            &self.export_active_archive,
            &self.export_active_pack,
        ]
    }
}

/// Everything the model needs that is not the slot list itself.
///
/// `resident_slot` is supplied by the caller because `ClientRuntime`'s own
/// `resident_slot()` is private; the model must not re-derive residency from a
/// session snapshot, or the screen and the store could disagree about which
/// refusal applies.
#[derive(Clone, Copy, Debug)]
pub struct LibraryUiContext<'a> {
    pub slots: Option<&'a SlotList>,
    pub status: LibrarySlotsStatus,
    /// The row the user chose. Filtered against the list, so a selection that
    /// named a slot the store no longer reports falls back to no selection.
    pub selected_slot: Option<SlotId>,
    pub resident_slot: Option<SlotId>,
    /// Whether a Workshop session is active and therefore exportable.
    pub workshop_active: bool,
    /// Whether the exact-catalog Library client is wired to Open, Use for
    /// Continue and row Export.
    pub library_client_available: bool,
    /// Whether a portable transfer adapter is installed.
    pub transfer_available: bool,
}

impl Default for LibraryUiContext<'_> {
    fn default() -> Self {
        Self {
            slots: None,
            status: LibrarySlotsStatus::Idle,
            selected_slot: None,
            resident_slot: None,
            workshop_active: false,
            library_client_available: false,
            transfer_available: false,
        }
    }
}

/// The immutable Library screen model.
#[derive(Clone, Debug, PartialEq)]
pub struct LibraryUiModel {
    pub close_control: LibraryControl,
    pub refresh_control: LibraryControl,
    pub request: LibraryRequestModel,
    pub content: LibraryContent,
    /// Active rows first, then archived, each group in the store's list order.
    pub rows: Vec<LibraryRow>,
    pub actions: LibraryActionsModel,
    pub transfer: LibraryTransferModel,
    pub semantics: SemanticTree,
    controls: BTreeMap<SemanticActionId, LibraryControl>,
    focus_order: Vec<SemanticActionId>,
}

impl LibraryUiModel {
    pub fn build(context: LibraryUiContext<'_>) -> Self {
        let request = build_request(context.status);
        let content = library_content(context.slots, context.status);
        let rows = build_rows(context);
        let selected_row = context
            .selected_slot
            .and_then(|slot| rows.iter().find(|row| row.slot == slot))
            .cloned();
        let actions = build_actions(selected_row.as_ref(), &request, context);
        let transfer = build_transfer(context);
        let close_control = build_close();
        let refresh_control = build_refresh(&request);

        let mut controls = BTreeMap::new();
        let mut focus_order = Vec::new();
        let mut ordered: Vec<&LibraryControl> = Vec::new();
        ordered.push(&close_control);
        ordered.push(&refresh_control);
        if let Some(control) = &request.retry_control {
            ordered.push(control);
        }
        if let Some(control) = &request.cancel_control {
            ordered.push(control);
        }
        ordered.extend(rows.iter().map(|row| &row.control));
        ordered.extend(actions.controls());
        ordered.extend(transfer.controls());
        for control in ordered {
            if controls
                .insert(control.action_id.clone(), control.clone())
                .is_none()
            {
                focus_order.push(control.action_id.clone());
            }
        }

        let semantics = build_semantic_tree(
            &close_control,
            &refresh_control,
            &request,
            content,
            &rows,
            &actions,
            &transfer,
        );

        Self {
            close_control,
            refresh_control,
            request,
            content,
            rows,
            actions,
            transfer,
            semantics,
            controls,
            focus_order,
        }
    }

    pub fn controls(&self) -> impl ExactSizeIterator<Item = &LibraryControl> {
        self.controls.values()
    }

    /// The deterministic traversal order.
    ///
    /// Header, then the pending decision, then every row, then the actions
    /// aimed at the selected row, then the transfer strip. Reading order in all
    /// three responsive modes, and stable under a selection change because the
    /// action identifiers do not depend on which row is selected.
    pub fn focus_order(&self) -> &[SemanticActionId] {
        &self.focus_order
    }

    pub fn activate(
        &self,
        action: &SemanticActionId,
        _modality: InputModality,
    ) -> Option<LibraryUiIntent> {
        self.controls
            .get(action)
            .filter(|control| control.enabled)
            .map(|control| control.intent.clone())
    }
}

/// Which of the four content states the row area is in.
///
/// The two `None` arms are the whole point. `None` never means "no saves":
/// an empty store lists as `Some` with an empty `slots`, and that is the only
/// representation of emptiness the runtime produces. `None` means there is no
/// list to render, and whether something is already fetching one is what
/// separates a loading frame from a resting state that needs Refresh.
const fn library_content(slots: Option<&SlotList>, status: LibrarySlotsStatus) -> LibraryContent {
    match slots {
        Some(list) => {
            if list.slots.is_empty() {
                LibraryContent::Empty
            } else {
                LibraryContent::Slots
            }
        }
        None => match status {
            LibrarySlotsStatus::Working {
                kind: SlotRequestKind::List,
                ..
            } => LibraryContent::Loading,
            _ => LibraryContent::NotListed,
        },
    }
}

fn build_close() -> LibraryControl {
    LibraryControl::enabled(
        "library.close",
        "Close",
        "Return to the screen the Library was opened from.",
        false,
        LibraryUiIntent::Close,
    )
}

/// Refresh is gated on the one slot-request lane being free.
///
/// `ClientRuntime::start_slot_request` accepts a request only from `Idle`, so
/// offering an enabled Refresh over a `Working` or `Failed` machine would offer
/// a control the runtime is certain to refuse.
fn build_refresh(request: &LibraryRequestModel) -> LibraryControl {
    LibraryControl::gated(
        "library.refresh",
        "Refresh",
        "List the saved galaxies again.",
        false,
        LibraryUiIntent::RefreshSlots,
        &[lane_reason(request)],
    )
}

/// The lane-occupancy reason, if the lane is not free.
const fn lane_reason(request: &LibraryRequestModel) -> Option<LibraryDisabledReason> {
    match request.status {
        LibrarySlotsStatus::Idle => None,
        LibrarySlotsStatus::Working { .. } => Some(LibraryDisabledReason::RequestInFlight),
        LibrarySlotsStatus::Failed { .. } => Some(LibraryDisabledReason::DecisionPending),
    }
}

/// Retry and Cancel over the slot-request lane.
///
/// A `Failed` state always produces both, including the one
/// `ClientRuntime::open_library` deliberately preserves across a visit: it did
/// not re-list, because re-listing would resolve a decision the user still
/// owns. Neither control ever routes to the recovery screen — §6 forbids
/// collapsing a Library failure into `RecoverableError`.
fn build_request(status: LibrarySlotsStatus) -> LibraryRequestModel {
    let (failure_code, retry_control, cancel_control) = match status {
        LibrarySlotsStatus::Idle => (None, None, None),
        LibrarySlotsStatus::Working { .. } => (
            None,
            None,
            Some(LibraryControl::enabled(
                "library.request.cancel",
                "Cancel",
                "Stop waiting for the Library request. Nothing already stored is removed.",
                false,
                LibraryUiIntent::CancelSlotRequest,
            )),
        ),
        LibrarySlotsStatus::Failed { code, .. } => (
            Some(code),
            Some(LibraryControl::enabled(
                "library.request.retry",
                "Retry",
                "Submit the failed Library request again.",
                false,
                LibraryUiIntent::RetrySlotRequest,
            )),
            Some(LibraryControl::enabled(
                "library.request.cancel",
                "Cancel",
                "Discard the failed Library request. Nothing already stored is removed.",
                false,
                LibraryUiIntent::CancelSlotRequest,
            )),
        ),
    };
    LibraryRequestModel {
        status,
        failure_code,
        retry_control,
        cancel_control,
    }
}

/// Builds one row per listed slot, active rows before archived ones.
///
/// Row order within a section is the store's own list order, and identity is
/// always the [`SlotId`]: the action identifier is derived from it, so a row
/// keeps its focus slot and its pointer target across a re-list that reorders
/// or removes neighbours.
fn build_rows(context: LibraryUiContext<'_>) -> Vec<LibraryRow> {
    let Some(list) = context.slots else {
        return Vec::new();
    };
    let mut rows: Vec<LibraryRow> = list
        .slots
        .iter()
        .filter(|summary| !summary.archived)
        .map(|summary| build_row(summary, context))
        .collect();
    rows.extend(
        list.slots
            .iter()
            .filter(|summary| summary.archived)
            .map(|summary| build_row(summary, context)),
    );
    rows
}

fn build_row(summary: &SlotSummary, context: LibraryUiContext<'_>) -> LibraryRow {
    let resident = context.resident_slot == Some(summary.id);
    let selected = context.selected_slot == Some(summary.id);
    LibraryRow {
        slot: summary.id,
        name: summary.name.as_str().to_owned(),
        generation: summary.generation,
        archived: summary.archived,
        has_previous_generation: summary.has_previous_generation,
        selected_for_continue: summary.selected_for_continue,
        resident,
        section: if summary.archived {
            LibrarySection::Archived
        } else {
            LibrarySection::Active
        },
        control: LibraryControl::enabled(
            row_action_id(summary.id),
            summary.name.as_str(),
            row_description(summary, resident),
            selected,
            LibraryUiIntent::SelectSlot(summary.id),
        ),
    }
}

fn row_action_id(slot: SlotId) -> String {
    format!("library.slot.{}", slot.0)
}

/// The five row facts §2 requires, as one bounded sentence.
///
/// The slot name is not repeated here: it is the control's label, which the
/// [`SemanticRole::Option`] role routes through the single-line ellipsis path.
fn row_description(summary: &SlotSummary, resident: bool) -> String {
    let mut description = format!(
        "Save {} at generation {}",
        summary.id.0, summary.generation.0
    );
    if summary.archived {
        description.push_str(", archived");
    }
    if summary.has_previous_generation {
        description.push_str(", previous generation retained");
    }
    if summary.selected_for_continue {
        description.push_str(", selected for Continue");
    }
    if resident {
        description.push_str(", open in the Workshop");
    }
    description.push('.');
    description
}

/// The five actions aimed at the selected row.
///
/// Every one of them exists in every state. With no selection they are disabled
/// with [`LibraryDisabledReason::NoSelection`] rather than removed, because a
/// control that disappears takes its focus slot and its layout box with it,
/// which is the defect this screen was told not to repeat.
///
/// Open, Use for Continue and row Export do not take the lane reason: §4 routes
/// them through the exact-catalog client instead of the slot-request lane. Once
/// that client is wired, §6's replacement and persistence gate has to be added
/// to them here — the `LibraryClientUnavailable` reason is what stands in for
/// it today, and it is not a substitute.
fn build_actions(
    row: Option<&LibraryRow>,
    request: &LibraryRequestModel,
    context: LibraryUiContext<'_>,
) -> LibraryActionsModel {
    let no_selection = row.is_none().then_some(LibraryDisabledReason::NoSelection);
    let archived = row
        .filter(|row| row.archived)
        .map(|_| LibraryDisabledReason::ArchivedSlot);
    let resident = row
        .filter(|row| row.resident)
        .map(|_| LibraryDisabledReason::ResidentSlot);
    let already_continue = row
        .filter(|row| row.selected_for_continue)
        .map(|_| LibraryDisabledReason::AlreadyContinue);
    let lane = lane_reason(request);
    let client = (!context.library_client_available)
        .then_some(LibraryDisabledReason::LibraryClientUnavailable);
    let slot = row.map(|row| row.slot);
    let generation = row.map_or(SaveGeneration(0), |row| row.generation);
    let subject = slot.unwrap_or(SlotId(0));
    let archiving = row.is_none_or(|row| !row.archived);

    LibraryActionsModel {
        selected: slot,
        open: LibraryControl::gated(
            "library.action.open",
            "Open",
            "Open this saved galaxy in the Workshop.",
            false,
            LibraryUiIntent::OpenSlot {
                slot: subject,
                generation,
            },
            &[no_selection, archived, client],
        ),
        rename: LibraryControl::gated(
            "library.action.rename",
            "Rename",
            "Give this saved galaxy a different name.",
            false,
            LibraryUiIntent::RenameSlot { slot: subject },
            &[no_selection, lane],
        ),
        // One action identifier for both directions, deliberately. The label,
        // the description and the intent all flip, but the identifier must not:
        // archiving the selected row is the case most likely to be confirmed
        // through a modal, and when the mutation lands the re-listed row comes
        // back archived. If the identifier flipped with it, the id the focus
        // manager is holding — and the one a modal restores to — would no
        // longer exist in the order, which is exactly the stable-focus
        // obligation in §2. The two intents are already distinct variants, so
        // nothing is ambiguous about which direction was submitted.
        archive: if archiving {
            LibraryControl::gated(
                "library.action.archive",
                "Archive",
                "Move this saved galaxy to the archived section. Clears Continue if it was selected.",
                false,
                LibraryUiIntent::ArchiveSlot { slot: subject },
                &[no_selection, resident, lane],
            )
        } else {
            LibraryControl::gated(
                "library.action.archive",
                "Unarchive",
                "Return this saved galaxy to the active section. Does not open it or restore Continue.",
                false,
                LibraryUiIntent::UnarchiveSlot { slot: subject },
                &[no_selection, lane],
            )
        },
        use_for_continue: LibraryControl::gated(
            "library.action.use-for-continue",
            "Use for Continue",
            "Make this exact generation the galaxy Continue opens.",
            false,
            LibraryUiIntent::UseForContinue {
                slot: subject,
                generation,
            },
            &[no_selection, archived, already_continue, client],
        ),
        export: LibraryControl::gated(
            "library.action.export",
            "Export",
            "Prepare a portable copy of this exact generation.",
            false,
            LibraryUiIntent::ExportSlot {
                slot: subject,
                generation,
            },
            &[no_selection, client],
        ),
    }
}

/// The four transfer controls, live with no adapter installed.
fn build_transfer(context: LibraryUiContext<'_>) -> LibraryTransferModel {
    let transfer =
        (!context.transfer_available).then_some(LibraryDisabledReason::TransferUnavailable);
    let inactive = (!context.workshop_active).then_some(LibraryDisabledReason::WorkshopInactive);
    LibraryTransferModel {
        import_archive: LibraryControl::gated(
            "library.transfer.import-archive",
            "Import galaxy",
            "Choose a portable galaxy file to validate and open.",
            false,
            LibraryUiIntent::ImportArchive,
            &[transfer],
        ),
        import_pack: LibraryControl::gated(
            "library.transfer.import-pack",
            "Import content pack",
            "Choose a portable content pack to store.",
            false,
            LibraryUiIntent::ImportPack,
            &[transfer],
        ),
        export_active_archive: LibraryControl::gated(
            "library.transfer.export-active-archive",
            "Export this galaxy",
            "Prepare a portable copy of the open Workshop.",
            false,
            LibraryUiIntent::ExportActiveArchive,
            &[inactive, transfer],
        ),
        export_active_pack: LibraryControl::gated(
            "library.transfer.export-active-pack",
            "Export content pack",
            "Prepare a portable copy of the open Workshop's content pack.",
            false,
            LibraryUiIntent::ExportActivePack,
            &[inactive, transfer],
        ),
    }
}

fn semantic_control(control: &LibraryControl, role: SemanticRole) -> SemanticNode {
    let mut node = SemanticNode::control(
        format!("library.control.{}", control.action_id.as_str()),
        role,
        &control.label,
        &control.description,
        control.enabled,
        control.selected,
        control.action_id.clone(),
    );
    if let Some(reason) = control.disabled_reason {
        node.value = Some(reason.message().to_owned());
    }
    node
}

const fn content_message(content: LibraryContent) -> &'static str {
    match content {
        LibraryContent::Loading => "Listing saved galaxies.",
        LibraryContent::NotListed => "Saved galaxies have not been listed. Refresh to list them.",
        LibraryContent::Empty => "No saved galaxies yet.",
        LibraryContent::Slots => "Saved galaxies listed.",
    }
}

const fn request_message(status: LibrarySlotsStatus) -> &'static str {
    match status {
        LibrarySlotsStatus::Idle => "No Library request is running.",
        LibrarySlotsStatus::Working { kind, .. } => match kind {
            SlotRequestKind::List => "Listing saved galaxies.",
            SlotRequestKind::Rename => "Renaming a saved galaxy.",
            SlotRequestKind::Archive => "Archiving a saved galaxy.",
            SlotRequestKind::Unarchive => "Unarchiving a saved galaxy.",
        },
        LibrarySlotsStatus::Failed { kind, .. } => match kind {
            SlotRequestKind::List => "Listing saved galaxies failed. Retry or cancel.",
            SlotRequestKind::Rename => "Renaming a saved galaxy failed. Retry or cancel.",
            SlotRequestKind::Archive => "Archiving a saved galaxy failed. Retry or cancel.",
            SlotRequestKind::Unarchive => "Unarchiving a saved galaxy failed. Retry or cancel.",
        },
    }
}

fn build_semantic_tree(
    close: &LibraryControl,
    refresh: &LibraryControl,
    request: &LibraryRequestModel,
    content: LibraryContent,
    rows: &[LibraryRow],
    actions: &LibraryActionsModel,
    transfer: &LibraryTransferModel,
) -> SemanticTree {
    let header = SemanticNode::container(
        "library.header",
        SemanticRole::Toolbar,
        "Library controls",
        vec![
            semantic_control(close, SemanticRole::Button),
            semantic_control(refresh, SemanticRole::Button),
        ],
    );

    let mut request_children = vec![SemanticNode::text(
        "library.request.status",
        if request.decision_pending() {
            SemanticRole::Alert
        } else {
            SemanticRole::Status
        },
        "Library request",
        request_message(request.status),
    )];
    if let Some(control) = &request.retry_control {
        request_children.push(semantic_control(control, SemanticRole::Button));
    }
    if let Some(control) = &request.cancel_control {
        request_children.push(semantic_control(control, SemanticRole::Button));
    }
    let request_region = SemanticNode::container(
        "library.request",
        SemanticRole::Region,
        "Library request state",
        request_children,
    );

    let mut list_children = vec![SemanticNode::text(
        "library.content",
        SemanticRole::Status,
        "Saved galaxies",
        content_message(content),
    )];
    for section in [LibrarySection::Active, LibrarySection::Archived] {
        let section_rows: Vec<&LibraryRow> =
            rows.iter().filter(|row| row.section == section).collect();
        if section_rows.is_empty() {
            continue;
        }
        let (id, name) = match section {
            LibrarySection::Active => ("library.section.active", "Saved galaxies"),
            LibrarySection::Archived => ("library.section.archived", "Archived galaxies"),
        };
        list_children.push(SemanticNode::container(
            id,
            SemanticRole::List,
            name,
            section_rows
                .into_iter()
                .map(|row| semantic_control(&row.control, SemanticRole::Option))
                .collect(),
        ));
    }
    let list_region = SemanticNode::container(
        "library.list",
        SemanticRole::Region,
        "Saved galaxy list",
        list_children,
    );

    let actions_region = SemanticNode::container(
        "library.actions",
        SemanticRole::Group,
        "Actions for the selected save",
        actions
            .controls()
            .into_iter()
            .map(|control| semantic_control(control, SemanticRole::Button))
            .collect(),
    );

    let transfer_region = SemanticNode::container(
        "library.transfer",
        SemanticRole::Group,
        "Portable transfer",
        transfer
            .controls()
            .into_iter()
            .map(|control| semantic_control(control, SemanticRole::Button))
            .collect(),
    );

    let mut announcements = vec![SemanticAnnouncement {
        kind: AnnouncementKind::Status,
        code: "library-content",
        message: content_message(content).to_owned(),
    }];
    if request.decision_pending() {
        announcements.push(SemanticAnnouncement {
            kind: AnnouncementKind::Error,
            code: "library-request-failed",
            message: request_message(request.status).to_owned(),
        });
    }

    SemanticTree {
        root: SemanticNode::container(
            "library.application",
            SemanticRole::Application,
            "NYON Workshop Library",
            vec![
                header,
                request_region,
                list_region,
                actions_region,
                transfer_region,
            ],
        ),
        announcements,
    }
}
