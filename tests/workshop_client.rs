use std::time::Duration;

use nyon::{
    app::{
        AppCore, AppMode,
        client_runtime::{
            ActiveSession, ClientDiagnosticCode, ClientRuntime, ClientRuntimeEffect,
            ClientRuntimeError, ClientScreen, MainMenuRoute,
        },
    },
    game::model::DEFAULT_SEED,
    preferences::store::MemoryPreferencesStore,
    scenario::{ScenarioDraft, store::MemoryScenarioStore},
    workshop::{
        BatchLocalId, CatalogHash, CreatorBatchV1, CreatorOpV1, GalaxyPointV1, ObjectName,
        WorkshopHistory, decode_catalog_pack, encode_archive, encode_catalog_pack,
        session::WorkshopAction,
        store::{
            MemoryWorkshopStore, SlotName, StoreJobId, StoreJobState, WorkshopStore,
            WorkshopStoreError, WorkshopStoreRequest, WorkshopStoreResult,
        },
    },
};

type Runtime = ClientRuntime<MemoryScenarioStore, MemoryPreferencesStore, MemoryWorkshopStore>;

fn classic() -> AppCore<MemoryScenarioStore, MemoryPreferencesStore> {
    AppCore::new_with_preferences(
        ScenarioDraft::factory_default().validated().unwrap(),
        MemoryScenarioStore::default(),
        MemoryPreferencesStore::default(),
    )
}

fn runtime(store: MemoryWorkshopStore) -> Runtime {
    ClientRuntime::new(classic(), store)
}

fn complete(store: &mut dyn WorkshopStore, request: WorkshopStoreRequest) -> WorkshopStoreResult {
    let job = store.start(request).unwrap();
    match store.poll(job) {
        StoreJobState::Complete(Ok(result)) => result,
        result => panic!("unexpected Workshop store result: {result:?}"),
    }
}

fn valid_archive(seed: u64) -> Box<[u8]> {
    let catalog =
        decode_catalog_pack(include_bytes!("../assets/workshop/core-pack-v1.json")).unwrap();
    encode_archive(&WorkshopHistory::from_seed_u64(catalog, seed))
        .unwrap()
        .bytes
        .into_boxed_slice()
}

fn selected_store(archive: Box<[u8]>) -> MemoryWorkshopStore {
    let mut store = MemoryWorkshopStore::default();
    let created = complete(
        &mut store,
        WorkshopStoreRequest::CreateSlot {
            name: SlotName::new("Two-System Forge").unwrap(),
            archive,
        },
    );
    let WorkshopStoreResult::SlotCreated { slot, .. } = created else {
        panic!("unexpected create result: {created:?}");
    };
    complete(&mut store, WorkshopStoreRequest::SelectContinue { slot });
    store
}

fn custom_pack() -> Vec<u8> {
    let mut value: serde_json::Value =
        serde_json::from_slice(include_bytes!("../assets/workshop/core-pack-v1.json")).unwrap();
    value["title"] = serde_json::json!("Custom Forge Catalog");
    let validated = decode_catalog_pack(&serde_json::to_vec(&value).unwrap()).unwrap();
    encode_catalog_pack(&validated).unwrap()
}

fn custom_archive(seed: u64) -> (Vec<u8>, Box<[u8]>) {
    let pack_bytes = custom_pack();
    let catalog = decode_catalog_pack(&pack_bytes).unwrap();
    let archive = encode_archive(&WorkshopHistory::from_seed_u64(catalog, seed))
        .unwrap()
        .bytes
        .into_boxed_slice();
    (pack_bytes, archive)
}

fn selected_custom_store(canonical_pack: Option<&[u8]>, archive: Box<[u8]>) -> MemoryWorkshopStore {
    let mut store = selected_store(archive);
    if let Some(pack) = canonical_pack {
        complete(
            &mut store,
            WorkshopStoreRequest::PutPack {
                canonical_pack: Box::from(pack),
            },
        );
    }
    store
}

fn finish_continue<W: WorkshopStore>(
    runtime: &mut ClientRuntime<MemoryScenarioStore, MemoryPreferencesStore, W>,
) {
    for _ in 0..16 {
        if !runtime.continue_bootstrap_active() {
            return;
        }
        runtime.update(Duration::ZERO);
    }
    panic!("Continue bootstrap exceeded its bounded test polls");
}

