use std::collections::HashSet;

use serde::Deserialize;

pub mod accessibility;
pub mod creator;
pub mod guide;
pub mod platform;
pub mod platform_inspector;
#[cfg(not(target_arch = "wasm32"))]
pub mod platform_native;
pub(crate) mod platform_projection;
pub mod platform_sdf;
#[cfg(target_arch = "wasm32")]
pub mod platform_web;
pub mod start_marker;
pub mod virtual_list;
pub mod workshop;
mod workshop_inspector;
pub mod workshop_layout;
pub mod workshop_view;

pub const UI_ATLAS_WIDTH: u32 = 1024;
pub const UI_ATLAS_HEIGHT: u32 = 1024;
pub const UI_ATLAS_BYTES: &[u8] = include_bytes!("../assets/ui/atlas.r8");
pub const UI_ATLAS_METRICS_JSON: &str = include_str!("../assets/ui/atlas-metrics.json");
pub const UI_SDF_WGSL: &str = include_str!("../assets/shaders/ui_sdf.wgsl");
pub const MAX_UI_GLYPHS: usize = 8_192;
pub const MAX_UI_PANELS: usize = 512;
pub const UI_ATLAS_ENTRY_COUNT: usize = 95 * 2 + UiIcon::ALL.len();

const UI_ATLAS_ORDERING: &str =
    "regular ASCII U+0020..U+007E; semibold ASCII U+0020..U+007E; frozen icons in source order";

/// Static text owned by the SDF command interface. Presentation builders use
/// these values directly; tests validate this same production surface instead
/// of maintaining a parallel literal list.
pub mod text {
    pub const TITLE: &str = "NYON";
    pub const NEURAL_ADVISORY: &str = "NEURAL ADVISORY";
    pub const ADVISORY_ONLY: &str = "ADVISORY ONLY";
    pub const CPU_FALLBACK: &str = "CPU FALLBACK";
    pub const APPLY_AND_RESTART: &str = "APPLY AND RESTART";
    pub const FACTORY_DEFAULTS: &str = "FACTORY DEFAULTS";
    pub const REGENERATE_FROM_SEED: &str = "REGENERATE FROM SEED";
    pub const COMMAND_READY: &str = "COMMAND READY";
    pub const SPEED_DOWN: &str = "SPEED -";
    pub const PLAY: &str = "PLAY";
    pub const PAUSE: &str = "PAUSE";
    pub const SPEED_UP: &str = "SPEED +";
    pub const LAUNCH: &str = "LAUNCH";
    pub const RESET: &str = "RESET";
    pub const PAUSED: &str = "PAUSED";
    pub const SETTINGS: &str = "SETTINGS";
    pub const HELP: &str = "HELP";
    pub const SCENARIO: &str = "SCENARIO";
    pub const SKIP: &str = "SKIP";
    pub const ADVISORY: &str = "ADVISORY";
    pub const RESTART_GUIDANCE: &str = "RESTART GUIDANCE";
    pub const SCALE_85: &str = "85%";
    pub const SCALE_100: &str = "100%";
    pub const SCALE_115: &str = "115%";
    pub const SCALE_130: &str = "130%";
    pub const REDUCED_MOTION: &str = "REDUCED MOTION";
    pub const HIGH_CONTRAST: &str = "HIGH CONTRAST";
    pub const AUTO: &str = "AUTO";
    pub const LOW: &str = "LOW";
    pub const HIGH: &str = "HIGH";
    pub const RESET_GUIDANCE: &str = "RESET GUIDANCE";
    pub const CLOSE: &str = "CLOSE";
    pub const SELECT_UNION_WORLD: &str = "SELECT A UNION WORLD";
    pub const PREVIEW_DESTINATION: &str = "PREVIEW A DESTINATION";
    pub const LAUNCH_FLEET: &str = "LAUNCH THE FLEET";
    pub const TUNE_FIELD: &str = "TUNE A FIELD";
    pub const READ_ADVISORY: &str = "READ THE ADVISORY";
    pub const OPEN_SCENARIO_EDITOR: &str = "OPEN THE SCENARIO EDITOR";
    pub const SAVE: &str = "SAVE";
    pub const LOAD: &str = "LOAD";
    pub const CANCEL: &str = "CANCEL";

