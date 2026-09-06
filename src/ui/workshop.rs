//! Immutable, render-platform-neutral UI model for Galaxy Workshop.
//!
//! Pointer input, keyboard input, DOM accessibility, and native accessibility
//! all address the same semantic action identifiers. Activating a control
//! returns a typed intent; it never mutates authoritative state directly.

use std::collections::{BTreeMap, BTreeSet};

use nyon_workshop_core::{
    BatchLocalId, BranchId, CatalogHash, CatalogId, CreatorBatchV1, CreatorOpV1, EntityId,
    GalaxyPointV1, ObjectName, ObjectRefV1, RevisionId, ValidatedCatalogPackV1, WorkshopStateV1,
    WorkshopTick, model::EntityKind,
};

use crate::{
    engine::backend::BackendKind,
    workshop::session::{
        WorkshopAction, WorkshopDiagnosticCode, WorkshopSessionSnapshot, WorkshopSpeed,
    },
};

use super::accessibility::{
    AnnouncementKind, InputModality, SemanticActionId, SemanticAnnouncement, SemanticNode,
    SemanticNodeId, SemanticRole, SemanticTree,
};
pub use super::creator::CreatorTool;
use super::creator::{CreatorDraft, CreatorFieldKind};
use super::workshop_inspector::build_inspector;

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