#[derive(Clone, Copy)]
enum PackFault {
    WrongHash,
    CorruptBytes,
}

struct FaultyPackStore {
    inner: MemoryWorkshopStore,
    fault: PackFault,
    pending: Option<(StoreJobId, StoreJobState)>,
}

impl FaultyPackStore {
    fn new(inner: MemoryWorkshopStore, fault: PackFault) -> Self {
        Self {
            inner,
            fault,
            pending: None,
        }
    }
}

impl WorkshopStore for FaultyPackStore {
    fn start(&mut self, request: WorkshopStoreRequest) -> Result<StoreJobId, WorkshopStoreError> {
        if let WorkshopStoreRequest::GetPack { hash } = request {
            let job = StoreJobId(u64::MAX);
            let result = match self.fault {
                PackFault::WrongHash => WorkshopStoreResult::PackLoaded {
                    hash: CatalogHash([0xa5; 32]),
                    canonical_pack: custom_pack().into_boxed_slice(),
                },
                PackFault::CorruptBytes => WorkshopStoreResult::PackLoaded {
                    hash,
                    canonical_pack: Box::from(&b"{}"[..]),
                },
            };
            self.pending = Some((job, StoreJobState::Complete(Ok(result))));
            Ok(job)
        } else {
            self.inner.start(request)
        }
    }

    fn poll(&mut self, job: StoreJobId) -> StoreJobState {
        if self
            .pending
            .as_ref()
            .is_some_and(|pending| pending.0 == job)
        {
            return self.pending.take().unwrap().1;
        }
        self.inner.poll(job)
    }
}

#[test]
fn startup_is_main_menu_without_constructing_a_workshop_authority() {
    let runtime = runtime(MemoryWorkshopStore::default());

    assert_eq!(runtime.screen(), ClientScreen::MainMenu);
    assert!(matches!(runtime.active_session(), ActiveSession::None));
    assert!(runtime.workshop_snapshot().is_none());
    assert!(!runtime.continue_bootstrap_active());
    assert!(!runtime.continue_available());
}

#[test]
fn continue_remains_disabled_when_storage_has_no_explicit_selection() {
    let mut runtime = runtime(MemoryWorkshopStore::default());
    assert_eq!(
        runtime.select_menu_route(MainMenuRoute::Continue),
        Err(ClientRuntimeError::ContinueUnavailable)
    );
    assert!(matches!(runtime.active_session(), ActiveSession::None));

    runtime.begin_continue_bootstrap().unwrap();
    runtime.update(Duration::ZERO);

    assert_eq!(runtime.screen(), ClientScreen::MainMenu);
    assert!(!runtime.continue_available());
    assert!(runtime.recovery_diagnostic().is_none());
}

#[test]
fn classic_route_preserves_the_frozen_rules_v1_behavior() {
    let mut runtime = runtime(MemoryWorkshopStore::default());
    assert_eq!(runtime.classic().active_scenario().seed(), DEFAULT_SEED);
    assert_eq!(runtime.classic().simulation().state().worlds.len(), 7);
    assert_eq!(
        runtime.classic().simulation().canonical_fingerprint(),
        0x67D9_6E98_3D6C_9330
    );

    runtime
        .select_menu_route(MainMenuRoute::ClassicSector)
        .unwrap();
    assert_eq!(runtime.screen(), ClientScreen::ClassicSector);
    assert!(matches!(runtime.active_session(), ActiveSession::Classic));
    assert_eq!(runtime.classic().mode(), AppMode::Playing);
}

#[test]
fn new_workshop_is_created_only_after_the_lazy_menu_route() {
    let mut runtime = runtime(MemoryWorkshopStore::default());
    assert!(runtime.workshop_snapshot().is_none());

    runtime.start_new_workshop(0xC0FFEE).unwrap();

    assert_eq!(runtime.screen(), ClientScreen::GalaxyWorkshop);
    let session = runtime.workshop_snapshot().unwrap();
    assert_eq!(session.state.tick.0, 0);
    assert!(session.state.systems.is_empty());
    let ActiveSession::Workshop(workshop) = runtime.active_session() else {
        panic!("Workshop route did not install a Workshop session");
    };
    assert_eq!(
        workshop.history().genesis_seed()[..8],
        0xC0FFEE_u64.to_le_bytes()
    );
}

