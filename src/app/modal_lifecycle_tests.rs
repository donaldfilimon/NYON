use super::*;
use crate::preferences::store::PreferencesStoreError;
use crate::scenario::{ScenarioDraft, store::MemoryScenarioStore};
use crate::ui::workshop_view::WorkshopViewAction;
use nyon_workshop_core::{
    BatchLocalId, CatalogId, CreatorBatchV1, CreatorOpV1, GalaxyPointV1, ObjectName, ObjectRefV1,
};
use std::{cell::RefCell, rc::Rc};

type TestApp = App<MemoryScenarioStore>;

#[derive(Clone, Default)]
struct RecordingPreferencesStore {
    slot: Rc<RefCell<Option<String>>>,
}

impl PreferencesStore for RecordingPreferencesStore {
    fn load(&self) -> Result<Option<String>, PreferencesStoreError> {
        Ok(self.slot.borrow().clone())
    }

    fn save(&mut self, payload: &str) -> Result<(), PreferencesStoreError> {
        *self.slot.borrow_mut() = Some(payload.to_owned());
        Ok(())
    }
}

fn submit(app: &mut TestApp, operations: Vec<CreatorOpV1>) {
    for chunk in operations.chunks(128) {
        let snapshot = app.runtime.workshop_snapshot().unwrap();
        let batch = CreatorBatchV1 {
            expected_cursor: snapshot.active_view.view_cursor,
            expected_tick: snapshot.active_view.tick,
            operations: chunk.to_vec(),
        };
        app.apply_workshop_intent(WorkshopUiIntent::Dispatch(WorkshopAction::Submit(batch)));
        app.runtime.update(std::time::Duration::ZERO);
    }
}

pub(super) fn capacity_app() -> TestApp {
    let core = AppCore::new_with_preferences(
        ScenarioDraft::factory_default().validated().unwrap(),
        MemoryScenarioStore::default(),
        MemoryPreferencesStore::default(),
    );
    let mut app = App::with_event_proxy(
        core,
        MemoryWorkshopStore::default(),
        AppEventProxy::Headless,
    );
    app.runtime
        .classic_mut()
        .set_viewport(glam::Vec2::new(1280.0, 480.0));
    app.runtime.start_new_workshop(47).unwrap();
    submit(
        &mut app,
        (0..64)
            .map(|index| CreatorOpV1::CreateSystem {
                local: BatchLocalId(index),
                name: ObjectName::new("S".repeat(64)).unwrap(),
                position: GalaxyPointV1::new(i64::from(index), 0).unwrap(),
            })
            .collect(),
    );
    let system = *app
        .runtime
        .workshop_snapshot()
        .unwrap()
        .state
        .systems
        .keys()
        .next()
        .unwrap();
    submit(
        &mut app,
        (0..128)
            .map(|index| CreatorOpV1::CreateStar {
                local: BatchLocalId(index),
                system: ObjectRefV1::Existing(system),
                name: ObjectName::new("T".repeat(64)).unwrap(),
                archetype_id: CatalogId::new("yellow-dwarf").unwrap(),
            })
            .collect(),
    );
    let star = *app
        .runtime
        .workshop_snapshot()
        .unwrap()
        .state
        .stars
        .keys()
        .next()
        .unwrap();
    submit(
        &mut app,
        (0..512)
            .map(|index| CreatorOpV1::CreateWorld {
                local: BatchLocalId(index),
                system: ObjectRefV1::Existing(system),
                primary: ObjectRefV1::Existing(star),
                name: ObjectName::new("W".repeat(64)).unwrap(),
                archetype_id: CatalogId::new("rocky-world").unwrap(),
                orbit_radius_milli_au: 1000,
                orbit_period_ticks: 100,
                phase_millidegrees: 0,
            })
            .collect(),
    );
    app.build_workshop_frame();
    app
}