/// Builds the concrete, single-operation batch preview submitted by a creator
/// form. The defaults are deliberately deterministic and are derived only from
/// the immutable session snapshot. A missing prerequisite keeps the form open
/// with an actionable validation message instead of mutating authority.
pub fn default_creator_batch(
    snapshot: &WorkshopSessionSnapshot,
    selected: Option<EntityId>,
    tool: CreatorTool,
) -> Result<CreatorBatchV1, CreatorFormError> {
    let state = &snapshot.state;
    let local = BatchLocalId(1);
    let operation = match tool {
        CreatorTool::CreateFaction => CreatorOpV1::CreateFaction {
            local,
            name: object_name(format!("Faction {}", state.factions.len() + 1))?,
            color_rgb: deterministic_faction_color(state.factions.len()),
        },
        CreatorTool::CreateSystem => {
            let position = next_system_position(state)?;
            CreatorOpV1::CreateSystem {
                local,
                name: object_name(format!("System {}", state.systems.len() + 1))?,
                position,
            }
        }
        CreatorTool::CreateStar => {
            let system = selected_of_kind(state, selected, EntityKind::System)
                .or_else(|| state.systems.keys().next().copied())
                .ok_or(CreatorFormError::MissingPrerequisite(
                    "Create or select a system before adding a star.",
                ))?;
            CreatorOpV1::CreateStar {
                local,
                system: ObjectRefV1::Existing(system),
                name: object_name(format!("Star {}", state.stars.len() + 1))?,
                archetype_id: catalog_id("yellow-dwarf")?,
            }
        }
        CreatorTool::CreateWorld => {
            let (system, primary) =
                world_parent(state, selected).ok_or(CreatorFormError::MissingPrerequisite(
                    "Create a star, then select that star or its system before adding a world.",
                ))?;
            CreatorOpV1::CreateWorld {
                local,
                system: ObjectRefV1::Existing(system),
                primary: ObjectRefV1::Existing(primary),
                name: object_name(format!("World {}", state.worlds.len() + 1))?,
                archetype_id: catalog_id("rocky-world")?,
                orbit_radius_milli_au: 1_000,
                orbit_period_ticks: 3_600,
                phase_millidegrees: 0,
            }
        }
        CreatorTool::ConnectLane => {
            let pair =
                lane_candidate(state, selected).ok_or(CreatorFormError::MissingPrerequisite(
                    "Create two systems without an existing lane between them.",
                ))?;
            CreatorOpV1::ConnectLane {
                local,
                a: ObjectRefV1::Existing(pair.0),
                b: ObjectRefV1::Existing(pair.1),
            }
        }
        CreatorTool::CreateDeposit => {
            let world = selected_of_kind(state, selected, EntityKind::World)
                .or_else(|| state.worlds.keys().next().copied())
                .ok_or(CreatorFormError::MissingPrerequisite(
                    "Create or select a world before adding an ore deposit.",
                ))?;
            CreatorOpV1::CreateDeposit {
                local,
                world: ObjectRefV1::Existing(world),
                resource_id: catalog_id("ore")?,
                reserve_units: 1_000,
            }
        }
        CreatorTool::SetOwner => {
            let target = selected
                .filter(|entity| is_ownable(state.entity_kind(*entity)))
                .or_else(|| first_ownable(state))
                .ok_or(CreatorFormError::MissingPrerequisite(
                    "Create an ownable galaxy object before assigning ownership.",
                ))?;
            let faction = if state.owners.contains_key(&target) {
                None
            } else {
                Some(ObjectRefV1::Existing(
                    state.factions.keys().next().copied().ok_or(
                        CreatorFormError::MissingPrerequisite(
                            "Create a faction before assigning ownership.",
                        ),
                    )?,
                ))
            };
            CreatorOpV1::SetOwner {
                target: ObjectRefV1::Existing(target),
                faction,
            }
        }
        CreatorTool::PlaceIndustry => {
            let world =
                selected_world(state, selected).ok_or(CreatorFormError::MissingPrerequisite(
                    "Create or select a world before placing an industry.",
                ))?;
            let ore = catalog_id("ore")?;
            let deposit = state.deposits.iter().find_map(|(id, deposit)| {
                (deposit.world == world && deposit.resource_id == ore).then_some(*id)
            });
            let has_solar = state.industries.values().any(|industry| {
                industry.world == world && industry.definition_id.as_str() == "solar-array"
            });
            let has_extractor = state.industries.values().any(|industry| {
                industry.world == world && industry.definition_id.as_str() == "extractor"
            });
            let (definition_id, linked_deposit) = if !has_solar {
                (catalog_id("solar-array")?, None)
            } else if !has_extractor && deposit.is_some() {
                (catalog_id("extractor")?, deposit.map(ObjectRefV1::Existing))
            } else {
                (catalog_id("foundry")?, None)
            };
            CreatorOpV1::PlaceIndustry {
                local,
                world: ObjectRefV1::Existing(world),
                definition_id,
                linked_deposit,
            }
        }
        CreatorTool::ConnectRoute => {
            let (source, destination, resource) =
                route_candidate(state, selected).ok_or(CreatorFormError::MissingPrerequisite(
                    "Create two route-compatible worlds before connecting a resource route.",
                ))?;
            CreatorOpV1::ConnectRoute {
                local,
                source: ObjectRefV1::Existing(source),
                destination: ObjectRefV1::Existing(destination),
                resource_id: catalog_id(resource)?,
                batch_units: 1,
            }
        }
        CreatorTool::SetIndustryEnabled => {
            let industry = selected_of_kind(state, selected, EntityKind::Industry)
                .or_else(|| state.industries.keys().next().copied())
                .ok_or(CreatorFormError::MissingPrerequisite(
                    "Place or select an industry before changing its enabled state.",
                ))?;
            CreatorOpV1::SetIndustryEnabled {
                industry: ObjectRefV1::Existing(industry),
                enabled: !state.industries[&industry].enabled,
            }
        }
        CreatorTool::RenameObject => {
            let target = selected
                .filter(|entity| is_named(state.entity_kind(*entity)))
                .or_else(|| first_named(state))
                .ok_or(CreatorFormError::MissingPrerequisite(
                    "Create or select a named object before renaming it.",
                ))?;
            CreatorOpV1::RenameObject {
                target: ObjectRefV1::Existing(target),
                name: object_name(format!("Renamed {}", short_hex(&target.0)))?,
            }
        }
        CreatorTool::RemoveObject => {
            let target = selected
                .filter(|entity| is_removable(state.entity_kind(*entity)))
                .or_else(|| first_removable_without_blockers(state))
                .ok_or(CreatorFormError::MissingPrerequisite(
                    "Select a removable object. Remove its listed dependencies first.",
                ))?;
            if !removal_blockers(state, target).is_empty() {
                return Err(CreatorFormError::MissingPrerequisite(
                    "This object is still in use. Remove its listed dependencies first.",
                ));
            }
            CreatorOpV1::RemoveObject {
                target: ObjectRefV1::Existing(target),
            }
        }
        CreatorTool::ScheduleHazard => {
            let lane = selected_of_kind(state, selected, EntityKind::Lane)
                .or_else(|| state.lanes.keys().next().copied())
                .ok_or(CreatorFormError::MissingPrerequisite(
                    "Connect or select a lane before scheduling an ion storm.",
                ))?;
            CreatorOpV1::ScheduleHazard {
                local,
                lane: ObjectRefV1::Existing(lane),
                hazard_id: catalog_id("ion-storm")?,
                start_tick: WorkshopTick(
                    state
                        .tick
                        .0
                        .checked_add(1)
                        .ok_or(CreatorFormError::InvalidDefault)?,
                ),
                duration_ticks: 10,
            }
        }
        CreatorTool::CancelHazard => {
            let hazard = selected
                .filter(|entity| {
                    state
                        .hazards
                        .get(entity)
                        .is_some_and(|hazard| !hazard.cancelled)
                })
                .or_else(|| {
                    state
                        .hazards
                        .iter()
                        .find_map(|(id, hazard)| (!hazard.cancelled).then_some(*id))
                })
                .ok_or(CreatorFormError::MissingPrerequisite(
                    "Schedule or select a non-cancelled hazard first.",
                ))?;
            CreatorOpV1::CancelHazard {
                hazard: ObjectRefV1::Existing(hazard),
            }
        }
    };

    Ok(CreatorBatchV1 {
        expected_cursor: snapshot.active_view.view_cursor,
        expected_tick: snapshot.active_view.tick,
        operations: vec![operation],
    })
}

