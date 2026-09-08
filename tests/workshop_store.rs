#![cfg(not(target_arch = "wasm32"))]

use std::{
    path::Path,
    sync::{Arc, Condvar, Mutex, mpsc},
    time::{Duration, Instant},
};

use nyon::workshop::store::{
    LoadedSlot, MAX_WORKSHOP_ARCHIVE_BYTES, MAX_WORKSHOP_PACKS, MemoryWorkshopStore,
    NativeStoreFaultInjector, NativeStoreFaultPoint, NativeWorkshopPlatform, NativeWorkshopStore,
    SaveGeneration, SlotId, SlotList, SlotName, StoreJobClass, StoreJobId, StoreJobState,
    WorkshopPathEnvironment, WorkshopStore, WorkshopStoreError, WorkshopStoreRequest,
    WorkshopStoreResult, native_workshop_root,
};

fn archive(version: u32) -> Box<[u8]> {
    format!(r#"{{"format":"workshop-test","version":{version}}}"#)
        .into_bytes()
        .into_boxed_slice()
}

fn run(
    store: &mut dyn WorkshopStore,
    request: WorkshopStoreRequest,
) -> Result<WorkshopStoreResult, WorkshopStoreError> {
    let job = store.start(request)?;
    match wait_for_job(store, job) {
        StoreJobState::Complete(result) => result,
        state => panic!("local store did not finish its job: {state:?}"),
    }
}

fn wait_for_job(store: &mut dyn WorkshopStore, job: StoreJobId) -> StoreJobState {
    let deadline = Instant::now() + Duration::from_secs(10);
    loop {
        match store.poll(job) {
            state @ StoreJobState::Complete(_) => return state,
            StoreJobState::Pending if Instant::now() < deadline => std::thread::yield_now(),
            state => return state,
        }
    }
}

fn create(
    store: &mut dyn WorkshopStore,
    name: &str,
    bytes: Box<[u8]>,
) -> Result<(SlotId, SaveGeneration), WorkshopStoreError> {
    match run(
        store,
        WorkshopStoreRequest::CreateSlot {
            name: SlotName::new(name)?,
            archive: bytes,
        },
    )? {
        WorkshopStoreResult::SlotCreated { slot, generation } => Ok((slot, generation)),
        result => panic!("unexpected create result: {result:?}"),
    }
}

fn list(store: &mut dyn WorkshopStore) -> SlotList {
    match run(store, WorkshopStoreRequest::ListSlots).unwrap() {
        WorkshopStoreResult::Slots(list) => list,
        result => panic!("unexpected list result: {result:?}"),
    }
}

fn head(store: &mut dyn WorkshopStore, slot: SlotId) -> LoadedSlot {
    match run(store, WorkshopStoreRequest::LoadSlot { slot }).unwrap() {
        WorkshopStoreResult::SlotLoaded(loaded) => loaded,
        result => panic!("unexpected load result: {result:?}"),
    }
}

fn predecessor(
    store: &mut dyn WorkshopStore,
    slot: SlotId,
    expected_head_generation: SaveGeneration,
) -> LoadedSlot {
    match run(
        store,
        WorkshopStoreRequest::LoadPreviousGeneration {
            slot,
            expected_head_generation,
        },
    )
    .unwrap()
    {
        WorkshopStoreResult::SlotLoaded(loaded) => loaded,
        result => panic!("unexpected previous load result: {result:?}"),
    }
}

/// Builds a slot with a head, a retained predecessor and an explicit Continue
/// selection, so an archive/unarchive round trip has something to disturb.
fn slot_with_predecessor_and_continue(store: &mut dyn WorkshopStore) -> (SlotId, SaveGeneration) {
    let (slot, first) = create(store, "Forge", archive(1)).unwrap();
    let second = match run(
        store,
        WorkshopStoreRequest::CommitSlot {
            slot,
            expected_generation: first,
            archive: archive(2),
        },
    )
    .unwrap()
    {
        WorkshopStoreResult::SlotCommitted { generation, .. } => generation,
        result => panic!("unexpected commit result: {result:?}"),
    };
    run(
        store,
        WorkshopStoreRequest::SelectContinue {
            slot,
            expected_generation: second,
        },
    )
    .unwrap();
    (slot, second)
}

fn commit(
    store: &mut dyn WorkshopStore,
    slot: SlotId,
    expected_generation: SaveGeneration,
    bytes: Box<[u8]>,
) -> SaveGeneration {
    match run(
        store,
        WorkshopStoreRequest::CommitSlot {
            slot,
            expected_generation,
            archive: bytes,
        },
    )
    .unwrap()
    {
        WorkshopStoreResult::SlotCommitted { generation, .. } => generation,
        result => panic!("unexpected commit result: {result:?}"),
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

/// The generation-checked Continue contract, shared by every adapter.
///
/// `tests/workshop_store_web.rs` runs the same body against the IndexedDB
/// transaction model, so memory, native and the browser model are held to one
/// description of the behavior rather than three drifting copies.
///
/// The store is expected to be empty when this begins.
fn continue_selection_is_generation_checked(store: &mut dyn WorkshopStore) {
    let (slot, first) = create(store, "Forge", archive(1)).unwrap();

    // A mismatched expectation is refused, and refusal writes nothing.
    assert!(matches!(
        select(store, slot, SaveGeneration(2)),
        Err(WorkshopStoreError::StaleGeneration {
            slot: refused,
            expected,
            actual,
        }) if refused == slot && expected == SaveGeneration(2) && actual == first
    ));
    assert_eq!(list(store).selected_continue, None);

    // At the head it succeeds and reports the generation it selected against.
    assert_eq!(
        select(store, slot, first).unwrap(),
        WorkshopStoreResult::ContinueSelected {
            slot,
            generation: first,
        }
    );
    assert_eq!(list(store).selected_continue, Some(slot));

    // The race the addendum names: a row observed at generation N, a commit to
    // N+1 that lands before the selection, and a selection still carrying N.
    // Two things must hold. The commit clears the marker it invalidated, in
    // the same mutation that advanced the head, so no marker is ever left
    // naming a superseded generation. And the late selection is refused rather
    // than silently promoted onto the newer head.
    let observed = list(store).slots[0].generation;
    assert_eq!(observed, first);
    let second = commit(store, slot, first, archive(2));
    assert_eq!(
        list(store).selected_continue,
        None,
        "a commit must clear the Continue marker it invalidated"
    );
    // ⚠️ THIS ASSERTION IS THE ONLY ONE IN THE REPOSITORY THAT DISTINGUISHES A
    // COMPARE-AND-SWAP FROM A MEMBERSHIP TEST, established by mutation rather
    // than by reading: replacing the adapters' head comparison with "is this
    // generation retained?" leaves every other assertion in this shared body
    // passing, and fails here alone (plus its hand-copied twin in
    // tests/workshop_store_web.rs). `observed` is a *retained predecessor*, so
    // a membership test accepts it and a real CAS refuses it. If you weaken,
    // reorder or delete this line, the suite still reads like it covers the
    // race and no longer does.
    assert!(matches!(
        select(store, slot, observed),
        Err(WorkshopStoreError::StaleGeneration {
            expected,
            actual,
            ..
        }) if expected == observed && actual == second
    ));
    assert_eq!(list(store).selected_continue, None);

    // Selecting the generation that is now the head restores the marker.
    assert_eq!(
        select(store, slot, second).unwrap(),
        WorkshopStoreResult::ContinueSelected {
            slot,
            generation: second,
        }
    );
    assert_eq!(list(store).selected_continue, Some(slot));

    // Promotion advances the head exactly as a commit does, so it clears the
    // marker in the same mutation too.
    let promoted = match run(
        store,
        WorkshopStoreRequest::PromoteRecoveredSlot {
            slot,
            expected_head_generation: second,
            recovered_generation: first,
            archive: archive(3),
        },
    )
    .unwrap()
    {
        WorkshopStoreResult::SlotCommitted { generation, .. } => generation,
        result => panic!("unexpected promotion result: {result:?}"),
    };
    assert_eq!(promoted, SaveGeneration(3));
    assert_eq!(
        list(store).selected_continue,
        None,
        "a promotion must clear the Continue marker it invalidated"
    );
    // Bound rather than left as `..`: an unbound match proves only that *some*
    // StaleGeneration was raised, not that it named the right pair. This one is
    // not sensitive to the membership-test mutation above -- `second` is not a
    // retained generation once promotion has recovered `first` -- so it does
    // not remove that single point of proof, and is not claimed to.
    assert!(matches!(
        select(store, slot, second),
        Err(WorkshopStoreError::StaleGeneration {
            expected,
            actual,
            ..
        }) if expected == second && actual == promoted
    ));
    select(store, slot, promoted).unwrap();
    assert_eq!(list(store).selected_continue, Some(slot));

    // Error precedence, in the order `CommitSlot` already uses: an unknown
    // slot outranks everything, and an archived slot outranks a stale
    // generation because unarchiving, not refreshing, is what unblocks it.
    assert!(matches!(
        select(store, SlotId(9), SaveGeneration(1)),
        Err(WorkshopStoreError::UnknownSlot { slot: unknown }) if unknown == SlotId(9)
    ));
    run(store, WorkshopStoreRequest::ArchiveSlot { slot }).unwrap();
    assert_eq!(list(store).selected_continue, None);
    assert!(matches!(
        select(store, slot, SaveGeneration(999)),
        Err(WorkshopStoreError::ArchivedSlot { slot: archived }) if archived == slot
    ));
    run(store, WorkshopStoreRequest::UnarchiveSlot { slot }).unwrap();
    assert!(matches!(
        select(store, slot, SaveGeneration(999)),
        Err(WorkshopStoreError::StaleGeneration { .. })
    ));
    select(store, slot, promoted).unwrap();
    assert_eq!(list(store).selected_continue, Some(slot));
}

#[test]
fn memory_continue_selection_is_generation_checked_and_cleared_by_every_new_head() {
    continue_selection_is_generation_checked(&mut MemoryWorkshopStore::default());
}

#[test]
fn native_continue_selection_is_generation_checked_and_cleared_by_every_new_head() {
    let directory = tempfile::tempdir().unwrap();
    let mut store = NativeWorkshopStore::at_root(directory.path()).unwrap();
    continue_selection_is_generation_checked(&mut store);

    // The manifest, not a cached field, carries the marker: a fresh instance
    // over the same root sees the selection the run above left behind.
    let mut reopened = NativeWorkshopStore::at_root(directory.path()).unwrap();
    let slots = list(&mut reopened);
    assert_eq!(slots.selected_continue, Some(SlotId(0)));
    assert!(slots.slots[0].selected_for_continue);
}

/// The race across two store instances sharing one directory.
///
/// Instance A lists the row at generation N. Instance B commits N+1. A's
/// selection still carries N and must be refused. This is the version of the
/// race that a cached-manifest CAS would pass wrongly: `execute` re-reads the
/// manifest under the store lock, so the comparison sees B's commit.
#[test]
fn native_continue_selection_rejects_a_generation_another_instance_superseded() {
    let directory = tempfile::tempdir().unwrap();
    let mut first_instance = NativeWorkshopStore::at_root(directory.path()).unwrap();
    let (slot, first) = create(&mut first_instance, "Forge", archive(1)).unwrap();
    select(&mut first_instance, slot, first).unwrap();
    let observed = list(&mut first_instance).slots[0].generation;
    assert_eq!(observed, first);

    let mut second_instance = NativeWorkshopStore::at_root(directory.path()).unwrap();
    let second = commit(&mut second_instance, slot, first, archive(2));
    assert_eq!(second, SaveGeneration(2));
    assert_eq!(
        list(&mut second_instance).selected_continue,
        None,
        "the concurrent commit clears the marker the first instance selected"
    );

    assert!(matches!(
        select(&mut first_instance, slot, observed),
        Err(WorkshopStoreError::StaleGeneration {
            expected,
            actual,
            ..
        }) if expected == observed && actual == second
    ));
    assert_eq!(list(&mut first_instance).selected_continue, None);

    // Refreshing and retrying at the real head is what unblocks it.
    let refreshed = list(&mut first_instance).slots[0].generation;
    select(&mut first_instance, slot, refreshed).unwrap();
    assert_eq!(list(&mut second_instance).selected_continue, Some(slot));
}

#[test]
fn store_is_object_safe_and_continue_is_initially_absent() {
    let mut store: Box<dyn WorkshopStore> = Box::new(MemoryWorkshopStore::default());

    let slots = list(store.as_mut());
    assert!(slots.slots.is_empty());
    assert_eq!(slots.selected_continue, None);
}

#[test]
fn memory_store_orders_slots_and_applies_rename_select_archive_deterministically() {
    let mut store = MemoryWorkshopStore::default();
    let (alpha, _) = create(&mut store, "Alpha", archive(1)).unwrap();
    let (beta, beta_generation) = create(&mut store, "Beta", archive(1)).unwrap();
    assert_eq!((alpha, beta), (SlotId(0), SlotId(1)));

    run(
        &mut store,
        WorkshopStoreRequest::RenameSlot {
            slot: beta,
            name: SlotName::new("Beta Prime").unwrap(),
        },
    )
    .unwrap();
    run(
        &mut store,
        WorkshopStoreRequest::SelectContinue {
            slot: beta,
            expected_generation: beta_generation,
        },
    )
    .unwrap();

    let slots = list(&mut store);
    assert_eq!(slots.selected_continue, Some(beta));
    assert_eq!(slots.slots[0].id, alpha);
    assert_eq!(slots.slots[1].id, beta);
    assert_eq!(slots.slots[1].name.as_str(), "Beta Prime");
    assert!(!slots.slots[0].selected_for_continue);
    assert!(slots.slots[1].selected_for_continue);

    run(&mut store, WorkshopStoreRequest::ArchiveSlot { slot: beta }).unwrap();
    let slots = list(&mut store);
    assert!(slots.slots[1].archived);
    assert!(!slots.slots[1].selected_for_continue);
    assert_eq!(slots.selected_continue, None);
    assert!(matches!(
        run(
            &mut store,
            WorkshopStoreRequest::SelectContinue {
                slot: beta,
                expected_generation: beta_generation,
            }
        ),
        Err(WorkshopStoreError::ArchivedSlot { slot }) if slot == beta
    ));
}

#[test]
fn memory_unarchive_clears_only_the_flag_and_restores_neither_continue_nor_the_open_slot() {
    let mut store = MemoryWorkshopStore::default();
    let (slot, second) = slot_with_predecessor_and_continue(&mut store);
    let before_head = head(&mut store, slot);
    let before_predecessor = predecessor(&mut store, slot, second);

    run(&mut store, WorkshopStoreRequest::ArchiveSlot { slot }).unwrap();
    assert_eq!(
        run(&mut store, WorkshopStoreRequest::UnarchiveSlot { slot }).unwrap(),
        WorkshopStoreResult::SlotUnarchived { slot }
    );

    // Byte identity: name, both generations, both archives and the recovery
    // flag are compared as whole values rather than spot-checked fields.
    assert_eq!(head(&mut store, slot), before_head);
    assert_eq!(predecessor(&mut store, slot, second), before_predecessor);

    let slots = list(&mut store);
    assert_eq!(slots.slots.len(), 1);
    assert!(!slots.slots[0].archived);
    assert!(slots.slots[0].has_previous_generation);
    assert_eq!(slots.slots[0].name.as_str(), "Forge");
    assert_eq!(slots.slots[0].generation, second);
    // Archiving cleared Continue; unarchiving must not put it back.
    assert_eq!(slots.selected_continue, None);
    assert!(!slots.slots[0].selected_for_continue);

    // Unarchiving is idempotent, and an unknown slot is refused the same way
    // archiving refuses one.
    assert_eq!(
        run(&mut store, WorkshopStoreRequest::UnarchiveSlot { slot }).unwrap(),
        WorkshopStoreResult::SlotUnarchived { slot }
    );
    assert_eq!(head(&mut store, slot), before_head);
    assert!(matches!(
        run(
            &mut store,
            WorkshopStoreRequest::UnarchiveSlot { slot: SlotId(9) }
        ),
        Err(WorkshopStoreError::UnknownSlot { slot }) if slot == SlotId(9)
    ));

    // The flag really is clear: the operations archiving refused now succeed.
    run(
        &mut store,
        WorkshopStoreRequest::SelectContinue {
            slot,
            expected_generation: second,
        },
    )
    .unwrap();
    assert!(matches!(
        run(
            &mut store,
            WorkshopStoreRequest::CommitSlot {
                slot,
                expected_generation: second,
                archive: archive(3),
            },
        ),
        Ok(WorkshopStoreResult::SlotCommitted { .. })
    ));
}

#[test]
fn memory_store_allocates_only_the_sixteen_bounded_u64_slot_ids() {
    let mut store = MemoryWorkshopStore::default();
    for expected in 0..16_u64 {
        let (slot, generation) =
            create(&mut store, &format!("Slot {expected}"), archive(1)).unwrap();
        assert_eq!(slot, SlotId(expected));
        assert_eq!(generation, SaveGeneration(1));
    }
    assert!(matches!(
        create(&mut store, "Slot 16", archive(1)),
        Err(WorkshopStoreError::SlotCapacity { max_slots: 16 })
    ));
    assert_eq!(list(&mut store).slots.len(), 16);
}

#[test]
fn memory_store_rejects_stale_invalid_oversized_and_over_quota_commits_atomically() {
    let mut store = MemoryWorkshopStore::default();
    let (slot, generation) = create(&mut store, "Forge", archive(1)).unwrap();

    assert!(matches!(
        run(
            &mut store,
            WorkshopStoreRequest::CommitSlot {
                slot,
                expected_generation: SaveGeneration(0),
                archive: archive(2),
            },
        ),
        Err(WorkshopStoreError::StaleGeneration {
            expected: SaveGeneration(0),
            actual: SaveGeneration(1),
            ..
        })
    ));
    assert!(matches!(
        run(
            &mut store,
            WorkshopStoreRequest::CommitSlot {
                slot,
                expected_generation: generation,
                archive: Box::from(&b"not-json"[..]),
            },
        ),
        Err(WorkshopStoreError::InvalidArchive)
    ));
    assert!(matches!(
        run(
            &mut store,
            WorkshopStoreRequest::CommitSlot {
                slot,
                expected_generation: generation,
                archive: vec![b' '; MAX_WORKSHOP_ARCHIVE_BYTES + 1].into_boxed_slice(),
            },
        ),
        Err(WorkshopStoreError::ArchiveTooLarge { .. })
    ));

    let loaded = run(&mut store, WorkshopStoreRequest::LoadSlot { slot }).unwrap();
    let WorkshopStoreResult::SlotLoaded(loaded) = loaded else {
        panic!("unexpected load result: {loaded:?}");
    };
    assert_eq!(loaded.generation, generation);
    assert_eq!(loaded.archive, archive(1));

    let mut quota_store = MemoryWorkshopStore::with_quota_bytes(8);
    assert!(matches!(
        create(&mut quota_store, "No Room", archive(1)),
        Err(WorkshopStoreError::QuotaExceeded { max_bytes: 8 })
    ));
    assert!(list(&mut quota_store).slots.is_empty());
}

#[test]
fn commit_and_load_jobs_have_independent_in_flight_lanes() {
    let mut store = MemoryWorkshopStore::default();
    let create = store
        .start(WorkshopStoreRequest::CreateSlot {
            name: SlotName::new("Pending").unwrap(),
            archive: archive(1),
        })
        .unwrap();
    let list = store.start(WorkshopStoreRequest::ListSlots).unwrap();

    assert!(matches!(
        store.start(WorkshopStoreRequest::ArchiveSlot { slot: SlotId(0) }),
        Err(WorkshopStoreError::Busy { .. })
    ));
    assert!(matches!(
        store.start(WorkshopStoreRequest::ListPacks),
        Err(WorkshopStoreError::Busy { .. })
    ));
    assert!(matches!(store.poll(create), StoreJobState::Complete(Ok(_))));
    assert!(matches!(store.poll(list), StoreJobState::Complete(Ok(_))));
    assert_eq!(store.poll(create), StoreJobState::Unknown);
}

#[test]
fn an_unpolled_job_holds_its_lane_and_only_the_terminal_poll_frees_it() {
    // The invariant on `WorkshopStore::start`, made executable. A client that
    // drops a job ID without reaching a terminal state wedges that lane for the
    // lifetime of the store. On the Commit lane that starves `CommitSlot` and
    // `PromoteRecoveredSlot`, meaning the resident Workshop can no longer save
    // or discharge a recovery obligation.
    //
    // This test owns the *poll* half of the release contract, and deliberately
    // still asserts that nothing incidental releases a lane. The abandon half
    // is the test immediately below; keeping them apart is what makes "only
    // these two free a lane" checkable rather than assumed.
    let mut store = MemoryWorkshopStore::default();
    let pack = store
        .start(WorkshopStoreRequest::PutPack {
            canonical_pack: Box::from(&b"{}"[..]),
        })
        .unwrap();

    // Dropping the ID does not release anything, and neither does elapsed work
    // on the other lane.
    let list = store.start(WorkshopStoreRequest::ListSlots).unwrap();
    assert!(matches!(store.poll(list), StoreJobState::Complete(Ok(_))));
    for _ in 0..4 {
        assert!(matches!(
            store.start(WorkshopStoreRequest::CreateSlot {
                name: SlotName::new("Starved").unwrap(),
                archive: archive(1),
            }),
            Err(WorkshopStoreError::Busy {
                class: StoreJobClass::Commit
            })
        ));
    }

    // Polling an unknown ID is not an escape hatch: it frees nothing.
    assert_eq!(store.poll(StoreJobId(u64::MAX)), StoreJobState::Unknown);
    assert!(matches!(
        store.start(WorkshopStoreRequest::CreateSlot {
            name: SlotName::new("Starved").unwrap(),
            archive: archive(1),
        }),
        Err(WorkshopStoreError::Busy {
            class: StoreJobClass::Commit
        })
    ));

    // The terminal poll, and nothing else, releases the lane. A rejected
    // request frees it just as a successful one does: what matters is that the
    // job reached a terminal state, not which one.
    assert!(matches!(store.poll(pack), StoreJobState::Complete(_)));
    assert!(
        store
            .start(WorkshopStoreRequest::CreateSlot {
                name: SlotName::new("Recovered").unwrap(),
                archive: archive(1),
            })
            .is_ok()
    );
}

#[test]
fn abandoning_an_in_flight_job_frees_its_lane_for_the_next_commit() {
    // The escape the Library needs. `ff9b6ca` removed a leaky one and left the
    // wedged-`Storing` case with no clean route out at all: the only way to
    // release a Commit lane was to poll the very job the caller had given up
    // on. This is the invariant, not the implementation -- wedge the lane,
    // escape, and prove a real `CommitSlot` gets through afterwards.
    let mut store = MemoryWorkshopStore::default();
    let (slot, generation) = create(&mut store, "Forge", archive(1)).unwrap();

    // A pack that genuinely validates, so the "the work still happened" half
    // below is about abandonment rather than about a rejected request. `{}` was
    // the first attempt here and it stores nothing at all: `validate_pack`
    // refuses it before the adapter writes.
    let wedge = store
        .start(WorkshopStoreRequest::PutPack {
            canonical_pack: Box::from(&include_bytes!("../assets/workshop/core-pack-v1.json")[..]),
        })
        .unwrap();
    assert!(matches!(
        store.start(WorkshopStoreRequest::CommitSlot {
            slot,
            expected_generation: generation,
            archive: archive(2),
        }),
        Err(WorkshopStoreError::Busy {
            class: StoreJobClass::Commit
        })
    ));

    assert!(
        store.abandon(wedge),
        "abandoning a live job reports the release"
    );

    let committed = run(
        &mut store,
        WorkshopStoreRequest::CommitSlot {
            slot,
            expected_generation: generation,
            archive: archive(2),
        },
    )
    .expect("the Commit lane is free again");
    assert_eq!(
        committed,
        WorkshopStoreResult::SlotCommitted {
            slot,
            generation: SaveGeneration(generation.0 + 1),
        }
    );

    // The abandoned job is gone rather than merely detached: its outcome is
    // undeliverable, and abandoning it a second time reports no release.
    assert_eq!(store.poll(wedge), StoreJobState::Unknown);
    assert!(!store.abandon(wedge));
    assert!(!store.abandon(StoreJobId(u64::MAX)));

    // Abandonment discards the answer, never the work: the memory adapter had
    // already executed the pack write when `start` returned, and nothing undid
    // it. That is the property a caller must absorb, so it is asserted rather
    // than left to the doc comment.
    let hash = match run(&mut store, WorkshopStoreRequest::ListPacks).unwrap() {
        WorkshopStoreResult::Packs(packs) => {
            assert_eq!(
                packs.len(),
                1,
                "the abandoned PutPack still stored its pack"
            );
            packs[0]
        }
        result => panic!("unexpected pack list: {result:?}"),
    };
    assert!(matches!(
        run(&mut store, WorkshopStoreRequest::GetPack { hash }).unwrap(),
        WorkshopStoreResult::PackLoaded { .. }
    ));
}

#[test]
fn abandoning_a_native_job_frees_its_lane_while_the_worker_thread_finishes() {
    // The native adapter is the one where work is genuinely in flight: its
    // worker owns the receiver this drops. Abandoning must free the lane
    // without panicking when that thread later sends into a dropped channel.
    let directory = tempfile::tempdir().unwrap();
    let mut store = NativeWorkshopStore::at_root(directory.path()).unwrap();
    let (slot, generation) = create(&mut store, "Forge", archive(1)).unwrap();

    let wedge = store
        .start(WorkshopStoreRequest::PutPack {
            canonical_pack: Box::from(&include_bytes!("../assets/workshop/core-pack-v1.json")[..]),
        })
        .unwrap();
    assert!(matches!(
        store.start(WorkshopStoreRequest::CommitSlot {
            slot,
            expected_generation: generation,
            archive: archive(2),
        }),
        Err(WorkshopStoreError::Busy {
            class: StoreJobClass::Commit
        })
    ));
    assert!(store.abandon(wedge));
    assert_eq!(store.poll(wedge), StoreJobState::Unknown);

    let committed = run(
        &mut store,
        WorkshopStoreRequest::CommitSlot {
            slot,
            expected_generation: generation,
            archive: archive(2),
        },
    )
    .expect("the Commit lane is free again");
    assert_eq!(
        committed,
        WorkshopStoreResult::SlotCommitted {
            slot,
            generation: SaveGeneration(generation.0 + 1),
        }
    );
}

#[test]
fn memory_store_canonicalizes_and_round_trips_validated_packs() {
    let mut store = MemoryWorkshopStore::default();
    let source = include_bytes!("../assets/workshop/core-pack-v1.json");
    let stored = run(
        &mut store,
        WorkshopStoreRequest::PutPack {
            canonical_pack: Box::from(&source[..]),
        },
    )
    .unwrap();
    let WorkshopStoreResult::PackStored { hash } = stored else {
        panic!("unexpected pack-store result: {stored:?}");
    };

    let loaded = run(&mut store, WorkshopStoreRequest::GetPack { hash }).unwrap();
    let WorkshopStoreResult::PackLoaded {
        hash: loaded_hash,
        canonical_pack,
    } = loaded
    else {
        panic!("unexpected pack-load result: {loaded:?}");
    };
    assert_eq!(loaded_hash, hash);
    assert!(canonical_pack.len() < source.len());
    assert_eq!(
        run(&mut store, WorkshopStoreRequest::ListPacks).unwrap(),
        WorkshopStoreResult::Packs(vec![hash])
    );
}

#[test]
fn native_root_uses_the_nyon_workshop_namespace() {
    let environment = WorkshopPathEnvironment {
        home: Some("/Users/example".into()),
        appdata: Some("C:/Users/example/AppData/Roaming".into()),
        xdg_data_home: Some("/var/data".into()),
    };
    assert!(
        native_workshop_root(NativeWorkshopPlatform::MacOs, &environment)
            .unwrap()
            .ends_with("Library/Application Support/NYON/workshop-v1")
    );
    assert!(
        native_workshop_root(NativeWorkshopPlatform::Windows, &environment)
            .unwrap()
            .ends_with("NYON/workshop-v1")
    );
    assert_eq!(
        native_workshop_root(NativeWorkshopPlatform::Linux, &environment).unwrap(),
        Path::new("/var/data/NYON/workshop-v1")
    );
}

struct PauseFirstNativeJob {
    entered: mpsc::SyncSender<()>,
    release: Arc<(Mutex<bool>, Condvar)>,
    paused: bool,
}

impl NativeStoreFaultInjector for PauseFirstNativeJob {
    fn check(&mut self, point: NativeStoreFaultPoint) -> Result<(), WorkshopStoreError> {
        if point != NativeStoreFaultPoint::BeforeJobExecute || self.paused {
            return Ok(());
        }
        self.paused = true;
        self.entered
            .send(())
            .expect("native store test remains ready for the worker");
        let (lock, wake) = &*self.release;
        let released = lock.lock().expect("native store test gate is not poisoned");
        drop(
            wake.wait_while(released, |released| !*released)
                .expect("native store test gate is not poisoned"),
        );
        Ok(())
    }
}

#[test]
fn native_start_dispatches_off_thread_and_preserves_both_in_flight_lanes() {
    let directory = tempfile::tempdir().unwrap();
    let (entered_sender, entered_receiver) = mpsc::sync_channel(1);
    let release = Arc::new((Mutex::new(false), Condvar::new()));
    let store = NativeWorkshopStore::with_fault_injector(
        directory.path(),
        Box::new(PauseFirstNativeJob {
            entered: entered_sender,
            release: Arc::clone(&release),
            paused: false,
        }),
    )
    .unwrap();

    let (started_sender, started_receiver) = mpsc::sync_channel(1);
    let start_thread = std::thread::spawn(move || {
        let mut store = store;
        let load = store.start(WorkshopStoreRequest::ListSlots).unwrap();
        started_sender
            .send((store, load))
            .expect("native start test remains ready for the store");
    });
    entered_receiver
        .recv_timeout(Duration::from_secs(5))
        .expect("native worker reaches the controlled gate");
    let (mut store, load) = match started_receiver.recv_timeout(Duration::from_secs(5)) {
        Ok(started) => started,
        Err(error) => {
            let (lock, wake) = &*release;
            *lock.lock().unwrap() = true;
            wake.notify_all();
            start_thread.join().unwrap();
            panic!("native start blocked on worker disk execution: {error}");
        }
    };
    start_thread.join().unwrap();
    assert_eq!(store.poll(load), StoreJobState::Pending);
    assert!(matches!(
        store.start(WorkshopStoreRequest::ListPacks),
        Err(WorkshopStoreError::Busy {
            class: StoreJobClass::LoadOrImport
        })
    ));

    let commit = store
        .start(WorkshopStoreRequest::CreateSlot {
            name: SlotName::new("Concurrent").unwrap(),
            archive: archive(1),
        })
        .unwrap();
    assert_eq!(store.poll(commit), StoreJobState::Pending);
    assert!(matches!(
        store.start(WorkshopStoreRequest::ArchiveSlot { slot: SlotId(0) }),
        Err(WorkshopStoreError::Busy {
            class: StoreJobClass::Commit
        })
    ));

    let (lock, wake) = &*release;
    *lock.lock().unwrap() = true;
    wake.notify_all();

    assert!(matches!(
        wait_for_job(&mut store, load),
        StoreJobState::Complete(Ok(WorkshopStoreResult::Slots(_)))
    ));
    assert!(matches!(
        wait_for_job(&mut store, commit),
        StoreJobState::Complete(Ok(WorkshopStoreResult::SlotCreated { .. }))
    ));
}

#[test]
fn native_pack_capacity_rejects_before_mutating_a_reopenable_manifest() {
    let directory = tempfile::tempdir().unwrap();
    std::fs::create_dir_all(directory.path().join("slots")).unwrap();
    std::fs::create_dir_all(directory.path().join("packs")).unwrap();
    let packs = (0..MAX_WORKSHOP_PACKS)
        .map(|index| format!("{:02x}{}", u8::try_from(index).unwrap(), "00".repeat(31)))
        .collect::<Vec<_>>();
    let manifest = serde_json::json!({
        "format": "nyon-workshop-store",
        "version": 1,
        "selected_continue": null,
        "slots": [],
        "packs": packs,
    });
    std::fs::write(
        directory.path().join("manifest-v1.json"),
        serde_json::to_vec(&manifest).unwrap(),
    )
    .unwrap();

    let mut store = NativeWorkshopStore::at_root(directory.path()).unwrap();
    assert_eq!(
        match run(&mut store, WorkshopStoreRequest::ListPacks).unwrap() {
            WorkshopStoreResult::Packs(hashes) => hashes.len(),
            result => panic!("unexpected pack list: {result:?}"),
        },
        MAX_WORKSHOP_PACKS
    );
    assert!(matches!(
        run(
            &mut store,
            WorkshopStoreRequest::PutPack {
                canonical_pack: Box::from(
                    &include_bytes!("../assets/workshop/core-pack-v1.json")[..]
                ),
            },
        ),
        Err(WorkshopStoreError::PackCapacity { max_packs })
            if max_packs == MAX_WORKSHOP_PACKS
    ));

    let mut reopened = NativeWorkshopStore::at_root(directory.path()).unwrap();
    assert_eq!(
        match run(&mut reopened, WorkshopStoreRequest::ListPacks).unwrap() {
            WorkshopStoreResult::Packs(hashes) => hashes.len(),
            result => panic!("unexpected reopened pack list: {result:?}"),
        },
        MAX_WORKSHOP_PACKS
    );
}

#[test]
fn native_generation_cas_is_atomic_across_store_instances() {
    let directory = tempfile::tempdir().unwrap();
    let mut first_store = NativeWorkshopStore::at_root(directory.path()).unwrap();
    let (slot, first_generation) = create(&mut first_store, "Shared Slot", archive(1)).unwrap();
    let mut second_store = NativeWorkshopStore::at_root(directory.path()).unwrap();

    let first_job = first_store
        .start(WorkshopStoreRequest::CommitSlot {
            slot,
            expected_generation: first_generation,
            archive: archive(2),
        })
        .unwrap();
    let second_job = second_store
        .start(WorkshopStoreRequest::CommitSlot {
            slot,
            expected_generation: first_generation,
            archive: archive(3),
        })
        .unwrap();

    let first_result = wait_for_job(&mut first_store, first_job);
    let second_result = wait_for_job(&mut second_store, second_job);
    let results = [first_result, second_result];
    assert_eq!(
        results
            .iter()
            .filter(|state| matches!(
                state,
                StoreJobState::Complete(Ok(WorkshopStoreResult::SlotCommitted {
                    generation: SaveGeneration(2),
                    ..
                }))
            ))
            .count(),
        1
    );
    assert_eq!(
        results
            .iter()
            .filter(|state| matches!(
                state,
                StoreJobState::Complete(Err(WorkshopStoreError::StaleGeneration {
                    expected: SaveGeneration(1),
                    actual: SaveGeneration(2),
                    ..
                }))
            ))
            .count(),
        1
    );

    let mut reopened = NativeWorkshopStore::at_root(directory.path()).unwrap();
    assert_eq!(list(&mut reopened).slots[0].generation, SaveGeneration(2));
}

#[test]
fn native_store_recovers_the_previous_valid_generation_without_moving_the_head() {
    let directory = tempfile::tempdir().unwrap();
    let mut store = NativeWorkshopStore::at_root(directory.path()).unwrap();
    let (slot, first) = create(&mut store, "Forge", archive(1)).unwrap();
    assert_eq!(first, SaveGeneration(1));
    let second = match run(
        &mut store,
        WorkshopStoreRequest::CommitSlot {
            slot,
            expected_generation: first,
            archive: archive(2),
        },
    )
    .unwrap()
    {
        WorkshopStoreResult::SlotCommitted { generation, .. } => generation,
        result => panic!("unexpected commit result: {result:?}"),
    };
    assert_eq!(second, SaveGeneration(2));
    // Selection follows the commit: a commit advances the head and clears any
    // marker naming this slot, so selecting first would be cleared again here.
    run(
        &mut store,
        WorkshopStoreRequest::SelectContinue {
            slot,
            expected_generation: second,
        },
    )
    .unwrap();

    std::fs::write(
        directory
            .path()
            .join("slots/slot-00/generation-00000000000000000002.archive"),
        b"corrupt latest",
    )
    .unwrap();
    let mut reopened = NativeWorkshopStore::at_root(directory.path()).unwrap();
    let loaded = run(&mut reopened, WorkshopStoreRequest::LoadSlot { slot }).unwrap();
    let WorkshopStoreResult::SlotLoaded(loaded) = loaded else {
        panic!("unexpected load result: {loaded:?}");
    };
    assert_eq!(loaded.generation, first);
    assert_eq!(loaded.head_generation, second);
    assert!(loaded.recovered_from_previous);
    assert_eq!(loaded.archive, archive(1));

    let slots = list(&mut reopened);
    assert_eq!(slots.selected_continue, Some(slot));
    assert_eq!(slots.slots[0].generation, second);
}

#[test]
fn native_manifest_replacement_selects_the_committed_generation_after_reopen() {
    let directory = tempfile::tempdir().unwrap();
    let mut store = NativeWorkshopStore::at_root(directory.path()).unwrap();
    let (slot, first) = create(&mut store, "Forge", archive(1)).unwrap();
    let second = match run(
        &mut store,
        WorkshopStoreRequest::CommitSlot {
            slot,
            expected_generation: first,
            archive: archive(2),
        },
    )
    .unwrap()
    {
        WorkshopStoreResult::SlotCommitted { generation, .. } => generation,
        result => panic!("unexpected commit result: {result:?}"),
    };

    let mut reopened = NativeWorkshopStore::at_root(directory.path()).unwrap();
    let loaded = run(&mut reopened, WorkshopStoreRequest::LoadSlot { slot }).unwrap();
    let WorkshopStoreResult::SlotLoaded(loaded) = loaded else {
        panic!("unexpected load result: {loaded:?}");
    };
    assert_eq!(loaded.generation, second);
    assert_eq!(loaded.head_generation, second);
    assert_eq!(loaded.archive, archive(2));
    assert!(!loaded.recovered_from_previous);
    assert_eq!(list(&mut reopened).slots[0].generation, second);
}

#[test]
fn native_invalid_and_stale_commits_preserve_the_last_valid_generation() {
    let directory = tempfile::tempdir().unwrap();
    let mut store = NativeWorkshopStore::at_root(directory.path()).unwrap();
    let (slot, first) = create(&mut store, "Forge", archive(1)).unwrap();

    assert!(matches!(
        run(
            &mut store,
            WorkshopStoreRequest::CommitSlot {
                slot,
                expected_generation: SaveGeneration(99),
                archive: archive(2),
            },
        ),
        Err(WorkshopStoreError::StaleGeneration { .. })
    ));
    assert!(matches!(
        run(
            &mut store,
            WorkshopStoreRequest::CommitSlot {
                slot,
                expected_generation: first,
                archive: Box::from(&b"{"[..]),
            },
        ),
        Err(WorkshopStoreError::InvalidArchive)
    ));

    let loaded = run(&mut store, WorkshopStoreRequest::LoadSlot { slot }).unwrap();
    let WorkshopStoreResult::SlotLoaded(loaded) = loaded else {
        panic!("unexpected load result: {loaded:?}");
    };
    assert_eq!(loaded.generation, first);
    assert_eq!(loaded.archive, archive(1));
}

#[test]
fn native_rename_select_and_archive_survive_reopen_without_deleting_the_slot() {
    let directory = tempfile::tempdir().unwrap();
    let mut store = NativeWorkshopStore::at_root(directory.path()).unwrap();
    let (slot, generation) = create(&mut store, "Original", archive(1)).unwrap();
    run(
        &mut store,
        WorkshopStoreRequest::RenameSlot {
            slot,
            name: SlotName::new("Renamed").unwrap(),
        },
    )
    .unwrap();
    run(
        &mut store,
        WorkshopStoreRequest::SelectContinue {
            slot,
            expected_generation: generation,
        },
    )
    .unwrap();
    run(&mut store, WorkshopStoreRequest::ArchiveSlot { slot }).unwrap();

    let mut reopened = NativeWorkshopStore::at_root(directory.path()).unwrap();
    let slots = list(&mut reopened);
    assert_eq!(slots.slots.len(), 1);
    assert_eq!(slots.slots[0].name.as_str(), "Renamed");
    assert!(slots.slots[0].archived);
    assert_eq!(slots.selected_continue, None);
    assert!(matches!(
        run(&mut reopened, WorkshopStoreRequest::LoadSlot { slot }),
        Ok(WorkshopStoreResult::SlotLoaded(_))
    ));
}

#[test]
fn native_unarchive_clears_only_the_flag_and_survives_reopen_without_restoring_continue() {
    let directory = tempfile::tempdir().unwrap();
    let mut store = NativeWorkshopStore::at_root(directory.path()).unwrap();
    let (slot, second) = slot_with_predecessor_and_continue(&mut store);
    let before_head = head(&mut store, slot);
    let before_predecessor = predecessor(&mut store, slot, second);

    run(&mut store, WorkshopStoreRequest::ArchiveSlot { slot }).unwrap();
    assert_eq!(
        run(&mut store, WorkshopStoreRequest::UnarchiveSlot { slot }).unwrap(),
        WorkshopStoreResult::SlotUnarchived { slot }
    );

    // Reopening re-reads the manifest and re-verifies every generation digest,
    // so identical loads here prove no archive file was rewritten.
    let mut reopened = NativeWorkshopStore::at_root(directory.path()).unwrap();
    assert_eq!(head(&mut reopened, slot), before_head);
    assert_eq!(predecessor(&mut reopened, slot, second), before_predecessor);

    let slots = list(&mut reopened);
    assert_eq!(slots.slots.len(), 1);
    assert!(!slots.slots[0].archived);
    assert!(slots.slots[0].has_previous_generation);
    assert_eq!(slots.slots[0].name.as_str(), "Forge");
    assert_eq!(slots.slots[0].generation, second);
    assert_eq!(slots.selected_continue, None);
    assert!(!slots.slots[0].selected_for_continue);

    assert_eq!(
        run(&mut reopened, WorkshopStoreRequest::UnarchiveSlot { slot }).unwrap(),
        WorkshopStoreResult::SlotUnarchived { slot }
    );
    assert!(matches!(
        run(
            &mut reopened,
            WorkshopStoreRequest::UnarchiveSlot { slot: SlotId(9) }
        ),
        Err(WorkshopStoreError::UnknownSlot { slot }) if slot == SlotId(9)
    ));
    run(
        &mut reopened,
        WorkshopStoreRequest::SelectContinue {
            slot,
            expected_generation: second,
        },
    )
    .unwrap();
}

struct FailAfterSecondGeneration {
    generation_persists: usize,
}

impl NativeStoreFaultInjector for FailAfterSecondGeneration {
    fn check(&mut self, point: NativeStoreFaultPoint) -> Result<(), WorkshopStoreError> {
        if point == NativeStoreFaultPoint::AfterGenerationPersist {
            self.generation_persists += 1;
            if self.generation_persists == 2 {
                return Err(WorkshopStoreError::InjectedFault { point });
            }
        }
        Ok(())
    }
}

#[test]
fn native_crash_after_generation_write_leaves_the_atomic_manifest_on_the_old_head() {
    let directory = tempfile::tempdir().unwrap();
    let mut store = NativeWorkshopStore::with_fault_injector(
        directory.path(),
        Box::new(FailAfterSecondGeneration {
            generation_persists: 0,
        }),
    )
    .unwrap();
    let (slot, first) = create(&mut store, "Forge", archive(1)).unwrap();

    assert!(matches!(
        run(
            &mut store,
            WorkshopStoreRequest::CommitSlot {
                slot,
                expected_generation: first,
                archive: archive(2),
            },
        ),
        Err(WorkshopStoreError::InjectedFault {
            point: NativeStoreFaultPoint::AfterGenerationPersist
        })
    ));

    let mut reopened = NativeWorkshopStore::at_root(directory.path()).unwrap();
    let slots = list(&mut reopened);
    assert_eq!(slots.slots[0].generation, first);
    let loaded = run(&mut reopened, WorkshopStoreRequest::LoadSlot { slot }).unwrap();
    let WorkshopStoreResult::SlotLoaded(loaded) = loaded else {
        panic!("unexpected load result: {loaded:?}");
    };
    assert_eq!(loaded.archive, archive(1));
    assert!(!loaded.recovered_from_previous);
}