#[test]
fn rendered_settings_scale_action_routes_full_cycle_and_persists_next_frame() {
    let preference_store = RecordingPreferencesStore::default();
    let persisted = Rc::clone(&preference_store.slot);
    let core = AppCore::new_with_preferences(
        ScenarioDraft::factory_default().validated().unwrap(),
        MemoryScenarioStore::default(),
        preference_store,
    );
    let mut app = App::with_event_proxy(
        core,
        MemoryWorkshopStore::default(),
        AppEventProxy::Headless,
    );
    app.runtime
        .classic_mut()
        .set_viewport(glam::Vec2::new(723.0, 802.0));

    app.build_shell_frame();
    let settings_action = SemanticActionId::new("shell.menu.settings");
    assert!(
        app.platform_ui
            .as_ref()
            .is_some_and(|frame| frame.action(&settings_action).is_some())
    );
    app.activate_platform_action_id(&settings_action, InputModality::Pointer);
    assert_eq!(app.runtime.screen(), ClientScreen::Settings);
    app.build_shell_frame();

    let scale_action = SemanticActionId::new("shell.settings.scale");
    assert!(app.ui_focus.request_focus(&scale_action));
    for (expected, modality) in [
        (
            crate::preferences::UiScale::Percent115,
            InputModality::Pointer,
        ),
        (
            crate::preferences::UiScale::Percent130,
            InputModality::Keyboard,
        ),
        (
            crate::preferences::UiScale::Percent85,
            InputModality::Pointer,
        ),
        (
            crate::preferences::UiScale::Percent100,
            InputModality::Keyboard,
        ),
    ] {
        let rendered_action = app
            .platform_ui
            .as_ref()
            .and_then(|frame| {
                frame
                    .controls
                    .iter()
                    .find(|control| control.action_id == scale_action)
            })
            .map(|control| control.action_id.clone())
            .expect("the rendered Settings frame must expose the scale action");
        app.activate_platform_action_id(&rendered_action, modality);
        assert_eq!(app.runtime.classic().preferences().ui_scale, expected);
        let saved = persisted
            .borrow()
            .clone()
            .expect("scale activation must persist preferences");
        assert_eq!(
            crate::preferences::decode(&saved).unwrap().ui_scale,
            expected
        );

        app.build_shell_frame();
        let frame = app.platform_ui.as_ref().unwrap();
        assert_eq!(frame.layout.ui_scale, expected.factor());
        assert_eq!(app.ui_focus.focused(), Some(&scale_action));
        let control = frame
            .controls
            .iter()
            .find(|control| control.action_id == scale_action)
            .unwrap();
        assert_eq!(control.label, format!("UI scale: {}%", expected.percent()));
        assert_eq!(frame.action(&scale_action), Some(&control.action));
    }
}

pub(super) fn open(app: &mut TestApp, creator: bool) {
    let system = *app
        .runtime
        .workshop_snapshot()
        .unwrap()
        .state
        .systems
        .keys()
        .next()
        .unwrap();
    app.apply_workshop_intent(if creator {
        WorkshopUiIntent::OpenCreatorForm {
            tool: CreatorTool::CreateWorld,
            subject: None,
        }
    } else {
        WorkshopUiIntent::OpenRemovalConfirmation(system)
    });
    app.build_workshop_frame();
}

#[test]
fn app_modal_focus_consumes_frame_order_and_preserves_restore_target() {
    let mut app = capacity_app();
    let restore = app
        .workshop_ui
        .as_ref()
        .unwrap()
        .menu_control
        .action_id
        .clone();
    for creator in [true, false] {
        assert!(app.ui_focus.request_focus(&restore));
        open(&mut app, creator);
        let order = app.platform_ui.as_ref().unwrap().focus_order();
        assert_eq!(app.ui_focus.active_order(), order);
        assert!(!app.ui_focus.request_focus(&restore));
        for action in &order {
            assert!(app.ui_focus.request_focus(action), "rejected {action}");
            app.build_workshop_frame();
            assert_eq!(app.ui_focus.focused(), Some(action));
            let control = app
                .platform_ui
                .as_ref()
                .unwrap()
                .controls
                .iter()
                .find(|control| control.action_id == *action)
                .unwrap_or_else(|| panic!("focused action {action} was not revealed"));
            if control.enabled {
                assert!(app.platform_ui.as_ref().unwrap().action(action).is_some());
            }
        }
        assert!(app.ui_focus.request_focus(&order[0]));
        for expected in order.iter().skip(1).chain(order.first()) {
            assert_eq!(app.ui_focus.move_next(), Some(expected));
            app.build_workshop_frame();
        }
        app.apply_workshop_intent(if creator {
            WorkshopUiIntent::CloseCreatorForm
        } else {
            WorkshopUiIntent::CloseRemovalConfirmation
        });
        app.build_workshop_frame();
        assert_eq!(app.ui_focus.focused(), Some(&restore));
    }
}

#[test]
fn app_modal_paging_keeps_keyboard_and_pointer_pages_in_sync() {
    let mut app = capacity_app();
    for creator in [true, false] {
        let mut pages = Vec::new();
        for modality in [InputModality::Keyboard, InputModality::Pointer] {
            open(&mut app, creator);
            let initial_row = app.workshop_view.modal_row;
            let next = WorkshopViewAction::ScrollNext.action_id();
            if modality == InputModality::Keyboard {
                assert!(app.ui_focus.request_focus(&next));
            }
            app.activate_platform_action_id(&next, modality);
            app.build_workshop_frame();
            assert!(
                app.workshop_view.modal_row > initial_row,
                "page reverted for {modality:?}"
            );
            pages.push(app.platform_ui.as_ref().unwrap().visible_nodes.clone());
            app.apply_workshop_intent(if creator {
                WorkshopUiIntent::CloseCreatorForm
            } else {
                WorkshopUiIntent::CloseRemovalConfirmation
            });
            app.build_workshop_frame();
        }
        assert_eq!(pages[0], pages[1]);
    }
}