    pub const ALL: &[&str] = &[
        TITLE,
        NEURAL_ADVISORY,
        ADVISORY_ONLY,
        CPU_FALLBACK,
        APPLY_AND_RESTART,
        FACTORY_DEFAULTS,
        REGENERATE_FROM_SEED,
        COMMAND_READY,
        SPEED_DOWN,
        PLAY,
        PAUSE,
        SPEED_UP,
        LAUNCH,
        RESET,
        PAUSED,
        SETTINGS,
        HELP,
        SCENARIO,
        SKIP,
        ADVISORY,
        RESTART_GUIDANCE,
        SCALE_85,
        SCALE_100,
        SCALE_115,
        SCALE_130,
        REDUCED_MOTION,
        HIGH_CONTRAST,
        AUTO,
        LOW,
        HIGH,
        RESET_GUIDANCE,
        CLOSE,
        SELECT_UNION_WORLD,
        PREVIEW_DESTINATION,
        LAUNCH_FLEET,
        TUNE_FIELD,
        READ_ADVISORY,
        OPEN_SCENARIO_EDITOR,
        SAVE,
        LOAD,
        CANCEL,
    ];
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq)]
pub enum AtlasFormat {
    #[serde(rename = "r8_unorm")]
    R8Unorm,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum EntryKind {
    Glyph,
    Icon,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, Hash, PartialEq)]
pub struct AtlasBounds {
    pub x: u32,
    pub y: u32,
    pub width: u32,
    pub height: u32,
}

#[derive(Clone, Debug, Deserialize, PartialEq)]
pub struct AtlasEntry {
    pub key: String,
    pub kind: EntryKind,
    pub codepoint: Option<u32>,
    pub font_weight: Option<String>,
    pub icon: Option<String>,
    pub atlas_bounds: AtlasBounds,
    pub advance: f32,
}

#[derive(Clone, Debug, Deserialize, PartialEq)]
pub struct AtlasMetrics {
    pub format_version: u32,
    pub atlas: AtlasDescriptor,
    pub generator: GeneratorDescriptor,
    pub entries: Vec<AtlasEntry>,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq)]
pub struct AtlasDescriptor {
    pub width: u32,
    pub height: u32,
    pub format: AtlasFormat,
}

#[derive(Clone, Debug, Deserialize, PartialEq)]
pub struct GeneratorDescriptor {
    pub cell_size: u32,
    pub font_px: f32,
    pub font_origin_x: i32,
    pub font_baseline_y: i32,
    pub icon_size: u32,
    pub edge_threshold: u8,
    pub sdf_spread: u32,
    pub ordering: String,
}

impl AtlasMetrics {
    pub fn embedded() -> Result<Self, serde_json::Error> {
        serde_json::from_str(UI_ATLAS_METRICS_JSON)
    }

    pub fn entry(&self, key: &str) -> Option<&AtlasEntry> {
        self.entries.iter().find(|entry| entry.key == key)
    }

    pub fn measure_text(
        &self,
        font_size: f32,
        weight: FontWeight,
        value: &str,
    ) -> Result<TextMetrics, UiBatchError> {
        glyph_walk(self, font_size, weight, value, |_, _, _| Ok(()))
    }

