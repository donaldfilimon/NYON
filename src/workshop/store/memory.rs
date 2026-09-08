//! In-memory Workshop store.
//!
//! Extracted verbatim from the parent module. `super::` resolves to
//! `crate::workshop::store` exactly as it did inline, so the parent's
//! private helpers stay reachable and no path in these bodies changed.

use super::*;

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

    pub(super) fn execute(
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
            WorkshopStoreRequest::UnarchiveSlot { slot } => {
                // Flag only. Generations, name and Continue selection are left
                // exactly as archiving found them.
                self.slots
                    .get_mut(&slot)
                    .ok_or(WorkshopStoreError::UnknownSlot { slot })?
                    .archived = false;
                Ok(WorkshopStoreResult::SlotUnarchived { slot })
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
