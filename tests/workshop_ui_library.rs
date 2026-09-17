//! The Workshop Library UI model: content states, gating, intents, focus order.
//!
//! The three contracts these suites exist for all fail toward a screen that
//! looks fine:
//!
//! 1. a dropped list with a listing request in flight is **loading**, and an
//!    empty store is **empty** — confusing them flashes a false "no saves yet"
//!    on every successful rename;
//! 2. a `Failed` slot request preserved across a Library visit is a **pending
//!    decision**, not something to list away;
//! 3. a store failure yields Retry and Cancel and never routes to the global
//!    recovery screen.
//!
//! Every assertion below that names a property is paired with the arm that
//! would have to break for it to fail.

use nyon::{
    app::{
        client_runtime::{ClientDiagnosticCode, ExportSource, LibrarySlotsStatus, SlotRequestKind},
        transfer::{HandoffOutcome, TransferFailureCode},
    },
    ui::{
        accessibility::{InputModality, SemanticActionId, SemanticRole},
        library::{
            LibraryConfirmationKind, LibraryConfirmationRequest, LibraryContent, LibraryControl,
            LibraryDisabledReason, LibrarySection, LibraryUiContext, LibraryUiIntent,
            LibraryUiModel, library_confirmation_order, rename_draft_acceptable,
        },
    },
    workshop::store::{SaveGeneration, SlotId, SlotList, SlotName, SlotSummary},
};

const HEAD: ExportSource = ExportSource::Head {
    slot: SlotId(1),
    generation: SaveGeneration(11),
};
const PREDECESSOR: ExportSource = ExportSource::RecoveredPredecessor {
    slot: SlotId(1),
    generation: SaveGeneration(11),
};

fn summary(id: u64, name: &str, archived: bool) -> SlotSummary {
    SlotSummary {
        id: SlotId(id),
        name: SlotName::new(name).unwrap(),
        generation: SaveGeneration(id + 10),
        archived,
        has_previous_generation: false,
        selected_for_continue: false,
    }
}

fn list(slots: Vec<SlotSummary>) -> SlotList {
    let selected_continue = slots
        .iter()
        .find(|slot| slot.selected_for_continue)
        .map(|slot| slot.id);
    SlotList {
        slots,
        selected_continue,
    }
}

fn model(context: LibraryUiContext<'_>) -> LibraryUiModel {
    LibraryUiModel::build(context)
}

fn control<'a>(model: &'a LibraryUiModel, action: &str) -> &'a LibraryControl {
    model
        .controls()
        .find(|control| control.action_id.as_str() == action)
        .unwrap_or_else(|| panic!("no control {action}"))
}

// ---------------------------------------------------------------------------
// Contract 1: loading, not-listed, empty and populated are four distinct states
// ---------------------------------------------------------------------------

/// A dropped list with a listing request in flight is loading.
///
/// This is the frame every successful mutation passes through: the runtime
/// clears the cached list and immediately re-lists. Reporting `Empty` here is
/// the false-empty flash.
#[test]
fn a_dropped_list_with_a_listing_request_in_flight_is_loading_not_empty() {
    let built = model(LibraryUiContext {
        slots: None,
        status: LibrarySlotsStatus::Working {
            kind: SlotRequestKind::List,
            slot: None,
        },
        ..LibraryUiContext::default()
    });
    assert_eq!(built.content, LibraryContent::Loading);
    assert_ne!(built.content, LibraryContent::Empty);
    assert!(built.rows.is_empty());
}

/// A listed store with no slots is empty, and empty is not loading.
///
/// `Some` with an empty `slots` is the runtime's *only* representation of
/// emptiness, so this arm and the one above must not collapse into each other.
#[test]
fn a_listed_store_with_no_slots_is_empty_not_loading() {
    let empty = list(Vec::new());
    let built = model(LibraryUiContext {
        slots: Some(&empty),
        ..LibraryUiContext::default()
    });
    assert_eq!(built.content, LibraryContent::Empty);
    assert_ne!(built.content, LibraryContent::Loading);
}

/// No list and nothing fetching one is a resting state with a way out.
///
/// Reached after a cancelled mutation. It must not read as loading, because
/// nothing is coming, and it must not read as empty, because nothing was
/// listed.
#[test]
fn no_list_and_no_request_is_not_listed_with_refresh_live() {
    let built = model(LibraryUiContext::default());
    assert_eq!(built.content, LibraryContent::NotListed);
    assert!(control(&built, "library.refresh").enabled);
}

/// A cached list survives a refresh instead of flashing through loading.
///
/// The runtime keeps the cached list across a `ListSlots`; only a *mutation*
/// drops it. If content were derived from the status alone, this would flash.
#[test]
fn a_cached_list_survives_a_refresh_without_flashing_loading() {
    let listed = list(vec![summary(1, "Andromeda", false)]);
    let built = model(LibraryUiContext {
        slots: Some(&listed),
        status: LibrarySlotsStatus::Working {
            kind: SlotRequestKind::List,
            slot: None,
        },
        ..LibraryUiContext::default()
    });
    assert_eq!(built.content, LibraryContent::Slots);
    assert_eq!(built.rows.len(), 1);
}

/// A dropped list with a *mutation* in flight is not loading either.
///
/// Only a listing request produces rows. Treating every `Working` as loading
/// would promise rows that nothing is fetching.
#[test]
fn a_dropped_list_with_a_mutation_in_flight_is_not_listed() {
    let built = model(LibraryUiContext {
        slots: None,
        status: LibrarySlotsStatus::Working {
            kind: SlotRequestKind::Rename,
            slot: Some(SlotId(1)),
        },
        ..LibraryUiContext::default()
    });
    assert_eq!(built.content, LibraryContent::NotListed);
    assert_ne!(built.content, LibraryContent::Loading);
}

// ---------------------------------------------------------------------------
// Contract 2: a preserved failure is a pending decision
// ---------------------------------------------------------------------------

/// A `Failed` request renders Retry and Cancel even with rows on screen.
///
/// `open_library` auto-lists only from `Idle`, so a failure from a previous
/// visit survives re-entry with its cached list intact. If the decision
/// controls were gated on there being nothing to show, this is the case that
/// would strand the user.
#[test]
fn a_preserved_failure_renders_retry_and_cancel_over_cached_rows() {
    let listed = list(vec![summary(1, "Andromeda", false)]);
    let built = model(LibraryUiContext {
        slots: Some(&listed),
        status: LibrarySlotsStatus::Failed {
            kind: SlotRequestKind::Rename,
            slot: Some(SlotId(1)),
            code: ClientDiagnosticCode::Store,
        },
        ..LibraryUiContext::default()
    });
    assert_eq!(built.content, LibraryContent::Slots);
    assert!(built.request.decision_pending());
    let retry = built.request.retry_control.as_ref().expect("retry control");
    let cancel = built
        .request
        .cancel_control
        .as_ref()
        .expect("cancel control");
    assert!(retry.enabled);
    assert!(cancel.enabled);
    assert_eq!(retry.intent(), Some(&LibraryUiIntent::RetrySlotRequest));
    assert_eq!(cancel.intent(), Some(&LibraryUiIntent::CancelSlotRequest));
    assert_eq!(
        built.request.failure_code,
        Some(ClientDiagnosticCode::Store)
    );
}

/// An in-flight request offers Cancel and no Retry.
///
/// Retrying something still running would occupy a lane that is already
/// occupied; cancelling releases the Commit lane a resident Workshop needs.
#[test]
fn an_in_flight_request_offers_cancel_but_not_retry() {
    let built = model(LibraryUiContext {
        status: LibrarySlotsStatus::Working {
            kind: SlotRequestKind::Archive,
            slot: Some(SlotId(2)),
        },
        ..LibraryUiContext::default()
    });
    assert!(built.request.in_flight());
    assert!(built.request.retry_control.is_none());
    assert!(built.request.cancel_control.is_some());
}

/// An idle lane offers neither.
#[test]
fn an_idle_lane_offers_neither_retry_nor_cancel() {
    let built = model(LibraryUiContext::default());
    assert!(built.request.retry_control.is_none());
    assert!(built.request.cancel_control.is_none());
    assert!(!built.request.decision_pending());
    assert!(!built.request.in_flight());
}

// ---------------------------------------------------------------------------
// Contract 3: failure never leaves this screen
// ---------------------------------------------------------------------------

/// Whether an intent takes the user off the Library screen.
///
/// Exhaustive on purpose, with **no wildcard arm**: adding a variant to
/// `LibraryUiIntent` must fail to compile here rather than default quietly to
/// "stays". §6 forbids collapsing a Library failure into `RecoverableError`, and
/// a future variant that did so would otherwise be invisible.
///
/// Open and its acceptance leave on success, which is what opening is for.
/// They count as leaving here, so the test below proves neither is offered
/// while a failure is pending: Open takes the lane reason, and acceptance
/// exists only for a held candidate.
const fn leaves_screen(intent: &LibraryUiIntent) -> bool {
    match intent {
        LibraryUiIntent::Close | LibraryUiIntent::OpenSlot { .. } | LibraryUiIntent::AcceptOpen => {
            true
        }
        LibraryUiIntent::RefreshSlots
        | LibraryUiIntent::RetrySlotRequest
        | LibraryUiIntent::CancelSlotRequest
        | LibraryUiIntent::SelectSlot(_)
        | LibraryUiIntent::RenameSlot { .. }
        | LibraryUiIntent::ArchiveSlot { .. }
        | LibraryUiIntent::UnarchiveSlot { .. }
        | LibraryUiIntent::UseForContinue { .. }
        | LibraryUiIntent::ExportSlot { .. }
        | LibraryUiIntent::ImportArchive
        | LibraryUiIntent::ImportPack
        | LibraryUiIntent::ExportActiveArchive
        | LibraryUiIntent::ExportActivePack
        | LibraryUiIntent::CancelConfirmation
        | LibraryUiIntent::SubmitConfirmation(_)
        | LibraryUiIntent::AcceptExportRecovery
        | LibraryUiIntent::HandOffExport => false,
    }
}

/// Exactly one control leaves the Library, and it is Close.
///
/// The earlier form of this test counted controls emitting `Close` and asserted
/// one — which proves Close exists once and says nothing about whether some
/// *other* control leaves. Pointing an existing control at a leaving intent,
/// keeping its identifier and its label, passed it. This form checks the
/// property the name claims: for every control, leaving iff it is Close.
#[test]
fn a_store_failure_offers_no_intent_that_leaves_the_screen() {
    let listed = list(vec![summary(1, "Andromeda", false)]);
    let built = model(LibraryUiContext {
        slots: Some(&listed),
        status: LibrarySlotsStatus::Failed {
            kind: SlotRequestKind::List,
            slot: None,
            code: ClientDiagnosticCode::StoreProtocol,
        },
        archive_import_available: true,
        handoff_available: true,
        workshop_active: true,
        selected_slot: Some(SlotId(1)),
        ..LibraryUiContext::default()
    });
    let mut leaving = Vec::new();
    for control in built.controls() {
        let Some(intent) = control.intent() else {
            continue;
        };
        let is_close = control.action_id.as_str() == "library.close";
        assert_eq!(
            leaves_screen(intent),
            is_close,
            "{} leaves the Library but is not Close",
            control.action_id
        );
        if leaves_screen(intent) {
            leaving.push(control.action_id.as_str().to_owned());
        }
    }
    assert_eq!(
        leaving,
        vec!["library.close".to_owned()],
        "Close is the only route off the Library"
    );
}

// ---------------------------------------------------------------------------
// Rows: identity, sectioning, role
// ---------------------------------------------------------------------------

