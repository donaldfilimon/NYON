//! Import galaxy: route-design task 11's last strip route, which is task 12's
//! Import archive.
//!
//! Addendum §5: a portable archive is validated against its exact declared
//! catalog and fully replayed before it becomes a session, and the imported
//! Workshop is paused, slotless, dirty, labelled `Imported; not saved`, and
//! not a Continue target. A missing pack is a retryable Library state that
//! resumes without a new archive choice once the matching pack has stored.
//! §6: the import is disabled while the resident Workshop cannot be replaced,
//! and nothing fails into the global recovery screen. The file choice runs
//! through `ScriptedTransfer`; no live picker is exercised.

use std::{cell::RefCell, rc::Rc, time::Duration};

use nyon::{
    app::{
        AppCore,
        client_runtime::{
            ActiveSession, ClientDiagnosticCode, ClientRuntime, ClientRuntimeError, ClientScreen,
            LibrarySlotsStatus, SlotRequestKind,
        },
        transfer::{
            ScriptedReply, ScriptedTransfer, TransferFailureCode, TransferKind, TransferRequest,
        },
    },
    preferences::store::MemoryPreferencesStore,
    scenario::{ScenarioDraft, store::MemoryScenarioStore},
    workshop::{
        BatchLocalId, CatalogHash, CreatorBatchV1, CreatorOpV1, GalaxyPointV1, ObjectName,
        WorkshopHistory, decode_catalog_pack, encode_archive, encode_catalog_pack,
        session::{WorkshopAction, WorkshopSpeed},
        store::{
            MAX_WORKSHOP_ARCHIVE_BYTES, MAX_WORKSHOP_PACK_BYTES, MemoryWorkshopStore, SlotName,
            StoreJobId, StoreJobState, WorkshopStore, WorkshopStoreError, WorkshopStoreRequest,
            WorkshopStoreResult,
        },
    },
};

const CORE_PACK: &[u8] = include_bytes!("../assets/workshop/core-pack-v1.json");

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

fn stored_packs(store: &Shared) -> Vec<CatalogHash> {
    let WorkshopStoreResult::Packs(packs) = complete(store, WorkshopStoreRequest::ListPacks) else {
        panic!("ListPacks answered with the wrong shape");
    };
    packs
}

fn stored_slots(store: &Shared) -> (usize, Option<nyon::workshop::store::SlotId>) {
    let WorkshopStoreResult::Slots(list) = complete(store, WorkshopStoreRequest::ListSlots) else {
        panic!("ListSlots answered with the wrong shape");
    };
    (list.slots.len(), list.selected_continue)
}

fn custom_pack() -> Box<[u8]> {
    let mut value: serde_json::Value = serde_json::from_slice(CORE_PACK).unwrap();
    value["title"] = serde_json::json!("Custom Forge Catalog");
    let validated = decode_catalog_pack(&serde_json::to_vec(&value).unwrap()).unwrap();
    encode_catalog_pack(&validated).unwrap().into_boxed_slice()
}

fn hash_of(pack: &[u8]) -> CatalogHash {
    decode_catalog_pack(pack).unwrap().catalog_hash()
}

/// An archive with one created system on top of `seed`'s galaxy, so a
/// replay that silently dropped history would not match.
fn archive_over(pack: &[u8], seed: u64) -> Box<[u8]> {
    let mut history = WorkshopHistory::from_seed_u64(decode_catalog_pack(pack).unwrap(), seed);
    let view = history.active_view();
    history
        .submit(CreatorBatchV1 {
            expected_cursor: view.view_cursor,
            expected_tick: view.tick,
            operations: vec![CreatorOpV1::CreateSystem {
                local: BatchLocalId(1),
                name: ObjectName::new("Imported Forge").unwrap(),
                position: GalaxyPointV1::new(40, 40).unwrap(),
            }],
        })
        .unwrap();
    encode_archive(&history).unwrap().bytes.into_boxed_slice()
}

