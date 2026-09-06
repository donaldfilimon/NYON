use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::{
    ids::{CatalogId, EntityId, StateDigest, WorkshopTick},
    model::{
        DepositV1, EntityKind, HazardV1, IndustryV1, MAX_SHIPMENTS, RouteV1, ShipmentV1,
        WorkshopStateV1, WorldV1,
    },
    pack::{CatalogDefinitionKind, IndustryDefinitionV1, QuantityV1, ValidatedCatalogPackV1},
};

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum TickEventV1 {
    HazardActivated {
        hazard: EntityId,
    },
    HazardExpired {
        hazard: EntityId,
    },
    ShipmentDelivered {
        shipment: EntityId,
        destination: EntityId,
        resource_id: CatalogId,
        units: u64,
    },
    IndustryRan {
        industry: EntityId,
    },
    RouteDispatched {
        route: EntityId,
        shipment: EntityId,
        units: u64,
        arrival_tick: WorkshopTick,
    },
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct TickReceiptV1 {
    pub tick: WorkshopTick,
    pub events: Vec<TickEventV1>,
    pub events_digest: [u8; 32],
    pub post_tick_digest: StateDigest,
}

#[derive(Clone, Debug, Eq, PartialEq, thiserror::Error)]
pub enum DeterministicFault {
    #[error("checked deterministic arithmetic failed")]
    Arithmetic,
    #[error("authoritative tick space is exhausted")]
    TickExhausted,
    #[error("in-flight shipment capacity is exhausted")]
    ShipmentCapacity,
    #[error("a deterministic shipment identifier collided")]
    IdentityCollision,
    #[error("authoritative state references missing or incompatible content")]
    Invariant,
    #[error("canonical tick encoding failed")]
    Encoding,
}

struct StepCandidate {
    tick: WorkshopTick,
    worlds: BTreeMap<EntityId, WorldV1>,
    deposits: BTreeMap<EntityId, DepositV1>,
    shipments: BTreeMap<EntityId, ShipmentV1>,
}

pub(crate) fn step_state(
    catalog: &ValidatedCatalogPackV1,
    state: &mut WorkshopStateV1,
) -> Result<TickReceiptV1, DeterministicFault> {
    let mut candidate = StepCandidate {
        tick: state.tick,
        worlds: state.worlds.clone(),
        deposits: state.deposits.clone(),
        shipments: state.shipments.clone(),
    };
    let tick = candidate.tick;
    let mut events = Vec::new();

    hazard_boundaries(tick, &state.hazards, &mut events)?;
    deliver_shipments(
        tick,
        &mut candidate.shipments,
        &mut candidate.worlds,
        &mut events,
    )?;
    run_primary_industries(
        catalog,
        tick,
        &state.industries,
        &mut candidate.worlds,
        &mut candidate.deposits,
        &mut events,
    )?;
    run_industries(
        catalog,
        tick,
        &state.industries,
        &mut candidate.worlds,
        &mut candidate.deposits,
        CatalogDefinitionKind::Processor,
        &mut events,
    )?;
    let lane_index = build_lane_index(state);
    let active_hazards = active_hazard_ratios(catalog, state, tick)?;
    let mut occupied_ids = candidate_entity_ids(state, &candidate);
    dispatch_routes(
        &state.routes,
        &lane_index,
        &active_hazards,
        &mut candidate,
        &mut occupied_ids,
        &mut events,
    )?;

    let events_bytes =
        serde_json::to_vec(&(tick, &events)).map_err(|_| DeterministicFault::Encoding)?;
    let mut event_hash = Sha256::new();
    event_hash.update(b"NYON-WORKSHOP-TICK-EVENTS-V1\0");
    event_hash.update(events_bytes);
    let events_digest = event_hash.finalize().into();
    candidate.tick = WorkshopTick(
        tick.0
            .checked_add(1)
            .ok_or(DeterministicFault::TickExhausted)?,
    );
    let post_tick_digest = state.digest_with_dynamic(
        candidate.tick,
        &candidate.worlds,
        &candidate.deposits,
        &candidate.shipments,
    );
    state.tick = candidate.tick;
    state.worlds = candidate.worlds;
    state.deposits = candidate.deposits;
    state.shipments = candidate.shipments;
    Ok(TickReceiptV1 {
        tick,
        events,
        events_digest,
        post_tick_digest,
    })
}

fn hazard_boundaries(
    tick: WorkshopTick,
    hazards: &BTreeMap<EntityId, HazardV1>,
    events: &mut Vec<TickEventV1>,
) -> Result<(), DeterministicFault> {
    for (id, hazard) in hazards {
        if hazard.cancelled {
            continue;
        }
        if hazard.start_tick == tick {
            events.push(TickEventV1::HazardActivated { hazard: *id });
        }
        let end = hazard
            .start_tick
            .0
            .checked_add(hazard.duration_ticks)
            .ok_or(DeterministicFault::Arithmetic)?;
        if end == tick.0 {
            events.push(TickEventV1::HazardExpired { hazard: *id });
        }
    }
    Ok(())
}

fn deliver_shipments(
    tick: WorkshopTick,
    shipments: &mut BTreeMap<EntityId, ShipmentV1>,
    worlds: &mut BTreeMap<EntityId, WorldV1>,
    events: &mut Vec<TickEventV1>,
) -> Result<(), DeterministicFault> {
    let deliveries: Vec<EntityId> = shipments
        .iter()
        .filter_map(|(id, shipment)| (shipment.arrival_tick == tick).then_some(*id))
        .collect();
    for id in deliveries {
        let shipment = shipments.remove(&id).ok_or(DeterministicFault::Invariant)?;
        let world = worlds
            .get_mut(&shipment.destination)
            .ok_or(DeterministicFault::Invariant)?;
        add_inventory(&mut world.inventory, &shipment.resource_id, shipment.units)?;
        events.push(TickEventV1::ShipmentDelivered {
            shipment: id,
            destination: shipment.destination,
            resource_id: shipment.resource_id,
            units: shipment.units,
        });
    }
    Ok(())
}

fn run_industries(
    catalog: &ValidatedCatalogPackV1,
    tick: WorkshopTick,
    industries: &BTreeMap<EntityId, IndustryV1>,
    worlds: &mut BTreeMap<EntityId, WorldV1>,
    deposits: &mut BTreeMap<EntityId, DepositV1>,
    phase: CatalogDefinitionKind,
    events: &mut Vec<TickEventV1>,
) -> Result<(), DeterministicFault> {
    for (id, industry) in industries {
        if !industry.enabled {
            continue;
        }
        let definition = catalog
            .industry(&industry.definition_id)
            .ok_or(DeterministicFault::Invariant)?;
        if definition.kind() != phase || !tick.0.is_multiple_of(definition.duration_ticks()) {
            continue;
        }
        let ran = match definition {
            IndustryDefinitionV1::Solar { outputs, .. } => {
                apply_recipe(worlds, industry.world, &[], outputs)?
            }
            IndustryDefinitionV1::Extractor {
                energy_input,
                output,
                ..
            } => run_extractor(worlds, deposits, industry, energy_input, output)?,
            IndustryDefinitionV1::Processor {
                inputs, outputs, ..
            } => apply_recipe(worlds, industry.world, inputs, outputs)?,
        };
        if ran {
            events.push(TickEventV1::IndustryRan { industry: *id });
        }
    }
    Ok(())
}

fn run_primary_industries(
    catalog: &ValidatedCatalogPackV1,
    tick: WorkshopTick,
    industries: &BTreeMap<EntityId, IndustryV1>,
    worlds: &mut BTreeMap<EntityId, WorldV1>,
    deposits: &mut BTreeMap<EntityId, DepositV1>,
    events: &mut Vec<TickEventV1>,
) -> Result<(), DeterministicFault> {
    for (id, industry) in industries {
        if !industry.enabled {
            continue;
        }
        let definition = catalog
            .industry(&industry.definition_id)
            .ok_or(DeterministicFault::Invariant)?;
        if definition.kind() == CatalogDefinitionKind::Processor
            || !tick.0.is_multiple_of(definition.duration_ticks())
        {
            continue;
        }
        let ran = match definition {
            IndustryDefinitionV1::Solar { outputs, .. } => {
                apply_recipe(worlds, industry.world, &[], outputs)?
            }
            IndustryDefinitionV1::Extractor {
                energy_input,
                output,
                ..
            } => run_extractor(worlds, deposits, industry, energy_input, output)?,
            IndustryDefinitionV1::Processor { .. } => false,
        };
        if ran {
            events.push(TickEventV1::IndustryRan { industry: *id });
        }
    }
    Ok(())
}

fn apply_recipe(
    worlds: &mut BTreeMap<EntityId, WorldV1>,
    world_id: EntityId,
    inputs: &[QuantityV1],
    outputs: &[QuantityV1],
) -> Result<bool, DeterministicFault> {
    let world = worlds
        .get_mut(&world_id)
        .ok_or(DeterministicFault::Invariant)?;
    if inputs.iter().any(|input| {
        world
            .inventory
            .get(&input.resource_id)
            .copied()
            .unwrap_or(0)
            < input.quantity
    }) {
        return Ok(false);
    }
    let mut inventory = world.inventory.clone();
    for input in inputs {
        subtract_inventory(&mut inventory, &input.resource_id, input.quantity)?;
    }
    for output in outputs {
        add_inventory(&mut inventory, &output.resource_id, output.quantity)?;
    }
    world.inventory = inventory;
    Ok(true)
}

fn run_extractor(
    worlds: &mut BTreeMap<EntityId, WorldV1>,
    deposits: &mut BTreeMap<EntityId, DepositV1>,
    industry: &IndustryV1,
    energy_input: &QuantityV1,
    output: &QuantityV1,
) -> Result<bool, DeterministicFault> {
    let deposit_id = industry
        .linked_deposit
        .ok_or(DeterministicFault::Invariant)?;
    let deposit = deposits
        .get(&deposit_id)
        .ok_or(DeterministicFault::Invariant)?;
    if deposit.world != industry.world || deposit.resource_id != output.resource_id {
        return Err(DeterministicFault::Invariant);
    }
    let available_energy = worlds
        .get(&industry.world)
        .ok_or(DeterministicFault::Invariant)?
        .inventory
        .get(&energy_input.resource_id)
        .copied()
        .unwrap_or(0);
    if available_energy < energy_input.quantity || deposit.reserve_units == 0 {
        return Ok(false);
    }
    let produced = output.quantity.min(deposit.reserve_units);
    let mut inventory = worlds[&industry.world].inventory.clone();
    subtract_inventory(
        &mut inventory,
        &energy_input.resource_id,
        energy_input.quantity,
    )?;
    add_inventory(&mut inventory, &output.resource_id, produced)?;
    deposits
        .get_mut(&deposit_id)
        .ok_or(DeterministicFault::Invariant)?
        .reserve_units = deposit
        .reserve_units
        .checked_sub(produced)
        .ok_or(DeterministicFault::Arithmetic)?;
    worlds
        .get_mut(&industry.world)
        .ok_or(DeterministicFault::Invariant)?
        .inventory = inventory;
    Ok(true)
}

fn dispatch_routes(
    routes: &BTreeMap<EntityId, RouteV1>,
    lane_index: &BTreeMap<(EntityId, EntityId), (EntityId, u64)>,
    active_hazards: &BTreeMap<EntityId, Vec<(u64, u64)>>,
    candidate: &mut StepCandidate,
    occupied_ids: &mut BTreeSet<EntityId>,
    events: &mut Vec<TickEventV1>,
) -> Result<(), DeterministicFault> {
    for (route_id, route) in routes {
        let source = candidate
            .worlds
            .get(&route.source)
            .ok_or(DeterministicFault::Invariant)?;
        let destination = candidate
            .worlds
            .get(&route.destination)
            .ok_or(DeterministicFault::Invariant)?;
        let source_system = source.system;
        let destination_system = destination.system;
        let (lane_id, travel_ticks) = if source_system == destination_system {
            (None, 1_u64)
        } else {
            let key = ordered_pair(source_system, destination_system);
            let (lane_id, distance_units) = lane_index
                .get(&key)
                .copied()
                .ok_or(DeterministicFault::Invariant)?;
            let rounded = distance_units
                .checked_add(255)
                .ok_or(DeterministicFault::Arithmetic)?
                / 256;
            (Some(lane_id), rounded.max(1))
        };

        let mut capacity = route.batch_units;
        if let Some(lane_id) = lane_id
            && let Some(ratios) = active_hazards.get(&lane_id)
        {
            for &(numerator, denominator) in ratios {
                let scaled = u128::from(capacity)
                    .checked_mul(u128::from(numerator))
                    .ok_or(DeterministicFault::Arithmetic)?
                    / u128::from(denominator);
                capacity = u64::try_from(scaled).map_err(|_| DeterministicFault::Arithmetic)?;
            }
        }
        let available = source
            .inventory
            .get(&route.resource_id)
            .copied()
            .unwrap_or(0);
        let units = capacity.min(available);
        if units == 0 {
            continue;
        }
        if candidate.shipments.len() >= MAX_SHIPMENTS {
            return Err(DeterministicFault::ShipmentCapacity);
        }
        let shipment_id = shipment_id(*route_id, candidate.tick);
        if !occupied_ids.insert(shipment_id) {
            return Err(DeterministicFault::IdentityCollision);
        }
        let arrival_tick = WorkshopTick(
            candidate
                .tick
                .0
                .checked_add(travel_ticks.max(1))
                .ok_or(DeterministicFault::Arithmetic)?,
        );
        let source_world = candidate
            .worlds
            .get_mut(&route.source)
            .ok_or(DeterministicFault::Invariant)?;
        subtract_inventory(&mut source_world.inventory, &route.resource_id, units)?;
        candidate.shipments.insert(
            shipment_id,
            ShipmentV1 {
                route: *route_id,
                source: route.source,
                destination: route.destination,
                resource_id: route.resource_id.clone(),
                units,
                dispatched_tick: candidate.tick,
                arrival_tick,
            },
        );
        events.push(TickEventV1::RouteDispatched {
            route: *route_id,
            shipment: shipment_id,
            units,
            arrival_tick,
        });
    }
    Ok(())
}

fn ordered_pair(a: EntityId, b: EntityId) -> (EntityId, EntityId) {
    if a <= b { (a, b) } else { (b, a) }
}

fn build_lane_index(state: &WorkshopStateV1) -> BTreeMap<(EntityId, EntityId), (EntityId, u64)> {
    state
        .lanes
        .iter()
        .map(|(id, lane)| (ordered_pair(lane.a, lane.b), (*id, lane.distance_units)))
        .collect()
}

fn active_hazard_ratios(
    catalog: &ValidatedCatalogPackV1,
    state: &WorkshopStateV1,
    tick: WorkshopTick,
) -> Result<BTreeMap<EntityId, Vec<(u64, u64)>>, DeterministicFault> {
    let mut ratios = BTreeMap::<EntityId, Vec<(u64, u64)>>::new();
    for hazard in state.hazards.values() {
        let end = hazard
            .start_tick
            .0
            .checked_add(hazard.duration_ticks)
            .ok_or(DeterministicFault::Arithmetic)?;
        if hazard.cancelled || hazard.start_tick > tick || tick.0 >= end {
            continue;
        }
        let definition = catalog
            .hazard(&hazard.hazard_id)
            .ok_or(DeterministicFault::Invariant)?;
        ratios
            .entry(hazard.lane)
            .or_default()
            .push(definition.route_capacity_ratio());
    }
    Ok(ratios)
}

fn candidate_entity_ids(state: &WorkshopStateV1, candidate: &StepCandidate) -> BTreeSet<EntityId> {
    state
        .factions
        .keys()
        .chain(state.systems.keys())
        .chain(state.stars.keys())
        .chain(candidate.worlds.keys())
        .chain(state.lanes.keys())
        .chain(candidate.deposits.keys())
        .chain(state.industries.keys())
        .chain(state.routes.keys())
        .chain(candidate.shipments.keys())
        .chain(state.hazards.keys())
        .copied()
        .collect()
}

fn add_inventory(
    inventory: &mut BTreeMap<CatalogId, u64>,
    resource: &CatalogId,
    units: u64,
) -> Result<(), DeterministicFault> {
    let current = inventory.get(resource).copied().unwrap_or(0);
    inventory.insert(
        resource.clone(),
        current
            .checked_add(units)
            .ok_or(DeterministicFault::Arithmetic)?,
    );
    Ok(())
}

fn subtract_inventory(
    inventory: &mut BTreeMap<CatalogId, u64>,
    resource: &CatalogId,
    units: u64,
) -> Result<(), DeterministicFault> {
    let current = inventory.get(resource).copied().unwrap_or(0);
    inventory.insert(
        resource.clone(),
        current
            .checked_sub(units)
            .ok_or(DeterministicFault::Arithmetic)?,
    );
    Ok(())
}

fn shipment_id(route: EntityId, tick: WorkshopTick) -> EntityId {
    let mut hash = Sha256::new();
    hash.update(b"NYON-WORKSHOP-SHIPMENT-V1\0");
    hash.update(route.0);
    hash.update(tick.0.to_le_bytes());
    let bytes: [u8; 32] = hash.finalize().into();
    let mut id = [0_u8; 16];
    id.copy_from_slice(&bytes[..16]);
    EntityId(id)
}

pub fn entity_count_for_kind(state: &WorkshopStateV1, kind: EntityKind) -> usize {
    match kind {
        EntityKind::Faction => state.factions.len(),
        EntityKind::System => state.systems.len(),
        EntityKind::Star => state.stars.len(),
        EntityKind::World => state.worlds.len(),
        EntityKind::Lane => state.lanes.len(),
        EntityKind::Deposit => state.deposits.len(),
        EntityKind::Industry => state.industries.len(),
        EntityKind::Route => state.routes.len(),
        EntityKind::Shipment => state.shipments.len(),
        EntityKind::Hazard => state.hazards.len(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        ids::{EntityId, GalaxyPointV1, ObjectName},
        model::{IndustryV1, WorldV1},
        pack::decode_catalog_pack,
    };

    #[test]
    fn deterministic_fault_preserves_the_entire_pre_step_state() {
        let catalog =
            decode_catalog_pack(include_bytes!("../../../assets/workshop/core-pack-v1.json"))
                .unwrap();
        let system = EntityId([1; 16]);
        let star = EntityId([2; 16]);
        let world = EntityId([3; 16]);
        let industry = EntityId([4; 16]);
        let mut state = WorkshopStateV1::default();
        state.systems.insert(
            system,
            crate::model::SystemV1 {
                name: ObjectName::new("System").unwrap(),
                position: GalaxyPointV1::new(0, 0).unwrap(),
            },
        );
        state.stars.insert(
            star,
            crate::model::StarV1 {
                system,
                name: ObjectName::new("Star").unwrap(),
                archetype_id: CatalogId::new("yellow-dwarf").unwrap(),
            },
        );
        state.worlds.insert(
            world,
            WorldV1 {
                system,
                primary: star,
                name: ObjectName::new("World").unwrap(),
                archetype_id: CatalogId::new("rocky-world").unwrap(),
                orbit_radius_milli_au: 1,
                orbit_period_ticks: 1,
                phase_millidegrees: 0,
                inventory: BTreeMap::from([(CatalogId::new("energy").unwrap(), u64::MAX)]),
            },
        );
        state.industries.insert(
            industry,
            IndustryV1 {
                world,
                definition_id: CatalogId::new("solar-array").unwrap(),
                linked_deposit: None,
                enabled: true,
            },
        );
        let before = state.canonical_bytes();
        assert_eq!(
            step_state(&catalog, &mut state),
            Err(DeterministicFault::Arithmetic)
        );
        assert_eq!(state.canonical_bytes(), before);
    }
}
