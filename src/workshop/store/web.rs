//! IndexedDB persistence for Galaxy Workshop.
//!
//! Browser writes stage archive bytes and their slot reference in the same
//! read-write transaction. A completed job is published only after IndexedDB
//! reports that transaction committed. Request, transaction, quota, and schema
//! failures therefore leave the previously committed browser state intact.

use super::*;

pub const INDEXED_DB_NAME: &str = "nyon.workshop.v1";
pub const INDEXED_DB_VERSION: u32 = 1;
pub const SLOTS_OBJECT_STORE: &str = "slots";
pub const ARCHIVES_OBJECT_STORE: &str = "archives";
pub const PACKS_OBJECT_STORE: &str = "packs";

/// Failure modes used by the deterministic transaction conformance model.
///
/// The model stages a request against a cloned committed state, then either
/// publishes the candidate or injects one of these terminal failures. It lets
/// native tests prove the state-preservation contract that the wasm adapter
/// implements with real IndexedDB transactions.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum IndexedDbModelFailure {
    Abort,
    QuotaDenied,
    Unavailable,
    SchemaMismatch,
    Evicted,
}

impl IndexedDbModelFailure {
    fn error(self) -> WorkshopStoreError {
        match self {
            Self::Abort => WorkshopStoreError::IndexedDbTransactionAborted,
            Self::QuotaDenied => WorkshopStoreError::IndexedDbQuotaDenied,
            Self::Unavailable => WorkshopStoreError::IndexedDbUnavailable,
            Self::SchemaMismatch => WorkshopStoreError::IndexedDbSchemaMismatch,
            Self::Evicted => WorkshopStoreError::IndexedDbEvicted,
        }
    }
}

/// Pure deterministic conformance model for IndexedDB transaction publication.
///
/// This is not a second persistence implementation. It is a host-testable
/// oracle for the important transaction boundary: mutations become visible
/// only after the transaction succeeds.
#[derive(Clone, Default)]
pub struct IndexedDbTransactionModel {
    jobs: JobTable,
    committed: MemoryWorkshopStore,
    next_failure: Option<IndexedDbModelFailure>,
}

impl IndexedDbTransactionModel {
    pub fn inject_next_failure(&mut self, failure: IndexedDbModelFailure) {
        self.next_failure = Some(failure);
    }
}

impl WorkshopStore for IndexedDbTransactionModel {
    fn start(&mut self, request: WorkshopStoreRequest) -> Result<StoreJobId, WorkshopStoreError> {
        let job = self.jobs.reserve(request.class())?;
        let mut candidate = self.committed.clone();
        let result = candidate.execute(request).and_then(|result| {
            if let Some(failure) = self.next_failure.take() {
                return Err(failure.error());
            }
            self.committed = candidate;
            Ok(result)
        });
        self.jobs.finish(job, result);
        Ok(job)
    }

    fn poll(&mut self, job: StoreJobId) -> StoreJobState {
        self.jobs.poll(job)
    }
}

#[cfg(target_arch = "wasm32")]
mod wasm {
    use std::{cell::RefCell, collections::BTreeMap, rc::Rc};

    use js_sys::{Array, Function, JsString, Promise, Uint8Array};
    use sha2::{Digest, Sha256};
    use wasm_bindgen::{JsCast, JsValue, closure::Closure};
    use wasm_bindgen_futures::JsFuture;
    use web_sys::{
        Event, IdbDatabase, IdbObjectStore, IdbOpenDbRequest, IdbRequest, IdbTransaction,
        IdbTransactionMode,
    };

    use super::*;

    const REFERENCE_FORMAT: &str = "nyon-workshop-indexeddb";
    const REFERENCE_VERSION: u32 = 1;
    const METADATA_KEY: &str = "metadata";
    const SCHEMA_SENTINEL: &str = "nyon-indexeddb-schema-mismatch";
    const BLOCKED_SENTINEL: &str = "nyon-indexeddb-open-blocked";
    const MAX_REFERENCE_RECORD_BYTES: usize = 4 * 1024;

    #[derive(Clone, Debug, serde::Deserialize, serde::Serialize, Eq, PartialEq)]
    #[serde(deny_unknown_fields)]
    struct GenerationReference {
        generation: SaveGeneration,
        byte_len: u64,
        sha256: [u8; 32],
    }

    #[derive(Clone, Debug, serde::Deserialize, serde::Serialize)]
    #[serde(tag = "record_type", rename_all = "snake_case", deny_unknown_fields)]
    enum ReferenceRecord {
        Metadata {
            format: String,
            version: u32,
            selected_continue: Option<SlotId>,
        },
        Slot {
            id: SlotId,
            name: SlotName,
            archived: bool,
            generations: Vec<GenerationReference>,
        },
    }

    #[derive(Clone, Debug)]
    struct SlotReference {
        name: SlotName,
        archived: bool,
        generations: Vec<GenerationReference>,
    }

    #[derive(Clone, Debug, Default)]
    struct ReferenceState {
        has_metadata: bool,
        selected_continue: Option<SlotId>,
        slots: BTreeMap<SlotId, SlotReference>,
    }

    impl ReferenceState {
        fn decode(value: JsValue) -> Result<Self, WorkshopStoreError> {
            if !Array::is_array(&value) {
                return Err(WorkshopStoreError::IndexedDbSchemaMismatch);
            }
            let values = Array::from(&value);
            if values.length() as usize > MAX_WORKSHOP_SLOTS + 1 {
                return Err(WorkshopStoreError::IndexedDbSchemaMismatch);
            }

            let mut state = Self::default();
            for value in values.iter() {
                let text: JsString = value
                    .dyn_into()
                    .map_err(|_| WorkshopStoreError::IndexedDbSchemaMismatch)?;
                let length = usize::try_from(text.length())
                    .map_err(|_| WorkshopStoreError::IndexedDbSchemaMismatch)?;
                if length > MAX_REFERENCE_RECORD_BYTES {
                    return Err(WorkshopStoreError::IndexedDbSchemaMismatch);
                }
                let text = text
                    .as_string()
                    .ok_or(WorkshopStoreError::IndexedDbSchemaMismatch)?;
                let record: ReferenceRecord = serde_json::from_str(&text)
                    .map_err(|_| WorkshopStoreError::IndexedDbSchemaMismatch)?;
                match record {
                    ReferenceRecord::Metadata {
                        format,
                        version,
                        selected_continue,
                    } => {
                        if state.has_metadata
                            || format != REFERENCE_FORMAT
                            || version != REFERENCE_VERSION
                        {
                            return Err(WorkshopStoreError::IndexedDbSchemaMismatch);
                        }
                        state.has_metadata = true;
                        state.selected_continue = selected_continue;
                    }
                    ReferenceRecord::Slot {
                        id,
                        name,
                        archived,
                        generations,
                    } => {
                        if state
                            .slots
                            .insert(
                                id,
                                SlotReference {
                                    name,
                                    archived,
                                    generations,
                                },
                            )
                            .is_some()
                        {
                            return Err(WorkshopStoreError::IndexedDbSchemaMismatch);
                        }
                    }
                }
            }
            state.validate()?;
            Ok(state)
        }

