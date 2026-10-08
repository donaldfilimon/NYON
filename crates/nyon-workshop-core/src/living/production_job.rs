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
use super::super::receipt::{
    LivingEventKindV2, LivingEventProvenanceV2, LivingPendingEventAppendErrorV2,
    LivingPendingEventAppendJobV2, LivingPendingEventsV2,
};
use super::super::simulation::LivingDeterministicFaultV2 as Fault;
use super::super::wire::LivingSortedVecV2;
use super::{
    LIVING_MAX_PHASE_POLL_VISITS_V2, LivingLifecycleJobErrorV2 as Error,
    LivingLifecycleProgressV2 as Progress, LivingRecipeJobV2, LivingRepairJobV2,
    LivingSortedStorageJobV2, LivingStorageJobErrorV2, LivingVecStorageJobV2,
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

// Only these fixed causes arise from production, recipe or repair. No recursive
// deterministic envelope is retained as scalar metadata.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum ProductionFailure {
    Invariant,
    ProtocolInvariant,
    Arithmetic,
    KeyedOrder,
    ExplainedFacts,
    ExplainedTooManyEvents,
    ExplainedJson,
    ExplainedCapacity,
}
impl ProductionFailure {
    pub(crate) fn capture(fault: &Fault) -> Self {
        use super::super::event_facts::LivingEventFactsErrorV2 as Facts;
        use super::super::receipt::LivingReceiptErrorV2 as Receipt;
        use super::super::wire::LivingWireErrorV2 as Wire;
        match fault {
            Fault::Invariant => Self::Invariant,
            Fault::Arithmetic => Self::Arithmetic,
            Fault::Wire(Wire::KeyedOrder) => Self::KeyedOrder,
            Fault::EventFacts(Facts::Facts) => Self::ExplainedFacts,
            Fault::EventFacts(Facts::Receipt(Receipt::TooManyEvents { limit }))
                if *limit == super::super::receipt::LIVING_MAX_BOUNDARY_EVENTS_V2 =>
            {
                Self::ExplainedTooManyEvents
            }
            Fault::EventFacts(Facts::Wire(Wire::Json)) => Self::ExplainedJson,
            Fault::EventFacts(Facts::Capacity) => Self::ExplainedCapacity,
            // These variants cannot be constructed by the current phase or its
            // two children. Classify a protocol violation, never retain payloads.
            Fault::ViewingHistory
            | Fault::Capacity
            | Fault::Replay { .. }
            | Fault::Validation(_)
            | Fault::Wire(_)
            | Fault::Receipt(_)
            | Fault::Decision(_)
            | Fault::EventFacts(_) => Self::ProtocolInvariant,
        }
    }
    pub(crate) fn public(self) -> Fault {
        use super::super::event_facts::LivingEventFactsErrorV2 as Facts;
        use super::super::receipt::LivingReceiptErrorV2 as Receipt;
        use super::super::wire::LivingWireErrorV2 as Wire;
        match self {
            Self::Invariant | Self::ProtocolInvariant => Fault::Invariant,
            Self::Arithmetic => Fault::Arithmetic,
            Self::KeyedOrder => Fault::Wire(Wire::KeyedOrder),
            Self::ExplainedFacts => Fault::EventFacts(Facts::Facts),
            Self::ExplainedTooManyEvents => {
                Fault::EventFacts(Facts::Receipt(Receipt::TooManyEvents {
                    limit: super::super::receipt::LIVING_MAX_BOUNDARY_EVENTS_V2,
                }))
            }
            Self::ExplainedJson => Fault::EventFacts(Facts::Wire(Wire::Json)),
            Self::ExplainedCapacity => Fault::EventFacts(Facts::Capacity),
        }
    }
}

