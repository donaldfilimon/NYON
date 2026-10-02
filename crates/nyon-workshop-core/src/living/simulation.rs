//! Living Galaxy V2 creator queue and atomic boundary orchestration.
//!
//! Section 2 of
//! `docs/superpowers/specs/2026-09-04-nyon-living-galaxy-rules.md` is binding.
//! The authority publishes a cursor of committed revision, completed tick and
//! pending tail; a submission must name all three exactly or it is stale. A
//! running envelope is validated against a private projection that already
//! holds every earlier accepted envelope for the same boundary, and a paused
//! envelope applies synchronously at the current boundary with an empty queue.
//!
//! A step clones the state, runs the ten published phases in registry order on
//! the clone, validates it, hashes it, seals the receipt, and only then swaps
//! every authority field at once. A fault leaves the state, the revision log
//! and the queue exactly as they were and pauses the authority with the typed
//! fault retained for diagnosis.
//!
//! # What this module decides rather than transcribes
//!
//! - Phases 2 through 9 run in this file, in order and on their published
//!   cadence. Their bodies live in `rules`, which this module calls.
//! - Phase 1 applies creator operations. Removals stay refused at submission
//!   with [`LivingCommandRejectionV2::UnsupportedOperation`] rather than
//!   accepted and skipped, so no accepted revision can silently fail to mean
//!   what it says.
//! - Each applied revision emits exactly one `creator_intervention` event whose
//!   provenance names that revision.
//! - A state's historical `accepted_sequence` is the sequence of the last
//!   revision applied into it, which replay reproduces from the revision log
//!   alone. The live allocator mark is kept separately and never hashed.
//! - A lane is stored with its lower-identified endpoint first whatever order
//!   the batch named them in, because a batch creating both systems cannot know
//!   their identities' order before its revision exists.

use std::sync::Arc;

mod retirement;
use retirement::OwnerSlot;
pub use retirement::{
    LivingPublicationRetirementJobV2, LivingPublicationRetirementPolicyV2,
    LivingPublicationRetirementProgressV2,
};

use super::catalog::ValidatedLivingCatalogPackV2;
use super::command::{
    LivingAcceptedCommandV2, LivingCommandEnvelopeV2, LivingCommandErrorV2, LivingCommandModeV2,
    LivingCommandTargetV2, LivingCommandV2, LivingCreatorOperationV2, LivingRevisionV2,
};
use super::genesis::{LivingGenesisGeneratorV2, ValidatedLivingGenesisV2};
use super::history::{
    LivingBookmarkV2, LivingComparisonV2, LivingHistoryBranchV2, LivingHistoryErrorV2,
    LivingHistoryViewV2, LivingRecordedRevisionV2,
};
use super::ids::{
    LIVING_PHASE_REGISTRY_V2, LivingBranchIdV2, LivingEntityIdV2, LivingPhaseV2,
    LivingRevisionIdV2, LivingStateDigestV2, LivingTickV2, fork_branch_id_v2,
    genesis_fork_branch_id_v2,
};
use super::model::{
    LivingCivilizationV2, LivingDepositV2, LivingGalaxyStateV2, LivingLaneV2, LivingStarV2,
    LivingSystemV2, LivingValidationErrorV2, LivingWorldV2,
};
use super::receipt::{
    LivingEventKindV2, LivingEventProvenanceV2, LivingPendingEventsV2, LivingReceiptErrorV2,
    LivingTickReceiptV2, seal_living_tick_receipt_v2,
};
use super::rules;
use super::wire::{LivingKeyedV2, LivingSortedVecV2, LivingWireErrorV2};

/// The deepest the running queue may grow before one boundary drains it.
///
/// Section 1 sets it to the declared work-unit poll bound, so one boundary's
/// queue can never exceed one poll's budget.
pub const LIVING_MAX_PENDING_QUEUE_DEPTH_V2: usize = 1_024;

/// Diplomacy runs on multiples of this many ticks (section 2, step 7).
const DIPLOMACY_PERIOD_TICKS: u64 = 100;

/// Intent generation runs on multiples of this many ticks (section 2, step 8).
const INTENT_PERIOD_TICKS: u64 = 50;

/// What the authority publishes for submitters to name.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct LivingCommandCursorV2 {
    /// The last committed revision, or `None` before any exists.
    pub committed_revision: Option<LivingRevisionIdV2>,
    /// The completed boundary.
    pub tick: LivingTickV2,
    /// The accepted sequence at the queue tail, or zero for an empty queue.
    pub pending_sequence: u64,
}

