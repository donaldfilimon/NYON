//! Host runtime/local-replay/native-store journey, separate from live browser acceptance.
#![cfg(not(target_arch = "wasm32"))]
use glam::Vec2;
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
use std::time::{Duration, Instant};
type Runtime = ClientRuntime<MemoryScenarioStore, MemoryPreferencesStore, MemoryWorkshopStore>;
fn fixture(
    resource: LivingResourceV2,
    arrival: u64,
    due: u64,
    wrong_endpoint: bool,
    same_system: bool,
) -> LivingGalaxyAuthorityV2 {
    let baseline = builtin_first_expansion_v2().unwrap();
    let mut state = baseline.state().clone();
    let actor = state.civilizations.as_slice()[0].id;
    let source = state
        .worlds
        .as_slice()
        .iter()
        .find(|world| world.owner == Some(actor))
        .unwrap()
        .id;
    let destination = state
        .worlds
        .as_slice()
        .iter()
        .find(|world| world.owner.is_none())
        .unwrap()
        .id;
    let source_system = state
        .worlds
        .as_slice()
        .iter()
        .find(|world| world.id == source)
        .unwrap()
        .system;
    let mut worlds = state.worlds.as_slice().to_vec();
    for world in &mut worlds {
        world.owner = Some(actor);
        world.inventory = LivingInventoryV2 {
            energy: 100,
            ore: 100,
            alloy: 0,
        };
        if same_system {
            world.system = source_system;
        }
    }
    state.worlds = LivingSortedVecV2::new(worlds).unwrap();
    let mut frontier_colony = state.colonies.as_slice()[0].clone();
    frontier_colony.world = destination;
    state.colonies =
        LivingSortedVecV2::new(vec![state.colonies.as_slice()[0].clone(), frontier_colony])
            .unwrap();
    let other = builtin_living_sandbox_v2().unwrap();
    let mut foundry = other
        .state()
        .facilities
        .as_slice()
        .iter()
        .find(|facility| facility.definition.as_str() == "foundry")
        .unwrap()
        .clone();
    foundry.id = LivingEntityIdV2([0xa3; 16]);
    foundry.world = if (resource == LivingResourceV2::Ore) != wrong_endpoint {
        destination
    } else {
        source
    };
    foundry.next_due = Some(LivingTickV2(due));
    state.facilities = LivingSortedVecV2::new(vec![foundry]).unwrap();
    state.fleets = LivingSortedVecV2::default();
    state.routes = LivingSortedVecV2::default();
    state.shipments = LivingSortedVecV2::new(vec![LivingShipmentV2 {
        id: LivingEntityIdV2([0xa4; 16]),
        dispatch_owner: actor,
        intended_receiver: actor,
        source_world: source,
        destination_world: destination,
        resource,
        units: 1,
        departure_tick: LivingTickV2(0),
        arrival_tick: LivingTickV2(arrival),
        disposition: LivingShipmentDispositionV2::Outbound,
        dispatch_kind: Some(LivingRouteKindV2::Internal),
        returning_to_source: false,
    }])
    .unwrap();
    let catalog = living_core_pack_v2().unwrap();
    let genesis = validate_living_genesis_manifest_v2(
        LivingGenesisManifestV2 {
            kind: LIVING_GENESIS_KIND_V2.into(),
            format_version: LIVING_GENESIS_FORMAT_VERSION_V2,
            rules_version: 2,
            state,
        },
        &catalog,
    )
    .unwrap();
    LivingGalaxyAuthorityV2::from_genesis(
        catalog,
        [0xa7; 32],
        LivingGenesisGeneratorV2 {
            generator_id: LivingSlugV2::new("supply_completion_fixture").unwrap(),
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
        assert!(Instant::now() < deadline, "native supply save timed out");
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
            "supply runtime operation timed out: {}",
            runtime.living_storage_status()
        );
        std::thread::sleep(Duration::from_millis(1));
    }
}
fn visible(runtime: &mut Runtime) -> String {
    runtime.apply_sandbox_action(A::NotebookPicker);
    runtime.apply_sandbox_action(A::ExperimentDetails(Some(0)));
    let mut text = String::new();
    for _ in 0..100 {
        let frame = runtime.sandbox_frame(Vec2::new(800., 480.), false).unwrap();
        frame.semantics.validate().unwrap();
        for control in &frame.controls {
            if control
                .action_id
                .as_str()
                .starts_with("sandbox.notebook.fact.supply")
                || control
                    .action_id
                    .as_str()
                    .starts_with("sandbox.notebook.supply.open")
            {
                text.push_str(&control.label);
            }
        }
        if !frame
            .controls
            .iter()
            .any(|control| control.action_id.as_str() == "sandbox.notebook.next" && control.enabled)
        {
            break;
        }
        runtime.apply_sandbox_action(A::InspectorPage(false));
    }
    text
}
#[test]
fn local_verify_then_save_destroy_continue_retains_visible_supply_proof() {
    let root = tempfile::tempdir().unwrap();
    let authority = fixture(LivingResourceV2::Alloy, 21, 20, false, false);
    let mut writer = LivingClient::new(
        Box::new(NativeLivingStoreV2::new(root.path())),
        Box::new(NativeLibraryCoordinator::new(root.path())),
    );
    writer.read_selection().unwrap();
    assert!(matches!(
        terminal(&mut writer),
        LivingClientEvent::AbsentSelection
    ));
    writer.save(&authority, false).unwrap();
    assert!(matches!(
        terminal(&mut writer),
        LivingClientEvent::Saved { .. }
    ));
    drop(writer);
    let mut running = runtime(root.path());
    assert_eq!(running.sandbox_tick(), Some(0));
    for _ in 0..20 {
        running.step_sandbox(1).unwrap();
    }
    running.apply_sandbox_action(A::RecordExperiment(0));
    running.step_sandbox(1).unwrap();
    running.apply_sandbox_action(A::RecordExperiment(0));
    assert!(visible(&mut running).contains("Supply completion waits for replay-verified"));
    running.apply_sandbox_action(A::VerifyNotebookEvidence);
    settle(&mut running, |runtime| {
        let frame = runtime.sandbox_frame(Vec2::new(800., 480.), false).unwrap();
        frame.controls.iter().any(|control| {
            control
                .action_id
                .as_str()
                .starts_with("sandbox.notebook.fact.supply.complete")
        })
    });
    let verified = visible(&mut running);
    assert!(
        verified.contains("Alloy production and matching inter-system delivery observed"),
        "{verified}"
    );
    assert!(
        verified.contains("Open verified supply production bookmark"),
        "{verified}"
    );
    assert!(
        verified.contains("Open verified supply delivery bookmark"),
        "{verified}"
    );
    assert!(
        verified.contains("does not identify the same fungible units"),
        "{verified}"
    );
    assert_eq!(running.sandbox_tick(), Some(21));
    running.save_living(true).unwrap();
    settle(&mut running, |runtime| {
        runtime.living_storage_status() == "Galaxy and Continue selection saved"
    });
    assert!(!running.living_dirty());
    drop(running);
    let mut reopened = runtime(root.path());
    assert_eq!(reopened.sandbox_tick(), Some(21));
    assert!(!reopened.living_dirty());
    let restored = visible(&mut reopened);
    assert!(
        restored.contains("Alloy production and matching inter-system delivery observed"),
        "{restored}"
    );
    assert!(
        restored.contains("Open verified supply production bookmark"),
        "{restored}"
    );
    assert!(
        restored.contains("Open verified supply delivery bookmark"),
        "{restored}"
    );
}
