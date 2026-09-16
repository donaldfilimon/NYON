//! The Library screen through a real `App`: route-design task 8, slice 1.
//!
//! The frame suite proves geometry. These prove the wiring the frame cannot
//! see: that `ClientScreen::Library` reaches its own builder instead of the
//! shell placeholder it replaced, that a Library control resolves against the
//! Library model, and that a control the model disables cannot reach the store.

use super::*;
use crate::app::client_runtime::LibrarySlotsStatus;
use crate::scenario::{ScenarioDraft, store::MemoryScenarioStore};
use crate::workshop::store::{
    SlotId, SlotName, StoreJobState, WorkshopStoreRequest, WorkshopStoreResult,
};
use crate::workshop::{WorkshopHistory, decode_catalog_pack, encode_archive};

type TestApp = App<MemoryScenarioStore>;

/// A store already holding one save, so the Library has a row to select.
fn store_with_one_slot() -> (MemoryWorkshopStore, SlotId) {
    let mut store = MemoryWorkshopStore::default();
    let catalog =
        decode_catalog_pack(include_bytes!("../../assets/workshop/core-pack-v1.json")).unwrap();
    let archive = encode_archive(&WorkshopHistory::from_seed_u64(catalog, 0x0F18))
        .unwrap()
        .bytes
        .into_boxed_slice();
    let job = store
        .start(WorkshopStoreRequest::CreateSlot {
            name: SlotName::new("Two-System Forge").unwrap(),
            archive,
        })
        .unwrap();
    let StoreJobState::Complete(Ok(WorkshopStoreResult::SlotCreated { slot, .. })) =
        store.poll(job)
    else {
        panic!("the memory store did not create the fixture slot");
    };
    (store, slot)
}

fn library_app() -> (TestApp, SlotId) {
    let core = AppCore::new_with_preferences(
        ScenarioDraft::factory_default().validated().unwrap(),
        MemoryScenarioStore::default(),
        MemoryPreferencesStore::default(),
    );
    let (store, slot) = store_with_one_slot();
    let mut app = App::with_event_proxy(core, store, AppEventProxy::Headless);
    app.runtime
        .classic_mut()
        .set_viewport(glam::Vec2::new(1440.0, 900.0));
    app.runtime.open_library().unwrap();
    // `open_library` dispatched `ListSlots`; one update delivers the list.
    app.runtime.update(std::time::Duration::ZERO);
    assert_eq!(app.runtime.library_slots_status(), LibrarySlotsStatus::Idle);
    app.build_frame();
    (app, slot)
}

fn has_control(app: &TestApp, id: &str) -> bool {
    app.platform_ui
        .as_ref()
        .is_some_and(|frame| frame.controls.iter().any(|c| c.action_id.as_str() == id))
}

fn control_enabled(app: &TestApp, id: &str) -> bool {
    app.platform_ui
        .as_ref()
        .and_then(|frame| frame.controls.iter().find(|c| c.action_id.as_str() == id))
        .unwrap_or_else(|| panic!("no placed control {id}"))
        .enabled
}

#[test]
fn the_library_screen_builds_its_own_frame_and_close_returns_to_the_menu() {
    let (mut app, _) = library_app();
    assert_eq!(app.runtime.screen(), ClientScreen::Library);
    assert!(app.library_ui.is_some(), "the Library model was not built");
    assert!(app.workshop_ui.is_none());
    assert!(has_control(&app, "library.close"));
    assert!(
        !has_control(&app, "shell.library.close"),
        "the shell placeholder is still drawing a second Close"
    );

    app.activate_platform_action_id(
        &SemanticActionId::new("library.close"),
        InputModality::Keyboard,
    );
    assert_eq!(app.runtime.screen(), ClientScreen::MainMenu);

    // The next frame is the menu's, and it must drop the Library model so a
    // stale Library control cannot activate from it.
    app.build_frame();
    assert!(app.library_ui.is_none());
    assert!(!has_control(&app, "library.close"));
}

