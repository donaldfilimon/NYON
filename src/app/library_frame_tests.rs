//! The Library screen through a real `App`: route-design task 8, slice 1.
//!
//! The frame suite proves geometry. These prove the wiring the frame cannot
//! see: that `ClientScreen::Library` reaches its own builder instead of the
//! shell placeholder it replaced, that a Library control resolves against the
//! Library model, and that a control the model disables cannot reach the store.

use super::*;
use crate::app::client_runtime::LibrarySlotsStatus;
use crate::scenario::{ScenarioDraft, store::MemoryScenarioStore};
use crate::ui::workshop_layout::WorkshopLayoutMode;
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

pub(super) fn library_app() -> (TestApp, SlotId) {
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

    // With a row selected, every action exists, and the ones that would
    // mutate or open without a route still refuse: their capability flags are
    // off. Archive is live since task 9a, but only opens a confirmation.
    assert!(control_enabled(&app, "library.action.archive"));
    for id in [
        "library.action.open",
        "library.action.rename",
        "library.action.use-for-continue",
        "library.action.export",
    ] {
        assert!(!control_enabled(&app, id), "{id} is live without a route");
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

fn placed_slot_ids(app: &TestApp) -> Vec<String> {
    app.platform_ui
        .as_ref()
        .unwrap()
        .controls
        .iter()
        .map(|control| control.action_id.as_str())
        .filter(|id| id.starts_with("library.slot."))
        .map(str::to_owned)
        .collect()
}

#[test]
fn next_and_previous_page_through_every_save_and_keep_focus_on_the_pager() {
    let core = AppCore::new_with_preferences(
        ScenarioDraft::factory_default().validated().unwrap(),
        MemoryScenarioStore::default(),
        MemoryPreferencesStore::default(),
    );
    let (mut store, first) = store_with_one_slot();
    let catalog =
        decode_catalog_pack(include_bytes!("../../assets/workshop/core-pack-v1.json")).unwrap();
    for seed in 1..16u64 {
        let archive = encode_archive(&WorkshopHistory::from_seed_u64(catalog.clone(), seed))
            .unwrap()
            .bytes
            .into_boxed_slice();
        let job = store
            .start(WorkshopStoreRequest::CreateSlot {
                name: SlotName::new(format!("Forge {seed}")).unwrap(),
                archive,
            })
            .unwrap();
        assert!(matches!(store.poll(job), StoreJobState::Complete(Ok(_))));
    }
    let mut app = App::with_event_proxy(core, store, AppEventProxy::Headless);
    app.runtime
        .classic_mut()
        .set_viewport(glam::Vec2::new(1280.0, 480.0));
    app.runtime.open_library().unwrap();
    app.runtime.update(std::time::Duration::ZERO);
    app.build_frame();
    assert_eq!(app.runtime.library_slots().unwrap().slots.len(), 16);
    let first_page = placed_slot_ids(&app);
    assert!(
        first_page.len() < 16,
        "the viewport must overflow for this test"
    );
    assert!(first_page.contains(&format!("library.slot.{}", first.0)));

    let next = SemanticActionId::new("library.rows.next");
    let previous = SemanticActionId::new("library.rows.previous");
    let mut seen: std::collections::BTreeSet<String> = first_page.iter().cloned().collect();
    for _ in 0..16 {
        app.activate_platform_action_id(&next, InputModality::Keyboard);
        app.build_frame();
        assert_eq!(app.ui_focus.focused(), Some(&next), "focus left the pager");
        seen.extend(placed_slot_ids(&app));
    }
    assert_eq!(seen.len(), 16, "Next did not reach every save");
    // Next at the end is a no-op, not a blank page.
    assert!(!placed_slot_ids(&app).is_empty());

    for _ in 0..16 {
        app.activate_platform_action_id(&previous, InputModality::Pointer);
        app.build_frame();
    }
    assert_eq!(
        placed_slot_ids(&app),
        first_page,
        "Previous did not return to the top"
    );

    // Leaving forgets the window, like the selection.
    app.activate_platform_action_id(&next, InputModality::Keyboard);
    app.build_frame();
    assert_ne!(placed_slot_ids(&app), first_page);
    app.runtime.close_library();
    app.build_frame();
    app.runtime.open_library().unwrap();
    app.runtime.update(std::time::Duration::ZERO);
    app.build_frame();
    assert_eq!(
        placed_slot_ids(&app),
        first_page,
        "reopening kept the old window"
    );
}

#[test]
fn in_compact_selecting_a_row_opens_the_actions_sheet_and_back_returns_to_it() {
    let (mut app, slot) = library_app();
    app.runtime
        .classic_mut()
        .set_viewport(glam::Vec2::new(723.0, 802.0));
    app.build_frame();
    let frame = app.platform_ui.as_ref().unwrap();
    assert_eq!(frame.layout.mode, WorkshopLayoutMode::Compact);
    assert!(frame.drawer.is_none());
    assert!(!has_control(&app, "library.action.open"));

    let row = SemanticActionId::new(format!("library.slot.{}", slot.0));
    app.ui_focus.request_focus(&row);
    app.activate_platform_action_id(&row, InputModality::Keyboard);
    app.build_frame();
    assert!(
        app.platform_ui.as_ref().unwrap().drawer.is_some(),
        "no sheet"
    );
    assert!(has_control(&app, "library.action.open"));
    // The sheet covers the lower part of the canvas; whatever still shows is
    // above it, never under it.
    let frame = app.platform_ui.as_ref().unwrap();
    let sheet = frame.drawer.unwrap();
    assert!(
        frame
            .controls
            .iter()
            .all(|control| sheet.contains_rect(control.bounds) || !control.bounds.overlaps(sheet)),
        "a control shows through the sheet"
    );
    let back = SemanticActionId::new("library.sheet.close");
    assert_eq!(
        app.ui_focus.focused(),
        Some(&back),
        "focus did not move into the sheet"
    );

    app.activate_platform_action_id(&back, InputModality::Keyboard);
    app.build_frame();
    assert!(app.platform_ui.as_ref().unwrap().drawer.is_none());
    assert_eq!(
        app.ui_focus.focused(),
        Some(&row),
        "focus did not return to the row"
    );
    assert_eq!(
        app.selected_library_slot,
        Some(slot),
        "Back dropped the selection"
    );

    // Transfer opens its own page, and Back returns to Transfer.
    let transfer = SemanticActionId::new("library.sheet.transfer");
    app.activate_platform_action_id(&transfer, InputModality::Pointer);
    app.build_frame();
    assert!(has_control(&app, "library.transfer.import-archive"));
    assert!(!has_control(&app, "library.action.open"));
    app.activate_platform_action_id(&back, InputModality::Pointer);
    app.build_frame();
    assert_eq!(
        app.ui_focus.focused(),
        Some(&transfer),
        "focus did not return to Transfer"
    );

    // Growing out of Compact closes the sheet; it does not come back on shrink.
    app.activate_platform_action_id(&transfer, InputModality::Pointer);
    app.build_frame();
    app.runtime
        .classic_mut()
        .set_viewport(glam::Vec2::new(1440.0, 900.0));
    app.build_frame();
    assert!(app.platform_ui.as_ref().unwrap().drawer.is_none());
    app.runtime
        .classic_mut()
        .set_viewport(glam::Vec2::new(723.0, 802.0));
    app.build_frame();
    assert!(
        app.platform_ui.as_ref().unwrap().drawer.is_none(),
        "the sheet came back"
    );

    // Leaving forgets an open sheet.
    app.activate_platform_action_id(&transfer, InputModality::Pointer);
    app.build_frame();
    app.runtime.close_library();
    app.build_frame();
    app.runtime.open_library().unwrap();
    app.runtime.update(std::time::Duration::ZERO);
    app.build_frame();
    assert!(
        app.platform_ui.as_ref().unwrap().drawer.is_none(),
        "reopened with a sheet"
    );
}

#[test]
fn in_a_docked_layout_selecting_a_row_opens_no_sheet() {
    let (mut app, slot) = library_app();
    let row = SemanticActionId::new(format!("library.slot.{}", slot.0));
    app.activate_platform_action_id(&row, InputModality::Pointer);
    app.build_frame();
    let frame = app.platform_ui.as_ref().unwrap();
    assert_ne!(frame.layout.mode, WorkshopLayoutMode::Compact);
    assert!(frame.drawer.is_none());
    assert!(has_control(&app, &format!("library.slot.{}", slot.0)));
    assert!(!has_control(&app, "library.sheet.close"));
}

/// Runs the runtime until the slot-request lane is idle again, bounded.
fn settle(app: &mut TestApp) {
    for _ in 0..8 {
        if app.runtime.library_slots_status() == LibrarySlotsStatus::Idle
            && app.runtime.library_slots().is_some()
        {
            break;
        }
        app.runtime.update(std::time::Duration::ZERO);
    }
    app.build_frame();
}

fn archived(app: &TestApp, slot: SlotId) -> bool {
    app.runtime
        .library_slots()
        .unwrap()
        .slots
        .iter()
        .find(|summary| summary.id == slot)
        .unwrap()
        .archived
}

fn select(app: &mut TestApp, slot: SlotId) {
    let row = SemanticActionId::new(format!("library.slot.{}", slot.0));
    app.activate_platform_action_id(&row, InputModality::Pointer);
    app.build_frame();
}

/// Task 9a, addendum §3: Archive asks first, Cancel changes nothing, and the
/// confirmed change reaches the store and comes back as Unarchive under the
/// same identifier, which is where focus returns.
#[test]
fn archive_asks_first_and_only_the_confirmed_change_reaches_the_store() {
    let (mut app, slot) = library_app();
    select(&mut app, slot);
    let archive = SemanticActionId::new("library.action.archive");
    let cancel = SemanticActionId::new(crate::ui::library::LIBRARY_CONFIRM_CANCEL_ACTION);
    let submit = SemanticActionId::new(crate::ui::library::LIBRARY_CONFIRM_SUBMIT_ACTION);

    for (answer, changes) in [(cancel.clone(), false), (submit.clone(), true)] {
        assert!(app.ui_focus.request_focus(&archive));
        app.activate_platform_action_id(&archive, InputModality::Keyboard);
        assert_eq!(
            app.runtime.library_slots_status(),
            LibrarySlotsStatus::Idle,
            "Archive reached the store before it was confirmed"
        );
        app.build_frame();
        let frame = app.platform_ui.as_ref().unwrap();
        assert!(frame.modal.is_some(), "no confirmation");
        assert!(app.ui_focus.modal_is_open());
        assert_eq!(
            app.ui_focus.focused(),
            Some(&cancel),
            "focus not in the dialog"
        );
        assert!(
            !control_enabled(&app, "library.close"),
            "the screen behind the dialog is live"
        );

        app.activate_platform_action_id(&answer, InputModality::Keyboard);
        assert!(
            app.library_confirmation.is_none(),
            "{answer:?} left it open"
        );
        assert!(!app.ui_focus.modal_is_open());
        settle(&mut app);
        assert_eq!(archived(&app, slot), changes, "after {answer:?}");
        assert!(app.platform_ui.as_ref().unwrap().modal.is_none());
        assert_eq!(app.ui_focus.focused(), Some(&archive), "after {answer:?}");
    }
    let unarchive = app
        .platform_ui
        .as_ref()
        .unwrap()
        .controls
        .iter()
        .find(|control| control.action_id == archive)
        .unwrap();
    assert_eq!(unarchive.label, "Unarchive");

    // And back: Unarchive also asks, and restores the row.
    app.activate_platform_action_id(&archive, InputModality::Pointer);
    app.build_frame();
    assert!(app.platform_ui.as_ref().unwrap().modal.is_some());
    app.activate_platform_action_id(&submit, InputModality::Pointer);
    settle(&mut app);
    assert!(!archived(&app, slot));
}

/// A confirm the runtime refuses leaves the dialog open, so the user is not
/// told by a vanished dialog that something happened. Leaving the Library, or
/// a save that drops out of the list, closes it together with its trap.
#[test]
fn a_refused_confirmation_stays_open_and_leaving_closes_it() {
    let (mut app, slot) = library_app();
    select(&mut app, slot);
    let request = crate::ui::library::LibraryConfirmationRequest {
        kind: crate::ui::library::LibraryConfirmationKind::Archive,
        slot,
    };
    app.apply_library_intent(LibraryUiIntent::ArchiveSlot { slot });
    app.build_frame();
    // Occupy the one lane behind the dialog's back, then confirm.
    app.runtime.refresh_library_slots().unwrap();
    app.apply_library_intent(LibraryUiIntent::SubmitConfirmation(request));
    assert_eq!(
        app.library_confirmation,
        Some(request),
        "a refusal closed it"
    );
    assert!(app.ui_focus.modal_is_open());
    settle(&mut app);
    assert!(!archived(&app, slot));

    // A stale request (not the open one) is ignored and closes nothing.
    let stale = crate::ui::library::LibraryConfirmationRequest {
        kind: crate::ui::library::LibraryConfirmationKind::Unarchive,
        slot,
    };
    app.apply_library_intent(LibraryUiIntent::SubmitConfirmation(stale));
    assert_eq!(app.library_confirmation, Some(request));

    app.runtime.close_library();
    app.build_frame();
    assert!(app.library_confirmation.is_none());
    assert!(
        !app.ui_focus.modal_is_open(),
        "the trap outlived the Library"
    );

    // A request naming a save the list no longer holds closes on the next frame.
    app.runtime.open_library().unwrap();
    settle(&mut app);
    app.apply_library_intent(LibraryUiIntent::ArchiveSlot { slot: SlotId(999) });
    assert!(app.ui_focus.modal_is_open());
    app.build_frame();
    assert!(app.library_confirmation.is_none());
    assert!(!app.ui_focus.modal_is_open());
    assert!(app.platform_ui.as_ref().unwrap().modal.is_none());
}
