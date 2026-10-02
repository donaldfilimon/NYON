//! Public player metadata journey; host-native persistence is not live GUI evidence.
#![cfg(not(target_arch = "wasm32"))]
use glam::Vec2;
use nyon::{
    app::client_runtime::living::{LivingClient, LivingClientEvent},
    library_selection::native::NativeLibraryCoordinator,
    living_store::native::NativeLivingStoreV2,
    sandbox_session::{SandboxSession, SandboxUiAction as A},
};
use nyon_workshop_core::living::*;
use std::time::{Duration, Instant};

fn settlement() -> SandboxSession {
    let mut session = SandboxSession::from_authority(builtin_first_expansion_v2().unwrap());
    for _ in 0..361 {
        session.step_once().unwrap();
    }
    session.apply_action(A::RecordExperiment(1)).unwrap();
    session
}
fn portable(
    session: &SandboxSession,
    evidence: Option<LivingNotebookEvidenceV2>,
) -> ValidatedLivingNotebookV2 {
    let archive = decode_living_archive_v2(
        &encode_living_archive_v2(session.authority()).unwrap(),
        session.authority().catalog(),
    )
    .unwrap();
    let (bookmarks, experiments) = session.notebook_snapshot();
    let notebook =
        LivingNotebookV2::seal_with_evidence(&archive, bookmarks, experiments, evidence).unwrap();
    let mut job =
        begin_decode_living_notebook_v2(&notebook.canonical_bytes().unwrap(), archive).unwrap();
    while job.poll(128).unwrap() == LivingArchiveReplayStatusV2::Pending {}
    job.finish().unwrap()
}
fn terminal(client: &mut LivingClient) -> LivingClientEvent {
    let deadline = Instant::now() + Duration::from_secs(30);
    loop {
        let event = client.poll();
        if !matches!(event, LivingClientEvent::Pending) {
            return event;
        }
        assert!(Instant::now() < deadline, "native notebook job timed out");
        std::thread::sleep(Duration::from_millis(1));
    }
}
fn client(root: &std::path::Path) -> LivingClient {
    LivingClient::new(
        Box::new(NativeLivingStoreV2::new(root)),
        Box::new(NativeLibraryCoordinator::new(root)),
    )
}
fn visible_notebook(session: &mut SandboxSession) -> String {
    session.apply_action(A::NotebookPicker).unwrap();
    session.apply_action(A::ExperimentDetails(None)).unwrap();
    let mut text = String::new();
    for _ in 0..100 {
        let frame = session.frame(Vec2::new(800.0, 480.0), false);
        for control in &frame.controls {
            if control.action_id.as_str().starts_with("sandbox.notebook.")
                && !matches!(
                    control.action_id.as_str(),
                    "sandbox.notebook.previous"
                        | "sandbox.notebook.next"
                        | "sandbox.notebook.close"
                )
            {
                text.push_str(&control.label);
            }
        }
        if !frame
            .controls
            .iter()
            .any(|c| c.action_id.as_str() == "sandbox.notebook.next" && c.enabled)
        {
            return text;
        }
        session.apply_action(A::InspectorPage(false)).unwrap();
    }
    panic!("notebook pages did not terminate")
}
fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}

