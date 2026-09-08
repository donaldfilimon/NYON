use std::time::Duration;

use nyon::workshop::{
    BatchLocalId, CreatorBatchV1, CreatorOpV1, GalaxyPointV1, ObjectName, WorkshopHistory,
    decode_catalog_pack, encode_archive,
    session::{
        MAX_WORKSHOP_ACTIONS_QUEUED, MAX_WORKSHOP_DIAGNOSTICS, WorkshopAction, WorkshopSession,
        WorkshopSessionError, WorkshopSpeed,
    },
    store::{
        MemoryWorkshopStore, SlotName, StoreJobState, WorkshopStore, WorkshopStoreError,
        WorkshopStoreRequest, WorkshopStoreResult,
    },
};

#[derive(Default)]
struct RejectingStore {
    starts: usize,
}

impl WorkshopStore for RejectingStore {
    fn start(
        &mut self,
        _request: WorkshopStoreRequest,
    ) -> Result<nyon::workshop::store::StoreJobId, WorkshopStoreError> {
        self.starts += 1;
        Err(WorkshopStoreError::QuotaExceeded { max_bytes: 0 })
    }

    fn abandon(&mut self, _job: nyon::workshop::store::StoreJobId) -> bool {
        false
    }

    fn poll(&mut self, _job: nyon::workshop::store::StoreJobId) -> StoreJobState {
        StoreJobState::Unknown
    }
}

#[derive(Default)]
struct CountingStore {
    inner: MemoryWorkshopStore,
    save_starts: usize,
}

impl WorkshopStore for CountingStore {
    fn start(
        &mut self,
        request: WorkshopStoreRequest,
    ) -> Result<nyon::workshop::store::StoreJobId, WorkshopStoreError> {
        if matches!(
            request,
            WorkshopStoreRequest::CreateSlot { .. } | WorkshopStoreRequest::CommitSlot { .. }
        ) {
            self.save_starts += 1;
        }
        self.inner.start(request)
    }

    fn abandon(&mut self, job: nyon::workshop::store::StoreJobId) -> bool {
        self.inner.abandon(job)
    }

    fn poll(&mut self, job: nyon::workshop::store::StoreJobId) -> StoreJobState {
        self.inner.poll(job)
    }
}

fn session(seed: u64) -> WorkshopSession {
    let catalog =
        decode_catalog_pack(include_bytes!("../assets/workshop/core-pack-v1.json")).unwrap();
    WorkshopSession::new(WorkshopHistory::from_seed_u64(catalog, seed))
}

fn create_system(session: &WorkshopSession, local: u16, name: &str) -> CreatorBatchV1 {
    CreatorBatchV1 {
        expected_cursor: session.snapshot().active_view.view_cursor,
        expected_tick: session.snapshot().active_view.tick,
        operations: vec![CreatorOpV1::CreateSystem {
            local: BatchLocalId(local),
            name: ObjectName::new(name).unwrap(),
            position: GalaxyPointV1::new(i64::from(local) * 1_024, 0).unwrap(),
        }],
    }
}

fn archive_with_system(seed: u64, name: &str) -> Box<[u8]> {
    let mut replacement = session(seed);
    let mut store = MemoryWorkshopStore::default();
    replacement
        .enqueue(WorkshopAction::Submit(create_system(&replacement, 1, name)))
        .unwrap();
    replacement.update(Duration::ZERO, &mut store);
    encode_archive(replacement.history())
        .unwrap()
        .bytes
        .into_boxed_slice()
}

#[test]
fn creator_actions_commit_before_bounded_simulation_steps() {
    let mut session = session(1);
    let mut store = MemoryWorkshopStore::default();
    session
        .enqueue(WorkshopAction::Resume(WorkshopSpeed::One))
        .unwrap();
    session
        .enqueue(WorkshopAction::Submit(create_system(
            &session,
            1,
            "Forge Alpha",
        )))
        .unwrap();

    let update = session.update(Duration::from_millis(100), &mut store);
    assert_eq!(update.accepted_batches, 1);
    assert_eq!(update.authority_steps, 1);
    assert_eq!(session.snapshot().state.tick.0, 1);
    assert_eq!(session.snapshot().state.systems.len(), 1);
    let revision = session.history().revisions().values().next().unwrap();
    assert_eq!(revision.tick.0, 0);
}

