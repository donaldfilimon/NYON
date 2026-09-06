//! Pure command-deck UI layout, hit testing, and SDF batch construction.

use glam::Vec2;

use crate::{
    app::{onboarding::OnboardingStep, settings::SettingsAction},
    engine::primitives::PrimitiveBatch,
    game::model::FieldKind,
    preferences::{UiScale, UserPreferencesV1},
    presentation::{GraphicsQuality, MotionPreference},
    ui::{self, AtlasMetrics, FontWeight, UiBatch, UiBatchError, UiIcon},
};

pub const MIN_CONTROL_EXTENT: f32 = 44.0;
pub const WIDE_UI_BREAKPOINT: f32 = 1_100.0;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum UiScreen {
    Playing,
    Settings,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum UiAction {
    SelectGuidanceWorld(crate::game::model::WorldId),
    Launch,
    ClearSelection,
    SpeedDown,
    TogglePause,
    SpeedUp,
    ResetCamera,
    OpenSettings,
    ToggleHelp,
    OpenScenario,
    AcknowledgeAdvisory,
    SkipOnboarding,
    ResetOnboarding,
    SelectField(FieldKind),
    DecreaseField,
    IncreaseField,
    Settings(SettingsAction),
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct UiRect {
    pub min: Vec2,
    pub max: Vec2,
}

impl UiRect {
    pub fn from_xywh(x: f32, y: f32, width: f32, height: f32) -> Self {
        Self {
            min: Vec2::new(x, y),
            max: Vec2::new(x + width.max(0.0), y + height.max(0.0)),
        }
    }

    pub fn contains(self, point: Vec2) -> bool {
        point.is_finite() && point.cmpge(self.min).all() && point.cmple(self.max).all()
    }

    pub fn size(self) -> Vec2 {
        self.max - self.min
    }

    pub fn center(self) -> Vec2 {
        (self.min + self.max) * 0.5
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct UiControl {
    pub action: UiAction,
    pub bounds: UiRect,
    pub label: &'static str,
    pub tooltip: &'static str,
    pub icon: Option<UiIcon>,
    pub enabled: bool,
    pub focused: bool,
    pub selected: bool,
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct CommandTrayState {
    pub source: String,
    pub destination: String,
    pub launch_strength: String,
    pub status: String,
    pub launch_enabled: bool,
}

#[derive(Clone, Copy, Debug)]
pub struct UiBuildState<'a> {
    pub viewport: Vec2,
    pub ui_scale: UiScale,
    pub screen: UiScreen,
    pub focus_index: Option<usize>,
    pub speed_multiplier: u8,
    pub help_visible: bool,
    pub selected_field: FieldKind,
    pub decrease_field: (bool, &'static str),
    pub increase_field: (bool, &'static str),
    pub pointer: Option<Vec2>,
    pub onboarding: Option<(OnboardingStep, usize, usize)>,
    pub preferences: UserPreferencesV1,
    pub command_tray: &'a CommandTrayState,
}

#[derive(Clone, Debug, PartialEq)]
pub struct UiFrame {
    pub viewport: Vec2,
    pub screen: UiScreen,
    pub wide: bool,
    pub scale: f32,
    pub controls: Vec<UiControl>,
    pub command_lines: Vec<String>,
    pub onboarding_lines: Vec<String>,
    pub high_contrast: bool,
    pub help_visible: bool,
    pub tooltip: Option<String>,
}

impl UiFrame {
    fn command_tray_rect(&self) -> UiRect {
        let width =
            bounded_panel_width(448.0 * self.scale, 344.0, (self.viewport.x - 16.0).max(1.0));
        let height = (108.0 * self.scale).max(92.0);
        UiRect::from_xywh(
            (self.viewport.x - width) * 0.5,
            (70.0 * self.scale).max(52.0),
            width,
            height,
        )
    }

    fn onboarding_rect(&self) -> UiRect {
        let width = bounded_panel_width(
            448.0 * self.scale,
            344.0,
            (self.viewport.x - 132.0).max(1.0),
        );
        UiRect::from_xywh(
            (self.viewport.x - width) * 0.5,
            8.0,
            width,
            (52.0 * self.scale).max(44.0),
        )
    }

    fn tooltip_rect(&self) -> UiRect {
        let width = (620.0 * self.scale).min((self.viewport.x - 16.0).max(1.0));
        let bottom_bar_top = self
            .controls
            .iter()
            .filter(|control| control.bounds.max.y >= self.viewport.y - 120.0)
            .map(|control| control.bounds.min.y)
            .fold(self.viewport.y, f32::min);
        UiRect::from_xywh(8.0, (bottom_bar_top - 34.0).max(0.0), width, 30.0)
    }

    pub fn hit_test(&self, point: Vec2) -> Option<UiAction> {
        self.controls
            .iter()
            .find(|control| control.enabled && control.bounds.contains(point))
            .map(|control| control.action)
    }

    pub fn tooltip_at(&self, point: Vec2) -> Option<&str> {
        self.controls
            .iter()
            .find(|control| control.bounds.contains(point))
            .map(|control| control.tooltip)
    }

    pub fn consumes_pointer(&self, point: Vec2) -> bool {
        if !point.is_finite() {
            return true;
        }
        if self.screen == UiScreen::Settings || self.hit_test(point).is_some() {
            return true;
        }
        // Legacy chrome uses `GameLayout`'s 0.72 minimum scale even when the
        // retained controls can render a smaller 85% compact layout. Consume
        // the complete underlay so gaps between controls never leak scene
        // hover or selection through an opaque panel.
        let legacy_scale = self.scale.max(0.72);
        let bottom = if self.viewport.x < WIDE_UI_BREAKPOINT {
            102.0
        } else {
            56.0 * legacy_scale
        };
        let in_fixed_hud = point.x <= 240.0 * legacy_scale
            || point.x >= self.viewport.x - 280.0 * legacy_scale
            || point.y <= 64.0 * legacy_scale
            || point.y >= self.viewport.y - bottom;
        let in_command_tray = self.command_tray_rect().contains(point);
        let in_help = self.help_visible
            && UiRect::from_xywh(
                self.viewport.x * 0.5 - 190.0,
                self.viewport.y * 0.5 - 90.0,
                380.0,
                180.0,
            )
            .contains(point);
        in_fixed_hud || in_command_tray || in_help
    }
}

fn bounded_panel_width(desired: f32, minimum: f32, maximum: f32) -> f32 {
    if maximum >= minimum {
        desired.clamp(minimum, maximum)
    } else {
        maximum
    }
}

pub fn build_ui_frame(state: UiBuildState<'_>) -> UiFrame {
    let viewport = if state.viewport.is_finite() {
        state.viewport.max(Vec2::ONE)
    } else {
        Vec2::ONE
    };
    let wide = viewport.x >= WIDE_UI_BREAKPOINT;
    let scale = ((viewport.x / 1440.0)
        .min(viewport.y / 900.0)
        .clamp(0.72, 1.15)
        * state.ui_scale.factor())
    .clamp(0.60, 1.50);
    let mut controls = match state.screen {
        UiScreen::Playing => playing_controls(viewport, scale, state),
        UiScreen::Settings => settings_controls(viewport, scale, state.preferences),
    };
    for (index, control) in controls.iter_mut().enumerate() {
        control.focused = state.focus_index == Some(index);
    }

    let command_lines = if state.screen == UiScreen::Playing {
        vec![
            format!("SOURCE {}", state.command_tray.source),
            format!("TARGET {}", state.command_tray.destination),
            format!("STRENGTH {}", state.command_tray.launch_strength),
            state.command_tray.status.clone(),
        ]
    } else {
        Vec::new()
    };
    let onboarding_lines = state
        .onboarding
        .map(|(step, index, count)| {
            vec![
                format!("TUTORIAL {index}/{count} // PAUSED"),
                step.title().to_owned(),
            ]
        })
        .unwrap_or_default();
    let tooltip = state.pointer.and_then(|point| {
        controls
            .iter()
            .find(|control| control.bounds.contains(point))
            .map(|control| control.tooltip.to_owned())
    });

    UiFrame {
        viewport,
        screen: state.screen,
        wide,
        scale,
        controls,
        command_lines,
        onboarding_lines,
        high_contrast: state.preferences.high_contrast,
        help_visible: state.help_visible,
        tooltip,
    }
}

pub fn build_ui_batch(
    frame: &UiFrame,
    metrics: &AtlasMetrics,
    batch: &mut UiBatch,
) -> Result<(), UiBatchError> {
    batch.clear();
    let panel = if frame.high_contrast {
        [0.0, 0.0, 0.0, 0.98]
    } else {
        [0.015, 0.035, 0.075, 0.985]
    };
    if frame.screen == UiScreen::Settings {
        batch.push_panel(
            [0.0, 0.0, frame.viewport.x, frame.viewport.y],
            [0.01, 0.02, 0.05, 0.97],
        )?;
    }
    if !frame.command_lines.is_empty() {
        let tray = frame.command_tray_rect();
        let size = tray.size();
        batch.push_panel([tray.min.x, tray.min.y, size.x, size.y], panel)?;
        batch.push_panel(
            [tray.min.x, tray.min.y, (4.0 * frame.scale).max(3.0), size.y],
            [0.20, 0.78, 1.0, 1.0],
        )?;
    }
    if !frame.onboarding_lines.is_empty() {
        let guidance = frame.onboarding_rect();
        let size = guidance.size();
        batch.push_panel(
            [guidance.min.x, guidance.min.y, size.x, size.y],
            [0.025, 0.075, 0.13, 0.99],
        )?;
        batch.push_panel(
            [guidance.min.x, guidance.max.y - 3.0, size.x, 3.0],
            [1.0, 0.72, 0.20, 1.0],
        )?;
    }
    if frame.tooltip.is_some() {
        let tooltip = frame.tooltip_rect();
        let size = tooltip.size();
        batch.push_panel([tooltip.min.x, tooltip.min.y, size.x, size.y], panel)?;
    }
    let text = if frame.high_contrast {
        [1.0, 1.0, 1.0, 1.0]
    } else {
        [0.76, 0.94, 1.0, 1.0]
    };
    for control in &frame.controls {
        // The advisory's existing primitive content remains the canonical
        // readable panel for this slice. This control contributes only its hit
        // target and final-stage focus ring, avoiding a second panel painted on
        // top of the advisory text.
        if control.action == UiAction::AcknowledgeAdvisory {
            continue;
        }
        let fill = if control.selected {
            [0.04, 0.42, 0.58, 1.0]
        } else if control.enabled {
            [0.025, 0.075, 0.13, 0.99]
        } else {
            [0.018, 0.028, 0.045, 0.96]
        };
        let size = control.bounds.size();
        batch.push_panel(
            [control.bounds.min.x, control.bounds.min.y, size.x, size.y],
            fill,
        )?;
        let color = if control.enabled {
            text
        } else {
            [0.48, 0.58, 0.66, 1.0]
        };
        let font_size = if matches!(control.action, UiAction::SelectGuidanceWorld(_)) {
            26.0
        } else {
            (13.0 * frame.scale).max(12.0)
        };
        let baseline = [
            control.bounds.min.x + 8.0,
            control.bounds.center().y + font_size * 0.35,
        ];
        if let Some(icon) = control.icon {
            let extent = (22.0 * frame.scale).clamp(18.0, 26.0);
            batch.push_icon(
                metrics,
                icon,
                [
                    control.bounds.min.x + 6.0,
                    control.bounds.center().y - extent * 0.5,
                    extent,
                    extent,
                ],
                color,
            )?;
            let text_baseline = [baseline[0] + extent + 5.0, baseline[1]];
            batch.push_text(
                metrics,
                text_baseline,
                font_size,
                FontWeight::SemiBold,
                color,
                control.label,
            )?;
        } else {
            batch.push_text(
                metrics,
                baseline,
                font_size,
                FontWeight::SemiBold,
                color,
                control.label,
            )?;
        }
    }

    for (index, line) in frame.command_lines.iter().enumerate() {
        let tray = frame.command_tray_rect();
        let color = if index == 3 {
            if line.contains("READY") {
                [0.32, 0.95, 0.72, 1.0]
            } else {
                [1.0, 0.78, 0.30, 1.0]
            }
        } else {
            text
        };
        batch.push_text(
            metrics,
            [
                tray.min.x + 20.0 * frame.scale,
                tray.min.y + 22.0 * frame.scale + index as f32 * 20.0 * frame.scale,
            ],
            (14.0 * frame.scale).max(11.0),
            if index == 3 {
                FontWeight::SemiBold
            } else {
                FontWeight::Regular
            },
            color,
            line,
        )?;
    }
    for (index, line) in frame.onboarding_lines.iter().enumerate() {
        let guidance = frame.onboarding_rect();
        batch.push_text(
            metrics,
            [
                guidance.min.x + 20.0 * frame.scale,
                guidance.min.y + 20.0 * frame.scale + index as f32 * 19.0 * frame.scale,
            ],
            (if index == 0 { 12.0 } else { 15.0 } * frame.scale).max(10.0),
            FontWeight::SemiBold,
            if index == 0 {
                [0.38, 0.82, 1.0, 1.0]
            } else {
                [1.0, 0.92, 0.68, 1.0]
            },
            line,
        )?;
    }
    if let Some(tooltip) = &frame.tooltip {
        let tooltip_rect = frame.tooltip_rect();
        batch.push_text(
            metrics,
            [tooltip_rect.min.x + 10.0, tooltip_rect.min.y + 19.0],
            (13.0 * frame.scale).max(10.0),
            FontWeight::Regular,
            [1.0, 0.88, 0.32, 1.0],
            tooltip,
        )?;
    }
    Ok(())
}

pub fn append_ui_primitives(frame: &UiFrame, batch: &mut PrimitiveBatch) {
    for control in &frame.controls {
        if control.focused {
            let center = control.bounds.center();
            let half = control.bounds.size() * 0.5;
            let color = if frame.high_contrast {
                [1.0, 1.0, 0.0, 1.0]
            } else {
                [0.25, 0.82, 1.0, 1.0]
            };
            batch.ring(center, half.x.min(half.y), 3.0, color);
        }
    }
}

fn playing_controls(viewport: Vec2, scale: f32, state: UiBuildState<'_>) -> Vec<UiControl> {
    let extent = (48.0 * scale).max(MIN_CONTROL_EXTENT);
    let gap = 6.0;
    let specs = [
        (
            UiAction::SpeedDown,
            ui::text::SPEED_DOWN,
            "Decrease game speed ([)",
            Some(UiIcon::Speed),
            state.onboarding.is_none(),
            false,
        ),
        (
            UiAction::TogglePause,
            if state.speed_multiplier == 0 {
                ui::text::PLAY
            } else {
                ui::text::PAUSE
            },
            "Pause or resume (P)",
            Some(if state.speed_multiplier == 0 {
                UiIcon::Play
            } else {
                UiIcon::Pause
            }),
            state.onboarding.is_none(),
            state.speed_multiplier == 0,
        ),
        (
            UiAction::SpeedUp,
            ui::text::SPEED_UP,
            "Increase game speed (])",
            Some(UiIcon::Speed),
            state.onboarding.is_none(),
            false,
        ),
        (
            UiAction::Launch,
            ui::text::LAUNCH,
            "Launch the command tray preview",
            Some(UiIcon::Crosshair),
            state.command_tray.launch_enabled,
            false,
        ),
        (
            UiAction::ClearSelection,
            "CLEAR",
            "Clear source and destination (Escape)",
            Some(UiIcon::Close),
            state.command_tray.source != "NONE" || state.command_tray.destination != "NONE",
            false,
        ),
        (
            UiAction::ResetCamera,
            ui::text::RESET,
            "Reset tactical camera (C)",
            Some(UiIcon::Reset),
            true,
            false,
        ),
        (
            UiAction::OpenSettings,
            ui::text::SETTINGS,
            "Open local settings",
            Some(UiIcon::Settings),
            true,
            false,
        ),
        (
            UiAction::ToggleHelp,
            ui::text::HELP,
            "Show controls and guidance",
            Some(UiIcon::Help),
            true,
            state.help_visible,
        ),
        (
            UiAction::OpenScenario,
            ui::text::SCENARIO,
            "Open detached scenario editor (F4)",
            None,
            true,
            false,
        ),
    ];
    let columns = if viewport.x >= WIDE_UI_BREAKPOINT {
        specs.len()
    } else {
        5
    };
    let rows = specs.len().div_ceil(columns);
    let available_width = (viewport.x - 16.0 - (columns - 1) as f32 * gap) / columns as f32;
    let width = if columns == specs.len() {
        (92.0 * scale).max(72.0).min(available_width)
    } else {
        (124.0 * scale).max(104.0).min(available_width)
    };
    let start_y = (viewport.y - rows as f32 * extent - (rows - 1) as f32 * gap - 4.0).max(0.0);
    let mut controls = specs
        .into_iter()
        .enumerate()
        .map(
            |(index, (action, label, tooltip, icon, enabled, selected))| UiControl {
                action,
                bounds: {
                    let row = index / columns;
                    let column = index % columns;
                    let row_count = (specs.len() - row * columns).min(columns);
                    let row_width = row_count as f32 * width + (row_count - 1) as f32 * gap;
                    let row_start = ((viewport.x - row_width) * 0.5).max(4.0);
                    UiRect::from_xywh(
                        row_start + column as f32 * (width + gap),
                        start_y + row as f32 * (extent + gap),
                        width,
                        extent,
                    )
                },
                label,
                tooltip,
                icon,
                enabled,
                focused: false,
                selected,
            },
        )
        .collect::<Vec<_>>();
    if state.onboarding.is_some() {
        controls.push(UiControl {
            action: UiAction::SkipOnboarding,
            bounds: UiRect::from_xywh(viewport.x - 116.0, 8.0, 108.0, MIN_CONTROL_EXTENT),
            label: ui::text::SKIP,
            tooltip: "Skip tutorial and play this match now",
            icon: Some(UiIcon::Close),
            enabled: true,
            focused: false,
            selected: false,
        });
    }
    let field_width = (72.0 * scale).max(MIN_CONTROL_EXTENT);
    let field_height = (44.0 * scale).max(MIN_CONTROL_EXTENT);
    let right_x = (viewport.x - 268.0 * scale).max(4.0);
    for (index, (field, label, icon)) in [
        (FieldKind::Atmosphere, "ATM", UiIcon::FieldAtmosphere),
        (FieldKind::Hydrosphere, "HYD", UiIcon::FieldHydrosphere),
        (FieldKind::Topology, "TOP", UiIcon::FieldTopology),
    ]
    .into_iter()
    .enumerate()
    {
        controls.push(UiControl {
            action: UiAction::SelectField(field),
            bounds: UiRect::from_xywh(
                right_x + index as f32 * (field_width + 4.0),
                236.0 * scale,
                field_width,
                field_height,
            ),
            label,
            tooltip: "Select field channel (1/2/3)",
            icon: Some(icon),
            enabled: true,
            focused: false,
            selected: state.selected_field == field,
        });
    }
    for (index, (action, label, state)) in [
        (UiAction::DecreaseField, "FIELD -", state.decrease_field),
        (UiAction::IncreaseField, "FIELD +", state.increase_field),
    ]
    .into_iter()
    .enumerate()
    {
        controls.push(UiControl {
            action,
            bounds: UiRect::from_xywh(
                right_x + index as f32 * (field_width * 1.5 + 4.0),
                286.0 * scale,
                field_width * 1.5,
                field_height,
            ),
            label,
            tooltip: state.1,
            icon: None,
            enabled: state.0,
            focused: false,
            selected: false,
        });
    }
    controls.push(UiControl {
        action: UiAction::AcknowledgeAdvisory,
        bounds: UiRect::from_xywh(
            (viewport.x - 276.0 * scale).max(0.0),
            380.0 * scale,
            (268.0 * scale).max(MIN_CONTROL_EXTENT),
            (150.0 * scale).max(MIN_CONTROL_EXTENT),
        ),
        label: ui::text::ADVISORY,
        tooltip: "Read the advisory status",
        icon: None,
        enabled: true,
        focused: false,
        selected: false,
    });
    if state.help_visible {
        controls.push(UiControl {
            action: UiAction::ResetOnboarding,
            bounds: UiRect::from_xywh(
                viewport.x * 0.5 - 110.0,
                viewport.y * 0.5 + 54.0,
                220.0,
                MIN_CONTROL_EXTENT,
            ),
            label: ui::text::RESTART_GUIDANCE,
            tooltip: "Restart first-run guidance",
            icon: Some(UiIcon::Reset),
            enabled: true,
            focused: false,
            selected: false,
        });
    }
    controls
}

fn settings_controls(viewport: Vec2, scale: f32, preferences: UserPreferencesV1) -> Vec<UiControl> {
    let target_height = (48.0 * scale).max(MIN_CONTROL_EXTENT);
    let gap = 8.0;
    let columns = if viewport.x >= WIDE_UI_BREAKPOINT {
        4
    } else {
        2
    };
    let available_width = (viewport.x - 16.0 - (columns - 1) as f32 * gap) / columns as f32;
    let width = (150.0 * scale).max(118.0).min(available_width);
    let panel_width = width * columns as f32 + gap * (columns - 1) as f32;
    let start_x = (viewport.x - panel_width) * 0.5;
    let start_y = (viewport.y * 0.5 - 130.0).max(8.0);
    let specs = [
        setting(
            SettingsAction::SetUiScale(UiScale::Percent85),
            ui::text::SCALE_85,
            preferences.ui_scale == UiScale::Percent85,
        ),
        setting(
            SettingsAction::SetUiScale(UiScale::Percent100),
            ui::text::SCALE_100,
            preferences.ui_scale == UiScale::Percent100,
        ),
        setting(
            SettingsAction::SetUiScale(UiScale::Percent115),
            ui::text::SCALE_115,
            preferences.ui_scale == UiScale::Percent115,
        ),
        setting(
            SettingsAction::SetUiScale(UiScale::Percent130),
            ui::text::SCALE_130,
            preferences.ui_scale == UiScale::Percent130,
        ),
        setting(
            SettingsAction::SetMotion(if preferences.motion == MotionPreference::Reduced {
                MotionPreference::Full
            } else {
                MotionPreference::Reduced
            }),
            ui::text::REDUCED_MOTION,
            preferences.motion == MotionPreference::Reduced,
        ),
        setting(
            SettingsAction::SetHighContrast(!preferences.high_contrast),
            ui::text::HIGH_CONTRAST,
            preferences.high_contrast,
        ),
        setting(
            SettingsAction::SetGraphicsQuality(GraphicsQuality::Auto),
            ui::text::AUTO,
            preferences.graphics_quality == GraphicsQuality::Auto,
        ),
        setting(
            SettingsAction::SetGraphicsQuality(GraphicsQuality::Low),
            ui::text::LOW,
            preferences.graphics_quality == GraphicsQuality::Low,
        ),
        setting(
            SettingsAction::SetGraphicsQuality(GraphicsQuality::High),
            ui::text::HIGH,
            preferences.graphics_quality == GraphicsQuality::High,
        ),
        setting(
            SettingsAction::ResetOnboarding,
            ui::text::RESET_GUIDANCE,
            false,
        ),
        setting(SettingsAction::Close, ui::text::CLOSE, false),
    ];
    specs
        .into_iter()
        .enumerate()
        .map(|(index, (action, label, selected))| {
            let column = index % columns;
            let row = index / columns;
            UiControl {
                action: UiAction::Settings(action),
                bounds: UiRect::from_xywh(
                    start_x + column as f32 * (width + gap),
                    start_y + row as f32 * (target_height + gap),
                    width,
                    target_height,
                ),
                label,
                tooltip: "Local offline preference",
                icon: None,
                enabled: true,
                focused: false,
                selected,
            }
        })
        .collect()
}

const fn setting(
    action: SettingsAction,
    label: &'static str,
    selected: bool,
) -> (SettingsAction, &'static str, bool) {
    (action, label, selected)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn compact_tooltip_stays_above_both_action_rows() {
        let command_tray = CommandTrayState::default();
        let mut frame = build_ui_frame(UiBuildState {
            viewport: Vec2::new(960.0, 600.0),
            ui_scale: UiScale::Percent100,
            screen: UiScreen::Playing,
            focus_index: None,
            speed_multiplier: 0,
            help_visible: false,
            selected_field: FieldKind::Atmosphere,
            decrease_field: (false, "Unavailable"),
            increase_field: (false, "Unavailable"),
            pointer: None,
            onboarding: None,
            preferences: UserPreferencesV1::default(),
            command_tray: &command_tray,
        });
        frame.tooltip = Some("Help".to_owned());
        let bottom_bar_top = frame
            .controls
            .iter()
            .filter(|control| control.bounds.max.y >= frame.viewport.y - 120.0)
            .map(|control| control.bounds.min.y)
            .fold(frame.viewport.y, f32::min);
        assert!(frame.tooltip_rect().max.y <= bottom_bar_top);
    }
}