        fn validate(&self) -> Result<(), WorkshopStoreError> {
            if (!self.slots.is_empty() && !self.has_metadata)
                || self.slots.len() > MAX_WORKSHOP_SLOTS
            {
                return Err(WorkshopStoreError::IndexedDbSchemaMismatch);
            }
            for (id, slot) in &self.slots {
                let id = usize::try_from(id.0)
                    .map_err(|_| WorkshopStoreError::IndexedDbSchemaMismatch)?;
                if id >= MAX_WORKSHOP_SLOTS
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
                    return Err(WorkshopStoreError::IndexedDbSchemaMismatch);
                }
            }
            if let Some(selected) = self.selected_continue {
                let Some(slot) = self.slots.get(&selected) else {
                    return Err(WorkshopStoreError::IndexedDbSchemaMismatch);
                };
                if slot.archived {
                    return Err(WorkshopStoreError::IndexedDbSchemaMismatch);
                }
            }
            Ok(())
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
                            .expect("validated IndexedDB slot has a generation");
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
    }

    /// Browser implementation of [`WorkshopStore`] backed by IndexedDB.
    ///
    /// Every job opens the versioned database independently. This avoids
    /// retaining a stale handle across a browser version-change event while
    /// preserving the store's single commit and single load/import lanes.
    pub struct IndexedDbWorkshopStore {
        jobs: Rc<RefCell<JobTable>>,
    }

    impl Default for IndexedDbWorkshopStore {
        fn default() -> Self {
            Self::new()
        }
    }

    impl IndexedDbWorkshopStore {
        pub fn new() -> Self {
            Self {
                jobs: Rc::new(RefCell::new(JobTable::default())),
            }
        }
    }

    impl WorkshopStore for IndexedDbWorkshopStore {
        fn start(
            &mut self,
            request: WorkshopStoreRequest,
        ) -> Result<StoreJobId, WorkshopStoreError> {
            let job = self.jobs.borrow_mut().reserve(request.class())?;
            let jobs = Rc::clone(&self.jobs);
            wasm_bindgen_futures::spawn_local(async move {
                let result = execute(request).await;
                jobs.borrow_mut().finish(job, result);
            });
            Ok(job)
        }

        fn poll(&mut self, job: StoreJobId) -> StoreJobState {
            self.jobs.borrow_mut().poll(job)
        }
    }

    async fn execute(
        request: WorkshopStoreRequest,
    ) -> Result<WorkshopStoreResult, WorkshopStoreError> {
        match &request {
            WorkshopStoreRequest::CreateSlot { archive, .. }
            | WorkshopStoreRequest::CommitSlot { archive, .. }
            | WorkshopStoreRequest::PromoteRecoveredSlot { archive, .. } => {
                validate_archive(archive)?
            }
            WorkshopStoreRequest::PutPack { canonical_pack } => {
                let _ = validate_pack(canonical_pack)?;
            }
            _ => {}
        }

        let database = open_database().await?;
        execute_with_database(&database.database, request).await
    }

    async fn execute_with_database(
        database: &IdbDatabase,
        request: WorkshopStoreRequest,
    ) -> Result<WorkshopStoreResult, WorkshopStoreError> {
        match request {
            WorkshopStoreRequest::ListSlots => list_slots(database).await,
            WorkshopStoreRequest::CreateSlot { name, archive } => {
                create_slot(database, name, archive).await
            }
            WorkshopStoreRequest::LoadSlot { slot } => load_slot(database, slot).await,
            WorkshopStoreRequest::LoadPreviousGeneration {
                slot,
                expected_head_generation,
            } => load_previous_generation(database, slot, expected_head_generation).await,
            WorkshopStoreRequest::CommitSlot {
                slot,
                expected_generation,
                archive,
            } => commit_slot(database, slot, expected_generation, archive).await,
            WorkshopStoreRequest::PromoteRecoveredSlot {
                slot,
                expected_head_generation,
                recovered_generation,
                archive,
            } => {
                promote_recovered_slot(
                    database,
                    slot,
                    expected_head_generation,
                    recovered_generation,
                    archive,
                )
                .await
            }
            WorkshopStoreRequest::RenameSlot { slot, name } => {
                update_slot_metadata(database, MetadataMutation::Rename { slot, name }).await
            }
            WorkshopStoreRequest::ArchiveSlot { slot } => {
                update_slot_metadata(database, MetadataMutation::Archive { slot }).await
            }
            WorkshopStoreRequest::SelectContinue { slot } => {
                update_slot_metadata(database, MetadataMutation::Select { slot }).await
            }
            WorkshopStoreRequest::PutPack { canonical_pack } => {
                put_pack(database, canonical_pack).await
            }
            WorkshopStoreRequest::GetPack { hash } => get_pack(database, hash).await,
            WorkshopStoreRequest::ListPacks => list_packs(database).await,
        }
    }

    async fn list_slots(database: &IdbDatabase) -> Result<WorkshopStoreResult, WorkshopStoreError> {
        Ok(WorkshopStoreResult::Slots(
            read_reference_state(database).await?.slot_list(),
        ))
    }

