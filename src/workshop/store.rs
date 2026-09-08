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
mod native;

#[cfg(not(target_arch = "wasm32"))]
pub use native::{
    NativeStoreFaultInjector, NativeWorkshopPlatform, NativeWorkshopStore, WorkshopPathEnvironment,
    native_workshop_root,
};
