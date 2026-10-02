//! Resumable phase five on an unpublished, owned candidate.
use std::collections::{BTreeMap, BTreeSet};
use std::sync::Arc;
use std::vec::IntoIter;

use super::super::catalog::{LivingIndustryEffectV2, ValidatedLivingCatalogPackV2};
use super::super::event_facts::{
    LivingDepositInputFactV2, LivingEventFactsV2, LivingEventResourceFactV2,
    LivingEventResourceRoleV2,
};
use super::super::ids::{
    LivingAutonomousEntityInputsV2, LivingBranchIdV2, LivingEntityIdV2 as Id, LivingEntityKindV2,
    LivingPhaseV2, LivingTickV2 as Tick, autonomous_entity_id_v2,
};
use super::super::model::*;
use super::super::receipt::{LivingEventKindV2, LivingEventProvenanceV2, LivingPendingEventsV2};
use super::super::simulation::LivingDeterministicFaultV2 as Fault;
use super::super::wire::LivingSortedVecV2;
use super::{
    LIVING_MAX_PHASE_POLL_VISITS_V2, LivingLifecycleJobErrorV2 as Error,
    LivingLifecycleProgressV2 as Progress, LivingRecipeJobV2, LivingRepairJobV2,
};

#[derive(Clone)]
struct Plan {
    period: Option<u64>,
    deposit: u32,
    hp: u32,
    inputs: super::PhaseArc<Vec<(LivingResourceV2, u32)>>,
    outputs: super::PhaseArc<Vec<(LivingResourceV2, u32)>>,
}
struct FleetRows {
    fleet: LivingFleetV2,
    hulls: BTreeMap<Id, LivingHullV2>,
}
#[derive(Clone, Copy, Eq, PartialEq)]
enum Stage {
    Catalog,
    Inputs,
    Outputs,
    HullDefinitions,
    Worlds,
    Deposits,
    Colonies,
    Facilities,
    Fleets,
    HullRows,
    Construction,
    ConstructionKeep,
    ConstructionCreate,
    ConstructionEvent,
    HullJobs,
    HullKeep,
    HullCreate,
    HullFleet,
    HullInsert,
    HullEvent,
    Hubs,
    Recipes,
    RecipeStart,
    RecipeCommit,
    FacilityOutput,
    ColonyOutput,
    FleetOutput,
    HullOutput,
    Repair,
    RetireRecipe,
    RetireIndexes,
    Ready,
}
#[derive(Clone, Copy)]
enum Target {
    Hub(Id, u8),
    Facility(Id),
}