#[test]
fn continue_enables_only_after_the_explicit_selected_slot_fully_validates() {
    let mut runtime = runtime(selected_store(valid_archive(91)));
    assert!(!runtime.continue_available());

    runtime.begin_continue_bootstrap().unwrap();
    assert_eq!(runtime.screen(), ClientScreen::Loading);
    runtime.update(Duration::ZERO);
    assert!(!runtime.continue_available());
    finish_continue(&mut runtime);

    assert_eq!(runtime.screen(), ClientScreen::MainMenu);
    assert!(runtime.continue_available());
    assert!(
        runtime
            .menu_capabilities()
            .iter()
            .find(|item| item.route == MainMenuRoute::Continue)
            .unwrap()
            .enabled
    );
    runtime.select_menu_route(MainMenuRoute::Continue).unwrap();
    assert_eq!(runtime.screen(), ClientScreen::GalaxyWorkshop);
    let session = runtime.workshop_snapshot().unwrap();
    assert_eq!(session.store.slot.map(|slot| slot.0), Some(0));
    assert_eq!(
        session.store.generation.map(|generation| generation.0),
        Some(1)
    );
    let ActiveSession::Workshop(workshop) = runtime.active_session() else {
        panic!("Continue did not install its validated Workshop");
    };
    assert_eq!(workshop.history().genesis_seed()[..8], 91_u64.to_le_bytes());
    assert!(runtime.continue_available());
}

#[test]
fn custom_catalog_is_persisted_started_exported_and_continued_by_exact_hash() {
    let pack_bytes = custom_pack();
    let expected = decode_catalog_pack(&pack_bytes).unwrap().catalog_hash();
    let mut imported = runtime(MemoryWorkshopStore::default());
    imported.start_new_workshop(7).unwrap();
    let prior = imported.workshop_snapshot().unwrap().state_digest;

    assert_eq!(
        imported
            .begin_new_workshop_from_catalog(&pack_bytes, 0xCA7A10)
            .unwrap(),
        expected
    );
    assert_eq!(imported.workshop_snapshot().unwrap().state_digest, prior);
    assert!(imported.catalog_import_active());
    imported.update(Duration::ZERO);
    assert!(!imported.catalog_import_active());
    assert_eq!(imported.imported_catalog_hash(), Some(expected));
    assert_eq!(
        imported.export_active_catalog().unwrap().as_ref(),
        pack_bytes
    );
    let ActiveSession::Workshop(workshop) = imported.active_session() else {
        panic!("custom catalog creation did not install a Workshop");
    };
    assert_eq!(
        workshop.history().genesis_seed()[..8],
        0xCA7A10_u64.to_le_bytes()
    );

    let (_, archive) = custom_archive(404);
    let store = selected_custom_store(Some(&pack_bytes), archive);
    let mut continued = runtime(store);
    continued.begin_continue_bootstrap().unwrap();
    finish_continue(&mut continued);
    assert!(continued.continue_available());
    continued
        .select_menu_route(MainMenuRoute::Continue)
        .unwrap();
    assert_eq!(
        continued.export_active_catalog().unwrap().as_ref(),
        pack_bytes
    );
    let ActiveSession::Workshop(workshop) = continued.active_session() else {
        panic!("custom Continue did not install a Workshop");
    };
    assert_eq!(
        workshop.history().genesis_seed()[..8],
        404_u64.to_le_bytes()
    );
}

#[test]
fn missing_custom_catalog_recovery_preserves_the_active_session() {
    let (_, archive) = custom_archive(405);
    let mut runtime = runtime(selected_custom_store(None, archive));
    runtime.start_new_workshop(99).unwrap();
    let before = runtime.workshop_snapshot().unwrap().state_digest;

    runtime.begin_continue_bootstrap().unwrap();
    finish_continue(&mut runtime);

    assert_eq!(runtime.screen(), ClientScreen::RecoverableError);
    assert_eq!(
        runtime.recovery_diagnostic().unwrap().code,
        ClientDiagnosticCode::Store
    );
    assert_eq!(runtime.workshop_snapshot().unwrap().state_digest, before);
    assert!(!runtime.continue_available());
}