/// Row identity is the `SlotId`, so a re-list that reorders keeps action ids.
///
/// If the action identifier were derived from the row index, reordering would
/// silently move every focus slot and every pointer target one row over.
#[test]
fn row_identity_survives_reordering_because_it_derives_from_the_slot_id() {
    let first = list(vec![
        summary(7, "Andromeda", false),
        summary(3, "Bode", false),
    ]);
    let second = list(vec![
        summary(3, "Bode", false),
        summary(7, "Andromeda", false),
    ]);
    let a = model(LibraryUiContext {
        slots: Some(&first),
        ..LibraryUiContext::default()
    });
    let b = model(LibraryUiContext {
        slots: Some(&second),
        ..LibraryUiContext::default()
    });
    let ids = |built: &LibraryUiModel| {
        built
            .rows
            .iter()
            .map(|row| (row.slot, row.control.action_id.as_str().to_owned()))
            .collect::<Vec<_>>()
    };
    let mut a_ids = ids(&a);
    let mut b_ids = ids(&b);
    a_ids.sort();
    b_ids.sort();
    assert_eq!(a_ids, b_ids);
    assert_eq!(a.rows[0].control.action_id.as_str(), "library.slot.7");
    assert_eq!(b.rows[0].control.action_id.as_str(), "library.slot.3");
}

/// Archived rows form a distinct section after the active ones.
#[test]
fn archived_rows_are_a_distinct_section_after_the_active_ones() {
    let listed = list(vec![
        summary(1, "Andromeda", true),
        summary(2, "Bode", false),
        summary(3, "Cigar", true),
        summary(4, "Draco", false),
    ]);
    let built = model(LibraryUiContext {
        slots: Some(&listed),
        ..LibraryUiContext::default()
    });
    let sections: Vec<LibrarySection> = built.rows.iter().map(|row| row.section).collect();
    assert_eq!(
        sections,
        vec![
            LibrarySection::Active,
            LibrarySection::Active,
            LibrarySection::Archived,
            LibrarySection::Archived,
        ]
    );
    let slots: Vec<u64> = built.rows.iter().map(|row| row.slot.0).collect();
    assert_eq!(
        slots,
        vec![2, 4, 1, 3],
        "store order holds within a section"
    );
}

/// Each non-empty section opens with a heading a sighted user can read.
///
/// Before this, the only thing telling an archived row from an active one was
/// the list container's accessible name, which nothing draws.
#[test]
fn each_nonempty_section_opens_with_a_counted_heading() {
    let listed = list(vec![
        summary(1, "Andromeda", true),
        summary(2, "Bode", false),
        summary(3, "Cigar", false),
    ]);
    let built = model(LibraryUiContext {
        slots: Some(&listed),
        ..LibraryUiContext::default()
    });
    for (section, heading, name, description, rows) in [
        (
            "library.section.active",
            "library.section.active.heading",
            "Active",
            "2 saves",
            2,
        ),
        (
            "library.section.archived",
            "library.section.archived.heading",
            "Archived",
            "1 save",
            1,
        ),
    ] {
        let container = built.semantics.node(section).expect(section);
        let first = &container.children[0];
        assert_eq!(first.id.as_str(), heading);
        assert_eq!(first.role, SemanticRole::Heading);
        assert_eq!(first.name, name);
        assert_eq!(first.description, description);
        assert!(first.action_id.is_none() && first.children.is_empty());
        assert_eq!(container.children.len(), rows + 1, "{section}");
    }

    // An empty section has neither a container nor a heading.
    let active_only = list(vec![summary(2, "Bode", false)]);
    let built = model(LibraryUiContext {
        slots: Some(&active_only),
        ..LibraryUiContext::default()
    });
    assert!(
        built
            .semantics
            .node("library.section.archived.heading")
            .is_none()
    );
    assert_eq!(
        built
            .semantics
            .node("library.section.active.heading")
            .unwrap()
            .description,
        "1 save"
    );
}

/// Every row control takes `SemanticRole::Option`.
///
/// That role is what routes a 64-byte user-authored slot name through the
/// variable-label path to single-line ellipsis. Under any other role the SDF
/// batch rejects the whole frame rather than truncating the label.
#[test]
fn every_row_node_takes_the_option_role_for_the_ellipsis_path() {
    let listed = list(vec![
        summary(1, &"N".repeat(64), false),
        summary(2, "Bode", true),
    ]);
    let built = model(LibraryUiContext {
        slots: Some(&listed),
        ..LibraryUiContext::default()
    });
    for row in &built.rows {
        let node = built
            .semantics
            .node(&format!(
                "library.control.{}",
                row.control.action_id.as_str()
            ))
            .expect("row node");
        assert_eq!(node.role, SemanticRole::Option, "row {}", row.slot.0);
    }
    assert_eq!(built.rows[0].name.len(), 64);
}

/// Non-row controls are buttons, so the ellipsis path is not applied to them.
#[test]
fn header_action_and_transfer_nodes_are_buttons() {
    let listed = list(vec![summary(1, "Andromeda", false)]);
    let built = model(LibraryUiContext {
        slots: Some(&listed),
        ..LibraryUiContext::default()
    });
    for id in [
        "library.close",
        "library.refresh",
        "library.action.open",
        "library.action.rename",
        "library.action.archive",
        "library.action.use-for-continue",
        "library.action.export",
        "library.transfer.import-archive",
        "library.transfer.import-pack",
        "library.transfer.export-active-archive",
        "library.transfer.export-active-pack",
    ] {
        let node = built
            .semantics
            .node(&format!("library.control.{id}"))
            .unwrap_or_else(|| panic!("no node for {id}"));
        assert_eq!(node.role, SemanticRole::Button, "{id}");
    }
}

// ---------------------------------------------------------------------------
// Gating: disabled with a reason, never omitted
// ---------------------------------------------------------------------------

/// `enabled` and `disabled_reason` never disagree, for every control.
#[test]
fn a_disabled_control_always_carries_exactly_one_reason() {
    for (transfer, workshop, selected) in [
        (false, false, None),
        (true, true, Some(SlotId(1))),
        (true, false, Some(SlotId(2))),
    ] {
        let listed = list(vec![
            summary(1, "Andromeda", false),
            summary(2, "Bode", true),
        ]);
        let built = model(LibraryUiContext {
            slots: Some(&listed),
            selected_slot: selected,
            archive_import_available: transfer,
            handoff_available: transfer,
            workshop_active: workshop,
            ..LibraryUiContext::default()
        });
        for control in built.controls() {
            assert_eq!(
                control.enabled,
                control.disabled_reason.is_none(),
                "{}",
                control.action_id
            );
        }
    }
}

/// Transfer controls are always present. With no adapter, which is every
/// product build before the §8 spike, the two imports are disabled with a
/// visible reason while the two exports prepare bytes, and row Export no
/// longer waits on any capability (route-design task 12c).
///
/// This is baseline Finding 5: a control that exists logically but cannot be
/// reached. Presence plus a reason is the contract; absence is the defect.
#[test]
fn without_an_adapter_the_imports_are_disabled_and_the_exports_are_live() {
    let listed = list(vec![summary(1, "Andromeda", false)]);
    let built = model(LibraryUiContext {
        slots: Some(&listed),
        selected_slot: Some(SlotId(1)),
        workshop_active: true,
        archive_import_available: true,
        ..LibraryUiContext::default()
    });
    for (id, intent) in [
        ("library.action.export", None),
        (
            "library.transfer.export-active-archive",
            Some(LibraryUiIntent::ExportActiveArchive),
        ),
        (
            "library.transfer.export-active-pack",
            Some(LibraryUiIntent::ExportActivePack),
        ),
    ] {
        let found = control(&built, id);
        assert!(found.enabled, "{id}");
        assert_eq!(found.disabled_reason, None, "{id}");
        if let Some(intent) = intent {
            assert_eq!(found.intent(), Some(&intent), "{id}");
        }
    }
    for id in [
        "library.transfer.import-archive",
        "library.transfer.import-pack",
    ] {
        let found = control(&built, id);
        assert!(!found.enabled, "{id}");
        assert_eq!(
            found.disabled_reason,
            Some(LibraryDisabledReason::TransferUnavailable),
            "{id}"
        );
    }

    // An adapter makes Import content pack live. Import galaxy also needs its
    // own route, which is a separate fact from the adapter.
    for (routed, archive_reason) in [
        (false, Some(LibraryDisabledReason::TransferUnavailable)),
        (true, None),
    ] {
        let built = model(LibraryUiContext {
            slots: Some(&listed),
            handoff_available: true,
            archive_import_available: routed,
            ..LibraryUiContext::default()
        });
        assert_eq!(
            control(&built, "library.transfer.import-pack").intent(),
            Some(&LibraryUiIntent::ImportPack)
        );
        assert_eq!(
            control(&built, "library.transfer.import-archive").disabled_reason,
            archive_reason,
            "routed: {routed}"
        );
    }
}

/// Every transfer control takes the one request lane: each shows its
/// progress, failure or prepared bytes on the request strip, so none may
/// start while that strip holds something else.
#[test]
fn every_transfer_control_waits_for_the_request_lane() {
    let listed = list(vec![summary(1, "Andromeda", false)]);
    for (status, reason) in [
        (
            LibrarySlotsStatus::Working {
                kind: SlotRequestKind::Rename,
                slot: Some(SlotId(1)),
            },
            LibraryDisabledReason::RequestInFlight,
        ),
        (
            LibrarySlotsStatus::Failed {
                kind: SlotRequestKind::StorePack,
                slot: None,
                code: ClientDiagnosticCode::Store,
            },
            LibraryDisabledReason::DecisionPending,
        ),
        (
            LibrarySlotsStatus::ExportReady {
                source: ExportSource::ActivePack {
                    hash: nyon::workshop::CatalogHash([1; 32]),
                },
            },
            LibraryDisabledReason::ExportWaiting,
        ),
        (
            LibrarySlotsStatus::PackStored {
                hash: nyon::workshop::CatalogHash([1; 32]),
            },
            LibraryDisabledReason::ImportFinished,
        ),
    ] {
        let built = model(LibraryUiContext {
            slots: Some(&listed),
            status,
            workshop_active: true,
            handoff_available: true,
            archive_import_available: true,
            ..LibraryUiContext::default()
        });
        for id in [
            "library.transfer.import-archive",
            "library.transfer.import-pack",
            "library.transfer.export-active-archive",
            "library.transfer.export-active-pack",
        ] {
            assert_eq!(
                control(&built, id).disabled_reason,
                Some(reason),
                "{id} over {status:?}"
            );
        }
    }
}

/// §6: Workshop Archive import stays disabled while the resident Workshop
/// cannot be replaced; content-pack import and both exports do not.
#[test]
fn only_import_galaxy_takes_the_replacement_gate() {
    let built = model(LibraryUiContext {
        workshop_active: true,
        replacement_blocked: true,
        handoff_available: true,
        archive_import_available: true,
        ..LibraryUiContext::default()
    });
    assert_eq!(
        control(&built, "library.transfer.import-archive").disabled_reason,
        Some(LibraryDisabledReason::ReplacementBlocked)
    );
    for id in [
        "library.transfer.import-pack",
        "library.transfer.export-active-archive",
        "library.transfer.export-active-pack",
    ] {
        assert!(control(&built, id).enabled, "{id}");
    }
    // The actionable fact outranks the missing adapter.
    let neither = model(LibraryUiContext {
        workshop_active: true,
        replacement_blocked: true,
        ..LibraryUiContext::default()
    });
    assert_eq!(
        control(&neither, "library.transfer.import-archive").disabled_reason,
        Some(LibraryDisabledReason::ReplacementBlocked)
    );
}