/// Why an authority could not be started.
#[derive(Clone, Copy, Debug, Eq, PartialEq, thiserror::Error)]
pub enum LivingAuthorityErrorV2 {
    /// The genesis was validated under a different catalog than the one given.
    #[error("the Living V2 genesis was validated under a different catalog")]
    CatalogMismatch,
    /// The genesis state does not validate under the catalog.
    #[error("the Living V2 genesis state is invalid: {0}")]
    Validation(#[from] LivingValidationErrorV2),
}

/// Every way a submission can be refused.
///
/// A refused submission consumes no sequence, no entity identity, no queue
/// position and no resource.
#[derive(Clone, Debug, Eq, PartialEq, thiserror::Error)]
pub enum LivingCommandRejectionV2 {
    /// The envelope named a committed revision other than the published one.
    #[error("the Living V2 envelope expects a committed revision that is not current")]
    StaleRevision {
        /// What the envelope named.
        expected: Option<LivingRevisionIdV2>,
        /// What the authority publishes.
        found: Option<LivingRevisionIdV2>,
    },
    /// The envelope named a completed tick other than the published one.
    #[error("the Living V2 envelope expects tick {expected:?}; the authority is at {found:?}")]
    StaleTick {
        /// What the envelope named.
        expected: LivingTickV2,
        /// What the authority publishes.
        found: LivingTickV2,
    },
    /// The envelope named a queue tail other than the published one.
    #[error("the Living V2 envelope expects queue tail {expected}; the tail is {found}")]
    StalePendingSequence {
        /// What the envelope named.
        expected: u64,
        /// What the authority publishes.
        found: u64,
    },
    /// The envelope's mode does not match the entry point it was given to.
    #[error("a {found:?} Living V2 envelope was given to the {expected:?} entry point")]
    ModeMismatch {
        /// The mode the entry point accepts.
        expected: LivingCommandModeV2,
        /// The mode the envelope carries.
        found: LivingCommandModeV2,
    },
    /// Paused application requires an empty running queue.
    #[error("a paused Living V2 edit requires an empty running queue")]
    PausedWithRunningQueue,
    /// The running queue is already at its declared depth.
    #[error("the Living V2 running queue holds at most {limit} envelopes")]
    QueueFull {
        /// The declared depth.
        limit: usize,
    },
    /// Retained immutable history has reached its declared capacity.
    #[error("the Living V2 {collection} history holds at most {limit} entries")]
    HistoryFull {
        /// The exhausted history collection.
        collection: &'static str,
        /// Its normative capacity.
        limit: usize,
    },
    /// History browsing requires explicit Branch here before editing.
    #[error("branch from the selected Living history view before editing")]
    ViewingHistory,
    /// The authority is paused on a deterministic fault.
    #[error("the Living V2 authority is paused on a fault")]
    Faulted,
    /// The document-global sequence or the boundary counter is exhausted.
    #[error("the Living V2 sequence or tick counter is exhausted")]
    Exhausted,
    /// The batch failed its structural checks.
    #[error("the Living V2 command is structurally invalid: {0}")]
    Command(#[from] LivingCommandErrorV2),
    /// This slice does not yet apply the operation at this batch position.
    #[error("Living V2 operation {operation} is not applied by this rules slice")]
    UnsupportedOperation {
        /// Position of the operation in its batch.
        operation: usize,
    },
    /// An edit names an existing entity the projection does not hold.
    #[error("Living V2 operation {operation} names a missing {reference}")]
    MissingTarget {
        /// Position of the operation in its batch.
        operation: usize,
        /// What kind of record was expected.
        reference: &'static str,
    },
    /// A reviewed removal omitted, duplicated, or added unrelated dependents.
    #[error("Living V2 removal cascade does not match the target's exact dependencies")]
    CascadeMismatch,
    /// A no-cascade removal still has entity or derived-record dependents.
    #[error("Living V2 removal requires an explicit reviewed cascade")]
    ReferencedTarget,
    /// The projection with this command applied does not validate.
    #[error("the Living V2 projection is invalid: {0}")]
    Projection(#[from] LivingValidationErrorV2),
    /// The committed paused edit could not be hashed or sealed.
    #[error("the Living V2 paused edit could not be sealed: {0}")]
    Seal(LivingDeterministicFaultV2),
}

/// A failure inside a boundary. The boundary commits nothing.
#[derive(Clone, Debug, Eq, PartialEq, thiserror::Error)]
pub enum LivingDeterministicFaultV2 {
    /// A browsed history view must return to present or branch before stepping.
    #[error("the Living authority is viewing retained history")]
    ViewingHistory,
    /// An internal invariant did not hold.
    #[error("a Living V2 authority invariant failed")]
    Invariant,
    /// A capacity reservation could not be honored.
    #[error("a Living V2 capacity reservation failed")]
    Capacity,
    /// Checked arithmetic overflowed.
    #[error("Living V2 arithmetic overflowed")]
    Arithmetic,
    /// A queued envelope no longer reproduces its validated result.
    #[error("queued Living V2 envelope {accepted_sequence} did not reproduce: {reason}")]
    Replay {
        /// The envelope's accepted sequence.
        accepted_sequence: u64,
        /// Why it failed, as the submission path would have reported it.
        reason: Box<LivingCommandRejectionV2>,
    },
    /// The candidate state failed validation before commit.
    #[error("the Living V2 candidate state is invalid: {0}")]
    Validation(#[from] LivingValidationErrorV2),
    /// The candidate state could not be encoded.
    #[error("the Living V2 candidate state is not canonical: {0}")]
    Wire(#[from] LivingWireErrorV2),
    /// The receipt could not be sealed.
    #[error("the Living V2 receipt could not be sealed: {0}")]
    Receipt(#[from] LivingReceiptErrorV2),
    /// An authoritative decision sidecar failed before boundary commit.
    #[error("the Living V2 decision receipt could not be sealed: {0}")]
    Decision(#[from] super::decision::LivingDecisionErrorV2),
    /// An explained-event sidecar failed before atomic boundary publication.
    #[error("the Living V2 event facts sidecar could not be sealed: {0}")]
    EventFacts(#[from] super::event_facts::LivingEventFactsErrorV2),
}

/// The mutable view a phase receives of one boundary's candidate.
///
/// Phase code reaches the candidate only through this context and emits events
/// only through [`Self::emit`], which assigns the ordinal.
#[allow(dead_code)] // The civilization phase modules are the readers.
pub(crate) struct LivingStepContextV2<'a> {
    pub(crate) catalog: &'a ValidatedLivingCatalogPackV2,
    pub(crate) genesis_seed: [u8; 32],
    pub(crate) branch: LivingBranchIdV2,
    pub(crate) tick: LivingTickV2,
    pub(crate) state: &'a mut LivingGalaxyStateV2,
    pub(crate) events: &'a mut LivingPendingEventsV2,
    /// Fleets that docked during phase 3, so phase 4 can open claims.
    pub(crate) arrivals: Vec<LivingEntityIdV2>,
}

impl LivingStepContextV2<'_> {
    /// Record one event of this boundary.
    pub(crate) fn emit(
        &mut self,
        provenance: LivingEventProvenanceV2,
        kind: LivingEventKindV2,
    ) -> Result<(), LivingDeterministicFaultV2> {
        self.events.record(provenance, kind)?;
        Ok(())
    }
}

#[derive(Clone, Debug)]
struct QueuedCommandV2 {
    accepted: OwnerSlot<Arc<LivingAcceptedCommandV2>>,
    revision: OwnerSlot<Arc<LivingRevisionV2>>,
}

/// Immutable inputs and freshness token for a prospective boundary job.
/// Capturing this token does not clone queued command payloads or state records.
#[derive(Clone, Debug)]
pub struct LivingBoundaryCaptureV2 {
    state: OwnerSlot<Arc<LivingGalaxyStateV2>>,
    catalog: OwnerSlot<Arc<ValidatedLivingCatalogPackV2>>,
    queue: OwnerSlot<Arc<Vec<QueuedCommandV2>>>,
    revisions: OwnerSlot<Arc<Vec<LivingRevisionV2>>>,
    revision_modes: OwnerSlot<Arc<Vec<LivingCommandModeV2>>>,
    genesis_seed: [u8; 32],
    branch: LivingBranchIdV2,
    committed_revision: Option<LivingRevisionIdV2>,
    accepted_sequence_high_water: u64,
    branch_sequence_high_water: u64,
}

impl LivingBoundaryCaptureV2 {
    /// Immutable source state, retained in constant time.
    pub fn state_snapshot(&self) -> Arc<LivingGalaxyStateV2> {
        Arc::clone(&self.state)
    }
    /// Immutable exact catalog, retained in constant time.
    pub fn catalog_snapshot(&self) -> Arc<ValidatedLivingCatalogPackV2> {
        Arc::clone(&self.catalog)
    }
    /// Number of immutable accepted records captured for phase one.
    pub fn pending_count(&self) -> usize {
        self.queue.len()
    }
}

/// Private atomic candidate used by bounded archive replay. Individual creator
/// inputs can be staged across polls without advancing the committed boundary.
#[derive(Debug)]
pub(crate) struct LivingReplayBoundaryV2 {
    candidate: LivingGalaxyStateV2,
    events: LivingPendingEventsV2,
    records: Vec<LivingRecordedRevisionV2>,
    tick: LivingTickV2,
    parent: Option<LivingRevisionIdV2>,
}

/// Receipt-bound creator recovery derived from an entire accepted revision.
/// This ephemeral proof is excluded from canonical state, archives and wires.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LivingCreatorRecoveryV2 {
    revision: LivingRevisionIdV2,
    actor: LivingEntityIdV2,
    world: LivingEntityIdV2,
    receipt_digest: super::ids::LivingReceiptDigestV2,
}
impl LivingCreatorRecoveryV2 {
    /// Exact accepted revision that changed the existing dormant civilization.
    pub const fn revision(&self) -> LivingRevisionIdV2 {
        self.revision
    }
    /// Existing civilization recovered by that revision.
    pub const fn actor(&self) -> LivingEntityIdV2 {
        self.actor
    }
    /// Lowest-ID owned colony at the final committed boundary.
    pub const fn world(&self) -> LivingEntityIdV2 {
        self.world
    }
    /// Exact ordinary receipt this recovery proof belongs to.
    pub const fn receipt_digest(&self) -> super::ids::LivingReceiptDigestV2 {
        self.receipt_digest
    }
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct LivingCreatorRecoveryCandidateV2 {
    revision: LivingRevisionIdV2,
    actor: LivingEntityIdV2,
}

/// Progress of the staged boundary preparation. This partial executor never
/// disguises its remaining synchronous phases as bounded record visits.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum LivingStepProgressV2 {
    /// State copying or source hashing has more actual records to visit.
    Pending,
    /// The bounded preparation finished; remaining phases are synchronous.
    NeedsSynchronousCompletion,
    /// A delegated creator operation still requires explicit synchronous work.
    NeedsSynchronousCreatorOperation,
}

/// Job control errors do not mutate or fault the resident authority.
#[derive(Clone, Debug, Eq, PartialEq, thiserror::Error)]
pub enum LivingStepJobErrorV2 {
    /// Poll budget was invalid.
    #[error("Living step poll requires 1..=1024 record visits")]
    Budget,
    /// The private job was cancelled.
    #[error("Living step job was cancelled")]
    Cancelled,
    /// Bounded preparation is incomplete.
    #[error("Living step preparation is incomplete")]
    NotReady,
    /// The capture no longer names the authority's exact publication.
    #[error("Living step publication is stale")]
    Stale,
    /// The job failed; only its opaque failure result can latch the authority.
    #[error("Living step preparation failed: {0}")]
    Fault(LivingDeterministicFaultV2),
}

/// An executor-created success or failure. Neither token exposes constructors.
#[derive(Debug)]
pub enum LivingStepPreparationV2 {
    /// Fully computed and sealed success, still unpublished.
    Ready(Box<LivingPreparedBoundaryV2>),
    /// Deterministic failure, still unlatching until its capture is checked.
    Failed(Box<LivingPreparedStepFailureV2>),
}

/// Complete unpublished owners. Fields intentionally remain private.
#[derive(Debug)]
pub struct LivingPreparedBoundaryV2 {
    owners: Option<LivingPreparedBoundaryOwnersV2>,
}
#[derive(Debug)]
struct LivingPreparedBoundaryOwnersV2 {
    capture: OwnerSlot<LivingBoundaryCaptureV2>,
    state: OwnerSlot<Arc<LivingGalaxyStateV2>>,
    revisions: OwnerSlot<Arc<Vec<LivingRevisionV2>>>,
    revision_modes: OwnerSlot<Arc<Vec<LivingCommandModeV2>>>,
    queue: OwnerSlot<Arc<Vec<QueuedCommandV2>>>,
    committed_revision: Option<LivingRevisionIdV2>,
    receipt: OwnerSlot<LivingTickReceiptV2>,
    decision: OwnerSlot<super::decision::LivingDecisionReceiptV2>,
    event_facts: OwnerSlot<super::event_facts::LivingEventFactsReceiptV2>,
    creator_recoveries: Vec<LivingCreatorRecoveryV2>,
    corridor_restrictions: Vec<super::event_facts::LivingCorridorRestrictionV2>,
}

/// A deterministic failure that can only originate inside boundary execution.
#[derive(Debug)]
pub struct LivingPreparedStepFailureV2 {
    owners: Option<LivingPreparedFailureOwnersV2>,
}
#[derive(Debug)]
struct LivingPreparedFailureOwnersV2 {
    capture: OwnerSlot<LivingBoundaryCaptureV2>,
    fault: LivingDeterministicFaultV2,
}
impl LivingPreparedStepFailureV2 {
    /// Inspect the authentic failure without gaining a fault-construction path.
    pub fn fault(&self) -> &LivingDeterministicFaultV2 {
        &self.owners.as_ref().expect("unconsumed failure").fault
    }
}

/// A rejected commit retains its candidate/failure ownership for explicit
/// reclamation outside the publication operation; inspect the reason first.
pub struct LivingStepCommitRejectionV2 {
    reason: LivingStepJobErrorV2,
    _preparation: Option<LivingStepPreparationV2>,
}
impl LivingStepCommitRejectionV2 {
    /// Typed publication refusal; no resident field changed.
    pub fn reason(&self) -> &LivingStepJobErrorV2 {
        &self.reason
    }
}
impl std::fmt::Debug for LivingStepCommitRejectionV2 {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("LivingStepCommitRejectionV2")
            .field("reason", &self.reason)
            .finish_non_exhaustive()
    }
}
impl std::fmt::Display for LivingStepCommitRejectionV2 {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.reason.fmt(formatter)
    }
}
impl std::error::Error for LivingStepCommitRejectionV2 {}

/// Retired publication ownership returned by a successful atomic commit.
/// Dropping it can traverse large collections; reclaim outside the publication
/// operation, or move it into counted retirement with `into_retirement`.
#[must_use = "retain retired owners until outside the publication operation"]
#[derive(Debug)]
pub struct LivingRetiredBoundaryV2 {
    _capture: Option<LivingBoundaryCaptureV2>,
    _token: Option<LivingStepPreparationV2>,
    _receipt: Option<LivingTickReceiptV2>,
    _decision: Option<super::decision::LivingDecisionReceiptV2>,
    _event_facts: Option<super::event_facts::LivingEventFactsReceiptV2>,
    _creator_recoveries: Vec<LivingCreatorRecoveryV2>,
    _corridor_restrictions: Vec<super::event_facts::LivingCorridorRestrictionV2>,
}

// This private marker records completed authority phases, including empty ones.
// It cannot be inferred from events, arrival IDs, or collection cardinality.
enum LivingCompletedStepPhasesV2 {
    Scheduled { arrivals: Vec<LivingEntityIdV2> },
    ObservationsComplete,
    DiplomacyComplete,
    DispatchComplete,
}

/// Partial staged executor: bounded copying, hashing, supported creator edits,
/// and scheduled phases through diplomacy, plus dispatch on non-intent ticks.
/// Due intents and final sealing use an explicitly synchronous completion API.
/// Resident transport polls this job; the synchronous suffix remains explicit.
pub struct LivingStepJobV2 {
    capture: OwnerSlot<LivingBoundaryCaptureV2>,
    copy: Option<super::step_job::LivingStateCopyJobV2>,
    hash: Option<super::step_job::LivingStateHashJobV2>,
    candidate: Option<LivingGalaxyStateV2>,
    source_digest: Option<super::ids::LivingStateDigestV2>,
    creator: Option<Box<LivingQueuedCreatorJobV2>>,
    scheduled: Option<Box<super::step_job::LivingScheduledPhasesJobV2>>,
    conflict: Option<Box<super::step_job::LivingConflictJobV2<'static>>>,
    production: Option<Box<super::step_job::LivingProductionJobV2<'static>>>,
    observation: Option<Box<super::step_job::LivingObservationJobV2>>,
    observation_events: Option<LivingPendingEventsV2>,
    diplomacy: Option<Box<super::step_job::LivingDiplomacyJobV2>>,
    dispatch: Option<Box<super::step_job::LivingDispatchJobV2<'static>>>,
    completed_output: Option<
        Box<(
            LivingGalaxyStateV2,
            LivingPendingEventsV2,
            LivingCompletedStepPhasesV2,
        )>,
    >,
    fault: Option<LivingDeterministicFaultV2>,
    cancelled: bool,
    delivered: bool,
    retirement: super::step_job::PhaseRetirementLane<'static>,
    last_poll_visits: usize,
    total_visits: u64,
}
impl LivingStepJobV2 {
    /// Exact publication fence for resident cancellation/restart before polling.
    pub fn is_current(&self, authority: &LivingGalaxyAuthorityV2) -> bool {
        !self.cancelled
            && !self.delivered
            && self.capture.is_present()
            && authority.boundary_capture_is_current(&self.capture)
    }
    fn needs_synchronous_completion(&self) -> bool {
        self.retirement.is_empty()
            && self.completed_output.as_ref().is_some_and(|output| {
                matches!(output.2, LivingCompletedStepPhasesV2::DispatchComplete)
                    || (matches!(output.2, LivingCompletedStepPhasesV2::DiplomacyComplete)
                        && (self.capture.state.tick.0 + 1).is_multiple_of(INTENT_PERIOD_TICKS))
            })
    }
    /// Cancel without consuming accepted commands or changing resident truth.
    pub fn cancel(&mut self) {
        self.cancelled = true;
    }
    /// Actual source and nested record visits used in the preceding poll.
    pub const fn last_poll_visits(&self) -> usize {
        self.last_poll_visits
    }
    /// Actual preparation visits across polls.
    pub const fn total_visits(&self) -> u64 {
        self.total_visits
    }
    /// Poll private preparation and supported creator/lifecycle/arrival phases
    /// under one shared record budget, through diplomacy and non-intent dispatch.
    /// Due intents and final sealing remain explicitly synchronous.
    pub fn poll(&mut self, budget: usize) -> Result<LivingStepProgressV2, LivingStepJobErrorV2> {
        use super::step_job::{LivingLifecycleJobErrorV2, LivingLifecycleProgressV2};
        self.last_poll_visits = 0;
        if self.cancelled {
            return Err(LivingStepJobErrorV2::Cancelled);
        }
        if budget == 0 || budget > 1024 {
            return Err(LivingStepJobErrorV2::Budget);
        }
        if self.delivered || !self.capture.is_present() {
            return Err(LivingStepJobErrorV2::NotReady);
        }
        if let Some(fault) = &self.fault {
            return Err(LivingStepJobErrorV2::Fault(fault.clone()));
        }
        while self.last_poll_visits < budget {
            if self.retirement.visit() {
                self.last_poll_visits += 1;
                self.total_visits += 1;
                continue;
            }
            if self.needs_synchronous_completion() {
                return Ok(LivingStepProgressV2::NeedsSynchronousCompletion);
            }
            if let Some(output) = &mut self.completed_output {
                let tick = LivingTickV2(self.capture.state.tick.0 + 1);
                if !matches!(output.2, LivingCompletedStepPhasesV2::DiplomacyComplete)
                    && !tick.0.is_multiple_of(DIPLOMACY_PERIOD_TICKS)
                {
                    output.2 = LivingCompletedStepPhasesV2::DiplomacyComplete;
                } else {
                    let state = std::mem::take(&mut output.0);
                    let events = std::mem::take(&mut output.1);
                    if matches!(output.2, LivingCompletedStepPhasesV2::DiplomacyComplete) {
                        self.dispatch =
                            Some(Box::new(super::step_job::LivingDispatchJobV2::begin_owned(
                                Arc::clone(&self.capture.catalog),
                                state,
                                events,
                                tick,
                                self.capture.genesis_seed,
                                self.capture.branch,
                            )));
                    } else {
                        self.diplomacy = Some(Box::new(
                            super::step_job::LivingDiplomacyJobV2::begin(state, events, tick),
                        ));
                    }
                    self.retirement.start(self.completed_output.take());
                }
                self.last_poll_visits += 1;
                self.total_visits += 1;
                continue;
            }
            let remaining = budget - self.last_poll_visits;
            let (result, visits) = if let Some(copy) = &mut self.copy {
                (copy.poll(remaining), copy.last_poll_visits())
            } else if let Some(hash) = &mut self.hash {
                (hash.poll(remaining), hash.last_poll_visits())
            } else if let Some(scheduled) = &mut self.scheduled {
                (scheduled.poll(remaining), scheduled.last_poll_visits())
            } else if let Some(conflict) = &mut self.conflict {
                (conflict.poll(remaining), conflict.last_poll_visits())
            } else if let Some(production) = &mut self.production {
                (production.poll(remaining), production.last_poll_visits())
            } else if let Some(observation) = &mut self.observation {
                (observation.poll(remaining), observation.last_poll_visits())
            } else if let Some(diplomacy) = &mut self.diplomacy {
                (diplomacy.poll(remaining), diplomacy.last_poll_visits())
            } else if let Some(dispatch) = &mut self.dispatch {
                (dispatch.poll(remaining), dispatch.last_poll_visits())
            } else {
                if self.creator.is_none() {
                    let state = self
                        .candidate
                        .take()
                        .ok_or(LivingStepJobErrorV2::NotReady)?;
                    self.creator = Some(Box::new(LivingQueuedCreatorJobV2::begin(
                        state,
                        Arc::clone(&self.capture.queue),
                        Arc::clone(&self.capture.catalog),
                    )));
                }
                let creator = self.creator.as_mut().expect("creator was initialized");
                let result = creator.poll(remaining);
                self.last_poll_visits += creator.last_poll_visits;
                self.total_visits += creator.last_poll_visits as u64;
                match result {
                    Ok(LivingStepProgressV2::NeedsSynchronousCompletion) => {
                        if creator.last_poll_visits != 0 {
                            continue;
                        }
                        self.last_poll_visits += 1;
                        self.total_visits += 1;
                        let (state, events) = creator
                            .take_ready_result()
                            .map_err(LivingStepJobErrorV2::Fault)?;
                        self.retirement.start(self.creator.take());
                        let tick = LivingTickV2(self.capture.state.tick.0 + 1);
                        self.scheduled = Some(Box::new(
                            super::step_job::LivingScheduledPhasesJobV2::begin(state, events, tick),
                        ));
                        continue;
                    }
                    Ok(progress) => return Ok(progress),
                    Err(fault) => {
                        self.fault = Some(fault.clone());
                        return Err(LivingStepJobErrorV2::Fault(fault));
                    }
                }
            };
            self.last_poll_visits += visits;
            self.total_visits += visits as u64;
            let progress = match result {
                Ok(progress) => progress,
                Err(LivingLifecycleJobErrorV2::Fault(fault)) => {
                    self.fault = Some(fault.clone());
                    return Err(LivingStepJobErrorV2::Fault(fault));
                }
                Err(_) => {
                    self.fault = Some(LivingDeterministicFaultV2::Invariant);
                    return Err(LivingStepJobErrorV2::Fault(
                        LivingDeterministicFaultV2::Invariant,
                    ));
                }
            };
            if progress == LivingLifecycleProgressV2::Pending {
                return Ok(LivingStepProgressV2::Pending);
            }
            if visits != 0 {
                continue;
            }
            self.last_poll_visits += 1;
            self.total_visits += 1;
            if let Some(copy) = &mut self.copy {
                self.candidate = Some(copy.take_ready_result().map_err(|_| {
                    LivingStepJobErrorV2::Fault(LivingDeterministicFaultV2::Invariant)
                })?);

                self.retirement.start(self.copy.take());
            } else if let Some(hash) = &mut self.hash {
                self.source_digest = Some(hash.take_ready_result().map_err(|_| {
                    LivingStepJobErrorV2::Fault(LivingDeterministicFaultV2::Invariant)
                })?);

                self.retirement.start(self.hash.take());
            } else if let Some(scheduled) = &mut self.scheduled {
                let (state, events, arrivals) = scheduled.take_ready_result().map_err(|_| {
                    LivingStepJobErrorV2::Fault(LivingDeterministicFaultV2::Invariant)
                })?;
                self.conflict = Some(Box::new(super::step_job::LivingConflictJobV2::begin_owned(
                    Arc::clone(&self.capture.catalog),
                    state,
                    events,
                    LivingTickV2(self.capture.state.tick.0 + 1),
                    self.capture.genesis_seed,
                    arrivals,
                )));

                self.retirement.start(self.scheduled.take());
            } else if let Some(conflict) = &mut self.conflict {
                let (state, events) = conflict.take_ready_result().map_err(|_| {
                    LivingStepJobErrorV2::Fault(LivingDeterministicFaultV2::Invariant)
                })?;
                self.production = Some(Box::new(
                    super::step_job::LivingProductionJobV2::begin_owned(
                        Arc::clone(&self.capture.catalog),
                        state,
                        events,
                        LivingTickV2(self.capture.state.tick.0 + 1),
                        self.capture.genesis_seed,
                        self.capture.branch,
                    ),
                ));

                self.retirement.start(self.conflict.take());
            } else if let Some(production) = &mut self.production {
                let (state, events) = production.take_ready_result().map_err(|_| {
                    LivingStepJobErrorV2::Fault(LivingDeterministicFaultV2::Invariant)
                })?;
                self.observation_events = Some(events);
                self.observation = Some(Box::new(super::step_job::LivingObservationJobV2::begin(
                    state,
                    LivingTickV2(self.capture.state.tick.0 + 1),
                )));

                self.retirement.start(self.production.take());
            } else if let Some(observation) = &mut self.observation {
                let state = observation.take_ready_result().map_err(|_| {
                    LivingStepJobErrorV2::Fault(LivingDeterministicFaultV2::Invariant)
                })?;
                let events = self
                    .observation_events
                    .take()
                    .ok_or(LivingStepJobErrorV2::NotReady)?;
                self.completed_output = Some(Box::new((
                    state,
                    events,
                    LivingCompletedStepPhasesV2::ObservationsComplete,
                )));

                self.retirement.start(self.observation.take());
            } else if let Some(diplomacy) = &mut self.diplomacy {
                let (state, events) = diplomacy.take_ready_result().map_err(|_| {
                    LivingStepJobErrorV2::Fault(LivingDeterministicFaultV2::Invariant)
                })?;
                self.completed_output = Some(Box::new((
                    state,
                    events,
                    LivingCompletedStepPhasesV2::DiplomacyComplete,
                )));

                self.retirement.start(self.diplomacy.take());
            } else if let Some(dispatch) = &mut self.dispatch {
                let (state, events) = dispatch.take_ready_result().map_err(|_| {
                    LivingStepJobErrorV2::Fault(LivingDeterministicFaultV2::Invariant)
                })?;
                self.completed_output = Some(Box::new((
                    state,
                    events,
                    LivingCompletedStepPhasesV2::DispatchComplete,
                )));
                self.retirement.start(self.dispatch.take());
            }
        }
        Ok(if self.needs_synchronous_completion() {
            LivingStepProgressV2::NeedsSynchronousCompletion
        } else {
            LivingStepProgressV2::Pending
        })
    }
    fn capture_for_preparation(&mut self) -> OwnerSlot<LivingBoundaryCaptureV2> {
        // Keep the original fixed source anchors until executor retirement.
        // A failed active child may still own those same source/catalog Arcs.
        // This transitional synchronous suffix clones only five Arc handles.
        self.delivered = true;
        self.capture.clone()
    }
    /// Explicit transitional completion: phases, validation, receipt sealing and
    /// journal copying are synchronous. This method is not a bounded poll.
    pub fn finish_synchronously(
        mut self,
        authority: &LivingGalaxyAuthorityV2,
    ) -> Result<LivingStepPreparationV2, LivingStepJobErrorV2> {
        self.take_synchronous_preparation(authority)
    }
    /// Transitional synchronous suffix with borrowed, one-time extraction.
    /// Retains the original executor and child Boxes for explicit retirement.
    /// This computes the same synchronous suffix and is not a bounded poll.
    pub fn take_synchronous_preparation(
        &mut self,
        authority: &LivingGalaxyAuthorityV2,
    ) -> Result<LivingStepPreparationV2, LivingStepJobErrorV2> {
        if self.delivered || !self.capture.is_present() {
            return Err(LivingStepJobErrorV2::NotReady);
        }
        if self.cancelled {
            return Err(LivingStepJobErrorV2::Cancelled);
        }
        if !authority.boundary_capture_is_current(&self.capture) {
            return Err(LivingStepJobErrorV2::Stale);
        }
        if let Some(fault) = self.fault.take() {
            return Ok(LivingStepPreparationV2::Failed(Box::new(
                LivingPreparedStepFailureV2 {
                    owners: Some(LivingPreparedFailureOwnersV2 {
                        capture: self.capture_for_preparation(),
                        fault,
                    }),
                },
            )));
        }
        let source_digest = self.source_digest.ok_or(LivingStepJobErrorV2::NotReady)?;
        if self.completed_output.is_some() && !self.needs_synchronous_completion() {
            return Err(LivingStepJobErrorV2::NotReady);
        }
        let (candidate, events, completed) = if let Some(output) = &mut self.completed_output {
            let candidate = std::mem::take(&mut output.0);
            let events = std::mem::take(&mut output.1);
            let completed =
                std::mem::replace(&mut output.2, LivingCompletedStepPhasesV2::DispatchComplete);
            if !matches!(
                completed,
                LivingCompletedStepPhasesV2::DiplomacyComplete
                    | LivingCompletedStepPhasesV2::DispatchComplete
            ) {
                return Err(LivingStepJobErrorV2::NotReady);
            }
            (candidate, events, completed)
        } else {
            let creator = self
                .creator
                .as_mut()
                .ok_or(LivingStepJobErrorV2::NotReady)?;
            if !matches!(creator.stage, QueuedCreatorStage::NeedsSynchronous) {
                return Err(LivingStepJobErrorV2::NotReady);
            }
            let (candidate, events) = match creator.take_synchronous_result() {
                Ok(output) => output,
                Err(fault) => {
                    return Ok(LivingStepPreparationV2::Failed(Box::new(
                        LivingPreparedStepFailureV2 {
                            owners: Some(LivingPreparedFailureOwnersV2 {
                                capture: self.capture_for_preparation(),
                                fault,
                            }),
                        },
                    )));
                }
            };
            // The fallback is explicitly synchronous: it may also drain these
            // scheduled phases, never counting a complete phase as one poll visit.
            let mut scheduled = super::step_job::LivingScheduledPhasesJobV2::begin(
                candidate,
                events,
                LivingTickV2(self.capture.state.tick.0 + 1),
            );
            loop {
                match scheduled.poll(1024) {
                    Ok(super::step_job::LivingLifecycleProgressV2::Pending) => {}
                    Ok(super::step_job::LivingLifecycleProgressV2::Ready) => break,
                    Err(error) => {
                        let fault = match error {
                            super::step_job::LivingLifecycleJobErrorV2::Fault(fault) => fault,
                            _ => LivingDeterministicFaultV2::Invariant,
                        };
                        return Ok(LivingStepPreparationV2::Failed(Box::new(
                            LivingPreparedStepFailureV2 {
                                owners: Some(LivingPreparedFailureOwnersV2 {
                                    capture: self
                                        .capture
                                        .take()
                                        .expect("unconsumed capture")
                                        .into(),
                                    fault,
                                }),
                            },
                        )));
                    }
                }
            }
            let (candidate, events, arrivals) = scheduled
                .finish()
                .map_err(|_| LivingStepJobErrorV2::Fault(LivingDeterministicFaultV2::Invariant))?;
            (
                candidate,
                events,
                LivingCompletedStepPhasesV2::Scheduled { arrivals },
            )
        };
        let applied = self
            .capture
            .queue
            .iter()
            .map(|queued| queued.revision.id)
            .collect();
        let (candidate, receipt, decision, event_facts, creator_recoveries, corridor_restrictions) =
            match authority.run_boundary_candidate_with_source_digest(
                candidate,
                events,
                Some(applied),
                Some(source_digest),
                Some(completed),
            ) {
                Ok(output) => output,
                Err(fault) => {
                    return Ok(LivingStepPreparationV2::Failed(Box::new(
                        LivingPreparedStepFailureV2 {
                            owners: Some(LivingPreparedFailureOwnersV2 {
                                capture: self.capture_for_preparation(),
                                fault,
                            }),
                        },
                    )));
                }
            };
        let mut revisions = self.capture.revisions.as_ref().clone();
        revisions.extend(
            self.capture
                .queue
                .iter()
                .map(|queued| queued.revision.as_ref().clone()),
        );
        let mut modes = self.capture.revision_modes.as_ref().clone();
        modes.extend(std::iter::repeat_n(
            LivingCommandModeV2::Running,
            self.capture.queue.len(),
        ));
        let committed_revision = self
            .capture
            .queue
            .last()
            .map(|queued| queued.revision.id)
            .or(self.capture.committed_revision);
        Ok(LivingStepPreparationV2::Ready(Box::new(
            LivingPreparedBoundaryV2 {
                owners: Some(LivingPreparedBoundaryOwnersV2 {
                    capture: self.capture_for_preparation(),
                    state: Arc::new(candidate).into(),
                    revisions: Arc::new(revisions).into(),
                    revision_modes: Arc::new(modes).into(),
                    queue: Arc::new(Vec::new()).into(),
                    committed_revision,
                    receipt: receipt.into(),
                    decision: decision.into(),
                    event_facts: event_facts.into(),
                    creator_recoveries,
                    corridor_restrictions,
                }),
            },
        )))
    }
}

// A phase-one cursor for the core operations. Delegated operations explicitly
// stop for synchronous completion until their own cascade cursors exist.
#[derive(Clone, Copy)]
enum QueuedCreatorStage {
    Revision,
    DormantCivilizations,
    OwnedColonies,
    Declarations,
    Operation,
    Insertion,
    EmitCreator,
    FinishRevision,
    NeedsSynchronous,
    Ready,
}
enum QueuedCreatorInsertion {
    System(super::step_job::LivingSortedInsertionJobV2<LivingSystemV2>),
    Star(super::step_job::LivingSortedInsertionJobV2<LivingStarV2>),
    World(super::step_job::LivingSortedInsertionJobV2<LivingWorldV2>),
    Lane(super::step_job::LivingSortedInsertionJobV2<LivingLaneV2>),
    Civilization(super::step_job::LivingSortedInsertionJobV2<LivingCivilizationV2>),
    Deposit(super::step_job::LivingSortedInsertionJobV2<super::model::LivingDepositV2>),
}
struct LivingQueuedCreatorJobV2 {
    state: LivingGalaxyStateV2,
    queue: OwnerSlot<Arc<Vec<QueuedCommandV2>>>,
    catalog: OwnerSlot<Arc<ValidatedLivingCatalogPackV2>>,
    events: LivingPendingEventsV2,
    stage: QueuedCreatorStage,
    revision_index: usize,
    operation_index: usize,
    index: usize,
    locals: std::collections::BTreeMap<u16, LivingEntityIdV2>,
    dormant: std::collections::BTreeSet<LivingEntityIdV2>,
    insertion: Option<QueuedCreatorInsertion>,
    retirement: super::step_job::PhaseRetirementLane<'static>,
    last_poll_visits: usize,
    delivered: bool,
}
impl LivingQueuedCreatorJobV2 {
    fn begin(
        state: LivingGalaxyStateV2,
        queue: Arc<Vec<QueuedCommandV2>>,
        catalog: Arc<ValidatedLivingCatalogPackV2>,
    ) -> Self {
        Self {
            delivered: false,
            state,
            queue: queue.into(),
            catalog: catalog.into(),
            events: LivingPendingEventsV2::new(),
            stage: QueuedCreatorStage::Revision,
            revision_index: 0,
            operation_index: 0,
            index: 0,
            locals: std::collections::BTreeMap::new(),
            dormant: std::collections::BTreeSet::new(),
            insertion: None,
            retirement: super::step_job::PhaseRetirementLane::default(),
            last_poll_visits: 0,
        }
    }
    fn poll(&mut self, budget: usize) -> Result<LivingStepProgressV2, LivingDeterministicFaultV2> {
        self.last_poll_visits = 0;
        while self.last_poll_visits < budget {
            if self.retirement.visit() {
                self.last_poll_visits += 1;
                continue;
            }
            match self.stage {
                QueuedCreatorStage::Ready => {
                    return Ok(LivingStepProgressV2::NeedsSynchronousCompletion);
                }
                QueuedCreatorStage::NeedsSynchronous => {
                    return Ok(LivingStepProgressV2::NeedsSynchronousCreatorOperation);
                }
                _ => {}
            }
            match self.visit() {
                Ok(true) => self.last_poll_visits += 1,
                Ok(false) => {}
                Err(fault) => {
                    self.last_poll_visits += 1;
                    let fault = match fault {
                        LivingDeterministicFaultV2::Validation(reason) => {
                            self.replay_fault(LivingCommandRejectionV2::Projection(reason))
                        }
                        LivingDeterministicFaultV2::Wire(reason) => {
                            self.replay_fault(LivingCommandRejectionV2::Projection(reason.into()))
                        }
                        other => other,
                    };
                    return Err(fault);
                }
            }
        }
        Ok(match self.stage {
            QueuedCreatorStage::Ready => LivingStepProgressV2::NeedsSynchronousCompletion,
            QueuedCreatorStage::NeedsSynchronous => {
                LivingStepProgressV2::NeedsSynchronousCreatorOperation
            }
            _ => LivingStepProgressV2::Pending,
        })
    }
    fn resolve(
        &self,
        target: &LivingCommandTargetV2,
    ) -> Result<LivingEntityIdV2, LivingDeterministicFaultV2> {
        match target {
            LivingCommandTargetV2::Existing { id } => Ok(*id),
            LivingCommandTargetV2::Local { local } => self
                .locals
                .get(local)
                .copied()
                .ok_or(LivingDeterministicFaultV2::Invariant),
        }
    }
    fn replay_fault(&self, reason: LivingCommandRejectionV2) -> LivingDeterministicFaultV2 {
        LivingDeterministicFaultV2::Replay {
            accepted_sequence: self.queue[self.revision_index].accepted.accepted_sequence,
            reason: Box::new(reason),
        }
    }
    fn visit(&mut self) -> Result<bool, LivingDeterministicFaultV2> {
        use super::model::LivingCivilizationStatusV2;
        use QueuedCreatorStage::*;
        match self.stage {
            Revision => {
                if self.revision_index == self.queue.len() {
                    self.stage = Ready;
                    return Ok(false);
                }
                self.operation_index = 0;
                self.index = 0;
                self.stage = DormantCivilizations;
                return Ok(true);
            }
            DormantCivilizations => {
                if let Some(row) = self.state.civilizations.as_slice().get(self.index) {
                    self.index += 1;
                    if row.status == LivingCivilizationStatusV2::Dormant {
                        self.dormant.insert(row.id);
                    }
                    return Ok(true);
                }
                self.index = 0;
                self.stage = OwnedColonies;
            }
            OwnedColonies => {
                if let Some(row) = self.state.colonies.as_slice().get(self.index) {
                    self.index += 1;
                    self.dormant.remove(&row.owner);
                    return Ok(true);
                }
                self.index = 0;
                self.stage = Declarations;
            }
            Declarations => {
                let revision = &self.queue[self.revision_index].revision;
                if let Some(operation) = revision.command.operations().get(self.index) {
                    self.index += 1;
                    if let (Some(local), Some(kind)) =
                        (operation.declared_local(), operation.created_kind())
                    {
                        self.locals.insert(
                            local,
                            super::ids::creator_entity_id_v2(revision.id, kind.ordinal(), local),
                        );
                    }
                    return Ok(true);
                }
                self.stage = Operation;
            }
            Operation => {
                let revision = &self.queue[self.revision_index].revision;
                let Some(operation) = revision.command.operations().get(self.operation_index)
                else {
                    self.stage = EmitCreator;
                    return Ok(false);
                };
                use super::step_job::LivingSortedInsertionJobV2 as Insert;
                let insertion = match operation {
                    LivingCreatorOperationV2::CreateSystem { local, name } => {
                        Some(QueuedCreatorInsertion::System(Insert::begin(
                            std::mem::take(&mut self.state.systems),
                            LivingSystemV2 {
                                id: self.locals[local],
                                name: name.clone(),
                            },
                        )))
                    }
                    LivingCreatorOperationV2::CreateStar {
                        local,
                        system,
                        archetype,
                        name,
                    } => {
                        let row = LivingStarV2 {
                            id: self.locals[local],
                            system: self.resolve(system)?,
                            archetype: archetype.clone(),
                            name: name.clone(),
                        };
                        Some(QueuedCreatorInsertion::Star(Insert::begin(
                            std::mem::take(&mut self.state.stars),
                            row,
                        )))
                    }
                    LivingCreatorOperationV2::CreateWorld {
                        local,
                        system,
                        archetype,
                        name,
                        owner,
                        inventory,
                    } => {
                        let row = LivingWorldV2 {
                            id: self.locals[local],
                            system: self.resolve(system)?,
                            archetype: archetype.clone(),
                            name: name.clone(),
                            owner: owner
                                .as_ref()
                                .map(|target| self.resolve(target))
                                .transpose()?,
                            inventory: *inventory,
                        };
                        Some(QueuedCreatorInsertion::World(Insert::begin(
                            std::mem::take(&mut self.state.worlds),
                            row,
                        )))
                    }
                    LivingCreatorOperationV2::CreateLane {
                        local,
                        system_a,
                        system_b,
                        distance_units,
                    } => {
                        let a = self.resolve(system_a)?;
                        let b = self.resolve(system_b)?;
                        Some(QueuedCreatorInsertion::Lane(Insert::begin(
                            std::mem::take(&mut self.state.lanes),
                            LivingLaneV2 {
                                id: self.locals[local],
                                system_a: a.min(b),
                                system_b: a.max(b),
                                distance_units: *distance_units,
                            },
                        )))
                    }
                    LivingCreatorOperationV2::CreateCivilization {
                        local,
                        name,
                        policy,
                        status,
                    } => Some(QueuedCreatorInsertion::Civilization(Insert::begin(
                        std::mem::take(&mut self.state.civilizations),
                        LivingCivilizationV2 {
                            id: self.locals[local],
                            name: name.clone(),
                            policy: policy.clone(),
                            status: *status,
                        },
                    ))),
                    LivingCreatorOperationV2::CreateDeposit {
                        local,
                        world,
                        resource,
                        remaining_units,
                    } => {
                        let row = super::model::LivingDepositV2 {
                            id: self.locals[local],
                            world: self.resolve(world)?,
                            resource: *resource,
                            remaining_units: *remaining_units,
                        };
                        Some(QueuedCreatorInsertion::Deposit(Insert::begin(
                            std::mem::take(&mut self.state.deposits),
                            row,
                        )))
                    }
                    LivingCreatorOperationV2::SetWorldInventory { world, inventory } => {
                        let id = self.resolve(world)?;
                        edit(&mut self.state.worlds, id, self.operation_index, "world")
                            .map_err(|reason| LivingDeterministicFaultV2::Replay {
                                accepted_sequence: revision.ordinal,
                                reason: Box::new(reason),
                            })?
                            .inventory = *inventory;
                        None
                    }
                    LivingCreatorOperationV2::SetDepositRemaining {
                        deposit,
                        remaining_units,
                    } => {
                        let id = self.resolve(deposit)?;
                        edit(
                            &mut self.state.deposits,
                            id,
                            self.operation_index,
                            "deposit",
                        )
                        .map_err(|reason| LivingDeterministicFaultV2::Replay {
                            accepted_sequence: revision.ordinal,
                            reason: Box::new(reason),
                        })?
                        .remaining_units = *remaining_units;
                        None
                    }
                    LivingCreatorOperationV2::SetCivilizationPolicy {
                        civilization,
                        policy,
                    } => {
                        let id = self.resolve(civilization)?;
                        edit(
                            &mut self.state.civilizations,
                            id,
                            self.operation_index,
                            "civilization",
                        )
                        .map_err(|reason| LivingDeterministicFaultV2::Replay {
                            accepted_sequence: revision.ordinal,
                            reason: Box::new(reason),
                        })?
                        .policy = policy.clone();
                        None
                    }
                    _ => {
                        self.stage = NeedsSynchronous;
                        return Ok(true);
                    }
                };
                if let Some(insertion) = insertion {
                    self.insertion = Some(insertion);
                    self.stage = Insertion;
                } else {
                    self.operation_index += 1;
                }
                return Ok(true);
            }
            Insertion => {
                let Some(insertion) = &mut self.insertion else {
                    return Err(LivingDeterministicFaultV2::Invariant);
                };
                let ready = match insertion {
                    QueuedCreatorInsertion::System(job) => job.ready(),
                    QueuedCreatorInsertion::Star(job) => job.ready(),
                    QueuedCreatorInsertion::World(job) => job.ready(),
                    QueuedCreatorInsertion::Lane(job) => job.ready(),
                    QueuedCreatorInsertion::Civilization(job) => job.ready(),
                    QueuedCreatorInsertion::Deposit(job) => job.ready(),
                };
                if !ready {
                    return match insertion {
                        QueuedCreatorInsertion::System(job) => job.visit(),
                        QueuedCreatorInsertion::Star(job) => job.visit(),
                        QueuedCreatorInsertion::World(job) => job.visit(),
                        QueuedCreatorInsertion::Lane(job) => job.visit(),
                        QueuedCreatorInsertion::Civilization(job) => job.visit(),
                        QueuedCreatorInsertion::Deposit(job) => job.visit(),
                    };
                }
                match insertion {
                    QueuedCreatorInsertion::System(job) => {
                        self.state.systems = job
                            .take_ready_result()
                            .map_err(|_| LivingDeterministicFaultV2::Invariant)?
                    }
                    QueuedCreatorInsertion::Star(job) => {
                        self.state.stars = job
                            .take_ready_result()
                            .map_err(|_| LivingDeterministicFaultV2::Invariant)?
                    }
                    QueuedCreatorInsertion::World(job) => {
                        self.state.worlds = job
                            .take_ready_result()
                            .map_err(|_| LivingDeterministicFaultV2::Invariant)?
                    }
                    QueuedCreatorInsertion::Lane(job) => {
                        self.state.lanes = job
                            .take_ready_result()
                            .map_err(|_| LivingDeterministicFaultV2::Invariant)?
                    }
                    QueuedCreatorInsertion::Civilization(job) => {
                        self.state.civilizations = job
                            .take_ready_result()
                            .map_err(|_| LivingDeterministicFaultV2::Invariant)?
                    }
                    QueuedCreatorInsertion::Deposit(job) => {
                        self.state.deposits = job
                            .take_ready_result()
                            .map_err(|_| LivingDeterministicFaultV2::Invariant)?
                    }
                }
                self.retirement.start(self.insertion.take());
                self.operation_index += 1;
                self.stage = Operation;
                return Ok(true);
            }
            EmitCreator => {
                let revision = &self.queue[self.revision_index].revision;
                self.events.record(
                    LivingEventProvenanceV2::Creator {
                        revision: revision.id,
                    },
                    LivingEventKindV2::CreatorIntervention,
                )?;
                self.state.accepted_sequence = revision.ordinal;
                // Core operations above cannot recover an existing dormant
                // civilization; colony/status-changing operations stop explicitly.
                self.stage = FinishRevision;
                return Ok(true);
            }
            FinishRevision => {
                if self.locals.pop_first().is_some() || self.dormant.pop_first().is_some() {
                    return Ok(true);
                }
                self.revision_index += 1;
                self.stage = Revision;
                return Ok(true);
            }
            Ready | NeedsSynchronous => {}
        }
        Ok(false)
    }
    fn take_ready_result(
        &mut self,
    ) -> Result<(LivingGalaxyStateV2, LivingPendingEventsV2), LivingDeterministicFaultV2> {
        if self.delivered
            || !self.retirement.is_empty()
            || !matches!(self.stage, QueuedCreatorStage::Ready)
        {
            return Err(LivingDeterministicFaultV2::Invariant);
        }
        self.delivered = true;
        Ok((
            std::mem::take(&mut self.state),
            std::mem::take(&mut self.events),
        ))
    }
    fn take_synchronous_result(
        &mut self,
    ) -> Result<(LivingGalaxyStateV2, LivingPendingEventsV2), LivingDeterministicFaultV2> {
        if self.delivered || !self.retirement.is_empty() {
            return Err(LivingDeterministicFaultV2::Invariant);
        }
        if matches!(self.stage, QueuedCreatorStage::Ready) {
            return self.take_ready_result();
        }
        if !matches!(self.stage, QueuedCreatorStage::NeedsSynchronous) {
            return Err(LivingDeterministicFaultV2::Invariant);
        }
        let revision = &self.queue[self.revision_index].revision;
        let effects = apply_revision_range(
            &mut self.state,
            revision,
            &self.catalog,
            self.operation_index,
        )
        .map_err(|reason| self.replay_fault(reason))?;
        let recovery_candidates = self
            .dormant
            .iter()
            .filter(|actor| {
                self.state.civilizations.as_slice().iter().any(|row| {
                    row.id == **actor
                        && row.status == super::model::LivingCivilizationStatusV2::Active
                }) && self
                    .state
                    .colonies
                    .as_slice()
                    .iter()
                    .any(|row| row.owner == **actor)
            })
            .map(|actor| LivingCreatorRecoveryCandidateV2 {
                revision: revision.id,
                actor: *actor,
            });
        self.events.creator_recoveries.extend(recovery_candidates);
        self.events.record(
            LivingEventProvenanceV2::Creator {
                revision: revision.id,
            },
            LivingEventKindV2::CreatorIntervention,
        )?;
        for kind in effects {
            self.events.record(
                LivingEventProvenanceV2::Creator {
                    revision: revision.id,
                },
                kind,
            )?;
        }
        self.revision_index += 1;
        for queued in &self.queue[self.revision_index..] {
            let (effects, candidates) =
                apply_revision_with_recoveries(&mut self.state, &queued.revision, &self.catalog)
                    .map_err(|reason| LivingDeterministicFaultV2::Replay {
                        accepted_sequence: queued.accepted.accepted_sequence,
                        reason: Box::new(reason),
                    })?;
            self.events.creator_recoveries.extend(candidates);
            self.events.record(
                LivingEventProvenanceV2::Creator {
                    revision: queued.revision.id,
                },
                LivingEventKindV2::CreatorIntervention,
            )?;
            for kind in effects {
                self.events.record(
                    LivingEventProvenanceV2::Creator {
                        revision: queued.revision.id,
                    },
                    kind,
                )?;
            }
        }
        self.delivered = true;
        Ok((
            std::mem::take(&mut self.state),
            std::mem::take(&mut self.events),
        ))
    }
}

type LivingBoundaryOutputV2 = (
    LivingGalaxyStateV2,
    LivingTickReceiptV2,
    super::decision::LivingDecisionReceiptV2,
    super::event_facts::LivingEventFactsReceiptV2,
    Vec<LivingCreatorRecoveryV2>,
    Vec<super::event_facts::LivingCorridorRestrictionV2>,
);

/// The Living Galaxy V2 authority for one active view.
#[derive(Clone, Debug)]
pub struct LivingGalaxyAuthorityV2 {
    catalog: OwnerSlot<Arc<ValidatedLivingCatalogPackV2>>,
    genesis_seed: [u8; 32],
    generator: LivingGenesisGeneratorV2,
    branch: LivingBranchIdV2,
    state: OwnerSlot<Arc<LivingGalaxyStateV2>>,
    committed_revision: Option<LivingRevisionIdV2>,
    revisions: OwnerSlot<Arc<Vec<LivingRevisionV2>>>,
    /// Application timing retained alongside immutable creator inputs.
    /// The canonical revision wire currently does not encode this distinction.
    revision_modes: OwnerSlot<Arc<Vec<LivingCommandModeV2>>>,
    queue: OwnerSlot<Arc<Vec<QueuedCommandV2>>>,
    accepted_sequence_high_water: u64,
    fault: Option<LivingDeterministicFaultV2>,
    /// Genesis state. Replay starts here and never reruns the generator.
    origin: OwnerSlot<Arc<LivingGalaxyStateV2>>,
    /// Branch id assigned at genesis. Forks hash against this parent.
    root_branch: LivingBranchIdV2,
    /// Durable fork allocator. Not a digest input.
    branch_sequence_high_water: u64,
    /// Branch sequence baked into this view before its recorded revisions.
    replay_branch_sequence: u64,
    /// Parent views that remain addressable after a fork.
    archives: Vec<LivingGalaxyAuthorityV2>,
    /// Historical parent boundary from which this branch diverged.
    replay_base: Option<Box<LivingGalaxyAuthorityV2>>,
    /// Retained present while browsing a counterfactual view.
    saved_present: Option<Box<LivingGalaxyAuthorityV2>>,
    redo: Vec<LivingHistoryViewV2>,
    last_receipt: Option<LivingTickReceiptV2>,
    last_decision: Option<super::decision::LivingDecisionReceiptV2>,
    last_event_facts: Option<super::event_facts::LivingEventFactsReceiptV2>,
    last_creator_recoveries: Vec<LivingCreatorRecoveryV2>,
    last_corridor_restrictions: Vec<super::event_facts::LivingCorridorRestrictionV2>,
}

impl LivingGalaxyAuthorityV2 {
    /// Start an authority at a validated genesis.
    pub fn from_genesis(
        catalog: ValidatedLivingCatalogPackV2,
        seed: [u8; 32],
        generator: LivingGenesisGeneratorV2,
        genesis: ValidatedLivingGenesisV2,
    ) -> Result<Self, LivingAuthorityErrorV2> {
        if genesis.catalog_hash() != catalog.catalog_hash() {
            return Err(LivingAuthorityErrorV2::CatalogMismatch);
        }
        let state = Arc::new(genesis.state().clone());
        state.validate(&catalog)?;
        let branch = genesis.root_branch_id(seed);
        Ok(Self {
            branch,
            catalog: Arc::new(catalog).into(),
            genesis_seed: seed,
            generator,
            origin: state.clone().into(),
            root_branch: branch,
            state: state.into(),
            committed_revision: None,
            revisions: Arc::new(Vec::new()).into(),
            revision_modes: Arc::new(Vec::new()).into(),
            queue: Arc::new(Vec::new()).into(),
            accepted_sequence_high_water: 0,
            branch_sequence_high_water: 0,
            replay_branch_sequence: 0,
            archives: Vec::new(),
            replay_base: None,
            saved_present: None,
            redo: Vec::new(),
            last_receipt: None,
            last_decision: None,
            last_event_facts: None,
            last_corridor_restrictions: Vec::new(),
            last_creator_recoveries: Vec::new(),
            fault: None,
        })
    }

    /// Capture the current running boundary inputs without deep cloning.
    /// This is a preparation interface, not publication of a completed job.
    pub fn capture_boundary(&self) -> Result<LivingBoundaryCaptureV2, LivingDeterministicFaultV2> {
        if self.saved_present.is_some() {
            return Err(LivingDeterministicFaultV2::ViewingHistory);
        }
        if let Some(fault) = &self.fault {
            return Err(fault.clone());
        }
        self.state
            .tick
            .0
            .checked_add(1)
            .ok_or(LivingDeterministicFaultV2::Arithmetic)?;
        Ok(self.capture_publication())
    }

    fn capture_publication(&self) -> LivingBoundaryCaptureV2 {
        LivingBoundaryCaptureV2 {
            state: Arc::clone(&self.state).into(),
            catalog: Arc::clone(&self.catalog).into(),
            queue: Arc::clone(&self.queue).into(),
            revisions: Arc::clone(&self.revisions).into(),
            revision_modes: Arc::clone(&self.revision_modes).into(),
            genesis_seed: self.genesis_seed,
            branch: self.branch,
            committed_revision: self.committed_revision,
            accepted_sequence_high_water: self.accepted_sequence_high_water,
            branch_sequence_high_water: self.branch_sequence_high_water,
        }
    }

    /// Begin bounded preparation without mutating or faulting the authority.
    pub fn begin_step(&self) -> Result<LivingStepJobV2, LivingDeterministicFaultV2> {
        if self.saved_present.is_some() {
            return Err(LivingDeterministicFaultV2::ViewingHistory);
        }
        if let Some(fault) = &self.fault {
            return Err(fault.clone());
        }
        let capture = self.capture_publication();
        let fault = self
            .state
            .tick
            .0
            .checked_add(1)
            .is_none()
            .then_some(LivingDeterministicFaultV2::Arithmetic);
        let copy = fault
            .is_none()
            .then(|| super::step_job::LivingStateCopyJobV2::begin(Arc::clone(&self.state)));
        let hash = fault
            .is_none()
            .then(|| super::step_job::LivingStateHashJobV2::begin(Arc::clone(&self.state)));
        Ok(LivingStepJobV2 {
            capture: capture.into(),
            copy,
            hash,
            candidate: None,
            source_digest: None,
            creator: None,
            scheduled: None,
            conflict: None,
            production: None,
            observation: None,
            observation_events: None,
            diplomacy: None,
            dispatch: None,
            completed_output: None,
            fault,
            cancelled: false,
            delivered: false,
            retirement: super::step_job::PhaseRetirementLane::default(),
            last_poll_visits: 0,
            total_visits: 0,
        })
    }

    /// Publish only executor-created success after its exact capture fence.
    /// Retired owners are returned so their destruction is outside this operation.
    pub fn commit_step(
        &mut self,
        mut prepared: Box<LivingPreparedBoundaryV2>,
    ) -> Result<LivingRetiredBoundaryV2, LivingStepCommitRejectionV2> {
        if !self
            .boundary_capture_is_current(&prepared.owners.as_ref().expect("sealed token").capture)
        {
            return Err(LivingStepCommitRejectionV2 {
                reason: LivingStepJobErrorV2::Stale,
                _preparation: Some(LivingStepPreparationV2::Ready(prepared)),
            });
        }
        let owners = prepared.owners.take().expect("sealed token");
        let retired = LivingRetiredBoundaryV2 {
            _capture: Some(owners.capture.into_inner()),
            _token: Some(LivingStepPreparationV2::Ready(prepared)),
            _receipt: self.last_receipt.take(),
            _decision: self.last_decision.take(),
            _event_facts: self.last_event_facts.take(),
            _creator_recoveries: std::mem::take(&mut self.last_creator_recoveries),
            _corridor_restrictions: std::mem::take(&mut self.last_corridor_restrictions),
        };
        self.state = owners.state;
        self.revisions = owners.revisions;
        self.revision_modes = owners.revision_modes;
        self.queue = owners.queue;
        self.committed_revision = owners.committed_revision;
        self.last_receipt = Some(owners.receipt.into_inner());
        self.last_decision = Some(owners.decision.into_inner());
        self.last_event_facts = Some(owners.event_facts.into_inner());
        self.last_creator_recoveries = owners.creator_recoveries;
        self.last_corridor_restrictions = owners.corridor_restrictions;
        Ok(retired)
    }

    /// Latch only an authentic current failure, retaining its original Box.
    pub fn commit_step_failure(
        &mut self,
        mut failure: Box<LivingPreparedStepFailureV2>,
    ) -> Result<LivingRetiredBoundaryV2, LivingStepCommitRejectionV2> {
        if !self
            .boundary_capture_is_current(&failure.owners.as_ref().expect("failure token").capture)
        {
            return Err(LivingStepCommitRejectionV2 {
                reason: LivingStepJobErrorV2::Stale,
                _preparation: Some(LivingStepPreparationV2::Failed(failure)),
            });
        }
        let owners = failure.owners.take().expect("failure token");
        self.fault = Some(owners.fault);
        Ok(LivingRetiredBoundaryV2 {
            _capture: Some(owners.capture.into_inner()),
            _token: Some(LivingStepPreparationV2::Failed(failure)),
            _receipt: None,
            _decision: None,
            _event_facts: None,
            _creator_recoveries: Vec::new(),
            _corridor_restrictions: Vec::new(),
        })
    }

    /// Whether an unpublished candidate still belongs to this exact authority
    /// publication. Admission, stepping, creator edits and history navigation
    /// invalidate an older token; rejected operations preserve it.
    pub fn boundary_capture_is_current(&self, capture: &LivingBoundaryCaptureV2) -> bool {
        self.saved_present.is_none()
            && self.fault.is_none()
            && Arc::ptr_eq(&self.state, &capture.state)
            && Arc::ptr_eq(&self.catalog, &capture.catalog)
            && Arc::ptr_eq(&self.queue, &capture.queue)
            && Arc::ptr_eq(&self.revisions, &capture.revisions)
            && Arc::ptr_eq(&self.revision_modes, &capture.revision_modes)
            && self.genesis_seed == capture.genesis_seed
            && self.branch == capture.branch
            && self.committed_revision == capture.committed_revision
            && self.accepted_sequence_high_water == capture.accepted_sequence_high_water
            && self.branch_sequence_high_water == capture.branch_sequence_high_water
    }

    /// Ephemeral authenticated route restrictions at the last committed tick.
    /// Replay regenerates these records; canonical exports never serialize them.
    pub fn last_corridor_restrictions(&self) -> &[super::event_facts::LivingCorridorRestrictionV2] {
        &self.last_corridor_restrictions
    }

    /// Latest atomic ordinary receipt, including paused creator interventions.
    pub const fn last_receipt(&self) -> Option<&LivingTickReceiptV2> {
        self.last_receipt.as_ref()
    }

    /// Exact receipt-bound recovery proofs from the latest successful boundary.
    pub fn last_creator_recoveries(&self) -> &[LivingCreatorRecoveryV2] {
        &self.last_creator_recoveries
    }

    /// Replayable decision explanation paired with the latest running boundary.
    pub const fn last_decision(&self) -> Option<&super::decision::LivingDecisionReceiptV2> {
        self.last_decision.as_ref()
    }

    /// Authoritative event explanations paired with the latest running receipt.
    pub const fn last_event_facts(&self) -> Option<&super::event_facts::LivingEventFactsReceiptV2> {
        self.last_event_facts.as_ref()
    }

    /// The committed state.
    pub fn state(&self) -> &LivingGalaxyStateV2 {
        &self.state
    }

    /// The catalog this authority is bound to.
    pub fn catalog(&self) -> &ValidatedLivingCatalogPackV2 {
        &self.catalog
    }

    /// Retain the exact immutable state publication without copying entities.
    pub fn state_snapshot(&self) -> Arc<LivingGalaxyStateV2> {
        Arc::clone(&self.state)
    }

    /// Retain the exact immutable catalog without copying pack bytes.
    pub fn catalog_snapshot(&self) -> Arc<ValidatedLivingCatalogPackV2> {
        Arc::clone(&self.catalog)
    }

    /// Digest of the immutable, validated genesis state.
    pub fn genesis_digest(&self) -> Result<LivingStateDigestV2, LivingWireErrorV2> {
        self.origin.digest()
    }

    /// The generator provenance of this authority's genesis.
    pub const fn generator(&self) -> &LivingGenesisGeneratorV2 {
        &self.generator
    }

    /// The branch this authority advances.
    pub const fn branch(&self) -> LivingBranchIdV2 {
        self.branch
    }

    /// Parent views retained when this branch diverged.
    pub fn archived(&self) -> &[Self] {
        &self.archives
    }

    /// Replay recorded creator inputs from genesis and return the resulting digest.
    pub fn replay_digest(&self) -> Result<LivingStateDigestV2, LivingDeterministicFaultV2> {
        let mut scratch = self.replay_prefix(self.revisions.len())?;
        while scratch.state.tick.0 < self.state.tick.0 {
            scratch.step()?;
        }
        if scratch.state.tick != self.state.tick {
            return Err(LivingDeterministicFaultV2::Invariant);
        }
        Ok(scratch.state.digest()?)
    }

    /// Replay one archived parent and return its digest.
    pub fn replay_archived_digest(
        &self,
        branch: LivingBranchIdV2,
    ) -> Result<LivingStateDigestV2, LivingDeterministicFaultV2> {
        let archived = self
            .archives
            .iter()
            .find(|view| view.branch == branch)
            .ok_or(LivingDeterministicFaultV2::Invariant)?;
        archived.replay_digest()
    }

    /// Branch from an exact committed revision cursor with a new paused command.
    ///
    /// The untouched later history stays in [`Self::archived`]. This view becomes
    /// the new branch, stepped forward to the same completed tick.
    pub fn fork_paused(
        &mut self,
        at: LivingRevisionIdV2,
        operations: Vec<LivingCreatorOperationV2>,
    ) -> Result<LivingBranchIdV2, LivingCommandRejectionV2> {
        if self.fault.is_some() {
            return Err(LivingCommandRejectionV2::Faulted);
        }
        if !self.queue.is_empty() {
            return Err(LivingCommandRejectionV2::PausedWithRunningQueue);
        }
        if self.archives.len() + 1 >= 64 {
            return Err(LivingCommandRejectionV2::HistoryFull {
                collection: "branches",
                limit: 64,
            });
        }
        self.check_revision_capacity()?;
        let position = self
            .revisions
            .iter()
            .position(|revision| revision.id == at)
            .ok_or(LivingCommandRejectionV2::StaleRevision {
                expected: Some(at),
                found: self.committed_revision,
            })?;
        let fork_tick = self.revisions[position].tick;
        let parent_tick = self.state.tick;
        let parent_high_water = self.accepted_sequence_high_water;
        let ordinal = self
            .branch_sequence_high_water
            .checked_add(1)
            .ok_or(LivingCommandRejectionV2::Exhausted)?;
        let fork_branch = fork_branch_id_v2(self.branch, at, fork_tick, ordinal);
        let mut child = self
            .replay_prefix(position + 1)
            .map_err(LivingCommandRejectionV2::Seal)?;
        while child.state.tick.0 < fork_tick.0 {
            child.step().map_err(LivingCommandRejectionV2::Seal)?;
        }
        if child.state.tick != fork_tick {
            return Err(LivingCommandRejectionV2::StaleTick {
                expected: fork_tick,
                found: child.state.tick,
            });
        }
        child.archives.clear();
        child.saved_present = None;
        child.redo.clear();
        child.replay_base = Some(Box::new(child.clone()));
        child.accepted_sequence_high_water = parent_high_water;
        child.branch_sequence_high_water = ordinal;
        child.replay_branch_sequence = ordinal;
        child.branch = fork_branch;
        Arc::make_mut(&mut child.state).branch_sequence = ordinal;
        let cursor = child.published_cursor();
        child.apply_paused(LivingCommandEnvelopeV2 {
            expected_committed_revision: cursor.committed_revision,
            expected_tick: cursor.tick,
            expected_pending_sequence: cursor.pending_sequence,
            mode: LivingCommandModeV2::Paused,
            command: LivingCommandV2::CreatorBatch { operations },
        })?;
        while child.state.tick.0 < parent_tick.0 {
            child.step().map_err(LivingCommandRejectionV2::Seal)?;
        }
        let mut archived = self.saved_present.as_deref().unwrap_or(self).clone();
        archived.archives.clear();
        child.archives = self.archives.clone();
        child.archives.push(archived);
        *self = child;
        Ok(fork_branch)
    }

    /// Preserve the exact selected view as a new branch, including autonomous
    /// history after its latest revision and before the fork.
    pub fn fork_view_paused(
        &mut self,
        operations: Vec<LivingCreatorOperationV2>,
    ) -> Result<LivingBranchIdV2, LivingCommandRejectionV2> {
        if self.fault.is_some() {
            return Err(LivingCommandRejectionV2::Faulted);
        }
        if !self.queue.is_empty() {
            return Err(LivingCommandRejectionV2::PausedWithRunningQueue);
        }
        if self.archives.len() + 1 >= 64 {
            return Err(LivingCommandRejectionV2::HistoryFull {
                collection: "branches",
                limit: 64,
            });
        }
        if !operations.is_empty() {
            self.check_revision_capacity()?;
        }
        let ordinal = self
            .branch_sequence_high_water
            .checked_add(1)
            .ok_or(LivingCommandRejectionV2::Exhausted)?;
        let genesis = self.origin.digest().map_err(|error| {
            LivingCommandRejectionV2::Seal(LivingDeterministicFaultV2::Wire(error))
        })?;
        let branch = match self.committed_revision {
            Some(revision) => fork_branch_id_v2(self.branch, revision, self.state.tick, ordinal),
            None => genesis_fork_branch_id_v2(self.branch, genesis, self.state.tick, ordinal),
        };
        let mut child = self.clone();
        child.archives.clear();
        child.saved_present = None;
        child.redo.clear();
        child.last_receipt = None;
        child.last_decision = None;
        child.last_event_facts = None;
        child.last_corridor_restrictions.clear();
        child.last_creator_recoveries.clear();
        child.replay_base = Some(Box::new(child.clone()));
        child.branch = branch;
        child.branch_sequence_high_water = ordinal;
        child.replay_branch_sequence = ordinal;
        Arc::make_mut(&mut child.state).branch_sequence = ordinal;
        if !operations.is_empty() {
            let cursor = child.published_cursor();
            child.apply_paused(LivingCommandEnvelopeV2 {
                expected_committed_revision: cursor.committed_revision,
                expected_tick: cursor.tick,
                expected_pending_sequence: cursor.pending_sequence,
                mode: LivingCommandModeV2::Paused,
                command: LivingCommandV2::CreatorBatch { operations },
            })?;
        }
        let mut archived = self.saved_present.as_deref().unwrap_or(self).clone();
        archived.archives.clear();
        child.archives = self.archives.clone();
        child.archives.push(archived);
        *self = child;
        Ok(branch)
    }

    /// Exact cursor and digest of the selected committed history view.
    pub fn bookmark(&self) -> Result<LivingBookmarkV2, LivingHistoryErrorV2> {
        Ok(LivingBookmarkV2 {
            view: LivingHistoryViewV2 {
                branch: self.branch,
                revision: self.committed_revision,
                tick: self.state.tick,
            },
            digest: self.state.digest()?,
        })
    }

    fn retained_branch(&self, branch: LivingBranchIdV2) -> Option<&Self> {
        if self.branch == branch {
            return Some(self.saved_present.as_deref().unwrap_or(self));
        }
        self.archives.iter().find(|view| view.branch == branch)
    }

    /// Read the retained branch head rather than its currently selected
    /// historical cursor. This getter does not replay, clone, or change state.
    pub fn retained_history_branch(&self, branch: LivingBranchIdV2) -> Option<&Self> {
        self.retained_branch(branch)
    }

    fn materialize_view(&self, view: LivingHistoryViewV2) -> Result<Self, LivingHistoryErrorV2> {
        let source = self
            .retained_branch(view.branch)
            .ok_or(LivingHistoryErrorV2::UnknownView)?;
        let document_tick = std::iter::once(self)
            .chain(self.archives.iter())
            .map(|branch| {
                branch
                    .saved_present
                    .as_deref()
                    .unwrap_or(branch)
                    .state
                    .tick
                    .0
            })
            .max()
            .unwrap_or(0);
        if view.tick.0 > document_tick {
            return Err(LivingHistoryErrorV2::UnknownView);
        }
        if source
            .replay_base
            .as_ref()
            .is_some_and(|base| view.tick.0 < base.state.tick.0)
        {
            return Err(LivingHistoryErrorV2::UnknownView);
        }
        let count = match view.revision {
            None => 0,
            Some(id) => source
                .revisions
                .iter()
                .position(|revision| revision.id == id)
                .map(|position| position + 1)
                .ok_or(LivingHistoryErrorV2::UnknownView)?,
        };
        if source
            .replay_base
            .as_ref()
            .is_some_and(|base| count < base.revisions.len())
        {
            return Err(LivingHistoryErrorV2::UnknownView);
        }
        let mut candidate = source.replay_prefix(count)?;
        if candidate.state.tick.0 > view.tick.0 {
            return Err(LivingHistoryErrorV2::UnknownView);
        }
        while candidate.state.tick.0 < view.tick.0 {
            candidate.step()?;
        }
        candidate.branch = source.branch;
        candidate.accepted_sequence_high_water = self.accepted_sequence_high_water;
        candidate.branch_sequence_high_water = self.branch_sequence_high_water;
        candidate.replay_base = source.replay_base.clone();
        candidate.replay_branch_sequence = source.replay_branch_sequence;
        candidate.saved_present = None;
        candidate.redo.clear();
        candidate.archives.clear();
        Ok(candidate)
    }

    /// Materialize retained creator inputs at an exact completed tick. Failures
    /// leave the active state, queue and allocator marks unchanged.
    pub fn restore_view(&mut self, view: LivingHistoryViewV2) -> Result<(), LivingHistoryErrorV2> {
        if !self.queue.is_empty() {
            return Err(LivingHistoryErrorV2::PendingQueue);
        }
        let mut present = self
            .retained_branch(view.branch)
            .ok_or(LivingHistoryErrorV2::UnknownView)?
            .clone();
        if present.state.tick.0 < view.tick.0 {
            present = self.materialize_view(LivingHistoryViewV2 {
                branch: view.branch,
                revision: present.committed_revision,
                tick: view.tick,
            })?;
        }
        let mut candidate = self.materialize_view(view)?;
        let mut retained = self.archives.clone();
        if self.branch != view.branch {
            retained.retain(|branch| branch.branch != view.branch);
            let mut old = self.saved_present.as_deref().unwrap_or(self).clone();
            old.archives.clear();
            old.saved_present = None;
            retained.push(old);
        }
        candidate.archives = retained;
        if present.committed_revision != view.revision || present.state.tick != view.tick {
            let mut present = present;
            present.archives.clear();
            present.saved_present = None;
            candidate.saved_present = Some(Box::new(present));
        }
        *self = candidate;
        Ok(())
    }

    /// Restore a bookmark only if its replay digest still agrees exactly.
    pub fn restore_bookmark(
        &mut self,
        bookmark: LivingBookmarkV2,
    ) -> Result<(), LivingHistoryErrorV2> {
        if !self.queue.is_empty() {
            return Err(LivingHistoryErrorV2::PendingQueue);
        }
        if self.materialize_view(bookmark.view)?.state.digest()? != bookmark.digest {
            return Err(LivingHistoryErrorV2::DigestMismatch);
        }
        self.restore_view(bookmark.view)
    }

    /// Whether the selected view differs from its retained present.
    pub const fn is_viewing_history(&self) -> bool {
        self.saved_present.is_some()
    }

    /// Exact retained present target of a historical selection, without hashing
    /// or materializing any state. UI restoration uses its cancellable builder.
    pub fn present_view(&self) -> Option<LivingHistoryViewV2> {
        self.saved_present
            .as_ref()
            .map(|present| LivingHistoryViewV2 {
                branch: present.branch,
                revision: present.committed_revision,
                tick: present.state.tick,
            })
    }

    /// Exact same-tick counterfactual target, without doing replay work.
    pub fn undo_view(&self) -> Result<LivingHistoryViewV2, LivingHistoryErrorV2> {
        let revision = self.revisions.last().ok_or(LivingHistoryErrorV2::Empty)?;
        let branch = self
            .replay_base
            .as_ref()
            .filter(|base| self.revisions.len() <= base.revisions.len())
            .map_or(self.branch, |base| base.branch);
        Ok(LivingHistoryViewV2 {
            branch,
            revision: revision.parent,
            tick: self.state.tick,
        })
    }
    /// Exact next redo target, without doing replay work.
    pub fn redo_view(&self) -> Result<LivingHistoryViewV2, LivingHistoryErrorV2> {
        self.redo.last().copied().ok_or(LivingHistoryErrorV2::Empty)
    }
    pub(crate) fn undo_redo_targets(&self) -> Vec<LivingHistoryViewV2> {
        self.redo.clone()
    }
    pub(crate) fn install_redo_targets(&mut self, redo: Vec<LivingHistoryViewV2>) {
        self.redo = redo;
    }

    /// Synchronous off-UI wrapper. UI callers use the cancellable history
    /// builders, which install only fully reconstructed views.
    pub fn undo(&mut self) -> Result<LivingBookmarkV2, LivingHistoryErrorV2> {
        let view = self.undo_view()?;
        let mut redo = self.redo.clone();
        redo.push(self.bookmark()?.view);
        self.restore_view(view)?;
        self.redo = redo;
        self.bookmark()
    }
    /// Synchronous off-UI wrapper for reapplying a same-tick creator cursor.
    pub fn redo(&mut self) -> Result<LivingBookmarkV2, LivingHistoryErrorV2> {
        let mut redo = self.redo.clone();
        let view = redo.pop().ok_or(LivingHistoryErrorV2::Empty)?;
        self.restore_view(view)?;
        self.redo = redo;
        self.bookmark()
    }
    /// Synchronous off-UI restoration of the selected branch's retained future.
    pub fn return_to_present(&mut self) -> Result<LivingBookmarkV2, LivingHistoryErrorV2> {
        if let Some(present) = &self.saved_present {
            let view = present.bookmark()?.view;
            self.restore_view(view)?;
        }
        self.bookmark()
    }

    /// Cheap exact creator cursors for a common-tick comparison. Materialize
    /// them through the cancellable comparison builder on the UI path.
    pub fn comparison_views_at(
        &self,
        left: LivingBranchIdV2,
        right: LivingBranchIdV2,
        tick: LivingTickV2,
    ) -> Result<(LivingHistoryViewV2, LivingHistoryViewV2), LivingHistoryErrorV2> {
        let view = |branch| -> Result<LivingHistoryViewV2, LivingHistoryErrorV2> {
            let source = self
                .retained_branch(branch)
                .ok_or(LivingHistoryErrorV2::UnknownView)?;
            let revision = source
                .revisions
                .iter()
                .rev()
                .find(|revision| revision.tick.0 <= tick.0)
                .map(|revision| revision.id);
            Ok(LivingHistoryViewV2 {
                branch,
                revision,
                tick,
            })
        };
        Ok((view(left)?, view(right)?))
    }

    /// Compare retained branch heads at one common completed tick.
    pub fn compare_at(
        &self,
        left: LivingBranchIdV2,
        right: LivingBranchIdV2,
        tick: LivingTickV2,
    ) -> Result<LivingComparisonV2, LivingHistoryErrorV2> {
        let materialize = |branch| {
            let source = self
                .retained_branch(branch)
                .ok_or(LivingHistoryErrorV2::UnknownView)?;
            let revision = source
                .revisions
                .iter()
                .rev()
                .find(|revision| revision.tick.0 <= tick.0)
                .map(|revision| revision.id);
            self.materialize_view(LivingHistoryViewV2 {
                branch,
                revision,
                tick,
            })
        };
        let left = materialize(left)?;
        let right = materialize(right)?;
        Ok(LivingComparisonV2 {
            tick,
            left: left.bookmark()?,
            right: right.bookmark()?,
            left_state: left.state.as_ref().clone(),
            right_state: right.state.as_ref().clone(),
        })
    }

    /// Independent document-global branch allocator mark.
    pub const fn branch_sequence_high_water(&self) -> u64 {
        self.branch_sequence_high_water
    }

    /// Genesis seed, preserved exactly rather than regenerated.
    pub const fn genesis_seed(&self) -> [u8; 32] {
        self.genesis_seed
    }

    pub(crate) fn genesis_manifest(&self) -> super::genesis::LivingGenesisManifestV2 {
        super::genesis::LivingGenesisManifestV2 {
            kind: super::genesis::LIVING_GENESIS_KIND_V2.to_owned(),
            format_version: super::genesis::LIVING_GENESIS_FORMAT_VERSION_V2,
            rules_version: super::LIVING_RULES_VERSION,
            state: self.origin.as_ref().clone(),
        }
    }

    pub(crate) fn canonical_history(
        &self,
    ) -> Result<
        (Vec<LivingRecordedRevisionV2>, Vec<LivingHistoryBranchV2>),
        LivingDeterministicFaultV2,
    > {
        let mut revisions = std::collections::BTreeMap::new();
        let mut branches = std::collections::BTreeMap::new();
        for view in std::iter::once(self.saved_present.as_deref().unwrap_or(self))
            .chain(self.archives.iter())
        {
            if view.revisions.len() != view.revision_modes.len() {
                return Err(LivingDeterministicFaultV2::Invariant);
            }
            for (revision, mode) in view.revisions.iter().zip(view.revision_modes.iter()) {
                let record = LivingRecordedRevisionV2 {
                    revision: revision.clone(),
                    mode: *mode,
                };
                if revisions
                    .get(&revision.id)
                    .is_some_and(|old| old != &record)
                {
                    return Err(LivingDeterministicFaultV2::Invariant);
                }
                revisions.insert(revision.id, record);
            }
            let (parent, fork_revision, fork_tick) = view
                .replay_base
                .as_ref()
                .map_or((None, None, LivingTickV2(0)), |base| {
                    (Some(base.branch), base.committed_revision, base.state.tick)
                });
            branches.insert(
                view.branch,
                LivingHistoryBranchV2 {
                    id: view.branch,
                    parent,
                    fork_revision,
                    fork_tick,
                    ordinal: view.replay_branch_sequence,
                    head: view.committed_revision,
                    tip_tick: view.state.tick,
                },
            );
        }
        Ok((
            revisions.into_values().collect(),
            branches.into_values().collect(),
        ))
    }

    pub(crate) fn replay_paused(
        &mut self,
        record: &LivingRecordedRevisionV2,
    ) -> Result<(), LivingDeterministicFaultV2> {
        if record.mode != LivingCommandModeV2::Paused
            || record.revision.tick != self.state.tick
            || record.revision.parent != self.committed_revision
        {
            return Err(LivingDeterministicFaultV2::Invariant);
        }
        let mut candidate = self.state.as_ref().clone();
        let (effects, recoveries) =
            apply_revision_with_recoveries(&mut candidate, &record.revision, &self.catalog)
                .map_err(|reason| LivingDeterministicFaultV2::Replay {
                    accepted_sequence: record.revision.ordinal,
                    reason: Box::new(reason),
                })?;
        candidate.validate(&self.catalog)?;
        let mut events = LivingPendingEventsV2::new();
        let provenance = LivingEventProvenanceV2::Creator {
            revision: record.revision.id,
        };
        events.record(provenance, LivingEventKindV2::CreatorIntervention)?;
        for kind in effects {
            events.record(provenance, kind)?;
        }
        let receipt = seal_living_tick_receipt_v2(
            self.state.tick,
            vec![record.revision.id],
            &events,
            candidate.digest()?,
        )?;
        self.last_creator_recoveries = bind_creator_recoveries(&candidate, &receipt, &recoveries);
        self.state = Arc::new(candidate).into();
        self.last_receipt = Some(receipt);
        self.last_decision = None;
        self.last_event_facts = None;
        self.last_corridor_restrictions.clear();
        self.committed_revision = Some(record.revision.id);
        self.accepted_sequence_high_water = self
            .accepted_sequence_high_water
            .max(record.revision.ordinal);
        Arc::make_mut(&mut self.revisions).push(record.revision.clone());
        Arc::make_mut(&mut self.revision_modes).push(record.mode);
        Ok(())
    }

    pub(crate) fn begin_replay_boundary(
        &self,
    ) -> Result<LivingReplayBoundaryV2, LivingDeterministicFaultV2> {
        let tick = self
            .state
            .tick
            .0
            .checked_add(1)
            .map(LivingTickV2)
            .ok_or(LivingDeterministicFaultV2::Arithmetic)?;
        Ok(LivingReplayBoundaryV2 {
            candidate: self.state.as_ref().clone(),
            events: LivingPendingEventsV2::new(),
            records: Vec::new(),
            tick,
            parent: self.committed_revision,
        })
    }
    pub(crate) fn stage_replay_record(
        &self,
        boundary: &mut LivingReplayBoundaryV2,
        record: &LivingRecordedRevisionV2,
    ) -> Result<(), LivingDeterministicFaultV2> {
        let parent = boundary
            .records
            .last()
            .map(|record| record.revision.id)
            .or(boundary.parent);
        if record.mode != LivingCommandModeV2::Running
            || record.revision.tick != boundary.tick
            || record.revision.parent != parent
            || boundary.records.len() >= LIVING_MAX_PENDING_QUEUE_DEPTH_V2
        {
            return Err(LivingDeterministicFaultV2::Invariant);
        }
        let (effects, recoveries) = apply_revision_with_recoveries(
            &mut boundary.candidate,
            &record.revision,
            &self.catalog,
        )
        .map_err(|reason| LivingDeterministicFaultV2::Replay {
            accepted_sequence: record.revision.ordinal,
            reason: Box::new(reason),
        })?;
        boundary.events.record(
            LivingEventProvenanceV2::Creator {
                revision: record.revision.id,
            },
            LivingEventKindV2::CreatorIntervention,
        )?;
        for kind in effects {
            boundary.events.record(
                LivingEventProvenanceV2::Creator {
                    revision: record.revision.id,
                },
                kind,
            )?;
        }
        boundary.events.creator_recoveries.extend(recoveries);
        boundary.records.push(record.clone());
        Ok(())
    }
    pub(crate) fn finish_replay_boundary(
        &mut self,
        boundary: LivingReplayBoundaryV2,
    ) -> Result<(), LivingDeterministicFaultV2> {
        if boundary.parent != self.committed_revision
            || boundary.tick.0 != self.state.tick.0 + 1
            || boundary.records.is_empty()
        {
            return Err(LivingDeterministicFaultV2::Invariant);
        }
        let ids = boundary
            .records
            .iter()
            .map(|record| record.revision.id)
            .collect();
        let (candidate, receipt, decision, event_facts, creator_recoveries, corridor_restrictions) =
            self.run_boundary_candidate(boundary.candidate, boundary.events, Some(ids))?;
        let last = boundary
            .records
            .last()
            .ok_or(LivingDeterministicFaultV2::Invariant)?;
        self.committed_revision = Some(last.revision.id);
        self.accepted_sequence_high_water =
            self.accepted_sequence_high_water.max(last.revision.ordinal);
        Arc::make_mut(&mut self.revisions).extend(
            boundary
                .records
                .iter()
                .map(|record| record.revision.clone()),
        );
        Arc::make_mut(&mut self.revision_modes)
            .extend(boundary.records.iter().map(|record| record.mode));
        self.state = Arc::new(candidate).into();
        self.last_receipt = Some(receipt);
        self.last_decision = Some(decision);
        self.last_event_facts = Some(event_facts);
        self.last_creator_recoveries = creator_recoveries;
        self.last_corridor_restrictions = corridor_restrictions;
        Ok(())
    }

    pub(crate) fn replay_fork(&mut self, branch: LivingBranchIdV2, ordinal: u64) {
        let mut base = self.clone();
        base.archives.clear();
        base.saved_present = None;
        self.replay_base = Some(Box::new(base));
        self.branch = branch;
        Arc::make_mut(&mut self.state).branch_sequence = ordinal;
        self.replay_branch_sequence = ordinal;
        self.last_creator_recoveries.clear();
        self.last_corridor_restrictions.clear();
        self.branch_sequence_high_water = self.branch_sequence_high_water.max(ordinal);
    }

    pub(crate) fn install_history(
        &mut self,
        mut branches: Vec<Self>,
        accepted: u64,
        branch: u64,
        present: Self,
    ) {
        branches.retain(|view| view.branch != self.branch);
        self.archives = branches;
        self.accepted_sequence_high_water = accepted;
        self.branch_sequence_high_water = branch;
        if present.committed_revision != self.committed_revision
            || present.state.tick != self.state.tick
        {
            self.saved_present = Some(Box::new(present));
        }
    }

    /// Committed revisions in application order.
    pub fn revisions(&self) -> &[LivingRevisionV2] {
        &self.revisions
    }

    /// The fault the authority is paused on, if any.
    pub const fn fault(&self) -> Option<&LivingDeterministicFaultV2> {
        self.fault.as_ref()
    }

    /// The durable allocator mark: the last accepted sequence ever issued,
    /// including sequences a retained faulted queue has spent.
    pub const fn accepted_sequence_high_water(&self) -> u64 {
        self.accepted_sequence_high_water
    }

    /// The cursor a submitter must name.
    pub fn published_cursor(&self) -> LivingCommandCursorV2 {
        LivingCommandCursorV2 {
            committed_revision: self.committed_revision,
            tick: self.state.tick,
            pending_sequence: self
                .queue
                .last()
                .map_or(0, |queued| queued.accepted.accepted_sequence),
        }
    }

    /// Queue a running envelope for the next boundary.
    pub fn submit(
        &mut self,
        envelope: LivingCommandEnvelopeV2,
    ) -> Result<LivingAcceptedCommandV2, LivingCommandRejectionV2> {
        self.check_admission(&envelope, LivingCommandModeV2::Running)?;
        if self.queue.len() >= LIVING_MAX_PENDING_QUEUE_DEPTH_V2 {
            return Err(LivingCommandRejectionV2::QueueFull {
                limit: LIVING_MAX_PENDING_QUEUE_DEPTH_V2,
            });
        }
        let application_tick = self
            .state
            .tick
            .0
            .checked_add(1)
            .map(LivingTickV2)
            .ok_or(LivingCommandRejectionV2::Exhausted)?;
        let parent = self
            .queue
            .last()
            .map(|queued| queued.revision.id)
            .or(self.committed_revision);
        let (accepted, revision) = self.seal(envelope, application_tick, parent)?;

        let mut projection = self.state.as_ref().clone();
        for queued in self.queue.iter() {
            apply_revision(&mut projection, &queued.revision, &self.catalog)?;
        }
        apply_revision(&mut projection, &revision, &self.catalog)?;
        projection.validate(&self.catalog)?;

        self.accepted_sequence_high_water = accepted.accepted_sequence;
        Arc::make_mut(&mut self.queue).push(QueuedCommandV2 {
            accepted: Arc::new(accepted.clone()).into(),
            revision: Arc::new(revision).into(),
        });
        Ok(accepted)
    }

    /// Apply a paused envelope synchronously at the current boundary.
    pub fn apply_paused(
        &mut self,
        envelope: LivingCommandEnvelopeV2,
    ) -> Result<LivingTickReceiptV2, LivingCommandRejectionV2> {
        self.check_admission(&envelope, LivingCommandModeV2::Paused)?;
        if !self.queue.is_empty() {
            return Err(LivingCommandRejectionV2::PausedWithRunningQueue);
        }
        let tick = self.state.tick;
        let (_, revision) = self.seal(envelope, tick, self.committed_revision)?;

        let mut candidate = self.state.as_ref().clone();
        let (effects, recoveries) =
            apply_revision_with_recoveries(&mut candidate, &revision, &self.catalog)?;
        candidate.validate(&self.catalog)?;

        let mut events = LivingPendingEventsV2::new();
        let mut seal = || -> Result<LivingTickReceiptV2, LivingDeterministicFaultV2> {
            events.record(
                LivingEventProvenanceV2::Creator {
                    revision: revision.id,
                },
                LivingEventKindV2::CreatorIntervention,
            )?;
            for kind in &effects {
                events.record(
                    LivingEventProvenanceV2::Creator {
                        revision: revision.id,
                    },
                    *kind,
                )?;
            }
            let digest = candidate.digest()?;
            Ok(seal_living_tick_receipt_v2(
                tick,
                vec![revision.id],
                &events,
                digest,
            )?)
        };
        let receipt = seal().map_err(LivingCommandRejectionV2::Seal)?;

        // Everything that can fail has run. Commit every field together.
        self.last_creator_recoveries = bind_creator_recoveries(&candidate, &receipt, &recoveries);
        self.accepted_sequence_high_water = revision.ordinal;
        self.committed_revision = Some(revision.id);
        Arc::make_mut(&mut self.revisions).push(revision);
        Arc::make_mut(&mut self.revision_modes).push(LivingCommandModeV2::Paused);
        self.state = Arc::new(candidate).into();
        self.last_receipt = Some(receipt.clone());
        self.last_decision = None;
        self.last_event_facts = None;
        self.last_corridor_restrictions.clear();
        Ok(receipt)
    }

    /// Advance one boundary, committing everything or nothing.
    pub fn step(&mut self) -> Result<LivingTickReceiptV2, LivingDeterministicFaultV2> {
        if self.saved_present.is_some() {
            return Err(LivingDeterministicFaultV2::ViewingHistory);
        }
        if let Some(fault) = &self.fault {
            return Err(fault.clone());
        }
        match self.run_boundary() {
            Ok((
                candidate,
                receipt,
                decision,
                event_facts,
                creator_recoveries,
                corridor_restrictions,
            )) => {
                // The only mutation of a successful boundary, after every
                // fallible step has run.
                let applied = std::mem::take(&mut self.queue);
                if let Some(last) = applied.last() {
                    self.committed_revision = Some(last.revision.id);
                }
                Arc::make_mut(&mut self.revision_modes).extend(std::iter::repeat_n(
                    LivingCommandModeV2::Running,
                    applied.len(),
                ));
                Arc::make_mut(&mut self.revisions).extend(
                    applied
                        .iter()
                        .map(|queued| queued.revision.as_ref().clone()),
                );
                self.state = Arc::new(candidate).into();
                self.last_receipt = Some(receipt.clone());
                self.last_decision = Some(decision);
                self.last_event_facts = Some(event_facts);
                self.last_creator_recoveries = creator_recoveries;
                self.last_corridor_restrictions = corridor_restrictions;
                Ok(receipt)
            }
            Err(fault) => {
                self.fault = Some(fault.clone());
                Err(fault)
            }
        }
    }

    /// Discard a queue retained by a fault and resume.
    ///
    /// The sequences the discarded envelopes were issued stay spent: the
    /// allocator mark does not move back.
    pub fn discard_faulted_queue(&mut self) {
        if self.fault.take().is_some() {
            Arc::make_mut(&mut self.queue).clear();
        }
    }

    fn check_revision_capacity(&self) -> Result<(), LivingCommandRejectionV2> {
        let mut revisions = std::collections::BTreeSet::new();
        for view in std::iter::once(self).chain(self.archives.iter()) {
            revisions.extend(view.revisions.iter().map(|revision| revision.id));
            revisions.extend(view.queue.iter().map(|queued| queued.revision.id));
        }
        if revisions.len() >= 10_000 {
            return Err(LivingCommandRejectionV2::HistoryFull {
                collection: "revisions",
                limit: 10_000,
            });
        }
        Ok(())
    }

    fn check_admission(
        &self,
        envelope: &LivingCommandEnvelopeV2,
        mode: LivingCommandModeV2,
    ) -> Result<(), LivingCommandRejectionV2> {
        if self.saved_present.is_some() {
            return Err(LivingCommandRejectionV2::ViewingHistory);
        }
        if self.fault.is_some() {
            return Err(LivingCommandRejectionV2::Faulted);
        }
        if envelope.mode != mode {
            return Err(LivingCommandRejectionV2::ModeMismatch {
                expected: mode,
                found: envelope.mode,
            });
        }
        let cursor = self.published_cursor();
        if envelope.expected_committed_revision != cursor.committed_revision {
            return Err(LivingCommandRejectionV2::StaleRevision {
                expected: envelope.expected_committed_revision,
                found: cursor.committed_revision,
            });
        }
        if envelope.expected_tick != cursor.tick {
            return Err(LivingCommandRejectionV2::StaleTick {
                expected: envelope.expected_tick,
                found: cursor.tick,
            });
        }
        if envelope.expected_pending_sequence != cursor.pending_sequence {
            return Err(LivingCommandRejectionV2::StalePendingSequence {
                expected: envelope.expected_pending_sequence,
                found: cursor.pending_sequence,
            });
        }
        self.check_revision_capacity()
    }

    /// Allocate the next sequence tentatively and seal the revision. Nothing
    /// is recorded until the caller commits.
    fn seal(
        &self,
        envelope: LivingCommandEnvelopeV2,
        application_tick: LivingTickV2,
        parent: Option<LivingRevisionIdV2>,
    ) -> Result<(LivingAcceptedCommandV2, LivingRevisionV2), LivingCommandRejectionV2> {
        envelope.command.validate()?;
        for (operation, op) in envelope.command.operations().iter().enumerate() {
            if !is_applied_by_this_slice(op) {
                return Err(LivingCommandRejectionV2::UnsupportedOperation { operation });
            }
        }
        let accepted_sequence = self
            .accepted_sequence_high_water
            .checked_add(1)
            .ok_or(LivingCommandRejectionV2::Exhausted)?;
        let accepted = LivingAcceptedCommandV2 {
            accepted_sequence,
            application_tick,
            envelope,
        };
        let revision = LivingRevisionV2::seal(
            self.catalog.catalog_hash(),
            self.genesis_seed,
            parent,
            &accepted,
        )?;
        Ok((accepted, revision))
    }

    fn run_boundary(&self) -> Result<LivingBoundaryOutputV2, LivingDeterministicFaultV2> {
        self.run_boundary_candidate(
            self.state.as_ref().clone(),
            LivingPendingEventsV2::new(),
            None,
        )
    }

    fn run_boundary_candidate(
        &self,
        candidate: LivingGalaxyStateV2,
        events: LivingPendingEventsV2,
        recorded: Option<Vec<LivingRevisionIdV2>>,
    ) -> Result<LivingBoundaryOutputV2, LivingDeterministicFaultV2> {
        self.run_boundary_candidate_with_source_digest(candidate, events, recorded, None, None)
    }

    fn run_boundary_candidate_with_source_digest(
        &self,
        mut candidate: LivingGalaxyStateV2,
        mut events: LivingPendingEventsV2,
        recorded: Option<Vec<LivingRevisionIdV2>>,
        source_digest: Option<super::ids::LivingStateDigestV2>,
        completed_phases: Option<LivingCompletedStepPhasesV2>,
    ) -> Result<LivingBoundaryOutputV2, LivingDeterministicFaultV2> {
        let tick = self
            .state
            .tick
            .0
            .checked_add(1)
            .map(LivingTickV2)
            .ok_or(LivingDeterministicFaultV2::Arithmetic)?;
        let scheduled_applied = completed_phases.is_some();
        let observations_applied = matches!(
            completed_phases,
            Some(
                LivingCompletedStepPhasesV2::ObservationsComplete
                    | LivingCompletedStepPhasesV2::DiplomacyComplete
                    | LivingCompletedStepPhasesV2::DispatchComplete
            )
        );
        let diplomacy_applied = matches!(
            completed_phases,
            Some(
                LivingCompletedStepPhasesV2::DiplomacyComplete
                    | LivingCompletedStepPhasesV2::DispatchComplete
            )
        );
        let dispatch_applied = matches!(
            completed_phases,
            Some(LivingCompletedStepPhasesV2::DispatchComplete)
        );
        let arrivals = match completed_phases {
            Some(LivingCompletedStepPhasesV2::Scheduled { arrivals }) => arrivals,
            _ => Vec::new(),
        };
        let mut context = LivingStepContextV2 {
            catalog: &self.catalog,
            genesis_seed: self.genesis_seed,
            branch: self.branch,
            tick,
            state: &mut candidate,
            events: &mut events,
            arrivals,
        };

        for (phase, _) in LIVING_PHASE_REGISTRY_V2 {
            match phase {
                LivingPhaseV2::ApplyCreatorInterventions => {
                    if recorded.is_none() {
                        apply_queued_interventions(&mut context, &self.queue)?;
                    }
                }
                LivingPhaseV2::ExpireLifecycles => {
                    if !scheduled_applied {
                        advance_lifecycles(&mut context)?;
                    }
                }
                LivingPhaseV2::ResolveArrivals => {
                    if !scheduled_applied {
                        resolve_arrivals(&mut context)?;
                    }
                }
                LivingPhaseV2::ResolveConflict => {
                    if !observations_applied {
                        resolve_combat_occupation_and_claims(&mut context)?;
                    }
                }
                LivingPhaseV2::RunProduction => {
                    if !observations_applied {
                        complete_jobs_and_run_recipes(&mut context)?;
                    }
                }
                LivingPhaseV2::RefreshObservations => {
                    if !observations_applied {
                        refresh_observations(&mut context)?;
                    }
                }
                LivingPhaseV2::UpdateDiplomacy => {
                    if !diplomacy_applied && tick.0.is_multiple_of(DIPLOMACY_PERIOD_TICKS) {
                        update_diplomacy(&mut context)?;
                    }
                }
                LivingPhaseV2::GenerateIntents => {
                    if !dispatch_applied && tick.0.is_multiple_of(INTENT_PERIOD_TICKS) {
                        generate_and_reserve_intents(&mut context)?;
                    }
                }
                LivingPhaseV2::DispatchOrders => {
                    if !dispatch_applied {
                        dispatch_orders_and_freight(&mut context)?;
                    }
                }
                LivingPhaseV2::CommitBoundary => context.state.tick = tick,
            }
        }

        candidate.validate(&self.catalog)?;
        let digest = candidate.digest()?;
        let mut applied: Vec<LivingRevisionIdV2> = recorded
            .unwrap_or_else(|| self.queue.iter().map(|queued| queued.revision.id).collect());
        applied.sort_unstable();
        let receipt = seal_living_tick_receipt_v2(tick, applied, &events, digest)?;
        let event_facts =
            super::event_facts::seal_living_event_facts_receipt_v2(&receipt, &events)?;
        let creator_recoveries =
            bind_creator_recoveries(&candidate, &receipt, &events.creator_recoveries);
        let mut corridor_restrictions = std::mem::take(&mut events.corridor_restrictions);
        for witness in &mut corridor_restrictions {
            if witness.tick != receipt.tick {
                return Err(LivingDeterministicFaultV2::Invariant);
            }
            witness.receipt_digest = receipt.digest;
        }
        let decision = super::decision::seal_living_decision_receipt_v2(
            self.catalog.catalog_hash(),
            self.branch,
            match source_digest {
                Some(digest) => digest,
                None => self.state.digest()?,
            },
            &receipt,
            events.decisions,
        )?;
        Ok((
            candidate,
            receipt,
            decision,
            event_facts,
            creator_recoveries,
            corridor_restrictions,
        ))
    }

    /// Reconstruct a prefix using the branch that originally generated each
    /// autonomous interval, then transition at the retained fork boundary.
    fn replay_prefix(&self, count: usize) -> Result<Self, LivingDeterministicFaultV2> {
        if count > self.revisions.len() {
            return Err(LivingDeterministicFaultV2::Invariant);
        }
        let (mut scratch, start) = if let Some(base) = &self.replay_base {
            let start = base.revisions.len();
            if count < start {
                return base.replay_prefix(count);
            }
            let mut scratch = base.replay_prefix(start)?;
            while scratch.state.tick.0 < base.state.tick.0 {
                scratch.step()?;
            }
            scratch.branch = self.branch;
            Arc::make_mut(&mut scratch.state).branch_sequence = self.replay_branch_sequence;
            scratch.replay_branch_sequence = self.replay_branch_sequence;
            (scratch, start)
        } else {
            (self.origin_view(), 0)
        };
        scratch.play_recorded(
            &self.revisions[start..count],
            &self.revision_modes[start..count],
        )?;
        Ok(scratch)
    }

    fn origin_view(&self) -> Self {
        Self {
            catalog: self.catalog.clone(),
            genesis_seed: self.genesis_seed,
            generator: self.generator.clone(),
            branch: self.root_branch,
            state: self.origin.clone(),
            committed_revision: None,
            revisions: Arc::new(Vec::new()).into(),
            revision_modes: Arc::new(Vec::new()).into(),
            queue: Arc::new(Vec::new()).into(),
            accepted_sequence_high_water: 0,
            fault: None,
            origin: self.origin.clone(),
            root_branch: self.root_branch,
            branch_sequence_high_water: 0,
            replay_branch_sequence: 0,
            archives: Vec::new(),
            replay_base: None,
            saved_present: None,
            redo: Vec::new(),
            last_receipt: None,
            last_decision: None,
            last_event_facts: None,
            last_corridor_restrictions: Vec::new(),
            last_creator_recoveries: Vec::new(),
        }
    }

    /// Replay using the accepted application mode: running edits precede phases,
    /// while paused edits follow the completed boundary at the same tick.
    fn play_recorded(
        &mut self,
        revisions: &[LivingRevisionV2],
        modes: &[LivingCommandModeV2],
    ) -> Result<(), LivingDeterministicFaultV2> {
        if revisions.len() != modes.len() {
            return Err(LivingDeterministicFaultV2::Invariant);
        }
        let mut index = 0;
        while index < revisions.len() {
            let revision = &revisions[index];
            if modes[index] == LivingCommandModeV2::Paused {
                while self.state.tick.0 < revision.tick.0 {
                    self.step()?;
                }
                if revision.tick != self.state.tick {
                    return Err(LivingDeterministicFaultV2::Invariant);
                }
                self.replay_paused(&LivingRecordedRevisionV2 {
                    revision: revision.clone(),
                    mode: LivingCommandModeV2::Paused,
                })?;
                index += 1;
            } else if revision.tick.0 > self.state.tick.0 {
                while self.state.tick.0 + 1 < revision.tick.0 {
                    self.step()?;
                }
                if self.state.tick.0 + 1 != revision.tick.0 {
                    return Err(LivingDeterministicFaultV2::Invariant);
                }
                let tick = revision.tick;
                Arc::make_mut(&mut self.queue).clear();
                while index < revisions.len()
                    && revisions[index].tick == tick
                    && modes[index] == LivingCommandModeV2::Running
                {
                    let revision = revisions[index].clone();
                    Arc::make_mut(&mut self.queue).push(QueuedCommandV2 {
                        accepted: Arc::new(LivingAcceptedCommandV2 {
                            accepted_sequence: revision.ordinal,
                            application_tick: revision.tick,
                            envelope: LivingCommandEnvelopeV2 {
                                expected_committed_revision: revision.parent,
                                expected_tick: LivingTickV2(revision.tick.0.saturating_sub(1)),
                                expected_pending_sequence: 0,
                                mode: LivingCommandModeV2::Running,
                                command: revision.command.clone(),
                            },
                        })
                        .into(),
                        revision: Arc::new(revision).into(),
                    });
                    index += 1;
                }
                self.step()?;
            } else {
                return Err(LivingDeterministicFaultV2::Invariant);
            }
        }
        Ok(())
    }
}

// ------------------------------------------------------------------ phases

/// Phase 1: replay the accepted queue into the candidate in sequence order.
fn apply_queued_interventions(
    ctx: &mut LivingStepContextV2<'_>,
    queue: &[QueuedCommandV2],
) -> Result<(), LivingDeterministicFaultV2> {
    for queued in queue {
        let (effects, recoveries) =
            apply_revision_with_recoveries(ctx.state, &queued.revision, ctx.catalog).map_err(
                |reason| LivingDeterministicFaultV2::Replay {
                    accepted_sequence: queued.accepted.accepted_sequence,
                    reason: Box::new(reason),
                },
            )?;
        ctx.events.creator_recoveries.extend(recoveries);
        ctx.emit(
            LivingEventProvenanceV2::Creator {
                revision: queued.revision.id,
            },
            LivingEventKindV2::CreatorIntervention,
        )?;
        for kind in effects {
            ctx.emit(
                LivingEventProvenanceV2::Creator {
                    revision: queued.revision.id,
                },
                kind,
            )?;
        }
    }
    Ok(())
}

fn advance_lifecycles(ctx: &mut LivingStepContextV2<'_>) -> Result<(), LivingDeterministicFaultV2> {
    rules::advance_lifecycles(ctx)
}

fn resolve_arrivals(ctx: &mut LivingStepContextV2<'_>) -> Result<(), LivingDeterministicFaultV2> {
    rules::resolve_arrivals(ctx)
}

fn resolve_combat_occupation_and_claims(
    ctx: &mut LivingStepContextV2<'_>,
) -> Result<(), LivingDeterministicFaultV2> {
    rules::resolve_combat_occupation_and_claims(ctx)
}

fn complete_jobs_and_run_recipes(
    ctx: &mut LivingStepContextV2<'_>,
) -> Result<(), LivingDeterministicFaultV2> {
    rules::complete_jobs_and_run_recipes(ctx)
}

fn refresh_observations(
    ctx: &mut LivingStepContextV2<'_>,
) -> Result<(), LivingDeterministicFaultV2> {
    rules::refresh_observations(ctx)
}

fn update_diplomacy(ctx: &mut LivingStepContextV2<'_>) -> Result<(), LivingDeterministicFaultV2> {
    rules::update_diplomacy(ctx)
}

fn generate_and_reserve_intents(
    ctx: &mut LivingStepContextV2<'_>,
) -> Result<(), LivingDeterministicFaultV2> {
    rules::generate_and_reserve_intents(ctx)
}

fn dispatch_orders_and_freight(
    ctx: &mut LivingStepContextV2<'_>,
) -> Result<(), LivingDeterministicFaultV2> {
    rules::dispatch_orders_and_freight(ctx)
}

// ------------------------------------------------------ creator application

fn is_applied_by_this_slice(operation: &LivingCreatorOperationV2) -> bool {
    rules::creator_operation_applied(operation)
}

/// Apply one revision's operations to `state`, in declaration order, and
/// record the revision's sequence as the state's historical sequence.
///
/// References to existing entities that a creation names are left for
/// [`LivingGalaxyStateV2::validate`] to resolve, so this is not a second
/// validator. An edit must find its record here, because it has nothing to
/// write into otherwise.
fn apply_revision_with_recoveries(
    state: &mut LivingGalaxyStateV2,
    revision: &LivingRevisionV2,
    catalog: &ValidatedLivingCatalogPackV2,
) -> Result<
    (
        Vec<LivingEventKindV2>,
        Vec<LivingCreatorRecoveryCandidateV2>,
    ),
    LivingCommandRejectionV2,
> {
    use super::model::LivingCivilizationStatusV2;
    let dormant: Vec<_> = state
        .civilizations
        .as_slice()
        .iter()
        .filter(|row| {
            row.status == LivingCivilizationStatusV2::Dormant
                && !state
                    .colonies
                    .as_slice()
                    .iter()
                    .any(|colony| colony.owner == row.id)
        })
        .map(|row| row.id)
        .collect();
    let effects = apply_revision(state, revision, catalog)?;
    let recoveries = dormant
        .into_iter()
        .filter_map(|actor| {
            if !state
                .civilizations
                .as_slice()
                .iter()
                .any(|row| row.id == actor && row.status == LivingCivilizationStatusV2::Active)
            {
                return None;
            }
            state
                .colonies
                .as_slice()
                .iter()
                .find(|row| row.owner == actor)?;
            Some(LivingCreatorRecoveryCandidateV2 {
                revision: revision.id,
                actor,
            })
        })
        .collect();
    Ok((effects, recoveries))
}
fn bind_creator_recoveries(
    state: &LivingGalaxyStateV2,
    receipt: &LivingTickReceiptV2,
    candidates: &[LivingCreatorRecoveryCandidateV2],
) -> Vec<LivingCreatorRecoveryV2> {
    use super::model::LivingCivilizationStatusV2;
    let mut proofs: Vec<_> = candidates
        .iter()
        .filter(|candidate| {
            receipt.applied_revisions.contains(&candidate.revision)
                && state.civilizations.as_slice().iter().any(|row| {
                    row.id == candidate.actor && row.status == LivingCivilizationStatusV2::Active
                })
                && state
                    .colonies
                    .as_slice()
                    .iter()
                    .any(|row| row.owner == candidate.actor)
                && receipt.events.iter().any(|event| {
                    event.kind == LivingEventKindV2::CreatorIntervention
                        && event.provenance
                            == LivingEventProvenanceV2::Creator {
                                revision: candidate.revision,
                            }
                })
        })
        .map(|candidate| LivingCreatorRecoveryV2 {
            revision: candidate.revision,
            actor: candidate.actor,
            world: state
                .colonies
                .as_slice()
                .iter()
                .filter(|row| row.owner == candidate.actor)
                .map(|row| row.world)
                .min()
                .expect("filter proved an owned colony"),
            receipt_digest: receipt.digest,
        })
        .collect();
    proofs.sort_by_key(|proof| (proof.revision, proof.actor, proof.world));
    proofs
}

fn apply_revision(
    state: &mut LivingGalaxyStateV2,
    revision: &LivingRevisionV2,
    catalog: &ValidatedLivingCatalogPackV2,
) -> Result<Vec<LivingEventKindV2>, LivingCommandRejectionV2> {
    apply_revision_range(state, revision, catalog, 0)
}

fn apply_revision_range(
    state: &mut LivingGalaxyStateV2,
    revision: &LivingRevisionV2,
    catalog: &ValidatedLivingCatalogPackV2,
    first_operation: usize,
) -> Result<Vec<LivingEventKindV2>, LivingCommandRejectionV2> {
    let mut effects = Vec::new();
    let created = revision.created_entities();
    let resolve = |target: &LivingCommandTargetV2| -> LivingEntityIdV2 {
        match *target {
            LivingCommandTargetV2::Existing { id } => id,
            LivingCommandTargetV2::Local { local } => created
                .iter()
                .find(|(declared, _, _)| *declared == local)
                .map(|(_, _, id)| *id)
                // `LivingCommandV2::validate` already proved every local
                // reference names an earlier declaration.
                .unwrap_or(LivingEntityIdV2([0; 16])),
        }
    };
    let created_id =
        |local: u16| -> LivingEntityIdV2 { resolve(&LivingCommandTargetV2::Local { local }) };

    for (index, operation) in revision
        .command
        .operations()
        .iter()
        .enumerate()
        .skip(first_operation)
    {
        match operation {
            LivingCreatorOperationV2::CreateSystem { local, name } => {
                insert(
                    &mut state.systems,
                    LivingSystemV2 {
                        id: created_id(*local),
                        name: name.clone(),
                    },
                )?;
            }
            LivingCreatorOperationV2::CreateStar {
                local,
                system,
                archetype,
                name,
            } => {
                insert(
                    &mut state.stars,
                    LivingStarV2 {
                        id: created_id(*local),
                        system: resolve(system),
                        archetype: archetype.clone(),
                        name: name.clone(),
                    },
                )?;
            }
            LivingCreatorOperationV2::CreateWorld {
                local,
                system,
                archetype,
                name,
                owner,
                inventory,
            } => {
                insert(
                    &mut state.worlds,
                    LivingWorldV2 {
                        id: created_id(*local),
                        system: resolve(system),
                        archetype: archetype.clone(),
                        name: name.clone(),
                        owner: owner.as_ref().map(resolve),
                        inventory: *inventory,
                    },
                )?;
            }
            LivingCreatorOperationV2::CreateLane {
                local,
                system_a,
                system_b,
                distance_units,
            } => {
                let first = resolve(system_a);
                let second = resolve(system_b);
                insert(
                    &mut state.lanes,
                    LivingLaneV2 {
                        id: created_id(*local),
                        system_a: first.min(second),
                        system_b: first.max(second),
                        distance_units: *distance_units,
                    },
                )?;
            }
            LivingCreatorOperationV2::CreateCivilization {
                local,
                name,
                policy,
                status,
            } => {
                insert(
                    &mut state.civilizations,
                    LivingCivilizationV2 {
                        id: created_id(*local),
                        name: name.clone(),
                        policy: policy.clone(),
                        status: *status,
                    },
                )?;
            }
            LivingCreatorOperationV2::CreateDeposit {
                local,
                world,
                resource,
                remaining_units,
            } => {
                insert(
                    &mut state.deposits,
                    LivingDepositV2 {
                        id: created_id(*local),
                        world: resolve(world),
                        resource: *resource,
                        remaining_units: *remaining_units,
                    },
                )?;
            }
            LivingCreatorOperationV2::SetWorldInventory { world, inventory } => {
                edit(&mut state.worlds, resolve(world), index, "world")?.inventory = *inventory;
            }
            LivingCreatorOperationV2::SetDepositRemaining {
                deposit,
                remaining_units,
            } => {
                edit(&mut state.deposits, resolve(deposit), index, "deposit")?.remaining_units =
                    *remaining_units;
            }
            LivingCreatorOperationV2::SetCivilizationPolicy {
                civilization,
                policy,
            } => {
                edit(
                    &mut state.civilizations,
                    resolve(civilization),
                    index,
                    "civilization",
                )?
                .policy = policy.clone();
            }
            other => {
                rules::apply_creator_operation(
                    state,
                    revision.tick,
                    index,
                    other,
                    &resolve,
                    &mut effects,
                    catalog,
                )?;
            }
        }
    }
    state.accepted_sequence = revision.ordinal;
    Ok(effects)
}

/// Insert one record at its sorted position. A repeated identity is a
/// validation failure, reported the way the state validator reports it.
fn insert<T>(
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
        *collection = LivingSortedVecV2::new(records)?;
        return Err(LivingValidationErrorV2::DuplicateIdentity {
            context: "creator batch",
        });
    }
    records.insert(position, record);
    *collection = LivingSortedVecV2::new(records)?;
    Ok(())
}

fn edit<'a, T>(
    collection: &'a mut LivingSortedVecV2<T>,
    id: LivingEntityIdV2,
    operation: usize,
    reference: &'static str,
) -> Result<&'a mut T, LivingCommandRejectionV2>
where
    T: LivingKeyedV2<Key = LivingEntityIdV2>,
{
    collection
        .find_mut(&id)
        .ok_or(LivingCommandRejectionV2::MissingTarget {
            operation,
            reference,
        })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::living::LIVING_RULES_VERSION;
    use crate::living::catalog::{LivingNameV2, LivingSlugV2, living_core_pack_v2};
    use crate::living::command::LivingCommandV2;
    use crate::living::genesis::{
        LIVING_GENESIS_FORMAT_VERSION_V2, LIVING_GENESIS_KIND_V2, LivingGenesisManifestV2,
        validate_living_genesis_manifest_v2,
    };

    fn authority() -> LivingGalaxyAuthorityV2 {
        let catalog = living_core_pack_v2().unwrap();
        let genesis = validate_living_genesis_manifest_v2(
            LivingGenesisManifestV2 {
                kind: LIVING_GENESIS_KIND_V2.to_owned(),
                format_version: LIVING_GENESIS_FORMAT_VERSION_V2,
                rules_version: LIVING_RULES_VERSION,
                state: LivingGalaxyStateV2::default(),
            },
            &catalog,
        )
        .unwrap();
        LivingGalaxyAuthorityV2::from_genesis(
            catalog,
            [0x33; 32],
            LivingGenesisGeneratorV2 {
                generator_id: LivingSlugV2::new("unit").unwrap(),
                generator_version: 1,
            },
            genesis,
        )
        .unwrap()
    }

    fn prepare_staged(
        authority: &LivingGalaxyAuthorityV2,
        budget: usize,
    ) -> LivingStepPreparationV2 {
        let mut job = authority.begin_step().unwrap();
        assert_eq!(job.poll(0), Err(LivingStepJobErrorV2::Budget));
        assert_eq!(job.last_poll_visits(), 0);
        loop {
            let progress = job.poll(budget).unwrap();
            assert!(job.last_poll_visits() <= budget);
            if progress != LivingStepProgressV2::Pending {
                break;
            }
        }
        job.finish_synchronously(authority).unwrap()
    }

    #[test]
    fn staged_step_dispatch_is_prepared_before_sealing_on_non_intent_boundaries() {
        let base = scheduled_staged_fixture();
        let mut reference = base.clone();
        reference.step().unwrap();
        let mut expected = reference.state().clone();
        expected.tick = base.state().tick;
        let mut total = None;
        for budget in [1, 64, 1024] {
            let mut staged = base.clone();
            let capture = staged.capture_boundary().unwrap();
            let mut job = staged.begin_step().unwrap();
            loop {
                let progress = job.poll(budget).unwrap();
                assert!(job.last_poll_visits() <= budget);
                assert!(staged.boundary_capture_is_current(&capture));
                if progress == LivingStepProgressV2::NeedsSynchronousCompletion {
                    break;
                }
                assert_eq!(progress, LivingStepProgressV2::Pending);
            }
            // Dispatch must already have changed private resources/manifests;
            // final sealing must not repeat it or defer it outside polling.
            assert_eq!(job.completed_output.as_ref().unwrap().0, expected);
            if let Some(total) = total {
                assert_eq!(job.total_visits(), total);
            } else {
                total = Some(job.total_visits());
            }
            let LivingStepPreparationV2::Ready(prepared) =
                job.finish_synchronously(&staged).unwrap()
            else {
                panic!("unexpected dispatch failure");
            };
            drop(staged.commit_step(prepared).unwrap());
            assert_eq!(staged.state(), reference.state());
            assert_eq!(staged.last_receipt(), reference.last_receipt());
            assert_eq!(staged.last_decision(), reference.last_decision());
            assert_eq!(staged.last_event_facts(), reference.last_event_facts());
            assert_eq!(
                staged.last_corridor_restrictions(),
                reference.last_corridor_restrictions()
            );
        }
    }

    #[test]
    fn staged_step_dispatch_cancel_not_ready_and_stale_tail_preserve_authority() {
        for mode in 0..3 {
            let mut authority = scheduled_staged_fixture();
            let before = authority.capture_boundary().unwrap();
            let mut job = authority.begin_step().unwrap();
            loop {
                assert_eq!(job.poll(1).unwrap(), LivingStepProgressV2::Pending);
                if job.dispatch.is_some() {
                    break;
                }
            }
            if mode == 0 {
                job.cancel();
                assert_eq!(job.poll(1), Err(LivingStepJobErrorV2::Cancelled));
                assert!(matches!(
                    job.finish_synchronously(&authority),
                    Err(LivingStepJobErrorV2::Cancelled)
                ));
                assert!(authority.boundary_capture_is_current(&before));
            } else if mode == 1 {
                assert!(matches!(
                    job.finish_synchronously(&authority),
                    Err(LivingStepJobErrorV2::NotReady)
                ));
                assert!(authority.boundary_capture_is_current(&before));
            } else {
                submit(&mut authority, vec![system("Tail during dispatch")]);
                let current = authority.capture_boundary().unwrap();
                while job.poll(1).unwrap() == LivingStepProgressV2::Pending {}
                assert!(matches!(
                    job.finish_synchronously(&authority),
                    Err(LivingStepJobErrorV2::Stale)
                ));
                assert!(authority.boundary_capture_is_current(&current));
                assert_eq!(authority.queue.len(), 1);
            }
            assert!(authority.fault.is_none());
        }
    }

    #[test]
    fn staged_step_matches_synchronous_state_receipts_queue_and_journal_at_all_budgets() {
        for budget in [1, 64, 1024] {
            let mut staged = super::super::rules::builtin_living_sandbox_v2().unwrap();
            submit(&mut staged, vec![system("Staged running revision")]);
            let mut reference = staged.clone();
            for _ in 0..55 {
                let before = staged.state_snapshot();
                let expected = reference.step().unwrap();
                let LivingStepPreparationV2::Ready(prepared) = prepare_staged(&staged, budget)
                else {
                    panic!("unexpected staged failure");
                };
                assert!(Arc::ptr_eq(&before, &staged.state));
                let retired = staged.commit_step(prepared).unwrap();
                assert_eq!(staged.state(), reference.state());
                assert_eq!(staged.last_receipt(), Some(&expected));
                assert_eq!(staged.last_decision(), reference.last_decision());
                assert_eq!(staged.last_event_facts(), reference.last_event_facts());
                assert_eq!(staged.published_cursor(), reference.published_cursor());
                assert_eq!(staged.revisions, reference.revisions);
                assert_eq!(staged.revision_modes, reference.revision_modes);
                assert_eq!(
                    staged.accepted_sequence_high_water,
                    reference.accepted_sequence_high_water
                );
                assert!(Arc::ptr_eq(
                    &retired._capture.as_ref().unwrap().state,
                    &before
                ));
                drop(retired);
            }
        }
    }

    #[test]
    fn staged_step_rejects_stale_prepared_success_without_consuming_new_tail() {
        let mut authority = authority();
        submit(&mut authority, vec![system("First")]);
        let LivingStepPreparationV2::Ready(prepared) = prepare_staged(&authority, 1) else {
            panic!("unexpected failure");
        };
        submit(&mut authority, vec![system("New tail")]);
        let current = authority.capture_boundary().unwrap();
        let rejected = authority.commit_step(prepared).unwrap_err();
        assert_eq!(rejected.reason(), &LivingStepJobErrorV2::Stale);
        assert!(authority.boundary_capture_is_current(&current));
        assert_eq!(authority.queue.len(), 2);
        assert!(authority.last_receipt().is_none());
        assert!(authority.fault.is_none());
    }

    #[test]
    fn staged_step_cancellation_and_not_ready_never_publish_or_fault() {
        let mut authority = authority();
        submit(&mut authority, vec![system("Retained")]);
        let current = authority.capture_boundary().unwrap();
        let job = authority.begin_step().unwrap();
        assert!(matches!(
            job.finish_synchronously(&authority),
            Err(LivingStepJobErrorV2::NotReady)
        ));
        let mut job = authority.begin_step().unwrap();
        job.poll(1).unwrap();
        job.cancel();
        assert_eq!(job.poll(1), Err(LivingStepJobErrorV2::Cancelled));
        assert!(matches!(
            job.finish_synchronously(&authority),
            Err(LivingStepJobErrorV2::Cancelled)
        ));
        assert!(authority.boundary_capture_is_current(&current));
        assert_eq!(authority.queue.len(), 1);
    }

    #[test]
    fn staged_step_max_tick_failure_is_authentic_atomic_and_stale_safe() {
        let mut authority = authority();
        Arc::make_mut(&mut authority.state).tick = LivingTickV2(u64::MAX);
        let digest = authority.state.digest().unwrap();
        let mut reference = authority.clone();
        assert_eq!(
            reference.step(),
            Err(LivingDeterministicFaultV2::Arithmetic)
        );
        let mut job = authority.begin_step().unwrap();
        assert_eq!(
            job.poll(1),
            Err(LivingStepJobErrorV2::Fault(
                LivingDeterministicFaultV2::Arithmetic
            ))
        );
        assert_eq!(job.last_poll_visits(), 0);
        let LivingStepPreparationV2::Failed(failure) =
            job.finish_synchronously(&authority).unwrap()
        else {
            panic!("expected failure");
        };
        assert_eq!(failure.fault(), &LivingDeterministicFaultV2::Arithmetic);
        assert!(authority.fault.is_none());
        let retired = authority.commit_step_failure(failure).unwrap();
        assert_eq!(authority.fault, reference.fault);
        assert_eq!(authority.state.digest().unwrap(), digest);
        drop(retired);
        authority.discard_faulted_queue();
        let job = authority.begin_step().unwrap();
        let LivingStepPreparationV2::Failed(failure) =
            job.finish_synchronously(&authority).unwrap()
        else {
            panic!("expected failure");
        };
        Arc::make_mut(&mut authority.state).tick = LivingTickV2(u64::MAX - 1);
        let rejected = authority.commit_step_failure(failure).unwrap_err();
        assert_eq!(rejected.reason(), &LivingStepJobErrorV2::Stale);
        assert!(authority.fault.is_none());
    }

    fn paused_batch(
        authority: &mut LivingGalaxyAuthorityV2,
        operations: Vec<LivingCreatorOperationV2>,
    ) -> Result<LivingTickReceiptV2, LivingCommandRejectionV2> {
        let cursor = authority.published_cursor();
        authority.apply_paused(LivingCommandEnvelopeV2 {
            expected_committed_revision: cursor.committed_revision,
            expected_tick: cursor.tick,
            expected_pending_sequence: cursor.pending_sequence,
            mode: LivingCommandModeV2::Paused,
            command: LivingCommandV2::CreatorBatch { operations },
        })
    }
    fn recovery_fixture() -> (LivingGalaxyAuthorityV2, LivingEntityIdV2, LivingEntityIdV2) {
        let mut authority = super::super::rules::builtin_living_sandbox_v2().unwrap();
        let actor = authority.state.civilizations.as_slice()[0].id;
        let worlds: Vec<_> = authority
            .state
            .colonies
            .as_slice()
            .iter()
            .filter(|row| row.owner == actor)
            .map(|row| row.world)
            .collect();
        let world = worlds[0];
        paused_batch(
            &mut authority,
            worlds
                .into_iter()
                .map(|world| LivingCreatorOperationV2::RemoveColony {
                    world: LivingCommandTargetV2::Existing { id: world },
                })
                .collect(),
        )
        .unwrap();
        assert!(
            authority
                .state
                .civilizations
                .as_slice()
                .iter()
                .any(|row| row.id == actor
                    && row.status == super::super::model::LivingCivilizationStatusV2::Dormant)
        );
        assert!(authority.last_creator_recoveries().is_empty());
        (authority, actor, world)
    }
    fn create_recovery(
        actor: LivingEntityIdV2,
        world: LivingEntityIdV2,
    ) -> LivingCreatorOperationV2 {
        LivingCreatorOperationV2::CreateColony {
            world: LivingCommandTargetV2::Existing { id: world },
            owner: LivingCommandTargetV2::Existing { id: actor },
        }
    }
    #[test]
    fn creator_recovery_proof_binds_paused_running_and_replay_without_new_events() {
        for mode in [LivingCommandModeV2::Paused, LivingCommandModeV2::Running] {
            let (mut authority, actor, world) = recovery_fixture();
            let mut replay = authority.clone();
            if mode == LivingCommandModeV2::Paused {
                paused_batch(&mut authority, vec![create_recovery(actor, world)]).unwrap();
                let revision = authority.revisions.last().unwrap().clone();
                replay
                    .replay_paused(&LivingRecordedRevisionV2 { revision, mode })
                    .unwrap();
            } else {
                submit(&mut authority, vec![create_recovery(actor, world)]);
                assert!(authority.last_creator_recoveries().is_empty());
                let record = LivingRecordedRevisionV2 {
                    revision: authority.queue.last().unwrap().revision.as_ref().clone(),
                    mode,
                };
                let mut boundary = replay.begin_replay_boundary().unwrap();
                replay.stage_replay_record(&mut boundary, &record).unwrap();
                replay.finish_replay_boundary(boundary).unwrap();
                authority.step().unwrap();
            }
            let proofs = authority.last_creator_recoveries();
            assert_eq!(proofs.len(), 1);
            assert_eq!(proofs[0].actor(), actor);
            assert_eq!(proofs[0].world(), world);
            assert_eq!(
                proofs[0].receipt_digest(),
                authority.last_receipt().unwrap().digest
            );
            assert_eq!(Some(proofs[0].revision()), authority.committed_revision);
            assert_eq!(proofs, replay.last_creator_recoveries());
            assert_eq!(authority.last_receipt(), replay.last_receipt());
            assert!(
                !authority
                    .last_receipt()
                    .unwrap()
                    .events
                    .iter()
                    .any(
                        |event| event.kind == LivingEventKindV2::CivilizationReactivated
                            && matches!(event.provenance, LivingEventProvenanceV2::Creator { .. })
                    )
            );
            let previous = proofs.to_vec();
            assert!(
                paused_batch(
                    &mut authority,
                    vec![create_recovery(actor, LivingEntityIdV2([0xff; 16]))]
                )
                .is_err()
            );
            assert_eq!(authority.last_creator_recoveries(), previous);
            authority.step().unwrap();
            assert!(authority.last_creator_recoveries().is_empty());
        }
    }
    #[test]
    fn creator_recovery_proof_excludes_transient_batches_and_later_running_loss() {
        let (mut authority, actor, world) = recovery_fixture();
        paused_batch(
            &mut authority,
            vec![
                create_recovery(actor, world),
                LivingCreatorOperationV2::RemoveColony {
                    world: LivingCommandTargetV2::Existing { id: world },
                },
            ],
        )
        .unwrap();
        assert!(authority.last_creator_recoveries().is_empty());
        submit(&mut authority, vec![create_recovery(actor, world)]);
        submit(
            &mut authority,
            vec![LivingCreatorOperationV2::RemoveColony {
                world: LivingCommandTargetV2::Existing { id: world },
            }],
        );
        authority.step().unwrap();
        assert!(authority.last_creator_recoveries().is_empty());
        let (mut authority, actor, world) = recovery_fixture();
        let other = authority
            .state
            .worlds
            .as_slice()
            .iter()
            .find(|row| row.owner == Some(actor) && row.id != world)
            .unwrap()
            .id;
        submit(
            &mut authority,
            vec![create_recovery(actor, world), create_recovery(actor, other)],
        );
        let recovered_revision = authority.queue.last().unwrap().revision.id;
        submit(
            &mut authority,
            vec![LivingCreatorOperationV2::RemoveColony {
                world: LivingCommandTargetV2::Existing { id: world },
            }],
        );
        authority.step().unwrap();
        assert_eq!(authority.last_creator_recoveries().len(), 1);
        assert_eq!(
            authority.last_creator_recoveries()[0].revision(),
            recovered_revision
        );
        assert_eq!(authority.last_creator_recoveries()[0].world(), other);
    }

    #[test]
    fn creator_recovery_proof_excludes_new_actors_and_preowned_dormant_status_refresh() {
        let mut authority = super::super::rules::builtin_living_sandbox_v2().unwrap();
        let actor = authority.state.civilizations.as_slice()[0].id;
        Arc::make_mut(&mut authority.state)
            .civilizations
            .find_mut(&actor)
            .unwrap()
            .status = super::super::model::LivingCivilizationStatusV2::Dormant;
        paused_batch(&mut authority, vec![system("Unrelated")]).unwrap();
        assert!(authority.last_creator_recoveries().is_empty());
        let world = authority.state.worlds.as_slice()[0].id;
        paused_batch(
            &mut authority,
            vec![
                LivingCreatorOperationV2::CreateCivilization {
                    local: 0,
                    name: LivingNameV2::new("New civilization").unwrap(),
                    policy: super::super::model::LivingPolicyV2(
                        LivingSlugV2::new("neutral").unwrap(),
                    ),
                    status: super::super::model::LivingCivilizationStatusV2::Dormant,
                },
                LivingCreatorOperationV2::SetWorldOwner {
                    world: LivingCommandTargetV2::Existing { id: world },
                    owner: Some(LivingCommandTargetV2::Local { local: 0 }),
                },
            ],
        )
        .unwrap();
        assert!(authority.last_creator_recoveries().is_empty());
    }

    #[test]
    fn staged_step_phase_one_slices_local_creation_and_1024_scalar_operations() {
        let mut base = authority();
        submit(
            &mut base,
            vec![
                system("Local first"),
                LivingCreatorOperationV2::CreateStar {
                    local: 1,
                    system: LivingCommandTargetV2::Local { local: 0 },
                    archetype: LivingSlugV2::new("main_sequence").unwrap(),
                    name: LivingNameV2::new("Star").unwrap(),
                },
                LivingCreatorOperationV2::CreateWorld {
                    local: 2,
                    system: LivingCommandTargetV2::Local { local: 0 },
                    archetype: LivingSlugV2::new("barren").unwrap(),
                    name: LivingNameV2::new("World").unwrap(),
                    owner: None,
                    inventory: super::super::model::LivingInventoryV2::default(),
                },
                LivingCreatorOperationV2::CreateSystem {
                    local: 3,
                    name: LivingNameV2::new("Second system").unwrap(),
                },
                LivingCreatorOperationV2::CreateLane {
                    local: 4,
                    system_a: LivingCommandTargetV2::Local { local: 0 },
                    system_b: LivingCommandTargetV2::Local { local: 3 },
                    distance_units: 10,
                },
                LivingCreatorOperationV2::CreateCivilization {
                    local: 5,
                    name: LivingNameV2::new("Dormant").unwrap(),
                    policy: super::super::model::LivingPolicyV2(
                        LivingSlugV2::new("neutral").unwrap(),
                    ),
                    status: super::super::model::LivingCivilizationStatusV2::Dormant,
                },
                LivingCreatorOperationV2::CreateDeposit {
                    local: 6,
                    world: LivingCommandTargetV2::Local { local: 2 },
                    resource: super::super::model::LivingResourceV2::Ore,
                    remaining_units: 20,
                },
                LivingCreatorOperationV2::SetWorldInventory {
                    world: LivingCommandTargetV2::Local { local: 2 },
                    inventory: super::super::model::LivingInventoryV2 {
                        energy: 4,
                        ore: 3,
                        alloy: 2,
                    },
                },
            ],
        );
        let world = base.queue[0]
            .revision
            .created_entities()
            .iter()
            .find(|(local, _, _)| *local == 2)
            .unwrap()
            .2;
        submit(
            &mut base,
            (0..1024)
                .map(|index| LivingCreatorOperationV2::SetWorldInventory {
                    world: LivingCommandTargetV2::Existing { id: world },
                    inventory: super::super::model::LivingInventoryV2 {
                        energy: index,
                        ore: 0,
                        alloy: 0,
                    },
                })
                .collect(),
        );
        let mut reference = base.clone();
        reference.step().unwrap();
        let mut total = None;
        for budget in [1, 64, 1024] {
            let mut authority = base.clone();
            let mut job = authority.begin_step().unwrap();
            loop {
                let progress = job.poll(budget).unwrap();
                assert!(job.last_poll_visits() <= budget);
                assert_ne!(
                    progress,
                    LivingStepProgressV2::NeedsSynchronousCreatorOperation
                );
                if progress == LivingStepProgressV2::NeedsSynchronousCompletion {
                    break;
                }
            }
            assert!(
                job.total_visits() > 2048,
                "declarations and individual scalar edits each consume visits"
            );
            if let Some(previous) = total {
                assert_eq!(job.total_visits(), previous);
            }
            total = Some(job.total_visits());
            let LivingStepPreparationV2::Ready(prepared) =
                job.finish_synchronously(&authority).unwrap()
            else {
                panic!("unexpected failure");
            };
            let retired = authority.commit_step(prepared).unwrap();
            assert_eq!(authority.state(), reference.state());
            assert_eq!(authority.last_receipt(), reference.last_receipt());
            assert_eq!(authority.last_decision(), reference.last_decision());
            assert_eq!(authority.last_event_facts(), reference.last_event_facts());
            drop(retired);
        }
    }
    #[test]
    fn staged_step_explicit_creator_fallback_preserves_prefix_local_ids_and_recovery() {
        let (mut base, actor, world) = recovery_fixture();
        submit(
            &mut base,
            vec![
                system("Core prefix"),
                LivingCreatorOperationV2::SetWorldInventory {
                    world: LivingCommandTargetV2::Existing { id: world },
                    inventory: super::super::model::LivingInventoryV2 {
                        energy: 90,
                        ore: 60,
                        alloy: 40,
                    },
                },
                create_recovery(actor, world),
            ],
        );
        let mut reference = base.clone();
        reference.step().unwrap();
        for budget in [1, 64, 1024] {
            let mut authority = base.clone();
            let mut job = authority.begin_step().unwrap();
            loop {
                let progress = job.poll(budget).unwrap();
                assert!(job.last_poll_visits() <= budget);
                if progress == LivingStepProgressV2::NeedsSynchronousCreatorOperation {
                    break;
                }
                assert_ne!(progress, LivingStepProgressV2::NeedsSynchronousCompletion);
            }
            let LivingStepPreparationV2::Ready(prepared) =
                job.finish_synchronously(&authority).unwrap()
            else {
                panic!("unexpected failure");
            };
            let retired = authority.commit_step(prepared).unwrap();
            assert_eq!(authority.state(), reference.state());
            assert_eq!(authority.last_receipt(), reference.last_receipt());
            assert_eq!(
                authority.last_creator_recoveries(),
                reference.last_creator_recoveries()
            );
            drop(retired);
        }
    }

    #[test]
    fn staged_step_phase_one_faults_match_reference_and_keep_private_insertion_unpublished() {
        let mut base = authority();
        submit(&mut base, vec![system("Collision")]);
        let id = base.queue[0].revision.created_entities()[0].2;
        Arc::make_mut(&mut base.state)
            .systems
            .push_sorted(LivingSystemV2 {
                id,
                name: LivingNameV2::new("Injected collision").unwrap(),
            })
            .unwrap();
        let before = base.state.digest().unwrap();
        let mut reference = base.clone();
        let expected = reference.step().unwrap_err();
        for budget in [1, 64, 1024] {
            let mut authority = base.clone();
            let mut job = authority.begin_step().unwrap();
            loop {
                match job.poll(budget) {
                    Ok(LivingStepProgressV2::Pending) => {}
                    Err(LivingStepJobErrorV2::Fault(fault)) => {
                        assert_eq!(fault, expected);
                        break;
                    }
                    other => panic!("unexpected phase-one result: {other:?}"),
                }
                assert!(job.last_poll_visits() <= budget);
            }
            assert!(job.last_poll_visits() <= budget);
            assert_eq!(
                job.poll(1),
                Err(LivingStepJobErrorV2::Fault(expected.clone()))
            );
            assert_eq!(job.last_poll_visits(), 0);
            let LivingStepPreparationV2::Failed(failure) =
                job.finish_synchronously(&authority).unwrap()
            else {
                panic!("expected authentic failure");
            };
            let retired = authority.commit_step_failure(failure).unwrap();
            assert_eq!(authority.state.digest().unwrap(), before);
            assert_eq!(authority.queue.len(), 1);
            assert_eq!(authority.fault, Some(expected.clone()));
            drop(retired);
        }
    }

    pub(super) fn scheduled_staged_fixture() -> LivingGalaxyAuthorityV2 {
        let mut authority = super::super::rules::builtin_living_sandbox_v2().unwrap();
        let state = Arc::make_mut(&mut authority.state);
        state.tick = LivingTickV2(9);
        let a = state.civilizations.as_slice()[0].id;
        let b = state.civilizations.as_slice()[1].id;
        state
            .wars
            .push_sorted(super::super::model::LivingWarV2 {
                participant_a: a.min(b),
                participant_b: a.max(b),
                declarer: a,
                start_tick: LivingTickV2(0),
                end_tick: LivingTickV2(10),
            })
            .unwrap();
        let owned: Vec<_> = state
            .worlds
            .as_slice()
            .iter()
            .filter(|row| row.owner == Some(a))
            .map(|row| row.id)
            .collect();
        let source = owned[0];
        let destination = owned[1];
        let fleet = state.fleets.record_at_mut(0).unwrap();
        fleet.location = super::super::model::LivingFleetLocationV2::Travelling {
            origin_world: source,
            destination_world: destination,
            departure_tick: LivingTickV2(0),
            arrival_tick: LivingTickV2(10),
        };
        for hull in fleet
            .hulls
            .as_slice()
            .iter()
            .map(|row| row.id)
            .collect::<Vec<_>>()
        {
            fleet.hulls.find_mut(&hull).unwrap().return_credit = true;
        }
        state
            .shipments
            .push_sorted(super::super::model::LivingShipmentV2 {
                id: LivingEntityIdV2([240; 16]),
                dispatch_owner: a,
                intended_receiver: a,
                source_world: source,
                destination_world: destination,
                resource: super::super::model::LivingResourceV2::Energy,
                units: 7,
                departure_tick: LivingTickV2(0),
                arrival_tick: LivingTickV2(10),
                disposition: super::super::model::LivingShipmentDispositionV2::Outbound,
                dispatch_kind: Some(super::super::model::LivingRouteKindV2::Internal),
                returning_to_source: false,
            })
            .unwrap();
        state.validate(&authority.catalog).unwrap();
        authority
    }
    #[test]
    fn staged_step_scheduled_mixed_budgets_preserve_expiry_arrival_delivery_and_receipts() {
        let base = scheduled_staged_fixture();
        let mut reference = base.clone();
        reference.step().unwrap();
        let mut visits = None;
        for budgets in [&[1][..], &[64][..], &[1024][..], &[1, 64, 3, 1024][..]] {
            let mut authority = base.clone();
            let mut job = authority.begin_step().unwrap();
            let mut index = 0;
            loop {
                let budget = budgets[index % budgets.len()];
                index += 1;
                let progress = job.poll(budget).unwrap();
                assert!(job.last_poll_visits() <= budget);
                assert_ne!(
                    progress,
                    LivingStepProgressV2::NeedsSynchronousCreatorOperation
                );
                if progress == LivingStepProgressV2::NeedsSynchronousCompletion {
                    break;
                }
            }
            if let Some(previous) = visits {
                assert_eq!(job.total_visits(), previous);
            }
            visits = Some(job.total_visits());
            let LivingStepPreparationV2::Ready(prepared) =
                job.finish_synchronously(&authority).unwrap()
            else {
                panic!("unexpected failure");
            };
            let retired = authority.commit_step(prepared).unwrap();
            assert_eq!(authority.state(), reference.state());
            assert_eq!(authority.last_receipt(), reference.last_receipt());
            assert_eq!(authority.last_decision(), reference.last_decision());
            assert_eq!(authority.last_event_facts(), reference.last_event_facts());
            for kind in [
                LivingEventKindV2::WarEnded,
                LivingEventKindV2::FleetArrived,
                LivingEventKindV2::ShipmentDelivered,
            ] {
                assert_eq!(
                    authority
                        .last_receipt()
                        .unwrap()
                        .events
                        .iter()
                        .filter(|event| event.kind == kind)
                        .count(),
                    1
                );
            }
            drop(retired);
        }
    }
    #[test]
    fn staged_step_cancel_and_pending_tail_during_each_scheduled_phase_preserve_authority() {
        for phase in [
            LivingPhaseV2::ExpireLifecycles,
            LivingPhaseV2::ResolveArrivals,
        ] {
            let mut authority = scheduled_staged_fixture();
            let before = authority.capture_boundary().unwrap();
            let mut job = authority.begin_step().unwrap();
            loop {
                job.poll(1).unwrap();
                if job
                    .scheduled
                    .as_ref()
                    .and_then(|scheduled| scheduled.phase())
                    == Some(phase)
                {
                    break;
                }
            }
            job.cancel();
            assert_eq!(job.poll(1), Err(LivingStepJobErrorV2::Cancelled));
            assert!(matches!(
                job.finish_synchronously(&authority),
                Err(LivingStepJobErrorV2::Cancelled)
            ));
            assert!(authority.boundary_capture_is_current(&before));
            let mut job = authority.begin_step().unwrap();
            loop {
                job.poll(1).unwrap();
                if job
                    .scheduled
                    .as_ref()
                    .and_then(|scheduled| scheduled.phase())
                    == Some(phase)
                {
                    break;
                }
            }
            submit(&mut authority, vec![system("Tail while scheduled")]);
            let current = authority.capture_boundary().unwrap();
            loop {
                if job.poll(1).unwrap() == LivingStepProgressV2::NeedsSynchronousCompletion {
                    break;
                }
            }
            assert!(matches!(
                job.finish_synchronously(&authority),
                Err(LivingStepJobErrorV2::Stale)
            ));
            assert!(authority.boundary_capture_is_current(&current));
            assert_eq!(authority.queue.len(), 1);
            assert_eq!(authority.state.tick, LivingTickV2(9));
            assert!(authority.last_receipt().is_none());
            assert!(authority.fault.is_none());
        }
    }
    #[test]
    fn staged_step_phase_four_through_six_cancel_and_stale_tail_are_atomic() {
        for phase in 4..=6 {
            let mut authority = scheduled_staged_fixture();
            let before = authority.capture_boundary().unwrap();
            let reached = |job: &LivingStepJobV2| match phase {
                4 => job.conflict.is_some(),
                5 => job.production.is_some(),
                _ => job.observation.is_some(),
            };
            let mut cancelled = authority.begin_step().unwrap();
            loop {
                assert_eq!(cancelled.poll(1).unwrap(), LivingStepProgressV2::Pending);
                if reached(&cancelled) {
                    break;
                }
            }
            cancelled.cancel();
            assert_eq!(cancelled.poll(1), Err(LivingStepJobErrorV2::Cancelled));
            assert!(matches!(
                cancelled.finish_synchronously(&authority),
                Err(LivingStepJobErrorV2::Cancelled)
            ));
            assert!(authority.boundary_capture_is_current(&before));
            let mut stale = authority.begin_step().unwrap();
            loop {
                assert_eq!(stale.poll(1).unwrap(), LivingStepProgressV2::Pending);
                if reached(&stale) {
                    break;
                }
            }
            submit(&mut authority, vec![system("Tail during later phase")]);
            let current = authority.capture_boundary().unwrap();
            loop {
                if stale.poll(1).unwrap() == LivingStepProgressV2::NeedsSynchronousCompletion {
                    break;
                }
            }
            assert!(matches!(
                stale.finish_synchronously(&authority),
                Err(LivingStepJobErrorV2::Stale)
            ));
            assert!(authority.boundary_capture_is_current(&current));
            assert_eq!(authority.queue.len(), 1);
            assert!(authority.fault.is_none());
        }
    }

    #[test]
    fn staged_step_empty_completed_phases_are_explicit_and_match_all_receipts() {
        let base = authority();
        let mut reference = base.clone();
        reference.step().unwrap();
        let mut total = None;
        for budgets in [&[1][..], &[64][..], &[1024][..], &[1, 3, 64, 1024][..]] {
            let mut staged = base.clone();
            let mut job = staged.begin_step().unwrap();
            let mut index = 0;
            loop {
                let budget = budgets[index % budgets.len()];
                index += 1;
                let progress = job.poll(budget).unwrap();
                assert!(job.last_poll_visits() <= budget);
                if progress == LivingStepProgressV2::NeedsSynchronousCompletion {
                    break;
                }
                assert_eq!(progress, LivingStepProgressV2::Pending);
            }
            let output = job.completed_output.as_ref().unwrap();
            assert!(output.0.worlds.as_slice().is_empty());
            assert!(
                job.scheduled.is_none()
                    && job.conflict.is_none()
                    && job.production.is_none()
                    && job.observation.is_none()
            );
            if let Some(expected) = total {
                assert_eq!(job.total_visits(), expected);
            } else {
                total = Some(job.total_visits());
            }
            let LivingStepPreparationV2::Ready(prepared) =
                job.finish_synchronously(&staged).unwrap()
            else {
                panic!("unexpected empty-boundary failure");
            };
            let retired = staged.commit_step(prepared).unwrap();
            assert_eq!(staged.state(), reference.state());
            assert_eq!(staged.last_receipt(), reference.last_receipt());
            assert_eq!(staged.last_decision(), reference.last_decision());
            assert_eq!(staged.last_event_facts(), reference.last_event_facts());
            drop(retired);
        }
    }

    #[test]
    fn staged_step_conflict_events_and_destroyed_entities_are_not_repeated() {
        let mut base = combat_cursor_fixture();
        let state = Arc::make_mut(&mut base.state);
        state.tick = LivingTickV2(9);
        for fleet_index in 0..state.fleets.as_slice().len() {
            let fleet = state.fleets.record_at_mut(fleet_index).unwrap();
            fleet.order = None;
            for index in 0..fleet.hulls.as_slice().len() {
                fleet.hulls.record_at_mut(index).unwrap().hit_points = 1;
            }
        }
        state.validate(&base.catalog).unwrap();
        let mut reference = base.clone();
        reference.step().unwrap();
        assert!(
            reference
                .last_receipt()
                .unwrap()
                .events
                .as_slice()
                .iter()
                .any(|event| event.kind == LivingEventKindV2::CombatRound)
        );
        for budget in [1, 64, 1024] {
            let mut staged = base.clone();
            let LivingStepPreparationV2::Ready(prepared) = prepare_staged(&staged, budget) else {
                panic!("unexpected conflict failure");
            };
            let retired = staged.commit_step(prepared).unwrap();
            assert_eq!(staged.state(), reference.state());
            assert_eq!(staged.last_receipt(), reference.last_receipt());
            assert_eq!(staged.last_decision(), reference.last_decision());
            assert_eq!(staged.last_event_facts(), reference.last_event_facts());
            drop(retired);
        }
    }

    #[test]
    fn staged_step_production_fault_is_sticky_and_does_not_consume_queue() {
        let mut base = super::super::rules::builtin_living_sandbox_v2().unwrap();
        let state = Arc::make_mut(&mut base.state);
        state.tick = LivingTickV2(u64::MAX - 2);
        state.colonies.record_at_mut(0).unwrap().hub.energy_next_due = LivingTickV2(u64::MAX - 1);
        submit(&mut base, vec![system("Queued before production fault")]);
        let before = base.capture_publication();
        let mut reference = base.clone();
        let expected = reference.step().unwrap_err();
        assert_eq!(expected, LivingDeterministicFaultV2::Arithmetic);
        for budget in [1, 64, 1024] {
            let mut authority = base.clone();
            let mut job = authority.begin_step().unwrap();
            loop {
                match job.poll(budget) {
                    Ok(LivingStepProgressV2::Pending) => {}
                    Err(LivingStepJobErrorV2::Fault(fault)) => {
                        assert_eq!(fault, expected);
                        break;
                    }
                    other => panic!("unexpected result {other:?}"),
                }
                assert!(job.last_poll_visits() <= budget);
            }
            assert!(job.production.is_some());
            assert_eq!(
                job.poll(1),
                Err(LivingStepJobErrorV2::Fault(expected.clone()))
            );
            assert_eq!(job.last_poll_visits(), 0);
            assert!(authority.boundary_capture_is_current(&before));
            let LivingStepPreparationV2::Failed(failure) =
                job.finish_synchronously(&authority).unwrap()
            else {
                panic!("expected authentic failure");
            };
            let retired = authority.commit_step_failure(failure).unwrap();
            assert!(Arc::ptr_eq(&authority.state, &before.state));
            assert!(Arc::ptr_eq(&authority.queue, &before.queue));
            drop(retired);
        }
    }

    fn diplomacy_staged_fixture(before_tick: u64) -> LivingGalaxyAuthorityV2 {
        let mut authority = super::super::rules::builtin_living_sandbox_v2().unwrap();
        let a = authority.state.civilizations.as_slice()[0].id;
        let b = authority.state.civilizations.as_slice()[1].id;
        submit(
            &mut authority,
            vec![
                LivingCreatorOperationV2::SetRelationBase {
                    from: super::super::command::LivingCommandTargetV2::Existing { id: a },
                    to: super::super::command::LivingCommandTargetV2::Existing { id: b },
                    base_disposition: 20,
                },
                LivingCreatorOperationV2::SetRelationBase {
                    from: super::super::command::LivingCommandTargetV2::Existing { id: b },
                    to: super::super::command::LivingCommandTargetV2::Existing { id: a },
                    base_disposition: 20,
                },
            ],
        );
        for _ in 0..before_tick {
            authority.step().unwrap();
        }
        authority
    }
    #[test]
    fn staged_step_diplomacy_fault_is_sticky_and_preserves_publication() {
        let mut base = diplomacy_staged_fixture(99);
        // Controlled fault injection: the next boundary is a diplomacy tick,
        // while agreement expiry cannot be represented. This is not a replay
        // fixture; its purpose is to test the atomic failure fence.
        let state = Arc::make_mut(&mut base.state);
        state.tick = LivingTickV2(u64::MAX - 16);
        state.fleets = Default::default();
        state.hull_jobs = Default::default();
        state.construction_jobs = Default::default();
        state.routes = Default::default();
        state.shipments = Default::default();
        state.wars = Default::default();
        state.agreements = Default::default();
        submit(&mut base, vec![system("Queued before diplomacy fault")]);
        let before = base.capture_publication();
        let mut reference = base.clone();
        let expected = reference.step().unwrap_err();
        assert_eq!(expected, LivingDeterministicFaultV2::Arithmetic);
        for budget in [1, 64, 1024] {
            let mut authority = base.clone();
            let mut job = authority.begin_step().unwrap();
            loop {
                let result = job.poll(budget);
                assert!(job.last_poll_visits() <= budget);
                match result {
                    Ok(LivingStepProgressV2::Pending) => {}
                    Err(LivingStepJobErrorV2::Fault(fault)) => {
                        assert_eq!(fault, expected);
                        break;
                    }
                    other => panic!("unexpected result {other:?}"),
                }
            }
            assert!(job.diplomacy.is_some());
            assert_eq!(
                job.poll(1),
                Err(LivingStepJobErrorV2::Fault(expected.clone()))
            );
            assert_eq!(job.last_poll_visits(), 0);
            assert!(authority.boundary_capture_is_current(&before));
            let LivingStepPreparationV2::Failed(failure) =
                job.finish_synchronously(&authority).unwrap()
            else {
                panic!("expected authentic failure");
            };
            let retired = authority.commit_step_failure(failure).unwrap();
            assert!(Arc::ptr_eq(&authority.state, &before.state));
            assert!(Arc::ptr_eq(&authority.queue, &before.queue));
            assert_eq!(authority.last_receipt(), base.last_receipt());
            assert_eq!(authority.last_decision(), base.last_decision());
            assert_eq!(authority.last_event_facts(), base.last_event_facts());
            assert_eq!(
                authority.last_creator_recoveries(),
                base.last_creator_recoveries()
            );
            assert_eq!(
                authority.last_corridor_restrictions(),
                base.last_corridor_restrictions()
            );
            drop(retired);
        }
    }

    #[test]
    fn staged_step_diplomacy_cadence_due_decay_and_not_due_match_reference_receipts() {
        for before_tick in [98, 99, 599] {
            let base = diplomacy_staged_fixture(before_tick);
            let mut reference = base.clone();
            reference.step().unwrap();
            let due = (before_tick + 1).is_multiple_of(DIPLOMACY_PERIOD_TICKS);
            if before_tick == 99 {
                assert_eq!(
                    reference
                        .last_receipt()
                        .unwrap()
                        .events
                        .iter()
                        .filter(|event| event.kind == LivingEventKindV2::AgreementFormed)
                        .count(),
                    2
                );
            }
            let mut total = None;
            for budgets in [&[1][..], &[64][..], &[1024][..], &[1, 3, 64, 1024][..]] {
                let mut staged = base.clone();
                let mut job = staged.begin_step().unwrap();
                let mut observed_diplomacy = false;
                let mut index = 0;
                loop {
                    let budget = budgets[index % budgets.len()];
                    index += 1;
                    let progress = job.poll(budget).unwrap();
                    assert!(job.last_poll_visits() <= budget);
                    observed_diplomacy |= job.diplomacy.is_some();
                    if progress == LivingStepProgressV2::NeedsSynchronousCompletion {
                        break;
                    }
                    assert_eq!(progress, LivingStepProgressV2::Pending);
                }
                if budgets == [1] {
                    assert_eq!(observed_diplomacy, due);
                }
                if (before_tick + 1).is_multiple_of(INTENT_PERIOD_TICKS) {
                    assert!(matches!(
                        job.completed_output.as_ref().unwrap().2,
                        LivingCompletedStepPhasesV2::DiplomacyComplete
                    ));
                } else {
                    assert!(matches!(
                        job.completed_output.as_ref().unwrap().2,
                        LivingCompletedStepPhasesV2::DispatchComplete
                    ));
                }
                if let Some(total) = total {
                    assert_eq!(job.total_visits(), total);
                } else {
                    total = Some(job.total_visits());
                }
                assert_eq!(
                    job.poll(1).unwrap(),
                    LivingStepProgressV2::NeedsSynchronousCompletion
                );
                assert_eq!(job.last_poll_visits(), 0);
                let LivingStepPreparationV2::Ready(prepared) =
                    job.finish_synchronously(&staged).unwrap()
                else {
                    panic!("unexpected failure");
                };
                let retired = staged.commit_step(prepared).unwrap();
                assert_eq!(staged.state(), reference.state());
                assert_eq!(staged.last_receipt(), reference.last_receipt());
                assert_eq!(staged.last_decision(), reference.last_decision());
                assert_eq!(staged.last_event_facts(), reference.last_event_facts());
                assert_eq!(
                    staged.last_creator_recoveries(),
                    reference.last_creator_recoveries()
                );
                assert_eq!(
                    staged.last_corridor_restrictions(),
                    reference.last_corridor_restrictions()
                );
                drop(retired);
            }
        }
    }
    #[test]
    fn staged_step_diplomacy_cancel_not_ready_and_stale_tail_preserve_authority() {
        let mut authority = diplomacy_staged_fixture(99);
        let before = authority.capture_boundary().unwrap();
        for mode in 0..3 {
            let mut job = authority.begin_step().unwrap();
            loop {
                job.poll(1).unwrap();
                if job.diplomacy.is_some() {
                    break;
                }
            }
            if mode == 0 {
                job.cancel();
                assert_eq!(job.poll(1), Err(LivingStepJobErrorV2::Cancelled));
                assert!(matches!(
                    job.finish_synchronously(&authority),
                    Err(LivingStepJobErrorV2::Cancelled)
                ));
                assert!(authority.boundary_capture_is_current(&before));
            } else if mode == 1 {
                assert!(matches!(
                    job.finish_synchronously(&authority),
                    Err(LivingStepJobErrorV2::NotReady)
                ));
                assert!(authority.boundary_capture_is_current(&before));
            } else {
                submit(&mut authority, vec![system("Tail during diplomacy")]);
                let current = authority.capture_boundary().unwrap();
                loop {
                    if job.poll(1).unwrap() == LivingStepProgressV2::NeedsSynchronousCompletion {
                        break;
                    }
                }
                assert!(matches!(
                    job.finish_synchronously(&authority),
                    Err(LivingStepJobErrorV2::Stale)
                ));
                assert!(authority.boundary_capture_is_current(&current));
                assert_eq!(authority.queue.len(), 1);
                assert!(authority.fault.is_none());
            }
        }
    }
    #[test]
    fn staged_step_empty_due_diplomacy_completion_marker_matches_reference() {
        let mut base = authority();
        for _ in 0..99 {
            base.step().unwrap();
        }
        let mut reference = base.clone();
        reference.step().unwrap();
        for budget in [1, 64, 1024] {
            let mut staged = base.clone();
            let mut job = staged.begin_step().unwrap();
            loop {
                let progress = job.poll(budget).unwrap();
                assert!(job.last_poll_visits() <= budget);
                if progress == LivingStepProgressV2::NeedsSynchronousCompletion {
                    break;
                }
            }
            assert!(matches!(
                job.completed_output.as_ref().unwrap().2,
                LivingCompletedStepPhasesV2::DiplomacyComplete
            ));
            let LivingStepPreparationV2::Ready(prepared) =
                job.finish_synchronously(&staged).unwrap()
            else {
                panic!("unexpected failure");
            };
            let retired = staged.commit_step(prepared).unwrap();
            assert_eq!(staged.state(), reference.state());
            assert_eq!(staged.last_receipt(), reference.last_receipt());
            assert_eq!(staged.last_decision(), reference.last_decision());
            assert_eq!(staged.last_event_facts(), reference.last_event_facts());
            drop(retired);
        }
    }

    #[test]
    fn staged_step_lifecycle_fault_is_sticky_and_preserves_last_boundary() {
        let mut base = scheduled_staged_fixture();
        let state = Arc::make_mut(&mut base.state);
        state.tick = LivingTickV2(u64::MAX - 2);
        state.wars.record_at_mut(0).unwrap().end_tick = LivingTickV2(u64::MAX - 1);
        let before = base.state.digest().unwrap();
        let mut reference = base.clone();
        let expected = reference.step().unwrap_err();
        assert_eq!(expected, LivingDeterministicFaultV2::Arithmetic);
        for budget in [1, 64, 1024] {
            let mut authority = base.clone();
            let mut job = authority.begin_step().unwrap();
            loop {
                let result = job.poll(budget);
                assert!(job.last_poll_visits() <= budget);
                match result {
                    Ok(LivingStepProgressV2::Pending) => {}
                    Err(LivingStepJobErrorV2::Fault(fault)) => {
                        assert_eq!(fault, expected);
                        break;
                    }
                    other => panic!("unexpected result {other:?}"),
                }
            }
            assert_eq!(
                job.poll(1),
                Err(LivingStepJobErrorV2::Fault(expected.clone()))
            );
            assert_eq!(job.last_poll_visits(), 0);
            let LivingStepPreparationV2::Failed(failure) =
                job.finish_synchronously(&authority).unwrap()
            else {
                panic!("expected failure");
            };
            let retired = authority.commit_step_failure(failure).unwrap();
            assert_eq!(authority.state.digest().unwrap(), before);
            assert_eq!(authority.fault, Some(expected.clone()));
            drop(retired);
        }
    }

    /// The original exhaustive LowerRank disclosure exceeded 39 MiB here.
    /// Actual-attempt disclosure must preserve the legal boundary instead.
    #[test]
    fn capacity_sized_candidate_search_discloses_only_actual_attempts() {
        use super::super::rules::builtin_living_sandbox_v2;
        let source = builtin_living_sandbox_v2().unwrap();
        let template = source.state();
        let world_template = &template.worlds.as_slice()[0];
        let colony_template = &template.colonies.as_slice()[0];
        let extractor_template = template
            .facilities
            .as_slice()
            .iter()
            .find(|row| row.definition.as_str() == "extractor")
            .unwrap();
        let mut civilization = template.civilizations.as_slice()[0].clone();
        civilization.policy =
            super::super::model::LivingPolicyV2(LivingSlugV2::new("neutral").unwrap());
        let actor = civilization.id;
        let identity = |group: u8, index: u16| {
            let mut bytes = [group; 16];
            bytes[14..].copy_from_slice(&index.to_be_bytes());
            super::super::ids::LivingEntityIdV2(bytes)
        };
        let mut worlds = Vec::new();
        let mut colonies = Vec::new();
        let mut facilities = Vec::new();
        let mut deposits = Vec::new();
        let mut routes = Vec::new();
        for index in 0_u16..512 {
            let id = identity(200, index);
            let mut world = world_template.clone();
            world.id = id;
            world.owner = Some(actor);
            world.inventory = if index < 256 {
                super::super::model::LivingInventoryV2 {
                    energy: 30,
                    ore: 30,
                    alloy: 0,
                }
            } else {
                super::super::model::LivingInventoryV2::default()
            };
            worlds.push(world);
            let mut colony = colony_template.clone();
            colony.world = id;
            colony.owner = actor;
            colonies.push(colony);
            if index >= 256 {
                for slot in 0_u16..2 {
                    let mut facility = extractor_template.clone();
                    facility.id = identity(201, (index - 256) * 2 + slot);
                    facility.world = id;
                    facilities.push(facility);
                }
                deposits.push(super::super::model::LivingDepositV2 {
                    id: identity(202, index),
                    world: id,
                    resource: super::super::model::LivingResourceV2::Ore,
                    remaining_units: 1_000,
                });
                routes.push(super::super::model::LivingFreightRouteV2 {
                    id: identity(203, index),
                    kind: super::super::model::LivingRouteKindV2::Internal,
                    source_world: id,
                    destination_world: identity(200, 0),
                    resource: super::super::model::LivingResourceV2::Ore,
                    batch_size: 10,
                    cadence_ticks: 10,
                    source_reserve: 20,
                    source_owner: actor,
                    receiver_owner: actor,
                    next_due: LivingTickV2(10),
                    suspended: false,
                });
            }
        }
        let state = LivingGalaxyStateV2 {
            systems: super::super::wire::LivingSortedVecV2::new(vec![
                template.systems.as_slice()[0].clone(),
            ])
            .unwrap(),
            civilizations: super::super::wire::LivingSortedVecV2::new(vec![civilization]).unwrap(),
            worlds: super::super::wire::LivingSortedVecV2::new(worlds).unwrap(),
            colonies: super::super::wire::LivingSortedVecV2::new(colonies).unwrap(),
            facilities: super::super::wire::LivingSortedVecV2::new(facilities).unwrap(),
            deposits: super::super::wire::LivingSortedVecV2::new(deposits).unwrap(),
            routes: super::super::wire::LivingSortedVecV2::new(routes).unwrap(),
            ..LivingGalaxyStateV2::default()
        };
        let catalog = source.catalog().clone();
        let genesis = validate_living_genesis_manifest_v2(
            LivingGenesisManifestV2 {
                kind: LIVING_GENESIS_KIND_V2.to_owned(),
                format_version: LIVING_GENESIS_FORMAT_VERSION_V2,
                rules_version: LIVING_RULES_VERSION,
                state,
            },
            &catalog,
        )
        .unwrap();
        let mut authority = LivingGalaxyAuthorityV2::from_genesis(
            catalog,
            [0x51; 32],
            LivingGenesisGeneratorV2 {
                generator_id: LivingSlugV2::new("capacity_audit").unwrap(),
                generator_version: 1,
            },
            genesis,
        )
        .unwrap();
        for _ in 0..49 {
            authority.step().unwrap();
        }
        let receipt = authority.step().unwrap();
        assert_eq!(authority.state().tick, LivingTickV2(50));
        let decision = authority.last_decision().unwrap();
        decision.verify_receipt(&receipt).unwrap();
        assert_eq!(decision.intents.len(), 1);
        assert_eq!(decision.intents[0].ordinal, 0);
        assert_eq!(
            decision.intents[0].outcome,
            super::super::decision::LivingDecisionOutcomeV2::Accepted
        );
    }

    fn submit(authority: &mut LivingGalaxyAuthorityV2, operations: Vec<LivingCreatorOperationV2>) {
        let cursor = authority.published_cursor();
        authority
            .submit(LivingCommandEnvelopeV2 {
                expected_committed_revision: cursor.committed_revision,
                expected_tick: cursor.tick,
                expected_pending_sequence: cursor.pending_sequence,
                mode: LivingCommandModeV2::Running,
                command: LivingCommandV2::CreatorBatch { operations },
            })
            .unwrap();
    }

    fn system(text: &str) -> LivingCreatorOperationV2 {
        LivingCreatorOperationV2::CreateSystem {
            local: 0,
            name: LivingNameV2::new(text).unwrap(),
        }
    }

    /// Make the committed state disagree with what the queued envelopes were
    /// validated against. No public path can do this; it stands in for any
    /// defect that makes a supposedly accepted envelope stop reproducing.
    fn corrupt_committed_state(authority: &mut LivingGalaxyAuthorityV2) {
        Arc::make_mut(&mut authority.state).systems = LivingSortedVecV2::default();
    }

    #[test]
    fn history_getters_retain_present_head_after_same_tick_null_cursor_reload() {
        let mut authority = authority();
        for _ in 0..3 {
            authority.step().unwrap();
        }
        let cursor = authority.published_cursor();
        authority
            .apply_paused(LivingCommandEnvelopeV2 {
                expected_committed_revision: cursor.committed_revision,
                expected_tick: cursor.tick,
                expected_pending_sequence: cursor.pending_sequence,
                mode: LivingCommandModeV2::Paused,
                command: LivingCommandV2::CreatorBatch {
                    operations: vec![system("Retained future")],
                },
            })
            .unwrap();
        let present = authority.bookmark().unwrap();
        let branch = authority.branch();
        assert_eq!(authority.present_view(), None);
        authority
            .restore_view(LivingHistoryViewV2 {
                branch,
                revision: None,
                tick: LivingTickV2(3),
            })
            .unwrap();
        assert!(authority.is_viewing_history());
        assert_eq!(authority.present_view(), Some(present.view));
        assert!(authority.state().systems.as_slice().is_empty());
        let retained = authority.retained_history_branch(branch).unwrap();
        assert_eq!(retained.committed_revision, present.view.revision);
        assert_eq!(retained.state().systems.as_slice().len(), 1);
        let bytes = super::super::archive::encode_living_archive_v2(&authority).unwrap();
        let decoded =
            super::super::archive::decode_living_archive_v2(&bytes, authority.catalog()).unwrap();
        let restored = decoded.authority();
        assert!(restored.is_viewing_history());
        assert_eq!(restored.present_view(), Some(present.view));
        assert!(restored.state().systems.as_slice().is_empty());
        assert_eq!(
            restored
                .retained_history_branch(branch)
                .unwrap()
                .state()
                .systems
                .as_slice()
                .len(),
            1
        );
        assert_eq!(
            super::super::archive::encode_living_archive_v2(restored).unwrap(),
            bytes
        );
    }

    #[test]
    fn production_event_facts_bind_actual_entities_and_authoritative_blocked_reasons() {
        let original = crate::living::builtin_living_sandbox_v2().unwrap();
        let catalog = original.catalog().clone();
        let mut state = original.state().clone();
        let world = state.facilities.as_slice()[0].world;
        let owner = state
            .worlds
            .as_slice()
            .iter()
            .find(|row| row.id == world)
            .unwrap()
            .owner
            .unwrap();
        let facility_id = state.facilities.as_slice()[0].id;
        state.facilities.find_mut(&facility_id).unwrap().disabled = true;
        let industry = catalog.industry("solar_array").unwrap();
        state.construction_jobs =
            LivingSortedVecV2::new(vec![crate::living::model::LivingConstructionJobV2 {
                id: LivingEntityIdV2([201; 16]),
                world,
                owner,
                definition: industry.id.clone(),
                accepted_tick: LivingTickV2(0),
                completion_tick: LivingTickV2(u64::from(industry.build_ticks)),
                paid_alloy: industry.alloy_cost,
            }])
            .unwrap();
        let hull = catalog.hull("scout").unwrap();
        state.hull_jobs = LivingSortedVecV2::new(vec![crate::living::model::LivingHullJobV2 {
            id: LivingEntityIdV2([202; 16]),
            world,
            owner,
            definition: hull.id.clone(),
            accepted_tick: LivingTickV2(0),
            completion_tick: LivingTickV2(u64::from(hull.build_ticks)),
            paid_alloy: hull.alloy_cost,
            target_fleet: None,
        }])
        .unwrap();
        let genesis = validate_living_genesis_manifest_v2(
            LivingGenesisManifestV2 {
                kind: LIVING_GENESIS_KIND_V2.to_owned(),
                format_version: LIVING_GENESIS_FORMAT_VERSION_V2,
                rules_version: LIVING_RULES_VERSION,
                state,
            },
            &catalog,
        )
        .unwrap();
        let mut authority = LivingGalaxyAuthorityV2::from_genesis(
            catalog,
            original.genesis_seed,
            original.generator.clone(),
            genesis,
        )
        .unwrap();
        let mut construction_seen = false;
        let mut hull_seen = false;
        let mut blocked_seen = false;
        for _ in 0..300 {
            let receipt = authority.step().unwrap();
            let sidecar = authority.last_event_facts().unwrap();
            sidecar.verify_receipt(&receipt).unwrap();
            for entry in &sidecar.events {
                let ordinary = &receipt.events[usize::from(entry.ordinal)];
                match ordinary.kind {
                    LivingEventKindV2::ConstructionCompleted => {
                        let entity = entry.facts.entity.unwrap();
                        construction_seen = true;
                        let facility = authority
                            .state()
                            .facilities
                            .as_slice()
                            .iter()
                            .find(|row| row.id == entity)
                            .unwrap();
                        let owner = authority
                            .state()
                            .worlds
                            .as_slice()
                            .iter()
                            .find(|row| row.id == facility.world)
                            .unwrap()
                            .owner;
                        assert_eq!(entry.facts.actor, owner);
                    }
                    LivingEventKindV2::HullCompleted => {
                        let entity = entry.facts.entity.unwrap();
                        hull_seen = true;
                        let fleet = authority
                            .state()
                            .fleets
                            .as_slice()
                            .iter()
                            .find(|fleet| {
                                fleet.hulls.as_slice().iter().any(|hull| hull.id == entity)
                            })
                            .unwrap();
                        assert_eq!(entry.facts.actor, Some(fleet.owner));
                    }
                    LivingEventKindV2::FacilityProduced => {
                        let entity = entry.facts.entity.unwrap();
                        let facility = authority
                            .state()
                            .facilities
                            .as_slice()
                            .iter()
                            .find(|row| row.id == entity)
                            .unwrap();
                        use crate::living::event_facts::{
                            LivingDepositInputFactV2, LivingEventResourceFactV2,
                            LivingEventResourceRoleV2,
                        };
                        let definition = authority
                            .catalog()
                            .industry(facility.definition.as_str())
                            .unwrap();
                        let crate::living::catalog::LivingIndustryEffectV2::Recipe {
                            inputs,
                            outputs,
                            deposit_units,
                            ..
                        } = &definition.effect
                        else {
                            panic!("a successful producer must have a recipe");
                        };
                        let resource = |slug: &str| match slug {
                            "alloy" => crate::living::LivingResourceV2::Alloy,
                            "energy" => crate::living::LivingResourceV2::Energy,
                            "ore" => crate::living::LivingResourceV2::Ore,
                            _ => panic!("unsupported resource"),
                        };
                        let mut expected: Vec<_> = inputs
                            .as_slice()
                            .iter()
                            .map(|amount| LivingEventResourceFactV2 {
                                role: LivingEventResourceRoleV2::Input,
                                resource: resource(amount.resource.as_str()),
                                quantity: amount.quantity,
                            })
                            .chain(outputs.as_slice().iter().map(|amount| {
                                LivingEventResourceFactV2 {
                                    role: LivingEventResourceRoleV2::Output,
                                    resource: resource(amount.resource.as_str()),
                                    quantity: amount.quantity,
                                }
                            }))
                            .collect();
                        expected.sort_by_key(|fact| (fact.role, fact.resource));
                        assert_eq!(entry.facts.resources, expected);
                        assert_eq!(
                            entry.facts.deposit_input,
                            (*deposit_units > 0).then_some(LivingDepositInputFactV2 {
                                world: facility.world,
                                units: *deposit_units
                            })
                        );
                        if let Some(draw) = &entry.facts.deposit_input {
                            assert_eq!(draw.world, facility.world);
                            assert!(draw.units > 0);
                        }
                    }
                    LivingEventKindV2::FacilityBlocked => {
                        let entity = entry.facts.entity.unwrap();
                        blocked_seen = true;
                        let facility = authority
                            .state()
                            .facilities
                            .as_slice()
                            .iter()
                            .find(|row| row.id == entity)
                            .unwrap();
                        assert_eq!(
                            entry.facts.blocked_reasons.as_slice(),
                            facility.blocked_reasons.as_slice()
                        );
                    }
                    _ => {}
                }
            }
        }
        assert!(construction_seen && hull_seen && blocked_seen);
    }

    #[test]
    fn boundary_capture_is_immutable_and_tracks_queue_publication_freshness() {
        let mut authority = authority();
        let empty = authority.capture_boundary().unwrap();
        assert!(authority.boundary_capture_is_current(&empty));
        assert_eq!(empty.pending_count(), 0);
        submit(&mut authority, vec![system("First")]);
        assert!(!authority.boundary_capture_is_current(&empty));
        assert!(Arc::ptr_eq(&empty.state, &authority.state));
        let first = authority.capture_boundary().unwrap();
        let cursor = authority.published_cursor();
        let rejected = authority.submit(LivingCommandEnvelopeV2 {
            expected_committed_revision: cursor.committed_revision,
            expected_tick: LivingTickV2(cursor.tick.0 + 1),
            expected_pending_sequence: cursor.pending_sequence,
            mode: LivingCommandModeV2::Running,
            command: LivingCommandV2::CreatorBatch {
                operations: vec![system("Rejected")],
            },
        });
        assert!(rejected.is_err());
        assert!(authority.boundary_capture_is_current(&first));
        submit(&mut authority, vec![system("Second")]);
        assert!(!authority.boundary_capture_is_current(&first));
        assert_eq!(first.pending_count(), 1);
        assert_eq!(authority.queue.len(), 2);
        assert!(Arc::ptr_eq(
            &first.queue[0].revision,
            &authority.queue[0].revision
        ));
        assert!(Arc::ptr_eq(
            &first.queue[0].accepted,
            &authority.queue[0].accepted
        ));
        let pending = authority.capture_boundary().unwrap();
        authority.step().unwrap();
        assert!(!authority.boundary_capture_is_current(&pending));
        assert_eq!(pending.pending_count(), 2);
        assert_eq!(pending.state_snapshot().systems.as_slice().len(), 0);
        assert_eq!(authority.state().systems.as_slice().len(), 2);
        assert_eq!(authority.queue.len(), 0);
        assert!(authority.boundary_capture_is_current(&authority.capture_boundary().unwrap()));
    }

    #[test]
    fn document_revision_capacity_counts_retained_and_pending_ids() {
        let mut authority = authority();
        submit(&mut authority, vec![system("Seed")]);
        authority.step().unwrap();
        let template = authority.revisions[0].clone();
        *authority.revisions = Arc::new(
            (0_u64..9_999)
                .map(|ordinal| {
                    let mut revision = template.clone();
                    let mut id = [0_u8; 32];
                    id[24..].copy_from_slice(&ordinal.to_be_bytes());
                    revision.id = LivingRevisionIdV2(id);
                    revision
                })
                .collect(),
        );
        submit(&mut authority, vec![system("Last")]);
        let before = authority.state.clone();
        let cursor = authority.published_cursor();
        let high_water = authority.accepted_sequence_high_water;
        let result = authority.submit(LivingCommandEnvelopeV2 {
            expected_committed_revision: cursor.committed_revision,
            expected_tick: cursor.tick,
            expected_pending_sequence: cursor.pending_sequence,
            mode: LivingCommandModeV2::Running,
            command: LivingCommandV2::CreatorBatch {
                operations: vec![system("Rejected")],
            },
        });
        assert!(matches!(
            result,
            Err(LivingCommandRejectionV2::HistoryFull {
                collection: "revisions",
                limit: 10_000
            })
        ));
        assert_eq!(authority.state, before);
        assert_eq!(authority.published_cursor(), cursor);
        assert_eq!(authority.accepted_sequence_high_water, high_water);
    }

    #[test]
    fn a_fault_commits_nothing_retains_the_queue_and_pauses() {
        let mut authority = authority();
        submit(&mut authority, vec![system("Vale")]);
        authority.step().unwrap();
        let vale = authority.state.systems.as_slice()[0].id;

        submit(&mut authority, vec![system("Outer")]);
        submit(
            &mut authority,
            vec![LivingCreatorOperationV2::CreateStar {
                local: 0,
                system: LivingCommandTargetV2::Existing { id: vale },
                archetype: LivingSlugV2::new("red_dwarf").unwrap(),
                name: LivingNameV2::new("Ember").unwrap(),
            }],
        );
        corrupt_committed_state(&mut authority);
        let state_before = authority.state.clone();
        let cursor_before = authority.published_cursor();
        let revisions_before = authority.revisions.clone();

        let fault = authority.step().unwrap_err();
        assert!(
            matches!(
                fault,
                LivingDeterministicFaultV2::Validation(
                    LivingValidationErrorV2::DanglingReference { .. }
                )
            ),
            "{fault:?}"
        );
        assert_eq!(authority.state, state_before, "no partial state");
        assert_eq!(
            authority.revisions, revisions_before,
            "no partial revisions"
        );
        assert_eq!(
            authority.published_cursor(),
            cursor_before,
            "the queue is retained for diagnosis"
        );
        assert_eq!(authority.fault(), Some(&fault));

        // Paused: stepping repeats the fault and submission is refused.
        assert_eq!(authority.step().unwrap_err(), fault);
        let cursor = authority.published_cursor();
        let refused = authority.submit(LivingCommandEnvelopeV2 {
            expected_committed_revision: cursor.committed_revision,
            expected_tick: cursor.tick,
            expected_pending_sequence: cursor.pending_sequence,
            mode: LivingCommandModeV2::Running,
            command: LivingCommandV2::CreatorBatch {
                operations: Vec::new(),
            },
        });
        assert_eq!(refused, Err(LivingCommandRejectionV2::Faulted));

        // Discarding the queue resumes without reissuing its sequences.
        assert_eq!(authority.accepted_sequence_high_water(), 3);
        authority.discard_faulted_queue();
        assert!(authority.fault().is_none());
        assert_eq!(authority.published_cursor().pending_sequence, 0);
        submit(&mut authority, Vec::new());
        assert_eq!(authority.published_cursor().pending_sequence, 4);
        authority.step().unwrap();
        assert_eq!(authority.state.accepted_sequence, 4);
    }

    #[test]
    fn an_edit_whose_target_vanished_faults_as_a_replay_failure() {
        let mut authority = authority();
        let cursor = authority.published_cursor();
        authority
            .apply_paused(LivingCommandEnvelopeV2 {
                expected_committed_revision: cursor.committed_revision,
                expected_tick: cursor.tick,
                expected_pending_sequence: 0,
                mode: LivingCommandModeV2::Paused,
                command: LivingCommandV2::CreatorBatch {
                    operations: vec![LivingCreatorOperationV2::CreateCivilization {
                        local: 0,
                        name: LivingNameV2::new("Kin").unwrap(),
                        policy: crate::living::model::LivingPolicyV2(
                            LivingSlugV2::new("neutral").unwrap(),
                        ),
                        status: crate::living::model::LivingCivilizationStatusV2::Dormant,
                    }],
                },
            })
            .unwrap();
        let kin = authority.state.civilizations.as_slice()[0].id;
        submit(
            &mut authority,
            vec![LivingCreatorOperationV2::SetCivilizationPolicy {
                civilization: LivingCommandTargetV2::Existing { id: kin },
                policy: crate::living::model::LivingPolicyV2(LivingSlugV2::new("trader").unwrap()),
            }],
        );
        Arc::make_mut(&mut authority.state).civilizations = LivingSortedVecV2::default();
        let fault = authority.step().unwrap_err();
        assert!(
            matches!(
                &fault,
                LivingDeterministicFaultV2::Replay {
                    accepted_sequence: 2,
                    reason,
                } if matches!(**reason, LivingCommandRejectionV2::MissingTarget { .. })
            ),
            "{fault:?}"
        );
        assert_eq!(authority.published_cursor().pending_sequence, 2);
    }

    #[test]
    fn phases_run_in_registry_order_and_the_emit_path_assigns_ordinals() {
        let catalog = living_core_pack_v2().unwrap();
        let mut state = LivingGalaxyStateV2::default();
        let mut events = LivingPendingEventsV2::new();
        let mut context = LivingStepContextV2 {
            catalog: &catalog,
            genesis_seed: [0; 32],
            branch: LivingBranchIdV2([0; 16]),
            tick: LivingTickV2(1),
            state: &mut state,
            events: &mut events,
            arrivals: Vec::new(),
        };
        for kind in [
            LivingEventKindV2::HazardStarted,
            LivingEventKindV2::HazardEnded,
        ] {
            context
                .emit(
                    LivingEventProvenanceV2::Autonomous {
                        phase: LivingPhaseV2::ExpireLifecycles,
                    },
                    kind,
                )
                .unwrap();
        }
        let ordinals: Vec<u16> = events.payloads().iter().map(|row| row.ordinal).collect();
        assert_eq!(ordinals, vec![0, 1]);

        let order: Vec<u16> = LIVING_PHASE_REGISTRY_V2.iter().map(|row| row.1).collect();
        assert_eq!(order, (1..=10).collect::<Vec<_>>());
    }
    pub(super) fn combat_cursor_fixture() -> LivingGalaxyAuthorityV2 {
        use crate::living::model::*;
        let mut authority = super::super::rules::builtin_living_sandbox_v2().unwrap();
        let state = Arc::make_mut(&mut authority.state);
        let world = state.worlds.as_slice()[0].id;
        let a = state.civilizations.as_slice()[0].id;
        let b = state.civilizations.as_slice()[1].id;
        state.wars = LivingSortedVecV2::new(vec![LivingWarV2 {
            participant_a: a.min(b),
            participant_b: a.max(b),
            declarer: a,
            start_tick: LivingTickV2(0),
            end_tick: LivingTickV2(1000),
        }])
        .unwrap();
        let fleet_ids: Vec<_> = state
            .fleets
            .as_slice()
            .iter()
            .map(|fleet| fleet.id)
            .collect();
        for (index, id) in fleet_ids.into_iter().enumerate() {
            let fleet = state.fleets.find_mut(&id).unwrap();
            fleet.owner = if index % 2 == 0 { a } else { b };
            fleet.location = LivingFleetLocationV2::Docked { world };
            let hull_ids: Vec<_> = fleet.hulls.as_slice().iter().map(|hull| hull.id).collect();
            for (hull_index, id) in hull_ids.into_iter().enumerate() {
                let hull = fleet.hulls.find_mut(&id).unwrap();
                hull.definition = LivingSlugV2::new(if hull_index % 2 == 0 {
                    "escort"
                } else {
                    "scout"
                })
                .unwrap();
                hull.hit_points = if index % 2 == 0 { 1 } else { 500 };
                hull.next_repair = None;
            }
        }
        state.worlds.find_mut(&world).unwrap().owner = Some(a);
        state.facilities = LivingSortedVecV2::new(vec![LivingFacilityV2 {
            id: LivingEntityIdV2([250; 16]),
            world,
            definition: LivingSlugV2::new("defense_battery").unwrap(),
            status: LivingFacilityStatusV2::Operational,
            paid_alloy: 40,
            hit_points: 40,
            next_due: None,
            next_repair: None,
            blocked_reasons: LivingSortedVecV2::default(),
            disabled: false,
        }])
        .unwrap();
        authority
    }
    #[test]
    fn combat_cursor_matches_reference_simultaneous_damage_and_destruction() {
        use crate::living::step_job::{LivingCombatJobV2, LivingLifecycleProgressV2};
        let authority = combat_cursor_fixture();
        let mut reference = authority.state.as_ref().clone();
        let mut reference_events = LivingPendingEventsV2::default();
        rules::reference_combat_for_cursor_test(&mut LivingStepContextV2 {
            catalog: &authority.catalog,
            genesis_seed: authority.genesis_seed,
            branch: authority.branch,
            tick: LivingTickV2(10),
            state: &mut reference,
            events: &mut reference_events,
            arrivals: Vec::new(),
        })
        .unwrap();
        let mut total = None;
        for budgets in [&[1][..], &[64][..], &[1024][..], &[1, 7, 64, 1024][..]] {
            let mut job = LivingCombatJobV2::begin_owned(
                Arc::clone(&authority.catalog),
                authority.state.as_ref().clone(),
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
            if let Some(total) = total {
                assert_eq!(job.total_visits(), total);
            } else {
                total = Some(job.total_visits());
            }
            let (state, events) = job.finish().unwrap();
            assert_eq!(state, reference);
            assert_eq!(events.payloads(), reference_events.payloads());
        }
        assert!(
            reference_events
                .payloads()
                .iter()
                .any(|event| event.kind == LivingEventKindV2::CombatRound)
        );
        assert!(
            reference_events
                .payloads()
                .iter()
                .any(|event| event.kind == LivingEventKindV2::HullDestroyed)
        );
    }
    #[test]
    fn combat_cursor_battery_disable_death_and_transit_axes_match_reference() {
        use crate::living::model::LivingFleetLocationV2;
        use crate::living::step_job::{LivingCombatJobV2, LivingLifecycleProgressV2};
        for axis in 0..4 {
            let mut authority = combat_cursor_fixture();
            let state = Arc::make_mut(&mut authority.state);
            match axis {
                0 => state.facilities.record_at_mut(0).unwrap().disabled = true,
                1 => state.facilities.record_at_mut(0).unwrap().hit_points = 0,
                2 => {
                    state.facilities.record_at_mut(0).unwrap().hit_points = 1;
                    let owner = state.civilizations.as_slice()[1].id;
                    let ids: Vec<_> = state
                        .fleets
                        .as_slice()
                        .iter()
                        .map(|fleet| fleet.id)
                        .collect();
                    for id in ids {
                        state.fleets.find_mut(&id).unwrap().owner = owner;
                    }
                }
                _ => {
                    let origin_world = state.worlds.as_slice()[0].id;
                    let destination_world = state.worlds.as_slice()[1].id;
                    state.fleets.record_at_mut(0).unwrap().location =
                        LivingFleetLocationV2::Travelling {
                            origin_world,
                            destination_world,
                            departure_tick: LivingTickV2(0),
                            arrival_tick: LivingTickV2(20),
                        };
                }
            }
            let mut reference = authority.state.as_ref().clone();
            let mut reference_events = LivingPendingEventsV2::default();
            rules::reference_combat_for_cursor_test(&mut LivingStepContextV2 {
                catalog: &authority.catalog,
                genesis_seed: authority.genesis_seed,
                branch: authority.branch,
                tick: LivingTickV2(10),
                state: &mut reference,
                events: &mut reference_events,
                arrivals: Vec::new(),
            })
            .unwrap();
            let mut job = LivingCombatJobV2::begin(
                &authority.catalog,
                authority.state.as_ref().clone(),
                LivingPendingEventsV2::default(),
                LivingTickV2(10),
            );
            loop {
                let progress = job.poll(1).unwrap();
                assert!(job.last_poll_visits() <= 1);
                if progress == LivingLifecycleProgressV2::Ready {
                    break;
                }
            }
            let (state, events) = job.finish().unwrap();
            assert_eq!(state, reference, "axis {axis}");
            assert_eq!(
                events.payloads(),
                reference_events.payloads(),
                "axis {axis}"
            );
        }
    }
    #[test]
    fn combat_cursor_cancel_budget_and_arithmetic_fault_do_not_publish() {
        use crate::living::step_job::{LivingCombatJobV2, LivingLifecycleJobErrorV2};
        let authority = combat_cursor_fixture();
        let mut job = LivingCombatJobV2::begin(
            &authority.catalog,
            authority.state.as_ref().clone(),
            LivingPendingEventsV2::default(),
            LivingTickV2(10),
        );
        assert_eq!(job.poll(0), Err(LivingLifecycleJobErrorV2::Budget));
        assert_eq!(job.last_poll_visits(), 0);
        assert_eq!(job.poll(1025), Err(LivingLifecycleJobErrorV2::Budget));
        job.poll(1).unwrap();
        job.cancel();
        assert_eq!(job.poll(1), Err(LivingLifecycleJobErrorV2::Cancelled));
        assert!(matches!(
            job.finish(),
            Err(LivingLifecycleJobErrorV2::Cancelled)
        ));
        let tick = LivingTickV2(u64::MAX - 5); // Combat cadence, repair deadline overflows.
        let mut state = authority.state.as_ref().clone();
        state.wars.record_at_mut(0).unwrap().end_tick = LivingTickV2(u64::MAX);
        let mut job = LivingCombatJobV2::begin(
            &authority.catalog,
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
                other => panic!("expected repair arithmetic failure: {other:?}"),
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
