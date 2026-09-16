//! Deterministic creator-form defaults and their prerequisite lookups, split
//! out of `workshop.rs`.

use nyon_workshop_core::{
    BatchLocalId, CatalogId, CreatorBatchV1, CreatorOpV1, EntityId, GalaxyPointV1, ObjectName,
    ObjectRefV1, WorkshopStateV1, WorkshopTick, model::EntityKind,
};

use crate::{
    ui::creator::{CreatorDraft, CreatorFieldKind, CreatorTool},
    workshop::session::{WorkshopAction, WorkshopSessionSnapshot},
};

use super::{
    CreatorFormError, CreatorFormFieldModel, CreatorFormModel, WorkshopControl, WorkshopUiIntent,
    is_removable, removal_blockers, short_hex,
};

/// Builds the concrete, single-operation batch preview submitted by a creator
/// form. The defaults are deliberately deterministic and are derived only from
/// the immutable session snapshot. A missing prerequisite keeps the form open
/// with an actionable validation message instead of mutating authority.
/// Action IDs of the creator dialog's fixed controls, and the shape of its field IDs.
///
/// `src/app.rs` opens the focus trap while handling `OpenCreatorForm`, before the model
/// and its semantic tree exist, so it cannot derive the order the way
/// `platform_projection::modal_action_ids` does. `ActiveCreatorEditor::modal_order` used
/// to rebuild these strings by hand instead; now it and the builder below read one
/// definition, so a rename here cannot leave the app focusing actions that no longer
/// exist. The removal dialog follows the same pattern in `removal.rs`.
pub(crate) const CREATOR_CANCEL_ACTION: &str = "creator.cancel";
pub(crate) const CREATOR_SUBMIT_ACTION: &str = "creator.submit";

pub(crate) fn creator_field_action_id(field_id: &str) -> String {
    format!("creator.field.{field_id}")
}

/// The creator dialog's focus order for a draft: its fields in order, then cancel,
/// then submit. Callers that *have* a frame should use `modal_action_ids` instead.
pub(crate) fn creator_modal_order(
    draft: Option<&CreatorDraft>,
) -> Vec<crate::ui::accessibility::SemanticActionId> {
    use crate::ui::accessibility::SemanticActionId;
    draft
        .into_iter()
        .flat_map(|draft| draft.fields.iter())
        .map(|field| SemanticActionId::new(creator_field_action_id(field.id)))
        .chain([
            SemanticActionId::new(CREATOR_CANCEL_ACTION),
            SemanticActionId::new(CREATOR_SUBMIT_ACTION),
        ])
        .collect()
}

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

pub(super) fn build_creator_form(
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
                            creator_field_action_id(field.id),
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
            CREATOR_CANCEL_ACTION,
            "Cancel",
            "Close the creator form without changing the Workshop.",
            true,
            false,
            WorkshopUiIntent::CloseCreatorForm,
        ),
        submit_control: WorkshopControl::new(
            CREATOR_SUBMIT_ACTION,
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

pub(in crate::ui) fn is_ownable(kind: Option<EntityKind>) -> bool {
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