#[test]
fn selecting_a_row_is_client_state_and_enables_nothing_the_store_cannot_serve() {
    let (mut app, slot) = library_app();
    let row = format!("library.slot.{}", slot.0);
    assert!(control_enabled(&app, &row));

    app.activate_platform_action_id(&SemanticActionId::new(&row), InputModality::Pointer);
    assert_eq!(app.selected_library_slot, Some(slot));
    assert_eq!(
        app.runtime.library_slots_status(),
        LibrarySlotsStatus::Idle,
        "selection must not touch the store"
    );
    app.build_frame();
    assert_eq!(
        app.library_ui.as_ref().unwrap().actions.selected,
        Some(slot),
        "the model did not pick up the selection"
    );

    // With a row selected, every action exists, and the three that would
    // mutate or open still refuse: their capability flags are off.
    for id in [
        "library.action.open",
        "library.action.rename",
        "library.action.archive",
        "library.action.use-for-continue",
        "library.action.export",
    ] {
        assert!(!control_enabled(&app, id), "{id} is live in slice 1");
        app.activate_platform_action_id(&SemanticActionId::new(id), InputModality::Keyboard);
        assert_eq!(
            app.runtime.screen(),
            ClientScreen::Library,
            "{id} left the screen"
        );
        assert_eq!(
            app.runtime.library_slots_status(),
            LibrarySlotsStatus::Idle,
            "{id} reached the store through a disabled control"
        );
    }
    assert_eq!(app.runtime.library_slots().unwrap().slots.len(), 1);
}

#[test]
fn refresh_reaches_the_store_and_the_list_comes_back() {
    let (mut app, slot) = library_app();
    assert!(control_enabled(&app, "library.refresh"));
    app.activate_platform_action_id(
        &SemanticActionId::new("library.refresh"),
        InputModality::Pointer,
    );
    assert!(
        matches!(
            app.runtime.library_slots_status(),
            LibrarySlotsStatus::Working { .. }
        ),
        "Refresh did not dispatch a listing"
    );
    app.runtime.update(std::time::Duration::ZERO);
    assert_eq!(app.runtime.library_slots_status(), LibrarySlotsStatus::Idle);
    assert_eq!(
        app.runtime
            .library_slots()
            .unwrap()
            .slots
            .iter()
            .map(|summary| summary.id)
            .collect::<Vec<_>>(),
        vec![slot]
    );
}

#[test]
fn escape_forgets_the_selection_exactly_as_close_does() {
    // Escape reaches `close_library` directly, not through the Close intent,
    // so clearing the selection only in that intent would leave a stale one
    // behind for the next visit.
    let (mut app, slot) = library_app();
    let row = SemanticActionId::new(format!("library.slot.{}", slot.0));
    app.activate_platform_action_id(&row, InputModality::Pointer);
    assert_eq!(app.selected_library_slot, Some(slot));

    app.runtime.close_library();
    app.build_frame();
    assert_eq!(app.runtime.screen(), ClientScreen::MainMenu);
    assert_eq!(app.selected_library_slot, None, "Escape kept the selection");

    app.runtime.open_library().unwrap();
    app.runtime.update(std::time::Duration::ZERO);
    app.build_frame();
    assert_eq!(app.library_ui.as_ref().unwrap().actions.selected, None);
}

#[test]
fn a_window_below_the_layout_floor_still_builds_the_library_frame() {
    // A browser canvas can be any size and the viewport is zero for a frame
    // at startup. The shell placeholder this replaced clamped to 640 by 480
    // and never panicked; the Library frame must not either.
    let (mut app, _) = library_app();
    for viewport in [
        glam::Vec2::new(100.0, 100.0),
        glam::Vec2::ZERO,
        glam::Vec2::new(319.0, 2000.0),
    ] {
        app.runtime.classic_mut().set_viewport(viewport);
        app.build_frame();
        assert!(has_control(&app, "library.close"), "{viewport}: no Close");
    }
}

#[test]
fn the_main_menu_library_control_opens_the_library_and_close_returns_to_it() {
    // The route design found the screen reachable only through the runtime
    // seam. This drives the platform control a user would press.
    let core = AppCore::new_with_preferences(
        ScenarioDraft::factory_default().validated().unwrap(),
        MemoryScenarioStore::default(),
        MemoryPreferencesStore::default(),
    );
    let (store, slot) = store_with_one_slot();
    let mut app = App::with_event_proxy(core, store, AppEventProxy::Headless);
    app.runtime
        .classic_mut()
        .set_viewport(glam::Vec2::new(1280.0, 480.0));
    app.build_frame();
    assert_eq!(app.runtime.screen(), ClientScreen::MainMenu);
    assert!(control_enabled(&app, "shell.menu.library"));

    app.activate_platform_action_id(
        &SemanticActionId::new("shell.menu.library"),
        InputModality::Pointer,
    );
    assert_eq!(app.runtime.screen(), ClientScreen::Library);
    app.runtime.update(std::time::Duration::ZERO);
    app.build_frame();
    assert!(app.library_ui.is_some(), "the Library frame was not built");
    assert!(has_control(&app, &format!("library.slot.{}", slot.0)));

    app.activate_platform_action_id(
        &SemanticActionId::new("library.close"),
        InputModality::Keyboard,
    );
    assert_eq!(app.runtime.screen(), ClientScreen::MainMenu);
    app.build_frame();
    assert!(has_control(&app, "shell.menu.library"));
}