    async fn create_slot(
        database: &IdbDatabase,
        name: SlotName,
        archive: Box<[u8]>,
    ) -> Result<WorkshopStoreResult, WorkshopStoreError> {
        mutate_references(
            database,
            true,
            move |references, slots_store, archives_store| {
                if references.slots.len() >= MAX_WORKSHOP_SLOTS {
                    return Err(WorkshopStoreError::SlotCapacity {
                        max_slots: MAX_WORKSHOP_SLOTS,
                    });
                }
                let slot_limit = u64::try_from(MAX_WORKSHOP_SLOTS)
                    .expect("Workshop slot limit is representable as u64");
                let slot = (0..slot_limit)
                    .map(SlotId)
                    .find(|id| !references.slots.contains_key(id))
                    .expect("slot count below capacity has a free identifier");
                let generation = SaveGeneration(1);
                let descriptor = generation_reference(generation, &archive);
                put_bytes_sync(
                    archives_store.expect("archive transaction includes archive store"),
                    &archive_key(slot, generation),
                    &archive,
                )?;
                references.slots.insert(
                    slot,
                    SlotReference {
                        name,
                        archived: false,
                        generations: vec![descriptor],
                    },
                );
                write_slot_sync(
                    slots_store,
                    slot,
                    references
                        .slots
                        .get(&slot)
                        .expect("inserted slot remains present"),
                )?;
                write_metadata_sync(slots_store, references.selected_continue)?;
                Ok(WorkshopStoreResult::SlotCreated { slot, generation })
            },
        )
        .await
    }

    async fn commit_slot(
        database: &IdbDatabase,
        slot: SlotId,
        expected_generation: SaveGeneration,
        archive: Box<[u8]>,
    ) -> Result<WorkshopStoreResult, WorkshopStoreError> {
        // Validate the recoverable predecessor before opening the write
        // transaction. The write transaction repeats the generation CAS and
        // verifies that this descriptor still belongs to that same head.
        let previous = first_valid_generation(database, slot).await?;
        mutate_references(
            database,
            true,
            move |references, slots_store, archives_store| {
                let current = references
                    .slots
                    .get(&slot)
                    .ok_or(WorkshopStoreError::UnknownSlot { slot })?;
                if current.archived {
                    return Err(WorkshopStoreError::ArchivedSlot { slot });
                }
                let actual = current
                    .generations
                    .first()
                    .expect("validated IndexedDB slot has a generation")
                    .generation;
                if actual != expected_generation {
                    return Err(WorkshopStoreError::StaleGeneration {
                        slot,
                        expected: expected_generation,
                        actual,
                    });
                }
                if previous.as_ref().is_some_and(|descriptor| {
                    !current
                        .generations
                        .iter()
                        .any(|current| current == descriptor)
                }) {
                    return Err(WorkshopStoreError::IndexedDbSchemaMismatch);
                }
                let generation = SaveGeneration(
                    actual
                        .0
                        .checked_add(1)
                        .ok_or(WorkshopStoreError::GenerationExhausted)?,
                );
                let descriptor = generation_reference(generation, &archive);
                put_bytes_sync(
                    archives_store.expect("archive transaction includes archive store"),
                    &archive_key(slot, generation),
                    &archive,
                )?;
                let current = references
                    .slots
                    .get_mut(&slot)
                    .expect("validated IndexedDB slot remains present");
                current.generations.clear();
                current.generations.push(descriptor);
                if let Some(previous) = previous {
                    current.generations.push(previous);
                }
                write_slot_sync(slots_store, slot, current)?;
                Ok(WorkshopStoreResult::SlotCommitted { slot, generation })
            },
        )
        .await
    }

    async fn promote_recovered_slot(
        database: &IdbDatabase,
        slot: SlotId,
        expected_head_generation: SaveGeneration,
        recovered_generation: SaveGeneration,
        archive: Box<[u8]>,
    ) -> Result<WorkshopStoreResult, WorkshopStoreError> {
        let references = read_reference_state(database).await?;
        let reference = references
            .slots
            .get(&slot)
            .ok_or(WorkshopStoreError::UnknownSlot { slot })?;
        let predecessor = reference
            .generations
            .iter()
            .find(|descriptor| descriptor.generation == recovered_generation)
            .cloned()
            .ok_or(WorkshopStoreError::NoValidGeneration { slot })?;
        match read_generation_from_database(database, slot, &predecessor).await? {
            GenerationRead::Valid(_) => {}
            GenerationRead::Missing | GenerationRead::Invalid => {
                return Err(WorkshopStoreError::NoValidGeneration { slot });
            }
        }
        mutate_references(
            database,
            true,
            move |references, slots_store, archives_store| {
                let current = references
                    .slots
                    .get(&slot)
                    .ok_or(WorkshopStoreError::UnknownSlot { slot })?;
                if current.archived {
                    return Err(WorkshopStoreError::ArchivedSlot { slot });
                }
                let actual = current
                    .generations
                    .first()
                    .expect("validated IndexedDB slot has a generation")
                    .generation;
                if actual != expected_head_generation {
                    return Err(WorkshopStoreError::StaleGeneration {
                        slot,
                        expected: expected_head_generation,
                        actual,
                    });
                }
                if !current
                    .generations
                    .iter()
                    .any(|descriptor| descriptor == &predecessor)
                {
                    return Err(WorkshopStoreError::IndexedDbSchemaMismatch);
                }
                let generation = SaveGeneration(
                    actual
                        .0
                        .checked_add(1)
                        .ok_or(WorkshopStoreError::GenerationExhausted)?,
                );
                let descriptor = generation_reference(generation, &archive);
                put_bytes_sync(
                    archives_store.expect("archive transaction includes archive store"),
                    &archive_key(slot, generation),
                    &archive,
                )?;
                let current = references
                    .slots
                    .get_mut(&slot)
                    .expect("validated IndexedDB slot remains present");
                current.generations = vec![descriptor, predecessor];
                write_slot_sync(slots_store, slot, current)?;
                Ok(WorkshopStoreResult::SlotCommitted { slot, generation })
            },
        )
        .await
    }

