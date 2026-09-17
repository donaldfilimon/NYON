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
//! Controls whose backing capability has not shipped yet — the transfer
//! strip, and the export handoff while no transfer adapter is installed —
//! render **disabled with a visible reason**, keeping their focus slot. Omitting them is baseline Finding 5
//! exactly: a control that exists logically and cannot be reached.
//! [`LibraryControl`] makes that structural, because `enabled` and
//! `disabled_reason` can only be set together.

use std::collections::BTreeMap;

mod confirmation;

pub use confirmation::{
    LIBRARY_CONFIRM_CANCEL_ACTION, LIBRARY_CONFIRM_DIALOG, LIBRARY_CONFIRM_SUBMIT_ACTION,
    LIBRARY_RENAME_FIELD_ACTION, LibraryConfirmationKind, LibraryConfirmationModel,
    LibraryConfirmationRequest, library_confirmation_order, rename_draft_acceptable,
};

use crate::{
    app::client_runtime::{
        ClientDiagnosticCode, ExportSource, LibrarySlotsStatus, SlotRequestKind,
    },
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
    /// Close the open confirmation without changing anything.
    CancelConfirmation,
    /// Submit exactly the change the open confirmation describes.
    SubmitConfirmation(LibraryConfirmationRequest),
    /// Install the validated open the runtime is holding.
    AcceptOpen,
    /// Accept §4's export-recovery choice: prepare the validated predecessor.
    AcceptExportRecovery,
    /// Stage 2 of row Export: hand the prepared bytes to the platform. A
    /// separate intent from [`Self::ExportSlot`], on a separate control, so a
    /// repeated stage-1 activation can never perform the handoff (§7).
    HandOffExport,
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
    ///
    /// **Correct only because of a store invariant this model does not
    /// enforce** (task 7 review F12). Addendum §4 requires `CommitSlot` and
    /// `PromoteRecoveredSlot` to clear `selected_continue` atomically when they
    /// name the marked slot, so a set marker always names the current head and
    /// re-selecting it would be a no-op compare-and-swap. If that atomic clear
    /// were ever relaxed, this would strand a stale marker the user cannot
    /// re-select past, and no test here would fail.
    ///
    /// Read from `SlotSummary::selected_for_continue`, not
    /// `SlotList::selected_continue`: the two represent one fact, and the test
    /// fixtures derive the second from the first, so no fixture can make them
    /// disagree. Anything that starts reading `selected_continue` should not
    /// assume they are checked against each other.
    AlreadyContinue,
    /// The single slot-request lane is occupied by a request in flight.
    RequestInFlight,
    /// The lane holds a failure the user has not resolved yet.
    DecisionPending,
    /// The lane holds a prepared export the user has not handed off or
    /// discarded.
    ExportWaiting,
    /// The lane holds a finished handoff the user has not dismissed.
    CopyFinished,
    /// The rename draft is not a name the store accepts.
    InvalidName,
    /// No portable transfer adapter is installed, or (for the transfer strip)
    /// the control's route has not shipped.
    TransferUnavailable,
    /// There is no active Workshop session to export.
    WorkshopInactive,
    /// The resident Workshop has unsaved work or a persistence obligation,
    /// so nothing may replace it. §6.
    ReplacementBlocked,
    /// The row is the save the resident Workshop already has open.
    AlreadyOpen,
    /// A Workshop is resident, and it decides the Continue save while it is.
    WorkshopResident,
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
            Self::ExportWaiting => "Discard the prepared copy first.",
            Self::CopyFinished => "Dismiss the finished copy first.",
            Self::InvalidName => {
                "Names use 1 to 64 printable characters with single spaces between words."
            }
            Self::TransferUnavailable => "Portable file transfer is not available yet.",
            Self::WorkshopInactive => "Open a Workshop before exporting it.",
            Self::ReplacementBlocked => "Save the open Workshop before opening another save.",
            Self::AlreadyOpen => "This save is already open in the Workshop.",
            Self::WorkshopResident => "Continue follows the open Workshop while it is open.",
        }
    }
}

