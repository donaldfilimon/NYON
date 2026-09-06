#![cfg(not(target_arch = "wasm32"))]

use std::{
    path::Path,
    sync::{Arc, Condvar, Mutex, mpsc},
    time::{Duration, Instant},
};

use nyon::workshop::store::{
    MAX_WORKSHOP_ARCHIVE_BYTES, MAX_WORKSHOP_PACKS, MemoryWorkshopStore, NativeStoreFaultInjector,
    NativeStoreFaultPoint, NativeWorkshopPlatform, NativeWorkshopStore, SaveGeneration, SlotId,
    SlotList, SlotName, StoreJobClass, StoreJobId, StoreJobState, WorkshopPathEnvironment,
    WorkshopStore, WorkshopStoreError, WorkshopStoreRequest, WorkshopStoreResult,
    native_workshop_root,
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
    let (beta, _) = create(&mut store, "Beta", archive(1)).unwrap();
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
        WorkshopStoreRequest::SelectContinue { slot: beta },
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
            WorkshopStoreRequest::SelectContinue { slot: beta }
        ),
        Err(WorkshopStoreError::ArchivedSlot { slot }) if slot == beta
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
    run(&mut store, WorkshopStoreRequest::SelectContinue { slot }).unwrap();
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
    let (slot, _) = create(&mut store, "Original", archive(1)).unwrap();
    run(
        &mut store,
        WorkshopStoreRequest::RenameSlot {
            slot,
            name: SlotName::new("Renamed").unwrap(),
        },
    )
    .unwrap();
    run(&mut store, WorkshopStoreRequest::SelectContinue { slot }).unwrap();
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