/// Polls until the Library lane is no longer working, bounded.
fn settle(runtime: &mut Runtime) {
    for _ in 0..64 {
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

fn library(store: &Shared) -> (Runtime, ScriptedTransfer) {
    let mut runtime = runtime(store);
    runtime.open_library().unwrap();
    settle(&mut runtime);
    let adapter = ScriptedTransfer::default();
    runtime
        .install_transfer_adapter(Box::new(adapter.clone()))
        .unwrap();
    (runtime, adapter)
}

fn session_archive(runtime: &Runtime) -> Box<[u8]> {
    let ActiveSession::Workshop(session) = runtime.active_session() else {
        panic!("no Workshop is open");
    };
    encode_archive(session.history())
        .unwrap()
        .bytes
        .into_boxed_slice()
}

/// §5's four predicates, plus what follows from them.
fn assert_imported(runtime: &Runtime, archive: &[u8]) {
    assert_eq!(runtime.screen(), ClientScreen::GalaxyWorkshop);
    let snapshot = runtime.workshop_snapshot().expect("a Workshop is open");
    assert_eq!(snapshot.speed, WorkshopSpeed::Paused);
    assert_eq!(snapshot.store.slot, None);
    assert!(snapshot.store.dirty);
    assert_eq!(snapshot.store.status, "Imported; not saved");
    assert!(!runtime.continue_available());
    assert!(runtime.resident_workshop_blocks_replacement());
    assert_eq!(session_archive(runtime).as_ref(), archive);
    assert!(runtime.recovery_diagnostic().is_none());
}

fn make_dirty(runtime: &mut Runtime) {
    let snapshot = runtime.workshop_snapshot().unwrap();
    runtime
        .enqueue_workshop_action(WorkshopAction::Submit(CreatorBatchV1 {
            expected_cursor: snapshot.active_view.view_cursor,
            expected_tick: snapshot.active_view.tick,
            operations: vec![CreatorOpV1::CreateSystem {
                local: BatchLocalId(9),
                name: ObjectName::new("Unsaved Forge").unwrap(),
                position: GalaxyPointV1::new(64, 64).unwrap(),
            }],
        }))
        .unwrap();
}

#[test]
fn a_built_in_catalog_archive_imports_as_a_paused_slotless_dirty_workshop() {
    let store = Shared::default();
    let (mut runtime, adapter) = library(&store);
    let archive = archive_over(CORE_PACK, 21);
    adapter.set_import_bytes(Some(archive.clone()));

    runtime.import_library_archive().unwrap();
    assert_eq!(
        runtime.library_slots_status(),
        LibrarySlotsStatus::Working {
            kind: SlotRequestKind::ChooseArchive,
            slot: None,
        }
    );
    assert_eq!(
        adapter.started(),
        vec![TransferRequest::ChooseImport {
            kind: TransferKind::WorkshopArchive,
            max_bytes: MAX_WORKSHOP_ARCHIVE_BYTES,
        }]
    );
    // The lane is held while the file is chosen and replayed.
    assert_eq!(
        runtime.refresh_library_slots(),
        Err(ClientRuntimeError::LibraryRequestActive)
    );

    settle(&mut runtime);
    assert_imported(&runtime, &archive);
    assert_eq!(runtime.library_slots_status(), LibrarySlotsStatus::Idle);
    // Importing stores nothing and selects nothing.
    assert_eq!(stored_slots(&store), (0, None));
    assert!(stored_packs(&store).is_empty());
}

#[test]
fn import_galaxy_needs_an_adapter_a_free_lane_and_a_replaceable_resident() {
    let store = Shared::default();
    let mut runtime = runtime(&store);
    runtime.open_library().unwrap();
    settle(&mut runtime);
    assert_eq!(
        runtime.import_library_archive(),
        Err(ClientRuntimeError::TransferUnavailable)
    );
    assert_eq!(runtime.library_slots_status(), LibrarySlotsStatus::Idle);

    // An unsaved resident blocks it before anything is chosen (§6).
    runtime.close_library();
    runtime.start_new_workshop(3).unwrap();
    make_dirty(&mut runtime);
    runtime.update(Duration::ZERO);
    assert!(runtime.resident_workshop_blocks_replacement());
    let digest = runtime.workshop_snapshot().unwrap().state_digest;
    runtime.open_library().unwrap();
    settle(&mut runtime);
    let adapter = ScriptedTransfer::default();
    runtime
        .install_transfer_adapter(Box::new(adapter.clone()))
        .unwrap();
    adapter.set_import_bytes(Some(archive_over(CORE_PACK, 4)));
    assert_eq!(
        runtime.import_library_archive(),
        Err(ClientRuntimeError::RouteUnavailable)
    );
    assert!(adapter.started().is_empty());
    assert_eq!(runtime.library_slots_status(), LibrarySlotsStatus::Idle);
    assert_eq!(runtime.workshop_snapshot().unwrap().state_digest, digest);

    // The lane: an export holds it.
    runtime.export_active_pack().unwrap();
    assert_eq!(
        runtime.import_library_archive(),
        Err(ClientRuntimeError::LibraryRequestActive)
    );
}

#[test]
fn a_resident_that_became_unsaved_holds_the_validated_import() {
    let store = Shared::default();
    let (mut runtime, adapter) = library(&store);
    runtime.close_library();
    runtime.start_new_workshop(3).unwrap();
    runtime.update(Duration::ZERO);
    assert!(!runtime.resident_workshop_blocks_replacement());
    runtime.open_library().unwrap();
    settle(&mut runtime);
    let archive = archive_over(CORE_PACK, 22);
    adapter.set_import_bytes(Some(archive.clone()));
    adapter.set_pending_polls(1);
    runtime.import_library_archive().unwrap();
    // The resident gains unsaved work while the file is being chosen.
    make_dirty(&mut runtime);
    settle(&mut runtime);
    assert_eq!(
        runtime.library_slots_status(),
        LibrarySlotsStatus::ImportHeld
    );
    assert_eq!(runtime.screen(), ClientScreen::Library);
    let resident = runtime.workshop_snapshot().unwrap().clone();
    assert!(resident.store.dirty);

    // Accepting is refused while the resident still cannot be replaced, and
    // keeps the validated import.
    assert_eq!(
        runtime.accept_library_open(),
        Err(ClientRuntimeError::RouteUnavailable)
    );
    assert_eq!(
        runtime.library_slots_status(),
        LibrarySlotsStatus::ImportHeld
    );
    assert_eq!(
        runtime.workshop_snapshot().unwrap().state_digest,
        resident.state_digest
    );

    // Once the resident has saved, acceptance installs without a new choice.
    for _ in 0..64 {
        runtime.update(Duration::from_millis(300));
        if !runtime.resident_workshop_blocks_replacement() {
            break;
        }
    }
    assert!(!runtime.resident_workshop_blocks_replacement());
    runtime.accept_library_open().unwrap();
    assert_imported(&runtime, &archive);
    assert_eq!(adapter.started().len(), 1);
    assert_eq!(runtime.library_slots_status(), LibrarySlotsStatus::Idle);
}

#[test]
fn an_import_finishing_after_the_library_closed_is_held_and_cancel_drops_it() {
    let store = Shared::default();
    let (mut runtime, adapter) = library(&store);
    adapter.set_import_bytes(Some(archive_over(CORE_PACK, 23)));
    adapter.set_pending_polls(1);
    runtime.import_library_archive().unwrap();
    runtime.close_library();
    for _ in 0..16 {
        runtime.update(Duration::ZERO);
    }
    assert_eq!(runtime.screen(), ClientScreen::MainMenu);
    assert!(matches!(runtime.active_session(), ActiveSession::None));
    assert_eq!(
        runtime.library_slots_status(),
        LibrarySlotsStatus::ImportHeld
    );
    runtime.open_library().unwrap();
    runtime.cancel_library_slot_request().unwrap();
    assert_eq!(runtime.library_slots_status(), LibrarySlotsStatus::Idle);
    assert_eq!(
        runtime.accept_library_open(),
        Err(ClientRuntimeError::RouteUnavailable)
    );
    assert!(matches!(runtime.active_session(), ActiveSession::None));
}

#[test]
fn an_archive_whose_pack_is_stored_imports_through_the_exact_pack() {
    let store = Shared::default();
    let pack = custom_pack();
    complete(
        &store,
        WorkshopStoreRequest::PutPack {
            canonical_pack: pack.clone(),
        },
    );
    let (mut runtime, adapter) = library(&store);
    let archive = archive_over(&pack, 24);
    adapter.set_import_bytes(Some(archive.clone()));
    runtime.import_library_archive().unwrap();
    settle(&mut runtime);
    assert_imported(&runtime, &archive);
    let ActiveSession::Workshop(session) = runtime.active_session() else {
        unreachable!();
    };
    assert_eq!(session.history().catalog().catalog_hash(), hash_of(&pack));
}

#[test]
fn a_missing_pack_is_retryable_and_resumes_without_choosing_the_archive_again() {
    let store = Shared::default();
    let (mut runtime, adapter) = library(&store);
    let pack = custom_pack();
    let hash = hash_of(&pack);
    let archive = archive_over(&pack, 25);
    adapter.set_import_bytes(Some(archive.clone()));
    runtime.import_library_archive().unwrap();
    settle(&mut runtime);
    let waiting = LibrarySlotsStatus::ImportNeedsPack {
        hash,
        problem: None,
    };
    assert_eq!(runtime.library_slots_status(), waiting);
    assert!(matches!(runtime.active_session(), ActiveSession::None));
    assert_eq!(runtime.screen(), ClientScreen::Library);
    assert!(runtime.recovery_diagnostic().is_none());
    for _ in 0..4 {
        runtime.update(Duration::ZERO);
    }
    assert_eq!(runtime.library_slots_status(), waiting, "a decision waits");

    // "Import pack" asks for a content pack, not the archive again.
    adapter.set_import_bytes(Some(pack.clone()));
    runtime.retry_library_slot_request().unwrap();
    assert_eq!(
        runtime.library_slots_status(),
        LibrarySlotsStatus::Working {
            kind: SlotRequestKind::ChoosePack,
            slot: None,
        }
    );
    assert_eq!(
        adapter.started()[1],
        TransferRequest::ChooseImport {
            kind: TransferKind::ContentPack,
            max_bytes: MAX_WORKSHOP_PACK_BYTES,
        }
    );
    settle(&mut runtime);
    assert_imported(&runtime, &archive);
    assert_eq!(adapter.started().len(), 2, "the archive was chosen once");
    assert_eq!(stored_packs(&store), vec![hash]);
    assert_eq!(runtime.imported_catalog_hash(), Some(hash));
    assert_eq!(stored_slots(&store), (0, None));
}

#[test]
fn a_different_or_invalid_pack_is_refused_and_the_archive_keeps_waiting() {
    let store = Shared::default();
    let (mut runtime, adapter) = library(&store);
    let pack = custom_pack();
    let hash = hash_of(&pack);
    let archive = archive_over(&pack, 26);
    adapter.set_import_bytes(Some(archive.clone()));
    runtime.import_library_archive().unwrap();
    settle(&mut runtime);

    let refused = LibrarySlotsStatus::ImportNeedsPack {
        hash,
        problem: Some(ClientDiagnosticCode::Catalog),
    };
    for wrong in [
        Box::<[u8]>::from(CORE_PACK),
        Box::<[u8]>::from(&b"{\"not\":\"a pack\"}"[..]),
    ] {
        adapter.set_import_bytes(Some(wrong));
        runtime.retry_library_slot_request().unwrap();
        settle(&mut runtime);
        assert_eq!(runtime.library_slots_status(), refused);
        // A refused pack is never stored, and nothing is installed.
        assert!(stored_packs(&store).is_empty());
        assert!(matches!(runtime.active_session(), ActiveSession::None));
    }

    // A dismissed pack picker goes back to waiting, without a problem.
    adapter.set_import_bytes(None);
    runtime.retry_library_slot_request().unwrap();
    settle(&mut runtime);
    assert_eq!(
        runtime.library_slots_status(),
        LibrarySlotsStatus::ImportNeedsPack {
            hash,
            problem: None,
        }
    );
    // A failed pack picker is a transfer problem, still waiting.
    adapter.push_reply(ScriptedReply::Fail(TransferFailureCode::Platform));
    runtime.retry_library_slot_request().unwrap();
    settle(&mut runtime);
    assert_eq!(
        runtime.library_slots_status(),
        LibrarySlotsStatus::ImportNeedsPack {
            hash,
            problem: Some(ClientDiagnosticCode::Transfer),
        }
    );

    adapter.set_import_bytes(Some(pack));
    runtime.retry_library_slot_request().unwrap();
    settle(&mut runtime);
    assert_imported(&runtime, &archive);
    assert_eq!(adapter.started().len(), 6, "one archive choice, five packs");
}

#[test]
fn cancel_releases_the_archive_and_never_deletes_a_stored_pack() {
    let store = Shared::default();
    let (mut runtime, adapter) = library(&store);
    let pack = custom_pack();
    let hash = hash_of(&pack);
    let archive = archive_over(&pack, 27);

    // Cancel while waiting for the pack.
    adapter.set_import_bytes(Some(archive.clone()));
    runtime.import_library_archive().unwrap();
    settle(&mut runtime);
    runtime.cancel_library_slot_request().unwrap();
    assert_eq!(runtime.library_slots_status(), LibrarySlotsStatus::Idle);
    assert_eq!(
        runtime.retry_library_slot_request(),
        Err(ClientRuntimeError::RouteUnavailable),
        "the archive was released"
    );

    // Cancel while the matching pack is storing: the pack still lands and
    // stays, the archive is released, and nothing is installed.
    runtime.import_library_archive().unwrap();
    settle(&mut runtime);
    adapter.set_import_bytes(Some(pack));
    runtime.retry_library_slot_request().unwrap();
    runtime.update(Duration::ZERO);
    assert_eq!(
        runtime.library_slots_status(),
        LibrarySlotsStatus::Working {
            kind: SlotRequestKind::StorePack,
            slot: None,
        }
    );
    runtime.cancel_library_slot_request().unwrap();
    for _ in 0..8 {
        runtime.update(Duration::ZERO);
    }
    assert_eq!(runtime.library_slots_status(), LibrarySlotsStatus::Idle);
    assert_eq!(stored_packs(&store), vec![hash]);
    assert!(matches!(runtime.active_session(), ActiveSession::None));
    assert_eq!(runtime.screen(), ClientScreen::Library);

    // With the pack stored, the same archive now imports directly.
    adapter.set_import_bytes(Some(archive.clone()));
    runtime.import_library_archive().unwrap();
    settle(&mut runtime);
    assert_imported(&runtime, &archive);
    assert_eq!(stored_packs(&store), vec![hash]);
}

#[test]
fn a_pack_store_failure_during_import_offers_retry_and_cancel() {
    let store = Shared::default();
    let (mut runtime, adapter) = library(&store);
    let pack = custom_pack();
    let archive = archive_over(&pack, 28);
    adapter.set_import_bytes(Some(archive.clone()));
    runtime.import_library_archive().unwrap();
    settle(&mut runtime);

    // Hold the store's Commit lane, so `PutPack` is refused.
    let holder = store
        .inner
        .borrow_mut()
        .start(WorkshopStoreRequest::CreateSlot {
            name: SlotName::new("Holder").unwrap(),
            archive: archive_over(CORE_PACK, 1),
        })
        .unwrap();
    adapter.set_import_bytes(Some(pack));
    runtime.retry_library_slot_request().unwrap();
    settle(&mut runtime);
    let failed = LibrarySlotsStatus::Failed {
        kind: SlotRequestKind::StorePack,
        slot: None,
        code: ClientDiagnosticCode::Store,
    };
    assert_eq!(runtime.library_slots_status(), failed);
    assert_eq!(runtime.screen(), ClientScreen::Library);
    assert!(runtime.recovery_diagnostic().is_none());

    assert!(matches!(
        store.inner.borrow_mut().poll(holder),
        StoreJobState::Complete(Ok(_))
    ));
    runtime.retry_library_slot_request().unwrap();
    settle(&mut runtime);
    assert_imported(&runtime, &archive);
    assert_eq!(adapter.started().len(), 2, "no file is chosen again");
}

#[test]
fn a_store_failure_resolving_the_pack_is_retried_from_the_retained_archive() {
    let store = Shared::default();
    let pack = custom_pack();
    complete(
        &store,
        WorkshopStoreRequest::PutPack {
            canonical_pack: pack.clone(),
        },
    );
    let (mut runtime, adapter) = library(&store);
    let archive = archive_over(&pack, 29);
    adapter.set_import_bytes(Some(archive.clone()));
    // Hold the store's load lane, so `GetPack` is refused.
    let holder = store
        .inner
        .borrow_mut()
        .start(WorkshopStoreRequest::ListSlots)
        .unwrap();
    runtime.import_library_archive().unwrap();
    settle(&mut runtime);
    let failed = LibrarySlotsStatus::Failed {
        kind: SlotRequestKind::ImportArchive,
        slot: None,
        code: ClientDiagnosticCode::Store,
    };
    assert_eq!(runtime.library_slots_status(), failed);
    assert!(matches!(runtime.active_session(), ActiveSession::None));
    assert!(runtime.recovery_diagnostic().is_none());

    assert!(matches!(
        store.inner.borrow_mut().poll(holder),
        StoreJobState::Complete(Ok(_))
    ));
    runtime.retry_library_slot_request().unwrap();
    settle(&mut runtime);
    assert_imported(&runtime, &archive);
    assert_eq!(adapter.started().len(), 1, "the archive was chosen once");
}

#[test]
fn an_invalid_archive_is_refused_and_chosen_again() {
    let store = Shared::default();
    let (mut runtime, adapter) = library(&store);
    // Still a JSON object declaring a catalog hash, but its integrity digest
    // no longer matches.
    let mut value: serde_json::Value =
        serde_json::from_slice(&archive_over(CORE_PACK, 30)).unwrap();
    let first = value["integrity_sha256"][0]
        .as_u64()
        .expect("the archive carries a digest");
    value["integrity_sha256"][0] = serde_json::json!((first + 1) % 256);
    let tampered = serde_json::to_vec(&value).unwrap();
    for bytes in [
        Box::<[u8]>::from(&b"{}"[..]),
        Box::<[u8]>::from(&b"not json"[..]),
        tampered.into_boxed_slice(),
    ] {
        adapter.set_import_bytes(Some(bytes));
        match runtime.library_slots_status() {
            LibrarySlotsStatus::Idle => runtime.import_library_archive().unwrap(),
            _ => runtime.retry_library_slot_request().unwrap(),
        }
        settle(&mut runtime);
        assert_eq!(
            runtime.library_slots_status(),
            LibrarySlotsStatus::Failed {
                kind: SlotRequestKind::ChooseArchive,
                slot: None,
                code: ClientDiagnosticCode::Archive,
            }
        );
        assert!(matches!(runtime.active_session(), ActiveSession::None));
        assert_eq!(runtime.screen(), ClientScreen::Library);
        assert!(runtime.recovery_diagnostic().is_none());
    }
    assert_eq!(adapter.started().len(), 3, "each retry is a new choice");
    let archive = archive_over(CORE_PACK, 31);
    adapter.set_import_bytes(Some(archive.clone()));
    runtime.retry_library_slot_request().unwrap();
    settle(&mut runtime);
    assert_imported(&runtime, &archive);
}

#[test]
fn a_dismissed_or_failed_archive_picker_changes_nothing() {
    let store = Shared::default();
    let (mut runtime, adapter) = library(&store);
    runtime.import_library_archive().unwrap();
    settle(&mut runtime);
    assert_eq!(runtime.library_slots_status(), LibrarySlotsStatus::Idle);
    assert_eq!(runtime.diagnostics().count(), 0);

    adapter.push_reply(ScriptedReply::Fail(TransferFailureCode::Platform));
    runtime.import_library_archive().unwrap();
    settle(&mut runtime);
    assert_eq!(
        runtime.library_slots_status(),
        LibrarySlotsStatus::Failed {
            kind: SlotRequestKind::ChooseArchive,
            slot: None,
            code: ClientDiagnosticCode::Transfer,
        }
    );
    assert!(matches!(runtime.active_session(), ActiveSession::None));
    runtime.cancel_library_slot_request().unwrap();
    assert_eq!(runtime.library_slots_status(), LibrarySlotsStatus::Idle);
}

#[test]
fn a_replaceable_resident_is_replaced_by_the_import() {
    let store = Shared::default();
    let (mut runtime, adapter) = library(&store);
    runtime.close_library();
    runtime.start_new_workshop(5).unwrap();
    runtime.update(Duration::ZERO);
    assert!(!runtime.resident_workshop_blocks_replacement());
    runtime.open_library().unwrap();
    settle(&mut runtime);
    let archive = archive_over(CORE_PACK, 32);
    adapter.set_import_bytes(Some(archive.clone()));
    runtime.import_library_archive().unwrap();
    settle(&mut runtime);
    assert_imported(&runtime, &archive);
}

#[test]
fn cancelling_an_import_in_flight_frees_the_client_and_the_store() {
    let store = Shared::default();
    let pack = custom_pack();
    complete(
        &store,
        WorkshopStoreRequest::PutPack {
            canonical_pack: pack.clone(),
        },
    );
    let (mut runtime, adapter) = library(&store);
    let archive = archive_over(&pack, 33);
    adapter.set_import_bytes(Some(archive.clone()));
    runtime.import_library_archive().unwrap();
    runtime.update(Duration::ZERO);
    assert_eq!(
        runtime.library_slots_status(),
        LibrarySlotsStatus::Working {
            kind: SlotRequestKind::ImportArchive,
            slot: None,
        }
    );
    runtime.cancel_library_slot_request().unwrap();
    assert_eq!(runtime.library_slots_status(), LibrarySlotsStatus::Idle);
    for _ in 0..4 {
        runtime.update(Duration::ZERO);
    }
    assert!(matches!(runtime.active_session(), ActiveSession::None));
    // The store's load lane is free and the client can begin again.
    runtime.refresh_library_slots().unwrap();
    settle(&mut runtime);
    runtime.import_library_archive().unwrap();
    settle(&mut runtime);
    assert_imported(&runtime, &archive);
}

#[test]
fn choosing_another_galaxy_takes_the_replacement_gate_again() {
    let store = Shared::default();
    let (mut runtime, adapter) = library(&store);
    runtime.close_library();
    runtime.start_new_workshop(6).unwrap();
    runtime.update(Duration::ZERO);
    runtime.open_library().unwrap();
    settle(&mut runtime);
    adapter.set_import_bytes(Some(Box::from(&b"{}"[..])));
    runtime.import_library_archive().unwrap();
    settle(&mut runtime);
    let refused = LibrarySlotsStatus::Failed {
        kind: SlotRequestKind::ChooseArchive,
        slot: None,
        code: ClientDiagnosticCode::Archive,
    };
    assert_eq!(runtime.library_slots_status(), refused);
    make_dirty(&mut runtime);
    runtime.update(Duration::ZERO);
    assert!(runtime.resident_workshop_blocks_replacement());
    assert_eq!(
        runtime.retry_library_slot_request(),
        Err(ClientRuntimeError::RouteUnavailable)
    );
    assert_eq!(
        runtime.library_slots_status(),
        refused,
        "the decision stays"
    );
    assert_eq!(adapter.started().len(), 1);
}
