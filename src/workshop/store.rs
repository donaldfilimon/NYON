//! Persistence boundary for local Galaxy Workshop archives and data packs.
//!
//! The interface is deliberately job-shaped and object-safe. Native disk work
//! runs away from the event-loop thread, while in-memory work completes
//! eagerly and browser work is driven by IndexedDB callbacks. Callers observe
//! every implementation through the same [`WorkshopStore::poll`] contract.
//!
//! This adapter bounds archive bytes, requires a JSON object, and verifies the
//! integrity of native generations. It intentionally does not declare an
//! archive authoritative. The session layer must decode and replay loaded
//! bytes into temporary state before replacing a currently valid Workshop.

use std::collections::BTreeMap;

pub use nyon_workshop_core::CatalogHash;
use nyon_workshop_core::{PackValidationError, decode_catalog_pack, encode_catalog_pack};

pub const MAX_WORKSHOP_SLOTS: usize = 16;
pub const MAX_WORKSHOP_ARCHIVE_BYTES: usize = 16 * 1024 * 1024;
pub const MAX_WORKSHOP_PACK_BYTES: usize = 1024 * 1024;
pub const MAX_WORKSHOP_PACKS: usize = 256;
pub const MAX_SLOT_NAME_BYTES: usize = 64;

pub mod web;

#[cfg(target_arch = "wasm32")]
pub use web::IndexedDbWorkshopStore;

#[derive(
    Clone, Copy, Debug, serde::Deserialize, serde::Serialize, Eq, Hash, Ord, PartialEq, PartialOrd,
)]
#[serde(transparent)]
pub struct StoreJobId(pub u64);

#[derive(
    Clone, Copy, Debug, serde::Deserialize, serde::Serialize, Eq, Hash, Ord, PartialEq, PartialOrd,
)]
#[serde(transparent)]
pub struct SlotId(pub u64);

#[derive(
    Clone, Copy, Debug, serde::Deserialize, serde::Serialize, Eq, Hash, Ord, PartialEq, PartialOrd,
)]
#[serde(transparent)]
pub struct SaveGeneration(pub u64);

#[derive(Clone, Debug, serde::Deserialize, serde::Serialize, Eq, Ord, PartialEq, PartialOrd)]
#[serde(transparent)]
pub struct SlotName(String);