/// The resident slot's Archive is refused with the runtime's own reason.
///
/// The runtime refuses it in `dispatch_slot_request` with
/// `ClientDiagnosticCode::ResidentSlot`; the screen must report that code
/// rather than inventing a second sentence for the same rule. A neighbouring
/// non-resident row proves the disable tracks residency and is not a blanket
/// Archive gate.
#[test]
fn the_resident_slot_cannot_be_archived_and_says_why_in_the_runtimes_words() {
    let listed = list(vec![
        summary(1, "Andromeda", false),
        summary(2, "Bode", false),
    ]);
    let resident = model(LibraryUiContext {
        slots: Some(&listed),
        selected_slot: Some(SlotId(1)),
        resident_slot: Some(SlotId(1)),
        ..LibraryUiContext::default()
    });
    let archive = control(&resident, "library.action.archive");
    assert!(!archive.enabled);
    assert_eq!(
        archive.disabled_reason,
        Some(LibraryDisabledReason::ResidentSlot)
    );
    assert_eq!(
        LibraryDisabledReason::ResidentSlot.diagnostic_code(),
        Some(ClientDiagnosticCode::ResidentSlot)
    );
    assert!(resident.rows[0].resident);

    let neighbour = model(LibraryUiContext {
        slots: Some(&listed),
        selected_slot: Some(SlotId(2)),
        resident_slot: Some(SlotId(1)),
        ..LibraryUiContext::default()
    });
    assert!(control(&neighbour, "library.action.archive").enabled);
}

/// The Archive control's two directions, read together, plus archived Export.
///
/// Both halves matter and the suite previously had only one. The identifier is
/// shared so focus survives the toggle, which means the **intent** is the only
/// thing distinguishing Archive from Unarchive: an Archive button that submits
/// `UnarchiveSlot` keeps its label, its id and every gate, and nothing else in
/// the suite would notice. Task 9 puts a confirmation modal in front of this
/// exact control and will choose its wording from the label while the store
/// receives the intent, so the pairing is the whole argument.
///
/// Export of an archived row is asserted here too. §3 and §10 forbid *opening*
/// and *selecting for Continue* an archived slot; neither forbids reading bytes
/// out of one, and §4 makes row Export non-mutating. Permitting it is a reading
/// of the spec, so it gets a witness rather than a silence.
#[test]
fn the_archive_control_submits_each_direction_under_one_stable_identifier() {
    let active = list(vec![summary(9, "Bode", false)]);
    let archived = list(vec![summary(9, "Bode", true)]);
    let built_active = model(LibraryUiContext {
        slots: Some(&active),
        selected_slot: Some(SlotId(9)),
        ..LibraryUiContext::default()
    });
    let built_archived = model(LibraryUiContext {
        slots: Some(&archived),
        selected_slot: Some(SlotId(9)),
        ..LibraryUiContext::default()
    });

    assert_eq!(
        built_active.actions.archive.action_id, built_archived.actions.archive.action_id,
        "the identifier is stable across the toggle; only the label and intent flip"
    );
    assert_eq!(
        built_active.actions.archive.action_id.as_str(),
        "library.action.archive"
    );

    assert_eq!(built_active.actions.archive.label, "Archive");
    assert!(built_active.actions.archive.enabled);
    assert_eq!(
        built_active.actions.archive.intent(),
        Some(&LibraryUiIntent::ArchiveSlot { slot: SlotId(9) }),
        "the Archive label must not submit Unarchive"
    );

    assert_eq!(built_archived.actions.archive.label, "Unarchive");
    assert!(built_archived.actions.archive.enabled);
    assert_eq!(
        built_archived.actions.archive.intent(),
        Some(&LibraryUiIntent::UnarchiveSlot { slot: SlotId(9) }),
        "the Unarchive label must not submit Archive"
    );

    assert_eq!(
        built_archived.actions.open.disabled_reason,
        Some(LibraryDisabledReason::ArchivedSlot)
    );
    assert_eq!(
        built_archived.actions.use_for_continue.disabled_reason,
        Some(LibraryDisabledReason::ArchivedSlot)
    );
    assert!(
        built_archived.actions.export.enabled,
        "an archived save must still be exportable: §3 forbids opening it, not reading it"
    );
}

/// Row state outranks a capability deferral in the disabled reason.
///
/// Documented precedence: no subject, then row facts, then lane occupancy, then
/// session-wide facts. An archived row with a Workshop resident reports
/// `ArchivedSlot`, the thing about this row the user can act on now.
#[test]
fn row_state_outranks_capability_in_the_reported_reason() {
    let listed = list(vec![summary(9, "Bode", true)]);
    let built = model(LibraryUiContext {
        slots: Some(&listed),
        selected_slot: Some(SlotId(9)),
        workshop_active: true,
        ..LibraryUiContext::default()
    });
    assert_eq!(
        built.actions.use_for_continue.disabled_reason,
        Some(LibraryDisabledReason::ArchivedSlot)
    );
}

/// With no selection every action is present and disabled with `NoSelection`.
#[test]
fn with_no_selection_every_action_is_present_and_disabled() {
    let listed = list(vec![summary(1, "Andromeda", false)]);
    let built = model(LibraryUiContext {
        slots: Some(&listed),
        ..LibraryUiContext::default()
    });
    assert!(built.actions.selected.is_none());
    for control in [
        &built.actions.open,
        &built.actions.rename,
        &built.actions.archive,
        &built.actions.use_for_continue,
        &built.actions.export,
    ] {
        assert!(!control.enabled, "{}", control.action_id);
        assert_eq!(
            control.disabled_reason,
            Some(LibraryDisabledReason::NoSelection),
            "{}",
            control.action_id
        );
    }
}

/// A selection naming a slot the store no longer reports falls back to none.
#[test]
fn a_selection_the_store_no_longer_reports_falls_back_to_no_selection() {
    let listed = list(vec![summary(1, "Andromeda", false)]);
    let built = model(LibraryUiContext {
        slots: Some(&listed),
        selected_slot: Some(SlotId(42)),
        ..LibraryUiContext::default()
    });
    assert!(built.actions.selected.is_none());
    assert_eq!(
        built.actions.rename.disabled_reason,
        Some(LibraryDisabledReason::NoSelection)
    );
}

/// The single slot-request lane disables exactly the controls that use it.
///
/// `start_slot_request` accepts only from `Idle`, so Refresh, Rename and
/// Archive would be refused outright. Open, Use for Continue and row Export
/// joined them in tasks 12a to 12c, and the transfer strip in task 11: their
/// progress, failures and results are shown in the same request strip. Close
/// and the rows do not share it.
#[test]
fn an_occupied_lane_disables_only_the_controls_that_share_it() {
    let listed = list(vec![summary(1, "Andromeda", false)]);
    for (status, reason) in [
        (
            LibrarySlotsStatus::Working {
                kind: SlotRequestKind::List,
                slot: None,
            },
            LibraryDisabledReason::RequestInFlight,
        ),
        (
            LibrarySlotsStatus::Failed {
                kind: SlotRequestKind::List,
                slot: None,
                code: ClientDiagnosticCode::Store,
            },
            LibraryDisabledReason::DecisionPending,
        ),
    ] {
        let built = model(LibraryUiContext {
            slots: Some(&listed),
            status,
            selected_slot: Some(SlotId(1)),
            handoff_available: true,
            archive_import_available: true,
            ..LibraryUiContext::default()
        });
        for id in [
            "library.refresh",
            "library.action.open",
            "library.action.rename",
            "library.action.archive",
            "library.action.use-for-continue",
            "library.action.export",
        ] {
            assert_eq!(control(&built, id).disabled_reason, Some(reason), "{id}");
        }
        // With a Workshop open, so the two exports have a subject.
        let built = model(LibraryUiContext {
            workshop_active: true,
            ..LibraryUiContext {
                slots: Some(&listed),
                status,
                selected_slot: Some(SlotId(1)),
                handoff_available: true,
                archive_import_available: true,
                ..LibraryUiContext::default()
            }
        });
        for id in [
            "library.transfer.import-archive",
            "library.transfer.import-pack",
            "library.transfer.export-active-archive",
            "library.transfer.export-active-pack",
        ] {
            assert_eq!(control(&built, id).disabled_reason, Some(reason), "{id}");
        }
        for id in ["library.close", "library.slot.1"] {
            assert!(control(&built, id).enabled, "{id}");
        }
    }
}

/// Exporting the active Workshop needs one to be active.
#[test]
fn exporting_the_active_workshop_requires_an_active_workshop() {
    let built = model(LibraryUiContext {
        archive_import_available: true,
        handoff_available: true,
        workshop_active: false,
        ..LibraryUiContext::default()
    });
    for id in [
        "library.transfer.export-active-archive",
        "library.transfer.export-active-pack",
    ] {
        assert_eq!(
            control(&built, id).disabled_reason,
            Some(LibraryDisabledReason::WorkshopInactive),
            "{id}"
        );
    }
    for id in [
        "library.transfer.import-archive",
        "library.transfer.import-pack",
    ] {
        assert!(control(&built, id).enabled, "{id}");
    }

    // Neither a session nor an adapter: the shape a user with no adapter and no
    // open Workshop actually sees, and the only one where the documented
    // precedence — subject facts before capability deferrals — is observable on
    // the transfer strip.
    let neither = model(LibraryUiContext {
        archive_import_available: false,
        handoff_available: false,
        workshop_active: false,
        ..LibraryUiContext::default()
    });
    for id in [
        "library.transfer.export-active-archive",
        "library.transfer.export-active-pack",
    ] {
        assert_eq!(
            control(&neither, id).disabled_reason,
            Some(LibraryDisabledReason::WorkshopInactive),
            "{id}: the missing subject outranks the missing adapter"
        );
    }
}

// ---------------------------------------------------------------------------
// Intents
// ---------------------------------------------------------------------------

/// Intents that resolve a generation carry the one observed in the row.
///
/// §4 requires a compare-and-swap against the generation the user saw, not
/// against whatever the head is by the time the request lands.
#[test]
fn open_continue_and_export_carry_the_generation_observed_in_the_row() {
    let listed = list(vec![summary(5, "Andromeda", false)]);
    let observed = listed.slots[0].generation;
    let built = model(LibraryUiContext {
        slots: Some(&listed),
        selected_slot: Some(SlotId(5)),
        ..LibraryUiContext::default()
    });
    assert_eq!(
        built.actions.open.intent(),
        Some(&LibraryUiIntent::OpenSlot {
            slot: SlotId(5),
            generation: observed
        })
    );
    assert_eq!(
        built.actions.use_for_continue.intent(),
        Some(&LibraryUiIntent::UseForContinue {
            slot: SlotId(5),
            generation: observed
        })
    );
    assert_eq!(
        built.actions.export.intent(),
        Some(&LibraryUiIntent::ExportSlot {
            slot: SlotId(5),
            generation: observed
        })
    );
    assert_eq!(
        built.actions.rename.intent(),
        Some(&LibraryUiIntent::RenameSlot { slot: SlotId(5) })
    );
}

/// Activation returns the control's intent, and a disabled control returns none.
#[test]
fn activation_yields_the_intent_and_a_disabled_control_yields_nothing() {
    let listed = list(vec![summary(1, "Andromeda", false)]);
    let built = model(LibraryUiContext {
        slots: Some(&listed),
        ..LibraryUiContext::default()
    });
    assert_eq!(
        built.activate(
            &SemanticActionId::new("library.slot.1"),
            InputModality::Pointer
        ),
        Some(LibraryUiIntent::SelectSlot(SlotId(1)))
    );
    assert_eq!(
        built.activate(
            &SemanticActionId::new("library.slot.1"),
            InputModality::Keyboard
        ),
        Some(LibraryUiIntent::SelectSlot(SlotId(1)))
    );
    assert!(
        built
            .activate(
                &SemanticActionId::new("library.action.open"),
                InputModality::Keyboard
            )
            .is_none(),
        "a disabled control must not be activatable"
    );
    assert!(
        built
            .activate(
                &SemanticActionId::new("library.nonexistent"),
                InputModality::Keyboard
            )
            .is_none()
    );
}

// ---------------------------------------------------------------------------
// Focus order and the semantic tree
// ---------------------------------------------------------------------------

