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
    app::client_runtime::{ClientDiagnosticCode, LibrarySlotsStatus, SlotRequestKind},
    ui::{
        accessibility::{InputModality, SemanticActionId, SemanticRole},
        library::{
            LibraryContent, LibraryControl, LibraryDisabledReason, LibrarySection,
            LibraryUiContext, LibraryUiIntent, LibraryUiModel,
        },
    },
    workshop::store::{SaveGeneration, SlotId, SlotList, SlotName, SlotSummary},
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

/// The only intent that leaves the Library is Close.
///
/// §6 forbids collapsing a Library failure into `RecoverableError`. There is no
/// recovery intent to emit, and this pins that there is no other exit either.
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
        library_client_available: true,
        transfer_available: true,
        workshop_active: true,
        selected_slot: Some(SlotId(1)),
        ..LibraryUiContext::default()
    });
    let leaving = built
        .controls()
        .filter(|control| matches!(control.intent(), Some(LibraryUiIntent::Close)))
        .count();
    assert_eq!(leaving, 1, "Close is the only route off the Library");
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
    for (client, transfer, workshop, selected) in [
        (false, false, false, None),
        (true, true, true, Some(SlotId(1))),
        (false, true, false, Some(SlotId(2))),
    ] {
        let listed = list(vec![
            summary(1, "Andromeda", false),
            summary(2, "Bode", true),
        ]);
        let built = model(LibraryUiContext {
            slots: Some(&listed),
            selected_slot: selected,
            library_client_available: client,
            transfer_available: transfer,
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

/// Client-dependent controls are present and disabled, never missing.
///
/// This is baseline Finding 5: a control that exists logically but cannot be
/// reached. Presence plus a reason is the contract; absence is the defect.
#[test]
fn client_and_transfer_controls_are_present_and_disabled_with_a_reason() {
    let listed = list(vec![summary(1, "Andromeda", false)]);
    let built = model(LibraryUiContext {
        slots: Some(&listed),
        selected_slot: Some(SlotId(1)),
        workshop_active: true,
        ..LibraryUiContext::default()
    });
    for id in [
        "library.action.open",
        "library.action.use-for-continue",
        "library.action.export",
    ] {
        let found = control(&built, id);
        assert!(!found.enabled, "{id}");
        assert_eq!(
            found.disabled_reason,
            Some(LibraryDisabledReason::LibraryClientUnavailable),
            "{id}"
        );
    }
    for id in [
        "library.transfer.import-archive",
        "library.transfer.import-pack",
        "library.transfer.export-active-archive",
        "library.transfer.export-active-pack",
    ] {
        let found = control(&built, id);
        assert!(!found.enabled, "{id}");
        assert_eq!(
            found.disabled_reason,
            Some(LibraryDisabledReason::TransferUnavailable),
            "{id}"
        );
    }
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

/// An archived row swaps Archive for Unarchive and blocks Open and Continue.
#[test]
fn an_archived_row_offers_unarchive_and_refuses_open_and_continue() {
    let listed = list(vec![summary(9, "Bode", true)]);
    let built = model(LibraryUiContext {
        slots: Some(&listed),
        selected_slot: Some(SlotId(9)),
        library_client_available: true,
        ..LibraryUiContext::default()
    });
    let archive = &built.actions.archive;
    assert_eq!(
        archive.action_id.as_str(),
        "library.action.archive",
        "the identifier is stable across the toggle; only the label and intent flip"
    );
    assert_eq!(archive.label, "Unarchive");
    assert!(archive.enabled);
    assert_eq!(
        archive.intent(),
        Some(&LibraryUiIntent::UnarchiveSlot { slot: SlotId(9) })
    );
    assert_eq!(
        built.actions.open.disabled_reason,
        Some(LibraryDisabledReason::ArchivedSlot)
    );
    assert_eq!(
        built.actions.use_for_continue.disabled_reason,
        Some(LibraryDisabledReason::ArchivedSlot)
    );
}

/// Row state outranks a capability deferral in the disabled reason.
///
/// Documented precedence: no subject, then row facts, then lane occupancy, then
/// capability. An archived row with no Library client reports `ArchivedSlot`,
/// the thing the user can act on now.
#[test]
fn row_state_outranks_capability_in_the_reported_reason() {
    let listed = list(vec![summary(9, "Bode", true)]);
    let built = model(LibraryUiContext {
        slots: Some(&listed),
        selected_slot: Some(SlotId(9)),
        library_client_available: false,
        ..LibraryUiContext::default()
    });
    assert_eq!(
        built.actions.open.disabled_reason,
        Some(LibraryDisabledReason::ArchivedSlot)
    );
    assert_ne!(
        built.actions.open.disabled_reason,
        Some(LibraryDisabledReason::LibraryClientUnavailable)
    );
}

/// With no selection every action is present and disabled with `NoSelection`.
#[test]
fn with_no_selection_every_action_is_present_and_disabled() {
    let listed = list(vec![summary(1, "Andromeda", false)]);
    let built = model(LibraryUiContext {
        slots: Some(&listed),
        library_client_available: true,
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
        library_client_available: true,
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
/// Archive would be refused outright. Open, Use for Continue and Export take
/// the exact-catalog client instead and are not gated by this lane.
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
            library_client_available: true,
            ..LibraryUiContext::default()
        });
        for id in [
            "library.refresh",
            "library.action.rename",
            "library.action.archive",
        ] {
            assert_eq!(control(&built, id).disabled_reason, Some(reason), "{id}");
        }
        for id in [
            "library.action.open",
            "library.action.use-for-continue",
            "library.action.export",
        ] {
            assert!(control(&built, id).enabled, "{id}");
        }
    }
}

/// Exporting the active Workshop needs one to be active.
#[test]
fn exporting_the_active_workshop_requires_an_active_workshop() {
    let built = model(LibraryUiContext {
        transfer_available: true,
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
        library_client_available: true,
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
        library_client_available: true,
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
        library_client_available: true,
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
            library_client_available: true,
            transfer_available: true,
            workshop_active: true,
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
        library_client_available: true,
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
