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
        accessibility::{InputModality, SemanticActionId},
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
        // A text edit may have arrived from the DOM or accessibility bridge
        // before this canvas click. Dispatch it first; a context replacement
        // also retires the pointer target drawn by the previous frame.
        let owner = self.semantic_input_owner();
        self.drain_semantic_actions();
        if !owner.matches(&self.semantic_input_owner()) {
            return;
        }
        if self.player_guide.is_some() || self.runtime.screen() != ClientScreen::ClassicSector {
            if button == PointerButton::Primary {
                let action = self
                    .platform_ui
                    .as_ref()
                    .and_then(|frame| frame.hit_test(position))
                    .map(|control| (control.action_id.clone(), control.action.clone()));
                if let Some((action_id, action)) = action {
                    self.ui_focus.request_focus(&action_id);
                    let living_field = action_id.as_str().starts_with("sandbox.creator.field.");
                    self.apply_platform_action(action, InputModality::Pointer);
                    #[cfg(not(target_arch = "wasm32"))]
                    if living_field {
                        self.place_living_pointer_caret(
                            id,
                            position,
                            self.modifiers.shift_key(),
                            true,
                        );
                    }
                    #[cfg(target_arch = "wasm32")]
                    let _ = living_field;
                } else if self.player_guide.is_none()
                    && self.runtime.screen() == ClientScreen::GalaxySandbox
                    && self.runtime.sandbox_creator_scene_open()
                    && let Some(frame) = self.platform_ui.as_ref()
                    && frame.layout.canvas.contains(position)
                    && !frame.modal.is_some_and(|panel| panel.contains(position))
                    && let Some(entity) = self
                        .runtime
                        .sandbox_scene(frame.layout.canvas)
                        .and_then(|scene| scene.hit_entity(position))
                {
                    self.apply_platform_action(
                        crate::ui::platform::PlatformUiAction::Sandbox(
                            crate::sandbox_session::SandboxUiAction::SnapCreatorParent(entity),
                        ),
                        InputModality::Pointer,
                    );
                } else if self.player_guide.is_none()
                    && self.runtime.screen() == ClientScreen::GalaxySandbox
                    && !self.runtime.sandbox_creator_open()
                    && let Some(frame) = self.platform_ui.as_ref()
                    && frame.layout.canvas.contains(position)
                    && !frame.drawer.is_some_and(|panel| panel.contains(position))
                {
                    let orbit = self.modifiers.alt_key() || self.runtime.sandbox_camera_tool();
                    self.living_scene_focused = true;
                    let second_touch =
                        source == PointerSource::Touch && !self.living_camera_touches.is_empty();
                    if source == PointerSource::Touch {
                        if !self.living_camera_touches.begin(id, position) {
                            return;
                        }
                        self.living_camera_drag = None;
                    } else {
                        self.living_camera_drag = Some((id, position, orbit));
                    }
                    if !orbit
                        && !second_touch
                        && let Some(entity) = self
                            .runtime
                            .sandbox_scene(frame.layout.canvas)
                            .and_then(|scene| scene.hit_entity(position))
                    {
                        let now = super::Instant::now();
                        let double_click =
                            self.living_scene_click
                                .is_some_and(|(time, point, previous)| {
                                    previous == entity
                                        && now.saturating_duration_since(time)
                                            <= std::time::Duration::from_millis(350)
                                        && point.distance(position) <= 8.0
                                });
                        self.living_scene_click =
                            (!double_click).then_some((now, position, entity));
                        self.apply_platform_action(
                            crate::ui::platform::PlatformUiAction::Sandbox(
                                crate::sandbox_session::SandboxUiAction::SelectEntity(entity),
                            ),
                            InputModality::Pointer,
                        );
                        self.living_scene_focused = true;
                        if double_click {
                            self.runtime.apply_sandbox_action(
                                crate::sandbox_session::SandboxUiAction::FocusSelection,
                            );
                        }
                    } else {
                        self.living_scene_click = None;
                    }
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

    #[cfg(not(target_arch = "wasm32"))]
    fn place_living_pointer_caret(&mut self, id: u64, position: Vec2, extend: bool, begin: bool) {
        let Some(action) = self.ui_focus.focused().cloned() else {
            return;
        };
        let Some(key) = action.as_str().strip_prefix("sandbox.creator.field.") else {
            return;
        };
        let Some(scope) = self.runtime.living_presentation_scope() else {
            return;
        };
        self.synchronize_living_field_editor();
        let Some((editor_key, editor)) = self.living_text_edit.as_mut() else {
            return;
        };
        if editor_key != key {
            return;
        }
        let Some(mut frame) = self.platform_ui.clone() else {
            return;
        };
        frame.reconcile_focused(Some(&action));
        let Ok(Some(offset)) = crate::ui::platform::platform_editor_caret_at(
            &frame,
            &self.ui_metrics,
            &action,
            editor,
            position,
        ) else {
            return;
        };
        if editor.place_caret(offset, extend) && begin {
            self.living_text_drag = Some((id, key.to_owned(), scope));
        }
    }

    pub(super) fn end_pointer(&mut self, id: u64, position: Vec2) {
        #[cfg(not(target_arch = "wasm32"))]
        if self
            .living_text_drag
            .as_ref()
            .is_some_and(|(active, _, _)| *active == id)
        {
            self.update_pointer(id, position);
            self.living_text_drag = None;
            return;
        }
        if self.living_camera_touches.contains(id) {
            self.update_pointer(id, position);
            self.living_camera_touches.end(id);
            return;
        }
        if self
            .living_camera_drag
            .is_some_and(|(active, _, _)| active == id)
        {
            self.update_pointer(id, position);
            self.living_camera_drag = None;
        }
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
        #[cfg(not(target_arch = "wasm32"))]
        if let Some((active, key, scope)) = self.living_text_drag.as_ref()
            && *active == id
        {
            let current_scope = self.runtime.living_presentation_scope();
            let valid = self.player_guide.is_none()
                && self.runtime.screen() == ClientScreen::GalaxySandbox
                && current_scope
                    .as_ref()
                    .is_some_and(|current| std::rc::Rc::ptr_eq(current, scope))
                && self.ui_focus.focused().is_some_and(|action| {
                    action.as_str().strip_prefix("sandbox.creator.field.") == Some(key.as_str())
                });
            if valid {
                self.place_living_pointer_caret(id, position, true, false);
            } else {
                self.living_text_drag = None;
            }
            return;
        }
        if self.living_camera_touches.contains(id) {
            if self.player_guide.is_none()
                && self.runtime.screen() == ClientScreen::GalaxySandbox
                && !self.runtime.sandbox_creator_open()
                && let Some(bounds) = self.platform_ui.as_ref().map(|frame| frame.layout.canvas)
                && let Some(gesture) = self.living_camera_touches.update(id, position)
            {
                self.runtime
                    .pan_sandbox_camera(gesture.pan, self.runtime.sandbox_camera_tool());
                if let Some((factor, anchor)) = gesture.zoom {
                    self.runtime.zoom_sandbox_camera(factor, anchor, bounds);
                }
            } else {
                self.living_camera_touches.clear();
            }
            return;
        }
        if let Some((active, previous, orbit)) = self.living_camera_drag
            && active == id
        {
            if self.player_guide.is_none()
                && self.runtime.screen() == ClientScreen::GalaxySandbox
                && !self.runtime.sandbox_creator_open()
            {
                self.runtime.pan_sandbox_camera(position - previous, orbit);
                self.living_camera_drag = Some((id, position, orbit));
            } else {
                self.living_camera_drag = None;
            }
            return;
        }
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
        #[cfg(not(target_arch = "wasm32"))]
        self.living_clipboard.cancel();
        if self.player_guide.is_none()
            && self.runtime.screen() == ClientScreen::GalaxySandbox
            && matches!(navigation, NavigationAction::Left | NavigationAction::Right)
            && let Some(action) = self.ui_focus.focused()
            && action.as_str().starts_with("sandbox.navigator.entity.")
            && let Some(crate::ui::platform::PlatformUiAction::Sandbox(
                crate::sandbox_session::SandboxUiAction::SelectEntity(selection),
            )) = self
                .platform_ui
                .as_ref()
                .and_then(|frame| frame.action(action))
        {
            let selection = *selection;
            let action = if navigation == NavigationAction::Left {
                crate::sandbox_session::SandboxUiAction::NavigatorLeft(selection)
            } else {
                crate::sandbox_session::SandboxUiAction::NavigatorRight(selection)
            };
            self.apply_platform_action(
                crate::ui::platform::PlatformUiAction::Sandbox(action),
                InputModality::Keyboard,
            );
            return;
        }
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
                    if self.runtime.screen() == ClientScreen::GalaxySandbox {
                        self.runtime.apply_sandbox_action(
                            crate::sandbox_session::SandboxUiAction::InspectorPage(true),
                        );
                    } else {
                        self.apply_workshop_view_action(WorkshopViewAction::ScrollPrevious);
                    }
                }
                NavigationAction::PageDown => {
                    if self.runtime.screen() == ClientScreen::GalaxySandbox {
                        self.runtime.apply_sandbox_action(
                            crate::sandbox_session::SandboxUiAction::InspectorPage(false),
                        );
                    } else {
                        self.apply_workshop_view_action(WorkshopViewAction::ScrollNext);
                    }
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
        if self.runtime.screen() == ClientScreen::GalaxySandbox
            && self.runtime.sandbox_creator_open()
        {
            self.runtime
                .apply_sandbox_action(crate::sandbox_session::SandboxUiAction::CreatorCancel);
            self.living_text_edit = None;
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
        if self.library_confirmation.is_some() {
            self.close_library_confirmation();
            return;
        }
        if self.runtime.screen() == ClientScreen::GalaxyWorkshop
            && self.workshop_view.open_drawer.is_some()
        {
            self.apply_workshop_view_action(WorkshopViewAction::CloseDrawer);
            return;
        }
        // Through Back's own path, so focus returns to the sheet's opener.
        if self.runtime.screen() == ClientScreen::Library && self.library_view.sheet.is_some() {
            self.apply_library_view_action(crate::ui::platform::LibraryViewAction::CloseSheet);
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
            ClientScreen::Library => self.runtime.close_library(),
            ClientScreen::RecoverableError => self.runtime.dismiss_recovery(),
            ClientScreen::GalaxyWorkshop => {
                self.apply_workshop_intent(crate::ui::workshop::WorkshopUiIntent::ReturnToMainMenu);
            }
            ClientScreen::GalaxySandbox => self.runtime.close_sandbox(),
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
            if self.runtime.screen() == ClientScreen::GalaxySandbox
                && self.handle_living_field_key(event)
            {
                return;
            }
            if self.runtime.screen() == ClientScreen::GalaxyWorkshop
                && self.handle_creator_field_key(event)
            {
                return;
            }
            if self.runtime.screen() == ClientScreen::Library
                && self.handle_library_rename_key(event)
            {
                return;
            }
            if self.runtime.screen() == ClientScreen::GalaxySandbox
                && !self.runtime.sandbox_creator_open()
                && !event.repeat
                && self.modifiers.is_empty()
            {
                if self.living_scene_focused
                    && matches!(event.logical_key, Key::Named(NamedKey::Home))
                {
                    self.runtime
                        .apply_sandbox_action(crate::sandbox_session::SandboxUiAction::FitScope);
                    return;
                }
                if self.living_scene_focused
                    && matches!(&event.logical_key, Key::Character(value) if value.eq_ignore_ascii_case("f"))
                {
                    self.runtime.apply_sandbox_action(
                        crate::sandbox_session::SandboxUiAction::FocusSelection,
                    );
                    return;
                }
                if matches!(&event.logical_key, Key::Character(value) if value == ".") {
                    self.runtime
                        .apply_sandbox_action(crate::sandbox_session::SandboxUiAction::Step);
                    return;
                }
                if matches!(&event.logical_key, Key::Character(value) if value.eq_ignore_ascii_case("p"))
                {
                    self.runtime
                        .apply_sandbox_action(crate::sandbox_session::SandboxUiAction::TogglePause);
                    return;
                }
            }
            if let Some(navigation) = navigation_for_key(&event.logical_key, self.modifiers) {
                self.living_scene_focused = false;
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

    fn handle_living_field_key(&mut self, event: &winit::event::KeyEvent) -> bool {
        let Some(key) = self
            .ui_focus
            .focused()
            .and_then(|action| action.as_str().strip_prefix("sandbox.creator.field."))
            .map(str::to_owned)
        else {
            return false;
        };
        let Some(value) = self.runtime.sandbox_creator_field(&key).map(str::to_owned) else {
            return false;
        };
        if matches!(
            event.logical_key,
            Key::Named(NamedKey::Tab | NamedKey::Escape | NamedKey::Enter)
        ) {
            return false;
        }
        if self
            .living_text_edit
            .as_ref()
            .is_none_or(|(old_key, edit)| old_key != &key || edit.value() != value)
        {
            self.living_text_edit = Some((key.clone(), crate::ui::text_edit::TextEdit::new(value)));
        }
        let edit = &mut self
            .living_text_edit
            .as_mut()
            .expect("field editor installed")
            .1;
        let command = self.modifiers.control_key() || self.modifiers.super_key();
        if command
            && matches!(&event.logical_key, Key::Character(text) if text.eq_ignore_ascii_case("a"))
        {
            edit.select_all();
            return true;
        }
        #[cfg(not(target_arch = "wasm32"))]
        if command && let Key::Character(text) = &event.logical_key {
            use crate::ui::clipboard::ClipboardOperation;
            let operation = match text.to_ascii_lowercase().as_str() {
                "c" => Some(ClipboardOperation::Copy),
                "x" => Some(ClipboardOperation::Cut),
                "v" => Some(ClipboardOperation::Paste),
                _ => None,
            };
            if let Some(operation) = operation {
                if let Some(scope) = self.runtime.living_presentation_scope()
                    && let Err(error) =
                        self.living_clipboard
                            .request(scope, key, edit.clone(), operation)
                {
                    self.runtime.report_living_presentation_failure(format!(
                        "Clipboard operation failed: {error}"
                    ));
                }
                return true;
            }
        }
        // Platform paste/IME/semantic value updates insert their complete bounded text.
        // Other modified keys remain typing-safe and do not activate transport.
        let modified_edit = matches!(
            event.logical_key,
            Key::Named(
                NamedKey::ArrowLeft
                    | NamedKey::ArrowRight
                    | NamedKey::Home
                    | NamedKey::End
                    | NamedKey::Backspace
                    | NamedKey::Delete
            )
        );
        if (command || self.modifiers.alt_key()) && !modified_edit {
            return true;
        }
        let mut staged = edit.clone();
        let edit = &mut staged;
        let word = self.modifiers.control_key() || self.modifiers.alt_key();
        let edge = self.modifiers.super_key();
        match &event.logical_key {
            Key::Named(NamedKey::ArrowLeft | NamedKey::ArrowRight) if edge => {
                edit.move_edge(
                    matches!(event.logical_key, Key::Named(NamedKey::ArrowRight)),
                    self.modifiers.shift_key(),
                );
            }
            Key::Named(NamedKey::ArrowLeft | NamedKey::ArrowRight) if word => {
                edit.move_word(
                    if matches!(event.logical_key, Key::Named(NamedKey::ArrowLeft)) {
                        -1
                    } else {
                        1
                    },
                    self.modifiers.shift_key(),
                );
            }
            Key::Named(NamedKey::Backspace | NamedKey::Delete) if edge => {
                let backward = matches!(event.logical_key, Key::Named(NamedKey::Backspace));
                if edit.selection().is_empty() {
                    edit.move_edge(!backward, true);
                }
                edit.delete(backward);
            }
            Key::Named(NamedKey::Backspace | NamedKey::Delete) if word => {
                edit.delete_word(matches!(event.logical_key, Key::Named(NamedKey::Backspace)));
            }
            Key::Named(NamedKey::ArrowLeft) => edit.move_caret(-1, self.modifiers.shift_key()),
            Key::Named(NamedKey::ArrowRight) => edit.move_caret(1, self.modifiers.shift_key()),
            Key::Named(NamedKey::Home) => edit.move_edge(false, self.modifiers.shift_key()),
            Key::Named(NamedKey::End) => edit.move_edge(true, self.modifiers.shift_key()),
            Key::Named(NamedKey::Backspace) => edit.delete(true),
            Key::Named(NamedKey::Delete) => edit.delete(false),
            _ => {
                if let Some(text) = event.text.as_deref() {
                    edit.insert(text, 4096);
                }
            }
        }
        let value = edit.value().to_owned();
        if self.runtime.edit_sandbox_creator_field(&key, &value) {
            self.living_text_edit
                .as_mut()
                .expect("field editor installed")
                .1 = staged;
        }
        true
    }

    #[cfg(not(target_arch = "wasm32"))]
    pub(super) fn poll_living_clipboard(&mut self) {
        let Some(completion) = self.living_clipboard.poll() else {
            return;
        };
        let Some(scope) = self.runtime.living_presentation_scope() else {
            return;
        };
        let Some((key, editor)) = self.living_text_edit.as_mut() else {
            return;
        };
        let focused = self
            .ui_focus
            .focused()
            .map(SemanticActionId::as_str)
            .and_then(|id| id.strip_prefix("sandbox.creator.field."));
        if focused != Some(key.as_str()) {
            return;
        }
        if self.runtime.screen() != ClientScreen::GalaxySandbox
            || self.player_guide.is_some()
            || self.runtime.sandbox_creator_field(key) != Some(editor.value())
        {
            return;
        }
        let mut staged = editor.clone();
        match completion.apply(&scope, key, &mut staged, 4096) {
            Ok(crate::ui::clipboard::ClipboardApply::Changed) => {
                if self.runtime.edit_sandbox_creator_field(key, staged.value()) {
                    *editor = staged;
                }
            }
            Ok(_) => {}
            Err(error) => self
                .runtime
                .report_living_presentation_failure(format!("Clipboard operation failed: {error}")),
        }
    }

    pub(super) fn insert_living_field_text(&mut self, text: &str) -> bool {
        let Some(key) = self
            .ui_focus
            .focused()
            .and_then(|action| action.as_str().strip_prefix("sandbox.creator.field."))
            .map(str::to_owned)
        else {
            return false;
        };
        let Some(value) = self.runtime.sandbox_creator_field(&key).map(str::to_owned) else {
            return false;
        };
        if self
            .living_text_edit
            .as_ref()
            .is_none_or(|(old_key, edit)| old_key != &key || edit.value() != value)
        {
            self.living_text_edit = Some((key.clone(), crate::ui::text_edit::TextEdit::new(value)));
        }
        let edit = &mut self
            .living_text_edit
            .as_mut()
            .expect("field editor installed")
            .1;
        let mut staged = edit.clone();
        if staged.insert(text, 4096)
            && self
                .runtime
                .edit_sandbox_creator_field(&key, staged.value())
        {
            *edit = staged;
        }
        true
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

    /// Typing into the Library rename field: the creator's text rules, the
    /// first keystroke replacing the name the dialog opened with. A key that
    /// would make the draft unacceptable (past 64 bytes) is consumed and
    /// ignored rather than falling through to navigation.
    fn handle_library_rename_key(&mut self, event: &winit::event::KeyEvent) -> bool {
        if self.modifiers.control_key() || self.modifiers.super_key() || self.modifiers.alt_key() {
            return false;
        }
        self.library_rename_input(
            event.text.as_deref(),
            matches!(event.logical_key, Key::Named(NamedKey::Backspace)),
        )
    }

    /// The key-independent half of [`Self::handle_library_rename_key`].
    fn library_rename_input(&mut self, text: Option<&str>, backspace: bool) -> bool {
        if self.ui_focus.focused().map(SemanticActionId::as_str)
            != Some(crate::ui::library::LIBRARY_RENAME_FIELD_ACTION)
        {
            return false;
        }
        let Some(next) = edited_creator_text(
            CreatorFieldKind::Text,
            &self.library_rename_draft,
            text,
            backspace,
            !self.library_rename_edited,
        ) else {
            return false;
        };
        self.set_library_rename_draft(next);
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

    /// Escape closes the Compact Library sheet before it leaves the Library,
    /// as it closes a Workshop drawer before leaving the Workshop, and focus
    /// goes back to whatever opened the sheet, exactly as Back does.
    #[cfg(not(target_arch = "wasm32"))]
    #[test]
    fn escape_closes_the_library_sheet_before_it_leaves_the_library() {
        use crate::app::library_frame_tests::library_app;
        use crate::ui::accessibility::SemanticActionId;
        use crate::ui::platform::LibraryViewAction;
        let (mut app, slot) = library_app();
        app.runtime
            .classic_mut()
            .set_viewport(Vec2::new(723.0, 802.0));
        app.build_frame();

        let row = SemanticActionId::new(format!("library.slot.{}", slot.0));
        let transfer = LibraryViewAction::OpenTransfer.action_id();
        for (opener, modality) in [
            (row.clone(), InputModality::Keyboard),
            (transfer.clone(), InputModality::Pointer),
        ] {
            assert!(app.ui_focus.request_focus(&opener), "{opener:?} not placed");
            app.activate_platform_action_id(&opener, modality);
            app.build_frame();
            assert!(app.platform_ui.as_ref().unwrap().drawer.is_some());

            app.handle_navigation(NavigationAction::Escape);
            assert_eq!(
                app.runtime.screen(),
                ClientScreen::Library,
                "Escape left the Library with the sheet from {opener:?} open"
            );
            app.build_frame();
            assert!(
                app.platform_ui.as_ref().unwrap().drawer.is_none(),
                "Escape left the sheet from {opener:?} open"
            );
            assert_eq!(app.ui_focus.focused(), Some(&opener));
            assert_eq!(app.selected_library_slot, Some(slot));
        }

        // With no sheet open, Escape leaves as before.
        app.handle_navigation(NavigationAction::Escape);
        assert_eq!(app.runtime.screen(), ClientScreen::MainMenu);
    }

    /// Task 9b end to end: Rename opens on the save's name with focus in the
    /// field, the first key replaces it, a name the store would refuse keeps
    /// the dialog and the draft, and a valid one reaches the store.
    #[cfg(not(target_arch = "wasm32"))]
    #[test]
    fn rename_types_into_its_field_and_only_a_valid_name_reaches_the_store() {
        use crate::app::library_frame_tests::{library_app, select, settle};
        use crate::ui::library::{LIBRARY_CONFIRM_SUBMIT_ACTION, LIBRARY_RENAME_FIELD_ACTION};
        let (mut app, slot) = library_app();
        select(&mut app, slot);
        let rename = SemanticActionId::new("library.action.rename");
        let field = SemanticActionId::new(LIBRARY_RENAME_FIELD_ACTION);
        let submit = SemanticActionId::new(LIBRARY_CONFIRM_SUBMIT_ACTION);
        let name = |app: &crate::app::library_frame_tests::TestApp| {
            app.runtime
                .library_slots()
                .unwrap()
                .slots
                .iter()
                .find(|summary| summary.id == slot)
                .unwrap()
                .name
                .as_str()
                .to_owned()
        };
        assert!(app.ui_focus.request_focus(&rename));
        app.activate_platform_action_id(&rename, InputModality::Keyboard);
        app.build_frame();
        assert_eq!(app.library_rename_draft, "Two-System Forge");
        assert_eq!(
            app.ui_focus.focused(),
            Some(&field),
            "typing has nowhere to go"
        );

        // The first key replaces the name; later keys append; Backspace edits.
        for (text, backspace, expected) in [
            (Some("N"), false, "N"),
            (Some("o"), false, "No"),
            (Some("vaX"), false, "NovaX"),
            (None, true, "Nova"),
            (Some(" "), false, "Nova "),
            (Some("\u{e9}"), false, "Nova "),
        ] {
            assert!(
                app.library_rename_input(text, backspace) || text == Some("\u{e9}"),
                "{text:?} was not consumed"
            );
            assert_eq!(app.library_rename_draft, expected, "after {text:?}");
        }
        app.build_frame();
        assert!(
            !app.platform_ui
                .as_ref()
                .unwrap()
                .controls
                .iter()
                .find(|control| control.action_id == submit)
                .unwrap()
                .enabled,
            "a trailing space was offered to the store"
        );

        // Enter in the field submits; the store would refuse, so nothing moves.
        app.activate_platform_action_id(&field, InputModality::Keyboard);
        assert!(
            app.library_confirmation.is_some(),
            "a refusal closed the dialog"
        );
        assert_eq!(
            app.library_rename_draft, "Nova ",
            "a refusal dropped the draft"
        );
        assert_eq!(
            app.runtime.library_slots_status(),
            crate::app::client_runtime::LibrarySlotsStatus::Idle
        );

        // A key past the 64-byte limit is consumed and changes nothing.
        let full = "W".repeat(64);
        assert!(app.set_library_rename_draft(full.clone()));
        assert!(app.library_rename_input(Some("X"), false));
        assert_eq!(app.library_rename_draft, full);

        // A browser edit follows the same rule.
        app.apply_creator_semantic_value(&field, "Caf\u{e9}".to_owned());
        assert_eq!(app.library_rename_draft, full);
        app.apply_creator_semantic_value(&field, "Nova".to_owned());
        assert_eq!(app.library_rename_draft, "Nova");

        app.build_frame();
        app.activate_platform_action_id(&field, InputModality::Keyboard);
        assert!(app.library_confirmation.is_none());
        assert!(app.library_rename_draft.is_empty());
        settle(&mut app);
        assert_eq!(name(&app), "Nova");
        assert_eq!(app.ui_focus.focused(), Some(&rename));

        // Cancel discards a draft and changes nothing.
        app.activate_platform_action_id(&rename, InputModality::Keyboard);
        assert!(app.library_rename_input(Some("Z"), false));
        app.handle_navigation(NavigationAction::Escape);
        assert!(app.library_rename_draft.is_empty());
        app.build_frame();
        assert_eq!(name(&app), "Nova");
        // Typing with no dialog open goes nowhere.
        assert!(!app.library_rename_input(Some("Q"), false));
    }

    /// Escape unwinds one layer at a time: the confirmation, then the sheet it
    /// was opened from, then the Library.
    #[cfg(not(target_arch = "wasm32"))]
    #[test]
    fn escape_closes_a_library_confirmation_before_its_sheet() {
        use crate::app::library_frame_tests::library_app;
        use crate::ui::accessibility::SemanticActionId;
        let (mut app, slot) = library_app();
        app.runtime
            .classic_mut()
            .set_viewport(Vec2::new(723.0, 802.0));
        app.build_frame();
        let row = SemanticActionId::new(format!("library.slot.{}", slot.0));
        let archive = SemanticActionId::new("library.action.archive");
        app.activate_platform_action_id(&row, InputModality::Pointer);
        app.build_frame();
        assert!(app.ui_focus.request_focus(&archive));
        app.activate_platform_action_id(&archive, InputModality::Keyboard);
        app.build_frame();
        assert!(app.platform_ui.as_ref().unwrap().modal.is_some());

        app.handle_navigation(NavigationAction::Escape);
        app.build_frame();
        let frame = app.platform_ui.as_ref().unwrap();
        assert!(frame.modal.is_none(), "Escape left the dialog open");
        assert!(
            frame.drawer.is_some(),
            "Escape closed the sheet with the dialog"
        );
        assert_eq!(app.ui_focus.focused(), Some(&archive));

        app.handle_navigation(NavigationAction::Escape);
        app.build_frame();
        assert!(app.platform_ui.as_ref().unwrap().drawer.is_none());
        assert_eq!(app.runtime.screen(), ClientScreen::Library);
        app.handle_navigation(NavigationAction::Escape);
        assert_eq!(app.runtime.screen(), ClientScreen::MainMenu);
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

    #[cfg(not(target_arch = "wasm32"))]
    fn sandbox_unopened_app() -> App<crate::scenario::store::MemoryScenarioStore> {
        use crate::{
            app::{AppCore, AppEventProxy},
            preferences::store::MemoryPreferencesStore,
            scenario::{ScenarioDraft, store::MemoryScenarioStore},
            workshop::store::MemoryWorkshopStore,
        };
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
            .set_viewport(Vec2::new(1280.0, 720.0));
        app
    }

    #[cfg(not(target_arch = "wasm32"))]
    #[test]
    fn owned_semantic_edit_precedes_preview_and_commits_the_reviewed_name() {
        use crate::sandbox_session::SandboxUiAction as A;
        use crate::ui::accessibility::{
            OwnedPlatformSemanticEvent as Queued, PlatformSemanticEvent as Event,
        };
        let mut app = sandbox_app();
        app.runtime.apply_sandbox_action(A::OpenCreator(
            crate::living_creator::LivingCreatorTool::System,
        ));
        app.build_sandbox_frame();
        let owner = app.semantic_input_owner();
        app.apply_owned_semantic_events(vec![
            Queued {
                owner: owner.clone(),
                event: Event::Edit(
                    SemanticActionId::new("sandbox.creator.field.0.name"),
                    "Ordered preview system".into(),
                ),
            },
            Queued {
                owner: owner.clone(),
                event: Event::Action(SemanticActionId::new("sandbox.creator.preview")),
            },
        ]);
        app.build_sandbox_frame();
        assert!(app.runtime.sandbox_creator_scene_open());
        assert_eq!(
            app.runtime.sandbox_creator_field("0.name"),
            Some("Ordered preview system")
        );
        app.apply_owned_semantic_events(vec![Queued {
            owner,
            event: Event::Action(SemanticActionId::new("sandbox.creator.scene.apply")),
        }]);
        app.build_sandbox_frame();
        assert!(!app.runtime.sandbox_creator_open());
        let scene = app
            .runtime
            .sandbox_scene(app.platform_ui.as_ref().unwrap().layout.canvas)
            .unwrap();
        assert!(
            scene
                .entities
                .iter()
                .any(|entity| entity.name == "Ordered preview system")
        );
        assert_eq!(app.runtime.sandbox_tick(), Some(0));
    }

    #[cfg(not(target_arch = "wasm32"))]
    #[test]
    fn queued_native_edit_is_flushed_before_pointer_preview() {
        use crate::sandbox_session::SandboxUiAction as A;
        let mut app = sandbox_app();
        app.runtime.apply_sandbox_action(A::OpenCreator(
            crate::living_creator::LivingCreatorTool::System,
        ));
        app.build_sandbox_frame();
        let point = app
            .platform_ui
            .as_ref()
            .unwrap()
            .controls
            .iter()
            .find(|control| control.action_id.as_str() == "sandbox.creator.preview")
            .unwrap()
            .bounds
            .center();
        app.semantic_adapter.request_edit(
            SemanticActionId::new("sandbox.creator.field.0.name"),
            "Pointer ordered system".into(),
        );
        app.begin_pointer(0, point, PointerSource::Mouse, PointerButton::Primary);
        assert!(app.runtime.sandbox_creator_scene_open());
        assert_eq!(
            app.runtime.sandbox_creator_field("0.name"),
            Some("Pointer ordered system")
        );
        app.runtime.apply_sandbox_action(A::CreatorApply);
        app.build_sandbox_frame();
        let scene = app
            .runtime
            .sandbox_scene(app.platform_ui.as_ref().unwrap().layout.canvas)
            .unwrap();
        assert!(
            scene
                .entities
                .iter()
                .any(|entity| entity.name == "Pointer ordered system")
        );
        assert_eq!(app.runtime.sandbox_tick(), Some(0));
    }

    #[cfg(not(target_arch = "wasm32"))]
    #[test]
    fn queued_context_replacement_retires_old_pointer_target() {
        use crate::sandbox_session::SandboxUiAction as A;
        let mut app = sandbox_app();
        app.runtime.apply_sandbox_action(A::OpenCreator(
            crate::living_creator::LivingCreatorTool::System,
        ));
        app.build_sandbox_frame();
        let point = app
            .platform_ui
            .as_ref()
            .unwrap()
            .controls
            .iter()
            .find(|control| control.action_id.as_str() == "sandbox.creator.preview")
            .unwrap()
            .bounds
            .center();
        let cancel = app
            .platform_ui
            .as_ref()
            .unwrap()
            .controls
            .iter()
            .find(|control| {
                control.action == crate::ui::platform::PlatformUiAction::Sandbox(A::CreatorCancel)
            })
            .unwrap()
            .action_id
            .clone();
        app.semantic_adapter.request_action(cancel);
        app.begin_pointer(0, point, PointerSource::Mouse, PointerButton::Primary);
        assert!(!app.runtime.sandbox_creator_open());
        assert!(!app.runtime.sandbox_creator_scene_open());
        assert_eq!(app.runtime.sandbox_tick(), Some(0));
    }

    #[cfg(not(target_arch = "wasm32"))]
    #[test]
    fn owned_semantic_input_accepts_same_draft_and_rejects_replacement_owners() {
        use crate::sandbox_session::SandboxUiAction as A;
        use crate::ui::accessibility::{
            OwnedPlatformSemanticEvent as Queued, PlatformSemanticEvent as Event,
        };
        let mut app = sandbox_app();
        app.runtime.apply_sandbox_action(A::OpenCreator(
            crate::living_creator::LivingCreatorTool::System,
        ));
        app.build_sandbox_frame();
        let owner = app.semantic_input_owner();
        for value in ["First edit", "Second edit"] {
            app.apply_owned_semantic_events(vec![Queued {
                owner: owner.clone(),
                event: Event::Edit(
                    SemanticActionId::new("sandbox.creator.field.0.name"),
                    value.into(),
                ),
            }]);
            assert_eq!(app.runtime.sandbox_creator_field("0.name"), Some(value));
            if value == "First edit" {
                app.apply_owned_semantic_events(vec![Queued {
                    owner: owner.clone(),
                    event: Event::Action(SemanticActionId::new("sandbox.creator.preview")),
                }]);
                app.build_sandbox_frame();
                assert!(app.runtime.sandbox_creator_scene_open());
            }
        }
        app.runtime.apply_sandbox_action(A::OpenCreator(
            crate::living_creator::LivingCreatorTool::System,
        ));
        app.build_sandbox_frame();
        let replacement_value = app
            .runtime
            .sandbox_creator_field("0.name")
            .unwrap()
            .to_owned();
        app.apply_owned_semantic_events(vec![
            Queued {
                owner: owner.clone(),
                event: Event::Edit(
                    SemanticActionId::new("sandbox.creator.field.0.name"),
                    "Stale edit".into(),
                ),
            },
            Queued {
                owner,
                event: Event::Action(SemanticActionId::new("sandbox.creator.preview")),
            },
        ]);
        assert_eq!(
            app.runtime.sandbox_creator_field("0.name"),
            Some(replacement_value.as_str())
        );
        assert!(!app.runtime.sandbox_creator_scene_open());
        app.runtime.apply_sandbox_action(A::CreatorCancel);
        app.build_sandbox_frame();
        let resident = app.semantic_input_owner();
        // Fresh resident fixture replaces the unsaved current runtime explicitly;
        // normal content creation correctly refuses dirty resident replacement.
        app.runtime = sandbox_app().runtime;
        app.build_sandbox_frame();
        assert!(!resident.matches(&app.semantic_input_owner()));
        app.apply_owned_semantic_events(vec![Queued {
            owner: resident,
            event: Event::Action(SemanticActionId::new("sandbox.toggle_pause")),
        }]);
        assert_eq!(app.runtime.sandbox_paused(), Some(true));
        assert_eq!(app.runtime.sandbox_tick(), Some(0));
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn sandbox_app() -> App<crate::scenario::store::MemoryScenarioStore> {
        let mut app = sandbox_unopened_app();
        app.runtime
            .select_menu_route(crate::app::client_runtime::MainMenuRoute::GalaxySandbox)
            .unwrap();
        app.build_sandbox_frame();
        app
    }

    #[cfg(not(target_arch = "wasm32"))]
    #[test]
    fn first_living_menu_entry_holds_intro_then_resident_reentry_keeps_dismissal() {
        let mut app = sandbox_unopened_app();
        app.apply_shell_action(crate::ui::platform::ShellUiAction::Menu(
            crate::app::client_runtime::MainMenuRoute::GalaxySandbox,
        ));
        app.build_sandbox_frame();
        assert!(app.platform_ui.as_ref().unwrap().modal.is_some());
        app.runtime.update(std::time::Duration::from_secs(20));
        assert_eq!(app.runtime.sandbox_tick(), Some(0));
        app.runtime
            .apply_sandbox_action(crate::sandbox_session::SandboxUiAction::IntroStart(false));
        app.build_sandbox_frame();
        assert!(app.platform_ui.as_ref().unwrap().modal.is_none());
        app.runtime.step_sandbox(1).unwrap();
        app.runtime.close_sandbox();
        app.apply_shell_action(crate::ui::platform::ShellUiAction::Menu(
            crate::app::client_runtime::MainMenuRoute::GalaxySandbox,
        ));
        app.build_sandbox_frame();
        assert!(app.platform_ui.as_ref().unwrap().modal.is_none());
        assert_eq!(app.runtime.sandbox_tick(), Some(1));
    }

    #[cfg(not(target_arch = "wasm32"))]
    #[test]
    fn sandbox_field_focus_installs_caret_without_typing_and_preserves_selection() {
        let mut app = sandbox_app();
        app.apply_platform_action(
            crate::ui::platform::PlatformUiAction::Sandbox(
                crate::sandbox_session::SandboxUiAction::OpenCreator(
                    crate::living_creator::LivingCreatorTool::System,
                ),
            ),
            InputModality::Keyboard,
        );
        app.build_sandbox_frame();
        let action = app
            .platform_ui
            .as_ref()
            .unwrap()
            .controls
            .iter()
            .find(|control| {
                control
                    .action_id
                    .as_str()
                    .starts_with("sandbox.creator.field.")
            })
            .unwrap()
            .action_id
            .clone();
        assert!(app.ui_focus.request_focus(&action));
        app.living_text_edit = None;
        app.build_sandbox_frame();
        let editor = &mut app.living_text_edit.as_mut().unwrap().1;
        assert_eq!(editor.caret(), editor.value().len());
        editor.select_all();
        let selection = editor.selection();
        app.build_sandbox_frame();
        assert_eq!(
            app.living_text_edit.as_ref().unwrap().1.selection(),
            selection
        );
        assert_eq!(app.runtime.sandbox_tick(), Some(0));
    }

    #[cfg(not(target_arch = "wasm32"))]
    #[test]
    fn native_field_pointer_places_caret_and_drag_selects_without_publishing() {
        let mut app = sandbox_app();
        app.apply_platform_action(
            crate::ui::platform::PlatformUiAction::Sandbox(
                crate::sandbox_session::SandboxUiAction::OpenCreator(
                    crate::living_creator::LivingCreatorTool::System,
                ),
            ),
            InputModality::Keyboard,
        );
        app.build_sandbox_frame();
        let bounds = app
            .platform_ui
            .as_ref()
            .unwrap()
            .controls
            .iter()
            .find(|control| control.action_id.as_str() == "sandbox.creator.field.0.name")
            .unwrap()
            .bounds;
        let point = Vec2::new(bounds.min.x + 2.0, (bounds.min.y + bounds.max.y) / 2.0);
        app.begin_pointer(0, point, PointerSource::Mouse, PointerButton::Primary);
        assert_eq!(app.living_text_edit.as_ref().unwrap().1.caret(), 0);
        assert!(app.living_text_drag.is_some());
        app.update_pointer(0, Vec2::new(bounds.max.x + 100.0, point.y));
        let editor = &app.living_text_edit.as_ref().unwrap().1;
        assert_eq!(editor.selection(), 0..editor.value().len());
        let value = editor.value().to_owned();
        app.end_pointer(0, Vec2::new(bounds.max.x + 100.0, point.y));
        assert!(app.living_text_drag.is_none());
        assert_eq!(
            app.runtime.sandbox_creator_field("0.name"),
            Some(value.as_str())
        );
        assert_eq!(app.runtime.sandbox_tick(), Some(0));
    }

    #[cfg(not(target_arch = "wasm32"))]
    #[test]
    fn sandbox_focused_space_activation_matches_pointer_selection() {
        let mut keyboard = sandbox_app();
        let next = SemanticActionId::new("sandbox.world.next");
        assert!(keyboard.ui_focus.request_focus(&next));
        let activate =
            navigation_for_key(&Key::Named(NamedKey::Space), keyboard.modifiers).unwrap();
        assert_eq!(activate, NavigationAction::Activate);
        keyboard.handle_navigation(activate);
        let bounds = keyboard.platform_ui.as_ref().unwrap().layout.canvas;
        let selected = keyboard
            .runtime
            .sandbox_scene(bounds)
            .unwrap()
            .selected_world;
        assert!(selected.is_some());
        assert_eq!(keyboard.runtime.sandbox_tick(), Some(0));
        assert_eq!(keyboard.runtime.sandbox_paused(), Some(true));

        let mut pointer = sandbox_app();
        let point = pointer
            .platform_ui
            .as_ref()
            .unwrap()
            .controls
            .iter()
            .find(|control| control.action_id == next)
            .unwrap()
            .bounds
            .center();
        pointer.begin_pointer(0, point, PointerSource::Mouse, PointerButton::Primary);
        assert_eq!(
            pointer
                .runtime
                .sandbox_scene(bounds)
                .unwrap()
                .selected_world,
            selected
        );
        assert_eq!(pointer.runtime.sandbox_tick(), Some(0));

        assert!(
            keyboard
                .ui_focus
                .request_focus(&SemanticActionId::new("sandbox.toggle_pause"))
        );
        keyboard.handle_navigation(
            navigation_for_key(&Key::Named(NamedKey::Space), keyboard.modifiers).unwrap(),
        );
        assert_eq!(keyboard.runtime.sandbox_paused(), Some(false));
        assert_eq!(keyboard.runtime.sandbox_tick(), Some(0));
    }

    #[cfg(not(target_arch = "wasm32"))]
    #[test]
    fn sandbox_visibility_holds_are_independent_and_clear_fractional_debt() {
        use crate::sandbox_session::SandboxUiAction;
        use std::time::Duration;
        let mut app = sandbox_app();
        app.runtime
            .apply_sandbox_action(SandboxUiAction::TogglePause);
        app.runtime.update(Duration::from_millis(90));
        assert!(!app.sandbox_transport_held());
        app.sandbox_window_occluded = true;
        app.runtime
            .set_sandbox_suspended(app.sandbox_transport_held());
        app.runtime.update(Duration::from_secs(10));
        assert_eq!(app.runtime.sandbox_tick(), Some(0));
        app.sandbox_window_focused = false;
        app.sandbox_window_occluded = false;
        assert!(app.sandbox_transport_held());
        app.sandbox_window_focused = true;
        app.sandbox_window_suspended = true;
        assert!(app.sandbox_transport_held());
        app.sandbox_window_suspended = false;
        assert!(!app.sandbox_transport_held());
        app.runtime
            .set_sandbox_suspended(app.sandbox_transport_held());
        app.runtime.update(Duration::from_millis(10));
        assert_eq!(app.runtime.sandbox_tick(), Some(0));
        app.runtime.update(Duration::from_millis(90));
        for _ in 0..100 {
            if app.runtime.sandbox_tick() == Some(1) {
                break;
            }
            app.runtime.update(Duration::ZERO);
        }
        assert_eq!(app.runtime.sandbox_tick(), Some(1));
    }
}
