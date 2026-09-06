//! Routes winit input into Classic or the typed product-shell action path.

use glam::Vec2;
use winit::{
    event::{ElementState, MouseButton},
    keyboard::{Key, NamedKey},
};

use crate::{
    editor::EditorAction,
    engine::input::NavigationAction,
    preferences::store::PreferencesStore,
    presentation::{GestureAction, InteractionHit, PointerButton, PointerSource},
    scenario::store::ScenarioStore,
    ui::{
        accessibility::InputModality,
        creator::{CreatorFieldKind, edited_creator_text},
        platform::PlatformRect,
        workshop_view::WorkshopViewAction,
    },
    workshop::{
        session::{WorkshopAction, WorkshopSpeed},
        store::WorkshopStore,
    },
};

use super::{
    App, AppMode, action_for_key, client_runtime::ClientScreen, mouse_pointer, navigation_for_key,
};

impl<S: ScenarioStore, P: PreferencesStore, W: WorkshopStore> App<S, P, W> {
    pub(super) fn begin_pointer(
        &mut self,
        id: u64,
        position: Vec2,
        source: PointerSource,
        button: PointerButton,
    ) {
        if self.player_guide.is_some() || self.runtime.screen() != ClientScreen::ClassicSector {
            if button == PointerButton::Primary {
                let action = self
                    .platform_ui
                    .as_ref()
                    .and_then(|frame| frame.hit_test(position))
                    .map(|control| (control.action_id.clone(), control.action.clone()));
                if let Some((action_id, action)) = action {
                    self.ui_focus.request_focus(&action_id);
                    self.apply_platform_action(action, InputModality::Pointer);
                }
            }
            return;
        }

        if self.runtime.classic().mode() == AppMode::EditingScenario {
            if button == PointerButton::Primary {
                let action = self
                    .runtime
                    .classic_mut()
                    .editor_mut()
                    .and_then(|editor| editor.activate_at(position));
                if let Some(action) = action {
                    self.apply_editor_action(action);
                }
            }
            return;
        }
        let ui = self.runtime.classic().ui_frame();
        let control_hit = ui
            .controls
            .iter()
            .enumerate()
            .find(|(_, control)| control.enabled && control.bounds.contains(position))
            .map(|(index, control)| (index, control.action));
        let hit = if let Some((index, action)) = control_hit {
            self.runtime.classic_mut().focus_ui(index);
            InteractionHit::Hud(action)
        } else if ui.consumes_pointer(position) {
            return;
        } else if self
            .runtime
            .classic()
            .pick_scene_world(position, false)
            .is_some()
        {
            self.runtime.classic_mut().clear_ui_focus();
            InteractionHit::World
        } else {
            self.runtime.classic_mut().clear_ui_focus();
            InteractionHit::EmptyScene
        };
        self.interaction.begin(id, position, source, button, hit);
    }

    pub(super) fn end_pointer(&mut self, id: u64, position: Vec2) {
        if self.player_guide.is_some() || self.runtime.screen() != ClientScreen::ClassicSector {
            return;
        }
        let hud_hit = self.runtime.classic().ui_frame().hit_test(position);
        if let Some(action) = self.interaction.end(id, position, hud_hit) {
            self.apply_gesture_action(action);
        }
    }

    fn apply_gesture_action(&mut self, action: GestureAction) {
        match action {
            GestureAction::Hud(action) => {
                if action == crate::presentation::ui::UiAction::ToggleHelp {
                    self.apply_guide_action(crate::ui::guide::GuideAction::Open);
                } else {
                    self.runtime.classic_mut().handle_ui_action(action);
                }
            }
            GestureAction::SceneTap { position, source } => {
                self.runtime
                    .classic_mut()
                    .handle_scene_tap(position, source == PointerSource::Touch);
            }
            GestureAction::ImmediateLaunch { position } => {
                self.runtime
                    .classic_mut()
                    .handle_scene_launch_pointer(position);
            }
            GestureAction::Orbit { delta } => {
                self.runtime
                    .classic_mut()
                    .camera_mut()
                    .orbit(-delta.x * 0.005, delta.y * 0.005);
            }
            GestureAction::Zoom { logical_delta } => {
                self.runtime.classic_mut().zoom_camera(logical_delta);
            }
        }
    }

