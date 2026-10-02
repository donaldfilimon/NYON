//! Living-only persistence identities and transactional request surface.
use nyon_workshop_core::living::{
    LivingArchiveReplayStatusV2, LivingCatalogHashV2, LivingGalaxyAuthorityV2,
    begin_decode_living_notebook_v2, decode_living_archive_v2, decode_living_catalog_pack_v2,
};
use serde::{Deserialize, Serialize};

pub const MAX_LIVING_SLOTS: usize = 64;
pub const MAX_LIVING_PACKS: usize = 256;

pub const LIVING_DATABASE: &str = "nyon.living.v2";
#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Serialize, Deserialize)]
pub struct LivingSlotIdV2(pub u64);
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct LivingGenerationV2(pub u64);
#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub struct LivingStoreJobIdV2(pub u64);
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct LivingSlotDescriptorV2 {
    pub id: LivingSlotIdV2,
    pub name: String,
    pub archived: bool,
    pub generation: LivingGenerationV2,
    pub catalog_hash: LivingCatalogHashV2,
    #[serde(skip)]
    pub diagnostic: Option<LivingStoreErrorV2>,
}
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct LivingPackDescriptorV2 {
    #[serde(skip)]
    pub diagnostic: Option<LivingStoreErrorV2>,
    pub hash: LivingCatalogHashV2,
    pub archived: bool,
    pub generation: LivingGenerationV2,
}
/// Display-only exact retained archive reference. Mutations recheck references atomically.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LivingPackReferenceV2 {
    pub slot: LivingSlotIdV2,
    pub name: String,
    pub archived: bool,
    pub retained_generation: LivingGenerationV2,
}
#[derive(Clone, Debug)]
pub enum LivingStoreRequestV2 {
    ListSlots,
    ListPacks,
    ListPackReferences {
        hash: LivingCatalogHashV2,
    },
    LoadSlot {
        id: LivingSlotIdV2,
    },
    CreateSlot {
        name: String,
        archive: Vec<u8>,
    },
    SaveSlot {
        id: LivingSlotIdV2,
        expected_generation: LivingGenerationV2,
        archive: Vec<u8>,
    },
    RenameSlot {
        id: LivingSlotIdV2,
        expected_generation: LivingGenerationV2,
        name: String,
    },
    SetArchived {
        id: LivingSlotIdV2,
        expected_generation: LivingGenerationV2,
        archived: bool,
    },
    DeleteArchivedSlot {
        id: LivingSlotIdV2,
        expected_generation: LivingGenerationV2,
    },
    PutPack {
        canonical_bytes: Vec<u8>,
    },
    SetPackArchived {
        hash: LivingCatalogHashV2,
        expected_generation: LivingGenerationV2,
        archived: bool,
    },
    ReadPack {
        hash: LivingCatalogHashV2,
    },
    ReadArchive {
        id: LivingSlotIdV2,
        generation: LivingGenerationV2,
    },
    WriteNotebook {
        id: LivingSlotIdV2,
        expected_generation: LivingGenerationV2,
        canonical_bytes: Vec<u8>,
    },
    ReadNotebook {
        id: LivingSlotIdV2,
        generation: LivingGenerationV2,
    },
    DeletePack {
        hash: LivingCatalogHashV2,
        expected_generation: LivingGenerationV2,
    },
}
#[derive(Clone, Debug)]
pub enum LivingStoreResultV2 {
    Slots(Vec<LivingSlotDescriptorV2>),
    Packs(Vec<LivingPackDescriptorV2>),
    PackReferences {
        hash: LivingCatalogHashV2,
        references: Vec<LivingPackReferenceV2>,
    },
    Loaded {
        descriptor: LivingSlotDescriptorV2,
        authority: Box<LivingGalaxyAuthorityV2>,
    },
    Slot(LivingSlotDescriptorV2),
    Pack(LivingPackDescriptorV2),
    Bytes {
        canonical_bytes: Vec<u8>,
        file_sha256: [u8; 32],
    },
    NotebookBytes(Option<Vec<u8>>),
    Deleted,
}
#[derive(Clone, Debug, Eq, PartialEq, thiserror::Error)]
pub enum LivingStoreErrorV2 {
    #[error("not found")]
    NotFound,
    #[error("stale generation")]
    StaleGeneration,
    #[error("capacity limit")]
    Limit,
    #[error("input exceeds byte limit")]
    TooLarge,
    #[error("wrong authority")]
    WrongAuthority,
    #[error("noncanonical input")]
    NonCanonical,
    #[error("integrity or replay mismatch")]
    Integrity,
    #[error("missing catalog")]
    MissingCatalog,
    #[error("referenced content")]
    Referenced,
    #[error("invalid lifecycle")]
    InvalidLifecycle,
    #[error("wrong job owner")]
    WrongOwner,
    #[error("cancelled")]
    Cancelled,
    #[error("corrupt recovery")]
    CorruptRecovery,
    #[error("storage: {0}")]
    Storage(String),
}
#[derive(Clone, Debug)]
pub enum LivingStorePollV2 {
    Pending { work_done: u64 },
    Ready(LivingStoreResultV2),
    Failed(LivingStoreErrorV2),
}
#[derive(Clone, Debug)]
pub enum LivingStoreCancelV2 {
    Cancelled,
    CancelPending,
    CommitAlreadyWon(LivingStoreResultV2),
}
pub trait LivingStoreV2 {
    fn begin(
        &mut self,
        owner_epoch: u64,
        request: LivingStoreRequestV2,
    ) -> Result<LivingStoreJobIdV2, LivingStoreErrorV2>;
    fn poll(
        &mut self,
        owner_epoch: u64,
        job: LivingStoreJobIdV2,
        work_budget: usize,
    ) -> LivingStorePollV2;
    fn cancel(
        &mut self,
        owner_epoch: u64,
        job: LivingStoreJobIdV2,
    ) -> Result<LivingStoreCancelV2, LivingStoreErrorV2>;
    fn release(&mut self, owner_epoch: u64, job: LivingStoreJobIdV2) -> bool;
}
#[derive(Clone, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct State {
    pub(crate) next_slot: u64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) pack_generation_high_water: Option<u64>,
    slots: Vec<Slot>,
    packs: Vec<Pack>,
}
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Slot {
    descriptor: LivingSlotDescriptorV2,
    archives: Vec<(LivingGenerationV2, Vec<u8>)>,
    retained_catalogs: Vec<LivingCatalogHashV2>,
    #[serde(default)]
    notebooks: Vec<(LivingGenerationV2, Vec<u8>)>,
}
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Pack {
    descriptor: LivingPackDescriptorV2,
    bytes: Vec<u8>,
}
fn next(generation: LivingGenerationV2) -> Result<LivingGenerationV2, LivingStoreErrorV2> {
    generation
        .0
        .checked_add(1)
        .map(LivingGenerationV2)
        .ok_or(LivingStoreErrorV2::Limit)
}
fn name(value: &str) -> Result<(), LivingStoreErrorV2> {
    if value.is_empty()
        || value.len() > 64
        || !value.bytes().all(|b| (b' '..=b'~').contains(&b))
        || value.starts_with(' ')
        || value.ends_with(' ')
        || value.contains("  ")
    {
        Err(LivingStoreErrorV2::NonCanonical)
    } else {
        Ok(())
    }
}
fn hash(bytes: &[u8]) -> [u8; 32] {
    use sha2::{Digest, Sha256};
    Sha256::digest(bytes).into()
}
pub(crate) fn preflight(request: &LivingStoreRequestV2) -> Result<(), LivingStoreErrorV2> {
    let (bytes, limit, kind) = match request {
        LivingStoreRequestV2::CreateSlot { archive, .. }
        | LivingStoreRequestV2::SaveSlot { archive, .. } => {
            if let LivingStoreRequestV2::CreateSlot { name: value, .. } = request {
                name(value)?;
            }
            (archive, 32 * 1024 * 1024, "NYON_LIVING_GALAXY_ARCHIVE")
        }
        LivingStoreRequestV2::WriteNotebook {
            canonical_bytes, ..
        } => (canonical_bytes, 1024 * 1024, "NYON_LIVING_NOTEBOOK"),
        LivingStoreRequestV2::PutPack { canonical_bytes } => {
            (canonical_bytes, 2 * 1024 * 1024, "NYON_LIVING_GALAXY_DATA")
        }
        LivingStoreRequestV2::RenameSlot { name: value, .. } => return name(value),
        _ => return Ok(()),
    };
    if bytes.len() > limit {
        return Err(LivingStoreErrorV2::TooLarge);
    }
    let prefix = format!("{{\"kind\":\"{kind}\"");
    if !bytes.starts_with(prefix.as_bytes()) {
        return Err(LivingStoreErrorV2::WrongAuthority);
    }
    Ok(())
}
impl Slot {
    fn references(&self, hash: LivingCatalogHashV2) -> bool {
        self.descriptor.catalog_hash == hash
            || self.retained_catalogs.contains(&hash)
            || self.archives.iter().any(|(_, bytes)| {
                serde_json::from_slice::<serde_json::Value>(bytes)
                    .ok()
                    .and_then(|value| value.get("catalog_hash").cloned())
                    .and_then(|value| serde_json::from_value::<LivingCatalogHashV2>(value).ok())
                    == Some(hash)
            })
    }
    fn advance_metadata(
        &mut self,
        generation: LivingGenerationV2,
    ) -> Result<(), LivingStoreErrorV2> {
        let archive = self
            .archives
            .last()
            .ok_or(LivingStoreErrorV2::CorruptRecovery)?
            .1
            .clone();
        if let Some((_, bytes)) = self
            .notebooks
            .iter()
            .find(|(g, _)| *g == self.descriptor.generation)
        {
            self.notebooks.push((generation, bytes.clone()));
        }
        self.descriptor.generation = generation;
        self.archives.push((generation, archive));
        self.retained_catalogs.push(self.descriptor.catalog_hash);
        if self.retained_catalogs.len() > 2 {
            self.retained_catalogs.remove(0);
        }
        if self.archives.len() > 2 {
            self.archives.remove(0);
        }
        self.notebooks
            .retain(|(g, _)| self.archives.iter().any(|(a, _)| a == g));
        Ok(())
    }
}
impl State {
    fn validate_archive(
        &self,
        bytes: &[u8],
    ) -> Result<(LivingCatalogHashV2, LivingGalaxyAuthorityV2), LivingStoreErrorV2> {
        let value: serde_json::Value =
            serde_json::from_slice(bytes).map_err(|_| LivingStoreErrorV2::NonCanonical)?;
        let catalog_hash: LivingCatalogHashV2 = serde_json::from_value(
            value
                .get("catalog_hash")
                .cloned()
                .ok_or(LivingStoreErrorV2::NonCanonical)?,
        )
        .map_err(|_| LivingStoreErrorV2::NonCanonical)?;
        let pack = self
            .packs
            .iter()
            .find(|pack| pack.descriptor.hash == catalog_hash)
            .ok_or(LivingStoreErrorV2::MissingCatalog)?;
        let catalog = decode_living_catalog_pack_v2(&pack.bytes)
            .map_err(|_| LivingStoreErrorV2::Integrity)?;
        let archive =
            decode_living_archive_v2(bytes, &catalog).map_err(|_| LivingStoreErrorV2::Integrity)?;
        Ok((catalog_hash, archive.into_authority()))
    }
    #[cfg(not(target_arch = "wasm32"))]
    pub(crate) fn execute(
        &mut self,
        request: LivingStoreRequestV2,
    ) -> Result<LivingStoreResultV2, LivingStoreErrorV2> {
        self.execute_ready(request, None, false)
    }
    pub(crate) fn execute_ready(
        &mut self,
        request: LivingStoreRequestV2,
        validated: Option<(LivingCatalogHashV2, LivingGalaxyAuthorityV2)>,
        notebook_validated: bool,
    ) -> Result<LivingStoreResultV2, LivingStoreErrorV2> {
        preflight(&request)?;
        let existing_max = self
            .packs
            .iter()
            .map(|p| p.descriptor.generation.0)
            .max()
            .unwrap_or(0);
        if self
            .pack_generation_high_water
            .is_some_and(|high| high < existing_max)
        {
            return Err(LivingStoreErrorV2::Integrity);
        }
        use LivingStoreRequestV2::*;
        match request {
            ListSlots => Ok(LivingStoreResultV2::Slots(
                self.slots.iter().map(|s| s.descriptor.clone()).collect(),
            )),
            ListPacks => Ok(LivingStoreResultV2::Packs(
                self.packs.iter().map(|s| s.descriptor.clone()).collect(),
            )),
            ListPackReferences { hash } => {
                if self
                    .slots
                    .iter()
                    .any(|slot| slot.archives.len() != slot.retained_catalogs.len())
                {
                    return Err(LivingStoreErrorV2::CorruptRecovery);
                }
                if !self.packs.iter().any(|pack| pack.descriptor.hash == hash) {
                    return Err(LivingStoreErrorV2::NotFound);
                }
                let mut references = self
                    .slots
                    .iter()
                    .flat_map(|slot| {
                        slot.archives
                            .iter()
                            .zip(&slot.retained_catalogs)
                            .filter(move |(_, catalog)| **catalog == hash)
                            .map(move |((generation, _), _)| LivingPackReferenceV2 {
                                slot: slot.descriptor.id,
                                name: slot.descriptor.name.clone(),
                                archived: slot.descriptor.archived,
                                retained_generation: *generation,
                            })
                    })
                    .collect::<Vec<_>>();
                references
                    .sort_by_key(|reference| (reference.slot.0, reference.retained_generation.0));
                Ok(LivingStoreResultV2::PackReferences { hash, references })
            }
            PutPack { canonical_bytes } => {
                let catalog = decode_living_catalog_pack_v2(&canonical_bytes)
                    .map_err(|_| LivingStoreErrorV2::NonCanonical)?;
                let hash = catalog.catalog_hash();
                if let Some(pack) = self.packs.iter().find(|pack| pack.descriptor.hash == hash) {
                    return Ok(LivingStoreResultV2::Pack(pack.descriptor.clone()));
                }
                if self.packs.len() >= MAX_LIVING_PACKS {
                    return Err(LivingStoreErrorV2::Limit);
                }
                let descriptor = LivingPackDescriptorV2 {
                    diagnostic: None,
                    hash,
                    archived: false,
                    generation: LivingGenerationV2(
                        self.pack_generation_high_water
                            .unwrap_or(existing_max)
                            .checked_add(1)
                            .ok_or(LivingStoreErrorV2::Limit)?,
                    ),
                };
                self.pack_generation_high_water = Some(descriptor.generation.0);
                self.packs.push(Pack {
                    descriptor: descriptor.clone(),
                    bytes: canonical_bytes,
                });
                self.packs.sort_by_key(|pack| pack.descriptor.hash);
                Ok(LivingStoreResultV2::Pack(descriptor))
            }
            CreateSlot { name, archive } => {
                if self.slots.len() >= MAX_LIVING_SLOTS {
                    return Err(LivingStoreErrorV2::Limit);
                }
                let (catalog_hash, _) = match &validated {
                    Some(value) => value.clone(),
                    None => self.validate_archive(&archive)?,
                };
                let id = self
                    .next_slot
                    .checked_add(1)
                    .ok_or(LivingStoreErrorV2::Limit)?;
                let descriptor = LivingSlotDescriptorV2 {
                    id: LivingSlotIdV2(id),
                    name,
                    archived: false,
                    generation: LivingGenerationV2(1),
                    catalog_hash,
                    diagnostic: None,
                };
                self.slots.push(Slot {
                    descriptor: descriptor.clone(),
                    archives: vec![(descriptor.generation, archive)],
                    retained_catalogs: vec![catalog_hash],
                    notebooks: Vec::new(),
                });
                self.next_slot = id;
                Ok(LivingStoreResultV2::Slot(descriptor))
            }
            LoadSlot { id } => {
                let slot = self
                    .slots
                    .iter()
                    .find(|s| s.descriptor.id == id)
                    .ok_or(LivingStoreErrorV2::NotFound)?;
                if slot.descriptor.archived {
                    return Err(LivingStoreErrorV2::InvalidLifecycle);
                }
                let mut recovery_error = LivingStoreErrorV2::CorruptRecovery;
                for (generation, bytes) in slot.archives.iter().rev() {
                    match match &validated {
                        Some(value) => Ok(value.clone()),
                        None => self.validate_archive(bytes),
                    } {
                        Ok((catalog_hash, authority)) => {
                            let mut descriptor = slot.descriptor.clone();
                            descriptor.generation = *generation;
                            descriptor.catalog_hash = catalog_hash;
                            return Ok(LivingStoreResultV2::Loaded {
                                descriptor,
                                authority: Box::new(authority),
                            });
                        }
                        Err(error) => recovery_error = error,
                    }
                }
                Err(recovery_error)
            }
            SaveSlot {
                id,
                expected_generation,
                archive,
            } => {
                let index = self
                    .slots
                    .iter()
                    .position(|s| s.descriptor.id == id)
                    .ok_or(LivingStoreErrorV2::NotFound)?;
                if self.slots[index].descriptor.generation != expected_generation {
                    return Err(LivingStoreErrorV2::StaleGeneration);
                }
                if self.slots[index].descriptor.archived {
                    return Err(LivingStoreErrorV2::InvalidLifecycle);
                }
                let (catalog_hash, _) = match &validated {
                    Some(value) => value.clone(),
                    None => self.validate_archive(&archive)?,
                };
                let slot = &mut self.slots[index];
                slot.descriptor.generation = next(expected_generation)?;
                slot.descriptor.catalog_hash = catalog_hash;
                slot.archives.push((slot.descriptor.generation, archive));
                slot.retained_catalogs.push(catalog_hash);
                if slot.retained_catalogs.len() > 2 {
                    slot.retained_catalogs.remove(0);
                }
                if slot.archives.len() > 2 {
                    slot.archives.remove(0);
                }
                slot.notebooks
                    .retain(|(g, _)| slot.archives.iter().any(|(a, _)| a == g));
                Ok(LivingStoreResultV2::Slot(slot.descriptor.clone()))
            }
            RenameSlot {
                id,
                expected_generation,
                name,
            } => {
                let slot = self.slot_mut(id, expected_generation)?;
                let generation = next(expected_generation)?;
                slot.descriptor.name = name;
                slot.advance_metadata(generation)?;
                Ok(LivingStoreResultV2::Slot(slot.descriptor.clone()))
            }
            SetArchived {
                id,
                expected_generation,
                archived,
            } => {
                let slot = self.slot_mut(id, expected_generation)?;
                let generation = next(expected_generation)?;
                slot.descriptor.archived = archived;
                slot.advance_metadata(generation)?;
                Ok(LivingStoreResultV2::Slot(slot.descriptor.clone()))
            }
            DeleteArchivedSlot {
                id,
                expected_generation,
            } => {
                let slot = self.slot_mut(id, expected_generation)?;
                if !slot.descriptor.archived {
                    return Err(LivingStoreErrorV2::InvalidLifecycle);
                }
                self.slots.retain(|s| s.descriptor.id != id);
                Ok(LivingStoreResultV2::Deleted)
            }
            ReadPack { hash: id } => {
                let pack = self
                    .packs
                    .iter()
                    .find(|p| p.descriptor.hash == id)
                    .ok_or(LivingStoreErrorV2::NotFound)?;
                Ok(LivingStoreResultV2::Bytes {
                    canonical_bytes: pack.bytes.clone(),
                    file_sha256: hash(&pack.bytes),
                })
            }
            ReadArchive { id, generation } => {
                let slot = self
                    .slots
                    .iter()
                    .find(|s| s.descriptor.id == id)
                    .ok_or(LivingStoreErrorV2::NotFound)?;
                let bytes = &slot
                    .archives
                    .iter()
                    .find(|(g, _)| *g == generation)
                    .ok_or(LivingStoreErrorV2::NotFound)?
                    .1;
                Ok(LivingStoreResultV2::Bytes {
                    canonical_bytes: bytes.clone(),
                    file_sha256: hash(bytes),
                })
            }
            ReadNotebook { id, generation } => {
                let slot = self
                    .slots
                    .iter()
                    .find(|s| s.descriptor.id == id)
                    .ok_or(LivingStoreErrorV2::NotFound)?;
                if !slot.archives.iter().any(|(g, _)| *g == generation) {
                    return Err(LivingStoreErrorV2::NotFound);
                }
                Ok(LivingStoreResultV2::NotebookBytes(
                    slot.notebooks
                        .iter()
                        .find(|(g, _)| *g == generation)
                        .map(|(_, bytes)| bytes.clone()),
                ))
            }
            WriteNotebook {
                id,
                expected_generation,
                canonical_bytes,
            } => {
                let slot = self
                    .slots
                    .iter()
                    .find(|s| s.descriptor.id == id)
                    .ok_or(LivingStoreErrorV2::NotFound)?;
                if slot.descriptor.generation != expected_generation {
                    return Err(LivingStoreErrorV2::StaleGeneration);
                }
                if slot.descriptor.archived {
                    return Err(LivingStoreErrorV2::InvalidLifecycle);
                }
                if !notebook_validated {
                    let bytes = &slot
                        .archives
                        .last()
                        .ok_or(LivingStoreErrorV2::CorruptRecovery)?
                        .1;
                    let pack = self
                        .packs
                        .iter()
                        .find(|p| p.descriptor.hash == slot.descriptor.catalog_hash)
                        .ok_or(LivingStoreErrorV2::MissingCatalog)?;
                    let catalog = decode_living_catalog_pack_v2(&pack.bytes)
                        .map_err(|_| LivingStoreErrorV2::Integrity)?;
                    let archive = decode_living_archive_v2(bytes, &catalog)
                        .map_err(|_| LivingStoreErrorV2::Integrity)?;
                    let mut replay = begin_decode_living_notebook_v2(&canonical_bytes, archive)
                        .map_err(|_| LivingStoreErrorV2::Integrity)?;
                    while replay
                        .poll(128)
                        .map_err(|_| LivingStoreErrorV2::Integrity)?
                        == LivingArchiveReplayStatusV2::Pending
                    {}
                    replay.finish().map_err(|_| LivingStoreErrorV2::Integrity)?;
                }
                let slot = self.slot_mut(id, expected_generation)?;
                let generation = next(expected_generation)?;
                slot.advance_metadata(generation)?;
                slot.notebooks.retain(|(g, _)| *g != generation);
                slot.notebooks.push((generation, canonical_bytes));
                Ok(LivingStoreResultV2::Slot(slot.descriptor.clone()))
            }
            SetPackArchived {
                hash,
                expected_generation,
                archived,
            } => {
                let current = self
                    .packs
                    .iter()
                    .find(|pack| pack.descriptor.hash == hash)
                    .ok_or(LivingStoreErrorV2::NotFound)?;
                if current.descriptor.generation != expected_generation {
                    return Err(LivingStoreErrorV2::StaleGeneration);
                }
                if archived
                    && self
                        .slots
                        .iter()
                        .any(|s| !s.descriptor.archived && s.references(hash))
                {
                    return Err(LivingStoreErrorV2::Referenced);
                }
                let pack = self
                    .packs
                    .iter_mut()
                    .find(|p| p.descriptor.hash == hash)
                    .ok_or(LivingStoreErrorV2::NotFound)?;
                if pack.descriptor.generation != expected_generation {
                    return Err(LivingStoreErrorV2::StaleGeneration);
                }
                let generation = self
                    .pack_generation_high_water
                    .unwrap_or(existing_max)
                    .checked_add(1)
                    .ok_or(LivingStoreErrorV2::Limit)?;
                pack.descriptor.generation = LivingGenerationV2(generation);
                self.pack_generation_high_water = Some(generation);
                pack.descriptor.archived = archived;
                Ok(LivingStoreResultV2::Pack(pack.descriptor.clone()))
            }
            DeletePack {
                hash,
                expected_generation,
            } => {
                let pack = self
                    .packs
                    .iter()
                    .find(|p| p.descriptor.hash == hash)
                    .ok_or(LivingStoreErrorV2::NotFound)?;
                if pack.descriptor.generation != expected_generation {
                    return Err(LivingStoreErrorV2::StaleGeneration);
                }
                if !pack.descriptor.archived {
                    return Err(LivingStoreErrorV2::InvalidLifecycle);
                }
                if self.slots.iter().any(|s| s.references(hash)) {
                    return Err(LivingStoreErrorV2::Referenced);
                }
                self.pack_generation_high_water =
                    Some(self.pack_generation_high_water.unwrap_or(existing_max));
                self.packs.retain(|p| p.descriptor.hash != hash);
                Ok(LivingStoreResultV2::Deleted)
            }
        }
    }
    fn slot_mut(
        &mut self,
        id: LivingSlotIdV2,
        generation: LivingGenerationV2,
    ) -> Result<&mut Slot, LivingStoreErrorV2> {
        let slot = self
            .slots
            .iter_mut()
            .find(|s| s.descriptor.id == id)
            .ok_or(LivingStoreErrorV2::NotFound)?;
        if slot.descriptor.generation != generation {
            return Err(LivingStoreErrorV2::StaleGeneration);
        }
        Ok(slot)
    }
}
#[cfg(not(target_arch = "wasm32"))]
pub mod native;

