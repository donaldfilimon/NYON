//! Settings focus and action model shared by pointer and keyboard input.

use crate::{
    preferences::UiScale,
    presentation::{GraphicsQuality, MotionPreference},
};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SettingsAction {
    SetUiScale(UiScale),
    SetMotion(MotionPreference),
    SetHighContrast(bool),
    SetGraphicsQuality(GraphicsQuality),
    ResetOnboarding,
    Close,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct SettingsState {
    focus_index: usize,
}

impl SettingsState {
    pub const CONTROL_COUNT: usize = 11;

    pub const fn focus_index(self) -> usize {
        self.focus_index
    }

    pub fn focus(&mut self, index: usize) {
        self.focus_index = index.min(Self::CONTROL_COUNT - 1);
    }

    pub fn focus_next(&mut self) {
        self.focus_index = (self.focus_index + 1) % Self::CONTROL_COUNT;
    }

    pub fn focus_previous(&mut self) {
        self.focus_index = (self.focus_index + Self::CONTROL_COUNT - 1) % Self::CONTROL_COUNT;
    }
}