    pub(super) fn update_pointer(&mut self, id: u64, position: Vec2) {
        if self.player_guide.is_some() || self.runtime.screen() != ClientScreen::ClassicSector {
            return;
        }
        if let Some(action) = self.interaction.update(id, position) {
            self.apply_gesture_action(action);
        }
    }

    pub(super) fn handle_mouse_input(&mut self, state: ElementState, button: MouseButton) {
        let Some((id, button)) = mouse_pointer(button) else {
            return;
        };
        let position = self.input.cursor_logical;
        match state {
            ElementState::Pressed => {
                self.begin_pointer(id, position, PointerSource::Mouse, button);
            }
            ElementState::Released => self.end_pointer(id, position),
        }
    }

    fn apply_editor_action(&mut self, action: EditorAction) {
        match self.runtime.classic_mut().handle_editor_action(action) {
            Ok(()) => self.runtime.classic_mut().clear_recoverable_message(),
            Err(error) => self
                .runtime
                .classic_mut()
                .set_recoverable_message(error.to_string()),
        }
    }

    fn handle_navigation(&mut self, navigation: NavigationAction) {
        if self.player_guide.is_some() || self.runtime.screen() != ClientScreen::ClassicSector {
            match navigation {
                NavigationAction::TabBackward | NavigationAction::Left | NavigationAction::Up => {
                    self.ui_focus.move_previous();
                    self.reveal_workshop_focus();
                }
                NavigationAction::TabForward | NavigationAction::Right | NavigationAction::Down => {
                    self.ui_focus.move_next();
                    self.reveal_workshop_focus();
                }
                NavigationAction::PageUp => {
                    self.apply_workshop_view_action(WorkshopViewAction::ScrollPrevious)
                }
                NavigationAction::PageDown => {
                    self.apply_workshop_view_action(WorkshopViewAction::ScrollNext)
                }
                NavigationAction::Activate => {
                    if let Some(action) = self.ui_focus.focused().cloned() {
                        self.activate_platform_action_id(&action, InputModality::Keyboard);
                    }
                }
                NavigationAction::Escape => self.handle_shell_escape(),
                _ => {}
            }
            return;
        }

        match self.runtime.classic().mode() {
            AppMode::EditingScenario => {
                let action = self
                    .runtime
                    .classic_mut()
                    .editor_mut()
                    .and_then(|editor| editor.navigate(navigation));
                if let Some(action) = action {
                    self.apply_editor_action(action);
                }
            }
            AppMode::Settings => match navigation {
                NavigationAction::TabBackward | NavigationAction::Left | NavigationAction::Up => {
                    self.runtime.classic_mut().focus_ui_next(true);
                }
                NavigationAction::TabForward | NavigationAction::Right | NavigationAction::Down => {
                    self.runtime.classic_mut().focus_ui_next(false)
                }
                NavigationAction::Activate => {
                    self.activate_classic_control();
                }
                NavigationAction::Escape => self.runtime.classic_mut().close_settings(),
                _ => {}
            },
            AppMode::Playing => match navigation {
                NavigationAction::TabBackward => self.runtime.classic_mut().focus_ui_next(true),
                NavigationAction::TabForward => self.runtime.classic_mut().focus_ui_next(false),
                NavigationAction::Activate => {
                    self.activate_classic_control();
                }
                _ => {}
            },
        }
    }

    fn handle_shell_escape(&mut self) {
        if self.player_guide.is_some() {
            self.apply_guide_action(crate::ui::guide::GuideAction::Close);
            return;
        }
        if self.cancel_durable_exit() {
            return;
        }
        if self.active_creator.take().is_some() {
            self.close_workshop_modal_focus();
            return;
        }
        if self.pending_removal.take().is_some() {
            self.close_workshop_modal_focus();
            return;
        }
        if self.runtime.screen() == ClientScreen::GalaxyWorkshop
            && self.workshop_view.open_drawer.is_some()
        {
            self.apply_workshop_view_action(WorkshopViewAction::CloseDrawer);
            return;
        }
        if self.runtime.credits_visible() {
            self.runtime.dismiss_credits();
            return;
        }
        match self.runtime.screen() {
            ClientScreen::Settings => {
                self.runtime.close_settings();
                self.runtime.classic_mut().close_settings();
            }
            ClientScreen::RecoverableError => self.runtime.dismiss_recovery(),
            ClientScreen::GalaxyWorkshop => {
                self.apply_workshop_intent(crate::ui::workshop::WorkshopUiIntent::ReturnToMainMenu);
            }
            ClientScreen::MainMenu | ClientScreen::ClassicSector | ClientScreen::Loading => {}
        }
    }

