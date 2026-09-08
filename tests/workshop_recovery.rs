#![cfg(not(target_arch = "wasm32"))]

use std::{
    cell::{Cell, RefCell},
    path::Path,
    rc::Rc,
    time::{Duration, Instant},
};

use nyon::{
    app::{
        AppCore,
        client_runtime::{ClientRuntime, ClientRuntimeError, ClientScreen, MainMenuRoute},
    },
    preferences::store::MemoryPreferencesStore,
    scenario::{ScenarioDraft, store::MemoryScenarioStore},
    workshop::{
        WorkshopHistory, decode_catalog_pack, encode_archive,
        store::{
            LoadedSlot, MemoryWorkshopStore, NativeWorkshopStore, SaveGeneration, SlotId, SlotName,
            StoreJobId, StoreJobState, WorkshopStore, WorkshopStoreError, WorkshopStoreRequest,
            WorkshopStoreResult, web::IndexedDbTransactionModel,
        },
    },
};

fn classic() -> AppCore<MemoryScenarioStore, MemoryPreferencesStore> {
    AppCore::new_with_preferences(
        ScenarioDraft::factory_default().validated().unwrap(),
        MemoryScenarioStore::default(),
        MemoryPreferencesStore::default(),
    )
}

fn valid_archive(seed: u64) -> Box<[u8]> {
    let catalog =
        decode_catalog_pack(include_bytes!("../assets/workshop/core-pack-v1.json")).unwrap();
    encode_archive(&WorkshopHistory::from_seed_u64(catalog, seed))
        .unwrap()
        .bytes
        .into_boxed_slice()
}

fn wait_for_job(store: &mut dyn WorkshopStore, job: StoreJobId) -> StoreJobState {
    let deadline = Instant::now() + Duration::from_secs(10);
    loop {
        match store.poll(job) {
            state @ StoreJobState::Complete(_) => return state,
            StoreJobState::Pending if Instant::now() < deadline => std::thread::yield_now(),
            state => return state,
        }
    }
}

fn complete(store: &mut dyn WorkshopStore, request: WorkshopStoreRequest) -> WorkshopStoreResult {
    let job = store.start(request).unwrap();
    match wait_for_job(store, job) {
        StoreJobState::Complete(Ok(result)) => result,
        state => panic!("unexpected Workshop store result: {state:?}"),
    }
}

fn create_selected(store: &mut dyn WorkshopStore, archive: Box<[u8]>) -> (SlotId, SaveGeneration) {
    let WorkshopStoreResult::SlotCreated { slot, generation } = complete(
        store,
        WorkshopStoreRequest::CreateSlot {
            name: SlotName::new("Recovery Forge").unwrap(),
            archive,
        },
    ) else {
        panic!("create returned the wrong result");
    };
    complete(
        store,
        WorkshopStoreRequest::SelectContinue {
            slot,
            expected_generation: generation,
        },
    );
    (slot, generation)
}

/// Commits, then re-selects the committed head for Continue.
///
/// This is the sequence the addendum mandates for every save: commit, validate
/// the returned generation, then issue a generation-checked `SelectContinue`.
/// A commit clears any Continue marker naming the slot, because the new head
/// invalidates a marker chosen against the old one, so a fixture that stopped
/// at the commit would build a store state the product can no longer produce
/// and these recovery journeys would start with no Continue candidate at all.
fn commit(
    store: &mut dyn WorkshopStore,
    slot: SlotId,
    generation: SaveGeneration,
    archive: Box<[u8]>,
) -> SaveGeneration {
    let WorkshopStoreResult::SlotCommitted { generation, .. } = complete(
        store,
        WorkshopStoreRequest::CommitSlot {
            slot,
            expected_generation: generation,
            archive,
        },
    ) else {
        panic!("commit returned the wrong result");
    };
    complete(
        store,
        WorkshopStoreRequest::SelectContinue {
            slot,
            expected_generation: generation,
        },
    );
    generation
}

fn load(store: &mut dyn WorkshopStore, slot: SlotId) -> LoadedSlot {
    let WorkshopStoreResult::SlotLoaded(loaded) =
        complete(store, WorkshopStoreRequest::LoadSlot { slot })
    else {
        panic!("load returned the wrong result");
    };
    loaded
}

