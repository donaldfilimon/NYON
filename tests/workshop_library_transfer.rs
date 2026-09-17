//! Stage 2 of row Export through the transfer protocol: route-design task 11.
//!
//! Addendum §7 makes export two-stage: validated bytes first reach Ready, and
//! only a second direct activation hands them to the platform. §1 and §9
//! bound what a finished handoff may claim. The route design keeps the
//! handoff disabled with a visible reason when no adapter is installed, and
//! closes the loop with a test adapter. Each test pins one of those rules at
//! the runtime seam, with `ScriptedTransfer` standing in for a platform.

use std::{cell::RefCell, rc::Rc, time::Duration};

use nyon::{
    app::{
        AppCore,
        client_runtime::{
            ClientDiagnosticCode, ClientRuntime, ClientRuntimeError, ClientScreen, ExportSource,
            LibrarySlotsStatus, SlotRequestKind,
        },
        transfer::{
            HandoffOutcome, ScriptedHandoff, ScriptedReply, ScriptedTransfer, SuggestedName,
            TransferFailureCode, TransferKind, TransferRequest,
        },
    },
    preferences::store::MemoryPreferencesStore,
    scenario::{ScenarioDraft, store::MemoryScenarioStore},
    workshop::{
        WorkshopHistory, decode_catalog_pack, encode_archive,
        store::{
            MemoryWorkshopStore, SaveGeneration, SlotId, SlotList, SlotName, StoreJobId,
            StoreJobState, WorkshopStore, WorkshopStoreError, WorkshopStoreRequest,
            WorkshopStoreResult,
        },
    },
};

#[derive(Clone, Default)]
struct Shared {
    inner: Rc<RefCell<MemoryWorkshopStore>>,
}

impl WorkshopStore for Shared {
    fn start(&mut self, request: WorkshopStoreRequest) -> Result<StoreJobId, WorkshopStoreError> {
        self.inner.borrow_mut().start(request)
    }

    fn abandon(&mut self, job: StoreJobId) -> bool {
        self.inner.borrow_mut().abandon(job)
    }

    fn poll(&mut self, job: StoreJobId) -> StoreJobState {
        self.inner.borrow_mut().poll(job)
    }
}

type Runtime = ClientRuntime<MemoryScenarioStore, MemoryPreferencesStore, Shared>;

fn complete(store: &Shared, request: WorkshopStoreRequest) -> WorkshopStoreResult {
    let mut inner = store.inner.borrow_mut();
    let job = inner.start(request).unwrap();
    match inner.poll(job) {
        StoreJobState::Complete(Ok(result)) => result,
        result => panic!("unexpected Workshop store result: {result:?}"),
    }
}

fn valid_archive(seed: u64) -> Box<[u8]> {
    let catalog =
        decode_catalog_pack(include_bytes!("../assets/workshop/core-pack-v1.json")).unwrap();
    encode_archive(&WorkshopHistory::from_seed_u64(catalog, seed))
        .unwrap()
        .bytes
        .into_boxed_slice()
}

fn create(store: &Shared, name: &str, seed: u64) -> (SlotId, SaveGeneration) {
    let WorkshopStoreResult::SlotCreated { slot, generation } = complete(
        store,
        WorkshopStoreRequest::CreateSlot {
            name: SlotName::new(name).unwrap(),
            archive: valid_archive(seed),
        },
    ) else {
        panic!("unexpected create result");
    };
    (slot, generation)
}

fn stored_list(store: &Shared) -> SlotList {
    let WorkshopStoreResult::Slots(list) = complete(store, WorkshopStoreRequest::ListSlots) else {
        panic!("ListSlots answered with the wrong shape");
    };
    list
}

fn settle(runtime: &mut Runtime) {
    for _ in 0..32 {
        if !matches!(
            runtime.library_slots_status(),
            LibrarySlotsStatus::Working { .. }
        ) {
            return;
        }
        runtime.update(Duration::ZERO);
    }
    panic!("the Library lane exceeded its bounded test polls");
}

/// A runtime on the Library holding a prepared copy of "Second Forge".
struct Fixture {
    store: Shared,
    runtime: Runtime,
    slot: SlotId,
    generation: SaveGeneration,
    other: SlotId,
    before: SlotList,
}

