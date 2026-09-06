use std::collections::{BTreeMap, BTreeSet, VecDeque};

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::{
    command::{
        CreatorBatchV1, CreatorReceiptV1, CreatorRejectionV1, WorkshopAuthority,
        apply_recorded_batch,
    },
    ids::{BranchId, RevisionId, StateDigest, WorkshopTick},
    model::{RevisionRecordV1, WorkshopStateV1},
    pack::ValidatedCatalogPackV1,
    simulation::{DeterministicFault, TickReceiptV1, step_state},
};

pub const MAX_BRANCHES: usize = 64;
pub const MAX_REVISIONS: usize = 10_000;

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct BranchRef {
    pub id: BranchId,
    pub name: String,
    pub head: Option<RevisionId>,
    pub last_tick: WorkshopTick,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ActiveView {
    pub selected_branch: BranchId,
    pub view_cursor: Option<RevisionId>,
    pub tick: WorkshopTick,
    pub digest: StateDigest,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct CheckpointPolicy {
    pub tick_interval: u64,
    pub revision_interval: usize,
    pub maximum_entries: usize,
}

impl Default for CheckpointPolicy {
    fn default() -> Self {
        Self {
            tick_interval: 600,
            revision_interval: 128,
            maximum_entries: 16,
        }
    }
}

#[derive(Clone, Debug)]
struct DerivedCheckpoint {
    branch: BranchId,
    cursor: Option<RevisionId>,
    tick: WorkshopTick,
    digest: StateDigest,
    revision_count: usize,
    canonical_state: Vec<u8>,
}

#[derive(Clone, Debug, Eq, PartialEq, thiserror::Error)]
pub enum HistoryError {
    #[error(transparent)]
    Creator(#[from] CreatorRejectionV1),
    #[error(transparent)]
    Simulation(#[from] DeterministicFault),
    #[error("revision capacity is exhausted")]
    RevisionCapacity,
    #[error("branch capacity is exhausted")]
    BranchCapacity,
    #[error("branch identifier collided")]
    BranchIdentityCollision,
    #[error("revision identifier collided with different canonical content")]
    RevisionIdentityCollision,
    #[error("history operation requires a paused session")]
    NotPaused,
    #[error("there is no earlier revision to undo to")]
    CannotUndo,
    #[error("there is no child revision to redo")]
    CannotRedo,
    #[error("redo is ambiguous and requires an explicit revision")]
    AmbiguousRedo { choices: Vec<RevisionId> },
    #[error("revision is not a valid redo child")]
    NotRedoChild,
    #[error("branch does not exist")]
    UnknownBranch,
    #[error("revision graph is invalid")]
    InvalidRevisionGraph,
    #[error("replayed revision content does not match its record")]
    ReplayMismatch,
    #[error("branch name is invalid")]
    InvalidBranchName,
}

#[derive(Clone, Debug)]
pub struct WorkshopHistory {
    pub(crate) catalog: ValidatedCatalogPackV1,
    pub(crate) genesis_seed: [u8; 32],
    pub(crate) revisions: BTreeMap<RevisionId, RevisionRecordV1>,
    pub(crate) branches: BTreeMap<BranchId, BranchRef>,
    pub(crate) active: ActiveView,
    active_state: WorkshopStateV1,
    paused: bool,
    checkpoint_policy: CheckpointPolicy,
    checkpoints: VecDeque<DerivedCheckpoint>,
}

pub type WorkshopHistoryV1 = WorkshopHistory;
pub type BranchRefV1 = BranchRef;
pub type ActiveViewV1 = ActiveView;

impl WorkshopHistory {
    pub fn new(catalog: ValidatedCatalogPackV1, genesis_seed: [u8; 32]) -> Self {
        let state = WorkshopStateV1::default();
        let digest = state.digest();
        let branch_id = root_branch_id(catalog.catalog_hash().0, genesis_seed);
        let branch = BranchRef {
            id: branch_id,
            name: "Main".to_owned(),
            head: None,
            last_tick: WorkshopTick(0),
        };
        Self {
            catalog,
            genesis_seed,
            revisions: BTreeMap::new(),
            branches: BTreeMap::from([(branch_id, branch)]),
            active: ActiveView {
                selected_branch: branch_id,
                view_cursor: None,
                tick: WorkshopTick(0),
                digest,
            },
            active_state: state,
            paused: true,
            checkpoint_policy: CheckpointPolicy::default(),
            checkpoints: VecDeque::new(),
        }
    }

    pub fn from_seed_u64(catalog: ValidatedCatalogPackV1, seed: u64) -> Self {
        let mut genesis_seed = [0_u8; 32];
        genesis_seed[..8].copy_from_slice(&seed.to_le_bytes());
        Self::new(catalog, genesis_seed)
    }

    pub fn catalog(&self) -> &ValidatedCatalogPackV1 {
        &self.catalog
    }

    pub const fn genesis_seed(&self) -> [u8; 32] {
        self.genesis_seed
    }

    pub fn state(&self) -> &WorkshopStateV1 {
        &self.active_state
    }

    pub fn active_state(&self) -> &WorkshopStateV1 {
        self.state()
    }

    pub const fn active_view(&self) -> &ActiveView {
        &self.active
    }

    pub const fn active_revision(&self) -> Option<RevisionId> {
        self.active.view_cursor
    }

    pub const fn active_state_digest(&self) -> StateDigest {
        self.active.digest
    }

    pub fn branches(&self) -> &BTreeMap<BranchId, BranchRef> {
        &self.branches
    }

    pub fn revisions(&self) -> &BTreeMap<RevisionId, RevisionRecordV1> {
        &self.revisions
    }

    pub const fn is_paused(&self) -> bool {
        self.paused
    }

    pub fn set_paused(&mut self, paused: bool) {
        self.paused = paused;
    }

    pub fn set_checkpoint_policy(&mut self, policy: CheckpointPolicy) {
        self.checkpoint_policy = policy;
        while self.checkpoints.len() > policy.maximum_entries {
            self.checkpoints.pop_front();
        }
    }

    pub fn checkpoint_count(&self) -> usize {
        self.checkpoints.len()
    }

    pub fn revision_count(&self) -> usize {
        self.revisions.len()
    }

    pub fn submit(&mut self, batch: CreatorBatchV1) -> Result<CreatorReceiptV1, HistoryError> {
        if self.revisions.len() >= MAX_REVISIONS {
            return Err(HistoryError::RevisionCapacity);
        }
        if batch.expected_cursor != self.active.view_cursor {
            return Err(CreatorRejectionV1::StaleCursor.into());
        }
        if batch.expected_tick != self.active.tick {
            return Err(CreatorRejectionV1::StaleTick.into());
        }

        let selected = self
            .branches
            .get(&self.active.selected_branch)
            .ok_or(HistoryError::UnknownBranch)?
            .clone();
        let forks = selected.head != self.active.view_cursor;
        if forks && self.branches.len() >= MAX_BRANCHES {
            return Err(HistoryError::BranchCapacity);
        }

        let next_ordinal = self.next_ordinal(self.active.view_cursor, self.active.tick)?;
        let mut authority = WorkshopAuthority::from_parts(
            self.catalog.clone(),
            self.genesis_seed,
            self.active.view_cursor,
            self.active_state.clone(),
            next_ordinal,
        );
        let receipt = authority.submit(batch.clone())?;
        if authority.next_ordinal() != receipt.ordinal + 1 {
            return Err(HistoryError::ReplayMismatch);
        }
        let record = RevisionRecordV1 {
            id: receipt.revision,
            parent: self.active.view_cursor,
            tick: receipt.tick,
            ordinal: receipt.ordinal,
            batch,
            state_digest: receipt.post_batch_digest,
        };
        let fork = if forks {
            let id = fork_branch_id(selected.id, self.active.view_cursor, receipt.revision);
            if self.branches.contains_key(&id) {
                return Err(HistoryError::BranchIdentityCollision);
            }
            Some(BranchRef {
                id,
                name: self.next_branch_name(),
                head: Some(receipt.revision),
                last_tick: self.active.tick,
            })
        } else {
            None
        };
        if let Some(existing) = self.revisions.get(&record.id) {
            if existing != &record {
                return Err(HistoryError::RevisionIdentityCollision);
            }
        } else {
            self.revisions.insert(record.id, record);
        }

        if let Some(branch) = fork {
            let id = branch.id;
            self.branches.insert(id, branch);
            self.active.selected_branch = id;
        } else {
            let branch = self
                .branches
                .get_mut(&self.active.selected_branch)
                .ok_or(HistoryError::UnknownBranch)?;
            branch.head = Some(receipt.revision);
            branch.last_tick = self.active.tick;
        }

        self.active.view_cursor = Some(receipt.revision);
        self.active_state = authority.state().clone();
        self.active.digest = receipt.post_batch_digest;
        self.maybe_checkpoint();
        Ok(receipt)
    }

    pub fn apply_batch(&mut self, batch: CreatorBatchV1) -> Result<CreatorReceiptV1, HistoryError> {
        self.submit(batch)
    }

    pub fn step(&mut self) -> Result<TickReceiptV1, HistoryError> {
        let receipt = step_state(&self.catalog, &mut self.active_state)?;
        self.active.tick = self.active_state.tick;
        self.active.digest = receipt.post_tick_digest;
        let branch = self
            .branches
            .get_mut(&self.active.selected_branch)
            .ok_or(HistoryError::UnknownBranch)?;
        if branch.head == self.active.view_cursor {
            branch.last_tick = self.active.tick;
        }
        self.maybe_checkpoint();
        Ok(receipt)
    }

    pub fn advance_ticks(&mut self, count: u64) -> Result<Vec<TickReceiptV1>, HistoryError> {
        let mut receipts = Vec::new();
        for _ in 0..count {
            receipts.push(self.step()?);
        }
        Ok(receipts)
    }

    pub fn undo(&mut self) -> Result<Option<RevisionId>, HistoryError> {
        if !self.paused {
            return Err(HistoryError::NotPaused);
        }
        let cursor = self.active.view_cursor.ok_or(HistoryError::CannotUndo)?;
        let parent = self
            .revisions
            .get(&cursor)
            .ok_or(HistoryError::InvalidRevisionGraph)?
            .parent;
        let state = self.materialize(parent, self.active.tick)?;
        self.active.view_cursor = parent;
        self.active.digest = state.digest();
        self.active_state = state;
        Ok(parent)
    }

    pub fn redo_choices(&self) -> Vec<RevisionId> {
        self.revisions
            .values()
            .filter_map(|record| {
                (record.parent == self.active.view_cursor
                    && record.tick <= self.active.tick
                    && self
                        .branches
                        .values()
                        .any(|branch| self.revision_is_ancestor(record.id, branch.head)))
                .then_some(record.id)
            })
            .collect()
    }

    pub fn redo(&mut self) -> Result<RevisionId, HistoryError> {
        if !self.paused {
            return Err(HistoryError::NotPaused);
        }
        let choices = self.redo_choices();
        match choices.as_slice() {
            [] => Err(HistoryError::CannotRedo),
            [choice] => self.redo_to(*choice),
            _ => Err(HistoryError::AmbiguousRedo { choices }),
        }
    }

    pub fn redo_to(&mut self, revision: RevisionId) -> Result<RevisionId, HistoryError> {
        if !self.paused {
            return Err(HistoryError::NotPaused);
        }
        let record = self
            .revisions
            .get(&revision)
            .ok_or(HistoryError::NotRedoChild)?;
        if record.parent != self.active.view_cursor || record.tick > self.active.tick {
            return Err(HistoryError::NotRedoChild);
        }
        let selected_head = self
            .branches
            .get(&self.active.selected_branch)
            .ok_or(HistoryError::UnknownBranch)?
            .head;
        let branch = if self.revision_is_ancestor(revision, selected_head) {
            self.active.selected_branch
        } else {
            self.branches
                .values()
                .find(|branch| self.revision_is_ancestor(revision, branch.head))
                .map(|branch| branch.id)
                .ok_or(HistoryError::NotRedoChild)?
        };
        let state = self.materialize(Some(revision), self.active.tick)?;
        self.active.selected_branch = branch;
        self.active.view_cursor = Some(revision);
        self.active.digest = state.digest();
        self.active_state = state;
        Ok(revision)
    }

    pub fn switch_branch(&mut self, branch: BranchId) -> Result<(), HistoryError> {
        if !self.paused {
            return Err(HistoryError::NotPaused);
        }
        let reference = self
            .branches
            .get(&branch)
            .ok_or(HistoryError::UnknownBranch)?
            .clone();
        let state = self.materialize(reference.head, reference.last_tick)?;
        self.active = ActiveView {
            selected_branch: branch,
            view_cursor: reference.head,
            tick: reference.last_tick,
            digest: state.digest(),
        };
        self.active_state = state;
        Ok(())
    }

    pub fn rename_branch(&mut self, branch: BranchId, name: &str) -> Result<(), HistoryError> {
        if name.is_empty()
            || name.len() > 64
            || !name.bytes().all(|byte| (0x20..=0x7e).contains(&byte))
            || name.starts_with(' ')
            || name.ends_with(' ')
            || name.contains("  ")
        {
            return Err(HistoryError::InvalidBranchName);
        }
        self.branches
            .get_mut(&branch)
            .ok_or(HistoryError::UnknownBranch)?
            .name = name.to_owned();
        Ok(())
    }

    pub(crate) fn materialize(
        &self,
        cursor: Option<RevisionId>,
        requested_tick: WorkshopTick,
    ) -> Result<WorkshopStateV1, HistoryError> {
        let mut chain = Vec::new();
        let mut current = cursor;
        let mut visited = BTreeSet::new();
        while let Some(id) = current {
            if !visited.insert(id) || chain.len() >= MAX_REVISIONS {
                return Err(HistoryError::InvalidRevisionGraph);
            }
            let record = self
                .revisions
                .get(&id)
                .ok_or(HistoryError::InvalidRevisionGraph)?;
            chain.push(record);
            current = record.parent;
        }
        chain.reverse();

        let nearest = self
            .checkpoints
            .iter()
            .filter_map(|checkpoint| {
                let state: WorkshopStateV1 =
                    serde_json::from_slice(&checkpoint.canonical_state).ok()?;
                if serde_json::to_vec(&state).ok()? != checkpoint.canonical_state
                    || checkpoint.tick > requested_tick
                    || state.tick != checkpoint.tick
                    || state.digest() != checkpoint.digest
                {
                    return None;
                }
                let depth = match checkpoint.cursor {
                    Some(id) => chain.iter().position(|record| record.id == id)? + 1,
                    None => 0,
                };
                if chain
                    .get(depth)
                    .is_some_and(|next| checkpoint.tick > next.tick)
                {
                    return None;
                }
                Some((depth, checkpoint.tick, checkpoint.cursor, state))
            })
            .max_by_key(|(depth, tick, _, _)| (*depth, *tick));

        let (mut state, mut parent, start_index) = if let Some((depth, _, cursor, state)) = nearest
        {
            (state, cursor, depth)
        } else {
            (WorkshopStateV1::default(), None, 0)
        };
        for record in chain.into_iter().skip(start_index) {
            if record.tick < state.tick || record.tick > requested_tick {
                return Err(HistoryError::InvalidRevisionGraph);
            }
            while state.tick < record.tick {
                step_state(&self.catalog, &mut state)?;
            }
            let (candidate, receipt) = apply_recorded_batch(
                &self.catalog,
                self.genesis_seed,
                parent,
                record.tick,
                record.ordinal,
                &record.batch,
                &state,
            )?;
            if receipt.revision != record.id || receipt.post_batch_digest != record.state_digest {
                return Err(HistoryError::ReplayMismatch);
            }
            state = candidate;
            parent = Some(record.id);
        }
        while state.tick < requested_tick {
            step_state(&self.catalog, &mut state)?;
        }
        Ok(state)
    }

    fn next_ordinal(
        &self,
        cursor: Option<RevisionId>,
        tick: WorkshopTick,
    ) -> Result<u64, HistoryError> {
        let Some(cursor) = cursor else {
            return Ok(0);
        };
        let record = self
            .revisions
            .get(&cursor)
            .ok_or(HistoryError::InvalidRevisionGraph)?;
        if record.tick == tick {
            record
                .ordinal
                .checked_add(1)
                .ok_or(CreatorRejectionV1::OrdinalExhausted.into())
        } else {
            Ok(0)
        }
    }

    fn revision_is_ancestor(&self, ancestor: RevisionId, mut cursor: Option<RevisionId>) -> bool {
        let mut traversed = 0_usize;
        while let Some(id) = cursor {
            if id == ancestor {
                return true;
            }
            traversed += 1;
            if traversed > MAX_REVISIONS {
                return false;
            }
            cursor = self.revisions.get(&id).and_then(|record| record.parent);
        }
        false
    }

    fn next_branch_name(&self) -> String {
        let names: BTreeSet<&str> = self
            .branches
            .values()
            .map(|branch| branch.name.as_str())
            .collect();
        let mut number = 1_u64;
        loop {
            let candidate = format!("Branch {number}");
            if !names.contains(candidate.as_str()) {
                return candidate;
            }
            number += 1;
        }
    }

    fn maybe_checkpoint(&mut self) {
        let policy = self.checkpoint_policy;
        if policy.maximum_entries == 0 {
            return;
        }
        let active_branch = self.active.selected_branch;
        let lineage_revision_count = self.lineage_revision_count(self.active.view_cursor);
        let last_revision_count = self
            .checkpoints
            .iter()
            .rev()
            .find(|checkpoint| checkpoint.branch == active_branch)
            .map_or(0, |checkpoint| checkpoint.revision_count);
        let revision_due = policy.revision_interval != 0
            && lineage_revision_count.saturating_sub(last_revision_count)
                >= policy.revision_interval;
        let last_tick = self
            .checkpoints
            .iter()
            .rev()
            .find(|checkpoint| checkpoint.branch == active_branch)
            .map_or(WorkshopTick(0), |checkpoint| checkpoint.tick);
        let ticks_since_checkpoint = self.active.tick.0.saturating_sub(last_tick.0);
        let tick_due = policy.tick_interval != 0 && ticks_since_checkpoint >= policy.tick_interval;
        if !revision_due && !tick_due {
            return;
        }
        if self.checkpoints.back().is_some_and(|checkpoint| {
            checkpoint.cursor == self.active.view_cursor && checkpoint.tick == self.active.tick
        }) {
            return;
        }
        self.checkpoints.push_back(DerivedCheckpoint {
            branch: active_branch,
            cursor: self.active.view_cursor,
            tick: self.active.tick,
            digest: self.active.digest,
            revision_count: lineage_revision_count,
            canonical_state: self.active_state.canonical_bytes(),
        });
        while self.checkpoints.len() > policy.maximum_entries {
            self.checkpoints.pop_front();
        }
    }

    fn lineage_revision_count(&self, mut cursor: Option<RevisionId>) -> usize {
        let mut count = 0;
        while let Some(id) = cursor {
            count += 1;
            if count >= MAX_REVISIONS {
                break;
            }
            cursor = self.revisions.get(&id).and_then(|record| record.parent);
        }
        count
    }

    pub(crate) fn from_validated_parts(
        catalog: ValidatedCatalogPackV1,
        genesis_seed: [u8; 32],
        revisions: BTreeMap<RevisionId, RevisionRecordV1>,
        branches: BTreeMap<BranchId, BranchRef>,
        active: ActiveView,
        active_state: WorkshopStateV1,
    ) -> Self {
        Self {
            catalog,
            genesis_seed,
            revisions,
            branches,
            active,
            active_state,
            paused: true,
            checkpoint_policy: CheckpointPolicy::default(),
            checkpoints: VecDeque::new(),
        }
    }
}

pub(crate) fn root_branch_id(catalog_hash: [u8; 32], genesis_seed: [u8; 32]) -> BranchId {
    let mut hash = Sha256::new();
    hash.update(b"NYON-WORKSHOP-ROOT-BRANCH-V1\0");
    hash.update(catalog_hash);
    hash.update(genesis_seed);
    truncate_branch_id(hash.finalize().into())
}

pub(crate) fn fork_branch_id(
    source: BranchId,
    cursor: Option<RevisionId>,
    first_revision: RevisionId,
) -> BranchId {
    let mut hash = Sha256::new();
    hash.update(b"NYON-WORKSHOP-FORK-BRANCH-V1\0");
    hash.update(source.0);
    match cursor {
        Some(cursor) => {
            hash.update([1]);
            hash.update(cursor.0);
        }
        None => hash.update([0]),
    }
    hash.update(first_revision.0);
    truncate_branch_id(hash.finalize().into())
}

fn truncate_branch_id(bytes: [u8; 32]) -> BranchId {
    let mut id = [0_u8; 16];
    id.copy_from_slice(&bytes[..16]);
    BranchId(id)
}
