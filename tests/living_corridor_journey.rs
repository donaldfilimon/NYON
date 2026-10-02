//! Host runtime/local-replay/native-store journey, separate from live browser acceptance.
#![cfg(not(target_arch = "wasm32"))]
use glam::Vec2;
use nyon::living_transfer::{
    LivingExportOutcome, LivingTransfer, LivingTransferError, LivingTransferKind,
    LivingTransferOutcome, LivingTransferRequest, LivingTransferStatus,
};
use nyon::{
    app::{
        AppCore,
        client_runtime::{
            ClientRuntime, MainMenuRoute,
            living::{LivingClient, LivingClientEvent},
        },
    },
    library_selection::native::NativeLibraryCoordinator,
    living_store::native::NativeLivingStoreV2,
    preferences::store::MemoryPreferencesStore,
    sandbox_session::SandboxUiAction as A,
    scenario::{ScenarioDraft, store::MemoryScenarioStore},
    workshop::store::MemoryWorkshopStore,
};
use nyon_workshop_core::living::*;
use std::{
    cell::RefCell,
    rc::Rc,
    time::{Duration, Instant},
};
type Runtime = ClientRuntime<MemoryScenarioStore, MemoryPreferencesStore, MemoryWorkshopStore>;
fn fixture() -> LivingGalaxyAuthorityV2 {
    fixture_with_energy(100)
}
fn fixture_with_energy(destination_energy: u32) -> LivingGalaxyAuthorityV2 {
    let base = builtin_first_expansion_v2().unwrap();
    let mut state = base.state().clone();
    let actor = state.civilizations.as_slice()[0].id;
    let source = state.colonies.as_slice()[0].world;
    let destination = state
        .worlds
        .as_slice()
        .iter()
        .find(|w| w.id != source)
        .unwrap()
        .id;
    let mut worlds = state.worlds.as_slice().to_vec();
    for w in &mut worlds {
        w.owner = Some(actor);
        w.inventory = LivingInventoryV2 {
            energy: if w.id == source {
                100
            } else {
                destination_energy
            },
            ore: if w.id == source { 100 } else { 0 },
            alloy: 0,
        };
    }
    state.worlds = LivingSortedVecV2::new(worlds).unwrap();
    let mut colony = state.colonies.as_slice()[0].clone();
    colony.world = destination;
    state.colonies =
        LivingSortedVecV2::new(vec![state.colonies.as_slice()[0].clone(), colony]).unwrap();
    let other = builtin_living_sandbox_v2().unwrap();
    let mut foundry = other
        .state()
        .facilities
        .as_slice()
        .iter()
        .find(|f| f.definition.as_str() == "foundry")
        .unwrap()
        .clone();
    foundry.id = LivingEntityIdV2([0xa1; 16]);
    foundry.world = destination;
    foundry.next_due = Some(LivingTickV2(20));
    state.facilities = LivingSortedVecV2::new(vec![foundry]).unwrap();
    state.fleets = Default::default();
    state.routes = LivingSortedVecV2::new(vec![LivingFreightRouteV2 {
        id: LivingEntityIdV2([0xa2; 16]),
        kind: LivingRouteKindV2::Internal,
        source_world: source,
        destination_world: destination,
        resource: LivingResourceV2::Ore,
        batch_size: 1,
        cadence_ticks: 10,
        source_reserve: 20,
        source_owner: actor,
        receiver_owner: actor,
        next_due: LivingTickV2(11),
        suspended: false,
    }])
    .unwrap();
    state.hazards = LivingSortedVecV2::new(vec![LivingHazardV2 {
        id: LivingEntityIdV2([0xa3; 16]),
        definition: LivingSlugV2::new("ion_storm").unwrap(),
        lane: state.lanes.as_slice()[0].id,
        start_tick: LivingTickV2(10),
        end_tick: LivingTickV2(150),
    }])
    .unwrap();
    let catalog = living_core_pack_v2().unwrap();
    let genesis = validate_living_genesis_manifest_v2(
        LivingGenesisManifestV2 {
            kind: LIVING_GENESIS_KIND_V2.into(),
            format_version: 2,
            rules_version: 2,
            state,
        },
        &catalog,
    )
    .unwrap();
    LivingGalaxyAuthorityV2::from_genesis(
        catalog,
        [0xa8; 32],
        LivingGenesisGeneratorV2 {
            generator_id: LivingSlugV2::new("corridor_notebook_fixture").unwrap(),
            generator_version: 1,
        },
        genesis,
    )
    .unwrap()
}
fn terminal(client: &mut LivingClient) -> LivingClientEvent {
    let deadline = Instant::now() + Duration::from_secs(30);
    loop {
        let event = client.poll();
        if !matches!(event, LivingClientEvent::Pending) {
            return event;
        }
        assert!(Instant::now() < deadline, "native corridor save timed out");
        std::thread::sleep(Duration::from_millis(1));
    }
}
fn runtime(root: &std::path::Path) -> Runtime {
    let classic = AppCore::new_with_preferences(
        ScenarioDraft::factory_default().validated().unwrap(),
        MemoryScenarioStore::default(),
        MemoryPreferencesStore::default(),
    );
    let mut runtime = ClientRuntime::new(classic, MemoryWorkshopStore::default());
    runtime.install_living_storage(
        Box::new(NativeLivingStoreV2::new(root)),
        Box::new(NativeLibraryCoordinator::new(root)),
    );
    runtime.select_menu_route(MainMenuRoute::Continue).unwrap();
    settle(&mut runtime, |runtime| runtime.sandbox_tick().is_some());
    runtime
}
fn settle(runtime: &mut Runtime, done: impl Fn(&Runtime) -> bool) {
    let deadline = Instant::now() + Duration::from_secs(30);
    loop {
        runtime.update(Duration::ZERO);
        if done(runtime) {
            return;
        }
        assert!(
            Instant::now() < deadline,
            "corridor runtime operation timed out: {}",
            runtime.living_storage_status()
        );
        std::thread::sleep(Duration::from_millis(1));
    }
}