fn ready() -> Fixture {
    let store = Shared::default();
    let (other, _) = create(&store, "First Forge", 11);
    let (slot, generation) = create(&store, "Second Forge", 12);
    let before = stored_list(&store);
    let mut runtime = ClientRuntime::new(
        AppCore::new_with_preferences(
            ScenarioDraft::factory_default().validated().unwrap(),
            MemoryScenarioStore::default(),
            MemoryPreferencesStore::default(),
        ),
        store.clone(),
    );
    runtime.open_library().unwrap();
    settle(&mut runtime);
    runtime.export_library_slot(slot, generation).unwrap();
    settle(&mut runtime);
    assert_eq!(
        runtime.library_slots_status(),
        ready_status(slot, generation)
    );
    Fixture {
        store,
        runtime,
        slot,
        generation,
        other,
        before,
    }
}

const fn ready_status(slot: SlotId, generation: SaveGeneration) -> LibrarySlotsStatus {
    LibrarySlotsStatus::ExportReady {
        slot,
        generation,
        source: ExportSource::Head,
    }
}

fn install(fixture: &mut Fixture) -> ScriptedTransfer {
    let adapter = ScriptedTransfer::default();
    fixture
        .runtime
        .install_transfer_adapter(Box::new(adapter.clone()))
        .unwrap();
    adapter
}

fn expected_handoff(bytes: Box<[u8]>) -> ScriptedHandoff {
    ScriptedHandoff {
        kind: TransferKind::WorkshopArchive,
        suggested_name: SuggestedName::workshop_archive(&SlotName::new("Second Forge").unwrap()),
        bytes,
    }
}

/// Nothing a handoff does may reach the store, the session or the list.
fn assert_store_untouched(fixture: &Fixture) {
    assert_eq!(stored_list(&fixture.store), fixture.before);
    assert_eq!(fixture.runtime.screen(), ClientScreen::Library);
    assert_eq!(fixture.runtime.library_slots(), Some(&fixture.before));
    assert!(fixture.runtime.workshop_snapshot().is_none());
}

fn transfer_diagnostics(runtime: &Runtime) -> usize {
    runtime
        .diagnostics()
        .filter(|diagnostic| diagnostic.code == ClientDiagnosticCode::Transfer)
        .count()
}

#[test]
fn without_an_adapter_the_handoff_is_refused_and_the_bytes_stay_ready() {
    let mut fixture = ready();
    assert!(!fixture.runtime.transfer_available());
    assert_eq!(
        fixture.runtime.hand_off_library_export(),
        Err(ClientRuntimeError::TransferUnavailable)
    );
    assert_eq!(
        fixture.runtime.library_slots_status(),
        ready_status(fixture.slot, fixture.generation)
    );
    assert_eq!(
        fixture
            .runtime
            .prepared_slot_export()
            .map(|export| &export.archive),
        Some(&valid_archive(12))
    );
    assert_store_untouched(&fixture);
}

