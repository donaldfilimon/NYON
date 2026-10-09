//! Living Galaxy V2 phase bodies and the curated sandbox genesis.
//!
//! Sections 2 through 7 of the rules spec are binding. Phase order stays in
//! the authority; this module only fills the bodies those phases call.

use super::LIVING_RULES_VERSION;
use super::catalog::{
    LIVING_RESOURCE_STORAGE_LIMIT_V2, LivingIndustryEffectV2, LivingNameV2, LivingSlugV2,
    ValidatedLivingCatalogPackV2, living_core_pack_v2,
};
use super::command::{
    LivingCascadeDispositionV2, LivingCommandTargetV2, LivingCreatorOperationV2,
    LivingCreatorRemovalPreviewV2,
};
use super::decision::{
    LivingDecisionOutcomeV2, LivingDecisionReasonV2, LivingDecisionScoreV2, LivingDecisionTierV2,
    LivingIntentActionV2, LivingIntentDecisionV2, living_decision_snapshot_digest_v2,
};
use super::genesis::{
    LIVING_GENESIS_FORMAT_VERSION_V2, LIVING_GENESIS_KIND_V2, LIVING_HUB_ENERGY_PERIOD_TICKS_V2,
    LIVING_HUB_FALLBACK_PERIOD_TICKS_V2, LIVING_HUB_ORE_PERIOD_TICKS_V2, LivingGenesisGeneratorV2,
    LivingGenesisManifestV2, validate_living_genesis_manifest_v2,
};
use super::ids::{
    LivingAutonomousEntityInputsV2, LivingEntityIdV2, LivingEntityKindV2, LivingPhaseV2,
    LivingTickV2, autonomous_entity_id_v2, claim_rank_v2,
};
use super::model::{
    LIVING_OCCUPATION_TRANSFER_TICKS_V2, LivingAgreementKindV2, LivingAgreementV2,
    LivingBlockedReasonV2, LivingCivilizationStatusV2, LivingCivilizationV2, LivingColonyV2,
    LivingConstructionJobV2, LivingDepositV2, LivingFacilityStatusV2, LivingFacilityV2,
    LivingFleetLocationV2, LivingFleetOrderKindV2, LivingFleetOrderV2, LivingFleetV2,
    LivingFreightRouteV2, LivingGalaxyStateV2, LivingHazardV2, LivingHubV2, LivingHullJobV2,
    LivingHullV2, LivingInventoryV2, LivingOccupationV2, LivingPolicyV2, LivingRelationReasonsV2,
    LivingRelationV2, LivingResourceV2, LivingRouteKindV2, LivingSettlementClaimV2,
    LivingShipmentDispositionV2, LivingShipmentV2, LivingStarV2, LivingSystemV2,
    LivingValidationErrorV2, LivingWarV2, LivingWorldV2,
};
#[cfg(test)]
use super::model::{
    LivingLaneV2, LivingObservationV2, LivingObservedFacilityV2, LivingObservedHullV2,
};
use super::receipt::{LivingEventKindV2, LivingEventProvenanceV2};
use super::simulation::{
    LivingCommandRejectionV2, LivingDeterministicFaultV2, LivingGalaxyAuthorityV2,
    LivingStepContextV2,
};
use super::wire::{LivingKeyedV2, LivingSortedVecV2};

#[cfg(test)]
const HUB_ENERGY_OUTPUT: u32 = 2;
#[cfg(test)]
const HUB_ORE_OUTPUT: u32 = 1;
#[cfg(test)]
const HUB_FALLBACK_ENERGY: u32 = 4;
#[cfg(test)]
const HUB_FALLBACK_ORE: u32 = 4;
#[cfg(test)]
const HUB_FALLBACK_ALLOY: u32 = 1;
#[cfg(test)]
const HUB_FALLBACK_CEILING: u32 = 120;
const CLAIM_WINDOW_TICKS: u64 = 50;
const WAR_DURATION_TICKS: u64 = 1_200;
const AGREEMENT_DURATION_TICKS: u64 = 1_200;
const REPAIR_PERIOD_TICKS: u64 = 10;
const LOCAL_TRAVEL_TICKS: u64 = 10;
const LAUNCH_ENERGY_PER_HULL: u32 = 2;
const SETTLEMENT_ENERGY: u32 = 20;
const SETTLEMENT_ORE: u32 = 20;
const OBSERVATION_FRESH_TICKS: u64 = 100;
const DECAY_PERIOD_TICKS: u64 = 600;
const COMBAT_PERIOD_TICKS: u64 = 10;
const ESCORT_STRENGTH: i64 = 10;
const BATTERY_STRENGTH: i64 = 20;

pub(crate) fn creator_operation_applied(_operation: &LivingCreatorOperationV2) -> bool {
    true
}

pub(crate) fn apply_creator_operation(
    state: &mut LivingGalaxyStateV2,
    tick: LivingTickV2,
    index: usize,
    operation: &LivingCreatorOperationV2,
    resolve: &dyn Fn(&LivingCommandTargetV2) -> LivingEntityIdV2,
    effects: &mut Vec<LivingEventKindV2>,
    catalog: &ValidatedLivingCatalogPackV2,
) -> Result<(), LivingCommandRejectionV2> {
    if super::creator_additions::apply_creator_addition(
        state, tick, index, operation, resolve, effects, catalog,
    )? {
        return Ok(());
    }
    let id_of = |target: &LivingCommandTargetV2| resolve(target);
    match operation {
        LivingCreatorOperationV2::CreateFacility {
            local,
            world,
            definition,
            status,
            paid_alloy,
            hit_points,
            next_due,
        } => {
            put(
                &mut state.facilities,
                LivingFacilityV2 {
                    id: resolve(&LivingCommandTargetV2::Local { local: *local }),
                    world: id_of(world),
                    definition: definition.clone(),
                    status: *status,
                    paid_alloy: *paid_alloy,
                    hit_points: *hit_points,
                    next_due: *next_due,
                    next_repair: None,
                    blocked_reasons: LivingSortedVecV2::default(),
                    disabled: false,
                },
            )?;
        }
        LivingCreatorOperationV2::CreateConstructionJob {
            local,
            world,
            owner,
            definition,
            accepted_tick,
            completion_tick,
            paid_alloy,
        } => {
            put(
                &mut state.construction_jobs,
                LivingConstructionJobV2 {
                    id: resolve(&LivingCommandTargetV2::Local { local: *local }),
                    world: id_of(world),
                    owner: id_of(owner),
                    definition: definition.clone(),
                    accepted_tick: *accepted_tick,
                    completion_tick: *completion_tick,
                    paid_alloy: *paid_alloy,
                },
            )?;
        }
        LivingCreatorOperationV2::CreateHullJob {
            local,
            world,
            owner,
            definition,
            accepted_tick,
            completion_tick,
            paid_alloy,
            target_fleet,
        } => {
            put(
                &mut state.hull_jobs,
                LivingHullJobV2 {
                    id: resolve(&LivingCommandTargetV2::Local { local: *local }),
                    world: id_of(world),
                    owner: id_of(owner),
                    definition: definition.clone(),
                    accepted_tick: *accepted_tick,
                    completion_tick: *completion_tick,
                    paid_alloy: *paid_alloy,
                    target_fleet: target_fleet.as_ref().map(id_of),
                },
            )?;
        }
        LivingCreatorOperationV2::CreateFleet {
            local,
            owner,
            world,
        } => {
            put(
                &mut state.fleets,
                LivingFleetV2 {
                    id: resolve(&LivingCommandTargetV2::Local { local: *local }),
                    owner: id_of(owner),
                    location: LivingFleetLocationV2::Docked {
                        world: id_of(world),
                    },
                    order: None,
                    hulls: LivingSortedVecV2::default(),
                },
            )?;
        }
        LivingCreatorOperationV2::CreateHull {
            local,
            fleet,
            definition,
            hit_points,
            return_credit,
        } => {
            let fleet_id = id_of(fleet);
            let fleet = state.fleets.find_mut(&fleet_id).ok_or(
                LivingCommandRejectionV2::MissingTarget {
                    operation: index,
                    reference: "fleet",
                },
            )?;
            put(
                &mut fleet.hulls,
                LivingHullV2 {
                    id: resolve(&LivingCommandTargetV2::Local { local: *local }),
                    definition: definition.clone(),
                    hit_points: *hit_points,
                    return_credit: *return_credit,
                    next_repair: None,
                },
            )?;
        }
        LivingCreatorOperationV2::CreateRoute {
            local,
            kind,
            source_world,
            destination_world,
            resource,
            batch_size,
            cadence_ticks,
            source_reserve,
            source_owner,
            receiver_owner,
            next_due,
        } => {
            put(
                &mut state.routes,
                LivingFreightRouteV2 {
                    id: resolve(&LivingCommandTargetV2::Local { local: *local }),
                    kind: *kind,
                    source_world: id_of(source_world),
                    destination_world: id_of(destination_world),
                    resource: *resource,
                    batch_size: *batch_size,
                    cadence_ticks: *cadence_ticks,
                    source_reserve: *source_reserve,
                    source_owner: id_of(source_owner),
                    receiver_owner: id_of(receiver_owner),
                    next_due: *next_due,
                    suspended: false,
                },
            )?;
        }
        LivingCreatorOperationV2::CreateShipment {
            local,
            dispatch_owner,
            intended_receiver,
            source_world,
            destination_world,
            resource,
            units,
            departure_tick,
            arrival_tick,
            disposition,
        } => {
            put(
                &mut state.shipments,
                LivingShipmentV2 {
                    id: resolve(&LivingCommandTargetV2::Local { local: *local }),
                    dispatch_owner: id_of(dispatch_owner),
                    intended_receiver: id_of(intended_receiver),
                    source_world: id_of(source_world),
                    destination_world: id_of(destination_world),
                    resource: *resource,
                    units: *units,
                    departure_tick: *departure_tick,
                    arrival_tick: *arrival_tick,
                    disposition: *disposition,
                    dispatch_kind: None,
                    returning_to_source: false,
                },
            )?;
        }
        LivingCreatorOperationV2::CreateHazard {
            local,
            definition,
            lane,
            start_tick,
            end_tick,
        } => {
            put(
                &mut state.hazards,
                LivingHazardV2 {
                    id: resolve(&LivingCommandTargetV2::Local { local: *local }),
                    definition: definition.clone(),
                    lane: id_of(lane),
                    start_tick: *start_tick,
                    end_tick: *end_tick,
                },
            )?;
        }
        LivingCreatorOperationV2::CreateColony { world, owner } => {
            let world_id = id_of(world);
            let owner_id = id_of(owner);
            if let Some(world) = state.worlds.find_mut(&world_id) {
                world.owner = Some(owner_id);
            } else {
                return Err(LivingCommandRejectionV2::MissingTarget {
                    operation: index,
                    reference: "world",
                });
            }
            put(
                &mut state.colonies,
                LivingColonyV2 {
                    world: world_id,
                    owner: owner_id,
                    hub: LivingHubV2 {
                        energy_next_due: add_ticks(tick, LIVING_HUB_ENERGY_PERIOD_TICKS_V2)
                            .map_err(|_| LivingCommandRejectionV2::Exhausted)?,
                        ore_next_due: add_ticks(tick, LIVING_HUB_ORE_PERIOD_TICKS_V2)
                            .map_err(|_| LivingCommandRejectionV2::Exhausted)?,
                        fallback_next_due: add_ticks(tick, LIVING_HUB_FALLBACK_PERIOD_TICKS_V2)
                            .map_err(|_| LivingCommandRejectionV2::Exhausted)?,
                        blocked_reasons: LivingSortedVecV2::default(),
                    },
                },
            )?;
            refresh_creator_statuses(state);
        }
        LivingCreatorOperationV2::SetWorldOwner { world, owner } => {
            let world_id = id_of(world);
            let owner_id = owner.as_ref().map(id_of);
            if state
                .construction_jobs
                .as_slice()
                .iter()
                .any(|job| job.world == world_id && Some(job.owner) != owner_id)
                || state
                    .hull_jobs
                    .as_slice()
                    .iter()
                    .any(|job| job.world == world_id && Some(job.owner) != owner_id)
            {
                return Err(LivingCommandRejectionV2::Projection(
                    LivingValidationErrorV2::Range {
                        field: "foreign_jobs_on_world",
                    },
                ));
            }
            let Some(world) = state.worlds.find_mut(&world_id) else {
                return Err(LivingCommandRejectionV2::MissingTarget {
                    operation: index,
                    reference: "world",
                });
            };
            world.owner = owner_id;
            if let Some(owner_id) = owner_id {
                if let Some(colony) = state.colonies.find_mut(&world_id) {
                    colony.owner = owner_id;
                }
            } else {
                remove_key(&mut state.colonies, &world_id)?;
            }
            refresh_creator_statuses(state);
        }
        LivingCreatorOperationV2::SetRelationBase {
            from,
            to,
            base_disposition,
        } => {
            let from_id = id_of(from);
            let to_id = id_of(to);
            let reasons = state
                .relations
                .as_slice()
                .iter()
                .find(|relation| relation.from == from_id && relation.to == to_id)
                .map(|relation| relation.reasons)
                .unwrap_or_default();
            put(
                &mut state.relations,
                LivingRelationV2 {
                    from: from_id,
                    to: to_id,
                    base_disposition: *base_disposition,
                    reasons,
                },
            )?;
        }
        LivingCreatorOperationV2::SetAgreement {
            kind,
            participant_a,
            participant_b,
            start_tick,
            end_tick,
        } => {
            let (participant_a, participant_b) =
                ordered_pair(id_of(participant_a), id_of(participant_b));
            put(
                &mut state.agreements,
                LivingAgreementV2 {
                    kind: *kind,
                    participant_a,
                    participant_b,
                    start_tick: *start_tick,
                    end_tick: *end_tick,
                },
            )?;
        }
        LivingCreatorOperationV2::RemoveAgreement {
            kind,
            participant_a,
            participant_b,
        } => {
            let (participant_a, participant_b) =
                ordered_pair(id_of(participant_a), id_of(participant_b));
            if !state
                .civilizations
                .as_slice()
                .iter()
                .any(|row| row.id == participant_a)
                || !state
                    .civilizations
                    .as_slice()
                    .iter()
                    .any(|row| row.id == participant_b)
            {
                return Err(LivingCommandRejectionV2::MissingTarget {
                    operation: index,
                    reference: "civilization",
                });
            }
            let key = (participant_a, participant_b, kind.as_wire_str());
            if !state
                .agreements
                .as_slice()
                .iter()
                .any(|row| row.living_key() == key)
            {
                return Err(LivingCommandRejectionV2::MissingTarget {
                    operation: index,
                    reference: "agreement",
                });
            }
            remove_key(&mut state.agreements, &key)?;
        }
        LivingCreatorOperationV2::ForceWar {
            participant_a,
            participant_b,
            declarer,
            start_tick,
            end_tick,
        } => {
            let (participant_a, participant_b) =
                ordered_pair(id_of(participant_a), id_of(participant_b));
            remove_pair_agreements(state, participant_a, participant_b)?;
            put(
                &mut state.wars,
                LivingWarV2 {
                    participant_a,
                    participant_b,
                    declarer: id_of(declarer),
                    start_tick: *start_tick,
                    end_tick: *end_tick,
                },
            )?;
        }
        LivingCreatorOperationV2::ForcePeace {
            participant_a,
            participant_b,
        } => {
            let (participant_a, participant_b) =
                ordered_pair(id_of(participant_a), id_of(participant_b));
            if !state
                .civilizations
                .as_slice()
                .iter()
                .any(|row| row.id == participant_a)
                || !state
                    .civilizations
                    .as_slice()
                    .iter()
                    .any(|row| row.id == participant_b)
            {
                return Err(LivingCommandRejectionV2::MissingTarget {
                    operation: index,
                    reference: "civilization",
                });
            }
            remove_key(&mut state.wars, &(participant_a, participant_b))?;
            let worlds: Vec<_> = state
                .occupations
                .as_slice()
                .iter()
                .filter(|occupation| {
                    let owner = world_owner(state, occupation.world);
                    (owner == Some(participant_a) && occupation.claimant == participant_b)
                        || (owner == Some(participant_b) && occupation.claimant == participant_a)
                })
                .map(|occupation| occupation.world)
                .collect();
            for world in worlds {
                remove_key(&mut state.occupations, &world)?;
            }
        }
        LivingCreatorOperationV2::SetFleetOrder {
            fleet,
            kind,
            target_world,
            not_before_tick,
            lane_path,
        } => {
            let fleet_id = id_of(fleet);
            if state
                .hull_jobs
                .as_slice()
                .iter()
                .any(|job| job.target_fleet == Some(fleet_id))
            {
                return Err(LivingCommandRejectionV2::ReferencedTarget);
            }
            let Some(fleet) = state.fleets.find_mut(&fleet_id) else {
                return Err(LivingCommandRejectionV2::MissingTarget {
                    operation: index,
                    reference: "fleet",
                });
            };
            fleet.order = kind.map(|kind| LivingFleetOrderV2 {
                kind,
                target_world: target_world.as_ref().map(id_of),
                not_before_tick: *not_before_tick,
                lane_path: lane_path.iter().map(id_of).collect(),
            });
        }
        LivingCreatorOperationV2::SetShipmentDisposition {
            shipment,
            disposition,
        } => {
            let shipment_id = id_of(shipment);
            let Some(shipment) = state.shipments.find_mut(&shipment_id) else {
                return Err(LivingCommandRejectionV2::MissingTarget {
                    operation: index,
                    reference: "shipment",
                });
            };
            shipment.returning_to_source = *disposition
                == LivingShipmentDispositionV2::WaitingForStorage
                && (shipment.disposition == LivingShipmentDispositionV2::Returning
                    || shipment.returning_to_source);
            shipment.disposition = *disposition;
        }
        LivingCreatorOperationV2::RemoveColony { world } => {
            let world = id_of(world);
            if !state
                .colonies
                .as_slice()
                .iter()
                .any(|colony| colony.world == world)
            {
                return Err(LivingCommandRejectionV2::MissingTarget {
                    operation: index,
                    reference: "colony",
                });
            }
            if state
                .construction_jobs
                .as_slice()
                .iter()
                .any(|job| job.world == world)
                || state
                    .hull_jobs
                    .as_slice()
                    .iter()
                    .any(|job| job.world == world)
            {
                return Err(LivingCommandRejectionV2::ReferencedTarget);
            }
            remove_key(&mut state.colonies, &world)?;
            refresh_creator_statuses(state);
        }
        LivingCreatorOperationV2::RemoveEntity { target, cascade } => {
            remove_creator_entity(state, id_of(target), cascade, index, &id_of, effects)?;
        }
        LivingCreatorOperationV2::CreateSystem { .. }
        | LivingCreatorOperationV2::CreateStar { .. }
        | LivingCreatorOperationV2::CreateWorld { .. }
        | LivingCreatorOperationV2::CreateLane { .. }
        | LivingCreatorOperationV2::CreateCivilization { .. }
        | LivingCreatorOperationV2::CreateDeposit { .. }
        | LivingCreatorOperationV2::SetWorldInventory { .. }
        | LivingCreatorOperationV2::SetDepositRemaining { .. }
        | LivingCreatorOperationV2::SetCivilizationPolicy { .. } => {
            return Err(LivingCommandRejectionV2::UnsupportedOperation { operation: index });
        }
        _ => return Err(LivingCommandRejectionV2::UnsupportedOperation { operation: index }),
    }
    Ok(())
}

/// Inspect exact stable-entity closure without changing authority. A command
/// must name the same set against its accepted projected state.
pub fn preview_living_removal_v2(
    state: &LivingGalaxyStateV2,
    target: LivingEntityIdV2,
) -> Option<LivingCreatorRemovalPreviewV2> {
    let rows = entity_references(state);
    if !rows.iter().any(|(id, _)| *id == target) {
        return None;
    }
    let mut removed = std::collections::BTreeSet::from([target]);
    loop {
        let before = removed.len();
        for (id, references) in &rows {
            if references
                .iter()
                .any(|reference| removed.contains(reference))
            {
                removed.insert(*id);
            }
        }
        if removed.len() == before {
            break;
        }
    }
    let derived_records = derived_removal_count(state, &removed);
    removed.remove(&target);
    Some(LivingCreatorRemovalPreviewV2 {
        target,
        dependents: removed.into_iter().collect(),
        derived_records,
    })
}
fn entity_references(
    state: &LivingGalaxyStateV2,
) -> Vec<(LivingEntityIdV2, Vec<LivingEntityIdV2>)> {
    let mut rows = Vec::new();
    rows.extend(state.systems.as_slice().iter().map(|row| (row.id, vec![])));
    rows.extend(
        state
            .stars
            .as_slice()
            .iter()
            .map(|row| (row.id, vec![row.system])),
    );
    rows.extend(state.worlds.as_slice().iter().map(|row| {
        let mut refs = vec![row.system];
        refs.extend(row.owner);
        (row.id, refs)
    }));
    rows.extend(
        state
            .lanes
            .as_slice()
            .iter()
            .map(|row| (row.id, vec![row.system_a, row.system_b])),
    );
    rows.extend(
        state
            .civilizations
            .as_slice()
            .iter()
            .map(|row| (row.id, vec![])),
    );
    rows.extend(
        state
            .deposits
            .as_slice()
            .iter()
            .map(|row| (row.id, vec![row.world])),
    );
    rows.extend(
        state
            .facilities
            .as_slice()
            .iter()
            .map(|row| (row.id, vec![row.world])),
    );
    rows.extend(
        state
            .construction_jobs
            .as_slice()
            .iter()
            .map(|row| (row.id, vec![row.world, row.owner])),
    );
    rows.extend(state.hull_jobs.as_slice().iter().map(|row| {
        let mut refs = vec![row.world, row.owner];
        refs.extend(row.target_fleet);
        (row.id, refs)
    }));
    for fleet in state.fleets.as_slice() {
        let mut refs = vec![fleet.owner];
        match fleet.location {
            LivingFleetLocationV2::Docked { world } => refs.push(world),
            LivingFleetLocationV2::Travelling {
                origin_world,
                destination_world,
                ..
            } => refs.extend([origin_world, destination_world]),
        }
        if let Some(order) = &fleet.order {
            refs.extend(order.target_world);
            refs.extend(&order.lane_path);
        }
        rows.push((fleet.id, refs));
        rows.extend(
            fleet
                .hulls
                .as_slice()
                .iter()
                .map(|hull| (hull.id, vec![fleet.id])),
        );
    }
    rows.extend(state.routes.as_slice().iter().map(|row| {
        (
            row.id,
            vec![
                row.source_world,
                row.destination_world,
                row.source_owner,
                row.receiver_owner,
            ],
        )
    }));
    rows.extend(state.shipments.as_slice().iter().map(|row| {
        (
            row.id,
            vec![
                row.source_world,
                row.destination_world,
                row.dispatch_owner,
                row.intended_receiver,
            ],
        )
    }));
    rows.extend(
        state
            .hazards
            .as_slice()
            .iter()
            .map(|row| (row.id, vec![row.lane])),
    );
    rows
}
fn derived_removal_count(
    state: &LivingGalaxyStateV2,
    removed: &std::collections::BTreeSet<LivingEntityIdV2>,
) -> usize {
    state
        .colonies
        .as_slice()
        .iter()
        .filter(|row| removed.contains(&row.world) || removed.contains(&row.owner))
        .count()
        + state
            .observations
            .as_slice()
            .iter()
            .filter(|row| removed.contains(&row.world) || removed.contains(&row.civilization))
            .count()
        + state
            .relations
            .as_slice()
            .iter()
            .filter(|row| removed.contains(&row.from) || removed.contains(&row.to))
            .count()
        + state
            .agreements
            .as_slice()
            .iter()
            .filter(|row| {
                removed.contains(&row.participant_a) || removed.contains(&row.participant_b)
            })
            .count()
        + state
            .wars
            .as_slice()
            .iter()
            .filter(|row| {
                removed.contains(&row.participant_a) || removed.contains(&row.participant_b)
            })
            .count()
        + state
            .settlement_claims
            .as_slice()
            .iter()
            .filter(|row| removed.contains(&row.world) || removed.contains(&row.civilization))
            .count()
        + state
            .occupations
            .as_slice()
            .iter()
            .filter(|row| removed.contains(&row.world) || removed.contains(&row.claimant))
            .count()
}
fn retain_rows<T: LivingKeyedV2>(
    collection: &mut LivingSortedVecV2<T>,
    keep: impl Fn(&T) -> bool,
) -> Result<(), LivingValidationErrorV2> {
    let rows = std::mem::take(collection)
        .into_vec()
        .into_iter()
        .filter(keep)
        .collect();
    *collection = LivingSortedVecV2::new(rows)?;
    Ok(())
}
fn refresh_creator_statuses(state: &mut LivingGalaxyStateV2) {
    let owned: std::collections::BTreeSet<_> = state
        .colonies
        .as_slice()
        .iter()
        .map(|colony| colony.owner)
        .collect();
    let ids: Vec<_> = state
        .civilizations
        .as_slice()
        .iter()
        .map(|row| row.id)
        .collect();
    for id in ids {
        if let Some(row) = state.civilizations.find_mut(&id) {
            row.status = if owned.contains(&id) {
                LivingCivilizationStatusV2::Active
            } else {
                LivingCivilizationStatusV2::Dormant
            };
        }
    }
}
pub(crate) fn refund_creator_job(
    state: &mut LivingGalaxyStateV2,
    world: LivingEntityIdV2,
    paid_alloy: u32,
    effects: &mut Vec<LivingEventKindV2>,
) -> Result<(), LivingCommandRejectionV2> {
    if let Some(world) = state.worlds.find_mut(&world) {
        let refunded = world
            .inventory
            .alloy
            .checked_add(paid_alloy / 2)
            .ok_or(LivingCommandRejectionV2::Exhausted)?;
        if refunded > LIVING_RESOURCE_STORAGE_LIMIT_V2 {
            effects.push(LivingEventKindV2::RefundDiscarded);
        }
        world.inventory.alloy = refunded.min(LIVING_RESOURCE_STORAGE_LIMIT_V2);
    }
    Ok(())
}
pub(crate) fn remove_creator_entity(
    state: &mut LivingGalaxyStateV2,
    target: LivingEntityIdV2,
    cascade: &LivingCascadeDispositionV2,
    operation: usize,
    resolve: &(impl Fn(&LivingCommandTargetV2) -> LivingEntityIdV2 + ?Sized),
    _effects: &mut Vec<LivingEventKindV2>,
) -> Result<(), LivingCommandRejectionV2> {
    let preview = preview_living_removal_v2(state, target).ok_or(
        LivingCommandRejectionV2::MissingTarget {
            operation,
            reference: "removal entity",
        },
    )?;
    let reviewed = match cascade {
        LivingCascadeDispositionV2::RejectIfReferenced => {
            if !preview.dependents.is_empty() || preview.derived_records != 0 {
                return Err(LivingCommandRejectionV2::ReferencedTarget);
            }
            false
        }
        LivingCascadeDispositionV2::RemoveReviewed { .. } => {
            return Err(LivingCommandRejectionV2::CascadeMismatch);
        }
        LivingCascadeDispositionV2::RemoveListed { dependents } => {
            let declared: std::collections::BTreeSet<_> = dependents.iter().map(resolve).collect();
            let expected: std::collections::BTreeSet<_> =
                preview.dependents.iter().copied().collect();
            if declared.len() != dependents.len() || declared != expected {
                return Err(LivingCommandRejectionV2::CascadeMismatch);
            }
            true
        }
    };
    let removed: std::collections::BTreeSet<_> =
        std::iter::once(target).chain(preview.dependents).collect();
    retain_rows(&mut state.systems, |row| !removed.contains(&row.id))?;
    retain_rows(&mut state.stars, |row| !removed.contains(&row.id))?;
    retain_rows(&mut state.worlds, |row| !removed.contains(&row.id))?;
    retain_rows(&mut state.lanes, |row| !removed.contains(&row.id))?;
    retain_rows(&mut state.civilizations, |row| !removed.contains(&row.id))?;
    retain_rows(&mut state.deposits, |row| !removed.contains(&row.id))?;
    retain_rows(&mut state.facilities, |row| !removed.contains(&row.id))?;
    retain_rows(&mut state.construction_jobs, |row| {
        !removed.contains(&row.id)
    })?;
    retain_rows(&mut state.hull_jobs, |row| !removed.contains(&row.id))?;
    let ids: Vec<_> = state.fleets.as_slice().iter().map(|row| row.id).collect();
    for id in ids {
        if let Some(fleet) = state.fleets.find_mut(&id) {
            retain_rows(&mut fleet.hulls, |hull| !removed.contains(&hull.id))?;
        }
    }
    retain_rows(&mut state.fleets, |row| !removed.contains(&row.id))?;
    retain_rows(&mut state.routes, |row| !removed.contains(&row.id))?;
    retain_rows(&mut state.shipments, |row| !removed.contains(&row.id))?;
    retain_rows(&mut state.hazards, |row| !removed.contains(&row.id))?;
    if reviewed {
        retain_rows(&mut state.colonies, |row| {
            !removed.contains(&row.world) && !removed.contains(&row.owner)
        })?;
        retain_rows(&mut state.observations, |row| {
            !removed.contains(&row.world) && !removed.contains(&row.civilization)
        })?;
        retain_rows(&mut state.relations, |row| {
            !removed.contains(&row.from) && !removed.contains(&row.to)
        })?;
        retain_rows(&mut state.agreements, |row| {
            !removed.contains(&row.participant_a) && !removed.contains(&row.participant_b)
        })?;
        retain_rows(&mut state.wars, |row| {
            !removed.contains(&row.participant_a) && !removed.contains(&row.participant_b)
        })?;
        retain_rows(&mut state.settlement_claims, |row| {
            !removed.contains(&row.world) && !removed.contains(&row.civilization)
        })?;
        retain_rows(&mut state.occupations, |row| {
            !removed.contains(&row.world) && !removed.contains(&row.claimant)
        })?;
    }
    refresh_creator_statuses(state);
    Ok(())
}

pub(crate) fn advance_lifecycles(
    ctx: &mut LivingStepContextV2<'_>,
) -> Result<(), LivingDeterministicFaultV2> {
    // Move only the private boundary candidate. The resident authority stays at
    // its last valid committed boundary until every phase validates and seals.
    use super::step_job::{
        LIVING_MAX_PHASE_POLL_VISITS_V2, LivingLifecycleJobErrorV2, LivingLifecycleJobV2,
        LivingLifecycleProgressV2,
    };
    let mut job = LivingLifecycleJobV2::begin(
        std::mem::take(ctx.state),
        std::mem::take(ctx.events),
        ctx.tick,
    );
    let map_error = |error| match error {
        LivingLifecycleJobErrorV2::PreparationCapacity => {
            // Explicit noninteractive compatibility boundary retains legacy
            // process-level capacity failure, never a canonical gameplay fault.
            panic!("Living lifecycle synchronous preparation capacity exceeded")
        }
        LivingLifecycleJobErrorV2::Fault(fault) => fault,
        _ => LivingDeterministicFaultV2::Invariant,
    };
    while job
        .poll(LIVING_MAX_PHASE_POLL_VISITS_V2)
        .map_err(map_error)?
        != LivingLifecycleProgressV2::Ready
    {}
    let (state, events) = job.finish().map_err(map_error)?;
    *ctx.state = state;
    *ctx.events = events;
    Ok(())
}

pub(crate) fn resolve_arrivals(
    ctx: &mut LivingStepContextV2<'_>,
) -> Result<(), LivingDeterministicFaultV2> {
    use super::step_job::{
        LIVING_MAX_PHASE_POLL_VISITS_V2, LivingArrivalJobV2, LivingLifecycleJobErrorV2,
        LivingLifecycleProgressV2,
    };
    let mut job = LivingArrivalJobV2::begin(
        std::mem::take(ctx.state),
        std::mem::take(ctx.events),
        ctx.tick,
    );
    let map_error = |error| match error {
        LivingLifecycleJobErrorV2::PreparationCapacity => {
            panic!("Living arrival synchronous preparation capacity exceeded")
        }
        LivingLifecycleJobErrorV2::Fault(fault) => fault,
        _ => LivingDeterministicFaultV2::Invariant,
    };
    while job
        .poll(LIVING_MAX_PHASE_POLL_VISITS_V2)
        .map_err(map_error)?
        != LivingLifecycleProgressV2::Ready
    {}
    let (state, events, arrivals) = job.finish().map_err(map_error)?;
    *ctx.state = state;
    *ctx.events = events;
    ctx.arrivals.extend(arrivals);
    Ok(())
}

pub(crate) fn resolve_combat_occupation_and_claims(
    ctx: &mut LivingStepContextV2<'_>,
) -> Result<(), LivingDeterministicFaultV2> {
    retreat_noncombatants(ctx)?;
    if ctx.tick.0.is_multiple_of(COMBAT_PERIOD_TICKS) {
        resolve_combat(ctx)?;
    }
    resolve_occupation(ctx)?;
    resolve_claims(ctx)?;
    Ok(())
}

pub(crate) fn complete_jobs_and_run_recipes(
    ctx: &mut LivingStepContextV2<'_>,
) -> Result<(), LivingDeterministicFaultV2> {
    use super::step_job::{
        LivingLifecycleJobErrorV2, LivingLifecycleProgressV2, LivingProductionJobV2,
    };
    let mut job = LivingProductionJobV2::begin(
        ctx.catalog,
        std::mem::take(ctx.state),
        std::mem::take(ctx.events),
        ctx.tick,
        ctx.genesis_seed,
        ctx.branch,
    )
    .with_repair_exclusions(std::mem::take(&mut ctx.repair_exclusions));
    let map_error = |error| match error {
        LivingLifecycleJobErrorV2::PreparationCapacity => {
            panic!("Living production synchronous preparation capacity exceeded")
        }
        LivingLifecycleJobErrorV2::Fault(fault) => fault,
        _ => LivingDeterministicFaultV2::Invariant,
    };
    while job.poll(1_024).map_err(map_error)? != LivingLifecycleProgressV2::Ready {}
    let (state, events) = job.finish().map_err(map_error)?;
    *ctx.state = state;
    *ctx.events = events;
    Ok(())
}

#[cfg(test)]
fn complete_jobs_and_run_recipes_reference(
    ctx: &mut LivingStepContextV2<'_>,
) -> Result<(), LivingDeterministicFaultV2> {
    complete_construction(ctx)?;
    complete_hulls(ctx)?;
    run_hubs(ctx)?;
    run_facilities(ctx, "solar_array")?;
    run_facilities(ctx, "extractor")?;
    run_facilities(ctx, "foundry")?;
    run_repairs_reference(ctx)?;
    Ok(())
}

pub(crate) fn refresh_observations(
    ctx: &mut LivingStepContextV2<'_>,
) -> Result<(), LivingDeterministicFaultV2> {
    use super::step_job::{
        LivingLifecycleJobErrorV2, LivingLifecycleProgressV2, LivingObservationJobV2,
    };
    let mut job = LivingObservationJobV2::begin(std::mem::take(ctx.state), ctx.tick);
    loop {
        match job.poll(1024) {
            Ok(LivingLifecycleProgressV2::Pending) => {}
            Ok(LivingLifecycleProgressV2::Ready) => break,
            Err(LivingLifecycleJobErrorV2::Fault(fault)) => return Err(fault),
            Err(_) => return Err(LivingDeterministicFaultV2::Invariant),
        }
    }
    *ctx.state = job.finish().map_err(|error| match error {
        LivingLifecycleJobErrorV2::Fault(fault) => fault,
        _ => LivingDeterministicFaultV2::Invariant,
    })?;
    Ok(())
}

#[cfg(test)]
fn refresh_observations_reference(
    ctx: &mut LivingStepContextV2<'_>,
) -> Result<(), LivingDeterministicFaultV2> {
    let tick = ctx.tick;
    let civs: Vec<_> = ctx
        .state
        .civilizations
        .as_slice()
        .iter()
        .map(|civ| civ.id)
        .collect();
    for civ in civs {
        let mut worlds = Vec::new();
        for colony in ctx.state.colonies.as_slice() {
            if colony.owner != civ {
                continue;
            }
            if let Some(system) = world_system(ctx.state, colony.world) {
                push_system_worlds(ctx.state, system, &mut worlds);
            }
        }
        for fleet in ctx.state.fleets.as_slice() {
            if fleet.owner != civ {
                continue;
            }
            let LivingFleetLocationV2::Docked { world } = fleet.location else {
                continue;
            };
            let scout = fleet
                .hulls
                .as_slice()
                .iter()
                .any(|hull| hull.definition.as_str() == "scout");
            if scout {
                if let Some(system) = world_system(ctx.state, world) {
                    push_system_worlds(ctx.state, system, &mut worlds);
                }
            } else {
                push_unique(&mut worlds, world);
            }
        }
        worlds.sort();
        worlds.dedup();
        for world in worlds {
            let observation = observe_world(ctx, civ, world, tick)?;
            put(&mut ctx.state.observations, observation)?;
        }
    }
    Ok(())
}

pub(crate) fn update_diplomacy(
    ctx: &mut LivingStepContextV2<'_>,
) -> Result<(), LivingDeterministicFaultV2> {
    if ctx.tick.0.is_multiple_of(DECAY_PERIOD_TICKS) {
        decay_relations(ctx)?;
    }
    // Expiry and delivered-event counters have already run. Every bilateral
    // agreement and unilateral war proposal sees this identical updated view.
    let mut snapshot = ctx.state.clone();
    let civs: Vec<_> = snapshot
        .civilizations
        .as_slice()
        .iter()
        .filter(|civ| civ.status == LivingCivilizationStatusV2::Active)
        .map(|civ| civ.id)
        .collect();
    let snapshot_digest = living_decision_snapshot_digest_v2(&snapshot, ctx.tick)?;
    let mut diplomacy_decisions = Vec::new();
    let mut agreements = Vec::new();
    for (offset, left) in civs.iter().enumerate() {
        for right in &civs[offset + 1..] {
            let truce = snapshot.agreements.as_slice().iter().any(|agreement| {
                agreement.kind == LivingAgreementKindV2::Truce
                    && agreement.participant_a == *left
                    && agreement.participant_b == *right
                    && active_interval(agreement.start_tick, agreement.end_tick, ctx.tick)
            });
            if truce || at_war(&snapshot, *left, *right, ctx.tick) {
                continue;
            }
            let mutual = relation_total(&snapshot, *left, *right)
                .min(relation_total(&snapshot, *right, *left));
            let connected = snapshot
                .colonies
                .as_slice()
                .iter()
                .filter(|colony| colony.owner == *left)
                .any(|left_colony| {
                    snapshot
                        .colonies
                        .as_slice()
                        .iter()
                        .filter(|colony| colony.owner == *right)
                        .any(|right_colony| {
                            travel(&snapshot, left_colony.world, right_colony.world).is_some()
                        })
                });
            for (kind, eligible) in [
                (LivingAgreementKindV2::Trade, mutual >= 0 && connected),
                (LivingAgreementKindV2::Nonaggression, mutual >= 20),
            ] {
                let current = snapshot.agreements.as_slice().iter().any(|agreement| {
                    agreement.kind == kind
                        && agreement.participant_a == *left
                        && agreement.participant_b == *right
                        && active_interval(agreement.start_tick, agreement.end_tick, ctx.tick)
                });
                if eligible && !current {
                    for (actor, peer) in [(*left, *right), (*right, *left)] {
                        diplomacy_decisions.push(LivingIntentDecisionV2 {
                            phase: LivingPhaseV2::UpdateDiplomacy,
                            actor,
                            ordinal: 0,
                            action: LivingIntentActionV2::Agreement {
                                peer,
                                agreement_kind: kind,
                            },
                            target: Some(peer),
                            score: LivingDecisionScoreV2(0),
                            tier: LivingDecisionTierV2::Ordinary,
                            snapshot_tick: ctx.tick,
                            snapshot_digest,
                            observed_tick: None,
                            outcome: LivingDecisionOutcomeV2::Accepted,
                            reasons: vec![
                                LivingDecisionReasonV2::BilateralThreshold {
                                    peer,
                                    forward: relation_total(&snapshot, actor, peer),
                                    reverse: relation_total(&snapshot, peer, actor),
                                    required: if kind == LivingAgreementKindV2::Trade {
                                        0
                                    } else {
                                        20
                                    },
                                },
                                LivingDecisionReasonV2::Accepted {
                                    rule: slug(if kind == LivingAgreementKindV2::Trade {
                                        "mutual_trade_with_connection"
                                    } else {
                                        "mutual_nonaggression"
                                    })
                                    .map_err(|_| LivingDeterministicFaultV2::Invariant)?,
                                },
                            ],
                        });
                    }
                    agreements.push(LivingAgreementV2 {
                        kind,
                        participant_a: *left,
                        participant_b: *right,
                        start_tick: ctx.tick,
                        end_tick: add_ticks(ctx.tick, AGREEMENT_DURATION_TICKS)?,
                    });
                }
            }
        }
    }
    let mut proposal_events = super::receipt::LivingPendingEventsV2::new();
    let snapshot_context = LivingStepContextV2 {
        catalog: ctx.catalog,
        genesis_seed: ctx.genesis_seed,
        branch: ctx.branch,
        tick: ctx.tick,
        state: &mut snapshot,
        events: &mut proposal_events,
        arrivals: Vec::new(),
        repair_exclusions: Default::default(),
    };
    let wars: Vec<_> = civs
        .iter()
        .filter_map(|civ| {
            war_target(&snapshot_context, *civ).map(|(opponent, world)| (*civ, opponent, world))
        })
        .collect();
    for agreement in agreements {
        put(&mut ctx.state.agreements, agreement)?;
        ctx.events.record_explained(
            LivingEventProvenanceV2::Autonomous {
                phase: LivingPhaseV2::UpdateDiplomacy,
            },
            LivingEventKindV2::AgreementFormed,
            super::event_facts::LivingEventFactsV2 {
                agreement: Some(agreement),
                ..Default::default()
            },
        )?;
    }
    // Stable actor order deduplicates identical bilateral declarations without
    // allowing earlier declarations to alter later actors' chosen target.
    for (civ, opponent, world) in wars {
        let duplicate = at_war(ctx.state, civ, opponent, ctx.tick);
        declare_war(ctx, civ, opponent, world)?;
        diplomacy_decisions.push(LivingIntentDecisionV2 {
            phase: LivingPhaseV2::UpdateDiplomacy,
            actor: civ,
            ordinal: 0,
            action: LivingIntentActionV2::War { peer: opponent },
            target: Some(world),
            score: LivingDecisionScoreV2(0),
            tier: LivingDecisionTierV2::Ordinary,
            snapshot_tick: ctx.tick,
            snapshot_digest,
            observed_tick: snapshot
                .observations
                .as_slice()
                .iter()
                .find(|row| row.civilization == civ && row.world == world)
                .map(|row| row.observed_tick),
            outcome: if duplicate {
                LivingDecisionOutcomeV2::Rejected
            } else {
                LivingDecisionOutcomeV2::Accepted
            },
            reasons: vec![if duplicate {
                LivingDecisionReasonV2::InvalidTarget {
                    target: opponent,
                    rule: slug("bilateral_war_already_reserved")
                        .map_err(|_| LivingDeterministicFaultV2::Invariant)?,
                }
            } else {
                LivingDecisionReasonV2::Accepted {
                    rule: slug("fresh_adjacent_target_assembled_escort_heuristic")
                        .map_err(|_| LivingDeterministicFaultV2::Invariant)?,
                }
            }],
        });
    }
    let mut keyed = diplomacy_decisions
        .into_iter()
        .map(|decision| {
            Ok((
                (
                    decision.actor,
                    decision.action.ordinal(),
                    decision.target,
                    decision.action.canonical_bytes()?,
                ),
                decision,
            ))
        })
        .collect::<Result<Vec<_>, LivingDeterministicFaultV2>>()?;
    keyed.sort_by(|left, right| left.0.cmp(&right.0));
    let mut actor = None;
    let mut ordinal = 0u16;
    for (_, mut decision) in keyed {
        if actor != Some(decision.actor) {
            actor = Some(decision.actor);
            ordinal = 0;
        }
        decision.ordinal = ordinal;
        ordinal = ordinal
            .checked_add(1)
            .ok_or(LivingDeterministicFaultV2::Arithmetic)?;
        ctx.events.decisions.push(decision);
    }
    Ok(())
}

enum LivingIntentProposalV2 {
    Economic(u16, LivingEconomicCandidateV2),
    Fleet(Box<LivingGalaxyStateV2>, LivingIntentDecisionV2),
}

pub(crate) fn generate_and_reserve_intents(
    ctx: &mut LivingStepContextV2<'_>,
) -> Result<(), LivingDeterministicFaultV2> {
    generate_and_reserve_intents_with_disclosure(ctx, false)
}

fn generate_and_reserve_intents_with_disclosure(
    ctx: &mut LivingStepContextV2<'_>,
    _disclose_unattempted: bool,
) -> Result<(), LivingDeterministicFaultV2> {
    // All proposals see precisely the same post-observation boundary. A later
    // actor never learns about an earlier actor's accepted decision here.
    let snapshot = ctx.state.clone();
    let snapshot_digest = living_decision_snapshot_digest_v2(&snapshot, ctx.tick)?;
    let forecasts = snapshot
        .civilizations
        .as_slice()
        .iter()
        .filter(|civ| civ.status == LivingCivilizationStatusV2::Active)
        .map(|civ| Ok((civ.id, forecast_scheduled_economy(ctx, civ.id)?)))
        .collect::<Result<Vec<_>, LivingDeterministicFaultV2>>()?;
    // Each receiver originates demand from its own known state. Other actors
    // receive only this explicit request, never its private inventory.
    let mut requests = Vec::new();
    for (receiver, forecast) in &forecasts {
        for (world, resource) in &forecast.deficits {
            let stock = world_quantity(&snapshot, *world, *resource);
            if owned_by(&snapshot, *world, *receiver) && stock < 20 {
                requests.push(LivingReceiverDemandRequestV2 {
                    receiver: *receiver,
                    world: *world,
                    resource: *resource,
                    requested_units: 20 - stock,
                });
            }
        }
    }
    let mut proposals = Vec::new();
    for civ in snapshot.civilizations.as_slice() {
        let mut original = snapshot.clone();
        let mut ignored_events = super::receipt::LivingPendingEventsV2::new();
        let mut candidate = LivingStepContextV2 {
            catalog: ctx.catalog,
            genesis_seed: ctx.genesis_seed,
            branch: ctx.branch,
            tick: ctx.tick,
            state: &mut original,
            events: &mut ignored_events,
            arrivals: Vec::new(),
            repair_exclusions: Default::default(),
        };
        let economic_candidates = if civ.status == LivingCivilizationStatusV2::Active {
            reserve_economy_proposals(
                &candidate,
                civ.id,
                civ.policy.0.as_str(),
                &forecasts
                    .iter()
                    .find(|(actor, _)| *actor == civ.id)
                    .ok_or(LivingDeterministicFaultV2::Invariant)?
                    .1,
                &requests,
            )?
        } else {
            Vec::new()
        };
        let economic_count = economic_candidates.len();
        for (ordinal, economic) in economic_candidates {
            proposals.push((
                civ.id,
                true,
                LivingIntentProposalV2::Economic(ordinal, economic),
                super::receipt::LivingPendingEventsV2::new(),
            ));
        }
        reserve_settlement(&mut candidate, civ.id, civ.policy.0.as_str())?;
        let settlement = candidate.state.clone();
        *candidate.state = snapshot.clone();
        reserve_fleet(&mut candidate, civ.id)?;
        let ordinary = candidate.state.clone();
        *candidate.state = snapshot.clone();
        reserve_guard(&mut candidate, civ.id, true)?;
        let defense = candidate.state.clone();
        *candidate.state = snapshot.clone();
        reserve_guard(&mut candidate, civ.id, false)?;
        let reinforcement = candidate.state.clone();
        let mut fleets = Vec::new();
        for proposed in [settlement, ordinary, defense, reinforcement] {
            if let Some(decision) = fleet_proposal_decision(
                &snapshot,
                &proposed,
                civ.id,
                civ.policy.0.as_str(),
                ctx.tick,
                snapshot_digest,
            )? {
                let key = (
                    decision.action.ordinal(),
                    decision.target,
                    decision.action.canonical_bytes()?,
                );
                fleets.push((key, proposed, decision));
            }
        }
        fleets.sort_by(|left, right| left.0.cmp(&right.0));
        for (index, (_, proposed, decision)) in fleets.iter_mut().enumerate() {
            let ordinal = u16::try_from(economic_count + index)
                .map_err(|_| LivingDeterministicFaultV2::Arithmetic)?;
            decision.ordinal = ordinal;
            // A split uses the normative action identity ordinal; the dense
            // explanation ordinal names the proposal and its input fleet.
            let created: Vec<_> = proposed
                .fleets
                .as_slice()
                .iter()
                .filter(|fleet| {
                    !snapshot
                        .fleets
                        .as_slice()
                        .iter()
                        .any(|old| old.id == fleet.id)
                })
                .cloned()
                .collect();
            for mut fleet in created {
                remove_key(&mut proposed.fleets, &fleet.id)?;
                fleet.id = autonomous_id(
                    ctx,
                    civ.id,
                    LivingPhaseV2::GenerateIntents,
                    match decision.action {
                        LivingIntentActionV2::Explore { .. } => 14,
                        LivingIntentActionV2::Settle { .. } => 12,
                        LivingIntentActionV2::Attack { .. } => 13,
                        LivingIntentActionV2::Defend { .. } => {
                            if decision.target.is_some_and(|world| {
                                attacked_owned_world(&snapshot, decision.actor, world, ctx.tick)
                            }) {
                                10
                            } else {
                                15
                            }
                        }
                        _ => return Err(LivingDeterministicFaultV2::Invariant),
                    },
                    LivingEntityKindV2::Fleet,
                    0,
                )?;
                put(&mut proposed.fleets, fleet)?;
            }
        }
        fleets.sort_by(|left, right| {
            (
                fleet_priority(&snapshot, ctx.tick, &left.2),
                std::cmp::Reverse(left.2.score.0),
                left.2.target,
                &left.0,
            )
                .cmp(&(
                    fleet_priority(&snapshot, ctx.tick, &right.2),
                    std::cmp::Reverse(right.2.score.0),
                    right.2.target,
                    &right.0,
                ))
        });
        for (_, proposed, decision) in fleets {
            proposals.push((
                civ.id,
                false,
                LivingIntentProposalV2::Fleet(Box::new(proposed), decision),
                super::receipt::LivingPendingEventsV2::new(),
            ));
        }
    }
    // Civilization rows are sorted by ID; economy precedes fleet within an
    // actor. Each proposal reserves its entire delta or spends nothing.
    let mut accepted = Vec::new();
    for (actor, economic, proposal, events) in proposals {
        // The full immutable ranking remains available for conflict fallback.
        // After this category wins, remaining alternatives are not attempted.
        if let Some((_, _, winning_ordinal)) = accepted
            .iter()
            .find(|(winner, category, _)| *winner == actor && *category == economic)
        {
            #[cfg(test)]
            if _disclose_unattempted {
                let mut decision = match &proposal {
                    LivingIntentProposalV2::Economic(ordinal, candidate) => economic_decision(
                        &snapshot,
                        actor,
                        *ordinal,
                        candidate,
                        ctx.tick,
                        snapshot_digest,
                    )?,
                    LivingIntentProposalV2::Fleet(_, decision) => decision.clone(),
                };
                decision.reasons.push(LivingDecisionReasonV2::LowerRank {
                    winning_actor: actor,
                    winning_ordinal: *winning_ordinal,
                });
                ctx.events.decisions.push(decision);
            }
            let _ = winning_ordinal;
            continue;
        }
        let mut decision = match &proposal {
            LivingIntentProposalV2::Economic(ordinal, candidate) => Some(economic_decision(
                &snapshot,
                actor,
                *ordinal,
                candidate,
                ctx.tick,
                snapshot_digest,
            )?),
            LivingIntentProposalV2::Fleet(_, decision) => Some(decision.clone()),
        };
        let proposal = match proposal {
            LivingIntentProposalV2::Economic(ordinal, candidate) => {
                let mut original = snapshot.clone();
                let mut ignored_events = super::receipt::LivingPendingEventsV2::new();
                let original_ctx = LivingStepContextV2 {
                    catalog: ctx.catalog,
                    genesis_seed: ctx.genesis_seed,
                    branch: ctx.branch,
                    tick: ctx.tick,
                    state: &mut original,
                    events: &mut ignored_events,
                    arrivals: Vec::new(),
                    repair_exclusions: Default::default(),
                };
                apply_economic_proposal(&original_ctx, actor, ordinal, candidate)?
            }
            LivingIntentProposalV2::Fleet(state, _) => *state,
        };
        let mut reserved = ctx.state.clone();
        if !reserve_delta(&snapshot.worlds, &proposal.worlds, &mut reserved.worlds)?
            || !reserve_delta(
                &snapshot.hull_jobs,
                &proposal.hull_jobs,
                &mut reserved.hull_jobs,
            )?
            || !reserve_delta(
                &snapshot.construction_jobs,
                &proposal.construction_jobs,
                &mut reserved.construction_jobs,
            )?
            || !reserve_delta(
                &snapshot.facilities,
                &proposal.facilities,
                &mut reserved.facilities,
            )?
            || !reserve_delta(&snapshot.fleets, &proposal.fleets, &mut reserved.fleets)?
            || !reserve_delta(&snapshot.routes, &proposal.routes, &mut reserved.routes)?
        {
            if let Some(mut decision) = decision {
                decision
                    .reasons
                    .push(LivingDecisionReasonV2::InvalidTarget {
                        target: decision.target.unwrap_or(actor),
                        rule: slug("reservation_conflict")
                            .map_err(|_| LivingDeterministicFaultV2::Invariant)?,
                    });
                ctx.events.decisions.push(decision);
            }
            continue;
        }
        let completed_tick = reserved.tick;
        reserved.tick = ctx.tick;
        let validation = reserved.validate(ctx.catalog);
        reserved.tick = completed_tick;
        match validation {
            Ok(()) => {
                for event in events.payloads() {
                    ctx.emit(event.provenance, event.kind)?;
                }
                *ctx.state = reserved;
                let ordinal = decision.as_ref().map_or(0, |decision| decision.ordinal);
                if let Some(decision) = decision.as_mut() {
                    decision.outcome = LivingDecisionOutcomeV2::Accepted;
                    decision.reasons.push(LivingDecisionReasonV2::Accepted {
                        rule: slug("atomic_reservation")
                            .map_err(|_| LivingDeterministicFaultV2::Invariant)?,
                    });
                }
                accepted.push((actor, economic, ordinal));
            }
            Err(LivingValidationErrorV2::Capacity {
                collection,
                limit,
                found,
            }) => {
                if let Some(decision) = decision.as_mut() {
                    decision
                        .reasons
                        .push(LivingDecisionReasonV2::QueueCapacity {
                            world: decision.target.unwrap_or(actor),
                            queue: slug(collection.replace(' ', "_").as_str())
                                .map_err(|_| LivingDeterministicFaultV2::Invariant)?,
                            limit: u32::try_from(limit)
                                .map_err(|_| LivingDeterministicFaultV2::Arithmetic)?,
                            reserved: u32::try_from(found)
                                .map_err(|_| LivingDeterministicFaultV2::Arithmetic)?,
                        });
                }
            }
            Err(error) => return Err(error.into()),
        }
        if let Some(decision) = decision {
            ctx.events.decisions.push(decision);
        }
    }
    ctx.events
        .decisions
        .sort_by_key(|decision| (decision.phase.ordinal(), decision.actor, decision.ordinal));
    // Candidate search ordinals identify the complete immutable ordering;
    // portable disclosure ordinals identify only actual attempts, densely.
    let mut remap = std::collections::BTreeMap::new();
    let mut previous = None;
    let mut next = 0_u32;
    for decision in &mut ctx.events.decisions {
        let group = (decision.phase.ordinal(), decision.actor);
        if previous != Some(group) {
            next = 0;
            previous = Some(group);
        }
        let dense = u16::try_from(next).map_err(|_| LivingDeterministicFaultV2::Arithmetic)?;
        remap.insert((group.0, group.1, decision.ordinal), dense);
        decision.ordinal = dense;
        next += 1;
    }
    for decision in &mut ctx.events.decisions {
        for reason in &mut decision.reasons {
            if let LivingDecisionReasonV2::LowerRank {
                winning_actor,
                winning_ordinal,
            } = reason
            {
                *winning_ordinal = *remap
                    .get(&(decision.phase.ordinal(), *winning_actor, *winning_ordinal))
                    .ok_or(LivingDeterministicFaultV2::Invariant)?;
            }
        }
    }
    Ok(())
}

/// Explain a single role proposal using its existing input fleet and target.
fn fleet_proposal_decision(
    snapshot: &LivingGalaxyStateV2,
    proposal: &LivingGalaxyStateV2,
    actor: LivingEntityIdV2,
    policy: &str,
    tick: LivingTickV2,
    snapshot_digest: super::ids::LivingStateDigestV2,
) -> Result<Option<LivingIntentDecisionV2>, LivingDeterministicFaultV2> {
    let Some(fleet) = proposal
        .fleets
        .as_slice()
        .iter()
        .filter(|fleet| fleet.owner == actor)
        .find(|fleet| {
            fleet.order.is_some()
                && !snapshot
                    .fleets
                    .as_slice()
                    .iter()
                    .find(|old| old.id == fleet.id)
                    .is_some_and(|old| old.order == fleet.order)
        })
    else {
        return Ok(None);
    };
    let order = fleet
        .order
        .as_ref()
        .ok_or(LivingDeterministicFaultV2::Invariant)?;
    let target = order
        .target_world
        .ok_or(LivingDeterministicFaultV2::Invariant)?;
    let input = if snapshot
        .fleets
        .as_slice()
        .iter()
        .any(|old| old.id == fleet.id)
    {
        fleet.id
    } else {
        snapshot
            .fleets
            .as_slice()
            .iter()
            .find(|old| {
                fleet.hulls.as_slice().iter().all(|hull| {
                    old.hulls
                        .as_slice()
                        .iter()
                        .any(|old_hull| old_hull.id == hull.id)
                })
            })
            .ok_or(LivingDeterministicFaultV2::Invariant)?
            .id
    };
    let (action, score, bonus) = match order.kind {
        LivingFleetOrderKindV2::Settle => (
            LivingIntentActionV2::Settle {
                fleet: input,
                destination: target,
            },
            60,
            if policy == "expansionist" { 20 } else { 0 },
        ),
        LivingFleetOrderKindV2::Attack => (
            LivingIntentActionV2::Attack {
                fleet: input,
                world: target,
            },
            60,
            0,
        ),
        LivingFleetOrderKindV2::Defend => (
            LivingIntentActionV2::Defend {
                fleet: input,
                world: target,
            },
            if attacked_owned_world(snapshot, actor, target, tick) {
                60
            } else {
                40
            },
            if policy == "guardian" { 20 } else { 0 },
        ),
        LivingFleetOrderKindV2::Explore => (
            LivingIntentActionV2::Explore {
                fleet: input,
                destination: target,
            },
            50,
            0,
        ),
        _ => return Err(LivingDeterministicFaultV2::Invariant),
    };
    let reasons = if bonus > 0 {
        vec![LivingDecisionReasonV2::PolicyScore {
            policy: slug(policy).map_err(|_| LivingDeterministicFaultV2::Invariant)?,
            component: slug(if matches!(action, LivingIntentActionV2::Defend { .. }) {
                "defend"
            } else {
                "settlement"
            })
            .map_err(|_| LivingDeterministicFaultV2::Invariant)?,
            value: LivingDecisionScoreV2(bonus),
        }]
    } else {
        Vec::new()
    };
    Ok(Some(LivingIntentDecisionV2 {
        phase: LivingPhaseV2::GenerateIntents,
        actor,
        ordinal: 0,
        action,
        target: Some(target),
        score: LivingDecisionScoreV2(score + bonus),
        tier: if snapshot
            .civilizations
            .as_slice()
            .iter()
            .any(|row| row.id == actor && row.status == LivingCivilizationStatusV2::Dormant)
        {
            LivingDecisionTierV2::Recovery
        } else {
            LivingDecisionTierV2::Ordinary
        },
        snapshot_tick: tick,
        snapshot_digest,
        observed_tick: snapshot
            .observations
            .as_slice()
            .iter()
            .find(|observation| observation.civilization == actor && observation.world == target)
            .map(|observation| observation.observed_tick),
        outcome: LivingDecisionOutcomeV2::Rejected,
        reasons,
    }))
}

/// Merge changed rows only if earlier reservations left their snapshot value
/// untouched. Capacity validation happens on the complete candidate afterward.
fn reserve_delta<T>(
    snapshot: &LivingSortedVecV2<T>,
    proposal: &LivingSortedVecV2<T>,
    reserved: &mut LivingSortedVecV2<T>,
) -> Result<bool, LivingDeterministicFaultV2>
where
    T: LivingKeyedV2 + Clone + Eq,
{
    for row in proposal.as_slice() {
        let key = row.living_key();
        let before = snapshot
            .as_slice()
            .binary_search_by_key(&key, T::living_key)
            .ok()
            .map(|index| &snapshot.as_slice()[index]);
        if before == Some(row) {
            continue;
        }
        let current = reserved
            .as_slice()
            .binary_search_by_key(&key, T::living_key)
            .ok()
            .map(|index| &reserved.as_slice()[index]);
        if current != before {
            return Ok(false);
        }
        put(reserved, row.clone())?;
    }
    for old in snapshot.as_slice() {
        let key = old.living_key();
        if proposal
            .as_slice()
            .binary_search_by_key(&key, T::living_key)
            .is_ok()
        {
            continue;
        }
        if reserved
            .as_slice()
            .binary_search_by_key(&key, T::living_key)
            .ok()
            .map(|index| &reserved.as_slice()[index])
            != Some(old)
        {
            return Ok(false);
        }
        remove_key(reserved, &key)?;
    }
    Ok(true)
}

pub(crate) fn dispatch_orders_and_freight(
    ctx: &mut LivingStepContextV2<'_>,
) -> Result<(), LivingDeterministicFaultV2> {
    dispatch_fleets(ctx)?;
    dispatch_routes(ctx)?;
    Ok(())
}

/// The curated one-system sandbox the shell and the play suite both step.
pub fn builtin_living_sandbox_v2() -> Result<LivingGalaxyAuthorityV2, String> {
    let catalog = living_core_pack_v2().map_err(|error| error.to_string())?;
    let state = sandbox_state()?;
    let manifest = LivingGenesisManifestV2 {
        kind: LIVING_GENESIS_KIND_V2.to_owned(),
        format_version: LIVING_GENESIS_FORMAT_VERSION_V2,
        rules_version: LIVING_RULES_VERSION,
        state,
    };
    let genesis = validate_living_genesis_manifest_v2(manifest, &catalog)
        .map_err(|error| error.to_string())?;
    let generator = LivingGenesisGeneratorV2 {
        generator_id: LivingSlugV2::new("builtin_sandbox").map_err(|error| error.to_string())?,
        generator_version: 1,
    };
    LivingGalaxyAuthorityV2::from_genesis(catalog, [0x4c; 32], generator, genesis)
        .map_err(|error| error.to_string())
}

/// The approved isolated construction, exploration and settlement fixture.
pub fn builtin_first_expansion_v2() -> Result<LivingGalaxyAuthorityV2, String> {
    let catalog = living_core_pack_v2().map_err(|error| error.to_string())?;
    let mut state = LivingGalaxyStateV2::default();
    let home_system = eid(1);
    let frontier_system = eid(2);
    let civilization = eid(3);
    let home = eid(4);
    let frontier = eid(5);
    install(
        &mut state.systems,
        LivingSystemV2 {
            id: home_system,
            name: name("Founding System")?,
        },
    )?;
    install(
        &mut state.systems,
        LivingSystemV2 {
            id: frontier_system,
            name: name("Frontier System")?,
        },
    )?;
    install(
        &mut state.lanes,
        super::model::LivingLaneV2 {
            id: eid(6),
            system_a: home_system,
            system_b: frontier_system,
            distance_units: 100,
        },
    )?;
    install(
        &mut state.civilizations,
        LivingCivilizationV2 {
            id: civilization,
            name: name("Founders")?,
            policy: LivingPolicyV2(slug("neutral")?),
            status: LivingCivilizationStatusV2::Active,
        },
    )?;
    install(
        &mut state.worlds,
        world(
            home,
            home_system,
            "barren",
            "Founding World",
            Some(civilization),
            LivingInventoryV2 {
                energy: 100,
                ore: 100,
                alloy: 120,
            },
        )?,
    )?;
    install(
        &mut state.worlds,
        world(
            frontier,
            frontier_system,
            "barren",
            "Frontier World",
            None,
            LivingInventoryV2::default(),
        )?,
    )?;
    install(&mut state.colonies, colony(home, civilization)?)?;
    install(
        &mut state.deposits,
        LivingDepositV2 {
            id: eid(7),
            world: home,
            resource: LivingResourceV2::Ore,
            remaining_units: 20_000,
        },
    )?;
    install(
        &mut state.facilities,
        facility(eid(8), home, "solar_array", Some(LivingTickV2(10)))?,
    )?;
    install(
        &mut state.facilities,
        facility(eid(9), home, "extractor", Some(LivingTickV2(10)))?,
    )?;
    install(
        &mut state.facilities,
        facility(eid(10), home, "shipyard", None)?,
    )?;
    install(
        &mut state.fleets,
        fleet_with(
            eid(11),
            civilization,
            home,
            vec![
                hull(eid(12), "scout", false)?,
                hull(eid(13), "escort", false)?,
                hull(eid(14), "escort", false)?,
            ],
        )?,
    )?;
    let genesis = validate_living_genesis_manifest_v2(
        LivingGenesisManifestV2 {
            kind: LIVING_GENESIS_KIND_V2.to_owned(),
            format_version: LIVING_GENESIS_FORMAT_VERSION_V2,
            rules_version: LIVING_RULES_VERSION,
            state,
        },
        &catalog,
    )
    .map_err(|error| error.to_string())?;
    let mut seed = [0_u8; 32];
    seed[24..].copy_from_slice(&0x4E59_4F4E_5F45_5850_u64.to_be_bytes());
    LivingGalaxyAuthorityV2::from_genesis(
        catalog,
        seed,
        LivingGenesisGeneratorV2 {
            generator_id: slug("first_expansion")?,
            generator_version: 1,
        },
        genesis,
    )
    .map_err(|error| error.to_string())
}

fn sandbox_state() -> Result<LivingGalaxyStateV2, String> {
    let mut state = LivingGalaxyStateV2::default();
    let system = eid(1);
    let star = eid(2);
    let civ_a = eid(3);
    let civ_b = eid(4);
    let home_a = eid(5);
    let home_b = eid(6);
    let frontier = eid(7);
    let depot = eid(8);
    install(
        &mut state.systems,
        LivingSystemV2 {
            id: system,
            name: name("Core")?,
        },
    )?;
    install(
        &mut state.stars,
        LivingStarV2 {
            id: star,
            system,
            archetype: slug("main_sequence")?,
            name: name("Solace")?,
        },
    )?;
    install(
        &mut state.civilizations,
        LivingCivilizationV2 {
            id: civ_a,
            name: name("Arbor")?,
            policy: LivingPolicyV2(slug("expansionist")?),
            status: LivingCivilizationStatusV2::Active,
        },
    )?;
    install(
        &mut state.civilizations,
        LivingCivilizationV2 {
            id: civ_b,
            name: name("Bastion")?,
            policy: LivingPolicyV2(slug("guardian")?),
            status: LivingCivilizationStatusV2::Active,
        },
    )?;
    install(
        &mut state.worlds,
        world(
            home_a,
            system,
            "temperate",
            "Home A",
            Some(civ_a),
            LivingInventoryV2 {
                energy: 100,
                ore: 100,
                alloy: 120,
            },
        )?,
    )?;
    install(
        &mut state.worlds,
        world(
            home_b,
            system,
            "temperate",
            "Home B",
            Some(civ_b),
            LivingInventoryV2 {
                energy: 100,
                ore: 100,
                alloy: 40,
            },
        )?,
    )?;
    install(
        &mut state.worlds,
        world(
            frontier,
            system,
            "barren",
            "Frontier",
            None,
            LivingInventoryV2::default(),
        )?,
    )?;
    install(
        &mut state.worlds,
        world(
            depot,
            system,
            "barren",
            "Depot",
            Some(civ_a),
            LivingInventoryV2 {
                energy: 20,
                ore: 100,
                alloy: 20,
            },
        )?,
    )?;
    install(
        &mut state.deposits,
        LivingDepositV2 {
            id: eid(9),
            world: home_a,
            resource: LivingResourceV2::Ore,
            remaining_units: 20_000,
        },
    )?;
    install(&mut state.colonies, colony(home_a, civ_a)?)?;
    install(&mut state.colonies, colony(home_b, civ_b)?)?;
    install(&mut state.colonies, colony(depot, civ_a)?)?;
    install(
        &mut state.facilities,
        facility(eid(10), home_a, "solar_array", Some(LivingTickV2(10)))?,
    )?;
    install(
        &mut state.facilities,
        facility(eid(11), home_a, "extractor", Some(LivingTickV2(10)))?,
    )?;
    install(
        &mut state.facilities,
        facility(eid(12), home_a, "foundry", Some(LivingTickV2(20)))?,
    )?;
    install(
        &mut state.facilities,
        facility(eid(13), home_a, "shipyard", None)?,
    )?;
    install(
        &mut state.fleets,
        fleet_with(
            eid(14),
            civ_a,
            home_a,
            vec![
                hull(eid(15), "escort", false)?,
                hull(eid(16), "escort", false)?,
                hull(eid(17), "escort", false)?,
            ],
        )?,
    )?;
    install(
        &mut state.fleets,
        fleet_with(
            eid(18),
            civ_b,
            home_b,
            vec![hull(eid(19), "escort", false)?],
        )?,
    )?;
    install(
        &mut state.fleets,
        fleet_with(eid(20), civ_a, home_b, vec![hull(eid(21), "scout", true)?])?,
    )?;
    install(
        &mut state.routes,
        LivingFreightRouteV2 {
            id: eid(22),
            kind: LivingRouteKindV2::Internal,
            source_world: home_a,
            destination_world: depot,
            resource: LivingResourceV2::Ore,
            batch_size: 10,
            cadence_ticks: 10,
            source_reserve: 20,
            source_owner: civ_a,
            receiver_owner: civ_a,
            next_due: LivingTickV2(10),
            suspended: false,
        },
    )?;
    install(
        &mut state.relations,
        LivingRelationV2 {
            from: civ_a,
            to: civ_b,
            base_disposition: -40,
            reasons: LivingRelationReasonsV2::default(),
        },
    )?;
    install(
        &mut state.relations,
        LivingRelationV2 {
            from: civ_b,
            to: civ_a,
            base_disposition: -40,
            reasons: LivingRelationReasonsV2::default(),
        },
    )?;
    Ok(state)
}

fn reserve_exploration(
    ctx: &mut LivingStepContextV2<'_>,
    civ: LivingEntityIdV2,
) -> Result<(), LivingDeterministicFaultV2> {
    if !ctx
        .state
        .civilizations
        .as_slice()
        .iter()
        .any(|row| row.id == civ && row.status == LivingCivilizationStatusV2::Active)
    {
        return Ok(());
    }
    let fleet = ctx.state.fleets.as_slice().iter().find(|fleet| fleet.owner == civ && fleet.order.is_none()
        && matches!(fleet.location, LivingFleetLocationV2::Docked { world } if owned_by(ctx.state, world, civ))
        && fleet.hulls.as_slice().iter().any(|hull| hull.definition.as_str() == "scout"
            && matches!(fleet.location, LivingFleetLocationV2::Docked { world }
                if world_quantity(ctx.state, world, LivingResourceV2::Energy) >= if hull.return_credit { 2 } else { 4 }))
        && !ctx.state.hull_jobs.as_slice().iter().any(|job| job.target_fleet == Some(fleet.id))).cloned();
    let Some(mut fleet) = fleet else {
        return Ok(());
    };
    let LivingFleetLocationV2::Docked { world: origin } = fleet.location else {
        return Err(LivingDeterministicFaultV2::Invariant);
    };
    let target = ctx
        .state
        .worlds
        .as_slice()
        .iter()
        .filter(|world| world.id != origin)
        .filter_map(|world| {
            let observation = ctx
                .state
                .observations
                .as_slice()
                .iter()
                .find(|row| row.civilization == civ && row.world == world.id);
            if observation.is_some_and(|row| row.observed_tick == ctx.tick) {
                return None;
            }
            let system_observed = ctx.state.observations.as_slice().iter().any(|row| {
                row.civilization == civ && world_system(ctx.state, row.world) == Some(world.system)
            });
            travel(ctx.state, origin, world.id).map(|(duration, _)| {
                (
                    system_observed,
                    observation.map_or(0, |row| row.observed_tick.0),
                    duration,
                    world.system,
                    world.id,
                )
            })
        })
        .min()
        .map(|(_, _, _, _, world)| world);
    let Some(target) = target else { return Ok(()) };
    let order = LivingFleetOrderV2 {
        kind: LivingFleetOrderKindV2::Explore,
        target_world: Some(target),
        not_before_tick: add_ticks(ctx.tick, 1)?,
        lane_path: Vec::new(),
    };
    if fleet.hulls.as_slice().len() == 1 {
        fleet.order = Some(order);
        put(&mut ctx.state.fleets, fleet)?;
    } else {
        let scout = fleet
            .hulls
            .as_slice()
            .iter()
            .find(|hull| {
                hull.definition.as_str() == "scout"
                    && world_quantity(ctx.state, origin, LivingResourceV2::Energy)
                        >= if hull.return_credit { 2 } else { 4 }
            })
            .cloned()
            .ok_or(LivingDeterministicFaultV2::Invariant)?;
        remove_key(&mut fleet.hulls, &scout.id)?;
        let id = autonomous_id(
            ctx,
            civ,
            LivingPhaseV2::GenerateIntents,
            2,
            LivingEntityKindV2::Fleet,
            0,
        )?;
        put(&mut ctx.state.fleets, fleet)?;
        put(
            &mut ctx.state.fleets,
            LivingFleetV2 {
                id,
                owner: civ,
                location: LivingFleetLocationV2::Docked { world: origin },
                order: Some(order),
                hulls: LivingSortedVecV2::new(vec![scout])
                    .map_err(LivingValidationErrorV2::from)?,
            },
        )?;
    }
    Ok(())
}

#[cfg(test)]
fn reference_forecast_input(
    source: &LivingGalaxyStateV2,
    actor: LivingEntityIdV2,
    tick: LivingTickV2,
) -> Result<(LivingGalaxyStateV2, Vec<LivingEntityIdV2>), LivingDeterministicFaultV2> {
    let mut state = source.clone();
    let idle_facilities: Vec<_> = state
        .facilities
        .as_slice()
        .iter()
        .filter(|facility| {
            owned_by(&state, facility.world, actor)
                && matches!(
                    facility.definition.as_str(),
                    "solar_array" | "extractor" | "foundry"
                )
        })
        .map(|facility| facility.id)
        .collect();
    state.tick = tick;
    // Future foreign dispatches are private and uncommitted. Already travelling
    // cargo retains its explicit amount and follows its recorded disposition.
    let foreign_routes: Vec<_> = state
        .routes
        .as_slice()
        .iter()
        .filter(|route| route.source_owner != actor)
        .map(|route| route.id)
        .collect();
    for id in foreign_routes {
        remove_key(&mut state.routes, &id)?;
    }
    let foreign_facilities: Vec<_> = state
        .facilities
        .as_slice()
        .iter()
        .filter(|facility| !owned_by(&state, facility.world, actor))
        .map(|facility| facility.id)
        .collect();
    for id in foreign_facilities {
        remove_key(&mut state.facilities, &id)?;
    }
    let foreign_builds: Vec<_> = state
        .construction_jobs
        .as_slice()
        .iter()
        .filter(|job| job.owner != actor)
        .map(|job| job.id)
        .collect();
    for id in foreign_builds {
        remove_key(&mut state.construction_jobs, &id)?;
    }
    let foreign_hulls: Vec<_> = state
        .hull_jobs
        .as_slice()
        .iter()
        .filter(|job| job.owner != actor)
        .map(|job| job.id)
        .collect();
    for id in foreign_hulls {
        remove_key(&mut state.hull_jobs, &id)?;
    }
    let foreign_colonies: Vec<_> = state
        .colonies
        .as_slice()
        .iter()
        .filter(|colony| colony.owner != actor)
        .map(|colony| colony.world)
        .collect();
    for world in foreign_colonies {
        remove_key(&mut state.colonies, &world)?;
    }
    for world in state.worlds.as_slice().to_vec() {
        if !owned_by(&state, world.id, actor) {
            // Foreign ownership is dynamic intelligence, just like facilities.
            // An existing contract supplies its recorded parties only when no
            // later observation exists; unseen capture must not change forecasts.
            let known_owner = state
                .observations
                .as_slice()
                .iter()
                .find(|observation| {
                    observation.civilization == actor && observation.world == world.id
                })
                .map(|observation| observation.owner)
                .unwrap_or_else(|| {
                    state
                        .routes
                        .as_slice()
                        .iter()
                        .find(|route| {
                            route.source_owner == actor && route.destination_world == world.id
                        })
                        .map(|route| route.receiver_owner)
                        .or_else(|| {
                            state.shipments.as_slice().iter().find_map(|shipment| {
                                if shipment.dispatch_owner == actor
                                    && shipment.destination_world == world.id
                                {
                                    Some(shipment.intended_receiver)
                                } else if shipment.intended_receiver == actor
                                    && shipment.source_world == world.id
                                {
                                    Some(shipment.dispatch_owner)
                                } else {
                                    None
                                }
                            })
                        })
                });
            let stored = state
                .worlds
                .find_mut(&world.id)
                .ok_or(LivingDeterministicFaultV2::Invariant)?;
            stored.inventory = LivingInventoryV2::default();
            stored.owner = known_owner;
        }
    }
    Ok((state, idle_facilities))
}

/// Integer forecast of already scheduled economic work. This is private
/// decision input: no commands, diplomacy proposals, or desired jobs enter it.
#[derive(Debug, Eq, PartialEq)]
struct LivingScheduledForecastV2 {
    inventories: Vec<(LivingTickV2, LivingEntityIdV2, LivingInventoryV2)>,
    deficits: Vec<(LivingEntityIdV2, LivingResourceV2)>,
    idle_facilities: Vec<LivingEntityIdV2>,
    // Private ownership retained for budgeted reclamation by the intent cursor.
    final_state: LivingGalaxyStateV2,
}

fn forecast_scheduled_economy(
    ctx: &LivingStepContextV2<'_>,
    actor: LivingEntityIdV2,
) -> Result<LivingScheduledForecastV2, LivingDeterministicFaultV2> {
    forecast_scheduled_economy_with_explanations(ctx, actor, false)
}

#[cfg(test)]
fn reference_scheduled_forecast(
    ctx: &LivingStepContextV2<'_>,
    actor: LivingEntityIdV2,
    explanations: bool,
) -> Result<LivingScheduledForecastV2, LivingDeterministicFaultV2> {
    let (mut state, mut idle_facilities) = reference_forecast_input(ctx.state, actor, ctx.tick)?;
    let mut inventories = Vec::new();
    let mut deficits = Vec::new();
    for offset in 1..=200 {
        let tick = add_ticks(ctx.tick, offset)?;
        let mut events = if explanations {
            super::receipt::LivingPendingEventsV2::new()
        } else {
            super::receipt::LivingPendingEventsV2::for_forecast()
        };
        let mut forecast = LivingStepContextV2 {
            catalog: ctx.catalog,
            genesis_seed: ctx.genesis_seed,
            branch: ctx.branch,
            tick,
            state: &mut state,
            events: &mut events,
            arrivals: Vec::new(),
            repair_exclusions: Default::default(),
        };
        advance_lifecycles(&mut forecast)?;
        resolve_arrivals(&mut forecast)?;
        // Independent admission reference runs the original conflict kernels on
        // a detached test snapshot, retaining only world exclusions.
        let mut projected = forecast.state.clone();
        let mut ignored = super::receipt::LivingPendingEventsV2::for_forecast();
        let mut admission = LivingStepContextV2 {
            catalog: forecast.catalog,
            genesis_seed: forecast.genesis_seed,
            branch: forecast.branch,
            tick,
            state: &mut projected,
            events: &mut ignored,
            arrivals: Vec::new(),
            repair_exclusions: Default::default(),
        };
        retreat_noncombatants(&mut admission)?;
        if tick.0.is_multiple_of(COMBAT_PERIOD_TICKS) {
            resolve_combat(&mut admission)?;
        }
        forecast.repair_exclusions = admission.repair_exclusions;
        let mut repairs = Vec::new();
        for f in forecast.state.fleets.as_slice() {
            let LivingFleetLocationV2::Docked { world } = f.location else {
                continue;
            };
            if f.owner != actor
                || !owned_by(forecast.state, world, actor)
                || forecast.repair_exclusions.contains(&world)
            {
                continue;
            }
            for h in f.hulls.as_slice() {
                if h.hit_points > 0
                    && h.next_repair.is_some_and(|d| d <= tick)
                    && h.hit_points
                        < forecast
                            .catalog
                            .hull(h.definition.as_str())
                            .map_or(1, |d| d.hit_points)
                {
                    repairs.push((h.id, Some(f.id), world, h.hit_points));
                }
            }
        }
        for f in forecast.state.facilities.as_slice() {
            if f.hit_points > 0
                && f.next_repair.is_some_and(|d| d <= tick)
                && owned_by(forecast.state, f.world, actor)
                && !forecast.repair_exclusions.contains(&f.world)
                && let Some(d) = forecast.catalog.industry(f.definition.as_str())
                && let LivingIndustryEffectV2::Defense { hit_points, .. } = d.effect
                && f.hit_points < hit_points
            {
                repairs.push((f.id, None, f.world, f.hit_points));
            }
        }
        repairs.sort_by_key(|r| r.0);
        let due: Vec<_> = forecast
            .state
            .facilities
            .as_slice()
            .iter()
            .filter(|facility| facility.next_due.is_some_and(|due| due.0 <= tick.0))
            .map(|facility| facility.id)
            .collect();
        complete_jobs_and_run_recipes(&mut forecast)?;
        idle_facilities.retain(|id| {
            !due.contains(id)
                || !forecast.state.facilities.as_slice().iter().any(|facility| {
                    facility.id == *id && facility.status == LivingFacilityStatusV2::Operational
                })
        });
        for (id, fleet, world, previous_hp) in repairs {
            let after = match fleet {
                Some(fleet) => forecast
                    .state
                    .fleets
                    .as_slice()
                    .iter()
                    .find(|f| f.id == fleet)
                    .and_then(|f| f.hulls.as_slice().iter().find(|h| h.id == id))
                    .map(|h| (h.hit_points, h.next_repair)),
                None => forecast
                    .state
                    .facilities
                    .as_slice()
                    .iter()
                    .find(|f| f.id == id)
                    .map(|f| (f.hit_points, f.next_repair)),
            };
            if after.is_some_and(|(hp, next)| hp == previous_hp && next.is_some())
                && !deficits.contains(&(world, LivingResourceV2::Energy))
            {
                deficits.push((world, LivingResourceV2::Energy));
            }
        }
        for facility in forecast
            .state
            .facilities
            .as_slice()
            .iter()
            .filter(|facility| {
                due.contains(&facility.id) && owned_by(forecast.state, facility.world, actor)
            })
        {
            for reason in facility.blocked_reasons.as_slice() {
                if let LivingBlockedReasonV2::MissingInput { resource, .. } = reason
                    && !deficits.contains(&(facility.world, *resource))
                {
                    deficits.push((facility.world, *resource));
                }
            }
        }
        // Existing authorized route batches are known scheduled demand. A
        // partial/zero dispatch does not erase its source-side shortage.
        for route in forecast.state.routes.as_slice().iter().filter(|route| {
            route.source_owner == actor
                && route.next_due <= tick
                && !route.suspended
                && owned_by(forecast.state, route.source_world, actor)
                && world_owner(forecast.state, route.destination_world)
                    == Some(route.receiver_owner)
                && travel(forecast.state, route.source_world, route.destination_world).is_some()
                && (route.kind != LivingRouteKindV2::Trade
                    || trade_open(
                        forecast.state,
                        route.source_owner,
                        route.receiver_owner,
                        tick,
                    ))
        }) {
            let batch = hazard_batch(&forecast, route, route.batch_size);
            if batch > 0
                && world_quantity(forecast.state, route.source_world, route.resource)
                    < route.source_reserve.saturating_add(batch)
                && !deficits.contains(&(route.source_world, route.resource))
            {
                deficits.push((route.source_world, route.resource));
            }
        }
        dispatch_orders_and_freight(&mut forecast)?;
        forecast.state.tick = tick;
        for world in forecast
            .state
            .worlds
            .as_slice()
            .iter()
            .filter(|world| world.owner == Some(actor))
        {
            inventories.push((tick, world.id, world.inventory));
        }
    }
    Ok(LivingScheduledForecastV2 {
        inventories,
        deficits,
        idle_facilities,
        #[cfg(test)]
        final_state: state,
    })
}

#[derive(Clone, Eq, PartialEq)]
enum LivingEconomicActionV2 {
    Construction {
        world: LivingEntityIdV2,
        definition: &'static str,
        scrap: Option<LivingEntityIdV2>,
    },
    Hull {
        world: LivingEntityIdV2,
        definition: &'static str,
    },
    Supply {
        source: LivingEntityIdV2,
        destination: LivingEntityIdV2,
        resource: LivingResourceV2,
        kind: LivingRouteKindV2,
        receiver: LivingEntityIdV2,
        batch_size: u32,
        existing_route: Option<LivingEntityIdV2>,
    },
}

#[derive(Debug, Eq, PartialEq)]
struct LivingReceiverDemandRequestV2 {
    receiver: LivingEntityIdV2,
    world: LivingEntityIdV2,
    resource: LivingResourceV2,
    requested_units: u32,
}

#[derive(Eq, PartialEq)]
struct LivingEconomicCandidateV2 {
    tier: u8,
    score: u16,
    target: LivingEntityIdV2,
    action: LivingEconomicActionV2,
}
impl LivingEconomicCandidateV2 {
    fn tie(&self) -> (u8, &'static str) {
        match self.action {
            LivingEconomicActionV2::Construction { definition, .. } => (0, definition),
            LivingEconomicActionV2::Hull { definition, .. } => (1, definition),
            LivingEconomicActionV2::Supply { resource, .. } => (2, resource.as_wire_str()),
        }
    }
}

fn reserve_economy_proposals(
    ctx: &LivingStepContextV2<'_>,
    civ: LivingEntityIdV2,
    policy: &str,
    forecast: &LivingScheduledForecastV2,
    requests: &[LivingReceiverDemandRequestV2],
) -> Result<Vec<(u16, LivingEconomicCandidateV2)>, LivingDeterministicFaultV2> {
    let mut candidates = Vec::new();
    let colony_count = ctx
        .state
        .colonies
        .as_slice()
        .iter()
        .filter(|colony| colony.owner == civ)
        .count();
    let hull_count = |definition: &str| {
        ctx.state
            .fleets
            .as_slice()
            .iter()
            .filter(|fleet| fleet.owner == civ)
            .map(|fleet| {
                fleet
                    .hulls
                    .as_slice()
                    .iter()
                    .filter(|hull| hull.definition.as_str() == definition)
                    .count()
            })
            .sum::<usize>()
            + ctx
                .state
                .hull_jobs
                .as_slice()
                .iter()
                .filter(|job| job.owner == civ && job.definition.as_str() == definition)
                .count()
    };
    for colony in ctx
        .state
        .colonies
        .as_slice()
        .iter()
        .filter(|colony| colony.owner == civ)
    {
        let world = ctx
            .state
            .worlds
            .as_slice()
            .iter()
            .find(|world| world.id == colony.world)
            .ok_or(LivingDeterministicFaultV2::Invariant)?;
        let has = |definition: &str| {
            ctx.state.facilities.as_slice().iter().any(|facility| {
                facility.world == world.id && facility.definition.as_str() == definition
            }) || ctx
                .state
                .construction_jobs
                .as_slice()
                .iter()
                .any(|job| job.world == world.id && job.definition.as_str() == definition)
        };
        let inputs = forecast
            .inventories
            .iter()
            .filter(|(_, target, _)| *target == world.id)
            .any(|(_, _, stock)| stock.energy >= 4 && stock.ore >= 4);
        let threatened = ctx.state.fleets.as_slice().iter().any(|fleet| fleet.owner != civ && at_war(ctx.state, civ, fleet.owner, ctx.tick)
            && matches!(fleet.location, LivingFleetLocationV2::Docked { world: target } if target == world.id)
            && fleet.hulls.as_slice().iter().any(|hull| hull.definition.as_str() == "escort"));
        let facilities: Vec<_> = ctx
            .state
            .facilities
            .as_slice()
            .iter()
            .filter(|facility| facility.world == world.id)
            .collect();
        let scrap = if facilities.len() < 6 {
            None
        } else {
            facilities
                .iter()
                .filter(|facility| {
                    matches!(
                        facility.definition.as_str(),
                        "solar_array" | "extractor" | "foundry"
                    ) && (facility.disabled || forecast.idle_facilities.contains(&facility.id))
                        && facility.next_repair.is_none()
                        && facilities.iter().any(|other| {
                            other.id != facility.id
                                && other.definition == facility.definition
                                && !other.disabled
                        })
                })
                .map(|facility| facility.id)
                .min()
        };
        let building_free = !ctx
            .state
            .construction_jobs
            .as_slice()
            .iter()
            .any(|job| job.world == world.id)
            && (facilities.len() < 6 || scrap.is_some());
        if building_free {
            for (definition, useful, base) in [
                (
                    "extractor",
                    !has("extractor")
                        && ctx.state.observations.as_slice().iter().any(|observation| {
                            observation.civilization == civ
                                && observation.world == world.id
                                && observation.remaining_deposit >= 4
                        }),
                    50,
                ),
                (
                    "foundry",
                    !has("foundry")
                        && inputs
                        && forecast.inventories.iter().any(|(_, target, stock)| {
                            *target == world.id
                                && stock.alloy <= LIVING_RESOURCE_STORAGE_LIMIT_V2 - 2
                        }),
                    50,
                ),
                (
                    "solar_array",
                    forecast
                        .deficits
                        .contains(&(world.id, LivingResourceV2::Energy)),
                    40,
                ),
                ("defense_battery", !has("defense_battery") && threatened, 55),
            ] {
                let Some(pack) = ctx.catalog.industry(definition) else {
                    return Err(LivingDeterministicFaultV2::Invariant);
                };
                if useful && world.inventory.alloy >= pack.alloy_cost {
                    let deficit = if definition == "solar_array" {
                        LivingResourceV2::Energy
                    } else if definition == "extractor" {
                        LivingResourceV2::Ore
                    } else {
                        LivingResourceV2::Alloy
                    };
                    let tier = if forecast.deficits.contains(&(world.id, deficit))
                        && definition != "defense_battery"
                    {
                        match deficit {
                            LivingResourceV2::Energy => 0,
                            LivingResourceV2::Ore => 1,
                            LivingResourceV2::Alloy => 2,
                        }
                    } else {
                        3
                    };
                    let bonus = if (policy == "industrialist"
                        && matches!(definition, "extractor" | "foundry"))
                        || (policy == "guardian" && definition == "defense_battery")
                    {
                        20
                    } else {
                        0
                    };
                    candidates.push(LivingEconomicCandidateV2 {
                        tier,
                        score: base + bonus,
                        target: world.id,
                        action: LivingEconomicActionV2::Construction {
                            world: world.id,
                            definition,
                            scrap,
                        },
                    });
                }
            }
        }
        for (definition, needed, base, bonus) in [
            (
                "colony_ark",
                hull_count("colony_ark") == 0
                    && ctx.state.observations.as_slice().iter().any(|observation| {
                        observation.civilization == civ
                            && observation.owner.is_none()
                            && travel(ctx.state, world.id, observation.world).is_some()
                    }),
                50,
                if policy == "expansionist" { 20 } else { 0 },
            ),
            (
                "escort",
                hull_count("escort") < colony_count * 2,
                50,
                if policy == "guardian" { 20 } else { 0 },
            ),
            ("scout", hull_count("scout") < 2, 40, 0),
        ] {
            if !needed {
                continue;
            }
            let pack = ctx
                .catalog
                .hull(definition)
                .ok_or(LivingDeterministicFaultV2::Invariant)?;
            let built_shipyard = ctx.state.facilities.as_slice().iter().any(|facility| {
                facility.world == world.id
                    && facility.definition.as_str() == "shipyard"
                    && !facility.disabled
            });
            if built_shipyard {
                if !ctx
                    .state
                    .hull_jobs
                    .as_slice()
                    .iter()
                    .any(|job| job.world == world.id)
                    && world.inventory.alloy >= pack.alloy_cost
                {
                    candidates.push(LivingEconomicCandidateV2 {
                        tier: 3,
                        score: base + bonus,
                        target: world.id,
                        action: LivingEconomicActionV2::Hull {
                            world: world.id,
                            definition,
                        },
                    });
                }
            } else if !has("shipyard")
                && building_free
                && world.inventory.alloy
                    >= ctx
                        .catalog
                        .industry("shipyard")
                        .ok_or(LivingDeterministicFaultV2::Invariant)?
                        .alloy_cost
            {
                candidates.push(LivingEconomicCandidateV2 {
                    tier: 3,
                    score: 55,
                    target: world.id,
                    action: LivingEconomicActionV2::Construction {
                        world: world.id,
                        definition: "shipyard",
                        scrap,
                    },
                });
            }
        }
        for resource in [
            LivingResourceV2::Energy,
            LivingResourceV2::Ore,
            LivingResourceV2::Alloy,
        ] {
            if inventory_amount(world.inventory, resource) >= 20
                || !forecast.deficits.contains(&(world.id, resource))
            {
                continue;
            }
            for source in ctx
                .state
                .worlds
                .as_slice()
                .iter()
                .filter(|source| source.owner == Some(civ) && source.id != world.id)
            {
                if inventory_amount(source.inventory, resource) < 30
                    || travel(ctx.state, source.id, world.id).is_none()
                {
                    continue;
                }
                let Some(existing_route) = supply_route_adjustment(
                    ctx.state,
                    civ,
                    source.id,
                    world.id,
                    resource,
                    LivingRouteKindV2::Internal,
                    civ,
                    10,
                ) else {
                    continue;
                };
                candidates.push(LivingEconomicCandidateV2 {
                    tier: match resource {
                        LivingResourceV2::Energy => 0,
                        LivingResourceV2::Ore => 1,
                        LivingResourceV2::Alloy => 2,
                    },
                    score: 70 + if policy == "trader" { 20 } else { 0 },
                    target: world.id,
                    action: LivingEconomicActionV2::Supply {
                        source: source.id,
                        destination: world.id,
                        resource,
                        kind: LivingRouteKindV2::Internal,
                        receiver: civ,
                        batch_size: 10,
                        existing_route,
                    },
                });
            }
        }
    }
    for request in requests.iter().filter(|request| request.receiver != civ) {
        let (a, b) = ordered_pair(civ, request.receiver);
        let trade = ctx.state.agreements.as_slice().iter().any(|agreement| {
            agreement.kind == LivingAgreementKindV2::Trade
                && agreement.participant_a == a
                && agreement.participant_b == b
                && active_interval(agreement.start_tick, agreement.end_tick, ctx.tick)
        });
        let aid = policy == "trader"
            && !at_war(ctx.state, civ, request.receiver, ctx.tick)
            && adjacent_to_owned_colony(ctx.state, civ, request.world);
        if !trade && !aid {
            continue;
        }
        let kind = if trade {
            LivingRouteKindV2::Trade
        } else {
            LivingRouteKindV2::Aid
        };
        let batch_size = 10.min(request.requested_units);
        for source in ctx
            .state
            .worlds
            .as_slice()
            .iter()
            .filter(|world| world.owner == Some(civ))
        {
            if inventory_amount(source.inventory, request.resource) < 20 + batch_size
                || travel(ctx.state, source.id, request.world).is_none()
            {
                continue;
            }
            let Some(existing_route) = supply_route_adjustment(
                ctx.state,
                civ,
                source.id,
                request.world,
                request.resource,
                kind,
                request.receiver,
                batch_size,
            ) else {
                continue;
            };
            candidates.push(LivingEconomicCandidateV2 {
                tier: 3,
                score: 70 + if policy == "trader" { 20 } else { 0 },
                target: request.world,
                action: LivingEconomicActionV2::Supply {
                    source: source.id,
                    destination: request.world,
                    resource: request.resource,
                    kind,
                    receiver: request.receiver,
                    batch_size,
                    existing_route,
                },
            });
        }
    }
    candidates.sort_by_key(|candidate| {
        let source = match candidate.action {
            LivingEconomicActionV2::Supply { source, .. } => source,
            _ => candidate.target,
        };
        (
            candidate.tie().0,
            candidate.target,
            source,
            candidate.tie().1,
        )
    });
    candidates.dedup_by(|right, left| right.action == left.action);
    let mut ordered: Vec<_> = candidates
        .into_iter()
        .enumerate()
        .map(|(ordinal, candidate)| {
            u16::try_from(ordinal)
                .map(|ordinal| (ordinal, candidate))
                .map_err(|_| LivingDeterministicFaultV2::Arithmetic)
        })
        .collect::<Result<_, _>>()?;
    ordered.sort_by_key(|(_, candidate)| {
        (
            candidate.tier,
            std::cmp::Reverse(candidate.score),
            candidate.target,
            candidate.tie(),
        )
    });
    Ok(ordered)
}

/// A working route already supplies this request. Otherwise adjust the first
/// stable matching route, or establish one when none exists. Recorded parties
/// may be stale after capture; only an explicit current-owner adjustment below
/// can refresh them and clear suspension.
#[allow(clippy::too_many_arguments)]
fn supply_route_adjustment(
    state: &LivingGalaxyStateV2,
    actor: LivingEntityIdV2,
    source: LivingEntityIdV2,
    destination: LivingEntityIdV2,
    resource: LivingResourceV2,
    kind: LivingRouteKindV2,
    receiver: LivingEntityIdV2,
    batch_size: u32,
) -> Option<Option<LivingEntityIdV2>> {
    let matches = |route: &&LivingFreightRouteV2| {
        route.source_world == source
            && route.destination_world == destination
            && route.resource == resource
    };
    let working = state.routes.as_slice().iter().filter(matches).any(|route| {
        !route.suspended
            && route.source_owner == actor
            && route.receiver_owner == receiver
            && route.kind == kind
            && route.batch_size >= batch_size
            && route.cadence_ticks == 10
            && route.source_reserve == 20
    });
    if working {
        return None;
    }
    Some(
        state
            .routes
            .as_slice()
            .iter()
            .find(matches)
            .map(|route| route.id),
    )
}

fn economic_decision(
    snapshot: &LivingGalaxyStateV2,
    actor: LivingEntityIdV2,
    ordinal: u16,
    candidate: &LivingEconomicCandidateV2,
    tick: LivingTickV2,
    snapshot_digest: super::ids::LivingStateDigestV2,
) -> Result<LivingIntentDecisionV2, LivingDeterministicFaultV2> {
    let action = match candidate.action {
        LivingEconomicActionV2::Construction {
            world,
            definition,
            scrap,
        } => LivingIntentActionV2::Construction {
            world,
            definition: slug(definition).map_err(|_| LivingDeterministicFaultV2::Invariant)?,
            scrap,
        },
        LivingEconomicActionV2::Hull { world, definition } => LivingIntentActionV2::Hull {
            world,
            definition: slug(definition).map_err(|_| LivingDeterministicFaultV2::Invariant)?,
        },
        LivingEconomicActionV2::Supply {
            source,
            destination,
            resource,
            existing_route,
            ..
        } => LivingIntentActionV2::Route {
            source,
            destination,
            resource,
            existing_route,
        },
    };
    let tier = match candidate.tier {
        0 => LivingDecisionTierV2::PowerEmergency,
        1 => LivingDecisionTierV2::OreEmergency,
        2 => LivingDecisionTierV2::AlloyEmergency,
        _ => LivingDecisionTierV2::Ordinary,
    };
    let (base, component, rule) = match candidate.action {
        LivingEconomicActionV2::Construction { definition, .. } => match definition {
            "solar_array" => (40, definition, "projected_power_deficit"),
            "extractor" => (50, definition, "observed_usable_deposit"),
            "foundry" => (50, definition, "projected_inputs_output_room"),
            "shipyard" => (55, definition, "missing_required_shipyard"),
            "defense_battery" => (55, definition, "threatened_owned_colony"),
            _ => return Err(LivingDeterministicFaultV2::Invariant),
        },
        LivingEconomicActionV2::Hull { definition, .. } => match definition {
            "colony_ark" => (50, definition, "eligible_settlement_target"),
            "escort" => (50, definition, "below_defense_target"),
            "scout" => (40, definition, "fewer_than_two_scouts"),
            _ => return Err(LivingDeterministicFaultV2::Invariant),
        },
        LivingEconomicActionV2::Supply { kind, .. } => (
            70,
            "supply",
            match kind {
                LivingRouteKindV2::Internal => "owned_needed_resource_deficit",
                LivingRouteKindV2::Trade => "receiver_demand_with_trade_permission",
                LivingRouteKindV2::Aid => "receiver_demand_nonhostile_neighbor_trader_aid",
            },
        ),
    };
    let mut reasons = vec![LivingDecisionReasonV2::Accepted {
        rule: slug(rule).map_err(|_| LivingDeterministicFaultV2::Invariant)?,
    }];
    if candidate.score > base {
        reasons.push(LivingDecisionReasonV2::PolicyScore {
            policy: snapshot
                .civilizations
                .as_slice()
                .iter()
                .find(|civilization| civilization.id == actor)
                .ok_or(LivingDeterministicFaultV2::Invariant)?
                .policy
                .0
                .clone(),
            component: slug(component).map_err(|_| LivingDeterministicFaultV2::Invariant)?,
            value: LivingDecisionScoreV2(i64::from(candidate.score - base)),
        });
    }
    Ok(LivingIntentDecisionV2 {
        phase: LivingPhaseV2::GenerateIntents,
        actor,
        ordinal,
        action,
        target: Some(candidate.target),
        score: LivingDecisionScoreV2(i64::from(candidate.score)),
        tier,
        snapshot_tick: tick,
        snapshot_digest,
        observed_tick: snapshot
            .observations
            .as_slice()
            .iter()
            .find(|observation| {
                observation.civilization == actor && observation.world == candidate.target
            })
            .map(|observation| observation.observed_tick),
        outcome: LivingDecisionOutcomeV2::Rejected,
        reasons,
    })
}

fn apply_economic_proposal(
    ctx: &LivingStepContextV2<'_>,
    civ: LivingEntityIdV2,
    _ordinal: u16,
    candidate: LivingEconomicCandidateV2,
) -> Result<LivingGalaxyStateV2, LivingDeterministicFaultV2> {
    let mut proposed = ctx.state.clone();
    let id = |kind: LivingEntityKindV2| {
        autonomous_entity_id_v2(&LivingAutonomousEntityInputsV2 {
            catalog_hash: ctx.catalog.catalog_hash(),
            genesis_seed: ctx.genesis_seed,
            branch: ctx.branch,
            tick: ctx.tick,
            phase: LivingPhaseV2::GenerateIntents.ordinal(),
            actor: civ,
            intent_ordinal: match candidate.action {
                LivingEconomicActionV2::Supply { .. } => 1,
                LivingEconomicActionV2::Construction { definition, .. } => match definition {
                    "extractor" => 2,
                    "foundry" => 3,
                    "solar_array" => 6,
                    "shipyard" => 8,
                    "defense_battery" => 9,
                    _ => 16,
                },
                LivingEconomicActionV2::Hull { definition, .. } => match definition {
                    "colony_ark" => 4,
                    "escort" => 5,
                    "scout" => 7,
                    _ => 16,
                },
            },
            entity_kind: kind.ordinal(),
            local_id: 0,
        })
    };
    match candidate.action {
        LivingEconomicActionV2::Construction {
            world,
            definition,
            scrap,
        } => {
            if let Some(scrap) = scrap {
                remove_key(&mut proposed.facilities, &scrap)?;
            }
            let pack = ctx
                .catalog
                .industry(definition)
                .ok_or(LivingDeterministicFaultV2::Invariant)?;
            proposed
                .worlds
                .find_mut(&world)
                .ok_or(LivingDeterministicFaultV2::Invariant)?
                .inventory
                .alloy -= pack.alloy_cost;
            put(
                &mut proposed.construction_jobs,
                LivingConstructionJobV2 {
                    id: id(LivingEntityKindV2::ConstructionJob),
                    world,
                    owner: civ,
                    definition: pack.id.clone(),
                    accepted_tick: ctx.tick,
                    completion_tick: add_ticks(ctx.tick, u64::from(pack.build_ticks))?,
                    paid_alloy: pack.alloy_cost,
                },
            )?;
        }
        LivingEconomicActionV2::Hull { world, definition } => {
            let pack = ctx
                .catalog
                .hull(definition)
                .ok_or(LivingDeterministicFaultV2::Invariant)?;
            proposed
                .worlds
                .find_mut(&world)
                .ok_or(LivingDeterministicFaultV2::Invariant)?
                .inventory
                .alloy -= pack.alloy_cost;
            put(
                &mut proposed.hull_jobs,
                LivingHullJobV2 {
                    id: id(LivingEntityKindV2::HullJob),
                    world,
                    owner: civ,
                    definition: pack.id.clone(),
                    accepted_tick: ctx.tick,
                    completion_tick: add_ticks(ctx.tick, u64::from(pack.build_ticks))?,
                    paid_alloy: pack.alloy_cost,
                    target_fleet: None,
                },
            )?;
        }
        LivingEconomicActionV2::Supply {
            source,
            destination,
            resource,
            kind,
            receiver,
            batch_size,
            existing_route,
        } => {
            let participants_match = owned_by(ctx.state, source, civ)
                && world_owner(ctx.state, destination) == Some(receiver)
                && travel(ctx.state, source, destination).is_some();
            let permission = match kind {
                LivingRouteKindV2::Internal => receiver == civ,
                LivingRouteKindV2::Trade => trade_open(ctx.state, civ, receiver, ctx.tick),
                LivingRouteKindV2::Aid => {
                    ctx.state
                        .civilizations
                        .as_slice()
                        .iter()
                        .any(|actor| actor.id == civ && actor.policy.0.as_str() == "trader")
                        && !at_war(ctx.state, civ, receiver, ctx.tick)
                        && adjacent_to_owned_colony(ctx.state, civ, destination)
                }
            };
            if !participants_match || !permission {
                return Err(LivingDeterministicFaultV2::Invariant);
            }
            let (route_id, next_due) = if let Some(route_id) = existing_route {
                let route = ctx
                    .state
                    .routes
                    .as_slice()
                    .iter()
                    .find(|route| route.id == route_id)
                    .filter(|route| {
                        route.source_world == source
                            && route.destination_world == destination
                            && route.resource == resource
                    })
                    .ok_or(LivingDeterministicFaultV2::Invariant)?;
                (route.id, route.next_due)
            } else {
                (id(LivingEntityKindV2::Route), add_ticks(ctx.tick, 10)?)
            };
            put(
                &mut proposed.routes,
                LivingFreightRouteV2 {
                    id: route_id,
                    kind,
                    source_world: source,
                    destination_world: destination,
                    resource,
                    batch_size,
                    cadence_ticks: 10,
                    source_reserve: 20,
                    source_owner: civ,
                    receiver_owner: receiver,
                    next_due,
                    suspended: false,
                },
            )?;
        }
    }
    Ok(proposed)
}

fn inventory_amount(stock: LivingInventoryV2, resource: LivingResourceV2) -> u32 {
    match resource {
        LivingResourceV2::Energy => stock.energy,
        LivingResourceV2::Ore => stock.ore,
        LivingResourceV2::Alloy => stock.alloy,
    }
}

fn reserve_settlement(
    ctx: &mut LivingStepContextV2<'_>,
    civ: LivingEntityIdV2,
    policy: &str,
) -> Result<(), LivingDeterministicFaultV2> {
    let _ = policy;
    let dormant = ctx
        .state
        .civilizations
        .as_slice()
        .iter()
        .any(|row| row.id == civ && row.status == LivingCivilizationStatusV2::Dormant);
    let candidate = ctx
        .state
        .observations
        .as_slice()
        .iter()
        .filter(|observation| observation.civilization == civ && observation.owner.is_none())
        .filter_map(|observation| {
            ctx.state
                .fleets
                .as_slice()
                .iter()
                .filter(|fleet| {
                    fleet.owner == civ
                        && fleet.order.is_none()
                        && fleet
                            .hulls
                            .as_slice()
                            .iter()
                            .any(|hull| hull.definition.as_str() == "colony_ark")
                        && !ctx
                            .state
                            .hull_jobs
                            .as_slice()
                            .iter()
                            .any(|job| job.target_fleet == Some(fleet.id))
                })
                .filter_map(|fleet| {
                    let LivingFleetLocationV2::Docked { world: origin } = fleet.location else {
                        return None;
                    };
                    if origin != observation.world
                        && travel(ctx.state, origin, observation.world).is_none()
                    {
                        return None;
                    }
                    let permitted = if dormant {
                        fleet.hulls.as_slice().iter().all(|hull| hull.return_credit)
                    } else {
                        let required = fleet
                            .hulls
                            .as_slice()
                            .iter()
                            .map(|hull| if hull.return_credit { 2u32 } else { 4u32 })
                            .sum::<u32>();
                        owned_by(ctx.state, origin, civ)
                            && world_quantity(ctx.state, origin, LivingResourceV2::Energy)
                                >= required
                    };
                    permitted.then_some((observation.world, fleet.id))
                })
                .min()
        })
        .min();
    let Some((target, fleet_id)) = candidate else {
        return Ok(());
    };
    let Some(fleet) = ctx.state.fleets.find_mut(&fleet_id) else {
        return Ok(());
    };
    fleet.order = Some(LivingFleetOrderV2 {
        kind: LivingFleetOrderKindV2::Settle,
        target_world: Some(target),
        not_before_tick: add_ticks(ctx.tick, 1)?,
        lane_path: Vec::new(),
    });
    Ok(())
}

fn fleet_priority(
    state: &LivingGalaxyStateV2,
    tick: LivingTickV2,
    decision: &LivingIntentDecisionV2,
) -> u8 {
    if matches!(decision.action, LivingIntentActionV2::Defend { .. })
        && decision
            .target
            .is_some_and(|world| attacked_owned_world(state, decision.actor, world, tick))
    {
        0
    } else if matches!(decision.action, LivingIntentActionV2::Return { .. }) {
        1
    } else {
        2
    }
}

fn attacked_owned_world(
    state: &LivingGalaxyStateV2,
    civ: LivingEntityIdV2,
    world: LivingEntityIdV2,
    tick: LivingTickV2,
) -> bool {
    owned_by(state, world, civ) && state.fleets.as_slice().iter().any(|fleet| {
        fleet.owner != civ
            && at_war(state, civ, fleet.owner, tick)
            && matches!(fleet.location, LivingFleetLocationV2::Docked { world: at } if at == world)
            && fleet
                .hulls
                .as_slice()
                .iter()
                .any(|hull| hull.definition.as_str() == "escort")
    })
}

fn reserve_guard(
    ctx: &mut LivingStepContextV2<'_>,
    civ: LivingEntityIdV2,
    emergency: bool,
) -> Result<(), LivingDeterministicFaultV2> {
    if !ctx
        .state
        .civilizations
        .as_slice()
        .iter()
        .any(|row| row.id == civ && row.status == LivingCivilizationStatusV2::Active)
    {
        return Ok(());
    }
    let target = ctx.state.colonies.as_slice().iter().filter(|row| row.owner == civ)
        .filter(|row| if emergency { attacked_owned_world(ctx.state, civ, row.world, ctx.tick) }
            else { !ctx.state.fleets.as_slice().iter().any(|fleet| fleet.owner == civ
                && matches!(fleet.location, LivingFleetLocationV2::Docked { world } if world == row.world)
                && fleet.hulls.as_slice().iter().any(|hull| hull.definition.as_str() == "escort")) })
        .filter_map(|row| ctx.state.fleets.as_slice().iter().filter(|fleet| fleet.owner == civ
            && fleet.order.is_none()
            && fleet.hulls.as_slice().iter().any(|hull| hull.definition.as_str() == "escort")
            && !ctx.state.hull_jobs.as_slice().iter().any(|job| job.target_fleet == Some(fleet.id)))
            .filter_map(|fleet| match fleet.location {
                LivingFleetLocationV2::Docked { world } if world != row.world && owned_by(ctx.state, world, civ)
                    && travel(ctx.state, world, row.world).is_some()
                            && world_quantity(ctx.state, world, LivingResourceV2::Energy)
                                >= role_launch_cost(ctx.state, fleet, "escort") => Some((row.world, fleet.id)),
                _ => None,
            }).min()).min();
    let Some((world, fleet)) = target else {
        return Ok(());
    };
    let fleet = ctx
        .state
        .fleets
        .as_slice()
        .iter()
        .find(|row| row.id == fleet)
        .ok_or(LivingDeterministicFaultV2::Invariant)?
        .clone();
    reserve_role_order(
        ctx,
        fleet,
        world,
        LivingFleetOrderKindV2::Defend,
        "escort",
        if emergency { 10 } else { 15 },
    )
}

fn role_hulls<'a>(
    state: &LivingGalaxyStateV2,
    fleet: &'a LivingFleetV2,
    definition: &str,
) -> Vec<&'a LivingHullV2> {
    let role: Vec<_> = fleet
        .hulls
        .as_slice()
        .iter()
        .filter(|hull| hull.definition.as_str() == definition)
        .collect();
    let reserved_fleets = state.fleets.as_slice().len()
        + state
            .hull_jobs
            .as_slice()
            .iter()
            .filter(|job| job.target_fleet.is_none())
            .count();
    if reserved_fleets >= super::model::LIVING_MAX_FLEETS_V2 {
        fleet.hulls.as_slice().iter().collect()
    } else {
        role
    }
}

fn role_launch_cost(state: &LivingGalaxyStateV2, fleet: &LivingFleetV2, definition: &str) -> u32 {
    role_hulls(state, fleet, definition)
        .iter()
        .map(|hull| if hull.return_credit { 2u32 } else { 4u32 })
        .sum()
}

fn reserve_role_order(
    ctx: &mut LivingStepContextV2<'_>,
    fleet: LivingFleetV2,
    target: LivingEntityIdV2,
    kind: LivingFleetOrderKindV2,
    definition: &str,
    intent_ordinal: u16,
) -> Result<(), LivingDeterministicFaultV2> {
    let selected: Vec<_> = role_hulls(ctx.state, &fleet, definition)
        .into_iter()
        .cloned()
        .collect();
    let order = LivingFleetOrderV2 {
        kind,
        target_world: Some(target),
        not_before_tick: add_ticks(ctx.tick, 1)?,
        lane_path: Vec::new(),
    };
    if selected.len() == fleet.hulls.as_slice().len() {
        let mut fleet = fleet;
        fleet.order = Some(order);
        put(&mut ctx.state.fleets, fleet)?;
        return Ok(());
    }
    let id = autonomous_id(
        ctx,
        fleet.owner,
        LivingPhaseV2::GenerateIntents,
        intent_ordinal,
        LivingEntityKindV2::Fleet,
        0,
    )?;
    let mut retained = fleet.clone();
    for hull in &selected {
        remove_key(&mut retained.hulls, &hull.id)?;
    }
    put(&mut ctx.state.fleets, retained)?;
    put(
        &mut ctx.state.fleets,
        LivingFleetV2 {
            id,
            owner: fleet.owner,
            location: fleet.location,
            order: Some(order),
            hulls: LivingSortedVecV2::new(selected).map_err(LivingValidationErrorV2::from)?,
        },
    )?;
    Ok(())
}

fn reserve_fleet(
    ctx: &mut LivingStepContextV2<'_>,
    civ: LivingEntityIdV2,
) -> Result<(), LivingDeterministicFaultV2> {
    if !ctx
        .state
        .civilizations
        .as_slice()
        .iter()
        .any(|row| row.id == civ && row.status == LivingCivilizationStatusV2::Active)
    {
        return Ok(());
    }
    let candidate = ctx
        .state
        .observations
        .as_slice()
        .iter()
        .filter(|observation| {
            observation.civilization == civ
                && observation
                    .owner
                    .is_some_and(|opponent| at_war(ctx.state, civ, opponent, ctx.tick))
                && ctx.tick.0.saturating_sub(observation.observed_tick.0) <= OBSERVATION_FRESH_TICKS
        })
        .filter_map(|observation| {
            ctx.state
                .fleets
                .as_slice()
                .iter()
                .filter(|fleet| {
                    fleet.owner == civ
                        && fleet.order.is_none()
                        && fleet
                            .hulls
                            .as_slice()
                            .iter()
                            .any(|hull| hull.definition.as_str() == "escort")
                        && !ctx
                            .state
                            .hull_jobs
                            .as_slice()
                            .iter()
                            .any(|job| job.target_fleet == Some(fleet.id))
                })
                .filter_map(|fleet| {
                    let LivingFleetLocationV2::Docked { world } = fleet.location else {
                        return None;
                    };
                    (owned_by(ctx.state, world, civ)
                        && world != observation.world
                        && travel(ctx.state, world, observation.world).is_some()
                        && world_quantity(ctx.state, world, LivingResourceV2::Energy)
                            >= role_launch_cost(ctx.state, fleet, "escort"))
                    .then_some((observation.world, fleet.id))
                })
                .min()
        })
        .min();
    let Some((target, fleet_id)) = candidate else {
        return reserve_exploration(ctx, civ);
    };
    let fleet = ctx
        .state
        .fleets
        .as_slice()
        .iter()
        .find(|fleet| fleet.id == fleet_id)
        .ok_or(LivingDeterministicFaultV2::Invariant)?
        .clone();
    reserve_role_order(
        ctx,
        fleet,
        target,
        LivingFleetOrderKindV2::Attack,
        "escort",
        13,
    )
}

fn war_target(
    ctx: &LivingStepContextV2<'_>,
    civ: LivingEntityIdV2,
) -> Option<(LivingEntityIdV2, LivingEntityIdV2)> {
    let mut best: Option<(i32, LivingEntityIdV2, LivingEntityIdV2)> = None;
    for opponent in ctx.state.civilizations.as_slice() {
        if opponent.id == civ || opponent.status != LivingCivilizationStatusV2::Active {
            continue;
        }
        if pair_blocked(ctx.state, civ, opponent.id, ctx.tick) {
            continue;
        }
        let relation = relation_total(ctx.state, civ, opponent.id);
        if relation > -20 {
            continue;
        }
        for observation in ctx
            .state
            .observations
            .as_slice()
            .iter()
            .filter(|observation| {
                observation.civilization == civ
                    && observation.owner == Some(opponent.id)
                    && ctx.tick.0.saturating_sub(observation.observed_tick.0)
                        <= OBSERVATION_FRESH_TICKS
            })
        {
            let world = observation.world;
            if !adjacent_to_owned_colony(ctx.state, civ, world) {
                continue;
            }
            let attack = assembled_attack(ctx.state, civ, world);
            if attack < ESCORT_STRENGTH {
                continue;
            }
            let defense = observed_defense(ctx.state, civ, world);
            if attack.saturating_mul(2) < defense.saturating_mul(3) {
                continue;
            }
            let candidate = (relation, world, opponent.id);
            if best.is_none_or(|current| candidate < current) {
                best = Some(candidate);
            }
        }
    }
    best.map(|(_, world, opponent)| (opponent, world))
}

fn declare_war(
    ctx: &mut LivingStepContextV2<'_>,
    civ: LivingEntityIdV2,
    opponent: LivingEntityIdV2,
    _world: LivingEntityIdV2,
) -> Result<(), LivingDeterministicFaultV2> {
    let (participant_a, participant_b) = ordered_pair(civ, opponent);
    if ctx.state.wars.as_slice().iter().any(|war| {
        war.participant_a == participant_a
            && war.participant_b == participant_b
            && active_interval(war.start_tick, war.end_tick, ctx.tick)
    }) {
        return Ok(());
    }
    remove_pair_agreements(ctx.state, participant_a, participant_b)?;
    put(
        &mut ctx.state.wars,
        LivingWarV2 {
            participant_a,
            participant_b,
            declarer: civ,
            start_tick: ctx.tick,
            end_tick: add_ticks(ctx.tick, WAR_DURATION_TICKS)?,
        },
    )?;
    let mut reasons = relation_reasons(ctx.state, opponent, civ);
    reasons.war_declared_against_it = (reasons.war_declared_against_it - 30).max(-30);
    let base = ctx
        .state
        .relations
        .as_slice()
        .iter()
        .find(|relation| relation.from == opponent && relation.to == civ)
        .map(|relation| relation.base_disposition)
        .unwrap_or(0);
    put(
        &mut ctx.state.relations,
        LivingRelationV2 {
            from: opponent,
            to: civ,
            base_disposition: base,
            reasons,
        },
    )?;
    emit(
        ctx,
        LivingPhaseV2::UpdateDiplomacy,
        LivingEventKindV2::WarDeclared,
    )?;
    Ok(())
}

fn dispatch_fleets(ctx: &mut LivingStepContextV2<'_>) -> Result<(), LivingDeterministicFaultV2> {
    let fleet_ids: Vec<_> = ctx
        .state
        .fleets
        .as_slice()
        .iter()
        .map(|fleet| fleet.id)
        .collect();
    for fleet_id in fleet_ids {
        if ctx
            .state
            .hull_jobs
            .as_slice()
            .iter()
            .any(|job| job.target_fleet == Some(fleet_id))
        {
            continue;
        }
        let Some(fleet) = ctx
            .state
            .fleets
            .as_slice()
            .iter()
            .find(|fleet| fleet.id == fleet_id)
        else {
            continue;
        };
        let LivingFleetLocationV2::Docked { world: origin } = fleet.location else {
            continue;
        };
        let Some(order) = fleet.order.clone() else {
            continue;
        };
        if order.not_before_tick.0 > ctx.tick.0 {
            continue;
        }
        let Some(destination) = order.target_world else {
            continue;
        };
        if destination == origin {
            continue;
        }
        let Some((duration, path)) = travel(ctx.state, origin, destination) else {
            continue;
        };
        let owner = fleet.owner;
        let hull_count = u32::try_from(fleet.hulls.as_slice().len()).unwrap_or(u32::MAX);
        let missing_credit = u32::try_from(
            fleet
                .hulls
                .as_slice()
                .iter()
                .filter(|hull| !hull.return_credit)
                .count(),
        )
        .unwrap_or(u32::MAX);
        let recovery = order.kind == LivingFleetOrderKindV2::Settle
            && ctx
                .state
                .civilizations
                .as_slice()
                .iter()
                .any(|civilization| {
                    civilization.id == owner
                        && civilization.status == LivingCivilizationStatusV2::Dormant
                })
            && world_owner(ctx.state, destination).is_none()
            && ctx.state.observations.as_slice().iter().any(|observation| {
                observation.civilization == owner && observation.world == destination
            })
            && fleet
                .hulls
                .as_slice()
                .iter()
                .any(|hull| hull.definition.as_str() == "colony_ark");
        let returning = order.kind == LivingFleetOrderKindV2::Return;
        let consumes_credit = returning || (!owned_by(ctx.state, origin, owner) && recovery);
        if consumes_credit {
            if returning && !owned_by(ctx.state, destination, owner) {
                continue;
            }
            if missing_credit != 0 {
                continue;
            }
        } else if owned_by(ctx.state, origin, owner) {
            let cost = hull_count.saturating_mul(LAUNCH_ENERGY_PER_HULL)
                + missing_credit.saturating_mul(LAUNCH_ENERGY_PER_HULL);
            let Some(world) = ctx.state.worlds.find_mut(&origin) else {
                continue;
            };
            if world.inventory.energy < cost {
                continue;
            }
            world.inventory.energy -= cost;
        } else {
            // Prepaid credits authorize a return, never a fresh excursion.
            continue;
        }
        let hull_ids: Vec<_> = ctx
            .state
            .fleets
            .as_slice()
            .iter()
            .find(|fleet| fleet.id == fleet_id)
            .map(|fleet| fleet.hulls.as_slice().iter().map(|hull| hull.id).collect())
            .unwrap_or_default();
        let grant_credit = !returning && owned_by(ctx.state, origin, owner);
        for hull_id in hull_ids {
            let Some(fleet) = ctx.state.fleets.find_mut(&fleet_id) else {
                continue;
            };
            if let Some(hull) = fleet.hulls.find_mut(&hull_id) {
                hull.return_credit = if consumes_credit {
                    false
                } else {
                    grant_credit || hull.return_credit
                };
            }
        }
        let Some(fleet) = ctx.state.fleets.find_mut(&fleet_id) else {
            continue;
        };
        if let Some(order) = fleet.order.as_mut() {
            order.lane_path = path;
        }
        fleet.location = LivingFleetLocationV2::Travelling {
            origin_world: origin,
            destination_world: destination,
            departure_tick: ctx.tick,
            arrival_tick: add_ticks(ctx.tick, duration)?,
        };
        emit(
            ctx,
            LivingPhaseV2::DispatchOrders,
            LivingEventKindV2::FleetDeparted,
        )?;
    }
    Ok(())
}

fn dispatch_routes(ctx: &mut LivingStepContextV2<'_>) -> Result<(), LivingDeterministicFaultV2> {
    let route_ids: Vec<_> = ctx
        .state
        .routes
        .as_slice()
        .iter()
        .map(|route| route.id)
        .collect();
    for route_id in route_ids {
        let Some(route) = ctx
            .state
            .routes
            .as_slice()
            .iter()
            .find(|route| route.id == route_id)
            .cloned()
        else {
            continue;
        };
        if route.next_due.0 > ctx.tick.0 {
            continue;
        }
        let source_owner_now = world_owner(ctx.state, route.source_world);
        let destination_owner_now = world_owner(ctx.state, route.destination_world);
        let participants_match = source_owner_now == Some(route.source_owner)
            && destination_owner_now == Some(route.receiver_owner);
        let permitted = !route.suspended
            && travel(ctx.state, route.source_world, route.destination_world).is_some()
            && participants_match
            && match route.kind {
                LivingRouteKindV2::Internal => route.source_owner == route.receiver_owner,
                LivingRouteKindV2::Aid => true,
                LivingRouteKindV2::Trade => trade_open(
                    ctx.state,
                    route.source_owner,
                    route.receiver_owner,
                    ctx.tick,
                ),
            };
        if let Some(stored) = ctx.state.routes.find_mut(&route_id) {
            stored.suspended = route.suspended || !participants_match;
            stored.next_due = add_ticks(route.next_due, u64::from(route.cadence_ticks))?;
        }
        if !permitted {
            continue;
        }
        let available = world_quantity(ctx.state, route.source_world, route.resource);
        let above = available.saturating_sub(route.source_reserve);
        let eligible = route.batch_size.min(above);
        let (batch, hazard) = hazard_batch_details(ctx, &route, eligible);
        if ctx.state.shipments.as_slice().len() >= super::model::LIVING_MAX_SHIPMENTS_V2 {
            continue;
        }
        if batch == 0 {
            record_corridor_restriction(ctx, &route, eligible, batch, 0, hazard)?;
            continue;
        }
        if !spend(ctx.state, route.source_world, route.resource, batch) {
            continue;
        }
        let duration = travel(ctx.state, route.source_world, route.destination_world)
            .map(|(duration, _)| duration)
            .unwrap_or(LOCAL_TRAVEL_TICKS);
        let id = autonomous_id(
            ctx,
            route.id,
            LivingPhaseV2::DispatchOrders,
            1,
            LivingEntityKindV2::Shipment,
            0,
        )?;
        put(
            &mut ctx.state.shipments,
            LivingShipmentV2 {
                id,
                dispatch_owner: route.source_owner,
                intended_receiver: route.receiver_owner,
                source_world: route.source_world,
                destination_world: route.destination_world,
                resource: route.resource,
                units: batch,
                departure_tick: ctx.tick,
                arrival_tick: add_ticks(ctx.tick, duration)?,
                disposition: LivingShipmentDispositionV2::Outbound,
                dispatch_kind: Some(route.kind),
                returning_to_source: false,
            },
        )?;
        emit(
            ctx,
            LivingPhaseV2::DispatchOrders,
            LivingEventKindV2::ShipmentDispatched,
        )?;
        record_corridor_restriction(ctx, &route, eligible, batch, batch, hazard)?;
    }
    Ok(())
}

/// Credit only immutable dispatch evidence after an actual inventory transfer.
/// Legacy manifests and same-owner transfers provide no cross-party evidence.
#[cfg(test)]
fn credit_delivery_relation(
    state: &mut LivingGalaxyStateV2,
    shipment: &LivingShipmentV2,
) -> Result<(), LivingDeterministicFaultV2> {
    if shipment.dispatch_owner == shipment.intended_receiver {
        return Ok(());
    }
    let Some(kind @ (LivingRouteKindV2::Aid | LivingRouteKindV2::Trade)) = shipment.dispatch_kind
    else {
        return Ok(());
    };
    let from = shipment.intended_receiver;
    let to = shipment.dispatch_owner;
    let mut relation = state
        .relations
        .as_slice()
        .iter()
        .find(|row| row.from == from && row.to == to)
        .copied()
        .unwrap_or(LivingRelationV2 {
            from,
            to,
            base_disposition: 0,
            reasons: LivingRelationReasonsV2::default(),
        });
    apply_delivery_credit(&mut relation, kind, shipment.units)?;
    put(&mut state.relations, relation)?;
    Ok(())
}

pub(crate) fn apply_delivery_credit(
    relation: &mut LivingRelationV2,
    kind: LivingRouteKindV2,
    units: u32,
) -> Result<(), LivingDeterministicFaultV2> {
    let (counter, remainder, increment) = match kind {
        LivingRouteKindV2::Aid => (
            &mut relation.reasons.delivered_aid,
            &mut relation.reasons.aid_units_remainder,
            10,
        ),
        LivingRouteKindV2::Trade => (
            &mut relation.reasons.successful_trade,
            &mut relation.reasons.trade_units_remainder,
            1,
        ),
        LivingRouteKindV2::Internal => return Ok(()),
    };
    let total = u64::from(*remainder) + u64::from(units);
    let credited =
        i32::try_from(total / 100).map_err(|_| LivingDeterministicFaultV2::Arithmetic)?;
    *counter = counter
        .saturating_add(credited.saturating_mul(increment))
        .min(20);
    *remainder = (total % 100) as u32;
    Ok(())
}

fn retreat_noncombatants(
    ctx: &mut LivingStepContextV2<'_>,
) -> Result<(), LivingDeterministicFaultV2> {
    let fleet_ids: Vec<_> = ctx
        .state
        .fleets
        .as_slice()
        .iter()
        .map(|fleet| fleet.id)
        .collect();
    for fleet_id in fleet_ids {
        let Some(fleet) = ctx
            .state
            .fleets
            .as_slice()
            .iter()
            .find(|fleet| fleet.id == fleet_id)
        else {
            continue;
        };
        let LivingFleetLocationV2::Docked { world } = fleet.location else {
            continue;
        };
        let Some(owner) = world_owner(ctx.state, world) else {
            continue;
        };
        if owner == fleet.owner || !at_war(ctx.state, fleet.owner, owner, ctx.tick) {
            continue;
        }
        let damaging = fleet
            .hulls
            .as_slice()
            .iter()
            .any(|hull| hull_damage(ctx.catalog, hull) > 0);
        if damaging {
            continue;
        }
        if fleet
            .hulls
            .as_slice()
            .iter()
            .any(|hull| !hull.return_credit)
            || fleet.hulls.as_slice().is_empty()
        {
            continue;
        }
        let Some(destination) = nearest_owned_colony(ctx.state, fleet.owner, world) else {
            continue;
        };
        let Some((duration, path)) = travel(ctx.state, world, destination) else {
            continue;
        };
        let hull_ids: Vec<_> = ctx
            .state
            .fleets
            .as_slice()
            .iter()
            .find(|fleet| fleet.id == fleet_id)
            .map(|fleet| fleet.hulls.as_slice().iter().map(|hull| hull.id).collect())
            .unwrap_or_default();
        for hull_id in hull_ids {
            if let Some(fleet) = ctx.state.fleets.find_mut(&fleet_id)
                && let Some(hull) = fleet.hulls.find_mut(&hull_id)
            {
                hull.return_credit = false;
            }
        }
        let Some(fleet) = ctx.state.fleets.find_mut(&fleet_id) else {
            continue;
        };
        fleet.order = Some(LivingFleetOrderV2 {
            kind: LivingFleetOrderKindV2::Return,
            target_world: Some(destination),
            not_before_tick: ctx.tick,
            lane_path: path,
        });
        fleet.location = LivingFleetLocationV2::Travelling {
            origin_world: world,
            destination_world: destination,
            departure_tick: ctx.tick,
            arrival_tick: add_ticks(ctx.tick, duration)?,
        };
        ctx.repair_exclusions.insert(world);
        emit(
            ctx,
            LivingPhaseV2::ResolveConflict,
            LivingEventKindV2::FleetDeparted,
        )?;
    }
    Ok(())
}

fn resolve_combat(ctx: &mut LivingStepContextV2<'_>) -> Result<(), LivingDeterministicFaultV2> {
    #[derive(Clone, Copy)]
    struct Piece {
        id: LivingEntityIdV2,
        fleet: Option<LivingEntityIdV2>,
        owner: LivingEntityIdV2,
        world: LivingEntityIdV2,
        damage: u32,
        class: u8,
    }
    let mut pieces = Vec::new();
    for fleet in ctx.state.fleets.as_slice() {
        let LivingFleetLocationV2::Docked { world } = fleet.location else {
            continue;
        };
        for hull in fleet.hulls.as_slice() {
            let damage = hull_damage(ctx.catalog, hull);
            let class = if hull.definition.as_str() == "escort" {
                0
            } else if damage > 0 {
                1
            } else {
                2
            };
            pieces.push(Piece {
                id: hull.id,
                fleet: Some(fleet.id),
                owner: fleet.owner,
                world,
                damage,
                class,
            });
        }
    }
    for facility in ctx.state.facilities.as_slice() {
        let Some(definition) = ctx.catalog.industry(facility.definition.as_str()) else {
            continue;
        };
        let LivingIndustryEffectV2::Defense {
            damage_per_round, ..
        } = definition.effect
        else {
            continue;
        };
        if facility.hit_points == 0 {
            continue;
        }
        let Some(owner) = world_owner(ctx.state, facility.world) else {
            continue;
        };
        pieces.push(Piece {
            id: facility.id,
            fleet: None,
            owner,
            world: facility.world,
            damage: if facility.disabled {
                0
            } else {
                damage_per_round
            },
            class: 1,
        });
    }
    let mut harm: Vec<(LivingEntityIdV2, u32, bool)> = Vec::new();
    let mut any = false;
    for attacker in &pieces {
        if attacker.damage == 0 {
            continue;
        }
        let mut hostiles: Vec<_> = pieces
            .iter()
            .filter(|target| {
                target.world == attacker.world
                    && target.owner != attacker.owner
                    && at_war(ctx.state, attacker.owner, target.owner, ctx.tick)
            })
            .copied()
            .collect();
        if hostiles.is_empty() {
            continue;
        }
        hostiles.sort_by(|left, right| left.class.cmp(&right.class).then(left.id.cmp(&right.id)));
        let target = hostiles[0];
        ctx.repair_exclusions.insert(attacker.world);
        any = true;
        if let Some(entry) = harm.iter_mut().find(|entry| entry.0 == target.id) {
            entry.1 = entry.1.saturating_add(attacker.damage);
        } else {
            harm.push((target.id, attacker.damage, target.fleet.is_none()));
        }
    }
    if any {
        emit(
            ctx,
            LivingPhaseV2::ResolveConflict,
            LivingEventKindV2::CombatRound,
        )?;
    }
    for (id, damage, battery) in harm {
        if battery {
            if let Some(facility) = ctx.state.facilities.find_mut(&id) {
                facility.hit_points = facility.hit_points.saturating_sub(damage);
                if facility.hit_points == 0 {
                    // Removed after the loop so the sorted collection stays valid.
                } else if facility.next_repair.is_none() {
                    facility.next_repair = Some(add_ticks(ctx.tick, REPAIR_PERIOD_TICKS)?);
                }
            }
        } else if let Some((fleet_id, hull_hp)) = fleet_hull_hp(ctx.state, id) {
            let next = hull_hp.saturating_sub(damage);
            if let Some(fleet) = ctx.state.fleets.find_mut(&fleet_id)
                && let Some(hull) = fleet.hulls.find_mut(&id)
            {
                hull.hit_points = next;
                if next > 0 && hull.next_repair.is_none() {
                    hull.next_repair = Some(add_ticks(ctx.tick, REPAIR_PERIOD_TICKS)?);
                }
            }
        }
    }
    let dead_batteries: Vec<_> = ctx
        .state
        .facilities
        .as_slice()
        .iter()
        .filter(|facility| {
            matches!(
                ctx.catalog
                    .industry(facility.definition.as_str())
                    .map(|definition| &definition.effect),
                Some(LivingIndustryEffectV2::Defense { .. })
            ) && facility.hit_points == 0
        })
        .map(|facility| facility.id)
        .collect();
    for id in dead_batteries {
        remove_key(&mut ctx.state.facilities, &id)?;
        emit(
            ctx,
            LivingPhaseV2::ResolveConflict,
            LivingEventKindV2::HullDestroyed,
        )?;
    }
    let fleet_ids: Vec<_> = ctx
        .state
        .fleets
        .as_slice()
        .iter()
        .map(|fleet| fleet.id)
        .collect();
    for fleet_id in fleet_ids {
        let dead: Vec<_> = ctx
            .state
            .fleets
            .as_slice()
            .iter()
            .find(|fleet| fleet.id == fleet_id)
            .map(|fleet| {
                fleet
                    .hulls
                    .as_slice()
                    .iter()
                    .filter(|hull| hull.hit_points == 0)
                    .map(|hull| hull.id)
                    .collect()
            })
            .unwrap_or_default();
        for hull_id in dead {
            if let Some(fleet) = ctx.state.fleets.find_mut(&fleet_id) {
                remove_key(&mut fleet.hulls, &hull_id)?;
            }
            emit(
                ctx,
                LivingPhaseV2::ResolveConflict,
                LivingEventKindV2::HullDestroyed,
            )?;
        }
    }
    Ok(())
}

fn resolve_occupation(ctx: &mut LivingStepContextV2<'_>) -> Result<(), LivingDeterministicFaultV2> {
    let colonies: Vec<_> = ctx
        .state
        .colonies
        .as_slice()
        .iter()
        .map(|colony| (colony.world, colony.owner))
        .collect();
    for (world, owner) in colonies {
        let defenders = defense_present(ctx, world, owner);
        let mut claimants: Vec<_> = escort_owners(ctx.state, world)
            .into_iter()
            .filter(|claimant| *claimant != owner && at_war(ctx.state, *claimant, owner, ctx.tick))
            .collect();
        claimants.sort();
        claimants.dedup();
        let contested = claimants.len() != 1
            || defenders
            || claimants.iter().any(|claimant| {
                escort_owners(ctx.state, world).into_iter().any(|other| {
                    other != *claimant
                        && other != owner
                        && at_war(ctx.state, other, *claimant, ctx.tick)
                })
            });
        if contested {
            remove_key(&mut ctx.state.occupations, &world)?;
            continue;
        }
        let claimant = claimants[0];
        let progress = ctx
            .state
            .occupations
            .as_slice()
            .iter()
            .find(|occupation| occupation.world == world && occupation.claimant == claimant)
            .map(|occupation| occupation.progress_ticks)
            .unwrap_or(0)
            .saturating_add(1);
        if progress >= LIVING_OCCUPATION_TRANSFER_TICKS_V2 {
            transfer_colony(ctx, world, owner, claimant)?;
            remove_key(&mut ctx.state.occupations, &world)?;
        } else {
            put(
                &mut ctx.state.occupations,
                LivingOccupationV2 {
                    world,
                    claimant,
                    progress_ticks: progress,
                },
            )?;
        }
    }
    Ok(())
}

fn resolve_claims(ctx: &mut LivingStepContextV2<'_>) -> Result<(), LivingDeterministicFaultV2> {
    let worlds: Vec<_> = ctx
        .state
        .worlds
        .as_slice()
        .iter()
        .map(|world| world.id)
        .collect();
    for world in worlds {
        if armed_presence(ctx, world) {
            clear_claims(ctx, world)?;
            continue;
        }
        if world_owner(ctx.state, world).is_some() {
            clear_claims(ctx, world)?;
            continue;
        }
        let close = ctx
            .state
            .settlement_claims
            .as_slice()
            .iter()
            .find(|claim| claim.world == world)
            .map(|claim| claim.close_tick);
        if let Some(close) = close
            && ctx.tick == close
        {
            arbitrate_claim(ctx, world, close)?;
        }
        if world_owner(ctx.state, world).is_some() {
            continue;
        }
        let close_now = ctx
            .state
            .settlement_claims
            .as_slice()
            .iter()
            .find(|claim| claim.world == world)
            .map(|claim| claim.close_tick);
        for fleet_id in ctx.arrivals.clone() {
            let Some(fleet) = ctx
                .state
                .fleets
                .as_slice()
                .iter()
                .find(|fleet| fleet.id == fleet_id)
            else {
                continue;
            };
            let LivingFleetLocationV2::Docked { world: docked } = fleet.location else {
                continue;
            };
            if docked != world {
                continue;
            }
            let settling = fleet
                .order
                .as_ref()
                .is_some_and(|order| order.kind == LivingFleetOrderKindV2::Settle)
                && fleet
                    .hulls
                    .as_slice()
                    .iter()
                    .any(|hull| hull.definition.as_str() == "colony_ark");
            if !settling {
                continue;
            }
            match close_now {
                Some(close) if ctx.tick.0 < close.0 => {
                    let exists = ctx
                        .state
                        .settlement_claims
                        .as_slice()
                        .iter()
                        .any(|claim| claim.world == world && claim.civilization == fleet.owner);
                    if !exists {
                        put(
                            &mut ctx.state.settlement_claims,
                            LivingSettlementClaimV2 {
                                world,
                                civilization: fleet.owner,
                                arrival_tick: ctx.tick,
                                close_tick: close,
                            },
                        )?;
                    }
                }
                None => {
                    let close_tick = add_ticks(ctx.tick, CLAIM_WINDOW_TICKS)?;
                    put(
                        &mut ctx.state.settlement_claims,
                        LivingSettlementClaimV2 {
                            world,
                            civilization: fleet.owner,
                            arrival_tick: ctx.tick,
                            close_tick,
                        },
                    )?;
                    emit(
                        ctx,
                        LivingPhaseV2::ResolveConflict,
                        LivingEventKindV2::SettlementClaimOpened,
                    )?;
                }
                Some(_) => {}
            }
        }
    }
    Ok(())
}

fn arbitrate_claim(
    ctx: &mut LivingStepContextV2<'_>,
    world: LivingEntityIdV2,
    close: LivingTickV2,
) -> Result<(), LivingDeterministicFaultV2> {
    let mut present: Vec<_> = ctx
        .state
        .settlement_claims
        .as_slice()
        .iter()
        .filter(|claim| claim.world == world && claim.close_tick == close)
        .filter(|claim| ark_docked(ctx.state, world, claim.civilization))
        .map(|claim| claim.civilization)
        .collect();
    present.sort();
    present.dedup();
    if present.is_empty() {
        clear_claims(ctx, world)?;
        return Ok(());
    }
    present.sort_by(|left, right| {
        claim_rank_v2(ctx.genesis_seed, world, close, *left)
            .cmp(&claim_rank_v2(ctx.genesis_seed, world, close, *right))
            .then(left.cmp(right))
    });
    let winner = present[0];
    consume_one_ark(ctx, world, winner)?;
    if let Some(row) = ctx.state.worlds.find_mut(&world) {
        row.owner = Some(winner);
        row.inventory = LivingInventoryV2 {
            energy: SETTLEMENT_ENERGY,
            ore: SETTLEMENT_ORE,
            alloy: 0,
        };
    }
    put(
        &mut ctx.state.colonies,
        LivingColonyV2 {
            world,
            owner: winner,
            hub: LivingHubV2 {
                energy_next_due: add_ticks(ctx.tick, LIVING_HUB_ENERGY_PERIOD_TICKS_V2)?,
                ore_next_due: add_ticks(ctx.tick, LIVING_HUB_ORE_PERIOD_TICKS_V2)?,
                fallback_next_due: add_ticks(ctx.tick, LIVING_HUB_FALLBACK_PERIOD_TICKS_V2)?,
                blocked_reasons: LivingSortedVecV2::default(),
            },
        },
    )?;
    if let Some(civ) = ctx.state.civilizations.find_mut(&winner)
        && civ.status == LivingCivilizationStatusV2::Dormant
    {
        civ.status = LivingCivilizationStatusV2::Active;
        emit(
            ctx,
            LivingPhaseV2::ResolveConflict,
            LivingEventKindV2::CivilizationReactivated,
        )?;
    }
    clear_claims(ctx, world)?;
    emit(
        ctx,
        LivingPhaseV2::ResolveConflict,
        LivingEventKindV2::SettlementClaimResolved,
    )?;
    Ok(())
}

fn transfer_colony(
    ctx: &mut LivingStepContextV2<'_>,
    world: LivingEntityIdV2,
    previous: LivingEntityIdV2,
    claimant: LivingEntityIdV2,
) -> Result<(), LivingDeterministicFaultV2> {
    if let Some(row) = ctx.state.worlds.find_mut(&world) {
        row.owner = Some(claimant);
    }
    if let Some(colony) = ctx.state.colonies.find_mut(&world) {
        colony.owner = claimant;
    }
    refund_jobs(ctx, world, previous)?;
    let mut reasons = relation_reasons(ctx.state, previous, claimant);
    reasons.colony_captured_from_it = (reasons.colony_captured_from_it - 30).max(-60);
    let base = ctx
        .state
        .relations
        .as_slice()
        .iter()
        .find(|relation| relation.from == previous && relation.to == claimant)
        .map(|relation| relation.base_disposition)
        .unwrap_or(0);
    put(
        &mut ctx.state.relations,
        LivingRelationV2 {
            from: previous,
            to: claimant,
            base_disposition: base,
            reasons,
        },
    )?;
    let still = ctx
        .state
        .colonies
        .as_slice()
        .iter()
        .any(|colony| colony.owner == previous);
    if !still {
        if let Some(civ) = ctx.state.civilizations.find_mut(&previous) {
            civ.status = LivingCivilizationStatusV2::Dormant;
        }
        emit(
            ctx,
            LivingPhaseV2::ResolveConflict,
            LivingEventKindV2::CivilizationDormant,
        )?;
    }
    if let Some(civ) = ctx.state.civilizations.find_mut(&claimant) {
        civ.status = LivingCivilizationStatusV2::Active;
    }
    emit(
        ctx,
        LivingPhaseV2::ResolveConflict,
        LivingEventKindV2::ColonyCaptured,
    )?;
    Ok(())
}

#[cfg(test)]
fn complete_construction(
    ctx: &mut LivingStepContextV2<'_>,
) -> Result<(), LivingDeterministicFaultV2> {
    let due: Vec<_> = ctx
        .state
        .construction_jobs
        .as_slice()
        .iter()
        .filter(|job| job.completion_tick == ctx.tick)
        .cloned()
        .collect();
    for (offset, job) in due.into_iter().enumerate() {
        let local_id = u16::try_from(offset).map_err(|_| LivingDeterministicFaultV2::Invariant)?;
        let period = ctx
            .catalog
            .industry(job.definition.as_str())
            .and_then(|definition| match definition.effect {
                LivingIndustryEffectV2::Recipe { period_ticks, .. } => {
                    Some(u64::from(period_ticks))
                }
                _ => None,
            });
        let hit_points = ctx
            .catalog
            .industry(job.definition.as_str())
            .and_then(|definition| match definition.effect {
                LivingIndustryEffectV2::Defense { hit_points, .. } => Some(hit_points),
                _ => None,
            })
            .unwrap_or(0);
        let id = autonomous_id(
            ctx,
            job.owner,
            LivingPhaseV2::RunProduction,
            1,
            LivingEntityKindV2::Facility,
            local_id,
        )?;
        put(
            &mut ctx.state.facilities,
            LivingFacilityV2 {
                id,
                world: job.world,
                definition: job.definition,
                status: LivingFacilityStatusV2::Operational,
                paid_alloy: job.paid_alloy,
                hit_points,
                next_due: period
                    .map(|period| add_ticks(ctx.tick, period))
                    .transpose()?,
                next_repair: None,
                blocked_reasons: LivingSortedVecV2::default(),
                disabled: false,
            },
        )?;
        remove_key(&mut ctx.state.construction_jobs, &job.id)?;
        ctx.events.record_explained(
            LivingEventProvenanceV2::Autonomous {
                phase: LivingPhaseV2::RunProduction,
            },
            LivingEventKindV2::ConstructionCompleted,
            super::event_facts::LivingEventFactsV2 {
                actor: Some(job.owner),
                entity: Some(id),
                ..Default::default()
            },
        )?;
    }
    Ok(())
}

#[cfg(test)]
fn complete_hulls(ctx: &mut LivingStepContextV2<'_>) -> Result<(), LivingDeterministicFaultV2> {
    let due: Vec<_> = ctx
        .state
        .hull_jobs
        .as_slice()
        .iter()
        .filter(|job| job.completion_tick == ctx.tick)
        .cloned()
        .collect();
    for (offset, job) in due.into_iter().enumerate() {
        let local_id = u16::try_from(offset).map_err(|_| LivingDeterministicFaultV2::Invariant)?;
        let hit_points = ctx
            .catalog
            .hull(job.definition.as_str())
            .map(|definition| definition.hit_points)
            .unwrap_or(1);
        let hull_id = autonomous_id(
            ctx,
            job.owner,
            LivingPhaseV2::RunProduction,
            1,
            LivingEntityKindV2::Hull,
            local_id,
        )?;
        let hull = LivingHullV2 {
            id: hull_id,
            definition: job.definition,
            hit_points,
            return_credit: false,
            next_repair: None,
        };
        if let Some(fleet_id) = job.target_fleet {
            if let Some(fleet) = ctx.state.fleets.find_mut(&fleet_id) {
                put(&mut fleet.hulls, hull)?;
            }
        } else {
            let fleet_id = autonomous_id(
                ctx,
                job.owner,
                LivingPhaseV2::RunProduction,
                2,
                LivingEntityKindV2::Fleet,
                local_id,
            )?;
            put(
                &mut ctx.state.fleets,
                LivingFleetV2 {
                    id: fleet_id,
                    owner: job.owner,
                    location: LivingFleetLocationV2::Docked { world: job.world },
                    order: None,
                    hulls: LivingSortedVecV2::new(vec![hull])
                        .map_err(LivingValidationErrorV2::from)?,
                },
            )?;
        }
        remove_key(&mut ctx.state.hull_jobs, &job.id)?;
        ctx.events.record_explained(
            LivingEventProvenanceV2::Autonomous {
                phase: LivingPhaseV2::RunProduction,
            },
            LivingEventKindV2::HullCompleted,
            super::event_facts::LivingEventFactsV2 {
                actor: Some(job.owner),
                entity: Some(hull_id),
                ..Default::default()
            },
        )?;
    }
    Ok(())
}

#[cfg(test)]
fn run_hubs(ctx: &mut LivingStepContextV2<'_>) -> Result<(), LivingDeterministicFaultV2> {
    let worlds: Vec<_> = ctx
        .state
        .colonies
        .as_slice()
        .iter()
        .map(|colony| colony.world)
        .collect();
    for world in worlds {
        let Some(colony) = ctx
            .state
            .colonies
            .as_slice()
            .iter()
            .find(|colony| colony.world == world)
            .cloned()
        else {
            continue;
        };
        if colony.hub.energy_next_due.0 <= ctx.tick.0 {
            let reasons = produce(
                ctx.state,
                world,
                &[],
                &[(LivingResourceV2::Energy, HUB_ENERGY_OUTPUT)],
                0,
            );
            let next = add_ticks(
                colony.hub.energy_next_due,
                LIVING_HUB_ENERGY_PERIOD_TICKS_V2,
            )?;
            if let Some(stored) = ctx.state.colonies.find_mut(&world) {
                stored.hub.energy_next_due = next;
                stored.hub.blocked_reasons = reasons?;
            }
        }
        let Some(colony) = ctx
            .state
            .colonies
            .as_slice()
            .iter()
            .find(|colony| colony.world == world)
            .cloned()
        else {
            continue;
        };
        if colony.hub.ore_next_due.0 <= ctx.tick.0 {
            let reasons = produce(
                ctx.state,
                world,
                &[],
                &[(LivingResourceV2::Ore, HUB_ORE_OUTPUT)],
                0,
            );
            let next = add_ticks(colony.hub.ore_next_due, LIVING_HUB_ORE_PERIOD_TICKS_V2)?;
            if let Some(stored) = ctx.state.colonies.find_mut(&world) {
                stored.hub.ore_next_due = next;
                if stored.hub.blocked_reasons.as_slice().is_empty() {
                    stored.hub.blocked_reasons = reasons?;
                }
            }
        }
        let Some(colony) = ctx
            .state
            .colonies
            .as_slice()
            .iter()
            .find(|colony| colony.world == world)
            .cloned()
        else {
            continue;
        };
        if colony.hub.fallback_next_due.0 <= ctx.tick.0 {
            let alloy = world_quantity(ctx.state, world, LivingResourceV2::Alloy);
            let reasons = if alloy >= HUB_FALLBACK_CEILING {
                Ok(LivingSortedVecV2::default())
            } else {
                produce(
                    ctx.state,
                    world,
                    &[
                        (LivingResourceV2::Energy, HUB_FALLBACK_ENERGY),
                        (LivingResourceV2::Ore, HUB_FALLBACK_ORE),
                    ],
                    &[(LivingResourceV2::Alloy, HUB_FALLBACK_ALLOY)],
                    0,
                )
            };
            let next = add_ticks(
                colony.hub.fallback_next_due,
                LIVING_HUB_FALLBACK_PERIOD_TICKS_V2,
            )?;
            if let Some(stored) = ctx.state.colonies.find_mut(&world) {
                stored.hub.fallback_next_due = next;
                let reasons = reasons?;
                if !reasons.as_slice().is_empty() {
                    stored.hub.blocked_reasons = reasons;
                }
            }
        }
    }
    Ok(())
}

#[cfg(test)]
fn run_facilities(
    ctx: &mut LivingStepContextV2<'_>,
    definition_id: &str,
) -> Result<(), LivingDeterministicFaultV2> {
    let ids: Vec<_> = ctx
        .state
        .facilities
        .as_slice()
        .iter()
        .filter(|facility| facility.definition.as_str() == definition_id)
        .map(|facility| facility.id)
        .collect();
    for id in ids {
        let Some(facility) = ctx
            .state
            .facilities
            .as_slice()
            .iter()
            .find(|facility| facility.id == id)
            .cloned()
        else {
            continue;
        };
        let Some(due) = facility.next_due else {
            continue;
        };
        if due.0 > ctx.tick.0 {
            continue;
        }
        let Some(definition) = ctx.catalog.industry(facility.definition.as_str()) else {
            continue;
        };
        let LivingIndustryEffectV2::Recipe {
            period_ticks,
            deposit_units,
            ref inputs,
            ref outputs,
        } = definition.effect
        else {
            continue;
        };
        let input_pairs: Vec<_> = inputs
            .as_slice()
            .iter()
            .filter_map(|amount| {
                resource_of(amount.resource.as_str()).map(|resource| (resource, amount.quantity))
            })
            .collect();
        let output_pairs: Vec<_> = outputs
            .as_slice()
            .iter()
            .filter_map(|amount| {
                resource_of(amount.resource.as_str()).map(|resource| (resource, amount.quantity))
            })
            .collect();
        let reasons = if facility.disabled {
            LivingSortedVecV2::new(vec![LivingBlockedReasonV2::Disabled])
                .map_err(LivingValidationErrorV2::from)?
        } else {
            produce(
                ctx.state,
                facility.world,
                &input_pairs,
                &output_pairs,
                deposit_units,
            )?
        };
        let blocked = !reasons.as_slice().is_empty();
        let explained_reasons = reasons.as_slice().to_vec();
        if let Some(stored) = ctx.state.facilities.find_mut(&id) {
            stored.next_due = Some(add_ticks(due, u64::from(period_ticks))?);
            stored.blocked_reasons = reasons;
            stored.status = if blocked {
                LivingFacilityStatusV2::Blocked
            } else {
                LivingFacilityStatusV2::Operational
            };
        }
        if blocked {
            ctx.events.record_explained(
                LivingEventProvenanceV2::Autonomous {
                    phase: LivingPhaseV2::RunProduction,
                },
                LivingEventKindV2::FacilityBlocked,
                super::event_facts::LivingEventFactsV2 {
                    actor: world_owner(ctx.state, facility.world),
                    entity: Some(id),
                    blocked_reasons: explained_reasons,
                    ..Default::default()
                },
            )?;
        } else {
            use super::event_facts::{
                LivingDepositInputFactV2, LivingEventResourceFactV2, LivingEventResourceRoleV2,
            };
            let mut resources: Vec<_> =
                input_pairs
                    .iter()
                    .map(|(resource, quantity)| LivingEventResourceFactV2 {
                        role: LivingEventResourceRoleV2::Input,
                        resource: *resource,
                        quantity: *quantity,
                    })
                    .chain(output_pairs.iter().map(|(resource, quantity)| {
                        LivingEventResourceFactV2 {
                            role: LivingEventResourceRoleV2::Output,
                            resource: *resource,
                            quantity: *quantity,
                        }
                    }))
                    .collect();
            resources.sort_by_key(|fact| (fact.role, fact.resource));
            ctx.events.record_explained(
                LivingEventProvenanceV2::Autonomous {
                    phase: LivingPhaseV2::RunProduction,
                },
                LivingEventKindV2::FacilityProduced,
                super::event_facts::LivingEventFactsV2 {
                    actor: world_owner(ctx.state, facility.world),
                    entity: Some(id),
                    resources,
                    deposit_input: (deposit_units > 0).then_some(LivingDepositInputFactV2 {
                        world: facility.world,
                        units: deposit_units,
                    }),
                    ..Default::default()
                },
            )?;
        }
    }
    Ok(())
}

#[cfg(test)]
fn run_repairs_reference(
    ctx: &mut LivingStepContextV2<'_>,
) -> Result<(), LivingDeterministicFaultV2> {
    // Independent test oracle: collect IDs, sort once, then resolve by ID.
    // No cursor, retained index or production repair implementation is shared.
    let mut assets = Vec::new();
    for fleet in ctx.state.fleets.as_slice() {
        let LivingFleetLocationV2::Docked { world } = fleet.location else {
            continue;
        };
        if !owned_by(ctx.state, world, fleet.owner) || ctx.repair_exclusions.contains(&world) {
            continue;
        }
        for hull in fleet.hulls.as_slice() {
            if hull.hit_points > 0
                && let Some(due) = hull.next_repair.filter(|due| *due <= ctx.tick)
            {
                let max = ctx
                    .catalog
                    .hull(hull.definition.as_str())
                    .map_or(1, |d| d.hit_points);
                assets.push((hull.id, Some(fleet.id), world, due, max));
            }
        }
    }
    for battery in ctx.state.facilities.as_slice() {
        if battery.hit_points == 0
            || world_owner(ctx.state, battery.world).is_none()
            || ctx.repair_exclusions.contains(&battery.world)
        {
            continue;
        }
        if let Some(due) = battery.next_repair.filter(|due| *due <= ctx.tick)
            && let Some(definition) = ctx.catalog.industry(battery.definition.as_str())
            && let LivingIndustryEffectV2::Defense { hit_points, .. } = definition.effect
        {
            assets.push((battery.id, None, battery.world, due, hit_points));
        }
    }
    assets.sort_by_key(|row| row.0);
    for (id, fleet, world, due, max) in assets {
        let current = match fleet {
            Some(fleet) => {
                ctx.state
                    .fleets
                    .as_slice()
                    .iter()
                    .find(|f| f.id == fleet)
                    .unwrap()
                    .hulls
                    .as_slice()
                    .iter()
                    .find(|h| h.id == id)
                    .unwrap()
                    .hit_points
            }
            None => {
                ctx.state
                    .facilities
                    .as_slice()
                    .iter()
                    .find(|f| f.id == id)
                    .unwrap()
                    .hit_points
            }
        };
        let paid = current < max && spend(ctx.state, world, LivingResourceV2::Energy, 1);
        let (hp, next) = match fleet {
            Some(fleet) => {
                let h = ctx
                    .state
                    .fleets
                    .find_mut(&fleet)
                    .unwrap()
                    .hulls
                    .find_mut(&id)
                    .unwrap();
                (&mut h.hit_points, &mut h.next_repair)
            }
            None => {
                let f = ctx.state.facilities.find_mut(&id).unwrap();
                (&mut f.hit_points, &mut f.next_repair)
            }
        };
        if paid {
            *hp += 1;
        }
        *next = if *hp >= max {
            None
        } else {
            Some(add_ticks(due, REPAIR_PERIOD_TICKS)?)
        };
    }
    Ok(())
}

#[cfg(test)]
fn old_run_repairs_reference(
    ctx: &mut LivingStepContextV2<'_>,
) -> Result<(), LivingDeterministicFaultV2> {
    let fleet_ids: Vec<_> = ctx
        .state
        .fleets
        .as_slice()
        .iter()
        .map(|fleet| fleet.id)
        .collect();
    for fleet_id in fleet_ids {
        let Some(fleet) = ctx
            .state
            .fleets
            .as_slice()
            .iter()
            .find(|fleet| fleet.id == fleet_id)
        else {
            continue;
        };
        let LivingFleetLocationV2::Docked { world } = fleet.location else {
            continue;
        };
        if !owned_by(ctx.state, world, fleet.owner) {
            continue;
        }
        let hulls: Vec<_> = fleet
            .hulls
            .as_slice()
            .iter()
            .filter(|hull| hull.next_repair.is_some_and(|due| due.0 <= ctx.tick.0))
            .map(|hull| hull.id)
            .collect();
        for hull_id in hulls {
            let max = ctx
                .state
                .fleets
                .as_slice()
                .iter()
                .find(|fleet| fleet.id == fleet_id)
                .and_then(|fleet| {
                    fleet
                        .hulls
                        .as_slice()
                        .iter()
                        .find(|hull| hull.id == hull_id)
                })
                .and_then(|hull| {
                    ctx.catalog
                        .hull(hull.definition.as_str())
                        .map(|definition| definition.hit_points)
                })
                .unwrap_or(1);
            let current = ctx
                .state
                .fleets
                .as_slice()
                .iter()
                .find(|fleet| fleet.id == fleet_id)
                .and_then(|fleet| {
                    fleet
                        .hulls
                        .as_slice()
                        .iter()
                        .find(|hull| hull.id == hull_id)
                })
                .map(|hull| hull.hit_points)
                .unwrap_or(max);
            if current >= max {
                if let Some(fleet) = ctx.state.fleets.find_mut(&fleet_id)
                    && let Some(hull) = fleet.hulls.find_mut(&hull_id)
                {
                    hull.next_repair = None;
                }
                continue;
            }
            let paid = spend(ctx.state, world, LivingResourceV2::Energy, 1);
            if let Some(fleet) = ctx.state.fleets.find_mut(&fleet_id)
                && let Some(hull) = fleet.hulls.find_mut(&hull_id)
            {
                if paid {
                    hull.hit_points = hull.hit_points.saturating_add(1).min(max);
                }
                hull.next_repair = if hull.hit_points >= max {
                    None
                } else {
                    Some(add_ticks(ctx.tick, REPAIR_PERIOD_TICKS)?)
                };
            }
        }
    }
    Ok(())
}

#[cfg(test)]
fn produce(
    state: &mut LivingGalaxyStateV2,
    world: LivingEntityIdV2,
    inputs: &[(LivingResourceV2, u32)],
    outputs: &[(LivingResourceV2, u32)],
    deposit_units: u32,
) -> Result<LivingSortedVecV2<LivingBlockedReasonV2>, LivingDeterministicFaultV2> {
    use super::step_job::{
        LivingLifecycleJobErrorV2, LivingLifecycleProgressV2, recipe_legacy_oracle::Recipe,
    };
    let inventory = state
        .worlds
        .as_slice()
        .iter()
        .find(|row| row.id == world)
        .map(|row| row.inventory)
        .unwrap_or_default();
    let remaining = if deposit_units > 0 {
        ore_deposit_remaining(state, world)
    } else {
        0
    };
    // The surrounding production phase still owns its entity cursor. This
    // component already slices individual amount/reason records and reserves
    // all resources before applying any private recipe payment.
    let mut job = Recipe::begin(
        inventory,
        remaining,
        deposit_units,
        inputs.to_vec(),
        outputs.to_vec(),
    );
    let map_error = |error| match error {
        LivingLifecycleJobErrorV2::Fault(fault) => fault,
        _ => LivingDeterministicFaultV2::Invariant,
    };
    while job.poll(1_024).map_err(map_error)? != LivingLifecycleProgressV2::Ready {}
    let result = job.finish().map_err(map_error)?;
    if result.blocked_reasons.as_slice().is_empty() {
        if let Some(stored) = state.worlds.find_mut(&world) {
            stored.inventory = result.inventory;
        }
        if deposit_units > 0 {
            draw_ore_deposit(state, world, deposit_units);
        }
    }
    Ok(result.blocked_reasons)
}

#[cfg(test)]
fn observe_world(
    ctx: &LivingStepContextV2<'_>,
    civ: LivingEntityIdV2,
    world: LivingEntityIdV2,
    tick: LivingTickV2,
) -> Result<LivingObservationV2, LivingDeterministicFaultV2> {
    let mut facilities: Vec<LivingObservedFacilityV2> = Vec::new();
    for facility in ctx.state.facilities.as_slice() {
        if facility.world != world {
            continue;
        }
        if let Some(row) = facilities
            .iter_mut()
            .find(|row| row.definition == facility.definition)
        {
            row.count = row.count.saturating_add(1);
        } else {
            facilities.push(LivingObservedFacilityV2 {
                definition: facility.definition.clone(),
                count: 1,
            });
        }
    }
    let aggregate_batteries = ctx
        .state
        .facilities
        .as_slice()
        .iter()
        .filter(|facility| {
            facility.world == world && facility.definition.as_str() == "defense_battery"
        })
        .count();
    let operational_batteries = ctx
        .state
        .facilities
        .as_slice()
        .iter()
        .filter(|facility| {
            facility.world == world
                && facility.definition.as_str() == "defense_battery"
                && !facility.disabled
                && facility.hit_points > 0
        })
        .count();
    let operational_defense_batteries = if operational_batteries < aggregate_batteries {
        Some(
            u16::try_from(operational_batteries)
                .map_err(|_| LivingDeterministicFaultV2::Arithmetic)?,
        )
    } else {
        None
    };
    facilities.sort_by(|left, right| left.definition.cmp(&right.definition));
    let mut stationed: Vec<LivingObservedHullV2> = Vec::new();
    for fleet in ctx.state.fleets.as_slice() {
        let LivingFleetLocationV2::Docked { world: docked } = fleet.location else {
            continue;
        };
        if docked != world {
            continue;
        }
        for hull in fleet.hulls.as_slice() {
            if let Some(row) = stationed
                .iter_mut()
                .find(|row| row.definition == hull.definition)
            {
                row.count = row.count.saturating_add(1);
            } else {
                stationed.push(LivingObservedHullV2 {
                    definition: hull.definition.clone(),
                    count: 1,
                });
            }
        }
    }
    stationed.sort_by(|left, right| left.definition.cmp(&right.definition));
    Ok(LivingObservationV2 {
        civilization: civ,
        world,
        observed_tick: tick,
        owner: world_owner(ctx.state, world),
        remaining_deposit: ore_deposit_remaining(ctx.state, world),
        facilities: LivingSortedVecV2::new(facilities).map_err(LivingValidationErrorV2::from)?,
        stationed_hulls: LivingSortedVecV2::new(stationed)
            .map_err(LivingValidationErrorV2::from)?,
        operational_defense_batteries,
    })
}

fn decay_relations(ctx: &mut LivingStepContextV2<'_>) -> Result<(), LivingDeterministicFaultV2> {
    let rows: Vec<_> = ctx.state.relations.as_slice().to_vec();
    for mut relation in rows {
        let reasons = &mut relation.reasons;
        if reasons.delivered_aid > 0 {
            reasons.delivered_aid -= 1;
        }
        if reasons.successful_trade > 0 {
            reasons.successful_trade -= 1;
        }
        if reasons.adjacent_rival_settlement < 0 {
            reasons.adjacent_rival_settlement += 1;
        }
        if reasons.war_declared_against_it < 0 {
            reasons.war_declared_against_it += 1;
        }
        if reasons.colony_captured_from_it < 0 {
            reasons.colony_captured_from_it += 1;
        }
        put(&mut ctx.state.relations, relation)?;
    }
    Ok(())
}

fn refund_jobs(
    ctx: &mut LivingStepContextV2<'_>,
    world: LivingEntityIdV2,
    owner: LivingEntityIdV2,
) -> Result<(), LivingDeterministicFaultV2> {
    let jobs: Vec<_> = ctx
        .state
        .construction_jobs
        .as_slice()
        .iter()
        .filter(|job| job.world == world && job.owner == owner)
        .map(|job| (job.id, job.paid_alloy))
        .collect();
    for (id, paid) in jobs {
        let _ = credit(ctx.state, world, LivingResourceV2::Alloy, paid / 2);
        remove_key(&mut ctx.state.construction_jobs, &id)?;
        emit(
            ctx,
            LivingPhaseV2::ResolveConflict,
            LivingEventKindV2::ConstructionCancelled,
        )?;
    }
    let hulls: Vec<_> = ctx
        .state
        .hull_jobs
        .as_slice()
        .iter()
        .filter(|job| job.world == world && job.owner == owner)
        .map(|job| (job.id, job.paid_alloy))
        .collect();
    for (id, paid) in hulls {
        let _ = credit(ctx.state, world, LivingResourceV2::Alloy, paid / 2);
        remove_key(&mut ctx.state.hull_jobs, &id)?;
    }
    Ok(())
}

fn travel(
    state: &LivingGalaxyStateV2,
    from: LivingEntityIdV2,
    to: LivingEntityIdV2,
) -> Option<(u64, Vec<LivingEntityIdV2>)> {
    let origin = world_system(state, from)?;
    let destination = world_system(state, to)?;
    if origin == destination {
        return Some((LOCAL_TRAVEL_TICKS, Vec::new()));
    }
    let systems: Vec<_> = state
        .systems
        .as_slice()
        .iter()
        .map(|system| system.id)
        .collect();
    let mut best: Vec<(u64, Vec<LivingEntityIdV2>)> = systems
        .iter()
        .map(|system| {
            if *system == origin {
                (0, Vec::new())
            } else {
                (u64::MAX, Vec::new())
            }
        })
        .collect();
    let index_of = |id: LivingEntityIdV2| systems.iter().position(|system| *system == id);
    let mut used = vec![false; systems.len()];
    for _ in 0..systems.len() {
        let mut choice = None;
        for (index, (cost, path)) in best.iter().enumerate() {
            if used[index] || *cost == u64::MAX {
                continue;
            }
            let better = match &choice {
                None => true,
                Some((best_cost, best_path, _)) => (*cost, path) < (*best_cost, best_path),
            };
            if better {
                choice = Some((*cost, path.clone(), index));
            }
        }
        let Some((cost, path, index)) = choice else {
            break;
        };
        used[index] = true;
        if systems[index] == destination {
            return Some((cost, path));
        }
        for lane in state.lanes.as_slice() {
            let next = if lane.system_a == systems[index] {
                Some(lane.system_b)
            } else if lane.system_b == systems[index] {
                Some(lane.system_a)
            } else {
                None
            };
            let Some(next) = next else {
                continue;
            };
            let Some(next_index) = index_of(next) else {
                continue;
            };
            let mut next_path = path.clone();
            next_path.push(lane.id);
            let next_cost = cost.saturating_add(leg_ticks(lane.distance_units));
            if (next_cost, next_path.clone()) < (best[next_index].0, best[next_index].1.clone()) {
                best[next_index] = (next_cost, next_path);
            }
        }
    }
    None
}

fn leg_ticks(distance: u64) -> u64 {
    distance.div_ceil(10).max(LOCAL_TRAVEL_TICKS)
}

#[cfg(test)]
fn hazard_batch(ctx: &LivingStepContextV2<'_>, route: &LivingFreightRouteV2, batch: u32) -> u32 {
    hazard_batch_details(ctx, route, batch).0
}

fn hazard_batch_details(
    ctx: &LivingStepContextV2<'_>,
    route: &LivingFreightRouteV2,
    batch: u32,
) -> (u32, Option<LivingHazardV2>) {
    let Some(from) = world_system(ctx.state, route.source_world) else {
        return (batch, None);
    };
    let Some(to) = world_system(ctx.state, route.destination_world) else {
        return (batch, None);
    };
    if from == to {
        return (batch, None);
    }
    let mut scaled = batch;
    let mut effective = None;
    for hazard in ctx.state.hazards.as_slice() {
        if hazard.start_tick.0 > ctx.tick.0 || hazard.end_tick.0 <= ctx.tick.0 {
            continue;
        }
        let Some(lane) = ctx
            .state
            .lanes
            .as_slice()
            .iter()
            .find(|lane| lane.id == hazard.lane)
        else {
            continue;
        };
        let connects = (lane.system_a == from && lane.system_b == to)
            || (lane.system_a == to && lane.system_b == from);
        if !connects {
            continue;
        }
        if let Some(definition) = ctx.catalog.hazard(hazard.definition.as_str()) {
            let next = definition.effect.scale_batch(batch);
            if next < scaled {
                scaled = next;
                effective = Some(hazard.clone());
            }
        }
    }
    (scaled, effective)
}

fn record_corridor_restriction(
    ctx: &mut LivingStepContextV2<'_>,
    route: &LivingFreightRouteV2,
    eligible: u32,
    restricted: u32,
    actual: u32,
    hazard: Option<LivingHazardV2>,
) -> Result<(), LivingDeterministicFaultV2> {
    if !ctx.events.explanations_enabled() || eligible == 0 || restricted >= eligible {
        return Ok(());
    }
    let Some(hazard) = hazard else {
        return Ok(());
    };
    if ctx.events.corridor_restrictions.len() >= 4096 {
        return Err(LivingDeterministicFaultV2::Capacity);
    }
    let Some((source_system, destination_system)) = world_system(ctx.state, route.source_world)
        .zip(world_system(ctx.state, route.destination_world))
    else {
        return Err(LivingDeterministicFaultV2::Invariant);
    };
    ctx.events
        .corridor_restrictions
        .push(super::event_facts::LivingCorridorRestrictionV2 {
            tick: ctx.tick,
            receipt_digest: super::ids::LivingReceiptDigestV2([0; 32]),
            route: route.id,
            source_world: route.source_world,
            destination_world: route.destination_world,
            source_system,
            destination_system,
            sender: route.source_owner,
            receiver: route.receiver_owner,
            resource: route.resource,
            requested_batch: route.batch_size,
            eligible_batch: eligible,
            restricted_batch: restricted,
            actual_dispatched_units: actual,
            hazard,
        });
    Ok(())
}

fn autonomous_id(
    ctx: &LivingStepContextV2<'_>,
    actor: LivingEntityIdV2,
    phase: LivingPhaseV2,
    intent_ordinal: u16,
    kind: LivingEntityKindV2,
    local_id: u16,
) -> Result<LivingEntityIdV2, LivingDeterministicFaultV2> {
    if intent_ordinal == 0 {
        return Err(LivingDeterministicFaultV2::Invariant);
    }
    Ok(autonomous_entity_id_v2(&LivingAutonomousEntityInputsV2 {
        catalog_hash: ctx.catalog.catalog_hash(),
        genesis_seed: ctx.genesis_seed,
        branch: ctx.branch,
        tick: ctx.tick,
        phase: phase.ordinal(),
        actor,
        intent_ordinal,
        entity_kind: kind.ordinal(),
        local_id,
    }))
}

fn emit(
    ctx: &mut LivingStepContextV2<'_>,
    phase: LivingPhaseV2,
    kind: LivingEventKindV2,
) -> Result<(), LivingDeterministicFaultV2> {
    ctx.emit(LivingEventProvenanceV2::Autonomous { phase }, kind)
}

fn add_ticks(tick: LivingTickV2, period: u64) -> Result<LivingTickV2, LivingDeterministicFaultV2> {
    tick.0
        .checked_add(period)
        .map(LivingTickV2)
        .ok_or(LivingDeterministicFaultV2::Arithmetic)
}

fn install<T>(collection: &mut LivingSortedVecV2<T>, record: T) -> Result<(), String>
where
    T: LivingKeyedV2,
{
    put(collection, record).map_err(|error| error.to_string())
}

pub(crate) fn put<T>(
    collection: &mut LivingSortedVecV2<T>,
    record: T,
) -> Result<(), LivingValidationErrorV2>
where
    T: LivingKeyedV2,
{
    let mut records = std::mem::take(collection).into_vec();
    let key = record.living_key();
    let position = records.partition_point(|existing| existing.living_key() < key);
    if records
        .get(position)
        .is_some_and(|existing| existing.living_key() == key)
    {
        records[position] = record;
    } else {
        records.insert(position, record);
    }
    *collection = LivingSortedVecV2::new(records)?;
    Ok(())
}

pub(crate) fn remove_key<T>(
    collection: &mut LivingSortedVecV2<T>,
    key: &T::Key,
) -> Result<(), LivingValidationErrorV2>
where
    T: LivingKeyedV2,
{
    let mut records = std::mem::take(collection).into_vec();
    if let Ok(position) = records.binary_search_by(|record| record.living_key().cmp(key)) {
        records.remove(position);
    }
    *collection = LivingSortedVecV2::new(records)?;
    Ok(())
}

fn ordered_pair(
    left: LivingEntityIdV2,
    right: LivingEntityIdV2,
) -> (LivingEntityIdV2, LivingEntityIdV2) {
    if left < right {
        (left, right)
    } else {
        (right, left)
    }
}

fn remove_pair_agreements(
    state: &mut LivingGalaxyStateV2,
    left: LivingEntityIdV2,
    right: LivingEntityIdV2,
) -> Result<(), LivingValidationErrorV2> {
    let keys: Vec<_> = state
        .agreements
        .as_slice()
        .iter()
        .filter(|agreement| agreement.participant_a == left && agreement.participant_b == right)
        .map(|agreement| agreement.living_key())
        .collect();
    for key in keys {
        remove_key(&mut state.agreements, &key)?;
    }
    Ok(())
}

fn world_owner(state: &LivingGalaxyStateV2, world: LivingEntityIdV2) -> Option<LivingEntityIdV2> {
    state
        .worlds
        .as_slice()
        .iter()
        .find(|row| row.id == world)
        .and_then(|row| row.owner)
}

fn world_system(state: &LivingGalaxyStateV2, world: LivingEntityIdV2) -> Option<LivingEntityIdV2> {
    state
        .worlds
        .as_slice()
        .iter()
        .find(|row| row.id == world)
        .map(|row| row.system)
}

fn world_quantity(
    state: &LivingGalaxyStateV2,
    world: LivingEntityIdV2,
    resource: LivingResourceV2,
) -> u32 {
    state
        .worlds
        .as_slice()
        .iter()
        .find(|row| row.id == world)
        .map(|row| row.inventory.quantity(resource))
        .unwrap_or(0)
}

fn spend(
    state: &mut LivingGalaxyStateV2,
    world: LivingEntityIdV2,
    resource: LivingResourceV2,
    amount: u32,
) -> bool {
    let Some(row) = state.worlds.find_mut(&world) else {
        return false;
    };
    let current = row.inventory.quantity(resource);
    if current < amount {
        return false;
    }
    set_quantity(&mut row.inventory, resource, current - amount);
    true
}

fn credit(
    state: &mut LivingGalaxyStateV2,
    world: LivingEntityIdV2,
    resource: LivingResourceV2,
    amount: u32,
) -> bool {
    let Some(row) = state.worlds.find_mut(&world) else {
        return false;
    };
    let current = row.inventory.quantity(resource);
    let Some(next) = current.checked_add(amount) else {
        return false;
    };
    if next > LIVING_RESOURCE_STORAGE_LIMIT_V2 {
        return false;
    }
    set_quantity(&mut row.inventory, resource, next);
    true
}

fn set_quantity(inventory: &mut LivingInventoryV2, resource: LivingResourceV2, quantity: u32) {
    match resource {
        LivingResourceV2::Alloy => inventory.alloy = quantity,
        LivingResourceV2::Energy => inventory.energy = quantity,
        LivingResourceV2::Ore => inventory.ore = quantity,
    }
}

#[cfg(test)]
fn ore_deposit_remaining(state: &LivingGalaxyStateV2, world: LivingEntityIdV2) -> u64 {
    state
        .deposits
        .as_slice()
        .iter()
        .filter(|deposit| deposit.world == world && deposit.resource == LivingResourceV2::Ore)
        .map(|deposit| deposit.remaining_units)
        .min()
        .unwrap_or(0)
}

#[cfg(test)]
fn draw_ore_deposit(state: &mut LivingGalaxyStateV2, world: LivingEntityIdV2, units: u32) {
    let Some(id) = state
        .deposits
        .as_slice()
        .iter()
        .filter(|deposit| deposit.world == world && deposit.resource == LivingResourceV2::Ore)
        .min_by_key(|deposit| deposit.id)
        .map(|deposit| deposit.id)
    else {
        return;
    };
    if let Some(deposit) = state.deposits.find_mut(&id) {
        deposit.remaining_units = deposit.remaining_units.saturating_sub(u64::from(units));
    }
}

#[cfg(test)]
fn resource_of(slug: &str) -> Option<LivingResourceV2> {
    match slug {
        "alloy" => Some(LivingResourceV2::Alloy),
        "energy" => Some(LivingResourceV2::Energy),
        "ore" => Some(LivingResourceV2::Ore),
        _ => None,
    }
}

fn hull_damage(catalog: &ValidatedLivingCatalogPackV2, hull: &LivingHullV2) -> u32 {
    catalog
        .hull(hull.definition.as_str())
        .map(|definition| definition.damage)
        .unwrap_or(0)
}

fn owned_by(state: &LivingGalaxyStateV2, world: LivingEntityIdV2, civ: LivingEntityIdV2) -> bool {
    state
        .colonies
        .as_slice()
        .iter()
        .any(|colony| colony.world == world && colony.owner == civ)
}

fn at_war(
    state: &LivingGalaxyStateV2,
    left: LivingEntityIdV2,
    right: LivingEntityIdV2,
    tick: LivingTickV2,
) -> bool {
    let (participant_a, participant_b) = ordered_pair(left, right);
    state.wars.as_slice().iter().any(|war| {
        war.participant_a == participant_a
            && war.participant_b == participant_b
            && active_interval(war.start_tick, war.end_tick, tick)
    })
}

fn active_interval(start: LivingTickV2, end: LivingTickV2, tick: LivingTickV2) -> bool {
    start.0 <= tick.0 && tick.0 < end.0
}

fn trade_open(
    state: &LivingGalaxyStateV2,
    left: LivingEntityIdV2,
    right: LivingEntityIdV2,
    tick: LivingTickV2,
) -> bool {
    let (participant_a, participant_b) = ordered_pair(left, right);
    state.agreements.as_slice().iter().any(|agreement| {
        agreement.kind == LivingAgreementKindV2::Trade
            && agreement.participant_a == participant_a
            && agreement.participant_b == participant_b
            && active_interval(agreement.start_tick, agreement.end_tick, tick)
    })
}

fn pair_blocked(
    state: &LivingGalaxyStateV2,
    left: LivingEntityIdV2,
    right: LivingEntityIdV2,
    tick: LivingTickV2,
) -> bool {
    let (participant_a, participant_b) = ordered_pair(left, right);
    at_war(state, left, right, tick)
        || state.agreements.as_slice().iter().any(|agreement| {
            agreement.participant_a == participant_a
                && agreement.participant_b == participant_b
                && active_interval(agreement.start_tick, agreement.end_tick, tick)
        })
}

fn relation_total(
    state: &LivingGalaxyStateV2,
    from: LivingEntityIdV2,
    to: LivingEntityIdV2,
) -> i32 {
    state
        .relations
        .as_slice()
        .iter()
        .find(|relation| relation.from == from && relation.to == to)
        .map(LivingRelationV2::total)
        .unwrap_or(0)
}

fn relation_reasons(
    state: &LivingGalaxyStateV2,
    from: LivingEntityIdV2,
    to: LivingEntityIdV2,
) -> LivingRelationReasonsV2 {
    state
        .relations
        .as_slice()
        .iter()
        .find(|relation| relation.from == from && relation.to == to)
        .map(|relation| relation.reasons)
        .unwrap_or_default()
}

fn adjacent_to_owned_colony(
    state: &LivingGalaxyStateV2,
    civ: LivingEntityIdV2,
    target: LivingEntityIdV2,
) -> bool {
    let Some(system) = world_system(state, target) else {
        return false;
    };
    state
        .colonies
        .as_slice()
        .iter()
        .filter(|colony| colony.owner == civ)
        .any(|colony| {
            let Some(home) = world_system(state, colony.world) else {
                return false;
            };
            home == system
                || state.lanes.as_slice().iter().any(|lane| {
                    (lane.system_a == home && lane.system_b == system)
                        || (lane.system_b == home && lane.system_a == system)
                })
        })
}

fn assembled_attack(
    state: &LivingGalaxyStateV2,
    civ: LivingEntityIdV2,
    target: LivingEntityIdV2,
) -> i64 {
    state
        .fleets
        .as_slice()
        .iter()
        .filter_map(|fleet| {
            let LivingFleetLocationV2::Docked { world } = fleet.location else {
                return None;
            };
            if fleet.owner != civ
                || fleet.order.is_some()
                || !owned_by(state, world, civ)
                || state
                    .hull_jobs
                    .as_slice()
                    .iter()
                    .any(|job| job.target_fleet == Some(fleet.id))
                || world == target
                || travel(state, world, target).is_none()
                || world_quantity(state, world, LivingResourceV2::Energy)
                    < role_launch_cost(state, fleet, "escort")
            {
                return None;
            }
            // A declaration needs one assembled eligible fleet. Unrelated fleets
            // cannot lend nominal strength across origins or occupied intent slots.
            let escorts = fleet
                .hulls
                .as_slice()
                .iter()
                .filter(|hull| hull.definition.as_str() == "escort")
                .count();
            Some(
                i64::try_from(escorts)
                    .unwrap_or(0)
                    .saturating_mul(ESCORT_STRENGTH),
            )
        })
        .max()
        .unwrap_or(0)
}

fn observed_defense(
    state: &LivingGalaxyStateV2,
    civ: LivingEntityIdV2,
    world: LivingEntityIdV2,
) -> i64 {
    let Some(observation) = state
        .observations
        .as_slice()
        .iter()
        .find(|observation| observation.civilization == civ && observation.world == world)
    else {
        return 0;
    };
    let escorts = observation
        .stationed_hulls
        .as_slice()
        .iter()
        .filter(|hull| hull.definition.as_str() == "escort")
        .map(|hull| i64::from(hull.count))
        .sum::<i64>();
    let batteries = observation
        .facilities
        .as_slice()
        .iter()
        .filter(|facility| facility.definition.as_str() == "defense_battery")
        .map(|facility| i64::from(facility.count))
        .sum::<i64>();
    let batteries = observation
        .operational_defense_batteries
        .map(i64::from)
        .unwrap_or(batteries);
    escorts.saturating_mul(ESCORT_STRENGTH) + batteries.saturating_mul(BATTERY_STRENGTH)
}

fn nearest_owned_colony(
    state: &LivingGalaxyStateV2,
    civ: LivingEntityIdV2,
    from: LivingEntityIdV2,
) -> Option<LivingEntityIdV2> {
    let mut best: Option<(u64, LivingEntityIdV2)> = None;
    for colony in state.colonies.as_slice() {
        if colony.owner != civ || colony.world == from {
            continue;
        }
        let Some((duration, _)) = travel(state, from, colony.world) else {
            continue;
        };
        let candidate = (duration, colony.world);
        if best.is_none_or(|current| candidate < current) {
            best = Some(candidate);
        }
    }
    best.map(|(_, world)| world)
}

fn escort_owners(state: &LivingGalaxyStateV2, world: LivingEntityIdV2) -> Vec<LivingEntityIdV2> {
    state
        .fleets
        .as_slice()
        .iter()
        .filter(|fleet| {
            matches!(fleet.location, LivingFleetLocationV2::Docked { world: docked } if docked == world)
                && fleet.hulls.as_slice().iter().any(|hull| hull.definition.as_str() == "escort")
        })
        .map(|fleet| fleet.owner)
        .collect()
}

fn defense_present(
    ctx: &LivingStepContextV2<'_>,
    world: LivingEntityIdV2,
    owner: LivingEntityIdV2,
) -> bool {
    let escorts = ctx.state.fleets.as_slice().iter().any(|fleet| {
        fleet.owner == owner
            && matches!(fleet.location, LivingFleetLocationV2::Docked { world: docked } if docked == world)
            && fleet.hulls.as_slice().iter().any(|hull| hull.definition.as_str() == "escort" && hull.hit_points > 0)
    });
    let battery = ctx.state.facilities.as_slice().iter().any(|facility| {
        facility.world == world
            && facility.hit_points > 0
            && !facility.disabled
            && matches!(
                ctx.catalog
                    .industry(facility.definition.as_str())
                    .map(|definition| &definition.effect),
                Some(LivingIndustryEffectV2::Defense { .. })
            )
    });
    escorts || battery
}

fn armed_presence(ctx: &LivingStepContextV2<'_>, world: LivingEntityIdV2) -> bool {
    let hull = ctx.state.fleets.as_slice().iter().any(|fleet| {
        matches!(fleet.location, LivingFleetLocationV2::Docked { world: docked } if docked == world)
            && fleet
                .hulls
                .as_slice()
                .iter()
                .any(|hull| hull_damage(ctx.catalog, hull) > 0 && hull.hit_points > 0)
    });
    let battery = ctx.state.facilities.as_slice().iter().any(|facility| {
        facility.world == world
            && facility.hit_points > 0
            && !facility.disabled
            && matches!(
                ctx.catalog
                    .industry(facility.definition.as_str())
                    .map(|definition| &definition.effect),
                Some(LivingIndustryEffectV2::Defense { .. })
            )
    });
    hull || battery
}

fn ark_docked(state: &LivingGalaxyStateV2, world: LivingEntityIdV2, civ: LivingEntityIdV2) -> bool {
    state.fleets.as_slice().iter().any(|fleet| {
        fleet.owner == civ
            && matches!(fleet.location, LivingFleetLocationV2::Docked { world: docked } if docked == world)
            && fleet.hulls.as_slice().iter().any(|hull| hull.definition.as_str() == "colony_ark")
    })
}

fn clear_claims(
    ctx: &mut LivingStepContextV2<'_>,
    world: LivingEntityIdV2,
) -> Result<(), LivingDeterministicFaultV2> {
    let keys: Vec<_> = ctx
        .state
        .settlement_claims
        .as_slice()
        .iter()
        .filter(|claim| claim.world == world)
        .map(|claim| claim.living_key())
        .collect();
    for key in keys {
        remove_key(&mut ctx.state.settlement_claims, &key)?;
    }
    Ok(())
}

fn consume_one_ark(
    ctx: &mut LivingStepContextV2<'_>,
    world: LivingEntityIdV2,
    civ: LivingEntityIdV2,
) -> Result<(), LivingDeterministicFaultV2> {
    let Some(fleet_id) = ctx.state.fleets.as_slice().iter().find_map(|fleet| {
        (fleet.owner == civ
            && matches!(fleet.location, LivingFleetLocationV2::Docked { world: docked } if docked == world)
            && fleet.hulls.as_slice().iter().any(|hull| hull.definition.as_str() == "colony_ark"))
        .then_some(fleet.id)
    }) else {
        return Ok(());
    };
    let hull_id = ctx
        .state
        .fleets
        .as_slice()
        .iter()
        .find(|fleet| fleet.id == fleet_id)
        .and_then(|fleet| {
            fleet
                .hulls
                .as_slice()
                .iter()
                .find(|hull| hull.definition.as_str() == "colony_ark")
                .map(|hull| hull.id)
        });
    if let Some(hull_id) = hull_id
        && let Some(fleet) = ctx.state.fleets.find_mut(&fleet_id)
    {
        remove_key(&mut fleet.hulls, &hull_id)?;
    }
    let empty = ctx
        .state
        .fleets
        .as_slice()
        .iter()
        .find(|fleet| fleet.id == fleet_id)
        .is_some_and(|fleet| fleet.hulls.as_slice().is_empty());
    if empty {
        remove_key(&mut ctx.state.fleets, &fleet_id)?;
    }
    Ok(())
}

fn fleet_hull_hp(
    state: &LivingGalaxyStateV2,
    hull: LivingEntityIdV2,
) -> Option<(LivingEntityIdV2, u32)> {
    state.fleets.as_slice().iter().find_map(|fleet| {
        fleet
            .hulls
            .as_slice()
            .iter()
            .find(|row| row.id == hull)
            .map(|row| (fleet.id, row.hit_points))
    })
}

#[cfg(test)]
fn push_system_worlds(
    state: &LivingGalaxyStateV2,
    system: LivingEntityIdV2,
    worlds: &mut Vec<LivingEntityIdV2>,
) {
    for world in state.worlds.as_slice() {
        if world.system == system {
            push_unique(worlds, world.id);
        }
    }
}

#[cfg(test)]
fn push_unique(worlds: &mut Vec<LivingEntityIdV2>, world: LivingEntityIdV2) {
    if !worlds.contains(&world) {
        worlds.push(world);
    }
}

fn eid(byte: u8) -> LivingEntityIdV2 {
    let mut bytes = [0_u8; 16];
    bytes[0] = byte;
    LivingEntityIdV2(bytes)
}

fn name(value: &str) -> Result<LivingNameV2, String> {
    LivingNameV2::new(value).map_err(|error| error.to_string())
}

fn slug(value: &str) -> Result<LivingSlugV2, String> {
    LivingSlugV2::new(value).map_err(|error| error.to_string())
}

fn world(
    id: LivingEntityIdV2,
    system: LivingEntityIdV2,
    archetype: &str,
    label: &str,
    owner: Option<LivingEntityIdV2>,
    inventory: LivingInventoryV2,
) -> Result<LivingWorldV2, String> {
    Ok(LivingWorldV2 {
        id,
        system,
        archetype: slug(archetype)?,
        name: name(label)?,
        owner,
        inventory,
    })
}

fn colony(world: LivingEntityIdV2, owner: LivingEntityIdV2) -> Result<LivingColonyV2, String> {
    Ok(LivingColonyV2 {
        world,
        owner,
        hub: LivingHubV2 {
            energy_next_due: LivingTickV2(LIVING_HUB_ENERGY_PERIOD_TICKS_V2),
            ore_next_due: LivingTickV2(LIVING_HUB_ORE_PERIOD_TICKS_V2),
            fallback_next_due: LivingTickV2(LIVING_HUB_FALLBACK_PERIOD_TICKS_V2),
            blocked_reasons: LivingSortedVecV2::default(),
        },
    })
}

fn facility(
    id: LivingEntityIdV2,
    world: LivingEntityIdV2,
    definition: &str,
    next_due: Option<LivingTickV2>,
) -> Result<LivingFacilityV2, String> {
    Ok(LivingFacilityV2 {
        id,
        world,
        definition: slug(definition)?,
        status: LivingFacilityStatusV2::Operational,
        paid_alloy: 0,
        hit_points: 0,
        next_due,
        next_repair: None,
        blocked_reasons: LivingSortedVecV2::default(),
        disabled: false,
    })
}

fn hull(
    id: LivingEntityIdV2,
    definition: &str,
    return_credit: bool,
) -> Result<LivingHullV2, String> {
    let hit_points = if definition == "colony_ark" { 20 } else { 10 };
    Ok(LivingHullV2 {
        id,
        definition: slug(definition)?,
        hit_points,
        return_credit,
        next_repair: None,
    })
}

fn fleet_with(
    id: LivingEntityIdV2,
    owner: LivingEntityIdV2,
    world: LivingEntityIdV2,
    hulls: Vec<LivingHullV2>,
) -> Result<LivingFleetV2, String> {
    Ok(LivingFleetV2 {
        id,
        owner,
        location: LivingFleetLocationV2::Docked { world },
        order: None,
        hulls: LivingSortedVecV2::new(hulls).map_err(|error| error.to_string())?,
    })
}

#[cfg(test)]
pub(crate) fn reference_dispatch_for_cursor_test(
    context: &mut LivingStepContextV2<'_>,
) -> Result<(), LivingDeterministicFaultV2> {
    dispatch_orders_and_freight(context)
}

#[cfg(test)]
pub(crate) fn reference_diplomacy_for_cursor_test(
    context: &mut LivingStepContextV2<'_>,
) -> Result<(), LivingDeterministicFaultV2> {
    update_diplomacy(context)
}

#[cfg(test)]
pub(crate) fn reference_occupation_for_cursor_test(
    ctx: &mut LivingStepContextV2<'_>,
) -> Result<(), LivingDeterministicFaultV2> {
    resolve_occupation(ctx)
}
#[cfg(test)]
pub(crate) fn reference_settlement_for_cursor_test(
    ctx: &mut LivingStepContextV2<'_>,
) -> Result<(), LivingDeterministicFaultV2> {
    resolve_claims(ctx)
}

#[cfg(test)]
pub(crate) fn reference_retreat_for_cursor_test(
    ctx: &mut LivingStepContextV2<'_>,
) -> Result<(), LivingDeterministicFaultV2> {
    retreat_noncombatants(ctx)
}

#[cfg(test)]
pub(crate) fn reference_combat_for_cursor_test(
    ctx: &mut LivingStepContextV2<'_>,
) -> Result<(), LivingDeterministicFaultV2> {
    resolve_combat(ctx)
}

#[cfg(test)]
mod intent_reservation_tests {
    use super::*;

    fn fleet_test_economics(
        authority: &LivingGalaxyAuthorityV2,
        state: &LivingGalaxyStateV2,
        count: usize,
    ) -> LivingEconomicEnumerationV2 {
        let tick = LivingTickV2(state.tick.0 + 1);
        let mut normalized = state.clone();
        normalized.tick = tick;
        LivingEconomicEnumerationV2 {
            catalog: authority.catalog_snapshot().into(),
            inputs: LivingIntentInputsV2 {
                snapshot: std::sync::Arc::new(state.clone()).into(),
                digest: living_decision_snapshot_digest_v2(state, tick).unwrap(),
                forecasts: state
                    .civilizations
                    .as_slice()
                    .iter()
                    .filter(|c| c.status == LivingCivilizationStatusV2::Active)
                    .map(|c| {
                        (
                            c.id,
                            LivingScheduledForecastV2 {
                                inventories: Vec::new(),
                                deficits: Vec::new(),
                                idle_facilities: Vec::new(),
                                final_state: state.clone(),
                            },
                        )
                    })
                    .collect(),
                requests: Vec::new(),
                hash_owner: std::sync::Arc::new(normalized).into(),
            }
            .into(),
            graph: Default::default(),
            batches: state
                .civilizations
                .as_slice()
                .iter()
                .filter(|c| c.status == LivingCivilizationStatusV2::Active)
                .map(|c| LivingEconomicBatchV2 {
                    actor: c.id,
                    economic_count: count,
                    ranked: Vec::new(),
                    explanations: Vec::new(),
                })
                .collect(),
        }
    }
    fn fleet_reference_candidates(
        authority: &LivingGalaxyAuthorityV2,
        state: &LivingGalaxyStateV2,
        actor: &LivingCivilizationV2,
        count: usize,
        digest: super::super::ids::LivingStateDigestV2,
    ) -> Vec<LivingFleetCandidateV2> {
        let tick = LivingTickV2(state.tick.0 + 1);
        let mut proposals = Vec::new();
        for emission in 0..4 {
            let mut candidate = state.clone();
            let mut events = super::super::receipt::LivingPendingEventsV2::new();
            let mut ctx = LivingStepContextV2 {
                catalog: authority.catalog(),
                genesis_seed: [0; 32],
                branch: authority.branch(),
                tick,
                state: &mut candidate,
                events: &mut events,
                arrivals: Vec::new(),
                repair_exclusions: Default::default(),
            };
            match emission {
                0 => reserve_settlement(&mut ctx, actor.id, actor.policy.0.as_str()),
                1 => reserve_fleet(&mut ctx, actor.id),
                2 => reserve_guard(&mut ctx, actor.id, true),
                _ => reserve_guard(&mut ctx, actor.id, false),
            }
            .unwrap();
            if let Some(decision) = fleet_proposal_decision(
                state,
                &candidate,
                actor.id,
                actor.policy.0.as_str(),
                tick,
                digest,
            )
            .unwrap()
            {
                let bytes = decision.action.canonical_bytes().unwrap();
                let key = (
                    decision.action.ordinal(),
                    decision.target,
                    bytes.clone(),
                    emission,
                );
                proposals.push((
                    key,
                    LivingFleetCandidateV2 {
                        state: Box::new(candidate),
                        decision,
                        action_bytes: bytes,
                        deltas: Vec::new(),
                    },
                ));
            }
        }
        proposals.sort_by(|a, b| a.0.cmp(&b.0));
        for (index, (_, candidate)) in proposals.iter_mut().enumerate() {
            candidate.decision.ordinal = u16::try_from(count + index).unwrap();
            let created = candidate
                .state
                .fleets
                .as_slice()
                .iter()
                .filter(|f| !state.fleets.as_slice().iter().any(|old| old.id == f.id))
                .cloned()
                .collect::<Vec<_>>();
            for mut fleet in created {
                remove_key(&mut candidate.state.fleets, &fleet.id).unwrap();
                let ordinal = match candidate.decision.action {
                    LivingIntentActionV2::Explore { .. } => 14,
                    LivingIntentActionV2::Settle { .. } => 12,
                    LivingIntentActionV2::Attack { .. } => 13,
                    LivingIntentActionV2::Defend { .. } => {
                        if attacked_owned_world(
                            state,
                            actor.id,
                            candidate.decision.target.unwrap(),
                            tick,
                        ) {
                            10
                        } else {
                            15
                        }
                    }
                    _ => unreachable!(),
                };
                fleet.id = autonomous_entity_id_v2(&LivingAutonomousEntityInputsV2 {
                    catalog_hash: authority.catalog().catalog_hash(),
                    genesis_seed: [0; 32],
                    branch: authority.branch(),
                    tick,
                    phase: LivingPhaseV2::GenerateIntents.ordinal(),
                    actor: actor.id,
                    intent_ordinal: ordinal,
                    entity_kind: LivingEntityKindV2::Fleet.ordinal(),
                    local_id: 0,
                });
                put(&mut candidate.state.fleets, fleet).unwrap();
            }
            let mut ids = std::collections::BTreeSet::new();
            ids.extend(state.fleets.as_slice().iter().map(|f| f.id));
            ids.extend(candidate.state.fleets.as_slice().iter().map(|f| f.id));
            candidate.deltas = ids
                .into_iter()
                .filter_map(|id| {
                    let before_index = state.fleets.as_slice().iter().position(|f| f.id == id);
                    let after_index = candidate
                        .state
                        .fleets
                        .as_slice()
                        .iter()
                        .position(|f| f.id == id);
                    (before_index.map(|i| &state.fleets.as_slice()[i])
                        != after_index.map(|i| &candidate.state.fleets.as_slice()[i]))
                    .then_some(LivingFleetDeltaV2 {
                        id,
                        before_index,
                        after_index,
                    })
                })
                .collect();
        }
        proposals.sort_by(|a, b| {
            (
                fleet_priority(state, tick, &a.1.decision),
                std::cmp::Reverse(a.1.decision.score.0),
                a.1.decision.target,
                &a.0,
            )
                .cmp(&(
                    fleet_priority(state, tick, &b.1.decision),
                    std::cmp::Reverse(b.1.decision.score.0),
                    b.1.decision.target,
                    &b.0,
                ))
        });
        proposals
            .into_iter()
            .map(|(_, candidate)| candidate)
            .collect()
    }
    fn assert_fleet_enumeration_parity(
        authority: &LivingGalaxyAuthorityV2,
        state: &LivingGalaxyStateV2,
        count: usize,
    ) -> LivingFleetEnumerationV2 {
        use super::super::step_job::{
            LivingLifecycleJobErrorV2 as Error, LivingLifecycleProgressV2 as Progress,
        };
        let mut total = None;
        let mut output = None;
        for budgets in [&[1][..], &[64], &[1024], &[1, 64, 7, 1024, 3]] {
            let economics = fleet_test_economics(authority, state, count);
            let expected = state
                .civilizations
                .as_slice()
                .iter()
                .map(|actor| {
                    fleet_reference_candidates(
                        authority,
                        state,
                        actor,
                        if actor.status == LivingCivilizationStatusV2::Active {
                            count
                        } else {
                            0
                        },
                        economics.inputs.digest,
                    )
                })
                .collect::<Vec<_>>();
            let source = std::sync::Arc::clone(&economics.inputs.snapshot);
            let hash = std::sync::Arc::downgrade(&economics.inputs.hash_owner);
            let forecast_pointers = economics
                .inputs
                .forecasts
                .iter()
                .map(|(_, f)| f.final_state.worlds.as_slice().as_ptr())
                .collect::<Vec<_>>();
            let mut job = LivingFleetEnumerationJobV2::begin(
                economics,
                LivingTickV2(state.tick.0 + 1),
                [0; 32],
                authority.branch(),
            );
            assert!(matches!(job.finish(), Err(Error::NotReady)));
            assert_eq!(job.poll(0), Err(Error::Budget));
            assert_eq!(job.poll(1025), Err(Error::Budget));
            let mut calls = 0;
            loop {
                let before = job.total_visits;
                let budget = budgets[calls % budgets.len()];
                let progress = job.poll(budget).unwrap();
                calls += 1;
                assert!(job.last_poll_visits <= budget);
                assert_eq!(job.total_visits - before, job.last_poll_visits as u64);
                assert!(hash.strong_count() > 0);
                assert!(calls < 1_000_000);
                if progress == Progress::Ready {
                    break;
                }
            }
            if let Some(total) = total {
                assert_eq!(job.total_visits, total)
            }
            total = Some(job.total_visits);
            assert!(
                job.worlds.is_empty()
                    && job.colonies.is_empty()
                    && job.summaries.is_empty()
                    && job.attacked.is_empty()
                    && job.known.is_empty()
            );
            assert!(
                job.canonical.is_empty()
                    && job.ranking.is_empty()
                    && job.path.is_none()
                    && job.copy.is_none()
                    && job.assembly.is_none()
            );
            assert!(job.graph.systems.is_empty() && job.graph.lanes.is_empty());
            assert_eq!(job.poll(1), Ok(Progress::Ready));
            assert_eq!(job.last_poll_visits, 0);
            let actual = job.finish().unwrap();
            assert!(matches!(job.finish(), Err(Error::NotReady)));
            assert!(std::sync::Arc::ptr_eq(
                &actual.economics.inputs.snapshot,
                &source
            ));
            assert!(std::sync::Arc::ptr_eq(&actual.graph, &job.graph));
            assert_eq!(
                actual
                    .economics
                    .inputs
                    .forecasts
                    .iter()
                    .map(|(_, f)| f.final_state.worlds.as_slice().as_ptr())
                    .collect::<Vec<_>>(),
                forecast_pointers
            );
            assert_eq!(source.as_ref(), state);
            assert_eq!(
                source.canonical_bytes().unwrap(),
                state.canonical_bytes().unwrap(),
                "source canonical bytes remain unchanged"
            );
            assert_eq!(actual.batches.len(), expected.len());
            for ((batch, expected), actor) in actual
                .batches
                .iter()
                .zip(expected)
                .zip(state.civilizations.as_slice())
            {
                assert_eq!(batch.actor, actor.id);
                assert_eq!(batch.fleets.len(), expected.len());
                assert_eq!(
                    batch.economy.as_ref().map(|b| b.economic_count),
                    if actor.status == LivingCivilizationStatusV2::Active {
                        Some(count)
                    } else {
                        None
                    }
                );
                for (actual, expected) in batch.fleets.iter().zip(expected) {
                    assert_eq!(actual.state, expected.state, "full fleet state");
                    assert_eq!(actual.decision, expected.decision, "exact fleet decision");
                    assert_eq!(actual.action_bytes, expected.action_bytes);
                    assert_eq!(actual.deltas, expected.deltas);
                    let mut unchanged = actual.state.as_ref().clone();
                    unchanged.fleets = state.fleets.clone();
                    assert_eq!(unchanged, *state, "only fleet collection changes");
                }
            }
            output = Some(actual);
        }
        output.unwrap()
    }
    fn fleet_enumeration_fixture() -> (LivingGalaxyAuthorityV2, LivingGalaxyStateV2) {
        let authority = builtin_living_sandbox_v2().unwrap();
        let mut state = authority.state().clone();
        state.tick = LivingTickV2(49);
        for target in [eid(6), eid(8)] {
            state.worlds.find_mut(&target).unwrap().owner = Some(eid(3));
            state.colonies.find_mut(&target).unwrap().owner = eid(3);
        }
        state.fleets = LivingSortedVecV2::new(vec![
            fleet_with(
                eid(90),
                eid(3),
                eid(5),
                vec![
                    hull(eid(91), "colony_ark", false).unwrap(),
                    hull(eid(92), "escort", false).unwrap(),
                    hull(eid(93), "scout", true).unwrap(),
                ],
            )
            .unwrap(),
            fleet_with(
                eid(94),
                eid(3),
                eid(8),
                vec![
                    hull(eid(95), "escort", false).unwrap(),
                    hull(eid(96), "scout", false).unwrap(),
                ],
            )
            .unwrap(),
            fleet_with(
                eid(97),
                eid(4),
                eid(6),
                vec![hull(eid(98), "escort", false).unwrap()],
            )
            .unwrap(),
        ])
        .unwrap();
        put(
            &mut state.wars,
            LivingWarV2 {
                participant_a: eid(3),
                participant_b: eid(4),
                declarer: eid(4),
                start_tick: LivingTickV2(1),
                end_tick: LivingTickV2(500),
            },
        )
        .unwrap();
        state.observations = LivingSortedVecV2::default();
        for (world, owner) in [(eid(6), Some(eid(4))), (eid(7), None)] {
            put(
                &mut state.observations,
                LivingObservationV2 {
                    civilization: eid(3),
                    world,
                    observed_tick: LivingTickV2(20),
                    owner,
                    remaining_deposit: 0,
                    facilities: Default::default(),
                    stationed_hulls: Default::default(),
                    operational_defense_batteries: None,
                },
            )
            .unwrap();
        }
        (authority, state)
    }
    #[test]
    fn fleet_enumeration_four_independent_alternatives_policies_and_two_orders() {
        let (authority, base) = fleet_enumeration_fixture();
        for policy in ["neutral", "expansionist", "guardian", "trader"] {
            let mut state = base.clone();
            state.civilizations.find_mut(&eid(3)).unwrap().policy =
                LivingPolicyV2(slug(policy).unwrap());
            let output = assert_fleet_enumeration_parity(&authority, &state, 7);
            let batch = output.batches.iter().find(|b| b.actor == eid(3)).unwrap();
            assert_eq!(batch.fleets.len(), 4);
            assert!(
                matches!(batch.fleets[0].decision.action,LivingIntentActionV2::Defend{world,..} if world==eid(6))
            );
            assert!(
                batch
                    .fleets
                    .iter()
                    .any(|f| matches!(f.decision.action, LivingIntentActionV2::Settle { .. }))
            );
            assert!(batch.fleets.iter().all(|f| f.decision.ordinal >= 7));
        }
    }
    #[test]
    fn fleet_enumeration_observation_age_fuel_and_dormant_same_world() {
        let (authority, base) = fleet_enumeration_fixture();
        for age in [99, 100, 101] {
            let mut state = base.clone();
            state.tick = LivingTickV2(149);
            state.observations.record_at_mut(0).unwrap().observed_tick = LivingTickV2(150 - age);
            let output = assert_fleet_enumeration_parity(&authority, &state, 0);
            assert_eq!(
                output.batches[0]
                    .fleets
                    .iter()
                    .any(|f| matches!(f.decision.action, LivingIntentActionV2::Attack { .. })),
                age <= 100
            );
        }
        for energy in [1, 2, 3, 4, 9, 10] {
            let mut state = base.clone();
            state.worlds.find_mut(&eid(5)).unwrap().inventory.energy = energy;
            assert_fleet_enumeration_parity(&authority, &state, 0);
        }
        let mut state = base;
        state.civilizations.find_mut(&eid(3)).unwrap().status = LivingCivilizationStatusV2::Dormant;
        for hull in state
            .fleets
            .find_mut(&eid(90))
            .unwrap()
            .hulls
            .as_slice()
            .to_vec()
        {
            state
                .fleets
                .find_mut(&eid(90))
                .unwrap()
                .hulls
                .find_mut(&hull.id)
                .unwrap()
                .return_credit = true;
        }
        state.worlds.find_mut(&eid(5)).unwrap().inventory.energy = 0;
        state.colonies.find_mut(&eid(5)).unwrap().owner = eid(4);
        state.observations = LivingSortedVecV2::new(vec![LivingObservationV2 {
            civilization: eid(3),
            world: eid(5),
            observed_tick: LivingTickV2(0),
            owner: None,
            remaining_deposit: 0,
            facilities: Default::default(),
            stationed_hulls: Default::default(),
            operational_defense_batteries: None,
        }])
        .unwrap();
        let output = assert_fleet_enumeration_parity(&authority, &state, 5);
        assert_eq!(output.batches[0].fleets.len(), 1);
        assert_eq!(
            output.batches[0].fleets[0].decision.tier,
            LivingDecisionTierV2::Recovery
        );
        state
            .fleets
            .find_mut(&eid(90))
            .unwrap()
            .hulls
            .record_at_mut(0)
            .unwrap()
            .return_credit = false;
        assert!(
            assert_fleet_enumeration_parity(&authority, &state, 0).batches[0]
                .fleets
                .is_empty()
        );
    }

    fn fleet_numbered_id(prefix: u8, index: u16) -> LivingEntityIdV2 {
        let mut bytes = [0; 16];
        bytes[0] = prefix;
        bytes[14..].copy_from_slice(&index.to_be_bytes());
        LivingEntityIdV2(bytes)
    }
    fn fleet_test_job(
        authority: &LivingGalaxyAuthorityV2,
        state: &LivingGalaxyStateV2,
        count: usize,
    ) -> LivingFleetEnumerationJobV2 {
        LivingFleetEnumerationJobV2::begin(
            fleet_test_economics(authority, state, count),
            LivingTickV2(state.tick.0 + 1),
            [0; 32],
            authority.branch(),
        )
    }
    #[test]
    fn fleet_enumeration_first_scout_no_target_does_not_retry_later_fleet() {
        let (authority, mut state) = fleet_enumeration_fixture();
        state.systems = LivingSortedVecV2::new(vec![
            LivingSystemV2 {
                id: eid(30),
                name: name("First").unwrap(),
            },
            LivingSystemV2 {
                id: eid(31),
                name: name("Second").unwrap(),
            },
        ])
        .unwrap();
        state.lanes = Default::default();
        for (world, system) in [
            (eid(5), eid(30)),
            (eid(6), eid(30)),
            (eid(7), eid(31)),
            (eid(8), eid(31)),
        ] {
            state.worlds.find_mut(&world).unwrap().system = system;
        }
        state.fleets = LivingSortedVecV2::new(vec![
            fleet_with(
                eid(90),
                eid(3),
                eid(5),
                vec![hull(eid(91), "scout", false).unwrap()],
            )
            .unwrap(),
            fleet_with(
                eid(94),
                eid(3),
                eid(8),
                vec![hull(eid(95), "scout", false).unwrap()],
            )
            .unwrap(),
        ])
        .unwrap();
        state.observations = LivingSortedVecV2::new(
            [eid(6), eid(8)]
                .into_iter()
                .map(|world| LivingObservationV2 {
                    civilization: eid(3),
                    world,
                    observed_tick: LivingTickV2(50),
                    owner: Some(eid(3)),
                    remaining_deposit: 0,
                    facilities: Default::default(),
                    stationed_hulls: Default::default(),
                    operational_defense_batteries: None,
                })
                .collect(),
        )
        .unwrap();
        assert!(
            assert_fleet_enumeration_parity(&authority, &state, 0).batches[0]
                .fleets
                .is_empty()
        );
        state.fleets.find_mut(&eid(90)).unwrap().order = Some(LivingFleetOrderV2 {
            kind: LivingFleetOrderKindV2::Move,
            target_world: Some(eid(5)),
            not_before_tick: LivingTickV2(60),
            lane_path: vec![eid(80); 64],
        });
        let output = assert_fleet_enumeration_parity(&authority, &state, 0);
        assert_eq!(output.batches[0].fleets.len(), 1);
        assert!(
            matches!(output.batches[0].fleets[0].decision.action,LivingIntentActionV2::Explore{fleet,destination} if fleet==eid(94) && destination==eid(7))
        );
    }
    #[test]
    fn fleet_enumeration_oldest_known_target_and_exact_affordable_scout_split() {
        let (authority, mut state) = fleet_enumeration_fixture();
        state.wars = Default::default();
        state.fleets = LivingSortedVecV2::new(vec![
            fleet_with(
                eid(90),
                eid(3),
                eid(5),
                vec![
                    hull(eid(91), "scout", false).unwrap(),
                    hull(eid(92), "scout", true).unwrap(),
                ],
            )
            .unwrap(),
        ])
        .unwrap();
        state.worlds.find_mut(&eid(5)).unwrap().inventory.energy = 2;
        let system = state
            .worlds
            .as_slice()
            .iter()
            .find(|w| w.id == eid(7))
            .unwrap()
            .system;
        for world in [eid(6), eid(8)] {
            state.worlds.find_mut(&world).unwrap().system = system;
        }
        state.observations = LivingSortedVecV2::new(
            [(eid(6), 20), (eid(7), 10), (eid(8), 50)]
                .into_iter()
                .map(|(world, tick)| LivingObservationV2 {
                    civilization: eid(3),
                    world,
                    observed_tick: LivingTickV2(tick),
                    owner: Some(eid(3)),
                    remaining_deposit: 0,
                    facilities: Default::default(),
                    stationed_hulls: Default::default(),
                    operational_defense_batteries: None,
                })
                .collect(),
        )
        .unwrap();
        let output = assert_fleet_enumeration_parity(&authority, &state, 0);
        let proposal = &output.batches[0].fleets[0];
        assert!(
            matches!(proposal.decision.action,LivingIntentActionV2::Explore{destination,..} if destination==eid(7))
        );
        assert_eq!(
            proposal
                .state
                .fleets
                .as_slice()
                .iter()
                .find(|f| f.id == eid(90))
                .unwrap()
                .hulls
                .as_slice()[0]
                .id,
            eid(91)
        );
        assert_eq!(
            proposal
                .state
                .fleets
                .as_slice()
                .iter()
                .find(|f| f.order.is_some())
                .unwrap()
                .hulls
                .as_slice()[0]
                .id,
            eid(92)
        );
    }
    #[test]
    fn fleet_enumeration_capacity_and_paid_jobs_select_whole_escort_role() {
        let (authority, base) = fleet_enumeration_fixture();
        for pending in [252, 253, 254] {
            let mut state = base.clone();
            state.hull_jobs = LivingSortedVecV2::new(
                (0..pending)
                    .map(|index| LivingHullJobV2 {
                        id: fleet_numbered_id(240, index),
                        world: eid(5),
                        owner: eid(3),
                        definition: slug("scout").unwrap(),
                        accepted_tick: LivingTickV2(1),
                        completion_tick: LivingTickV2(1000),
                        paid_alloy: 1,
                        target_fleet: None,
                    })
                    .collect(),
            )
            .unwrap();
            let output = assert_fleet_enumeration_parity(&authority, &state, 0);
            let attack = output.batches[0]
                .fleets
                .iter()
                .find(|f| matches!(f.decision.action, LivingIntentActionV2::Attack { .. }))
                .unwrap();
            let ordered = attack
                .state
                .fleets
                .as_slice()
                .iter()
                .find(|f| {
                    matches!(
                        f.order.as_ref().map(|o| o.kind),
                        Some(LivingFleetOrderKindV2::Attack)
                    )
                })
                .unwrap();
            assert_eq!(
                ordered.hulls.as_slice().len(),
                if pending >= 253 { 3 } else { 1 }
            );
        }
        let mut state = base;
        state.wars = Default::default();
        state.observations = Default::default();
        let mut fleets = state.fleets.clone().into_vec();
        for index in 0..253 {
            fleets.push(
                fleet_with(
                    fleet_numbered_id(240, index),
                    eid(4),
                    eid(7),
                    vec![hull(fleet_numbered_id(241, index), "scout", false).unwrap()],
                )
                .unwrap(),
            );
        }
        state.fleets = LivingSortedVecV2::new(fleets).unwrap();
        let output = assert_fleet_enumeration_parity(&authority, &state, 0);
        let explore = output.batches[0]
            .fleets
            .iter()
            .find(|f| matches!(f.decision.action, LivingIntentActionV2::Explore { .. }))
            .unwrap();
        assert_eq!(
            explore.state.fleets.as_slice().len(),
            257,
            "full capacity exploration is preserved for later rejection"
        );
        assert!(explore.state.fleets.as_slice().iter().any(|f| {
            f.order
                .as_ref()
                .is_some_and(|o| o.kind == LivingFleetOrderKindV2::Explore)
                && f.hulls.as_slice().len() == 1
        }));
    }
    #[test]
    fn fleet_enumeration_busy_donors_and_guardian_reinforcement_priority() {
        let (authority, base) = fleet_enumeration_fixture();
        for blocked in [eid(90), eid(94)] {
            let mut state = base.clone();
            put(
                &mut state.hull_jobs,
                LivingHullJobV2 {
                    id: eid(120),
                    world: eid(5),
                    owner: eid(3),
                    definition: slug("escort").unwrap(),
                    accepted_tick: LivingTickV2(1),
                    completion_tick: LivingTickV2(1000),
                    paid_alloy: 1,
                    target_fleet: Some(blocked),
                },
            )
            .unwrap();
            assert_fleet_enumeration_parity(&authority, &state, 0);
            state.hull_jobs = Default::default();
            state.fleets.find_mut(&blocked).unwrap().order = Some(LivingFleetOrderV2 {
                kind: LivingFleetOrderKindV2::Move,
                target_world: Some(eid(5)),
                not_before_tick: LivingTickV2(80),
                lane_path: vec![eid(100); 17],
            });
            assert_fleet_enumeration_parity(&authority, &state, 0);
        }
        let mut state = base;
        state.wars = Default::default();
        state.civilizations.find_mut(&eid(3)).unwrap().policy =
            LivingPolicyV2(slug("guardian").unwrap());
        let output = assert_fleet_enumeration_parity(&authority, &state, 0);
        let batch = &output.batches[0];
        let reinforcement = batch
            .fleets
            .iter()
            .find(|f| matches!(f.decision.action, LivingIntentActionV2::Defend { .. }))
            .unwrap();
        assert_eq!(reinforcement.decision.score.0, 60);
        assert_eq!(
            fleet_priority(&state, LivingTickV2(50), &reinforcement.decision),
            2
        );
        assert!(
            matches!(
                batch.fleets[0].decision.action,
                LivingIntentActionV2::Defend { .. }
            ),
            "same score retains canonical action ordering"
        );
    }
    fn fleet_collision_fixture(
        ordinal: u16,
        identical_order: bool,
    ) -> (
        LivingGalaxyAuthorityV2,
        LivingGalaxyStateV2,
        LivingEntityIdV2,
    ) {
        let (authority, mut state) = fleet_enumeration_fixture();
        state.wars = Default::default();
        state.observations = Default::default();
        state.colonies = LivingSortedVecV2::new(vec![colony(eid(5), eid(3)).unwrap()]).unwrap();
        state.fleets = LivingSortedVecV2::new(vec![
            fleet_with(
                eid(90),
                eid(3),
                eid(5),
                vec![
                    hull(eid(91), "scout", false).unwrap(),
                    hull(eid(92), "scout", true).unwrap(),
                ],
            )
            .unwrap(),
        ])
        .unwrap();
        let target = fleet_reference_candidates(
            &authority,
            &state,
            &state.civilizations.as_slice()[0],
            0,
            living_decision_snapshot_digest_v2(&state, LivingTickV2(50)).unwrap(),
        )[0]
        .decision
        .target
        .unwrap();
        let collision = autonomous_entity_id_v2(&LivingAutonomousEntityInputsV2 {
            catalog_hash: authority.catalog().catalog_hash(),
            genesis_seed: [0; 32],
            branch: authority.branch(),
            tick: LivingTickV2(50),
            phase: LivingPhaseV2::GenerateIntents.ordinal(),
            actor: eid(3),
            intent_ordinal: ordinal,
            entity_kind: LivingEntityKindV2::Fleet.ordinal(),
            local_id: 0,
        });
        let mut row = fleet_with(
            collision,
            if identical_order { eid(3) } else { eid(4) },
            eid(5),
            vec![hull(eid(150), "scout", false).unwrap()],
        )
        .unwrap();
        row.order = Some(LivingFleetOrderV2 {
            kind: if identical_order {
                LivingFleetOrderKindV2::Explore
            } else {
                LivingFleetOrderKindV2::Move
            },
            target_world: Some(target),
            not_before_tick: LivingTickV2(if identical_order { 51 } else { 100 }),
            lane_path: if identical_order {
                Vec::new()
            } else {
                vec![eid(100); 512]
            },
        });
        put(&mut state.fleets, row).unwrap();
        (authority, state, collision)
    }
    #[test]
    fn fleet_enumeration_identity_replacements_no_decision_and_rejected_ids() {
        for ordinal in [2, 14] {
            let (authority, state, collision) = fleet_collision_fixture(ordinal, false);
            let output = assert_fleet_enumeration_parity(&authority, &state, 0);
            let proposal = &output.batches[0].fleets[0];
            assert_eq!(proposal.decision.outcome, LivingDecisionOutcomeV2::Rejected);
            assert_eq!(
                proposal
                    .state
                    .fleets
                    .as_slice()
                    .iter()
                    .find(|f| f.id == collision)
                    .unwrap()
                    .owner,
                eid(3)
            );
            if ordinal == 2 {
                assert!(
                    matches!(proposal.decision.action,LivingIntentActionV2::Explore{fleet,..} if fleet==collision)
                );
            }
            let shifted = assert_fleet_enumeration_parity(&authority, &state, 123);
            assert_eq!(
                proposal.state, shifted.batches[0].fleets[0].state,
                "identity never uses explanation ordinal"
            );
            assert_eq!(shifted.batches[0].fleets[0].decision.ordinal, 123);
        }
        let (authority, state, _) = fleet_collision_fixture(2, true);
        let output = assert_fleet_enumeration_parity(&authority, &state, 0);
        assert!(output.batches[0].fleets.is_empty());
    }
    #[test]
    fn fleet_enumeration_equal_canonical_keys_keep_emission_order_duplicates() {
        let (authority, state) = fleet_enumeration_fixture();
        let output = assert_fleet_enumeration_parity(&authority, &state, 0);
        let guards = output.batches[0]
            .fleets
            .iter()
            .filter(|f| matches!(f.decision.action, LivingIntentActionV2::Defend { .. }))
            .collect::<Vec<_>>();
        assert_eq!(guards.len(), 2);
        assert_eq!(guards[0].action_bytes, guards[1].action_bytes);
        assert_eq!(guards[0].state, guards[1].state);
        assert_eq!(guards[0].decision.ordinal + 1, guards[1].decision.ordinal);
    }
    fn fleet_retained_shape(
        job: &LivingFleetEnumerationJobV2,
    ) -> (usize, usize, usize, usize, usize, usize, usize, usize) {
        (
            job.economics.as_ref().map_or(0, |e| {
                e.inputs.forecasts.len() + e.inputs.snapshot.fleets.as_slice().len()
            }),
            job.copy.as_ref().map_or(0, |j| j.total_visits() as usize),
            job.path.as_ref().map_or(0, |j| {
                let (a, b, c, d) = j.scratch_path_lengths();
                a + b + c + d
            }),
            job.assembly.as_ref().map_or(0, |a| {
                a.source.len()
                    + a.rows.len()
                    + a.hulls.len()
                    + a.moving.as_slice().len()
                    + a.remaining.as_slice().len()
                    + a.retired.len()
                    + a.retired_hulls.len()
                    + a.retired_path.len()
                    + a.output.as_slice().len()
            }),
            job.candidate
                .as_ref()
                .map_or(0, |c| c.fleets.as_slice().len()),
            job.canonical.len(),
            job.ranking.len(),
            job.delta.as_ref().map_or(0, |d| d.output.len()),
        )
    }
    #[test]
    fn fleet_enumeration_cancel_copy_search_partition_rekey_and_delta_retains_owners() {
        use super::super::step_job::{
            LivingLifecycleJobErrorV2 as Error, LivingLifecycleProgressV2 as Progress,
        };
        let (authority, mut state) = fleet_enumeration_fixture();
        state.systems = LivingSortedVecV2::new(vec![
            LivingSystemV2 {
                id: eid(30),
                name: name("Home").unwrap(),
            },
            LivingSystemV2 {
                id: eid(31),
                name: name("Frontier").unwrap(),
            },
        ])
        .unwrap();
        state.lanes = LivingSortedVecV2::new(vec![LivingLaneV2 {
            id: eid(120),
            system_a: eid(30),
            system_b: eid(31),
            distance_units: 100,
        }])
        .unwrap();
        for world in state.worlds.as_slice().to_vec() {
            state.worlds.find_mut(&world.id).unwrap().system =
                if world.id == eid(7) { eid(31) } else { eid(30) };
        }
        for stage in [
            FleetEnumerationStage::Path,
            FleetEnumerationStage::Copy,
            FleetEnumerationStage::Assemble,
            FleetEnumerationStage::Rekey,
            FleetEnumerationStage::Delta,
        ] {
            let mut job = fleet_test_job(&authority, &state, 0);
            let source = std::sync::Arc::clone(&job.economics.as_ref().unwrap().inputs.snapshot);
            let mut calls = 0;
            loop {
                let partial = job.stage == stage
                    && (stage != FleetEnumerationStage::Assemble
                        || job.assembly.as_ref().is_some_and(|a| {
                            a.stage == 2
                                && a.hulls.len() > 0
                                && (!a.moving.as_slice().is_empty()
                                    || !a.remaining.as_slice().is_empty())
                        }))
                    && (stage != FleetEnumerationStage::Rekey
                        || job
                            .assembly
                            .as_ref()
                            .is_some_and(|a| a.stage >= 4 && !a.ready()));
                if partial {
                    break;
                }
                assert_eq!(
                    job.poll(1).unwrap(),
                    Progress::Pending,
                    "checkpoint {stage:?} was not reached"
                );
                calls += 1;
                assert!(calls < 100_000);
            }
            let before = fleet_retained_shape(&job);
            let total = job.total_visits;
            assert!(matches!(job.finish(), Err(Error::NotReady)));
            assert_eq!(fleet_retained_shape(&job), before);
            job.cancel();
            assert_eq!(job.poll(1024), Err(Error::Cancelled));
            assert_eq!(job.last_poll_visits, 0);
            assert_eq!(job.total_visits, total);
            assert!(matches!(job.finish(), Err(Error::Cancelled)));
            assert_eq!(fleet_retained_shape(&job), before);
            assert_eq!(source.as_ref(), &state);
        }
    }
    #[test]
    fn fleet_enumeration_sticky_ordinal_and_order_tick_faults_charge_attempts() {
        use super::super::step_job::LivingLifecycleJobErrorV2 as Error;
        let (authority, base) = fleet_enumeration_fixture();
        for (mut state, count) in [(base.clone(), 65536), (base, 0)] {
            if count == 0 {
                state.tick = LivingTickV2(u64::MAX - 1);
            }
            let mut job = fleet_test_job(&authority, &state, count);
            let source = std::sync::Arc::clone(&job.economics.as_ref().unwrap().inputs.snapshot);
            let fault = loop {
                if let Err(error) = job.poll(1) {
                    break error;
                }
            };
            assert_eq!(fault, Error::Fault(LivingDeterministicFaultV2::Arithmetic));
            assert_eq!(job.last_poll_visits, 1);
            let before = fleet_retained_shape(&job);
            let total = job.total_visits;
            assert_eq!(job.poll(1), Err(fault.clone()));
            assert_eq!(job.last_poll_visits, 0);
            assert_eq!(job.total_visits, total);
            assert!(matches!(
                job.finish(),
                Err(Error::Fault(LivingDeterministicFaultV2::Arithmetic))
            ));
            assert_eq!(fleet_retained_shape(&job), before);
            assert_eq!(source.as_ref(), &state);
        }
    }

    #[test]
    fn fleet_enumeration_long_path_counts_and_owned_upsert_nested_retirement() {
        use super::super::step_job::LivingLifecycleProgressV2 as Progress;
        let (authority, mut state) = fleet_enumeration_fixture();
        let mut lanes: Vec<_> = (0..20).map(|i| (i, i + 1, 100)).collect();
        lanes.extend([(20, 21, 100), (20, 22, 100), (21, 23, 100), (22, 23, 100)]);
        let graph = exact_path_state(24, &lanes);
        state.systems = graph.systems;
        state.lanes = graph.lanes;
        for world in state.worlds.as_slice().to_vec() {
            state.worlds.find_mut(&world.id).unwrap().system =
                if [eid(6), eid(7)].contains(&world.id) {
                    eid(55)
                } else {
                    eid(32)
                };
        }
        assert_fleet_enumeration_parity(&authority, &state, 0);
        let mut job = fleet_test_job(&authority, &state, 0);
        let mut path_visits = 0;
        loop {
            let path = job.stage == FleetEnumerationStage::Path;
            let progress = job.poll(1).unwrap();
            if path {
                path_visits += job.last_poll_visits;
            }
            if progress == Progress::Ready {
                break;
            }
        }
        assert!(
            path_visits > 1024,
            "individual nested search visits, not whole-search accounting"
        );
        let mut selected = fleet_with(
            eid(90),
            eid(3),
            eid(5),
            (1..=16)
                .map(|i| hull(eid(i), "scout", false).unwrap())
                .collect(),
        )
        .unwrap();
        selected.order = Some(LivingFleetOrderV2 {
            kind: LivingFleetOrderKindV2::Move,
            target_world: Some(eid(5)),
            not_before_tick: LivingTickV2(60),
            lane_path: vec![eid(180); 17],
        });
        let mut collision = fleet_with(
            eid(91),
            eid(4),
            eid(6),
            (20..28)
                .map(|i| hull(eid(i), "escort", false).unwrap())
                .collect(),
        )
        .unwrap();
        collision.order = Some(LivingFleetOrderV2 {
            kind: LivingFleetOrderKindV2::Move,
            target_world: Some(eid(6)),
            not_before_tick: LivingTickV2(60),
            lane_path: vec![eid(180); 512],
        });
        let mut assembly = LivingFleetAssemblyCursorV2::begin(
            LivingSortedVecV2::new(vec![selected.clone(), collision]).unwrap(),
            FleetAssemblyChange::Order {
                fleet: eid(90),
                target: eid(7),
                kind: LivingFleetOrderKindV2::Explore,
                scout: Some(eid(2)),
                all: false,
                split: eid(91),
                tick: LivingTickV2(50),
            },
        );
        let mut partition = 0;
        let mut cleanup_visits = 0;
        while !assembly.ready() {
            let stage = assembly.stage;
            let hulls = assembly.hulls.len();
            let cleanup = !assembly.retirement.is_empty();
            let visited = assembly.visit().unwrap();
            if cleanup && visited {
                cleanup_visits += 1;
            }
            if stage == 2 && visited && !cleanup {
                partition += 1;
                assert_eq!(assembly.hulls.len() + 1, hulls);
            }
        }
        assert_eq!(partition, 16);
        assert!(
            cleanup_visits >= 8 + 512,
            "collision hull and path records retire before successor"
        );
        let fleets = assembly.finish();
        assert_eq!(fleets.as_slice()[0].hulls.as_slice().len(), 15);
        assert_eq!(
            fleets.as_slice()[0].order,
            selected.order,
            "retained fleet's order is unchanged during a split"
        );
        assert_eq!(fleets.as_slice()[1].owner, eid(3));
        assert_eq!(fleets.as_slice()[1].hulls.as_slice()[0].id, eid(2));
        let mut rekey = LivingFleetAssemblyCursorV2::begin(
            fleets,
            FleetAssemblyChange::Rekey {
                fleet: eid(91),
                id: eid(90),
            },
        );
        while !rekey.ready() {
            rekey.visit().unwrap();
        }
        assert!(
            rekey.retired.is_empty()
                && rekey.retired_hulls.len() == 0
                && rekey.retired_path.is_empty()
        );
        let fleets = rekey.finish();
        assert_eq!(fleets.as_slice().len(), 1);
        assert_eq!(fleets.as_slice()[0].id, eid(90));
        assert_eq!(fleets.as_slice()[0].hulls.as_slice()[0].id, eid(2));
    }
    #[test]
    fn fleet_enumeration_consumes_real_economics_and_retains_original_owners() {
        use super::super::step_job::LivingLifecycleProgressV2 as Progress;
        let authority = builtin_living_sandbox_v2().unwrap();
        let state = authority.state().clone();
        let tick = LivingTickV2(state.tick.0 + 1);
        let inputs = economic_test_inputs(&authority, std::sync::Arc::new(state.clone()));
        let source = std::sync::Arc::clone(&inputs.snapshot);
        let hash = std::sync::Arc::downgrade(&inputs.hash_owner);
        let mut economic =
            LivingEconomicEnumerationJobV2::begin(authority.catalog_snapshot(), inputs, tick);
        while economic.poll(64).unwrap() != Progress::Ready {}
        let economics = economic.finish().unwrap();
        let expected = economics
            .batches
            .iter()
            .map(|b| {
                (
                    b.actor,
                    b.economic_count,
                    b.ranked
                        .iter()
                        .map(|(ordinal, c)| {
                            (
                                *ordinal,
                                LivingEconomicCandidateV2 {
                                    tier: c.tier,
                                    score: c.score,
                                    target: c.target,
                                    action: c.action.clone(),
                                },
                            )
                        })
                        .collect::<Vec<_>>(),
                    b.explanations.clone(),
                )
            })
            .collect::<Vec<_>>();
        let digest = economics.inputs.digest;
        let mut fleet =
            LivingFleetEnumerationJobV2::begin(economics, tick, [0; 32], authority.branch());
        while fleet.poll(64).unwrap() != Progress::Ready {}
        let output = fleet.finish().unwrap();
        assert!(std::sync::Arc::ptr_eq(
            &output.economics.inputs.snapshot,
            &source
        ));
        assert!(hash.strong_count() > 0);
        for (actor, count, ranked, explanations) in expected {
            let batch = output.batches.iter().find(|b| b.actor == actor).unwrap();
            let economy = batch.economy.as_ref().unwrap();
            assert_eq!(economy.economic_count, count);
            assert!(economy.ranked == ranked);
            assert_eq!(economy.explanations, explanations);
            let reference = fleet_reference_candidates(
                &authority,
                &state,
                state
                    .civilizations
                    .as_slice()
                    .iter()
                    .find(|c| c.id == actor)
                    .unwrap(),
                count,
                digest,
            );
            for (actual, reference) in batch.fleets.iter().zip(&reference) {
                assert_eq!(actual.state, reference.state);
                assert_eq!(actual.decision, reference.decision);
                assert_eq!(actual.deltas, reference.deltas);
            }
            assert_eq!(batch.fleets.len(), reference.len());
        }
        assert_eq!(authority.state(), &state);
        assert_eq!(source.as_ref(), &state);
    }

    #[test]
    fn fleet_enumeration_ordinal_boundary_empty_actors_and_busy_zero_hp_presence() {
        use super::super::step_job::LivingLifecycleJobErrorV2 as Error;
        let (authority, mut state, _) = fleet_collision_fixture(14, false);
        state.fleets = LivingSortedVecV2::new(vec![
            state
                .fleets
                .as_slice()
                .iter()
                .find(|f| f.id == eid(90))
                .unwrap()
                .clone(),
        ])
        .unwrap();
        let output = assert_fleet_enumeration_parity(&authority, &state, 65535);
        assert_eq!(output.batches[0].fleets.len(), 1);
        assert_eq!(output.batches[0].fleets[0].decision.ordinal, u16::MAX);
        state.fleets = Default::default();
        assert!(
            assert_fleet_enumeration_parity(&authority, &state, 65536)
                .batches
                .iter()
                .all(|b| b.fleets.is_empty())
        );
        let inputs = fleet_test_economics(&authority, &state, 0);
        let mut invalid = LivingFleetEnumerationJobV2::begin(
            inputs,
            LivingTickV2(52),
            [0; 32],
            authority.branch(),
        );
        assert_eq!(
            invalid.poll(1),
            Err(Error::Fault(LivingDeterministicFaultV2::Decision(
                super::super::decision::LivingDecisionErrorV2::Snapshot
            )))
        );
        assert_eq!(invalid.last_poll_visits, 0);
        assert!(matches!(invalid.finish(), Err(Error::Fault(_))));
        assert!(invalid.economics.is_some());
        state.civilizations = Default::default();
        assert!(
            assert_fleet_enumeration_parity(&authority, &state, 0)
                .batches
                .is_empty()
        );
        let (authority, mut state) = fleet_enumeration_fixture();
        state.wars = Default::default();
        let mut friendly = fleet_with(
            eid(99),
            eid(3),
            eid(6),
            vec![hull(eid(100), "escort", false).unwrap()],
        )
        .unwrap();
        friendly.hulls.record_at_mut(0).unwrap().hit_points = 0;
        friendly.order = Some(LivingFleetOrderV2 {
            kind: LivingFleetOrderKindV2::Move,
            target_world: Some(eid(5)),
            not_before_tick: LivingTickV2(100),
            lane_path: Vec::new(),
        });
        put(&mut state.fleets, friendly).unwrap();
        put(
            &mut state.hull_jobs,
            LivingHullJobV2 {
                id: eid(120),
                world: eid(6),
                owner: eid(3),
                definition: slug("escort").unwrap(),
                accepted_tick: LivingTickV2(1),
                completion_tick: LivingTickV2(1000),
                paid_alloy: 1,
                target_fleet: Some(eid(99)),
            },
        )
        .unwrap();
        let output = assert_fleet_enumeration_parity(&authority, &state, 0);
        assert!(
            !output.batches[0]
                .fleets
                .iter()
                .any(|f| matches!(f.decision.action, LivingIntentActionV2::Defend { .. })),
            "any friendly docked escort suppresses reinforcement, including busy targeted zero-HP hulls"
        );
    }
    #[test]
    fn fleet_enumeration_delta_indexes_include_added_changed_and_deleted_rows() {
        let (_, mut before) = fleet_enumeration_fixture();
        before.fleets = LivingSortedVecV2::new(vec![
            fleet_with(
                eid(90),
                eid(3),
                eid(5),
                vec![hull(eid(1), "scout", false).unwrap()],
            )
            .unwrap(),
            fleet_with(
                eid(91),
                eid(3),
                eid(5),
                vec![hull(eid(2), "escort", false).unwrap()],
            )
            .unwrap(),
        ])
        .unwrap();
        before.fleets.record_at_mut(1).unwrap().order = Some(LivingFleetOrderV2 {
            kind: LivingFleetOrderKindV2::Move,
            target_world: Some(eid(6)),
            not_before_tick: LivingTickV2(50),
            lane_path: vec![eid(10), eid(11)],
        });
        let mut after = before.clone();
        let mut changed = before.fleets.as_slice()[1].clone();
        changed.owner = eid(4);
        changed.hulls.record_at_mut(0).unwrap().return_credit = true;
        changed.order.as_mut().unwrap().lane_path[1] = eid(12);
        after.fleets = LivingSortedVecV2::new(vec![
            changed,
            fleet_with(
                eid(92),
                eid(3),
                eid(5),
                vec![hull(eid(3), "scout", true).unwrap()],
            )
            .unwrap(),
        ])
        .unwrap();
        let mut cursor = LivingFleetDeltaCursorV2::begin();
        let mut nested_hulls = 0;
        let mut path_elements = 0;
        while cursor.stage != 4 {
            let stage = cursor.stage;
            let visited = cursor.visit(&before, &after);
            if stage == 1 && visited {
                path_elements += 1;
            }
            if stage == 2 && visited {
                nested_hulls += 1;
            }
        }
        assert_eq!((path_elements, nested_hulls), (2, 3));
        assert_eq!(
            cursor.output,
            vec![
                LivingFleetDeltaV2 {
                    id: eid(90),
                    before_index: Some(0),
                    after_index: None
                },
                LivingFleetDeltaV2 {
                    id: eid(91),
                    before_index: Some(1),
                    after_index: Some(0)
                },
                LivingFleetDeltaV2 {
                    id: eid(92),
                    before_index: None,
                    after_index: Some(1)
                }
            ]
        );
    }

    #[test]
    fn fleet_enumeration_unseen_system_first_and_active_same_world_fuel_threshold() {
        let (authority, mut state) = fleet_enumeration_fixture();
        state.wars = Default::default();
        state.colonies = LivingSortedVecV2::new(vec![colony(eid(5), eid(3)).unwrap()]).unwrap();
        state.fleets = LivingSortedVecV2::new(vec![
            fleet_with(
                eid(90),
                eid(3),
                eid(5),
                vec![hull(eid(91), "scout", true).unwrap()],
            )
            .unwrap(),
        ])
        .unwrap();
        state.systems = LivingSortedVecV2::new(
            (30..33)
                .map(|i| LivingSystemV2 {
                    id: eid(i),
                    name: name(&format!("System {i}")).unwrap(),
                })
                .collect(),
        )
        .unwrap();
        state.lanes = LivingSortedVecV2::new(vec![
            LivingLaneV2 {
                id: eid(120),
                system_a: eid(30),
                system_b: eid(31),
                distance_units: 100,
            },
            LivingLaneV2 {
                id: eid(121),
                system_a: eid(30),
                system_b: eid(32),
                distance_units: 10000,
            },
        ])
        .unwrap();
        for (world, system) in [
            (eid(5), eid(30)),
            (eid(6), eid(31)),
            (eid(7), eid(32)),
            (eid(8), eid(31)),
        ] {
            state.worlds.find_mut(&world).unwrap().system = system;
        }
        state.observations = LivingSortedVecV2::new(
            [(eid(6), 1), (eid(8), 50)]
                .into_iter()
                .map(|(world, tick)| LivingObservationV2 {
                    civilization: eid(3),
                    world,
                    observed_tick: LivingTickV2(tick),
                    owner: Some(eid(3)),
                    remaining_deposit: 0,
                    facilities: Default::default(),
                    stationed_hulls: Default::default(),
                    operational_defense_batteries: None,
                })
                .collect(),
        )
        .unwrap();
        let output = assert_fleet_enumeration_parity(&authority, &state, 0);
        assert_eq!(
            output.batches[0].fleets[0].decision.target,
            Some(eid(7)),
            "unseen system wins despite greater duration"
        );
        put(
            &mut state.observations,
            LivingObservationV2 {
                civilization: eid(3),
                world: eid(7),
                observed_tick: LivingTickV2(2),
                owner: Some(eid(3)),
                remaining_deposit: 0,
                facilities: Default::default(),
                stationed_hulls: Default::default(),
                operational_defense_batteries: None,
            },
        )
        .unwrap();
        assert_eq!(
            assert_fleet_enumeration_parity(&authority, &state, 0).batches[0].fleets[0]
                .decision
                .target,
            Some(eid(6)),
            "every remembered observation establishes known system"
        );
        state.fleets = LivingSortedVecV2::new(vec![
            fleet_with(
                eid(90),
                eid(3),
                eid(5),
                vec![
                    hull(eid(91), "colony_ark", false).unwrap(),
                    hull(eid(92), "escort", true).unwrap(),
                    hull(eid(93), "scout", true).unwrap(),
                ],
            )
            .unwrap(),
        ])
        .unwrap();
        state.observations = LivingSortedVecV2::new(vec![LivingObservationV2 {
            civilization: eid(3),
            world: eid(5),
            observed_tick: LivingTickV2(0),
            owner: None,
            remaining_deposit: 0,
            facilities: Default::default(),
            stationed_hulls: Default::default(),
            operational_defense_batteries: None,
        }])
        .unwrap();
        for energy in [7, 8] {
            state.worlds.find_mut(&eid(5)).unwrap().inventory.energy = energy;
            let output = assert_fleet_enumeration_parity(&authority, &state, 0);
            assert_eq!(output.batches[0].fleets.iter().any(|f|matches!(f.decision.action,LivingIntentActionV2::Settle{destination,..} if destination==eid(5))),energy==8);
        }
    }

    fn exact_path_state(system_count: u8, lanes: &[(u8, u8, u64)]) -> LivingGalaxyStateV2 {
        let mut state = builtin_living_sandbox_v2().unwrap().state().clone();
        state.systems = LivingSortedVecV2::new(
            (0..system_count)
                .map(|index| LivingSystemV2 {
                    id: eid(index + 32),
                    name: name(&format!("Path System {index}")).unwrap(),
                })
                .collect(),
        )
        .unwrap();
        state.lanes = LivingSortedVecV2::new(
            lanes
                .iter()
                .enumerate()
                .map(|(index, &(a, b, distance_units))| LivingLaneV2 {
                    id: eid(u8::try_from(index).unwrap() + 100),
                    system_a: eid(a.min(b) + 32),
                    system_b: eid(a.max(b) + 32),
                    distance_units,
                })
                .collect(),
        )
        .unwrap();
        state.worlds = LivingSortedVecV2::new(vec![
            world(
                eid(200),
                eid(32),
                "rocky",
                "Origin",
                None,
                LivingInventoryV2::default(),
            )
            .unwrap(),
            world(
                eid(201),
                eid(system_count + 31),
                "rocky",
                "Destination",
                None,
                LivingInventoryV2::default(),
            )
            .unwrap(),
            world(
                eid(202),
                eid(32),
                "rocky",
                "Local",
                None,
                LivingInventoryV2::default(),
            )
            .unwrap(),
        ])
        .unwrap();
        state
    }

    // A test economics caller: mappings and graph construction consume one
    // source row each, then only true search visits consume the shared budget.
    fn counted_exact_path(
        state: &LivingGalaxyStateV2,
        from: LivingEntityIdV2,
        to: LivingEntityIdV2,
        budgets: &[usize],
    ) -> (Option<(u64, Vec<LivingEntityIdV2>)>, [usize; 3]) {
        use super::super::step_job::{LivingLifecycleJobErrorV2, RetreatGraph, RetreatPathJob};
        use std::sync::Arc;
        let mut graph = Arc::new(RetreatGraph::default());
        let mut search = None;
        let mut mappings = (None, None);
        let mut stage = 0;
        let mut index = 0;
        let mut totals = [0; 3];
        let mut calls = 0;
        let mut saw_multi_visit = false;
        loop {
            let budget = budgets[calls % budgets.len()];
            assert!((1..=1024).contains(&budget));
            calls += 1;
            assert!(calls < 100_000, "search must make bounded progress");
            let mut visits = 0;
            while visits < budget {
                let charged = match stage {
                    0 => {
                        if let Some(row) = state.worlds.as_slice().get(index) {
                            if row.id == from {
                                mappings.0 = Some(row.system);
                            }
                            if row.id == to {
                                mappings.1 = Some(row.system);
                            }
                            index += 1;
                            totals[0] += 1;
                            1
                        } else {
                            let (Some(origin), Some(destination)) = mappings else {
                                return (None, totals);
                            };
                            if origin == destination {
                                return (Some((10, Vec::new())), totals);
                            }
                            index = 0;
                            stage = 1;
                            0
                        }
                    }
                    1 => {
                        if let Some(row) = state.systems.as_slice().get(index) {
                            Arc::get_mut(&mut graph).unwrap().systems.push(row.id);
                            index += 1;
                            totals[1] += 1;
                            1
                        } else {
                            index = 0;
                            stage = 2;
                            0
                        }
                    }
                    2 => {
                        if let Some(row) = state.lanes.as_slice().get(index) {
                            Arc::get_mut(&mut graph).unwrap().lanes.push((
                                row.id,
                                row.system_a,
                                row.system_b,
                                row.distance_units,
                            ));
                            index += 1;
                            totals[1] += 1;
                            1
                        } else {
                            search = Some(RetreatPathJob::begin(
                                Arc::clone(&graph),
                                mappings.0.unwrap(),
                                mappings.1.unwrap(),
                            ));
                            stage = 3;
                            0
                        }
                    }
                    3 => {
                        let job = search.as_mut().unwrap();
                        assert_eq!(Arc::strong_count(&graph), 2);
                        if job.ready() {
                            assert_eq!(job.scratch_path_lengths(), (0, 0, 0, 0));
                            job.poll(1).unwrap();
                            assert_eq!(job.last_poll_visits(), 0);
                            let result = job.take_ready_result().unwrap();
                            assert_eq!(
                                job.take_ready_result(),
                                Err(LivingLifecycleJobErrorV2::NotReady)
                            );
                            assert_eq!(Arc::strong_count(&graph), 2);
                            assert_eq!(job.scratch_path_lengths(), (0, 0, 0, 0));
                            if budgets.iter().any(|budget| *budget > 1) {
                                assert!(
                                    saw_multi_visit,
                                    "actual child must receive multi-visit allowance"
                                );
                            }
                            return (result, totals);
                        }
                        let before = job.scratch_path_lengths();
                        assert_eq!(
                            job.take_ready_result(),
                            Err(LivingLifecycleJobErrorV2::NotReady)
                        );
                        assert_eq!(job.scratch_path_lengths(), before);
                        let remaining = budget - visits;
                        let result = job.poll(remaining);
                        let charged = job.last_poll_visits();
                        assert!(charged <= remaining);
                        totals[2] += charged;
                        saw_multi_visit |= charged > 1;
                        result.unwrap();
                        charged
                    }
                    _ => unreachable!(),
                };
                visits += charged;
                assert!(visits <= budget);
            }
        }
    }

    fn assert_exact_path_budgets(
        state: &LivingGalaxyStateV2,
        from: LivingEntityIdV2,
        to: LivingEntityIdV2,
    ) -> [usize; 3] {
        let expected = travel(state, from, to);
        let mut totals = None;
        for budgets in [&[1][..], &[64], &[1024], &[1, 64, 7, 1024, 3]] {
            let (actual, visits) = counted_exact_path(state, from, to, budgets);
            assert_eq!(actual, expected, "budgets {budgets:?}");
            assert_eq!(visits[0], state.worlds.as_slice().len());
            if let Some(previous) = totals {
                assert_eq!(visits, previous, "budgets must charge the same records");
            }
            totals = Some(visits);
        }
        totals.unwrap()
    }

    #[test]
    fn bounded_path_public_admitted_travel_cases_match_independent_reference() {
        // These are admitted public genesis states. Malformed/missing graph
        // cases stay in separate private characterization tests.
        let mut cases = vec![
            (3, vec![(0, 1, 100)]),
            (4, vec![(0, 2, 100), (0, 1, 100), (1, 3, 100), (2, 3, 100)]),
            (
                5,
                vec![
                    (0, 1, 100),
                    (1, 2, 100),
                    (2, 4, 400),
                    (0, 3, 400),
                    (3, 4, 100),
                ],
            ),
            (3, vec![(0, 1, 1), (1, 2, 101)]),
            (4, vec![(0, 1, 100), (0, 2, 400), (1, 2, 100), (1, 3, 100)]),
        ];
        let mut long: Vec<_> = (0..20).map(|index| (index, index + 1, 100)).collect();
        long.extend([(20, 21, 100), (20, 22, 100), (21, 23, 100), (22, 23, 100)]);
        cases.push((24, long));
        for legs in [9, 11] {
            cases.push((
                legs + 1,
                (0..legs)
                    .map(|index| (index, index + 1, u64::MAX))
                    .collect(),
            ));
        }
        for (systems, lanes) in cases {
            let private = exact_path_state(systems, &lanes);
            let catalog = living_core_pack_v2().unwrap();
            let worlds = LivingSortedVecV2::new(
                private
                    .worlds
                    .as_slice()
                    .iter()
                    .cloned()
                    .map(|mut world| {
                        world.archetype = catalog.world_archetypes()[0].id.clone();
                        world
                    })
                    .collect(),
            )
            .unwrap();
            let state = LivingGalaxyStateV2 {
                systems: private.systems,
                lanes: private.lanes,
                worlds,
                ..Default::default()
            };
            let genesis = validate_living_genesis_manifest_v2(
                LivingGenesisManifestV2 {
                    kind: LIVING_GENESIS_KIND_V2.into(),
                    format_version: LIVING_GENESIS_FORMAT_VERSION_V2,
                    rules_version: LIVING_RULES_VERSION,
                    state,
                },
                &catalog,
            )
            .unwrap();
            let authority = LivingGalaxyAuthorityV2::from_genesis(
                catalog,
                [0xD5; 32],
                LivingGenesisGeneratorV2 {
                    generator_id: slug("bounded_path_public").unwrap(),
                    generator_version: 1,
                },
                genesis,
            )
            .unwrap();
            if lanes == [(0, 1, 100), (0, 2, 400), (1, 2, 100), (1, 3, 100)] {
                // Same graph as the original-buffer every-cut proof, with remapped IDs.
                assert_eq!(
                    travel(authority.state(), eid(200), eid(201)),
                    Some((20, vec![eid(100), eid(103)]))
                );
            }
            assert_exact_path_budgets(authority.state(), eid(200), eid(201));
            assert_eq!(
                travel(authority.state(), eid(200), eid(202)),
                Some((10, vec![]))
            );
            assert_exact_path_budgets(authority.state(), eid(200), eid(202));
        }
    }

    #[test]
    fn exact_path_mapping_local_missing_and_disconnected_match_travel() {
        let state = exact_path_state(3, &[(0, 1, 100)]);
        assert_eq!(travel(&state, eid(200), eid(202)), Some((10, Vec::new())));
        assert_eq!(
            assert_exact_path_budgets(&state, eid(200), eid(202)),
            [3, 0, 0]
        );
        for (from, to) in [
            (eid(200), eid(201)),
            (eid(255), eid(201)),
            (eid(200), eid(255)),
        ] {
            assert_eq!(travel(&state, from, to), None);
            assert_exact_path_budgets(&state, from, to);
        }
    }

    #[test]
    fn exact_path_equal_cost_and_shorter_replacement_match_ordered_reference() {
        let ties = exact_path_state(4, &[(0, 2, 100), (0, 1, 100), (1, 3, 100), (2, 3, 100)]);
        assert_eq!(
            travel(&ties, eid(200), eid(201)),
            Some((20, vec![eid(100), eid(103)]))
        );
        assert_exact_path_budgets(&ties, eid(200), eid(201));
        let replacement = exact_path_state(
            5,
            &[
                (0, 1, 100),
                (1, 2, 100),
                (2, 4, 400),
                (0, 3, 400),
                (3, 4, 100),
            ],
        );
        assert_eq!(
            travel(&replacement, eid(200), eid(201)),
            Some((50, vec![eid(103), eid(104)]))
        );
        assert_exact_path_budgets(&replacement, eid(200), eid(201));
    }

    #[test]
    fn exact_path_long_common_prefix_and_saturating_costs_match_reference() {
        let mut lanes: Vec<_> = (0..20).map(|index| (index, index + 1, 100)).collect();
        lanes.extend([(20, 21, 100), (20, 22, 100), (21, 23, 100), (22, 23, 100)]);
        let state = exact_path_state(24, &lanes);
        let mut path: Vec<_> = (100..120).map(eid).collect();
        path.extend([eid(120), eid(122)]);
        assert_eq!(travel(&state, eid(200), eid(201)), Some((220, path)));
        let visits = assert_exact_path_budgets(&state, eid(200), eid(201));
        assert!(
            visits[2] > 1024,
            "long paths require individually charged work"
        );
        for legs in [9, 11] {
            let lanes: Vec<_> = (0..legs)
                .map(|index| (index, index + 1, u64::MAX))
                .collect();
            let state = exact_path_state(legs + 1, &lanes);
            if legs == 9 {
                assert_eq!(
                    travel(&state, eid(200), eid(201)).unwrap().0,
                    u64::MAX.div_ceil(10) * 9
                );
            } else {
                assert_eq!(travel(&state, eid(200), eid(201)), None);
            }
            assert_exact_path_budgets(&state, eid(200), eid(201));
        }
        let state = exact_path_state(3, &[(0, 1, 0), (1, 2, 101)]);
        assert_eq!(travel(&state, eid(200), eid(201)).unwrap().0, 21);
        assert_exact_path_budgets(&state, eid(200), eid(201));
    }

    #[test]
    fn exact_path_visit_accounting_and_borrowed_extraction_retain_graph() {
        use super::super::step_job::{LivingLifecycleJobErrorV2, RetreatGraph, RetreatPathJob};
        use std::sync::Arc;
        let graph = Arc::new(RetreatGraph {
            systems: vec![eid(1)],
            lanes: Vec::new(),
        });
        let mut job = RetreatPathJob::begin(Arc::clone(&graph), eid(1), eid(1));
        assert_eq!(
            job.take_ready_result(),
            Err(LivingLifecycleJobErrorV2::NotReady)
        );
        while !job.ready() {
            job.poll(1).unwrap();
        }
        assert_eq!(job.total_visits(), 37);
        assert!(job.ready());
        assert_eq!(job.scratch_path_lengths(), (0, 0, 0, 0));
        // This primitive operates on systems. The counted world caller above
        // must apply travel's duration10 local rule instead of this duration0.
        assert_eq!(job.take_ready_result(), Ok(Some((0, Vec::new()))));
        assert_eq!(
            job.take_ready_result(),
            Err(LivingLifecycleJobErrorV2::NotReady)
        );
        assert_eq!(Arc::strong_count(&graph), 2);
        drop(job);
        assert_eq!(Arc::strong_count(&graph), 1);
    }

    #[test]
    fn bounded_path_rules_parents_defer_extraction_until_counted_handoff() {
        use super::super::step_job::{
            LivingLifecycleProgressV2 as Progress, LivingPhaseRetirementJobV2, RetreatGraph,
            RetreatPathJob,
        };
        use std::sync::Arc;
        let (authority, state) = fleet_enumeration_fixture();
        for is_fleet in [false, true] {
            let graph = Arc::new(RetreatGraph {
                systems: vec![eid(1), eid(2)],
                lanes: vec![(eid(7), eid(1), eid(2), 100)],
            });
            let mut path = RetreatPathJob::begin(graph.clone(), eid(1), eid(2));
            path.poll(76).unwrap();
            assert!(!path.ready());
            if is_fleet {
                let mut parent = fleet_test_job(&authority, &state, 1);
                parent.graph = graph.clone().into();
                parent.path = Some(path);
                parent.stage = FleetEnumerationStage::Path;
                parent.poll(1).unwrap();
                assert_eq!(parent.last_poll_visits, 1);
                assert!(parent.path.as_ref().unwrap().ready());
                assert!(parent.duration.is_none());
                parent.poll(1).unwrap();
                assert!(parent.path.is_none());
                assert_eq!(parent.duration, Some(10));
                assert!(!parent.retirement.is_empty());
                assert_eq!(Arc::strong_count(&graph), 3);
                while !parent.retirement.is_empty() {
                    parent.poll(1).unwrap();
                }
                assert_eq!(Arc::strong_count(&graph), 2);
                let mut retired = LivingPhaseRetirementJobV2::begin(
                    parent,
                    LivingStateRetirementPolicyV2::ReleaseHandle,
                );
                while retired.poll(64).unwrap() != Progress::Ready {}
            } else {
                let inputs = economic_test_inputs(&authority, Arc::new(state.clone()));
                let mut parent = LivingEconomicEnumerationJobV2::begin(
                    authority.catalog_snapshot(),
                    inputs,
                    LivingTickV2(50),
                );
                parent.graph = graph.clone().into();
                parent.path = Some(path);
                parent.stage = EconomicEnumerationStage::Path;
                parent.poll(1).unwrap();
                assert_eq!(parent.last_poll_visits, 1);
                assert!(parent.path.as_ref().unwrap().ready());
                assert!(!parent.path_found);
                parent.poll(1).unwrap();
                assert!(parent.path.is_none());
                assert!(parent.path_found);
                assert!(!parent.retirement.is_empty());
                assert_eq!(Arc::strong_count(&graph), 3);
                while !parent.retirement.is_empty() {
                    parent.poll(1).unwrap();
                }
                assert_eq!(Arc::strong_count(&graph), 2);
                let mut retired = LivingPhaseRetirementJobV2::begin(
                    parent,
                    LivingStateRetirementPolicyV2::ReleaseHandle,
                );
                while retired.poll(64).unwrap() != Progress::Ready {}
            }
            assert_eq!(Arc::strong_count(&graph), 1);
        }
    }

    #[test]
    fn bounded_path_economic_fleet_failures_retain_real_parents_and_owners() {
        use super::super::step_job::{
            LivingLifecycleJobErrorV2 as Error, LivingLifecycleProgressV2 as Progress,
            LivingPhaseRetirementJobV2, RetreatGraph, RetreatPathJob,
        };
        use std::sync::Arc;
        // Private stage injection; source Arc allocations are the actual retained owners.
        let (authority, state) = fleet_enumeration_fixture();
        for extraction in [false, true] {
            for trigger in [1, 64, 7, 1024] {
                for schedule in [&[1][..], &[64], &[1024], &[1, 64, 7, 1024, 3]] {
                    let graph = Arc::new(RetreatGraph {
                        systems: vec![eid(1), eid(2)],
                        lanes: vec![(eid(7), eid(1), eid(2), 100)],
                    });
                    let make_path = || {
                        let mut path = RetreatPathJob::begin(graph.clone(), eid(1), eid(2));
                        if extraction {
                            while path.poll(1024).unwrap() != Progress::Ready {}
                            path.inject_extraction_failure();
                        } else {
                            path.inject_capacity_failure();
                        }
                        path
                    };
                    let error = if extraction {
                        Error::Fault(LivingDeterministicFaultV2::Invariant)
                    } else {
                        Error::PreparationCapacity
                    };
                    let inputs = economic_test_inputs(&authority, Arc::new(state.clone()));
                    let mut economic = LivingEconomicEnumerationJobV2::begin(
                        authority.catalog_snapshot(),
                        inputs,
                        LivingTickV2(50),
                    );
                    economic.graph = graph.clone().into();
                    economic.path = Some(make_path());
                    economic.stage = EconomicEnumerationStage::Path;
                    let source = Arc::as_ptr(&economic.inputs.as_ref().unwrap().snapshot);
                    assert_eq!(economic.poll(trigger), Err(error.clone()));
                    assert_eq!(economic.last_poll_visits, 1);
                    assert_eq!(
                        Arc::as_ptr(&economic.inputs.as_ref().unwrap().snapshot),
                        source
                    );
                    assert!(economic.path.is_some());
                    assert_eq!(economic.finish().err(), Some(error.clone()));
                    assert_eq!(economic.poll(1024), Err(error.clone()));
                    assert_eq!(economic.last_poll_visits, 0);
                    economic.cancel();
                    assert_eq!(economic.poll(1), Err(error.clone()));
                    let mut retired = LivingPhaseRetirementJobV2::begin(
                        economic,
                        LivingStateRetirementPolicyV2::ReleaseHandle,
                    );
                    let mut index = 0;
                    while retired.poll(schedule[index % schedule.len()]).unwrap() != Progress::Ready
                    {
                        index += 1;
                        assert!(index < 100000);
                    }
                    let mut fleet = fleet_test_job(&authority, &state, 1);
                    fleet.graph = graph.clone().into();
                    fleet.path = Some(make_path());
                    fleet.stage = FleetEnumerationStage::Path;
                    let source = Arc::as_ptr(&fleet.economics.as_ref().unwrap().inputs.snapshot);
                    assert_eq!(fleet.poll(trigger), Err(error.clone()));
                    assert_eq!(fleet.last_poll_visits, 1);
                    assert_eq!(
                        Arc::as_ptr(&fleet.economics.as_ref().unwrap().inputs.snapshot),
                        source
                    );
                    assert!(fleet.path.is_some());
                    assert_eq!(fleet.finish().err(), Some(error.clone()));
                    assert_eq!(fleet.poll(1024), Err(error.clone()));
                    assert_eq!(fleet.last_poll_visits, 0);
                    fleet.cancel();
                    assert_eq!(fleet.poll(1), Err(error));
                    let mut retired = LivingPhaseRetirementJobV2::begin(
                        fleet,
                        LivingStateRetirementPolicyV2::ReleaseHandle,
                    );
                    let mut index = 0;
                    while retired.poll(schedule[index % schedule.len()]).unwrap() != Progress::Ready
                    {
                        index += 1;
                        assert!(index < 100000);
                    }
                    assert_eq!(Arc::strong_count(&graph), 1);
                }
            }
        }
    }

    #[derive(Clone, Debug, Eq, PartialEq)]
    struct CountedReservationRow {
        key: u32,
        value: u32,
        visits: std::rc::Rc<std::cell::Cell<usize>>,
    }
    impl LivingKeyedV2 for CountedReservationRow {
        type Key = u32;
        fn living_key(&self) -> u32 {
            self.visits.set(self.visits.get() + 1);
            self.key
        }
    }
    fn reservation_rows(
        rows: impl IntoIterator<Item = (u32, u32)>,
        visits: &std::rc::Rc<std::cell::Cell<usize>>,
    ) -> LivingSortedVecV2<CountedReservationRow> {
        LivingSortedVecV2::new(
            rows.into_iter()
                .map(|(key, value)| CountedReservationRow {
                    key,
                    value,
                    visits: std::rc::Rc::clone(visits),
                })
                .collect(),
        )
        .unwrap()
    }
    #[test]
    fn reservation_binary_lookup_matches_linear_partial_conflicts() {
        let visits = std::rc::Rc::new(std::cell::Cell::new(0));
        let snapshot = reservation_rows([(2, 20), (4, 40), (6, 60)], &visits);
        for included in 0u32..128 {
            for changed in 0u32..4 {
                let proposal = reservation_rows(
                    (1..=7)
                        .filter(|key| included & (1 << (key - 1)) != 0)
                        .map(|key| (key, key * 10 + u32::from(key % 4 == changed))),
                    &visits,
                );
                for conflicts in 0u32..16 {
                    let reserved = reservation_rows(
                        (1..=6).filter_map(|key| match key {
                            1 if conflicts & 4 != 0 => Some((key, 42)),
                            2 => Some((key, if conflicts & 1 != 0 { 99 } else { 20 })),
                            4 if conflicts & 2 == 0 => Some((key, 40)),
                            6 => Some((key, if conflicts & 8 != 0 { 99 } else { 60 })),
                            _ => None,
                        }),
                        &visits,
                    );
                    let mut actual = reserved.clone();
                    let mut reference = reserved;
                    let result = reserve_delta(&snapshot, &proposal, &mut actual);
                    let expected =
                        reserve_delta_linear_reference(&snapshot, &proposal, &mut reference);
                    assert_eq!(
                        result, expected,
                        "included={included}, changed={changed}, conflicts={conflicts}"
                    );
                    assert_eq!(
                        actual, reference,
                        "partial candidate differs: included={included}, changed={changed}, conflicts={conflicts}"
                    );
                }
            }
        }
    }
    #[test]
    fn reservation_unchanged_rows_use_logarithmic_key_lookups() {
        let visits = std::rc::Rc::new(std::cell::Cell::new(0));
        let snapshot = reservation_rows((0..1024).map(|key| (key, key)), &visits);
        let proposal = snapshot.clone();
        let mut reserved = snapshot.clone();
        visits.set(0);
        assert!(reserve_delta(&snapshot, &proposal, &mut reserved).unwrap());
        assert_eq!(reserved, snapshot);
        assert!(visits.get() <= 1024 * 32, "{} key visits", visits.get());
    }

    fn reserve_delta_linear_reference<T>(
        snapshot: &LivingSortedVecV2<T>,
        proposal: &LivingSortedVecV2<T>,
        reserved: &mut LivingSortedVecV2<T>,
    ) -> Result<bool, LivingDeterministicFaultV2>
    where
        T: LivingKeyedV2 + Clone + Eq,
    {
        for row in proposal.as_slice() {
            let key = row.living_key();
            let before = snapshot
                .as_slice()
                .iter()
                .find(|old| old.living_key() == key);
            if before == Some(row) {
                continue;
            }
            let current = reserved
                .as_slice()
                .iter()
                .find(|old| old.living_key() == key);
            if current != before {
                return Ok(false);
            }
            put(reserved, row.clone())?;
        }
        for old in snapshot.as_slice() {
            let key = old.living_key();
            if proposal
                .as_slice()
                .iter()
                .any(|row| row.living_key() == key)
            {
                continue;
            }
            if reserved
                .as_slice()
                .iter()
                .find(|row| row.living_key() == key)
                != Some(old)
            {
                return Ok(false);
            }
            remove_key(reserved, &key)?;
        }
        Ok(true)
    }

    fn route_adjustment_state() -> LivingGalaxyStateV2 {
        let authority = builtin_living_sandbox_v2().unwrap();
        let mut state = authority.state().clone();
        state.tick = LivingTickV2(49);
        state.worlds.find_mut(&eid(5)).unwrap().inventory.energy = 30;
        state.worlds.find_mut(&eid(8)).unwrap().inventory.energy = 0;
        for colony in state.colonies.as_slice().to_vec() {
            state
                .colonies
                .find_mut(&colony.world)
                .unwrap()
                .hub
                .energy_next_due = LivingTickV2(60);
        }
        for facility in state.facilities.as_slice().to_vec() {
            if facility.next_due.is_some() {
                state.facilities.find_mut(&facility.id).unwrap().next_due = Some(LivingTickV2(60));
            }
        }
        state.routes.find_mut(&eid(22)).unwrap().next_due = LivingTickV2(60);
        put(
            &mut state.facilities,
            facility(eid(80), eid(8), "extractor", Some(LivingTickV2(60))).unwrap(),
        )
        .unwrap();
        put(
            &mut state.facilities,
            facility(eid(81), eid(8), "foundry", Some(LivingTickV2(60))).unwrap(),
        )
        .unwrap();
        put(
            &mut state.deposits,
            LivingDepositV2 {
                id: eid(82),
                world: eid(8),
                resource: LivingResourceV2::Ore,
                remaining_units: 1_000,
            },
        )
        .unwrap();
        put(
            &mut state.routes,
            LivingFreightRouteV2 {
                id: eid(90),
                kind: LivingRouteKindV2::Internal,
                source_world: eid(5),
                destination_world: eid(8),
                resource: LivingResourceV2::Energy,
                batch_size: 1,
                cadence_ticks: 10,
                source_reserve: 9_000,
                source_owner: eid(3),
                receiver_owner: eid(3),
                next_due: LivingTickV2(77),
                suspended: true,
            },
        )
        .unwrap();
        state.validate(authority.catalog()).unwrap();
        state
    }

    #[test]
    fn supply_adjusts_suspended_ineffective_route_without_reallocating_or_resetting_clock() {
        let authority = builtin_living_sandbox_v2().unwrap();
        let mut state = route_adjustment_state();
        let count = state.routes.as_slice().len();
        let mut events = super::super::receipt::LivingPendingEventsV2::new();
        let mut ctx = LivingStepContextV2 {
            catalog: authority.catalog(),
            genesis_seed: [0; 32],
            branch: authority.branch(),
            tick: LivingTickV2(50),
            state: &mut state,
            events: &mut events,
            arrivals: Vec::new(),
            repair_exclusions: Default::default(),
        };
        generate_and_reserve_intents(&mut ctx).unwrap();
        let adjusted = ctx
            .state
            .routes
            .as_slice()
            .iter()
            .find(|route| route.id == eid(90))
            .unwrap();
        assert_eq!(adjusted.next_due, LivingTickV2(77));
        assert_eq!(adjusted.batch_size, 10);
        assert_eq!(adjusted.source_reserve, 20);
        assert_eq!(adjusted.cadence_ticks, 10);
        assert!(!adjusted.suspended);
        assert_eq!(ctx.state.routes.as_slice().len(), count);
        assert_eq!(
            world_quantity(ctx.state, eid(5), LivingResourceV2::Energy),
            30
        );
        assert_eq!(
            world_quantity(ctx.state, eid(8), LivingResourceV2::Energy),
            0
        );
        assert!(ctx.events.decisions.iter().any(|decision| decision.actor == eid(3)
            && decision.outcome == LivingDecisionOutcomeV2::Accepted
            && matches!(decision.action, LivingIntentActionV2::Route { existing_route: Some(id), .. } if id == eid(90))));
        assert_eq!(
            supply_route_adjustment(
                ctx.state,
                eid(3),
                eid(5),
                eid(8),
                LivingResourceV2::Energy,
                LivingRouteKindV2::Internal,
                eid(3),
                10
            ),
            None
        );
    }

    #[test]
    fn attempted_disclosure_preserves_state_ordinary_receipt_and_autonomous_identities() {
        let authority = builtin_living_sandbox_v2().unwrap();
        let initial = route_adjustment_state();
        let run = |full_disclosure| {
            let mut state = initial.clone();
            let mut events = super::super::receipt::LivingPendingEventsV2::new();
            let mut ctx = LivingStepContextV2 {
                catalog: authority.catalog(),
                genesis_seed: [0; 32],
                branch: authority.branch(),
                tick: LivingTickV2(50),
                state: &mut state,
                events: &mut events,
                arrivals: Vec::new(),
                repair_exclusions: Default::default(),
            };
            generate_and_reserve_intents_with_disclosure(&mut ctx, full_disclosure).unwrap();
            let receipt = super::super::receipt::seal_living_tick_receipt_v2(
                LivingTickV2(50),
                Vec::new(),
                &events,
                state.digest().unwrap(),
            )
            .unwrap();
            (state, events, receipt)
        };
        let (old_state, old_events, old_receipt) = run(true);
        let (state, events, receipt) = run(false);
        assert_eq!(state, old_state);
        assert_eq!(events.payloads(), old_events.payloads());
        assert_eq!(events.explained, old_events.explained);
        assert_eq!(receipt, old_receipt);
        assert!(old_events.decisions.len() > events.decisions.len());
        assert!(!events.decisions.iter().any(|decision| {
            decision
                .reasons
                .iter()
                .any(|reason| matches!(reason, LivingDecisionReasonV2::LowerRank { .. }))
        }));
        let sidecar = super::super::decision::seal_living_decision_receipt_v2(
            authority.catalog().catalog_hash(),
            authority.branch(),
            initial.digest().unwrap(),
            &receipt,
            events.decisions,
        )
        .unwrap();
        sidecar.verify_receipt(&receipt).unwrap();
    }

    #[test]
    fn attempted_capacity_rejection_is_retained_before_legal_fallback() {
        let authority = builtin_living_sandbox_v2().unwrap();
        let mut state = route_adjustment_state();
        remove_key(&mut state.routes, &eid(90)).unwrap();
        let mut routes = state.routes.as_slice().to_vec();
        for index in 0_u16..2047 {
            let mut bytes = [220; 16];
            bytes[14..].copy_from_slice(&index.to_be_bytes());
            routes.push(LivingFreightRouteV2 {
                id: LivingEntityIdV2(bytes),
                kind: LivingRouteKindV2::Aid,
                source_world: eid(5),
                destination_world: eid(6),
                resource: LivingResourceV2::Alloy,
                batch_size: 1,
                cadence_ticks: 10,
                source_reserve: 20,
                source_owner: eid(3),
                receiver_owner: eid(4),
                next_due: LivingTickV2(60),
                suspended: true,
            });
        }
        state.routes = LivingSortedVecV2::new(routes).unwrap();
        state.validate(authority.catalog()).unwrap();
        let mut events = super::super::receipt::LivingPendingEventsV2::new();
        let mut ctx = LivingStepContextV2 {
            catalog: authority.catalog(),
            genesis_seed: [0; 32],
            branch: authority.branch(),
            tick: LivingTickV2(50),
            state: &mut state,
            events: &mut events,
            arrivals: Vec::new(),
            repair_exclusions: Default::default(),
        };
        generate_and_reserve_intents(&mut ctx).unwrap();
        assert!(ctx.events.decisions.iter().any(|decision| {
            decision.actor == eid(3)
                && decision.outcome == LivingDecisionOutcomeV2::Rejected
                && decision
                    .reasons
                    .iter()
                    .any(|reason| matches!(reason, LivingDecisionReasonV2::QueueCapacity { .. }))
        }));
        assert!(
            ctx.events
                .decisions
                .iter()
                .any(|decision| decision.actor == eid(3)
                    && decision.outcome == LivingDecisionOutcomeV2::Accepted)
        );
        assert_eq!(ctx.state.routes.as_slice().len(), 2048);
    }

    #[test]
    fn complete_production_cursor_preserves_jobs_hubs_recipes_repairs_and_receipt_order() {
        use super::super::step_job::{LivingLifecycleProgressV2, LivingProductionJobV2};
        let authority = builtin_living_sandbox_v2().unwrap();
        let mut initial = authority.state().clone();
        initial.tick = LivingTickV2(99);
        for id in initial
            .colonies
            .as_slice()
            .iter()
            .map(|row| row.world)
            .collect::<Vec<_>>()
        {
            let hub = &mut initial.colonies.find_mut(&id).unwrap().hub;
            hub.energy_next_due = LivingTickV2(100);
            hub.ore_next_due = LivingTickV2(100);
            hub.fallback_next_due = LivingTickV2(100);
        }
        for id in initial
            .facilities
            .as_slice()
            .iter()
            .map(|row| row.id)
            .collect::<Vec<_>>()
        {
            let row = initial.facilities.find_mut(&id).unwrap();
            if row.next_due.is_some() {
                row.next_due = Some(LivingTickV2(100));
            }
        }
        initial.facilities.find_mut(&eid(10)).unwrap().disabled = true;
        let fleet_id = initial
            .fleets
            .as_slice()
            .iter()
            .find(|row| row.owner == eid(3))
            .unwrap()
            .id;
        let fleet = initial.fleets.find_mut(&fleet_id).unwrap();
        let hull_id = fleet.hulls.as_slice()[0].id;
        let hull = fleet.hulls.find_mut(&hull_id).unwrap();
        hull.hit_points = 1;
        hull.next_repair = Some(LivingTickV2(100));
        put(
            &mut initial.facilities,
            facility(eid(84), eid(8), "shipyard", None).unwrap(),
        )
        .unwrap();
        for (id, world, definition) in [(80, eid(8), "solar_array"), (81, eid(5), "extractor")] {
            put(
                &mut initial.construction_jobs,
                LivingConstructionJobV2 {
                    id: eid(id),
                    world,
                    owner: eid(3),
                    definition: slug(definition).unwrap(),
                    accepted_tick: LivingTickV2(0),
                    completion_tick: LivingTickV2(100),
                    paid_alloy: 20,
                },
            )
            .unwrap();
        }
        for (id, target_fleet) in [(82, None), (83, Some(fleet_id))] {
            put(
                &mut initial.hull_jobs,
                LivingHullJobV2 {
                    id: eid(id),
                    world: if target_fleet.is_some() {
                        eid(5)
                    } else {
                        eid(8)
                    },
                    owner: eid(3),
                    definition: slug("scout").unwrap(),
                    accepted_tick: LivingTickV2(0),
                    completion_tick: LivingTickV2(100),
                    paid_alloy: 40,
                    target_fleet,
                },
            )
            .unwrap();
        }
        initial.validate(authority.catalog()).unwrap();
        let mut reference = initial.clone();
        let mut expected_events = super::super::receipt::LivingPendingEventsV2::new();
        complete_jobs_and_run_recipes_reference(&mut LivingStepContextV2 {
            catalog: authority.catalog(),
            genesis_seed: [0; 32],
            branch: authority.branch(),
            tick: LivingTickV2(100),
            state: &mut reference,
            events: &mut expected_events,
            arrivals: Vec::new(),
            repair_exclusions: Default::default(),
        })
        .unwrap();
        let mut expected_visits = None;
        // Owned catalogs must hold independent immutable ownership and
        // produce exactly the borrowed job's state, facts and visit schedule.
        let catalog_owner = std::sync::Arc::new(authority.catalog().clone());
        let branch = authority.branch();
        for owned in [false, true] {
            // Complete forward parity, including every rotation of mixed polls.
            for schedule in [
                &[1][..],
                &[64][..],
                &[1024][..],
                &[1, 64, 7, 1024][..],
                &[64, 7, 1024, 1][..],
                &[7, 1024, 1, 64][..],
                &[1024, 1, 64, 7][..],
            ] {
                let mut job = if owned {
                    let catalog = std::sync::Arc::clone(&catalog_owner);
                    LivingProductionJobV2::begin_owned(
                        catalog,
                        initial.clone(),
                        super::super::receipt::LivingPendingEventsV2::new(),
                        LivingTickV2(100),
                        [0; 32],
                        branch,
                    )
                } else {
                    LivingProductionJobV2::begin(
                        authority.catalog(),
                        initial.clone(),
                        super::super::receipt::LivingPendingEventsV2::new(),
                        LivingTickV2(100),
                        [0; 32],
                        branch,
                    )
                };
                let mut poll_index = 0;
                loop {
                    let budget = schedule[poll_index % schedule.len()];
                    poll_index += 1;
                    let before = job.total_visits();
                    let progress = job.poll(budget).unwrap();
                    assert_eq!(job.total_visits() - before, job.last_poll_visits() as u64);
                    assert!(job.last_poll_visits() <= budget);
                    if progress == LivingLifecycleProgressV2::Ready {
                        break;
                    }
                }
                if let Some(visits) = expected_visits {
                    assert_eq!(job.total_visits(), visits);
                } else {
                    expected_visits = Some(job.total_visits());
                }
                let (state, events) = job.finish().unwrap();
                assert_eq!(state, reference);
                assert_eq!(events.payloads(), expected_events.payloads());
                assert_eq!(events.explained, expected_events.explained);
                let seal =
                    |state: &LivingGalaxyStateV2,
                     events: &super::super::receipt::LivingPendingEventsV2| {
                        super::super::receipt::seal_living_tick_receipt_v2(
                            LivingTickV2(100),
                            Vec::new(),
                            events,
                            state.digest().unwrap(),
                        )
                        .unwrap()
                    };
                assert_eq!(seal(&state, &events), seal(&reference, &expected_events));
            }
        }
        let mut detached = LivingProductionJobV2::begin_owned(
            catalog_owner,
            initial.clone(),
            super::super::receipt::LivingPendingEventsV2::new(),
            LivingTickV2(100),
            [0; 32],
            branch,
        );
        detached.poll(1).unwrap();
        detached.cancel();
        assert!(detached.finish().is_err());
        let mut cancelled = LivingProductionJobV2::begin(
            authority.catalog(),
            initial.clone(),
            super::super::receipt::LivingPendingEventsV2::new(),
            LivingTickV2(100),
            [0; 32],
            authority.branch(),
        );
        cancelled.poll(1).unwrap();
        cancelled.cancel();
        assert!(cancelled.finish().is_err());
        assert_eq!(initial.tick, LivingTickV2(99));
    }

    #[test]
    fn observation_cursor_preserves_exact_snapshots_and_nested_visit_budgets() {
        use super::super::step_job::{LivingLifecycleProgressV2, LivingObservationJobV2};
        let authority = builtin_living_sandbox_v2().unwrap();
        let mut initial = authority.state().clone();
        // Disabled batteries must retain their aggregate while exposing the operational count.
        let mut battery = initial.facilities.as_slice()[0].clone();
        battery.id = eid(250);
        battery.definition = slug("defense_battery").unwrap();
        battery.disabled = true;
        put(&mut initial.facilities, battery).unwrap();
        let isolated_system = eid(240);
        put(
            &mut initial.systems,
            LivingSystemV2 {
                id: isolated_system,
                name: name("Isolated").unwrap(),
            },
        )
        .unwrap();
        for (id, label) in [
            (eid(241), "Dock"),
            (eid(242), "Neighbour"),
            (eid(243), "Remembered"),
        ] {
            put(
                &mut initial.worlds,
                world(
                    id,
                    isolated_system,
                    "barren",
                    label,
                    None,
                    LivingInventoryV2::default(),
                )
                .unwrap(),
            )
            .unwrap();
        }
        // A scout after a non-scout hull reveals both worlds in the remote system.
        put(
            &mut initial.fleets,
            fleet_with(
                eid(244),
                eid(3),
                eid(241),
                vec![
                    hull(eid(245), "escort", false).unwrap(),
                    hull(eid(246), "scout", false).unwrap(),
                ],
            )
            .unwrap(),
        )
        .unwrap();
        // The other civilization sees only its non-scout fleet's dock, not the system.
        put(
            &mut initial.fleets,
            fleet_with(
                eid(247),
                eid(4),
                eid(241),
                vec![hull(eid(248), "escort", false).unwrap()],
            )
            .unwrap(),
        )
        .unwrap();
        let mut travelling = fleet_with(
            eid(251),
            eid(4),
            eid(242),
            vec![hull(eid(252), "scout", true).unwrap()],
        )
        .unwrap();
        travelling.location = LivingFleetLocationV2::Travelling {
            origin_world: eid(242),
            destination_world: eid(243),
            departure_tick: LivingTickV2(70),
            arrival_tick: LivingTickV2(80),
        };
        put(&mut initial.fleets, travelling).unwrap();
        put(
            &mut initial.observations,
            LivingObservationV2 {
                civilization: eid(4),
                world: eid(243),
                observed_tick: LivingTickV2(2),
                owner: Some(eid(3)),
                remaining_deposit: 17,
                facilities: LivingSortedVecV2::default(),
                stationed_hulls: LivingSortedVecV2::default(),
                operational_defense_batteries: None,
            },
        )
        .unwrap();
        let mut reference = initial.clone();
        let mut events = super::super::receipt::LivingPendingEventsV2::new();
        refresh_observations_reference(&mut LivingStepContextV2 {
            catalog: authority.catalog(),
            genesis_seed: [0; 32],
            branch: authority.branch(),
            tick: LivingTickV2(77),
            state: &mut reference,
            events: &mut events,
            arrivals: Vec::new(),
            repair_exclusions: Default::default(),
        })
        .unwrap();
        assert!(
            reference
                .observations
                .as_slice()
                .iter()
                .any(|row| row.civilization == eid(3)
                    && row.world == eid(242)
                    && row.observed_tick == LivingTickV2(77))
        );
        assert!(
            !reference
                .observations
                .as_slice()
                .iter()
                .any(|row| row.civilization == eid(4) && row.world == eid(242))
        );
        assert_eq!(
            reference
                .observations
                .as_slice()
                .iter()
                .find(|row| row.civilization == eid(4) && row.world == eid(243))
                .unwrap()
                .observed_tick,
            LivingTickV2(2)
        );
        let mut visits = None;
        for budget in [1, 64, 1024] {
            let mut job = LivingObservationJobV2::begin(initial.clone(), LivingTickV2(77));
            assert_eq!(
                job.poll(0),
                Err(super::super::step_job::LivingLifecycleJobErrorV2::Budget)
            );
            assert_eq!(
                job.poll(1025),
                Err(super::super::step_job::LivingLifecycleJobErrorV2::Budget)
            );
            assert_eq!(job.total_visits(), 0);
            assert_eq!(job.last_poll_visits(), 0);
            loop {
                let progress = job.poll(budget).unwrap();
                assert!(job.last_poll_visits() <= budget);
                if progress == LivingLifecycleProgressV2::Ready {
                    break;
                }
            }
            if let Some(previous) = visits {
                assert_eq!(job.total_visits(), previous);
            }
            visits = Some(job.total_visits());
            assert_eq!(job.finish().unwrap(), reference);
        }
        let mut cancelled = LivingObservationJobV2::begin(initial, LivingTickV2(77));
        cancelled.poll(1).unwrap();
        cancelled.cancel();
        assert_eq!(
            cancelled.poll(1),
            Err(super::super::step_job::LivingLifecycleJobErrorV2::Cancelled)
        );
        assert_eq!(cancelled.last_poll_visits(), 0);
        assert!(cancelled.finish().is_err());
        assert!(matches!(
            LivingObservationJobV2::begin(reference, LivingTickV2(78)).finish(),
            Err(super::super::step_job::LivingLifecycleJobErrorV2::NotReady)
        ));
    }

    #[test]
    fn production_fault_after_private_inventory_change_is_latched_and_cannot_publish() {
        use super::super::step_job::{LivingLifecycleJobErrorV2, LivingProductionJobV2};
        let authority = builtin_living_sandbox_v2().unwrap();
        let mut initial = authority.state().clone();
        initial.tick = LivingTickV2(u64::MAX - 1);
        let hub = &mut initial.colonies.find_mut(&eid(5)).unwrap().hub;
        hub.energy_next_due = LivingTickV2(u64::MAX);
        initial.worlds.find_mut(&eid(5)).unwrap().inventory.energy = 0;
        let digest = initial.digest().unwrap();
        let mut expected_visits = None;
        for budget in [1, 64, 1024] {
            let mut job = LivingProductionJobV2::begin(
                authority.catalog(),
                initial.clone(),
                super::super::receipt::LivingPendingEventsV2::new(),
                LivingTickV2(u64::MAX),
                [0; 32],
                authority.branch(),
            );
            assert_eq!(job.poll(0), Err(LivingLifecycleJobErrorV2::Budget));
            assert_eq!(job.last_poll_visits(), 0);
            loop {
                let result = job.poll(budget);
                assert!(job.last_poll_visits() <= budget);
                if let Err(error) = result {
                    assert_eq!(
                        error,
                        LivingLifecycleJobErrorV2::Fault(LivingDeterministicFaultV2::Arithmetic)
                    );
                    break;
                }
            }
            if let Some(visits) = expected_visits {
                assert_eq!(job.total_visits(), visits);
            } else {
                expected_visits = Some(job.total_visits());
            }
            assert_eq!(
                job.poll(1),
                Err(LivingLifecycleJobErrorV2::Fault(
                    LivingDeterministicFaultV2::Arithmetic
                ))
            );
            assert_eq!(job.last_poll_visits(), 0);
            assert!(job.finish().is_err());
        }
        assert_eq!(initial.digest().unwrap(), digest);
        assert_eq!(
            initial
                .worlds
                .as_slice()
                .iter()
                .find(|world| world.id == eid(5))
                .unwrap()
                .inventory
                .energy,
            0
        );
    }

    #[test]
    fn captured_route_requires_explicit_current_owner_permission_and_preserves_cargo_manifest() {
        let authority = builtin_living_sandbox_v2().unwrap();
        let mut state = route_adjustment_state();
        put(
            &mut state.shipments,
            LivingShipmentV2 {
                id: eid(91),
                dispatch_owner: eid(3),
                intended_receiver: eid(3),
                source_world: eid(5),
                destination_world: eid(8),
                resource: LivingResourceV2::Energy,
                units: 7,
                departure_tick: LivingTickV2(40),
                arrival_tick: LivingTickV2(50),
                disposition: LivingShipmentDispositionV2::Outbound,
                dispatch_kind: Some(LivingRouteKindV2::Internal),
                returning_to_source: false,
            },
        )
        .unwrap();
        let cargo = state.shipments.clone();
        let resolve = |target: &LivingCommandTargetV2| match target {
            LivingCommandTargetV2::Existing { id } => *id,
            LivingCommandTargetV2::Local { .. } => unreachable!(),
        };
        apply_creator_operation(
            &mut state,
            LivingTickV2(49),
            0,
            &LivingCreatorOperationV2::SetWorldOwner {
                world: LivingCommandTargetV2::Existing { id: eid(5) },
                owner: Some(LivingCommandTargetV2::Existing { id: eid(4) }),
            },
            &resolve,
            &mut Vec::new(),
            authority.catalog(),
        )
        .unwrap();
        state.civilizations.find_mut(&eid(4)).unwrap().policy =
            LivingPolicyV2(slug("neutral").unwrap());
        let held = state
            .routes
            .as_slice()
            .iter()
            .find(|route| route.id == eid(90))
            .unwrap()
            .clone();
        let mut events = super::super::receipt::LivingPendingEventsV2::new();
        let ctx = LivingStepContextV2 {
            catalog: authority.catalog(),
            genesis_seed: [0; 32],
            branch: authority.branch(),
            tick: LivingTickV2(50),
            state: &mut state,
            events: &mut events,
            arrivals: Vec::new(),
            repair_exclusions: Default::default(),
        };
        let rejected = LivingEconomicCandidateV2 {
            tier: 0,
            score: 70,
            target: eid(8),
            action: LivingEconomicActionV2::Supply {
                source: eid(5),
                destination: eid(8),
                resource: LivingResourceV2::Energy,
                kind: LivingRouteKindV2::Internal,
                receiver: eid(3),
                batch_size: 10,
                existing_route: Some(eid(90)),
            },
        };
        let before = ctx.state.clone();
        assert!(apply_economic_proposal(&ctx, eid(3), 0, rejected).is_err());
        assert_eq!(*ctx.state, before);
        let forecast = forecast_scheduled_economy(&ctx, eid(4)).unwrap();
        let request = LivingReceiverDemandRequestV2 {
            receiver: eid(3),
            world: eid(8),
            resource: LivingResourceV2::Energy,
            requested_units: 20,
        };
        let denied =
            reserve_economy_proposals(&ctx, eid(4), "neutral", &forecast, &[request]).unwrap();
        assert!(!denied.iter().any(|(_, candidate)| matches!(candidate.action, LivingEconomicActionV2::Supply { existing_route: Some(id), .. } if id == eid(90))));
        assert_eq!(
            ctx.state
                .routes
                .as_slice()
                .iter()
                .find(|route| route.id == eid(90))
                .unwrap(),
            &held
        );
        put(
            &mut ctx.state.agreements,
            LivingAgreementV2 {
                kind: LivingAgreementKindV2::Trade,
                participant_a: eid(3),
                participant_b: eid(4),
                start_tick: LivingTickV2(0),
                end_tick: LivingTickV2(1_200),
            },
        )
        .unwrap();
        let forecast = forecast_scheduled_economy(&ctx, eid(4)).unwrap();
        let request = LivingReceiverDemandRequestV2 {
            receiver: eid(3),
            world: eid(8),
            resource: LivingResourceV2::Energy,
            requested_units: 20,
        };
        let approved =
            reserve_economy_proposals(&ctx, eid(4), "neutral", &forecast, &[request]).unwrap();
        let (ordinal, candidate) = approved.into_iter().find(|(_, candidate)| matches!(candidate.action, LivingEconomicActionV2::Supply { existing_route: Some(id), .. } if id == eid(90))).unwrap();
        let adjusted = apply_economic_proposal(&ctx, eid(4), ordinal, candidate).unwrap();
        let route = adjusted
            .routes
            .as_slice()
            .iter()
            .find(|route| route.id == eid(90))
            .unwrap();
        assert_eq!(route.source_owner, eid(4));
        assert_eq!(route.receiver_owner, eid(3));
        assert_eq!(route.kind, LivingRouteKindV2::Trade);
        assert_eq!(route.next_due, held.next_due);
        assert!(!route.suspended);
        assert_eq!(adjusted.shipments, cargo);
        assert_eq!(
            ctx.state
                .routes
                .as_slice()
                .iter()
                .find(|route| route.id == eid(90))
                .unwrap(),
            &held,
            "a proposed adjustment does not alter the original snapshot"
        );
    }

    #[test]
    fn exploration_splits_the_affordable_scout_and_launches_on_next_boundary() {
        let authority = builtin_living_sandbox_v2().unwrap();
        let mut state = authority.state().clone();
        let home = eid(5);
        state.worlds.find_mut(&home).unwrap().inventory.energy = 2;
        state.fleets = LivingSortedVecV2::new(vec![
            fleet_with(
                eid(90),
                eid(3),
                home,
                vec![
                    hull(eid(91), "scout", false).unwrap(),
                    hull(eid(92), "scout", true).unwrap(),
                ],
            )
            .unwrap(),
        ])
        .unwrap();
        state.validate(authority.catalog()).unwrap();
        let mut events = super::super::receipt::LivingPendingEventsV2::new();
        let mut ctx = LivingStepContextV2 {
            catalog: authority.catalog(),
            genesis_seed: [0; 32],
            branch: authority.branch(),
            tick: LivingTickV2(50),
            state: &mut state,
            events: &mut events,
            arrivals: Vec::new(),
            repair_exclusions: Default::default(),
        };
        reserve_exploration(&mut ctx, eid(3)).unwrap();
        let dispatched = ctx
            .state
            .fleets
            .as_slice()
            .iter()
            .find(|fleet| fleet.order.is_some())
            .unwrap();
        let fleet_id = dispatched.id;
        assert_eq!(dispatched.hulls.as_slice()[0].id, eid(92));
        assert_eq!(
            dispatched.order.as_ref().unwrap().not_before_tick,
            LivingTickV2(51)
        );
        assert_eq!(
            ctx.state
                .fleets
                .as_slice()
                .iter()
                .find(|fleet| fleet.id == eid(90))
                .unwrap()
                .hulls
                .as_slice()[0]
                .id,
            eid(91)
        );
        dispatch_fleets(&mut ctx).unwrap();
        assert_eq!(world_quantity(ctx.state, home, LivingResourceV2::Energy), 2);
        ctx.tick = LivingTickV2(51);
        dispatch_fleets(&mut ctx).unwrap();
        let departed = ctx
            .state
            .fleets
            .as_slice()
            .iter()
            .find(|fleet| fleet.id == fleet_id)
            .unwrap();
        assert!(matches!(
            departed.location,
            LivingFleetLocationV2::Travelling {
                departure_tick: LivingTickV2(51),
                ..
            }
        ));
        assert_eq!(world_quantity(ctx.state, home, LivingResourceV2::Energy), 0);
    }

    #[test]
    fn war_eligibility_uses_one_funded_idle_reachable_unreserved_escort_fleet() {
        let authority = builtin_living_sandbox_v2().unwrap();
        let mut state = authority.state().clone();
        let prototype = state
            .worlds
            .as_slice()
            .iter()
            .find(|world| world.id == eid(5))
            .unwrap()
            .clone();
        for (system, world_id) in [(eid(70), eid(60)), (eid(71), eid(61))] {
            put(
                &mut state.systems,
                LivingSystemV2 {
                    id: system,
                    name: name("Disconnected").unwrap(),
                },
            )
            .unwrap();
            let mut remote = prototype.clone();
            remote.id = world_id;
            remote.system = system;
            put(&mut state.worlds, remote).unwrap();
            put(&mut state.colonies, colony(world_id, eid(3)).unwrap()).unwrap();
        }
        state.fleets = LivingSortedVecV2::new(vec![
            fleet_with(
                eid(90),
                eid(3),
                eid(5),
                vec![hull(eid(93), "escort", false).unwrap()],
            )
            .unwrap(),
            fleet_with(
                eid(91),
                eid(3),
                eid(60),
                vec![hull(eid(94), "escort", false).unwrap()],
            )
            .unwrap(),
            fleet_with(
                eid(92),
                eid(3),
                eid(61),
                vec![hull(eid(95), "escort", false).unwrap()],
            )
            .unwrap(),
        ])
        .unwrap();
        let mut battery = facility(eid(100), eid(6), "defense_battery", None).unwrap();
        battery.hit_points = 40;
        put(&mut state.facilities, battery).unwrap();
        state.validate(authority.catalog()).unwrap();
        let mut events = super::super::receipt::LivingPendingEventsV2::new();
        let ctx = LivingStepContextV2 {
            catalog: authority.catalog(),
            genesis_seed: [0; 32],
            branch: authority.branch(),
            tick: LivingTickV2(100),
            state: &mut state,
            events: &mut events,
            arrivals: Vec::new(),
            repair_exclusions: Default::default(),
        };
        let observed = observe_world(&ctx, eid(3), eid(6), LivingTickV2(0)).unwrap();
        put(&mut ctx.state.observations, observed).unwrap();
        assert_eq!(observed_defense(ctx.state, eid(3), eid(6)), 20);
        assert_eq!(assembled_attack(ctx.state, eid(3), eid(6)), 10);
        assert_eq!(
            war_target(&ctx, eid(3)),
            None,
            "three separate fleets cannot assemble strength across disconnected origins"
        );
        ctx.state.fleets.find_mut(&eid(90)).unwrap().hulls = LivingSortedVecV2::new(vec![
            hull(eid(93), "escort", false).unwrap(),
            hull(eid(96), "escort", false).unwrap(),
            hull(eid(97), "escort", false).unwrap(),
        ])
        .unwrap();
        assert_eq!(war_target(&ctx, eid(3)), Some((eid(4), eid(6))));
        ctx.state.worlds.find_mut(&eid(5)).unwrap().inventory.energy = 11;
        assert_eq!(
            war_target(&ctx, eid(3)),
            None,
            "assembled fleet must fund its actual role launch"
        );
        ctx.state.worlds.find_mut(&eid(5)).unwrap().inventory.energy = 12;
        ctx.state.fleets.find_mut(&eid(90)).unwrap().order = Some(LivingFleetOrderV2 {
            kind: LivingFleetOrderKindV2::Explore,
            target_world: Some(eid(7)),
            not_before_tick: LivingTickV2(101),
            lane_path: Vec::new(),
        });
        assert_eq!(
            war_target(&ctx, eid(3)),
            None,
            "occupied fleet intent is unavailable"
        );
        ctx.state.fleets.find_mut(&eid(90)).unwrap().order = None;
        put(
            &mut ctx.state.hull_jobs,
            LivingHullJobV2 {
                id: eid(101),
                world: eid(5),
                owner: eid(3),
                definition: slug("colony_ark").unwrap(),
                accepted_tick: LivingTickV2(0),
                completion_tick: LivingTickV2(200),
                paid_alloy: 60,
                target_fleet: Some(eid(90)),
            },
        )
        .unwrap();
        ctx.state.validate(authority.catalog()).unwrap();
        assert_eq!(
            war_target(&ctx, eid(3)),
            None,
            "paid hull delivery reserves fleet formation"
        );
    }

    #[test]
    fn repair_cursor_matches_reference_across_cadences_and_scarce_energy() {
        use super::super::step_job::{
            LivingLifecycleJobErrorV2, LivingLifecycleProgressV2, LivingRepairJobV2,
        };
        let authority = builtin_living_sandbox_v2().unwrap();
        let mut source = authority.state().clone();
        let owner = source.colonies.as_slice()[0].owner;
        let home = source.colonies.as_slice()[0].world;
        let foreign = source
            .colonies
            .as_slice()
            .iter()
            .find(|row| row.owner != owner)
            .unwrap()
            .world;
        source.worlds.find_mut(&home).unwrap().inventory.energy = 3;
        let prototype = source.fleets.as_slice()[0].clone();
        let mut fleets = Vec::new();
        for fleet_index in 0_u8..2 {
            let mut fleet = prototype.clone();
            fleet.id = LivingEntityIdV2([180 + fleet_index; 16]);
            fleet.owner = owner;
            fleet.location = LivingFleetLocationV2::Docked {
                world: if fleet_index == 0 { home } else { foreign },
            };
            fleet.order = None;
            let mut hulls = Vec::new();
            for hull_index in 0_u8..16 {
                let mut hull = prototype.hulls.as_slice()[0].clone();
                let mut bytes = [182 + fleet_index; 16];
                bytes[15] = hull_index;
                hull.id = LivingEntityIdV2(bytes);
                let maximum = authority
                    .catalog()
                    .hull(hull.definition.as_str())
                    .unwrap()
                    .hit_points;
                hull.hit_points = if hull_index == 0 {
                    maximum
                } else {
                    maximum - 1
                };
                hull.next_repair = Some(LivingTickV2(if hull_index == 15 { 50 } else { 10 }));
                hulls.push(hull);
            }
            fleet.hulls = LivingSortedVecV2::new(hulls).unwrap();
            fleets.push(fleet);
        }
        source.fleets = LivingSortedVecV2::new(fleets).unwrap();
        source.validate(authority.catalog()).unwrap();
        let mut expected = source.clone();
        let mut ignored = super::super::receipt::LivingPendingEventsV2::new();
        let mut context = LivingStepContextV2 {
            catalog: authority.catalog(),
            genesis_seed: [0; 32],
            branch: authority.branch(),
            tick: LivingTickV2(10),
            state: &mut expected,
            events: &mut ignored,
            arrivals: Vec::new(),
            repair_exclusions: Default::default(),
        };
        run_repairs_reference(&mut context).unwrap();
        let mut old_state = source.clone();
        context.state = &mut old_state;
        old_run_repairs_reference(&mut context).unwrap();
        assert_eq!(old_state, expected, "unchanged on-time hull-only policy");
        let mut visit_count = None;
        for budget in [1, 64, 1_024] {
            let mut job =
                LivingRepairJobV2::begin(authority.catalog(), source.clone(), LivingTickV2(10));
            loop {
                let progress = job.poll(budget).unwrap();
                assert!(job.last_poll_visits() <= budget);
                if progress == LivingLifecycleProgressV2::Ready {
                    break;
                }
            }
            if let Some(visits) = visit_count {
                assert_eq!(job.total_visits(), visits);
            }
            visit_count = Some(job.total_visits());
            assert_eq!(job.finish().unwrap(), expected);
        }
        assert_eq!(
            expected
                .worlds
                .as_slice()
                .iter()
                .find(|row| row.id == home)
                .unwrap()
                .inventory
                .energy,
            0
        );
        assert_eq!(
            expected.fleets.as_slice()[0].hulls.as_slice()[0].next_repair,
            None
        );
        assert_eq!(
            expected.fleets.as_slice()[0].hulls.as_slice()[4].next_repair,
            Some(LivingTickV2(20))
        );
        assert_eq!(
            expected.fleets.as_slice()[0].hulls.as_slice()[15].next_repair,
            Some(LivingTickV2(50))
        );
        assert_eq!(expected.fleets.as_slice()[1], source.fleets.as_slice()[1]);
        let mut cancelled =
            LivingRepairJobV2::begin(authority.catalog(), source.clone(), LivingTickV2(10));
        cancelled.poll(1).unwrap();
        cancelled.cancel();
        assert!(matches!(
            cancelled.poll(1),
            Err(LivingLifecycleJobErrorV2::Cancelled)
        ));
        assert!(matches!(
            cancelled.finish(),
            Err(LivingLifecycleJobErrorV2::Cancelled)
        ));
    }

    #[test]
    fn repair_cursor_latches_clock_overflow_without_publishing_payment() {
        use super::super::step_job::{
            LivingLifecycleJobErrorV2, LivingLifecycleProgressV2, LivingRepairJobV2,
        };
        let authority = builtin_living_sandbox_v2().unwrap();
        let mut source = authority.state().clone();
        let colony = source.colonies.as_slice()[0].clone();
        let mut fleet = source.fleets.as_slice()[0].clone();
        fleet.owner = colony.owner;
        fleet.location = LivingFleetLocationV2::Docked {
            world: colony.world,
        };
        fleet.order = None;
        fleet.hulls = LivingSortedVecV2::new(vec![LivingHullV2 {
            id: eid(200),
            definition: slug("escort").unwrap(),
            hit_points: 1,
            return_credit: false,
            next_repair: Some(LivingTickV2(u64::MAX)),
        }])
        .unwrap();
        source.fleets = LivingSortedVecV2::new(vec![fleet]).unwrap();
        source
            .worlds
            .find_mut(&colony.world)
            .unwrap()
            .inventory
            .energy = 0;
        let before = source.clone();
        let mut job = LivingRepairJobV2::begin(authority.catalog(), source, LivingTickV2(u64::MAX));
        loop {
            match job.poll(1) {
                Ok(LivingLifecycleProgressV2::Pending) => {}
                result => {
                    assert!(matches!(
                        result,
                        Err(LivingLifecycleJobErrorV2::Fault(
                            LivingDeterministicFaultV2::Arithmetic
                        ))
                    ));
                    assert_eq!(job.last_poll_visits(), 1);
                    break;
                }
            }
        }
        let visits = job.total_visits();
        assert!(matches!(
            job.poll(1),
            Err(LivingLifecycleJobErrorV2::Fault(_))
        ));
        assert_eq!(job.last_poll_visits(), 0);
        assert_eq!(job.total_visits(), visits);
        assert!(matches!(
            job.finish(),
            Err(LivingLifecycleJobErrorV2::Fault(_))
        ));
        assert_eq!(
            before
                .worlds
                .as_slice()
                .iter()
                .find(|row| row.id == colony.world)
                .unwrap()
                .inventory
                .energy,
            0
        );
    }

    #[test]
    fn economic_ranking_applies_policy_and_actionable_emergency_precedence() {
        for (policy_name, emergency, expected) in [
            ("neutral", false, "foundry"),
            ("guardian", false, "escort"),
            ("guardian", true, "solar_array"),
        ] {
            let mut authority = builtin_first_expansion_v2().unwrap();
            for _ in 0..49 {
                authority.step().unwrap();
            }
            let mut state = authority.state().clone();
            let actor = state.civilizations.as_slice()[0].id;
            let home = state.colonies.as_slice()[0].world;
            state.civilizations.find_mut(&actor).unwrap().policy =
                LivingPolicyV2(slug(policy_name).unwrap());
            remove_key(
                &mut state.fleets.find_mut(&eid(11)).unwrap().hulls,
                &eid(14),
            )
            .unwrap();
            if emergency {
                remove_key(&mut state.facilities, &eid(8)).unwrap();
                state.worlds.find_mut(&home).unwrap().inventory.energy = 0;
                let hull = state
                    .fleets
                    .find_mut(&eid(11))
                    .unwrap()
                    .hulls
                    .find_mut(&eid(13))
                    .unwrap();
                hull.hit_points = 1;
                hull.next_repair = Some(LivingTickV2(50));
            }
            let mut events = super::super::receipt::LivingPendingEventsV2::new();
            let mut ctx = LivingStepContextV2 {
                catalog: authority.catalog(),
                genesis_seed: [0; 32],
                branch: authority.branch(),
                tick: LivingTickV2(50),
                state: &mut state,
                events: &mut events,
                arrivals: Vec::new(),
                repair_exclusions: Default::default(),
            };
            for target in [home, eid(5)] {
                let observation = observe_world(&ctx, actor, target, ctx.tick).unwrap();
                put(&mut ctx.state.observations, observation).unwrap();
            }
            generate_and_reserve_intents(&mut ctx).unwrap();
            let jobs: Vec<_> = ctx
                .state
                .construction_jobs
                .as_slice()
                .iter()
                .filter(|job| job.owner == actor)
                .map(|job| job.definition.as_str())
                .chain(
                    ctx.state
                        .hull_jobs
                        .as_slice()
                        .iter()
                        .filter(|job| job.owner == actor)
                        .map(|job| job.definition.as_str()),
                )
                .collect();
            assert_eq!(
                jobs,
                [expected],
                "ranked legal candidates accept exactly one economic winner"
            );
        }
    }

    #[test]
    fn a_paid_shipyard_is_not_used_until_it_actually_completes() {
        let mut authority = builtin_first_expansion_v2().unwrap();
        for _ in 0..49 {
            authority.step().unwrap();
        }
        let mut state = authority.state().clone();
        let actor = state.civilizations.as_slice()[0].id;
        let home = state.colonies.as_slice()[0].world;
        remove_key(&mut state.facilities, &eid(10)).unwrap();
        put(
            &mut state.construction_jobs,
            LivingConstructionJobV2 {
                id: eid(60),
                world: home,
                owner: actor,
                definition: slug("shipyard").unwrap(),
                accepted_tick: LivingTickV2(0),
                completion_tick: LivingTickV2(100),
                paid_alloy: 60,
            },
        )
        .unwrap();
        let mut events = super::super::receipt::LivingPendingEventsV2::new();
        let mut ctx = LivingStepContextV2 {
            catalog: authority.catalog(),
            genesis_seed: [0; 32],
            branch: authority.branch(),
            tick: LivingTickV2(50),
            state: &mut state,
            events: &mut events,
            arrivals: Vec::new(),
            repair_exclusions: Default::default(),
        };
        let observation = observe_world(&ctx, actor, eid(5), ctx.tick).unwrap();
        put(&mut ctx.state.observations, observation).unwrap();
        generate_and_reserve_intents(&mut ctx).unwrap();
        assert!(ctx.state.hull_jobs.as_slice().is_empty());
        assert_eq!(ctx.state.construction_jobs.as_slice().len(), 1);
        assert_eq!(
            ctx.state.construction_jobs.as_slice()[0]
                .definition
                .as_str(),
            "shipyard"
        );
    }

    #[test]
    fn forecast_explanation_retention_cannot_change_economics_or_candidate_scores() {
        let authority = builtin_living_sandbox_v2().unwrap();
        let mut state = authority.state().clone();
        let actor = eid(3);
        let mut facilities = state.facilities.as_slice().to_vec();
        let mut worlds = state.worlds.as_slice().to_vec();
        let mut colonies = state.colonies.as_slice().to_vec();
        let prototype_world = worlds[0].clone();
        let prototype_colony = colonies[0].clone();
        for index in 0_u16..508 {
            let mut id = [200; 16];
            id[14..].copy_from_slice(&index.to_be_bytes());
            let world_id = LivingEntityIdV2(id);
            let mut world = prototype_world.clone();
            world.id = world_id;
            world.inventory = LivingInventoryV2 {
                energy: LIVING_RESOURCE_STORAGE_LIMIT_V2,
                ore: LIVING_RESOURCE_STORAGE_LIMIT_V2,
                alloy: LIVING_RESOURCE_STORAGE_LIMIT_V2,
            };
            worlds.push(world);
            let mut colony = prototype_colony.clone();
            colony.world = world_id;
            colonies.push(colony);
            for slot in 0_u16..4 {
                let mut row = facilities[0].clone();
                let mut id = [201; 16];
                id[14..].copy_from_slice(&(index * 4 + slot).to_be_bytes());
                row.id = LivingEntityIdV2(id);
                row.world = world_id;
                row.disabled = slot % 2 == 0;
                facilities.push(row);
            }
        }
        facilities.sort_by_key(LivingKeyedV2::living_key);
        worlds.sort_by_key(LivingKeyedV2::living_key);
        colonies.sort_by_key(LivingKeyedV2::living_key);
        state.facilities = LivingSortedVecV2::new(facilities).unwrap();
        state.worlds = LivingSortedVecV2::new(worlds).unwrap();
        state.colonies = LivingSortedVecV2::new(colonies).unwrap();
        state.validate(authority.catalog()).unwrap();
        let mut events = super::super::receipt::LivingPendingEventsV2::new();
        let ctx = LivingStepContextV2 {
            catalog: authority.catalog(),
            genesis_seed: [0; 32],
            branch: authority.branch(),
            tick: LivingTickV2(0),
            state: &mut state,
            events: &mut events,
            arrivals: Vec::new(),
            repair_exclusions: Default::default(),
        };
        let retained = forecast_scheduled_economy_with_explanations(&ctx, actor, true).unwrap();
        let discarded = forecast_scheduled_economy_with_explanations(&ctx, actor, false).unwrap();
        assert_eq!(retained.inventories, discarded.inventories);
        assert_eq!(retained.deficits, discarded.deficits);
        assert_eq!(retained.idle_facilities, discarded.idle_facilities);
        assert_eq!(retained.final_state, discarded.final_state);
        let left = reserve_economy_proposals(&ctx, actor, "expansionist", &retained, &[]).unwrap();
        let right =
            reserve_economy_proposals(&ctx, actor, "expansionist", &discarded, &[]).unwrap();
        assert!(left == right);
    }

    fn assert_scheduled_forecast_retryable_budget(job: &mut LivingScheduledForecastJobV2<'_>) {
        use super::super::step_job::LivingLifecycleJobErrorV2 as Error;
        let stage = job.stage;
        let visits = job.total_visits();
        assert_eq!(job.poll(0), Err(Error::Budget));
        assert_eq!(job.poll(1025), Err(Error::Budget));
        assert!(job.stage == stage);
        assert_eq!(job.last_poll_visits(), 0);
        assert_eq!(job.total_visits(), visits);
    }

    #[test]
    fn scheduled_forecast_cursor_owned_detaches_callers_and_retires_source() {
        use super::super::step_job::LivingLifecycleProgressV2 as Progress;
        let mut visits = None;
        for budget in [1, 64, 1024] {
            let (mut job, expected, before, source, catalog): (
                LivingScheduledForecastJobV2<'static>,
                _,
                _,
                _,
                _,
            ) = {
                let authority = builtin_living_sandbox_v2().unwrap();
                let source = std::sync::Arc::new(authority.state().clone());
                let actor = source.civilizations.as_slice()[0].id;
                let before = source.as_ref().clone();
                let mut state = before.clone();
                let mut events = super::super::receipt::LivingPendingEventsV2::new();
                let ctx = LivingStepContextV2 {
                    catalog: authority.catalog(),
                    genesis_seed: [0; 32],
                    branch: authority.branch(),
                    tick: LivingTickV2(50),
                    state: &mut state,
                    events: &mut events,
                    arrivals: Vec::new(),
                    repair_exclusions: Default::default(),
                };
                let expected = reference_scheduled_forecast(&ctx, actor, false).unwrap();
                let catalog = authority.catalog_snapshot();
                let source_weak = std::sync::Arc::downgrade(&source);
                let catalog_weak = std::sync::Arc::downgrade(&catalog);
                let job = LivingScheduledForecastJobV2::begin_owned(
                    catalog,
                    source,
                    actor,
                    LivingTickV2(50),
                    [0; 32],
                    authority.branch(),
                );
                (job, expected, before, source_weak, catalog_weak)
            };
            assert_eq!(job.total_visits(), 0);
            assert_eq!(
                source.strong_count(),
                3,
                "forecast, input cursor and state-copy cursor share source"
            );
            assert_eq!(catalog.strong_count(), 1, "begin moves the catalog owner");
            assert_scheduled_forecast_retryable_budget(&mut job);
            let mut scheduled = false;
            let mut production = false;
            let mut dispatch = false;
            loop {
                let progress = job.poll(budget).unwrap();
                assert!(job.last_poll_visits() <= budget);
                if job.stage == ScheduledForecastStage::Scheduled && !scheduled {
                    assert_scheduled_forecast_retryable_budget(&mut job);
                    scheduled = true;
                }
                if job.production.is_some() {
                    if !production {
                        assert_scheduled_forecast_retryable_budget(&mut job);
                    }
                    production = true;
                    // Production now retires its original catalog during normal
                    // cleanup, before its exhausted Box is handed back.
                    let minimum = if job.production.as_ref().unwrap().retains_catalog_for_test() {
                        2
                    } else {
                        1
                    };
                    assert!(catalog.strong_count() >= minimum);
                    let super::super::step_job::LivingPhaseCatalogV2::Owned(parent_catalog) =
                        &*job.catalog
                    else {
                        panic!("owned forecast catalog anchor");
                    };
                    assert!(catalog.ptr_eq(&std::sync::Arc::downgrade(parent_catalog)));
                }
                if job.dispatch.is_some() {
                    if !dispatch {
                        assert_scheduled_forecast_retryable_budget(&mut job);
                    }
                    dispatch = true;
                    assert!(catalog.strong_count() >= 2);
                }
                if progress == Progress::Ready {
                    break;
                }
            }
            if budget == 1 {
                assert!(
                    scheduled && production && dispatch,
                    "all scheduled children remain pending"
                );
            }
            if let Some(visits) = visits {
                assert_eq!(job.total_visits(), visits);
            }
            visits = Some(job.total_visits());
            let (output, retired_source) = job.finish().unwrap();
            assert_eq!(output, expected);
            assert_eq!(retired_source.as_ref(), &before);
            assert!(source.ptr_eq(&std::sync::Arc::downgrade(&retired_source)));
            assert_eq!(catalog.strong_count(), 0);
            assert_eq!(source.strong_count(), 1);
            drop(retired_source);
            assert_eq!(source.strong_count(), 0);
        }
    }

    fn intent_inputs_reference(
        authority: &LivingGalaxyAuthorityV2,
        source: &LivingGalaxyStateV2,
        tick: LivingTickV2,
    ) -> (
        super::super::ids::LivingStateDigestV2,
        Vec<(LivingEntityIdV2, LivingScheduledForecastV2)>,
        Vec<LivingReceiverDemandRequestV2>,
    ) {
        let digest = living_decision_snapshot_digest_v2(source, tick).unwrap();
        let mut state = source.clone();
        let mut events = super::super::receipt::LivingPendingEventsV2::new();
        let ctx = LivingStepContextV2 {
            catalog: authority.catalog(),
            genesis_seed: [0; 32],
            branch: authority.branch(),
            tick,
            state: &mut state,
            events: &mut events,
            arrivals: Vec::new(),
            repair_exclusions: Default::default(),
        };
        let forecasts = source
            .civilizations
            .as_slice()
            .iter()
            .filter(|civ| civ.status == LivingCivilizationStatusV2::Active)
            .map(|civ| {
                (
                    civ.id,
                    reference_scheduled_forecast(&ctx, civ.id, false).unwrap(),
                )
            })
            .collect::<Vec<_>>();
        let mut requests = Vec::new();
        for (receiver, forecast) in &forecasts {
            for (world, resource) in &forecast.deficits {
                let stock = world_quantity(source, *world, *resource);
                if owned_by(source, *world, *receiver) && stock < 20 {
                    requests.push(LivingReceiverDemandRequestV2 {
                        receiver: *receiver,
                        world: *world,
                        resource: *resource,
                        requested_units: 20 - stock,
                    });
                }
            }
        }
        (digest, forecasts, requests)
    }

    fn drain_intent_inputs(
        authority: &LivingGalaxyAuthorityV2,
        source: std::sync::Arc<LivingGalaxyStateV2>,
        budgets: &[usize],
    ) -> (LivingIntentInputsV2, u64) {
        use super::super::step_job::{
            LivingLifecycleJobErrorV2 as Error, LivingLifecycleProgressV2 as Progress,
        };
        let before = source.as_ref().clone();
        let mut job = LivingIntentInputsJobV2::begin(
            authority.catalog_snapshot(),
            std::sync::Arc::clone(&source),
            LivingTickV2(source.tick.0 + 1),
            [0; 32],
            authority.branch(),
        );
        assert!(matches!(job.finish(), Err(Error::NotReady)));
        assert_eq!(job.poll(1), Ok(Progress::Pending));
        assert_eq!(job.last_poll_visits, 1);
        assert!(job.forecasts.is_empty());
        let mut polls = 0;
        loop {
            let budget = budgets[polls % budgets.len()];
            let before_visits = job.total_visits;
            let progress = job.poll(budget).unwrap();
            assert!(job.last_poll_visits <= budget);
            assert_eq!(
                job.total_visits - before_visits,
                job.last_poll_visits as u64
            );
            polls += 1;
            if progress == Progress::Ready {
                break;
            }
        }
        let visits = job.total_visits;
        assert_eq!(job.poll(1), Ok(Progress::Ready));
        assert_eq!(job.last_poll_visits, 0);
        assert_eq!(job.total_visits, visits);
        assert_eq!(source.as_ref(), &before);
        let output = job.finish().unwrap();
        assert!(matches!(job.finish(), Err(Error::NotReady)));
        assert_eq!(job.poll(1024), Ok(Progress::Ready));
        assert_eq!(job.last_poll_visits, 0);
        assert_eq!(output.hash_owner.tick, LivingTickV2(source.tick.0 + 1));
        assert_eq!(output.hash_owner.digest().unwrap(), output.digest);
        (output, visits)
    }

    #[test]
    fn intent_inputs_match_reference_all_budgets_actor_counts_and_stock_thresholds() {
        let authority = builtin_living_sandbox_v2().unwrap();
        let mut fixtures = Vec::new();
        let mut state = authority.state().clone();
        state.tick = LivingTickV2(49);
        fixtures.push(state.clone());
        let mut dormant = state.clone();
        for id in dormant
            .civilizations
            .as_slice()
            .iter()
            .map(|civ| civ.id)
            .collect::<Vec<_>>()
        {
            dormant.civilizations.find_mut(&id).unwrap().status =
                LivingCivilizationStatusV2::Dormant;
        }
        fixtures.push(dormant);
        let mut empty = state.clone();
        empty.civilizations = Default::default();
        fixtures.push(empty);
        let mut many = state.clone();
        let mut civs = many.civilizations.as_slice().to_vec();
        for id in 230..235 {
            let mut civ = civs[0].clone();
            civ.id = eid(id);
            civs.push(civ);
        }
        many.civilizations = LivingSortedVecV2::new(civs).unwrap();
        fixtures.push(many);
        for stock in [0, 19, 20] {
            let mut threshold = route_adjustment_state();
            threshold.tick = LivingTickV2(49);
            for id in threshold
                .worlds
                .as_slice()
                .iter()
                .map(|world| world.id)
                .collect::<Vec<_>>()
            {
                let world = threshold.worlds.find_mut(&id).unwrap();
                world.inventory = LivingInventoryV2 {
                    energy: stock,
                    ore: stock,
                    alloy: stock,
                };
            }
            fixtures.push(threshold);
        }
        let mut lost = route_adjustment_state();
        lost.tick = LivingTickV2(49);
        lost.colonies = LivingSortedVecV2::new(
            lost.colonies
                .as_slice()
                .iter()
                .filter(|colony| colony.world != eid(8))
                .cloned()
                .collect(),
        )
        .unwrap();
        lost.worlds.find_mut(&eid(8)).unwrap().owner = None;
        fixtures.push(lost);
        for fixture in fixtures {
            let expected = intent_inputs_reference(&authority, &fixture, LivingTickV2(50));
            let source = std::sync::Arc::new(fixture);
            let mut visits = None;
            for budgets in [&[1][..], &[64][..], &[1024][..], &[1, 64, 1024, 7][..]] {
                let (output, actual) =
                    drain_intent_inputs(&authority, std::sync::Arc::clone(&source), budgets);
                assert_eq!(output.snapshot.as_ref(), source.as_ref());
                assert_eq!(output.digest, expected.0);
                assert_eq!(output.forecasts, expected.1);
                assert_eq!(output.requests, expected.2);
                if let Some(visits) = visits {
                    assert_eq!(actual, visits);
                }
                visits = Some(actual);
            }
        }
        assert_eq!(authority.state().tick, LivingTickV2(0));
    }

    #[test]
    fn intent_inputs_hidden_stale_observations_and_route_shipment_fallback_match_reference() {
        let authority = builtin_living_sandbox_v2().unwrap();
        for axis in 0..4 {
            let mut state = authority.state().clone();
            let actor = state.civilizations.as_slice()[0].id;
            let source_world = state
                .worlds
                .as_slice()
                .iter()
                .find(|world| world.owner == Some(actor))
                .unwrap()
                .id;
            let foreign = state
                .worlds
                .as_slice()
                .iter()
                .find(|world| world.owner.is_some() && world.owner != Some(actor))
                .unwrap();
            let destination = foreign.id;
            let receiver = foreign.owner.unwrap();
            state.tick = LivingTickV2(if axis == 2 { 149 } else { 49 });
            put(
                &mut state.routes,
                LivingFreightRouteV2 {
                    id: eid(200),
                    kind: LivingRouteKindV2::Aid,
                    source_world,
                    destination_world: destination,
                    resource: LivingResourceV2::Energy,
                    batch_size: 2,
                    cadence_ticks: 10,
                    source_reserve: 20,
                    source_owner: actor,
                    receiver_owner: receiver,
                    next_due: LivingTickV2(state.tick.0 + 1),
                    suspended: false,
                },
            )
            .unwrap();
            if axis == 3 {
                // A shipment remains a known fallback after its route disappears.
                state.routes = LivingSortedVecV2::new(
                    state
                        .routes
                        .as_slice()
                        .iter()
                        .filter(|route| route.id != eid(200))
                        .cloned()
                        .collect(),
                )
                .unwrap();
                put(
                    &mut state.shipments,
                    LivingShipmentV2 {
                        id: eid(201),
                        dispatch_owner: actor,
                        intended_receiver: receiver,
                        source_world,
                        destination_world: destination,
                        resource: LivingResourceV2::Energy,
                        units: 2,
                        departure_tick: LivingTickV2(40),
                        arrival_tick: LivingTickV2(60),
                        disposition: LivingShipmentDispositionV2::Outbound,
                        dispatch_kind: Some(LivingRouteKindV2::Aid),
                        returning_to_source: false,
                    },
                )
                .unwrap();
            }
            if axis == 1 || axis == 2 {
                put(
                    &mut state.observations,
                    LivingObservationV2 {
                        civilization: actor,
                        world: destination,
                        observed_tick: LivingTickV2(if axis == 1 { 49 } else { 0 }),
                        owner: None,
                        remaining_deposit: 0,
                        facilities: Default::default(),
                        stationed_hulls: Default::default(),
                        operational_defense_batteries: None,
                    },
                )
                .unwrap();
            }
            let tick = LivingTickV2(state.tick.0 + 1);
            let expected = intent_inputs_reference(&authority, &state, tick);
            let source = std::sync::Arc::new(state.clone());
            let (left, _) = drain_intent_inputs(&authority, source, &[1, 64, 1024]);
            assert_eq!(left.digest, expected.0);
            assert_eq!(left.forecasts, expected.1);
            assert_eq!(left.requests, expected.2);
            // Inventory is never carried by the explicit receiver observation.
            state.worlds.find_mut(&destination).unwrap().inventory = LivingInventoryV2 {
                energy: 999,
                ore: 888,
                alloy: 777,
            };
            let (right, _) = drain_intent_inputs(&authority, std::sync::Arc::new(state), &[64]);
            assert_eq!(
                left.forecasts.iter().find(|(id, _)| *id == actor),
                right.forecasts.iter().find(|(id, _)| *id == actor)
            );
            assert_eq!(
                left.requests
                    .iter()
                    .filter(|request| request.receiver == actor)
                    .collect::<Vec<_>>(),
                right
                    .requests
                    .iter()
                    .filter(|request| request.receiver == actor)
                    .collect::<Vec<_>>()
            );
        }
    }

    #[test]
    fn intent_inputs_cancel_every_parent_and_forecast_stage_retains_owners() {
        use super::super::step_job::{
            LivingLifecycleJobErrorV2 as Error, LivingLifecycleProgressV2 as Progress,
        };
        let authority = builtin_living_sandbox_v2().unwrap();
        let mut initial = route_adjustment_state();
        initial.tick = LivingTickV2(49);
        let source = std::sync::Arc::new(initial);
        let mut stages = std::collections::BTreeSet::new();
        let mut child_stages = std::collections::BTreeSet::new();
        // Discover the actual reachable stages, then recreate and cancel at each.
        let begin = || {
            LivingIntentInputsJobV2::begin(
                authority.catalog_snapshot(),
                std::sync::Arc::clone(&source),
                LivingTickV2(50),
                [0; 32],
                authority.branch(),
            )
        };
        let mut probe = begin();
        loop {
            stages.insert(probe.stage as u8);
            if let Some(child) = &probe.child {
                child_stages.insert(child.stage as u8);
            }
            if probe.poll(1).unwrap() == Progress::Ready {
                stages.insert(probe.stage as u8);
                break;
            }
        }
        for target in stages
            .iter()
            .map(|stage| (false, *stage))
            .chain(child_stages.iter().map(|stage| (true, *stage)))
        {
            let mut job = begin();
            while if target.0 {
                job.child
                    .as_ref()
                    .is_none_or(|child| child.stage as u8 != target.1)
            } else {
                job.stage as u8 != target.1
            } {
                let progress = job.poll(1).unwrap();
                assert!(
                    progress == Progress::Pending
                        || (!target.0 && target.1 == IntentInputsStage::Ready as u8)
                );
            }
            let owners = std::sync::Arc::strong_count(&source);
            let hash = job.hash_owner.as_ref().map(std::sync::Arc::downgrade);
            let visits = job.total_visits;
            for budget in [0, 1025] {
                assert_eq!(job.poll(budget), Err(Error::Budget));
                assert_eq!(job.last_poll_visits, 0);
            }
            job.cancel();
            for budget in [1, 1024] {
                assert_eq!(job.poll(budget), Err(Error::Cancelled));
                assert_eq!(job.last_poll_visits, 0);
            }
            assert!(matches!(job.finish(), Err(Error::Cancelled)));
            assert_eq!(job.total_visits, visits);
            assert_eq!(std::sync::Arc::strong_count(&source), owners);
            if let Some(hash) = hash {
                assert!(hash.upgrade().is_some());
            }
        }
        assert_eq!(stages.len(), 11); // BeginHash preserves the normalized owner while copy cleanup drains.
        assert!(stages.contains(&(IntentInputsStage::BeginHash as u8)));
        assert!(child_stages.contains(&(ScheduledForecastStage::Preparation as u8)));
        assert!(child_stages.contains(&(ScheduledForecastStage::Scheduled as u8)));
        assert!(child_stages.contains(&(ScheduledForecastStage::Production as u8)));
        assert!(child_stages.contains(&(ScheduledForecastStage::Dispatch as u8)));
    }

    #[test]
    fn intent_inputs_invalid_ticks_and_horizon_overflow_are_sticky_and_retain_source() {
        use super::super::step_job::{
            LivingLifecycleJobErrorV2 as Error, LivingLifecycleProgressV2 as Progress,
        };
        let authority = builtin_living_sandbox_v2().unwrap();
        for (source_tick, tick) in [(0, 0), (0, 50), (u64::MAX, 1), (u64::MAX - 1, u64::MAX)] {
            let mut state = authority.state().clone();
            state.tick = LivingTickV2(source_tick);
            let before = state.clone();
            let source = std::sync::Arc::new(state);
            let weak = std::sync::Arc::downgrade(&source);
            let mut job = LivingIntentInputsJobV2::begin(
                authority.catalog_snapshot(),
                source,
                LivingTickV2(tick),
                [0; 32],
                authority.branch(),
            );
            let error = loop {
                match job.poll(64) {
                    Err(error) => break error,
                    Ok(Progress::Pending) => {}
                    other => panic!("expected tick/horizon fault: {other:?}"),
                }
            };
            let visits = job.total_visits;
            if tick != u64::MAX {
                assert_eq!(visits, 0);
            } else {
                assert_eq!(error, Error::Fault(LivingDeterministicFaultV2::Arithmetic));
                assert!(job.child.is_some());
            }
            assert_eq!(job.poll(1), Err(error.clone()));
            assert_eq!(job.last_poll_visits, 0);
            assert_eq!(job.total_visits, visits);
            assert!(matches!(job.finish(), Err(ref actual) if actual == &error));
            assert_eq!(weak.upgrade().unwrap().as_ref(), &before);
        }
    }

    #[test]
    fn scheduled_forecast_cursor_slices_horizon_and_matches_reference_at_all_budgets() {
        use super::super::step_job::LivingLifecycleProgressV2 as Progress;
        let authority = builtin_living_sandbox_v2().unwrap();
        let source = std::sync::Arc::new(authority.state().clone());
        let actor = source.civilizations.as_slice()[0].id;
        let mut state = source.as_ref().clone();
        let mut events = super::super::receipt::LivingPendingEventsV2::new();
        let ctx = LivingStepContextV2 {
            catalog: authority.catalog(),
            genesis_seed: [0; 32],
            branch: authority.branch(),
            tick: LivingTickV2(50),
            state: &mut state,
            events: &mut events,
            arrivals: Vec::new(),
            repair_exclusions: Default::default(),
        };
        let expected = reference_scheduled_forecast(&ctx, actor, false).unwrap();
        let begin = |owned| {
            if owned {
                LivingScheduledForecastJobV2::begin_owned(
                    authority.catalog_snapshot(),
                    std::sync::Arc::clone(&source),
                    actor,
                    LivingTickV2(50),
                    [0; 32],
                    authority.branch(),
                )
            } else {
                LivingScheduledForecastJobV2::begin(
                    authority.catalog(),
                    std::sync::Arc::clone(&source),
                    actor,
                    LivingTickV2(50),
                    [0; 32],
                    authority.branch(),
                )
            }
        };
        let mut visits = None;
        for budget in [1, 64, 1024] {
            for owned in [false, true] {
                let mut job = begin(owned);
                assert_eq!(
                    job.poll(1).unwrap(),
                    Progress::Pending,
                    "one visited record cannot complete 200 boundaries"
                );
                while job.poll(budget).unwrap() != Progress::Ready {
                    assert!(job.last_poll_visits() <= budget);
                }
                assert!(job.last_poll_visits() <= budget);
                if let Some(visits) = visits {
                    assert_eq!(job.total_visits(), visits);
                }
                visits = Some(job.total_visits());
                assert_eq!(job.finish().unwrap().0, expected);
                assert_eq!(authority.state(), source.as_ref());
            }
        }
    }

    #[test]
    fn scheduled_forecast_cursor_cancel_budget_and_arithmetic_fault_preserve_source() {
        use super::super::step_job::{
            LivingLifecycleJobErrorV2 as Error, LivingLifecycleProgressV2 as Progress,
        };
        let authority = builtin_living_sandbox_v2().unwrap();
        let source = std::sync::Arc::new(authority.state().clone());
        let actor = source.civilizations.as_slice()[0].id;
        let begin = |tick| {
            LivingScheduledForecastJobV2::begin(
                authority.catalog(),
                std::sync::Arc::clone(&source),
                actor,
                tick,
                [0; 32],
                authority.branch(),
            )
        };
        assert!(matches!(
            begin(LivingTickV2(50)).finish(),
            Err(Error::NotReady)
        ));
        for stage in [
            ScheduledForecastStage::Preparation,
            ScheduledForecastStage::Scheduled,
            ScheduledForecastStage::Production,
            ScheduledForecastStage::Dispatch,
        ] {
            let mut job = begin(LivingTickV2(50));
            while job.stage != stage {
                assert_eq!(job.poll(1).unwrap(), Progress::Pending);
            }
            assert_scheduled_forecast_retryable_budget(&mut job);
            let visits = job.total_visits();
            job.cancel();
            assert_eq!(job.poll(1), Err(Error::Cancelled));
            assert_eq!(job.last_poll_visits(), 0);
            assert_eq!(job.total_visits(), visits);
            assert!(matches!(job.finish(), Err(Error::Cancelled)));
        }
        let mut job = begin(LivingTickV2(u64::MAX));
        loop {
            match job.poll(1) {
                Ok(Progress::Pending) => {}
                Err(Error::Fault(LivingDeterministicFaultV2::Arithmetic)) => break,
                other => panic!("expected a latched horizon overflow: {other:?}"),
            }
        }
        assert_eq!(
            job.poll(1024),
            Err(Error::Fault(LivingDeterministicFaultV2::Arithmetic))
        );
        assert_eq!(job.last_poll_visits(), 0);
        assert!(matches!(
            job.finish(),
            Err(Error::Fault(LivingDeterministicFaultV2::Arithmetic))
        ));
        assert_eq!(authority.state(), source.as_ref());
    }

    #[test]
    fn scheduled_forecast_cursor_owned_cancel_and_fault_after_callers_drop() {
        use super::super::step_job::{
            LivingLifecycleJobErrorV2 as Error, LivingLifecycleProgressV2 as Progress,
        };
        for stage in [
            ScheduledForecastStage::Preparation,
            ScheduledForecastStage::Scheduled,
            ScheduledForecastStage::Production,
            ScheduledForecastStage::Dispatch,
        ] {
            let authority = builtin_living_sandbox_v2().unwrap();
            let source = std::sync::Arc::new(authority.state().clone());
            let before = source.as_ref().clone();
            let catalog = authority.catalog_snapshot();
            let source_weak = std::sync::Arc::downgrade(&source);
            let catalog_weak = std::sync::Arc::downgrade(&catalog);
            let mut job = LivingScheduledForecastJobV2::begin_owned(
                std::sync::Arc::clone(&catalog),
                std::sync::Arc::clone(&source),
                source.civilizations.as_slice()[0].id,
                LivingTickV2(50),
                [0; 32],
                authority.branch(),
            );
            while job.stage != stage {
                assert_eq!(job.poll(1).unwrap(), Progress::Pending);
            }
            assert_eq!(job.poll(1).unwrap(), Progress::Pending);
            assert!(job.stage == stage, "cancel while the child remains pending");
            drop(source);
            drop(catalog);
            drop(authority);
            assert_eq!(source_weak.upgrade().unwrap().as_ref(), &before);
            assert_scheduled_forecast_retryable_budget(&mut job);
            let visits = job.total_visits();
            job.cancel();
            assert_eq!(job.poll(1), Err(Error::Cancelled));
            assert_eq!(job.last_poll_visits(), 0);
            assert_eq!(job.total_visits(), visits);
            assert_eq!(source_weak.upgrade().unwrap().as_ref(), &before);
            assert!(matches!(job.finish(), Err(Error::Cancelled)));
            assert_eq!(source_weak.strong_count(), 0);
            assert_eq!(catalog_weak.strong_count(), 0);
        }
        let begin = || -> LivingScheduledForecastJobV2<'static> {
            let authority = builtin_living_sandbox_v2().unwrap();
            let source = std::sync::Arc::new(authority.state().clone());
            let actor = source.civilizations.as_slice()[0].id;
            LivingScheduledForecastJobV2::begin_owned(
                authority.catalog_snapshot(),
                source,
                actor,
                LivingTickV2(u64::MAX),
                [0; 32],
                authority.branch(),
            )
        };
        assert!(matches!(begin().finish(), Err(Error::NotReady)));
        let mut job = begin();
        loop {
            match job.poll(1) {
                Ok(Progress::Pending) => {}
                Err(Error::Fault(LivingDeterministicFaultV2::Arithmetic)) => break,
                other => panic!("expected a latched horizon overflow: {other:?}"),
            }
        }
        let visits = job.total_visits();
        assert_eq!(
            job.poll(1024),
            Err(Error::Fault(LivingDeterministicFaultV2::Arithmetic))
        );
        assert_eq!(job.last_poll_visits(), 0);
        assert_eq!(job.total_visits(), visits);
        assert!(matches!(
            job.finish(),
            Err(Error::Fault(LivingDeterministicFaultV2::Arithmetic))
        ));
    }

    #[test]
    fn scheduled_forecast_cursor_paid_build_repair_trade_expiry_and_hazard_match_reference() {
        use super::super::step_job::LivingLifecycleProgressV2 as Progress;
        let authority = builtin_living_sandbox_v2().unwrap();
        let mut state = authority.state().clone();
        let actor = eid(3);
        put(
            &mut state.systems,
            LivingSystemV2 {
                id: eid(200),
                name: name("Forecast frontier").unwrap(),
            },
        )
        .unwrap();
        state.worlds.find_mut(&eid(6)).unwrap().system = eid(200);
        state.worlds.find_mut(&eid(8)).unwrap().system = eid(200);
        put(
            &mut state.lanes,
            LivingLaneV2 {
                id: eid(201),
                system_a: eid(1),
                system_b: eid(200),
                distance_units: 250,
            },
        )
        .unwrap();
        put(
            &mut state.hazards,
            LivingHazardV2 {
                id: eid(202),
                definition: slug("ion_storm").unwrap(),
                lane: eid(201),
                start_tick: LivingTickV2(40),
                end_tick: LivingTickV2(120),
            },
        )
        .unwrap();
        put(
            &mut state.routes,
            LivingFreightRouteV2 {
                id: eid(203),
                kind: LivingRouteKindV2::Trade,
                source_world: eid(5),
                destination_world: eid(6),
                resource: LivingResourceV2::Energy,
                batch_size: 10,
                cadence_ticks: 10,
                source_reserve: 20,
                source_owner: actor,
                receiver_owner: eid(4),
                next_due: LivingTickV2(60),
                suspended: false,
            },
        )
        .unwrap();
        put(
            &mut state.agreements,
            LivingAgreementV2 {
                kind: LivingAgreementKindV2::Trade,
                participant_a: actor,
                participant_b: eid(4),
                start_tick: LivingTickV2(0),
                end_tick: LivingTickV2(100),
            },
        )
        .unwrap();
        put(
            &mut state.construction_jobs,
            LivingConstructionJobV2 {
                id: eid(204),
                world: eid(5),
                owner: actor,
                definition: slug("foundry").unwrap(),
                accepted_tick: LivingTickV2(0),
                completion_tick: LivingTickV2(70),
                paid_alloy: 40,
            },
        )
        .unwrap();
        let fleet = state.fleets.find_mut(&eid(20)).unwrap();
        let hull = fleet.hulls.find_mut(&eid(21)).unwrap();
        hull.hit_points = 1;
        hull.next_repair = Some(LivingTickV2(51));
        state.worlds.find_mut(&eid(5)).unwrap().inventory = LivingInventoryV2 {
            energy: 0,
            ore: 20,
            alloy: 0,
        };
        state.validate(authority.catalog()).unwrap();
        let source = std::sync::Arc::new(state.clone());
        let mut events = super::super::receipt::LivingPendingEventsV2::new();
        let ctx = LivingStepContextV2 {
            catalog: authority.catalog(),
            genesis_seed: [0; 32],
            branch: authority.branch(),
            tick: LivingTickV2(50),
            state: &mut state,
            events: &mut events,
            arrivals: Vec::new(),
            repair_exclusions: Default::default(),
        };
        let expected = reference_scheduled_forecast(&ctx, actor, false).unwrap();
        let begin = |owned| {
            if owned {
                LivingScheduledForecastJobV2::begin_owned(
                    authority.catalog_snapshot(),
                    std::sync::Arc::clone(&source),
                    actor,
                    LivingTickV2(50),
                    [0; 32],
                    authority.branch(),
                )
            } else {
                LivingScheduledForecastJobV2::begin(
                    authority.catalog(),
                    std::sync::Arc::clone(&source),
                    actor,
                    LivingTickV2(50),
                    [0; 32],
                    authority.branch(),
                )
            }
        };
        let mut visits = None;
        for budget in [1, 64, 1024] {
            for owned in [false, true] {
                let mut job = begin(owned);
                while job.poll(budget).unwrap() != Progress::Ready {
                    assert!(job.last_poll_visits() <= budget);
                }
                if let Some(visits) = visits {
                    assert_eq!(job.total_visits(), visits);
                }
                visits = Some(job.total_visits());
                let output = job.finish().unwrap().0;
                assert_eq!(output, expected);
                assert_eq!(output.final_state.tick, LivingTickV2(250));
                assert!(output.final_state.construction_jobs.as_slice().is_empty());
                assert_eq!(source.as_ref(), ctx.state);
            }
        }
    }

    #[test]
    fn forecast_input_cursor_projects_actor_private_state_across_budgets() {
        use super::super::step_job::LivingLifecycleProgressV2;
        let authority = builtin_living_sandbox_v2().unwrap();
        let source = std::sync::Arc::new(authority.state().clone());
        let mut total = None;
        for civ in source.civilizations.as_slice() {
            let expected = reference_forecast_input(&source, civ.id, LivingTickV2(50)).unwrap();
            for budget in [1, 64, 1024] {
                let mut job = LivingForecastInputJobV2::begin(
                    std::sync::Arc::clone(&source),
                    civ.id,
                    LivingTickV2(50),
                );
                loop {
                    let progress = job.poll(budget).unwrap();
                    assert!(job.last_poll_visits() <= budget);
                    if progress == LivingLifecycleProgressV2::Ready {
                        break;
                    }
                }
                if budget == 1 {
                    total = Some(job.total_visits());
                } else {
                    assert_eq!(Some(job.total_visits()), total);
                }
                let output = job.finish().unwrap();
                assert_eq!((output.state, output.idle_facilities), expected);
                assert_eq!(authority.state(), source.as_ref());
            }
        }
    }

    #[test]
    fn forecast_input_cursor_rejects_cancelled_incomplete_and_invalid_budget_work() {
        use super::super::step_job::{
            LivingLifecycleJobErrorV2 as Error, LivingLifecycleProgressV2 as Progress,
        };
        let authority = builtin_living_sandbox_v2().unwrap();
        let source = std::sync::Arc::new(authority.state().clone());
        let actor = source.civilizations.as_slice()[0].id;
        let begin = || {
            LivingForecastInputJobV2::begin(std::sync::Arc::clone(&source), actor, LivingTickV2(50))
        };
        assert!(matches!(begin().finish(), Err(Error::NotReady)));
        for stage in [
            ForecastInputStage::Copy,
            ForecastInputStage::Facilities,
            ForecastInputStage::Observations,
            ForecastInputStage::RetireOwners,
        ] {
            let mut job = begin();
            while job.stage != stage {
                assert_eq!(job.poll(1).unwrap(), Progress::Pending);
            }
            let before = job.total_visits();
            assert_eq!(job.poll(0), Err(Error::Budget));
            assert_eq!(job.poll(1025), Err(Error::Budget));
            assert_eq!(job.last_poll_visits(), 0);
            assert_eq!(job.total_visits(), before);
            job.cancel();
            assert_eq!(job.poll(1), Err(Error::Cancelled));
            assert_eq!(job.last_poll_visits(), 0);
            assert!(matches!(job.finish(), Err(Error::Cancelled)));
            assert_eq!(authority.state(), source.as_ref());
        }
    }

    #[test]
    fn forecast_input_cursor_masks_unseen_foreign_ownership_and_inventory() {
        use super::super::step_job::LivingLifecycleProgressV2 as Progress;
        let authority = builtin_living_sandbox_v2().unwrap();
        let mut left = authority.state().clone();
        let actor = left.civilizations.as_slice()[0].id;
        let foreign = left
            .worlds
            .as_slice()
            .iter()
            .find(|world| world.owner.is_some() && world.owner != Some(actor))
            .unwrap()
            .id;
        left.worlds.find_mut(&foreign).unwrap().inventory = LivingInventoryV2 {
            energy: 1,
            ore: 2,
            alloy: 3,
        };
        let mut right = left.clone();
        // Unseen ownership loss must not change the actor's projected state.
        let world = right.worlds.find_mut(&foreign).unwrap();
        world.owner = None;
        world.inventory = LivingInventoryV2 {
            energy: 900,
            ore: 800,
            alloy: 700,
        };
        let project = |state| {
            let mut job = LivingForecastInputJobV2::begin(
                std::sync::Arc::new(state),
                actor,
                LivingTickV2(50),
            );
            while job.poll(1).unwrap() != Progress::Ready {}
            let output = job.finish().unwrap();
            (output.state, output.idle_facilities)
        };
        assert_eq!(project(left), project(right));
    }

    #[test]
    fn forecast_input_cursor_observed_unowned_world_overrides_route_fallback() {
        use super::super::step_job::LivingLifecycleProgressV2 as Progress;
        let authority = builtin_living_sandbox_v2().unwrap();
        let mut state = authority.state().clone();
        let actor = state.civilizations.as_slice()[0].id;
        let source_world = state
            .worlds
            .as_slice()
            .iter()
            .find(|world| world.owner == Some(actor))
            .unwrap()
            .id;
        let foreign = state
            .worlds
            .as_slice()
            .iter()
            .find(|world| world.owner.is_some() && world.owner != Some(actor))
            .unwrap();
        let destination = foreign.id;
        let receiver_owner = foreign.owner.unwrap();
        put(
            &mut state.routes,
            LivingFreightRouteV2 {
                id: eid(200),
                kind: LivingRouteKindV2::Aid,
                source_world,
                destination_world: destination,
                resource: LivingResourceV2::Energy,
                batch_size: 2,
                cadence_ticks: 10,
                source_reserve: 20,
                source_owner: actor,
                receiver_owner,
                next_due: LivingTickV2(50),
                suspended: false,
            },
        )
        .unwrap();
        put(
            &mut state.observations,
            LivingObservationV2 {
                civilization: actor,
                world: destination,
                observed_tick: LivingTickV2(49),
                owner: None,
                remaining_deposit: 0,
                facilities: Default::default(),
                stationed_hulls: Default::default(),
                operational_defense_batteries: None,
            },
        )
        .unwrap();
        let expected = reference_forecast_input(&state, actor, LivingTickV2(50)).unwrap();
        let mut job =
            LivingForecastInputJobV2::begin(std::sync::Arc::new(state), actor, LivingTickV2(50));
        while job.poll(1).unwrap() != Progress::Ready {}
        let output = job.finish().unwrap();
        assert_eq!(
            output
                .state
                .worlds
                .as_slice()
                .iter()
                .find(|world| world.id == destination)
                .unwrap()
                .owner,
            None
        );
        assert_eq!((output.state, output.idle_facilities), expected);
    }

    #[test]
    fn forecast_uses_paid_queue_completion_and_first_recipe_without_new_choices() {
        let authority = builtin_first_expansion_v2().unwrap();
        let mut state = authority.state().clone();
        let actor = state.civilizations.as_slice()[0].id;
        let home = state.colonies.as_slice()[0].world;
        state.worlds.find_mut(&home).unwrap().inventory.alloy = 80;
        put(
            &mut state.construction_jobs,
            LivingConstructionJobV2 {
                id: eid(60),
                world: home,
                owner: actor,
                definition: slug("foundry").unwrap(),
                accepted_tick: LivingTickV2(0),
                completion_tick: LivingTickV2(10),
                paid_alloy: 40,
            },
        )
        .unwrap();
        let before = state.clone();
        let mut events = super::super::receipt::LivingPendingEventsV2::new();
        let ctx = LivingStepContextV2 {
            catalog: authority.catalog(),
            genesis_seed: [0; 32],
            branch: authority.branch(),
            tick: LivingTickV2(0),
            state: &mut state,
            events: &mut events,
            arrivals: Vec::new(),
            repair_exclusions: Default::default(),
        };
        let forecast = forecast_scheduled_economy(&ctx, actor).unwrap();
        let at = |tick| {
            forecast
                .inventories
                .iter()
                .find(|(boundary, world, _)| boundary.0 == tick && *world == home)
                .unwrap()
                .2
        };
        assert_eq!(at(9).alloy, 80);
        assert_eq!(at(10).alloy, 80, "completion neither repays nor produces");
        assert_eq!(at(29).alloy, 80);
        assert_eq!(
            at(30).alloy,
            82,
            "first foundry recipe is completion plus twenty"
        );
        assert_eq!(forecast.final_state.tick, LivingTickV2(200));
        assert!(forecast.final_state.construction_jobs.as_slice().is_empty());
        assert!(
            forecast.final_state.hull_jobs.as_slice().is_empty(),
            "no desired ark is manufactured by the forecast"
        );
        assert_eq!(
            forecast.final_state.fleets, before.fleets,
            "no exploratory intent is generated"
        );
        assert_eq!(*ctx.state, before, "forecast never mutates authority input");
    }

    #[test]
    fn forecast_does_not_depend_on_private_rival_inventory() {
        let authority = builtin_living_sandbox_v2().unwrap();
        let mut left = authority.state().clone();
        let mut right = left.clone();
        right.worlds.find_mut(&eid(6)).unwrap().inventory = LivingInventoryV2 {
            energy: 500,
            ore: 500,
            alloy: 500,
        };
        let project = |state: &mut LivingGalaxyStateV2| {
            let mut events = super::super::receipt::LivingPendingEventsV2::new();
            let ctx = LivingStepContextV2 {
                catalog: authority.catalog(),
                genesis_seed: [0; 32],
                branch: authority.branch(),
                tick: LivingTickV2(0),
                state,
                events: &mut events,
                arrivals: Vec::new(),
                repair_exclusions: Default::default(),
            };
            forecast_scheduled_economy(&ctx, eid(3)).unwrap()
        };
        assert_eq!(
            project(&mut left).inventories,
            project(&mut right).inventories
        );
    }

    #[test]
    fn competing_settlement_and_attack_reserve_only_one_fleet_intent() {
        for (policy_name, expected_kind) in [
            ("neutral", LivingFleetOrderKindV2::Attack),
            ("expansionist", LivingFleetOrderKindV2::Settle),
        ] {
            let authority = builtin_living_sandbox_v2().unwrap();
            let mut state = authority.state().clone();
            state.tick = LivingTickV2(49);
            state.civilizations.find_mut(&eid(3)).unwrap().policy =
                LivingPolicyV2(slug(policy_name).unwrap());
            put(
                &mut state.fleets,
                fleet_with(
                    eid(50),
                    eid(3),
                    eid(5),
                    vec![hull(eid(51), "colony_ark", false).unwrap()],
                )
                .unwrap(),
            )
            .unwrap();
            put(
                &mut state.fleets,
                fleet_with(
                    eid(52),
                    eid(3),
                    eid(5),
                    vec![hull(eid(53), "escort", false).unwrap()],
                )
                .unwrap(),
            )
            .unwrap();
            put(
                &mut state.wars,
                LivingWarV2 {
                    participant_a: eid(3),
                    participant_b: eid(4),
                    declarer: eid(3),
                    start_tick: LivingTickV2(1),
                    end_tick: LivingTickV2(500),
                },
            )
            .unwrap();
            let mut events = super::super::receipt::LivingPendingEventsV2::new();
            let mut ctx = LivingStepContextV2 {
                catalog: authority.catalog(),
                genesis_seed: [0; 32],
                branch: authority.branch(),
                tick: LivingTickV2(50),
                state: &mut state,
                events: &mut events,
                arrivals: Vec::new(),
                repair_exclusions: Default::default(),
            };
            for target in [eid(6), eid(7)] {
                let observation = observe_world(&ctx, eid(3), target, ctx.tick).unwrap();
                put(&mut ctx.state.observations, observation).unwrap();
            }
            generate_and_reserve_intents(&mut ctx).unwrap();
            let orders: Vec<_> = ctx
                .state
                .fleets
                .as_slice()
                .iter()
                .filter(|fleet| fleet.owner == eid(3))
                .filter_map(|fleet| fleet.order.as_ref())
                .collect();
            assert_eq!(orders.len(), 1, "one actor cannot accept two role orders");
            assert_eq!(orders[0].kind, expected_kind);
        }
    }

    #[test]
    fn defense_is_actionable_and_precedes_high_scoring_settlement() {
        let authority = builtin_living_sandbox_v2().unwrap();
        let mut state = authority.state().clone();
        state.tick = LivingTickV2(49);
        state.worlds.find_mut(&eid(6)).unwrap().owner = Some(eid(3));
        state.colonies.find_mut(&eid(6)).unwrap().owner = eid(3);
        put(
            &mut state.wars,
            LivingWarV2 {
                participant_a: eid(3),
                participant_b: eid(4),
                declarer: eid(4),
                start_tick: LivingTickV2(1),
                end_tick: LivingTickV2(500),
            },
        )
        .unwrap();
        put(
            &mut state.fleets,
            fleet_with(
                eid(60),
                eid(4),
                eid(6),
                vec![hull(eid(61), "escort", true).unwrap()],
            )
            .unwrap(),
        )
        .unwrap();
        put(
            &mut state.fleets,
            fleet_with(
                eid(62),
                eid(3),
                eid(5),
                vec![hull(eid(63), "escort", false).unwrap()],
            )
            .unwrap(),
        )
        .unwrap();
        put(
            &mut state.fleets,
            fleet_with(
                eid(64),
                eid(3),
                eid(5),
                vec![hull(eid(65), "colony_ark", false).unwrap()],
            )
            .unwrap(),
        )
        .unwrap();
        state.civilizations.find_mut(&eid(3)).unwrap().policy =
            LivingPolicyV2(slug("expansionist").unwrap());
        let mut events = super::super::receipt::LivingPendingEventsV2::new();
        let mut ctx = LivingStepContextV2 {
            catalog: authority.catalog(),
            genesis_seed: [0; 32],
            branch: authority.branch(),
            tick: LivingTickV2(50),
            state: &mut state,
            events: &mut events,
            arrivals: Vec::new(),
            repair_exclusions: Default::default(),
        };
        let observation = observe_world(&ctx, eid(3), eid(7), ctx.tick).unwrap();
        put(&mut ctx.state.observations, observation).unwrap();
        generate_and_reserve_intents(&mut ctx).unwrap();
        let decision = ctx
            .events
            .decisions
            .iter()
            .find(|row| {
                row.actor == eid(3)
                    && matches!(row.action, LivingIntentActionV2::Defend { .. })
                    && row.outcome == LivingDecisionOutcomeV2::Accepted
            })
            .unwrap();
        assert_eq!(decision.target, Some(eid(6)));
        assert_eq!(decision.score.0, 60);
        assert!(
            !ctx.events.decisions.iter().any(|row| row.actor == eid(3)
                && matches!(row.action, LivingIntentActionV2::Settle { .. }))
        );
        let LivingIntentActionV2::Defend {
            fleet: ordered_fleet,
            ..
        } = decision.action
        else {
            panic!("defense action")
        };
        let order = ctx
            .state
            .fleets
            .as_slice()
            .iter()
            .find(|row| row.id == ordered_fleet)
            .unwrap()
            .order
            .as_ref()
            .unwrap();
        assert_eq!(order.kind, LivingFleetOrderKindV2::Defend);
        assert_eq!(order.not_before_tick, LivingTickV2(51));
    }

    #[test]
    fn guardian_reinforcement_keeps_ordinary_priority_despite_bonus() {
        let authority = builtin_living_sandbox_v2().unwrap();
        let mut state = authority.state().clone();
        state.tick = LivingTickV2(49);
        state.worlds.find_mut(&eid(6)).unwrap().owner = Some(eid(3));
        state.colonies.find_mut(&eid(6)).unwrap().owner = eid(3);
        put(
            &mut state.fleets,
            fleet_with(
                eid(62),
                eid(3),
                eid(5),
                vec![hull(eid(63), "escort", false).unwrap()],
            )
            .unwrap(),
        )
        .unwrap();
        let snapshot = state.clone();
        let mut events = super::super::receipt::LivingPendingEventsV2::new();
        let mut ctx = LivingStepContextV2 {
            catalog: authority.catalog(),
            genesis_seed: [0; 32],
            branch: authority.branch(),
            tick: LivingTickV2(50),
            state: &mut state,
            events: &mut events,
            arrivals: Vec::new(),
            repair_exclusions: Default::default(),
        };
        reserve_guard(&mut ctx, eid(3), true).unwrap();
        assert_eq!(
            *ctx.state, snapshot,
            "no actionable attack means no emergency order"
        );
        reserve_guard(&mut ctx, eid(3), false).unwrap();
        let decision = fleet_proposal_decision(
            &snapshot,
            ctx.state,
            eid(3),
            "guardian",
            ctx.tick,
            living_decision_snapshot_digest_v2(&snapshot, ctx.tick).unwrap(),
        )
        .unwrap()
        .unwrap();
        assert_eq!(decision.target, Some(eid(6)));
        assert_eq!(decision.score.0, 60);
        assert_eq!(fleet_priority(&snapshot, ctx.tick, &decision), 2);
    }

    #[test]
    fn disabled_battery_remains_targetable_without_firing_or_defending() {
        for disabled in [false, true] {
            let authority = builtin_living_sandbox_v2().unwrap();
            let mut state = authority.state().clone();
            state.fleets = LivingSortedVecV2::default();
            let mut battery = facility(eid(60), eid(5), "defense_battery", None).unwrap();
            battery.hit_points = 40;
            battery.disabled = disabled;
            put(&mut state.facilities, battery).unwrap();
            put(
                &mut state.fleets,
                fleet_with(
                    eid(62),
                    eid(4),
                    eid(5),
                    vec![hull(eid(63), "escort", true).unwrap()],
                )
                .unwrap(),
            )
            .unwrap();
            put(
                &mut state.wars,
                LivingWarV2 {
                    participant_a: eid(3),
                    participant_b: eid(4),
                    declarer: eid(4),
                    start_tick: LivingTickV2(1),
                    end_tick: LivingTickV2(500),
                },
            )
            .unwrap();
            let mut events = super::super::receipt::LivingPendingEventsV2::new();
            let mut ctx = LivingStepContextV2 {
                catalog: authority.catalog(),
                genesis_seed: [0; 32],
                branch: authority.branch(),
                tick: LivingTickV2(50),
                state: &mut state,
                events: &mut events,
                arrivals: Vec::new(),
                repair_exclusions: Default::default(),
            };
            let observation = observe_world(&ctx, eid(3), eid(5), ctx.tick).unwrap();
            assert_eq!(
                observation.operational_defense_batteries,
                if disabled { Some(0) } else { None }
            );
            put(&mut ctx.state.observations, observation).unwrap();
            assert_eq!(
                observed_defense(ctx.state, eid(3), eid(5)),
                if disabled { 10 } else { 30 }
            );
            assert_eq!(defense_present(&ctx, eid(5), eid(3)), !disabled);
            resolve_combat(&mut ctx).unwrap();
            let battery = ctx
                .state
                .facilities
                .as_slice()
                .iter()
                .find(|row| row.id == eid(60))
                .unwrap();
            assert!(
                battery.hit_points < 40,
                "disabled installations remain combat targets"
            );
            let enemy = ctx
                .state
                .fleets
                .as_slice()
                .iter()
                .find(|fleet| fleet.id == eid(62))
                .unwrap();
            let hp = enemy.hulls.as_slice()[0].hit_points;
            if disabled {
                assert_eq!(hp, 10, "disabled battery never fires");
            } else {
                assert!(hp < 10, "enabled battery inflicts damage");
            }
        }
    }

    #[test]
    fn dormant_settlement_recovery_uses_only_existing_credits() {
        for credited in [false, true] {
            let authority = builtin_living_sandbox_v2().unwrap();
            let mut state = authority.state().clone();
            state.civilizations.find_mut(&eid(3)).unwrap().status =
                LivingCivilizationStatusV2::Dormant;
            remove_key(&mut state.colonies, &eid(5)).unwrap();
            state.worlds.find_mut(&eid(5)).unwrap().owner = None;
            state.worlds.find_mut(&eid(5)).unwrap().inventory.energy = 0;
            put(
                &mut state.fleets,
                fleet_with(
                    eid(60),
                    eid(3),
                    eid(5),
                    vec![hull(eid(61), "colony_ark", credited).unwrap()],
                )
                .unwrap(),
            )
            .unwrap();
            let mut events = super::super::receipt::LivingPendingEventsV2::new();
            let mut ctx = LivingStepContextV2 {
                catalog: authority.catalog(),
                genesis_seed: [0; 32],
                branch: authority.branch(),
                tick: LivingTickV2(50),
                state: &mut state,
                events: &mut events,
                arrivals: Vec::new(),
                repair_exclusions: Default::default(),
            };
            let observation = observe_world(&ctx, eid(3), eid(7), ctx.tick).unwrap();
            put(&mut ctx.state.observations, observation).unwrap();
            reserve_settlement(&mut ctx, eid(3), "neutral").unwrap();
            let fleet = ctx
                .state
                .fleets
                .as_slice()
                .iter()
                .find(|row| row.id == eid(60))
                .unwrap();
            assert_eq!(fleet.order.is_some(), credited);
            if credited {
                ctx.tick = LivingTickV2(51);
                dispatch_fleets(&mut ctx).unwrap();
                let fleet = ctx
                    .state
                    .fleets
                    .as_slice()
                    .iter()
                    .find(|row| row.id == eid(60))
                    .unwrap();
                assert!(matches!(
                    fleet.location,
                    LivingFleetLocationV2::Travelling { .. }
                ));
                assert!(!fleet.hulls.as_slice()[0].return_credit);
            }
            assert_eq!(
                world_quantity(ctx.state, eid(5), LivingResourceV2::Energy),
                0
            );
        }
    }

    #[test]
    fn receiver_owned_demand_enables_trade_or_trader_aid_without_private_stock_reads() {
        for trade in [false, true] {
            let authority = builtin_living_sandbox_v2().unwrap();
            let mut state = authority.state().clone();
            state.tick = LivingTickV2(49);
            state.worlds.find_mut(&eid(6)).unwrap().inventory.ore = 0;
            put(
                &mut state.facilities,
                facility(eid(60), eid(6), "foundry", Some(LivingTickV2(50))).unwrap(),
            )
            .unwrap();
            state.civilizations.find_mut(&eid(3)).unwrap().policy =
                LivingPolicyV2(slug(if trade { "neutral" } else { "trader" }).unwrap());
            if trade {
                put(
                    &mut state.agreements,
                    LivingAgreementV2 {
                        kind: LivingAgreementKindV2::Trade,
                        participant_a: eid(3),
                        participant_b: eid(4),
                        start_tick: LivingTickV2(1),
                        end_tick: LivingTickV2(1201),
                    },
                )
                .unwrap();
            }
            let mut events = super::super::receipt::LivingPendingEventsV2::new();
            let mut ctx = LivingStepContextV2 {
                catalog: authority.catalog(),
                genesis_seed: [0; 32],
                branch: authority.branch(),
                tick: LivingTickV2(50),
                state: &mut state,
                events: &mut events,
                arrivals: Vec::new(),
                repair_exclusions: Default::default(),
            };
            generate_and_reserve_intents(&mut ctx).unwrap();
            let route = ctx
                .state
                .routes
                .as_slice()
                .iter()
                .find(|route| {
                    route.source_owner == eid(3)
                        && route.receiver_owner == eid(4)
                        && route.destination_world == eid(6)
                        && route.resource == LivingResourceV2::Ore
                })
                .unwrap();
            assert_eq!(
                route.kind,
                if trade {
                    LivingRouteKindV2::Trade
                } else {
                    LivingRouteKindV2::Aid
                }
            );
            assert_eq!(route.source_reserve, 20);
            let decision = ctx.events.decisions.iter().find(|decision| decision.actor == eid(3)
                && matches!(decision.action, LivingIntentActionV2::Route { destination, resource: LivingResourceV2::Ore, .. } if destination == eid(6))
                && decision.outcome == LivingDecisionOutcomeV2::Accepted).unwrap();
            assert_eq!(decision.score.0, if trade { 70 } else { 90 });
            assert_eq!(decision.tier, LivingDecisionTierV2::Ordinary);
        }
    }

    #[test]
    fn committed_route_shortfall_is_demand_even_when_reserve_blocks_dispatch() {
        for suspended in [false, true] {
            let authority = builtin_living_sandbox_v2().unwrap();
            let mut state = authority.state().clone();
            state.facilities = LivingSortedVecV2::default();
            state.worlds.find_mut(&eid(5)).unwrap().inventory.ore = 20;
            state.routes.find_mut(&eid(22)).unwrap().suspended = suspended;
            let mut events = super::super::receipt::LivingPendingEventsV2::new();
            let ctx = LivingStepContextV2 {
                catalog: authority.catalog(),
                genesis_seed: [0; 32],
                branch: authority.branch(),
                tick: LivingTickV2(0),
                state: &mut state,
                events: &mut events,
                arrivals: Vec::new(),
                repair_exclusions: Default::default(),
            };
            let forecast = forecast_scheduled_economy(&ctx, eid(3)).unwrap();
            assert_eq!(
                forecast.deficits.contains(&(eid(5), LivingResourceV2::Ore)),
                !suspended
            );
            assert!(
                forecast
                    .inventories
                    .iter()
                    .filter(|(_, world, _)| *world == eid(5))
                    .all(|(_, _, stock)| stock.ore >= 20),
                "partial dispatch preserves the reserve"
            );
        }
    }

    #[test]
    fn exploration_refreshes_oldest_known_world_and_ignores_current_visibility() {
        for current in [false, true] {
            let authority = builtin_living_sandbox_v2().unwrap();
            let mut state = authority.state().clone();
            state.tick = LivingTickV2(49);
            put(
                &mut state.fleets,
                fleet_with(
                    eid(60),
                    eid(3),
                    eid(5),
                    vec![hull(eid(61), "scout", false).unwrap()],
                )
                .unwrap(),
            )
            .unwrap();
            let mut events = super::super::receipt::LivingPendingEventsV2::new();
            let mut ctx = LivingStepContextV2 {
                catalog: authority.catalog(),
                genesis_seed: [0; 32],
                branch: authority.branch(),
                tick: LivingTickV2(50),
                state: &mut state,
                events: &mut events,
                arrivals: Vec::new(),
                repair_exclusions: Default::default(),
            };
            for (world, observed) in [(eid(5), 50), (eid(6), 10), (eid(7), 1), (eid(8), 50)] {
                let observation = observe_world(
                    &ctx,
                    eid(3),
                    world,
                    LivingTickV2(if current { 50 } else { observed }),
                )
                .unwrap();
                put(&mut ctx.state.observations, observation).unwrap();
            }
            reserve_exploration(&mut ctx, eid(3)).unwrap();
            let orders: Vec<_> = ctx
                .state
                .fleets
                .as_slice()
                .iter()
                .filter(|fleet| fleet.owner == eid(3))
                .filter_map(|fleet| fleet.order.as_ref())
                .collect();
            if current {
                assert!(
                    orders.is_empty(),
                    "already visible worlds need no scout excursion"
                );
            } else {
                assert_eq!(orders.len(), 1);
                assert_eq!(orders[0].target_world, Some(eid(7)));
            }
        }
    }

    #[test]
    fn suspended_route_resumes_only_through_explicit_adjustment_without_duplicate() {
        let authority = builtin_living_sandbox_v2().unwrap();
        let mut state = authority.state().clone();
        state.tick = LivingTickV2(49);
        state.worlds.find_mut(&eid(6)).unwrap().inventory.ore = 0;
        state.civilizations.find_mut(&eid(3)).unwrap().policy =
            LivingPolicyV2(slug("trader").unwrap());
        put(
            &mut state.facilities,
            facility(eid(60), eid(6), "foundry", Some(LivingTickV2(50))).unwrap(),
        )
        .unwrap();
        put(
            &mut state.routes,
            LivingFreightRouteV2 {
                id: eid(70),
                kind: LivingRouteKindV2::Aid,
                source_world: eid(5),
                destination_world: eid(6),
                resource: LivingResourceV2::Ore,
                batch_size: 10,
                cadence_ticks: 10,
                source_reserve: 20,
                source_owner: eid(3),
                receiver_owner: eid(4),
                next_due: LivingTickV2(60),
                suspended: true,
            },
        )
        .unwrap();
        let mut events = super::super::receipt::LivingPendingEventsV2::new();
        let mut ctx = LivingStepContextV2 {
            catalog: authority.catalog(),
            genesis_seed: [0; 32],
            branch: authority.branch(),
            tick: LivingTickV2(50),
            state: &mut state,
            events: &mut events,
            arrivals: Vec::new(),
            repair_exclusions: Default::default(),
        };
        generate_and_reserve_intents(&mut ctx).unwrap();
        let routes: Vec<_> = ctx
            .state
            .routes
            .as_slice()
            .iter()
            .filter(|route| {
                route.source_world == eid(5)
                    && route.destination_world == eid(6)
                    && route.resource == LivingResourceV2::Ore
            })
            .collect();
        assert_eq!(routes.len(), 1);
        assert_eq!(routes[0].id, eid(70));
        assert!(!routes[0].suspended);
        assert_eq!(routes[0].next_due, LivingTickV2(60));
        assert!(ctx.events.decisions.iter().any(|decision| {
            decision.actor == eid(3)
                && decision.outcome == LivingDecisionOutcomeV2::Accepted
                && matches!(decision.action, LivingIntentActionV2::Route { existing_route: Some(id), .. } if id == eid(70))
        }));
    }

    #[test]
    fn full_colony_replaces_only_an_idle_duplicate_with_an_explicit_atomic_plan() {
        for stock in [0, 200] {
            let authority = builtin_first_expansion_v2().unwrap();
            let mut state = authority.state().clone();
            state.tick = LivingTickV2(49);
            remove_key(&mut state.facilities, &eid(10)).unwrap();
            for offset in 0..4 {
                put(
                    &mut state.facilities,
                    facility(
                        eid(60 + offset),
                        eid(4),
                        "solar_array",
                        Some(LivingTickV2(60)),
                    )
                    .unwrap(),
                )
                .unwrap();
            }
            state.worlds.find_mut(&eid(4)).unwrap().inventory = LivingInventoryV2 {
                energy: LIVING_RESOURCE_STORAGE_LIMIT_V2,
                ore: LIVING_RESOURCE_STORAGE_LIMIT_V2,
                alloy: stock,
            };
            let mut events = super::super::receipt::LivingPendingEventsV2::new();
            let mut ctx = LivingStepContextV2 {
                catalog: authority.catalog(),
                genesis_seed: [0; 32],
                branch: authority.branch(),
                tick: LivingTickV2(50),
                state: &mut state,
                events: &mut events,
                arrivals: Vec::new(),
                repair_exclusions: Default::default(),
            };
            let observation = observe_world(&ctx, eid(3), eid(5), ctx.tick).unwrap();
            put(&mut ctx.state.observations, observation).unwrap();
            let hub_before = ctx.state.colonies.as_slice()[0].hub.clone();
            let facilities_before = ctx.state.facilities.clone();
            generate_and_reserve_intents(&mut ctx).unwrap();
            if stock == 0 {
                assert_eq!(
                    ctx.state.facilities, facilities_before,
                    "unaffordable replacement never scraps"
                );
                assert!(ctx.state.construction_jobs.as_slice().is_empty());
                continue;
            }
            let decision = ctx
                .events
                .decisions
                .iter()
                .find(|decision| {
                    decision.actor == eid(3)
                        && decision.outcome == LivingDecisionOutcomeV2::Accepted
                        && matches!(decision.action, LivingIntentActionV2::Construction { .. })
                })
                .unwrap_or_else(|| panic!("decisions: {:?}", ctx.events.decisions));
            let LivingIntentActionV2::Construction {
                ref definition,
                scrap: Some(scrap),
                ..
            } = decision.action
            else {
                panic!("explicit scrap plan required");
            };
            assert_eq!(definition.as_str(), "shipyard");
            assert!(
                !ctx.state
                    .facilities
                    .as_slice()
                    .iter()
                    .any(|facility| facility.id == scrap)
            );
            assert_eq!(ctx.state.facilities.as_slice().len(), 5);
            assert!(ctx.state.facilities.as_slice().iter().any(|facility| {
                facility.definition.as_str() == "solar_array" && !facility.disabled
            }));
            assert!(
                ctx.state
                    .facilities
                    .as_slice()
                    .iter()
                    .any(|facility| facility.definition.as_str() == "extractor"
                        && !facility.disabled)
            );
            assert_eq!(ctx.state.colonies.as_slice()[0].hub, hub_before);
            let job = ctx
                .state
                .construction_jobs
                .as_slice()
                .iter()
                .find(|job| job.world == eid(4))
                .unwrap();
            assert_eq!(job.definition.as_str(), "shipyard");
            assert_eq!(
                ctx.state
                    .worlds
                    .as_slice()
                    .iter()
                    .find(|world| world.id == eid(4))
                    .unwrap()
                    .inventory
                    .alloy,
                stock - job.paid_alloy
            );
        }
    }

    #[test]
    fn defensive_role_split_preserves_hulls_and_reserves_only_its_launch_fuel() {
        let authority = builtin_living_sandbox_v2().unwrap();
        let mut state = authority.state().clone();
        state.worlds.find_mut(&eid(6)).unwrap().owner = Some(eid(3));
        state.colonies.find_mut(&eid(6)).unwrap().owner = eid(3);
        state.worlds.find_mut(&eid(5)).unwrap().inventory.energy = 6;
        put(
            &mut state.wars,
            LivingWarV2 {
                participant_a: eid(3),
                participant_b: eid(4),
                declarer: eid(4),
                start_tick: LivingTickV2(1),
                end_tick: LivingTickV2(500),
            },
        )
        .unwrap();
        put(
            &mut state.fleets,
            fleet_with(
                eid(60),
                eid(4),
                eid(6),
                vec![hull(eid(61), "escort", true).unwrap()],
            )
            .unwrap(),
        )
        .unwrap();
        put(
            &mut state.fleets,
            fleet_with(
                eid(62),
                eid(3),
                eid(5),
                vec![
                    hull(eid(63), "escort", false).unwrap(),
                    hull(eid(64), "escort", true).unwrap(),
                    hull(eid(65), "scout", true).unwrap(),
                ],
            )
            .unwrap(),
        )
        .unwrap();
        let mut events = super::super::receipt::LivingPendingEventsV2::new();
        let mut ctx = LivingStepContextV2 {
            catalog: authority.catalog(),
            genesis_seed: [0; 32],
            branch: authority.branch(),
            tick: LivingTickV2(50),
            state: &mut state,
            events: &mut events,
            arrivals: Vec::new(),
            repair_exclusions: Default::default(),
        };
        reserve_guard(&mut ctx, eid(3), true).unwrap();
        let retained = ctx
            .state
            .fleets
            .as_slice()
            .iter()
            .find(|fleet| fleet.id == eid(62))
            .unwrap();
        assert_eq!(retained.hulls.as_slice().len(), 1);
        assert_eq!(retained.hulls.as_slice()[0].id, eid(65));
        assert!(retained.hulls.as_slice()[0].return_credit);
        assert!(retained.order.is_none());
        let role = ctx
            .state
            .fleets
            .as_slice()
            .iter()
            .find(|fleet| {
                fleet.owner == eid(3)
                    && fleet
                        .order
                        .as_ref()
                        .is_some_and(|order| order.kind == LivingFleetOrderKindV2::Defend)
            })
            .unwrap();
        let role_id = role.id;
        assert_eq!(
            role.hulls
                .as_slice()
                .iter()
                .map(|hull| hull.id)
                .collect::<Vec<_>>(),
            vec![eid(63), eid(64)]
        );
        assert!(!role.hulls.as_slice()[0].return_credit);
        assert!(role.hulls.as_slice()[1].return_credit);
        assert_eq!(
            world_quantity(ctx.state, eid(5), LivingResourceV2::Energy),
            6
        );
        ctx.tick = LivingTickV2(51);
        dispatch_fleets(&mut ctx).unwrap();
        assert_eq!(
            world_quantity(ctx.state, eid(5), LivingResourceV2::Energy),
            0
        );
        let role = ctx
            .state
            .fleets
            .as_slice()
            .iter()
            .find(|fleet| fleet.id == role_id)
            .unwrap();
        assert!(matches!(
            role.location,
            LivingFleetLocationV2::Travelling { .. }
        ));
        assert!(role.hulls.as_slice().iter().all(|hull| hull.return_credit));
    }

    #[test]
    fn forecast_does_not_react_to_unseen_foreign_capture() {
        for observed in [false, true] {
            let authority = builtin_living_sandbox_v2().unwrap();
            let mut left = authority.state().clone();
            put(
                &mut left.routes,
                LivingFreightRouteV2 {
                    id: eid(70),
                    kind: LivingRouteKindV2::Aid,
                    source_world: eid(5),
                    destination_world: eid(6),
                    resource: LivingResourceV2::Ore,
                    batch_size: 10,
                    cadence_ticks: 10,
                    source_reserve: 20,
                    source_owner: eid(3),
                    receiver_owner: eid(4),
                    next_due: LivingTickV2(10),
                    suspended: false,
                },
            )
            .unwrap();
            if observed {
                let mut events = super::super::receipt::LivingPendingEventsV2::new();
                let ctx = LivingStepContextV2 {
                    catalog: authority.catalog(),
                    genesis_seed: [0; 32],
                    branch: authority.branch(),
                    tick: LivingTickV2(0),
                    state: &mut left,
                    events: &mut events,
                    arrivals: Vec::new(),
                    repair_exclusions: Default::default(),
                };
                let observation = observe_world(&ctx, eid(3), eid(6), ctx.tick).unwrap();
                put(&mut ctx.state.observations, observation).unwrap();
            }
            let mut right = left.clone();
            right.worlds.find_mut(&eid(6)).unwrap().owner = None;
            remove_key(&mut right.colonies, &eid(6)).unwrap();
            let project = |state: &mut LivingGalaxyStateV2| {
                let mut events = super::super::receipt::LivingPendingEventsV2::new();
                let ctx = LivingStepContextV2 {
                    catalog: authority.catalog(),
                    genesis_seed: [0; 32],
                    branch: authority.branch(),
                    tick: LivingTickV2(0),
                    state,
                    events: &mut events,
                    arrivals: Vec::new(),
                    repair_exclusions: Default::default(),
                };
                forecast_scheduled_economy(&ctx, eid(3)).unwrap()
            };
            let original = project(&mut left);
            let unseen_capture = project(&mut right);
            assert_eq!(original.inventories, unseen_capture.inventories);
            assert_eq!(original.deficits, unseen_capture.deficits);
            assert_eq!(
                right
                    .worlds
                    .as_slice()
                    .iter()
                    .find(|world| world.id == eid(6))
                    .unwrap()
                    .owner,
                None
            );
        }
    }

    #[test]
    fn impossible_supply_shortage_does_not_suppress_an_affordable_ark() {
        let authority = builtin_living_sandbox_v2().unwrap();
        let mut state = authority.state().clone();
        state.tick = LivingTickV2(49);
        // No world has energy surplus available for an immediate transfer.
        // That unactionable supply deficit cannot suppress a paid alloy-only job.
        for world in state.worlds.as_slice().to_vec() {
            state.worlds.find_mut(&world.id).unwrap().inventory.energy = 0;
        }
        let before = state
            .worlds
            .as_slice()
            .iter()
            .find(|world| world.id == eid(5))
            .unwrap()
            .inventory
            .alloy;
        let mut events = super::super::receipt::LivingPendingEventsV2::new();
        let mut ctx = LivingStepContextV2 {
            catalog: authority.catalog(),
            genesis_seed: [0; 32],
            branch: authority.branch(),
            tick: LivingTickV2(50),
            state: &mut state,
            events: &mut events,
            arrivals: Vec::new(),
            repair_exclusions: Default::default(),
        };
        let observation = observe_world(&ctx, eid(3), eid(7), ctx.tick).unwrap();
        put(&mut ctx.state.observations, observation).unwrap();
        generate_and_reserve_intents(&mut ctx).unwrap();
        assert_eq!(
            ctx.state
                .hull_jobs
                .as_slice()
                .iter()
                .filter(|job| job.owner == eid(3) && job.definition.as_str() == "colony_ark")
                .count(),
            1
        );
        assert_eq!(
            ctx.state
                .worlds
                .as_slice()
                .iter()
                .find(|world| world.id == eid(5))
                .unwrap()
                .inventory
                .alloy,
            before - authority.catalog().hull("colony_ark").unwrap().alloy_cost
        );
        assert_eq!(
            ctx.state
                .worlds
                .as_slice()
                .iter()
                .find(|world| world.id == eid(8))
                .unwrap()
                .inventory
                .energy,
            0
        );
    }

    #[test]
    fn reservations_compare_against_one_snapshot_and_reject_double_spending() {
        let snapshot = sandbox_state().unwrap();
        let world = snapshot.worlds.as_slice()[0].id;
        let mut first = snapshot.clone();
        first.worlds.find_mut(&world).unwrap().inventory.alloy -= 20;
        let mut second = snapshot.clone();
        second.worlds.find_mut(&world).unwrap().inventory.alloy -= 30;
        let mut committed = snapshot.clone();
        assert!(reserve_delta(&snapshot.worlds, &first.worlds, &mut committed.worlds).unwrap());
        let before_conflict = committed.clone();
        let mut rejected = committed.clone();
        assert!(!reserve_delta(&snapshot.worlds, &second.worlds, &mut rejected.worlds).unwrap());
        assert_eq!(committed, before_conflict);
        assert_eq!(rejected, before_conflict);
        assert_eq!(
            snapshot.worlds.as_slice()[0].inventory.alloy,
            first.worlds.as_slice()[0].inventory.alloy + 20
        );
    }
    #[test]
    fn war_eligibility_uses_observed_owner_and_skips_stale_lower_targets() {
        let authority = builtin_living_sandbox_v2().unwrap();
        let mut state = authority.state().clone();
        state.agreements = LivingSortedVecV2::default();
        state.wars = LivingSortedVecV2::default();
        put(
            &mut state.relations,
            LivingRelationV2 {
                from: eid(3),
                to: eid(4),
                base_disposition: -30,
                reasons: LivingRelationReasonsV2::default(),
            },
        )
        .unwrap();
        put(
            &mut state.fleets,
            fleet_with(
                eid(60),
                eid(3),
                eid(5),
                vec![
                    hull(eid(61), "escort", false).unwrap(),
                    hull(eid(62), "escort", false).unwrap(),
                    hull(eid(63), "escort", false).unwrap(),
                    hull(eid(64), "escort", false).unwrap(),
                ],
            )
            .unwrap(),
        )
        .unwrap();
        let mut events = super::super::receipt::LivingPendingEventsV2::new();
        let ctx = LivingStepContextV2 {
            catalog: authority.catalog(),
            genesis_seed: [0; 32],
            branch: authority.branch(),
            tick: LivingTickV2(150),
            state: &mut state,
            events: &mut events,
            arrivals: Vec::new(),
            repair_exclusions: Default::default(),
        };
        let mut observation = observe_world(&ctx, eid(3), eid(6), ctx.tick).unwrap();
        observation.owner = None;
        put(&mut ctx.state.observations, observation.clone()).unwrap();
        assert_eq!(
            war_target(&ctx, eid(3)),
            None,
            "a fresh unowned observation cannot reveal an unseen foreign settlement"
        );
        observation.owner = Some(eid(4));
        put(&mut ctx.state.observations, observation.clone()).unwrap();
        assert_eq!(war_target(&ctx, eid(3)), Some((eid(4), eid(6))));
        // The lowest actual foreign colony has a stale observation. A higher
        // target with fresh foreign ownership must still be considered.
        observation.observed_tick = LivingTickV2(1);
        put(&mut ctx.state.observations, observation).unwrap();
        let mut next = observe_world(&ctx, eid(3), eid(7), ctx.tick).unwrap();
        next.owner = Some(eid(4));
        put(&mut ctx.state.observations, next).unwrap();
        assert_eq!(
            war_target(&ctx, eid(3)),
            Some((eid(4), eid(7))),
            "historical owner evidence is authoritative for planning even before refresh"
        );
    }
    fn cargo(id: u8, kind: Option<LivingRouteKindV2>, units: u32) -> LivingShipmentV2 {
        LivingShipmentV2 {
            id: eid(id),
            dispatch_owner: eid(3),
            intended_receiver: eid(4),
            source_world: eid(5),
            destination_world: eid(6),
            resource: LivingResourceV2::Ore,
            units,
            departure_tick: LivingTickV2(0),
            arrival_tick: LivingTickV2(10),
            disposition: LivingShipmentDispositionV2::Outbound,
            dispatch_kind: kind,
            returning_to_source: false,
        }
    }

    #[test]
    fn successful_cargo_credits_immutable_dispatch_kind_once_after_route_changes() {
        let authority = builtin_living_sandbox_v2().unwrap();
        let mut state = authority.state().clone();
        state.routes = LivingSortedVecV2::default();
        let stock = world_quantity(&state, eid(6), LivingResourceV2::Ore);
        let base = relation_total(&state, eid(4), eid(3));
        for shipment in [
            cargo(60, Some(LivingRouteKindV2::Trade), 40),
            cargo(61, Some(LivingRouteKindV2::Trade), 60),
            cargo(62, Some(LivingRouteKindV2::Aid), 90),
            cargo(63, Some(LivingRouteKindV2::Aid), 10),
            cargo(64, None, 100),
        ] {
            put(&mut state.shipments, shipment).unwrap();
        }
        let mut events = super::super::receipt::LivingPendingEventsV2::new();
        let mut ctx = LivingStepContextV2 {
            catalog: authority.catalog(),
            genesis_seed: [0; 32],
            branch: authority.branch(),
            tick: LivingTickV2(10),
            state: &mut state,
            events: &mut events,
            arrivals: Vec::new(),
            repair_exclusions: Default::default(),
        };
        resolve_arrivals(&mut ctx).unwrap();
        assert!(ctx.state.shipments.as_slice().is_empty());
        assert_eq!(
            world_quantity(ctx.state, eid(6), LivingResourceV2::Ore),
            stock + 300
        );
        let reasons = relation_reasons(ctx.state, eid(4), eid(3));
        assert_eq!(reasons.successful_trade, 1);
        assert_eq!(reasons.delivered_aid, 10);
        assert_eq!(reasons.trade_units_remainder, 0);
        assert_eq!(reasons.aid_units_remainder, 0);
        assert_eq!(relation_total(ctx.state, eid(4), eid(3)), base + 11);
        assert_eq!(
            relation_reasons(ctx.state, eid(3), eid(4)),
            LivingRelationReasonsV2::default()
        );
        let committed = ctx.state.clone();
        resolve_arrivals(&mut ctx).unwrap();
        assert_eq!(
            *ctx.state, committed,
            "delivered cargo cannot be counted twice"
        );
    }

    #[test]
    fn return_wait_keeps_its_source_after_storage_and_war_change() {
        let authority = builtin_living_sandbox_v2().unwrap();
        let mut state = authority.state().clone();
        let destination_stock = world_quantity(&state, eid(6), LivingResourceV2::Ore);
        state.worlds.find_mut(&eid(5)).unwrap().inventory.ore = LIVING_RESOURCE_STORAGE_LIMIT_V2;
        let mut shipment = cargo(60, Some(LivingRouteKindV2::Trade), 100);
        shipment.disposition = LivingShipmentDispositionV2::Returning;
        put(&mut state.shipments, shipment).unwrap();
        let mut events = super::super::receipt::LivingPendingEventsV2::new();
        let mut ctx = LivingStepContextV2 {
            catalog: authority.catalog(),
            genesis_seed: [0; 32],
            branch: authority.branch(),
            tick: LivingTickV2(10),
            state: &mut state,
            events: &mut events,
            arrivals: Vec::new(),
            repair_exclusions: Default::default(),
        };
        resolve_arrivals(&mut ctx).unwrap();
        let waiting = &ctx.state.shipments.as_slice()[0];
        assert_eq!(
            waiting.disposition,
            LivingShipmentDispositionV2::WaitingForStorage
        );
        assert!(waiting.returning_to_source);
        assert_eq!(
            relation_reasons(ctx.state, eid(4), eid(3)),
            LivingRelationReasonsV2::default()
        );
        ctx.state.wars = LivingSortedVecV2::default();
        ctx.state.worlds.find_mut(&eid(5)).unwrap().inventory.ore -= 100;
        ctx.tick = LivingTickV2(11);
        resolve_arrivals(&mut ctx).unwrap();
        assert!(ctx.state.shipments.as_slice().is_empty());
        assert_eq!(
            world_quantity(ctx.state, eid(5), LivingResourceV2::Ore),
            LIVING_RESOURCE_STORAGE_LIMIT_V2
        );
        assert_eq!(
            world_quantity(ctx.state, eid(6), LivingResourceV2::Ore),
            destination_stock
        );
        assert_eq!(
            relation_reasons(ctx.state, eid(4), eid(3)),
            LivingRelationReasonsV2::default()
        );
        assert_eq!(
            ctx.events
                .payloads()
                .iter()
                .map(|event| event.kind)
                .collect::<Vec<_>>(),
            vec![
                LivingEventKindV2::ShipmentWaiting,
                LivingEventKindV2::ShipmentReturned
            ]
        );
    }

    #[test]
    fn captured_or_unknown_manifest_cargo_never_fabricates_relationship_credit() {
        let authority = builtin_living_sandbox_v2().unwrap();
        let mut state = authority.state().clone();
        let mut captured = cargo(60, Some(LivingRouteKindV2::Aid), 100);
        captured.destination_world = eid(7);
        put(&mut state.shipments, captured).unwrap();
        put(&mut state.shipments, cargo(61, None, 100)).unwrap();
        let mut events = super::super::receipt::LivingPendingEventsV2::new();
        let mut ctx = LivingStepContextV2 {
            catalog: authority.catalog(),
            genesis_seed: [0; 32],
            branch: authority.branch(),
            tick: LivingTickV2(10),
            state: &mut state,
            events: &mut events,
            arrivals: Vec::new(),
            repair_exclusions: Default::default(),
        };
        resolve_arrivals(&mut ctx).unwrap();
        assert!(ctx.state.shipments.as_slice().is_empty());
        assert_eq!(
            relation_reasons(ctx.state, eid(4), eid(3)),
            LivingRelationReasonsV2::default()
        );
        assert_eq!(
            ctx.events.payloads()[0].kind,
            LivingEventKindV2::ShipmentCaptured
        );
        assert_eq!(
            ctx.events.payloads()[1].kind,
            LivingEventKindV2::ShipmentDelivered
        );
    }

    #[test]
    fn delivery_tranches_saturate_counters_keep_partial_units_and_decay_one_point() {
        let authority = builtin_living_sandbox_v2().unwrap();
        let mut state = authority.state().clone();
        let reasons = &mut state.relations.find_mut(&(eid(4), eid(3))).unwrap().reasons;
        reasons.delivered_aid = 20;
        reasons.successful_trade = 20;
        reasons.aid_units_remainder = 45;
        reasons.trade_units_remainder = 45;
        credit_delivery_relation(&mut state, &cargo(60, Some(LivingRouteKindV2::Aid), 250))
            .unwrap();
        credit_delivery_relation(&mut state, &cargo(61, Some(LivingRouteKindV2::Trade), 250))
            .unwrap();
        let reasons = relation_reasons(&state, eid(4), eid(3));
        assert_eq!((reasons.delivered_aid, reasons.successful_trade), (20, 20));
        assert_eq!(
            (reasons.aid_units_remainder, reasons.trade_units_remainder),
            (95, 95)
        );
        let mut events = super::super::receipt::LivingPendingEventsV2::new();
        let mut ctx = LivingStepContextV2 {
            catalog: authority.catalog(),
            genesis_seed: [0; 32],
            branch: authority.branch(),
            tick: LivingTickV2(600),
            state: &mut state,
            events: &mut events,
            arrivals: Vec::new(),
            repair_exclusions: Default::default(),
        };
        decay_relations(&mut ctx).unwrap();
        let reasons = relation_reasons(ctx.state, eid(4), eid(3));
        assert_eq!((reasons.delivered_aid, reasons.successful_trade), (19, 19));
        assert_eq!(
            (reasons.aid_units_remainder, reasons.trade_units_remainder),
            (95, 95)
        );
    }
    fn economic_test_inputs(
        authority: &LivingGalaxyAuthorityV2,
        source: std::sync::Arc<LivingGalaxyStateV2>,
    ) -> LivingIntentInputsV2 {
        let tick = LivingTickV2(source.tick.0 + 1);
        let (digest, forecasts, requests) = intent_inputs_reference(authority, &source, tick);
        let mut normalized = source.as_ref().clone();
        normalized.tick = tick;
        LivingIntentInputsV2 {
            snapshot: source.into(),
            digest,
            forecasts,
            requests,
            hash_owner: std::sync::Arc::new(normalized).into(),
        }
    }

    fn assert_economic_enumeration_parity(
        authority: &LivingGalaxyAuthorityV2,
        state: &LivingGalaxyStateV2,
        adjust: impl Fn(&mut LivingIntentInputsV2),
    ) -> Vec<LivingEconomicBatchV2> {
        use super::super::step_job::{
            LivingLifecycleJobErrorV2 as Error, LivingLifecycleProgressV2 as Progress,
        };
        let source = std::sync::Arc::new(state.clone());
        let mut visits = None;
        let mut result = Vec::new();
        for budgets in [&[1][..], &[64][..], &[1024][..], &[1, 64, 7, 1024, 3][..]] {
            let mut inputs = economic_test_inputs(authority, std::sync::Arc::clone(&source));
            adjust(&mut inputs);
            let tick = LivingTickV2(state.tick.0 + 1);
            let mut reference_state = state.clone();
            let mut events = super::super::receipt::LivingPendingEventsV2::new();
            let ctx = LivingStepContextV2 {
                catalog: authority.catalog(),
                genesis_seed: [0; 32],
                branch: authority.branch(),
                tick,
                state: &mut reference_state,
                events: &mut events,
                arrivals: Vec::new(),
                repair_exclusions: Default::default(),
            };
            let expected = inputs
                .forecasts
                .iter()
                .map(|(actor, forecast)| {
                    let policy = state
                        .civilizations
                        .as_slice()
                        .iter()
                        .find(|row| row.id == *actor)
                        .unwrap()
                        .policy
                        .0
                        .as_str();
                    let ranked =
                        reserve_economy_proposals(&ctx, *actor, policy, forecast, &inputs.requests)
                            .unwrap();
                    let explanations = ranked
                        .iter()
                        .map(|(ordinal, candidate)| {
                            economic_decision(
                                state,
                                *actor,
                                *ordinal,
                                candidate,
                                tick,
                                inputs.digest,
                            )
                            .unwrap()
                        })
                        .collect::<Vec<_>>();
                    (*actor, ranked, explanations)
                })
                .collect::<Vec<_>>();
            let hash_owner = std::sync::Arc::downgrade(&inputs.hash_owner);
            let forecast_owners = inputs
                .forecasts
                .iter()
                .map(|(_, f)| f.final_state.worlds.as_slice().as_ptr())
                .collect::<Vec<_>>();
            let mut job =
                LivingEconomicEnumerationJobV2::begin(authority.catalog_snapshot(), inputs, tick);
            assert!(matches!(job.finish(), Err(Error::NotReady)));
            assert_eq!(job.poll(0), Err(Error::Budget));
            assert_eq!(job.poll(1025), Err(Error::Budget));
            let mut polls = 0;
            loop {
                let before = job.total_visits;
                let budget = budgets[polls % budgets.len()];
                let progress = job.poll(budget).unwrap();
                assert!(job.last_poll_visits <= budget);
                assert_eq!(job.total_visits - before, job.last_poll_visits as u64);
                polls += 1;
                if progress == Progress::Ready {
                    break;
                }
                assert!(job.inputs.is_some());
                assert!(hash_owner.strong_count() > 0);
            }
            if let Some(visits) = visits {
                assert_eq!(job.total_visits, visits);
            }
            visits = Some(job.total_visits);
            assert_eq!(job.poll(1), Ok(Progress::Ready));
            assert_eq!(job.last_poll_visits, 0);
            assert!(
                job.canonical.is_empty() && job.ranking.is_empty() && job.facilities.is_empty()
            );
            assert!(job.worlds.is_empty() && job.projected.is_empty() && job.idle.is_empty());
            assert!(job.graph.systems.is_empty() && job.graph.lanes.is_empty());
            let output = job.finish().unwrap();
            assert!(matches!(job.finish(), Err(Error::NotReady)));
            assert!(std::sync::Arc::ptr_eq(&output.inputs.snapshot, &source));
            assert!(std::sync::Arc::ptr_eq(&output.catalog, &job.catalog));
            assert!(std::sync::Arc::ptr_eq(&output.graph, &job.graph));
            assert_eq!(
                output
                    .inputs
                    .forecasts
                    .iter()
                    .map(|(_, f)| f.final_state.worlds.as_slice().as_ptr())
                    .collect::<Vec<_>>(),
                forecast_owners
            );
            assert_eq!(output.batches.len(), expected.len());
            for (batch, (actor, ranked, explanations)) in output.batches.iter().zip(expected) {
                assert_eq!(batch.actor, actor);
                assert_eq!(batch.economic_count, ranked.len());
                assert!(
                    batch.ranked == ranked,
                    "full original candidate parity for actor {actor:?}"
                );
                assert_eq!(batch.explanations, explanations);
                for (actual, expected) in batch.explanations.iter().zip(&explanations) {
                    assert_eq!(
                        actual.action.canonical_bytes().unwrap(),
                        expected.action.canonical_bytes().unwrap()
                    );
                }
            }
            assert_eq!(source.as_ref(), state);
            assert_eq!(reference_state, *state);
            assert!(events.decisions.is_empty());
            result = output.batches;
        }
        result
    }

    #[test]
    fn economic_enumeration_matches_reference_baseline_policy_and_projected_repair() {
        let authority = builtin_living_sandbox_v2().unwrap();
        for policy in [
            "neutral",
            "industrialist",
            "guardian",
            "expansionist",
            "trader",
        ] {
            let mut state = authority.state().clone();
            state.tick = LivingTickV2(49);
            for actor in state.civilizations.as_slice().to_vec() {
                state.civilizations.find_mut(&actor.id).unwrap().policy =
                    LivingPolicyV2(slug(policy).unwrap());
            }
            for world in state.worlds.as_slice().to_vec() {
                state.worlds.find_mut(&world.id).unwrap().inventory.alloy = 200;
            }
            let fleet = state
                .fleets
                .as_slice()
                .iter()
                .find(|row| matches!(row.location, LivingFleetLocationV2::Docked { .. }))
                .unwrap()
                .id;
            let hull = state.fleets.find_mut(&fleet).unwrap().hulls.as_slice()[0].id;
            state
                .fleets
                .find_mut(&fleet)
                .unwrap()
                .hulls
                .find_mut(&hull)
                .unwrap()
                .hit_points = 1;
            state
                .fleets
                .find_mut(&fleet)
                .unwrap()
                .hulls
                .find_mut(&hull)
                .unwrap()
                .next_repair = Some(LivingTickV2(50));
            let LivingFleetLocationV2::Docked {
                world: repair_world,
            } = state.fleets.find_mut(&fleet).unwrap().location
            else {
                unreachable!()
            };
            state
                .worlds
                .find_mut(&repair_world)
                .unwrap()
                .inventory
                .energy = 0;
            for facility in state.facilities.as_slice().to_vec() {
                if facility.world == repair_world {
                    state.facilities.find_mut(&facility.id).unwrap().disabled = true;
                }
            }
            state
                .colonies
                .find_mut(&repair_world)
                .unwrap()
                .hub
                .energy_next_due = LivingTickV2(300);
            state
                .colonies
                .find_mut(&repair_world)
                .unwrap()
                .hub
                .fallback_next_due = LivingTickV2(300);
            assert_economic_enumeration_parity(&authority, &state, |_| {});
        }
    }

    #[test]
    fn economic_enumeration_replacement_repair_exclusion_and_repeated_shipyard() {
        let authority = builtin_first_expansion_v2().unwrap();
        for repair in [false, true] {
            let mut state = authority.state().clone();
            state.tick = LivingTickV2(49);
            state.facilities = LivingSortedVecV2::default();
            for index in 0..6 {
                let mut row = facility(
                    eid(60 + index),
                    eid(4),
                    "solar_array",
                    Some(LivingTickV2(60)),
                )
                .unwrap();
                row.disabled = index < 2;
                if repair && index == 0 {
                    row.next_repair = Some(LivingTickV2(60));
                }
                put(&mut state.facilities, row).unwrap();
            }
            state.fleets = LivingSortedVecV2::default();
            state.worlds.find_mut(&eid(4)).unwrap().inventory.alloy = 200;
            put(
                &mut state.observations,
                LivingObservationV2 {
                    civilization: eid(3),
                    world: eid(5),
                    observed_tick: LivingTickV2(1),
                    owner: None,
                    remaining_deposit: 0,
                    facilities: Default::default(),
                    stationed_hulls: Default::default(),
                    operational_defense_batteries: None,
                },
            )
            .unwrap();
            let batches = assert_economic_enumeration_parity(&authority, &state, |_| {});
            let shipyards = batches[0]
                .ranked
                .iter()
                .filter(|(_, c)| {
                    matches!(
                        c.action,
                        LivingEconomicActionV2::Construction {
                            definition: "shipyard",
                            ..
                        }
                    )
                })
                .collect::<Vec<_>>();
            assert_eq!(shipyards.len(), 1);
            assert!(
                matches!(shipyards[0].1.action, LivingEconomicActionV2::Construction { scrap: Some(id), .. } if id == eid(if repair {61} else {60}))
            );
        }
    }

    #[test]
    fn economic_enumeration_internal_tied_donors_stock_thresholds_and_any_working_route() {
        let authority = builtin_living_sandbox_v2().unwrap();
        for stock in [19, 20, 29, 30] {
            for working in [false, true] {
                let mut state = route_adjustment_state();
                state.worlds.find_mut(&eid(5)).unwrap().inventory.energy = stock;
                let mut donor = state
                    .worlds
                    .as_slice()
                    .iter()
                    .find(|row| row.id == eid(5))
                    .unwrap()
                    .clone();
                donor.id = eid(100);
                put(&mut state.worlds, donor).unwrap();
                let mut route = state
                    .routes
                    .as_slice()
                    .iter()
                    .find(|row| row.id == eid(90))
                    .unwrap()
                    .clone();
                route.id = eid(91);
                route.suspended = !working;
                route.source_owner = eid(3);
                route.receiver_owner = eid(3);
                route.batch_size = 10;
                route.source_reserve = 20;
                put(&mut state.routes, route).unwrap();
                let batches = assert_economic_enumeration_parity(&authority, &state, |inputs| {
                    let (_, forecast) = inputs
                        .forecasts
                        .iter_mut()
                        .find(|(actor, _)| *actor == eid(3))
                        .unwrap();
                    if !forecast
                        .deficits
                        .contains(&(eid(8), LivingResourceV2::Energy))
                    {
                        forecast.deficits.push((eid(8), LivingResourceV2::Energy));
                    }
                });
                let supplies = batches.iter().find(|batch| batch.actor == eid(3)).unwrap().ranked.iter().filter(|(_, c)| matches!(c.action, LivingEconomicActionV2::Supply { destination, resource: LivingResourceV2::Energy, .. } if destination == eid(8))).count();
                assert_eq!(
                    supplies,
                    if stock < 30 {
                        0
                    } else if working {
                        1
                    } else {
                        2
                    }
                );
            }
        }
    }

    #[test]
    fn economic_enumeration_foreign_trade_aid_half_open_and_captured_worlds() {
        let authority = builtin_living_sandbox_v2().unwrap();
        for (trade_start, trade_end, war_end) in [
            (1, 51, 50),
            (50, 51, 50),
            (1, 50, 50),
            (1, 50, 51),
            (51, 100, 50),
        ] {
            for captured in [false, true] {
                let mut state = authority.state().clone();
                state.tick = LivingTickV2(49);
                state.civilizations.find_mut(&eid(3)).unwrap().policy =
                    LivingPolicyV2(slug("trader").unwrap());
                state.worlds.find_mut(&eid(5)).unwrap().inventory.ore = 30;
                state.worlds.find_mut(&eid(6)).unwrap().inventory.ore = 0;
                if captured {
                    state.worlds.find_mut(&eid(6)).unwrap().owner = Some(eid(3));
                }
                state.agreements = Default::default();
                state.wars = Default::default();
                put(
                    &mut state.agreements,
                    LivingAgreementV2 {
                        kind: LivingAgreementKindV2::Trade,
                        participant_a: eid(3),
                        participant_b: eid(4),
                        start_tick: LivingTickV2(trade_start),
                        end_tick: LivingTickV2(trade_end),
                    },
                )
                .unwrap();
                put(
                    &mut state.wars,
                    LivingWarV2 {
                        participant_a: eid(3),
                        participant_b: eid(4),
                        declarer: eid(3),
                        start_tick: LivingTickV2(1),
                        end_tick: LivingTickV2(war_end),
                    },
                )
                .unwrap();
                assert_economic_enumeration_parity(&authority, &state, |inputs| {
                    inputs.requests.push(LivingReceiverDemandRequestV2 {
                        receiver: eid(4),
                        world: eid(6),
                        resource: LivingResourceV2::Ore,
                        requested_units: 10,
                    });
                    for (_, forecast) in &mut inputs.forecasts {
                        forecast.deficits.push((eid(6), LivingResourceV2::Ore));
                    }
                });
            }
        }
    }

    #[test]
    fn economic_enumeration_projected_existentials_hidden_stale_and_unowned_presence() {
        let authority = builtin_first_expansion_v2().unwrap();
        let mut state = authority.state().clone();
        state.tick = LivingTickV2(149);
        state.worlds.find_mut(&eid(4)).unwrap().inventory.alloy = 200;
        state.facilities = Default::default();
        let mut extractor =
            facility(eid(60), eid(4), "extractor", Some(LivingTickV2(160))).unwrap();
        extractor.disabled = true;
        put(&mut state.facilities, extractor).unwrap();
        put(
            &mut state.observations,
            LivingObservationV2 {
                civilization: eid(3),
                world: eid(5),
                observed_tick: LivingTickV2(1),
                owner: None,
                remaining_deposit: 0,
                facilities: Default::default(),
                stationed_hulls: Default::default(),
                operational_defense_batteries: None,
            },
        )
        .unwrap();
        let batches = assert_economic_enumeration_parity(&authority, &state, |inputs| {
            let forecast = &mut inputs.forecasts[0].1;
            forecast.inventories = vec![
                (
                    LivingTickV2(150),
                    eid(4),
                    LivingInventoryV2 {
                        energy: 4,
                        ore: 4,
                        alloy: LIVING_RESOURCE_STORAGE_LIMIT_V2,
                    },
                ),
                (
                    LivingTickV2(160),
                    eid(4),
                    LivingInventoryV2 {
                        energy: 0,
                        ore: 0,
                        alloy: 0,
                    },
                ),
            ];
        });
        assert!(batches[0].ranked.iter().any(|(_, c)| matches!(
            c.action,
            LivingEconomicActionV2::Construction {
                definition: "foundry",
                ..
            }
        )));
        assert!(!batches[0].ranked.iter().any(|(_, c)| matches!(
            c.action,
            LivingEconomicActionV2::Construction {
                definition: "extractor",
                ..
            }
        )));
    }

    #[test]
    fn economic_enumeration_long_disconnected_and_local_paths_share_budget() {
        let authority = builtin_living_sandbox_v2().unwrap();
        for connected in [false, true] {
            let mut state = route_adjustment_state();
            let chain = (0..28).map(|i| (i, i + 1, 81)).collect::<Vec<_>>();
            let graph = exact_path_state(29, &chain);
            state.systems = graph.systems;
            state.lanes = graph.lanes;
            if !connected {
                remove_key(&mut state.lanes, &eid(114)).unwrap();
            }
            for world in state.worlds.as_slice().to_vec() {
                state.worlds.find_mut(&world.id).unwrap().system = eid(32);
            }
            state.worlds.find_mut(&eid(8)).unwrap().system = eid(60);
            assert_economic_enumeration_parity(&authority, &state, |inputs| {
                inputs
                    .forecasts
                    .iter_mut()
                    .find(|(actor, _)| *actor == eid(3))
                    .unwrap()
                    .1
                    .deficits
                    .push((eid(8), LivingResourceV2::Energy));
            });
            let mut job = LivingEconomicEnumerationJobV2::begin(
                authority.catalog_snapshot(),
                economic_test_inputs(&authority, std::sync::Arc::new(state)),
                LivingTickV2(50),
            );
            let mut path_visits = 0;
            while job.stage != EconomicEnumerationStage::Ready {
                let was_path = job.stage == EconomicEnumerationStage::Path;
                job.poll(1).unwrap();
                if was_path {
                    path_visits += job.last_poll_visits;
                }
            }
            assert!(
                path_visits > 1024,
                "actual nested path work must span multiple maximum polls"
            );
        }
    }

    #[test]
    fn economic_enumeration_foreign_batch_thresholds_and_construction_jobs() {
        let authority = builtin_living_sandbox_v2().unwrap();
        for (stock, requested) in [(20, 1), (21, 1), (29, 10), (30, 10), (30, 20)] {
            let mut state = authority.state().clone();
            state.tick = LivingTickV2(49);
            state.worlds.find_mut(&eid(5)).unwrap().inventory.ore = stock;
            state.civilizations.find_mut(&eid(3)).unwrap().policy =
                LivingPolicyV2(slug("trader").unwrap());
            put(
                &mut state.construction_jobs,
                LivingConstructionJobV2 {
                    id: eid(90),
                    world: eid(5),
                    owner: eid(4),
                    definition: slug("foundry").unwrap(),
                    accepted_tick: LivingTickV2(1),
                    completion_tick: LivingTickV2(100),
                    paid_alloy: 0,
                },
            )
            .unwrap();
            assert_economic_enumeration_parity(&authority, &state, |inputs| {
                inputs.requests = vec![LivingReceiverDemandRequestV2 {
                    receiver: eid(4),
                    world: eid(6),
                    resource: LivingResourceV2::Ore,
                    requested_units: requested,
                }];
            });
        }
    }

    #[test]
    fn economic_enumeration_missing_catalog_cost_faults_only_on_reference_branch() {
        use super::super::step_job::{
            LivingLifecycleJobErrorV2 as Error, LivingLifecycleProgressV2 as Progress,
        };
        let authority = builtin_living_sandbox_v2().unwrap();
        // Remove one definition while preserving the typed catalog's wire field order.
        let mut bytes = authority.catalog().canonical_bytes().to_vec();
        let token = b"\"id\":\"defense_battery\"";
        let id = bytes
            .windows(token.len())
            .position(|window| window == token)
            .unwrap();
        let mut begin = bytes[..id].iter().rposition(|byte| *byte == b'{').unwrap();
        let mut depth = 0;
        let mut end = begin;
        for (index, byte) in bytes.iter().enumerate().skip(begin) {
            if *byte == b'{' {
                depth += 1;
            }
            if *byte == b'}' {
                depth -= 1;
                if depth == 0 {
                    end = index + 1;
                    break;
                }
            }
        }
        if bytes[end] == b',' {
            end += 1;
        } else {
            begin -= 1;
            assert_eq!(bytes[begin], b',');
        }
        bytes.drain(begin..end);
        let catalog = std::sync::Arc::new(
            super::super::catalog::decode_living_catalog_pack_v2(&bytes).unwrap(),
        );
        for blocked in [false, true] {
            let mut state = authority.state().clone();
            if blocked {
                for (index, colony) in state.colonies.as_slice().to_vec().into_iter().enumerate() {
                    put(
                        &mut state.construction_jobs,
                        LivingConstructionJobV2 {
                            id: eid(80 + index as u8),
                            world: colony.world,
                            owner: colony.owner,
                            definition: slug("shipyard").unwrap(),
                            accepted_tick: LivingTickV2(0),
                            completion_tick: LivingTickV2(100),
                            paid_alloy: 0,
                        },
                    )
                    .unwrap();
                }
            }
            let inputs = economic_test_inputs(&authority, std::sync::Arc::new(state.clone()));
            let mut ref_state = state.clone();
            let mut events = super::super::receipt::LivingPendingEventsV2::new();
            let ctx = LivingStepContextV2 {
                catalog: &catalog,
                genesis_seed: [0; 32],
                branch: authority.branch(),
                tick: LivingTickV2(1),
                state: &mut ref_state,
                events: &mut events,
                arrivals: Vec::new(),
                repair_exclusions: Default::default(),
            };
            let expected = inputs
                .forecasts
                .iter()
                .map(|(actor, forecast)| {
                    reserve_economy_proposals(&ctx, *actor, "neutral", forecast, &inputs.requests)
                })
                .collect::<Result<Vec<_>, _>>();
            assert_eq!(expected.is_ok(), blocked);
            let mut job = LivingEconomicEnumerationJobV2::begin(
                std::sync::Arc::clone(&catalog),
                inputs,
                LivingTickV2(1),
            );
            let result = loop {
                match job.poll(1) {
                    Ok(Progress::Pending) => {}
                    result => break result,
                }
            };
            if blocked {
                assert_eq!(result, Ok(Progress::Ready));
                job.finish().unwrap();
            } else {
                assert_eq!(
                    result,
                    Err(Error::Fault(LivingDeterministicFaultV2::Invariant))
                );
                assert_eq!(job.poll(1024), result);
                assert!(matches!(job.finish(), Err(Error::Fault(_))));
                assert!(job.inputs.is_some());
            }
        }
    }

    #[test]
    fn economic_enumeration_threats_all_nested_hulls_and_paid_jobs_match_reference() {
        let authority = builtin_living_sandbox_v2().unwrap();
        for end in [50, 51] {
            let mut state = authority.state().clone();
            state.tick = LivingTickV2(49);
            state.civilizations.find_mut(&eid(3)).unwrap().policy =
                LivingPolicyV2(slug("guardian").unwrap());
            state.worlds.find_mut(&eid(5)).unwrap().inventory.alloy = 200;
            let foreign = state
                .fleets
                .as_slice()
                .iter()
                .find(|fleet| fleet.owner == eid(4))
                .unwrap()
                .id;
            state.fleets.find_mut(&foreign).unwrap().location =
                LivingFleetLocationV2::Docked { world: eid(5) };
            put(
                &mut state.fleets.find_mut(&foreign).unwrap().hulls,
                hull(eid(90), "escort", false).unwrap(),
            )
            .unwrap();
            put(
                &mut state.wars,
                LivingWarV2 {
                    participant_a: eid(3),
                    participant_b: eid(4),
                    declarer: eid(4),
                    start_tick: LivingTickV2(1),
                    end_tick: LivingTickV2(end),
                },
            )
            .unwrap();
            for index in 0..4 {
                put(
                    &mut state.hull_jobs,
                    LivingHullJobV2 {
                        id: eid(100 + index),
                        world: eid(8),
                        owner: eid(3),
                        definition: slug("escort").unwrap(),
                        accepted_tick: LivingTickV2(1),
                        completion_tick: LivingTickV2(300),
                        paid_alloy: 0,
                        target_fleet: None,
                    },
                )
                .unwrap();
            }
            let batches = assert_economic_enumeration_parity(&authority, &state, |_| {});
            assert_eq!(batches.iter().find(|batch| batch.actor == eid(3)).unwrap().ranked.iter().filter(|(_,candidate)| matches!(candidate.action,LivingEconomicActionV2::Construction { world,definition:"defense_battery",.. } if world == eid(5))).count(),usize::from(end == 51));
        }
    }

    fn economic_ordering_job() -> LivingEconomicEnumerationJobV2 {
        let authority = builtin_living_sandbox_v2().unwrap();
        let mut job = LivingEconomicEnumerationJobV2::begin(
            authority.catalog_snapshot(),
            economic_test_inputs(&authority, std::sync::Arc::new(authority.state().clone())),
            LivingTickV2(1),
        );
        while job.stage != EconomicEnumerationStage::ColonyCount {
            job.poll(1).unwrap();
        }
        job.stage = EconomicEnumerationStage::Canonical;
        job
    }
    fn economic_supply_candidate(
        kind: LivingRouteKindV2,
        receiver: LivingEntityIdV2,
        batch_size: u32,
    ) -> LivingEconomicCandidateV2 {
        LivingEconomicCandidateV2 {
            tier: 3,
            score: 70,
            target: eid(6),
            action: LivingEconomicActionV2::Supply {
                source: eid(5),
                destination: eid(6),
                resource: LivingResourceV2::Energy,
                kind,
                receiver,
                batch_size,
                existing_route: None,
            },
        }
    }
    #[test]
    fn economic_enumeration_adjacent_action_dedup_and_original_stable_ordinals() {
        let mut job = economic_ordering_job();
        for (kind, receiver, batch) in [
            (LivingRouteKindV2::Internal, eid(3), 10),
            (LivingRouteKindV2::Trade, eid(4), 10),
            (LivingRouteKindV2::Internal, eid(3), 10),
            (LivingRouteKindV2::Internal, eid(3), 10),
            (LivingRouteKindV2::Internal, eid(3), 9),
        ] {
            job.emit(economic_supply_candidate(kind, receiver, batch))
                .unwrap();
        }
        while job.batches.is_empty() {
            job.poll(1).unwrap();
        }
        let batch = &job.batches[0];
        assert_eq!(batch.economic_count, 4);
        assert_eq!(
            batch
                .ranked
                .iter()
                .map(|(ordinal, _)| *ordinal)
                .collect::<Vec<_>>(),
            [0, 1, 2, 3]
        );
        let bytes = batch.explanations[0].action.canonical_bytes().unwrap();
        for row in &batch.explanations {
            assert_eq!(row.action.canonical_bytes().unwrap(), bytes);
        }
    }

    #[test]
    fn economic_enumeration_ordinal_65536_valid_overflow_and_emission_fault_retain_owners() {
        use super::super::step_job::LivingLifecycleJobErrorV2 as Error;
        let mut job = economic_ordering_job();
        job.batch.as_mut().unwrap().economic_count = 65535;
        job.emit(economic_supply_candidate(
            LivingRouteKindV2::Internal,
            eid(3),
            10,
        ))
        .unwrap();
        job.poll(1).unwrap();
        assert_eq!(job.batch.as_ref().unwrap().economic_count, 65536);
        assert_eq!(job.ranking.first_key_value().unwrap().0.5, 65535);
        job.emit(economic_supply_candidate(
            LivingRouteKindV2::Trade,
            eid(4),
            10,
        ))
        .unwrap();
        let before = job.inputs.as_ref().unwrap().snapshot.clone();
        assert_eq!(
            job.poll(1),
            Err(Error::Fault(LivingDeterministicFaultV2::Arithmetic))
        );
        assert_eq!(
            job.poll(1),
            Err(Error::Fault(LivingDeterministicFaultV2::Arithmetic))
        );
        assert!(matches!(
            job.finish(),
            Err(Error::Fault(LivingDeterministicFaultV2::Arithmetic))
        ));
        assert_eq!(job.canonical.len(), 1);
        assert_eq!(job.ranking.len(), 1);
        assert!(std::sync::Arc::ptr_eq(
            &job.inputs.as_ref().unwrap().snapshot,
            &before
        ));
        let mut emission = economic_ordering_job();
        emission.emission = usize::MAX;
        assert!(matches!(
            emission.emit(economic_supply_candidate(
                LivingRouteKindV2::Internal,
                eid(3),
                10
            )),
            Err(LivingDeterministicFaultV2::Arithmetic)
        ));
        assert!(emission.failed_candidate.is_some());
        assert!(emission.inputs.is_some());
    }

    #[test]
    fn economic_enumeration_full_65536_ordinals_are_valid() {
        use super::super::step_job::LivingLifecycleJobErrorV2 as Error;
        let mut job = economic_ordering_job();
        for batch in 0..=u32::from(u16::MAX) {
            job.emit(economic_supply_candidate(
                LivingRouteKindV2::Internal,
                eid(3),
                batch,
            ))
            .unwrap();
        }
        while job.stage == EconomicEnumerationStage::Canonical {
            job.poll(64).unwrap();
        }
        assert_eq!(job.batch.as_ref().unwrap().economic_count, 65536);
        assert_eq!(
            job.ranking.len() + job.batch.as_ref().unwrap().ranked.len(),
            65536
        );
        while job.batches.is_empty() {
            job.poll(1024).unwrap();
        }
        assert_eq!(job.batches[0].ranked.len(), 65536);
        assert!(
            job.batches[0]
                .ranked
                .iter()
                .enumerate()
                .all(|(i, (ordinal, _))| usize::from(*ordinal) == i)
        );
        let mut overflow = economic_ordering_job();
        for batch in 0..=65536 {
            overflow
                .emit(economic_supply_candidate(
                    LivingRouteKindV2::Internal,
                    eid(3),
                    batch,
                ))
                .unwrap();
        }
        let failure = loop {
            match overflow.poll(1024) {
                Ok(_) => {}
                Err(error) => break error,
            }
        };
        assert_eq!(
            failure,
            Error::Fault(LivingDeterministicFaultV2::Arithmetic)
        );
        assert_eq!(overflow.batch.as_ref().unwrap().economic_count, 65536);
        assert_eq!(overflow.canonical.len(), 1);
        assert_eq!(overflow.ranking.len(), 65536);
    }

    #[test]
    fn economic_enumeration_cancel_each_stage_retains_all_owners_without_visiting() {
        use super::super::step_job::LivingLifecycleJobErrorV2 as Error;
        use EconomicEnumerationStage::*;
        for stage in [
            Worlds,
            Systems,
            Lanes,
            Industries,
            HullCosts,
            Wars,
            Agreements,
            Observations,
            ForecastStocks,
            ForecastDeficits,
            ForecastIdle,
            Civilizations,
            FleetCounts,
            PaidHullCounts,
            Actors,
            ColonyCount,
            Colonies,
            Facilities,
            ConstructionJobs,
            ColonyHullJobs,
            Threats,
            Replacement,
            Buildings,
            Hulls,
            ArkTargets,
            Path,
            RetirePath,
            Resources,
            Sources,
            Routes,
            Requests,
            AdjacentColonies,
            AdjacentLanes,
            ForeignSources,
            Canonical,
            Ranking,
            RetireFacilities,
            RetireWorlds,
            RetireIndustries,
            RetireHulls,
            RetireWars,
            RetireTrade,
            RetireObservations,
            RetireStocks,
            RetireDeficits,
            RetireIdle,
            RetireActors,
            RetireCounts,
            RetireEscorts,
            RetireGraphLanes,
            RetireGraphSystems,
            Ready,
        ] {
            let mut job = economic_ordering_job();
            job.stage = stage;
            let snapshot = std::sync::Arc::downgrade(&job.inputs.as_ref().unwrap().snapshot);
            let normalized = std::sync::Arc::downgrade(&job.inputs.as_ref().unwrap().hash_owner);
            let sizes = (
                job.worlds.len(),
                job.projected.len(),
                job.inputs.as_ref().unwrap().forecasts.len(),
                job.graph.lanes.len(),
            );
            let counts = (snapshot.strong_count(), normalized.strong_count());
            job.cancel();
            assert_eq!(job.poll(1), Err(Error::Cancelled));
            assert!(matches!(job.finish(), Err(Error::Cancelled)));
            assert_eq!(job.last_poll_visits, 0);
            assert_eq!(counts, (snapshot.strong_count(), normalized.strong_count()));
            assert_eq!(
                sizes,
                (
                    job.worlds.len(),
                    job.projected.len(),
                    job.inputs.as_ref().unwrap().forecasts.len(),
                    job.graph.lanes.len()
                )
            );
        }
    }

    #[test]
    fn economic_enumeration_cancel_all_reached_stages_and_invalid_snapshot_retain_owners() {
        use super::super::step_job::LivingLifecycleJobErrorV2 as Error;
        let authority = builtin_living_sandbox_v2().unwrap();
        let mut source = route_adjustment_state();
        source.civilizations.find_mut(&eid(3)).unwrap().policy =
            LivingPolicyV2(slug("trader").unwrap());
        let source = std::sync::Arc::new(source);
        let make = || {
            let mut inputs = economic_test_inputs(&authority, std::sync::Arc::clone(&source));
            inputs.requests.push(LivingReceiverDemandRequestV2 {
                receiver: eid(4),
                world: eid(6),
                resource: LivingResourceV2::Ore,
                requested_units: 10,
            });
            LivingEconomicEnumerationJobV2::begin(
                authority.catalog_snapshot(),
                inputs,
                LivingTickV2(50),
            )
        };
        let mut probe = make();
        let mut stages = std::collections::BTreeSet::new();
        while probe.stage != EconomicEnumerationStage::Ready {
            stages.insert(probe.stage as u8);
            probe.poll(1).unwrap();
        }
        stages.insert(EconomicEnumerationStage::Ready as u8);
        for target in stages {
            let mut job = make();
            while job.stage as u8 != target {
                job.poll(1).unwrap();
            }
            let hash = std::sync::Arc::downgrade(&job.inputs.as_ref().unwrap().hash_owner);
            let lengths = (
                job.canonical.len(),
                job.ranking.len(),
                job.batches.len(),
                job.retired_path.len(),
                job.facilities.len(),
            );
            job.cancel();
            assert_eq!(job.poll(1), Err(Error::Cancelled));
            assert_eq!(job.poll(1), Err(Error::Cancelled));
            assert!(matches!(job.finish(), Err(Error::Cancelled)));
            assert_eq!(
                lengths,
                (
                    job.canonical.len(),
                    job.ranking.len(),
                    job.batches.len(),
                    job.retired_path.len(),
                    job.facilities.len()
                )
            );
            assert_eq!(hash.strong_count(), 1);
            assert!(std::sync::Arc::ptr_eq(
                &job.inputs.as_ref().unwrap().snapshot,
                &source
            ));
        }
        let mut bad = LivingEconomicEnumerationJobV2::begin(
            authority.catalog_snapshot(),
            economic_test_inputs(&authority, std::sync::Arc::clone(&source)),
            LivingTickV2(49),
        );
        assert!(matches!(bad.poll(1), Err(Error::Fault(_))));
        assert!(matches!(bad.poll(1024), Err(Error::Fault(_))));
        assert!(matches!(bad.finish(), Err(Error::Fault(_))));
        assert!(bad.inputs.is_some());
    }

    fn due_intent_reference(
        authority: &LivingGalaxyAuthorityV2,
        source: &LivingGalaxyStateV2,
        events: super::super::receipt::LivingPendingEventsV2,
    ) -> (
        LivingGalaxyStateV2,
        super::super::receipt::LivingPendingEventsV2,
    ) {
        let mut state = source.clone();
        let mut events = events;
        let mut ctx = LivingStepContextV2 {
            catalog: authority.catalog(),
            genesis_seed: [0; 32],
            branch: authority.branch(),
            tick: LivingTickV2(source.tick.0 + 1),
            state: &mut state,
            events: &mut events,
            arrivals: Vec::new(),
            repair_exclusions: Default::default(),
        };
        generate_and_reserve_intents_with_disclosure(&mut ctx, false).unwrap();
        (state, events)
    }
    fn due_intent_cross_system_state() -> LivingGalaxyStateV2 {
        let mut state = route_adjustment_state();
        let graph = exact_path_state(3, &[(0, 1, 1), (0, 2, 1)]);
        state.systems = graph.systems;
        state.lanes = graph.lanes;
        for row in state.stars.as_slice().to_vec() {
            state.stars.find_mut(&row.id).unwrap().system = eid(32);
        }
        for row in state.worlds.as_slice().to_vec() {
            state.worlds.find_mut(&row.id).unwrap().system = eid(32);
        }
        state.worlds.find_mut(&eid(8)).unwrap().system = eid(34);
        state
    }
    fn due_intent_job(
        authority: &LivingGalaxyAuthorityV2,
        state: &LivingGalaxyStateV2,
    ) -> LivingDueIntentsJobV2 {
        LivingDueIntentsJobV2::begin(
            authority.catalog_snapshot(),
            state.clone(),
            Default::default(),
            LivingTickV2(state.tick.0 + 1),
            [0; 32],
            authority.branch(),
        )
    }
    #[test]
    fn due_intent_parent_independent_orchestration_state_decisions_and_funding() {
        use super::super::step_job::{
            LivingLifecycleJobErrorV2 as Error, LivingLifecycleProgressV2 as Progress,
        };
        let authority = builtin_living_sandbox_v2().unwrap();
        let (_, fleet) = fleet_enumeration_fixture();
        let mut winning_actors = std::collections::BTreeSet::new();
        let mut fleet_winner = false;
        for state in [
            authority.state().clone(),
            route_adjustment_state(),
            fleet,
            due_intent_cross_system_state(),
        ] {
            let (expected, events) = due_intent_reference(&authority, &state, Default::default());
            let mut total = None;
            for budgets in [&[1][..], &[64][..], &[1024][..], RULES_MIXED] {
                let mut job = due_intent_job(&authority, &state);
                let source = job.source.clone();
                let source_ptr = source.worlds.as_slice().as_ptr();
                assert!(matches!(job.take_ready_result(), Err(Error::NotReady)));
                for invalid in [0, 1025] {
                    assert_eq!(job.poll(invalid), Err(Error::Budget));
                    assert_eq!(job.last_poll_visits(), 0);
                }
                let mut polls = 0;
                loop {
                    let budget = budgets[polls % budgets.len()];
                    polls += 1;
                    let before = job.total_visits;
                    let progress = job.poll(budget).unwrap();
                    assert!(job.last_poll_visits <= budget);
                    assert_eq!(job.total_visits - before, job.last_poll_visits as u64);
                    assert_eq!(source.worlds.as_slice().as_ptr(), source_ptr);
                    assert_eq!(*source, state);
                    if progress == Progress::Ready {
                        break;
                    }
                    assert!(polls < 2_000_000);
                }
                if let Some(total) = total {
                    assert_eq!(job.total_visits, total);
                } else {
                    total = Some(job.total_visits);
                }
                assert_eq!(job.poll(1), Ok(Progress::Ready));
                assert_eq!(job.last_poll_visits, 0);
                let (actual, actual_events) = job.take_ready_result().unwrap();
                assert_eq!(actual, expected);
                assert_eq!(actual_events.decisions, events.decisions);
                assert_eq!(actual_events.payloads(), events.payloads());
                assert_eq!(actual_events.explained, events.explained);
                assert!(matches!(job.take_ready_result(), Err(Error::NotReady)));
                for actor in state.civilizations.as_slice() {
                    let accepted: Vec<_> = actual_events
                        .decisions
                        .iter()
                        .filter(|d| {
                            d.actor == actor.id && d.outcome == LivingDecisionOutcomeV2::Accepted
                        })
                        .collect();
                    assert!(accepted.len() <= 2);
                    for decision in accepted {
                        winning_actors.insert(decision.actor);
                        fleet_winner |= matches!(
                            decision.action,
                            LivingIntentActionV2::Settle { .. }
                                | LivingIntentActionV2::Explore { .. }
                                | LivingIntentActionV2::Attack { .. }
                                | LivingIntentActionV2::Defend { .. }
                        );
                        assert_eq!(
                            decision.snapshot_digest,
                            living_decision_snapshot_digest_v2(
                                &state,
                                LivingTickV2(state.tick.0 + 1)
                            )
                            .unwrap()
                        );
                    }
                }
                rules_retirement_drain(job, budgets);
            }
        }
        assert!(fleet_winner, "fixture must accept a fleet proposal");
        assert!(
            winning_actors.len() >= 2,
            "fixture must accept multiple actors"
        );
    }
    #[test]
    fn due_intent_empty_all_budgets_and_stage_cancel_retirement() {
        use super::super::step_job::LivingLifecycleProgressV2 as Progress;
        let authority = builtin_living_sandbox_v2().unwrap();
        let state = LivingGalaxyStateV2::default();
        let mut expected = None;
        for budget in 1..=1024 {
            let mut job = due_intent_job(&authority, &state);
            while job.poll(budget).unwrap() != Progress::Ready {}
            if let Some(total) = expected {
                assert_eq!(job.total_visits, total);
            } else {
                expected = Some(job.total_visits);
            }
            assert_eq!(job.take_ready_result().unwrap().0, state);
            rules_retirement_drain(job, &[budget]);
        }
        let state = route_adjustment_state();
        rules_retirement_exercise(
            || due_intent_job(&authority, &state),
            |j, b| j.poll(b),
            |j| j.take_ready_result(),
            |j, f| {
                if f {
                    j.fault = Some(super::super::step_job::LivingLifecycleJobErrorV2::Fault(
                        LivingDeterministicFaultV2::Arithmetic,
                    ));
                } else {
                    j.cancel();
                }
            },
            |j| j.test_stage_key(),
        );
    }
    #[test]
    fn due_intent_nested_domain_conflict_and_original_payload_retention() {
        let (_, state) = fleet_enumeration_fixture();
        let mut old = state.fleets.as_slice()[0].clone();
        old.order = Some(LivingFleetOrderV2 {
            kind: LivingFleetOrderKindV2::Explore,
            target_world: Some(eid(5)),
            not_before_tick: LivingTickV2(51),
            lane_path: vec![eid(7); 513],
        });
        let mut changed = old.clone();
        changed.order.as_mut().unwrap().lane_path[512] = eid(9);
        let mut reserved = old.clone();
        reserved.hulls = LivingSortedVecV2::default();
        let before = LivingSortedVecV2::new(vec![old.clone()]).unwrap();
        let mut measured = None;
        for budgets in [&[1][..], &[64][..], &[1024][..], RULES_MIXED] {
            let proposed = LivingSortedVecV2::new(vec![changed.clone()]).unwrap();
            let pointer = proposed.as_slice()[0]
                .order
                .as_ref()
                .unwrap()
                .lane_path
                .as_ptr();
            let current = LivingSortedVecV2::new(vec![reserved.clone()]).unwrap();
            let mut job = IntentReservationDomain::begin(proposed, current);
            let mut visits = 0;
            while !job.ready() {
                job.visit(&before).unwrap();
                visits += 1;
                assert!(visits < 10000);
            }
            assert!(job.conflict);
            assert!(visits > 513);
            assert_eq!(
                job.proposal_row
                    .as_ref()
                    .unwrap()
                    .order
                    .as_ref()
                    .unwrap()
                    .lane_path
                    .as_ptr(),
                pointer
            );
            let retirement = rules_retirement_drain(job, budgets);
            if let Some(expected) = measured {
                assert_eq!((visits, retirement), expected);
            } else {
                measured = Some((visits, retirement));
            }
        }
    }

    #[test]
    fn due_intent_reservation_conflict_fallback_uses_common_snapshot() {
        use super::super::step_job::LivingLifecycleProgressV2 as Progress;
        let authority = builtin_living_sandbox_v2().unwrap();
        let state = route_adjustment_state();
        for budgets in [&[1][..], &[64][..], &[1024][..], RULES_MIXED] {
            let mut job = due_intent_job(&authority, &state);
            while job.stage != IntentStage::Actor || !job.retirement.is_empty() {
                job.poll(1).unwrap();
            }
            let first = &job.batches.as_slice()[0].economy.as_ref().unwrap().ranked[0].1;
            let mut reserved = state.clone();
            match first.action {
                LivingEconomicActionV2::Construction { world, .. }
                | LivingEconomicActionV2::Hull { world, .. } => {
                    reserved.worlds.find_mut(&world).unwrap().inventory.alloy += 1
                }
                LivingEconomicActionV2::Supply {
                    existing_route: Some(id),
                    ..
                } => reserved.routes.find_mut(&id).unwrap().batch_size += 1,
                _ => panic!("fixture must exercise a changed original row"),
            }
            job.reserved = Some(std::sync::Arc::new(reserved));
            let mut polls = 0;
            while job.poll(budgets[polls % budgets.len()]).unwrap() != Progress::Ready {
                polls += 1;
            }
            let (_, events) = job.take_ready_result().unwrap();
            assert!(events.decisions.iter().any(|d|d.reasons.iter().any(|r|matches!(r,LivingDecisionReasonV2::InvalidTarget{rule,..} if rule.as_str()=="reservation_conflict"))));
            assert!(
                events
                    .decisions
                    .iter()
                    .any(|d| d.outcome == LivingDecisionOutcomeV2::Accepted)
            );
            let digest = living_decision_snapshot_digest_v2(&state, LivingTickV2(50)).unwrap();
            let mut ordinals = std::collections::BTreeMap::new();
            for decision in &events.decisions {
                assert_eq!(decision.snapshot_digest, digest);
                let next = ordinals.entry(decision.actor).or_insert(0);
                assert_eq!(decision.ordinal, *next);
                *next += 1;
            }
            rules_retirement_drain(job, budgets);
        }
    }
    #[test]
    fn due_intent_dense_lower_rank_remaps_actual_original_ordinals() {
        use super::super::step_job::LivingLifecycleProgressV2 as Progress;
        let authority = builtin_living_sandbox_v2().unwrap();
        let state = route_adjustment_state();
        let (_, reference) = due_intent_reference(&authority, &state, Default::default());
        let mut first = reference.decisions[0].clone();
        first.phase = LivingPhaseV2::UpdateDiplomacy;
        first.ordinal = 11;
        let mut second = first.clone();
        second.ordinal = 29;
        second.reasons.push(LivingDecisionReasonV2::LowerRank {
            winning_actor: first.actor,
            winning_ordinal: 11,
        });
        let mut events = super::super::receipt::LivingPendingEventsV2::new();
        events.decisions = vec![second, first];
        let (expected, expected_events) = due_intent_reference(&authority, &state, events.clone());
        for budgets in [&[1][..], &[64][..], &[1024][..], RULES_MIXED] {
            let mut job = due_intent_job(&authority, &state);
            job.events = events.clone();
            let mut polls = 0;
            while job.poll(budgets[polls % budgets.len()]).unwrap() != Progress::Ready {
                polls += 1;
            }
            let (actual, actual_events) = job.take_ready_result().unwrap();
            assert_eq!(actual, expected);
            assert_eq!(actual_events.decisions, expected_events.decisions);
            assert!(
                actual_events
                    .decisions
                    .iter()
                    .flat_map(|d| &d.reasons)
                    .any(|r| matches!(
                        r,
                        LivingDecisionReasonV2::LowerRank {
                            winning_ordinal: 0,
                            ..
                        }
                    ))
            );
            rules_retirement_drain(job, budgets);
        }
    }
    #[test]
    fn due_intent_genuine_storage_capacity_and_arithmetic_failure_are_sticky() {
        use super::super::step_job::LivingLifecycleJobErrorV2 as Error;
        let authority = builtin_living_sandbox_v2().unwrap();
        let state = route_adjustment_state();
        let mut job = due_intent_job(&authority, &state);
        while !job.worlds.as_ref().is_some_and(|d| d.storage.is_some()) {
            job.poll(1).unwrap();
        }
        let pointer = job.worlds.as_ref().unwrap().proposed.as_slice().as_ptr();
        assert!(
            job.worlds
                .as_mut()
                .unwrap()
                .storage
                .as_mut()
                .unwrap()
                .inject_structural_empty_capacity_failure()
        );
        assert_eq!(job.poll(1), Err(Error::PreparationCapacity));
        assert_eq!(job.last_poll_visits, 1);
        assert_eq!(
            job.worlds.as_ref().unwrap().proposed.as_slice().as_ptr(),
            pointer
        );
        assert_eq!(**job.source, state);
        for budget in [1, 64, 1024] {
            assert_eq!(job.poll(budget), Err(Error::PreparationCapacity));
            assert_eq!(job.last_poll_visits, 0);
        }
        assert!(matches!(
            job.take_ready_result(),
            Err(Error::PreparationCapacity)
        ));
        rules_retirement_drain(job, RULES_MIXED);
        let mut state = state;
        state.tick = LivingTickV2(u64::MAX - 1);
        let mut reference = state.clone();
        let mut events = super::super::receipt::LivingPendingEventsV2::new();
        let mut ctx = LivingStepContextV2 {
            catalog: authority.catalog(),
            genesis_seed: [0; 32],
            branch: authority.branch(),
            tick: LivingTickV2(u64::MAX),
            state: &mut reference,
            events: &mut events,
            arrivals: Vec::new(),
            repair_exclusions: Default::default(),
        };
        assert_eq!(
            generate_and_reserve_intents_with_disclosure(&mut ctx, false),
            Err(LivingDeterministicFaultV2::Arithmetic)
        );
        let mut job = due_intent_job(&authority, &state);
        let error = loop {
            if let Err(error) = job.poll(64) {
                break error;
            }
        };
        assert_eq!(error, Error::Fault(LivingDeterministicFaultV2::Arithmetic));
        for budget in [1, 64, 1024] {
            assert_eq!(job.poll(budget), Err(error.clone()));
            assert_eq!(job.last_poll_visits, 0);
        }
        assert_eq!(**job.source, state);
        rules_retirement_drain(job, RULES_MIXED);
    }
    #[test]
    fn due_intent_domain_literal_retirement_counts_and_original_paths() {
        use super::super::step_job::RetirementStep;
        let (_, state) = fleet_enumeration_fixture();
        for paths in [0, 1, 17, 513] {
            for hulls in [0, 1, 3] {
                for budgets in [&[1][..], &[64][..], &[1024][..], RULES_MIXED] {
                    let mut fleet = state.fleets.as_slice()[0].clone();
                    fleet.order = Some(LivingFleetOrderV2 {
                        kind: LivingFleetOrderKindV2::Explore,
                        target_world: Some(eid(5)),
                        not_before_tick: LivingTickV2(51),
                        lane_path: vec![eid(7); paths],
                    });
                    fleet.hulls = LivingSortedVecV2::new(
                        (0..hulls)
                            .map(|i| hull(eid(100 + i as u8), "scout", false).unwrap())
                            .collect(),
                    )
                    .unwrap();
                    let proposed = LivingSortedVecV2::new(vec![fleet.clone()]).unwrap();
                    let current = LivingSortedVecV2::new(vec![fleet]).unwrap();
                    let job = IntentReservationDomain::begin(proposed, current);
                    // Seven domain fields: 15 tree visits plus empty lane7,
                    // fresh optional storage25, two iterators1 each, two None1,
                    // sorted output4. Each fleet adds path+hulls+7 visits.
                    assert_eq!(
                        rules_retirement_drain(job, budgets),
                        69 + 2 * (paths + hulls) as u64
                    );
                }
            }
        }
        let mut fleet = state.fleets.as_slice()[0].clone();
        fleet.order = Some(LivingFleetOrderV2 {
            kind: LivingFleetOrderKindV2::Explore,
            target_world: Some(eid(5)),
            not_before_tick: LivingTickV2(51),
            lane_path: vec![eid(7); 513],
        });
        let path = fleet.order.as_ref().unwrap().lane_path.as_ptr() as usize;
        let job = IntentReservationDomain::begin(
            LivingSortedVecV2::new(vec![fleet]).unwrap(),
            LivingSortedVecV2::default(),
        );
        let mut node = retirement_node(job, LivingStateRetirementPolicyV2::ReleaseHandle);
        let mut last = 513;
        let mut removed = false;
        loop {
            let mut owners = Vec::new();
            node.creator_allocations(&mut owners);
            let tail = owners
                .iter()
                .find(|(pointer, _, _, size)| {
                    *size == std::mem::size_of::<LivingEntityIdV2>()
                        && *pointer >= path
                        && *pointer <= path + 513 * size
                })
                .map(|(_, len, _, _)| *len);
            if let Some(len) = tail {
                assert!(!removed);
                assert!(len <= last && last - len <= 1);
                last = len;
            } else {
                assert!(last <= 1);
                removed = true;
            }
            if matches!(node.retirement_step(), RetirementStep::Done) {
                break;
            }
        }
        assert!(removed);
    }

    #[test]
    fn due_intent_prospective_capacity_rejects_trial_then_falls_back() {
        use super::super::step_job::LivingLifecycleProgressV2 as Progress;
        let authority = builtin_living_sandbox_v2().unwrap();
        let state = route_adjustment_state();
        let mut job = due_intent_job(&authority, &state);
        while job.stage != IntentStage::Reserve || job.domain != 6 || !job.retirement.is_empty() {
            job.poll(1).unwrap();
        }
        let trial = job.trial.as_mut().unwrap();
        let world = trial.facilities.as_slice()[0].world;
        for i in 0..7 {
            put(
                &mut trial.facilities,
                facility(eid(230 + i), world, "extractor", Some(LivingTickV2(60))).unwrap(),
            )
            .unwrap();
        }
        let mut observed = false;
        loop {
            let progress = job.poll(1).unwrap();
            observed |= job.stage == IntentStage::CapacityReason;
            if progress == Progress::Ready {
                break;
            }
        }
        assert!(observed);
        let (actual, events) = job.take_ready_result().unwrap();
        assert!(events.decisions.iter().any(|d| {
            d.outcome == LivingDecisionOutcomeV2::Rejected
                && d.reasons
                    .iter()
                    .any(|r| matches!(r, LivingDecisionReasonV2::QueueCapacity { .. }))
        }));
        assert!(
            events
                .decisions
                .iter()
                .any(|d| d.outcome == LivingDecisionOutcomeV2::Accepted)
        );
        assert!(
            !actual
                .facilities
                .as_slice()
                .iter()
                .any(|row| (230..237).any(|n| row.id == eid(n)))
        );
        assert_eq!(**job.source, state);
        rules_retirement_drain(job, RULES_MIXED);
    }
    #[test]
    fn due_intent_invalid_trial_fault_preserves_original_authority_and_owners() {
        use super::super::step_job::LivingLifecycleJobErrorV2 as Error;
        let authority = builtin_living_sandbox_v2().unwrap();
        let state = route_adjustment_state();
        let mut job = due_intent_job(&authority, &state);
        while job.stage != IntentStage::Reserve || job.domain != 6 || !job.retirement.is_empty() {
            job.poll(1).unwrap();
        }
        job.trial
            .as_mut()
            .unwrap()
            .worlds
            .record_at_mut(0)
            .unwrap()
            .inventory
            .alloy = u32::MAX;
        let error = loop {
            if let Err(error) = job.poll(1) {
                break error;
            }
        };
        assert!(matches!(
            error,
            Error::Fault(LivingDeterministicFaultV2::Validation(_))
        ));
        let pointer = job
            .validation_source
            .as_ref()
            .unwrap()
            .worlds
            .as_slice()
            .as_ptr();
        for budget in [1, 64, 1024] {
            assert_eq!(job.poll(budget), Err(error.clone()));
            assert_eq!(job.last_poll_visits, 0);
        }
        assert_eq!(
            job.validation_source
                .as_ref()
                .unwrap()
                .worlds
                .as_slice()
                .as_ptr(),
            pointer
        );
        assert_eq!(**job.source, state);
        rules_retirement_drain(job, RULES_MIXED);
    }
    #[test]
    fn due_intent_positive_cross_system_supply_and_funding() {
        use super::super::step_job::LivingLifecycleProgressV2 as Progress;
        let authority = builtin_living_sandbox_v2().unwrap();
        let state = due_intent_cross_system_state();
        state.validate(authority.catalog()).unwrap();
        let mut job = due_intent_job(&authority, &state);
        while job.poll(64).unwrap() != Progress::Ready {}
        let (actual, events) = job.take_ready_result().unwrap();
        let route = actual
            .routes
            .as_slice()
            .iter()
            .find(|r| r.id == eid(90))
            .unwrap();
        assert!(!route.suspended);
        assert_eq!(route.source_reserve, 20);
        assert_eq!(route.next_due, LivingTickV2(77));
        assert!(
            events
                .decisions
                .iter()
                .any(|d| d.outcome == LivingDecisionOutcomeV2::Accepted
                    && matches!(d.action, LivingIntentActionV2::Route { .. }))
        );
        let original_alloy: u64 = state
            .worlds
            .as_slice()
            .iter()
            .map(|r| u64::from(r.inventory.alloy))
            .sum();
        let final_alloy: u64 = actual
            .worlds
            .as_slice()
            .iter()
            .map(|r| u64::from(r.inventory.alloy))
            .sum();
        let jobs = actual
            .hull_jobs
            .as_slice()
            .iter()
            .filter(|r| !state.hull_jobs.as_slice().iter().any(|o| o.id == r.id))
            .map(|r| u64::from(r.paid_alloy))
            .sum::<u64>()
            + actual
                .construction_jobs
                .as_slice()
                .iter()
                .filter(|r| {
                    !state
                        .construction_jobs
                        .as_slice()
                        .iter()
                        .any(|o| o.id == r.id)
                })
                .map(|r| u64::from(r.paid_alloy))
                .sum::<u64>();
        assert_eq!(original_alloy, final_alloy + jobs);
        rules_retirement_drain(job, RULES_MIXED);
    }

    #[test]
    fn due_intent_positive_paid_construction_hull_and_emergency() {
        use super::super::step_job::LivingLifecycleProgressV2 as Progress;
        for (policy, emergency, definition) in [
            ("neutral", false, "foundry"),
            ("guardian", false, "escort"),
            ("guardian", true, "solar_array"),
        ] {
            // The existing independent ranking regression establishes these
            // concrete winners. Reuse its gameplay state, not a private cursor edit.
            let mut authority = builtin_first_expansion_v2().unwrap();
            for _ in 0..49 {
                authority.step().unwrap();
            }
            let mut state = authority.state().clone();
            let actor = state.civilizations.as_slice()[0].id;
            let home = state.colonies.as_slice()[0].world;
            state.civilizations.find_mut(&actor).unwrap().policy =
                LivingPolicyV2(slug(policy).unwrap());
            remove_key(
                &mut state.fleets.find_mut(&eid(11)).unwrap().hulls,
                &eid(14),
            )
            .unwrap();
            if emergency {
                remove_key(&mut state.facilities, &eid(8)).unwrap();
                state.worlds.find_mut(&home).unwrap().inventory.energy = 0;
                let hull = state
                    .fleets
                    .find_mut(&eid(11))
                    .unwrap()
                    .hulls
                    .find_mut(&eid(13))
                    .unwrap();
                hull.hit_points = 1;
                hull.next_repair = Some(LivingTickV2(50));
            }
            let mut ignored = super::super::receipt::LivingPendingEventsV2::new();
            let ctx = LivingStepContextV2 {
                catalog: authority.catalog(),
                genesis_seed: [0; 32],
                branch: authority.branch(),
                tick: LivingTickV2(50),
                state: &mut state,
                events: &mut ignored,
                arrivals: Vec::new(),
                repair_exclusions: Default::default(),
            };
            for target in [home, eid(5)] {
                let observation = observe_world(&ctx, actor, target, ctx.tick).unwrap();
                put(&mut ctx.state.observations, observation).unwrap();
            }
            let (expected, expected_events) =
                due_intent_reference(&authority, &state, Default::default());
            for budgets in [&[1][..], &[64][..], &[1024][..], RULES_MIXED] {
                let mut job = due_intent_job(&authority, &state);
                let mut calls = 0;
                loop {
                    let budget = budgets[calls % budgets.len()];
                    calls += 1;
                    if job.poll(budget).unwrap() == Progress::Ready {
                        break;
                    }
                }
                let (actual, events) = job.take_ready_result().unwrap();
                assert_eq!(actual, expected);
                assert_eq!(events.decisions, expected_events.decisions);
                let jobs: Vec<_> = actual
                    .construction_jobs
                    .as_slice()
                    .iter()
                    .filter(|r| r.owner == actor)
                    .map(|r| (r.definition.as_str(), r.paid_alloy))
                    .chain(
                        actual
                            .hull_jobs
                            .as_slice()
                            .iter()
                            .filter(|r| r.owner == actor)
                            .map(|r| (r.definition.as_str(), r.paid_alloy)),
                    )
                    .collect();
                assert_eq!(jobs.len(), 1);
                assert_eq!(jobs[0].0, definition);
                assert!(jobs[0].1 > 0);
                let alloy = |s: &LivingGalaxyStateV2| {
                    s.worlds
                        .as_slice()
                        .iter()
                        .map(|r| u64::from(r.inventory.alloy))
                        .sum::<u64>()
                };
                assert_eq!(alloy(&state) - alloy(&actual), u64::from(jobs[0].1));
                assert_eq!(
                    events
                        .decisions
                        .iter()
                        .filter(|d| d.actor == actor
                            && d.outcome == LivingDecisionOutcomeV2::Accepted
                            && matches!(
                                d.action,
                                LivingIntentActionV2::Construction { .. }
                                    | LivingIntentActionV2::Hull { .. }
                            ))
                        .count(),
                    1
                );
                rules_retirement_drain(job, budgets);
            }
        }
    }

    #[test]
    fn due_intent_full_colony_replacement_and_original_facility_retirement() {
        use super::super::step_job::{LivingLifecycleProgressV2 as Progress, RetirementStep};
        let authority = builtin_first_expansion_v2().unwrap();
        for stock in [0, 200] {
            let mut state = authority.state().clone();
            state.tick = LivingTickV2(49);
            remove_key(&mut state.facilities, &eid(10)).unwrap();
            for offset in 0..4 {
                let mut row = facility(
                    eid(60 + offset),
                    eid(4),
                    "solar_array",
                    Some(LivingTickV2(60)),
                )
                .unwrap();
                if offset == 0 {
                    row.disabled = true;
                    row.status = LivingFacilityStatusV2::Blocked;
                    row.blocked_reasons =
                        LivingSortedVecV2::new(vec![LivingBlockedReasonV2::Disabled]).unwrap();
                }
                put(&mut state.facilities, row).unwrap();
            }
            state.worlds.find_mut(&eid(4)).unwrap().inventory = LivingInventoryV2 {
                energy: LIVING_RESOURCE_STORAGE_LIMIT_V2,
                ore: LIVING_RESOURCE_STORAGE_LIMIT_V2,
                alloy: stock,
            };
            let mut ignored = super::super::receipt::LivingPendingEventsV2::new();
            let ctx = LivingStepContextV2 {
                catalog: authority.catalog(),
                genesis_seed: [0; 32],
                branch: authority.branch(),
                tick: LivingTickV2(50),
                state: &mut state,
                events: &mut ignored,
                arrivals: Vec::new(),
                repair_exclusions: Default::default(),
            };
            let observation = observe_world(&ctx, eid(3), eid(5), ctx.tick).unwrap();
            put(&mut ctx.state.observations, observation).unwrap();
            let (expected, expected_events) =
                due_intent_reference(&authority, &state, Default::default());
            for budgets in [&[1][..], &[64][..], &[1024][..], RULES_MIXED] {
                let mut job = due_intent_job(&authority, &state);
                let mut calls = 0;
                loop {
                    let budget = budgets[calls % budgets.len()];
                    calls += 1;
                    if job.poll(budget).unwrap() == Progress::Ready {
                        break;
                    }
                }
                let (actual, events) = job.take_ready_result().unwrap();
                assert_eq!(actual, expected);
                assert_eq!(events.decisions, expected_events.decisions);
                if stock == 0 {
                    assert_eq!(actual.facilities, state.facilities);
                    assert!(actual.construction_jobs.as_slice().is_empty());
                } else {
                    // At full Energy, solar eid(8) is forecast-idle and is
                    // the smallest eligible duplicate, before disabled eid(60).
                    assert_eq!(actual.facilities.as_slice().len(), 5);
                    assert!(!actual.facilities.as_slice().iter().any(|r| r.id == eid(8)));
                    let paid = actual.construction_jobs.as_slice();
                    assert_eq!(paid.len(), 1);
                    assert_eq!(paid[0].definition.as_str(), "shipyard");
                    assert_eq!(
                        state
                            .worlds
                            .as_slice()
                            .iter()
                            .find(|r| r.id == eid(4))
                            .unwrap()
                            .inventory
                            .alloy
                            - actual
                                .worlds
                                .as_slice()
                                .iter()
                                .find(|r| r.id == eid(4))
                                .unwrap()
                                .inventory
                                .alloy,
                        paid[0].paid_alloy
                    );
                }
                rules_retirement_drain(job, budgets);
            }
            if stock != 0 {
                let mut job = due_intent_job(&authority, &state);
                while job.facility_remove.is_none() {
                    job.poll(1).unwrap();
                }
                let mut original = Vec::new();
                job.facility_remove.creator_allocations(&mut original);
                let nested = original
                    .iter()
                    .find(|(_, n, _, size)| {
                        *n == 1 && *size == std::mem::size_of::<LivingBlockedReasonV2>()
                    })
                    .copied()
                    .expect("original blocked-reason allocation");
                job.cancel();
                assert_eq!(
                    job.poll(1),
                    Err(super::super::step_job::LivingLifecycleJobErrorV2::Cancelled)
                );
                // Exercise the actual parent field edge, retaining its original
                // filter and nested reason allocation while the owner drains.
                let mut node = job
                    .retirement_child(28, LivingStateRetirementPolicyV2::ReleaseHandle)
                    .unwrap();
                let mut saw_original = false;
                let mut remaining = nested.1;
                loop {
                    let mut held = Vec::new();
                    node.creator_allocations(&mut held);
                    if let Some(row) = held.iter().find(|r| {
                        r.3 == nested.3 && r.0 >= nested.0 && r.0 <= nested.0 + nested.1 * nested.3
                    }) {
                        assert!(row.1 <= remaining && remaining - row.1 <= 1);
                        if let Some(capacity) = row.2 {
                            assert_eq!(Some(capacity), nested.2);
                        }
                        remaining = row.1;
                        saw_original = true;
                    } else {
                        assert!(remaining <= 1);
                    }
                    if matches!(node.retirement_step(), RetirementStep::Done) {
                        break;
                    }
                }
                assert!(saw_original);
                assert_eq!(**job.source, state);
                rules_retirement_drain(job, RULES_MIXED);
            }
        }
    }

    #[test]
    fn due_intent_reason_storage_growth_capacity_and_recursive_diagnostic_retention() {
        use super::super::step_job::LivingLifecycleJobErrorV2 as Error;
        let authority = builtin_living_sandbox_v2().unwrap();
        let state = route_adjustment_state();
        for fail in [false, true] {
            let mut job = due_intent_job(&authority, &state);
            while job.reason_storage.is_none() {
                job.poll(1).unwrap();
            }
            let mut original = Vec::new();
            job.reason_storage.creator_allocations(&mut original);
            let original = original
                .into_iter()
                .find(|(_, len, _, size)| {
                    *len > 0 && *size == std::mem::size_of::<LivingDecisionReasonV2>()
                })
                .unwrap();
            if fail {
                assert!(
                    job.reason_storage
                        .as_mut()
                        .unwrap()
                        .inject_structural_growth_capacity_failure()
                );
                assert_eq!(job.poll(1), Err(Error::PreparationCapacity));
                assert_eq!(job.last_poll_visits, 1);
                let mut held = Vec::new();
                job.reason_storage.creator_allocations(&mut held);
                assert!(held.contains(&original));
                for budget in [1, 64, 1024] {
                    assert_eq!(job.poll(budget), Err(Error::PreparationCapacity));
                    assert_eq!(job.last_poll_visits, 0);
                }
            } else {
                while job.stage == IntentStage::Reasons || !job.retirement.is_empty() {
                    job.poll(1).unwrap();
                }
                let reasons = &job.decision.as_ref().unwrap().reasons;
                assert!(reasons.capacity() > reasons.len());
                let prepared = reasons.as_ptr();
                let len = reasons.len();
                while job.stage != IntentStage::DecisionStore {
                    job.poll(1).unwrap();
                }
                let reasons = &job.decision.as_ref().unwrap().reasons;
                assert_eq!(reasons.as_ptr(), prepared);
                assert_eq!(reasons.len(), len + 1);
                job.cancel();
            }
            assert_eq!(**job.source, state);
            rules_retirement_drain(job, RULES_MIXED);
        }
        // Unreachable from due-intent children, but a recursively owned injected
        // diagnostic must never be recursively cloned by terminal reporting.
        let mut job = due_intent_job(&authority, &state);
        let mut fault = LivingDeterministicFaultV2::Invariant;
        for _ in 0..513 {
            fault = LivingDeterministicFaultV2::Replay {
                accepted_sequence: 1,
                reason: Box::new(super::super::simulation::LivingCommandRejectionV2::Seal(
                    fault,
                )),
            };
        }
        job.fault = Some(Error::Fault(fault));
        for budget in [1, 64, 1024] {
            assert_eq!(
                job.poll(budget),
                Err(Error::Fault(LivingDeterministicFaultV2::Invariant))
            );
            assert_eq!(job.last_poll_visits, 0);
        }
        assert!(matches!(
            job.take_ready_result(),
            Err(Error::Fault(LivingDeterministicFaultV2::Invariant))
        ));
        rules_retirement_drain(job, RULES_MIXED);
    }

    #[test]
    fn due_intent_facility_and_batch_literal_nested_retirement_counts() {
        let authority = builtin_living_sandbox_v2().unwrap();
        let state = route_adjustment_state();
        let (_, events) = due_intent_reference(&authority, &state, Default::default());
        for n in [0, 1, 7, 65] {
            for reasons in [0, 1, 3] {
                for budgets in [&[1][..], &[64][..], &[1024][..], RULES_MIXED] {
                    let mut row = state.facilities.as_slice()[0].clone();
                    let entries = vec![
                        LivingBlockedReasonV2::Disabled,
                        LivingBlockedReasonV2::MissingDeposit {
                            required: 1,
                            available: 0,
                        },
                        LivingBlockedReasonV2::MissingInput {
                            resource: LivingResourceV2::Energy,
                            required: 1,
                            available: 0,
                        },
                    ];
                    row.blocked_reasons =
                        LivingSortedVecV2::new(entries.into_iter().take(reasons).collect())
                            .unwrap();
                    let domain = IntentReservationDomain::begin(
                        LivingSortedVecV2::new(vec![row.clone()]).unwrap(),
                        LivingSortedVecV2::new(vec![row]).unwrap(),
                    );
                    assert_eq!(
                        rules_retirement_drain(domain, budgets),
                        65 + 2 * reasons as u64
                    );
                    let batches = (0..n)
                        .map(|_| {
                            let mut decision = events.decisions[0].clone();
                            decision.reasons = (0..reasons)
                                .map(|_| LivingDecisionReasonV2::Accepted {
                                    rule: slug("atomic_reservation").unwrap(),
                                })
                                .collect();
                            LivingFleetBatchV2 {
                                actor: decision.actor,
                                economy: Some(LivingEconomicBatchV2 {
                                    actor: decision.actor,
                                    economic_count: 1,
                                    ranked: vec![(
                                        0,
                                        LivingEconomicCandidateV2 {
                                            tier: 1,
                                            score: 1,
                                            target: eid(5),
                                            action: LivingEconomicActionV2::Hull {
                                                world: eid(5),
                                                definition: "scout",
                                            },
                                        },
                                    )],
                                    explanations: vec![decision],
                                }),
                                fleets: Vec::with_capacity(7),
                            }
                        })
                        .collect::<Vec<_>>();
                    // Each batch: outer roots/edges18, one scalar candidate3,
                    // one decision6 plus three visits per reason; Vec root1.
                    assert_eq!(
                        rules_retirement_drain(batches, budgets),
                        1 + n as u64 * (27 + 3 * reasons as u64)
                    );
                }
            }
        }
    }

    const RULES_MIXED: &[usize] = &[1, 64, 7, 1024, 3];
    fn rules_retirement_drain<T: RetirementFields<'static>>(owner: T, budgets: &[usize]) -> u64 {
        use super::super::step_job::{
            LivingLifecycleJobErrorV2 as Error, LivingLifecycleProgressV2 as Progress,
        };
        let mut job =
            LivingPhaseRetirementJobV2::begin(owner, LivingStateRetirementPolicyV2::ReleaseHandle);
        assert_eq!(job.total_visits(), 0);
        for invalid in [0, 1025] {
            assert_eq!(job.poll(invalid), Err(Error::Budget));
            assert_eq!(job.last_poll_visits(), 0);
        }
        let mut index = 0;
        loop {
            let before = job.total_visits();
            let budget = budgets[index % budgets.len()];
            let progress = job.poll(budget).unwrap();
            assert!(job.last_poll_visits() <= budget);
            assert_eq!(job.total_visits() - before, job.last_poll_visits() as u64);
            assert!(job.take_deferred_owner().is_none());
            index += 1;
            if progress == Progress::Ready {
                break;
            }
            assert!(index < 2_000_000, "retirement did not complete");
        }
        let total = job.total_visits();
        assert_eq!(job.poll(1024), Ok(Progress::Ready));
        assert_eq!(job.last_poll_visits(), 0);
        assert_eq!(job.total_visits(), total);
        total
    }
    fn rules_retirement_exercise<T, U>(
        factory: impl Fn() -> T,
        poll: impl Fn(
            &mut T,
            usize,
        ) -> Result<
            super::super::step_job::LivingLifecycleProgressV2,
            super::super::step_job::LivingLifecycleJobErrorV2,
        >,
        take: impl Fn(&mut T) -> Result<U, super::super::step_job::LivingLifecycleJobErrorV2>,
        stop: impl Fn(&mut T, bool),
        key: impl Fn(&T) -> (u8, bool),
    ) where
        T: RetirementFields<'static>,
        U: RetirementFields<'static>,
    {
        use super::super::step_job::{
            LivingLifecycleJobErrorV2 as Error, LivingLifecycleProgressV2 as Progress,
        };
        let mut probe = factory();
        assert!(matches!(take(&mut probe), Err(Error::NotReady)));
        let mut seen = std::collections::BTreeMap::new();
        let mut cuts = vec![0];
        let mut calls = 0;
        loop {
            let count = seen.entry(key(&probe)).or_insert(0usize);
            *count += 1;
            // Sample stage entrance and later owning cleanup substates, not just discriminants.
            if [1, 2, 5].contains(count) {
                cuts.push(calls);
            }
            let progress = poll(&mut probe, 1).unwrap();
            calls += 1;
            if progress == Progress::Ready {
                cuts.push(calls);
                break;
            }
            assert!(calls < 2_000_000, "forward fixture did not finish");
        }
        let output = take(&mut probe).expect("ready output");
        assert!(matches!(take(&mut probe), Err(Error::NotReady)));
        assert_eq!(poll(&mut probe, 1024), Ok(Progress::Ready));
        // Residual children release under the output's exact source/catalog anchors.
        rules_retirement_drain(probe, RULES_MIXED);
        rules_retirement_drain(output, RULES_MIXED);
        cuts.sort_unstable();
        cuts.dedup();
        for cutoff in cuts {
            for fault in [false, true] {
                let mut expected = None;
                for budgets in [&[1][..], &[64][..], &[1024][..], RULES_MIXED] {
                    let mut job = factory();
                    for _ in 0..cutoff {
                        poll(&mut job, 1).unwrap();
                    }
                    stop(&mut job, fault);
                    let error = if fault {
                        Error::Fault(LivingDeterministicFaultV2::Arithmetic)
                    } else {
                        Error::Cancelled
                    };
                    assert!(matches!(take(&mut job),Err(ref actual) if actual==&error));
                    assert_eq!(poll(&mut job, 1), Err(error));
                    let count = rules_retirement_drain(job, budgets);
                    if let Some(expected) = expected {
                        assert_eq!(count, expected, "cut {cutoff}");
                    } else {
                        expected = Some(count);
                    }
                }
            }
        }
    }
    #[test]
    fn rules_retirement_forecast_input_cutpoints() {
        let authority = builtin_living_sandbox_v2().unwrap();
        let source = std::sync::Arc::new(authority.state().clone());
        let actor = source.civilizations.as_slice()[0].id;
        rules_retirement_exercise(
            || LivingForecastInputJobV2::begin(source.clone(), actor, LivingTickV2(1)),
            |j, b| j.poll(b),
            |j| j.take_ready_result(),
            |j, f| {
                if f {
                    j.fault = Some(super::super::step_job::LivingLifecycleJobErrorV2::Fault(
                        LivingDeterministicFaultV2::Arithmetic,
                    ))
                } else {
                    j.cancel()
                }
            },
            |j| (j.stage as u8, !j.retirement.is_empty()),
        );
    }
    #[test]
    fn rules_retirement_scheduled_cutpoints() {
        let authority = builtin_living_sandbox_v2().unwrap();
        let source = std::sync::Arc::new(authority.state().clone());
        let actor = source.civilizations.as_slice()[0].id;
        rules_retirement_exercise(
            || {
                LivingScheduledForecastJobV2::begin_owned(
                    authority.catalog_snapshot(),
                    source.clone(),
                    actor,
                    LivingTickV2(1),
                    [0; 32],
                    authority.branch(),
                )
            },
            |j, b| j.poll(b),
            |j| j.take_ready_result(),
            |j, f| {
                if f {
                    j.fault = Some(super::super::step_job::LivingLifecycleJobErrorV2::Fault(
                        LivingDeterministicFaultV2::Arithmetic,
                    ))
                } else {
                    j.cancel()
                }
            },
            |j| (j.stage as u8, !j.retirement.is_empty()),
        );
    }
    #[test]
    fn rules_retirement_inputs_cutpoints() {
        let authority = builtin_living_sandbox_v2().unwrap();
        let source = std::sync::Arc::new(authority.state().clone());
        let tick = LivingTickV2(source.tick.0 + 1);
        rules_retirement_exercise(
            || {
                LivingIntentInputsJobV2::begin(
                    authority.catalog_snapshot(),
                    source.clone(),
                    tick,
                    [0; 32],
                    authority.branch(),
                )
            },
            |j, b| j.poll(b),
            |j| j.finish(),
            |j, f| {
                if f {
                    j.fault = Some(super::super::step_job::LivingLifecycleJobErrorV2::Fault(
                        LivingDeterministicFaultV2::Arithmetic,
                    ))
                } else {
                    j.cancel()
                }
            },
            |j| (j.stage as u8, !j.retirement.is_empty()),
        );
    }
    #[test]
    fn rules_retirement_economic_cutpoints() {
        let (authority, state) = fleet_enumeration_fixture();
        let tick = LivingTickV2(state.tick.0 + 1);
        rules_retirement_exercise(
            || {
                LivingEconomicEnumerationJobV2::begin(
                    authority.catalog_snapshot(),
                    fleet_test_economics(&authority, &state, 0)
                        .inputs
                        .0
                        .take()
                        .unwrap(),
                    tick,
                )
            },
            |j, b| j.poll(b),
            |j| j.finish(),
            |j, f| {
                if f {
                    j.fault = Some(super::super::step_job::LivingLifecycleJobErrorV2::Fault(
                        LivingDeterministicFaultV2::Arithmetic,
                    ))
                } else {
                    j.cancel()
                }
            },
            |j| (j.stage as u8, !j.retirement.is_empty()),
        );
    }
    #[test]
    fn rules_retirement_fleet_cutpoints() {
        let (authority, state) = fleet_enumeration_fixture();
        rules_retirement_exercise(
            || fleet_test_job(&authority, &state, 0),
            |j, b| j.poll(b),
            |j| j.finish(),
            |j, f| {
                if f {
                    j.fault = Some(super::super::step_job::LivingLifecycleJobErrorV2::Fault(
                        LivingDeterministicFaultV2::Arithmetic,
                    ))
                } else {
                    j.cancel()
                }
            },
            |j| (j.stage as u8, !j.retirement.is_empty()),
        );
    }
    #[test]
    fn rules_retirement_byte_storage_exact_counts_and_empty_allocation() {
        for budgets in [&[1][..], &[64][..], &[1024][..], RULES_MIXED] {
            // Byte storage is a single release, independent of payload length.
            assert_eq!(
                rules_retirement_drain(RulesByteBuffer(Vec::with_capacity(4096)), budgets),
                1
            );
            assert_eq!(
                rules_retirement_drain(RulesByteBuffer(vec![7; 4096]), budgets),
                1
            );
            // Key child setup + byte buffer release + child Box release + key root = 4.
            assert_eq!(
                rules_retirement_drain(RulesFleetKey((0, None, vec![7; 4096], 0)), budgets),
                4
            );
            // Each of 3 IDs: acquisition + record release + node release; then Vec storage.
            assert_eq!(
                rules_retirement_drain(vec![eid(1), eid(2), eid(3)], budgets),
                10
            );
        }
    }
    #[test]
    fn rules_retirement_exact_shared_ticket_and_no_internal_alias_deadlock() {
        use super::super::step_job::LivingLifecycleProgressV2 as Progress;
        let authority = builtin_living_sandbox_v2().unwrap();
        let source = std::sync::Arc::new(authority.state().clone());
        let weak = std::sync::Arc::downgrade(&source);
        let ptr = std::sync::Arc::as_ptr(&source);
        let job = LivingForecastInputJobV2::begin(
            source.clone(),
            source.civilizations.as_slice()[0].id,
            LivingTickV2(1),
        );
        let mut retire = job.into_rules_retirement(LivingStateRetirementPolicyV2::DeferUntilUnique);
        let ticket = loop {
            assert_eq!(retire.poll(1).unwrap(), Progress::Pending);
            if let Some(ticket) = retire.take_deferred_owner() {
                break ticket;
            }
        };
        assert_eq!(std::sync::Arc::as_ptr(&weak.upgrade().unwrap()), ptr);
        assert_eq!(
            weak.strong_count(),
            2,
            "only caller and exact deferred owner remain"
        );
        while retire.poll(1).unwrap() != Progress::Ready {}
        drop(source);
        let mut resumed = LivingPhaseRetirementJobV2::begin(
            ticket,
            LivingStateRetirementPolicyV2::DeferUntilUnique,
        );
        while resumed.poll(1).unwrap() != Progress::Ready {
            assert!(resumed.take_deferred_owner().is_none());
        }
        assert!(weak.upgrade().is_none());
    }
    #[test]
    fn rules_retirement_malformed_forecast_row_preserves_nested_pointer() {
        let authority = builtin_living_sandbox_v2().unwrap();
        let source = std::sync::Arc::new(authority.state().clone());
        let actor = source.civilizations.as_slice()[0].id;
        let mut job = LivingForecastInputJobV2::begin(source, actor, LivingTickV2(1));
        job.stage = ForecastInputStage::Facilities;
        let row = LivingFacilityV2 {
            id: eid(100),
            world: eid(5),
            definition: slug("foundry").unwrap(),
            status: LivingFacilityStatusV2::Blocked,
            paid_alloy: 0,
            hit_points: 1,
            next_due: None,
            next_repair: None,
            blocked_reasons: LivingSortedVecV2::default(),
            disabled: false,
        };
        job.world_owners.insert(eid(5), Some(actor));
        job.state.facilities = LivingSortedVecV2::new(vec![row.clone()]).unwrap();
        let mut row = row;
        row.blocked_reasons = LivingSortedVecV2::new(Vec::with_capacity(100)).unwrap();
        let pointer = row.blocked_reasons.as_slice().as_ptr();
        job.facilities = vec![row].into_iter();
        assert!(job.poll(1).is_err());
        assert_eq!(
            job.pending_facility
                .as_ref()
                .unwrap()
                .blocked_reasons
                .as_slice()
                .as_ptr(),
            pointer
        );
        rules_retirement_drain(job, RULES_MIXED);
    }
    #[test]
    fn rules_retirement_boxed_forecast_and_candidate_preserve_allocations() {
        let authority = builtin_living_sandbox_v2().unwrap();
        let source = std::sync::Arc::new(authority.state().clone());
        let actor = source.civilizations.as_slice()[0].id;
        let mut job = Box::new(LivingScheduledForecastJobV2::begin_owned(
            authority.catalog_snapshot(),
            source,
            actor,
            LivingTickV2(1),
            [0; 32],
            authority.branch(),
        ));
        let ptr = std::ptr::from_ref(job.as_ref());
        // Each field extraction borrows the original Box, including when nested child exists.
        for index in 0..26 {
            if let Some(mut child) =
                job.retirement_child(index, LivingStateRetirementPolicyV2::ReleaseHandle)
            {
                while !matches!(
                    child.retirement_step(),
                    super::super::step_job::RetirementStep::Done
                ) {}
                assert_eq!(std::ptr::from_ref(job.as_ref()), ptr);
            }
        }
        let (authority, state) = fleet_enumeration_fixture();
        let mut job = fleet_test_job(&authority, &state, 0);
        while job.candidate.is_none() {
            job.poll(1).unwrap();
        }
        let candidate = job.candidate.as_ref().unwrap();
        let ptr = std::ptr::from_ref(candidate.as_ref());
        job.cancel();
        assert_eq!(
            std::ptr::from_ref(job.candidate.as_ref().unwrap().as_ref()),
            ptr
        );
        rules_retirement_drain(job, RULES_MIXED);
    }
    #[test]
    fn rules_retirement_composed_phase_box_pointers_and_unused_outputs() {
        use super::super::step_job::LivingLifecycleProgressV2 as Progress;
        let authority = builtin_living_sandbox_v2().unwrap();
        macro_rules! child {
            ($field:ident) => {{
                let source = std::sync::Arc::new(authority.state().clone());
                let actor = source.civilizations.as_slice()[0].id;
                let mut job = LivingScheduledForecastJobV2::begin_owned(
                    authority.catalog_snapshot(),
                    source,
                    actor,
                    LivingTickV2(1),
                    [0; 32],
                    authority.branch(),
                );
                while job.$field.is_none() {
                    job.poll(1).unwrap();
                }
                let mut child = job.$field.take().unwrap();
                let ptr = std::ptr::from_ref(child.as_ref());
                while child.poll(1).unwrap() != Progress::Ready {}
                let output = child.take_ready_result().unwrap();
                assert_eq!(std::ptr::from_ref(child.as_ref()), ptr);
                assert!(child.take_ready_result().is_err());
                rules_retirement_drain(child, RULES_MIXED);
                rules_retirement_drain(output, RULES_MIXED);
                rules_retirement_drain(job, RULES_MIXED);
            }};
        }
        child!(scheduled);
        child!(production);
        child!(dispatch);
    }
    #[test]
    fn rules_retirement_malformed_assembly_keeps_original_nested_buffers() {
        let mut fleet = fleet_with(
            eid(90),
            eid(3),
            eid(5),
            vec![hull(eid(1), "scout", false).unwrap()],
        )
        .unwrap();
        fleet.order = Some(LivingFleetOrderV2 {
            kind: LivingFleetOrderKindV2::Move,
            target_world: Some(eid(6)),
            not_before_tick: LivingTickV2(51),
            lane_path: vec![eid(7); 512],
        });
        let mut job = LivingFleetAssemblyCursorV2::begin(
            LivingSortedVecV2::default(),
            FleetAssemblyChange::Rekey {
                fleet: eid(90),
                id: eid(91),
            },
        );
        job.stage = 7;
        job.output = LivingSortedVecV2::new(vec![fleet.clone()]).unwrap();
        let hulls = fleet.hulls.as_slice().as_ptr();
        let path = fleet.order.as_ref().unwrap().lane_path.as_ptr();
        job.rows.insert(fleet.id, fleet);
        assert!(job.visit().is_err());
        let held = job.pending_output.as_ref().unwrap();
        assert_eq!(held.hulls.as_slice().as_ptr(), hulls);
        assert_eq!(held.order.as_ref().unwrap().lane_path.as_ptr(), path);
        rules_retirement_drain(job, RULES_MIXED);
    }
    #[test]
    fn rules_retirement_discarded_forecast_events_block_successor() {
        let authority = builtin_living_sandbox_v2().unwrap();
        let source = std::sync::Arc::new(authority.state().clone());
        let actor = source.civilizations.as_slice()[0].id;
        let mut job = LivingScheduledForecastJobV2::begin_owned(
            authority.catalog_snapshot(),
            source,
            actor,
            LivingTickV2(1),
            [0; 32],
            authority.branch(),
        );
        while job.stage != ScheduledForecastStage::Start {
            job.poll(1).unwrap();
        }
        // Forecast events are retained through the boundary and explicitly discarded.
        job.events
            .record(
                LivingEventProvenanceV2::Autonomous {
                    phase: LivingPhaseV2::GenerateIntents,
                },
                LivingEventKindV2::FleetDeparted,
            )
            .unwrap();
        job.poll(1).unwrap();
        assert_eq!(
            job.stage as u8,
            ScheduledForecastStage::BeginScheduled as u8
        );
        let mut cleanup = 0;
        while !job.retirement.is_empty() {
            assert!(job.scheduled.is_none());
            job.poll(1).unwrap();
            assert!(job.last_poll_visits <= 1);
            cleanup += 1;
        }
        assert!(cleanup > 1);
        assert!(job.scheduled.is_none());
        rules_retirement_drain(job, RULES_MIXED);
    }
    #[test]
    fn rules_retirement_nested_empty_batch_exact_count() {
        for budgets in [&[1][..], &[64][..], &[1024][..], RULES_MIXED] {
            // Option field: setup/release/node=3; allocated-empty Vec: same 3; parent=1.
            let batch = LivingFleetBatchV2 {
                actor: eid(1),
                economy: None,
                fleets: Vec::with_capacity(17),
            };
            assert_eq!(rules_retirement_drain(batch, budgets), 7);
        }
    }
}

#[cfg(test)]
mod corridor_restriction_producer_tests {
    use super::*;
    fn exercise(
        batch: u32,
        axis: u8,
    ) -> (
        Vec<super::super::event_facts::LivingCorridorRestrictionV2>,
        usize,
    ) {
        let authority = builtin_first_expansion_v2().unwrap();
        let mut state = authority.state().clone();
        state.worlds.find_mut(&eid(4)).unwrap().inventory.ore = if axis == 1 { 0 } else { 100 };
        state.worlds.find_mut(&eid(5)).unwrap().owner = Some(eid(3));
        let route = LivingFreightRouteV2 {
            id: eid(90),
            kind: LivingRouteKindV2::Internal,
            source_world: eid(4),
            destination_world: eid(5),
            resource: LivingResourceV2::Ore,
            batch_size: batch,
            cadence_ticks: 10,
            source_reserve: 0,
            source_owner: eid(3),
            receiver_owner: eid(3),
            next_due: LivingTickV2(10),
            suspended: axis == 2,
        };
        state.routes = LivingSortedVecV2::new(vec![route]).unwrap();
        state.hazards = LivingSortedVecV2::new(vec![LivingHazardV2 {
            id: eid(91),
            definition: slug("ion_storm").unwrap(),
            lane: eid(6),
            start_tick: LivingTickV2(10),
            end_tick: LivingTickV2(150),
        }])
        .unwrap();
        if axis == 3 {
            state.worlds.find_mut(&eid(5)).unwrap().owner = None;
        }
        if axis == 4 {
            state.shipments = LivingSortedVecV2::new(
                (0..super::super::model::LIVING_MAX_SHIPMENTS_V2)
                    .map(|n| LivingShipmentV2 {
                        id: LivingEntityIdV2((n as u128 + 1000).to_be_bytes()),
                        dispatch_owner: eid(3),
                        intended_receiver: eid(3),
                        source_world: eid(4),
                        destination_world: eid(5),
                        resource: LivingResourceV2::Ore,
                        units: 1,
                        departure_tick: LivingTickV2(0),
                        arrival_tick: LivingTickV2(20),
                        disposition: LivingShipmentDispositionV2::Outbound,
                        dispatch_kind: Some(LivingRouteKindV2::Internal),
                        returning_to_source: false,
                    })
                    .collect(),
            )
            .unwrap();
        }
        let initial = state.shipments.as_slice().len();
        let mut events = super::super::receipt::LivingPendingEventsV2::new();
        let mut ctx = LivingStepContextV2 {
            catalog: authority.catalog(),
            genesis_seed: [0; 32],
            branch: authority.branch(),
            tick: LivingTickV2(if axis == 5 { 150 } else { 10 }),
            state: &mut state,
            events: &mut events,
            arrivals: vec![],
            repair_exclusions: Default::default(),
        };
        dispatch_routes(&mut ctx).unwrap();
        (
            events.corridor_restrictions,
            state.shipments.as_slice().len() - initial,
        )
    }
    #[test]
    fn actual_ion_storm_partial_and_zero_attempts_preserve_dispatch_truth() {
        let (partial, count) = exercise(2, 0);
        assert_eq!(count, 1);
        assert_eq!(partial.len(), 1);
        let p = &partial[0];
        assert_eq!(
            (
                p.requested_batch(),
                p.eligible_batch(),
                p.restricted_batch(),
                p.actual_dispatched_units()
            ),
            (2, 2, 1, 1)
        );
        assert_eq!(p.tick(), LivingTickV2(10));
        assert_eq!(p.hazard().end_tick, LivingTickV2(150));
        let (zero, count) = exercise(1, 0);
        assert_eq!(count, 0);
        assert_eq!(zero.len(), 1);
        assert_eq!(
            (
                zero[0].eligible_batch(),
                zero[0].restricted_batch(),
                zero[0].actual_dispatched_units()
            ),
            (1, 0, 0)
        );
    }
    #[test]
    fn shortage_suspension_owner_capacity_and_expired_hazard_are_not_restrictions() {
        for axis in 1..=5 {
            assert!(exercise(2, axis).0.is_empty(), "axis {axis}");
        }
    }
}

// Private forecast projection. Future boundary composition retains the source
// snapshot until it retires outside publication; no owner is destroyed in poll.
struct LivingForecastInputJobV2 {
    retirement: super::step_job::PhaseRetirementLane<'static>,
    source_owner: RulesOwner<std::sync::Arc<LivingGalaxyStateV2>>,
    copy: Option<super::step_job::LivingStateCopyJobV2>,
    pending_route: Option<LivingFreightRouteV2>,
    pending_facility: Option<LivingFacilityV2>,
    pending_build: Option<LivingConstructionJobV2>,
    pending_hull_job: Option<LivingHullJobV2>,
    pending_colony: Option<LivingColonyV2>,
    state: LivingGalaxyStateV2,
    actor: LivingEntityIdV2,
    tick: LivingTickV2,
    stage: ForecastInputStage,
    index: usize,
    search: usize,
    world_owners: std::collections::BTreeMap<LivingEntityIdV2, Option<LivingEntityIdV2>>,
    idle_facilities: Vec<LivingEntityIdV2>,
    routes: std::vec::IntoIter<LivingFreightRouteV2>,
    facilities: std::vec::IntoIter<LivingFacilityV2>,
    builds: std::vec::IntoIter<LivingConstructionJobV2>,
    hull_jobs: std::vec::IntoIter<LivingHullJobV2>,
    colonies: std::vec::IntoIter<LivingColonyV2>,
    retired_reasons: std::vec::IntoIter<LivingBlockedReasonV2>,
    delivered: bool,
    cancelled: bool,
    fault: Option<super::step_job::LivingLifecycleJobErrorV2>,
    last_poll_visits: usize,
    total_visits: u64,
}
#[derive(Clone, Copy, Eq, PartialEq)]
enum ForecastInputStage {
    Copy,
    WorldOwners,
    Routes,
    Facilities,
    FacilityReasons,
    Builds,
    HullJobs,
    Colonies,
    ColonyReasons,
    Worlds,
    Observations,
    KnownRoutes,
    Shipments,
    RetireOwners,
    Ready,
}
struct LivingForecastInputV2 {
    state: LivingGalaxyStateV2,
    idle_facilities: Vec<LivingEntityIdV2>,
    // The caller owns reclamation; finishing does not destroy the source tree.
    retired_source: RulesOwner<std::sync::Arc<LivingGalaxyStateV2>>,
}
impl LivingForecastInputJobV2 {
    fn begin(
        source: std::sync::Arc<LivingGalaxyStateV2>,
        actor: LivingEntityIdV2,
        tick: LivingTickV2,
    ) -> Self {
        Self {
            retirement: Default::default(),
            copy: Some(super::step_job::LivingStateCopyJobV2::begin(
                std::sync::Arc::clone(&source),
            )),
            source_owner: source.into(),
            pending_route: None,
            pending_facility: None,
            pending_build: None,
            pending_hull_job: None,
            pending_colony: None,
            state: LivingGalaxyStateV2::default(),
            actor,
            tick,
            stage: ForecastInputStage::Copy,
            index: 0,
            search: 0,
            world_owners: Default::default(),
            idle_facilities: Vec::new(),
            routes: Vec::new().into_iter(),
            facilities: Vec::new().into_iter(),
            builds: Vec::new().into_iter(),
            hull_jobs: Vec::new().into_iter(),
            colonies: Vec::new().into_iter(),
            retired_reasons: Vec::new().into_iter(),
            delivered: false,
            cancelled: false,
            fault: None,
            last_poll_visits: 0,
            total_visits: 0,
        }
    }
    #[cfg(test)]
    fn cancel(&mut self) {
        self.cancelled = true;
    }
    #[cfg(test)]
    fn last_poll_visits(&self) -> usize {
        self.last_poll_visits
    }
    #[cfg(test)]
    fn total_visits(&self) -> u64 {
        self.total_visits
    }
    fn poll(
        &mut self,
        budget: usize,
    ) -> Result<
        super::step_job::LivingLifecycleProgressV2,
        super::step_job::LivingLifecycleJobErrorV2,
    > {
        use super::step_job::{
            LivingLifecycleJobErrorV2 as Error, LivingLifecycleProgressV2 as Progress,
        };
        self.last_poll_visits = 0;
        if self.cancelled {
            return Err(Error::Cancelled);
        }
        if budget == 0 || budget > 1024 {
            return Err(Error::Budget);
        }
        if let Some(error) = &self.fault {
            return Err(error.clone());
        }
        while self.last_poll_visits < budget
            && (self.stage != ForecastInputStage::Ready || !self.retirement.is_empty())
        {
            if self.retirement.visit() {
                self.last_poll_visits += 1;
                self.total_visits += 1;
                continue;
            }
            if self.stage == ForecastInputStage::Copy {
                let job = self
                    .copy
                    .as_mut()
                    .ok_or(Error::Fault(LivingDeterministicFaultV2::Invariant))?;
                let result = job.poll(budget - self.last_poll_visits);
                let visits = job.last_poll_visits();
                self.last_poll_visits += visits;
                self.total_visits += visits as u64;
                match result {
                    Ok(Progress::Pending) => return Ok(Progress::Pending),
                    Ok(Progress::Ready) => {
                        if self.last_poll_visits == budget {
                            break;
                        }
                        self.last_poll_visits += 1;
                        self.total_visits += 1;
                        self.state = self
                            .copy
                            .as_mut()
                            .expect("ready copy")
                            .take_ready_result()?;
                        self.retirement.start(self.copy.take().expect("ready copy"));
                        self.state.tick = self.tick;
                        self.stage = ForecastInputStage::WorldOwners;
                    }
                    Err(error) => {
                        self.fault = Some(error.clone());
                        return Err(error);
                    }
                }
            } else {
                self.last_poll_visits += 1;
                self.total_visits += 1;
                if let Err(fault) = self.visit() {
                    let error = Error::Fault(fault);
                    self.fault = Some(error.clone());
                    return Err(error);
                }
            }
        }
        Ok(
            if self.stage == ForecastInputStage::Ready && self.retirement.is_empty() {
                Progress::Ready
            } else {
                Progress::Pending
            },
        )
    }
    // Explicit synchronous compatibility adapter. Composed polls borrow instead.
    #[cfg(test)]
    fn finish(
        mut self,
    ) -> Result<LivingForecastInputV2, super::step_job::LivingLifecycleJobErrorV2> {
        self.take_ready_result()
    }
    fn take_ready_result(
        &mut self,
    ) -> Result<LivingForecastInputV2, super::step_job::LivingLifecycleJobErrorV2> {
        use super::step_job::LivingLifecycleJobErrorV2 as Error;
        if self.cancelled {
            return Err(Error::Cancelled);
        }
        if let Some(error) = &self.fault {
            return Err(error.clone());
        }
        if self.stage != ForecastInputStage::Ready || self.delivered || !self.retirement.is_empty()
        {
            return Err(Error::NotReady);
        }
        self.delivered = true;
        Ok(LivingForecastInputV2 {
            state: std::mem::take(&mut self.state),
            idle_facilities: std::mem::take(&mut self.idle_facilities),
            retired_source: std::sync::Arc::clone(&self.source_owner).into(),
        })
    }
    fn owned(&self, world: LivingEntityIdV2) -> bool {
        self.world_owners.get(&world) == Some(&Some(self.actor))
    }
    fn mask_world(
        &mut self,
        owner: Option<LivingEntityIdV2>,
    ) -> Result<(), LivingDeterministicFaultV2> {
        let row = self
            .state
            .worlds
            .record_at_mut(self.index)
            .ok_or(LivingDeterministicFaultV2::Invariant)?;
        row.inventory = LivingInventoryV2::default();
        row.owner = owner;
        self.index += 1;
        self.search = 0;
        self.stage = ForecastInputStage::Worlds;
        Ok(())
    }
    fn visit(&mut self) -> Result<(), LivingDeterministicFaultV2> {
        use ForecastInputStage::*;
        match self.stage {
            Copy | Ready => return Err(LivingDeterministicFaultV2::Invariant),
            WorldOwners => {
                if let Some(row) = self.state.worlds.as_slice().get(self.index) {
                    self.world_owners.insert(row.id, row.owner);
                    self.index += 1;
                } else {
                    self.routes = std::mem::take(&mut self.state.routes)
                        .into_vec()
                        .into_iter();
                    self.stage = Routes;
                }
            }
            Routes => {
                if let Some(row) = self.routes.next() {
                    self.pending_route = Some(row);
                    let row = self.pending_route.as_ref().expect("held row");
                    if row.source_owner == self.actor {
                        rules_ensure_append(&self.state.routes, row)?;
                        self.state
                            .routes
                            .push_sorted(self.pending_route.take().expect("checked row"))?;
                    } else {
                        self.retirement
                            .start(self.pending_route.take().expect("filtered row"));
                    }
                } else {
                    self.facilities = std::mem::take(&mut self.state.facilities)
                        .into_vec()
                        .into_iter();
                    self.stage = Facilities;
                }
            }
            Facilities => {
                if let Some(row) = self.facilities.next() {
                    self.pending_facility = Some(row);
                    let row = self.pending_facility.as_ref().expect("held row");
                    if self.owned(row.world) {
                        if matches!(
                            row.definition.as_str(),
                            "solar_array" | "extractor" | "foundry"
                        ) {
                            self.idle_facilities.push(row.id);
                        }
                        rules_ensure_append(&self.state.facilities, row)?;
                        self.state
                            .facilities
                            .push_sorted(self.pending_facility.take().expect("checked row"))?;
                    } else {
                        self.retirement
                            .start(self.pending_facility.take().expect("filtered facility"));
                        self.stage = FacilityReasons;
                    }
                } else {
                    self.builds = std::mem::take(&mut self.state.construction_jobs)
                        .into_vec()
                        .into_iter();
                    self.stage = Builds;
                }
            }
            FacilityReasons => {
                if self.retired_reasons.next().is_none() {
                    self.stage = Facilities;
                }
            }
            Builds => {
                if let Some(row) = self.builds.next() {
                    self.pending_build = Some(row);
                    let row = self.pending_build.as_ref().expect("held row");
                    if row.owner == self.actor {
                        rules_ensure_append(&self.state.construction_jobs, row)?;
                        self.state
                            .construction_jobs
                            .push_sorted(self.pending_build.take().expect("checked row"))?;
                    } else {
                        self.retirement
                            .start(self.pending_build.take().expect("filtered row"));
                    }
                } else {
                    self.hull_jobs = std::mem::take(&mut self.state.hull_jobs)
                        .into_vec()
                        .into_iter();
                    self.stage = HullJobs;
                }
            }
            HullJobs => {
                if let Some(row) = self.hull_jobs.next() {
                    self.pending_hull_job = Some(row);
                    let row = self.pending_hull_job.as_ref().expect("held row");
                    if row.owner == self.actor {
                        rules_ensure_append(&self.state.hull_jobs, row)?;
                        self.state
                            .hull_jobs
                            .push_sorted(self.pending_hull_job.take().expect("checked row"))?;
                    } else {
                        self.retirement
                            .start(self.pending_hull_job.take().expect("filtered row"));
                    }
                } else {
                    self.colonies = std::mem::take(&mut self.state.colonies)
                        .into_vec()
                        .into_iter();
                    self.stage = Colonies;
                }
            }
            Colonies => {
                if let Some(row) = self.colonies.next() {
                    self.pending_colony = Some(row);
                    let row = self.pending_colony.as_ref().expect("held row");
                    if row.owner == self.actor {
                        rules_ensure_append(&self.state.colonies, row)?;
                        self.state
                            .colonies
                            .push_sorted(self.pending_colony.take().expect("checked row"))?;
                    } else {
                        self.retirement
                            .start(self.pending_colony.take().expect("filtered colony"));
                        self.stage = ColonyReasons;
                    }
                } else {
                    self.index = 0;
                    self.stage = Worlds;
                }
            }
            ColonyReasons => {
                if self.retired_reasons.next().is_none() {
                    self.stage = Colonies;
                }
            }
            Worlds => {
                if let Some(row) = self.state.worlds.as_slice().get(self.index) {
                    if self.owned(row.id) {
                        self.index += 1;
                    } else {
                        self.search = 0;
                        self.stage = Observations;
                    }
                } else {
                    self.stage = RetireOwners;
                }
            }
            Observations => {
                let world = self.state.worlds.as_slice()[self.index].id;
                if let Some(row) = self.state.observations.as_slice().get(self.search) {
                    self.search += 1;
                    if row.civilization == self.actor && row.world == world {
                        self.mask_world(row.owner)?;
                    }
                } else {
                    self.search = 0;
                    self.stage = KnownRoutes;
                }
            }
            KnownRoutes => {
                let world = self.state.worlds.as_slice()[self.index].id;
                if let Some(row) = self.state.routes.as_slice().get(self.search) {
                    self.search += 1;
                    if row.source_owner == self.actor && row.destination_world == world {
                        self.mask_world(Some(row.receiver_owner))?;
                    }
                } else {
                    self.search = 0;
                    self.stage = Shipments;
                }
            }
            Shipments => {
                let world = self.state.worlds.as_slice()[self.index].id;
                if let Some(row) = self.state.shipments.as_slice().get(self.search) {
                    self.search += 1;
                    let owner =
                        if row.dispatch_owner == self.actor && row.destination_world == world {
                            Some(row.intended_receiver)
                        } else if row.intended_receiver == self.actor && row.source_world == world {
                            Some(row.dispatch_owner)
                        } else {
                            None
                        };
                    if let Some(owner) = owner {
                        self.mask_world(Some(owner))?;
                    }
                } else {
                    self.mask_world(None)?;
                }
            }
            RetireOwners => {
                if self.world_owners.pop_first().is_none() {
                    self.stage = Ready;
                }
            }
        }
        Ok(())
    }
}

/// Private fixed-horizon forecast. All child polls and monitoring scans share
/// the caller's record budget; compatibility callers explicitly drain it.
struct LivingScheduledForecastJobV2<'a> {
    retirement: super::step_job::PhaseRetirementLane<'a>,
    catalog: RulesOwner<super::step_job::LivingPhaseCatalogV2<'a>>,
    source: RulesOwner<std::sync::Arc<LivingGalaxyStateV2>>,
    actor: LivingEntityIdV2,
    start: LivingTickV2,
    tick: LivingTickV2,
    seed: [u8; 32],
    branch: super::ids::LivingBranchIdV2,
    preparation: Option<LivingForecastInputJobV2>,
    scheduled: Option<Box<super::step_job::LivingScheduledPhasesJobV2>>,
    projection: Option<super::step_job::RepairEligibilityProjection>,
    repair_exclusions: std::collections::BTreeSet<LivingEntityIdV2>,
    production: Option<Box<super::step_job::LivingProductionJobV2<'a>>>,
    dispatch: Option<Box<super::step_job::LivingDispatchJobV2<'a>>>,
    state: LivingGalaxyStateV2,
    events: super::receipt::LivingPendingEventsV2,
    stage: ScheduledForecastStage,
    index: usize,
    fleet_index: usize,
    hull_index: usize,
    reason_index: usize,
    offset: u64,
    idle_input: std::vec::IntoIter<LivingEntityIdV2>,
    idle: std::collections::BTreeSet<LivingEntityIdV2>,
    idle_output: Vec<LivingEntityIdV2>,
    due: std::collections::BTreeSet<LivingEntityIdV2>,
    repairs: std::collections::BTreeMap<LivingEntityIdV2, (LivingEntityIdV2, u32)>,
    hulls: std::collections::BTreeMap<LivingEntityIdV2, (u32, bool)>,
    worlds: std::collections::BTreeMap<
        LivingEntityIdV2,
        (
            Option<LivingEntityIdV2>,
            LivingEntityIdV2,
            LivingInventoryV2,
        ),
    >,
    graph: super::step_job::Graph,
    lanes: std::collections::BTreeMap<LivingEntityIdV2, (LivingEntityIdV2, LivingEntityIdV2)>,
    hazard_effects: std::collections::BTreeMap<String, super::catalog::LivingHazardEffectV2>,
    trade: std::collections::BTreeSet<(LivingEntityIdV2, LivingEntityIdV2)>,
    search: Option<super::step_job::CostSearch>,
    route_eligible: bool,
    batch: u32,
    inventories: Vec<(LivingTickV2, LivingEntityIdV2, LivingInventoryV2)>,
    deficits: Vec<(LivingEntityIdV2, LivingResourceV2)>,
    deficit_set: std::collections::BTreeSet<(LivingEntityIdV2, LivingResourceV2)>,
    retired_edges: Vec<(LivingEntityIdV2, u64)>,
    production_fault: Option<super::step_job::ProductionFailure>,
    production_preparation_capacity: bool,
    delivered: bool,
    cancelled: bool,
    fault: Option<super::step_job::LivingLifecycleJobErrorV2>,
    last_poll_visits: usize,
    total_visits: u64,
}
#[derive(Clone, Copy, Eq, PartialEq)]
enum ScheduledForecastStage {
    Preparation,
    Idle,
    Systems,
    Lanes,
    HazardDefinitions,
    Start,
    BeginScheduled,
    Scheduled,
    Projection,
    RepairWorlds,
    RepairFleets,
    RepairHulls,
    RepairBatteries,
    DueFacilities,
    Production,
    Worlds,
    FleetIndex,
    HullIndex,
    FacilityIndex,
    Repairs,
    Facilities,
    Reasons,
    Agreements,
    Routes,
    Travel,
    RetireSearch,
    Hazards,
    Dispatch,
    Inventory,
    ClearBoundary,
    IdleOutput,
    RetireGraph,
    RetireEdges,
    RetireLanes,
    RetireHazards,
    RetireDeficits,
    Ready,
}
impl<'a> LivingScheduledForecastJobV2<'a> {
    fn begin(
        catalog: &'a ValidatedLivingCatalogPackV2,
        source: std::sync::Arc<LivingGalaxyStateV2>,
        actor: LivingEntityIdV2,
        tick: LivingTickV2,
        seed: [u8; 32],
        branch: super::ids::LivingBranchIdV2,
    ) -> Self {
        Self::with_catalog(
            super::step_job::LivingPhaseCatalogV2::Borrowed(catalog),
            source,
            actor,
            tick,
            seed,
            branch,
        )
    }
    fn with_catalog(
        catalog: super::step_job::LivingPhaseCatalogV2<'a>,
        source: std::sync::Arc<LivingGalaxyStateV2>,
        actor: LivingEntityIdV2,
        tick: LivingTickV2,
        seed: [u8; 32],
        branch: super::ids::LivingBranchIdV2,
    ) -> Self {
        Self {
            retirement: Default::default(),
            catalog: catalog.into(),
            preparation: Some(LivingForecastInputJobV2::begin(
                std::sync::Arc::clone(&source),
                actor,
                tick,
            )),
            source: source.into(),
            actor,
            start: tick,
            tick,
            seed,
            branch,
            scheduled: None,
            projection: None,
            repair_exclusions: Default::default(),
            production: None,
            dispatch: None,
            state: Default::default(),
            events: super::receipt::LivingPendingEventsV2::for_forecast(),
            stage: ScheduledForecastStage::Preparation,
            index: 0,
            fleet_index: 0,
            hull_index: 0,
            reason_index: 0,
            offset: 0,
            idle_input: Vec::new().into_iter(),
            idle: Default::default(),
            idle_output: Vec::new(),
            due: Default::default(),
            repairs: Default::default(),
            hulls: Default::default(),
            worlds: Default::default(),
            graph: Default::default(),
            lanes: Default::default(),
            hazard_effects: Default::default(),
            trade: Default::default(),
            search: None,
            route_eligible: false,
            batch: 0,
            inventories: Vec::new(),
            deficits: Vec::new(),
            deficit_set: Default::default(),
            retired_edges: Vec::new(),
            delivered: false,
            cancelled: false,
            fault: None,
            production_fault: None,
            production_preparation_capacity: false,
            last_poll_visits: 0,
            total_visits: 0,
        }
    }
    #[cfg(test)]
    fn cancel(&mut self) {
        self.cancelled = true;
    }
    #[cfg(test)]
    fn last_poll_visits(&self) -> usize {
        self.last_poll_visits
    }
    #[cfg(test)]
    fn total_visits(&self) -> u64 {
        self.total_visits
    }
    fn advance(&mut self, stage: ScheduledForecastStage) {
        self.index = 0;
        self.stage = stage;
    }
    fn poll(
        &mut self,
        budget: usize,
    ) -> Result<
        super::step_job::LivingLifecycleProgressV2,
        super::step_job::LivingLifecycleJobErrorV2,
    > {
        use super::step_job::{
            LivingLifecycleJobErrorV2 as Error, LivingLifecycleProgressV2 as Progress,
        };
        use ScheduledForecastStage::*;
        self.last_poll_visits = 0;
        if self.cancelled {
            return Err(Error::Cancelled);
        }
        if budget == 0 || budget > 1024 {
            return Err(Error::Budget);
        }
        if self.production_preparation_capacity {
            return Err(Error::PreparationCapacity);
        }
        if let Some(fault) = self.production_fault {
            return Err(Error::Fault(fault.public()));
        }
        if let Some(error) = &self.fault {
            return Err(error.clone());
        }
        while self.last_poll_visits < budget && (self.stage != Ready || !self.retirement.is_empty())
        {
            if self.retirement.visit() {
                self.last_poll_visits += 1;
                self.total_visits += 1;
                continue;
            }
            let remaining = budget - self.last_poll_visits;
            let child = match self.stage {
                Preparation => {
                    let job = self.preparation.as_mut().ok_or(Error::NotReady)?;
                    Some((job.poll(remaining), job.last_poll_visits))
                }
                Scheduled => {
                    let job = self.scheduled.as_mut().ok_or(Error::NotReady)?;
                    Some((job.poll(remaining), job.last_poll_visits()))
                }
                Projection => {
                    let job = self.projection.as_mut().ok_or(Error::NotReady)?;
                    Some((
                        job.poll(&self.state, &self.catalog, remaining),
                        job.last_poll_visits(),
                    ))
                }
                Production => {
                    let job = self.production.as_mut().ok_or(Error::NotReady)?;
                    Some((job.poll(remaining), job.last_poll_visits()))
                }
                Dispatch => {
                    let job = self.dispatch.as_mut().ok_or(Error::NotReady)?;
                    Some((job.poll(remaining), job.last_poll_visits()))
                }
                _ => None,
            };
            let result = if let Some((result, visits)) = child {
                self.last_poll_visits += visits;
                self.total_visits += visits as u64;
                match result {
                    Ok(Progress::Pending) => return Ok(Progress::Pending),
                    Err(error) => Err(error),
                    Ok(Progress::Ready) => {
                        if self.last_poll_visits == budget {
                            break;
                        }
                        self.last_poll_visits += 1;
                        self.total_visits += 1;
                        self.finish_child()
                    }
                }
            } else {
                self.last_poll_visits += 1;
                self.total_visits += 1;
                self.visit().map_err(Error::Fault)
            };
            if let Err(error) = result {
                if self.stage == Production {
                    return Err(match error {
                        Error::PreparationCapacity => {
                            self.production_preparation_capacity = true;
                            Error::PreparationCapacity
                        }
                        error => {
                            let cause = match error {
                                Error::Fault(fault) => {
                                    super::step_job::ProductionFailure::capture(&fault)
                                }
                                _ => super::step_job::ProductionFailure::Invariant,
                            };
                            self.production_fault = Some(cause);
                            Error::Fault(cause.public())
                        }
                    });
                }
                self.fault = Some(error.clone());
                return Err(error);
            }
        }
        Ok(if self.stage == Ready && self.retirement.is_empty() {
            Progress::Ready
        } else {
            Progress::Pending
        })
    }
    fn finish_child(&mut self) -> Result<(), super::step_job::LivingLifecycleJobErrorV2> {
        use super::step_job::LivingLifecycleJobErrorV2 as Error;
        use ScheduledForecastStage::*;
        match self.stage {
            Preparation => {
                let mut output = self
                    .preparation
                    .as_mut()
                    .ok_or(Error::NotReady)?
                    .take_ready_result()?;
                self.state = output.state;
                self.idle_input = output.idle_facilities.into_iter();
                // self.source keeps this shared source owner alive.
                self.retirement.start((
                    self.preparation.take().expect("ready preparation"),
                    output.retired_source.0.take(),
                ));
                self.advance(Idle);
            }
            Scheduled => {
                let (state, events, arrivals) = self
                    .scheduled
                    .as_mut()
                    .ok_or(Error::NotReady)?
                    .take_ready_result()?;
                self.retirement
                    .start((self.scheduled.take().expect("ready scheduled"), arrivals));
                self.state = state;
                self.events = events;
                self.fleet_index = 0;
                self.hull_index = 0;
                self.projection = Some(super::step_job::RepairEligibilityProjection::begin(
                    self.tick,
                ));
                self.advance(Projection);
            }
            Projection => {
                self.repair_exclusions = self
                    .projection
                    .as_mut()
                    .ok_or(Error::NotReady)?
                    .take_ready_result()?;
                self.retirement.start(self.projection.take());
                self.advance(RepairWorlds);
            }
            Production => {
                let (state, events) = self
                    .production
                    .as_mut()
                    .ok_or(Error::NotReady)?
                    .take_ready_result()?;
                self.retirement
                    .start(self.production.take().expect("ready phase"));
                self.state = state;
                self.events = events;
                self.advance(Worlds);
            }
            Dispatch => {
                let (state, events) = self
                    .dispatch
                    .as_mut()
                    .ok_or(Error::NotReady)?
                    .take_ready_result()?;
                self.retirement
                    .start(self.dispatch.take().expect("ready phase"));
                self.state = state;
                self.events = events;
                self.state.tick = self.tick;
                self.advance(Inventory);
            }
            _ => return Err(Error::NotReady),
        }
        Ok(())
    }
    fn deficit(&mut self, world: LivingEntityIdV2, resource: LivingResourceV2) {
        if self.deficit_set.insert((world, resource)) {
            self.deficits.push((world, resource));
        }
    }
    fn visit(&mut self) -> Result<(), LivingDeterministicFaultV2> {
        use ScheduledForecastStage::*;
        match self.stage {
            Preparation | Scheduled | Projection | Production | Dispatch | Ready => {
                return Err(LivingDeterministicFaultV2::Invariant);
            }
            Idle => {
                if let Some(id) = self.idle_input.next() {
                    self.idle.insert(id);
                } else {
                    self.advance(Systems);
                }
            }
            Systems => {
                if let Some(row) = self.state.systems.as_slice().get(self.index) {
                    self.graph.insert(row.id, Vec::new());
                    self.index += 1;
                } else {
                    self.advance(Lanes);
                }
            }
            Lanes => {
                if let Some(row) = self.state.lanes.as_slice().get(self.index) {
                    self.lanes.insert(row.id, (row.system_a, row.system_b));
                    if let Some(edges) = self.graph.get_mut(&row.system_a) {
                        edges.push((row.system_b, leg_ticks(row.distance_units)));
                    }
                    if let Some(edges) = self.graph.get_mut(&row.system_b) {
                        edges.push((row.system_a, leg_ticks(row.distance_units)));
                    }
                    self.index += 1;
                } else {
                    self.advance(HazardDefinitions);
                }
            }
            HazardDefinitions => {
                if let Some(row) = self.catalog.hazards().get(self.index) {
                    self.hazard_effects
                        .insert(row.id.as_str().to_owned(), row.effect.clone());
                    self.index += 1;
                } else {
                    self.advance(Start);
                }
            }
            Start => {
                self.offset += 1;
                self.tick = add_ticks(self.start, self.offset)?;
                self.retirement.start(std::mem::replace(
                    &mut self.events,
                    super::receipt::LivingPendingEventsV2::for_forecast(),
                ));
                self.advance(BeginScheduled);
            }
            BeginScheduled => {
                self.scheduled = Some(Box::new(
                    super::step_job::LivingScheduledPhasesJobV2::begin(
                        std::mem::take(&mut self.state),
                        super::receipt::LivingPendingEventsV2::for_forecast(),
                        self.tick,
                    ),
                ));
                self.advance(Scheduled);
            }
            RepairWorlds => {
                if let Some(row) = self.state.worlds.as_slice().get(self.index) {
                    self.worlds
                        .insert(row.id, (row.owner, row.system, row.inventory));
                    self.index += 1;
                } else {
                    self.advance(RepairFleets);
                }
            }
            RepairFleets => {
                if let Some(fleet) = self.state.fleets.as_slice().get(self.fleet_index) {
                    if fleet.owner == self.actor
                        && matches!(fleet.location, LivingFleetLocationV2::Docked { world } if self.worlds.get(&world).is_some_and(|row| row.0 == Some(self.actor)) && !self.repair_exclusions.contains(&world))
                    {
                        self.hull_index = 0;
                        self.advance(RepairHulls);
                    } else {
                        self.fleet_index += 1;
                    }
                } else {
                    self.advance(RepairBatteries);
                }
            }
            RepairHulls => {
                let fleet = &self.state.fleets.as_slice()[self.fleet_index];
                if let Some(hull) = fleet.hulls.as_slice().get(self.hull_index) {
                    if hull.hit_points > 0
                        && hull.next_repair.is_some_and(|due| due <= self.tick)
                        && hull.hit_points
                            < super::step_job::repair_hull_definition(
                                &self.catalog,
                                hull.definition.as_str(),
                            )
                            .map_or(1, |d| d.hit_points)
                        && let LivingFleetLocationV2::Docked { world } = fleet.location
                    {
                        self.repairs.insert(hull.id, (world, hull.hit_points));
                    }
                    self.hull_index += 1;
                } else {
                    self.fleet_index += 1;
                    self.advance(RepairFleets);
                }
            }
            RepairBatteries => {
                if let Some(row) = self.state.facilities.as_slice().get(self.index) {
                    if row.hit_points > 0
                        && row.next_repair.is_some_and(|due| due <= self.tick)
                        && self
                            .worlds
                            .get(&row.world)
                            .is_some_and(|row| row.0 == Some(self.actor))
                        && !self.repair_exclusions.contains(&row.world)
                        && let Some(d) = super::step_job::repair_industry_definition(
                            &self.catalog,
                            row.definition.as_str(),
                        )
                        && let LivingIndustryEffectV2::Defense { hit_points, .. } = d.effect
                        && row.hit_points < hit_points
                    {
                        self.repairs.insert(row.id, (row.world, row.hit_points));
                    }
                    self.index += 1;
                } else {
                    self.advance(DueFacilities);
                }
            }
            DueFacilities => {
                if let Some(row) = self.state.facilities.as_slice().get(self.index) {
                    if row.next_due.is_some_and(|due| due <= self.tick) {
                        self.due.insert(row.id);
                    }
                    self.index += 1;
                } else {
                    use super::step_job::{LivingPhaseCatalogV2, LivingProductionJobV2};
                    self.production = Some(Box::new(
                        match &*self.catalog {
                            LivingPhaseCatalogV2::Borrowed(catalog) => {
                                LivingProductionJobV2::begin(
                                    catalog,
                                    std::mem::take(&mut self.state),
                                    std::mem::take(&mut self.events),
                                    self.tick,
                                    self.seed,
                                    self.branch,
                                )
                            }
                            LivingPhaseCatalogV2::Owned(catalog) => {
                                LivingProductionJobV2::begin_owned(
                                    std::sync::Arc::clone(catalog),
                                    std::mem::take(&mut self.state),
                                    std::mem::take(&mut self.events),
                                    self.tick,
                                    self.seed,
                                    self.branch,
                                )
                            }
                        }
                        .with_repair_exclusions(std::mem::take(&mut self.repair_exclusions)),
                    ));
                    self.advance(Production);
                }
            }
            Worlds => {
                if let Some(row) = self.state.worlds.as_slice().get(self.index) {
                    self.worlds
                        .insert(row.id, (row.owner, row.system, row.inventory));
                    self.index += 1;
                } else {
                    self.fleet_index = 0;
                    self.advance(FleetIndex);
                }
            }
            FleetIndex => {
                if self.state.fleets.as_slice().get(self.fleet_index).is_some() {
                    self.hull_index = 0;
                    self.advance(HullIndex);
                } else {
                    self.advance(FacilityIndex);
                }
            }
            HullIndex => {
                let fleet = &self.state.fleets.as_slice()[self.fleet_index];
                if let Some(hull) = fleet.hulls.as_slice().get(self.hull_index) {
                    self.hulls
                        .insert(hull.id, (hull.hit_points, hull.next_repair.is_some()));
                    self.hull_index += 1;
                } else {
                    self.fleet_index += 1;
                    self.advance(FleetIndex);
                }
            }
            FacilityIndex => {
                if let Some(row) = self.state.facilities.as_slice().get(self.index) {
                    self.hulls
                        .insert(row.id, (row.hit_points, row.next_repair.is_some()));
                    self.index += 1;
                } else {
                    self.advance(Repairs);
                }
            }
            Repairs => {
                if let Some((id, (world, hp))) = self.repairs.pop_first() {
                    if self.hulls.get(&id) == Some(&(hp, true)) {
                        self.deficit(world, LivingResourceV2::Energy);
                    }
                } else {
                    self.retirement.start(std::mem::take(&mut self.repairs));
                    self.advance(Facilities);
                }
            }
            Facilities => {
                if let Some(row) = self.state.facilities.as_slice().get(self.index) {
                    if self.due.contains(&row.id) {
                        if row.status == LivingFacilityStatusV2::Operational {
                            self.idle.remove(&row.id);
                        }
                        if self
                            .worlds
                            .get(&row.world)
                            .is_some_and(|meta| meta.0 == Some(self.actor))
                        {
                            self.reason_index = 0;
                            self.stage = Reasons;
                        } else {
                            self.index += 1;
                        }
                    } else {
                        self.index += 1;
                    }
                } else {
                    self.advance(Agreements);
                }
            }
            Reasons => {
                let row = &self.state.facilities.as_slice()[self.index];
                if let Some(reason) = row
                    .blocked_reasons
                    .as_slice()
                    .get(self.reason_index)
                    .copied()
                {
                    let world = row.world;
                    if let LivingBlockedReasonV2::MissingInput { resource, .. } = reason {
                        self.deficit(world, resource);
                    }
                    self.reason_index += 1;
                } else {
                    self.index += 1;
                    self.stage = Facilities;
                }
            }
            Agreements => {
                if let Some(row) = self.state.agreements.as_slice().get(self.index) {
                    if row.kind == LivingAgreementKindV2::Trade
                        && active_interval(row.start_tick, row.end_tick, self.tick)
                    {
                        self.trade.insert((row.participant_a, row.participant_b));
                    }
                    self.index += 1;
                } else {
                    self.advance(Routes);
                }
            }
            Routes => {
                if let Some(row) = self.state.routes.as_slice().get(self.index) {
                    let from = self.worlds.get(&row.source_world);
                    let to = self.worlds.get(&row.destination_world);
                    let allowed = row.source_owner == self.actor
                        && row.next_due <= self.tick
                        && !row.suspended
                        && from.is_some_and(|meta| meta.0 == Some(self.actor))
                        && to.is_some_and(|meta| meta.0 == Some(row.receiver_owner))
                        && (row.kind != LivingRouteKindV2::Trade
                            || self
                                .trade
                                .contains(&ordered_pair(row.source_owner, row.receiver_owner)));
                    if allowed {
                        self.search = Some(super::step_job::CostSearch::begin(
                            from.map(|meta| meta.1),
                            to.map(|meta| meta.1),
                            &self.graph,
                        ));
                        self.stage = Travel;
                    } else {
                        self.index += 1;
                    }
                } else {
                    use super::step_job::{LivingDispatchJobV2, LivingPhaseCatalogV2};
                    self.dispatch = Some(Box::new(match &*self.catalog {
                        LivingPhaseCatalogV2::Borrowed(catalog) => LivingDispatchJobV2::begin(
                            catalog,
                            std::mem::take(&mut self.state),
                            std::mem::take(&mut self.events),
                            self.tick,
                            self.seed,
                            self.branch,
                        ),
                        LivingPhaseCatalogV2::Owned(catalog) => LivingDispatchJobV2::begin_owned(
                            std::sync::Arc::clone(catalog),
                            std::mem::take(&mut self.state),
                            std::mem::take(&mut self.events),
                            self.tick,
                            self.seed,
                            self.branch,
                        ),
                    }));
                    self.advance(Dispatch);
                }
            }
            Travel => {
                let search = self
                    .search
                    .as_mut()
                    .ok_or(LivingDeterministicFaultV2::Invariant)?;
                if let Some(result) = search.result {
                    self.route_eligible = result.is_some();
                    self.stage = RetireSearch;
                } else {
                    search.visit(&self.graph);
                }
            }
            RetireSearch => {
                let search = self
                    .search
                    .as_mut()
                    .ok_or(LivingDeterministicFaultV2::Invariant)?;
                if !search.retire_record() {
                    self.retirement
                        .start(self.search.take().expect("retained search"));
                    if self.route_eligible {
                        self.batch = self.state.routes.as_slice()[self.index].batch_size;
                        self.reason_index = 0;
                        self.stage = Hazards;
                    } else {
                        self.index += 1;
                        self.stage = Routes;
                    }
                }
            }
            Hazards => {
                let route = &self.state.routes.as_slice()[self.index];
                let from = self.worlds.get(&route.source_world).map(|meta| meta.1);
                let to = self.worlds.get(&route.destination_world).map(|meta| meta.1);
                if let Some(row) = self.state.hazards.as_slice().get(self.reason_index) {
                    if from != to
                        && active_interval(row.start_tick, row.end_tick, self.tick)
                        && self.lanes.get(&row.lane).is_some_and(|&(a, b)| {
                            (Some(a) == from && Some(b) == to) || (Some(b) == from && Some(a) == to)
                        })
                        && let Some(effect) = self.hazard_effects.get(row.definition.as_str())
                    {
                        self.batch = self.batch.min(effect.scale_batch(route.batch_size));
                    }
                    self.reason_index += 1;
                } else {
                    let stock = self
                        .worlds
                        .get(&route.source_world)
                        .map(|meta| meta.2.quantity(route.resource))
                        .unwrap_or(0);
                    if self.batch > 0 && stock < route.source_reserve.saturating_add(self.batch) {
                        let world = route.source_world;
                        let resource = route.resource;
                        self.deficit(world, resource);
                    }
                    self.index += 1;
                    self.stage = Routes;
                }
            }
            Inventory => {
                if let Some(row) = self.state.worlds.as_slice().get(self.index) {
                    if row.owner == Some(self.actor) {
                        self.inventories.push((self.tick, row.id, row.inventory));
                    }
                    self.index += 1;
                } else {
                    self.advance(ClearBoundary);
                }
            }
            ClearBoundary => {
                if self.worlds.pop_first().is_some()
                    || self.hulls.pop_first().is_some()
                    || self.due.pop_first().is_some()
                    || self.trade.pop_first().is_some()
                {
                } else if self.offset == 200 {
                    self.advance(IdleOutput);
                } else {
                    self.advance(Start);
                }
            }
            IdleOutput => {
                if let Some(id) = self.idle.pop_first() {
                    self.idle_output.push(id);
                } else {
                    self.advance(RetireGraph);
                }
            }
            RetireGraph => {
                if let Some((_, edges)) = self.graph.pop_first() {
                    self.retired_edges = edges;
                    self.stage = RetireEdges;
                } else {
                    self.advance(RetireLanes);
                }
            }
            RetireEdges => {
                if self.retired_edges.pop().is_none() {
                    self.retirement
                        .start(std::mem::take(&mut self.retired_edges));
                    self.stage = RetireGraph;
                }
            }
            RetireLanes => {
                if self.lanes.pop_first().is_none() {
                    self.advance(RetireHazards);
                }
            }
            RetireHazards => {
                if self.hazard_effects.pop_first().is_none() {
                    self.advance(RetireDeficits);
                }
            }
            RetireDeficits => {
                if self.deficit_set.pop_first().is_none() {
                    self.advance(Ready);
                }
            }
        }
        Ok(())
    }
    // Explicit synchronous compatibility adapter; residual owners drop only here.
    fn finish(
        mut self,
    ) -> Result<
        (
            LivingScheduledForecastV2,
            std::sync::Arc<LivingGalaxyStateV2>,
        ),
        super::step_job::LivingLifecycleJobErrorV2,
    > {
        let result = self.take_ready_result();
        let mut retirement =
            self.into_rules_retirement(LivingStateRetirementPolicyV2::ReleaseHandle);
        while retirement.poll(1024).expect("valid retirement budget")
            != super::step_job::LivingLifecycleProgressV2::Ready
        {}
        result
    }
    fn take_ready_result(
        &mut self,
    ) -> Result<
        (
            LivingScheduledForecastV2,
            std::sync::Arc<LivingGalaxyStateV2>,
        ),
        super::step_job::LivingLifecycleJobErrorV2,
    > {
        use super::step_job::LivingLifecycleJobErrorV2 as Error;
        if self.cancelled {
            return Err(Error::Cancelled);
        }
        if self.production_preparation_capacity {
            return Err(Error::PreparationCapacity);
        }
        if let Some(fault) = self.production_fault {
            return Err(Error::Fault(fault.public()));
        }
        if let Some(error) = &self.fault {
            return Err(error.clone());
        }
        if self.stage != ScheduledForecastStage::Ready
            || self.delivered
            || !self.retirement.is_empty()
        {
            return Err(Error::NotReady);
        }
        self.delivered = true;
        Ok((
            LivingScheduledForecastV2 {
                inventories: std::mem::take(&mut self.inventories),
                deficits: std::mem::take(&mut self.deficits),
                idle_facilities: std::mem::take(&mut self.idle_output),
                final_state: std::mem::take(&mut self.state),
            },
            std::sync::Arc::clone(&self.source),
        ))
    }
}

impl LivingScheduledForecastJobV2<'static> {
    /// Move catalog and source owners without scanning or cloning their records.
    fn begin_owned(
        catalog: std::sync::Arc<ValidatedLivingCatalogPackV2>,
        source: std::sync::Arc<LivingGalaxyStateV2>,
        actor: LivingEntityIdV2,
        tick: LivingTickV2,
        seed: [u8; 32],
        branch: super::ids::LivingBranchIdV2,
    ) -> Self {
        Self::with_catalog(
            super::step_job::LivingPhaseCatalogV2::Owned(catalog),
            source,
            actor,
            tick,
            seed,
            branch,
        )
    }
}

fn forecast_scheduled_economy_with_explanations(
    ctx: &LivingStepContextV2<'_>,
    actor: LivingEntityIdV2,
    explanations: bool,
) -> Result<LivingScheduledForecastV2, LivingDeterministicFaultV2> {
    #[cfg(test)]
    if explanations {
        return reference_scheduled_forecast(ctx, actor, true);
    }
    let _ = explanations;
    let mut job = LivingScheduledForecastJobV2::begin(
        ctx.catalog,
        std::sync::Arc::new(ctx.state.clone()),
        actor,
        ctx.tick,
        ctx.genesis_seed,
        ctx.branch,
    );
    loop {
        match job.poll(1024) {
            Ok(super::step_job::LivingLifecycleProgressV2::Pending) => {}
            Ok(super::step_job::LivingLifecycleProgressV2::Ready) => break,
            Err(super::step_job::LivingLifecycleJobErrorV2::PreparationCapacity) => {
                panic!("Living scheduled synchronous preparation capacity exceeded")
            }
            Err(super::step_job::LivingLifecycleJobErrorV2::Fault(fault)) => return Err(fault),
            Err(_) => return Err(LivingDeterministicFaultV2::Invariant),
        }
    }
    let (output, _retired_source) = job.finish().map_err(|error| match error {
        super::step_job::LivingLifecycleJobErrorV2::PreparationCapacity => {
            panic!("Living scheduled synchronous preparation capacity exceeded")
        }
        super::step_job::LivingLifecycleJobErrorV2::Fault(fault) => fault,
        _ => LivingDeterministicFaultV2::Invariant,
    })?;
    debug_assert_eq!(output.final_state.tick.0, ctx.tick.0 + 200);
    Ok(output)
}

// Private decision-input composition; later intent slices consume this output.
struct LivingIntentInputsV2 {
    snapshot: RulesOwner<std::sync::Arc<LivingGalaxyStateV2>>,
    digest: super::ids::LivingStateDigestV2,
    forecasts: Vec<(LivingEntityIdV2, LivingScheduledForecastV2)>,
    requests: Vec<LivingReceiverDemandRequestV2>,
    // Keep the normalized copy alive for the parent's bounded retirement queue.
    hash_owner: RulesOwner<std::sync::Arc<LivingGalaxyStateV2>>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum IntentInputsStage {
    Copy,
    Hash,
    BeginHash,
    Actors,
    Forecast,
    Colonies,
    Worlds,
    Requests,
    RetireOwners,
    RetireStocks,
    Ready,
}

struct LivingIntentInputsJobV2 {
    retirement: super::step_job::PhaseRetirementLane<'static>,
    catalog: RulesOwner<std::sync::Arc<ValidatedLivingCatalogPackV2>>,
    source: RulesOwner<std::sync::Arc<LivingGalaxyStateV2>>,
    tick: LivingTickV2,
    seed: [u8; 32],
    branch: super::ids::LivingBranchIdV2,
    copy: Option<super::step_job::LivingStateCopyJobV2>,
    hash: Option<super::step_job::LivingStateHashJobV2>,
    hash_owner: Option<std::sync::Arc<LivingGalaxyStateV2>>,
    digest: Option<super::ids::LivingStateDigestV2>,
    child: Option<Box<LivingScheduledForecastJobV2<'static>>>,
    forecasts: Vec<(LivingEntityIdV2, LivingScheduledForecastV2)>,
    requests: Vec<LivingReceiverDemandRequestV2>,
    owners: std::collections::BTreeSet<(LivingEntityIdV2, LivingEntityIdV2)>,
    stocks: std::collections::BTreeMap<LivingEntityIdV2, LivingInventoryV2>,
    stage: IntentInputsStage,
    index: usize,
    actor_index: usize,
    cancelled: bool,
    delivered: bool,
    fault: Option<super::step_job::LivingLifecycleJobErrorV2>,
    last_poll_visits: usize,
    total_visits: u64,
}

impl LivingIntentInputsJobV2 {
    fn begin(
        catalog: std::sync::Arc<ValidatedLivingCatalogPackV2>,
        source: std::sync::Arc<LivingGalaxyStateV2>,
        tick: LivingTickV2,
        seed: [u8; 32],
        branch: super::ids::LivingBranchIdV2,
    ) -> Self {
        let fault = (tick.0 == 0 || source.tick.0.checked_add(1) != Some(tick.0)).then_some(
            super::step_job::LivingLifecycleJobErrorV2::Fault(
                LivingDeterministicFaultV2::Decision(
                    super::decision::LivingDecisionErrorV2::Snapshot,
                ),
            ),
        );
        Self {
            retirement: Default::default(),
            catalog: catalog.into(),
            copy: Some(super::step_job::LivingStateCopyJobV2::begin(
                std::sync::Arc::clone(&source),
            )),
            source: source.into(),
            tick,
            seed,
            branch,
            hash: None,
            hash_owner: None,
            digest: None,
            child: None,
            forecasts: Vec::new(),
            requests: Vec::new(),
            owners: Default::default(),
            stocks: Default::default(),
            stage: IntentInputsStage::Copy,
            index: 0,
            actor_index: 0,
            cancelled: false,
            delivered: false,
            fault,
            last_poll_visits: 0,
            total_visits: 0,
        }
    }

    #[cfg(test)]
    fn cancel(&mut self) {
        // Retain partial copy/hash/forecast owners. The parent retires this job.
        self.cancelled = true;
    }

    fn advance(&mut self, stage: IntentInputsStage) {
        self.stage = stage;
        self.index = 0;
    }

    fn poll(
        &mut self,
        budget: usize,
    ) -> Result<
        super::step_job::LivingLifecycleProgressV2,
        super::step_job::LivingLifecycleJobErrorV2,
    > {
        use super::step_job::{
            LivingLifecycleJobErrorV2 as Error, LivingLifecycleProgressV2 as Progress,
        };
        use IntentInputsStage::*;
        self.last_poll_visits = 0;
        if self.cancelled {
            return Err(Error::Cancelled);
        }
        if budget == 0 || budget > 1024 {
            return Err(Error::Budget);
        }
        if let Some(error) = &self.fault {
            return Err(error.clone());
        }
        while self.last_poll_visits < budget && (self.stage != Ready || !self.retirement.is_empty())
        {
            if self.retirement.visit() {
                self.last_poll_visits += 1;
                self.total_visits += 1;
                continue;
            }
            let remaining = budget - self.last_poll_visits;
            let child = match self.stage {
                Copy => {
                    let job = self.copy.as_mut().ok_or(Error::NotReady)?;
                    Some((job.poll(remaining), job.last_poll_visits()))
                }
                Hash => {
                    let job = self.hash.as_mut().ok_or(Error::NotReady)?;
                    Some((job.poll(remaining), job.last_poll_visits()))
                }
                Forecast => {
                    let job = self.child.as_mut().ok_or(Error::NotReady)?;
                    Some((job.poll(remaining), job.last_poll_visits))
                }
                _ => None,
            };
            let result = if let Some((result, visits)) = child {
                self.last_poll_visits += visits;
                self.total_visits += visits as u64;
                match result {
                    Ok(Progress::Pending) => return Ok(Progress::Pending),
                    Err(error) => Err(error),
                    Ok(Progress::Ready) => {
                        if self.last_poll_visits == budget {
                            break;
                        }
                        self.last_poll_visits += 1;
                        self.total_visits += 1;
                        self.finish_child()
                    }
                }
            } else {
                self.last_poll_visits += 1;
                self.total_visits += 1;
                self.visit();
                Ok(())
            };
            if let Err(error) = result {
                self.fault = Some(error.clone());
                return Err(error);
            }
        }
        Ok(if self.stage == Ready && self.retirement.is_empty() {
            Progress::Ready
        } else {
            Progress::Pending
        })
    }

    fn finish_child(&mut self) -> Result<(), super::step_job::LivingLifecycleJobErrorV2> {
        use super::step_job::LivingLifecycleJobErrorV2 as Error;
        match self.stage {
            IntentInputsStage::Copy => {
                // finish is called only after Ready; self.source retains its source.
                let mut normalized = self
                    .copy
                    .as_mut()
                    .ok_or(Error::NotReady)?
                    .take_ready_result()?;
                self.retirement.start(self.copy.take().expect("ready copy"));
                normalized.tick = self.tick;
                let owner = std::sync::Arc::new(normalized);
                self.hash_owner = Some(owner);
                self.advance(IntentInputsStage::BeginHash);
            }
            IntentInputsStage::Hash => {
                // hash_owner retains the last large owner after consuming the hash.
                self.digest = Some(
                    self.hash
                        .as_mut()
                        .ok_or(Error::NotReady)?
                        .take_ready_result()?,
                );
                self.retirement.start(self.hash.take().expect("ready hash"));
                self.advance(IntentInputsStage::Actors);
            }
            IntentInputsStage::Forecast => {
                let actor = self.source.civilizations.as_slice()[self.index - 1].id;
                let (forecast, source) = self
                    .child
                    .as_mut()
                    .ok_or(Error::NotReady)?
                    .take_ready_result()?;
                self.forecasts.push((actor, forecast));
                // Original immutable source is retained by this parent.
                self.retirement
                    .start((self.child.take().expect("ready forecast"), source));
                self.stage = IntentInputsStage::Actors;
            }
            _ => return Err(Error::NotReady),
        }
        Ok(())
    }

    fn visit(&mut self) {
        use IntentInputsStage::*;
        match self.stage {
            BeginHash => {
                self.hash = Some(super::step_job::LivingStateHashJobV2::begin(
                    std::sync::Arc::clone(self.hash_owner.as_ref().expect("normalized owner")),
                ));
                self.advance(Hash);
            }
            Actors => {
                if let Some(actor) = self.source.civilizations.as_slice().get(self.index) {
                    self.index += 1;
                    if actor.status == LivingCivilizationStatusV2::Active {
                        self.child = Some(Box::new(LivingScheduledForecastJobV2::begin_owned(
                            std::sync::Arc::clone(&self.catalog),
                            std::sync::Arc::clone(&self.source),
                            actor.id,
                            self.tick,
                            self.seed,
                            self.branch,
                        )));
                        self.stage = Forecast;
                    }
                } else {
                    self.advance(Colonies);
                }
            }
            Colonies => {
                if let Some(colony) = self.source.colonies.as_slice().get(self.index) {
                    self.owners.insert((colony.world, colony.owner));
                    self.index += 1;
                } else {
                    self.advance(Worlds);
                }
            }
            Worlds => {
                if let Some(world) = self.source.worlds.as_slice().get(self.index) {
                    self.stocks.insert(world.id, world.inventory);
                    self.index += 1;
                } else {
                    self.advance(Requests);
                }
            }
            Requests => {
                if let Some((receiver, forecast)) = self.forecasts.get(self.actor_index) {
                    if let Some((world, resource)) = forecast.deficits.get(self.index) {
                        let stock = self
                            .stocks
                            .get(world)
                            .map(|stock| stock.quantity(*resource))
                            .unwrap_or(0);
                        if self.owners.contains(&(*world, *receiver)) && stock < 20 {
                            self.requests.push(LivingReceiverDemandRequestV2 {
                                receiver: *receiver,
                                world: *world,
                                resource: *resource,
                                requested_units: 20 - stock,
                            });
                        }
                        self.index += 1;
                    } else {
                        self.actor_index += 1;
                        self.index = 0;
                    }
                } else {
                    self.advance(RetireOwners);
                }
            }
            RetireOwners => {
                if self.owners.pop_first().is_none() {
                    self.advance(RetireStocks);
                }
            }
            RetireStocks => {
                if self.stocks.pop_first().is_none() {
                    self.advance(Ready);
                }
            }
            Copy | Hash | Forecast | Ready => unreachable!("child stages are polled separately"),
        }
    }

    // Borrowing on error preserves all partial owners for explicit retirement.
    fn finish(
        &mut self,
    ) -> Result<LivingIntentInputsV2, super::step_job::LivingLifecycleJobErrorV2> {
        use super::step_job::LivingLifecycleJobErrorV2 as Error;
        if self.cancelled {
            return Err(Error::Cancelled);
        }
        if let Some(error) = &self.fault {
            return Err(error.clone());
        }
        if self.stage != IntentInputsStage::Ready
            || self.delivered
            || !self.retirement.is_empty()
            || self.digest.is_none()
            || self.hash_owner.is_none()
        {
            return Err(Error::NotReady);
        }
        let digest = self.digest.take().ok_or(Error::NotReady)?;
        let hash_owner = self.hash_owner.take().ok_or(Error::NotReady)?;
        self.delivered = true;
        Ok(LivingIntentInputsV2 {
            snapshot: std::sync::Arc::clone(&self.source).into(),
            digest,
            forecasts: std::mem::take(&mut self.forecasts),
            requests: std::mem::take(&mut self.requests),
            hash_owner: hash_owner.into(),
        })
    }
}

// Private bounded enumeration. The reference above remains the parity oracle.
struct LivingEconomicBatchV2 {
    actor: LivingEntityIdV2,
    economic_count: usize,
    ranked: Vec<(u16, LivingEconomicCandidateV2)>,
    explanations: Vec<LivingIntentDecisionV2>,
}
struct LivingEconomicEnumerationV2 {
    catalog: RulesOwner<std::sync::Arc<ValidatedLivingCatalogPackV2>>,
    inputs: RulesOwner<LivingIntentInputsV2>,
    // The caller must eventually retire both graph vectors per record.
    graph: RulesOwner<std::sync::Arc<super::step_job::RetreatGraph>>,
    batches: Vec<LivingEconomicBatchV2>,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum EconomicEnumerationStage {
    Worlds,
    Systems,
    Lanes,
    Industries,
    HullCosts,
    Wars,
    Agreements,
    Observations,
    ForecastStocks,
    ForecastDeficits,
    ForecastIdle,
    Civilizations,
    FleetCounts,
    PaidHullCounts,
    Actors,
    ColonyCount,
    Colonies,
    Facilities,
    ConstructionJobs,
    ColonyHullJobs,
    Threats,
    Replacement,
    Buildings,
    Hulls,
    ArkTargets,
    Path,
    RetirePath,
    Resources,
    Sources,
    Routes,
    Requests,
    AdjacentColonies,
    AdjacentLanes,
    ForeignSources,
    Canonical,
    Ranking,
    RetireFacilities,
    RetireWorlds,
    RetireIndustries,
    RetireHulls,
    RetireWars,
    RetireTrade,
    RetireObservations,
    RetireStocks,
    RetireDeficits,
    RetireIdle,
    RetireActors,
    RetireCounts,
    RetireEscorts,
    RetireGraphLanes,
    RetireGraphSystems,
    Ready,
}
type EconomicCanonicalKey = (u8, LivingEntityIdV2, LivingEntityIdV2, &'static str, usize);
type EconomicRankingKey = (
    u8,
    std::cmp::Reverse<u16>,
    LivingEntityIdV2,
    u8,
    &'static str,
    u16,
);
struct LivingEconomicEnumerationJobV2 {
    retirement: super::step_job::PhaseRetirementLane<'static>,
    catalog: RulesOwner<std::sync::Arc<ValidatedLivingCatalogPackV2>>,
    inputs: Option<LivingIntentInputsV2>,
    tick: LivingTickV2,
    graph: RulesOwner<std::sync::Arc<super::step_job::RetreatGraph>>,
    worlds: std::collections::BTreeMap<LivingEntityIdV2, usize>,
    industries: std::collections::BTreeMap<String, u32>,
    hull_costs: std::collections::BTreeMap<String, u32>,
    wars: std::collections::BTreeSet<(LivingEntityIdV2, LivingEntityIdV2)>,
    trade: std::collections::BTreeSet<(LivingEntityIdV2, LivingEntityIdV2)>,
    observations: std::collections::BTreeMap<(LivingEntityIdV2, LivingEntityIdV2), LivingTickV2>,
    projected: std::collections::BTreeMap<(LivingEntityIdV2, LivingEntityIdV2), (bool, bool)>,
    deficits: std::collections::BTreeSet<(LivingEntityIdV2, LivingEntityIdV2, LivingResourceV2)>,
    idle: std::collections::BTreeSet<(LivingEntityIdV2, LivingEntityIdV2)>,
    actors: std::collections::BTreeMap<LivingEntityIdV2, usize>,
    counts: std::collections::BTreeMap<LivingEntityIdV2, [usize; 3]>,
    escorts: std::collections::BTreeSet<LivingEntityIdV2>,
    stage: EconomicEnumerationStage,
    index: usize,
    nested: usize,
    forecast_index: usize,
    actor_index: usize,
    colony_index: usize,
    world_index: usize,
    colony_count: usize,
    facilities: Vec<usize>,
    has: [bool; 5],
    enabled: [usize; 5],
    shipyard: bool,
    construction_job: bool,
    hull_job: bool,
    deposit: bool,
    threatened: bool,
    scrap: Option<LivingEntityIdV2>,
    building_free: bool,
    hull_index: usize,
    ark: bool,
    resource_index: usize,
    source_index: usize,
    request_index: usize,
    adjacent: bool,
    adjacent_home: Option<LivingEntityIdV2>,
    supply: Option<LivingEconomicCandidateV2>,
    first_route: Option<LivingEntityIdV2>,
    working_route: bool,
    path: Option<super::step_job::RetreatPathJob>,
    retired_path: Vec<LivingEntityIdV2>,
    path_found: bool,
    path_resume: EconomicEnumerationStage,
    emission: usize,
    canonical: std::collections::BTreeMap<EconomicCanonicalKey, LivingEconomicCandidateV2>,
    ranking: std::collections::BTreeMap<EconomicRankingKey, LivingEconomicCandidateV2>,
    previous: Option<LivingEconomicActionV2>,
    failed_candidate: Option<LivingEconomicCandidateV2>,
    batch: Option<LivingEconomicBatchV2>,
    batches: Vec<LivingEconomicBatchV2>,
    cancelled: bool,
    delivered: bool,
    path_fault: Option<super::step_job::RetreatPathFailure>,
    fault: Option<super::step_job::LivingLifecycleJobErrorV2>,
    last_poll_visits: usize,
    total_visits: u64,
}
impl LivingEconomicEnumerationJobV2 {
    fn begin(
        catalog: std::sync::Arc<ValidatedLivingCatalogPackV2>,
        inputs: LivingIntentInputsV2,
        tick: LivingTickV2,
    ) -> Self {
        let fault = (tick.0 == 0 || inputs.snapshot.tick.0.checked_add(1) != Some(tick.0))
            .then_some(super::step_job::LivingLifecycleJobErrorV2::Fault(
                LivingDeterministicFaultV2::Decision(
                    super::decision::LivingDecisionErrorV2::Snapshot,
                ),
            ));
        Self {
            retirement: Default::default(),
            catalog: catalog.into(),
            inputs: Some(inputs),
            tick,
            graph: Default::default(),
            worlds: Default::default(),
            industries: Default::default(),
            hull_costs: Default::default(),
            wars: Default::default(),
            trade: Default::default(),
            observations: Default::default(),
            projected: Default::default(),
            deficits: Default::default(),
            idle: Default::default(),
            actors: Default::default(),
            counts: Default::default(),
            escorts: Default::default(),
            stage: EconomicEnumerationStage::Worlds,
            index: 0,
            nested: 0,
            forecast_index: 0,
            actor_index: 0,
            colony_index: 0,
            world_index: 0,
            colony_count: 0,
            facilities: Vec::new(),
            has: [false; 5],
            enabled: [0; 5],
            shipyard: false,
            construction_job: false,
            hull_job: false,
            deposit: false,
            threatened: false,
            scrap: None,
            building_free: false,
            hull_index: 0,
            ark: false,
            resource_index: 0,
            source_index: 0,
            request_index: 0,
            adjacent: false,
            adjacent_home: None,
            supply: None,
            first_route: None,
            working_route: false,
            path: None,
            retired_path: Vec::new(),
            path_found: false,
            path_resume: EconomicEnumerationStage::Actors,
            emission: 0,
            canonical: Default::default(),
            ranking: Default::default(),
            previous: None,
            failed_candidate: None,
            batch: None,
            batches: Vec::new(),
            cancelled: false,
            delivered: false,
            path_fault: None,
            fault,
            last_poll_visits: 0,
            total_visits: 0,
        }
    }
    #[cfg(test)]
    fn cancel(&mut self) {
        self.cancelled = true;
    }
    fn advance(&mut self, stage: EconomicEnumerationStage) {
        self.stage = stage;
        self.index = 0;
        self.nested = 0;
    }
    fn actor(&self) -> LivingEntityIdV2 {
        self.inputs.as_ref().expect("retained inputs").forecasts[self.actor_index].0
    }
    fn policy(&self) -> &str {
        let actor = self.actor();
        self.inputs
            .as_ref()
            .expect("retained inputs")
            .snapshot
            .civilizations
            .as_slice()[self.actors[&actor]]
            .policy
            .0
            .as_str()
    }
    fn emit(
        &mut self,
        candidate: LivingEconomicCandidateV2,
    ) -> Result<(), LivingDeterministicFaultV2> {
        let source = match candidate.action {
            LivingEconomicActionV2::Supply { source, .. } => source,
            _ => candidate.target,
        };
        let tie = candidate.tie();
        let emission = self.emission;
        let Some(next_emission) = self.emission.checked_add(1) else {
            self.failed_candidate = Some(candidate);
            return Err(LivingDeterministicFaultV2::Arithmetic);
        };
        self.emission = next_emission;
        self.canonical.insert(
            (tie.0, candidate.target, source, tie.1, emission),
            candidate,
        );
        Ok(())
    }
    fn start_path(
        &mut self,
        from: LivingEntityIdV2,
        to: LivingEntityIdV2,
        resume: EconomicEnumerationStage,
    ) {
        let state = &self.inputs.as_ref().expect("retained inputs").snapshot;
        let origin = self
            .worlds
            .get(&from)
            .map(|i| state.worlds.as_slice()[*i].system);
        let destination = self
            .worlds
            .get(&to)
            .map(|i| state.worlds.as_slice()[*i].system);
        self.path_resume = resume;
        self.path_found = false;
        match (origin, destination) {
            (Some(origin), Some(destination)) if origin == destination => {
                // Local world travel has duration 10 and no lane records.
                self.path_found = true;
                self.stage = resume;
            }
            (Some(origin), Some(destination)) => {
                self.path = Some(super::step_job::RetreatPathJob::begin(
                    std::sync::Arc::clone(&self.graph),
                    origin,
                    destination,
                ));
                self.stage = EconomicEnumerationStage::Path;
            }
            _ => self.stage = resume,
        }
    }
    fn poll(
        &mut self,
        budget: usize,
    ) -> Result<
        super::step_job::LivingLifecycleProgressV2,
        super::step_job::LivingLifecycleJobErrorV2,
    > {
        use super::step_job::{
            LivingLifecycleJobErrorV2 as Error, LivingLifecycleProgressV2 as Progress,
        };
        self.last_poll_visits = 0;
        if let Some(error) = self.path_fault {
            return Err(error.public());
        }
        if self.cancelled {
            return Err(Error::Cancelled);
        }
        if budget == 0 || budget > 1024 {
            return Err(Error::Budget);
        }
        if let Some(error) = &self.fault {
            return Err(error.clone());
        }
        while self.last_poll_visits < budget
            && (self.stage != EconomicEnumerationStage::Ready || !self.retirement.is_empty())
        {
            if self.retirement.visit() {
                self.last_poll_visits += 1;
                self.total_visits += 1;
                continue;
            }

            if self.stage == EconomicEnumerationStage::Path {
                let child = self.path.as_mut().expect("retained path child");
                let progress = child.poll(budget - self.last_poll_visits);
                let visits = child.last_poll_visits();
                self.last_poll_visits += visits;
                self.total_visits += visits as u64;
                match progress {
                    Err(error) => {
                        let failure = super::step_job::RetreatPathFailure::from_public(error);
                        self.path_fault = Some(failure);
                        return Err(failure.public());
                    }
                    Ok(Progress::Pending) => {}
                    Ok(Progress::Ready) if self.last_poll_visits < budget => {
                        self.last_poll_visits += 1;
                        self.total_visits += 1;
                        let result = match self.path.as_mut().unwrap().take_ready_result() {
                            Ok(result) => result,
                            Err(error) => {
                                let failure =
                                    super::step_job::RetreatPathFailure::from_public(error);
                                self.path_fault = Some(failure);
                                return Err(failure.public());
                            }
                        };
                        self.path_found = result.is_some();
                        if let Some((_, lanes)) = result {
                            self.retired_path = lanes;
                        }
                        self.retirement.start((
                            self.path.take().unwrap(),
                            std::mem::take(&mut self.retired_path),
                        ));
                        self.stage = EconomicEnumerationStage::RetirePath;
                    }
                    Ok(Progress::Ready) => {}
                }
                continue;
            }
            match self.visit() {
                Ok(true) => {
                    self.last_poll_visits += 1;
                    self.total_visits += 1;
                }
                Ok(false) => {}
                Err(fault) => {
                    // The faulting record is still an attempted visit.
                    self.last_poll_visits += 1;
                    self.total_visits += 1;
                    let error = Error::Fault(fault);
                    self.fault = Some(error.clone());
                    return Err(error);
                }
            }
        }
        Ok(
            if self.stage == EconomicEnumerationStage::Ready && self.retirement.is_empty() {
                Progress::Ready
            } else {
                Progress::Pending
            },
        )
    }
    // A false result is an empty monotonic transition, never an uncharged scan.
    fn visit(&mut self) -> Result<bool, LivingDeterministicFaultV2> {
        use EconomicEnumerationStage::*;
        let inputs = self.inputs.as_ref().expect("retained inputs");
        let state = inputs.snapshot.as_ref();
        match self.stage {
            Worlds => {
                if let Some(world) = state.worlds.as_slice().get(self.index) {
                    self.worlds.insert(world.id, self.index);
                    self.index += 1;
                    return Ok(true);
                }
                self.advance(Systems);
            }
            Systems => {
                if let Some(row) = state.systems.as_slice().get(self.index) {
                    std::sync::Arc::get_mut(&mut self.graph)
                        .expect("unshared graph")
                        .systems
                        .push(row.id);
                    self.index += 1;
                    return Ok(true);
                }
                self.advance(Lanes);
            }
            Lanes => {
                if let Some(row) = state.lanes.as_slice().get(self.index) {
                    std::sync::Arc::get_mut(&mut self.graph)
                        .expect("unshared graph")
                        .lanes
                        .push((row.id, row.system_a, row.system_b, row.distance_units));
                    self.index += 1;
                    return Ok(true);
                }
                self.advance(Industries);
            }
            Industries => {
                if let Some(row) = self.catalog.industries().get(self.index) {
                    self.industries
                        .insert(row.id.as_str().to_owned(), row.alloy_cost);
                    self.index += 1;
                    return Ok(true);
                }
                self.advance(HullCosts);
            }
            HullCosts => {
                if let Some(row) = self.catalog.hulls().get(self.index) {
                    self.hull_costs
                        .insert(row.id.as_str().to_owned(), row.alloy_cost);
                    self.index += 1;
                    return Ok(true);
                }
                self.advance(Wars);
            }
            Wars => {
                if let Some(row) = state.wars.as_slice().get(self.index) {
                    if active_interval(row.start_tick, row.end_tick, self.tick) {
                        self.wars
                            .insert(ordered_pair(row.participant_a, row.participant_b));
                    }
                    self.index += 1;
                    return Ok(true);
                }
                self.advance(Agreements);
            }
            Agreements => {
                if let Some(row) = state.agreements.as_slice().get(self.index) {
                    if row.kind == LivingAgreementKindV2::Trade
                        && active_interval(row.start_tick, row.end_tick, self.tick)
                    {
                        self.trade
                            .insert(ordered_pair(row.participant_a, row.participant_b));
                    }
                    self.index += 1;
                    return Ok(true);
                }
                self.advance(Observations);
            }
            Observations => {
                if let Some(row) = state.observations.as_slice().get(self.index) {
                    self.observations
                        .entry((row.civilization, row.world))
                        .or_insert(row.observed_tick);
                    self.index += 1;
                    return Ok(true);
                }
                self.advance(ForecastStocks);
            }
            ForecastStocks | ForecastDeficits | ForecastIdle => {
                if let Some((actor, forecast)) = inputs.forecasts.get(self.forecast_index) {
                    match self.stage {
                        ForecastStocks => {
                            if let Some((_, world, stock)) = forecast.inventories.get(self.index) {
                                let facts = self.projected.entry((*actor, *world)).or_default();
                                facts.0 |= stock.energy >= 4 && stock.ore >= 4;
                                facts.1 |= stock.alloy <= LIVING_RESOURCE_STORAGE_LIMIT_V2 - 2;
                                self.index += 1;
                                return Ok(true);
                            }
                            self.advance(ForecastDeficits);
                        }
                        ForecastDeficits => {
                            if let Some((world, resource)) = forecast.deficits.get(self.index) {
                                self.deficits.insert((*actor, *world, *resource));
                                self.index += 1;
                                return Ok(true);
                            }
                            self.advance(ForecastIdle);
                        }
                        ForecastIdle => {
                            if let Some(id) = forecast.idle_facilities.get(self.index) {
                                self.idle.insert((*actor, *id));
                                self.index += 1;
                                return Ok(true);
                            }
                            self.forecast_index += 1;
                            self.advance(ForecastStocks);
                        }
                        _ => unreachable!(),
                    }
                } else {
                    self.advance(Civilizations);
                }
            }
            Civilizations => {
                if let Some(row) = state.civilizations.as_slice().get(self.index) {
                    self.actors.insert(row.id, self.index);
                    self.index += 1;
                    return Ok(true);
                }
                self.advance(FleetCounts);
            }
            FleetCounts => {
                if let Some(fleet) = state.fleets.as_slice().get(self.index) {
                    if let Some(hull) = fleet.hulls.as_slice().get(self.nested) {
                        if let Some(i) = economic_hull_index(hull.definition.as_str()) {
                            self.counts.entry(fleet.owner).or_default()[i] += 1;
                        }
                        if hull.definition.as_str() == "escort" {
                            self.escorts.insert(fleet.id);
                        }
                        self.nested += 1;
                        return Ok(true);
                    }
                    self.index += 1;
                    self.nested = 0;
                    return Ok(true);
                }
                self.advance(PaidHullCounts);
            }
            PaidHullCounts => {
                if let Some(job) = state.hull_jobs.as_slice().get(self.index) {
                    if let Some(i) = economic_hull_index(job.definition.as_str()) {
                        self.counts.entry(job.owner).or_default()[i] += 1;
                    }
                    self.index += 1;
                    return Ok(true);
                }
                self.advance(Actors);
            }
            Actors => {
                if let Some((actor, _)) = inputs.forecasts.get(self.actor_index) {
                    if !self.actors.contains_key(actor) {
                        return Err(LivingDeterministicFaultV2::Invariant);
                    }
                    self.batch = Some(LivingEconomicBatchV2 {
                        actor: *actor,
                        economic_count: 0,
                        ranked: Vec::new(),
                        explanations: Vec::new(),
                    });
                    self.emission = 0;
                    self.colony_count = 0;
                    self.colony_index = 0;
                    self.request_index = 0;
                    self.previous = None;
                    self.advance(ColonyCount);
                    return Ok(true);
                }
                self.advance(RetireWorlds);
            }
            ColonyCount => {
                if let Some(colony) = state.colonies.as_slice().get(self.index) {
                    self.colony_count += usize::from(colony.owner == self.actor());
                    self.index += 1;
                    return Ok(true);
                }
                self.advance(Colonies);
            }
            Colonies => {
                if let Some(colony) = state.colonies.as_slice().get(self.colony_index) {
                    self.colony_index += 1;
                    if colony.owner == self.actor() {
                        self.world_index = *self
                            .worlds
                            .get(&colony.world)
                            .ok_or(LivingDeterministicFaultV2::Invariant)?;
                        self.has = [false; 5];
                        self.enabled = [0; 5];
                        self.shipyard = false;
                        self.construction_job = false;
                        self.hull_job = false;
                        self.deposit = false;
                        self.threatened = false;
                        self.scrap = None;
                        self.ark = false;
                        self.advance(Facilities);
                    }
                    return Ok(true);
                }
                self.advance(Requests);
            }
            Facilities => {
                let world = state.worlds.as_slice()[self.world_index].id;
                if let Some(facility) = state.facilities.as_slice().get(self.index) {
                    if facility.world == world {
                        self.facilities.push(self.index);
                        if let Some(i) = economic_industry_index(facility.definition.as_str()) {
                            self.has[i] = true;
                            self.enabled[i] += usize::from(!facility.disabled);
                        }
                        self.shipyard |=
                            facility.definition.as_str() == "shipyard" && !facility.disabled;
                    }
                    self.index += 1;
                    return Ok(true);
                }
                self.advance(ConstructionJobs);
            }
            ConstructionJobs => {
                if let Some(job) = state.construction_jobs.as_slice().get(self.index) {
                    if job.world == state.worlds.as_slice()[self.world_index].id {
                        self.construction_job = true;
                        if let Some(i) = economic_industry_index(job.definition.as_str()) {
                            self.has[i] = true;
                        }
                    }
                    self.index += 1;
                    return Ok(true);
                }
                self.advance(ColonyHullJobs);
            }
            ColonyHullJobs => {
                if let Some(job) = state.hull_jobs.as_slice().get(self.index) {
                    self.hull_job |= job.world == state.worlds.as_slice()[self.world_index].id;
                    self.index += 1;
                    return Ok(true);
                }
                self.advance(Threats);
            }
            Threats => {
                if let Some(fleet) = state.fleets.as_slice().get(self.index) {
                    self.threatened |= fleet.owner != self.actor()
                        && self.wars.contains(&ordered_pair(self.actor(), fleet.owner))
                        && matches!(fleet.location, LivingFleetLocationV2::Docked { world } if world == state.worlds.as_slice()[self.world_index].id)
                        && self.escorts.contains(&fleet.id);
                    self.index += 1;
                    return Ok(true);
                }
                // Extractor usefulness has its own per-observation scan below.
                self.advance(Replacement);
            }
            Replacement => {
                if let Some(i) = self.facilities.get(self.index) {
                    let facility = &state.facilities.as_slice()[*i];
                    if self.facilities.len() >= 6
                        && let Some(def) = economic_industry_index(facility.definition.as_str())
                        && def < 3
                        && (facility.disabled || self.idle.contains(&(self.actor(), facility.id)))
                        && facility.next_repair.is_none()
                        && self.enabled[def] > usize::from(!facility.disabled)
                    {
                        self.scrap = Some(self.scrap.map_or(facility.id, |id| id.min(facility.id)));
                    }
                    self.index += 1;
                    return Ok(true);
                }
                self.building_free =
                    !self.construction_job && (self.facilities.len() < 6 || self.scrap.is_some());
                self.advance(Buildings);
            }
            Buildings => {
                let world = &state.worlds.as_slice()[self.world_index];
                if self.index == 0 && self.nested < state.observations.as_slice().len() {
                    let row = &state.observations.as_slice()[self.nested];
                    self.deposit |= row.civilization == self.actor()
                        && row.world == world.id
                        && row.remaining_deposit >= 4;
                    self.nested += 1;
                    return Ok(true);
                }
                if self.index < 4 && self.building_free {
                    let definition =
                        ["extractor", "foundry", "solar_array", "defense_battery"][self.index];
                    let cost = *self
                        .industries
                        .get(definition)
                        .ok_or(LivingDeterministicFaultV2::Invariant)?;
                    let projected = self
                        .projected
                        .get(&(self.actor(), world.id))
                        .copied()
                        .unwrap_or_default();
                    let resource = [
                        LivingResourceV2::Ore,
                        LivingResourceV2::Alloy,
                        LivingResourceV2::Energy,
                        LivingResourceV2::Alloy,
                    ][self.index];
                    let deficit = self.deficits.contains(&(self.actor(), world.id, resource));
                    let useful = match self.index {
                        0 => !self.has[0] && self.deposit,
                        1 => !self.has[1] && projected.0 && projected.1,
                        2 => deficit,
                        _ => !self.has[3] && self.threatened,
                    };
                    let tier = if deficit && self.index != 3 {
                        economic_resource_tier(resource)
                    } else {
                        3
                    };
                    let bonus = if (self.policy() == "industrialist" && self.index < 2)
                        || (self.policy() == "guardian" && self.index == 3)
                    {
                        20
                    } else {
                        0
                    };
                    if useful && world.inventory.alloy >= cost {
                        self.emit(LivingEconomicCandidateV2 {
                            tier,
                            score: [50, 50, 40, 55][self.index] + bonus,
                            target: world.id,
                            action: LivingEconomicActionV2::Construction {
                                world: world.id,
                                definition,
                                scrap: self.scrap,
                            },
                        })?;
                    }
                    self.index += 1;
                    return Ok(true);
                }
                self.hull_index = 0;
                self.advance(Hulls);
            }
            Hulls => {
                // Keep the ark lookup complete when its nested path resumes here.
                if self.hull_index == 0 && self.index == 0 {
                    self.index = 1;
                    self.nested = 0;
                    self.stage = ArkTargets;
                    return Ok(false);
                }
                if self.hull_index < 3 {
                    let counts = self.counts.get(&self.actor()).copied().unwrap_or_default();
                    let needed = match self.hull_index {
                        0 => counts[0] == 0 && self.ark,
                        1 => counts[1] < self.colony_count * 2,
                        _ => counts[2] < 2,
                    };
                    let world = &state.worlds.as_slice()[self.world_index];
                    if needed {
                        let definition = ["colony_ark", "escort", "scout"][self.hull_index];
                        let cost = *self
                            .hull_costs
                            .get(definition)
                            .ok_or(LivingDeterministicFaultV2::Invariant)?;
                        if self.shipyard {
                            let bonus = if (self.hull_index == 0 && self.policy() == "expansionist")
                                || (self.hull_index == 1 && self.policy() == "guardian")
                            {
                                20
                            } else {
                                0
                            };
                            if !self.hull_job && world.inventory.alloy >= cost {
                                self.emit(LivingEconomicCandidateV2 {
                                    tier: 3,
                                    score: [50, 50, 40][self.hull_index] + bonus,
                                    target: world.id,
                                    action: LivingEconomicActionV2::Hull {
                                        world: world.id,
                                        definition,
                                    },
                                })?;
                            }
                        } else if !self.has[4] && self.building_free {
                            let cost = *self
                                .industries
                                .get("shipyard")
                                .ok_or(LivingDeterministicFaultV2::Invariant)?;
                            if world.inventory.alloy >= cost {
                                self.emit(LivingEconomicCandidateV2 {
                                    tier: 3,
                                    score: 55,
                                    target: world.id,
                                    action: LivingEconomicActionV2::Construction {
                                        world: world.id,
                                        definition: "shipyard",
                                        scrap: self.scrap,
                                    },
                                })?;
                            }
                        }
                    }
                    self.hull_index += 1;
                    return Ok(true);
                }
                self.resource_index = 0;
                self.advance(Resources);
            }
            ArkTargets => {
                if self.path_found {
                    self.ark = true;
                    self.path_found = false;
                    self.stage = Hulls;
                    return Ok(false);
                }
                let counts = self.counts.get(&self.actor()).copied().unwrap_or_default();
                if counts[0] == 0
                    && let Some(row) = state.observations.as_slice().get(self.nested)
                {
                    self.nested += 1;
                    if row.civilization == self.actor() && row.owner.is_none() {
                        self.start_path(
                            state.worlds.as_slice()[self.world_index].id,
                            row.world,
                            ArkTargets,
                        );
                    }
                    return Ok(true);
                }
                self.stage = Hulls;
            }
            Path => unreachable!("path handled by poll"),
            RetirePath => {
                if self.retired_path.pop().is_some() {
                    return Ok(true);
                }
                self.stage = self.path_resume;
            }
            Resources => {
                if self.resource_index < 3 {
                    let resource = economic_resource(self.resource_index);
                    let world = &state.worlds.as_slice()[self.world_index];
                    if inventory_amount(world.inventory, resource) < 20
                        && self.deficits.contains(&(self.actor(), world.id, resource))
                    {
                        self.source_index = 0;
                        self.stage = Sources;
                    } else {
                        self.resource_index += 1;
                    }
                    return Ok(true);
                }
                self.advance(RetireFacilities);
            }
            Sources | ForeignSources => {
                let foreign = self.stage == ForeignSources;
                if let Some(source) = state.worlds.as_slice().get(self.source_index) {
                    self.source_index += 1;
                    let (destination, resource, receiver, kind, batch_size, tier) = if foreign {
                        let request = &inputs.requests[self.request_index - 1];
                        (
                            request.world,
                            request.resource,
                            request.receiver,
                            if self
                                .trade
                                .contains(&ordered_pair(self.actor(), request.receiver))
                            {
                                LivingRouteKindV2::Trade
                            } else {
                                LivingRouteKindV2::Aid
                            },
                            10.min(request.requested_units),
                            3,
                        )
                    } else {
                        (
                            state.worlds.as_slice()[self.world_index].id,
                            economic_resource(self.resource_index),
                            self.actor(),
                            LivingRouteKindV2::Internal,
                            10,
                            self.resource_index as u8,
                        )
                    };
                    if source.owner == Some(self.actor())
                        && (foreign || source.id != destination)
                        && inventory_amount(source.inventory, resource) >= 20 + batch_size
                    {
                        self.supply = Some(LivingEconomicCandidateV2 {
                            tier,
                            score: 70 + if self.policy() == "trader" { 20 } else { 0 },
                            target: destination,
                            action: LivingEconomicActionV2::Supply {
                                source: source.id,
                                destination,
                                resource,
                                kind,
                                receiver,
                                batch_size,
                                existing_route: None,
                            },
                        });
                        self.first_route = None;
                        self.working_route = false;
                        self.index = 0;
                        self.start_path(source.id, destination, Routes);
                    }
                    return Ok(true);
                }
                if foreign {
                    self.advance(Requests);
                } else {
                    self.resource_index += 1;
                    self.stage = Resources;
                }
            }
            Routes => {
                if !self.path_found {
                    let internal = self.supply_is_internal();
                    self.supply = None;
                    self.stage = if internal { Sources } else { ForeignSources };
                    return Ok(true);
                }
                let candidate = self.supply.as_ref().expect("pending supply");
                let LivingEconomicActionV2::Supply {
                    source,
                    destination,
                    resource,
                    kind,
                    receiver,
                    batch_size,
                    ..
                } = candidate.action
                else {
                    unreachable!()
                };
                if let Some(route) = state.routes.as_slice().get(self.index) {
                    if route.source_world == source
                        && route.destination_world == destination
                        && route.resource == resource
                    {
                        if self.first_route.is_none() {
                            self.first_route = Some(route.id);
                        }
                        self.working_route |= !route.suspended
                            && route.source_owner == self.actor()
                            && route.receiver_owner == receiver
                            && route.kind == kind
                            && route.batch_size >= batch_size
                            && route.cadence_ticks == 10
                            && route.source_reserve == 20;
                    }
                    self.index += 1;
                    return Ok(true);
                }
                let mut candidate = self.supply.take().expect("pending supply");
                if let LivingEconomicActionV2::Supply { existing_route, .. } = &mut candidate.action
                {
                    *existing_route = self.first_route;
                }
                if !self.working_route {
                    self.emit(candidate)?;
                }
                self.path_found = false;
                self.stage = if kind == LivingRouteKindV2::Internal {
                    Sources
                } else {
                    ForeignSources
                };
                // Emitting or suppressing this pending combination is one record.
                return Ok(true);
            }
            Requests => {
                if let Some(request) = inputs.requests.get(self.request_index) {
                    self.request_index += 1;
                    self.adjacent = false;
                    self.adjacent_home = None;
                    if request.receiver != self.actor() {
                        if self
                            .trade
                            .contains(&ordered_pair(self.actor(), request.receiver))
                        {
                            self.source_index = 0;
                            self.stage = ForeignSources;
                        } else if self.policy() == "trader"
                            && !self
                                .wars
                                .contains(&ordered_pair(self.actor(), request.receiver))
                        {
                            self.advance(AdjacentColonies);
                        }
                    }
                    return Ok(true);
                }
                self.advance(Canonical);
            }
            AdjacentColonies => {
                if self.adjacent {
                    self.source_index = 0;
                    self.stage = ForeignSources;
                    return Ok(false);
                }
                let request = &inputs.requests[self.request_index - 1];
                let target = self
                    .worlds
                    .get(&request.world)
                    .map(|i| state.worlds.as_slice()[*i].system);
                if let Some(colony) = state.colonies.as_slice().get(self.index) {
                    self.index += 1;
                    if colony.owner == self.actor()
                        && let (Some(target), Some(home)) = (
                            target,
                            self.worlds
                                .get(&colony.world)
                                .map(|i| state.worlds.as_slice()[*i].system),
                        )
                    {
                        if home == target {
                            self.adjacent = true;
                        } else {
                            self.adjacent_home = Some(home);
                            self.nested = 0;
                            self.stage = AdjacentLanes;
                        }
                    }
                    return Ok(true);
                }
                self.advance(Requests);
            }
            AdjacentLanes => {
                let request = &inputs.requests[self.request_index - 1];
                let target = state.worlds.as_slice()[self.worlds[&request.world]].system;
                let home = self.adjacent_home.expect("adjacent home");
                if let Some(lane) = state.lanes.as_slice().get(self.nested) {
                    self.adjacent |= (lane.system_a == home && lane.system_b == target)
                        || (lane.system_b == home && lane.system_a == target);
                    self.nested += 1;
                    if self.adjacent {
                        self.stage = AdjacentColonies;
                    }
                    return Ok(true);
                }
                self.stage = AdjacentColonies;
            }
            Canonical => {
                if let Some((_, candidate)) = self.canonical.first_key_value() {
                    let duplicate = self.previous.as_ref() == Some(&candidate.action);
                    let ordinal = if duplicate {
                        None
                    } else {
                        Some(
                            u16::try_from(self.batch.as_ref().expect("actor batch").economic_count)
                                .map_err(|_| LivingDeterministicFaultV2::Arithmetic)?,
                        )
                    };
                    let (_, candidate) = self.canonical.pop_first().expect("canonical record");
                    if let Some(ordinal) = ordinal {
                        self.batch.as_mut().expect("actor batch").economic_count += 1;
                        self.previous = Some(candidate.action.clone());
                        let tie = candidate.tie();
                        self.ranking.insert(
                            (
                                candidate.tier,
                                std::cmp::Reverse(candidate.score),
                                candidate.target,
                                tie.0,
                                tie.1,
                                ordinal,
                            ),
                            candidate,
                        );
                    }
                    return Ok(true);
                }
                self.previous = None;
                self.advance(Ranking);
            }
            Ranking => {
                if let Some((key, candidate)) = self.ranking.first_key_value() {
                    let actor = self.actor();
                    let policy = &state.civilizations.as_slice()[self.actors[&actor]].policy.0;
                    let explanation = indexed_economic_decision(
                        actor,
                        key.5,
                        candidate,
                        self.tick,
                        inputs.digest,
                        self.observations.get(&(actor, candidate.target)).copied(),
                        policy,
                    )?;
                    let (key, candidate) = self.ranking.pop_first().expect("ranking record");
                    let batch = self.batch.as_mut().expect("actor batch");
                    batch.ranked.push((key.5, candidate));
                    batch.explanations.push(explanation);
                    return Ok(true);
                }
                self.batches.push(self.batch.take().expect("actor batch"));
                self.actor_index += 1;
                self.advance(Actors);
                return Ok(true);
            }
            RetireFacilities => {
                if self.facilities.pop().is_some() {
                    return Ok(true);
                }
                self.advance(Colonies);
            }
            RetireWorlds => {
                if self.worlds.pop_first().is_some() {
                    return Ok(true);
                }
                self.advance(RetireIndustries);
            }
            RetireIndustries => {
                if self.industries.pop_first().is_some() {
                    return Ok(true);
                }
                self.advance(RetireHulls);
            }
            RetireHulls => {
                if self.hull_costs.pop_first().is_some() {
                    return Ok(true);
                }
                self.advance(RetireWars);
            }
            RetireWars => {
                if self.wars.pop_first().is_some() {
                    return Ok(true);
                }
                self.advance(RetireTrade);
            }
            RetireTrade => {
                if self.trade.pop_first().is_some() {
                    return Ok(true);
                }
                self.advance(RetireObservations);
            }
            RetireObservations => {
                if self.observations.pop_first().is_some() {
                    return Ok(true);
                }
                self.advance(RetireStocks);
            }
            RetireStocks => {
                if self.projected.pop_first().is_some() {
                    return Ok(true);
                }
                self.advance(RetireDeficits);
            }
            RetireDeficits => {
                if self.deficits.pop_first().is_some() {
                    return Ok(true);
                }
                self.advance(RetireIdle);
            }
            RetireIdle => {
                if self.idle.pop_first().is_some() {
                    return Ok(true);
                }
                self.advance(RetireActors);
            }
            RetireActors => {
                if self.actors.pop_first().is_some() {
                    return Ok(true);
                }
                self.advance(RetireCounts);
            }
            RetireCounts => {
                if self.counts.pop_first().is_some() {
                    return Ok(true);
                }
                self.advance(RetireEscorts);
            }
            RetireEscorts => {
                if self.escorts.pop_first().is_some() {
                    return Ok(true);
                }
                self.advance(RetireGraphLanes);
            }
            RetireGraphLanes => {
                if std::sync::Arc::get_mut(&mut self.graph)
                    .expect("retained graph")
                    .lanes
                    .pop()
                    .is_some()
                {
                    return Ok(true);
                }
                self.advance(RetireGraphSystems);
            }
            RetireGraphSystems => {
                if std::sync::Arc::get_mut(&mut self.graph)
                    .expect("retained graph")
                    .systems
                    .pop()
                    .is_some()
                {
                    return Ok(true);
                }
                self.advance(Ready);
            }
            Ready => {}
        }
        Ok(false)
    }
    fn supply_is_internal(&self) -> bool {
        self.supply.as_ref().is_some_and(|candidate| {
            matches!(
                candidate.action,
                LivingEconomicActionV2::Supply {
                    kind: LivingRouteKindV2::Internal,
                    ..
                }
            )
        })
    }
    fn finish(
        &mut self,
    ) -> Result<LivingEconomicEnumerationV2, super::step_job::LivingLifecycleJobErrorV2> {
        use super::step_job::LivingLifecycleJobErrorV2 as Error;
        if let Some(error) = self.path_fault {
            return Err(error.public());
        }
        if self.cancelled {
            return Err(Error::Cancelled);
        }
        if let Some(error) = &self.fault {
            return Err(error.clone());
        }
        if self.stage != EconomicEnumerationStage::Ready
            || self.delivered
            || !self.retirement.is_empty()
            || self.inputs.is_none()
        {
            return Err(Error::NotReady);
        }
        let inputs = self.inputs.take().ok_or(Error::NotReady)?;
        self.delivered = true;
        Ok(LivingEconomicEnumerationV2 {
            catalog: std::sync::Arc::clone(&self.catalog).into(),
            inputs: inputs.into(),
            graph: std::sync::Arc::clone(&self.graph).into(),
            batches: std::mem::take(&mut self.batches),
        })
    }
}
fn economic_industry_index(definition: &str) -> Option<usize> {
    match definition {
        "extractor" => Some(0),
        "foundry" => Some(1),
        "solar_array" => Some(2),
        "defense_battery" => Some(3),
        "shipyard" => Some(4),
        _ => None,
    }
}
fn economic_hull_index(definition: &str) -> Option<usize> {
    match definition {
        "colony_ark" => Some(0),
        "escort" => Some(1),
        "scout" => Some(2),
        _ => None,
    }
}
fn economic_resource(index: usize) -> LivingResourceV2 {
    [
        LivingResourceV2::Energy,
        LivingResourceV2::Ore,
        LivingResourceV2::Alloy,
    ][index]
}
fn economic_resource_tier(resource: LivingResourceV2) -> u8 {
    match resource {
        LivingResourceV2::Energy => 0,
        LivingResourceV2::Ore => 1,
        LivingResourceV2::Alloy => 2,
    }
}

fn indexed_economic_decision(
    actor: LivingEntityIdV2,
    ordinal: u16,
    candidate: &LivingEconomicCandidateV2,
    tick: LivingTickV2,
    snapshot_digest: super::ids::LivingStateDigestV2,
    observed_tick: Option<LivingTickV2>,
    policy: &LivingSlugV2,
) -> Result<LivingIntentDecisionV2, LivingDeterministicFaultV2> {
    let action = match candidate.action {
        LivingEconomicActionV2::Construction {
            world,
            definition,
            scrap,
        } => LivingIntentActionV2::Construction {
            world,
            definition: slug(definition).map_err(|_| LivingDeterministicFaultV2::Invariant)?,
            scrap,
        },
        LivingEconomicActionV2::Hull { world, definition } => LivingIntentActionV2::Hull {
            world,
            definition: slug(definition).map_err(|_| LivingDeterministicFaultV2::Invariant)?,
        },
        LivingEconomicActionV2::Supply {
            source,
            destination,
            resource,
            existing_route,
            ..
        } => LivingIntentActionV2::Route {
            source,
            destination,
            resource,
            existing_route,
        },
    };
    let tier = match candidate.tier {
        0 => LivingDecisionTierV2::PowerEmergency,
        1 => LivingDecisionTierV2::OreEmergency,
        2 => LivingDecisionTierV2::AlloyEmergency,
        _ => LivingDecisionTierV2::Ordinary,
    };
    let (base, component, rule) = match candidate.action {
        LivingEconomicActionV2::Construction { definition, .. } => match definition {
            "solar_array" => (40, definition, "projected_power_deficit"),
            "extractor" => (50, definition, "observed_usable_deposit"),
            "foundry" => (50, definition, "projected_inputs_output_room"),
            "shipyard" => (55, definition, "missing_required_shipyard"),
            "defense_battery" => (55, definition, "threatened_owned_colony"),
            _ => return Err(LivingDeterministicFaultV2::Invariant),
        },
        LivingEconomicActionV2::Hull { definition, .. } => match definition {
            "colony_ark" => (50, definition, "eligible_settlement_target"),
            "escort" => (50, definition, "below_defense_target"),
            "scout" => (40, definition, "fewer_than_two_scouts"),
            _ => return Err(LivingDeterministicFaultV2::Invariant),
        },
        LivingEconomicActionV2::Supply { kind, .. } => (
            70,
            "supply",
            match kind {
                LivingRouteKindV2::Internal => "owned_needed_resource_deficit",
                LivingRouteKindV2::Trade => "receiver_demand_with_trade_permission",
                LivingRouteKindV2::Aid => "receiver_demand_nonhostile_neighbor_trader_aid",
            },
        ),
    };
    let mut reasons = vec![LivingDecisionReasonV2::Accepted {
        rule: slug(rule).map_err(|_| LivingDeterministicFaultV2::Invariant)?,
    }];
    if candidate.score > base {
        reasons.push(LivingDecisionReasonV2::PolicyScore {
            policy: policy.clone(),
            component: slug(component).map_err(|_| LivingDeterministicFaultV2::Invariant)?,
            value: LivingDecisionScoreV2(i64::from(candidate.score - base)),
        });
    }
    Ok(LivingIntentDecisionV2 {
        phase: LivingPhaseV2::GenerateIntents,
        actor,
        ordinal,
        action,
        target: Some(candidate.target),
        score: LivingDecisionScoreV2(i64::from(candidate.score)),
        tier,
        snapshot_tick: tick,
        snapshot_digest,
        observed_tick,
        outcome: LivingDecisionOutcomeV2::Rejected,
        reasons,
    })
}

// Private owned fleet assembly. Upserts deliberately replace an equal identity,
// just like put; overwritten nested owners remain until counted retirement.
#[derive(Clone, Copy)]
enum FleetAssemblyChange {
    Order {
        fleet: LivingEntityIdV2,
        target: LivingEntityIdV2,
        kind: LivingFleetOrderKindV2,
        scout: Option<LivingEntityIdV2>,
        all: bool,
        split: LivingEntityIdV2,
        tick: LivingTickV2,
    },
    Rekey {
        fleet: LivingEntityIdV2,
        id: LivingEntityIdV2,
    },
}
struct LivingFleetAssemblyCursorV2 {
    retirement: super::step_job::PhaseRetirementLane<'static>,
    pending_output: Option<LivingFleetV2>,
    pending_hull: Option<LivingHullV2>,
    delivered: bool,
    source: std::vec::IntoIter<LivingFleetV2>,
    rows: std::collections::BTreeMap<LivingEntityIdV2, LivingFleetV2>,
    change: FleetAssemblyChange,
    stage: u8,
    selected: Option<LivingFleetV2>,
    hulls: std::vec::IntoIter<LivingHullV2>,
    moving: LivingSortedVecV2<LivingHullV2>,
    remaining: LivingSortedVecV2<LivingHullV2>,
    ordered: Option<LivingFleetV2>,
    retired: Vec<LivingFleetV2>,
    retired_hulls: std::vec::IntoIter<LivingHullV2>,
    retired_path: Vec<LivingEntityIdV2>,
    output: LivingSortedVecV2<LivingFleetV2>,
}
impl LivingFleetAssemblyCursorV2 {
    fn begin(fleets: LivingSortedVecV2<LivingFleetV2>, change: FleetAssemblyChange) -> Self {
        Self {
            retirement: Default::default(),
            pending_output: None,
            pending_hull: None,
            delivered: false,
            source: fleets.into_vec().into_iter(),
            rows: Default::default(),
            change,
            stage: 0,
            selected: None,
            hulls: Vec::new().into_iter(),
            moving: Default::default(),
            remaining: Default::default(),
            ordered: None,
            retired: Vec::new(),
            retired_hulls: Vec::new().into_iter(),
            retired_path: Vec::new(),
            output: Default::default(),
        }
    }
    fn upsert(&mut self, row: LivingFleetV2) {
        if let Some(old) = self.rows.insert(row.id, row) {
            self.retirement.start(old);
        }
    }
    fn visit(&mut self) -> Result<bool, LivingDeterministicFaultV2> {
        if self.retirement.visit() {
            return Ok(true);
        }
        match self.stage {
            0 => {
                if let Some(row) = self.source.next() {
                    self.upsert(row);
                    return Ok(true);
                }
                self.stage = 1;
            }
            1 => {
                let id = match self.change {
                    FleetAssemblyChange::Order { fleet, .. }
                    | FleetAssemblyChange::Rekey { fleet, .. } => fleet,
                };
                self.selected = Some(
                    self.rows
                        .remove(&id)
                        .ok_or(LivingDeterministicFaultV2::Invariant)?,
                );
                if let FleetAssemblyChange::Rekey { id, .. } = self.change {
                    self.selected.as_mut().expect("selected row").id = id;
                    self.stage = 4;
                } else {
                    self.hulls =
                        std::mem::take(&mut self.selected.as_mut().expect("selected row").hulls)
                            .into_vec()
                            .into_iter();
                    self.stage = 2;
                }
                return Ok(true);
            }
            2 => {
                if let Some(hull) = self.hulls.next() {
                    self.pending_hull = Some(hull);
                    let hull = self.pending_hull.as_ref().expect("held hull");
                    let FleetAssemblyChange::Order { scout, all, .. } = self.change else {
                        unreachable!()
                    };
                    if all || scout.map_or(hull.definition.as_str() == "escort", |id| hull.id == id)
                    {
                        rules_ensure_append(&self.moving, hull)?;
                        self.moving
                            .push_sorted(self.pending_hull.take().expect("checked hull"))?;
                    } else {
                        rules_ensure_append(&self.remaining, hull)?;
                        self.remaining
                            .push_sorted(self.pending_hull.take().expect("checked hull"))?;
                    }
                    return Ok(true);
                }
                self.stage = 3;
            }
            3 => {
                let FleetAssemblyChange::Order {
                    target,
                    kind,
                    split,
                    tick,
                    ..
                } = self.change
                else {
                    unreachable!()
                };
                // Compute the fallible scalar before moving any owners.
                let order = LivingFleetOrderV2 {
                    kind,
                    target_world: Some(target),
                    not_before_tick: add_ticks(tick, 1)?,
                    lane_path: Vec::new(),
                };
                let row = self.selected.as_mut().expect("selected row");
                if self.remaining.as_slice().is_empty() {
                    if let Some(old) = row.order.take() {
                        self.retired_path = old.lane_path;
                    }
                    row.order = Some(order);
                    row.hulls = std::mem::take(&mut self.moving);
                } else {
                    row.hulls = std::mem::take(&mut self.remaining);
                    self.ordered = Some(LivingFleetV2 {
                        id: split,
                        owner: row.owner,
                        location: row.location,
                        order: Some(order),
                        hulls: std::mem::take(&mut self.moving),
                    });
                }
                self.stage = 4;
                return Ok(true);
            }
            4 => {
                let row = self.selected.take().expect("selected row");
                self.upsert(row);
                self.stage = 5;
                return Ok(true);
            }
            5 => {
                self.stage = 6;
                if let Some(row) = self.ordered.take() {
                    self.upsert(row);
                    return Ok(true);
                }
            }
            6 => {
                if self.retired_hulls.next().is_some() || self.retired_path.pop().is_some() {
                    return Ok(true);
                }
                if let Some(row) = self.retired.pop() {
                    self.retirement.start(row);
                    return Ok(true);
                }
                self.stage = 7;
            }
            7 => {
                if let Some((_, row)) = self.rows.pop_first() {
                    self.pending_output = Some(row);
                    rules_ensure_append(
                        &self.output,
                        self.pending_output.as_ref().expect("held output"),
                    )?;
                    self.output
                        .push_sorted(self.pending_output.take().expect("checked output"))?;
                    return Ok(true);
                }
                self.stage = 8;
            }
            _ => {}
        }
        Ok(false)
    }
    fn ready(&self) -> bool {
        self.stage == 8 && self.retirement.is_empty()
    }
    // Synchronous compatibility only.
    #[cfg(test)]
    fn finish(mut self) -> LivingSortedVecV2<LivingFleetV2> {
        self.take_ready_result().expect("ready assembly")
    }
    fn take_ready_result(
        &mut self,
    ) -> Result<LivingSortedVecV2<LivingFleetV2>, LivingDeterministicFaultV2> {
        if !self.ready() || self.delivered {
            return Err(LivingDeterministicFaultV2::Invariant);
        }
        self.delivered = true;
        Ok(std::mem::take(&mut self.output))
    }
}

#[derive(Debug, Eq, PartialEq)]
struct LivingFleetDeltaV2 {
    id: LivingEntityIdV2,
    before_index: Option<usize>,
    after_index: Option<usize>,
}
struct LivingFleetDeltaCursorV2 {
    delivered: bool,
    before: usize,
    after: usize,
    stage: u8,
    nested: usize,
    pair: (Option<usize>, Option<usize>),
    changed: bool,
    output: Vec<LivingFleetDeltaV2>,
}
impl LivingFleetDeltaCursorV2 {
    fn begin() -> Self {
        Self {
            delivered: false,
            before: 0,
            after: 0,
            stage: 0,
            nested: 0,
            pair: (None, None),
            changed: false,
            output: Vec::new(),
        }
    }
    fn take_ready_result(&mut self) -> Result<Vec<LivingFleetDeltaV2>, LivingDeterministicFaultV2> {
        if self.stage != 4 || self.delivered {
            return Err(LivingDeterministicFaultV2::Invariant);
        }
        self.delivered = true;
        Ok(std::mem::take(&mut self.output))
    }
    fn visit(&mut self, snapshot: &LivingGalaxyStateV2, candidate: &LivingGalaxyStateV2) -> bool {
        if self.stage == 0 {
            let before = snapshot.fleets.as_slice().get(self.before);
            let after = candidate.fleets.as_slice().get(self.after);
            self.pair = match (before, after) {
                (None, None) => {
                    self.stage = 4;
                    return false;
                }
                (Some(a), Some(b)) if a.id == b.id => (Some(self.before), Some(self.after)),
                (Some(a), Some(b)) if a.id < b.id => (Some(self.before), None),
                (Some(_), Some(_)) => (None, Some(self.after)),
                (Some(_), None) => (Some(self.before), None),
                (None, Some(_)) => (None, Some(self.after)),
            };
            let before = self.pair.0.map(|i| &snapshot.fleets.as_slice()[i]);
            let after = self.pair.1.map(|i| &candidate.fleets.as_slice()[i]);
            self.changed = match (before, after) {
                (Some(a), Some(b)) => {
                    a.owner != b.owner
                        || a.location != b.location
                        || a.order
                            .as_ref()
                            .map(|o| (o.kind, o.target_world, o.not_before_tick, o.lane_path.len()))
                            != b.order.as_ref().map(|o| {
                                (o.kind, o.target_world, o.not_before_tick, o.lane_path.len())
                            })
                        || a.hulls.as_slice().len() != b.hulls.as_slice().len()
                }
                _ => true,
            };
            self.nested = 0;
            self.stage = 1;
            return true;
        }
        let before = self.pair.0.map(|i| &snapshot.fleets.as_slice()[i]);
        let after = self.pair.1.map(|i| &candidate.fleets.as_slice()[i]);
        if self.stage == 1 {
            let a = before
                .and_then(|f| f.order.as_ref())
                .and_then(|o| o.lane_path.get(self.nested));
            let b = after
                .and_then(|f| f.order.as_ref())
                .and_then(|o| o.lane_path.get(self.nested));
            if a.is_some() || b.is_some() {
                self.changed |= a != b;
                self.nested += 1;
                return true;
            }
            self.nested = 0;
            self.stage = 2;
            return false;
        }
        if self.stage == 2 {
            let a = before.and_then(|f| f.hulls.as_slice().get(self.nested));
            let b = after.and_then(|f| f.hulls.as_slice().get(self.nested));
            if a.is_some() || b.is_some() {
                self.changed |= a != b;
                self.nested += 1;
                return true;
            }
            self.stage = 3;
            return false;
        }
        if self.stage == 3 {
            if self.changed {
                self.output.push(LivingFleetDeltaV2 {
                    id: before.or(after).expect("delta row").id,
                    before_index: self.pair.0,
                    after_index: self.pair.1,
                });
            }
            self.before += usize::from(self.pair.0.is_some());
            self.after += usize::from(self.pair.1.is_some());
            self.stage = 0;
            return self.changed;
        }
        false
    }
}

struct LivingFleetDecisionCursorV2 {
    index: usize,
    old: usize,
    proposed_hull: usize,
    old_hull: usize,
    stage: u8,
    path: usize,
    result: Option<(usize, LivingEntityIdV2)>,
}
impl LivingFleetDecisionCursorV2 {
    fn begin() -> Self {
        Self {
            index: 0,
            old: 0,
            proposed_hull: 0,
            old_hull: 0,
            stage: 0,
            path: 0,
            result: None,
        }
    }
    fn select_existing(&mut self, id: LivingEntityIdV2) {
        self.result = Some((self.index, id));
        self.stage = 4;
    }
    fn visit(
        &mut self,
        snapshot: &LivingGalaxyStateV2,
        candidate: &LivingGalaxyStateV2,
        indexes: &std::collections::BTreeMap<LivingEntityIdV2, usize>,
        actor: LivingEntityIdV2,
    ) -> Result<bool, LivingDeterministicFaultV2> {
        match self.stage {
            0 => {
                let Some(row) = candidate.fleets.as_slice().get(self.index) else {
                    self.stage = 4;
                    return Ok(false);
                };
                if row.owner != actor || row.order.is_none() {
                    self.index += 1;
                    return Ok(true);
                }
                if let Some(old) = indexes.get(&row.id) {
                    self.old = *old;
                    let order = row.order.as_ref().expect("ordered row");
                    let previous = snapshot.fleets.as_slice()[*old].order.as_ref();
                    if previous
                        .map(|o| (o.kind, o.target_world, o.not_before_tick, o.lane_path.len()))
                        != Some((
                            order.kind,
                            order.target_world,
                            order.not_before_tick,
                            order.lane_path.len(),
                        ))
                    {
                        self.select_existing(row.id);
                    } else {
                        self.path = 0;
                        self.stage = 1;
                    }
                } else {
                    self.old = 0;
                    self.stage = 2;
                }
                Ok(true)
            }
            1 => {
                let row = &candidate.fleets.as_slice()[self.index];
                let a = &row.order.as_ref().expect("ordered row").lane_path;
                let b = &snapshot.fleets.as_slice()[self.old]
                    .order
                    .as_ref()
                    .expect("equal order header")
                    .lane_path;
                if let Some(a) = a.get(self.path) {
                    if Some(a) != b.get(self.path) {
                        self.select_existing(row.id)
                    } else {
                        self.path += 1
                    }
                    return Ok(true);
                }
                self.index += 1;
                self.stage = 0;
                Ok(false)
            }
            2 => {
                if self.old == snapshot.fleets.as_slice().len() {
                    return Err(LivingDeterministicFaultV2::Invariant);
                }
                self.proposed_hull = 0;
                self.old_hull = 0;
                self.stage = 3;
                Ok(true)
            }
            3 => {
                let proposed = &candidate.fleets.as_slice()[self.index].hulls;
                let old = &snapshot.fleets.as_slice()[self.old];
                let Some(hull) = proposed.as_slice().get(self.proposed_hull) else {
                    self.select_existing(old.id);
                    return Ok(false);
                };
                let Some(previous) = old.hulls.as_slice().get(self.old_hull) else {
                    self.old += 1;
                    self.stage = 2;
                    return Ok(false);
                };
                match previous.id.cmp(&hull.id) {
                    std::cmp::Ordering::Less => self.old_hull += 1,
                    std::cmp::Ordering::Equal => {
                        self.old_hull += 1;
                        self.proposed_hull += 1;
                    }
                    std::cmp::Ordering::Greater => {
                        self.old += 1;
                        self.stage = 2;
                    }
                }
                Ok(true)
            }
            _ => Ok(false),
        }
    }
}

#[derive(Default)]
struct FleetEnumerationSummary {
    hulls: usize,
    escorts: usize,
    ark: bool,
    all_credited: bool,
    all_cost: u32,
    escort_cost: u32,
    scout: Option<LivingEntityIdV2>,
}
struct LivingFleetCandidateV2 {
    state: Box<LivingGalaxyStateV2>,
    decision: LivingIntentDecisionV2,
    action_bytes: Vec<u8>,
    deltas: Vec<LivingFleetDeltaV2>,
}
struct LivingFleetBatchV2 {
    actor: LivingEntityIdV2,
    economy: Option<LivingEconomicBatchV2>,
    fleets: Vec<LivingFleetCandidateV2>,
}
struct LivingFleetEnumerationV2 {
    economics: RulesOwner<LivingEconomicEnumerationV2>,
    graph: RulesOwner<std::sync::Arc<super::step_job::RetreatGraph>>,
    batches: Vec<LivingFleetBatchV2>,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum FleetEnumerationStage {
    Worlds,
    Colonies,
    Civilizations,
    HullJobs,
    Fleets,
    Hulls,
    Wars,
    Threats,
    Observations,
    Systems,
    Lanes,
    Actors,
    Targets,
    Pairs,
    ExploreFleet,
    ExploreTargets,
    Path,
    RetirePath,
    PairComplete,
    Selected,
    Copy,
    Assemble,
    Decision,
    Explain,
    Canonical,
    RekeyScan,
    Rekey,
    Delta,
    Ranking,
    Retire,
    Ready,
}
type FleetCanonicalKey = (u16, Option<LivingEntityIdV2>, Vec<u8>, usize);
type FleetRankingKey = (
    u8,
    std::cmp::Reverse<i64>,
    Option<LivingEntityIdV2>,
    FleetCanonicalKey,
);
struct LivingFleetEnumerationJobV2 {
    retirement: super::step_job::PhaseRetirementLane<'static>,
    economics: Option<LivingEconomicEnumerationV2>,
    economic_batches: std::vec::IntoIter<LivingEconomicBatchV2>,
    tick: LivingTickV2,
    seed: [u8; 32],
    branch: super::ids::LivingBranchIdV2,
    graph: RulesOwner<std::sync::Arc<super::step_job::RetreatGraph>>,
    worlds: std::collections::BTreeMap<LivingEntityIdV2, usize>,
    colonies: std::collections::BTreeMap<LivingEntityIdV2, LivingEntityIdV2>,
    actors: std::collections::BTreeMap<LivingEntityIdV2, usize>,
    fleet_indexes: std::collections::BTreeMap<LivingEntityIdV2, usize>,
    summaries: std::collections::BTreeMap<LivingEntityIdV2, FleetEnumerationSummary>,
    targeted: std::collections::BTreeSet<LivingEntityIdV2>,
    wars: std::collections::BTreeSet<(LivingEntityIdV2, LivingEntityIdV2)>,
    escorts: std::collections::BTreeSet<(LivingEntityIdV2, LivingEntityIdV2)>,
    attacked: std::collections::BTreeSet<(LivingEntityIdV2, LivingEntityIdV2)>,
    threat_cursor: Option<(LivingEntityIdV2, LivingEntityIdV2)>,
    observations: std::collections::BTreeMap<(LivingEntityIdV2, LivingEntityIdV2), LivingTickV2>,
    known: std::collections::BTreeSet<(LivingEntityIdV2, LivingEntityIdV2)>,
    pending_fleets: usize,
    stage: FleetEnumerationStage,
    index: usize,
    nested: usize,
    actor_index: usize,
    alternative: u8,
    target_index: usize,
    fleet_index: usize,
    target: LivingEntityIdV2,
    choice: Option<(LivingEntityIdV2, LivingEntityIdV2)>,
    explore: bool,
    explore_fleet: Option<usize>,
    explore_key: Option<(bool, u64, u64, LivingEntityIdV2, LivingEntityIdV2)>,
    pending_pair: Option<(LivingEntityIdV2, usize)>,
    duration: Option<u64>,
    path: Option<super::step_job::RetreatPathJob>,
    retired_path: Vec<LivingEntityIdV2>,
    copy: Option<super::step_job::LivingStateCopyJobV2>,
    assembly: Option<LivingFleetAssemblyCursorV2>,
    candidate: Option<Box<LivingGalaxyStateV2>>,
    decision_cursor: Option<LivingFleetDecisionCursorV2>,
    canonical: std::collections::BTreeMap<FleetCanonicalKey, LivingFleetCandidateV2>,
    ranking: std::collections::BTreeMap<FleetRankingKey, LivingFleetCandidateV2>,
    working: Option<(FleetCanonicalKey, LivingFleetCandidateV2)>,
    delta: Option<LivingFleetDeltaCursorV2>,
    ordinal_index: usize,
    batch: Option<LivingFleetBatchV2>,
    batches: Vec<LivingFleetBatchV2>,
    retire_index: u8,
    cancelled: bool,
    delivered: bool,
    path_fault: Option<super::step_job::RetreatPathFailure>,
    fault: Option<super::step_job::LivingLifecycleJobErrorV2>,
    last_poll_visits: usize,
    total_visits: u64,
}
impl LivingFleetEnumerationJobV2 {
    fn begin(
        mut economics: LivingEconomicEnumerationV2,
        tick: LivingTickV2,
        seed: [u8; 32],
        branch: super::ids::LivingBranchIdV2,
    ) -> Self {
        let fault = (tick.0 == 0
            || economics.inputs.snapshot.tick.0.checked_add(1) != Some(tick.0))
        .then_some(super::step_job::LivingLifecycleJobErrorV2::Fault(
            LivingDeterministicFaultV2::Decision(super::decision::LivingDecisionErrorV2::Snapshot),
        ));
        let economic_batches = std::mem::take(&mut economics.batches).into_iter();
        Self {
            retirement: Default::default(),
            economics: Some(economics),
            economic_batches,
            tick,
            seed,
            branch,
            graph: Default::default(),
            worlds: Default::default(),
            colonies: Default::default(),
            actors: Default::default(),
            fleet_indexes: Default::default(),
            summaries: Default::default(),
            targeted: Default::default(),
            wars: Default::default(),
            escorts: Default::default(),
            attacked: Default::default(),
            threat_cursor: None,
            observations: Default::default(),
            known: Default::default(),
            pending_fleets: 0,
            stage: FleetEnumerationStage::Worlds,
            index: 0,
            nested: 0,
            actor_index: 0,
            alternative: 0,
            target_index: 0,
            fleet_index: 0,
            target: LivingEntityIdV2([0; 16]),
            choice: None,
            explore: false,
            explore_fleet: None,
            explore_key: None,
            pending_pair: None,
            duration: None,
            path: None,
            retired_path: Vec::new(),
            copy: None,
            assembly: None,
            candidate: None,
            decision_cursor: None,
            canonical: Default::default(),
            ranking: Default::default(),
            working: None,
            delta: None,
            ordinal_index: 0,
            batch: None,
            batches: Vec::new(),
            retire_index: 0,
            cancelled: false,
            delivered: false,
            path_fault: None,
            fault,
            last_poll_visits: 0,
            total_visits: 0,
        }
    }
    fn snapshot(&self) -> &LivingGalaxyStateV2 {
        &self
            .economics
            .as_ref()
            .expect("retained economics")
            .inputs
            .snapshot
    }
    fn actor(&self) -> &LivingCivilizationV2 {
        &self.snapshot().civilizations.as_slice()[self.actor_index]
    }
    fn advance(&mut self, stage: FleetEnumerationStage) {
        self.stage = stage;
        self.index = 0;
        self.nested = 0;
    }
    #[cfg(test)]
    fn cancel(&mut self) {
        self.cancelled = true;
    }
    fn identity(&self, actor: LivingEntityIdV2, ordinal: u16) -> LivingEntityIdV2 {
        autonomous_entity_id_v2(&LivingAutonomousEntityInputsV2 {
            catalog_hash: self
                .economics
                .as_ref()
                .expect("retained economics")
                .catalog
                .catalog_hash(),
            genesis_seed: self.seed,
            branch: self.branch,
            tick: self.tick,
            phase: LivingPhaseV2::GenerateIntents.ordinal(),
            actor,
            intent_ordinal: ordinal,
            entity_kind: LivingEntityKindV2::Fleet.ordinal(),
            local_id: 0,
        })
    }
    fn own_colony(&self, world: LivingEntityIdV2, actor: LivingEntityIdV2) -> bool {
        self.colonies.get(&world) == Some(&actor)
    }
    fn energy(&self, world: LivingEntityIdV2) -> u32 {
        self.worlds.get(&world).map_or(0, |i| {
            self.snapshot().worlds.as_slice()[*i].inventory.energy
        })
    }
    fn full(&self) -> bool {
        self.snapshot().fleets.as_slice().len() + self.pending_fleets
            >= super::model::LIVING_MAX_FLEETS_V2
    }
    fn role_cost(&self, summary: &FleetEnumerationSummary) -> u32 {
        if self.full() {
            summary.all_cost
        } else {
            summary.escort_cost
        }
    }
    fn reset_alternative(&mut self) {
        self.target_index = 0;
        self.fleet_index = 0;
        self.choice = None;
        self.explore = false;
        self.explore_fleet = None;
        self.explore_key = None;
        self.pending_pair = None;
        self.stage = FleetEnumerationStage::Targets;
    }
    fn next_alternative(&mut self) {
        self.alternative += 1;
        if self.alternative == 4 {
            self.ordinal_index = 0;
            self.advance(FleetEnumerationStage::Canonical)
        } else {
            self.reset_alternative();
        }
    }
    fn start_path(&mut self, from: LivingEntityIdV2, to: LivingEntityIdV2) {
        self.duration = None;
        if self.alternative == 0 && from == to {
            self.duration = Some(10);
            self.stage = FleetEnumerationStage::PairComplete;
            return;
        }
        let from = self
            .worlds
            .get(&from)
            .map(|i| self.snapshot().worlds.as_slice()[*i].system);
        let to = self
            .worlds
            .get(&to)
            .map(|i| self.snapshot().worlds.as_slice()[*i].system);
        match (from, to) {
            (Some(from), Some(to)) if from == to => {
                self.duration = Some(10);
                self.stage = FleetEnumerationStage::PairComplete;
            }
            (Some(from), Some(to)) => {
                self.path = Some(super::step_job::RetreatPathJob::begin(
                    std::sync::Arc::clone(&self.graph),
                    from,
                    to,
                ));
                self.stage = FleetEnumerationStage::Path;
            }
            _ => self.stage = FleetEnumerationStage::PairComplete,
        }
    }
    fn poll(
        &mut self,
        budget: usize,
    ) -> Result<
        super::step_job::LivingLifecycleProgressV2,
        super::step_job::LivingLifecycleJobErrorV2,
    > {
        use super::step_job::{
            LivingLifecycleJobErrorV2 as Error, LivingLifecycleProgressV2 as Progress,
        };
        self.last_poll_visits = 0;
        if let Some(error) = self.path_fault {
            return Err(error.public());
        }
        if self.cancelled {
            return Err(Error::Cancelled);
        }
        if budget == 0 || budget > 1024 {
            return Err(Error::Budget);
        }
        if let Some(error) = &self.fault {
            return Err(error.clone());
        }
        while self.last_poll_visits < budget
            && (self.stage != FleetEnumerationStage::Ready || !self.retirement.is_empty())
        {
            if self.retirement.visit() {
                self.last_poll_visits += 1;
                self.total_visits += 1;
                continue;
            }
            if self.stage == FleetEnumerationStage::Copy {
                let child = self.copy.as_mut().expect("copy child");
                let progress = child.poll(budget - self.last_poll_visits);
                let visits = child.last_poll_visits();
                self.last_poll_visits += visits;
                self.total_visits += visits as u64;
                match progress {
                    Ok(Progress::Pending) => {}
                    Ok(Progress::Ready) => {
                        if self.last_poll_visits == budget {
                            break;
                        }
                        self.last_poll_visits += 1;
                        self.total_visits += 1;
                        self.candidate = Some(Box::new(
                            self.copy
                                .as_mut()
                                .expect("ready copy")
                                .take_ready_result()?,
                        ));
                        self.retirement.start(self.copy.take().expect("ready copy"));
                        self.stage = FleetEnumerationStage::Assemble;
                    }
                    Err(error) => {
                        self.fault = Some(error.clone());
                        return Err(error);
                    }
                }
                continue;
            }

            if self.stage == FleetEnumerationStage::Path {
                let child = self.path.as_mut().expect("retained path child");
                let progress = child.poll(budget - self.last_poll_visits);
                let visits = child.last_poll_visits();
                self.last_poll_visits += visits;
                self.total_visits += visits as u64;
                match progress {
                    Err(error) => {
                        let failure = super::step_job::RetreatPathFailure::from_public(error);
                        self.path_fault = Some(failure);
                        return Err(failure.public());
                    }
                    Ok(Progress::Pending) => {}
                    Ok(Progress::Ready) if self.last_poll_visits < budget => {
                        self.last_poll_visits += 1;
                        self.total_visits += 1;
                        let result = match self.path.as_mut().unwrap().take_ready_result() {
                            Ok(result) => result,
                            Err(error) => {
                                let failure =
                                    super::step_job::RetreatPathFailure::from_public(error);
                                self.path_fault = Some(failure);
                                return Err(failure.public());
                            }
                        };
                        if let Some((duration, path)) = result {
                            self.duration = Some(duration);
                            self.retired_path = path;
                        }
                        self.retirement.start((
                            self.path.take().unwrap(),
                            std::mem::take(&mut self.retired_path),
                        ));
                        self.stage = FleetEnumerationStage::RetirePath;
                    }
                    Ok(Progress::Ready) => {}
                }
                continue;
            }
            match self.visit() {
                Ok(true) => {
                    self.last_poll_visits += 1;
                    self.total_visits += 1;
                }
                Ok(false) => {}
                Err(fault) => {
                    self.last_poll_visits += 1;
                    self.total_visits += 1;
                    let error = Error::Fault(fault);
                    self.fault = Some(error.clone());
                    return Err(error);
                }
            }
        }
        Ok(
            if self.stage == FleetEnumerationStage::Ready && self.retirement.is_empty() {
                Progress::Ready
            } else {
                Progress::Pending
            },
        )
    }
    fn visit(&mut self) -> Result<bool, LivingDeterministicFaultV2> {
        use FleetEnumerationStage::*;
        let state = &self
            .economics
            .as_ref()
            .expect("retained economics")
            .inputs
            .snapshot;
        match self.stage {
            Worlds => {
                if let Some(row) = state.worlds.as_slice().get(self.index) {
                    self.worlds.insert(row.id, self.index);
                    self.index += 1;
                    return Ok(true);
                }
                self.advance(Colonies);
            }
            Colonies => {
                if let Some(row) = state.colonies.as_slice().get(self.index) {
                    self.colonies.insert(row.world, row.owner);
                    self.index += 1;
                    return Ok(true);
                }
                self.advance(Civilizations);
            }
            Civilizations => {
                if let Some(row) = state.civilizations.as_slice().get(self.index) {
                    self.actors.insert(row.id, self.index);
                    self.index += 1;
                    return Ok(true);
                }
                self.advance(HullJobs);
            }
            HullJobs => {
                if let Some(row) = state.hull_jobs.as_slice().get(self.index) {
                    if let Some(id) = row.target_fleet {
                        self.targeted.insert(id);
                    } else {
                        self.pending_fleets = self
                            .pending_fleets
                            .checked_add(1)
                            .ok_or(LivingDeterministicFaultV2::Arithmetic)?;
                    }
                    self.index += 1;
                    return Ok(true);
                }
                self.advance(Fleets);
            }
            Fleets => {
                if let Some(row) = state.fleets.as_slice().get(self.index) {
                    self.fleet_indexes.insert(row.id, self.index);
                    self.summaries.insert(
                        row.id,
                        FleetEnumerationSummary {
                            all_credited: true,
                            ..Default::default()
                        },
                    );
                    self.nested = 0;
                    self.stage = Hulls;
                    return Ok(true);
                }
                self.advance(Wars);
            }
            Hulls => {
                let fleet = &state.fleets.as_slice()[self.index];
                if let Some(hull) = fleet.hulls.as_slice().get(self.nested) {
                    let funded = match fleet.location {
                        LivingFleetLocationV2::Docked { world } => {
                            self.energy(world) >= if hull.return_credit { 2 } else { 4 }
                        }
                        _ => false,
                    };
                    let summary = self.summaries.get_mut(&fleet.id).expect("indexed summary");
                    let cost = if hull.return_credit { 2 } else { 4 };
                    summary.hulls += 1;
                    summary.all_cost = summary
                        .all_cost
                        .checked_add(cost)
                        .ok_or(LivingDeterministicFaultV2::Arithmetic)?;
                    summary.all_credited &= hull.return_credit;
                    if hull.definition.as_str() == "escort" {
                        summary.escorts += 1;
                        summary.escort_cost = summary
                            .escort_cost
                            .checked_add(cost)
                            .ok_or(LivingDeterministicFaultV2::Arithmetic)?;
                    }
                    summary.ark |= hull.definition.as_str() == "colony_ark";
                    if summary.scout.is_none() && hull.definition.as_str() == "scout" && funded {
                        summary.scout = Some(hull.id);
                    }
                    self.nested += 1;
                    return Ok(true);
                }
                if self.summaries[&fleet.id].escorts > 0
                    && let LivingFleetLocationV2::Docked { world } = fleet.location
                {
                    self.escorts.insert((world, fleet.owner));
                }
                self.index += 1;
                self.stage = Fleets;
            }
            Wars => {
                if let Some(row) = state.wars.as_slice().get(self.index) {
                    if active_interval(row.start_tick, row.end_tick, self.tick) {
                        self.wars
                            .insert(ordered_pair(row.participant_a, row.participant_b));
                    }
                    self.index += 1;
                    return Ok(true);
                }
                self.advance(Threats);
            }
            Threats => {
                let Some(actor) = state.civilizations.as_slice().get(self.index) else {
                    self.advance(Observations);
                    return Ok(false);
                };
                let next = match self.threat_cursor {
                    None => self.escorts.first().copied(),
                    Some(last) => self
                        .escorts
                        .range((std::ops::Bound::Excluded(last), std::ops::Bound::Unbounded))
                        .next()
                        .copied(),
                };
                if let Some((world, enemy)) = next {
                    if enemy != actor.id
                        && self.own_colony(world, actor.id)
                        && self.wars.contains(&ordered_pair(actor.id, enemy))
                    {
                        self.attacked.insert((actor.id, world));
                    }
                    self.threat_cursor = next;
                    return Ok(true);
                }
                self.index += 1;
                self.threat_cursor = None;
            }
            Observations => {
                if let Some(row) = state.observations.as_slice().get(self.index) {
                    self.observations
                        .entry((row.civilization, row.world))
                        .or_insert(row.observed_tick);
                    if let Some(world) = self.worlds.get(&row.world) {
                        self.known
                            .insert((row.civilization, state.worlds.as_slice()[*world].system));
                    }
                    self.index += 1;
                    return Ok(true);
                }
                self.advance(Systems);
            }
            Systems => {
                if let Some(row) = state.systems.as_slice().get(self.index) {
                    std::sync::Arc::get_mut(&mut self.graph)
                        .expect("private graph")
                        .systems
                        .push(row.id);
                    self.index += 1;
                    return Ok(true);
                }
                self.advance(Lanes);
            }
            Lanes => {
                if let Some(row) = state.lanes.as_slice().get(self.index) {
                    std::sync::Arc::get_mut(&mut self.graph)
                        .expect("private graph")
                        .lanes
                        .push((row.id, row.system_a, row.system_b, row.distance_units));
                    self.index += 1;
                    return Ok(true);
                }
                self.advance(Actors);
            }
            Actors => {
                let Some(actor) = state.civilizations.as_slice().get(self.actor_index) else {
                    if !self.economic_batches.as_slice().is_empty() {
                        return Err(LivingDeterministicFaultV2::Invariant);
                    }
                    self.advance(Retire);
                    return Ok(false);
                };
                let economy = if self
                    .economic_batches
                    .as_slice()
                    .first()
                    .is_some_and(|batch| batch.actor == actor.id)
                {
                    self.economic_batches.next()
                } else {
                    None
                };
                self.batch = Some(LivingFleetBatchV2 {
                    actor: actor.id,
                    economy,
                    fleets: Vec::new(),
                });
                self.alternative = 0;
                self.reset_alternative();
                return Ok(true);
            }
            Targets => {
                let actor = self.actor().id;
                if self.alternative > 0 && self.actor().status != LivingCivilizationStatusV2::Active
                {
                    self.stage = Selected;
                    return Ok(false);
                }
                let target = if self.alternative < 2 {
                    let Some(row) = state.observations.as_slice().get(self.target_index) else {
                        self.stage = Selected;
                        return Ok(false);
                    };
                    (row.civilization == actor
                        && if self.alternative == 0 {
                            row.owner.is_none()
                        } else {
                            row.owner.is_some_and(|enemy| {
                                self.wars.contains(&ordered_pair(actor, enemy))
                            }) && self.tick.0.saturating_sub(row.observed_tick.0)
                                <= OBSERVATION_FRESH_TICKS
                        })
                    .then_some(row.world)
                } else {
                    let Some(row) = state.colonies.as_slice().get(self.target_index) else {
                        self.stage = Selected;
                        return Ok(false);
                    };
                    (row.owner == actor
                        && if self.alternative == 2 {
                            self.attacked.contains(&(actor, row.world))
                        } else {
                            !self.escorts.contains(&(row.world, actor))
                        })
                    .then_some(row.world)
                };
                if let Some(target) = target {
                    self.target = target;
                    self.fleet_index = 0;
                    self.stage = Pairs;
                } else {
                    self.target_index += 1;
                }
                return Ok(true);
            }
            Pairs => {
                let Some(fleet) = state.fleets.as_slice().get(self.fleet_index) else {
                    self.target_index += 1;
                    self.stage = Targets;
                    return Ok(false);
                };
                let actor = self.actor().id;
                let summary = &self.summaries[&fleet.id];
                let eligible = fleet.owner == actor
                    && fleet.order.is_none()
                    && !self.targeted.contains(&fleet.id)
                    && if self.alternative == 0 {
                        summary.ark
                    } else {
                        summary.escorts > 0
                    };
                let origin = match fleet.location {
                    LivingFleetLocationV2::Docked { world } => Some(world),
                    _ => None,
                };
                let permitted = eligible
                    && origin.is_some_and(|world| {
                        if self.alternative == 0 {
                            if self.actor().status == LivingCivilizationStatusV2::Dormant {
                                summary.all_credited
                            } else {
                                self.own_colony(world, actor)
                                    && self.energy(world) >= summary.all_cost
                            }
                        } else {
                            world != self.target
                                && self.own_colony(world, actor)
                                && self.energy(world) >= self.role_cost(summary)
                        }
                    });
                if permitted {
                    self.pending_pair = Some((self.target, self.fleet_index));
                    self.start_path(origin.expect("docked candidate"), self.target);
                } else {
                    self.fleet_index += 1;
                }
                return Ok(true);
            }
            ExploreFleet => {
                let Some(fleet) = state.fleets.as_slice().get(self.fleet_index) else {
                    self.stage = Selected;
                    return Ok(false);
                };
                if fleet.owner == self.actor().id
                    && fleet.order.is_none()
                    && !self.targeted.contains(&fleet.id)
                    && self.summaries[&fleet.id].scout.is_some()
                    && matches!(fleet.location,LivingFleetLocationV2::Docked {world} if self.own_colony(world,self.actor().id))
                {
                    self.explore_fleet = Some(self.fleet_index);
                    self.target_index = 0;
                    self.stage = ExploreTargets;
                } else {
                    self.fleet_index += 1;
                }
                return Ok(true);
            }
            ExploreTargets => {
                let Some(world) = state.worlds.as_slice().get(self.target_index) else {
                    self.stage = Selected;
                    return Ok(false);
                };
                let fleet_index = self.explore_fleet.expect("first affordable scout");
                let LivingFleetLocationV2::Docked { world: origin } =
                    state.fleets.as_slice()[fleet_index].location
                else {
                    return Err(LivingDeterministicFaultV2::Invariant);
                };
                if world.id != origin
                    && self.observations.get(&(self.actor().id, world.id)) != Some(&self.tick)
                {
                    self.pending_pair = Some((world.id, fleet_index));
                    self.start_path(origin, world.id);
                } else {
                    self.target_index += 1;
                }
                return Ok(true);
            }
            Path => unreachable!("path handled by poll"),
            RetirePath => {
                if self.retired_path.pop().is_some() {
                    return Ok(true);
                }
                self.stage = PairComplete;
            }
            PairComplete => {
                let (target, fleet_index) = self.pending_pair.take().expect("pending pair");
                let fleet = state.fleets.as_slice()[fleet_index].id;
                if let Some(duration) = self.duration {
                    if self.explore {
                        let system = state.worlds.as_slice()[self.worlds[&target]].system;
                        let key = (
                            self.known.contains(&(self.actor().id, system)),
                            self.observations
                                .get(&(self.actor().id, target))
                                .map_or(0, |t| t.0),
                            duration,
                            system,
                            target,
                        );
                        if self.explore_key.is_none_or(|old| key < old) {
                            self.explore_key = Some(key);
                            self.choice = Some((target, fleet));
                        }
                    } else if self.choice.is_none_or(|old| (target, fleet) < old) {
                        self.choice = Some((target, fleet));
                    }
                }
                if self.explore {
                    self.target_index += 1;
                    self.stage = ExploreTargets;
                } else {
                    self.fleet_index += 1;
                    self.stage = Pairs;
                }
                return Ok(true);
            }
            Selected => {
                if self.alternative == 1
                    && !self.explore
                    && self.choice.is_none()
                    && self.actor().status == LivingCivilizationStatusV2::Active
                {
                    self.explore = true;
                    self.fleet_index = 0;
                    self.stage = ExploreFleet;
                    return Ok(false);
                }
                if self.choice.is_none() {
                    self.next_alternative();
                    return Ok(false);
                }
                self.copy = Some(super::step_job::LivingStateCopyJobV2::begin(
                    std::sync::Arc::clone(state),
                ));
                self.stage = Copy;
                return Ok(true);
            }
            Copy => unreachable!("copy shares the parent's remaining budget"),
            Assemble => {
                if self.assembly.is_none() {
                    let (target, fleet) = self.choice.expect("selected alternative");
                    let kind = match self.alternative {
                        0 => LivingFleetOrderKindV2::Settle,
                        1 if self.explore => LivingFleetOrderKindV2::Explore,
                        1 => LivingFleetOrderKindV2::Attack,
                        _ => LivingFleetOrderKindV2::Defend,
                    };
                    let ordinal = match self.alternative {
                        0 => 12,
                        1 if self.explore => 2,
                        1 => 13,
                        2 => 10,
                        _ => 15,
                    };
                    let scout = if self.explore {
                        self.summaries[&fleet].scout
                    } else {
                        None
                    };
                    let all = self.alternative == 0 || (!self.explore && self.full());
                    let change = FleetAssemblyChange::Order {
                        fleet,
                        target,
                        kind,
                        scout,
                        all,
                        split: self.identity(self.actor().id, ordinal),
                        tick: self.tick,
                    };
                    let rows = std::mem::take(
                        &mut self.candidate.as_mut().expect("candidate copy").fleets,
                    );
                    self.assembly = Some(LivingFleetAssemblyCursorV2::begin(rows, change));
                    return Ok(true);
                }
                let assembly = self.assembly.as_mut().expect("assembly cursor");
                if !assembly.ready() {
                    return assembly.visit();
                }
                self.candidate.as_mut().expect("candidate copy").fleets = self
                    .assembly
                    .as_mut()
                    .expect("ready assembly")
                    .take_ready_result()?;
                self.retirement
                    .start(self.assembly.take().expect("ready assembly"));
                self.stage = Decision;
                return Ok(true);
            }
            Decision => {
                if self.decision_cursor.is_none() {
                    self.decision_cursor = Some(LivingFleetDecisionCursorV2::begin());
                    return Ok(true);
                }
                let actor = self.actor().id;
                let cursor = self.decision_cursor.as_mut().expect("decision cursor");
                if cursor.stage != 4 {
                    return cursor.visit(
                        state,
                        self.candidate.as_ref().expect("candidate"),
                        &self.fleet_indexes,
                        actor,
                    );
                }
                if cursor.result.is_some() {
                    self.stage = Explain;
                } else {
                    self.retirement
                        .start(self.candidate.take().expect("unexplained candidate"));
                    self.decision_cursor = None;
                    self.next_alternative();
                    return Ok(true);
                }
            }
            Explain => {
                let (index, input) = self
                    .decision_cursor
                    .as_ref()
                    .expect("decision cursor")
                    .result
                    .expect("changed fleet");
                let order = self
                    .candidate
                    .as_ref()
                    .expect("candidate")
                    .fleets
                    .as_slice()[index]
                    .order
                    .as_ref()
                    .expect("changed order");
                let target = order
                    .target_world
                    .ok_or(LivingDeterministicFaultV2::Invariant)?;
                let actor = self.actor();
                let decision = indexed_fleet_decision(
                    actor,
                    input,
                    order,
                    self.tick,
                    self.economics.as_ref().expect("economics").inputs.digest,
                    self.observations.get(&(actor.id, target)).copied(),
                    self.attacked.contains(&(actor.id, target)),
                )?;
                let bytes = decision.action.canonical_bytes()?;
                let key = (
                    decision.action.ordinal(),
                    decision.target,
                    bytes.clone(),
                    usize::from(self.alternative),
                );
                let candidate = LivingFleetCandidateV2 {
                    state: self.candidate.take().expect("candidate"),
                    decision,
                    action_bytes: bytes,
                    deltas: Vec::new(),
                };
                if let Some((old_key, old)) = self.canonical.remove_entry(&key) {
                    self.retirement.start((RulesFleetKey(old_key), old));
                }
                self.canonical.insert(key, candidate);
                self.decision_cursor = None;
                self.next_alternative();
                return Ok(true);
            }
            Canonical => {
                if self.canonical.is_empty() {
                    self.advance(Ranking);
                    return Ok(false);
                }
                let count = self
                    .batch
                    .as_ref()
                    .expect("actor batch")
                    .economy
                    .as_ref()
                    .map_or(0, |batch| batch.economic_count);
                let ordinal = u16::try_from(
                    count
                        .checked_add(self.ordinal_index)
                        .ok_or(LivingDeterministicFaultV2::Arithmetic)?,
                )
                .map_err(|_| LivingDeterministicFaultV2::Arithmetic)?;
                let (key, mut candidate) = self.canonical.pop_first().expect("canonical candidate");
                candidate.decision.ordinal = ordinal;
                self.working = Some((key, candidate));
                self.ordinal_index += 1;
                self.advance(RekeyScan);
                return Ok(true);
            }
            RekeyScan => {
                let (_, candidate) = self.working.as_ref().expect("canonical work");
                if let Some(row) = candidate.state.fleets.as_slice().get(self.index) {
                    if !self.fleet_indexes.contains_key(&row.id) {
                        let ordinal = match candidate.decision.action {
                            LivingIntentActionV2::Explore { .. } => 14,
                            LivingIntentActionV2::Settle { .. } => 12,
                            LivingIntentActionV2::Attack { .. } => 13,
                            LivingIntentActionV2::Defend { .. } => {
                                if self.attacked.contains(&(
                                    candidate.decision.actor,
                                    candidate.decision.target.expect("fleet target"),
                                )) {
                                    10
                                } else {
                                    15
                                }
                            }
                            _ => return Err(LivingDeterministicFaultV2::Invariant),
                        };
                        let change = FleetAssemblyChange::Rekey {
                            fleet: row.id,
                            id: self.identity(candidate.decision.actor, ordinal),
                        };
                        let rows = std::mem::take(
                            &mut self
                                .working
                                .as_mut()
                                .expect("canonical work")
                                .1
                                .state
                                .fleets,
                        );
                        self.assembly = Some(LivingFleetAssemblyCursorV2::begin(rows, change));
                        self.stage = Rekey;
                    } else {
                        self.index += 1;
                    }
                    return Ok(true);
                }
                self.delta = Some(LivingFleetDeltaCursorV2::begin());
                self.stage = Delta;
                return Ok(true);
            }
            Rekey => {
                let assembly = self.assembly.as_mut().expect("rekey cursor");
                if !assembly.ready() {
                    return assembly.visit();
                }
                self.working
                    .as_mut()
                    .expect("canonical work")
                    .1
                    .state
                    .fleets = self
                    .assembly
                    .as_mut()
                    .expect("ready rekey")
                    .take_ready_result()?;
                self.retirement
                    .start(self.assembly.take().expect("ready rekey"));
                // An alternative changes one input fleet and creates at most
                // one new fleet. Scan finished before rekeying that sole owner.
                self.stage = Delta;
                return Ok(true);
            }
            Delta => {
                if self.delta.is_none() {
                    self.delta = Some(LivingFleetDeltaCursorV2::begin());
                    return Ok(true);
                }
                let delta = self.delta.as_mut().expect("delta cursor");
                if delta.stage != 4 {
                    return Ok(delta.visit(
                        state,
                        &self.working.as_ref().expect("canonical work").1.state,
                    ));
                }
                let (key, mut candidate) = self.working.take().expect("canonical work");
                let previous_deltas = std::mem::replace(
                    &mut candidate.deltas,
                    self.delta
                        .as_mut()
                        .expect("ready delta")
                        .take_ready_result()?,
                );
                let delta = self.delta.take().expect("ready delta");
                let priority = if matches!(
                    candidate.decision.action,
                    LivingIntentActionV2::Defend { .. }
                ) && self.attacked.contains(&(
                    candidate.decision.actor,
                    candidate.decision.target.expect("fleet target"),
                )) {
                    0
                } else {
                    2
                };
                let key = (
                    priority,
                    std::cmp::Reverse(candidate.decision.score.0),
                    candidate.decision.target,
                    key,
                );
                let old = self
                    .ranking
                    .remove_entry(&key)
                    .map(|(key, value)| (RulesFleetKey(key.3), value));
                self.ranking.insert(key, candidate);
                self.retirement.start((delta, (previous_deltas, old)));
                self.advance(Canonical);
                return Ok(true);
            }
            Ranking => {
                if let Some((key, candidate)) = self.ranking.pop_first() {
                    self.retirement.start(RulesFleetKey(key.3));
                    self.batch
                        .as_mut()
                        .expect("actor batch")
                        .fleets
                        .push(candidate);
                    return Ok(true);
                }
                self.batches.push(self.batch.take().expect("actor batch"));
                self.actor_index += 1;
                self.advance(Actors);
                return Ok(true);
            }
            Retire => {
                let consumed = match self.retire_index {
                    0 => self.worlds.pop_first().is_some(),
                    1 => self.colonies.pop_first().is_some(),
                    2 => self.actors.pop_first().is_some(),
                    3 => self.fleet_indexes.pop_first().is_some(),
                    4 => self.summaries.pop_first().is_some(),
                    5 => self.targeted.pop_first().is_some(),
                    6 => self.wars.pop_first().is_some(),
                    7 => self.escorts.pop_first().is_some(),
                    8 => self.attacked.pop_first().is_some(),
                    9 => self.observations.pop_first().is_some(),
                    10 => self.known.pop_first().is_some(),
                    11 => std::sync::Arc::get_mut(&mut self.graph)
                        .expect("retained private graph")
                        .lanes
                        .pop()
                        .is_some(),
                    12 => std::sync::Arc::get_mut(&mut self.graph)
                        .expect("retained private graph")
                        .systems
                        .pop()
                        .is_some(),
                    _ => {
                        self.stage = Ready;
                        false
                    }
                };
                if consumed {
                    return Ok(true);
                }
                self.retire_index += 1;
            }
            Ready => {}
        }
        Ok(false)
    }
    fn finish(
        &mut self,
    ) -> Result<LivingFleetEnumerationV2, super::step_job::LivingLifecycleJobErrorV2> {
        use super::step_job::LivingLifecycleJobErrorV2 as Error;
        if let Some(error) = self.path_fault {
            return Err(error.public());
        }
        if self.cancelled {
            return Err(Error::Cancelled);
        }
        if let Some(error) = &self.fault {
            return Err(error.clone());
        }
        if self.stage != FleetEnumerationStage::Ready
            || self.delivered
            || !self.retirement.is_empty()
            || self.economics.is_none()
        {
            return Err(Error::NotReady);
        }
        let economics = self.economics.take().ok_or(Error::NotReady)?;
        self.delivered = true;
        Ok(LivingFleetEnumerationV2 {
            economics: economics.into(),
            graph: std::sync::Arc::clone(&self.graph).into(),
            batches: std::mem::take(&mut self.batches),
        })
    }
}

fn indexed_fleet_decision(
    actor: &LivingCivilizationV2,
    input: LivingEntityIdV2,
    order: &LivingFleetOrderV2,
    tick: LivingTickV2,
    digest: super::ids::LivingStateDigestV2,
    observed_tick: Option<LivingTickV2>,
    attacked: bool,
) -> Result<LivingIntentDecisionV2, LivingDeterministicFaultV2> {
    let kind = order.kind;
    let target = order
        .target_world
        .ok_or(LivingDeterministicFaultV2::Invariant)?;
    let policy = actor.policy.0.as_str();
    let (action, score, bonus) = match kind {
        LivingFleetOrderKindV2::Settle => (
            LivingIntentActionV2::Settle {
                fleet: input,
                destination: target,
            },
            60,
            if policy == "expansionist" { 20 } else { 0 },
        ),
        LivingFleetOrderKindV2::Attack => (
            LivingIntentActionV2::Attack {
                fleet: input,
                world: target,
            },
            60,
            0,
        ),
        LivingFleetOrderKindV2::Explore => (
            LivingIntentActionV2::Explore {
                fleet: input,
                destination: target,
            },
            50,
            0,
        ),
        LivingFleetOrderKindV2::Defend => (
            LivingIntentActionV2::Defend {
                fleet: input,
                world: target,
            },
            if attacked { 60 } else { 40 },
            if policy == "guardian" { 20 } else { 0 },
        ),
        _ => return Err(LivingDeterministicFaultV2::Invariant),
    };
    let reasons = if bonus > 0 {
        vec![LivingDecisionReasonV2::PolicyScore {
            policy: actor.policy.0.clone(),
            component: slug(if kind == LivingFleetOrderKindV2::Defend {
                "defend"
            } else {
                "settlement"
            })
            .map_err(|_| LivingDeterministicFaultV2::Invariant)?,
            value: LivingDecisionScoreV2(bonus),
        }]
    } else {
        Vec::new()
    };
    Ok(LivingIntentDecisionV2 {
        phase: LivingPhaseV2::GenerateIntents,
        actor: actor.id,
        ordinal: 0,
        action,
        target: Some(target),
        score: LivingDecisionScoreV2(score + bonus),
        tier: if actor.status == LivingCivilizationStatusV2::Dormant {
            LivingDecisionTierV2::Recovery
        } else {
            LivingDecisionTierV2::Ordinary
        },
        snapshot_tick: tick,
        snapshot_digest: digest,
        observed_tick,
        outcome: LivingDecisionOutcomeV2::Rejected,
        reasons,
    })
}

// Rules ownership adapters share the reviewed phase/state/catalog substrate.
use super::step_job::{
    LivingPhaseRetirementJobV2, LivingStateRetirementPolicyV2, RetirementFields, RetirementPoll,
    retirement_node,
};

struct RulesOwner<T>(Option<T>);
impl<T> From<T> for RulesOwner<T> {
    fn from(value: T) -> Self {
        Self(Some(value))
    }
}
impl<T: Default> Default for RulesOwner<T> {
    fn default() -> Self {
        Self(Some(T::default()))
    }
}
impl<T> std::ops::Deref for RulesOwner<T> {
    type Target = T;
    fn deref(&self) -> &T {
        self.0.as_ref().expect("retained Rules owner")
    }
}
impl<T> std::ops::DerefMut for RulesOwner<T> {
    fn deref_mut(&mut self) -> &mut T {
        self.0.as_mut().expect("retained Rules owner")
    }
}
impl<'a, T: RetirementFields<'a>> RetirementFields<'a> for RulesOwner<T> {
    fn retirement_child(
        &mut self,
        index: usize,
        policy: LivingStateRetirementPolicyV2,
    ) -> Option<Box<dyn RetirementPoll + 'a>> {
        (index == 0).then(|| retirement_node(self.0.take(), policy))
    }
}
// u8 has no nested destructor. Count each independently owned backing allocation.
struct RulesByteBuffer(Vec<u8>);
impl<'a> RetirementFields<'a> for RulesByteBuffer {
    fn retirement_child(
        &mut self,
        _: usize,
        _: LivingStateRetirementPolicyV2,
    ) -> Option<Box<dyn RetirementPoll + 'a>> {
        let _ = &self.0;
        None
    }
}
struct RulesFleetKey(FleetCanonicalKey);
impl<'a> RetirementFields<'a> for RulesFleetKey {
    fn retirement_child(
        &mut self,
        index: usize,
        policy: LivingStateRetirementPolicyV2,
    ) -> Option<Box<dyn RetirementPoll + 'a>> {
        (index == 0)
            .then(|| retirement_node(RulesByteBuffer(std::mem::take(&mut self.0.2)), policy))
    }
}
#[allow(dead_code)]
struct RulesFleetCanonical(std::collections::BTreeMap<FleetCanonicalKey, LivingFleetCandidateV2>);
impl<'a> RetirementFields<'a> for RulesFleetCanonical {
    fn retirement_child(
        &mut self,
        _: usize,
        policy: LivingStateRetirementPolicyV2,
    ) -> Option<Box<dyn RetirementPoll + 'a>> {
        self.0
            .pop_first()
            .map(|(key, value)| retirement_node((RulesFleetKey(key), value), policy))
    }
}
#[allow(dead_code)]
struct RulesFleetRanking(std::collections::BTreeMap<FleetRankingKey, LivingFleetCandidateV2>);
impl<'a> RetirementFields<'a> for RulesFleetRanking {
    fn retirement_child(
        &mut self,
        _: usize,
        policy: LivingStateRetirementPolicyV2,
    ) -> Option<Box<dyn RetirementPoll + 'a>> {
        self.0
            .pop_first()
            .map(|(key, value)| retirement_node((RulesFleetKey(key.3), value), policy))
    }
}
macro_rules! rules_retirement_leaves {
    ($($ty:ty),* $(,)?)=>{$(impl<'a> RetirementFields<'a> for $ty {
        const LEAF:bool=true;
        fn retirement_child(&mut self,_:usize,_:LivingStateRetirementPolicyV2)->Option<Box<dyn RetirementPoll+'a>> { None }
    })*};
}
rules_retirement_leaves!(
    LivingInventoryV2,
    super::catalog::LivingHazardEffectV2,
    LivingEconomicActionV2,
    LivingEconomicCandidateV2,
    LivingReceiverDemandRequestV2,
    LivingFleetDeltaV2,
    LivingFleetDecisionCursorV2,
    FleetEnumerationSummary,
    EconomicCanonicalKey,
    EconomicRankingKey,
    [usize; 3]
);

impl<'a> RetirementFields<'a> for LivingForecastInputJobV2 {
    fn retirement_child(
        &mut self,
        index: usize,
        policy: LivingStateRetirementPolicyV2,
    ) -> Option<Box<dyn RetirementPoll + 'a>> {
        match index {
            0 => Some(retirement_node(
                std::mem::take(&mut self.retirement),
                policy,
            )),
            1 => Some(retirement_node(
                std::mem::take(&mut self.copy),
                LivingStateRetirementPolicyV2::ReleaseHandle,
            )),
            2 => Some(retirement_node(std::mem::take(&mut self.state), policy)),
            3 => Some(retirement_node(
                std::mem::take(&mut self.world_owners),
                policy,
            )),
            4 => Some(retirement_node(
                std::mem::take(&mut self.idle_facilities),
                policy,
            )),
            5 => Some(retirement_node(std::mem::take(&mut self.routes), policy)),
            6 => Some(retirement_node(
                std::mem::take(&mut self.facilities),
                policy,
            )),
            7 => Some(retirement_node(std::mem::take(&mut self.builds), policy)),
            8 => Some(retirement_node(std::mem::take(&mut self.hull_jobs), policy)),
            9 => Some(retirement_node(std::mem::take(&mut self.colonies), policy)),
            10 => Some(retirement_node(
                std::mem::take(&mut self.retired_reasons),
                policy,
            )),
            11 => Some(retirement_node(
                std::mem::take(&mut self.pending_route),
                policy,
            )),
            12 => Some(retirement_node(
                std::mem::take(&mut self.pending_facility),
                policy,
            )),
            13 => Some(retirement_node(
                std::mem::take(&mut self.pending_build),
                policy,
            )),
            14 => Some(retirement_node(
                std::mem::take(&mut self.pending_hull_job),
                policy,
            )),
            15 => Some(retirement_node(
                std::mem::take(&mut self.pending_colony),
                policy,
            )),
            16 => Some(retirement_node(self.source_owner.0.take(), policy)),
            _ => None,
        }
    }
}
#[allow(dead_code)]
impl<'a> LivingForecastInputJobV2 {
    fn into_rules_retirement(
        self,
        policy: LivingStateRetirementPolicyV2,
    ) -> LivingPhaseRetirementJobV2<'a, Self> {
        LivingPhaseRetirementJobV2::begin(self, policy)
    }
}

impl<'a> RetirementFields<'a> for LivingForecastInputV2 {
    fn retirement_child(
        &mut self,
        index: usize,
        policy: LivingStateRetirementPolicyV2,
    ) -> Option<Box<dyn RetirementPoll + 'a>> {
        match index {
            0 => Some(retirement_node(std::mem::take(&mut self.state), policy)),
            1 => Some(retirement_node(
                std::mem::take(&mut self.idle_facilities),
                policy,
            )),
            2 => Some(retirement_node(self.retired_source.0.take(), policy)),
            _ => None,
        }
    }
}
#[allow(dead_code)]
impl<'a> LivingForecastInputV2 {
    fn into_rules_retirement(
        self,
        policy: LivingStateRetirementPolicyV2,
    ) -> LivingPhaseRetirementJobV2<'a, Self> {
        LivingPhaseRetirementJobV2::begin(self, policy)
    }
}

impl<'a> RetirementFields<'a> for LivingScheduledForecastJobV2<'a> {
    fn retirement_child(
        &mut self,
        index: usize,
        policy: LivingStateRetirementPolicyV2,
    ) -> Option<Box<dyn RetirementPoll + 'a>> {
        match index {
            0 => Some(retirement_node(
                std::mem::take(&mut self.retirement),
                policy,
            )),
            1 => Some(retirement_node(
                std::mem::take(&mut self.preparation),
                LivingStateRetirementPolicyV2::ReleaseHandle,
            )),
            2 => Some(retirement_node(
                std::mem::take(&mut self.scheduled),
                LivingStateRetirementPolicyV2::ReleaseHandle,
            )),
            3 => Some(retirement_node(
                std::mem::take(&mut self.production),
                LivingStateRetirementPolicyV2::ReleaseHandle,
            )),
            4 => Some(retirement_node(
                std::mem::take(&mut self.dispatch),
                LivingStateRetirementPolicyV2::ReleaseHandle,
            )),
            5 => Some(retirement_node(std::mem::take(&mut self.state), policy)),
            6 => Some(retirement_node(std::mem::take(&mut self.events), policy)),
            7 => Some(retirement_node(
                std::mem::take(&mut self.idle_input),
                policy,
            )),
            8 => Some(retirement_node(std::mem::take(&mut self.idle), policy)),
            9 => Some(retirement_node(
                std::mem::take(&mut self.idle_output),
                policy,
            )),
            10 => Some(retirement_node(std::mem::take(&mut self.due), policy)),
            11 => Some(retirement_node(std::mem::take(&mut self.repairs), policy)),
            12 => Some(retirement_node(
                std::mem::take(&mut self.projection),
                policy,
            )),
            13 => Some(retirement_node(std::mem::take(&mut self.hulls), policy)),
            14 => Some(retirement_node(std::mem::take(&mut self.worlds), policy)),
            15 => Some(retirement_node(std::mem::take(&mut self.graph), policy)),
            16 => Some(retirement_node(std::mem::take(&mut self.lanes), policy)),
            17 => Some(retirement_node(
                std::mem::take(&mut self.hazard_effects),
                policy,
            )),
            18 => Some(retirement_node(std::mem::take(&mut self.trade), policy)),
            19 => Some(retirement_node(std::mem::take(&mut self.search), policy)),
            20 => Some(retirement_node(
                std::mem::take(&mut self.inventories),
                policy,
            )),
            21 => Some(retirement_node(std::mem::take(&mut self.deficits), policy)),
            22 => Some(retirement_node(
                std::mem::take(&mut self.deficit_set),
                policy,
            )),
            23 => Some(retirement_node(
                std::mem::take(&mut self.retired_edges),
                policy,
            )),
            24 => Some(retirement_node(self.source.0.take(), policy)),
            25 => Some(retirement_node(self.catalog.0.take(), policy)),
            26 => Some(retirement_node(
                std::mem::take(&mut self.repair_exclusions),
                policy,
            )),
            _ => None,
        }
    }
}
#[allow(dead_code)]
impl<'a> LivingScheduledForecastJobV2<'a> {
    fn into_rules_retirement(
        self,
        policy: LivingStateRetirementPolicyV2,
    ) -> LivingPhaseRetirementJobV2<'a, Self> {
        LivingPhaseRetirementJobV2::begin(self, policy)
    }
}

impl<'a> RetirementFields<'a> for LivingScheduledForecastV2 {
    fn retirement_child(
        &mut self,
        index: usize,
        policy: LivingStateRetirementPolicyV2,
    ) -> Option<Box<dyn RetirementPoll + 'a>> {
        match index {
            0 => Some(retirement_node(
                std::mem::take(&mut self.inventories),
                policy,
            )),
            1 => Some(retirement_node(std::mem::take(&mut self.deficits), policy)),
            2 => Some(retirement_node(
                std::mem::take(&mut self.idle_facilities),
                policy,
            )),
            3 => Some(retirement_node(
                std::mem::take(&mut self.final_state),
                policy,
            )),
            _ => None,
        }
    }
}
#[allow(dead_code)]
impl<'a> LivingScheduledForecastV2 {
    fn into_rules_retirement(
        self,
        policy: LivingStateRetirementPolicyV2,
    ) -> LivingPhaseRetirementJobV2<'a, Self> {
        LivingPhaseRetirementJobV2::begin(self, policy)
    }
}

impl<'a> RetirementFields<'a> for LivingIntentInputsV2 {
    fn retirement_child(
        &mut self,
        index: usize,
        policy: LivingStateRetirementPolicyV2,
    ) -> Option<Box<dyn RetirementPoll + 'a>> {
        match index {
            0 => Some(retirement_node(std::mem::take(&mut self.forecasts), policy)),
            1 => Some(retirement_node(std::mem::take(&mut self.requests), policy)),
            2 => Some(retirement_node(self.hash_owner.0.take(), policy)),
            3 => Some(retirement_node(self.snapshot.0.take(), policy)),
            _ => None,
        }
    }
}
#[allow(dead_code)]
impl<'a> LivingIntentInputsV2 {
    fn into_rules_retirement(
        self,
        policy: LivingStateRetirementPolicyV2,
    ) -> LivingPhaseRetirementJobV2<'a, Self> {
        LivingPhaseRetirementJobV2::begin(self, policy)
    }
}

impl<'a> RetirementFields<'a> for LivingIntentInputsJobV2 {
    fn retirement_child(
        &mut self,
        index: usize,
        policy: LivingStateRetirementPolicyV2,
    ) -> Option<Box<dyn RetirementPoll + 'a>> {
        match index {
            0 => Some(retirement_node(
                std::mem::take(&mut self.retirement),
                policy,
            )),
            1 => Some(retirement_node(
                std::mem::take(&mut self.copy),
                LivingStateRetirementPolicyV2::ReleaseHandle,
            )),
            2 => Some(retirement_node(
                std::mem::take(&mut self.hash),
                LivingStateRetirementPolicyV2::ReleaseHandle,
            )),
            3 => Some(retirement_node(
                std::mem::take(&mut self.child),
                LivingStateRetirementPolicyV2::ReleaseHandle,
            )),
            4 => Some(retirement_node(std::mem::take(&mut self.forecasts), policy)),
            5 => Some(retirement_node(std::mem::take(&mut self.requests), policy)),
            6 => Some(retirement_node(std::mem::take(&mut self.owners), policy)),
            7 => Some(retirement_node(std::mem::take(&mut self.stocks), policy)),
            8 => Some(retirement_node(
                std::mem::take(&mut self.hash_owner),
                policy,
            )),
            9 => Some(retirement_node(self.source.0.take(), policy)),
            10 => Some(retirement_node(
                self.catalog
                    .0
                    .take()
                    .map(super::step_job::LivingPhaseCatalogV2::Owned),
                policy,
            )),
            _ => None,
        }
    }
}
#[allow(dead_code)]
impl<'a> LivingIntentInputsJobV2 {
    fn into_rules_retirement(
        self,
        policy: LivingStateRetirementPolicyV2,
    ) -> LivingPhaseRetirementJobV2<'a, Self> {
        LivingPhaseRetirementJobV2::begin(self, policy)
    }
}

impl<'a> RetirementFields<'a> for LivingEconomicBatchV2 {
    fn retirement_child(
        &mut self,
        index: usize,
        policy: LivingStateRetirementPolicyV2,
    ) -> Option<Box<dyn RetirementPoll + 'a>> {
        match index {
            0 => Some(retirement_node(std::mem::take(&mut self.ranked), policy)),
            1 => Some(retirement_node(
                std::mem::take(&mut self.explanations),
                policy,
            )),
            _ => None,
        }
    }
}
#[allow(dead_code)]
impl<'a> LivingEconomicBatchV2 {
    fn into_rules_retirement(
        self,
        policy: LivingStateRetirementPolicyV2,
    ) -> LivingPhaseRetirementJobV2<'a, Self> {
        LivingPhaseRetirementJobV2::begin(self, policy)
    }
}

impl<'a> RetirementFields<'a> for LivingEconomicEnumerationV2 {
    fn retirement_child(
        &mut self,
        index: usize,
        policy: LivingStateRetirementPolicyV2,
    ) -> Option<Box<dyn RetirementPoll + 'a>> {
        match index {
            0 => Some(retirement_node(std::mem::take(&mut self.batches), policy)),
            1 => Some(retirement_node(self.inputs.0.take(), policy)),
            2 => Some(retirement_node(self.graph.0.take(), policy)),
            3 => Some(retirement_node(
                self.catalog
                    .0
                    .take()
                    .map(super::step_job::LivingPhaseCatalogV2::Owned),
                policy,
            )),
            _ => None,
        }
    }
}
#[allow(dead_code)]
impl<'a> LivingEconomicEnumerationV2 {
    fn into_rules_retirement(
        self,
        policy: LivingStateRetirementPolicyV2,
    ) -> LivingPhaseRetirementJobV2<'a, Self> {
        LivingPhaseRetirementJobV2::begin(self, policy)
    }
}

impl<'a> RetirementFields<'a> for LivingEconomicEnumerationJobV2 {
    fn retirement_child(
        &mut self,
        index: usize,
        policy: LivingStateRetirementPolicyV2,
    ) -> Option<Box<dyn RetirementPoll + 'a>> {
        // Exhaustive inventory: owner slots below, fixed controls remain in the root.
        let Self {
            retirement: _,
            catalog: _,
            inputs: _,
            tick: _,
            graph: _,
            worlds: _,
            industries: _,
            hull_costs: _,
            wars: _,
            trade: _,
            observations: _,
            projected: _,
            deficits: _,
            idle: _,
            actors: _,
            counts: _,
            escorts: _,
            stage: _,
            index: _,
            nested: _,
            forecast_index: _,
            actor_index: _,
            colony_index: _,
            world_index: _,
            colony_count: _,
            facilities: _,
            has: _,
            enabled: _,
            shipyard: _,
            construction_job: _,
            hull_job: _,
            deposit: _,
            threatened: _,
            scrap: _,
            building_free: _,
            hull_index: _,
            ark: _,
            resource_index: _,
            source_index: _,
            request_index: _,
            adjacent: _,
            adjacent_home: _,
            supply: _,
            first_route: _,
            working_route: _,
            path: _,
            retired_path: _,
            path_found: _,
            path_resume: _,
            emission: _,
            canonical: _,
            ranking: _,
            previous: _,
            failed_candidate: _,
            batch: _,
            batches: _,
            cancelled: _,
            delivered: _,
            path_fault: _,
            fault: _,
            last_poll_visits: _,
            total_visits: _,
        } = self;
        match index {
            0 => Some(retirement_node(
                std::mem::take(&mut self.retirement),
                policy,
            )),
            1 => Some(retirement_node(
                std::mem::take(&mut self.path),
                LivingStateRetirementPolicyV2::ReleaseHandle,
            )),
            2 => Some(retirement_node(std::mem::take(&mut self.inputs), policy)),
            3 => Some(retirement_node(std::mem::take(&mut self.worlds), policy)),
            4 => Some(retirement_node(
                std::mem::take(&mut self.industries),
                policy,
            )),
            5 => Some(retirement_node(
                std::mem::take(&mut self.hull_costs),
                policy,
            )),
            6 => Some(retirement_node(std::mem::take(&mut self.wars), policy)),
            7 => Some(retirement_node(std::mem::take(&mut self.trade), policy)),
            8 => Some(retirement_node(
                std::mem::take(&mut self.observations),
                policy,
            )),
            9 => Some(retirement_node(std::mem::take(&mut self.projected), policy)),
            10 => Some(retirement_node(std::mem::take(&mut self.deficits), policy)),
            11 => Some(retirement_node(std::mem::take(&mut self.idle), policy)),
            12 => Some(retirement_node(std::mem::take(&mut self.actors), policy)),
            13 => Some(retirement_node(std::mem::take(&mut self.counts), policy)),
            14 => Some(retirement_node(std::mem::take(&mut self.escorts), policy)),
            15 => Some(retirement_node(
                std::mem::take(&mut self.facilities),
                policy,
            )),
            16 => Some(retirement_node(std::mem::take(&mut self.supply), policy)),
            17 => Some(retirement_node(
                std::mem::take(&mut self.retired_path),
                policy,
            )),
            18 => Some(retirement_node(std::mem::take(&mut self.canonical), policy)),
            19 => Some(retirement_node(std::mem::take(&mut self.ranking), policy)),
            20 => Some(retirement_node(std::mem::take(&mut self.previous), policy)),
            21 => Some(retirement_node(
                std::mem::take(&mut self.failed_candidate),
                policy,
            )),
            22 => Some(retirement_node(std::mem::take(&mut self.batch), policy)),
            23 => Some(retirement_node(std::mem::take(&mut self.batches), policy)),
            24 => Some(retirement_node(self.graph.0.take(), policy)),
            25 => Some(retirement_node(
                self.catalog
                    .0
                    .take()
                    .map(super::step_job::LivingPhaseCatalogV2::Owned),
                policy,
            )),
            _ => None,
        }
    }
}
#[allow(dead_code)]
impl<'a> LivingEconomicEnumerationJobV2 {
    fn into_rules_retirement(
        self,
        policy: LivingStateRetirementPolicyV2,
    ) -> LivingPhaseRetirementJobV2<'a, Self> {
        LivingPhaseRetirementJobV2::begin(self, policy)
    }
}

impl<'a> RetirementFields<'a> for LivingFleetAssemblyCursorV2 {
    fn retirement_child(
        &mut self,
        index: usize,
        policy: LivingStateRetirementPolicyV2,
    ) -> Option<Box<dyn RetirementPoll + 'a>> {
        match index {
            0 => Some(retirement_node(std::mem::take(&mut self.source), policy)),
            1 => Some(retirement_node(std::mem::take(&mut self.rows), policy)),
            2 => Some(retirement_node(std::mem::take(&mut self.selected), policy)),
            3 => Some(retirement_node(std::mem::take(&mut self.hulls), policy)),
            4 => Some(retirement_node(std::mem::take(&mut self.moving), policy)),
            5 => Some(retirement_node(std::mem::take(&mut self.remaining), policy)),
            6 => Some(retirement_node(std::mem::take(&mut self.ordered), policy)),
            7 => Some(retirement_node(std::mem::take(&mut self.retired), policy)),
            8 => Some(retirement_node(
                std::mem::take(&mut self.retired_hulls),
                policy,
            )),
            9 => Some(retirement_node(
                std::mem::take(&mut self.retired_path),
                policy,
            )),
            10 => Some(retirement_node(std::mem::take(&mut self.output), policy)),
            11 => Some(retirement_node(
                std::mem::take(&mut self.retirement),
                policy,
            )),
            12 => Some(retirement_node(
                std::mem::take(&mut self.pending_output),
                policy,
            )),
            13 => Some(retirement_node(
                std::mem::take(&mut self.pending_hull),
                policy,
            )),
            _ => None,
        }
    }
}
#[allow(dead_code)]
impl<'a> LivingFleetAssemblyCursorV2 {
    fn into_rules_retirement(
        self,
        policy: LivingStateRetirementPolicyV2,
    ) -> LivingPhaseRetirementJobV2<'a, Self> {
        LivingPhaseRetirementJobV2::begin(self, policy)
    }
}

impl<'a> RetirementFields<'a> for LivingFleetDeltaCursorV2 {
    fn retirement_child(
        &mut self,
        index: usize,
        policy: LivingStateRetirementPolicyV2,
    ) -> Option<Box<dyn RetirementPoll + 'a>> {
        match index {
            0 => Some(retirement_node(std::mem::take(&mut self.output), policy)),
            _ => None,
        }
    }
}
#[allow(dead_code)]
impl<'a> LivingFleetDeltaCursorV2 {
    fn into_rules_retirement(
        self,
        policy: LivingStateRetirementPolicyV2,
    ) -> LivingPhaseRetirementJobV2<'a, Self> {
        LivingPhaseRetirementJobV2::begin(self, policy)
    }
}

impl<'a> RetirementFields<'a> for LivingFleetCandidateV2 {
    const CUSTOM_NODE: bool = true;
    fn retirement_child(
        &mut self,
        _: usize,
        _: LivingStateRetirementPolicyV2,
    ) -> Option<Box<dyn RetirementPoll + 'a>> {
        unreachable!("candidate moves fixed roots")
    }
    fn retirement_node(
        self,
        policy: LivingStateRetirementPolicyV2,
    ) -> Box<dyn RetirementPoll + 'a> {
        retirement_node(
            (
                Some(self.state),
                (
                    self.decision,
                    (RulesByteBuffer(self.action_bytes), self.deltas),
                ),
            ),
            policy,
        )
    }
}
#[allow(dead_code)]
impl<'a> LivingFleetCandidateV2 {
    fn into_rules_retirement(
        self,
        policy: LivingStateRetirementPolicyV2,
    ) -> LivingPhaseRetirementJobV2<'a, Self> {
        LivingPhaseRetirementJobV2::begin(self, policy)
    }
}

impl<'a> RetirementFields<'a> for LivingFleetBatchV2 {
    fn retirement_child(
        &mut self,
        index: usize,
        policy: LivingStateRetirementPolicyV2,
    ) -> Option<Box<dyn RetirementPoll + 'a>> {
        match index {
            0 => Some(retirement_node(std::mem::take(&mut self.economy), policy)),
            1 => Some(retirement_node(std::mem::take(&mut self.fleets), policy)),
            _ => None,
        }
    }
}
#[allow(dead_code)]
impl<'a> LivingFleetBatchV2 {
    fn into_rules_retirement(
        self,
        policy: LivingStateRetirementPolicyV2,
    ) -> LivingPhaseRetirementJobV2<'a, Self> {
        LivingPhaseRetirementJobV2::begin(self, policy)
    }
}

impl<'a> RetirementFields<'a> for LivingFleetEnumerationV2 {
    fn retirement_child(
        &mut self,
        index: usize,
        policy: LivingStateRetirementPolicyV2,
    ) -> Option<Box<dyn RetirementPoll + 'a>> {
        match index {
            0 => Some(retirement_node(std::mem::take(&mut self.batches), policy)),
            1 => Some(retirement_node(self.economics.0.take(), policy)),
            2 => Some(retirement_node(self.graph.0.take(), policy)),
            _ => None,
        }
    }
}
#[allow(dead_code)]
impl<'a> LivingFleetEnumerationV2 {
    fn into_rules_retirement(
        self,
        policy: LivingStateRetirementPolicyV2,
    ) -> LivingPhaseRetirementJobV2<'a, Self> {
        LivingPhaseRetirementJobV2::begin(self, policy)
    }
}

impl<'a> RetirementFields<'a> for LivingFleetEnumerationJobV2 {
    fn retirement_child(
        &mut self,
        index: usize,
        policy: LivingStateRetirementPolicyV2,
    ) -> Option<Box<dyn RetirementPoll + 'a>> {
        // Exhaustive inventory: owner slots below, fixed controls remain in the root.
        let Self {
            retirement: _,
            economics: _,
            economic_batches: _,
            tick: _,
            seed: _,
            branch: _,
            graph: _,
            worlds: _,
            colonies: _,
            actors: _,
            fleet_indexes: _,
            summaries: _,
            targeted: _,
            wars: _,
            escorts: _,
            attacked: _,
            threat_cursor: _,
            observations: _,
            known: _,
            pending_fleets: _,
            stage: _,
            index: _,
            nested: _,
            actor_index: _,
            alternative: _,
            target_index: _,
            fleet_index: _,
            target: _,
            choice: _,
            explore: _,
            explore_fleet: _,
            explore_key: _,
            pending_pair: _,
            duration: _,
            path: _,
            retired_path: _,
            copy: _,
            assembly: _,
            candidate: _,
            decision_cursor: _,
            canonical: _,
            ranking: _,
            working: _,
            delta: _,
            ordinal_index: _,
            batch: _,
            batches: _,
            retire_index: _,
            cancelled: _,
            delivered: _,
            path_fault: _,
            fault: _,
            last_poll_visits: _,
            total_visits: _,
        } = self;
        match index {
            0 => Some(retirement_node(
                std::mem::take(&mut self.retirement),
                policy,
            )),
            1 => Some(retirement_node(
                std::mem::take(&mut self.path),
                LivingStateRetirementPolicyV2::ReleaseHandle,
            )),
            2 => Some(retirement_node(
                std::mem::take(&mut self.copy),
                LivingStateRetirementPolicyV2::ReleaseHandle,
            )),
            3 => Some(retirement_node(std::mem::take(&mut self.assembly), policy)),
            4 => Some(retirement_node(std::mem::take(&mut self.candidate), policy)),
            5 => Some(retirement_node(
                std::mem::take(&mut self.decision_cursor),
                policy,
            )),
            6 => Some(retirement_node(std::mem::take(&mut self.delta), policy)),
            7 => Some(retirement_node(
                RulesFleetCanonical(std::mem::take(&mut self.canonical)),
                policy,
            )),
            8 => Some(retirement_node(
                RulesFleetRanking(std::mem::take(&mut self.ranking)),
                policy,
            )),
            9 => Some(retirement_node(
                self.working
                    .take()
                    .map(|(key, value)| (RulesFleetKey(key), value)),
                policy,
            )),
            10 => Some(retirement_node(std::mem::take(&mut self.batch), policy)),
            11 => Some(retirement_node(std::mem::take(&mut self.batches), policy)),
            12 => Some(retirement_node(
                std::mem::take(&mut self.economic_batches),
                policy,
            )),
            13 => Some(retirement_node(std::mem::take(&mut self.worlds), policy)),
            14 => Some(retirement_node(std::mem::take(&mut self.colonies), policy)),
            15 => Some(retirement_node(std::mem::take(&mut self.actors), policy)),
            16 => Some(retirement_node(
                std::mem::take(&mut self.fleet_indexes),
                policy,
            )),
            17 => Some(retirement_node(std::mem::take(&mut self.summaries), policy)),
            18 => Some(retirement_node(std::mem::take(&mut self.targeted), policy)),
            19 => Some(retirement_node(std::mem::take(&mut self.wars), policy)),
            20 => Some(retirement_node(std::mem::take(&mut self.escorts), policy)),
            21 => Some(retirement_node(std::mem::take(&mut self.attacked), policy)),
            22 => Some(retirement_node(
                std::mem::take(&mut self.observations),
                policy,
            )),
            23 => Some(retirement_node(std::mem::take(&mut self.known), policy)),
            24 => Some(retirement_node(
                std::mem::take(&mut self.retired_path),
                policy,
            )),
            25 => Some(retirement_node(std::mem::take(&mut self.economics), policy)),
            26 => Some(retirement_node(self.graph.0.take(), policy)),
            _ => None,
        }
    }
}
#[allow(dead_code)]
impl<'a> LivingFleetEnumerationJobV2 {
    fn into_rules_retirement(
        self,
        policy: LivingStateRetirementPolicyV2,
    ) -> LivingPhaseRetirementJobV2<'a, Self> {
        LivingPhaseRetirementJobV2::begin(self, policy)
    }
}

impl<'a> RetirementFields<'a> for Box<LivingScheduledForecastJobV2<'a>> {
    fn retirement_child(
        &mut self,
        index: usize,
        policy: LivingStateRetirementPolicyV2,
    ) -> Option<Box<dyn RetirementPoll + 'a>> {
        (**self).retirement_child(index, policy)
    }
}

fn rules_ensure_append<T: super::wire::LivingKeyedV2>(
    output: &LivingSortedVecV2<T>,
    row: &T,
) -> Result<(), LivingDeterministicFaultV2> {
    if output
        .as_slice()
        .last()
        .is_some_and(|last| last.living_key() >= row.living_key())
    {
        return Err(super::wire::LivingWireErrorV2::KeyedOrder.into());
    }
    Ok(())
}

#[cfg(test)]
mod corrected_repair_forecast_tests {
    use super::*;
    #[test]
    fn corrected_repair_forecast_battery_demand_and_eligibility_are_independent() {
        let a = crate::living::builtin_living_sandbox_v2().unwrap();
        for mode in 0..5 {
            let mut source = a.state().clone();
            let colony = source.colonies.as_slice()[0].clone();
            let home = colony.world;
            let other = source
                .civilizations
                .as_slice()
                .iter()
                .find(|c| c.id != colony.owner)
                .unwrap()
                .id;
            source.routes = Default::default();
            source.shipments = Default::default();
            let mut fleet = source.fleets.as_slice()[0].clone();
            source.fleets = Default::default();
            source.worlds.find_mut(&home).unwrap().inventory.energy = 0;
            let world = if mode == 2 {
                source
                    .worlds
                    .as_slice()
                    .iter()
                    .find(|w| w.owner.is_none())
                    .unwrap()
                    .id
            } else {
                home
            };
            source.facilities = LivingSortedVecV2::new(vec![LivingFacilityV2 {
                id: LivingEntityIdV2([230; 16]),
                world,
                definition: LivingSlugV2::new("defense_battery").unwrap(),
                status: LivingFacilityStatusV2::Operational,
                paid_alloy: 40,
                hit_points: if mode == 3 { 40 } else { 39 },
                next_due: None,
                next_repair: Some(LivingTickV2(if mode == 4 { 1000 } else { 0 })),
                blocked_reasons: Default::default(),
                disabled: true,
            }])
            .unwrap();
            if mode == 1 {
                fleet.owner = other;
                fleet.location = LivingFleetLocationV2::Docked { world: home };
                fleet.order = None;
                let mut hull = fleet.hulls.as_slice()[0].clone();
                hull.definition = LivingSlugV2::new("scout").unwrap();
                hull.hit_points = 2;
                hull.return_credit = true;
                hull.next_repair = None;
                fleet.hulls = LivingSortedVecV2::new(vec![hull]).unwrap();
                source.fleets = LivingSortedVecV2::new(vec![fleet]).unwrap();
                source.wars = LivingSortedVecV2::new(vec![LivingWarV2 {
                    participant_a: colony.owner.min(other),
                    participant_b: colony.owner.max(other),
                    declarer: colony.owner,
                    start_tick: LivingTickV2(0),
                    end_tick: LivingTickV2(1000),
                }])
                .unwrap();
            }
            source.validate(a.catalog()).unwrap();
            let immutable = source.canonical_bytes().unwrap();
            let mut state = source.clone();
            let mut events = super::super::receipt::LivingPendingEventsV2::new();
            let ctx = LivingStepContextV2 {
                catalog: a.catalog(),
                genesis_seed: [0; 32],
                branch: a.branch(),
                tick: LivingTickV2(0),
                state: &mut state,
                events: &mut events,
                arrivals: Vec::new(),
                repair_exclusions: Default::default(),
            };
            // Normalized input intentionally omits foreign colonies; mode1's
            // foreign scout cannot retreat there and creates ordinary demand.
            if mode == 1 {
                let (normalized, _) =
                    reference_forecast_input(&source, colony.owner, LivingTickV2(0)).unwrap();
                assert!(
                    normalized
                        .colonies
                        .as_slice()
                        .iter()
                        .all(|c| c.owner == colony.owner)
                );
                // Separately exercise the actual post-scheduled snapshot seam,
                // where a valid foreign destination remains visible.
                let mut direct = LivingScheduledForecastJobV2::begin(
                    a.catalog(),
                    std::sync::Arc::new(source.clone()),
                    colony.owner,
                    LivingTickV2(0),
                    [0; 32],
                    a.branch(),
                );
                direct.retirement.start(direct.preparation.take());
                direct.state = source.clone();
                direct.tick = LivingTickV2(1);
                direct.projection = Some(
                    super::super::step_job::RepairEligibilityProjection::begin(direct.tick),
                );
                direct.stage = ScheduledForecastStage::Projection;
                while direct.stage != ScheduledForecastStage::Facilities {
                    direct.poll(1).unwrap();
                }
                assert_eq!(direct.state.facilities.as_slice()[0].hit_points, 39);
                assert!(direct.deficits.is_empty());
                direct.cancel();
                let mut retired = direct.into_rules_retirement(
                    super::super::step_job::LivingStateRetirementPolicyV2::ReleaseHandle,
                );
                while retired.poll(64).unwrap()
                    != super::super::step_job::LivingLifecycleProgressV2::Ready
                {}
            }
            let expected = reference_scheduled_forecast(&ctx, colony.owner, false).unwrap();
            assert_eq!(
                expected
                    .deficits
                    .contains(&(home, LivingResourceV2::Energy)),
                mode <= 1,
                "independent battery demand mode {mode}"
            );
            for schedule in [&[1][..], &[64][..], &[1024][..], &[1, 64, 7, 1024][..]] {
                let mut job = LivingScheduledForecastJobV2::begin(
                    a.catalog(),
                    std::sync::Arc::new(source.clone()),
                    colony.owner,
                    LivingTickV2(0),
                    [0; 32],
                    a.branch(),
                );
                let mut n = 0;
                loop {
                    let b = schedule[n % schedule.len()];
                    n += 1;
                    let before = job.total_visits;
                    let p = job.poll(b).unwrap();
                    assert_eq!(job.total_visits - before, job.last_poll_visits as u64);
                    assert!(job.last_poll_visits <= b);
                    if p == super::super::step_job::LivingLifecycleProgressV2::Ready {
                        break;
                    }
                    assert!(n < 1000000);
                }
                let (actual, _) = job.finish().unwrap();
                assert_eq!(actual, expected, "mode {mode}");
            }
            assert_eq!(source.canonical_bytes().unwrap(), immutable);
        }
    }
}

#[cfg(test)]
mod general_production_forecast_tests {
    use super::super::step_job::{
        LivingLifecycleJobErrorV2 as Error, LivingLifecycleProgressV2 as Progress,
        ProductionFailure,
    };
    use super::*;
    #[test]
    fn scheduled_forecast_cursor_production_rejected_extraction_retains_box_and_finite_cause() {
        let authority = builtin_living_sandbox_v2().unwrap();
        for trigger in [1, 64, 7, 1024] {
            let source = authority.state_snapshot();
            let mut job = LivingScheduledForecastJobV2::begin(
                authority.catalog(),
                source.clone(),
                source.civilizations.as_slice()[0].id,
                LivingTickV2(1),
                [0; 32],
                authority.branch(),
            );
            for _ in 0..100_000 {
                if job.stage == ScheduledForecastStage::Production && job.retirement.is_empty() {
                    break;
                }
                assert_eq!(job.poll(1).unwrap(), Progress::Pending);
            }
            let child = job.production.as_mut().expect("actual forecast production");
            for _ in 0..100_000 {
                if child.poll(64).unwrap() == Progress::Ready {
                    break;
                }
            }
            let output = child.take_ready_result().unwrap();
            let original = std::ptr::from_ref(child.as_ref());
            let before = job.total_visits();
            assert_eq!(
                job.poll(trigger),
                Err(Error::Fault(LivingDeterministicFaultV2::Invariant))
            );
            assert_eq!(job.total_visits() - before, 1);
            assert_eq!(job.last_poll_visits(), 1);
            assert_eq!(job.production_fault, Some(ProductionFailure::Invariant));
            assert!(job.fault.is_none() && job.stage == ScheduledForecastStage::Production);
            assert_eq!(
                std::ptr::from_ref(job.production.as_ref().unwrap().as_ref()),
                original
            );
            assert_eq!(job.poll(0), Err(Error::Budget));
            assert_eq!(job.last_poll_visits(), 0);
            assert!(matches!(
                job.take_ready_result(),
                Err(Error::Fault(LivingDeterministicFaultV2::Invariant))
            ));
            assert!(std::sync::Arc::ptr_eq(&job.source, &source));
            assert!(job.state.worlds.as_slice().is_empty());
            assert!(!output.0.worlds.as_slice().is_empty());
            job.cancel();
            assert!(matches!(job.take_ready_result(), Err(Error::Cancelled)));
            let mut r = job.into_rules_retirement(LivingStateRetirementPolicyV2::ReleaseHandle);
            for _ in 0..100_000 {
                if r.poll(64).unwrap() == Progress::Ready {
                    break;
                }
            }
            assert_eq!(r.poll(1).unwrap(), Progress::Ready);
        }
    }
}

// Due-intent reservation compares nested rows without cloning their payloads.
trait IntentReservationRow: LivingKeyedV2<Key = LivingEntityIdV2> {
    /// None means another nested record remains; Some is the completed equality.
    fn equal_visit(&self, other: &Self, index: usize) -> Option<bool>;
}
macro_rules! intent_flat_rows {
    ($($ty:ty),* $(,)?) => {$(impl IntentReservationRow for $ty {
        fn equal_visit(&self, other: &Self, _: usize) -> Option<bool> { Some(self == other) }
    })*};
}
intent_flat_rows!(
    LivingWorldV2,
    LivingHullJobV2,
    LivingConstructionJobV2,
    LivingFreightRouteV2
);
impl IntentReservationRow for LivingFacilityV2 {
    fn equal_visit(&self, other: &Self, index: usize) -> Option<bool> {
        if index == 0 {
            return (self.id != other.id
                || self.world != other.world
                || self.definition != other.definition
                || self.status != other.status
                || self.paid_alloy != other.paid_alloy
                || self.hit_points != other.hit_points
                || self.next_due != other.next_due
                || self.next_repair != other.next_repair
                || self.disabled != other.disabled
                || self.blocked_reasons.as_slice().len()
                    != other.blocked_reasons.as_slice().len())
            .then_some(false);
        }
        match (
            self.blocked_reasons.as_slice().get(index - 1),
            other.blocked_reasons.as_slice().get(index - 1),
        ) {
            (Some(a), Some(b)) if a == b => None,
            (None, None) => Some(true),
            _ => Some(false),
        }
    }
}
impl IntentReservationRow for LivingFleetV2 {
    fn equal_visit(&self, other: &Self, index: usize) -> Option<bool> {
        let header = |f: &Self| {
            f.order
                .as_ref()
                .map(|o| (o.kind, o.target_world, o.not_before_tick, o.lane_path.len()))
        };
        if index == 0 {
            return (self.id != other.id
                || self.owner != other.owner
                || self.location != other.location
                || header(self) != header(other)
                || self.hulls.as_slice().len() != other.hulls.as_slice().len())
            .then_some(false);
        }
        let paths = self.order.as_ref().map_or(0, |o| o.lane_path.len());
        if index <= paths {
            return (self.order.as_ref().unwrap().lane_path[index - 1]
                != other.order.as_ref().unwrap().lane_path[index - 1])
                .then_some(false);
        }
        match (
            self.hulls.as_slice().get(index - paths - 1),
            other.hulls.as_slice().get(index - paths - 1),
        ) {
            (Some(a), Some(b)) if a == b => None,
            (None, None) => Some(true),
            _ => Some(false),
        }
    }
}

/// Merge a proposal delta into an owned trial. Original input iterators, held
/// rows and displaced nested rows remain owned until counted retirement.
struct IntentReservationDomain<T: IntentReservationRow + RetirementFields<'static>> {
    proposed: std::vec::IntoIter<T>,
    current: std::vec::IntoIter<T>,
    proposal_row: Option<T>,
    current_row: Option<T>,
    output: LivingSortedVecV2<T>,
    storage: Option<super::step_job::LivingSortedStorageJobV2<'static, T>>,
    retirement: super::step_job::PhaseRetirementLane<'static>,
    before: usize,
    key: Option<LivingEntityIdV2>,
    stage: u8,
    nested: usize,
    conflict: bool,
}
impl<T: IntentReservationRow + RetirementFields<'static>> IntentReservationDomain<T> {
    fn begin(proposed: LivingSortedVecV2<T>, current: LivingSortedVecV2<T>) -> Self {
        let count = proposed
            .as_slice()
            .len()
            .saturating_add(current.as_slice().len());
        Self {
            proposed: proposed.into_vec().into_iter(),
            current: current.into_vec().into_iter(),
            proposal_row: None,
            current_row: None,
            output: Default::default(),
            storage: Some(super::step_job::LivingSortedStorageJobV2::begin_empty(
                count,
            )),
            retirement: Default::default(),
            before: 0,
            key: None,
            stage: 0,
            nested: 0,
            conflict: false,
        }
    }
    fn compare(a: Option<&T>, b: Option<&T>, nested: usize) -> Option<bool> {
        match (a, b) {
            (Some(a), Some(b)) => a.equal_visit(b, nested),
            (None, None) => Some(true),
            _ => Some(false),
        }
    }
    fn visit(
        &mut self,
        snapshot: &LivingSortedVecV2<T>,
    ) -> Result<(), super::step_job::LivingLifecycleJobErrorV2> {
        use super::step_job::LivingLifecycleJobErrorV2 as Error;
        if self.retirement.visit() {
            return Ok(());
        }
        match self.stage {
            0 => {
                let storage = self.storage.as_mut().unwrap();
                if !storage.ready() {
                    storage.poll(1).map_err(|_| Error::PreparationCapacity)?;
                } else {
                    self.output = storage
                        .take_ready_result()
                        .map_err(|_| Error::PreparationCapacity)?;
                    self.retirement.start(self.storage.take());
                    self.stage = 1;
                }
            }
            1 => {
                if self.proposal_row.is_none() {
                    self.proposal_row = self.proposed.next();
                }
                self.stage = 2;
            }
            2 => {
                if self.current_row.is_none() {
                    self.current_row = self.current.next();
                }
                self.stage = 3;
            }
            3 => {
                self.key = [
                    self.proposal_row.as_ref().map(LivingKeyedV2::living_key),
                    self.current_row.as_ref().map(LivingKeyedV2::living_key),
                    snapshot
                        .as_slice()
                        .get(self.before)
                        .map(LivingKeyedV2::living_key),
                ]
                .into_iter()
                .flatten()
                .min();
                self.stage = if self.key.is_none() { 9 } else { 4 };
                self.nested = 0;
            }
            4 | 5 => {
                let key = self.key.unwrap();
                let before = snapshot
                    .as_slice()
                    .get(self.before)
                    .filter(|r| r.living_key() == key);
                let other = if self.stage == 4 {
                    self.proposal_row.as_ref()
                } else {
                    self.current_row.as_ref()
                }
                .filter(|r| r.living_key() == key);
                if let Some(equal) = Self::compare(before, other, self.nested) {
                    if self.stage == 4 {
                        self.stage = if equal { 6 } else { 5 };
                    } else if equal {
                        self.stage = 7;
                    } else {
                        self.conflict = true;
                        self.stage = 9;
                    }
                    self.nested = 0;
                } else {
                    self.nested += 1;
                }
            }
            6 | 7 => {
                let key = self.key.unwrap();
                let (selected, discarded) = if self.stage == 6 {
                    (&mut self.current_row, &mut self.proposal_row)
                } else {
                    (&mut self.proposal_row, &mut self.current_row)
                };
                if selected.as_ref().is_some_and(|r| r.living_key() == key) {
                    self.output
                        .push_sorted_retained(selected)
                        .map_err(|e| Error::Fault(e.into()))?;
                }
                if discarded.as_ref().is_some_and(|r| r.living_key() == key) {
                    self.retirement.start(discarded.take());
                }
                self.stage = 8;
            }
            8 => {
                if snapshot
                    .as_slice()
                    .get(self.before)
                    .is_some_and(|r| Some(r.living_key()) == self.key)
                {
                    self.before += 1;
                }
                self.stage = 1;
            }
            _ => {}
        }
        Ok(())
    }
    fn ready(&self) -> bool {
        self.stage == 9 && self.retirement.is_empty()
    }
}
impl<'a, T: IntentReservationRow + RetirementFields<'static>> RetirementFields<'a>
    for IntentReservationDomain<T>
{
    #[cfg(test)]
    fn creator_allocations(&self, out: &mut Vec<(usize, usize, Option<usize>, usize)>) {
        self.proposed.creator_allocations(out);
        self.current.creator_allocations(out);
        self.proposal_row.creator_allocations(out);
        self.current_row.creator_allocations(out);
        self.output.creator_allocations(out);
        self.storage.creator_allocations(out);
        self.retirement.creator_allocations(out);
    }

    fn retirement_child(
        &mut self,
        index: usize,
        policy: LivingStateRetirementPolicyV2,
    ) -> Option<Box<dyn RetirementPoll + 'a>> {
        let Self {
            proposed: _,
            current: _,
            proposal_row: _,
            current_row: _,
            output: _,
            storage: _,
            retirement: _,
            before: _,
            key: _,
            stage: _,
            nested: _,
            conflict: _,
        } = self;
        match index {
            0 => Some(retirement_node(
                std::mem::take(&mut self.retirement),
                policy,
            )),
            1 => Some(retirement_node(self.storage.take(), policy)),
            2 => Some(retirement_node(
                std::mem::replace(&mut self.proposed, Vec::new().into_iter()),
                policy,
            )),
            3 => Some(retirement_node(
                std::mem::replace(&mut self.current, Vec::new().into_iter()),
                policy,
            )),
            4 => Some(retirement_node(self.proposal_row.take(), policy)),
            5 => Some(retirement_node(self.current_row.take(), policy)),
            6 => Some(retirement_node(std::mem::take(&mut self.output), policy)),
            _ => None,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum IntentStage {
    Inputs,
    Economics,
    Fleets,
    GraphSystems,
    GraphLanes,
    Actor,
    Economy,
    Fleet,
    Reasons,
    ProposalCopy,
    Apply,
    TrialCopy,
    Reserve,
    Validate,
    CapacityReason,
    Attempt,
    DecisionStore,
    AttemptCleanup,
    Sort,
    Dense,
    Remap,
    FinalCopy,
    Ready,
}

/// Private transactional due-intent phase. Enumeration always reads `source`;
/// only successful, prospectively validated trials replace `reserved`.
pub(crate) struct LivingDueIntentsJobV2 {
    catalog: RulesOwner<std::sync::Arc<ValidatedLivingCatalogPackV2>>,
    source: RulesOwner<std::sync::Arc<LivingGalaxyStateV2>>,
    reserved: Option<std::sync::Arc<LivingGalaxyStateV2>>,
    inputs: Option<LivingIntentInputsJobV2>,
    economics: Option<LivingEconomicEnumerationJobV2>,
    fleets: Option<LivingFleetEnumerationJobV2>,
    enumeration: Option<LivingFleetEnumerationV2>,
    batches: std::vec::IntoIter<LivingFleetBatchV2>,
    batch: Option<LivingFleetBatchV2>,
    economic_rows: std::vec::IntoIter<(u16, LivingEconomicCandidateV2)>,
    explanations: std::vec::IntoIter<LivingIntentDecisionV2>,
    fleet_rows: std::vec::IntoIter<LivingFleetCandidateV2>,
    economic: Option<LivingEconomicCandidateV2>,
    decision: Option<LivingIntentDecisionV2>,
    reason_storage: Option<super::step_job::LivingVecStorageJobV2<'static, LivingDecisionReasonV2>>,
    copy: Option<super::step_job::LivingStateCopyJobV2>,
    proposal: Option<LivingGalaxyStateV2>,
    trial: Option<LivingGalaxyStateV2>,
    validation_source: Option<std::sync::Arc<LivingGalaxyStateV2>>,
    validation: Option<super::step_job::LivingStateValidationJobV2>,
    validation_capacity: Option<LivingValidationErrorV2>,
    worlds: Option<IntentReservationDomain<LivingWorldV2>>,
    hull_jobs: Option<IntentReservationDomain<LivingHullJobV2>>,
    construction_jobs: Option<IntentReservationDomain<LivingConstructionJobV2>>,
    facilities: Option<IntentReservationDomain<LivingFacilityV2>>,
    fleet_delta: Option<IntentReservationDomain<LivingFleetV2>>,
    routes: Option<IntentReservationDomain<LivingFreightRouteV2>>,
    hull_insert: Option<super::step_job::LivingSortedInsertionJobV2<LivingHullJobV2>>,
    construction_insert:
        Option<super::step_job::LivingSortedInsertionJobV2<LivingConstructionJobV2>>,
    route_insert: Option<super::step_job::LivingSortedInsertionJobV2<LivingFreightRouteV2>>,
    facility_remove: Option<super::step_job::LivingPlannedFilterJobV2<LivingFacilityV2>>,
    path: Option<super::step_job::RetreatPathJob>,
    events: super::receipt::LivingPendingEventsV2,
    decisions: std::collections::BTreeMap<(u16, LivingEntityIdV2, u16), LivingIntentDecisionV2>,
    old_decisions: std::vec::IntoIter<LivingIntentDecisionV2>,
    decision_storage:
        Option<super::step_job::LivingVecStorageJobV2<'static, LivingIntentDecisionV2>>,
    remap: std::collections::BTreeMap<(u16, LivingEntityIdV2, u16), u16>,
    retirement: super::step_job::PhaseRetirementLane<'static>,
    tick: LivingTickV2,
    seed: [u8; 32],
    branch: super::ids::LivingBranchIdV2,
    stage: IntentStage,
    apply_stage: u8,
    domain: u8,
    index: usize,
    nested: usize,
    colony_index: usize,
    source_system: Option<LivingEntityIdV2>,
    destination_system: Option<LivingEntityIdV2>,
    permission: bool,
    war: bool,
    adjacent: bool,
    accepted_economy: bool,
    accepted_fleet: bool,
    is_economy: bool,
    accepted: bool,
    previous: Option<(u16, LivingEntityIdV2)>,
    next: u32,
    fault: Option<super::step_job::LivingLifecycleJobErrorV2>,
    cancelled: bool,
    delivered: bool,
    last_poll_visits: usize,
    total_visits: u64,
}
impl LivingDueIntentsJobV2 {
    pub(crate) fn begin(
        catalog: std::sync::Arc<ValidatedLivingCatalogPackV2>,
        state: LivingGalaxyStateV2,
        events: super::receipt::LivingPendingEventsV2,
        tick: LivingTickV2,
        seed: [u8; 32],
        branch: super::ids::LivingBranchIdV2,
    ) -> Self {
        let source = std::sync::Arc::new(state);
        Self {
            inputs: Some(LivingIntentInputsJobV2::begin(
                catalog.clone(),
                source.clone(),
                tick,
                seed,
                branch,
            )),
            catalog: catalog.into(),
            source: source.clone().into(),
            reserved: Some(source),
            economics: None,
            fleets: None,
            enumeration: None,
            batches: Vec::new().into_iter(),
            batch: None,
            economic_rows: Vec::new().into_iter(),
            explanations: Vec::new().into_iter(),
            fleet_rows: Vec::new().into_iter(),
            economic: None,
            decision: None,
            reason_storage: None,
            copy: None,
            proposal: None,
            trial: None,
            validation_source: None,
            validation: None,
            validation_capacity: None,
            worlds: None,
            hull_jobs: None,
            construction_jobs: None,
            facilities: None,
            fleet_delta: None,
            routes: None,
            hull_insert: None,
            construction_insert: None,
            route_insert: None,
            facility_remove: None,
            path: None,
            events,
            decisions: Default::default(),
            old_decisions: Vec::new().into_iter(),
            decision_storage: None,
            remap: Default::default(),
            retirement: Default::default(),
            tick,
            seed,
            branch,
            stage: IntentStage::Inputs,
            apply_stage: 0,
            domain: 0,
            index: 0,
            nested: 0,
            colony_index: 0,
            source_system: None,
            destination_system: None,
            permission: false,
            war: false,
            adjacent: false,
            accepted_economy: false,
            accepted_fleet: false,
            is_economy: false,
            accepted: false,
            previous: None,
            next: 0,
            fault: None,
            cancelled: false,
            delivered: false,
            last_poll_visits: 0,
            total_visits: 0,
        }
    }
    #[cfg(test)]
    pub(crate) fn test_stage_key(&self) -> (u8, bool) {
        (
            match self.stage {
                IntentStage::Apply => 100 + self.apply_stage.min(30),
                IntentStage::Reserve => 140 + self.domain,
                _ => self.stage as u8,
            },
            !self.retirement.is_empty(),
        )
    }
    // Enumeration, copy, storage, path and prospective validation produce
    // finite diagnostics. Replay belongs to creator replay, outside this phase;
    // preserve an unexpected recursive owner for retirement without cloning it.
    fn reported_error(
        error: &super::step_job::LivingLifecycleJobErrorV2,
    ) -> super::step_job::LivingLifecycleJobErrorV2 {
        use super::step_job::LivingLifecycleJobErrorV2 as Error;
        use LivingDeterministicFaultV2 as Fault;
        match error {
            Error::Budget => Error::Budget,
            Error::Cancelled => Error::Cancelled,
            Error::NotReady => Error::NotReady,
            Error::PreparationCapacity => Error::PreparationCapacity,
            Error::Fault(Fault::Replay { .. }) => Error::Fault(Fault::Invariant),
            Error::Fault(
                fault @ (Fault::ViewingHistory
                | Fault::Invariant
                | Fault::Capacity
                | Fault::Arithmetic
                | Fault::Validation(_)
                | Fault::Wire(_)
                | Fault::Receipt(_)
                | Fault::Decision(_)
                | Fault::EventFacts(_)),
            ) => Error::Fault(fault.clone()),
        }
    }
    pub(crate) fn last_poll_visits(&self) -> usize {
        self.last_poll_visits
    }
    #[cfg(test)]
    fn cancel(&mut self) {
        self.cancelled = true;
    }
    pub(crate) fn poll(
        &mut self,
        budget: usize,
    ) -> Result<
        super::step_job::LivingLifecycleProgressV2,
        super::step_job::LivingLifecycleJobErrorV2,
    > {
        use super::step_job::{
            LivingLifecycleJobErrorV2 as Error, LivingLifecycleProgressV2 as Progress,
        };
        self.last_poll_visits = 0;
        if budget == 0 || budget > 1024 {
            return Err(Error::Budget);
        }
        if let Some(error) = &self.fault {
            return Err(Self::reported_error(error));
        }
        if self.cancelled {
            return Err(Error::Cancelled);
        }
        while self.last_poll_visits < budget
            && (self.stage != IntentStage::Ready || !self.retirement.is_empty())
        {
            if self.retirement.visit() {
                self.last_poll_visits += 1;
                self.total_visits += 1;
                continue;
            }
            let remaining = budget - self.last_poll_visits;
            let child = match self.stage {
                IntentStage::Inputs => {
                    let j = self.inputs.as_mut().unwrap();
                    Some((j.poll(remaining), j.last_poll_visits))
                }
                IntentStage::Economics => {
                    let j = self.economics.as_mut().unwrap();
                    Some((j.poll(remaining), j.last_poll_visits))
                }
                IntentStage::Fleets => {
                    let j = self.fleets.as_mut().unwrap();
                    Some((j.poll(remaining), j.last_poll_visits))
                }
                IntentStage::Reasons => {
                    let j = self.reason_storage.as_mut().unwrap();
                    Some((
                        j.poll(remaining).map_err(|_| Error::PreparationCapacity),
                        j.last_poll_visits(),
                    ))
                }
                IntentStage::ProposalCopy | IntentStage::TrialCopy | IntentStage::FinalCopy => {
                    let j = self.copy.as_mut().unwrap();
                    Some((j.poll(remaining), j.last_poll_visits()))
                }
                IntentStage::Validate => {
                    let j = self.validation.as_mut().unwrap();
                    Some((j.poll(remaining), j.last_poll_visits()))
                }
                _ => None,
            };
            if let Some((result, visits)) = child {
                self.last_poll_visits += visits;
                self.total_visits += visits as u64;
                match result {
                    Ok(Progress::Pending) => continue,
                    Ok(Progress::Ready) => {
                        if self.last_poll_visits == budget {
                            break;
                        }
                    }
                    Err(Error::Fault(LivingDeterministicFaultV2::Validation(
                        LivingValidationErrorV2::Capacity {
                            collection,
                            limit,
                            found,
                        },
                    ))) if self.stage == IntentStage::Validate => {
                        self.validation_capacity = Some(LivingValidationErrorV2::Capacity {
                            collection,
                            limit,
                            found,
                        });
                        self.stage = IntentStage::CapacityReason;
                        continue;
                    }
                    Err(error) => {
                        let reported = Self::reported_error(&error);
                        self.fault = Some(error);
                        return Err(reported);
                    }
                }
            }
            self.last_poll_visits += 1;
            self.total_visits += 1;
            if let Err(error) = self.visit() {
                let reported = Self::reported_error(&error);
                self.fault = Some(error);
                return Err(reported);
            }
        }
        Ok(
            if self.stage == IntentStage::Ready && self.retirement.is_empty() {
                Progress::Ready
            } else {
                Progress::Pending
            },
        )
    }
    fn visit(&mut self) -> Result<(), super::step_job::LivingLifecycleJobErrorV2> {
        use super::step_job::LivingLifecycleJobErrorV2 as Error;
        use IntentStage::*;
        match self.stage {
            Inputs => {
                let inputs = self.inputs.as_mut().unwrap().finish()?;
                self.economics = Some(LivingEconomicEnumerationJobV2::begin(
                    self.catalog.clone(),
                    inputs,
                    self.tick,
                ));
                self.retirement.start(self.inputs.take());
                self.stage = Economics;
            }
            Economics => {
                let economics = self.economics.as_mut().unwrap().finish()?;
                self.fleets = Some(LivingFleetEnumerationJobV2::begin(
                    economics,
                    self.tick,
                    self.seed,
                    self.branch,
                ));
                self.retirement.start(self.economics.take());
                self.stage = Fleets;
            }
            Fleets => {
                let mut enumeration = self.fleets.as_mut().unwrap().finish()?;
                self.batches = std::mem::take(&mut enumeration.batches).into_iter();
                self.enumeration = Some(enumeration);
                self.retirement.start(self.fleets.take());
                self.index = 0;
                self.stage = GraphSystems;
            }
            GraphSystems => {
                let graph = std::sync::Arc::get_mut(&mut self.enumeration.as_mut().unwrap().graph)
                    .expect("retired enumerator releases graph handle");
                if let Some(row) = self.source.systems.as_slice().get(self.index) {
                    // Reuse the enumerator's original, now-empty allocation.
                    if graph.systems.len() == graph.systems.capacity() {
                        return Err(Error::PreparationCapacity);
                    }
                    graph.systems.push(row.id);
                    self.index += 1;
                } else {
                    self.index = 0;
                    self.stage = GraphLanes;
                }
            }
            GraphLanes => {
                let graph = std::sync::Arc::get_mut(&mut self.enumeration.as_mut().unwrap().graph)
                    .expect("private graph");
                if let Some(row) = self.source.lanes.as_slice().get(self.index) {
                    if graph.lanes.len() == graph.lanes.capacity() {
                        return Err(Error::PreparationCapacity);
                    }
                    graph
                        .lanes
                        .push((row.id, row.system_a, row.system_b, row.distance_units));
                    self.index += 1;
                } else {
                    self.stage = Actor;
                }
            }
            Actor => {
                if let Some(mut batch) = self.batches.next() {
                    self.fleet_rows = std::mem::take(&mut batch.fleets).into_iter();
                    if let Some(economy) = &mut batch.economy {
                        self.economic_rows = std::mem::take(&mut economy.ranked).into_iter();
                        self.explanations = std::mem::take(&mut economy.explanations).into_iter();
                    }
                    self.batch = Some(batch);
                    self.accepted_economy = false;
                    self.accepted_fleet = false;
                    self.stage = Economy;
                } else {
                    self.old_decisions = std::mem::take(&mut self.events.decisions).into_iter();
                    self.stage = Sort;
                }
            }
            Economy => {
                if self.accepted_economy {
                    self.retirement.start((
                        std::mem::replace(&mut self.economic_rows, Vec::new().into_iter()),
                        std::mem::replace(&mut self.explanations, Vec::new().into_iter()),
                    ));
                    self.stage = Fleet;
                } else if let Some((_, candidate)) = self.economic_rows.next() {
                    self.economic = Some(candidate);
                    self.decision = self.explanations.next();
                    if self.decision.is_none() {
                        return Err(Error::Fault(LivingDeterministicFaultV2::Invariant));
                    }
                    if self.decision.as_ref().unwrap().actor != self.batch.as_ref().unwrap().actor {
                        return Err(Error::Fault(LivingDeterministicFaultV2::Invariant));
                    }
                    self.is_economy = true;
                    self.begin_reasons();
                } else {
                    self.retirement.start((
                        std::mem::replace(&mut self.economic_rows, Vec::new().into_iter()),
                        std::mem::replace(&mut self.explanations, Vec::new().into_iter()),
                    ));
                    self.stage = Fleet;
                }
            }
            Fleet => {
                if self.accepted_fleet {
                    self.retirement.start((
                        std::mem::replace(&mut self.fleet_rows, Vec::new().into_iter()),
                        self.batch.take(),
                    ));
                    self.stage = Actor;
                } else if let Some(candidate) = self.fleet_rows.next() {
                    let LivingFleetCandidateV2 {
                        mut state,
                        decision,
                        action_bytes,
                        deltas,
                    } = candidate;
                    self.proposal = Some(std::mem::take(&mut *state));
                    self.decision = Some(decision);
                    self.is_economy = false;
                    self.retirement
                        .start((Some(state), (RulesByteBuffer(action_bytes), deltas)));
                    self.begin_reasons();
                } else {
                    self.retirement.start((
                        std::mem::replace(&mut self.fleet_rows, Vec::new().into_iter()),
                        self.batch.take(),
                    ));
                    self.stage = Actor;
                }
            }
            Reasons => {
                self.decision.as_mut().unwrap().reasons = self
                    .reason_storage
                    .as_mut()
                    .unwrap()
                    .take_ready_result()
                    .map_err(|_| Error::PreparationCapacity)?;
                self.retirement.start(self.reason_storage.take());
                if self.is_economy {
                    self.copy = Some(super::step_job::LivingStateCopyJobV2::begin(
                        self.source.clone(),
                    ));
                    self.stage = ProposalCopy;
                } else {
                    self.begin_trial();
                }
            }
            ProposalCopy => {
                self.proposal = Some(self.copy.as_mut().unwrap().take_ready_result()?);
                self.retirement.start(self.copy.take());
                self.apply_stage = 0;
                self.index = 0;
                self.nested = 0;
                self.colony_index = 0;
                self.source_system = None;
                self.destination_system = None;
                self.permission = false;
                self.war = false;
                self.adjacent = false;
                self.stage = Apply;
            }
            Apply => self.apply_visit()?,
            TrialCopy => {
                self.trial = Some(self.copy.as_mut().unwrap().take_ready_result()?);
                self.retirement.start(self.copy.take());
                self.domain = 0;
                self.stage = Reserve;
            }
            Reserve => self.reserve_visit()?,
            Validate => {
                self.accepted = true;
                self.stage = Attempt;
            }
            CapacityReason => {
                let LivingValidationErrorV2::Capacity {
                    collection,
                    limit,
                    found,
                } = self.validation_capacity.take().expect("capacity failure")
                else {
                    unreachable!()
                };
                // Validate the static label bound before allocating replacement text.
                if collection.len() > 64 {
                    return Err(Error::Fault(LivingDeterministicFaultV2::Invariant));
                }
                let decision = self.decision.as_mut().unwrap();
                decision
                    .reasons
                    .push(LivingDecisionReasonV2::QueueCapacity {
                        world: decision.target.unwrap_or(decision.actor),
                        queue: slug(&collection.replace(' ', "_"))
                            .map_err(|_| Error::Fault(LivingDeterministicFaultV2::Invariant))?,
                        limit: u32::try_from(limit)
                            .map_err(|_| Error::Fault(LivingDeterministicFaultV2::Arithmetic))?,
                        reserved: u32::try_from(found)
                            .map_err(|_| Error::Fault(LivingDeterministicFaultV2::Arithmetic))?,
                    });
                self.accepted = false;
                self.stage = Attempt;
            }
            Attempt => {
                if self.accepted {
                    let decision = self.decision.as_mut().unwrap();
                    decision.outcome = LivingDecisionOutcomeV2::Accepted;
                    decision.reasons.push(LivingDecisionReasonV2::Accepted {
                        rule: slug("atomic_reservation")
                            .map_err(|_| Error::Fault(LivingDeterministicFaultV2::Invariant))?,
                    });
                    self.retirement
                        .start((self.validation.take(), self.reserved.take()));
                    self.reserved = self.validation_source.take();
                    if self.is_economy {
                        self.accepted_economy = true;
                    } else {
                        self.accepted_fleet = true;
                    }
                } else {
                    self.retirement
                        .start((self.validation.take(), self.validation_source.take()));
                }
                self.stage = DecisionStore;
            }
            DecisionStore => {
                let decision = self.decision.take().unwrap();
                self.decisions.insert(
                    (decision.phase.ordinal(), decision.actor, decision.ordinal),
                    decision,
                );
                self.stage = AttemptCleanup;
            }
            AttemptCleanup => {
                // No successor begins while a rejected trial or proposal owns records.
                self.retirement.start((
                    self.proposal.take(),
                    (self.trial.take(), self.economic.take()),
                ));
                self.stage = if self.is_economy { Economy } else { Fleet };
            }
            Sort => {
                if let Some(decision) = self.old_decisions.next() {
                    self.decisions.insert(
                        (decision.phase.ordinal(), decision.actor, decision.ordinal),
                        decision,
                    );
                } else {
                    self.decision_storage = Some(
                        super::step_job::LivingVecStorageJobV2::begin_empty(self.decisions.len()),
                    );
                    self.stage = Dense;
                }
            }
            Dense => {
                if let Some(storage) = &mut self.decision_storage {
                    if !storage.ready() {
                        storage.poll(1).map_err(|_| Error::PreparationCapacity)?;
                    } else {
                        self.events.decisions = storage
                            .take_ready_result()
                            .map_err(|_| Error::PreparationCapacity)?;
                        self.retirement.start(self.decision_storage.take());
                    }
                } else if let Some(((_, actor, ordinal), mut decision)) = self.decisions.pop_first()
                {
                    let group = (decision.phase.ordinal(), actor);
                    if self.previous != Some(group) {
                        self.next = 0;
                        self.previous = Some(group);
                    }
                    let dense = u16::try_from(self.next)
                        .map_err(|_| Error::Fault(LivingDeterministicFaultV2::Arithmetic))?;
                    self.remap.insert((group.0, actor, ordinal), dense);
                    decision.ordinal = dense;
                    self.next += 1;
                    self.events.decisions.push(decision);
                } else {
                    self.index = 0;
                    self.nested = 0;
                    self.stage = Remap;
                }
            }
            Remap => {
                if let Some(decision) = self.events.decisions.get_mut(self.index) {
                    if let Some(reason) = decision.reasons.get_mut(self.nested) {
                        if let LivingDecisionReasonV2::LowerRank {
                            winning_actor,
                            winning_ordinal,
                        } = reason
                        {
                            *winning_ordinal = *self
                                .remap
                                .get(&(decision.phase.ordinal(), *winning_actor, *winning_ordinal))
                                .ok_or(Error::Fault(LivingDeterministicFaultV2::Invariant))?;
                        }
                        self.nested += 1;
                    } else {
                        self.index += 1;
                        self.nested = 0;
                    }
                } else {
                    self.copy = Some(super::step_job::LivingStateCopyJobV2::begin(
                        self.reserved.as_ref().unwrap().clone(),
                    ));
                    self.stage = FinalCopy;
                }
            }
            FinalCopy => {
                self.trial = Some(self.copy.as_mut().unwrap().take_ready_result()?);
                self.retirement.start(self.copy.take());
                self.stage = Ready;
            }
            Ready => {}
        }
        Ok(())
    }
    fn begin_reasons(&mut self) {
        // Exactly one terminal reason is appended to each attempted decision.
        // Preserve and move the original reason allocation through counted
        // preparation; terminal pushes cannot reallocate populated storage.
        self.reason_storage = Some(super::step_job::LivingVecStorageJobV2::begin(
            std::mem::take(&mut self.decision.as_mut().unwrap().reasons),
            1,
        ));
        self.stage = IntentStage::Reasons;
    }
    fn begin_trial(&mut self) {
        self.copy = Some(super::step_job::LivingStateCopyJobV2::begin(
            self.reserved.as_ref().unwrap().clone(),
        ));
        self.stage = IntentStage::TrialCopy;
    }
    pub(crate) fn take_ready_result(
        &mut self,
    ) -> Result<
        (LivingGalaxyStateV2, super::receipt::LivingPendingEventsV2),
        super::step_job::LivingLifecycleJobErrorV2,
    > {
        use super::step_job::LivingLifecycleJobErrorV2 as Error;
        if let Some(error) = &self.fault {
            return Err(Self::reported_error(error));
        }
        if self.cancelled {
            return Err(Error::Cancelled);
        }
        if self.stage != IntentStage::Ready || !self.retirement.is_empty() || self.delivered {
            return Err(Error::NotReady);
        }
        self.delivered = true;
        Ok((
            self.trial.take().ok_or(Error::NotReady)?,
            std::mem::take(&mut self.events),
        ))
    }
}
impl LivingDueIntentsJobV2 {
    fn economic_id(&self, kind: LivingEntityKindV2) -> LivingEntityIdV2 {
        let ordinal = match self.economic.as_ref().unwrap().action {
            LivingEconomicActionV2::Supply { .. } => 1,
            LivingEconomicActionV2::Construction { definition, .. } => match definition {
                "extractor" => 2,
                "foundry" => 3,
                "solar_array" => 6,
                "shipyard" => 8,
                "defense_battery" => 9,
                _ => 16,
            },
            LivingEconomicActionV2::Hull { definition, .. } => match definition {
                "colony_ark" => 4,
                "escort" => 5,
                "scout" => 7,
                _ => 16,
            },
        };
        autonomous_entity_id_v2(&LivingAutonomousEntityInputsV2 {
            catalog_hash: self.catalog.catalog_hash(),
            genesis_seed: self.seed,
            branch: self.branch,
            tick: self.tick,
            phase: LivingPhaseV2::GenerateIntents.ordinal(),
            actor: self.decision.as_ref().unwrap().actor,
            intent_ordinal: ordinal,
            entity_kind: kind.ordinal(),
            local_id: 0,
        })
    }
    fn apply_visit(&mut self) -> Result<(), super::step_job::LivingLifecycleJobErrorV2> {
        use super::step_job::{
            LivingLifecycleJobErrorV2 as Error, LivingLifecycleProgressV2 as Progress,
        };
        let invariant = || Error::Fault(LivingDeterministicFaultV2::Invariant);
        let actor = self.decision.as_ref().unwrap().actor;
        let action = self.economic.as_ref().unwrap().action.clone(); // fixed scalar descriptor
        match self.apply_stage {
            0 => {
                if let LivingEconomicActionV2::Construction {
                    scrap: Some(key), ..
                } = action
                {
                    let proposed = self.proposal.as_mut().unwrap();
                    let mut plan = std::collections::BTreeSet::new();
                    if let Ok(index) = proposed
                        .facilities
                        .as_slice()
                        .binary_search_by_key(&key, |r| r.id)
                    {
                        plan.insert(index);
                    }
                    self.facility_remove = Some(super::step_job::LivingPlannedFilterJobV2::begin(
                        std::mem::take(&mut proposed.facilities),
                        plan,
                    ));
                    self.apply_stage = 1;
                } else {
                    self.apply_stage = 2;
                }
            }
            1 => {
                let job = self.facility_remove.as_mut().unwrap();
                if job.ready() {
                    self.proposal.as_mut().unwrap().facilities = job.take_ready_result()?;
                    self.retirement.start(self.facility_remove.take());
                    self.apply_stage = 2;
                } else {
                    job.visit()?;
                }
            }
            2 => match action {
                LivingEconomicActionV2::Construction {
                    world, definition, ..
                } => {
                    let pack = &self.catalog.industries()[self
                        .catalog
                        .industries()
                        .binary_search_by(|row| row.id.as_str().cmp(definition))
                        .map_err(|_| invariant())?];
                    let row = LivingConstructionJobV2 {
                        id: self.economic_id(LivingEntityKindV2::ConstructionJob),
                        world,
                        owner: actor,
                        definition: pack.id.clone(),
                        accepted_tick: self.tick,
                        completion_tick: add_ticks(self.tick, u64::from(pack.build_ticks))
                            .map_err(Error::Fault)?,
                        paid_alloy: pack.alloy_cost,
                    };
                    let proposed = self.proposal.as_mut().unwrap();
                    let world = proposed.worlds.find_mut(&world).ok_or_else(invariant)?;
                    world.inventory.alloy = world
                        .inventory
                        .alloy
                        .checked_sub(pack.alloy_cost)
                        .ok_or_else(invariant)?;
                    self.construction_insert = Some(
                        super::step_job::LivingSortedInsertionJobV2::begin_replacement(
                            std::mem::take(&mut proposed.construction_jobs),
                            row,
                        ),
                    );
                    self.apply_stage = 3;
                }
                LivingEconomicActionV2::Hull { world, definition } => {
                    let pack = &self.catalog.hulls()[self
                        .catalog
                        .hulls()
                        .binary_search_by(|row| row.id.as_str().cmp(definition))
                        .map_err(|_| invariant())?];
                    let row = LivingHullJobV2 {
                        id: self.economic_id(LivingEntityKindV2::HullJob),
                        world,
                        owner: actor,
                        definition: pack.id.clone(),
                        accepted_tick: self.tick,
                        completion_tick: add_ticks(self.tick, u64::from(pack.build_ticks))
                            .map_err(Error::Fault)?,
                        paid_alloy: pack.alloy_cost,
                        target_fleet: None,
                    };
                    let proposed = self.proposal.as_mut().unwrap();
                    let world = proposed.worlds.find_mut(&world).ok_or_else(invariant)?;
                    world.inventory.alloy = world
                        .inventory
                        .alloy
                        .checked_sub(pack.alloy_cost)
                        .ok_or_else(invariant)?;
                    self.hull_insert = Some(
                        super::step_job::LivingSortedInsertionJobV2::begin_replacement(
                            std::mem::take(&mut proposed.hull_jobs),
                            row,
                        ),
                    );
                    self.apply_stage = 3;
                }
                LivingEconomicActionV2::Supply {
                    source,
                    destination,
                    receiver,
                    kind,
                    ..
                } => {
                    let get = |key| {
                        self.source
                            .worlds
                            .as_slice()
                            .binary_search_by_key(&key, |r| r.id)
                            .ok()
                            .map(|i| &self.source.worlds.as_slice()[i])
                    };
                    let from = get(source).ok_or_else(invariant)?;
                    let to = get(destination).ok_or_else(invariant)?;
                    if from.owner != Some(actor) || to.owner != Some(receiver) {
                        return Err(invariant());
                    }
                    self.permission = kind == LivingRouteKindV2::Internal && receiver == actor;
                    if from.system == to.system {
                        self.apply_stage = 5;
                    } else {
                        self.path = Some(super::step_job::RetreatPathJob::begin(
                            self.enumeration.as_ref().unwrap().graph.clone(),
                            from.system,
                            to.system,
                        ));
                        self.apply_stage = 4;
                    }
                }
            },
            3 => {
                if let Some(job) = &mut self.construction_insert {
                    if job.ready() {
                        self.proposal.as_mut().unwrap().construction_jobs =
                            job.take_ready_result()?;
                        self.retirement.start(self.construction_insert.take());
                        self.begin_trial();
                    } else {
                        job.visit()?;
                    }
                } else if let Some(job) = &mut self.hull_insert {
                    if job.ready() {
                        self.proposal.as_mut().unwrap().hull_jobs = job.take_ready_result()?;
                        self.retirement.start(self.hull_insert.take());
                        self.begin_trial();
                    } else {
                        job.visit()?;
                    }
                }
            }
            4 => {
                let job = self.path.as_mut().unwrap();
                if job.poll(1)? == Progress::Ready {
                    // Extraction is a separate transition after the final path visit.
                    self.apply_stage = 14;
                }
            }
            14 => {
                let (_, path) = self
                    .path
                    .as_mut()
                    .unwrap()
                    .take_ready_result()?
                    .ok_or_else(invariant)?;
                self.retirement.start((self.path.take(), path));
                self.apply_stage = 5;
            }
            5..=9 => {
                let LivingEconomicActionV2::Supply {
                    destination,
                    receiver,
                    kind,
                    ..
                } = action
                else {
                    return Err(invariant());
                };
                let pair = ordered_pair(actor, receiver);
                match self.apply_stage {
                    5 => {
                        if let Some(row) = self.source.agreements.as_slice().get(self.index) {
                            self.index += 1;
                            if kind == LivingRouteKindV2::Trade
                                && row.kind == LivingAgreementKindV2::Trade
                                && (row.participant_a, row.participant_b) == pair
                                && active_interval(row.start_tick, row.end_tick, self.tick)
                            {
                                self.permission = true;
                            }
                        } else {
                            self.index = 0;
                            self.apply_stage = 6;
                        }
                    }
                    6 => {
                        if let Some(row) = self.source.wars.as_slice().get(self.index) {
                            self.index += 1;
                            if (row.participant_a, row.participant_b) == pair
                                && active_interval(row.start_tick, row.end_tick, self.tick)
                            {
                                self.war = true;
                            }
                        } else {
                            self.index = 0;
                            self.apply_stage = 7;
                        }
                    }
                    7 => {
                        if kind != LivingRouteKindV2::Aid {
                            self.apply_stage = 10;
                        } else {
                            let trader = self
                                .source
                                .civilizations
                                .as_slice()
                                .binary_search_by_key(&actor, |r| r.id)
                                .ok()
                                .is_some_and(|i| {
                                    self.source.civilizations.as_slice()[i].policy.0.as_str()
                                        == "trader"
                                });
                            if !trader || self.war {
                                return Err(invariant());
                            }
                            self.apply_stage = 8;
                        }
                    }
                    8 => {
                        if let Some(colony) = self.source.colonies.as_slice().get(self.colony_index)
                        {
                            self.colony_index += 1;
                            if colony.owner == actor {
                                let worlds = self.source.worlds.as_slice();
                                let home = worlds
                                    .binary_search_by_key(&colony.world, |r| r.id)
                                    .ok()
                                    .map(|i| worlds[i].system);
                                let target = worlds
                                    .binary_search_by_key(&destination, |r| r.id)
                                    .ok()
                                    .map(|i| worlds[i].system);
                                self.source_system = home;
                                self.destination_system = target;
                                if home.is_some() && home == target {
                                    self.adjacent = true;
                                }
                                self.index = 0;
                                self.apply_stage = 9;
                            }
                        } else {
                            self.permission = self.adjacent;
                            self.apply_stage = 10;
                        }
                    }
                    9 => {
                        if let Some(lane) = self.source.lanes.as_slice().get(self.index) {
                            self.index += 1;
                            if (Some(lane.system_a) == self.source_system
                                && Some(lane.system_b) == self.destination_system)
                                || (Some(lane.system_b) == self.source_system
                                    && Some(lane.system_a) == self.destination_system)
                            {
                                self.adjacent = true;
                            }
                        } else {
                            self.apply_stage = 8;
                        }
                    }
                    _ => unreachable!(),
                }
            }
            10 => {
                if !self.permission {
                    return Err(invariant());
                }
                let LivingEconomicActionV2::Supply {
                    source,
                    destination,
                    resource,
                    kind,
                    receiver,
                    batch_size,
                    existing_route,
                } = action
                else {
                    return Err(invariant());
                };
                let (id, next_due) = if let Some(id) = existing_route {
                    let i = self
                        .source
                        .routes
                        .as_slice()
                        .binary_search_by_key(&id, |r| r.id)
                        .map_err(|_| invariant())?;
                    let route = &self.source.routes.as_slice()[i];
                    if route.source_world != source
                        || route.destination_world != destination
                        || route.resource != resource
                    {
                        return Err(invariant());
                    }
                    (id, route.next_due)
                } else {
                    (
                        self.economic_id(LivingEntityKindV2::Route),
                        add_ticks(self.tick, 10).map_err(Error::Fault)?,
                    )
                };
                let row = LivingFreightRouteV2 {
                    id,
                    kind,
                    source_world: source,
                    destination_world: destination,
                    resource,
                    batch_size,
                    cadence_ticks: 10,
                    source_reserve: 20,
                    source_owner: actor,
                    receiver_owner: receiver,
                    next_due,
                    suspended: false,
                };
                self.route_insert = Some(
                    super::step_job::LivingSortedInsertionJobV2::begin_replacement(
                        std::mem::take(&mut self.proposal.as_mut().unwrap().routes),
                        row,
                    ),
                );
                self.apply_stage = 11;
            }
            11 => {
                let job = self.route_insert.as_mut().unwrap();
                if job.ready() {
                    self.proposal.as_mut().unwrap().routes = job.take_ready_result()?;
                    self.retirement.start(self.route_insert.take());
                    self.begin_trial();
                } else {
                    job.visit()?;
                }
            }
            _ => return Err(invariant()),
        }
        Ok(())
    }
    fn reserve_visit(&mut self) -> Result<(), super::step_job::LivingLifecycleJobErrorV2> {
        use super::step_job::LivingLifecycleJobErrorV2 as Error;
        macro_rules! domain {
            ($job:ident,$field:ident) => {{
                if let Some(job) = &mut self.$job {
                    if !job.ready() {
                        job.visit(&self.source.$field)?;
                    } else if job.conflict {
                        self.retirement.start(self.$job.take());
                        let decision = self.decision.as_mut().unwrap();
                        decision
                            .reasons
                            .push(LivingDecisionReasonV2::InvalidTarget {
                                target: decision.target.unwrap_or(decision.actor),
                                rule: slug("reservation_conflict").map_err(|_| {
                                    Error::Fault(LivingDeterministicFaultV2::Invariant)
                                })?,
                            });
                        self.accepted = false;
                        self.stage = IntentStage::Attempt;
                    } else {
                        self.trial.as_mut().unwrap().$field = std::mem::take(&mut job.output);
                        self.retirement.start(self.$job.take());
                        self.domain += 1;
                    }
                } else {
                    self.$job = Some(IntentReservationDomain::begin(
                        std::mem::take(&mut self.proposal.as_mut().unwrap().$field),
                        std::mem::take(&mut self.trial.as_mut().unwrap().$field),
                    ));
                }
            }};
        }
        match self.domain {
            0 => domain!(worlds, worlds),
            1 => domain!(hull_jobs, hull_jobs),
            2 => domain!(construction_jobs, construction_jobs),
            3 => domain!(facilities, facilities),
            4 => domain!(fleet_delta, fleets),
            5 => domain!(routes, routes),
            _ => {
                let trial = std::sync::Arc::new(self.trial.take().unwrap());
                self.validation = Some(super::step_job::LivingStateValidationJobV2::begin_at_tick(
                    self.catalog.clone(),
                    trial.clone(),
                    self.tick,
                ));
                self.validation_source = Some(trial);
                self.stage = IntentStage::Validate;
            }
        }
        Ok(())
    }
}

impl<'a> RetirementFields<'a> for LivingDueIntentsJobV2 {
    fn retirement_child(
        &mut self,
        index: usize,
        policy: LivingStateRetirementPolicyV2,
    ) -> Option<Box<dyn RetirementPoll + 'a>> {
        let Self {
            catalog: _,
            source: _,
            reserved: _,
            inputs: _,
            economics: _,
            fleets: _,
            enumeration: _,
            batches: _,
            batch: _,
            economic_rows: _,
            explanations: _,
            fleet_rows: _,
            economic: _,
            decision: _,
            reason_storage: _,
            copy: _,
            proposal: _,
            trial: _,
            validation_source: _,
            validation: _,
            validation_capacity: _,
            worlds: _,
            hull_jobs: _,
            construction_jobs: _,
            facilities: _,
            fleet_delta: _,
            routes: _,
            hull_insert: _,
            construction_insert: _,
            route_insert: _,
            facility_remove: _,
            path: _,
            events: _,
            decisions: _,
            old_decisions: _,
            decision_storage: _,
            remap: _,
            retirement: _,
            tick: _,
            seed: _,
            branch: _,
            stage: _,
            apply_stage: _,
            domain: _,
            index: _,
            nested: _,
            colony_index: _,
            source_system: _,
            destination_system: _,
            permission: _,
            war: _,
            adjacent: _,
            accepted_economy: _,
            accepted_fleet: _,
            is_economy: _,
            accepted: _,
            previous: _,
            next: _,
            fault: _,
            cancelled: _,
            delivered: _,
            last_poll_visits: _,
            total_visits: _,
        } = self;
        match index {
            0 => Some(retirement_node(
                self.catalog
                    .0
                    .take()
                    .map(super::step_job::LivingPhaseCatalogV2::Owned),
                policy,
            )),
            1 => Some(retirement_node(self.source.0.take(), policy)),
            2 => Some(retirement_node(std::mem::take(&mut self.reserved), policy)),
            3 => Some(retirement_node(std::mem::take(&mut self.inputs), policy)),
            4 => Some(retirement_node(std::mem::take(&mut self.economics), policy)),
            5 => Some(retirement_node(std::mem::take(&mut self.fleets), policy)),
            6 => Some(retirement_node(
                std::mem::take(&mut self.enumeration),
                policy,
            )),
            7 => Some(retirement_node(
                std::mem::replace(&mut self.batches, Vec::new().into_iter()),
                policy,
            )),
            8 => Some(retirement_node(std::mem::take(&mut self.batch), policy)),
            9 => Some(retirement_node(
                std::mem::replace(&mut self.economic_rows, Vec::new().into_iter()),
                policy,
            )),
            10 => Some(retirement_node(
                std::mem::replace(&mut self.explanations, Vec::new().into_iter()),
                policy,
            )),
            11 => Some(retirement_node(
                std::mem::replace(&mut self.fleet_rows, Vec::new().into_iter()),
                policy,
            )),
            12 => Some(retirement_node(std::mem::take(&mut self.economic), policy)),
            13 => Some(retirement_node(std::mem::take(&mut self.decision), policy)),
            14 => Some(retirement_node(std::mem::take(&mut self.copy), policy)),
            15 => Some(retirement_node(std::mem::take(&mut self.proposal), policy)),
            16 => Some(retirement_node(std::mem::take(&mut self.trial), policy)),
            17 => Some(retirement_node(
                std::mem::take(&mut self.validation_source),
                policy,
            )),
            18 => Some(retirement_node(
                std::mem::take(&mut self.validation),
                policy,
            )),
            19 => Some(retirement_node(std::mem::take(&mut self.worlds), policy)),
            20 => Some(retirement_node(std::mem::take(&mut self.hull_jobs), policy)),
            21 => Some(retirement_node(
                std::mem::take(&mut self.construction_jobs),
                policy,
            )),
            22 => Some(retirement_node(
                std::mem::take(&mut self.facilities),
                policy,
            )),
            23 => Some(retirement_node(
                std::mem::take(&mut self.fleet_delta),
                policy,
            )),
            24 => Some(retirement_node(std::mem::take(&mut self.routes), policy)),
            25 => Some(retirement_node(
                std::mem::take(&mut self.hull_insert),
                policy,
            )),
            26 => Some(retirement_node(
                std::mem::take(&mut self.construction_insert),
                policy,
            )),
            27 => Some(retirement_node(
                std::mem::take(&mut self.route_insert),
                policy,
            )),
            28 => Some(retirement_node(
                std::mem::take(&mut self.facility_remove),
                policy,
            )),
            29 => Some(retirement_node(std::mem::take(&mut self.path), policy)),
            30 => Some(retirement_node(std::mem::take(&mut self.events), policy)),
            31 => Some(retirement_node(std::mem::take(&mut self.decisions), policy)),
            32 => Some(retirement_node(
                std::mem::replace(&mut self.old_decisions, Vec::new().into_iter()),
                policy,
            )),
            33 => Some(retirement_node(
                std::mem::take(&mut self.decision_storage),
                policy,
            )),
            34 => Some(retirement_node(std::mem::take(&mut self.remap), policy)),
            35 => Some(retirement_node(
                std::mem::take(&mut self.retirement),
                policy,
            )),
            36 => Some(retirement_node(
                IntentFailureOwner(self.fault.take()),
                policy,
            )),
            37 => Some(retirement_node(self.reason_storage.take(), policy)),
            _ => None,
        }
    }
}
impl<'a> RetirementFields<'a> for Box<LivingDueIntentsJobV2> {
    fn retirement_child(
        &mut self,
        index: usize,
        policy: LivingStateRetirementPolicyV2,
    ) -> Option<Box<dyn RetirementPoll + 'a>> {
        (**self).retirement_child(index, policy)
    }
}

struct IntentFailureOwner(Option<super::step_job::LivingLifecycleJobErrorV2>);
impl<'a> RetirementFields<'a> for IntentFailureOwner {
    fn retirement_child(
        &mut self,
        _: usize,
        _: LivingStateRetirementPolicyV2,
    ) -> Option<Box<dyn RetirementPoll + 'a>> {
        let Some(super::step_job::LivingLifecycleJobErrorV2::Fault(fault)) = self.0.take() else {
            return None;
        };
        let job = super::simulation::LivingPublicationRetirementJobV2::begin_fault(fault);
        Some(Box::new(job))
    }
}
