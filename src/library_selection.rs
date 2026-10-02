//! Version-neutral Continue record. Galaxy payloads remain in authority stores.
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

pub const LIBRARY_DATABASE: &str = "nyon.library.v1";
const MAX_RECORD_BYTES: usize = 4096;
const KIND: &str = "NYON_LIBRARY_SELECTION";

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SelectedAuthority {
    LivingV2,
    WorkshopV1,
}

/// Exact reference; strings deliberately have no authority wrapper conversions.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SelectionTarget {
    pub authority: SelectedAuthority,
    pub slot: u64,
    pub branch: String,
    pub revision: Option<String>,
    pub tick: u64,
    pub digest: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LibrarySelection {
    kind: String,
    format_version: u32,
    pub generation: u64,
    pub authority: Option<SelectedAuthority>,
    pub slot: Option<u64>,
    pub branch: Option<String>,
    pub revision: Option<String>,
    pub tick: Option<u64>,
    pub digest: Option<String>,
    integrity_sha256: String,
}

#[derive(Clone, Debug, Eq, PartialEq, thiserror::Error)]
pub enum SelectionError {
    #[error("selection record is corrupt")]
    CorruptRecovery,
    #[error("selection generation changed")]
    StaleGeneration,
    #[error("selection generation exhausted")]
    Exhausted,
    #[error("selection job cancelled")]
    Cancelled,
    #[error("selection target is invalid")]
    InvalidTarget,
    #[error("selection storage: {0}")]
    Storage(String),
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}
fn valid_hex(value: &str, length: usize) -> bool {
    value.len() == length
        && value
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
}
impl LibrarySelection {
    pub fn new(generation: u64, target: Option<SelectionTarget>) -> Result<Self, SelectionError> {
        if generation == 0 {
            return Err(SelectionError::InvalidTarget);
        }
        let mut record = Self {
            kind: KIND.into(),
            format_version: 1,
            generation,
            authority: None,
            slot: None,
            branch: None,
            revision: None,
            tick: None,
            digest: None,
            integrity_sha256: String::new(),
        };
        if let Some(target) = target {
            if (target.authority == SelectedAuthority::LivingV2 && target.slot == 0)
                || !valid_hex(&target.branch, 32)
                || !valid_hex(&target.digest, 64)
                || target
                    .revision
                    .as_ref()
                    .is_some_and(|id| !valid_hex(id, 64))
            {
                return Err(SelectionError::InvalidTarget);
            }
            record.authority = Some(target.authority);
            record.slot = Some(target.slot);
            record.branch = Some(target.branch);
            record.revision = target.revision;
            record.tick = Some(target.tick);
            record.digest = Some(target.digest);
        }
        record.integrity_sha256 = record.integrity()?;
        Ok(record)
    }
    fn integrity(&self) -> Result<String, SelectionError> {
        let mut payload = self.clone();
        payload.integrity_sha256.clear();
        let bytes = serde_json::to_vec(&payload).map_err(|_| SelectionError::CorruptRecovery)?;
        let mut hash = Sha256::new();
        hash.update(b"NYON-LIBRARY-SELECTION-V1\0");
        hash.update(bytes);
        Ok(hex(&hash.finalize()))
    }
    pub fn encode(&self) -> Result<Vec<u8>, SelectionError> {
        serde_json::to_vec(self).map_err(|_| SelectionError::CorruptRecovery)
    }
    pub fn decode(bytes: &[u8]) -> Result<Self, SelectionError> {
        if bytes.len() > MAX_RECORD_BYTES {
            return Err(SelectionError::CorruptRecovery);
        }
        let record: Self =
            serde_json::from_slice(bytes).map_err(|_| SelectionError::CorruptRecovery)?;
        if record.kind != KIND
            || record.format_version != 1
            || record.generation == 0
            || record.encode()? != bytes
            || record.integrity()? != record.integrity_sha256
        {
            return Err(SelectionError::CorruptRecovery);
        }
        let target = record.target()?;
        if Self::new(record.generation, target)? != record {
            return Err(SelectionError::CorruptRecovery);
        }
        Ok(record)
    }
    pub fn target(&self) -> Result<Option<SelectionTarget>, SelectionError> {
        match (
            self.authority,
            self.slot,
            &self.branch,
            self.tick,
            &self.digest,
        ) {
            (Some(authority), Some(slot), Some(branch), Some(tick), Some(digest)) => {
                Ok(Some(SelectionTarget {
                    authority,
                    slot,
                    branch: branch.clone(),
                    revision: self.revision.clone(),
                    tick,
                    digest: digest.clone(),
                }))
            }
            (None, None, None, None, None) if self.revision.is_none() => Ok(None),
            _ => Err(SelectionError::CorruptRecovery),
        }
    }
}