#[test]
fn a_handoff_delivers_the_exact_prepared_bytes_and_says_only_what_happened() {
    let mut fixture = ready();
    let adapter = install(&mut fixture);
    assert!(fixture.runtime.transfer_available());
    adapter.set_pending_polls(2);

    fixture.runtime.hand_off_library_export().unwrap();
    assert_eq!(
        fixture.runtime.library_slots_status(),
        LibrarySlotsStatus::Working {
            kind: SlotRequestKind::HandOff,
            slot: Some(fixture.slot),
        }
    );
    // The request carries §7's kind, sanitized name and the validated bytes.
    assert_eq!(
        adapter.started(),
        vec![TransferRequest::HandOffExport {
            kind: TransferKind::WorkshopArchive,
            suggested_name: SuggestedName::workshop_archive(
                &SlotName::new("Second Forge").unwrap()
            ),
            bytes: valid_archive(12),
        }]
    );
    // In flight, the lane stays occupied and a second handoff cannot start.
    assert_eq!(
        fixture.runtime.hand_off_library_export(),
        Err(ClientRuntimeError::RouteUnavailable)
    );
    assert_eq!(
        fixture
            .runtime
            .rename_library_slot(fixture.other, SlotName::new("Busy").unwrap()),
        Err(ClientRuntimeError::LibraryRequestActive)
    );
    assert_eq!(adapter.started().len(), 1);
    // Closing the Library does not stop the job from being polled.
    fixture.runtime.close_library();
    fixture.runtime.update(Duration::ZERO);
    fixture.runtime.open_library().unwrap();
    settle(&mut fixture.runtime);

    assert_eq!(
        fixture.runtime.library_slots_status(),
        LibrarySlotsStatus::ExportHandedOff {
            slot: fixture.slot,
            generation: fixture.generation,
            source: ExportSource::Head,
            outcome: HandoffOutcome::DownloadStarted,
        }
    );
    assert_eq!(
        adapter.handed_off(),
        vec![expected_handoff(valid_archive(12))]
    );
    assert!(
        fixture.runtime.prepared_slot_export().is_none(),
        "handed-off bytes are released"
    );
    assert_eq!(transfer_diagnostics(&fixture.runtime), 0);
    assert_store_untouched(&fixture);

    // The receipt keeps the lane until it is dismissed, and nothing re-sends.
    for _ in 0..4 {
        fixture.runtime.update(Duration::ZERO);
    }
    assert_eq!(adapter.started().len(), 1);
    assert_eq!(
        fixture.runtime.hand_off_library_export(),
        Err(ClientRuntimeError::RouteUnavailable)
    );
    assert_eq!(
        fixture.runtime.retry_library_slot_request(),
        Err(ClientRuntimeError::RouteUnavailable)
    );
    assert!(matches!(
        fixture.runtime.library_slots_status(),
        LibrarySlotsStatus::ExportHandedOff { .. }
    ));
    fixture.runtime.cancel_library_slot_request().unwrap();
    assert_eq!(
        fixture.runtime.library_slots_status(),
        LibrarySlotsStatus::Idle
    );
    assert_store_untouched(&fixture);
    fixture
        .runtime
        .rename_library_slot(fixture.other, SlotName::new("Renamed Forge").unwrap())
        .unwrap();
}

#[test]
fn every_outcome_is_reported_as_itself() {
    for outcome in [
        HandoffOutcome::DownloadStarted,
        HandoffOutcome::HandedToSystem,
        HandoffOutcome::Written,
        HandoffOutcome::DurablySaved,
    ] {
        let mut fixture = ready();
        let adapter = install(&mut fixture);
        adapter.push_reply(ScriptedReply::Complete(outcome));
        fixture.runtime.hand_off_library_export().unwrap();
        settle(&mut fixture.runtime);
        assert_eq!(
            fixture.runtime.library_slots_status(),
            LibrarySlotsStatus::ExportHandedOff {
                slot: fixture.slot,
                generation: fixture.generation,
                source: ExportSource::Head,
                outcome,
            }
        );
    }
}

#[test]
fn a_recovered_predecessor_keeps_its_label_through_the_handoff() {
    let store = Shared::default();
    let (slot, valid) = create(&store, "Second Forge", 12);
    let WorkshopStoreResult::SlotCommitted {
        generation: corrupt,
        ..
    } = complete(
        &store,
        WorkshopStoreRequest::CommitSlot {
            slot,
            expected_generation: valid,
            archive: Box::from(&b"{}"[..]),
        },
    )
    else {
        panic!("unexpected commit result");
    };
    let before = stored_list(&store);
    let mut runtime = ClientRuntime::new(
        AppCore::new_with_preferences(
            ScenarioDraft::factory_default().validated().unwrap(),
            MemoryScenarioStore::default(),
            MemoryPreferencesStore::default(),
        ),
        store.clone(),
    );
    let adapter = ScriptedTransfer::default();
    runtime
        .install_transfer_adapter(Box::new(adapter.clone()))
        .unwrap();
    runtime.open_library().unwrap();
    settle(&mut runtime);
    runtime.export_library_slot(slot, corrupt).unwrap();
    settle(&mut runtime);
    // An offer is not Ready: nothing may be handed off from it.
    assert_eq!(
        runtime.hand_off_library_export(),
        Err(ClientRuntimeError::RouteUnavailable)
    );
    assert_eq!(
        runtime.library_slots_status(),
        LibrarySlotsStatus::ExportRecoveryOffered { slot }
    );
    runtime.accept_library_export_recovery().unwrap();
    runtime.hand_off_library_export().unwrap();
    settle(&mut runtime);
    assert_eq!(
        runtime.library_slots_status(),
        LibrarySlotsStatus::ExportHandedOff {
            slot,
            generation: valid,
            source: ExportSource::RecoveredPredecessor,
            outcome: HandoffOutcome::DownloadStarted,
        }
    );
    assert_eq!(
        adapter.handed_off(),
        vec![expected_handoff(valid_archive(12))]
    );
    assert_eq!(stored_list(&store), before);
    assert_eq!(stored_list(&store).selected_continue, None);
}