#[test]
fn pause_step_and_speed_are_session_only_repeated_core_steps() {
    let mut store = MemoryWorkshopStore::default();
    let mut paused = session(2);
    assert_eq!(
        paused
            .update(Duration::from_secs(1), &mut store)
            .authority_steps,
        0
    );
    paused.enqueue(WorkshopAction::StepOnce).unwrap();
    assert_eq!(paused.update(Duration::ZERO, &mut store).authority_steps, 1);
    assert_eq!(paused.snapshot().state.tick.0, 1);

    for (speed, expected) in [
        (WorkshopSpeed::One, 1),
        (WorkshopSpeed::Four, 4),
        (WorkshopSpeed::Twenty, 20),
    ] {
        let mut running = session(2);
        running.enqueue(WorkshopAction::Resume(speed)).unwrap();
        assert_eq!(
            running
                .update(Duration::from_millis(100), &mut store)
                .authority_steps,
            expected
        );
    }
}

#[test]
fn simulation_steps_are_dirty_without_arming_creator_debounce_and_pause_saves() {
    let mut store = MemoryWorkshopStore::default();
    let mut stepped = session(20);
    stepped.enqueue(WorkshopAction::StepOnce).unwrap();
    stepped.update(Duration::from_millis(250), &mut store);
    assert_eq!(stepped.snapshot().state.tick.0, 1);
    assert!(stepped.snapshot().store.dirty);
    assert!(!stepped.snapshot().store.commit_pending);

    stepped.enqueue(WorkshopAction::Pause).unwrap();
    stepped.update(Duration::ZERO, &mut store);
    assert!(stepped.snapshot().store.commit_pending);
}

#[test]
fn running_simulation_requests_save_when_a_frame_crosses_600_ticks() {
    let mut store = MemoryWorkshopStore::default();
    let mut running = session(21);
    running
        .enqueue(WorkshopAction::Resume(WorkshopSpeed::Twenty))
        .unwrap();
    for _ in 0..30 {
        running.update(Duration::from_millis(100), &mut store);
    }
    assert_eq!(running.snapshot().state.tick.0, 600);
    assert!(running.snapshot().store.dirty);
    assert!(running.snapshot().store.commit_pending);
}

#[test]
fn equivalent_frame_chunking_produces_the_same_authoritative_digest() {
    let mut whole = session(3);
    let mut chunked = session(3);
    let mut whole_store = MemoryWorkshopStore::default();
    let mut chunked_store = MemoryWorkshopStore::default();
    whole
        .enqueue(WorkshopAction::Resume(WorkshopSpeed::Four))
        .unwrap();
    chunked
        .enqueue(WorkshopAction::Resume(WorkshopSpeed::Four))
        .unwrap();

    whole.update(Duration::from_millis(200), &mut whole_store);
    chunked.update(Duration::from_millis(100), &mut chunked_store);
    chunked.update(Duration::from_millis(100), &mut chunked_store);

    assert_eq!(
        whole.snapshot().state_digest,
        chunked.snapshot().state_digest
    );
    assert_eq!(whole.snapshot().state, chunked.snapshot().state);
}

#[test]
fn undo_redo_and_fork_preserve_the_original_branch() {
    let mut session = session(4);
    let mut store = MemoryWorkshopStore::default();
    session
        .enqueue(WorkshopAction::Submit(create_system(&session, 1, "Alpha")))
        .unwrap();
    session.update(Duration::ZERO, &mut store);
    let alpha_revision = session.snapshot().active_view.view_cursor.unwrap();
    let alpha_digest = session.snapshot().state_digest;
    let original_branch = session.snapshot().active_view.selected_branch;

    session.enqueue(WorkshopAction::Undo).unwrap();
    session.update(Duration::ZERO, &mut store);
    assert_eq!(session.snapshot().active_view.view_cursor, None);

    session
        .enqueue(WorkshopAction::Redo(alpha_revision))
        .unwrap();
    session.update(Duration::ZERO, &mut store);
    assert_eq!(session.snapshot().state_digest, alpha_digest);

    session.enqueue(WorkshopAction::Undo).unwrap();
    session.update(Duration::ZERO, &mut store);
    session
        .enqueue(WorkshopAction::Submit(create_system(&session, 2, "Beta")))
        .unwrap();
    session.update(Duration::ZERO, &mut store);

    assert_eq!(session.snapshot().branches.len(), 2);
    assert_ne!(
        session.snapshot().active_view.selected_branch,
        original_branch
    );
    let original = session
        .snapshot()
        .branches
        .iter()
        .find(|branch| branch.id == original_branch)
        .unwrap();
    assert_eq!(original.head, Some(alpha_revision));
}

