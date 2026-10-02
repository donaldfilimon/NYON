#![cfg(not(target_arch = "wasm32"))]
//! Cross-authority qualification: durable neutral selection is independent of
//! either authority's local marker and numeric slot IDs can overlap.
use nyon::{
    app::{
        AppCore,
        client_runtime::{ClientRuntime, LibrarySlotsStatus},
    },
    library_selection::{
        SelectedAuthority, SelectionTarget,
        native::{NativeLibraryCoordinator, NativeLibrarySelection},
    },
    living_store::{native::NativeLivingStoreV2, *},
    preferences::store::MemoryPreferencesStore,
    scenario::{ScenarioDraft, store::MemoryScenarioStore},
    workshop::{WorkshopHistory, decode_catalog_pack, encode_archive, store::*},
};
use std::time::{Duration, Instant};
type Runtime = ClientRuntime<MemoryScenarioStore, MemoryPreferencesStore, MemoryWorkshopStore>;
fn workshop_request(
    store: &mut MemoryWorkshopStore,
    request: WorkshopStoreRequest,
) -> WorkshopStoreResult {
    let job = store.start(request).unwrap();
    match store.poll(job) {
        StoreJobState::Complete(Ok(result)) => result,
        other => panic!("{other:?}"),
    }
}
fn selected_workshop() -> (MemoryWorkshopStore, SlotId) {
    let catalog =
        decode_catalog_pack(include_bytes!("../assets/workshop/core-pack-v1.json")).unwrap();
    let archive = encode_archive(&WorkshopHistory::from_seed_u64(catalog, 11))
        .unwrap()
        .bytes
        .into_boxed_slice();
    let mut store = MemoryWorkshopStore::default();
    let WorkshopStoreResult::SlotCreated { slot, generation } = workshop_request(
        &mut store,
        WorkshopStoreRequest::CreateSlot {
            name: SlotName::new("Selected Workshop").unwrap(),
            archive,
        },
    ) else {
        panic!()
    };
    workshop_request(
        &mut store,
        WorkshopStoreRequest::SelectContinue {
            slot,
            expected_generation: generation,
        },
    );
    (store, slot)
}
fn runtime(root: &std::path::Path, store: MemoryWorkshopStore) -> Runtime {
    let classic = AppCore::new_with_preferences(
        ScenarioDraft::factory_default().validated().unwrap(),
        MemoryScenarioStore::default(),
        MemoryPreferencesStore::default(),
    );
    let mut runtime = ClientRuntime::new(classic, store);
    runtime.install_living_storage(
        Box::new(NativeLivingStoreV2::new(root)),
        Box::new(NativeLibraryCoordinator::new(root)),
    );
    runtime
}
fn settle(runtime: &mut Runtime, done: impl Fn(&Runtime) -> bool) {
    let deadline = Instant::now() + Duration::from_secs(20);
    loop {
        runtime.update(Duration::ZERO);
        if done(runtime) {
            return;
        }
        assert!(
            Instant::now() < deadline,
            "runtime did not reach qualification boundary"
        );
        std::thread::sleep(Duration::from_millis(1));
    }
}
#[test]
fn missing_neutral_record_migrates_exact_workshop_authority_then_archiving_selected_slot_clears_it()
{
    let root = tempfile::tempdir().unwrap();
    let (store, slot) = selected_workshop();
    let mut runtime = runtime(root.path(), store);
    runtime.begin_continue_bootstrap().unwrap();
    settle(&mut runtime, |runtime| !runtime.continue_bootstrap_active());
    let coordinator = NativeLibrarySelection::new(root.path());
    let selected = coordinator.read().unwrap().unwrap_or_else(|| {
        panic!(
            "neutral migration absent: screen {:?}, diagnostics {:?}, bootstrap {}",
            runtime.screen(),
            runtime.diagnostics().collect::<Vec<_>>(),
            runtime.continue_bootstrap_active()
        )
    });
    let target = selected.target().unwrap().unwrap();
    assert_eq!(target.authority, SelectedAuthority::WorkshopV1);
    assert_eq!(target.slot, slot.0);
    assert!(runtime.continue_available());
    runtime.archive_library_slot(slot).unwrap();
    settle(&mut runtime, |runtime| {
        runtime.library_slots_status() == LibrarySlotsStatus::Idle
    });
    // Neutral Continue must not keep naming a Workshop archive whose V1 marker
    // has already been withdrawn by this successful durable mutation.
    let deadline = Instant::now() + Duration::from_secs(2);
    loop {
        runtime.update(Duration::ZERO);
        if coordinator
            .read()
            .unwrap()
            .unwrap()
            .target()
            .unwrap()
            .is_none()
        {
            break;
        }
        assert!(
            Instant::now() < deadline,
            "archiving selected Workshop left a stale neutral Continue target"
        );
        std::thread::sleep(Duration::from_millis(1));
    }
}
fn living_request(
    store: &mut NativeLivingStoreV2,
    request: LivingStoreRequestV2,
) -> LivingStoreResultV2 {
    let job = store.begin(1, request).unwrap();
    let deadline = Instant::now() + Duration::from_secs(20);
    loop {
        match store.poll(1, job, 1024) {
            LivingStorePollV2::Ready(result) => {
                assert!(store.release(1, job));
                return result;
            }
            LivingStorePollV2::Failed(error) => panic!("{error}"),
            LivingStorePollV2::Pending { .. } => {
                assert!(Instant::now() < deadline);
                std::thread::yield_now();
            }
        }
    }
}
#[test]
fn deleting_living_slot_with_same_numeric_id_preserves_workshop_neutral_selection() {
    use nyon::app::client_runtime::living::{LivingClient, LivingClientEvent};
    use nyon_workshop_core::living::{
        LIVING_CORE_PACK_BYTES_V2, builtin_first_expansion_v2, encode_living_archive_v2,
    };
    let root = tempfile::tempdir().unwrap();
    let target = SelectionTarget {
        authority: SelectedAuthority::WorkshopV1,
        slot: 1,
        branch: "11".repeat(16),
        revision: None,
        tick: 0,
        digest: "22".repeat(32),
    };
    let coordinator = NativeLibrarySelection::new(root.path());
    coordinator
        .compare_and_swap(None, Some(target.clone()))
        .unwrap();
    let mut store = NativeLivingStoreV2::new(root.path());
    living_request(
        &mut store,
        LivingStoreRequestV2::PutPack {
            canonical_bytes: LIVING_CORE_PACK_BYTES_V2.to_vec(),
        },
    );
    let LivingStoreResultV2::Slot(slot) = living_request(
        &mut store,
        LivingStoreRequestV2::CreateSlot {
            name: "Separate Living slot".into(),
            archive: encode_living_archive_v2(&builtin_first_expansion_v2().unwrap()).unwrap(),
        },
    ) else {
        panic!()
    };
    assert_eq!(slot.id.0, target.slot);
    let LivingStoreResultV2::Slot(archived) = living_request(
        &mut store,
        LivingStoreRequestV2::SetArchived {
            id: slot.id,
            expected_generation: slot.generation,
            archived: true,
        },
    ) else {
        panic!()
    };
    let mut client = LivingClient::new(
        Box::new(store),
        Box::new(NativeLibraryCoordinator::new(root.path())),
    );
    fn terminal(client: &mut LivingClient) -> LivingClientEvent {
        let deadline = Instant::now() + Duration::from_secs(20);
        loop {
            let event = client.poll();
            if !matches!(event, LivingClientEvent::Pending) {
                return event;
            }
            assert!(Instant::now() < deadline);
            std::thread::sleep(Duration::from_millis(1));
        }
    }
    client.read_selection().unwrap();
    assert!(matches!(
        terminal(&mut client),
        LivingClientEvent::WorkshopSelection(_)
    ));
    client
        .mutate(LivingStoreRequestV2::DeleteArchivedSlot {
            id: archived.id,
            expected_generation: archived.generation,
        })
        .unwrap();
    assert!(matches!(terminal(&mut client), LivingClientEvent::Mutated));
    assert_eq!(
        coordinator.read().unwrap().unwrap().target().unwrap(),
        Some(target)
    );
}