/// Complete scheduled production, with a shared 1..=1024 visit budget.
/// Construction and hull identities and recipe ordering match the fixed phase.
/// No partial candidate escapes a cancelled, failed, or incomplete job.
pub struct LivingProductionJobV2<'a> {
    retirement: super::PhaseRetirementLane<'a>,
    delivered: bool,
    catalog: super::PhaseCatalogSlot<'a>,
    state: LivingGalaxyStateV2,
    events: LivingPendingEventsV2,
    tick: Tick,
    seed: [u8; 32],
    branch: LivingBranchIdV2,
    stage: Stage,
    index: usize,
    catalog_index: usize,
    plans: BTreeMap<String, Plan>,
    hull_hit_points: BTreeMap<String, u32>,
    plan: Option<Plan>,
    worlds: BTreeMap<Id, (usize, Option<Id>)>,
    minimum_deposits: BTreeMap<Id, BTreeSet<(u64, Id, usize)>>,
    first_deposits: BTreeMap<Id, usize>,
    colony_source: IntoIter<LivingColonyV2>,
    colonies: BTreeMap<Id, LivingColonyV2>,
    facility_source: IntoIter<LivingFacilityV2>,
    facilities: BTreeMap<Id, LivingFacilityV2>,
    recipe_order: BTreeSet<(u8, Id)>,
    fleet_source: IntoIter<LivingFleetV2>,
    fleets: BTreeMap<Id, FleetRows>,
    fleet: Option<LivingFleetV2>,
    hull_source: IntoIter<LivingHullV2>,
    hulls: BTreeMap<Id, LivingHullV2>,
    construction_source: IntoIter<LivingConstructionJobV2>,
    construction_ordinal: u16,
    pending_construction: Option<LivingConstructionJobV2>,
    pending_hull_job: Option<LivingHullJobV2>,
    pending_hull: Option<LivingHullV2>,
    completion_fleet: Option<Id>,
    completion_event: Option<(LivingEventKindV2, LivingEventFactsV2)>,
    job_source: IntoIter<LivingHullJobV2>,
    hull_ordinal: u16,
    hub_order: BTreeSet<Id>,
    hub: Option<Id>,
    hub_clock: u8,
    recipe: Option<LivingRecipeJobV2>,
    target: Option<Target>,
    active_plan: Option<Plan>,
    retired_plan: Option<Plan>,
    retired_reasons: IntoIter<LivingBlockedReasonV2>,
    recipe_output: Option<super::LivingRecipeOutputV2>,
    recipe_resume: Stage,
    commit_stage: u8,
    commit_index: usize,
    commit_reasons: IntoIter<LivingBlockedReasonV2>,
    committed_reasons: LivingSortedVecV2<LivingBlockedReasonV2>,
    pending_reason: Option<LivingBlockedReasonV2>,
    facts: LivingEventFactsV2,
    resource_facts:
        BTreeMap<(LivingEventResourceRoleV2, LivingResourceV2), LivingEventResourceFactV2>,
    repair: Option<LivingRepairJobV2<'a>>,
    cancelled: bool,
    fault: Option<Fault>,
    last_poll_visits: usize,
    total_visits: u64,
}
impl<'a> LivingProductionJobV2<'a> {
    /// Move all source collections in constant time; indexing happens in polls.
    pub fn begin(
        catalog: &'a ValidatedLivingCatalogPackV2,
        state: LivingGalaxyStateV2,
        events: LivingPendingEventsV2,
        tick: Tick,
        seed: [u8; 32],
        branch: LivingBranchIdV2,
    ) -> Self {
        Self::begin_with_catalog(
            super::LivingPhaseCatalogV2::Borrowed(catalog),
            state,
            events,
            tick,
            seed,
            branch,
        )
    }
    /// Capture immutable catalog ownership for a job retained across UI updates.
    pub fn begin_owned(
        catalog: Arc<ValidatedLivingCatalogPackV2>,
        state: LivingGalaxyStateV2,
        events: LivingPendingEventsV2,
        tick: Tick,
        seed: [u8; 32],
        branch: LivingBranchIdV2,
    ) -> LivingProductionJobV2<'static> {
        LivingProductionJobV2::begin_with_catalog(
            super::LivingPhaseCatalogV2::Owned(catalog),
            state,
            events,
            tick,
            seed,
            branch,
        )
    }
    fn begin_with_catalog(
        catalog: super::LivingPhaseCatalogV2<'a>,
        mut state: LivingGalaxyStateV2,
        events: LivingPendingEventsV2,
        tick: Tick,
        seed: [u8; 32],
        branch: LivingBranchIdV2,
    ) -> Self {
        Self {
            retirement: super::PhaseRetirementLane::default(),
            delivered: false,
            colony_source: std::mem::take(&mut state.colonies).into_vec().into_iter(),
            facility_source: std::mem::take(&mut state.facilities).into_vec().into_iter(),
            fleet_source: std::mem::take(&mut state.fleets).into_vec().into_iter(),
            construction_source: std::mem::take(&mut state.construction_jobs)
                .into_vec()
                .into_iter(),
            job_source: std::mem::take(&mut state.hull_jobs).into_vec().into_iter(),
            catalog: super::PhaseCatalogSlot(Some(catalog)),
            state,
            events,
            tick,
            seed,
            branch,
            stage: Stage::Catalog,
            index: 0,
            catalog_index: 0,
            plans: BTreeMap::new(),
            hull_hit_points: BTreeMap::new(),
            plan: None,
            worlds: BTreeMap::new(),
            minimum_deposits: BTreeMap::new(),
            first_deposits: BTreeMap::new(),
            colonies: BTreeMap::new(),
            facilities: BTreeMap::new(),
            recipe_order: BTreeSet::new(),
            fleets: BTreeMap::new(),
            fleet: None,
            hull_source: Vec::new().into_iter(),
            hulls: BTreeMap::new(),
            construction_ordinal: 0,
            pending_construction: None,
            pending_hull_job: None,
            pending_hull: None,
            completion_fleet: None,
            completion_event: None,
            hull_ordinal: 0,
            hub_order: BTreeSet::new(),
            hub: None,
            hub_clock: 0,
            recipe: None,
            target: None,
            active_plan: None,
            retired_plan: None,
            retired_reasons: Vec::new().into_iter(),
            recipe_output: None,
            recipe_resume: Stage::Ready,
            commit_stage: 0,
            commit_index: 0,
            commit_reasons: Vec::new().into_iter(),
            committed_reasons: LivingSortedVecV2::default(),
            pending_reason: None,
            facts: LivingEventFactsV2::default(),
            resource_facts: BTreeMap::new(),
            repair: None,
            cancelled: false,
            fault: None,
            last_poll_visits: 0,
            total_visits: 0,
        }
    }
    /// Abandon private work without modifying its source authority.
    pub fn cancel(&mut self) {
        self.cancelled = true;
    }
    /// Actual source/output/recipe/repair visits in the preceding poll.
    pub const fn last_poll_visits(&self) -> usize {
        self.last_poll_visits
    }
    /// Cumulative visits, independent of caller poll grouping.
    pub const fn total_visits(&self) -> u64 {
        self.total_visits
    }
    /// Advance no more than the admitted number of actual record visits.
    pub fn poll(&mut self, budget: usize) -> Result<Progress, Error> {
        self.last_poll_visits = 0;
        if self.cancelled {
            return Err(Error::Cancelled);
        }
        if budget == 0 || budget > LIVING_MAX_PHASE_POLL_VISITS_V2 {
            return Err(Error::Budget);
        }
        if let Some(fault) = &self.fault {
            return Err(Error::Fault(fault.clone()));
        }
        while self.stage != Stage::Ready && self.last_poll_visits < budget {
            if self.retirement.visit() {
                self.last_poll_visits += 1;
                self.total_visits += 1;
                continue;
            }

            let delegated = self.recipe.is_some() || self.repair.is_some();
            let result = if let Some(recipe) = &mut self.recipe {
                let result = recipe.poll(budget - self.last_poll_visits);
                let used = recipe.last_poll_visits();
                self.last_poll_visits += used;
                self.total_visits += used as u64;
                match result {
                    Ok(Progress::Ready) => {
                        if self.last_poll_visits == budget {
                            break;
                        }
                        self.recipe_output = Some(recipe.take_ready_result()?);
                        self.retirement
                            .start(self.recipe.take().expect("retained recipe"));
                        self.last_poll_visits += 1;
                        self.total_visits += 1;
                        self.commit_stage = 0;
                        self.commit_index = 0;
                        self.transition(Stage::RecipeCommit);
                        Ok(false)
                    }
                    Ok(Progress::Pending) => Ok(false),
                    Err(error) => Err(error),
                }
            } else if let Some(repair) = &mut self.repair {
                let result = repair.poll(budget - self.last_poll_visits);
                let used = repair.last_poll_visits();
                self.last_poll_visits += used;
                self.total_visits += used as u64;
                match result {
                    Ok(Progress::Ready) => {
                        if self.last_poll_visits == budget {
                            break;
                        }
                        self.state = repair.take_ready_result()?;
                        self.retirement
                            .start(self.repair.take().expect("retained repair"));
                        self.last_poll_visits += 1;
                        self.total_visits += 1;
                        self.stage = Stage::RetireIndexes;
                        Ok(false)
                    }
                    Ok(Progress::Pending) => Ok(false),
                    Err(error) => Err(error),
                }
            } else {
                self.visit()
            };
            match result {
                Ok(true) => {
                    self.last_poll_visits += 1;
                    self.total_visits += 1;
                }
                Ok(false) => {}
                Err(Error::Fault(fault)) => {
                    if !delegated && self.last_poll_visits < budget {
                        self.last_poll_visits += 1;
                        self.total_visits += 1;
                    }
                    self.fault = Some(fault.clone());
                    return Err(Error::Fault(fault));
                }
                Err(_) => {
                    if !delegated && self.last_poll_visits < budget {
                        self.last_poll_visits += 1;
                        self.total_visits += 1;
                    }
                    self.fault = Some(Fault::Invariant);
                    return Err(Error::Fault(Fault::Invariant));
                }
            }
        }
        Ok(if self.stage == Stage::Ready {
            Progress::Ready
        } else {
            Progress::Pending
        })
    }
    /// Return the complete unpublished phase, never a partial result.
    pub fn finish(mut self) -> Result<(LivingGalaxyStateV2, LivingPendingEventsV2), Error> {
        self.take_ready_result()
    }
    /// Borrowed one-time extraction; rejected calls preserve every owner.
    pub fn take_ready_result(
        &mut self,
    ) -> Result<(LivingGalaxyStateV2, LivingPendingEventsV2), Error> {
        if self.cancelled {
            return Err(Error::Cancelled);
        }
        if let Some(fault) = self.fault.clone() {
            return Err(Error::Fault(fault));
        }
        if self.stage != Stage::Ready {
            return Err(Error::NotReady);
        }
        if self.delivered {
            return Err(Error::NotReady);
        }
        self.delivered = true;
        Ok((
            std::mem::take(&mut self.state),
            std::mem::take(&mut self.events),
        ))
    }
    fn transition(&mut self, stage: Stage) {
        self.stage = stage;
        self.index = 0;
    }
    fn id(&self, actor: Id, intent: u16, kind: LivingEntityKindV2, local: u16) -> Id {
        autonomous_entity_id_v2(&LivingAutonomousEntityInputsV2 {
            catalog_hash: self.catalog.catalog_hash(),
            genesis_seed: self.seed,
            branch: self.branch,
            tick: self.tick,
            phase: LivingPhaseV2::RunProduction.ordinal(),
            actor,
            intent_ordinal: intent,
            entity_kind: kind.ordinal(),
            local_id: local,
        })
    }
    fn event(&mut self, kind: LivingEventKindV2, facts: LivingEventFactsV2) -> Result<(), Error> {
        self.events
            .record_explained(
                LivingEventProvenanceV2::Autonomous {
                    phase: LivingPhaseV2::RunProduction,
                },
                kind,
                facts,
            )
            .map_err(Fault::from)?;
        Ok(())
    }
    fn position(&self, world: Id) -> Result<usize, Error> {
        self.worlds
            .get(&world)
            .map(|row| row.0)
            .ok_or(Error::Fault(Fault::Invariant))
    }
    fn start_recipe(&mut self, target: Target, _world: Id, plan: Plan) -> Result<(), Error> {
        self.recipe_resume = self.stage;
        self.target = Some(target);
        self.active_plan = Some(plan);
        self.transition(Stage::RecipeStart);
        Ok(())
    }
    fn begin_recipe(&mut self) -> Result<bool, Error> {
        let target = self.target.ok_or(Error::NotReady)?;
        let world = match target {
            Target::Hub(world, _) => world,
            Target::Facility(id) => self.facilities.get(&id).ok_or(Error::NotReady)?.world,
        };
        let inventory = self.state.worlds.as_slice()[self.position(world)?].inventory;
        let plan = self.active_plan.as_ref().ok_or(Error::NotReady)?;
        let disabled = match target {
            Target::Facility(id) => self.facilities.get(&id).ok_or(Error::NotReady)?.disabled,
            _ => false,
        };
        let ceiling = matches!(target, Target::Hub(_, 2)) && inventory.alloy >= 120;
        if disabled || ceiling {
            self.recipe_output = Some(super::LivingRecipeOutputV2 {
                inventory,
                remaining_deposit: 0,
                blocked_reasons: if disabled {
                    LivingSortedVecV2::new(vec![LivingBlockedReasonV2::Disabled])
                        .map_err(Fault::from)?
                } else {
                    LivingSortedVecV2::default()
                },
            });
            self.commit_stage = 0;
            self.commit_index = 0;
            self.transition(Stage::RecipeCommit);
        } else {
            let minimum = self
                .minimum_deposits
                .get(&world)
                .and_then(BTreeSet::first)
                .map_or(0, |row| row.0);
            self.recipe = Some(LivingRecipeJobV2::begin_shared(
                inventory,
                minimum,
                plan.deposit,
                Arc::clone(&plan.inputs),
                Arc::clone(&plan.outputs),
            ));
        }
        Ok(true)
    }
    fn commit_recipe(&mut self) -> Result<bool, Error> {
        let target = self.target.ok_or(Error::NotReady)?;
        let world = match target {
            Target::Hub(world, _) => world,
            Target::Facility(id) => self.facilities.get(&id).ok_or(Error::NotReady)?.world,
        };
        let position = self.position(world)?;
        match self.commit_stage {
            0 => {
                let output = self.recipe_output.as_ref().ok_or(Error::NotReady)?;
                self.state
                    .worlds
                    .record_at_mut(position)
                    .ok_or(Error::NotReady)?
                    .inventory = output.inventory;
                self.commit_stage = 1;
                Ok(true)
            }
            1 => {
                let output = self.recipe_output.as_ref().ok_or(Error::NotReady)?;
                let plan = self.active_plan.as_ref().ok_or(Error::NotReady)?;
                if output.blocked_reasons.as_slice().is_empty() && plan.deposit > 0 {
                    let position = *self.first_deposits.get(&world).ok_or(Error::NotReady)?;
                    let deposit = self
                        .state
                        .deposits
                        .record_at_mut(position)
                        .ok_or(Error::NotReady)?;
                    let before = (deposit.remaining_units, deposit.id, position);
                    deposit.remaining_units = deposit
                        .remaining_units
                        .checked_sub(u64::from(plan.deposit))
                        .ok_or(Fault::Arithmetic)?;
                    let after = (deposit.remaining_units, deposit.id, position);
                    let minimum = self
                        .minimum_deposits
                        .get_mut(&world)
                        .ok_or(Error::NotReady)?;
                    minimum.remove(&before);
                    minimum.insert(after);
                    self.commit_stage = 2;
                    return Ok(true);
                }
                self.commit_stage = 2;
                Ok(false)
            }
            2 => {
                // Check the deadline while recipe output remains job-owned on failure.
                if let Target::Hub(world, clock) = target {
                    let hub = &self.colonies.get(&world).ok_or(Error::NotReady)?.hub;
                    match clock {
                        0 => {
                            add(hub.energy_next_due, 10)?;
                        }
                        1 => {
                            add(hub.ore_next_due, 100)?;
                        }
                        _ => {
                            add(hub.fallback_next_due, 100)?;
                        }
                    }
                }
                let output = self.recipe_output.take().ok_or(Error::NotReady)?;
                match target {
                    Target::Hub(world, clock) => {
                        let colony = self.colonies.get_mut(&world).ok_or(Error::NotReady)?;
                        let reasons = output.blocked_reasons;
                        let replace = clock == 0
                            || (clock == 1 && colony.hub.blocked_reasons.as_slice().is_empty())
                            || (clock > 1 && !reasons.as_slice().is_empty());
                        if replace {
                            self.retired_reasons = std::mem::take(&mut colony.hub.blocked_reasons)
                                .into_vec()
                                .into_iter();
                            colony.hub.blocked_reasons = reasons;
                        } else {
                            self.retired_reasons = reasons.into_vec().into_iter();
                        }
                        match clock {
                            0 => {
                                colony.hub.energy_next_due = add(colony.hub.energy_next_due, 10)?;
                            }
                            1 => {
                                colony.hub.ore_next_due = add(colony.hub.ore_next_due, 100)?;
                            }
                            _ => {
                                colony.hub.fallback_next_due =
                                    add(colony.hub.fallback_next_due, 100)?;
                            }
                        }
                        self.target = None;
                        self.retired_plan = self.active_plan.take();
                        self.transition(Stage::RetireRecipe);
                    }
                    Target::Facility(id) => {
                        self.facts = LivingEventFactsV2 {
                            actor: self.worlds.get(&world).ok_or(Error::NotReady)?.1,
                            entity: Some(id),
                            ..Default::default()
                        };
                        self.commit_reasons = output.blocked_reasons.into_vec().into_iter();
                        self.commit_stage = 3;
                    }
                }
                Ok(true)
            }
            3 => {
                if let Some(reason) = self.commit_reasons.next() {
                    self.pending_reason = Some(reason);
                    self.commit_stage = 4;
                    Ok(true)
                } else {
                    self.commit_stage = 5;
                    self.commit_index = 0;
                    Ok(false)
                }
            }
            4 => {
                let reason = self.pending_reason.take().ok_or(Error::NotReady)?;
                self.facts.blocked_reasons.push(reason);
                self.committed_reasons
                    .push_sorted(reason)
                    .map_err(Fault::from)?;
                self.commit_stage = 3;
                Ok(true)
            }
            5 | 6 => {
                if self.facts.blocked_reasons.is_empty() {
                    let plan = self.active_plan.as_ref().ok_or(Error::NotReady)?;
                    let (role, amounts) = if self.commit_stage == 5 {
                        (LivingEventResourceRoleV2::Input, &plan.inputs)
                    } else {
                        (LivingEventResourceRoleV2::Output, &plan.outputs)
                    };
                    if let Some((resource, quantity)) = amounts.get(self.commit_index) {
                        self.resource_facts.insert(
                            (role, *resource),
                            LivingEventResourceFactV2 {
                                role,
                                resource: *resource,
                                quantity: *quantity,
                            },
                        );
                        self.commit_index += 1;
                        return Ok(true);
                    }
                }
                self.commit_stage += 1;
                self.commit_index = 0;
                Ok(false)
            }
            7 => {
                if let Some((_, fact)) = self.resource_facts.pop_first() {
                    self.facts.resources.push(fact);
                    Ok(true)
                } else {
                    self.commit_stage = 8;
                    Ok(false)
                }
            }
            8 => {
                let Target::Facility(id) = target else {
                    return Err(Fault::Invariant.into());
                };
                let blocked = !self.facts.blocked_reasons.is_empty();
                let plan = self.active_plan.as_ref().ok_or(Error::NotReady)?;
                let row = self.facilities.get_mut(&id).ok_or(Error::NotReady)?;
                row.next_due = Some(add(
                    row.next_due.ok_or(Error::NotReady)?,
                    plan.period.ok_or(Error::NotReady)?,
                )?);
                self.retired_reasons = std::mem::replace(
                    &mut row.blocked_reasons,
                    std::mem::take(&mut self.committed_reasons),
                )
                .into_vec()
                .into_iter();
                row.status = if blocked {
                    LivingFacilityStatusV2::Blocked
                } else {
                    LivingFacilityStatusV2::Operational
                };
                if !blocked && plan.deposit > 0 {
                    self.facts.deposit_input = Some(LivingDepositInputFactV2 {
                        world,
                        units: plan.deposit,
                    });
                }
                self.commit_stage = 9;
                Ok(true)
            }
            _ => {
                let blocked = !self.facts.blocked_reasons.is_empty();
                let facts = std::mem::take(&mut self.facts);
                self.event(
                    if blocked {
                        LivingEventKindV2::FacilityBlocked
                    } else {
                        LivingEventKindV2::FacilityProduced
                    },
                    facts,
                )?;
                self.target = None;
                self.retired_plan = self.active_plan.take();
                self.transition(Stage::RetireRecipe);
                Ok(true)
            }
        }
    }
    fn visit(&mut self) -> Result<bool, Error> {
        match self.stage {
            Stage::Catalog => {
                let Some(definition) = self.catalog.industries().get(self.catalog_index) else {
                    self.transition(Stage::HullDefinitions);
                    return Ok(false);
                };
                let (period, deposit, hp) = match definition.effect {
                    LivingIndustryEffectV2::Recipe {
                        period_ticks,
                        deposit_units,
                        ..
                    } => (Some(u64::from(period_ticks)), deposit_units, 0),
                    LivingIndustryEffectV2::Defense { hit_points, .. } => (None, 0, hit_points),
                    _ => (None, 0, 0),
                };
                self.plan = Some(Plan {
                    period,
                    deposit,
                    hp,
                    inputs: Arc::new(Vec::new()).into(),
                    outputs: Arc::new(Vec::new()).into(),
                });
                self.transition(Stage::Inputs);
                Ok(true)
            }
            Stage::Inputs | Stage::Outputs => {
                let definition = &self.catalog.industries()[self.catalog_index];
                if let LivingIndustryEffectV2::Recipe {
                    inputs, outputs, ..
                } = &definition.effect
                {
                    let amounts = if self.stage == Stage::Inputs {
                        inputs
                    } else {
                        outputs
                    };
                    if let Some(amount) = amounts.as_slice().get(self.index) {
                        let resource = match amount.resource.as_str() {
                            "energy" => LivingResourceV2::Energy,
                            "ore" => LivingResourceV2::Ore,
                            "alloy" => LivingResourceV2::Alloy,
                            _ => return Err(Fault::Invariant.into()),
                        };
                        let plan = self.plan.as_mut().ok_or(Error::NotReady)?;
                        let output = if self.stage == Stage::Inputs {
                            &mut plan.inputs
                        } else {
                            &mut plan.outputs
                        };
                        Arc::get_mut(output)
                            .ok_or(Error::NotReady)?
                            .push((resource, amount.quantity));
                        self.index += 1;
                        return Ok(true);
                    }
                }
                if self.stage == Stage::Inputs {
                    self.transition(Stage::Outputs);
                } else {
                    self.plans.insert(
                        definition.id.as_str().to_owned(),
                        self.plan.take().ok_or(Error::NotReady)?,
                    );
                    self.catalog_index += 1;
                    self.transition(Stage::Catalog);
                }
                Ok(false)
            }
            Stage::HullDefinitions => {
                if let Some(row) = self.catalog.hulls().get(self.index) {
                    self.hull_hit_points
                        .insert(row.id.as_str().to_owned(), row.hit_points);
                    self.index += 1;
                    Ok(true)
                } else {
                    self.transition(Stage::Worlds);
                    Ok(false)
                }
            }
            Stage::Worlds => {
                if let Some(world) = self.state.worlds.as_slice().get(self.index) {
                    self.worlds.insert(world.id, (self.index, world.owner));
                    self.index += 1;
                    Ok(true)
                } else {
                    self.transition(Stage::Deposits);
                    Ok(false)
                }
            }
            Stage::Deposits => {
                if let Some(row) = self.state.deposits.as_slice().get(self.index) {
                    if row.resource == LivingResourceV2::Ore {
                        self.minimum_deposits.entry(row.world).or_default().insert((
                            row.remaining_units,
                            row.id,
                            self.index,
                        ));
                        self.first_deposits.entry(row.world).or_insert(self.index);
                    }
                    self.index += 1;
                    Ok(true)
                } else {
                    self.transition(Stage::Colonies);
                    Ok(false)
                }
            }
            Stage::Colonies => {
                if let Some(row) = self.colony_source.next() {
                    self.hub_order.insert(row.world);
                    self.colonies.insert(row.world, row);
                    Ok(true)
                } else {
                    self.transition(Stage::Facilities);
                    Ok(false)
                }
            }
            Stage::Facilities => {
                if let Some(row) = self.facility_source.next() {
                    self.insert_facility(row);
                    Ok(true)
                } else {
                    self.transition(Stage::Fleets);
                    Ok(false)
                }
            }
            Stage::Fleets => {
                if let Some(mut row) = self.fleet_source.next() {
                    self.hull_source = std::mem::take(&mut row.hulls).into_vec().into_iter();
                    self.fleet = Some(row);
                    self.transition(Stage::HullRows);
                    Ok(true)
                } else {
                    self.transition(Stage::Construction);
                    Ok(false)
                }
            }
            Stage::HullRows => {
                if let Some(row) = self.hull_source.next() {
                    self.hulls.insert(row.id, row);
                    Ok(true)
                } else {
                    let fleet = self.fleet.take().ok_or(Error::NotReady)?;
                    self.fleets.insert(
                        fleet.id,
                        FleetRows {
                            fleet,
                            hulls: std::mem::take(&mut self.hulls),
                        },
                    );
                    self.transition(Stage::Fleets);
                    Ok(true)
                }
            }
            Stage::Construction => {
                let Some(job) = self.construction_source.next() else {
                    self.transition(Stage::HullJobs);
                    return Ok(false);
                };
                let due = job.completion_tick == self.tick;
                self.pending_construction = Some(job);
                self.transition(if due {
                    Stage::ConstructionCreate
                } else {
                    Stage::ConstructionKeep
                });
                Ok(true)
            }
            Stage::ConstructionKeep => {
                self.state
                    .construction_jobs
                    .push_sorted(self.pending_construction.take().ok_or(Error::NotReady)?)
                    .map_err(Fault::from)?;
                self.transition(Stage::Construction);
                Ok(true)
            }
            Stage::ConstructionCreate => {
                let job = self.pending_construction.take().ok_or(Error::NotReady)?;
                let plan = self
                    .plans
                    .get(job.definition.as_str())
                    .cloned()
                    .ok_or(Error::NotReady)?;
                let id = self.id(
                    job.owner,
                    1,
                    LivingEntityKindV2::Facility,
                    self.construction_ordinal,
                );
                self.construction_ordinal = self
                    .construction_ordinal
                    .checked_add(1)
                    .ok_or(Fault::Arithmetic)?;
                self.insert_facility(LivingFacilityV2 {
                    id,
                    world: job.world,
                    definition: job.definition,
                    status: LivingFacilityStatusV2::Operational,
                    paid_alloy: job.paid_alloy,
                    hit_points: plan.hp,
                    next_due: plan
                        .period
                        .map(|period| add(self.tick, period))
                        .transpose()?,
                    next_repair: None,
                    blocked_reasons: LivingSortedVecV2::default(),
                    disabled: false,
                });
                self.completion_event = Some((
                    LivingEventKindV2::ConstructionCompleted,
                    LivingEventFactsV2 {
                        actor: Some(job.owner),
                        entity: Some(id),
                        ..Default::default()
                    },
                ));
                self.transition(Stage::ConstructionEvent);
                Ok(true)
            }
            Stage::ConstructionEvent => {
                let (kind, facts) = self.completion_event.take().ok_or(Error::NotReady)?;
                self.event(kind, facts)?;
                self.transition(Stage::Construction);
                Ok(true)
            }
            Stage::HullJobs => {
                let Some(job) = self.job_source.next() else {
                    self.transition(Stage::Hubs);
                    return Ok(false);
                };
                let due = job.completion_tick == self.tick;
                self.pending_hull_job = Some(job);
                self.transition(if due {
                    Stage::HullCreate
                } else {
                    Stage::HullKeep
                });
                Ok(true)
            }
            Stage::HullKeep => {
                self.state
                    .hull_jobs
                    .push_sorted(self.pending_hull_job.take().ok_or(Error::NotReady)?)
                    .map_err(Fault::from)?;
                self.transition(Stage::HullJobs);
                Ok(true)
            }
            Stage::HullCreate => {
                let job = self.pending_hull_job.as_ref().ok_or(Error::NotReady)?;
                let hp = self
                    .hull_hit_points
                    .get(job.definition.as_str())
                    .copied()
                    .ok_or(Error::NotReady)?;
                let id = self.id(job.owner, 1, LivingEntityKindV2::Hull, self.hull_ordinal);
                self.pending_hull = Some(LivingHullV2 {
                    id,
                    definition: job.definition.clone(),
                    hit_points: hp,
                    return_credit: false,
                    next_repair: None,
                });
                self.completion_event = Some((
                    LivingEventKindV2::HullCompleted,
                    LivingEventFactsV2 {
                        actor: Some(job.owner),
                        entity: Some(id),
                        ..Default::default()
                    },
                ));
                self.transition(Stage::HullFleet);
                Ok(true)
            }
            Stage::HullFleet => {
                let job = self.pending_hull_job.as_ref().ok_or(Error::NotReady)?;
                if let Some(id) = job.target_fleet {
                    self.completion_fleet = Some(id);
                    self.transition(Stage::HullInsert);
                    Ok(false)
                } else {
                    let id = self.id(job.owner, 2, LivingEntityKindV2::Fleet, self.hull_ordinal);
                    self.fleets.insert(
                        id,
                        FleetRows {
                            fleet: LivingFleetV2 {
                                id,
                                owner: job.owner,
                                location: LivingFleetLocationV2::Docked { world: job.world },
                                order: None,
                                hulls: LivingSortedVecV2::default(),
                            },
                            hulls: BTreeMap::new(),
                        },
                    );
                    self.completion_fleet = Some(id);
                    self.transition(Stage::HullInsert);
                    Ok(true)
                }
            }
            Stage::HullInsert => {
                let fleet_id = self.completion_fleet.take().ok_or(Error::NotReady)?;
                let hull = self.pending_hull.take().ok_or(Error::NotReady)?;
                self.fleets
                    .get_mut(&fleet_id)
                    .ok_or(Error::NotReady)?
                    .hulls
                    .insert(hull.id, hull);
                self.hull_ordinal = self.hull_ordinal.checked_add(1).ok_or(Fault::Arithmetic)?;
                self.pending_hull_job = None;
                self.transition(Stage::HullEvent);
                Ok(true)
            }
            Stage::HullEvent => {
                let (kind, facts) = self.completion_event.take().ok_or(Error::NotReady)?;
                self.event(kind, facts)?;
                self.transition(Stage::HullJobs);
                Ok(true)
            }
            Stage::Hubs => {
                if self.hub.is_none() {
                    self.hub = self.hub_order.pop_first();
                    self.hub_clock = 0;
                    if self.hub.is_none() {
                        self.transition(Stage::Recipes);
                        return Ok(false);
                    }
                }
                let world = self.hub.ok_or(Error::NotReady)?;
                let hub = &self.colonies.get(&world).ok_or(Error::NotReady)?.hub;
                let clock = self.hub_clock;
                let due = match clock {
                    0 => hub.energy_next_due,
                    1 => hub.ore_next_due,
                    _ => hub.fallback_next_due,
                };
                self.hub_clock += 1;
                if self.hub_clock == 3 {
                    self.hub = None;
                }
                if due <= self.tick {
                    let (inputs, outputs) = match clock {
                        0 => (vec![], vec![(LivingResourceV2::Energy, 2)]),
                        1 => (vec![], vec![(LivingResourceV2::Ore, 1)]),
                        _ => (
                            vec![(LivingResourceV2::Energy, 4), (LivingResourceV2::Ore, 4)],
                            vec![(LivingResourceV2::Alloy, 1)],
                        ),
                    };
                    self.start_recipe(
                        Target::Hub(world, clock),
                        world,
                        Plan {
                            period: None,
                            deposit: 0,
                            hp: 0,
                            inputs: Arc::new(inputs).into(),
                            outputs: Arc::new(outputs).into(),
                        },
                    )?;
                }
                Ok(true)
            }
            Stage::Recipes => {
                let Some((_, id)) = self.recipe_order.pop_first() else {
                    self.transition(Stage::FacilityOutput);
                    return Ok(false);
                };
                let row = self.facilities.get(&id).ok_or(Error::NotReady)?;
                if row.next_due.is_some_and(|due| due <= self.tick) {
                    let plan = self
                        .plans
                        .get(row.definition.as_str())
                        .cloned()
                        .ok_or(Error::NotReady)?;
                    self.start_recipe(Target::Facility(id), row.world, plan)?;
                }
                Ok(true)
            }
            Stage::FacilityOutput => {
                if let Some((_, row)) = self.facilities.first_key_value() {
                    super::ensure_retained_sorted(&self.state.facilities, row)?;
                }
                if let Some((_, row)) = self.facilities.pop_first() {
                    self.state
                        .facilities
                        .push_sorted(row)
                        .map_err(Fault::from)?;
                    Ok(true)
                } else {
                    self.transition(Stage::ColonyOutput);
                    Ok(false)
                }
            }
            Stage::ColonyOutput => {
                if let Some((_, row)) = self.colonies.first_key_value() {
                    super::ensure_retained_sorted(&self.state.colonies, row)?;
                }
                if let Some((_, row)) = self.colonies.pop_first() {
                    self.state.colonies.push_sorted(row).map_err(Fault::from)?;
                    Ok(true)
                } else {
                    self.transition(Stage::FleetOutput);
                    Ok(false)
                }
            }
            Stage::FleetOutput => {
                if let Some((_, rows)) = self.fleets.pop_first() {
                    self.fleet = Some(rows.fleet);
                    self.hulls = rows.hulls;
                    self.transition(Stage::HullOutput);
                    Ok(true)
                } else {
                    self.repair = Some(LivingRepairJobV2::begin_with_catalog(
                        (*self.catalog).clone(),
                        std::mem::take(&mut self.state),
                        self.tick,
                    ));
                    self.transition(Stage::Repair);
                    Ok(false)
                }
            }
            Stage::HullOutput => {
                if let Some((_, row)) = self.hulls.pop_first() {
                    self.fleet
                        .as_mut()
                        .ok_or(Error::NotReady)?
                        .hulls
                        .push_sorted(row)
                        .map_err(Fault::from)?;
                    Ok(true)
                } else {
                    super::ensure_retained_sorted(
                        &self.state.fleets,
                        self.fleet.as_ref().ok_or(Error::NotReady)?,
                    )?;
                    self.state
                        .fleets
                        .push_sorted(self.fleet.take().ok_or(Error::NotReady)?)
                        .map_err(Fault::from)?;
                    self.transition(Stage::FleetOutput);
                    Ok(true)
                }
            }
            Stage::RecipeStart => self.begin_recipe(),
            Stage::RecipeCommit => self.commit_recipe(),
            Stage::RetireRecipe => {
                if self.retired_reasons.next().is_some() || self.retire_plan_record() {
                    return Ok(true);
                }
                self.transition(self.recipe_resume);
                Ok(false)
            }
            Stage::RetireIndexes => {
                if self.retire_plan_record() {
                    return Ok(true);
                }
                if let Some((_, plan)) = self.plans.pop_first() {
                    self.retired_plan = Some(plan);
                    return Ok(true);
                }
                if let Some((_, values)) = self.minimum_deposits.first_key_value() {
                    if values.is_empty() {
                        self.minimum_deposits.pop_first();
                    } else {
                        self.minimum_deposits
                            .first_entry()
                            .ok_or(Error::NotReady)?
                            .get_mut()
                            .pop_first();
                    }
                    return Ok(true);
                }
                if self.hull_hit_points.pop_first().is_some()
                    || self.worlds.pop_first().is_some()
                    || self.first_deposits.pop_first().is_some()
                    || self.recipe_order.pop_first().is_some()
                    || self.hub_order.pop_first().is_some()
                {
                    return Ok(true);
                }
                self.stage = Stage::Ready;
                Ok(false)
            }
            Stage::Repair | Stage::Ready => Ok(false),
        }
    }
    // A shared plan releases only its handle. Last-owner resource arrays retire
    // one resource record per visit before their final allocation is released.
    fn retire_plan_record(&mut self) -> bool {
        let Some(plan) = self.retired_plan.take() else {
            return false;
        };
        self.retirement.start(plan);
        true
    }
    fn insert_facility(&mut self, row: LivingFacilityV2) {
        let group = match row.definition.as_str() {
            "solar_array" => Some(0),
            "extractor" => Some(1),
            "foundry" => Some(2),
            _ => None,
        };
        if let Some(group) = group {
            self.recipe_order.insert((group, row.id));
        }
        self.facilities.insert(row.id, row);
    }
}
fn add(tick: Tick, delta: u64) -> Result<Tick, Error> {
    tick.0
        .checked_add(delta)
        .map(Tick)
        .ok_or(Error::Fault(Fault::Arithmetic))
}

