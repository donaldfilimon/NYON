//! Immutable, render-platform-neutral UI model for Galaxy Workshop.
//!
//! Pointer input, keyboard input, DOM accessibility, and native accessibility
//! all address the same semantic action identifiers. Activating a control
//! returns a typed intent; it never mutates authoritative state directly.

use std::collections::BTreeMap;

use nyon_workshop_core::{
    BranchId, CatalogHash, EntityId, RevisionId, ValidatedCatalogPackV1, WorkshopStateV1,
    model::EntityKind,
};

use crate::{
    engine::backend::BackendKind,
    workshop::session::{WorkshopAction, WorkshopSessionSnapshot, WorkshopSpeed},
};

use super::accessibility::{InputModality, SemanticActionId, SemanticNodeId, SemanticTree};
pub use super::creator::CreatorTool;
use super::creator::{CreatorDraft, CreatorFieldKind};
use super::workshop_inspector::build_inspector;

mod creator_defaults;
mod history;
mod outliner;
mod removal;
mod semantics;

use creator_defaults::build_creator_form;
pub use creator_defaults::default_creator_batch;
pub(in crate::ui) use creator_defaults::is_ownable;
use history::{build_branches, build_save_status, build_timeline};
use outliner::build_outliner;
use removal::build_removal_confirmation;
pub use removal::removal_blockers;
use semantics::{all_controls, build_semantic_tree};

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum WorkshopUiIntent {
    OpenCreatorForm {
        tool: CreatorTool,
        subject: Option<EntityId>,
    },
    SelectEntity(EntityId),
    Dispatch(WorkshopAction),
    OpenRemovalConfirmation(EntityId),
    CloseRemovalConfirmation,
    SetReducedMotion(bool),
    SetHighContrast(bool),
    EditCreatorField {
        field_id: String,
        cycle: bool,
    },
    CloseCreatorForm,
    ReturnToMainMenu,
}

