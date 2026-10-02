//! Additive creator capabilities. The caller applies these only to an atomic
//! private projection, then validates/hashes before publishing any revision.
use super::{
    catalog::ValidatedLivingCatalogPackV2,
    command::{
        LivingCascadeDispositionV2, LivingCommandTargetV2, LivingCreatorOperationV2,
        LivingDerivedRemovalV2,
    },
    ids::{LivingEntityIdV2, LivingTickV2},
    model::*,
    receipt::LivingEventKindV2,
    rules,
    simulation::LivingCommandRejectionV2,
    wire::{LivingKeyedV2, LivingSortedVecV2},
};
use std::collections::BTreeSet;
fn missing(index: usize, reference: &'static str) -> LivingCommandRejectionV2 {
    LivingCommandRejectionV2::MissingTarget {
        operation: index,
        reference,
    }
}
fn invalid(field: &'static str) -> LivingCommandRejectionV2 {
    LivingValidationErrorV2::Range { field }.into()
}
fn edit<'a, T: LivingKeyedV2<Key = LivingEntityIdV2>>(
    rows: &'a mut LivingSortedVecV2<T>,
    id: LivingEntityIdV2,
    index: usize,
    kind: &'static str,
) -> Result<&'a mut T, LivingCommandRejectionV2> {
    rows.find_mut(&id).ok_or(missing(index, kind))
}
fn unreserved_fleet(
    state: &LivingGalaxyStateV2,
    id: LivingEntityIdV2,
    index: usize,
) -> Result<LivingFleetV2, LivingCommandRejectionV2> {
    let fleet = state
        .fleets
        .as_slice()
        .iter()
        .find(|row| row.id == id)
        .ok_or(missing(index, "fleet"))?;
    if !matches!(fleet.location, LivingFleetLocationV2::Docked { .. })
        || state
            .hull_jobs
            .as_slice()
            .iter()
            .any(|job| job.target_fleet == Some(id))
    {
        return Err(LivingCommandRejectionV2::ReferencedTarget);
    }
    Ok(fleet.clone())
}
/// Exact reviewed derived-key list for the existing deterministic stable closure.
pub fn preview_living_derived_removal_v2(
    state: &LivingGalaxyStateV2,
    target: LivingEntityIdV2,
) -> Option<Vec<LivingDerivedRemovalV2>> {
    let preview = rules::preview_living_removal_v2(state, target)?;
    let removed: BTreeSet<_> = std::iter::once(target).chain(preview.dependents).collect();
    let mut keys = Vec::new();
    for row in state.colonies.as_slice() {
        if removed.contains(&row.world) || removed.contains(&row.owner) {
            keys.push(LivingDerivedRemovalV2::Colony {
                world: row.world,
                owner: row.owner,
            });
        }
    }
    for row in state.observations.as_slice() {
        if removed.contains(&row.world) || removed.contains(&row.civilization) {
            keys.push(LivingDerivedRemovalV2::Observation {
                civilization: row.civilization,
                world: row.world,
            });
        }
    }
    for row in state.relations.as_slice() {
        if removed.contains(&row.from) || removed.contains(&row.to) {
            keys.push(LivingDerivedRemovalV2::Relation {
                from: row.from,
                to: row.to,
            });
        }
    }
    for row in state.agreements.as_slice() {
        if removed.contains(&row.participant_a) || removed.contains(&row.participant_b) {
            keys.push(LivingDerivedRemovalV2::Agreement {
                a: row.participant_a,
                b: row.participant_b,
                agreement_kind: row.kind,
            });
        }
    }
    for row in state.wars.as_slice() {
        if removed.contains(&row.participant_a) || removed.contains(&row.participant_b) {
            keys.push(LivingDerivedRemovalV2::War {
                a: row.participant_a,
                b: row.participant_b,
            });
        }
    }
    for row in state.occupations.as_slice() {
        if removed.contains(&row.world) || removed.contains(&row.claimant) {
            keys.push(LivingDerivedRemovalV2::Occupation {
                world: row.world,
                claimant: row.claimant,
            });
        }
    }
    for row in state.settlement_claims.as_slice() {
        if removed.contains(&row.world) || removed.contains(&row.civilization) {
            keys.push(LivingDerivedRemovalV2::SettlementClaim {
                world: row.world,
                civilization: row.civilization,
            });
        }
    }
    for row in state.fleets.as_slice() {
        if removed.contains(&row.id) && row.order.is_some() {
            keys.push(LivingDerivedRemovalV2::Order { fleet: row.id });
        }
    }
    keys.sort();
    Some(keys)
}
fn remove_reviewed(
    state: &mut LivingGalaxyStateV2,
    target: LivingEntityIdV2,
    cascade: &LivingCascadeDispositionV2,
    index: usize,
    resolve: &dyn Fn(&LivingCommandTargetV2) -> LivingEntityIdV2,
    effects: &mut Vec<LivingEventKindV2>,
) -> Result<(), LivingCommandRejectionV2> {
    if let LivingCascadeDispositionV2::RemoveReviewed {
        dependents,
        derived,
    } = cascade
    {
        let preview = rules::preview_living_removal_v2(state, target)
            .ok_or(missing(index, "removal entity"))?;
        let ids: Vec<_> = dependents.iter().map(resolve).collect();
        if ids != preview.dependents
            || Some(derived) != preview_living_derived_removal_v2(state, target).as_ref()
        {
            return Err(LivingCommandRejectionV2::CascadeMismatch);
        }
        let legacy = LivingCascadeDispositionV2::RemoveListed {
            dependents: dependents.clone(),
        };
        rules::remove_creator_entity(
            state,
            target,
            &legacy,
            index,
            &|value| resolve(value),
            effects,
        )
    } else {
        rules::remove_creator_entity(
            state,
            target,
            cascade,
            index,
            &|value| resolve(value),
            effects,
        )
    }
}
fn permitted_route(
    state: &LivingGalaxyStateV2,
    route: &LivingFreightRouteV2,
    tick: LivingTickV2,
) -> bool {
    if state
        .worlds
        .as_slice()
        .iter()
        .find(|row| row.id == route.source_world)
        .and_then(|w| w.owner)
        != Some(route.source_owner)
        || state
            .worlds
            .as_slice()
            .iter()
            .find(|row| row.id == route.destination_world)
            .and_then(|w| w.owner)
            != Some(route.receiver_owner)
    {
        return false;
    }
    let (a, b) = if route.source_owner < route.receiver_owner {
        (route.source_owner, route.receiver_owner)
    } else {
        (route.receiver_owner, route.source_owner)
    };
    if state.wars.as_slice().iter().any(|war| {
        war.participant_a == a
            && war.participant_b == b
            && war.start_tick <= tick
            && tick < war.end_tick
    }) {
        return false;
    }
    match route.kind {
        LivingRouteKindV2::Internal => a == b,
        LivingRouteKindV2::Aid => true,
        LivingRouteKindV2::Trade => state.agreements.as_slice().iter().any(|agreement| {
            agreement.participant_a == a
                && agreement.participant_b == b
                && agreement.kind == LivingAgreementKindV2::Trade
                && agreement.start_tick <= tick
                && tick < agreement.end_tick
        }),
    }
}
/// Returns false only for legacy operations. Errors leave the outer atomic
/// transaction uncommitted; this helper is not a public authority bypass.
#[allow(clippy::too_many_arguments)] // Mirrors the authoritative operation transaction context.
pub(crate) fn apply_creator_addition(
    state: &mut LivingGalaxyStateV2,
    tick: LivingTickV2,
    index: usize,
    operation: &LivingCreatorOperationV2,
    resolve: &dyn Fn(&LivingCommandTargetV2) -> LivingEntityIdV2,
    effects: &mut Vec<LivingEventKindV2>,
    catalog: &ValidatedLivingCatalogPackV2,
) -> Result<bool, LivingCommandRejectionV2> {
    use LivingCreatorOperationV2::*;
    let id = |target: &LivingCommandTargetV2| resolve(target);
    match operation {
        EditSystem { target, name } => {
            edit(&mut state.systems, id(target), index, "system")?.name = name.clone()
        }
        EditStar {
            target,
            system,
            archetype,
            name,
        } => {
            let star = edit(&mut state.stars, id(target), index, "star")?;
            star.system = id(system);
            star.archetype = archetype.clone();
            star.name = name.clone();
        }
        EditWorld {
            target,
            system,
            archetype,
            name,
        } => {
            let target = id(target);
            let existing = state
                .worlds
                .as_slice()
                .iter()
                .find(|world| world.id == target)
                .ok_or(missing(index, "world"))?;
            if existing.system != id(system)
                && (state.shipments.as_slice().iter().any(|shipment| {
                    shipment.source_world == target || shipment.destination_world == target
                }) || state
                    .fleets
                    .as_slice()
                    .iter()
                    .any(|fleet| match fleet.location {
                        LivingFleetLocationV2::Travelling {
                            origin_world,
                            destination_world,
                            ..
                        } => origin_world == target || destination_world == target,
                        LivingFleetLocationV2::Docked { world } => {
                            world == target && fleet.order.is_some()
                        }
                    }))
            {
                return Err(LivingCommandRejectionV2::ReferencedTarget);
            }
            let world = edit(&mut state.worlds, target, index, "world")?;
            world.system = id(system);
            world.archetype = archetype.clone();
            world.name = name.clone();
        }
        EditLane {
            target,
            system_a,
            system_b,
            distance_units,
        } => {
            let lane = id(target);
            let a = id(system_a);
            let b = id(system_b);
            if a == b || *distance_units == 0 {
                return Err(invalid("lane endpoints/distance"));
            }
            let existing = state
                .lanes
                .as_slice()
                .iter()
                .find(|row| row.id == lane)
                .ok_or(missing(index, "lane"))?;
            let changed = existing.system_a != a.min(b)
                || existing.system_b != a.max(b)
                || existing.distance_units != *distance_units;
            if changed
                && state.fleets.as_slice().iter().any(|fleet| {
                    fleet
                        .order
                        .as_ref()
                        .is_some_and(|order| order.lane_path.contains(&lane))
                })
            {
                return Err(LivingCommandRejectionV2::ReferencedTarget);
            }
            let row = edit(&mut state.lanes, lane, index, "lane")?;
            row.system_a = a.min(b);
            row.system_b = a.max(b);
            row.distance_units = *distance_units;
        }
        EditDeposit {
            target,
            resource,
            remaining_units,
        } => {
            let deposit = edit(&mut state.deposits, id(target), index, "deposit")?;
            deposit.resource = *resource;
            deposit.remaining_units = *remaining_units;
        }
        EditCivilization {
            target,
            name,
            policy,
        } => {
            let civ = edit(&mut state.civilizations, id(target), index, "civilization")?;
            civ.name = name.clone();
            civ.policy = policy.clone();
        }
        SetFacilityEnabled { target, enabled } => {
            edit(&mut state.facilities, id(target), index, "facility")?.disabled = !*enabled
        }
        ScrapFacility { target, cascade } => {
            let target = id(target);
            if state
                .facilities
                .as_slice()
                .iter()
                .find(|row| row.id == target)
                .is_none()
            {
                return Err(missing(index, "facility"));
            }
            remove_reviewed(state, target, cascade, index, resolve, effects)?;
        }
        CancelConstructionJob { target } => {
            let target = id(target);
            let job = state
                .construction_jobs
                .as_slice()
                .iter()
                .find(|row| row.id == target)
                .ok_or(missing(index, "construction job"))?
                .clone();
            rules::refund_creator_job(state, job.world, job.paid_alloy, effects)?;
            rules::remove_key(&mut state.construction_jobs, &target)?;
            effects.push(LivingEventKindV2::ConstructionCancelled);
        }
        CancelHullJob { target } => {
            let target = id(target);
            let job = state
                .hull_jobs
                .as_slice()
                .iter()
                .find(|row| row.id == target)
                .ok_or(missing(index, "hull job"))?
                .clone();
            rules::refund_creator_job(state, job.world, job.paid_alloy, effects)?;
            rules::remove_key(&mut state.hull_jobs, &target)?;
            effects.push(LivingEventKindV2::ConstructionCancelled);
        }
        EditFleet {
            target,
            owner,
            world,
        } => {
            let target = id(target);
            let mut fleet = unreserved_fleet(state, target, index)?;
            fleet.owner = id(owner);
            fleet.location = LivingFleetLocationV2::Docked { world: id(world) };
            fleet.order = None;
            rules::put(&mut state.fleets, fleet)?;
        }
        EditHull {
            target,
            definition,
            hit_points,
            return_credit,
        } => {
            let target = id(target);
            let def = catalog
                .hull(definition.as_str())
                .ok_or(invalid("hull definition"))?;
            if *hit_points == 0 || *hit_points > def.hit_points {
                return Err(invalid("hull hit points"));
            }
            let fleet_id = state
                .fleets
                .as_slice()
                .iter()
                .find(|fleet| {
                    fleet
                        .hulls
                        .as_slice()
                        .iter()
                        .find(|row| row.id == target)
                        .is_some()
                })
                .map(|fleet| fleet.id)
                .ok_or(missing(index, "hull"))?;
            unreserved_fleet(state, fleet_id, index)?;
            let fleet = state
                .fleets
                .find_mut(&fleet_id)
                .ok_or(missing(index, "fleet"))?;
            let hull = fleet
                .hulls
                .find_mut(&target)
                .ok_or(missing(index, "hull"))?;
            hull.definition = definition.clone();
            hull.hit_points = *hit_points;
            hull.return_credit = *return_credit;
            fleet.order = None;
        }
        SplitFleet {
            local,
            source,
            hulls,
        } => {
            let source = id(source);
            let mut fleet = unreserved_fleet(state, source, index)?;
            let selected: Vec<_> = hulls.iter().map(resolve).collect();
            if selected.is_empty()
                || selected.len() >= fleet.hulls.as_slice().len()
                || selected.windows(2).any(|w| w[0] >= w[1])
            {
                return Err(invalid("split hull membership"));
            }
            if state.fleets.as_slice().len()
                + state
                    .hull_jobs
                    .as_slice()
                    .iter()
                    .filter(|job| job.target_fleet.is_none())
                    .count()
                >= LIVING_MAX_FLEETS_V2
            {
                return Err(LivingValidationErrorV2::Capacity {
                    collection: "fleets",
                    limit: LIVING_MAX_FLEETS_V2,
                    found: LIVING_MAX_FLEETS_V2 + 1,
                }
                .into());
            }
            let mut moved = Vec::new();
            for hull in &selected {
                moved.push(
                    fleet
                        .hulls
                        .as_slice()
                        .iter()
                        .find(|row| row.id == *hull)
                        .ok_or(missing(index, "split hull"))?
                        .clone(),
                );
            }
            let kept = fleet
                .hulls
                .as_slice()
                .iter()
                .filter(|hull| selected.binary_search(&hull.id).is_err())
                .cloned()
                .collect();
            fleet.hulls = LivingSortedVecV2::new(kept).map_err(LivingValidationErrorV2::from)?;
            fleet.order = None;
            let new = LivingFleetV2 {
                id: id(&LivingCommandTargetV2::Local { local: *local }),
                owner: fleet.owner,
                location: fleet.location,
                order: None,
                hulls: LivingSortedVecV2::new(moved).map_err(LivingValidationErrorV2::from)?,
            };
            if state
                .fleets
                .as_slice()
                .iter()
                .find(|row| row.id == new.id)
                .is_some()
            {
                return Err(invalid("split fleet identity"));
            }
            rules::put(&mut state.fleets, fleet)?;
            rules::put(&mut state.fleets, new)?;
        }
        MergeFleets { target, source } => {
            let target = id(target);
            let source = id(source);
            if target == source {
                return Err(invalid("merge fleet identity"));
            }
            let mut keep = unreserved_fleet(state, target, index)?;
            let merged = unreserved_fleet(state, source, index)?;
            if keep.owner != merged.owner || keep.location != merged.location {
                return Err(invalid("merge fleet colocation"));
            }
            let found = keep.hulls.as_slice().len() + merged.hulls.as_slice().len();
            if found > LIVING_MAX_HULLS_PER_FLEET_V2 {
                return Err(LivingValidationErrorV2::Capacity {
                    collection: "fleet hulls",
                    limit: LIVING_MAX_HULLS_PER_FLEET_V2,
                    found,
                }
                .into());
            }
            let mut hulls = keep.hulls.into_vec();
            hulls.extend(merged.hulls.into_vec());
            hulls.sort_by_key(|hull| hull.id);
            keep.hulls = LivingSortedVecV2::new(hulls).map_err(LivingValidationErrorV2::from)?;
            keep.order = None;
            rules::remove_key(&mut state.fleets, &source)?;
            rules::put(&mut state.fleets, keep)?;
        }
        EditRoute {
            target,
            kind,
            source_world,
            destination_world,
            resource,
            batch_size,
            cadence_ticks,
            source_reserve,
            source_owner,
            receiver_owner,
        } => {
            let mut route = state
                .routes
                .as_slice()
                .iter()
                .find(|row| row.id == id(target))
                .ok_or(missing(index, "route"))?
                .clone();
            if *batch_size == 0
                || *batch_size > LIVING_ROUTE_BATCH_LIMIT_V2
                || *cadence_ticks != LIVING_ROUTE_CADENCE_TICKS_V2
                || *source_reserve < LIVING_ROUTE_RESERVE_FLOOR_V2
            {
                return Err(invalid("route bounds"));
            }
            route.kind = *kind;
            route.source_world = id(source_world);
            route.destination_world = id(destination_world);
            route.resource = *resource;
            route.batch_size = *batch_size;
            route.cadence_ticks = *cadence_ticks;
            route.source_reserve = *source_reserve;
            route.source_owner = id(source_owner);
            route.receiver_owner = id(receiver_owner);
            if !permitted_route(state, &route, tick) {
                return Err(invalid("route permission"));
            }
            route.suspended = false;
            if route.next_due <= tick {
                route.next_due = LivingTickV2(
                    tick.0
                        .checked_add(u64::from(route.cadence_ticks))
                        .ok_or(LivingCommandRejectionV2::Exhausted)?,
                );
            }
            rules::put(&mut state.routes, route)?;
        }
        SetRouteSuspended { target, suspended } => {
            let target = id(target);
            let mut route = state
                .routes
                .as_slice()
                .iter()
                .find(|row| row.id == target)
                .ok_or(missing(index, "route"))?
                .clone();
            if !*suspended && !permitted_route(state, &route, tick) {
                return Err(invalid("route permission"));
            }
            route.suspended = *suspended;
            if !*suspended && route.next_due <= tick {
                route.next_due = LivingTickV2(
                    tick.0
                        .checked_add(u64::from(route.cadence_ticks))
                        .ok_or(LivingCommandRejectionV2::Exhausted)?,
                );
            }
            rules::put(&mut state.routes, route)?;
        }
        EditHazard {
            target,
            lane,
            definition,
            start_tick,
            end_tick,
        } => {
            if *start_tick < tick || end_tick <= start_tick {
                return Err(invalid("hazard interval"));
            }
            if catalog.hazard(definition.as_str()).is_none() {
                return Err(invalid("hazard definition"));
            }
            let hazard = edit(&mut state.hazards, id(target), index, "hazard")?;
            hazard.lane = id(lane);
            hazard.definition = definition.clone();
            hazard.start_tick = *start_tick;
            hazard.end_tick = *end_tick;
        }
        CancelHazard { target } => {
            let target = id(target);
            if state
                .hazards
                .as_slice()
                .iter()
                .find(|row| row.id == target)
                .is_none()
            {
                return Err(missing(index, "hazard"));
            }
            rules::remove_key(&mut state.hazards, &target)?;
        }
        RemoveEntity {
            target,
            cascade: cascade @ LivingCascadeDispositionV2::RemoveReviewed { .. },
        } => remove_reviewed(state, id(target), cascade, index, resolve, effects)?,
        _ => return Ok(false),
    }
    Ok(true)
}
