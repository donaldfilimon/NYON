//! Settings, onboarding, and keyboard-focus transitions for [`AppCore`].

use super::*;

impl<S: ScenarioStore, P: PreferencesStore> AppCore<S, P> {
    pub fn open_settings(&mut self) {
        if self.mode != AppMode::Playing {
            return;
        }
        self.mode = AppMode::Settings;
        self.settings = SettingsState::default();
        self.ui_focus_action = None;
        self.clock.clear();
    }

    pub fn close_settings(&mut self) {
        if self.mode == AppMode::Settings {
            self.mode = AppMode::Playing;
            self.clock.clear();
        }
    }

    pub fn handle_ui_action(&mut self, action: UiAction) -> bool {
        match action {
            UiAction::Launch => self.launch_contextual_preview(),
            UiAction::ClearSelection => {
                self.clear_scene_selection();
                true
            }
            UiAction::SpeedDown => self.handle_action(Action::SpeedDown),
            UiAction::TogglePause => self.handle_action(Action::Pause),
            UiAction::SpeedUp => self.handle_action(Action::SpeedUp),
            UiAction::ResetCamera => self.handle_action(Action::CameraReset),
            UiAction::OpenSettings => self.handle_action(Action::Settings),
            UiAction::ToggleHelp => self.handle_action(Action::Help),
            UiAction::OpenScenario => self.handle_action(Action::Scenario),
            UiAction::AcknowledgeAdvisory => {
                self.observe_onboarding(OnboardingEvent::AdvisoryRead);
                true
            }
            UiAction::SkipOnboarding => {
                self.skip_onboarding();
                true
            }
            UiAction::ResetOnboarding => {
                self.reset_onboarding();
                true
            }
            UiAction::SelectField(field) => self.handle_action(match field {
                FieldKind::Atmosphere => Action::Field1,
                FieldKind::Hydrosphere => Action::Field2,
                FieldKind::Topology => Action::Field3,
            }),
            UiAction::DecreaseField => self.handle_action(Action::Decrease),
            UiAction::IncreaseField => self.handle_action(Action::Increase),
            UiAction::Settings(action) => self.handle_settings_action(action),
        }
    }

    pub fn clear_ui_focus(&mut self) {
        if self.mode == AppMode::Playing {
            self.ui_focus_action = None;
        }
    }

    pub fn focus_ui_next(&mut self, backwards: bool) {
        if self.mode == AppMode::Settings {
            if backwards {
                self.settings.focus_previous();
            } else {
                self.settings.focus_next();
            }
            return;
        }
        let count = self.ui_frame().controls.len();
        if count == 0 {
            self.ui_focus_action = None;
            return;
        }
        let controls = self.ui_frame().controls;
        let current = self
            .ui_focus_action
            .and_then(|action| controls.iter().position(|control| control.action == action))
            .unwrap_or(if backwards { 0 } else { count - 1 });
        let next = if backwards {
            (current + count - 1) % count
        } else {
            (current + 1) % count
        };
        self.ui_focus_action = Some(controls[next].action);
    }

    pub fn focus_ui(&mut self, index: usize) {
        if self.mode == AppMode::Settings {
            self.settings.focus(index);
        } else if index < self.ui_frame().controls.len() {
            self.ui_focus_action = self
                .ui_frame()
                .controls
                .get(index)
                .map(|control| control.action);
        }
    }

    pub fn activate_focused_ui(&mut self) -> bool {
        let index = if self.mode == AppMode::Settings {
            self.settings.focus_index()
        } else {
            let Some(action) = self.ui_focus_action else {
                return false;
            };
            let frame = self.ui_frame();
            let Some(control) = frame
                .controls
                .iter()
                .find(|control| control.action == action && control.enabled)
            else {
                self.ui_focus_action = None;
                return false;
            };
            return self.handle_ui_action(control.action);
        };
        let Some(action) = self
            .ui_frame()
            .controls
            .get(index)
            .filter(|control| control.enabled)
            .map(|control| control.action)
        else {
            return false;
        };
        self.handle_ui_action(action)
    }

    pub fn handle_settings_action(&mut self, action: SettingsAction) -> bool {
        if self.mode != AppMode::Settings {
            return false;
        }
        match action {
            SettingsAction::SetUiScale(scale) => self.preferences.ui_scale = scale,
            SettingsAction::SetMotion(motion) => self.preferences.motion = motion,
            SettingsAction::SetHighContrast(enabled) => {
                self.preferences.high_contrast = enabled;
            }
            SettingsAction::SetGraphicsQuality(quality) => {
                self.preferences.graphics_quality = quality;
            }
            SettingsAction::ResetOnboarding => {
                self.preferences.onboarding_completed = false;
                self.onboarding.reset();
                self.set_game_speed(GameSpeed::Paused);
            }
            SettingsAction::Close => {
                self.close_settings();
                return true;
            }
        }
        self.apply_preferences();
        self.persist_preferences();
        true
    }

    pub fn skip_onboarding(&mut self) {
        if self.onboarding.skip() {
            self.preferences.onboarding_completed = true;
            if self.game_speed == GameSpeed::Paused {
                self.set_game_speed(self.resume_speed);
            }
            self.persist_preferences();
        }
    }

    pub fn reset_onboarding(&mut self) {
        self.onboarding.reset();
        self.preferences.onboarding_completed = false;
        self.help_visible = false;
        self.set_game_speed(GameSpeed::Paused);
        self.persist_preferences();
    }
}