fn load_previous(store: &mut dyn WorkshopStore, slot: SlotId, head: SaveGeneration) -> LoadedSlot {
    let WorkshopStoreResult::SlotLoaded(loaded) = complete(
        store,
        WorkshopStoreRequest::LoadPreviousGeneration {
            slot,
            expected_head_generation: head,
        },
    ) else {
        panic!("previous-generation load returned the wrong result");
    };
    loaded
}

fn finish_bootstrap<W: WorkshopStore>(
    runtime: &mut ClientRuntime<MemoryScenarioStore, MemoryPreferencesStore, W>,
) {
    let deadline = Instant::now() + Duration::from_secs(10);
    while Instant::now() < deadline {
        if !runtime.continue_bootstrap_active() {
            return;
        }
        runtime.update(Duration::ZERO);
        std::thread::yield_now();
    }
    panic!("Continue bootstrap exceeded its bounded test polls");
}

fn finish_recovery_promotion<W: WorkshopStore>(
    runtime: &mut ClientRuntime<MemoryScenarioStore, MemoryPreferencesStore, W>,
) {
    let deadline = Instant::now() + Duration::from_secs(10);
    while Instant::now() < deadline {
        runtime.update(Duration::ZERO);
        if runtime.continue_available() {
            return;
        }
        std::thread::yield_now();
    }
    panic!("recovery promotion did not become Continue-ready");
}

struct SharedStore<T>(Rc<RefCell<T>>);

impl<T> Clone for SharedStore<T> {
    fn clone(&self) -> Self {
        Self(Rc::clone(&self.0))
    }
}

impl<T> SharedStore<T> {
    fn new(store: T) -> Self {
        Self(Rc::new(RefCell::new(store)))
    }
}

impl<T: WorkshopStore> WorkshopStore for SharedStore<T> {
    fn start(&mut self, request: WorkshopStoreRequest) -> Result<StoreJobId, WorkshopStoreError> {
        self.0.borrow_mut().start(request)
    }

    fn poll(&mut self, job: StoreJobId) -> StoreJobState {
        self.0.borrow_mut().poll(job)
    }
}

#[derive(Default)]
struct PromotionFailureProbe {
    attempts: Cell<usize>,
    failed_once: Cell<bool>,
    load_starts: Cell<usize>,
}

struct FailPromotionOnce<T> {
    inner: T,
    probe: Rc<PromotionFailureProbe>,
}

type SharedMemoryStore = SharedStore<MemoryWorkshopStore>;
type FailingRecoveryRuntime = ClientRuntime<
    MemoryScenarioStore,
    MemoryPreferencesStore,
    FailPromotionOnce<SharedMemoryStore>,
>;

impl<T: WorkshopStore> WorkshopStore for FailPromotionOnce<T> {
    fn start(&mut self, request: WorkshopStoreRequest) -> Result<StoreJobId, WorkshopStoreError> {
        if matches!(request, WorkshopStoreRequest::LoadSlot { .. }) {
            self.probe.load_starts.set(self.probe.load_starts.get() + 1);
        }
        if matches!(request, WorkshopStoreRequest::PromoteRecoveredSlot { .. }) {
            self.probe.attempts.set(self.probe.attempts.get() + 1);
            if !self.probe.failed_once.replace(true) {
                return Err(WorkshopStoreError::QuotaExceeded { max_bytes: 0 });
            }
        }
        self.inner.start(request)
    }

    fn poll(&mut self, job: StoreJobId) -> StoreJobState {
        self.inner.poll(job)
    }
}

fn failed_promotion_runtime(
    seed: u64,
) -> (
    FailingRecoveryRuntime,
    SharedMemoryStore,
    Rc<PromotionFailureProbe>,
) {
    let shared = SharedStore::new(MemoryWorkshopStore::default());
    let mut setup = shared.clone();
    let (slot, first) = create_selected(&mut setup, valid_archive(seed));
    commit(&mut setup, slot, first, Box::from(&b"{}"[..]));
    let probe = Rc::new(PromotionFailureProbe::default());
    let store = FailPromotionOnce {
        inner: shared.clone(),
        probe: Rc::clone(&probe),
    };
    let mut runtime = ClientRuntime::new(classic(), store);
    runtime.begin_continue_bootstrap().unwrap();
    finish_bootstrap(&mut runtime);
    runtime.select_menu_route(MainMenuRoute::Continue).unwrap();
    runtime.update(Duration::ZERO);
    assert_eq!(probe.attempts.get(), 1);
    (runtime, shared, probe)
}

