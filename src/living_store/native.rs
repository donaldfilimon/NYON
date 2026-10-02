//! Native per-object bounded payloads and two recoverable manifest generations.
use super::manifest::*;
use super::*;
use std::{
    collections::{BTreeMap, BTreeSet},
    fs::{self, OpenOptions},
    io::{Read, Write},
    path::{Path, PathBuf},
    sync::{
        Arc,
        atomic::{AtomicU8, Ordering},
        mpsc,
    },
    thread,
};
impl DiskState {
    fn materialize(
        &self,
        root: &Path,
        request: &LivingStoreRequestV2,
    ) -> Result<State, LivingStoreErrorV2> {
        let mut state = State {
            next_slot: self.next_slot,
            pack_generation_high_water: self.pack_generation_high_water,
            slots: self
                .slots
                .iter()
                .map(|s| Slot {
                    descriptor: s.descriptor.clone(),
                    archives: s.archives.iter().map(|(g, _)| (*g, Vec::new())).collect(),
                    retained_catalogs: s.archives.iter().map(|(_, r)| r.catalog).collect(),
                    notebooks: s.notebooks.iter().map(|(g, _)| (*g, Vec::new())).collect(),
                })
                .collect(),
            packs: self
                .packs
                .iter()
                .map(|p| Pack {
                    descriptor: p.descriptor.clone(),
                    bytes: Vec::new(),
                })
                .collect(),
        };
        let slot_id = match request {
            LivingStoreRequestV2::LoadSlot { id }
            | LivingStoreRequestV2::ReadArchive { id, .. }
            | LivingStoreRequestV2::ReadNotebook { id, .. }
            | LivingStoreRequestV2::WriteNotebook { id, .. }
            | LivingStoreRequestV2::RenameSlot { id, .. }
            | LivingStoreRequestV2::SetArchived { id, .. }
            | LivingStoreRequestV2::SaveSlot { id, .. } => Some(*id),
            _ => None,
        };
        let mut needed = BTreeSet::new();
        if let Some(id) = slot_id
            && let Some((i, slot)) = self
                .slots
                .iter()
                .enumerate()
                .find(|(_, s)| s.descriptor.id == id)
        {
            for (j, (_, r)) in
                slot.archives
                    .iter()
                    .enumerate()
                    .filter(|(_, (g, _))| match request {
                        LivingStoreRequestV2::ReadArchive { generation, .. }
                        | LivingStoreRequestV2::ReadNotebook { generation, .. } => {
                            g.0 == generation.0
                        }
                        _ => true,
                    })
            {
                match read_payload(root, r, 32 * 1024 * 1024) {
                    Ok(bytes) => state.slots[i].archives[j].1 = bytes,
                    Err(_)
                        if !matches!(
                            request,
                            LivingStoreRequestV2::ReadArchive { .. }
                                | LivingStoreRequestV2::ReadNotebook { .. }
                        ) => {}
                    Err(error) => return Err(error),
                }
                needed.insert(r.catalog);
            }
            for (j, (_, r)) in slot
                .notebooks
                .iter()
                .enumerate()
                .filter(|(_, (g, _))| match request {
                    LivingStoreRequestV2::ReadNotebook { generation, .. } => g.0 == generation.0,
                    LivingStoreRequestV2::RenameSlot { .. }
                    | LivingStoreRequestV2::SetArchived { .. } => *g == slot.descriptor.generation,
                    _ => false,
                })
            {
                state.slots[i].notebooks[j].1 = read_payload(root, r, 1024 * 1024)?;
            }
        }
        match request {
            LivingStoreRequestV2::ReadPack { hash } => {
                needed.insert(*hash);
            }
            LivingStoreRequestV2::CreateSlot { archive, .. }
            | LivingStoreRequestV2::SaveSlot { archive, .. } => {
                let v: serde_json::Value = serde_json::from_slice(archive)
                    .map_err(|_| LivingStoreErrorV2::NonCanonical)?;
                let h = serde_json::from_value(
                    v.get("catalog_hash")
                        .cloned()
                        .ok_or(LivingStoreErrorV2::NonCanonical)?,
                )
                .map_err(|_| LivingStoreErrorV2::NonCanonical)?;
                needed.insert(h);
            }
            _ => {}
        }
        for (i, pack) in self.packs.iter().enumerate() {
            if needed.contains(&pack.descriptor.hash) {
                match read_payload(root, &pack.reference, 2 * 1024 * 1024) {
                    Ok(bytes) => state.packs[i].bytes = bytes,
                    Err(_)
                        if slot_id.is_some()
                            && !matches!(
                                request,
                                LivingStoreRequestV2::ReadArchive { .. }
                                    | LivingStoreRequestV2::ReadNotebook { .. }
                            ) => {}
                    Err(error) => return Err(error),
                }
            }
        }
        let expected = match request {
            LivingStoreRequestV2::SaveSlot {
                id,
                expected_generation,
                ..
            }
            | LivingStoreRequestV2::WriteNotebook {
                id,
                expected_generation,
                ..
            }
            | LivingStoreRequestV2::RenameSlot {
                id,
                expected_generation,
                ..
            }
            | LivingStoreRequestV2::SetArchived {
                id,
                expected_generation,
                ..
            } => Some((*id, *expected_generation)),
            _ => None,
        };
        if let Some((id, expected)) = expected
            && let Some(index) = state.slots.iter().position(|slot| slot.descriptor.id == id)
        {
            let mut recovered = None;
            for (position, (generation, bytes)) in
                state.slots[index].archives.iter().enumerate().rev()
            {
                if let Ok((catalog, _)) = state.validate_archive(bytes) {
                    recovered = Some((position, *generation, catalog));
                    break;
                }
            }
            let (position, generation, catalog) =
                recovered.ok_or(LivingStoreErrorV2::CorruptRecovery)?;
            if generation != expected {
                return Err(LivingStoreErrorV2::StaleGeneration);
            }
            let slot = &mut state.slots[index];
            slot.archives.truncate(position + 1);
            slot.retained_catalogs.truncate(position + 1);
            slot.notebooks
                .retain(|(g, _)| slot.archives.iter().any(|(a, _)| a == g));
            slot.descriptor.generation = generation;
            slot.descriptor.catalog_hash = catalog;
        }
        Ok(state)
    }
    fn persist_candidate(
        &self,
        root: &Path,
        state: State,
        journal: &mut StagingJournal,
    ) -> Result<Self, LivingStoreErrorV2> {
        let mut disk = Self {
            next_slot: state.next_slot,
            pack_generation_high_water: state.pack_generation_high_water,
            ..Self::default()
        };
        for pack in state.packs {
            let reference = if pack.bytes.is_empty() {
                self.packs
                    .iter()
                    .find(|p| p.descriptor.hash == pack.descriptor.hash)
                    .ok_or(LivingStoreErrorV2::CorruptRecovery)?
                    .reference
                    .clone()
            } else {
                persist_payload(root, &pack.bytes, pack.descriptor.hash, journal)?
            };
            disk.packs.push(DiskPack {
                descriptor: pack.descriptor,
                reference,
            });
        }
        for slot in state.slots {
            let mut archives = Vec::new();
            for (i, (g, bytes)) in slot.archives.into_iter().enumerate() {
                let reference = if bytes.is_empty() {
                    self.slots
                        .iter()
                        .find(|s| s.descriptor.id == slot.descriptor.id)
                        .and_then(|s| s.archives.iter().find(|(old, _)| *old == g))
                        .ok_or(LivingStoreErrorV2::CorruptRecovery)?
                        .1
                        .clone()
                } else {
                    persist_payload(
                        root,
                        &bytes,
                        *slot
                            .retained_catalogs
                            .get(i)
                            .ok_or(LivingStoreErrorV2::CorruptRecovery)?,
                        journal,
                    )?
                };
                archives.push((g, reference));
            }
            let mut notebooks = Vec::new();
            for (g, bytes) in slot.notebooks {
                let r = if bytes.is_empty() {
                    self.slots
                        .iter()
                        .find(|s| s.descriptor.id == slot.descriptor.id)
                        .and_then(|s| s.notebooks.iter().find(|(old, _)| *old == g))
                        .ok_or(LivingStoreErrorV2::CorruptRecovery)?
                        .1
                        .clone()
                } else {
                    persist_payload(root, &bytes, slot.descriptor.catalog_hash, journal)?
                };
                notebooks.push((g, r));
            }
            disk.slots.push(DiskSlot {
                descriptor: slot.descriptor,
                archives,
                notebooks,
            });
        }
        disk.validate()?;
        Ok(disk)
    }
}
fn storage(e: std::io::Error) -> LivingStoreErrorV2 {
    LivingStoreErrorV2::Storage(e.to_string())
}
fn read_bounded(path: &Path, limit: usize) -> Result<Option<Vec<u8>>, LivingStoreErrorV2> {
    let file = match fs::File::open(path) {
        Ok(f) => f,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(e) => return Err(storage(e)),
    };
    if file.metadata().map_err(storage)?.len() > limit as u64 {
        return Err(LivingStoreErrorV2::TooLarge);
    }
    let mut bytes = Vec::new();
    file.take(limit as u64 + 1)
        .read_to_end(&mut bytes)
        .map_err(storage)?;
    if bytes.len() > limit {
        return Err(LivingStoreErrorV2::TooLarge);
    }
    Ok(Some(bytes))
}
fn read_payload(root: &Path, r: &Reference, limit: usize) -> Result<Vec<u8>, LivingStoreErrorV2> {
    validate_ref(r, limit)?;
    let bytes =
        read_bounded(&root.join(&r.file), limit)?.ok_or(LivingStoreErrorV2::CorruptRecovery)?;
    if bytes.len() as u64 != r.len || hash(&bytes) != r.sha256 {
        return Err(LivingStoreErrorV2::Integrity);
    }
    Ok(bytes)
}
fn persist_payload(
    root: &Path,
    bytes: &[u8],
    catalog: LivingCatalogHashV2,
    journal: &mut StagingJournal,
) -> Result<Reference, LivingStoreErrorV2> {
    let digest = hash(bytes);
    let file = digest
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect::<String>();
    let reference = Reference {
        file: file.clone(),
        len: bytes.len() as u64,
        sha256: digest,
        catalog,
    };
    match read_bounded(&root.join(&file), bytes.len())? {
        Some(old) if old.len() == bytes.len() && hash(&old) == digest => {}
        Some(_) => return Err(LivingStoreErrorV2::Integrity),
        None => {
            journal.track(root, &reference)?;
            write_atomic(root, &file, bytes)?;
        }
    }
    Ok(reference)
}
fn recover(root: &Path) -> Result<(u64, DiskState, &'static str), LivingStoreErrorV2> {
    let a = match read_bounded(&root.join("generation-a"), MANIFEST_LIMIT) {
        Err(LivingStoreErrorV2::TooLarge) => Some(Vec::new()),
        result => result?,
    };
    let b = match read_bounded(&root.join("generation-b"), MANIFEST_LIMIT) {
        Err(LivingStoreErrorV2::TooLarge) => Some(Vec::new()),
        result => result?,
    };
    let head = read_bounded(&root.join("head"), 32)?;
    if a.is_none() && b.is_none() && head.is_none() {
        return Ok((0, DiskState::default(), "generation-a"));
    }
    let marker = head
        .as_ref()
        .and_then(|b| std::str::from_utf8(b).ok())
        .and_then(|s| s.parse::<u64>().ok());
    let mut valid: Vec<_> = [(a, "generation-b"), (b, "generation-a")]
        .into_iter()
        .filter_map(|(b, side)| {
            b.and_then(|bytes| Manifest::decode(&bytes).ok())
                .map(|g| (g, side))
        })
        .collect();
    // Legacy payload-only checksums cannot authenticate recovery ordering.
    // Admit them only through the intact exact head marker.
    valid.retain(|(g, _)| g.format_version == Some(2) || marker == Some(g.generation));
    valid.sort_by_key(|(g, _)| g.generation);
    if let Some(marker) = marker {
        if let Some(i) = valid.iter().position(|(g, _)| g.generation == marker) {
            let (g, side) = valid.remove(i);
            return Ok((g.generation, g.payload, side));
        }
        valid.retain(|(g, _)| g.generation < marker);
    }
    valid
        .pop()
        .map(|(g, side)| (g.generation, g.payload, side))
        .ok_or(LivingStoreErrorV2::CorruptRecovery)
}
fn write_atomic(root: &Path, name: &str, bytes: &[u8]) -> Result<(), LivingStoreErrorV2> {
    let pending = root.join(format!("{name}.pending"));
    if pending.exists() {
        fs::remove_file(&pending).map_err(storage)?;
    }
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&pending)
        .map_err(storage)?;
    file.write_all(bytes).map_err(storage)?;
    file.sync_all().map_err(storage)?;
    drop(file);
    fs::rename(pending, root.join(name)).map_err(storage)?;
    fs::File::open(root)
        .and_then(|f| f.sync_all())
        .map_err(storage)
}
fn transact(
    root: PathBuf,
    request: LivingStoreRequestV2,
    state: &AtomicU8,
    fault: Option<NativeLivingFaultPoint>,
) -> Result<LivingStoreResultV2, LivingStoreErrorV2> {
    if state.load(Ordering::Acquire) == 2 {
        return Err(LivingStoreErrorV2::Cancelled);
    }
    fs::create_dir_all(&root).map_err(storage)?;
    let lock = OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(false)
        .open(root.join("lock"))
        .map_err(storage)?;
    lock.lock().map_err(storage)?;
    if state.load(Ordering::Acquire) == 2 {
        return Err(LivingStoreErrorV2::Cancelled);
    }
    recover_staging(&root)?;
    let (generation, disk, side) = recover(&root)?;
    let mut candidate = disk.materialize(&root, &request)?;
    let read_only = matches!(
        request,
        LivingStoreRequestV2::ListSlots
            | LivingStoreRequestV2::ListPacks
            | LivingStoreRequestV2::ListPackReferences { .. }
            | LivingStoreRequestV2::LoadSlot { .. }
            | LivingStoreRequestV2::ReadPack { .. }
            | LivingStoreRequestV2::ReadArchive { .. }
            | LivingStoreRequestV2::ReadNotebook { .. }
    );
    let list_slots = matches!(request, LivingStoreRequestV2::ListSlots);
    let list_packs = matches!(request, LivingStoreRequestV2::ListPacks);
    if list_slots {
        for (entry, reference) in candidate.slots.iter_mut().zip(&disk.slots) {
            entry.descriptor.diagnostic = reference
                .archives
                .last()
                .and_then(|(_, reference)| payload_diagnostic(&root, reference));
            if entry.descriptor.diagnostic.is_none() {
                entry.descriptor.diagnostic = disk
                    .packs
                    .iter()
                    .find(|pack| pack.descriptor.hash == entry.descriptor.catalog_hash)
                    .and_then(|pack| payload_diagnostic(&root, &pack.reference));
            }
        }
    }
    if list_packs {
        for (entry, reference) in candidate.packs.iter_mut().zip(&disk.packs) {
            entry.descriptor.diagnostic = payload_diagnostic(&root, &reference.reference);
        }
    }
    let result = candidate.execute(request)?;
    if state
        .compare_exchange(0, 1, Ordering::AcqRel, Ordering::Acquire)
        .is_err()
    {
        return Err(LivingStoreErrorV2::Cancelled);
    }
    if !read_only {
        let next = generation.checked_add(1).ok_or(LivingStoreErrorV2::Limit)?;
        let retiring = read_bounded(&root.join(side), MANIFEST_LIMIT)?
            .and_then(|bytes| Manifest::decode(&bytes).ok());
        let mut journal = StagingJournal {
            base_generation: generation,
            references: Vec::new(),
            integrity: [0; 32],
        };
        // Preserve durable ownership of every object in the manifest being replaced.
        // Recovery keeps either valid generation and deletes only retired known refs.
        if let Some(retiring) = &retiring {
            for reference in retiring.payload.packs.iter().map(|p| &p.reference).chain(
                retiring.payload.slots.iter().flat_map(|slot| {
                    slot.archives
                        .iter()
                        .chain(&slot.notebooks)
                        .map(|(_, reference)| reference)
                }),
            ) {
                journal.references.push(reference.clone());
            }
            journal.flush(&root)?;
        }
        let payload = disk.persist_candidate(&root, candidate, &mut journal)?;
        let bytes = serde_json::to_vec(&Manifest::new(next, payload.clone())?)
            .map_err(|_| LivingStoreErrorV2::Integrity)?;
        if bytes.len() > MANIFEST_LIMIT {
            return Err(LivingStoreErrorV2::TooLarge);
        }
        if fault == Some(NativeLivingFaultPoint::BeforeManifest) {
            return Err(LivingStoreErrorV2::Storage(
                "injected before manifest".into(),
            ));
        }
        write_atomic(&root, side, &bytes)?;
        if fault == Some(NativeLivingFaultPoint::BeforeHead) {
            return Err(LivingStoreErrorV2::Storage("injected before head".into()));
        }
        write_atomic(&root, "head", next.to_string().as_bytes())?;
        // The journal survives until the committed head and both manifest references
        // are durable; cleanup never determines the mutation result.
        let _ = recover_staging(&root);
        if let Some(retiring) = retiring {
            // Durability already won. Cleanup failure preserves the confirmed
            // archive and cannot change its committed result.
            let _ = prune_retired(&root, &retiring.payload, &payload, &disk);
        }
    }
    Ok(result)
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum NativeLivingFaultPoint {
    BeforeManifest,
    BeforeHead,
}
struct Job {
    fault: Option<NativeLivingFaultPoint>,
    owner: u64,
    request: Option<LivingStoreRequestV2>,
    state: Arc<AtomicU8>,
    receiver: Option<mpsc::Receiver<LivingStorePollV2>>,
    result: Option<LivingStorePollV2>,
}
pub struct NativeLivingStoreV2 {
    fault: Option<NativeLivingFaultPoint>,
    root: PathBuf,
    jobs: BTreeMap<LivingStoreJobIdV2, Job>,
}
impl NativeLivingStoreV2 {
    /// Deterministic storage failure hook; never mutates authority directly.
    pub fn inject_next_failure(&mut self, point: NativeLivingFaultPoint) {
        self.fault = Some(point);
    }
    pub fn new(application_data_root: impl Into<PathBuf>) -> Self {
        Self {
            fault: None,
            root: application_data_root.into().join("living-v2"),
            jobs: BTreeMap::new(),
        }
    }
}
impl LivingStoreV2 for NativeLivingStoreV2 {
    fn begin(
        &mut self,
        owner_epoch: u64,
        request: LivingStoreRequestV2,
    ) -> Result<LivingStoreJobIdV2, LivingStoreErrorV2> {
        preflight(&request)?;
        if self.jobs.len() >= 16 {
            return Err(LivingStoreErrorV2::Limit);
        }
        let id = LivingStoreJobIdV2(super::allocate_job_id()?);
        self.jobs.insert(
            id,
            Job {
                fault: self.fault.take(),
                owner: owner_epoch,
                request: Some(request),
                state: Arc::new(AtomicU8::new(0)),
                receiver: None,
                result: None,
            },
        );
        Ok(id)
    }
    fn poll(
        &mut self,
        owner_epoch: u64,
        id: LivingStoreJobIdV2,
        work_budget: usize,
    ) -> LivingStorePollV2 {
        let Some(job) = self.jobs.get_mut(&id) else {
            return LivingStorePollV2::Failed(LivingStoreErrorV2::NotFound);
        };
        if job.owner != owner_epoch {
            return LivingStorePollV2::Failed(LivingStoreErrorV2::WrongOwner);
        }
        if work_budget == 0 {
            return LivingStorePollV2::Pending { work_done: 0 };
        }
        if let Some(request) = job.request.take() {
            let root = self.root.clone();
            let state = Arc::clone(&job.state);
            let fault = job.fault;
            let (sender, receiver) = mpsc::channel();
            thread::spawn(move || {
                let outcome = match transact(root, request, &state, fault) {
                    Ok(result) => LivingStorePollV2::Ready(result),
                    Err(error) => LivingStorePollV2::Failed(error),
                };
                let _ = sender.send(outcome);
            });
            job.receiver = Some(receiver);
        }
        if job.result.is_none() {
            job.result = job.receiver.as_ref().and_then(|r| r.try_recv().ok());
        }
        job.result
            .clone()
            .unwrap_or(LivingStorePollV2::Pending { work_done: 0 })
    }
    fn cancel(
        &mut self,
        owner_epoch: u64,
        id: LivingStoreJobIdV2,
    ) -> Result<LivingStoreCancelV2, LivingStoreErrorV2> {
        let Some(job) = self.jobs.get_mut(&id) else {
            return Err(LivingStoreErrorV2::NotFound);
        };
        if job.owner != owner_epoch {
            return Err(LivingStoreErrorV2::WrongOwner);
        }
        if job.result.is_none() {
            job.result = job.receiver.as_ref().and_then(|r| r.try_recv().ok());
        }
        if let Some(LivingStorePollV2::Ready(result)) = &job.result {
            return Ok(LivingStoreCancelV2::CommitAlreadyWon(result.clone()));
        }
        if job.result.is_some() && job.state.load(Ordering::Acquire) == 1 {
            return Ok(LivingStoreCancelV2::CancelPending);
        }
        if job.result.is_some()
            || job
                .state
                .compare_exchange(0, 2, Ordering::AcqRel, Ordering::Acquire)
                .is_ok()
        {
            self.jobs.remove(&id);
            return Ok(LivingStoreCancelV2::Cancelled);
        }
        Ok(LivingStoreCancelV2::CancelPending)
    }
    fn release(&mut self, owner_epoch: u64, id: LivingStoreJobIdV2) -> bool {
        if self
            .jobs
            .get(&id)
            .is_some_and(|j| j.owner == owner_epoch && j.result.is_some())
        {
            self.jobs.remove(&id);
            true
        } else {
            false
        }
    }
}

fn payload_diagnostic(root: &Path, reference: &Reference) -> Option<LivingStoreErrorV2> {
    match fs::metadata(root.join(&reference.file)) {
        Ok(metadata) if metadata.len() == reference.len => None,
        Ok(_) => Some(LivingStoreErrorV2::Integrity),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            Some(LivingStoreErrorV2::NotFound)
        }
        Err(error) => Some(storage(error)),
    }
}
impl Drop for NativeLivingStoreV2 {
    fn drop(&mut self) {
        for job in self.jobs.values() {
            let _ = job
                .state
                .compare_exchange(0, 2, Ordering::AcqRel, Ordering::Acquire);
        }
    }
}

