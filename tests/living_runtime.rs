//! Runtime transport dispatch is independent of redraw cadence.
use nyon::{
    app::{
        AppCore,
        client_runtime::{ClientRuntime, MainMenuRoute},
    },
    preferences::store::MemoryPreferencesStore,
    sandbox_session::{SandboxSpeed, SandboxUiAction},
    scenario::{ScenarioDraft, store::MemoryScenarioStore},
    workshop::store::MemoryWorkshopStore,
};
use std::time::Duration;

fn bare_runtime() -> ClientRuntime<MemoryScenarioStore, MemoryPreferencesStore, MemoryWorkshopStore>
{
    let classic = AppCore::new_with_preferences(
        ScenarioDraft::factory_default().validated().unwrap(),
        MemoryScenarioStore::default(),
        MemoryPreferencesStore::default(),
    );
    ClientRuntime::new(classic, MemoryWorkshopStore::default())
}

fn runtime() -> ClientRuntime<MemoryScenarioStore, MemoryPreferencesStore, MemoryWorkshopStore> {
    let mut runtime = bare_runtime();
    runtime
        .select_menu_route(MainMenuRoute::GalaxySandbox)
        .unwrap();
    runtime
}

#[test]
fn repeated_unheld_redraws_keep_fractional_time_and_modal_holds_clear_it() {
    let mut runtime = runtime();
    runtime.apply_sandbox_action(SandboxUiAction::TogglePause);
    for _ in 0..100 {
        runtime.set_sandbox_suspended(false);
        runtime.update(Duration::from_millis(10));
        for _ in 0..100 {
            runtime.update(Duration::ZERO);
        }
    }
    assert_eq!(runtime.sandbox_tick(), Some(10));
    runtime.update(Duration::from_millis(90));
    for _ in 0..100 {
        runtime.update(Duration::ZERO);
    }
    runtime.set_sandbox_suspended(true);
    runtime.update(Duration::from_secs(10));
    assert_eq!(runtime.sandbox_tick(), Some(10));
    runtime.set_sandbox_suspended(false);
    runtime.update(Duration::from_millis(10));
    for _ in 0..100 {
        runtime.update(Duration::ZERO);
    }
    assert_eq!(runtime.sandbox_tick(), Some(10));
    runtime.update(Duration::from_millis(90));
    for _ in 0..100 {
        runtime.update(Duration::ZERO);
    }
    assert_eq!(runtime.sandbox_tick(), Some(11));
}

#[test]
fn typed_controls_step_only_when_paused_and_apply_requested_speed() {
    let mut runtime = runtime();
    runtime.apply_sandbox_action(SandboxUiAction::Step);
    for _ in 0..100 {
        runtime.update(Duration::ZERO);
    }
    assert_eq!(runtime.sandbox_tick(), Some(1));
    runtime.apply_sandbox_action(SandboxUiAction::Speed(SandboxSpeed::Fast));
    runtime.apply_sandbox_action(SandboxUiAction::TogglePause);
    runtime.apply_sandbox_action(SandboxUiAction::Step);
    for _ in 0..100 {
        runtime.update(Duration::ZERO);
    }
    assert_eq!(runtime.sandbox_tick(), Some(1));
    runtime.update(Duration::from_secs(1));
    for _ in 0..1000 {
        runtime.update(Duration::ZERO);
    }
    assert_eq!(runtime.sandbox_tick(), Some(21)); // bounded work, sequential ticks
    runtime.apply_sandbox_action(SandboxUiAction::TogglePause);
    runtime.update(Duration::from_secs(10));
    assert_eq!(runtime.sandbox_tick(), Some(21));
}

#[test]
fn leaving_and_reopening_preserves_the_resident_galaxy() {
    let mut runtime = runtime();
    runtime.step_sandbox(10).unwrap();
    runtime.close_sandbox();
    assert_eq!(runtime.sandbox_tick(), Some(10));
    runtime
        .select_menu_route(MainMenuRoute::GalaxySandbox)
        .unwrap();
    assert_eq!(runtime.sandbox_tick(), Some(10));
    assert_eq!(runtime.sandbox_paused(), Some(true));
}

use nyon::{
    app::client_runtime::ClientScreen,
    library_selection::native::{NativeLibraryCoordinator, NativeLibrarySelection},
    living_store::{native::NativeLivingStoreV2, *},
};
use std::{cell::Cell, collections::BTreeSet, path::Path, rc::Rc, time::Instant};

type Runtime = ClientRuntime<MemoryScenarioStore, MemoryPreferencesStore, MemoryWorkshopStore>;

