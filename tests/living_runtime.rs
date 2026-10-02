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

fn runtime() -> ClientRuntime<MemoryScenarioStore, MemoryPreferencesStore, MemoryWorkshopStore> {
    let classic = AppCore::new_with_preferences(
        ScenarioDraft::factory_default().validated().unwrap(),
        MemoryScenarioStore::default(),
        MemoryPreferencesStore::default(),
    );
    let mut runtime = ClientRuntime::new(classic, MemoryWorkshopStore::default());
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
