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
    run(
        &mut store,
        WorkshopStoreRequest::SelectContinue {
            slot,
            expected_generation: SaveGeneration(1),
        },
    )
    .unwrap();
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
fn model_unarchive_clears_only_the_flag_and_does_not_restore_continue() {
    let mut store = IndexedDbTransactionModel::default();
    let slot = create(&mut store);
    let second = match run(
        &mut store,
        WorkshopStoreRequest::CommitSlot {
            slot,
            expected_generation: SaveGeneration(1),
            archive: archive("two"),
        },
    )
    .unwrap()
    {
        WorkshopStoreResult::SlotCommitted { generation, .. } => generation,
        result => panic!("unexpected commit result: {result:?}"),
    };
    run(
        &mut store,
        WorkshopStoreRequest::SelectContinue {
            slot,
            expected_generation: second,
        },
    )
    .unwrap();
    let before_head = loaded(&mut store, slot);
    let before_predecessor = match run(
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

    run(&mut store, WorkshopStoreRequest::ArchiveSlot { slot }).unwrap();
    assert_eq!(
        run(&mut store, WorkshopStoreRequest::UnarchiveSlot { slot }).unwrap(),
        WorkshopStoreResult::SlotUnarchived { slot }
    );

    assert_eq!(loaded(&mut store, slot), before_head);
    assert_eq!(
        run(
            &mut store,
            WorkshopStoreRequest::LoadPreviousGeneration {
                slot,
                expected_head_generation: second,
            },
        )
        .unwrap(),
        WorkshopStoreResult::SlotLoaded(before_predecessor)
    );

    let list = match run(&mut store, WorkshopStoreRequest::ListSlots).unwrap() {
        WorkshopStoreResult::Slots(list) => list,
        result => panic!("unexpected list result: {result:?}"),
    };
    assert_eq!(list.slots.len(), 1);
    assert!(!list.slots[0].archived);
    assert!(list.slots[0].has_previous_generation);
    assert_eq!(list.slots[0].name.as_str(), "Forge");
    assert_eq!(list.slots[0].generation, second);
    assert_eq!(list.selected_continue, None);
    assert!(!list.slots[0].selected_for_continue);

    assert_eq!(
        run(&mut store, WorkshopStoreRequest::UnarchiveSlot { slot }).unwrap(),
        WorkshopStoreResult::SlotUnarchived { slot }
    );
    assert!(matches!(
        run(
            &mut store,
            WorkshopStoreRequest::UnarchiveSlot { slot: SlotId(9) }
        ),
        Err(WorkshopStoreError::UnknownSlot { slot }) if slot == SlotId(9)
    ));
    run(
        &mut store,
        WorkshopStoreRequest::SelectContinue {
            slot,
            expected_generation: second,
        },
    )
    .unwrap();
}

/// An aborted unarchive must publish nothing, exactly like an aborted commit.
#[test]
fn model_unarchive_abort_leaves_the_slot_archived() {
    let mut store = IndexedDbTransactionModel::default();
    let slot = create(&mut store);
    run(&mut store, WorkshopStoreRequest::ArchiveSlot { slot }).unwrap();

    store.inject_next_failure(IndexedDbModelFailure::Abort);
    assert_eq!(
        run(&mut store, WorkshopStoreRequest::UnarchiveSlot { slot }),
        Err(WorkshopStoreError::IndexedDbTransactionAborted)
    );

    let list = match run(&mut store, WorkshopStoreRequest::ListSlots).unwrap() {
        WorkshopStoreResult::Slots(list) => list,
        result => panic!("unexpected list result: {result:?}"),
    };
    assert!(list.slots[0].archived);
    assert_eq!(&*loaded(&mut store, slot).archive, &*archive("one"));
}

fn list(store: &mut dyn WorkshopStore) -> nyon::workshop::store::SlotList {
    match run(store, WorkshopStoreRequest::ListSlots).unwrap() {
        WorkshopStoreResult::Slots(list) => list,
        result => panic!("unexpected list result: {result:?}"),
    }
}

fn select(
    store: &mut dyn WorkshopStore,
    slot: SlotId,
    expected_generation: SaveGeneration,
) -> Result<WorkshopStoreResult, WorkshopStoreError> {
    run(
        store,
        WorkshopStoreRequest::SelectContinue {
            slot,
            expected_generation,
        },
    )
}

/// The generation-checked Continue contract, against the browser adapter's
/// host-testable transaction model.
///
/// The model stages every request against a cloned committed state and
/// publishes it only if the transaction succeeds, so a selection refused for a
/// stale generation, and a marker cleared by a commit, are both proven to be
/// all-or-nothing here rather than merely in-memory bookkeeping. The wasm
/// implementation performs the same comparison inside its `mutate_references`
/// read-write transaction; only compilation covers that file on the host.
#[test]
fn model_continue_selection_is_generation_checked_and_cleared_by_every_new_head() {
    let mut store = IndexedDbTransactionModel::default();
    let slot = create(&mut store);
    let first = SaveGeneration(1);

    assert!(matches!(
        select(&mut store, slot, SaveGeneration(2)),
        Err(WorkshopStoreError::StaleGeneration {
            expected,
            actual,
            ..
        }) if expected == SaveGeneration(2) && actual == first
    ));
    assert_eq!(list(&mut store).selected_continue, None);

    assert_eq!(
        select(&mut store, slot, first).unwrap(),
        WorkshopStoreResult::ContinueSelected {
            slot,
            generation: first,
        }
    );
    assert_eq!(list(&mut store).selected_continue, Some(slot));

    // Listed at N, committed to N+1, then selected at N: refused, with the
    // marker already cleared by the commit that invalidated it.
    let observed = list(&mut store).slots[0].generation;
    assert_eq!(observed, first);
    let second = match run(
        &mut store,
        WorkshopStoreRequest::CommitSlot {
            slot,
            expected_generation: first,
            archive: archive("two"),
        },
    )
    .unwrap()
    {
        WorkshopStoreResult::SlotCommitted { generation, .. } => generation,
        result => panic!("unexpected commit result: {result:?}"),
    };
    assert_eq!(
        list(&mut store).selected_continue,
        None,
        "a commit must clear the Continue marker it invalidated"
    );
    assert!(matches!(
        select(&mut store, slot, observed),
        Err(WorkshopStoreError::StaleGeneration {
            expected,
            actual,
            ..
        }) if expected == observed && actual == second
    ));
    assert_eq!(list(&mut store).selected_continue, None);

    select(&mut store, slot, second).unwrap();
    assert_eq!(list(&mut store).selected_continue, Some(slot));

    // A refused selection publishes nothing, and an aborted successful one
    // publishes nothing either: the marker still names the old head.
    store.inject_next_failure(IndexedDbModelFailure::Abort);
    assert_eq!(
        select(&mut store, slot, second),
        Err(WorkshopStoreError::IndexedDbTransactionAborted)
    );
    assert_eq!(list(&mut store).selected_continue, Some(slot));

    let promoted = match run(
        &mut store,
        WorkshopStoreRequest::PromoteRecoveredSlot {
            slot,
            expected_head_generation: second,
            recovered_generation: first,
            archive: archive("promoted"),
        },
    )
    .unwrap()
    {
        WorkshopStoreResult::SlotCommitted { generation, .. } => generation,
        result => panic!("unexpected promotion result: {result:?}"),
    };
    assert_eq!(promoted, SaveGeneration(3));
    assert_eq!(
        list(&mut store).selected_continue,
        None,
        "a promotion must clear the Continue marker it invalidated"
    );
    select(&mut store, slot, promoted).unwrap();
    assert_eq!(list(&mut store).selected_continue, Some(slot));

    // Precedence: unknown outranks archived outranks stale.
    assert!(matches!(
        select(&mut store, SlotId(9), SaveGeneration(1)),
        Err(WorkshopStoreError::UnknownSlot { slot: unknown }) if unknown == SlotId(9)
    ));
    run(&mut store, WorkshopStoreRequest::ArchiveSlot { slot }).unwrap();
    assert!(matches!(
        select(&mut store, slot, SaveGeneration(999)),
        Err(WorkshopStoreError::ArchivedSlot { slot: archived }) if archived == slot
    ));
    run(&mut store, WorkshopStoreRequest::UnarchiveSlot { slot }).unwrap();
    assert!(matches!(
        select(&mut store, slot, SaveGeneration(999)),
        Err(WorkshopStoreError::StaleGeneration { .. })
    ));
}

#[test]
fn wasm_source_uses_atomic_transactions_and_bounded_diagnostics() {
    // The browser store is split across the host-visible transaction model and
    // the wasm-only implementation, so this contract reads both: a required
    // substring must appear somewhere in the browser store's source, and a
    // forbidden one must appear in none of it. Reading a single file would let
    // a future move silently retire the contract rather than fail it.
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let source = [
        root.join("src/workshop/store/web.rs"),
        root.join("src/workshop/store/web/wasm.rs"),
    ]
    .into_iter()
    .map(|path| fs::read_to_string(&path).expect("browser store source is readable"))
    .collect::<Vec<_>>()
    .join("\n");

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
    // Counted contracts, for invariants that need *every* site rather than one.
    //
    // `contains` is the wrong tool when a rule must hold at more than one call
    // site: it passes while a site is deleted. Clear-on-new-head is exactly
    // that shape -- `commit_slot` and `promote_recovered_slot` must each clear
    // any marker naming the slot, inside their own transaction. Memory and
    // native are covered behaviourally; the browser arm is not, because
    // `store/web/wasm.rs` executes nowhere in this repository. The only
    // accidental protection is that the helper is private, so deleting *both*
    // sites makes it dead code and trips wasm clippy -- deleting *one* trips
    // nothing at all.
    //
    // Still not caught here, and stated rather than implied: hoisting a call
    // out of its `mutate_references` closure. That is the atomicity property
    // the doc comments claim, and no test, lint or source contract constrains
    // where the call sits.
    // Written as a direct assertion rather than a one-element loop, which
    // clippy rejects as `single_element_loop`. Turn it back into a loop when a
    // second counted contract joins it.
    const CLEAR_CONTINUE_CALL: &str = "clear_continue_for_sync(references, slots_store, slot)?";
    let clear_continue_sites = source.matches(CLEAR_CONTINUE_CALL).count();
    assert_eq!(
        clear_continue_sites, 2,
        "clear-on-new-head must be called from both commit_slot and \
         promote_recovered_slot; found {clear_continue_sites} call site(s)"
    );

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
