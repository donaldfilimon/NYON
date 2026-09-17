//! The transfer strip's routes: route-design task 11.
//!
//! Export this galaxy and Export content pack prepare bytes to Ready with no
//! adapter installed (the design's "Deferrals" section), and hand off through
//! the same Save copy as row Export. Import content pack asks the adapter for
//! a file and stores it through the retryable catalog-import machine (§6).
//! Each test pins one rule at the runtime seam, with `ScriptedTransfer`
//! standing in for a platform; no live picker or download is exercised here.

use std::{cell::RefCell, rc::Rc, time::Duration};

use nyon::{
    app::{
        AppCore,
        client_runtime::{
            ClientDiagnosticCode, ClientRuntime, ClientRuntimeError, ClientScreen, ExportSource,
            LibrarySlotsStatus, SlotRequestKind,
        },
        transfer::{
            HandoffOutcome, ScriptedReply, ScriptedTransfer, SuggestedName, TransferFailureCode,
            TransferKind, TransferRequest,
        },
    },
    preferences::store::MemoryPreferencesStore,
    scenario::{ScenarioDraft, store::MemoryScenarioStore},
    workshop::{
        BatchLocalId, CatalogHash, CreatorBatchV1, CreatorOpV1, GalaxyPointV1, ObjectName,
        WorkshopHistory, decode_catalog_pack, encode_archive, encode_catalog_pack,
        session::WorkshopAction,
        store::{
            MAX_WORKSHOP_PACK_BYTES, MemoryWorkshopStore, SlotName, StoreJobId, StoreJobState,
            WorkshopStore, WorkshopStoreError, WorkshopStoreRequest, WorkshopStoreResult,
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

fn stored_slot_count(store: &Shared) -> usize {
    let WorkshopStoreResult::Slots(list) = complete(store, WorkshopStoreRequest::ListSlots) else {
        panic!("ListSlots answered with the wrong shape");
    };
    list.slots.len()
}

fn core_hash() -> CatalogHash {
    decode_catalog_pack(CORE_PACK).unwrap().catalog_hash()
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

/// A runtime on the Library over a fresh, slotless Workshop.
fn open_workshop(store: &Shared) -> Runtime {
    let mut runtime = runtime(store);
    runtime.start_new_workshop(7).unwrap();
    runtime.open_library().unwrap();
    settle(&mut runtime);
    assert_eq!(runtime.screen(), ClientScreen::Library);
    runtime
}

fn install(runtime: &mut Runtime) -> ScriptedTransfer {
    let adapter = ScriptedTransfer::default();
    runtime
        .install_transfer_adapter(Box::new(adapter.clone()))
        .unwrap();
    adapter
}

fn make_dirty(runtime: &mut Runtime) {
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

fn diagnostics(runtime: &Runtime, code: ClientDiagnosticCode) -> usize {
    runtime
        .diagnostics()
        .filter(|diagnostic| diagnostic.code == code)
        .count()
}

// ---------------------------------------------------------------------------
// Export content pack
// ---------------------------------------------------------------------------

#[test]
fn exporting_the_content_pack_prepares_its_canonical_bytes_without_an_adapter() {
    let store = Shared::default();
    let mut runtime = open_workshop(&store);
    let packs = stored_packs(&store);
    assert!(!runtime.transfer_available());

    runtime.export_active_pack().unwrap();
    let hash = core_hash();
    assert_eq!(
        runtime.library_slots_status(),
        LibrarySlotsStatus::ExportReady {
            source: ExportSource::ActivePack { hash },
        }
    );
    let prepared = runtime.prepared_export().expect("pack bytes are ready");
    assert_eq!(prepared.kind(), TransferKind::ContentPack);
    assert_eq!(prepared.bytes, runtime.export_active_catalog().unwrap());
    assert_eq!(
        prepared.bytes.as_ref(),
        encode_catalog_pack(&decode_catalog_pack(CORE_PACK).unwrap())
            .unwrap()
            .as_slice()
    );
    assert_eq!(prepared.suggested_name, SuggestedName::content_pack(hash));
    assert_eq!(prepared.source.slot(), None);

    // Stage 2 still needs an adapter, and refusing it keeps the bytes.
    assert_eq!(
        runtime.hand_off_library_export(),
        Err(ClientRuntimeError::TransferUnavailable)
    );
    assert!(runtime.prepared_export().is_some());

    let adapter = install(&mut runtime);
    runtime.hand_off_library_export().unwrap();
    settle(&mut runtime);
    assert_eq!(
        runtime.library_slots_status(),
        LibrarySlotsStatus::ExportHandedOff {
            source: ExportSource::ActivePack { hash },
            outcome: HandoffOutcome::DownloadStarted,
        }
    );
    let handed = adapter.handed_off();
    assert_eq!(handed.len(), 1);
    assert_eq!(handed[0].kind, TransferKind::ContentPack);
    assert_eq!(
        handed[0].suggested_name.as_str(),
        format!(
            "{}.nyonpack.json",
            hash.0[..8]
                .iter()
                .map(|byte| format!("{byte:02x}"))
                .collect::<String>()
        )
    );
    assert_eq!(handed[0].bytes, runtime.export_active_catalog().unwrap());
    // An export stores nothing.
    assert_eq!(stored_packs(&store), packs);
    assert_eq!(stored_slot_count(&store), 0);
    assert_eq!(runtime.screen(), ClientScreen::Library);
}

#[test]
fn active_exports_need_an_open_workshop_and_a_free_lane() {
    let store = Shared::default();
    let mut runtime = runtime(&store);
    runtime.open_library().unwrap();
    settle(&mut runtime);
    for result in [
        runtime.export_active_pack(),
        runtime.export_active_workshop(),
    ] {
        assert_eq!(result, Err(ClientRuntimeError::WorkshopInactive));
    }
    assert_eq!(runtime.library_slots_status(), LibrarySlotsStatus::Idle);
    assert_eq!(
        diagnostics(&runtime, ClientDiagnosticCode::WorkshopInactive),
        2
    );

    let mut runtime = open_workshop(&store);
    runtime.export_active_pack().unwrap();
    let held = runtime.library_slots_status();
    assert_eq!(
        runtime.export_active_workshop(),
        Err(ClientRuntimeError::LibraryRequestActive)
    );
    assert_eq!(
        runtime.export_active_pack(),
        Err(ClientRuntimeError::LibraryRequestActive)
    );
    assert_eq!(runtime.library_slots_status(), held);
}

// ---------------------------------------------------------------------------
// Export this galaxy
// ---------------------------------------------------------------------------

#[test]
fn exporting_this_galaxy_uses_the_session_export_and_says_it_is_not_saved() {
    let store = Shared::default();
    let mut runtime = open_workshop(&store);
    make_dirty(&mut runtime);
    assert!(runtime.resident_workshop_blocks_replacement());
    let digest = runtime.workshop_snapshot().unwrap().state_digest;
    let expected = encode_archive(match runtime.active_session() {
        nyon::app::client_runtime::ActiveSession::Workshop(session) => session.history(),
        _ => unreachable!("a Workshop is open"),
    })
    .unwrap()
    .bytes
    .into_boxed_slice();

    // Not gated on the unsaved resident: nothing is replaced.
    runtime.export_active_workshop().unwrap();
    assert_eq!(
        runtime.library_slots_status(),
        LibrarySlotsStatus::Working {
            kind: SlotRequestKind::ExportWorkshop,
            slot: None,
        }
    );
    assert!(runtime.prepared_export().is_none());
    runtime.update(Duration::ZERO);
    let unsaved = ExportSource::ActiveWorkshop {
        continue_ready: false,
    };
    assert_eq!(
        runtime.library_slots_status(),
        LibrarySlotsStatus::ExportReady { source: unsaved }
    );
    assert_eq!(
        unsaved.label(),
        "Portable export; Workshop not saved for Continue"
    );
    let prepared = runtime.prepared_export().unwrap();
    assert_eq!(prepared.bytes, expected, "the dirty in-memory state (§10)");
    assert_eq!(prepared.kind(), TransferKind::WorkshopArchive);
    assert_eq!(
        prepared.suggested_name.as_str(),
        "Workshop.nyonworkshop.json"
    );

    // Exporting changed nothing about the Workshop or the store.
    let snapshot = runtime.workshop_snapshot().unwrap();
    assert_eq!(snapshot.state_digest, digest);
    assert!(snapshot.store.dirty);
    assert_eq!(snapshot.store.slot, None);
    assert_eq!(stored_slot_count(&store), 0);
    assert_eq!(runtime.screen(), ClientScreen::Library);

    let adapter = install(&mut runtime);
    runtime.hand_off_library_export().unwrap();
    settle(&mut runtime);
    assert_eq!(
        runtime.library_slots_status(),
        LibrarySlotsStatus::ExportHandedOff {
            source: unsaved,
            outcome: HandoffOutcome::DownloadStarted,
        }
    );
    assert_eq!(adapter.handed_off()[0].bytes, expected);
    assert_eq!(adapter.handed_off()[0].kind, TransferKind::WorkshopArchive);
}

#[test]
fn a_saved_continue_workshop_exports_under_its_own_name_without_the_qualifier() {
    let store = Shared::default();
    let catalog = decode_catalog_pack(CORE_PACK).unwrap();
    let archive = encode_archive(&WorkshopHistory::from_seed_u64(catalog, 11))
        .unwrap()
        .bytes
        .into_boxed_slice();
    let WorkshopStoreResult::SlotCreated { slot, generation } = complete(
        &store,
        WorkshopStoreRequest::CreateSlot {
            name: SlotName::new("First Forge").unwrap(),
            archive: archive.clone(),
        },
    ) else {
        panic!("unexpected create result");
    };
    let mut runtime = runtime(&store);
    runtime.open_library().unwrap();
    settle(&mut runtime);
    runtime.open_library_slot(slot, generation).unwrap();
    settle(&mut runtime);
    assert_eq!(runtime.screen(), ClientScreen::GalaxyWorkshop);
    for _ in 0..8 {
        runtime.update(Duration::ZERO);
    }
    runtime.open_library().unwrap();
    settle(&mut runtime);

    runtime.export_active_workshop().unwrap();
    runtime.update(Duration::ZERO);
    let source = ExportSource::ActiveWorkshop {
        continue_ready: true,
    };
    assert_eq!(
        runtime.library_slots_status(),
        LibrarySlotsStatus::ExportReady { source }
    );
    assert_eq!(source.qualifier(), None);
    let prepared = runtime.prepared_export().unwrap();
    assert_eq!(prepared.bytes, archive);
    assert_eq!(
        prepared.suggested_name.as_str(),
        "First-Forge.nyonworkshop.json"
    );
}

#[test]
fn a_refused_workshop_export_is_a_retryable_library_failure() {
    let store = Shared::default();
    let catalog = decode_catalog_pack(CORE_PACK).unwrap();
    let WorkshopStoreResult::SlotCreated { slot, .. } = complete(
        &store,
        WorkshopStoreRequest::CreateSlot {
            name: SlotName::new("First Forge").unwrap(),
            archive: encode_archive(&WorkshopHistory::from_seed_u64(catalog, 11))
                .unwrap()
                .bytes
                .into_boxed_slice(),
        },
    ) else {
        panic!("unexpected create result");
    };
    let mut runtime = open_workshop(&store);
    // A load queued ahead of the export makes the session refuse it: an
    // export is unavailable while a replacement is active.
    runtime
        .enqueue_workshop_action(WorkshopAction::RequestLoad(slot))
        .unwrap();
    runtime.export_active_workshop().unwrap();
    runtime.update(Duration::ZERO);
    assert_eq!(
        runtime.library_slots_status(),
        LibrarySlotsStatus::Failed {
            kind: SlotRequestKind::ExportWorkshop,
            slot: None,
            code: ClientDiagnosticCode::Archive,
        }
    );
    assert!(runtime.prepared_export().is_none());
    assert_eq!(runtime.screen(), ClientScreen::Library);
    assert!(runtime.recovery_diagnostic().is_none());

    // Once the load has finished, Retry exports what is open then.
    for _ in 0..16 {
        runtime.update(Duration::ZERO);
    }
    runtime.retry_library_slot_request().unwrap();
    runtime.update(Duration::ZERO);
    assert!(matches!(
        runtime.library_slots_status(),
        LibrarySlotsStatus::ExportReady {
            source: ExportSource::ActiveWorkshop { .. }
        }
    ));

    // Cancel from a failure frees the lane.
    runtime.cancel_library_slot_request().unwrap();
    runtime
        .enqueue_workshop_action(WorkshopAction::RequestLoad(slot))
        .unwrap();
    runtime.export_active_workshop().unwrap();
    runtime.update(Duration::ZERO);
    runtime.cancel_library_slot_request().unwrap();
    assert_eq!(runtime.library_slots_status(), LibrarySlotsStatus::Idle);
}

#[test]
fn a_cancelled_queued_export_is_never_offered() {
    let store = Shared::default();
    let mut runtime = open_workshop(&store);
    runtime.export_active_workshop().unwrap();
    runtime.cancel_library_slot_request().unwrap();
    assert_eq!(runtime.library_slots_status(), LibrarySlotsStatus::Idle);
    runtime.update(Duration::ZERO);
    assert_eq!(runtime.library_slots_status(), LibrarySlotsStatus::Idle);
    assert!(runtime.prepared_export().is_none());

    // The session did encode the abandoned request; the next export must
    // answer with its own bytes, taken after the state changes.
    make_dirty(&mut runtime);
    runtime.export_active_workshop().unwrap();
    runtime.update(Duration::ZERO);
    let fresh = encode_archive(match runtime.active_session() {
        nyon::app::client_runtime::ActiveSession::Workshop(session) => session.history(),
        _ => unreachable!("a Workshop is open"),
    })
    .unwrap()
    .bytes
    .into_boxed_slice();
    assert_eq!(runtime.prepared_export().unwrap().bytes, fresh);

    // A stale archive must never answer a request the session refused.
    runtime.cancel_library_slot_request().unwrap();
    runtime.export_active_workshop().unwrap();
    runtime.cancel_library_slot_request().unwrap();
    runtime.update(Duration::ZERO);
    let catalog = decode_catalog_pack(CORE_PACK).unwrap();
    let WorkshopStoreResult::SlotCreated { slot, .. } = complete(
        &store,
        WorkshopStoreRequest::CreateSlot {
            name: SlotName::new("Elsewhere").unwrap(),
            archive: encode_archive(&WorkshopHistory::from_seed_u64(catalog, 5))
                .unwrap()
                .bytes
                .into_boxed_slice(),
        },
    ) else {
        panic!("unexpected create result");
    };
    // Saving first, so the load is not refused as a replacement of unsaved work.
    for _ in 0..64 {
        runtime.update(Duration::from_millis(300));
        if !runtime.resident_workshop_blocks_replacement() {
            break;
        }
    }
    assert!(!runtime.resident_workshop_blocks_replacement());
    runtime
        .enqueue_workshop_action(WorkshopAction::RequestLoad(slot))
        .unwrap();
    runtime.export_active_workshop().unwrap();
    runtime.update(Duration::ZERO);
    assert!(
        matches!(
            runtime.library_slots_status(),
            LibrarySlotsStatus::Failed {
                kind: SlotRequestKind::ExportWorkshop,
                ..
            }
        ),
        "{:?}",
        runtime.library_slots_status()
    );
    assert!(runtime.prepared_export().is_none());
}

// ---------------------------------------------------------------------------
// Import content pack
// ---------------------------------------------------------------------------

fn pack_runtime(store: &Shared) -> (Runtime, ScriptedTransfer) {
    let mut runtime = runtime(store);
    runtime.open_library().unwrap();
    settle(&mut runtime);
    let adapter = install(&mut runtime);
    (runtime, adapter)
}

#[test]
fn importing_a_content_pack_chooses_validates_and_stores_it() {
    let store = Shared::default();
    let (mut runtime, adapter) = pack_runtime(&store);
    adapter.set_import_bytes(Some(Box::from(CORE_PACK)));
    assert!(stored_packs(&store).is_empty());

    runtime.import_library_pack().unwrap();
    assert_eq!(
        runtime.library_slots_status(),
        LibrarySlotsStatus::Working {
            kind: SlotRequestKind::ChoosePack,
            slot: None,
        }
    );
    assert_eq!(
        adapter.started(),
        vec![TransferRequest::ChooseImport {
            kind: TransferKind::ContentPack,
            max_bytes: MAX_WORKSHOP_PACK_BYTES,
        }]
    );
    // The lane is held: nothing else may start meanwhile.
    assert_eq!(
        runtime.refresh_library_slots(),
        Err(ClientRuntimeError::LibraryRequestActive)
    );
    assert_eq!(
        runtime.install_transfer_adapter(Box::new(ScriptedTransfer::default())),
        Err(ClientRuntimeError::LibraryRequestActive),
        "the adapter owns the choice"
    );

    let hash = core_hash();
    settle(&mut runtime);
    assert_eq!(
        runtime.library_slots_status(),
        LibrarySlotsStatus::PackStored { hash }
    );
    assert_eq!(stored_packs(&store), vec![hash]);
    assert_eq!(runtime.imported_catalog_hash(), Some(hash));
    // Storing a pack requests no session and changes no screen.
    assert!(runtime.workshop_snapshot().is_none());
    assert_eq!(runtime.screen(), ClientScreen::Library);

    // Done frees the lane; the pack stays stored.
    runtime.cancel_library_slot_request().unwrap();
    assert_eq!(runtime.library_slots_status(), LibrarySlotsStatus::Idle);
    assert_eq!(stored_packs(&store), vec![hash]);
    runtime.refresh_library_slots().unwrap();
}

#[test]
fn importing_a_pack_is_not_gated_on_an_unsaved_resident() {
    let store = Shared::default();
    let mut runtime = open_workshop(&store);
    make_dirty(&mut runtime);
    let digest = runtime.workshop_snapshot().unwrap().state_digest;
    let adapter = install(&mut runtime);
    adapter.set_import_bytes(Some(Box::from(CORE_PACK)));
    runtime.import_library_pack().unwrap();
    for _ in 0..8 {
        runtime.update(Duration::ZERO);
    }
    assert_eq!(
        runtime.library_slots_status(),
        LibrarySlotsStatus::PackStored { hash: core_hash() }
    );
    let snapshot = runtime.workshop_snapshot().unwrap();
    assert_eq!(snapshot.state_digest, digest);
    assert!(snapshot.store.dirty);
    assert_eq!(runtime.screen(), ClientScreen::Library);
}

#[test]
fn a_dismissed_picker_is_ordinary() {
    let store = Shared::default();
    let (mut runtime, adapter) = pack_runtime(&store);
    // No file scripted: the picker is dismissed.
    runtime.import_library_pack().unwrap();
    settle(&mut runtime);
    assert_eq!(runtime.library_slots_status(), LibrarySlotsStatus::Idle);
    assert_eq!(runtime.diagnostics().count(), 0);
    assert!(stored_packs(&store).is_empty());
    assert!(!adapter.busy());
}

#[test]
fn import_pack_needs_an_adapter_and_a_free_lane() {
    let store = Shared::default();
    let mut runtime = runtime(&store);
    runtime.open_library().unwrap();
    settle(&mut runtime);
    assert_eq!(
        runtime.import_library_pack(),
        Err(ClientRuntimeError::TransferUnavailable)
    );
    assert_eq!(runtime.library_slots_status(), LibrarySlotsStatus::Idle);

    let adapter = install(&mut runtime);
    adapter.set_pending_polls(4);
    runtime.import_library_pack().unwrap();
    assert_eq!(
        runtime.import_library_pack(),
        Err(ClientRuntimeError::LibraryRequestActive)
    );
    assert_eq!(adapter.started().len(), 1);
}

#[test]
fn an_invalid_pack_is_refused_and_chosen_again() {
    let store = Shared::default();
    let (mut runtime, adapter) = pack_runtime(&store);
    adapter.set_import_bytes(Some(Box::from(&b"{\"not\":\"a pack\"}"[..])));
    runtime.import_library_pack().unwrap();
    settle(&mut runtime);
    let refused = LibrarySlotsStatus::Failed {
        kind: SlotRequestKind::ChoosePack,
        slot: None,
        code: ClientDiagnosticCode::Catalog,
    };
    assert_eq!(runtime.library_slots_status(), refused);
    assert!(stored_packs(&store).is_empty());
    assert_eq!(runtime.screen(), ClientScreen::Library);
    assert!(runtime.recovery_diagnostic().is_none());

    // Retry is a new choice, not a resubmission of the refused bytes.
    adapter.set_import_bytes(Some(Box::from(CORE_PACK)));
    runtime.retry_library_slot_request().unwrap();
    assert_eq!(adapter.started().len(), 2);
    settle(&mut runtime);
    assert_eq!(
        runtime.library_slots_status(),
        LibrarySlotsStatus::PackStored { hash: core_hash() }
    );
}

#[test]
fn a_failed_or_misbehaving_picker_is_a_visible_transfer_failure() {
    for reply in [
        ScriptedReply::Fail(TransferFailureCode::Platform),
        ScriptedReply::Refuse(TransferFailureCode::Busy),
        ScriptedReply::WrongShape,
        ScriptedReply::Forget,
    ] {
        let store = Shared::default();
        let (mut runtime, adapter) = pack_runtime(&store);
        adapter.set_import_bytes(Some(Box::from(CORE_PACK)));
        adapter.push_reply(reply.clone());
        let started = runtime.import_library_pack();
        assert_eq!(
            started.is_err(),
            matches!(reply, ScriptedReply::Refuse(_)),
            "{reply:?}"
        );
        settle(&mut runtime);
        assert_eq!(
            runtime.library_slots_status(),
            LibrarySlotsStatus::Failed {
                kind: SlotRequestKind::ChoosePack,
                slot: None,
                code: ClientDiagnosticCode::Transfer,
            },
            "{reply:?}"
        );
        assert_eq!(
            diagnostics(&runtime, ClientDiagnosticCode::Transfer),
            1,
            "{reply:?}"
        );
        assert!(stored_packs(&store).is_empty(), "{reply:?}");
        assert!(runtime.recovery_diagnostic().is_none(), "{reply:?}");
        runtime.cancel_library_slot_request().unwrap();
        assert_eq!(runtime.library_slots_status(), LibrarySlotsStatus::Idle);
    }
}

#[test]
fn an_oversized_file_is_refused_by_the_adapter() {
    let store = Shared::default();
    let (mut runtime, adapter) = pack_runtime(&store);
    adapter.set_import_bytes(Some(
        vec![b' '; MAX_WORKSHOP_PACK_BYTES + 1].into_boxed_slice(),
    ));
    runtime.import_library_pack().unwrap();
    settle(&mut runtime);
    assert_eq!(
        runtime.library_slots_status(),
        LibrarySlotsStatus::Failed {
            kind: SlotRequestKind::ChoosePack,
            slot: None,
            code: ClientDiagnosticCode::Transfer,
        }
    );
}

#[test]
fn cancelling_the_picker_abandons_its_job() {
    let store = Shared::default();
    let (mut runtime, adapter) = pack_runtime(&store);
    adapter.set_import_bytes(Some(Box::from(CORE_PACK)));
    adapter.set_pending_polls(4);
    runtime.import_library_pack().unwrap();
    runtime.update(Duration::ZERO);
    runtime.cancel_library_slot_request().unwrap();
    assert_eq!(adapter.abandoned(), 1);
    for _ in 0..8 {
        runtime.update(Duration::ZERO);
    }
    assert_eq!(runtime.library_slots_status(), LibrarySlotsStatus::Idle);
    assert!(stored_packs(&store).is_empty(), "the file is never read");
    assert_eq!(runtime.diagnostics().count(), 0);
}

#[test]
fn a_pack_store_failure_offers_retry_and_cancel_and_never_recovery() {
    let store = Shared::default();
    let (mut runtime, adapter) = pack_runtime(&store);
    adapter.set_import_bytes(Some(Box::from(CORE_PACK)));
    // Hold the store's Commit lane, so `PutPack` is refused.
    let holder = store
        .inner
        .borrow_mut()
        .start(WorkshopStoreRequest::CreateSlot {
            name: SlotName::new("Holder").unwrap(),
            archive: encode_archive(&WorkshopHistory::from_seed_u64(
                decode_catalog_pack(CORE_PACK).unwrap(),
                3,
            ))
            .unwrap()
            .bytes
            .into_boxed_slice(),
        })
        .unwrap();

    runtime.import_library_pack().unwrap();
    settle(&mut runtime);
    let failed = LibrarySlotsStatus::Failed {
        kind: SlotRequestKind::StorePack,
        slot: None,
        code: ClientDiagnosticCode::Store,
    };
    assert_eq!(runtime.library_slots_status(), failed);
    assert_eq!(runtime.screen(), ClientScreen::Library);
    assert!(runtime.recovery_diagnostic().is_none());
    for _ in 0..4 {
        runtime.update(Duration::ZERO);
    }
    assert_eq!(runtime.library_slots_status(), failed, "a decision waits");

    // Retry while the lane is still held fails the same way, retained.
    assert!(runtime.retry_library_slot_request().is_err());
    assert_eq!(runtime.library_slots_status(), failed);

    // Free the lane; Retry stores the retained pack without a new choice.
    assert!(matches!(
        store.inner.borrow_mut().poll(holder),
        StoreJobState::Complete(Ok(_))
    ));
    runtime.retry_library_slot_request().unwrap();
    assert_eq!(adapter.started().len(), 1, "no file is chosen again");
    settle(&mut runtime);
    assert_eq!(
        runtime.library_slots_status(),
        LibrarySlotsStatus::PackStored { hash: core_hash() }
    );
    assert_eq!(stored_packs(&store), vec![core_hash()]);
}

#[test]
fn cancel_never_deletes_a_pack_that_already_stored() {
    let store = Shared::default();
    let (mut runtime, adapter) = pack_runtime(&store);
    adapter.set_import_bytes(Some(Box::from(CORE_PACK)));
    runtime.import_library_pack().unwrap();
    // Deliver the file; the memory store executes `PutPack` when it starts.
    runtime.update(Duration::ZERO);
    assert_eq!(
        runtime.library_slots_status(),
        LibrarySlotsStatus::Working {
            kind: SlotRequestKind::StorePack,
            slot: None,
        }
    );
    runtime.cancel_library_slot_request().unwrap();
    assert_eq!(runtime.library_slots_status(), LibrarySlotsStatus::Idle);
    for _ in 0..4 {
        runtime.update(Duration::ZERO);
    }
    assert_eq!(runtime.library_slots_status(), LibrarySlotsStatus::Idle);
    assert_eq!(
        stored_packs(&store),
        vec![core_hash()],
        "the abandoned write landed and stays"
    );

    // A later import of the same pack is accepted and stores idempotently.
    runtime.import_library_pack().unwrap();
    settle(&mut runtime);
    assert_eq!(
        runtime.library_slots_status(),
        LibrarySlotsStatus::PackStored { hash: core_hash() }
    );
    runtime.cancel_library_slot_request().unwrap();
    assert_eq!(stored_packs(&store), vec![core_hash()]);
}

#[test]
fn a_pack_import_cancelled_outside_the_library_frees_the_lane() {
    let store = Shared::default();
    let (mut runtime, adapter) = pack_runtime(&store);
    adapter.set_import_bytes(Some(Box::from(CORE_PACK)));
    runtime.import_library_pack().unwrap();
    runtime.update(Duration::ZERO);
    assert_eq!(
        runtime.library_slots_status(),
        LibrarySlotsStatus::Working {
            kind: SlotRequestKind::StorePack,
            slot: None,
        }
    );
    // The machine's own cancel, not the Library's.
    runtime.cancel_catalog_import().unwrap();
    assert_eq!(runtime.library_slots_status(), LibrarySlotsStatus::Idle);
    runtime.update(Duration::ZERO);
    // Status and lane agree: a new request is accepted.
    runtime.refresh_library_slots().unwrap();
    settle(&mut runtime);
    assert_eq!(runtime.library_slots_status(), LibrarySlotsStatus::Idle);
}