#[cfg(test)]
mod cleanup_tests {
    use super::*;
    #[test]
    fn production_cleanup_counts_last_owner_arrays_and_nested_indexes() {
        let authority = crate::living::builtin_living_sandbox_v2().unwrap();
        for budget in [1, 64, 1024] {
            let mut job = LivingProductionJobV2::begin_owned(
                authority.catalog_snapshot(),
                authority.state().clone(),
                LivingPendingEventsV2::default(),
                Tick(1),
                [0; 32],
                authority.branch(),
            );
            // Controlled private-stage fixture: output already complete, caches retained.
            job.stage = Stage::RetireIndexes;
            job.plans.insert(
                "test".into(),
                Plan {
                    period: None,
                    deposit: 0,
                    hp: 0,
                    inputs: Arc::new(vec![(LivingResourceV2::Ore, 1); 70]).into(),
                    outputs: Arc::new(vec![(LivingResourceV2::Energy, 1); 90]).into(),
                },
            );
            let id = authority.state().worlds.as_slice()[0].id;
            job.minimum_deposits
                .insert(id, (0..10).map(|i| (i, id, i as usize)).collect());
            loop {
                let progress = job.poll(budget).unwrap();
                assert!(job.last_poll_visits() <= budget);
                if progress == Progress::Ready {
                    break;
                }
            }
            // Per Plan array: 3 visits/resource plus 8 ownership/storage visits.
            // Two arrays: 480+16; Plan and lane shells: 2; lane handoff:
            // 1; plan-map extraction: 1; deposit entries/root: 11.
            assert_eq!(job.total_visits(), 480 + 16 + 2 + 1 + 1 + 11);
            assert!(job.plans.is_empty() && job.minimum_deposits.is_empty());
            assert!(job.retired_plan.is_none());
            assert!(job.finish().is_ok());
        }
    }
    #[test]
    fn production_cleanup_shared_plan_retains_resources_and_cancel_is_private() {
        let authority = crate::living::builtin_living_sandbox_v2().unwrap();
        let shared = Arc::new(vec![(LivingResourceV2::Ore, 1); 128]);
        let mut job = LivingProductionJobV2::begin_owned(
            authority.catalog_snapshot(),
            authority.state().clone(),
            LivingPendingEventsV2::default(),
            Tick(1),
            [0; 32],
            authority.branch(),
        );
        job.stage = Stage::RetireRecipe;
        job.recipe_resume = Stage::Ready;
        job.retired_plan = Some(Plan {
            period: None,
            deposit: 0,
            hp: 0,
            inputs: shared.clone().into(),
            outputs: Arc::new(Vec::new()).into(),
        });
        assert_eq!(job.poll(1), Ok(Progress::Pending));
        assert_eq!(job.last_poll_visits(), 1);
        assert_eq!(shared.len(), 128);
        job.cancel();
        assert!(matches!(job.finish(), Err(Error::Cancelled)));
        assert_eq!(shared.len(), 128);
    }
}

