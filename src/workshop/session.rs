//! Client-only pacing and action coordination for Galaxy Workshop.

use std::{collections::VecDeque, time::Duration};

use nyon_workshop_core::{
    ActiveView, ArchiveDecodeJob, ArchiveDecodeStatus, BranchId, BranchRef, CreatorBatchV1,
    RevisionId, StateDigest, WorkshopHistory, WorkshopStateV1, encode_archive,
};

use super::store::{
    SaveGeneration, SlotId, SlotName, StoreJobId, StoreJobState, WorkshopStore,
    WorkshopStoreRequest, WorkshopStoreResult,
};

pub const WORKSHOP_FRAME_DELTA_LIMIT: Duration = Duration::from_millis(250);
pub const WORKSHOP_STEP_DURATION: Duration = Duration::from_millis(100);
pub const MAX_WORKSHOP_STEPS_PER_FRAME: usize = 20;
pub const MAX_WORKSHOP_ACTIONS_QUEUED: usize = 512;
pub const MAX_WORKSHOP_DIAGNOSTICS: usize = 100;
pub const WORKSHOP_AUTOSAVE_DEBOUNCE: Duration = Duration::from_millis(250);
pub const WORKSHOP_AUTOSAVE_RETRY_BACKOFF: Duration = Duration::from_secs(5);
pub const WORKSHOP_AUTOSAVE_TICK_INTERVAL: u64 = 600;
const ARCHIVE_REPLAY_UNITS_PER_UPDATE: u64 = 1_024;

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum WorkshopAction {
    Submit(CreatorBatchV1),
    Pause,
    Resume(WorkshopSpeed),
    StepOnce,
    Undo,
    Redo(RevisionId),
    SelectBranch(BranchId),
    RequestSave,
    RequestLoad(SlotId),
    RequestExport,
    RequestImport(Box<[u8]>),
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum WorkshopDiagnosticCode {
    ActionQueueFull,
    CreatorRejected,
    HistoryRejected,
    SimulationFault,
    ArchiveRejected,
    StoreRejected,
    StoreProtocol,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct WorkshopDiagnostic {
    pub code: WorkshopDiagnosticCode,
    pub message: String,
}

#[derive(Clone, Debug, Eq, PartialEq, thiserror::Error)]
pub enum WorkshopSessionError {
    #[error("Workshop action mailbox reached its {limit}-action limit")]
    ActionQueueFull { limit: usize },
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct WorkshopStoreSnapshot {
    pub slot: Option<SlotId>,
    /// The generation whose archive most recently passed authoritative replay.
    pub generation: Option<SaveGeneration>,
    /// The current durable store head used for compare-and-swap.
    pub head_generation: Option<SaveGeneration>,
    pub commit_pending: bool,
    pub load_pending: bool,
    pub dirty: bool,
    pub status: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct WorkshopSessionSnapshot {
    pub state: WorkshopStateV1,
    pub state_digest: StateDigest,
    pub active_view: ActiveView,
    pub branches: Vec<BranchRef>,
    pub revision_count: usize,
    pub speed: WorkshopSpeed,
    pub diagnostics: Vec<WorkshopDiagnostic>,
    pub store: WorkshopStoreSnapshot,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct WorkshopUpdate {
    pub drained_actions: usize,
    pub accepted_batches: usize,
    pub authority_steps: usize,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum CommitPurpose {
    Save {
        dirty_version: u64,
    },
    SelectContinue {
        slot: SlotId,
        generation: SaveGeneration,
    },
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct PendingCommit {
    job: StoreJobId,
    purpose: CommitPurpose,
}

#[derive(Debug)]
enum PendingDecode {
    Import(Box<ArchiveDecodeJob>),
    Load {
        decoder: Box<ArchiveDecodeJob>,
        slot: SlotId,
        generation: SaveGeneration,
        head_generation: SaveGeneration,
        recovered_from_previous: bool,
    },
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct ResidentSlot {
    slot: SlotId,
    generation: SaveGeneration,
    head_generation: SaveGeneration,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum RecoveryPersistenceObligation {
    Promote {
        slot: SlotId,
        recovered_generation: SaveGeneration,
        expected_head_generation: SaveGeneration,
    },
    SelectContinue {
        slot: SlotId,
        generation: SaveGeneration,
    },
}

pub struct WorkshopSession {
    history: WorkshopHistory,
    speed: WorkshopSpeed,
    resume_speed: WorkshopSpeed,
    clock: WorkshopClock,
    mailbox: VecDeque<WorkshopAction>,
    diagnostics: VecDeque<WorkshopDiagnostic>,
    snapshot: WorkshopSessionSnapshot,
    pending_commit: Option<PendingCommit>,
    pending_load: Option<StoreJobId>,
    pending_decode: Option<PendingDecode>,
    slot: Option<ResidentSlot>,
    recovery_persistence: Option<RecoveryPersistenceObligation>,
    dirty: bool,
    dirty_version: u64,
    dirty_elapsed: Duration,
    dirty_debounce_armed: bool,
    save_retry_backoff: Duration,
    save_requested: bool,
    select_continue_requested: bool,
    durable_save_requested: bool,
    continue_selected_generation: Option<(SlotId, SaveGeneration)>,
    exported_archive: Option<Box<[u8]>>,
    store_status: String,
}

impl WorkshopSession {
    pub fn new(mut history: WorkshopHistory) -> Self {
        history.set_paused(true);
        let snapshot = session_snapshot(
            &history,
            WorkshopSpeed::Paused,
            &VecDeque::new(),
            WorkshopStoreSnapshot {
                slot: None,
                generation: None,
                head_generation: None,
                commit_pending: false,
                load_pending: false,
                dirty: false,
                status: "Not saved".to_owned(),
            },
        );
        Self {
            history,
            speed: WorkshopSpeed::Paused,
            resume_speed: WorkshopSpeed::One,
            clock: WorkshopClock::default(),
            mailbox: VecDeque::new(),
            diagnostics: VecDeque::new(),
            snapshot,
            pending_commit: None,
            pending_load: None,
            pending_decode: None,
            slot: None,
            recovery_persistence: None,
            dirty: false,
            dirty_version: 0,
            dirty_elapsed: Duration::ZERO,
            dirty_debounce_armed: false,
            save_retry_backoff: Duration::ZERO,
            save_requested: false,
            select_continue_requested: false,
            durable_save_requested: false,
            continue_selected_generation: None,
            exported_archive: None,
            store_status: "Not saved".to_owned(),
        }
    }

    /// Restores a fully validated archive while retaining the store generation
    /// that future compare-and-swap commits must extend.
    pub fn from_loaded(
        history: WorkshopHistory,
        slot: SlotId,
        generation: SaveGeneration,
        head_generation: SaveGeneration,
        recovered_from_previous: bool,
    ) -> Self {
        let mut session = Self::new(history);
        session.slot = Some(ResidentSlot {
            slot,
            generation,
            head_generation,
        });
        session.continue_selected_generation =
            (!recovered_from_previous).then_some((slot, head_generation));
        session.recovery_persistence =
            recovered_from_previous.then_some(RecoveryPersistenceObligation::Promote {
                slot,
                recovered_generation: generation,
                expected_head_generation: head_generation,
            });
        session.save_requested = recovered_from_previous;
        session.store_status = if recovered_from_previous {
            "Recovered previous valid generation".to_owned()
        } else {
            "Loaded".to_owned()
        };
        session.durable_save_requested = false;
        session.refresh_snapshot();
        session
    }

    pub fn history(&self) -> &WorkshopHistory {
        &self.history
    }

    pub fn snapshot(&self) -> &WorkshopSessionSnapshot {
        &self.snapshot
    }

    pub const fn speed(&self) -> WorkshopSpeed {
        self.speed
    }

    pub fn replacement_active(&self) -> bool {
        self.pending_load.is_some()
            || self.pending_decode.is_some()
            || self.pending_commit.is_some()
    }

    /// Returns whether replacing this resident session could discard accepted
    /// authority state, queued actions, or an unpolled persistence job.
    pub fn replacement_blocked(&self) -> bool {
        self.dirty
            || !self.mailbox.is_empty()
            || self.save_requested
            || self.select_continue_requested
            || self.durable_save_requested
            || self.recovery_persistence.is_some()
            || self.replacement_active()
    }

    /// True only after the exact current generation was durably selected for
    /// Continue and no queued or in-flight work can change that generation.
    pub fn continue_ready(&self) -> bool {
        self.speed == WorkshopSpeed::Paused
            && !self.dirty
            && self.mailbox.is_empty()
            && !self.save_requested
            && !self.select_continue_requested
            && self.recovery_persistence.is_none()
            && !self.replacement_active()
            && self.slot.is_some()
            && self.continue_selected_generation
                == self.slot.map(|slot| (slot.slot, slot.head_generation))
    }

    /// Atomically schedules the pause and durable-save intent required before
    /// leaving a resident Workshop. Repeated callers share one latched request
    /// until the exact saved generation is selected for Continue.
    pub fn prepare_for_durable_transition(&mut self) -> Result<(), WorkshopSessionError> {
        if self.continue_ready() || self.durable_save_requested {
            return Ok(());
        }
        const ACTION_COUNT: usize = 2;
        if self.mailbox.len().saturating_add(ACTION_COUNT) > MAX_WORKSHOP_ACTIONS_QUEUED {
            self.push_diagnostic(
                WorkshopDiagnosticCode::ActionQueueFull,
                "Workshop action mailbox cannot accept the durable transition",
            );
            return Err(WorkshopSessionError::ActionQueueFull {
                limit: MAX_WORKSHOP_ACTIONS_QUEUED,
            });
        }
        self.mailbox.push_back(WorkshopAction::Pause);
        self.mailbox.push_back(WorkshopAction::RequestSave);
        self.durable_save_requested = true;
        Ok(())
    }

    pub fn enqueue(&mut self, action: WorkshopAction) -> Result<(), WorkshopSessionError> {
        if self.mailbox.len() >= MAX_WORKSHOP_ACTIONS_QUEUED {
            self.push_diagnostic(
                WorkshopDiagnosticCode::ActionQueueFull,
                "Workshop action mailbox is full",
            );
            return Err(WorkshopSessionError::ActionQueueFull {
                limit: MAX_WORKSHOP_ACTIONS_QUEUED,
            });
        }
        self.mailbox.push_back(action);
        Ok(())
    }

    pub fn take_exported_archive(&mut self) -> Option<Box<[u8]>> {
        self.exported_archive.take()
    }

    pub fn update(
        &mut self,
        frame_delta: Duration,
        store: &mut dyn WorkshopStore,
    ) -> WorkshopUpdate {
        let mut update = WorkshopUpdate::default();
        let mut manual_steps = 0usize;
        let mut replacement_this_update = false;
        let bounded_frame_delta = frame_delta.min(WORKSHOP_FRAME_DELTA_LIMIT);
        self.save_retry_backoff = self.save_retry_backoff.saturating_sub(bounded_frame_delta);

        while let Some(action) = self.mailbox.pop_front() {
            update.drained_actions += 1;
            match action {
                WorkshopAction::Submit(batch) => {
                    if self.authority_replacement_active() {
                        self.push_diagnostic(
                            WorkshopDiagnosticCode::CreatorRejected,
                            "Creator edits are unavailable while a Workshop replacement is active",
                        );
                    } else {
                        match self.history.submit(batch) {
                            Ok(_) => {
                                update.accepted_batches += 1;
                                self.mark_dirty();
                            }
                            Err(error) => self.push_diagnostic(
                                WorkshopDiagnosticCode::CreatorRejected,
                                error.to_string(),
                            ),
                        }
                    }
                }
                WorkshopAction::Pause => {
                    self.pause();
                    if self.dirty && self.pending_commit.is_none() {
                        self.save_requested = true;
                    }
                }
                WorkshopAction::Resume(speed) => {
                    if self.authority_replacement_active() {
                        self.push_diagnostic(
                            WorkshopDiagnosticCode::HistoryRejected,
                            "Playback is unavailable while a Workshop replacement is active",
                        );
                    } else if speed == WorkshopSpeed::Paused {
                        self.pause();
                    } else {
                        self.speed = speed;
                        self.resume_speed = speed;
                        self.history.set_paused(false);
                        self.clock.clear();
                    }
                }
                WorkshopAction::StepOnce => {
                    if self.speed == WorkshopSpeed::Paused && !self.replacement_active() {
                        manual_steps = manual_steps
                            .saturating_add(1)
                            .min(MAX_WORKSHOP_STEPS_PER_FRAME);
                    } else {
                        self.push_diagnostic(
                            WorkshopDiagnosticCode::HistoryRejected,
                            "Single-step requires a paused Workshop with no store replacement",
                        );
                    }
                }
                WorkshopAction::Undo => {
                    if self.replacement_active() {
                        self.push_diagnostic(
                            WorkshopDiagnosticCode::HistoryRejected,
                            "Undo is unavailable during a store replacement",
                        );
                    } else {
                        match self.history.undo() {
                            Ok(_) => self.mark_dirty(),
                            Err(error) => self.push_diagnostic(
                                WorkshopDiagnosticCode::HistoryRejected,
                                error.to_string(),
                            ),
                        }
                    }
                }
                WorkshopAction::Redo(revision) => {
                    if self.replacement_active() {
                        self.push_diagnostic(
                            WorkshopDiagnosticCode::HistoryRejected,
                            "Redo is unavailable during a store replacement",
                        );
                    } else {
                        match self.history.redo_to(revision) {
                            Ok(_) => self.mark_dirty(),
                            Err(error) => self.push_diagnostic(
                                WorkshopDiagnosticCode::HistoryRejected,
                                error.to_string(),
                            ),
                        }
                    }
                }
                WorkshopAction::SelectBranch(branch) => {
                    if self.replacement_active() {
                        self.push_diagnostic(
                            WorkshopDiagnosticCode::HistoryRejected,
                            "Branch switching is unavailable during a store replacement",
                        );
                    } else {
                        match self.history.switch_branch(branch) {
                            Ok(()) => self.mark_dirty(),
                            Err(error) => self.push_diagnostic(
                                WorkshopDiagnosticCode::HistoryRejected,
                                error.to_string(),
                            ),
                        }
                    }
                }
                WorkshopAction::RequestSave => {
                    // Save requests coalesce behind the one allowed commit or
                    // load/import job. This keeps menu/exit durability intact
                    // even when a store operation was already in flight.
                    self.save_retry_backoff = Duration::ZERO;
                    let current_persistence_covers_state =
                        self.pending_commit
                            .is_some_and(|pending| match pending.purpose {
                                CommitPurpose::Save { dirty_version } => {
                                    dirty_version == self.dirty_version
                                }
                                CommitPurpose::SelectContinue { .. } => !self.dirty,
                            });
                    if !current_persistence_covers_state {
                        self.save_requested = true;
                    }
                }
                WorkshopAction::RequestLoad(slot) => {
                    self.pause();
                    if self.dirty && self.pending_commit.is_none() {
                        self.save_requested = true;
                    }
                    if self.replacement_request_blocked() {
                        self.push_diagnostic(
                            WorkshopDiagnosticCode::StoreRejected,
                            "Save the current Workshop before loading another archive",
                        );
                    } else {
                        match store.start(WorkshopStoreRequest::LoadSlot { slot }) {
                            Ok(job) => {
                                self.pending_load = Some(job);
                                self.store_status = "Loading".to_owned();
                                replacement_this_update = true;
                            }
                            Err(error) => self.push_diagnostic(
                                WorkshopDiagnosticCode::StoreRejected,
                                error.to_string(),
                            ),
                        }
                    }
                }
                WorkshopAction::RequestExport => {
                    if self.authority_replacement_active() {
                        self.push_diagnostic(
                            WorkshopDiagnosticCode::StoreRejected,
                            "Archive export is unavailable while a Workshop replacement is active",
                        );
                    } else {
                        match encode_archive(&self.history) {
                            Ok(archive) => {
                                self.exported_archive = Some(archive.bytes.into_boxed_slice());
                                self.store_status = "Export ready".to_owned();
                            }
                            Err(error) => self.push_diagnostic(
                                WorkshopDiagnosticCode::ArchiveRejected,
                                error.to_string(),
                            ),
                        }
                    }
                }
                WorkshopAction::RequestImport(bytes) => {
                    self.pause();
                    if self.dirty && self.pending_commit.is_none() {
                        self.save_requested = true;
                    }
                    if self.replacement_request_blocked() {
                        self.push_diagnostic(
                            WorkshopDiagnosticCode::StoreRejected,
                            "Save the current Workshop before importing another archive",
                        );
                    } else {
                        let catalog = self.history.catalog().clone();
                        match ArchiveDecodeJob::new(&catalog, &bytes) {
                            Ok(decoder) => {
                                self.pending_decode =
                                    Some(PendingDecode::Import(Box::new(decoder)));
                                self.store_status = "Validating import".to_owned();
                                replacement_this_update = true;
                            }
                            Err(error) => self.push_diagnostic(
                                WorkshopDiagnosticCode::ArchiveRejected,
                                error.to_string(),
                            ),
                        }
                    }
                }
            }
        }

        if !replacement_this_update && self.pending_load.is_none() && self.pending_decode.is_none()
        {
            let tick_before_steps = self.history.state().tick.0;
            let scheduled = self.clock.steps_for_frame(frame_delta, self.speed);
            let step_count = manual_steps
                .saturating_add(scheduled)
                .min(MAX_WORKSHOP_STEPS_PER_FRAME);
            for _ in 0..step_count {
                match self.history.step() {
                    Ok(_) => update.authority_steps += 1,
                    Err(error) => {
                        self.push_diagnostic(
                            WorkshopDiagnosticCode::SimulationFault,
                            error.to_string(),
                        );
                        self.pause();
                        break;
                    }
                }
            }
            if update.authority_steps > 0 {
                self.mark_authority_dirty();
                let tick_after_steps = self.history.state().tick.0;
                if tick_before_steps / WORKSHOP_AUTOSAVE_TICK_INTERVAL
                    < tick_after_steps / WORKSHOP_AUTOSAVE_TICK_INTERVAL
                {
                    self.save_requested = true;
                }
            }
        } else {
            self.clock.clear();
        }

        self.poll_load(store);
        self.poll_decode();
        self.poll_commit(store);

        if self.save_retry_backoff.is_zero() && self.pending_commit.is_none() {
            match self.recovery_persistence {
                Some(RecoveryPersistenceObligation::Promote { .. }) => {
                    self.save_requested = true;
                }
                Some(RecoveryPersistenceObligation::SelectContinue { .. }) => {
                    self.select_continue_requested = true;
                }
                None => {}
            }
        }

        if self.dirty && self.dirty_debounce_armed {
            self.dirty_elapsed = self.dirty_elapsed.saturating_add(bounded_frame_delta);
            if self.save_retry_backoff.is_zero() && self.dirty_elapsed >= WORKSHOP_AUTOSAVE_DEBOUNCE
            {
                self.save_requested = true;
            }
        } else {
            self.dirty_elapsed = Duration::ZERO;
        }

        if self.pending_commit.is_none() {
            if self.select_continue_requested {
                self.start_select_continue(store);
            } else if self.save_requested
                && self.pending_load.is_none()
                && self.pending_decode.is_none()
            {
                self.start_save(store);
            }
        }

        self.refresh_snapshot();
        if self.continue_ready() {
            self.durable_save_requested = false;
        }
        update
    }

    fn pause(&mut self) {
        self.speed = WorkshopSpeed::Paused;
        self.history.set_paused(true);
        self.clock.clear();
    }

    fn authority_replacement_active(&self) -> bool {
        self.pending_load.is_some() || self.pending_decode.is_some()
    }

    fn replacement_request_blocked(&mut self) -> bool {
        // The current replacement action has already been removed from the
        // mailbox. Later actions remain ordered behind it and will observe the
        // replacement job it starts, so they are not pre-existing state that
        // this request could discard.
        let later_actions = std::mem::take(&mut self.mailbox);
        let blocked = self.replacement_blocked();
        self.mailbox = later_actions;
        blocked
    }

    fn mark_dirty(&mut self) {
        self.dirty = true;
        self.dirty_version = self.dirty_version.saturating_add(1);
        self.dirty_elapsed = Duration::ZERO;
        self.dirty_debounce_armed = true;
        self.save_retry_backoff = Duration::ZERO;
    }

    fn mark_authority_dirty(&mut self) {
        self.dirty = true;
        self.dirty_version = self.dirty_version.saturating_add(1);
    }

    fn poll_load(&mut self, store: &mut dyn WorkshopStore) {
        let Some(job) = self.pending_load else {
            return;
        };
        match store.poll(job) {
            StoreJobState::Pending => {}
            StoreJobState::Unknown => {
                self.pending_load = None;
                self.push_diagnostic(
                    WorkshopDiagnosticCode::StoreProtocol,
                    "Workshop store forgot an active load job",
                );
            }
            StoreJobState::Complete(result) => {
                self.pending_load = None;
                match result {
                    Ok(WorkshopStoreResult::SlotLoaded(loaded)) => {
                        let catalog = self.history.catalog().clone();
                        match ArchiveDecodeJob::new(&catalog, &loaded.archive) {
                            Ok(decoder) => {
                                self.pending_decode = Some(PendingDecode::Load {
                                    decoder: Box::new(decoder),
                                    slot: loaded.slot,
                                    generation: loaded.generation,
                                    head_generation: loaded.head_generation,
                                    recovered_from_previous: loaded.recovered_from_previous,
                                });
                                self.store_status = "Validating load".to_owned();
                            }
                            Err(error) => self.push_diagnostic(
                                WorkshopDiagnosticCode::ArchiveRejected,
                                error.to_string(),
                            ),
                        }
                    }
                    Ok(_) => self.push_diagnostic(
                        WorkshopDiagnosticCode::StoreProtocol,
                        "Workshop load job returned an unexpected result",
                    ),
                    Err(error) => self
                        .push_diagnostic(WorkshopDiagnosticCode::StoreRejected, error.to_string()),
                }
            }
        }
    }

    fn poll_decode(&mut self) {
        let Some(mut pending) = self.pending_decode.take() else {
            return;
        };
        let status = match &mut pending {
            PendingDecode::Import(decoder) | PendingDecode::Load { decoder, .. } => {
                decoder.poll(ARCHIVE_REPLAY_UNITS_PER_UPDATE)
            }
        };
        match status {
            Ok(ArchiveDecodeStatus::Pending) => self.pending_decode = Some(pending),
            Ok(ArchiveDecodeStatus::Complete) => {
                let (decoder, loaded) = match pending {
                    PendingDecode::Import(decoder) => (decoder, None),
                    PendingDecode::Load {
                        decoder,
                        slot,
                        generation,
                        head_generation,
                        recovered_from_previous,
                    } => (
                        decoder,
                        Some((slot, generation, head_generation, recovered_from_previous)),
                    ),
                };
                match (*decoder).finish() {
                    Ok(mut history) => {
                        history.set_paused(true);
                        self.history = history;
                        if let Some((slot, generation, head_generation, recovered)) = loaded {
                            self.slot = Some(ResidentSlot {
                                slot,
                                generation,
                                head_generation,
                            });
                            self.continue_selected_generation = None;
                            self.dirty = false;
                            self.dirty_elapsed = Duration::ZERO;
                            self.dirty_debounce_armed = false;
                            self.store_status = if recovered {
                                "Recovered previous valid generation".to_owned()
                            } else {
                                "Loaded".to_owned()
                            };
                            self.recovery_persistence =
                                recovered.then_some(RecoveryPersistenceObligation::Promote {
                                    slot,
                                    recovered_generation: generation,
                                    expected_head_generation: head_generation,
                                });
                            self.save_requested = recovered;
                        } else {
                            self.slot = None;
                            self.continue_selected_generation = None;
                            self.mark_dirty();
                            self.store_status = "Imported; not saved".to_owned();
                        }
                    }
                    Err(error) => self.push_diagnostic(
                        WorkshopDiagnosticCode::ArchiveRejected,
                        error.to_string(),
                    ),
                }
            }
            Err(error) => {
                self.push_diagnostic(WorkshopDiagnosticCode::ArchiveRejected, error.to_string())
            }
        }
    }

    fn poll_commit(&mut self, store: &mut dyn WorkshopStore) {
        let Some(pending) = self.pending_commit else {
            return;
        };
        match store.poll(pending.job) {
            StoreJobState::Pending => {}
            StoreJobState::Unknown => {
                self.pending_commit = None;
                match pending.purpose {
                    CommitPurpose::Save { .. } => self.defer_autosave_retry(),
                    CommitPurpose::SelectContinue { .. } => self.defer_continue_selection_retry(),
                }
                self.push_diagnostic(
                    WorkshopDiagnosticCode::StoreProtocol,
                    "Workshop store forgot an active commit job",
                );
            }
            StoreJobState::Complete(result) => {
                self.pending_commit = None;
                match (pending.purpose, result) {
                    (
                        CommitPurpose::Save { dirty_version },
                        Ok(WorkshopStoreResult::SlotCreated { slot, generation })
                        | Ok(WorkshopStoreResult::SlotCommitted { slot, generation }),
                    ) => {
                        let completed_recovery_promotion = matches!(
                            self.recovery_persistence,
                            Some(RecoveryPersistenceObligation::Promote { .. })
                        );
                        self.slot = Some(ResidentSlot {
                            slot,
                            generation,
                            head_generation: generation,
                        });
                        if self.dirty_version == dirty_version {
                            self.dirty = false;
                            self.dirty_elapsed = Duration::ZERO;
                            self.dirty_debounce_armed = false;
                        } else if self.dirty_debounce_armed {
                            self.save_requested = true;
                        }
                        if completed_recovery_promotion {
                            self.recovery_persistence =
                                Some(RecoveryPersistenceObligation::SelectContinue {
                                    slot,
                                    generation,
                                });
                        }
                        self.select_continue_requested = true;
                        self.store_status = "Saved".to_owned();
                    }
                    (
                        CommitPurpose::SelectContinue { slot, generation },
                        Ok(WorkshopStoreResult::ContinueSelected {
                            slot: selected_slot,
                        }),
                    ) => {
                        self.select_continue_requested = false;
                        if selected_slot == slot
                            && self
                                .slot
                                .map(|resident| (resident.slot, resident.head_generation))
                                == Some((slot, generation))
                        {
                            self.continue_selected_generation = Some((slot, generation));
                            if self.recovery_persistence
                                == Some(RecoveryPersistenceObligation::SelectContinue {
                                    slot,
                                    generation,
                                })
                            {
                                self.recovery_persistence = None;
                            }
                            self.store_status = "Saved and selected for Continue".to_owned();
                        } else {
                            self.push_diagnostic(
                                WorkshopDiagnosticCode::StoreProtocol,
                                "Workshop Continue selection returned a stale slot or generation",
                            );
                        }
                    }
                    (purpose, Ok(_)) => {
                        match purpose {
                            CommitPurpose::Save { .. } => self.defer_autosave_retry(),
                            CommitPurpose::SelectContinue { .. } => {
                                self.defer_continue_selection_retry();
                            }
                        }
                        self.push_diagnostic(
                            WorkshopDiagnosticCode::StoreProtocol,
                            "Workshop commit job returned an unexpected result",
                        );
                    }
                    (purpose, Err(error)) => {
                        match purpose {
                            CommitPurpose::Save { .. } => {
                                self.defer_autosave_retry();
                                self.store_status = "Save failed; retry delayed".to_owned();
                            }
                            CommitPurpose::SelectContinue { .. } => {
                                self.defer_continue_selection_retry();
                                self.store_status = "Saved; Continue selection failed".to_owned();
                            }
                        }
                        self.push_diagnostic(
                            WorkshopDiagnosticCode::StoreRejected,
                            error.to_string(),
                        );
                    }
                }
            }
        }
    }

    fn start_save(&mut self, store: &mut dyn WorkshopStore) {
        let archive = match encode_archive(&self.history) {
            Ok(archive) => archive.bytes.into_boxed_slice(),
            Err(error) => {
                self.defer_autosave_retry();
                self.push_diagnostic(WorkshopDiagnosticCode::ArchiveRejected, error.to_string());
                return;
            }
        };
        let request = if let Some(resident) = self.slot {
            if let Some(RecoveryPersistenceObligation::Promote {
                slot,
                recovered_generation,
                expected_head_generation,
            }) = self.recovery_persistence
            {
                if resident
                    != (ResidentSlot {
                        slot,
                        generation: recovered_generation,
                        head_generation: expected_head_generation,
                    })
                {
                    self.defer_autosave_retry();
                    self.push_diagnostic(
                        WorkshopDiagnosticCode::StoreProtocol,
                        "Workshop recovery generations changed before promotion",
                    );
                    return;
                }
                WorkshopStoreRequest::PromoteRecoveredSlot {
                    slot,
                    expected_head_generation,
                    recovered_generation,
                    archive,
                }
            } else {
                WorkshopStoreRequest::CommitSlot {
                    slot: resident.slot,
                    expected_generation: resident.head_generation,
                    archive,
                }
            }
        } else {
            let Ok(name) = SlotName::new("Workshop") else {
                self.push_diagnostic(
                    WorkshopDiagnosticCode::StoreProtocol,
                    "Built-in Workshop slot name is invalid",
                );
                return;
            };
            WorkshopStoreRequest::CreateSlot { name, archive }
        };
        match store.start(request) {
            Ok(job) => {
                self.pending_commit = Some(PendingCommit {
                    job,
                    purpose: CommitPurpose::Save {
                        dirty_version: self.dirty_version,
                    },
                });
                self.save_requested = false;
                self.dirty_debounce_armed = false;
                self.store_status = "Saving".to_owned();
            }
            Err(error) => {
                self.defer_autosave_retry();
                self.store_status = "Save failed; retry delayed".to_owned();
                self.push_diagnostic(WorkshopDiagnosticCode::StoreRejected, error.to_string());
            }
        }
    }

    fn start_select_continue(&mut self, store: &mut dyn WorkshopStore) {
        let Some(resident) = self.slot else {
            self.select_continue_requested = false;
            return;
        };
        match store.start(WorkshopStoreRequest::SelectContinue {
            slot: resident.slot,
        }) {
            Ok(job) => {
                self.pending_commit = Some(PendingCommit {
                    job,
                    purpose: CommitPurpose::SelectContinue {
                        slot: resident.slot,
                        generation: resident.head_generation,
                    },
                });
            }
            Err(error) => {
                self.defer_continue_selection_retry();
                self.store_status = "Saved; Continue selection failed".to_owned();
                self.push_diagnostic(WorkshopDiagnosticCode::StoreRejected, error.to_string());
            }
        }
    }

    fn defer_autosave_retry(&mut self) {
        self.save_requested = false;
        self.dirty_elapsed = Duration::ZERO;
        self.dirty_debounce_armed = self.dirty;
        self.save_retry_backoff = WORKSHOP_AUTOSAVE_RETRY_BACKOFF;
    }

    fn defer_continue_selection_retry(&mut self) {
        self.select_continue_requested = false;
        if matches!(
            self.recovery_persistence,
            Some(RecoveryPersistenceObligation::SelectContinue { .. })
        ) {
            self.save_retry_backoff = WORKSHOP_AUTOSAVE_RETRY_BACKOFF;
        }
    }

    fn push_diagnostic(&mut self, code: WorkshopDiagnosticCode, message: impl Into<String>) {
        if self.diagnostics.len() == MAX_WORKSHOP_DIAGNOSTICS {
            self.diagnostics.pop_front();
        }
        self.diagnostics.push_back(WorkshopDiagnostic {
            code,
            message: message.into(),
        });
    }

    fn refresh_snapshot(&mut self) {
        self.snapshot = session_snapshot(
            &self.history,
            self.speed,
            &self.diagnostics,
            WorkshopStoreSnapshot {
                slot: self.slot.map(|resident| resident.slot),
                generation: self.slot.map(|resident| resident.generation),
                head_generation: self.slot.map(|resident| resident.head_generation),
                commit_pending: self.pending_commit.is_some(),
                load_pending: self.pending_load.is_some() || self.pending_decode.is_some(),
                dirty: self.dirty,
                status: self.store_status.clone(),
            },
        );
    }
}

fn session_snapshot(
    history: &WorkshopHistory,
    speed: WorkshopSpeed,
    diagnostics: &VecDeque<WorkshopDiagnostic>,
    store: WorkshopStoreSnapshot,
) -> WorkshopSessionSnapshot {
    WorkshopSessionSnapshot {
        state: history.state().clone(),
        state_digest: history.active_state_digest(),
        active_view: history.active_view().clone(),
        branches: history.branches().values().cloned().collect(),
        revision_count: history.revision_count(),
        speed,
        diagnostics: diagnostics.iter().cloned().collect(),
        store,
    }
}

#[derive(Clone, Copy, Debug, Default, Eq, Ord, PartialEq, PartialOrd)]
pub enum WorkshopSpeed {
    #[default]
    Paused,
    One,
    Four,
    Twenty,
}

impl WorkshopSpeed {
    pub const fn multiplier(self) -> u32 {
        match self {
            Self::Paused => 0,
            Self::One => 1,
            Self::Four => 4,
            Self::Twenty => 20,
        }
    }
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct WorkshopClock {
    accumulator: Duration,
}

impl WorkshopClock {
    pub fn clear(&mut self) {
        self.accumulator = Duration::ZERO;
    }

    pub fn accumulator(&self) -> Duration {
        self.accumulator
    }

    /// Returns the bounded number of exact 100 ms authority steps due for one
    /// rendered frame. Any whole-tick backlog beyond the per-frame budget is
    /// discarded while the sub-tick remainder is retained.
    pub fn steps_for_frame(&mut self, delta: Duration, speed: WorkshopSpeed) -> usize {
        let multiplier = speed.multiplier();
        if multiplier == 0 {
            self.clear();
            return 0;
        }
        let clamped = delta.min(WORKSHOP_FRAME_DELTA_LIMIT);
        self.accumulator = self
            .accumulator
            .saturating_add(clamped.saturating_mul(multiplier));

        let tick_nanos = WORKSHOP_STEP_DURATION.as_nanos();
        let due = self.accumulator.as_nanos() / tick_nanos;
        let steps = usize::try_from(due)
            .unwrap_or(usize::MAX)
            .min(MAX_WORKSHOP_STEPS_PER_FRAME);
        let remainder = self.accumulator.as_nanos() % tick_nanos;
        self.accumulator = Duration::from_nanos(
            u64::try_from(remainder).expect("100 ms remainder always fits in u64 nanoseconds"),
        );
        steps
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pause_clamp_and_step_budget_are_explicit() {
        let mut clock = WorkshopClock::default();
        assert_eq!(
            clock.steps_for_frame(Duration::from_secs(10), WorkshopSpeed::Paused),
            0
        );
        assert_eq!(clock.accumulator(), Duration::ZERO);
        assert_eq!(
            clock.steps_for_frame(Duration::from_secs(10), WorkshopSpeed::Twenty),
            MAX_WORKSHOP_STEPS_PER_FRAME
        );
        assert!(clock.accumulator() < WORKSHOP_STEP_DURATION);
    }

    #[test]
    fn speed_is_only_bounded_repetition_of_the_ten_hertz_step() {
        for (speed, expected) in [
            (WorkshopSpeed::One, 1),
            (WorkshopSpeed::Four, 4),
            (WorkshopSpeed::Twenty, 20),
        ] {
            let mut clock = WorkshopClock::default();
            assert_eq!(
                clock.steps_for_frame(Duration::from_millis(100), speed),
                expected
            );
        }
    }
}