#[test]
fn invalid_loaded_archive_never_replaces_the_live_session() {
    let mut session = session(5);
    let mut store = MemoryWorkshopStore::default();
    session
        .enqueue(WorkshopAction::Submit(create_system(&session, 1, "Kept")))
        .unwrap();
    session.update(Duration::ZERO, &mut store);
    let before = session.snapshot().state_digest;

    let create = store
        .start(WorkshopStoreRequest::CreateSlot {
            name: SlotName::new("Invalid").unwrap(),
            archive: Box::from(&b"{}"[..]),
        })
        .unwrap();
    let slot = match store.poll(create) {
        StoreJobState::Complete(Ok(WorkshopStoreResult::SlotCreated { slot, .. })) => slot,
        result => panic!("unexpected create result: {result:?}"),
    };

    session.enqueue(WorkshopAction::RequestLoad(slot)).unwrap();
    session.update(Duration::ZERO, &mut store);
    assert_eq!(session.snapshot().state_digest, before);
    assert!(
        session
            .snapshot()
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.message.contains("archive"))
    );
}

#[test]
fn save_jobs_coalesce_and_select_the_created_slot_for_continue() {
    let mut session = session(6);
    let mut store = MemoryWorkshopStore::default();
    session
        .enqueue(WorkshopAction::Submit(create_system(&session, 1, "Saved")))
        .unwrap();
    session.enqueue(WorkshopAction::RequestSave).unwrap();
    session.update(Duration::ZERO, &mut store);
    assert!(session.snapshot().store.commit_pending);

    session.update(Duration::ZERO, &mut store);
    assert!(session.snapshot().store.slot.is_some());
    session.update(Duration::ZERO, &mut store);
    session.update(Duration::ZERO, &mut store);
    assert!(!session.snapshot().store.dirty);

    let list = store.start(WorkshopStoreRequest::ListSlots).unwrap();
    match store.poll(list) {
        StoreJobState::Complete(Ok(WorkshopStoreResult::Slots(slots))) => {
            assert_eq!(slots.selected_continue, session.snapshot().store.slot);
        }
        result => panic!("unexpected slot list: {result:?}"),
    }
}

#[test]
fn mailbox_and_rejection_diagnostics_are_bounded() {
    let mut session = session(7);
    for _ in 0..MAX_WORKSHOP_ACTIONS_QUEUED {
        session.enqueue(WorkshopAction::StepOnce).unwrap();
    }
    assert_eq!(
        session.enqueue(WorkshopAction::StepOnce),
        Err(WorkshopSessionError::ActionQueueFull {
            limit: MAX_WORKSHOP_ACTIONS_QUEUED
        })
    );
    let mut store = MemoryWorkshopStore::default();
    session.update(Duration::ZERO, &mut store);
    assert!(session.snapshot().diagnostics.len() <= MAX_WORKSHOP_DIAGNOSTICS);
}

#[test]
fn persistent_store_failure_is_backed_off_but_manual_save_retries_immediately() {
    let mut session = session(8);
    let mut store = RejectingStore::default();
    session
        .enqueue(WorkshopAction::Submit(create_system(
            &session, 1, "Unsaved",
        )))
        .unwrap();

    session.update(Duration::from_millis(250), &mut store);
    assert_eq!(store.starts, 1);
    assert!(session.snapshot().store.dirty);
    assert_eq!(
        session.snapshot().store.status,
        "Save failed; retry delayed"
    );

    for _ in 0..100 {
        session.update(Duration::from_millis(16), &mut store);
    }
    assert_eq!(store.starts, 1);

    session.enqueue(WorkshopAction::RequestSave).unwrap();
    session.update(Duration::ZERO, &mut store);
    assert_eq!(store.starts, 2);
}

