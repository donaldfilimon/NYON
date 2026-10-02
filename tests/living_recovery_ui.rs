//! Public creator recovery flow; native-store coverage is not live platform UI evidence.
#![cfg(not(target_arch = "wasm32"))]
use glam::Vec2;
use nyon::{
    app::client_runtime::living::{LivingClient, LivingClientEvent},
    library_selection::native::NativeLibraryCoordinator,
    living_creator::LivingCreatorTool as Tool,
    living_store::native::NativeLivingStoreV2,
    sandbox_session::{SandboxSession, SandboxUiAction as A},
};
use nyon_workshop_core::living::*;
use std::time::{Duration, Instant};

fn edit(session: &mut SandboxSession, tool: Tool) {
    session.apply_action(A::OpenCreator(tool)).unwrap();
    session.apply_action(A::CreatorPreview).unwrap();
    session.apply_action(A::CreatorApply).unwrap();
}
fn dormant() -> SandboxSession {
    let mut session = SandboxSession::from_authority(builtin_first_expansion_v2().unwrap());
    edit(&mut session, Tool::RemoveColony);
    assert!(session.state().colonies.as_slice().is_empty());
    assert!(
        session
            .state()
            .civilizations
            .as_slice()
            .iter()
            .all(|actor| actor.status == LivingCivilizationStatusV2::Dormant)
    );
    session
}
fn terminal(client: &mut LivingClient) -> LivingClientEvent {
    let deadline = Instant::now() + Duration::from_secs(30);
    loop {
        let event = client.poll();
        if !matches!(event, LivingClientEvent::Pending) {
            return event;
        }
        assert!(
            Instant::now() < deadline,
            "native recovery Notebook job timed out"
        );
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
        frame.semantics.validate().unwrap();
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
            .any(|control| control.action_id.as_str() == "sandbox.notebook.next" && control.enabled)
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
fn creator_recovery_preview_apply_record_save_continue_and_visible_replay_reference() {
    let mut session = dormant();
    edit(&mut session, Tool::Colony);
    let receipt = session.receipts().back().unwrap().clone();
    assert!(
        !receipt
            .events
            .iter()
            .any(|event| event.kind == LivingEventKindV2::CivilizationReactivated)
    );
    assert_eq!(session.authority().last_creator_recoveries().len(), 1);
    session.apply_action(A::RecordExperiment(5)).unwrap();
    let before = session.bookmark().unwrap();
    let evidence = session.notebook_evidence_snapshot().unwrap();
    assert_eq!(evidence.records.len(), 1);
    assert_eq!(
        evidence.records[0].experiment,
        LivingNotebookExperimentKindV2::DormantRecovery
    );
    assert!(
        evidence.records[0]
            .events
            .iter()
            .all(|event| event.kind == LivingEventKindV2::CreatorIntervention)
    );
    let local = visible_notebook(&mut session);
    assert!(
        local.contains("Current authoritative observation DormantRecovery"),
        "{local}"
    );
    session.apply_action(A::CreatorCancel).unwrap();
    let root = tempfile::tempdir().unwrap();
    let mut writer = client(root.path());
    writer.read_selection().unwrap();
    assert!(matches!(
        terminal(&mut writer),
        LivingClientEvent::AbsentSelection
    ));
    let (bookmarks, experiments) = session.notebook_snapshot();
    writer
        .save_with_notebook_evidence(
            session.authority(),
            bookmarks,
            experiments,
            Some(evidence.clone()),
            true,
        )
        .unwrap();
    assert!(
        matches!(terminal(&mut writer), LivingClientEvent::Saved { bookmark, leave: true } if bookmark == before)
    );
    drop(writer);
    let mut reader = client(root.path());
    reader.read_selection().unwrap();
    let LivingClientEvent::ReadyWithNotebook {
        authority,
        notebook,
    } = terminal(&mut reader)
    else {
        panic!("Continue did not restore creator recovery evidence")
    };
    let mut reopened = SandboxSession::from_authority(*authority);
    reopened.install_notebook(notebook).unwrap();
    assert_eq!(reopened.bookmark().unwrap(), before);
    // Opening projects only the authenticated latest replay boundary.
    assert_eq!(reopened.receipts().len(), 1);
    let latest = &reopened.receipts()[0];
    assert_eq!(latest, reopened.authority().last_receipt().unwrap());
    assert_eq!(latest.tick.0, reopened.tick());
    assert_eq!(latest.state_digest, reopened.state().digest().unwrap());
    assert_eq!(
        reopened.notebook_evidence_snapshot(),
        Some(evidence.clone())
    );
    let text = visible_notebook(&mut reopened);
    assert!(
        text.contains("Replay-verified observation DormantRecovery"),
        "{text}"
    );
    assert!(text.contains("creator_intervention"), "{text}");
    assert!(text.contains(&hex(&receipt.digest.0)), "{text}");
    for event in &evidence.records[0].events {
        assert!(text.contains(&hex(&event.id.0)), "{text}");
    }
    assert!(
        text.contains("No broader completion or causal claim"),
        "{text}"
    );
}

#[test]
fn unrelated_creator_inventory_edit_does_not_record_recovery_or_mutate_notebook() {
    let mut session = dormant();
    edit(&mut session, Tool::Inventory);
    let before = session.bookmark().unwrap();
    let bytes = encode_living_archive_v2(session.authority()).unwrap();
    let notes = session.notebook_snapshot();
    let generation = session.notebook_generation();
    assert!(session.authority().last_creator_recoveries().is_empty());
    assert!(session.apply_action(A::RecordExperiment(5)).is_err());
    assert_eq!(session.bookmark().unwrap(), before);
    assert_eq!(
        encode_living_archive_v2(session.authority()).unwrap(),
        bytes
    );
    assert_eq!(session.notebook_snapshot(), notes);
    assert_eq!(session.notebook_generation(), generation);
    assert!(session.notebook_evidence_snapshot().is_none());
}