impl SlotName {
    pub fn new(value: impl Into<String>) -> Result<Self, WorkshopStoreError> {
        let value = value.into();
        let bytes = value.as_bytes();
        if bytes.is_empty()
            || bytes.len() > MAX_SLOT_NAME_BYTES
            || bytes.iter().any(|byte| !(b' '..=b'~').contains(byte))
            || value.starts_with(' ')
            || value.ends_with(' ')
            || value.contains("  ")
        {
            return Err(WorkshopStoreError::InvalidSlotName);
        }
        Ok(Self(value))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl TryFrom<String> for SlotName {
    type Error = WorkshopStoreError;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        Self::new(value)
    }
}

impl TryFrom<&str> for SlotName {
    type Error = WorkshopStoreError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        Self::new(value)
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum StoreJobClass {
    Commit,
    LoadOrImport,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum StoreIoOperation {
    Initialize,
    ReadManifest,
    ReadGeneration,
    ReadPack,
    WriteGeneration,
    WritePack,
    WriteManifest,
    SyncDirectory,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum StoreIoKind {
    NotFound,
    PermissionDenied,
    AlreadyExists,
    InvalidData,
    WriteZero,
    UnexpectedEof,
    Other,
}

impl From<std::io::ErrorKind> for StoreIoKind {
    fn from(value: std::io::ErrorKind) -> Self {
        match value {
            std::io::ErrorKind::NotFound => Self::NotFound,
            std::io::ErrorKind::PermissionDenied => Self::PermissionDenied,
            std::io::ErrorKind::AlreadyExists => Self::AlreadyExists,
            std::io::ErrorKind::InvalidData => Self::InvalidData,
            std::io::ErrorKind::WriteZero => Self::WriteZero,
            std::io::ErrorKind::UnexpectedEof => Self::UnexpectedEof,
            _ => Self::Other,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PackValidationCode {
    TooLarge,
    Json,
    Version,
    TickRate,
    Identifier,
    Duplicate,
    UnknownReference,
    InvalidLimit,
    Capacity,
    InvalidLink,
    DuplicateCoordinates,
    ResourceAmount,
}

impl From<&PackValidationError> for PackValidationCode {
    fn from(value: &PackValidationError) -> Self {
        match value {
            PackValidationError::PackTooLarge { .. } => Self::TooLarge,
            PackValidationError::Bom
            | PackValidationError::InvalidUtf8
            | PackValidationError::DepthExceeded { .. }
            | PackValidationError::DuplicateKey
            | PackValidationError::Float
            | PackValidationError::Json => Self::Json,
            PackValidationError::Format => Self::Version,
            PackValidationError::InvalidField { .. } => Self::Identifier,
            PackValidationError::DefinitionLimit { .. } => Self::InvalidLimit,
            PackValidationError::DuplicateCatalogId => Self::Duplicate,
            PackValidationError::UnknownResource => Self::UnknownReference,
            PackValidationError::InvalidRecipe => Self::ResourceAmount,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum NativeStoreFaultPoint {
    BeforeJobExecute,
    BeforeGenerationPersist,
    AfterGenerationPersist,
    BeforePackPersist,
    BeforeManifestPersist,
}

#[derive(Clone, Debug, thiserror::Error, Eq, PartialEq)]
pub enum WorkshopStoreError {
    #[error("a {class:?} Workshop storage job is already in flight")]
    Busy { class: StoreJobClass },
    #[error("Workshop storage exhausted its job identifiers")]
    JobIdExhausted,
    #[error("Workshop storage exhausted its save generations")]
    GenerationExhausted,
    #[error("Workshop storage has no slot {slot:?}")]
    UnknownSlot { slot: SlotId },
    #[error("Workshop slot {slot:?} is archived")]
    ArchivedSlot { slot: SlotId },
    #[error("Workshop storage has reached its {max_slots}-slot limit")]
    SlotCapacity { max_slots: usize },
    #[error("Workshop slot name is invalid")]
    InvalidSlotName,
    #[error("Workshop archive exceeds the {max_bytes}-byte limit")]
    ArchiveTooLarge { max_bytes: usize },
    #[error("Workshop archive is not a JSON object")]
    InvalidArchive,
    #[error("Workshop pack exceeds the {max_bytes}-byte limit")]
    PackTooLarge { max_bytes: usize },
    #[error("Workshop storage has reached its {max_packs}-pack limit")]
    PackCapacity { max_packs: usize },
    #[error("Workshop pack failed validation with code {code:?}")]
    InvalidPack { code: PackValidationCode },
    #[error("Workshop store quota exceeds the configured {max_bytes}-byte limit")]
    QuotaExceeded { max_bytes: usize },
    #[error(
        "Workshop slot {slot:?} expected generation {expected:?}, but the current generation is {actual:?}"
    )]
    StaleGeneration {
        slot: SlotId,
        expected: SaveGeneration,
        actual: SaveGeneration,
    },
    #[error("Workshop slot {slot:?} has no valid saved generation")]
    NoValidGeneration { slot: SlotId },
    #[error("Workshop pack integrity verification failed")]
    CorruptPack,
    #[error("Workshop storage manifest is corrupt or unsupported")]
    CorruptManifest,
    #[error("IndexedDB is unavailable")]
    IndexedDbUnavailable,
    #[error("the IndexedDB schema is unsupported or corrupt")]
    IndexedDbSchemaMismatch,
    #[error("the IndexedDB transaction was aborted")]
    IndexedDbTransactionAborted,
    #[error("the browser denied IndexedDB storage quota")]
    IndexedDbQuotaDenied,
    #[error("IndexedDB evicted referenced Workshop data")]
    IndexedDbEvicted,
    #[error("Workshop storage fault was injected at {point:?}")]
    InjectedFault { point: NativeStoreFaultPoint },
    #[error("Workshop storage {operation:?} failed with {kind:?}")]
    Io {
        operation: StoreIoOperation,
        kind: StoreIoKind,
    },
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SlotSummary {
    pub id: SlotId,
    pub name: SlotName,
    pub generation: SaveGeneration,
    pub archived: bool,
    pub has_previous_generation: bool,
    pub selected_for_continue: bool,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SlotList {
    pub slots: Vec<SlotSummary>,
    pub selected_continue: Option<SlotId>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LoadedSlot {
    pub slot: SlotId,
    pub name: SlotName,
    /// The generation from which `archive` was read.
    pub generation: SaveGeneration,
    /// The generation callers must use for compare-and-swap when committing.
    pub head_generation: SaveGeneration,
    /// Structurally bounded bytes. The session must still perform authoritative
    /// archive decode and replay before replacing live state.
    pub archive: Box<[u8]>,
    pub recovered_from_previous: bool,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum WorkshopStoreResult {
    Slots(SlotList),
    SlotCreated {
        slot: SlotId,
        generation: SaveGeneration,
    },
    SlotLoaded(LoadedSlot),
    SlotCommitted {
        slot: SlotId,
        generation: SaveGeneration,
    },
    SlotRenamed {
        slot: SlotId,
    },
    SlotArchived {
        slot: SlotId,
    },
    ContinueSelected {
        slot: SlotId,
    },
    PackStored {
        hash: CatalogHash,
    },
    PackLoaded {
        hash: CatalogHash,
        canonical_pack: Box<[u8]>,
    },
    Packs(Vec<CatalogHash>),
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum StoreJobState {
    Pending,
    Complete(Result<WorkshopStoreResult, WorkshopStoreError>),
    Unknown,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum WorkshopStoreRequest {
    ListSlots,
    CreateSlot {
        name: SlotName,
        archive: Box<[u8]>,
    },
    LoadSlot {
        slot: SlotId,
    },
    /// Loads the retained predecessor only when the caller's observed head is
    /// still current. The returned bytes remain structurally validated; the
    /// session must replay them before treating them as authoritative.
    LoadPreviousGeneration {
        slot: SlotId,
        expected_head_generation: SaveGeneration,
    },
    CommitSlot {
        slot: SlotId,
        expected_generation: SaveGeneration,
        archive: Box<[u8]>,
    },
    /// Publishes replay-validated recovery bytes while retaining the exact
    /// generation that supplied those bytes, rather than selecting a
    /// predecessor using storage-shape checks.
    PromoteRecoveredSlot {
        slot: SlotId,
        expected_head_generation: SaveGeneration,
        recovered_generation: SaveGeneration,
        archive: Box<[u8]>,
    },
    RenameSlot {
        slot: SlotId,
        name: SlotName,
    },
    ArchiveSlot {
        slot: SlotId,
    },
    SelectContinue {
        slot: SlotId,
    },
    PutPack {
        canonical_pack: Box<[u8]>,
    },
    GetPack {
        hash: CatalogHash,
    },
    ListPacks,
}

impl WorkshopStoreRequest {
    fn class(&self) -> StoreJobClass {
        match self {
            Self::ListSlots
            | Self::LoadSlot { .. }
            | Self::LoadPreviousGeneration { .. }
            | Self::GetPack { .. }
            | Self::ListPacks => StoreJobClass::LoadOrImport,
            Self::CreateSlot { .. }
            | Self::CommitSlot { .. }
            | Self::PromoteRecoveredSlot { .. }
            | Self::RenameSlot { .. }
            | Self::ArchiveSlot { .. }
            | Self::SelectContinue { .. }
            | Self::PutPack { .. } => StoreJobClass::Commit,
        }
    }
}

pub trait WorkshopStore {
    fn start(&mut self, request: WorkshopStoreRequest) -> Result<StoreJobId, WorkshopStoreError>;

    /// Returns a terminal state at most once. Polling a consumed or unknown ID
    /// returns [`StoreJobState::Unknown`].
    fn poll(&mut self, job: StoreJobId) -> StoreJobState;
}

#[derive(Clone)]
struct JobRecord {
    class: StoreJobClass,
    result: Option<Result<WorkshopStoreResult, WorkshopStoreError>>,
}

#[derive(Clone, Default)]
struct JobTable {
    next_id: u64,
    active_commit: Option<StoreJobId>,
    active_load: Option<StoreJobId>,
    records: BTreeMap<StoreJobId, JobRecord>,
}

impl JobTable {
    fn reserve(&mut self, class: StoreJobClass) -> Result<StoreJobId, WorkshopStoreError> {
        let active = match class {
            StoreJobClass::Commit => self.active_commit,
            StoreJobClass::LoadOrImport => self.active_load,
        };
        if active.is_some() {
            return Err(WorkshopStoreError::Busy { class });
        }
        let next = self
            .next_id
            .checked_add(1)
            .ok_or(WorkshopStoreError::JobIdExhausted)?;
        self.next_id = next;
        let id = StoreJobId(next);
        match class {
            StoreJobClass::Commit => self.active_commit = Some(id),
            StoreJobClass::LoadOrImport => self.active_load = Some(id),
        }
        self.records.insert(
            id,
            JobRecord {
                class,
                result: None,
            },
        );
        Ok(id)
    }

    fn finish(&mut self, job: StoreJobId, result: Result<WorkshopStoreResult, WorkshopStoreError>) {
        self.records
            .get_mut(&job)
            .expect("reserved Workshop job remains present")
            .result = Some(result);
    }

    fn poll(&mut self, job: StoreJobId) -> StoreJobState {
        let Some(mut record) = self.records.remove(&job) else {
            return StoreJobState::Unknown;
        };
        let Some(result) = record.result.take() else {
            self.records.insert(job, record);
            return StoreJobState::Pending;
        };
        match record.class {
            StoreJobClass::Commit => self.active_commit = None,
            StoreJobClass::LoadOrImport => self.active_load = None,
        }
        StoreJobState::Complete(result)
    }
}

#[derive(Clone)]
struct StoredGeneration {
    generation: SaveGeneration,
    archive: Box<[u8]>,
}

#[derive(Clone)]
struct MemorySlot {
    name: SlotName,
    archived: bool,
    generations: Vec<StoredGeneration>,
}

#[derive(Clone)]
pub struct MemoryWorkshopStore {
    jobs: JobTable,
    slots: BTreeMap<SlotId, MemorySlot>,
    selected_continue: Option<SlotId>,
    packs: BTreeMap<CatalogHash, Box<[u8]>>,
    quota_bytes: usize,
}

impl Default for MemoryWorkshopStore {
    fn default() -> Self {
        Self::with_quota_bytes(usize::MAX)
    }
}

impl MemoryWorkshopStore {
    pub fn with_quota_bytes(quota_bytes: usize) -> Self {
        Self {
            jobs: JobTable::default(),
            slots: BTreeMap::new(),
            selected_continue: None,
            packs: BTreeMap::new(),
            quota_bytes,
        }
    }

    fn execute(
        &mut self,
        request: WorkshopStoreRequest,
    ) -> Result<WorkshopStoreResult, WorkshopStoreError> {
        match request {
            WorkshopStoreRequest::ListSlots => Ok(WorkshopStoreResult::Slots(self.slot_list())),
            WorkshopStoreRequest::CreateSlot { name, archive } => {
                validate_archive(&archive)?;
                if self.slots.len() >= MAX_WORKSHOP_SLOTS {
                    return Err(WorkshopStoreError::SlotCapacity {
                        max_slots: MAX_WORKSHOP_SLOTS,
                    });
                }
                let slot_limit = u64::try_from(MAX_WORKSHOP_SLOTS)
                    .expect("Workshop slot limit is representable as u64");
                let slot = (0..slot_limit)
                    .map(SlotId)
                    .find(|id| !self.slots.contains_key(id))
                    .expect("slot count below capacity has a free identifier");
                let generation = SaveGeneration(1);
                let mut candidate = self.slots.clone();
                candidate.insert(
                    slot,
                    MemorySlot {
                        name,
                        archived: false,
                        generations: vec![StoredGeneration {
                            generation,
                            archive,
                        }],
                    },
                );
                self.ensure_quota(&candidate, &self.packs)?;
                self.slots = candidate;
                Ok(WorkshopStoreResult::SlotCreated { slot, generation })
            }
            WorkshopStoreRequest::LoadSlot { slot } => {
                let record = self
                    .slots
                    .get(&slot)
                    .ok_or(WorkshopStoreError::UnknownSlot { slot })?;
                let current = record
                    .generations
                    .first()
                    .expect("stored slot has a generation");
                Ok(WorkshopStoreResult::SlotLoaded(LoadedSlot {
                    slot,
                    name: record.name.clone(),
                    generation: current.generation,
                    head_generation: current.generation,
                    archive: current.archive.clone(),
                    recovered_from_previous: false,
                }))
            }
            WorkshopStoreRequest::LoadPreviousGeneration {
                slot,
                expected_head_generation,
            } => {
                let record = self
                    .slots
                    .get(&slot)
                    .ok_or(WorkshopStoreError::UnknownSlot { slot })?;
                let actual = record
                    .generations
                    .first()
                    .expect("stored slot has a generation")
                    .generation;
                if actual != expected_head_generation {
                    return Err(WorkshopStoreError::StaleGeneration {
                        slot,
                        expected: expected_head_generation,
                        actual,
                    });
                }
                let previous = record
                    .generations
                    .get(1)
                    .ok_or(WorkshopStoreError::NoValidGeneration { slot })?;
                Ok(WorkshopStoreResult::SlotLoaded(LoadedSlot {
                    slot,
                    name: record.name.clone(),
                    generation: previous.generation,
                    head_generation: actual,
                    archive: previous.archive.clone(),
                    recovered_from_previous: true,
                }))
            }
            WorkshopStoreRequest::CommitSlot {
                slot,
                expected_generation,
                archive,
            } => {
                validate_archive(&archive)?;
                let record = self
                    .slots
                    .get(&slot)
                    .ok_or(WorkshopStoreError::UnknownSlot { slot })?;
                if record.archived {
                    return Err(WorkshopStoreError::ArchivedSlot { slot });
                }
                let actual = record
                    .generations
                    .first()
                    .expect("stored slot has a generation")
                    .generation;
                if expected_generation != actual {
                    return Err(WorkshopStoreError::StaleGeneration {
                        slot,
                        expected: expected_generation,
                        actual,
                    });
                }
                let generation = SaveGeneration(
                    actual
                        .0
                        .checked_add(1)
                        .ok_or(WorkshopStoreError::GenerationExhausted)?,
                );
                let mut candidate = self.slots.clone();
                let candidate_slot = candidate
                    .get_mut(&slot)
                    .expect("validated slot remains present");
                candidate_slot.generations.insert(
                    0,
                    StoredGeneration {
                        generation,
                        archive,
                    },
                );
                candidate_slot.generations.truncate(2);
                self.ensure_quota(&candidate, &self.packs)?;
                self.slots = candidate;
                Ok(WorkshopStoreResult::SlotCommitted { slot, generation })
            }
            WorkshopStoreRequest::PromoteRecoveredSlot {
                slot,
                expected_head_generation,
                recovered_generation,
                archive,
            } => {
                validate_archive(&archive)?;
                let record = self
                    .slots
                    .get(&slot)
                    .ok_or(WorkshopStoreError::UnknownSlot { slot })?;
                if record.archived {
                    return Err(WorkshopStoreError::ArchivedSlot { slot });
                }
                let actual = record
                    .generations
                    .first()
                    .expect("stored slot has a generation")
                    .generation;
                if actual != expected_head_generation {
                    return Err(WorkshopStoreError::StaleGeneration {
                        slot,
                        expected: expected_head_generation,
                        actual,
                    });
                }
                let predecessor = record
                    .generations
                    .iter()
                    .find(|generation| generation.generation == recovered_generation)
                    .cloned()
                    .ok_or(WorkshopStoreError::NoValidGeneration { slot })?;
                let generation = SaveGeneration(
                    actual
                        .0
                        .checked_add(1)
                        .ok_or(WorkshopStoreError::GenerationExhausted)?,
                );
                let mut candidate = self.slots.clone();
                let candidate_slot = candidate
                    .get_mut(&slot)
                    .expect("validated slot remains present");
                candidate_slot.generations = vec![
                    StoredGeneration {
                        generation,
                        archive,
                    },
                    predecessor,
                ];
                self.ensure_quota(&candidate, &self.packs)?;
                self.slots = candidate;
                Ok(WorkshopStoreResult::SlotCommitted { slot, generation })
            }
            WorkshopStoreRequest::RenameSlot { slot, name } => {
                self.slots
                    .get_mut(&slot)
                    .ok_or(WorkshopStoreError::UnknownSlot { slot })?
                    .name = name;
                Ok(WorkshopStoreResult::SlotRenamed { slot })
            }
            WorkshopStoreRequest::ArchiveSlot { slot } => {
                self.slots
                    .get_mut(&slot)
                    .ok_or(WorkshopStoreError::UnknownSlot { slot })?
                    .archived = true;
                if self.selected_continue == Some(slot) {
                    self.selected_continue = None;
                }
                Ok(WorkshopStoreResult::SlotArchived { slot })
            }
            WorkshopStoreRequest::SelectContinue { slot } => {
                let record = self
                    .slots
                    .get(&slot)
                    .ok_or(WorkshopStoreError::UnknownSlot { slot })?;
                if record.archived {
                    return Err(WorkshopStoreError::ArchivedSlot { slot });
                }
                self.selected_continue = Some(slot);
                Ok(WorkshopStoreResult::ContinueSelected { slot })
            }
            WorkshopStoreRequest::PutPack { canonical_pack } => {
                let (hash, canonical_pack) = validate_pack(&canonical_pack)?;
                if !self.packs.contains_key(&hash) && self.packs.len() >= MAX_WORKSHOP_PACKS {
                    return Err(WorkshopStoreError::PackCapacity {
                        max_packs: MAX_WORKSHOP_PACKS,
                    });
                }
                let mut candidate = self.packs.clone();
                candidate.insert(hash, canonical_pack);
                self.ensure_quota(&self.slots, &candidate)?;
                self.packs = candidate;
                Ok(WorkshopStoreResult::PackStored { hash })
            }
            WorkshopStoreRequest::GetPack { hash } => {
                let canonical_pack = self
                    .packs
                    .get(&hash)
                    .ok_or(WorkshopStoreError::CorruptPack)?
                    .clone();
                Ok(WorkshopStoreResult::PackLoaded {
                    hash,
                    canonical_pack,
                })
            }
            WorkshopStoreRequest::ListPacks => Ok(WorkshopStoreResult::Packs(
                self.packs.keys().copied().collect(),
            )),
        }
    }

    fn slot_list(&self) -> SlotList {
        SlotList {
            slots: self
                .slots
                .iter()
                .map(|(id, slot)| {
                    let current = slot
                        .generations
                        .first()
                        .expect("stored slot has a generation");
                    SlotSummary {
                        id: *id,
                        name: slot.name.clone(),
                        generation: current.generation,
                        archived: slot.archived,
                        has_previous_generation: slot.generations.len() > 1,
                        selected_for_continue: self.selected_continue == Some(*id),
                    }
                })
                .collect(),
            selected_continue: self.selected_continue,
        }
    }

    fn ensure_quota(
        &self,
        slots: &BTreeMap<SlotId, MemorySlot>,
        packs: &BTreeMap<CatalogHash, Box<[u8]>>,
    ) -> Result<(), WorkshopStoreError> {
        let used = slots
            .values()
            .flat_map(|slot| &slot.generations)
            .try_fold(0usize, |total, generation| {
                total.checked_add(generation.archive.len())
            })
            .and_then(|total| {
                packs
                    .values()
                    .try_fold(total, |total, pack| total.checked_add(pack.len()))
            });
        if used.is_none_or(|used| used > self.quota_bytes) {
            return Err(WorkshopStoreError::QuotaExceeded {
                max_bytes: self.quota_bytes,
            });
        }
        Ok(())
    }
}

impl WorkshopStore for MemoryWorkshopStore {
    fn start(&mut self, request: WorkshopStoreRequest) -> Result<StoreJobId, WorkshopStoreError> {
        let job = self.jobs.reserve(request.class())?;
        let result = self.execute(request);
        self.jobs.finish(job, result);
        Ok(job)
    }

    fn poll(&mut self, job: StoreJobId) -> StoreJobState {
        self.jobs.poll(job)
    }
}

fn validate_archive(bytes: &[u8]) -> Result<(), WorkshopStoreError> {
    if bytes.len() > MAX_WORKSHOP_ARCHIVE_BYTES {
        return Err(WorkshopStoreError::ArchiveTooLarge {
            max_bytes: MAX_WORKSHOP_ARCHIVE_BYTES,
        });
    }
    let value: serde_json::Value =
        serde_json::from_slice(bytes).map_err(|_| WorkshopStoreError::InvalidArchive)?;
    if !value.is_object() {
        return Err(WorkshopStoreError::InvalidArchive);
    }
    Ok(())
}

fn validate_pack(bytes: &[u8]) -> Result<(CatalogHash, Box<[u8]>), WorkshopStoreError> {
    if bytes.len() > MAX_WORKSHOP_PACK_BYTES {
        return Err(WorkshopStoreError::PackTooLarge {
            max_bytes: MAX_WORKSHOP_PACK_BYTES,
        });
    }
    let pack = decode_catalog_pack(bytes).map_err(|error| WorkshopStoreError::InvalidPack {
        code: PackValidationCode::from(&error),
    })?;
    let canonical = encode_catalog_pack(&pack)
        .expect("re-encoding a validated Workshop pack is infallible")
        .into_boxed_slice();
    Ok((pack.catalog_hash(), canonical))
}

#[cfg(not(target_arch = "wasm32"))]
mod native {
    use std::{
        collections::BTreeSet,
        fs::{File, OpenOptions},
        io::{Read, Write},
        path::{Path, PathBuf},
        sync::{Arc, Mutex, mpsc},
        thread,
    };

    use sha2::{Digest, Sha256};

    use super::*;

    const MANIFEST_FORMAT: &str = "nyon-workshop-store";
    const MANIFEST_VERSION: u32 = 1;
    const MAX_MANIFEST_BYTES: usize = 64 * 1024;

    #[derive(Clone, Copy, Debug, Eq, PartialEq)]
    pub enum NativeWorkshopPlatform {
        MacOs,
        Windows,
        Linux,
    }

    #[derive(Clone, Debug, Default)]
    pub struct WorkshopPathEnvironment {
        pub home: Option<PathBuf>,
        pub appdata: Option<PathBuf>,
        pub xdg_data_home: Option<PathBuf>,
    }

    pub fn native_workshop_root(
        platform: NativeWorkshopPlatform,
        environment: &WorkshopPathEnvironment,
    ) -> Result<PathBuf, WorkshopStoreError> {
        let unavailable = || WorkshopStoreError::Io {
            operation: StoreIoOperation::Initialize,
            kind: StoreIoKind::NotFound,
        };
        match platform {
            NativeWorkshopPlatform::MacOs => Ok(environment
                .home
                .as_ref()
                .ok_or_else(unavailable)?
                .join("Library/Application Support/NYON/workshop-v1")),
            NativeWorkshopPlatform::Windows => Ok(environment
                .appdata
                .as_ref()
                .ok_or_else(unavailable)?
                .join("NYON/workshop-v1")),
            NativeWorkshopPlatform::Linux => {
                let root = if let Some(xdg) = environment
                    .xdg_data_home
                    .as_ref()
                    .filter(|path| path.is_absolute())
                {
                    xdg.clone()
                } else {
                    environment
                        .home
                        .as_ref()
                        .filter(|path| path.is_absolute())
                        .ok_or_else(unavailable)?
                        .join(".local/share")
                };
                Ok(root.join("NYON/workshop-v1"))
            }
        }
    }

    pub trait NativeStoreFaultInjector: Send {
        fn check(&mut self, point: NativeStoreFaultPoint) -> Result<(), WorkshopStoreError>;
    }

    #[derive(Default)]
    struct NoNativeStoreFaults;

    impl NativeStoreFaultInjector for NoNativeStoreFaults {
        fn check(&mut self, _point: NativeStoreFaultPoint) -> Result<(), WorkshopStoreError> {
            Ok(())
        }
    }

    #[derive(Clone, Debug, serde::Deserialize, serde::Serialize)]
    #[serde(deny_unknown_fields)]
    struct GenerationDescriptor {
        generation: SaveGeneration,
        byte_len: u64,
        sha256: [u8; 32],
    }

    #[derive(Clone, Debug, serde::Deserialize, serde::Serialize)]
    #[serde(deny_unknown_fields)]
    struct NativeSlotRecord {
        id: SlotId,
        name: SlotName,
        archived: bool,
        generations: Vec<GenerationDescriptor>,
    }

    #[derive(Clone, Debug, serde::Deserialize, serde::Serialize)]
    #[serde(deny_unknown_fields)]
    struct NativeManifest {
        format: String,
        version: u32,
        selected_continue: Option<SlotId>,
        slots: Vec<NativeSlotRecord>,
        packs: Vec<CatalogHash>,
    }

    impl Default for NativeManifest {
        fn default() -> Self {
            Self {
                format: MANIFEST_FORMAT.to_owned(),
                version: MANIFEST_VERSION,
                selected_continue: None,
                slots: Vec::new(),
                packs: Vec::new(),
            }
        }
    }

    pub struct NativeWorkshopStore {
        root: PathBuf,
        jobs: JobTable,
        pending:
            BTreeMap<StoreJobId, mpsc::Receiver<Result<WorkshopStoreResult, WorkshopStoreError>>>,
        faults: Arc<Mutex<Box<dyn NativeStoreFaultInjector>>>,
    }

    struct NativeStoreWorker {
        root: PathBuf,
        manifest: NativeManifest,
        faults: Arc<Mutex<Box<dyn NativeStoreFaultInjector>>>,
    }

    impl NativeWorkshopStore {
        pub fn at_root(root: impl Into<PathBuf>) -> Result<Self, WorkshopStoreError> {
            Self::with_fault_injector(root, Box::<NoNativeStoreFaults>::default())
        }

        pub fn with_fault_injector(
            root: impl Into<PathBuf>,
            faults: Box<dyn NativeStoreFaultInjector>,
        ) -> Result<Self, WorkshopStoreError> {
            let root = root.into();
            std::fs::create_dir_all(root.join("slots"))
                .and_then(|_| std::fs::create_dir_all(root.join("packs")))
                .map_err(|error| io_error(StoreIoOperation::Initialize, error))?;
            read_manifest(&root)?;
            Ok(Self {
                root,
                jobs: JobTable::default(),
                pending: BTreeMap::new(),
                faults: Arc::new(Mutex::new(faults)),
            })
        }

        pub fn for_current_platform() -> Result<Self, WorkshopStoreError> {
            let environment = WorkshopPathEnvironment {
                home: std::env::var_os("HOME").map(PathBuf::from),
                appdata: std::env::var_os("APPDATA").map(PathBuf::from),
                xdg_data_home: std::env::var_os("XDG_DATA_HOME").map(PathBuf::from),
            };
            #[cfg(target_os = "macos")]
            let platform = NativeWorkshopPlatform::MacOs;
            #[cfg(target_os = "windows")]
            let platform = NativeWorkshopPlatform::Windows;
            #[cfg(all(not(target_os = "macos"), not(target_os = "windows")))]
            let platform = NativeWorkshopPlatform::Linux;
            Self::at_root(native_workshop_root(platform, &environment)?)
        }

        pub fn root(&self) -> &Path {
            &self.root
        }
    }

    impl NativeStoreWorker {
        fn execute(
            &mut self,
            request: WorkshopStoreRequest,
        ) -> Result<WorkshopStoreResult, WorkshopStoreError> {
            self.check_fault(NativeStoreFaultPoint::BeforeJobExecute)?;
            let _store_lock = acquire_store_lock(&self.root)?;
            self.manifest = read_manifest(&self.root)?;
            match request {
                WorkshopStoreRequest::ListSlots => Ok(WorkshopStoreResult::Slots(self.slot_list())),
                WorkshopStoreRequest::CreateSlot { name, archive } => {
                    validate_archive(&archive)?;
                    if self.manifest.slots.len() >= MAX_WORKSHOP_SLOTS {
                        return Err(WorkshopStoreError::SlotCapacity {
                            max_slots: MAX_WORKSHOP_SLOTS,
                        });
                    }
                    let occupied: BTreeSet<SlotId> =
                        self.manifest.slots.iter().map(|slot| slot.id).collect();
                    let slot_limit = u64::try_from(MAX_WORKSHOP_SLOTS)
                        .expect("Workshop slot limit is representable as u64");
                    let slot = (0..slot_limit)
                        .map(SlotId)
                        .find(|id| !occupied.contains(id))
                        .expect("slot count below capacity has a free identifier");
                    let descriptor = self.persist_generation(slot, SaveGeneration(1), &archive)?;
                    let mut manifest = self.manifest.clone();
                    manifest.slots.push(NativeSlotRecord {
                        id: slot,
                        name,
                        archived: false,
                        generations: vec![descriptor],
                    });
                    manifest.slots.sort_by_key(|slot| slot.id);
                    self.persist_manifest(manifest)?;
                    Ok(WorkshopStoreResult::SlotCreated {
                        slot,
                        generation: SaveGeneration(1),
                    })
                }
                WorkshopStoreRequest::LoadSlot { slot } => self.load_slot(slot),
                WorkshopStoreRequest::LoadPreviousGeneration {
                    slot,
                    expected_head_generation,
                } => self.load_previous_generation(slot, expected_head_generation),
                WorkshopStoreRequest::CommitSlot {
                    slot,
                    expected_generation,
                    archive,
                } => {
                    validate_archive(&archive)?;
                    let record = self
                        .manifest
                        .slots
                        .iter()
                        .find(|record| record.id == slot)
                        .ok_or(WorkshopStoreError::UnknownSlot { slot })?;
                    if record.archived {
                        return Err(WorkshopStoreError::ArchivedSlot { slot });
                    }
                    let actual = record
                        .generations
                        .first()
                        .expect("validated native slot has a generation")
                        .generation;
                    if actual != expected_generation {
                        return Err(WorkshopStoreError::StaleGeneration {
                            slot,
                            expected: expected_generation,
                            actual,
                        });
                    }
                    let generation = SaveGeneration(
                        actual
                            .0
                            .checked_add(1)
                            .ok_or(WorkshopStoreError::GenerationExhausted)?,
                    );
                    let previous = record
                        .generations
                        .iter()
                        .find(|descriptor| self.read_generation(slot, descriptor).is_ok())
                        .cloned();
                    let descriptor = self.persist_generation(slot, generation, &archive)?;
                    let mut manifest = self.manifest.clone();
                    let record = manifest
                        .slots
                        .iter_mut()
                        .find(|record| record.id == slot)
                        .expect("validated native slot remains present");
                    record.generations.clear();
                    record.generations.push(descriptor);
                    if let Some(previous) = previous {
                        record.generations.push(previous);
                    }
                    self.persist_manifest(manifest)?;
                    Ok(WorkshopStoreResult::SlotCommitted { slot, generation })
                }
                WorkshopStoreRequest::PromoteRecoveredSlot {
                    slot,
                    expected_head_generation,
                    recovered_generation,
                    archive,
                } => {
                    validate_archive(&archive)?;
                    let record = self
                        .manifest
                        .slots
                        .iter()
                        .find(|record| record.id == slot)
                        .ok_or(WorkshopStoreError::UnknownSlot { slot })?;
                    if record.archived {
                        return Err(WorkshopStoreError::ArchivedSlot { slot });
                    }
                    let actual = record
                        .generations
                        .first()
                        .expect("validated native slot has a generation")
                        .generation;
                    if actual != expected_head_generation {
                        return Err(WorkshopStoreError::StaleGeneration {
                            slot,
                            expected: expected_head_generation,
                            actual,
                        });
                    }
                    let predecessor = record
                        .generations
                        .iter()
                        .find(|descriptor| descriptor.generation == recovered_generation)
                        .cloned()
                        .ok_or(WorkshopStoreError::NoValidGeneration { slot })?;
                    self.read_generation(slot, &predecessor)
                        .map_err(|error| match error {
                            ReadGenerationError::Invalid => {
                                WorkshopStoreError::NoValidGeneration { slot }
                            }
                            ReadGenerationError::Store(error) => error,
                        })?;
                    let generation = SaveGeneration(
                        actual
                            .0
                            .checked_add(1)
                            .ok_or(WorkshopStoreError::GenerationExhausted)?,
                    );
                    let descriptor = self.persist_generation(slot, generation, &archive)?;
                    let mut manifest = self.manifest.clone();
                    let record = manifest
                        .slots
                        .iter_mut()
                        .find(|record| record.id == slot)
                        .expect("validated native slot remains present");
                    record.generations = vec![descriptor, predecessor];
                    self.persist_manifest(manifest)?;
                    Ok(WorkshopStoreResult::SlotCommitted { slot, generation })
                }
                WorkshopStoreRequest::RenameSlot { slot, name } => {
                    let mut manifest = self.manifest.clone();
                    manifest
                        .slots
                        .iter_mut()
                        .find(|record| record.id == slot)
                        .ok_or(WorkshopStoreError::UnknownSlot { slot })?
                        .name = name;
                    self.persist_manifest(manifest)?;
                    Ok(WorkshopStoreResult::SlotRenamed { slot })
                }
                WorkshopStoreRequest::ArchiveSlot { slot } => {
                    let mut manifest = self.manifest.clone();
                    manifest
                        .slots
                        .iter_mut()
                        .find(|record| record.id == slot)
                        .ok_or(WorkshopStoreError::UnknownSlot { slot })?
                        .archived = true;
                    if manifest.selected_continue == Some(slot) {
                        manifest.selected_continue = None;
                    }
                    self.persist_manifest(manifest)?;
                    Ok(WorkshopStoreResult::SlotArchived { slot })
                }
                WorkshopStoreRequest::SelectContinue { slot } => {
                    let record = self
                        .manifest
                        .slots
                        .iter()
                        .find(|record| record.id == slot)
                        .ok_or(WorkshopStoreError::UnknownSlot { slot })?;
                    if record.archived {
                        return Err(WorkshopStoreError::ArchivedSlot { slot });
                    }
                    let mut manifest = self.manifest.clone();
                    manifest.selected_continue = Some(slot);
                    self.persist_manifest(manifest)?;
                    Ok(WorkshopStoreResult::ContinueSelected { slot })
                }
                WorkshopStoreRequest::PutPack { canonical_pack } => {
                    let (hash, canonical_pack) = validate_pack(&canonical_pack)?;
                    if !self.manifest.packs.contains(&hash) {
                        if self.manifest.packs.len() >= MAX_WORKSHOP_PACKS {
                            return Err(WorkshopStoreError::PackCapacity {
                                max_packs: MAX_WORKSHOP_PACKS,
                            });
                        }
                        let mut manifest = self.manifest.clone();
                        manifest.packs.push(hash);
                        manifest.packs.sort();
                        manifest.packs.dedup();
                        validate_manifest(&manifest)?;
                        self.persist_pack(hash, &canonical_pack)?;
                        self.persist_manifest(manifest)?;
                    } else {
                        self.persist_pack(hash, &canonical_pack)?;
                    }
                    Ok(WorkshopStoreResult::PackStored { hash })
                }
                WorkshopStoreRequest::GetPack { hash } => {
                    if !self.manifest.packs.contains(&hash) {
                        return Err(WorkshopStoreError::CorruptPack);
                    }
                    let bytes = read_bounded(
                        &self.pack_path(hash),
                        MAX_WORKSHOP_PACK_BYTES,
                        StoreIoOperation::ReadPack,
                    )?;
                    let (actual, canonical_pack) = validate_pack(&bytes)?;
                    if actual != hash {
                        return Err(WorkshopStoreError::CorruptPack);
                    }
                    Ok(WorkshopStoreResult::PackLoaded {
                        hash,
                        canonical_pack,
                    })
                }
                WorkshopStoreRequest::ListPacks => {
                    Ok(WorkshopStoreResult::Packs(self.manifest.packs.clone()))
                }
            }
        }

        fn slot_list(&self) -> SlotList {
            SlotList {
                slots: self
                    .manifest
                    .slots
                    .iter()
                    .map(|slot| {
                        let current = slot
                            .generations
                            .first()
                            .expect("validated native slot has a generation");
                        SlotSummary {
                            id: slot.id,
                            name: slot.name.clone(),
                            generation: current.generation,
                            archived: slot.archived,
                            has_previous_generation: slot.generations.len() > 1,
                            selected_for_continue: self.manifest.selected_continue == Some(slot.id),
                        }
                    })
                    .collect(),
                selected_continue: self.manifest.selected_continue,
            }
        }

        fn load_slot(&self, slot: SlotId) -> Result<WorkshopStoreResult, WorkshopStoreError> {
            let record = self
                .manifest
                .slots
                .iter()
                .find(|record| record.id == slot)
                .ok_or(WorkshopStoreError::UnknownSlot { slot })?;
            let head_generation = record
                .generations
                .first()
                .expect("validated native slot has a generation")
                .generation;
            for (index, descriptor) in record.generations.iter().enumerate() {
                match self.read_generation(slot, descriptor) {
                    Ok(archive) => {
                        return Ok(WorkshopStoreResult::SlotLoaded(LoadedSlot {
                            slot,
                            name: record.name.clone(),
                            generation: descriptor.generation,
                            head_generation,
                            archive,
                            recovered_from_previous: index > 0,
                        }));
                    }
                    Err(ReadGenerationError::Invalid) => continue,
                    Err(ReadGenerationError::Store(error)) => return Err(error),
                }
            }
            Err(WorkshopStoreError::NoValidGeneration { slot })
        }

        fn load_previous_generation(
            &self,
            slot: SlotId,
            expected_head_generation: SaveGeneration,
        ) -> Result<WorkshopStoreResult, WorkshopStoreError> {
            let record = self
                .manifest
                .slots
                .iter()
                .find(|record| record.id == slot)
                .ok_or(WorkshopStoreError::UnknownSlot { slot })?;
            let actual = record
                .generations
                .first()
                .expect("validated native slot has a generation")
                .generation;
            if actual != expected_head_generation {
                return Err(WorkshopStoreError::StaleGeneration {
                    slot,
                    expected: expected_head_generation,
                    actual,
                });
            }
            let descriptor = record
                .generations
                .get(1)
                .ok_or(WorkshopStoreError::NoValidGeneration { slot })?;
            let archive = self
                .read_generation(slot, descriptor)
                .map_err(|error| match error {
                    ReadGenerationError::Invalid => WorkshopStoreError::NoValidGeneration { slot },
                    ReadGenerationError::Store(error) => error,
                })?;
            Ok(WorkshopStoreResult::SlotLoaded(LoadedSlot {
                slot,
                name: record.name.clone(),
                generation: descriptor.generation,
                head_generation: actual,
                archive,
                recovered_from_previous: true,
            }))
        }

        fn persist_generation(
            &mut self,
            slot: SlotId,
            generation: SaveGeneration,
            archive: &[u8],
        ) -> Result<GenerationDescriptor, WorkshopStoreError> {
            let directory = self.slot_directory(slot);
            std::fs::create_dir_all(&directory)
                .map_err(|error| io_error(StoreIoOperation::WriteGeneration, error))?;
            let mut temporary = tempfile::NamedTempFile::new_in(&directory)
                .map_err(|error| io_error(StoreIoOperation::WriteGeneration, error))?;
            temporary
                .write_all(archive)
                .and_then(|_| temporary.flush())
                .and_then(|_| temporary.as_file().sync_all())
                .map_err(|error| io_error(StoreIoOperation::WriteGeneration, error))?;
            self.check_fault(NativeStoreFaultPoint::BeforeGenerationPersist)?;
            temporary
                .persist(self.generation_path(slot, generation))
                .map_err(|error| io_error(StoreIoOperation::WriteGeneration, error.error))?;
            sync_directory(&directory)?;
            self.check_fault(NativeStoreFaultPoint::AfterGenerationPersist)?;
            Ok(GenerationDescriptor {
                generation,
                byte_len: archive.len() as u64,
                sha256: Sha256::digest(archive).into(),
            })
        }

        fn read_generation(
            &self,
            slot: SlotId,
            descriptor: &GenerationDescriptor,
        ) -> Result<Box<[u8]>, ReadGenerationError> {
            let path = self.generation_path(slot, descriptor.generation);
            let bytes = match read_bounded(
                &path,
                MAX_WORKSHOP_ARCHIVE_BYTES,
                StoreIoOperation::ReadGeneration,
            ) {
                Ok(bytes) => bytes,
                Err(WorkshopStoreError::Io {
                    kind: StoreIoKind::NotFound | StoreIoKind::InvalidData,
                    ..
                })
                | Err(WorkshopStoreError::ArchiveTooLarge { .. }) => {
                    return Err(ReadGenerationError::Invalid);
                }
                Err(error) => return Err(ReadGenerationError::Store(error)),
            };
            if bytes.len() as u64 != descriptor.byte_len
                || <[u8; 32]>::from(Sha256::digest(&bytes)) != descriptor.sha256
                || validate_archive(&bytes).is_err()
            {
                return Err(ReadGenerationError::Invalid);
            }
            Ok(bytes.into_boxed_slice())
        }

        fn persist_pack(
            &mut self,
            hash: CatalogHash,
            pack: &[u8],
        ) -> Result<(), WorkshopStoreError> {
            let directory = self.root.join("packs");
            let mut temporary = tempfile::NamedTempFile::new_in(&directory)
                .map_err(|error| io_error(StoreIoOperation::WritePack, error))?;
            temporary
                .write_all(pack)
                .and_then(|_| temporary.flush())
                .and_then(|_| temporary.as_file().sync_all())
                .map_err(|error| io_error(StoreIoOperation::WritePack, error))?;
            self.check_fault(NativeStoreFaultPoint::BeforePackPersist)?;
            temporary
                .persist(self.pack_path(hash))
                .map_err(|error| io_error(StoreIoOperation::WritePack, error.error))?;
            sync_directory(&directory)
        }

        fn persist_manifest(&mut self, manifest: NativeManifest) -> Result<(), WorkshopStoreError> {
            validate_manifest(&manifest)?;
            let bytes = serde_json::to_vec(&manifest)
                .expect("serializing a validated Workshop manifest is infallible");
            let mut temporary = tempfile::NamedTempFile::new_in(&self.root)
                .map_err(|error| io_error(StoreIoOperation::WriteManifest, error))?;
            temporary
                .write_all(&bytes)
                .and_then(|_| temporary.flush())
                .and_then(|_| temporary.as_file().sync_all())
                .map_err(|error| io_error(StoreIoOperation::WriteManifest, error))?;
            self.check_fault(NativeStoreFaultPoint::BeforeManifestPersist)?;
            temporary
                .persist(self.root.join("manifest-v1.json"))
                .map_err(|error| io_error(StoreIoOperation::WriteManifest, error.error))?;
            sync_directory(&self.root)?;
            self.manifest = manifest;
            Ok(())
        }

        fn slot_directory(&self, slot: SlotId) -> PathBuf {
            self.root.join("slots").join(format!("slot-{:02}", slot.0))
        }

        fn generation_path(&self, slot: SlotId, generation: SaveGeneration) -> PathBuf {
            self.slot_directory(slot)
                .join(format!("generation-{:020}.archive", generation.0))
        }

        fn pack_path(&self, hash: CatalogHash) -> PathBuf {
            self.root
                .join("packs")
                .join(format!("{}.nyonpack.json", hex(&hash.0)))
        }

        fn check_fault(&self, point: NativeStoreFaultPoint) -> Result<(), WorkshopStoreError> {
            self.faults
                .lock()
                .map_err(|_| WorkshopStoreError::Io {
                    operation: StoreIoOperation::Initialize,
                    kind: StoreIoKind::Other,
                })?
                .check(point)
        }
    }

    impl WorkshopStore for NativeWorkshopStore {
        fn start(
            &mut self,
            request: WorkshopStoreRequest,
        ) -> Result<StoreJobId, WorkshopStoreError> {
            let job = self.jobs.reserve(request.class())?;
            let root = self.root.clone();
            let faults = Arc::clone(&self.faults);
            let (sender, receiver) = mpsc::sync_channel(1);
            let spawn = thread::Builder::new()
                .name(format!("nyon-workshop-store-{}", job.0))
                .spawn(move || {
                    let mut worker = NativeStoreWorker {
                        root,
                        manifest: NativeManifest::default(),
                        faults,
                    };
                    let result = worker.execute(request);
                    let _ = sender.send(result);
                });
            match spawn {
                Ok(_worker) => {
                    self.pending.insert(job, receiver);
                }
                Err(error) => {
                    self.jobs
                        .finish(job, Err(io_error(StoreIoOperation::Initialize, error)));
                }
            }
            Ok(job)
        }

        fn poll(&mut self, job: StoreJobId) -> StoreJobState {
            let received = self.pending.get(&job).map(mpsc::Receiver::try_recv);
            match received {
                Some(Ok(result)) => {
                    self.pending.remove(&job);
                    self.jobs.finish(job, result);
                }
                Some(Err(mpsc::TryRecvError::Empty)) => return StoreJobState::Pending,
                Some(Err(mpsc::TryRecvError::Disconnected)) => {
                    self.pending.remove(&job);
                    self.jobs.finish(
                        job,
                        Err(WorkshopStoreError::Io {
                            operation: StoreIoOperation::Initialize,
                            kind: StoreIoKind::Other,
                        }),
                    );
                }
                None => {}
            }
            self.jobs.poll(job)
        }
    }

    enum ReadGenerationError {
        Invalid,
        Store(WorkshopStoreError),
    }

    fn acquire_store_lock(root: &Path) -> Result<File, WorkshopStoreError> {
        let lock = OpenOptions::new()
            .create(true)
            .read(true)
            .write(true)
            .truncate(false)
            .open(root.join("store-v1.lock"))
            .map_err(|error| io_error(StoreIoOperation::Initialize, error))?;
        lock.lock()
            .map_err(|error| io_error(StoreIoOperation::Initialize, error))?;
        Ok(lock)
    }

    fn read_manifest(root: &Path) -> Result<NativeManifest, WorkshopStoreError> {
        let path = root.join("manifest-v1.json");
        let bytes = match read_bounded(&path, MAX_MANIFEST_BYTES, StoreIoOperation::ReadManifest) {
            Ok(bytes) => bytes,
            Err(WorkshopStoreError::Io {
                kind: StoreIoKind::NotFound,
                ..
            }) => return Ok(NativeManifest::default()),
            Err(error) => return Err(error),
        };
        let manifest: NativeManifest =
            serde_json::from_slice(&bytes).map_err(|_| WorkshopStoreError::CorruptManifest)?;
        validate_manifest(&manifest)?;
        Ok(manifest)
    }

    fn validate_manifest(manifest: &NativeManifest) -> Result<(), WorkshopStoreError> {
        if manifest.format != MANIFEST_FORMAT
            || manifest.version != MANIFEST_VERSION
            || manifest.slots.len() > MAX_WORKSHOP_SLOTS
            || manifest.packs.len() > MAX_WORKSHOP_PACKS
            || !manifest
                .slots
                .windows(2)
                .all(|pair| pair[0].id < pair[1].id)
            || !manifest.packs.windows(2).all(|pair| pair[0] < pair[1])
        {
            return Err(WorkshopStoreError::CorruptManifest);
        }
        let manifest_bytes =
            serde_json::to_vec(manifest).map_err(|_| WorkshopStoreError::CorruptManifest)?;
        if manifest_bytes.len() > MAX_MANIFEST_BYTES {
            return Err(WorkshopStoreError::CorruptManifest);
        }
        for slot in &manifest.slots {
            let Ok(slot_index) = usize::try_from(slot.id.0) else {
                return Err(WorkshopStoreError::CorruptManifest);
            };
            if slot_index >= MAX_WORKSHOP_SLOTS
                || SlotName::new(slot.name.as_str()).ok().as_ref() != Some(&slot.name)
                || slot.generations.is_empty()
                || slot.generations.len() > 2
                || !slot
                    .generations
                    .windows(2)
                    .all(|pair| pair[0].generation > pair[1].generation)
                || slot.generations.iter().any(|generation| {
                    generation.generation.0 == 0
                        || generation.byte_len > MAX_WORKSHOP_ARCHIVE_BYTES as u64
                })
            {
                return Err(WorkshopStoreError::CorruptManifest);
            }
        }
        if let Some(selected) = manifest.selected_continue {
            let Some(slot) = manifest.slots.iter().find(|slot| slot.id == selected) else {
                return Err(WorkshopStoreError::CorruptManifest);
            };
            if slot.archived {
                return Err(WorkshopStoreError::CorruptManifest);
            }
        }
        Ok(())
    }

    fn read_bounded(
        path: &Path,
        maximum: usize,
        operation: StoreIoOperation,
    ) -> Result<Vec<u8>, WorkshopStoreError> {
        let file = File::open(path).map_err(|error| io_error(operation, error))?;
        let mut bytes = Vec::with_capacity(maximum.min(64 * 1024) + 1);
        file.take((maximum + 1) as u64)
            .read_to_end(&mut bytes)
            .map_err(|error| io_error(operation, error))?;
        if bytes.len() > maximum {
            return if maximum == MAX_WORKSHOP_ARCHIVE_BYTES {
                Err(WorkshopStoreError::ArchiveTooLarge { max_bytes: maximum })
            } else if maximum == MAX_WORKSHOP_PACK_BYTES {
                Err(WorkshopStoreError::PackTooLarge { max_bytes: maximum })
            } else {
                Err(WorkshopStoreError::CorruptManifest)
            };
        }
        Ok(bytes)
    }

    fn io_error(operation: StoreIoOperation, error: std::io::Error) -> WorkshopStoreError {
        WorkshopStoreError::Io {
            operation,
            kind: error.kind().into(),
        }
    }

    #[cfg(unix)]
    fn sync_directory(path: &Path) -> Result<(), WorkshopStoreError> {
        File::open(path)
            .and_then(|directory| directory.sync_all())
            .map_err(|error| io_error(StoreIoOperation::SyncDirectory, error))
    }

    #[cfg(not(unix))]
    fn sync_directory(_path: &Path) -> Result<(), WorkshopStoreError> {
        Ok(())
    }

    fn hex(bytes: &[u8]) -> String {
        const HEX: &[u8; 16] = b"0123456789abcdef";
        let mut result = String::with_capacity(bytes.len() * 2);
        for byte in bytes {
            result.push(HEX[(byte >> 4) as usize] as char);
            result.push(HEX[(byte & 0x0f) as usize] as char);
        }
        result
    }
}

#[cfg(not(target_arch = "wasm32"))]
pub use native::{
    NativeStoreFaultInjector, NativeWorkshopPlatform, NativeWorkshopStore, WorkshopPathEnvironment,
    native_workshop_root,
};