/// Every control has exactly one focus slot, and every focus slot a control.
#[test]
fn focus_order_covers_every_control_exactly_once() {
    let listed = list(vec![
        summary(1, "Andromeda", false),
        summary(2, "Bode", true),
    ]);
    let built = model(LibraryUiContext {
        slots: Some(&listed),
        selected_slot: Some(SlotId(1)),
        status: LibrarySlotsStatus::Failed {
            kind: SlotRequestKind::Archive,
            slot: Some(SlotId(1)),
            code: ClientDiagnosticCode::Store,
        },
        ..LibraryUiContext::default()
    });
    let order = built.focus_order();
    let mut sorted = order.to_vec();
    sorted.sort();
    sorted.dedup();
    assert_eq!(sorted.len(), order.len(), "focus order has no duplicates");
    assert_eq!(order.len(), built.controls().len());
    for control in built.controls() {
        assert!(
            order.contains(&control.action_id),
            "{} has no focus slot",
            control.action_id
        );
    }
}

/// Focus order is header, decision, rows, actions, transfer.
#[test]
fn focus_order_is_deterministic_and_reads_top_to_bottom() {
    let listed = list(vec![
        summary(1, "Andromeda", false),
        summary(2, "Bode", true),
    ]);
    let built = model(LibraryUiContext {
        slots: Some(&listed),
        selected_slot: Some(SlotId(1)),
        status: LibrarySlotsStatus::Failed {
            kind: SlotRequestKind::Archive,
            slot: Some(SlotId(1)),
            code: ClientDiagnosticCode::Store,
        },
        ..LibraryUiContext::default()
    });
    let order: Vec<&str> = built
        .focus_order()
        .iter()
        .map(SemanticActionId::as_str)
        .collect();
    assert_eq!(
        order,
        vec![
            "library.close",
            "library.refresh",
            "library.request.retry",
            "library.request.cancel",
            "library.slot.1",
            "library.slot.2",
            "library.action.open",
            "library.action.rename",
            "library.action.archive",
            "library.action.use-for-continue",
            "library.action.export",
            "library.transfer.import-archive",
            "library.transfer.import-pack",
            "library.transfer.export-active-archive",
            "library.transfer.export-active-pack",
        ]
    );
}

/// Focus order is stable across a selection change, archived rows included.
///
/// Action identifiers do not encode the selected row, so moving the selection
/// must not renumber anything the focus manager is holding.
///
/// The archived row is the case that matters and the one an all-active fixture
/// would pass regardless: Archive and Unarchive are one identifier, so the
/// focused control survives the moment an Archive lands and the re-listed row
/// comes back archived. A toggling identifier would strand both the focus
/// manager and a modal's restore target on an id that no longer exists.
#[test]
fn focus_order_is_stable_across_a_selection_change_including_archived_rows() {
    let listed = list(vec![
        summary(1, "Andromeda", false),
        summary(2, "Bode", false),
        summary(3, "Cigar", true),
    ]);
    let first = model(LibraryUiContext {
        slots: Some(&listed),
        selected_slot: Some(SlotId(1)),
        ..LibraryUiContext::default()
    });
    let second = model(LibraryUiContext {
        slots: Some(&listed),
        selected_slot: Some(SlotId(2)),
        ..LibraryUiContext::default()
    });
    let archived = model(LibraryUiContext {
        slots: Some(&listed),
        selected_slot: Some(SlotId(3)),
        ..LibraryUiContext::default()
    });
    assert_eq!(first.focus_order(), second.focus_order());
    assert_eq!(
        first.focus_order(),
        archived.focus_order(),
        "selecting an archived row must not move or rename a focus slot"
    );
    assert_eq!(first.actions.archive.label, "Archive");
    assert_eq!(archived.actions.archive.label, "Unarchive");
    assert!(first.rows[0].control.selected);
    assert!(!first.rows[1].control.selected);
    assert!(second.rows[1].control.selected);
    assert!(archived.rows[2].control.selected);
}

/// The Continue target does not offer to become the Continue target again.
///
/// Not spelled out by the design; it is this model's decision, so it is pinned
/// rather than left implicit. The neighbouring row proves the disable tracks
/// the marker instead of blanket-disabling the action.
#[test]
fn the_row_already_selected_for_continue_cannot_be_selected_again() {
    let mut slots = vec![summary(1, "Andromeda", false), summary(2, "Bode", false)];
    slots[0].selected_for_continue = true;
    let listed = list(slots);
    let marked = model(LibraryUiContext {
        slots: Some(&listed),
        selected_slot: Some(SlotId(1)),
        ..LibraryUiContext::default()
    });
    assert!(marked.rows[0].selected_for_continue);
    assert_eq!(
        marked.actions.use_for_continue.disabled_reason,
        Some(LibraryDisabledReason::AlreadyContinue)
    );
    let other = model(LibraryUiContext {
        slots: Some(&listed),
        selected_slot: Some(SlotId(2)),
        ..LibraryUiContext::default()
    });
    assert!(other.actions.use_for_continue.enabled);
}

/// Keyboard traversal reaches the sixteenth of sixteen rows.
#[test]
fn keyboard_traversal_reaches_row_sixteen_of_sixteen() {
    let listed = list(
        (1..=16)
            .map(|id| summary(id, &format!("Galaxy {id}"), false))
            .collect(),
    );
    let built = model(LibraryUiContext {
        slots: Some(&listed),
        ..LibraryUiContext::default()
    });
    assert_eq!(built.rows.len(), 16);
    let last = SemanticActionId::new("library.slot.16");
    assert!(built.focus_order().contains(&last));
    assert_eq!(
        built.activate(&last, InputModality::Keyboard),
        Some(LibraryUiIntent::SelectSlot(SlotId(16)))
    );
}

/// The semantic tree validates: unique node ids, unique actions, all focusable.
#[test]
fn the_semantic_tree_validates_in_every_shape() {
    let listed = list(vec![
        summary(1, &"N".repeat(64), false),
        summary(2, "Bode", true),
    ]);
    let empty = list(Vec::new());
    for context in [
        LibraryUiContext::default(),
        LibraryUiContext {
            slots: Some(&empty),
            ..LibraryUiContext::default()
        },
        LibraryUiContext {
            slots: Some(&listed),
            selected_slot: Some(SlotId(2)),
            resident_slot: Some(SlotId(1)),
            replacement_blocked: true,
            archive_import_available: true,
            handoff_available: true,
            workshop_active: true,
            confirmation: None,
            rename_draft: None,
            status: LibrarySlotsStatus::Failed {
                kind: SlotRequestKind::Unarchive,
                slot: Some(SlotId(2)),
                code: ClientDiagnosticCode::Store,
            },
        },
    ] {
        let built = model(context);
        built.semantics.validate().expect("semantic tree validates");
    }
}

/// Every disabled control's node carries its reason as a value.
///
/// A focusable control that refuses activation and says nothing about why is
/// the accessibility half of Finding 5.
#[test]
fn every_disabled_node_states_its_reason() {
    let listed = list(vec![summary(1, "Andromeda", false)]);
    let built = model(LibraryUiContext {
        slots: Some(&listed),
        ..LibraryUiContext::default()
    });
    let mut disabled = 0;
    for node in built.semantics.nodes_depth_first() {
        if node.action_id.is_some() && !node.enabled {
            disabled += 1;
            let value = node.value.as_deref().unwrap_or_default();
            assert!(!value.is_empty(), "{} has no reason", node.id);
        }
    }
    assert!(disabled > 0, "this shape must contain disabled controls");
}

/// A disabled control refuses to hand out its intent at all.
///
/// The placeholder subject in a disabled action is `SlotId(0)`, and the native
/// store mints slot identifiers from zero upward, so that placeholder names a
/// real save. Reading the intent off a disabled control must be impossible
/// rather than merely discouraged.
#[test]
fn a_disabled_control_hands_out_no_intent_because_its_subject_is_a_real_slot_id() {
    let listed = list(vec![summary(0, "Andromeda", false)]);
    let built = model(LibraryUiContext {
        slots: Some(&listed),
        selected_slot: None,
        ..LibraryUiContext::default()
    });
    let open = &built.actions.open;
    assert!(!open.enabled);
    assert_eq!(
        open.disabled_reason,
        Some(LibraryDisabledReason::NoSelection)
    );
    assert!(
        open.intent().is_none(),
        "a disabled action must not name slot 0, which is a real save"
    );
    assert_eq!(
        built.rows[0].slot,
        SlotId(0),
        "slot 0 exists, so the placeholder is not a safe sentinel"
    );
    for control in built.controls() {
        assert_eq!(
            control.intent().is_some(),
            control.enabled,
            "{}",
            control.action_id
        );
    }
}

// ---------------------------------------------------------------------------
// Payloads: every control, not just the ones an earlier test happened to name
// ---------------------------------------------------------------------------

/// Every control's exact intent, in one fully-enabled shape.
///
/// Six of the fifteen intent variants had no witness at all — `RefreshSlots`,
/// `ArchiveSlot`, `ImportArchive`, `ImportPack`, `ExportActiveArchive` and
/// `ExportActivePack`. For those controls the suite asserted identity,
/// enablement and disabled reason, but never the payload the runtime acts on,
/// so two controls could swap intents and stay green. This pins the payload of
/// all fifteen at once; it deliberately overlaps the Archive-direction test,
/// because redundancy about which button submits what is the right direction to
/// be redundant in.
#[test]
fn every_control_submits_its_own_intent_in_a_fully_enabled_shape() {
    let listed = list(vec![summary(4, "Andromeda", false)]);
    let built = model(LibraryUiContext {
        slots: Some(&listed),
        selected_slot: Some(SlotId(4)),
        archive_import_available: true,
        handoff_available: true,
        workshop_active: true,
        status: LibrarySlotsStatus::Idle,
        resident_slot: None,
        replacement_blocked: false,
        confirmation: None,
        rename_draft: None,
    });
    let generation = listed.slots[0].generation;
    let expected: Vec<(&str, LibraryUiIntent)> = vec![
        ("library.close", LibraryUiIntent::Close),
        ("library.refresh", LibraryUiIntent::RefreshSlots),
        ("library.slot.4", LibraryUiIntent::SelectSlot(SlotId(4))),
        (
            "library.action.open",
            LibraryUiIntent::OpenSlot {
                slot: SlotId(4),
                generation,
            },
        ),
        (
            "library.action.rename",
            LibraryUiIntent::RenameSlot { slot: SlotId(4) },
        ),
        (
            "library.action.archive",
            LibraryUiIntent::ArchiveSlot { slot: SlotId(4) },
        ),
        (
            "library.action.export",
            LibraryUiIntent::ExportSlot {
                slot: SlotId(4),
                generation,
            },
        ),
        (
            "library.transfer.import-archive",
            LibraryUiIntent::ImportArchive,
        ),
        ("library.transfer.import-pack", LibraryUiIntent::ImportPack),
        (
            "library.transfer.export-active-archive",
            LibraryUiIntent::ExportActiveArchive,
        ),
        (
            "library.transfer.export-active-pack",
            LibraryUiIntent::ExportActivePack,
        ),
    ];
    for (action, intent) in &expected {
        let found = control(&built, action);
        assert!(found.enabled, "{action} must be enabled in this shape");
        assert_eq!(found.intent(), Some(intent), "{action}");
    }
    // Use for Continue cannot be enabled beside the two active-Workshop
    // exports: a resident Workshop is exactly what refuses it. Its payload is
    // pinned in the same shape without one.
    let without_workshop = model(LibraryUiContext {
        workshop_active: false,
        slots: Some(&listed),
        selected_slot: Some(SlotId(4)),
        ..LibraryUiContext::default()
    });
    assert_eq!(
        control(&without_workshop, "library.action.use-for-continue").intent(),
        Some(&LibraryUiIntent::UseForContinue {
            slot: SlotId(4),
            generation,
        })
    );
    assert_eq!(
        built.controls().len(),
        expected.len() + 1,
        "the table plus Use for Continue must cover every control in this shape"
    );
}