    async fn load_slot(
        database: &IdbDatabase,
        slot: SlotId,
    ) -> Result<WorkshopStoreResult, WorkshopStoreError> {
        let references = read_reference_state(database).await?;
        let reference = references
            .slots
            .get(&slot)
            .ok_or(WorkshopStoreError::UnknownSlot { slot })?;
        let head_generation = reference
            .generations
            .first()
            .expect("validated IndexedDB slot has a generation")
            .generation;
        let mut saw_eviction = false;
        for (index, descriptor) in reference.generations.iter().enumerate() {
            match read_generation_from_database(database, slot, descriptor).await? {
                GenerationRead::Valid(archive) => {
                    return Ok(WorkshopStoreResult::SlotLoaded(LoadedSlot {
                        slot,
                        name: reference.name.clone(),
                        generation: descriptor.generation,
                        head_generation,
                        archive,
                        recovered_from_previous: index > 0,
                    }));
                }
                GenerationRead::Missing => saw_eviction = true,
                GenerationRead::Invalid => {}
            }
        }
        if saw_eviction {
            Err(WorkshopStoreError::IndexedDbEvicted)
        } else {
            Err(WorkshopStoreError::NoValidGeneration { slot })
        }
    }

    async fn load_previous_generation(
        database: &IdbDatabase,
        slot: SlotId,
        expected_head_generation: SaveGeneration,
    ) -> Result<WorkshopStoreResult, WorkshopStoreError> {
        let references = read_reference_state(database).await?;
        let reference = references
            .slots
            .get(&slot)
            .ok_or(WorkshopStoreError::UnknownSlot { slot })?;
        let actual = reference
            .generations
            .first()
            .expect("validated IndexedDB slot has a generation")
            .generation;
        if actual != expected_head_generation {
            return Err(WorkshopStoreError::StaleGeneration {
                slot,
                expected: expected_head_generation,
                actual,
            });
        }
        let descriptor = reference
            .generations
            .get(1)
            .ok_or(WorkshopStoreError::NoValidGeneration { slot })?;
        let archive = match read_generation_from_database(database, slot, descriptor).await? {
            GenerationRead::Valid(archive) => archive,
            GenerationRead::Missing => return Err(WorkshopStoreError::IndexedDbEvicted),
            GenerationRead::Invalid => {
                return Err(WorkshopStoreError::NoValidGeneration { slot });
            }
        };
        Ok(WorkshopStoreResult::SlotLoaded(LoadedSlot {
            slot,
            name: reference.name.clone(),
            generation: descriptor.generation,
            head_generation: actual,
            archive,
            recovered_from_previous: true,
        }))
    }

    enum MetadataMutation {
        Rename { slot: SlotId, name: SlotName },
        Archive { slot: SlotId },
        Select { slot: SlotId },
    }

    async fn update_slot_metadata(
        database: &IdbDatabase,
        mutation: MetadataMutation,
    ) -> Result<WorkshopStoreResult, WorkshopStoreError> {
        mutate_references(
            database,
            false,
            move |references, store, _archives_store| match mutation {
                MetadataMutation::Rename { slot, name } => {
                    let reference = references
                        .slots
                        .get_mut(&slot)
                        .ok_or(WorkshopStoreError::UnknownSlot { slot })?;
                    reference.name = name;
                    write_slot_sync(store, slot, reference)?;
                    Ok(WorkshopStoreResult::SlotRenamed { slot })
                }
                MetadataMutation::Archive { slot } => {
                    let reference = references
                        .slots
                        .get_mut(&slot)
                        .ok_or(WorkshopStoreError::UnknownSlot { slot })?;
                    reference.archived = true;
                    write_slot_sync(store, slot, reference)?;
                    if references.selected_continue == Some(slot) {
                        references.selected_continue = None;
                        write_metadata_sync(store, None)?;
                    }
                    Ok(WorkshopStoreResult::SlotArchived { slot })
                }
                MetadataMutation::Select { slot } => {
                    let reference = references
                        .slots
                        .get(&slot)
                        .ok_or(WorkshopStoreError::UnknownSlot { slot })?;
                    if reference.archived {
                        return Err(WorkshopStoreError::ArchivedSlot { slot });
                    }
                    write_metadata_sync(store, Some(slot))?;
                    Ok(WorkshopStoreResult::ContinueSelected { slot })
                }
            },
        )
        .await
    }

    async fn put_pack(
        database: &IdbDatabase,
        bytes: Box<[u8]>,
    ) -> Result<WorkshopStoreResult, WorkshopStoreError> {
        let (hash, canonical_pack) = validate_pack(&bytes)?;
        let transaction = transaction(
            database,
            &[PACKS_OBJECT_STORE],
            IdbTransactionMode::Readwrite,
        )?;
        let waiter = TransactionWaiter::new(&transaction);
        let result = async {
            let store = object_store(&transaction, PACKS_OBJECT_STORE)?;
            let hashes = read_pack_hashes(&store).await?;
            if !hashes.contains(&hash) && hashes.len() >= MAX_WORKSHOP_PACKS {
                return Err(WorkshopStoreError::PackCapacity {
                    max_packs: MAX_WORKSHOP_PACKS,
                });
            }
            put_bytes(&store, &catalog_key(hash), &canonical_pack).await?;
            Ok(WorkshopStoreResult::PackStored { hash })
        }
        .await;
        finish_transaction(&transaction, waiter, result).await
    }

    async fn get_pack(
        database: &IdbDatabase,
        hash: CatalogHash,
    ) -> Result<WorkshopStoreResult, WorkshopStoreError> {
        let transaction = transaction(
            database,
            &[PACKS_OBJECT_STORE],
            IdbTransactionMode::Readonly,
        )?;
        let waiter = TransactionWaiter::new(&transaction);
        let result = async {
            let store = object_store(&transaction, PACKS_OBJECT_STORE)?;
            let value = request_result(
                store
                    .get(&JsValue::from_str(&catalog_key(hash)))
                    .map_err(map_js_error)?,
            )
            .await?;
            if value.is_undefined() {
                return Err(WorkshopStoreError::IndexedDbEvicted);
            }
            let bytes = js_bytes(value, MAX_WORKSHOP_PACK_BYTES)
                .map_err(|_| WorkshopStoreError::CorruptPack)?;
            let (actual, canonical_pack) = validate_pack(&bytes)?;
            if actual != hash {
                return Err(WorkshopStoreError::CorruptPack);
            }
            Ok(WorkshopStoreResult::PackLoaded {
                hash,
                canonical_pack,
            })
        }
        .await;
        finish_transaction(&transaction, waiter, result).await
    }