use super::{
    LivingPhaseRetirementJobV2, LivingStateRetirementPolicyV2, RetirementFields, RetirementPoll,
    retirement_node,
};

impl<'a> RetirementFields<'a> for LivingProductionJobV2<'a> {
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
            1 => Some(retirement_node(self.catalog.0.take(), policy)),
            2 => Some(retirement_node(std::mem::take(&mut self.state), policy)),
            3 => Some(retirement_node(std::mem::take(&mut self.events), policy)),
            4 => Some(retirement_node(std::mem::take(&mut self.plans), policy)),
            5 => Some(retirement_node(
                std::mem::take(&mut self.hull_hit_points),
                policy,
            )),
            6 => Some(retirement_node(std::mem::take(&mut self.plan), policy)),
            7 => Some(retirement_node(std::mem::take(&mut self.worlds), policy)),
            8 => Some(retirement_node(
                std::mem::take(&mut self.minimum_deposits),
                policy,
            )),
            9 => Some(retirement_node(
                std::mem::take(&mut self.first_deposits),
                policy,
            )),
            10 => Some(retirement_node(
                std::mem::take(&mut self.colony_source),
                policy,
            )),
            11 => Some(retirement_node(std::mem::take(&mut self.colonies), policy)),
            12 => Some(retirement_node(
                std::mem::take(&mut self.facility_source),
                policy,
            )),
            13 => Some(retirement_node(
                std::mem::take(&mut self.facilities),
                policy,
            )),
            14 => Some(retirement_node(
                std::mem::take(&mut self.recipe_order),
                policy,
            )),
            15 => Some(retirement_node(
                std::mem::take(&mut self.fleet_source),
                policy,
            )),
            16 => Some(retirement_node(std::mem::take(&mut self.fleets), policy)),
            17 => Some(retirement_node(std::mem::take(&mut self.fleet), policy)),
            18 => Some(retirement_node(
                std::mem::take(&mut self.hull_source),
                policy,
            )),
            19 => Some(retirement_node(std::mem::take(&mut self.hulls), policy)),
            20 => Some(retirement_node(
                std::mem::take(&mut self.construction_source),
                policy,
            )),
            21 => Some(retirement_node(
                std::mem::take(&mut self.pending_construction),
                policy,
            )),
            22 => Some(retirement_node(
                std::mem::take(&mut self.pending_hull_job),
                policy,
            )),
            23 => Some(retirement_node(
                std::mem::take(&mut self.pending_hull),
                policy,
            )),
            24 => Some(retirement_node(
                std::mem::take(&mut self.completion_event),
                policy,
            )),
            25 => Some(retirement_node(
                std::mem::take(&mut self.job_source),
                policy,
            )),
            26 => Some(retirement_node(std::mem::take(&mut self.hub_order), policy)),
            27 => Some(retirement_node(std::mem::take(&mut self.recipe), policy)),
            28 => Some(retirement_node(
                std::mem::take(&mut self.active_plan),
                policy,
            )),
            29 => Some(retirement_node(
                std::mem::take(&mut self.retired_plan),
                policy,
            )),
            30 => Some(retirement_node(
                std::mem::take(&mut self.retired_reasons),
                policy,
            )),
            31 => Some(retirement_node(
                std::mem::take(&mut self.recipe_output),
                policy,
            )),
            32 => Some(retirement_node(
                std::mem::take(&mut self.commit_reasons),
                policy,
            )),
            33 => Some(retirement_node(
                std::mem::take(&mut self.committed_reasons),
                policy,
            )),
            34 => Some(retirement_node(
                std::mem::take(&mut self.pending_reason),
                policy,
            )),
            35 => Some(retirement_node(std::mem::take(&mut self.facts), policy)),
            36 => Some(retirement_node(
                std::mem::take(&mut self.resource_facts),
                policy,
            )),
            37 => Some(retirement_node(std::mem::take(&mut self.repair), policy)),
            _ => None,
        }
    }
}