fn prune_retired(
    root: &Path,
    retiring: &DiskState,
    current: &DiskState,
    prior: &DiskState,
) -> Result<(), LivingStoreErrorV2> {
    for file in retired_payloads(retiring, current, prior) {
        match fs::remove_file(root.join(file)) {
            Ok(()) => {}
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(error) => return Err(storage(error)),
        }
    }
    fs::File::open(root)
        .and_then(|file| file.sync_all())
        .map_err(storage)
}

// Private, bounded write-ahead ownership journal. Never infer ownership by scanning
// a directory: only integrity-verified references tracked by this adapter are eligible.
const STAGING_LIMIT: usize = 256 * 1024;
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct StagingJournal {
    base_generation: u64,
    references: Vec<Reference>,
    integrity: [u8; 32],
}
impl StagingJournal {
    fn digest(&self) -> Result<[u8; 32], LivingStoreErrorV2> {
        let mut bytes = b"NYON-LIVING-PRIVATE-STAGING-V1\0".to_vec();
        bytes.extend(self.base_generation.to_le_bytes());
        bytes.extend(
            serde_json::to_vec(&self.references).map_err(|_| LivingStoreErrorV2::Integrity)?,
        );
        Ok(hash(&bytes))
    }
    fn track(&mut self, root: &Path, reference: &Reference) -> Result<(), LivingStoreErrorV2> {
        if self.references.iter().any(|r| r.file == reference.file) {
            return Ok(());
        }
        if self.references.len() >= 1024 {
            return Err(LivingStoreErrorV2::Limit);
        }
        self.references.push(reference.clone());
        self.flush(root)
    }
    fn flush(&mut self, root: &Path) -> Result<(), LivingStoreErrorV2> {
        let mut seen = BTreeSet::new();
        self.references
            .retain(|reference| seen.insert(reference.file.clone()));
        if self.references.len() > 1024 {
            return Err(LivingStoreErrorV2::Limit);
        }
        self.integrity = self.digest()?;
        let bytes = serde_json::to_vec(self).map_err(|_| LivingStoreErrorV2::Integrity)?;
        if bytes.len() > STAGING_LIMIT {
            return Err(LivingStoreErrorV2::TooLarge);
        }
        write_atomic(root, "staging", &bytes)
    }
}
fn recover_staging(root: &Path) -> Result<(), LivingStoreErrorV2> {
    let Some(bytes) = read_bounded(&root.join("staging"), STAGING_LIMIT)? else {
        return Ok(());
    };
    let journal: StagingJournal =
        serde_json::from_slice(&bytes).map_err(|_| LivingStoreErrorV2::CorruptRecovery)?;
    if journal.references.len() > 1024 || journal.digest()? != journal.integrity {
        return Err(LivingStoreErrorV2::CorruptRecovery);
    }
    let mut keep = BTreeSet::new();
    for side in ["generation-a", "generation-b"] {
        if let Some(bytes) = read_bounded(&root.join(side), MANIFEST_LIMIT)? {
            // An ambiguous manifest might reference an object: preserve everything.
            let manifest =
                Manifest::decode(&bytes).map_err(|_| LivingStoreErrorV2::CorruptRecovery)?;
            keep.extend(
                manifest
                    .payload
                    .packs
                    .iter()
                    .map(|p| p.reference.file.clone()),
            );
            keep.extend(manifest.payload.slots.iter().flat_map(|s| {
                s.archives
                    .iter()
                    .chain(&s.notebooks)
                    .map(|(_, r)| r.file.clone())
            }));
        }
    }
    let mut remove = Vec::new();
    let mut seen = BTreeSet::new();
    for reference in &journal.references {
        validate_ref(reference, 32 * 1024 * 1024)?;
        if !seen.insert(&reference.file) {
            return Err(LivingStoreErrorV2::CorruptRecovery);
        }
        if !keep.contains(&reference.file)
            && let Some(bytes) = read_bounded(&root.join(&reference.file), reference.len as usize)?
        {
            if bytes.len() as u64 != reference.len || hash(&bytes) != reference.sha256 {
                return Err(LivingStoreErrorV2::CorruptRecovery);
            }
            remove.push(reference.file.clone());
        }
    }
    // Validate the entire journal before the first unlink.
    for file in remove {
        fs::remove_file(root.join(file)).map_err(storage)?;
    }
    // A crash during the atomic payload write may leave a partial temporary
    // object. Its exact private name was reserved by the durable journal.
    for reference in &journal.references {
        match fs::remove_file(root.join(format!("{}.pending", reference.file))) {
            Ok(()) => {}
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(error) => return Err(storage(error)),
        }
    }
    // Persist reclamation before discarding its ownership record. A power loss
    // between these syncs can repeat cleanup, but cannot forget an orphan.
    fs::File::open(root)
        .and_then(|file| file.sync_all())
        .map_err(storage)?;
    fs::remove_file(root.join("staging")).map_err(storage)?;
    fs::File::open(root)
        .and_then(|file| file.sync_all())
        .map_err(storage)
}

