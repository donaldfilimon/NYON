//! Private preparation of one coherent portable file set and notebook imports.
use super::*;
use crate::living_transfer::LivingTransferKind;
use nyon_workshop_core::living::*;
use std::rc::Rc;

pub(super) struct PreparedLivingExport {
    pub archive: Vec<u8>,
    pub catalog: Vec<u8>,
    pub notebook: Vec<u8>,
    pub status: [ExportFileStatus; 3],
    bookmark: LivingBookmarkV2,
    generation: u64,
    scope: Rc<()>,
}
#[derive(Clone, Copy)]
pub(super) enum ExportFileStatus {
    Prepared,
    DownloadStarted,
    Confirmed,
}
pub(super) enum LivingNotebookJob {
    Archive {
        replay: Box<LivingArchiveReplayV2>,
        data: NotebookJobData,
    },
    Notebook {
        replay: Box<LivingNotebookReplayV2>,
        data: NotebookJobData,
        bytes: Vec<u8>,
    },
}
pub(super) struct NotebookJobData {
    archive: Vec<u8>,
    catalog: Vec<u8>,
    bookmarks: Vec<LivingNamedBookmarkV2>,
    experiments: Vec<LivingNotebookExperimentV2>,
    evidence: Option<LivingNotebookEvidenceV2>,
    imported: Option<Vec<u8>>,
    verify_only: bool,
    bookmark: LivingBookmarkV2,
    generation: u64,
    scope: Rc<()>,
}
impl<S: ScenarioStore, P: PreferencesStore, W: WorkshopStore> ClientRuntime<S, P, W> {
    pub(super) fn prepare_living_export(&mut self) -> Result<(), String> {
        if self.living_transfer_hold.is_some() || self.living_notebook_job.is_some() {
            return Err("A Living file operation is already active".into());
        }
        self.begin_living_notebook_job(None)?;
        self.living_export_bundle = None;
        self.living_storage_status = "Preparing exact galaxy, catalog and notebook files".into();
        Ok(())
    }
    pub(super) fn verify_living_notebook(&mut self) -> Result<(), String> {
        if self.living_transfer_hold.is_some() || self.living_notebook_job.is_some() {
            return Err("A Living file operation is already active".into());
        }
        self.begin_living_notebook_job(None)?;
        if let Some(LivingNotebookJob::Archive { data, .. }) = &mut self.living_notebook_job {
            data.verify_only = true;
        }
        self.living_storage_status = "Replaying exact notebook observations".into();
        Ok(())
    }
    pub(super) fn begin_living_notebook_job(
        &mut self,
        imported: Option<Vec<u8>>,
    ) -> Result<(), String> {
        let session = self.sandbox.as_mut().ok_or("No resident Living galaxy")?;
        let archive = session.export_bytes()?;
        let replay = begin_decode_living_archive_v2(&archive, session.authority().catalog())
            .map_err(|error| error.to_string())?;
        let (bookmarks, experiments) = session.notebook_snapshot();
        let data = NotebookJobData {
            archive,
            catalog: session.authority().catalog().canonical_bytes().to_vec(),
            bookmarks,
            experiments,
            evidence: session.notebook_evidence_snapshot(),
            imported,
            verify_only: false,
            bookmark: session.bookmark()?,
            generation: session.notebook_generation(),
            scope: session.presentation_scope(),
        };
        if self.living_transfer_hold.is_none() {
            self.living_transfer_hold = Some(session.begin_hold()?);
        }
        self.living_notebook_job = Some(LivingNotebookJob::Archive {
            replay: Box::new(replay),
            data,
        });
        Ok(())
    }
    pub(super) fn prepared_export_bytes(
        &self,
        kind: LivingTransferKind,
    ) -> Result<Vec<u8>, String> {
        let bundle = self
            .living_export_bundle
            .as_ref()
            .ok_or("Prepare the file set before exporting")?;
        let session = self.sandbox.as_ref().ok_or("No resident Living galaxy")?;
        if !Rc::ptr_eq(&bundle.scope, &session.presentation_scope())
            || bundle.bookmark != session.bookmark()?
            || bundle.generation != session.notebook_generation()
        {
            return Err("Galaxy or notebook changed; prepare a fresh file set".into());
        }
        Ok(match kind {
            LivingTransferKind::Archive => bundle.archive.clone(),
            LivingTransferKind::Catalog => bundle.catalog.clone(),
            LivingTransferKind::Notebook => bundle.notebook.clone(),
        })
    }
    pub(super) fn confirm_living_export(&mut self, index: u8) -> Result<(), String> {
        let bundle = self
            .living_export_bundle
            .as_mut()
            .ok_or("No prepared file set")?;
        let status = bundle
            .status
            .get_mut(usize::from(index))
            .ok_or("Unknown export file")?;
        if !matches!(status, ExportFileStatus::DownloadStarted) {
            return Err("Only a browser download that started can be confirmed".into());
        }
        *status = ExportFileStatus::Confirmed;
        self.living_storage_status = "Download completion recorded from your confirmation".into();
        Ok(())
    }
    pub(super) fn export_projection(&self) -> (bool, String, [bool; 3]) {
        let Some(bundle) = &self.living_export_bundle else {
            return (
                false,
                "Prepare a coherent galaxy, catalog and optional notebook file set.".into(),
                [false; 3],
            );
        };
        let current = self.sandbox.as_ref().is_some_and(|session| {
            Rc::ptr_eq(&bundle.scope, &session.presentation_scope())
                && session.bookmark().ok() == Some(bundle.bookmark)
                && session.notebook_generation() == bundle.generation
        });
        let names = ["Galaxy", "Catalog", "Notebook"];
        let status = names
            .iter()
            .zip(bundle.status)
            .map(|(name, status)| {
                format!(
                    "{name}: {}",
                    match status {
                        ExportFileStatus::Prepared => "prepared",
                        ExportFileStatus::DownloadStarted =>
                            "download started; disk completion unconfirmed",
                        ExportFileStatus::Confirmed => "saved or completion confirmed",
                    }
                )
            })
            .collect::<Vec<_>>()
            .join(". ");
        (
            current,
            status,
            bundle
                .status
                .map(|status| matches!(status, ExportFileStatus::DownloadStarted)),
        )
    }
    pub(super) fn poll_living_notebook_job(&mut self) {
        let Some(job) = self.living_notebook_job.take() else {
            return;
        };
        let result = self.advance_living_notebook_job(job);
        if let Err(error) = result {
            self.living_storage_status = format!("Notebook operation failed: {error}");
            self.release_living_transfer_hold();
        }
    }
    fn advance_living_notebook_job(&mut self, job: LivingNotebookJob) -> Result<(), String> {
        match job {
            LivingNotebookJob::Archive { mut replay, data } => {
                if replay.poll(128).map_err(|error| error.to_string())?
                    == LivingArchiveReplayStatusV2::Pending
                {
                    self.living_notebook_job = Some(LivingNotebookJob::Archive { replay, data });
                    return Ok(());
                }
                let archive = (*replay).finish().map_err(|error| error.to_string())?;
                let bytes = if let Some(bytes) = &data.imported {
                    bytes.clone()
                } else {
                    LivingNotebookV2::seal_with_evidence(
                        &archive,
                        data.bookmarks.clone(),
                        data.experiments.clone(),
                        data.evidence.clone(),
                    )
                    .and_then(|notebook| notebook.canonical_bytes())
                    .map_err(|error| error.to_string())?
                };
                let replay = begin_decode_living_notebook_v2(&bytes, archive)
                    .map_err(|error| error.to_string())?;
                self.living_notebook_job = Some(LivingNotebookJob::Notebook {
                    replay: Box::new(replay),
                    data,
                    bytes,
                });
            }
            LivingNotebookJob::Notebook {
                mut replay,
                data,
                bytes,
            } => {
                if replay.poll(128).map_err(|error| error.to_string())?
                    == LivingArchiveReplayStatusV2::Pending
                {
                    self.living_notebook_job = Some(LivingNotebookJob::Notebook {
                        replay,
                        data,
                        bytes,
                    });
                    return Ok(());
                }
                let validated = (*replay).finish().map_err(|error| error.to_string())?;
                let session = self.sandbox.as_mut().ok_or("Resident galaxy disappeared")?;
                if !Rc::ptr_eq(&data.scope, &session.presentation_scope())
                    || session.bookmark()? != data.bookmark
                    || session.notebook_generation() != data.generation
                {
                    return Err(
                        "Resident changed during notebook validation; current session preserved"
                            .into(),
                    );
                }
                if data.verify_only {
                    session.accept_notebook_validation(&validated)?;
                    self.living_storage_status = format!(
                        "Notebook observations replay-verified; {}; {} matching supply observations; {} matching corridor observations",
                        if validated.trade_completions().is_empty() {
                            "no matching trade pair established"
                        } else {
                            "matching trade evidence is available"
                        },
                        validated.supply_completions().len(),
                        validated.corridor_observations().len(),
                    );
                } else if data.imported.is_some() {
                    session.import_notebook(validated)?;
                    self.living_export_bundle = None;
                    self.living_storage_status =
                        "Exact notebook imported; save to preserve its metadata".into();
                } else {
                    self.living_export_bundle = Some(PreparedLivingExport {
                        archive: data.archive,
                        catalog: data.catalog,
                        notebook: bytes,
                        status: [ExportFileStatus::Prepared; 3],
                        bookmark: data.bookmark,
                        generation: data.generation,
                        scope: data.scope,
                    });
                    self.living_storage_status =
                        "File set prepared; export each file to hand it to your platform".into();
                }
                self.release_living_transfer_hold();
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        living_transfer::*,
        preferences::store::MemoryPreferencesStore,
        sandbox_session::SandboxUiAction,
        scenario::{ScenarioDraft, store::MemoryScenarioStore},
        workshop::store::MemoryWorkshopStore,
    };
    use std::cell::RefCell;
    struct Handoff {
        requests: Rc<RefCell<Vec<LivingTransferRequest>>>,
        pending: bool,
    }
    impl LivingTransfer for Handoff {
        fn begin(&mut self, request: LivingTransferRequest) -> Result<(), LivingTransferError> {
            request.validate()?;
            self.requests.borrow_mut().push(request);
            self.pending = true;
            Ok(())
        }
        fn poll(&mut self) -> LivingTransferStatus {
            if std::mem::take(&mut self.pending) {
                LivingTransferStatus::Ready(Ok(LivingTransferOutcome::Exported(
                    LivingExportOutcome::DownloadStarted,
                )))
            } else {
                LivingTransferStatus::Idle
            }
        }
        fn cancel(&mut self) {
            self.pending = false;
        }
    }
    type Runtime = ClientRuntime<MemoryScenarioStore, MemoryPreferencesStore, MemoryWorkshopStore>;
    fn runtime() -> (Runtime, Rc<RefCell<Vec<LivingTransferRequest>>>) {
        let classic = AppCore::new_with_preferences(
            ScenarioDraft::factory_default().validated().unwrap(),
            MemoryScenarioStore::default(),
            MemoryPreferencesStore::default(),
        );
        let mut runtime = ClientRuntime::new(classic, MemoryWorkshopStore::default());
        runtime
            .select_menu_route(MainMenuRoute::GalaxySandbox)
            .unwrap();
        let requests = Rc::new(RefCell::new(Vec::new()));
        runtime.install_living_transfer(Box::new(Handoff {
            requests: requests.clone(),
            pending: false,
        }));
        (runtime, requests)
    }
    fn prepare(runtime: &mut Runtime) {
        runtime.prepare_living_export().unwrap();
        for _ in 0..1000 {
            runtime.update(Duration::ZERO);
            if runtime.living_notebook_job.is_none() {
                break;
            }
        }
        assert!(runtime.living_notebook_job.is_none());
        assert!(
            runtime.living_export_bundle.is_some(),
            "{}",
            runtime.living_storage_status
        );
    }
    #[test]
    fn local_verification_preserves_metadata_dirty_state_and_canonical_history() {
        let (mut runtime, requests) = runtime();
        runtime.sandbox = Some(crate::sandbox_session::SandboxSession::from_authority(
            crate::living_content::LivingContent::Commerce
                .open()
                .unwrap(),
        ));
        let session = runtime.sandbox.as_mut().unwrap();
        let mut formed = false;
        let mut delivered = false;
        for _ in 0..200 {
            session.step_once().unwrap();
            let receipt = session.receipts().back().unwrap();
            let formation = receipt
                .events
                .iter()
                .any(|event| event.kind == LivingEventKindV2::AgreementFormed);
            let delivery = receipt
                .events
                .iter()
                .any(|event| event.kind == LivingEventKindV2::ShipmentDelivered);
            if (!formed && formation) || (!delivered && delivery) {
                session
                    .apply_action(SandboxUiAction::RecordExperiment(2))
                    .unwrap();
                formed |= formation;
                delivered |= delivery;
            }
            if formed && delivered {
                break;
            }
        }
        assert!(formed && delivered);
        let before = (
            session.export_bytes().unwrap(),
            session.notebook_snapshot(),
            session.notebook_evidence_snapshot(),
            session.notebook_generation(),
            session.dirty(),
        );
        runtime.apply_sandbox_action(SandboxUiAction::VerifyNotebookEvidence);
        assert!(runtime.living_notebook_job.is_some());
        for _ in 0..10000 {
            runtime.update(Duration::ZERO);
            if runtime.living_notebook_job.is_none() {
                break;
            }
        }
        assert!(runtime.living_notebook_job.is_none());
        assert!(runtime.living_transfer_hold.is_none());
        assert!(
            runtime.living_storage_status.contains("replay-verified"),
            "{}",
            runtime.living_storage_status
        );
        let session = runtime.sandbox.as_mut().unwrap();
        assert_eq!(
            before,
            (
                session.export_bytes().unwrap(),
                session.notebook_snapshot(),
                session.notebook_evidence_snapshot(),
                session.notebook_generation(),
                session.dirty()
            )
        );
        session
            .apply_action(SandboxUiAction::NotebookPicker)
            .unwrap();
        session
            .apply_action(SandboxUiAction::ExperimentDetails(Some(2)))
            .unwrap();
        let mut labels = String::new();
        for _ in 0..100 {
            let frame = session.frame(glam::Vec2::new(800.0, 480.0), false);
            labels.extend(
                frame
                    .controls
                    .iter()
                    .filter(|control| {
                        control
                            .action_id
                            .as_str()
                            .starts_with("sandbox.notebook.fact.trade")
                    })
                    .map(|control| control.label.as_str()),
            );
            if !frame.controls.iter().any(|control| {
                control.action_id.as_str() == "sandbox.notebook.next" && control.enabled
            }) {
                break;
            }
            session
                .apply_action(SandboxUiAction::InspectorPage(false))
                .unwrap();
        }
        assert!(
            labels.contains("Agreement and matching delivery observed"),
            "{labels}"
        );
        assert!(requests.borrow().is_empty());
        assert!(runtime.living_export_bundle.is_none());
    }
    #[test]
    fn stale_or_cancelled_local_verification_preserves_current_notebook() {
        let (mut runtime, requests) = runtime();
        runtime.verify_living_notebook().unwrap();
        runtime
            .sandbox
            .as_mut()
            .unwrap()
            .mark_notebook_unsaved()
            .unwrap();
        for _ in 0..1000 {
            runtime.update(Duration::ZERO);
            if runtime.living_notebook_job.is_none() {
                break;
            }
        }
        assert!(runtime.living_storage_status.contains("Resident changed"));
        assert_eq!(runtime.sandbox.as_ref().unwrap().notebook_generation(), 1);
        assert!(runtime.living_transfer_hold.is_none());
        runtime.verify_living_notebook().unwrap();
        runtime.cancel_living_import();
        assert!(runtime.living_notebook_job.is_none());
        assert!(runtime.living_transfer_hold.is_none());
        assert_eq!(runtime.sandbox.as_ref().unwrap().notebook_generation(), 1);
        assert!(requests.borrow().is_empty());
    }
    #[test]
    fn local_verification_cannot_publish_into_replacement_resident() {
        let (mut runtime, requests) = runtime();
        let old_bookmark = runtime.sandbox.as_ref().unwrap().bookmark().unwrap();
        runtime.verify_living_notebook().unwrap();
        runtime.sandbox = Some(crate::sandbox_session::SandboxSession::from_authority(
            crate::living_content::LivingContent::ValeConfluence
                .open()
                .unwrap(),
        ));
        assert_eq!(
            runtime.sandbox.as_ref().unwrap().bookmark().unwrap(),
            old_bookmark
        );
        for _ in 0..1000 {
            runtime.update(Duration::ZERO);
            if runtime.living_notebook_job.is_none() {
                break;
            }
        }
        assert!(runtime.living_storage_status.contains("Resident changed"));
        assert!(runtime.living_transfer_hold.is_none());
        assert_eq!(runtime.sandbox.as_ref().unwrap().notebook_generation(), 0);
        assert!(requests.borrow().is_empty());
    }
    #[test]
    fn verification_without_matching_evidence_never_reports_trade_completion() {
        let (mut runtime, requests) = runtime();
        let before = runtime.sandbox.as_ref().unwrap().notebook_generation();
        runtime.verify_living_notebook().unwrap();
        for _ in 0..1000 {
            runtime.update(Duration::ZERO);
            if runtime.living_notebook_job.is_none() {
                break;
            }
        }
        assert!(runtime.living_notebook_job.is_none());
        assert!(
            runtime
                .living_storage_status
                .contains("no matching trade pair established")
        );
        assert_eq!(
            runtime.sandbox.as_ref().unwrap().notebook_generation(),
            before
        );
        assert!(runtime.living_transfer_hold.is_none());
        assert!(requests.borrow().is_empty());
    }
    #[test]
    fn prepared_file_set_has_exact_attachment_and_handoffs_are_not_disk_confirmations() {
        let (mut runtime, requests) = runtime();
        assert!(
            runtime
                .start_living_transfer(LivingTransferKind::Archive, true)
                .is_err()
        );
        assert!(requests.borrow().is_empty());
        prepare(&mut runtime);
        let bundle = runtime.living_export_bundle.as_ref().unwrap();
        let catalog = decode_living_catalog_pack_v2(&bundle.catalog).unwrap();
        let archive = decode_living_archive_v2(&bundle.archive, &catalog).unwrap();
        let mut notebook = begin_decode_living_notebook_v2(&bundle.notebook, archive).unwrap();
        while notebook.poll(128).unwrap() == LivingArchiveReplayStatusV2::Pending {}
        notebook.finish().unwrap();
        for kind in [
            LivingTransferKind::Archive,
            LivingTransferKind::Catalog,
            LivingTransferKind::Notebook,
        ] {
            runtime.start_living_transfer(kind, true).unwrap();
            runtime.update(Duration::ZERO);
        }
        assert_eq!(requests.borrow().len(), 3);
        assert_eq!(runtime.export_projection().2, [true; 3]);
        runtime.confirm_living_export(1).unwrap();
        assert_eq!(runtime.export_projection().2, [true, false, true]);
        assert!(runtime.confirm_living_export(1).is_err());
    }
    #[test]
    fn later_metadata_changes_invalidate_prepared_files_without_handoff() {
        let (mut runtime, requests) = runtime();
        prepare(&mut runtime);
        runtime.apply_sandbox_action(SandboxUiAction::RecordBookmark);
        assert!(!runtime.export_projection().0);
        assert!(
            runtime
                .start_living_transfer(LivingTransferKind::Notebook, true)
                .is_err()
        );
        assert!(requests.borrow().is_empty());
    }
    #[test]
    fn cancelling_private_preparation_preserves_resident_and_releases_hold() {
        let (mut runtime, requests) = runtime();
        let before = runtime.sandbox.as_ref().unwrap().bookmark().unwrap();
        runtime.prepare_living_export().unwrap();
        runtime.cancel_living_import();
        assert!(runtime.living_notebook_job.is_none());
        assert!(runtime.living_transfer_hold.is_none());
        assert_eq!(
            runtime.sandbox.as_ref().unwrap().bookmark().unwrap(),
            before
        );
        assert!(requests.borrow().is_empty());
    }
    #[test]
    fn adapter_that_becomes_idle_after_cancel_releases_transport_hold() {
        let (mut runtime, _) = runtime();
        prepare(&mut runtime);
        runtime
            .start_living_transfer(LivingTransferKind::Archive, true)
            .unwrap();
        runtime.cancel_living_import();
        assert!(runtime.living_transfer_hold.is_some());
        runtime.update(Duration::ZERO);
        assert!(runtime.living_transfer_hold.is_none());
        assert!(!runtime.living_transfer_cancelling);
        runtime.apply_sandbox_action(SandboxUiAction::Step);
        for _ in 0..100 {
            runtime.update(Duration::ZERO);
        }
        assert_eq!(runtime.sandbox_tick(), Some(1));
    }
    struct PlacementWon {
        polls: u8,
    }
    impl LivingTransfer for PlacementWon {
        fn begin(&mut self, _: LivingTransferRequest) -> Result<(), LivingTransferError> {
            self.polls = 2;
            Ok(())
        }
        fn poll(&mut self) -> LivingTransferStatus {
            if self.polls == 0 {
                return LivingTransferStatus::Idle;
            }
            self.polls -= 1;
            if self.polls != 0 {
                return LivingTransferStatus::Pending;
            }
            LivingTransferStatus::Ready(Ok(LivingTransferOutcome::Imported {
                kind: LivingTransferKind::Notebook,
                bytes: b"not a valid notebook".to_vec(),
            }))
        }
        fn cancel(&mut self) {}
    }
    #[test]
    fn cancellation_waits_for_pending_handoff_and_discards_late_import() {
        let (mut runtime, _) = runtime();
        let session = runtime.sandbox.as_mut().unwrap();
        let before = session.bookmark().unwrap();
        assert!(session.mark_saved(before));
        runtime.install_living_transfer(Box::new(PlacementWon { polls: 0 }));
        runtime
            .start_living_transfer(LivingTransferKind::Notebook, false)
            .unwrap();
        runtime.cancel_living_import();
        runtime.update(Duration::ZERO);
        assert!(runtime.living_transfer_hold.is_some());
        runtime.apply_sandbox_action(SandboxUiAction::Step);
        for _ in 0..100 {
            runtime.update(Duration::ZERO);
        }
        assert_eq!(runtime.sandbox_tick(), Some(0));
        runtime.update(Duration::ZERO);
        assert!(runtime.living_transfer_hold.is_none());
        assert!(runtime.living_notebook_job.is_none());
        assert_eq!(runtime.sandbox.as_ref().unwrap().notebook_generation(), 0);
        assert_eq!(
            runtime.sandbox.as_ref().unwrap().bookmark().unwrap(),
            before
        );
        runtime.apply_sandbox_action(SandboxUiAction::Step);
        for _ in 0..100 {
            runtime.update(Duration::ZERO);
        }
        assert_eq!(runtime.sandbox_tick(), Some(1));
    }
}