#[test]
fn explicit_neutral_none_wins_over_legacy_workshop_selected_marker() {
    let root = tempfile::tempdir().unwrap();
    let coordinator = NativeLibrarySelection::new(root.path());
    let record = coordinator.compare_and_swap(None, None).unwrap();
    let (store, _) = selected_workshop();
    let mut runtime = runtime(root.path(), store);
    runtime.begin_continue_bootstrap().unwrap();
    settle(&mut runtime, |runtime| !runtime.continue_bootstrap_active());
    assert!(!runtime.continue_available());
    assert_eq!(coordinator.read().unwrap(), Some(record));
    assert!(runtime.recovery_diagnostic().is_none());
}

#[test]
fn missing_recorded_living_slot_enters_recovery_without_legacy_workshop_fallback() {
    use nyon::app::client_runtime::ClientScreen;
    let root = tempfile::tempdir().unwrap();
    let coordinator = NativeLibrarySelection::new(root.path());
    let record = coordinator
        .compare_and_swap(
            None,
            Some(SelectionTarget {
                authority: SelectedAuthority::LivingV2,
                slot: 999,
                branch: "11".repeat(16),
                revision: None,
                tick: 0,
                digest: "22".repeat(32),
            }),
        )
        .unwrap();
    let (store, _) = selected_workshop();
    let mut runtime = runtime(root.path(), store);
    runtime.begin_continue_bootstrap().unwrap();
    settle(&mut runtime, |runtime| !runtime.continue_bootstrap_active());
    assert_eq!(runtime.screen(), ClientScreen::RecoverableError);
    assert!(runtime.recovery_diagnostic().is_some());
    assert_eq!(coordinator.read().unwrap(), Some(record));
    assert!(runtime.workshop_snapshot().is_none());
}