#[test]
fn real_settlement_evidence_survives_portable_decode_and_native_save_continue_without_receipt_cache()
 {
    let source = settlement();
    let expected = source.bookmark().unwrap();
    let evidence = source.notebook_evidence_snapshot().unwrap();
    assert_eq!(evidence.records.len(), 1);
    let record = &evidence.records[0];
    assert_eq!(
        record.experiment,
        LivingNotebookExperimentKindV2::AutonomousSettlement
    );
    assert!(
        record
            .events
            .iter()
            .all(|event| event.kind == LivingEventKindV2::SettlementClaimResolved)
    );
    assert!(!record.events.is_empty());
    let mut imported = SandboxSession::from_authority(source.authority().clone());
    imported
        .import_notebook(portable(&source, Some(evidence.clone())))
        .unwrap();
    assert!(imported.dirty());
    assert_eq!(
        imported.notebook_evidence_snapshot(),
        Some(evidence.clone())
    );
    assert!(imported.bookmark_receipt(0).is_none());
    let root = tempfile::tempdir().unwrap();
    let mut writer = client(root.path());
    writer.read_selection().unwrap();
    assert!(matches!(
        terminal(&mut writer),
        LivingClientEvent::AbsentSelection
    ));
    let (bookmarks, experiments) = source.notebook_snapshot();
    writer
        .save_with_notebook_evidence(
            source.authority(),
            bookmarks,
            experiments,
            Some(evidence.clone()),
            true,
        )
        .unwrap();
    assert!(
        matches!(terminal(&mut writer), LivingClientEvent::Saved { bookmark, leave: true } if bookmark == expected)
    );
    drop(writer);
    let mut reader = client(root.path());
    reader.read_selection().unwrap();
    let LivingClientEvent::ReadyWithNotebook {
        authority,
        notebook,
    } = terminal(&mut reader)
    else {
        panic!("Continue did not restore authority and evidence attachment")
    };
    let mut reopened = SandboxSession::from_authority(*authority);
    reopened.install_notebook(notebook).unwrap();
    assert_eq!(reopened.bookmark().unwrap(), expected);
    assert_eq!(
        reopened.notebook_evidence_snapshot(),
        Some(evidence.clone())
    );
    // Opening projects only the authenticated latest replay boundary.
    assert_eq!(reopened.receipts().len(), 1);
    let latest = &reopened.receipts()[0];
    assert_eq!(latest, reopened.authority().last_receipt().unwrap());
    assert_eq!(latest.tick.0, reopened.tick());
    assert_eq!(latest.state_digest, reopened.state().digest().unwrap());
    assert!(reopened.bookmark_receipt(0).is_none());
    let text = visible_notebook(&mut reopened);
    assert!(
        text.contains("Replay-verified observation AutonomousSettlement"),
        "{text}"
    );
    assert!(text.contains(&hex(&record.receipt_digest.0)), "{text}");
    for event in &record.events {
        assert!(text.contains(&hex(&event.id.0)), "{text}");
    }
    assert!(
        text.contains("No broader completion or causal claim"),
        "{text}"
    );
}

#[test]
fn legacy_empty_import_preserves_evidence_allocator_and_same_tick_edit_rejects_old_witness_atomically()
 {
    let mut source = settlement();
    let highwater = source.notebook_evidence_snapshot().unwrap().next_ordinal;
    let legacy = portable(&source, None);
    source.import_notebook(legacy).unwrap();
    let empty = source.notebook_evidence_snapshot().unwrap();
    assert!(empty.records.is_empty());
    assert_eq!(empty.next_ordinal, highwater);
    // An empty legacy attachment clears records but cannot recycle their identities.
    source.apply_action(A::RecordExperiment(1)).unwrap();
    let allocated = source.notebook_evidence_snapshot().unwrap();
    assert_eq!(allocated.records.len(), 1);
    assert_eq!(allocated.records[0].ordinal, highwater);
    assert_eq!(allocated.next_ordinal, highwater + 1);
    let validated = portable(&source, Some(allocated.clone()));
    assert_eq!(validated.notebook().evidence, Some(allocated));
    let mut session = settlement();
    // A paused creator boundary at the witness tick has a different state/revision.
    session
        .apply_action(A::OpenCreator(
            nyon::living_creator::LivingCreatorTool::Inventory,
        ))
        .unwrap();
    session.apply_action(A::CreatorPreview).unwrap();
    session.apply_action(A::CreatorApply).unwrap();
    assert_eq!(session.tick(), 361);
    let state = session.bookmark().unwrap();
    let archive = encode_living_archive_v2(session.authority()).unwrap();
    let metadata = session.notebook_snapshot();
    let evidence = session.notebook_evidence_snapshot();
    let generation = session.notebook_generation();
    assert!(session.apply_action(A::RecordExperiment(1)).is_err());
    assert_eq!(session.bookmark().unwrap(), state);
    assert_eq!(
        encode_living_archive_v2(session.authority()).unwrap(),
        archive
    );
    assert_eq!(session.notebook_snapshot(), metadata);
    assert_eq!(session.notebook_evidence_snapshot(), evidence);
    assert_eq!(session.notebook_generation(), generation);
}
