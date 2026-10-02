//! Exact Living opens and save/selection sequencing. Candidates are never
//! installed until their durable selection succeeds; failed work keeps residents.
use crate::{library_selection::*, living_store::*};
use nyon_workshop_core::living::{
    LivingArchiveReplayStatusV2, LivingArchiveReplayV2, LivingBookmarkV2, LivingCatalogHashV2,
    LivingGalaxyAuthorityV2, LivingNamedBookmarkV2, LivingNotebookEvidenceV2,
    LivingNotebookExperimentV2, LivingNotebookReplayV2, LivingNotebookV2,
    ValidatedLivingNotebookV2, begin_decode_living_archive_v2, begin_decode_living_notebook_v2,
    encode_living_archive_v2,
};

enum NotebookPurpose {
    Save {
        archive: Vec<u8>,
        catalog: Vec<u8>,
        bookmark: LivingBookmarkV2,
        leave: bool,
        bookmarks: Vec<LivingNamedBookmarkV2>,
        experiments: Vec<LivingNotebookExperimentV2>,
        evidence: Option<LivingNotebookEvidenceV2>,
    },
    Open {
        authority: Box<LivingGalaxyAuthorityV2>,
        descriptor: LivingSlotDescriptorV2,
        bookmark: LivingBookmarkV2,
        select: bool,
        bytes: Vec<u8>,
    },
}
enum Phase {
    RestoreRecorded {
        replay: Box<LivingArchiveReplayV2>,
        descriptor: LivingSlotDescriptorV2,
        reference: SelectionTarget,
    },
    DecodeArchive {
        replay: Box<LivingArchiveReplayV2>,
        purpose: NotebookPurpose,
    },
    DecodeNotebook {
        replay: Box<LivingNotebookReplayV2>,
        purpose: NotebookPurpose,
    },
    ReadNotebook {
        job: LivingStoreJobIdV2,
        authority: Box<LivingGalaxyAuthorityV2>,
        descriptor: LivingSlotDescriptorV2,
        bookmark: LivingBookmarkV2,
        select: bool,
    },
    WriteNotebook {
        job: LivingStoreJobIdV2,
        bookmark: LivingBookmarkV2,
        leave: bool,
        candidate: Option<Box<LivingGalaxyAuthorityV2>>,
    },
    Idle,
    ReadSelection(SelectionJobId),
    NeutralSelect(SelectionJobId),
    Load {
        job: LivingStoreJobIdV2,
        expected: Option<SelectionTarget>,
        observed_generation: Option<LivingGenerationV2>,
        select: bool,
    },
    Register {
        job: LivingStoreJobIdV2,
        archive: Vec<u8>,
        bookmark: LivingBookmarkV2,
        leave: bool,
        candidate: Option<Box<LivingGalaxyAuthorityV2>>,
    },
    Save {
        job: LivingStoreJobIdV2,
        bookmark: LivingBookmarkV2,
        leave: bool,
        candidate: Option<Box<LivingGalaxyAuthorityV2>>,
    },
    Select {
        job: SelectionJobId,
        candidate: Option<Box<LivingGalaxyAuthorityV2>>,
        descriptor: LivingSlotDescriptorV2,
        bookmark: LivingBookmarkV2,
        leave: bool,
    },
    List(LivingStoreJobIdV2),
    ListPacks(LivingStoreJobIdV2),
    ListPackReferences {
        job: LivingStoreJobIdV2,
        hash: LivingCatalogHashV2,
    },
    PackMutation {
        job: LivingStoreJobIdV2,
        descriptor: LivingPackDescriptorV2,
        mutation: LivingPackMutation,
    },
    Mutation(LivingStoreJobIdV2),
    ReadCatalog {
        job: LivingStoreJobIdV2,
        archive: Vec<u8>,
    },
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum LivingPackMutation {
    SetArchived(bool),
    Delete,
}

pub enum LivingClientEvent {
    Pending,
    Cancelled,
    NoSelection,
    AbsentSelection,
    SelectionCommitted,
    WorkshopSelection(SelectionTarget),
    Ready(Box<LivingGalaxyAuthorityV2>),
    ReadyWithNotebook {
        authority: Box<LivingGalaxyAuthorityV2>,
        notebook: Box<ValidatedLivingNotebookV2>,
    },
    Saved {
        bookmark: LivingBookmarkV2,
        leave: bool,
    },
    Listed(Vec<LivingSlotDescriptorV2>),
    PacksListed(Vec<LivingPackDescriptorV2>),
    PackReferencesListed {
        hash: LivingCatalogHashV2,
        references: Vec<LivingPackReferenceV2>,
    },
    PackMutated,
    Mutated,
    ArchiveCatalog {
        archive: Vec<u8>,
        catalog: Vec<u8>,
    },
    CatalogUnavailable {
        archive: Vec<u8>,
        error: String,
    },
    Failed(String),
}

pub struct LivingClient {
    store: Box<dyn LivingStoreV2>,
    coordinator: Box<dyn LibrarySelectionCoordinator>,
    epoch: u64,
    phase: Phase,
    cancel_requested: bool,
    mutation_clears_selection: bool,
    mutation_after_selection: Option<LivingStoreRequestV2>,
    notebook_bytes: Option<Vec<u8>>,
    ready_notebook: Option<ValidatedLivingNotebookV2>,
    recorded_target: Option<SelectionTarget>,
    packs: Vec<LivingPackDescriptorV2>,
    pack_references: std::collections::BTreeMap<LivingCatalogHashV2, Vec<LivingPackReferenceV2>>,
    pub selection: Option<LibrarySelection>,
    pub slot: Option<LivingSlotDescriptorV2>,
    pub saved: Option<LivingBookmarkV2>,
}

impl Drop for LivingClient {
    fn drop(&mut self) {
        match &self.phase {
            Phase::ReadSelection(job) | Phase::NeutralSelect(job) | Phase::Select { job, .. } => {
                let _ = self.coordinator.cancel(self.epoch, *job);
            }
            Phase::ReadNotebook { job, .. }
            | Phase::WriteNotebook { job, .. }
            | Phase::Load { job, .. }
            | Phase::Register { job, .. }
            | Phase::Save { job, .. }
            | Phase::ListPackReferences { job, .. }
            | Phase::ListPacks(job)
            | Phase::PackMutation { job, .. }
            | Phase::List(job)
            | Phase::Mutation(job) => {
                let _ = self.store.cancel(self.epoch, *job);
            }
            Phase::ReadCatalog { job, .. } => {
                let _ = self.store.cancel(self.epoch, *job);
            }
            Phase::RestoreRecorded { .. }
            | Phase::DecodeArchive { .. }
            | Phase::DecodeNotebook { .. }
            | Phase::Idle => {}
        }
    }
}

fn encoded<T: serde::Serialize>(value: &T) -> String {
    serde_json::to_string(value)
        .expect("fixed identity serializes")
        .trim_matches('"')
        .to_owned()
}
fn target(slot: LivingSlotIdV2, bookmark: LivingBookmarkV2) -> SelectionTarget {
    SelectionTarget {
        authority: SelectedAuthority::LivingV2,
        slot: slot.0,
        branch: encoded(&bookmark.view.branch),
        revision: bookmark.view.revision.as_ref().map(encoded),
        tick: bookmark.view.tick.0,
        digest: encoded(&bookmark.digest),
    }
}
fn matches_target(reference: &SelectionTarget, bookmark: LivingBookmarkV2) -> bool {
    reference.authority == SelectedAuthority::LivingV2
        && reference.branch == encoded(&bookmark.view.branch)
        && reference.revision == bookmark.view.revision.as_ref().map(encoded)
        && reference.tick == bookmark.view.tick.0
        && reference.digest == encoded(&bookmark.digest)
}

impl LivingClient {
    pub fn new(
        store: Box<dyn LivingStoreV2>,
        coordinator: Box<dyn LibrarySelectionCoordinator>,
    ) -> Self {
        Self {
            store,
            coordinator,
            epoch: 1,
            phase: Phase::Idle,
            cancel_requested: false,
            mutation_clears_selection: false,
            mutation_after_selection: None,
            notebook_bytes: None,
            ready_notebook: None,
            recorded_target: None,
            packs: Vec::new(),
            pack_references: std::collections::BTreeMap::new(),
            selection: None,
            slot: None,
            saved: None,
        }
    }
    pub fn busy(&self) -> bool {
        !matches!(self.phase, Phase::Idle)
    }
    pub fn cancel(&mut self) {
        self.cancel_requested = self.busy();
        match &self.phase {
            Phase::ReadSelection(job) | Phase::NeutralSelect(job) | Phase::Select { job, .. } => {
                let _ = self.coordinator.cancel(self.epoch, *job);
            }
            Phase::ReadNotebook { job, .. }
            | Phase::WriteNotebook { job, .. }
            | Phase::Load { job, .. }
            | Phase::Register { job, .. }
            | Phase::Save { job, .. }
            | Phase::ListPackReferences { job, .. }
            | Phase::ListPacks(job)
            | Phase::PackMutation { job, .. }
            | Phase::List(job)
            | Phase::Mutation(job)
            | Phase::ReadCatalog { job, .. } => {
                let _ = self.store.cancel(self.epoch, *job);
            }
            Phase::RestoreRecorded { .. }
            | Phase::DecodeArchive { .. }
            | Phase::DecodeNotebook { .. }
            | Phase::Idle => {}
        }
    }
    fn idle(&self) -> Result<(), String> {
        if self.busy() {
            Err("Living storage operation already active".into())
        } else {
            Ok(())
        }
    }
    pub fn read_selection(&mut self) -> Result<(), String> {
        self.idle()?;
        let job = self
            .coordinator
            .begin(self.epoch, SelectionRequest::Read)
            .map_err(|e| e.to_string())?;
        self.phase = Phase::ReadSelection(job);
        Ok(())
    }
    pub fn select_neutral(&mut self, target: Option<SelectionTarget>) -> Result<(), String> {
        self.idle()?;
        let job = self
            .coordinator
            .begin(
                self.epoch,
                SelectionRequest::CompareAndSwap {
                    expected: self
                        .selection
                        .as_ref()
                        .map(|selection| selection.generation),
                    target,
                },
            )
            .map_err(|e| e.to_string())?;
        self.phase = Phase::NeutralSelect(job);
        Ok(())
    }
    pub fn list(&mut self) -> Result<(), String> {
        self.idle()?;
        let job = self
            .store
            .begin(self.epoch, LivingStoreRequestV2::ListSlots)
            .map_err(|e| e.to_string())?;
        self.phase = Phase::List(job);
        Ok(())
    }
    pub fn pack_operation_pending(&self) -> bool {
        matches!(
            self.phase,
            Phase::ListPacks(_) | Phase::ListPackReferences { .. } | Phase::PackMutation { .. }
        )
    }
    pub fn pack_mutation_pending(&self) -> bool {
        matches!(self.phase, Phase::PackMutation { .. })
    }
    pub fn packs(&self) -> &[LivingPackDescriptorV2] {
        &self.packs
    }
    pub fn list_packs(&mut self) -> Result<(), String> {
        self.idle()?;
        let job = self
            .store
            .begin(self.epoch, LivingStoreRequestV2::ListPacks)
            .map_err(|e| e.to_string())?;
        self.phase = Phase::ListPacks(job);
        Ok(())
    }
    pub fn pack_references(&self, hash: LivingCatalogHashV2) -> Option<&[LivingPackReferenceV2]> {
        self.pack_references.get(&hash).map(Vec::as_slice)
    }
    pub fn list_pack_references(&mut self, hash: LivingCatalogHashV2) -> Result<(), String> {
        self.idle()?;
        let job = self
            .store
            .begin(
                self.epoch,
                LivingStoreRequestV2::ListPackReferences { hash },
            )
            .map_err(|e| e.to_string())?;
        self.phase = Phase::ListPackReferences { job, hash };
        Ok(())
    }
    pub fn mutate_pack(
        &mut self,
        descriptor: &LivingPackDescriptorV2,
        mutation: LivingPackMutation,
        resident_catalog: Option<LivingCatalogHashV2>,
    ) -> Result<(), String> {
        self.idle()?;
        let current = self
            .packs
            .iter()
            .find(|pack| pack.hash == descriptor.hash)
            .ok_or("Refresh the exact pack Library before this operation")?;
        if current.generation != descriptor.generation || current.archived != descriptor.archived {
            return Err("Pack review is stale; refresh its exact generation".into());
        }
        let builtin = nyon_workshop_core::living::living_core_pack_v2()
            .map_err(|e| e.to_string())?
            .catalog_hash();
        if descriptor.hash == builtin {
            return Err("The built-in Living catalog cannot be archived or deleted".into());
        }
        if resident_catalog == Some(descriptor.hash) {
            return Err("Close the resident galaxy before changing its exact catalog".into());
        }
        let request = match mutation {
            LivingPackMutation::SetArchived(archived) => LivingStoreRequestV2::SetPackArchived {
                hash: descriptor.hash,
                expected_generation: descriptor.generation,
                archived,
            },
            LivingPackMutation::Delete => {
                if !descriptor.archived {
                    return Err("Archive the exact pack before permanent deletion".into());
                }
                LivingStoreRequestV2::DeletePack {
                    hash: descriptor.hash,
                    expected_generation: descriptor.generation,
                }
            }
        };
        let job = self
            .store
            .begin(self.epoch, request)
            .map_err(|e| e.to_string())?;
        self.phase = Phase::PackMutation {
            job,
            descriptor: descriptor.clone(),
            mutation,
        };
        Ok(())
    }
    pub fn open(&mut self, descriptor: &LivingSlotDescriptorV2) -> Result<(), String> {
        self.idle()?;
        if descriptor.archived {
            return Err("Archived Living galaxies must be restored before opening".into());
        }
        let job = self
            .store
            .begin(
                self.epoch,
                LivingStoreRequestV2::LoadSlot { id: descriptor.id },
            )
            .map_err(|e| e.to_string())?;
        self.phase = Phase::Load {
            job,
            expected: None,
            observed_generation: Some(descriptor.generation),
            select: true,
        };
        Ok(())
    }
    pub fn new_resident(&mut self) -> Result<(), String> {
        self.idle()?;
        self.slot = None;
        self.saved = None;
        self.epoch = self
            .epoch
            .checked_add(1)
            .ok_or("Living session epoch exhausted")?;
        Ok(())
    }
    pub fn save_with_notebook(
        &mut self,
        authority: &LivingGalaxyAuthorityV2,
        bookmarks: Vec<LivingNamedBookmarkV2>,
        experiments: Vec<LivingNotebookExperimentV2>,
        leave: bool,
    ) -> Result<(), String> {
        self.save_with_notebook_evidence(authority, bookmarks, experiments, None, leave)
    }
    pub fn save_with_notebook_evidence(
        &mut self,
        authority: &LivingGalaxyAuthorityV2,
        bookmarks: Vec<LivingNamedBookmarkV2>,
        experiments: Vec<LivingNotebookExperimentV2>,
        evidence: Option<LivingNotebookEvidenceV2>,
        leave: bool,
    ) -> Result<(), String> {
        self.idle()?;
        let archive = encode_living_archive_v2(authority).map_err(|e| e.to_string())?;
        let replay = begin_decode_living_archive_v2(&archive, authority.catalog())
            .map_err(|e| e.to_string())?;
        self.phase = Phase::DecodeArchive {
            replay: Box::new(replay),
            purpose: NotebookPurpose::Save {
                archive,
                catalog: authority.catalog().canonical_bytes().to_vec(),
                bookmark: authority.bookmark().map_err(|e| e.to_string())?,
                leave,
                bookmarks,
                experiments,
                evidence,
            },
        };
        Ok(())
    }
    pub fn save(&mut self, authority: &LivingGalaxyAuthorityV2, leave: bool) -> Result<(), String> {
        self.idle()?;
        let archive = encode_living_archive_v2(authority).map_err(|e| e.to_string())?;
        let bookmark = authority.bookmark().map_err(|e| e.to_string())?;
        let job = self
            .store
            .begin(
                self.epoch,
                LivingStoreRequestV2::PutPack {
                    canonical_bytes: authority.catalog().canonical_bytes().to_vec(),
                },
            )
            .map_err(|e| e.to_string())?;
        self.phase = Phase::Register {
            job,
            archive,
            bookmark,
            leave,
            candidate: None,
        };
        Ok(())
    }
    pub fn import_candidate(
        &mut self,
        authority: Box<LivingGalaxyAuthorityV2>,
    ) -> Result<(), String> {
        self.idle()?;
        let archive = encode_living_archive_v2(&authority).map_err(|error| error.to_string())?;
        let bookmark = authority.bookmark().map_err(|error| error.to_string())?;
        let job = self
            .store
            .begin(
                self.epoch,
                LivingStoreRequestV2::PutPack {
                    canonical_bytes: authority.catalog().canonical_bytes().to_vec(),
                },
            )
            .map_err(|error| error.to_string())?;
        self.phase = Phase::Register {
            job,
            archive,
            bookmark,
            leave: false,
            candidate: Some(authority),
        };
        Ok(())
    }
    pub fn catalog_for_import(
        &mut self,
        archive: Vec<u8>,
        hash: nyon_workshop_core::living::LivingCatalogHashV2,
    ) -> Result<(), String> {
        self.idle()?;
        let job = self
            .store
            .begin(self.epoch, LivingStoreRequestV2::ReadPack { hash })
            .map_err(|error| error.to_string())?;
        self.phase = Phase::ReadCatalog { job, archive };
        Ok(())
    }
    pub fn mutate(&mut self, request: LivingStoreRequestV2) -> Result<(), String> {
        self.idle()?;
        if matches!(
            request,
            LivingStoreRequestV2::SetPackArchived { .. } | LivingStoreRequestV2::DeletePack { .. }
        ) {
            return Err("Use exact pack review for catalog lifecycle operations".into());
        }
        let active = self.slot.as_ref().map(|slot| slot.id);
        if matches!(&request, LivingStoreRequestV2::DeleteArchivedSlot{id,..} if Some(*id)==active)
        {
            return Err("Close the resident galaxy before permanent deletion".into());
        }
        if matches!(&request, LivingStoreRequestV2::SetArchived{id,archived:true,..} if Some(*id)==active)
        {
            return Err("Close the resident galaxy before archiving its save".into());
        }
        self.mutation_clears_selection = self
            .selection
            .as_ref()
            .and_then(|selection| selection.target().ok().flatten())
            .is_some_and(|selected| {
                selected.authority == SelectedAuthority::LivingV2
                    && match &request {
                        LivingStoreRequestV2::SetArchived {
                            id, archived: true, ..
                        }
                        | LivingStoreRequestV2::DeleteArchivedSlot { id, .. } => {
                            selected.slot == id.0
                        }
                        _ => false,
                    }
            });
        if self.mutation_clears_selection
            && matches!(request, LivingStoreRequestV2::DeleteArchivedSlot { .. })
        {
            // Permanent deletion first relinquishes the global Continue reference.
            // A failed coordinator CAS must never begin the destructive store job.
            self.mutation_clears_selection = false;
            self.mutation_after_selection = None;
            self.select_neutral(None)?;
            self.mutation_after_selection = Some(request);
            return Ok(());
        }
        let job = self
            .store
            .begin(self.epoch, request)
            .map_err(|e| e.to_string())?;
        self.phase = Phase::Mutation(job);
        Ok(())
    }
    fn select(
        &mut self,
        descriptor: LivingSlotDescriptorV2,
        bookmark: LivingBookmarkV2,
        candidate: Option<Box<LivingGalaxyAuthorityV2>>,
        leave: bool,
    ) -> Result<(), String> {
        let job = self
            .coordinator
            .begin(
                self.epoch,
                SelectionRequest::CompareAndSwap {
                    expected: self.selection.as_ref().map(|s| s.generation),
                    target: Some(target(descriptor.id, bookmark)),
                },
            )
            .map_err(|e| e.to_string())?;
        self.phase = Phase::Select {
            job,
            candidate,
            descriptor,
            bookmark,
            leave,
        };
        Ok(())
    }
    fn finish_open(
        &mut self,
        authority: Box<LivingGalaxyAuthorityV2>,
        descriptor: LivingSlotDescriptorV2,
        bookmark: LivingBookmarkV2,
        select: bool,
    ) -> Result<LivingClientEvent, String> {
        if let Some(reference) = self.recorded_target.take()
            && !matches_target(&reference, bookmark)
        {
            let parse = |value: &str| serde_json::Value::String(value.to_owned());
            let view = nyon_workshop_core::living::LivingHistoryViewV2 {
                branch: serde_json::from_value(parse(&reference.branch))
                    .map_err(|_| "Recorded Living branch is invalid; explicit recovery required")?,
                revision: reference
                    .revision
                    .as_ref()
                    .map(|value| serde_json::from_value(parse(value)))
                    .transpose()
                    .map_err(
                        |_| "Recorded Living revision is invalid; explicit recovery required",
                    )?,
                tick: nyon_workshop_core::living::LivingTickV2(reference.tick),
            };
            let replay = nyon_workshop_core::living::begin_restore_living_view_v2(&authority, view).map_err(|error| format!("Recorded Living state differs from retained history; explicit recovery required: {error}"))?;
            self.phase = Phase::RestoreRecorded {
                replay: Box::new(replay),
                descriptor,
                reference,
            };
            return Ok(LivingClientEvent::Pending);
        }
        if select {
            self.select(descriptor, bookmark, Some(authority), false)?;
            Ok(LivingClientEvent::Pending)
        } else {
            self.slot = Some(descriptor);
            self.saved = Some(bookmark);
            Ok(if let Some(notebook) = self.ready_notebook.take() {
                LivingClientEvent::ReadyWithNotebook {
                    authority,
                    notebook: Box::new(notebook),
                }
            } else {
                LivingClientEvent::Ready(authority)
            })
        }
    }
    pub fn poll(&mut self) -> LivingClientEvent {
        let phase = std::mem::replace(&mut self.phase, Phase::Idle);
        match self.advance(phase) {
            Ok(event) => event,
            Err(error) => {
                self.mutation_clears_selection = false;
                self.mutation_after_selection = None;
                self.notebook_bytes = None;
                self.ready_notebook = None;
                self.recorded_target = None;
                LivingClientEvent::Failed(error)
            }
        }
    }
    fn advance(&mut self, phase: Phase) -> Result<LivingClientEvent, String> {
        if self.cancel_requested {
            let pending = match &phase {
                Phase::ReadSelection(job)
                | Phase::NeutralSelect(job)
                | Phase::Select { job, .. } => {
                    match self.coordinator.poll(self.epoch, *job, 1024) {
                        SelectionPoll::Pending => true,
                        result => {
                            if let SelectionPoll::Ready(selection) = result {
                                self.selection = selection;
                            }
                            self.coordinator.release(self.epoch, *job);
                            false
                        }
                    }
                }
                Phase::ReadNotebook { job, .. }
                | Phase::WriteNotebook { job, .. }
                | Phase::Load { job, .. }
                | Phase::Register { job, .. }
                | Phase::Save { job, .. }
                | Phase::ListPackReferences { job, .. }
                | Phase::ListPacks(job)
                | Phase::PackMutation { job, .. }
                | Phase::List(job)
                | Phase::Mutation(job)
                | Phase::ReadCatalog { job, .. } => {
                    if matches!(
                        self.store.poll(self.epoch, *job, 1024),
                        LivingStorePollV2::Pending { .. }
                    ) {
                        true
                    } else {
                        self.store.release(self.epoch, *job);
                        false
                    }
                }
                Phase::RestoreRecorded { .. }
                | Phase::DecodeArchive { .. }
                | Phase::DecodeNotebook { .. }
                | Phase::Idle => false,
            };
            if pending {
                self.phase = phase;
                return Ok(LivingClientEvent::Pending);
            }
            self.notebook_bytes = None;
            self.ready_notebook = None;
            self.recorded_target = None;
            self.cancel_requested = false;
            self.mutation_clears_selection = false;
            self.mutation_after_selection = None;
            return Ok(LivingClientEvent::Cancelled);
        }
        match phase {
            Phase::RestoreRecorded {
                mut replay,
                descriptor,
                reference,
            } => {
                if replay.poll(128).map_err(|e| e.to_string())?
                    == LivingArchiveReplayStatusV2::Pending
                {
                    self.phase = Phase::RestoreRecorded {
                        replay,
                        descriptor,
                        reference,
                    };
                    return Ok(LivingClientEvent::Pending);
                }
                let authority = Box::new(
                    (*replay)
                        .finish()
                        .map_err(|e| e.to_string())?
                        .into_authority(),
                );
                let bookmark = authority.bookmark().map_err(|e| e.to_string())?;
                if !matches_target(&reference, bookmark) {
                    return Err("Recorded Living state digest differs from retained history; explicit recovery required".into());
                }
                self.finish_open(authority, descriptor, bookmark, false)
            }
            Phase::DecodeArchive {
                mut replay,
                purpose,
            } => {
                if replay.poll(128).map_err(|e| e.to_string())?
                    == LivingArchiveReplayStatusV2::Pending
                {
                    self.phase = Phase::DecodeArchive { replay, purpose };
                    return Ok(LivingClientEvent::Pending);
                }
                let archive = (*replay).finish().map_err(|e| e.to_string())?;
                let bytes = match &purpose {
                    NotebookPurpose::Save {
                        bookmarks,
                        experiments,
                        evidence,
                        ..
                    } => LivingNotebookV2::seal_with_evidence(
                        &archive,
                        bookmarks.clone(),
                        experiments.clone(),
                        evidence.clone(),
                    )
                    .map_err(|e| e.to_string())?
                    .canonical_bytes()
                    .map_err(|e| e.to_string())?,
                    NotebookPurpose::Open { bytes, .. } => bytes.clone(),
                };
                let replay =
                    begin_decode_living_notebook_v2(&bytes, archive).map_err(|e| e.to_string())?;
                self.phase = Phase::DecodeNotebook {
                    replay: Box::new(replay),
                    purpose,
                };
                Ok(LivingClientEvent::Pending)
            }
            Phase::DecodeNotebook {
                mut replay,
                purpose,
            } => {
                if replay.poll(128).map_err(|e| e.to_string())?
                    == LivingArchiveReplayStatusV2::Pending
                {
                    self.phase = Phase::DecodeNotebook { replay, purpose };
                    return Ok(LivingClientEvent::Pending);
                }
                let validated = (*replay).finish().map_err(|e| e.to_string())?;
                match purpose {
                    NotebookPurpose::Save {
                        archive,
                        catalog,
                        bookmark,
                        leave,
                        ..
                    } => {
                        self.notebook_bytes = Some(
                            validated
                                .notebook()
                                .canonical_bytes()
                                .map_err(|e| e.to_string())?,
                        );
                        let job = self
                            .store
                            .begin(
                                self.epoch,
                                LivingStoreRequestV2::PutPack {
                                    canonical_bytes: catalog,
                                },
                            )
                            .map_err(|e| e.to_string())?;
                        self.phase = Phase::Register {
                            job,
                            archive,
                            bookmark,
                            leave,
                            candidate: None,
                        };
                        Ok(LivingClientEvent::Pending)
                    }
                    NotebookPurpose::Open {
                        authority,
                        descriptor,
                        bookmark,
                        select,
                        ..
                    } => {
                        self.ready_notebook = Some(validated);
                        self.finish_open(authority, descriptor, bookmark, select)
                    }
                }
            }
            Phase::ReadNotebook {
                job,
                authority,
                descriptor,
                bookmark,
                select,
            } => match self.store.poll(self.epoch, job, 128) {
                LivingStorePollV2::Pending { .. } => {
                    self.phase = Phase::ReadNotebook {
                        job,
                        authority,
                        descriptor,
                        bookmark,
                        select,
                    };
                    Ok(LivingClientEvent::Pending)
                }
                LivingStorePollV2::Ready(LivingStoreResultV2::NotebookBytes(bytes)) => {
                    self.store.release(self.epoch, job);
                    if let Some(bytes) = bytes {
                        let archive_bytes =
                            encode_living_archive_v2(&authority).map_err(|e| e.to_string())?;
                        let replay =
                            begin_decode_living_archive_v2(&archive_bytes, authority.catalog())
                                .map_err(|e| e.to_string())?;
                        self.phase = Phase::DecodeArchive {
                            replay: Box::new(replay),
                            purpose: NotebookPurpose::Open {
                                authority,
                                descriptor,
                                bookmark,
                                select,
                                bytes,
                            },
                        };
                        Ok(LivingClientEvent::Pending)
                    } else {
                        self.finish_open(authority, descriptor, bookmark, select)
                    }
                }
                LivingStorePollV2::Failed(error) => {
                    self.store.release(self.epoch, job);
                    Err(error.to_string())
                }
                _ => {
                    self.store.release(self.epoch, job);
                    Err("Living notebook read protocol mismatch".into())
                }
            },
            Phase::WriteNotebook {
                job,
                bookmark,
                leave,
                candidate,
            } => match self.store.poll(self.epoch, job, 128) {
                LivingStorePollV2::Pending { .. } => {
                    self.phase = Phase::WriteNotebook {
                        job,
                        bookmark,
                        leave,
                        candidate,
                    };
                    Ok(LivingClientEvent::Pending)
                }
                LivingStorePollV2::Ready(LivingStoreResultV2::Slot(slot)) => {
                    self.store.release(self.epoch, job);
                    if candidate.is_none() {
                        self.slot = Some(slot.clone());
                    }
                    self.select(slot, bookmark, candidate, leave)?;
                    Ok(LivingClientEvent::Pending)
                }
                LivingStorePollV2::Failed(error) => {
                    self.store.release(self.epoch, job);
                    Err(error.to_string())
                }
                _ => {
                    self.store.release(self.epoch, job);
                    Err("Living notebook write protocol mismatch".into())
                }
            },
            Phase::Idle => Ok(LivingClientEvent::Pending),
            Phase::ReadSelection(job) => match self.coordinator.poll(self.epoch, job, 1024) {
                SelectionPoll::Pending => {
                    self.phase = Phase::ReadSelection(job);
                    Ok(LivingClientEvent::Pending)
                }
                SelectionPoll::Ready(selection) => {
                    self.coordinator.release(self.epoch, job);
                    self.selection = selection;
                    let reference = self
                        .selection
                        .as_ref()
                        .map(LibrarySelection::target)
                        .transpose()
                        .map_err(|e| e.to_string())?
                        .flatten();
                    let Some(reference) = reference else {
                        return Ok(if self.selection.is_some() {
                            LivingClientEvent::NoSelection
                        } else {
                            LivingClientEvent::AbsentSelection
                        });
                    };
                    if reference.authority == SelectedAuthority::WorkshopV1 {
                        return Ok(LivingClientEvent::WorkshopSelection(reference));
                    }
                    let job = self
                        .store
                        .begin(
                            self.epoch,
                            LivingStoreRequestV2::LoadSlot {
                                id: LivingSlotIdV2(reference.slot),
                            },
                        )
                        .map_err(|e| e.to_string())?;
                    self.phase = Phase::Load {
                        job,
                        expected: Some(reference),
                        observed_generation: None,
                        select: false,
                    };
                    Ok(LivingClientEvent::Pending)
                }
                SelectionPoll::Failed(error) => {
                    self.coordinator.release(self.epoch, job);
                    Err(error.to_string())
                }
                SelectionPoll::Cancelled => {
                    self.coordinator.release(self.epoch, job);
                    Err("Continue selection cancelled".into())
                }
            },
            Phase::NeutralSelect(job) => match self.coordinator.poll(self.epoch, job, 1024) {
                SelectionPoll::Pending => {
                    self.phase = Phase::NeutralSelect(job);
                    Ok(LivingClientEvent::Pending)
                }
                SelectionPoll::Ready(selection) => {
                    self.coordinator.release(self.epoch, job);
                    self.selection = selection;
                    if let Some(request) = self.mutation_after_selection.take() {
                        let job = self
                            .store
                            .begin(self.epoch, request)
                            .map_err(|e| e.to_string())?;
                        self.phase = Phase::Mutation(job);
                        return Ok(LivingClientEvent::Pending);
                    }
                    if std::mem::take(&mut self.mutation_clears_selection) {
                        Ok(LivingClientEvent::Mutated)
                    } else {
                        Ok(LivingClientEvent::SelectionCommitted)
                    }
                }
                SelectionPoll::Failed(error) => {
                    self.coordinator.release(self.epoch, job);
                    Err(error.to_string())
                }
                SelectionPoll::Cancelled => {
                    self.coordinator.release(self.epoch, job);
                    Err("Continue selection cancelled".into())
                }
            },
            Phase::Load {
                job,
                expected,
                observed_generation,
                select,
            } => match self.store.poll(self.epoch, job, 1024) {
                LivingStorePollV2::Pending { .. } => {
                    self.phase = Phase::Load {
                        job,
                        expected,
                        observed_generation,
                        select,
                    };
                    Ok(LivingClientEvent::Pending)
                }
                LivingStorePollV2::Ready(LivingStoreResultV2::Loaded {
                    descriptor,
                    authority,
                }) => {
                    self.store.release(self.epoch, job);
                    let bookmark = authority.bookmark().map_err(|e| e.to_string())?;
                    if observed_generation
                        .is_some_and(|generation| generation != descriptor.generation)
                    {
                        return Err("Living Library row changed; refresh and retry".into());
                    }
                    self.recorded_target = expected;
                    let job = self
                        .store
                        .begin(
                            self.epoch,
                            LivingStoreRequestV2::ReadNotebook {
                                id: descriptor.id,
                                generation: descriptor.generation,
                            },
                        )
                        .map_err(|e| e.to_string())?;
                    self.phase = Phase::ReadNotebook {
                        job,
                        authority,
                        descriptor,
                        bookmark,
                        select,
                    };
                    Ok(LivingClientEvent::Pending)
                }
                LivingStorePollV2::Failed(error) => {
                    self.store.release(self.epoch, job);
                    Err(error.to_string())
                }
                _ => {
                    self.store.release(self.epoch, job);
                    Err("Living load protocol mismatch".into())
                }
            },
            Phase::Register {
                job,
                archive,
                bookmark,
                leave,
                candidate,
            } => match self.store.poll(self.epoch, job, 1024) {
                LivingStorePollV2::Pending { .. } => {
                    self.phase = Phase::Register {
                        job,
                        archive,
                        bookmark,
                        leave,
                        candidate,
                    };
                    Ok(LivingClientEvent::Pending)
                }
                LivingStorePollV2::Ready(LivingStoreResultV2::Pack(_)) => {
                    self.store.release(self.epoch, job);
                    let request = if let Some(slot) = &self.slot
                        && candidate.is_none()
                    {
                        LivingStoreRequestV2::SaveSlot {
                            id: slot.id,
                            expected_generation: slot.generation,
                            archive,
                        }
                    } else {
                        LivingStoreRequestV2::CreateSlot {
                            name: "Living Galaxy".into(),
                            archive,
                        }
                    };
                    let job = self
                        .store
                        .begin(self.epoch, request)
                        .map_err(|e| e.to_string())?;
                    self.phase = Phase::Save {
                        job,
                        bookmark,
                        leave,
                        candidate,
                    };
                    Ok(LivingClientEvent::Pending)
                }
                LivingStorePollV2::Failed(error) => {
                    self.store.release(self.epoch, job);
                    Err(error.to_string())
                }
                _ => {
                    self.store.release(self.epoch, job);
                    Err("Living catalog registration protocol mismatch".into())
                }
            },
            Phase::Save {
                job,
                bookmark,
                leave,
                candidate,
            } => match self.store.poll(self.epoch, job, 1024) {
                LivingStorePollV2::Pending { .. } => {
                    self.phase = Phase::Save {
                        job,
                        bookmark,
                        leave,
                        candidate,
                    };
                    Ok(LivingClientEvent::Pending)
                }
                LivingStorePollV2::Ready(LivingStoreResultV2::Slot(slot)) => {
                    self.store.release(self.epoch, job);
                    if candidate.is_none() {
                        self.slot = Some(slot.clone());
                    }
                    if let Some(canonical_bytes) = self.notebook_bytes.take() {
                        let job = self
                            .store
                            .begin(
                                self.epoch,
                                LivingStoreRequestV2::WriteNotebook {
                                    id: slot.id,
                                    expected_generation: slot.generation,
                                    canonical_bytes,
                                },
                            )
                            .map_err(|e| e.to_string())?;
                        self.phase = Phase::WriteNotebook {
                            job,
                            bookmark,
                            leave,
                            candidate,
                        };
                    } else {
                        self.select(slot, bookmark, candidate, leave)?;
                    }
                    Ok(LivingClientEvent::Pending)
                }
                LivingStorePollV2::Failed(error) => {
                    self.store.release(self.epoch, job);
                    Err(error.to_string())
                }
                _ => {
                    self.store.release(self.epoch, job);
                    Err("Living save protocol mismatch".into())
                }
            },
            Phase::Select {
                job,
                candidate,
                descriptor,
                bookmark,
                leave,
            } => match self.coordinator.poll(self.epoch, job, 1024) {
                SelectionPoll::Pending => {
                    self.phase = Phase::Select {
                        job,
                        candidate,
                        descriptor,
                        bookmark,
                        leave,
                    };
                    Ok(LivingClientEvent::Pending)
                }
                SelectionPoll::Ready(selection) => {
                    self.coordinator.release(self.epoch, job);
                    let returned = selection
                        .as_ref()
                        .map(LibrarySelection::target)
                        .transpose()
                        .map_err(|error| error.to_string())?
                        .flatten();
                    if returned != Some(target(descriptor.id, bookmark)) {
                        return Err(
                            "Continue coordinator returned a different saved reference".into()
                        );
                    }
                    self.selection = selection;
                    self.slot = Some(descriptor);
                    self.saved = Some(bookmark);
                    if let Some(authority) = candidate {
                        Ok(if let Some(notebook) = self.ready_notebook.take() {
                            LivingClientEvent::ReadyWithNotebook {
                                authority,
                                notebook: Box::new(notebook),
                            }
                        } else {
                            LivingClientEvent::Ready(authority)
                        })
                    } else {
                        Ok(LivingClientEvent::Saved { bookmark, leave })
                    }
                }
                SelectionPoll::Failed(error) => {
                    self.coordinator.release(self.epoch, job);
                    Err(format!(
                        "Galaxy slot committed but Continue selection failed: {error}"
                    ))
                }
                SelectionPoll::Cancelled => {
                    self.coordinator.release(self.epoch, job);
                    Err("Continue selection cancelled; durable slot remains available".into())
                }
            },
            Phase::List(job) => match self.store.poll(self.epoch, job, 1024) {
                LivingStorePollV2::Pending { .. } => {
                    self.phase = Phase::List(job);
                    Ok(LivingClientEvent::Pending)
                }
                LivingStorePollV2::Ready(LivingStoreResultV2::Slots(slots)) => {
                    self.store.release(self.epoch, job);
                    Ok(LivingClientEvent::Listed(slots))
                }
                LivingStorePollV2::Failed(error) => {
                    self.store.release(self.epoch, job);
                    Err(error.to_string())
                }
                _ => {
                    self.store.release(self.epoch, job);
                    Err("Living list protocol mismatch".into())
                }
            },
            Phase::ListPacks(job) => match self.store.poll(self.epoch, job, 1024) {
                LivingStorePollV2::Pending { .. } => {
                    self.phase = Phase::ListPacks(job);
                    Ok(LivingClientEvent::Pending)
                }
                LivingStorePollV2::Ready(LivingStoreResultV2::Packs(packs)) => {
                    self.store.release(self.epoch, job);
                    self.packs = packs.clone();
                    Ok(LivingClientEvent::PacksListed(packs))
                }
                LivingStorePollV2::Failed(error) => {
                    self.store.release(self.epoch, job);
                    Err(error.to_string())
                }
                _ => {
                    self.store.release(self.epoch, job);
                    Err("Living pack list protocol mismatch".into())
                }
            },
            Phase::ListPackReferences { job, hash } => match self.store.poll(self.epoch, job, 1024)
            {
                LivingStorePollV2::Pending { .. } => {
                    self.phase = Phase::ListPackReferences { job, hash };
                    Ok(LivingClientEvent::Pending)
                }
                LivingStorePollV2::Ready(LivingStoreResultV2::PackReferences {
                    hash: actual,
                    references,
                }) if hash == actual => {
                    self.store.release(self.epoch, job);
                    self.pack_references.insert(hash, references.clone());
                    Ok(LivingClientEvent::PackReferencesListed { hash, references })
                }
                LivingStorePollV2::Failed(error) => {
                    self.store.release(self.epoch, job);
                    Err(error.to_string())
                }
                _ => {
                    self.store.release(self.epoch, job);
                    Err("Living pack references protocol mismatch".into())
                }
            },
            Phase::PackMutation {
                job,
                descriptor,
                mutation,
            } => match self.store.poll(self.epoch, job, 1024) {
                LivingStorePollV2::Pending { .. } => {
                    self.phase = Phase::PackMutation {
                        job,
                        descriptor,
                        mutation,
                    };
                    Ok(LivingClientEvent::Pending)
                }
                LivingStorePollV2::Ready(result) => {
                    self.store.release(self.epoch, job);
                    match (mutation, result) {
                        (LivingPackMutation::Delete, LivingStoreResultV2::Deleted) => {
                            self.packs.retain(|pack| pack.hash != descriptor.hash);
                            self.pack_references.remove(&descriptor.hash);
                        }
                        (
                            LivingPackMutation::SetArchived(archived),
                            LivingStoreResultV2::Pack(pack),
                        ) if pack.hash == descriptor.hash
                            && pack.archived == archived
                            && pack.generation.0 > descriptor.generation.0 =>
                        {
                            let cached = self
                                .packs
                                .iter_mut()
                                .find(|row| row.hash == pack.hash)
                                .ok_or("Pack cache changed during mutation")?;
                            *cached = pack;
                        }
                        _ => return Err("Living pack mutation protocol mismatch".into()),
                    }
                    Ok(LivingClientEvent::PackMutated)
                }
                LivingStorePollV2::Failed(error) => {
                    self.store.release(self.epoch, job);
                    Err(if error == LivingStoreErrorV2::Referenced {
                        format!(
                            "Catalog {} is still referenced by active or archived retained galaxy history; no pack changed",
                            encoded(&descriptor.hash)
                        )
                    } else {
                        format!("Exact catalog pack operation failed: {error}")
                    })
                }
            },
            Phase::Mutation(job) => match self.store.poll(self.epoch, job, 1024) {
                LivingStorePollV2::Pending { .. } => {
                    self.phase = Phase::Mutation(job);
                    Ok(LivingClientEvent::Pending)
                }
                LivingStorePollV2::Ready(_) => {
                    self.store.release(self.epoch, job);
                    if self.mutation_clears_selection {
                        // Store and coordinator are separate authorities. Only clear
                        // Continue after the exact requested mutation committed.
                        self.select_neutral(None)?;
                        Ok(LivingClientEvent::Pending)
                    } else {
                        Ok(LivingClientEvent::Mutated)
                    }
                }
                LivingStorePollV2::Failed(error) => {
                    self.store.release(self.epoch, job);
                    Err(error.to_string())
                }
            },
            Phase::ReadCatalog { job, archive } => match self.store.poll(self.epoch, job, 1024) {
                LivingStorePollV2::Pending { .. } => {
                    self.phase = Phase::ReadCatalog { job, archive };
                    Ok(LivingClientEvent::Pending)
                }
                LivingStorePollV2::Ready(LivingStoreResultV2::Bytes {
                    canonical_bytes, ..
                }) => {
                    self.store.release(self.epoch, job);
                    Ok(LivingClientEvent::ArchiveCatalog {
                        archive,
                        catalog: canonical_bytes,
                    })
                }
                LivingStorePollV2::Failed(error) => {
                    self.store.release(self.epoch, job);
                    Ok(LivingClientEvent::CatalogUnavailable {
                        archive,
                        error: error.to_string(),
                    })
                }
                _ => {
                    self.store.release(self.epoch, job);
                    Err("Living catalog read protocol mismatch".into())
                }
            },
        }
    }
}