/// Fault completion of an accepted job, before its native mutation runs.
struct PollFaultStore {
    native: NativeLivingStoreV2,
    fault: Rc<Cell<u8>>,
    jobs: BTreeSet<LivingStoreJobIdV2>,
}
impl LivingStoreV2 for PollFaultStore {
    fn begin(
        &mut self,
        owner: u64,
        request: LivingStoreRequestV2,
    ) -> Result<LivingStoreJobIdV2, LivingStoreErrorV2> {
        let should_fail = match self.fault.get() {
            1 => matches!(
                request,
                LivingStoreRequestV2::SaveSlot { .. } | LivingStoreRequestV2::ListSlots
            ),
            2 => matches!(request, LivingStoreRequestV2::WriteNotebook { .. }),
            _ => false,
        };
        let job = self.native.begin(owner, request)?;
        if should_fail {
            self.fault.set(0);
            self.jobs.insert(job);
        }
        Ok(job)
    }
    fn poll(&mut self, owner: u64, job: LivingStoreJobIdV2, budget: usize) -> LivingStorePollV2 {
        if self.jobs.remove(&job) {
            self.native.cancel(owner, job).unwrap();
            self.native.release(owner, job);
            LivingStorePollV2::Failed(LivingStoreErrorV2::Storage(
                "injected quota completion".into(),
            ))
        } else {
            self.native.poll(owner, job, budget)
        }
    }
    fn cancel(
        &mut self,
        owner: u64,
        job: LivingStoreJobIdV2,
    ) -> Result<LivingStoreCancelV2, LivingStoreErrorV2> {
        self.native.cancel(owner, job)
    }
    fn release(&mut self, owner: u64, job: LivingStoreJobIdV2) -> bool {
        self.native.release(owner, job)
    }
}
fn stored_runtime(root: &Path, fault: Rc<Cell<u8>>) -> Runtime {
    let mut runtime = runtime();
    runtime.install_living_storage(
        Box::new(PollFaultStore {
            native: NativeLivingStoreV2::new(root),
            fault,
            jobs: BTreeSet::new(),
        }),
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
            "Living runtime did not settle: {}",
            runtime.living_storage_status()
        );
        std::thread::sleep(Duration::from_millis(1));
    }
}
#[test]
fn active_resident_poll_failure_preserves_save_and_leave_then_retry_continue() {
    for (leave, failure_phase) in [(false, 1), (true, 1), (false, 2), (true, 2)] {
        let root = tempfile::tempdir().unwrap();
        let fault = Rc::new(Cell::new(0));
        let mut runtime = stored_runtime(root.path(), fault.clone());
        runtime.save_living(false).unwrap();
        settle(&mut runtime, |r| !r.living_dirty());
        let selected = NativeLibrarySelection::new(root.path())
            .read()
            .unwrap()
            .unwrap();
        runtime.step_sandbox(2).unwrap();
        let diagnostics = runtime.diagnostics().len();
        fault.set(failure_phase);
        runtime.save_living(leave).unwrap();
        settle(&mut runtime, |r| r.diagnostics().len() > diagnostics);
        assert_eq!(runtime.screen(), ClientScreen::GalaxySandbox);
        assert_eq!(runtime.sandbox_tick(), Some(2));
        assert!(runtime.living_dirty());
        assert_eq!(runtime.diagnostics().len(), diagnostics + 1);
        assert!(runtime.recovery_diagnostic().is_none());
        assert!(
            runtime
                .living_storage_status()
                .contains("storage: injected quota completion")
        );
        assert!(runtime.living_storage_status().contains("retry"));
        assert_eq!(
            NativeLibrarySelection::new(root.path()).read().unwrap(),
            Some(selected)
        );
        // Notebook failure happens after archive commit. Continue still restores
        // the older acknowledged snapshot, never that newer unselected head.
        let mut old_continue = bare_runtime();
        old_continue.install_living_storage(
            Box::new(NativeLivingStoreV2::new(root.path())),
            Box::new(NativeLibraryCoordinator::new(root.path())),
        );
        old_continue
            .select_menu_route(MainMenuRoute::Continue)
            .unwrap();
        settle(&mut old_continue, |r| {
            r.screen() == ClientScreen::GalaxySandbox
        });
        assert_eq!(old_continue.sandbox_tick(), Some(0));
        assert!(!old_continue.living_dirty());
        drop(old_continue);
        runtime.apply_sandbox_action(SandboxUiAction::Step);
        settle(&mut runtime, |r| r.sandbox_tick() == Some(3));
        runtime.save_living(false).unwrap();
        settle(&mut runtime, |r| !r.living_dirty());
        drop(runtime);
        let mut fresh = bare_runtime();
        fresh.install_living_storage(
            Box::new(NativeLivingStoreV2::new(root.path())),
            Box::new(NativeLibraryCoordinator::new(root.path())),
        );
        fresh.select_menu_route(MainMenuRoute::Continue).unwrap();
        settle(&mut fresh, |r| r.screen() == ClientScreen::GalaxySandbox);
        assert_eq!(fresh.sandbox_tick(), Some(3));
        assert!(!fresh.living_dirty());
    }
}
#[test]
fn inactive_or_absent_resident_poll_failure_retains_recovery_policy() {
    for resident in [false, true] {
        let root = tempfile::tempdir().unwrap();
        let fault = Rc::new(Cell::new(1));
        let mut runtime = if resident {
            let mut runtime = stored_runtime(root.path(), fault);
            runtime.close_sandbox();
            runtime
        } else {
            let mut runtime = bare_runtime();
            runtime.install_living_storage(
                Box::new(PollFaultStore {
                    native: NativeLivingStoreV2::new(root.path()),
                    fault,
                    jobs: BTreeSet::new(),
                }),
                Box::new(NativeLibraryCoordinator::new(root.path())),
            );
            runtime
        };
        runtime.list_living_saves().unwrap();
        settle(&mut runtime, |r| r.recovery_diagnostic().is_some());
        assert_eq!(runtime.screen(), ClientScreen::RecoverableError);
        assert!(
            runtime
                .recovery_diagnostic()
                .unwrap()
                .message
                .contains("injected quota completion")
        );
    }
}