    async fn list_packs(database: &IdbDatabase) -> Result<WorkshopStoreResult, WorkshopStoreError> {
        let transaction = transaction(
            database,
            &[PACKS_OBJECT_STORE],
            IdbTransactionMode::Readonly,
        )?;
        let waiter = TransactionWaiter::new(&transaction);
        let result = async {
            let store = object_store(&transaction, PACKS_OBJECT_STORE)?;
            Ok(WorkshopStoreResult::Packs(read_pack_hashes(&store).await?))
        }
        .await;
        finish_transaction(&transaction, waiter, result).await
    }

    async fn read_references(store: &IdbObjectStore) -> Result<ReferenceState, WorkshopStoreError> {
        let request = store
            .get_all_with_key_and_limit(
                &JsValue::UNDEFINED,
                u32::try_from(MAX_WORKSHOP_SLOTS + 2)
                    .expect("Workshop slot query limit fits in u32"),
            )
            .map_err(map_js_error)?;
        ReferenceState::decode(request_result(request).await?)
    }

    async fn read_pack_hashes(
        store: &IdbObjectStore,
    ) -> Result<Vec<CatalogHash>, WorkshopStoreError> {
        let value = request_result(
            store
                .get_all_keys_with_key_and_limit(
                    &JsValue::UNDEFINED,
                    u32::try_from(MAX_WORKSHOP_PACKS + 1)
                        .expect("Workshop pack query limit fits in u32"),
                )
                .map_err(map_js_error)?,
        )
        .await?;
        if !Array::is_array(&value) {
            return Err(WorkshopStoreError::IndexedDbSchemaMismatch);
        }
        let keys = Array::from(&value);
        if keys.length() as usize > MAX_WORKSHOP_PACKS {
            return Err(WorkshopStoreError::IndexedDbSchemaMismatch);
        }
        let mut hashes = Vec::with_capacity(keys.length() as usize);
        for key in keys.iter() {
            let key: JsString = key
                .dyn_into()
                .map_err(|_| WorkshopStoreError::IndexedDbSchemaMismatch)?;
            if key.length() != 64 {
                return Err(WorkshopStoreError::IndexedDbSchemaMismatch);
            }
            let key = key
                .as_string()
                .ok_or(WorkshopStoreError::IndexedDbSchemaMismatch)?;
            hashes.push(parse_catalog_key(&key)?);
        }
        hashes.sort();
        if !hashes.windows(2).all(|pair| pair[0] < pair[1]) {
            return Err(WorkshopStoreError::IndexedDbSchemaMismatch);
        }
        Ok(hashes)
    }

    async fn read_reference_state(
        database: &IdbDatabase,
    ) -> Result<ReferenceState, WorkshopStoreError> {
        let transaction = transaction(
            database,
            &[SLOTS_OBJECT_STORE],
            IdbTransactionMode::Readonly,
        )?;
        let waiter = TransactionWaiter::new(&transaction);
        let result = async {
            let store = object_store(&transaction, SLOTS_OBJECT_STORE)?;
            read_references(&store).await
        }
        .await;
        finish_transaction(&transaction, waiter, result).await
    }

    fn write_metadata_sync(
        store: &IdbObjectStore,
        selected_continue: Option<SlotId>,
    ) -> Result<(), WorkshopStoreError> {
        let record = ReferenceRecord::Metadata {
            format: REFERENCE_FORMAT.to_owned(),
            version: REFERENCE_VERSION,
            selected_continue,
        };
        put_json_sync(store, METADATA_KEY, &record)
    }

    fn write_slot_sync(
        store: &IdbObjectStore,
        id: SlotId,
        slot: &SlotReference,
    ) -> Result<(), WorkshopStoreError> {
        let record = ReferenceRecord::Slot {
            id,
            name: slot.name.clone(),
            archived: slot.archived,
            generations: slot.generations.clone(),
        };
        put_json_sync(store, &slot_key(id), &record)
    }

    fn put_json_sync<T: serde::Serialize>(
        store: &IdbObjectStore,
        key: &str,
        value: &T,
    ) -> Result<(), WorkshopStoreError> {
        let value = serde_json::to_string(value)
            .map_err(|_| WorkshopStoreError::IndexedDbSchemaMismatch)?;
        store
            .put_with_key(&JsValue::from_str(&value), &JsValue::from_str(key))
            .map_err(map_js_error)?;
        Ok(())
    }

    fn put_bytes_sync(
        store: &IdbObjectStore,
        key: &str,
        bytes: &[u8],
    ) -> Result<(), WorkshopStoreError> {
        let value = Uint8Array::from(bytes);
        store
            .put_with_key(value.as_ref(), &JsValue::from_str(key))
            .map_err(map_js_error)?;
        Ok(())
    }

    async fn put_bytes(
        store: &IdbObjectStore,
        key: &str,
        bytes: &[u8],
    ) -> Result<(), WorkshopStoreError> {
        let value = Uint8Array::from(bytes);
        let request = store
            .put_with_key(value.as_ref(), &JsValue::from_str(key))
            .map_err(map_js_error)?;
        request_result(request).await?;
        Ok(())
    }

    fn generation_reference(generation: SaveGeneration, archive: &[u8]) -> GenerationReference {
        GenerationReference {
            generation,
            byte_len: archive.len() as u64,
            sha256: Sha256::digest(archive).into(),
        }
    }

    async fn first_valid_generation(
        database: &IdbDatabase,
        slot: SlotId,
    ) -> Result<Option<GenerationReference>, WorkshopStoreError> {
        let references = read_reference_state(database).await?;
        let reference = references
            .slots
            .get(&slot)
            .ok_or(WorkshopStoreError::UnknownSlot { slot })?;
        for descriptor in &reference.generations {
            if matches!(
                read_generation_from_database(database, slot, descriptor).await?,
                GenerationRead::Valid(_)
            ) {
                return Ok(Some(descriptor.clone()));
            }
        }
        Ok(None)
    }