    pub(super) fn handle_key(&mut self, event: &winit::event::KeyEvent) {
        let pressed = event.state == ElementState::Pressed;
        if pressed && !event.repeat && matches!(event.logical_key, Key::Named(NamedKey::F1)) {
            self.apply_guide_action(if self.player_guide.is_some() {
                crate::ui::guide::GuideAction::Close
            } else {
                crate::ui::guide::GuideAction::Open
            });
            return;
        }
        if self.player_guide.is_some() {
            if pressed
                && let Some(navigation) = navigation_for_key(&event.logical_key, self.modifiers)
            {
                self.handle_navigation(navigation);
            }
            return;
        }
        if self.runtime.screen() != ClientScreen::ClassicSector {
            if !pressed {
                return;
            }
            if self.runtime.screen() == ClientScreen::GalaxyWorkshop
                && self.handle_creator_field_key(event)
            {
                return;
            }
            if let Some(navigation) = navigation_for_key(&event.logical_key, self.modifiers) {
                self.handle_navigation(navigation);
                return;
            }
            if self.runtime.screen() == ClientScreen::GalaxyWorkshop
                && let Key::Character(value) = &event.logical_key
            {
                match value.to_ascii_lowercase().as_str() {
                    "p" => {
                        let action = if self
                            .runtime
                            .workshop_snapshot()
                            .is_some_and(|snapshot| snapshot.speed == WorkshopSpeed::Paused)
                        {
                            WorkshopAction::Resume(WorkshopSpeed::One)
                        } else {
                            WorkshopAction::Pause
                        };
                        let _ = self.runtime.enqueue_workshop_action(action);
                    }
                    "s" if self.modifiers.super_key() || self.modifiers.control_key() => {
                        let _ = self
                            .runtime
                            .enqueue_workshop_action(WorkshopAction::RequestSave);
                    }
                    _ => {}
                }
            }
            return;
        }

        if self.runtime.classic().mode() == AppMode::EditingScenario {
            if !pressed {
                return;
            }
            if let Some(text) = event.text.as_deref()
                && !text.chars().any(char::is_control)
                && self
                    .runtime
                    .classic()
                    .editor()
                    .is_some_and(crate::editor::EditorState::focused_accepts_text)
            {
                if let Some(editor) = self.runtime.classic_mut().editor_mut() {
                    editor.insert_focused_text(text);
                }
                return;
            }
            if let Some(navigation) = navigation_for_key(&event.logical_key, self.modifiers) {
                self.handle_navigation(navigation);
            }
            return;
        }
        if self.runtime.classic().mode() == AppMode::Settings {
            if pressed
                && let Some(navigation) = navigation_for_key(&event.logical_key, self.modifiers)
            {
                self.handle_navigation(navigation);
            }
            return;
        }
        if pressed {
            if !event.repeat
                && matches!(&event.logical_key, Key::Character(value) if value.eq_ignore_ascii_case("h"))
            {
                self.apply_guide_action(crate::ui::guide::GuideAction::Open);
                return;
            }
            if matches!(event.logical_key, Key::Named(NamedKey::Tab)) {
                self.handle_navigation(if self.modifiers.shift_key() {
                    NavigationAction::TabBackward
                } else {
                    NavigationAction::TabForward
                });
                return;
            }
            if matches!(
                event.logical_key,
                Key::Named(NamedKey::Enter | NamedKey::Space)
            ) && self.runtime.classic().ui_focus_index().is_some()
            {
                self.handle_navigation(NavigationAction::Activate);
                return;
            }
        }
        if let Some(action) = action_for_key(&event.logical_key) {
            self.input.set_action(action, pressed);
            if pressed && self.input.take_pressed(action) {
                self.runtime.classic_mut().handle_action(action);
            }
        }
    }

    fn activate_classic_control(&mut self) {
        let frame = self.runtime.classic().ui_frame();
        let is_help = self
            .runtime
            .classic()
            .ui_focus_index()
            .and_then(|index| frame.controls.get(index))
            .is_some_and(|control| control.action == crate::presentation::ui::UiAction::ToggleHelp);
        if is_help {
            self.apply_guide_action(crate::ui::guide::GuideAction::Open);
        } else {
            self.runtime.classic_mut().activate_focused_ui();
        }
    }

