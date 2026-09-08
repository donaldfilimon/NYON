//! Native filesystem-backed Workshop store.
//!
//! Extracted verbatim from the `mod native` block of the parent module.
//! `super::` still resolves to `crate::workshop::store`, exactly as it did
//! when this was an inline module, so no path in these bodies changed.

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
    pending: BTreeMap<StoreJobId, mpsc::Receiver<Result<WorkshopStoreResult, WorkshopStoreError>>>,
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
            WorkshopStoreRequest::UnarchiveSlot { slot } => {
                // Flag only. The generation descriptors, their digests, the
                // name and the Continue selection are copied through unchanged,
                // so no archive file is rewritten and nothing is reordered.
                let mut manifest = self.manifest.clone();
                manifest
                    .slots
                    .iter_mut()
                    .find(|record| record.id == slot)
                    .ok_or(WorkshopStoreError::UnknownSlot { slot })?
                    .archived = false;
                self.persist_manifest(manifest)?;
                Ok(WorkshopStoreResult::SlotUnarchived { slot })
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

    fn persist_pack(&mut self, hash: CatalogHash, pack: &[u8]) -> Result<(), WorkshopStoreError> {
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
    fn start(&mut self, request: WorkshopStoreRequest) -> Result<StoreJobId, WorkshopStoreError> {
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
