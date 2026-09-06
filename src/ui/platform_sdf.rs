//! Typed, presentation-only witnesses for platform SDF composition.

use super::{
    AtlasMetrics, FontWeight, UiBatch, UiBatchError, UiIcon,
    accessibility::{SemanticActionId, SemanticNodeId},
    platform::{PlatformBackground, PlatformRect, PlatformUiAction, PlatformUiFrame},
    workshop_layout::WorkshopLayoutMode,
    workshop_view::WorkshopViewAction,
};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PlatformTextRole {
    Control,
    Body,
    Metadata,
    SectionTitle,
    Status,
    Alert,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PlatformTextOverflow {
    SingleLineEllipsis,
    Wrap,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PlatformTextStyle {
    pub weight: FontWeight,
    pub font_size: f32,
    pub line_height: f32,
    pub role: PlatformTextRole,
    pub overflow: PlatformTextOverflow,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct PlatformVisibleNodeState {
    pub enabled: bool,
    pub selected: bool,
    pub focused: bool,
}

#[derive(Clone, Debug, PartialEq)]
pub struct PlatformVisibleNodeRecord {
    pub semantic_id: SemanticNodeId,
    pub action_id: Option<SemanticActionId>,
    pub display_text: String,
    pub semantic_name: String,
    pub semantic_description: String,
    pub semantic_value: Option<String>,
    pub role: PlatformTextRole,
    pub overflow: PlatformTextOverflow,
    pub bounds: PlatformRect,
    pub clip: Option<PlatformRect>,
    pub state: PlatformVisibleNodeState,
    pub icon: Option<UiIcon>,
    pub prewrapped_lines: Option<Vec<String>>,
}

impl PlatformVisibleNodeRecord {
    pub fn informative(&self) -> bool {
        !self.display_text.is_empty() || self.icon.is_some()
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct PlatformTextRun {
    pub semantic_id: SemanticNodeId,
    pub exact_text: String,
    pub bounds: PlatformRect,
    pub clip: PlatformRect,
    pub style: PlatformTextStyle,
    pub logical_advance: f32,
    pub emitted_glyph_range: [usize; 2],
}

#[derive(Clone, Debug, PartialEq)]
pub struct PlatformIconRun {
    pub semantic_id: SemanticNodeId,
    pub icon: UiIcon,
    pub bounds: PlatformRect,
    pub clip: PlatformRect,
    pub emitted_glyph_range: [usize; 2],
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PlatformPanelRole {
    FullSurface,
    PersistentChrome,
    ControlFill,
    InspectorRowFill,
    Drawer,
    Modal,
    Scrim,
    ContrastBorder,
}

#[derive(Clone, Debug, PartialEq)]
pub struct PlatformPanelWitness {
    pub role: PlatformPanelRole,
    pub bounds: PlatformRect,
    pub owning_node: Option<SemanticNodeId>,
    pub emitted_panel_range: [usize; 2],
}

pub struct PlatformSdfOutput {
    pub batch: UiBatch,
    pub text_runs: Vec<PlatformTextRun>,
    pub icon_runs: Vec<PlatformIconRun>,
    pub panel_witnesses: Vec<PlatformPanelWitness>,
}

pub fn build_platform_ui_batch(
    frame: &PlatformUiFrame,
    metrics: &AtlasMetrics,
) -> Result<PlatformSdfOutput, UiBatchError> {
    let viewport = PlatformRect::from_xywh(0.0, 0.0, frame.viewport.x, frame.viewport.y);
    let mut output = PlatformSdfOutput {
        batch: UiBatch::default(),
        text_runs: Vec::new(),
        icon_runs: Vec::new(),
        panel_witnesses: Vec::new(),
    };
    let chrome = if frame.high_contrast {
        [0.0, 0.0, 0.0, 0.98]
    } else {
        [0.035, 0.075, 0.12, 0.96]
    };
    let control_fill = if frame.high_contrast {
        [0.14, 0.14, 0.14, 1.0]
    } else {
        [0.06, 0.2, 0.32, 0.96]
    };
    let text_color = if frame.high_contrast {
        [1.0, 1.0, 1.0, 1.0]
    } else {
        [0.72, 0.9, 1.0, 1.0]
    };
    let modal_scope = frame.modal_semantic_ids();

    match frame.background {
        PlatformBackground::Full => {
            push_panel(
                &mut output,
                PlatformPanelRole::FullSurface,
                viewport,
                None,
                if frame.high_contrast {
                    [0.01, 0.01, 0.015, 1.0]
                } else {
                    [0.015, 0.035, 0.065, 0.96]
                },
                viewport,
            )?;
            push_panel(
                &mut output,
                PlatformPanelRole::PersistentChrome,
                frame.layout.top_bar,
                None,
                chrome,
                viewport,
            )?;
        }
        PlatformBackground::ChromeOnly => {
            for rect in &frame.layout.chrome {
                push_panel(
                    &mut output,
                    PlatformPanelRole::PersistentChrome,
                    *rect,
                    None,
                    chrome,
                    viewport,
                )?;
            }
        }
    }
    if let Some(drawer) = frame.drawer {
        push_panel(
            &mut output,
            PlatformPanelRole::Drawer,
            drawer,
            None,
            chrome,
            viewport,
        )?;
    }
    if let Some(modal) = frame.modal {
        push_panel(
            &mut output,
            PlatformPanelRole::Scrim,
            frame.layout.canvas,
            None,
            [0.0, 0.0, 0.0, 0.58],
            viewport,
        )?;
        push_panel(
            &mut output,
            PlatformPanelRole::Modal,
            modal,
            None,
            chrome,
            viewport,
        )?;
    }
    for control in frame.controls.iter().filter(|control| {
        modal_scope
            .as_ref()
            .is_none_or(|scope| scope.contains(&control.semantic_id))
    }) {
        let fill = if !control.enabled {
            [0.11, 0.13, 0.15, 0.82]
        } else if control.focused {
            [0.95, 0.58, 0.12, 1.0]
        } else if control.selected {
            [0.08, 0.48, 0.72, 1.0]
        } else {
            control_fill
        };
        push_panel(
            &mut output,
            PlatformPanelRole::ControlFill,
            control.bounds,
            Some(control.semantic_id.clone()),
            fill,
            viewport,
        )?;
        if frame.high_contrast && (control.focused || control.selected) {
            push_border(
                &mut output,
                control.bounds,
                control.semantic_id.clone(),
                [1.0, 1.0, 1.0, 1.0],
                viewport,
            )?;
        }
    }
    for record in frame.visible_nodes.iter().filter(|record| {
        record.action_id.is_none()
            && record.semantic_id.as_str().starts_with("inspector.")
            && modal_scope
                .as_ref()
                .is_none_or(|scope| scope.contains(&record.semantic_id))
    }) {
        push_panel(
            &mut output,
            PlatformPanelRole::InspectorRowFill,
            record.bounds,
            Some(record.semantic_id.clone()),
            if frame.high_contrast {
                [0.06, 0.06, 0.06, 1.0]
            } else {
                [0.025, 0.09, 0.14, 0.78]
            },
            viewport,
        )?;
    }

    for record in frame.visible_nodes.iter().filter(|record| {
        record.informative()
            && modal_scope
                .as_ref()
                .is_none_or(|scope| scope.contains(&record.semantic_id))
    }) {
        let fixed_label = record.role == PlatformTextRole::Control
            && record.overflow == PlatformTextOverflow::Wrap;
        let Some(mut bounds) = record.bounds.intersection(viewport) else {
            continue;
        };
        let accepted_clip = record.clip.unwrap_or(record.bounds);
        let Some(clip) = accepted_clip.intersection(viewport) else {
            continue;
        };
        if fixed_label {
            bounds = PlatformRect::from_xywh(
                bounds.min.x + 3.0,
                bounds.min.y + 3.0,
                (bounds.width() - 6.0).max(0.0),
                (bounds.height() - 6.0).max(0.0),
            );
        } else {
            let padding = text_ink_padding(frame.layout.ui_scale).min(bounds.width() * 0.25);
            bounds = PlatformRect::from_xywh(
                bounds.min.x + padding,
                bounds.min.y,
                (bounds.width() - 2.0 * padding).max(0.0),
                bounds.height(),
            );
        }
        let mut text_bounds = bounds;
        if let Some(icon) = record.icon {
            let size = bounds.height().min(20.0).min(bounds.width());
            let icon_bounds = PlatformRect::from_xywh(bounds.min.x, bounds.min.y, size, size);
            let glyph_start = output.batch.glyphs().len();
            output.batch.push_icon_clipped(
                metrics,
                icon,
                rect_array(icon_bounds),
                text_color,
                Some(rect_array(clip)),
            )?;
            output.icon_runs.push(PlatformIconRun {
                semantic_id: record.semantic_id.clone(),
                icon,
                bounds: icon_bounds,
                clip,
                emitted_glyph_range: [glyph_start, output.batch.glyphs().len()],
            });
            // Only the actual Medium Navigator rail uses its persistent top-bar
            // record for the label; do not paint a cropped duplicate in the rail.
            if frame.layout.mode == WorkshopLayoutMode::Medium
                && frame.controls.iter().any(|control| {
                    control.semantic_id == record.semantic_id
                        && matches!(
                            control.action,
                            PlatformUiAction::WorkshopView(WorkshopViewAction::OpenNavigator)
                        )
                        && frame
                            .layout
                            .left_panel
                            .is_some_and(|rail| rail.contains_rect(control.bounds))
                })
            {
                continue;
            }
            if !record.display_text.is_empty() && bounds.width() > size + 6.0 {
                text_bounds = PlatformRect::from_xywh(
                    bounds.min.x + size + 6.0,
                    bounds.min.y,
                    bounds.width() - size - 6.0,
                    bounds.height(),
                );
            }
        }
        if record.display_text.is_empty() || text_bounds.width() <= 0.0 {
            continue;
        }
        let mut style = style_for(record, frame.layout.ui_scale);
        let lines = match &record.prewrapped_lines {
            Some(lines)
                if lines.concat() == record.display_text
                    && lines.iter().all(|line| {
                        metrics
                            .measure_text(style.font_size, style.weight, line)
                            .is_ok_and(|measured| measured.advance <= text_bounds.width())
                    }) =>
            {
                lines.clone()
            }
            _ => layout_text(metrics, &record.display_text, text_bounds.width(), style)?,
        };
        if fixed_label {
            // Fixed controls keep the scaled 13px type. Fit the measured line
            // count before emitting anything, using compact leading only when
            // the ordinary 1.35em leading exceeds the accepted control box.
            style.line_height = style
                .line_height
                .min(text_bounds.height() / lines.len() as f32);
            if style.line_height < style.font_size {
                return Err(UiBatchError::NonFiniteGeometry);
            }
        }
        for (index, line) in lines.into_iter().enumerate() {
            let line_bounds = PlatformRect::from_xywh(
                text_bounds.min.x,
                text_bounds.min.y + index as f32 * style.line_height,
                text_bounds.width(),
                style.line_height,
            );
            if line_bounds.min.y >= text_bounds.max.y {
                break;
            }
            let Some(line_clip) = (if fixed_label {
                // Preserve glyph bearings and the SDF edge fringe in the
                // control's existing padding, rather than cutting at the pen
                // origin or each line's typographic box.
                Some(clip)
            } else {
                record.bounds.intersection(clip)
            }) else {
                continue;
            };
            if fixed_label && !line_clip.contains_rect(line_bounds) {
                return Err(UiBatchError::NonFiniteGeometry);
            }
            let glyph_start = output.batch.glyphs().len();
            // The bundled Inter control ink uses an 0.8em ascent. Using the
            // full em as the baseline offset spends the compact line's descent
            // space above the glyphs and clips its final line at larger scales.
            let logical_advance = output.batch.push_text_clipped(
                metrics,
                [
                    line_bounds.min.x,
                    line_bounds.min.y + style.font_size * if fixed_label { 0.8 } else { 1.0 },
                ],
                style.font_size,
                style.weight,
                text_color,
                &line,
                Some(rect_array(line_clip)),
            )?;
            if fixed_label && glyph_start == output.batch.glyphs().len() {
                return Err(UiBatchError::NonFiniteGeometry);
            }
            output.text_runs.push(PlatformTextRun {
                semantic_id: record.semantic_id.clone(),
                exact_text: line,
                bounds: line_bounds,
                clip: line_clip,
                style,
                logical_advance,
                emitted_glyph_range: [glyph_start, output.batch.glyphs().len()],
            });
        }
    }
    Ok(output)
}

fn style_for(record: &PlatformVisibleNodeRecord, scale: f32) -> PlatformTextStyle {
    let (font_size, weight) = match record.role {
        PlatformTextRole::SectionTitle => (18.0, FontWeight::SemiBold),
        PlatformTextRole::Control => (13.0, FontWeight::SemiBold),
        PlatformTextRole::Metadata => (13.0, FontWeight::Regular),
        PlatformTextRole::Body | PlatformTextRole::Status | PlatformTextRole::Alert => {
            (15.0, FontWeight::Regular)
        }
    };
    let font_size = font_size * scale;
    PlatformTextStyle {
        weight,
        font_size,
        line_height: font_size * 1.35,
        role: record.role,
        overflow: record.overflow,
    }
}

pub(crate) fn text_ink_padding(scale: f32) -> f32 {
    3.0 * scale
}

fn layout_text(
    metrics: &AtlasMetrics,
    value: &str,
    width: f32,
    style: PlatformTextStyle,
) -> Result<Vec<String>, UiBatchError> {
    match style.overflow {
        PlatformTextOverflow::SingleLineEllipsis => {
            Ok(vec![ellipsize(metrics, value, width, style)?])
        }
        PlatformTextOverflow::Wrap => wrap_measured(metrics, value, width, style),
    }
}

fn ellipsize(
    metrics: &AtlasMetrics,
    value: &str,
    width: f32,
    style: PlatformTextStyle,
) -> Result<String, UiBatchError> {
    if metrics
        .measure_text(style.font_size, style.weight, value)?
        .advance
        <= width
    {
        return Ok(value.to_owned());
    }
    let suffix = ["...", "..", ".", ""]
        .into_iter()
        .find(|suffix| {
            metrics
                .measure_text(style.font_size, style.weight, suffix)
                .is_ok_and(|measured| measured.advance <= width)
        })
        .unwrap_or("");
    let suffix_width = metrics
        .measure_text(style.font_size, style.weight, suffix)?
        .advance;
    let mut result = String::new();
    for character in value.chars() {
        let mut candidate = result.clone();
        candidate.push(character);
        if metrics
            .measure_text(style.font_size, style.weight, &candidate)?
            .advance
            + suffix_width
            > width
        {
            break;
        }
        result.push(character);
    }
    result.push_str(suffix);
    Ok(result)
}

fn wrap_measured(
    metrics: &AtlasMetrics,
    value: &str,
    width: f32,
    style: PlatformTextStyle,
) -> Result<Vec<String>, UiBatchError> {
    let mut lines = Vec::new();
    let mut line = String::new();
    for character in value.chars() {
        let mut candidate = line.clone();
        candidate.push(character);
        if !line.is_empty()
            && metrics
                .measure_text(style.font_size, style.weight, &candidate)?
                .advance
                > width
        {
            lines.push(std::mem::take(&mut line));
        }
        line.push(character);
    }
    if !line.is_empty() {
        lines.push(line);
    }
    if lines.is_empty() {
        lines.push(String::new());
    }
    Ok(lines)
}

fn push_panel(
    output: &mut PlatformSdfOutput,
    role: PlatformPanelRole,
    bounds: PlatformRect,
    owning_node: Option<SemanticNodeId>,
    color: [f32; 4],
    viewport: PlatformRect,
) -> Result<(), UiBatchError> {
    let Some(bounds) = bounds.intersection(viewport) else {
        return Ok(());
    };
    let start = output.batch.panels().len();
    output.batch.push_panel(rect_array(bounds), color)?;
    output.panel_witnesses.push(PlatformPanelWitness {
        role,
        bounds,
        owning_node,
        emitted_panel_range: [start, output.batch.panels().len()],
    });
    Ok(())
}

fn push_border(
    output: &mut PlatformSdfOutput,
    bounds: PlatformRect,
    owning_node: SemanticNodeId,
    color: [f32; 4],
    viewport: PlatformRect,
) -> Result<(), UiBatchError> {
    let Some(bounds) = bounds.intersection(viewport) else {
        return Ok(());
    };
    let thickness = 2.0_f32.min(bounds.width() * 0.5).min(bounds.height() * 0.5);
    let strips = [
        PlatformRect::from_xywh(bounds.min.x, bounds.min.y, bounds.width(), thickness),
        PlatformRect::from_xywh(
            bounds.min.x,
            bounds.max.y - thickness,
            bounds.width(),
            thickness,
        ),
        PlatformRect::from_xywh(
            bounds.min.x,
            bounds.min.y + thickness,
            thickness,
            (bounds.height() - thickness * 2.0).max(0.0),
        ),
        PlatformRect::from_xywh(
            bounds.max.x - thickness,
            bounds.min.y + thickness,
            thickness,
            (bounds.height() - thickness * 2.0).max(0.0),
        ),
    ];
    let start = output.batch.panels().len();
    for strip in strips {
        if strip.width() > 0.0 && strip.height() > 0.0 {
            output.batch.push_panel(rect_array(strip), color)?;
        }
    }
    output.panel_witnesses.push(PlatformPanelWitness {
        role: PlatformPanelRole::ContrastBorder,
        bounds,
        owning_node: Some(owning_node),
        emitted_panel_range: [start, output.batch.panels().len()],
    });
    Ok(())
}

fn rect_array(rect: PlatformRect) -> [f32; 4] {
    [rect.min.x, rect.min.y, rect.width(), rect.height()]
}