/// One activatable Library control.
///
/// `enabled` and `disabled_reason` are never set independently: an enabled
/// control has no reason and a disabled one always has exactly one. Both
/// constructors enforce it, so no caller can produce a disabled control with
/// nothing to display.
#[derive(Clone, Eq, PartialEq)]
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

/// Hand-written so a disabled control's placeholder subject never reaches a log.
///
/// The derived form printed `intent` unconditionally, which put
/// `OpenSlot { slot: SlotId(0), .. }` into any `{:?}` of a disabled action — and
/// slot 0 is a real save, because the native store mints identifiers from zero
/// upward. The typed path was already safe; this closes the textual one.
impl std::fmt::Debug for LibraryControl {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut debug = formatter.debug_struct("LibraryControl");
        debug
            .field("action_id", &self.action_id)
            .field("label", &self.label)
            .field("description", &self.description)
            .field("enabled", &self.enabled)
            .field("selected", &self.selected)
            .field("disabled_reason", &self.disabled_reason);
        match self.intent() {
            Some(intent) => debug.field("intent", intent),
            None => debug.field("intent", &"<disabled>"),
        };
        debug.finish()
    }
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
        matches!(
            self.status,
            LibrarySlotsStatus::Failed { .. }
                | LibrarySlotsStatus::Held { .. }
                | LibrarySlotsStatus::ExportRecoveryOffered { .. }
                | LibrarySlotsStatus::HandOffFailed { .. }
        )
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
    pub(crate) fn controls(&self) -> [&LibraryControl; 5] {
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
/// Present with no route behind it: every control keeps its label, its focus
/// slot and its disabled reason. Their stage-1 sources and the Choose Import
/// consumers have not shipped (route-design tasks 11 and 12), and §8 gates the
/// real adapters on a compatibility spike, which is a reason to disable these,
/// never to hide them.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LibraryTransferModel {
    pub import_archive: LibraryControl,
    pub import_pack: LibraryControl,
    pub export_active_archive: LibraryControl,
    pub export_active_pack: LibraryControl,
}