#[allow(dead_code)]
impl<'a> LivingProductionJobV2<'a> {
    pub(crate) fn into_phase_retirement(
        self,
        policy: LivingStateRetirementPolicyV2,
    ) -> LivingPhaseRetirementJobV2<'a, Self> {
        LivingPhaseRetirementJobV2::begin(self, policy)
    }
}
impl<'a> RetirementFields<'a> for Plan {
    fn retirement_child(
        &mut self,
        index: usize,
        policy: LivingStateRetirementPolicyV2,
    ) -> Option<Box<dyn RetirementPoll + 'a>> {
        match index {
            0 => Some(retirement_node(self.inputs.0.take(), policy)),
            1 => Some(retirement_node(self.outputs.0.take(), policy)),
            _ => None,
        }
    }
}
impl<'a> RetirementFields<'a> for FleetRows {
    const CUSTOM_NODE: bool = true;
    fn retirement_child(
        &mut self,
        _: usize,
        _: LivingStateRetirementPolicyV2,
    ) -> Option<Box<dyn RetirementPoll + 'a>> {
        unreachable!("split retained fleet/hull map roots")
    }
    fn retirement_node(
        self,
        policy: LivingStateRetirementPolicyV2,
    ) -> Box<dyn RetirementPoll + 'a> {
        retirement_node((self.fleet, self.hulls), policy)
    }
}