fn advance_promotion_backoff<W: WorkshopStore>(
    runtime: &mut ClientRuntime<MemoryScenarioStore, MemoryPreferencesStore, W>,
) {
    for _ in 0..20 {
        runtime.update(Duration::from_millis(250));
    }
    finish_recovery_promotion(runtime);
}

fn authority_invalid_head_is_promoted_and_retained<T: WorkshopStore + Default>() {
    let shared = SharedStore::new(T::default());
    let mut setup = shared.clone();
    let (slot, first) = create_selected(&mut setup, valid_archive(0xA57E));
    let invalid_head = commit(&mut setup, slot, first, Box::from(&b"{}"[..]));

    let mut runtime = ClientRuntime::new(classic(), shared.clone());
    runtime.begin_continue_bootstrap().unwrap();
    finish_bootstrap(&mut runtime);
    assert_eq!(runtime.screen(), ClientScreen::RecoverableError);
    assert!(runtime.continue_available());

    runtime.select_menu_route(MainMenuRoute::Continue).unwrap();
    let recovered = runtime.workshop_snapshot().unwrap();
    assert_eq!(recovered.store.generation, Some(first));
    assert_eq!(recovered.store.head_generation, Some(invalid_head));
    assert!(!runtime.continue_available());

    finish_recovery_promotion(&mut runtime);
    let promoted = runtime.workshop_snapshot().unwrap();
    let promoted_generation = promoted.store.generation.unwrap();
    assert_eq!(promoted_generation, SaveGeneration(invalid_head.0 + 1));
    assert_eq!(promoted.store.head_generation, Some(promoted_generation));
    runtime.return_to_main_menu().unwrap();
    assert_eq!(runtime.screen(), ClientScreen::MainMenu);
    assert!(runtime.continue_available());

    let predecessor = load_previous(&mut shared.clone(), slot, promoted_generation);
    assert_eq!(predecessor.generation, first);
    assert_eq!(predecessor.archive, valid_archive(0xA57E));

    drop(runtime);
    let corrupt_new_head = commit(
        &mut shared.clone(),
        slot,
        promoted_generation,
        Box::from(&b"{}"[..]),
    );
    let mut reopened = ClientRuntime::new(classic(), shared);
    reopened.begin_continue_bootstrap().unwrap();
    finish_bootstrap(&mut reopened);
    assert_eq!(reopened.screen(), ClientScreen::RecoverableError);
    reopened.select_menu_route(MainMenuRoute::Continue).unwrap();
    let recovered_again = reopened.workshop_snapshot().unwrap();
    assert_eq!(recovered_again.store.generation, Some(promoted_generation));
    assert_eq!(
        recovered_again.store.head_generation,
        Some(corrupt_new_head)
    );
}

#[test]
fn memory_recovery_promotes_authoritative_bytes_and_retains_the_known_good_predecessor() {
    authority_invalid_head_is_promoted_and_retained::<MemoryWorkshopStore>();
}

#[test]
fn indexeddb_model_recovery_promotes_authoritative_bytes_and_retains_the_known_good_predecessor() {
    authority_invalid_head_is_promoted_and_retained::<IndexedDbTransactionModel>();
}

#[test]
fn failed_recovery_promotion_stays_blocking_and_retries_after_bounded_backoff() {
    let shared = SharedStore::new(MemoryWorkshopStore::default());
    let mut setup = shared.clone();
    let (slot, first) = create_selected(&mut setup, valid_archive(0xFA11));
    let invalid_head = commit(&mut setup, slot, first, Box::from(&b"{}"[..]));
    let probe = Rc::new(PromotionFailureProbe::default());
    let store = FailPromotionOnce {
        inner: shared.clone(),
        probe: Rc::clone(&probe),
    };
    let mut runtime = ClientRuntime::new(classic(), store);
    runtime.begin_continue_bootstrap().unwrap();
    finish_bootstrap(&mut runtime);
    runtime.select_menu_route(MainMenuRoute::Continue).unwrap();

    runtime.update(Duration::ZERO);
    assert_eq!(probe.attempts.get(), 1);
    assert!(!runtime.continue_available());
    assert_eq!(
        runtime.start_new_workshop(0xDEAD),
        Err(ClientRuntimeError::RouteUnavailable)
    );
    assert_eq!(
        runtime.workshop_snapshot().unwrap().store.generation,
        Some(first)
    );
    assert_eq!(
        runtime.workshop_snapshot().unwrap().store.head_generation,
        Some(invalid_head)
    );

    for _ in 0..19 {
        runtime.update(Duration::from_millis(250));
    }
    assert_eq!(probe.attempts.get(), 1);
    runtime.update(Duration::from_millis(250));
    assert_eq!(probe.attempts.get(), 2);
    finish_recovery_promotion(&mut runtime);
    assert!(runtime.continue_available());
    runtime.return_to_main_menu().unwrap();
    drop(runtime);

    let mut reopened = ClientRuntime::new(classic(), shared);
    reopened.begin_continue_bootstrap().unwrap();
    finish_bootstrap(&mut reopened);
    assert_eq!(reopened.screen(), ClientScreen::MainMenu);
    assert!(reopened.recovery_diagnostic().is_none());
    assert!(reopened.continue_available());
}

