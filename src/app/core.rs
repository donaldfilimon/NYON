//! Platform-neutral application and session transitions.

use std::time::Duration;

use glam::Vec2;

use crate::{
    advisory::{
        AdvisoryController, AdvisoryRequest, AdvisorySnapshot, AdvisoryTrigger, CompletionOutcome,
        gpu::GpuAdvisoryError,
    },
    editor::{ConfirmedEditorAction, EditorAction, EditorError, EditorState},
    engine::{input::Action, time::FixedClock},
    game::{
        model::{CommandSequence, Faction, FieldAdjustment, FieldKind, GameCommand, Tick, WorldId},
        simulation::{CommandResult, Simulation},
        view::{GameLayout, ViewState, hit_test_world},
    },
    preferences::{
        UserPreferencesV1, load_or_default, save_or_default,
        store::{MemoryPreferencesStore, PreferencesStore},
    },
    presentation::{
        CameraState, PresentationPreferences, SceneFrame, SceneRay, build_scene_frame,
        pick_world_after_hud,
        ui::{CommandTrayState, UiAction, UiBuildState, UiFrame, UiScreen, build_ui_frame},
    },
    scenario::{ScenarioV1, store::ScenarioStore},
};

use super::{
    onboarding::{OnboardingEvent, OnboardingState, OnboardingStep},
    settings::{SettingsAction, SettingsState},
};

mod session;

const MAX_STEPS_PER_FRAME: usize = 15;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AppMode {
    Playing,
    EditingScenario,
    Settings,
}

#[derive(Clone, Copy, Debug, Default, Eq, Ord, PartialEq, PartialOrd)]
pub enum GameSpeed {
    Paused,
    #[default]
    Normal,
    Fast,
    VeryFast,
}

impl GameSpeed {
    pub const ALL: [Self; 4] = [Self::Paused, Self::Normal, Self::Fast, Self::VeryFast];

    pub const fn multiplier(self) -> u8 {
        match self {
            Self::Paused => 0,
            Self::Normal => 1,
            Self::Fast => 2,
            Self::VeryFast => 4,
        }
    }

    pub const fn faster(self) -> Self {
        match self {
            Self::Paused => Self::Normal,
            Self::Normal => Self::Fast,
            Self::Fast | Self::VeryFast => Self::VeryFast,
        }
    }