    pub fn validate(&self) -> Result<(), AtlasValidationError> {
        if self.format_version != 1 {
            return Err(AtlasValidationError::FormatVersion(self.format_version));
        }
        if self.atlas.width != UI_ATLAS_WIDTH
            || self.atlas.height != UI_ATLAS_HEIGHT
            || self.atlas.format != AtlasFormat::R8Unorm
        {
            return Err(AtlasValidationError::AtlasDescriptor);
        }
        if UI_ATLAS_BYTES.len() != (self.atlas.width * self.atlas.height) as usize {
            return Err(AtlasValidationError::PayloadLength(UI_ATLAS_BYTES.len()));
        }
        if self.generator.cell_size != 64
            || self.generator.font_px != 40.0
            || self.generator.font_origin_x != 8
            || self.generator.font_baseline_y != 48
            || self.generator.icon_size != 40
            || self.generator.edge_threshold != 96
            || self.generator.sdf_spread != 8
            || self.generator.ordering != UI_ATLAS_ORDERING
        {
            return Err(AtlasValidationError::GeneratorDescriptor);
        }
        if self.entries.len() != UI_ATLAS_ENTRY_COUNT {
            return Err(AtlasValidationError::EntryCount {
                actual: self.entries.len(),
                expected: UI_ATLAS_ENTRY_COUNT,
            });
        }

        let mut keys = HashSet::with_capacity(self.entries.len());
        let mut bounds_seen = Vec::with_capacity(self.entries.len());
        for (index, entry) in self.entries.iter().enumerate() {
            if !keys.insert(entry.key.as_str()) {
                return Err(AtlasValidationError::DuplicateKey(entry.key.clone()));
            }
            let bounds = entry.atlas_bounds;
            if bounds.width == 0
                || bounds.height == 0
                || bounds.x.checked_add(bounds.width).is_none()
                || bounds.y.checked_add(bounds.height).is_none()
                || bounds.x + bounds.width > self.atlas.width
                || bounds.y + bounds.height > self.atlas.height
            {
                return Err(AtlasValidationError::OutOfBounds(entry.key.clone()));
            }
            if bounds_seen
                .iter()
                .any(|previous| bounds_overlap(*previous, bounds))
            {
                return Err(AtlasValidationError::OverlappingBounds(entry.key.clone()));
            }
            if bounds.width != self.generator.cell_size
                || bounds.height != self.generator.cell_size
                || bounds.x % self.generator.cell_size != 0
                || bounds.y % self.generator.cell_size != 0
            {
                return Err(AtlasValidationError::CellGeometry(entry.key.clone()));
            }
            bounds_seen.push(bounds);
            if !entry.advance.is_finite() || entry.advance < 0.0 {
                return Err(AtlasValidationError::InvalidAdvance(entry.key.clone()));
            }
            validate_entry_mapping(index, entry)?;
        }
        Ok(())
    }
}