/// The two decision controls' payloads, in the shape that produces them.
///
/// They cannot appear in the fully-enabled table above, because an idle lane
/// emits neither.
#[test]
fn the_decision_controls_submit_retry_and_cancel() {
    let built = model(LibraryUiContext {
        status: LibrarySlotsStatus::Failed {
            kind: SlotRequestKind::Rename,
            slot: Some(SlotId(1)),
            code: ClientDiagnosticCode::Store,
        },
        ..LibraryUiContext::default()
    });
    assert_eq!(
        control(&built, "library.request.retry").intent(),
        Some(&LibraryUiIntent::RetrySlotRequest)
    );
    assert_eq!(
        control(&built, "library.request.cancel").intent(),
        Some(&LibraryUiIntent::CancelSlotRequest)
    );
}

// ---------------------------------------------------------------------------
// The content line must name the route that is actually live
// ---------------------------------------------------------------------------

/// Returns the content status line the screen shows.
fn content_line(built: &LibraryUiModel) -> String {
    built
        .semantics
        .node("library.content")
        .expect("content node")
        .description
        .clone()
}

/// A first-visit listing failure must not point at the disabled Refresh.
///
/// `open_library` dispatches `ListSlots` from `Idle`; a store that refuses
/// leaves `Failed { List }` with no cached list, so this is the **first-run**
/// failure path. Refresh is disabled with `DecisionPending` there and Retry is
/// the live route, but the content line told the user to press Refresh.
#[test]
fn a_failed_first_listing_does_not_tell_the_user_to_press_disabled_refresh() {
    let built = model(LibraryUiContext {
        slots: None,
        status: LibrarySlotsStatus::Failed {
            kind: SlotRequestKind::List,
            slot: None,
            code: ClientDiagnosticCode::Store,
        },
        ..LibraryUiContext::default()
    });
    assert_eq!(built.content, LibraryContent::NotListed);
    assert_eq!(
        control(&built, "library.refresh").disabled_reason,
        Some(LibraryDisabledReason::DecisionPending),
        "the premise: Refresh is not the live route here"
    );
    let line = content_line(&built);
    assert!(
        !line.contains("Refresh"),
        "content line points at a disabled control: {line}"
    );
    assert!(line.contains("failed request"), "{line}");
    let request_line = built
        .semantics
        .node("library.request.status")
        .expect("request node")
        .description
        .clone();
    assert_ne!(
        line, request_line,
        "two announcements must not carry the identical sentence"
    );
}

/// A mutation still in flight must not point at the disabled Refresh either.
///
/// The transient sibling of the case above: Refresh is disabled with
/// `RequestInFlight`, so the same instruction is equally wrong, just briefly.
#[test]
fn a_listing_pending_behind_a_mutation_does_not_point_at_disabled_refresh() {
    let built = model(LibraryUiContext {
        slots: None,
        status: LibrarySlotsStatus::Working {
            kind: SlotRequestKind::Rename,
            slot: Some(SlotId(1)),
        },
        ..LibraryUiContext::default()
    });
    assert_eq!(built.content, LibraryContent::NotListed);
    assert_eq!(
        control(&built, "library.refresh").disabled_reason,
        Some(LibraryDisabledReason::RequestInFlight)
    );
    let line = content_line(&built);
    assert!(
        !line.contains("Refresh"),
        "content line points at a disabled control: {line}"
    );
    assert!(line.contains("Waiting"), "{line}");
}

/// With the lane free, Refresh really is the route, and the line says so.
///
/// The third arm, and the one that stops the conditional from collapsing into
/// "never mention Refresh".
#[test]
fn an_idle_unlisted_library_does_point_at_refresh() {
    let built = model(LibraryUiContext::default());
    assert_eq!(built.content, LibraryContent::NotListed);
    assert!(control(&built, "library.refresh").enabled);
    let line = content_line(&built);
    assert!(line.contains("Refresh to list them"), "{line}");
}

/// A disabled control's `Debug` never prints its placeholder subject.
///
/// The typed path was already closed by `intent()`, but the derived `Debug`
/// printed `OpenSlot { slot: SlotId(0), .. }` for a disabled action — and slot 0
/// is a real save, because the native store mints identifiers from zero upward.
/// A log line or a panic message is a way out of the type system.
#[test]
fn a_disabled_controls_debug_output_names_no_slot() {
    let built = model(LibraryUiContext {
        slots: Some(&list(vec![summary(0, "Andromeda", false)])),
        selected_slot: None,
        ..LibraryUiContext::default()
    });
    let rendered = format!("{:?}", built.actions.open);
    assert!(!built.actions.open.enabled);
    assert!(
        rendered.contains("<disabled>"),
        "disabled intent must be elided: {rendered}"
    );
    assert!(
        !rendered.contains("SlotId"),
        "disabled Debug must not name a slot: {rendered}"
    );

    let enabled = model(LibraryUiContext {
        slots: Some(&list(vec![summary(0, "Andromeda", false)])),
        selected_slot: Some(SlotId(0)),
        ..LibraryUiContext::default()
    });
    let rendered = format!("{:?}", enabled.actions.open);
    assert!(
        rendered.contains("SlotId(0)"),
        "an enabled control still prints its intent: {rendered}"
    );
}

/// Task 7 review F10: a colliding action id is loud, not silently absorbed.
///
/// `build` inserts every control into a map keyed by action id and pushes a
/// focus slot only for a fresh key. On a collision the later control overwrote
/// the earlier one and no slot was pushed, so one control lost its focus slot
/// while `activate` resolved to the other — and
/// `focus_order_covers_every_control_exactly_once` could not see it, because
/// `order.len()` and `controls().len()` shrink together.
///
/// Row ids derive from `SlotId`, so two rows sharing one is the collision this
/// test can build through the public API. Hand-named constants colliding is
/// the other source, and it is the one task 8 creates when it adds controls.
/// Either is a bug rather than input, which is why this is a debug assertion
/// and not a fallible `build`: every caller is a frame builder, and release
/// behaviour is deliberately unchanged.
#[test]
#[cfg(debug_assertions)]
#[should_panic(expected = "duplicate Library action id")]
fn a_colliding_action_id_is_loud_rather_than_silently_dropped() {
    let colliding = list(vec![
        summary(7, "First", false),
        summary(7, "Second", false),
    ]);
    let _ = model(LibraryUiContext {
        slots: Some(&colliding),
        ..LibraryUiContext::default()
    });
}

/// Rename, Archive and Unarchive are live for any selected row the lane can
/// serve, because each only opens its dialog (task 9); the resident save still
/// cannot be archived, and that reason is the one it shows.
#[test]
fn slot_changes_are_live_because_each_opens_a_dialog_first() {
    let listed = list(vec![
        summary(1, "Andromeda", false),
        summary(2, "Bode", true),
        summary(3, "Cartwheel", false),
    ]);
    for (selected, archive_label, archive_intent) in [
        (
            SlotId(1),
            "Archive",
            LibraryUiIntent::ArchiveSlot { slot: SlotId(1) },
        ),
        (
            SlotId(2),
            "Unarchive",
            LibraryUiIntent::UnarchiveSlot { slot: SlotId(2) },
        ),
    ] {
        let built = model(LibraryUiContext {
            slots: Some(&listed),
            selected_slot: Some(selected),
            ..LibraryUiContext::default()
        });
        let rename = control(&built, "library.action.rename");
        assert!(rename.enabled);
        assert_eq!(
            built.activate(&rename.action_id, InputModality::Pointer),
            Some(LibraryUiIntent::RenameSlot { slot: selected })
        );
        let archive = control(&built, "library.action.archive");
        assert_eq!(archive.label, archive_label);
        assert_eq!(
            built.activate(&archive.action_id, InputModality::Pointer),
            Some(archive_intent)
        );
    }

    let resident = model(LibraryUiContext {
        slots: Some(&listed),
        selected_slot: Some(SlotId(3)),
        resident_slot: Some(SlotId(3)),
        ..LibraryUiContext::default()
    });
    assert_eq!(
        control(&resident, "library.action.archive").disabled_reason,
        Some(LibraryDisabledReason::ResidentSlot),
        "the resident save must not be archivable"
    );
    assert!(control(&resident, "library.action.rename").enabled);
}

fn confirm_model<'a>(
    listed: &'a SlotList,
    kind: LibraryConfirmationKind,
    slot: u64,
    tweak: impl FnOnce(&mut LibraryUiContext<'a>),
) -> LibraryUiModel {
    let mut context = LibraryUiContext {
        slots: Some(listed),
        selected_slot: Some(SlotId(slot)),
        confirmation: Some(LibraryConfirmationRequest {
            kind,
            slot: SlotId(slot),
        }),
        ..LibraryUiContext::default()
    };
    tweak(&mut context);
    model(context)
}

/// Addendum §3: the confirmation names the save and says whether Archive will
/// clear Continue, and its confirm control submits exactly the change the text
/// describes. The dialog is a direct child of the root, where the frame and
/// `modal_dialog` look for it, and its two controls close the focus order.
#[test]
fn archive_and_unarchive_confirm_with_text_that_states_the_consequence() {
    let mut continue_row = summary(1, "Andromeda", false);
    continue_row.selected_for_continue = true;
    let listed = list(vec![
        continue_row,
        summary(2, "Bode", true),
        summary(3, "Cartwheel", false),
    ]);
    for (kind, slot, name, sentence, label, forbidden) in [
        (
            LibraryConfirmationKind::Archive,
            1,
            "Andromeda",
            "Continue will be cleared",
            "Archive",
            "not affected",
        ),
        (
            LibraryConfirmationKind::Archive,
            3,
            "Cartwheel",
            "Continue is not affected",
            "Archive",
            "will be cleared",
        ),
        (
            LibraryConfirmationKind::Unarchive,
            2,
            "Bode",
            "does not become the Continue save",
            "Unarchive",
            "cleared",
        ),
    ] {
        let built = confirm_model(&listed, kind, slot, |_| {});
        let dialog = built.confirmation.as_ref().expect("no dialog");
        assert!(dialog.body.contains(name), "{kind:?} body omits the name");
        assert!(dialog.body.contains(sentence), "{kind:?}: {}", dialog.body);
        assert!(
            !dialog.body.contains(forbidden),
            "{kind:?}: {}",
            dialog.body
        );
        assert!(!dialog.title.contains(name), "the title must stay short");
        let submit = control(&built, "library.confirm.submit");
        assert_eq!(submit.label, label);
        let request = LibraryConfirmationRequest {
            kind,
            slot: SlotId(slot),
        };
        assert_eq!(
            built.activate(&submit.action_id, InputModality::Keyboard),
            Some(LibraryUiIntent::SubmitConfirmation(request))
        );
        assert_eq!(
            built.activate(
                &SemanticActionId::new("library.confirm.cancel"),
                InputModality::Keyboard
            ),
            Some(LibraryUiIntent::CancelConfirmation)
        );
        let node = built
            .semantics
            .root
            .children
            .iter()
            .find(|node| node.role == SemanticRole::Dialog)
            .expect("the dialog is not a child of the root");
        assert_eq!(node.name, dialog.title);
        let order = library_confirmation_order(kind);
        assert_eq!(
            built.focus_order()[built.focus_order().len() - order.len()..],
            order[..]
        );
    }

    // The confirm control never offers what the runtime would refuse.
    let resident = confirm_model(&listed, LibraryConfirmationKind::Archive, 3, |context| {
        context.resident_slot = Some(SlotId(3));
    });
    assert_eq!(
        control(&resident, "library.confirm.submit").disabled_reason,
        Some(LibraryDisabledReason::ResidentSlot)
    );
    let busy = confirm_model(&listed, LibraryConfirmationKind::Unarchive, 2, |context| {
        context.status = LibrarySlotsStatus::Working {
            kind: SlotRequestKind::List,
            slot: None,
        };
    });
    assert_eq!(
        control(&busy, "library.confirm.submit").disabled_reason,
        Some(LibraryDisabledReason::RequestInFlight)
    );
    assert!(control(&busy, "library.confirm.cancel").enabled);

    // A request naming a save the list no longer holds builds no dialog.
    let gone = confirm_model(&listed, LibraryConfirmationKind::Archive, 9, |_| {});
    assert!(gone.confirmation.is_none());
    assert!(
        gone.semantics
            .root
            .children
            .iter()
            .all(|node| node.role != SemanticRole::Dialog)
    );
    let closed = model(LibraryUiContext {
        slots: Some(&listed),
        ..LibraryUiContext::default()
    });
    assert!(
        closed
            .controls()
            .all(|control| !control.action_id.as_str().starts_with("library.confirm"))
    );
}