#[test]
fn placement_won_cancel_keeps_time_held_until_terminal_and_ignores_late_import() {
    use nyon::{
        app::client_runtime::MainMenuRoute, living_transfer::*, sandbox_session::SandboxUiAction,
    };
    use std::{
        cell::{Cell, RefCell},
        rc::Rc,
    };
    struct PendingPlacement {
        active: bool,
        cancelled: Rc<Cell<bool>>,
        outcome: Rc<RefCell<Option<LivingTransferOutcome>>>,
    }
    impl LivingTransfer for PendingPlacement {
        fn begin(&mut self, request: LivingTransferRequest) -> Result<(), LivingTransferError> {
            request.validate()?;
            self.active = true;
            Ok(())
        }
        fn poll(&mut self) -> LivingTransferStatus {
            if let Some(outcome) = self.outcome.borrow_mut().take() {
                self.active = false;
                LivingTransferStatus::Ready(Ok(outcome))
            } else if self.active {
                LivingTransferStatus::Pending
            } else {
                LivingTransferStatus::Idle
            }
        }
        fn cancel(&mut self) {
            self.cancelled.set(true);
        }
    }
    let root = tempfile::tempdir().unwrap();
    let mut runtime = runtime(root.path(), MemoryWorkshopStore::default());
    runtime
        .select_menu_route(MainMenuRoute::GalaxySandbox)
        .unwrap();
    let cancelled = Rc::new(Cell::new(false));
    let outcome = Rc::new(RefCell::new(None));
    runtime.install_living_transfer(Box::new(PendingPlacement {
        active: false,
        cancelled: cancelled.clone(),
        outcome: outcome.clone(),
    }));
    runtime.apply_sandbox_action(SandboxUiAction::PrepareExport);
    settle(&mut runtime, |runtime| {
        runtime
            .living_storage_status()
            .contains("File set prepared")
    });
    let tick = runtime.sandbox_tick().unwrap();
    runtime.apply_sandbox_action(SandboxUiAction::ExportGalaxy);
    runtime.apply_sandbox_action(SandboxUiAction::CancelImport);
    assert!(cancelled.get());
    runtime.update(Duration::ZERO);
    runtime.apply_sandbox_action(SandboxUiAction::Step);
    for _ in 0..100 {
        runtime.update(Duration::ZERO);
    }
    assert_eq!(
        runtime.sandbox_tick(),
        Some(tick),
        "unknown durable placement must retain modal time hold"
    );
    *outcome.borrow_mut() = Some(LivingTransferOutcome::Exported(
        LivingExportOutcome::DurablySaved,
    ));
    runtime.update(Duration::ZERO);
    runtime.apply_sandbox_action(SandboxUiAction::Step);
    for _ in 0..100 {
        runtime.update(Duration::ZERO);
    }
    assert_eq!(
        runtime.sandbox_tick(),
        Some(tick + 1),
        "terminal placement must release modal time hold"
    );
    runtime.apply_sandbox_action(SandboxUiAction::Save);
    settle(&mut runtime, |runtime| !runtime.living_dirty());
    runtime.apply_sandbox_action(SandboxUiAction::ImportNotebook);
    runtime.apply_sandbox_action(SandboxUiAction::CancelImport);
    runtime.update(Duration::ZERO);
    runtime.apply_sandbox_action(SandboxUiAction::Step);
    for _ in 0..100 {
        runtime.update(Duration::ZERO);
    }
    assert_eq!(runtime.sandbox_tick(), Some(tick + 1));
    // A file read can complete after cancellation; its bytes must never start
    // a private validation job or mutate the preserved resident notebook.
    *outcome.borrow_mut() = Some(LivingTransferOutcome::Imported {
        kind: LivingTransferKind::Notebook,
        bytes: b"invalid late notebook".to_vec(),
    });
    runtime.update(Duration::ZERO);
    runtime.apply_sandbox_action(SandboxUiAction::Step);
    for _ in 0..100 {
        runtime.update(Duration::ZERO);
    }
    assert_eq!(runtime.sandbox_tick(), Some(tick + 2));
}