#[derive(Debug, thiserror::Error, PartialEq)]
pub enum AtlasValidationError {
    #[error("unsupported UI atlas metrics version {0}")]
    FormatVersion(u32),
    #[error("UI atlas must be a 1024x1024 R8 payload")]
    AtlasDescriptor,
    #[error("UI atlas payload has {0} bytes")]
    PayloadLength(usize),
    #[error("UI atlas generator descriptor does not match the frozen generator")]
    GeneratorDescriptor,
    #[error("UI atlas has {actual} entries; expected exactly {expected}")]
    EntryCount { actual: usize, expected: usize },
    #[error("duplicate UI atlas key {0}")]
    DuplicateKey(String),
    #[error("UI atlas entry {0} is outside the atlas")]
    OutOfBounds(String),
    #[error("UI atlas entry {0} overlaps another entry")]
    OverlappingBounds(String),
    #[error("UI atlas entry {0} does not occupy one aligned generator cell")]
    CellGeometry(String),
    #[error("UI atlas entry {0} has an invalid advance")]
    InvalidAdvance(String),
    #[error("UI atlas entry {0} has inconsistent mapping metadata")]
    EntryMapping(String),
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum FontWeight {
    Regular,
    SemiBold,
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct TextMetrics {
    pub advance: f32,
}

impl FontWeight {
    const fn name(self) -> &'static str {
        match self {
            Self::Regular => "regular",
            Self::SemiBold => "semibold",
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum UiIcon {
    Play,
    Pause,
    Speed,
    Reset,
    Settings,
    Help,
    Crosshair,
    Save,
    Load,
    Check,
    Close,
    Zoom,
    FieldAtmosphere,
    FieldHydrosphere,
    FieldTopology,
}

impl UiIcon {
    pub const ALL: [Self; 15] = [
        Self::Play,
        Self::Pause,
        Self::Speed,
        Self::Reset,
        Self::Settings,
        Self::Help,
        Self::Crosshair,
        Self::Save,
        Self::Load,
        Self::Check,
        Self::Close,
        Self::Zoom,
        Self::FieldAtmosphere,
        Self::FieldHydrosphere,
        Self::FieldTopology,
    ];

    pub const fn key(self) -> &'static str {
        match self {
            Self::Play => "icon:play",
            Self::Pause => "icon:pause",
            Self::Speed => "icon:speed",
            Self::Reset => "icon:reset",
            Self::Settings => "icon:settings",
            Self::Help => "icon:help",
            Self::Crosshair => "icon:crosshair",
            Self::Save => "icon:save",
            Self::Load => "icon:load",
            Self::Check => "icon:check",
            Self::Close => "icon:close",
            Self::Zoom => "icon:zoom",
            Self::FieldAtmosphere => "icon:field-atmosphere",
            Self::FieldHydrosphere => "icon:field-hydrosphere",
            Self::FieldTopology => "icon:field-topology",
        }
    }

    const fn name(self) -> &'static str {
        match self {
            Self::Play => "play",
            Self::Pause => "pause",
            Self::Speed => "speed",
            Self::Reset => "reset",
            Self::Settings => "settings",
            Self::Help => "help",
            Self::Crosshair => "crosshair",
            Self::Save => "save",
            Self::Load => "load",
            Self::Check => "check",
            Self::Close => "close",
            Self::Zoom => "zoom",
            Self::FieldAtmosphere => "field-atmosphere",
            Self::FieldHydrosphere => "field-hydrosphere",
            Self::FieldTopology => "field-topology",
        }
    }
}

#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, bytemuck::Pod, bytemuck::Zeroable)]
pub struct UiGlyphInstance {
    pub rect: [f32; 4],
    pub uv_rect: [f32; 4],
    pub color: [f32; 4],
}

#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, bytemuck::Pod, bytemuck::Zeroable)]
pub struct UiPanelInstance {
    pub rect: [f32; 4],
    pub color: [f32; 4],
}

impl UiPanelInstance {
    pub const ATTRIBUTES: [wgpu::VertexAttribute; 2] = wgpu::vertex_attr_array![
        0 => Float32x4,
        1 => Float32x4,
    ];

    pub const LAYOUT: wgpu::VertexBufferLayout<'static> = wgpu::VertexBufferLayout {
        array_stride: std::mem::size_of::<Self>() as wgpu::BufferAddress,
        step_mode: wgpu::VertexStepMode::Instance,
        attributes: &Self::ATTRIBUTES,
    };
}

impl UiGlyphInstance {
    pub const ATTRIBUTES: [wgpu::VertexAttribute; 3] = wgpu::vertex_attr_array![
        0 => Float32x4,
        1 => Float32x4,
        2 => Float32x4,
    ];

    pub const LAYOUT: wgpu::VertexBufferLayout<'static> = wgpu::VertexBufferLayout {
        array_stride: std::mem::size_of::<Self>() as wgpu::BufferAddress,
        step_mode: wgpu::VertexStepMode::Instance,
        attributes: &Self::ATTRIBUTES,
    };
}

#[derive(Default)]
pub struct UiBatch {
    panels: Vec<UiPanelInstance>,
    glyphs: Vec<UiGlyphInstance>,
}

impl UiBatch {
    pub fn clear(&mut self) {
        self.panels.clear();
        self.glyphs.clear();
    }

    pub fn panels(&self) -> &[UiPanelInstance] {
        &self.panels
    }

    pub fn glyphs(&self) -> &[UiGlyphInstance] {
        &self.glyphs
    }

    pub fn push_panel(&mut self, rect: [f32; 4], color: [f32; 4]) -> Result<(), UiBatchError> {
        validate_rect_and_color(rect, color)?;
        let requested = self.panels.len().saturating_add(1);
        if requested > MAX_UI_PANELS {
            return Err(UiBatchError::PanelCapacity { requested });
        }
        self.panels.push(UiPanelInstance { rect, color });
        Ok(())
    }