#[test]
fn a_handoff_is_refused_with_nothing_ready() {
    let mut fixture = ready();
    fixture.runtime.cancel_library_slot_request().unwrap();
    let adapter = install(&mut fixture);
    assert_eq!(
        fixture.runtime.hand_off_library_export(),
        Err(ClientRuntimeError::RouteUnavailable)
    );
    assert_eq!(
        fixture.runtime.library_slots_status(),
        LibrarySlotsStatus::Idle
    );
    assert!(adapter.started().is_empty());
}

/// A refused start and a failed job both keep the bytes; the handoff control
/// is the retry, and the generic Retry cannot reach it.
#[test]
fn a_failed_handoff_keeps_the_bytes_and_is_retried_only_by_handing_off() {
    for refuse in [true, false] {
        let mut fixture = ready();
        let adapter = install(&mut fixture);
        adapter.push_reply(if refuse {
            ScriptedReply::Refuse(TransferFailureCode::Platform)
        } else {
            ScriptedReply::Fail(TransferFailureCode::Platform)
        });
        let started = fixture.runtime.hand_off_library_export();
        if refuse {
            assert_eq!(
                started,
                Err(ClientRuntimeError::Transfer(TransferFailureCode::Platform))
            );
        } else {
            started.unwrap();
            settle(&mut fixture.runtime);
        }
        let failed = LibrarySlotsStatus::HandOffFailed {
            slot: fixture.slot,
            generation: fixture.generation,
            source: ExportSource::Head,
            code: TransferFailureCode::Platform,
        };
        assert_eq!(fixture.runtime.library_slots_status(), failed, "{refuse}");
        assert_eq!(
            fixture
                .runtime
                .prepared_slot_export()
                .map(|export| &export.archive),
            Some(&valid_archive(12)),
            "{refuse}"
        );
        assert_eq!(transfer_diagnostics(&fixture.runtime), 1, "{refuse}");
        assert!(adapter.handed_off().is_empty());
        assert_eq!(
            fixture.runtime.retry_library_slot_request(),
            Err(ClientRuntimeError::RouteUnavailable)
        );
        assert_eq!(fixture.runtime.library_slots_status(), failed);
        assert_eq!(
            fixture
                .runtime
                .rename_library_slot(fixture.other, SlotName::new("Busy").unwrap()),
            Err(ClientRuntimeError::LibraryRequestActive)
        );

        fixture.runtime.hand_off_library_export().unwrap();
        settle(&mut fixture.runtime);
        assert!(matches!(
            fixture.runtime.library_slots_status(),
            LibrarySlotsStatus::ExportHandedOff { .. }
        ));
        assert_eq!(
            adapter.handed_off(),
            vec![expected_handoff(valid_archive(12))]
        );
        assert_store_untouched(&fixture);
    }
}

#[test]
fn discarding_a_failed_handoff_releases_the_bytes() {
    let mut fixture = ready();
    let adapter = install(&mut fixture);
    adapter.push_reply(ScriptedReply::Fail(TransferFailureCode::TooLarge));
    fixture.runtime.hand_off_library_export().unwrap();
    settle(&mut fixture.runtime);
    fixture.runtime.cancel_library_slot_request().unwrap();
    assert_eq!(
        fixture.runtime.library_slots_status(),
        LibrarySlotsStatus::Idle
    );
    assert!(fixture.runtime.prepared_slot_export().is_none());
    assert_store_untouched(&fixture);
}