#[derive(Clone, Debug, Eq, PartialEq, thiserror::Error)]
pub enum CreatorFormError {
    #[error("{0}")]
    MissingPrerequisite(&'static str),
    #[error("the deterministic creator defaults could not be represented")]
    InvalidDefault,
    #[error("{0}")]
    Draft(String),
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct WorkshopControl {
    pub action_id: SemanticActionId,
    pub label: String,
    pub description: String,
    pub enabled: bool,
    pub selected: bool,
    pub intent: WorkshopUiIntent,
}

impl WorkshopControl {
    pub(super) fn new(
        action_id: impl Into<String>,
        label: impl Into<String>,
        description: impl Into<String>,
        enabled: bool,
        selected: bool,
        intent: WorkshopUiIntent,
    ) -> Self {
        Self {
            action_id: SemanticActionId::new(action_id),
            label: label.into(),
            description: description.into(),
            enabled,
            selected,
            intent,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CreatorToolControl {
    pub tool: CreatorTool,
    pub control: WorkshopControl,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum OutlinerKind {
    Faction,
    System,
    Star,
    World,
    Deposit,
    Industry,
    Lane,
    Route,
    Shipment,
    Hazard,
}

impl OutlinerKind {
    const fn label(self) -> &'static str {
        match self {
            Self::Faction => "Faction",
            Self::System => "System",
            Self::Star => "Star",
            Self::World => "World",
            Self::Deposit => "Deposit",
            Self::Industry => "Industry",
            Self::Lane => "Lane",
            Self::Route => "Route",
            Self::Shipment => "Shipment",
            Self::Hazard => "Hazard",
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct OutlinerEntry {
    pub entity: EntityId,
    pub parent: Option<EntityId>,
    pub depth: u32,
    pub kind: OutlinerKind,
    pub name: String,
    pub description: String,
    pub control: WorkshopControl,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct InspectorRow {
    pub semantic_id: SemanticNodeId,
    pub label: String,
    pub value: String,
    pub(super) stable_key: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct InspectorSection {
    pub semantic_id: SemanticNodeId,
    pub heading: &'static str,
    pub rows: Vec<InspectorRow>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum InspectorTextKind {
    Title,
    SectionHeading,
    Fact,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct InspectorTextRecord {
    pub semantic_id: SemanticNodeId,
    pub kind: InspectorTextKind,
    pub label: String,
    pub value: Option<String>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct InspectorModel {
    pub selected: Option<EntityId>,
    pub title: String,
    pub sections: Vec<InspectorSection>,
    pub remove_control: Option<WorkshopControl>,
}

impl InspectorModel {
    pub fn title_record(&self) -> InspectorTextRecord {
        InspectorTextRecord {
            semantic_id: SemanticNodeId::new("inspector.title"),
            kind: InspectorTextKind::Title,
            label: format!("Inspector: {}", self.title),
            value: None,
        }
    }

    pub fn text_records(&self) -> Vec<InspectorTextRecord> {
        let mut records = Vec::with_capacity(self.text_record_count());
        for section in &self.sections {
            records.push(InspectorTextRecord {
                semantic_id: section.semantic_id.clone(),
                kind: InspectorTextKind::SectionHeading,
                label: section.heading.to_owned(),
                value: None,
            });
            records.extend(section.rows.iter().map(|row| InspectorTextRecord {
                semantic_id: row.semantic_id.clone(),
                kind: InspectorTextKind::Fact,
                label: row.label.clone(),
                value: Some(row.value.clone()),
            }));
        }
        records
    }

    pub fn text_record_count(&self) -> usize {
        self.sections.len()
            + self
                .sections
                .iter()
                .map(|section| section.rows.len())
                .sum::<usize>()
    }

    pub fn rows(&self) -> impl Iterator<Item = &InspectorRow> {
        self.sections.iter().flat_map(|section| section.rows.iter())
    }

    pub fn text_record_index(&self, semantic_id: &SemanticNodeId) -> Option<usize> {
        self.text_records()
            .iter()
            .position(|record| &record.semantic_id == semantic_id)
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TimelineModel {
    pub tick: u64,
    pub speed: WorkshopSpeed,
    pub controls: Vec<WorkshopControl>,
    pub redo_children: Vec<RevisionId>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BranchChoice {
    pub branch: BranchId,
    pub name: String,
    pub head: Option<RevisionId>,
    pub selected_cursor: Option<RevisionId>,
    pub last_tick: u64,
    pub selected: bool,
    pub browsing_history: bool,
    pub control: WorkshopControl,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SaveStatusModel {
    pub slot: Option<u64>,
    pub generation: Option<u64>,
    pub dirty: bool,
    pub commit_pending: bool,
    pub load_pending: bool,
    pub status: String,
    pub save_control: WorkshopControl,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct WorkshopDiagnosticsModel {
    pub backend: String,
    pub catalog_hash: String,
    pub state_digest: String,
    pub tick: u64,
    pub revision_count: usize,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RemovalBlocker {
    pub entity: EntityId,
    pub label: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RemovalConfirmation {
    pub target: EntityId,
    pub target_label: String,
    pub blockers: Vec<RemovalBlocker>,
    pub cancel_control: WorkshopControl,
    pub confirm_control: WorkshopControl,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct WorkshopPresentationPreferences {
    pub reduced_motion: bool,
    pub high_contrast: bool,
    pub reduced_motion_control: WorkshopControl,
    pub high_contrast_control: WorkshopControl,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CreatorFormModel {
    pub tool: CreatorTool,
    pub title: String,
    pub fields: Vec<CreatorFormFieldModel>,
    pub preview: Vec<String>,
    pub validation_message: Option<String>,
    pub cancel_control: WorkshopControl,
    pub submit_control: WorkshopControl,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CreatorFormFieldModel {
    pub id: String,
    pub label: String,
    pub value: String,
    pub kind: CreatorFieldKind,
    pub control: WorkshopControl,
}

#[derive(Clone, Debug, Default)]
pub struct WorkshopUiContext<'a> {
    pub backend: Option<BackendKind>,
    pub catalog: Option<&'a ValidatedCatalogPackV1>,
    pub catalog_hash: Option<CatalogHash>,
    pub selected_entity: Option<EntityId>,
    pub redo_children: &'a [RevisionId],
    pub pending_removal: Option<EntityId>,
    pub reduced_motion: bool,
    pub high_contrast: bool,
    pub creator_form: Option<CreatorTool>,
    pub creator_draft: Option<&'a CreatorDraft>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct WorkshopUiModel {
    pub menu_control: WorkshopControl,
    pub tool_palette: Vec<CreatorToolControl>,
    pub outliner: Vec<OutlinerEntry>,
    pub inspector: InspectorModel,
    pub timeline: TimelineModel,
    pub branches: Vec<BranchChoice>,
    pub save: SaveStatusModel,
    pub diagnostics: WorkshopDiagnosticsModel,
    pub removal_confirmation: Option<RemovalConfirmation>,
    pub preferences: WorkshopPresentationPreferences,
    pub creator_form: Option<CreatorFormModel>,
    pub semantics: SemanticTree,
    controls: BTreeMap<SemanticActionId, WorkshopControl>,
    focus_order: Vec<SemanticActionId>,
}

impl WorkshopUiModel {
    pub fn build(snapshot: &WorkshopSessionSnapshot, context: WorkshopUiContext<'_>) -> Self {
        let selected = context
            .selected_entity
            .filter(|entity| snapshot.state.contains_entity(*entity));
        let replacement_active = snapshot.store.commit_pending || snapshot.store.load_pending;
        let paused = snapshot.speed == WorkshopSpeed::Paused;

        let tool_palette: Vec<CreatorToolControl> = CreatorTool::ALL
            .into_iter()
            .map(|tool| CreatorToolControl {
                tool,
                control: WorkshopControl::new(
                    format!("tool.{}", tool.slug()),
                    tool.label(),
                    tool.description(),
                    !snapshot.store.load_pending,
                    false,
                    WorkshopUiIntent::OpenCreatorForm {
                        tool,
                        subject: selected,
                    },
                ),
            })
            .collect();
        let menu_control = WorkshopControl::new(
            "workshop.main-menu",
            "Main menu",
            "Pause, request a durable save, and return to the main menu.",
            !snapshot.store.load_pending,
            false,
            WorkshopUiIntent::ReturnToMainMenu,
        );
        let outliner = build_outliner(&snapshot.state, selected);
        let inspector = build_inspector(&snapshot.state, selected, context.catalog);
        let timeline = build_timeline(snapshot, context.redo_children, paused, replacement_active);
        let branches = build_branches(snapshot, paused, replacement_active);
        let save = build_save_status(snapshot);
        let diagnostics = WorkshopDiagnosticsModel {
            backend: context.backend.map_or_else(
                || "NOT INITIALIZED".to_owned(),
                |backend| backend.label().to_owned(),
            ),
            catalog_hash: context
                .catalog
                .map(ValidatedCatalogPackV1::catalog_hash)
                .or(context.catalog_hash)
                .map_or_else(|| "UNAVAILABLE".to_owned(), |hash| fixed_hex(&hash.0)),
            state_digest: fixed_hex(&snapshot.state_digest.0),
            tick: snapshot.state.tick.0,
            revision_count: snapshot.revision_count,
        };
        let removal_confirmation = context
            .pending_removal
            .filter(|entity| is_removable(snapshot.state.entity_kind(*entity)))
            .map(|target| build_removal_confirmation(snapshot, target));
        let preferences = WorkshopPresentationPreferences {
            reduced_motion: context.reduced_motion,
            high_contrast: context.high_contrast,
            reduced_motion_control: WorkshopControl::new(
                "preferences.reduced-motion",
                "Reduced motion",
                "Reduce nonessential Workshop presentation motion.",
                true,
                context.reduced_motion,
                WorkshopUiIntent::SetReducedMotion(!context.reduced_motion),
            ),
            high_contrast_control: WorkshopControl::new(
                "preferences.high-contrast",
                "High contrast",
                "Use the high-contrast Workshop presentation palette.",
                true,
                context.high_contrast,
                WorkshopUiIntent::SetHighContrast(!context.high_contrast),
            ),
        };
        let creator_form = context
            .creator_form
            .map(|tool| build_creator_form(snapshot, selected, tool, context.creator_draft));

        let mut controls = BTreeMap::new();
        let mut focus_order = Vec::new();
        for control in all_controls(
            &menu_control,
            &tool_palette,
            &outliner,
            &inspector,
            &timeline,
            &branches,
            &save,
            removal_confirmation.as_ref(),
            &preferences,
            creator_form.as_ref(),
        ) {
            if controls
                .insert(control.action_id.clone(), control.clone())
                .is_none()
            {
                focus_order.push(control.action_id.clone());
            }
        }

        let semantics = build_semantic_tree(
            snapshot,
            &menu_control,
            &tool_palette,
            &outliner,
            &inspector,
            &timeline,
            &branches,
            &save,
            &diagnostics,
            removal_confirmation.as_ref(),
            &preferences,
            creator_form.as_ref(),
        );

        Self {
            menu_control,
            tool_palette,
            outliner,
            inspector,
            timeline,
            branches,
            save,
            diagnostics,
            removal_confirmation,
            preferences,
            creator_form,
            semantics,
            controls,
            focus_order,
        }
    }

    pub fn controls(&self) -> impl ExactSizeIterator<Item = &WorkshopControl> {
        self.controls.values()
    }

    pub fn focus_order(&self) -> &[SemanticActionId] {
        &self.focus_order
    }

    pub fn activate(
        &self,
        action: &SemanticActionId,
        _modality: InputModality,
    ) -> Option<WorkshopUiIntent> {
        self.controls
            .get(action)
            .filter(|control| control.enabled)
            .map(|control| control.intent.clone())
    }
}

pub fn build_workshop_ui(
    snapshot: &WorkshopSessionSnapshot,
    backend: Option<BackendKind>,
) -> WorkshopUiModel {
    WorkshopUiModel::build(
        snapshot,
        WorkshopUiContext {
            backend,
            ..WorkshopUiContext::default()
        },
    )
}

fn safe_store_status(status: &str) -> &'static str {
    match status {
        "Not saved" => "Not saved",
        "Recovered previous valid generation" => "Recovered previous valid generation",
        "Loaded" => "Loaded",
        "Loading" => "Loading",
        "Export ready" => "Export ready",
        "Imported; not saved" => "Imported; not saved",
        "Saved" => "Saved",
        "Saved and selected for Continue" => "Saved and selected for Continue",
        "Saving" => "Saving",
        "Save failed; retry delayed" => "Save failed; retry delayed",
        "Saved; Continue selection failed" => "Saved; Continue selection failed",
        _ => "Workshop storage status changed",
    }
}

pub(super) fn is_removable(kind: Option<EntityKind>) -> bool {
    matches!(
        kind,
        Some(
            EntityKind::Faction
                | EntityKind::System
                | EntityKind::Star
                | EntityKind::World
                | EntityKind::Lane
                | EntityKind::Deposit
                | EntityKind::Industry
                | EntityKind::Route
                | EntityKind::Hazard
        )
    )
}

pub(super) fn entity_display_name(state: &WorkshopStateV1, entity: EntityId) -> String {
    if let Some(value) = state.factions.get(&entity) {
        value.name.to_string()
    } else if let Some(value) = state.systems.get(&entity) {
        value.name.to_string()
    } else if let Some(value) = state.stars.get(&entity) {
        value.name.to_string()
    } else if let Some(value) = state.worlds.get(&entity) {
        value.name.to_string()
    } else if let Some(value) = state.deposits.get(&entity) {
        format!("{} deposit {}", value.resource_id, short_hex(&entity.0))
    } else if let Some(value) = state.industries.get(&entity) {
        format!("{} {}", value.definition_id, short_hex(&entity.0))
    } else if state.lanes.contains_key(&entity) {
        format!("Lane {}", short_hex(&entity.0))
    } else if let Some(value) = state.routes.get(&entity) {
        format!("{} route {}", value.resource_id, short_hex(&entity.0))
    } else if let Some(value) = state.shipments.get(&entity) {
        format!("{} shipment {}", value.resource_id, short_hex(&entity.0))
    } else if let Some(value) = state.hazards.get(&entity) {
        format!("{} hazard {}", value.hazard_id, short_hex(&entity.0))
    } else {
        format!("Unknown object {}", short_hex(&entity.0))
    }
}

pub(super) fn hazard_status(hazard: &nyon_workshop_core::model::HazardV1, tick: u64) -> String {
    let end = hazard.start_tick.0.saturating_add(hazard.duration_ticks);
    if hazard.cancelled {
        "Cancelled".to_owned()
    } else if hazard.start_tick.0 <= tick && tick < end {
        format!("Active until tick {end}")
    } else if tick >= end {
        format!("Expired at tick {end}")
    } else {
        format!("Scheduled for tick {}", hazard.start_tick.0)
    }
}

pub(super) fn fixed_hex(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut output = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        output.push(char::from(HEX[usize::from(byte >> 4)]));
        output.push(char::from(HEX[usize::from(byte & 0x0f)]));
    }
    output
}

pub(super) fn short_hex(bytes: &[u8]) -> String {
    fixed_hex(&bytes[..bytes.len().min(6)])
}
