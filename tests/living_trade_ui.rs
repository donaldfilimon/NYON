//! Host player-journey evidence; this does not substitute for live native/IndexedDB checks.
#![cfg(not(target_arch = "wasm32"))]
use glam::Vec2;
use nyon::{
    app::client_runtime::living::{LivingClient, LivingClientEvent},
    library_selection::native::NativeLibraryCoordinator,
    living_content::LivingContent,
    living_store::native::NativeLivingStoreV2,
    sandbox_session::{SandboxSession, SandboxUiAction as A},
};
use nyon_workshop_core::living::*;
use std::time::{Duration, Instant};
// This is a test deadlock guard, not a player-facing latency requirement.
fn terminal(client: &mut LivingClient, stage: &str) -> LivingClientEvent {
    let started = Instant::now();
    let guard = Duration::from_secs(120);
    let mut polls = 0_u64;
    let mut next_report = Duration::from_secs(10);
    loop {
        let event = client.poll();
        polls += 1;
        let elapsed = started.elapsed();
        if !matches!(event, LivingClientEvent::Pending) {
            eprintln!("trade Notebook {stage}: terminal after {elapsed:?}, {polls} polls");
            return event;
        }
        assert!(
            elapsed < guard,
            "trade Notebook {stage}: still Pending after {elapsed:?}, {polls} polls (guard {guard:?})"
        );
        if elapsed >= next_report {
            eprintln!("trade Notebook {stage}: Pending after {elapsed:?}, {polls} polls");
            next_report = elapsed + Duration::from_secs(10);
        }
        std::thread::sleep(Duration::from_millis(1));
    }
}
fn client(root: &std::path::Path) -> LivingClient {
    LivingClient::new(
        Box::new(NativeLivingStoreV2::new(root)),
        Box::new(NativeLibraryCoordinator::new(root)),
    )
}
#[test]
fn record_formation_and_delivery_observations_save_and_continue_exact_replay() {
    let mut session = SandboxSession::from_authority(LivingContent::Commerce.open().unwrap());
    assert!(session.apply_action(A::RecordExperiment(2)).is_err());
    let mut formation = false;
    let mut delivery = false;
    for _ in 0..200 {
        session.step_once().unwrap();
        let receipt = session.receipts().back().unwrap();
        let formed = receipt
            .events
            .iter()
            .any(|event| event.kind == LivingEventKindV2::AgreementFormed);
        let delivered = receipt
            .events
            .iter()
            .any(|event| event.kind == LivingEventKindV2::ShipmentDelivered);
        if (!formation && formed) || (!delivery && delivered) {
            session.apply_action(A::RecordExperiment(2)).unwrap();
            formation |= formed;
            delivery |= delivered;
        }
        if formation && delivery {
            break;
        }
    }
    assert!(
        formation && delivery,
        "Commerce requires both separate receipt observations"
    );
    let evidence = session.notebook_evidence_snapshot().unwrap();
    assert_eq!(evidence.records.len(), 2);
    assert!(
        evidence
            .records
            .iter()
            .all(|record| record.experiment == LivingNotebookExperimentKindV2::TradeDelivery)
    );
    let kinds: Vec<_> = evidence
        .records
        .iter()
        .flat_map(|record| &record.events)
        .map(|event| event.kind)
        .collect();
    assert!(kinds.contains(&LivingEventKindV2::AgreementFormed));
    assert!(kinds.contains(&LivingEventKindV2::ShipmentDelivered));
    let before = session.bookmark().unwrap();
    let root = tempfile::tempdir().unwrap();
    let mut writer = client(root.path());
    writer.read_selection().unwrap();
    assert!(matches!(
        terminal(&mut writer, "initial selection"),
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
        matches!(terminal(&mut writer, "save evidence"), LivingClientEvent::Saved { bookmark, leave: true } if bookmark == before)
    );
    drop(writer);
    let mut reader = client(root.path());
    reader.read_selection().unwrap();
    let LivingClientEvent::ReadyWithNotebook {
        authority,
        notebook,
    } = terminal(&mut reader, "Continue replay")
    else {
        panic!("Continue did not replay trade evidence");
    };
    let mut reopened = SandboxSession::from_authority(*authority);
    reopened.install_notebook(notebook).unwrap();
    assert_eq!(reopened.bookmark().unwrap(), before);
    assert_eq!(reopened.notebook_evidence_snapshot(), Some(evidence));
    // Opening projects only the authenticated latest replay boundary.
    assert_eq!(reopened.receipts().len(), 1);
    let latest = &reopened.receipts()[0];
    assert_eq!(latest, reopened.authority().last_receipt().unwrap());
    assert_eq!(latest.tick.0, reopened.tick());
    assert_eq!(latest.state_digest, reopened.state().digest().unwrap());
    reopened.apply_action(A::NotebookPicker).unwrap();
    reopened
        .apply_action(A::ExperimentDetails(Some(2)))
        .unwrap();
    let mut visible = String::new();
    for _ in 0..100 {
        let frame = reopened.frame(Vec2::new(800.0, 480.0), false);
        frame.semantics.validate().unwrap();
        for control in &frame.controls {
            if control
                .action_id
                .as_str()
                .starts_with("sandbox.notebook.fact.trade")
                || control
                    .action_id
                    .as_str()
                    .starts_with("sandbox.notebook.trade.open")
            {
                visible.push_str(&control.label);
            }
        }
        if !frame
            .controls
            .iter()
            .any(|control| control.action_id.as_str() == "sandbox.notebook.next" && control.enabled)
        {
            break;
        }
        reopened.apply_action(A::InspectorPage(false)).unwrap();
    }
    assert!(
        visible.contains("Agreement and matching delivery observed"),
        "{visible}"
    );
    assert!(
        visible.contains("Open verified trade formation bookmark"),
        "{visible}"
    );
    assert!(
        visible.contains("Open verified trade delivery bookmark"),
        "{visible}"
    );
}