#[test]
fn cancellation_that_leaves_platform_idle_releases_time_hold() {
    use nyon::{
        app::client_runtime::MainMenuRoute, living_transfer::*, sandbox_session::SandboxUiAction,
    };
    struct CancelWins {
        active: bool,
    }
    impl LivingTransfer for CancelWins {
        fn begin(&mut self, request: LivingTransferRequest) -> Result<(), LivingTransferError> {
            request.validate()?;
            self.active = true;
            Ok(())
        }
        fn poll(&mut self) -> LivingTransferStatus {
            if self.active {
                LivingTransferStatus::Pending
            } else {
                LivingTransferStatus::Idle
            }
        }
        fn cancel(&mut self) {
            self.active = false;
        }
    }
    let root = tempfile::tempdir().unwrap();
    let mut runtime = runtime(root.path(), MemoryWorkshopStore::default());
    runtime
        .select_menu_route(MainMenuRoute::GalaxySandbox)
        .unwrap();
    runtime.install_living_transfer(Box::new(CancelWins { active: false }));
    runtime.apply_sandbox_action(SandboxUiAction::PrepareExport);
    settle(&mut runtime, |runtime| {
        runtime
            .living_storage_status()
            .contains("File set prepared")
    });
    let tick = runtime.sandbox_tick().unwrap();
    runtime.apply_sandbox_action(SandboxUiAction::ExportGalaxy);
    runtime.apply_sandbox_action(SandboxUiAction::CancelImport);
    runtime.update(Duration::ZERO);
    runtime.apply_sandbox_action(SandboxUiAction::Step);
    for _ in 0..100 {
        runtime.update(Duration::ZERO);
    }
    assert_eq!(runtime.sandbox_tick(), Some(tick + 1));
}