    enum GenerationRead {
        Valid(Box<[u8]>),
        Missing,
        Invalid,
    }

    async fn read_generation(
        store: &IdbObjectStore,
        slot: SlotId,
        descriptor: &GenerationReference,
    ) -> Result<GenerationRead, WorkshopStoreError> {
        let key = archive_key(slot, descriptor.generation);
        let value =
            request_result(store.get(&JsValue::from_str(&key)).map_err(map_js_error)?).await?;
        if value.is_undefined() {
            return Ok(GenerationRead::Missing);
        }
        let Ok(bytes) = js_bytes(value, MAX_WORKSHOP_ARCHIVE_BYTES) else {
            return Ok(GenerationRead::Invalid);
        };
        if bytes.len() as u64 != descriptor.byte_len
            || <[u8; 32]>::from(Sha256::digest(&bytes)) != descriptor.sha256
            || validate_archive(&bytes).is_err()
        {
            return Ok(GenerationRead::Invalid);
        }
        Ok(GenerationRead::Valid(bytes.into_boxed_slice()))
    }

    async fn read_generation_from_database(
        database: &IdbDatabase,
        slot: SlotId,
        descriptor: &GenerationReference,
    ) -> Result<GenerationRead, WorkshopStoreError> {
        let transaction = transaction(
            database,
            &[ARCHIVES_OBJECT_STORE],
            IdbTransactionMode::Readonly,
        )?;
        let waiter = TransactionWaiter::new(&transaction);
        let result = async {
            let store = object_store(&transaction, ARCHIVES_OBJECT_STORE)?;
            read_generation(&store, slot, descriptor).await
        }
        .await;
        finish_transaction(&transaction, waiter, result).await
    }

    fn js_bytes(value: JsValue, maximum: usize) -> Result<Vec<u8>, ()> {
        let bytes: Uint8Array = value.dyn_into().map_err(|_| ())?;
        let length = usize::try_from(bytes.length()).map_err(|_| ())?;
        if length > maximum {
            return Err(());
        }
        Ok(bytes.to_vec())
    }

    fn slot_key(slot: SlotId) -> String {
        format!("slot:{:02}", slot.0)
    }

    fn archive_key(slot: SlotId, generation: SaveGeneration) -> String {
        format!("slot:{:02}:generation:{:020}", slot.0, generation.0)
    }

    fn catalog_key(hash: CatalogHash) -> String {
        hex(&hash.0)
    }

    fn parse_catalog_key(value: &str) -> Result<CatalogHash, WorkshopStoreError> {
        if value.len() != 64 || !value.bytes().all(|byte| byte.is_ascii_hexdigit()) {
            return Err(WorkshopStoreError::IndexedDbSchemaMismatch);
        }
        let mut bytes = [0u8; 32];
        for (index, pair) in value.as_bytes().as_chunks::<2>().0.iter().enumerate() {
            bytes[index] = (hex_nibble(pair[0])? << 4) | hex_nibble(pair[1])?;
        }
        Ok(CatalogHash(bytes))
    }

