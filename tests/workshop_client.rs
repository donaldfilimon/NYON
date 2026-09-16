use std::{cell::RefCell, rc::Rc, time::Duration};

use nyon::{
    app::{
        AppCore, AppMode,
        client_runtime::{
            ActiveSession, CatalogImportStatus, ClientDiagnosticCode, ClientRuntime,
            ClientRuntimeEffect, ClientRuntimeError, ClientScreen, LibrarySlotsStatus,
            MainMenuRoute, SlotRequestKind,
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
            MemoryWorkshopStore, SlotId, SlotName, StoreJobId, StoreJobState, WorkshopStore,
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
    let WorkshopStoreResult::SlotCreated { slot, generation } = created else {
        panic!("unexpected create result: {created:?}");
    };
    complete(
        &mut store,
        WorkshopStoreRequest::SelectContinue {
            slot,
            expected_generation: generation,
        },
    );
    store
}

/// A store holding a committed slot with a valid head and **no** Continue
/// marker — the state addendum §12 requires be qualified as
/// "process failure after Commit or Promote but before Select Continue".
///
/// Deliberately distinct from `selected_store`: this one never issues
/// `SelectContinue`, and it commits so the head has genuinely advanced rather
/// than resting at creation.
fn committed_but_unselected_store(archive: Box<[u8]>) -> MemoryWorkshopStore {
    let mut store = MemoryWorkshopStore::default();
    let created = complete(
        &mut store,
        WorkshopStoreRequest::CreateSlot {
            name: SlotName::new("Two-System Forge").unwrap(),
            archive: archive.clone(),
        },
    );
    let WorkshopStoreResult::SlotCreated { slot, generation } = created else {
        panic!("unexpected create result: {created:?}");
    };
    let committed = complete(
        &mut store,
        WorkshopStoreRequest::CommitSlot {
            slot,
            expected_generation: generation,
            archive,
        },
    );
    assert!(
        matches!(committed, WorkshopStoreResult::SlotCommitted { .. }),
        "unexpected commit result: {committed:?}"
    );
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

    fn abandon(&mut self, job: StoreJobId) -> bool {
        if self
            .pending
            .as_ref()
            .is_some_and(|pending| pending.0 == job)
        {
            self.pending = None;
            return true;
        }
        self.inner.abandon(job)
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

/// A store the test can still read after the runtime has taken ownership of it.
#[derive(Clone, Default)]
struct SharedWorkshopStore(Rc<RefCell<MemoryWorkshopStore>>);

impl WorkshopStore for SharedWorkshopStore {
    fn start(&mut self, request: WorkshopStoreRequest) -> Result<StoreJobId, WorkshopStoreError> {
        self.0.borrow_mut().start(request)
    }

    fn abandon(&mut self, job: StoreJobId) -> bool {
        self.0.borrow_mut().abandon(job)
    }

    fn poll(&mut self, job: StoreJobId) -> StoreJobState {
        self.0.borrow_mut().poll(job)
    }
}

#[derive(Clone, Copy)]
enum PutPackFault {
    StartRejects,
    PollFails,
}

/// Fails the first `PutPack` only, so a retry from retained material succeeds.
struct FaultyPutPackStore {
    inner: SharedWorkshopStore,
    fault: Option<PutPackFault>,
    pending: Option<(StoreJobId, StoreJobState)>,
}

impl FaultyPutPackStore {
    fn new(inner: SharedWorkshopStore, fault: PutPackFault) -> Self {
        Self {
            inner,
            fault: Some(fault),
            pending: None,
        }
    }
}

impl WorkshopStore for FaultyPutPackStore {
    fn abandon(&mut self, job: StoreJobId) -> bool {
        if self
            .pending
            .as_ref()
            .is_some_and(|pending| pending.0 == job)
        {
            self.pending = None;
            return true;
        }
        self.inner.abandon(job)
    }

    fn start(&mut self, request: WorkshopStoreRequest) -> Result<StoreJobId, WorkshopStoreError> {
        if matches!(request, WorkshopStoreRequest::PutPack { .. }) {
            match self.fault.take() {
                Some(PutPackFault::StartRejects) => {
                    return Err(WorkshopStoreError::PackCapacity { max_packs: 4 });
                }
                Some(PutPackFault::PollFails) => {
                    let job = StoreJobId(u64::MAX);
                    self.pending = Some((
                        job,
                        StoreJobState::Complete(Err(WorkshopStoreError::QuotaExceeded {
                            max_bytes: 1,
                        })),
                    ));
                    return Ok(job);
                }
                None => {}
            }
        }
        self.inner.start(request)
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
fn continue_stays_disabled_for_a_committed_slot_that_was_never_selected() {
    // Addendum §12 names this race explicitly: "process failure after Commit
    // or Promote but before Select Continue", which "must reject or expose no
    // selected candidate rather than silently opening an unvalidated
    // generation". §4 states the same for the crash case.
    //
    // `continue_remains_disabled_when_storage_has_no_explicit_selection` looks
    // like it covers this and does not: it runs against an EMPTY store, so it
    // only proves "no slots means no Continue". The state that matters here is
    // the harder one — a slot that exists, whose head is valid and committed,
    // with the marker absent. Nothing exercised it at the runtime level until
    // now, and the commit that introduced clear-on-commit made it strictly
    // easier to reach.
    let mut runtime = runtime(committed_but_unselected_store(valid_archive(0x5EED)));

    assert_eq!(
        runtime.select_menu_route(MainMenuRoute::Continue),
        Err(ClientRuntimeError::ContinueUnavailable)
    );

    runtime.begin_continue_bootstrap().unwrap();
    runtime.update(Duration::ZERO);

    // Never silently open the unselected generation.
    assert!(!runtime.continue_available());
    assert_eq!(runtime.screen(), ClientScreen::MainMenu);
    assert!(matches!(runtime.active_session(), ActiveSession::None));
    // Not a fault: an unselected slot is an ordinary outcome, not recovery.
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

    // Content that never validates is a typed rejection, not a recoverable
    // error: it holds no retry material, so it leaves the runtime on its own
    // screen with no recovery obligation and an idle import machine.
    assert_eq!(runtime.screen(), ClientScreen::GalaxyWorkshop);
    assert!(runtime.recovery_diagnostic().is_none());
    assert_eq!(runtime.catalog_import_status(), CatalogImportStatus::Idle);
    assert_eq!(
        runtime.diagnostics().last().unwrap().code,
        ClientDiagnosticCode::Catalog
    );

    // The rejection did not consume the import machine either.
    let pack_bytes = custom_pack();
    let expected = decode_catalog_pack(&pack_bytes).unwrap().catalog_hash();
    assert_eq!(runtime.begin_catalog_import(&pack_bytes).unwrap(), expected);
    runtime.update(Duration::ZERO);
    assert_eq!(
        runtime.catalog_import_status(),
        CatalogImportStatus::Stored { hash: expected }
    );
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
fn library_is_a_lateral_menu_route_that_an_unsaved_resident_does_not_block() {
    // Addendum section 2 requires main-menu entry. New Workshop and Classic
    // Sector refuse while the resident Workshop holds unsaved work, because
    // they replace it; the Library replaces nothing, so it must stay open.
    let mut runtime = runtime(MemoryWorkshopStore::default());
    runtime.start_new_workshop(0x0118).unwrap();
    let initial = runtime.workshop_snapshot().unwrap();
    runtime
        .enqueue_workshop_action(WorkshopAction::Submit(CreatorBatchV1 {
            expected_cursor: initial.active_view.view_cursor,
            expected_tick: initial.active_view.tick,
            operations: vec![CreatorOpV1::CreateSystem {
                local: BatchLocalId(1),
                name: ObjectName::new("Lateral Forge").unwrap(),
                position: GalaxyPointV1::new(512, 512).unwrap(),
            }],
        }))
        .unwrap();
    runtime.update(Duration::ZERO);
    let digest = runtime.workshop_snapshot().unwrap().state_digest;
    runtime.return_to_main_menu().unwrap();
    assert_eq!(runtime.screen(), ClientScreen::MainMenu);

    let enabled = |runtime: &ClientRuntime<
        MemoryScenarioStore,
        MemoryPreferencesStore,
        MemoryWorkshopStore,
    >,
                   route| {
        runtime
            .menu_capabilities()
            .iter()
            .find(|capability| capability.route == route)
            .unwrap()
            .enabled
    };
    assert!(!enabled(&runtime, MainMenuRoute::NewWorkshop));
    assert!(enabled(&runtime, MainMenuRoute::Library));

    assert_eq!(
        runtime.select_menu_route(MainMenuRoute::Library),
        Ok(ClientRuntimeEffect::None)
    );
    assert_eq!(runtime.screen(), ClientScreen::Library);
    assert!(
        matches!(
            runtime.library_slots_status(),
            LibrarySlotsStatus::Working { .. }
        ),
        "entering from the menu must list the saves"
    );
    runtime.close_library();
    assert_eq!(runtime.screen(), ClientScreen::MainMenu);
    assert_eq!(runtime.workshop_snapshot().unwrap().state_digest, digest);
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
            MainMenuRoute::Library,
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
fn catalog_import_surfaces_a_failed_new_workshop_install_as_start_when_safe() {
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

    let diagnostics_before = runtime.diagnostics().len();
    runtime.update(Duration::ZERO);

    // The rejected install must still be surfaced rather than dropped, but as
    // the addendum's typed blocked-start state offering Start When Safe and
    // Cancel over a live runtime, not as the global recovery screen, which the
    // Library cannot present Retry/Cancel on top of.
    assert_eq!(
        runtime.catalog_import_status(),
        CatalogImportStatus::StartBlocked { hash: expected }
    );
    assert_eq!(runtime.screen(), ClientScreen::GalaxyWorkshop);
    assert!(runtime.recovery_diagnostic().is_none());

    // This is the regression guard for the defect that a rejected install was
    // silently discarded, so it must be attributable to this branch alone.
    // `install_new_workshop` already pushes its own generic `RouteUnavailable`
    // refusal through `ensure_resident_workshop_replaceable`, so the last code
    // by itself proves nothing: it still reads `RouteUnavailable` if the
    // import-specific push is deleted. Pin the count delta and the message only
    // this branch can produce.
    assert_eq!(runtime.diagnostics().len(), diagnostics_before + 2);
    let blocked = runtime.diagnostics().last().unwrap();
    assert_eq!(blocked.code, ClientDiagnosticCode::RouteUnavailable);
    assert!(
        blocked
            .message
            .starts_with("The imported Workshop catalog was saved,"),
        "the blocked-start diagnostic this branch owns is missing: {blocked:?}"
    );

    // Neither the resident session nor the persisted catalog is lost.
    assert!(!runtime.catalog_import_active());
    assert_eq!(runtime.imported_catalog_hash(), Some(expected));
    let ActiveSession::Workshop(workshop) = runtime.active_session() else {
        panic!("the resident Workshop did not survive the rejected install");
    };
    assert_eq!(workshop.history().genesis_seed()[..8], 7_u64.to_le_bytes());

    // Its replacement obligation survives too: the batch it accepted is still
    // unsaved, so replacement stays blocked exactly as it was.
    assert!(
        !runtime
            .menu_capabilities()
            .iter()
            .find(|capability| capability.route == MainMenuRoute::NewWorkshop)
            .unwrap()
            .enabled
    );

    // A blocked start survives further frames rather than being polled away,
    // and refuses a fresh import until the user resolves it.
    runtime.update(Duration::ZERO);
    runtime.update(Duration::ZERO);
    assert_eq!(
        runtime.catalog_import_status(),
        CatalogImportStatus::StartBlocked { hash: expected }
    );
    assert_eq!(
        runtime.begin_catalog_import(&pack_bytes),
        Err(ClientRuntimeError::CatalogImportActive)
    );

    // Start When Safe is refused while the resident Workshop still blocks
    // replacement, and stays retryable.
    assert_eq!(
        runtime.start_workshop_from_blocked_import(),
        Err(ClientRuntimeError::RouteUnavailable)
    );
    assert_eq!(
        runtime.catalog_import_status(),
        CatalogImportStatus::StartBlocked { hash: expected }
    );

    // Once the resident Workshop reaches a durable, Continue-selected save it
    // is replaceable again and the retained start completes.
    runtime.return_to_main_menu().unwrap();
    for _ in 0..8 {
        runtime.update(Duration::ZERO);
        if runtime.continue_available() {
            break;
        }
    }
    assert!(runtime.continue_available());
    runtime.start_workshop_from_blocked_import().unwrap();
    assert_eq!(runtime.screen(), ClientScreen::GalaxyWorkshop);
    assert_eq!(
        runtime.catalog_import_status(),
        CatalogImportStatus::Stored { hash: expected }
    );
    let ActiveSession::Workshop(workshop) = runtime.active_session() else {
        panic!("Start When Safe did not install the imported-catalog Workshop");
    };
    assert_eq!(
        workshop.history().genesis_seed()[..8],
        0xCA7A10_u64.to_le_bytes()
    );
}

#[test]
fn catalog_import_store_start_failure_is_retryable_without_entering_recovery() {
    let pack_bytes = custom_pack();
    let expected = decode_catalog_pack(&pack_bytes).unwrap().catalog_hash();
    let shared = SharedWorkshopStore::default();
    let mut runtime = ClientRuntime::new(
        classic(),
        FaultyPutPackStore::new(shared.clone(), PutPackFault::StartRejects),
    );
    runtime.start_new_workshop(11).unwrap();
    let before = runtime.workshop_snapshot().unwrap().state_digest;

    assert!(matches!(
        runtime.begin_catalog_import(&pack_bytes),
        Err(ClientRuntimeError::Store(_))
    ));

    let failed = CatalogImportStatus::StoreFailed {
        expected_hash: expected,
        code: ClientDiagnosticCode::Store,
        starts_new_workshop: false,
    };
    assert_eq!(runtime.catalog_import_status(), failed);
    assert!(!runtime.catalog_import_active());
    assert_eq!(runtime.screen(), ClientScreen::GalaxyWorkshop);
    assert!(runtime.recovery_diagnostic().is_none());
    assert_eq!(runtime.workshop_snapshot().unwrap().state_digest, before);
    assert!(runtime.imported_catalog_hash().is_none());

    // The failure waits on the user, so further frames must not poll it away,
    // and a fresh import may not silently discard the retained pack.
    runtime.update(Duration::ZERO);
    runtime.update(Duration::ZERO);
    assert_eq!(runtime.catalog_import_status(), failed);
    assert_eq!(
        runtime.begin_catalog_import(&pack_bytes),
        Err(ClientRuntimeError::CatalogImportActive)
    );

    // Retry resumes from that retained pack rather than asking for the file.
    assert_eq!(runtime.retry_catalog_import().unwrap(), expected);
    assert_eq!(
        runtime.catalog_import_status(),
        CatalogImportStatus::Storing {
            expected_hash: expected,
            starts_new_workshop: false,
        }
    );
    runtime.update(Duration::ZERO);
    assert_eq!(
        runtime.catalog_import_status(),
        CatalogImportStatus::Stored { hash: expected }
    );
    assert_eq!(runtime.imported_catalog_hash(), Some(expected));
    assert_eq!(runtime.screen(), ClientScreen::GalaxyWorkshop);
    assert!(runtime.recovery_diagnostic().is_none());
    assert_eq!(runtime.workshop_snapshot().unwrap().state_digest, before);
}

#[test]
fn catalog_import_poll_failure_offers_cancel_without_touching_the_session() {
    let pack_bytes = custom_pack();
    let expected = decode_catalog_pack(&pack_bytes).unwrap().catalog_hash();
    let shared = SharedWorkshopStore::default();
    let mut runtime = ClientRuntime::new(
        classic(),
        FaultyPutPackStore::new(shared.clone(), PutPackFault::PollFails),
    );
    runtime.start_new_workshop(13).unwrap();
    let before = runtime.workshop_snapshot().unwrap().state_digest;

    assert_eq!(
        runtime
            .begin_new_workshop_from_catalog(&pack_bytes, 0xFA11)
            .unwrap(),
        expected
    );
    assert!(runtime.catalog_import_active());
    runtime.update(Duration::ZERO);

    assert_eq!(
        runtime.catalog_import_status(),
        CatalogImportStatus::StoreFailed {
            expected_hash: expected,
            code: ClientDiagnosticCode::Store,
            starts_new_workshop: true,
        }
    );
    assert!(!runtime.catalog_import_active());
    assert_eq!(runtime.screen(), ClientScreen::GalaxyWorkshop);
    assert!(runtime.recovery_diagnostic().is_none());
    assert_eq!(runtime.workshop_snapshot().unwrap().state_digest, before);
    assert!(runtime.imported_catalog_hash().is_none());
    let ActiveSession::Workshop(workshop) = runtime.active_session() else {
        panic!("the resident Workshop did not survive the failed import");
    };
    assert_eq!(workshop.history().genesis_seed()[..8], 13_u64.to_le_bytes());

    // Cancel clears only the pending client intent.
    runtime.cancel_catalog_import().unwrap();
    assert_eq!(runtime.catalog_import_status(), CatalogImportStatus::Idle);
    assert_eq!(
        runtime.retry_catalog_import(),
        Err(ClientRuntimeError::RouteUnavailable)
    );
    assert_eq!(runtime.workshop_snapshot().unwrap().state_digest, before);

    // With the intent released, a fresh import is accepted again.
    assert_eq!(runtime.begin_catalog_import(&pack_bytes).unwrap(), expected);
    runtime.update(Duration::ZERO);
    assert_eq!(
        runtime.catalog_import_status(),
        CatalogImportStatus::Stored { hash: expected }
    );
}

#[test]
fn cancelling_a_blocked_catalog_import_never_deletes_the_stored_pack() {
    let pack_bytes = custom_pack();
    let expected = decode_catalog_pack(&pack_bytes).unwrap().catalog_hash();
    let shared = SharedWorkshopStore::default();
    let mut runtime = ClientRuntime::new(classic(), shared.clone());
    runtime.start_new_workshop(12).unwrap();
    assert_eq!(
        runtime
            .begin_new_workshop_from_catalog(&pack_bytes, 0xB10C)
            .unwrap(),
        expected
    );

    // The resident Workshop acquires queued work while `PutPack` is pending, so
    // the requested replacement is refused at import completion.
    let snapshot = runtime.workshop_snapshot().unwrap();
    let batch = CreatorBatchV1 {
        expected_cursor: snapshot.active_view.view_cursor,
        expected_tick: snapshot.active_view.tick,
        operations: vec![CreatorOpV1::CreateSystem {
            local: BatchLocalId(1),
            name: ObjectName::new("Blocked Forge").unwrap(),
            position: GalaxyPointV1::new(2_048, 0).unwrap(),
        }],
    };
    runtime
        .enqueue_workshop_action(WorkshopAction::Submit(batch))
        .unwrap();
    runtime.update(Duration::ZERO);
    assert_eq!(
        runtime.catalog_import_status(),
        CatalogImportStatus::StartBlocked { hash: expected }
    );

    let digest = runtime.workshop_snapshot().unwrap().state_digest;
    runtime.cancel_catalog_import().unwrap();

    assert_eq!(runtime.catalog_import_status(), CatalogImportStatus::Idle);
    assert_eq!(runtime.workshop_snapshot().unwrap().state_digest, digest);
    assert_eq!(runtime.screen(), ClientScreen::GalaxyWorkshop);
    assert!(runtime.recovery_diagnostic().is_none());
    assert_eq!(
        runtime.start_workshop_from_blocked_import(),
        Err(ClientRuntimeError::RouteUnavailable)
    );

    // The pack that already reached durable storage is still there, byte for
    // byte, and is still the recorded imported catalog.
    assert_eq!(runtime.imported_catalog_hash(), Some(expected));
    let loaded = complete(
        &mut *shared.0.borrow_mut(),
        WorkshopStoreRequest::GetPack { hash: expected },
    );
    let WorkshopStoreResult::PackLoaded {
        hash,
        canonical_pack,
    } = loaded
    else {
        panic!("cancelling the import removed the stored pack: {loaded:?}");
    };
    assert_eq!(hash, expected);
    assert_eq!(canonical_pack.as_ref(), pack_bytes.as_slice());
}

#[test]
fn cancelling_an_in_flight_catalog_import_frees_the_commit_lane_it_held() {
    // The inverse of the contract `ff9b6ca` shipped. That commit had to refuse
    // this cancel, because the only way out of a reserved Commit lane was to
    // poll the very job the caller had given up on, and dropping the job would
    // starve the resident Workshop's own save and its recovery-persistence
    // obligation. `WorkshopStore::abandon` is the clean escape the Library
    // addendum's section 6 needs, so `Storing` becomes cancellable.
    let pack_bytes = custom_pack();
    let expected = decode_catalog_pack(&pack_bytes).unwrap().catalog_hash();
    let shared = SharedWorkshopStore::default();
    let mut runtime = ClientRuntime::new(classic(), shared.clone());
    runtime.start_new_workshop(14).unwrap();
    assert_eq!(runtime.begin_catalog_import(&pack_bytes).unwrap(), expected);
    assert_eq!(
        runtime.catalog_import_status(),
        CatalogImportStatus::Storing {
            expected_hash: expected,
            starts_new_workshop: false,
        }
    );

    // The lane is genuinely occupied while the import is in flight: this is the
    // wedge, observed rather than assumed.
    assert!(matches!(
        shared.clone().start(WorkshopStoreRequest::PutPack {
            canonical_pack: Box::from(&pack_bytes[..]),
        }),
        Err(WorkshopStoreError::Busy { .. })
    ));

    // Cancellation is ordinary, so it is neither a diagnostic nor a recovery.
    let diagnostics_before = runtime.diagnostics().len();
    runtime.cancel_catalog_import().unwrap();
    assert_eq!(runtime.catalog_import_status(), CatalogImportStatus::Idle);
    assert!(!runtime.catalog_import_active());
    assert_eq!(runtime.diagnostics().len(), diagnostics_before);
    assert_eq!(runtime.screen(), ClientScreen::GalaxyWorkshop);
    assert!(runtime.recovery_diagnostic().is_none());

    // The invariant, not the implementation: the resident Workshop can save
    // again. Under the old refusal this same sequence left the lane occupied
    // for the store's lifetime.
    runtime
        .enqueue_workshop_action(WorkshopAction::RequestSave)
        .unwrap();
    for _ in 0..8 {
        runtime.update(Duration::ZERO);
        if runtime
            .workshop_snapshot()
            .unwrap()
            .store
            .generation
            .is_some()
        {
            break;
        }
    }
    assert!(
        runtime
            .workshop_snapshot()
            .unwrap()
            .store
            .generation
            .is_some(),
        "the resident Workshop must be able to commit after the cancel"
    );

    // Abandoning drops the answer, never the write. The memory adapter had
    // already stored the pack when `start` returned, and cancel does not, and
    // must not, withdraw it -- the request vocabulary has no pack deletion.
    let loaded = complete(
        &mut *shared.clone().0.borrow_mut(),
        WorkshopStoreRequest::GetPack { hash: expected },
    );
    let WorkshopStoreResult::PackLoaded { hash, .. } = loaded else {
        panic!("cancelling an in-flight import withdrew the pack: {loaded:?}");
    };
    assert_eq!(hash, expected);

    // Idle has no intent left to cancel.
    assert_eq!(
        runtime.cancel_catalog_import(),
        Err(ClientRuntimeError::RouteUnavailable)
    );

    // ...and neither does a terminal `Stored`. This half was asserted by the
    // test this one replaced, and was lost in the inversion. Restored because
    // nothing else pins it: moving `CatalogImport::Stored` into
    // `cancel_catalog_import`'s `Ok(())` arm -- making a stored import silently
    // cancellable -- passed workshop_client, workshop_library,
    // workshop_recovery and workshop_session in full.
    //
    // The distinction the branch encodes: cancelling an *in-flight* import
    // abandons an outcome that has not been consumed, while a `Stored` import
    // has already completed. There is no pack deletion in the request
    // vocabulary, so "cancelling" it could only mean forgetting a pack that is
    // still on disk -- a lie about storage rather than an undo.
    //
    // Drain first: the save above is a two-step sequence -- commit, then the
    // addendum-mandated generation-checked SelectContinue -- and that second
    // job is Commit-class too, so beginning an import before it retires fails
    // with `Busy { class: Commit }` rather than testing anything.
    for _ in 0..8 {
        runtime.update(Duration::ZERO);
    }
    let stored_pack = custom_pack();
    let stored_hash = decode_catalog_pack(&stored_pack).unwrap().catalog_hash();
    assert_eq!(
        runtime.begin_catalog_import(&stored_pack).unwrap(),
        stored_hash
    );
    runtime.update(Duration::ZERO);
    assert_eq!(
        runtime.catalog_import_status(),
        CatalogImportStatus::Stored { hash: stored_hash }
    );
    assert_eq!(
        runtime.cancel_catalog_import(),
        Err(ClientRuntimeError::RouteUnavailable),
        "a terminal Stored import has no intent left to cancel"
    );
    assert_eq!(
        runtime.catalog_import_status(),
        CatalogImportStatus::Stored { hash: stored_hash },
        "a refused cancel must not disturb the stored import"
    );
}

/// The exact-catalog open algorithm's own protocol messages, pinned before the
/// algorithm moved into `WorkshopLibraryClient`.
///
/// Every other bootstrap test asserts a [`ClientDiagnosticCode`], which is a
/// seven-value enum: three of the five arms below collapse onto
/// `StoreProtocol`, so a code-only suite cannot tell a mis-copied arm from a
/// correct one. These strings are the arm identities. They exist so that a
/// behaviour-preserving extraction has evidence beyond a green gate.
#[derive(Clone, Copy)]
enum BootstrapFault {
    ForgotListJob,
    ArchivedSelection,
    WrongLoadedSlot,
    InconsistentGenerations,
}

/// Corrupts exactly one store answer inside the bootstrap pipeline, leaving
/// every lane correctly released so the fault under test is the only one.
struct FaultyBootstrapStore {
    inner: MemoryWorkshopStore,
    fault: BootstrapFault,
    list_job: Option<StoreJobId>,
    load_job: Option<StoreJobId>,
}

impl FaultyBootstrapStore {
    fn new(inner: MemoryWorkshopStore, fault: BootstrapFault) -> Self {
        Self {
            inner,
            fault,
            list_job: None,
            load_job: None,
        }
    }
}

impl WorkshopStore for FaultyBootstrapStore {
    fn abandon(&mut self, job: StoreJobId) -> bool {
        self.inner.abandon(job)
    }

    fn start(&mut self, request: WorkshopStoreRequest) -> Result<StoreJobId, WorkshopStoreError> {
        let is_list = matches!(request, WorkshopStoreRequest::ListSlots);
        let is_load = matches!(request, WorkshopStoreRequest::LoadSlot { .. });
        let job = self.inner.start(request)?;
        if is_list {
            self.list_job = Some(job);
        }
        if is_load {
            self.load_job = Some(job);
        }
        Ok(job)
    }

    fn poll(&mut self, job: StoreJobId) -> StoreJobState {
        if self.list_job == Some(job) && matches!(self.fault, BootstrapFault::ForgotListJob) {
            // Consume the real job first so the lane is genuinely released;
            // the client must cope with the *answer* going missing, not with a
            // second wedged lane.
            let _ = self.inner.poll(job);
            self.list_job = None;
            return StoreJobState::Unknown;
        }
        let mut state = self.inner.poll(job);
        if self.list_job == Some(job)
            && let StoreJobState::Complete(Ok(WorkshopStoreResult::Slots(list))) = &mut state
            && matches!(self.fault, BootstrapFault::ArchivedSelection)
        {
            for summary in &mut list.slots {
                summary.archived = true;
            }
        }
        if self.load_job == Some(job)
            && let StoreJobState::Complete(Ok(WorkshopStoreResult::SlotLoaded(loaded))) = &mut state
        {
            match self.fault {
                BootstrapFault::WrongLoadedSlot => {
                    loaded.slot = nyon::workshop::store::SlotId(loaded.slot.0 + 1);
                }
                BootstrapFault::InconsistentGenerations => {
                    loaded.head_generation =
                        nyon::workshop::store::SaveGeneration(loaded.generation.0 + 1);
                }
                BootstrapFault::ForgotListJob | BootstrapFault::ArchivedSelection => {}
            }
        }
        state
    }
}

#[test]
fn continue_bootstrap_protocol_messages_are_the_pinned_arm_identities() {
    let expected: [(BootstrapFault, &str); 4] = [
        (
            BootstrapFault::ForgotListJob,
            "Workshop storage forgot the active slot-list job",
        ),
        (
            BootstrapFault::ArchivedSelection,
            "The explicitly selected Continue slot is missing or archived",
        ),
        (
            BootstrapFault::WrongLoadedSlot,
            "Workshop storage loaded a slot other than the selected Continue slot",
        ),
        (
            BootstrapFault::InconsistentGenerations,
            "Workshop storage returned inconsistent loaded and head generations",
        ),
    ];

    for (fault, message) in expected {
        let inner = selected_store(valid_archive(0x9A11));
        let mut runtime = ClientRuntime::new(classic(), FaultyBootstrapStore::new(inner, fault));
        runtime.begin_continue_bootstrap().unwrap();
        finish_continue(&mut runtime);

        let diagnostic = runtime.recovery_diagnostic().unwrap();
        assert_eq!(diagnostic.code, ClientDiagnosticCode::StoreProtocol);
        assert_eq!(diagnostic.message, message);
        assert_eq!(runtime.screen(), ClientScreen::RecoverableError);
        // A protocol failure always drops the candidate; it never leaves a
        // half-validated Continue behind.
        assert!(!runtime.continue_available());
    }

    // The catalog arm is a fifth `StoreProtocol` message and reaches the same
    // sink through `LoadingCatalog` rather than `Listing` or `Loading`.
    let (_, archive) = custom_archive(0x9A12);
    let inner = selected_custom_store(None, archive);
    let mut runtime =
        ClientRuntime::new(classic(), FaultyPackStore::new(inner, PackFault::WrongHash));
    runtime.begin_continue_bootstrap().unwrap();
    finish_continue(&mut runtime);
    assert_eq!(
        runtime.recovery_diagnostic().unwrap().message,
        "Workshop storage returned a catalog other than the archive's exact catalog"
    );
}

#[test]
fn recovered_predecessor_continue_keeps_its_exact_offer_message_and_candidate() {
    // Head generation 2 is structurally valid JSON that cannot replay, so the
    // pipeline falls back to the retained predecessor at generation 1.
    let mut store = MemoryWorkshopStore::default();
    let created = complete(
        &mut store,
        WorkshopStoreRequest::CreateSlot {
            name: SlotName::new("Two-System Forge").unwrap(),
            archive: valid_archive(0x9A13),
        },
    );
    let WorkshopStoreResult::SlotCreated { slot, generation } = created else {
        panic!("unexpected create result: {created:?}");
    };
    let committed = complete(
        &mut store,
        WorkshopStoreRequest::CommitSlot {
            slot,
            expected_generation: generation,
            archive: Box::from(&b"{}"[..]),
        },
    );
    let WorkshopStoreResult::SlotCommitted {
        generation: head, ..
    } = committed
    else {
        panic!("unexpected commit result: {committed:?}");
    };
    complete(
        &mut store,
        WorkshopStoreRequest::SelectContinue {
            slot,
            expected_generation: head,
        },
    );

    let mut runtime = runtime(store);
    runtime.begin_continue_bootstrap().unwrap();
    finish_continue(&mut runtime);

    // The offer is a recovery screen that still holds a usable candidate:
    // "recovered" is not "failed", and the two differ only by this message.
    assert_eq!(runtime.screen(), ClientScreen::RecoverableError);
    let diagnostic = runtime.recovery_diagnostic().unwrap();
    assert_eq!(diagnostic.code, ClientDiagnosticCode::Archive);
    assert_eq!(
        diagnostic.message,
        "The latest save was invalid; a previous valid generation is available"
    );
    assert!(runtime.continue_available());
}

// ---------------------------------------------------------------------------
// Library route: `ClientScreen::Library`, return handling, and the poll machine
// over `ListSlots`, `RenameSlot`, `ArchiveSlot` and `UnarchiveSlot`.
//
// The route design's client-suite obligations are exercised here:
// round-tripping from all three prior screens with no durable transition side
// effect, the gating matrix under a dirty resident session, and a store failure
// that yields Retry/Cancel and never `RecoverableError`.
// ---------------------------------------------------------------------------

/// Two slots, neither selected for Continue, so a Library test can mutate one
/// row without disturbing the other and without a Continue marker in play.
fn two_slot_store() -> (MemoryWorkshopStore, SlotId, SlotId) {
    let mut store = MemoryWorkshopStore::default();
    let mut created = |name: &str, seed: u64| {
        let result = complete(
            &mut store,
            WorkshopStoreRequest::CreateSlot {
                name: SlotName::new(name).unwrap(),
                archive: valid_archive(seed),
            },
        );
        let WorkshopStoreResult::SlotCreated { slot, .. } = result else {
            panic!("unexpected create result: {result:?}");
        };
        slot
    };
    let first = created("First Forge", 11);
    let second = created("Second Forge", 12);
    (store, first, second)
}

/// Drives the runtime until the Library slot machine is idle again, bounded.
fn settle_library<W: WorkshopStore>(
    runtime: &mut ClientRuntime<MemoryScenarioStore, MemoryPreferencesStore, W>,
) {
    for _ in 0..16 {
        if runtime.library_slots_status() == LibrarySlotsStatus::Idle {
            return;
        }
        runtime.update(Duration::ZERO);
    }
    panic!("Library slot machine exceeded its bounded test polls");
}

fn slot_named<W: WorkshopStore>(
    runtime: &ClientRuntime<MemoryScenarioStore, MemoryPreferencesStore, W>,
    slot: SlotId,
) -> String {
    runtime
        .library_slots()
        .expect("the Library has listed")
        .slots
        .iter()
        .find(|summary| summary.id == slot)
        .expect("the listed slots contain the row")
        .name
        .as_str()
        .to_owned()
}

fn slot_archived<W: WorkshopStore>(
    runtime: &ClientRuntime<MemoryScenarioStore, MemoryPreferencesStore, W>,
    slot: SlotId,
) -> bool {
    runtime
        .library_slots()
        .expect("the Library has listed")
        .slots
        .iter()
        .find(|summary| summary.id == slot)
        .expect("the listed slots contain the row")
        .archived
}

#[derive(Clone, Copy, Eq, PartialEq)]
enum SlotFault {
    List,
    Rename,
    Archive,
}

/// Fails the first matching Library slot request only, so a Retry from the
/// retained request succeeds. Modelled on `FaultyPutPackStore`: the failure is
/// delivered through `poll`, which is the path the runtime's own state machine
/// takes.
struct FaultySlotStore {
    inner: MemoryWorkshopStore,
    fault: Option<SlotFault>,
    pending: Option<(StoreJobId, StoreJobState)>,
}

impl FaultySlotStore {
    fn new(inner: MemoryWorkshopStore, fault: SlotFault) -> Self {
        Self {
            inner,
            fault: Some(fault),
            pending: None,
        }
    }
}

impl WorkshopStore for FaultySlotStore {
    fn start(&mut self, request: WorkshopStoreRequest) -> Result<StoreJobId, WorkshopStoreError> {
        let matched = matches!(
            (&request, self.fault),
            (WorkshopStoreRequest::ListSlots, Some(SlotFault::List))
                | (
                    WorkshopStoreRequest::RenameSlot { .. },
                    Some(SlotFault::Rename)
                )
                | (
                    WorkshopStoreRequest::ArchiveSlot { .. },
                    Some(SlotFault::Archive)
                )
        );
        if matched {
            self.fault = None;
            let job = StoreJobId(u64::MAX);
            self.pending = Some((
                job,
                StoreJobState::Complete(Err(WorkshopStoreError::CorruptManifest)),
            ));
            return Ok(job);
        }
        self.inner.start(request)
    }

    fn abandon(&mut self, job: StoreJobId) -> bool {
        if self
            .pending
            .as_ref()
            .is_some_and(|pending| pending.0 == job)
        {
            self.pending = None;
            return true;
        }
        self.inner.abandon(job)
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
fn library_round_trips_from_each_of_the_three_prior_screens() {
    let (store, _, _) = two_slot_store();
    let mut runtime = runtime(store);

    // Main menu: the addendum's required entry point, where there is no
    // Workshop frame at all and a drawer section could not exist.
    assert_eq!(runtime.screen(), ClientScreen::MainMenu);
    runtime.open_library().unwrap();
    assert_eq!(runtime.screen(), ClientScreen::Library);
    runtime.close_library();
    assert_eq!(runtime.screen(), ClientScreen::MainMenu);

    runtime
        .select_menu_route(MainMenuRoute::ClassicSector)
        .unwrap();
    assert_eq!(runtime.screen(), ClientScreen::ClassicSector);
    runtime.open_library().unwrap();
    assert_eq!(runtime.screen(), ClientScreen::Library);
    runtime.close_library();
    assert_eq!(runtime.screen(), ClientScreen::ClassicSector);

    runtime.start_new_workshop(21).unwrap();
    assert_eq!(runtime.screen(), ClientScreen::GalaxyWorkshop);
    runtime.open_library().unwrap();
    assert_eq!(runtime.screen(), ClientScreen::Library);
    runtime.close_library();
    assert_eq!(runtime.screen(), ClientScreen::GalaxyWorkshop);
}

#[test]
fn opening_the_library_prepares_no_durable_transition() {
    // The discriminating comparison is against `return_to_main_menu`, which
    // deliberately pauses and saves. Library is a lateral route: §2 requires the
    // resident session and its recovery obligations to survive opening it
    // untouched, so the dirty flag must still be set and no commit may start.
    let mut runtime = runtime(MemoryWorkshopStore::default());
    runtime.start_new_workshop(0x11B).unwrap();
    let snapshot = runtime.workshop_snapshot().unwrap();
    let batch = CreatorBatchV1 {
        expected_cursor: snapshot.active_view.view_cursor,
        expected_tick: snapshot.active_view.tick,
        operations: vec![CreatorOpV1::CreateSystem {
            local: BatchLocalId(1),
            name: ObjectName::new("Library Forge").unwrap(),
            position: GalaxyPointV1::new(512, 256).unwrap(),
        }],
    };
    runtime
        .enqueue_workshop_action(WorkshopAction::Submit(batch))
        .unwrap();
    runtime.update(Duration::ZERO);
    let dirty_digest = runtime.workshop_snapshot().unwrap().state_digest;
    assert!(runtime.workshop_snapshot().unwrap().store.dirty);
    assert!(!runtime.workshop_snapshot().unwrap().store.commit_pending);

    runtime.open_library().unwrap();
    for _ in 0..4 {
        runtime.update(Duration::ZERO);
    }

    assert_eq!(runtime.screen(), ClientScreen::Library);
    let during = runtime.workshop_snapshot().unwrap();
    assert!(during.store.dirty, "Library must not clear the dirty flag");
    assert!(
        !during.store.commit_pending,
        "Library must not start a durable save"
    );
    assert_eq!(during.state_digest, dirty_digest);

    runtime.close_library();
    for _ in 0..4 {
        runtime.update(Duration::ZERO);
    }
    assert_eq!(runtime.screen(), ClientScreen::GalaxyWorkshop);
    let after = runtime.workshop_snapshot().unwrap();
    assert!(after.store.dirty);
    assert!(!after.store.commit_pending);
    assert_eq!(after.state_digest, dirty_digest);
}

#[test]
fn library_returns_to_the_main_menu_a_resident_workshop_was_left_on() {
    // `return_to_main_menu` leaves the Workshop resident, so the screen being
    // left and `screen_for_active_session` disagree here. The addendum's "exact
    // prior screen" is the screen, not the session.
    let mut runtime = runtime(MemoryWorkshopStore::default());
    runtime.start_new_workshop(0x5EED).unwrap();
    runtime.return_to_main_menu().unwrap();
    assert_eq!(runtime.screen(), ClientScreen::MainMenu);
    assert!(matches!(
        runtime.active_session(),
        ActiveSession::Workshop(_)
    ));

    runtime.open_library().unwrap();
    runtime.close_library();

    assert_eq!(runtime.screen(), ClientScreen::MainMenu);
    assert!(matches!(
        runtime.active_session(),
        ActiveSession::Workshop(_)
    ));
}

#[test]
fn library_refuses_reentry_and_both_transient_screens() {
    let (store, _, _) = two_slot_store();
    let mut reentry = runtime(store);
    reentry.open_library().unwrap();
    settle_library(&mut reentry);

    // Re-entry would make the Library its own return screen and strand the
    // user, so it is refused rather than absorbed.
    assert_eq!(
        reentry.open_library(),
        Err(ClientRuntimeError::RouteUnavailable)
    );
    assert_eq!(reentry.screen(), ClientScreen::Library);
    reentry.close_library();
    assert_eq!(reentry.screen(), ClientScreen::MainMenu);

    // Loading is owned by the bootstrap, which writes `screen` itself.
    let mut loading = runtime(selected_store(valid_archive(31)));
    loading.begin_continue_bootstrap().unwrap();
    assert_eq!(loading.screen(), ClientScreen::Loading);
    assert_eq!(
        loading.open_library(),
        Err(ClientRuntimeError::RouteUnavailable)
    );
    assert_eq!(loading.screen(), ClientScreen::Loading);

    let (_, archive) = custom_archive(407);
    let mut recovered = runtime(selected_custom_store(None, archive));
    recovered.begin_continue_bootstrap().unwrap();
    finish_continue(&mut recovered);
    assert_eq!(recovered.screen(), ClientScreen::RecoverableError);
    assert_eq!(
        recovered.open_library(),
        Err(ClientRuntimeError::RouteUnavailable)
    );
    assert_eq!(recovered.screen(), ClientScreen::RecoverableError);
}

#[test]
fn library_lists_slots_and_re_lists_after_every_successful_mutation() {
    let (store, first, second) = two_slot_store();
    let mut runtime = runtime(store);
    assert!(runtime.library_slots().is_none());

    runtime.open_library().unwrap();
    settle_library(&mut runtime);
    assert_eq!(runtime.library_slots().unwrap().slots.len(), 2);
    assert_eq!(slot_named(&runtime, first), "First Forge");
    assert!(!slot_archived(&runtime, second));

    runtime
        .rename_library_slot(first, SlotName::new("Renamed Forge").unwrap())
        .unwrap();
    assert_eq!(
        runtime.library_slots_status(),
        LibrarySlotsStatus::Working {
            kind: SlotRequestKind::Rename,
            slot: Some(first),
        }
    );
    runtime.update(Duration::ZERO);
    // The cached list is dropped the instant the mutation lands: it now
    // misstates a name, and a row activated from it would carry a generation
    // the mutation invalidated.
    assert!(runtime.library_slots().is_none());
    assert_eq!(
        runtime.library_slots_status(),
        LibrarySlotsStatus::Working {
            kind: SlotRequestKind::List,
            slot: None,
        }
    );
    settle_library(&mut runtime);
    assert_eq!(slot_named(&runtime, first), "Renamed Forge");

    runtime.archive_library_slot(second).unwrap();
    settle_library(&mut runtime);
    assert!(slot_archived(&runtime, second));
    assert!(!slot_archived(&runtime, first));

    runtime.unarchive_library_slot(second).unwrap();
    settle_library(&mut runtime);
    assert!(!slot_archived(&runtime, second));
    assert_eq!(slot_named(&runtime, second), "Second Forge");
    assert_eq!(runtime.screen(), ClientScreen::Library);
    assert!(runtime.recovery_diagnostic().is_none());
}

#[test]
fn library_slot_mutations_proceed_while_the_resident_workshop_is_dirty() {
    // §6: Rename and Unarchive for unrelated slots may proceed while an active
    // session is not replaceable. Nothing in the slot paths may call
    // `ensure_resident_workshop_replaceable`.
    let (store, first, second) = two_slot_store();
    let mut runtime = runtime(store);
    runtime.start_new_workshop(0xD127).unwrap();
    let snapshot = runtime.workshop_snapshot().unwrap();
    runtime
        .enqueue_workshop_action(WorkshopAction::Submit(CreatorBatchV1 {
            expected_cursor: snapshot.active_view.view_cursor,
            expected_tick: snapshot.active_view.tick,
            operations: vec![CreatorOpV1::CreateSystem {
                local: BatchLocalId(1),
                name: ObjectName::new("Dirty Forge").unwrap(),
                position: GalaxyPointV1::new(64, 64).unwrap(),
            }],
        }))
        .unwrap();
    runtime.update(Duration::ZERO);
    assert!(runtime.workshop_snapshot().unwrap().store.dirty);
    // The session genuinely blocks replacement: New Workshop is refused.
    assert_eq!(
        runtime.start_new_workshop(1),
        Err(ClientRuntimeError::RouteUnavailable)
    );

    runtime.open_library().unwrap();
    settle_library(&mut runtime);

    runtime
        .rename_library_slot(first, SlotName::new("Dirty Rename").unwrap())
        .unwrap();
    settle_library(&mut runtime);
    assert_eq!(slot_named(&runtime, first), "Dirty Rename");

    runtime.archive_library_slot(second).unwrap();
    settle_library(&mut runtime);
    assert!(slot_archived(&runtime, second));

    runtime.unarchive_library_slot(second).unwrap();
    settle_library(&mut runtime);
    assert!(!slot_archived(&runtime, second));
    assert!(runtime.workshop_snapshot().unwrap().store.dirty);
}

#[test]
fn the_resident_workshops_own_slot_cannot_be_archived() {
    // §3: archiving the slot a live authoritative session occupies would leave
    // that session's next save failing as archived.
    let mut store = MemoryWorkshopStore::default();
    let created = complete(
        &mut store,
        WorkshopStoreRequest::CreateSlot {
            name: SlotName::new("Resident Forge").unwrap(),
            archive: valid_archive(41),
        },
    );
    let WorkshopStoreResult::SlotCreated { slot, generation } = created else {
        panic!("unexpected create result: {created:?}");
    };
    complete(
        &mut store,
        WorkshopStoreRequest::SelectContinue {
            slot,
            expected_generation: generation,
        },
    );
    let other = complete(
        &mut store,
        WorkshopStoreRequest::CreateSlot {
            name: SlotName::new("Spare Forge").unwrap(),
            archive: valid_archive(42),
        },
    );
    let WorkshopStoreResult::SlotCreated { slot: other, .. } = other else {
        panic!("unexpected create result: {other:?}");
    };

    let mut runtime = runtime(store);
    runtime.begin_continue_bootstrap().unwrap();
    finish_continue(&mut runtime);
    runtime
        .select_menu_route(MainMenuRoute::Continue)
        .unwrap_or_else(|error| panic!("Continue was refused: {error}"));
    assert_eq!(
        runtime.workshop_snapshot().unwrap().store.slot,
        Some(slot),
        "the resident session must occupy the selected slot"
    );

    runtime.open_library().unwrap();
    settle_library(&mut runtime);

    assert_eq!(
        runtime.archive_library_slot(slot),
        Err(ClientRuntimeError::ResidentSlot)
    );
    // A gating refusal starts no request at all, so nothing is left for Retry
    // or Cancel to resolve and no lane was reserved.
    assert_eq!(runtime.library_slots_status(), LibrarySlotsStatus::Idle);
    assert!(!slot_archived(&runtime, slot));

    // The unrelated row is unaffected by the refusal.
    runtime.archive_library_slot(other).unwrap();
    settle_library(&mut runtime);
    assert!(slot_archived(&runtime, other));
    assert!(!slot_archived(&runtime, slot));
}

#[test]
fn archiving_the_validated_continue_candidate_withdraws_it() {
    // The store clears its own Continue marker on Archive, but a candidate the
    // runtime already validated would survive that and could still be installed
    // as resident — an archived slot whose next save must fail as archived.
    let mut store = MemoryWorkshopStore::default();
    let created = complete(
        &mut store,
        WorkshopStoreRequest::CreateSlot {
            name: SlotName::new("Candidate Forge").unwrap(),
            archive: valid_archive(51),
        },
    );
    let WorkshopStoreResult::SlotCreated { slot, generation } = created else {
        panic!("unexpected create result: {created:?}");
    };
    complete(
        &mut store,
        WorkshopStoreRequest::SelectContinue {
            slot,
            expected_generation: generation,
        },
    );

    let mut runtime = runtime(store);
    runtime.begin_continue_bootstrap().unwrap();
    finish_continue(&mut runtime);
    assert!(runtime.continue_available());

    runtime.open_library().unwrap();
    settle_library(&mut runtime);
    runtime.archive_library_slot(slot).unwrap();
    settle_library(&mut runtime);

    assert!(slot_archived(&runtime, slot));
    assert!(
        !runtime.continue_available(),
        "an archived slot must not remain openable through a retained candidate"
    );
    // Asserted from the main menu, where the route is otherwise available:
    // from the Library screen it would be refused as `RouteUnavailable`
    // whatever the candidate held, which witnesses nothing.
    runtime.close_library();
    runtime.return_to_main_menu().unwrap();
    assert_eq!(
        runtime.select_menu_route(MainMenuRoute::Continue),
        Err(ClientRuntimeError::ContinueUnavailable)
    );
}

#[test]
fn a_library_store_failure_offers_retry_and_never_enters_recovery() {
    let (inner, first, _) = two_slot_store();
    let mut runtime = ClientRuntime::new(classic(), FaultySlotStore::new(inner, SlotFault::List));

    runtime.open_library().unwrap();
    runtime.update(Duration::ZERO);

    assert_eq!(
        runtime.library_slots_status(),
        LibrarySlotsStatus::Failed {
            kind: SlotRequestKind::List,
            slot: None,
            code: ClientDiagnosticCode::Store,
        }
    );
    // §6 forbids collapsing a Library failure into the global recovery screen:
    // Retry and Cancel cannot be offered over a replaced screen.
    assert_eq!(runtime.screen(), ClientScreen::Library);
    assert!(runtime.recovery_diagnostic().is_none());
    assert!(runtime.library_slots().is_none());

    runtime.retry_library_slot_request().unwrap();
    settle_library(&mut runtime);
    assert_eq!(runtime.screen(), ClientScreen::Library);
    assert!(runtime.recovery_diagnostic().is_none());
    assert_eq!(slot_named(&runtime, first), "First Forge");
}

#[test]
fn a_failed_mutation_retries_from_its_retained_request() {
    // Retry must not ask the user to retype the slot name, so the request
    // itself is the retry material.
    let (inner, first, _) = two_slot_store();
    let mut runtime = ClientRuntime::new(classic(), FaultySlotStore::new(inner, SlotFault::Rename));
    runtime.open_library().unwrap();
    settle_library(&mut runtime);

    runtime
        .rename_library_slot(first, SlotName::new("Retried Forge").unwrap())
        .unwrap();
    runtime.update(Duration::ZERO);
    assert_eq!(
        runtime.library_slots_status(),
        LibrarySlotsStatus::Failed {
            kind: SlotRequestKind::Rename,
            slot: Some(first),
            code: ClientDiagnosticCode::Store,
        }
    );
    // A machine holding a decision the user has not made refuses a new request
    // rather than silently discarding the retained one.
    assert_eq!(
        runtime.refresh_library_slots(),
        Err(ClientRuntimeError::LibraryRequestActive)
    );

    runtime.retry_library_slot_request().unwrap();
    settle_library(&mut runtime);
    assert_eq!(slot_named(&runtime, first), "Retried Forge");
    assert!(runtime.recovery_diagnostic().is_none());
}

#[test]
fn a_busy_commit_lane_becomes_a_retryable_library_failure() {
    // §6: an unrelated Rename may proceed "only when they do not contend with
    // an occupied store mutation lane". Contention is the store's refusal to
    // report, not this runtime's to pre-empt.
    let (store, first, _) = two_slot_store();
    let mut runtime = runtime(store);
    runtime.start_new_workshop(0xBADC0DE).unwrap();
    let snapshot = runtime.workshop_snapshot().unwrap();
    runtime
        .enqueue_workshop_action(WorkshopAction::Submit(CreatorBatchV1 {
            expected_cursor: snapshot.active_view.view_cursor,
            expected_tick: snapshot.active_view.tick,
            operations: vec![CreatorOpV1::CreateSystem {
                local: BatchLocalId(1),
                name: ObjectName::new("Busy Forge").unwrap(),
                position: GalaxyPointV1::new(128, 128).unwrap(),
            }],
        }))
        .unwrap();
    runtime
        .enqueue_workshop_action(WorkshopAction::RequestSave)
        .unwrap();
    runtime.open_library().unwrap();
    settle_library(&mut runtime);
    runtime.update(Duration::ZERO);
    assert!(
        runtime.workshop_snapshot().unwrap().store.commit_pending,
        "the resident Workshop must hold the Commit lane"
    );

    let rejected = runtime.rename_library_slot(first, SlotName::new("Contended").unwrap());
    assert!(matches!(
        rejected,
        Err(ClientRuntimeError::Store(WorkshopStoreError::Busy { .. }))
    ));
    assert_eq!(
        runtime.library_slots_status(),
        LibrarySlotsStatus::Failed {
            kind: SlotRequestKind::Rename,
            slot: Some(first),
            code: ClientDiagnosticCode::Store,
        }
    );
    assert_eq!(runtime.screen(), ClientScreen::Library);
    assert!(runtime.recovery_diagnostic().is_none());

    for _ in 0..16 {
        if !runtime.workshop_snapshot().unwrap().store.commit_pending {
            break;
        }
        runtime.update(Duration::ZERO);
    }
    runtime.retry_library_slot_request().unwrap();
    settle_library(&mut runtime);
    assert_eq!(slot_named(&runtime, first), "Contended");
}

#[test]
fn cancelling_a_mutation_drops_the_cached_list_and_a_cancelled_list_does_not() {
    // Abandoning drops the outcome, not the work: a mutation that was going to
    // land still lands, so every generation the cached list held is suspect and
    // the store's own contract is to re-list before trusting one. Cancelling a
    // list invalidates nothing, so the cached list survives it.
    let (store, first, _) = two_slot_store();
    let mut runtime = runtime(store);
    runtime.open_library().unwrap();
    settle_library(&mut runtime);
    assert!(runtime.library_slots().is_some());

    runtime.refresh_library_slots().unwrap();
    runtime.cancel_library_slot_request().unwrap();
    assert_eq!(runtime.library_slots_status(), LibrarySlotsStatus::Idle);
    assert!(
        runtime.library_slots().is_some(),
        "an abandoned list invalidates no generation"
    );

    runtime
        .rename_library_slot(first, SlotName::new("Abandoned").unwrap())
        .unwrap();
    runtime.cancel_library_slot_request().unwrap();
    assert_eq!(runtime.library_slots_status(), LibrarySlotsStatus::Idle);
    assert!(
        runtime.library_slots().is_none(),
        "an abandoned mutation invalidates every generation the list held"
    );

    // The Commit lane really was released: the very next mutation is accepted
    // rather than refused as Busy.
    runtime
        .unarchive_library_slot(first)
        .expect("the abandoned job released its lane");
    settle_library(&mut runtime);
    assert!(runtime.library_slots().is_some());

    assert_eq!(
        runtime.cancel_library_slot_request(),
        Err(ClientRuntimeError::RouteUnavailable)
    );
}

#[test]
fn a_library_request_still_in_flight_survives_closing_the_library() {
    // A job left unpolled wedges its lane for the store's lifetime, so the poll
    // machine runs from `update` unconditionally rather than only on screen.
    let (store, first, _) = two_slot_store();
    let mut runtime = runtime(store);
    runtime.start_new_workshop(0xC105E).unwrap();
    runtime.open_library().unwrap();
    settle_library(&mut runtime);

    runtime
        .rename_library_slot(first, SlotName::new("Closed Forge").unwrap())
        .unwrap();
    runtime.close_library();
    assert_eq!(runtime.screen(), ClientScreen::GalaxyWorkshop);
    settle_library(&mut runtime);

    assert_eq!(slot_named(&runtime, first), "Closed Forge");
    assert_eq!(runtime.library_slots_status(), LibrarySlotsStatus::Idle);
}

/// A store holding one slot already selected for Continue, so a bootstrap
/// produces a validated candidate naming it.
fn selected_single_slot_store(name: &str, seed: u64) -> (MemoryWorkshopStore, SlotId) {
    let mut store = MemoryWorkshopStore::default();
    let created = complete(
        &mut store,
        WorkshopStoreRequest::CreateSlot {
            name: SlotName::new(name).unwrap(),
            archive: valid_archive(seed),
        },
    );
    let WorkshopStoreResult::SlotCreated { slot, generation } = created else {
        panic!("unexpected create result: {created:?}");
    };
    complete(
        &mut store,
        WorkshopStoreRequest::SelectContinue {
            slot,
            expected_generation: generation,
        },
    );
    (store, slot)
}

#[test]
fn cancelling_an_in_flight_archive_still_withdraws_the_continue_candidate() {
    // `abandon` drops the outcome, not the work, so a cancelled Archive still
    // lands. A candidate withdrawn only on the success edge therefore survives
    // exactly the path that most needs it withdrawn, and §10's "no archived
    // slot is opened or selected for Continue" is violated by an ordinary
    // Cancel.
    let (store, slot) = selected_single_slot_store("Cancelled Archive", 61);
    let mut runtime = runtime(store);
    runtime.begin_continue_bootstrap().unwrap();
    finish_continue(&mut runtime);
    assert!(runtime.continue_available());

    runtime.open_library().unwrap();
    settle_library(&mut runtime);
    runtime.archive_library_slot(slot).unwrap();
    runtime.cancel_library_slot_request().unwrap();

    // The control: the archive really did land despite the cancel. Without
    // this the test could pass on a store that never performed the write.
    runtime.refresh_library_slots().unwrap();
    settle_library(&mut runtime);
    assert!(
        slot_archived(&runtime, slot),
        "the abandoned Archive still reached the store"
    );

    assert!(!runtime.continue_available());
    runtime.close_library();
    assert_eq!(runtime.screen(), ClientScreen::MainMenu);
    assert_eq!(
        runtime.select_menu_route(MainMenuRoute::Continue),
        Err(ClientRuntimeError::ContinueUnavailable)
    );
    assert!(matches!(runtime.active_session(), ActiveSession::None));
}

#[test]
fn a_retried_archive_is_refused_once_its_slot_has_become_resident() {
    // The §3 refusal cannot be attached to the first attempt alone: residency
    // changes between a failure and its Retry by an entirely ordinary path.
    let (inner, slot) = selected_single_slot_store("Retried Archive", 62);
    let mut runtime =
        ClientRuntime::new(classic(), FaultySlotStore::new(inner, SlotFault::Archive));
    runtime.open_library().unwrap();
    settle_library(&mut runtime);

    // Fails while the slot is not resident, so the first attempt is legal.
    runtime.archive_library_slot(slot).unwrap();
    runtime.update(Duration::ZERO);
    assert_eq!(
        runtime.library_slots_status(),
        LibrarySlotsStatus::Failed {
            kind: SlotRequestKind::Archive,
            slot: Some(slot),
            code: ClientDiagnosticCode::Store,
        }
    );

    // The user leaves, opens that very slot through Continue, and comes back.
    runtime.close_library();
    runtime.begin_continue_bootstrap().unwrap();
    finish_continue(&mut runtime);
    runtime.select_menu_route(MainMenuRoute::Continue).unwrap();
    assert_eq!(runtime.workshop_snapshot().unwrap().store.slot, Some(slot));
    runtime.open_library().unwrap();

    assert_eq!(
        runtime.retry_library_slot_request(),
        Err(ClientRuntimeError::ResidentSlot)
    );
    // A refusal that starts nothing must not swallow the pending decision
    // either: Retry and Cancel are still on offer over the same request.
    assert_eq!(
        runtime.library_slots_status(),
        LibrarySlotsStatus::Failed {
            kind: SlotRequestKind::Archive,
            slot: Some(slot),
            code: ClientDiagnosticCode::Store,
        }
    );
    runtime.refresh_library_slots().unwrap_err();
    runtime.cancel_library_slot_request().unwrap();
    runtime.refresh_library_slots().unwrap();
    settle_library(&mut runtime);
    assert!(
        !slot_archived(&runtime, slot),
        "the resident Workshop's own slot must be left un-archived"
    );
}

#[test]
fn a_retained_failure_survives_closing_and_reopening_the_library() {
    // `open_library` auto-lists only from `Idle`, so a decision the user has
    // not made yet is preserved across a visit rather than being replaced by a
    // fresh list. Finding 2's exploit path depends on this, and task 7 has to
    // render it as the pending decision it is.
    let (inner, first, _) = two_slot_store();
    let mut runtime = ClientRuntime::new(classic(), FaultySlotStore::new(inner, SlotFault::Rename));
    runtime.open_library().unwrap();
    settle_library(&mut runtime);

    runtime
        .rename_library_slot(first, SlotName::new("Survives").unwrap())
        .unwrap();
    runtime.update(Duration::ZERO);
    let failed = LibrarySlotsStatus::Failed {
        kind: SlotRequestKind::Rename,
        slot: Some(first),
        code: ClientDiagnosticCode::Store,
    };
    assert_eq!(runtime.library_slots_status(), failed);

    runtime.close_library();
    assert_eq!(runtime.screen(), ClientScreen::MainMenu);
    runtime.open_library().unwrap();

    assert_eq!(
        runtime.library_slots_status(),
        failed,
        "re-entry must not dispatch a list over a retained decision"
    );
    runtime.retry_library_slot_request().unwrap();
    settle_library(&mut runtime);
    assert_eq!(slot_named(&runtime, first), "Survives");
}

/// Answers the first Rename with a slot the user never activated, then refuses
/// to start the Retry outright.
///
/// The two failures land under *different* diagnostic codes — `StoreProtocol`
/// then `Store` — which is what makes the retained-code question observable at
/// all. Every other double in this file fails under `Store` both times, so a
/// stale code overwriting a fresh one would look identical to the correct
/// behaviour.
struct ProtocolThenRejectStore {
    inner: MemoryWorkshopStore,
    answered_wrong: bool,
    rejected_retry: bool,
    wrong_slot: SlotId,
    pending: Option<(StoreJobId, StoreJobState)>,
}

impl ProtocolThenRejectStore {
    fn new(inner: MemoryWorkshopStore, wrong_slot: SlotId) -> Self {
        Self {
            inner,
            answered_wrong: false,
            rejected_retry: false,
            wrong_slot,
            pending: None,
        }
    }
}

impl WorkshopStore for ProtocolThenRejectStore {
    fn start(&mut self, request: WorkshopStoreRequest) -> Result<StoreJobId, WorkshopStoreError> {
        if matches!(request, WorkshopStoreRequest::RenameSlot { .. }) {
            if !self.answered_wrong {
                self.answered_wrong = true;
                let job = StoreJobId(u64::MAX);
                self.pending = Some((
                    job,
                    StoreJobState::Complete(Ok(WorkshopStoreResult::SlotRenamed {
                        slot: self.wrong_slot,
                    })),
                ));
                return Ok(job);
            }
            if !self.rejected_retry {
                self.rejected_retry = true;
                return Err(WorkshopStoreError::CorruptManifest);
            }
        }
        self.inner.start(request)
    }

    fn abandon(&mut self, job: StoreJobId) -> bool {
        if self
            .pending
            .as_ref()
            .is_some_and(|pending| pending.0 == job)
        {
            self.pending = None;
            return true;
        }
        self.inner.abandon(job)
    }

    fn poll(&mut self, job: StoreJobId) -> StoreJobState {
        if self
            .pending
            .as_ref()
            .is_some_and(|pending| pending.0 == job)
        {
            let (_, state) = self
                .pending
                .take()
                .expect("the pending job was just matched");
            return state;
        }
        self.inner.poll(job)
    }
}

#[test]
fn a_refused_retry_reports_its_own_failure_rather_than_the_one_it_replaced() {
    // `retry_library_slot_request` restores the retained request only when the
    // dispatch left the machine `Idle`. A dispatch that failed by retaining a
    // failure of its *own* is not Idle, so the restore is skipped and the newer
    // code survives. Without that guard the Library would answer "why did this
    // fail?" with the reason the *previous* attempt failed, which is the one
    // thing a Retry button must never do.
    let (inner, first, second) = two_slot_store();
    let mut runtime = ClientRuntime::new(classic(), ProtocolThenRejectStore::new(inner, second));
    runtime.open_library().unwrap();
    settle_library(&mut runtime);

    // Attempt 1 fails as a protocol violation: the store renamed another row.
    runtime
        .rename_library_slot(first, SlotName::new("Recoded").unwrap())
        .unwrap();
    runtime.update(Duration::ZERO);
    assert_eq!(
        runtime.library_slots_status(),
        LibrarySlotsStatus::Failed {
            kind: SlotRequestKind::Rename,
            slot: Some(first),
            code: ClientDiagnosticCode::StoreProtocol,
        }
    );

    // Attempt 2 is refused by the store before a job exists, which is an
    // ordinary storage failure and retains itself under `Store`.
    assert!(matches!(
        runtime.retry_library_slot_request(),
        Err(ClientRuntimeError::Store(_))
    ));
    assert_eq!(
        runtime.library_slots_status(),
        LibrarySlotsStatus::Failed {
            kind: SlotRequestKind::Rename,
            slot: Some(first),
            code: ClientDiagnosticCode::Store,
        },
        "the retained StoreProtocol code must not overwrite this attempt's own"
    );

    // Replacing the code must not have cost the pending decision: the same
    // request is still retryable, and Cancel still clears it.
    runtime.retry_library_slot_request().unwrap();
    settle_library(&mut runtime);
    assert_eq!(slot_named(&runtime, first), "Recoded");
}
