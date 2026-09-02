//! Routes winit input into the session and the single gesture owner.

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
};

use super::{App, AppMode, action_for_key, mouse_pointer, navigation_for_key};

impl<S: ScenarioStore, P: PreferencesStore> App<S, P> {
    pub(super) fn begin_pointer(
        &mut self,
        id: u64,
        position: Vec2,
        source: PointerSource,
        button: PointerButton,
    ) {
        if self.core.mode() == AppMode::EditingScenario {
            if button == PointerButton::Primary {
                let action = self
                    .core
                    .editor_mut()
                    .and_then(|editor| editor.activate_at(position));
                if let Some(action) = action {
                    self.apply_editor_action(action);
                }
            }
            return;
        }
        let ui = self.core.ui_frame();
        let control_hit = ui
            .controls
            .iter()
            .enumerate()
            .find(|(_, control)| control.enabled && control.bounds.contains(position))
            .map(|(index, control)| (index, control.action));
        let hit = if let Some((index, action)) = control_hit {
            self.core.focus_ui(index);
            InteractionHit::Hud(action)
        } else if ui.consumes_pointer(position) {
            return;
        } else if self.core.pick_scene_world(position, false).is_some() {
            self.core.clear_ui_focus();
            InteractionHit::World
        } else {
            self.core.clear_ui_focus();
            InteractionHit::EmptyScene
        };
        self.interaction.begin(id, position, source, button, hit);
    }

    pub(super) fn end_pointer(&mut self, id: u64, position: Vec2) {
        let hud_hit = self.core.ui_frame().hit_test(position);
        if let Some(action) = self.interaction.end(id, position, hud_hit) {
            self.apply_gesture_action(action);
        }
    }

    fn apply_gesture_action(&mut self, action: GestureAction) {
        match action {
            GestureAction::Hud(action) => {
                self.core.handle_ui_action(action);
            }
            GestureAction::SceneTap { position, source } => {
                self.core
                    .handle_scene_tap(position, source == PointerSource::Touch);
            }
            GestureAction::ImmediateLaunch { position } => {
                self.core.handle_scene_launch_pointer(position);
            }
            GestureAction::Orbit { delta } => {
                self.core
                    .camera_mut()
                    .orbit(-delta.x * 0.005, delta.y * 0.005);
            }
            GestureAction::Zoom { logical_delta } => self.core.zoom_camera(logical_delta),
        }
    }

    pub(super) fn update_pointer(&mut self, id: u64, position: Vec2) {
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
        match self.core.handle_editor_action(action) {
            Ok(()) => self.core.clear_recoverable_message(),
            Err(error) => self.core.set_recoverable_message(error.to_string()),
        }
    }

    fn handle_navigation(&mut self, navigation: NavigationAction) {
        match self.core.mode() {
            AppMode::EditingScenario => {
                let action = self
                    .core
                    .editor_mut()
                    .and_then(|editor| editor.navigate(navigation));
                if let Some(action) = action {
                    self.apply_editor_action(action);
                }
            }
            AppMode::Settings => match navigation {
                NavigationAction::TabBackward | NavigationAction::Left | NavigationAction::Up => {
                    self.core.focus_ui_next(true);
                }
                NavigationAction::TabForward | NavigationAction::Right | NavigationAction::Down => {
                    self.core.focus_ui_next(false)
                }
                NavigationAction::Activate => {
                    self.core.activate_focused_ui();
                }
                NavigationAction::Escape => self.core.close_settings(),
                _ => {}
            },
            AppMode::Playing => match navigation {
                NavigationAction::TabBackward => self.core.focus_ui_next(true),
                NavigationAction::TabForward => self.core.focus_ui_next(false),
                NavigationAction::Activate => {
                    self.core.activate_focused_ui();
                }
                _ => {}
            },
        }
    }

    pub(super) fn handle_key(&mut self, event: &winit::event::KeyEvent) {
        let pressed = event.state == ElementState::Pressed;
        if self.core.mode() == AppMode::EditingScenario {
            if !pressed {
                return;
            }
            if let Some(text) = event.text.as_deref()
                && !text.chars().any(char::is_control)
                && self
                    .core
                    .editor()
                    .is_some_and(crate::editor::EditorState::focused_accepts_text)
            {
                if let Some(editor) = self.core.editor_mut() {
                    editor.insert_focused_text(text);
                }
                return;
            }
            if let Some(navigation) = navigation_for_key(&event.logical_key, self.modifiers) {
                self.handle_navigation(navigation);
            }
            return;
        }
        if self.core.mode() == AppMode::Settings {
            if pressed
                && let Some(navigation) = navigation_for_key(&event.logical_key, self.modifiers)
            {
                self.handle_navigation(navigation);
            }
            return;
        }
        if pressed {
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
            ) && self.core.ui_focus_index().is_some()
            {
                self.handle_navigation(NavigationAction::Activate);
                return;
            }
        }
        if let Some(action) = action_for_key(&event.logical_key) {
            self.input.set_action(action, pressed);
            if pressed && self.input.take_pressed(action) {
                self.core.handle_action(action);
            }
        }
    }
}