    pub const fn slower(self) -> Self {
        match self {
            Self::Paused | Self::Normal => Self::Paused,
            Self::Fast => Self::Normal,
            Self::VeryFast => Self::Fast,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PointerIntent {
    Select,
    Launch,
}

pub fn pointer_command(
    campaign: &crate::game::model::Campaign,
    layout: &GameLayout,
    selected_world: Option<WorldId>,
    cursor: Vec2,
    intent: PointerIntent,
) -> Option<GameCommand> {
    let target = hit_test_world(campaign, layout, cursor)?;
    match intent {
        PointerIntent::Select => None,
        PointerIntent::Launch => Some(GameCommand::Launch {
            source: selected_world?,
            destination: target,
        }),
    }
}

pub const fn game_command_for_action(
    action: Action,
    selected_world: Option<WorldId>,
    hovered_world: Option<WorldId>,
    selected_field: FieldKind,
) -> Option<GameCommand> {
    match action {
        Action::Launch => Some(GameCommand::Launch {
            source: match selected_world {
                Some(world) => world,
                None => return None,
            },
            destination: match hovered_world {
                Some(world) => world,
                None => return None,
            },
        }),
        Action::Decrease | Action::Increase => Some(GameCommand::TuneField {
            world: match selected_world {
                Some(world) => world,
                None => return None,
            },
            field: selected_field,
            adjustment: if matches!(action, Action::Decrease) {
                FieldAdjustment::Decrease
            } else {
                FieldAdjustment::Increase
            },
        }),
        _ => None,
    }
}

#[derive(Debug, thiserror::Error)]
pub enum AppTransitionError {
    #[error(transparent)]
    Editor(#[from] EditorError),
}

/// Platform-neutral application/session state. Campaign truth remains inside
/// `Simulation`; every other field here is presentation or lifecycle state.
pub struct AppCore<S: ScenarioStore, P: PreferencesStore = MemoryPreferencesStore> {
    active_scenario: ScenarioV1,
    simulation: Simulation,
    mode: AppMode,
    editor: Option<EditorState>,
    store: S,
    preference_store: P,
    preferences: UserPreferencesV1,
    clock: FixedClock,
    selected_world: Option<WorldId>,
    hovered_world: Option<WorldId>,
    touch_command_target: Option<WorldId>,
    pointer_position: Vec2,
    selected_field: FieldKind,
    game_speed: GameSpeed,
    resume_speed: GameSpeed,
    help_visible: bool,
    onboarding: OnboardingState,
    settings: SettingsState,
    ui_focus_action: Option<UiAction>,
    next_sequence: u64,
    advisory: AdvisoryController,
    pending_gpu_request: Option<AdvisoryRequest>,
    viewport: Vec2,
    recoverable_message: Option<String>,
    preference_message: Option<String>,
    camera: CameraState,
    presentation_preferences: PresentationPreferences,
    visual_time: f32,
}

impl<S: ScenarioStore> AppCore<S, MemoryPreferencesStore> {
    pub fn new(active_scenario: ScenarioV1, store: S) -> Self {
        Self::new_with_preferences(active_scenario, store, MemoryPreferencesStore::default())
    }
}

impl<S: ScenarioStore, P: PreferencesStore> AppCore<S, P> {
    pub fn new_with_preferences(
        active_scenario: ScenarioV1,
        store: S,
        preference_store: P,
    ) -> Self {
        let rules = active_scenario.rules();
        let simulation = Simulation::from_scenario(&active_scenario);
        let preference_outcome = load_or_default(&preference_store);
        let preferences = preference_outcome.preferences;
        let preference_message = preference_outcome.recoverable_message();
        let onboarding = OnboardingState::new(preferences.onboarding_completed);
        let game_speed = if onboarding.active() {
            GameSpeed::Paused
        } else {
            GameSpeed::Normal
        };
        let mut advisory = AdvisoryController::new(0, false);
        advisory.request_if_due(
            simulation.state(),
            simulation.canonical_fingerprint(),
            AdvisoryTrigger::Initialization,
        );
        let mut camera = CameraState::new(Vec2::new(1440.0, 900.0));
        camera.set_motion(preferences.motion);
        Self {
            active_scenario,
            simulation,
            mode: AppMode::Playing,
            editor: None,
            store,
            preference_store,
            preferences,
            clock: FixedClock::new(rules.tick_hz, MAX_STEPS_PER_FRAME),
            selected_world: None,
            hovered_world: None,
            touch_command_target: None,
            pointer_position: Vec2::ZERO,
            selected_field: FieldKind::Atmosphere,
            game_speed,
            resume_speed: GameSpeed::Normal,
            help_visible: false,
            onboarding,
            settings: SettingsState::default(),
            ui_focus_action: None,
            next_sequence: 0,
            advisory,
            pending_gpu_request: None,
            viewport: Vec2::new(1440.0, 900.0),
            recoverable_message: None,
            preference_message,
            camera,
            presentation_preferences: preferences.presentation(),
            visual_time: 0.0,
        }
    }

    pub fn active_scenario(&self) -> &ScenarioV1 {
        &self.active_scenario
    }

    pub fn simulation(&self) -> &Simulation {
        &self.simulation
    }

    pub fn mode(&self) -> AppMode {
        self.mode
    }

    pub fn editor(&self) -> Option<&EditorState> {
        self.editor.as_ref()
    }

    pub fn editor_mut(&mut self) -> Option<&mut EditorState> {
        self.editor.as_mut()
    }

    pub fn paused(&self) -> bool {
        self.game_speed == GameSpeed::Paused
    }

    pub fn game_speed(&self) -> GameSpeed {
        self.game_speed
    }

    pub fn set_game_speed(&mut self, speed: GameSpeed) {
        if self.onboarding.active() && speed != GameSpeed::Paused {
            return;
        }
        if self.game_speed == speed {
            return;
        }
        self.game_speed = speed;
        if speed != GameSpeed::Paused {
            self.resume_speed = speed;
        }
        self.clock.clear();
    }

    pub fn help_visible(&self) -> bool {
        self.help_visible
    }

    pub fn onboarding(&self) -> OnboardingState {
        self.onboarding
    }

    pub fn onboarding_step(&self) -> Option<OnboardingStep> {
        self.onboarding.step()
    }

    pub fn preferences(&self) -> UserPreferencesV1 {
        self.preferences
    }

    pub fn settings(&self) -> SettingsState {
        self.settings
    }

    pub fn ui_focus_index(&self) -> Option<usize> {
        if self.mode == AppMode::Settings {
            Some(self.settings.focus_index())
        } else {
            let action = self.ui_focus_action?;
            self.build_ui_frame_base()
                .controls
                .iter()
                .position(|control| control.action == action)
        }
    }

    pub fn selected_world(&self) -> Option<WorldId> {
        self.selected_world
    }

    pub fn hovered_world(&self) -> Option<WorldId> {
        self.hovered_world
    }

    pub fn command_target(&self) -> Option<WorldId> {
        self.hovered_world.or(self.touch_command_target)
    }

    pub(crate) fn clear_hovered_world(&mut self) {
        self.hovered_world = None;
    }

    pub fn selected_field(&self) -> FieldKind {
        self.selected_field
    }

    pub fn next_sequence(&self) -> u64 {
        self.next_sequence
    }

    pub fn clock_alpha(&self) -> f32 {
        self.clock.alpha()
    }

    pub fn viewport(&self) -> Vec2 {
        self.viewport
    }

    pub fn set_viewport(&mut self, viewport: Vec2) {
        if !viewport.is_finite() {
            return;
        }
        self.viewport = viewport.max(Vec2::ZERO);
        self.camera.set_viewport(self.viewport);
        if let Some(editor) = &mut self.editor {
            editor.rebuild_widgets(self.viewport);
        }
    }

    pub fn camera(&self) -> &CameraState {
        &self.camera
    }

    pub fn camera_mut(&mut self) -> &mut CameraState {
        &mut self.camera
    }

    pub fn presentation_preferences(&self) -> PresentationPreferences {
        self.presentation_preferences
    }

    pub fn set_presentation_preferences(&mut self, preferences: PresentationPreferences) {
        self.presentation_preferences = preferences;
        self.camera.set_motion(preferences.motion);
    }

    pub fn scene_frame(&self) -> SceneFrame {
        build_scene_frame(
            self.simulation.state(),
            self.active_scenario.fingerprint(),
            self.clock.alpha(),
            self.view_state(),
            self.advisory_snapshot(),
            self.visual_time,
            &self.camera,
            self.presentation_preferences,
        )
    }

    pub fn pick_scene_world(&self, pointer: Vec2, hud_consumed: bool) -> Option<WorldId> {
        let frame = self.scene_frame();
        let ray = SceneRay::from_logical_pointer(
            pointer,
            self.viewport,
            self.camera.current_matrices.inverse_view_projection,
        )?;
        pick_world_after_hud(hud_consumed, ray, &frame.worlds).map(|hit| hit.world)
    }

    pub fn select_scene_world(&mut self, pointer: Vec2, hud_consumed: bool) -> bool {
        if self.mode != AppMode::Playing || hud_consumed {
            return false;
        }
        self.selected_world = self.pick_scene_world(pointer, false);
        self.touch_command_target = None;
        self.ui_focus_action = None;
        true
    }

    pub fn hover_scene_world(&mut self, pointer: Vec2, hud_consumed: bool) {
        if self.mode == AppMode::Playing {
            self.hovered_world = self.pick_scene_world(pointer, hud_consumed);
        }
    }

    pub fn zoom_camera(&mut self, logical_delta: f32) {
        self.camera.zoom(logical_delta);
    }

    pub fn reset_camera(&mut self) {
        self.camera.reset();
    }

    pub fn preview_contextual_launch(
        &self,
    ) -> Option<
        Result<crate::game::simulation::CommandPreview, crate::game::simulation::CommandRejection>,
    > {
        Some(self.simulation.preview_command(GameCommand::Launch {
            source: self.selected_world?,
            destination: self.command_target()?,
        }))
    }

    pub fn command_tray(&self) -> CommandTrayState {
        let source = self
            .selected_world
            .and_then(|id| self.simulation.state().worlds.get(usize::from(id.0)))
            .map_or_else(|| "NONE".to_owned(), |world| world.name.to_string());
        let destination = self
            .command_target()
            .and_then(|id| self.simulation.state().worlds.get(usize::from(id.0)))
            .map_or_else(|| "NONE".to_owned(), |world| world.name.to_string());
        match self.preview_contextual_launch() {
            Some(Ok(preview)) => CommandTrayState {
                source,
                destination,
                launch_strength: preview
                    .launch_strength
                    .map_or_else(|| "-".to_owned(), |strength| strength.0.to_string()),
                status: format!(
                    "{} - EXPLICIT LAUNCH REQUIRED",
                    crate::ui::text::COMMAND_READY
                ),
                launch_enabled: true,
            },
            Some(Err(reason)) => CommandTrayState {
                source,
                destination,
                launch_strength: "-".to_owned(),
                status: command_rejection_label(reason).to_owned(),
                launch_enabled: false,
            },
            None => CommandTrayState {
                source,
                destination,
                launch_strength: "-".to_owned(),
                status: "SELECT SOURCE AND TARGET".to_owned(),
                launch_enabled: false,
            },
        }
    }

    fn build_ui_frame_base(&self) -> UiFrame {
        let command_tray = self.command_tray();
        let decrease_field = self.field_control_state(FieldAdjustment::Decrease);
        let increase_field = self.field_control_state(FieldAdjustment::Increase);
        let (progress, count) = self.onboarding.progress();
        let mut frame = build_ui_frame(UiBuildState {
            viewport: self.viewport,
            ui_scale: self.preferences.ui_scale,
            screen: if self.mode == AppMode::Settings {
                UiScreen::Settings
            } else {
                UiScreen::Playing
            },
            focus_index: None,
            speed_multiplier: self.game_speed.multiplier(),
            help_visible: self.help_visible,
            selected_field: self.selected_field,
            decrease_field,
            increase_field,
            pointer: Some(self.pointer_position),
            onboarding: self.onboarding.step().map(|step| (step, progress, count)),
            preferences: self.preferences,
            command_tray: &command_tray,
        });
        if self.mode == AppMode::Playing
            && let Some(marker) = crate::ui::start_marker::start_marker(
                self.onboarding_step(),
                self.simulation.state(),
                &self.scene_frame(),
                &frame,
            )
        {
            frame.controls.push(crate::presentation::ui::UiControl {
                action: UiAction::SelectGuidanceWorld(marker.world),
                bounds: crate::presentation::ui::UiRect::from_xywh(
                    marker.label_min.x,
                    marker.label_min.y,
                    180.0,
                    54.0,
                ),
                label: "START HERE",
                tooltip: "Select your Union world to begin the tutorial",
                icon: None,
                enabled: true,
                focused: false,
                selected: false,
            });
        }
        if frame.tooltip.is_none()
            && let Some(world) = self
                .hovered_world
                .and_then(|id| self.simulation.state().worlds.get(usize::from(id.0)))
        {
            frame.tooltip = Some(format!(
                "WORLD {} - SELECT OR PREVIEW DESTINATION",
                world.name
            ));
        }
        frame
    }

    pub fn ui_frame(&self) -> UiFrame {
        let mut frame = self.build_ui_frame_base();
        if self.mode == AppMode::Settings {
            if let Some(control) = frame.controls.get_mut(self.settings.focus_index()) {
                control.focused = true;
            }
        } else if let Some(action) = self.ui_focus_action
            && let Some(control) = frame
                .controls
                .iter_mut()
                .find(|control| control.action == action)
        {
            control.focused = true;
        }
        frame
    }

    fn field_control_state(&self, adjustment: FieldAdjustment) -> (bool, &'static str) {
        let Some(world) = self.selected_world else {
            return (false, "Select a Union world");
        };
        let command = GameCommand::TuneField {
            world,
            field: self.selected_field,
            adjustment,
        };
        match self.simulation.preview_command(command) {
            Ok(_) => (
                true,
                match adjustment {
                    FieldAdjustment::Decrease => "Decrease selected field (Q)",
                    FieldAdjustment::Increase => "Increase selected field (E)",
                },
            ),
            Err(reason) => (false, command_rejection_label(reason)),
        }
    }

    pub fn advisory_snapshot(&self) -> Option<&AdvisorySnapshot> {
        self.advisory.published()
    }

    pub fn advisory_updating(&self) -> bool {
        self.advisory.has_in_flight_request() || self.advisory.has_queued_request()
    }

    pub fn recoverable_message(&self) -> Option<&str> {
        self.recoverable_message
            .as_deref()
            .or(self.preference_message.as_deref())
    }

    pub fn view_state(&self) -> ViewState {
        ViewState {
            selected_world: self.selected_world,
            hovered_world: self.hovered_world,
            selected_field: self.selected_field,
            paused: self.paused(),
            speed_multiplier: self.game_speed.multiplier(),
            help_visible: self.help_visible,
        }
    }

    pub fn set_recoverable_message(&mut self, message: impl Into<String>) {
        self.recoverable_message = Some(message.into());
    }

    pub fn clear_recoverable_message(&mut self) {
        self.recoverable_message = None;
    }

    pub fn handle_action(&mut self, action: Action) -> bool {
        if self.mode != AppMode::Playing {
            return false;
        }
        match action {
            Action::Field1 => self.selected_field = FieldKind::Atmosphere,
            Action::Field2 => self.selected_field = FieldKind::Hydrosphere,
            Action::Field3 => self.selected_field = FieldKind::Topology,
            Action::Pause => {
                if self.paused() {
                    self.set_game_speed(self.resume_speed);
                } else {
                    self.set_game_speed(GameSpeed::Paused);
                }
            }
            Action::SpeedDown => self.set_game_speed(self.game_speed.slower()),
            Action::SpeedUp => self.set_game_speed(self.game_speed.faster()),
            Action::CameraReset => self.reset_camera(),
            Action::Settings => self.open_settings(),
            Action::Restart => self.restart_active_scenario(),
            Action::Help => self.help_visible = !self.help_visible,
            Action::Cancel => {
                self.clear_scene_selection();
            }
            Action::Scenario => {
                self.observe_onboarding(OnboardingEvent::ScenarioEditorOpened);
                self.open_editor();
            }
            Action::Decrease | Action::Increase | Action::Launch => {
                let Some(command) = game_command_for_action(
                    action,
                    self.selected_world,
                    self.hovered_world,
                    self.selected_field,
                ) else {
                    return false;
                };
                return self.queue_game_command(command);
            }
        }
        true
    }

    fn launch_contextual_preview(&mut self) -> bool {
        let (Some(source), Some(destination)) = (self.selected_world, self.command_target()) else {
            return false;
        };
        self.queue_game_command(GameCommand::Launch {
            source,
            destination,
        })
    }

    fn clear_scene_selection(&mut self) {
        self.selected_world = None;
        self.hovered_world = None;
        self.touch_command_target = None;
        self.ui_focus_action = None;
    }

    fn world_is_union_owned(&self, id: WorldId) -> bool {
        self.simulation
            .state()
            .worlds
            .get(usize::from(id.0))
            .is_some_and(|world| world.owner == Some(Faction::Union))
    }

    pub fn set_cursor(&mut self, cursor: Vec2) {
        self.set_scene_cursor(cursor, false);
    }

    pub fn set_scene_cursor(&mut self, cursor: Vec2, hud_consumed: bool) {
        if !cursor.is_finite() {
            return;
        }
        self.pointer_position = cursor;
        if self.mode == AppMode::Playing {
            self.hovered_world = self.pick_scene_world(cursor, hud_consumed);
            if self
                .preview_contextual_launch()
                .is_some_and(|preview| preview.is_ok())
            {
                self.observe_onboarding(OnboardingEvent::LaunchPreviewed);
            }
        }
    }

    pub fn handle_primary_pointer(&mut self, cursor: Vec2) -> bool {
        self.handle_scene_tap(cursor, false)
    }

    pub fn handle_scene_tap(&mut self, cursor: Vec2, touch: bool) -> bool {
        if self.mode != AppMode::Playing {
            return false;
        }
        let target = self.pick_scene_world(cursor, false);
        self.ui_focus_action = None;
        if touch {
            let friendly_target = target.is_some_and(|id| self.world_is_union_owned(id));
            if target.is_none() {
                self.clear_scene_selection();
                return true;
            }
            if friendly_target {
                self.selected_world = target;
                self.hovered_world = target;
                self.touch_command_target = None;
                self.observe_onboarding(OnboardingEvent::UnionWorldSelected);
                return true;
            }
            if self
                .selected_world
                .is_some_and(|id| self.world_is_union_owned(id))
            {
                self.hovered_world = target;
                self.touch_command_target = target;
                if self
                    .preview_contextual_launch()
                    .is_some_and(|preview| preview.is_ok())
                {
                    self.observe_onboarding(OnboardingEvent::LaunchPreviewed);
                }
                return true;
            }
            self.selected_world = None;
            self.hovered_world = target;
            self.touch_command_target = None;
            return true;
        }
        self.selected_world = target;
        self.hovered_world = target;
        self.touch_command_target = None;
        if target.is_some_and(|id| self.world_is_union_owned(id)) {
            self.observe_onboarding(OnboardingEvent::UnionWorldSelected);
        }
        true
    }

    pub fn handle_scene_launch_pointer(&mut self, cursor: Vec2) -> bool {
        if self.mode != AppMode::Playing {
            return false;
        }
        let Some(destination) = self.pick_scene_world(cursor, false) else {
            return false;
        };
        let Some(source) = self.selected_world else {
            return false;
        };
        self.hovered_world = Some(destination);
        self.touch_command_target = None;
        self.queue_game_command(GameCommand::Launch {
            source,
            destination,
        })
    }

    pub fn handle_launch_pointer(&mut self, cursor: Vec2) -> bool {
        self.handle_scene_launch_pointer(cursor)
    }

    pub fn queue_game_command(&mut self, command: GameCommand) -> bool {
        if self.mode != AppMode::Playing {
            return false;
        }
        let preview_valid = self.simulation.preview_command(command).is_ok();
        let sequence = CommandSequence(self.next_sequence);
        if self.simulation.enqueue_next(sequence, command).is_err() {
            return false;
        }
        self.next_sequence = self.next_sequence.wrapping_add(1);
        match command {
            GameCommand::Launch { .. } if preview_valid => {
                self.touch_command_target = None;
                self.observe_onboarding(OnboardingEvent::FleetLaunched);
            }
            GameCommand::TuneField { .. } if preview_valid => {
                self.observe_onboarding(OnboardingEvent::FieldTuned);
            }
            GameCommand::Launch { .. } | GameCommand::TuneField { .. } => {}
        }
        true
    }

    pub fn advance(&mut self, delta: Duration) {
        self.camera.update(delta);
        if self.presentation_preferences.motion == crate::presentation::MotionPreference::Full {
            self.visual_time = (self.visual_time + delta.as_secs_f32()).rem_euclid(4096.0);
        }
        if self.mode != AppMode::Playing || self.paused() {
            self.clock.clear();
            return;
        }
        for _ in 0..self.game_speed.multiplier() {
            self.clock.push_frame(delta);
            self.take_clock_steps();
        }
    }

    fn take_clock_steps(&mut self) {
        while self.clock.take_step() {
            let report = self.simulation.step();
            let material = report
                .command_results
                .iter()
                .any(|result| matches!(result, CommandResult::Accepted { .. }))
                || !report.events.is_empty();
            self.schedule_advisory(if material {
                AdvisoryTrigger::MaterialEvent
            } else {
                AdvisoryTrigger::Tick
            });
        }
    }

    pub fn open_editor(&mut self) {
        if self.mode == AppMode::EditingScenario {
            return;
        }
        let mut editor = EditorState::new(self.active_scenario.to_draft());
        editor.rebuild_widgets(self.viewport);
        editor.focus_first();
        self.editor = Some(editor);
        self.mode = AppMode::EditingScenario;
        self.clock.clear();
    }

    pub fn handle_editor_action(&mut self, action: EditorAction) -> Result<(), AppTransitionError> {
        if self.mode != AppMode::EditingScenario {
            return Ok(());
        }
        match action {
            EditorAction::SelectSection(section) => self
                .editor
                .as_mut()
                .expect("editor mode")
                .select_section(section),
            EditorAction::SelectWorld(index) => self
                .editor
                .as_mut()
                .expect("editor mode")
                .select_world(index),
            EditorAction::Revert => self.editor.as_mut().expect("editor mode").request_revert(),
            EditorAction::FactoryDefaults => self
                .editor
                .as_mut()
                .expect("editor mode")
                .request_factory_defaults(),
            EditorAction::RegenerateFromSeed => self
                .editor
                .as_mut()
                .expect("editor mode")
                .request_regenerate_from_seed(),
            EditorAction::Load => {
                self.editor
                    .as_mut()
                    .expect("editor mode")
                    .request_load(&self.store)?;
            }
            EditorAction::Save => {
                self.editor
                    .as_mut()
                    .expect("editor mode")
                    .request_save(&mut self.store)?;
            }
            EditorAction::Cancel => {
                if self.editor.as_mut().expect("editor mode").request_cancel() {
                    self.close_editor();
                }
            }
            EditorAction::ApplyAndRestart => self
                .editor
                .as_mut()
                .expect("editor mode")
                .request_apply_and_restart()?,
            EditorAction::DismissConfirmation => self
                .editor
                .as_mut()
                .expect("editor mode")
                .cancel_confirmation(),
            EditorAction::Confirm => {
                let confirmed = self
                    .editor
                    .as_mut()
                    .expect("editor mode")
                    .confirm(&mut self.store)?;
                match confirmed {
                    Some(ConfirmedEditorAction::Cancel) => self.close_editor(),
                    Some(ConfirmedEditorAction::ApplyAndRestart(scenario)) => {
                        self.apply_validated_scenario(scenario)
                    }
                    None => {}
                }
            }
        }
        Ok(())
    }

    pub fn apply_validated_scenario(&mut self, scenario: Box<ScenarioV1>) {
        let simulation = Simulation::from_scenario(&scenario);
        let rules = scenario.rules();
        self.active_scenario = *scenario;
        self.simulation = simulation;
        self.clock = FixedClock::new(rules.tick_hz, MAX_STEPS_PER_FRAME);
        self.reset_session_after_restart();
        self.editor = None;
        self.mode = AppMode::Playing;
        self.schedule_advisory(AdvisoryTrigger::Restart);
    }

    pub fn restart_active_scenario(&mut self) {
        self.simulation = Simulation::from_scenario(&self.active_scenario);
        self.clock = FixedClock::new(self.active_scenario.rules().tick_hz, MAX_STEPS_PER_FRAME);
        self.reset_session_after_restart();
        self.schedule_advisory(AdvisoryTrigger::Restart);
    }

    pub fn on_suspend(&mut self) {
        self.clock.clear();
    }

    pub fn on_resume(&mut self) {
        self.clock.clear();
    }

    pub fn set_gpu_epoch(&mut self, device_epoch: u64, available: bool) {
        self.pending_gpu_request = None;
        self.advisory.set_device_epoch(device_epoch, available);
        self.schedule_advisory(AdvisoryTrigger::Restart);
    }

    pub fn take_gpu_request(&mut self) -> Option<AdvisoryRequest> {
        self.pending_gpu_request.take()
    }

    pub fn complete_gpu(
        &mut self,
        metadata: crate::advisory::AdvisoryMetadata,
        result: Result<[f32; crate::advisory::SCORE_COUNT], GpuAdvisoryError>,
    ) -> CompletionOutcome {
        let outcome = self.advisory.complete_gpu(metadata, result);
        if let Some(request) = &outcome.gpu_request {
            self.pending_gpu_request = Some(request.clone());
        }
        outcome
    }

    pub fn fail_device_epoch(&mut self, message: impl Into<String>) -> CompletionOutcome {
        self.pending_gpu_request = None;
        self.advisory
            .fail_device_epoch(GpuAdvisoryError::Device(message.into()))
    }

    fn close_editor(&mut self) {
        self.editor = None;
        self.mode = AppMode::Playing;
        self.clock.clear();
        self.recoverable_message = None;
    }

    fn reset_session_after_restart(&mut self) {
        self.selected_world = None;
        self.hovered_world = None;
        self.touch_command_target = None;
        self.selected_field = FieldKind::Atmosphere;
        self.game_speed = if self.onboarding.active() {
            GameSpeed::Paused
        } else {
            GameSpeed::Normal
        };
        self.resume_speed = GameSpeed::Normal;
        self.help_visible = false;
        self.ui_focus_action = None;
        self.next_sequence = 0;
        self.clock.clear();
        self.camera.reset();
        self.visual_time = 0.0;
    }

    fn observe_onboarding(&mut self, event: OnboardingEvent) {
        if self.onboarding.observe(event) {
            self.preferences.onboarding_completed = true;
            if self.game_speed == GameSpeed::Paused {
                self.set_game_speed(self.resume_speed);
            }
            self.persist_preferences();
        }
    }

    fn apply_preferences(&mut self) {
        self.presentation_preferences = self.preferences.presentation();
        self.camera.set_motion(self.preferences.motion);
    }

    fn persist_preferences(&mut self) {
        let outcome = save_or_default(&mut self.preference_store, &self.preferences);
        self.preference_message = outcome.recoverable_message();
    }

    fn schedule_advisory(&mut self, trigger: AdvisoryTrigger) {
        if let Some(outcome) = self.advisory.request_if_due(
            self.simulation.state(),
            self.simulation.canonical_fingerprint(),
            trigger,
        ) && let Some(request) = outcome.gpu_request
        {
            self.pending_gpu_request = Some(request);
        }
    }
}

pub fn source_tick(snapshot: Option<&AdvisorySnapshot>) -> Tick {
    snapshot.map_or(Tick(0), |snapshot| snapshot.metadata.source_tick)
}

const fn command_rejection_label(
    rejection: crate::game::simulation::CommandRejection,
) -> &'static str {
    use crate::game::simulation::CommandRejection;
    match rejection {
        CommandRejection::WrongTick => "REJECTED - WRONG TICK",
        CommandRejection::DuplicateSequence => "REJECTED - DUPLICATE COMMAND",
        CommandRejection::CampaignFinished => "REJECTED - CAMPAIGN FINISHED",
        CommandRejection::InvalidWorld => "REJECTED - INVALID WORLD",
        CommandRejection::SameSourceAndTarget => "REJECTED - CHOOSE ANOTHER TARGET",
        CommandRejection::SourceNotOwnedByIssuer => "REJECTED - SOURCE IS NOT UNION",
        CommandRejection::InsufficientEnergy => "REJECTED - INSUFFICIENT ENERGY",
        CommandRejection::FieldAtLimit => "REJECTED - FIELD AT LIMIT",
    }
}
