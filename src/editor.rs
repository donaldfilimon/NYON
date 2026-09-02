use std::collections::{BTreeMap, BTreeSet};

use glam::Vec2;

use crate::engine::input::NavigationAction;
use crate::scenario::codec;
use crate::scenario::store::{ScenarioStore, StoreError};
use crate::scenario::{ScenarioDraft, ScenarioV1, ValidationReport};

mod layout;
mod render;

pub use render::build_frame;

const HEADER_HEIGHT: f32 = 56.0;
const ROW_HEIGHT: f32 = 44.0;
const ROW_GAP: f32 = 8.0;
const MIN_TARGET: f32 = 44.0;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum EditorLayoutKind {
    Wide,
    Compact,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum EditorSection {
    Worlds,
    Rules,
    Validation,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum EditorAction {
    SelectSection(EditorSection),
    SelectWorld(usize),
    Revert,
    FactoryDefaults,
    RegenerateFromSeed,
    Load,
    Save,
    Cancel,
    ApplyAndRestart,
    Confirm,
    DismissConfirmation,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ConfirmationAction {
    Revert,
    FactoryDefaults,
    RegenerateFromSeed,
    LoadDirty,
    OverwriteSave,
    CancelDirty,
    ApplyAndRestart,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ConfirmedEditorAction {
    Cancel,
    ApplyAndRestart(Box<ScenarioV1>),
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum WidgetKind {
    Section,
    Preview,
    Field,
    Status,
    Confirmation,
    Action(EditorAction),
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct EditorRect {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
}

impl EditorRect {
    pub fn new(x: f32, y: f32, width: f32, height: f32) -> Self {
        Self {
            x: if x.is_finite() { x } else { 0.0 },
            y: if y.is_finite() { y } else { 0.0 },
            width: width.max(0.0),
            height: height.max(0.0),
        }
    }

    pub fn center(self) -> Vec2 {
        Vec2::new(self.x + self.width * 0.5, self.y + self.height * 0.5)
    }

    pub fn contains(self, point: Vec2) -> bool {
        point.x >= self.x
            && point.y >= self.y
            && point.x <= self.x + self.width
            && point.y <= self.y + self.height
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct EditorWidget {
    pub id: String,
    pub kind: WidgetKind,
    pub rect: EditorRect,
    pub label: String,
    pub value: String,
    pub enabled: bool,
    pub error: Option<String>,
    pub focusable: bool,
    pub visible: bool,
}

#[derive(Debug, thiserror::Error)]
pub enum EditorError {
    #[error(transparent)]
    Store(#[from] StoreError),
    #[error(transparent)]
    Codec(#[from] codec::CodecError),
    #[error(transparent)]
    Validation(#[from] ValidationReport),
    #[error("no matching editor widget")]
    UnknownWidget,
    #[error("an editor field contains uncommitted invalid text")]
    InvalidText,
    #[error("no confirmation is active")]
    NoConfirmation,
    #[error("the editor confirmation became stale")]
    StaleConfirmation,
    #[error("another editor confirmation is already active")]
    ConfirmationActive,
}

pub struct EditorState {
    original: ScenarioDraft,
    draft: ScenarioDraft,
    widgets: Vec<EditorWidget>,
    text_buffers: BTreeMap<String, String>,
    parse_errors: BTreeSet<String>,
    focused: Option<String>,
    scroll: f32,
    maximum_scroll: f32,
    viewport: Vec2,
    layout_kind: EditorLayoutKind,
    section: EditorSection,
    selected_world: usize,
    content_rect: EditorRect,
    confirmation: Option<ConfirmationAction>,
    pending_save: Option<String>,
    pending_apply: Option<ScenarioV1>,
    ime_preedit: String,
    store_message: Option<String>,
}

impl EditorState {
    pub fn new(active: ScenarioDraft) -> Self {
        let mut state = Self {
            original: active.clone(),
            draft: active,
            widgets: Vec::new(),
            text_buffers: BTreeMap::new(),
            parse_errors: BTreeSet::new(),
            focused: None,
            scroll: 0.0,
            maximum_scroll: 0.0,
            viewport: Vec2::new(1440.0, 900.0),
            layout_kind: EditorLayoutKind::Wide,
            section: EditorSection::Worlds,
            selected_world: 0,
            content_rect: EditorRect::default(),
            confirmation: None,
            pending_save: None,
            pending_apply: None,
            ime_preedit: String::new(),
            store_message: None,
        };
        state.reset_buffers();
        state.rebuild_widgets(state.viewport);
        state
    }

    pub fn original(&self) -> &ScenarioDraft {
        &self.original
    }

    pub fn draft(&self) -> &ScenarioDraft {
        &self.draft
    }

    pub fn widgets(&self) -> &[EditorWidget] {
        &self.widgets
    }

    pub fn layout_kind(&self) -> EditorLayoutKind {
        self.layout_kind
    }

    pub fn section(&self) -> EditorSection {
        self.section
    }

    pub fn selected_world(&self) -> usize {
        self.selected_world
    }

    pub fn content_rect(&self) -> EditorRect {
        self.content_rect
    }

    pub fn confirmation(&self) -> Option<ConfirmationAction> {
        self.confirmation
    }

    pub fn store_message(&self) -> Option<&str> {
        self.store_message.as_deref()
    }

    pub fn is_dirty(&self) -> bool {
        self.draft != self.original || !self.parse_errors.is_empty()
    }

    pub fn validation_report(&self) -> ValidationReport {
        self.draft.validation_report()
    }

    pub fn can_apply(&self) -> bool {
        self.parse_errors.is_empty() && self.draft.validation_report().is_valid()
    }

    fn validate_candidate(&self) -> Result<ScenarioV1, EditorError> {
        if !self.parse_errors.is_empty() {
            return Err(EditorError::InvalidText);
        }
        Ok(self.draft.validated()?)
    }

    pub fn ime_preedit(&self) -> &str {
        &self.ime_preedit
    }

    pub fn set_ime_preedit(&mut self, value: impl Into<String>) {
        if self.confirmation.is_some() {
            return;
        }
        self.ime_preedit = value.into();
        self.rebuild_widgets(self.viewport);
    }

    pub fn commit_ime(&mut self, value: &str) {
        if self.confirmation.is_some() {
            return;
        }
        self.insert_focused_text(value);
        self.ime_preedit.clear();
        self.rebuild_widgets(self.viewport);
    }

    pub fn select_section(&mut self, section: EditorSection) {
        if self.confirmation.is_some() || self.section == section {
            return;
        }
        self.section = section;
        self.focused = None;
        self.scroll = 0.0;
        self.rebuild_widgets(self.viewport);
    }

    pub fn select_world(&mut self, index: usize) {
        if self.confirmation.is_some() || index >= self.draft.worlds.len() {
            return;
        }
        self.section = EditorSection::Worlds;
        self.selected_world = index;
        self.focused = None;
        self.scroll = 0.0;
        self.rebuild_widgets(self.viewport);
    }

    pub fn focused_widget_id(&self) -> Option<&str> {
        self.focused.as_deref()
    }

    /// Reports whether printable keyboard or IME text belongs to the focused
    /// widget. The platform layer uses this before interpreting Space as a
    /// control activation key.
    pub fn focused_accepts_text(&self) -> bool {
        self.focused
            .as_ref()
            .is_some_and(|id| self.text_buffers.contains_key(id))
    }

    pub fn focused_text(&self) -> Option<&str> {
        self.focused
            .as_ref()
            .and_then(|id| self.text_buffers.get(id))
            .map(String::as_str)
    }

    pub fn focus_widget(&mut self, id: &str) -> Result<(), EditorError> {
        if !self
            .widgets
            .iter()
            .any(|widget| widget.id == id && widget.focusable && widget.enabled)
        {
            return Err(EditorError::UnknownWidget);
        }
        self.focused = Some(id.into());
        self.reveal_focus();
        Ok(())
    }

    pub fn focus_first(&mut self) {
        self.focused = self
            .widgets
            .iter()
            .find(|widget| widget.focusable && widget.enabled)
            .map(|widget| widget.id.clone());
        self.reveal_focus();
    }

    pub fn focus_next(&mut self, reverse: bool) {
        let focusable: Vec<String> = self
            .widgets
            .iter()
            .filter(|widget| widget.focusable && widget.enabled)
            .map(|widget| widget.id.clone())
            .collect();
        if focusable.is_empty() {
            self.focused = None;
            return;
        }
        let current = self
            .focused
            .as_ref()
            .and_then(|id| focusable.iter().position(|candidate| candidate == id));
        let index = match (current, reverse) {
            (Some(0), true) | (None, true) => focusable.len() - 1,
            (Some(index), true) => index - 1,
            (Some(index), false) => (index + 1) % focusable.len(),
            (None, false) => 0,
        };
        self.focused = Some(focusable[index].clone());
        self.reveal_focus();
    }

    pub fn hit_test(&self, point: Vec2) -> Option<String> {
        self.widgets
            .iter()
            .rev()
            .find(|widget| {
                widget.visible && widget.enabled && widget.focusable && widget.rect.contains(point)
            })
            .map(|widget| widget.id.clone())
    }

    pub fn activate_focused(&self) -> Option<EditorAction> {
        let focused = self.focused.as_ref()?;
        let widget = self
            .widgets
            .iter()
            .find(|widget| widget.id == *focused && widget.enabled)?;
        match widget.kind {
            WidgetKind::Action(action) => Some(action),
            _ => None,
        }
    }

    pub fn activate_at(&mut self, point: Vec2) -> Option<EditorAction> {
        let id = self.hit_test(point)?;
        self.focused = Some(id);
        self.activate_focused()
    }

    pub fn navigate(&mut self, action: NavigationAction) -> Option<EditorAction> {
        match action {
            NavigationAction::TabForward | NavigationAction::Down | NavigationAction::Right => {
                self.focus_next(false)
            }
            NavigationAction::TabBackward | NavigationAction::Up | NavigationAction::Left => {
                self.focus_next(true)
            }
            NavigationAction::PageUp => self.scroll_by(-self.viewport.y.max(MIN_TARGET)),
            NavigationAction::PageDown => self.scroll_by(self.viewport.y.max(MIN_TARGET)),
            NavigationAction::Activate => return self.activate_focused(),
            NavigationAction::Escape => {
                return Some(if self.confirmation.is_some() {
                    EditorAction::DismissConfirmation
                } else {
                    EditorAction::Cancel
                });
            }
            NavigationAction::Backspace => self.backspace_focused(),
            NavigationAction::Delete => self.replace_focused_text(""),
            NavigationAction::Home => {
                self.scroll = 0.0;
                self.rebuild_widgets(self.viewport);
            }
            NavigationAction::End => {
                self.scroll = self.maximum_scroll;
                self.rebuild_widgets(self.viewport);
            }
        }
        None
    }

    pub fn replace_focused_text(&mut self, value: &str) {
        if self.confirmation.is_some() {
            self.confirmation = None;
            self.pending_apply = None;
            self.pending_save = None;
            self.rebuild_widgets(self.viewport);
            return;
        }
        let Some(id) = self.focused.clone() else {
            return;
        };
        if !self.text_buffers.contains_key(&id) {
            return;
        }
        self.text_buffers.insert(id.clone(), value.into());
        self.apply_buffer(&id);
        self.rebuild_widgets(self.viewport);
    }

    pub fn insert_focused_text(&mut self, value: &str) {
        if self.confirmation.is_some() {
            return;
        }
        let Some(id) = self.focused.clone() else {
            return;
        };
        let Some(buffer) = self.text_buffers.get_mut(&id) else {
            return;
        };
        buffer.push_str(value);
        self.apply_buffer(&id);
        self.rebuild_widgets(self.viewport);
    }

    pub fn backspace_focused(&mut self) {
        if self.confirmation.is_some() {
            return;
        }
        let Some(id) = self.focused.clone() else {
            return;
        };
        if let Some(buffer) = self.text_buffers.get_mut(&id) {
            buffer.pop();
            self.apply_buffer(&id);
            self.rebuild_widgets(self.viewport);
        }
    }

    pub fn scroll_by(&mut self, delta: f32) {
        if delta.is_finite() {
            self.scroll = (self.scroll + delta).clamp(0.0, self.maximum_scroll);
            self.rebuild_widgets(self.viewport);
        }
    }

    pub fn scroll(&self) -> f32 {
        self.scroll
    }

    pub fn maximum_scroll(&self) -> f32 {
        self.maximum_scroll
    }

    fn perform_revert(&mut self) {
        self.draft = self.original.clone();
        self.confirmation = None;
        self.pending_apply = None;
        self.pending_save = None;
        self.reset_buffers();
        self.rebuild_widgets(self.viewport);
    }

    pub fn request_revert(&mut self) {
        if self.confirmation.is_some() {
            return;
        }
        self.confirmation = Some(ConfirmationAction::Revert);
        self.rebuild_widgets(self.viewport);
    }

    fn perform_factory_defaults(&mut self) {
        self.draft = ScenarioDraft::factory_default();
        self.confirmation = None;
        self.pending_apply = None;
        self.pending_save = None;
        self.reset_buffers();
        self.rebuild_widgets(self.viewport);
    }

    pub fn request_factory_defaults(&mut self) {
        if self.confirmation.is_some() {
            return;
        }
        self.confirmation = Some(ConfirmationAction::FactoryDefaults);
        self.rebuild_widgets(self.viewport);
    }

    fn perform_regenerate_from_seed(&mut self) {
        self.draft.regenerate_worlds_from_seed();
        self.confirmation = None;
        self.pending_apply = None;
        self.pending_save = None;
        self.reset_buffers();
        self.rebuild_widgets(self.viewport);
    }

    pub fn request_regenerate_from_seed(&mut self) {
        if self.confirmation.is_some() {
            return;
        }
        self.confirmation = Some(ConfirmationAction::RegenerateFromSeed);
        self.rebuild_widgets(self.viewport);
    }

    fn load_slot<S: ScenarioStore>(&mut self, store: &S) -> Result<bool, EditorError> {
        let loaded = match store.load() {
            Ok(loaded) => loaded,
            Err(error) => {
                self.store_message = Some(error.to_string());
                self.rebuild_widgets(self.viewport);
                return Err(EditorError::Store(error));
            }
        };
        let Some(payload) = loaded else {
            self.store_message = Some("NO SAVED SCENARIO".into());
            self.rebuild_widgets(self.viewport);
            return Ok(false);
        };
        let temporary = match codec::decode_draft(&payload).and_then(|draft| {
            draft.validated()?;
            Ok(draft)
        }) {
            Ok(draft) => draft,
            Err(error) => {
                self.store_message = Some(error.to_string());
                self.rebuild_widgets(self.viewport);
                return Err(EditorError::Codec(error));
            }
        };
        self.draft = temporary;
        self.store_message = Some("SCENARIO LOADED INTO DRAFT".into());
        self.reset_buffers();
        self.rebuild_widgets(self.viewport);
        Ok(true)
    }

    pub fn request_load<S: ScenarioStore>(&mut self, store: &S) -> Result<bool, EditorError> {
        if self.confirmation.is_some() {
            return Err(EditorError::ConfirmationActive);
        }
        if self.is_dirty() {
            self.confirmation = Some(ConfirmationAction::LoadDirty);
            self.rebuild_widgets(self.viewport);
            return Ok(false);
        }
        self.load_slot(store)
    }

    pub fn request_cancel(&mut self) -> bool {
        if self.is_dirty() {
            self.confirmation = Some(ConfirmationAction::CancelDirty);
            self.rebuild_widgets(self.viewport);
            false
        } else {
            true
        }
    }

    pub fn request_apply_and_restart(&mut self) -> Result<(), EditorError> {
        if self.confirmation.is_some() {
            return Err(EditorError::ConfirmationActive);
        }
        let scenario = self.validate_candidate()?;
        self.pending_apply = Some(scenario);
        self.confirmation = Some(ConfirmationAction::ApplyAndRestart);
        self.rebuild_widgets(self.viewport);
        Ok(())
    }

    pub fn request_save<S: ScenarioStore>(&mut self, store: &mut S) -> Result<bool, EditorError> {
        if self.confirmation.is_some() {
            return Err(EditorError::ConfirmationActive);
        }
        if !self.parse_errors.is_empty() {
            return Err(EditorError::InvalidText);
        }
        let payload = codec::encode_draft(&self.draft)?;
        let occupied = match store.load() {
            Ok(occupied) => occupied,
            Err(error) => {
                self.store_message = Some(error.to_string());
                self.rebuild_widgets(self.viewport);
                return Err(EditorError::Store(error));
            }
        };
        if occupied.is_some() {
            self.pending_save = Some(payload);
            self.confirmation = Some(ConfirmationAction::OverwriteSave);
            self.rebuild_widgets(self.viewport);
            return Ok(false);
        }
        if let Err(error) = store.save(&payload) {
            self.store_message = Some(error.to_string());
            self.rebuild_widgets(self.viewport);
            return Err(EditorError::Store(error));
        }
        self.store_message = Some("SCENARIO SAVED".into());
        self.rebuild_widgets(self.viewport);
        Ok(true)
    }

    pub fn cancel_confirmation(&mut self) {
        self.confirmation = None;
        self.pending_save = None;
        self.pending_apply = None;
        self.rebuild_widgets(self.viewport);
    }

    pub fn confirm<S: ScenarioStore>(
        &mut self,
        store: &mut S,
    ) -> Result<Option<ConfirmedEditorAction>, EditorError> {
        let action = self
            .confirmation
            .take()
            .ok_or(EditorError::NoConfirmation)?;
        match action {
            ConfirmationAction::OverwriteSave => {
                let payload = self
                    .pending_save
                    .take()
                    .ok_or(EditorError::NoConfirmation)?;
                if let Err(error) = store.save(&payload) {
                    self.store_message = Some(error.to_string());
                    self.rebuild_widgets(self.viewport);
                    return Err(EditorError::Store(error));
                }
                self.store_message = Some("SCENARIO SAVED".into());
            }
            ConfirmationAction::Revert => self.perform_revert(),
            ConfirmationAction::FactoryDefaults => self.perform_factory_defaults(),
            ConfirmationAction::RegenerateFromSeed => self.perform_regenerate_from_seed(),
            ConfirmationAction::LoadDirty => {
                self.load_slot(store)?;
            }
            ConfirmationAction::CancelDirty => {
                self.pending_apply = None;
                self.pending_save = None;
                self.rebuild_widgets(self.viewport);
                return Ok(Some(ConfirmedEditorAction::Cancel));
            }
            ConfirmationAction::ApplyAndRestart => {
                let pending = self
                    .pending_apply
                    .take()
                    .ok_or(EditorError::StaleConfirmation)?;
                let current = self.validate_candidate()?;
                if current != pending {
                    self.pending_save = None;
                    self.rebuild_widgets(self.viewport);
                    return Err(EditorError::StaleConfirmation);
                }
                self.pending_save = None;
                self.rebuild_widgets(self.viewport);
                return Ok(Some(ConfirmedEditorAction::ApplyAndRestart(Box::new(
                    pending,
                ))));
            }
        }
        self.pending_apply = None;
        self.pending_save = None;
        self.rebuild_widgets(self.viewport);
        Ok(None)
    }

    fn reveal_focus(&mut self) {
        let Some(id) = self.focused.clone() else {
            return;
        };
        let Some(widget) = self.widgets.iter().find(|widget| widget.id == id) else {
            return;
        };
        if matches!(
            widget.kind,
            WidgetKind::Action(_) | WidgetKind::Status | WidgetKind::Preview
        ) {
            return;
        }
        if widget.rect.y < self.content_rect.y {
            self.scroll = (self.scroll - (self.content_rect.y - widget.rect.y)).max(0.0);
            self.rebuild_widgets(self.viewport);
        } else if widget.rect.y + widget.rect.height
            > self.content_rect.y + self.content_rect.height
        {
            self.scroll = (self.scroll + widget.rect.y + widget.rect.height
                - (self.content_rect.y + self.content_rect.height))
                .min(self.maximum_scroll);
            self.rebuild_widgets(self.viewport);
        }
    }

    fn reset_buffers(&mut self) {
        self.text_buffers.clear();
        self.parse_errors.clear();
        for (id, value) in current_field_values(&self.draft) {
            self.text_buffers.insert(id, value);
        }
    }

    fn synchronize_typed_buffers(&mut self) {
        for (id, value) in current_field_values(&self.draft) {
            if !self.parse_errors.contains(&id) {
                self.text_buffers.insert(id, value);
            }
        }
    }

    fn apply_buffer(&mut self, id: &str) {
        let Some(value) = self.text_buffers.get(id).cloned() else {
            return;
        };
        let parsed = apply_text_value(&mut self.draft, id, &value);
        if parsed {
            self.parse_errors.remove(id);
        } else {
            self.parse_errors.insert(id.into());
        }
    }
}

fn current_field_values(draft: &ScenarioDraft) -> Vec<(String, String)> {
    let rules = draft.rules;
    let mut values = vec![
        ("scenario.seed".into(), format!("{:016X}", draft.seed)),
        ("rule.tick_hz".into(), rules.tick_hz.to_string()),
        (
            "rule.win_world_count".into(),
            rules.win_world_count.to_string(),
        ),
        (
            "rule.ai_period_ticks".into(),
            rules.ai_period_ticks.to_string(),
        ),
        (
            "rule.hazard_first_tick".into(),
            rules.hazard_first_tick.to_string(),
        ),
        (
            "rule.hazard_period_ticks".into(),
            rules.hazard_period_ticks.to_string(),
        ),
        (
            "rule.hazard_duration_ticks".into(),
            rules.hazard_duration_ticks.to_string(),
        ),
        (
            "rule.maximum_energy".into(),
            rules.maximum_energy.0.to_string(),
        ),
        (
            "rule.maximum_defense".into(),
            rules.maximum_defense.0.to_string(),
        ),
        (
            "rule.minimum_launch".into(),
            rules.minimum_launch.0.to_string(),
        ),
        (
            "rule.field_raise_cost".into(),
            rules.field_raise_cost.0.to_string(),
        ),
        (
            "rule.field_lower_refund".into(),
            rules.field_lower_refund.0.to_string(),
        ),
        (
            "rule.base_fleet_speed".into(),
            rules.base_fleet_speed.to_string(),
        ),
    ];
    for (index, world) in draft.worlds.iter().enumerate() {
        let owner = match world.owner {
            None => "NEUTRAL",
            Some(crate::game::model::Faction::Union) => "UNION",
            Some(crate::game::model::Faction::Helix) => "HELIX",
            Some(crate::game::model::Faction::Choir) => "CHOIR",
        };
        for (suffix, value) in [
            ("name", world.name.clone()),
            ("x", world.x.to_string()),
            ("y", world.y.to_string()),
            ("owner", owner.into()),
            ("energy", world.energy.to_string()),
            ("defense", world.defense.to_string()),
            ("base_output", world.base_output.to_string()),
            ("base_regeneration", world.base_regeneration.to_string()),
            ("atmosphere", world.atmosphere.to_string()),
            ("hydrosphere", world.hydrosphere.to_string()),
            ("topology", world.topology.to_string()),
        ] {
            values.push((format!("world.{index}.{suffix}"), value));
        }
    }
    values
}

fn apply_text_value(draft: &mut ScenarioDraft, id: &str, value: &str) -> bool {
    match id {
        "scenario.seed" => {
            if value.len() != 16 {
                return false;
            }
            u64::from_str_radix(value, 16)
                .map(|parsed| draft.seed = parsed)
                .is_ok()
        }
        "rule.win_world_count" => parse_assign(value, &mut draft.rules.win_world_count),
        "rule.ai_period_ticks" => parse_assign(value, &mut draft.rules.ai_period_ticks),
        "rule.hazard_first_tick" => parse_assign(value, &mut draft.rules.hazard_first_tick),
        "rule.hazard_period_ticks" => parse_assign(value, &mut draft.rules.hazard_period_ticks),
        "rule.hazard_duration_ticks" => parse_assign(value, &mut draft.rules.hazard_duration_ticks),
        "rule.maximum_energy" => parse_assign(value, &mut draft.rules.maximum_energy.0),
        "rule.maximum_defense" => parse_assign(value, &mut draft.rules.maximum_defense.0),
        "rule.minimum_launch" => parse_assign(value, &mut draft.rules.minimum_launch.0),
        "rule.field_raise_cost" => parse_assign(value, &mut draft.rules.field_raise_cost.0),
        "rule.field_lower_refund" => parse_assign(value, &mut draft.rules.field_lower_refund.0),
        "rule.base_fleet_speed" => parse_assign(value, &mut draft.rules.base_fleet_speed),
        _ => apply_world_text_value(draft, id, value),
    }
}

fn apply_world_text_value(draft: &mut ScenarioDraft, id: &str, value: &str) -> bool {
    let mut parts = id.split('.');
    if parts.next() != Some("world") {
        return false;
    }
    let Some(index) = parts.next().and_then(|part| part.parse::<usize>().ok()) else {
        return false;
    };
    let Some(field) = parts.next() else {
        return false;
    };
    let Some(world) = draft.worlds.get_mut(index) else {
        return false;
    };
    match field {
        "name" => {
            world.name = value.into();
            true
        }
        "x" => parse_assign(value, &mut world.x),
        "y" => parse_assign(value, &mut world.y),
        "owner" => match value {
            "NEUTRAL" => {
                world.owner = None;
                true
            }
            "UNION" => {
                world.owner = Some(crate::game::model::Faction::Union);
                true
            }
            "HELIX" => {
                world.owner = Some(crate::game::model::Faction::Helix);
                true
            }
            "CHOIR" => {
                world.owner = Some(crate::game::model::Faction::Choir);
                true
            }
            _ => false,
        },
        "energy" => parse_assign(value, &mut world.energy),
        "defense" => parse_assign(value, &mut world.defense),
        "base_output" => parse_assign(value, &mut world.base_output),
        "base_regeneration" => parse_assign(value, &mut world.base_regeneration),
        "atmosphere" => parse_assign(value, &mut world.atmosphere),
        "hydrosphere" => parse_assign(value, &mut world.hydrosphere),
        "topology" => parse_assign(value, &mut world.topology),
        _ => false,
    }
}

fn parse_assign<T: std::str::FromStr>(value: &str, destination: &mut T) -> bool {
    match value.parse() {
        Ok(parsed) => {
            *destination = parsed;
            true
        }
        Err(_) => false,
    }
}

fn widget_id_for_issue(field: &str) -> String {
    if let Some(rest) = field.strip_prefix("rules.") {
        return format!("rule.{rest}");
    }
    if let Some(rest) = field.strip_prefix("worlds[")
        && let Some((index, suffix)) = rest.split_once(']')
    {
        let suffix = suffix.trim_start_matches('.');
        let suffix = match suffix {
            "position" => "x",
            "fields" => "atmosphere",
            other => other,
        };
        return format!("world.{index}.{suffix}");
    }
    field.into()
}

const fn action_id(action: EditorAction) -> &'static str {
    match action {
        EditorAction::SelectSection(_) => "action.select_section",
        EditorAction::SelectWorld(_) => "action.select_world",
        EditorAction::Revert => "action.revert",
        EditorAction::FactoryDefaults => "action.factory_defaults",
        EditorAction::RegenerateFromSeed => "action.regenerate",
        EditorAction::Load => "action.load",
        EditorAction::Save => "action.save",
        EditorAction::Cancel => "action.cancel",
        EditorAction::ApplyAndRestart => "action.apply",
        EditorAction::Confirm => "action.confirm",
        EditorAction::DismissConfirmation => "action.dismiss_confirmation",
    }
}
