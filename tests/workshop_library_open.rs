//! Library Open through the exact-catalog client: route-design task 12a.
//!
//! Addendum §4 makes an explicit Open compare the row's observed generation,
//! claim the Continue marker by compare-and-swap, and replace the resident
//! session only after every store mutation succeeded. §6 keeps every failure
//! on the Library and gates the open on the resident Workshop being
//! replaceable. Each test below pins one of those rules at the runtime seam;
//! the screen's gating is pinned separately in `workshop_ui_library`.

use std::{
    cell::{Cell, RefCell},
    rc::Rc,
    time::Duration,
};

use nyon::{
    app::{
        AppCore,
        client_runtime::{
            ActiveSession, ClientDiagnosticCode, ClientRuntime, ClientRuntimeError, ClientScreen,
            LibrarySlotsStatus, SlotRequestKind,
        },
    },
    preferences::store::MemoryPreferencesStore,
    scenario::{ScenarioDraft, store::MemoryScenarioStore},
    workshop::{
        BatchLocalId, CreatorBatchV1, CreatorOpV1, GalaxyPointV1, ObjectName, WorkshopHistory,
        decode_catalog_pack, encode_archive,
        session::WorkshopAction,
        store::{
            MemoryWorkshopStore, SaveGeneration, SlotId, SlotList, SlotName, StoreJobId,
            StoreJobState, WorkshopStore, WorkshopStoreError, WorkshopStoreRequest,
            WorkshopStoreResult,
        },
    },
};

/// A store both the runtime and the test can reach, so a test can move a head
/// behind the runtime's back and read the Continue marker afterwards.
#[derive(Clone, Default)]
struct Shared {
    inner: Rc<RefCell<MemoryWorkshopStore>>,
    /// Refuses the next `LoadSlot` start once, then behaves.
    fail_next_load: Rc<Cell<bool>>,
}

