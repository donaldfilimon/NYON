//! Presentation-only first-run guidance.

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum OnboardingStep {
    SelectUnionWorld,
    PreviewLaunch,
    LaunchFleet,
    TuneField,
    ReadAdvisory,
    OpenScenarioEditor,
}

impl OnboardingStep {
    pub const ALL: [Self; 6] = [
        Self::SelectUnionWorld,
        Self::PreviewLaunch,
        Self::LaunchFleet,
        Self::TuneField,
        Self::ReadAdvisory,
        Self::OpenScenarioEditor,
    ];

    pub const fn title(self) -> &'static str {
        match self {
            Self::SelectUnionWorld => crate::ui::text::SELECT_UNION_WORLD,
            Self::PreviewLaunch => crate::ui::text::PREVIEW_DESTINATION,
            Self::LaunchFleet => crate::ui::text::LAUNCH_FLEET,
            Self::TuneField => crate::ui::text::TUNE_FIELD,
            Self::ReadAdvisory => crate::ui::text::READ_ADVISORY,
            Self::OpenScenarioEditor => crate::ui::text::OPEN_SCENARIO_EDITOR,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum OnboardingEvent {
    UnionWorldSelected,
    LaunchPreviewed,
    FleetLaunched,
    FieldTuned,
    AdvisoryRead,
    ScenarioEditorOpened,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct OnboardingState {
    step_index: Option<usize>,
}

impl OnboardingState {
    pub const fn new(completed: bool) -> Self {
        Self {
            step_index: if completed { None } else { Some(0) },
        }
    }

    pub const fn active(self) -> bool {
        self.step_index.is_some()
    }

    pub fn step(self) -> Option<OnboardingStep> {
        self.step_index
            .and_then(|index| OnboardingStep::ALL.get(index).copied())
    }

    pub const fn progress(self) -> (usize, usize) {
        (
            match self.step_index {
                Some(index) => index + 1,
                None => OnboardingStep::ALL.len(),
            },
            OnboardingStep::ALL.len(),
        )
    }

    /// Advances only when the action observed by the session matches the
    /// current instruction. Returns true when onboarding just completed.
    pub fn observe(&mut self, event: OnboardingEvent) -> bool {
        let expected = match self.step() {
            Some(OnboardingStep::SelectUnionWorld) => OnboardingEvent::UnionWorldSelected,
            Some(OnboardingStep::PreviewLaunch) => OnboardingEvent::LaunchPreviewed,
            Some(OnboardingStep::LaunchFleet) => OnboardingEvent::FleetLaunched,
            Some(OnboardingStep::TuneField) => OnboardingEvent::FieldTuned,
            Some(OnboardingStep::ReadAdvisory) => OnboardingEvent::AdvisoryRead,
            Some(OnboardingStep::OpenScenarioEditor) => OnboardingEvent::ScenarioEditorOpened,
            None => return false,
        };
        if event != expected {
            return false;
        }
        let index = self.step_index.expect("active onboarding has a step");
        if index + 1 == OnboardingStep::ALL.len() {
            self.step_index = None;
            true
        } else {
            self.step_index = Some(index + 1);
            false
        }
    }

    pub fn skip(&mut self) -> bool {
        let was_active = self.active();
        self.step_index = None;
        was_active
    }

    pub fn reset(&mut self) {
        self.step_index = Some(0);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn guidance_advances_only_from_the_expected_observed_action() {
        let mut state = OnboardingState::new(false);
        assert_eq!(state.step(), Some(OnboardingStep::SelectUnionWorld));
        assert!(!state.observe(OnboardingEvent::FleetLaunched));
        assert_eq!(state.step(), Some(OnboardingStep::SelectUnionWorld));

        for event in [
            OnboardingEvent::UnionWorldSelected,
            OnboardingEvent::LaunchPreviewed,
            OnboardingEvent::FleetLaunched,
            OnboardingEvent::FieldTuned,
            OnboardingEvent::AdvisoryRead,
        ] {
            assert!(!state.observe(event));
        }
        assert!(state.observe(OnboardingEvent::ScenarioEditorOpened));
        assert!(!state.active());
    }
}