/// Dismissing the picker is ordinary: back to Ready, no diagnostic.
#[test]
fn a_cancelled_handoff_returns_to_ready_without_a_diagnostic() {
    let mut fixture = ready();
    let adapter = install(&mut fixture);
    adapter.push_reply(ScriptedReply::Cancel);
    fixture.runtime.hand_off_library_export().unwrap();
    settle(&mut fixture.runtime);
    assert_eq!(
        fixture.runtime.library_slots_status(),
        ready_status(fixture.slot, fixture.generation)
    );
    assert!(fixture.runtime.prepared_slot_export().is_some());
    assert_eq!(transfer_diagnostics(&fixture.runtime), 0);
    assert!(adapter.handed_off().is_empty());
    fixture.runtime.hand_off_library_export().unwrap();
    settle(&mut fixture.runtime);
    assert_eq!(adapter.handed_off().len(), 1);
}

/// Stop waiting abandons the job, frees the adapter and keeps the bytes. The
/// abandoned outcome never arrives.
#[test]
fn cancelling_a_handoff_in_flight_abandons_it_and_keeps_the_bytes() {
    let mut fixture = ready();
    let adapter = install(&mut fixture);
    adapter.set_pending_polls(8);
    fixture.runtime.hand_off_library_export().unwrap();
    fixture.runtime.update(Duration::ZERO);
    assert!(adapter.busy());
    fixture.runtime.cancel_library_slot_request().unwrap();
    assert_eq!(adapter.abandoned(), 1);
    assert!(!adapter.busy());
    assert_eq!(
        fixture.runtime.library_slots_status(),
        ready_status(fixture.slot, fixture.generation)
    );
    for _ in 0..16 {
        fixture.runtime.update(Duration::ZERO);
    }
    assert_eq!(
        fixture.runtime.library_slots_status(),
        ready_status(fixture.slot, fixture.generation)
    );
    assert!(adapter.handed_off().is_empty());
    assert_eq!(transfer_diagnostics(&fixture.runtime), 0);

    adapter.set_pending_polls(0);
    fixture.runtime.hand_off_library_export().unwrap();
    settle(&mut fixture.runtime);
    assert_eq!(
        adapter.handed_off(),
        vec![expected_handoff(valid_archive(12))]
    );
}

/// An adapter that forgets its job, or answers with the wrong shape, is a
/// protocol failure that keeps the bytes.
#[test]
fn a_misbehaving_adapter_is_a_protocol_failure() {
    for reply in [ScriptedReply::Forget, ScriptedReply::WrongShape] {
        let mut fixture = ready();
        let adapter = install(&mut fixture);
        adapter.push_reply(reply.clone());
        fixture.runtime.hand_off_library_export().unwrap();
        settle(&mut fixture.runtime);
        assert_eq!(
            fixture.runtime.library_slots_status(),
            LibrarySlotsStatus::HandOffFailed {
                slot: fixture.slot,
                generation: fixture.generation,
                source: ExportSource::Head,
                code: TransferFailureCode::Protocol,
            },
            "{reply:?}"
        );
        assert!(fixture.runtime.prepared_slot_export().is_some());
        assert_eq!(transfer_diagnostics(&fixture.runtime), 1);
        assert_store_untouched(&fixture);
    }
}

/// The adapter cannot be swapped out from under a job it owns.
#[test]
fn the_adapter_cannot_be_replaced_mid_handoff() {
    let mut fixture = ready();
    let adapter = install(&mut fixture);
    adapter.set_pending_polls(4);
    fixture.runtime.hand_off_library_export().unwrap();
    let replacement = ScriptedTransfer::default();
    assert_eq!(
        fixture
            .runtime
            .install_transfer_adapter(Box::new(replacement.clone())),
        Err(ClientRuntimeError::LibraryRequestActive)
    );
    settle(&mut fixture.runtime);
    assert_eq!(adapter.handed_off().len(), 1);
    assert!(replacement.started().is_empty());
    fixture
        .runtime
        .install_transfer_adapter(Box::new(replacement))
        .unwrap();
}