fn build_creator_form(
    snapshot: &WorkshopSessionSnapshot,
    selected: Option<EntityId>,
    tool: CreatorTool,
    supplied_draft: Option<&CreatorDraft>,
) -> CreatorFormModel {
    let generated_draft = if supplied_draft.is_some_and(|draft| draft.tool == tool) {
        None
    } else {
        default_creator_batch(snapshot, selected, tool)
            .ok()
            .and_then(|batch| CreatorDraft::from_batch(&snapshot.state, &batch).ok())
    };
    let draft = supplied_draft
        .filter(|draft| draft.tool == tool)
        .or(generated_draft.as_ref());
    let fields = draft
        .map(|draft| {
            draft
                .fields
                .iter()
                .map(|field| {
                    let cycle = field.kind == CreatorFieldKind::Choice;
                    CreatorFormFieldModel {
                        id: field.id.to_owned(),
                        label: field.label.to_owned(),
                        value: field.display_value(),
                        kind: field.kind,
                        control: WorkshopControl::new(
                            format!("creator.field.{}", field.id),
                            format!("{}: {}", field.label, field.display_value()),
                            if cycle {
                                "Activate to choose the next available value."
                            } else {
                                "Focus this field and type to replace its value."
                            },
                            true,
                            false,
                            WorkshopUiIntent::EditCreatorField {
                                field_id: field.id.to_owned(),
                                cycle,
                            },
                        ),
                    }
                })
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    let result = draft.map_or_else(
        || default_creator_batch(snapshot, selected, tool),
        |draft| {
            draft
                .to_batch()
                .map_err(|error| CreatorFormError::Draft(error.to_string()))
        },
    );
    let (preview, validation_message, enabled, intent) = match result {
        Ok(batch) => {
            let preview = batch
                .operations
                .first()
                .map(operation_preview)
                .into_iter()
                .collect();
            (
                preview,
                None,
                true,
                WorkshopUiIntent::Dispatch(WorkshopAction::Submit(batch)),
            )
        }
        Err(error) => {
            let message = error.to_string();
            (
                vec!["No authoritative edit will be submitted yet.".to_owned()],
                Some(message),
                false,
                WorkshopUiIntent::CloseCreatorForm,
            )
        }
    };
    CreatorFormModel {
        tool,
        title: tool.label().to_owned(),
        fields,
        preview,
        validation_message,
        cancel_control: WorkshopControl::new(
            "creator.cancel",
            "Cancel",
            "Close the creator form without changing the Workshop.",
            true,
            false,
            WorkshopUiIntent::CloseCreatorForm,
        ),
        submit_control: WorkshopControl::new(
            "creator.submit",
            format!("Apply {}", tool.label().to_ascii_lowercase()),
            "Submit this exact recorded creator batch through the typed Workshop mailbox.",
            enabled,
            false,
            intent,
        ),
    }
}

fn operation_preview(operation: &CreatorOpV1) -> String {
    format!(
        "Recorded operation: {}",
        CreatorTool::for_operation(operation).label()
    )
}

fn object_name(value: String) -> Result<ObjectName, CreatorFormError> {
    ObjectName::new(value).map_err(|_| CreatorFormError::InvalidDefault)
}

fn catalog_id(value: &'static str) -> Result<CatalogId, CreatorFormError> {
    CatalogId::new(value).map_err(|_| CreatorFormError::InvalidDefault)
}

fn deterministic_faction_color(index: usize) -> [u8; 3] {
    const COLORS: [[u8; 3]; 8] = [
        [0x4F, 0xA3, 0xFF],
        [0xFF, 0xA6, 0x2B],
        [0x75, 0xD6, 0x8A],
        [0xD8, 0x6B, 0xFF],
        [0xFF, 0x6B, 0x7A],
        [0x4D, 0xD9, 0xD0],
        [0xFF, 0xD1, 0x66],
        [0xAA, 0xB4, 0xC3],
    ];
    COLORS[index % COLORS.len()]
}

fn next_system_position(state: &WorkshopStateV1) -> Result<GalaxyPointV1, CreatorFormError> {
    for index in 0_i64..=128 {
        let coordinate = index
            .checked_mul(8_192)
            .ok_or(CreatorFormError::InvalidDefault)?;
        let candidate = GalaxyPointV1::new(coordinate, coordinate / 2)
            .map_err(|_| CreatorFormError::InvalidDefault)?;
        if state
            .systems
            .values()
            .all(|system| system.position != candidate)
        {
            return Ok(candidate);
        }
    }
    Err(CreatorFormError::InvalidDefault)
}

fn selected_of_kind(
    state: &WorkshopStateV1,
    selected: Option<EntityId>,
    kind: EntityKind,
) -> Option<EntityId> {
    selected.filter(|entity| state.entity_kind(*entity) == Some(kind))
}

fn selected_world(state: &WorkshopStateV1, selected: Option<EntityId>) -> Option<EntityId> {
    selected_of_kind(state, selected, EntityKind::World)
        .or_else(|| {
            selected.and_then(|entity| state.industries.get(&entity).map(|value| value.world))
        })
        .or_else(|| state.worlds.keys().next().copied())
}

fn world_parent(
    state: &WorkshopStateV1,
    selected: Option<EntityId>,
) -> Option<(EntityId, EntityId)> {
    if let Some(star) = selected_of_kind(state, selected, EntityKind::Star) {
        return Some((state.stars[&star].system, star));
    }
    let system = selected_of_kind(state, selected, EntityKind::System);
    state.stars.iter().find_map(|(star, value)| {
        (system.is_none() || system == Some(value.system)).then_some((value.system, *star))
    })
}

fn lane_candidate(
    state: &WorkshopStateV1,
    selected: Option<EntityId>,
) -> Option<(EntityId, EntityId)> {
    let preferred = selected_of_kind(state, selected, EntityKind::System);
    for a in preferred.into_iter().chain(state.systems.keys().copied()) {
        for b in state.systems.keys().copied() {
            if a != b
                && !state
                    .lanes
                    .values()
                    .any(|lane| (lane.a == a && lane.b == b) || (lane.a == b && lane.b == a))
            {
                return Some((a, b));
            }
        }
    }
    None
}

fn route_candidate(
    state: &WorkshopStateV1,
    selected: Option<EntityId>,
) -> Option<(EntityId, EntityId, &'static str)> {
    let preferred = selected_world(state, selected);
    for source in preferred.into_iter().chain(state.worlds.keys().copied()) {
        for destination in state.worlds.keys().copied() {
            if source == destination || !worlds_route_compatible(state, source, destination) {
                continue;
            }
            for resource in ["energy", "ore", "alloy"] {
                if !state.routes.values().any(|route| {
                    route.source == source
                        && route.destination == destination
                        && route.resource_id.as_str() == resource
                }) {
                    return Some((source, destination, resource));
                }
            }
        }
    }
    None
}

fn worlds_route_compatible(
    state: &WorkshopStateV1,
    source: EntityId,
    destination: EntityId,
) -> bool {
    let source_system = state.worlds[&source].system;
    let destination_system = state.worlds[&destination].system;
    source_system == destination_system
        || state.lanes.values().any(|lane| {
            (lane.a == source_system && lane.b == destination_system)
                || (lane.a == destination_system && lane.b == source_system)
        })
}

pub(super) fn is_ownable(kind: Option<EntityKind>) -> bool {
    matches!(
        kind,
        Some(
            EntityKind::System
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

fn is_named(kind: Option<EntityKind>) -> bool {
    matches!(
        kind,
        Some(EntityKind::Faction | EntityKind::System | EntityKind::Star | EntityKind::World)
    )
}

fn first_ownable(state: &WorkshopStateV1) -> Option<EntityId> {
    state
        .systems
        .keys()
        .chain(state.stars.keys())
        .chain(state.worlds.keys())
        .chain(state.lanes.keys())
        .chain(state.deposits.keys())
        .chain(state.industries.keys())
        .chain(state.routes.keys())
        .chain(state.hazards.keys())
        .next()
        .copied()
}

fn first_named(state: &WorkshopStateV1) -> Option<EntityId> {
    state
        .factions
        .keys()
        .chain(state.systems.keys())
        .chain(state.stars.keys())
        .chain(state.worlds.keys())
        .next()
        .copied()
}

fn first_removable_without_blockers(state: &WorkshopStateV1) -> Option<EntityId> {
    state
        .factions
        .keys()
        .chain(state.systems.keys())
        .chain(state.stars.keys())
        .chain(state.worlds.keys())
        .chain(state.lanes.keys())
        .chain(state.deposits.keys())
        .chain(state.industries.keys())
        .chain(state.routes.keys())
        .chain(state.hazards.keys())
        .copied()
        .find(|entity| removal_blockers(state, *entity).is_empty())
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

fn build_timeline(
    snapshot: &WorkshopSessionSnapshot,
    redo_children: &[RevisionId],
    paused: bool,
    replacement_active: bool,
) -> TimelineModel {
    let mut controls = vec![
        WorkshopControl::new(
            "timeline.pause",
            "Pause",
            "Pause authoritative Workshop stepping.",
            !paused,
            paused,
            WorkshopUiIntent::Dispatch(WorkshopAction::Pause),
        ),
        WorkshopControl::new(
            "timeline.step",
            "Step once",
            "Advance exactly one authoritative tick while paused.",
            paused && !replacement_active,
            false,
            WorkshopUiIntent::Dispatch(WorkshopAction::StepOnce),
        ),
        speed_control("1x", WorkshopSpeed::One, snapshot, replacement_active),
        speed_control("4x", WorkshopSpeed::Four, snapshot, replacement_active),
        speed_control("20x", WorkshopSpeed::Twenty, snapshot, replacement_active),
        WorkshopControl::new(
            "timeline.undo",
            "Undo",
            "Browse the immutable parent revision at the current tick.",
            paused && !replacement_active && snapshot.active_view.view_cursor.is_some(),
            false,
            WorkshopUiIntent::Dispatch(WorkshopAction::Undo),
        ),
    ];

    for revision in redo_children {
        controls.push(WorkshopControl::new(
            format!("timeline.redo.{}", fixed_hex(&revision.0)),
            format!("Redo {}", short_hex(&revision.0)),
            "Select this existing child revision without rewriting history.",
            paused && !replacement_active,
            false,
            WorkshopUiIntent::Dispatch(WorkshopAction::Redo(*revision)),
        ));
    }

    TimelineModel {
        tick: snapshot.state.tick.0,
        speed: snapshot.speed,
        controls,
        redo_children: redo_children.to_vec(),
    }
}

fn speed_control(
    label: &'static str,
    speed: WorkshopSpeed,
    snapshot: &WorkshopSessionSnapshot,
    replacement_active: bool,
) -> WorkshopControl {
    WorkshopControl::new(
        format!("timeline.speed.{}", label.to_ascii_lowercase()),
        label,
        format!("Run the Workshop simulation at {label}."),
        !snapshot.store.load_pending && !replacement_active,
        snapshot.speed == speed,
        WorkshopUiIntent::Dispatch(WorkshopAction::Resume(speed)),
    )
}

fn build_branches(
    snapshot: &WorkshopSessionSnapshot,
    paused: bool,
    replacement_active: bool,
) -> Vec<BranchChoice> {
    snapshot
        .branches
        .iter()
        .map(|branch| {
            let selected = branch.id == snapshot.active_view.selected_branch;
            let cursor = selected
                .then_some(snapshot.active_view.view_cursor)
                .flatten();
            let browsing_history = selected && branch.head != snapshot.active_view.view_cursor;
            let description = if browsing_history {
                format!(
                    "Selected branch; cursor {} is behind head {}.",
                    optional_revision(cursor),
                    optional_revision(branch.head)
                )
            } else {
                format!(
                    "Branch head {}; last authoritative tick {}.",
                    optional_revision(branch.head),
                    branch.last_tick.0
                )
            };
            BranchChoice {
                branch: branch.id,
                name: branch.name.clone(),
                head: branch.head,
                selected_cursor: cursor,
                last_tick: branch.last_tick.0,
                selected,
                browsing_history,
                control: WorkshopControl::new(
                    format!("branch.{}", fixed_hex(&branch.id.0)),
                    branch.name.clone(),
                    description,
                    paused && !replacement_active,
                    selected,
                    WorkshopUiIntent::Dispatch(WorkshopAction::SelectBranch(branch.id)),
                ),
            }
        })
        .collect()
}

fn build_save_status(snapshot: &WorkshopSessionSnapshot) -> SaveStatusModel {
    SaveStatusModel {
        slot: snapshot.store.slot.map(|slot| slot.0),
        generation: snapshot.store.generation.map(|generation| generation.0),
        dirty: snapshot.store.dirty,
        commit_pending: snapshot.store.commit_pending,
        load_pending: snapshot.store.load_pending,
        status: safe_store_status(&snapshot.store.status).to_owned(),
        save_control: WorkshopControl::new(
            "save.commit",
            "Save Workshop",
            "Commit the current archive using generation compare-and-swap.",
            !snapshot.store.commit_pending && !snapshot.store.load_pending,
            false,
            WorkshopUiIntent::Dispatch(WorkshopAction::RequestSave),
        ),
    }
}

fn build_outliner(state: &WorkshopStateV1, selected: Option<EntityId>) -> Vec<OutlinerEntry> {
    let mut entries = Vec::with_capacity(
        state.factions.len()
            + state.systems.len()
            + state.stars.len()
            + state.worlds.len()
            + state.deposits.len()
            + state.industries.len()
            + state.lanes.len()
            + state.routes.len()
            + state.shipments.len()
            + state.hazards.len(),
    );

    for (id, faction) in &state.factions {
        push_outliner(
            &mut entries,
            *id,
            None,
            0,
            OutlinerKind::Faction,
            faction.name.to_string(),
            format!(
                "Ownership color #{:02X}{:02X}{:02X}",
                faction.color_rgb[0], faction.color_rgb[1], faction.color_rgb[2]
            ),
            selected,
        );
    }

    for (system_id, system) in &state.systems {
        push_outliner(
            &mut entries,
            *system_id,
            None,
            0,
            OutlinerKind::System,
            system.name.to_string(),
            format!(
                "Galaxy coordinates {}, {}",
                system.position.x.get(),
                system.position.y.get()
            ),
            selected,
        );
        for (star_id, star) in state
            .stars
            .iter()
            .filter(|(_, star)| star.system == *system_id)
        {
            push_outliner(
                &mut entries,
                *star_id,
                Some(*system_id),
                1,
                OutlinerKind::Star,
                star.name.to_string(),
                format!("{} archetype", star.archetype_id),
                selected,
            );
        }
        for (world_id, world) in state
            .worlds
            .iter()
            .filter(|(_, world)| world.system == *system_id)
        {
            push_outliner(
                &mut entries,
                *world_id,
                Some(*system_id),
                1,
                OutlinerKind::World,
                world.name.to_string(),
                format!("{} archetype", world.archetype_id),
                selected,
            );
            for (deposit_id, deposit) in state
                .deposits
                .iter()
                .filter(|(_, deposit)| deposit.world == *world_id)
            {
                push_outliner(
                    &mut entries,
                    *deposit_id,
                    Some(*world_id),
                    2,
                    OutlinerKind::Deposit,
                    format!("{} deposit", deposit.resource_id),
                    format!("{} reserve units", deposit.reserve_units),
                    selected,
                );
            }
            for (industry_id, industry) in state
                .industries
                .iter()
                .filter(|(_, industry)| industry.world == *world_id)
            {
                push_outliner(
                    &mut entries,
                    *industry_id,
                    Some(*world_id),
                    2,
                    OutlinerKind::Industry,
                    industry.definition_id.to_string(),
                    if industry.enabled {
                        "Enabled"
                    } else {
                        "Disabled"
                    }
                    .to_owned(),
                    selected,
                );
            }
        }
    }

    for (lane_id, lane) in &state.lanes {
        push_outliner(
            &mut entries,
            *lane_id,
            None,
            0,
            OutlinerKind::Lane,
            format!("Lane {}", short_hex(&lane_id.0)),
            format!("{} distance units", lane.distance_units),
            selected,
        );
        for (hazard_id, hazard) in state
            .hazards
            .iter()
            .filter(|(_, hazard)| hazard.lane == *lane_id)
        {
            push_outliner(
                &mut entries,
                *hazard_id,
                Some(*lane_id),
                1,
                OutlinerKind::Hazard,
                hazard.hazard_id.to_string(),
                hazard_status(hazard, state.tick.0),
                selected,
            );
        }
    }

    for (route_id, route) in &state.routes {
        push_outliner(
            &mut entries,
            *route_id,
            None,
            0,
            OutlinerKind::Route,
            format!("{} route", route.resource_id),
            format!("{} units per dispatch", route.batch_units),
            selected,
        );
        for (shipment_id, shipment) in state
            .shipments
            .iter()
            .filter(|(_, shipment)| shipment.route == *route_id)
        {
            push_outliner(
                &mut entries,
                *shipment_id,
                Some(*route_id),
                1,
                OutlinerKind::Shipment,
                format!("{} shipment", shipment.resource_id),
                format!(
                    "{} units arriving at tick {}",
                    shipment.units, shipment.arrival_tick.0
                ),
                selected,
            );
        }
    }

    entries
}

#[allow(clippy::too_many_arguments)]
fn push_outliner(
    entries: &mut Vec<OutlinerEntry>,
    entity: EntityId,
    parent: Option<EntityId>,
    depth: u32,
    kind: OutlinerKind,
    name: String,
    description: String,
    selected: Option<EntityId>,
) {
    let is_selected = selected == Some(entity);
    entries.push(OutlinerEntry {
        entity,
        parent,
        depth,
        kind,
        name: name.clone(),
        description: description.clone(),
        control: WorkshopControl::new(
            format!("outliner.{}", fixed_hex(&entity.0)),
            name,
            format!("{}: {description}", kind.label()),
            true,
            is_selected,
            WorkshopUiIntent::SelectEntity(entity),
        ),
    });
}

fn build_removal_confirmation(
    snapshot: &WorkshopSessionSnapshot,
    target: EntityId,
) -> RemovalConfirmation {
    let blockers = removal_blockers(&snapshot.state, target)
        .into_iter()
        .map(|entity| RemovalBlocker {
            entity,
            label: entity_display_name(&snapshot.state, entity),
        })
        .collect::<Vec<_>>();
    let can_remove = blockers.is_empty();
    RemovalConfirmation {
        target,
        target_label: entity_display_name(&snapshot.state, target),
        blockers,
        cancel_control: WorkshopControl::new(
            "remove.cancel",
            "Cancel removal",
            "Close this confirmation without changing the Workshop.",
            true,
            false,
            WorkshopUiIntent::CloseRemovalConfirmation,
        ),
        confirm_control: WorkshopControl::new(
            "remove.confirm",
            "Remove object",
            if can_remove {
                "Submit one explicit non-cascading removal batch."
            } else {
                "Remove the listed dependencies before removing this object."
            },
            can_remove,
            false,
            WorkshopUiIntent::Dispatch(WorkshopAction::Submit(CreatorBatchV1 {
                expected_cursor: snapshot.active_view.view_cursor,
                expected_tick: snapshot.active_view.tick,
                operations: vec![CreatorOpV1::RemoveObject {
                    target: ObjectRefV1::Existing(target),
                }],
            })),
        ),
    }
}

pub fn removal_blockers(state: &WorkshopStateV1, target: EntityId) -> Vec<EntityId> {
    let mut found = BTreeSet::new();
    if state.factions.contains_key(&target) {
        found.extend(
            state
                .owners
                .iter()
                .filter_map(|(entity, owner)| (*owner == target).then_some(*entity)),
        );
    }
    for (id, star) in &state.stars {
        if star.system == target {
            found.insert(*id);
        }
    }
    for (id, world) in &state.worlds {
        if world.system == target || world.primary == target {
            found.insert(*id);
        }
    }
    for (id, lane) in &state.lanes {
        if lane.a == target || lane.b == target {
            found.insert(*id);
        }
    }
    for (id, deposit) in &state.deposits {
        if deposit.world == target {
            found.insert(*id);
        }
    }
    for (id, industry) in &state.industries {
        if industry.world == target || industry.linked_deposit == Some(target) {
            found.insert(*id);
        }
    }
    for (id, route) in &state.routes {
        if route.source == target || route.destination == target {
            found.insert(*id);
        }
    }
    if let Some(lane) = state.lanes.get(&target) {
        for (id, route) in &state.routes {
            let Some(source) = state.worlds.get(&route.source) else {
                continue;
            };
            let Some(destination) = state.worlds.get(&route.destination) else {
                continue;
            };
            if (source.system == lane.a && destination.system == lane.b)
                || (source.system == lane.b && destination.system == lane.a)
            {
                found.insert(*id);
            }
        }
    }
    for (id, shipment) in &state.shipments {
        if shipment.route == target || shipment.source == target || shipment.destination == target {
            found.insert(*id);
        }
    }
    for (id, hazard) in &state.hazards {
        if hazard.lane == target {
            found.insert(*id);
        }
    }
    found.into_iter().collect()
}

#[allow(clippy::too_many_arguments)]
fn all_controls<'a>(
    menu: &'a WorkshopControl,
    tools: &'a [CreatorToolControl],
    outliner: &'a [OutlinerEntry],
    inspector: &'a InspectorModel,
    timeline: &'a TimelineModel,
    branches: &'a [BranchChoice],
    save: &'a SaveStatusModel,
    removal: Option<&'a RemovalConfirmation>,
    preferences: &'a WorkshopPresentationPreferences,
    creator: Option<&'a CreatorFormModel>,
) -> Vec<&'a WorkshopControl> {
    let mut controls = vec![menu];
    controls.extend(tools.iter().map(|tool| &tool.control));
    controls.extend(outliner.iter().map(|entry| &entry.control));
    if let Some(control) = &inspector.remove_control {
        controls.push(control);
    }
    controls.extend(&timeline.controls);
    controls.extend(branches.iter().map(|branch| &branch.control));
    controls.push(&save.save_control);
    controls.push(&preferences.reduced_motion_control);
    controls.push(&preferences.high_contrast_control);
    if let Some(removal) = removal {
        controls.push(&removal.cancel_control);
        controls.push(&removal.confirm_control);
    }
    if let Some(creator) = creator {
        controls.extend(creator.fields.iter().map(|field| &field.control));
        controls.push(&creator.cancel_control);
        controls.push(&creator.submit_control);
    }
    controls
}

#[allow(clippy::too_many_arguments)]
fn build_semantic_tree(
    snapshot: &WorkshopSessionSnapshot,
    menu: &WorkshopControl,
    tools: &[CreatorToolControl],
    outliner: &[OutlinerEntry],
    inspector: &InspectorModel,
    timeline: &TimelineModel,
    branches: &[BranchChoice],
    save: &SaveStatusModel,
    diagnostics: &WorkshopDiagnosticsModel,
    removal: Option<&RemovalConfirmation>,
    preferences: &WorkshopPresentationPreferences,
    creator: Option<&CreatorFormModel>,
) -> SemanticTree {
    let tool_nodes = tools
        .iter()
        .map(|tool| semantic_control("tool", &tool.control, SemanticRole::Button))
        .collect();
    let mut outliner_nodes = outliner
        .iter()
        .map(|entry| {
            let mut node = semantic_control("outliner", &entry.control, SemanticRole::TreeItem);
            node.hierarchy_level = Some(entry.depth + 1);
            node
        })
        .collect::<Vec<_>>();
    if outliner_nodes.is_empty() {
        outliner_nodes.push(SemanticNode::text(
            "outliner.empty",
            SemanticRole::Text,
            "No galaxy objects",
            "Use the creator tools to begin shaping the galaxy.",
        ));
    }

    let mut inspector_nodes = inspector
        .sections
        .iter()
        .map(|section| {
            let rows = section
                .rows
                .iter()
                .map(|row| {
                    SemanticNode::text(
                        row.semantic_id.as_str(),
                        SemanticRole::Text,
                        &row.label,
                        &row.value,
                    )
                })
                .collect();
            SemanticNode::container(
                section.semantic_id.as_str(),
                SemanticRole::Group,
                section.heading,
                rows,
            )
        })
        .chain(
            inspector
                .remove_control
                .iter()
                .map(|control| semantic_control("inspector", control, SemanticRole::Button)),
        )
        .collect::<Vec<_>>();
    inspector_nodes.insert(
        0,
        SemanticNode::text(
            "inspector.title",
            SemanticRole::Heading,
            format!("Inspector: {}", inspector.title),
            "",
        ),
    );

    let timeline_nodes = timeline
        .controls
        .iter()
        .map(|control| semantic_control("timeline", control, SemanticRole::Button))
        .collect();
    let branch_nodes = branches
        .iter()
        .map(|branch| semantic_control("branches", &branch.control, SemanticRole::Option))
        .collect();
    let save_nodes = vec![
        SemanticNode::text(
            "save.status",
            SemanticRole::Status,
            "Save status",
            format!(
                "{}; {}; generation {}",
                save.status,
                if save.dirty {
                    "unsaved changes"
                } else {
                    "clean"
                },
                save.generation
                    .map_or_else(|| "none".to_owned(), |value| value.to_string())
            ),
        ),
        semantic_control("save", &save.save_control, SemanticRole::Button),
    ];
    let diagnostic_nodes = vec![
        SemanticNode::text(
            "diagnostics.backend",
            SemanticRole::Text,
            "Graphics backend",
            &diagnostics.backend,
        ),
        SemanticNode::text(
            "diagnostics.catalog",
            SemanticRole::Text,
            "Catalog hash",
            &diagnostics.catalog_hash,
        ),
        SemanticNode::text(
            "diagnostics.digest",
            SemanticRole::Text,
            "State digest",
            &diagnostics.state_digest,
        ),
        SemanticNode::text(
            "diagnostics.tick",
            SemanticRole::Text,
            "Authoritative tick",
            diagnostics.tick.to_string(),
        ),
    ];
    let preference_nodes = vec![
        semantic_control(
            "preferences",
            &preferences.reduced_motion_control,
            SemanticRole::Checkbox,
        ),
        semantic_control(
            "preferences",
            &preferences.high_contrast_control,
            SemanticRole::Checkbox,
        ),
    ];

    let mut children = vec![
        semantic_control("workshop", menu, SemanticRole::Button),
        SemanticNode::container(
            "workshop.tools",
            SemanticRole::Toolbar,
            "Creator tools",
            tool_nodes,
        ),
        SemanticNode::container(
            "workshop.outliner",
            SemanticRole::Tree,
            "Galaxy hierarchy",
            outliner_nodes,
        ),
        SemanticNode::container(
            "workshop.inspector",
            SemanticRole::Region,
            format!("Inspector: {}", inspector.title),
            inspector_nodes,
        ),
        SemanticNode::container(
            "workshop.timeline",
            SemanticRole::Toolbar,
            format!("Timeline at tick {}", timeline.tick),
            timeline_nodes,
        ),
        SemanticNode::container(
            "workshop.branches",
            SemanticRole::List,
            "History branches",
            branch_nodes,
        ),
        SemanticNode::container(
            "workshop.save",
            SemanticRole::Region,
            "Workshop save",
            save_nodes,
        ),
        SemanticNode::container(
            "workshop.diagnostics",
            SemanticRole::Region,
            "Workshop diagnostics",
            diagnostic_nodes,
        ),
        SemanticNode::container(
            "workshop.preferences",
            SemanticRole::Group,
            "Presentation accessibility",
            preference_nodes,
        ),
    ];

    if let Some(removal) = removal {
        let mut removal_nodes = removal
            .blockers
            .iter()
            .enumerate()
            .map(|(index, blocker)| {
                SemanticNode::text(
                    format!("removal.blocker.{index}"),
                    SemanticRole::ListItem,
                    &blocker.label,
                    format!("Blocking entity {}", fixed_hex(&blocker.entity.0)),
                )
            })
            .collect::<Vec<_>>();
        removal_nodes.push(semantic_control(
            "removal",
            &removal.cancel_control,
            SemanticRole::Button,
        ));
        removal_nodes.push(semantic_control(
            "removal",
            &removal.confirm_control,
            SemanticRole::Button,
        ));
        children.push(SemanticNode::container(
            "workshop.removal-dialog",
            SemanticRole::Dialog,
            format!("Remove {}", removal.target_label),
            removal_nodes,
        ));
    }

    if let Some(creator) = creator {
        let mut creator_nodes = creator
            .fields
            .iter()
            .map(|field| {
                let mut node = semantic_control(
                    "creator-field",
                    &field.control,
                    match field.kind {
                        CreatorFieldKind::Text => SemanticRole::TextInput,
                        CreatorFieldKind::Integer => SemanticRole::SpinButton,
                        CreatorFieldKind::Choice => SemanticRole::Option,
                    },
                );
                node.value = Some(field.value.clone());
                node
            })
            .collect::<Vec<_>>();
        creator_nodes.extend(creator.preview.iter().enumerate().map(|(index, line)| {
            SemanticNode::text(
                format!("creator.preview.{index}"),
                SemanticRole::Text,
                "Creator batch preview",
                line,
            )
        }));
        if let Some(message) = &creator.validation_message {
            creator_nodes.push(SemanticNode::text(
                "creator.validation",
                SemanticRole::Alert,
                "Creator form needs attention",
                message,
            ));
        }
        creator_nodes.push(semantic_control(
            "creator",
            &creator.cancel_control,
            SemanticRole::Button,
        ));
        creator_nodes.push(semantic_control(
            "creator",
            &creator.submit_control,
            SemanticRole::Button,
        ));
        children.push(SemanticNode::container(
            "workshop.creator-dialog",
            SemanticRole::Dialog,
            format!("{} editor", creator.title),
            creator_nodes,
        ));
    }

    SemanticTree {
        root: SemanticNode::container(
            "workshop.application",
            SemanticRole::Application,
            "NYON Galaxy Workshop",
            children,
        ),
        announcements: semantic_announcements(snapshot),
    }
}

fn semantic_control(prefix: &str, control: &WorkshopControl, role: SemanticRole) -> SemanticNode {
    SemanticNode::control(
        format!("{prefix}.control.{}", control.action_id.as_str()),
        role,
        &control.label,
        &control.description,
        control.enabled,
        control.selected,
        control.action_id.clone(),
    )
}

fn semantic_announcements(snapshot: &WorkshopSessionSnapshot) -> Vec<SemanticAnnouncement> {
    let mut announcements = snapshot
        .diagnostics
        .iter()
        .map(|diagnostic| {
            let (code, message) = safe_diagnostic(diagnostic.code);
            SemanticAnnouncement {
                kind: AnnouncementKind::Error,
                code,
                message: message.to_owned(),
            }
        })
        .collect::<Vec<_>>();
    announcements.push(SemanticAnnouncement {
        kind: AnnouncementKind::Status,
        code: "workshop-store-status",
        message: safe_store_status(&snapshot.store.status).to_owned(),
    });
    announcements
}

fn safe_diagnostic(code: WorkshopDiagnosticCode) -> (&'static str, &'static str) {
    match code {
        WorkshopDiagnosticCode::ActionQueueFull => (
            "action-queue-full",
            "The Workshop action could not be queued.",
        ),
        WorkshopDiagnosticCode::CreatorRejected => (
            "creator-rejected",
            "The creator edit was rejected. Review its fields and dependencies.",
        ),
        WorkshopDiagnosticCode::HistoryRejected => (
            "history-rejected",
            "The history action is unavailable in the current Workshop state.",
        ),
        WorkshopDiagnosticCode::SimulationFault => (
            "simulation-fault",
            "The deterministic simulation paused after a recoverable fault.",
        ),
        WorkshopDiagnosticCode::ArchiveRejected => (
            "archive-rejected",
            "The Workshop archive could not be validated.",
        ),
        WorkshopDiagnosticCode::StoreRejected => (
            "store-rejected",
            "The Workshop storage operation did not complete.",
        ),
        WorkshopDiagnosticCode::StoreProtocol => (
            "store-protocol",
            "The Workshop storage adapter returned an unexpected result.",
        ),
    }
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

fn optional_revision(revision: Option<RevisionId>) -> String {
    revision.map_or_else(|| "genesis".to_owned(), |id| short_hex(&id.0))
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
