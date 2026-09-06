//! Pure, presentation-only layout for the Galaxy Workshop.

use std::fmt;

use glam::Vec2;

use super::platform::PlatformRect;

const MIN_UI_SCALE: f32 = 0.85;
const MAX_UI_SCALE: f32 = 1.30;
const MIN_CANVAS_WIDTH: f32 = 320.0;
const MIN_CANVAS_HEIGHT: f32 = 300.0;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum WorkshopLayoutMode {
    Compact,
    Medium,
    Wide,
}

#[derive(Clone, Debug, PartialEq)]
pub struct WorkshopLayout {
    pub viewport: PlatformRect,
    pub top_bar: PlatformRect,
    pub canvas: PlatformRect,
    pub left_panel: Option<PlatformRect>,
    pub right_panel: Option<PlatformRect>,
    pub bottom_bar: PlatformRect,
    pub drawer_sheet: PlatformRect,
    pub chrome: Vec<PlatformRect>,
    pub mode: WorkshopLayoutMode,
    pub ui_scale: f32,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum LayoutError {
    InvalidViewport,
    ViewportTooSmall,
}

impl fmt::Display for LayoutError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidViewport => {
                formatter.write_str("Workshop viewport must be finite and positive")
            }
            Self::ViewportTooSmall => {
                formatter.write_str("Workshop viewport cannot preserve a 320 by 300 canvas")
            }
        }
    }
}

impl std::error::Error for LayoutError {}

impl WorkshopLayout {
    pub fn resolve(viewport: Vec2, ui_scale: f32) -> Result<Self, LayoutError> {
        if !viewport.is_finite() || viewport.x <= 0.0 || viewport.y <= 0.0 {
            return Err(LayoutError::InvalidViewport);
        }
        let ui_scale = if ui_scale.is_finite() {
            ui_scale.clamp(MIN_UI_SCALE, MAX_UI_SCALE)
        } else {
            1.0
        };
        let top_height = 56.0 * ui_scale;
        let bottom_height = 58.0 * ui_scale;
        let working_height = viewport.y - top_height - bottom_height;
        if viewport.x < MIN_CANVAS_WIDTH || working_height < MIN_CANVAS_HEIGHT {
            return Err(LayoutError::ViewportTooSmall);
        }

        let effective_width = viewport.x / ui_scale;
        let mode = if effective_width >= 1_200.0 {
            WorkshopLayoutMode::Wide
        } else if effective_width >= 900.0 {
            WorkshopLayoutMode::Medium
        } else {
            WorkshopLayoutMode::Compact
        };
        let viewport_rect = PlatformRect::from_xywh(0.0, 0.0, viewport.x, viewport.y);
        let top_bar = PlatformRect::from_xywh(0.0, 0.0, viewport.x, top_height);
        let bottom_bar =
            PlatformRect::from_xywh(0.0, viewport.y - bottom_height, viewport.x, bottom_height);

        let (canvas, left_panel, right_panel) = match mode {
            WorkshopLayoutMode::Wide | WorkshopLayoutMode::Medium => {
                let (left_target, right_target) = if mode == WorkshopLayoutMode::Wide {
                    (280.0, 304.0)
                } else {
                    (56.0, 264.0)
                };
                let mut left_width = left_target * ui_scale;
                let mut right_width = right_target * ui_scale;
                let available_for_panels = viewport.x - MIN_CANVAS_WIDTH;
                let requested = left_width + right_width;
                if requested > available_for_panels {
                    let ratio = (available_for_panels / requested).max(0.0);
                    left_width *= ratio;
                    right_width *= ratio;
                }
                let canvas = PlatformRect::from_xywh(
                    left_width,
                    top_height,
                    viewport.x - left_width - right_width,
                    working_height,
                );
                let left = PlatformRect::from_xywh(0.0, top_height, left_width, working_height);
                let right = PlatformRect::from_xywh(
                    viewport.x - right_width,
                    top_height,
                    right_width,
                    working_height,
                );
                (canvas, Some(left), Some(right))
            }
            WorkshopLayoutMode::Compact => {
                let canvas = PlatformRect::from_xywh(0.0, top_height, viewport.x, working_height);
                (canvas, None, None)
            }
        };

        let drawer_sheet = match mode {
            WorkshopLayoutMode::Wide => left_panel.expect("wide layout has a left panel"),
            WorkshopLayoutMode::Medium => {
                let width = (420.0 * ui_scale).min(canvas.width() * 0.72).max(320.0);
                PlatformRect::from_xywh(canvas.min.x, canvas.min.y, width, canvas.height())
            }
            WorkshopLayoutMode::Compact => {
                let margin = 8.0;
                let height = (canvas.height() * 0.68).max(360.0).min(canvas.height());
                PlatformRect::from_xywh(
                    canvas.min.x + margin,
                    canvas.max.y - height,
                    (canvas.width() - margin * 2.0).max(MIN_CANVAS_WIDTH),
                    height,
                )
            }
        };

        let mut chrome = vec![top_bar, bottom_bar];
        chrome.extend(left_panel);
        chrome.extend(right_panel);
        debug_assert!(chrome.iter().all(|rect| !rect.overlaps(canvas)));

        Ok(Self {
            viewport: viewport_rect,
            top_bar,
            canvas,
            left_panel,
            right_panel,
            bottom_bar,
            drawer_sheet,
            chrome,
            mode,
            ui_scale,
        })
    }
}