    fn handle_creator_field_key(&mut self, event: &winit::event::KeyEvent) -> bool {
        if self.modifiers.control_key() || self.modifiers.super_key() || self.modifiers.alt_key() {
            return false;
        }
        let Some(field_id) = self
            .ui_focus
            .focused()
            .and_then(|action| action.as_str().strip_prefix("creator.field."))
            .map(str::to_owned)
        else {
            return false;
        };
        let Some(kind) = self
            .active_creator
            .as_ref()
            .and_then(|editor| editor.draft.as_ref())
            .and_then(|draft| draft.field(&field_id))
            .map(|field| field.kind)
        else {
            return false;
        };
        if kind == CreatorFieldKind::Choice {
            return false;
        }

        let replacement = self
            .active_creator
            .as_ref()
            .and_then(|editor| editor.editing_field.as_deref())
            != Some(&field_id);
        let current = self
            .active_creator
            .as_ref()
            .and_then(|editor| editor.draft.as_ref())
            .and_then(|draft| draft.field(&field_id))
            .map_or("", |field| field.value.as_str());
        let next = edited_creator_text(
            kind,
            current,
            event.text.as_deref(),
            matches!(event.logical_key, Key::Named(NamedKey::Backspace)),
            replacement,
        );
        let Some(next) = next else {
            return false;
        };
        if let Some(editor) = &mut self.active_creator
            && editor
                .draft
                .as_mut()
                .is_some_and(|draft| draft.set_text(&field_id, next).is_ok())
        {
            editor.editing_field = Some(field_id);
        }
        true
    }

    fn reveal_workshop_focus(&mut self) {
        if self.runtime.screen() != ClientScreen::GalaxyWorkshop {
            return;
        }
        let (Some(model), Some(frame), Some(focused)) = (
            self.workshop_ui.as_ref(),
            self.platform_ui.as_ref(),
            self.ui_focus.focused(),
        ) else {
            return;
        };
        self.workshop_view
            .reveal_action(model, &frame.layout, focused);
    }

    pub(super) fn apply_workshop_view_action(&mut self, action: WorkshopViewAction) {
        if self.runtime.screen() != ClientScreen::GalaxyWorkshop {
            return;
        }
        let (Some(model), Some(frame)) = (self.workshop_ui.as_ref(), self.platform_ui.as_ref())
        else {
            return;
        };
        self.workshop_view.apply(action, model, &frame.layout);
        if frame.modal.is_some() {
            // All input routes must keep field reveal from undoing the requested page.
            self.ui_focus.request_focus(&action.action_id());
        }
    }