#[cfg(test)]
mod staging_tests {
    use super::*;
    fn root() -> PathBuf {
        static NEXT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
        let root = std::env::temp_dir().join(format!(
            "nyon-staging-{}-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        fs::create_dir_all(&root).unwrap();
        root
    }
    #[test]
    fn interrupted_payload_is_reclaimed_without_directory_scan() {
        let root = root();
        let mut journal = StagingJournal {
            base_generation: 0,
            references: Vec::new(),
            integrity: [0; 32],
        };
        let reference = persist_payload(
            &root,
            b"owned orphan",
            LivingCatalogHashV2([0; 32]),
            &mut journal,
        )
        .unwrap();
        fs::write(root.join("unrelated"), b"preserve").unwrap();
        let pending = root.join(format!("{}.pending", reference.file));
        fs::write(&pending, b"interrupted partial payload").unwrap();
        assert!(root.join(&reference.file).exists());
        recover_staging(&root).unwrap();
        assert!(!root.join(&reference.file).exists());
        assert!(!pending.exists());
        assert_eq!(fs::read(root.join("unrelated")).unwrap(), b"preserve");
        fs::remove_dir_all(root).unwrap();
    }
    #[test]
    fn retired_known_payload_survives_then_reclaims_after_last_manifest_replacement() {
        let root = root();
        let mut journal = StagingJournal {
            base_generation: 0,
            references: Vec::new(),
            integrity: [0; 32],
        };
        let catalog = LivingCatalogHashV2([9; 32]);
        let reference =
            persist_payload(&root, b"retired known pack", catalog, &mut journal).unwrap();
        let old = DiskState {
            packs: vec![DiskPack {
                descriptor: LivingPackDescriptorV2 {
                    diagnostic: None,
                    hash: catalog,
                    archived: true,
                    generation: LivingGenerationV2(2),
                },
                reference: reference.clone(),
            }],
            ..DiskState::default()
        };
        write_atomic(
            &root,
            "generation-a",
            &serde_json::to_vec(&Manifest::new(1, old).unwrap()).unwrap(),
        )
        .unwrap();
        write_atomic(
            &root,
            "generation-b",
            &serde_json::to_vec(&Manifest::new(2, DiskState::default()).unwrap()).unwrap(),
        )
        .unwrap();
        recover_staging(&root).unwrap();
        assert!(
            root.join(&reference.file).exists(),
            "prior recovery still owns pack"
        );
        journal.references.clear();
        journal.track(&root, &reference).unwrap();
        // Simulate durable replacement followed by interruption before cleanup.
        write_atomic(
            &root,
            "generation-a",
            &serde_json::to_vec(&Manifest::new(3, DiskState::default()).unwrap()).unwrap(),
        )
        .unwrap();
        fs::write(root.join("unknown-user-object"), b"preserve").unwrap();
        recover_staging(&root).unwrap();
        assert!(!root.join(&reference.file).exists());
        assert!(root.join("unknown-user-object").exists());
        fs::remove_dir_all(root).unwrap();
    }
    #[test]
    fn stale_journal_preserves_both_recovery_generations() {
        let root = root();
        let mut journal = StagingJournal {
            base_generation: 0,
            references: Vec::new(),
            integrity: [0; 32],
        };
        for (generation, side, bytes) in [
            (7, "generation-a", b"prior".as_slice()),
            (8, "generation-b", b"current".as_slice()),
        ] {
            let catalog = LivingCatalogHashV2([generation as u8; 32]);
            let reference = persist_payload(&root, bytes, catalog, &mut journal).unwrap();
            let state = DiskState {
                packs: vec![DiskPack {
                    descriptor: LivingPackDescriptorV2 {
                        diagnostic: None,
                        hash: catalog,
                        archived: false,
                        generation: LivingGenerationV2(generation),
                    },
                    reference,
                }],
                ..DiskState::default()
            };
            write_atomic(
                &root,
                side,
                &serde_json::to_vec(&Manifest::new(generation, state).unwrap()).unwrap(),
            )
            .unwrap();
        }
        let refs = journal.references.clone();
        recover_staging(&root).unwrap();
        assert!(refs.iter().all(|r| root.join(&r.file).exists()));
        assert!(!root.join("staging").exists());
        fs::remove_dir_all(root).unwrap();
    }
    #[test]
    fn malformed_journal_preserves_tracked_content() {
        let root = root();
        let mut journal = StagingJournal {
            base_generation: 0,
            references: Vec::new(),
            integrity: [0; 32],
        };
        let reference = persist_payload(
            &root,
            b"owned orphan",
            LivingCatalogHashV2([0; 32]),
            &mut journal,
        )
        .unwrap();
        fs::write(root.join("staging"), b"{}").unwrap();
        assert!(matches!(
            recover_staging(&root),
            Err(LivingStoreErrorV2::CorruptRecovery)
        ));
        assert!(root.join(&reference.file).exists());
        fs::remove_dir_all(root).unwrap();
    }
}