fn rename_model(listed: &SlotList, draft: &str) -> LibraryUiModel {
    model(LibraryUiContext {
        slots: Some(listed),
        selected_slot: Some(SlotId(3)),
        confirmation: Some(LibraryConfirmationRequest {
            kind: LibraryConfirmationKind::Rename,
            slot: SlotId(3),
        }),
        rename_draft: Some(draft),
        ..LibraryUiContext::default()
    })
}

/// Task 9b: the rename dialog validates live through `SlotName`.
///
/// An invalid draft disables Rename with a reason and shows a one-line notice,
/// and the field still submits on Enter, because the dispatcher validates
/// again and keeps the draft on refusal. The field starts the focus order so
/// typing goes to it, and a draft the field could never hold is not drawn.
#[test]
fn rename_validates_the_draft_and_keeps_the_field_first() {
    let listed = list(vec![summary(3, "Cartwheel", false)]);
    let request = LibraryConfirmationRequest {
        kind: LibraryConfirmationKind::Rename,
        slot: SlotId(3),
    };
    let valid = rename_model(&listed, "Cartwheel Two");
    let dialog = valid.confirmation.as_ref().unwrap();
    let field = control(&valid, "library.confirm.name");
    assert_eq!(field.label, "Name: Cartwheel Two");
    assert_eq!(dialog.name_value.as_deref(), Some("Cartwheel Two"));
    assert!(field.enabled);
    assert_eq!(
        valid.activate(&field.action_id, InputModality::Keyboard),
        Some(LibraryUiIntent::SubmitConfirmation(request))
    );
    let submit = control(&valid, "library.confirm.submit");
    assert_eq!(submit.label, "Rename");
    assert!(submit.enabled);
    assert_eq!(dialog.problem, None);
    let node = valid
        .semantics
        .root
        .children
        .iter()
        .find(|node| node.role == SemanticRole::Dialog)
        .unwrap();
    let field_node = node
        .children
        .iter()
        .find(|child| child.action_id.as_ref() == Some(&field.action_id))
        .unwrap();
    assert_eq!(field_node.role, SemanticRole::TextInput);
    assert_eq!(field_node.value.as_deref(), Some("Cartwheel Two"));
    assert!(
        node.children
            .iter()
            .all(|child| child.role != SemanticRole::Alert)
    );
    assert_eq!(
        valid.focus_order()[valid.focus_order().len() - 3..],
        library_confirmation_order(LibraryConfirmationKind::Rename)[..]
    );
    assert_eq!(
        library_confirmation_order(LibraryConfirmationKind::Rename)[0].as_str(),
        "library.confirm.name"
    );

    for draft in ["", " Cartwheel", "Cartwheel ", "Cart  wheel"] {
        let built = rename_model(&listed, draft);
        let submit = control(&built, "library.confirm.submit");
        assert_eq!(
            submit.disabled_reason,
            Some(LibraryDisabledReason::InvalidName),
            "{draft:?} was accepted"
        );
        assert!(built.confirmation.as_ref().unwrap().problem.is_some());
        assert!(
            built.semantics.nodes_depth_first().into_iter().any(|node| {
                node.role == SemanticRole::Alert && node.id.as_str() == "library.confirm.problem"
            }),
            "{draft:?} shows no notice"
        );
        assert!(
            control(&built, "library.confirm.name").enabled,
            "the field must stay editable"
        );
    }

    let long = "W".repeat(64);
    assert!(
        control(&rename_model(&listed, &long), "library.confirm.submit").enabled,
        "64 bytes is the limit, not past it"
    );
    for unholdable in [
        "W".repeat(65),
        "Caf\u{e9}".to_owned(),
        "Tab\there".to_owned(),
    ] {
        let built = rename_model(&listed, &unholdable);
        assert_eq!(
            control(&built, "library.confirm.name").label,
            "Name: Cartwheel",
            "{unholdable:?} was drawn"
        );
    }
    assert!(rename_draft_acceptable(&long));
    assert!(!rename_draft_acceptable(&"W".repeat(65)));
    assert!(!rename_draft_acceptable("Caf\u{e9}"));
    assert!(
        rename_draft_acceptable(" padded "),
        "spacing is validated live, not filtered"
    );
}

// ---------------------------------------------------------------------------
// Route-design task 12a: Open through the exact-catalog client
// ---------------------------------------------------------------------------

fn request_line(built: &LibraryUiModel) -> String {
    built
        .semantics
        .node("library.request.status")
        .expect("request status node")
        .description
        .clone()
}

/// Open is live, and each §6 or row fact that forbids it reports itself.
///
/// Row Export, beside it, is live in the same shape: task 12c removed the
/// Library-client flag both once read.
#[test]
fn open_is_live_and_refused_only_by_the_facts_that_forbid_it() {
    let listed = list(vec![
        summary(1, "Andromeda", false),
        summary(2, "Bode", true),
    ]);
    let base = LibraryUiContext {
        slots: Some(&listed),
        selected_slot: Some(SlotId(1)),
        ..LibraryUiContext::default()
    };
    let live = model(base);
    let open = control(&live, "library.action.open");
    assert!(open.enabled, "{:?}", open.disabled_reason);
    assert_eq!(
        live.activate(&open.action_id, InputModality::Keyboard),
        Some(LibraryUiIntent::OpenSlot {
            slot: SlotId(1),
            generation: SaveGeneration(11),
        })
    );
    assert_eq!(
        control(&live, "library.action.export").disabled_reason,
        None
    );

    for (context, reason) in [
        (
            LibraryUiContext {
                replacement_blocked: true,
                ..base
            },
            LibraryDisabledReason::ReplacementBlocked,
        ),
        (
            LibraryUiContext {
                resident_slot: Some(SlotId(1)),
                ..base
            },
            LibraryDisabledReason::AlreadyOpen,
        ),
        (
            LibraryUiContext {
                status: LibrarySlotsStatus::Working {
                    kind: SlotRequestKind::Rename,
                    slot: Some(SlotId(1)),
                },
                ..base
            },
            LibraryDisabledReason::RequestInFlight,
        ),
        (
            LibraryUiContext {
                selected_slot: Some(SlotId(2)),
                ..base
            },
            LibraryDisabledReason::ArchivedSlot,
        ),
    ] {
        let built = model(context);
        let open = control(&built, "library.action.open");
        assert!(!open.enabled, "{reason:?}");
        assert_eq!(open.disabled_reason, Some(reason));
    }
}

/// A held open offers its own Open and Cancel, and says which generation.
#[test]
fn a_held_open_offers_acceptance_and_cancel_and_blocks_the_lane() {
    let listed = list(vec![summary(1, "Andromeda", false)]);
    for (recovered, label, line) in [
        (
            true,
            "Open previous",
            "The latest save is invalid. Open its previous generation or cancel.",
        ),
        (
            false,
            "Open",
            "A saved galaxy is now the Continue save. Open it or cancel.",
        ),
    ] {
        let built = model(LibraryUiContext {
            slots: Some(&listed),
            selected_slot: Some(SlotId(1)),
            status: LibrarySlotsStatus::Held {
                slot: SlotId(1),
                recovered,
            },
            ..LibraryUiContext::default()
        });
        let accept = control(&built, "library.request.retry");
        assert_eq!(accept.label, label);
        assert_eq!(accept.intent(), Some(&LibraryUiIntent::AcceptOpen));
        assert_eq!(
            control(&built, "library.request.cancel").intent(),
            Some(&LibraryUiIntent::CancelSlotRequest)
        );
        assert_eq!(request_line(&built), line);
        assert!(built.request.decision_pending());
        let status = built
            .semantics
            .node("library.request.status")
            .expect("request status node");
        assert_eq!(status.role, SemanticRole::Alert);
        for id in [
            "library.action.open",
            "library.action.rename",
            "library.refresh",
        ] {
            assert_eq!(
                control(&built, id).disabled_reason,
                Some(LibraryDisabledReason::DecisionPending),
                "{id}"
            );
        }
    }
}

/// A conflicted open is resolved by Refresh; any other failure by Retry.
#[test]
fn a_conflicted_open_offers_refresh_where_a_failure_offers_retry() {
    for (code, label, line) in [
        (
            ClientDiagnosticCode::StaleSave,
            "Refresh",
            "That save changed. Refresh, then try again.",
        ),
        (
            ClientDiagnosticCode::Store,
            "Retry",
            "Opening a saved galaxy failed. Retry or cancel.",
        ),
    ] {
        let built = model(LibraryUiContext {
            status: LibrarySlotsStatus::Failed {
                kind: SlotRequestKind::Open,
                slot: Some(SlotId(1)),
                code,
            },
            ..LibraryUiContext::default()
        });
        let retry = control(&built, "library.request.retry");
        assert_eq!(retry.label, label);
        assert_eq!(retry.intent(), Some(&LibraryUiIntent::RetrySlotRequest));
        assert_eq!(built.request.failure_code, Some(code));
        assert_eq!(request_line(&built), line);
    }
    let working = model(LibraryUiContext {
        status: LibrarySlotsStatus::Working {
            kind: SlotRequestKind::Open,
            slot: Some(SlotId(1)),
        },
        ..LibraryUiContext::default()
    });
    assert_eq!(request_line(&working), "Opening a saved galaxy.");
}

// ---------------------------------------------------------------------------
// Route-design task 12b: Use for Continue
// ---------------------------------------------------------------------------

/// Use for Continue is live with no Workshop, and each fact that refuses it
/// reports itself. A resident Workshop refuses it even when it could be
/// replaced, which is stricter than §6 on purpose.
#[test]
fn use_for_continue_is_live_and_refused_while_a_workshop_is_resident() {
    let mut continued = summary(3, "Cartwheel", false);
    continued.selected_for_continue = true;
    let listed = list(vec![
        summary(1, "Andromeda", false),
        summary(2, "Bode", true),
        continued,
    ]);
    let base = LibraryUiContext {
        slots: Some(&listed),
        selected_slot: Some(SlotId(1)),
        ..LibraryUiContext::default()
    };
    let live = model(base);
    let action = control(&live, "library.action.use-for-continue");
    assert!(action.enabled, "{:?}", action.disabled_reason);
    assert_eq!(
        live.activate(&action.action_id, InputModality::Pointer),
        Some(LibraryUiIntent::UseForContinue {
            slot: SlotId(1),
            generation: SaveGeneration(11),
        })
    );
    for (context, reason) in [
        (
            LibraryUiContext {
                workshop_active: true,
                ..base
            },
            LibraryDisabledReason::WorkshopResident,
        ),
        (
            LibraryUiContext {
                selected_slot: Some(SlotId(3)),
                ..base
            },
            LibraryDisabledReason::AlreadyContinue,
        ),
        (
            LibraryUiContext {
                selected_slot: Some(SlotId(2)),
                ..base
            },
            LibraryDisabledReason::ArchivedSlot,
        ),
        (
            LibraryUiContext {
                status: LibrarySlotsStatus::Working {
                    kind: SlotRequestKind::Open,
                    slot: Some(SlotId(3)),
                },
                ..base
            },
            LibraryDisabledReason::RequestInFlight,
        ),
    ] {
        let built = model(context);
        let action = control(&built, "library.action.use-for-continue");
        assert!(!action.enabled, "{reason:?}");
        assert_eq!(action.disabled_reason, Some(reason));
    }
    // The resident refusal says why in the semantic tree, in its own words.
    let resident = model(LibraryUiContext {
        workshop_active: true,
        ..base
    });
    let node = resident
        .semantics
        .node("library.control.library.action.use-for-continue")
        .expect("Use for Continue node");
    assert_eq!(
        node.value.as_deref(),
        Some("Continue follows the open Workshop while it is open.")
    );
}