    pub(super) fn scroll_workshop_view(&mut self, logical_delta_y: f32) {
        if self.runtime.screen() != ClientScreen::GalaxyWorkshop || logical_delta_y == 0.0 {
            return;
        }
        let scroll_target = self.platform_ui.as_ref().and_then(|frame| {
            workshop_scroll_target(
                frame.drawer,
                frame.layout.right_panel,
                self.input.cursor_logical,
            )
        });
        if let Some(target) = scroll_target {
            self.apply_workshop_view_action(match (target, logical_delta_y < 0.0) {
                (WorkshopScrollTarget::Drawer, true) => WorkshopViewAction::ScrollNext,
                (WorkshopScrollTarget::Drawer, false) => WorkshopViewAction::ScrollPrevious,
                (WorkshopScrollTarget::Inspector, true) => WorkshopViewAction::ScrollInspectorNext,
                (WorkshopScrollTarget::Inspector, false) => {
                    WorkshopViewAction::ScrollInspectorPrevious
                }
            });
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum WorkshopScrollTarget {
    Drawer,
    Inspector,
}

fn workshop_scroll_target(
    drawer: Option<PlatformRect>,
    docked_inspector: Option<PlatformRect>,
    point: Vec2,
) -> Option<WorkshopScrollTarget> {
    if drawer.is_some_and(|region| region.contains(point)) {
        Some(WorkshopScrollTarget::Drawer)
    } else if docked_inspector.is_some_and(|region| region.contains(point)) {
        Some(WorkshopScrollTarget::Inspector)
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(not(target_arch = "wasm32"))]
    #[test]
    fn modal_page_keys_preserve_requested_page_from_initial_field_focus() {
        use crate::app::modal_lifecycle_tests::{capacity_app, open};
        for scale in [100, 130] {
            let mut app = capacity_app();
            app.runtime.classic_mut().open_settings();
            assert!(app.runtime.classic_mut().handle_settings_action(
                crate::app::settings::SettingsAction::SetUiScale(
                    crate::preferences::UiScale::try_from(scale).unwrap()
                )
            ));
            app.runtime.classic_mut().close_settings();
            open(&mut app, true);
            assert_eq!(
                app.platform_ui.as_ref().unwrap().layout.ui_scale,
                f32::from(scale) / 100.0
            );
            assert!(
                app.ui_focus
                    .focused()
                    .unwrap()
                    .as_str()
                    .starts_with("creator.field.")
            );
            let mut first = app.platform_ui.as_ref().unwrap().visible_nodes.clone();
            // Page content returns unchanged; focus deliberately transfers to Previous.
            for record in &mut first {
                record.state.focused = false;
            }
            app.handle_navigation(NavigationAction::PageDown);
            app.build_workshop_frame();
            assert!(
                app.workshop_view.modal_row > 0,
                "PageDown reverted at {scale}%"
            );
            let next = app.platform_ui.as_ref().unwrap().visible_nodes.clone();
            assert_ne!(next, first);
            assert_eq!(
                app.ui_focus.focused(),
                Some(&WorkshopViewAction::ScrollNext.action_id())
            );
            app.handle_navigation(NavigationAction::PageUp);
            app.build_workshop_frame();
            assert_eq!(app.workshop_view.modal_row, 0);
            let mut previous = app.platform_ui.as_ref().unwrap().visible_nodes.clone();
            for record in &mut previous {
                record.state.focused = false;
            }
            assert_eq!(previous, first);
            assert_eq!(
                app.ui_focus.focused(),
                Some(&WorkshopViewAction::ScrollPrevious.action_id())
            );
            app.activate_platform_action_id(
                &WorkshopViewAction::ScrollNext.action_id(),
                InputModality::Pointer,
            );
            app.build_workshop_frame();
            assert_eq!(app.platform_ui.as_ref().unwrap().visible_nodes, next);
        }
    }

    #[cfg(not(target_arch = "wasm32"))]
    #[test]
    fn escape_clears_each_modal_paging_scope_before_rebuild_or_reopen() {
        use crate::app::modal_lifecycle_tests::{capacity_app, open};
        let mut app = capacity_app();
        for creator in [true, false] {
            let restore = app
                .workshop_ui
                .as_ref()
                .unwrap()
                .menu_control
                .action_id
                .clone();
            assert!(app.ui_focus.request_focus(&restore));
            let before = app.workshop_view.clone();
            open(&mut app, creator);
            let first = app.platform_ui.as_ref().unwrap().visible_nodes.clone();
            app.activate_platform_action_id(
                &WorkshopViewAction::ScrollNext.action_id(),
                InputModality::Pointer,
            );
            app.build_workshop_frame();
            assert!(app.workshop_view.modal_row > 0);
            app.handle_shell_escape();
            assert!(app.active_creator.is_none());
            assert!(app.pending_removal.is_none());
            assert!(!app.ui_focus.modal_is_open());
            assert_eq!(app.ui_focus.focused(), Some(&restore));
            assert_eq!(
                app.workshop_view, before,
                "Escape left paging state at creator={creator}"
            );
            open(&mut app, creator);
            assert_eq!(app.platform_ui.as_ref().unwrap().visible_nodes, first);
            app.handle_navigation(NavigationAction::Escape);
            assert_eq!(app.workshop_view, before);
            app.build_workshop_frame();
        }
    }

    #[test]
    fn workshop_wheel_targets_drawer_or_docked_inspector_but_not_scene_canvas() {
        let drawer = PlatformRect::from_xywh(8.0, 200.0, 400.0, 480.0);
        let inspector = PlatformRect::from_xywh(1_136.0, 56.0, 304.0, 786.0);

        assert_eq!(
            workshop_scroll_target(Some(drawer), Some(inspector), drawer.center()),
            Some(WorkshopScrollTarget::Drawer)
        );
        assert_eq!(
            workshop_scroll_target(None, Some(inspector), inspector.center()),
            Some(WorkshopScrollTarget::Inspector)
        );
        assert_eq!(
            workshop_scroll_target(None, Some(inspector), Vec2::new(720.0, 450.0)),
            None
        );
        assert_eq!(
            workshop_scroll_target(None, None, Vec2::new(320.0, 400.0)),
            None
        );
    }
}