#[test]
fn simulation_during_an_in_flight_save_waits_for_the_tick_boundary_or_pause() {
    let mut session = session(9);
    let mut store = CountingStore::default();
    session
        .enqueue(WorkshopAction::Submit(create_system(
            &session,
            1,
            "Saved While Running",
        )))
        .unwrap();
    session.enqueue(WorkshopAction::RequestSave).unwrap();
    session.update(Duration::ZERO, &mut store);
    assert_eq!(store.save_starts, 1);

    session
        .enqueue(WorkshopAction::Resume(WorkshopSpeed::Twenty))
        .unwrap();
    session.update(Duration::from_millis(100), &mut store);
    session.update(Duration::from_millis(100), &mut store);
    session.update(Duration::from_millis(100), &mut store);

    assert_eq!(session.snapshot().state.tick.0, 60);
    assert!(session.snapshot().store.dirty);
    assert!(!session.snapshot().store.commit_pending);
    assert_eq!(store.save_starts, 1);
}

#[test]
fn simulation_progress_does_not_clear_a_failed_autosave_backoff() {
    let mut session = session(10);
    let mut store = RejectingStore::default();
    session
        .enqueue(WorkshopAction::Submit(create_system(
            &session,
            1,
            "Backed Off",
        )))
        .unwrap();
    session.update(Duration::from_millis(250), &mut store);
    assert_eq!(store.starts, 1);

    session
        .enqueue(WorkshopAction::Resume(WorkshopSpeed::One))
        .unwrap();
    for _ in 0..10 {
        session.update(Duration::from_millis(100), &mut store);
    }

    assert_eq!(session.snapshot().state.tick.0, 10);
    assert!(session.snapshot().store.dirty);
    assert_eq!(store.starts, 1);
}

#[test]
fn load_serializes_import_playback_and_creator_edits() {
    let loaded_archive = archive_with_system(30, "Loaded Authority");
    let imported_archive = archive_with_system(31, "Rejected Import");
    let mut store = MemoryWorkshopStore::default();
    let create = store
        .start(WorkshopStoreRequest::CreateSlot {
            name: SlotName::new("Loaded Slot").unwrap(),
            archive: loaded_archive,
        })
        .unwrap();
    let slot = match store.poll(create) {
        StoreJobState::Complete(Ok(WorkshopStoreResult::SlotCreated { slot, .. })) => slot,
        state => panic!("unexpected slot creation result: {state:?}"),
    };

    let mut active = session(32);
    active.enqueue(WorkshopAction::RequestLoad(slot)).unwrap();
    active
        .enqueue(WorkshopAction::RequestImport(imported_archive))
        .unwrap();
    active
        .enqueue(WorkshopAction::Resume(WorkshopSpeed::Twenty))
        .unwrap();
    active
        .enqueue(WorkshopAction::Submit(create_system(
            &active,
            2,
            "Rejected Edit",
        )))
        .unwrap();

    let update = active.update(Duration::from_millis(100), &mut store);
    assert_eq!(update.accepted_batches, 0);
    assert_eq!(update.authority_steps, 0);
    assert_eq!(active.snapshot().state.systems.len(), 1);
    assert_eq!(
        active
            .snapshot()
            .state
            .systems
            .values()
            .next()
            .unwrap()
            .name
            .as_str(),
        "Loaded Authority"
    );
    assert_eq!(active.snapshot().speed, WorkshopSpeed::Paused);
    assert!(
        active
            .snapshot()
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.message.contains("before importing"))
    );
    assert!(
        active
            .snapshot()
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.message.contains("Playback is unavailable"))
    );
    assert!(
        active
            .snapshot()
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.message.contains("Creator edits are unavailable"))
    );
}

#[test]
fn import_is_rejected_while_a_save_owns_the_commit_lane() {
    let imported_archive = archive_with_system(40, "Replacement");
    let mut store = CountingStore::default();
    let mut active = session(41);
    active
        .enqueue(WorkshopAction::Submit(create_system(
            &active,
            1,
            "Preserved",
        )))
        .unwrap();
    active.enqueue(WorkshopAction::RequestSave).unwrap();
    active.update(Duration::ZERO, &mut store);
    assert!(active.snapshot().store.commit_pending);

    active
        .enqueue(WorkshopAction::RequestImport(imported_archive))
        .unwrap();
    active.update(Duration::ZERO, &mut store);
    for _ in 0..4 {
        active.update(Duration::ZERO, &mut store);
    }

    assert_eq!(store.save_starts, 1);
    assert_eq!(active.snapshot().state.systems.len(), 1);
    assert_eq!(
        active
            .snapshot()
            .state
            .systems
            .values()
            .next()
            .unwrap()
            .name
            .as_str(),
        "Preserved"
    );
    assert!(
        active
            .snapshot()
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.message.contains("before importing"))
    );
}