#[test]
fn cancelled_workshop_archive_reconciles_neutral_selection_after_confirmed_relist() {
    let root = tempfile::tempdir().unwrap();
    let (store, slot) = selected_workshop();
    let mut runtime = runtime(root.path(), store);
    runtime.begin_continue_bootstrap().unwrap();
    settle(&mut runtime, |runtime| !runtime.continue_bootstrap_active());
    let coordinator = NativeLibrarySelection::new(root.path());
    assert_eq!(
        coordinator
            .read()
            .unwrap()
            .unwrap()
            .target()
            .unwrap()
            .unwrap()
            .slot,
        slot.0
    );
    runtime.open_library().unwrap();
    settle(&mut runtime, |runtime| {
        runtime.library_slots_status() == LibrarySlotsStatus::Idle
    });
    // Workshop abandon forgets the result, not the mutation. Its confirmed
    // re-list must reconcile this exact neutral selection after archive lands.
    runtime.archive_library_slot(slot).unwrap();
    runtime.cancel_library_slot_request().unwrap();
    runtime.refresh_library_slots().unwrap();
    settle(&mut runtime, |runtime| {
        runtime.library_slots_status() == LibrarySlotsStatus::Idle
    });
    assert!(
        runtime
            .library_slots()
            .unwrap()
            .slots
            .iter()
            .find(|row| row.id == slot)
            .unwrap()
            .archived
    );
    let deadline = Instant::now() + Duration::from_secs(2);
    loop {
        runtime.update(Duration::ZERO);
        if coordinator
            .read()
            .unwrap()
            .unwrap()
            .target()
            .unwrap()
            .is_none()
        {
            break;
        }
        assert!(
            Instant::now() < deadline,
            "confirmed cancelled Workshop archive retained stale neutral selection"
        );
        std::thread::sleep(Duration::from_millis(1));
    }
}

#[test]
fn pack_modal_preserves_unknown_loading_and_cancels_only_pack_operations() {
    use nyon::{app::client_runtime::MainMenuRoute, sandbox_session::SandboxUiAction};
    use std::{cell::Cell, collections::BTreeSet, rc::Rc};
    struct HeldPackStore {
        native: NativeLivingStoreV2,
        pack_jobs: BTreeSet<LivingStoreJobIdV2>,
        block: Rc<Cell<bool>>,
        cancellations: Rc<Cell<usize>>,
    }
    impl LivingStoreV2 for HeldPackStore {
        fn begin(
            &mut self,
            owner: u64,
            request: LivingStoreRequestV2,
        ) -> Result<LivingStoreJobIdV2, LivingStoreErrorV2> {
            let pack = matches!(
                request,
                LivingStoreRequestV2::ListPacks
                    | LivingStoreRequestV2::ListPackReferences { .. }
                    | LivingStoreRequestV2::SetPackArchived { .. }
                    | LivingStoreRequestV2::DeletePack { .. }
            );
            let job = self.native.begin(owner, request)?;
            if pack {
                self.pack_jobs.insert(job);
            }
            Ok(job)
        }
        fn poll(
            &mut self,
            owner: u64,
            job: LivingStoreJobIdV2,
            budget: usize,
        ) -> LivingStorePollV2 {
            if self.block.get() && self.pack_jobs.contains(&job) {
                LivingStorePollV2::Pending { work_done: 0 }
            } else {
                self.native.poll(owner, job, budget)
            }
        }
        fn cancel(
            &mut self,
            owner: u64,
            job: LivingStoreJobIdV2,
        ) -> Result<LivingStoreCancelV2, LivingStoreErrorV2> {
            self.cancellations.set(self.cancellations.get() + 1);
            self.native.cancel(owner, job)
        }
        fn release(&mut self, owner: u64, job: LivingStoreJobIdV2) -> bool {
            self.pack_jobs.remove(&job);
            self.native.release(owner, job)
        }
    }
    let root = tempfile::tempdir().unwrap();
    let mut runtime = runtime(root.path(), MemoryWorkshopStore::default());
    let block = Rc::new(Cell::new(true));
    let cancellations = Rc::new(Cell::new(0));
    runtime.install_living_storage(
        Box::new(HeldPackStore {
            native: NativeLivingStoreV2::new(root.path()),
            pack_jobs: BTreeSet::new(),
            block: block.clone(),
            cancellations: cancellations.clone(),
        }),
        Box::new(NativeLibraryCoordinator::new(root.path())),
    );
    runtime
        .select_menu_route(MainMenuRoute::GalaxySandbox)
        .unwrap();
    runtime.apply_sandbox_action(SandboxUiAction::ListPacks);
    runtime.update(Duration::ZERO);
    let frame = runtime
        .sandbox_frame(glam::Vec2::new(1440.0, 900.0), false)
        .unwrap();
    assert!(format!("{frame:?}").contains("Pack catalog has not been loaded"));
    runtime.apply_sandbox_action(SandboxUiAction::CreatorCancel);
    assert_eq!(cancellations.get(), 1);
    block.set(false);
    runtime.update(Duration::ZERO);
    // A save begun while the pack drawer is open is not a pack operation.
    runtime.apply_sandbox_action(SandboxUiAction::OpenPackLibrary);
    runtime.apply_sandbox_action(SandboxUiAction::Save);
    runtime.apply_sandbox_action(SandboxUiAction::CreatorCancel);
    assert_eq!(
        cancellations.get(),
        1,
        "closing pack drawer must preserve unrelated save"
    );
    settle(&mut runtime, |runtime| {
        runtime
            .living_storage_status()
            .contains("Galaxy and Continue selection saved")
    });
}