#[test]
fn app_creator_submit_resets_paging_before_same_tool_reopens() {
    let mut app = capacity_app();
    let world = *app
        .runtime
        .workshop_snapshot()
        .unwrap()
        .state
        .worlds
        .keys()
        .next()
        .unwrap();
    submit(
        &mut app,
        vec![CreatorOpV1::RemoveObject {
            target: ObjectRefV1::Existing(world),
        }],
    );
    assert_eq!(
        app.runtime.workshop_snapshot().unwrap().state.worlds.len(),
        511
    );
    open(&mut app, true);
    app.activate_platform_action_id(
        &WorkshopViewAction::ScrollNext.action_id(),
        InputModality::Pointer,
    );
    app.build_workshop_frame();
    assert!(app.workshop_view.modal_row > 0);
    app.activate_platform_action_id(
        &SemanticActionId::new("creator.submit"),
        InputModality::Keyboard,
    );
    assert!(app.active_creator.is_none());
    assert!(!app.ui_focus.modal_is_open());
    assert_eq!(app.workshop_view.modal_row, 0);
    app.runtime.update(std::time::Duration::ZERO);
    assert_eq!(
        app.runtime.workshop_snapshot().unwrap().state.worlds.len(),
        512
    );
    open(&mut app, true);
    assert_eq!(app.workshop_view.modal_row, 0);
}

#[test]
fn app_modal_close_and_same_context_reopen_starts_at_first_page() {
    let mut app = capacity_app();
    for render_closed in [false, true] {
        for creator in [true, false] {
            open(&mut app, creator);
            let first = app.platform_ui.as_ref().unwrap().visible_nodes.clone();
            // A footer focus must not force the body back to the first field.
            let cancel = SemanticActionId::new(if creator {
                "creator.cancel"
            } else {
                "remove.cancel"
            });
            assert!(
                app.ui_focus.request_focus(&cancel),
                "creator={creator}, active={:?}",
                app.ui_focus.active_order()
            );
            app.activate_platform_action_id(
                &WorkshopViewAction::ScrollNext.action_id(),
                InputModality::Pointer,
            );
            app.build_workshop_frame();
            assert_ne!(app.platform_ui.as_ref().unwrap().visible_nodes, first);
            app.apply_workshop_intent(if creator {
                WorkshopUiIntent::CloseCreatorForm
            } else {
                WorkshopUiIntent::CloseRemovalConfirmation
            });
            assert_eq!(app.workshop_view.modal_row, 0);
            // No synthetic view reveal or apply is inserted between close and reopen.
            if render_closed {
                app.build_workshop_frame();
            }
            open(&mut app, creator);
            assert_eq!(app.platform_ui.as_ref().unwrap().visible_nodes, first);
            app.apply_workshop_intent(if creator {
                WorkshopUiIntent::CloseCreatorForm
            } else {
                WorkshopUiIntent::CloseRemovalConfirmation
            });
            app.build_workshop_frame();
        }
    }
}

#[test]
fn app_frame_time_reveal_keeps_last_wide_outliner_action_materialized() {
    for scale in [
        crate::preferences::UiScale::Percent100,
        crate::preferences::UiScale::Percent130,
    ] {
        let mut app = capacity_app();
        app.runtime
            .classic_mut()
            .set_viewport(glam::Vec2::new(1920.0, 1080.0));
        app.runtime.classic_mut().open_settings();
        assert!(
            app.runtime
                .classic_mut()
                .handle_settings_action(crate::app::settings::SettingsAction::SetUiScale(scale))
        );
        app.runtime.classic_mut().close_settings();
        app.build_workshop_frame();

        let target = app
            .workshop_ui
            .as_ref()
            .unwrap()
            .outliner
            .last()
            .unwrap()
            .control
            .action_id
            .clone();
        assert!(app.ui_focus.request_focus(&target));
        for rebuild in 0..2 {
            app.build_workshop_frame();
            let frame = app.platform_ui.as_ref().unwrap();
            let control = frame
                .controls
                .iter()
                .find(|control| control.action_id == target)
                .unwrap_or_else(|| {
                    panic!("App frame-time reveal lost {target} at {scale:?} on rebuild {rebuild}")
                });
            assert!(frame.action(&target).is_some());
            assert_eq!(
                frame
                    .hit_test(control.bounds.center())
                    .map(|hit| &hit.action_id),
                Some(&target)
            );
        }
    }
}
