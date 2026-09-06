use std::{fs, path::PathBuf};

use nyon::workshop::store::{
    SaveGeneration, SlotId, SlotName, StoreJobState, WorkshopStore, WorkshopStoreError,
    WorkshopStoreRequest, WorkshopStoreResult,
    web::{
        ARCHIVES_OBJECT_STORE, INDEXED_DB_NAME, INDEXED_DB_VERSION, IndexedDbModelFailure,
        IndexedDbTransactionModel, PACKS_OBJECT_STORE, SLOTS_OBJECT_STORE,
    },
};

fn archive(label: &str) -> Box<[u8]> {
    format!(r#"{{"revision":"{label}"}}"#)
        .into_bytes()
        .into_boxed_slice()
}

fn run(
    store: &mut dyn WorkshopStore,
    request: WorkshopStoreRequest,
) -> Result<WorkshopStoreResult, WorkshopStoreError> {
    let job = store.start(request)?;
    match store.poll(job) {
        StoreJobState::Complete(result) => result,
        state => panic!("expected completed storage job, got {state:?}"),
    }
}

fn create(store: &mut dyn WorkshopStore) -> SlotId {
    match run(
        store,
        WorkshopStoreRequest::CreateSlot {
            name: SlotName::new("Forge").unwrap(),
            archive: archive("one"),
        },
    )
    .unwrap()
    {
        WorkshopStoreResult::SlotCreated { slot, generation } => {
            assert_eq!(generation, SaveGeneration(1));
            slot
        }
        result => panic!("unexpected create result: {result:?}"),
    }
}

fn loaded(store: &mut dyn WorkshopStore, slot: SlotId) -> nyon::workshop::store::LoadedSlot {
    match run(store, WorkshopStoreRequest::LoadSlot { slot }).unwrap() {
        WorkshopStoreResult::SlotLoaded(loaded) => loaded,
        result => panic!("unexpected load result: {result:?}"),
    }
}

#[test]
fn indexeddb_identity_and_object_store_names_are_frozen() {
    assert_eq!(INDEXED_DB_NAME, "nyon.workshop.v1");
    assert_eq!(INDEXED_DB_VERSION, 1);
    assert_eq!(SLOTS_OBJECT_STORE, "slots");
    assert_eq!(ARCHIVES_OBJECT_STORE, "archives");
    assert_eq!(PACKS_OBJECT_STORE, "packs");
}

#[test]
fn transaction_model_publishes_only_after_success() {
    let mut store = IndexedDbTransactionModel::default();
    let slot = create(&mut store);

    store.inject_next_failure(IndexedDbModelFailure::Abort);
    assert_eq!(
        run(
            &mut store,
            WorkshopStoreRequest::CommitSlot {
                slot,
                expected_generation: SaveGeneration(1),
                archive: archive("aborted"),
            },
        ),
        Err(WorkshopStoreError::IndexedDbTransactionAborted)
    );
    let after_abort = loaded(&mut store, slot);
    assert_eq!(after_abort.generation, SaveGeneration(1));
    assert_eq!(&*after_abort.archive, &*archive("one"));

    assert_eq!(
        run(
            &mut store,
            WorkshopStoreRequest::CommitSlot {
                slot,
                expected_generation: SaveGeneration(1),
                archive: archive("two"),
            },
        )
        .unwrap(),
        WorkshopStoreResult::SlotCommitted {
            slot,
            generation: SaveGeneration(2),
        }
    );
    let committed = loaded(&mut store, slot);
    assert_eq!(committed.generation, SaveGeneration(2));
    assert_eq!(&*committed.archive, &*archive("two"));
}

#[test]
fn browser_failure_classes_preserve_the_committed_generation() {
    for failure in [
        IndexedDbModelFailure::QuotaDenied,
        IndexedDbModelFailure::Unavailable,
        IndexedDbModelFailure::SchemaMismatch,
        IndexedDbModelFailure::Evicted,
    ] {
        let mut store = IndexedDbTransactionModel::default();
        let slot = create(&mut store);
        store.inject_next_failure(failure);
        let result = run(
            &mut store,
            WorkshopStoreRequest::CommitSlot {
                slot,
                expected_generation: SaveGeneration(1),
                archive: archive("not-published"),
            },
        );
        let expected = match failure {
            IndexedDbModelFailure::QuotaDenied => WorkshopStoreError::IndexedDbQuotaDenied,
            IndexedDbModelFailure::Unavailable => WorkshopStoreError::IndexedDbUnavailable,
            IndexedDbModelFailure::SchemaMismatch => WorkshopStoreError::IndexedDbSchemaMismatch,
            IndexedDbModelFailure::Evicted => WorkshopStoreError::IndexedDbEvicted,
            IndexedDbModelFailure::Abort => unreachable!("abort is covered separately"),
        };
        assert_eq!(result, Err(expected));
        let preserved = loaded(&mut store, slot);
        assert_eq!(preserved.generation, SaveGeneration(1));
        assert_eq!(&*preserved.archive, &*archive("one"));
    }
}

#[test]
fn promotion_abort_and_quota_failure_preserve_head_and_exact_recovered_predecessor() {
    for (failure, expected_error) in [
        (
            IndexedDbModelFailure::Abort,
            WorkshopStoreError::IndexedDbTransactionAborted,
        ),
        (
            IndexedDbModelFailure::QuotaDenied,
            WorkshopStoreError::IndexedDbQuotaDenied,
        ),
    ] {
        let mut store = IndexedDbTransactionModel::default();
        let slot = create(&mut store);
        let second = match run(
            &mut store,
            WorkshopStoreRequest::CommitSlot {
                slot,
                expected_generation: SaveGeneration(1),
                archive: archive("authority-invalid-head"),
            },
        )
        .unwrap()
        {
            WorkshopStoreResult::SlotCommitted { generation, .. } => generation,
            result => panic!("unexpected commit result: {result:?}"),
        };

        store.inject_next_failure(failure);
        assert_eq!(
            run(
                &mut store,
                WorkshopStoreRequest::PromoteRecoveredSlot {
                    slot,
                    expected_head_generation: second,
                    recovered_generation: SaveGeneration(1),
                    archive: archive("promoted"),
                },
            ),
            Err(expected_error)
        );
        assert_eq!(loaded(&mut store, slot).generation, second);
        let previous = match run(
            &mut store,
            WorkshopStoreRequest::LoadPreviousGeneration {
                slot,
                expected_head_generation: second,
            },
        )
        .unwrap()
        {
            WorkshopStoreResult::SlotLoaded(loaded) => loaded,
            result => panic!("unexpected previous load result: {result:?}"),
        };
        assert_eq!(previous.generation, SaveGeneration(1));
        assert_eq!(&*previous.archive, &*archive("one"));

        let promoted = match run(
            &mut store,
            WorkshopStoreRequest::PromoteRecoveredSlot {
                slot,
                expected_head_generation: second,
                recovered_generation: SaveGeneration(1),
                archive: archive("promoted"),
            },
        )
        .unwrap()
        {
            WorkshopStoreResult::SlotCommitted { generation, .. } => generation,
            result => panic!("unexpected promotion result: {result:?}"),
        };
        assert_eq!(promoted, SaveGeneration(3));
        let retained = match run(
            &mut store,
            WorkshopStoreRequest::LoadPreviousGeneration {
                slot,
                expected_head_generation: promoted,
            },
        )
        .unwrap()
        {
            WorkshopStoreResult::SlotLoaded(loaded) => loaded,
            result => panic!("unexpected retained load result: {result:?}"),
        };
        assert_eq!(retained.generation, SaveGeneration(1));
        assert_eq!(&*retained.archive, &*archive("one"));
    }
}

#[test]
fn model_keeps_continue_explicit_and_archive_non_destructive() {
    let mut store = IndexedDbTransactionModel::default();
    let slot = create(&mut store);
    run(&mut store, WorkshopStoreRequest::SelectContinue { slot }).unwrap();
    run(&mut store, WorkshopStoreRequest::ArchiveSlot { slot }).unwrap();

    let list = match run(&mut store, WorkshopStoreRequest::ListSlots).unwrap() {
        WorkshopStoreResult::Slots(list) => list,
        result => panic!("unexpected list result: {result:?}"),
    };
    assert_eq!(list.selected_continue, None);
    assert_eq!(list.slots.len(), 1);
    assert!(list.slots[0].archived);
    assert!(!list.slots[0].selected_for_continue);
    assert_eq!(&*loaded(&mut store, slot).archive, &*archive("one"));
}

#[test]
fn wasm_source_uses_atomic_transactions_and_bounded_diagnostics() {
    let source = fs::read_to_string(
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("src/workshop/store/web.rs"),
    )
    .unwrap();

    for contract in [
        "open_with_u32(INDEXED_DB_NAME, INDEXED_DB_VERSION)",
        "SLOTS_OBJECT_STORE, ARCHIVES_OBJECT_STORE",
        "transaction_with_str_sequence_and_mode",
        "IdbTransactionMode::Readwrite",
        "mutate_references(",
        "put_bytes_sync(",
        "write_slot_sync(",
        "set_oncomplete(",
        "set_onabort(",
        "set_onupgradeneeded(",
        "set_onversionchange(",
        "database.object_store_names()",
        "get_all_with_key_and_limit(",
        "get_all_keys_with_key_and_limit(",
        "MAX_REFERENCE_RECORD_BYTES",
        "MAX_WORKSHOP_PACKS + 1",
        "wasm_bindgen_futures::spawn_local",
    ] {
        assert!(
            source.contains(contract),
            "missing browser contract: {contract}"
        );
    }
    for forbidden in [
        "local_storage(",
        "console.log",
        "log::error!",
        "format!(\"IndexedDB request failed: {",
        ".get_all()",
        ".get_all_keys()",
    ] {
        assert!(
            !source.contains(forbidden),
            "browser storage must not expose content through {forbidden}"
        );
    }
}