#[test]
fn pack_modal_cancel_after_won_mutation_relists_exactly_once() {
    use nyon::{
        app::client_runtime::{MainMenuRoute, living::LivingPackMutation},
        sandbox_session::SandboxUiAction,
    };
    use std::{cell::Cell, collections::BTreeSet, rc::Rc};
    struct WonStore {
        native: NativeLivingStoreV2,
        mutations: BTreeSet<LivingStoreJobIdV2>,
        hold: bool,
        won: Rc<Cell<bool>>,
        lists: Rc<Cell<usize>>,
    }
    impl LivingStoreV2 for WonStore {
        fn begin(
            &mut self,
            o: u64,
            r: LivingStoreRequestV2,
        ) -> Result<LivingStoreJobIdV2, LivingStoreErrorV2> {
            if matches!(r, LivingStoreRequestV2::ListPacks) {
                self.lists.set(self.lists.get() + 1);
            }
            let mutation = matches!(
                r,
                LivingStoreRequestV2::SetPackArchived { .. }
                    | LivingStoreRequestV2::DeletePack { .. }
            );
            let job = self.native.begin(o, r)?;
            if mutation {
                self.mutations.insert(job);
            }
            Ok(job)
        }
        fn poll(&mut self, o: u64, j: LivingStoreJobIdV2, b: usize) -> LivingStorePollV2 {
            let result = self.native.poll(o, j, b);
            if self.hold
                && self.mutations.contains(&j)
                && matches!(result, LivingStorePollV2::Ready(_))
            {
                self.won.set(true);
                LivingStorePollV2::Pending { work_done: 0 }
            } else {
                result
            }
        }
        fn cancel(
            &mut self,
            o: u64,
            j: LivingStoreJobIdV2,
        ) -> Result<LivingStoreCancelV2, LivingStoreErrorV2> {
            self.hold = false;
            let result = self.native.cancel(o, j)?;
            assert!(matches!(result, LivingStoreCancelV2::CommitAlreadyWon(_)));
            Ok(result)
        }
        fn release(&mut self, o: u64, j: LivingStoreJobIdV2) -> bool {
            self.mutations.remove(&j);
            self.native.release(o, j)
        }
    }
    let root = tempfile::tempdir().unwrap();
    let mut store = NativeLivingStoreV2::new(root.path());
    let bytes = std::str::from_utf8(nyon_workshop_core::living::LIVING_CORE_PACK_BYTES_V2)
        .unwrap()
        .replace(
            "\"pack_id\":\"nyon_living_core\"",
            "\"pack_id\":\"won_pack_qualification\"",
        )
        .into_bytes();
    let LivingStoreResultV2::Pack(pack) = living_request(
        &mut store,
        LivingStoreRequestV2::PutPack {
            canonical_bytes: bytes,
        },
    ) else {
        panic!()
    };
    let won = Rc::new(Cell::new(false));
    let lists = Rc::new(Cell::new(0));
    let mut runtime = runtime(root.path(), MemoryWorkshopStore::default());
    runtime.install_living_storage(
        Box::new(WonStore {
            native: store,
            mutations: BTreeSet::new(),
            hold: true,
            won: won.clone(),
            lists: lists.clone(),
        }),
        Box::new(NativeLibraryCoordinator::new(root.path())),
    );
    runtime
        .select_menu_route(MainMenuRoute::GalaxySandbox)
        .unwrap();
    runtime.apply_sandbox_action(SandboxUiAction::ListPacks);
    settle(&mut runtime, |r| {
        r.living_packs().iter().any(|p| p.hash == pack.hash)
    });
    runtime
        .mutate_living_pack(&pack, LivingPackMutation::SetArchived(true))
        .unwrap();
    settle(&mut runtime, |_| won.get());
    runtime.apply_sandbox_action(SandboxUiAction::CreatorCancel);
    settle(&mut runtime, |r| {
        r.living_packs()
            .iter()
            .any(|p| p.hash == pack.hash && p.archived)
    });
    assert_eq!(
        lists.get(),
        2,
        "initial list plus exactly one post-cancellation reconciliation"
    );
    for _ in 0..10 {
        runtime.update(Duration::ZERO);
    }
    assert_eq!(lists.get(), 2);
}

