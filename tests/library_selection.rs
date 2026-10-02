use nyon::library_selection::{
    LibrarySelection, SelectedAuthority, SelectionError, SelectionTarget, recover_selection,
};
fn target() -> SelectionTarget {
    SelectionTarget {
        authority: SelectedAuthority::LivingV2,
        slot: 7,
        branch: "a".repeat(32),
        revision: None,
        tick: 123,
        digest: "b".repeat(64),
    }
}
#[test]
fn exact_selection_and_clear_are_canonical_integrity_checked() {
    let record = LibrarySelection::new(1, Some(target())).unwrap();
    let bytes = record.encode().unwrap();
    assert_eq!(LibrarySelection::decode(&bytes).unwrap(), record);
    assert_eq!(record.target().unwrap(), Some(target()));
    let mut spaced = bytes.clone();
    spaced.push(b' ');
    assert_eq!(
        LibrarySelection::decode(&spaced),
        Err(SelectionError::CorruptRecovery)
    );
    let clear = LibrarySelection::new(2, None).unwrap();
    assert_eq!(
        LibrarySelection::decode(&clear.encode().unwrap())
            .unwrap()
            .target()
            .unwrap(),
        None
    );
}
#[test]
fn recovery_only_uses_prior_valid_record_and_never_an_uncommitted_newer_record() {
    let a = LibrarySelection::new(1, Some(target()))
        .unwrap()
        .encode()
        .unwrap();
    let b = LibrarySelection::new(2, None).unwrap().encode().unwrap();
    assert_eq!(
        recover_selection(Some(&a), Some(&b), Some(1))
            .unwrap()
            .unwrap()
            .generation,
        1
    );
    assert_eq!(
        recover_selection(Some(&a), Some(b"broken"), Some(2))
            .unwrap()
            .unwrap()
            .generation,
        1
    );
    assert_eq!(
        recover_selection(Some(b"broken"), Some(b"broken"), Some(2)),
        Err(SelectionError::CorruptRecovery)
    );
}
#[cfg(not(target_arch = "wasm32"))]
#[test]
fn native_two_generations_recover_and_cas_uses_recovered_generation() {
    use nyon::library_selection::native::NativeLibrarySelection;
    let root = std::env::temp_dir().join(format!(
        "nyon-selection-test-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    let store = NativeLibrarySelection::new(&root);
    assert_eq!(store.read().unwrap(), None);
    store.compare_and_swap(None, Some(target())).unwrap();
    store.compare_and_swap(Some(1), None).unwrap();
    assert_eq!(
        store.compare_and_swap(Some(1), Some(target())),
        Err(SelectionError::StaleGeneration)
    );
    std::fs::write(root.join("library-selection-v1.b"), b"corrupt").unwrap();
    assert_eq!(store.read().unwrap().unwrap().generation, 1);
    store.compare_and_swap(Some(1), Some(target())).unwrap();
    assert_eq!(store.read().unwrap().unwrap().generation, 2);
    std::fs::remove_dir_all(root).unwrap();
}

#[cfg(not(target_arch = "wasm32"))]
#[test]
fn native_jobs_do_not_mutate_at_begin_and_enforce_owner_and_release() {
    use nyon::library_selection::{
        LibrarySelectionCoordinator, SelectionCancel, SelectionPoll, SelectionRequest,
        native::NativeLibraryCoordinator,
    };
    let root = std::env::temp_dir().join(format!("nyon-selection-job-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    let mut jobs = NativeLibraryCoordinator::new(&root);
    let job = jobs
        .begin(
            4,
            SelectionRequest::CompareAndSwap {
                expected: None,
                target: Some(target()),
            },
        )
        .unwrap();
    assert!(!root.exists());
    assert!(matches!(jobs.poll(5, job, 1024), SelectionPoll::Failed(_)));
    assert_eq!(jobs.cancel(4, job).unwrap(), SelectionCancel::Cancelled);
    assert!(!root.exists());
    let job = jobs
        .begin(
            4,
            SelectionRequest::CompareAndSwap {
                expected: None,
                target: Some(target()),
            },
        )
        .unwrap();
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(10);
    loop {
        match jobs.poll(4, job, 1024) {
            SelectionPoll::Pending => {
                assert!(std::time::Instant::now() < deadline);
                std::thread::yield_now();
            }
            SelectionPoll::Ready(Some(record)) => {
                assert_eq!(record.generation, 1);
                break;
            }
            other => panic!("{other:?}"),
        }
    }
    assert!(matches!(
        jobs.cancel(4, job).unwrap(),
        SelectionCancel::CommitAlreadyWon(_)
    ));
    assert!(jobs.release(4, job));
    std::fs::remove_dir_all(root).unwrap();
}

#[cfg(not(target_arch = "wasm32"))]
#[test]
fn coordinator_cancellation_before_commit_preserves_unset_selection() {
    use nyon::library_selection::{
        LibrarySelectionCoordinator, SelectionCancel, SelectionPoll, SelectionRequest,
        native::NativeLibraryCoordinator,
    };
    let root = std::env::temp_dir().join(format!(
        "nyon-selection-cancel-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    std::fs::create_dir_all(&root).unwrap();
    let blocker = std::fs::OpenOptions::new()
        .create(true)
        .truncate(false)
        .write(true)
        .open(root.join("library-selection-v1.lock"))
        .unwrap();
    blocker.lock().unwrap();
    let mut store = NativeLibraryCoordinator::new(&root);
    let job = store
        .begin(
            9,
            SelectionRequest::CompareAndSwap {
                expected: None,
                target: Some(target()),
            },
        )
        .unwrap();
    assert_eq!(store.poll(9, job, 1024), SelectionPoll::Pending);
    assert_eq!(store.cancel(9, job).unwrap(), SelectionCancel::Cancelled);
    drop(blocker);
    let job = store.begin(9, SelectionRequest::Read).unwrap();
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(10);
    loop {
        match store.poll(9, job, 1024) {
            SelectionPoll::Pending => {
                assert!(std::time::Instant::now() < deadline);
                std::thread::yield_now();
            }
            SelectionPoll::Ready(None) => break,
            other => panic!("{other:?}"),
        }
    }
    assert!(store.release(9, job));
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn neutral_selection_preserves_zero_based_workshop_slot_identity_only() {
    let mut selected = target();
    selected.slot = 0;
    selected.authority = SelectedAuthority::WorkshopV1;
    let record = LibrarySelection::new(1, Some(selected.clone())).unwrap();
    assert_eq!(
        LibrarySelection::decode(&record.encode().unwrap())
            .unwrap()
            .target()
            .unwrap(),
        Some(selected.clone())
    );
    selected.authority = SelectedAuthority::LivingV2;
    assert_eq!(
        LibrarySelection::new(1, Some(selected)).unwrap_err(),
        SelectionError::InvalidTarget
    );
}