    pub fn push_text(
        &mut self,
        metrics: &AtlasMetrics,
        baseline: [f32; 2],
        font_size: f32,
        weight: FontWeight,
        color: [f32; 4],
        value: &str,
    ) -> Result<f32, UiBatchError> {
        self.push_text_clipped(metrics, baseline, font_size, weight, color, value, None)
    }

    #[allow(clippy::too_many_arguments)]
    pub fn push_text_clipped(
        &mut self,
        metrics: &AtlasMetrics,
        baseline: [f32; 2],
        font_size: f32,
        weight: FontWeight,
        color: [f32; 4],
        value: &str,
        clip: Option<[f32; 4]>,
    ) -> Result<f32, UiBatchError> {
        validate_draw_values(baseline, font_size, color)?;
        validate_clip(clip)?;
        let scale = text_scale(metrics, font_size)?;
        let existing_glyphs = self.glyphs.len();
        let remaining_capacity = MAX_UI_GLYPHS.saturating_sub(existing_glyphs);
        let mut glyphs = Vec::with_capacity(value.len().min(MAX_UI_GLYPHS));
        let measured = glyph_walk(
            metrics,
            font_size,
            weight,
            value,
            |character, entry, offset| {
                if character == ' ' {
                    return Ok(());
                }
                let size = metrics.generator.cell_size as f32 * scale;
                let rect = [
                    baseline[0] + offset - metrics.generator.font_origin_x as f32 * scale,
                    baseline[1] - metrics.generator.font_baseline_y as f32 * scale,
                    size,
                    size,
                ];
                if rect.iter().any(|value| !value.is_finite()) {
                    return Err(UiBatchError::NonFiniteGeometry);
                }
                if let Some(glyph) = clipped_instance(entry, rect, color, clip) {
                    if glyphs.len() >= remaining_capacity {
                        return Err(UiBatchError::Capacity {
                            requested: existing_glyphs
                                .saturating_add(glyphs.len())
                                .saturating_add(1),
                        });
                    }
                    glyphs.push(glyph);
                }
                Ok(())
            },
        )?;
        self.reserve(glyphs.len())?;
        self.glyphs.extend(glyphs);
        Ok(measured.advance)
    }

    pub fn push_icon(
        &mut self,
        metrics: &AtlasMetrics,
        icon: UiIcon,
        rect: [f32; 4],
        color: [f32; 4],
    ) -> Result<(), UiBatchError> {
        self.push_icon_clipped(metrics, icon, rect, color, None)
    }

    pub fn push_icon_clipped(
        &mut self,
        metrics: &AtlasMetrics,
        icon: UiIcon,
        rect: [f32; 4],
        color: [f32; 4],
        clip: Option<[f32; 4]>,
    ) -> Result<(), UiBatchError> {
        validate_rect_and_color(rect, color)?;
        validate_clip(clip)?;
        let entry = validated_icon_entry(metrics, icon)?;
        let glyph = clipped_instance(entry, rect, color, clip);
        self.reserve(usize::from(glyph.is_some()))?;
        if let Some(glyph) = glyph {
            self.glyphs.push(glyph);
        }
        Ok(())
    }

    fn reserve(&self, additional: usize) -> Result<(), UiBatchError> {
        let requested = self.glyphs.len().saturating_add(additional);
        if requested > MAX_UI_GLYPHS {
            Err(UiBatchError::Capacity { requested })
        } else {
            Ok(())
        }
    }
}

#[derive(Debug, thiserror::Error, PartialEq)]
pub enum UiBatchError {
    #[error("UI text contains unsupported character {0:?}")]
    UnsupportedCharacter(char),
    #[error("UI atlas is missing {0}")]
    MissingEntry(String),
    #[error("UI geometry or color is invalid")]
    NonFiniteGeometry,
    #[error("UI batch capacity exceeded by request for {requested} glyphs")]
    Capacity { requested: usize },
    #[error("UI batch capacity exceeded by request for {requested} panels")]
    PanelCapacity { requested: usize },
}