/// Head-first recovery; a bad newest record only falls back to a lower generation.
pub fn recover_selection(
    a: Option<&[u8]>,
    b: Option<&[u8]>,
    head: Option<u64>,
) -> Result<Option<LibrarySelection>, SelectionError> {
    if a.is_none() && b.is_none() && head.is_none() {
        return Ok(None);
    }
    let mut valid: Vec<_> = [a, b]
        .into_iter()
        .flatten()
        .filter_map(|bytes| LibrarySelection::decode(bytes).ok())
        .collect();
    valid.sort_by_key(|record| record.generation);
    if let Some(head) = head {
        if let Some(record) = valid.iter().find(|record| record.generation == head) {
            return Ok(Some(record.clone()));
        }
        valid.retain(|record| record.generation < head);
    }
    valid.pop().map(Some).ok_or(SelectionError::CorruptRecovery)
}

#[cfg(not(target_arch = "wasm32"))]
pub mod native;

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub struct SelectionJobId(pub u64);
#[derive(Clone, Debug)]
pub enum SelectionRequest {
    Read,
    CompareAndSwap {
        expected: Option<u64>,
        target: Option<SelectionTarget>,
    },
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum SelectionPoll {
    Pending,
    Ready(Option<LibrarySelection>),
    Failed(SelectionError),
    Cancelled,
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum SelectionCancel {
    Cancelled,
    CancelPending,
    CommitAlreadyWon(LibrarySelection),
}
/// Cancellation of an in-flight native commit returns CancelPending until poll
/// establishes its durable outcome. It never falsely reports cancellation.
pub trait LibrarySelectionCoordinator {
    fn release(&mut self, owner_epoch: u64, id: SelectionJobId) -> bool;
    fn begin(
        &mut self,
        owner_epoch: u64,
        request: SelectionRequest,
    ) -> Result<SelectionJobId, SelectionError>;
    fn poll(&mut self, owner_epoch: u64, id: SelectionJobId, work_budget: usize) -> SelectionPoll;
    fn cancel(
        &mut self,
        owner_epoch: u64,
        id: SelectionJobId,
    ) -> Result<SelectionCancel, SelectionError>;
}

#[cfg(target_arch = "wasm32")]
pub mod web;

#[cfg(not(target_arch = "wasm32"))]
fn allocate_job_id() -> Result<u64, SelectionError> {
    use std::sync::atomic::{AtomicU64, Ordering};
    static NEXT: AtomicU64 = AtomicU64::new(0);
    NEXT.try_update(Ordering::Relaxed, Ordering::Relaxed, |value| {
        value.checked_add(1)
    })
    .map(|value| value + 1)
    .map_err(|_| SelectionError::Exhausted)
}
#[cfg(target_arch = "wasm32")]
fn allocate_job_id() -> Result<u64, SelectionError> {
    thread_local! {static NEXT:std::cell::Cell<u64>=const {std::cell::Cell::new(0)};}
    NEXT.with(|next| {
        let value = next.get().checked_add(1).ok_or(SelectionError::Exhausted)?;
        next.set(value);
        Ok(value)
    })
}
