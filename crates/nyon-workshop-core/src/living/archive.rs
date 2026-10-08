//! Canonical portable Living documents. The section-10 top-level field order
//! is normative. Nested history records retain accepted creator timing and
//! branch provenance; immutable revision identity bytes remain unchanged.
//! Replay starts from embedded validated genesis, never from a generator or a
//! checkpoint. Replay is cancellable; complete nested forward budgets are separate
//! from the counted retirement API.
use super::catalog::ValidatedLivingCatalogPackV2;
use super::command::{LivingCommandErrorV2, LivingCommandModeV2};
use super::genesis::{
    LivingGenesisErrorV2, LivingGenesisGeneratorV2, LivingGenesisManifestV2,
    validate_living_genesis_manifest_v2,
};
use super::history::{
    LivingComparisonV2, LivingHistoryBranchV2, LivingHistoryErrorV2, LivingHistoryViewV2,
    LivingRecordedRevisionV2,
};
use super::ids::{
    LivingArchiveIntegrityV2, LivingBranchIdV2, LivingCatalogHashV2, LivingGenesisSeedV2,
    LivingRevisionIdV2, LivingStateDigestV2, LivingTickV2, archive_integrity_v2, fork_branch_id_v2,
    genesis_fork_branch_id_v2, revision_id_v2,
};
use super::simulation::{
    LivingAuthorityErrorV2, LivingDeterministicFaultV2, LivingGalaxyAuthorityV2,
    LivingReplayBoundaryV2,
};
use super::wire::{
    LIVING_MAX_ARCHIVE_BYTES_V2, LivingSortedVecV2, LivingWireErrorV2, decode_canonical_v2,
    encode_canonical_v2,
};
mod retirement;
use retirement::ArchiveSlot;
pub use retirement::LivingArchiveRetirementJobV2;

use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

/// Exact portable document kind.
pub const LIVING_ARCHIVE_KIND_V2: &str = "NYON_LIVING_GALAXY_ARCHIVE";
/// Canonical archive format version.
pub const LIVING_ARCHIVE_FORMAT_VERSION_V2: u32 = 2;
/// Maximum work units accepted by one replay poll.
pub const LIVING_MAX_REPLAY_POLL_WORK_V2: usize = 1_024;