fn validate_rect_and_color(rect: [f32; 4], color: [f32; 4]) -> Result<(), UiBatchError> {
    if rect.iter().any(|value| !value.is_finite())
        || rect[2] < 0.0
        || rect[3] < 0.0
        || color.iter().any(|value| !value.is_finite())
    {
        Err(UiBatchError::NonFiniteGeometry)
    } else {
        Ok(())
    }
}

fn validate_draw_values(
    baseline: [f32; 2],
    font_size: f32,
    color: [f32; 4],
) -> Result<(), UiBatchError> {
    if baseline.iter().any(|value| !value.is_finite())
        || !font_size.is_finite()
        || font_size < 0.0
        || color.iter().any(|value| !value.is_finite())
    {
        Err(UiBatchError::NonFiniteGeometry)
    } else {
        Ok(())
    }
}

fn validate_clip(clip: Option<[f32; 4]>) -> Result<(), UiBatchError> {
    if let Some(rect) = clip {
        validate_rect_and_color(rect, [0.0; 4])?;
        let right = rect[0] + rect[2];
        let bottom = rect[1] + rect[3];
        if !right.is_finite() || !bottom.is_finite() {
            return Err(UiBatchError::NonFiniteGeometry);
        }
    }
    Ok(())
}

fn text_scale(metrics: &AtlasMetrics, font_size: f32) -> Result<f32, UiBatchError> {
    if !font_size.is_finite()
        || font_size < 0.0
        || !metrics.generator.font_px.is_finite()
        || metrics.generator.font_px <= 0.0
    {
        return Err(UiBatchError::NonFiniteGeometry);
    }
    let scale = font_size / metrics.generator.font_px;
    if scale.is_finite() {
        Ok(scale)
    } else {
        Err(UiBatchError::NonFiniteGeometry)
    }
}

fn glyph_walk(
    metrics: &AtlasMetrics,
    font_size: f32,
    weight: FontWeight,
    value: &str,
    mut visit: impl FnMut(char, &AtlasEntry, f32) -> Result<(), UiBatchError>,
) -> Result<TextMetrics, UiBatchError> {
    let scale = text_scale(metrics, font_size)?;
    let mut advance = 0.0_f32;
    for character in value.chars() {
        let codepoint = character as u32;
        if !(0x20..=0x7e).contains(&codepoint) {
            return Err(UiBatchError::UnsupportedCharacter(character));
        }
        let entry = validated_glyph_entry(metrics, weight, codepoint)?;
        visit(character, entry, advance)?;
        let next = advance + entry.advance * scale;
        if !next.is_finite() {
            return Err(UiBatchError::NonFiniteGeometry);
        }
        advance = next;
    }
    Ok(TextMetrics { advance })
}

fn validated_glyph_entry(
    metrics: &AtlasMetrics,
    weight: FontWeight,
    codepoint: u32,
) -> Result<&AtlasEntry, UiBatchError> {
    let weight_offset = match weight {
        FontWeight::Regular => 0,
        FontWeight::SemiBold => 95,
    };
    let index = weight_offset + (codepoint - 0x20) as usize;
    let entry = metrics
        .entries
        .get(index)
        .ok_or_else(|| UiBatchError::MissingEntry(glyph_key(weight, codepoint)))?;
    if entry.kind != EntryKind::Glyph
        || entry.codepoint != Some(codepoint)
        || entry.font_weight.as_deref() != Some(weight.name())
        || !glyph_key_matches(&entry.key, weight, codepoint)
    {
        return Err(UiBatchError::MissingEntry(glyph_key(weight, codepoint)));
    }
    Ok(entry)
}

fn validated_icon_entry(metrics: &AtlasMetrics, icon: UiIcon) -> Result<&AtlasEntry, UiBatchError> {
    let index = 95 * 2 + icon as usize;
    let entry = metrics
        .entries
        .get(index)
        .ok_or_else(|| UiBatchError::MissingEntry(icon.key().to_owned()))?;
    if entry.kind != EntryKind::Icon
        || entry.icon.as_deref() != Some(icon.name())
        || entry.key != icon.key()
    {
        return Err(UiBatchError::MissingEntry(icon.key().to_owned()));
    }
    Ok(entry)
}