impl LibraryTransferModel {
    pub(crate) fn controls(&self) -> [&LibraryControl; 4] {
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
    /// Whether the resident Workshop cannot be replaced right now. Read from
    /// `ClientRuntime::resident_workshop_blocks_replacement`, the same rule the
    /// runtime refuses an open with.
    pub replacement_blocked: bool,
    /// Whether the transfer strip's routes are wired. Always `false` in the
    /// product today: an installed adapter alone does not give Import galaxy,
    /// Import content pack or the two active exports anything to run.
    pub transfer_available: bool,
    /// Whether a portable transfer adapter is installed, which is all the row
    /// Export handoff needs. Read from `ClientRuntime::transfer_available`.
    pub handoff_available: bool,
    /// The confirmation the user opened, if any. Resolved against the rows:
    /// one naming a save the list no longer holds builds no dialog.
    pub confirmation: Option<LibraryConfirmationRequest>,
    /// The rename draft, when the open confirmation is a rename.
    pub rename_draft: Option<&'a str>,
}

impl Default for LibraryUiContext<'_> {
    fn default() -> Self {
        Self {
            slots: None,
            status: LibrarySlotsStatus::Idle,
            selected_slot: None,
            resident_slot: None,
            workshop_active: false,
            replacement_blocked: false,
            transfer_available: false,
            handoff_available: false,
            confirmation: None,
            rename_draft: None,
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
    /// The open confirmation dialog. Its controls are in [`Self::controls`]
    /// and last in [`Self::focus_order`]; the frame traps focus on them.
    pub confirmation: Option<LibraryConfirmationModel>,
    pub semantics: SemanticTree,
    controls: BTreeMap<SemanticActionId, LibraryControl>,
    focus_order: Vec<SemanticActionId>,
}

impl LibraryUiModel {
    pub fn build(context: LibraryUiContext<'_>) -> Self {
        let request = build_request(
            context.status,
            context.replacement_blocked,
            context.handoff_available,
        );
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
        let confirmation = context.confirmation.and_then(|request_to_confirm| {
            confirmation::build_confirmation(
                request_to_confirm,
                &rows,
                lane_reason(&request),
                context.rename_draft,
            )
        });

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
        if let Some(confirmation) = &confirmation {
            ordered.extend(confirmation.controls());
        }
        for control in ordered {
            let previous = controls.insert(control.action_id.clone(), control.clone());
            // A collision is a bug, never input: the ids are hand-named
            // constants plus `library.slot.{SlotId}`, so two controls sharing
            // one means two constants were given the same name or an adapter
            // minted a duplicate slot id. Silently absorbing it cost one control
            // its focus slot while `activate` resolved to the other, and no
            // count-based test could see it (task 7 review F10). Loud in debug;
            // release keeps the old drop rather than pushing a slot for a
            // control the map no longer holds.
            debug_assert!(
                previous.is_none(),
                "duplicate Library action id: {}",
                control.action_id.as_str()
            );
            if previous.is_none() {
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
        // A direct child of the root: `modal_dialog` and the frame both look
        // for the dialog there.
        let mut semantics = semantics;
        semantics.root.children.extend(
            confirmation.as_ref().map(|confirmation| {
                confirmation::confirmation_node(confirmation, semantic_control)
            }),
        );

        Self {
            close_control,
            refresh_control,
            request,
            content,
            rows,
            actions,
            transfer,
            confirmation,
            semantics,
            controls,
            focus_order,
        }
    }

    /// Every control, in **lexicographic action-identifier order** — which is
    /// not the traversal order and not the layout order.
    ///
    /// `library.slot.10` precedes `library.slot.2` here, and every action
    /// precedes every row. Use [`Self::focus_order`] for traversal and for
    /// anything a user perceives as a sequence; this iterator is for lookups
    /// and for whole-model assertions.
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
        LibrarySlotsStatus::Failed { .. }
        | LibrarySlotsStatus::Held { .. }
        | LibrarySlotsStatus::ExportRecoveryOffered { .. } => {
            Some(LibraryDisabledReason::DecisionPending)
        }
        LibrarySlotsStatus::ExportReady { .. } | LibrarySlotsStatus::HandOffFailed { .. } => {
            Some(LibraryDisabledReason::ExportWaiting)
        }
        LibrarySlotsStatus::ExportHandedOff { .. } => Some(LibraryDisabledReason::CopyFinished),
    }
}

/// Retry and Cancel over the slot-request lane.
///
/// A `Failed` state always produces both, including the one
/// `ClientRuntime::open_library` deliberately preserves across a visit: it did
/// not re-list, because re-listing would resolve a decision the user still
/// owns. Neither control ever routes to the recovery screen — §6 forbids
/// collapsing a Library failure into `RecoverableError`.
fn build_request(
    status: LibrarySlotsStatus,
    replacement_blocked: bool,
    handoff_available: bool,
) -> LibraryRequestModel {
    let replacement = replacement_blocked.then_some(LibraryDisabledReason::ReplacementBlocked);
    let adapter = (!handoff_available).then_some(LibraryDisabledReason::TransferUnavailable);
    let (failure_code, retry_control, cancel_control) = match status {
        LibrarySlotsStatus::Idle => (None, None, None),
        // Stopping the wait keeps the prepared copy, so the label must not
        // read like Discard.
        LibrarySlotsStatus::Working {
            kind: SlotRequestKind::HandOff,
            ..
        } => (
            None,
            None,
            Some(LibraryControl::enabled(
                "library.request.cancel",
                "Stop waiting",
                "Stop waiting for the system. The prepared copy is kept.",
                false,
                LibraryUiIntent::CancelSlotRequest,
            )),
        ),
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
        // A conflicted open is retried by re-listing; the runtime does exactly
        // that for this code, so the label says so.
        LibrarySlotsStatus::Failed {
            kind: SlotRequestKind::Open | SlotRequestKind::UseForContinue | SlotRequestKind::Export,
            code: code @ ClientDiagnosticCode::StaleSave,
            ..
        } => (
            Some(code),
            Some(LibraryControl::enabled(
                "library.request.retry",
                "Refresh",
                "List the saved galaxies again, then try the save once more.",
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
        // Same identifiers as Retry and Cancel, so focus held on the request
        // strip survives the open moving from in flight to held. Acceptance
        // installs, so it takes the replacement gate the runtime refuses it
        // with rather than offering a control certain to be refused.
        LibrarySlotsStatus::Held { recovered, .. } => (
            None,
            Some(LibraryControl::gated(
                "library.request.retry",
                if recovered { "Open previous" } else { "Open" },
                if recovered {
                    "Open the previous valid generation of this save. The invalid latest one is replaced when it saves."
                } else {
                    "Open the validated save in the Workshop."
                },
                false,
                LibraryUiIntent::AcceptOpen,
                &[replacement],
            )),
            Some(LibraryControl::enabled(
                "library.request.cancel",
                "Cancel",
                "Do not open this save. Nothing stored is removed.",
                false,
                LibraryUiIntent::CancelSlotRequest,
            )),
        ),
        // §4's export-recovery choice. Accepting prepares bytes and replaces
        // nothing, so unlike Open previous it takes no replacement gate.
        LibrarySlotsStatus::ExportRecoveryOffered { .. } => (
            None,
            Some(LibraryControl::enabled(
                "library.request.retry",
                "Export previous",
                "Prepare a portable copy of the previous valid generation. The stored save and Continue are not changed.",
                false,
                LibraryUiIntent::AcceptExportRecovery,
            )),
            Some(LibraryControl::enabled(
                "library.request.cancel",
                "Cancel",
                "Do not export this save. Nothing stored is changed.",
                false,
                LibraryUiIntent::CancelSlotRequest,
            )),
        ),
        // Stage 2 has its own identifier, never the one Export previous used,
        // so a repeated activation of the control that prepared the bytes
        // cannot land on the handoff (§7's second direct activation). It is
        // disabled with a visible reason while no transfer adapter is
        // installed, and keeps its focus slot meanwhile. A failed handoff is
        // retried on this same control, never on the generic Retry.
        LibrarySlotsStatus::ExportReady { .. } | LibrarySlotsStatus::HandOffFailed { .. } => (
            matches!(status, LibrarySlotsStatus::HandOffFailed { .. })
                .then_some(ClientDiagnosticCode::Transfer),
            Some(LibraryControl::gated(
                "library.request.handoff",
                "Save copy",
                "Hand the prepared copy to the system.",
                false,
                LibraryUiIntent::HandOffExport,
                &[adapter],
            )),
            Some(LibraryControl::enabled(
                "library.request.cancel",
                "Discard",
                "Discard the prepared copy. Nothing stored is changed.",
                false,
                LibraryUiIntent::CancelSlotRequest,
            )),
        ),
        // The copy has left; nothing here may send it again.
        LibrarySlotsStatus::ExportHandedOff { .. } => (
            None,
            None,
            Some(LibraryControl::enabled(
                "library.request.cancel",
                "Done",
                "Dismiss this notice. The copy was already handed off.",
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
/// Open (task 12a), Use for Continue (12b) and row Export (12c) take the lane
/// reason. Open and Use for Continue need it because their `SelectContinue`
/// contends for the Commit lane a Rename holds; all three need it because
/// their progress, failure, held candidate or prepared bytes are shown in the
/// one request strip.
///
/// **§6's replacement-and-persistence gate, item by item, so task 12 adds it
/// where it belongs rather than where an earlier summary of this list said.**
/// §6 names exactly four things that "remain disabled until replacement and
/// persistence invariants are safe":
///
/// 1. **Library Open** — this function, gated since task 12a on
///    [`LibraryDisabledReason::ReplacementBlocked`]. The runtime refuses the
///    same case in `dispatch_open`, and re-checks it when a held candidate is
///    accepted.
/// 2. **Workshop Archive import** — *not here*. That is `ImportArchive` in
///    [`build_transfer`], which carries a matching note. `ImportPack` is exempt:
///    §6 explicitly permits content-pack storage that requests no session
///    replacement.
/// 3. **Use for Continue** — this function, gated since task 12b on
///    [`LibraryDisabledReason::WorkshopResident`], which is stricter than §6:
///    any resident Workshop, not only one that cannot be replaced. The
///    runtime's `use_library_slot_for_continue` states why.
/// 4. **Recovery of another slot** — reached through Open since task 12a. An
///    open whose head is invalid is held as
///    [`LibrarySlotsStatus::Held`] with `recovered: true`, and its acceptance
///    is refused by the same replacement gate. There is no separate repair
///    control.
///
/// **Row Export is deliberately absent from that list.** §6 does not name it and
/// §4 states it "does not mutate storage, select Continue, or install/replace a
/// session", so there is no replacement gate to add to it, no archived gate
/// (task 7 review F5) and no residency gate: exporting the resident's own row
/// reads its stored generation. The lane is its only gate.
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
    let already_open = row
        .filter(|row| row.resident)
        .map(|_| LibraryDisabledReason::AlreadyOpen);
    let replacement = context
        .replacement_blocked
        .then_some(LibraryDisabledReason::ReplacementBlocked);
    let workshop = context
        .workshop_active
        .then_some(LibraryDisabledReason::WorkshopResident);
    let already_continue = row
        .filter(|row| row.selected_for_continue)
        .map(|_| LibraryDisabledReason::AlreadyContinue);
    let lane = lane_reason(request);
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
            &[no_selection, archived, already_open, replacement, lane],
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
            &[no_selection, archived, already_continue, workshop, lane],
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
            &[no_selection, lane],
        ),
    }
}

/// The four transfer controls, live with no adapter installed.
///
/// **`ImportArchive` needs §6's replacement-and-persistence gate and does not
/// have it.** It is §6's "Workshop Archive import", gated today only on
/// [`LibraryDisabledReason::TransferUnavailable`], which is a capability
/// deferral and not that invariant. `ImportPack` needs no such gate: §6
/// explicitly permits content-pack storage that requests no session
/// replacement. See [`build_actions`] for the whole four-item disposition.
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

/// The content line, whose only conditional arm is [`LibraryContent::NotListed`].
///
/// `NotListed` is one state with three causes, and they do not share a remedy.
/// Refresh takes the one slot-request lane, so it is disabled whenever that lane
/// is busy or holds a failure — and the unconditional copy told the user to
/// press it anyway. That is reachable on the **first visit**: `open_library`
/// dispatches `ListSlots` from `Idle`, and a store that refuses leaves
/// `Failed { List }` with no cached list at all.
///
/// The failed arm deliberately does not restate [`request_message`] verbatim.
/// The content line says what is on screen; the request line owns the decision,
/// and they are emitted as two separate announcements, so two identical
/// sentences would be its own defect.
const fn content_message(
    content: LibraryContent,
    lane: Option<LibraryDisabledReason>,
) -> &'static str {
    match content {
        LibraryContent::Loading => "Listing saved galaxies.",
        LibraryContent::NotListed => match lane {
            None => "Saved galaxies have not been listed. Refresh to list them.",
            Some(LibraryDisabledReason::DecisionPending) => {
                "Saved galaxies have not been listed. Resolve the failed request first."
            }
            Some(_) => "Saved galaxies have not been listed. Waiting for the current request.",
        },
        LibraryContent::Empty => "No saved galaxies yet.",
        LibraryContent::Slots => "Saved galaxies listed.",
    }
}

fn request_message(status: LibrarySlotsStatus) -> String {
    let line = match status {
        LibrarySlotsStatus::Idle => "No Library request is running.",
        LibrarySlotsStatus::Working { kind, .. } => match kind {
            SlotRequestKind::List => "Listing saved galaxies.",
            SlotRequestKind::Rename => "Renaming a saved galaxy.",
            SlotRequestKind::Archive => "Archiving a saved galaxy.",
            SlotRequestKind::Unarchive => "Unarchiving a saved galaxy.",
            SlotRequestKind::Open => "Opening a saved galaxy.",
            SlotRequestKind::UseForContinue => "Checking a saved galaxy for Continue.",
            SlotRequestKind::Export => "Preparing a portable copy of a saved galaxy.",
            SlotRequestKind::HandOff => "Handing the portable copy to the system.",
        },
        LibrarySlotsStatus::Failed { kind, code, .. } => match kind {
            SlotRequestKind::List => "Listing saved galaxies failed. Retry or cancel.",
            SlotRequestKind::Rename => "Renaming a saved galaxy failed. Retry or cancel.",
            SlotRequestKind::Archive => "Archiving a saved galaxy failed. Retry or cancel.",
            SlotRequestKind::Unarchive => "Unarchiving a saved galaxy failed. Retry or cancel.",
            SlotRequestKind::Open | SlotRequestKind::UseForContinue | SlotRequestKind::Export
                if matches!(code, ClientDiagnosticCode::StaleSave) =>
            {
                "That save changed. Refresh, then try again."
            }
            SlotRequestKind::Open => "Opening a saved galaxy failed. Retry or cancel.",
            SlotRequestKind::UseForContinue => {
                "Choosing the Continue save failed. Retry or cancel."
            }
            SlotRequestKind::Export => "Preparing a portable copy failed. Retry or cancel.",
            // Never produced: a failed handoff is `HandOffFailed`.
            SlotRequestKind::HandOff => "Handing off the portable copy failed.",
        },
        LibrarySlotsStatus::Held {
            recovered: true, ..
        } => "The latest save is invalid. Open its previous generation or cancel.",
        LibrarySlotsStatus::Held {
            recovered: false, ..
        } => "A saved galaxy is now the Continue save. Open it or cancel.",
        LibrarySlotsStatus::ExportRecoveryOffered { .. } => {
            "The latest save is invalid. Export its previous generation or cancel."
        }
        LibrarySlotsStatus::ExportReady { source } => match source {
            ExportSource::Head { .. } => "A portable copy of the latest save is ready.",
            ExportSource::RecoveredPredecessor { .. } => {
                "Portable copy ready. Recovered predecessor; stored head and Continue unchanged."
            }
            ExportSource::ActiveWorkshop {
                continue_ready: false,
            } => {
                "Portable copy of the open Workshop ready. Portable export; Workshop not saved for Continue."
            }
            ExportSource::ActiveWorkshop {
                continue_ready: true,
            } => "A portable copy of the open Workshop, as saved for Continue, is ready.",
            ExportSource::ActivePack { .. } => {
                "A portable copy of the open Workshop's content pack is ready."
            }
        },
        LibrarySlotsStatus::HandOffFailed { source, .. } => {
            return with_source(
                "Handing off the portable copy failed. Save copy again or discard it.",
                source,
            );
        }
        // Keyed to what the adapter reported, never to which adapter ran
        // (addendum §1, §8, §9): only a durable native write says "saved".
        LibrarySlotsStatus::ExportHandedOff {
            source, outcome, ..
        } => return with_source(outcome.label(), source),
    };
    line.to_owned()
}

/// A qualifying label travels with the bytes to the end, so a copy of a
/// recovered generation is never reported as the stored head, and a copy of
/// unsaved open-Workshop state never loses §10's "not saved for Continue".
fn with_source(line: &str, source: ExportSource) -> String {
    match source.qualifier() {
        None => line.to_owned(),
        Some(qualifier) => format!("{line} {qualifier}."),
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
        content_message(content, lane_reason(request)),
    )];
    for section in [LibrarySection::Active, LibrarySection::Archived] {
        let section_rows: Vec<&LibraryRow> =
            rows.iter().filter(|row| row.section == section).collect();
        if section_rows.is_empty() {
            continue;
        }
        let (id, name, heading) = match section {
            LibrarySection::Active => ("library.section.active", "Saved galaxies", "Active"),
            LibrarySection::Archived => {
                ("library.section.archived", "Archived galaxies", "Archived")
            }
        };
        // The container's name is only announced; this heading is what a
        // sighted user reads to tell an archived row from an active one.
        let count = match section_rows.len() {
            1 => "1 save".to_owned(),
            count => format!("{count} saves"),
        };
        let mut children = vec![SemanticNode::text(
            format!("{id}.heading"),
            SemanticRole::Heading,
            heading,
            count,
        )];
        children.extend(
            section_rows
                .into_iter()
                .map(|row| semantic_control(&row.control, SemanticRole::Option)),
        );
        list_children.push(SemanticNode::container(
            id,
            SemanticRole::List,
            name,
            children,
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
        message: content_message(content, lane_reason(request)).to_owned(),
    }];
    if request.decision_pending() {
        announcements.push(SemanticAnnouncement {
            kind: AnnouncementKind::Error,
            code: "library-request-failed",
            message: request_message(request.status),
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