/// Its progress, failure and conflict read as Continue, not as Open.
#[test]
fn use_for_continue_states_say_continue_and_a_conflict_offers_refresh() {
    for (status, line, retry) in [
        (
            LibrarySlotsStatus::Working {
                kind: SlotRequestKind::UseForContinue,
                slot: Some(SlotId(1)),
            },
            "Checking a saved galaxy for Continue.",
            None,
        ),
        (
            LibrarySlotsStatus::Failed {
                kind: SlotRequestKind::UseForContinue,
                slot: Some(SlotId(1)),
                code: ClientDiagnosticCode::Store,
            },
            "Choosing the Continue save failed. Retry or cancel.",
            Some("Retry"),
        ),
        (
            LibrarySlotsStatus::Failed {
                kind: SlotRequestKind::UseForContinue,
                slot: Some(SlotId(1)),
                code: ClientDiagnosticCode::StaleSave,
            },
            "That save changed. Refresh, then try again.",
            Some("Refresh"),
        ),
    ] {
        let built = model(LibraryUiContext {
            status,
            ..LibraryUiContext::default()
        });
        assert_eq!(request_line(&built), line);
        let label = built
            .controls()
            .find(|c| c.action_id.as_str() == "library.request.retry")
            .map(|c| c.label.as_str());
        assert_eq!(label, retry, "{status:?}");
    }
}

/// Folded in from 12a: accepting a held open installs, so it takes the
/// replacement gate the runtime refuses it with.
#[test]
fn a_held_open_cannot_be_accepted_while_the_resident_blocks_replacement() {
    let built = model(LibraryUiContext {
        status: LibrarySlotsStatus::Held {
            slot: SlotId(1),
            recovered: false,
        },
        replacement_blocked: true,
        ..LibraryUiContext::default()
    });
    let accept = control(&built, "library.request.retry");
    assert!(!accept.enabled);
    assert_eq!(
        accept.disabled_reason,
        Some(LibraryDisabledReason::ReplacementBlocked)
    );
    assert!(control(&built, "library.request.cancel").enabled);
}

// ---------------------------------------------------------------------------
// Route-design task 12c: row Export to Ready
// ---------------------------------------------------------------------------

/// Row Export mutates nothing (§4), so no replacement, residency or archived
/// fact refuses it; the one lane is its only gate.
#[test]
fn row_export_is_refused_only_by_the_lane() {
    let listed = list(vec![
        summary(1, "Andromeda", false),
        summary(2, "Bode", true),
    ]);
    for (selected, resident) in [(1, None), (1, Some(SlotId(1))), (2, None)] {
        let built = model(LibraryUiContext {
            slots: Some(&listed),
            selected_slot: Some(SlotId(selected)),
            resident_slot: resident,
            workshop_active: resident.is_some(),
            replacement_blocked: true,
            ..LibraryUiContext::default()
        });
        let export = control(&built, "library.action.export");
        assert!(
            export.enabled,
            "{selected} {resident:?}: {:?}",
            export.disabled_reason
        );
        assert_eq!(
            built.activate(&export.action_id, InputModality::Pointer),
            Some(LibraryUiIntent::ExportSlot {
                slot: SlotId(selected),
                generation: SaveGeneration(selected + 10),
            })
        );
    }
    for (status, reason) in [
        (
            LibrarySlotsStatus::Working {
                kind: SlotRequestKind::Export,
                slot: Some(SlotId(1)),
            },
            LibraryDisabledReason::RequestInFlight,
        ),
        (
            LibrarySlotsStatus::Failed {
                kind: SlotRequestKind::Export,
                slot: Some(SlotId(1)),
                code: ClientDiagnosticCode::Store,
            },
            LibraryDisabledReason::DecisionPending,
        ),
        (
            LibrarySlotsStatus::ExportRecoveryOffered { slot: SlotId(1) },
            LibraryDisabledReason::DecisionPending,
        ),
        (
            LibrarySlotsStatus::ExportReady {
                source: ExportSource::Head {
                    slot: SlotId(1),
                    generation: SaveGeneration(11),
                },
            },
            LibraryDisabledReason::ExportWaiting,
        ),
    ] {
        let built = model(LibraryUiContext {
            slots: Some(&listed),
            selected_slot: Some(SlotId(1)),
            status,
            ..LibraryUiContext::default()
        });
        for id in [
            "library.action.export",
            "library.action.open",
            "library.action.rename",
            "library.refresh",
        ] {
            assert_eq!(
                control(&built, id).disabled_reason,
                Some(reason),
                "{status:?} {id}"
            );
        }
    }
}

/// §4's export-recovery choice: Export previous, which replaces nothing and so
/// is live even when the resident cannot be replaced.
#[test]
fn an_invalid_head_export_offers_export_previous_and_cancel() {
    let listed = list(vec![summary(1, "Andromeda", false)]);
    let built = model(LibraryUiContext {
        slots: Some(&listed),
        selected_slot: Some(SlotId(1)),
        replacement_blocked: true,
        status: LibrarySlotsStatus::ExportRecoveryOffered { slot: SlotId(1) },
        ..LibraryUiContext::default()
    });
    let accept = control(&built, "library.request.retry");
    assert_eq!(accept.label, "Export previous");
    assert!(accept.enabled, "{:?}", accept.disabled_reason);
    assert_eq!(
        accept.intent(),
        Some(&LibraryUiIntent::AcceptExportRecovery)
    );
    assert_eq!(
        control(&built, "library.request.cancel").intent(),
        Some(&LibraryUiIntent::CancelSlotRequest)
    );
    assert_eq!(
        request_line(&built),
        "The latest save is invalid. Export its previous generation or cancel."
    );
    assert!(built.request.decision_pending());
    assert!(
        !built
            .controls()
            .any(|control| control.action_id.as_str() == "library.request.handoff")
    );
}

/// Ready: stage 2 is its own control, disabled with a visible reason until a
/// transfer adapter exists, and the label says where the bytes came from.
#[test]
fn a_ready_export_offers_a_separate_handoff_and_discard() {
    let listed = list(vec![summary(1, "Andromeda", false)]);
    for (source, line) in [
        (HEAD, "A portable copy of the latest save is ready."),
        (
            PREDECESSOR,
            "Portable copy ready. Recovered predecessor; stored head and Continue unchanged.",
        ),
        (
            ExportSource::ActiveWorkshop {
                continue_ready: false,
            },
            "Portable copy of the open Workshop ready. Portable export; Workshop not saved for Continue.",
        ),
        (
            ExportSource::ActiveWorkshop {
                continue_ready: true,
            },
            "A portable copy of the open Workshop, as saved for Continue, is ready.",
        ),
        (
            ExportSource::ActivePack {
                hash: nyon::workshop::CatalogHash([7; 32]),
            },
            "A portable copy of the open Workshop's content pack is ready.",
        ),
    ] {
        let status = LibrarySlotsStatus::ExportReady { source };
        let built = model(LibraryUiContext {
            slots: Some(&listed),
            selected_slot: Some(SlotId(1)),
            status,
            ..LibraryUiContext::default()
        });
        if let Some(qualifier) = source.qualifier() {
            assert!(line.contains(qualifier), "{line}");
        }
        assert_eq!(request_line(&built), line);
        // Not a failure: a status, not an alert.
        assert!(!built.request.decision_pending());
        assert_eq!(
            built
                .semantics
                .node("library.request.status")
                .expect("request status node")
                .role,
            SemanticRole::Status
        );
        assert!(
            !built
                .controls()
                .any(|control| control.action_id.as_str() == "library.request.retry"),
            "the handoff must not reuse the identifier Export previous held"
        );
        let handoff = control(&built, "library.request.handoff");
        assert_eq!(handoff.label, "Save copy");
        assert_eq!(
            handoff.disabled_reason,
            Some(LibraryDisabledReason::TransferUnavailable)
        );
        let discard = control(&built, "library.request.cancel");
        assert_eq!(discard.label, "Discard");
        assert_eq!(discard.intent(), Some(&LibraryUiIntent::CancelSlotRequest));
        // Present in the focus order, between the header and the rows.
        let order: Vec<&str> = built
            .focus_order()
            .iter()
            .map(SemanticActionId::as_str)
            .collect();
        assert_eq!(
            &order[..4],
            [
                "library.close",
                "library.refresh",
                "library.request.handoff",
                "library.request.cancel"
            ]
        );

        // The handoff reads adapter presence, and only that: Import galaxy's
        // route is a different flag.
        let live = model(LibraryUiContext {
            handoff_available: true,
            ..LibraryUiContext {
                slots: Some(&listed),
                selected_slot: Some(SlotId(1)),
                status,
                ..LibraryUiContext::default()
            }
        });
        assert_eq!(
            control(&live, "library.request.handoff").intent(),
            Some(&LibraryUiIntent::HandOffExport)
        );
        let strip_only = model(LibraryUiContext {
            archive_import_available: true,
            ..LibraryUiContext {
                slots: Some(&listed),
                selected_slot: Some(SlotId(1)),
                status,
                ..LibraryUiContext::default()
            }
        });
        assert_eq!(
            control(&strip_only, "library.request.handoff").disabled_reason,
            Some(LibraryDisabledReason::TransferUnavailable)
        );
    }
}

/// A conflicted export is the same refreshable conflict as Open.
#[test]
fn a_conflicted_export_offers_refresh() {
    let listed = list(vec![summary(1, "Andromeda", false)]);
    for (code, label, line) in [
        (
            ClientDiagnosticCode::StaleSave,
            "Refresh",
            "That save changed. Refresh, then try again.",
        ),
        (
            ClientDiagnosticCode::Store,
            "Retry",
            "Preparing a portable copy failed. Retry or cancel.",
        ),
    ] {
        let built = model(LibraryUiContext {
            slots: Some(&listed),
            status: LibrarySlotsStatus::Failed {
                kind: SlotRequestKind::Export,
                slot: Some(SlotId(1)),
                code,
            },
            ..LibraryUiContext::default()
        });
        assert_eq!(control(&built, "library.request.retry").label, label);
        assert_eq!(request_line(&built), line);
    }
    let working = model(LibraryUiContext {
        slots: Some(&listed),
        status: LibrarySlotsStatus::Working {
            kind: SlotRequestKind::Export,
            slot: Some(SlotId(1)),
        },
        ..LibraryUiContext::default()
    });
    assert_eq!(
        request_line(&working),
        "Preparing a portable copy of a saved galaxy."
    );
}

// ---------------------------------------------------------------------------
// Route-design task 11: stage 2 of row Export
// ---------------------------------------------------------------------------

fn handoff_statuses() -> [LibrarySlotsStatus; 3] {
    [
        LibrarySlotsStatus::Working {
            kind: SlotRequestKind::HandOff,
            slot: Some(SlotId(1)),
        },
        LibrarySlotsStatus::HandOffFailed {
            source: ExportSource::Head {
                slot: SlotId(1),
                generation: SaveGeneration(11),
            },
            code: TransferFailureCode::Platform,
        },
        LibrarySlotsStatus::ExportHandedOff {
            source: ExportSource::Head {
                slot: SlotId(1),
                generation: SaveGeneration(11),
            },
            outcome: HandoffOutcome::DownloadStarted,
        },
    ]
}