mod manifest;
#[cfg(target_arch = "wasm32")]
pub mod web;

#[cfg(not(target_arch = "wasm32"))]
fn allocate_job_id() -> Result<u64, LivingStoreErrorV2> {
    use std::sync::atomic::{AtomicU64, Ordering};
    static NEXT: AtomicU64 = AtomicU64::new(0);
    NEXT.try_update(Ordering::Relaxed, Ordering::Relaxed, |value| {
        value.checked_add(1)
    })
    .map(|value| value + 1)
    .map_err(|_| LivingStoreErrorV2::Limit)
}
#[cfg(target_arch = "wasm32")]
fn allocate_job_id() -> Result<u64, LivingStoreErrorV2> {
    thread_local! {static NEXT:std::cell::Cell<u64>=const {std::cell::Cell::new(0)};}
    NEXT.with(|next| {
        let value = next.get().checked_add(1).ok_or(LivingStoreErrorV2::Limit)?;
        next.set(value);
        Ok(value)
    })
}

#[cfg(test)]
mod pack_generation_tests {
    use super::*;
    use nyon_workshop_core::living::LIVING_CORE_PACK_BYTES_V2;
    #[test]
    fn pack_generation_overflow_preserves_candidate() {
        let mut state = State {
            pack_generation_high_water: Some(u64::MAX),
            ..State::default()
        };
        let before = serde_json::to_vec(&state).unwrap();
        assert_eq!(
            state
                .execute(LivingStoreRequestV2::PutPack {
                    canonical_bytes: LIVING_CORE_PACK_BYTES_V2.to_vec()
                })
                .unwrap_err(),
            LivingStoreErrorV2::Limit
        );
        assert_eq!(serde_json::to_vec(&state).unwrap(), before);
    }
    #[test]
    fn legacy_pack_high_water_initializes_only_on_mutation_and_rejects_corrupt_present_value() {
        let mut state = State::default();
        let LivingStoreResultV2::Pack(pack) = state
            .execute(LivingStoreRequestV2::PutPack {
                canonical_bytes: LIVING_CORE_PACK_BYTES_V2.to_vec(),
            })
            .unwrap()
        else {
            panic!()
        };
        state.pack_generation_high_water = None;
        let before = serde_json::to_vec(&state).unwrap();
        state.execute(LivingStoreRequestV2::ListPacks).unwrap();
        assert_eq!(serde_json::to_vec(&state).unwrap(), before);
        let LivingStoreResultV2::Pack(updated) = state
            .execute(LivingStoreRequestV2::SetPackArchived {
                hash: pack.hash,
                expected_generation: pack.generation,
                archived: true,
            })
            .unwrap()
        else {
            panic!()
        };
        assert_eq!(updated.generation.0, pack.generation.0 + 1);
        state.pack_generation_high_water = Some(0);
        assert_eq!(
            state.execute(LivingStoreRequestV2::ListPacks).unwrap_err(),
            LivingStoreErrorV2::Integrity
        );
    }
}
