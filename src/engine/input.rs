use std::collections::HashSet;

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum Action {
    Field1,
    Field2,
    Field3,
    Decrease,
    Increase,
    Launch,
    Pause,
    SpeedDown,
    SpeedUp,
    CameraReset,
    Settings,
    Restart,
    Help,
    Cancel,
    Scenario,
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum NavigationAction {
    TabForward,
    TabBackward,
    Up,
    Down,
    Left,
    Right,
    PageUp,
    PageDown,
    Activate,
    Escape,
    Backspace,
    Delete,
    Home,
    End,
}

#[derive(Default)]
pub struct InputState {
    pub cursor_logical: glam::Vec2,
    pub left_clicked: bool,
    pub right_clicked: bool,
    pub primary_down: bool,
    pub wheel_delta: glam::Vec2,
    held: HashSet<Action>,
    pressed: HashSet<Action>,
    navigation: HashSet<NavigationAction>,
    committed_text: String,
    ime_preedit: String,
}

impl InputState {
    pub fn set_action(&mut self, action: Action, down: bool) {
        if down {
            if self.held.insert(action) {
                self.pressed.insert(action);
            }
        } else {
            self.held.remove(&action);
        }
    }

    pub fn take_pressed(&mut self, action: Action) -> bool {
        self.pressed.remove(&action)
    }

    pub fn press_navigation(&mut self, action: NavigationAction) {
        self.navigation.insert(action);
    }

    pub fn take_navigation(&mut self, action: NavigationAction) -> bool {
        self.navigation.remove(&action)
    }

    pub fn push_text(&mut self, text: &str) {
        self.committed_text.push_str(text);
    }

    pub fn commit_ime(&mut self, text: &str) {
        self.push_text(text);
        self.ime_preedit.clear();
    }

    pub fn take_text(&mut self) -> String {
        std::mem::take(&mut self.committed_text)
    }

    pub fn set_ime_preedit(&mut self, text: impl Into<String>) {
        self.ime_preedit = text.into();
    }

    pub fn ime_preedit(&self) -> &str {
        &self.ime_preedit
    }

    pub fn add_wheel_delta(&mut self, delta: glam::Vec2) {
        if delta.is_finite() {
            self.wheel_delta += delta;
        }
    }

    pub fn clear_transient(&mut self) {
        self.pressed.clear();
        self.navigation.clear();
        self.left_clicked = false;
        self.right_clicked = false;
        self.wheel_delta = glam::Vec2::ZERO;
        self.committed_text.clear();
    }

    pub fn clear_all(&mut self) {
        self.clear_transient();
        self.held.clear();
        self.primary_down = false;
        self.ime_preedit.clear();
    }

    pub fn finish_frame(&mut self) {
        self.clear_transient();
    }
}

#[cfg(test)]
mod tests {
    use super::{Action, InputState, NavigationAction};

    #[test]
    fn a_pressed_action_is_consumed_once() {
        let mut input = InputState::default();
        input.set_action(Action::Launch, true);
        assert!(input.take_pressed(Action::Launch));
        assert!(!input.take_pressed(Action::Launch));
    }

    #[test]
    fn release_then_repress_creates_a_new_edge() {
        let mut input = InputState::default();
        input.set_action(Action::Launch, true);
        assert!(input.take_pressed(Action::Launch));
        input.set_action(Action::Launch, false);
        input.set_action(Action::Launch, true);
        assert!(input.take_pressed(Action::Launch));
    }

    #[test]
    fn finish_frame_clears_only_edges_and_clicks() {
        let mut input = InputState::default();
        input.set_action(Action::Launch, true);
        input.left_clicked = true;
        input.right_clicked = true;
        input.finish_frame();
        assert!(!input.take_pressed(Action::Launch));
        assert!(!input.left_clicked);
        assert!(!input.right_clicked);
        input.set_action(Action::Launch, false);
        assert!(!input.take_pressed(Action::Launch));
    }

    #[test]
    fn editor_text_ime_navigation_and_wheel_are_transient() {
        let mut input = InputState::default();
        input.push_text("A");
        input.set_ime_preedit("ST");
        input.commit_ime("ER");
        input.press_navigation(NavigationAction::TabForward);
        input.add_wheel_delta(glam::Vec2::new(0.0, 3.0));
        assert_eq!(input.take_text(), "AER");
        assert_eq!(input.ime_preedit(), "");
        assert!(input.take_navigation(NavigationAction::TabForward));
        assert_eq!(input.wheel_delta.y, 3.0);
        input.finish_frame();
        assert_eq!(input.wheel_delta, glam::Vec2::ZERO);
    }
}
