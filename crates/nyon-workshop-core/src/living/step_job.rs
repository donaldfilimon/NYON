//! Resumable lifecycle and arrival phases on an unpublished owned candidate.
//!
//! These phase cursors form the scheduled part of the boundary executor. They count actual
//! entity/record visits, including collection reconstruction, rather than whole
//! ticks. Other boundary phases, forecasts, final validation and digest sealing
//! still execute synchronously; this type does not claim a bounded whole step.
//! Beginning a job moves its inputs without cloning or scanning nested state.

use std::collections::{BTreeMap, BTreeSet};
use std::vec::IntoIter;

#[path = "production_job.rs"]
mod production_job;
pub use production_job::LivingProductionJobV2;

#[derive(Clone)]
pub(crate) enum LivingPhaseCatalogV2<'a> {
    Borrowed(&'a super::catalog::ValidatedLivingCatalogPackV2),
    Owned(std::sync::Arc<super::catalog::ValidatedLivingCatalogPackV2>),
}
impl std::ops::Deref for LivingPhaseCatalogV2<'_> {
    type Target = super::catalog::ValidatedLivingCatalogPackV2;
    fn deref(&self) -> &Self::Target {
        match self {
            Self::Borrowed(catalog) => catalog,
            Self::Owned(catalog) => catalog.as_ref(),
        }
    }
}

use super::ids::{LivingEntityIdV2, LivingPhaseV2, LivingTickV2};
use super::model::{
    LivingAgreementKindV2, LivingAgreementV2, LivingGalaxyStateV2, LivingOccupationV2, LivingWarV2,
};
use super::receipt::{LivingEventKindV2, LivingEventProvenanceV2, LivingPendingEventsV2};
use super::simulation::LivingDeterministicFaultV2;
use super::wire::LivingKeyedV2;

/// Maximum logical entity/record visits admitted by one lifecycle poll.
pub const LIVING_MAX_PHASE_POLL_VISITS_V2: usize = 1_024;
const TRUCE_DURATION_TICKS: u64 = 600;
type AgreementKey = (LivingEntityIdV2, LivingEntityIdV2, &'static str);
type Pair = (LivingEntityIdV2, LivingEntityIdV2);

/// State of the unpublished lifecycle candidate.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum LivingLifecycleProgressV2 {
    /// More record visits remain.
    Pending,
    /// All lifecycle records and emitted events are available to the caller.
    Ready,
}

/// Lifecycle polling failures, separate from committed gameplay faults.
#[derive(Clone, Debug, Eq, PartialEq, thiserror::Error)]
pub enum LivingLifecycleJobErrorV2 {
    /// Invalid polling budgets are retryable and do not consume work.
    #[error("Living lifecycle poll requires 1..=1024 record visits")]
    Budget,
    /// A cancelled candidate can never be installed.
    #[error("Living lifecycle job was cancelled")]
    Cancelled,
    /// The caller requested the candidate before processing all its records.
    #[error("Living lifecycle job is not ready")]
    NotReady,
    /// Arithmetic, collection ordering, or receipt capacity failed.
    #[error("Living lifecycle phase failed: {0}")]
    Fault(#[from] LivingDeterministicFaultV2),
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Cursor {
    Agreements,
    Wars,
    AgreementOutput,
    WorldOwners,
    Occupations,
    HazardStarts,
    HazardEnds,
    RetireIndices,
    Ready,
}

/// An owned phase-2 candidate that cannot mutate a resident authority.
///
/// Each work unit visits one source or output record. Tree key comparisons and
/// scalar field checks are bounded metadata work; no nested entity collection
/// is cloned. Unchanged state collections are moved through the job verbatim.
/// The caller must validate and seal the complete boundary before publishing it.
#[derive(Debug)]
pub struct LivingLifecycleJobV2 {
    delivered: bool,
    state: LivingGalaxyStateV2,
    events: LivingPendingEventsV2,
    tick: LivingTickV2,
    agreements: IntoIter<LivingAgreementV2>,
    wars: IntoIter<LivingWarV2>,
    occupations: IntoIter<LivingOccupationV2>,
    agreement_rows: BTreeMap<AgreementKey, LivingAgreementV2>,
    agreement_output:
        Option<std::collections::btree_map::IntoIter<AgreementKey, LivingAgreementV2>>,
    ended_pairs: BTreeSet<Pair>,
    owners: BTreeMap<LivingEntityIdV2, Option<LivingEntityIdV2>>,
    cursor: Cursor,
    index: usize,
    cancelled: bool,
    fault: Option<LivingDeterministicFaultV2>,
    last_poll_visits: usize,
    total_visits: u64,
}

impl LivingLifecycleJobV2 {
    /// Move one unpublished candidate and its existing phase-1 events into a job.
    /// No records are visited until `poll` is called.
    pub fn begin(
        mut state: LivingGalaxyStateV2,
        events: LivingPendingEventsV2,
        tick: LivingTickV2,
    ) -> Self {
        let agreements = std::mem::take(&mut state.agreements).into_vec().into_iter();
        let wars = std::mem::take(&mut state.wars).into_vec().into_iter();
        let occupations = std::mem::take(&mut state.occupations)
            .into_vec()
            .into_iter();
        Self {
            delivered: false,
            state,
            events,
            tick,
            agreements,
            wars,
            occupations,
            agreement_rows: BTreeMap::new(),
            agreement_output: None,
            ended_pairs: BTreeSet::new(),
            owners: BTreeMap::new(),
            cursor: Cursor::Agreements,
            index: 0,
            cancelled: false,
            fault: None,
            last_poll_visits: 0,
            total_visits: 0,
        }
    }

    /// Invalidate the unpublished candidate without touching any authority.
    pub fn cancel(&mut self) {
        self.cancelled = true;
    }

    /// Actual source/output record visits made by the preceding poll.
    pub const fn last_poll_visits(&self) -> usize {
        self.last_poll_visits
    }

    /// Actual source/output record visits since beginning this job.
    pub const fn total_visits(&self) -> u64 {
        self.total_visits
    }

    /// Process at most `budget` record visits, retaining all intermediate cursors.
    pub fn poll(
        &mut self,
        budget: usize,
    ) -> Result<LivingLifecycleProgressV2, LivingLifecycleJobErrorV2> {
        self.last_poll_visits = 0;
        if self.cancelled {
            return Err(LivingLifecycleJobErrorV2::Cancelled);
        }
        if budget == 0 || budget > LIVING_MAX_PHASE_POLL_VISITS_V2 {
            return Err(LivingLifecycleJobErrorV2::Budget);
        }
        if let Some(fault) = &self.fault {
            return Err(LivingLifecycleJobErrorV2::Fault(fault.clone()));
        }
        while self.last_poll_visits < budget && self.cursor != Cursor::Ready {
            match self.visit_next() {
                Ok(true) => {
                    self.last_poll_visits += 1;
                    self.total_visits += 1;
                }
                Ok(false) => {}
                Err(fault) => {
                    // The failed record was visited too; retain the fault so a
                    // retry cannot accidentally finish a partially applied phase.
                    self.last_poll_visits += 1;
                    self.total_visits += 1;
                    self.fault = Some(fault.clone());
                    return Err(LivingLifecycleJobErrorV2::Fault(fault));
                }
            }
        }
        Ok(if self.cursor == Cursor::Ready {
            LivingLifecycleProgressV2::Ready
        } else {
            LivingLifecycleProgressV2::Pending
        })
    }

    /// Consume a ready candidate; cancelled or faulted candidates never escape.
    pub fn finish(
        mut self,
    ) -> Result<(LivingGalaxyStateV2, LivingPendingEventsV2), LivingLifecycleJobErrorV2> {
        self.take_ready_result()
    }
    /// Borrowed one-time extraction; rejected calls preserve every owner.
    pub fn take_ready_result(
        &mut self,
    ) -> Result<(LivingGalaxyStateV2, LivingPendingEventsV2), LivingLifecycleJobErrorV2> {
        if self.cancelled {
            return Err(LivingLifecycleJobErrorV2::Cancelled);
        }
        if let Some(fault) = self.fault.clone() {
            return Err(LivingLifecycleJobErrorV2::Fault(fault));
        }
        if self.cursor != Cursor::Ready {
            return Err(LivingLifecycleJobErrorV2::NotReady);
        }
        if self.delivered {
            return Err(LivingLifecycleJobErrorV2::NotReady);
        }
        self.delivered = true;
        Ok((
            std::mem::take(&mut self.state),
            std::mem::take(&mut self.events),
        ))
    }

    fn emit(&mut self, kind: LivingEventKindV2) -> Result<(), LivingDeterministicFaultV2> {
        self.events.record(
            LivingEventProvenanceV2::Autonomous {
                phase: LivingPhaseV2::ExpireLifecycles,
            },
            kind,
        )?;
        Ok(())
    }

    fn visit_next(&mut self) -> Result<bool, LivingDeterministicFaultV2> {
        match self.cursor {
            Cursor::Agreements => {
                if let Some(row) = self.agreements.next() {
                    if row.end_tick.0 <= self.tick.0 {
                        self.emit(LivingEventKindV2::AgreementExpired)?;
                    } else {
                        self.agreement_rows.insert(row.living_key(), row);
                    }
                    return Ok(true);
                }
                self.cursor = Cursor::Wars;
            }
            Cursor::Wars => {
                if let Some(row) = self.wars.next() {
                    if row.end_tick.0 <= self.tick.0 {
                        let end = row
                            .end_tick
                            .0
                            .checked_add(TRUCE_DURATION_TICKS)
                            .ok_or(LivingDeterministicFaultV2::Arithmetic)?;
                        let truce = LivingAgreementV2 {
                            kind: LivingAgreementKindV2::Truce,
                            participant_a: row.participant_a,
                            participant_b: row.participant_b,
                            start_tick: row.end_tick,
                            end_tick: LivingTickV2(end),
                        };
                        self.agreement_rows.insert(truce.living_key(), truce);
                        self.ended_pairs.insert(row.living_key());
                        self.emit(LivingEventKindV2::WarEnded)?;
                    } else {
                        self.state.wars.push_sorted(row)?;
                    }
                    return Ok(true);
                }
                self.agreement_output = Some(std::mem::take(&mut self.agreement_rows).into_iter());
                self.cursor = Cursor::AgreementOutput;
            }
            Cursor::AgreementOutput => {
                if let Some((_, row)) = self.agreement_output.as_mut().and_then(Iterator::next) {
                    self.state.agreements.push_sorted(row)?;
                    return Ok(true);
                }
                self.cursor = Cursor::WorldOwners;
            }
            Cursor::WorldOwners => {
                if self.ended_pairs.is_empty() {
                    self.cursor = Cursor::Occupations;
                } else if let Some(row) = self.state.worlds.as_slice().get(self.index) {
                    self.owners.insert(row.id, row.owner);
                    self.index += 1;
                    return Ok(true);
                } else {
                    self.index = 0;
                    self.cursor = Cursor::Occupations;
                }
            }
            Cursor::Occupations => {
                if let Some(row) = self.occupations.next() {
                    let removed =
                        self.owners
                            .get(&row.world)
                            .copied()
                            .flatten()
                            .is_some_and(|owner| {
                                let pair = if owner < row.claimant {
                                    (owner, row.claimant)
                                } else {
                                    (row.claimant, owner)
                                };
                                self.ended_pairs.contains(&pair)
                            });
                    if !removed {
                        self.state.occupations.push_sorted(row)?;
                    }
                    return Ok(true);
                }
                self.cursor = Cursor::HazardStarts;
            }
            Cursor::HazardStarts => {
                if let Some(row) = self.state.hazards.as_slice().get(self.index) {
                    let started = row.start_tick == self.tick;
                    self.index += 1;
                    if started {
                        self.emit(LivingEventKindV2::HazardStarted)?;
                    }
                    return Ok(true);
                }
                self.index = 0;
                self.cursor = Cursor::HazardEnds;
            }
            Cursor::HazardEnds => {
                if let Some(row) = self.state.hazards.as_slice().get(self.index) {
                    let ended = row.end_tick == self.tick;
                    self.index += 1;
                    if ended {
                        self.emit(LivingEventKindV2::HazardEnded)?;
                    }
                    return Ok(true);
                }
                self.cursor = Cursor::RetireIndices;
            }
            Cursor::RetireIndices => {
                if self.owners.pop_first().is_some() || self.ended_pairs.pop_first().is_some() {
                    return Ok(true);
                }
                self.cursor = Cursor::Ready;
            }
            Cursor::Ready => {}
        }
        Ok(false)
    }
}

// ---------------------------------------------------------------- arrivals

use super::catalog::LIVING_RESOURCE_STORAGE_LIMIT_V2;
use super::model::{
    LivingFleetLocationV2, LivingFleetOrderKindV2, LivingFleetV2, LivingRelationReasonsV2,
    LivingRelationV2, LivingResourceV2, LivingRouteKindV2, LivingShipmentDispositionV2,
    LivingShipmentV2,
};

type Edge = (LivingEntityIdV2, u64);
pub(crate) type Graph = BTreeMap<LivingEntityIdV2, Vec<Edge>>;

#[derive(Debug)]
pub(crate) struct CostSearch {
    destination: LivingEntityIdV2,
    distance: BTreeMap<LivingEntityIdV2, u64>,
    frontier: BTreeSet<(u64, LivingEntityIdV2)>,
    current: Option<(u64, LivingEntityIdV2, usize)>,
    pub(crate) result: Option<Option<u64>>,
}

impl CostSearch {
    pub(crate) fn begin(
        from: Option<LivingEntityIdV2>,
        to: Option<LivingEntityIdV2>,
        graph: &Graph,
    ) -> Self {
        let mut search = Self {
            destination: to.unwrap_or(LivingEntityIdV2([0; 16])),
            distance: BTreeMap::new(),
            frontier: BTreeSet::new(),
            current: None,
            result: None,
        };
        match (from, to) {
            (Some(from), Some(to)) if from == to => search.result = Some(Some(10)),
            (Some(from), Some(to)) if graph.contains_key(&from) && graph.contains_key(&to) => {
                search.distance.insert(from, 0);
                search.frontier.insert((0, from));
            }
            _ => search.result = Some(None),
        }
        search
    }

    // One cached system or lane record is visited, never an entire adjacency list.
    // Only total duration is authoritative for freight returns; equal-cost path
    // tie breaking cannot change the returned arrival boundary.
    pub(crate) fn visit(&mut self, graph: &Graph) -> bool {
        if self.result.is_some() {
            return false;
        }
        if let Some((cost, system, index)) = self.current {
            if let Some(&(next, leg)) = graph.get(&system).and_then(|edges| edges.get(index)) {
                self.current = Some((cost, system, index + 1));
                let candidate = cost.saturating_add(leg);
                let old = self.distance.get(&next).copied().unwrap_or(u64::MAX);
                if candidate < old {
                    self.frontier.remove(&(old, next));
                    self.distance.insert(next, candidate);
                    self.frontier.insert((candidate, next));
                }
                return true;
            }
            self.current = None;
            return false;
        }
        if let Some((cost, system)) = self.frontier.pop_first() {
            if system == self.destination {
                self.result = Some(Some(cost));
            } else {
                self.current = Some((cost, system, 0));
            }
            true
        } else {
            self.result = Some(None);
            false
        }
    }
    /// Release one completed search index record under a caller's budget.
    pub(crate) fn retire_record(&mut self) -> bool {
        self.distance.pop_first().is_some() || self.frontier.pop_first().is_some()
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum ArrivalCursor {
    Fleets,
    Worlds,
    Systems,
    Lanes,
    Wars,
    Relations,
    Shipments,
    RelationOutput,
    RetireIndices,
    Ready,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum CargoCursor {
    Travel,
    RetireSearch,
    Return,
    Credit,
    Relation,
    Output,
}

/// Resumable phase-3 fleet arrivals and freight resolution.
///
/// Unchanged nested hulls are moved, not cloned. Indexed world metadata, active
/// wars and relation records are built incrementally. Return route search visits
/// one cached system or lane per work unit. Inventory and relation changes stay
/// in the owned private candidate until a complete boundary can be published.
#[derive(Debug)]
pub struct LivingArrivalJobV2 {
    retirement: PhaseRetirementLane<'static>,
    active_fleet: Option<LivingFleetV2>,
    delivered: bool,
    state: LivingGalaxyStateV2,
    events: LivingPendingEventsV2,
    tick: LivingTickV2,
    fleets: IntoIter<LivingFleetV2>,
    shipments: IntoIter<LivingShipmentV2>,
    relations: IntoIter<LivingRelationV2>,
    world_meta: BTreeMap<LivingEntityIdV2, (usize, Option<LivingEntityIdV2>, LivingEntityIdV2)>,
    graph: Graph,
    active_wars: BTreeSet<Pair>,
    relation_rows: BTreeMap<Pair, LivingRelationV2>,
    relation_output: Option<std::collections::btree_map::IntoIter<Pair, LivingRelationV2>>,
    arrivals: Vec<LivingEntityIdV2>,
    cursor: ArrivalCursor,
    index: usize,
    cargo: Option<(LivingShipmentV2, CargoCursor)>,
    search: Option<CostSearch>,
    cancelled: bool,
    fault: Option<LivingDeterministicFaultV2>,
    last_poll_visits: usize,
    total_visits: u64,
}

impl LivingArrivalJobV2 {
    /// Move an unpublished candidate into phase 3 without scanning it.
    pub fn begin(
        mut state: LivingGalaxyStateV2,
        events: LivingPendingEventsV2,
        tick: LivingTickV2,
    ) -> Self {
        let fleets = std::mem::take(&mut state.fleets).into_vec().into_iter();
        let shipments = std::mem::take(&mut state.shipments).into_vec().into_iter();
        let relations = std::mem::take(&mut state.relations).into_vec().into_iter();
        Self {
            retirement: PhaseRetirementLane::default(),
            active_fleet: None,
            delivered: false,
            state,
            events,
            tick,
            fleets,
            shipments,
            relations,
            world_meta: BTreeMap::new(),
            graph: BTreeMap::new(),
            active_wars: BTreeSet::new(),
            relation_rows: BTreeMap::new(),
            relation_output: None,
            arrivals: Vec::new(),
            cursor: ArrivalCursor::Fleets,
            index: 0,
            cargo: None,
            search: None,
            cancelled: false,
            fault: None,
            last_poll_visits: 0,
            total_visits: 0,
        }
    }

    /// Cancel without publishing or scanning the candidate.
    pub fn cancel(&mut self) {
        self.cancelled = true;
    }
    /// Source/output entity and cached route-record visits made by the last poll.
    pub const fn last_poll_visits(&self) -> usize {
        self.last_poll_visits
    }
    /// Entity/record visits made since beginning this phase.
    pub const fn total_visits(&self) -> u64 {
        self.total_visits
    }

    /// Advance at most 1024 actual entity/record visits; invalid budgets are retryable.
    pub fn poll(
        &mut self,
        budget: usize,
    ) -> Result<LivingLifecycleProgressV2, LivingLifecycleJobErrorV2> {
        self.last_poll_visits = 0;
        if self.cancelled {
            return Err(LivingLifecycleJobErrorV2::Cancelled);
        }
        if budget == 0 || budget > LIVING_MAX_PHASE_POLL_VISITS_V2 {
            return Err(LivingLifecycleJobErrorV2::Budget);
        }
        if let Some(fault) = &self.fault {
            return Err(LivingLifecycleJobErrorV2::Fault(fault.clone()));
        }
        while self.last_poll_visits < budget && self.cursor != ArrivalCursor::Ready {
            if self.retirement.visit() {
                self.last_poll_visits += 1;
                self.total_visits += 1;
                continue;
            }

            match self.visit_next() {
                Ok(true) => {
                    self.last_poll_visits += 1;
                    self.total_visits += 1;
                }
                Ok(false) => {}
                Err(fault) => {
                    self.last_poll_visits += 1;
                    self.total_visits += 1;
                    self.fault = Some(fault.clone());
                    return Err(LivingLifecycleJobErrorV2::Fault(fault));
                }
            }
        }
        Ok(if self.cursor == ArrivalCursor::Ready {
            LivingLifecycleProgressV2::Ready
        } else {
            LivingLifecycleProgressV2::Pending
        })
    }

    /// Consume a ready phase and its exact list of arrived fleet identities.
    pub fn finish(
        mut self,
    ) -> Result<
        (
            LivingGalaxyStateV2,
            LivingPendingEventsV2,
            Vec<LivingEntityIdV2>,
        ),
        LivingLifecycleJobErrorV2,
    > {
        self.take_ready_result()
    }
    /// Borrowed one-time extraction; rejected calls preserve every owner.
    pub fn take_ready_result(
        &mut self,
    ) -> Result<
        (
            LivingGalaxyStateV2,
            LivingPendingEventsV2,
            Vec<LivingEntityIdV2>,
        ),
        LivingLifecycleJobErrorV2,
    > {
        if self.cancelled {
            return Err(LivingLifecycleJobErrorV2::Cancelled);
        }
        if let Some(fault) = self.fault.clone() {
            return Err(LivingLifecycleJobErrorV2::Fault(fault));
        }
        if self.cursor != ArrivalCursor::Ready {
            return Err(LivingLifecycleJobErrorV2::NotReady);
        }
        if self.delivered {
            return Err(LivingLifecycleJobErrorV2::NotReady);
        }
        self.delivered = true;
        Ok((
            std::mem::take(&mut self.state),
            std::mem::take(&mut self.events),
            std::mem::take(&mut self.arrivals),
        ))
    }

    fn emit(&mut self, kind: LivingEventKindV2) -> Result<(), LivingDeterministicFaultV2> {
        self.events.record(
            LivingEventProvenanceV2::Autonomous {
                phase: LivingPhaseV2::ResolveArrivals,
            },
            kind,
        )?;
        Ok(())
    }

    fn visit_next(&mut self) -> Result<bool, LivingDeterministicFaultV2> {
        match self.cursor {
            ArrivalCursor::Fleets => {
                if let Some(fleet) = self.fleets.as_slice().first() {
                    ensure_retained_sorted(&self.state.fleets, fleet)?;
                }
                self.active_fleet = self.fleets.next();
                if let Some(fleet) = self.active_fleet.as_mut() {
                    if let LivingFleetLocationV2::Travelling {
                        destination_world,
                        arrival_tick,
                        ..
                    } = fleet.location
                        && arrival_tick.0 <= self.tick.0
                    {
                        let kind = fleet.order.as_ref().map(|order| order.kind);
                        let target = fleet.order.as_ref().and_then(|order| order.target_world);
                        fleet.location = LivingFleetLocationV2::Docked {
                            world: destination_world,
                        };
                        let clear = match kind {
                            Some(LivingFleetOrderKindV2::Move | LivingFleetOrderKindV2::Return) => {
                                true
                            }
                            Some(
                                LivingFleetOrderKindV2::Attack | LivingFleetOrderKindV2::Settle,
                            ) => target != Some(destination_world),
                            Some(
                                LivingFleetOrderKindV2::Explore | LivingFleetOrderKindV2::Defend,
                            ) => target.is_none() || target == Some(destination_world),
                            None => false,
                        };
                        if clear {
                            self.retirement.start(fleet.order.take());
                        }
                        self.arrivals.push(fleet.id);
                        self.emit(LivingEventKindV2::FleetArrived)?;
                    }
                    self.state
                        .fleets
                        .push_sorted(self.active_fleet.take().expect("retained arriving fleet"))?;
                    return Ok(true);
                }
                self.cursor = ArrivalCursor::Worlds;
            }
            ArrivalCursor::Worlds => {
                if let Some(world) = self.state.worlds.as_slice().get(self.index) {
                    self.world_meta
                        .insert(world.id, (self.index, world.owner, world.system));
                    self.index += 1;
                    return Ok(true);
                }
                self.index = 0;
                self.cursor = ArrivalCursor::Systems;
            }
            ArrivalCursor::Systems => {
                if let Some(system) = self.state.systems.as_slice().get(self.index) {
                    self.graph.insert(system.id, Vec::new());
                    self.index += 1;
                    return Ok(true);
                }
                self.index = 0;
                self.cursor = ArrivalCursor::Lanes;
            }
            ArrivalCursor::Lanes => {
                if let Some(lane) = self.state.lanes.as_slice().get(self.index) {
                    if self.graph.contains_key(&lane.system_a)
                        && self.graph.contains_key(&lane.system_b)
                    {
                        let cost = lane.distance_units.div_ceil(10).max(10);
                        self.graph
                            .get_mut(&lane.system_a)
                            .unwrap()
                            .push((lane.system_b, cost));
                        self.graph
                            .get_mut(&lane.system_b)
                            .unwrap()
                            .push((lane.system_a, cost));
                    }
                    self.index += 1;
                    return Ok(true);
                }
                self.index = 0;
                self.cursor = ArrivalCursor::Wars;
            }
            ArrivalCursor::Wars => {
                if let Some(war) = self.state.wars.as_slice().get(self.index) {
                    if war.start_tick.0 <= self.tick.0 && self.tick.0 < war.end_tick.0 {
                        self.active_wars.insert(war.living_key());
                    }
                    self.index += 1;
                    return Ok(true);
                }
                self.cursor = ArrivalCursor::Relations;
            }
            ArrivalCursor::Relations => {
                if let Some(row) = self.relations.next() {
                    self.relation_rows.insert(row.living_key(), row);
                    return Ok(true);
                }
                self.cursor = ArrivalCursor::Shipments;
            }
            ArrivalCursor::Shipments => return self.visit_cargo(),
            ArrivalCursor::RelationOutput => {
                if let Some((_, row)) = self.relation_output.as_mut().and_then(Iterator::next) {
                    self.state.relations.push_sorted(row)?;
                    return Ok(true);
                }
                self.cursor = ArrivalCursor::RetireIndices;
            }
            ArrivalCursor::RetireIndices => {
                if self.world_meta.pop_first().is_some() || self.active_wars.pop_first().is_some() {
                    return Ok(true);
                }
                if let Some(mut entry) = self.graph.first_entry() {
                    if entry.get_mut().pop().is_none() {
                        entry.remove();
                    }
                    return Ok(true);
                }
                self.cursor = ArrivalCursor::Ready;
            }
            ArrivalCursor::Ready => {}
        }
        Ok(false)
    }

    fn visit_cargo(&mut self) -> Result<bool, LivingDeterministicFaultV2> {
        let Some((mut cargo, cursor)) = self.cargo.take() else {
            let Some(cargo) = self.shipments.next() else {
                self.relation_output = Some(std::mem::take(&mut self.relation_rows).into_iter());
                self.cursor = ArrivalCursor::RelationOutput;
                return Ok(false);
            };
            let due = cargo.disposition == LivingShipmentDispositionV2::WaitingForStorage
                || cargo.arrival_tick.0 <= self.tick.0;
            let returning = cargo.disposition == LivingShipmentDispositionV2::Returning
                || cargo.returning_to_source;
            let owner = self
                .world_meta
                .get(&cargo.destination_world)
                .and_then(|row| row.1);
            let pair = if cargo.dispatch_owner < cargo.intended_receiver {
                (cargo.dispatch_owner, cargo.intended_receiver)
            } else {
                (cargo.intended_receiver, cargo.dispatch_owner)
            };
            let cursor = if !due {
                CargoCursor::Output
            } else if !returning
                && owner == Some(cargo.intended_receiver)
                && self.active_wars.contains(&pair)
            {
                let from = self
                    .world_meta
                    .get(&cargo.destination_world)
                    .map(|row| row.2);
                let to = self.world_meta.get(&cargo.source_world).map(|row| row.2);
                self.search = Some(CostSearch::begin(from, to, &self.graph));
                CargoCursor::Travel
            } else {
                CargoCursor::Credit
            };
            self.cargo = Some((cargo, cursor));
            return Ok(true);
        };
        match cursor {
            CargoCursor::Travel => {
                let search = self
                    .search
                    .as_mut()
                    .ok_or(LivingDeterministicFaultV2::Invariant)?;
                if search.result.is_some() {
                    self.cargo = Some((cargo, CargoCursor::RetireSearch));
                    return Ok(false);
                }
                let visited = search.visit(&self.graph);
                self.cargo = Some((cargo, CargoCursor::Travel));
                return Ok(visited);
            }
            CargoCursor::RetireSearch => {
                let search = self
                    .search
                    .as_mut()
                    .ok_or(LivingDeterministicFaultV2::Invariant)?;
                if search.distance.pop_first().is_some() || search.frontier.pop_first().is_some() {
                    self.cargo = Some((cargo, CargoCursor::RetireSearch));
                    return Ok(true);
                }
                self.cargo = Some((cargo, CargoCursor::Return));
                return Ok(false);
            }
            CargoCursor::Return => {
                let cost = self
                    .search
                    .take()
                    .and_then(|search| search.result)
                    .flatten()
                    .unwrap_or(10);
                cargo.disposition = LivingShipmentDispositionV2::Returning;
                cargo.returning_to_source = false;
                cargo.departure_tick = self.tick;
                cargo.arrival_tick = LivingTickV2(
                    self.tick
                        .0
                        .checked_add(cost)
                        .ok_or(LivingDeterministicFaultV2::Arithmetic)?,
                );
                self.cargo = Some((cargo, CargoCursor::Output));
            }
            CargoCursor::Credit => {
                let returning = cargo.disposition == LivingShipmentDispositionV2::Returning
                    || cargo.returning_to_source;
                let destination = if returning {
                    cargo.source_world
                } else {
                    cargo.destination_world
                };
                let meta = self.world_meta.get(&destination).copied();
                let mut credited = false;
                if let Some((index, _, _)) = meta {
                    let world = self
                        .state
                        .worlds
                        .record_at_mut(index)
                        .ok_or(LivingDeterministicFaultV2::Invariant)?;
                    if let Some(next) = world
                        .inventory
                        .quantity(cargo.resource)
                        .checked_add(cargo.units)
                        && next <= LIVING_RESOURCE_STORAGE_LIMIT_V2
                    {
                        match cargo.resource {
                            LivingResourceV2::Energy => world.inventory.energy = next,
                            LivingResourceV2::Ore => world.inventory.ore = next,
                            LivingResourceV2::Alloy => world.inventory.alloy = next,
                        }
                        credited = true;
                    }
                }
                if !credited {
                    cargo.disposition = LivingShipmentDispositionV2::WaitingForStorage;
                    cargo.returning_to_source = returning;
                    self.emit(LivingEventKindV2::ShipmentWaiting)?;
                    self.cargo = Some((cargo, CargoCursor::Output));
                } else {
                    let captured =
                        !returning && meta.and_then(|row| row.1) != Some(cargo.intended_receiver);
                    if captured {
                        self.emit(LivingEventKindV2::ShipmentCaptured)?;
                    } else if returning {
                        self.emit(LivingEventKindV2::ShipmentReturned)?;
                    } else {
                        self.events.record_explained(
                            LivingEventProvenanceV2::Autonomous {
                                phase: LivingPhaseV2::ResolveArrivals,
                            },
                            LivingEventKindV2::ShipmentDelivered,
                            super::event_facts::LivingEventFactsV2 {
                                delivery: Some(super::event_facts::LivingDeliveryFactV2 {
                                    source_world: cargo.source_world,
                                    destination_world: cargo.destination_world,
                                    dispatch_owner: cargo.dispatch_owner,
                                    intended_receiver: cargo.intended_receiver,
                                    recipient: cargo.intended_receiver,
                                    resource: cargo.resource,
                                    units: cargo.units,
                                    dispatch_kind: cargo.dispatch_kind,
                                    source_system: self
                                        .world_meta
                                        .get(&cargo.source_world)
                                        .map(|row| row.2),
                                    destination_system: meta.map(|row| row.2),
                                }),
                                ..Default::default()
                            },
                        )?;
                    }
                    if !captured
                        && !returning
                        && cargo.dispatch_owner != cargo.intended_receiver
                        && matches!(
                            cargo.dispatch_kind,
                            Some(LivingRouteKindV2::Aid | LivingRouteKindV2::Trade)
                        )
                    {
                        self.cargo = Some((cargo, CargoCursor::Relation));
                    }
                }
            }
            CargoCursor::Relation => {
                let from = cargo.intended_receiver;
                let to = cargo.dispatch_owner;
                let relation = self
                    .relation_rows
                    .entry((from, to))
                    .or_insert(LivingRelationV2 {
                        from,
                        to,
                        base_disposition: 0,
                        reasons: LivingRelationReasonsV2::default(),
                    });
                super::rules::apply_delivery_credit(
                    relation,
                    cargo
                        .dispatch_kind
                        .ok_or(LivingDeterministicFaultV2::Invariant)?,
                    cargo.units,
                )?;
            }
            CargoCursor::Output => {
                self.state.shipments.push_sorted(cargo)?;
            }
        }
        Ok(true)
    }
}

/// Output of the scheduled lifecycle and arrival phases, still unpublished.
pub type LivingScheduledPhaseOutputV2 = (
    LivingGalaxyStateV2,
    LivingPendingEventsV2,
    Vec<LivingEntityIdV2>,
);

#[derive(Debug)]
enum ScheduledCursor {
    Lifecycles(Box<LivingLifecycleJobV2>),
    Arrivals(Box<LivingArrivalJobV2>),
    Ready(Option<Box<LivingScheduledPhaseOutputV2>>),
}

/// Compose phases 2 and 3 under one shared record-visit budget.
///
/// The input is the private candidate after creator commands. Phases 4 through
/// 10 and final validation/sealing remain the caller's responsibility. `Ready`
/// means these two phases are complete, never that a full boundary is committed.
#[derive(Debug)]
pub struct LivingScheduledPhasesJobV2 {
    retirement: PhaseRetirementLane<'static>,
    delivered: bool,
    cursor: ScheduledCursor,
    tick: LivingTickV2,
    cancelled: bool,
    fault: Option<LivingLifecycleJobErrorV2>,
    last_poll_visits: usize,
    total_visits: u64,
}

impl LivingScheduledPhasesJobV2 {
    #[cfg(test)]
    pub(crate) fn phase(&self) -> Option<LivingPhaseV2> {
        match &self.cursor {
            ScheduledCursor::Lifecycles(_) => Some(LivingPhaseV2::ExpireLifecycles),
            ScheduledCursor::Arrivals(_) => Some(LivingPhaseV2::ResolveArrivals),
            ScheduledCursor::Ready(_) => None,
        }
    }
    /// Move the unpublished candidate into the first phase, without cloning.
    pub fn begin(
        state: LivingGalaxyStateV2,
        events: LivingPendingEventsV2,
        tick: LivingTickV2,
    ) -> Self {
        Self {
            retirement: PhaseRetirementLane::default(),
            delivered: false,
            cursor: ScheduledCursor::Lifecycles(Box::new(LivingLifecycleJobV2::begin(
                state, events, tick,
            ))),
            tick,
            cancelled: false,
            fault: None,
            last_poll_visits: 0,
            total_visits: 0,
        }
    }
    /// Cancel the entire unpublished sequence.
    pub fn cancel(&mut self) {
        self.cancelled = true;
    }
    /// Visits consumed across both phases by the last poll.
    pub const fn last_poll_visits(&self) -> usize {
        self.last_poll_visits
    }
    /// Visits consumed across both phases since creation.
    pub const fn total_visits(&self) -> u64 {
        self.total_visits
    }
    /// Poll both phase cursors with a single total budget of at most 1024 visits.
    pub fn poll(
        &mut self,
        budget: usize,
    ) -> Result<LivingLifecycleProgressV2, LivingLifecycleJobErrorV2> {
        self.last_poll_visits = 0;
        if self.cancelled {
            return Err(LivingLifecycleJobErrorV2::Cancelled);
        }
        if budget == 0 || budget > LIVING_MAX_PHASE_POLL_VISITS_V2 {
            return Err(LivingLifecycleJobErrorV2::Budget);
        }
        if let Some(error) = &self.fault {
            return Err(error.clone());
        }
        while self.last_poll_visits < budget {
            if self.retirement.visit() {
                self.last_poll_visits += 1;
                self.total_visits += 1;
                continue;
            }

            let remaining = budget - self.last_poll_visits;
            let (result, visits) = match &mut self.cursor {
                ScheduledCursor::Lifecycles(job) => (job.poll(remaining), job.last_poll_visits()),
                ScheduledCursor::Arrivals(job) => (job.poll(remaining), job.last_poll_visits()),
                ScheduledCursor::Ready(_) => return Ok(LivingLifecycleProgressV2::Ready),
            };
            self.last_poll_visits += visits;
            self.total_visits += visits as u64;
            let progress = match result {
                Ok(progress) => progress,
                Err(error) => {
                    self.fault = Some(error.clone());
                    return Err(error);
                }
            };
            if progress == LivingLifecycleProgressV2::Pending {
                return Ok(progress);
            }
            if self.last_poll_visits == budget {
                break;
            }
            self.last_poll_visits += 1;
            self.total_visits += 1;
            match std::mem::replace(&mut self.cursor, ScheduledCursor::Ready(None)) {
                ScheduledCursor::Lifecycles(mut job) => {
                    let (state, events) = match job.take_ready_result() {
                        Ok(output) => output,
                        Err(error) => {
                            self.cursor = ScheduledCursor::Lifecycles(job);
                            return Err(error);
                        }
                    };
                    self.retirement.start(job);
                    self.cursor = ScheduledCursor::Arrivals(Box::new(LivingArrivalJobV2::begin(
                        state, events, self.tick,
                    )));
                }
                ScheduledCursor::Arrivals(mut job) => {
                    let output = match job.take_ready_result() {
                        Ok(output) => output,
                        Err(error) => {
                            self.cursor = ScheduledCursor::Arrivals(job);
                            return Err(error);
                        }
                    };
                    self.retirement.start(job);
                    self.cursor = ScheduledCursor::Ready(Some(Box::new(output)));
                }
                ScheduledCursor::Ready(output) => {
                    self.cursor = ScheduledCursor::Ready(output);
                }
            }
        }
        Ok(
            if matches!(self.cursor, ScheduledCursor::Ready(_)) && self.retirement.node.is_none() {
                LivingLifecycleProgressV2::Ready
            } else {
                LivingLifecycleProgressV2::Pending
            },
        )
    }
    /// Consume completed scheduled phases, rejecting cancelled or faulted work.
    pub fn finish(mut self) -> Result<LivingScheduledPhaseOutputV2, LivingLifecycleJobErrorV2> {
        self.take_ready_result()
    }
    /// Borrowed one-time extraction; rejected calls preserve every owner.
    pub fn take_ready_result(
        &mut self,
    ) -> Result<LivingScheduledPhaseOutputV2, LivingLifecycleJobErrorV2> {
        if self.cancelled {
            return Err(LivingLifecycleJobErrorV2::Cancelled);
        }
        if let Some(error) = self.fault.clone() {
            return Err(error);
        }
        if self.delivered || self.retirement.node.is_some() {
            return Err(LivingLifecycleJobErrorV2::NotReady);
        }
        match &mut self.cursor {
            ScheduledCursor::Ready(Some(output)) => {
                self.delivered = true;
                Ok((
                    std::mem::take(&mut output.0),
                    std::mem::take(&mut output.1),
                    std::mem::take(&mut output.2),
                ))
            }
            _ => Err(LivingLifecycleJobErrorV2::NotReady),
        }
    }
}

// --------------------------------------------------------- candidate copying

use super::model::{
    LivingColonyV2, LivingFacilityV2, LivingFleetOrderV2, LivingHubV2, LivingObservationV2,
};
use super::wire::LivingSortedVecV2;
use std::sync::Arc;

#[derive(Debug)]
enum CopyNested {
    Colony(Box<LivingColonyV2>, usize),
    Facility(Box<LivingFacilityV2>, usize),
    Fleet(Box<LivingFleetV2>, usize, usize),
    Observation(Box<LivingObservationV2>, usize, usize),
}

/// Incrementally copy an immutable authority publication into a private candidate.
///
/// Beginning the job retains one Arc and copies only scalar header metadata.
/// Nested hulls, blocked reasons, observation counts and lane references each
/// consume their own visit. No full state or nested vector is cloned in a poll.
/// This prepares a candidate; it does not validate or publish a boundary.
#[derive(Debug)]
pub struct LivingStateCopyJobV2 {
    source: Arc<LivingGalaxyStateV2>,
    candidate: LivingGalaxyStateV2,
    collection: u8,
    prepared: bool,
    storage_preparations: u64,
    delivered: bool,
    index: usize,
    nested: Option<CopyNested>,
    cancelled: bool,
    fault: Option<LivingDeterministicFaultV2>,
    last_poll_visits: usize,
    total_visits: u64,
}

impl LivingStateCopyJobV2 {
    /// Retain an existing immutable publication without visiting entities.
    pub fn begin(source: Arc<LivingGalaxyStateV2>) -> Self {
        let candidate = LivingGalaxyStateV2 {
            tick: source.tick,
            accepted_sequence: source.accepted_sequence,
            branch_sequence: source.branch_sequence,
            counters: source.counters,
            ..LivingGalaxyStateV2::default()
        };
        Self {
            source,
            candidate,
            collection: 0,
            prepared: false,
            storage_preparations: 0,
            delivered: false,
            index: 0,
            nested: None,
            cancelled: false,
            fault: None,
            last_poll_visits: 0,
            total_visits: 0,
        }
    }
    pub(crate) fn into_retirement(
        self,
        policy: LivingStateRetirementPolicyV2,
    ) -> LivingCopyHashRetirementJobV2 {
        let mut candidate = LivingStateRetirementJobV2::begin_owned(self.candidate);
        candidate.parent = self.nested.map(StateRetirementParent::from_copy);
        LivingCopyHashRetirementJobV2::new(
            self.source,
            policy,
            Some(candidate),
            None,
            #[cfg(test)]
            Vec::new(),
        )
    }
    /// Cancel the private copy while leaving its source publication unchanged.
    pub fn cancel(&mut self) {
        self.cancelled = true;
    }
    /// Record or storage-preparation visits consumed by the preceding poll.
    pub const fn last_poll_visits(&self) -> usize {
        self.last_poll_visits
    }
    /// Record or storage-preparation visits consumed since creation.
    pub const fn total_visits(&self) -> u64 {
        self.total_visits
    }
    /// Copy using at most 1024 record, lane-reference or preparation visits.
    pub fn poll(
        &mut self,
        budget: usize,
    ) -> Result<LivingLifecycleProgressV2, LivingLifecycleJobErrorV2> {
        self.last_poll_visits = 0;
        if self.cancelled {
            return Err(LivingLifecycleJobErrorV2::Cancelled);
        }
        if budget == 0 || budget > LIVING_MAX_PHASE_POLL_VISITS_V2 {
            return Err(LivingLifecycleJobErrorV2::Budget);
        }
        if let Some(fault) = &self.fault {
            return Err(LivingLifecycleJobErrorV2::Fault(fault.clone()));
        }
        while self.last_poll_visits < budget && self.collection < 20 {
            match self.visit_next() {
                Ok(true) => {
                    self.last_poll_visits += 1;
                    self.total_visits += 1;
                }
                Ok(false) => {}
                Err(fault) => {
                    self.last_poll_visits += 1;
                    self.total_visits += 1;
                    self.fault = Some(fault.clone());
                    return Err(LivingLifecycleJobErrorV2::Fault(fault));
                }
            }
        }
        Ok(if self.collection == 20 {
            LivingLifecycleProgressV2::Ready
        } else {
            LivingLifecycleProgressV2::Pending
        })
    }
    /// Consume a complete copy; incomplete, cancelled and faulted copies cannot escape.
    pub fn finish(mut self) -> Result<LivingGalaxyStateV2, LivingLifecycleJobErrorV2> {
        self.take_ready_result()
    }
    /// Move the candidate once; retain all remaining owners for explicit retirement.
    pub fn take_ready_result(&mut self) -> Result<LivingGalaxyStateV2, LivingLifecycleJobErrorV2> {
        if self.cancelled {
            return Err(LivingLifecycleJobErrorV2::Cancelled);
        }
        if let Some(fault) = &self.fault {
            return Err(LivingLifecycleJobErrorV2::Fault(fault.clone()));
        }
        if self.collection != 20 || self.delivered {
            return Err(LivingLifecycleJobErrorV2::NotReady);
        }
        self.delivered = true;
        Ok(std::mem::take(&mut self.candidate))
    }

    fn visit_next(&mut self) -> Result<bool, LivingDeterministicFaultV2> {
        if self.nested.is_some() {
            return self.visit_nested();
        }
        if !self.prepared {
            macro_rules! prepare {
                ($field:ident) => {
                    self.candidate.$field = LivingSortedVecV2::new(Vec::with_capacity(
                        self.source.$field.as_slice().len(),
                    ))?
                };
            }
            match self.collection {
                0 => prepare!(systems),
                1 => prepare!(stars),
                2 => prepare!(worlds),
                3 => prepare!(lanes),
                4 => prepare!(civilizations),
                5 => prepare!(deposits),
                6 => prepare!(colonies),
                7 => prepare!(facilities),
                8 => prepare!(construction_jobs),
                9 => prepare!(hull_jobs),
                10 => prepare!(fleets),
                11 => prepare!(routes),
                12 => prepare!(shipments),
                13 => prepare!(relations),
                14 => prepare!(agreements),
                15 => prepare!(wars),
                16 => prepare!(observations),
                17 => prepare!(hazards),
                18 => prepare!(settlement_claims),
                19 => prepare!(occupations),
                _ => return Ok(false),
            }
            self.storage_preparations += 1;
            self.prepared = true;
            return Ok(true);
        }
        macro_rules! plain {
            ($field:ident) => {
                if let Some(row) = self.source.$field.as_slice().get(self.index) {
                    self.candidate.$field.push_sorted(row.clone())?;
                    self.index += 1;
                    return Ok(true);
                }
            };
        }
        match self.collection {
            0 => {
                plain!(systems);
            }
            1 => {
                plain!(stars);
            }
            2 => {
                plain!(worlds);
            }
            3 => {
                plain!(lanes);
            }
            4 => {
                plain!(civilizations);
            }
            5 => {
                plain!(deposits);
            }
            6 => {
                if let Some(row) = self.source.colonies.as_slice().get(self.index) {
                    self.storage_preparations += 2;
                    self.nested = Some(CopyNested::Colony(
                        Box::new(LivingColonyV2 {
                            world: row.world,
                            owner: row.owner,
                            hub: LivingHubV2 {
                                energy_next_due: row.hub.energy_next_due,
                                ore_next_due: row.hub.ore_next_due,
                                fallback_next_due: row.hub.fallback_next_due,
                                blocked_reasons: LivingSortedVecV2::new(Vec::with_capacity(
                                    row.hub.blocked_reasons.as_slice().len(),
                                ))?,
                            },
                        }),
                        0,
                    ));
                    return Ok(true);
                }
            }
            7 => {
                if let Some(row) = self.source.facilities.as_slice().get(self.index) {
                    self.storage_preparations += 2;
                    self.nested = Some(CopyNested::Facility(
                        Box::new(LivingFacilityV2 {
                            id: row.id,
                            world: row.world,
                            definition: row.definition.clone(),
                            status: row.status,
                            paid_alloy: row.paid_alloy,
                            hit_points: row.hit_points,
                            next_due: row.next_due,
                            next_repair: row.next_repair,
                            blocked_reasons: LivingSortedVecV2::new(Vec::with_capacity(
                                row.blocked_reasons.as_slice().len(),
                            ))?,
                            disabled: row.disabled,
                        }),
                        0,
                    ));
                    return Ok(true);
                }
            }
            8 => {
                plain!(construction_jobs);
            }
            9 => {
                plain!(hull_jobs);
            }
            10 => {
                if let Some(row) = self.source.fleets.as_slice().get(self.index) {
                    self.storage_preparations += 2 + u64::from(row.order.is_some());
                    self.nested = Some(CopyNested::Fleet(
                        Box::new(LivingFleetV2 {
                            id: row.id,
                            owner: row.owner,
                            location: row.location,
                            order: row.order.as_ref().map(|order| LivingFleetOrderV2 {
                                kind: order.kind,
                                target_world: order.target_world,
                                not_before_tick: order.not_before_tick,
                                lane_path: Vec::with_capacity(order.lane_path.len()),
                            }),
                            hulls: LivingSortedVecV2::new(Vec::with_capacity(
                                row.hulls.as_slice().len(),
                            ))?,
                        }),
                        0,
                        0,
                    ));
                    return Ok(true);
                }
            }
            11 => {
                plain!(routes);
            }
            12 => {
                plain!(shipments);
            }
            13 => {
                plain!(relations);
            }
            14 => {
                plain!(agreements);
            }
            15 => {
                plain!(wars);
            }
            16 => {
                if let Some(row) = self.source.observations.as_slice().get(self.index) {
                    self.storage_preparations += 3;
                    self.nested = Some(CopyNested::Observation(
                        Box::new(LivingObservationV2 {
                            civilization: row.civilization,
                            world: row.world,
                            observed_tick: row.observed_tick,
                            owner: row.owner,
                            remaining_deposit: row.remaining_deposit,
                            facilities: LivingSortedVecV2::new(Vec::with_capacity(
                                row.facilities.as_slice().len(),
                            ))?,
                            stationed_hulls: LivingSortedVecV2::new(Vec::with_capacity(
                                row.stationed_hulls.as_slice().len(),
                            ))?,
                            operational_defense_batteries: row.operational_defense_batteries,
                        }),
                        0,
                        0,
                    ));
                    return Ok(true);
                }
            }
            17 => {
                plain!(hazards);
            }
            18 => {
                plain!(settlement_claims);
            }
            19 => {
                plain!(occupations);
            }
            _ => {}
        }
        self.collection += 1;
        self.prepared = false;
        self.index = 0;
        Ok(false)
    }

    fn visit_nested(&mut self) -> Result<bool, LivingDeterministicFaultV2> {
        match self
            .nested
            .as_mut()
            .ok_or(LivingDeterministicFaultV2::Invariant)?
        {
            CopyNested::Colony(copy, index) => {
                let source = &self.source.colonies.as_slice()[self.index]
                    .hub
                    .blocked_reasons;
                if let Some(row) = source.as_slice().get(*index) {
                    copy.hub.blocked_reasons.push_sorted(*row)?;
                    *index += 1;
                    return Ok(true);
                }
            }
            CopyNested::Facility(copy, index) => {
                let source = &self.source.facilities.as_slice()[self.index].blocked_reasons;
                if let Some(row) = source.as_slice().get(*index) {
                    copy.blocked_reasons.push_sorted(*row)?;
                    *index += 1;
                    return Ok(true);
                }
            }
            CopyNested::Fleet(copy, lane, hull) => {
                let source = &self.source.fleets.as_slice()[self.index];
                if let Some(id) = source
                    .order
                    .as_ref()
                    .and_then(|order| order.lane_path.get(*lane))
                {
                    copy.order
                        .as_mut()
                        .ok_or(LivingDeterministicFaultV2::Invariant)?
                        .lane_path
                        .push(*id);
                    *lane += 1;
                    return Ok(true);
                }
                if let Some(row) = source.hulls.as_slice().get(*hull) {
                    copy.hulls.push_sorted(row.clone())?;
                    *hull += 1;
                    return Ok(true);
                }
            }
            CopyNested::Observation(copy, facility, hull) => {
                let source = &self.source.observations.as_slice()[self.index];
                if let Some(row) = source.facilities.as_slice().get(*facility) {
                    copy.facilities.push_sorted(row.clone())?;
                    *facility += 1;
                    return Ok(true);
                }
                if let Some(row) = source.stationed_hulls.as_slice().get(*hull) {
                    copy.stationed_hulls.push_sorted(row.clone())?;
                    *hull += 1;
                    return Ok(true);
                }
            }
        }
        // Check before taking ownership: push_sorted consumes its rejected item.
        let ordered = match self.nested.as_ref().expect("partial row retained") {
            CopyNested::Colony(row, _) => self
                .candidate
                .colonies
                .as_slice()
                .last()
                .is_none_or(|last| last.living_key() < row.living_key()),
            CopyNested::Facility(row, _) => self
                .candidate
                .facilities
                .as_slice()
                .last()
                .is_none_or(|last| last.living_key() < row.living_key()),
            CopyNested::Fleet(row, _, _) => self
                .candidate
                .fleets
                .as_slice()
                .last()
                .is_none_or(|last| last.living_key() < row.living_key()),
            CopyNested::Observation(row, _, _) => self
                .candidate
                .observations
                .as_slice()
                .last()
                .is_none_or(|last| last.living_key() < row.living_key()),
        };
        if !ordered {
            return Err(super::wire::LivingWireErrorV2::KeyedOrder.into());
        }
        match self
            .nested
            .take()
            .ok_or(LivingDeterministicFaultV2::Invariant)?
        {
            CopyNested::Colony(copy, _) => self.candidate.colonies.push_sorted(*copy)?,
            CopyNested::Facility(copy, _) => self.candidate.facilities.push_sorted(*copy)?,
            CopyNested::Fleet(copy, _, _) => self.candidate.fleets.push_sorted(*copy)?,
            CopyNested::Observation(copy, _, _) => {
                self.candidate.observations.push_sorted(*copy)?
            }
        }
        self.index += 1;
        Ok(true)
    }
}

/// Incremental canonical hashing. Each visit encodes one schema token (at most
/// 256 bytes), advances one nested cursor, or releases one exhausted buffer.
pub struct LivingStateHashJobV2 {
    source: Arc<LivingGalaxyStateV2>,
    normalized_tick: Option<LivingTickV2>,
    hasher: sha2::Sha256,
    collection: usize,
    index: usize,
    token: usize,
    header: usize,
    opened: bool,
    ready: bool,
    delivered: bool,
    child: Option<(HashChildren, usize, usize)>,
    fragment: Option<Vec<u8>>,
    bytes: usize,
    measured_bytes: usize,
    over_limit: bool,
    #[cfg(test)]
    byte_limit: usize,
    #[cfg(test)]
    encoded_prepared: bool,
    cancelled: bool,
    fault: Option<LivingDeterministicFaultV2>,
    last_poll_visits: usize,
    total_visits: u64,
    counts: HashVisitCounts,
    #[cfg(test)]
    encoded: Vec<u8>,
}
#[derive(Default, Debug)]
struct HashVisitCounts {
    tokens: u64,
    structural_tokens: u64,
    records: u64,
    nested_records: u64,
    storage_releases: u64,
    max_encoded_token: usize,
}
#[derive(Clone, Copy, Debug)]
enum HashChildren {
    HubReasons,
    FacilityReasons,
    FleetLanes,
    FleetHulls,
    ObservationFacilities,
    ObservationHulls,
}
enum HashToken {
    Bytes(Vec<u8>, bool),
    Children(HashChildren),
}

// Replaying a record visits fixed schema metadata only. It never serializes an
// ignored string leaf or traverses a nested collection. ID Serialize still
// formats its fixed 16 bytes into a 32-character temporary on metadata replay;
// this is bounded scalar overhead, not allocation-free or a timing guarantee.
// Only the selected primitive is JSON encoded. Current records have fewer than 64 fields including inline
// structs; token selection stops immediately at the selected token.
struct HashTokenSelector {
    remaining: usize,
    output: Option<HashToken>,
    collection: Option<usize>,
    visited_fields: usize,
}
#[derive(Debug)]
enum HashTokenError {
    Selected,
    Json,
}
impl std::fmt::Display for HashTokenError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("state token selection")
    }
}
impl std::error::Error for HashTokenError {}
impl serde::ser::Error for HashTokenError {
    fn custom<T: std::fmt::Display>(_: T) -> Self {
        Self::Json
    }
}
impl HashTokenSelector {
    fn selected(&mut self) -> bool {
        if self.remaining == 0 {
            true
        } else {
            self.remaining -= 1;
            false
        }
    }
    fn literal(&mut self, bytes: &'static [u8]) -> Result<(), HashTokenError> {
        if self.selected() {
            self.output = Some(HashToken::Bytes(bytes.to_vec(), true));
            Err(HashTokenError::Selected)
        } else {
            Ok(())
        }
    }
    fn primitive<T: serde::Serialize + ?Sized>(&mut self, value: &T) -> Result<(), HashTokenError> {
        if self.selected() {
            let bytes = serde_json::to_vec(value).map_err(|_| HashTokenError::Json)?;
            // Schema leaves: printable ASCII names <=64 (escaped <=130),
            // validated slugs <=64, entity IDs 32 hex bytes, integers <=20, fixed enum tags.
            if bytes.len() > 256 {
                return Err(HashTokenError::Json);
            }
            self.output = Some(HashToken::Bytes(bytes, false));
            Err(HashTokenError::Selected)
        } else {
            Ok(())
        }
    }
    fn nested(&mut self, kind: HashChildren) -> Result<(), HashTokenError> {
        self.literal(b"[")?;
        if self.selected() {
            self.output = Some(HashToken::Children(kind));
            return Err(HashTokenError::Selected);
        }
        self.literal(b"]")
    }
}
struct HashStruct<'a> {
    selector: &'a mut HashTokenSelector,
    first: bool,
}
impl serde::ser::SerializeStruct for HashStruct<'_> {
    type Ok = ();
    type Error = HashTokenError;
    fn serialize_field<T: serde::Serialize + ?Sized>(
        &mut self,
        key: &'static str,
        value: &T,
    ) -> Result<(), Self::Error> {
        self.selector.visited_fields += 1;
        if self.selector.visited_fields > 64 {
            return Err(HashTokenError::Json);
        }
        if !self.first {
            self.selector.literal(b",")?;
        }
        self.first = false;
        self.selector.primitive(key)?;
        self.selector.literal(b":")?;
        let kind = match (self.selector.collection, key) {
            (Some(6), "blocked_reasons") => Some(HashChildren::HubReasons),
            (Some(7), "blocked_reasons") => Some(HashChildren::FacilityReasons),
            (Some(10), "lane_path") => Some(HashChildren::FleetLanes),
            (Some(10), "hulls") => Some(HashChildren::FleetHulls),
            (Some(16), "facilities") => Some(HashChildren::ObservationFacilities),
            (Some(16), "stationed_hulls") => Some(HashChildren::ObservationHulls),
            _ => None,
        };
        if let Some(kind) = kind {
            self.selector.nested(kind)
        } else {
            value.serialize(&mut *self.selector)
        }
    }
    fn end(self) -> Result<(), Self::Error> {
        self.selector.literal(b"}")
    }
}
macro_rules! hash_primitive_serializer {
    ($($method:ident($ty:ty)),* $(,)?) => {$ (
        fn $method(self, value: $ty) -> Result<(), Self::Error> { self.primitive(&value) }
    )*};
}
impl<'a> serde::Serializer for &'a mut HashTokenSelector {
    type Ok = ();
    type Error = HashTokenError;
    type SerializeSeq = serde::ser::Impossible<(), HashTokenError>;
    type SerializeTuple = serde::ser::Impossible<(), HashTokenError>;
    type SerializeTupleStruct = serde::ser::Impossible<(), HashTokenError>;
    type SerializeTupleVariant = serde::ser::Impossible<(), HashTokenError>;
    type SerializeMap = serde::ser::Impossible<(), HashTokenError>;
    type SerializeStruct = HashStruct<'a>;
    type SerializeStructVariant = serde::ser::Impossible<(), HashTokenError>;
    hash_primitive_serializer!(
        serialize_bool(bool),
        serialize_i8(i8),
        serialize_i16(i16),
        serialize_i32(i32),
        serialize_i64(i64),
        serialize_u8(u8),
        serialize_u16(u16),
        serialize_u32(u32),
        serialize_u64(u64),
        serialize_char(char)
    );
    fn serialize_f32(self, _: f32) -> Result<(), Self::Error> {
        Err(HashTokenError::Json)
    }
    fn serialize_f64(self, _: f64) -> Result<(), Self::Error> {
        Err(HashTokenError::Json)
    }
    fn serialize_str(self, value: &str) -> Result<(), Self::Error> {
        // Check length without scanning or encoding ignored leaf contents.
        if value.len() > 64 {
            return Err(HashTokenError::Json);
        }
        self.primitive(value)
    }
    fn serialize_bytes(self, _: &[u8]) -> Result<(), Self::Error> {
        Err(HashTokenError::Json)
    }
    fn serialize_none(self) -> Result<(), Self::Error> {
        self.literal(b"null")
    }
    fn serialize_some<T: serde::Serialize + ?Sized>(self, value: &T) -> Result<(), Self::Error> {
        value.serialize(self)
    }
    fn serialize_unit(self) -> Result<(), Self::Error> {
        self.literal(b"null")
    }
    fn serialize_unit_struct(self, _: &'static str) -> Result<(), Self::Error> {
        self.literal(b"null")
    }
    fn serialize_unit_variant(
        self,
        _: &'static str,
        _: u32,
        value: &'static str,
    ) -> Result<(), Self::Error> {
        self.primitive(value)
    }
    fn serialize_newtype_struct<T: serde::Serialize + ?Sized>(
        self,
        _: &'static str,
        value: &T,
    ) -> Result<(), Self::Error> {
        value.serialize(self)
    }
    fn serialize_newtype_variant<T: serde::Serialize + ?Sized>(
        self,
        _: &'static str,
        _: u32,
        _: &'static str,
        _: &T,
    ) -> Result<(), Self::Error> {
        Err(HashTokenError::Json)
    }
    fn serialize_seq(self, _: Option<usize>) -> Result<Self::SerializeSeq, Self::Error> {
        Err(HashTokenError::Json)
    }
    fn serialize_tuple(self, _: usize) -> Result<Self::SerializeTuple, Self::Error> {
        Err(HashTokenError::Json)
    }
    fn serialize_tuple_struct(
        self,
        _: &'static str,
        _: usize,
    ) -> Result<Self::SerializeTupleStruct, Self::Error> {
        Err(HashTokenError::Json)
    }
    fn serialize_tuple_variant(
        self,
        _: &'static str,
        _: u32,
        _: &'static str,
        _: usize,
    ) -> Result<Self::SerializeTupleVariant, Self::Error> {
        Err(HashTokenError::Json)
    }
    fn serialize_map(self, _: Option<usize>) -> Result<Self::SerializeMap, Self::Error> {
        Err(HashTokenError::Json)
    }
    fn serialize_struct(
        self,
        _: &'static str,
        _: usize,
    ) -> Result<Self::SerializeStruct, Self::Error> {
        self.literal(b"{")?;
        Ok(HashStruct {
            selector: self,
            first: true,
        })
    }
    fn serialize_struct_variant(
        self,
        _: &'static str,
        _: u32,
        _: &'static str,
        _: usize,
    ) -> Result<Self::SerializeStructVariant, Self::Error> {
        Err(HashTokenError::Json)
    }
}
fn state_hash_token<T: serde::Serialize>(
    value: &T,
    token: usize,
    collection: Option<usize>,
) -> Result<Option<HashToken>, LivingDeterministicFaultV2> {
    let mut selector = HashTokenSelector {
        remaining: token,
        output: None,
        collection,
        visited_fields: 0,
    };
    match value.serialize(&mut selector) {
        Ok(()) => Ok(None),
        Err(HashTokenError::Selected) => Ok(selector.output),
        Err(HashTokenError::Json) => Err(super::wire::LivingWireErrorV2::Json.into()),
    }
}
const STATE_COLLECTION_NAMES: [&str; 20] = [
    "systems",
    "stars",
    "worlds",
    "lanes",
    "civilizations",
    "deposits",
    "colonies",
    "facilities",
    "construction_jobs",
    "hull_jobs",
    "fleets",
    "routes",
    "shipments",
    "relations",
    "agreements",
    "wars",
    "observations",
    "hazards",
    "settlement_claims",
    "occupations",
];

impl LivingStateHashJobV2 {
    /// Retain one source owner without traversing state.
    pub fn begin(source: Arc<LivingGalaxyStateV2>) -> Self {
        use sha2::Digest;
        let mut hasher = sha2::Sha256::new();
        hasher.update(b"NYON-LIVING-STATE-V2\0");
        Self {
            source,
            normalized_tick: None,
            hasher,
            collection: 0,
            index: 0,
            token: 0,
            header: 0,
            opened: false,
            ready: false,
            delivered: false,
            child: None,
            fragment: None,
            bytes: 0,
            measured_bytes: 0,
            over_limit: false,
            #[cfg(test)]
            byte_limit: super::wire::LIVING_MAX_ARCHIVE_BYTES_V2,
            #[cfg(test)]
            encoded_prepared: false,
            cancelled: false,
            fault: None,
            last_poll_visits: 0,
            total_visits: 0,
            counts: HashVisitCounts::default(),
            #[cfg(test)]
            encoded: Vec::new(),
        }
    }
    fn begin_decision_snapshot(source: Arc<LivingGalaxyStateV2>, tick: LivingTickV2) -> Self {
        let valid = tick.0 != 0 && source.tick.0.checked_add(1) == Some(tick.0);
        let mut job = Self::begin(source);
        job.normalized_tick = Some(tick);
        if !valid {
            job.fault = Some(LivingDeterministicFaultV2::Decision(
                super::decision::LivingDecisionErrorV2::Snapshot,
            ));
        }
        job
    }
    pub(crate) fn into_retirement(
        self,
        policy: LivingStateRetirementPolicyV2,
    ) -> LivingCopyHashRetirementJobV2 {
        LivingCopyHashRetirementJobV2::new(
            self.source,
            policy,
            None,
            self.fragment,
            #[cfg(test)]
            self.encoded,
        )
    }
    /// Cancel without dropping any owners or exposing a digest.
    pub fn cancel(&mut self) {
        self.cancelled = true;
    }
    /// Logical visits used by the preceding poll.
    pub const fn last_poll_visits(&self) -> usize {
        self.last_poll_visits
    }
    /// Logical visits used since construction.
    pub const fn total_visits(&self) -> u64 {
        self.total_visits
    }
    /// Bytes actually fed to SHA-256; oversize measurement is tracked separately.
    pub const fn encoded_bytes(&self) -> usize {
        self.bytes
    }
    /// Encode or release at most the requested number of bounded units.
    pub fn poll(
        &mut self,
        budget: usize,
    ) -> Result<LivingLifecycleProgressV2, LivingLifecycleJobErrorV2> {
        self.last_poll_visits = 0;
        if self.cancelled {
            return Err(LivingLifecycleJobErrorV2::Cancelled);
        }
        if budget == 0 || budget > LIVING_MAX_PHASE_POLL_VISITS_V2 {
            return Err(LivingLifecycleJobErrorV2::Budget);
        }
        if let Some(fault) = &self.fault {
            return Err(LivingLifecycleJobErrorV2::Fault(fault.clone()));
        }
        while !self.ready && self.last_poll_visits < budget {
            self.last_poll_visits += 1;
            self.total_visits += 1;
            if let Err(fault) = self.visit_next_hash() {
                self.fault = Some(fault.clone());
                return Err(LivingLifecycleJobErrorV2::Fault(fault));
            }
        }
        Ok(if self.ready {
            LivingLifecycleProgressV2::Ready
        } else {
            LivingLifecycleProgressV2::Pending
        })
    }
    /// Compatibility consuming extraction; bounded callers use borrowed extraction
    /// and transfer remaining owners to explicit retirement.
    pub fn finish(mut self) -> Result<super::ids::LivingStateDigestV2, LivingLifecycleJobErrorV2> {
        self.take_ready_result()
    }
    /// Extract once without releasing source or temporary owners.
    pub fn take_ready_result(
        &mut self,
    ) -> Result<super::ids::LivingStateDigestV2, LivingLifecycleJobErrorV2> {
        use sha2::Digest;
        if self.cancelled {
            return Err(LivingLifecycleJobErrorV2::Cancelled);
        }
        if let Some(fault) = &self.fault {
            return Err(LivingLifecycleJobErrorV2::Fault(fault.clone()));
        }
        if !self.ready || self.delivered {
            return Err(LivingLifecycleJobErrorV2::NotReady);
        }
        self.delivered = true;
        Ok(super::ids::LivingStateDigestV2(
            self.hasher.clone().finalize().into(),
        ))
    }
    fn byte_limit(&self) -> usize {
        #[cfg(test)]
        {
            self.byte_limit
        }
        #[cfg(not(test))]
        {
            super::wire::LIVING_MAX_ARCHIVE_BYTES_V2
        }
    }
    fn emit(&mut self, bytes: Vec<u8>, structural: bool) -> Result<(), LivingDeterministicFaultV2> {
        use sha2::Digest;
        debug_assert!(self.fragment.is_none());
        debug_assert!(bytes.len() <= 256);
        // Move the exact owner before fallible measurement so even arithmetic
        // failure retains its allocation for the retirement handoff.
        self.fragment = Some(bytes);
        let bytes = self.fragment.as_ref().expect("retained token");
        self.measured_bytes = self
            .measured_bytes
            .checked_add(bytes.len())
            .ok_or(LivingDeterministicFaultV2::Arithmetic)?;
        self.over_limit |= self.measured_bytes > self.byte_limit();
        if !self.over_limit {
            self.hasher.update(bytes);
            self.bytes += bytes.len();
            #[cfg(test)]
            self.encoded.extend_from_slice(bytes);
        }
        self.counts.tokens += 1;
        self.counts.structural_tokens += u64::from(structural);
        self.counts.max_encoded_token = self.counts.max_encoded_token.max(bytes.len());
        Ok(())
    }
    fn visit_next_hash(&mut self) -> Result<(), LivingDeterministicFaultV2> {
        #[cfg(test)]
        if !self.encoded_prepared {
            // Prepare an empty diagnostic buffer before any bytes are appended;
            // subsequent appends cannot relocate a populated buffer.
            self.encoded.reserve_exact(self.byte_limit);
            self.encoded_prepared = true;
            return Ok(());
        }
        if self.fragment.is_some() {
            drop(self.fragment.take());
            self.counts.storage_releases += 1;
            return Ok(());
        }
        if self.header < 6 {
            let bytes = match self.header {
                0 => b"{\"tick\":".to_vec(),
                1 => self
                    .normalized_tick
                    .unwrap_or(self.source.tick)
                    .0
                    .to_string()
                    .into_bytes(),
                2 => b",\"accepted_sequence\":".to_vec(),
                3 => self.source.accepted_sequence.to_string().into_bytes(),
                4 => b",\"branch_sequence\":".to_vec(),
                _ => self.source.branch_sequence.to_string().into_bytes(),
            };
            let structural = self.header.is_multiple_of(2);
            self.header += 1;
            return self.emit(bytes, structural);
        }
        if let Some((kind, index, token)) = self.child {
            // A separate comma visit precedes each subsequent child.
            if token == 0 && index > 0 {
                if !self.child_exists(kind, index) {
                    self.child = None;
                    return Ok(());
                }
                self.child = Some((kind, index, 1));
                return self.emit(b",".to_vec(), true);
            }
            let selected =
                self.child_token(kind, index, token.saturating_sub(usize::from(index > 0)))?;
            match selected {
                Some(HashToken::Bytes(bytes, structural)) => {
                    self.child = Some((kind, index, token + 1));
                    self.emit(bytes, structural)?;
                }
                Some(HashToken::Children(_)) => return Err(LivingDeterministicFaultV2::Invariant),
                None => {
                    if token == 0 {
                        self.child = None;
                    } else {
                        self.child = Some((kind, index + 1, 0));
                        self.counts.nested_records += 1;
                    }
                }
            }
            return Ok(());
        }
        if self.collection == 20 {
            if self.token == 0 {
                self.token = 1;
                return self.emit(b",\"counters\":".to_vec(), true);
            }
            if let Some(HashToken::Bytes(bytes, structural)) =
                state_hash_token(&self.source.counters, self.token - 1, None)?
            {
                self.token += 1;
                return self.emit(bytes, structural);
            }
            if self.opened {
                self.opened = false;
                return self.emit(b"}".to_vec(), true);
            }
            if self.over_limit {
                return Err(super::wire::LivingWireErrorV2::TooLarge {
                    actual: self.measured_bytes,
                    limit: self.byte_limit(),
                }
                .into());
            }
            self.ready = true;
            return Ok(());
        }
        if !self.opened {
            self.opened = true;
            return self.emit(
                format!(",\"{}\":[", STATE_COLLECTION_NAMES[self.collection]).into_bytes(),
                true,
            );
        }
        if self.token == 0 && self.index > 0 {
            if !self.record_exists() {
                return self.close_collection();
            }
            self.token = 1;
            return self.emit(b",".to_vec(), true);
        }
        let token = self.token.saturating_sub(usize::from(self.index > 0));
        match self.record_token(token)? {
            Some(HashToken::Bytes(bytes, structural)) => {
                self.token += 1;
                self.emit(bytes, structural)?;
            }
            Some(HashToken::Children(kind)) => {
                self.token += 1;
                self.child = Some((kind, 0, 0));
            }
            None if self.token == 0 => return self.close_collection(),
            None => {
                self.index += 1;
                self.token = 0;
                self.counts.records += 1;
            }
        }
        Ok(())
    }
    fn close_collection(&mut self) -> Result<(), LivingDeterministicFaultV2> {
        self.collection += 1;
        self.index = 0;
        self.token = 0;
        self.opened = self.collection == 20;
        self.emit(b"]".to_vec(), true)
    }
    fn record_exists(&self) -> bool {
        macro_rules! row {
            ($field:ident) => {
                self.index < self.source.$field.as_slice().len()
            };
        }
        match self.collection {
            0 => row!(systems),
            1 => row!(stars),
            2 => row!(worlds),
            3 => row!(lanes),
            4 => row!(civilizations),
            5 => row!(deposits),
            6 => row!(colonies),
            7 => row!(facilities),
            8 => row!(construction_jobs),
            9 => row!(hull_jobs),
            10 => row!(fleets),
            11 => row!(routes),
            12 => row!(shipments),
            13 => row!(relations),
            14 => row!(agreements),
            15 => row!(wars),
            16 => row!(observations),
            17 => row!(hazards),
            18 => row!(settlement_claims),
            19 => row!(occupations),
            _ => false,
        }
    }
    fn child_exists(&self, kind: HashChildren, index: usize) -> bool {
        macro_rules! child {
            ($rows:expr) => {
                $rows.get(index).is_some()
            };
        }
        match kind {
            HashChildren::HubReasons => child!(
                self.source.colonies.as_slice()[self.index]
                    .hub
                    .blocked_reasons
                    .as_slice()
            ),
            HashChildren::FacilityReasons => child!(
                self.source.facilities.as_slice()[self.index]
                    .blocked_reasons
                    .as_slice()
            ),
            HashChildren::FleetLanes => child!(
                &self.source.fleets.as_slice()[self.index]
                    .order
                    .as_ref()
                    .expect("fleet order field exists")
                    .lane_path
            ),
            HashChildren::FleetHulls => {
                child!(self.source.fleets.as_slice()[self.index].hulls.as_slice())
            }
            HashChildren::ObservationFacilities => child!(
                self.source.observations.as_slice()[self.index]
                    .facilities
                    .as_slice()
            ),
            HashChildren::ObservationHulls => child!(
                self.source.observations.as_slice()[self.index]
                    .stationed_hulls
                    .as_slice()
            ),
        }
    }
    fn record_token(&self, token: usize) -> Result<Option<HashToken>, LivingDeterministicFaultV2> {
        macro_rules! row {
            ($field:ident) => {
                self.source
                    .$field
                    .as_slice()
                    .get(self.index)
                    .map(|row| state_hash_token(row, token, Some(self.collection)))
                    .transpose()
                    .map(Option::flatten)
            };
        }
        match self.collection {
            0 => row!(systems),
            1 => row!(stars),
            2 => row!(worlds),
            3 => row!(lanes),
            4 => row!(civilizations),
            5 => row!(deposits),
            6 => row!(colonies),
            7 => row!(facilities),
            8 => row!(construction_jobs),
            9 => row!(hull_jobs),
            10 => row!(fleets),
            11 => row!(routes),
            12 => row!(shipments),
            13 => row!(relations),
            14 => row!(agreements),
            15 => row!(wars),
            16 => row!(observations),
            17 => row!(hazards),
            18 => row!(settlement_claims),
            19 => row!(occupations),
            _ => Ok(None),
        }
    }
    fn child_token(
        &self,
        kind: HashChildren,
        index: usize,
        token: usize,
    ) -> Result<Option<HashToken>, LivingDeterministicFaultV2> {
        macro_rules! child {
            ($rows:expr) => {
                $rows
                    .get(index)
                    .map(|row| state_hash_token(row, token, None))
                    .transpose()
                    .map(Option::flatten)
            };
        }
        match kind {
            HashChildren::HubReasons => child!(
                self.source.colonies.as_slice()[self.index]
                    .hub
                    .blocked_reasons
                    .as_slice()
            ),
            HashChildren::FacilityReasons => child!(
                self.source.facilities.as_slice()[self.index]
                    .blocked_reasons
                    .as_slice()
            ),
            HashChildren::FleetLanes => child!(
                &self.source.fleets.as_slice()[self.index]
                    .order
                    .as_ref()
                    .ok_or(LivingDeterministicFaultV2::Invariant)?
                    .lane_path
            ),
            HashChildren::FleetHulls => {
                child!(self.source.fleets.as_slice()[self.index].hulls.as_slice())
            }
            HashChildren::ObservationFacilities => child!(
                self.source.observations.as_slice()[self.index]
                    .facilities
                    .as_slice()
            ),
            HashChildren::ObservationHulls => child!(
                self.source.observations.as_slice()[self.index]
                    .stationed_hulls
                    .as_slice()
            ),
        }
    }
}

/// Opaque move-only handoff. Retain incomplete jobs and deferred tickets;
/// ordinary Drop is not a bounded retirement path. No state is published here.
pub(crate) struct LivingCopyHashRetirementJobV2 {
    candidate: Option<LivingStateRetirementJobV2>,
    source: LivingStateRetirementJobV2,
    fragment: Option<Vec<u8>>,
    #[cfg(test)]
    encoded: Option<Vec<u8>>,
    disposition: Option<LivingStateRetirementDispositionV2>,
    ready: bool,
    delivered: bool,
    last_poll_visits: usize,
    total_visits: u64,
    storage_releases: u64,
}
impl LivingCopyHashRetirementJobV2 {
    fn new(
        source: Arc<LivingGalaxyStateV2>,
        policy: LivingStateRetirementPolicyV2,
        candidate: Option<LivingStateRetirementJobV2>,
        fragment: Option<Vec<u8>>,
        #[cfg(test)] encoded: Vec<u8>,
    ) -> Self {
        Self {
            candidate,
            source: LivingStateRetirementJobV2::begin_shared(source, policy),
            fragment,
            #[cfg(test)]
            encoded: Some(encoded),
            disposition: None,
            ready: false,
            delivered: false,
            last_poll_visits: 0,
            total_visits: 0,
            storage_releases: 0,
        }
    }
    pub(crate) const fn last_poll_visits(&self) -> usize {
        self.last_poll_visits
    }
    pub(crate) const fn total_visits(&self) -> u64 {
        self.total_visits
    }
    pub(crate) fn poll(
        &mut self,
        budget: usize,
    ) -> Result<LivingLifecycleProgressV2, LivingLifecycleJobErrorV2> {
        self.last_poll_visits = 0;
        if budget == 0 || budget > LIVING_MAX_PHASE_POLL_VISITS_V2 {
            return Err(LivingLifecycleJobErrorV2::Budget);
        }
        while !self.ready && self.last_poll_visits < budget {
            self.last_poll_visits += 1;
            self.total_visits += 1;
            if self.fragment.is_some() {
                drop(self.fragment.take());
                self.storage_releases += 1;
                continue;
            }
            #[cfg(test)]
            if self.encoded.is_some() {
                drop(self.encoded.take());
                self.storage_releases += 1;
                continue;
            }
            if let Some(candidate) = &mut self.candidate {
                if candidate.poll(1)? == LivingLifecycleProgressV2::Ready {
                    let disposition = candidate.finish()?;
                    debug_assert!(matches!(
                        disposition,
                        LivingStateRetirementDispositionV2::Drained
                    ));
                    self.candidate = None;
                }
                continue;
            }
            if self.source.poll(1)? == LivingLifecycleProgressV2::Ready {
                self.disposition = Some(self.source.finish()?);
                self.ready = true;
            }
        }
        Ok(if self.ready {
            LivingLifecycleProgressV2::Ready
        } else {
            LivingLifecycleProgressV2::Pending
        })
    }
    pub(crate) fn finish(
        &mut self,
    ) -> Result<LivingStateRetirementDispositionV2, LivingLifecycleJobErrorV2> {
        if !self.ready || self.delivered {
            return Err(LivingLifecycleJobErrorV2::NotReady);
        }
        self.delivered = true;
        Ok(self
            .disposition
            .take()
            .expect("completed source disposition"))
    }
}

/// Resumable recipe reservation and application on private scalar inventory.
/// Resource amounts and emitted blocked reasons are separate visited records.
/// This is a component of phase five, not a complete production phase.
pub(crate) struct LivingRecipeJobV2 {
    delivered: bool,
    inventory: super::model::LivingInventoryV2,
    remaining_deposit: u64,
    deposit_units: u32,
    inputs: PhaseArc<Vec<(LivingResourceV2, u32)>>,
    outputs: PhaseArc<Vec<(LivingResourceV2, u32)>>,
    stage: u8,
    index: usize,
    pending_reason: Option<super::model::LivingBlockedReasonV2>,
    reasons: BTreeMap<(&'static str, &'static str), super::model::LivingBlockedReasonV2>,
    reason_output: LivingSortedVecV2<super::model::LivingBlockedReasonV2>,
    cancelled: bool,
    fault: Option<LivingDeterministicFaultV2>,
    last_poll_visits: usize,
    total_visits: u64,
}
/// Private recipe result. No inventory transfer occurs until its caller installs it.
pub struct LivingRecipeOutputV2 {
    /// Inventory after a successful unblocked recipe, or unchanged when blocked.
    pub inventory: super::model::LivingInventoryV2,
    /// Ore deposit remaining after the same recipe.
    pub remaining_deposit: u64,
    /// Canonical reasons for a blocked attempt.
    pub blocked_reasons: LivingSortedVecV2<super::model::LivingBlockedReasonV2>,
}
impl LivingRecipeJobV2 {
    /// Move the bounded catalog amount lists and capture scalar local resources.
    #[cfg(test)]
    pub fn begin(
        inventory: super::model::LivingInventoryV2,
        remaining_deposit: u64,
        deposit_units: u32,
        inputs: Vec<(LivingResourceV2, u32)>,
        outputs: Vec<(LivingResourceV2, u32)>,
    ) -> Self {
        Self::begin_shared(
            inventory,
            remaining_deposit,
            deposit_units,
            std::sync::Arc::new(inputs),
            std::sync::Arc::new(outputs),
        )
    }
    fn begin_shared(
        inventory: super::model::LivingInventoryV2,
        remaining_deposit: u64,
        deposit_units: u32,
        inputs: std::sync::Arc<Vec<(LivingResourceV2, u32)>>,
        outputs: std::sync::Arc<Vec<(LivingResourceV2, u32)>>,
    ) -> Self {
        Self {
            delivered: false,
            inventory,
            remaining_deposit,
            deposit_units,
            inputs: inputs.into(),
            outputs: outputs.into(),
            stage: 0,
            index: 0,
            pending_reason: None,
            reasons: BTreeMap::new(),
            reason_output: LivingSortedVecV2::default(),
            cancelled: false,
            fault: None,
            last_poll_visits: 0,
            total_visits: 0,
        }
    }
    /// Cancel without exposing a partially consumed inventory.
    #[cfg(test)]
    pub fn cancel(&mut self) {
        self.cancelled = true;
    }
    /// Visits used by the preceding poll.
    pub const fn last_poll_visits(&self) -> usize {
        self.last_poll_visits
    }
    /// Visits used since creation.
    #[cfg(test)]
    pub const fn total_visits(&self) -> u64 {
        self.total_visits
    }
    /// Visit individual input/output amounts, deposit records and blocked reasons.
    pub fn poll(
        &mut self,
        budget: usize,
    ) -> Result<LivingLifecycleProgressV2, LivingLifecycleJobErrorV2> {
        self.last_poll_visits = 0;
        if self.cancelled {
            return Err(LivingLifecycleJobErrorV2::Cancelled);
        }
        if budget == 0 || budget > LIVING_MAX_PHASE_POLL_VISITS_V2 {
            return Err(LivingLifecycleJobErrorV2::Budget);
        }
        if let Some(fault) = &self.fault {
            return Err(LivingLifecycleJobErrorV2::Fault(fault.clone()));
        }
        while self.stage < 7 && self.last_poll_visits < budget {
            match self.visit_recipe() {
                Ok(visited) => {
                    if visited {
                        self.last_poll_visits += 1;
                        self.total_visits += 1;
                    }
                }
                Err(LivingLifecycleJobErrorV2::Fault(fault)) => {
                    self.last_poll_visits += 1;
                    self.total_visits += 1;
                    self.fault = Some(fault.clone());
                    return Err(LivingLifecycleJobErrorV2::Fault(fault));
                }
                Err(error) => return Err(error),
            }
        }
        Ok(if self.stage == 7 {
            LivingLifecycleProgressV2::Ready
        } else {
            LivingLifecycleProgressV2::Pending
        })
    }
    /// Consume the complete result. Interrupted partial payment never escapes.
    #[allow(dead_code)]
    pub fn finish(mut self) -> Result<LivingRecipeOutputV2, LivingLifecycleJobErrorV2> {
        self.take_ready_result()
    }
    /// Borrowed one-time extraction; rejected calls preserve every owner.
    pub fn take_ready_result(&mut self) -> Result<LivingRecipeOutputV2, LivingLifecycleJobErrorV2> {
        if self.cancelled {
            return Err(LivingLifecycleJobErrorV2::Cancelled);
        }
        if let Some(fault) = self.fault.clone() {
            return Err(LivingLifecycleJobErrorV2::Fault(fault));
        }
        if self.stage != 7 {
            return Err(LivingLifecycleJobErrorV2::NotReady);
        }
        if self.delivered {
            return Err(LivingLifecycleJobErrorV2::NotReady);
        }
        self.delivered = true;
        Ok(LivingRecipeOutputV2 {
            inventory: self.inventory,
            remaining_deposit: self.remaining_deposit,
            blocked_reasons: std::mem::take(&mut self.reason_output),
        })
    }
    fn visit_recipe(&mut self) -> Result<bool, LivingLifecycleJobErrorV2> {
        use super::model::LivingBlockedReasonV2;
        if let Some(reason) = self.pending_reason.take() {
            self.reasons.insert(reason.living_key(), reason);
            return Ok(true);
        }
        match self.stage {
            0 => {
                if let Some((resource, required)) = self.inputs.get(self.index).copied() {
                    let available = recipe_quantity(self.inventory, resource);
                    if available < required {
                        self.pending_reason = Some(LivingBlockedReasonV2::MissingInput {
                            resource,
                            required,
                            available,
                        });
                    }
                    self.index += 1;
                    return Ok(true);
                }
            }
            1 => {
                self.stage = 2;
                self.index = 0;
                if self.deposit_units > 0 {
                    if self.remaining_deposit < u64::from(self.deposit_units) {
                        self.pending_reason = Some(LivingBlockedReasonV2::MissingDeposit {
                            required: self.deposit_units,
                            available: u32::try_from(self.remaining_deposit).unwrap_or(u32::MAX),
                        });
                    }
                    return Ok(true);
                }
                return Ok(false);
            }
            2 => {
                if let Some((resource, quantity)) = self.outputs.get(self.index).copied() {
                    if recipe_quantity(self.inventory, resource).saturating_add(quantity)
                        > LIVING_RESOURCE_STORAGE_LIMIT_V2
                    {
                        self.pending_reason = Some(LivingBlockedReasonV2::OutputFull { resource });
                    }
                    self.index += 1;
                    return Ok(true);
                }
                if !self.reasons.is_empty() {
                    self.stage = 6;
                    self.index = 0;
                    return Ok(false);
                }
            }
            3 => {
                if let Some((resource, required)) = self.inputs.get(self.index).copied() {
                    let current = recipe_quantity(self.inventory, resource);
                    // Catalog amounts are unique per resource; all were reserved above.
                    let remaining =
                        current
                            .checked_sub(required)
                            .ok_or(LivingLifecycleJobErrorV2::Fault(
                                LivingDeterministicFaultV2::Invariant,
                            ))?;
                    recipe_set_quantity(&mut self.inventory, resource, remaining);
                    self.index += 1;
                    return Ok(true);
                }
            }
            4 => {
                self.stage = 5;
                self.index = 0;
                if self.deposit_units > 0 {
                    self.remaining_deposit = self
                        .remaining_deposit
                        .checked_sub(u64::from(self.deposit_units))
                        .ok_or(LivingLifecycleJobErrorV2::Fault(
                            LivingDeterministicFaultV2::Invariant,
                        ))?;
                    return Ok(true);
                }
                return Ok(false);
            }
            5 => {
                if let Some((resource, quantity)) = self.outputs.get(self.index).copied() {
                    let current = recipe_quantity(self.inventory, resource);
                    let produced = current
                        .checked_add(quantity)
                        .filter(|value| *value <= LIVING_RESOURCE_STORAGE_LIMIT_V2)
                        .ok_or(LivingLifecycleJobErrorV2::Fault(
                            LivingDeterministicFaultV2::Invariant,
                        ))?;
                    recipe_set_quantity(&mut self.inventory, resource, produced);
                    self.index += 1;
                    return Ok(true);
                }
            }
            6 => {
                if let Some((_, reason)) = self.reasons.pop_first() {
                    self.reason_output
                        .push_sorted(reason)
                        .map_err(|error| LivingLifecycleJobErrorV2::Fault(error.into()))?;
                    return Ok(true);
                }
            }
            _ => {}
        }
        self.stage += 1;
        self.index = 0;
        Ok(false)
    }
}
fn recipe_quantity(inventory: super::model::LivingInventoryV2, resource: LivingResourceV2) -> u32 {
    match resource {
        LivingResourceV2::Energy => inventory.energy,
        LivingResourceV2::Ore => inventory.ore,
        LivingResourceV2::Alloy => inventory.alloy,
    }
}
fn recipe_set_quantity(
    inventory: &mut super::model::LivingInventoryV2,
    resource: LivingResourceV2,
    quantity: u32,
) {
    match resource {
        LivingResourceV2::Energy => inventory.energy = quantity,
        LivingResourceV2::Ore => inventory.ore = quantity,
        LivingResourceV2::Alloy => inventory.alloy = quantity,
    }
}

#[derive(Clone, Copy, Eq, PartialEq)]
enum RepairCursor {
    Worlds,
    Definitions,
    Fleet,
    Hull,
    Energy,
    Update,
    HullOutput,
    FleetOutput,
    RetireIndices,
    Ready,
}

/// Resumable hull repairs on an unpublished candidate. Fleet orders and their
/// paths are moved unchanged; each hull read and output is a separate visit.
pub struct LivingRepairJobV2<'a> {
    delivered: bool,
    catalog: PhaseCatalogSlot<'a>,
    state: LivingGalaxyStateV2,
    tick: LivingTickV2,
    source: IntoIter<super::model::LivingFleetV2>,
    world_index: BTreeMap<LivingEntityIdV2, (usize, Option<LivingEntityIdV2>)>,
    hit_points: BTreeMap<String, u32>,
    index: usize,
    cursor: RepairCursor,
    fleet: Option<super::model::LivingFleetV2>,
    hull_source: IntoIter<super::model::LivingHullV2>,
    hull_output: LivingSortedVecV2<super::model::LivingHullV2>,
    hull: Option<super::model::LivingHullV2>,
    world_position: Option<usize>,
    maximum: u32,
    paid: bool,
    cancelled: bool,
    fault: Option<LivingDeterministicFaultV2>,
    last_poll_visits: usize,
    total_visits: u64,
}
impl<'a> LivingRepairJobV2<'a> {
    /// Capture owned inputs without traversing fleets, hulls, or catalog rows.
    pub fn begin(
        catalog: &'a super::catalog::ValidatedLivingCatalogPackV2,
        state: LivingGalaxyStateV2,
        tick: LivingTickV2,
    ) -> Self {
        Self::begin_with_catalog(LivingPhaseCatalogV2::Borrowed(catalog), state, tick)
    }
    /// Capture an immutable catalog owner for a resident job without cloning rows.
    pub fn begin_owned(
        catalog: std::sync::Arc<super::catalog::ValidatedLivingCatalogPackV2>,
        state: LivingGalaxyStateV2,
        tick: LivingTickV2,
    ) -> LivingRepairJobV2<'static> {
        LivingRepairJobV2::begin_with_catalog(LivingPhaseCatalogV2::Owned(catalog), state, tick)
    }
    pub(crate) fn begin_with_catalog(
        catalog: LivingPhaseCatalogV2<'a>,
        mut state: LivingGalaxyStateV2,
        tick: LivingTickV2,
    ) -> Self {
        let source = std::mem::take(&mut state.fleets).into_vec().into_iter();
        Self {
            delivered: false,
            catalog: PhaseCatalogSlot(Some(catalog)),
            state,
            tick,
            source,
            world_index: BTreeMap::new(),
            hit_points: BTreeMap::new(),
            index: 0,
            cursor: RepairCursor::Worlds,
            fleet: None,
            hull_source: Vec::new().into_iter(),
            hull_output: LivingSortedVecV2::default(),
            hull: None,
            world_position: None,
            maximum: 1,
            paid: false,
            cancelled: false,
            fault: None,
            last_poll_visits: 0,
            total_visits: 0,
        }
    }
    /// Discard the unpublished repair attempt.
    pub fn cancel(&mut self) {
        self.cancelled = true;
    }
    /// Source, metadata, inventory and output visits in the preceding poll.
    pub const fn last_poll_visits(&self) -> usize {
        self.last_poll_visits
    }
    /// Actual visits made by this candidate.
    pub const fn total_visits(&self) -> u64 {
        self.total_visits
    }
    /// Repair at most the requested number of individual records.
    pub fn poll(
        &mut self,
        budget: usize,
    ) -> Result<LivingLifecycleProgressV2, LivingLifecycleJobErrorV2> {
        self.last_poll_visits = 0;
        if self.cancelled {
            return Err(LivingLifecycleJobErrorV2::Cancelled);
        }
        if budget == 0 || budget > LIVING_MAX_PHASE_POLL_VISITS_V2 {
            return Err(LivingLifecycleJobErrorV2::Budget);
        }
        if let Some(fault) = &self.fault {
            return Err(LivingLifecycleJobErrorV2::Fault(fault.clone()));
        }
        while self.cursor != RepairCursor::Ready && self.last_poll_visits < budget {
            match self.visit_repair() {
                Ok(visited) => {
                    if visited {
                        self.last_poll_visits += 1;
                        self.total_visits += 1;
                    }
                }
                Err(fault) => {
                    self.last_poll_visits += 1;
                    self.total_visits += 1;
                    self.fault = Some(fault.clone());
                    return Err(LivingLifecycleJobErrorV2::Fault(fault));
                }
            }
        }
        Ok(if self.cursor == RepairCursor::Ready {
            LivingLifecycleProgressV2::Ready
        } else {
            LivingLifecycleProgressV2::Pending
        })
    }
    /// Publish only the complete private repair result to its phase owner.
    pub fn finish(mut self) -> Result<LivingGalaxyStateV2, LivingLifecycleJobErrorV2> {
        self.take_ready_result()
    }
    /// Borrowed one-time extraction; rejected calls preserve every owner.
    pub fn take_ready_result(&mut self) -> Result<LivingGalaxyStateV2, LivingLifecycleJobErrorV2> {
        if self.cancelled {
            return Err(LivingLifecycleJobErrorV2::Cancelled);
        }
        if let Some(fault) = self.fault.clone() {
            return Err(LivingLifecycleJobErrorV2::Fault(fault));
        }
        if self.cursor != RepairCursor::Ready {
            return Err(LivingLifecycleJobErrorV2::NotReady);
        }
        if self.delivered {
            return Err(LivingLifecycleJobErrorV2::NotReady);
        }
        self.delivered = true;
        Ok(std::mem::take(&mut self.state))
    }
    fn visit_repair(&mut self) -> Result<bool, LivingDeterministicFaultV2> {
        use super::model::LivingFleetLocationV2;
        match self.cursor {
            RepairCursor::Worlds => {
                if let Some(row) = self.state.worlds.as_slice().get(self.index) {
                    self.world_index.insert(row.id, (self.index, row.owner));
                    self.index += 1;
                    return Ok(true);
                }
                self.index = 0;
                self.cursor = RepairCursor::Definitions;
            }
            RepairCursor::Definitions => {
                if let Some(row) = self.catalog.hulls().get(self.index) {
                    self.hit_points
                        .insert(row.id.as_str().to_owned(), row.hit_points);
                    self.index += 1;
                    return Ok(true);
                }
                self.cursor = RepairCursor::Fleet;
            }
            RepairCursor::Fleet => {
                if let Some(mut fleet) = self.source.next() {
                    self.world_position = match fleet.location {
                        LivingFleetLocationV2::Docked { world } => self
                            .world_index
                            .get(&world)
                            .filter(|(_, owner)| *owner == Some(fleet.owner))
                            .map(|(position, _)| *position),
                        LivingFleetLocationV2::Travelling { .. } => None,
                    };
                    self.hull_source = std::mem::take(&mut fleet.hulls).into_vec().into_iter();
                    self.fleet = Some(fleet);
                    self.cursor = RepairCursor::Hull;
                    return Ok(true);
                }
                self.cursor = RepairCursor::RetireIndices;
            }
            RepairCursor::Hull => {
                if let Some(hull) = self.hull_source.next() {
                    let due = self.world_position.is_some()
                        && hull.next_repair.is_some_and(|due| due.0 <= self.tick.0);
                    self.maximum = self
                        .hit_points
                        .get(hull.definition.as_str())
                        .copied()
                        .unwrap_or(1);
                    self.paid = false;
                    self.cursor = if due {
                        if hull.hit_points >= self.maximum {
                            RepairCursor::Update
                        } else {
                            RepairCursor::Energy
                        }
                    } else {
                        RepairCursor::HullOutput
                    };
                    self.hull = Some(hull);
                    return Ok(true);
                }
                self.cursor = RepairCursor::FleetOutput;
            }
            RepairCursor::Energy => {
                let position = self
                    .world_position
                    .ok_or(LivingDeterministicFaultV2::Invariant)?;
                let world = self
                    .state
                    .worlds
                    .record_at_mut(position)
                    .ok_or(LivingDeterministicFaultV2::Invariant)?;
                if world.inventory.energy > 0 {
                    world.inventory.energy -= 1;
                    self.paid = true;
                }
                self.cursor = RepairCursor::Update;
                return Ok(true);
            }
            RepairCursor::Update => {
                let hull = self
                    .hull
                    .as_mut()
                    .ok_or(LivingDeterministicFaultV2::Invariant)?;
                if hull.hit_points >= self.maximum {
                    hull.next_repair = None;
                } else {
                    if self.paid {
                        hull.hit_points = hull.hit_points.saturating_add(1).min(self.maximum);
                    }
                    hull.next_repair = if hull.hit_points >= self.maximum {
                        None
                    } else {
                        Some(LivingTickV2(
                            self.tick
                                .0
                                .checked_add(10)
                                .ok_or(LivingDeterministicFaultV2::Arithmetic)?,
                        ))
                    };
                }
                self.cursor = RepairCursor::HullOutput;
                return Ok(true);
            }
            RepairCursor::HullOutput => {
                let hull = self
                    .hull
                    .take()
                    .ok_or(LivingDeterministicFaultV2::Invariant)?;
                self.hull_output.push_sorted(hull)?;
                self.cursor = RepairCursor::Hull;
                return Ok(true);
            }
            RepairCursor::FleetOutput => {
                if let Some(fleet) = self.fleet.as_ref() {
                    ensure_retained_sorted(&self.state.fleets, fleet)?;
                }
                let mut fleet = self
                    .fleet
                    .take()
                    .ok_or(LivingDeterministicFaultV2::Invariant)?;
                fleet.hulls = std::mem::take(&mut self.hull_output);
                self.state.fleets.push_sorted(fleet)?;
                self.cursor = RepairCursor::Fleet;
                return Ok(true);
            }
            RepairCursor::RetireIndices => {
                if self.world_index.pop_first().is_some() || self.hit_points.pop_first().is_some() {
                    return Ok(true);
                }
                self.cursor = RepairCursor::Ready;
            }
            RepairCursor::Ready => {}
        }
        Ok(false)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::living::model::LivingHazardV2;
    use crate::living::{LivingSlugV2, builtin_living_sandbox_v2};

    #[test]
    fn recipe_cursor_latches_failed_payment_and_never_publishes_partial_inventory() {
        let original = super::super::model::LivingInventoryV2 {
            energy: 3,
            ore: 0,
            alloy: 0,
        };
        // Deliberately bypass the validated catalog boundary inside this unit test.
        // A duplicate payment must fault rather than silently producing free output.
        let mut job = LivingRecipeJobV2::begin(
            original,
            0,
            0,
            vec![(LivingResourceV2::Energy, 2), (LivingResourceV2::Energy, 2)],
            vec![(LivingResourceV2::Ore, 1)],
        );
        loop {
            match job.poll(1) {
                Ok(LivingLifecycleProgressV2::Pending) => {}
                result => {
                    assert!(matches!(result, Err(LivingLifecycleJobErrorV2::Fault(_))));
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
        assert_eq!(original.energy, 3);
        assert_eq!(original.ore, 0);
    }

    #[test]
    fn recipe_cursor_slices_amounts_and_preserves_atomic_resource_reservations() {
        use super::super::model::LivingInventoryV2;
        let source = LivingInventoryV2 {
            energy: 10,
            ore: 5,
            alloy: 20,
        };
        let run = |budget| {
            let mut job = LivingRecipeJobV2::begin(
                source,
                9,
                2,
                vec![(LivingResourceV2::Energy, 4), (LivingResourceV2::Ore, 4)],
                vec![(LivingResourceV2::Alloy, 1)],
            );
            while job.poll(budget).unwrap() != LivingLifecycleProgressV2::Ready {
                assert!(job.last_poll_visits() <= budget);
            }
            let visits = job.total_visits();
            let result = job.finish().unwrap();
            (
                result.inventory,
                result.remaining_deposit,
                result.blocked_reasons,
                visits,
            )
        };
        let expected = run(1);
        assert_eq!(
            expected.0,
            LivingInventoryV2 {
                energy: 6,
                ore: 1,
                alloy: 21
            }
        );
        assert_eq!(expected.1, 7);
        assert!(expected.2.as_slice().is_empty());
        assert_eq!(run(64), expected);
        assert_eq!(run(1_024), expected);
        let mut cancelled = LivingRecipeJobV2::begin(
            source,
            0,
            0,
            vec![(LivingResourceV2::Energy, 4), (LivingResourceV2::Ore, 4)],
            vec![(LivingResourceV2::Alloy, 1)],
        );
        assert_eq!(cancelled.poll(4), Ok(LivingLifecycleProgressV2::Pending));
        cancelled.cancel();
        assert!(matches!(
            cancelled.finish(),
            Err(LivingLifecycleJobErrorV2::Cancelled)
        ));
        assert_eq!(source.energy, 10);
    }

    #[test]
    fn recipe_cursor_reports_all_scarcity_reasons_without_consuming_anything() {
        use super::super::model::{LivingBlockedReasonV2, LivingInventoryV2};
        let source = LivingInventoryV2 {
            energy: 0,
            ore: 1,
            alloy: LIVING_RESOURCE_STORAGE_LIMIT_V2,
        };
        let mut job = LivingRecipeJobV2::begin(
            source,
            0,
            2,
            vec![(LivingResourceV2::Energy, 4), (LivingResourceV2::Ore, 4)],
            vec![(LivingResourceV2::Alloy, 1)],
        );
        assert_eq!(job.poll(0), Err(LivingLifecycleJobErrorV2::Budget));
        assert_eq!(job.total_visits(), 0);
        while job.poll(1).unwrap() != LivingLifecycleProgressV2::Ready {
            assert_eq!(job.last_poll_visits(), 1);
        }
        let result = job.finish().unwrap();
        assert_eq!(result.inventory, source);
        assert_eq!(result.remaining_deposit, 0);
        assert_eq!(
            result.blocked_reasons.as_slice(),
            &[
                LivingBlockedReasonV2::MissingDeposit {
                    required: 2,
                    available: 0
                },
                LivingBlockedReasonV2::MissingInput {
                    resource: LivingResourceV2::Energy,
                    required: 4,
                    available: 0
                },
                LivingBlockedReasonV2::MissingInput {
                    resource: LivingResourceV2::Ore,
                    required: 4,
                    available: 1
                },
                LivingBlockedReasonV2::OutputFull {
                    resource: LivingResourceV2::Alloy
                },
            ]
        );
    }

    #[test]
    fn incremental_hash_matches_exact_canonical_nested_bytes_and_digest() {
        let mut state = arrival_fixture();
        let id = state.fleets.as_slice()[0].id;
        let lane_ids = state.lanes.as_slice().iter().map(|lane| lane.id).collect();
        state
            .fleets
            .find_mut(&id)
            .unwrap()
            .order
            .as_mut()
            .unwrap()
            .lane_path = lane_ids;
        let colony_world = state.colonies.as_slice()[0].world;
        state
            .colonies
            .find_mut(&colony_world)
            .unwrap()
            .hub
            .blocked_reasons = LivingSortedVecV2::new(vec![
            super::super::model::LivingBlockedReasonV2::MissingInput {
                resource: LivingResourceV2::Energy,
                required: 2,
                available: 0,
            },
        ])
        .unwrap();
        if let Some(id) = state.facilities.as_slice().first().map(|row| row.id) {
            let facility = state.facilities.find_mut(&id).unwrap();
            facility.disabled = true;
            facility.blocked_reasons =
                LivingSortedVecV2::new(vec![super::super::model::LivingBlockedReasonV2::Disabled])
                    .unwrap();
        }
        let mut observations = state.observations.as_slice().to_vec();
        for observation in &mut observations {
            observation.operational_defense_batteries = Some(0);
        }
        state.observations = LivingSortedVecV2::new(observations).unwrap();
        let source = Arc::new(state);
        let expected = source.canonical_bytes().unwrap();
        let expected_digest = source.digest().unwrap();
        let mut totals = Vec::new();
        for budget in [1, 64, 1_024] {
            let mut job = LivingStateHashJobV2::begin(Arc::clone(&source));
            assert_eq!(job.encoded_bytes(), 0);
            while job.poll(budget).unwrap() != LivingLifecycleProgressV2::Ready {
                assert!(job.last_poll_visits() <= budget);
            }
            assert_eq!(job.encoded, expected);
            assert_eq!(job.encoded_bytes(), expected.len());
            totals.push(job.total_visits());
            assert_eq!(job.finish().unwrap(), expected_digest);
        }
        assert_eq!(totals, vec![totals[0]; 3]);
        let mut cancelled = LivingStateHashJobV2::begin(source);
        assert_eq!(cancelled.poll(0), Err(LivingLifecycleJobErrorV2::Budget));
        assert_eq!(cancelled.poll(1), Ok(LivingLifecycleProgressV2::Pending));
        cancelled.cancel();
        assert_eq!(
            cancelled.finish(),
            Err(LivingLifecycleJobErrorV2::Cancelled)
        );
        let mut empty = LivingStateHashJobV2::begin(Arc::new(LivingGalaxyStateV2::default()));
        assert_eq!(empty.poll(1), Ok(LivingLifecycleProgressV2::Pending));
        while empty.poll(1).unwrap() != LivingLifecycleProgressV2::Ready {}
        assert_eq!(
            empty.encoded,
            LivingGalaxyStateV2::default().canonical_bytes().unwrap()
        );
        assert_eq!(
            empty.finish().unwrap(),
            LivingGalaxyStateV2::default().digest().unwrap()
        );
    }

    #[test]
    fn incremental_state_copy_preserves_nested_records_and_exact_wire_bytes() {
        let mut state = arrival_fixture();
        let fleet_id = state.fleets.as_slice()[0].id;
        let lanes: Vec<_> = state.lanes.as_slice().iter().map(|lane| lane.id).collect();
        state
            .fleets
            .find_mut(&fleet_id)
            .unwrap()
            .order
            .as_mut()
            .unwrap()
            .lane_path = lanes;
        let source = Arc::new(state);
        let bytes = source.canonical_bytes().unwrap();
        let mut counts = Vec::new();
        for budget in [1, 64, 1_024] {
            let mut job = LivingStateCopyJobV2::begin(Arc::clone(&source));
            assert_eq!(job.total_visits(), 0);
            while job.poll(budget).unwrap() != LivingLifecycleProgressV2::Ready {
                assert!(job.last_poll_visits() <= budget);
            }
            counts.push(job.total_visits());
            let copy = job.finish().unwrap();
            assert_eq!(copy, *source);
            assert_eq!(copy.canonical_bytes().unwrap(), bytes);
            assert_eq!(copy.digest().unwrap(), source.digest().unwrap());
        }
        assert!(counts[0] > source.fleets.as_slice().len() as u64);
        assert_eq!(counts, vec![counts[0]; 3]);
        assert_eq!(source.canonical_bytes().unwrap(), bytes);
    }

    #[test]
    fn incremental_copy_budget_and_cancellation_keep_source_publication_immutable() {
        let mut state = arrival_fixture();
        let prototype = state.shipments.as_slice()[0].clone();
        state.shipments = LivingSortedVecV2::new(
            (0_u16..4_096)
                .map(|index| {
                    let mut row = prototype.clone();
                    row.id.0[14..].copy_from_slice(&index.to_be_bytes());
                    row
                })
                .collect(),
        )
        .unwrap();
        let source = Arc::new(state);
        let expected = source.canonical_bytes().unwrap();
        let mut job = LivingStateCopyJobV2::begin(Arc::clone(&source));
        assert_eq!(job.poll(0), Err(LivingLifecycleJobErrorV2::Budget));
        assert_eq!(job.poll(1_025), Err(LivingLifecycleJobErrorV2::Budget));
        assert_eq!(job.total_visits(), 0);
        assert_eq!(job.poll(1_024), Ok(LivingLifecycleProgressV2::Pending));
        assert_eq!(job.last_poll_visits(), 1_024);
        job.cancel();
        assert_eq!(job.poll(1), Err(LivingLifecycleJobErrorV2::Cancelled));
        assert_eq!(job.finish(), Err(LivingLifecycleJobErrorV2::Cancelled));
        assert_eq!(source.canonical_bytes().unwrap(), expected);
        assert_eq!(
            LivingStateCopyJobV2::begin(source).finish(),
            Err(LivingLifecycleJobErrorV2::NotReady)
        );
    }

    fn fixture() -> LivingGalaxyStateV2 {
        let mut state = builtin_living_sandbox_v2().unwrap().state().clone();
        let left = state.civilizations.as_slice()[0].id;
        let right = state.civilizations.as_slice()[1].id;
        state.agreements = LivingSortedVecV2::new(vec![
            LivingAgreementV2 {
                kind: LivingAgreementKindV2::Trade,
                participant_a: left,
                participant_b: right,
                start_tick: LivingTickV2(0),
                end_tick: LivingTickV2(10),
            },
            LivingAgreementV2 {
                kind: LivingAgreementKindV2::Truce,
                participant_a: left,
                participant_b: right,
                start_tick: LivingTickV2(0),
                end_tick: LivingTickV2(1_000),
            },
        ])
        .unwrap();
        state.wars = LivingSortedVecV2::new(vec![LivingWarV2 {
            participant_a: left,
            participant_b: right,
            declarer: left,
            start_tick: LivingTickV2(0),
            end_tick: LivingTickV2(10),
        }])
        .unwrap();
        let world = state
            .worlds
            .as_slice()
            .iter()
            .find(|world| world.owner == Some(left))
            .unwrap()
            .id;
        state.occupations = LivingSortedVecV2::new(vec![LivingOccupationV2 {
            world,
            claimant: right,
            progress_ticks: 4,
        }])
        .unwrap();
        state.hazards = LivingSortedVecV2::new(vec![
            LivingHazardV2 {
                id: LivingEntityIdV2([240; 16]),
                definition: LivingSlugV2::new("lane_storm").unwrap(),
                lane: LivingEntityIdV2([239; 16]),
                start_tick: LivingTickV2(10),
                end_tick: LivingTickV2(12),
            },
            LivingHazardV2 {
                id: LivingEntityIdV2([241; 16]),
                definition: LivingSlugV2::new("lane_storm").unwrap(),
                lane: LivingEntityIdV2([239; 16]),
                start_tick: LivingTickV2(1),
                end_tick: LivingTickV2(10),
            },
        ])
        .unwrap();
        state
    }

    fn run(
        state: LivingGalaxyStateV2,
        budget: usize,
        tick: u64,
    ) -> (LivingGalaxyStateV2, LivingPendingEventsV2, u64) {
        let mut job =
            LivingLifecycleJobV2::begin(state, LivingPendingEventsV2::new(), LivingTickV2(tick));
        assert_eq!(job.total_visits(), 0, "begin must not scan the owned state");
        loop {
            let progress = job.poll(budget).unwrap();
            assert!(job.last_poll_visits() <= budget);
            if progress == LivingLifecycleProgressV2::Ready {
                break;
            }
            assert!(job.last_poll_visits() > 0);
        }
        let visits = job.total_visits();
        let (state, events) = job.finish().unwrap();
        (state, events, visits)
    }

    #[test]
    fn lifecycle_budget_one_matches_large_polls_and_preserves_event_order() {
        let original = fixture();
        let a = run(original.clone(), 1, 10);
        let b = run(original.clone(), 64, 10);
        let c = run(original.clone(), 1_024, 10);
        assert_eq!(a, b);
        assert_eq!(a, c);
        // Two input agreements, one war, one output truce, every owner record,
        // one occupation, two independent passes over two hazards, plus each
        // owner index and the ended pair reclaimed before Ready.
        assert_eq!(a.2, 10 + 2 * original.worlds.as_slice().len() as u64);
        assert_eq!(a.0.agreements.as_slice().len(), 1);
        assert_eq!(a.0.agreements.as_slice()[0].start_tick, LivingTickV2(10));
        assert_eq!(a.0.agreements.as_slice()[0].end_tick, LivingTickV2(610));
        assert!(a.0.wars.as_slice().is_empty());
        assert!(a.0.occupations.as_slice().is_empty());
        assert_eq!(
            a.0.hazards, original.hazards,
            "lifecycle keeps hazard witnesses"
        );
        assert_eq!(
            a.0.fleets, original.fleets,
            "nested hull state moves untouched"
        );
        assert_eq!(a.0.observations, original.observations);
        assert_eq!(
            a.1.payloads()
                .iter()
                .map(|event| event.kind)
                .collect::<Vec<_>>(),
            vec![
                LivingEventKindV2::AgreementExpired,
                LivingEventKindV2::WarEnded,
                LivingEventKindV2::HazardStarted,
                LivingEventKindV2::HazardEnded,
            ]
        );
    }

    #[test]
    fn lifecycle_start_and_end_are_exact_half_open_boundaries() {
        let original = fixture();
        let before = run(original.clone(), 1, 9);
        assert_eq!(before.0.agreements, original.agreements);
        assert_eq!(before.0.wars, original.wars);
        assert_eq!(before.0.occupations, original.occupations);
        assert!(before.1.payloads().is_empty());
        let after = run(original, 1, 11);
        assert_eq!(
            after
                .1
                .payloads()
                .iter()
                .map(|event| event.kind)
                .collect::<Vec<_>>(),
            vec![
                LivingEventKindV2::AgreementExpired,
                LivingEventKindV2::WarEnded,
            ]
        );
    }

    #[test]
    fn lifecycle_cancel_and_bad_budget_do_not_publish_or_advance() {
        let original = fixture();
        let mut job = LivingLifecycleJobV2::begin(
            original.clone(),
            LivingPendingEventsV2::new(),
            LivingTickV2(10),
        );
        assert_eq!(job.poll(0), Err(LivingLifecycleJobErrorV2::Budget));
        assert_eq!(job.poll(1_025), Err(LivingLifecycleJobErrorV2::Budget));
        assert_eq!(job.total_visits(), 0);
        assert_eq!(job.poll(1), Ok(LivingLifecycleProgressV2::Pending));
        assert_eq!(job.total_visits(), 1);
        job.cancel();
        assert_eq!(job.poll(1), Err(LivingLifecycleJobErrorV2::Cancelled));
        assert_eq!(job.total_visits(), 1);
        assert_eq!(job.finish(), Err(LivingLifecycleJobErrorV2::Cancelled));
        assert_eq!(original.agreements.as_slice().len(), 2);
        assert_eq!(original.wars.as_slice().len(), 1);
        assert_eq!(original.occupations.as_slice().len(), 1);
    }

    #[test]
    fn lifecycle_arithmetic_fault_is_latched_and_candidate_cannot_escape() {
        let mut original = fixture();
        let mut wars = original.wars.clone().into_vec();
        wars[0].end_tick = LivingTickV2(u64::MAX);
        original.wars = LivingSortedVecV2::new(wars).unwrap();
        let mut job = LivingLifecycleJobV2::begin(
            original.clone(),
            LivingPendingEventsV2::new(),
            LivingTickV2(u64::MAX),
        );
        let error = LivingLifecycleJobErrorV2::Fault(LivingDeterministicFaultV2::Arithmetic);
        assert_eq!(job.poll(1_024), Err(error.clone()));
        let visits = job.total_visits();
        assert_eq!(visits, 3);
        assert_eq!(job.poll(1), Err(error.clone()));
        assert_eq!(job.total_visits(), visits);
        assert_eq!(job.last_poll_visits(), 0);
        assert_eq!(job.finish(), Err(error));
        assert_eq!(original.wars.as_slice()[0].end_tick, LivingTickV2(u64::MAX));
    }
    #[test]
    fn lifecycle_large_owner_and_occupation_scans_yield_at_the_record_budget() {
        let mut source = fixture();
        let prototype = source.worlds.as_slice()[0].clone();
        let claimant = source.wars.as_slice()[0].participant_b;
        let owner = source.wars.as_slice()[0].participant_a;
        let mut worlds = Vec::new();
        let mut occupations = Vec::new();
        for index in 0_u16..512 {
            let mut row = prototype.clone();
            let mut id = [100; 16];
            id[14..].copy_from_slice(&index.to_be_bytes());
            row.id = LivingEntityIdV2(id);
            row.owner = Some(owner);
            occupations.push(LivingOccupationV2 {
                world: row.id,
                claimant,
                progress_ticks: 1,
            });
            worlds.push(row);
        }
        // Exercise the phase's two largest collections at their declared caps.
        // This phase-only fixture does not claim genesis validation or UI proof.
        source.worlds = LivingSortedVecV2::new(worlds).unwrap();
        source.occupations = LivingSortedVecV2::new(occupations).unwrap();
        let expected = run(source.clone(), 1, 10);
        let mut job =
            LivingLifecycleJobV2::begin(source, LivingPendingEventsV2::new(), LivingTickV2(10));
        assert_eq!(job.poll(1_024), Ok(LivingLifecycleProgressV2::Pending));
        assert_eq!(job.last_poll_visits(), 1_024);
        assert_eq!(job.total_visits(), 1_024);
        assert_eq!(job.poll(1_024), Ok(LivingLifecycleProgressV2::Ready));
        // Source/output visits leave 8 records after the first poll, then
        // retirement visits all 512 owner entries and the one ended pair.
        assert_eq!(job.last_poll_visits(), 8 + 512 + 1);
        assert_eq!(job.total_visits(), expected.2);
        let result = job.finish().unwrap();
        assert_eq!(result, (expected.0, expected.1));
        assert!(result.0.occupations.as_slice().is_empty());
    }
    fn arrival_fixture() -> LivingGalaxyStateV2 {
        let mut state = builtin_living_sandbox_v2().unwrap().state().clone();
        let first = state.fleets.as_slice()[0].id;
        let fleet = state.fleets.find_mut(&first).unwrap();
        let home = state.worlds.as_slice()[0].id;
        let destination = state.worlds.as_slice()[1].id;
        fleet.location = LivingFleetLocationV2::Travelling {
            origin_world: home,
            destination_world: destination,
            departure_tick: LivingTickV2(0),
            arrival_tick: LivingTickV2(10),
        };
        fleet.order = Some(super::super::model::LivingFleetOrderV2 {
            kind: LivingFleetOrderKindV2::Move,
            target_world: Some(destination),
            not_before_tick: LivingTickV2(0),
            lane_path: Vec::new(),
        });
        state.worlds.find_mut(&home).unwrap().inventory.ore = LIVING_RESOURCE_STORAGE_LIMIT_V2;
        let sender = state.civilizations.as_slice()[0].id;
        let receiver = state.civilizations.as_slice()[1].id;
        let mut shipments = Vec::new();
        for (id, kind, units, arrival, disposition) in [
            (
                60,
                Some(LivingRouteKindV2::Trade),
                100,
                10,
                LivingShipmentDispositionV2::Outbound,
            ),
            (
                61,
                Some(LivingRouteKindV2::Aid),
                10,
                20,
                LivingShipmentDispositionV2::Outbound,
            ),
            (
                62,
                Some(LivingRouteKindV2::Trade),
                100,
                10,
                LivingShipmentDispositionV2::Returning,
            ),
            (63, None, 20, 10, LivingShipmentDispositionV2::Outbound),
        ] {
            let mut bytes = [0; 16];
            bytes[0] = id;
            shipments.push(LivingShipmentV2 {
                id: LivingEntityIdV2(bytes),
                dispatch_owner: sender,
                intended_receiver: receiver,
                source_world: home,
                destination_world: destination,
                resource: LivingResourceV2::Ore,
                units,
                departure_tick: LivingTickV2(0),
                arrival_tick: LivingTickV2(arrival),
                disposition,
                dispatch_kind: kind,
                returning_to_source: false,
            });
        }
        state.shipments = LivingSortedVecV2::new(shipments).unwrap();
        state
    }

    fn run_arrivals(
        state: LivingGalaxyStateV2,
        budget: usize,
    ) -> (LivingScheduledPhaseOutputV2, u64) {
        let mut job =
            LivingArrivalJobV2::begin(state, LivingPendingEventsV2::new(), LivingTickV2(10));
        assert_eq!(job.total_visits(), 0);
        loop {
            let status = job.poll(budget).unwrap();
            assert!(job.last_poll_visits() <= budget);
            if status == LivingLifecycleProgressV2::Ready {
                break;
            }
            assert!(job.last_poll_visits() > 0);
        }
        let visits = job.total_visits();
        (job.finish().unwrap(), visits)
    }

    #[test]
    fn arrival_budget_one_matches_large_polls_without_cloning_nested_hulls() {
        let source = arrival_fixture();
        let a = run_arrivals(source.clone(), 1);
        let b = run_arrivals(source.clone(), 64);
        let c = run_arrivals(source.clone(), 1_024);
        assert_eq!(a, b);
        assert_eq!(a, c);
        let (state, events, arrivals) = a.0;
        assert_eq!(arrivals, vec![source.fleets.as_slice()[0].id]);
        assert!(state.fleets.as_slice()[0].order.is_none());
        assert_eq!(
            state.fleets.as_slice()[0].hulls,
            source.fleets.as_slice()[0].hulls
        );
        assert_eq!(state.shipments.as_slice().len(), 2);
        assert_eq!(
            state.shipments.as_slice()[0],
            source.shipments.as_slice()[1]
        );
        assert!(state.shipments.as_slice()[1].returning_to_source);
        let relation = state
            .relations
            .as_slice()
            .iter()
            .find(|row| {
                row.from == source.civilizations.as_slice()[1].id
                    && row.to == source.civilizations.as_slice()[0].id
            })
            .unwrap();
        assert_eq!(relation.reasons.successful_trade, 1);
        assert_eq!(relation.reasons.delivered_aid, 0);
        assert_eq!(
            events
                .payloads()
                .iter()
                .map(|event| event.kind)
                .collect::<Vec<_>>(),
            vec![
                LivingEventKindV2::FleetArrived,
                LivingEventKindV2::ShipmentDelivered,
                LivingEventKindV2::ShipmentWaiting,
                LivingEventKindV2::ShipmentDelivered
            ]
        );
    }

    #[test]
    fn arrival_large_cargo_collection_yields_and_cancellation_cannot_publish() {
        let mut source = arrival_fixture();
        let prototype = source.shipments.as_slice()[1].clone();
        let mut cargo = Vec::new();
        for index in 0_u16..4_096 {
            let mut shipment = prototype.clone();
            let mut bytes = [100; 16];
            bytes[14..].copy_from_slice(&index.to_be_bytes());
            shipment.id = LivingEntityIdV2(bytes);
            cargo.push(shipment);
        }
        source.shipments = LivingSortedVecV2::new(cargo).unwrap();
        let expected = run_arrivals(source.clone(), 1);
        let mut job = LivingArrivalJobV2::begin(
            source.clone(),
            LivingPendingEventsV2::new(),
            LivingTickV2(10),
        );
        assert_eq!(job.poll(1_024), Ok(LivingLifecycleProgressV2::Pending));
        assert_eq!(job.last_poll_visits(), 1_024);
        let completed = run_arrivals(source.clone(), 1_024);
        assert_eq!(completed, expected);
        assert_eq!(completed.0.0.shipments, source.shipments);
        job.cancel();
        assert_eq!(job.poll(1), Err(LivingLifecycleJobErrorV2::Cancelled));
        assert_eq!(job.finish(), Err(LivingLifecycleJobErrorV2::Cancelled));
        assert_eq!(source.shipments.as_slice().len(), 4_096);
    }

    #[test]
    fn freight_return_search_visits_individual_lanes_and_keeps_shortest_duration() {
        use crate::living::model::{LivingLaneV2, LivingSystemV2};
        let mut source = arrival_fixture();
        let origin = source.worlds.as_slice()[0].system;
        let middle = LivingEntityIdV2([100; 16]);
        let destination = LivingEntityIdV2([101; 16]);
        let mut systems = source.systems.clone().into_vec();
        let name = source.systems.as_slice()[0].name.clone();
        systems.push(LivingSystemV2 {
            id: middle,
            name: name.clone(),
        });
        systems.push(LivingSystemV2 {
            id: destination,
            name,
        });
        source.systems = LivingSortedVecV2::new(systems).unwrap();
        let world = source.shipments.as_slice()[0].destination_world;
        source.worlds.find_mut(&world).unwrap().system = destination;
        source.lanes = LivingSortedVecV2::new(vec![
            LivingLaneV2 {
                id: LivingEntityIdV2([200; 16]),
                system_a: origin,
                system_b: middle,
                distance_units: 100,
            },
            LivingLaneV2 {
                id: LivingEntityIdV2([201; 16]),
                system_a: middle,
                system_b: destination,
                distance_units: 100,
            },
            LivingLaneV2 {
                id: LivingEntityIdV2([202; 16]),
                system_a: origin,
                system_b: destination,
                distance_units: 1_000,
            },
        ])
        .unwrap();
        source.wars = LivingSortedVecV2::new(vec![LivingWarV2 {
            participant_a: source.civilizations.as_slice()[0].id,
            participant_b: source.civilizations.as_slice()[1].id,
            declarer: source.civilizations.as_slice()[0].id,
            start_tick: LivingTickV2(1),
            end_tick: LivingTickV2(100),
        }])
        .unwrap();
        source.shipments =
            LivingSortedVecV2::new(vec![source.shipments.as_slice()[0].clone()]).unwrap();
        let small = run_arrivals(source.clone(), 1);
        let large = run_arrivals(source, 1_024);
        assert_eq!(small, large);
        let returned = &small.0.0.shipments.as_slice()[0];
        assert_eq!(returned.disposition, LivingShipmentDispositionV2::Returning);
        assert_eq!(returned.departure_tick, LivingTickV2(10));
        assert_eq!(returned.arrival_tick, LivingTickV2(30));
        assert_eq!(
            small
                .0
                .1
                .payloads()
                .iter()
                .filter(|event| event.kind == LivingEventKindV2::ShipmentDelivered)
                .count(),
            0
        );
    }

    #[test]
    fn scheduled_phase_composition_shares_one_budget_and_preserves_the_separate_outputs() {
        let source = arrival_fixture();
        let (after_lifecycle, events, _) = run(source.clone(), 1, 10);
        let mut arrival = LivingArrivalJobV2::begin(after_lifecycle, events, LivingTickV2(10));
        while arrival.poll(1).unwrap() != LivingLifecycleProgressV2::Ready {}
        let expected = arrival.finish().unwrap();
        for budget in [1, 8, 1_024] {
            let mut job = LivingScheduledPhasesJobV2::begin(
                source.clone(),
                LivingPendingEventsV2::new(),
                LivingTickV2(10),
            );
            assert_eq!(job.poll(0), Err(LivingLifecycleJobErrorV2::Budget));
            loop {
                let progress = job.poll(budget).unwrap();
                assert!(job.last_poll_visits() <= budget);
                if progress == LivingLifecycleProgressV2::Ready {
                    break;
                }
            }
            assert_eq!(job.finish().unwrap(), expected);
        }
        let mut cancelled = LivingScheduledPhasesJobV2::begin(
            source,
            LivingPendingEventsV2::new(),
            LivingTickV2(10),
        );
        cancelled.poll(1).unwrap();
        cancelled.cancel();
        assert_eq!(
            cancelled.finish(),
            Err(LivingLifecycleJobErrorV2::Cancelled)
        );
    }

    fn validation_index_entries(job: &LivingStateValidationJobV2) -> usize {
        job.targeted_jobs.len()
            + job.identities.len()
            + job.systems.len()
            + job.worlds.len()
            + job.civilizations.len()
            + job.lanes.len()
            + job.fleets.len()
            + job.declarations.len()
            + job.industry.len()
    }

    #[test]
    fn validation_cancellation_at_every_cursor_retains_owners_and_partial_indexes() {
        let authority = builtin_living_sandbox_v2().unwrap();
        let catalog = authority.catalog_snapshot();
        let mut state = authority.state().clone();
        state.observations = LivingSortedVecV2::new(vec![LivingObservationV2 {
            civilization: state.civilizations.as_slice()[0].id,
            world: state.worlds.as_slice()[0].id,
            observed_tick: state.tick,
            owner: None,
            remaining_deposit: 0,
            facilities: Default::default(),
            stationed_hulls: Default::default(),
            operational_defense_batteries: None,
        }])
        .unwrap();
        let source = Arc::new(state);
        // Visit each actual cursor, including empty phase transitions. Cancellation
        // must work before the next attempted record in every validation phase.
        for target in 0..=StateValidationCursor::Ready as usize {
            let mut job =
                LivingStateValidationJobV2::begin(Arc::clone(&catalog), Arc::clone(&source));
            let mut transitions = 0;
            while job.cursor as usize != target {
                job.visit_validation().unwrap();
                transitions += 1;
                assert!(transitions < 10_000);
            }
            let entries = validation_index_entries(&job);
            let counts = (Arc::strong_count(&source), Arc::strong_count(&catalog));
            job.cancel();
            assert_eq!(job.poll(1), Err(LivingLifecycleJobErrorV2::Cancelled));
            assert_eq!(job.poll(0), Err(LivingLifecycleJobErrorV2::Cancelled));
            assert_eq!(job.last_poll_visits(), 0);
            assert!(matches!(
                job.finish(),
                Err(LivingLifecycleJobErrorV2::Cancelled)
            ));
            assert_eq!(validation_index_entries(&job), entries);
            assert_eq!(
                (Arc::strong_count(&source), Arc::strong_count(&catalog)),
                counts
            );
            assert!(Arc::ptr_eq(job.source.as_ref().unwrap(), &source));
            assert!(Arc::ptr_eq(job.catalog.as_ref().unwrap(), &catalog));
        }
    }

    #[test]
    fn validation_explicit_retirement_counts_each_index_and_hands_off_owners_once() {
        let authority = builtin_living_sandbox_v2().unwrap();
        for disposition in 0..3 {
            let mut state = authority.state().clone();
            if disposition == 2 {
                state.worlds.record_at_mut(0).unwrap().archetype =
                    LivingSlugV2::new("missing_world_definition").unwrap();
            }
            let source = Arc::new(state);
            let catalog = authority.catalog_snapshot();
            for budgets in [&[1][..], &[64][..], &[1024][..], &[3, 1, 64, 7, 1024][..]] {
                let mut job =
                    LivingStateValidationJobV2::begin(Arc::clone(&catalog), Arc::clone(&source));
                assert_eq!(validation_index_entries(&job), 0);
                assert_eq!(
                    job.poll_retirement(0),
                    Err(LivingLifecycleJobErrorV2::Budget)
                );
                assert_eq!(
                    job.poll_retirement(1025),
                    Err(LivingLifecycleJobErrorV2::Budget)
                );
                assert!(!job.retirement_requested);
                assert!(matches!(
                    job.finish_retirement(),
                    Err(LivingLifecycleJobErrorV2::NotReady)
                ));
                loop {
                    let result = job.poll(1);
                    if disposition == 2 {
                        if matches!(result, Err(LivingLifecycleJobErrorV2::Fault(_))) {
                            break;
                        }
                    } else if job.cursor == StateValidationCursor::Worlds {
                        break;
                    }
                    assert_eq!(result, Ok(LivingLifecycleProgressV2::Pending));
                }
                if disposition == 1 {
                    job.cancel();
                }
                let error = if disposition == 2 {
                    LivingLifecycleJobErrorV2::Fault(job.fault.unwrap().into())
                } else {
                    LivingLifecycleJobErrorV2::Cancelled
                };
                let entries = validation_index_entries(&job);
                assert!(entries > 0);
                let before = job.total_visits();
                let counts = (Arc::strong_count(&source), Arc::strong_count(&catalog));
                assert!(job.finish().is_err());
                assert_eq!(validation_index_entries(&job), entries);
                let mut polls = 0;
                loop {
                    let budget = budgets[polls % budgets.len()];
                    let before_poll = job.total_visits();
                    let result = job.poll_retirement(budget).unwrap();
                    assert!(job.last_poll_visits() <= budget);
                    assert_eq!(
                        job.total_visits() - before_poll,
                        job.last_poll_visits() as u64
                    );
                    assert_eq!(
                        (Arc::strong_count(&source), Arc::strong_count(&catalog)),
                        counts
                    );
                    assert_eq!(job.poll(1), Err(error.clone()));
                    assert_eq!(job.last_poll_visits(), 0);
                    assert!(matches!(job.finish(), Err(found) if found == error));
                    polls += 1;
                    if result == LivingLifecycleProgressV2::Ready {
                        break;
                    }
                    assert!(matches!(
                        job.finish_retirement(),
                        Err(LivingLifecycleJobErrorV2::NotReady)
                    ));
                }
                assert_eq!(job.total_visits() - before, entries as u64);
                assert_eq!(validation_index_entries(&job), 0);
                assert_eq!(job.poll_retirement(1), Ok(LivingLifecycleProgressV2::Ready));
                assert_eq!(job.last_poll_visits(), 0);
                let (state_owner, pack_owner, tick) = job.finish_retirement().unwrap();
                assert!(Arc::ptr_eq(&state_owner, &source));
                assert!(Arc::ptr_eq(&pack_owner, &catalog));
                assert_eq!(tick, source.tick);
                assert!(matches!(
                    job.finish_retirement(),
                    Err(LivingLifecycleJobErrorV2::NotReady)
                ));
                assert_eq!(
                    job.poll_retirement(1),
                    Err(LivingLifecycleJobErrorV2::NotReady)
                );
                assert_eq!(job.poll(1), Err(error));
            }
        }
    }

    #[test]
    fn validation_normal_ready_drains_indexes_and_nested_visits_are_exact() {
        use crate::living::model::{LivingObservedFacilityV2, LivingObservedHullV2};
        let authority = builtin_living_sandbox_v2().unwrap();
        let mut state = authority.state().clone();
        state.hull_jobs = Default::default();
        let far_system = LivingEntityIdV2([230; 16]);
        state
            .systems
            .push_sorted(crate::living::model::LivingSystemV2 {
                id: far_system,
                name: crate::living::LivingNameV2::new("Far System").unwrap(),
            })
            .unwrap();
        state.lanes = LivingSortedVecV2::new(vec![crate::living::model::LivingLaneV2 {
            id: LivingEntityIdV2([231; 16]),
            system_a: state.systems.as_slice()[0].id,
            system_b: far_system,
            distance_units: 1,
        }])
        .unwrap();
        let fleet = state.fleets.record_at_mut(0).unwrap();
        fleet.order = Some(crate::living::model::LivingFleetOrderV2 {
            kind: LivingFleetOrderKindV2::Move,
            target_world: None,
            not_before_tick: LivingTickV2(u64::MAX),
            lane_path: Vec::new(),
        });
        let observer = state.civilizations.as_slice()[0].id;
        let world = state.worlds.as_slice()[0].id;
        state.observations = LivingSortedVecV2::new(vec![LivingObservationV2 {
            civilization: observer,
            world,
            observed_tick: state.tick,
            owner: None,
            remaining_deposit: 0,
            facilities: Default::default(),
            stationed_hulls: Default::default(),
            operational_defense_batteries: None,
        }])
        .unwrap();
        let mut totals = Vec::new();
        for nested in 0..3 {
            let mut source = state.clone();
            if nested == 1 {
                source
                    .fleets
                    .record_at_mut(0)
                    .unwrap()
                    .order
                    .as_mut()
                    .unwrap()
                    .lane_path = vec![source.lanes.as_slice()[0].id; 4096];
                let observation = source.observations.record_at_mut(0).unwrap();
                let mut industries: Vec<_> = authority
                    .catalog()
                    .industries()
                    .iter()
                    .map(|row| LivingObservedFacilityV2 {
                        definition: row.id.clone(),
                        count: u32::MAX,
                    })
                    .collect();
                industries.sort_by_key(|row| row.definition.clone());
                observation.facilities = LivingSortedVecV2::new(industries).unwrap();
                let mut hulls: Vec<_> = authority
                    .catalog()
                    .hulls()
                    .iter()
                    .map(|row| LivingObservedHullV2 {
                        definition: row.id.clone(),
                        count: 0,
                    })
                    .collect();
                hulls.sort_by_key(|row| row.definition.clone());
                observation.stationed_hulls = LivingSortedVecV2::new(hulls).unwrap();
            }
            if nested == 2 {
                let fleet = source.fleets.record_at_mut(0).unwrap();
                let mut hull = fleet.hulls.as_slice()[0].clone();
                hull.id = LivingEntityIdV2([232; 16]);
                fleet.hulls.push_sorted(hull).unwrap();
            }
            source.validate(authority.catalog()).unwrap();
            let source = Arc::new(source);
            let catalog = authority.catalog_snapshot();
            let mut total = None;
            for budgets in [&[1][..], &[64][..], &[1024][..], &[7, 1, 1024, 3, 64][..]] {
                let mut job =
                    LivingStateValidationJobV2::begin(Arc::clone(&catalog), Arc::clone(&source));
                let mut polls = 0;
                loop {
                    let budget = budgets[polls % budgets.len()];
                    let result = job.poll(budget).unwrap();
                    assert!(job.last_poll_visits() <= budget);
                    polls += 1;
                    if result == LivingLifecycleProgressV2::Ready {
                        break;
                    }
                }
                assert_eq!(validation_index_entries(&job), 0);
                if let Some(total) = total {
                    assert_eq!(job.total_visits(), total);
                }
                total = Some(job.total_visits());
                let counts = (Arc::strong_count(&source), Arc::strong_count(&catalog));
                let (state_owner, pack_owner, tick) = job.finish().unwrap();
                assert!(Arc::ptr_eq(&state_owner, &source));
                assert!(Arc::ptr_eq(&pack_owner, &catalog));
                assert_eq!(tick, source.tick);
                assert!(matches!(
                    job.finish(),
                    Err(LivingLifecycleJobErrorV2::NotReady)
                ));
                assert_eq!(
                    (Arc::strong_count(&source), Arc::strong_count(&catalog)),
                    counts
                );
            }
            totals.push(total.unwrap());
        }
        assert_eq!(
            totals[1] - totals[0],
            4096 + authority.catalog().industries().len() as u64
                + authority.catalog().hulls().len() as u64
        );
        // One extra hull visits its identity, semantic record and retired ID.
        assert_eq!(totals[2] - totals[0], 3);
    }
}

// ---------------------------------------------------- observation refresh

#[derive(Clone, Copy)]
enum ObservationCursor {
    Civilization,
    Colony,
    Fleet,
    ScoutHull,
    SystemWorld,
    VisibleWorld,
    Facility,
    StationedFleet,
    StationedHull,
    Deposit,
    Commit,
    Output,
    RetireRemembered,
    Ready,
}

/// Resumable phase-six observation refresh on a private candidate.
/// Every fleet hull and scanned entity consumes its own visit. Previously
/// remembered observations remain intact when their world is no longer visible.
pub struct LivingObservationJobV2 {
    delivered: bool,
    state: LivingGalaxyStateV2,
    tick: LivingTickV2,
    cursor: ObservationCursor,
    civilization_index: usize,
    index: usize,
    fleet_index: usize,
    hull_index: usize,
    civilization: Option<LivingEntityIdV2>,
    system: Option<LivingEntityIdV2>,
    resume: ObservationCursor,
    visible: BTreeSet<LivingEntityIdV2>,
    world: Option<LivingEntityIdV2>,
    observation: Option<LivingObservationV2>,
    facilities: BTreeMap<String, super::model::LivingObservedFacilityV2>,
    hulls: BTreeMap<String, super::model::LivingObservedHullV2>,
    batteries: u16,
    operational_batteries: u16,
    deposit: Option<u64>,
    remembered: IntoIter<LivingObservationV2>,
    observations: BTreeMap<(LivingEntityIdV2, LivingEntityIdV2), LivingObservationV2>,
    output_loaded: bool,
    retired_facilities: IntoIter<super::model::LivingObservedFacilityV2>,
    retired_hulls: IntoIter<super::model::LivingObservedHullV2>,
    cancelled: bool,
    fault: Option<LivingDeterministicFaultV2>,
    last_poll_visits: usize,
    total_visits: u64,
}
impl LivingObservationJobV2 {
    /// Move the unpublished candidate without scanning its entity collections.
    pub fn begin(mut state: LivingGalaxyStateV2, tick: LivingTickV2) -> Self {
        let remembered = std::mem::take(&mut state.observations)
            .into_vec()
            .into_iter();
        Self {
            delivered: false,
            state,
            tick,
            cursor: ObservationCursor::Output,
            civilization_index: 0,
            index: 0,
            fleet_index: 0,
            hull_index: 0,
            civilization: None,
            system: None,
            resume: ObservationCursor::Colony,
            visible: BTreeSet::new(),
            world: None,
            observation: None,
            facilities: BTreeMap::new(),
            hulls: BTreeMap::new(),
            batteries: 0,
            operational_batteries: 0,
            deposit: None,
            remembered,
            observations: BTreeMap::new(),
            output_loaded: false,
            retired_facilities: Vec::new().into_iter(),
            retired_hulls: Vec::new().into_iter(),
            cancelled: false,
            fault: None,
            last_poll_visits: 0,
            total_visits: 0,
        }
    }
    /// Prevent publication of this private candidate.
    pub fn cancel(&mut self) {
        self.cancelled = true;
    }
    /// Entity and nested hull visits consumed by the preceding poll.
    pub const fn last_poll_visits(&self) -> usize {
        self.last_poll_visits
    }
    /// Total actual visits consumed so far.
    pub const fn total_visits(&self) -> u64 {
        self.total_visits
    }
    /// Advance at most 1024 actual source/output record visits.
    pub fn poll(
        &mut self,
        budget: usize,
    ) -> Result<LivingLifecycleProgressV2, LivingLifecycleJobErrorV2> {
        self.last_poll_visits = 0;
        if self.cancelled {
            return Err(LivingLifecycleJobErrorV2::Cancelled);
        }
        if budget == 0 || budget > LIVING_MAX_PHASE_POLL_VISITS_V2 {
            return Err(LivingLifecycleJobErrorV2::Budget);
        }
        if let Some(fault) = &self.fault {
            return Err(fault.clone().into());
        }
        while self.last_poll_visits < budget {
            if matches!(self.cursor, ObservationCursor::Ready) {
                return Ok(LivingLifecycleProgressV2::Ready);
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
                    self.fault = Some(fault.clone());
                    return Err(fault.into());
                }
            }
        }
        Ok(if matches!(self.cursor, ObservationCursor::Ready) {
            LivingLifecycleProgressV2::Ready
        } else {
            LivingLifecycleProgressV2::Pending
        })
    }
    /// Take the complete private candidate only after all visits succeeded.
    pub fn finish(mut self) -> Result<LivingGalaxyStateV2, LivingLifecycleJobErrorV2> {
        self.take_ready_result()
    }
    /// Borrowed one-time extraction; rejected calls preserve every owner.
    pub fn take_ready_result(&mut self) -> Result<LivingGalaxyStateV2, LivingLifecycleJobErrorV2> {
        if self.cancelled {
            return Err(LivingLifecycleJobErrorV2::Cancelled);
        }
        if let Some(fault) = self.fault.clone() {
            return Err(fault.into());
        }
        if !matches!(self.cursor, ObservationCursor::Ready) {
            return Err(LivingLifecycleJobErrorV2::NotReady);
        }
        if self.delivered {
            return Err(LivingLifecycleJobErrorV2::NotReady);
        }
        self.delivered = true;
        Ok(std::mem::take(&mut self.state))
    }
    fn world_record(&self, id: LivingEntityIdV2) -> Option<&super::model::LivingWorldV2> {
        let rows = self.state.worlds.as_slice();
        rows.binary_search_by_key(&id, |row| row.id)
            .ok()
            .map(|index| &rows[index])
    }
    fn visit(&mut self) -> Result<bool, LivingDeterministicFaultV2> {
        use super::model::{
            LivingFleetLocationV2, LivingObservedFacilityV2, LivingObservedHullV2, LivingResourceV2,
        };
        use ObservationCursor::*;
        match self.cursor {
            Output => {
                if !self.output_loaded {
                    if let Some(row) = self.remembered.next() {
                        self.observations.insert(row.living_key(), row);
                        return Ok(true);
                    }
                    self.output_loaded = true;
                    self.cursor = Civilization;
                } else if self.observations.first_key_value().is_some() {
                    let row = &self
                        .observations
                        .first_key_value()
                        .expect("pending observation")
                        .1;
                    ensure_retained_sorted(&self.state.observations, *row)?;
                    let (_, row) = self.observations.pop_first().expect("retained observation");
                    self.state.observations.push_sorted(row)?;
                    return Ok(true);
                } else {
                    self.cursor = Ready;
                }
            }
            Civilization => {
                let Some(row) = self
                    .state
                    .civilizations
                    .as_slice()
                    .get(self.civilization_index)
                else {
                    self.cursor = Output;
                    return Ok(false);
                };
                self.civilization = Some(row.id);
                self.civilization_index += 1;
                self.index = 0;
                self.cursor = Colony;
                return Ok(true);
            }
            Colony => {
                if let Some(row) = self.state.colonies.as_slice().get(self.index) {
                    self.index += 1;
                    if Some(row.owner) == self.civilization {
                        self.system = self.world_record(row.world).map(|world| world.system);
                        if self.system.is_some() {
                            self.fleet_index = self.index;
                            self.index = 0;
                            self.resume = Colony;
                            self.cursor = SystemWorld;
                        }
                    }
                    return Ok(true);
                }
                self.fleet_index = 0;
                self.cursor = Fleet;
            }
            Fleet => {
                let Some(row) = self.state.fleets.as_slice().get(self.fleet_index) else {
                    self.cursor = VisibleWorld;
                    return Ok(false);
                };
                self.fleet_index += 1;
                if Some(row.owner) == self.civilization
                    && let LivingFleetLocationV2::Docked { world } = row.location
                {
                    self.world = Some(world);
                    self.hull_index = 0;
                    self.cursor = ScoutHull;
                }
                return Ok(true);
            }
            ScoutHull => {
                let fleet = &self.state.fleets.as_slice()[self.fleet_index - 1];
                if let Some(row) = fleet.hulls.as_slice().get(self.hull_index) {
                    self.hull_index += 1;
                    if row.definition.as_str() == "scout" {
                        self.system = self
                            .world
                            .and_then(|id| self.world_record(id))
                            .map(|row| row.system);
                        if self.system.is_some() {
                            self.index = 0;
                            self.resume = Fleet;
                            self.cursor = SystemWorld;
                        } else {
                            self.cursor = Fleet;
                        }
                    }
                    return Ok(true);
                }
                if let Some(world) = self.world {
                    self.visible.insert(world);
                }
                self.cursor = Fleet;
            }
            SystemWorld => {
                if let Some(row) = self.state.worlds.as_slice().get(self.index) {
                    self.index += 1;
                    if Some(row.system) == self.system {
                        self.visible.insert(row.id);
                    }
                    return Ok(true);
                }
                self.cursor = self.resume;
                if matches!(self.resume, Colony) {
                    self.index = self.fleet_index;
                }
            }
            VisibleWorld => {
                let Some(world) = self.visible.pop_first() else {
                    self.cursor = Civilization;
                    return Ok(false);
                };
                self.world = Some(world);
                self.observation = Some(LivingObservationV2 {
                    civilization: self
                        .civilization
                        .ok_or(LivingDeterministicFaultV2::Invariant)?,
                    world,
                    observed_tick: self.tick,
                    owner: self.world_record(world).and_then(|row| row.owner),
                    remaining_deposit: 0,
                    facilities: LivingSortedVecV2::default(),
                    stationed_hulls: LivingSortedVecV2::default(),
                    operational_defense_batteries: None,
                });
                self.index = 0;
                self.batteries = 0;
                self.operational_batteries = 0;
                self.deposit = None;
                self.cursor = Facility;
                return Ok(true);
            }
            Facility => {
                if let Some(row) = self.state.facilities.as_slice().get(self.index) {
                    self.index += 1;
                    if Some(row.world) == self.world {
                        let count = self
                            .facilities
                            .entry(row.definition.as_str().to_owned())
                            .or_insert_with(|| LivingObservedFacilityV2 {
                                definition: row.definition.clone(),
                                count: 0,
                            });
                        count.count = count.count.saturating_add(1);
                        if row.definition.as_str() == "defense_battery" {
                            self.batteries = self
                                .batteries
                                .checked_add(1)
                                .ok_or(LivingDeterministicFaultV2::Arithmetic)?;
                            if !row.disabled && row.hit_points > 0 {
                                self.operational_batteries = self
                                    .operational_batteries
                                    .checked_add(1)
                                    .ok_or(LivingDeterministicFaultV2::Arithmetic)?;
                            }
                        }
                    }
                    return Ok(true);
                }
                self.fleet_index = 0;
                self.cursor = StationedFleet;
            }
            StationedFleet => {
                if let Some(row) = self.state.fleets.as_slice().get(self.fleet_index) {
                    self.fleet_index += 1;
                    if matches!(row.location, LivingFleetLocationV2::Docked { world } if Some(world) == self.world)
                    {
                        self.hull_index = 0;
                        self.cursor = StationedHull;
                    }
                    return Ok(true);
                }
                self.index = 0;
                self.cursor = Deposit;
            }
            StationedHull => {
                let fleet = &self.state.fleets.as_slice()[self.fleet_index - 1];
                if let Some(row) = fleet.hulls.as_slice().get(self.hull_index) {
                    self.hull_index += 1;
                    let count = self
                        .hulls
                        .entry(row.definition.as_str().to_owned())
                        .or_insert_with(|| LivingObservedHullV2 {
                            definition: row.definition.clone(),
                            count: 0,
                        });
                    count.count = count.count.saturating_add(1);
                    return Ok(true);
                }
                self.cursor = StationedFleet;
            }
            Deposit => {
                if let Some(row) = self.state.deposits.as_slice().get(self.index) {
                    self.index += 1;
                    if Some(row.world) == self.world && row.resource == LivingResourceV2::Ore {
                        self.deposit = Some(
                            self.deposit
                                .map_or(row.remaining_units, |old| old.min(row.remaining_units)),
                        );
                    }
                    return Ok(true);
                }
                self.cursor = Commit;
            }
            Commit => {
                let row = self
                    .observation
                    .as_mut()
                    .ok_or(LivingDeterministicFaultV2::Invariant)?;
                if let Some((_, facility)) = self.facilities.pop_first() {
                    row.facilities.push_sorted(facility)?;
                    return Ok(true);
                }
                if let Some((_, hull)) = self.hulls.pop_first() {
                    row.stationed_hulls.push_sorted(hull)?;
                    return Ok(true);
                }
                row.remaining_deposit = self.deposit.unwrap_or(0);
                row.operational_defense_batteries = (self.operational_batteries < self.batteries)
                    .then_some(self.operational_batteries);
                let row = self
                    .observation
                    .take()
                    .ok_or(LivingDeterministicFaultV2::Invariant)?;
                if let Some(old) = self.observations.insert(row.living_key(), row) {
                    self.retired_facilities = old.facilities.into_vec().into_iter();
                    self.retired_hulls = old.stationed_hulls.into_vec().into_iter();
                }
                self.cursor = RetireRemembered;
                return Ok(true);
            }
            RetireRemembered => {
                if self.retired_facilities.next().is_some() || self.retired_hulls.next().is_some() {
                    return Ok(true);
                }
                self.cursor = VisibleWorld;
            }
            Ready => {}
        }
        Ok(false)
    }
}

// --------------------------------------------- sorted creator insertion

/// A sorted insertion that visits each source and output record separately.
/// It never reconstructs or validates a complete collection inside one visit.
pub(crate) struct LivingSortedInsertionJobV2<T: LivingKeyedV2> {
    delivered: bool,
    source: IntoIter<T>,
    output: LivingSortedVecV2<T>,
    inserted: Option<T>,
    held: Option<T>,
    source_finished: bool,
}
impl<T: LivingKeyedV2> LivingSortedInsertionJobV2<T> {
    pub(crate) fn begin(source: LivingSortedVecV2<T>, inserted: T) -> Self {
        Self {
            delivered: false,
            source: source.into_vec().into_iter(),
            output: LivingSortedVecV2::default(),
            inserted: Some(inserted),
            held: None,
            source_finished: false,
        }
    }
    /// One actual source or output record; false denotes an empty transition.
    pub(crate) fn visit(&mut self) -> Result<bool, LivingDeterministicFaultV2> {
        if self.delivered {
            return Ok(false);
        }
        if let Some(held) = &self.held {
            if self
                .inserted
                .as_ref()
                .is_some_and(|inserted| inserted.living_key() == held.living_key())
            {
                return Err(super::model::LivingValidationErrorV2::DuplicateIdentity {
                    context: "creator batch",
                }
                .into());
            }
            if self
                .inserted
                .as_ref()
                .is_some_and(|inserted| inserted.living_key() < held.living_key())
            {
                ensure_retained_sorted(
                    &self.output,
                    self.inserted.as_ref().expect("pending insertion"),
                )?;
                self.output.push_sorted(
                    self.inserted
                        .take()
                        .expect("comparison proved pending insertion"),
                )?;
            } else {
                ensure_retained_sorted(&self.output, self.held.as_ref().expect("held record"))?;
                self.output
                    .push_sorted(self.held.take().expect("held record was inspected"))?;
            }
            return Ok(true);
        }
        if !self.source_finished {
            self.held = self.source.next();
            if self.held.is_some() {
                return Ok(true);
            }
            self.source_finished = true;
            return Ok(false);
        }
        if let Some(inserted) = self.inserted.as_ref() {
            ensure_retained_sorted(&self.output, inserted)?;
            self.output
                .push_sorted(self.inserted.take().expect("retained insertion"))?;
            return Ok(true);
        }
        Ok(false)
    }
    pub(crate) fn ready(&self) -> bool {
        self.source_finished && self.held.is_none() && self.inserted.is_none()
    }
    #[allow(dead_code)]
    pub(crate) fn take_ready_result(
        &mut self,
    ) -> Result<LivingSortedVecV2<T>, LivingLifecycleJobErrorV2> {
        if !self.ready() || self.delivered {
            return Err(LivingLifecycleJobErrorV2::NotReady);
        }
        self.delivered = true;
        Ok(std::mem::take(&mut self.output))
    }
}

/// Resumable simultaneous combat round within phase 4. Retreat, occupation and
/// settlement are separate phase-4 work and are not performed by this component.
/// Inputs move into an unpublished candidate; each hull, target comparison,
/// damage application, output record and retired index entry consumes a visit.
pub struct LivingCombatJobV2<'a> {
    delivered: bool,
    catalog: PhaseCatalogSlot<'a>,
    state: LivingGalaxyStateV2,
    events: LivingPendingEventsV2,
    tick: LivingTickV2,
    stage: CombatStage,
    index: usize,
    hull_index: usize,
    target_index: usize,
    target: Option<CombatPiece>,
    pieces: Vec<CombatPiece>,
    wars: BTreeSet<Pair>,
    hull_fleets: BTreeMap<LivingEntityIdV2, LivingEntityIdV2>,
    harm_index: BTreeMap<LivingEntityIdV2, usize>,
    harm: Vec<(LivingEntityIdV2, u32, bool)>,
    facilities: Option<IntoIter<super::model::LivingFacilityV2>>,
    fleets: Option<IntoIter<super::model::LivingFleetV2>>,
    dead_reasons: Option<IntoIter<super::model::LivingBlockedReasonV2>>,
    fleet: Option<super::model::LivingFleetV2>,
    hulls: Option<IntoIter<super::model::LivingHullV2>>,
    cancelled: bool,
    fault: Option<LivingDeterministicFaultV2>,
    last_poll_visits: usize,
    total_visits: u64,
}
#[derive(Clone, Copy)]
struct CombatPiece {
    id: LivingEntityIdV2,
    fleet: Option<LivingEntityIdV2>,
    owner: LivingEntityIdV2,
    world: LivingEntityIdV2,
    damage: u32,
    class: u8,
}
#[derive(Clone, Copy, Eq, PartialEq)]
enum CombatStage {
    Wars,
    Fleets,
    Facilities,
    Targets,
    ApplyDamage,
    FacilityOutput,
    FleetOutput,
    Retire,
    Ready,
}
impl<'a> LivingCombatJobV2<'a> {
    /// Begin a combat component with a borrowed immutable catalog.
    pub fn begin(
        catalog: &'a super::catalog::ValidatedLivingCatalogPackV2,
        state: LivingGalaxyStateV2,
        events: LivingPendingEventsV2,
        tick: LivingTickV2,
    ) -> Self {
        Self::with_catalog(LivingPhaseCatalogV2::Borrowed(catalog), state, events, tick)
    }
    fn with_catalog(
        catalog: LivingPhaseCatalogV2<'a>,
        state: LivingGalaxyStateV2,
        events: LivingPendingEventsV2,
        tick: LivingTickV2,
    ) -> Self {
        Self {
            delivered: false,
            catalog: PhaseCatalogSlot(Some(catalog)),
            state,
            events,
            tick,
            stage: if tick.0.is_multiple_of(10) {
                CombatStage::Wars
            } else {
                CombatStage::Ready
            },
            index: 0,
            hull_index: 0,
            target_index: 0,
            target: None,
            pieces: Vec::new(),
            wars: BTreeSet::new(),
            hull_fleets: BTreeMap::new(),
            harm_index: BTreeMap::new(),
            harm: Vec::new(),
            facilities: None,
            fleets: None,
            dead_reasons: None,
            fleet: None,
            hulls: None,
            cancelled: false,
            fault: None,
            last_poll_visits: 0,
            total_visits: 0,
        }
    }
    /// Cancel without publishing state or events.
    pub fn cancel(&mut self) {
        self.cancelled = true;
    }
    /// Actual work consumed in the preceding poll.
    pub const fn last_poll_visits(&self) -> usize {
        self.last_poll_visits
    }
    /// Actual work consumed by this component.
    pub const fn total_visits(&self) -> u64 {
        self.total_visits
    }
    /// Poll with a shared remaining record budget.
    pub fn poll(
        &mut self,
        budget: usize,
    ) -> Result<LivingLifecycleProgressV2, LivingLifecycleJobErrorV2> {
        self.last_poll_visits = 0;
        if self.cancelled {
            return Err(LivingLifecycleJobErrorV2::Cancelled);
        }
        if budget == 0 || budget > LIVING_MAX_PHASE_POLL_VISITS_V2 {
            return Err(LivingLifecycleJobErrorV2::Budget);
        }
        if let Some(fault) = &self.fault {
            return Err(LivingLifecycleJobErrorV2::Fault(fault.clone()));
        }
        while self.last_poll_visits < budget && self.stage != CombatStage::Ready {
            match self.visit() {
                Ok(true) => {
                    self.last_poll_visits += 1;
                    self.total_visits += 1;
                }
                Ok(false) => {}
                Err(fault) => {
                    self.last_poll_visits += 1;
                    self.total_visits += 1;
                    self.fault = Some(fault.clone());
                    return Err(LivingLifecycleJobErrorV2::Fault(fault));
                }
            }
        }
        Ok(if self.stage == CombatStage::Ready {
            LivingLifecycleProgressV2::Ready
        } else {
            LivingLifecycleProgressV2::Pending
        })
    }
    /// Move the completed combat candidate to its phase-4 continuation.
    pub fn finish(
        mut self,
    ) -> Result<(LivingGalaxyStateV2, LivingPendingEventsV2), LivingLifecycleJobErrorV2> {
        self.take_ready_result()
    }
    /// Borrowed one-time extraction; rejected calls preserve every owner.
    pub fn take_ready_result(
        &mut self,
    ) -> Result<(LivingGalaxyStateV2, LivingPendingEventsV2), LivingLifecycleJobErrorV2> {
        if self.cancelled {
            return Err(LivingLifecycleJobErrorV2::Cancelled);
        }
        if let Some(fault) = self.fault.clone() {
            return Err(LivingLifecycleJobErrorV2::Fault(fault));
        }
        if self.stage != CombatStage::Ready {
            return Err(LivingLifecycleJobErrorV2::NotReady);
        }
        if self.delivered {
            return Err(LivingLifecycleJobErrorV2::NotReady);
        }
        self.delivered = true;
        Ok((
            std::mem::take(&mut self.state),
            std::mem::take(&mut self.events),
        ))
    }
    fn emit(&mut self, kind: LivingEventKindV2) -> Result<(), LivingDeterministicFaultV2> {
        self.events.record(
            LivingEventProvenanceV2::Autonomous {
                phase: LivingPhaseV2::ResolveConflict,
            },
            kind,
        )?;
        Ok(())
    }
    fn next_stage(&mut self, stage: CombatStage) {
        self.stage = stage;
        self.index = 0;
    }
    fn visit(&mut self) -> Result<bool, LivingDeterministicFaultV2> {
        use super::catalog::LivingIndustryEffectV2;
        use super::model::LivingFleetLocationV2;
        match self.stage {
            CombatStage::Wars => {
                if let Some(war) = self.state.wars.as_slice().get(self.index) {
                    if war.start_tick.0 <= self.tick.0 && self.tick.0 < war.end_tick.0 {
                        self.wars.insert((war.participant_a, war.participant_b));
                    }
                    self.index += 1;
                    return Ok(true);
                }
                self.next_stage(CombatStage::Fleets);
            }
            CombatStage::Fleets => {
                let Some(fleet) = self.state.fleets.as_slice().get(self.index) else {
                    self.next_stage(CombatStage::Facilities);
                    return Ok(false);
                };
                let LivingFleetLocationV2::Docked { world } = fleet.location else {
                    self.index += 1;
                    self.hull_index = 0;
                    return Ok(true);
                };
                if self.hull_index == 0 {
                    self.hull_index = 1;
                    return Ok(true);
                }
                if let Some(hull) = fleet.hulls.as_slice().get(self.hull_index - 1) {
                    let damage = self
                        .catalog
                        .hull(hull.definition.as_str())
                        .map(|row| row.damage)
                        .unwrap_or(0);
                    self.pieces.push(CombatPiece {
                        id: hull.id,
                        fleet: Some(fleet.id),
                        owner: fleet.owner,
                        world,
                        damage,
                        class: if hull.definition.as_str() == "escort" {
                            0
                        } else if damage > 0 {
                            1
                        } else {
                            2
                        },
                    });
                    self.hull_fleets.insert(hull.id, fleet.id);
                    self.hull_index += 1;
                    return Ok(true);
                }
                self.index += 1;
                self.hull_index = 0;
            }
            CombatStage::Facilities => {
                if let Some(row) = self.state.facilities.as_slice().get(self.index) {
                    if let Some(definition) = self.catalog.industry(row.definition.as_str())
                        && let LivingIndustryEffectV2::Defense {
                            damage_per_round, ..
                        } = definition.effect
                        && row.hit_points > 0
                        && let Ok(position) = self
                            .state
                            .worlds
                            .as_slice()
                            .binary_search_by_key(&row.world, |world| world.id)
                        && let Some(owner) = self.state.worlds.as_slice()[position].owner
                    {
                        self.pieces.push(CombatPiece {
                            id: row.id,
                            fleet: None,
                            owner,
                            world: row.world,
                            damage: if row.disabled { 0 } else { damage_per_round },
                            class: 1,
                        });
                    }
                    self.index += 1;
                    return Ok(true);
                }
                self.next_stage(CombatStage::Targets);
            }
            CombatStage::Targets => {
                let Some(attacker) = self.pieces.get(self.index).copied() else {
                    if !self.harm.is_empty() {
                        self.emit(LivingEventKindV2::CombatRound)?;
                    }
                    self.next_stage(CombatStage::ApplyDamage);
                    return Ok(false);
                };
                if attacker.damage == 0 {
                    self.index += 1;
                    return Ok(true);
                }
                if let Some(target) = self.pieces.get(self.target_index).copied() {
                    let pair = if attacker.owner < target.owner {
                        (attacker.owner, target.owner)
                    } else {
                        (target.owner, attacker.owner)
                    };
                    if target.world == attacker.world
                        && target.owner != attacker.owner
                        && self.wars.contains(&pair)
                        && self.target.is_none_or(|previous| {
                            (target.class, target.id) < (previous.class, previous.id)
                        })
                    {
                        self.target = Some(target);
                    }
                    self.target_index += 1;
                    return Ok(true);
                }
                if let Some(target) = self.target.take() {
                    if let Some(position) = self.harm_index.get(&target.id).copied() {
                        self.harm[position].1 =
                            self.harm[position].1.saturating_add(attacker.damage);
                    } else {
                        self.harm_index.insert(target.id, self.harm.len());
                        self.harm
                            .push((target.id, attacker.damage, target.fleet.is_none()));
                    }
                }
                self.index += 1;
                self.target_index = 0;
                return Ok(true);
            }
            CombatStage::ApplyDamage => {
                if let Some((id, damage, battery)) = self.harm.get(self.index).copied() {
                    let repair_tick = || {
                        self.tick
                            .0
                            .checked_add(10)
                            .map(LivingTickV2)
                            .ok_or(LivingDeterministicFaultV2::Arithmetic)
                    };
                    if battery {
                        if let Some(row) = self.state.facilities.find_mut(&id) {
                            row.hit_points = row.hit_points.saturating_sub(damage);
                            if row.hit_points > 0 && row.next_repair.is_none() {
                                row.next_repair = Some(repair_tick()?);
                            }
                        }
                    } else if let Some(fleet_id) = self.hull_fleets.get(&id)
                        && let Some(fleet) = self.state.fleets.find_mut(fleet_id)
                        && let Some(hull) = fleet.hulls.find_mut(&id)
                    {
                        hull.hit_points = hull.hit_points.saturating_sub(damage);
                        if hull.hit_points > 0 && hull.next_repair.is_none() {
                            hull.next_repair = Some(repair_tick()?);
                        }
                    }
                    self.index += 1;
                    return Ok(true);
                }
                self.facilities = Some(
                    std::mem::take(&mut self.state.facilities)
                        .into_vec()
                        .into_iter(),
                );
                self.next_stage(CombatStage::FacilityOutput);
            }
            CombatStage::FacilityOutput => {
                if self
                    .dead_reasons
                    .as_mut()
                    .and_then(Iterator::next)
                    .is_some()
                {
                    return Ok(true);
                }
                self.dead_reasons = None;
                if let Some(row) = self
                    .facilities
                    .as_ref()
                    .and_then(|rows| rows.as_slice().first())
                {
                    let dead_battery = row.hit_points == 0
                        && self.catalog.industry(row.definition.as_str()).is_some_and(
                            |definition| {
                                matches!(
                                    definition.effect,
                                    super::catalog::LivingIndustryEffectV2::Defense { .. }
                                )
                            },
                        );
                    if !dead_battery {
                        ensure_retained_sorted(&self.state.facilities, row)?;
                    }
                }
                if let Some(mut row) = self
                    .facilities
                    .as_mut()
                    .expect("facility output initialized")
                    .next()
                {
                    let battery =
                        self.catalog
                            .industry(row.definition.as_str())
                            .is_some_and(|definition| {
                                matches!(definition.effect, LivingIndustryEffectV2::Defense { .. })
                            });
                    if battery && row.hit_points == 0 {
                        self.dead_reasons = Some(
                            std::mem::take(&mut row.blocked_reasons)
                                .into_vec()
                                .into_iter(),
                        );
                        self.emit(LivingEventKindV2::HullDestroyed)?;
                    } else {
                        self.state.facilities.push_sorted(row)?;
                    }
                    return Ok(true);
                }
                self.facilities = None;
                self.fleets = Some(
                    std::mem::take(&mut self.state.fleets)
                        .into_vec()
                        .into_iter(),
                );
                self.next_stage(CombatStage::FleetOutput);
            }
            CombatStage::FleetOutput => {
                if let Some(hull) = self.hulls.as_mut().and_then(Iterator::next) {
                    if hull.hit_points == 0 {
                        self.emit(LivingEventKindV2::HullDestroyed)?;
                    } else {
                        self.fleet
                            .as_mut()
                            .expect("fleet output initialized")
                            .hulls
                            .push_sorted(hull)?;
                    }
                    return Ok(true);
                }
                self.hulls = None;
                if let Some(fleet) = self.fleet.as_ref() {
                    ensure_retained_sorted(&self.state.fleets, fleet)?;
                }
                if let Some(fleet) = self.fleet.take() {
                    self.state.fleets.push_sorted(fleet)?;
                    return Ok(true);
                }
                if let Some(mut fleet) = self
                    .fleets
                    .as_mut()
                    .expect("fleet iterator initialized")
                    .next()
                {
                    self.hulls = Some(std::mem::take(&mut fleet.hulls).into_vec().into_iter());
                    self.fleet = Some(fleet);
                    return Ok(true);
                }
                self.fleets = None;
                self.next_stage(CombatStage::Retire);
            }
            CombatStage::Retire => {
                if self.pieces.pop().is_some()
                    || self.harm.pop().is_some()
                    || self.wars.pop_first().is_some()
                    || self.hull_fleets.pop_first().is_some()
                    || self.harm_index.pop_first().is_some()
                {
                    return Ok(true);
                }
                self.next_stage(CombatStage::Ready);
            }
            CombatStage::Ready => {}
        }
        Ok(false)
    }
}
impl LivingCombatJobV2<'static> {
    /// Begin with an immutable owned catalog handle without scanning or cloning it.
    pub fn begin_owned(
        catalog: std::sync::Arc<super::catalog::ValidatedLivingCatalogPackV2>,
        state: LivingGalaxyStateV2,
        events: LivingPendingEventsV2,
        tick: LivingTickV2,
    ) -> Self {
        Self::with_catalog(LivingPhaseCatalogV2::Owned(catalog), state, events, tick)
    }
}

#[derive(Default)]
pub(crate) struct RetreatGraph {
    // Callers append records through counted work in canonical sorted order.
    pub(crate) systems: Vec<LivingEntityIdV2>,
    pub(crate) lanes: Vec<(LivingEntityIdV2, LivingEntityIdV2, LivingEntityIdV2, u64)>,
}
struct RetreatDistance {
    cost: u64,
    path: Vec<LivingEntityIdV2>,
    used: bool,
}
#[derive(Clone, Copy)]
enum RetreatPathStage {
    Initialize,
    Choose,
    ChooseCompare,
    Select,
    Lane,
    LaneCompare,
    Copy,
    Replace,
    Retire,
    Ready,
}
/// Faithful resumable version of the reference's array Dijkstra search. Path
/// comparison, copying and retirement are each individual lane-ID visits.
pub(crate) struct RetreatPathJob {
    graph: PhaseArc<RetreatGraph>,
    origin: LivingEntityIdV2,
    destination: LivingEntityIdV2,
    stage: RetreatPathStage,
    best: Vec<RetreatDistance>,
    index: usize,
    choice: Option<usize>,
    selected: usize,
    lane: usize,
    next: usize,
    next_cost: u64,
    compare: usize,
    copied: Vec<LivingEntityIdV2>,
    retired: Vec<LivingEntityIdV2>,
    result: Option<(u64, Vec<LivingEntityIdV2>)>,
    delivered: bool,
}
impl RetreatPathJob {
    /// System-level search; callers resolve worlds and handle local travel first.
    pub(crate) fn begin(
        graph: std::sync::Arc<RetreatGraph>,
        origin: LivingEntityIdV2,
        destination: LivingEntityIdV2,
    ) -> Self {
        Self {
            graph: graph.into(),
            origin,
            destination,
            stage: RetreatPathStage::Initialize,
            best: Vec::new(),
            index: 0,
            choice: None,
            selected: 0,
            lane: 0,
            next: 0,
            next_cost: 0,
            compare: 0,
            copied: Vec::new(),
            retired: Vec::new(),
            result: None,
            delivered: false,
        }
    }
    pub(crate) fn ready(&self) -> bool {
        matches!(self.stage, RetreatPathStage::Ready)
    }
    /// Borrow the job so rejected extraction retains all partial owners. Only
    /// the ready path moves; the graph handle remains owned by this job.
    pub(crate) fn take_ready_result(
        &mut self,
    ) -> Result<Option<(u64, Vec<LivingEntityIdV2>)>, LivingLifecycleJobErrorV2> {
        if !self.ready() || self.delivered {
            return Err(LivingLifecycleJobErrorV2::NotReady);
        }
        self.delivered = true;
        Ok(self.result.take())
    }
    #[cfg(test)]
    pub(crate) fn scratch_path_lengths(&self) -> (usize, usize, usize, usize) {
        (
            self.best.len(),
            self.best.iter().map(|row| row.path.len()).sum(),
            self.copied.len(),
            self.retired.len(),
        )
    }
    fn candidate_element(&self, index: usize) -> Option<LivingEntityIdV2> {
        let path = &self.best[self.selected].path;
        if index < path.len() {
            Some(path[index])
        } else if index == path.len() {
            Some(self.graph.lanes[self.lane].0)
        } else {
            None
        }
    }
    fn begin_replacement(&mut self) {
        self.compare = 0;
        self.stage = RetreatPathStage::Copy;
    }
    /// At most one actual record; false denotes an empty stage transition.
    pub(crate) fn visit(&mut self) -> bool {
        use RetreatPathStage::*;
        match self.stage {
            Initialize => {
                if let Some(id) = self.graph.systems.get(self.index) {
                    self.best.push(RetreatDistance {
                        cost: if *id == self.origin { 0 } else { u64::MAX },
                        path: Vec::new(),
                        used: false,
                    });
                    self.index += 1;
                    return true;
                }
                self.index = 0;
                self.stage = Choose;
            }
            Choose => {
                if self.index == self.best.len() {
                    self.stage = Select;
                    return false;
                }
                let row = &self.best[self.index];
                if !row.used && row.cost != u64::MAX {
                    match self.choice {
                        None => self.choice = Some(self.index),
                        Some(choice) if row.cost < self.best[choice].cost => {
                            self.choice = Some(self.index)
                        }
                        Some(choice) if row.cost == self.best[choice].cost => {
                            self.compare = 0;
                            self.stage = ChooseCompare;
                            return true;
                        }
                        _ => {}
                    }
                }
                self.index += 1;
                return true;
            }
            ChooseCompare => {
                let choice = self.choice.expect("equal-cost choice exists");
                let left = self.best[self.index].path.get(self.compare);
                let right = self.best[choice].path.get(self.compare);
                match left.cmp(&right) {
                    std::cmp::Ordering::Less => {
                        self.choice = Some(self.index);
                        self.index += 1;
                        self.stage = Choose;
                    }
                    std::cmp::Ordering::Greater => {
                        self.index += 1;
                        self.stage = Choose;
                    }
                    std::cmp::Ordering::Equal if left.is_none() => {
                        self.index += 1;
                        self.stage = Choose;
                        return false;
                    }
                    std::cmp::Ordering::Equal => self.compare += 1,
                }
                return true;
            }
            Select => {
                let Some(choice) = self.choice.take() else {
                    self.stage = Retire;
                    return false;
                };
                self.selected = choice;
                self.best[choice].used = true;
                if self.graph.systems[choice] == self.destination {
                    self.result = Some((
                        self.best[choice].cost,
                        std::mem::take(&mut self.best[choice].path),
                    ));
                    self.stage = Retire;
                } else {
                    self.lane = 0;
                    self.stage = Lane;
                }
            }
            Lane => {
                let Some((_, a, b, distance)) = self.graph.lanes.get(self.lane).copied() else {
                    self.index = 0;
                    self.stage = Choose;
                    return false;
                };
                let system = self.graph.systems[self.selected];
                let next = if a == system {
                    Some(b)
                } else if b == system {
                    Some(a)
                } else {
                    None
                };
                if let Some(next) = next
                    && let Ok(index) = self.graph.systems.binary_search(&next)
                {
                    self.next = index;
                    self.next_cost = self.best[self.selected]
                        .cost
                        .saturating_add(distance.div_ceil(10).max(10));
                    match self.next_cost.cmp(&self.best[index].cost) {
                        std::cmp::Ordering::Less => self.begin_replacement(),
                        std::cmp::Ordering::Equal => {
                            self.compare = 0;
                            self.stage = LaneCompare;
                        }
                        std::cmp::Ordering::Greater => self.lane += 1,
                    }
                } else {
                    self.lane += 1;
                }
                return true;
            }
            LaneCompare => {
                let left = self.candidate_element(self.compare);
                let right = self.best[self.next].path.get(self.compare).copied();
                match left.cmp(&right) {
                    std::cmp::Ordering::Less => self.begin_replacement(),
                    std::cmp::Ordering::Greater => {
                        self.lane += 1;
                        self.stage = Lane;
                    }
                    std::cmp::Ordering::Equal if left.is_none() => {
                        self.lane += 1;
                        self.stage = Lane;
                        return false;
                    }
                    std::cmp::Ordering::Equal => self.compare += 1,
                }
                return true;
            }
            Copy => {
                if let Some(id) = self.candidate_element(self.compare) {
                    self.copied.push(id);
                    self.compare += 1;
                    return true;
                }
                self.retired = std::mem::take(&mut self.best[self.next].path);
                self.stage = Replace;
            }
            Replace => {
                if self.retired.pop().is_some() {
                    return true;
                }
                self.best[self.next].cost = self.next_cost;
                self.best[self.next].path = std::mem::take(&mut self.copied);
                self.lane += 1;
                self.stage = Lane;
            }
            Retire => {
                if self.retired.pop().is_some() {
                    return true;
                }
                if let Some(row) = self.best.pop() {
                    self.retired = row.path;
                    return true;
                }
                self.stage = Ready;
            }
            Ready => {}
        }
        false
    }
}

#[derive(Clone, Copy, Eq, PartialEq)]
enum RetreatStage {
    Worlds,
    Wars,
    Colonies,
    Systems,
    Lanes,
    Fleet,
    Hulls,
    Destinations,
    Search,
    RetirePath,
    Credits,
    RetireOrder,
    Depart,
    Retire,
    Ready,
}
/// Bounded pre-combat retreat component on an unpublished candidate. Every
/// nested hull/path/index record is charged, including discarded itineraries.
/// Occupation, combat and settlement remain separate phase-four components.
pub struct LivingRetreatJobV2<'a> {
    retirement: PhaseRetirementLane<'a>,
    delivered: bool,
    catalog: PhaseCatalogSlot<'a>,
    state: LivingGalaxyStateV2,
    events: LivingPendingEventsV2,
    tick: LivingTickV2,
    stage: RetreatStage,
    index: usize,
    fleet_index: usize,
    hull_index: usize,
    colony_index: usize,
    origin_world: LivingEntityIdV2,
    destination_world: LivingEntityIdV2,
    metadata: BTreeMap<LivingEntityIdV2, (LivingEntityIdV2, Option<LivingEntityIdV2>)>,
    wars: BTreeSet<Pair>,
    colonies: Vec<(LivingEntityIdV2, LivingEntityIdV2)>,
    graph: PhaseArc<RetreatGraph>,
    search: Option<RetreatPathJob>,
    winner: Option<(u64, LivingEntityIdV2, Vec<LivingEntityIdV2>)>,
    retired_path: Vec<LivingEntityIdV2>,
    cancelled: bool,
    fault: Option<LivingDeterministicFaultV2>,
    last_poll_visits: usize,
    total_visits: u64,
}
impl<'a> LivingRetreatJobV2<'a> {
    /// Move a candidate with a borrowed immutable catalog; no records are visited.
    pub fn begin(
        catalog: &'a super::catalog::ValidatedLivingCatalogPackV2,
        state: LivingGalaxyStateV2,
        events: LivingPendingEventsV2,
        tick: LivingTickV2,
    ) -> Self {
        Self::with_catalog(LivingPhaseCatalogV2::Borrowed(catalog), state, events, tick)
    }
    fn with_catalog(
        catalog: LivingPhaseCatalogV2<'a>,
        state: LivingGalaxyStateV2,
        events: LivingPendingEventsV2,
        tick: LivingTickV2,
    ) -> Self {
        Self {
            retirement: PhaseRetirementLane::default(),
            delivered: false,
            catalog: PhaseCatalogSlot(Some(catalog)),
            state,
            events,
            tick,
            stage: RetreatStage::Worlds,
            index: 0,
            fleet_index: 0,
            hull_index: 0,
            colony_index: 0,
            origin_world: LivingEntityIdV2([0; 16]),
            destination_world: LivingEntityIdV2([0; 16]),
            metadata: BTreeMap::new(),
            wars: BTreeSet::new(),
            colonies: Vec::new(),
            graph: std::sync::Arc::new(RetreatGraph::default()).into(),
            search: None,
            winner: None,
            retired_path: Vec::new(),
            cancelled: false,
            fault: None,
            last_poll_visits: 0,
            total_visits: 0,
        }
    }
    /// Cancel without changing resident truth.
    pub fn cancel(&mut self) {
        self.cancelled = true;
    }
    /// Actual records visited in the preceding poll.
    pub const fn last_poll_visits(&self) -> usize {
        self.last_poll_visits
    }
    /// Actual records visited across polls.
    pub const fn total_visits(&self) -> u64 {
        self.total_visits
    }
    /// Continue with 1..=1024 actual visits.
    pub fn poll(
        &mut self,
        budget: usize,
    ) -> Result<LivingLifecycleProgressV2, LivingLifecycleJobErrorV2> {
        self.last_poll_visits = 0;
        if self.cancelled {
            return Err(LivingLifecycleJobErrorV2::Cancelled);
        }
        if budget == 0 || budget > LIVING_MAX_PHASE_POLL_VISITS_V2 {
            return Err(LivingLifecycleJobErrorV2::Budget);
        }
        if let Some(fault) = &self.fault {
            return Err(LivingLifecycleJobErrorV2::Fault(fault.clone()));
        }
        while self.last_poll_visits < budget && self.stage != RetreatStage::Ready {
            if self.retirement.visit() {
                self.last_poll_visits += 1;
                self.total_visits += 1;
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
                    self.fault = Some(fault.clone());
                    return Err(LivingLifecycleJobErrorV2::Fault(fault));
                }
            }
        }
        Ok(if self.stage == RetreatStage::Ready {
            LivingLifecycleProgressV2::Ready
        } else {
            LivingLifecycleProgressV2::Pending
        })
    }
    /// Move a completed candidate to the combat component; no publication occurs.
    pub fn finish(
        mut self,
    ) -> Result<(LivingGalaxyStateV2, LivingPendingEventsV2), LivingLifecycleJobErrorV2> {
        self.take_ready_result()
    }
    /// Borrowed one-time extraction; rejected calls preserve every owner.
    pub fn take_ready_result(
        &mut self,
    ) -> Result<(LivingGalaxyStateV2, LivingPendingEventsV2), LivingLifecycleJobErrorV2> {
        if self.cancelled {
            return Err(LivingLifecycleJobErrorV2::Cancelled);
        }
        if let Some(fault) = self.fault.clone() {
            return Err(LivingLifecycleJobErrorV2::Fault(fault));
        }
        if self.stage != RetreatStage::Ready {
            return Err(LivingLifecycleJobErrorV2::NotReady);
        }
        if self.delivered {
            return Err(LivingLifecycleJobErrorV2::NotReady);
        }
        self.delivered = true;
        Ok((
            std::mem::take(&mut self.state),
            std::mem::take(&mut self.events),
        ))
    }
    fn advance(&mut self, stage: RetreatStage) {
        self.stage = stage;
        self.index = 0;
    }
    fn visit(&mut self) -> Result<bool, LivingDeterministicFaultV2> {
        use super::model::{LivingFleetLocationV2, LivingFleetOrderKindV2, LivingFleetOrderV2};
        use RetreatStage::*;
        match self.stage {
            Worlds => {
                if let Some(row) = self.state.worlds.as_slice().get(self.index) {
                    self.metadata.insert(row.id, (row.system, row.owner));
                    self.index += 1;
                    return Ok(true);
                }
                self.advance(Wars);
            }
            Wars => {
                if let Some(row) = self.state.wars.as_slice().get(self.index) {
                    if row.start_tick.0 <= self.tick.0 && self.tick.0 < row.end_tick.0 {
                        self.wars.insert((row.participant_a, row.participant_b));
                    }
                    self.index += 1;
                    return Ok(true);
                }
                self.advance(Colonies);
            }
            Colonies => {
                if let Some(row) = self.state.colonies.as_slice().get(self.index) {
                    self.colonies.push((row.world, row.owner));
                    self.index += 1;
                    return Ok(true);
                }
                self.advance(Systems);
            }
            Systems => {
                if let Some(row) = self.state.systems.as_slice().get(self.index) {
                    std::sync::Arc::get_mut(&mut self.graph)
                        .expect("unshared graph construction")
                        .systems
                        .push(row.id);
                    self.index += 1;
                    return Ok(true);
                }
                self.advance(Lanes);
            }
            Lanes => {
                if let Some(row) = self.state.lanes.as_slice().get(self.index) {
                    std::sync::Arc::get_mut(&mut self.graph)
                        .expect("unshared graph construction")
                        .lanes
                        .push((row.id, row.system_a, row.system_b, row.distance_units));
                    self.index += 1;
                    return Ok(true);
                }
                self.advance(Fleet);
            }
            Fleet => {
                let Some(fleet) = self.state.fleets.as_slice().get(self.fleet_index) else {
                    self.advance(Retire);
                    return Ok(false);
                };
                if let LivingFleetLocationV2::Docked { world } = fleet.location
                    && let Some((_, Some(owner))) = self.metadata.get(&world)
                    && *owner != fleet.owner
                    && self
                        .wars
                        .contains(&(fleet.owner.min(*owner), fleet.owner.max(*owner)))
                    && !fleet.hulls.as_slice().is_empty()
                {
                    self.origin_world = world;
                    self.hull_index = 0;
                    self.stage = Hulls;
                } else {
                    self.fleet_index += 1;
                }
                return Ok(true);
            }
            Hulls => {
                let fleet = &self.state.fleets.as_slice()[self.fleet_index];
                if let Some(hull) = fleet.hulls.as_slice().get(self.hull_index) {
                    let damage = self
                        .catalog
                        .hull(hull.definition.as_str())
                        .map(|row| row.damage)
                        .unwrap_or(0);
                    if damage > 0 || !hull.return_credit {
                        self.fleet_index += 1;
                        self.stage = Fleet;
                    } else {
                        self.hull_index += 1;
                    }
                    return Ok(true);
                }
                self.colony_index = 0;
                self.stage = Destinations;
            }
            Destinations => {
                let Some((world, owner)) = self.colonies.get(self.colony_index).copied() else {
                    if self.winner.is_some() {
                        self.hull_index = 0;
                        self.stage = Credits;
                    } else {
                        self.fleet_index += 1;
                        self.stage = Fleet;
                    }
                    return Ok(false);
                };
                self.colony_index += 1;
                if owner == self.state.fleets.as_slice()[self.fleet_index].owner
                    && world != self.origin_world
                    && let Some((from, _)) = self.metadata.get(&self.origin_world)
                    && let Some((to, _)) = self.metadata.get(&world)
                {
                    self.destination_world = world;
                    if from == to {
                        self.accept_path(10, Vec::new());
                    } else {
                        self.search = Some(RetreatPathJob::begin(
                            std::sync::Arc::clone(&self.graph),
                            *from,
                            *to,
                        ));
                        self.stage = Search;
                    }
                }
                return Ok(true);
            }
            Search => {
                let search = self.search.as_mut().expect("search initialized");
                if search.ready() {
                    if let Some((duration, path)) = search
                        .take_ready_result()
                        .expect("undelivered ready search")
                    {
                        self.accept_path(duration, path);
                    } else {
                        self.stage = Destinations;
                    }
                    self.retirement
                        .start(self.search.take().expect("retained ready search"));
                    return Ok(true);
                }
                return Ok(search.visit());
            }
            RetirePath => {
                if self.retired_path.pop().is_some() {
                    return Ok(true);
                }
                self.stage = Destinations;
            }
            Credits => {
                let fleet = self
                    .state
                    .fleets
                    .record_at_mut(self.fleet_index)
                    .expect("fleet exists");
                if let Some(hull) = fleet.hulls.record_at_mut(self.hull_index) {
                    hull.return_credit = false;
                    self.hull_index += 1;
                    return Ok(true);
                }
                if let Some(order) = fleet.order.take() {
                    self.retired_path = order.lane_path;
                }
                self.stage = RetireOrder;
            }
            RetireOrder => {
                if self.retired_path.pop().is_some() {
                    return Ok(true);
                }
                self.stage = Depart;
            }
            Depart => {
                let duration = self.winner.as_ref().expect("retreat destination exists").0;
                let arrival_tick = self
                    .tick
                    .0
                    .checked_add(duration)
                    .map(LivingTickV2)
                    .ok_or(LivingDeterministicFaultV2::Arithmetic)?;
                let (_, destination, path) =
                    self.winner.take().expect("retreat destination exists");
                let fleet = self
                    .state
                    .fleets
                    .record_at_mut(self.fleet_index)
                    .expect("fleet exists");
                fleet.order = Some(LivingFleetOrderV2 {
                    kind: LivingFleetOrderKindV2::Return,
                    target_world: Some(destination),
                    not_before_tick: self.tick,
                    lane_path: path,
                });
                fleet.location = LivingFleetLocationV2::Travelling {
                    origin_world: self.origin_world,
                    destination_world: destination,
                    departure_tick: self.tick,
                    arrival_tick,
                };
                self.events.record(
                    LivingEventProvenanceV2::Autonomous {
                        phase: LivingPhaseV2::ResolveConflict,
                    },
                    LivingEventKindV2::FleetDeparted,
                )?;
                self.fleet_index += 1;
                self.stage = Fleet;
                return Ok(true);
            }
            Retire => {
                let graph = std::sync::Arc::get_mut(&mut self.graph)
                    .expect("completed searches released graph");
                if graph.systems.pop().is_some()
                    || graph.lanes.pop().is_some()
                    || self.metadata.pop_first().is_some()
                    || self.wars.pop_first().is_some()
                    || self.colonies.pop().is_some()
                {
                    return Ok(true);
                }
                self.advance(Ready);
            }
            Ready => {}
        }
        Ok(false)
    }
    fn accept_path(&mut self, duration: u64, path: Vec<LivingEntityIdV2>) {
        if self
            .winner
            .as_ref()
            .is_none_or(|(cost, world, _)| (duration, self.destination_world) < (*cost, *world))
        {
            if let Some((_, _, previous)) = self.winner.take() {
                self.retired_path = previous;
            }
            self.winner = Some((duration, self.destination_world, path));
        } else {
            self.retired_path = path;
        }
        self.stage = RetreatStage::RetirePath;
    }
}
impl LivingRetreatJobV2<'static> {
    /// Move an owned immutable catalog handle without scanning it.
    pub fn begin_owned(
        catalog: std::sync::Arc<super::catalog::ValidatedLivingCatalogPackV2>,
        state: LivingGalaxyStateV2,
        events: LivingPendingEventsV2,
        tick: LivingTickV2,
    ) -> Self {
        Self::with_catalog(LivingPhaseCatalogV2::Owned(catalog), state, events, tick)
    }
}

#[cfg(test)]
mod retreat_tests {
    use super::*;
    use crate::living::model::*;
    use crate::living::simulation::LivingStepContextV2;
    use crate::living::{LivingSlugV2, builtin_living_sandbox_v2};
    fn fixture() -> (
        super::super::simulation::LivingGalaxyAuthorityV2,
        LivingGalaxyStateV2,
    ) {
        let authority = builtin_living_sandbox_v2().unwrap();
        let mut state = authority.state().clone();
        let a = state.civilizations.as_slice()[0].id;
        let b = state.civilizations.as_slice()[1].id;
        let origin = state
            .worlds
            .as_slice()
            .iter()
            .find(|world| world.owner == Some(b))
            .unwrap()
            .id;
        state.systems = super::super::wire::LivingSortedVecV2::new(
            (1..=4)
                .map(|index| LivingSystemV2 {
                    id: LivingEntityIdV2([100 + index; 16]),
                    name: crate::living::LivingNameV2::new(format!("Retreat System {index}"))
                        .unwrap(),
                })
                .collect(),
        )
        .unwrap();
        let systems: Vec<_> = state.systems.as_slice().iter().map(|row| row.id).collect();
        state.worlds.find_mut(&origin).unwrap().system = systems[0];
        let home: Vec<_> = state
            .colonies
            .as_slice()
            .iter()
            .filter(|row| row.owner == a)
            .map(|row| row.world)
            .collect();
        for world in home {
            state.worlds.find_mut(&world).unwrap().system = systems[3];
        }
        state.wars = super::super::wire::LivingSortedVecV2::new(vec![LivingWarV2 {
            participant_a: a.min(b),
            participant_b: a.max(b),
            declarer: a,
            start_tick: LivingTickV2(0),
            end_tick: LivingTickV2(u64::MAX),
        }])
        .unwrap();
        let fleet = state.fleets.record_at_mut(0).unwrap();
        fleet.owner = a;
        fleet.location = LivingFleetLocationV2::Docked { world: origin };
        let hulls: Vec<_> = fleet.hulls.as_slice().iter().map(|row| row.id).collect();
        for id in hulls {
            let hull = fleet.hulls.find_mut(&id).unwrap();
            hull.definition = LivingSlugV2::new("scout").unwrap();
            hull.return_credit = true;
        }
        let lane = |id, a: LivingEntityIdV2, b: LivingEntityIdV2| LivingLaneV2 {
            id: LivingEntityIdV2([id; 16]),
            system_a: a.min(b),
            system_b: a.max(b),
            distance_units: 100,
        };
        state.lanes = super::super::wire::LivingSortedVecV2::new(vec![
            lane(10, systems[0], systems[2]),
            lane(20, systems[0], systems[1]),
            lane(30, systems[1], systems[3]),
            lane(40, systems[2], systems[3]),
        ])
        .unwrap();
        (authority, state)
    }
    fn reference(
        authority: &super::super::simulation::LivingGalaxyAuthorityV2,
        state: &LivingGalaxyStateV2,
        tick: LivingTickV2,
    ) -> Result<(LivingGalaxyStateV2, LivingPendingEventsV2), LivingDeterministicFaultV2> {
        let mut state = state.clone();
        let mut events = LivingPendingEventsV2::default();
        super::super::rules::reference_retreat_for_cursor_test(&mut LivingStepContextV2 {
            catalog: authority.catalog(),
            genesis_seed: [0; 32],
            branch: authority.branch(),
            tick,
            state: &mut state,
            events: &mut events,
            arrivals: Vec::new(),
        })?;
        Ok((state, events))
    }
    #[test]
    fn retreat_cursor_exact_equal_cost_path_and_destination_ties_all_budgets() {
        let (authority, state) = fixture();
        let (expected, events) = reference(&authority, &state, LivingTickV2(10)).unwrap();
        let order = expected.fleets.as_slice()[0].order.as_ref().unwrap();
        assert_eq!(
            order.lane_path,
            vec![LivingEntityIdV2([10; 16]), LivingEntityIdV2([40; 16])]
        );
        let owner = state.fleets.as_slice()[0].owner;
        let lowest = state
            .colonies
            .as_slice()
            .iter()
            .filter(|row| row.owner == owner)
            .map(|row| row.world)
            .min()
            .unwrap();
        assert_eq!(order.target_world, Some(lowest));
        let mut visits = None;
        for budgets in [&[1][..], &[64][..], &[1024][..], &[1, 7, 64, 1024][..]] {
            let mut job = LivingRetreatJobV2::begin_owned(
                authority.catalog_snapshot(),
                state.clone(),
                LivingPendingEventsV2::default(),
                LivingTickV2(10),
            );
            let mut index = 0;
            loop {
                let budget = budgets[index % budgets.len()];
                index += 1;
                let progress = job.poll(budget).unwrap();
                assert!(job.last_poll_visits() <= budget);
                if progress == LivingLifecycleProgressV2::Ready {
                    break;
                }
            }
            if let Some(visits) = visits {
                assert_eq!(job.total_visits(), visits);
            } else {
                visits = Some(job.total_visits());
            }
            let (actual, actual_events) = job.finish().unwrap();
            assert_eq!(actual, expected);
            assert_eq!(actual_events.payloads(), events.payloads());
        }
    }
    #[test]
    fn retreat_cursor_eligibility_disconnection_local_and_direct_path_axes() {
        for axis in 0..6 {
            let (authority, mut state) = fixture();
            match axis {
                0 => {
                    state
                        .fleets
                        .record_at_mut(0)
                        .unwrap()
                        .hulls
                        .record_at_mut(0)
                        .unwrap()
                        .return_credit = false
                }
                1 => {
                    state
                        .fleets
                        .record_at_mut(0)
                        .unwrap()
                        .hulls
                        .record_at_mut(0)
                        .unwrap()
                        .definition = LivingSlugV2::new("escort").unwrap()
                }
                2 => state.lanes = Default::default(),
                3 => {
                    let origin = match state.fleets.as_slice()[0].location {
                        LivingFleetLocationV2::Docked { world } => world,
                        _ => unreachable!(),
                    };
                    let system = state
                        .worlds
                        .as_slice()
                        .iter()
                        .find(|row| row.id == origin)
                        .unwrap()
                        .system;
                    let owner = state.fleets.as_slice()[0].owner;
                    let homes: Vec<_> = state
                        .colonies
                        .as_slice()
                        .iter()
                        .filter(|row| row.owner == owner)
                        .map(|row| row.world)
                        .collect();
                    for world in homes {
                        state.worlds.find_mut(&world).unwrap().system = system;
                    }
                }
                4 => {
                    let mut lanes = state.lanes.into_vec();
                    let systems = state.systems.as_slice();
                    lanes.push(LivingLaneV2 {
                        id: LivingEntityIdV2([5; 16]),
                        system_a: systems[0].id,
                        system_b: systems[3].id,
                        distance_units: 200,
                    });
                    lanes.sort_by_key(|row| row.id);
                    state.lanes = super::super::wire::LivingSortedVecV2::new(lanes).unwrap();
                }
                _ => state.wars = Default::default(),
            }
            let (expected, events) = reference(&authority, &state, LivingTickV2(10)).unwrap();
            let mut job = LivingRetreatJobV2::begin(
                authority.catalog(),
                state,
                LivingPendingEventsV2::default(),
                LivingTickV2(10),
            );
            while job.poll(1).unwrap() != LivingLifecycleProgressV2::Ready {
                assert!(job.last_poll_visits() <= 1);
            }
            let (actual, actual_events) = job.finish().unwrap();
            assert_eq!(actual, expected, "axis {axis}");
            assert_eq!(actual_events.payloads(), events.payloads(), "axis {axis}");
        }
    }
    #[test]
    fn retreat_cursor_long_paths_and_replaced_orders_count_every_element() {
        use super::super::wire::LivingSortedVecV2;
        use crate::living::LivingNameV2;
        let (authority, mut state) = fixture();
        let id = |prefix: u8, index: u8| {
            let mut bytes = [prefix; 16];
            bytes[15] = index;
            LivingEntityIdV2(bytes)
        };
        state.systems = LivingSortedVecV2::new(
            (0..64)
                .map(|index| LivingSystemV2 {
                    id: id(140, index),
                    name: LivingNameV2::new(format!("System {index}")).unwrap(),
                })
                .collect(),
        )
        .unwrap();
        state.lanes = LivingSortedVecV2::new(
            (0..63)
                .map(|index| LivingLaneV2 {
                    id: id(150, index),
                    system_a: id(140, index),
                    system_b: id(140, index + 1),
                    distance_units: 100,
                })
                .collect(),
        )
        .unwrap();
        let fleet = &state.fleets.as_slice()[0];
        let owner = fleet.owner;
        let origin = match fleet.location {
            LivingFleetLocationV2::Docked { world } => world,
            _ => unreachable!(),
        };
        state.worlds.find_mut(&origin).unwrap().system = id(140, 0);
        let homes: Vec<_> = state
            .colonies
            .as_slice()
            .iter()
            .filter(|row| row.owner == owner)
            .map(|row| row.world)
            .collect();
        for world in homes {
            state.worlds.find_mut(&world).unwrap().system = id(140, 63);
        }
        let (expected, events) = reference(&authority, &state, LivingTickV2(10)).unwrap();
        assert_eq!(
            expected.fleets.as_slice()[0]
                .order
                .as_ref()
                .unwrap()
                .lane_path
                .len(),
            63
        );
        let mut count = None;
        for old_path_length in [0, 512] {
            let mut source = state.clone();
            source.fleets.record_at_mut(0).unwrap().order = Some(LivingFleetOrderV2 {
                kind: LivingFleetOrderKindV2::Explore,
                target_world: None,
                not_before_tick: LivingTickV2(0),
                lane_path: vec![id(150, 0); old_path_length],
            });
            let mut job = LivingRetreatJobV2::begin(
                authority.catalog(),
                source,
                LivingPendingEventsV2::default(),
                LivingTickV2(10),
            );
            while job.poll(1).unwrap() != LivingLifecycleProgressV2::Ready {
                assert!(job.last_poll_visits() <= 1);
            }
            if let Some(count) = count {
                assert_eq!(job.total_visits(), count + 512);
            } else {
                count = Some(job.total_visits());
            }
            let (actual, actual_events) = job.finish().unwrap();
            assert_eq!(actual, expected);
            assert_eq!(actual_events.payloads(), events.payloads());
        }
    }
    #[test]
    fn retreat_cursor_cancel_and_deadline_fault_retain_unpublished_candidate() {
        let (authority, state) = fixture();
        let mut job = LivingRetreatJobV2::begin(
            authority.catalog(),
            state.clone(),
            LivingPendingEventsV2::default(),
            LivingTickV2(10),
        );
        assert_eq!(job.poll(0), Err(LivingLifecycleJobErrorV2::Budget));
        assert_eq!(job.poll(1025), Err(LivingLifecycleJobErrorV2::Budget));
        assert_eq!(job.last_poll_visits(), 0);
        while job.stage != RetreatStage::Search {
            job.poll(1).unwrap();
        }
        job.poll(1).unwrap();
        job.cancel();
        assert_eq!(job.poll(1), Err(LivingLifecycleJobErrorV2::Cancelled));
        assert!(matches!(
            job.finish(),
            Err(LivingLifecycleJobErrorV2::Cancelled)
        ));
        let tick = LivingTickV2(u64::MAX - 1);
        assert!(matches!(
            reference(&authority, &state, tick),
            Err(LivingDeterministicFaultV2::Arithmetic)
        ));
        let mut job = LivingRetreatJobV2::begin(
            authority.catalog(),
            state,
            LivingPendingEventsV2::default(),
            tick,
        );
        loop {
            match job.poll(1) {
                Ok(_) => {}
                Err(LivingLifecycleJobErrorV2::Fault(LivingDeterministicFaultV2::Arithmetic)) => {
                    break;
                }
                other => panic!("unexpected retreat result {other:?}"),
            }
        }
        assert!(
            job.winner.is_some(),
            "fault retains the completed itinerary for outside-poll reclamation"
        );
        assert_eq!(
            job.poll(64),
            Err(LivingLifecycleJobErrorV2::Fault(
                LivingDeterministicFaultV2::Arithmetic
            ))
        );
        assert_eq!(job.last_poll_visits(), 0);
        assert!(matches!(
            job.finish(),
            Err(LivingLifecycleJobErrorV2::Fault(
                LivingDeterministicFaultV2::Arithmetic
            ))
        ));
    }
}

#[derive(Clone, Copy, Eq, PartialEq)]
enum OccupationStage {
    Wars,
    Fleet,
    Hull,
    Facilities,
    Colonies,
    Occupations,
    Relations,
    Colony,
    Claimants,
    ThirdParties,
    Decide,
    ConstructionRefunds,
    HullRefunds,
    Relation,
    RemainingColonies,
    Capture,
    OccupationOutput,
    RelationOutput,
    Retire,
    Ready,
}
/// Bounded occupation/capture component. Refunds retain reference job order,
/// all-or-nothing storage credit and exact cancellation/dormancy event order.
pub struct LivingOccupationJobV2<'a> {
    delivered: bool,
    catalog: PhaseCatalogSlot<'a>,
    state: LivingGalaxyStateV2,
    events: LivingPendingEventsV2,
    tick: LivingTickV2,
    stage: OccupationStage,
    index: usize,
    fleet_index: usize,
    hull_index: usize,
    colony_index: usize,
    snapshot: Vec<(LivingEntityIdV2, LivingEntityIdV2)>,
    wars: BTreeSet<Pair>,
    escorts: BTreeSet<Pair>,
    living_escorts: BTreeSet<Pair>,
    batteries: BTreeSet<LivingEntityIdV2>,
    occupations: BTreeMap<LivingEntityIdV2, super::model::LivingOccupationV2>,
    relations: BTreeMap<Pair, super::model::LivingRelationV2>,
    occupation_source: IntoIter<super::model::LivingOccupationV2>,
    relation_source: IntoIter<super::model::LivingRelationV2>,
    construction: Option<IntoIter<super::model::LivingConstructionJobV2>>,
    hull_jobs: Option<IntoIter<super::model::LivingHullJobV2>>,
    world: LivingEntityIdV2,
    owner: LivingEntityIdV2,
    claimant: Option<LivingEntityIdV2>,
    claimants: usize,
    after: Option<LivingEntityIdV2>,
    contested: bool,
    remaining: bool,
    cancelled: bool,
    fault: Option<LivingDeterministicFaultV2>,
    last_poll_visits: usize,
    total_visits: u64,
}
impl<'a> LivingOccupationJobV2<'a> {
    /// Move the candidate and pending events without visiting records.
    pub fn begin(
        catalog: &'a super::catalog::ValidatedLivingCatalogPackV2,
        state: LivingGalaxyStateV2,
        events: LivingPendingEventsV2,
        tick: LivingTickV2,
    ) -> Self {
        Self::with_catalog(LivingPhaseCatalogV2::Borrowed(catalog), state, events, tick)
    }
    fn with_catalog(
        catalog: LivingPhaseCatalogV2<'a>,
        mut state: LivingGalaxyStateV2,
        events: LivingPendingEventsV2,
        tick: LivingTickV2,
    ) -> Self {
        let occupation_source = std::mem::take(&mut state.occupations)
            .into_vec()
            .into_iter();
        let relation_source = std::mem::take(&mut state.relations).into_vec().into_iter();
        Self {
            delivered: false,
            catalog: PhaseCatalogSlot(Some(catalog)),
            state,
            events,
            tick,
            stage: OccupationStage::Wars,
            index: 0,
            fleet_index: 0,
            hull_index: 0,
            colony_index: 0,
            snapshot: Vec::new(),
            wars: BTreeSet::new(),
            escorts: BTreeSet::new(),
            living_escorts: BTreeSet::new(),
            batteries: BTreeSet::new(),
            occupations: BTreeMap::new(),
            relations: BTreeMap::new(),
            occupation_source,
            relation_source,
            construction: None,
            hull_jobs: None,
            world: LivingEntityIdV2([0; 16]),
            owner: LivingEntityIdV2([0; 16]),
            claimant: None,
            claimants: 0,
            after: None,
            contested: false,
            remaining: false,
            cancelled: false,
            fault: None,
            last_poll_visits: 0,
            total_visits: 0,
        }
    }
    /// Cancel private work without publication.
    pub fn cancel(&mut self) {
        self.cancelled = true;
    }
    /// Actual records consumed by the preceding poll.
    pub const fn last_poll_visits(&self) -> usize {
        self.last_poll_visits
    }
    /// Actual records consumed across polls.
    pub const fn total_visits(&self) -> u64 {
        self.total_visits
    }
    /// Continue with an actual-record budget.
    pub fn poll(
        &mut self,
        budget: usize,
    ) -> Result<LivingLifecycleProgressV2, LivingLifecycleJobErrorV2> {
        self.last_poll_visits = 0;
        if self.cancelled {
            return Err(LivingLifecycleJobErrorV2::Cancelled);
        }
        if budget == 0 || budget > LIVING_MAX_PHASE_POLL_VISITS_V2 {
            return Err(LivingLifecycleJobErrorV2::Budget);
        }
        if let Some(fault) = &self.fault {
            return Err(LivingLifecycleJobErrorV2::Fault(fault.clone()));
        }
        while self.last_poll_visits < budget && self.stage != OccupationStage::Ready {
            match self.visit() {
                Ok(true) => {
                    self.last_poll_visits += 1;
                    self.total_visits += 1
                }
                Ok(false) => {}
                Err(fault) => {
                    self.last_poll_visits += 1;
                    self.total_visits += 1;
                    self.fault = Some(fault.clone());
                    return Err(LivingLifecycleJobErrorV2::Fault(fault));
                }
            }
        }
        Ok(if self.stage == OccupationStage::Ready {
            LivingLifecycleProgressV2::Ready
        } else {
            LivingLifecycleProgressV2::Pending
        })
    }
    /// Move the completed candidate to settlement processing.
    pub fn finish(
        mut self,
    ) -> Result<(LivingGalaxyStateV2, LivingPendingEventsV2), LivingLifecycleJobErrorV2> {
        self.take_ready_result()
    }
    /// Borrowed one-time extraction; rejected calls preserve every owner.
    pub fn take_ready_result(
        &mut self,
    ) -> Result<(LivingGalaxyStateV2, LivingPendingEventsV2), LivingLifecycleJobErrorV2> {
        if self.cancelled {
            return Err(LivingLifecycleJobErrorV2::Cancelled);
        }
        if let Some(fault) = self.fault.clone() {
            return Err(LivingLifecycleJobErrorV2::Fault(fault));
        }
        if self.stage != OccupationStage::Ready {
            return Err(LivingLifecycleJobErrorV2::NotReady);
        }
        if self.delivered {
            return Err(LivingLifecycleJobErrorV2::NotReady);
        }
        self.delivered = true;
        Ok((
            std::mem::take(&mut self.state),
            std::mem::take(&mut self.events),
        ))
    }
    fn emit(&mut self, kind: LivingEventKindV2) -> Result<(), LivingDeterministicFaultV2> {
        self.events.record(
            LivingEventProvenanceV2::Autonomous {
                phase: LivingPhaseV2::ResolveConflict,
            },
            kind,
        )?;
        Ok(())
    }
    fn advance(&mut self, stage: OccupationStage) {
        self.stage = stage;
        self.index = 0;
    }
    fn hostile(&self, left: LivingEntityIdV2, right: LivingEntityIdV2) -> bool {
        self.wars.contains(&(left.min(right), left.max(right)))
    }
    fn next_escort(&self) -> Option<LivingEntityIdV2> {
        use std::ops::Bound::*;
        let lower = match self.after {
            Some(actor) => Excluded((self.world, actor)),
            None => Included((self.world, LivingEntityIdV2([0; 16]))),
        };
        self.escorts
            .range((lower, Included((self.world, LivingEntityIdV2([255; 16])))))
            .next()
            .map(|(_, actor)| *actor)
    }
    fn credit_refund(&mut self, paid: u32) {
        if let Some(world) = self.state.worlds.find_mut(&self.world)
            && let Some(next) = world.inventory.alloy.checked_add(paid / 2)
            && next <= super::catalog::LIVING_RESOURCE_STORAGE_LIMIT_V2
        {
            world.inventory.alloy = next;
        }
    }
    fn visit(&mut self) -> Result<bool, LivingDeterministicFaultV2> {
        use super::model::{
            LIVING_OCCUPATION_TRANSFER_TICKS_V2, LivingCivilizationStatusV2, LivingFleetLocationV2,
            LivingOccupationV2, LivingRelationV2,
        };
        use OccupationStage::*;
        match self.stage {
            Wars => {
                if let Some(row) = self.state.wars.as_slice().get(self.index) {
                    if row.start_tick.0 <= self.tick.0 && self.tick.0 < row.end_tick.0 {
                        self.wars.insert((row.participant_a, row.participant_b));
                    }
                    self.index += 1;
                    return Ok(true);
                }
                self.advance(Fleet);
            }
            Fleet => {
                if let Some(row) = self.state.fleets.as_slice().get(self.fleet_index) {
                    if matches!(row.location, LivingFleetLocationV2::Docked { .. }) {
                        self.hull_index = 0;
                        self.stage = Hull;
                    } else {
                        self.fleet_index += 1;
                    }
                    return Ok(true);
                }
                self.advance(Facilities);
            }
            Hull => {
                let fleet = &self.state.fleets.as_slice()[self.fleet_index];
                let LivingFleetLocationV2::Docked { world } = fleet.location else {
                    return Err(LivingDeterministicFaultV2::Invariant);
                };
                if let Some(hull) = fleet.hulls.as_slice().get(self.hull_index) {
                    if hull.definition.as_str() == "escort" {
                        self.escorts.insert((world, fleet.owner));
                        if hull.hit_points > 0 {
                            self.living_escorts.insert((world, fleet.owner));
                        }
                    }
                    self.hull_index += 1;
                    return Ok(true);
                }
                self.fleet_index += 1;
                self.stage = Fleet;
            }
            Facilities => {
                if let Some(row) = self.state.facilities.as_slice().get(self.index) {
                    if row.hit_points > 0
                        && !row.disabled
                        && self.catalog.industry(row.definition.as_str()).is_some_and(
                            |definition| {
                                matches!(
                                    definition.effect,
                                    super::catalog::LivingIndustryEffectV2::Defense { .. }
                                )
                            },
                        )
                    {
                        self.batteries.insert(row.world);
                    }
                    self.index += 1;
                    return Ok(true);
                }
                self.advance(Colonies);
            }
            Colonies => {
                if let Some(row) = self.state.colonies.as_slice().get(self.index) {
                    self.snapshot.push((row.world, row.owner));
                    self.index += 1;
                    return Ok(true);
                }
                self.advance(Occupations);
            }
            Occupations => {
                if let Some(row) = self.occupation_source.next() {
                    self.occupations.insert(row.world, row);
                    return Ok(true);
                }
                self.advance(Relations);
            }
            Relations => {
                if let Some(row) = self.relation_source.next() {
                    self.relations.insert((row.from, row.to), row);
                    return Ok(true);
                }
                self.advance(Colony);
            }
            Colony => {
                if let Some((world, owner)) = self.snapshot.get(self.colony_index).copied() {
                    self.world = world;
                    self.owner = owner;
                    self.claimant = None;
                    self.claimants = 0;
                    self.after = None;
                    self.contested = self.living_escorts.contains(&(world, owner))
                        || self.batteries.contains(&world);
                    self.stage = Claimants;
                    return Ok(true);
                }
                self.advance(OccupationOutput);
            }
            Claimants => {
                if let Some(actor) = self.next_escort() {
                    self.after = Some(actor);
                    if actor != self.owner && self.hostile(actor, self.owner) {
                        self.claimants += 1;
                        self.claimant = Some(actor);
                    }
                    return Ok(true);
                }
                if self.claimants != 1 || self.contested {
                    self.contested = true;
                    self.stage = Decide;
                } else {
                    self.after = None;
                    self.stage = ThirdParties;
                }
            }
            ThirdParties => {
                if let Some(actor) = self.next_escort() {
                    self.after = Some(actor);
                    let claimant = self.claimant.expect("single claimant exists");
                    if actor != claimant && actor != self.owner && self.hostile(actor, claimant) {
                        self.contested = true;
                    }
                    return Ok(true);
                }
                self.stage = Decide;
            }
            Decide => {
                if self.contested {
                    self.occupations.remove(&self.world);
                    self.colony_index += 1;
                    self.stage = Colony;
                    return Ok(true);
                }
                let claimant = self.claimant.expect("single uncontested claimant exists");
                let progress = self
                    .occupations
                    .get(&self.world)
                    .filter(|row| row.claimant == claimant)
                    .map(|row| row.progress_ticks)
                    .unwrap_or(0)
                    .saturating_add(1);
                if progress < LIVING_OCCUPATION_TRANSFER_TICKS_V2 {
                    self.occupations.insert(
                        self.world,
                        LivingOccupationV2 {
                            world: self.world,
                            claimant,
                            progress_ticks: progress,
                        },
                    );
                    self.colony_index += 1;
                    self.stage = Colony;
                } else {
                    if let Some(world) = self.state.worlds.find_mut(&self.world) {
                        world.owner = Some(claimant)
                    }
                    if let Some(colony) = self.state.colonies.find_mut(&self.world) {
                        colony.owner = claimant
                    }
                    self.construction = Some(
                        std::mem::take(&mut self.state.construction_jobs)
                            .into_vec()
                            .into_iter(),
                    );
                    self.stage = ConstructionRefunds;
                }
                return Ok(true);
            }
            ConstructionRefunds => {
                if let Some(row) = self
                    .construction
                    .as_mut()
                    .expect("construction refunds initialized")
                    .next()
                {
                    if row.world == self.world && row.owner == self.owner {
                        self.credit_refund(row.paid_alloy);
                        self.emit(LivingEventKindV2::ConstructionCancelled)?;
                    } else {
                        self.state.construction_jobs.push_sorted(row)?;
                    }
                    return Ok(true);
                }
                self.construction = None;
                self.hull_jobs = Some(
                    std::mem::take(&mut self.state.hull_jobs)
                        .into_vec()
                        .into_iter(),
                );
                self.stage = HullRefunds;
            }
            HullRefunds => {
                if let Some(row) = self
                    .hull_jobs
                    .as_mut()
                    .expect("hull refunds initialized")
                    .next()
                {
                    if row.world == self.world && row.owner == self.owner {
                        self.credit_refund(row.paid_alloy);
                    } else {
                        self.state.hull_jobs.push_sorted(row)?;
                    }
                    return Ok(true);
                }
                self.hull_jobs = None;
                self.stage = Relation;
            }
            Relation => {
                let claimant = self.claimant.expect("claimant exists");
                let row =
                    self.relations
                        .entry((self.owner, claimant))
                        .or_insert(LivingRelationV2 {
                            from: self.owner,
                            to: claimant,
                            base_disposition: 0,
                            reasons: Default::default(),
                        });
                row.reasons.colony_captured_from_it =
                    (row.reasons.colony_captured_from_it - 30).max(-60);
                self.remaining = false;
                self.advance(RemainingColonies);
                return Ok(true);
            }
            RemainingColonies => {
                if let Some(row) = self.state.colonies.as_slice().get(self.index) {
                    self.index += 1;
                    if row.owner == self.owner {
                        self.remaining = true;
                        self.stage = Capture;
                    }
                    return Ok(true);
                }
                self.stage = Capture;
            }
            Capture => {
                if !self.remaining {
                    if let Some(civ) = self.state.civilizations.find_mut(&self.owner) {
                        civ.status = LivingCivilizationStatusV2::Dormant;
                    }
                    self.emit(LivingEventKindV2::CivilizationDormant)?;
                }
                if let Some(civ) = self
                    .state
                    .civilizations
                    .find_mut(&self.claimant.expect("claimant exists"))
                {
                    civ.status = LivingCivilizationStatusV2::Active;
                }
                self.emit(LivingEventKindV2::ColonyCaptured)?;
                self.occupations.remove(&self.world);
                self.colony_index += 1;
                self.stage = Colony;
                return Ok(true);
            }
            OccupationOutput => {
                if let Some((_, row)) = self.occupations.pop_first() {
                    self.state.occupations.push_sorted(row)?;
                    return Ok(true);
                }
                self.advance(RelationOutput);
            }
            RelationOutput => {
                if let Some((_, row)) = self.relations.pop_first() {
                    self.state.relations.push_sorted(row)?;
                    return Ok(true);
                }
                self.advance(Retire);
            }
            Retire => {
                if self.snapshot.pop().is_some()
                    || self.wars.pop_first().is_some()
                    || self.escorts.pop_first().is_some()
                    || self.living_escorts.pop_first().is_some()
                    || self.batteries.pop_first().is_some()
                {
                    return Ok(true);
                }
                self.advance(Ready);
            }
            Ready => {}
        }
        Ok(false)
    }
}
impl LivingOccupationJobV2<'static> {
    /// Begin with a moved immutable catalog handle.
    pub fn begin_owned(
        catalog: std::sync::Arc<super::catalog::ValidatedLivingCatalogPackV2>,
        state: LivingGalaxyStateV2,
        events: LivingPendingEventsV2,
        tick: LivingTickV2,
    ) -> Self {
        Self::with_catalog(LivingPhaseCatalogV2::Owned(catalog), state, events, tick)
    }
}

#[derive(Clone, Copy, Eq, PartialEq)]
enum SettlementStage {
    Fleet,
    Hull,
    Facilities,
    Claims,
    Colonies,
    World,
    Arbitrate,
    ConsumeFleet,
    ConsumeHull,
    RetireFleetOrder,
    Establish,
    RetireColonyReasons,
    Reactivate,
    ClearClaims,
    Resolved,
    ArrivalsStart,
    Arrival,
    ClaimOutput,
    ColonyOutput,
    Retire,
    Ready,
}
/// Bounded settlement arbitration and arrival-claim component. It preserves
/// stable fleet/hull ark consumption and each world's reference window snapshot.
pub struct LivingSettlementJobV2<'a> {
    delivered: bool,
    catalog: PhaseCatalogSlot<'a>,
    state: LivingGalaxyStateV2,
    events: LivingPendingEventsV2,
    tick: LivingTickV2,
    seed: [u8; 32],
    arrivals: Vec<LivingEntityIdV2>,
    stage: SettlementStage,
    index: usize,
    fleet_index: usize,
    hull_index: usize,
    world_index: usize,
    arrival_index: usize,
    armed: BTreeSet<LivingEntityIdV2>,
    docked: BTreeMap<LivingEntityIdV2, (LivingEntityIdV2, LivingEntityIdV2, bool)>,
    fleet_arks: BTreeMap<LivingEntityIdV2, u32>,
    arks: BTreeMap<Pair, BTreeSet<Pair>>,
    claims: BTreeMap<Pair, super::model::LivingSettlementClaimV2>,
    colonies: BTreeMap<LivingEntityIdV2, super::model::LivingColonyV2>,
    claim_source: IntoIter<super::model::LivingSettlementClaimV2>,
    colony_source: IntoIter<super::model::LivingColonyV2>,
    world: LivingEntityIdV2,
    close: Option<LivingTickV2>,
    after: Option<LivingEntityIdV2>,
    winner: Option<([u8; 32], LivingEntityIdV2)>,
    consume_fleet: LivingEntityIdV2,
    consume_hull: LivingEntityIdV2,
    fleet_source: Option<IntoIter<super::model::LivingFleetV2>>,
    fleet: Option<super::model::LivingFleetV2>,
    hulls: Option<IntoIter<super::model::LivingHullV2>>,
    retired_path: Vec<LivingEntityIdV2>,
    retired_reasons: Option<IntoIter<super::model::LivingBlockedReasonV2>>,
    clear_then: SettlementStage,
    cancelled: bool,
    fault: Option<LivingDeterministicFaultV2>,
    last_poll_visits: usize,
    total_visits: u64,
}
impl<'a> LivingSettlementJobV2<'a> {
    /// Move a candidate and the ordered phase-three arrival IDs.
    pub fn begin(
        catalog: &'a super::catalog::ValidatedLivingCatalogPackV2,
        state: LivingGalaxyStateV2,
        events: LivingPendingEventsV2,
        tick: LivingTickV2,
        seed: [u8; 32],
        arrivals: Vec<LivingEntityIdV2>,
    ) -> Self {
        Self::with_catalog(
            LivingPhaseCatalogV2::Borrowed(catalog),
            state,
            events,
            tick,
            seed,
            arrivals,
        )
    }
    fn with_catalog(
        catalog: LivingPhaseCatalogV2<'a>,
        mut state: LivingGalaxyStateV2,
        events: LivingPendingEventsV2,
        tick: LivingTickV2,
        seed: [u8; 32],
        arrivals: Vec<LivingEntityIdV2>,
    ) -> Self {
        let claim_source = std::mem::take(&mut state.settlement_claims)
            .into_vec()
            .into_iter();
        let colony_source = std::mem::take(&mut state.colonies).into_vec().into_iter();
        Self {
            delivered: false,
            catalog: PhaseCatalogSlot(Some(catalog)),
            state,
            events,
            tick,
            seed,
            arrivals,
            stage: SettlementStage::Fleet,
            index: 0,
            fleet_index: 0,
            hull_index: 0,
            world_index: 0,
            arrival_index: 0,
            armed: BTreeSet::new(),
            docked: BTreeMap::new(),
            fleet_arks: BTreeMap::new(),
            arks: BTreeMap::new(),
            claims: BTreeMap::new(),
            colonies: BTreeMap::new(),
            claim_source,
            colony_source,
            world: LivingEntityIdV2([0; 16]),
            close: None,
            after: None,
            winner: None,
            consume_fleet: LivingEntityIdV2([0; 16]),
            consume_hull: LivingEntityIdV2([0; 16]),
            fleet_source: None,
            fleet: None,
            hulls: None,
            retired_path: Vec::new(),
            retired_reasons: None,
            clear_then: SettlementStage::World,
            cancelled: false,
            fault: None,
            last_poll_visits: 0,
            total_visits: 0,
        }
    }
    /// Cancel without publishing a candidate.
    pub fn cancel(&mut self) {
        self.cancelled = true;
    }
    /// Actual records consumed by the preceding poll.
    pub const fn last_poll_visits(&self) -> usize {
        self.last_poll_visits
    }
    /// Actual records consumed across polls.
    pub const fn total_visits(&self) -> u64 {
        self.total_visits
    }
    /// Continue with 1..=1024 actual record visits.
    pub fn poll(
        &mut self,
        budget: usize,
    ) -> Result<LivingLifecycleProgressV2, LivingLifecycleJobErrorV2> {
        self.last_poll_visits = 0;
        if self.cancelled {
            return Err(LivingLifecycleJobErrorV2::Cancelled);
        }
        if budget == 0 || budget > LIVING_MAX_PHASE_POLL_VISITS_V2 {
            return Err(LivingLifecycleJobErrorV2::Budget);
        }
        if let Some(fault) = &self.fault {
            return Err(LivingLifecycleJobErrorV2::Fault(fault.clone()));
        }
        while self.last_poll_visits < budget && self.stage != SettlementStage::Ready {
            match self.visit() {
                Ok(true) => {
                    self.last_poll_visits += 1;
                    self.total_visits += 1
                }
                Ok(false) => {}
                Err(fault) => {
                    self.last_poll_visits += 1;
                    self.total_visits += 1;
                    self.fault = Some(fault.clone());
                    return Err(LivingLifecycleJobErrorV2::Fault(fault));
                }
            }
        }
        Ok(if self.stage == SettlementStage::Ready {
            LivingLifecycleProgressV2::Ready
        } else {
            LivingLifecycleProgressV2::Pending
        })
    }
    /// Move the completed candidate onward, without publication.
    pub fn finish(
        mut self,
    ) -> Result<(LivingGalaxyStateV2, LivingPendingEventsV2), LivingLifecycleJobErrorV2> {
        self.take_ready_result()
    }
    /// Borrowed one-time extraction; rejected calls preserve every owner.
    pub fn take_ready_result(
        &mut self,
    ) -> Result<(LivingGalaxyStateV2, LivingPendingEventsV2), LivingLifecycleJobErrorV2> {
        if self.cancelled {
            return Err(LivingLifecycleJobErrorV2::Cancelled);
        }
        if let Some(fault) = self.fault.clone() {
            return Err(LivingLifecycleJobErrorV2::Fault(fault));
        }
        if self.stage != SettlementStage::Ready {
            return Err(LivingLifecycleJobErrorV2::NotReady);
        }
        if self.delivered {
            return Err(LivingLifecycleJobErrorV2::NotReady);
        }
        self.delivered = true;
        Ok((
            std::mem::take(&mut self.state),
            std::mem::take(&mut self.events),
        ))
    }
    fn emit(&mut self, kind: LivingEventKindV2) -> Result<(), LivingDeterministicFaultV2> {
        self.events.record(
            LivingEventProvenanceV2::Autonomous {
                phase: LivingPhaseV2::ResolveConflict,
            },
            kind,
        )?;
        Ok(())
    }
    fn first_claim(&self) -> Option<&super::model::LivingSettlementClaimV2> {
        self.claims
            .range(
                (self.world, LivingEntityIdV2([0; 16]))..=(self.world, LivingEntityIdV2([255; 16])),
            )
            .next()
            .map(|(_, row)| row)
    }
    fn next_claim(&self) -> Option<super::model::LivingSettlementClaimV2> {
        use std::ops::Bound::*;
        let lower = match self.after {
            Some(actor) => Excluded((self.world, actor)),
            None => Included((self.world, LivingEntityIdV2([0; 16]))),
        };
        self.claims
            .range((lower, Included((self.world, LivingEntityIdV2([255; 16])))))
            .next()
            .map(|(_, row)| *row)
    }
    fn clear(&mut self, then: SettlementStage) {
        self.clear_then = then;
        self.stage = SettlementStage::ClearClaims;
    }
    fn due(&self, period: u64) -> Result<LivingTickV2, LivingDeterministicFaultV2> {
        self.tick
            .0
            .checked_add(period)
            .map(LivingTickV2)
            .ok_or(LivingDeterministicFaultV2::Arithmetic)
    }
    fn visit(&mut self) -> Result<bool, LivingDeterministicFaultV2> {
        use super::model::{
            LivingCivilizationStatusV2, LivingColonyV2, LivingFleetLocationV2,
            LivingFleetOrderKindV2, LivingHubV2, LivingInventoryV2, LivingSettlementClaimV2,
        };
        use SettlementStage::*;
        match self.stage {
            Fleet => {
                if let Some(fleet) = self.state.fleets.as_slice().get(self.fleet_index) {
                    if let LivingFleetLocationV2::Docked { world } = fleet.location {
                        self.docked.insert(
                            fleet.id,
                            (
                                world,
                                fleet.owner,
                                fleet.order.as_ref().is_some_and(|order| {
                                    order.kind == LivingFleetOrderKindV2::Settle
                                }),
                            ),
                        );
                        self.hull_index = 0;
                        self.stage = Hull;
                    } else {
                        self.fleet_index += 1;
                    }
                    return Ok(true);
                }
                self.index = 0;
                self.stage = Facilities;
            }
            Hull => {
                let fleet = &self.state.fleets.as_slice()[self.fleet_index];
                let LivingFleetLocationV2::Docked { world } = fleet.location else {
                    return Err(LivingDeterministicFaultV2::Invariant);
                };
                if let Some(hull) = fleet.hulls.as_slice().get(self.hull_index) {
                    if hull.hit_points > 0
                        && self
                            .catalog
                            .hull(hull.definition.as_str())
                            .is_some_and(|row| row.damage > 0)
                    {
                        self.armed.insert(world);
                    }
                    if hull.definition.as_str() == "colony_ark" {
                        self.arks
                            .entry((world, fleet.owner))
                            .or_default()
                            .insert((fleet.id, hull.id));
                        *self.fleet_arks.entry(fleet.id).or_default() += 1;
                    }
                    self.hull_index += 1;
                    return Ok(true);
                }
                self.fleet_index += 1;
                self.stage = Fleet;
            }
            Facilities => {
                if let Some(row) = self.state.facilities.as_slice().get(self.index) {
                    if row.hit_points > 0
                        && !row.disabled
                        && self.catalog.industry(row.definition.as_str()).is_some_and(
                            |definition| {
                                matches!(
                                    definition.effect,
                                    super::catalog::LivingIndustryEffectV2::Defense { .. }
                                )
                            },
                        )
                    {
                        self.armed.insert(row.world);
                    }
                    self.index += 1;
                    return Ok(true);
                }
                self.stage = Claims;
            }
            Claims => {
                if let Some(row) = self.claim_source.next() {
                    self.claims.insert((row.world, row.civilization), row);
                    return Ok(true);
                }
                self.stage = Colonies;
            }
            Colonies => {
                if let Some(row) = self.colony_source.next() {
                    self.colonies.insert(row.world, row);
                    return Ok(true);
                }
                self.stage = World;
            }
            World => {
                if let Some(row) = self.state.worlds.as_slice().get(self.world_index) {
                    self.world_index += 1;
                    self.world = row.id;
                    self.close = self.first_claim().map(|claim| claim.close_tick);
                    self.winner = None;
                    self.after = None;
                    if self.armed.contains(&self.world) || row.owner.is_some() {
                        self.clear(World);
                    } else if self.close == Some(self.tick) {
                        self.stage = Arbitrate;
                    } else {
                        self.stage = ArrivalsStart;
                    }
                    return Ok(true);
                }
                self.stage = ClaimOutput;
            }
            Arbitrate => {
                if let Some(claim) = self.next_claim() {
                    self.after = Some(claim.civilization);
                    if Some(claim.close_tick) == self.close
                        && self
                            .arks
                            .get(&(self.world, claim.civilization))
                            .is_some_and(|set| !set.is_empty())
                    {
                        let rank = super::ids::claim_rank_v2(
                            self.seed,
                            self.world,
                            claim.close_tick,
                            claim.civilization,
                        );
                        let candidate = (rank, claim.civilization);
                        if self.winner.is_none_or(|winner| candidate < winner) {
                            self.winner = Some(candidate);
                        }
                    }
                    return Ok(true);
                }
                if let Some((_, actor)) = self.winner {
                    let set = self
                        .arks
                        .get_mut(&(self.world, actor))
                        .expect("winning ark exists");
                    let (fleet, hull) = set.pop_first().expect("winning ark exists");
                    if set.is_empty() {
                        self.arks.remove(&(self.world, actor));
                    }
                    self.consume_fleet = fleet;
                    self.consume_hull = hull;
                    let count = self.fleet_arks.get_mut(&fleet).expect("ark count exists");
                    *count -= 1;
                    if *count == 0 {
                        self.fleet_arks.remove(&fleet);
                    }
                    self.fleet_source = Some(
                        std::mem::take(&mut self.state.fleets)
                            .into_vec()
                            .into_iter(),
                    );
                    self.stage = ConsumeFleet;
                } else {
                    self.clear(ArrivalsStart);
                }
                return Ok(true);
            }
            ConsumeFleet => {
                if let Some(fleet) = self
                    .fleet_source
                    .as_ref()
                    .and_then(|rows| rows.as_slice().first())
                    && fleet.id != self.consume_fleet
                {
                    ensure_retained_sorted(&self.state.fleets, fleet)?;
                }
                if let Some(mut fleet) = self
                    .fleet_source
                    .as_mut()
                    .expect("fleet consumption initialized")
                    .next()
                {
                    if fleet.id == self.consume_fleet {
                        self.hulls = Some(std::mem::take(&mut fleet.hulls).into_vec().into_iter());
                        self.fleet = Some(fleet);
                        self.stage = ConsumeHull;
                    } else {
                        self.state.fleets.push_sorted(fleet)?;
                    }
                    return Ok(true);
                }
                self.fleet_source = None;
                self.stage = Establish;
            }
            ConsumeHull => {
                if let Some(hull) = self
                    .hulls
                    .as_mut()
                    .expect("hull consumption initialized")
                    .next()
                {
                    if hull.id != self.consume_hull {
                        self.fleet
                            .as_mut()
                            .expect("held fleet exists")
                            .hulls
                            .push_sorted(hull)?;
                    }
                    return Ok(true);
                }
                self.hulls = None;
                if self
                    .fleet
                    .as_ref()
                    .expect("held fleet exists")
                    .hulls
                    .as_slice()
                    .is_empty()
                {
                    let fleet = self.fleet.as_mut().expect("held fleet exists");
                    if let Some(order) = fleet.order.take() {
                        self.retired_path = order.lane_path;
                    }
                    self.docked.remove(&fleet.id);
                    self.stage = RetireFleetOrder;
                } else {
                    ensure_retained_sorted(
                        &self.state.fleets,
                        self.fleet.as_ref().expect("held fleet exists"),
                    )?;
                    self.state
                        .fleets
                        .push_sorted(self.fleet.take().expect("held fleet exists"))?;
                    self.stage = ConsumeFleet;
                    return Ok(true);
                }
            }
            RetireFleetOrder => {
                if self.retired_path.pop().is_some() {
                    return Ok(true);
                }
                self.fleet = None;
                self.stage = ConsumeFleet;
            }
            Establish => {
                let actor = self.winner.expect("winner exists").1;
                if let Some(world) = self.state.worlds.find_mut(&self.world) {
                    world.owner = Some(actor);
                    world.inventory = LivingInventoryV2 {
                        energy: 20,
                        ore: 20,
                        alloy: 0,
                    };
                }
                let colony = LivingColonyV2 {
                    world: self.world,
                    owner: actor,
                    hub: LivingHubV2 {
                        energy_next_due: self
                            .due(super::genesis::LIVING_HUB_ENERGY_PERIOD_TICKS_V2)?,
                        ore_next_due: self.due(super::genesis::LIVING_HUB_ORE_PERIOD_TICKS_V2)?,
                        fallback_next_due: self
                            .due(super::genesis::LIVING_HUB_FALLBACK_PERIOD_TICKS_V2)?,
                        blocked_reasons: Default::default(),
                    },
                };
                if let Some(old) = self.colonies.insert(self.world, colony) {
                    self.retired_reasons = Some(old.hub.blocked_reasons.into_vec().into_iter());
                }
                self.stage = RetireColonyReasons;
                return Ok(true);
            }
            RetireColonyReasons => {
                if self
                    .retired_reasons
                    .as_mut()
                    .and_then(Iterator::next)
                    .is_some()
                {
                    return Ok(true);
                }
                self.retired_reasons = None;
                self.stage = Reactivate;
            }
            Reactivate => {
                let actor = self.winner.expect("winner exists").1;
                if let Some(civ) = self.state.civilizations.find_mut(&actor)
                    && civ.status == LivingCivilizationStatusV2::Dormant
                {
                    civ.status = LivingCivilizationStatusV2::Active;
                    self.emit(LivingEventKindV2::CivilizationReactivated)?;
                }
                self.clear(Resolved);
                return Ok(true);
            }
            ClearClaims => {
                if let Some(key) = self
                    .first_claim()
                    .map(|claim| (claim.world, claim.civilization))
                {
                    self.claims.remove(&key);
                    return Ok(true);
                }
                self.stage = self.clear_then;
            }
            Resolved => {
                self.emit(LivingEventKindV2::SettlementClaimResolved)?;
                self.stage = World;
                return Ok(true);
            }
            ArrivalsStart => {
                self.close = self.first_claim().map(|claim| claim.close_tick);
                self.arrival_index = 0;
                self.stage = Arrival;
            }
            Arrival => {
                if let Some(id) = self.arrivals.get(self.arrival_index).copied() {
                    self.arrival_index += 1;
                    if let Some((world, actor, true)) = self.docked.get(&id).copied()
                        && world == self.world
                        && self.fleet_arks.contains_key(&id)
                    {
                        match self.close {
                            Some(close) if self.tick.0 < close.0 => {
                                self.claims.entry((self.world, actor)).or_insert(
                                    LivingSettlementClaimV2 {
                                        world: self.world,
                                        civilization: actor,
                                        arrival_tick: self.tick,
                                        close_tick: close,
                                    },
                                );
                            }
                            None => {
                                let close_tick = self.due(50)?;
                                self.claims.insert(
                                    (self.world, actor),
                                    LivingSettlementClaimV2 {
                                        world: self.world,
                                        civilization: actor,
                                        arrival_tick: self.tick,
                                        close_tick,
                                    },
                                );
                                self.emit(LivingEventKindV2::SettlementClaimOpened)?;
                            }
                            _ => {}
                        }
                    }
                    return Ok(true);
                }
                self.stage = World;
            }
            ClaimOutput => {
                if let Some((_, row)) = self.claims.pop_first() {
                    self.state.settlement_claims.push_sorted(row)?;
                    return Ok(true);
                }
                self.stage = ColonyOutput;
            }
            ColonyOutput => {
                if let Some((_, row)) = self.colonies.first_key_value() {
                    ensure_retained_sorted(&self.state.colonies, row)?;
                }
                if let Some((_, row)) = self.colonies.pop_first() {
                    self.state.colonies.push_sorted(row)?;
                    return Ok(true);
                }
                self.stage = Retire;
            }
            Retire => {
                if let Some(mut entry) = self.arks.first_entry() {
                    if entry.get_mut().pop_first().is_some() {
                        return Ok(true);
                    }
                    entry.remove();
                    return Ok(true);
                }
                if self.armed.pop_first().is_some()
                    || self.docked.pop_first().is_some()
                    || self.fleet_arks.pop_first().is_some()
                    || self.arrivals.pop().is_some()
                {
                    return Ok(true);
                }
                self.stage = Ready;
            }
            Ready => {}
        }
        Ok(false)
    }
}
impl LivingSettlementJobV2<'static> {
    /// Begin with a moved immutable owned catalog handle.
    pub fn begin_owned(
        catalog: std::sync::Arc<super::catalog::ValidatedLivingCatalogPackV2>,
        state: LivingGalaxyStateV2,
        events: LivingPendingEventsV2,
        tick: LivingTickV2,
        seed: [u8; 32],
        arrivals: Vec<LivingEntityIdV2>,
    ) -> Self {
        Self::with_catalog(
            LivingPhaseCatalogV2::Owned(catalog),
            state,
            events,
            tick,
            seed,
            arrivals,
        )
    }
}

#[cfg(test)]
mod occupation_settlement_tests {
    use super::*;
    use crate::living::model::*;
    use crate::living::simulation::{LivingGalaxyAuthorityV2, LivingStepContextV2};
    use crate::living::wire::LivingSortedVecV2;
    use crate::living::{LivingSlugV2, builtin_living_sandbox_v2};
    fn occupation_fixture() -> (LivingGalaxyAuthorityV2, LivingGalaxyStateV2) {
        let authority = builtin_living_sandbox_v2().unwrap();
        let mut state = authority.state().clone();
        let a = state.civilizations.as_slice()[0].id;
        let b = state.civilizations.as_slice()[1].id;
        let homes: Vec<_> = state
            .colonies
            .as_slice()
            .iter()
            .filter(|row| row.owner == a)
            .map(|row| row.world)
            .collect();
        let template = state.fleets.as_slice()[0].clone();
        let mut fleets = Vec::new();
        let mut occupations = Vec::new();
        for (index, world) in homes.iter().copied().enumerate() {
            let mut fleet = template.clone();
            fleet.id = LivingEntityIdV2([200 + index as u8; 16]);
            fleet.owner = b;
            fleet.location = LivingFleetLocationV2::Docked { world };
            let hull = LivingHullV2 {
                id: LivingEntityIdV2([210 + index as u8; 16]),
                definition: LivingSlugV2::new("escort").unwrap(),
                hit_points: 10,
                return_credit: false,
                next_repair: None,
            };
            fleet.hulls = LivingSortedVecV2::new(vec![hull]).unwrap();
            fleets.push(fleet);
            occupations.push(LivingOccupationV2 {
                world,
                claimant: b,
                progress_ticks: 99,
            });
        }
        state.fleets = LivingSortedVecV2::new(fleets).unwrap();
        state.occupations = LivingSortedVecV2::new(occupations).unwrap();
        state.wars = LivingSortedVecV2::new(vec![LivingWarV2 {
            participant_a: a.min(b),
            participant_b: a.max(b),
            declarer: a,
            start_tick: LivingTickV2(0),
            end_tick: LivingTickV2(1000),
        }])
        .unwrap();
        let world = homes[0];
        state.worlds.find_mut(&world).unwrap().inventory.alloy = 9990;
        state.construction_jobs = LivingSortedVecV2::new(
            [40, 10]
                .into_iter()
                .enumerate()
                .map(|(index, paid_alloy)| LivingConstructionJobV2 {
                    id: LivingEntityIdV2([230 + index as u8; 16]),
                    world,
                    owner: a,
                    definition: LivingSlugV2::new("solar_array").unwrap(),
                    accepted_tick: LivingTickV2(0),
                    completion_tick: LivingTickV2(1000),
                    paid_alloy,
                })
                .collect(),
        )
        .unwrap();
        state.hull_jobs = LivingSortedVecV2::new(vec![LivingHullJobV2 {
            id: LivingEntityIdV2([232; 16]),
            world,
            owner: a,
            definition: LivingSlugV2::new("scout").unwrap(),
            accepted_tick: LivingTickV2(0),
            completion_tick: LivingTickV2(1000),
            paid_alloy: 10,
            target_fleet: None,
        }])
        .unwrap();
        (authority, state)
    }
    fn reference_occupation(
        authority: &LivingGalaxyAuthorityV2,
        state: &LivingGalaxyStateV2,
    ) -> (LivingGalaxyStateV2, LivingPendingEventsV2) {
        let mut state = state.clone();
        let mut events = LivingPendingEventsV2::default();
        super::super::rules::reference_occupation_for_cursor_test(&mut LivingStepContextV2 {
            catalog: authority.catalog(),
            genesis_seed: [33; 32],
            branch: authority.branch(),
            tick: LivingTickV2(11),
            state: &mut state,
            events: &mut events,
            arrivals: Vec::new(),
        })
        .unwrap();
        (state, events)
    }
    #[test]
    fn occupation_cursor_refunds_multiple_captures_and_dormancy_match_reference_all_budgets() {
        let (authority, state) = occupation_fixture();
        let (expected, events) = reference_occupation(&authority, &state);
        let a = state.civilizations.as_slice()[0].id;
        let b = state.civilizations.as_slice()[1].id;
        assert_eq!(
            expected
                .civilizations
                .as_slice()
                .iter()
                .find(|row| row.id == a)
                .unwrap()
                .status,
            LivingCivilizationStatusV2::Dormant
        );
        assert_eq!(
            expected
                .relations
                .as_slice()
                .iter()
                .find(|row| row.from == a && row.to == b)
                .unwrap()
                .reasons
                .colony_captured_from_it,
            -60
        );
        assert_eq!(expected.worlds.as_slice()[0].inventory.alloy, 10000);
        assert!(
            expected.construction_jobs.as_slice().is_empty()
                && expected.hull_jobs.as_slice().is_empty()
        );
        let mut total = None;
        for budgets in [&[1][..], &[64][..], &[1024][..], &[1, 3, 64, 1024][..]] {
            let mut job = LivingOccupationJobV2::begin_owned(
                authority.catalog_snapshot(),
                state.clone(),
                LivingPendingEventsV2::default(),
                LivingTickV2(11),
            );
            let mut index = 0;
            loop {
                let budget = budgets[index % budgets.len()];
                index += 1;
                let progress = job.poll(budget).unwrap();
                assert!(job.last_poll_visits() <= budget);
                if progress == LivingLifecycleProgressV2::Ready {
                    break;
                }
            }
            if let Some(total) = total {
                assert_eq!(job.total_visits(), total)
            } else {
                total = Some(job.total_visits());
            }
            let (actual, actual_events) = job.finish().unwrap();
            assert_eq!(actual, expected);
            assert_eq!(actual_events.payloads(), events.payloads());
        }
    }
    #[test]
    fn occupation_cursor_defender_disabled_battery_zero_hp_and_progress_reset_axes() {
        for axis in 0..5 {
            let (authority, mut state) = occupation_fixture();
            let world = state.colonies.as_slice()[0].world;
            let owner = state.colonies.as_slice()[0].owner;
            match axis {
                0 => {
                    state
                        .fleets
                        .record_at_mut(0)
                        .unwrap()
                        .hulls
                        .record_at_mut(0)
                        .unwrap()
                        .hit_points = 0
                }
                1 | 2 => {
                    let mut facilities = state.facilities.into_vec();
                    facilities.push(LivingFacilityV2 {
                        id: LivingEntityIdV2([240; 16]),
                        world,
                        definition: LivingSlugV2::new("defense_battery").unwrap(),
                        status: LivingFacilityStatusV2::Operational,
                        paid_alloy: 40,
                        hit_points: 40,
                        next_due: None,
                        next_repair: None,
                        blocked_reasons: Default::default(),
                        disabled: axis == 2,
                    });
                    facilities.sort_by_key(|row| row.id);
                    state.facilities = LivingSortedVecV2::new(facilities).unwrap();
                }
                3 => state.fleets.record_at_mut(0).unwrap().owner = owner,
                _ => state.occupations.record_at_mut(0).unwrap().claimant = owner,
            }
            let (expected, events) = reference_occupation(&authority, &state);
            let mut job = LivingOccupationJobV2::begin(
                authority.catalog(),
                state,
                LivingPendingEventsV2::default(),
                LivingTickV2(11),
            );
            while job.poll(1).unwrap() != LivingLifecycleProgressV2::Ready {
                assert!(job.last_poll_visits() <= 1)
            }
            let (actual, actual_events) = job.finish().unwrap();
            assert_eq!(actual, expected, "axis {axis}");
            assert_eq!(actual_events.payloads(), events.payloads(), "axis {axis}");
        }
    }
    #[test]
    fn occupation_cursor_third_party_hostility_and_multiple_claimants_match_reference() {
        for axis in 0..3 {
            let (authority, mut state) = occupation_fixture();
            let a = state.civilizations.as_slice()[0].id;
            let b = state.civilizations.as_slice()[1].id;
            let c = LivingEntityIdV2([244; 16]);
            let mut civilization = state.civilizations.as_slice()[1].clone();
            civilization.id = c;
            civilization.status = LivingCivilizationStatusV2::Dormant;
            state.civilizations.push_sorted(civilization).unwrap();
            let mut fleet = state.fleets.as_slice()[0].clone();
            fleet.id = LivingEntityIdV2([245; 16]);
            fleet.owner = c;
            fleet.hulls = LivingSortedVecV2::new(vec![LivingHullV2 {
                id: LivingEntityIdV2([246; 16]),
                definition: LivingSlugV2::new("escort").unwrap(),
                hit_points: 10,
                return_credit: false,
                next_repair: None,
            }])
            .unwrap();
            state.fleets.push_sorted(fleet).unwrap();
            if axis > 0 {
                let other = if axis == 1 { b } else { a };
                let mut wars = state.wars.into_vec();
                wars.push(LivingWarV2 {
                    participant_a: other.min(c),
                    participant_b: other.max(c),
                    declarer: c,
                    start_tick: LivingTickV2(0),
                    end_tick: LivingTickV2(1000),
                });
                wars.sort_by_key(|row| (row.participant_a, row.participant_b));
                state.wars = LivingSortedVecV2::new(wars).unwrap();
            }
            let (expected, events) = reference_occupation(&authority, &state);
            let mut job = LivingOccupationJobV2::begin(
                authority.catalog(),
                state,
                LivingPendingEventsV2::default(),
                LivingTickV2(11),
            );
            while job.poll(1).unwrap() != LivingLifecycleProgressV2::Ready {
                assert!(job.last_poll_visits() <= 1);
            }
            let (actual, actual_events) = job.finish().unwrap();
            assert_eq!(actual, expected, "axis {axis}");
            assert_eq!(actual_events.payloads(), events.payloads(), "axis {axis}");
        }
    }
    #[test]
    fn occupation_cursor_cancel_during_refund_and_sticky_receipt_fault() {
        let (authority, state) = occupation_fixture();
        let mut job = LivingOccupationJobV2::begin(
            authority.catalog(),
            state.clone(),
            LivingPendingEventsV2::default(),
            LivingTickV2(11),
        );
        assert_eq!(job.poll(0), Err(LivingLifecycleJobErrorV2::Budget));
        assert_eq!(job.poll(1025), Err(LivingLifecycleJobErrorV2::Budget));
        assert_eq!(job.last_poll_visits(), 0);
        while job.stage != OccupationStage::ConstructionRefunds {
            job.poll(1).unwrap();
        }
        job.cancel();
        assert_eq!(job.poll(1), Err(LivingLifecycleJobErrorV2::Cancelled));
        assert!(matches!(
            job.finish(),
            Err(LivingLifecycleJobErrorV2::Cancelled)
        ));
        let mut events = LivingPendingEventsV2::default();
        for _ in 0..super::super::receipt::LIVING_MAX_BOUNDARY_EVENTS_V2 {
            events
                .record(
                    LivingEventProvenanceV2::Autonomous {
                        phase: LivingPhaseV2::ResolveConflict,
                    },
                    LivingEventKindV2::CombatRound,
                )
                .unwrap();
        }
        let mut job =
            LivingOccupationJobV2::begin(authority.catalog(), state, events, LivingTickV2(11));
        loop {
            match job.poll(1) {
                Ok(_) => {}
                Err(LivingLifecycleJobErrorV2::Fault(LivingDeterministicFaultV2::Receipt(_))) => {
                    break;
                }
                other => panic!("unexpected occupation result {other:?}"),
            }
        }
        assert!(matches!(
            job.poll(64),
            Err(LivingLifecycleJobErrorV2::Fault(
                LivingDeterministicFaultV2::Receipt(_)
            ))
        ));
        assert_eq!(job.last_poll_visits(), 0);
        assert!(matches!(
            job.finish(),
            Err(LivingLifecycleJobErrorV2::Fault(
                LivingDeterministicFaultV2::Receipt(_)
            ))
        ));
    }
    fn settlement_fixture(
        close: Option<LivingTickV2>,
    ) -> (
        LivingGalaxyAuthorityV2,
        LivingGalaxyStateV2,
        Vec<LivingEntityIdV2>,
    ) {
        let authority = builtin_living_sandbox_v2().unwrap();
        let mut state = authority.state().clone();
        let world = state
            .worlds
            .as_slice()
            .iter()
            .find(|row| row.owner.is_none())
            .unwrap()
            .id;
        let template = state.fleets.as_slice()[0].clone();
        let mut fleets = Vec::new();
        let mut claims = Vec::new();
        let mut arrivals = Vec::new();
        for (index, actor) in state
            .civilizations
            .as_slice()
            .iter()
            .map(|row| row.id)
            .enumerate()
        {
            for sub in 0..2 {
                let mut fleet = template.clone();
                fleet.id = LivingEntityIdV2([200 + index as u8 * 2 + sub; 16]);
                fleet.owner = actor;
                fleet.location = LivingFleetLocationV2::Docked { world };
                fleet.order = Some(LivingFleetOrderV2 {
                    kind: LivingFleetOrderKindV2::Settle,
                    target_world: Some(world),
                    not_before_tick: LivingTickV2(0),
                    lane_path: Vec::new(),
                });
                fleet.hulls = LivingSortedVecV2::new(vec![LivingHullV2 {
                    id: LivingEntityIdV2([220 + index as u8 * 2 + sub; 16]),
                    definition: LivingSlugV2::new("colony_ark").unwrap(),
                    hit_points: 20,
                    return_credit: false,
                    next_repair: None,
                }])
                .unwrap();
                arrivals.push(fleet.id);
                fleets.push(fleet);
            }
            if let Some(close_tick) = close {
                claims.push(LivingSettlementClaimV2 {
                    world,
                    civilization: actor,
                    arrival_tick: LivingTickV2(0),
                    close_tick,
                });
            }
        }
        state.fleets = LivingSortedVecV2::new(fleets).unwrap();
        state.settlement_claims = LivingSortedVecV2::new(claims).unwrap();
        (authority, state, arrivals)
    }
    fn reference_settlement(
        authority: &LivingGalaxyAuthorityV2,
        state: &LivingGalaxyStateV2,
        arrivals: Vec<LivingEntityIdV2>,
        tick: LivingTickV2,
    ) -> Result<(LivingGalaxyStateV2, LivingPendingEventsV2), LivingDeterministicFaultV2> {
        let mut state = state.clone();
        let mut events = LivingPendingEventsV2::default();
        super::super::rules::reference_settlement_for_cursor_test(&mut LivingStepContextV2 {
            catalog: authority.catalog(),
            genesis_seed: [33; 32],
            branch: authority.branch(),
            tick,
            state: &mut state,
            events: &mut events,
            arrivals,
        })?;
        Ok((state, events))
    }
    #[test]
    fn settlement_cursor_arbitration_lowest_ark_and_arrival_window_snapshot_all_budgets() {
        for close in [None, Some(LivingTickV2(50)), Some(LivingTickV2(100))] {
            let (authority, state, arrivals) = settlement_fixture(close);
            let (expected, events) =
                reference_settlement(&authority, &state, arrivals.clone(), LivingTickV2(50))
                    .unwrap();
            if close.is_none() {
                assert_eq!(
                    events
                        .payloads()
                        .iter()
                        .filter(|event| event.kind == LivingEventKindV2::SettlementClaimOpened)
                        .count(),
                    4,
                    "reference captures absent close_now for the whole world"
                );
            }
            if close == Some(LivingTickV2(50)) {
                assert_eq!(
                    expected.fleets.as_slice().len(),
                    state.fleets.as_slice().len() - 1
                );
                assert!(expected.settlement_claims.as_slice().is_empty());
            }
            let mut total = None;
            for budgets in [&[1][..], &[64][..], &[1024][..], &[1, 3, 64, 1024][..]] {
                let mut job = LivingSettlementJobV2::begin_owned(
                    authority.catalog_snapshot(),
                    state.clone(),
                    LivingPendingEventsV2::default(),
                    LivingTickV2(50),
                    [33; 32],
                    arrivals.clone(),
                );
                let mut index = 0;
                loop {
                    let budget = budgets[index % budgets.len()];
                    index += 1;
                    let progress = job.poll(budget).unwrap();
                    assert!(job.last_poll_visits() <= budget);
                    if progress == LivingLifecycleProgressV2::Ready {
                        break;
                    }
                }
                if let Some(total) = total {
                    assert_eq!(job.total_visits(), total)
                } else {
                    total = Some(job.total_visits());
                }
                let (actual, actual_events) = job.finish().unwrap();
                assert_eq!(actual, expected);
                assert_eq!(actual_events.payloads(), events.payloads());
            }
        }
    }
    #[test]
    fn settlement_cursor_armed_owned_absent_ark_and_dormant_reactivation_axes() {
        for axis in 0..5 {
            let (authority, mut state, arrivals) = settlement_fixture(Some(LivingTickV2(50)));
            let world = state.settlement_claims.as_slice()[0].world;
            match axis {
                0 => {
                    state
                        .fleets
                        .record_at_mut(0)
                        .unwrap()
                        .hulls
                        .record_at_mut(0)
                        .unwrap()
                        .definition = LivingSlugV2::new("escort").unwrap()
                }
                1 => {
                    let actor = state.civilizations.as_slice()[0].id;
                    state.worlds.find_mut(&world).unwrap().owner = Some(actor);
                }
                2 => {
                    let ids: Vec<_> = state.fleets.as_slice().iter().map(|row| row.id).collect();
                    for id in ids {
                        state.fleets.find_mut(&id).unwrap().hulls = Default::default();
                    }
                }
                3 => {
                    let ids: Vec<_> = state
                        .civilizations
                        .as_slice()
                        .iter()
                        .map(|row| row.id)
                        .collect();
                    for id in ids {
                        state.civilizations.find_mut(&id).unwrap().status =
                            LivingCivilizationStatusV2::Dormant;
                    }
                }
                _ => {
                    state
                        .fleets
                        .record_at_mut(0)
                        .unwrap()
                        .hulls
                        .record_at_mut(0)
                        .unwrap()
                        .hit_points = 0
                }
            }
            let (expected, events) =
                reference_settlement(&authority, &state, arrivals.clone(), LivingTickV2(50))
                    .unwrap();
            let mut job = LivingSettlementJobV2::begin(
                authority.catalog(),
                state,
                LivingPendingEventsV2::default(),
                LivingTickV2(50),
                [33; 32],
                arrivals,
            );
            while job.poll(1).unwrap() != LivingLifecycleProgressV2::Ready {
                assert!(job.last_poll_visits() <= 1)
            }
            let (actual, actual_events) = job.finish().unwrap();
            assert_eq!(actual, expected, "axis {axis}");
            assert_eq!(actual_events.payloads(), events.payloads(), "axis {axis}");
        }
    }
    #[test]
    fn settlement_cursor_cancel_consumption_and_sticky_hub_deadline_fault() {
        let (authority, state, arrivals) = settlement_fixture(Some(LivingTickV2(50)));
        let mut job = LivingSettlementJobV2::begin(
            authority.catalog(),
            state.clone(),
            LivingPendingEventsV2::default(),
            LivingTickV2(50),
            [33; 32],
            arrivals,
        );
        assert_eq!(job.poll(0), Err(LivingLifecycleJobErrorV2::Budget));
        assert_eq!(job.poll(1025), Err(LivingLifecycleJobErrorV2::Budget));
        assert_eq!(job.last_poll_visits(), 0);
        while job.stage != SettlementStage::ConsumeHull {
            job.poll(1).unwrap();
        }
        job.cancel();
        assert_eq!(job.poll(1), Err(LivingLifecycleJobErrorV2::Cancelled));
        assert!(matches!(
            job.finish(),
            Err(LivingLifecycleJobErrorV2::Cancelled)
        ));
        let tick = LivingTickV2(u64::MAX - 1);
        let (authority, state, arrivals) = settlement_fixture(Some(tick));
        assert!(matches!(
            reference_settlement(&authority, &state, arrivals.clone(), tick),
            Err(LivingDeterministicFaultV2::Arithmetic)
        ));
        let mut job = LivingSettlementJobV2::begin(
            authority.catalog(),
            state,
            LivingPendingEventsV2::default(),
            tick,
            [33; 32],
            arrivals,
        );
        loop {
            match job.poll(1) {
                Ok(_) => {}
                Err(LivingLifecycleJobErrorV2::Fault(LivingDeterministicFaultV2::Arithmetic)) => {
                    break;
                }
                other => panic!("unexpected settlement result {other:?}"),
            }
        }
        assert_eq!(
            job.poll(64),
            Err(LivingLifecycleJobErrorV2::Fault(
                LivingDeterministicFaultV2::Arithmetic
            ))
        );
        assert_eq!(job.last_poll_visits(), 0);
        assert!(matches!(
            job.finish(),
            Err(LivingLifecycleJobErrorV2::Fault(
                LivingDeterministicFaultV2::Arithmetic
            ))
        ));
    }
    fn reference_conflict(
        authority: &LivingGalaxyAuthorityV2,
        state: &LivingGalaxyStateV2,
        tick: LivingTickV2,
        arrivals: Vec<LivingEntityIdV2>,
    ) -> Result<(LivingGalaxyStateV2, LivingPendingEventsV2), LivingDeterministicFaultV2> {
        let mut state = state.clone();
        let mut events = LivingPendingEventsV2::default();
        super::super::rules::resolve_combat_occupation_and_claims(&mut LivingStepContextV2 {
            catalog: authority.catalog(),
            genesis_seed: [33; 32],
            branch: authority.branch(),
            tick,
            state: &mut state,
            events: &mut events,
            arrivals,
        })?;
        Ok((state, events))
    }
    fn conflict_retreat_order_fixture() -> (LivingGalaxyAuthorityV2, LivingGalaxyStateV2) {
        let authority = builtin_living_sandbox_v2().unwrap();
        let mut state = authority.state().clone();
        let a = state.civilizations.as_slice()[0].id;
        let b = state.civilizations.as_slice()[1].id;
        let world = state
            .colonies
            .as_slice()
            .iter()
            .find(|row| row.owner == a)
            .unwrap()
            .world;
        let template = state.fleets.as_slice()[0].clone();
        let mut fleets = Vec::new();
        for (index, owner, definition, hit_points) in [(0, a, "escort", 10), (1, b, "scout", 1)] {
            let mut fleet = template.clone();
            fleet.id = LivingEntityIdV2([200 + index; 16]);
            fleet.owner = owner;
            fleet.location = LivingFleetLocationV2::Docked { world };
            fleet.hulls = LivingSortedVecV2::new(vec![LivingHullV2 {
                id: LivingEntityIdV2([220 + index; 16]),
                definition: LivingSlugV2::new(definition).unwrap(),
                hit_points,
                return_credit: true,
                next_repair: None,
            }])
            .unwrap();
            fleets.push(fleet);
        }
        state.fleets = LivingSortedVecV2::new(fleets).unwrap();
        state.wars = LivingSortedVecV2::new(vec![LivingWarV2 {
            participant_a: a.min(b),
            participant_b: a.max(b),
            declarer: a,
            start_tick: LivingTickV2(0),
            end_tick: LivingTickV2(1000),
        }])
        .unwrap();
        (authority, state)
    }
    #[test]
    fn conflict_composite_shared_budget_matches_complete_reference_and_retreat_order() {
        let (authority, occupation) = occupation_fixture();
        let (_, settlement, arrivals) = settlement_fixture(Some(LivingTickV2(10)));
        let (_, retreat) = conflict_retreat_order_fixture();
        let mut cases = vec![
            (occupation, Vec::new()),
            (settlement, arrivals),
            (retreat, Vec::new()),
            (LivingGalaxyStateV2::default(), Vec::new()),
        ];
        for (case, (state, arrivals)) in cases.drain(..).enumerate() {
            let (expected, events) =
                reference_conflict(&authority, &state, LivingTickV2(10), arrivals.clone()).unwrap();
            if case == 2 {
                assert_eq!(
                    events
                        .payloads()
                        .iter()
                        .filter(|event| event.kind == LivingEventKindV2::FleetDeparted)
                        .count(),
                    1
                );
                assert!(!events.payloads().iter().any(|event| matches!(
                    event.kind,
                    LivingEventKindV2::CombatRound | LivingEventKindV2::HullDestroyed
                )));
            }
            let mut total = None;
            for budgets in [&[1][..], &[64][..], &[1024][..], &[1, 3, 64, 1024][..]] {
                let mut job = LivingConflictJobV2::begin_owned(
                    authority.catalog_snapshot(),
                    state.clone(),
                    LivingPendingEventsV2::default(),
                    LivingTickV2(10),
                    [33; 32],
                    arrivals.clone(),
                );
                let mut index = 0;
                loop {
                    let budget = budgets[index % budgets.len()];
                    index += 1;
                    let progress = job.poll(budget).unwrap();
                    assert!(job.last_poll_visits() <= budget);
                    if progress == LivingLifecycleProgressV2::Ready {
                        break;
                    }
                }
                if let Some(total) = total {
                    assert_eq!(job.total_visits(), total)
                } else {
                    total = Some(job.total_visits());
                }
                let (actual, actual_events) = job.finish().unwrap();
                assert_eq!(actual, expected, "case {case}");
                assert_eq!(actual_events.payloads(), events.payloads(), "case {case}");
            }
        }
    }
    #[test]
    fn conflict_composite_cancellation_in_each_child_and_sticky_failure() {
        let (authority, state) = occupation_fixture();
        for target in ["retreat", "combat", "occupation", "settlement"] {
            let mut job = LivingConflictJobV2::begin(
                authority.catalog(),
                state.clone(),
                LivingPendingEventsV2::default(),
                LivingTickV2(10),
                [33; 32],
                Vec::new(),
            );
            assert_eq!(job.poll(0), Err(LivingLifecycleJobErrorV2::Budget));
            assert_eq!(job.poll(1025), Err(LivingLifecycleJobErrorV2::Budget));
            assert_eq!(job.last_poll_visits(), 0);
            while job.component_name() != target {
                assert_eq!(job.poll(1).unwrap(), LivingLifecycleProgressV2::Pending);
            }
            job.cancel();
            assert_eq!(job.poll(64), Err(LivingLifecycleJobErrorV2::Cancelled));
            assert_eq!(job.last_poll_visits(), 0);
            assert!(matches!(
                job.finish(),
                Err(LivingLifecycleJobErrorV2::Cancelled)
            ));
        }
        let tick = LivingTickV2(u64::MAX - 1);
        let (authority, state, arrivals) = settlement_fixture(Some(tick));
        assert!(matches!(
            reference_conflict(&authority, &state, tick, arrivals.clone()),
            Err(LivingDeterministicFaultV2::Arithmetic)
        ));
        let mut job = LivingConflictJobV2::begin(
            authority.catalog(),
            state,
            LivingPendingEventsV2::default(),
            tick,
            [33; 32],
            arrivals,
        );
        loop {
            match job.poll(1) {
                Ok(_) => {}
                Err(LivingLifecycleJobErrorV2::Fault(LivingDeterministicFaultV2::Arithmetic)) => {
                    break;
                }
                other => panic!("unexpected composite result {other:?}"),
            }
        }
        assert_eq!(
            job.poll(64),
            Err(LivingLifecycleJobErrorV2::Fault(
                LivingDeterministicFaultV2::Arithmetic
            ))
        );
        assert_eq!(job.last_poll_visits(), 0);
        assert!(matches!(
            job.finish(),
            Err(LivingLifecycleJobErrorV2::Fault(
                LivingDeterministicFaultV2::Arithmetic
            ))
        ));
    }
}

enum ConflictComponent<'a> {
    Retreat(Box<LivingRetreatJobV2<'a>>),
    Combat(Box<LivingCombatJobV2<'a>>),
    Occupation(Box<LivingOccupationJobV2<'a>>),
    Settlement(Box<LivingSettlementJobV2<'a>>),
}
/// Phase-four composition in normative retreat, combat, occupation, settlement
/// order. A poll shares one remaining budget across all child transitions.
/// Candidate state and events remain unpublished throughout this composition.
pub struct LivingConflictJobV2<'a> {
    retirement: PhaseRetirementLane<'a>,
    delivered: bool,
    catalog: PhaseCatalogSlot<'a>,
    tick: LivingTickV2,
    seed: [u8; 32],
    arrivals: Option<Vec<LivingEntityIdV2>>,
    component: Option<ConflictComponent<'a>>,
    output: Option<Box<(LivingGalaxyStateV2, LivingPendingEventsV2)>>,
    cancelled: bool,
    fault: Option<LivingDeterministicFaultV2>,
    last_poll_visits: usize,
    total_visits: u64,
}
impl<'a> LivingConflictJobV2<'a> {
    /// Move the phase-three candidate with a borrowed immutable catalog.
    pub fn begin(
        catalog: &'a super::catalog::ValidatedLivingCatalogPackV2,
        state: LivingGalaxyStateV2,
        events: LivingPendingEventsV2,
        tick: LivingTickV2,
        seed: [u8; 32],
        arrivals: Vec<LivingEntityIdV2>,
    ) -> Self {
        Self::with_catalog(
            LivingPhaseCatalogV2::Borrowed(catalog),
            state,
            events,
            tick,
            seed,
            arrivals,
        )
    }
    fn with_catalog(
        catalog: LivingPhaseCatalogV2<'a>,
        state: LivingGalaxyStateV2,
        events: LivingPendingEventsV2,
        tick: LivingTickV2,
        seed: [u8; 32],
        arrivals: Vec<LivingEntityIdV2>,
    ) -> Self {
        let retreat = LivingRetreatJobV2::with_catalog(catalog.clone(), state, events, tick);
        Self {
            retirement: PhaseRetirementLane::default(),
            delivered: false,
            catalog: PhaseCatalogSlot(Some(catalog)),
            tick,
            seed,
            arrivals: Some(arrivals),
            component: Some(ConflictComponent::Retreat(Box::new(retreat))),
            output: None,
            cancelled: false,
            fault: None,
            last_poll_visits: 0,
            total_visits: 0,
        }
    }
    /// Cancel private work; no accepted command or resident state is consumed.
    pub fn cancel(&mut self) {
        self.cancelled = true;
    }
    /// Actual record visits consumed by the preceding poll across all children.
    pub const fn last_poll_visits(&self) -> usize {
        self.last_poll_visits
    }
    /// Actual record visits consumed by the whole conflict phase.
    pub const fn total_visits(&self) -> u64 {
        self.total_visits
    }
    /// Continue every child using only the unconsumed portion of this budget.
    pub fn poll(
        &mut self,
        budget: usize,
    ) -> Result<LivingLifecycleProgressV2, LivingLifecycleJobErrorV2> {
        self.last_poll_visits = 0;
        if self.cancelled {
            return Err(LivingLifecycleJobErrorV2::Cancelled);
        }
        if budget == 0 || budget > LIVING_MAX_PHASE_POLL_VISITS_V2 {
            return Err(LivingLifecycleJobErrorV2::Budget);
        }
        if let Some(fault) = &self.fault {
            return Err(LivingLifecycleJobErrorV2::Fault(fault.clone()));
        }
        while self.last_poll_visits < budget {
            if self.retirement.visit() {
                self.last_poll_visits += 1;
                self.total_visits += 1;
                continue;
            }

            if self.output.is_some() {
                return Ok(LivingLifecycleProgressV2::Ready);
            }
            let remaining = budget - self.last_poll_visits;
            let (result, visits) = match self
                .component
                .as_mut()
                .expect("unfinished conflict child exists")
            {
                ConflictComponent::Retreat(job) => (job.poll(remaining), job.last_poll_visits()),
                ConflictComponent::Combat(job) => (job.poll(remaining), job.last_poll_visits()),
                ConflictComponent::Occupation(job) => (job.poll(remaining), job.last_poll_visits()),
                ConflictComponent::Settlement(job) => (job.poll(remaining), job.last_poll_visits()),
            };
            self.last_poll_visits += visits;
            self.total_visits += visits as u64;
            match result {
                Ok(LivingLifecycleProgressV2::Pending) => {
                    return Ok(LivingLifecycleProgressV2::Pending);
                }
                Ok(LivingLifecycleProgressV2::Ready) => {
                    if self.last_poll_visits == budget {
                        break;
                    }
                    self.last_poll_visits += 1;
                    self.total_visits += 1;
                    if let Err(fault) = self.advance() {
                        self.fault = Some(fault.clone());
                        return Err(LivingLifecycleJobErrorV2::Fault(fault));
                    }
                }
                Err(error) => {
                    let fault = match error {
                        LivingLifecycleJobErrorV2::Fault(fault) => fault,
                        _ => LivingDeterministicFaultV2::Invariant,
                    };
                    self.fault = Some(fault.clone());
                    return Err(LivingLifecycleJobErrorV2::Fault(fault));
                }
            }
        }
        Ok(if self.output.is_some() && self.retirement.node.is_none() {
            LivingLifecycleProgressV2::Ready
        } else {
            LivingLifecycleProgressV2::Pending
        })
    }
    /// Move the fully completed conflict candidate onward. Ready children have
    /// already retired their temporary records; this does not publish truth.
    pub fn finish(
        mut self,
    ) -> Result<(LivingGalaxyStateV2, LivingPendingEventsV2), LivingLifecycleJobErrorV2> {
        self.take_ready_result()
    }
    /// Borrowed one-time extraction; rejected calls preserve every owner.
    pub fn take_ready_result(
        &mut self,
    ) -> Result<(LivingGalaxyStateV2, LivingPendingEventsV2), LivingLifecycleJobErrorV2> {
        if self.cancelled {
            return Err(LivingLifecycleJobErrorV2::Cancelled);
        }
        if let Some(fault) = self.fault.clone() {
            return Err(LivingLifecycleJobErrorV2::Fault(fault));
        }
        if self.delivered || self.retirement.node.is_some() {
            return Err(LivingLifecycleJobErrorV2::NotReady);
        }
        let output = self
            .output
            .as_mut()
            .ok_or(LivingLifecycleJobErrorV2::NotReady)?;
        self.delivered = true;
        Ok((std::mem::take(&mut output.0), std::mem::take(&mut output.1)))
    }
    fn advance(&mut self) -> Result<(), LivingDeterministicFaultV2> {
        let map_error = |error| match error {
            LivingLifecycleJobErrorV2::Fault(fault) => fault,
            _ => LivingDeterministicFaultV2::Invariant,
        };
        let (kind, output) = match self.component.as_mut().expect("ready conflict child") {
            ConflictComponent::Retreat(job) => (0, job.take_ready_result()),
            ConflictComponent::Combat(job) => (1, job.take_ready_result()),
            ConflictComponent::Occupation(job) => (2, job.take_ready_result()),
            ConflictComponent::Settlement(job) => (3, job.take_ready_result()),
        };
        let (state, events) = output.map_err(map_error)?;
        self.retirement
            .start(self.component.take().expect("retained ready child"));
        self.component = match kind {
            0 => Some(ConflictComponent::Combat(Box::new(
                LivingCombatJobV2::with_catalog((*self.catalog).clone(), state, events, self.tick),
            ))),
            1 => Some(ConflictComponent::Occupation(Box::new(
                LivingOccupationJobV2::with_catalog(
                    (*self.catalog).clone(),
                    state,
                    events,
                    self.tick,
                ),
            ))),
            2 => Some(ConflictComponent::Settlement(Box::new(
                LivingSettlementJobV2::with_catalog(
                    (*self.catalog).clone(),
                    state,
                    events,
                    self.tick,
                    self.seed,
                    self.arrivals.take().expect("one arrival consumer"),
                ),
            ))),
            _ => {
                self.output = Some(Box::new((state, events)));
                None
            }
        };
        Ok(())
    }
    #[cfg(test)]
    fn component_name(&self) -> &'static str {
        match &self.component {
            Some(ConflictComponent::Retreat(_)) => "retreat",
            Some(ConflictComponent::Combat(_)) => "combat",
            Some(ConflictComponent::Occupation(_)) => "occupation",
            Some(ConflictComponent::Settlement(_)) => "settlement",
            None => "ready",
        }
    }
}
impl LivingConflictJobV2<'static> {
    /// Move an owned immutable catalog handle with the phase-three candidate.
    pub fn begin_owned(
        catalog: std::sync::Arc<super::catalog::ValidatedLivingCatalogPackV2>,
        state: LivingGalaxyStateV2,
        events: LivingPendingEventsV2,
        tick: LivingTickV2,
        seed: [u8; 32],
        arrivals: Vec<LivingEntityIdV2>,
    ) -> Self {
        Self::with_catalog(
            LivingPhaseCatalogV2::Owned(catalog),
            state,
            events,
            tick,
            seed,
            arrivals,
        )
    }
}

#[cfg(test)]
mod phase_cleanup_tests {
    use super::*;
    #[test]
    fn observation_replacement_retires_each_nested_remembered_record() {
        let authority = crate::living::builtin_living_sandbox_v2().unwrap();
        let state = authority.state().clone();
        let actor = state.civilizations.as_slice()[0].id;
        let world = state.worlds.as_slice()[0].id;
        for budget in [1, 64, 1024] {
            let mut job = LivingObservationJobV2::begin(state.clone(), LivingTickV2(1));
            // Controlled private-stage fixture isolates replaced-record cleanup.
            let row = LivingObservationV2 {
                civilization: actor,
                world,
                observed_tick: LivingTickV2(1),
                owner: None,
                remaining_deposit: 0,
                facilities: Default::default(),
                stationed_hulls: Default::default(),
                operational_defense_batteries: None,
            };
            let mut old = row.clone();
            for index in 0..37 {
                let definition =
                    crate::living::LivingSlugV2::new(format!("kind_{index:03}")).unwrap();
                old.facilities
                    .push_sorted(super::super::model::LivingObservedFacilityV2 {
                        definition: definition.clone(),
                        count: 1,
                    })
                    .unwrap();
                old.stationed_hulls
                    .push_sorted(super::super::model::LivingObservedHullV2 {
                        definition,
                        count: 1,
                    })
                    .unwrap();
            }
            job.observations.clear();
            job.observations.insert(old.living_key(), old);
            job.observation = Some(row);
            job.cursor = ObservationCursor::Commit;
            job.output_loaded = true;
            // Stop after exactly the replacement and its nested cleanup visits.
            for _ in 0..75 {
                job.poll(1).unwrap();
                assert_eq!(job.last_poll_visits(), 1);
            }
            assert!(matches!(job.cursor, ObservationCursor::RetireRemembered));
            assert!(!job.visit().unwrap());
            assert!(matches!(job.cursor, ObservationCursor::VisibleWorld));
            assert_eq!(job.total_visits(), 75); // replacement + 74 nested records
            assert_eq!(job.retired_facilities.len(), 0);
            assert_eq!(job.retired_hulls.len(), 0);
            // Finish through the ordinary continuation at the requested budget.
            loop {
                let progress = job.poll(budget).unwrap();
                assert!(job.last_poll_visits() <= budget);
                if progress == LivingLifecycleProgressV2::Ready {
                    break;
                }
            }
            assert!(job.finish().is_ok());
        }
    }
    #[test]
    fn repair_indexes_are_retired_before_ready_and_cancel_cannot_finish() {
        let authority = crate::living::builtin_living_sandbox_v2().unwrap();
        let mut job = LivingRepairJobV2::begin_owned(
            authority.catalog_snapshot(),
            authority.state().clone(),
            LivingTickV2(1),
        );
        job.cursor = RepairCursor::RetireIndices;
        let id = authority.state().worlds.as_slice()[0].id;
        job.world_index.insert(id, (0, None));
        job.hit_points.insert("scout".into(), 100);
        assert_eq!(job.poll(1), Ok(LivingLifecycleProgressV2::Pending));
        assert_eq!(job.last_poll_visits(), 1);
        assert!(job.world_index.is_empty());
        assert_eq!(job.hit_points.len(), 1);
        job.cancel();
        assert!(matches!(
            job.finish(),
            Err(LivingLifecycleJobErrorV2::Cancelled)
        ));
    }
}

// ---------------------------------------------------- diplomacy phase seven
use super::decision::{
    LivingDecisionOutcomeV2, LivingDecisionReasonV2, LivingDecisionScoreV2, LivingDecisionTierV2,
    LivingIntentActionV2, LivingIntentDecisionV2,
};
type DiplomacyDecisionKey = (LivingEntityIdV2, u16, Option<LivingEntityIdV2>, Vec<u8>);
#[derive(Clone, Copy, Eq, PartialEq)]
enum DiplomacyStage {
    Decay,
    Hash,
    Civilizations,
    Worlds,
    Colonies,
    Systems,
    Lanes,
    LaneReverse,
    Agreements,
    Wars,
    Relations,
    HullJobs,
    Fleets,
    Hulls,
    Observations,
    ObservedFacilities,
    ObservedHulls,
    Pair,
    PairColonies,
    PairSearch,
    PairProposals,
    WarActor,
    WarOpponent,
    WarObservation,
    AdjacentColony,
    AdjacentLane,
    AttackFleet,
    AttackSearch,
    WarCandidate,
    ApplyAgreements,
    ApplyWars,
    RemoveWarAgreements,
    DeclareWar,
    OutputDecisions,
    RetireOldAgreements,
    RetireOldWars,
    RetireOldRelations,
    OutputAgreements,
    OutputWars,
    OutputRelations,
    Retire,
    Ready,
}
/// Resumable diplomacy over one immutable, post-decay proposal snapshot.
/// Candidate changes remain private until finish; every scanned record and
/// retained index entry consumes a visit. This component does not seal a step.
pub struct LivingDiplomacyJobV2 {
    retirement: PhaseRetirementLane<'static>,
    delivered: bool,
    state: PhaseArc<LivingGalaxyStateV2>,
    events: LivingPendingEventsV2,
    tick: LivingTickV2,
    stage: DiplomacyStage,
    index: usize,
    nested: usize,
    hash: Option<LivingStateHashJobV2>,
    digest: Option<super::ids::LivingStateDigestV2>,
    active: Vec<LivingEntityIdV2>,
    worlds: BTreeMap<LivingEntityIdV2, (LivingEntityIdV2, u32)>,
    owners: BTreeMap<LivingEntityIdV2, LivingEntityIdV2>,
    graph: Graph,
    blocked: BTreeSet<Pair>,
    truces: BTreeSet<Pair>,
    existing: BTreeSet<AgreementKey>,
    agreements: BTreeMap<AgreementKey, LivingAgreementV2>,
    wars: BTreeMap<Pair, LivingWarV2>,
    relations: BTreeMap<Pair, LivingRelationV2>,
    reserved_fleets: usize,
    busy: BTreeSet<LivingEntityIdV2>,
    attacks: BTreeMap<LivingEntityIdV2, (LivingEntityIdV2, LivingEntityIdV2, i64)>,
    escorts: usize,
    escort_cost: u32,
    all_cost: u32,
    defenses: BTreeMap<Pair, i64>,
    observed_escorts: i64,
    observed_batteries: i64,
    left: usize,
    right: usize,
    colony_left: usize,
    colony_right: usize,
    connected: bool,
    proposal_kind: usize,
    search: Option<CostSearch>,
    search_found: Option<bool>,
    actor: usize,
    opponent: usize,
    observation: usize,
    home: usize,
    attack: i64,
    best: Option<(i32, LivingEntityIdV2, LivingEntityIdV2)>,
    agreement_proposals: std::collections::VecDeque<LivingAgreementV2>,
    war_proposals:
        std::collections::VecDeque<(LivingEntityIdV2, LivingEntityIdV2, LivingEntityIdV2)>,
    current_war: Option<(LivingEntityIdV2, LivingEntityIdV2, LivingEntityIdV2)>,
    decisions: BTreeMap<DiplomacyDecisionKey, LivingIntentDecisionV2>,
    decision_actor: Option<LivingEntityIdV2>,
    decision_ordinal: u16,
    old_agreements: IntoIter<LivingAgreementV2>,
    old_wars: IntoIter<LivingWarV2>,
    old_relations: IntoIter<LivingRelationV2>,
    cancelled: bool,
    fault: Option<LivingDeterministicFaultV2>,
    last_poll_visits: usize,
    total_visits: u64,
}
impl LivingDiplomacyJobV2 {
    /// Capture ownership without copying the snapshot. Proposal evaluation does
    /// not mutate it; only scalar decay precedes its incremental normalized hash.
    pub fn begin(
        state: LivingGalaxyStateV2,
        events: LivingPendingEventsV2,
        tick: LivingTickV2,
    ) -> Self {
        let reserved_fleets = state.fleets.as_slice().len();
        Self {
            retirement: PhaseRetirementLane::default(),
            delivered: false,
            state: Arc::new(state).into(),
            events,
            tick,
            stage: DiplomacyStage::Decay,
            index: 0,
            nested: 0,
            hash: None,
            digest: None,
            active: Vec::new(),
            worlds: BTreeMap::new(),
            owners: BTreeMap::new(),
            graph: BTreeMap::new(),
            blocked: BTreeSet::new(),
            truces: BTreeSet::new(),
            existing: BTreeSet::new(),
            agreements: BTreeMap::new(),
            wars: BTreeMap::new(),
            relations: BTreeMap::new(),
            reserved_fleets,
            busy: BTreeSet::new(),
            attacks: BTreeMap::new(),
            escorts: 0,
            escort_cost: 0,
            all_cost: 0,
            defenses: BTreeMap::new(),
            observed_escorts: 0,
            observed_batteries: 0,
            left: 0,
            right: 1,
            colony_left: 0,
            colony_right: 0,
            connected: false,
            proposal_kind: 0,
            search: None,
            search_found: None,
            actor: 0,
            opponent: 0,
            observation: 0,
            home: 0,
            attack: 0,
            best: None,
            agreement_proposals: Default::default(),
            war_proposals: Default::default(),
            current_war: None,
            decisions: BTreeMap::new(),
            decision_actor: None,
            decision_ordinal: 0,
            old_agreements: Vec::new().into_iter(),
            old_wars: Vec::new().into_iter(),
            old_relations: Vec::new().into_iter(),
            cancelled: false,
            fault: None,
            last_poll_visits: 0,
            total_visits: 0,
        }
    }
    /// Prevent publication of this private candidate.
    pub fn cancel(&mut self) {
        self.cancelled = true;
    }
    /// Records consumed by the preceding poll.
    pub const fn last_poll_visits(&self) -> usize {
        self.last_poll_visits
    }
    /// Records consumed across all polls.
    pub const fn total_visits(&self) -> u64 {
        self.total_visits
    }
    /// Advance under a single shared record budget, including snapshot hashing.
    pub fn poll(
        &mut self,
        budget: usize,
    ) -> Result<LivingLifecycleProgressV2, LivingLifecycleJobErrorV2> {
        self.last_poll_visits = 0;
        if self.cancelled {
            return Err(LivingLifecycleJobErrorV2::Cancelled);
        }
        if budget == 0 || budget > LIVING_MAX_PHASE_POLL_VISITS_V2 {
            return Err(LivingLifecycleJobErrorV2::Budget);
        }
        if let Some(fault) = &self.fault {
            return Err(fault.clone().into());
        }
        while self.stage != DiplomacyStage::Ready && self.last_poll_visits < budget {
            if self.retirement.visit() {
                self.last_poll_visits += 1;
                self.total_visits += 1;
                continue;
            }

            if let Some(hash) = &mut self.hash {
                let result = hash.poll(budget - self.last_poll_visits);
                let visits = hash.last_poll_visits();
                self.last_poll_visits += visits;
                self.total_visits += visits as u64;
                match result {
                    Ok(LivingLifecycleProgressV2::Pending) => break,
                    Ok(LivingLifecycleProgressV2::Ready) => {
                        if self.last_poll_visits == budget {
                            break;
                        }
                        self.digest = Some(hash.take_ready_result()?);
                        self.retirement
                            .start(self.hash.take().expect("retained hash"));
                        self.last_poll_visits += 1;
                        self.total_visits += 1;
                        self.transition(DiplomacyStage::Civilizations);
                    }
                    Err(error) => {
                        let fault = match error {
                            LivingLifecycleJobErrorV2::Fault(fault) => fault,
                            _ => LivingDeterministicFaultV2::Invariant,
                        };
                        self.fault = Some(fault.clone());
                        return Err(fault.into());
                    }
                }
                continue;
            }
            match self.visit_diplomacy() {
                Ok(true) => {
                    self.last_poll_visits += 1;
                    self.total_visits += 1;
                }
                Ok(false) => {}
                Err(fault) => {
                    self.last_poll_visits += 1;
                    self.total_visits += 1;
                    self.fault = Some(fault.clone());
                    return Err(fault.into());
                }
            }
        }
        Ok(if self.stage == DiplomacyStage::Ready {
            LivingLifecycleProgressV2::Ready
        } else {
            LivingLifecycleProgressV2::Pending
        })
    }
    /// Move the complete private candidate after all proposal and cleanup visits.
    pub fn finish(
        mut self,
    ) -> Result<(LivingGalaxyStateV2, LivingPendingEventsV2), LivingLifecycleJobErrorV2> {
        self.take_ready_result()
    }
    /// Borrowed one-time extraction; rejected calls preserve every owner.
    pub fn take_ready_result(
        &mut self,
    ) -> Result<(LivingGalaxyStateV2, LivingPendingEventsV2), LivingLifecycleJobErrorV2> {
        if self.cancelled {
            return Err(LivingLifecycleJobErrorV2::Cancelled);
        }
        if let Some(fault) = self.fault.clone() {
            return Err(fault.into());
        }
        if self.stage != DiplomacyStage::Ready {
            return Err(LivingLifecycleJobErrorV2::NotReady);
        }
        if self.delivered {
            return Err(LivingLifecycleJobErrorV2::NotReady);
        }
        let source = self
            .state
            .0
            .take()
            .ok_or(LivingLifecycleJobErrorV2::NotReady)?;
        let state = match Arc::try_unwrap(source) {
            Ok(state) => state,
            Err(source) => {
                self.state.0 = Some(source);
                return Err(LivingLifecycleJobErrorV2::NotReady);
            }
        };
        self.delivered = true;
        Ok((state, std::mem::take(&mut self.events)))
    }
    fn transition(&mut self, stage: DiplomacyStage) {
        self.stage = stage;
        self.index = 0;
        self.nested = 0;
    }
    fn relation(&self, from: LivingEntityIdV2, to: LivingEntityIdV2) -> i32 {
        self.relations
            .get(&(from, to))
            .map(LivingRelationV2::total)
            .unwrap_or(0)
    }
    fn decision(
        &mut self,
        decision: LivingIntentDecisionV2,
    ) -> Result<(), LivingDeterministicFaultV2> {
        let key = (
            decision.actor,
            decision.action.ordinal(),
            decision.target,
            decision.action.canonical_bytes()?,
        );
        self.decisions.insert(key, decision);
        Ok(())
    }
    fn start_search(
        &mut self,
        from: LivingEntityIdV2,
        to: LivingEntityIdV2,
        stage: DiplomacyStage,
    ) {
        self.search = Some(CostSearch::begin(
            self.worlds.get(&from).map(|row| row.0),
            self.worlds.get(&to).map(|row| row.0),
            &self.graph,
        ));
        self.search_found = None;
        self.stage = stage;
    }
    // Retain the whole search until each cached record has retired.
    fn visit_search(&mut self) -> Result<bool, LivingDeterministicFaultV2> {
        let search = self
            .search
            .as_mut()
            .ok_or(LivingDeterministicFaultV2::Invariant)?;
        if search.result.is_none() {
            return Ok(search.visit(&self.graph));
        }
        if self.search_found.is_none() {
            self.search_found = Some(search.result.flatten().is_some());
        }
        if search.distance.pop_first().is_some() || search.frontier.pop_first().is_some() {
            return Ok(true);
        }
        self.retirement
            .start(self.search.take().expect("retained completed cost search"));
        Ok(true)
    }
    fn visit_diplomacy(&mut self) -> Result<bool, LivingDeterministicFaultV2> {
        use super::model::LivingCivilizationStatusV2;
        use DiplomacyStage::*;
        match self.stage {
            Decay => {
                if self.tick.0.is_multiple_of(600) {
                    let state = Arc::get_mut(&mut self.state)
                        .ok_or(LivingDeterministicFaultV2::Invariant)?;
                    if let Some(row) = state.relations.record_at_mut(self.index) {
                        let r = &mut row.reasons;
                        if r.delivered_aid > 0 {
                            r.delivered_aid -= 1;
                        }
                        if r.successful_trade > 0 {
                            r.successful_trade -= 1;
                        }
                        if r.adjacent_rival_settlement < 0 {
                            r.adjacent_rival_settlement += 1;
                        }
                        if r.war_declared_against_it < 0 {
                            r.war_declared_against_it += 1;
                        }
                        if r.colony_captured_from_it < 0 {
                            r.colony_captured_from_it += 1;
                        }
                        self.index += 1;
                        return Ok(true);
                    }
                }
                self.hash = Some(LivingStateHashJobV2::begin_decision_snapshot(
                    Arc::clone(&self.state),
                    self.tick,
                ));
                self.transition(Hash);
            }
            Hash => return Err(LivingDeterministicFaultV2::Invariant),
            Civilizations => {
                if let Some(row) = self.state.civilizations.as_slice().get(self.index) {
                    if row.status == LivingCivilizationStatusV2::Active {
                        self.active.push(row.id);
                    }
                    self.index += 1;
                    return Ok(true);
                }
                self.transition(Worlds);
            }
            Worlds => {
                if let Some(row) = self.state.worlds.as_slice().get(self.index) {
                    self.worlds
                        .insert(row.id, (row.system, row.inventory.energy));
                    self.index += 1;
                    return Ok(true);
                }
                self.transition(Colonies);
            }
            Colonies => {
                if let Some(row) = self.state.colonies.as_slice().get(self.index) {
                    self.owners.insert(row.world, row.owner);
                    self.index += 1;
                    return Ok(true);
                }
                self.transition(Systems);
            }
            Systems => {
                if let Some(row) = self.state.systems.as_slice().get(self.index) {
                    self.graph.insert(row.id, Vec::new());
                    self.index += 1;
                    return Ok(true);
                }
                self.transition(Lanes);
            }
            Lanes | LaneReverse => {
                if let Some(row) = self.state.lanes.as_slice().get(self.index) {
                    let (from, to) = if self.stage == Lanes {
                        (row.system_a, row.system_b)
                    } else {
                        (row.system_b, row.system_a)
                    };
                    if let Some(edges) = self.graph.get_mut(&from) {
                        edges.push((to, row.distance_units.div_ceil(10).max(10)));
                    }
                    if self.stage == Lanes {
                        self.stage = LaneReverse;
                    } else {
                        self.index += 1;
                        self.stage = Lanes;
                    }
                    return Ok(true);
                }
                self.transition(Agreements);
            }
            Agreements => {
                if let Some(row) = self.state.agreements.as_slice().get(self.index) {
                    self.agreements.insert(row.living_key(), *row);
                    if row.start_tick.0 <= self.tick.0 && self.tick.0 < row.end_tick.0 {
                        let pair = (row.participant_a, row.participant_b);
                        self.blocked.insert(pair);
                        self.existing.insert(row.living_key());
                        if row.kind == LivingAgreementKindV2::Truce {
                            self.truces.insert(pair);
                        }
                    }
                    self.index += 1;
                    return Ok(true);
                }
                self.transition(Wars);
            }
            Wars => {
                if let Some(row) = self.state.wars.as_slice().get(self.index) {
                    self.wars
                        .insert((row.participant_a, row.participant_b), *row);
                    if row.start_tick.0 <= self.tick.0 && self.tick.0 < row.end_tick.0 {
                        self.blocked.insert((row.participant_a, row.participant_b));
                    }
                    self.index += 1;
                    return Ok(true);
                }
                self.transition(Relations);
            }
            Relations => {
                if let Some(row) = self.state.relations.as_slice().get(self.index) {
                    self.relations.insert((row.from, row.to), *row);
                    self.index += 1;
                    return Ok(true);
                }
                self.transition(HullJobs);
            }
            HullJobs => {
                if let Some(row) = self.state.hull_jobs.as_slice().get(self.index) {
                    if let Some(fleet) = row.target_fleet {
                        self.busy.insert(fleet);
                    } else {
                        self.reserved_fleets += 1;
                    }
                    self.index += 1;
                    return Ok(true);
                }
                self.transition(Fleets);
            }
            Fleets => {
                if self.state.fleets.as_slice().get(self.index).is_some() {
                    self.escorts = 0;
                    self.escort_cost = 0;
                    self.all_cost = 0;
                    self.nested = 0;
                    self.stage = Hulls;
                    return Ok(true);
                }
                self.transition(Observations);
            }
            Hulls => {
                let fleet = &self.state.fleets.as_slice()[self.index];
                if let Some(hull) = fleet.hulls.as_slice().get(self.nested) {
                    let cost = if hull.return_credit { 2 } else { 4 };
                    self.all_cost += cost;
                    if hull.definition.as_str() == "escort" {
                        self.escorts += 1;
                        self.escort_cost += cost;
                    }
                    self.nested += 1;
                    return Ok(true);
                }
                if let LivingFleetLocationV2::Docked { world } = fleet.location {
                    let cost = if self.reserved_fleets >= super::model::LIVING_MAX_FLEETS_V2 {
                        self.all_cost
                    } else {
                        self.escort_cost
                    };
                    if fleet.order.is_none()
                        && !self.busy.contains(&fleet.id)
                        && self.owners.get(&world) == Some(&fleet.owner)
                        && self.worlds.get(&world).is_some_and(|row| row.1 >= cost)
                    {
                        self.attacks
                            .insert(fleet.id, (fleet.owner, world, self.escorts as i64 * 10));
                    }
                }
                self.index += 1;
                self.stage = Fleets;
            }
            Observations => {
                if self.state.observations.as_slice().get(self.index).is_some() {
                    self.observed_escorts = 0;
                    self.observed_batteries = 0;
                    self.nested = 0;
                    self.stage = ObservedFacilities;
                    return Ok(true);
                }
                self.transition(Pair);
            }
            ObservedFacilities => {
                let row = &self.state.observations.as_slice()[self.index];
                if let Some(facility) = row.facilities.as_slice().get(self.nested) {
                    if facility.definition.as_str() == "defense_battery" {
                        self.observed_batteries += i64::from(facility.count);
                    }
                    self.nested += 1;
                    return Ok(true);
                }
                self.nested = 0;
                self.stage = ObservedHulls;
            }
            ObservedHulls => {
                let row = &self.state.observations.as_slice()[self.index];
                if let Some(hull) = row.stationed_hulls.as_slice().get(self.nested) {
                    if hull.definition.as_str() == "escort" {
                        self.observed_escorts += i64::from(hull.count);
                    }
                    self.nested += 1;
                    return Ok(true);
                }
                let batteries = row
                    .operational_defense_batteries
                    .map(i64::from)
                    .unwrap_or(self.observed_batteries);
                self.defenses.insert(
                    (row.civilization, row.world),
                    self.observed_escorts.saturating_mul(10) + batteries.saturating_mul(20),
                );
                self.index += 1;
                self.stage = Observations;
            }
            Pair => {
                if self.left >= self.active.len() {
                    self.transition(WarActor);
                    return Ok(false);
                }
                if self.right >= self.active.len() {
                    self.left += 1;
                    self.right = self.left + 1;
                    return Ok(false);
                }
                let pair = (self.active[self.left], self.active[self.right]);
                let war = self.wars.get(&pair).is_some_and(|row| {
                    row.start_tick.0 <= self.tick.0 && self.tick.0 < row.end_tick.0
                });
                if self.truces.contains(&pair) || war {
                    self.right += 1;
                    return Ok(true);
                }
                self.connected = false;
                self.colony_left = 0;
                self.colony_right = 0;
                self.stage = PairColonies;
                return Ok(true);
            }
            PairColonies => {
                let rows = self.state.colonies.as_slice();
                let Some(left) = rows.get(self.colony_left) else {
                    self.proposal_kind = 0;
                    self.stage = PairProposals;
                    return Ok(false);
                };
                if left.owner != self.active[self.left] {
                    self.colony_left += 1;
                    self.colony_right = 0;
                    return Ok(true);
                }
                let Some(right) = rows.get(self.colony_right) else {
                    self.colony_left += 1;
                    self.colony_right = 0;
                    return Ok(false);
                };
                self.colony_right += 1;
                if right.owner == self.active[self.right] {
                    self.start_search(left.world, right.world, PairSearch);
                }
                return Ok(true);
            }
            PairSearch => {
                if self.search.is_some() {
                    return self.visit_search();
                }
                if self.search_found.take().unwrap_or(false) {
                    self.connected = true;
                    self.proposal_kind = 0;
                    self.stage = PairProposals;
                } else {
                    self.stage = PairColonies;
                }
            }
            PairProposals => {
                if self.proposal_kind >= 2 {
                    self.right += 1;
                    self.stage = Pair;
                    return Ok(false);
                }
                let left = self.active[self.left];
                let right = self.active[self.right];
                let mutual = self.relation(left, right).min(self.relation(right, left));
                let (kind, required, eligible, rule) = if self.proposal_kind == 0 {
                    (
                        LivingAgreementKindV2::Trade,
                        0,
                        mutual >= 0 && self.connected,
                        "mutual_trade_with_connection",
                    )
                } else {
                    (
                        LivingAgreementKindV2::Nonaggression,
                        20,
                        mutual >= 20,
                        "mutual_nonaggression",
                    )
                };
                let agreement = LivingAgreementV2 {
                    kind,
                    participant_a: left,
                    participant_b: right,
                    start_tick: self.tick,
                    end_tick: self.tick,
                };
                if eligible && !self.existing.contains(&agreement.living_key()) {
                    let end_tick = LivingTickV2(
                        self.tick
                            .0
                            .checked_add(1200)
                            .ok_or(LivingDeterministicFaultV2::Arithmetic)?,
                    );
                    for (actor, peer) in [(left, right), (right, left)] {
                        self.decision(LivingIntentDecisionV2 {
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
                            snapshot_tick: self.tick,
                            snapshot_digest: self
                                .digest
                                .ok_or(LivingDeterministicFaultV2::Invariant)?,
                            observed_tick: None,
                            outcome: LivingDecisionOutcomeV2::Accepted,
                            reasons: vec![
                                LivingDecisionReasonV2::BilateralThreshold {
                                    peer,
                                    forward: self.relation(actor, peer),
                                    reverse: self.relation(peer, actor),
                                    required,
                                },
                                LivingDecisionReasonV2::Accepted {
                                    rule: super::catalog::LivingSlugV2::new(rule)
                                        .map_err(|_| LivingDeterministicFaultV2::Invariant)?,
                                },
                            ],
                        })?;
                    }
                    self.agreement_proposals.push_back(LivingAgreementV2 {
                        end_tick,
                        ..agreement
                    });
                }
                self.proposal_kind += 1;
                return Ok(true);
            }
            WarActor => {
                if self.actor >= self.active.len() {
                    self.transition(ApplyAgreements);
                    return Ok(false);
                }
                self.best = None;
                self.opponent = 0;
                self.stage = WarOpponent;
            }
            WarOpponent => {
                if self.opponent >= self.active.len() {
                    if let Some((_, world, peer)) = self.best.take() {
                        self.war_proposals
                            .push_back((self.active[self.actor], peer, world));
                    }
                    self.actor += 1;
                    self.stage = WarActor;
                    return Ok(true);
                }
                let actor = self.active[self.actor];
                let peer = self.active[self.opponent];
                let pair = if actor < peer {
                    (actor, peer)
                } else {
                    (peer, actor)
                };
                if actor == peer || self.blocked.contains(&pair) || self.relation(actor, peer) > -20
                {
                    self.opponent += 1;
                    return Ok(true);
                }
                self.observation = 0;
                self.stage = WarObservation;
                return Ok(true);
            }
            WarObservation => {
                let Some(row) = self.state.observations.as_slice().get(self.observation) else {
                    self.opponent += 1;
                    self.stage = WarOpponent;
                    return Ok(false);
                };
                if row.civilization != self.active[self.actor]
                    || row.owner != Some(self.active[self.opponent])
                    || self.tick.0.saturating_sub(row.observed_tick.0) > 100
                {
                    self.observation += 1;
                    return Ok(true);
                }
                self.home = 0;
                self.stage = AdjacentColony;
                return Ok(true);
            }
            AdjacentColony => {
                let target = self.state.observations.as_slice()[self.observation].world;
                let Some(colony) = self.state.colonies.as_slice().get(self.home) else {
                    self.observation += 1;
                    self.stage = WarObservation;
                    return Ok(false);
                };
                if colony.owner != self.active[self.actor] {
                    self.home += 1;
                    return Ok(true);
                }
                let home = self.worlds.get(&colony.world).map(|row| row.0);
                let target = self.worlds.get(&target).map(|row| row.0);
                if home.is_some() && home == target {
                    self.attack = 0;
                    self.index = 0;
                    self.stage = AttackFleet;
                } else {
                    self.index = 0;
                    self.stage = AdjacentLane;
                }
                return Ok(true);
            }
            AdjacentLane => {
                let target = self.state.observations.as_slice()[self.observation].world;
                let colony = &self.state.colonies.as_slice()[self.home];
                let home = self.worlds.get(&colony.world).map(|row| row.0);
                let target = self.worlds.get(&target).map(|row| row.0);
                if let Some(lane) = self.state.lanes.as_slice().get(self.index) {
                    if home.zip(target).is_some_and(|(h, t)| {
                        (lane.system_a == h && lane.system_b == t)
                            || (lane.system_b == h && lane.system_a == t)
                    }) {
                        self.attack = 0;
                        self.index = 0;
                        self.stage = AttackFleet;
                    } else {
                        self.index += 1;
                    }
                    return Ok(true);
                }
                self.home += 1;
                self.stage = AdjacentColony;
            }
            AttackFleet => {
                let target = self.state.observations.as_slice()[self.observation].world;
                let Some(fleet) = self.state.fleets.as_slice().get(self.index) else {
                    self.stage = WarCandidate;
                    return Ok(false);
                };
                self.index += 1;
                if let Some(&(actor, world, _)) = self.attacks.get(&fleet.id)
                    && actor == self.active[self.actor]
                    && world != target
                {
                    self.nested = self.index - 1; // retain exact eligible fleet during search
                    self.start_search(world, target, AttackSearch);
                }
                return Ok(true);
            }
            AttackSearch => {
                if self.search.is_some() {
                    return self.visit_search();
                }
                if self.search_found.take().unwrap_or(false) {
                    let fleet = &self.state.fleets.as_slice()[self.nested];
                    self.attack = self.attack.max(
                        self.attacks
                            .get(&fleet.id)
                            .ok_or(LivingDeterministicFaultV2::Invariant)?
                            .2,
                    );
                }
                self.stage = AttackFleet;
            }
            WarCandidate => {
                let world = self.state.observations.as_slice()[self.observation].world;
                let actor = self.active[self.actor];
                let peer = self.active[self.opponent];
                let defense = self.defenses.get(&(actor, world)).copied().unwrap_or(0);
                if self.attack >= 10 && self.attack.saturating_mul(2) >= defense.saturating_mul(3) {
                    let candidate = (self.relation(actor, peer), world, peer);
                    if self.best.is_none_or(|current| candidate < current) {
                        self.best = Some(candidate);
                    }
                }
                self.observation += 1;
                self.stage = WarObservation;
                return Ok(true);
            }
            ApplyAgreements => {
                if let Some(row) = self.agreement_proposals.pop_front() {
                    self.agreements.insert(row.living_key(), row);
                    self.events.record_explained(
                        LivingEventProvenanceV2::Autonomous {
                            phase: LivingPhaseV2::UpdateDiplomacy,
                        },
                        LivingEventKindV2::AgreementFormed,
                        super::event_facts::LivingEventFactsV2 {
                            agreement: Some(row),
                            ..Default::default()
                        },
                    )?;
                    return Ok(true);
                }
                self.transition(ApplyWars);
            }
            ApplyWars => {
                if let Some((actor, peer, world)) = self.war_proposals.pop_front() {
                    let pair = if actor < peer {
                        (actor, peer)
                    } else {
                        (peer, actor)
                    };
                    let duplicate = self.wars.get(&pair).is_some_and(|row| {
                        row.start_tick.0 <= self.tick.0 && self.tick.0 < row.end_tick.0
                    });
                    let rows = self.state.observations.as_slice();
                    let observed_tick = rows
                        .binary_search_by_key(&(actor, world), LivingKeyedV2::living_key)
                        .ok()
                        .map(|index| rows[index].observed_tick);
                    self.decision(LivingIntentDecisionV2 {
                        phase: LivingPhaseV2::UpdateDiplomacy,
                        actor,
                        ordinal: 0,
                        action: LivingIntentActionV2::War { peer },
                        target: Some(world),
                        score: LivingDecisionScoreV2(0),
                        tier: LivingDecisionTierV2::Ordinary,
                        snapshot_tick: self.tick,
                        snapshot_digest: self
                            .digest
                            .ok_or(LivingDeterministicFaultV2::Invariant)?,
                        observed_tick,
                        outcome: if duplicate {
                            LivingDecisionOutcomeV2::Rejected
                        } else {
                            LivingDecisionOutcomeV2::Accepted
                        },
                        reasons: vec![if duplicate {
                            LivingDecisionReasonV2::InvalidTarget {
                                target: peer,
                                rule: super::catalog::LivingSlugV2::new(
                                    "bilateral_war_already_reserved",
                                )
                                .map_err(|_| LivingDeterministicFaultV2::Invariant)?,
                            }
                        } else {
                            LivingDecisionReasonV2::Accepted {
                                rule: super::catalog::LivingSlugV2::new(
                                    "fresh_adjacent_target_assembled_escort_heuristic",
                                )
                                .map_err(|_| LivingDeterministicFaultV2::Invariant)?,
                            }
                        }],
                    })?;
                    if !duplicate {
                        self.current_war = Some((actor, peer, world));
                        self.stage = RemoveWarAgreements;
                    }
                    return Ok(true);
                }
                self.transition(OutputDecisions);
            }
            RemoveWarAgreements => {
                let (actor, peer, _) = self
                    .current_war
                    .ok_or(LivingDeterministicFaultV2::Invariant)?;
                let (a, b) = if actor < peer {
                    (actor, peer)
                } else {
                    (peer, actor)
                };
                let key = self
                    .agreements
                    .range((a, b, "")..=(a, b, "\u{10ffff}"))
                    .next()
                    .map(|(key, _)| *key);
                if let Some(key) = key {
                    self.agreements.remove(&key);
                    return Ok(true);
                }
                self.stage = DeclareWar;
            }
            DeclareWar => {
                let (actor, peer, _) = self
                    .current_war
                    .ok_or(LivingDeterministicFaultV2::Invariant)?;
                let (a, b) = if actor < peer {
                    (actor, peer)
                } else {
                    (peer, actor)
                };
                let end_tick = LivingTickV2(
                    self.tick
                        .0
                        .checked_add(1200)
                        .ok_or(LivingDeterministicFaultV2::Arithmetic)?,
                );
                self.wars.insert(
                    (a, b),
                    LivingWarV2 {
                        participant_a: a,
                        participant_b: b,
                        declarer: actor,
                        start_tick: self.tick,
                        end_tick,
                    },
                );
                let mut relation =
                    self.relations
                        .get(&(peer, actor))
                        .copied()
                        .unwrap_or(LivingRelationV2 {
                            from: peer,
                            to: actor,
                            base_disposition: 0,
                            reasons: Default::default(),
                        });
                relation.reasons.war_declared_against_it =
                    (relation.reasons.war_declared_against_it - 30).max(-30);
                self.relations.insert((peer, actor), relation);
                self.events.record(
                    LivingEventProvenanceV2::Autonomous {
                        phase: LivingPhaseV2::UpdateDiplomacy,
                    },
                    LivingEventKindV2::WarDeclared,
                )?;
                self.current_war = None;
                self.stage = ApplyWars;
                return Ok(true);
            }
            OutputDecisions => {
                if let Some((_, mut decision)) = self.decisions.pop_first() {
                    if self.decision_actor != Some(decision.actor) {
                        self.decision_actor = Some(decision.actor);
                        self.decision_ordinal = 0;
                    }
                    decision.ordinal = self.decision_ordinal;
                    self.decision_ordinal = self
                        .decision_ordinal
                        .checked_add(1)
                        .ok_or(LivingDeterministicFaultV2::Arithmetic)?;
                    self.events.decisions.push(decision);
                    return Ok(true);
                }
                let state =
                    Arc::get_mut(&mut self.state).ok_or(LivingDeterministicFaultV2::Invariant)?;
                self.old_agreements = std::mem::take(&mut state.agreements).into_vec().into_iter();
                self.old_wars = std::mem::take(&mut state.wars).into_vec().into_iter();
                self.old_relations = std::mem::take(&mut state.relations).into_vec().into_iter();
                self.transition(RetireOldAgreements);
            }
            RetireOldAgreements => {
                if self.old_agreements.next().is_some() {
                    return Ok(true);
                }
                self.transition(RetireOldWars);
            }
            RetireOldWars => {
                if self.old_wars.next().is_some() {
                    return Ok(true);
                }
                self.transition(RetireOldRelations);
            }
            RetireOldRelations => {
                if self.old_relations.next().is_some() {
                    return Ok(true);
                }
                self.transition(OutputAgreements);
            }
            OutputAgreements => {
                if let Some((_, row)) = self.agreements.pop_first() {
                    Arc::get_mut(&mut self.state)
                        .ok_or(LivingDeterministicFaultV2::Invariant)?
                        .agreements
                        .push_sorted(row)?;
                    return Ok(true);
                }
                self.transition(OutputWars);
            }
            OutputWars => {
                if let Some((_, row)) = self.wars.pop_first() {
                    Arc::get_mut(&mut self.state)
                        .ok_or(LivingDeterministicFaultV2::Invariant)?
                        .wars
                        .push_sorted(row)?;
                    return Ok(true);
                }
                self.transition(OutputRelations);
            }
            OutputRelations => {
                if let Some((_, row)) = self.relations.pop_first() {
                    Arc::get_mut(&mut self.state)
                        .ok_or(LivingDeterministicFaultV2::Invariant)?
                        .relations
                        .push_sorted(row)?;
                    return Ok(true);
                }
                self.transition(Retire);
            }
            Retire => {
                if let Some(entry) = self.graph.first_entry() {
                    if entry.get().is_empty() {
                        entry.remove_entry();
                    } else {
                        entry.into_mut().pop();
                    }
                    return Ok(true);
                }
                if self.active.pop().is_some()
                    || self.worlds.pop_first().is_some()
                    || self.owners.pop_first().is_some()
                    || self.blocked.pop_first().is_some()
                    || self.truces.pop_first().is_some()
                    || self.existing.pop_first().is_some()
                    || self.busy.pop_first().is_some()
                    || self.attacks.pop_first().is_some()
                    || self.defenses.pop_first().is_some()
                {
                    return Ok(true);
                }
                self.stage = Ready;
            }
            Ready => {}
        }
        Ok(false)
    }
}

#[cfg(test)]
mod diplomacy_tests {
    use super::super::model::*;
    use super::super::wire::LivingSortedVecV2;
    use super::*;
    fn reference(
        state: LivingGalaxyStateV2,
        tick: LivingTickV2,
    ) -> (LivingGalaxyStateV2, LivingPendingEventsV2) {
        let authority = crate::living::builtin_living_sandbox_v2().unwrap();
        let mut state = state;
        let mut events = LivingPendingEventsV2::default();
        let mut context = super::super::simulation::LivingStepContextV2 {
            catalog: authority.catalog(),
            genesis_seed: [0; 32],
            branch: authority.branch(),
            tick,
            state: &mut state,
            events: &mut events,
            arrivals: Vec::new(),
        };
        super::super::rules::reference_diplomacy_for_cursor_test(&mut context).unwrap();
        (state, events)
    }
    fn run(
        state: LivingGalaxyStateV2,
        tick: LivingTickV2,
        budgets: &[usize],
    ) -> (LivingGalaxyStateV2, LivingPendingEventsV2, u64) {
        let mut job = LivingDiplomacyJobV2::begin(state, LivingPendingEventsV2::default(), tick);
        let mut index = 0;
        loop {
            let budget = budgets[index % budgets.len()];
            index += 1;
            let progress = job.poll(budget).unwrap();
            assert!(job.last_poll_visits() <= budget);
            if progress == LivingLifecycleProgressV2::Ready {
                break;
            }
        }
        let total = job.total_visits();
        assert!(job.graph.is_empty() && job.decisions.is_empty() && job.defenses.is_empty());
        assert!(job.active.is_empty() && job.worlds.is_empty() && job.busy.is_empty());
        let (state, events) = job.finish().unwrap();
        (state, events, total)
    }
    fn peace(base: i32) -> LivingGalaxyStateV2 {
        let authority = crate::living::builtin_living_sandbox_v2().unwrap();
        let mut state = authority.state().clone();
        state.tick = LivingTickV2(49);
        let a = state.civilizations.as_slice()[0].id;
        let b = state.civilizations.as_slice()[1].id;
        state.relations = LivingSortedVecV2::new(vec![
            LivingRelationV2 {
                from: a,
                to: b,
                base_disposition: base,
                reasons: Default::default(),
            },
            LivingRelationV2 {
                from: b,
                to: a,
                base_disposition: base,
                reasons: Default::default(),
            },
        ])
        .unwrap();
        state.wars = Default::default();
        state.agreements = Default::default();
        state
    }
    fn war_fixture() -> LivingGalaxyStateV2 {
        let mut state = peace(-30);
        let actors: Vec<_> = state
            .civilizations
            .as_slice()
            .iter()
            .map(|row| row.id)
            .collect();
        let homes: Vec<_> = actors
            .iter()
            .map(|actor| {
                state
                    .colonies
                    .as_slice()
                    .iter()
                    .find(|row| row.owner == *actor)
                    .unwrap()
                    .world
            })
            .collect();
        for index in 0..state.fleets.as_slice().len() {
            let fleet = state.fleets.record_at_mut(index).unwrap();
            fleet.owner = actors[index % 2];
            fleet.location = LivingFleetLocationV2::Docked {
                world: homes[index % 2],
            };
            fleet.order = None;
            for index in 0..fleet.hulls.as_slice().len() {
                let hull = fleet.hulls.record_at_mut(index).unwrap();
                hull.definition = crate::living::LivingSlugV2::new("escort").unwrap();
                hull.hit_points = 1;
            }
        }
        for home in &homes {
            state.worlds.find_mut(home).unwrap().inventory.energy = 100;
        }
        state.observations = LivingSortedVecV2::new(
            (0..2)
                .map(|index| LivingObservationV2 {
                    civilization: actors[index],
                    world: homes[1 - index],
                    observed_tick: LivingTickV2(49),
                    owner: Some(actors[1 - index]),
                    remaining_deposit: 0,
                    facilities: Default::default(),
                    stationed_hulls: Default::default(),
                    operational_defense_batteries: None,
                })
                .collect(),
        )
        .unwrap();
        state
    }
    #[test]
    fn diplomacy_cursor_trade_nonaggression_war_and_shared_snapshot_match_reference() {
        for state in [peace(0), peace(20), war_fixture()] {
            let expected = reference(state.clone(), LivingTickV2(50));
            let mut total = None;
            for budgets in [&[1][..], &[64][..], &[1024][..], &[1, 3, 64, 1024][..]] {
                let (actual, events, visits) = run(state.clone(), LivingTickV2(50), budgets);
                assert_eq!(actual, expected.0);
                assert_eq!(events.payloads(), expected.1.payloads());
                assert_eq!(events.explained, expected.1.explained);
                assert_eq!(events.decisions, expected.1.decisions);
                if let Some(total) = total {
                    assert_eq!(visits, total);
                } else {
                    total = Some(visits);
                }
            }
        }
        let (_, events) = reference(war_fixture(), LivingTickV2(50));
        assert_eq!(
            events
                .payloads()
                .iter()
                .filter(|row| row.kind == LivingEventKindV2::WarDeclared)
                .count(),
            1
        );
        assert_eq!(events.decisions.len(), 2);
        assert_eq!(
            events.decisions[0].outcome,
            LivingDecisionOutcomeV2::Accepted
        );
        assert_eq!(
            events.decisions[1].outcome,
            LivingDecisionOutcomeV2::Rejected
        );
        assert_eq!(
            events.decisions[0].snapshot_digest,
            events.decisions[1].snapshot_digest
        );
    }
    #[test]
    fn diplomacy_cursor_decay_truce_existing_agreements_and_stale_observation_axes() {
        let mut decay = peace(20);
        decay.tick = LivingTickV2(599);
        for index in 0..decay.relations.as_slice().len() {
            decay.relations.record_at_mut(index).unwrap().reasons = LivingRelationReasonsV2 {
                delivered_aid: 3,
                successful_trade: 4,
                adjacent_rival_settlement: -2,
                war_declared_against_it: -3,
                colony_captured_from_it: -4,
                ..Default::default()
            };
        }
        let mut truce = peace(20);
        let a = truce.civilizations.as_slice()[0].id;
        let b = truce.civilizations.as_slice()[1].id;
        truce
            .agreements
            .push_sorted(LivingAgreementV2 {
                kind: LivingAgreementKindV2::Truce,
                participant_a: a,
                participant_b: b,
                start_tick: LivingTickV2(0),
                end_tick: LivingTickV2(100),
            })
            .unwrap();
        let mut existing = peace(20);
        existing
            .agreements
            .push_sorted(LivingAgreementV2 {
                kind: LivingAgreementKindV2::Trade,
                participant_a: a,
                participant_b: b,
                start_tick: LivingTickV2(0),
                end_tick: LivingTickV2(100),
            })
            .unwrap();
        let mut stale = war_fixture();
        stale.tick = LivingTickV2(199);
        for (state, tick) in [
            (decay, LivingTickV2(600)),
            (truce, LivingTickV2(50)),
            (existing, LivingTickV2(50)),
            (stale, LivingTickV2(200)),
        ] {
            let expected = reference(state.clone(), tick);
            let (actual, events, _) = run(state, tick, &[1, 64, 3, 1024]);
            assert_eq!(actual, expected.0);
            assert_eq!(events.payloads(), expected.1.payloads());
            assert_eq!(events.decisions, expected.1.decisions);
        }
    }
    #[test]
    fn diplomacy_cursor_zero_one_and_many_active_actors_match_reference() {
        let template = peace(20).civilizations.as_slice()[0].clone();
        for count in [0, 1, 20] {
            let mut state = LivingGalaxyStateV2 {
                tick: LivingTickV2(49),
                ..Default::default()
            };
            for index in 1..=count {
                let mut civ = template.clone();
                civ.id = LivingEntityIdV2([index as u8; 16]);
                state.civilizations.push_sorted(civ).unwrap();
            }
            for left in 1..=count {
                for right in 1..=count {
                    if left != right {
                        state
                            .relations
                            .push_sorted(LivingRelationV2 {
                                from: LivingEntityIdV2([left as u8; 16]),
                                to: LivingEntityIdV2([right as u8; 16]),
                                base_disposition: 20,
                                reasons: Default::default(),
                            })
                            .unwrap();
                    }
                }
            }
            let expected = reference(state.clone(), LivingTickV2(50));
            let (actual, events, _) = run(state, LivingTickV2(50), &[1, 64, 1024]);
            assert_eq!(actual, expected.0);
            assert_eq!(events.payloads(), expected.1.payloads());
            assert_eq!(events.decisions, expected.1.decisions);
        }
    }
    #[test]
    fn diplomacy_cursor_path_existence_preserves_saturated_cost_and_disconnection() {
        for (edges, distance, disconnected) in
            [(2, 10, false), (2, 10, true), (11, u64::MAX, false)]
        {
            let mut state = peace(0);
            let template = state.systems.as_slice()[0].clone();
            for index in 0..=edges {
                let mut system = template.clone();
                system.id = LivingEntityIdV2([200 + index as u8; 16]);
                state.systems.push_sorted(system).unwrap();
            }
            let a = state.civilizations.as_slice()[0].id;
            for index in 0..state.worlds.as_slice().len() {
                let world = state.worlds.record_at_mut(index).unwrap();
                if world.owner.is_some() {
                    world.system = LivingEntityIdV2(
                        [if world.owner == Some(a) {
                            200
                        } else {
                            200 + edges as u8
                        }; 16],
                    );
                }
            }
            for index in 0..edges {
                if disconnected && index == 0 {
                    continue;
                }
                state
                    .lanes
                    .push_sorted(LivingLaneV2 {
                        id: LivingEntityIdV2([100 + index as u8; 16]),
                        system_a: LivingEntityIdV2([200 + index as u8; 16]),
                        system_b: LivingEntityIdV2([201 + index as u8; 16]),
                        distance_units: distance,
                    })
                    .unwrap();
            }
            let expected = reference(state.clone(), LivingTickV2(50));
            let (actual, events, _) = run(state, LivingTickV2(50), &[1, 64, 3, 1024]);
            assert_eq!(actual, expected.0);
            assert_eq!(events.payloads(), expected.1.payloads());
            assert_eq!(events.decisions, expected.1.decisions);
            assert_eq!(
                actual.agreements.as_slice().len(),
                usize::from(!disconnected && distance != u64::MAX)
            );
        }
    }
    #[test]
    fn diplomacy_cursor_cancel_each_major_stage_and_sticky_deadline_fault() {
        for stage in [
            DiplomacyStage::Hash,
            DiplomacyStage::PairSearch,
            DiplomacyStage::AttackSearch,
            DiplomacyStage::ApplyWars,
            DiplomacyStage::Retire,
        ] {
            let mut job = LivingDiplomacyJobV2::begin(
                war_fixture(),
                LivingPendingEventsV2::default(),
                LivingTickV2(50),
            );
            assert_eq!(job.poll(0), Err(LivingLifecycleJobErrorV2::Budget));
            assert_eq!(job.last_poll_visits(), 0);
            loop {
                let progress = job.poll(1).unwrap();
                if job.stage == stage {
                    break;
                }
                assert_ne!(
                    progress,
                    LivingLifecycleProgressV2::Ready,
                    "stage never reached"
                );
            }
            job.cancel();
            assert_eq!(job.poll(1), Err(LivingLifecycleJobErrorV2::Cancelled));
            assert_eq!(job.last_poll_visits(), 0);
            assert!(matches!(
                job.finish(),
                Err(LivingLifecycleJobErrorV2::Cancelled)
            ));
        }
        let authority = crate::living::builtin_living_sandbox_v2().unwrap();
        let mut state = peace(0);
        state.tick = LivingTickV2(u64::MAX - 2);
        let tick = LivingTickV2(u64::MAX - 1);
        let mut expected_state = state.clone();
        let mut events = LivingPendingEventsV2::default();
        let expected = super::super::rules::reference_diplomacy_for_cursor_test(
            &mut super::super::simulation::LivingStepContextV2 {
                catalog: authority.catalog(),
                genesis_seed: [0; 32],
                branch: authority.branch(),
                tick,
                state: &mut expected_state,
                events: &mut events,
                arrivals: Vec::new(),
            },
        )
        .unwrap_err();
        assert_eq!(expected, LivingDeterministicFaultV2::Arithmetic);
        for budget in [1, 64, 1024] {
            let mut job =
                LivingDiplomacyJobV2::begin(state.clone(), LivingPendingEventsV2::default(), tick);
            loop {
                match job.poll(budget) {
                    Ok(LivingLifecycleProgressV2::Pending) => {}
                    Err(LivingLifecycleJobErrorV2::Fault(fault)) => {
                        assert_eq!(fault, expected);
                        break;
                    }
                    other => panic!("unexpected result {other:?}"),
                }
                assert!(job.last_poll_visits() <= budget);
            }
            assert!(job.last_poll_visits() <= budget);
            assert_eq!(
                job.poll(1),
                Err(LivingLifecycleJobErrorV2::Fault(expected.clone()))
            );
            assert_eq!(job.last_poll_visits(), 0);
            assert!(matches!(
                job.finish(),
                Err(LivingLifecycleJobErrorV2::Fault(_))
            ));
        }
    }
    #[test]
    fn diplomacy_cursor_assembly_permissions_and_observed_operational_defense_axes() {
        for axis in 0..5 {
            let mut state = war_fixture();
            match axis {
                0 => {
                    for index in 0..state.worlds.as_slice().len() {
                        state.worlds.record_at_mut(index).unwrap().inventory.energy = 0;
                    }
                }
                1 => {
                    for index in 0..state.fleets.as_slice().len() {
                        state.fleets.record_at_mut(index).unwrap().order =
                            Some(LivingFleetOrderV2 {
                                kind: LivingFleetOrderKindV2::Defend,
                                target_world: None,
                                not_before_tick: LivingTickV2(50),
                                lane_path: Vec::new(),
                            });
                    }
                }
                2 => {
                    let a = state.civilizations.as_slice()[0].id;
                    let b = state.civilizations.as_slice()[1].id;
                    state
                        .agreements
                        .push_sorted(LivingAgreementV2 {
                            kind: LivingAgreementKindV2::Nonaggression,
                            participant_a: a,
                            participant_b: b,
                            start_tick: LivingTickV2(0),
                            end_tick: LivingTickV2(100),
                        })
                        .unwrap();
                }
                _ => {
                    for index in 0..state.observations.as_slice().len() {
                        let observation = state.observations.record_at_mut(index).unwrap();
                        observation
                            .facilities
                            .push_sorted(LivingObservedFacilityV2 {
                                definition: crate::living::LivingSlugV2::new("defense_battery")
                                    .unwrap(),
                                count: 100,
                            })
                            .unwrap();
                        observation.operational_defense_batteries = (axis == 4).then_some(0);
                    }
                }
            }
            let expected = reference(state.clone(), LivingTickV2(50));
            let (actual, events, _) = run(state, LivingTickV2(50), &[1, 3, 64, 1024]);
            assert_eq!(actual, expected.0);
            assert_eq!(events.payloads(), expected.1.payloads());
            assert_eq!(events.decisions, expected.1.decisions);
            assert_eq!(actual.wars.as_slice().len(), usize::from(axis == 4));
        }
    }
    #[test]
    fn diplomacy_cursor_invalid_snapshot_tick_matches_reference_fault() {
        let authority = crate::living::builtin_living_sandbox_v2().unwrap();
        for tick in [LivingTickV2(0), LivingTickV2(49), LivingTickV2(51)] {
            let mut state = peace(0);
            let mut events = LivingPendingEventsV2::default();
            let expected = super::super::rules::reference_diplomacy_for_cursor_test(
                &mut super::super::simulation::LivingStepContextV2 {
                    catalog: authority.catalog(),
                    genesis_seed: [0; 32],
                    branch: authority.branch(),
                    tick,
                    state: &mut state,
                    events: &mut events,
                    arrivals: Vec::new(),
                },
            )
            .unwrap_err();
            let mut job =
                LivingDiplomacyJobV2::begin(peace(0), LivingPendingEventsV2::default(), tick);
            loop {
                match job.poll(1) {
                    Ok(LivingLifecycleProgressV2::Pending) => {}
                    Err(LivingLifecycleJobErrorV2::Fault(fault)) => {
                        assert_eq!(fault, expected);
                        break;
                    }
                    other => panic!("unexpected result {other:?}"),
                }
                assert!(job.last_poll_visits() <= 1);
            }
            assert_eq!(job.poll(1), Err(LivingLifecycleJobErrorV2::Fault(expected)));
            assert_eq!(job.last_poll_visits(), 0);
        }
    }
}

#[derive(Clone, Copy, Eq, PartialEq)]
enum DispatchStage {
    Worlds,
    Colonies,
    Civilizations,
    HullJobs,
    Observations,
    Systems,
    Lanes,
    Agreements,
    CatalogHazards,
    Shipments,
    Fleet,
    FleetTravel,
    FleetHulls,
    FleetSpend,
    FleetCredits,
    FleetPathRetire,
    FleetDepart,
    Route,
    RouteTravel,
    RouteAdvance,
    RouteHazards,
    RouteAdmit,
    RouteCreate,
    ShipmentRetire,
    RouteEmit,
    RouteEvidence,
    PathRetire,
    ShipmentOutput,
    Retire,
    Ready,
}
/// Standalone bounded dispatch on a private candidate. This is not a whole-step
/// executor; event sealing and authority publication remain caller-owned.
pub struct LivingDispatchJobV2<'a> {
    retirement: PhaseRetirementLane<'a>,
    delivered: bool,
    catalog: PhaseCatalogSlot<'a>,
    state: LivingGalaxyStateV2,
    events: LivingPendingEventsV2,
    tick: LivingTickV2,
    genesis_seed: [u8; 32],
    branch: super::ids::LivingBranchIdV2,
    stage: DispatchStage,
    index: usize,
    fleet_index: usize,
    hull_index: usize,
    route_index: usize,
    metadata: BTreeMap<LivingEntityIdV2, (usize, LivingEntityIdV2, Option<LivingEntityIdV2>)>,
    colonies: BTreeMap<LivingEntityIdV2, LivingEntityIdV2>,
    dormant: BTreeSet<LivingEntityIdV2>,
    busy: BTreeSet<LivingEntityIdV2>,
    remembered: BTreeSet<Pair>,
    trade: BTreeSet<Pair>,
    lanes: BTreeMap<LivingEntityIdV2, Pair>,
    hazard_effects: BTreeMap<crate::living::LivingSlugV2, (u32, u32)>,
    graph: PhaseArc<RetreatGraph>,
    search: Option<RetreatPathJob>,
    duration: Option<u64>,
    path: Vec<LivingEntityIdV2>,
    retired_path: Vec<LivingEntityIdV2>,
    origin: LivingEntityIdV2,
    destination: LivingEntityIdV2,
    missing_credit: u32,
    has_ark: bool,
    consumes_credit: bool,
    grant_credit: bool,
    route: Option<super::model::LivingFreightRouteV2>,
    eligible: u32,
    batch: u32,
    hazard: Option<super::model::LivingHazardV2>,
    permitted: bool,
    shipment_input: IntoIter<LivingShipmentV2>,
    shipments: BTreeMap<LivingEntityIdV2, LivingShipmentV2>,
    retired_shipment: Option<LivingShipmentV2>,
    cancelled: bool,
    fault: Option<LivingDeterministicFaultV2>,
    last_poll_visits: usize,
    total_visits: u64,
}
impl<'a> LivingDispatchJobV2<'a> {
    /// Move a private state and borrow its immutable catalog without scanning.
    pub fn begin(
        catalog: &'a super::catalog::ValidatedLivingCatalogPackV2,
        state: LivingGalaxyStateV2,
        events: LivingPendingEventsV2,
        tick: LivingTickV2,
        genesis_seed: [u8; 32],
        branch: super::ids::LivingBranchIdV2,
    ) -> Self {
        Self::with_catalog(
            LivingPhaseCatalogV2::Borrowed(catalog),
            state,
            events,
            tick,
            genesis_seed,
            branch,
        )
    }
    fn with_catalog(
        catalog: LivingPhaseCatalogV2<'a>,
        mut state: LivingGalaxyStateV2,
        events: LivingPendingEventsV2,
        tick: LivingTickV2,
        genesis_seed: [u8; 32],
        branch: super::ids::LivingBranchIdV2,
    ) -> Self {
        let shipment_input = std::mem::take(&mut state.shipments).into_vec().into_iter();
        Self {
            retirement: PhaseRetirementLane::default(),
            delivered: false,
            catalog: PhaseCatalogSlot(Some(catalog)),
            state,
            events,
            tick,
            genesis_seed,
            branch,
            stage: DispatchStage::Worlds,
            index: 0,
            fleet_index: 0,
            hull_index: 0,
            route_index: 0,
            metadata: BTreeMap::new(),
            colonies: BTreeMap::new(),
            dormant: BTreeSet::new(),
            busy: BTreeSet::new(),
            remembered: BTreeSet::new(),
            trade: BTreeSet::new(),
            lanes: BTreeMap::new(),
            hazard_effects: BTreeMap::new(),
            graph: Arc::new(RetreatGraph::default()).into(),
            search: None,
            duration: None,
            path: Vec::new(),
            retired_path: Vec::new(),
            origin: LivingEntityIdV2([0; 16]),
            destination: LivingEntityIdV2([0; 16]),
            missing_credit: 0,
            has_ark: false,
            consumes_credit: false,
            grant_credit: false,
            route: None,
            eligible: 0,
            batch: 0,
            hazard: None,
            permitted: false,
            shipment_input,
            shipments: BTreeMap::new(),
            retired_shipment: None,
            cancelled: false,
            fault: None,
            last_poll_visits: 0,
            total_visits: 0,
        }
    }
    /// Prevent private output from being finished.
    pub fn cancel(&mut self) {
        self.cancelled = true;
    }
    /// Actual visits in the latest poll, including failed attempted work.
    pub fn last_poll_visits(&self) -> usize {
        self.last_poll_visits
    }
    /// Actual visits over all successful and failed polls.
    pub fn total_visits(&self) -> u64 {
        self.total_visits
    }
    /// Perform at most the requested number of logical record visits.
    pub fn poll(
        &mut self,
        budget: usize,
    ) -> Result<LivingLifecycleProgressV2, LivingLifecycleJobErrorV2> {
        self.last_poll_visits = 0;
        if self.cancelled {
            return Err(LivingLifecycleJobErrorV2::Cancelled);
        }
        if budget == 0 || budget > LIVING_MAX_PHASE_POLL_VISITS_V2 {
            return Err(LivingLifecycleJobErrorV2::Budget);
        }
        if let Some(fault) = &self.fault {
            return Err(LivingLifecycleJobErrorV2::Fault(fault.clone()));
        }
        while self.last_poll_visits < budget && self.stage != DispatchStage::Ready {
            if self.retirement.visit() {
                self.last_poll_visits += 1;
                self.total_visits += 1;
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
                    self.fault = Some(fault.clone());
                    return Err(LivingLifecycleJobErrorV2::Fault(fault));
                }
            }
        }
        Ok(if self.stage == DispatchStage::Ready {
            LivingLifecycleProgressV2::Ready
        } else {
            LivingLifecycleProgressV2::Pending
        })
    }
    /// Move the ready private state/events without installing them in authority.
    pub fn finish(
        mut self,
    ) -> Result<(LivingGalaxyStateV2, LivingPendingEventsV2), LivingLifecycleJobErrorV2> {
        self.take_ready_result()
    }
    /// Borrowed one-time extraction; rejected calls preserve every owner.
    pub fn take_ready_result(
        &mut self,
    ) -> Result<(LivingGalaxyStateV2, LivingPendingEventsV2), LivingLifecycleJobErrorV2> {
        if self.cancelled {
            return Err(LivingLifecycleJobErrorV2::Cancelled);
        }
        if let Some(fault) = self.fault.clone() {
            return Err(LivingLifecycleJobErrorV2::Fault(fault));
        }
        if self.stage != DispatchStage::Ready {
            return Err(LivingLifecycleJobErrorV2::NotReady);
        }
        if self.delivered {
            return Err(LivingLifecycleJobErrorV2::NotReady);
        }
        self.delivered = true;
        Ok((
            std::mem::take(&mut self.state),
            std::mem::take(&mut self.events),
        ))
    }
    fn advance(&mut self, stage: DispatchStage) {
        self.index = 0;
        self.stage = stage;
    }
    fn start_travel(&mut self) {
        self.duration = None;
        if let Some((_, from, _)) = self.metadata.get(&self.origin)
            && let Some((_, to, _)) = self.metadata.get(&self.destination)
        {
            if from == to {
                self.duration = Some(10);
            } else {
                self.search = Some(RetreatPathJob::begin(
                    std::sync::Arc::clone(&self.graph),
                    *from,
                    *to,
                ));
            }
        }
    }
    fn travel_visit(&mut self) -> bool {
        if let Some(search) = &mut self.search {
            if !search.ready() {
                return search.visit();
            }
            if let Some((duration, path)) = search
                .take_ready_result()
                .expect("undelivered ready search")
            {
                self.duration = Some(duration);
                self.path = path;
            }
            self.retirement
                .start(self.search.take().expect("retained ready search"));
            return true;
        }
        false
    }
    fn owned(&self, world: LivingEntityIdV2, actor: LivingEntityIdV2) -> bool {
        self.colonies.get(&world) == Some(&actor)
    }
    fn emit(&mut self, kind: LivingEventKindV2) -> Result<(), LivingDeterministicFaultV2> {
        self.events.record(
            LivingEventProvenanceV2::Autonomous {
                phase: LivingPhaseV2::DispatchOrders,
            },
            kind,
        )?;
        Ok(())
    }
    fn skip_route(&mut self) {
        self.route_index += 1;
        self.route = None;
        self.hazard = None;
        self.advance(DispatchStage::PathRetire);
    }
    fn evidence(&mut self, actual: u32) -> Result<(), LivingDeterministicFaultV2> {
        if !self.events.explanations_enabled() || self.eligible == 0 || self.batch >= self.eligible
        {
            return Ok(());
        }
        let Some(hazard) = self.hazard.take() else {
            return Ok(());
        };
        if self.events.corridor_restrictions.len() >= 4096 {
            self.hazard = Some(hazard);
            return Err(LivingDeterministicFaultV2::Capacity);
        }
        let route = self.route.as_ref().expect("active route");
        let source_system = self
            .metadata
            .get(&route.source_world)
            .ok_or(LivingDeterministicFaultV2::Invariant)?
            .1;
        let destination_system = self
            .metadata
            .get(&route.destination_world)
            .ok_or(LivingDeterministicFaultV2::Invariant)?
            .1;
        self.events
            .corridor_restrictions
            .push(super::event_facts::LivingCorridorRestrictionV2 {
                tick: self.tick,
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
                eligible_batch: self.eligible,
                restricted_batch: self.batch,
                actual_dispatched_units: actual,
                hazard,
            });
        Ok(())
    }
    fn visit(&mut self) -> Result<bool, LivingDeterministicFaultV2> {
        use super::model::LivingCivilizationStatusV2;
        use DispatchStage::*;
        match self.stage {
            Worlds => {
                if let Some(row) = self.state.worlds.as_slice().get(self.index) {
                    self.metadata
                        .insert(row.id, (self.index, row.system, row.owner));
                    self.index += 1;
                    return Ok(true);
                }
                self.advance(Colonies);
            }
            Colonies => {
                if let Some(row) = self.state.colonies.as_slice().get(self.index) {
                    self.colonies.insert(row.world, row.owner);
                    self.index += 1;
                    return Ok(true);
                }
                self.advance(Civilizations);
            }
            Civilizations => {
                if let Some(row) = self.state.civilizations.as_slice().get(self.index) {
                    if row.status == LivingCivilizationStatusV2::Dormant {
                        self.dormant.insert(row.id);
                    }
                    self.index += 1;
                    return Ok(true);
                }
                self.advance(HullJobs);
            }
            HullJobs => {
                if let Some(row) = self.state.hull_jobs.as_slice().get(self.index) {
                    if let Some(id) = row.target_fleet {
                        self.busy.insert(id);
                    }
                    self.index += 1;
                    return Ok(true);
                }
                self.advance(Observations);
            }
            Observations => {
                if let Some(row) = self.state.observations.as_slice().get(self.index) {
                    self.remembered.insert((row.civilization, row.world));
                    self.index += 1;
                    return Ok(true);
                }
                self.advance(Systems);
            }
            Systems => {
                if let Some(row) = self.state.systems.as_slice().get(self.index) {
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
                if let Some(row) = self.state.lanes.as_slice().get(self.index) {
                    std::sync::Arc::get_mut(&mut self.graph)
                        .expect("unshared graph")
                        .lanes
                        .push((row.id, row.system_a, row.system_b, row.distance_units));
                    self.lanes.insert(row.id, (row.system_a, row.system_b));
                    self.index += 1;
                    return Ok(true);
                }
                self.advance(Agreements);
            }
            Agreements => {
                if let Some(row) = self.state.agreements.as_slice().get(self.index) {
                    if row.kind == LivingAgreementKindV2::Trade
                        && row.start_tick.0 <= self.tick.0
                        && self.tick.0 < row.end_tick.0
                    {
                        self.trade.insert((row.participant_a, row.participant_b));
                    }
                    self.index += 1;
                    return Ok(true);
                }
                self.advance(CatalogHazards);
            }
            CatalogHazards => {
                if let Some(row) = self.catalog.hazards().get(self.index) {
                    self.hazard_effects
                        .insert(row.id.clone(), row.effect.freight_batch_scale());
                    self.index += 1;
                    return Ok(true);
                }
                self.advance(Shipments);
            }
            Shipments => {
                if let Some(row) = self.shipment_input.next() {
                    self.shipments.insert(row.id, row);
                    return Ok(true);
                }
                self.advance(Fleet);
            }
            Fleet => {
                let Some(fleet) = self.state.fleets.as_slice().get(self.fleet_index) else {
                    self.advance(Route);
                    return Ok(false);
                };
                if !self.busy.contains(&fleet.id)
                    && let LivingFleetLocationV2::Docked { world } = fleet.location
                    && let Some(order) = &fleet.order
                    && order.not_before_tick.0 <= self.tick.0
                    && let Some(destination) = order.target_world
                    && destination != world
                {
                    self.origin = world;
                    self.destination = destination;
                    self.start_travel();
                    self.advance(FleetTravel);
                } else {
                    self.fleet_index += 1;
                }
                return Ok(true);
            }
            FleetTravel => {
                if self.search.is_some() {
                    return Ok(self.travel_visit());
                }
                if self.duration.is_none() {
                    self.fleet_index += 1;
                    self.advance(Fleet);
                } else {
                    self.hull_index = 0;
                    self.missing_credit = 0;
                    self.has_ark = false;
                    self.advance(FleetHulls);
                }
            }
            FleetHulls => {
                let fleet = &self.state.fleets.as_slice()[self.fleet_index];
                if let Some(hull) = fleet.hulls.as_slice().get(self.hull_index) {
                    self.missing_credit += u32::from(!hull.return_credit);
                    self.has_ark |= hull.definition.as_str() == "colony_ark";
                    self.hull_index += 1;
                    return Ok(true);
                }
                self.advance(FleetSpend);
            }
            FleetSpend => {
                let fleet = &self.state.fleets.as_slice()[self.fleet_index];
                let order = fleet.order.as_ref().expect("eligible order");
                let returning = order.kind == LivingFleetOrderKindV2::Return;
                let recovery = order.kind == LivingFleetOrderKindV2::Settle
                    && self.dormant.contains(&fleet.owner)
                    && self
                        .metadata
                        .get(&self.destination)
                        .is_some_and(|row| row.2.is_none())
                    && self.remembered.contains(&(fleet.owner, self.destination))
                    && self.has_ark;
                let own_origin = self.owned(self.origin, fleet.owner);
                self.consumes_credit = returning || (!own_origin && recovery);
                let admitted = if self.consumes_credit {
                    (!returning || self.owned(self.destination, fleet.owner))
                        && self.missing_credit == 0
                } else if own_origin {
                    let cost = (fleet.hulls.as_slice().len() as u32).saturating_mul(2)
                        + self.missing_credit.saturating_mul(2);
                    if let Some((index, _, _)) = self.metadata.get(&self.origin) {
                        let inventory = &mut self
                            .state
                            .worlds
                            .record_at_mut(*index)
                            .expect("indexed world")
                            .inventory;
                        if inventory.energy >= cost {
                            inventory.energy -= cost;
                            true
                        } else {
                            false
                        }
                    } else {
                        false
                    }
                } else {
                    false
                };
                self.permitted = admitted;
                if admitted {
                    self.grant_credit = !returning && own_origin;
                    self.hull_index = 0;
                    self.advance(FleetCredits);
                } else {
                    self.retired_path = std::mem::take(&mut self.path);
                    self.advance(FleetPathRetire);
                }
                return Ok(true);
            }
            FleetCredits => {
                let fleet = self
                    .state
                    .fleets
                    .record_at_mut(self.fleet_index)
                    .expect("indexed fleet");
                if let Some(hull) = fleet.hulls.record_at_mut(self.hull_index) {
                    hull.return_credit =
                        !self.consumes_credit && (self.grant_credit || hull.return_credit);
                    self.hull_index += 1;
                    return Ok(true);
                }
                let order = fleet.order.as_mut().expect("eligible order");
                self.retired_path =
                    std::mem::replace(&mut order.lane_path, std::mem::take(&mut self.path));
                self.advance(FleetPathRetire);
            }
            FleetPathRetire => {
                if self.retired_path.pop().is_some() {
                    return Ok(true);
                }
                if self.permitted {
                    self.advance(FleetDepart);
                } else {
                    self.fleet_index += 1;
                    self.advance(Fleet);
                }
            }
            FleetDepart => {
                let arrival_tick = self
                    .tick
                    .0
                    .checked_add(self.duration.expect("admitted travel"))
                    .ok_or(LivingDeterministicFaultV2::Arithmetic)?;
                self.state
                    .fleets
                    .record_at_mut(self.fleet_index)
                    .expect("indexed fleet")
                    .location = LivingFleetLocationV2::Travelling {
                    origin_world: self.origin,
                    destination_world: self.destination,
                    departure_tick: self.tick,
                    arrival_tick: LivingTickV2(arrival_tick),
                };
                self.emit(LivingEventKindV2::FleetDeparted)?;
                self.fleet_index += 1;
                self.advance(Fleet);
                return Ok(true);
            }
            Route => {
                let Some(route) = self.state.routes.as_slice().get(self.route_index) else {
                    self.advance(ShipmentOutput);
                    return Ok(false);
                };
                if route.next_due.0 > self.tick.0 {
                    self.route_index += 1;
                    return Ok(true);
                }
                self.origin = route.source_world;
                self.destination = route.destination_world;
                self.route = Some(route.clone());
                self.hazard = None;
                self.start_travel();
                self.advance(RouteTravel);
                return Ok(true);
            }
            RouteTravel => {
                if self.search.is_some() {
                    return Ok(self.travel_visit());
                }
                self.advance(RouteAdvance);
            }
            RouteAdvance => {
                let route = self.route.as_ref().expect("active route");
                let participants = self.metadata.get(&route.source_world).and_then(|row| row.2)
                    == Some(route.source_owner)
                    && self
                        .metadata
                        .get(&route.destination_world)
                        .and_then(|row| row.2)
                        == Some(route.receiver_owner);
                self.permitted = !route.suspended
                    && self.duration.is_some()
                    && participants
                    && match route.kind {
                        LivingRouteKindV2::Internal => route.source_owner == route.receiver_owner,
                        LivingRouteKindV2::Aid => true,
                        LivingRouteKindV2::Trade => self.trade.contains(&(
                            route.source_owner.min(route.receiver_owner),
                            route.source_owner.max(route.receiver_owner),
                        )),
                    };
                let stored = self
                    .state
                    .routes
                    .record_at_mut(self.route_index)
                    .expect("indexed route");
                stored.suspended = route.suspended || !participants;
                stored.next_due = LivingTickV2(
                    route
                        .next_due
                        .0
                        .checked_add(u64::from(route.cadence_ticks))
                        .ok_or(LivingDeterministicFaultV2::Arithmetic)?,
                );
                if !self.permitted {
                    self.skip_route();
                } else {
                    let available = self
                        .metadata
                        .get(&route.source_world)
                        .map(|row| {
                            self.state.worlds.as_slice()[row.0]
                                .inventory
                                .quantity(route.resource)
                        })
                        .unwrap_or(0);
                    self.eligible = route
                        .batch_size
                        .min(available.saturating_sub(route.source_reserve));
                    self.batch = self.eligible;
                    self.advance(RouteHazards);
                }
                return Ok(true);
            }
            RouteHazards => {
                let route = self.route.as_ref().expect("active route");
                let from = self
                    .metadata
                    .get(&route.source_world)
                    .expect("reachable source")
                    .1;
                let to = self
                    .metadata
                    .get(&route.destination_world)
                    .expect("reachable destination")
                    .1;
                if from != to
                    && let Some(row) = self.state.hazards.as_slice().get(self.index)
                {
                    if row.start_tick.0 <= self.tick.0
                        && self.tick.0 < row.end_tick.0
                        && let Some(&(a, b)) = self.lanes.get(&row.lane)
                        && ((a == from && b == to) || (b == from && a == to))
                        && let Some(&(num, den)) = self.hazard_effects.get(&row.definition)
                    {
                        let next = self.eligible / den * num + self.eligible % den * num / den;
                        if next < self.batch {
                            self.batch = next;
                            self.hazard = Some(row.clone());
                        }
                    }
                    self.index += 1;
                    return Ok(true);
                }
                self.advance(RouteAdmit);
            }
            RouteAdmit => {
                if self.shipments.len() >= super::model::LIVING_MAX_SHIPMENTS_V2 {
                    self.skip_route();
                    return Ok(true);
                }
                if self.batch == 0 {
                    self.evidence(0)?;
                    self.skip_route();
                    return Ok(true);
                }
                let route = self.route.as_ref().expect("active route");
                let Some((index, _, _)) = self.metadata.get(&route.source_world) else {
                    self.skip_route();
                    return Ok(true);
                };
                let inventory = &mut self
                    .state
                    .worlds
                    .record_at_mut(*index)
                    .expect("indexed world")
                    .inventory;
                let current = inventory.quantity(route.resource);
                if current < self.batch {
                    self.skip_route();
                    return Ok(true);
                }
                match route.resource {
                    LivingResourceV2::Alloy => inventory.alloy = current - self.batch,
                    LivingResourceV2::Energy => inventory.energy = current - self.batch,
                    LivingResourceV2::Ore => inventory.ore = current - self.batch,
                }
                self.advance(RouteCreate);
                return Ok(true);
            }
            RouteCreate => {
                let route = self.route.as_ref().expect("active route");
                let id = super::ids::autonomous_entity_id_v2(
                    &super::ids::LivingAutonomousEntityInputsV2 {
                        catalog_hash: self.catalog.catalog_hash(),
                        genesis_seed: self.genesis_seed,
                        branch: self.branch,
                        tick: self.tick,
                        phase: LivingPhaseV2::DispatchOrders.ordinal(),
                        actor: route.id,
                        intent_ordinal: 1,
                        entity_kind: super::ids::LivingEntityKindV2::Shipment.ordinal(),
                        local_id: 0,
                    },
                );
                let arrival_tick = LivingTickV2(
                    self.tick
                        .0
                        .checked_add(self.duration.unwrap_or(10))
                        .ok_or(LivingDeterministicFaultV2::Arithmetic)?,
                );
                self.retired_shipment = self.shipments.insert(
                    id,
                    LivingShipmentV2 {
                        id,
                        dispatch_owner: route.source_owner,
                        intended_receiver: route.receiver_owner,
                        source_world: route.source_world,
                        destination_world: route.destination_world,
                        resource: route.resource,
                        units: self.batch,
                        departure_tick: self.tick,
                        arrival_tick,
                        disposition: LivingShipmentDispositionV2::Outbound,
                        dispatch_kind: Some(route.kind),
                        returning_to_source: false,
                    },
                );
                self.advance(ShipmentRetire);
                return Ok(true);
            }
            ShipmentRetire => {
                if self.retired_shipment.take().is_some() {
                    return Ok(true);
                }
                self.advance(RouteEmit);
            }
            RouteEmit => {
                self.emit(LivingEventKindV2::ShipmentDispatched)?;
                self.advance(RouteEvidence);
                return Ok(true);
            }
            RouteEvidence => {
                self.evidence(self.batch)?;
                self.skip_route();
                return Ok(true);
            }
            PathRetire => {
                if self.path.pop().is_some() {
                    return Ok(true);
                }
                self.advance(Route);
            }
            ShipmentOutput => {
                if let Some((_, row)) = self.shipments.pop_first() {
                    self.state.shipments.push_sorted(row)?;
                    return Ok(true);
                }
                self.advance(Retire);
            }
            Retire => {
                if self.metadata.pop_first().is_some()
                    || self.colonies.pop_first().is_some()
                    || self.dormant.pop_first().is_some()
                    || self.busy.pop_first().is_some()
                    || self.remembered.pop_first().is_some()
                    || self.trade.pop_first().is_some()
                    || self.lanes.pop_first().is_some()
                    || self.hazard_effects.pop_first().is_some()
                {
                    return Ok(true);
                }
                let graph = std::sync::Arc::get_mut(&mut self.graph).expect("searches retired");
                if graph.lanes.pop().is_some() || graph.systems.pop().is_some() {
                    return Ok(true);
                }
                self.advance(Ready);
            }
            Ready => {}
        }
        Ok(false)
    }
}
impl LivingDispatchJobV2<'static> {
    /// Move a catalog handle and candidate without scanning or cloning records.
    pub fn begin_owned(
        catalog: std::sync::Arc<super::catalog::ValidatedLivingCatalogPackV2>,
        state: LivingGalaxyStateV2,
        events: LivingPendingEventsV2,
        tick: LivingTickV2,
        genesis_seed: [u8; 32],
        branch: super::ids::LivingBranchIdV2,
    ) -> Self {
        Self::with_catalog(
            LivingPhaseCatalogV2::Owned(catalog),
            state,
            events,
            tick,
            genesis_seed,
            branch,
        )
    }
}

#[cfg(test)]
mod dispatch_tests {
    use super::super::model::*;
    use super::super::simulation::LivingGalaxyAuthorityV2;
    use super::super::wire::LivingSortedVecV2;
    use super::*;
    fn id(value: u8) -> LivingEntityIdV2 {
        let mut bytes = [0; 16];
        bytes[0] = value;
        LivingEntityIdV2(bytes)
    }
    fn fixture() -> (LivingGalaxyAuthorityV2, LivingGalaxyStateV2) {
        let authority = crate::living::builtin_living_sandbox_v2().unwrap();
        let mut state = authority.state().clone();
        state.fleets = Default::default();
        state.routes = Default::default();
        state.shipments = Default::default();
        state.hull_jobs = Default::default();
        state.observations = Default::default();
        state.agreements = Default::default();
        state.hazards = Default::default();
        state.systems = LivingSortedVecV2::new(
            (100..=103)
                .map(|n| LivingSystemV2 {
                    id: id(n),
                    name: crate::living::LivingNameV2::new(format!("Dispatch {n}")).unwrap(),
                })
                .collect(),
        )
        .unwrap();
        for index in 0..state.worlds.as_slice().len() {
            state.worlds.record_at_mut(index).unwrap().system =
                if index == 0 { id(100) } else { id(103) };
        }
        state.lanes = LivingSortedVecV2::new(vec![
            LivingLaneV2 {
                id: id(110),
                system_a: id(100),
                system_b: id(101),
                distance_units: 100,
            },
            LivingLaneV2 {
                id: id(111),
                system_a: id(100),
                system_b: id(102),
                distance_units: 100,
            },
            LivingLaneV2 {
                id: id(112),
                system_a: id(102),
                system_b: id(103),
                distance_units: 100,
            },
            LivingLaneV2 {
                id: id(113),
                system_a: id(101),
                system_b: id(103),
                distance_units: 100,
            },
        ])
        .unwrap();
        (authority, state)
    }
    fn fleet(
        fleet_id: u8,
        owner: u8,
        world: u8,
        kind: LivingFleetOrderKindV2,
        target: u8,
        credits: &[bool],
    ) -> LivingFleetV2 {
        LivingFleetV2 {
            id: id(fleet_id),
            owner: id(owner),
            location: LivingFleetLocationV2::Docked { world: id(world) },
            order: Some(LivingFleetOrderV2 {
                kind,
                target_world: Some(id(target)),
                not_before_tick: LivingTickV2(99),
                lane_path: vec![id(240), id(241)],
            }),
            hulls: LivingSortedVecV2::new(
                credits
                    .iter()
                    .enumerate()
                    .map(|(n, credit)| LivingHullV2 {
                        id: id(fleet_id + 1 + n as u8),
                        definition: crate::living::LivingSlugV2::new("escort").unwrap(),
                        hit_points: 0,
                        return_credit: *credit,
                        next_repair: None,
                    })
                    .collect(),
            )
            .unwrap(),
        }
    }
    fn route(
        route_id: u8,
        kind: LivingRouteKindV2,
        source: u8,
        destination: u8,
        receiver: u8,
    ) -> LivingFreightRouteV2 {
        LivingFreightRouteV2 {
            id: id(route_id),
            kind,
            source_world: id(source),
            destination_world: id(destination),
            resource: LivingResourceV2::Energy,
            batch_size: 11,
            cadence_ticks: 10,
            source_reserve: 0,
            source_owner: id(3),
            receiver_owner: id(receiver),
            next_due: LivingTickV2(100),
            suspended: false,
        }
    }
    fn reference(
        authority: &LivingGalaxyAuthorityV2,
        mut state: LivingGalaxyStateV2,
        mut events: LivingPendingEventsV2,
        tick: LivingTickV2,
    ) -> Result<(LivingGalaxyStateV2, LivingPendingEventsV2), LivingDeterministicFaultV2> {
        super::super::rules::reference_dispatch_for_cursor_test(
            &mut super::super::simulation::LivingStepContextV2 {
                catalog: authority.catalog(),
                genesis_seed: [0; 32],
                branch: authority.branch(),
                tick,
                state: &mut state,
                events: &mut events,
                arrivals: Vec::new(),
            },
        )?;
        Ok((state, events))
    }
    fn parity(
        authority: &LivingGalaxyAuthorityV2,
        state: &LivingGalaxyStateV2,
        events: LivingPendingEventsV2,
        tick: LivingTickV2,
    ) -> (LivingGalaxyStateV2, LivingPendingEventsV2) {
        let expected = reference(authority, state.clone(), events.clone(), tick).unwrap();
        let mut total = None;
        for budgets in [&[1][..], &[64][..], &[1024][..], &[1, 64, 7, 1024][..]] {
            let mut job = LivingDispatchJobV2::begin(
                authority.catalog(),
                state.clone(),
                events.clone(),
                tick,
                [0; 32],
                authority.branch(),
            );
            let mut turn = 0;
            loop {
                let budget = budgets[turn % budgets.len()];
                turn += 1;
                let progress = job.poll(budget).unwrap();
                assert!(job.last_poll_visits() <= budget);
                if progress == LivingLifecycleProgressV2::Ready {
                    break;
                }
            }
            assert!(
                job.metadata.is_empty()
                    && job.colonies.is_empty()
                    && job.dormant.is_empty()
                    && job.busy.is_empty()
            );
            assert!(
                job.remembered.is_empty()
                    && job.trade.is_empty()
                    && job.lanes.is_empty()
                    && job.hazard_effects.is_empty()
            );
            assert!(job.graph.systems.is_empty() && job.graph.lanes.is_empty());
            assert!(
                job.search.is_none()
                    && job.retired_shipment.is_none()
                    && job.path.is_empty()
                    && job.retired_path.is_empty()
                    && job.shipments.is_empty()
            );
            assert!(job.route.is_none() && job.hazard.is_none());
            if let Some(total) = total {
                assert_eq!(job.total_visits(), total);
            } else {
                total = Some(job.total_visits());
            }
            assert_eq!(job.poll(1).unwrap(), LivingLifecycleProgressV2::Ready);
            assert_eq!(job.last_poll_visits(), 0);
            let actual = job.finish().unwrap();
            assert_eq!(actual, expected);
        }
        expected
    }
    #[test]
    fn dispatch_cursor_fleet_route_shared_stock_and_lane_path_ties() {
        let (authority, mut state) = fixture();
        state.worlds.find_mut(&id(5)).unwrap().inventory.energy = 15;
        state.fleets = LivingSortedVecV2::new(vec![
            fleet(
                30,
                3,
                5,
                LivingFleetOrderKindV2::Explore,
                7,
                &[false, false, false],
            ),
            fleet(
                40,
                3,
                5,
                LivingFleetOrderKindV2::Explore,
                7,
                &[false, false, false],
            ),
        ])
        .unwrap();
        state.routes =
            LivingSortedVecV2::new(vec![route(60, LivingRouteKindV2::Internal, 5, 8, 3)]).unwrap();
        let (actual, events) = parity(&authority, &state, Default::default(), LivingTickV2(100));
        assert_eq!(actual.worlds.as_slice()[0].inventory.energy, 0);
        assert!(matches!(
            actual.fleets.as_slice()[0].location,
            LivingFleetLocationV2::Travelling { .. }
        ));
        assert_eq!(
            actual.fleets.as_slice()[0]
                .order
                .as_ref()
                .unwrap()
                .lane_path,
            vec![id(110), id(113)]
        );
        assert!(matches!(
            actual.fleets.as_slice()[1].location,
            LivingFleetLocationV2::Docked { .. }
        ));
        assert_eq!(actual.shipments.as_slice()[0].units, 3);
        assert_eq!(events.payloads().len(), 2);
        state.worlds.find_mut(&id(5)).unwrap().inventory.energy = 20;
        state
            .fleets
            .record_at_mut(1)
            .unwrap()
            .hulls
            .record_at_mut(0)
            .unwrap()
            .return_credit = true;
        state
            .fleets
            .record_at_mut(1)
            .unwrap()
            .hulls
            .record_at_mut(1)
            .unwrap()
            .return_credit = true;
        let (actual, _) = parity(&authority, &state, Default::default(), LivingTickV2(100));
        assert!(
            actual
                .fleets
                .as_slice()
                .iter()
                .all(|f| matches!(f.location, LivingFleetLocationV2::Travelling { .. }))
        );
        assert!(actual.shipments.as_slice().is_empty());
    }
    #[test]
    fn dispatch_cursor_foreign_return_and_dormant_recovery_credit_predicates() {
        for axis in 0..7 {
            let (authority, mut state) = fixture();
            let mut row = fleet(30, 3, 6, LivingFleetOrderKindV2::Return, 5, &[true, true]);
            if axis == 1 {
                row.hulls.record_at_mut(0).unwrap().return_credit = false;
            }
            if axis == 2 {
                row.order.as_mut().unwrap().kind = LivingFleetOrderKindV2::Explore;
            }
            if axis >= 3 {
                // A real Dormant recovery starts without owned colonies.
                state.colonies = LivingSortedVecV2::new(
                    state
                        .colonies
                        .as_slice()
                        .iter()
                        .filter(|row| row.owner != id(3))
                        .cloned()
                        .collect(),
                )
                .unwrap();
                for index in 0..state.worlds.as_slice().len() {
                    let world = state.worlds.record_at_mut(index).unwrap();
                    if world.owner == Some(id(3)) {
                        world.owner = None;
                    }
                }
                state.civilizations.find_mut(&id(3)).unwrap().status =
                    LivingCivilizationStatusV2::Dormant;
                row.order.as_mut().unwrap().kind = LivingFleetOrderKindV2::Settle;
                row.order.as_mut().unwrap().target_world = Some(id(7));
                row.hulls.record_at_mut(0).unwrap().definition =
                    crate::living::LivingSlugV2::new("colony_ark").unwrap();
                if axis != 4 {
                    state.observations = LivingSortedVecV2::new(vec![LivingObservationV2 {
                        civilization: id(3),
                        world: id(7),
                        observed_tick: LivingTickV2(1),
                        owner: None,
                        remaining_deposit: 0,
                        facilities: Default::default(),
                        stationed_hulls: Default::default(),
                        operational_defense_batteries: None,
                    }])
                    .unwrap();
                }
                if axis == 5 {
                    row.hulls.record_at_mut(1).unwrap().return_credit = false;
                }
                if axis == 6 {
                    row.hulls.record_at_mut(0).unwrap().definition =
                        crate::living::LivingSlugV2::new("escort").unwrap();
                }
            }
            state.fleets = LivingSortedVecV2::new(vec![row]).unwrap();
            let (actual, _) = parity(&authority, &state, Default::default(), LivingTickV2(100));
            assert_eq!(
                matches!(
                    actual.fleets.as_slice()[0].location,
                    LivingFleetLocationV2::Travelling { .. }
                ),
                axis == 0 || axis == 3
            );
            assert_eq!(
                actual
                    .worlds
                    .as_slice()
                    .iter()
                    .find(|w| w.id == id(6))
                    .unwrap()
                    .inventory
                    .energy,
                100
            );
            if axis == 0 || axis == 3 {
                assert!(
                    actual.fleets.as_slice()[0]
                        .hulls
                        .as_slice()
                        .iter()
                        .all(|h| !h.return_credit)
                );
            }
        }
    }
    #[test]
    fn dispatch_cursor_due_busy_missing_order_and_unreachable_paths() {
        for axis in 0..6 {
            let (authority, mut state) = fixture();
            let mut row = fleet(30, 3, 5, LivingFleetOrderKindV2::Explore, 7, &[false]);
            match axis {
                0 => row.order.as_mut().unwrap().not_before_tick = LivingTickV2(101),
                1 => row.order = None,
                2 => row.order.as_mut().unwrap().target_world = Some(id(5)),
                3 => state.lanes = Default::default(),
                4 => {
                    state.hull_jobs = LivingSortedVecV2::new(vec![LivingHullJobV2 {
                        id: id(60),
                        world: id(5),
                        owner: id(3),
                        definition: crate::living::LivingSlugV2::new("escort").unwrap(),
                        accepted_tick: LivingTickV2(1),
                        completion_tick: LivingTickV2(200),
                        paid_alloy: 1,
                        target_fleet: Some(id(30)),
                    }])
                    .unwrap()
                }
                _ => {
                    row.location = LivingFleetLocationV2::Travelling {
                        origin_world: id(5),
                        destination_world: id(7),
                        departure_tick: LivingTickV2(1),
                        arrival_tick: LivingTickV2(200),
                    }
                }
            }
            state.fleets = LivingSortedVecV2::new(vec![row]).unwrap();
            let (actual, events) =
                parity(&authority, &state, Default::default(), LivingTickV2(100));
            assert_eq!(actual, state);
            assert!(events.payloads().is_empty());
        }
    }
    #[test]
    fn dispatch_cursor_route_contracts_clock_and_shared_reserves() {
        for axis in 0..8 {
            let (authority, mut state) = fixture();
            let mut first = route(60, LivingRouteKindV2::Trade, 5, 6, 4);
            if axis == 0 {
                first.kind = LivingRouteKindV2::Aid;
            }
            if axis == 1 {
                first.kind = LivingRouteKindV2::Internal;
            }
            if axis == 2 {
                first.suspended = true;
            }
            if axis == 3 {
                first.next_due = LivingTickV2(101);
            }
            if axis == 4 {
                state.worlds.find_mut(&id(6)).unwrap().owner = None;
            }
            if axis == 5 || axis == 6 {
                state.agreements = LivingSortedVecV2::new(vec![LivingAgreementV2 {
                    participant_a: id(3),
                    participant_b: id(4),
                    kind: LivingAgreementKindV2::Trade,
                    start_tick: LivingTickV2(1),
                    end_tick: LivingTickV2(if axis == 5 { 101 } else { 100 }),
                }])
                .unwrap();
            }
            if axis == 7 {
                first.kind = LivingRouteKindV2::Internal;
                first.destination_world = id(8);
                first.receiver_owner = id(3);
                first.source_reserve = 3;
                state.worlds.find_mut(&id(5)).unwrap().inventory.energy = 20;
            }
            let mut rows = vec![first];
            if axis == 7 {
                let mut second = rows[0].clone();
                second.id = id(61);
                rows.push(second);
            }
            state.routes = LivingSortedVecV2::new(rows).unwrap();
            let (actual, _) = parity(&authority, &state, Default::default(), LivingTickV2(100));
            assert_eq!(
                actual.shipments.as_slice().len(),
                if axis == 7 {
                    2
                } else {
                    usize::from(axis == 0 || axis == 5)
                }
            );
            assert_eq!(
                actual.routes.as_slice()[0].next_due,
                LivingTickV2(if axis == 3 { 101 } else { 110 })
            );
            assert_eq!(
                actual.routes.as_slice()[0].suspended,
                axis == 2 || axis == 4
            );
            if axis == 7 {
                assert_eq!(actual.worlds.as_slice()[0].inventory.energy, 3);
                assert_eq!(
                    actual
                        .shipments
                        .as_slice()
                        .iter()
                        .map(|s| s.units)
                        .sum::<u32>(),
                    17
                );
            }
        }
    }
    #[test]
    fn dispatch_cursor_direct_hazard_ties_half_open_and_forecast_evidence() {
        for axis in 0..7 {
            let (authority, mut state) = fixture();
            super::super::rules::put(
                &mut state.lanes,
                LivingLaneV2 {
                    id: id(114),
                    system_a: id(100),
                    system_b: id(103),
                    distance_units: 500,
                },
            )
            .unwrap();
            let mut row = route(60, LivingRouteKindV2::Internal, 5, 8, 3);
            if axis == 4 {
                row.batch_size = 1;
            }
            state.routes = LivingSortedVecV2::new(vec![row]).unwrap();
            let hazard = LivingHazardV2 {
                id: id(70),
                definition: crate::living::LivingSlugV2::new("ion_storm").unwrap(),
                lane: id(if axis == 0 { 110 } else { 114 }),
                start_tick: LivingTickV2(if axis == 1 { 101 } else { 100 }),
                end_tick: LivingTickV2(if axis == 2 { 100 } else { 101 }),
            };
            let mut second = hazard.clone();
            second.id = id(71);
            state.hazards = LivingSortedVecV2::new(vec![hazard, second]).unwrap();
            let events = if axis == 6 {
                LivingPendingEventsV2::for_forecast()
            } else {
                Default::default()
            };
            let (actual, events) = parity(&authority, &state, events, LivingTickV2(100));
            let restricted = axis >= 3;
            let units = if axis == 4 {
                0
            } else if restricted {
                5
            } else {
                11
            };
            assert_eq!(
                actual
                    .shipments
                    .as_slice()
                    .iter()
                    .map(|s| s.units)
                    .sum::<u32>(),
                units
            );
            assert_eq!(
                events.corridor_restrictions.len(),
                usize::from(restricted && axis != 6)
            );
            if let Some(proof) = events.corridor_restrictions.first() {
                assert_eq!(proof.hazard.id, id(70));
                assert_eq!(proof.actual_dispatched_units, units);
            }
        }
    }

    #[test]
    fn dispatch_cursor_shipment_capacity_replacement_and_zero_hazard_admission() {
        for full in [false, true] {
            let (authority, mut state) = fixture();
            state.routes =
                LivingSortedVecV2::new(vec![route(60, LivingRouteKindV2::Internal, 5, 8, 3)])
                    .unwrap();
            let (_, expected) = reference(
                &authority,
                state.clone(),
                Default::default(),
                LivingTickV2(100),
            )
            .unwrap();
            assert_eq!(expected.payloads().len(), 1);
            let (generated, _) = reference(
                &authority,
                state.clone(),
                Default::default(),
                LivingTickV2(100),
            )
            .unwrap();
            let mut old = generated.shipments.as_slice()[0].clone();
            old.units = 99;
            if full {
                let template = old.clone();
                state.shipments = LivingSortedVecV2::new(
                    (0..LIVING_MAX_SHIPMENTS_V2)
                        .map(|n| {
                            let mut row = template.clone();
                            row.id = LivingEntityIdV2((n as u128 + 1000).to_be_bytes());
                            row
                        })
                        .collect(),
                )
                .unwrap();
            } else {
                state.shipments = LivingSortedVecV2::new(vec![old]).unwrap();
            }
            let (actual, events) =
                parity(&authority, &state, Default::default(), LivingTickV2(100));
            if full {
                assert_eq!(actual.shipments, state.shipments);
                assert_eq!(actual.worlds, state.worlds);
                assert!(events.payloads().is_empty());
            } else {
                assert_eq!(actual.shipments.as_slice().len(), 1);
                assert_eq!(actual.shipments.as_slice()[0].units, 11);
            }
        }
        let (authority, mut state) = fixture();
        state.routes =
            LivingSortedVecV2::new(vec![route(60, LivingRouteKindV2::Internal, 5, 8, 3)]).unwrap();
        state.routes.record_at_mut(0).unwrap().batch_size = 1;
        super::super::rules::put(
            &mut state.lanes,
            LivingLaneV2 {
                id: id(114),
                system_a: id(100),
                system_b: id(103),
                distance_units: 100,
            },
        )
        .unwrap();
        state.hazards = LivingSortedVecV2::new(vec![LivingHazardV2 {
            id: id(70),
            definition: crate::living::LivingSlugV2::new("ion_storm").unwrap(),
            lane: id(114),
            start_tick: LivingTickV2(100),
            end_tick: LivingTickV2(101),
        }])
        .unwrap();
        let template = LivingShipmentV2 {
            id: id(90),
            dispatch_owner: id(3),
            intended_receiver: id(3),
            source_world: id(5),
            destination_world: id(8),
            resource: LivingResourceV2::Energy,
            units: 1,
            departure_tick: LivingTickV2(1),
            arrival_tick: LivingTickV2(200),
            disposition: LivingShipmentDispositionV2::Outbound,
            dispatch_kind: Some(LivingRouteKindV2::Internal),
            returning_to_source: false,
        };
        state.shipments = LivingSortedVecV2::new(
            (0..LIVING_MAX_SHIPMENTS_V2)
                .map(|n| {
                    let mut row = template.clone();
                    row.id = LivingEntityIdV2((n as u128 + 1000).to_be_bytes());
                    row
                })
                .collect(),
        )
        .unwrap();
        let (_, events) = parity(&authority, &state, Default::default(), LivingTickV2(100));
        assert!(events.corridor_restrictions.is_empty());
    }
    #[test]
    fn dispatch_cursor_faults_are_sticky_after_private_spending_and_event_capacity() {
        for axis in 0..4 {
            let (authority, mut state) = fixture();
            let mut events = LivingPendingEventsV2::default();
            let tick = LivingTickV2(if axis < 2 { u64::MAX - 5 } else { 100 });
            if axis == 0 {
                state.fleets = LivingSortedVecV2::new(vec![fleet(
                    30,
                    3,
                    5,
                    LivingFleetOrderKindV2::Explore,
                    7,
                    &[false, true],
                )])
                .unwrap();
            } else {
                state.routes =
                    LivingSortedVecV2::new(vec![route(60, LivingRouteKindV2::Internal, 5, 8, 3)])
                        .unwrap();
            }
            if axis == 2 {
                state.routes.record_at_mut(0).unwrap().next_due = LivingTickV2(u64::MAX - 1);
                state.routes.record_at_mut(0).unwrap().cadence_ticks = 10;
            }
            let tick = if axis == 2 {
                LivingTickV2(u64::MAX - 1)
            } else {
                tick
            };
            if axis == 3 {
                for _ in 0..super::super::receipt::LIVING_MAX_BOUNDARY_EVENTS_V2 {
                    events
                        .record(
                            LivingEventProvenanceV2::Autonomous {
                                phase: LivingPhaseV2::DispatchOrders,
                            },
                            LivingEventKindV2::ShipmentDispatched,
                        )
                        .unwrap();
                }
            }
            let expected = reference(&authority, state.clone(), events.clone(), tick).unwrap_err();
            for budget in [1, 64, 1024] {
                let mut job = LivingDispatchJobV2::begin(
                    authority.catalog(),
                    state.clone(),
                    events.clone(),
                    tick,
                    [0; 32],
                    authority.branch(),
                );
                loop {
                    let result = job.poll(budget);
                    assert!(job.last_poll_visits() <= budget);
                    match result {
                        Ok(LivingLifecycleProgressV2::Pending) => {}
                        Err(LivingLifecycleJobErrorV2::Fault(fault)) => {
                            assert_eq!(fault, expected);
                            break;
                        }
                        other => panic!("unexpected {other:?}"),
                    }
                }
                assert_eq!(
                    job.poll(1),
                    Err(LivingLifecycleJobErrorV2::Fault(expected.clone()))
                );
                assert_eq!(job.last_poll_visits(), 0);
                if axis != 2 {
                    assert!(
                        job.state.worlds.as_slice()[0].inventory.energy
                            < state.worlds.as_slice()[0].inventory.energy
                    );
                }
                if axis == 0 {
                    assert!(
                        job.state.fleets.as_slice()[0]
                            .hulls
                            .as_slice()
                            .iter()
                            .all(|h| h.return_credit)
                    );
                }
                assert!(matches!(
                    job.finish(),
                    Err(LivingLifecycleJobErrorV2::Fault(_))
                ));
            }
        }
    }
    #[test]
    fn dispatch_cursor_cancel_retry_and_counted_retirement() {
        for target in [
            DispatchStage::FleetTravel,
            DispatchStage::FleetCredits,
            DispatchStage::FleetPathRetire,
            DispatchStage::RouteHazards,
            DispatchStage::ShipmentOutput,
            DispatchStage::Retire,
        ] {
            let (authority, mut state) = fixture();
            state.fleets = LivingSortedVecV2::new(vec![fleet(
                30,
                3,
                5,
                LivingFleetOrderKindV2::Explore,
                7,
                &[false; 16],
            )])
            .unwrap();
            state.routes =
                LivingSortedVecV2::new(vec![route(60, LivingRouteKindV2::Internal, 5, 8, 3)])
                    .unwrap();
            state.hazards = LivingSortedVecV2::new(vec![LivingHazardV2 {
                id: id(70),
                definition: crate::living::LivingSlugV2::new("ion_storm").unwrap(),
                lane: id(110),
                start_tick: LivingTickV2(100),
                end_tick: LivingTickV2(101),
            }])
            .unwrap();
            let mut job = LivingDispatchJobV2::begin(
                authority.catalog(),
                state,
                Default::default(),
                LivingTickV2(100),
                [0; 32],
                authority.branch(),
            );
            assert_eq!(job.poll(0), Err(LivingLifecycleJobErrorV2::Budget));
            assert_eq!(job.total_visits(), 0);
            assert_eq!(job.poll(1025), Err(LivingLifecycleJobErrorV2::Budget));
            loop {
                let progress = job.poll(1).unwrap();
                if job.stage == target {
                    break;
                }
                assert_ne!(
                    progress,
                    LivingLifecycleProgressV2::Ready,
                    "target stage not reached"
                );
            }
            let visits = job.total_visits();
            job.cancel();
            assert_eq!(job.poll(64), Err(LivingLifecycleJobErrorV2::Cancelled));
            assert_eq!(job.last_poll_visits(), 0);
            assert_eq!(job.total_visits(), visits);
            assert!(matches!(
                job.finish(),
                Err(LivingLifecycleJobErrorV2::Cancelled)
            ));
        }
        let (authority, state) = fixture();
        let job = LivingDispatchJobV2::begin(
            authority.catalog(),
            state,
            Default::default(),
            LivingTickV2(100),
            [0; 32],
            authority.branch(),
        );
        assert!(matches!(
            job.finish(),
            Err(LivingLifecycleJobErrorV2::NotReady)
        ));
    }
    #[test]
    fn dispatch_cursor_zero_many_actors_local_and_saturated_graphs() {
        for axis in 0..4 {
            let (authority, mut state) = fixture();
            if axis == 0 {
                state.civilizations = Default::default();
                state.colonies = Default::default();
            }
            if axis == 1 {
                let template = state.civilizations.as_slice()[0].clone();
                state.civilizations = LivingSortedVecV2::new(
                    (1..=30)
                        .map(|n| {
                            let mut row = template.clone();
                            row.id = id(n);
                            row
                        })
                        .collect(),
                )
                .unwrap();
            }
            if axis >= 2 {
                state.fleets = LivingSortedVecV2::new(vec![fleet(
                    30,
                    3,
                    5,
                    LivingFleetOrderKindV2::Explore,
                    7,
                    &[false],
                )])
                .unwrap();
                state.routes =
                    LivingSortedVecV2::new(vec![route(60, LivingRouteKindV2::Internal, 5, 8, 3)])
                        .unwrap();
            }
            if axis == 2 {
                for index in 0..state.worlds.as_slice().len() {
                    state.worlds.record_at_mut(index).unwrap().system = id(100);
                }
            }
            if axis == 3 {
                state.systems = LivingSortedVecV2::new(
                    (100..=112)
                        .map(|n| LivingSystemV2 {
                            id: id(n),
                            name: crate::living::LivingNameV2::new(format!("Far {n}")).unwrap(),
                        })
                        .collect(),
                )
                .unwrap();
                state.worlds.find_mut(&id(7)).unwrap().system = id(112);
                state.worlds.find_mut(&id(8)).unwrap().system = id(112);
                state.lanes = LivingSortedVecV2::new(
                    (100..112)
                        .map(|n| LivingLaneV2 {
                            id: id(n + 20),
                            system_a: id(n),
                            system_b: id(n + 1),
                            distance_units: u64::MAX,
                        })
                        .collect(),
                )
                .unwrap();
            }
            let (actual, events) =
                parity(&authority, &state, Default::default(), LivingTickV2(100));
            if axis == 3 {
                assert!(events.payloads().is_empty());
                assert!(matches!(
                    actual.fleets.as_slice()[0].location,
                    LivingFleetLocationV2::Docked { .. }
                ));
            }
            if axis == 2 {
                assert!(
                    actual.fleets.as_slice()[0]
                        .order
                        .as_ref()
                        .unwrap()
                        .lane_path
                        .is_empty()
                );
            }
        }
    }
    #[test]
    fn dispatch_cursor_initial_reference_parity() {
        let authority = crate::living::builtin_living_sandbox_v2().unwrap();
        let mut reference = authority.state().clone();
        let mut events = LivingPendingEventsV2::default();
        super::super::rules::reference_dispatch_for_cursor_test(
            &mut super::super::simulation::LivingStepContextV2 {
                catalog: authority.catalog(),
                genesis_seed: [0; 32],
                branch: authority.branch(),
                tick: LivingTickV2(100),
                state: &mut reference,
                events: &mut events,
                arrivals: Vec::new(),
            },
        )
        .unwrap();
        for budget in [1, 64, 1024] {
            let mut job = LivingDispatchJobV2::begin(
                authority.catalog(),
                authority.state().clone(),
                LivingPendingEventsV2::default(),
                LivingTickV2(100),
                [0; 32],
                authority.branch(),
            );
            while job.poll(budget).unwrap() != LivingLifecycleProgressV2::Ready {
                assert!(job.last_poll_visits() <= budget);
            }
            assert!(job.last_poll_visits() <= budget);
            let (state, actual) = job.finish().unwrap();
            assert_eq!(state, reference);
            assert_eq!(actual.payloads(), events.payloads());
            assert_eq!(actual.corridor_restrictions, events.corridor_restrictions);
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum StateValidationCursor {
    HullJobTally,
    CollectionBounds,
    FleetBounds,
    TotalHullBound,
    Identities,
    LookupSystems,
    LookupWorlds,
    LookupCivilizations,
    LookupLanes,
    LookupFleets,
    CatalogDeclarations,
    Stars,
    Worlds,
    Lanes,
    Deposits,
    Civilizations,
    Colonies,
    CountFacilities,
    CountConstruction,
    CountHullJobs,
    WorldIndustry,
    Facilities,
    ConstructionJobs,
    HullJobs,
    Fleets,
    FleetLanes,
    FleetHulls,
    Routes,
    Shipments,
    Relations,
    Agreements,
    Wars,
    Observations,
    ObservationFacilities,
    ObservationBattery,
    ObservationHulls,
    Hazards,
    Claims,
    ClaimPairs,
    Occupations,
    Retire,
    Ready,
}
#[derive(Default)]
struct ValidationIndustryCounts {
    facilities: usize,
    buildings: usize,
    hulls: usize,
    shipyards: usize,
    batteries: usize,
}
/// Counted validation of an immutable publication against the unchanged model validator.
///
/// Construction retains only the two owners and a scalar comparison tick. Polling
/// visits individual records, nested references and temporary-index retirements.
/// Successful validation does not publish state or prove its stored tick equals
/// an effective observation-comparison tick. Final owner retirement is caller work.
pub struct LivingStateValidationJobV2 {
    catalog: Option<Arc<super::catalog::ValidatedLivingCatalogPackV2>>,
    source: Option<Arc<LivingGalaxyStateV2>>,
    effective_tick: LivingTickV2,
    cursor: StateValidationCursor,
    index: usize,
    nested: usize,
    collection: usize,
    untargeted_jobs: usize,
    hull_count: usize,
    targeted_jobs: BTreeMap<LivingEntityIdV2, usize>,
    identities: BTreeSet<LivingEntityIdV2>,
    systems: BTreeSet<LivingEntityIdV2>,
    worlds: BTreeMap<LivingEntityIdV2, usize>,
    civilizations: BTreeSet<LivingEntityIdV2>,
    lanes: BTreeSet<LivingEntityIdV2>,
    fleets: BTreeMap<LivingEntityIdV2, usize>,
    // Only declaration slugs and hull HP maxima; recipes never enter this index.
    declarations: BTreeMap<(u8, super::catalog::LivingSlugV2), u32>,
    industry: BTreeMap<LivingEntityIdV2, ValidationIndustryCounts>,
    observed_batteries: u64,
    cancelled: bool,
    delivered: bool,
    retirement_requested: bool,
    retirement_ready: bool,
    retirement_index: usize,
    fault: Option<super::model::LivingValidationErrorV2>,
    last_poll_visits: usize,
    total_visits: u64,
}
impl LivingStateValidationJobV2 {
    /// Validate using the publication's stored tick for observation comparisons.
    pub fn begin(
        catalog: Arc<super::catalog::ValidatedLivingCatalogPackV2>,
        source: Arc<LivingGalaxyStateV2>,
    ) -> Self {
        let tick = source.tick;
        Self::begin_at_tick(catalog, source, tick)
    }
    /// Override only the observation comparison tick, without changing the source.
    /// Zero, older and non-next ticks have no additional validity restriction.
    pub fn begin_at_tick(
        catalog: Arc<super::catalog::ValidatedLivingCatalogPackV2>,
        source: Arc<LivingGalaxyStateV2>,
        effective_tick: LivingTickV2,
    ) -> Self {
        Self {
            catalog: Some(catalog),
            source: Some(source),
            effective_tick,
            cursor: StateValidationCursor::HullJobTally,
            index: 0,
            nested: 0,
            collection: 0,
            untargeted_jobs: 0,
            hull_count: 0,
            targeted_jobs: Default::default(),
            identities: Default::default(),
            systems: Default::default(),
            worlds: Default::default(),
            civilizations: Default::default(),
            lanes: Default::default(),
            fleets: Default::default(),
            declarations: Default::default(),
            industry: Default::default(),
            observed_batteries: 0,
            cancelled: false,
            delivered: false,
            retirement_requested: false,
            retirement_ready: false,
            retirement_index: 0,
            fault: None,
            last_poll_visits: 0,
            total_visits: 0,
        }
    }
    /// Cancel without releasing any source, catalog or temporary-index owner.
    pub fn cancel(&mut self) {
        self.cancelled = true;
    }
    /// Actual source, nested-reference and index records visited by the last poll.
    pub const fn last_poll_visits(&self) -> usize {
        self.last_poll_visits
    }
    /// Actual records visited since construction, including a faulting record.
    pub const fn total_visits(&self) -> u64 {
        self.total_visits
    }
    /// Validate at most the requested number of actual records.
    pub fn poll(
        &mut self,
        budget: usize,
    ) -> Result<LivingLifecycleProgressV2, LivingLifecycleJobErrorV2> {
        self.last_poll_visits = 0;
        if self.cancelled {
            return Err(LivingLifecycleJobErrorV2::Cancelled);
        }
        if budget == 0 || budget > LIVING_MAX_PHASE_POLL_VISITS_V2 {
            return Err(LivingLifecycleJobErrorV2::Budget);
        }
        if let Some(fault) = self.fault {
            return Err(LivingLifecycleJobErrorV2::Fault(fault.into()));
        }
        if self.retirement_requested {
            return Err(LivingLifecycleJobErrorV2::Cancelled);
        }
        while self.last_poll_visits < budget && self.cursor != StateValidationCursor::Ready {
            match self.visit_validation() {
                Ok(true) => {
                    self.last_poll_visits += 1;
                    self.total_visits += 1;
                }
                Ok(false) => {}
                Err(error) => {
                    self.last_poll_visits += 1;
                    self.total_visits += 1;
                    self.fault = Some(error);
                    return Err(LivingLifecycleJobErrorV2::Fault(error.into()));
                }
            }
        }
        Ok(if self.cursor == StateValidationCursor::Ready {
            LivingLifecycleProgressV2::Ready
        } else {
            LivingLifecycleProgressV2::Pending
        })
    }
    /// Move the original owners once, after validation and counted index retirement.
    /// Every rejected extraction preserves owners and partial validation indexes.
    pub fn finish(
        &mut self,
    ) -> Result<
        (
            Arc<LivingGalaxyStateV2>,
            Arc<super::catalog::ValidatedLivingCatalogPackV2>,
            LivingTickV2,
        ),
        LivingLifecycleJobErrorV2,
    > {
        if self.cancelled {
            return Err(LivingLifecycleJobErrorV2::Cancelled);
        }
        if let Some(fault) = self.fault {
            return Err(LivingLifecycleJobErrorV2::Fault(fault.into()));
        }
        if self.retirement_requested {
            return Err(LivingLifecycleJobErrorV2::Cancelled);
        }
        if self.cursor != StateValidationCursor::Ready || self.delivered {
            return Err(LivingLifecycleJobErrorV2::NotReady);
        }
        // Both owners are present until the single successful extraction.
        let source = self.source.take().expect("undelivered validation source");
        let catalog = self.catalog.take().expect("undelivered validation catalog");
        self.delivered = true;
        Ok((source, catalog, self.effective_tick))
    }
    /// Explicitly abandon validation and drain temporary indexes under the budget.
    /// This also works after a fault or cancellation, without changing the error
    /// returned by normal polling/extraction. It never scans or destroys the source.
    pub fn poll_retirement(
        &mut self,
        budget: usize,
    ) -> Result<LivingLifecycleProgressV2, LivingLifecycleJobErrorV2> {
        self.last_poll_visits = 0;
        if budget == 0 || budget > LIVING_MAX_PHASE_POLL_VISITS_V2 {
            return Err(LivingLifecycleJobErrorV2::Budget);
        }
        if self.delivered {
            return Err(LivingLifecycleJobErrorV2::NotReady);
        }
        self.retirement_requested = true;
        while self.last_poll_visits < budget && !self.retirement_ready {
            match self.retire_validation_index(self.retirement_index) {
                Some(true) => {
                    self.last_poll_visits += 1;
                    self.total_visits += 1;
                }
                Some(false) => self.retirement_index += 1,
                None => self.retirement_ready = true,
            }
        }
        Ok(if self.retirement_ready {
            LivingLifecycleProgressV2::Ready
        } else {
            LivingLifecycleProgressV2::Pending
        })
    }
    /// Move original owners once after explicit index retirement. This is an owner
    /// handoff for further reclamation; it makes no successful-validation claim.
    pub fn finish_retirement(
        &mut self,
    ) -> Result<
        (
            Arc<LivingGalaxyStateV2>,
            Arc<super::catalog::ValidatedLivingCatalogPackV2>,
            LivingTickV2,
        ),
        LivingLifecycleJobErrorV2,
    > {
        if !self.retirement_requested || !self.retirement_ready || self.delivered {
            return Err(LivingLifecycleJobErrorV2::NotReady);
        }
        let source = self.source.take().expect("undelivered validation source");
        let catalog = self.catalog.take().expect("undelivered validation catalog");
        self.delivered = true;
        Ok((source, catalog, self.effective_tick))
    }
    fn retire_validation_index(&mut self, index: usize) -> Option<bool> {
        Some(match index {
            0 => self.targeted_jobs.pop_first().is_some(),
            1 => self.identities.pop_first().is_some(),
            2 => self.systems.pop_first().is_some(),
            3 => self.worlds.pop_first().is_some(),
            4 => self.civilizations.pop_first().is_some(),
            5 => self.lanes.pop_first().is_some(),
            6 => self.fleets.pop_first().is_some(),
            7 => self.declarations.pop_first().is_some(),
            8 => self.industry.pop_first().is_some(),
            _ => return None,
        })
    }
    fn advance_validation(&mut self, cursor: StateValidationCursor) {
        self.cursor = cursor;
        self.index = 0;
        self.nested = 0;
    }
    fn declares(&self, category: u8, slug: &super::catalog::LivingSlugV2) -> bool {
        self.declarations.contains_key(&(category, slug.clone()))
    }
    fn visit_validation(&mut self) -> Result<bool, super::model::LivingValidationErrorV2> {
        use super::model::*;
        use StateValidationCursor::*;
        let state = self.source.as_ref().expect("retained validation source");
        let catalog = self.catalog.as_ref().expect("retained validation catalog");
        macro_rules! row {
            ($field:ident, $next:ident, $name:ident, $body:block) => {
                if let Some($name) = state.$field.as_slice().get(self.index) { $body self.index += 1; return Ok(true); }
                self.advance_validation($next);
            };
        }
        match self.cursor {
            HullJobTally => {
                if let Some(job) = state.hull_jobs.as_slice().get(self.index) {
                    if let Some(target) = job.target_fleet {
                        *self.targeted_jobs.entry(target).or_default() += 1;
                    } else {
                        self.untargeted_jobs += 1;
                    }
                    self.hull_count += 1;
                    self.index += 1;
                    return Ok(true);
                }
                self.advance_validation(CollectionBounds);
            }
            CollectionBounds => {
                // Scalar lengths do not scan the collections. Preserve the table's order.
                let bound = match self.index {
                    0 => Some((
                        "civilizations",
                        LIVING_MAX_CIVILIZATIONS_V2,
                        state.civilizations.as_slice().len(),
                    )),
                    1 => Some((
                        "systems",
                        LIVING_MAX_SYSTEMS_V2,
                        state.systems.as_slice().len(),
                    )),
                    2 => Some(("stars", LIVING_MAX_STARS_V2, state.stars.as_slice().len())),
                    3 => Some((
                        "worlds",
                        LIVING_MAX_WORLDS_V2,
                        state.worlds.as_slice().len(),
                    )),
                    4 => Some(("lanes", LIVING_MAX_LANES_V2, state.lanes.as_slice().len())),
                    5 => Some((
                        "deposits",
                        LIVING_MAX_DEPOSITS_V2,
                        state.deposits.as_slice().len(),
                    )),
                    6 => Some((
                        "facilities",
                        LIVING_MAX_INDUSTRIES_V2,
                        state.facilities.as_slice().len()
                            + state.construction_jobs.as_slice().len(),
                    )),
                    7 => Some((
                        "routes",
                        LIVING_MAX_ROUTES_V2,
                        state.routes.as_slice().len(),
                    )),
                    8 => Some((
                        "shipments",
                        LIVING_MAX_SHIPMENTS_V2,
                        state.shipments.as_slice().len(),
                    )),
                    9 => Some((
                        "fleets",
                        LIVING_MAX_FLEETS_V2,
                        state.fleets.as_slice().len() + self.untargeted_jobs,
                    )),
                    10 => Some((
                        "hazards",
                        LIVING_MAX_HAZARDS_V2,
                        state.hazards.as_slice().len(),
                    )),
                    11 => Some((
                        "colonies",
                        state.worlds.as_slice().len(),
                        state.colonies.as_slice().len(),
                    )),
                    _ => None,
                };
                if let Some((collection, limit, found)) = bound {
                    validation_capacity(collection, limit, found)?;
                    self.index += 1;
                    return Ok(true);
                }
                self.advance_validation(FleetBounds);
            }
            FleetBounds => {
                if let Some(fleet) = state.fleets.as_slice().get(self.index) {
                    validation_capacity(
                        "fleet hulls",
                        LIVING_MAX_HULLS_PER_FLEET_V2,
                        fleet.hulls.as_slice().len()
                            + self.targeted_jobs.get(&fleet.id).copied().unwrap_or(0),
                    )?;
                    self.hull_count += fleet.hulls.as_slice().len();
                    self.index += 1;
                    return Ok(true);
                }
                self.advance_validation(TotalHullBound);
            }
            TotalHullBound => {
                validation_capacity("hulls", LIVING_MAX_HULLS_V2, self.hull_count)?;
                self.collection = 0;
                self.advance_validation(Identities);
                return Ok(true);
            }
            Identities => {
                let identity = match self.collection {
                    0 => state.systems.as_slice().get(self.index).map(|row| row.id),
                    1 => state.stars.as_slice().get(self.index).map(|row| row.id),
                    2 => state.worlds.as_slice().get(self.index).map(|row| row.id),
                    3 => state.lanes.as_slice().get(self.index).map(|row| row.id),
                    4 => state
                        .civilizations
                        .as_slice()
                        .get(self.index)
                        .map(|row| row.id),
                    5 => state.deposits.as_slice().get(self.index).map(|row| row.id),
                    6 => state
                        .facilities
                        .as_slice()
                        .get(self.index)
                        .map(|row| row.id),
                    7 => state
                        .construction_jobs
                        .as_slice()
                        .get(self.index)
                        .map(|row| row.id),
                    8 => state.hull_jobs.as_slice().get(self.index).map(|row| row.id),
                    9 => state.fleets.as_slice().get(self.index).map(|row| row.id),
                    10 => {
                        if let Some(fleet) = state.fleets.as_slice().get(self.index) {
                            if let Some(hull) = fleet.hulls.as_slice().get(self.nested) {
                                self.nested += 1;
                                validation_identity(&mut self.identities, hull.id)?;
                                return Ok(true);
                            }
                            self.index += 1;
                            self.nested = 0;
                            // Completing this fleet header is a record visit too;
                            // otherwise a run of empty fleets could scan uncharged.
                            return Ok(true);
                        }
                        None
                    }
                    11 => state.routes.as_slice().get(self.index).map(|row| row.id),
                    12 => state.shipments.as_slice().get(self.index).map(|row| row.id),
                    13 => state.hazards.as_slice().get(self.index).map(|row| row.id),
                    _ => {
                        self.advance_validation(LookupSystems);
                        return Ok(false);
                    }
                };
                if let Some(id) = identity {
                    validation_identity(&mut self.identities, id)?;
                    self.index += 1;
                    return Ok(true);
                }
                self.collection += 1;
                self.index = 0;
            }
            LookupSystems => {
                row!(systems, LookupWorlds, row, {
                    self.systems.insert(row.id);
                });
            }
            LookupWorlds => {
                row!(worlds, LookupCivilizations, row, {
                    self.worlds.insert(row.id, self.index);
                });
            }
            LookupCivilizations => {
                row!(civilizations, LookupLanes, row, {
                    self.civilizations.insert(row.id);
                });
            }
            LookupLanes => {
                row!(lanes, LookupFleets, row, {
                    self.lanes.insert(row.id);
                });
            }
            LookupFleets => {
                if let Some(fleet) = state.fleets.as_slice().get(self.index) {
                    self.fleets.insert(fleet.id, self.index);
                    self.index += 1;
                    return Ok(true);
                }
                self.collection = 0;
                self.advance_validation(CatalogDeclarations);
            }
            CatalogDeclarations => {
                let definition = match self.collection {
                    0 => catalog
                        .star_archetypes()
                        .get(self.index)
                        .map(|row| (&row.id, 0)),
                    1 => catalog
                        .world_archetypes()
                        .get(self.index)
                        .map(|row| (&row.id, 0)),
                    2 => catalog.resources().get(self.index).map(|row| (&row.id, 0)),
                    3 => catalog.industries().get(self.index).map(|row| (&row.id, 0)),
                    4 => catalog
                        .hulls()
                        .get(self.index)
                        .map(|row| (&row.id, row.hit_points)),
                    5 => catalog.hazards().get(self.index).map(|row| (&row.id, 0)),
                    6 => catalog.policies().get(self.index).map(|row| (&row.id, 0)),
                    _ => {
                        self.advance_validation(Stars);
                        return Ok(false);
                    }
                };
                if let Some((slug, value)) = definition {
                    self.declarations
                        .insert((self.collection as u8, slug.clone()), value);
                    self.index += 1;
                    return Ok(true);
                }
                self.collection += 1;
                self.index = 0;
            }
            Stars => {
                row!(stars, Worlds, star, {
                    validation_require(self.systems.contains(&star.system), "star system")?;
                    validation_require(self.declares(0, &star.archetype), "star archetype")?;
                });
            }
            Worlds => {
                row!(worlds, Lanes, world, {
                    validation_require(self.systems.contains(&world.system), "world system")?;
                    validation_require(self.declares(1, &world.archetype), "world archetype")?;
                    if let Some(owner) = world.owner {
                        validation_require(self.civilizations.contains(&owner), "world owner")?;
                    }
                    if world.inventory.energy > super::catalog::LIVING_RESOURCE_STORAGE_LIMIT_V2
                        || world.inventory.ore > super::catalog::LIVING_RESOURCE_STORAGE_LIMIT_V2
                        || world.inventory.alloy > super::catalog::LIVING_RESOURCE_STORAGE_LIMIT_V2
                    {
                        return Err(validation_range("world inventory"));
                    }
                });
            }
            Lanes => {
                row!(lanes, Deposits, lane, {
                    validation_require(self.systems.contains(&lane.system_a), "lane endpoint")?;
                    validation_require(self.systems.contains(&lane.system_b), "lane endpoint")?;
                    if lane.system_a >= lane.system_b {
                        return Err(validation_range("lane endpoints"));
                    }
                    if lane.distance_units == 0 {
                        return Err(validation_range("lane distance_units"));
                    }
                });
            }
            Deposits => {
                row!(deposits, Civilizations, deposit, {
                    validation_require(self.worlds.contains_key(&deposit.world), "deposit world")?;
                    validation_require(
                        self.declares(2, &validation_resource_slug(deposit.resource)),
                        "deposit resource",
                    )?;
                });
            }
            Civilizations => {
                row!(civilizations, Colonies, civilization, {
                    validation_require(
                        self.declares(6, &civilization.policy.0),
                        "civilization policy",
                    )?;
                });
            }
            Colonies => {
                row!(colonies, CountFacilities, colony, {
                    let index = self.worlds.get(&colony.world).ok_or(
                        LivingValidationErrorV2::DanglingReference {
                            reference: "colony world",
                        },
                    )?;
                    validation_require(self.civilizations.contains(&colony.owner), "colony owner")?;
                    if state.worlds.as_slice()[*index].owner != Some(colony.owner) {
                        return Err(validation_range("colony owner"));
                    }
                });
            }
            CountFacilities => {
                row!(facilities, CountConstruction, facility, {
                    let count = self.industry.entry(facility.world).or_default();
                    count.facilities += 1;
                    count.shipyards += usize::from(facility.definition.as_str() == "shipyard");
                    count.batteries +=
                        usize::from(facility.definition.as_str() == "defense_battery");
                });
            }
            CountConstruction => {
                row!(construction_jobs, CountHullJobs, job, {
                    let count = self.industry.entry(job.world).or_default();
                    count.buildings += 1;
                    count.shipyards += usize::from(job.definition.as_str() == "shipyard");
                    count.batteries += usize::from(job.definition.as_str() == "defense_battery");
                });
            }
            CountHullJobs => {
                row!(hull_jobs, WorldIndustry, job, {
                    self.industry.entry(job.world).or_default().hulls += 1;
                });
            }
            WorldIndustry => {
                row!(worlds, Facilities, world, {
                    if let Some(count) = self.industry.get(&world.id) {
                        validation_capacity(
                            "world industrial slots",
                            6,
                            count.facilities + count.buildings,
                        )?;
                        if count.shipyards > 1 {
                            return Err(validation_range("unique world industry"));
                        }
                        if count.batteries > 1 {
                            return Err(validation_range("unique world industry"));
                        }
                        if count.buildings > 1 || count.hulls > 1 {
                            return Err(validation_range("world construction queue"));
                        }
                    }
                });
            }
            Facilities => {
                row!(facilities, ConstructionJobs, facility, {
                    validation_require(
                        self.worlds.contains_key(&facility.world),
                        "facility world",
                    )?;
                    validation_require(
                        self.declares(3, &facility.definition),
                        "facility definition",
                    )?;
                });
            }
            ConstructionJobs => {
                row!(construction_jobs, HullJobs, job, {
                    validation_require(
                        self.worlds.contains_key(&job.world),
                        "construction job world",
                    )?;
                    validation_require(
                        self.civilizations.contains(&job.owner),
                        "construction job owner",
                    )?;
                    validation_require(
                        self.declares(3, &job.definition),
                        "construction job definition",
                    )?;
                    validation_interval(
                        job.accepted_tick,
                        job.completion_tick,
                        "construction job duration",
                    )?;
                });
            }
            HullJobs => {
                row!(hull_jobs, Fleets, job, {
                    validation_require(self.worlds.contains_key(&job.world), "hull job world")?;
                    validation_require(self.civilizations.contains(&job.owner), "hull job owner")?;
                    validation_require(self.declares(4, &job.definition), "hull job definition")?;
                    if let Some(target) = job.target_fleet {
                        let fleet = self
                            .fleets
                            .get(&target)
                            .map(|index| &state.fleets.as_slice()[*index])
                            .ok_or(LivingValidationErrorV2::DanglingReference {
                                reference: "hull job target fleet",
                            })?;
                        if fleet.owner != job.owner
                            || fleet.location
                                != (LivingFleetLocationV2::Docked { world: job.world })
                            || fleet.order.is_some()
                        {
                            return Err(validation_range("hull job target reservation"));
                        }
                    }
                    validation_interval(
                        job.accepted_tick,
                        job.completion_tick,
                        "hull job duration",
                    )?;
                });
            }
            Fleets => {
                if let Some(fleet) = state.fleets.as_slice().get(self.index) {
                    validation_require(self.civilizations.contains(&fleet.owner), "fleet owner")?;
                    match fleet.location {
                        LivingFleetLocationV2::Docked { world } => {
                            validation_require(self.worlds.contains_key(&world), "fleet world")?
                        }
                        LivingFleetLocationV2::Travelling {
                            origin_world,
                            destination_world,
                            departure_tick,
                            arrival_tick,
                        } => {
                            validation_require(
                                self.worlds.contains_key(&origin_world),
                                "fleet world",
                            )?;
                            validation_require(
                                self.worlds.contains_key(&destination_world),
                                "fleet world",
                            )?;
                            validation_interval(departure_tick, arrival_tick, "fleet leg")?;
                        }
                    }
                    if let Some(order) = &fleet.order
                        && let Some(target) = order.target_world
                    {
                        validation_require(
                            self.worlds.contains_key(&target),
                            "fleet order target",
                        )?;
                    }
                    self.nested = 0;
                    self.cursor = FleetLanes;
                    return Ok(true);
                }
                self.advance_validation(Routes);
            }
            FleetLanes => {
                let fleet = &state.fleets.as_slice()[self.index];
                if let Some(order) = &fleet.order
                    && let Some(lane) = order.lane_path.get(self.nested)
                {
                    validation_require(self.lanes.contains(lane), "fleet order lane")?;
                    self.nested += 1;
                    return Ok(true);
                }
                self.nested = 0;
                self.cursor = FleetHulls;
            }
            FleetHulls => {
                let fleet = &state.fleets.as_slice()[self.index];
                if let Some(hull) = fleet.hulls.as_slice().get(self.nested) {
                    let maximum = self.declarations.get(&(4, hull.definition.clone())).ok_or(
                        LivingValidationErrorV2::DanglingReference {
                            reference: "hull definition",
                        },
                    )?;
                    if hull.hit_points == 0 || hull.hit_points > *maximum {
                        return Err(validation_range("hull hit_points"));
                    }
                    self.nested += 1;
                    return Ok(true);
                }
                self.index += 1;
                self.nested = 0;
                self.cursor = Fleets;
            }
            Routes => {
                row!(routes, Shipments, route, {
                    validation_require(
                        self.worlds.contains_key(&route.source_world),
                        "route world",
                    )?;
                    validation_require(
                        self.worlds.contains_key(&route.destination_world),
                        "route world",
                    )?;
                    validation_require(
                        self.civilizations.contains(&route.source_owner),
                        "route owner",
                    )?;
                    validation_require(
                        self.civilizations.contains(&route.receiver_owner),
                        "route owner",
                    )?;
                    validation_require(
                        self.declares(2, &validation_resource_slug(route.resource)),
                        "route resource",
                    )?;
                    if route.source_world == route.destination_world {
                        return Err(validation_range("route endpoints"));
                    }
                    if route.batch_size == 0 || route.cadence_ticks != LIVING_ROUTE_CADENCE_TICKS_V2
                    {
                        return Err(validation_range("route cadence"));
                    }
                    if route.batch_size > LIVING_ROUTE_BATCH_LIMIT_V2 {
                        return Err(validation_range("route batch_size"));
                    }
                    if route.source_reserve < LIVING_ROUTE_RESERVE_FLOOR_V2
                        || route.source_reserve > super::catalog::LIVING_RESOURCE_STORAGE_LIMIT_V2
                    {
                        return Err(validation_range("route source_reserve"));
                    }
                });
            }
            Shipments => {
                row!(shipments, Relations, shipment, {
                    validation_require(
                        self.worlds.contains_key(&shipment.source_world),
                        "shipment world",
                    )?;
                    validation_require(
                        self.worlds.contains_key(&shipment.destination_world),
                        "shipment world",
                    )?;
                    validation_require(
                        self.civilizations.contains(&shipment.dispatch_owner),
                        "shipment owner",
                    )?;
                    validation_require(
                        self.civilizations.contains(&shipment.intended_receiver),
                        "shipment owner",
                    )?;
                    validation_require(
                        self.declares(2, &validation_resource_slug(shipment.resource)),
                        "shipment resource",
                    )?;
                    if shipment.units == 0
                        || shipment.units > super::catalog::LIVING_RESOURCE_STORAGE_LIMIT_V2
                    {
                        return Err(validation_range("shipment units"));
                    }
                    if shipment.returning_to_source
                        && shipment.disposition != LivingShipmentDispositionV2::WaitingForStorage
                    {
                        return Err(validation_range("shipment return wait"));
                    }
                    if shipment.dispatch_kind == Some(LivingRouteKindV2::Internal)
                        && shipment.dispatch_owner != shipment.intended_receiver
                    {
                        return Err(validation_range("shipment internal participants"));
                    }
                    validation_interval(
                        shipment.departure_tick,
                        shipment.arrival_tick,
                        "shipment travel",
                    )?;
                });
            }
            Relations => {
                row!(relations, Agreements, relation, {
                    validation_require(
                        self.civilizations.contains(&relation.from),
                        "relation party",
                    )?;
                    validation_require(
                        self.civilizations.contains(&relation.to),
                        "relation party",
                    )?;
                    if relation.from == relation.to {
                        return Err(validation_range("relation parties"));
                    }
                    validation_bounded(
                        relation.base_disposition,
                        -LIVING_RELATION_BOUND_V2,
                        LIVING_RELATION_BOUND_V2,
                        "relation base_disposition",
                    )?;
                    let reasons = &relation.reasons;
                    validation_bounded(reasons.delivered_aid, 0, 20, "relation delivered_aid")?;
                    validation_bounded(
                        reasons.successful_trade,
                        0,
                        20,
                        "relation successful_trade",
                    )?;
                    validation_bounded(
                        reasons.adjacent_rival_settlement,
                        -20,
                        0,
                        "relation adjacent_rival_settlement",
                    )?;
                    validation_bounded(
                        reasons.war_declared_against_it,
                        -30,
                        0,
                        "relation war_declared_against_it",
                    )?;
                    validation_bounded(
                        reasons.colony_captured_from_it,
                        -60,
                        0,
                        "relation colony_captured_from_it",
                    )?;
                    if reasons.aid_units_remainder >= LIVING_DELIVERY_ACCUMULATOR_BOUND_V2
                        || reasons.trade_units_remainder >= LIVING_DELIVERY_ACCUMULATOR_BOUND_V2
                    {
                        return Err(validation_range("relation delivery accumulator"));
                    }
                });
            }
            Agreements => {
                row!(agreements, Wars, agreement, {
                    validation_require(
                        self.civilizations.contains(&agreement.participant_a),
                        "agreement party",
                    )?;
                    validation_require(
                        self.civilizations.contains(&agreement.participant_b),
                        "agreement party",
                    )?;
                    if agreement.participant_a >= agreement.participant_b {
                        return Err(validation_range("agreement participants"));
                    }
                    validation_interval(agreement.start_tick, agreement.end_tick, "agreement")?;
                });
            }
            Wars => {
                row!(wars, Observations, war, {
                    validation_require(
                        self.civilizations.contains(&war.participant_a),
                        "war party",
                    )?;
                    validation_require(
                        self.civilizations.contains(&war.participant_b),
                        "war party",
                    )?;
                    if war.participant_a >= war.participant_b {
                        return Err(validation_range("war participants"));
                    }
                    if war.declarer != war.participant_a && war.declarer != war.participant_b {
                        return Err(validation_range("war declarer"));
                    }
                    validation_interval(war.start_tick, war.end_tick, "war")?;
                });
            }
            Observations => {
                if let Some(observation) = state.observations.as_slice().get(self.index) {
                    validation_require(
                        self.civilizations.contains(&observation.civilization),
                        "observer",
                    )?;
                    validation_require(
                        self.worlds.contains_key(&observation.world),
                        "observed world",
                    )?;
                    if let Some(owner) = observation.owner {
                        validation_require(self.civilizations.contains(&owner), "observed owner")?;
                    }
                    if observation.observed_tick > self.effective_tick {
                        return Err(validation_range("observation observed_tick"));
                    }
                    self.observed_batteries = 0;
                    self.nested = 0;
                    self.cursor = ObservationFacilities;
                    return Ok(true);
                }
                self.advance_validation(Hazards);
            }
            ObservationFacilities => {
                let observation = &state.observations.as_slice()[self.index];
                if let Some(facility) = observation.facilities.as_slice().get(self.nested) {
                    validation_require(
                        self.declares(3, &facility.definition),
                        "observed facility definition",
                    )?;
                    if observation.operational_defense_batteries.is_some()
                        && facility.definition.as_str() == "defense_battery"
                    {
                        self.observed_batteries += u64::from(facility.count);
                    }
                    self.nested += 1;
                    return Ok(true);
                }
                self.cursor = ObservationBattery;
            }
            ObservationBattery => {
                let observation = &state.observations.as_slice()[self.index];
                if let Some(operational) = observation.operational_defense_batteries {
                    if u64::from(operational) >= self.observed_batteries {
                        return Err(validation_range(
                            "observation operational_defense_batteries",
                        ));
                    }
                    self.nested = 0;
                    self.cursor = ObservationHulls;
                    return Ok(true);
                }
                self.nested = 0;
                self.cursor = ObservationHulls;
            }
            ObservationHulls => {
                let observation = &state.observations.as_slice()[self.index];
                if let Some(hull) = observation.stationed_hulls.as_slice().get(self.nested) {
                    validation_require(
                        self.declares(4, &hull.definition),
                        "observed hull definition",
                    )?;
                    self.nested += 1;
                    return Ok(true);
                }
                self.index += 1;
                self.nested = 0;
                self.cursor = Observations;
            }
            Hazards => {
                row!(hazards, Claims, hazard, {
                    validation_require(self.lanes.contains(&hazard.lane), "hazard lane")?;
                    validation_require(self.declares(5, &hazard.definition), "hazard definition")?;
                    validation_interval(hazard.start_tick, hazard.end_tick, "hazard")?;
                });
            }
            Claims => {
                row!(settlement_claims, ClaimPairs, claim, {
                    validation_require(self.worlds.contains_key(&claim.world), "claim world")?;
                    validation_require(
                        self.civilizations.contains(&claim.civilization),
                        "claimant",
                    )?;
                    validation_interval(claim.arrival_tick, claim.close_tick, "claim window")?;
                });
            }
            ClaimPairs => {
                if let (Some(left), Some(right)) = (
                    state.settlement_claims.as_slice().get(self.index),
                    state.settlement_claims.as_slice().get(self.index + 1),
                ) {
                    if left.world == right.world && left.close_tick != right.close_tick {
                        return Err(validation_range("claim close_tick"));
                    }
                    self.index += 1;
                    return Ok(true);
                }
                self.advance_validation(Occupations);
            }
            Occupations => {
                row!(occupations, Retire, occupation, {
                    validation_require(
                        self.worlds.contains_key(&occupation.world),
                        "occupation world",
                    )?;
                    validation_require(
                        self.civilizations.contains(&occupation.claimant),
                        "occupation claimant",
                    )?;
                    if occupation.progress_ticks > LIVING_OCCUPATION_TRANSFER_TICKS_V2 {
                        return Err(validation_range("occupation progress_ticks"));
                    }
                });
            }
            Retire => match self.retire_validation_index(self.index) {
                Some(true) => return Ok(true),
                Some(false) => self.index += 1,
                None => self.cursor = Ready,
            },
            Ready => {}
        }
        Ok(false)
    }
}
fn validation_identity(
    identities: &mut BTreeSet<LivingEntityIdV2>,
    identity: LivingEntityIdV2,
) -> Result<(), super::model::LivingValidationErrorV2> {
    if identities.insert(identity) {
        Ok(())
    } else {
        Err(super::model::LivingValidationErrorV2::DuplicateIdentity {
            context: "authority object",
        })
    }
}
fn validation_capacity(
    collection: &'static str,
    limit: usize,
    found: usize,
) -> Result<(), super::model::LivingValidationErrorV2> {
    if found > limit {
        Err(super::model::LivingValidationErrorV2::Capacity {
            collection,
            limit,
            found,
        })
    } else {
        Ok(())
    }
}
fn validation_require(
    present: bool,
    reference: &'static str,
) -> Result<(), super::model::LivingValidationErrorV2> {
    if present {
        Ok(())
    } else {
        Err(super::model::LivingValidationErrorV2::DanglingReference { reference })
    }
}
fn validation_range(field: &'static str) -> super::model::LivingValidationErrorV2 {
    super::model::LivingValidationErrorV2::Range { field }
}
fn validation_interval(
    start: LivingTickV2,
    end: LivingTickV2,
    field: &'static str,
) -> Result<(), super::model::LivingValidationErrorV2> {
    if start < end {
        Ok(())
    } else {
        Err(super::model::LivingValidationErrorV2::Interval { field })
    }
}
fn validation_bounded(
    value: i32,
    low: i32,
    high: i32,
    field: &'static str,
) -> Result<(), super::model::LivingValidationErrorV2> {
    if (low..=high).contains(&value) {
        Ok(())
    } else {
        Err(validation_range(field))
    }
}
fn validation_resource_slug(
    resource: super::model::LivingResourceV2,
) -> super::catalog::LivingSlugV2 {
    super::catalog::LivingSlugV2::new(resource.as_wire_str()).expect("fixed resource slug")
}

// Private state-only reclamation. Parent/phase/catalog adapters are later work.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum LivingStateRetirementPolicyV2 {
    ReleaseHandle,
    DeferUntilUnique,
}

/// This owner must be retained or resumed, never ordinarily dropped as reclamation.
#[must_use = "Retain or resume the deferred state owner through counted retirement."]
pub(crate) struct LivingDeferredStateRetirementV2 {
    source: Arc<LivingGalaxyStateV2>,
}
#[must_use = "SharedReleased is only a handle release; Deferred retains the full owner."]
pub(crate) enum LivingStateRetirementDispositionV2 {
    Drained,
    SharedReleased,
    Deferred(LivingDeferredStateRetirementV2),
}
#[derive(Default, Debug, Eq, PartialEq)]
struct StateRetirementCounts {
    buffer_moves: u64,
    rows: u64,
    nested_records: u64,
    storage_releases: u64,
    parent_releases: u64,
    box_releases: u64,
    root_releases: u64,
    shared_attempts: u64,
}

enum StateRetirementRows {
    Systems(IntoIter<super::model::LivingSystemV2>),
    Stars(IntoIter<super::model::LivingStarV2>),
    Worlds(IntoIter<super::model::LivingWorldV2>),
    Lanes(IntoIter<super::model::LivingLaneV2>),
    Civilizations(IntoIter<super::model::LivingCivilizationV2>),
    Deposits(IntoIter<super::model::LivingDepositV2>),
    Colonies(IntoIter<super::model::LivingColonyV2>),
    Facilities(IntoIter<super::model::LivingFacilityV2>),
    ConstructionJobs(IntoIter<super::model::LivingConstructionJobV2>),
    HullJobs(IntoIter<super::model::LivingHullJobV2>),
    Fleets(IntoIter<super::model::LivingFleetV2>),
    Routes(IntoIter<super::model::LivingFreightRouteV2>),
    Shipments(IntoIter<super::model::LivingShipmentV2>),
    Relations(IntoIter<super::model::LivingRelationV2>),
    Agreements(IntoIter<super::model::LivingAgreementV2>),
    Wars(IntoIter<super::model::LivingWarV2>),
    Observations(IntoIter<super::model::LivingObservationV2>),
    Hazards(IntoIter<super::model::LivingHazardV2>),
    Claims(IntoIter<super::model::LivingSettlementClaimV2>),
    Occupations(IntoIter<super::model::LivingOccupationV2>),
}
impl StateRetirementRows {
    fn begin(state: &mut LivingGalaxyStateV2, collection: usize) -> Self {
        match collection {
            0 => Self::Systems(std::mem::take(&mut state.systems).into_vec().into_iter()),
            1 => Self::Stars(std::mem::take(&mut state.stars).into_vec().into_iter()),
            2 => Self::Worlds(std::mem::take(&mut state.worlds).into_vec().into_iter()),
            3 => Self::Lanes(std::mem::take(&mut state.lanes).into_vec().into_iter()),
            4 => Self::Civilizations(
                std::mem::take(&mut state.civilizations)
                    .into_vec()
                    .into_iter(),
            ),
            5 => Self::Deposits(std::mem::take(&mut state.deposits).into_vec().into_iter()),
            6 => Self::Colonies(std::mem::take(&mut state.colonies).into_vec().into_iter()),
            7 => Self::Facilities(std::mem::take(&mut state.facilities).into_vec().into_iter()),
            8 => Self::ConstructionJobs(
                std::mem::take(&mut state.construction_jobs)
                    .into_vec()
                    .into_iter(),
            ),
            9 => Self::HullJobs(std::mem::take(&mut state.hull_jobs).into_vec().into_iter()),
            10 => Self::Fleets(std::mem::take(&mut state.fleets).into_vec().into_iter()),
            11 => Self::Routes(std::mem::take(&mut state.routes).into_vec().into_iter()),
            12 => Self::Shipments(std::mem::take(&mut state.shipments).into_vec().into_iter()),
            13 => Self::Relations(std::mem::take(&mut state.relations).into_vec().into_iter()),
            14 => Self::Agreements(std::mem::take(&mut state.agreements).into_vec().into_iter()),
            15 => Self::Wars(std::mem::take(&mut state.wars).into_vec().into_iter()),
            16 => Self::Observations(
                std::mem::take(&mut state.observations)
                    .into_vec()
                    .into_iter(),
            ),
            17 => Self::Hazards(std::mem::take(&mut state.hazards).into_vec().into_iter()),
            18 => Self::Claims(
                std::mem::take(&mut state.settlement_claims)
                    .into_vec()
                    .into_iter(),
            ),
            19 => Self::Occupations(
                std::mem::take(&mut state.occupations)
                    .into_vec()
                    .into_iter(),
            ),
            _ => unreachable!("twenty state collections"),
        }
    }
    fn remaining(&self) -> usize {
        match self {
            Self::Systems(rows) => rows.len(),
            Self::Stars(rows) => rows.len(),
            Self::Worlds(rows) => rows.len(),
            Self::Lanes(rows) => rows.len(),
            Self::Civilizations(rows) => rows.len(),
            Self::Deposits(rows) => rows.len(),
            Self::Colonies(rows) => rows.len(),
            Self::Facilities(rows) => rows.len(),
            Self::ConstructionJobs(rows) => rows.len(),
            Self::HullJobs(rows) => rows.len(),
            Self::Fleets(rows) => rows.len(),
            Self::Routes(rows) => rows.len(),
            Self::Shipments(rows) => rows.len(),
            Self::Relations(rows) => rows.len(),
            Self::Agreements(rows) => rows.len(),
            Self::Wars(rows) => rows.len(),
            Self::Observations(rows) => rows.len(),
            Self::Hazards(rows) => rows.len(),
            Self::Claims(rows) => rows.len(),
            Self::Occupations(rows) => rows.len(),
        }
    }
    fn retire_row(&mut self, nested: &mut Option<StateRetirementParent>) {
        match self {
            Self::Systems(rows) => {
                let _ = rows.next().expect("remaining row");
            }
            Self::Stars(rows) => {
                let _ = rows.next().expect("remaining row");
            }
            Self::Worlds(rows) => {
                let _ = rows.next().expect("remaining row");
            }
            Self::Lanes(rows) => {
                let _ = rows.next().expect("remaining row");
            }
            Self::Civilizations(rows) => {
                let _ = rows.next().expect("remaining row");
            }
            Self::Deposits(rows) => {
                let _ = rows.next().expect("remaining row");
            }
            Self::Colonies(rows) => {
                *nested = Some(StateRetirementParent::Colony(
                    rows.next().expect("remaining row"),
                ))
            }
            Self::Facilities(rows) => {
                *nested = Some(StateRetirementParent::Facility(
                    rows.next().expect("remaining row"),
                ))
            }
            Self::ConstructionJobs(rows) => {
                let _ = rows.next().expect("remaining row");
            }
            Self::HullJobs(rows) => {
                let _ = rows.next().expect("remaining row");
            }
            Self::Fleets(rows) => {
                *nested = Some(StateRetirementParent::Fleet(
                    rows.next().expect("remaining row"),
                ))
            }
            Self::Routes(rows) => {
                let _ = rows.next().expect("remaining row");
            }
            Self::Shipments(rows) => {
                let _ = rows.next().expect("remaining row");
            }
            Self::Relations(rows) => {
                let _ = rows.next().expect("remaining row");
            }
            Self::Agreements(rows) => {
                let _ = rows.next().expect("remaining row");
            }
            Self::Wars(rows) => {
                let _ = rows.next().expect("remaining row");
            }
            Self::Observations(rows) => {
                *nested = Some(StateRetirementParent::Observation(
                    rows.next().expect("remaining row"),
                ))
            }
            Self::Hazards(rows) => {
                let _ = rows.next().expect("remaining row");
            }
            Self::Claims(rows) => {
                let _ = rows.next().expect("remaining row");
            }
            Self::Occupations(rows) => {
                let _ = rows.next().expect("remaining row");
            }
        }
    }
}
// Each parent stays alive until every child iterator and allocation is drained.
enum StateRetirementParent {
    Colony(super::model::LivingColonyV2),
    BoxedColony(Box<super::model::LivingColonyV2>),
    Facility(super::model::LivingFacilityV2),
    BoxedFacility(Box<super::model::LivingFacilityV2>),
    Fleet(super::model::LivingFleetV2),
    BoxedFleet(Box<super::model::LivingFleetV2>),
    Observation(super::model::LivingObservationV2),
    BoxedObservation(Box<super::model::LivingObservationV2>),
}
enum StateRetirementChildren {
    Reasons(IntoIter<super::model::LivingBlockedReasonV2>),
    Path(IntoIter<LivingEntityIdV2>),
    Hulls(IntoIter<super::model::LivingHullV2>),
    ObservedFacilities(IntoIter<super::model::LivingObservedFacilityV2>),
    ObservedHulls(IntoIter<super::model::LivingObservedHullV2>),
}
impl StateRetirementChildren {
    fn remaining(&self) -> usize {
        match self {
            Self::Reasons(rows) => rows.len(),
            Self::Path(rows) => rows.len(),
            Self::Hulls(rows) => rows.len(),
            Self::ObservedFacilities(rows) => rows.len(),
            Self::ObservedHulls(rows) => rows.len(),
        }
    }
    fn retire_record(&mut self) {
        match self {
            Self::Reasons(rows) => {
                rows.next().expect("remaining reason");
            }
            Self::Path(rows) => {
                rows.next().expect("remaining lane");
            }
            Self::Hulls(rows) => {
                drop(rows.next().expect("remaining hull"));
            }
            Self::ObservedFacilities(rows) => {
                drop(rows.next().expect("remaining observed facility"));
            }
            Self::ObservedHulls(rows) => {
                drop(rows.next().expect("remaining observed hull"));
            }
        }
    }
}
impl StateRetirementParent {
    fn child(&mut self, index: usize) -> Option<StateRetirementChildren> {
        match (self, index) {
            (Self::Colony(row), 0) => Some(StateRetirementChildren::Reasons(
                std::mem::take(&mut row.hub.blocked_reasons)
                    .into_vec()
                    .into_iter(),
            )),
            (Self::Facility(row), 0) => Some(StateRetirementChildren::Reasons(
                std::mem::take(&mut row.blocked_reasons)
                    .into_vec()
                    .into_iter(),
            )),
            (Self::Fleet(row), 0) => row.order.as_mut().map(|order| {
                StateRetirementChildren::Path(std::mem::take(&mut order.lane_path).into_iter())
            }),
            (Self::Fleet(row), 1) => Some(StateRetirementChildren::Hulls(
                std::mem::take(&mut row.hulls).into_vec().into_iter(),
            )),
            (Self::Observation(row), 0) => Some(StateRetirementChildren::ObservedFacilities(
                std::mem::take(&mut row.facilities).into_vec().into_iter(),
            )),
            (Self::Observation(row), 1) => Some(StateRetirementChildren::ObservedHulls(
                std::mem::take(&mut row.stationed_hulls)
                    .into_vec()
                    .into_iter(),
            )),
            (Self::BoxedColony(row), 0) => Some(StateRetirementChildren::Reasons(
                std::mem::take(&mut row.hub.blocked_reasons)
                    .into_vec()
                    .into_iter(),
            )),
            (Self::BoxedFacility(row), 0) => Some(StateRetirementChildren::Reasons(
                std::mem::take(&mut row.blocked_reasons)
                    .into_vec()
                    .into_iter(),
            )),
            (Self::BoxedFleet(row), 0) => row.order.as_mut().map(|order| {
                StateRetirementChildren::Path(std::mem::take(&mut order.lane_path).into_iter())
            }),
            (Self::BoxedFleet(row), 1) => Some(StateRetirementChildren::Hulls(
                std::mem::take(&mut row.hulls).into_vec().into_iter(),
            )),
            (Self::BoxedObservation(row), 0) => Some(StateRetirementChildren::ObservedFacilities(
                std::mem::take(&mut row.facilities).into_vec().into_iter(),
            )),
            (Self::BoxedObservation(row), 1) => Some(StateRetirementChildren::ObservedHulls(
                std::mem::take(&mut row.stationed_hulls)
                    .into_vec()
                    .into_iter(),
            )),
            _ => None,
        }
    }
    fn is_boxed(&self) -> bool {
        matches!(
            self,
            Self::BoxedColony(_)
                | Self::BoxedFacility(_)
                | Self::BoxedFleet(_)
                | Self::BoxedObservation(_)
        )
    }
    fn from_copy(row: CopyNested) -> Self {
        match row {
            CopyNested::Colony(row, _) => Self::BoxedColony(row),
            CopyNested::Facility(row, _) => Self::BoxedFacility(row),
            CopyNested::Fleet(row, _, _) => Self::BoxedFleet(row),
            CopyNested::Observation(row, _, _) => Self::BoxedObservation(row),
        }
    }
    fn child_count(&self) -> usize {
        match self {
            Self::Colony(_) | Self::Facility(_) | Self::BoxedColony(_) | Self::BoxedFacility(_) => {
                1
            }
            Self::Fleet(_)
            | Self::Observation(_)
            | Self::BoxedFleet(_)
            | Self::BoxedObservation(_) => 2,
        }
    }
}

/// Move-only logical-record retirement. Suspending means retaining this job.
/// Ordinary Drop of an incomplete job or deferred ticket is not bounded.
/// Row leaves contain only fixed scalars and validated names/slugs of at most
/// 64 bytes; allocator execution time is not qualified by these record counts.
pub(crate) struct LivingStateRetirementJobV2 {
    owned: Option<LivingGalaxyStateV2>,
    boxed: Option<Box<LivingGalaxyStateV2>>,
    shared: Option<(Arc<LivingGalaxyStateV2>, LivingStateRetirementPolicyV2)>,
    rows: Option<StateRetirementRows>,
    parent: Option<StateRetirementParent>,
    children: Option<StateRetirementChildren>,
    child_index: usize,
    collection: usize,
    disposition: Option<LivingStateRetirementDispositionV2>,
    ready: bool,
    delivered: bool,
    last_poll_visits: usize,
    total_visits: u64,
    counts: StateRetirementCounts,
}
impl LivingStateRetirementJobV2 {
    fn empty() -> Self {
        Self {
            owned: None,
            boxed: None,
            shared: None,
            rows: None,
            parent: None,
            children: None,
            child_index: 0,
            collection: 0,
            disposition: None,
            ready: false,
            delivered: false,
            last_poll_visits: 0,
            total_visits: 0,
            counts: Default::default(),
        }
    }
    pub(crate) fn begin_owned(state: LivingGalaxyStateV2) -> Self {
        Self {
            owned: Some(state),
            ..Self::empty()
        }
    }
    pub(crate) fn begin_boxed(state: Box<LivingGalaxyStateV2>) -> Self {
        Self {
            boxed: Some(state),
            ..Self::empty()
        }
    }
    pub(crate) fn begin_shared(
        source: Arc<LivingGalaxyStateV2>,
        policy: LivingStateRetirementPolicyV2,
    ) -> Self {
        Self {
            shared: Some((source, policy)),
            ..Self::empty()
        }
    }
    pub(crate) fn resume(ticket: LivingDeferredStateRetirementV2) -> Self {
        Self::begin_shared(
            ticket.source,
            LivingStateRetirementPolicyV2::DeferUntilUnique,
        )
    }
    pub(crate) const fn last_poll_visits(&self) -> usize {
        self.last_poll_visits
    }
    pub(crate) const fn total_visits(&self) -> u64 {
        self.total_visits
    }
    pub(crate) fn poll(
        &mut self,
        budget: usize,
    ) -> Result<LivingLifecycleProgressV2, LivingLifecycleJobErrorV2> {
        self.last_poll_visits = 0;
        if budget == 0 || budget > LIVING_MAX_PHASE_POLL_VISITS_V2 {
            return Err(LivingLifecycleJobErrorV2::Budget);
        }
        while self.last_poll_visits < budget && !self.ready {
            self.visit();
            self.last_poll_visits += 1;
            self.total_visits += 1;
        }
        Ok(if self.ready {
            LivingLifecycleProgressV2::Ready
        } else {
            LivingLifecycleProgressV2::Pending
        })
    }
    pub(crate) fn finish(
        &mut self,
    ) -> Result<LivingStateRetirementDispositionV2, LivingLifecycleJobErrorV2> {
        if !self.ready || self.delivered {
            return Err(LivingLifecycleJobErrorV2::NotReady);
        }
        let output = self
            .disposition
            .take()
            .expect("ready retirement disposition");
        self.delivered = true;
        Ok(output)
    }
    fn visit(&mut self) {
        if let Some((source, policy)) = self.shared.take() {
            self.counts.shared_attempts += 1;
            let payload = match policy {
                LivingStateRetirementPolicyV2::ReleaseHandle => match Arc::into_inner(source) {
                    Some(state) => Some(state),
                    None => {
                        self.disposition = Some(LivingStateRetirementDispositionV2::SharedReleased);
                        None
                    }
                },
                LivingStateRetirementPolicyV2::DeferUntilUnique => match Arc::try_unwrap(source) {
                    Ok(state) => Some(state),
                    Err(source) => {
                        self.disposition = Some(LivingStateRetirementDispositionV2::Deferred(
                            LivingDeferredStateRetirementV2 { source },
                        ));
                        None
                    }
                },
            };
            if let Some(state) = payload {
                self.owned = Some(state);
            } else {
                self.ready = true;
            }
            return;
        }
        if let Some(children) = &mut self.children {
            if children.remaining() != 0 {
                children.retire_record();
                self.counts.nested_records += 1;
            } else {
                // Exhaustion is checked before Drop can release the buffer.
                drop(self.children.take().expect("exhausted child"));
                self.counts.storage_releases += 1;
                self.child_index += 1;
            }
            return;
        }
        if let Some(parent) = &mut self.parent {
            while self.child_index < parent.child_count() {
                if let Some(children) = parent.child(self.child_index) {
                    self.children = Some(children);
                    self.counts.buffer_moves += 1;
                    return;
                }
                // A missing order is an empty transition, not a buffer owner.
                self.child_index += 1;
            }
            self.counts.box_releases += u64::from(parent.is_boxed());
            drop(self.parent.take().expect("empty retired parent"));
            self.counts.parent_releases += 1;
            self.child_index = 0;
            return;
        }
        if let Some(rows) = &mut self.rows {
            if rows.remaining() != 0 {
                rows.retire_row(&mut self.parent);
                self.counts.rows += 1;
            } else {
                // Never drop or replace a partially consumed IntoIter.
                drop(self.rows.take().expect("exhausted collection"));
                self.counts.storage_releases += 1;
                self.collection += 1;
            }
            return;
        }
        if self.collection < 20 {
            let state = self
                .owned
                .as_mut()
                .or(self.boxed.as_deref_mut())
                .expect("state owner");
            self.rows = Some(StateRetirementRows::begin(state, self.collection));
            self.counts.buffer_moves += 1;
            return;
        }
        // All collection buffers and child owners have already been released.
        drop(self.owned.take());
        drop(self.boxed.take());
        self.counts.root_releases += 1;
        self.disposition = Some(LivingStateRetirementDispositionV2::Drained);
        self.ready = true;
    }
}

#[cfg(test)]
mod state_retirement_tests {
    use super::*;
    use crate::living::model::*;
    use crate::living::wire::LivingSortedVecV2;
    use crate::living::{LivingNameV2, LivingSlugV2};
    fn id(n: u64) -> LivingEntityIdV2 {
        let mut bytes = [0; 16];
        bytes[8..].copy_from_slice(&n.to_be_bytes());
        LivingEntityIdV2(bytes)
    }
    fn slug(s: &str) -> LivingSlugV2 {
        LivingSlugV2::new(s).unwrap()
    }
    fn one<T: LivingKeyedV2>(row: T) -> LivingSortedVecV2<T> {
        LivingSortedVecV2::new(vec![row]).unwrap()
    }
    pub(super) fn fixture() -> LivingGalaxyStateV2 {
        let name = || LivingNameV2::new("Retirement").unwrap();
        let reasons = || one(LivingBlockedReasonV2::Disabled);
        LivingGalaxyStateV2 {
            systems: one(LivingSystemV2 {
                id: id(1),
                name: name(),
            }),
            stars: one(LivingStarV2 {
                id: id(2),
                system: id(1),
                archetype: slug("missing"),
                name: name(),
            }),
            worlds: one(LivingWorldV2 {
                id: id(3),
                system: id(1),
                archetype: slug("missing"),
                name: name(),
                owner: Some(id(5)),
                inventory: LivingInventoryV2::default(),
            }),
            lanes: one(LivingLaneV2 {
                id: id(4),
                system_a: id(1),
                system_b: id(1),
                distance_units: 0,
            }),
            civilizations: one(LivingCivilizationV2 {
                id: id(5),
                name: name(),
                policy: LivingPolicyV2(slug("missing")),
                status: LivingCivilizationStatusV2::Dormant,
            }),
            deposits: one(LivingDepositV2 {
                id: id(6),
                world: id(3),
                resource: LivingResourceV2::Ore,
                remaining_units: 0,
            }),
            colonies: one(LivingColonyV2 {
                world: id(3),
                owner: id(999),
                hub: LivingHubV2 {
                    energy_next_due: LivingTickV2(0),
                    ore_next_due: LivingTickV2(0),
                    fallback_next_due: LivingTickV2(0),
                    blocked_reasons: reasons(),
                },
            }),
            facilities: one(LivingFacilityV2 {
                id: id(7),
                world: id(3),
                definition: slug("missing"),
                status: LivingFacilityStatusV2::Blocked,
                paid_alloy: 0,
                hit_points: 0,
                next_due: None,
                next_repair: None,
                blocked_reasons: reasons(),
                disabled: false,
            }),
            construction_jobs: one(LivingConstructionJobV2 {
                id: id(8),
                world: id(3),
                owner: id(5),
                definition: slug("missing"),
                accepted_tick: LivingTickV2(100),
                completion_tick: LivingTickV2(0),
                paid_alloy: 0,
            }),
            hull_jobs: one(LivingHullJobV2 {
                id: id(9),
                world: id(3),
                owner: id(5),
                definition: slug("missing"),
                accepted_tick: LivingTickV2(100),
                completion_tick: LivingTickV2(0),
                paid_alloy: 0,
                target_fleet: Some(id(999)),
            }),
            fleets: one(LivingFleetV2 {
                id: id(10),
                owner: id(5),
                location: LivingFleetLocationV2::Docked { world: id(999) },
                order: Some(LivingFleetOrderV2 {
                    kind: LivingFleetOrderKindV2::Move,
                    target_world: None,
                    not_before_tick: LivingTickV2(0),
                    lane_path: vec![id(99), id(98)],
                }),
                hulls: one(LivingHullV2 {
                    id: id(11),
                    definition: slug("missing"),
                    hit_points: u32::MAX,
                    return_credit: false,
                    next_repair: None,
                }),
            }),
            routes: one(LivingFreightRouteV2 {
                id: id(12),
                kind: LivingRouteKindV2::Internal,
                source_world: id(3),
                destination_world: id(3),
                resource: LivingResourceV2::Ore,
                batch_size: 0,
                cadence_ticks: 0,
                source_reserve: 0,
                source_owner: id(5),
                receiver_owner: id(999),
                next_due: LivingTickV2(0),
                suspended: false,
            }),
            shipments: one(LivingShipmentV2 {
                id: id(13),
                dispatch_owner: id(5),
                intended_receiver: id(999),
                source_world: id(3),
                destination_world: id(3),
                resource: LivingResourceV2::Ore,
                units: 0,
                departure_tick: LivingTickV2(100),
                arrival_tick: LivingTickV2(0),
                disposition: LivingShipmentDispositionV2::Outbound,
                dispatch_kind: None,
                returning_to_source: true,
            }),
            relations: one(LivingRelationV2 {
                from: id(5),
                to: id(5),
                base_disposition: i32::MAX,
                reasons: Default::default(),
            }),
            agreements: one(LivingAgreementV2 {
                kind: LivingAgreementKindV2::Trade,
                participant_a: id(5),
                participant_b: id(5),
                start_tick: LivingTickV2(100),
                end_tick: LivingTickV2(0),
            }),
            wars: one(LivingWarV2 {
                participant_a: id(5),
                participant_b: id(5),
                declarer: id(999),
                start_tick: LivingTickV2(100),
                end_tick: LivingTickV2(0),
            }),
            observations: one(LivingObservationV2 {
                civilization: id(5),
                world: id(3),
                observed_tick: LivingTickV2(u64::MAX),
                owner: Some(id(999)),
                remaining_deposit: 0,
                facilities: one(LivingObservedFacilityV2 {
                    definition: slug("missing"),
                    count: 0,
                }),
                stationed_hulls: one(LivingObservedHullV2 {
                    definition: slug("missing"),
                    count: 0,
                }),
                operational_defense_batteries: Some(0),
            }),
            hazards: one(LivingHazardV2 {
                id: id(14),
                lane: id(999),
                definition: slug("missing"),
                start_tick: LivingTickV2(100),
                end_tick: LivingTickV2(0),
            }),
            settlement_claims: one(LivingSettlementClaimV2 {
                world: id(3),
                civilization: id(999),
                arrival_tick: LivingTickV2(100),
                close_tick: LivingTickV2(0),
            }),
            occupations: one(LivingOccupationV2 {
                world: id(3),
                claimant: id(999),
                progress_ticks: u32::MAX,
            }),
            ..Default::default()
        }
    }
    fn run(
        mut job: LivingStateRetirementJobV2,
        budgets: &[usize],
    ) -> (
        LivingStateRetirementDispositionV2,
        StateRetirementCounts,
        u64,
    ) {
        assert_eq!(job.total_visits(), 0);
        assert!(matches!(
            job.finish(),
            Err(LivingLifecycleJobErrorV2::NotReady)
        ));
        let mut i = 0;
        loop {
            let budget = budgets[i % budgets.len()];
            i += 1;
            let previous = job.total_visits();
            let progress = job.poll(budget).unwrap();
            assert!(job.last_poll_visits() > 0 && job.last_poll_visits() <= budget);
            assert_eq!(job.total_visits() - previous, job.last_poll_visits() as u64);
            if progress == LivingLifecycleProgressV2::Ready {
                break;
            }
            assert!(matches!(
                job.finish(),
                Err(LivingLifecycleJobErrorV2::NotReady)
            ));
        }
        let total = job.total_visits();
        let out = job.finish().unwrap();
        assert!(matches!(
            job.finish(),
            Err(LivingLifecycleJobErrorV2::NotReady)
        ));
        assert_eq!(job.poll(1), Ok(LivingLifecycleProgressV2::Ready));
        assert_eq!(job.last_poll_visits(), 0);
        assert_eq!(job.total_visits(), total);
        assert_eq!(
            total,
            job.counts.buffer_moves
                + job.counts.rows
                + job.counts.nested_records
                + job.counts.storage_releases
                + job.counts.parent_releases
                + job.counts.root_releases
                + job.counts.shared_attempts
        );
        (out, job.counts, total)
    }
    #[test]
    fn state_retirement_all_twenty_collections_owned_boxed_and_exact_counts() {
        let expected = StateRetirementCounts {
            buffer_moves: 26,
            rows: 20,
            nested_records: 7,
            storage_releases: 26,
            parent_releases: 4,
            root_releases: 1,
            shared_attempts: 0,
            box_releases: 0,
        };
        for budget in [&[1][..], &[64], &[1024], &[1, 64, 7, 1024, 3]] {
            for boxed in [false, true] {
                let state = fixture();
                let job = if boxed {
                    LivingStateRetirementJobV2::begin_boxed(Box::new(state))
                } else {
                    LivingStateRetirementJobV2::begin_owned(state)
                };
                let (out, counts, total) = run(job, budget);
                assert!(matches!(out, LivingStateRetirementDispositionV2::Drained));
                assert_eq!(counts, expected);
                assert_eq!(total, 84);
            }
        }
    }
    #[test]
    fn state_retirement_empty_capacious_buffers_and_invalid_budget_retry() {
        let state = LivingGalaxyStateV2 {
            systems: LivingSortedVecV2::new(Vec::with_capacity(4096)).unwrap(),
            fleets: LivingSortedVecV2::new(Vec::with_capacity(4096)).unwrap(),
            ..Default::default()
        };
        let mut job = LivingStateRetirementJobV2::begin_boxed(Box::new(state));
        let pointer = job.boxed.as_ref().unwrap().as_ref() as *const LivingGalaxyStateV2;
        for bad in [0, 1025] {
            assert_eq!(job.poll(bad), Err(LivingLifecycleJobErrorV2::Budget));
            assert_eq!(job.last_poll_visits(), 0);
            assert_eq!(job.total_visits(), 0);
            assert_eq!(job.boxed.as_ref().unwrap().as_ref() as *const _, pointer);
        }
        let (out, counts, total) = run(job, &[1]);
        assert!(matches!(out, LivingStateRetirementDispositionV2::Drained));
        assert_eq!(total, 41);
        assert_eq!(counts.buffer_moves, 20);
        assert_eq!(counts.storage_releases, 20);
        assert_eq!(counts.rows, 0);
    }
    #[test]
    fn state_retirement_large_unsorted_nested_records_remain_sliced() {
        let mut state = fixture();
        let fleet = state.fleets.record_at_mut(0).unwrap();
        let prototype = fleet.hulls.as_slice()[0].clone();
        fleet.hulls = LivingSortedVecV2::new(
            (0..3000)
                .map(|i| LivingHullV2 {
                    id: id(i),
                    ..prototype.clone()
                })
                .collect(),
        )
        .unwrap();
        // Deliberately corrupt key ordering through the private mutable API.
        fleet.hulls.record_at_mut(1).unwrap().id = id(0);
        fleet.order.as_mut().unwrap().lane_path = vec![id(999); 3001];
        let mut totals = Vec::new();
        for budget in [&[1][..], &[64], &[1024], &[1, 64, 7, 1024, 3]] {
            let (out, counts, total) = run(
                LivingStateRetirementJobV2::begin_owned(state.clone()),
                budget,
            );
            assert!(matches!(out, LivingStateRetirementDispositionV2::Drained));
            assert_eq!(counts.nested_records, 6005);
            totals.push(total);
        }
        assert_eq!(totals, vec![totals[0]; 4]);
        // Also exercise long reason and remembered-record collections without
        // their normal sorting/capacity restrictions being retirement conditions.
        let facility = state.facilities.record_at_mut(0).unwrap();
        facility.blocked_reasons = LivingSortedVecV2::new(vec![
            LivingBlockedReasonV2::Disabled,
            LivingBlockedReasonV2::MissingDeposit {
                required: 0,
                available: 0,
            },
        ])
        .unwrap();
        let observation = state.observations.record_at_mut(0).unwrap();
        observation.facilities = LivingSortedVecV2::new(
            (0..1100)
                .map(|i| LivingObservedFacilityV2 {
                    definition: slug(&format!("f{i:04}")),
                    count: 0,
                })
                .collect(),
        )
        .unwrap();
        observation.stationed_hulls = LivingSortedVecV2::new(
            (0..1101)
                .map(|i| LivingObservedHullV2 {
                    definition: slug(&format!("h{i:04}")),
                    count: 0,
                })
                .collect(),
        )
        .unwrap();
        observation.facilities.record_at_mut(1).unwrap().definition = slug("f0000");
        let (_, counts, _) = run(LivingStateRetirementJobV2::begin_owned(state), &[1]);
        assert_eq!(counts.nested_records, 8205);
    }
    #[test]
    fn state_retirement_budget_boundary_keeps_parent_buffer_and_untouched_suffix() {
        let state = LivingGalaxyStateV2 {
            systems: LivingSortedVecV2::new(
                (0..5)
                    .map(|i| LivingSystemV2 {
                        id: id(i),
                        name: LivingNameV2::new("Retained").unwrap(),
                    })
                    .collect(),
            )
            .unwrap(),
            ..Default::default()
        };
        let base = state.systems.as_slice().as_ptr();
        let mut job = LivingStateRetirementJobV2::begin_owned(state);
        assert_eq!(job.poll(1), Ok(LivingLifecycleProgressV2::Pending));
        assert_eq!(job.counts.buffer_moves, 1);
        assert!(job.owned.as_ref().unwrap().systems.as_slice().is_empty());
        assert_eq!(job.poll(2), Ok(LivingLifecycleProgressV2::Pending));
        let Some(StateRetirementRows::Systems(rows)) = &job.rows else {
            panic!("systems owner")
        };
        assert_eq!(rows.len(), 3);
        assert_eq!(rows.as_slice()[0].id, id(2));
        assert_eq!(rows.as_slice().as_ptr(), base.wrapping_add(2));
        let saved_total = job.total_visits();
        assert!(matches!(
            job.finish(),
            Err(LivingLifecycleJobErrorV2::NotReady)
        ));
        assert_eq!(job.poll(0), Err(LivingLifecycleJobErrorV2::Budget));
        assert_eq!(job.total_visits(), saved_total);
        while job.poll(1).unwrap() != LivingLifecycleProgressV2::Ready {}
        assert!(matches!(
            job.finish().unwrap(),
            LivingStateRetirementDispositionV2::Drained
        ));

        let mut job = LivingStateRetirementJobV2::begin_owned(fixture());
        while !matches!(job.children, Some(StateRetirementChildren::Path(_))) {
            job.poll(1).unwrap();
        }
        job.poll(1).unwrap();
        let Some(StateRetirementChildren::Path(path)) = &job.children else {
            panic!("path owner")
        };
        assert_eq!(path.as_slice(), &[id(98)]);
        assert!(matches!(job.parent, Some(StateRetirementParent::Fleet(_))));
        while job.poll(1).unwrap() != LivingLifecycleProgressV2::Ready {}
        assert!(job.parent.is_none() && job.children.is_none() && job.rows.is_none());
        let _ = job.finish().unwrap();
    }
    #[test]
    fn state_retirement_shared_release_both_orders_exactly_one_payload() {
        for reverse in [false, true] {
            let original = Arc::new(fixture());
            let weak = Arc::downgrade(&original);
            let alias = Arc::clone(&original);
            let (first, last) = if reverse {
                (alias, original)
            } else {
                (original, alias)
            };
            let (out, counts, total) = run(
                LivingStateRetirementJobV2::begin_shared(
                    first,
                    LivingStateRetirementPolicyV2::ReleaseHandle,
                ),
                &[1],
            );
            assert!(matches!(
                out,
                LivingStateRetirementDispositionV2::SharedReleased
            ));
            assert_eq!(counts.shared_attempts, 1);
            assert_eq!(counts.rows, 0);
            assert_eq!(total, 1);
            assert_eq!(*weak.upgrade().unwrap(), *last);
            let (out, counts, total) = run(
                LivingStateRetirementJobV2::begin_shared(
                    last,
                    LivingStateRetirementPolicyV2::ReleaseHandle,
                ),
                &[64],
            );
            assert!(matches!(out, LivingStateRetirementDispositionV2::Drained));
            assert_eq!(counts.rows, 20);
            assert_eq!(total, 85);
            assert!(weak.upgrade().is_none());
        }
    }
    #[test]
    fn state_retirement_concurrent_shared_release_never_destroys_payload_unvisited() {
        let source = Arc::new(fixture());
        let alias = Arc::clone(&source);
        let left = std::thread::spawn(move || {
            run(
                LivingStateRetirementJobV2::begin_shared(
                    source,
                    LivingStateRetirementPolicyV2::ReleaseHandle,
                ),
                &[1],
            )
        });
        let right = std::thread::spawn(move || {
            run(
                LivingStateRetirementJobV2::begin_shared(
                    alias,
                    LivingStateRetirementPolicyV2::ReleaseHandle,
                ),
                &[64],
            )
        });
        let (a, ac, _) = left.join().unwrap();
        let (b, bc, _) = right.join().unwrap();
        assert_eq!(ac.rows + bc.rows, 20);
        assert!(matches!(
            (a, b),
            (
                LivingStateRetirementDispositionV2::Drained,
                LivingStateRetirementDispositionV2::SharedReleased
            ) | (
                LivingStateRetirementDispositionV2::SharedReleased,
                LivingStateRetirementDispositionV2::Drained
            )
        ));
    }
    #[test]
    fn state_retirement_deferred_ticket_preserves_pointer_and_resumes_unique() {
        let source = Arc::new(fixture());
        let expected = source.canonical_bytes().unwrap();
        let pointer = Arc::as_ptr(&source);
        let weak = Arc::downgrade(&source);
        let alias = Arc::clone(&source);
        let (out, counts, total) = run(
            LivingStateRetirementJobV2::begin_shared(
                source,
                LivingStateRetirementPolicyV2::DeferUntilUnique,
            ),
            &[1024],
        );
        let LivingStateRetirementDispositionV2::Deferred(ticket) = out else {
            panic!("deferred")
        };
        assert_eq!(total, 1);
        assert_eq!(counts.shared_attempts, 1);
        assert_eq!(Arc::as_ptr(&ticket.source), pointer);
        assert_eq!(alias.canonical_bytes().unwrap(), expected);
        assert!(Arc::ptr_eq(&weak.upgrade().unwrap(), &ticket.source));
        // One subsequent attempt is still blocked; it returns the same owner.
        let (out, _, total) = run(LivingStateRetirementJobV2::resume(ticket), &[1024]);
        assert_eq!(total, 1);
        let LivingStateRetirementDispositionV2::Deferred(ticket) = out else {
            panic!("still deferred")
        };
        assert_eq!(Arc::as_ptr(&ticket.source), pointer);
        drop(alias);
        let (out, counts, total) = run(LivingStateRetirementJobV2::resume(ticket), &[1]);
        assert!(matches!(out, LivingStateRetirementDispositionV2::Drained));
        assert_eq!(total, 85);
        assert_eq!(counts.rows, 20);
        assert!(weak.upgrade().is_none());
    }

    #[test]
    fn state_retirement_empty_capacious_nested_buffers_release_before_parent() {
        let mut state = fixture();
        let empty_reasons = || LivingSortedVecV2::new(Vec::with_capacity(4096)).unwrap();
        state.colonies.record_at_mut(0).unwrap().hub.blocked_reasons = empty_reasons();
        state.facilities.record_at_mut(0).unwrap().blocked_reasons = empty_reasons();
        let fleet = state.fleets.record_at_mut(0).unwrap();
        fleet.order.as_mut().unwrap().lane_path = Vec::with_capacity(4096);
        fleet.hulls = LivingSortedVecV2::new(Vec::with_capacity(4096)).unwrap();
        let observation = state.observations.record_at_mut(0).unwrap();
        observation.facilities = LivingSortedVecV2::new(Vec::with_capacity(4096)).unwrap();
        observation.stationed_hulls = LivingSortedVecV2::new(Vec::with_capacity(4096)).unwrap();
        let mut job = LivingStateRetirementJobV2::begin_boxed(Box::new(state));
        let pointer = job.boxed.as_ref().unwrap().as_ref() as *const LivingGalaxyStateV2;
        while !matches!(job.children, Some(StateRetirementChildren::Path(_))) {
            job.poll(1).unwrap();
        }
        let Some(StateRetirementChildren::Path(path)) = &job.children else {
            panic!("empty path buffer retained")
        };
        assert!(path.as_slice().is_empty());
        assert!(matches!(job.parent, Some(StateRetirementParent::Fleet(_))));
        assert_eq!(job.boxed.as_ref().unwrap().as_ref() as *const _, pointer);
        let parent_releases = job.counts.parent_releases;
        let storage = job.counts.storage_releases;
        job.poll(1).unwrap();
        assert_eq!(job.counts.storage_releases, storage + 1);
        assert_eq!(job.counts.parent_releases, parent_releases);
        assert!(job.children.is_none() && job.parent.is_some());
        while job.poll(1).unwrap() != LivingLifecycleProgressV2::Ready {}
        assert_eq!(job.counts.nested_records, 0);
        assert_eq!(job.counts.buffer_moves, 26);
        assert_eq!(job.counts.storage_releases, 26);
        assert_eq!(job.counts.parent_releases, 4);
        assert_eq!(job.total_visits(), 77);
        assert!(job.boxed.is_none());
        let _ = job.finish().unwrap();
    }
    #[test]
    fn state_retirement_all_blocked_reason_variants_are_individual_records() {
        let mut state = fixture();
        let mut reasons = vec![
            LivingBlockedReasonV2::Disabled,
            LivingBlockedReasonV2::MissingDeposit {
                required: 0,
                available: 0,
            },
        ];
        for resource in [
            LivingResourceV2::Alloy,
            LivingResourceV2::Energy,
            LivingResourceV2::Ore,
        ] {
            reasons.push(LivingBlockedReasonV2::MissingInput {
                resource,
                required: 0,
                available: 0,
            });
        }
        for resource in [
            LivingResourceV2::Alloy,
            LivingResourceV2::Energy,
            LivingResourceV2::Ore,
        ] {
            reasons.push(LivingBlockedReasonV2::OutputFull { resource });
        }
        state.colonies.record_at_mut(0).unwrap().hub.blocked_reasons =
            LivingSortedVecV2::new(reasons.clone()).unwrap();
        state.facilities.record_at_mut(0).unwrap().blocked_reasons =
            LivingSortedVecV2::new(reasons).unwrap();
        let (_, counts, total) = run(LivingStateRetirementJobV2::begin_owned(state), &[1]);
        assert_eq!(counts.nested_records, 21);
        assert_eq!(total, 98);
    }
    #[test]
    fn state_retirement_production_handoff_of_cancelled_and_faulted_copy_owners() {
        for fail in [false, true] {
            let mut state = fixture();
            let fleet = state.fleets.record_at_mut(0).unwrap();
            let prototype = fleet.hulls.as_slice()[0].clone();
            fleet.hulls = LivingSortedVecV2::new(
                (0..3)
                    .map(|i| LivingHullV2 {
                        id: id(i),
                        ..prototype.clone()
                    })
                    .collect(),
            )
            .unwrap();
            if fail {
                fleet.hulls.record_at_mut(1).unwrap().id = id(0);
            }
            let source = Arc::new(state);
            let before = source.as_ref().clone();
            let mut copy = LivingStateCopyJobV2::begin(Arc::clone(&source));
            if fail {
                loop {
                    if let Err(LivingLifecycleJobErrorV2::Fault(_)) = copy.poll(1) {
                        break;
                    }
                }
                assert!(copy.fault.is_some());
            } else {
                while !matches!(copy.nested, Some(CopyNested::Fleet(_, _, 1))) {
                    copy.poll(1).unwrap();
                }
                copy.cancel();
                assert_eq!(copy.poll(1), Err(LivingLifecycleJobErrorV2::Cancelled));
            }
            let mut job = copy.into_retirement(LivingStateRetirementPolicyV2::ReleaseHandle);
            assert!(matches!(
                job.candidate.as_ref().unwrap().parent,
                Some(StateRetirementParent::BoxedFleet(_))
            ));
            while job.poll(1).unwrap() != LivingLifecycleProgressV2::Ready {}
            assert!(matches!(
                job.finish().unwrap(),
                LivingStateRetirementDispositionV2::SharedReleased
            ));
            assert_eq!(*source, before);
            let (out, _, _) = run(
                LivingStateRetirementJobV2::begin_shared(
                    source,
                    LivingStateRetirementPolicyV2::ReleaseHandle,
                ),
                &[64],
            );
            assert!(matches!(out, LivingStateRetirementDispositionV2::Drained));
        }
    }
    #[test]
    fn state_retirement_unique_shared_with_weak_alias_and_absent_order() {
        for policy in [
            LivingStateRetirementPolicyV2::ReleaseHandle,
            LivingStateRetirementPolicyV2::DeferUntilUnique,
        ] {
            let mut state = fixture();
            state.fleets.record_at_mut(0).unwrap().order = None;
            let source = Arc::new(state);
            let weak = Arc::downgrade(&source);
            let (out, counts, total) = run(
                LivingStateRetirementJobV2::begin_shared(source, policy),
                &[1, 64],
            );
            assert!(matches!(out, LivingStateRetirementDispositionV2::Drained));
            assert_eq!(counts.buffer_moves, 25);
            assert_eq!(counts.storage_releases, 25);
            assert_eq!(counts.nested_records, 5);
            assert_eq!(total, 81);
            assert!(weak.upgrade().is_none());
        }
    }
    fn drain_adapter(
        mut job: LivingCopyHashRetirementJobV2,
        budgets: &[usize],
    ) -> LivingStateRetirementDispositionV2 {
        assert!(matches!(
            job.finish(),
            Err(LivingLifecycleJobErrorV2::NotReady)
        ));
        assert_eq!(job.poll(0), Err(LivingLifecycleJobErrorV2::Budget));
        assert_eq!(job.poll(1025), Err(LivingLifecycleJobErrorV2::Budget));
        let mut polls = 0;
        loop {
            let budget = budgets[polls % budgets.len()];
            let progress = job.poll(budget).unwrap();
            assert!(job.last_poll_visits() <= budget);
            polls += 1;
            if progress == LivingLifecycleProgressV2::Ready {
                break;
            }
            assert!(matches!(
                job.finish(),
                Err(LivingLifecycleJobErrorV2::NotReady)
            ));
        }
        let visits = job.total_visits();
        assert!(visits > 0);
        assert_eq!(job.poll(1), Ok(LivingLifecycleProgressV2::Ready));
        assert_eq!(job.last_poll_visits(), 0);
        assert_eq!(job.total_visits(), visits);
        let result = job.finish().unwrap();
        assert!(matches!(
            job.finish(),
            Err(LivingLifecycleJobErrorV2::NotReady)
        ));
        result
    }
    #[test]
    fn copy_hash_borrowed_extraction_and_deferred_handoff_are_once_only() {
        for budgets in [&[1][..], &[64], &[1024], &[1, 64, 7, 1024]] {
            let source = Arc::new(fixture());
            let pointer = Arc::as_ptr(&source);
            let mut copy = LivingStateCopyJobV2::begin(Arc::clone(&source));
            assert_eq!(
                copy.take_ready_result(),
                Err(LivingLifecycleJobErrorV2::NotReady)
            );
            assert_eq!(Arc::as_ptr(&copy.source), pointer);
            assert_eq!(copy.poll(1025), Err(LivingLifecycleJobErrorV2::Budget));
            let mut polls = 0;
            while copy.poll(budgets[polls % budgets.len()]).unwrap()
                != LivingLifecycleProgressV2::Ready
            {
                polls += 1;
            }
            assert!(copy.storage_preparations >= 20);
            let candidate = copy.take_ready_result().unwrap();
            assert_eq!(candidate, *source);
            assert_eq!(
                copy.take_ready_result(),
                Err(LivingLifecycleJobErrorV2::NotReady)
            );
            assert_eq!(copy.poll(1), Ok(LivingLifecycleProgressV2::Ready));
            assert_eq!(copy.last_poll_visits(), 0);
            let out = drain_adapter(
                copy.into_retirement(LivingStateRetirementPolicyV2::DeferUntilUnique),
                budgets,
            );
            let LivingStateRetirementDispositionV2::Deferred(ticket) = out else {
                panic!("source alias must be deferred")
            };
            assert_eq!(Arc::as_ptr(&ticket.source), pointer);
            let mut hash = LivingStateHashJobV2::begin(Arc::clone(&source));
            assert_eq!(
                hash.take_ready_result(),
                Err(LivingLifecycleJobErrorV2::NotReady)
            );
            assert_eq!(hash.poll(0), Err(LivingLifecycleJobErrorV2::Budget));
            assert_eq!(hash.poll(1025), Err(LivingLifecycleJobErrorV2::Budget));
            while hash.poll(budgets[polls % budgets.len()]).unwrap()
                != LivingLifecycleProgressV2::Ready
            {
                polls += 1;
            }
            assert_eq!(hash.take_ready_result().unwrap(), source.digest().unwrap());
            assert_eq!(
                hash.take_ready_result(),
                Err(LivingLifecycleJobErrorV2::NotReady)
            );
            assert_eq!(hash.poll(1), Ok(LivingLifecycleProgressV2::Ready));
            assert_eq!(hash.last_poll_visits(), 0);
            assert!(matches!(
                drain_adapter(
                    hash.into_retirement(LivingStateRetirementPolicyV2::ReleaseHandle),
                    budgets
                ),
                LivingStateRetirementDispositionV2::SharedReleased
            ));
            drop(source);
            let (out, _, _) = run(LivingStateRetirementJobV2::resume(ticket), budgets);
            assert!(matches!(out, LivingStateRetirementDispositionV2::Drained));
            let _ = run(LivingStateRetirementJobV2::begin_owned(candidate), budgets);
        }
    }
    #[test]
    fn copy_hash_leaf_encoding_reference_parity_and_full_oversize_measurement() {
        let mut state = fixture();
        let escaped = LivingNameV2::new("\\\"".repeat(32)).unwrap();
        state.systems.record_at_mut(0).unwrap().name = escaped.clone();
        state.worlds.record_at_mut(0).unwrap().name = escaped;
        state.facilities.record_at_mut(0).unwrap().disabled = true;
        state
            .observations
            .record_at_mut(0)
            .unwrap()
            .operational_defense_batteries = Some(u16::MAX);
        let source = Arc::new(state);
        let expected = source.canonical_bytes().unwrap();
        let mut totals = Vec::new();
        for budgets in [&[1][..], &[64], &[1024], &[1, 64, 7, 1024]] {
            for limit in [256, expected.len(), expected.len() + 1] {
                let reference = super::super::wire::encode_canonical_v2(source.as_ref(), limit);
                let mut hash = LivingStateHashJobV2::begin(Arc::clone(&source));
                hash.byte_limit = limit;
                let mut polls = 0;
                let mut frozen = None;
                let error = loop {
                    let result = hash.poll(budgets[polls % budgets.len()]);
                    assert!(hash.last_poll_visits() <= budgets[polls % budgets.len()]);
                    assert!(hash.counts.max_encoded_token <= 256);
                    assert!(hash.fragment.as_ref().map_or(0, Vec::len) <= 256);
                    if let Some(bytes) = frozen {
                        assert_eq!(hash.bytes, bytes);
                    }
                    if hash.over_limit {
                        frozen = Some(hash.bytes);
                    }
                    polls += 1;
                    match result {
                        Ok(LivingLifecycleProgressV2::Pending) => {}
                        Ok(LivingLifecycleProgressV2::Ready) => break None,
                        Err(error) => break Some(error),
                    }
                };
                assert_eq!(hash.measured_bytes, expected.len());
                assert_eq!(hash.counts.max_encoded_token, 130);
                assert!(hash.counts.nested_records >= 7);
                assert_eq!(hash.counts.tokens, hash.counts.storage_releases);
                assert!(hash.counts.structural_tokens > 0);
                assert!(hash.counts.structural_tokens < hash.counts.tokens);
                totals.push(hash.total_visits());
                if let Err(reference) = reference {
                    let expected_error = LivingLifecycleJobErrorV2::Fault(reference.into());
                    assert_eq!(error, Some(expected_error.clone()));
                    assert_eq!(hash.take_ready_result(), Err(expected_error.clone()));
                    let total = hash.total_visits();
                    assert_eq!(hash.poll(1), Err(expected_error));
                    assert_eq!(hash.last_poll_visits(), 0);
                    assert_eq!(hash.total_visits(), total);
                    assert!(hash.encoded.len() <= limit);
                } else {
                    assert_eq!(error, None);
                    assert_eq!(hash.encoded, expected);
                    assert_eq!(hash.take_ready_result().unwrap(), source.digest().unwrap());
                }
                let _ = drain_adapter(
                    hash.into_retirement(LivingStateRetirementPolicyV2::ReleaseHandle),
                    budgets,
                );
            }
        }
        assert!(totals.iter().all(|visits| *visits == totals[0]));
    }
    #[test]
    fn copy_hash_cancel_every_visit_retains_source_and_partial_allocations() {
        let source = Arc::new(fixture());
        // One cancellation at every boundary covers structural, selected-leaf,
        // buffer release, nested-record, complete and oversize-measurement phases.
        for oversize in [false, true] {
            let mut probe = LivingStateHashJobV2::begin(Arc::clone(&source));
            if oversize {
                probe.byte_limit = 64;
            }
            while matches!(probe.poll(1024), Ok(LivingLifecycleProgressV2::Pending)) {}
            let visits = probe.total_visits();
            let _ = drain_adapter(
                probe.into_retirement(LivingStateRetirementPolicyV2::ReleaseHandle),
                &[1024],
            );
            for cut in 0..=visits {
                let mut hash = LivingStateHashJobV2::begin(Arc::clone(&source));
                hash.byte_limit = if oversize { 64 } else { 8192 };
                for _ in 0..cut {
                    let _ = hash.poll(1);
                }
                let pointer = hash.fragment.as_ref().map(|bytes| bytes.as_ptr());
                hash.cancel();
                assert_eq!(
                    hash.take_ready_result(),
                    Err(LivingLifecycleJobErrorV2::Cancelled)
                );
                assert_eq!(hash.poll(1), Err(LivingLifecycleJobErrorV2::Cancelled));
                assert_eq!(hash.last_poll_visits(), 0);
                let job = hash.into_retirement(LivingStateRetirementPolicyV2::ReleaseHandle);
                assert_eq!(job.fragment.as_ref().map(|bytes| bytes.as_ptr()), pointer);
                assert!(matches!(
                    drain_adapter(job, &[64]),
                    LivingStateRetirementDispositionV2::SharedReleased
                ));
            }
        }
        let mut probe = LivingStateCopyJobV2::begin(Arc::clone(&source));
        while probe.poll(1024).unwrap() != LivingLifecycleProgressV2::Ready {}
        let visits = probe.total_visits();
        let _ = drain_adapter(
            probe.into_retirement(LivingStateRetirementPolicyV2::ReleaseHandle),
            &[1024],
        );
        for cut in 0..=visits {
            let mut copy = LivingStateCopyJobV2::begin(Arc::clone(&source));
            for _ in 0..cut {
                copy.poll(1).unwrap();
            }
            copy.cancel();
            assert_eq!(
                copy.take_ready_result(),
                Err(LivingLifecycleJobErrorV2::Cancelled)
            );
            assert_eq!(copy.poll(1), Err(LivingLifecycleJobErrorV2::Cancelled));
            assert_eq!(copy.last_poll_visits(), 0);
            let _ = drain_adapter(
                copy.into_retirement(LivingStateRetirementPolicyV2::ReleaseHandle),
                &[1, 64],
            );
        }
        assert_eq!(Arc::strong_count(&source), 1);
    }
    #[test]
    fn copy_hash_outer_key_preflight_retains_exact_box_for_all_nested_types() {
        for collection in [6, 7, 10, 16] {
            for descending in [false, true] {
                let mut state = fixture();
                macro_rules! duplicate {
                    ($field:ident, $key:ident) => {{
                        let first = state.$field.as_slice()[0].clone();
                        let mut second = first.clone();
                        second.$key = id(1000);
                        state.$field = LivingSortedVecV2::new(vec![first.clone(), second]).unwrap();
                        state.$field.record_at_mut(1).unwrap().$key =
                            if descending { id(0) } else { first.$key };
                    }};
                }
                match collection {
                    6 => duplicate!(colonies, world),
                    7 => duplicate!(facilities, id),
                    10 => duplicate!(fleets, id),
                    _ => duplicate!(observations, world),
                }
                let source = Arc::new(state);
                let mut copy = LivingStateCopyJobV2::begin(Arc::clone(&source));
                while copy.collection != collection || copy.index != 1 || copy.nested.is_none() {
                    copy.poll(1).unwrap();
                }
                let pointer = match copy.nested.as_ref().unwrap() {
                    CopyNested::Colony(row, _) => std::ptr::from_ref(row.as_ref()).cast::<()>(),
                    CopyNested::Facility(row, _) => std::ptr::from_ref(row.as_ref()).cast::<()>(),
                    CopyNested::Fleet(row, _, _) => std::ptr::from_ref(row.as_ref()).cast::<()>(),
                    CopyNested::Observation(row, _, _) => {
                        std::ptr::from_ref(row.as_ref()).cast::<()>()
                    }
                };
                let error = loop {
                    if let Err(error) = copy.poll(1) {
                        break error;
                    }
                };
                assert_eq!(
                    error,
                    LivingLifecycleJobErrorV2::Fault(
                        super::super::wire::LivingWireErrorV2::KeyedOrder.into()
                    )
                );
                assert_eq!(copy.take_ready_result(), Err(error.clone()));
                assert_eq!(copy.poll(1), Err(error));
                let mut retirement =
                    copy.into_retirement(LivingStateRetirementPolicyV2::ReleaseHandle);
                let candidate = retirement.candidate.as_mut().unwrap();
                let found = match candidate.parent.as_ref().unwrap() {
                    StateRetirementParent::BoxedColony(row) => {
                        std::ptr::from_ref(row.as_ref()).cast::<()>()
                    }
                    StateRetirementParent::BoxedFacility(row) => {
                        std::ptr::from_ref(row.as_ref()).cast::<()>()
                    }
                    StateRetirementParent::BoxedFleet(row) => {
                        std::ptr::from_ref(row.as_ref()).cast::<()>()
                    }
                    StateRetirementParent::BoxedObservation(row) => {
                        std::ptr::from_ref(row.as_ref()).cast::<()>()
                    }
                    _ => panic!("must retain boxed partial row"),
                };
                assert_eq!(found, pointer);
                while candidate.parent.is_some() {
                    candidate.poll(1).unwrap();
                }
                assert_eq!(candidate.counts.box_releases, 1);
                assert!(candidate.counts.nested_records > 0);
                let _ = drain_adapter(retirement, &[1, 64, 1024]);
            }
        }
    }
    #[test]
    fn copy_hash_real_archive_limit_measures_full_document_all_budgets() {
        let mut state = fixture();
        // Each ID is 34 quoted bytes plus its separator. This malformed authority
        // intentionally exceeds normal path/state limits; hashing must still
        // match canonical serialization's full document TooLarge.actual.
        state
            .fleets
            .record_at_mut(0)
            .unwrap()
            .order
            .as_mut()
            .unwrap()
            .lane_path = vec![id(99); super::super::wire::LIVING_MAX_ARCHIVE_BYTES_V2 / 35 + 1];
        let source = Arc::new(state);
        let expected = source.canonical_bytes().unwrap_err();
        let super::super::wire::LivingWireErrorV2::TooLarge { actual, .. } = expected else {
            panic!("oversize fixture")
        };
        let mut totals = Vec::new();
        for budgets in [&[1][..], &[64], &[1024], &[1, 64, 7, 1024]] {
            let mut hash = LivingStateHashJobV2::begin(Arc::clone(&source));
            let mut poll = 0;
            let error = loop {
                match hash.poll(budgets[poll % budgets.len()]) {
                    Ok(LivingLifecycleProgressV2::Pending) => {}
                    Ok(LivingLifecycleProgressV2::Ready) => panic!("oversize digest exposed"),
                    Err(error) => break error,
                }
                poll += 1;
            };
            assert_eq!(error, LivingLifecycleJobErrorV2::Fault(expected.into()));
            assert_eq!(hash.measured_bytes, actual);
            assert_eq!(hash.counts.tokens, hash.counts.storage_releases);
            assert!(hash.counts.structural_tokens > 0);
            assert!(hash.counts.structural_tokens < hash.counts.tokens);
            assert!(hash.counts.max_encoded_token <= 256);
            eprintln!(
                "real-limit budgets={budgets:?} actual={actual} visits={} tokens={} structural={} nested={} max_token={} storage_releases={}",
                hash.total_visits(),
                hash.counts.tokens,
                hash.counts.structural_tokens,
                hash.counts.nested_records,
                hash.counts.max_encoded_token,
                hash.counts.storage_releases
            );
            totals.push(hash.total_visits());
            let _ = drain_adapter(
                hash.into_retirement(LivingStateRetirementPolicyV2::ReleaseHandle),
                budgets,
            );
        }
        assert!(totals.iter().all(|total| *total == totals[0]));
        let (out, _, _) = run(
            LivingStateRetirementJobV2::begin_shared(
                source,
                LivingStateRetirementPolicyV2::ReleaseHandle,
            ),
            &[1024],
        );
        assert!(matches!(out, LivingStateRetirementDispositionV2::Drained));
    }
    #[test]
    fn copy_hash_selector_never_traverses_nested_values_and_preserves_json_errors() {
        struct MustNotVisit;
        impl serde::Serialize for MustNotVisit {
            fn serialize<S: serde::Serializer>(&self, _: S) -> Result<S::Ok, S::Error> {
                panic!("nested payload traversed")
            }
        }
        #[derive(serde::Serialize)]
        struct Header {
            blocked_reasons: MustNotVisit,
        }
        let header = Header {
            blocked_reasons: MustNotVisit,
        };
        let mut token = 0;
        let mut children = 0;
        while let Some(selected) = state_hash_token(&header, token, Some(7)).unwrap() {
            if matches!(selected, HashToken::Children(HashChildren::FacilityReasons)) {
                children += 1;
            }
            token += 1;
        }
        assert_eq!(children, 1);
        struct Broken;
        impl serde::Serialize for Broken {
            fn serialize<S: serde::Serializer>(&self, _: S) -> Result<S::Ok, S::Error> {
                Err(serde::ser::Error::custom("deliberate JSON failure"))
            }
        }
        #[derive(serde::Serialize)]
        struct FailsLate {
            prefix: &'static str,
            broken: Broken,
        }
        let value = FailsLate {
            prefix: "encoded before fault",
            broken: Broken,
        };
        assert_eq!(
            super::super::wire::encode_canonical_v2(&value, 1),
            Err(super::super::wire::LivingWireErrorV2::Json)
        );
        let mut hash = LivingStateHashJobV2::begin(Arc::new(LivingGalaxyStateV2::default()));
        hash.byte_limit = 1;
        let error = (0..64)
            .find_map(|token| match state_hash_token(&value, token, None) {
                Ok(Some(HashToken::Bytes(bytes, structural))) => {
                    hash.emit(bytes, structural).unwrap();
                    drop(hash.fragment.take());
                    None
                }
                Err(error) => Some(error),
                _ => None,
            })
            .unwrap();
        assert!(hash.over_limit);
        assert_eq!(
            error,
            LivingDeterministicFaultV2::Wire(super::super::wire::LivingWireErrorV2::Json)
        );
        let _ = drain_adapter(
            hash.into_retirement(LivingStateRetirementPolicyV2::ReleaseHandle),
            &[1],
        );
    }
    #[test]
    fn copy_hash_known_size_copy_buffers_do_not_relocate_populated_records() {
        let mut state = fixture();
        let prototype = state.systems.as_slice()[0].clone();
        state.systems = LivingSortedVecV2::new(
            (0..256)
                .map(|n| LivingSystemV2 {
                    id: id(n),
                    ..prototype.clone()
                })
                .collect(),
        )
        .unwrap();
        state
            .fleets
            .record_at_mut(0)
            .unwrap()
            .order
            .as_mut()
            .unwrap()
            .lane_path = vec![id(99); 2048];
        let source = Arc::new(state);
        let mut copy = LivingStateCopyJobV2::begin(Arc::clone(&source));
        copy.poll(1).unwrap();
        assert_eq!(copy.storage_preparations, 1);
        let systems = copy.candidate.systems.as_slice().as_ptr();
        let mut path = None;
        while copy.poll(1).unwrap() != LivingLifecycleProgressV2::Ready {
            assert_eq!(systems, copy.candidate.systems.as_slice().as_ptr());
            if let Some(CopyNested::Fleet(row, _, _)) = &copy.nested {
                let pointer = row.order.as_ref().unwrap().lane_path.as_ptr();
                if let Some(expected) = path {
                    assert_eq!(pointer, expected);
                }
                path = Some(pointer);
            }
        }
        let candidate = copy.take_ready_result().unwrap();
        assert_eq!(candidate, *source);
        assert_eq!(
            candidate.fleets.as_slice()[0]
                .order
                .as_ref()
                .unwrap()
                .lane_path
                .as_ptr(),
            path.unwrap()
        );
        let _ = drain_adapter(
            copy.into_retirement(LivingStateRetirementPolicyV2::ReleaseHandle),
            &[64],
        );
        let _ = run(LivingStateRetirementJobV2::begin_owned(candidate), &[64]);
    }
    #[test]
    fn copy_hash_absent_order_does_not_count_a_path_storage_preparation() {
        for has_order in [false, true] {
            let mut state = fixture();
            if !has_order {
                state.fleets.record_at_mut(0).unwrap().order = None;
            }
            let mut copy = LivingStateCopyJobV2::begin(Arc::new(state));
            while copy.collection != 10 || !copy.prepared {
                copy.poll(1).unwrap();
            }
            assert!(copy.nested.is_none());
            let before = copy.storage_preparations;
            copy.poll(1).unwrap();
            assert!(matches!(copy.nested, Some(CopyNested::Fleet(_, _, _))));
            assert_eq!(copy.storage_preparations - before, 2 + u64::from(has_order));
            copy.cancel();
            assert!(matches!(
                drain_adapter(
                    copy.into_retirement(LivingStateRetirementPolicyV2::ReleaseHandle),
                    &[1]
                ),
                LivingStateRetirementDispositionV2::Drained
            ));
        }
    }
}

// Counted ownership trees. Only explicitly registered payload types participate;
// LivingKeyedV2 alone never implies that ordinary Drop is a bounded operation.
#[derive(Clone)]
struct PhaseArc<T>(Option<Arc<T>>);
impl<T> From<Arc<T>> for PhaseArc<T> {
    fn from(value: Arc<T>) -> Self {
        Self(Some(value))
    }
}
impl<T> std::ops::Deref for PhaseArc<T> {
    type Target = Arc<T>;
    fn deref(&self) -> &Self::Target {
        self.0.as_ref().expect("live phase owner")
    }
}
impl<T> std::ops::DerefMut for PhaseArc<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.0.as_mut().expect("live phase owner")
    }
}

pub(crate) enum RetirementStep {
    Work,
    Deferred(LivingPhaseDeferredRetirementV2),
    Done,
}
pub(crate) trait RetirementPoll {
    #[cfg(test)]
    fn shared_identity(&self) -> Option<*const ()> {
        None
    }
    fn retirement_step(&mut self) -> RetirementStep;
}
/// A retained exact owner. Ordinary Drop is deliberately not a reclamation API.
#[must_use]
pub(crate) enum LivingPhaseDeferredRetirementV2 {
    Consumed,
    State(LivingDeferredStateRetirementV2),
    Catalog(super::catalog::LivingDeferredCatalogRetirementV2),
    Private(PrivateDeferredRetirementV2),
}
#[must_use]
pub(crate) struct PrivateDeferredRetirementV2 {
    node: Box<dyn RetirementPoll + 'static>,
}
pub(crate) trait RetirementFields<'a>: Sized + 'a {
    const CUSTOM_NODE: bool = false;
    const LEAF: bool = false;
    fn retirement_child(
        &mut self,
        index: usize,
        policy: LivingStateRetirementPolicyV2,
    ) -> Option<Box<dyn RetirementPoll + 'a>>;
    fn retirement_node(
        self,
        policy: LivingStateRetirementPolicyV2,
    ) -> Box<dyn RetirementPoll + 'a> {
        Box::new(RetirementTree::new(self, policy))
    }
}
struct RetirementTree<'a, T> {
    owner: Option<T>,
    entered: bool,
    index: usize,
    child: Option<Box<dyn RetirementPoll + 'a>>,
    policy: LivingStateRetirementPolicyV2,
}
impl<'a, T> RetirementTree<'a, T> {
    fn new(owner: T, policy: LivingStateRetirementPolicyV2) -> Self {
        Self {
            owner: Some(owner),
            entered: false,
            index: 0,
            child: None,
            policy,
        }
    }
}
impl<'a, T: RetirementFields<'a>> RetirementPoll for RetirementTree<'a, T> {
    fn retirement_step(&mut self) -> RetirementStep {
        if let Some(child) = &mut self.child {
            return match child.retirement_step() {
                RetirementStep::Done => {
                    // The trait-object Box is retained until its payload is empty.
                    drop(self.child.take());
                    self.index += 1;
                    RetirementStep::Work
                }
                other => other,
            };
        }
        if T::CUSTOM_NODE && !self.entered {
            self.entered = true;
            self.child = Some(
                self.owner
                    .take()
                    .expect("original custom root")
                    .retirement_node(self.policy),
            );
            return RetirementStep::Work;
        }
        let Some(owner) = &mut self.owner else {
            return RetirementStep::Done;
        };
        if let Some(child) = owner.retirement_child(self.index, self.policy) {
            self.child = Some(child);
        } else {
            // Every registered child was drained before the original root/Box.
            drop(self.owner.take());
        }
        RetirementStep::Work
    }
}
#[must_use]
pub(crate) struct LivingPhaseRetirementJobV2<'a, T> {
    root: RetirementTree<'a, T>,
    deferred: Option<LivingPhaseDeferredRetirementV2>,
    ready: bool,
    last_poll_visits: usize,
    total_visits: u64,
}
#[allow(dead_code, private_bounds)]
impl<'a, T: RetirementFields<'a>> LivingPhaseRetirementJobV2<'a, T> {
    pub(crate) fn begin(owner: T, policy: LivingStateRetirementPolicyV2) -> Self {
        Self {
            root: RetirementTree::new(owner, policy),
            deferred: None,
            ready: false,
            last_poll_visits: 0,
            total_visits: 0,
        }
    }
    pub(crate) fn poll(
        &mut self,
        budget: usize,
    ) -> Result<LivingLifecycleProgressV2, LivingLifecycleJobErrorV2> {
        self.last_poll_visits = 0;
        if budget == 0 || budget > LIVING_MAX_PHASE_POLL_VISITS_V2 {
            return Err(LivingLifecycleJobErrorV2::Budget);
        }
        while !self.ready && self.deferred.is_none() && self.last_poll_visits < budget {
            match self.root.retirement_step() {
                RetirementStep::Done => {
                    self.ready = true;
                    break;
                }
                RetirementStep::Deferred(owner) => self.deferred = Some(owner),
                RetirementStep::Work => {}
            }
            self.last_poll_visits += 1;
            self.total_visits += 1;
        }
        Ok(if self.ready {
            LivingLifecycleProgressV2::Ready
        } else {
            LivingLifecycleProgressV2::Pending
        })
    }
    pub(crate) fn take_deferred_owner(&mut self) -> Option<LivingPhaseDeferredRetirementV2> {
        self.deferred.take()
    }
    pub(crate) fn last_poll_visits(&self) -> usize {
        self.last_poll_visits
    }
    pub(crate) fn total_visits(&self) -> u64 {
        self.total_visits
    }
}
pub(crate) fn retirement_node<'a, T: RetirementFields<'a>>(
    value: T,
    policy: LivingStateRetirementPolicyV2,
) -> Box<dyn RetirementPoll + 'a> {
    value.retirement_node(policy)
}

macro_rules! retirement_leaves {
    ($($ty:ty),* $(,)?) => {$ (
        impl<'a> RetirementFields<'a> for $ty {
            const LEAF:bool=true;
            fn retirement_child(&mut self, _: usize, _: LivingStateRetirementPolicyV2) -> Option<Box<dyn RetirementPoll + 'a>> { None }
        }
    )* };
}
retirement_leaves!(
    (),
    u8,
    u16,
    u32,
    u64,
    usize,
    i32,
    i64,
    bool,
    String,
    &'static str,
    LivingEntityIdV2,
    LivingTickV2,
    LivingResourceV2,
    super::catalog::LivingSlugV2,
    super::model::LivingSystemV2,
    super::model::LivingStarV2,
    super::model::LivingWorldV2,
    super::model::LivingLaneV2,
    super::model::LivingCivilizationV2,
    super::model::LivingDepositV2,
    super::model::LivingConstructionJobV2,
    super::model::LivingHullJobV2,
    super::model::LivingHullV2,
    super::model::LivingFreightRouteV2,
    super::model::LivingShipmentV2,
    super::model::LivingRelationV2,
    super::model::LivingAgreementV2,
    super::model::LivingWarV2,
    super::model::LivingObservedFacilityV2,
    super::model::LivingObservedHullV2,
    super::model::LivingHazardV2,
    super::model::LivingSettlementClaimV2,
    super::model::LivingOccupationV2,
    super::model::LivingBlockedReasonV2,
    LivingEventKindV2,
    CombatPiece,
    CargoCursor,
    super::receipt::LivingEventPayloadV2,
    super::decision::LivingDecisionReasonV2,
    super::event_facts::LivingEventResourceFactV2,
    super::event_facts::LivingEventResourceRoleV2,
    super::event_facts::LivingCorridorRestrictionV2,
    super::simulation::LivingCreatorRecoveryCandidateV2
);
impl<'a, T: RetirementFields<'a>> RetirementFields<'a> for Option<T> {
    const LEAF: bool = T::LEAF;
    fn retirement_child(
        &mut self,
        _: usize,
        policy: LivingStateRetirementPolicyV2,
    ) -> Option<Box<dyn RetirementPoll + 'a>> {
        self.take().map(|v| retirement_node(v, policy))
    }
}
impl<'a, T: RetirementFields<'a>> RetirementFields<'a> for Vec<T> {
    fn retirement_child(
        &mut self,
        _: usize,
        policy: LivingStateRetirementPolicyV2,
    ) -> Option<Box<dyn RetirementPoll + 'a>> {
        self.pop().map(|v| retirement_node(v, policy))
    }
}
impl<'a, T: RetirementFields<'a>> RetirementFields<'a> for IntoIter<T> {
    fn retirement_child(
        &mut self,
        _: usize,
        policy: LivingStateRetirementPolicyV2,
    ) -> Option<Box<dyn RetirementPoll + 'a>> {
        self.next().map(|v| retirement_node(v, policy))
    }
}
impl<'a, T: RetirementFields<'a>> RetirementFields<'a> for std::collections::VecDeque<T> {
    fn retirement_child(
        &mut self,
        _: usize,
        policy: LivingStateRetirementPolicyV2,
    ) -> Option<Box<dyn RetirementPoll + 'a>> {
        self.pop_front().map(|v| retirement_node(v, policy))
    }
}
impl<'a, K: RetirementFields<'a> + Ord, V: RetirementFields<'a>> RetirementFields<'a>
    for BTreeMap<K, V>
{
    fn retirement_child(
        &mut self,
        _: usize,
        policy: LivingStateRetirementPolicyV2,
    ) -> Option<Box<dyn RetirementPoll + 'a>> {
        self.pop_first().map(|v| retirement_node(v, policy))
    }
}
impl<'a, T: RetirementFields<'a> + Ord> RetirementFields<'a> for BTreeSet<T> {
    fn retirement_child(
        &mut self,
        _: usize,
        policy: LivingStateRetirementPolicyV2,
    ) -> Option<Box<dyn RetirementPoll + 'a>> {
        self.pop_first().map(|v| retirement_node(v, policy))
    }
}
impl<'a, K: RetirementFields<'a>, V: RetirementFields<'a>> RetirementFields<'a>
    for std::collections::btree_map::IntoIter<K, V>
{
    fn retirement_child(
        &mut self,
        _: usize,
        policy: LivingStateRetirementPolicyV2,
    ) -> Option<Box<dyn RetirementPoll + 'a>> {
        self.next().map(|v| retirement_node(v, policy))
    }
}
impl<'a, T: RetirementFields<'a> + LivingKeyedV2> RetirementFields<'a> for LivingSortedVecV2<T> {
    fn retirement_child(
        &mut self,
        index: usize,
        policy: LivingStateRetirementPolicyV2,
    ) -> Option<Box<dyn RetirementPoll + 'a>> {
        (index == 0).then(|| retirement_node(std::mem::take(self).into_vec(), policy))
    }
}
// Box support is explicit: custom payload adapters must preserve the original
// allocation rather than accidentally delegating to a placeholder child method.
macro_rules! boxed_phase_retirement {
    ($($ty:ty),* $(,)?)=>{$(
        impl<'a> RetirementFields<'a> for Box<$ty> {
            fn retirement_child(&mut self,index:usize,policy:LivingStateRetirementPolicyV2)->Option<Box<dyn RetirementPoll+'a>> {
                (**self).retirement_child(index,policy)
            }
        }
    )*};
}
boxed_phase_retirement!(
    LivingLifecycleJobV2,
    LivingArrivalJobV2,
    LivingScheduledPhasesJobV2,
    LivingRecipeJobV2,
    LivingRepairJobV2<'a>,
    LivingObservationJobV2,
    LivingCombatJobV2<'a>,
    LivingRetreatJobV2<'a>,
    LivingOccupationJobV2<'a>,
    LivingSettlementJobV2<'a>,
    LivingConflictJobV2<'a>,
    LivingDiplomacyJobV2,
    LivingDispatchJobV2<'a>,
    LivingProductionJobV2<'a>,
    RetreatPathJob
);
impl<'a> RetirementFields<'a> for Box<LivingGalaxyStateV2> {
    const CUSTOM_NODE: bool = true;
    fn retirement_child(
        &mut self,
        _: usize,
        _: LivingStateRetirementPolicyV2,
    ) -> Option<Box<dyn RetirementPoll + 'a>> {
        unreachable!("boxed state uses reviewed drainer")
    }
    fn retirement_node(self, _: LivingStateRetirementPolicyV2) -> Box<dyn RetirementPoll + 'a> {
        Box::new(StateRetirementNode {
            job: LivingStateRetirementJobV2::begin_boxed(self),
            complete: false,
        })
    }
}

// Tuple fields are moved once without requiring Default or reconstructing rows.
struct RetiredPair<A, B>(Option<A>, Option<B>);
impl<'a, A: RetirementFields<'a>, B: RetirementFields<'a>> RetirementFields<'a> for (A, B) {
    const LEAF: bool = A::LEAF && B::LEAF;
    const CUSTOM_NODE: bool = true;
    fn retirement_child(
        &mut self,
        _: usize,
        _: LivingStateRetirementPolicyV2,
    ) -> Option<Box<dyn RetirementPoll + 'a>> {
        unreachable!("tuple enters its typed owner")
    }
    fn retirement_node(
        self,
        policy: LivingStateRetirementPolicyV2,
    ) -> Box<dyn RetirementPoll + 'a> {
        if Self::LEAF {
            Box::new(RetirementLeaf(Some(self)))
        } else {
            retirement_node(RetiredPair(Some(self.0), Some(self.1)), policy)
        }
    }
}
impl<'a, A: RetirementFields<'a>, B: RetirementFields<'a>> RetirementFields<'a>
    for RetiredPair<A, B>
{
    fn retirement_child(
        &mut self,
        index: usize,
        policy: LivingStateRetirementPolicyV2,
    ) -> Option<Box<dyn RetirementPoll + 'a>> {
        match index {
            0 => Some(retirement_node(self.0.take(), policy)),
            1 => Some(retirement_node(self.1.take(), policy)),
            _ => None,
        }
    }
}
impl<'a, A: RetirementFields<'a>, B: RetirementFields<'a>, C: RetirementFields<'a>>
    RetirementFields<'a> for (A, B, C)
{
    const CUSTOM_NODE: bool = true;
    fn retirement_child(
        &mut self,
        _: usize,
        _: LivingStateRetirementPolicyV2,
    ) -> Option<Box<dyn RetirementPoll + 'a>> {
        unreachable!("tuple enters its typed owner")
    }
    fn retirement_node(self, p: LivingStateRetirementPolicyV2) -> Box<dyn RetirementPoll + 'a> {
        retirement_node((self.0, (self.1, self.2)), p)
    }
}
impl<
    'a,
    A: RetirementFields<'a>,
    B: RetirementFields<'a>,
    C: RetirementFields<'a>,
    D: RetirementFields<'a>,
> RetirementFields<'a> for (A, B, C, D)
{
    const CUSTOM_NODE: bool = true;
    fn retirement_child(
        &mut self,
        _: usize,
        _: LivingStateRetirementPolicyV2,
    ) -> Option<Box<dyn RetirementPoll + 'a>> {
        unreachable!("tuple enters its typed owner")
    }
    fn retirement_node(self, p: LivingStateRetirementPolicyV2) -> Box<dyn RetirementPoll + 'a> {
        retirement_node((self.0, (self.1, (self.2, self.3))), p)
    }
}

struct StateRetirementNode {
    job: LivingStateRetirementJobV2,
    complete: bool,
}
impl RetirementPoll for StateRetirementNode {
    fn retirement_step(&mut self) -> RetirementStep {
        if self.complete {
            return RetirementStep::Done;
        }
        let before = self.job.total_visits();
        let progress = self.job.poll(1).expect("fixed valid retirement budget");
        debug_assert_eq!(
            self.job.total_visits() - before,
            self.job.last_poll_visits() as u64
        );
        debug_assert!(self.job.last_poll_visits() <= 1);
        if progress == LivingLifecycleProgressV2::Ready {
            self.complete = true;
            return match self.job.finish().expect("ready state owner") {
                LivingStateRetirementDispositionV2::Deferred(ticket) => {
                    RetirementStep::Deferred(LivingPhaseDeferredRetirementV2::State(ticket))
                }
                _ => RetirementStep::Work,
            };
        }
        RetirementStep::Work
    }
}
struct CatalogRetirementNode {
    job: super::catalog::LivingCatalogRetirementJobV2,
    complete: bool,
}
impl RetirementPoll for CatalogRetirementNode {
    fn retirement_step(&mut self) -> RetirementStep {
        if self.complete {
            return RetirementStep::Done;
        }
        let progress = self.job.poll(1).expect("fixed valid retirement budget");
        if progress == LivingLifecycleProgressV2::Ready {
            self.complete = true;
            return match self.job.finish().expect("ready catalog owner") {
                super::catalog::LivingCatalogRetirementDispositionV2::Deferred(ticket) => {
                    RetirementStep::Deferred(LivingPhaseDeferredRetirementV2::Catalog(ticket))
                }
                _ => RetirementStep::Work,
            };
        }
        RetirementStep::Work
    }
}
impl<'a> RetirementFields<'a> for LivingGalaxyStateV2 {
    const CUSTOM_NODE: bool = true;
    fn retirement_child(
        &mut self,
        _: usize,
        _: LivingStateRetirementPolicyV2,
    ) -> Option<Box<dyn RetirementPoll + 'a>> {
        unreachable!("state uses reviewed drainer")
    }
    fn retirement_node(self, _: LivingStateRetirementPolicyV2) -> Box<dyn RetirementPoll + 'a> {
        Box::new(StateRetirementNode {
            job: LivingStateRetirementJobV2::begin_owned(self),
            complete: false,
        })
    }
}
/// Atomic ownership resolution only. The caller counts this transition and
/// retains/drains every returned owner; shared release is not reclamation.
#[must_use]
pub(crate) enum LivingArcOwnerResolutionV2<T> {
    Owned(T),
    SharedReleased,
    Deferred(Arc<T>),
}
pub(crate) fn resolve_living_retirement_arc_v2<T>(
    source: Arc<T>,
    policy: LivingStateRetirementPolicyV2,
) -> LivingArcOwnerResolutionV2<T> {
    match policy {
        LivingStateRetirementPolicyV2::ReleaseHandle => match Arc::into_inner(source) {
            Some(value) => LivingArcOwnerResolutionV2::Owned(value),
            None => LivingArcOwnerResolutionV2::SharedReleased,
        },
        LivingStateRetirementPolicyV2::DeferUntilUnique => match Arc::try_unwrap(source) {
            Ok(value) => LivingArcOwnerResolutionV2::Owned(value),
            Err(source) => LivingArcOwnerResolutionV2::Deferred(source),
        },
    }
}

struct ArcRetirementNode<T> {
    source: Option<Arc<T>>,
    child: Option<Box<dyn RetirementPoll + 'static>>,
    policy: LivingStateRetirementPolicyV2,
}
impl<T: RetirementFields<'static>> RetirementPoll for ArcRetirementNode<T> {
    #[cfg(test)]
    fn shared_identity(&self) -> Option<*const ()> {
        self.source
            .as_ref()
            .map(|source| Arc::as_ptr(source).cast())
    }

    fn retirement_step(&mut self) -> RetirementStep {
        if let Some(child) = &mut self.child {
            return match child.retirement_step() {
                RetirementStep::Done => {
                    drop(self.child.take());
                    RetirementStep::Work
                }
                step => step,
            };
        }
        let Some(source) = self.source.take() else {
            return RetirementStep::Done;
        };
        let value = match resolve_living_retirement_arc_v2(source, self.policy) {
            LivingArcOwnerResolutionV2::Owned(value) => Some(value),
            LivingArcOwnerResolutionV2::SharedReleased => None,
            LivingArcOwnerResolutionV2::Deferred(source) => {
                return RetirementStep::Deferred(LivingPhaseDeferredRetirementV2::Private(
                    PrivateDeferredRetirementV2 {
                        node: Box::new(ArcRetirementNode {
                            source: Some(source),
                            child: None,
                            policy: self.policy,
                        }),
                    },
                ));
            }
        };
        if let Some(value) = value {
            self.child = Some(retirement_node(value, self.policy));
        }
        RetirementStep::Work
    }
}
impl<'a, T: RetirementFields<'static>> RetirementFields<'a> for Arc<T> {
    const CUSTOM_NODE: bool = true;
    fn retirement_child(
        &mut self,
        _: usize,
        _: LivingStateRetirementPolicyV2,
    ) -> Option<Box<dyn RetirementPoll + 'a>> {
        unreachable!("Arc uses atomic ownership adapter")
    }
    fn retirement_node(
        self,
        policy: LivingStateRetirementPolicyV2,
    ) -> Box<dyn RetirementPoll + 'a> {
        Box::new(ArcRetirementNode {
            source: Some(self),
            child: None,
            policy,
        })
    }
}
impl<'a, T: RetirementFields<'static>> RetirementFields<'a> for PhaseArc<T> {
    fn retirement_child(
        &mut self,
        index: usize,
        policy: LivingStateRetirementPolicyV2,
    ) -> Option<Box<dyn RetirementPoll + 'a>> {
        (index == 0).then(|| retirement_node(self.0.take(), policy))
    }
}
impl<'a> RetirementFields<'a> for LivingPhaseCatalogV2<'a> {
    const CUSTOM_NODE: bool = true;
    fn retirement_child(
        &mut self,
        _: usize,
        _: LivingStateRetirementPolicyV2,
    ) -> Option<Box<dyn RetirementPoll + 'a>> {
        unreachable!("catalog root moves once")
    }
    fn retirement_node(
        self,
        policy: LivingStateRetirementPolicyV2,
    ) -> Box<dyn RetirementPoll + 'a> {
        match self {
            Self::Owned(source) => Box::new(CatalogRetirementNode {
                job: super::catalog::LivingCatalogRetirementJobV2::begin_shared(source, policy),
                complete: false,
            }),
            Self::Borrowed(_) => retirement_node((), policy),
        }
    }
}
struct PhaseCatalogSlot<'a>(Option<LivingPhaseCatalogV2<'a>>);
impl<'a> std::ops::Deref for PhaseCatalogSlot<'a> {
    type Target = LivingPhaseCatalogV2<'a>;
    fn deref(&self) -> &Self::Target {
        self.0.as_ref().expect("live catalog slot")
    }
}

struct RowRetirementNode {
    parent: Option<StateRetirementParent>,
    children: Option<StateRetirementChildren>,
    index: usize,
}
impl RetirementPoll for RowRetirementNode {
    fn retirement_step(&mut self) -> RetirementStep {
        if let Some(children) = &mut self.children {
            if children.remaining() != 0 {
                children.retire_record();
            } else {
                drop(self.children.take());
                self.index += 1;
            }
            return RetirementStep::Work;
        }
        let Some(parent) = &mut self.parent else {
            return RetirementStep::Done;
        };
        while self.index < parent.child_count() {
            if let Some(child) = parent.child(self.index) {
                self.children = Some(child);
                return RetirementStep::Work;
            }
            self.index += 1;
        }
        drop(self.parent.take());
        RetirementStep::Work
    }
}
macro_rules! retirement_rows {
    ($($ty:ty => $variant:ident),* $(,)?)=>{$(
        impl<'a> RetirementFields<'a> for $ty {
            const CUSTOM_NODE: bool = true;
            fn retirement_child(&mut self,_:usize,_:LivingStateRetirementPolicyV2)->Option<Box<dyn RetirementPoll+'a>> { unreachable!("row uses shared state child cursor") }
            fn retirement_node(self,_:LivingStateRetirementPolicyV2)->Box<dyn RetirementPoll+'a> {
                Box::new(RowRetirementNode { parent:Some(StateRetirementParent::$variant(self)),children:None,index:0 })
            }
        }
    )*};
}
retirement_rows!(LivingColonyV2=>Colony, LivingFacilityV2=>Facility, LivingFleetV2=>Fleet, LivingObservationV2=>Observation);
impl RetirementPoll for LivingCopyHashRetirementJobV2 {
    fn retirement_step(&mut self) -> RetirementStep {
        if self.delivered {
            return RetirementStep::Done;
        }
        let before = self.total_visits();
        let progress = self.poll(1).expect("fixed valid budget");
        debug_assert_eq!(self.total_visits() - before, self.last_poll_visits() as u64);
        debug_assert!(self.last_poll_visits() <= 1);
        if progress == LivingLifecycleProgressV2::Ready {
            return match self.finish().expect("ready copy/hash retirement") {
                LivingStateRetirementDispositionV2::Deferred(ticket) => {
                    RetirementStep::Deferred(LivingPhaseDeferredRetirementV2::State(ticket))
                }
                _ => RetirementStep::Work,
            };
        }
        RetirementStep::Work
    }
}
impl<'a> RetirementFields<'a> for LivingStateHashJobV2 {
    const CUSTOM_NODE: bool = true;
    fn retirement_child(
        &mut self,
        _: usize,
        _: LivingStateRetirementPolicyV2,
    ) -> Option<Box<dyn RetirementPoll + 'a>> {
        unreachable!("hash uses reviewed adapter")
    }
    fn retirement_node(
        self,
        policy: LivingStateRetirementPolicyV2,
    ) -> Box<dyn RetirementPoll + 'a> {
        Box::new(self.into_retirement(policy))
    }
}
impl<'a> RetirementFields<'a> for LivingPhaseDeferredRetirementV2 {
    fn retirement_child(
        &mut self,
        index: usize,
        policy: LivingStateRetirementPolicyV2,
    ) -> Option<Box<dyn RetirementPoll + 'a>> {
        (index == 0).then(|| std::mem::replace(self, Self::Consumed).retirement_node(policy))
    }
    fn retirement_node(self, _: LivingStateRetirementPolicyV2) -> Box<dyn RetirementPoll + 'a> {
        match self {
            Self::Consumed => retirement_node((), LivingStateRetirementPolicyV2::ReleaseHandle),
            Self::State(ticket) => Box::new(StateRetirementNode {
                job: LivingStateRetirementJobV2::resume(ticket),
                complete: false,
            }),
            Self::Catalog(ticket) => Box::new(CatalogRetirementNode {
                job: super::catalog::LivingCatalogRetirementJobV2::resume(ticket),
                complete: false,
            }),
            Self::Private(owner) => owner.node,
        }
    }
}

impl<'a> RetirementFields<'a> for LivingLifecycleJobV2 {
    fn retirement_child(
        &mut self,
        index: usize,
        policy: LivingStateRetirementPolicyV2,
    ) -> Option<Box<dyn RetirementPoll + 'a>> {
        match index {
            0 => Some(retirement_node(std::mem::take(&mut self.state), policy)),
            1 => Some(retirement_node(std::mem::take(&mut self.events), policy)),
            2 => Some(retirement_node(
                std::mem::take(&mut self.agreements),
                policy,
            )),
            3 => Some(retirement_node(std::mem::take(&mut self.wars), policy)),
            4 => Some(retirement_node(
                std::mem::take(&mut self.occupations),
                policy,
            )),
            5 => Some(retirement_node(
                std::mem::take(&mut self.agreement_rows),
                policy,
            )),
            6 => Some(retirement_node(
                std::mem::take(&mut self.agreement_output),
                policy,
            )),
            7 => Some(retirement_node(
                std::mem::take(&mut self.ended_pairs),
                policy,
            )),
            8 => Some(retirement_node(std::mem::take(&mut self.owners), policy)),
            _ => None,
        }
    }
}

#[allow(dead_code)]
impl<'a> LivingLifecycleJobV2 {
    pub(crate) fn into_phase_retirement(
        self,
        policy: LivingStateRetirementPolicyV2,
    ) -> LivingPhaseRetirementJobV2<'a, Self> {
        LivingPhaseRetirementJobV2::begin(self, policy)
    }
}

impl<'a> RetirementFields<'a> for LivingArrivalJobV2 {
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
            1 => Some(retirement_node(self.active_fleet.take(), policy)),
            2 => Some(retirement_node(std::mem::take(&mut self.state), policy)),
            3 => Some(retirement_node(std::mem::take(&mut self.events), policy)),
            4 => Some(retirement_node(std::mem::take(&mut self.fleets), policy)),
            5 => Some(retirement_node(std::mem::take(&mut self.shipments), policy)),
            6 => Some(retirement_node(std::mem::take(&mut self.relations), policy)),
            7 => Some(retirement_node(
                std::mem::take(&mut self.world_meta),
                policy,
            )),
            8 => Some(retirement_node(std::mem::take(&mut self.graph), policy)),
            9 => Some(retirement_node(
                std::mem::take(&mut self.active_wars),
                policy,
            )),
            10 => Some(retirement_node(
                std::mem::take(&mut self.relation_rows),
                policy,
            )),
            11 => Some(retirement_node(
                std::mem::take(&mut self.relation_output),
                policy,
            )),
            12 => Some(retirement_node(std::mem::take(&mut self.arrivals), policy)),
            13 => Some(retirement_node(std::mem::take(&mut self.cargo), policy)),
            14 => Some(retirement_node(std::mem::take(&mut self.search), policy)),
            _ => None,
        }
    }
}

#[allow(dead_code)]
impl<'a> LivingArrivalJobV2 {
    pub(crate) fn into_phase_retirement(
        self,
        policy: LivingStateRetirementPolicyV2,
    ) -> LivingPhaseRetirementJobV2<'a, Self> {
        LivingPhaseRetirementJobV2::begin(self, policy)
    }
}

impl<'a> RetirementFields<'a> for LivingRecipeJobV2 {
    fn retirement_child(
        &mut self,
        index: usize,
        policy: LivingStateRetirementPolicyV2,
    ) -> Option<Box<dyn RetirementPoll + 'a>> {
        match index {
            0 => Some(retirement_node(self.inputs.0.take(), policy)),
            1 => Some(retirement_node(self.outputs.0.take(), policy)),
            2 => Some(retirement_node(
                std::mem::take(&mut self.pending_reason),
                policy,
            )),
            3 => Some(retirement_node(std::mem::take(&mut self.reasons), policy)),
            4 => Some(retirement_node(
                std::mem::take(&mut self.reason_output),
                policy,
            )),
            _ => None,
        }
    }
}

#[allow(dead_code)]
impl<'a> LivingRecipeJobV2 {
    pub(crate) fn into_phase_retirement(
        self,
        policy: LivingStateRetirementPolicyV2,
    ) -> LivingPhaseRetirementJobV2<'a, Self> {
        LivingPhaseRetirementJobV2::begin(self, policy)
    }
}

impl<'a> RetirementFields<'a> for LivingRepairJobV2<'a> {
    fn retirement_child(
        &mut self,
        index: usize,
        policy: LivingStateRetirementPolicyV2,
    ) -> Option<Box<dyn RetirementPoll + 'a>> {
        match index {
            0 => Some(retirement_node(self.catalog.0.take(), policy)),
            1 => Some(retirement_node(std::mem::take(&mut self.state), policy)),
            2 => Some(retirement_node(std::mem::take(&mut self.source), policy)),
            3 => Some(retirement_node(
                std::mem::take(&mut self.world_index),
                policy,
            )),
            4 => Some(retirement_node(
                std::mem::take(&mut self.hit_points),
                policy,
            )),
            5 => Some(retirement_node(std::mem::take(&mut self.fleet), policy)),
            6 => Some(retirement_node(
                std::mem::take(&mut self.hull_source),
                policy,
            )),
            7 => Some(retirement_node(
                std::mem::take(&mut self.hull_output),
                policy,
            )),
            8 => Some(retirement_node(std::mem::take(&mut self.hull), policy)),
            _ => None,
        }
    }
}

#[allow(dead_code)]
impl<'a> LivingRepairJobV2<'a> {
    pub(crate) fn into_phase_retirement(
        self,
        policy: LivingStateRetirementPolicyV2,
    ) -> LivingPhaseRetirementJobV2<'a, Self> {
        LivingPhaseRetirementJobV2::begin(self, policy)
    }
}

impl<'a> RetirementFields<'a> for LivingObservationJobV2 {
    fn retirement_child(
        &mut self,
        index: usize,
        policy: LivingStateRetirementPolicyV2,
    ) -> Option<Box<dyn RetirementPoll + 'a>> {
        match index {
            0 => Some(retirement_node(std::mem::take(&mut self.state), policy)),
            1 => Some(retirement_node(std::mem::take(&mut self.visible), policy)),
            2 => Some(retirement_node(
                std::mem::take(&mut self.observation),
                policy,
            )),
            3 => Some(retirement_node(
                std::mem::take(&mut self.facilities),
                policy,
            )),
            4 => Some(retirement_node(std::mem::take(&mut self.hulls), policy)),
            5 => Some(retirement_node(
                std::mem::take(&mut self.remembered),
                policy,
            )),
            6 => Some(retirement_node(
                std::mem::take(&mut self.observations),
                policy,
            )),
            7 => Some(retirement_node(
                std::mem::take(&mut self.retired_facilities),
                policy,
            )),
            8 => Some(retirement_node(
                std::mem::take(&mut self.retired_hulls),
                policy,
            )),
            _ => None,
        }
    }
}

#[allow(dead_code)]
impl<'a> LivingObservationJobV2 {
    pub(crate) fn into_phase_retirement(
        self,
        policy: LivingStateRetirementPolicyV2,
    ) -> LivingPhaseRetirementJobV2<'a, Self> {
        LivingPhaseRetirementJobV2::begin(self, policy)
    }
}

impl<'a> RetirementFields<'a> for LivingCombatJobV2<'a> {
    fn retirement_child(
        &mut self,
        index: usize,
        policy: LivingStateRetirementPolicyV2,
    ) -> Option<Box<dyn RetirementPoll + 'a>> {
        match index {
            0 => Some(retirement_node(self.catalog.0.take(), policy)),
            1 => Some(retirement_node(std::mem::take(&mut self.state), policy)),
            2 => Some(retirement_node(std::mem::take(&mut self.events), policy)),
            3 => Some(retirement_node(std::mem::take(&mut self.target), policy)),
            4 => Some(retirement_node(std::mem::take(&mut self.pieces), policy)),
            5 => Some(retirement_node(std::mem::take(&mut self.wars), policy)),
            6 => Some(retirement_node(
                std::mem::take(&mut self.hull_fleets),
                policy,
            )),
            7 => Some(retirement_node(
                std::mem::take(&mut self.harm_index),
                policy,
            )),
            8 => Some(retirement_node(std::mem::take(&mut self.harm), policy)),
            9 => Some(retirement_node(
                std::mem::take(&mut self.facilities),
                policy,
            )),
            10 => Some(retirement_node(std::mem::take(&mut self.fleets), policy)),
            11 => Some(retirement_node(
                std::mem::take(&mut self.dead_reasons),
                policy,
            )),
            12 => Some(retirement_node(std::mem::take(&mut self.fleet), policy)),
            13 => Some(retirement_node(std::mem::take(&mut self.hulls), policy)),
            _ => None,
        }
    }
}

#[allow(dead_code)]
impl<'a> LivingCombatJobV2<'a> {
    pub(crate) fn into_phase_retirement(
        self,
        policy: LivingStateRetirementPolicyV2,
    ) -> LivingPhaseRetirementJobV2<'a, Self> {
        LivingPhaseRetirementJobV2::begin(self, policy)
    }
}

impl<'a> RetirementFields<'a> for RetreatPathJob {
    fn retirement_child(
        &mut self,
        index: usize,
        policy: LivingStateRetirementPolicyV2,
    ) -> Option<Box<dyn RetirementPoll + 'a>> {
        match index {
            0 => Some(retirement_node(self.graph.0.take(), policy)),
            1 => Some(retirement_node(std::mem::take(&mut self.best), policy)),
            2 => Some(retirement_node(std::mem::take(&mut self.copied), policy)),
            3 => Some(retirement_node(std::mem::take(&mut self.retired), policy)),
            4 => Some(retirement_node(std::mem::take(&mut self.result), policy)),
            _ => None,
        }
    }
}

#[allow(dead_code)]
impl<'a> RetreatPathJob {
    pub(crate) fn into_phase_retirement(
        self,
        policy: LivingStateRetirementPolicyV2,
    ) -> LivingPhaseRetirementJobV2<'a, Self> {
        LivingPhaseRetirementJobV2::begin(self, policy)
    }
}

impl<'a> RetirementFields<'a> for LivingRetreatJobV2<'a> {
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
            4 => Some(retirement_node(std::mem::take(&mut self.metadata), policy)),
            5 => Some(retirement_node(std::mem::take(&mut self.wars), policy)),
            6 => Some(retirement_node(std::mem::take(&mut self.colonies), policy)),
            7 => Some(retirement_node(std::mem::take(&mut self.search), policy)),
            8 => Some(retirement_node(self.graph.0.take(), policy)),
            9 => Some(retirement_node(std::mem::take(&mut self.winner), policy)),
            10 => Some(retirement_node(
                std::mem::take(&mut self.retired_path),
                policy,
            )),
            _ => None,
        }
    }
}

#[allow(dead_code)]
impl<'a> LivingRetreatJobV2<'a> {
    pub(crate) fn into_phase_retirement(
        self,
        policy: LivingStateRetirementPolicyV2,
    ) -> LivingPhaseRetirementJobV2<'a, Self> {
        LivingPhaseRetirementJobV2::begin(self, policy)
    }
}

impl<'a> RetirementFields<'a> for LivingOccupationJobV2<'a> {
    fn retirement_child(
        &mut self,
        index: usize,
        policy: LivingStateRetirementPolicyV2,
    ) -> Option<Box<dyn RetirementPoll + 'a>> {
        match index {
            0 => Some(retirement_node(self.catalog.0.take(), policy)),
            1 => Some(retirement_node(std::mem::take(&mut self.state), policy)),
            2 => Some(retirement_node(std::mem::take(&mut self.events), policy)),
            3 => Some(retirement_node(std::mem::take(&mut self.snapshot), policy)),
            4 => Some(retirement_node(std::mem::take(&mut self.wars), policy)),
            5 => Some(retirement_node(std::mem::take(&mut self.escorts), policy)),
            6 => Some(retirement_node(
                std::mem::take(&mut self.living_escorts),
                policy,
            )),
            7 => Some(retirement_node(std::mem::take(&mut self.batteries), policy)),
            8 => Some(retirement_node(
                std::mem::take(&mut self.occupations),
                policy,
            )),
            9 => Some(retirement_node(std::mem::take(&mut self.relations), policy)),
            10 => Some(retirement_node(
                std::mem::take(&mut self.occupation_source),
                policy,
            )),
            11 => Some(retirement_node(
                std::mem::take(&mut self.relation_source),
                policy,
            )),
            12 => Some(retirement_node(
                std::mem::take(&mut self.construction),
                policy,
            )),
            13 => Some(retirement_node(std::mem::take(&mut self.hull_jobs), policy)),
            _ => None,
        }
    }
}

#[allow(dead_code)]
impl<'a> LivingOccupationJobV2<'a> {
    pub(crate) fn into_phase_retirement(
        self,
        policy: LivingStateRetirementPolicyV2,
    ) -> LivingPhaseRetirementJobV2<'a, Self> {
        LivingPhaseRetirementJobV2::begin(self, policy)
    }
}

impl<'a> RetirementFields<'a> for LivingSettlementJobV2<'a> {
    fn retirement_child(
        &mut self,
        index: usize,
        policy: LivingStateRetirementPolicyV2,
    ) -> Option<Box<dyn RetirementPoll + 'a>> {
        match index {
            0 => Some(retirement_node(self.catalog.0.take(), policy)),
            1 => Some(retirement_node(std::mem::take(&mut self.state), policy)),
            2 => Some(retirement_node(std::mem::take(&mut self.events), policy)),
            3 => Some(retirement_node(std::mem::take(&mut self.arrivals), policy)),
            4 => Some(retirement_node(std::mem::take(&mut self.armed), policy)),
            5 => Some(retirement_node(std::mem::take(&mut self.docked), policy)),
            6 => Some(retirement_node(
                std::mem::take(&mut self.fleet_arks),
                policy,
            )),
            7 => Some(retirement_node(std::mem::take(&mut self.arks), policy)),
            8 => Some(retirement_node(std::mem::take(&mut self.claims), policy)),
            9 => Some(retirement_node(std::mem::take(&mut self.colonies), policy)),
            10 => Some(retirement_node(
                std::mem::take(&mut self.claim_source),
                policy,
            )),
            11 => Some(retirement_node(
                std::mem::take(&mut self.colony_source),
                policy,
            )),
            12 => Some(retirement_node(
                std::mem::take(&mut self.fleet_source),
                policy,
            )),
            13 => Some(retirement_node(std::mem::take(&mut self.fleet), policy)),
            14 => Some(retirement_node(std::mem::take(&mut self.hulls), policy)),
            15 => Some(retirement_node(
                std::mem::take(&mut self.retired_path),
                policy,
            )),
            16 => Some(retirement_node(
                std::mem::take(&mut self.retired_reasons),
                policy,
            )),
            _ => None,
        }
    }
}

#[allow(dead_code)]
impl<'a> LivingSettlementJobV2<'a> {
    pub(crate) fn into_phase_retirement(
        self,
        policy: LivingStateRetirementPolicyV2,
    ) -> LivingPhaseRetirementJobV2<'a, Self> {
        LivingPhaseRetirementJobV2::begin(self, policy)
    }
}

impl<'a> RetirementFields<'a> for LivingDiplomacyJobV2 {
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
            1 => Some(retirement_node(std::mem::take(&mut self.hash), policy)),
            2 => Some(match self.state.0.take() {
                Some(source) => Box::new(StateRetirementNode {
                    job: LivingStateRetirementJobV2::begin_shared(source, policy),
                    complete: false,
                }) as Box<dyn RetirementPoll + 'a>,
                None => retirement_node((), policy),
            }),
            3 => Some(retirement_node(std::mem::take(&mut self.events), policy)),
            4 => Some(retirement_node(std::mem::take(&mut self.active), policy)),
            5 => Some(retirement_node(std::mem::take(&mut self.worlds), policy)),
            6 => Some(retirement_node(std::mem::take(&mut self.owners), policy)),
            7 => Some(retirement_node(std::mem::take(&mut self.graph), policy)),
            8 => Some(retirement_node(std::mem::take(&mut self.blocked), policy)),
            9 => Some(retirement_node(std::mem::take(&mut self.truces), policy)),
            10 => Some(retirement_node(std::mem::take(&mut self.existing), policy)),
            11 => Some(retirement_node(
                std::mem::take(&mut self.agreements),
                policy,
            )),
            12 => Some(retirement_node(std::mem::take(&mut self.wars), policy)),
            13 => Some(retirement_node(std::mem::take(&mut self.relations), policy)),
            14 => Some(retirement_node(std::mem::take(&mut self.busy), policy)),
            15 => Some(retirement_node(std::mem::take(&mut self.attacks), policy)),
            16 => Some(retirement_node(std::mem::take(&mut self.defenses), policy)),
            17 => Some(retirement_node(std::mem::take(&mut self.search), policy)),
            18 => Some(retirement_node(
                std::mem::take(&mut self.agreement_proposals),
                policy,
            )),
            19 => Some(retirement_node(
                std::mem::take(&mut self.war_proposals),
                policy,
            )),
            20 => Some(retirement_node(std::mem::take(&mut self.decisions), policy)),
            21 => Some(retirement_node(
                std::mem::take(&mut self.old_agreements),
                policy,
            )),
            22 => Some(retirement_node(std::mem::take(&mut self.old_wars), policy)),
            23 => Some(retirement_node(
                std::mem::take(&mut self.old_relations),
                policy,
            )),
            _ => None,
        }
    }
}

#[allow(dead_code)]
impl<'a> LivingDiplomacyJobV2 {
    pub(crate) fn into_phase_retirement(
        self,
        policy: LivingStateRetirementPolicyV2,
    ) -> LivingPhaseRetirementJobV2<'a, Self> {
        LivingPhaseRetirementJobV2::begin(self, policy)
    }
}

impl<'a> RetirementFields<'a> for LivingDispatchJobV2<'a> {
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
            4 => Some(retirement_node(std::mem::take(&mut self.metadata), policy)),
            5 => Some(retirement_node(std::mem::take(&mut self.colonies), policy)),
            6 => Some(retirement_node(std::mem::take(&mut self.dormant), policy)),
            7 => Some(retirement_node(std::mem::take(&mut self.busy), policy)),
            8 => Some(retirement_node(
                std::mem::take(&mut self.remembered),
                policy,
            )),
            9 => Some(retirement_node(std::mem::take(&mut self.trade), policy)),
            10 => Some(retirement_node(std::mem::take(&mut self.lanes), policy)),
            11 => Some(retirement_node(
                std::mem::take(&mut self.hazard_effects),
                policy,
            )),
            12 => Some(retirement_node(std::mem::take(&mut self.search), policy)),
            13 => Some(retirement_node(self.graph.0.take(), policy)),
            14 => Some(retirement_node(std::mem::take(&mut self.path), policy)),
            15 => Some(retirement_node(
                std::mem::take(&mut self.retired_path),
                policy,
            )),
            16 => Some(retirement_node(std::mem::take(&mut self.route), policy)),
            17 => Some(retirement_node(std::mem::take(&mut self.hazard), policy)),
            18 => Some(retirement_node(
                std::mem::take(&mut self.shipment_input),
                policy,
            )),
            19 => Some(retirement_node(std::mem::take(&mut self.shipments), policy)),
            20 => Some(retirement_node(
                std::mem::take(&mut self.retired_shipment),
                policy,
            )),
            _ => None,
        }
    }
}

#[allow(dead_code)]
impl<'a> LivingDispatchJobV2<'a> {
    pub(crate) fn into_phase_retirement(
        self,
        policy: LivingStateRetirementPolicyV2,
    ) -> LivingPhaseRetirementJobV2<'a, Self> {
        LivingPhaseRetirementJobV2::begin(self, policy)
    }
}

impl<'a> RetirementFields<'a> for LivingScheduledPhasesJobV2 {
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
                std::mem::replace(&mut self.cursor, ScheduledCursor::Ready(None)),
                policy,
            )),
            _ => None,
        }
    }
}

#[allow(dead_code)]
impl<'a> LivingScheduledPhasesJobV2 {
    pub(crate) fn into_phase_retirement(
        self,
        policy: LivingStateRetirementPolicyV2,
    ) -> LivingPhaseRetirementJobV2<'a, Self> {
        LivingPhaseRetirementJobV2::begin(self, policy)
    }
}

impl<'a> RetirementFields<'a> for LivingConflictJobV2<'a> {
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
            1 => Some(retirement_node(std::mem::take(&mut self.component), policy)),
            2 => Some(retirement_node(
                self.output.take().map(ConflictReadyRetirement),
                policy,
            )),
            3 => Some(retirement_node(std::mem::take(&mut self.arrivals), policy)),
            4 => Some(retirement_node(self.catalog.0.take(), policy)),
            _ => None,
        }
    }
}

#[allow(dead_code)]
impl<'a> LivingConflictJobV2<'a> {
    pub(crate) fn into_phase_retirement(
        self,
        policy: LivingStateRetirementPolicyV2,
    ) -> LivingPhaseRetirementJobV2<'a, Self> {
        LivingPhaseRetirementJobV2::begin(self, policy)
    }
}

impl<'a> RetirementFields<'a> for CostSearch {
    fn retirement_child(
        &mut self,
        index: usize,
        policy: LivingStateRetirementPolicyV2,
    ) -> Option<Box<dyn RetirementPoll + 'a>> {
        match index {
            0 => Some(retirement_node(std::mem::take(&mut self.distance), policy)),
            1 => Some(retirement_node(std::mem::take(&mut self.frontier), policy)),
            _ => None,
        }
    }
}

impl<'a> RetirementFields<'a> for RetreatGraph {
    fn retirement_child(
        &mut self,
        index: usize,
        policy: LivingStateRetirementPolicyV2,
    ) -> Option<Box<dyn RetirementPoll + 'a>> {
        match index {
            0 => Some(retirement_node(std::mem::take(&mut self.systems), policy)),
            1 => Some(retirement_node(std::mem::take(&mut self.lanes), policy)),
            _ => None,
        }
    }
}

impl<'a> RetirementFields<'a> for RetreatDistance {
    fn retirement_child(
        &mut self,
        index: usize,
        policy: LivingStateRetirementPolicyV2,
    ) -> Option<Box<dyn RetirementPoll + 'a>> {
        match index {
            0 => Some(retirement_node(std::mem::take(&mut self.path), policy)),
            _ => None,
        }
    }
}

impl<'a> RetirementFields<'a> for LivingRecipeOutputV2 {
    fn retirement_child(
        &mut self,
        index: usize,
        policy: LivingStateRetirementPolicyV2,
    ) -> Option<Box<dyn RetirementPoll + 'a>> {
        match index {
            0 => Some(retirement_node(
                std::mem::take(&mut self.blocked_reasons),
                policy,
            )),
            _ => None,
        }
    }
}

struct ScheduledReadyRetirement(Box<LivingScheduledPhaseOutputV2>);
impl<'a> RetirementFields<'a> for ScheduledReadyRetirement {
    fn retirement_child(
        &mut self,
        index: usize,
        policy: LivingStateRetirementPolicyV2,
    ) -> Option<Box<dyn RetirementPoll + 'a>> {
        match index {
            0 => Some(retirement_node(std::mem::take(&mut self.0.0), policy)),
            1 => Some(retirement_node(std::mem::take(&mut self.0.1), policy)),
            2 => Some(retirement_node(std::mem::take(&mut self.0.2), policy)),
            _ => None,
        }
    }
}
struct ConflictReadyRetirement(Box<(LivingGalaxyStateV2, LivingPendingEventsV2)>);
impl<'a> RetirementFields<'a> for ConflictReadyRetirement {
    fn retirement_child(
        &mut self,
        index: usize,
        policy: LivingStateRetirementPolicyV2,
    ) -> Option<Box<dyn RetirementPoll + 'a>> {
        match index {
            0 => Some(retirement_node(std::mem::take(&mut self.0.0), policy)),
            1 => Some(retirement_node(std::mem::take(&mut self.0.1), policy)),
            _ => None,
        }
    }
}
impl<'a> RetirementFields<'a> for ScheduledCursor {
    const CUSTOM_NODE: bool = true;
    fn retirement_child(
        &mut self,
        _: usize,
        _: LivingStateRetirementPolicyV2,
    ) -> Option<Box<dyn RetirementPoll + 'a>> {
        unreachable!("cursor moves its original Box")
    }
    fn retirement_node(self, p: LivingStateRetirementPolicyV2) -> Box<dyn RetirementPoll + 'a> {
        match self {
            Self::Lifecycles(job) => retirement_node(job, p),
            Self::Arrivals(job) => retirement_node(job, p),
            Self::Ready(output) => retirement_node(output.map(ScheduledReadyRetirement), p),
        }
    }
}
impl<'a> RetirementFields<'a> for ConflictComponent<'a> {
    const CUSTOM_NODE: bool = true;
    fn retirement_child(
        &mut self,
        _: usize,
        _: LivingStateRetirementPolicyV2,
    ) -> Option<Box<dyn RetirementPoll + 'a>> {
        unreachable!("component moves its original Box")
    }
    fn retirement_node(self, p: LivingStateRetirementPolicyV2) -> Box<dyn RetirementPoll + 'a> {
        match self {
            Self::Retreat(job) => retirement_node(job, p),
            Self::Combat(job) => retirement_node(job, p),
            Self::Occupation(job) => retirement_node(job, p),
            Self::Settlement(job) => retirement_node(job, p),
        }
    }
}
impl<'a, T: LivingKeyedV2 + RetirementFields<'a>> RetirementFields<'a>
    for LivingSortedInsertionJobV2<T>
{
    fn retirement_child(
        &mut self,
        index: usize,
        policy: LivingStateRetirementPolicyV2,
    ) -> Option<Box<dyn RetirementPoll + 'a>> {
        match index {
            0 => Some(retirement_node(
                std::mem::replace(&mut self.source, Vec::new().into_iter()),
                policy,
            )),
            1 => Some(retirement_node(std::mem::take(&mut self.output), policy)),
            2 => Some(retirement_node(self.inserted.take(), policy)),
            3 => Some(retirement_node(self.held.take(), policy)),
            _ => None,
        }
    }
}
#[allow(dead_code)]
impl<'a, T: LivingKeyedV2 + RetirementFields<'a>> LivingSortedInsertionJobV2<T> {
    pub(crate) fn into_phase_retirement(
        self,
        policy: LivingStateRetirementPolicyV2,
    ) -> LivingPhaseRetirementJobV2<'a, Self> {
        LivingPhaseRetirementJobV2::begin(self, policy)
    }
}

#[derive(Default)]
pub(crate) struct PhaseRetirementLane<'a> {
    node: Option<Box<dyn RetirementPoll + 'a>>,
    deferred: Option<LivingPhaseDeferredRetirementV2>,
}
impl<'a> PhaseRetirementLane<'a> {
    pub(crate) fn is_empty(&self) -> bool {
        self.node.is_none() && self.deferred.is_none()
    }
    pub(crate) fn start<T: RetirementFields<'a>>(&mut self, owner: T) {
        assert!(
            self.node.is_none() && self.deferred.is_none(),
            "one retained retirement lane"
        );
        self.node = Some(retirement_node(
            owner,
            LivingStateRetirementPolicyV2::ReleaseHandle,
        ));
    }
    pub(crate) fn visit(&mut self) -> bool {
        let Some(node) = &mut self.node else {
            return false;
        };
        match node.retirement_step() {
            RetirementStep::Work => {}
            RetirementStep::Done => drop(self.node.take()),
            RetirementStep::Deferred(ticket) => {
                self.deferred = Some(ticket);
                // Internal lanes always atomically release handles while their
                // parent holds the source anchor. Preserve unexpected tickets.
                panic!("internal release lane returned a deferred owner");
            }
        }
        true
    }
}
impl<'a, 'b: 'a> RetirementFields<'a> for PhaseRetirementLane<'b> {
    fn retirement_child(
        &mut self,
        index: usize,
        policy: LivingStateRetirementPolicyV2,
    ) -> Option<Box<dyn RetirementPoll + 'a>> {
        match index {
            0 => Some(
                self.node
                    .take()
                    .unwrap_or_else(|| retirement_node((), policy)),
            ),
            1 => Some(retirement_node(self.deferred.take(), policy)),
            _ => None,
        }
    }
}

impl std::fmt::Debug for PhaseRetirementLane<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PhaseRetirementLane")
            .field("active", &self.node.is_some())
            .field("deferred", &self.deferred.is_some())
            .finish()
    }
}

fn ensure_retained_sorted<T: LivingKeyedV2>(
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

impl<'a> RetirementFields<'a> for LivingFleetOrderV2 {
    fn retirement_child(
        &mut self,
        index: usize,
        policy: LivingStateRetirementPolicyV2,
    ) -> Option<Box<dyn RetirementPoll + 'a>> {
        (index == 0).then(|| retirement_node(std::mem::take(&mut self.lane_path), policy))
    }
}

#[cfg(test)]
mod phase_retirement_tests {
    use super::*;
    const MIXED: &[usize] = &[1, 64, 7, 1024];
    fn drain<'a, T: RetirementFields<'a>>(owner: T, budgets: &[usize]) -> u64 {
        let mut job =
            LivingPhaseRetirementJobV2::begin(owner, LivingStateRetirementPolicyV2::ReleaseHandle);
        assert_eq!(job.poll(0), Err(LivingLifecycleJobErrorV2::Budget));
        assert_eq!(job.last_poll_visits(), 0);
        assert_eq!(job.poll(1025), Err(LivingLifecycleJobErrorV2::Budget));
        assert_eq!(job.total_visits(), 0);
        let mut n = 0;
        loop {
            let budget = budgets[n % budgets.len()];
            let result = job.poll(budget).unwrap();
            assert!(job.last_poll_visits() <= budget);
            assert!(job.take_deferred_owner().is_none());
            n += 1;
            if result == LivingLifecycleProgressV2::Ready {
                break;
            }
            assert!(n < 100_000, "retirement failed to complete");
        }
        let total = job.total_visits();
        assert_eq!(job.poll(1), Ok(LivingLifecycleProgressV2::Ready));
        assert_eq!(job.last_poll_visits(), 0);
        assert_eq!(job.total_visits(), total);
        total
    }
    pub(super) fn exercise<T, F, P, C, E, G>(factory: F, poll: P, stop: C, take: E, stage: G)
    where
        T: RetirementFields<'static>,
        F: Fn() -> T,
        P: Fn(&mut T, usize) -> Result<LivingLifecycleProgressV2, LivingLifecycleJobErrorV2>,
        C: Fn(&mut T, bool),
        E: Fn(&mut T) -> Result<(), LivingLifecycleJobErrorV2>,
        G: Fn(&T) -> String,
    {
        let mut probe = factory();
        let mut stages = std::collections::HashSet::new();
        let mut cuts = vec![0];
        let mut polls = 0;
        loop {
            if stages.insert(stage(&probe)) {
                cuts.push(polls);
            }
            let progress = poll(&mut probe, 1).unwrap();
            polls += 1;
            if progress == LivingLifecycleProgressV2::Ready {
                cuts.push(polls);
                break;
            }
            assert!(polls < 100_000, "forward phase did not complete");
        }
        take(&mut probe).unwrap();
        assert_eq!(take(&mut probe), Err(LivingLifecycleJobErrorV2::NotReady));
        assert_eq!(poll(&mut probe, 1), Ok(LivingLifecycleProgressV2::Ready));
        drain(probe, MIXED);
        cuts.sort_unstable();
        cuts.dedup();
        for cutoff in cuts {
            for fault in [false, true] {
                let mut expected = None;
                for budgets in [&[1][..], &[64][..], &[1024][..], MIXED] {
                    let mut job = factory();
                    for _ in 0..cutoff {
                        poll(&mut job, 1).unwrap();
                    }
                    stop(&mut job, fault);
                    let error = if fault {
                        LivingLifecycleJobErrorV2::Fault(LivingDeterministicFaultV2::Arithmetic)
                    } else {
                        LivingLifecycleJobErrorV2::Cancelled
                    };
                    assert_eq!(take(&mut job), Err(error.clone()));
                    assert_eq!(poll(&mut job, 1), Err(error));
                    let total = drain(job, budgets);
                    if let Some(expected) = expected {
                        assert_eq!(total, expected, "cutoff {cutoff}, fault {fault}");
                    } else {
                        expected = Some(total);
                    }
                }
            }
        }
    }
    macro_rules! phase_case {
        ($name:ident,$factory:expr,$stage:ident) => {
            #[test]
            fn $name() {
                let authority = crate::living::builtin_living_sandbox_v2().unwrap();
                let state = authority.state().clone();
                let catalog = authority.catalog_snapshot();
                let factory = $factory;
                exercise(
                    || factory(state.clone(), catalog.clone()),
                    |j, b| j.poll(b),
                    |j, fault| {
                        if fault {
                            j.fault = Some(LivingDeterministicFaultV2::Arithmetic)
                        } else {
                            j.cancel()
                        }
                    },
                    |j| j.take_ready_result().map(|_| ()),
                    |j| format!("{:?}", std::mem::discriminant(&j.$stage)),
                );
            }
        };
    }
    phase_case!(
        phase_retirement_lifecycle,
        |s, _c| LivingLifecycleJobV2::begin(s, LivingPendingEventsV2::new(), LivingTickV2(1)),
        cursor
    );
    phase_case!(
        phase_retirement_arrival,
        |s, _c| LivingArrivalJobV2::begin(s, LivingPendingEventsV2::new(), LivingTickV2(1)),
        cursor
    );
    phase_case!(
        phase_retirement_observation,
        |s, _c| LivingObservationJobV2::begin(s, LivingTickV2(1)),
        cursor
    );
    phase_case!(
        phase_retirement_repair,
        |s, c| LivingRepairJobV2::begin_owned(c, s, LivingTickV2(1)),
        cursor
    );
    phase_case!(
        phase_retirement_combat,
        |s, c| LivingCombatJobV2::begin_owned(c, s, LivingPendingEventsV2::new(), LivingTickV2(10)),
        stage
    );
    phase_case!(
        phase_retirement_retreat,
        |s, c| LivingRetreatJobV2::begin_owned(c, s, LivingPendingEventsV2::new(), LivingTickV2(1)),
        stage
    );
    phase_case!(
        phase_retirement_occupation,
        |s, c| LivingOccupationJobV2::begin_owned(
            c,
            s,
            LivingPendingEventsV2::new(),
            LivingTickV2(1)
        ),
        stage
    );
    phase_case!(
        phase_retirement_settlement,
        |s, c| LivingSettlementJobV2::begin_owned(
            c,
            s,
            LivingPendingEventsV2::new(),
            LivingTickV2(1),
            [0; 32],
            Vec::new()
        ),
        stage
    );
    phase_case!(
        phase_retirement_diplomacy,
        |mut s: LivingGalaxyStateV2, _c| {
            s.tick = LivingTickV2(49);
            LivingDiplomacyJobV2::begin(s, LivingPendingEventsV2::new(), LivingTickV2(50))
        },
        stage
    );
    phase_case!(
        phase_retirement_dispatch,
        |s, c| LivingDispatchJobV2::begin_owned(
            c,
            s,
            LivingPendingEventsV2::new(),
            LivingTickV2(1),
            [0; 32],
            super::super::ids::LivingBranchIdV2([0; 16])
        ),
        stage
    );
    #[test]
    fn phase_retirement_scheduled() {
        let authority = crate::living::builtin_living_sandbox_v2().unwrap();
        exercise(
            || {
                LivingScheduledPhasesJobV2::begin(
                    authority.state().clone(),
                    LivingPendingEventsV2::new(),
                    LivingTickV2(1),
                )
            },
            |j, b| j.poll(b),
            |j, f| {
                if f {
                    j.fault = Some(LivingLifecycleJobErrorV2::Fault(
                        LivingDeterministicFaultV2::Arithmetic,
                    ))
                } else {
                    j.cancel()
                }
            },
            |j| j.take_ready_result().map(|_| ()),
            |j| format!("{:?}", std::mem::discriminant(&j.cursor)),
        );
    }
    #[test]
    fn phase_retirement_conflict() {
        let authority = crate::living::builtin_living_sandbox_v2().unwrap();
        exercise(
            || {
                LivingConflictJobV2::begin_owned(
                    authority.catalog_snapshot(),
                    authority.state().clone(),
                    LivingPendingEventsV2::new(),
                    LivingTickV2(10),
                    [0; 32],
                    Vec::new(),
                )
            },
            |j, b| j.poll(b),
            |j, f| {
                if f {
                    j.fault = Some(LivingDeterministicFaultV2::Arithmetic)
                } else {
                    j.cancel()
                }
            },
            |j| j.take_ready_result().map(|_| ()),
            |j| j.component_name().to_owned(),
        );
    }
    #[test]
    fn phase_retirement_recipe() {
        exercise(
            || {
                LivingRecipeJobV2::begin(
                    Default::default(),
                    10,
                    0,
                    vec![(LivingResourceV2::Ore, 2); 4],
                    vec![(LivingResourceV2::Energy, 1); 3],
                )
            },
            |j, b| j.poll(b),
            |j, f| {
                if f {
                    j.fault = Some(LivingDeterministicFaultV2::Arithmetic)
                } else {
                    j.cancel()
                }
            },
            |j| j.take_ready_result().map(|_| ()),
            |j| j.stage.to_string(),
        );
    }
    #[test]
    fn phase_retirement_exact_leaf_and_nested_storage_totals() {
        // Every moved leaf uses one acquisition, one parent release and one
        // dynamic Box release. The retained outer allocation releases last.
        for budgets in [&[1][..], &[64][..], &[1024][..], MIXED] {
            assert_eq!(drain(vec![1u64, 2, 3], budgets), 10);
            assert_eq!(drain(Vec::<u64>::with_capacity(80), budgets), 1);
            let mut values = vec![vec![1u64, 2], Vec::with_capacity(20)];
            values.reserve(30);
            assert_eq!(drain(values, budgets), 13);
        }
    }
    #[test]
    fn phase_retirement_nested_rows_use_state_cursor_without_reinsertion() {
        for budgets in [&[1][..], &[64][..], &[1024][..], MIXED] {
            let mut state = state_retirement_tests::fixture();
            let fleet = state.fleets.into_vec().pop().unwrap();
            let original_path = fleet.order.as_ref().unwrap().lane_path.as_ptr();
            let mut insertion =
                LivingSortedInsertionJobV2::begin(LivingSortedVecV2::default(), fleet);
            assert_eq!(
                insertion
                    .inserted
                    .as_ref()
                    .unwrap()
                    .order
                    .as_ref()
                    .unwrap()
                    .lane_path
                    .as_ptr(),
                original_path
            );
            assert!(insertion.take_ready_result().is_err());
            let total = drain(insertion, budgets);
            // source: 3 (move/storage/Box); output: 6 (SortedVec + Vec);
            // inserted: 13 (Option + shared Fleet cursor's 8 + shells);
            // absent held Option: 3; original insertion parent: 1.
            assert_eq!(total, 3 + 6 + 13 + 3 + 1);
            state.facilities = LivingSortedVecV2::default();
        }
    }
    #[test]
    fn phase_retirement_diplomacy_rejected_unwrap_preserves_exact_arc() {
        let mut job = LivingDiplomacyJobV2::begin(
            Default::default(),
            LivingPendingEventsV2::new(),
            LivingTickV2(0),
        );
        job.stage = DiplomacyStage::Ready;
        let anchor = Arc::clone(&job.state);
        assert_eq!(
            job.take_ready_result().err(),
            Some(LivingLifecycleJobErrorV2::NotReady)
        );
        assert!(Arc::ptr_eq(&anchor, &job.state));
        drop(anchor);
        assert!(job.take_ready_result().is_ok());
        drain(job, MIXED);
    }
    #[test]
    fn phase_retirement_boxed_state_uses_reviewed_accounting() {
        let mut reference =
            LivingStateRetirementJobV2::begin_boxed(Box::new(state_retirement_tests::fixture()));
        while reference.poll(1).unwrap() != LivingLifecycleProgressV2::Ready {}
        let expected = reference.total_visits() + 2; // counted root-node move and empty adapter Box release
        for budgets in [&[1][..], &[64][..], &[1024][..], MIXED] {
            assert_eq!(
                drain(Box::new(state_retirement_tests::fixture()), budgets),
                expected
            );
        }
    }
    #[test]
    fn phase_retirement_original_box_survives_all_children() {
        let mut state = state_retirement_tests::fixture();
        let owner = Box::new(LivingObservationJobV2::begin(
            std::mem::take(&mut state),
            LivingTickV2(1),
        ));
        let pointer = &*owner as *const LivingObservationJobV2;
        let mut job =
            LivingPhaseRetirementJobV2::begin(owner, LivingStateRetirementPolicyV2::ReleaseHandle);
        let mut visits = 0;
        while let Some(owner) = job.root.owner.as_ref() {
            assert_eq!(&**owner as *const LivingObservationJobV2, pointer);
            job.poll(1).unwrap();
            visits += 1;
            assert!(visits < 10000);
        }
        assert!(visits > 20);
        assert_eq!(job.poll(1), Ok(LivingLifecycleProgressV2::Ready));
    }
    #[test]
    fn phase_retirement_path_all_partial_cursors_and_shared_graph() {
        let graph = Arc::new(RetreatGraph {
            systems: vec![LivingEntityIdV2([1; 16]), LivingEntityIdV2([2; 16])],
            lanes: vec![(
                LivingEntityIdV2([3; 16]),
                LivingEntityIdV2([1; 16]),
                LivingEntityIdV2([2; 16]),
                10,
            )],
        });
        let make = || RetreatPathJob::begin(graph.clone(), graph.systems[0], graph.systems[1]);
        let mut probe = make();
        let mut count = 0;
        while !probe.ready() {
            probe.visit();
            count += 1;
            assert!(count < 1000);
        }
        for cutoff in 0..=count {
            let mut expected = None;
            for budgets in [&[1][..], &[64][..], &[1024][..], MIXED] {
                let mut path = make();
                for _ in 0..cutoff {
                    path.visit();
                }
                if !path.ready() {
                    assert!(path.take_ready_result().is_err());
                }
                let total = drain(path, budgets);
                if let Some(expected) = expected {
                    assert_eq!(total, expected);
                } else {
                    expected = Some(total);
                }
            }
        }
        assert_eq!(graph.systems.len(), 2);
    }
}

#[cfg(test)]
mod phase_retirement_shared_tests {
    use super::*;
    #[test]
    fn phase_retirement_deferred_graph_is_exact_and_unclaimed_owner_is_retained() {
        let graph = Arc::new(RetreatGraph {
            systems: vec![LivingEntityIdV2([1; 16]); 7],
            lanes: Vec::with_capacity(40),
        });
        let pointer = Arc::as_ptr(&graph).cast::<()>();
        let path = RetreatPathJob::begin(
            graph.clone(),
            LivingEntityIdV2([1; 16]),
            LivingEntityIdV2([1; 16]),
        );
        let mut job = path.into_phase_retirement(LivingStateRetirementPolicyV2::DeferUntilUnique);
        while job.deferred.is_none() {
            job.poll(1).unwrap();
        }
        let count = job.total_visits();
        assert_eq!(job.poll(64), Ok(LivingLifecycleProgressV2::Pending));
        assert_eq!(job.last_poll_visits(), 0);
        assert_eq!(job.total_visits(), count);
        let ticket = job.take_deferred_owner().unwrap();
        let LivingPhaseDeferredRetirementV2::Private(owner) = &ticket else {
            panic!("graph ticket")
        };
        assert_eq!(owner.node.shared_identity(), Some(pointer));
        assert_eq!(Arc::strong_count(&graph), 2);
        assert!(job.take_deferred_owner().is_none());
        while job.poll(64).unwrap() != LivingLifecycleProgressV2::Ready {}
        let weak = Arc::downgrade(&graph);
        drop(graph);
        let mut resumed = LivingPhaseRetirementJobV2::begin(
            ticket,
            LivingStateRetirementPolicyV2::DeferUntilUnique,
        );
        while resumed.poll(1).unwrap() != LivingLifecycleProgressV2::Ready {
            assert!(resumed.take_deferred_owner().is_none());
        }
        assert!(weak.upgrade().is_none());
    }
    #[test]
    fn phase_retirement_sorted_rejection_retains_nested_row() {
        let mut fixture = state_retirement_tests::fixture();
        let fleet = fixture.fleets.into_vec().pop().unwrap();
        let pointer = fleet.hulls.as_slice().as_ptr();
        let mut job =
            LivingSortedInsertionJobV2::begin(LivingSortedVecV2::default(), fleet.clone());
        job.output = LivingSortedVecV2::new(vec![fleet]).unwrap();
        job.source_finished = true;
        let inserted_pointer = job.inserted.as_ref().unwrap().hulls.as_slice().as_ptr();
        assert_ne!(pointer, inserted_pointer);
        assert!(job.visit().is_err());
        assert_eq!(
            job.inserted.as_ref().unwrap().hulls.as_slice().as_ptr(),
            inserted_pointer
        );
        let mut retirement =
            job.into_phase_retirement(LivingStateRetirementPolicyV2::ReleaseHandle);
        while retirement.poll(1).unwrap() != LivingLifecycleProgressV2::Ready {}
        fixture.colonies = Default::default();
    }
}

struct RetirementLeaf<T>(Option<T>);
impl<T> RetirementPoll for RetirementLeaf<T> {
    fn retirement_step(&mut self) -> RetirementStep {
        if self.0.take().is_some() {
            RetirementStep::Work
        } else {
            RetirementStep::Done
        }
    }
}

impl<'a> RetirementFields<'a> for LivingStateCopyJobV2 {
    const CUSTOM_NODE: bool = true;
    fn retirement_child(
        &mut self,
        _: usize,
        _: LivingStateRetirementPolicyV2,
    ) -> Option<Box<dyn RetirementPoll + 'a>> {
        unreachable!("copy uses reviewed adapter")
    }
    fn retirement_node(
        self,
        policy: LivingStateRetirementPolicyV2,
    ) -> Box<dyn RetirementPoll + 'a> {
        Box::new(self.into_retirement(policy))
    }
}