#[test]
fn failed_recovery_promotion_blocks_session_load_until_the_obligation_completes() {
    let (mut runtime, mut shared, probe) = failed_promotion_runtime(0x10AD);
    let before = runtime.workshop_snapshot().unwrap().state_digest;
    let (target, _) = create_selected(&mut shared, valid_archive(0xBEEF));
    let load_starts = probe.load_starts.get();

    runtime
        .enqueue_workshop_action(nyon::workshop::session::WorkshopAction::RequestLoad(target))
        .unwrap();
    runtime.update(Duration::ZERO);
    assert_eq!(probe.load_starts.get(), load_starts);
    assert_eq!(runtime.workshop_snapshot().unwrap().state_digest, before);
    assert!(!runtime.workshop_snapshot().unwrap().store.load_pending);

    advance_promotion_backoff(&mut runtime);
    assert!(runtime.continue_available());
    runtime
        .enqueue_workshop_action(nyon::workshop::session::WorkshopAction::RequestLoad(target))
        .unwrap();
    runtime.update(Duration::ZERO);
    assert_eq!(probe.load_starts.get(), load_starts + 1);
    assert_eq!(
        runtime.workshop_snapshot().unwrap().state_digest,
        WorkshopHistory::from_seed_u64(
            decode_catalog_pack(include_bytes!("../assets/workshop/core-pack-v1.json")).unwrap(),
            0xBEEF,
        )
        .active_state_digest()
    );
}

#[test]
fn failed_recovery_promotion_blocks_session_import_until_the_obligation_completes() {
    let (mut runtime, _shared, _probe) = failed_promotion_runtime(0x1A90);
    let before = runtime.workshop_snapshot().unwrap().state_digest;
    let imported = valid_archive(0xCAFE);

    runtime
        .enqueue_workshop_action(nyon::workshop::session::WorkshopAction::RequestImport(
            imported.clone(),
        ))
        .unwrap();
    runtime.update(Duration::ZERO);
    assert_eq!(runtime.workshop_snapshot().unwrap().state_digest, before);
    assert!(runtime.workshop_snapshot().unwrap().store.slot.is_some());
    assert!(!runtime.workshop_snapshot().unwrap().store.load_pending);

    advance_promotion_backoff(&mut runtime);
    assert!(runtime.continue_available());
    runtime
        .enqueue_workshop_action(nyon::workshop::session::WorkshopAction::RequestImport(
            imported,
        ))
        .unwrap();
    runtime.update(Duration::ZERO);
    assert_eq!(runtime.workshop_snapshot().unwrap().store.slot, None);
    assert_eq!(
        runtime.workshop_snapshot().unwrap().state_digest,
        WorkshopHistory::from_seed_u64(
            decode_catalog_pack(include_bytes!("../assets/workshop/core-pack-v1.json")).unwrap(),
            0xCAFE,
        )
        .active_state_digest()
    );
}

fn generation_path(root: &Path, slot: SlotId, generation: SaveGeneration) -> std::path::PathBuf {
    root.join("slots")
        .join(format!("slot-{:02}", slot.0))
        .join(format!("generation-{:020}.archive", generation.0))
}