#[cfg(test)]
mod phase_retirement_tests {
    use super::*;
    #[test]
    fn phase_retirement_production() {
        let authority = crate::living::builtin_living_sandbox_v2().unwrap();
        super::super::phase_retirement_tests::exercise(
            || {
                LivingProductionJobV2::begin_owned(
                    authority.catalog_snapshot(),
                    authority.state().clone(),
                    LivingPendingEventsV2::new(),
                    Tick(100),
                    [0; 32],
                    LivingBranchIdV2([0; 16]),
                )
            },
            |j, b| j.poll(b),
            |j, f| {
                if f {
                    j.fault = Some(Fault::Arithmetic)
                } else {
                    j.cancel()
                }
            },
            |j| j.take_ready_result().map(|_| ()),
            |j| format!("{:?}", std::mem::discriminant(&j.stage)),
        );
    }
}

#[cfg(test)]
mod phase_retirement_plan_tests {
    use super::*;
    #[test]
    fn phase_retirement_plan_aliases_release_exact_last_array_owner() {
        let authority = crate::living::builtin_living_sandbox_v2().unwrap();
        let mut expected = None;
        for schedule in [&[1][..], &[64][..], &[1024][..], &[1, 64, 7, 1024][..]] {
            let resources = Arc::new(vec![(LivingResourceV2::Ore, 2); 17]);
            let weak = Arc::downgrade(&resources);
            let plan = Plan {
                period: Some(10),
                deposit: 0,
                hp: 0,
                inputs: resources.clone().into(),
                outputs: resources.clone().into(),
            };
            let mut owner = LivingProductionJobV2::begin_owned(
                authority.catalog_snapshot(),
                LivingGalaxyStateV2::default(),
                LivingPendingEventsV2::new(),
                Tick(1),
                [0; 32],
                authority.branch(),
            );
            owner.plans.insert("alias".into(), plan.clone());
            owner.active_plan = Some(plan.clone());
            owner.retired_plan = Some(plan);
            owner.recipe = Some(LivingRecipeJobV2::begin_shared(
                Default::default(),
                0,
                0,
                resources.clone(),
                resources.clone(),
            ));
            drop(resources);
            let mut job = owner.into_phase_retirement(LivingStateRetirementPolicyV2::ReleaseHandle);
            let mut i = 0;
            while job.poll(schedule[i % schedule.len()]).unwrap() != Progress::Ready {
                i += 1;
                assert!(i < 10000);
                assert!(job.take_deferred_owner().is_none());
            }
            assert!(weak.upgrade().is_none());
            if let Some(expected) = expected {
                assert_eq!(job.total_visits(), expected);
            } else {
                expected = Some(job.total_visits());
            }
        }
    }
}