enum Preparation<'a> {
    PlanInputs(LivingVecStorageJobV2<'a, (LivingResourceV2, u32)>),
    PlanOutputs(LivingVecStorageJobV2<'a, (LivingResourceV2, u32)>),
    Construction(LivingSortedStorageJobV2<'a, LivingConstructionJobV2>),
    HullJobs(LivingSortedStorageJobV2<'a, LivingHullJobV2>),
    Facilities(LivingSortedStorageJobV2<'a, LivingFacilityV2>),
    Colonies(LivingSortedStorageJobV2<'a, LivingColonyV2>),
    Fleets(LivingSortedStorageJobV2<'a, LivingFleetV2>),
    Hulls(LivingSortedStorageJobV2<'a, LivingHullV2>),
    FactReasons(LivingVecStorageJobV2<'a, LivingBlockedReasonV2>),
    Reasons(LivingSortedStorageJobV2<'a, LivingBlockedReasonV2>),
    Resources(LivingVecStorageJobV2<'a, LivingEventResourceFactV2>),
}
impl<'a> Preparation<'a> {
    fn poll(&mut self, budget: usize) -> Result<Progress, LivingStorageJobErrorV2> {
        match self {
            Self::PlanInputs(job) => job.poll(budget),
            Self::PlanOutputs(job) => job.poll(budget),
            Self::Construction(job) => job.poll(budget),
            Self::HullJobs(job) => job.poll(budget),
            Self::Facilities(job) => job.poll(budget),
            Self::Colonies(job) => job.poll(budget),
            Self::Fleets(job) => job.poll(budget),
            Self::Hulls(job) => job.poll(budget),
            Self::FactReasons(job) => job.poll(budget),
            Self::Reasons(job) => job.poll(budget),
            Self::Resources(job) => job.poll(budget),
        }
    }
    fn last_poll_visits(&self) -> usize {
        match self {
            Self::PlanInputs(job) => job.last_poll_visits(),
            Self::PlanOutputs(job) => job.last_poll_visits(),
            Self::Construction(job) => job.last_poll_visits(),
            Self::HullJobs(job) => job.last_poll_visits(),
            Self::Facilities(job) => job.last_poll_visits(),
            Self::Colonies(job) => job.last_poll_visits(),
            Self::Fleets(job) => job.last_poll_visits(),
            Self::Hulls(job) => job.last_poll_visits(),
            Self::FactReasons(job) => job.last_poll_visits(),
            Self::Reasons(job) => job.last_poll_visits(),
            Self::Resources(job) => job.last_poll_visits(),
        }
    }
}
impl<'a> RetirementFields<'a> for Preparation<'a> {
    fn retirement_child(
        &mut self,
        index: usize,
        policy: LivingStateRetirementPolicyV2,
    ) -> Option<Box<dyn RetirementPoll + 'a>> {
        match self {
            Self::PlanInputs(job) => job.retirement_child(index, policy),
            Self::PlanOutputs(job) => job.retirement_child(index, policy),
            Self::Construction(job) => job.retirement_child(index, policy),
            Self::HullJobs(job) => job.retirement_child(index, policy),
            Self::Facilities(job) => job.retirement_child(index, policy),
            Self::Colonies(job) => job.retirement_child(index, policy),
            Self::Fleets(job) => job.retirement_child(index, policy),
            Self::Hulls(job) => job.retirement_child(index, policy),
            Self::FactReasons(job) => job.retirement_child(index, policy),
            Self::Reasons(job) => job.retirement_child(index, policy),
            Self::Resources(job) => job.retirement_child(index, policy),
        }
    }
}

/// Complete scheduled production, with a shared 1..=1024 visit budget.
/// Construction and hull identities and recipe ordering match the fixed phase.
/// No partial candidate escapes a cancelled, failed, or incomplete job.
pub struct LivingProductionJobV2<'a> {
    retirement: super::PhaseRetirementLane<'a>,
    preparation: Option<Preparation<'a>>,
    append: Option<LivingPendingEventAppendJobV2>,
    after_append: Option<Stage>,
    pending_facility: Option<LivingFacilityV2>,
    pending_colony: Option<LivingColonyV2>,
    pending_resource: Option<LivingEventResourceFactV2>,
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
    repair_exclusions: BTreeSet<Id>,
    repair: Option<LivingRepairJobV2<'a>>,
    cancelled: bool,
    fault: Option<ProductionFailure>,
    preparation_capacity: bool,
    last_poll_visits: usize,
    total_visits: u64,
}
impl<'a> LivingProductionJobV2<'a> {
    pub(crate) fn with_repair_exclusions(mut self, exclusions: BTreeSet<Id>) -> Self {
        self.repair_exclusions = exclusions;
        self
    }
    #[cfg(test)]
    pub(crate) fn retains_catalog_for_test(&self) -> bool {
        self.catalog.0.is_some()
    }
    #[cfg(test)]
    pub(crate) fn inject_repair_target_failure(&mut self) -> bool {
        let Some(child) = self.repair.as_mut() else {
            return false;
        };
        if child.cursor != super::RepairCursor::Update {
            return false;
        }
        let Some(target) = child.pending.as_mut() else {
            return false;
        };
        target.world = Id([255; 16]);
        true
    }
    #[cfg(test)]
    pub(crate) fn inject_recipe_capacity_failure(&mut self) -> bool {
        self.recipe
            .as_mut()
            .is_some_and(LivingRecipeJobV2::inject_capacity_failure)
    }
    #[cfg(test)]
    pub(crate) fn recipe_identity(&self) -> Option<*const LivingRecipeJobV2> {
        self.recipe.as_ref().map(std::ptr::from_ref)
    }
    #[cfg(test)]
    pub(crate) fn recipe_visits(&self) -> Option<(usize, u64)> {
        self.recipe
            .as_ref()
            .map(|recipe| (recipe.last_poll_visits(), recipe.total_visits()))
    }
    #[cfg(test)]
    pub(crate) fn recipe_preparation_failed(&self) -> bool {
        self.preparation_capacity
            && self.fault.is_none()
            && self.recipe_output.is_none()
            && self.stage == Stage::RecipeStart
    }

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
            preparation: None,
            append: None,
            after_append: None,
            pending_facility: None,
            pending_colony: None,
            pending_resource: None,
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
            repair_exclusions: BTreeSet::new(),
            repair: None,
            cancelled: false,
            fault: None,
            preparation_capacity: false,
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
            return Err(if self.preparation_capacity {
                Error::PreparationCapacity
            } else {
                self.fault
                    .map_or(Error::Cancelled, |cause| Error::Fault(cause.public()))
            });
        }
        if budget == 0 || budget > LIVING_MAX_PHASE_POLL_VISITS_V2 {
            return Err(Error::Budget);
        }
        if self.preparation_capacity {
            return Err(Error::PreparationCapacity);
        }
        if let Some(fault) = self.fault {
            return Err(Error::Fault(fault.public()));
        }
        while self.stage != Stage::Ready && self.last_poll_visits < budget {
            if self.retirement.visit() {
                self.last_poll_visits += 1;
                self.total_visits += 1;
                continue;
            }

            if self.append.is_none()
                && let Some(resume) = self.after_append.take()
            {
                if self.stage == Stage::RecipeCommit {
                    self.target = None;
                    self.retired_plan = self.active_plan.take();
                }
                self.transition(resume);
                self.last_poll_visits += 1;
                self.total_visits += 1;
                continue;
            }

            let delegated = self.preparation.is_some()
                || self.append.is_some()
                || self.recipe.is_some()
                || self.repair.is_some();
            let result = if self.preparation.is_some() {
                self.poll_preparation(budget)
            } else if self.append.is_some() {
                self.poll_append(budget)
            } else if let Some(recipe) = &mut self.recipe {
                let result = recipe.poll(budget - self.last_poll_visits);
                let used = recipe.last_poll_visits();
                self.last_poll_visits += used;
                self.total_visits += used as u64;
                match result {
                    Ok(Progress::Ready) => {
                        if self.last_poll_visits == budget {
                            break;
                        }
                        self.last_poll_visits += 1;
                        self.total_visits += 1;
                        match recipe.take_ready_result() {
                            Ok(output) => {
                                self.recipe_output = Some(output);
                                self.retirement
                                    .start(self.recipe.take().expect("retained recipe"));
                                self.commit_stage = 0;
                                self.commit_index = 0;
                                self.transition(Stage::RecipeCommit);
                                Ok(false)
                            }
                            Err(error) => Err(error),
                        }
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
                        self.last_poll_visits += 1;
                        self.total_visits += 1;
                        match repair.take_ready_result() {
                            Ok(state) => {
                                self.state = state;
                                self.retirement
                                    .start(self.repair.take().expect("retained repair"));
                                self.transition(Stage::RetireIndexes);
                                Ok(false)
                            }
                            Err(error) => Err(error),
                        }
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
                    let cause = ProductionFailure::capture(&fault);
                    self.fault = Some(cause);
                    return Err(Error::Fault(cause.public()));
                }
                Err(Error::PreparationCapacity) => {
                    if !delegated && self.last_poll_visits < budget {
                        self.last_poll_visits += 1;
                        self.total_visits += 1;
                    }
                    self.preparation_capacity = true;
                    return Err(Error::PreparationCapacity);
                }
                Err(Error::Budget | Error::Cancelled | Error::NotReady) => {
                    if !delegated && self.last_poll_visits < budget {
                        self.last_poll_visits += 1;
                        self.total_visits += 1;
                    }
                    self.fault = Some(ProductionFailure::ProtocolInvariant);
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
    /// Legacy consuming extraction cannot preserve the caller owner on error.
    pub fn finish(mut self) -> Result<(LivingGalaxyStateV2, LivingPendingEventsV2), Error> {
        let result = self.take_ready_result();
        let mut retirement =
            self.into_phase_retirement(LivingStateRetirementPolicyV2::ReleaseHandle);
        while retirement
            .poll(LIVING_MAX_PHASE_POLL_VISITS_V2)
            .expect("valid retirement budget")
            != Progress::Ready
        {}
        result
    }
    /// Borrowed one-time extraction; rejected calls preserve every owner.
    pub fn take_ready_result(
        &mut self,
    ) -> Result<(LivingGalaxyStateV2, LivingPendingEventsV2), Error> {
        if self.cancelled {
            return Err(Error::Cancelled);
        }
        if self.preparation_capacity {
            return Err(Error::PreparationCapacity);
        }
        if let Some(fault) = self.fault {
            return Err(Error::Fault(fault.public()));
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
    fn event(&mut self, kind: LivingEventKindV2, facts: LivingEventFactsV2, resume: Stage) {
        self.after_append = Some(resume);
        self.append = Some(LivingPendingEventAppendJobV2::begin_explained(
            std::mem::take(&mut self.events),
            LivingEventProvenanceV2::Autonomous {
                phase: LivingPhaseV2::RunProduction,
            },
            kind,
            facts,
        ));
    }
    fn poll_append(&mut self, budget: usize) -> Result<bool, Error> {
        let append = self.append.as_mut().expect("retained append");
        let result = append.poll(budget - self.last_poll_visits);
        let used = append.last_poll_visits();
        self.last_poll_visits += used;
        self.total_visits += used as u64;
        if result.map_err(append_error)? == Progress::Ready && self.last_poll_visits < budget {
            self.last_poll_visits += 1;
            self.total_visits += 1;
            let (events, _) = append.take_ready_result().map_err(append_error)?;
            self.events = events;
            self.retirement
                .start(self.append.take().expect("exhausted append"));
        }
        Ok(false)
    }
    fn poll_preparation(&mut self, budget: usize) -> Result<bool, Error> {
        let preparation = self.preparation.as_mut().expect("retained preparation");
        let result = preparation.poll(budget - self.last_poll_visits);
        let used = preparation.last_poll_visits();
        self.last_poll_visits += used;
        self.total_visits += used as u64;
        if result.map_err(storage_error)? != Progress::Ready || self.last_poll_visits == budget {
            return Ok(false);
        }
        self.last_poll_visits += 1;
        self.total_visits += 1;
        match preparation {
            Preparation::PlanInputs(job) => {
                let destination =
                    Arc::get_mut(&mut self.plan.as_mut().ok_or(Error::NotReady)?.inputs)
                        .ok_or(Error::NotReady)?;
                *destination = job.take_ready_result().map_err(storage_error)?;
            }
            Preparation::PlanOutputs(job) => {
                let destination =
                    Arc::get_mut(&mut self.plan.as_mut().ok_or(Error::NotReady)?.outputs)
                        .ok_or(Error::NotReady)?;
                *destination = job.take_ready_result().map_err(storage_error)?;
            }
            Preparation::Construction(job) => {
                self.state.construction_jobs = job.take_ready_result().map_err(storage_error)?
            }
            Preparation::HullJobs(job) => {
                self.state.hull_jobs = job.take_ready_result().map_err(storage_error)?
            }
            Preparation::Facilities(job) => {
                self.state.facilities = job.take_ready_result().map_err(storage_error)?
            }
            Preparation::Colonies(job) => {
                self.state.colonies = job.take_ready_result().map_err(storage_error)?
            }
            Preparation::Fleets(job) => {
                self.state.fleets = job.take_ready_result().map_err(storage_error)?
            }
            Preparation::Hulls(job) => {
                let destination = &mut self.fleet.as_mut().ok_or(Error::NotReady)?.hulls;
                *destination = job.take_ready_result().map_err(storage_error)?;
            }
            Preparation::FactReasons(job) => {
                self.facts.blocked_reasons = job.take_ready_result().map_err(storage_error)?
            }
            Preparation::Reasons(job) => {
                self.committed_reasons = job.take_ready_result().map_err(storage_error)?
            }
            Preparation::Resources(job) => {
                self.facts.resources = job.take_ready_result().map_err(storage_error)?
            }
        }
        self.retirement
            .start(self.preparation.take().expect("exhausted preparation"));
        Ok(false)
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
                    self.retirement
                        .start(std::mem::take(&mut self.commit_reasons));
                    self.commit_stage = 5;
                    self.commit_index = 0;
                    Ok(true)
                }
            }
            4 => {
                let reason = self.pending_reason.as_ref().ok_or(Error::NotReady)?;
                super::ensure_retained_sorted(&self.committed_reasons, reason)?;
                if self.facts.blocked_reasons.len() == self.facts.blocked_reasons.capacity() {
                    self.preparation =
                        Some(Preparation::FactReasons(LivingVecStorageJobV2::begin(
                            std::mem::take(&mut self.facts.blocked_reasons),
                            self.commit_reasons.len() + 1,
                        )));
                    return Ok(true);
                }
                if self.committed_reasons.as_slice().len() == self.committed_reasons.capacity() {
                    self.preparation = Some(Preparation::Reasons(LivingSortedStorageJobV2::begin(
                        std::mem::take(&mut self.committed_reasons),
                        self.commit_reasons.len() + 1,
                    )));
                    return Ok(true);
                }
                let mut copy = Some(*reason);
                LivingVecStorageJobV2::commit_one(&mut self.facts.blocked_reasons, &mut copy)
                    .map_err(storage_error)?;
                LivingSortedStorageJobV2::commit_one(
                    &mut self.committed_reasons,
                    &mut self.pending_reason,
                )
                .map_err(storage_error)?;
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
                if self.pending_resource.is_none() {
                    if let Some((_, fact)) = self.resource_facts.pop_first() {
                        self.pending_resource = Some(fact);
                        return Ok(true);
                    }
                    self.commit_stage = 8;
                    return Ok(false);
                }
                if self.facts.resources.len() == self.facts.resources.capacity() {
                    self.preparation = Some(Preparation::Resources(LivingVecStorageJobV2::begin(
                        std::mem::take(&mut self.facts.resources),
                        self.resource_facts.len() + 1,
                    )));
                    return Ok(true);
                }
                LivingVecStorageJobV2::commit_one(
                    &mut self.facts.resources,
                    &mut self.pending_resource,
                )
                .map_err(storage_error)?;
                Ok(true)
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
                    Stage::RetireRecipe,
                );
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
                        let output = Arc::get_mut(output).ok_or(Error::NotReady)?;
                        if output.len() == output.capacity() {
                            let job = LivingVecStorageJobV2::begin(
                                std::mem::take(output),
                                amounts.as_slice().len() - self.index,
                            );
                            self.preparation = Some(if self.stage == Stage::Inputs {
                                Preparation::PlanInputs(job)
                            } else {
                                Preparation::PlanOutputs(job)
                            });
                            return Ok(true);
                        }
                        LivingVecStorageJobV2::commit_one(
                            output,
                            &mut Some((resource, amount.quantity)),
                        )
                        .map_err(storage_error)?;
                        self.index += 1;
                        return Ok(true);
                    }
                }
                if self.stage == Stage::Inputs {
                    self.transition(Stage::Outputs);
                } else {
                    let replaced = self.plans.insert(
                        definition.id.as_str().to_owned(),
                        self.plan.take().ok_or(Error::NotReady)?,
                    );
                    if let Some(replaced) = replaced {
                        self.retirement.start(replaced);
                    }
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
                    if let Some(replaced) = self.colonies.insert(row.world, row) {
                        self.retirement.start(replaced);
                    }
                    Ok(true)
                } else {
                    self.retirement
                        .start(std::mem::take(&mut self.colony_source));
                    self.transition(Stage::Facilities);
                    Ok(true)
                }
            }
            Stage::Facilities => {
                if let Some(row) = self.facility_source.next() {
                    self.insert_facility(row);
                    Ok(true)
                } else {
                    self.retirement
                        .start(std::mem::take(&mut self.facility_source));
                    self.transition(Stage::Fleets);
                    Ok(true)
                }
            }
            Stage::Fleets => {
                if let Some(mut row) = self.fleet_source.next() {
                    self.hull_source = std::mem::take(&mut row.hulls).into_vec().into_iter();
                    self.fleet = Some(row);
                    self.transition(Stage::HullRows);
                    Ok(true)
                } else {
                    self.retirement
                        .start(std::mem::take(&mut self.fleet_source));
                    self.transition(Stage::Construction);
                    Ok(true)
                }
            }
            Stage::HullRows => {
                if let Some(row) = self.hull_source.next() {
                    if let Some(replaced) = self.hulls.insert(row.id, row) {
                        self.retirement.start(replaced);
                    }
                    Ok(true)
                } else {
                    let fleet = self.fleet.take().ok_or(Error::NotReady)?;
                    let replaced = self.fleets.insert(
                        fleet.id,
                        FleetRows {
                            fleet,
                            hulls: std::mem::take(&mut self.hulls),
                        },
                    );
                    self.retirement
                        .start((std::mem::take(&mut self.hull_source), replaced));
                    self.transition(Stage::Fleets);
                    Ok(true)
                }
            }
            Stage::Construction => {
                let Some(job) = self.construction_source.next() else {
                    self.retirement
                        .start(std::mem::take(&mut self.construction_source));
                    self.transition(Stage::HullJobs);
                    return Ok(true);
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
                super::ensure_retained_sorted(
                    &self.state.construction_jobs,
                    self.pending_construction.as_ref().ok_or(Error::NotReady)?,
                )?;
                if self.state.construction_jobs.as_slice().len()
                    == self.state.construction_jobs.capacity()
                {
                    self.preparation =
                        Some(Preparation::Construction(LivingSortedStorageJobV2::begin(
                            std::mem::take(&mut self.state.construction_jobs),
                            self.construction_source.len() + 1,
                        )));
                    return Ok(true);
                }
                LivingSortedStorageJobV2::commit_one(
                    &mut self.state.construction_jobs,
                    &mut self.pending_construction,
                )
                .map_err(storage_error)?;
                self.transition(Stage::Construction);
                Ok(true)
            }
            Stage::ConstructionCreate => {
                let job = self.pending_construction.as_ref().ok_or(Error::NotReady)?;
                let plan = self
                    .plans
                    .get(job.definition.as_str())
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
                let next_due = plan
                    .period
                    .map(|period| add(self.tick, period))
                    .transpose()?;
                let hit_points = plan.hp;
                let job = self
                    .pending_construction
                    .take()
                    .expect("checked original construction");
                self.insert_facility(LivingFacilityV2 {
                    id,
                    world: job.world,
                    definition: job.definition,
                    status: LivingFacilityStatusV2::Operational,
                    paid_alloy: job.paid_alloy,
                    hit_points,
                    next_due,
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
                self.event(kind, facts, Stage::Construction);
                Ok(true)
            }
            Stage::HullJobs => {
                let Some(job) = self.job_source.next() else {
                    self.retirement.start(std::mem::take(&mut self.job_source));
                    self.transition(Stage::Hubs);
                    return Ok(true);
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
                super::ensure_retained_sorted(
                    &self.state.hull_jobs,
                    self.pending_hull_job.as_ref().ok_or(Error::NotReady)?,
                )?;
                if self.state.hull_jobs.as_slice().len() == self.state.hull_jobs.capacity() {
                    self.preparation =
                        Some(Preparation::HullJobs(LivingSortedStorageJobV2::begin(
                            std::mem::take(&mut self.state.hull_jobs),
                            self.job_source.len() + 1,
                        )));
                    return Ok(true);
                }
                LivingSortedStorageJobV2::commit_one(
                    &mut self.state.hull_jobs,
                    &mut self.pending_hull_job,
                )
                .map_err(storage_error)?;
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
                    let replaced = self.fleets.insert(
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
                    if let Some(replaced) = replaced {
                        self.retirement.start(replaced);
                    }
                    self.completion_fleet = Some(id);
                    self.transition(Stage::HullInsert);
                    Ok(true)
                }
            }
            Stage::HullInsert => {
                let fleet_id = self.completion_fleet.ok_or(Error::NotReady)?;
                let fleet = self.fleets.get_mut(&fleet_id).ok_or(Error::NotReady)?;
                let ordinal = self.hull_ordinal.checked_add(1).ok_or(Fault::Arithmetic)?;
                let hull = self.pending_hull.take().ok_or(Error::NotReady)?;
                let replaced = fleet.hulls.insert(hull.id, hull);
                self.hull_ordinal = ordinal;
                self.completion_fleet = None;
                self.retirement
                    .start((self.pending_hull_job.take(), replaced));
                self.transition(Stage::HullEvent);
                Ok(true)
            }
            Stage::HullEvent => {
                let (kind, facts) = self.completion_event.take().ok_or(Error::NotReady)?;
                self.event(kind, facts, Stage::HullJobs);
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
                if self.pending_facility.is_none() {
                    if let Some((_, row)) = self.facilities.pop_first() {
                        self.pending_facility = Some(row);
                        return Ok(true);
                    }
                    self.transition(Stage::ColonyOutput);
                    return Ok(false);
                }
                super::ensure_retained_sorted(
                    &self.state.facilities,
                    self.pending_facility.as_ref().ok_or(Error::NotReady)?,
                )?;
                if self.state.facilities.as_slice().len() == self.state.facilities.capacity() {
                    self.preparation =
                        Some(Preparation::Facilities(LivingSortedStorageJobV2::begin(
                            std::mem::take(&mut self.state.facilities),
                            self.facilities.len() + 1,
                        )));
                    return Ok(true);
                }
                LivingSortedStorageJobV2::commit_one(
                    &mut self.state.facilities,
                    &mut self.pending_facility,
                )
                .map_err(storage_error)?;
                Ok(true)
            }
            Stage::ColonyOutput => {
                if self.pending_colony.is_none() {
                    if let Some((_, row)) = self.colonies.pop_first() {
                        self.pending_colony = Some(row);
                        return Ok(true);
                    }
                    self.transition(Stage::FleetOutput);
                    return Ok(false);
                }
                super::ensure_retained_sorted(
                    &self.state.colonies,
                    self.pending_colony.as_ref().ok_or(Error::NotReady)?,
                )?;
                if self.state.colonies.as_slice().len() == self.state.colonies.capacity() {
                    self.preparation =
                        Some(Preparation::Colonies(LivingSortedStorageJobV2::begin(
                            std::mem::take(&mut self.state.colonies),
                            self.colonies.len() + 1,
                        )));
                    return Ok(true);
                }
                LivingSortedStorageJobV2::commit_one(
                    &mut self.state.colonies,
                    &mut self.pending_colony,
                )
                .map_err(storage_error)?;
                Ok(true)
            }
            Stage::FleetOutput => {
                if let Some((_, rows)) = self.fleets.pop_first() {
                    self.fleet = Some(rows.fleet);
                    self.hulls = rows.hulls;
                    self.transition(Stage::HullOutput);
                    Ok(true)
                } else {
                    self.repair = Some(LivingRepairJobV2::with_exclusions(
                        (*self.catalog).clone(),
                        std::mem::take(&mut self.state),
                        self.tick,
                        std::mem::take(&mut self.repair_exclusions),
                    ));
                    self.transition(Stage::Repair);
                    Ok(false)
                }
            }
            Stage::HullOutput => {
                if self.pending_hull.is_none()
                    && let Some((_, row)) = self.hulls.pop_first()
                {
                    self.pending_hull = Some(row);
                    return Ok(true);
                }
                if let Some(row) = self.pending_hull.as_ref() {
                    let fleet = self.fleet.as_mut().ok_or(Error::NotReady)?;
                    super::ensure_retained_sorted(&fleet.hulls, row)?;
                    if fleet.hulls.as_slice().len() == fleet.hulls.capacity() {
                        self.preparation =
                            Some(Preparation::Hulls(LivingSortedStorageJobV2::begin(
                                std::mem::take(&mut fleet.hulls),
                                self.hulls.len() + 1,
                            )));
                        return Ok(true);
                    }
                    LivingSortedStorageJobV2::commit_one(&mut fleet.hulls, &mut self.pending_hull)
                        .map_err(storage_error)?;
                    return Ok(true);
                }
                super::ensure_retained_sorted(
                    &self.state.fleets,
                    self.fleet.as_ref().ok_or(Error::NotReady)?,
                )?;
                if self.state.fleets.as_slice().len() == self.state.fleets.capacity() {
                    self.preparation = Some(Preparation::Fleets(LivingSortedStorageJobV2::begin(
                        std::mem::take(&mut self.state.fleets),
                        self.fleets.len() + 1,
                    )));
                    return Ok(true);
                }
                LivingSortedStorageJobV2::commit_one(&mut self.state.fleets, &mut self.fleet)
                    .map_err(storage_error)?;
                self.transition(Stage::FleetOutput);
                Ok(true)
            }
            Stage::RecipeStart => self.begin_recipe(),
            Stage::RecipeCommit => self.commit_recipe(),
            Stage::RetireRecipe => {
                if self.index == 0 {
                    self.retirement
                        .start(std::mem::take(&mut self.retired_reasons));
                    self.index = 1;
                    return Ok(true);
                }
                if self.retire_plan_record() {
                    return Ok(true);
                }
                self.transition(self.recipe_resume);
                Ok(false)
            }
            Stage::RetireIndexes => {
                // Use the exhaustive actual field registration for normal cleanup
                // too. Preserve only the output state/events and the active lane.
                while matches!(self.index, 0 | 2 | 3) {
                    self.index += 1;
                }
                if let Some(node) =
                    self.retirement_child(self.index, LivingStateRetirementPolicyV2::ReleaseHandle)
                {
                    self.index += 1;
                    self.retirement.start_node(node);
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
        if let Some(replaced) = self.facilities.insert(row.id, row) {
            self.retirement.start(replaced);
        }
    }
}
fn storage_error(error: LivingStorageJobErrorV2) -> Error {
    match error {
        LivingStorageJobErrorV2::CapacityOverflow => Error::PreparationCapacity,
        LivingStorageJobErrorV2::KeyedOrder => Error::Fault(Fault::Wire(
            super::super::wire::LivingWireErrorV2::KeyedOrder,
        )),
        LivingStorageJobErrorV2::Budget
        | LivingStorageJobErrorV2::Cancelled
        | LivingStorageJobErrorV2::NotReady => Error::Fault(Fault::Invariant),
    }
}
fn append_error(error: LivingPendingEventAppendErrorV2) -> Error {
    if let Some(facts) = error.facts_error() {
        Error::Fault(Fault::EventFacts(facts))
    } else if let LivingPendingEventAppendErrorV2::Storage(error) = error {
        storage_error(error)
    } else {
        Error::Fault(Fault::Invariant)
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
                LivingGalaxyStateV2::default(),
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
            // Independent typed-owner sum: 41 temporary roots + catalog3 +
            // PlanMap511 + depositMap45 + sorted3 + facts9 =612. Each of41
            // slots adds setup1 and original Box1: 612+82=694.
            assert_eq!(job.total_visits(), 694);
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
        let Self {
            retirement,
            preparation,
            append,
            after_append: _,
            pending_facility,
            pending_colony,
            pending_resource,
            delivered: _,
            catalog,
            state,
            events,
            tick: _,
            seed: _,
            branch: _,
            stage: _,
            index: _,
            catalog_index: _,
            plans,
            hull_hit_points,
            plan,
            worlds,
            minimum_deposits,
            first_deposits,
            colony_source,
            colonies,
            facility_source,
            facilities,
            recipe_order,
            fleet_source,
            fleets,
            fleet,
            hull_source,
            hulls,
            construction_source,
            construction_ordinal: _,
            pending_construction,
            pending_hull_job,
            pending_hull,
            completion_fleet: _,
            completion_event,
            job_source,
            hull_ordinal: _,
            hub_order,
            hub: _,
            hub_clock: _,
            recipe,
            target: _,
            active_plan,
            retired_plan,
            retired_reasons,
            recipe_output,
            recipe_resume: _,
            commit_stage: _,
            commit_index: _,
            commit_reasons,
            committed_reasons,
            pending_reason,
            facts,
            resource_facts,
            repair,
            repair_exclusions,
            cancelled: _,
            fault: _,
            last_poll_visits: _,
            total_visits: _,
            preparation_capacity: _,
        } = self;
        match index {
            0 => Some(retirement_node(std::mem::take(retirement), policy)),
            1 => Some(retirement_node(catalog.0.take(), policy)),
            2 => Some(retirement_node(std::mem::take(state), policy)),
            3 => Some(retirement_node(std::mem::take(events), policy)),
            4 => Some(retirement_node(std::mem::take(plans), policy)),
            5 => Some(retirement_node(std::mem::take(hull_hit_points), policy)),
            6 => Some(retirement_node(std::mem::take(plan), policy)),
            7 => Some(retirement_node(std::mem::take(worlds), policy)),
            8 => Some(retirement_node(std::mem::take(minimum_deposits), policy)),
            9 => Some(retirement_node(std::mem::take(first_deposits), policy)),
            10 => Some(retirement_node(std::mem::take(colony_source), policy)),
            11 => Some(retirement_node(std::mem::take(colonies), policy)),
            12 => Some(retirement_node(std::mem::take(facility_source), policy)),
            13 => Some(retirement_node(std::mem::take(facilities), policy)),
            14 => Some(retirement_node(std::mem::take(recipe_order), policy)),
            15 => Some(retirement_node(std::mem::take(fleet_source), policy)),
            16 => Some(retirement_node(std::mem::take(fleets), policy)),
            17 => Some(retirement_node(std::mem::take(fleet), policy)),
            18 => Some(retirement_node(std::mem::take(hull_source), policy)),
            19 => Some(retirement_node(std::mem::take(hulls), policy)),
            20 => Some(retirement_node(std::mem::take(construction_source), policy)),
            21 => Some(retirement_node(
                std::mem::take(pending_construction),
                policy,
            )),
            22 => Some(retirement_node(std::mem::take(pending_hull_job), policy)),
            23 => Some(retirement_node(std::mem::take(pending_hull), policy)),
            24 => Some(retirement_node(std::mem::take(completion_event), policy)),
            25 => Some(retirement_node(std::mem::take(job_source), policy)),
            26 => Some(retirement_node(std::mem::take(hub_order), policy)),
            27 => Some(retirement_node(std::mem::take(recipe), policy)),
            28 => Some(retirement_node(std::mem::take(active_plan), policy)),
            29 => Some(retirement_node(std::mem::take(retired_plan), policy)),
            30 => Some(retirement_node(std::mem::take(retired_reasons), policy)),
            31 => Some(retirement_node(std::mem::take(recipe_output), policy)),
            32 => Some(retirement_node(std::mem::take(commit_reasons), policy)),
            33 => Some(retirement_node(std::mem::take(committed_reasons), policy)),
            34 => Some(retirement_node(std::mem::take(pending_reason), policy)),
            35 => Some(retirement_node(std::mem::take(facts), policy)),
            36 => Some(retirement_node(std::mem::take(resource_facts), policy)),
            37 => Some(retirement_node(std::mem::take(repair), policy)),
            38 => Some(retirement_node(std::mem::take(repair_exclusions), policy)),
            39 => Some(retirement_node(preparation.take(), policy)),
            40 => Some(retirement_node(append.take(), policy)),
            41 => Some(retirement_node(pending_facility.take(), policy)),
            42 => Some(retirement_node(pending_colony.take(), policy)),
            43 => Some(retirement_node(pending_resource.take(), policy)),
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
                    j.fault = Some(ProductionFailure::Arithmetic)
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

#[cfg(test)]
mod bounded_production_recipe_tests {
    use super::*;
    const SCHEDULES: [&[usize]; 4] = [&[1], &[64], &[1024], &[1, 64, 7, 1024]];
    #[test]
    fn bounded_production_recipe_fixed_diagnostics_keep_exact_public_nesting() {
        use super::super::super::event_facts::LivingEventFactsErrorV2 as Facts;
        use super::super::super::receipt::LivingReceiptErrorV2 as Receipt;
        use super::super::super::wire::LivingWireErrorV2 as Wire;
        for fault in [
            Fault::Invariant,
            Fault::Arithmetic,
            Fault::Wire(Wire::KeyedOrder),
            Fault::EventFacts(Facts::Facts),
            Fault::EventFacts(Facts::Receipt(Receipt::TooManyEvents { limit: 65536 })),
            Fault::EventFacts(Facts::Wire(Wire::Json)),
            Fault::EventFacts(Facts::Capacity),
        ] {
            assert_eq!(ProductionFailure::capture(&fault).public(), fault);
        }
        assert_eq!(
            ProductionFailure::capture(&Fault::ViewingHistory),
            ProductionFailure::ProtocolInvariant
        );
        assert!(std::mem::size_of::<ProductionFailure>() <= 16);
    }
    #[test]
    fn bounded_production_recipe_capacity_retains_real_inline_child_and_failed_visit_once() {
        let authority = crate::living::builtin_living_sandbox_v2().unwrap();
        let mut totals = Vec::new();
        for retirement_schedule in SCHEDULES {
            // Each actual first-failure forward position is separate from cleanup.
            for (forward_schedule, position) in [
                (&[1][..], 0),
                (&[64][..], 0),
                (&[1024][..], 0),
                (&[1, 64, 7, 1024][..], 0),
                (&[1, 64, 7, 1024][..], 1),
                (&[1, 64, 7, 1024][..], 2),
                (&[1, 64, 7, 1024][..], 3),
            ] {
                let trigger_budget = forward_schedule[position];
                let mut job = Box::new(LivingProductionJobV2::begin_owned(
                    authority.catalog_snapshot(),
                    authority.state().clone(),
                    LivingPendingEventsV2::new(),
                    Tick(10),
                    [0; 32],
                    authority.branch(),
                ));
                let parent = std::ptr::from_ref(job.as_ref());
                let mut guard = 0;
                while !job.inject_recipe_capacity_failure() {
                    assert_eq!(job.poll(1).unwrap(), Progress::Pending);
                    guard += 1;
                    assert!(guard < 10000);
                }
                let child = job.recipe_identity();
                let inputs = Arc::as_ptr(&job.recipe.as_ref().unwrap().inputs);
                let outputs = Arc::as_ptr(&job.recipe.as_ref().unwrap().outputs);
                let before = job.total_visits();
                let child_before = job.recipe.as_ref().unwrap().total_visits();
                assert_eq!(job.poll(trigger_budget), Err(Error::PreparationCapacity));
                assert!(job.last_poll_visits() <= trigger_budget);
                assert_eq!(job.recipe.as_ref().unwrap().last_poll_visits(), 1);
                assert_eq!(
                    job.recipe.as_ref().unwrap().total_visits(),
                    child_before + 1
                );
                assert_eq!(job.last_poll_visits(), 1);
                assert_eq!(job.total_visits(), before + 1);
                assert!(job.recipe_preparation_failed());
                assert_eq!(job.recipe_identity(), child);
                assert_eq!(std::ptr::from_ref(job.as_ref()), parent);
                assert_eq!(Arc::as_ptr(&job.recipe.as_ref().unwrap().inputs), inputs);
                assert_eq!(Arc::as_ptr(&job.recipe.as_ref().unwrap().outputs), outputs);
                assert!(job.recipe.as_ref().unwrap().reason_storage.is_some());
                for bad in [0, 1025] {
                    assert_eq!(job.poll(bad), Err(Error::Budget));
                    assert_eq!(job.last_poll_visits(), 0);
                }
                assert!(matches!(
                    job.take_ready_result(),
                    Err(Error::PreparationCapacity)
                ));
                job.cancel();
                for b in [0, 1, 64, 1025] {
                    assert_eq!(job.poll(b), Err(Error::PreparationCapacity));
                    assert_eq!(job.last_poll_visits(), 0);
                }
                assert!(matches!(job.take_ready_result(), Err(Error::Cancelled)));
                assert_eq!(job.recipe_identity(), child);
                let mut retired = LivingPhaseRetirementJobV2::begin(
                    job,
                    LivingStateRetirementPolicyV2::ReleaseHandle,
                );
                let mut i = 0;
                while retired
                    .poll(retirement_schedule[i % retirement_schedule.len()])
                    .unwrap()
                    != Progress::Ready
                {
                    i += 1;
                    assert!(i < 10000);
                    assert!(retired.take_deferred_owner().is_none());
                }
                totals.push(retired.total_visits());
            }
        }
        assert!(totals.iter().all(|n| *n == totals[0]));
    }
    #[test]
    fn bounded_production_recipe_rejected_ready_extraction_latches_and_retains_child() {
        let authority = crate::living::builtin_living_sandbox_v2().unwrap();
        let mut job = LivingProductionJobV2::begin(
            authority.catalog(),
            LivingGalaxyStateV2::default(),
            LivingPendingEventsV2::new(),
            Tick(1),
            [0; 32],
            authority.branch(),
        );
        job.stage = Stage::RecipeStart;
        let mut recipe = LivingRecipeJobV2::begin(Default::default(), 0, 0, vec![], vec![]);
        while recipe.poll(1).unwrap() != Progress::Ready {}
        let _output = recipe.take_ready_result().unwrap();
        job.recipe = Some(recipe);
        let pointer = job.recipe_identity();
        assert_eq!(job.poll(1), Err(Error::Fault(Fault::Invariant)));
        assert_eq!(job.last_poll_visits(), 1);
        assert_eq!(job.total_visits(), 1);
        assert_eq!(job.fault, Some(ProductionFailure::ProtocolInvariant));
        assert!(!job.preparation_capacity);
        assert!(job.recipe_output.is_none());
        assert!(job.stage == Stage::RecipeStart);
        assert_eq!(job.recipe_identity(), pointer);
        assert_eq!(job.poll(64), Err(Error::Fault(Fault::Invariant)));
        assert_eq!(job.last_poll_visits(), 0);
        job.cancel();
        assert!(matches!(job.take_ready_result(), Err(Error::Cancelled)));
        let mut retired = job.into_phase_retirement(LivingStateRetirementPolicyV2::ReleaseHandle);
        while retired.poll(64).unwrap() != Progress::Ready {}
    }
    #[test]
    fn bounded_production_recipe_every_parent_handoff_and_shell_cut_has_exact_retirement() {
        let authority = crate::living::builtin_living_sandbox_v2().unwrap();
        // Borrowed catalog, empty state:20 moves+20 storage+root=41. Empty
        // collector16, lane7, catalog Option4, sorted4, facts10,38 other roots1;
        // 44 edges88+parent1 gives209. No whole-production forward qualification.
        for cut in 0..=59 {
            for schedule in SCHEDULES {
                let inputs = Arc::new(vec![
                    (LivingResourceV2::Energy, 2),
                    (LivingResourceV2::Ore, 3),
                ]);
                let outputs = Arc::new(vec![(LivingResourceV2::Alloy, 1)]);
                let mut owner = LivingProductionJobV2::begin(
                    authority.catalog(),
                    LivingGalaxyStateV2::default(),
                    LivingPendingEventsV2::new(),
                    Tick(1),
                    [0; 32],
                    authority.branch(),
                );
                owner.stage = Stage::RecipeStart;
                owner.recipe = Some(LivingRecipeJobV2::begin_shared(
                    LivingInventoryV2 {
                        energy: 0,
                        ore: 0,
                        alloy: super::super::LIVING_RESOURCE_STORAGE_LIMIT_V2,
                    },
                    0,
                    1,
                    inputs.clone(),
                    outputs.clone(),
                ));
                let pointer = owner.recipe_identity();
                for _ in 0..cut {
                    owner.poll(1).unwrap();
                    assert_eq!(owner.last_poll_visits(), 1);
                }
                assert_eq!(owner.total_visits(), cut);
                let expected = if let Some(child) = &owner.recipe {
                    assert_eq!(owner.recipe_identity(), pointer);
                    // Seven child edges/root15 + IO8 + pending1/map1/sorted4/
                    // storage1/lane7=37; fixed map/output rows3; selected Option+3.
                    let r = 37
                        + 3 * (child.reasons.len() + child.reason_output.as_slice().len()) as u64
                        + if child.pending_reason.is_some() { 3 } else { 0 }
                        - u64::from(cut == 19);
                    209 + 2 + r
                } else {
                    assert!(owner.recipe_output.is_some());
                    assert!(owner.stage == Stage::RecipeCommit);
                    // Delivered K4 output Option adds21; raw recipe shell37.
                    // Handoff1 +37 child work + original node shell1 =>39.
                    let q = cut - 21;
                    if q < 38 { 209 + 21 + 36 - q } else { 209 + 21 }
                };
                owner.cancel();
                assert!(matches!(owner.take_ready_result(), Err(Error::Cancelled)));
                let mut retired =
                    owner.into_phase_retirement(LivingStateRetirementPolicyV2::ReleaseHandle);
                let mut i = 0;
                while retired.poll(schedule[i % schedule.len()]).unwrap() != Progress::Ready {
                    i += 1;
                    assert!(i < 10000);
                }
                assert_eq!(retired.total_visits(), expected, "cut {cut}");
                assert_eq!(Arc::strong_count(&inputs), 1);
                assert_eq!(Arc::strong_count(&outputs), 1);
            }
        }
    }
}

#[cfg(test)]
mod corrected_repair_parent_tests {
    use super::super::{RepairCursor, RepairFailure};
    use super::*;
    const SCHEDULES: &[&[usize]] = &[&[1], &[64], &[1024], &[1, 64, 7, 1024]];
    fn tiny() -> LivingGalaxyStateV2 {
        let a = crate::living::builtin_living_sandbox_v2().unwrap();
        let mut s = LivingGalaxyStateV2::default();
        let mut w = a.state().worlds.as_slice()[0].clone();
        w.inventory.energy = 1;
        let mut f = a.state().fleets.as_slice()[0].clone();
        f.order = None;
        f.location = LivingFleetLocationV2::Docked { world: w.id };
        f.owner = w.owner.unwrap();
        let mut h = f.hulls.as_slice()[0].clone();
        h.definition = super::super::super::LivingSlugV2::new("escort").unwrap();
        h.hit_points = 5;
        h.next_repair = Some(Tick(0));
        f.hulls = LivingSortedVecV2::new(vec![h]).unwrap();
        s.worlds = LivingSortedVecV2::new(vec![w]).unwrap();
        s.fleets = LivingSortedVecV2::new(vec![f]).unwrap();
        s
    }
    #[test]
    fn corrected_repair_parent_every_cut_analytical_retirement_and_original_box() {
        let a = crate::living::builtin_living_sandbox_v2().unwrap();
        let d = a.catalog().hulls().len();
        let i = a.catalog().industries().len();
        let b = a
            .catalog()
            .industries()
            .iter()
            .filter(|d| matches!(d.effect, LivingIndustryEffectV2::Defense { .. }))
            .count();
        let forward = (1 + d + i + 1 + 1 + 3 + 1 + 1 + d + b + 15) as u64;
        let before_cleanup = forward - 15;
        for cut in 0..=forward + 76 {
            for schedule in SCHEDULES {
                let mut parent = Box::new(LivingProductionJobV2::begin(
                    a.catalog(),
                    LivingGalaxyStateV2::default(),
                    LivingPendingEventsV2::new(),
                    Tick(1),
                    [0; 32],
                    a.branch(),
                ));
                parent.stage = Stage::Repair;
                parent.repair = Some(LivingRepairJobV2::begin(a.catalog(), tiny(), Tick(1)));
                let parent_ptr = std::ptr::from_ref(&*parent);
                let child_ptr = parent.repair.as_ref().map(std::ptr::from_ref);
                for _ in 0..cut {
                    parent.poll(1).unwrap();
                    assert_eq!(parent.last_poll_visits(), 1);
                }
                let expected = if let Some(child) = &parent.repair {
                    assert_eq!(parent.repair.as_ref().map(std::ptr::from_ref), child_ptr);
                    let n = child.world_index.len()
                        + child.hit_points.len()
                        + child.battery_hit_points.len()
                        + child.targets.len()
                        + child.exclusions.len();
                    let middle = cut > before_cleanup && (cut - before_cleanup) % 3 == 2;
                    209 + 2 + 80 + 3 * n as u64 - u64::from(middle)
                } else {
                    let q = cut - forward - 1;
                    if q < 75 { 209 + 6 + 73 - q } else { 215 }
                };
                parent.cancel();
                assert!(matches!(parent.take_ready_result(), Err(Error::Cancelled)));
                assert_eq!(std::ptr::from_ref(&*parent), parent_ptr);
                let mut r = LivingPhaseRetirementJobV2::begin(
                    parent,
                    LivingStateRetirementPolicyV2::ReleaseHandle,
                );
                let mut n = 0;
                loop {
                    let budget = schedule[n % schedule.len()];
                    n += 1;
                    let p = r.poll(budget).unwrap();
                    assert!(r.last_poll_visits() <= budget);
                    if p == Progress::Ready {
                        break;
                    }
                    assert!(n < 100000);
                }
                assert_eq!(r.total_visits(), expected, "cut {cut}");
            }
        }
    }
    #[test]
    fn corrected_repair_parent_actual_first_overflow_retains_inline_child() {
        let a = crate::living::builtin_living_sandbox_v2().unwrap();
        for trigger in [1, 64, 1024, 1, 64, 7, 1024] {
            for schedule in SCHEDULES {
                let mut state = tiny();
                state
                    .fleets
                    .record_at_mut(0)
                    .unwrap()
                    .hulls
                    .record_at_mut(0)
                    .unwrap()
                    .next_repair = Some(Tick(u64::MAX));
                let mut p = Box::new(LivingProductionJobV2::begin(
                    a.catalog(),
                    LivingGalaxyStateV2::default(),
                    LivingPendingEventsV2::new(),
                    Tick(u64::MAX),
                    [0; 32],
                    a.branch(),
                ));
                p.stage = Stage::Repair;
                p.repair = Some(LivingRepairJobV2::begin(a.catalog(), state, Tick(u64::MAX)));
                let parent = std::ptr::from_ref(&*p);
                let child = p.repair.as_ref().map(std::ptr::from_ref);
                while p.repair.as_ref().unwrap().cursor != RepairCursor::Update {
                    p.poll(1).unwrap();
                }
                let before = p.total_visits();
                assert_eq!(p.poll(trigger), Err(Error::Fault(Fault::Arithmetic)));
                assert_eq!(p.last_poll_visits(), 1);
                assert_eq!(p.total_visits() - before, 1);
                assert_eq!(p.repair.as_ref().unwrap().last_poll_visits(), 1);
                assert!(matches!(
                    p.repair.as_ref().unwrap().fault,
                    Some(RepairFailure::Arithmetic)
                ));
                assert_eq!(p.repair.as_ref().map(std::ptr::from_ref), child);
                assert_eq!(std::ptr::from_ref(&*p), parent);
                assert!(p.stage == Stage::Repair);
                p.cancel();
                assert_eq!(p.poll(1), Err(Error::Fault(Fault::Arithmetic)));
                assert_eq!(p.last_poll_visits(), 0);
                assert!(matches!(p.take_ready_result(), Err(Error::Cancelled)));
                let mut r = LivingPhaseRetirementJobV2::begin(
                    p,
                    LivingStateRetirementPolicyV2::ReleaseHandle,
                );
                let mut n = 0;
                while r.poll(schedule[n % schedule.len()]).unwrap() != Progress::Ready {
                    n += 1;
                    assert!(n < 100000);
                }
            }
        }
    }
}

#[cfg(test)]
mod general_production_tests {
    use super::super::super::LivingSlugV2;
    use super::*;
    const SCHEDULES: &[&[usize]] = &[&[1], &[64], &[1024], &[1, 64, 7, 1024]];
    fn id(n: u8) -> Id {
        Id([n; 16])
    }
    fn sorted<T: super::super::super::wire::LivingKeyedV2>(rows: Vec<T>) -> LivingSortedVecV2<T> {
        LivingSortedVecV2::new(rows).unwrap()
    }
    fn empty_source<T>() -> IntoIter<T> {
        Vec::with_capacity(7).into_iter()
    }
    fn fixture<'a>(
        catalog: &'a ValidatedLivingCatalogPackV2,
        branch: LivingBranchIdV2,
        world: LivingWorldV2,
    ) -> LivingProductionJobV2<'a> {
        let owner = world.owner.unwrap();
        let world_id = world.id;
        let mut job = LivingProductionJobV2::begin(
            catalog,
            LivingGalaxyStateV2::default(),
            LivingPendingEventsV2::new(),
            Tick(1),
            [0; 32],
            branch,
        );
        job.stage = Stage::ConstructionKeep;
        job.state.worlds = sorted(vec![world]);
        let construction = |n| LivingConstructionJobV2 {
            id: id(n),
            world: world_id,
            owner,
            definition: LivingSlugV2::new("solar_array").unwrap(),
            accepted_tick: Tick(0),
            completion_tick: Tick(2),
            paid_alloy: 20,
        };
        job.state.construction_jobs = sorted(vec![construction(20)]);
        job.pending_construction = Some(construction(21));
        job.construction_source = empty_source();
        job.colony_source = empty_source();
        job.facility_source = empty_source();
        job.fleet_source = empty_source();
        job.hull_source = empty_source();
        job.job_source = vec![LivingHullJobV2 {
            id: id(22),
            world: world_id,
            owner,
            definition: LivingSlugV2::new("scout").unwrap(),
            accepted_tick: Tick(0),
            completion_tick: Tick(2),
            paid_alloy: 40,
            target_fleet: None,
        }]
        .into_iter();
        job.facilities.insert(
            id(30),
            LivingFacilityV2 {
                id: id(30),
                world: world_id,
                definition: LivingSlugV2::new("shipyard").unwrap(),
                status: LivingFacilityStatusV2::Operational,
                paid_alloy: 20,
                hit_points: 0,
                next_due: None,
                next_repair: None,
                blocked_reasons: sorted(vec![
                    LivingBlockedReasonV2::Disabled,
                    LivingBlockedReasonV2::OutputFull {
                        resource: LivingResourceV2::Energy,
                    },
                ]),
                disabled: false,
            },
        );
        job.colonies.insert(
            world_id,
            LivingColonyV2 {
                world: world_id,
                owner,
                hub: LivingHubV2 {
                    energy_next_due: Tick(2),
                    ore_next_due: Tick(2),
                    fallback_next_due: Tick(2),
                    blocked_reasons: sorted(vec![LivingBlockedReasonV2::Disabled]),
                },
            },
        );
        job.fleets.insert(
            id(40),
            FleetRows {
                fleet: LivingFleetV2 {
                    id: id(40),
                    owner,
                    location: LivingFleetLocationV2::Docked { world: world_id },
                    order: Some(LivingFleetOrderV2 {
                        kind: LivingFleetOrderKindV2::Settle,
                        target_world: Some(world_id),
                        not_before_tick: Tick(2),
                        lane_path: vec![id(91), id(92)],
                    }),
                    hulls: LivingSortedVecV2::default(),
                },
                hulls: [41, 42]
                    .into_iter()
                    .map(|n| {
                        (
                            id(n),
                            LivingHullV2 {
                                id: id(n),
                                definition: LivingSlugV2::new("scout").unwrap(),
                                hit_points: 0,
                                return_credit: false,
                                next_repair: None,
                            },
                        )
                    })
                    .collect(),
            },
        );
        job.plans.insert(
            "solar_array".into(),
            Plan {
                period: Some(10),
                deposit: 0,
                hp: 0,
                inputs: Arc::new(vec![
                    (LivingResourceV2::Energy, 2),
                    (LivingResourceV2::Ore, 3),
                ])
                .into(),
                outputs: Arc::new(vec![(LivingResourceV2::Alloy, 1)]).into(),
            },
        );
        job.worlds.insert(world_id, (0, Some(owner)));
        job.first_deposits.insert(world_id, 0);
        job.minimum_deposits.insert(
            world_id,
            [(1, id(70), 0), (2, id(71), 1)].into_iter().collect(),
        );
        job.retired_reasons = vec![
            LivingBlockedReasonV2::Disabled,
            LivingBlockedReasonV2::OutputFull {
                resource: LivingResourceV2::Energy,
            },
        ]
        .into_iter();
        job.facts.resources = vec![LivingEventResourceFactV2 {
            role: LivingEventResourceRoleV2::Input,
            resource: LivingResourceV2::Energy,
            quantity: 2,
        }];
        job.facts.blocked_reasons = vec![LivingBlockedReasonV2::Disabled];
        job.facts.affected = vec![id(80), id(81)];
        job.events
            .record_explained(
                LivingEventProvenanceV2::Autonomous {
                    phase: LivingPhaseV2::RunProduction,
                },
                LivingEventKindV2::ConstructionCompleted,
                LivingEventFactsV2 {
                    actor: Some(owner),
                    entity: Some(id(19)),
                    ..Default::default()
                },
            )
            .unwrap();
        job
    }
    fn drain<'a, T: RetirementFields<'a>>(owner: T, schedule: &[usize]) -> u64 {
        let mut cleanup =
            LivingPhaseRetirementJobV2::begin(owner, LivingStateRetirementPolicyV2::ReleaseHandle);
        for i in 0..100_000 {
            let b = schedule[i % schedule.len()];
            let before = cleanup.total_visits();
            let progress = cleanup.poll(b).unwrap();
            assert_eq!(
                cleanup.total_visits() - before,
                cleanup.last_poll_visits() as u64
            );
            assert!(cleanup.last_poll_visits() <= b);
            if progress == Progress::Ready {
                return cleanup.total_visits();
            }
        }
        panic!("production cleanup exceeded fixture bound");
    }
    #[test]
    fn general_production_populated_every_cut_has_independent_owner_work() {
        let authority = crate::living::builtin_living_sandbox_v2().unwrap();
        // Literal constants from independently pinned Python owner algebra, not Rust counters.
        let expected: &[u64] = &[
            405, 437, 437, 440, 440, 437, 436, 437, 426, 425, 424, 423, 422, 421, 420, 419, 418,
            417, 416, 415, 414, 413, 412, 411, 410, 409, 408, 407, 406, 405, 404, 405, 403, 403,
            402, 403, 403, 433, 433, 436, 433, 432, 433, 424, 423, 422, 421, 420, 419, 418, 417,
            416, 415, 414, 413, 412, 411, 410, 409, 408, 407, 406, 405, 404, 403, 402, 403, 401,
            401, 400, 401, 389, 419, 419, 422, 419, 418, 419, 410, 409, 408, 407, 406, 405, 404,
            403, 402, 401, 400, 399, 398, 397, 396, 395, 394, 393, 392, 391, 390, 389, 388, 389,
            388, 376, 406, 406, 409, 406, 405, 406, 397, 396, 395, 394, 393, 392, 391, 390, 389,
            388, 387, 386, 385, 384, 383, 382, 381, 380, 379, 378, 377, 376, 375, 376, 375, 351,
            351, 381, 381, 384, 381, 380, 381, 372, 371, 370, 369, 368, 367, 366, 365, 364, 363,
            362, 361, 360, 359, 358, 357, 356, 355, 354, 353, 352, 351, 350, 351, 349, 349, 347,
            377, 377, 380, 377, 376, 377, 368, 367, 366, 365, 364, 363, 362, 361, 360, 359, 358,
            357, 356, 355, 354, 353, 352, 351, 350, 349, 348, 347, 346, 347, 346, 425, 428, 431,
            434, 437, 437, 437, 437, 437, 437, 437, 437, 437, 434, 431, 428, 425, 422, 422, 421,
            422, 422, 421, 422, 422, 421, 422, 422, 421, 422, 422, 421, 422, 419, 418, 417, 416,
            415, 414, 413, 412, 411, 410, 409, 408, 407, 406, 405, 404, 403, 402, 401, 400, 399,
            398, 397, 396, 395, 394, 393, 392, 391, 390, 389, 388, 387, 386, 385, 384, 383, 382,
            381, 380, 379, 378, 377, 376, 375, 374, 373, 372, 371, 370, 369, 368, 367, 366, 365,
            364, 363, 362, 361, 360, 359, 358, 357, 356, 355, 354, 353, 352, 351, 350, 349, 348,
            347, 346, 345, 346, 346, 345, 344, 343, 342, 343, 343, 342, 341, 340, 339, 338, 337,
            336, 335, 334, 333, 332, 331, 330, 329, 328, 327, 326, 325, 324, 323, 322, 321, 320,
            319, 318, 317, 316, 315, 314, 313, 312, 311, 310, 309, 308, 307, 306, 305, 304, 303,
            302, 303, 303, 302, 303, 303, 302, 303, 303, 302, 301, 300, 299, 300, 300, 299, 298,
            297, 296, 295, 294, 293, 292, 291, 290, 289, 288, 287, 286, 285, 284, 283, 282, 281,
            280, 279, 278, 279, 279, 278, 277, 276, 275, 276, 276, 275, 276, 276, 275, 276, 276,
            275, 276, 276, 275, 276, 276, 275, 276, 276, 275, 276, 276, 275, 276, 276, 275, 276,
            276, 275, 276, 276, 275, 276, 276, 275, 276, 276, 275, 276, 276, 275, 276, 276, 275,
            276, 276, 275, 276, 276, 275, 276, 276, 275, 276, 276, 275, 276, 276, 275, 276, 276,
            275, 276, 276, 275, 274, 273, 272, 271, 270, 269, 270, 270, 269, 270, 270, 269, 270,
            273, 272, 271, 270, 269, 270, 270, 269, 270, 279, 278, 277, 276, 275, 274, 273, 272,
            271, 270, 269, 268, 267, 266, 265, 264, 263, 262, 261, 260, 259, 258, 257, 258, 258,
            257, 258, 258, 257, 258, 258, 257, 258, 258, 257, 258, 258, 257, 258, 258, 257, 258,
            258, 257, 258, 258, 257, 258,
        ];
        for (cut, &retirement) in expected.iter().enumerate() {
            for schedule in SCHEDULES {
                let mut job = Box::new(fixture(
                    authority.catalog(),
                    authority.branch(),
                    authority.state().worlds.as_slice()[0].clone(),
                ));
                let original = std::ptr::from_ref(job.as_ref());
                let reason_pointer = job.facilities[&id(30)].blocked_reasons.as_slice().as_ptr();
                let reason_capacity = job.facilities[&id(30)].blocked_reasons.capacity();
                let plan = Arc::downgrade(&job.plans["solar_array"].inputs);
                for _ in 0..cut {
                    job.poll(1).unwrap();
                    assert_eq!(job.last_poll_visits(), 1);
                }
                assert_eq!(job.total_visits(), cut as u64);
                assert_eq!(std::ptr::from_ref(job.as_ref()), original);
                // Inspect original allocation in the actual surviving partitions only.
                let row = job
                    .facilities
                    .get(&id(30))
                    .or(job.pending_facility.as_ref())
                    .or_else(|| {
                        job.state
                            .facilities
                            .as_slice()
                            .iter()
                            .find(|row| row.id == id(30))
                    });
                if let Some(row) = row {
                    assert_eq!(row.blocked_reasons.as_slice().as_ptr(), reason_pointer);
                    assert_eq!(row.blocked_reasons.capacity(), reason_capacity);
                }
                for b in [0, 1025] {
                    assert_eq!(job.poll(b), Err(Error::Budget));
                    assert_eq!(job.last_poll_visits(), 0);
                }
                if cut < expected.len() - 1 {
                    assert!(matches!(job.take_ready_result(), Err(Error::NotReady)));
                }
                job.cancel();
                assert_eq!(job.poll(1), Err(Error::Cancelled));
                assert_eq!(job.last_poll_visits(), 0);
                assert!(matches!(job.take_ready_result(), Err(Error::Cancelled)));
                assert_eq!(drain(job, schedule), retirement, "cut {cut}");
                assert!(plan.upgrade().is_none());
            }
        }
    }
    #[test]
    fn general_production_populated_ready_has_only_output_and_once_only_extraction() {
        let authority = crate::living::builtin_living_sandbox_v2().unwrap();
        for schedule in SCHEDULES {
            let mut job = fixture(
                authority.catalog(),
                authority.branch(),
                authority.state().worlds.as_slice()[0].clone(),
            );
            for i in 0..100_000 {
                let budget = schedule[i % schedule.len()];
                let before = job.total_visits();
                let p = job.poll(budget).unwrap();
                assert_eq!(job.total_visits() - before, job.last_poll_visits() as u64);
                assert!(job.last_poll_visits() <= budget);
                if p == Progress::Ready {
                    break;
                }
            }
            assert_eq!(job.total_visits(), 532);
            assert!(job.catalog.0.is_none() && job.retirement.is_empty());
            assert!(
                job.preparation.is_none()
                    && job.append.is_none()
                    && job.recipe.is_none()
                    && job.repair.is_none()
            );
            assert_eq!(job.poll(1024), Ok(Progress::Ready));
            assert_eq!(job.last_poll_visits(), 0);
            let (state, events) = job.take_ready_result().unwrap();
            assert_eq!(state.construction_jobs.as_slice().len(), 2);
            assert_eq!(state.hull_jobs.as_slice().len(), 1);
            assert_eq!(state.fleets.as_slice()[0].hulls.as_slice().len(), 2);
            assert_eq!(events.len(), 1);
            assert!(matches!(job.take_ready_result(), Err(Error::NotReady)));
            assert_eq!(drain(job, schedule), 206);
        }
    }
    #[test]
    fn general_production_actual_storage_failure_retains_prefix_pending_and_parent() {
        let authority = crate::living::builtin_living_sandbox_v2().unwrap();
        for trigger in [1, 64, 7, 1024] {
            for cleanup in SCHEDULES {
                let mut job = Box::new(fixture(
                    authority.catalog(),
                    authority.branch(),
                    authority.state().worlds.as_slice()[0].clone(),
                ));
                let parent = std::ptr::from_ref(job.as_ref());
                let original = job.state.construction_jobs.as_slice().as_ptr();
                let capacity = job.state.construction_jobs.capacity();
                let pending = job
                    .pending_construction
                    .as_ref()
                    .unwrap()
                    .definition
                    .as_str()
                    .as_ptr();
                assert_eq!(job.poll(1), Ok(Progress::Pending));
                let Some(Preparation::Construction(storage)) = &mut job.preparation else {
                    panic!("retained construction storage")
                };
                storage.additional = usize::MAX;
                let before = job.total_visits();
                assert_eq!(job.poll(trigger), Err(Error::PreparationCapacity));
                assert_eq!(job.total_visits() - before, 1);
                assert_eq!(job.last_poll_visits(), 1);
                let Some(Preparation::Construction(storage)) = &job.preparation else {
                    panic!("original storage")
                };
                assert_eq!(
                    storage.original.as_ref().unwrap().as_slice().as_ptr(),
                    original
                );
                assert_eq!(storage.original.as_ref().unwrap().capacity(), capacity);
                assert_eq!(
                    job.pending_construction
                        .as_ref()
                        .unwrap()
                        .definition
                        .as_str()
                        .as_ptr(),
                    pending
                );
                assert_eq!(std::ptr::from_ref(job.as_ref()), parent);
                for bad in [0, 1025] {
                    assert_eq!(job.poll(bad), Err(Error::Budget));
                    assert_eq!(job.last_poll_visits(), 0);
                }
                assert!(matches!(
                    job.take_ready_result(),
                    Err(Error::PreparationCapacity)
                ));
                job.cancel();
                for b in [0, 1, 1025] {
                    assert_eq!(job.poll(b), Err(Error::PreparationCapacity));
                    assert_eq!(job.last_poll_visits(), 0);
                }
                assert!(matches!(job.take_ready_result(), Err(Error::Cancelled)));
                drain(job, cleanup);
            }
        }
    }
    #[test]
    fn general_production_selected_construction_and_hull_survive_real_refusals() {
        let authority = crate::living::builtin_living_sandbox_v2().unwrap();
        for trigger in [1, 64, 7, 1024] {
            let mut job = fixture(
                authority.catalog(),
                authority.branch(),
                authority.state().worlds.as_slice()[0].clone(),
            );
            job.stage = Stage::ConstructionCreate;
            job.tick = Tick(u64::MAX);
            let original = job
                .pending_construction
                .as_ref()
                .unwrap()
                .definition
                .as_str()
                .as_ptr();
            assert_eq!(job.poll(trigger), Err(Error::Fault(Fault::Arithmetic)));
            assert_eq!(job.last_poll_visits(), 1);
            assert_eq!(
                job.pending_construction
                    .as_ref()
                    .unwrap()
                    .definition
                    .as_str()
                    .as_ptr(),
                original
            );
            assert!(job.completion_event.is_none() && job.append.is_none());
            drain(job, &[64]);
            let mut job = fixture(
                authority.catalog(),
                authority.branch(),
                authority.state().worlds.as_slice()[0].clone(),
            );
            job.stage = Stage::HullInsert;
            job.completion_fleet = Some(id(99));
            job.pending_hull = Some(
                job.fleets
                    .get_mut(&id(40))
                    .unwrap()
                    .hulls
                    .remove(&id(41))
                    .unwrap(),
            );
            let original = job
                .pending_hull
                .as_ref()
                .unwrap()
                .definition
                .as_str()
                .as_ptr();
            assert_eq!(job.poll(trigger), Err(Error::Fault(Fault::Invariant)));
            assert_eq!(job.last_poll_visits(), 1);
            assert_eq!(
                job.pending_hull
                    .as_ref()
                    .unwrap()
                    .definition
                    .as_str()
                    .as_ptr(),
                original
            );
            assert_eq!(job.completion_fleet, Some(id(99)));
            assert!(job.append.is_none());
            drain(job, &[64]);
            let mut job = fixture(
                authority.catalog(),
                authority.branch(),
                authority.state().worlds.as_slice()[0].clone(),
            );
            job.pending_construction.as_mut().unwrap().id = id(20);
            let original = job
                .pending_construction
                .as_ref()
                .unwrap()
                .definition
                .as_str()
                .as_ptr();
            assert_eq!(
                job.poll(trigger),
                Err(Error::Fault(Fault::Wire(
                    super::super::super::wire::LivingWireErrorV2::KeyedOrder
                )))
            );
            assert_eq!(job.last_poll_visits(), 1);
            assert_eq!(
                job.pending_construction
                    .as_ref()
                    .unwrap()
                    .definition
                    .as_str()
                    .as_ptr(),
                original
            );
            assert!(job.preparation.is_none());
            drain(job, &[64]);
        }
    }
    #[test]
    fn general_production_append_failure_preserves_original_payload_and_stage() {
        let authority = crate::living::builtin_living_sandbox_v2().unwrap();
        for trigger in [1, 64, 7, 1024] {
            let mut job = fixture(
                authority.catalog(),
                authority.branch(),
                authority.state().worlds.as_slice()[0].clone(),
            );
            job.stage = Stage::ConstructionEvent;
            let facts = LivingEventFactsV2 {
                resources: vec![LivingEventResourceFactV2 {
                    role: LivingEventResourceRoleV2::Input,
                    resource: LivingResourceV2::Energy,
                    quantity: 10001,
                }],
                ..Default::default()
            };
            let pointer = facts.resources.as_ptr();
            let capacity = facts.resources.capacity();
            let prefix = job.events.payloads().as_ptr();
            job.completion_event = Some((LivingEventKindV2::ConstructionCompleted, facts));
            job.poll(1).unwrap();
            let before = job.total_visits();
            for _ in 0..100 {
                match job.poll(trigger) {
                    Ok(Progress::Pending) => {}
                    Err(Error::Fault(Fault::EventFacts(
                        super::super::super::event_facts::LivingEventFactsErrorV2::Facts,
                    ))) => break,
                    other => panic!("unexpected append {other:?}"),
                }
            }
            assert_eq!(job.total_visits() - before, 6); // Mode + four fixed validations + bad quantity.
            assert!(job.stage == Stage::ConstructionEvent);
            assert!(job.after_append == Some(Stage::Construction));
            let append = job.append.as_ref().unwrap();
            let (events, _) = append.retained_for_test();
            assert_eq!(events.unwrap().payloads().as_ptr(), prefix);
            let (facts, has_storage) = append.retained_explained_for_test();
            assert_eq!(facts.unwrap().resources.as_ptr(), pointer);
            assert_eq!(facts.unwrap().resources.capacity(), capacity);
            assert!(!has_storage);
            assert!(matches!(
                job.take_ready_result(),
                Err(Error::Fault(Fault::EventFacts(_)))
            ));
            drain(job, &[1, 64, 7, 1024]);
        }
    }
    #[test]
    fn general_production_prepared_extraction_checks_destination_before_moving_output() {
        let authority = crate::living::builtin_living_sandbox_v2().unwrap();
        for trigger in [1, 64, 7, 1024] {
            let mut job = fixture(
                authority.catalog(),
                authority.branch(),
                authority.state().worlds.as_slice()[0].clone(),
            );
            let mut storage = LivingVecStorageJobV2::begin(vec![(LivingResourceV2::Energy, 2)], 0);
            assert_eq!(storage.poll(1).unwrap(), Progress::Ready);
            let pointer = storage.original.as_ref().unwrap().as_ptr();
            job.preparation = Some(Preparation::PlanInputs(storage));
            assert!(job.plan.is_none());
            let before = job.total_visits();
            assert_eq!(job.poll(trigger), Err(Error::Fault(Fault::Invariant)));
            assert_eq!(job.total_visits() - before, 1);
            let Some(Preparation::PlanInputs(storage)) = &job.preparation else {
                panic!("retained storage")
            };
            assert!(!storage.delivered);
            assert_eq!(storage.original.as_ref().unwrap().as_ptr(), pointer);
            assert_eq!(storage.original.as_ref().unwrap().len(), 1);
            drain(job, &[1, 64, 7, 1024]);
        }
    }

    // Additional pre-review parent proof. Constants come from the pinned
    // standalone owner algebra, never from a production probe.
    fn proof_empty(catalog: &ValidatedLivingCatalogPackV2) -> Box<LivingProductionJobV2<'_>> {
        let mut job = Box::new(LivingProductionJobV2::begin(
            catalog,
            Default::default(),
            LivingPendingEventsV2::new(),
            Tick(1),
            [0; 32],
            LivingBranchIdV2([0; 16]),
        ));
        job.colony_source = empty_source();
        job.facility_source = empty_source();
        job.fleet_source = empty_source();
        job.hull_source = empty_source();
        job.construction_source = empty_source();
        job.job_source = empty_source();
        job
    }
    fn proof_catalog() -> ValidatedLivingCatalogPackV2 {
        use super::super::super::catalog::{
            LivingResourceAmountV2, decode_living_catalog_pack_v2, living_core_pack_v2,
        };
        let pack = living_core_pack_v2().unwrap();
        let old = serde_json::to_string(&pack.industry("foundry").unwrap().effect).unwrap();
        let amounts = || {
            sorted(
                vec![("alloy", 2), ("energy", 4), ("ore", 4)]
                    .into_iter()
                    .map(|(r, q)| LivingResourceAmountV2 {
                        resource: LivingSlugV2::new(r).unwrap(),
                        quantity: q,
                    })
                    .collect(),
            )
        };
        let new = serde_json::to_string(&LivingIndustryEffectV2::Recipe {
            period_ticks: 20,
            deposit_units: 0,
            inputs: amounts(),
            outputs: amounts(),
        })
        .unwrap();
        let bytes = std::str::from_utf8(pack.canonical_bytes()).unwrap();
        assert_eq!(bytes.matches(&old).count(), 1);
        decode_living_catalog_pack_v2(bytes.replacen(&old, &new, 1).as_bytes()).unwrap()
    }
    fn proof_catalog_job(
        catalog: &ValidatedLivingCatalogPackV2,
        mode: usize,
    ) -> Box<LivingProductionJobV2<'_>> {
        let mut job = proof_empty(catalog);
        job.catalog_index = 2;
        if mode > 0 {
            job.stage = if mode == 1 {
                Stage::Inputs
            } else {
                Stage::Outputs
            };
            job.index = 1;
            job.plan = Some(Plan {
                period: Some(20),
                deposit: 0,
                hp: 0,
                inputs: Arc::new(if mode == 1 {
                    vec![(LivingResourceV2::Alloy, 2)]
                } else {
                    vec![
                        (LivingResourceV2::Alloy, 2),
                        (LivingResourceV2::Energy, 4),
                        (LivingResourceV2::Ore, 4),
                    ]
                })
                .into(),
                outputs: Arc::new(if mode == 1 {
                    Vec::new()
                } else {
                    vec![(LivingResourceV2::Alloy, 2)]
                })
                .into(),
            });
        }
        job
    }
    // Counts observer metadata copies and assertion comparisons, never producer visits.
    macro_rules! proof_eq {
        ($stats:expr, $left:expr, $right:expr $(,)?) => {{
            $stats.comparisons += 1;
            assert_eq!($left, $right);
        }};
    }
    macro_rules! proof_check {
        ($stats:expr, $value:expr $(,)?) => {{
            $stats.comparisons += 1;
            assert!($value);
        }};
    }
    #[derive(Default)]
    struct ProofObservations {
        rows: usize,
        comparisons: usize,
        snapshots: usize,
    }
    fn proof_lane(job: &LivingProductionJobV2<'_>) -> Option<*const ()> {
        job.retirement
            .node
            .as_ref()
            .map(|node| std::ptr::from_ref(&**node).cast())
    }
    fn proof_drain_parent(job: Box<LivingProductionJobV2<'_>>, schedule: &[usize]) -> u64 {
        let original = std::ptr::from_ref(job.as_ref());
        let mut retirement =
            LivingPhaseRetirementJobV2::begin(job, LivingStateRetirementPolicyV2::ReleaseHandle);
        for call in 0..100_000 {
            if let Some(owner) = &retirement.root.owner {
                assert_eq!(std::ptr::from_ref(owner.as_ref()), original);
            }
            let budget = schedule[call % schedule.len()];
            let before = retirement.total_visits();
            let progress = retirement.poll(budget).unwrap();
            assert_eq!(
                retirement.total_visits() - before,
                retirement.last_poll_visits() as u64
            );
            assert!(retirement.last_poll_visits() <= budget);
            assert!(retirement.take_deferred_owner().is_none());
            if let Some(owner) = &retirement.root.owner {
                assert_eq!(std::ptr::from_ref(owner.as_ref()), original);
            }
            if progress == Progress::Ready {
                assert!(retirement.root.owner.is_none());
                let visits = retirement.total_visits();
                assert_eq!(retirement.poll(0), Err(Error::Budget));
                assert_eq!(retirement.last_poll_visits(), 0);
                assert_eq!(retirement.poll(1024).unwrap(), Progress::Ready);
                assert_eq!(retirement.last_poll_visits(), 0);
                assert_eq!(retirement.total_visits(), visits);
                return visits;
            }
        }
        panic!("parent retirement exceeded independent fixture bound");
    }
    fn proof_controls(job: &mut LivingProductionJobV2<'_>) {
        let before = job.total_visits();
        for budget in [0, 1025] {
            assert_eq!(job.poll(budget), Err(Error::Budget));
            assert_eq!(job.last_poll_visits(), 0);
        }
        assert!(matches!(job.take_ready_result(), Err(Error::NotReady)));
        job.cancel();
        assert_eq!(job.poll(1), Err(Error::Cancelled));
        assert_eq!(job.last_poll_visits(), 0);
        assert!(matches!(job.take_ready_result(), Err(Error::Cancelled)));
        assert_eq!(job.total_visits(), before);
    }
    struct CatalogOwners {
        parent: *const (),
        catalog: *const ValidatedLivingCatalogPackV2,
        arcs: Option<[*const (); 2]>,
        original: [Option<(*const (LivingResourceV2, u32), usize)>; 2],
        destination: [Option<*const (LivingResourceV2, u32)>; 2],
        sources: [*const (); 4],
        lane: Option<(Stage, *const ())>,
        stats: ProofObservations,
    }
    impl CatalogOwners {
        fn new(job: &LivingProductionJobV2<'_>) -> Self {
            Self {
                parent: std::ptr::from_ref(job).cast(),
                catalog: std::ptr::from_ref::<ValidatedLivingCatalogPackV2>(&job.catalog),
                arcs: None,
                original: [None; 2],
                destination: [None; 2],
                sources: [
                    job.colony_source.as_slice().as_ptr().cast(),
                    job.facility_source.as_slice().as_ptr().cast(),
                    job.fleet_source.as_slice().as_ptr().cast(),
                    job.hull_source.as_slice().as_ptr().cast(),
                ],
                lane: None,
                stats: Default::default(),
            }
        }
        fn observe(&mut self, job: &LivingProductionJobV2<'_>) {
            let visits = job.total_visits();
            self.stats.snapshots += 1;
            proof_eq!(
                self.stats,
                std::ptr::from_ref::<ValidatedLivingCatalogPackV2>(&job.catalog),
                self.catalog
            );
            proof_eq!(
                self.stats,
                std::ptr::from_ref(job).cast::<()>(),
                self.parent
            );
            proof_eq!(self.stats, job.catalog_index, 2);
            proof_eq!(
                self.stats,
                [
                    job.colony_source.as_slice().as_ptr().cast(),
                    job.facility_source.as_slice().as_ptr().cast(),
                    job.fleet_source.as_slice().as_ptr().cast(),
                    job.hull_source.as_slice().as_ptr().cast()
                ],
                self.sources
            );
            if let Some(plan) = &job.plan {
                let arcs = [
                    Arc::as_ptr(&plan.inputs).cast(),
                    Arc::as_ptr(&plan.outputs).cast(),
                ];
                proof_eq!(self.stats, *self.arcs.get_or_insert(arcs), arcs);
                for (which, array) in [&plan.inputs, &plan.outputs].into_iter().enumerate() {
                    if self.original[which].is_none() && job.preparation.is_none() {
                        self.original[which] = Some((array.as_ptr(), array.len()));
                    }
                    self.stats.rows += 1;
                }
            }
            if let Some(preparation) = &job.preparation {
                let (which, s) = match preparation {
                    Preparation::PlanInputs(s) => (0, s),
                    Preparation::PlanOutputs(s) => (1, s),
                    _ => panic!("only Plan storage"),
                };
                proof_eq!(
                    self.stats,
                    job.index,
                    if self.original[which].is_some_and(|(_, n)| n == 1) {
                        1
                    } else {
                        0
                    }
                );
                if let Some(v) = &s.original {
                    if s.stage == super::super::StorageStage::Inspect
                        || s.stage == super::super::StorageStage::Allocate
                    {
                        let old = *self.original[which].get_or_insert((v.as_ptr(), v.len()));
                        proof_eq!(self.stats, v.as_ptr(), old.0);
                        proof_eq!(self.stats, v.len(), old.1);
                    } else if let Some(destination) = self.destination[which] {
                        proof_eq!(self.stats, v.as_ptr(), destination);
                    }
                    self.stats.rows += 1;
                }
                if let Some(v) = &s.destination {
                    proof_eq!(
                        self.stats,
                        *self.destination[which].get_or_insert(v.as_ptr()),
                        v.as_ptr()
                    );
                    self.stats.rows += 1;
                }
                if let Some(source) = &s.source {
                    let (pointer, n) = self.original[which].expect("captured original prefix");
                    proof_eq!(
                        self.stats,
                        source.as_slice().as_ptr(),
                        pointer.wrapping_add(n - source.len())
                    );
                    self.stats.rows += 1;
                }
                if s.stage == super::super::StorageStage::Retire && s.last_poll_visits() > 0 {
                    let (pointer, n) = self.original[which].expect("original source");
                    let retained = s
                        .retirement
                        .production_source_tail(pointer.wrapping_add(n).cast());
                    if s.total_visits() == (n + 3) as u64 {
                        proof_eq!(self.stats, retained, Some(0));
                    } else {
                        proof_eq!(self.stats, retained, None);
                    }
                }
                proof_check!(self.stats, s.pending_transfer.is_none());
            } else if let Some(plan) = &job.plan {
                for (i, v) in [&plan.inputs, &plan.outputs].into_iter().enumerate() {
                    if let Some(destination) = self.destination[i] {
                        proof_eq!(self.stats, v.as_ptr(), destination);
                    }
                }
            }
            let node = proof_lane(job).map(|pointer| (job.stage, pointer));
            if let (Some(old), Some(now)) = (self.lane, node)
                && old.0 == now.0
            {
                proof_eq!(self.stats, old.1, now.1);
            }
            self.lane = node;
            proof_eq!(self.stats, job.total_visits(), visits);
        }
    }
    #[test]
    fn production_parent_proof_catalog_every_preparation_extraction_and_reclamation_cut() {
        let catalog = proof_catalog();
        let LivingIndustryEffectV2::Recipe {
            inputs, outputs, ..
        } = &catalog.industries()[2].effect
        else {
            panic!("literal recipe")
        };
        let source = [inputs.as_slice().as_ptr(), outputs.as_slice().as_ptr()];
        for (mode, expected) in [CATALOG_PROOF, INPUTS_PROOF, OUTPUTS_PROOF]
            .into_iter()
            .enumerate()
        {
            for (cut, &residual) in expected.iter().enumerate() {
                for forward in SCHEDULES {
                    for cleanup in SCHEDULES {
                        let mut job = proof_catalog_job(&catalog, mode);
                        let mut owners = CatalogOwners::new(&job);
                        owners.observe(&job);
                        let mut call = 0;
                        while job.total_visits() < (cut as u64) {
                            // Capture the natural original Plan Arc before backing preparation.
                            let budget = if mode == 0 && job.total_visits() == 0 {
                                1
                            } else {
                                forward[call % forward.len()].min(cut - job.total_visits() as usize)
                            };
                            let before = job.total_visits();
                            assert_eq!(job.poll(budget).unwrap(), Progress::Pending);
                            assert_eq!(job.total_visits() - before, job.last_poll_visits() as u64);
                            assert!(job.last_poll_visits() <= budget);
                            owners.observe(&job);
                            call += 1;
                        }
                        assert_eq!(
                            [inputs.as_slice().as_ptr(), outputs.as_slice().as_ptr()],
                            source
                        );
                        let expected_index = match mode {
                            0 => {
                                if cut <= 31 {
                                    0
                                } else if cut <= 34 {
                                    cut - 31
                                } else {
                                    cut.saturating_sub(64)
                                }
                            }
                            1 => {
                                if cut <= 31 {
                                    1
                                } else if cut <= 33 {
                                    cut - 30
                                } else {
                                    cut.saturating_sub(63)
                                }
                            }
                            _ => {
                                if cut <= 31 {
                                    1
                                } else {
                                    cut - 30
                                }
                            }
                        };
                        assert_eq!(job.index, expected_index, "mode{mode} cut{cut}");
                        proof_controls(&mut job);
                        owners.observe(&job);
                        assert!(owners.stats.snapshots >= 2 && owners.stats.comparisons >= 6);
                        assert_eq!(
                            proof_drain_parent(job, cleanup),
                            residual,
                            "mode{mode} cut{cut}"
                        );
                    }
                }
            }
        }
    }
    fn proof_new_facts() -> LivingEventFactsV2 {
        LivingEventFactsV2 {
            actor: Some(id(72)),
            resources: vec![LivingEventResourceFactV2 {
                role: LivingEventResourceRoleV2::Input,
                resource: LivingResourceV2::Energy,
                quantity: 100,
            }],
            affected: vec![id(7)],
            blocked_reasons: Vec::with_capacity(9),
            ..Default::default()
        }
    }
    fn proof_append_job<'a>(
        catalog: &'a ValidatedLivingCatalogPackV2,
        world: LivingWorldV2,
        route: usize,
    ) -> Box<LivingProductionJobV2<'a>> {
        let mut job = proof_empty(catalog);
        job.events = super::super::super::receipt::production_append_collector_fixture();
        job.construction_source = vec![LivingConstructionJobV2 {
            id: id(21),
            world: world.id,
            owner: id(1),
            definition: LivingSlugV2::new("solar_array").unwrap(),
            accepted_tick: Tick(0),
            completion_tick: Tick(2),
            paid_alloy: 20,
        }]
        .into_iter();
        job.job_source = vec![LivingHullJobV2 {
            id: id(22),
            world: world.id,
            owner: id(1),
            definition: LivingSlugV2::new("scout").unwrap(),
            accepted_tick: Tick(0),
            completion_tick: Tick(2),
            paid_alloy: 40,
            target_fleet: None,
        }]
        .into_iter();
        job.stage = match route {
            0 => Stage::ConstructionEvent,
            1 => Stage::HullEvent,
            _ => Stage::RecipeCommit,
        };
        if route < 2 {
            job.completion_event = Some((
                if route == 0 {
                    LivingEventKindV2::ConstructionCompleted
                } else {
                    LivingEventKindV2::HullCompleted
                },
                proof_new_facts(),
            ));
        } else {
            job.worlds.insert(world.id, (0, world.owner));
            job.state.worlds = sorted(vec![world.clone()]);
            job.facilities.insert(
                id(30),
                LivingFacilityV2 {
                    id: id(30),
                    world: world.id,
                    definition: LivingSlugV2::new("foundry").unwrap(),
                    paid_alloy: 0,
                    hit_points: 0,
                    next_due: Some(Tick(10)),
                    next_repair: None,
                    blocked_reasons: Default::default(),
                    disabled: false,
                    status: LivingFacilityStatusV2::Operational,
                },
            );
            job.target = Some(Target::Facility(id(30)));
            job.commit_stage = 9;
            job.facts = proof_new_facts();
            job.active_plan = Some(Plan {
                period: Some(10),
                deposit: 0,
                hp: 0,
                inputs: Arc::new(Vec::with_capacity(7)).into(),
                outputs: Arc::new(Vec::with_capacity(7)).into(),
            });
        }
        job
    }
    fn proof_append_observation(
        job: &LivingProductionJobV2<'_>,
    ) -> super::super::super::receipt::ProductionAppendObservation {
        let mut observation = if let Some(append) = &job.append {
            append.production_owner_observation()
        } else {
            let mut o = super::super::super::receipt::ProductionAppendObservation::default();
            o.collector(&job.events);
            o
        };
        if let Some((_, facts)) = &job.completion_event {
            observation.facts(facts);
        }
        if job.facts.actor == Some(id(72)) {
            observation.facts(&job.facts);
        }
        observation
    }
    struct AppendOwners {
        parent: *const (),
        catalog: *const ValidatedLivingCatalogPackV2,
        facts: Vec<super::super::super::receipt::ProductionFactsObservation>,
        buffers: Vec<super::super::super::receipt::ProductionBufferObservation>,
        construction: *const LivingConstructionJobV2,
        hulls: *const LivingHullJobV2,
        slugs: [*const u8; 2],
        worlds: *const LivingWorldV2,
        facility: Option<*const u8>,
        empty: [*const (); 4],
        append: Option<*const LivingPendingEventAppendJobV2>,
        lane: Option<*const ()>,
        plan: Option<[*const (); 2]>,
        destinations: [Option<*const ()>; 2],
        stats: ProofObservations,
    }
    impl AppendOwners {
        fn new(job: &LivingProductionJobV2<'_>) -> Self {
            let snapshot = proof_append_observation(job);
            Self {
                parent: std::ptr::from_ref(job).cast(),
                catalog: std::ptr::from_ref::<ValidatedLivingCatalogPackV2>(&job.catalog),
                facts: snapshot.facts,
                buffers: snapshot.buffers,
                construction: job.construction_source.as_slice().as_ptr(),
                hulls: job.job_source.as_slice().as_ptr(),
                worlds: job.state.worlds.as_slice().as_ptr(),
                facility: job
                    .facilities
                    .get(&id(30))
                    .map(|f| f.definition.as_str().as_ptr()),
                slugs: [
                    job.construction_source.as_slice()[0]
                        .definition
                        .as_str()
                        .as_ptr(),
                    job.job_source.as_slice()[0].definition.as_str().as_ptr(),
                ],
                empty: [
                    job.colony_source.as_slice().as_ptr().cast(),
                    job.facility_source.as_slice().as_ptr().cast(),
                    job.fleet_source.as_slice().as_ptr().cast(),
                    job.hull_source.as_slice().as_ptr().cast(),
                ],
                append: None,
                lane: None,
                plan: job.active_plan.as_ref().map(|p| {
                    [
                        Arc::as_ptr(&p.inputs).cast(),
                        Arc::as_ptr(&p.outputs).cast(),
                    ]
                }),
                destinations: [None; 2],
                stats: Default::default(),
            }
        }
        fn observe(&mut self, job: &LivingProductionJobV2<'_>, route: usize) {
            let visits = job.total_visits();
            let cut = visits as usize;
            self.stats.snapshots += 1;
            proof_eq!(
                self.stats,
                job.state.worlds.as_slice().as_ptr(),
                self.worlds
            );
            proof_eq!(
                self.stats,
                job.facilities
                    .get(&id(30))
                    .map(|f| f.definition.as_str().as_ptr()),
                self.facility
            );
            proof_eq!(
                self.stats,
                std::ptr::from_ref::<ValidatedLivingCatalogPackV2>(&job.catalog),
                self.catalog
            );
            proof_eq!(
                self.stats,
                std::ptr::from_ref(job).cast::<()>(),
                self.parent
            );
            proof_eq!(
                self.stats,
                job.construction_source.as_slice().as_ptr(),
                self.construction
            );
            proof_eq!(self.stats, job.construction_source.len(), 1);
            proof_eq!(self.stats, job.job_source.as_slice().as_ptr(), self.hulls);
            proof_eq!(self.stats, job.job_source.len(), 1);
            proof_eq!(
                self.stats,
                [
                    job.colony_source.as_slice().as_ptr().cast(),
                    job.facility_source.as_slice().as_ptr().cast(),
                    job.fleet_source.as_slice().as_ptr().cast(),
                    job.hull_source.as_slice().as_ptr().cast()
                ],
                self.empty
            );
            if let Some(arcs) = self.plan {
                let p = job
                    .active_plan
                    .as_ref()
                    .or(job.retired_plan.as_ref())
                    .unwrap();
                proof_eq!(
                    self.stats,
                    [
                        Arc::as_ptr(&p.inputs).cast(),
                        Arc::as_ptr(&p.outputs).cast()
                    ],
                    arcs
                );
                proof_eq!(self.stats, p.inputs.capacity(), 7);
                proof_eq!(self.stats, p.outputs.capacity(), 7);
            }
            if cut < 154 {
                proof_check!(
                    self.stats,
                    job.stage
                        == match route {
                            0 => Stage::ConstructionEvent,
                            1 => Stage::HullEvent,
                            _ => Stage::RecipeCommit,
                        }
                );
            } else {
                proof_check!(
                    self.stats,
                    job.stage
                        == match route {
                            0 => Stage::Construction,
                            1 => Stage::HullJobs,
                            _ => Stage::RetireRecipe,
                        }
                );
            }
            if let Some(append) = &job.append {
                proof_eq!(
                    self.stats,
                    *self.append.get_or_insert(std::ptr::from_ref(append)),
                    std::ptr::from_ref(append)
                );
                proof_check!(self.stats, cut <= 75);
                proof_check!(self.stats, job.events.payloads().is_empty());
            }
            proof_eq!(self.stats, job.append.is_some(), (1..=75).contains(&cut));
            let node = proof_lane(job);
            proof_eq!(self.stats, node.is_some(), (76..=152).contains(&cut));
            if let (Some(old), Some(now)) = (self.lane, node) {
                proof_eq!(self.stats, old, now);
            }
            self.lane = node;
            proof_eq!(
                self.stats,
                [
                    job.construction_source.as_slice()[0]
                        .definition
                        .as_str()
                        .as_ptr(),
                    job.job_source.as_slice()[0].definition.as_str().as_ptr()
                ],
                self.slugs
            );
            let observed = proof_append_observation(job);
            self.stats.rows += observed.observed_rows;
            for original in &self.facts {
                let matches = observed
                    .facts
                    .iter()
                    .filter(|f| f.actor == original.actor)
                    .collect::<Vec<_>>();
                proof_eq!(self.stats, matches.len(), 1);
                proof_eq!(self.stats, matches[0], original);
            }
            for collection in 0..4 {
                let old = self
                    .buffers
                    .iter()
                    .find(|b| b.collection == collection)
                    .unwrap();
                let buffers = observed
                    .buffers
                    .iter()
                    .filter(|b| b.collection == collection)
                    .collect::<Vec<_>>();
                if collection >= 2 || cut <= if collection == 0 { 61 } else { 67 } {
                    proof_check!(
                        self.stats,
                        buffers
                            .iter()
                            .any(|b| b.pointer == old.pointer && b.capacity == old.capacity)
                    );
                }
                if collection < 2 {
                    let allocate = if collection == 0 { 62 } else { 68 };
                    let row_size = if collection == 0 {
                        std::mem::size_of::<super::super::super::receipt::LivingEventPayloadV2>()
                    } else {
                        std::mem::size_of::<(u16, LivingEventFactsV2)>()
                    };
                    let end = old.pointer.cast::<u8>().wrapping_add(row_size).cast();
                    if cut == allocate || cut == allocate + 1 {
                        let source = buffers.iter().find(|b| b.partition == 2).unwrap();
                        proof_eq!(
                            self.stats,
                            source.pointer,
                            if cut == allocate { old.pointer } else { end }
                        );
                        proof_eq!(self.stats, source.len, usize::from(cut == allocate));
                    }
                    if (allocate..=allocate + 3).contains(&cut) {
                        let append = job.append.as_ref().unwrap();
                        proof_eq!(
                            self.stats,
                            append.production_empty_source(collection == 1, end),
                            if cut == allocate + 2 { Some(0) } else { None }
                        );
                    }
                    if let Some(destination) = buffers.iter().find(|b| b.partition == 3) {
                        proof_eq!(
                            self.stats,
                            *self.destinations[collection as usize]
                                .get_or_insert(destination.pointer),
                            destination.pointer
                        );
                    }
                    if cut >= allocate + 2
                        && let Some(pointer) = self.destinations[collection as usize]
                    {
                        proof_check!(self.stats, buffers.iter().any(|b| b.pointer == pointer));
                    }
                }
            }
            proof_eq!(self.stats, job.total_visits(), visits);
        }
    }
    #[test]
    fn production_parent_proof_all_live_append_routes_every_cut() {
        let authority = crate::living::builtin_living_sandbox_v2().unwrap();
        for route in 0..3 {
            let expected = if route == 2 {
                RECIPE_APPEND_PROOF
            } else {
                APPEND_PROOF
            };
            for (cut, &residual) in expected.iter().enumerate() {
                for forward in SCHEDULES {
                    for cleanup in SCHEDULES {
                        let mut job = proof_append_job(
                            authority.catalog(),
                            authority.state().worlds.as_slice()[0].clone(),
                            route,
                        );
                        let mut owners = AppendOwners::new(&job);
                        owners.observe(&job, route);
                        let mut call = 0;
                        while job.total_visits() < (cut as u64) {
                            let budget = forward[call % forward.len()]
                                .min(cut - job.total_visits() as usize);
                            let before = job.total_visits();
                            assert_eq!(job.poll(budget).unwrap(), Progress::Pending);
                            assert_eq!(job.total_visits() - before, job.last_poll_visits() as u64);
                            assert!(job.last_poll_visits() <= budget);
                            owners.observe(&job, route);
                            call += 1;
                        }
                        proof_controls(&mut job);
                        owners.observe(&job, route);
                        assert!(owners.stats.rows > 0 && owners.stats.comparisons > 0);
                        assert_eq!(
                            proof_drain_parent(job, cleanup),
                            residual,
                            "route{route} cut{cut}"
                        );
                    }
                }
            }
        }
    }
    #[test]
    fn production_parent_proof_append_ready_refusal_and_real_storage_overflow() {
        let authority = crate::living::builtin_living_sandbox_v2().unwrap();
        for route in 0..3 {
            for trigger in [1, 64, 7, 1024] {
                for cleanup in SCHEDULES {
                    for extraction in [false, true] {
                        let mut job = proof_append_job(
                            authority.catalog(),
                            authority.state().worlds.as_slice()[0].clone(),
                            route,
                        );
                        let mut owners = AppendOwners::new(&job);
                        owners.observe(&job, route);
                        let cut = if extraction { 75 } else { 60 };
                        for _ in 0..cut {
                            job.poll(1).unwrap();
                            owners.observe(&job, route);
                        }
                        let pointer = std::ptr::from_ref(job.append.as_ref().unwrap());
                        let original = proof_append_observation(&job);
                        let before = job.total_visits();
                        let output = if extraction {
                            Some(job.append.as_mut().unwrap().take_ready_result().unwrap().0)
                        } else {
                            assert!(job.append.as_mut().unwrap().inject_entry_capacity_failure());
                            None
                        };
                        let expected = if extraction {
                            Error::Fault(Fault::Invariant)
                        } else {
                            Error::PreparationCapacity
                        };
                        assert_eq!(job.poll(trigger), Err(expected.clone()));
                        assert_eq!(job.total_visits(), before + 1);
                        assert_eq!(job.last_poll_visits(), 1);
                        assert_eq!(std::ptr::from_ref(job.append.as_ref().unwrap()), pointer);
                        assert_eq!(std::ptr::from_ref(job.as_ref()).cast::<()>(), owners.parent);
                        assert!(job.events.payloads().is_empty());
                        assert!(job.retirement.is_empty());
                        assert!(job.after_append.is_some());
                        let mut retained =
                            job.append.as_ref().unwrap().production_owner_observation();
                        if let Some(output) = &output {
                            retained.collector(output);
                        }
                        for original in &original.buffers {
                            assert!(retained.buffers.iter().any(|b| b.collection
                                == original.collection
                                && b.pointer == original.pointer
                                && b.len == original.len
                                && b.capacity == original.capacity));
                        }
                        assert!(
                            job.stage
                                == match route {
                                    0 => Stage::ConstructionEvent,
                                    1 => Stage::HullEvent,
                                    _ => Stage::RecipeCommit,
                                }
                        );
                        for original in &original.facts {
                            assert_eq!(
                                retained
                                    .facts
                                    .iter()
                                    .filter(|f| f.actor == original.actor)
                                    .count(),
                                1
                            );
                            assert!(retained.facts.contains(original));
                        }
                        assert_eq!(
                            job.construction_source.as_slice().as_ptr(),
                            owners.construction
                        );
                        assert_eq!(job.job_source.as_slice().as_ptr(), owners.hulls);
                        assert_eq!(job.poll(0), Err(Error::Budget));
                        assert_eq!(job.last_poll_visits(), 0);
                        assert_eq!(job.poll(1), Err(expected.clone()));
                        assert_eq!(job.last_poll_visits(), 0);
                        assert!(matches!(
                            job.take_ready_result(),
                            Err(Error::Fault(Fault::Invariant) | Error::PreparationCapacity)
                        ));
                        job.cancel();
                        assert_eq!(job.poll(0), Err(expected));
                        assert_eq!(job.last_poll_visits(), 0);
                        assert!(matches!(job.take_ready_result(), Err(Error::Cancelled)));
                        assert_eq!(
                            proof_drain_parent(job, cleanup),
                            match (route == 2, extraction) {
                                (false, true) => 293,
                                (true, true) => 333,
                                (false, false) => 347,
                                (true, false) => 387,
                            }
                        );
                        if let Some(output) = output {
                            assert_eq!(drain(output, cleanup), 85);
                        }
                    }
                }
            }
        }
    }

    const CATALOG_PROOF: &[u64] = &[
        209, 228, 255, 255, 258, 255, 254, 255, 249, 248, 247, 246, 245, 244, 243, 242, 241, 240,
        239, 238, 237, 236, 235, 234, 233, 232, 231, 230, 229, 228, 227, 228, 231, 234, 237, 264,
        264, 267, 264, 263, 264, 258, 257, 256, 255, 254, 253, 252, 251, 250, 249, 248, 247, 246,
        245, 244, 243, 242, 241, 240, 239, 238, 237, 236, 237, 240, 243, 246,
    ];

    const INPUTS_PROOF: &[u64] = &[
        231, 258, 258, 261, 261, 258, 257, 258, 252, 251, 250, 249, 248, 247, 246, 245, 244, 243,
        242, 241, 240, 239, 238, 237, 236, 235, 234, 233, 232, 231, 230, 231, 234, 237, 264, 264,
        267, 264, 263, 264, 258, 257, 256, 255, 254, 253, 252, 251, 250, 249, 248, 247, 246, 245,
        244, 243, 242, 241, 240, 239, 238, 237, 236, 237, 240, 243, 246,
    ];

    const OUTPUTS_PROOF: &[u64] = &[
        240, 267, 267, 270, 270, 267, 266, 267, 261, 260, 259, 258, 257, 256, 255, 254, 253, 252,
        251, 250, 249, 248, 247, 246, 245, 244, 243, 242, 241, 240, 239, 240, 243, 246,
    ];

    const APPEND_PROOF: &[u64] = &[
        281, 317, 317, 317, 317, 317, 317, 317, 317, 320, 320, 320, 320, 320, 320, 320, 320, 320,
        320, 320, 320, 320, 320, 320, 320, 320, 320, 320, 320, 320, 320, 320, 320, 320, 320, 320,
        320, 320, 320, 320, 320, 320, 320, 320, 320, 320, 320, 320, 320, 320, 320, 320, 320, 320,
        320, 320, 320, 320, 320, 320, 320, 347, 350, 350, 347, 346, 344, 371, 374, 374, 371, 370,
        368, 368, 380, 380, 359, 358, 357, 356, 355, 354, 353, 352, 351, 350, 349, 348, 347, 346,
        345, 344, 343, 342, 341, 340, 339, 338, 337, 336, 335, 334, 333, 332, 331, 330, 329, 328,
        327, 326, 325, 324, 323, 322, 321, 320, 319, 318, 317, 316, 315, 314, 313, 312, 311, 310,
        309, 308, 307, 306, 305, 304, 303, 302, 301, 300, 299, 298, 297, 296, 295, 294, 293, 292,
        291, 290, 289, 288, 287, 286, 285, 284, 283, 284, 284,
    ];

    const RECIPE_APPEND_PROOF: &[u64] = &[
        297, 357, 357, 357, 357, 357, 357, 357, 357, 360, 360, 360, 360, 360, 360, 360, 360, 360,
        360, 360, 360, 360, 360, 360, 360, 360, 360, 360, 360, 360, 360, 360, 360, 360, 360, 360,
        360, 360, 360, 360, 360, 360, 360, 360, 360, 360, 360, 360, 360, 360, 360, 360, 360, 360,
        360, 360, 360, 360, 360, 360, 360, 387, 390, 390, 387, 386, 384, 411, 414, 414, 411, 410,
        408, 408, 420, 420, 399, 398, 397, 396, 395, 394, 393, 392, 391, 390, 389, 388, 387, 386,
        385, 384, 383, 382, 381, 380, 379, 378, 377, 376, 375, 374, 373, 372, 371, 370, 369, 368,
        367, 366, 365, 364, 363, 362, 361, 360, 359, 358, 357, 356, 355, 354, 353, 352, 351, 350,
        349, 348, 347, 346, 345, 344, 343, 342, 341, 340, 339, 338, 337, 336, 335, 334, 333, 332,
        331, 330, 329, 328, 327, 326, 325, 324, 323, 324, 324,
    ];
}