#[test]
fn wrong_or_corrupt_custom_catalog_recovery_preserves_the_active_session() {
    for (fault, expected_code) in [
        (PackFault::WrongHash, ClientDiagnosticCode::StoreProtocol),
        (PackFault::CorruptBytes, ClientDiagnosticCode::Catalog),
    ] {
        let (_, archive) = custom_archive(406);
        let inner = selected_custom_store(None, archive);
        let mut runtime = ClientRuntime::new(classic(), FaultyPackStore::new(inner, fault));
        runtime.start_new_workshop(100).unwrap();
        let before = runtime.workshop_snapshot().unwrap().state_digest;

        runtime.begin_continue_bootstrap().unwrap();
        finish_continue(&mut runtime);

        assert_eq!(runtime.screen(), ClientScreen::RecoverableError);
        assert_eq!(runtime.recovery_diagnostic().unwrap().code, expected_code);
        assert_eq!(runtime.workshop_snapshot().unwrap().state_digest, before);
        assert!(!runtime.continue_available());
    }
}

#[test]
fn invalid_catalog_import_preserves_a_valid_active_session() {
    let mut runtime = runtime(MemoryWorkshopStore::default());
    runtime.start_new_workshop(101).unwrap();
    let before = runtime.workshop_snapshot().unwrap().state_digest;

    assert_eq!(
        runtime.begin_catalog_import(b"{}"),
        Err(ClientRuntimeError::InvalidCatalog)
    );
    assert_eq!(runtime.workshop_snapshot().unwrap().state_digest, before);
    assert!(!runtime.catalog_import_active());
}

#[test]
fn corrupt_continue_enters_recovery_without_replacing_a_valid_live_workshop() {
    let mut runtime = runtime(selected_store(Box::from(&b"{}"[..])));
    runtime.start_new_workshop(73).unwrap();
    let before = runtime.workshop_snapshot().unwrap().state_digest;

    runtime.begin_continue_bootstrap().unwrap();
    finish_continue(&mut runtime);

    assert_eq!(runtime.screen(), ClientScreen::RecoverableError);
    assert_eq!(
        runtime.recovery_diagnostic().unwrap().code,
        ClientDiagnosticCode::Archive
    );
    assert_eq!(runtime.workshop_snapshot().unwrap().state_digest, before);
    assert!(!runtime.continue_available());
}

#[test]
fn workshop_mutations_cross_only_the_typed_mailbox() {
    let mut runtime = runtime(MemoryWorkshopStore::default());
    runtime.start_new_workshop(17).unwrap();
    let snapshot = runtime.workshop_snapshot().unwrap();
    let batch = CreatorBatchV1 {
        expected_cursor: snapshot.active_view.view_cursor,
        expected_tick: snapshot.active_view.tick,
        operations: vec![CreatorOpV1::CreateSystem {
            local: BatchLocalId(1),
            name: ObjectName::new("Mailbox Forge").unwrap(),
            position: GalaxyPointV1::new(1_024, 0).unwrap(),
        }],
    };

    runtime
        .enqueue_workshop_action(WorkshopAction::Submit(batch))
        .unwrap();
    assert!(
        runtime
            .workshop_snapshot()
            .unwrap()
            .state
            .systems
            .is_empty()
    );
    let update = runtime.update(Duration::ZERO);
    assert_eq!(update.accepted_batches, 1);
    assert_eq!(runtime.workshop_snapshot().unwrap().state.systems.len(), 1);

    runtime.return_to_main_menu().unwrap();
    assert_eq!(
        runtime.select_menu_route(MainMenuRoute::ClassicSector),
        Err(ClientRuntimeError::RouteUnavailable)
    );
    assert!(matches!(
        runtime.active_session(),
        ActiveSession::Workshop(_)
    ));
    runtime.return_to_active_session();
    assert_eq!(runtime.screen(), ClientScreen::GalaxyWorkshop);
    assert!(
        runtime
            .enqueue_workshop_action(WorkshopAction::Pause)
            .is_ok()
    );
}

