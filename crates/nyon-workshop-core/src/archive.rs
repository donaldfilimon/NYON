use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::{
    WORKSHOP_RULES_VERSION,
    command::{apply_recorded_batch, revision_id},
    history::{
        ActiveView, BranchRef, HistoryError, MAX_BRANCHES, MAX_REVISIONS, WorkshopHistory,
        fork_branch_id, root_branch_id,
    },
    ids::{BranchId, CatalogHash, RevisionId, StateDigest, WorkshopTick},
    model::{RevisionRecordV1, WorkshopStateV1},
    pack::ValidatedCatalogPackV1,
    simulation::step_state,
};

pub const MAX_ARCHIVE_BYTES: usize = 16 * 1_024 * 1_024;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CanonicalArchive {
    pub bytes: Vec<u8>,
    pub sha256: [u8; 32],
}

#[derive(Clone, Debug, Eq, PartialEq, thiserror::Error)]
pub enum ArchiveError {
    #[error("Workshop archive has {actual} bytes; the limit is {limit}")]
    TooLarge { actual: usize, limit: usize },
    #[error("Workshop archive is not one canonical JSON document")]
    Json,
    #[error("Workshop archive format or rules version is unsupported")]
    Format,
    #[error("Workshop archive catalog hash does not match the validated catalog")]
    CatalogMismatch,
    #[error("Workshop archive is not in canonical byte form")]
    NonCanonical,
    #[error("Workshop archive integrity hash does not match its payload")]
    Integrity,
    #[error("Workshop archive revision sequence is invalid")]
    Revisions,
    #[error("Workshop archive branch sequence is invalid")]
    Branches,
    #[error("Workshop archive active view is invalid")]
    ActiveView,
    #[error("Workshop archive replayed state digest does not match")]
    StateDigest,
    #[error("Workshop archive replay did not finish within the caller's {budget}-unit budget")]
    ReplayBudgetExhausted { budget: u64 },
    #[error("Workshop archive replay is not complete")]
    ReplayIncomplete,
    #[error(transparent)]
    History(#[from] HistoryError),
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
struct ArchivePayloadV1 {
    kind: String,
    format_version: u32,
    rules_version: u32,
    catalog_hash: CatalogHash,
    genesis_seed: [u8; 32],
    revisions: Vec<RevisionRecordV1>,
    branches: Vec<BranchRef>,
    active_view: ActiveView,
    final_digest: StateDigest,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
struct ArchiveWireV1 {
    kind: String,
    format_version: u32,
    rules_version: u32,
    catalog_hash: CatalogHash,
    genesis_seed: [u8; 32],
    revisions: Vec<RevisionRecordV1>,
    branches: Vec<BranchRef>,
    active_view: ActiveView,
    final_digest: StateDigest,
    integrity_sha256: [u8; 32],
}

impl ArchiveWireV1 {
    fn payload(&self) -> ArchivePayloadV1 {
        ArchivePayloadV1 {
            kind: self.kind.clone(),
            format_version: self.format_version,
            rules_version: self.rules_version,
            catalog_hash: self.catalog_hash,
            genesis_seed: self.genesis_seed,
            revisions: self.revisions.clone(),
            branches: self.branches.clone(),
            active_view: self.active_view.clone(),
            final_digest: self.final_digest,
        }
    }
}

pub fn encode_archive(history: &WorkshopHistory) -> Result<CanonicalArchive, ArchiveError> {
    let payload = ArchivePayloadV1 {
        kind: "NYON_WORKSHOP_ARCHIVE".to_owned(),
        format_version: 1,
        rules_version: WORKSHOP_RULES_VERSION,
        catalog_hash: history.catalog.catalog_hash(),
        genesis_seed: history.genesis_seed,
        revisions: history.revisions.values().cloned().collect(),
        branches: history.branches.values().cloned().collect(),
        active_view: history.active.clone(),
        final_digest: history.active.digest,
    };
    let payload_bytes = serde_json::to_vec(&payload).map_err(|_| ArchiveError::Json)?;
    let integrity_sha256 = integrity_hash(&payload_bytes);
    let wire = ArchiveWireV1 {
        kind: payload.kind,
        format_version: payload.format_version,
        rules_version: payload.rules_version,
        catalog_hash: payload.catalog_hash,
        genesis_seed: payload.genesis_seed,
        revisions: payload.revisions,
        branches: payload.branches,
        active_view: payload.active_view,
        final_digest: payload.final_digest,
        integrity_sha256,
    };
    let bytes = serde_json::to_vec(&wire).map_err(|_| ArchiveError::Json)?;
    if bytes.len() > MAX_ARCHIVE_BYTES {
        return Err(ArchiveError::TooLarge {
            actual: bytes.len(),
            limit: MAX_ARCHIVE_BYTES,
        });
    }
    Ok(CanonicalArchive {
        sha256: Sha256::digest(&bytes).into(),
        bytes,
    })
}

pub fn decode_archive(
    catalog: &ValidatedCatalogPackV1,
    bytes: &[u8],
    work_budget: u64,
) -> Result<WorkshopHistory, ArchiveError> {
    let mut job = ArchiveDecodeJob::new(catalog, bytes)?;
    if job.poll(work_budget)? == ArchiveDecodeStatus::Pending {
        return Err(ArchiveError::ReplayBudgetExhausted {
            budget: work_budget,
        });
    }
    job.finish()
}

/// Returns the catalog hash authenticated by a canonical archive envelope.
///
/// This bounded inspection validates the exact wire shape, canonical JSON,
/// archive/rules versions, and payload integrity. It deliberately does **not**
/// validate the revision DAG, branch semantics, replay budget, or final state;
/// callers must fetch the exact catalog and run [`ArchiveDecodeJob`] to
/// completion before treating the save as authoritative.
pub fn archive_catalog_hash(bytes: &[u8]) -> Result<CatalogHash, ArchiveError> {
    Ok(inspect_archive_wire(bytes)?.catalog_hash)
}

fn inspect_archive_wire(bytes: &[u8]) -> Result<ArchiveWireV1, ArchiveError> {
    if bytes.len() > MAX_ARCHIVE_BYTES {
        return Err(ArchiveError::TooLarge {
            actual: bytes.len(),
            limit: MAX_ARCHIVE_BYTES,
        });
    }
    if bytes.starts_with(&[0xef, 0xbb, 0xbf]) || std::str::from_utf8(bytes).is_err() {
        return Err(ArchiveError::Json);
    }
    let wire: ArchiveWireV1 = serde_json::from_slice(bytes).map_err(|_| ArchiveError::Json)?;
    let canonical = serde_json::to_vec(&wire).map_err(|_| ArchiveError::Json)?;
    if canonical != bytes {
        return Err(ArchiveError::NonCanonical);
    }
    if wire.kind != "NYON_WORKSHOP_ARCHIVE"
        || wire.format_version != 1
        || wire.rules_version != WORKSHOP_RULES_VERSION
    {
        return Err(ArchiveError::Format);
    }
    let payload_bytes = serde_json::to_vec(&wire.payload()).map_err(|_| ArchiveError::Json)?;
    if integrity_hash(&payload_bytes) != wire.integrity_sha256 {
        return Err(ArchiveError::Integrity);
    }
    Ok(wire)
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ArchiveDecodeStatus {
    Pending,
    Complete,
}

#[derive(Clone, Copy, Debug)]
enum ReplayTargetKind {
    Branch { supplies_active: bool },
    Active,
}

#[derive(Clone, Copy, Debug)]
struct ReplayTarget {
    cursor: Option<RevisionId>,
    tick: WorkshopTick,
    kind: ReplayTargetKind,
}

#[derive(Debug)]
struct IncrementalMaterializer {
    chain: Vec<RevisionRecordV1>,
    next_revision: usize,
    requested_tick: WorkshopTick,
    parent: Option<RevisionId>,
    state: WorkshopStateV1,
}

impl IncrementalMaterializer {
    fn new(
        revisions: &BTreeMap<RevisionId, RevisionRecordV1>,
        cursor: Option<RevisionId>,
        requested_tick: WorkshopTick,
    ) -> Result<Self, ArchiveError> {
        let mut chain = Vec::new();
        let mut current = cursor;
        let mut visited = BTreeSet::new();
        while let Some(id) = current {
            if !visited.insert(id) || chain.len() >= MAX_REVISIONS {
                return Err(ArchiveError::Revisions);
            }
            let record = revisions.get(&id).ok_or(ArchiveError::Revisions)?;
            if record.tick > requested_tick {
                return Err(ArchiveError::Revisions);
            }
            chain.push(record.clone());
            current = record.parent;
        }
        chain.reverse();
        Ok(Self {
            chain,
            next_revision: 0,
            requested_tick,
            parent: None,
            state: WorkshopStateV1::default(),
        })
    }

    fn poll(
        &mut self,
        catalog: &ValidatedCatalogPackV1,
        genesis_seed: [u8; 32],
        budget: &mut u64,
    ) -> Result<ArchiveDecodeStatus, ArchiveError> {
        loop {
            if let Some(record) = self.chain.get(self.next_revision) {
                if self.state.tick < record.tick {
                    if *budget == 0 {
                        return Ok(ArchiveDecodeStatus::Pending);
                    }
                    step_state(catalog, &mut self.state).map_err(HistoryError::from)?;
                    *budget -= 1;
                    continue;
                }
                if *budget == 0 {
                    return Ok(ArchiveDecodeStatus::Pending);
                }
                let (candidate, receipt) = apply_recorded_batch(
                    catalog,
                    genesis_seed,
                    self.parent,
                    record.tick,
                    record.ordinal,
                    &record.batch,
                    &self.state,
                )
                .map_err(HistoryError::from)?;
                if receipt.revision != record.id || receipt.post_batch_digest != record.state_digest
                {
                    return Err(ArchiveError::StateDigest);
                }
                self.state = candidate;
                self.parent = Some(record.id);
                self.next_revision += 1;
                *budget -= 1;
                continue;
            }
            if self.state.tick < self.requested_tick {
                if *budget == 0 {
                    return Ok(ArchiveDecodeStatus::Pending);
                }
                step_state(catalog, &mut self.state).map_err(HistoryError::from)?;
                *budget -= 1;
                continue;
            }
            return Ok(ArchiveDecodeStatus::Complete);
        }
    }
}

/// Incremental authoritative archive validation. Construction performs only
/// bounded envelope/DAG checks; [`Self::poll`] performs at most the caller's
/// requested number of tick-or-revision replay units and is safely cancellable
/// by dropping the job.
#[derive(Debug)]
pub struct ArchiveDecodeJob {
    catalog: ValidatedCatalogPackV1,
    genesis_seed: [u8; 32],
    revisions: BTreeMap<RevisionId, RevisionRecordV1>,
    branches: BTreeMap<BranchId, BranchRef>,
    active_view: ActiveView,
    final_digest: StateDigest,
    targets: Vec<ReplayTarget>,
    next_target: usize,
    materializer: Option<IncrementalMaterializer>,
    active_state: Option<WorkshopStateV1>,
    complete: bool,
}

impl ArchiveDecodeJob {
    pub fn new(catalog: &ValidatedCatalogPackV1, bytes: &[u8]) -> Result<Self, ArchiveError> {
        let wire = inspect_archive_wire(bytes)?;
        if wire.catalog_hash != catalog.catalog_hash() {
            return Err(ArchiveError::CatalogMismatch);
        }
        if wire.revisions.len() > MAX_REVISIONS
            || !wire
                .revisions
                .windows(2)
                .all(|items| items[0].id < items[1].id)
        {
            return Err(ArchiveError::Revisions);
        }
        if wire.branches.is_empty()
            || wire.branches.len() > MAX_BRANCHES
            || !wire
                .branches
                .windows(2)
                .all(|items| items[0].id < items[1].id)
        {
            return Err(ArchiveError::Branches);
        }

        let revisions: BTreeMap<RevisionId, RevisionRecordV1> = wire
            .revisions
            .iter()
            .cloned()
            .map(|record| (record.id, record))
            .collect();
        if revisions.len() != wire.revisions.len() {
            return Err(ArchiveError::Revisions);
        }
        let branches: BTreeMap<BranchId, BranchRef> = wire
            .branches
            .iter()
            .cloned()
            .map(|branch| (branch.id, branch))
            .collect();
        if branches.len() != wire.branches.len() {
            return Err(ArchiveError::Branches);
        }
        validate_revision_graph(catalog, wire.genesis_seed, &revisions)?;
        validate_branches(
            catalog,
            wire.genesis_seed,
            &revisions,
            &branches,
            &wire.active_view,
        )?;

        let active_matches_selected_head = branches
            .get(&wire.active_view.selected_branch)
            .is_some_and(|selected| {
                selected.head == wire.active_view.view_cursor
                    && selected.last_tick == wire.active_view.tick
            });
        let mut targets: Vec<_> = branches
            .values()
            .map(|branch| ReplayTarget {
                cursor: branch.head,
                tick: branch.last_tick,
                kind: ReplayTargetKind::Branch {
                    supplies_active: active_matches_selected_head
                        && branch.id == wire.active_view.selected_branch,
                },
            })
            .collect();
        if !active_matches_selected_head {
            targets.push(ReplayTarget {
                cursor: wire.active_view.view_cursor,
                tick: wire.active_view.tick,
                kind: ReplayTargetKind::Active,
            });
        }
        Ok(Self {
            catalog: catalog.clone(),
            genesis_seed: wire.genesis_seed,
            revisions,
            branches,
            active_view: wire.active_view,
            final_digest: wire.final_digest,
            targets,
            next_target: 0,
            materializer: None,
            active_state: None,
            complete: false,
        })
    }

    pub fn poll(&mut self, work_budget: u64) -> Result<ArchiveDecodeStatus, ArchiveError> {
        if self.complete {
            return Ok(ArchiveDecodeStatus::Complete);
        }
        let mut remaining = work_budget;
        loop {
            let Some(target) = self.targets.get(self.next_target).copied() else {
                self.complete = true;
                return Ok(ArchiveDecodeStatus::Complete);
            };
            if self.materializer.is_none() {
                self.materializer = Some(IncrementalMaterializer::new(
                    &self.revisions,
                    target.cursor,
                    target.tick,
                )?);
            }
            let status = self.materializer.as_mut().unwrap().poll(
                &self.catalog,
                self.genesis_seed,
                &mut remaining,
            )?;
            if status == ArchiveDecodeStatus::Pending {
                return Ok(status);
            }
            let materializer = self.materializer.take().unwrap();
            if materializer.state.tick != target.tick {
                return Err(ArchiveError::StateDigest);
            }
            if matches!(
                target.kind,
                ReplayTargetKind::Active
                    | ReplayTargetKind::Branch {
                        supplies_active: true
                    }
            ) {
                let digest = materializer.state.digest();
                if digest != self.active_view.digest || digest != self.final_digest {
                    return Err(ArchiveError::StateDigest);
                }
                self.active_state = Some(materializer.state);
            }
            self.next_target += 1;
        }
    }

    pub fn finish(self) -> Result<WorkshopHistory, ArchiveError> {
        if !self.complete {
            return Err(ArchiveError::ReplayIncomplete);
        }
        let active_state = self.active_state.ok_or(ArchiveError::ReplayIncomplete)?;
        Ok(WorkshopHistory::from_validated_parts(
            self.catalog,
            self.genesis_seed,
            self.revisions,
            self.branches,
            self.active_view,
            active_state,
        ))
    }
}

pub fn encode_workshop_archive(
    history: &WorkshopHistory,
) -> Result<CanonicalArchive, ArchiveError> {
    encode_archive(history)
}

pub fn decode_workshop_archive(
    catalog: &ValidatedCatalogPackV1,
    bytes: &[u8],
    work_budget: u64,
) -> Result<WorkshopHistory, ArchiveError> {
    decode_archive(catalog, bytes, work_budget)
}

fn validate_revision_graph(
    catalog: &ValidatedCatalogPackV1,
    genesis_seed: [u8; 32],
    revisions: &BTreeMap<RevisionId, RevisionRecordV1>,
) -> Result<(), ArchiveError> {
    for record in revisions.values() {
        let parent_record = record.parent.and_then(|parent| revisions.get(&parent));
        let ordinal_valid = match parent_record {
            Some(parent) if parent.tick == record.tick => parent
                .ordinal
                .checked_add(1)
                .is_some_and(|next| next == record.ordinal),
            Some(_) | None => record.ordinal == 0,
        };
        if record.batch.expected_cursor != record.parent
            || record.batch.expected_tick != record.tick
            || parent_record.is_some_and(|parent| parent.tick > record.tick)
            || record
                .parent
                .is_some_and(|parent| !revisions.contains_key(&parent))
            || !ordinal_valid
        {
            return Err(ArchiveError::Revisions);
        }
        let batch_bytes = serde_json::to_vec(&record.batch).map_err(|_| ArchiveError::Json)?;
        if revision_id(
            catalog,
            genesis_seed,
            record.parent,
            record.tick,
            record.ordinal,
            &batch_bytes,
        ) != record.id
        {
            return Err(ArchiveError::Revisions);
        }
    }
    let mut colors = BTreeMap::<RevisionId, u8>::new();
    for start in revisions.keys().copied() {
        if colors.get(&start) == Some(&2) {
            continue;
        }
        let mut path = Vec::new();
        let mut current = Some(start);
        while let Some(id) = current {
            match colors.get(&id).copied() {
                Some(1) => return Err(ArchiveError::Revisions),
                Some(2) => break,
                _ => {
                    colors.insert(id, 1);
                    path.push(id);
                    current = revisions.get(&id).ok_or(ArchiveError::Revisions)?.parent;
                }
            }
        }
        for id in path {
            colors.insert(id, 2);
        }
    }
    Ok(())
}

fn validate_branches(
    catalog: &ValidatedCatalogPackV1,
    genesis_seed: [u8; 32],
    revisions: &BTreeMap<RevisionId, RevisionRecordV1>,
    branches: &BTreeMap<BranchId, BranchRef>,
    active: &ActiveView,
) -> Result<(), ArchiveError> {
    let mut reachable = BTreeSet::new();
    for branch in branches.values() {
        if !valid_branch_name(&branch.name)
            || branch
                .head
                .and_then(|head| revisions.get(&head))
                .is_some_and(|head| head.tick > branch.last_tick)
            || branch
                .head
                .is_some_and(|head| !revisions.contains_key(&head))
        {
            return Err(ArchiveError::Branches);
        }
        let mut current = branch.head;
        while let Some(id) = current {
            reachable.insert(id);
            current = revisions.get(&id).ok_or(ArchiveError::Branches)?.parent;
        }
    }
    if reachable.len() != revisions.len() {
        return Err(ArchiveError::Branches);
    }
    validate_branch_ids(catalog, genesis_seed, revisions, branches)?;
    let selected = branches
        .get(&active.selected_branch)
        .ok_or(ArchiveError::ActiveView)?;
    if active.tick < selected.last_tick
        || (active.view_cursor == selected.head && active.tick != selected.last_tick)
        || active.view_cursor.is_some_and(|cursor| {
            !is_ancestor(revisions, cursor, selected.head)
                || revisions
                    .get(&cursor)
                    .is_none_or(|record| record.tick > active.tick)
        })
    {
        return Err(ArchiveError::ActiveView);
    }
    Ok(())
}

fn validate_branch_ids(
    catalog: &ValidatedCatalogPackV1,
    genesis_seed: [u8; 32],
    revisions: &BTreeMap<RevisionId, RevisionRecordV1>,
    branches: &BTreeMap<BranchId, BranchRef>,
) -> Result<(), ArchiveError> {
    let root = root_branch_id(catalog.catalog_hash().0, genesis_seed);
    if !branches.contains_key(&root) {
        return Err(ArchiveError::Branches);
    }

    let ancestry: BTreeMap<BranchId, BTreeSet<RevisionId>> = branches
        .values()
        .map(|branch| {
            let mut ids = BTreeSet::new();
            let mut current = branch.head;
            while let Some(id) = current {
                ids.insert(id);
                current = revisions.get(&id).and_then(|record| record.parent);
            }
            (branch.id, ids)
        })
        .collect();

    let mut derived = BTreeSet::from([root]);
    while derived.len() < branches.len() {
        let mut progress = false;
        for target in branches.values() {
            if derived.contains(&target.id) {
                continue;
            }
            let target_ancestry = &ancestry[&target.id];
            let valid_origin = branches.values().any(|source| {
                if !derived.contains(&source.id) || source.id == target.id {
                    return false;
                }
                let source_ancestry = &ancestry[&source.id];
                target_ancestry.iter().any(|first| {
                    let record = &revisions[first];
                    !source_ancestry.contains(first)
                        && record
                            .parent
                            .is_none_or(|parent| source_ancestry.contains(&parent))
                        && fork_branch_id(source.id, record.parent, record.id) == target.id
                })
            });
            if valid_origin {
                derived.insert(target.id);
                progress = true;
            }
        }
        if !progress {
            return Err(ArchiveError::Branches);
        }
    }
    Ok(())
}

fn valid_branch_name(name: &str) -> bool {
    (1..=64).contains(&name.len())
        && name.bytes().all(|byte| (0x20..=0x7e).contains(&byte))
        && !name.starts_with(' ')
        && !name.ends_with(' ')
        && !name.contains("  ")
}

fn is_ancestor(
    revisions: &BTreeMap<RevisionId, RevisionRecordV1>,
    ancestor: RevisionId,
    mut cursor: Option<RevisionId>,
) -> bool {
    while let Some(id) = cursor {
        if id == ancestor {
            return true;
        }
        cursor = revisions.get(&id).and_then(|record| record.parent);
    }
    false
}

fn integrity_hash(payload_bytes: &[u8]) -> [u8; 32] {
    let mut hash = Sha256::new();
    hash.update(b"NYON-WORKSHOP-ARCHIVE-INTEGRITY-V1\0");
    hash.update(payload_bytes);
    hash.finalize().into()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{CreatorBatchV1, CreatorOpV1, GalaxyPointV1, ObjectName, WorkshopTick};

    const CORE_PACK: &[u8] = include_bytes!("../../../assets/workshop/core-pack-v1.json");

    #[test]
    fn authenticated_archive_rejects_active_tick_before_selected_branch_tick() {
        let catalog = crate::decode_catalog_pack(CORE_PACK).unwrap();
        let mut history = WorkshopHistory::from_seed_u64(catalog.clone(), 77);
        history.step().unwrap();
        let archive = encode_archive(&history).unwrap();
        let mut wire: ArchiveWireV1 = serde_json::from_slice(&archive.bytes).unwrap();
        wire.active_view.tick = crate::WorkshopTick(0);
        let payload = serde_json::to_vec(&wire.payload()).unwrap();
        wire.integrity_sha256 = integrity_hash(&payload);
        let bytes = serde_json::to_vec(&wire).unwrap();
        assert!(matches!(
            decode_archive(&catalog, &bytes, 1),
            Err(ArchiveError::ActiveView)
        ));
    }

    #[test]
    fn zero_budget_finishes_a_zero_work_archive_without_spinning() {
        let catalog = crate::decode_catalog_pack(CORE_PACK).unwrap();
        let history = WorkshopHistory::from_seed_u64(catalog.clone(), 76);
        let archive = encode_archive(&history).unwrap();
        let mut job = ArchiveDecodeJob::new(&catalog, &archive.bytes).unwrap();
        assert_eq!(job.poll(0).unwrap(), ArchiveDecodeStatus::Complete);
        assert_eq!(job.finish().unwrap().state(), history.state());
    }

    #[test]
    fn authenticated_archive_rejects_active_tick_ahead_of_selected_head_tick() {
        let catalog = crate::decode_catalog_pack(CORE_PACK).unwrap();
        let history = WorkshopHistory::from_seed_u64(catalog.clone(), 78);
        let archive = encode_archive(&history).unwrap();
        let mut wire: ArchiveWireV1 = serde_json::from_slice(&archive.bytes).unwrap();
        wire.active_view.tick = WorkshopTick(1);
        resign(&mut wire);
        assert!(matches!(
            decode_archive(&catalog, &serde_json::to_vec(&wire).unwrap(), 1),
            Err(ArchiveError::ActiveView)
        ));
    }

    #[test]
    fn huge_branch_replay_is_incremental_instead_of_blocking_construction() {
        let catalog = crate::decode_catalog_pack(CORE_PACK).unwrap();
        let mut history = WorkshopHistory::from_seed_u64(catalog.clone(), 79);
        submit_system(&mut history, "First", 1);
        history.undo().unwrap();
        submit_system(&mut history, "Fork", 2);
        let archive = encode_archive(&history).unwrap();
        let mut wire: ArchiveWireV1 = serde_json::from_slice(&archive.bytes).unwrap();
        let nonselected = wire
            .branches
            .iter_mut()
            .find(|branch| branch.id != wire.active_view.selected_branch)
            .unwrap();
        nonselected.last_tick = WorkshopTick(u64::MAX);
        resign(&mut wire);
        let bytes = serde_json::to_vec(&wire).unwrap();
        let mut job = ArchiveDecodeJob::new(&catalog, &bytes).unwrap();
        assert_eq!(job.poll(10).unwrap(), ArchiveDecodeStatus::Pending);
    }

    #[test]
    fn huge_active_view_replay_is_incremental_instead_of_blocking_construction() {
        let catalog = crate::decode_catalog_pack(CORE_PACK).unwrap();
        let mut history = WorkshopHistory::from_seed_u64(catalog.clone(), 80);
        submit_system(&mut history, "Head", 1);
        history.undo().unwrap();
        let archive = encode_archive(&history).unwrap();
        let mut wire: ArchiveWireV1 = serde_json::from_slice(&archive.bytes).unwrap();
        wire.active_view.tick = WorkshopTick(u64::MAX);
        resign(&mut wire);
        let bytes = serde_json::to_vec(&wire).unwrap();
        let mut job = ArchiveDecodeJob::new(&catalog, &bytes).unwrap();
        assert_eq!(job.poll(10).unwrap(), ArchiveDecodeStatus::Pending);
    }

    #[test]
    fn huge_revision_gap_replay_is_incremental_instead_of_blocking_construction() {
        let catalog = crate::decode_catalog_pack(CORE_PACK).unwrap();
        let mut history = WorkshopHistory::from_seed_u64(catalog.clone(), 81);
        submit_system(&mut history, "Gap", 1);
        let archive = encode_archive(&history).unwrap();
        let mut wire: ArchiveWireV1 = serde_json::from_slice(&archive.bytes).unwrap();
        let record = wire.revisions.first_mut().unwrap();
        record.tick = WorkshopTick(u64::MAX);
        record.batch.expected_tick = record.tick;
        let batch_bytes = serde_json::to_vec(&record.batch).unwrap();
        record.id = revision_id(
            &catalog,
            wire.genesis_seed,
            record.parent,
            record.tick,
            record.ordinal,
            &batch_bytes,
        );
        wire.branches[0].head = Some(record.id);
        wire.branches[0].last_tick = record.tick;
        wire.active_view.view_cursor = Some(record.id);
        wire.active_view.tick = record.tick;
        resign(&mut wire);
        let bytes = serde_json::to_vec(&wire).unwrap();
        let mut job = ArchiveDecodeJob::new(&catalog, &bytes).unwrap();
        assert_eq!(job.poll(10).unwrap(), ArchiveDecodeStatus::Pending);
    }

    #[test]
    fn authenticated_archive_rejects_branch_id_without_a_valid_origin() {
        let catalog = crate::decode_catalog_pack(CORE_PACK).unwrap();
        let history = WorkshopHistory::from_seed_u64(catalog.clone(), 82);
        let archive = encode_archive(&history).unwrap();
        let mut wire: ArchiveWireV1 = serde_json::from_slice(&archive.bytes).unwrap();
        wire.branches[0].id.0[0] ^= 0xff;
        wire.active_view.selected_branch = wire.branches[0].id;
        resign(&mut wire);
        assert!(matches!(
            decode_archive(&catalog, &serde_json::to_vec(&wire).unwrap(), 1),
            Err(ArchiveError::Branches)
        ));
    }

    fn submit_system(history: &mut WorkshopHistory, name: &str, x: i64) {
        history
            .submit(CreatorBatchV1 {
                expected_cursor: history.active_revision(),
                expected_tick: history.state().tick,
                operations: vec![CreatorOpV1::CreateSystem {
                    local: crate::BatchLocalId(1),
                    name: ObjectName::new(name).unwrap(),
                    position: GalaxyPointV1::new(x, 0).unwrap(),
                }],
            })
            .unwrap();
    }

    fn resign(wire: &mut ArchiveWireV1) {
        let payload = serde_json::to_vec(&wire.payload()).unwrap();
        wire.integrity_sha256 = integrity_hash(&payload);
    }
}