#[derive(Default)]
struct CaptureNotebook {
    captured: Rc<RefCell<Vec<Vec<u8>>>>,
    pending: bool,
}
impl LivingTransfer for CaptureNotebook {
    fn begin(&mut self, request: LivingTransferRequest) -> Result<(), LivingTransferError> {
        request.validate()?;
        let LivingTransferRequest::Export {
            kind: LivingTransferKind::Notebook,
            bytes,
            ..
        } = request
        else {
            panic!("unexpected transfer request");
        };
        self.captured.borrow_mut().push(bytes);
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
fn visible(runtime: &mut Runtime) -> String {
    runtime.apply_sandbox_action(A::NotebookPicker);
    runtime.apply_sandbox_action(A::ExperimentDetails(Some(3)));
    let mut text = String::new();
    for _ in 0..100 {
        let frame = runtime.sandbox_frame(Vec2::new(800., 480.), false).unwrap();
        frame.semantics.validate().unwrap();
        for control in &frame.controls {
            if control
                .action_id
                .as_str()
                .starts_with("sandbox.notebook.fact.corridor")
                || control
                    .action_id
                    .as_str()
                    .starts_with("sandbox.notebook.corridor.open")
            {
                text.push_str(&control.label);
            }
        }
        if !frame
            .controls
            .iter()
            .any(|c| c.action_id.as_str() == "sandbox.notebook.next" && c.enabled)
        {
            break;
        }
        runtime.apply_sandbox_action(A::InspectorPage(false));
    }
    text
}
fn seeded_runtime(root: &std::path::Path) -> Runtime {
    let mut writer = LivingClient::new(
        Box::new(NativeLivingStoreV2::new(root)),
        Box::new(NativeLibraryCoordinator::new(root)),
    );
    writer.read_selection().unwrap();
    assert!(matches!(
        terminal(&mut writer),
        LivingClientEvent::AbsentSelection
    ));
    writer.save(&fixture(), false).unwrap();
    assert!(matches!(
        terminal(&mut writer),
        LivingClientEvent::Saved { .. }
    ));
    drop(writer);
    runtime(root)
}
fn verify(runtime: &mut Runtime, expected_pairs: usize) {
    runtime.apply_sandbox_action(A::VerifyNotebookEvidence);
    assert_eq!(
        runtime.living_storage_status(),
        "Replaying exact notebook observations"
    );
    settle(runtime, |r| {
        r.living_storage_status()
            .starts_with("Notebook observations replay-verified")
    });
    assert!(
        runtime
            .living_storage_status()
            .ends_with(&format!("{expected_pairs} matching corridor observations")),
        "{}",
        runtime.living_storage_status()
    );
}
#[test]
fn asynchronous_verification_and_native_continue_preserve_exact_corridor_observations() {
    let root = tempfile::tempdir().unwrap();
    let mut running = seeded_runtime(root.path());
    for _ in 0..11 {
        running.step_sandbox(1).unwrap();
    }
    running.apply_sandbox_action(A::RecordExperiment(3));
    for _ in 11..20 {
        running.step_sandbox(1).unwrap();
    }
    running.apply_sandbox_action(A::RecordExperiment(3));
    assert!(running.living_dirty());
    assert!(
        visible(&mut running).contains("Matching corridor observations wait for replay-verified")
    );
    let captured = Rc::new(RefCell::new(Vec::new()));
    running.install_living_transfer(Box::new(CaptureNotebook {
        captured: captured.clone(),
        pending: false,
    }));
    running.apply_sandbox_action(A::PrepareExport);
    settle(&mut running, |r| {
        r.living_storage_status()
            == "File set prepared; export each file to hand it to your platform"
    });
    verify(&mut running, 1);
    assert_eq!(running.sandbox_tick(), Some(20));
    assert!(running.living_dirty());
    let verified = visible(&mut running);
    assert!(
        verified.contains("Restricted dispatch and matching later input wait observed"),
        "{verified}"
    );
    assert!(
        verified.contains("dispatch tick 11; wait tick 20"),
        "{verified}"
    );
    assert!(
        verified.contains("actually dispatched 0 units"),
        "{verified}"
    );
    assert!(
        verified.contains("does not establish sole causation"),
        "{verified}"
    );
    assert!(
        verified.contains("Open verified corridor dispatch bookmark"),
        "{verified}"
    );
    assert!(
        verified.contains("Open verified corridor input wait bookmark"),
        "{verified}"
    );
    // Reusing the earlier prepared bundle invokes the production fence on exact
    // resident bookmark AND notebook generation. The mock only captures bytes;
    // it provides no durable file-transfer evidence.
    running.apply_sandbox_action(A::ExportNotebook);
    assert_eq!(
        captured.borrow().len(),
        1,
        "{}",
        running.living_storage_status()
    );
    running.update(Duration::ZERO);
    running.save_living(true).unwrap();
    settle(&mut running, |r| {
        r.living_storage_status() == "Galaxy and Continue selection saved"
    });
    assert!(!running.living_dirty());
    drop(running);
    let mut reader = LivingClient::new(
        Box::new(NativeLivingStoreV2::new(root.path())),
        Box::new(NativeLibraryCoordinator::new(root.path())),
    );
    reader.read_selection().unwrap();
    let LivingClientEvent::ReadyWithNotebook {
        authority,
        notebook,
    } = terminal(&mut reader)
    else {
        panic!("fresh Continue did not return validated Notebook");
    };
    let mut expected = fixture();
    for _ in 0..20 {
        expected.step().unwrap();
    }
    assert_eq!(authority.bookmark().unwrap(), expected.bookmark().unwrap());
    assert_eq!(notebook.corridor_observations().len(), 1);
    assert_eq!(
        notebook.notebook().canonical_bytes().unwrap(),
        captured.borrow()[0]
    );
    drop(reader);
    let mut reopened = runtime(root.path());
    assert_eq!(reopened.sandbox_tick(), Some(20));
    assert!(!reopened.living_dirty());
    assert_eq!(visible(&mut reopened), verified);
}
#[test]
fn lone_restricted_attempt_stays_unpaired_after_async_verify_save_and_fresh_continue() {
    let root = tempfile::tempdir().unwrap();
    let mut running = seeded_runtime(root.path());
    for _ in 0..11 {
        running.step_sandbox(1).unwrap();
    }
    running.apply_sandbox_action(A::RecordExperiment(3));
    verify(&mut running, 0);
    let text = visible(&mut running);
    assert!(
        text.contains("Matching corridor observations wait for replay-verified"),
        "{text}"
    );
    assert!(
        !text.contains("Restricted dispatch and matching later input wait observed"),
        "{text}"
    );
    assert!(
        !text.contains("Open verified corridor dispatch bookmark"),
        "{text}"
    );
    assert_eq!(running.sandbox_tick(), Some(11));
    running.save_living(true).unwrap();
    settle(&mut running, |r| {
        r.living_storage_status() == "Galaxy and Continue selection saved"
    });
    drop(running);
    let mut reader = LivingClient::new(
        Box::new(NativeLivingStoreV2::new(root.path())),
        Box::new(NativeLibraryCoordinator::new(root.path())),
    );
    reader.read_selection().unwrap();
    let LivingClientEvent::ReadyWithNotebook {
        authority,
        notebook,
    } = terminal(&mut reader)
    else {
        panic!("fresh Continue did not validate the lone observation");
    };
    assert!(notebook.corridor_observations().is_empty());
    let mut expected = fixture();
    for _ in 0..11 {
        expected.step().unwrap();
    }
    assert_eq!(authority.bookmark().unwrap(), expected.bookmark().unwrap());
    drop(reader);

    let mut reopened = runtime(root.path());
    assert_eq!(reopened.sandbox_tick(), Some(11));
    assert_eq!(visible(&mut reopened), text);
}