    fn hex_nibble(value: u8) -> Result<u8, WorkshopStoreError> {
        match value {
            b'0'..=b'9' => Ok(value - b'0'),
            b'a'..=b'f' => Ok(value - b'a' + 10),
            b'A'..=b'F' => Ok(value - b'A' + 10),
            _ => Err(WorkshopStoreError::IndexedDbSchemaMismatch),
        }
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

    fn object_store(
        transaction: &IdbTransaction,
        name: &str,
    ) -> Result<IdbObjectStore, WorkshopStoreError> {
        transaction.object_store(name).map_err(map_js_error)
    }

    fn transaction(
        database: &IdbDatabase,
        stores: &[&str],
        mode: IdbTransactionMode,
    ) -> Result<IdbTransaction, WorkshopStoreError> {
        let names = Array::new();
        for name in stores {
            names.push(&JsValue::from_str(name));
        }
        database
            .transaction_with_str_sequence_and_mode(names.as_ref(), mode)
            .map_err(map_js_error)
    }

    async fn mutate_references<F>(
        database: &IdbDatabase,
        include_archives: bool,
        mutation: F,
    ) -> Result<WorkshopStoreResult, WorkshopStoreError>
    where
        F: FnOnce(
                &mut ReferenceState,
                &IdbObjectStore,
                Option<&IdbObjectStore>,
            ) -> Result<WorkshopStoreResult, WorkshopStoreError>
            + 'static,
    {
        let store_names: &[&str] = if include_archives {
            &[SLOTS_OBJECT_STORE, ARCHIVES_OBJECT_STORE]
        } else {
            &[SLOTS_OBJECT_STORE]
        };
        let transaction = transaction(database, store_names, IdbTransactionMode::Readwrite)?;
        let waiter = TransactionWaiter::new(&transaction);
        let slots_store = object_store(&transaction, SLOTS_OBJECT_STORE)?;
        let archives_store = include_archives
            .then(|| object_store(&transaction, ARCHIVES_OBJECT_STORE))
            .transpose()?;
        let request = slots_store
            .get_all_with_key_and_limit(
                &JsValue::UNDEFINED,
                u32::try_from(MAX_WORKSHOP_SLOTS + 2)
                    .expect("Workshop slot query limit fits in u32"),
            )
            .map_err(map_js_error)?;
        let output = Rc::new(RefCell::new(None));
        let mutation = Rc::new(RefCell::new(Some(mutation)));

        let callback_request = request.clone();
        let callback_transaction = transaction.clone();
        let callback_slots = slots_store.clone();
        let callback_archives = archives_store.clone();
        let callback_output = Rc::clone(&output);
        let callback_mutation = Rc::clone(&mutation);
        let callback = Closure::wrap(Box::new(move |_event: Event| {
            let result = callback_request
                .result()
                .map_err(map_js_error)
                .and_then(ReferenceState::decode)
                .and_then(|mut references| {
                    let mutation = callback_mutation
                        .borrow_mut()
                        .take()
                        .ok_or(WorkshopStoreError::IndexedDbTransactionAborted)?;
                    mutation(&mut references, &callback_slots, callback_archives.as_ref())
                });
            if result.is_err() {
                let _ = callback_transaction.abort();
            }
            *callback_output.borrow_mut() = Some(result);
        }) as Box<dyn FnMut(_)>);
        request.set_onsuccess(Some(callback.as_ref().unchecked_ref()));

        let transaction_result = waiter.wait().await;
        request.set_onsuccess(None);
        drop(callback);
        let output = output.borrow_mut().take();
        match output {
            Some(Ok(result)) => {
                transaction_result?;
                Ok(result)
            }
            Some(Err(error)) => Err(error),
            None => {
                transaction_result?;
                Err(WorkshopStoreError::IndexedDbTransactionAborted)
            }
        }
    }

    async fn finish_transaction<T>(
        transaction: &IdbTransaction,
        waiter: TransactionWaiter,
        result: Result<T, WorkshopStoreError>,
    ) -> Result<T, WorkshopStoreError> {
        match result {
            Ok(value) => {
                waiter.wait().await?;
                Ok(value)
            }
            Err(error) => {
                let _ = transaction.abort();
                let _ = waiter.wait().await;
                Err(error)
            }
        }
    }

    struct EventHandlers {
        success: Option<Closure<dyn FnMut(Event)>>,
        error: Option<Closure<dyn FnMut(Event)>>,
        blocked: Option<Closure<dyn FnMut(Event)>>,
        upgrade: Option<Closure<dyn FnMut(Event)>>,
    }

    impl EventHandlers {
        fn empty() -> Self {
            Self {
                success: None,
                error: None,
                blocked: None,
                upgrade: None,
            }
        }
    }

    async fn request_result(request: IdbRequest) -> Result<JsValue, WorkshopStoreError> {
        let handlers = Rc::new(RefCell::new(EventHandlers::empty()));
        let promise = Promise::new(&mut |resolve: Function, reject: Function| {
            let success_request = request.clone();
            let success_resolve = resolve.clone();
            let success_reject = reject.clone();
            handlers.borrow_mut().success =
                Some(Closure::wrap(
                    Box::new(move |_event: Event| match success_request.result() {
                        Ok(value) => {
                            let _ = success_resolve.call1(&JsValue::UNDEFINED, &value);
                        }
                        Err(error) => {
                            let _ = success_reject.call1(&JsValue::UNDEFINED, &error);
                        }
                    }) as Box<dyn FnMut(_)>,
                ));

            let error_request = request.clone();
            let error_reject = reject.clone();
            handlers.borrow_mut().error = Some(Closure::wrap(Box::new(move |_event: Event| {
                let error = error_request
                    .error()
                    .ok()
                    .flatten()
                    .map(JsValue::from)
                    .unwrap_or_else(|| JsValue::from_str("IndexedDB request failed"));
                let _ = error_reject.call1(&JsValue::UNDEFINED, &error);
            }) as Box<dyn FnMut(_)>));

            let handlers = handlers.borrow();
            request.set_onsuccess(
                handlers
                    .success
                    .as_ref()
                    .map(|value| value.as_ref().unchecked_ref()),
            );
            request.set_onerror(
                handlers
                    .error
                    .as_ref()
                    .map(|value| value.as_ref().unchecked_ref()),
            );
        });

        let result = JsFuture::from(promise).await;
        request.set_onsuccess(None);
        request.set_onerror(None);
        let mut handlers = handlers.borrow_mut();
        handlers.success.take();
        handlers.error.take();
        drop(handlers);
        result.map_err(map_js_error)
    }

    struct TransactionWaiter {
        transaction: IdbTransaction,
        handlers: Rc<RefCell<EventHandlers>>,
        promise: Promise,
    }

    impl TransactionWaiter {
        fn new(transaction: &IdbTransaction) -> Self {
            let transaction = transaction.clone();
            let handlers = Rc::new(RefCell::new(EventHandlers::empty()));
            let promise = Promise::new(&mut |resolve: Function, reject: Function| {
                let complete_resolve = resolve.clone();
                handlers.borrow_mut().success =
                    Some(Closure::wrap(Box::new(move |_event: Event| {
                        let _ = complete_resolve.call0(&JsValue::UNDEFINED);
                    }) as Box<dyn FnMut(_)>));

                let abort_transaction = transaction.clone();
                let abort_reject = reject.clone();
                handlers.borrow_mut().error = Some(Closure::wrap(Box::new(move |_event: Event| {
                    let error = abort_transaction
                        .error()
                        .map(JsValue::from)
                        .unwrap_or_else(|| JsValue::from_str("IndexedDB transaction aborted"));
                    let _ = abort_reject.call1(&JsValue::UNDEFINED, &error);
                })
                    as Box<dyn FnMut(_)>));

                let handlers = handlers.borrow();
                transaction.set_oncomplete(
                    handlers
                        .success
                        .as_ref()
                        .map(|value| value.as_ref().unchecked_ref()),
                );
                transaction.set_onabort(
                    handlers
                        .error
                        .as_ref()
                        .map(|value| value.as_ref().unchecked_ref()),
                );
            });
            Self {
                transaction,
                handlers,
                promise,
            }
        }

        async fn wait(self) -> Result<(), WorkshopStoreError> {
            let result = JsFuture::from(self.promise).await;
            self.transaction.set_oncomplete(None);
            self.transaction.set_onabort(None);
            let mut handlers = self.handlers.borrow_mut();
            handlers.success.take();
            handlers.error.take();
            drop(handlers);
            result.map(|_| ()).map_err(map_js_error)
        }
    }

    struct OpenDatabase {
        database: IdbDatabase,
        _version_change: Closure<dyn FnMut(Event)>,
    }

    impl Drop for OpenDatabase {
        fn drop(&mut self) {
            self.database.set_onversionchange(None);
            self.database.close();
        }
    }

    async fn open_database() -> Result<OpenDatabase, WorkshopStoreError> {
        let window = web_sys::window().ok_or(WorkshopStoreError::IndexedDbUnavailable)?;
        let factory = window
            .indexed_db()
            .map_err(map_js_error)?
            .ok_or(WorkshopStoreError::IndexedDbUnavailable)?;
        let request = factory
            .open_with_u32(INDEXED_DB_NAME, INDEXED_DB_VERSION)
            .map_err(map_js_error)?;
        await_open_request(request).await
    }

    async fn await_open_request(
        request: IdbOpenDbRequest,
    ) -> Result<OpenDatabase, WorkshopStoreError> {
        let handlers = Rc::new(RefCell::new(EventHandlers::empty()));
        let promise = Promise::new(&mut |resolve: Function, reject: Function| {
            let success_request = request.clone();
            let success_resolve = resolve.clone();
            let success_reject = reject.clone();
            handlers.borrow_mut().success = Some(Closure::wrap(Box::new(move |_event: Event| {
                match success_request
                    .result()
                    .and_then(|value| value.dyn_into::<IdbDatabase>())
                {
                    Ok(database) => {
                        let _ = success_resolve.call1(&JsValue::UNDEFINED, database.as_ref());
                    }
                    Err(error) => {
                        let _ = success_reject.call1(&JsValue::UNDEFINED, &error);
                    }
                }
            })
                as Box<dyn FnMut(_)>));

            let error_request = request.clone();
            let error_reject = reject.clone();
            handlers.borrow_mut().error = Some(Closure::wrap(Box::new(move |_event: Event| {
                let error = error_request
                    .error()
                    .ok()
                    .flatten()
                    .map(JsValue::from)
                    .unwrap_or_else(|| JsValue::from_str("IndexedDB open failed"));
                let _ = error_reject.call1(&JsValue::UNDEFINED, &error);
            }) as Box<dyn FnMut(_)>));

            let blocked_reject = reject.clone();
            handlers.borrow_mut().blocked = Some(Closure::wrap(Box::new(move |_event: Event| {
                let _ =
                    blocked_reject.call1(&JsValue::UNDEFINED, &JsValue::from_str(BLOCKED_SENTINEL));
            })
                as Box<dyn FnMut(_)>));

            let upgrade_request = request.clone();
            let upgrade_reject = reject.clone();
            handlers.borrow_mut().upgrade = Some(Closure::wrap(Box::new(move |_event: Event| {
                let result = upgrade_request
                    .result()
                    .and_then(|value| value.dyn_into::<IdbDatabase>())
                    .and_then(|database| {
                        for name in [
                            SLOTS_OBJECT_STORE,
                            ARCHIVES_OBJECT_STORE,
                            PACKS_OBJECT_STORE,
                        ] {
                            if !database.object_store_names().contains(name) {
                                database.create_object_store(name)?;
                            }
                        }
                        Ok(())
                    });
                if result.is_err() {
                    let _ = upgrade_reject
                        .call1(&JsValue::UNDEFINED, &JsValue::from_str(SCHEMA_SENTINEL));
                    if let Some(transaction) = upgrade_request.transaction() {
                        let _ = transaction.abort();
                    }
                }
            })
                as Box<dyn FnMut(_)>));

            let handlers = handlers.borrow();
            request.set_onsuccess(
                handlers
                    .success
                    .as_ref()
                    .map(|value| value.as_ref().unchecked_ref()),
            );
            request.set_onerror(
                handlers
                    .error
                    .as_ref()
                    .map(|value| value.as_ref().unchecked_ref()),
            );
            request.set_onblocked(
                handlers
                    .blocked
                    .as_ref()
                    .map(|value| value.as_ref().unchecked_ref()),
            );
            request.set_onupgradeneeded(
                handlers
                    .upgrade
                    .as_ref()
                    .map(|value| value.as_ref().unchecked_ref()),
            );
        });

        let result = JsFuture::from(promise).await;
        request.set_onsuccess(None);
        request.set_onerror(None);
        request.set_onblocked(None);
        request.set_onupgradeneeded(None);
        let mut handlers = handlers.borrow_mut();
        handlers.success.take();
        handlers.error.take();
        handlers.blocked.take();
        handlers.upgrade.take();
        drop(handlers);

        let database: IdbDatabase = result
            .map_err(map_js_error)?
            .dyn_into()
            .map_err(map_js_error)?;
        if database.version() != f64::from(INDEXED_DB_VERSION) {
            database.close();
            return Err(WorkshopStoreError::IndexedDbSchemaMismatch);
        }
        let names = database.object_store_names();
        if names.length() != 3
            || !names.contains(SLOTS_OBJECT_STORE)
            || !names.contains(ARCHIVES_OBJECT_STORE)
            || !names.contains(PACKS_OBJECT_STORE)
        {
            database.close();
            return Err(WorkshopStoreError::IndexedDbSchemaMismatch);
        }
        let close_database = database.clone();
        let version_change = Closure::wrap(Box::new(move |_event: Event| {
            close_database.close();
        }) as Box<dyn FnMut(_)>);
        database.set_onversionchange(Some(version_change.as_ref().unchecked_ref()));
        Ok(OpenDatabase {
            database,
            _version_change: version_change,
        })
    }

    fn map_js_error(value: JsValue) -> WorkshopStoreError {
        if value.as_string().as_deref() == Some(SCHEMA_SENTINEL)
            || value.as_string().as_deref() == Some(BLOCKED_SENTINEL)
        {
            return WorkshopStoreError::IndexedDbSchemaMismatch;
        }
        let name = value
            .dyn_ref::<web_sys::DomException>()
            .map(web_sys::DomException::name)
            .unwrap_or_default();
        match name.as_str() {
            "QuotaExceededError" => WorkshopStoreError::IndexedDbQuotaDenied,
            "VersionError" | "NotFoundError" | "DataError" => {
                WorkshopStoreError::IndexedDbSchemaMismatch
            }
            "AbortError" | "ConstraintError" => WorkshopStoreError::IndexedDbTransactionAborted,
            "InvalidStateError" | "SecurityError" | "UnknownError" => {
                WorkshopStoreError::IndexedDbUnavailable
            }
            _ => WorkshopStoreError::IndexedDbTransactionAborted,
        }
    }
}

#[cfg(target_arch = "wasm32")]
pub use wasm::IndexedDbWorkshopStore;