#[test]
fn saved_resident_workshop_continues_with_the_exact_digest_and_tick() {
    let mut runtime = runtime(MemoryWorkshopStore::default());
    runtime.start_new_workshop(0xF0A6E).unwrap();
    let initial = runtime.workshop_snapshot().unwrap();
    let batch = CreatorBatchV1 {
        expected_cursor: initial.active_view.view_cursor,
        expected_tick: initial.active_view.tick,
        operations: vec![CreatorOpV1::CreateSystem {
            local: BatchLocalId(1),
            name: ObjectName::new("Resident Forge").unwrap(),
            position: GalaxyPointV1::new(2_048, 1_024).unwrap(),
        }],
    };
    runtime
        .enqueue_workshop_action(WorkshopAction::Submit(batch))
        .unwrap();
    runtime
        .enqueue_workshop_action(WorkshopAction::StepOnce)
        .unwrap();
    let update = runtime.update(Duration::ZERO);
    assert_eq!(update.accepted_batches, 1);
    assert_eq!(update.authority_steps, 1);
    let before = runtime.workshop_snapshot().unwrap();
    let digest = before.state_digest;
    let tick = before.state.tick;

    runtime.return_to_main_menu().unwrap();
    assert_eq!(runtime.screen(), ClientScreen::MainMenu);
    assert!(!runtime.continue_available());
    for route in [MainMenuRoute::NewWorkshop, MainMenuRoute::ClassicSector] {
        assert!(
            !runtime
                .menu_capabilities()
                .iter()
                .find(|capability| capability.route == route)
                .unwrap()
                .enabled
        );
        assert_eq!(
            runtime.select_menu_route(route),
            Err(ClientRuntimeError::RouteUnavailable)
        );
    }

    let transition = runtime.update(Duration::ZERO);
    assert_eq!(transition.drained_actions, 2);
    assert!(runtime.workshop_snapshot().unwrap().store.commit_pending);
    assert_eq!(
        runtime.start_new_workshop(0xDEAD_BEEF),
        Err(ClientRuntimeError::RouteUnavailable)
    );
    assert_eq!(runtime.workshop_snapshot().unwrap().state_digest, digest);

    for _ in 0..8 {
        runtime.update(Duration::ZERO);
        if runtime.continue_available() {
            break;
        }
    }
    assert!(runtime.continue_available());
    let persisted = runtime.workshop_snapshot().unwrap();
    assert!(!persisted.store.dirty);
    assert!(!persisted.store.commit_pending);
    assert!(persisted.store.slot.is_some());
    assert!(persisted.store.generation.is_some());
    assert_eq!(persisted.state_digest, digest);
    assert_eq!(persisted.state.tick, tick);

    runtime.select_menu_route(MainMenuRoute::Continue).unwrap();
    assert_eq!(runtime.screen(), ClientScreen::GalaxyWorkshop);
    let resumed = runtime.workshop_snapshot().unwrap();
    assert_eq!(resumed.state_digest, digest);
    assert_eq!(resumed.state.tick, tick);
    assert_eq!(
        resumed.state.systems.values().next().unwrap().name.as_str(),
        "Resident Forge"
    );
}

#[test]
fn menu_exit_does_not_commit_a_redundant_generation_for_a_current_save() {
    let mut runtime = runtime(MemoryWorkshopStore::default());
    runtime.start_new_workshop(0x5A7E).unwrap();
    let initial = runtime.workshop_snapshot().unwrap();
    runtime
        .enqueue_workshop_action(WorkshopAction::Submit(CreatorBatchV1 {
            expected_cursor: initial.active_view.view_cursor,
            expected_tick: initial.active_view.tick,
            operations: vec![CreatorOpV1::CreateSystem {
                local: BatchLocalId(1),
                name: ObjectName::new("Current Save").unwrap(),
                position: GalaxyPointV1::new(4_096, 0).unwrap(),
            }],
        }))
        .unwrap();
    runtime
        .enqueue_workshop_action(WorkshopAction::RequestSave)
        .unwrap();
    runtime.update(Duration::ZERO);
    assert!(runtime.workshop_snapshot().unwrap().store.commit_pending);
    assert_eq!(runtime.workshop_snapshot().unwrap().store.generation, None);

    runtime.return_to_main_menu().unwrap();
    let transition = runtime.update(Duration::ZERO);
    assert_eq!(transition.drained_actions, 2);
    for _ in 0..4 {
        runtime.update(Duration::ZERO);
    }

    let persisted = runtime.workshop_snapshot().unwrap();
    assert_eq!(
        persisted.store.generation,
        Some(nyon::workshop::store::SaveGeneration(1))
    );
    assert!(!persisted.store.commit_pending);
    assert!(!persisted.store.dirty);
    assert!(runtime.continue_available());
}