#[test]
fn cancelled_automatic_pack_refresh_does_not_hide_next_explicit_loaded_status() {
    use nyon::{
        app::client_runtime::{MainMenuRoute, living::LivingPackMutation},
        sandbox_session::SandboxUiAction,
    };
    use std::{cell::Cell, collections::BTreeSet, rc::Rc};
    struct WonStore {
        native: NativeLivingStoreV2,
        mutations: BTreeSet<LivingStoreJobIdV2>,
        hold: bool,
        list_jobs: BTreeSet<LivingStoreJobIdV2>,
        block_lists: Rc<Cell<bool>>,
        cancellations: Rc<Cell<usize>>,
        won: Rc<Cell<bool>>,
        lists: Rc<Cell<usize>>,
    }
    impl LivingStoreV2 for WonStore {
        fn begin(
            &mut self,
            o: u64,
            r: LivingStoreRequestV2,
        ) -> Result<LivingStoreJobIdV2, LivingStoreErrorV2> {
            if matches!(r, LivingStoreRequestV2::ListPacks) {
                self.lists.set(self.lists.get() + 1);
            }
            let listing = matches!(r, LivingStoreRequestV2::ListPacks);
            let mutation = matches!(
                r,
                LivingStoreRequestV2::SetPackArchived { .. }
                    | LivingStoreRequestV2::DeletePack { .. }
            );
            let job = self.native.begin(o, r)?;
            if listing {
                self.list_jobs.insert(job);
            }
            if mutation {
                self.mutations.insert(job);
            }
            Ok(job)
        }
        fn poll(&mut self, o: u64, j: LivingStoreJobIdV2, b: usize) -> LivingStorePollV2 {
            if self.block_lists.get() && self.list_jobs.contains(&j) {
                return LivingStorePollV2::Pending { work_done: 0 };
            }
            let result = self.native.poll(o, j, b);
            if self.hold
                && self.mutations.contains(&j)
                && matches!(result, LivingStorePollV2::Ready(_))
            {
                self.won.set(true);
                LivingStorePollV2::Pending { work_done: 0 }
            } else {
                result
            }
        }
        fn cancel(
            &mut self,
            o: u64,
            j: LivingStoreJobIdV2,
        ) -> Result<LivingStoreCancelV2, LivingStoreErrorV2> {
            self.cancellations.set(self.cancellations.get() + 1);
            self.hold = false;
            let result = self.native.cancel(o, j)?;
            if self.mutations.contains(&j) {
                assert!(matches!(result, LivingStoreCancelV2::CommitAlreadyWon(_)));
            } else {
                assert!(matches!(result, LivingStoreCancelV2::Cancelled));
            }
            Ok(result)
        }
        fn release(&mut self, o: u64, j: LivingStoreJobIdV2) -> bool {
            self.mutations.remove(&j);
            self.list_jobs.remove(&j);
            self.native.release(o, j)
        }
    }
    let root = tempfile::tempdir().unwrap();
    let mut store = NativeLivingStoreV2::new(root.path());
    let bytes = std::str::from_utf8(nyon_workshop_core::living::LIVING_CORE_PACK_BYTES_V2)
        .unwrap()
        .replace(
            "\"pack_id\":\"nyon_living_core\"",
            "\"pack_id\":\"won_pack_qualification\"",
        )
        .into_bytes();
    let LivingStoreResultV2::Pack(pack) = living_request(
        &mut store,
        LivingStoreRequestV2::PutPack {
            canonical_bytes: bytes,
        },
    ) else {
        panic!()
    };
    let block_lists = Rc::new(Cell::new(false));
    let cancellations = Rc::new(Cell::new(0));
    let won = Rc::new(Cell::new(false));
    let lists = Rc::new(Cell::new(0));
    let mut runtime = runtime(root.path(), MemoryWorkshopStore::default());
    runtime.install_living_storage(
        Box::new(WonStore {
            native: store,
            mutations: BTreeSet::new(),
            hold: true,
            list_jobs: BTreeSet::new(),
            block_lists: block_lists.clone(),
            cancellations: cancellations.clone(),
            won: won.clone(),
            lists: lists.clone(),
        }),
        Box::new(NativeLibraryCoordinator::new(root.path())),
    );
    runtime
        .select_menu_route(MainMenuRoute::GalaxySandbox)
        .unwrap();
    runtime.apply_sandbox_action(SandboxUiAction::ListPacks);
    settle(&mut runtime, |r| {
        r.living_packs().iter().any(|p| p.hash == pack.hash)
    });
    runtime
        .mutate_living_pack(&pack, LivingPackMutation::SetArchived(true))
        .unwrap();
    settle(&mut runtime, |_| won.get());
    block_lists.set(true);
    runtime.apply_sandbox_action(SandboxUiAction::CreatorCancel);
    settle(&mut runtime, |_| lists.get() == 2);
    runtime.update(Duration::ZERO);
    runtime.apply_sandbox_action(SandboxUiAction::OpenPackLibrary);
    runtime.apply_sandbox_action(SandboxUiAction::CreatorCancel);
    assert_eq!(
        cancellations.get(),
        2,
        "won mutation and pending automatic list each cancelled once"
    );
    block_lists.set(false);
    runtime.update(Duration::ZERO);
    runtime.apply_sandbox_action(SandboxUiAction::ListPacks);
    settle(&mut runtime, |r| {
        r.living_storage_status() == "Living catalog packs listed"
    });
    assert!(
        runtime
            .living_packs()
            .iter()
            .any(|p| p.hash == pack.hash && p.archived)
    );
    assert_eq!(lists.get(), 3);
    for _ in 0..10 {
        runtime.update(Duration::ZERO);
    }
    assert_eq!(lists.get(), 3, "cancelled refresh must not loop");
    assert_eq!(cancellations.get(), 2);
}