/// Unvalidated portable bytes represented in their canonical field order.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct LivingArchiveV2 {
    /// Always the Living archive kind.
    pub kind: String,
    /// Always two.
    pub format_version: u32,
    /// Always Living rules two.
    pub rules_version: u32,
    /// Exact separately supplied content catalog.
    pub catalog_hash: LivingCatalogHashV2,
    /// Original fixed-width seed, never regenerated.
    pub genesis_seed: LivingGenesisSeedV2,
    /// Provenance only; never executed while decoding.
    pub genesis_generator: LivingGenesisGeneratorV2,
    /// Validated tick-zero replay truth.
    pub genesis_manifest: LivingGenesisManifestV2,
    /// Sorted immutable creator inputs, including accepted phase metadata.
    pub revisions: LivingSortedVecV2<LivingRecordedRevisionV2>,
    /// Sorted retained branch provenance and future heads.
    pub branches: LivingSortedVecV2<LivingHistoryBranchV2>,
    /// Exact selected branch, creator cursor and completed tick.
    pub active_view: LivingHistoryViewV2,
    /// Must equal active_view.tick.
    pub final_tick: LivingTickV2,
    /// Verified by replay before installation.
    pub final_digest: LivingStateDigestV2,
    /// Durable accepted-command allocator, independent of historical state.
    pub accepted_sequence_high_water: u64,
    /// Durable fork allocator, independent of historical state.
    pub branch_sequence_high_water: u64,
    /// Integrity over the ordered payload without this field.
    pub integrity_sha256: LivingArchiveIntegrityV2,
}
#[derive(Serialize)]
struct ArchivePayload<'a> {
    kind: &'a str,
    format_version: u32,
    rules_version: u32,
    catalog_hash: LivingCatalogHashV2,
    genesis_seed: LivingGenesisSeedV2,
    genesis_generator: &'a LivingGenesisGeneratorV2,
    genesis_manifest: &'a LivingGenesisManifestV2,
    revisions: &'a LivingSortedVecV2<LivingRecordedRevisionV2>,
    branches: &'a LivingSortedVecV2<LivingHistoryBranchV2>,
    active_view: LivingHistoryViewV2,
    final_tick: LivingTickV2,
    final_digest: LivingStateDigestV2,
    accepted_sequence_high_water: u64,
    branch_sequence_high_water: u64,
}
impl LivingArchiveV2 {
    fn integrity(&self) -> Result<LivingArchiveIntegrityV2, LivingWireErrorV2> {
        let bytes = encode_canonical_v2(
            &ArchivePayload {
                kind: &self.kind,
                format_version: self.format_version,
                rules_version: self.rules_version,
                catalog_hash: self.catalog_hash,
                genesis_seed: self.genesis_seed,
                genesis_generator: &self.genesis_generator,
                genesis_manifest: &self.genesis_manifest,
                revisions: &self.revisions,
                branches: &self.branches,
                active_view: self.active_view,
                final_tick: self.final_tick,
                final_digest: self.final_digest,
                accepted_sequence_high_water: self.accepted_sequence_high_water,
                branch_sequence_high_water: self.branch_sequence_high_water,
            },
            LIVING_MAX_ARCHIVE_BYTES_V2,
        )?;
        Ok(LivingArchiveIntegrityV2(archive_integrity_v2(&bytes)))
    }
}
/// A portable archive whose graph, genesis, integrity and final digest were
/// actually validated and replayed, with all sibling futures reconstructed.
#[derive(Clone, Debug)]
pub struct ValidatedLivingArchiveV2 {
    archive: ArchiveSlot<LivingArchiveV2>,
    authority: ArchiveSlot<LivingGalaxyAuthorityV2>,
    canonical_bytes: Vec<u8>,
}
impl ValidatedLivingArchiveV2 {
    /// Validated document and storage metadata.
    pub const fn archive(&self) -> &LivingArchiveV2 {
        self.archive.get()
    }
    /// Exact reconstructed selected session. Panics after `take_authority` consumes it.
    pub const fn authority(&self) -> &LivingGalaxyAuthorityV2 {
        self.authority.get()
    }
    /// Synchronous compatibility: installs the authority and drops residual owners.
    pub fn into_authority(mut self) -> LivingGalaxyAuthorityV2 {
        self.take_authority()
            .expect("validated authority already extracted")
    }
    /// Move the selected authority once, retaining document/bytes for retirement.
    pub fn take_authority(&mut self) -> Result<LivingGalaxyAuthorityV2, LivingArchiveErrorV2> {
        self.authority.take().ok_or(LivingArchiveErrorV2::NotReady)
    }
    /// Bytes checked during decode, with no derived state cache included.
    pub fn canonical_bytes(&self) -> &[u8] {
        &self.canonical_bytes
    }
}
/// A decode/export failure never installs a partial authority.
#[derive(Clone, Debug, Eq, PartialEq, thiserror::Error)]
pub enum LivingArchiveErrorV2 {
    /// Different document kind.
    #[error("the document is not a Living Galaxy archive")]
    Kind,
    /// Unsupported format version.
    #[error("unsupported Living archive format {found}")]
    FormatVersion {
        /// Version supplied.
        found: u32,
    },
    /// Unsupported authority rules.
    #[error("unsupported Living archive rules {found}")]
    RulesVersion {
        /// Version supplied.
        found: u32,
    },
    /// Supplied catalog is not the archive's exact catalog.
    #[error("Living archive catalog hash does not match supplied catalog")]
    CatalogMismatch,
    /// Ordered payload integrity mismatch.
    #[error("Living archive integrity does not match its payload")]
    Integrity,
    /// Invalid immutable revision/branch graph or timing.
    #[error("Living archive history graph is invalid")]
    HistoryGraph,
    /// Allocator marks undercount already issued identities.
    #[error("Living archive allocator high-water marks undercount history")]
    HighWater,
    /// Selected view tick and/or digest differs from replay.
    #[error("Living archive final state does not match replay")]
    FinalDigest,
    /// Accepted creator work is not committed; it must be handled explicitly.
    #[error("commit or explicitly discard the Living creator queue before export")]
    PendingQueue,
    /// Incremental replay was explicitly cancelled.
    #[error("Living archive replay was cancelled")]
    Cancelled,
    /// Replay has not completed yet.
    #[error("Living archive replay is not complete")]
    NotReady,
    /// An invalid poll budget was supplied.
    #[error("Living replay poll budget must be between 1 and 1024")]
    Budget,
    /// Canonical byte validation failure.
    #[error("Living archive wire error: {0}")]
    Wire(#[from] LivingWireErrorV2),
    /// Embedded genesis is invalid.
    #[error("Living archive genesis error: {0}")]
    Genesis(#[from] LivingGenesisErrorV2),
    /// Creator command or identity is invalid.
    #[error("Living archive command error: {0}")]
    Command(#[from] LivingCommandErrorV2),
    /// Requested navigation is not a retained view.
    #[error("Living archive history error: {0}")]
    History(#[from] LivingHistoryErrorV2),
    /// Authority initialization failed.
    #[error("Living archive authority error: {0}")]
    Authority(#[from] LivingAuthorityErrorV2),
    /// Deterministic replay faulted at its last valid boundary.
    #[error("Living archive replay error: {0}")]
    Replay(#[from] LivingDeterministicFaultV2),
}

/// Export committed creator inputs, embedded genesis and retained futures.
/// Pending queues are never silently dropped; spent allocator marks survive.
pub fn encode_living_archive_v2(
    authority: &LivingGalaxyAuthorityV2,
) -> Result<Vec<u8>, LivingArchiveErrorV2> {
    if authority.published_cursor().pending_sequence != 0 {
        return Err(LivingArchiveErrorV2::PendingQueue);
    }
    let (revisions, branches) = authority.canonical_history()?;
    let view = authority
        .bookmark()
        .map_err(|_| LivingArchiveErrorV2::HistoryGraph)?;
    let mut archive = LivingArchiveV2 {
        kind: LIVING_ARCHIVE_KIND_V2.to_owned(),
        format_version: LIVING_ARCHIVE_FORMAT_VERSION_V2,
        rules_version: super::LIVING_RULES_VERSION,
        catalog_hash: authority.catalog().catalog_hash(),
        genesis_seed: LivingGenesisSeedV2(authority.genesis_seed()),
        genesis_generator: authority.generator().clone(),
        genesis_manifest: authority.genesis_manifest(),
        revisions: LivingSortedVecV2::new(revisions)?,
        branches: LivingSortedVecV2::new(branches)?,
        active_view: view.view,
        final_tick: view.view.tick,
        final_digest: view.digest,
        accepted_sequence_high_water: authority.accepted_sequence_high_water(),
        branch_sequence_high_water: authority.branch_sequence_high_water(),
        integrity_sha256: LivingArchiveIntegrityV2([0; 32]),
    };
    archive.integrity_sha256 = archive.integrity()?;
    encode_canonical_v2(&archive, LIVING_MAX_ARCHIVE_BYTES_V2).map_err(Into::into)
}

#[derive(Clone, Debug)]
enum ReplayAction {
    Advance(LivingTickV2),
    Paused(LivingRecordedRevisionV2),
    Running {
        tick: LivingTickV2,
        records: Vec<LivingRecordedRevisionV2>,
    },
    Fork {
        branch: LivingBranchIdV2,
        ordinal: u64,
    },
}
#[derive(Clone, Debug)]
struct ReplayTask {
    view: LivingHistoryViewV2,
    actions: Vec<ReplayAction>,
}
/// Public progress has no partially installable state.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum LivingArchiveReplayStatusV2 {
    /// More bounded work remains.
    Pending,
    /// finish() can now return the validated archive.
    Ready,
}
/// Cancellable reconstruction. Each work unit advances one fixed boundary,
/// applies one paused creator input, transitions a branch, or completes a view.
#[derive(Debug)]
pub struct LivingArchiveReplayV2 {
    archive: ArchiveSlot<LivingArchiveV2>,
    canonical_bytes: Vec<u8>,
    retired_bytes: Vec<u8>,
    origin: ArchiveSlot<LivingGalaxyAuthorityV2>,
    tasks: Vec<ReplayTask>,
    task: usize,
    action: usize,
    working: Option<LivingGalaxyAuthorityV2>,
    completed: Vec<LivingGalaxyAuthorityV2>,
    validated: Option<ValidatedLivingArchiveV2>,
    cancelled: bool,
    staged: Option<LivingReplayBoundaryV2>,
    staged_record: usize,
    verify_final: bool,
    redo_after: Vec<LivingHistoryViewV2>,
    failure: Option<LivingArchiveErrorV2>,
    delivered: bool,
}
impl LivingArchiveReplayV2 {
    /// Retain reconstructed owners for explicit retirement; stop execution.
    pub fn cancel(&mut self) {
        self.cancelled = true;
    }
    /// Execute at most the supplied work units. Budgets outside 1..=1024 reject.
    pub fn poll(
        &mut self,
        budget: usize,
    ) -> Result<LivingArchiveReplayStatusV2, LivingArchiveErrorV2> {
        if self.cancelled {
            return Err(LivingArchiveErrorV2::Cancelled);
        }
        if let Some(error) = &self.failure {
            return Err(error.clone());
        }
        if self.delivered {
            return Err(LivingArchiveErrorV2::NotReady);
        }
        let result = self.poll_work(budget);
        if let Err(error) = &result
            && *error != LivingArchiveErrorV2::Budget
        {
            self.failure = Some(error.clone());
        }
        result
    }
    fn poll_work(
        &mut self,
        budget: usize,
    ) -> Result<LivingArchiveReplayStatusV2, LivingArchiveErrorV2> {
        if self.cancelled {
            return Err(LivingArchiveErrorV2::Cancelled);
        }
        if !(1..=LIVING_MAX_REPLAY_POLL_WORK_V2).contains(&budget) {
            return Err(LivingArchiveErrorV2::Budget);
        }
        if self.validated.is_some() {
            return Ok(LivingArchiveReplayStatusV2::Ready);
        }
        for _ in 0..budget {
            if self.task == self.tasks.len() {
                let active = self
                    .completed
                    .last()
                    .ok_or(LivingArchiveErrorV2::HistoryGraph)?;
                if active.state().tick != self.archive.final_tick
                    || (self.verify_final && active.state().digest()? != self.archive.final_digest)
                {
                    return Err(LivingArchiveErrorV2::FinalDigest);
                }
                if !self.verify_final {
                    self.archive.final_digest = active.state().digest()?;
                    self.archive.integrity_sha256 = self.archive.integrity()?;
                    let bytes =
                        encode_canonical_v2(self.archive.get(), LIVING_MAX_ARCHIVE_BYTES_V2)?;
                    self.retired_bytes = std::mem::replace(&mut self.canonical_bytes, bytes);
                }
                let present = self.completed[..self.completed.len() - 1]
                    .iter()
                    .find(|branch| branch.branch() == active.branch())
                    .cloned()
                    .ok_or(LivingArchiveErrorV2::HistoryGraph)?;
                let mut active = self.completed.pop().expect("checked active owner");
                active.install_history(
                    std::mem::take(&mut self.completed),
                    self.archive.accepted_sequence_high_water,
                    self.archive.branch_sequence_high_water,
                    present,
                );
                active.install_redo_targets(std::mem::take(&mut self.redo_after));
                self.validated = Some(ValidatedLivingArchiveV2 {
                    archive: self.archive.get().clone().into(),
                    authority: active.into(),
                    canonical_bytes: self.canonical_bytes.clone(),
                });
                return Ok(LivingArchiveReplayStatusV2::Ready);
            }
            let task = &self.tasks[self.task];
            if self.working.is_none() {
                self.working = Some(self.origin.get().clone());
            }
            let working = self
                .working
                .as_mut()
                .ok_or(LivingArchiveErrorV2::HistoryGraph)?;
            if self.action == task.actions.len() {
                if working.state().tick != task.view.tick
                    || working.branch() != task.view.branch
                    || working.published_cursor().committed_revision != task.view.revision
                {
                    return Err(LivingArchiveErrorV2::HistoryGraph);
                }
                self.completed.push(
                    self.working
                        .take()
                        .ok_or(LivingArchiveErrorV2::HistoryGraph)?,
                );
                self.task += 1;
                self.action = 0;
                continue;
            }
            match &task.actions[self.action] {
                ReplayAction::Advance(tick) => {
                    if working.state().tick.0 < tick.0 {
                        working.step()?;
                    } else if working.state().tick == *tick {
                        self.action += 1;
                    } else {
                        return Err(LivingArchiveErrorV2::HistoryGraph);
                    }
                }
                ReplayAction::Paused(record) => {
                    if working.state().tick.0 < record.revision.tick.0 {
                        working.step()?;
                    } else {
                        working.replay_paused(record)?;
                        self.action += 1;
                    }
                }
                ReplayAction::Running { tick, records } => {
                    if working
                        .state()
                        .tick
                        .0
                        .checked_add(1)
                        .ok_or(LivingArchiveErrorV2::HistoryGraph)?
                        < tick.0
                    {
                        working.step()?;
                    } else if working.state().tick.0 + 1 == tick.0 {
                        if self.staged.is_none() {
                            self.staged = Some(working.begin_replay_boundary()?);
                            self.staged_record = 0;
                        }
                        if self.staged_record < records.len() {
                            working.stage_replay_record(
                                self.staged
                                    .as_mut()
                                    .ok_or(LivingArchiveErrorV2::HistoryGraph)?,
                                &records[self.staged_record],
                            )?;
                            self.staged_record += 1;
                        } else {
                            working.finish_replay_boundary(
                                self.staged
                                    .take()
                                    .ok_or(LivingArchiveErrorV2::HistoryGraph)?,
                            )?;
                            self.staged_record = 0;
                            self.action += 1;
                        }
                    } else {
                        return Err(LivingArchiveErrorV2::HistoryGraph);
                    }
                }
                ReplayAction::Fork { branch, ordinal } => {
                    working.replay_fork(*branch, *ordinal);
                    self.action += 1;
                }
            }
        }
        Ok(LivingArchiveReplayStatusV2::Pending)
    }
    /// Borrowed once-only extraction; rejection retains every owner.
    pub fn take_ready_result(&mut self) -> Result<ValidatedLivingArchiveV2, LivingArchiveErrorV2> {
        if self.cancelled {
            return Err(LivingArchiveErrorV2::Cancelled);
        }
        if let Some(error) = &self.failure {
            return Err(error.clone());
        }
        if self.delivered
            || self.validated.is_none()
            || self.task != self.tasks.len()
            || self.staged.is_some()
            || self.working.is_some()
        {
            return Err(LivingArchiveErrorV2::NotReady);
        }
        self.delivered = true;
        Ok(self.validated.take().expect("checked output"))
    }
    /// Synchronous compatibility: consumes the wrapper and its residual owners.
    pub fn finish(mut self) -> Result<ValidatedLivingArchiveV2, LivingArchiveErrorV2> {
        self.take_ready_result()
    }
}

/// Canonically decode and structurally validate before starting cancellable replay.
pub fn begin_decode_living_archive_v2(
    bytes: &[u8],
    catalog: &ValidatedLivingCatalogPackV2,
) -> Result<LivingArchiveReplayV2, LivingArchiveErrorV2> {
    let archive: LivingArchiveV2 = decode_canonical_v2(bytes, LIVING_MAX_ARCHIVE_BYTES_V2)?;
    if archive.kind != LIVING_ARCHIVE_KIND_V2 {
        return Err(LivingArchiveErrorV2::Kind);
    }
    if archive.format_version != LIVING_ARCHIVE_FORMAT_VERSION_V2 {
        return Err(LivingArchiveErrorV2::FormatVersion {
            found: archive.format_version,
        });
    }
    if archive.rules_version != super::LIVING_RULES_VERSION {
        return Err(LivingArchiveErrorV2::RulesVersion {
            found: archive.rules_version,
        });
    }
    if archive.catalog_hash != catalog.catalog_hash() {
        return Err(LivingArchiveErrorV2::CatalogMismatch);
    }
    if archive.integrity()? != archive.integrity_sha256 {
        return Err(LivingArchiveErrorV2::Integrity);
    }
    let genesis = validate_living_genesis_manifest_v2(archive.genesis_manifest.clone(), catalog)?;
    let origin = LivingGalaxyAuthorityV2::from_genesis(
        catalog.clone(),
        archive.genesis_seed.0,
        archive.genesis_generator.clone(),
        genesis,
    )?;
    validate_graph(&archive, origin.branch())?;
    let records: BTreeMap<_, _> = archive
        .revisions
        .as_slice()
        .iter()
        .map(|record| (record.revision.id, record.clone()))
        .collect();
    let branches: BTreeMap<_, _> = archive
        .branches
        .as_slice()
        .iter()
        .map(|branch| (branch.id, branch.clone()))
        .collect();
    let mut tasks = Vec::new();
    for branch in archive.branches.as_slice() {
        let view = LivingHistoryViewV2 {
            branch: branch.id,
            revision: branch.head,
            tick: branch.tip_tick,
        };
        tasks.push(ReplayTask {
            view,
            actions: schedule(view, &records, &branches)?,
        });
    }
    tasks.push(ReplayTask {
        view: archive.active_view,
        actions: schedule(archive.active_view, &records, &branches)?,
    });
    Ok(LivingArchiveReplayV2 {
        archive: archive.into(),
        canonical_bytes: bytes.to_vec(),
        retired_bytes: Vec::new(),
        origin: origin.into(),
        tasks,
        task: 0,
        action: 0,
        working: None,
        completed: Vec::new(),
        validated: None,
        cancelled: false,
        staged: None,
        staged_record: 0,
        verify_final: true,
        redo_after: Vec::new(),
        failure: None,
        delivered: false,
    })
}
/// Reconstruct a retained history view without blocking the event thread on
/// simulation. No state is installed until finish() returns a validated result.
pub fn begin_restore_living_view_v2(
    authority: &LivingGalaxyAuthorityV2,
    view: LivingHistoryViewV2,
) -> Result<LivingArchiveReplayV2, LivingArchiveErrorV2> {
    let bytes = encode_living_archive_v2(authority)?;
    let mut replay = begin_decode_living_archive_v2(&bytes, authority.catalog())?;
    let document_tick = replay
        .archive
        .branches
        .as_slice()
        .iter()
        .map(|branch| branch.tip_tick.0)
        .max()
        .unwrap_or(0);
    if view.tick.0 > document_tick {
        return Err(LivingArchiveErrorV2::HistoryGraph);
    }
    let mut branches_owned = replay.archive.branches.clone().into_vec();
    let branch = branches_owned
        .iter_mut()
        .find(|branch| branch.id == view.branch)
        .ok_or(LivingArchiveErrorV2::HistoryGraph)?;
    branch.tip_tick = LivingTickV2(branch.tip_tick.0.max(view.tick.0));
    replay.archive.branches = LivingSortedVecV2::new(branches_owned)?;
    let records = replay
        .archive
        .revisions
        .as_slice()
        .iter()
        .map(|record| (record.revision.id, record.clone()))
        .collect();
    let branches = replay
        .archive
        .branches
        .as_slice()
        .iter()
        .map(|branch| (branch.id, branch.clone()))
        .collect();
    replay.tasks = replay
        .archive
        .branches
        .as_slice()
        .iter()
        .map(|branch| {
            let tip = LivingHistoryViewV2 {
                branch: branch.id,
                revision: branch.head,
                tick: branch.tip_tick,
            };
            Ok(ReplayTask {
                view: tip,
                actions: schedule(tip, &records, &branches)?,
            })
        })
        .collect::<Result<Vec<_>, LivingArchiveErrorV2>>()?;
    replay.tasks.push(ReplayTask {
        view,
        actions: schedule(view, &records, &branches)?,
    });
    let actions = schedule(view, &records, &branches)?;
    let active = replay
        .tasks
        .last_mut()
        .ok_or(LivingArchiveErrorV2::HistoryGraph)?;
    *active = ReplayTask { view, actions };
    replay.archive.active_view = view;
    replay.archive.final_tick = view.tick;
    replay.verify_final = false;
    Ok(replay)
}
/// Bounded same-tick counterfactual Undo, preserving the exact Redo target.
pub fn begin_undo_living_v2(
    authority: &LivingGalaxyAuthorityV2,
) -> Result<LivingArchiveReplayV2, LivingArchiveErrorV2> {
    let mut replay = begin_restore_living_view_v2(authority, authority.undo_view()?)?;
    replay.redo_after = authority.undo_redo_targets();
    replay.redo_after.push(authority.bookmark()?.view);
    Ok(replay)
}
/// Bounded same-tick Redo, preserving any remaining undone creator cursors.
pub fn begin_redo_living_v2(
    authority: &LivingGalaxyAuthorityV2,
) -> Result<LivingArchiveReplayV2, LivingArchiveErrorV2> {
    let mut replay = begin_restore_living_view_v2(authority, authority.redo_view()?)?;
    replay.redo_after = authority.undo_redo_targets();
    replay.redo_after.pop();
    Ok(replay)
}

/// Bounded two-branch materialization with no installable intermediate result.
#[derive(Debug)]
pub struct LivingComparisonReplayV2 {
    left: Option<LivingArchiveReplayV2>,
    right: Option<LivingArchiveReplayV2>,
    left_ready: Option<ValidatedLivingArchiveV2>,
    right_ready: Option<ValidatedLivingArchiveV2>,
    comparison: Option<LivingComparisonV2>,
    cleanup: Option<Box<LivingArchiveRetirementJobV2>>,
    stage: u8,
    cancelled: bool,
    delivered: bool,
    failure: Option<LivingArchiveErrorV2>,
}
impl LivingComparisonReplayV2 {
    /// Stop execution, retaining both children, outputs and partial cleanup.
    pub fn cancel(&mut self) {
        self.cancelled = true;
    }
    /// Forward replay still uses synchronous phase helpers. Cleanup work uses
    /// the same remaining budget and retains output anchors until it finishes.
    pub fn poll(
        &mut self,
        budget: usize,
    ) -> Result<LivingArchiveReplayStatusV2, LivingArchiveErrorV2> {
        if self.cancelled {
            return Err(LivingArchiveErrorV2::Cancelled);
        }
        if !(1..=LIVING_MAX_REPLAY_POLL_WORK_V2).contains(&budget) {
            return Err(LivingArchiveErrorV2::Budget);
        }
        if let Some(error) = &self.failure {
            return Err(error.clone());
        }
        if self.delivered {
            return Err(LivingArchiveErrorV2::NotReady);
        }
        let result = self.poll_work(budget);
        if let Err(error) = &result {
            self.failure = Some(error.clone());
        }
        result
    }
    fn poll_work(
        &mut self,
        budget: usize,
    ) -> Result<LivingArchiveReplayStatusV2, LivingArchiveErrorV2> {
        use super::simulation::{
            LivingPublicationRetirementPolicyV2 as Policy,
            LivingPublicationRetirementProgressV2 as Progress,
        };
        for _ in 0..budget {
            if let Some(cleanup) = &mut self.cleanup {
                let progress = cleanup.poll(1)?;
                if cleanup.last_poll_visits() != 0 {
                    continue;
                }
                assert_eq!(
                    progress,
                    Progress::Ready,
                    "internal cleanup only releases handles"
                );
                self.cleanup.take(); // empty original retirement Box, one visit
                self.stage += 1;
                if self.stage == 3 {
                    return Ok(LivingArchiveReplayStatusV2::Pending);
                }
                continue;
            }
            match self.stage {
                0 => {
                    let left = self
                        .left
                        .as_mut()
                        .ok_or(LivingArchiveErrorV2::HistoryGraph)?;
                    if left.poll(1)? == LivingArchiveReplayStatusV2::Ready {
                        self.stage = 1;
                    }
                }
                1 => {
                    let left = self
                        .left
                        .as_mut()
                        .ok_or(LivingArchiveErrorV2::HistoryGraph)?;
                    self.left_ready = Some(left.take_ready_result()?);
                    self.stage = 2;
                }
                2 => {
                    self.cleanup = Some(Box::new(
                        self.left
                            .take()
                            .expect("retained left")
                            .into_retirement(Policy::ReleaseHandle),
                    ));
                }
                3 => {
                    let right = self
                        .right
                        .as_mut()
                        .ok_or(LivingArchiveErrorV2::HistoryGraph)?;
                    if right.poll(1)? == LivingArchiveReplayStatusV2::Ready {
                        self.stage = 4;
                    }
                }
                4 => {
                    let right = self
                        .right
                        .as_mut()
                        .ok_or(LivingArchiveErrorV2::HistoryGraph)?;
                    self.right_ready = Some(right.take_ready_result()?);
                    self.stage = 5;
                }
                5 => {
                    self.cleanup = Some(Box::new(
                        self.right
                            .take()
                            .expect("retained right")
                            .into_retirement(Policy::ReleaseHandle),
                    ));
                }
                6 => {
                    let left = self
                        .left_ready
                        .as_ref()
                        .ok_or(LivingArchiveErrorV2::HistoryGraph)?;
                    let right = self
                        .right_ready
                        .as_ref()
                        .ok_or(LivingArchiveErrorV2::HistoryGraph)?;
                    let left_view = left.authority().bookmark()?;
                    let right_view = right.authority().bookmark()?;
                    if left_view.view.tick != right_view.view.tick {
                        return Err(LivingArchiveErrorV2::HistoryGraph);
                    }
                    self.comparison = Some(LivingComparisonV2 {
                        tick: left_view.view.tick,
                        left: left_view,
                        right: right_view,
                        left_state: left.authority().state().clone(),
                        right_state: right.authority().state().clone(),
                    });
                    self.stage = 7;
                }
                7 => {
                    self.cleanup = Some(Box::new(
                        self.left_ready
                            .take()
                            .expect("left output")
                            .into_retirement(Policy::ReleaseHandle),
                    ))
                }
                8 => {
                    self.cleanup = Some(Box::new(
                        self.right_ready
                            .take()
                            .expect("right output")
                            .into_retirement(Policy::ReleaseHandle),
                    ))
                }
                9 => return Ok(LivingArchiveReplayStatusV2::Ready),
                _ => unreachable!("comparison stage"),
            }
        }
        Ok(LivingArchiveReplayStatusV2::Pending)
    }
    /// Borrowed once-only extraction; incomplete/error/cancel retains all owners.
    pub fn take_ready_result(&mut self) -> Result<LivingComparisonV2, LivingArchiveErrorV2> {
        if self.cancelled {
            return Err(LivingArchiveErrorV2::Cancelled);
        }
        if let Some(error) = &self.failure {
            return Err(error.clone());
        }
        if self.delivered || self.stage != 9 || self.cleanup.is_some() || self.comparison.is_none()
        {
            return Err(LivingArchiveErrorV2::NotReady);
        }
        self.delivered = true;
        Ok(self.comparison.take().expect("checked comparison"))
    }
    /// Synchronous compatibility: drops unreturned residual owners on rejection.
    pub fn finish(mut self) -> Result<LivingComparisonV2, LivingArchiveErrorV2> {
        self.take_ready_result()
    }
}
/// Reconstruct two retained creator histories at the same completed boundary.
pub fn begin_compare_living_v2(
    authority: &LivingGalaxyAuthorityV2,
    left: LivingBranchIdV2,
    right: LivingBranchIdV2,
    tick: LivingTickV2,
) -> Result<LivingComparisonReplayV2, LivingArchiveErrorV2> {
    let (left, right) = authority.comparison_views_at(left, right, tick)?;
    Ok(LivingComparisonReplayV2 {
        left: Some(begin_restore_living_view_v2(authority, left)?),
        right: Some(begin_restore_living_view_v2(authority, right)?),
        left_ready: None,
        right_ready: None,
        comparison: None,
        cleanup: None,
        stage: 0,
        cancelled: false,
        delivered: false,
        failure: None,
    })
}

/// Convenience for callers already running outside their UI/event thread.
pub fn decode_living_archive_v2(
    bytes: &[u8],
    catalog: &ValidatedLivingCatalogPackV2,
) -> Result<ValidatedLivingArchiveV2, LivingArchiveErrorV2> {
    let mut replay = begin_decode_living_archive_v2(bytes, catalog)?;
    while replay.poll(LIVING_MAX_REPLAY_POLL_WORK_V2)? == LivingArchiveReplayStatusV2::Pending {}
    replay.finish()
}

fn revision_chain(
    head: Option<LivingRevisionIdV2>,
    stop: Option<LivingRevisionIdV2>,
    records: &BTreeMap<LivingRevisionIdV2, LivingRecordedRevisionV2>,
) -> Result<Vec<LivingRecordedRevisionV2>, LivingArchiveErrorV2> {
    let mut cursor = head;
    let mut chain = Vec::new();
    while cursor != stop {
        let id = cursor.ok_or(LivingArchiveErrorV2::HistoryGraph)?;
        let record = records.get(&id).ok_or(LivingArchiveErrorV2::HistoryGraph)?;
        chain.push(record.clone());
        cursor = record.revision.parent;
        if chain.len() > records.len() {
            return Err(LivingArchiveErrorV2::HistoryGraph);
        }
    }
    chain.reverse();
    Ok(chain)
}
fn schedule(
    view: LivingHistoryViewV2,
    records: &BTreeMap<LivingRevisionIdV2, LivingRecordedRevisionV2>,
    branches: &BTreeMap<LivingBranchIdV2, LivingHistoryBranchV2>,
) -> Result<Vec<ReplayAction>, LivingArchiveErrorV2> {
    let mut lineage = Vec::new();
    let mut cursor = Some(view.branch);
    while let Some(id) = cursor {
        let branch = branches
            .get(&id)
            .ok_or(LivingArchiveErrorV2::HistoryGraph)?;
        lineage.push(branch);
        cursor = branch.parent;
        if lineage.len() > branches.len() {
            return Err(LivingArchiveErrorV2::HistoryGraph);
        }
    }
    lineage.reverse();
    let selected = lineage.last().ok_or(LivingArchiveErrorV2::HistoryGraph)?;
    if view.tick.0 < selected.fork_tick.0 || view.tick.0 > selected.tip_tick.0 {
        return Err(LivingArchiveErrorV2::HistoryGraph);
    }
    // The selected cursor must belong to this branch's retained chain, including
    // its inherited fork cursor; a sibling's revision never silently rebases.
    let retained = revision_chain(selected.head, None, records)?;
    if view
        .revision
        .is_some_and(|id| !retained.iter().any(|record| record.revision.id == id))
    {
        return Err(LivingArchiveErrorV2::HistoryGraph);
    }
    let mut actions = Vec::new();
    for (index, branch) in lineage.iter().enumerate() {
        if index != 0 {
            actions.push(ReplayAction::Fork {
                branch: branch.id,
                ordinal: branch.ordinal,
            });
        }
        let next = lineage.get(index + 1);
        let head = next.map_or(view.revision, |child| child.fork_revision);
        let end_tick = next.map_or(view.tick, |child| child.fork_tick);
        let chain = revision_chain(head, branch.fork_revision, records)?;
        let mut position = 0;
        let mut last_tick = branch.fork_tick;
        let mut paused_tick = None;
        while position < chain.len() {
            let record = &chain[position];
            if record.revision.tick.0 < last_tick.0 || record.revision.tick.0 > end_tick.0 {
                return Err(LivingArchiveErrorV2::HistoryGraph);
            }
            last_tick = record.revision.tick;
            match record.mode {
                LivingCommandModeV2::Paused => {
                    paused_tick = Some(record.revision.tick);
                    actions.push(ReplayAction::Paused(record.clone()));
                    position += 1;
                }
                LivingCommandModeV2::Running => {
                    if paused_tick == Some(record.revision.tick)
                        || record.revision.tick.0 <= branch.fork_tick.0
                    {
                        return Err(LivingArchiveErrorV2::HistoryGraph);
                    }
                    let tick = record.revision.tick;
                    let mut group = Vec::new();
                    while position < chain.len()
                        && chain[position].mode == LivingCommandModeV2::Running
                        && chain[position].revision.tick == tick
                    {
                        group.push(chain[position].clone());
                        position += 1;
                    }
                    if group.len() > super::simulation::LIVING_MAX_PENDING_QUEUE_DEPTH_V2 {
                        return Err(LivingArchiveErrorV2::HistoryGraph);
                    }
                    actions.push(ReplayAction::Running {
                        tick,
                        records: group,
                    });
                }
            }
        }
        actions.push(ReplayAction::Advance(end_tick));
    }
    Ok(actions)
}
fn validate_graph(
    archive: &LivingArchiveV2,
    root: LivingBranchIdV2,
) -> Result<(), LivingArchiveErrorV2> {
    if archive.revisions.as_slice().len() > 10_000
        || archive.branches.as_slice().is_empty()
        || archive.branches.as_slice().len() > 64
        || archive.active_view.tick != archive.final_tick
    {
        return Err(LivingArchiveErrorV2::HistoryGraph);
    }
    let records: BTreeMap<_, _> = archive
        .revisions
        .as_slice()
        .iter()
        .map(|record| (record.revision.id, record.clone()))
        .collect();
    let branches: BTreeMap<_, _> = archive
        .branches
        .as_slice()
        .iter()
        .map(|branch| (branch.id, branch))
        .collect();
    let mut ordinals = BTreeSet::new();
    for record in archive.revisions.as_slice() {
        let revision = &record.revision;
        revision.command.validate()?;
        let bytes = revision.command.canonical_bytes()?;
        if revision.ordinal == 0
            || !ordinals.insert(revision.ordinal)
            || revision.id
                != revision_id_v2(
                    archive.catalog_hash,
                    archive.genesis_seed.0,
                    revision.parent,
                    revision.tick,
                    revision.ordinal,
                    &bytes,
                )
        {
            return Err(LivingArchiveErrorV2::HistoryGraph);
        }
        if record.mode == LivingCommandModeV2::Running && revision.tick.0 == 0 {
            return Err(LivingArchiveErrorV2::HistoryGraph);
        }
        if let Some(parent) = revision.parent {
            let parent = records
                .get(&parent)
                .ok_or(LivingArchiveErrorV2::HistoryGraph)?;
            if parent.revision.ordinal >= revision.ordinal
                || parent.revision.tick.0 > revision.tick.0
            {
                return Err(LivingArchiveErrorV2::HistoryGraph);
            }
        }
    }
    if ordinals.last().copied().unwrap_or(0) > archive.accepted_sequence_high_water {
        return Err(LivingArchiveErrorV2::HighWater);
    }
    let mut fork_ordinals = BTreeSet::new();
    let mut reachable = BTreeSet::new();
    let mut roots = 0;
    for branch in archive.branches.as_slice() {
        if branch.tip_tick.0 < branch.fork_tick.0 {
            return Err(LivingArchiveErrorV2::HistoryGraph);
        }
        match branch.parent {
            None => {
                roots += 1;
                if branch.id != root
                    || branch.fork_revision.is_some()
                    || branch.fork_tick.0 != 0
                    || branch.ordinal != 0
                {
                    return Err(LivingArchiveErrorV2::HistoryGraph);
                }
            }
            Some(parent) => {
                let parent = branches
                    .get(&parent)
                    .ok_or(LivingArchiveErrorV2::HistoryGraph)?;
                let expected = match branch.fork_revision {
                    Some(revision) => {
                        fork_branch_id_v2(parent.id, revision, branch.fork_tick, branch.ordinal)
                    }
                    None => genesis_fork_branch_id_v2(
                        parent.id,
                        archive.genesis_manifest.state.digest()?,
                        branch.fork_tick,
                        branch.ordinal,
                    ),
                };
                if branch.ordinal == 0
                    || !fork_ordinals.insert(branch.ordinal)
                    || branch.ordinal <= parent.ordinal
                    || branch.fork_tick.0 < parent.fork_tick.0
                    || branch.fork_tick.0 > parent.tip_tick.0
                    || branch.id != expected
                {
                    return Err(LivingArchiveErrorV2::HistoryGraph);
                }
                if let Some(fork_revision) = branch.fork_revision {
                    let chain = revision_chain(parent.head, None, &records)?;
                    if !chain.iter().any(|record| {
                        record.revision.id == fork_revision
                            && record.revision.tick.0 <= branch.fork_tick.0
                    }) {
                        return Err(LivingArchiveErrorV2::HistoryGraph);
                    }
                }
            }
        }
        for record in revision_chain(branch.head, None, &records)? {
            reachable.insert(record.revision.id);
        }
    }
    if roots != 1
        || reachable.len() != records.len()
        || !branches.contains_key(&archive.active_view.branch)
    {
        return Err(LivingArchiveErrorV2::HistoryGraph);
    }
    if fork_ordinals.last().copied().unwrap_or(0) > archive.branch_sequence_high_water {
        return Err(LivingArchiveErrorV2::HighWater);
    }
    Ok(())
}

#[cfg(test)]
mod archive_retirement_regressions {
    use super::*;
    fn replay() -> LivingArchiveReplayV2 {
        let authority = crate::living::builtin_living_sandbox_v2().unwrap();
        begin_decode_living_archive_v2(
            &encode_living_archive_v2(&authority).unwrap(),
            authority.catalog(),
        )
        .unwrap()
    }
    #[test]
    fn archive_retirement_cancel_retains_working_owner() {
        let mut replay = replay();
        replay.working = Some(replay.origin.get().clone());
        replay.completed.push(replay.origin.get().clone());
        let state = replay.working.as_ref().unwrap().state() as *const _;
        let storage = replay.completed.as_ptr();
        replay.cancel();
        assert!(
            replay.working.is_some(),
            "cancel must retain the exact owner"
        );
        assert_eq!(replay.working.as_ref().unwrap().state() as *const _, state);
        assert_eq!(replay.completed.len(), 1);
        assert_eq!(replay.completed.as_ptr(), storage);
    }
    #[test]
    fn archive_retirement_fault_retains_final_authority() {
        let mut replay = replay();
        replay.task = replay.tasks.len();
        replay.archive.final_tick = LivingTickV2(1);
        replay.completed.push(replay.origin.get().clone());
        assert_eq!(replay.poll(1), Err(LivingArchiveErrorV2::FinalDigest));
        assert!(
            replay.completed.len() == 1,
            "failed final checks must retain active owner"
        );
    }
    #[test]
    fn archive_retirement_missing_present_does_not_select_active_as_its_own_tip() {
        let mut replay = replay();
        replay.task = replay.tasks.len();
        replay.completed.push(replay.origin.get().clone());
        assert_eq!(replay.poll(1), Err(LivingArchiveErrorV2::HistoryGraph));
        assert_eq!(replay.completed.len(), 1);
    }
}