#[test]
fn menu_exposes_only_working_v1_routes_and_native_quit_is_an_effect() {
    let mut runtime = runtime(MemoryWorkshopStore::default());
    let routes: Vec<_> = runtime
        .menu_capabilities()
        .into_iter()
        .map(|capability| capability.route)
        .collect();
    assert_eq!(
        routes,
        vec![
            MainMenuRoute::NewWorkshop,
            MainMenuRoute::Continue,
            MainMenuRoute::ClassicSector,
            MainMenuRoute::Settings,
            MainMenuRoute::Credits,
            MainMenuRoute::Quit,
        ]
    );

    assert_eq!(
        runtime.select_menu_route(MainMenuRoute::Credits).unwrap(),
        ClientRuntimeEffect::CreditsOpened
    );
    assert!(runtime.credits_visible());
    runtime.dismiss_credits();
    assert!(!runtime.credits_visible());

    runtime.select_menu_route(MainMenuRoute::Settings).unwrap();
    assert_eq!(runtime.screen(), ClientScreen::Settings);
    runtime.close_settings();
    assert_eq!(runtime.screen(), ClientScreen::MainMenu);
    assert_eq!(
        runtime.select_menu_route(MainMenuRoute::Quit).unwrap(),
        ClientRuntimeEffect::QuitRequested
    );
}

#[test]
fn workshop_is_not_a_rules_v1_app_mode() {
    fn exhaustive_classic_mode(mode: AppMode) -> &'static str {
        match mode {
            AppMode::Playing => "playing",
            AppMode::EditingScenario => "editing",
            AppMode::Settings => "settings",
        }
    }

    assert_eq!(exhaustive_classic_mode(AppMode::Playing), "playing");
    let source = include_str!("../src/app/core.rs");
    let enum_start = source.find("pub enum AppMode").unwrap();
    let enum_end = source[enum_start..].find('}').unwrap() + enum_start;
    assert!(!source[enum_start..=enum_end].contains("Workshop"));
}

#[test]
fn catalog_import_surfaces_a_failed_new_workshop_install_at_completion() {
    let pack_bytes = custom_pack();
    let expected = decode_catalog_pack(&pack_bytes).unwrap().catalog_hash();
    let mut runtime = runtime(MemoryWorkshopStore::default());
    runtime.start_new_workshop(7).unwrap();

    assert_eq!(
        runtime
            .begin_new_workshop_from_catalog(&pack_bytes, 0xCA7A10)
            .unwrap(),
        expected
    );
    assert!(runtime.catalog_import_active());

    // The resident Workshop acquires queued work while `PutPack` is pending, so
    // the replaceability predicate is false again when the import completes.
    let snapshot = runtime.workshop_snapshot().unwrap();
    let batch = CreatorBatchV1 {
        expected_cursor: snapshot.active_view.view_cursor,
        expected_tick: snapshot.active_view.tick,
        operations: vec![CreatorOpV1::CreateSystem {
            local: BatchLocalId(1),
            name: ObjectName::new("Pending Forge").unwrap(),
            position: GalaxyPointV1::new(1_024, 0).unwrap(),
        }],
    };
    runtime
        .enqueue_workshop_action(WorkshopAction::Submit(batch))
        .unwrap();

    runtime.update(Duration::ZERO);

    // The rejected install must reach the same recovery surface every other
    // completion failure in `poll_catalog_import` uses, not vanish.
    assert_eq!(runtime.screen(), ClientScreen::RecoverableError);
    assert_eq!(
        runtime.recovery_diagnostic().unwrap().code,
        ClientDiagnosticCode::RouteUnavailable
    );

    // Neither the resident session nor the persisted catalog is lost.
    assert!(!runtime.catalog_import_active());
    assert_eq!(runtime.imported_catalog_hash(), Some(expected));
    let ActiveSession::Workshop(workshop) = runtime.active_session() else {
        panic!("the resident Workshop did not survive the rejected install");
    };
    assert_eq!(workshop.history().genesis_seed()[..8], 7_u64.to_le_bytes());

    runtime.dismiss_recovery();
    assert_eq!(runtime.screen(), ClientScreen::GalaxyWorkshop);
}
