//! Galaxy outliner entry derivation, split out of `workshop.rs`.

use nyon_workshop_core::{EntityId, WorkshopStateV1};

use super::{
    OutlinerEntry, OutlinerKind, WorkshopControl, WorkshopUiIntent, fixed_hex, hazard_status,
    short_hex,
};

pub(super) fn build_outliner(
    state: &WorkshopStateV1,
    selected: Option<EntityId>,
) -> Vec<OutlinerEntry> {
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