#[test]
fn native_recovery_survives_promotion_exit_reopen_and_a_second_corrupt_head() {
    let directory = tempfile::tempdir().unwrap();
    let mut setup = NativeWorkshopStore::at_root(directory.path()).unwrap();
    let (slot, first) = create_selected(&mut setup, valid_archive(0xA57E));
    let bad_head = commit(&mut setup, slot, first, valid_archive(0xBAD));
    std::fs::write(
        generation_path(directory.path(), slot, bad_head),
        b"corrupt",
    )
    .unwrap();
    drop(setup);

    let mut runtime = ClientRuntime::new(
        classic(),
        NativeWorkshopStore::at_root(directory.path()).unwrap(),
    );
    runtime.begin_continue_bootstrap().unwrap();
    finish_bootstrap(&mut runtime);
    assert_eq!(runtime.screen(), ClientScreen::RecoverableError);
    runtime.select_menu_route(MainMenuRoute::Continue).unwrap();
    let recovered = runtime.workshop_snapshot().unwrap();
    assert_eq!(recovered.store.generation, Some(first));
    assert_eq!(recovered.store.head_generation, Some(bad_head));
    finish_recovery_promotion(&mut runtime);
    let promoted = runtime
        .workshop_snapshot()
        .unwrap()
        .store
        .generation
        .unwrap();
    runtime.return_to_main_menu().unwrap();
    assert_eq!(runtime.screen(), ClientScreen::MainMenu);
    assert!(runtime.continue_available());
    drop(runtime);

    let mut reopened = NativeWorkshopStore::at_root(directory.path()).unwrap();
    let clean = load(&mut reopened, slot);
    assert_eq!(clean.generation, promoted);
    assert_eq!(clean.head_generation, promoted);
    let predecessor = load_previous(&mut reopened, slot, promoted);
    assert_eq!(predecessor.generation, first);
    drop(reopened);

    std::fs::write(
        generation_path(directory.path(), slot, promoted),
        b"corrupt again",
    )
    .unwrap();
    let mut recovered_store = NativeWorkshopStore::at_root(directory.path()).unwrap();
    let fallback = load(&mut recovered_store, slot);
    assert_eq!(fallback.generation, first);
    assert_eq!(fallback.head_generation, promoted);
    assert!(fallback.recovered_from_previous);
}

#[test]
fn native_authority_invalid_head_is_promoted_with_the_exact_recovered_predecessor() {
    let directory = tempfile::tempdir().unwrap();
    let mut setup = NativeWorkshopStore::at_root(directory.path()).unwrap();
    let (slot, first) = create_selected(&mut setup, valid_archive(0xA11D));
    let invalid_head = commit(&mut setup, slot, first, Box::from(&b"{}"[..]));
    drop(setup);

    let mut runtime = ClientRuntime::new(
        classic(),
        NativeWorkshopStore::at_root(directory.path()).unwrap(),
    );
    runtime.begin_continue_bootstrap().unwrap();
    finish_bootstrap(&mut runtime);
    assert_eq!(runtime.screen(), ClientScreen::RecoverableError);
    runtime.select_menu_route(MainMenuRoute::Continue).unwrap();
    assert_eq!(
        runtime.workshop_snapshot().unwrap().store.generation,
        Some(first)
    );
    assert_eq!(
        runtime.workshop_snapshot().unwrap().store.head_generation,
        Some(invalid_head)
    );
    finish_recovery_promotion(&mut runtime);
    let promoted = runtime
        .workshop_snapshot()
        .unwrap()
        .store
        .generation
        .unwrap();
    runtime.return_to_main_menu().unwrap();
    drop(runtime);

    let mut reopened = NativeWorkshopStore::at_root(directory.path()).unwrap();
    let clean = load(&mut reopened, slot);
    assert_eq!(clean.generation, promoted);
    assert_eq!(clean.archive, valid_archive(0xA11D));
    let predecessor = load_previous(&mut reopened, slot, promoted);
    assert_eq!(predecessor.generation, first);
    assert_eq!(predecessor.archive, valid_archive(0xA11D));
    drop(reopened);

    std::fs::write(
        generation_path(directory.path(), slot, promoted),
        b"corrupt",
    )
    .unwrap();
    let mut fallback_store = NativeWorkshopStore::at_root(directory.path()).unwrap();
    let fallback = load(&mut fallback_store, slot);
    assert_eq!(fallback.generation, first);
    assert_eq!(fallback.archive, valid_archive(0xA11D));
    assert!(fallback.recovered_from_previous);
}