impl WorkshopStore for Shared {
    fn start(&mut self, request: WorkshopStoreRequest) -> Result<StoreJobId, WorkshopStoreError> {
        if matches!(request, WorkshopStoreRequest::LoadSlot { .. })
            && self.fail_next_load.replace(false)
        {
            return Err(WorkshopStoreError::JobIdExhausted);
        }
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

fn runtime(store: &Shared) -> Runtime {
    ClientRuntime::new(
        AppCore::new_with_preferences(
            ScenarioDraft::factory_default().validated().unwrap(),
            MemoryScenarioStore::default(),
            MemoryPreferencesStore::default(),
        ),
        store.clone(),
    )
}

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
    let result = complete(
        store,
        WorkshopStoreRequest::CreateSlot {
            name: SlotName::new(name).unwrap(),
            archive: valid_archive(seed),
        },
    );
    let WorkshopStoreResult::SlotCreated { slot, generation } = result else {
        panic!("unexpected create result: {result:?}");
    };
    (slot, generation)
}

fn commit(
    store: &Shared,
    slot: SlotId,
    expected: SaveGeneration,
    archive: Box<[u8]>,
) -> SaveGeneration {
    let result = complete(
        store,
        WorkshopStoreRequest::CommitSlot {
            slot,
            expected_generation: expected,
            archive,
        },
    );
    let WorkshopStoreResult::SlotCommitted { generation, .. } = result else {
        panic!("unexpected commit result: {result:?}");
    };
    generation
}

fn stored_list(store: &Shared) -> SlotList {
    let WorkshopStoreResult::Slots(list) = complete(store, WorkshopStoreRequest::ListSlots) else {
        panic!("ListSlots answered with the wrong shape");
    };
    list
}

/// Polls until the Library lane is no longer working, bounded.
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

fn library_generation(runtime: &Runtime, slot: SlotId) -> SaveGeneration {
    runtime
        .library_slots()
        .expect("the Library has listed")
        .slots
        .iter()
        .find(|summary| summary.id == slot)
        .expect("the slot is listed")
        .generation
}

fn dirty_resident(runtime: &mut Runtime) {
    let snapshot = runtime.workshop_snapshot().unwrap();
    runtime
        .enqueue_workshop_action(WorkshopAction::Submit(CreatorBatchV1 {
            expected_cursor: snapshot.active_view.view_cursor,
            expected_tick: snapshot.active_view.tick,
            operations: vec![CreatorOpV1::CreateSystem {
                local: BatchLocalId(1),
                name: ObjectName::new("Unsaved Forge").unwrap(),
                position: GalaxyPointV1::new(64, 64).unwrap(),
            }],
        }))
        .unwrap();
    runtime.update(Duration::ZERO);
    assert!(runtime.workshop_snapshot().unwrap().store.dirty);
}

#[test]
fn open_installs_the_exact_row_only_after_claiming_continue_for_it() {
    let store = Shared::default();
    let (first, generation) = create(&store, "First Forge", 11);
    create(&store, "Second Forge", 12);
    let mut runtime = runtime(&store);
    runtime.open_library().unwrap();
    settle(&mut runtime);

    runtime.open_library_slot(first, generation).unwrap();
    assert_eq!(
        runtime.library_slots_status(),
        LibrarySlotsStatus::Working {
            kind: SlotRequestKind::Open,
            slot: Some(first),
        }
    );
    // Nothing is replaced while the open is still in flight.
    assert!(matches!(runtime.active_session(), ActiveSession::None));
    assert_eq!(runtime.screen(), ClientScreen::Library);

    settle(&mut runtime);
    assert_eq!(runtime.library_slots_status(), LibrarySlotsStatus::Idle);
    assert_eq!(runtime.screen(), ClientScreen::GalaxyWorkshop);
    let snapshot = runtime.workshop_snapshot().expect("the row was installed");
    assert_eq!(snapshot.store.slot, Some(first));
    assert_eq!(snapshot.store.generation, Some(generation));
    assert_eq!(snapshot.store.head_generation, Some(generation));
    assert_eq!(stored_list(&store).selected_continue, Some(first));
    assert!(
        runtime.library_slots().is_none(),
        "the cached list named the old Continue marker"
    );
    // The opened save is the Continue target, so returning to the menu offers
    // it without another bootstrap.
    runtime.return_to_main_menu().unwrap();
    for _ in 0..8 {
        runtime.update(Duration::ZERO);
    }
    assert!(runtime.continue_available());
}

#[test]
fn open_is_refused_before_any_store_work_while_the_resident_is_unsaved() {
    let store = Shared::default();
    let (first, generation) = create(&store, "First Forge", 11);
    let mut runtime = runtime(&store);
    runtime.start_new_workshop(5).unwrap();
    dirty_resident(&mut runtime);
    let digest = runtime.workshop_snapshot().unwrap().state_digest;
    runtime.open_library().unwrap();
    settle(&mut runtime);
    assert!(runtime.resident_workshop_blocks_replacement());

    assert_eq!(
        runtime.open_library_slot(first, generation),
        Err(ClientRuntimeError::RouteUnavailable)
    );
    assert_eq!(runtime.library_slots_status(), LibrarySlotsStatus::Idle);
    assert_eq!(stored_list(&store).selected_continue, None);
    let after = runtime.workshop_snapshot().unwrap();
    assert_eq!(after.state_digest, digest);
    assert!(after.store.dirty);
    assert_eq!(runtime.screen(), ClientScreen::Library);
}

#[test]
fn the_resident_workshops_own_slot_is_refused() {
    let store = Shared::default();
    let (first, generation) = create(&store, "First Forge", 11);
    let mut runtime = runtime(&store);
    runtime.open_library().unwrap();
    settle(&mut runtime);
    runtime.open_library_slot(first, generation).unwrap();
    settle(&mut runtime);
    assert_eq!(runtime.workshop_snapshot().unwrap().store.slot, Some(first));

    runtime.open_library().unwrap();
    settle(&mut runtime);
    assert_eq!(
        runtime.open_library_slot(first, library_generation(&runtime, first)),
        Err(ClientRuntimeError::RouteUnavailable)
    );
    assert_eq!(runtime.library_slots_status(), LibrarySlotsStatus::Idle);
}

#[test]
fn a_stale_row_offers_refresh_and_refresh_relists_rather_than_reopening() {
    let store = Shared::default();
    let (first, observed) = create(&store, "First Forge", 11);
    let mut runtime = runtime(&store);
    runtime.open_library().unwrap();
    settle(&mut runtime);
    let head = commit(&store, first, observed, valid_archive(12));

    runtime.open_library_slot(first, observed).unwrap();
    settle(&mut runtime);
    assert_eq!(
        runtime.library_slots_status(),
        LibrarySlotsStatus::Failed {
            kind: SlotRequestKind::Open,
            slot: Some(first),
            code: ClientDiagnosticCode::StaleSave,
        }
    );
    assert_eq!(runtime.screen(), ClientScreen::Library);
    assert!(matches!(runtime.active_session(), ActiveSession::None));
    assert_eq!(stored_list(&store).selected_continue, None);
    assert_eq!(library_generation(&runtime, first), observed);

    runtime.retry_library_slot_request().unwrap();
    assert_eq!(
        runtime.library_slots_status(),
        LibrarySlotsStatus::Working {
            kind: SlotRequestKind::List,
            slot: None,
        },
        "Refresh re-lists; it must not repeat the stale open"
    );
    settle(&mut runtime);
    assert_eq!(library_generation(&runtime, first), head);

    runtime.open_library_slot(first, head).unwrap();
    settle(&mut runtime);
    assert_eq!(
        runtime.workshop_snapshot().unwrap().store.generation,
        Some(head)
    );
}

#[test]
fn a_store_refusal_stays_on_the_library_and_retry_reopens_the_same_row() {
    let store = Shared::default();
    let (first, generation) = create(&store, "First Forge", 11);
    let mut runtime = runtime(&store);
    runtime.open_library().unwrap();
    settle(&mut runtime);

    store.fail_next_load.set(true);
    assert!(matches!(
        runtime.open_library_slot(first, generation),
        Err(ClientRuntimeError::Store(_))
    ));
    assert_eq!(
        runtime.library_slots_status(),
        LibrarySlotsStatus::Failed {
            kind: SlotRequestKind::Open,
            slot: Some(first),
            code: ClientDiagnosticCode::Store,
        }
    );
    assert_eq!(runtime.screen(), ClientScreen::Library);
    assert!(runtime.recovery_diagnostic().is_none());
    // The retained failure is a decision the user owns: a second open may
    // not overwrite it.
    let failed = runtime.library_slots_status();
    assert_eq!(
        runtime.open_library_slot(first, generation),
        Err(ClientRuntimeError::LibraryRequestActive)
    );
    assert_eq!(runtime.library_slots_status(), failed);

    runtime.retry_library_slot_request().unwrap();
    settle(&mut runtime);
    assert_eq!(runtime.screen(), ClientScreen::GalaxyWorkshop);
    assert_eq!(runtime.workshop_snapshot().unwrap().store.slot, Some(first));
}

#[test]
fn a_retry_refused_by_the_replacement_gate_keeps_the_failure_to_resolve() {
    let store = Shared::default();
    let (first, generation) = create(&store, "First Forge", 11);
    let mut runtime = runtime(&store);
    runtime.start_new_workshop(5).unwrap();
    runtime.open_library().unwrap();
    settle(&mut runtime);
    store.fail_next_load.set(true);
    let _ = runtime.open_library_slot(first, generation);
    let failed = runtime.library_slots_status();
    assert!(matches!(failed, LibrarySlotsStatus::Failed { .. }));

    dirty_resident(&mut runtime);
    assert_eq!(
        runtime.retry_library_slot_request(),
        Err(ClientRuntimeError::RouteUnavailable)
    );
    assert_eq!(runtime.library_slots_status(), failed);
    runtime.cancel_library_slot_request().unwrap();
    assert_eq!(runtime.library_slots_status(), LibrarySlotsStatus::Idle);
}

#[test]
fn an_invalid_head_is_held_until_the_user_accepts_its_predecessor() {
    let store = Shared::default();
    let (first, valid) = create(&store, "First Forge", 11);
    let corrupt = commit(&store, first, valid, Box::from(&b"{}"[..]));
    let mut runtime = runtime(&store);
    runtime.open_library().unwrap();
    settle(&mut runtime);
    assert_eq!(library_generation(&runtime, first), corrupt);

    runtime.open_library_slot(first, corrupt).unwrap();
    settle(&mut runtime);
    assert_eq!(
        runtime.library_slots_status(),
        LibrarySlotsStatus::Held {
            slot: first,
            recovered: true,
        }
    );
    // A predecessor never claims the marker and is never installed unasked.
    assert_eq!(stored_list(&store).selected_continue, None);
    assert!(matches!(runtime.active_session(), ActiveSession::None));
    assert_eq!(runtime.screen(), ClientScreen::Library);

    runtime.accept_library_open().unwrap();
    assert_eq!(runtime.screen(), ClientScreen::GalaxyWorkshop);
    let snapshot = runtime.workshop_snapshot().unwrap();
    assert_eq!(snapshot.store.generation, Some(valid));
    assert_eq!(snapshot.store.head_generation, Some(corrupt));
    // The session owns promotion and then selection, exactly as after the
    // startup recovery offer.
    for _ in 0..16 {
        runtime.update(Duration::ZERO);
    }
    let promoted = runtime
        .workshop_snapshot()
        .unwrap()
        .store
        .generation
        .unwrap();
    assert!(
        promoted.0 > corrupt.0,
        "the predecessor was promoted to a new head"
    );
    assert_eq!(stored_list(&store).selected_continue, Some(first));
}

/// Cancel at every point an open can be in: before its first poll, while
/// loading, while replaying, and while `SelectContinue` holds the Commit lane.
/// After each, the Commit lane must be free for a rename, and the client must
/// be idle enough to accept a fresh open.
#[test]
fn cancelling_an_open_in_flight_frees_the_lane_and_installs_nothing() {
    for polls in 0..=3 {
        let store = Shared::default();
        let (first, generation) = create(&store, "First Forge", 11);
        let (second, _) = create(&store, "Second Forge", 12);
        let mut runtime = runtime(&store);
        runtime.open_library().unwrap();
        settle(&mut runtime);
        runtime.open_library_slot(first, generation).unwrap();
        for _ in 0..polls {
            runtime.update(Duration::ZERO);
        }
        if !matches!(
            runtime.library_slots_status(),
            LibrarySlotsStatus::Working { .. }
        ) {
            // The open already finished at this poll count; nothing to cancel.
            continue;
        }

        assert_eq!(
            runtime.rename_library_slot(second, SlotName::new("Busy").unwrap()),
            Err(ClientRuntimeError::LibraryRequestActive),
            "{polls}: an open occupies the one Library lane"
        );
        runtime.cancel_library_slot_request().unwrap();
        assert_eq!(runtime.library_slots_status(), LibrarySlotsStatus::Idle);
        assert!(runtime.library_slots().is_none());
        for _ in 0..8 {
            runtime.update(Duration::ZERO);
        }
        assert!(
            matches!(runtime.active_session(), ActiveSession::None),
            "{polls}"
        );
        assert_eq!(runtime.screen(), ClientScreen::Library);

        // Both store lanes are free again: a Commit-lane rename completes.
        runtime
            .rename_library_slot(second, SlotName::new("Renamed Forge").unwrap())
            .unwrap();
        settle(&mut runtime);
        assert_eq!(
            runtime.library_slots_status(),
            LibrarySlotsStatus::Idle,
            "{polls}: the rename did not complete"
        );

        // And the client was reset: a fresh open of the current head runs.
        let head = library_generation(&runtime, first);
        runtime
            .open_library_slot(first, head)
            .unwrap_or_else(|error| {
                panic!("{polls}: a fresh open after Cancel was refused: {error}")
            });
        settle(&mut runtime);
        assert_eq!(
            runtime.workshop_snapshot().map(|s| s.store.slot),
            Some(Some(first)),
            "{polls}"
        );
    }
}

#[test]
fn an_open_finishing_off_screen_is_held_instead_of_switching_screens() {
    let store = Shared::default();
    let (first, generation) = create(&store, "First Forge", 11);
    let mut runtime = runtime(&store);
    runtime.open_library().unwrap();
    settle(&mut runtime);
    runtime.open_library_slot(first, generation).unwrap();
    runtime.close_library();
    settle(&mut runtime);

    assert_eq!(runtime.screen(), ClientScreen::MainMenu);
    assert!(matches!(runtime.active_session(), ActiveSession::None));
    assert_eq!(
        runtime.library_slots_status(),
        LibrarySlotsStatus::Held {
            slot: first,
            recovered: false,
        }
    );

    runtime.open_library().unwrap();
    assert_eq!(
        runtime.library_slots_status(),
        LibrarySlotsStatus::Held {
            slot: first,
            recovered: false,
        },
        "reopening the Library must not resolve the held decision"
    );
    runtime.accept_library_open().unwrap();
    assert_eq!(runtime.screen(), ClientScreen::GalaxyWorkshop);
    assert_eq!(runtime.workshop_snapshot().unwrap().store.slot, Some(first));
}

#[test]
fn accepting_a_held_open_waits_while_the_resident_is_unsaved() {
    let store = Shared::default();
    let (first, generation) = create(&store, "First Forge", 11);
    let mut runtime = runtime(&store);
    runtime.start_new_workshop(5).unwrap();
    runtime.open_library().unwrap();
    settle(&mut runtime);
    runtime.open_library_slot(first, generation).unwrap();
    runtime.close_library();
    settle(&mut runtime);
    assert!(matches!(
        runtime.library_slots_status(),
        LibrarySlotsStatus::Held { .. }
    ));

    dirty_resident(&mut runtime);
    let digest = runtime.workshop_snapshot().unwrap().state_digest;
    assert_eq!(
        runtime.accept_library_open(),
        Err(ClientRuntimeError::RouteUnavailable)
    );
    assert!(matches!(
        runtime.library_slots_status(),
        LibrarySlotsStatus::Held { .. }
    ));
    let resident = runtime.workshop_snapshot().unwrap();
    assert_eq!(resident.state_digest, digest);
    assert_ne!(resident.store.slot, Some(first));
}