fn glyph_key_matches(key: &str, weight: FontWeight, codepoint: u32) -> bool {
    let prefix = match weight {
        FontWeight::Regular => "regular:U+",
        FontWeight::SemiBold => "semibold:U+",
    };
    let Some(hex) = key.strip_prefix(prefix) else {
        return false;
    };
    hex.len() == 4 && u32::from_str_radix(hex, 16) == Ok(codepoint)
}

fn glyph_key(weight: FontWeight, codepoint: u32) -> String {
    format!("{}:U+{codepoint:04X}", weight.name())
}

fn validate_entry_mapping(index: usize, entry: &AtlasEntry) -> Result<(), AtlasValidationError> {
    let glyph_count = 95;
    let expected = if index < glyph_count * 2 {
        let weight = if index < glyph_count {
            FontWeight::Regular
        } else {
            FontWeight::SemiBold
        };
        let codepoint = 0x20 + (index % glyph_count) as u32;
        (
            glyph_key(weight, codepoint),
            EntryKind::Glyph,
            Some(codepoint),
            Some(weight.name()),
            None,
        )
    } else {
        let icon = UiIcon::ALL[index - glyph_count * 2];
        (
            icon.key().to_owned(),
            EntryKind::Icon,
            None,
            None,
            Some(icon.name()),
        )
    };
    if entry.key != expected.0
        || entry.kind != expected.1
        || entry.codepoint != expected.2
        || entry.font_weight.as_deref() != expected.3
        || entry.icon.as_deref() != expected.4
    {
        Err(AtlasValidationError::EntryMapping(entry.key.clone()))
    } else {
        Ok(())
    }
}

fn bounds_overlap(left: AtlasBounds, right: AtlasBounds) -> bool {
    left.x < right.x + right.width
        && right.x < left.x + left.width
        && left.y < right.y + right.height
        && right.y < left.y + left.height
}

fn instance(entry: &AtlasEntry, rect: [f32; 4], color: [f32; 4]) -> UiGlyphInstance {
    let bounds = entry.atlas_bounds;
    UiGlyphInstance {
        rect,
        uv_rect: [
            bounds.x as f32 / UI_ATLAS_WIDTH as f32,
            bounds.y as f32 / UI_ATLAS_HEIGHT as f32,
            bounds.width as f32 / UI_ATLAS_WIDTH as f32,
            bounds.height as f32 / UI_ATLAS_HEIGHT as f32,
        ],
        color,
    }
}

fn clipped_instance(
    entry: &AtlasEntry,
    rect: [f32; 4],
    color: [f32; 4],
    clip: Option<[f32; 4]>,
) -> Option<UiGlyphInstance> {
    let mut glyph = instance(entry, rect, color);
    let Some(clip) = clip else { return Some(glyph) };
    if rect[2] == 0.0 || rect[3] == 0.0 || clip[2] == 0.0 || clip[3] == 0.0 {
        return None;
    }
    let left = rect[0].max(clip[0]);
    let top = rect[1].max(clip[1]);
    let right = (rect[0] + rect[2]).min(clip[0] + clip[2]);
    let bottom = (rect[1] + rect[3]).min(clip[1] + clip[3]);
    if right <= left || bottom <= top {
        return None;
    }
    let x0 = (left - rect[0]) / rect[2];
    let y0 = (top - rect[1]) / rect[3];
    let x1 = (right - rect[0]) / rect[2];
    let y1 = (bottom - rect[1]) / rect[3];
    glyph.rect = [left, top, right - left, bottom - top];
    glyph.uv_rect = [
        glyph.uv_rect[0] + glyph.uv_rect[2] * x0,
        glyph.uv_rect[1] + glyph.uv_rect[3] * y0,
        glyph.uv_rect[2] * (x1 - x0),
        glyph.uv_rect[3] * (y1 - y0),
    ];
    Some(glyph)
}