/// An adapter enables the handoff. Over prepared bytes the transfer strip is
/// held by the lane, whatever the adapter; Import galaxy additionally waits
/// on its own route, which an adapter does not supply.
#[test]
fn an_adapter_enables_the_handoff_and_the_lane_holds_the_strip() {
    let listed = list(vec![summary(1, "Andromeda", false)]);
    let built = model(LibraryUiContext {
        slots: Some(&listed),
        selected_slot: Some(SlotId(1)),
        workshop_active: true,
        handoff_available: true,
        status: LibrarySlotsStatus::ExportReady { source: HEAD },
        ..LibraryUiContext::default()
    });
    assert!(control(&built, "library.request.handoff").enabled);
    for id in [
        "library.transfer.import-archive",
        "library.transfer.import-pack",
        "library.transfer.export-active-archive",
        "library.transfer.export-active-pack",
    ] {
        assert_eq!(
            control(&built, id).disabled_reason,
            Some(LibraryDisabledReason::ExportWaiting),
            "{id}"
        );
    }
    let idle = model(LibraryUiContext {
        status: LibrarySlotsStatus::Idle,
        ..LibraryUiContext {
            slots: Some(&listed),
            workshop_active: true,
            handoff_available: true,
            ..LibraryUiContext::default()
        }
    });
    assert_eq!(
        control(&idle, "library.transfer.import-archive").disabled_reason,
        Some(LibraryDisabledReason::TransferUnavailable)
    );
    for id in [
        "library.transfer.import-pack",
        "library.transfer.export-active-archive",
        "library.transfer.export-active-pack",
    ] {
        assert!(control(&idle, id).enabled, "{id}");
    }
}

/// A failed handoff is retried on the handoff's own identifier, never on the
/// one Export previous used, and keeps Discard.
#[test]
fn a_failed_handoff_retries_on_the_handoff_control() {
    let listed = list(vec![summary(1, "Andromeda", false)]);
    for (source, line) in [
        (
            HEAD,
            "Handing off the portable copy failed. Save copy again or discard it.",
        ),
        (
            PREDECESSOR,
            "Handing off the portable copy failed. Save copy again or discard it. Recovered predecessor; stored head and Continue unchanged.",
        ),
        (
            ExportSource::ActiveWorkshop {
                continue_ready: false,
            },
            "Handing off the portable copy failed. Save copy again or discard it. Portable export; Workshop not saved for Continue.",
        ),
        (
            ExportSource::ActivePack {
                hash: nyon::workshop::CatalogHash([7; 32]),
            },
            "Handing off the portable copy failed. Save copy again or discard it.",
        ),
    ] {
        for available in [false, true] {
            let built = model(LibraryUiContext {
                slots: Some(&listed),
                selected_slot: Some(SlotId(1)),
                handoff_available: available,
                status: LibrarySlotsStatus::HandOffFailed {
                    source,
                    code: TransferFailureCode::Platform,
                },
                ..LibraryUiContext::default()
            });
            assert!(
                !built
                    .controls()
                    .any(|control| control.action_id.as_str() == "library.request.retry"),
                "a failed handoff must not offer the generic Retry"
            );
            let handoff = control(&built, "library.request.handoff");
            assert_eq!(handoff.label, "Save copy");
            if available {
                assert_eq!(handoff.intent(), Some(&LibraryUiIntent::HandOffExport));
            } else {
                assert_eq!(
                    handoff.disabled_reason,
                    Some(LibraryDisabledReason::TransferUnavailable)
                );
            }
            let discard = control(&built, "library.request.cancel");
            assert_eq!(discard.label, "Discard");
            assert_eq!(discard.intent(), Some(&LibraryUiIntent::CancelSlotRequest));
            assert_eq!(request_line(&built), line);
            assert_eq!(
                built.request.failure_code,
                Some(ClientDiagnosticCode::Transfer)
            );
            assert!(built.request.decision_pending());
            assert_eq!(
                built
                    .semantics
                    .node("library.request.status")
                    .expect("request status node")
                    .role,
                SemanticRole::Alert
            );
        }
    }
}

/// The receipt says exactly what the adapter reported, and only a durable
/// save says "saved". It offers Done, and nothing that could send again.
#[test]
fn a_handed_off_copy_reports_its_outcome_and_offers_done() {
    let listed = list(vec![summary(1, "Andromeda", false)]);
    for (outcome, source, line) in [
        (HandoffOutcome::DownloadStarted, HEAD, "Download started."),
        (
            HandoffOutcome::HandedToSystem,
            HEAD,
            "Copy handed to the operating system.",
        ),
        (HandoffOutcome::Written, HEAD, "Copy written."),
        (HandoffOutcome::DurablySaved, HEAD, "Copy saved."),
        (
            HandoffOutcome::DownloadStarted,
            PREDECESSOR,
            "Download started. Recovered predecessor; stored head and Continue unchanged.",
        ),
        (
            HandoffOutcome::DownloadStarted,
            ExportSource::ActiveWorkshop {
                continue_ready: false,
            },
            "Download started. Portable export; Workshop not saved for Continue.",
        ),
        (
            HandoffOutcome::Written,
            ExportSource::ActivePack {
                hash: nyon::workshop::CatalogHash([7; 32]),
            },
            "Copy written.",
        ),
    ] {
        let built = model(LibraryUiContext {
            slots: Some(&listed),
            selected_slot: Some(SlotId(1)),
            handoff_available: true,
            status: LibrarySlotsStatus::ExportHandedOff { source, outcome },
            ..LibraryUiContext::default()
        });
        assert_eq!(request_line(&built), line, "{outcome:?}");
        // Only the outcome's own words may claim a save; §10's qualifier
        // ("not saved for Continue") is a negation and is checked apart.
        let claim = line
            .strip_suffix(&format!(" {}.", source.qualifier().unwrap_or("")))
            .unwrap_or(line);
        assert_eq!(
            claim.to_ascii_lowercase().contains("saved"),
            outcome == HandoffOutcome::DurablySaved,
            "{outcome:?}"
        );
        for id in ["library.request.retry", "library.request.handoff"] {
            assert!(
                !built
                    .controls()
                    .any(|control| control.action_id.as_str() == id),
                "{id}"
            );
        }
        let done = control(&built, "library.request.cancel");
        assert_eq!(done.label, "Done");
        assert_eq!(done.intent(), Some(&LibraryUiIntent::CancelSlotRequest));
        assert!(!built.request.decision_pending());
        assert!(!built.request.in_flight());
        assert_eq!(
            built
                .semantics
                .node("library.request.status")
                .expect("request status node")
                .role,
            SemanticRole::Status
        );
    }
}

/// In flight, the handoff offers only a way to stop waiting, which keeps the
/// prepared copy.
#[test]
fn a_handoff_in_flight_offers_stop_waiting() {
    let [working, ..] = handoff_statuses();
    let built = model(LibraryUiContext {
        handoff_available: true,
        status: working,
        ..LibraryUiContext::default()
    });
    assert_eq!(
        request_line(&built),
        "Handing the portable copy to the system."
    );
    assert!(built.request.in_flight());
    for id in ["library.request.retry", "library.request.handoff"] {
        assert!(
            !built
                .controls()
                .any(|control| control.action_id.as_str() == id),
            "{id}"
        );
    }
    let stop = control(&built, "library.request.cancel");
    assert_eq!(stop.label, "Stop waiting");
    assert_eq!(
        stop.description,
        "Stop waiting for the system. The prepared copy is kept."
    );
    assert_eq!(stop.intent(), Some(&LibraryUiIntent::CancelSlotRequest));
}

/// Every handoff state occupies the one lane, with a reason that says why.
#[test]
fn every_handoff_state_occupies_the_lane() {
    let listed = list(vec![summary(1, "Andromeda", false)]);
    for (status, reason) in handoff_statuses().into_iter().zip([
        LibraryDisabledReason::RequestInFlight,
        LibraryDisabledReason::ExportWaiting,
        LibraryDisabledReason::CopyFinished,
    ]) {
        let built = model(LibraryUiContext {
            slots: Some(&listed),
            selected_slot: Some(SlotId(1)),
            handoff_available: true,
            status,
            ..LibraryUiContext::default()
        });
        for id in [
            "library.action.export",
            "library.action.open",
            "library.action.rename",
            "library.refresh",
        ] {
            assert_eq!(
                control(&built, id).disabled_reason,
                Some(reason),
                "{status:?} {id}"
            );
        }
    }
    assert_eq!(
        LibraryDisabledReason::CopyFinished.message(),
        "Dismiss the finished copy first."
    );
}

/// The transfer strip's lane states on the request strip (task 11): what the
/// line says, which controls it offers, and whether it is a decision.
#[test]
fn transfer_route_states_offer_their_own_strip_controls() {
    let hash = nyon::workshop::CatalogHash([9; 32]);
    let cases = [
        (
            LibrarySlotsStatus::Working {
                kind: SlotRequestKind::ExportWorkshop,
                slot: None,
            },
            "Preparing a portable copy of the open Workshop.",
            None,
            Some("Cancel"),
        ),
        (
            LibrarySlotsStatus::Failed {
                kind: SlotRequestKind::ExportWorkshop,
                slot: None,
                code: ClientDiagnosticCode::Archive,
            },
            "Preparing a portable copy of the open Workshop failed. Retry or cancel.",
            Some("Retry"),
            Some("Cancel"),
        ),
        (
            LibrarySlotsStatus::Working {
                kind: SlotRequestKind::ChoosePack,
                slot: None,
            },
            "Waiting for a portable file to be chosen.",
            None,
            Some("Cancel"),
        ),
        (
            LibrarySlotsStatus::Failed {
                kind: SlotRequestKind::ChoosePack,
                slot: None,
                code: ClientDiagnosticCode::Transfer,
            },
            "Choosing a portable file failed. Choose again or cancel.",
            Some("Choose again"),
            Some("Cancel"),
        ),
        (
            LibrarySlotsStatus::Failed {
                kind: SlotRequestKind::ChoosePack,
                slot: None,
                code: ClientDiagnosticCode::Catalog,
            },
            "That file cannot be imported. Choose another or cancel.",
            Some("Choose again"),
            Some("Cancel"),
        ),
        (
            LibrarySlotsStatus::Working {
                kind: SlotRequestKind::StorePack,
                slot: None,
            },
            "Storing the imported content pack.",
            None,
            Some("Cancel"),
        ),
        (
            LibrarySlotsStatus::Failed {
                kind: SlotRequestKind::StorePack,
                slot: None,
                code: ClientDiagnosticCode::Store,
            },
            "Storing the content pack failed. Retry or cancel; nothing stored is removed.",
            Some("Retry"),
            Some("Cancel"),
        ),
        (
            LibrarySlotsStatus::PackStored { hash },
            "Content pack stored in the Workshop store.",
            None,
            Some("Done"),
        ),
    ];
    for (status, line, retry, cancel) in cases {
        let built = model(LibraryUiContext {
            status,
            workshop_active: true,
            handoff_available: true,
            ..LibraryUiContext::default()
        });
        assert_eq!(request_line(&built), line, "{status:?}");
        assert_eq!(
            built
                .request
                .retry_control
                .as_ref()
                .map(|control| control.label.as_str()),
            retry,
            "{status:?}"
        );
        if let Some(control) = &built.request.retry_control {
            assert_eq!(
                control.intent(),
                Some(&LibraryUiIntent::RetrySlotRequest),
                "{status:?}"
            );
        }
        assert_eq!(
            built
                .request
                .cancel_control
                .as_ref()
                .map(|control| control.label.as_str()),
            cancel,
            "{status:?}"
        );
        assert_eq!(
            built.request.decision_pending(),
            matches!(status, LibrarySlotsStatus::Failed { .. }),
            "{status:?}"
        );
        // Nothing here claims a save of anything but the pack store.
        assert!(!line.contains("Copy saved"), "{status:?}");
    }
}
