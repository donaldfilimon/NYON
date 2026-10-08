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
use super::genesis::{
    LIVING_HUB_ENERGY_PERIOD_TICKS_V2, LIVING_HUB_FALLBACK_PERIOD_TICKS_V2,
    LIVING_HUB_ORE_PERIOD_TICKS_V2, LivingGenesisGeneratorV2, ValidatedLivingGenesisV2,
};
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
    pub(crate) repair_exclusions: std::collections::BTreeSet<LivingEntityIdV2>,
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
    /// Local preparation capacity failed; no deterministic token is produced.
    #[error("Living step preparation capacity exceeded")]
    PreparationCapacity,
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
    preparation_capacity: bool,
    production_fault: Option<super::step_job::ProductionFailure>,
    production_handoff: bool,
    fault: Option<LivingDeterministicFaultV2>,
    cancelled: bool,
    delivered: bool,
    retirement: super::step_job::PhaseRetirementLane<'static>,
    last_poll_visits: usize,
    total_visits: u64,
}
impl LivingStepJobV2 {
    /// Exact publication fence for resident cancellation/restart before polling.
    /// This is identity freshness, not success or retry eligibility after failure.
    pub fn is_current(&self, authority: &LivingGalaxyAuthorityV2) -> bool {
        !self.cancelled
            && !self.delivered
            && self.capture.is_present()
            && authority.boundary_capture_is_current(&self.capture)
    }
    fn needs_synchronous_completion(&self) -> bool {
        !self.preparation_capacity
            && self.retirement.is_empty()
            && self.completed_output.as_ref().is_some_and(|output| {
                matches!(output.2, LivingCompletedStepPhasesV2::DispatchComplete)
                    || (matches!(output.2, LivingCompletedStepPhasesV2::DiplomacyComplete)
                        && (self.capture.state.tick.0 + 1).is_multiple_of(INTENT_PERIOD_TICKS))
            })
    }
    fn install_production_from_conflict(&mut self) -> Result<(), LivingStepJobErrorV2> {
        use super::step_job::LivingLifecycleJobErrorV2;
        let conflict = self
            .conflict
            .as_mut()
            .ok_or(LivingStepJobErrorV2::NotReady)?;
        let (state, events, exclusions) = match conflict.take_ready_with_exclusions() {
            Ok(output) => output,
            Err(LivingLifecycleJobErrorV2::PreparationCapacity) => {
                self.preparation_capacity = true;
                return Err(LivingStepJobErrorV2::PreparationCapacity);
            }
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
        self.production = Some(Box::new(
            super::step_job::LivingProductionJobV2::begin_owned(
                Arc::clone(&self.capture.catalog),
                state,
                events,
                LivingTickV2(self.capture.state.tick.0 + 1),
                self.capture.genesis_seed,
                self.capture.branch,
            )
            .with_repair_exclusions(exclusions),
        ));
        self.retirement.start(self.conflict.take());
        Ok(())
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
            return Err(if self.preparation_capacity {
                LivingStepJobErrorV2::PreparationCapacity
            } else if let Some(fault) = self.production_fault {
                LivingStepJobErrorV2::Fault(fault.public())
            } else if let Some(fault) = &self.fault {
                LivingStepJobErrorV2::Fault(fault.clone())
            } else if let Some(super::step_job::LivingLifecycleJobErrorV2::Fault(fault)) = self
                .creator
                .as_ref()
                .and_then(|creator| creator.structural_failure())
            {
                LivingStepJobErrorV2::Fault(fault)
            } else {
                LivingStepJobErrorV2::Cancelled
            });
        }
        if budget == 0 || budget > 1024 {
            return Err(LivingStepJobErrorV2::Budget);
        }
        if self.delivered || !self.capture.is_present() {
            return Err(LivingStepJobErrorV2::NotReady);
        }
        if self.preparation_capacity {
            return Err(LivingStepJobErrorV2::PreparationCapacity);
        }
        if let Some(fault) = self.production_fault {
            return Err(LivingStepJobErrorV2::Fault(fault.public()));
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
            if self.production_handoff {
                self.last_poll_visits += 1;
                self.total_visits += 1;
                let state = self.candidate.take().expect("retained production output");
                self.observation = Some(Box::new(super::step_job::LivingObservationJobV2::begin(
                    state,
                    LivingTickV2(self.capture.state.tick.0 + 1),
                )));
                self.production_handoff = false;
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
                    Err(LivingLifecycleJobErrorV2::PreparationCapacity) => {
                        self.preparation_capacity = true;
                        return Err(LivingStepJobErrorV2::PreparationCapacity);
                    }
                    Err(LivingLifecycleJobErrorV2::Fault(fault)) => {
                        // The failed visit was debited above. Structural causes
                        // stay in the exact original child, including its owners.
                        if !matches!(&creator.insertion, Some(QueuedCreatorInsertion::FleetStructural(work)) if work.fault.is_some())
                            && !matches!(&creator.insertion, Some(QueuedCreatorInsertion::Removal(work)) if work.fault.is_some())
                        {
                            self.fault = Some(fault.clone());
                        }
                        return Err(LivingStepJobErrorV2::Fault(fault));
                    }
                    Err(_) => return Err(LivingStepJobErrorV2::NotReady),
                }
            };
            self.last_poll_visits += visits;
            self.total_visits += visits as u64;
            let progress = match result {
                Ok(progress) => progress,
                Err(LivingLifecycleJobErrorV2::PreparationCapacity) => {
                    self.preparation_capacity = true;
                    return Err(LivingStepJobErrorV2::PreparationCapacity);
                }
                Err(LivingLifecycleJobErrorV2::Fault(fault)) => {
                    if self.production.is_some() {
                        let cause = super::step_job::ProductionFailure::capture(&fault);
                        self.production_fault = Some(cause);
                        return Err(LivingStepJobErrorV2::Fault(cause.public()));
                    }
                    self.fault = Some(fault.clone());
                    return Err(LivingStepJobErrorV2::Fault(fault));
                }
                Err(_) => {
                    if self.production.is_some() {
                        self.production_fault = Some(super::step_job::ProductionFailure::Invariant);
                        return Err(LivingStepJobErrorV2::Fault(
                            LivingDeterministicFaultV2::Invariant,
                        ));
                    }
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
                let (state, events, arrivals) = match scheduled.take_ready_result() {
                    Ok(output) => output,
                    Err(LivingLifecycleJobErrorV2::PreparationCapacity) => {
                        self.preparation_capacity = true;
                        return Err(LivingStepJobErrorV2::PreparationCapacity);
                    }
                    Err(LivingLifecycleJobErrorV2::Fault(fault)) => {
                        return Err(LivingStepJobErrorV2::Fault(fault));
                    }
                    Err(_) => {
                        return Err(LivingStepJobErrorV2::Fault(
                            LivingDeterministicFaultV2::Invariant,
                        ));
                    }
                };
                self.conflict = Some(Box::new(super::step_job::LivingConflictJobV2::begin_owned(
                    Arc::clone(&self.capture.catalog),
                    state,
                    events,
                    LivingTickV2(self.capture.state.tick.0 + 1),
                    self.capture.genesis_seed,
                    arrivals,
                )));

                self.retirement.start(self.scheduled.take());
            } else if self.conflict.is_some() {
                self.install_production_from_conflict()?;
            } else if let Some(production) = &mut self.production {
                let (state, events) = match production.take_ready_result() {
                    Ok(output) => output,
                    Err(LivingLifecycleJobErrorV2::PreparationCapacity) => {
                        self.preparation_capacity = true;
                        return Err(LivingStepJobErrorV2::PreparationCapacity);
                    }
                    Err(error) => {
                        let cause = match error {
                            LivingLifecycleJobErrorV2::Fault(fault) => {
                                super::step_job::ProductionFailure::capture(&fault)
                            }
                            _ => super::step_job::ProductionFailure::Invariant,
                        };
                        self.production_fault = Some(cause);
                        return Err(LivingStepJobErrorV2::Fault(cause.public()));
                    }
                };
                self.candidate = Some(state);
                self.observation_events = Some(events);
                self.production_handoff = true;
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
    /// Legacy consuming errors cannot preserve the caller's job; resident errors
    /// require borrowed extraction and an explicit retained retirement handoff.
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
        if self.preparation_capacity {
            return Err(LivingStepJobErrorV2::PreparationCapacity);
        }
        if let Some(super::step_job::LivingLifecycleJobErrorV2::Fault(fault)) = self
            .creator
            .as_ref()
            .and_then(|creator| creator.structural_failure())
        {
            return Ok(LivingStepPreparationV2::Failed(Box::new(
                LivingPreparedStepFailureV2 {
                    owners: Some(LivingPreparedFailureOwnersV2 {
                        capture: self.capture_for_preparation(),
                        fault,
                    }),
                },
            )));
        }
        if let Some(fault) = self.production_fault.take() {
            return Ok(LivingStepPreparationV2::Failed(Box::new(
                LivingPreparedStepFailureV2 {
                    owners: Some(LivingPreparedFailureOwnersV2 {
                        capture: self.capture_for_preparation(),
                        fault: fault.public(),
                    }),
                },
            )));
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
            if self.scheduled.is_some() {
                return Err(LivingStepJobErrorV2::NotReady);
            }
            let creator = self
                .creator
                .as_mut()
                .ok_or(LivingStepJobErrorV2::NotReady)?;
            if !matches!(creator.stage, QueuedCreatorStage::NeedsSynchronous) {
                return Err(LivingStepJobErrorV2::NotReady);
            }
            let (candidate, events) = match creator.take_synchronous_result() {
                Ok(output) => output,
                Err(super::step_job::LivingLifecycleJobErrorV2::PreparationCapacity) => {
                    self.preparation_capacity = true;
                    return Err(LivingStepJobErrorV2::PreparationCapacity);
                }
                Err(error) => {
                    let fault = match error {
                        super::step_job::LivingLifecycleJobErrorV2::Fault(fault) => fault,
                        _ => LivingDeterministicFaultV2::Invariant,
                    };
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
            self.scheduled = Some(Box::new(
                super::step_job::LivingScheduledPhasesJobV2::begin(
                    candidate,
                    events,
                    LivingTickV2(self.capture.state.tick.0 + 1),
                ),
            ));
            loop {
                match self
                    .scheduled
                    .as_mut()
                    .expect("retained scheduled owner")
                    .poll(1024)
                {
                    Ok(super::step_job::LivingLifecycleProgressV2::Pending) => {}
                    Ok(super::step_job::LivingLifecycleProgressV2::Ready) => break,
                    Err(super::step_job::LivingLifecycleJobErrorV2::PreparationCapacity) => {
                        self.preparation_capacity = true;
                        return Err(LivingStepJobErrorV2::PreparationCapacity);
                    }
                    Err(error) => {
                        let fault = match error {
                            super::step_job::LivingLifecycleJobErrorV2::Fault(fault) => fault,
                            _ => LivingDeterministicFaultV2::Invariant,
                        };
                        return Ok(LivingStepPreparationV2::Failed(Box::new(
                            LivingPreparedStepFailureV2 {
                                owners: Some(LivingPreparedFailureOwnersV2 {
                                    capture: self.capture_for_preparation(),
                                    fault,
                                }),
                            },
                        )));
                    }
                }
            }
            let (candidate, events, arrivals) =
                match self.scheduled.as_mut().unwrap().take_ready_result() {
                    Ok(output) => output,
                    Err(super::step_job::LivingLifecycleJobErrorV2::PreparationCapacity) => {
                        self.preparation_capacity = true;
                        return Err(LivingStepJobErrorV2::PreparationCapacity);
                    }
                    Err(super::step_job::LivingLifecycleJobErrorV2::Fault(fault)) => {
                        return Err(LivingStepJobErrorV2::Fault(fault));
                    }
                    Err(_) => {
                        return Err(LivingStepJobErrorV2::Fault(
                            LivingDeterministicFaultV2::Invariant,
                        ));
                    }
                };
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
    HullReassemble,
    WorldShipments {
        world: usize,
        index: usize,
    },
    WorldFleets {
        world: usize,
        index: usize,
    },
    WorldInstall {
        world: usize,
    },
    LaneFleets {
        lane: usize,
        index: usize,
    },
    LanePath {
        lane: usize,
        fleet: usize,
        path: usize,
    },
    LaneInstall {
        lane: usize,
    },
    HullDefinition {
        hull: LivingEntityIdV2,
        index: usize,
    },
    HullParent {
        hull: LivingEntityIdV2,
        index: usize,
    },
    FleetEditReservations {
        fleet: usize,
        id: LivingEntityIdV2,
        hull: Option<(usize, LivingEntityIdV2)>,
        index: usize,
    },
    FleetEditInstall {
        fleet: usize,
        id: LivingEntityIdV2,
        hull: Option<(usize, LivingEntityIdV2)>,
    },
    HazardDefinition,
    HazardEdit,
    ColonyConstructionReservations,
    ColonyHullReservations,
    ColonyMutation,
    ColonyOwnerEdit,
    ColonyCreateInsertion,
    ColonyRemoval,
    StatusOwners,
    StatusCivilizations,
    RecoveryPrepare,
    RecoveryOwners,
    RecoveryActor,
    RecoveryEmit,
    EmitCreator,
    AppendCreator,
    FinishRevision,
    NeedsSynchronous,
    Ready,
    AgreementPlan,
    OccupationPlan,
    OccupationOwner,
    DiplomacyPlanEmit,
    DiplomacyFilter,
    WarInsertion,
    OrderReservations,
    OrderPrepare,
    OrderPath,
    OrderInstall,
    DiplomacyFinish,
    CancelRefund,
    CancelRefundWrite,
    CancelPlan,
    CancelFilterStart,
    CancelFilter,
    CancelEffect,
    CancellationFinish,
    RouteSource,
    RouteDestination,
    RouteWar,
    RouteKind,
    RouteTrade,
    RouteDeadline,
    RouteInstall,
    EffectPrepare,
    EffectCommit,
    EffectsStart,
    EffectsNext,
    EffectAppendStart,
    EffectAppend,
    BridgeEffectsStart,
    BridgeEffectsNext,
}
#[derive(Clone, Copy)]
enum QueuedColonyOperation {
    SetOwner {
        world: LivingEntityIdV2,
        owner: Option<LivingEntityIdV2>,
    },
    Remove {
        world: LivingEntityIdV2,
    },
}
impl QueuedColonyOperation {
    fn blocks(self, world: LivingEntityIdV2, owner: LivingEntityIdV2) -> bool {
        match self {
            Self::SetOwner {
                world: target,
                owner: desired,
            } => world == target && Some(owner) != desired,
            Self::Remove { world: target } => world == target,
        }
    }
    fn rejection(self) -> LivingCommandRejectionV2 {
        match self {
            Self::SetOwner { .. } => {
                LivingCommandRejectionV2::Projection(LivingValidationErrorV2::Range {
                    field: "foreign_jobs_on_world",
                })
            }
            Self::Remove { .. } => LivingCommandRejectionV2::ReferencedTarget,
        }
    }
}
// Keep complete ownership inline; no allocation is hidden in dispatch.
#[allow(clippy::large_enum_variant)]
enum QueuedCreatorInsertion {
    Removal(QueuedRemovalWork),
    FleetStructural(QueuedFleetStructuralWork),
    System(super::step_job::LivingSortedInsertionJobV2<LivingSystemV2>),
    Star(super::step_job::LivingSortedInsertionJobV2<LivingStarV2>),
    World(super::step_job::LivingSortedInsertionJobV2<LivingWorldV2>),
    Lane(super::step_job::LivingSortedInsertionJobV2<LivingLaneV2>),
    Civilization(super::step_job::LivingSortedInsertionJobV2<LivingCivilizationV2>),
    Deposit(super::step_job::LivingSortedInsertionJobV2<super::model::LivingDepositV2>),
    Facility(super::step_job::LivingSortedInsertionJobV2<super::model::LivingFacilityV2>),
    ConstructionJob(
        super::step_job::LivingSortedInsertionJobV2<super::model::LivingConstructionJobV2>,
    ),
    HullJob(super::step_job::LivingSortedInsertionJobV2<super::model::LivingHullJobV2>),
    Fleet(super::step_job::LivingSortedInsertionJobV2<super::model::LivingFleetV2>),
    Hull(super::step_job::LivingSortedInsertionJobV2<super::model::LivingHullV2>),
    Route(super::step_job::LivingSortedInsertionJobV2<super::model::LivingFreightRouteV2>),
    Shipment(super::step_job::LivingSortedInsertionJobV2<super::model::LivingShipmentV2>),
    Hazard(super::step_job::LivingSortedInsertionJobV2<super::model::LivingHazardV2>),
    Colony(super::step_job::LivingSortedInsertionJobV2<super::model::LivingColonyV2>),
    War(super::step_job::LivingSortedInsertionJobV2<super::model::LivingWarV2>),
    Agreement(super::step_job::LivingSortedInsertionJobV2<super::model::LivingAgreementV2>),
    Relation(super::step_job::LivingSortedInsertionJobV2<super::model::LivingRelationV2>),
}
// One complete removal operation stays inside the original creator insertion slot.
#[allow(clippy::large_enum_variant)]
enum QueuedRemovalFilter {
    System(super::step_job::LivingPlannedFilterJobV2<super::model::LivingSystemV2>),
    Star(super::step_job::LivingPlannedFilterJobV2<super::model::LivingStarV2>),
    World(super::step_job::LivingPlannedFilterJobV2<super::model::LivingWorldV2>),
    Lane(super::step_job::LivingPlannedFilterJobV2<super::model::LivingLaneV2>),
    Civilization(super::step_job::LivingPlannedFilterJobV2<super::model::LivingCivilizationV2>),
    Deposit(super::step_job::LivingPlannedFilterJobV2<super::model::LivingDepositV2>),
    Facility(super::step_job::LivingPlannedFilterJobV2<super::model::LivingFacilityV2>),
    ConstructionJob(
        super::step_job::LivingPlannedFilterJobV2<super::model::LivingConstructionJobV2>,
    ),
    HullJob(super::step_job::LivingPlannedFilterJobV2<super::model::LivingHullJobV2>),
    Fleet(super::step_job::LivingPlannedFilterJobV2<super::model::LivingFleetV2>),
    Route(super::step_job::LivingPlannedFilterJobV2<super::model::LivingFreightRouteV2>),
    Shipment(super::step_job::LivingPlannedFilterJobV2<super::model::LivingShipmentV2>),
    Hazard(super::step_job::LivingPlannedFilterJobV2<super::model::LivingHazardV2>),
    Colony(super::step_job::LivingPlannedFilterJobV2<super::model::LivingColonyV2>),
    Observation(super::step_job::LivingPlannedFilterJobV2<super::model::LivingObservationV2>),
    Relation(super::step_job::LivingPlannedFilterJobV2<super::model::LivingRelationV2>),
    Agreement(super::step_job::LivingPlannedFilterJobV2<super::model::LivingAgreementV2>),
    War(super::step_job::LivingPlannedFilterJobV2<super::model::LivingWarV2>),
    SettlementClaim(
        super::step_job::LivingPlannedFilterJobV2<super::model::LivingSettlementClaimV2>,
    ),
    Occupation(super::step_job::LivingPlannedFilterJobV2<super::model::LivingOccupationV2>),
    Hull(super::step_job::LivingPlannedFilterJobV2<super::model::LivingHullV2>),
}
impl QueuedRemovalFilter {
    #[cfg(test)]
    fn inject_capacity(&mut self) -> bool {
        match self {
            Self::System(job) => job.inject_creator_capacity_failure(),
            Self::Star(job) => job.inject_creator_capacity_failure(),
            Self::World(job) => job.inject_creator_capacity_failure(),
            Self::Lane(job) => job.inject_creator_capacity_failure(),
            Self::Civilization(job) => job.inject_creator_capacity_failure(),
            Self::Deposit(job) => job.inject_creator_capacity_failure(),
            Self::Facility(job) => job.inject_creator_capacity_failure(),
            Self::ConstructionJob(job) => job.inject_creator_capacity_failure(),
            Self::HullJob(job) => job.inject_creator_capacity_failure(),
            Self::Fleet(job) => job.inject_creator_capacity_failure(),
            Self::Route(job) => job.inject_creator_capacity_failure(),
            Self::Shipment(job) => job.inject_creator_capacity_failure(),
            Self::Hazard(job) => job.inject_creator_capacity_failure(),
            Self::Colony(job) => job.inject_creator_capacity_failure(),
            Self::Observation(job) => job.inject_creator_capacity_failure(),
            Self::Relation(job) => job.inject_creator_capacity_failure(),
            Self::Agreement(job) => job.inject_creator_capacity_failure(),
            Self::War(job) => job.inject_creator_capacity_failure(),
            Self::SettlementClaim(job) => job.inject_creator_capacity_failure(),
            Self::Occupation(job) => job.inject_creator_capacity_failure(),
            Self::Hull(job) => job.inject_creator_capacity_failure(),
        }
    }
    #[cfg(test)]
    fn take_test_preparation(&mut self) -> Box<dyn super::step_job::RetirementPoll> {
        match self {
            Self::System(job) => job.take_removal_test_preparation(),
            Self::Star(job) => job.take_removal_test_preparation(),
            Self::World(job) => job.take_removal_test_preparation(),
            Self::Lane(job) => job.take_removal_test_preparation(),
            Self::Civilization(job) => job.take_removal_test_preparation(),
            Self::Deposit(job) => job.take_removal_test_preparation(),
            Self::Facility(job) => job.take_removal_test_preparation(),
            Self::ConstructionJob(job) => job.take_removal_test_preparation(),
            Self::HullJob(job) => job.take_removal_test_preparation(),
            Self::Fleet(job) => job.take_removal_test_preparation(),
            Self::Route(job) => job.take_removal_test_preparation(),
            Self::Shipment(job) => job.take_removal_test_preparation(),
            Self::Hazard(job) => job.take_removal_test_preparation(),
            Self::Colony(job) => job.take_removal_test_preparation(),
            Self::Observation(job) => job.take_removal_test_preparation(),
            Self::Relation(job) => job.take_removal_test_preparation(),
            Self::Agreement(job) => job.take_removal_test_preparation(),
            Self::War(job) => job.take_removal_test_preparation(),
            Self::SettlementClaim(job) => job.take_removal_test_preparation(),
            Self::Occupation(job) => job.take_removal_test_preparation(),
            Self::Hull(job) => job.take_removal_test_preparation(),
        }
    }
    #[cfg(test)]
    fn take_test_output(&mut self) -> Box<dyn super::step_job::RetirementPoll> {
        use super::step_job::{LivingStateRetirementPolicyV2, retirement_node};
        match self {
            Self::System(job) => {
                let output = job.take_ready_result().unwrap();
                assert!(job.take_ready_result().is_err());
                retirement_node(output, LivingStateRetirementPolicyV2::ReleaseHandle)
            }
            Self::Star(job) => {
                let output = job.take_ready_result().unwrap();
                assert!(job.take_ready_result().is_err());
                retirement_node(output, LivingStateRetirementPolicyV2::ReleaseHandle)
            }
            Self::World(job) => {
                let output = job.take_ready_result().unwrap();
                assert!(job.take_ready_result().is_err());
                retirement_node(output, LivingStateRetirementPolicyV2::ReleaseHandle)
            }
            Self::Lane(job) => {
                let output = job.take_ready_result().unwrap();
                assert!(job.take_ready_result().is_err());
                retirement_node(output, LivingStateRetirementPolicyV2::ReleaseHandle)
            }
            Self::Civilization(job) => {
                let output = job.take_ready_result().unwrap();
                assert!(job.take_ready_result().is_err());
                retirement_node(output, LivingStateRetirementPolicyV2::ReleaseHandle)
            }
            Self::Deposit(job) => {
                let output = job.take_ready_result().unwrap();
                assert!(job.take_ready_result().is_err());
                retirement_node(output, LivingStateRetirementPolicyV2::ReleaseHandle)
            }
            Self::Facility(job) => {
                let output = job.take_ready_result().unwrap();
                assert!(job.take_ready_result().is_err());
                retirement_node(output, LivingStateRetirementPolicyV2::ReleaseHandle)
            }
            Self::ConstructionJob(job) => {
                let output = job.take_ready_result().unwrap();
                assert!(job.take_ready_result().is_err());
                retirement_node(output, LivingStateRetirementPolicyV2::ReleaseHandle)
            }
            Self::HullJob(job) => {
                let output = job.take_ready_result().unwrap();
                assert!(job.take_ready_result().is_err());
                retirement_node(output, LivingStateRetirementPolicyV2::ReleaseHandle)
            }
            Self::Fleet(job) => {
                let output = job.take_ready_result().unwrap();
                assert!(job.take_ready_result().is_err());
                retirement_node(output, LivingStateRetirementPolicyV2::ReleaseHandle)
            }
            Self::Route(job) => {
                let output = job.take_ready_result().unwrap();
                assert!(job.take_ready_result().is_err());
                retirement_node(output, LivingStateRetirementPolicyV2::ReleaseHandle)
            }
            Self::Shipment(job) => {
                let output = job.take_ready_result().unwrap();
                assert!(job.take_ready_result().is_err());
                retirement_node(output, LivingStateRetirementPolicyV2::ReleaseHandle)
            }
            Self::Hazard(job) => {
                let output = job.take_ready_result().unwrap();
                assert!(job.take_ready_result().is_err());
                retirement_node(output, LivingStateRetirementPolicyV2::ReleaseHandle)
            }
            Self::Colony(job) => {
                let output = job.take_ready_result().unwrap();
                assert!(job.take_ready_result().is_err());
                retirement_node(output, LivingStateRetirementPolicyV2::ReleaseHandle)
            }
            Self::Observation(job) => {
                let output = job.take_ready_result().unwrap();
                assert!(job.take_ready_result().is_err());
                retirement_node(output, LivingStateRetirementPolicyV2::ReleaseHandle)
            }
            Self::Relation(job) => {
                let output = job.take_ready_result().unwrap();
                assert!(job.take_ready_result().is_err());
                retirement_node(output, LivingStateRetirementPolicyV2::ReleaseHandle)
            }
            Self::Agreement(job) => {
                let output = job.take_ready_result().unwrap();
                assert!(job.take_ready_result().is_err());
                retirement_node(output, LivingStateRetirementPolicyV2::ReleaseHandle)
            }
            Self::War(job) => {
                let output = job.take_ready_result().unwrap();
                assert!(job.take_ready_result().is_err());
                retirement_node(output, LivingStateRetirementPolicyV2::ReleaseHandle)
            }
            Self::SettlementClaim(job) => {
                let output = job.take_ready_result().unwrap();
                assert!(job.take_ready_result().is_err());
                retirement_node(output, LivingStateRetirementPolicyV2::ReleaseHandle)
            }
            Self::Occupation(job) => {
                let output = job.take_ready_result().unwrap();
                assert!(job.take_ready_result().is_err());
                retirement_node(output, LivingStateRetirementPolicyV2::ReleaseHandle)
            }
            Self::Hull(job) => {
                let output = job.take_ready_result().unwrap();
                assert!(job.take_ready_result().is_err());
                retirement_node(output, LivingStateRetirementPolicyV2::ReleaseHandle)
            }
        }
    }

    fn ready(&self) -> bool {
        match self {
            Self::System(job) => job.ready(),
            Self::Star(job) => job.ready(),
            Self::World(job) => job.ready(),
            Self::Lane(job) => job.ready(),
            Self::Civilization(job) => job.ready(),
            Self::Deposit(job) => job.ready(),
            Self::Facility(job) => job.ready(),
            Self::ConstructionJob(job) => job.ready(),
            Self::HullJob(job) => job.ready(),
            Self::Fleet(job) => job.ready(),
            Self::Route(job) => job.ready(),
            Self::Shipment(job) => job.ready(),
            Self::Hazard(job) => job.ready(),
            Self::Colony(job) => job.ready(),
            Self::Observation(job) => job.ready(),
            Self::Relation(job) => job.ready(),
            Self::Agreement(job) => job.ready(),
            Self::War(job) => job.ready(),
            Self::SettlementClaim(job) => job.ready(),
            Self::Occupation(job) => job.ready(),
            Self::Hull(job) => job.ready(),
        }
    }
    fn visit(&mut self) -> Result<bool, QueuedRemovalFault> {
        match self {
            Self::System(job) => job.visit().map_err(|_| match job.failure() {
                Some(cause) => QueuedRemovalFault::filter(cause),
                None => QueuedRemovalFault::Invariant,
            }),
            Self::Star(job) => job.visit().map_err(|_| match job.failure() {
                Some(cause) => QueuedRemovalFault::filter(cause),
                None => QueuedRemovalFault::Invariant,
            }),
            Self::World(job) => job.visit().map_err(|_| match job.failure() {
                Some(cause) => QueuedRemovalFault::filter(cause),
                None => QueuedRemovalFault::Invariant,
            }),
            Self::Lane(job) => job.visit().map_err(|_| match job.failure() {
                Some(cause) => QueuedRemovalFault::filter(cause),
                None => QueuedRemovalFault::Invariant,
            }),
            Self::Civilization(job) => job.visit().map_err(|_| match job.failure() {
                Some(cause) => QueuedRemovalFault::filter(cause),
                None => QueuedRemovalFault::Invariant,
            }),
            Self::Deposit(job) => job.visit().map_err(|_| match job.failure() {
                Some(cause) => QueuedRemovalFault::filter(cause),
                None => QueuedRemovalFault::Invariant,
            }),
            Self::Facility(job) => job.visit().map_err(|_| match job.failure() {
                Some(cause) => QueuedRemovalFault::filter(cause),
                None => QueuedRemovalFault::Invariant,
            }),
            Self::ConstructionJob(job) => job.visit().map_err(|_| match job.failure() {
                Some(cause) => QueuedRemovalFault::filter(cause),
                None => QueuedRemovalFault::Invariant,
            }),
            Self::HullJob(job) => job.visit().map_err(|_| match job.failure() {
                Some(cause) => QueuedRemovalFault::filter(cause),
                None => QueuedRemovalFault::Invariant,
            }),
            Self::Fleet(job) => job.visit().map_err(|_| match job.failure() {
                Some(cause) => QueuedRemovalFault::filter(cause),
                None => QueuedRemovalFault::Invariant,
            }),
            Self::Route(job) => job.visit().map_err(|_| match job.failure() {
                Some(cause) => QueuedRemovalFault::filter(cause),
                None => QueuedRemovalFault::Invariant,
            }),
            Self::Shipment(job) => job.visit().map_err(|_| match job.failure() {
                Some(cause) => QueuedRemovalFault::filter(cause),
                None => QueuedRemovalFault::Invariant,
            }),
            Self::Hazard(job) => job.visit().map_err(|_| match job.failure() {
                Some(cause) => QueuedRemovalFault::filter(cause),
                None => QueuedRemovalFault::Invariant,
            }),
            Self::Colony(job) => job.visit().map_err(|_| match job.failure() {
                Some(cause) => QueuedRemovalFault::filter(cause),
                None => QueuedRemovalFault::Invariant,
            }),
            Self::Observation(job) => job.visit().map_err(|_| match job.failure() {
                Some(cause) => QueuedRemovalFault::filter(cause),
                None => QueuedRemovalFault::Invariant,
            }),
            Self::Relation(job) => job.visit().map_err(|_| match job.failure() {
                Some(cause) => QueuedRemovalFault::filter(cause),
                None => QueuedRemovalFault::Invariant,
            }),
            Self::Agreement(job) => job.visit().map_err(|_| match job.failure() {
                Some(cause) => QueuedRemovalFault::filter(cause),
                None => QueuedRemovalFault::Invariant,
            }),
            Self::War(job) => job.visit().map_err(|_| match job.failure() {
                Some(cause) => QueuedRemovalFault::filter(cause),
                None => QueuedRemovalFault::Invariant,
            }),
            Self::SettlementClaim(job) => job.visit().map_err(|_| match job.failure() {
                Some(cause) => QueuedRemovalFault::filter(cause),
                None => QueuedRemovalFault::Invariant,
            }),
            Self::Occupation(job) => job.visit().map_err(|_| match job.failure() {
                Some(cause) => QueuedRemovalFault::filter(cause),
                None => QueuedRemovalFault::Invariant,
            }),
            Self::Hull(job) => job.visit().map_err(|_| match job.failure() {
                Some(cause) => QueuedRemovalFault::filter(cause),
                None => QueuedRemovalFault::Invariant,
            }),
        }
    }
}
/// Closed causes of removal and its concrete storage/filter children.
#[derive(Clone, Copy)]
enum QueuedRemovalFault {
    Missing(&'static str),
    Referenced,
    Mismatch,
    KeyedOrder,
    DuplicateIdentity,
    PreparationCapacity,
    Invariant,
}
impl QueuedRemovalFault {
    fn storage(cause: super::step_job::LivingStorageJobErrorV2) -> Self {
        use super::step_job::LivingStorageJobErrorV2 as E;
        match cause {
            E::CapacityOverflow => Self::PreparationCapacity,
            E::KeyedOrder => Self::KeyedOrder,
            E::Budget | E::Cancelled | E::NotReady => Self::Invariant,
        }
    }
    fn filter(cause: super::step_job::InsertionFault) -> Self {
        use super::step_job::InsertionFault as E;
        match cause {
            E::KeyedOrder => Self::KeyedOrder,
            E::Capacity => Self::PreparationCapacity,
            // Preserve the concrete shared cause even though a planned filter
            // cannot currently construct Duplicate (only insertion can).
            E::Duplicate => Self::DuplicateIdentity,
            E::Preparation => Self::Invariant,
        }
    }
    fn public(self, sequence: u64, operation: usize) -> super::step_job::LivingLifecycleJobErrorV2 {
        use super::step_job::LivingLifecycleJobErrorV2 as E;
        let reason = match self {
            Self::Missing(reference) => LivingCommandRejectionV2::MissingTarget {
                operation,
                reference,
            },
            Self::Referenced => LivingCommandRejectionV2::ReferencedTarget,
            Self::Mismatch => LivingCommandRejectionV2::CascadeMismatch,
            Self::DuplicateIdentity => {
                LivingCommandRejectionV2::Projection(LivingValidationErrorV2::DuplicateIdentity {
                    context: "creator batch",
                })
            }
            Self::KeyedOrder => {
                LivingCommandRejectionV2::Projection(LivingWireErrorV2::KeyedOrder.into())
            }
            Self::PreparationCapacity => return E::PreparationCapacity,
            Self::Invariant => return E::Fault(LivingDeterministicFaultV2::Invariant),
        };
        E::Fault(LivingDeterministicFaultV2::Replay {
            accepted_sequence: sequence,
            reason: Box::new(reason),
        })
    }
}
#[derive(Clone, Copy, PartialEq, Eq)]
enum QueuedRemovalStage {
    Resolve,
    Discover,
    Seed,
    ClosureRow,
    ClosureReference,
    ClosureInsert,
    Legacy,
    DependentsPrepare,
    DependentsRead,
    DependentsEmit,
    Check,
    DeclaredRead,
    DeclaredInsert,
    Compare,
    FilterPlan,
    FilterInsert,
    FilterStart,
    Filter,
    Ready,
}
struct QueuedRemovalWork {
    stage: QueuedRemovalStage,
    target: Option<LivingEntityIdV2>,
    listed: bool,
    registry: usize,
    row: usize,
    hull: usize,
    reference: usize,
    grew: bool,
    legacy: usize,
    input_len: usize,
    index: usize,
    nested_fleet: usize,
    nested: bool,
    removed: std::collections::BTreeSet<LivingEntityIdV2>,
    dependents: Vec<LivingEntityIdV2>,
    declared: std::collections::BTreeSet<LivingEntityIdV2>,
    pending: Option<LivingEntityIdV2>,
    storage: Option<super::step_job::LivingVecStorageJobV2<'static, LivingEntityIdV2>>,
    plan: std::collections::BTreeSet<usize>,
    filter: Option<QueuedRemovalFilter>,
    retirement: super::step_job::PhaseRetirementLane<'static>,
    fault: Option<QueuedRemovalFault>,
}
impl QueuedRemovalWork {
    fn begin() -> Self {
        Self {
            stage: QueuedRemovalStage::Resolve,
            target: None,
            listed: false,
            registry: 0,
            row: 0,
            hull: 0,
            reference: 0,
            grew: false,
            legacy: 0,
            input_len: 0,
            index: 0,
            nested_fleet: 0,
            nested: false,
            removed: std::collections::BTreeSet::new(),
            dependents: Vec::new(),
            declared: std::collections::BTreeSet::new(),
            pending: None,
            storage: None,
            plan: std::collections::BTreeSet::new(),
            filter: None,
            retirement: super::step_job::PhaseRetirementLane::default(),
            fault: None,
        }
    }
    fn ready(&self) -> bool {
        self.stage == QueuedRemovalStage::Ready
            && self.fault.is_none()
            && self.retirement.is_empty()
    }
    fn resolve(
        target: &LivingCommandTargetV2,
        locals: &std::collections::BTreeMap<u16, LivingEntityIdV2>,
    ) -> Result<LivingEntityIdV2, QueuedRemovalFault> {
        match target {
            LivingCommandTargetV2::Existing { id } => Ok(*id),
            LivingCommandTargetV2::Local { local } => locals
                .get(local)
                .copied()
                .ok_or(QueuedRemovalFault::Invariant),
        }
    }
    // This view borrows the frozen candidate. Hulls immediately follow their Fleet,
    // including when the Fleet itself is already in the closure.
    fn stable_id(&self, state: &LivingGalaxyStateV2) -> Option<LivingEntityIdV2> {
        match self.registry {
            0 => state.systems.as_slice().get(self.row).map(|row| row.id),
            1 => state.stars.as_slice().get(self.row).map(|row| row.id),
            2 => state.worlds.as_slice().get(self.row).map(|row| row.id),
            3 => state.lanes.as_slice().get(self.row).map(|row| row.id),
            4 => state
                .civilizations
                .as_slice()
                .get(self.row)
                .map(|row| row.id),
            5 => state.deposits.as_slice().get(self.row).map(|row| row.id),
            6 => state.facilities.as_slice().get(self.row).map(|row| row.id),
            7 => state
                .construction_jobs
                .as_slice()
                .get(self.row)
                .map(|row| row.id),
            8 => state.hull_jobs.as_slice().get(self.row).map(|row| row.id),
            9 => state.fleets.as_slice().get(self.row).and_then(|fleet| {
                if self.hull == 0 {
                    Some(fleet.id)
                } else {
                    fleet
                        .hulls
                        .as_slice()
                        .get(self.hull - 1)
                        .map(|hull| hull.id)
                }
            }),
            10 => state.routes.as_slice().get(self.row).map(|row| row.id),
            11 => state.shipments.as_slice().get(self.row).map(|row| row.id),
            12 => state.hazards.as_slice().get(self.row).map(|row| row.id),
            _ => None,
        }
    }
    fn advance_stable(&mut self, state: &LivingGalaxyStateV2) {
        if self.registry == 9
            && self.hull < state.fleets.as_slice()[self.row].hulls.as_slice().len()
        {
            self.hull += 1;
        } else {
            self.row += 1;
            self.hull = 0;
        }
    }
    fn stable_reference(&self, state: &LivingGalaxyStateV2) -> Option<LivingEntityIdV2> {
        let i = self.reference;
        match self.registry {
            0 | 4 => None,
            1 => (i == 0).then(|| state.stars.as_slice()[self.row].system),
            2 => {
                let row = &state.worlds.as_slice()[self.row];
                match i {
                    0 => Some(row.system),
                    1 => row.owner,
                    _ => None,
                }
            }
            3 => {
                let row = &state.lanes.as_slice()[self.row];
                [row.system_a, row.system_b].get(i).copied()
            }
            5 => (i == 0).then(|| state.deposits.as_slice()[self.row].world),
            6 => (i == 0).then(|| state.facilities.as_slice()[self.row].world),
            7 => {
                let row = &state.construction_jobs.as_slice()[self.row];
                [row.world, row.owner].get(i).copied()
            }
            8 => {
                let row = &state.hull_jobs.as_slice()[self.row];
                match i {
                    0 => Some(row.world),
                    1 => Some(row.owner),
                    2 => row.target_fleet,
                    _ => None,
                }
            }
            9 => {
                let fleet = &state.fleets.as_slice()[self.row];
                if self.hull != 0 {
                    return (i == 0).then_some(fleet.id);
                }
                if i == 0 {
                    return Some(fleet.owner);
                }
                let mut i = i - 1;
                match fleet.location {
                    super::model::LivingFleetLocationV2::Docked { world } => {
                        if i == 0 {
                            return Some(world);
                        }
                        i -= 1;
                    }
                    super::model::LivingFleetLocationV2::Travelling {
                        origin_world,
                        destination_world,
                        ..
                    } => {
                        if let Some(id) = [origin_world, destination_world].get(i) {
                            return Some(*id);
                        }
                        i -= 2;
                    }
                }
                let order = fleet.order.as_ref()?;
                if let Some(id) = order.target_world {
                    if i == 0 {
                        return Some(id);
                    }
                    i -= 1;
                }
                order.lane_path.get(i).copied()
            }
            10 => {
                let row = &state.routes.as_slice()[self.row];
                [
                    row.source_world,
                    row.destination_world,
                    row.source_owner,
                    row.receiver_owner,
                ]
                .get(i)
                .copied()
            }
            11 => {
                let row = &state.shipments.as_slice()[self.row];
                [
                    row.source_world,
                    row.destination_world,
                    row.dispatch_owner,
                    row.intended_receiver,
                ]
                .get(i)
                .copied()
            }
            12 => (i == 0).then(|| state.hazards.as_slice()[self.row].lane),
            _ => None,
        }
    }
    fn derived_match(
        &self,
        state: &LivingGalaxyStateV2,
        registry: usize,
        index: usize,
    ) -> Option<bool> {
        let (a, b) = match registry {
            13 => {
                let row = state.colonies.as_slice().get(index)?;
                (row.world, row.owner)
            }
            14 => {
                let row = state.observations.as_slice().get(index)?;
                (row.world, row.civilization)
            }
            15 => {
                let row = state.relations.as_slice().get(index)?;
                (row.from, row.to)
            }
            16 => {
                let row = state.agreements.as_slice().get(index)?;
                (row.participant_a, row.participant_b)
            }
            17 => {
                let row = state.wars.as_slice().get(index)?;
                (row.participant_a, row.participant_b)
            }
            18 => {
                let row = state.settlement_claims.as_slice().get(index)?;
                (row.world, row.civilization)
            }
            19 => {
                let row = state.occupations.as_slice().get(index)?;
                (row.world, row.claimant)
            }
            _ => return None,
        };
        Some(self.removed.contains(&a) || self.removed.contains(&b))
    }
    fn visit(
        &mut self,
        state: &mut LivingGalaxyStateV2,
        operation: &LivingCreatorOperationV2,
        locals: &std::collections::BTreeMap<u16, LivingEntityIdV2>,
    ) -> Result<bool, QueuedRemovalFault> {
        if let Some(cause) = self.fault {
            return Err(cause);
        }
        if self.retirement.visit() {
            return Ok(true);
        }
        let result = self.visit_record(state, operation, locals);
        if let Err(cause) = result {
            self.fault = Some(cause);
        }
        result
    }
    fn visit_record(
        &mut self,
        state: &mut LivingGalaxyStateV2,
        operation: &LivingCreatorOperationV2,
        locals: &std::collections::BTreeMap<u16, LivingEntityIdV2>,
    ) -> Result<bool, QueuedRemovalFault> {
        use super::command::LivingCascadeDispositionV2 as Cascade;
        use QueuedRemovalFault as F;
        use QueuedRemovalStage as S;
        let (target, cascade, scrap) = match operation {
            LivingCreatorOperationV2::RemoveEntity { target, cascade } => (target, cascade, false),
            LivingCreatorOperationV2::ScrapFacility { target, cascade } => (target, cascade, true),
            _ => return Err(F::Invariant),
        };
        match self.stage {
            S::Resolve => {
                let id = Self::resolve(target, locals)?;
                self.target = Some(id);
                if scrap
                    && state
                        .facilities
                        .as_slice()
                        .binary_search_by_key(&id, |row| row.id)
                        .is_err()
                {
                    return Err(F::Missing("facility"));
                }
                match cascade {
                    Cascade::RejectIfReferenced => {}
                    Cascade::RemoveListed { dependents } => {
                        self.listed = true;
                        self.input_len = dependents.len();
                    }
                    Cascade::RemoveReviewed { .. } => return Err(F::Invariant),
                }
                self.stage = S::Discover;
            }
            S::Discover => {
                if self.registry == 13 {
                    return Err(F::Missing("removal entity"));
                }
                if let Some(id) = self.stable_id(state) {
                    if Some(id) == self.target {
                        self.stage = S::Seed;
                    } else {
                        self.advance_stable(state);
                    }
                } else {
                    self.registry += 1;
                    self.row = 0;
                    self.hull = 0;
                }
            }
            S::Seed => {
                self.removed.insert(self.target.ok_or(F::Invariant)?);
                self.registry = 0;
                self.row = 0;
                self.hull = 0;
                self.stage = S::ClosureRow;
            }
            S::ClosureRow => {
                if self.registry == 13 {
                    if self.grew {
                        self.grew = false;
                        self.registry = 0;
                        self.row = 0;
                        self.hull = 0;
                    } else {
                        self.row = 0;
                        self.stage = S::Legacy;
                    }
                } else if let Some(id) = self.stable_id(state) {
                    self.pending = Some(id);
                    self.reference = 0;
                    self.stage = S::ClosureReference;
                } else {
                    self.registry += 1;
                    self.row = 0;
                    self.hull = 0;
                }
            }
            S::ClosureReference => {
                if let Some(id) = self.stable_reference(state) {
                    self.reference += 1;
                    if self.removed.contains(&id) {
                        self.stage = S::ClosureInsert;
                    }
                } else {
                    self.pending = None;
                    self.advance_stable(state);
                    self.stage = S::ClosureRow;
                }
            }
            S::ClosureInsert => {
                self.grew |= self
                    .removed
                    .insert(self.pending.take().ok_or(F::Invariant)?);
                self.advance_stable(state);
                self.stage = S::ClosureRow;
            }
            S::Legacy => {
                if self.registry == 20 {
                    self.stage = S::DependentsPrepare;
                } else if let Some(hit) = self.derived_match(state, self.registry, self.row) {
                    self.legacy += usize::from(hit);
                    self.row += 1;
                } else {
                    self.registry += 1;
                    self.row = 0;
                }
            }
            S::DependentsPrepare => {
                if let Some(child) = &mut self.storage {
                    if !child.ready() {
                        let result = child.poll(1);
                        let visits = child.last_poll_visits();
                        result.map_err(F::storage)?;
                        return Ok(visits != 0);
                    }
                    self.dependents = child.take_ready_result().map_err(F::storage)?;
                    self.retirement.start(self.storage.take());
                    self.stage = S::DependentsRead;
                } else {
                    self.storage = Some(super::step_job::LivingVecStorageJobV2::begin_empty(
                        self.removed.len() - 1,
                    ));
                }
            }
            S::DependentsRead => {
                use std::ops::Bound::{Excluded, Unbounded};
                // Bounded point successor lookup, with one counted emission per ID.
                let after = self.dependents.last().copied();
                let next = match after {
                    Some(id) => self
                        .removed
                        .range((Excluded(id), Unbounded))
                        .next()
                        .copied(),
                    None => self.removed.first().copied(),
                };
                if next == self.target {
                    let target = self.target.ok_or(F::Invariant)?;
                    self.pending = self
                        .removed
                        .range((Excluded(target), Unbounded))
                        .next()
                        .copied();
                } else {
                    self.pending = next;
                }
                self.stage = if self.pending.is_some() {
                    S::DependentsEmit
                } else {
                    S::Check
                };
            }
            S::DependentsEmit => {
                super::step_job::LivingVecStorageJobV2::commit_one(
                    &mut self.dependents,
                    &mut self.pending,
                )
                .map_err(F::storage)?;
                self.stage = S::DependentsRead;
            }
            S::Check => {
                if self.listed {
                    self.index = 0;
                    self.stage = S::DeclaredRead;
                } else if !self.dependents.is_empty() || self.legacy != 0 {
                    return Err(F::Referenced);
                } else {
                    self.registry = 0;
                    self.row = 0;
                    self.stage = S::FilterPlan;
                }
            }
            S::DeclaredRead => {
                let Cascade::RemoveListed { dependents } = cascade else {
                    return Err(F::Invariant);
                };
                if let Some(reference) = dependents.get(self.index) {
                    self.pending = Some(Self::resolve(reference, locals)?);
                    self.index += 1;
                    self.stage = S::DeclaredInsert;
                } else {
                    if self.declared.len() != self.input_len
                        || self.declared.len() != self.dependents.len()
                    {
                        return Err(F::Mismatch);
                    }
                    self.index = 0;
                    self.stage = S::Compare;
                }
            }
            S::DeclaredInsert => {
                self.declared
                    .insert(self.pending.take().ok_or(F::Invariant)?);
                self.stage = S::DeclaredRead;
            }
            S::Compare => {
                if let Some(id) = self.dependents.get(self.index) {
                    if !self.declared.contains(id) {
                        return Err(F::Mismatch);
                    }
                    self.index += 1;
                } else {
                    self.registry = 0;
                    self.row = 0;
                    self.stage = S::FilterPlan;
                }
            }
            S::FilterPlan => {
                if self.registry == 9 && !self.nested {
                    if self.nested_fleet < state.fleets.as_slice().len() {
                        self.nested = true;
                        self.row = 0;
                    } else {
                        self.registry = 20;
                        self.row = 0;
                    } // Fleet row filter, after all Hulls.
                    return Ok(true);
                }
                if self.registry == 13 && !self.listed || self.registry == 21 {
                    self.stage = S::Ready;
                    return Ok(true);
                }
                let hit = if self.nested {
                    state.fleets.as_slice()[self.nested_fleet]
                        .hulls
                        .as_slice()
                        .get(self.row)
                        .map(|row| self.removed.contains(&row.id))
                } else if self.registry == 20 {
                    state
                        .fleets
                        .as_slice()
                        .get(self.row)
                        .map(|row| self.removed.contains(&row.id))
                } else if self.registry < 13 {
                    self.stable_id(state).map(|id| self.removed.contains(&id))
                } else {
                    self.derived_match(state, self.registry, self.row)
                };
                match hit {
                    Some(true) => self.stage = S::FilterInsert,
                    Some(false) => self.row += 1,
                    None => self.stage = S::FilterStart,
                }
            }
            S::FilterInsert => {
                self.plan.insert(self.row);
                self.row += 1;
                self.stage = S::FilterPlan;
            }
            S::FilterStart => {
                let plan = std::mem::take(&mut self.plan);
                self.filter = Some(if self.nested {
                    QueuedRemovalFilter::Hull(super::step_job::LivingPlannedFilterJobV2::begin(
                        std::mem::take(
                            &mut state
                                .fleets
                                .record_at_mut(self.nested_fleet)
                                .ok_or(F::Invariant)?
                                .hulls,
                        ),
                        plan,
                    ))
                } else {
                    match self.registry {
                        0 => QueuedRemovalFilter::System(
                            super::step_job::LivingPlannedFilterJobV2::begin(
                                std::mem::take(&mut state.systems),
                                plan,
                            ),
                        ),
                        1 => QueuedRemovalFilter::Star(
                            super::step_job::LivingPlannedFilterJobV2::begin(
                                std::mem::take(&mut state.stars),
                                plan,
                            ),
                        ),
                        2 => QueuedRemovalFilter::World(
                            super::step_job::LivingPlannedFilterJobV2::begin(
                                std::mem::take(&mut state.worlds),
                                plan,
                            ),
                        ),
                        3 => QueuedRemovalFilter::Lane(
                            super::step_job::LivingPlannedFilterJobV2::begin(
                                std::mem::take(&mut state.lanes),
                                plan,
                            ),
                        ),
                        4 => QueuedRemovalFilter::Civilization(
                            super::step_job::LivingPlannedFilterJobV2::begin(
                                std::mem::take(&mut state.civilizations),
                                plan,
                            ),
                        ),
                        5 => QueuedRemovalFilter::Deposit(
                            super::step_job::LivingPlannedFilterJobV2::begin(
                                std::mem::take(&mut state.deposits),
                                plan,
                            ),
                        ),
                        6 => QueuedRemovalFilter::Facility(
                            super::step_job::LivingPlannedFilterJobV2::begin(
                                std::mem::take(&mut state.facilities),
                                plan,
                            ),
                        ),
                        7 => QueuedRemovalFilter::ConstructionJob(
                            super::step_job::LivingPlannedFilterJobV2::begin(
                                std::mem::take(&mut state.construction_jobs),
                                plan,
                            ),
                        ),
                        8 => QueuedRemovalFilter::HullJob(
                            super::step_job::LivingPlannedFilterJobV2::begin(
                                std::mem::take(&mut state.hull_jobs),
                                plan,
                            ),
                        ),
                        20 => QueuedRemovalFilter::Fleet(
                            super::step_job::LivingPlannedFilterJobV2::begin(
                                std::mem::take(&mut state.fleets),
                                plan,
                            ),
                        ),
                        10 => QueuedRemovalFilter::Route(
                            super::step_job::LivingPlannedFilterJobV2::begin(
                                std::mem::take(&mut state.routes),
                                plan,
                            ),
                        ),
                        11 => QueuedRemovalFilter::Shipment(
                            super::step_job::LivingPlannedFilterJobV2::begin(
                                std::mem::take(&mut state.shipments),
                                plan,
                            ),
                        ),
                        12 => QueuedRemovalFilter::Hazard(
                            super::step_job::LivingPlannedFilterJobV2::begin(
                                std::mem::take(&mut state.hazards),
                                plan,
                            ),
                        ),
                        13 => QueuedRemovalFilter::Colony(
                            super::step_job::LivingPlannedFilterJobV2::begin(
                                std::mem::take(&mut state.colonies),
                                plan,
                            ),
                        ),
                        14 => QueuedRemovalFilter::Observation(
                            super::step_job::LivingPlannedFilterJobV2::begin(
                                std::mem::take(&mut state.observations),
                                plan,
                            ),
                        ),
                        15 => QueuedRemovalFilter::Relation(
                            super::step_job::LivingPlannedFilterJobV2::begin(
                                std::mem::take(&mut state.relations),
                                plan,
                            ),
                        ),
                        16 => QueuedRemovalFilter::Agreement(
                            super::step_job::LivingPlannedFilterJobV2::begin(
                                std::mem::take(&mut state.agreements),
                                plan,
                            ),
                        ),
                        17 => QueuedRemovalFilter::War(
                            super::step_job::LivingPlannedFilterJobV2::begin(
                                std::mem::take(&mut state.wars),
                                plan,
                            ),
                        ),
                        18 => QueuedRemovalFilter::SettlementClaim(
                            super::step_job::LivingPlannedFilterJobV2::begin(
                                std::mem::take(&mut state.settlement_claims),
                                plan,
                            ),
                        ),
                        19 => QueuedRemovalFilter::Occupation(
                            super::step_job::LivingPlannedFilterJobV2::begin(
                                std::mem::take(&mut state.occupations),
                                plan,
                            ),
                        ),
                        _ => return Err(F::Invariant),
                    }
                });
                self.stage = S::Filter;
            }
            S::Filter => {
                let child = self.filter.as_mut().ok_or(F::Invariant)?;
                if !child.ready() {
                    return child.visit();
                }
                match child {
                    QueuedRemovalFilter::System(job) => {
                        state.systems = job.take_ready_result().map_err(|_| F::Invariant)?
                    }
                    QueuedRemovalFilter::Star(job) => {
                        state.stars = job.take_ready_result().map_err(|_| F::Invariant)?
                    }
                    QueuedRemovalFilter::World(job) => {
                        state.worlds = job.take_ready_result().map_err(|_| F::Invariant)?
                    }
                    QueuedRemovalFilter::Lane(job) => {
                        state.lanes = job.take_ready_result().map_err(|_| F::Invariant)?
                    }
                    QueuedRemovalFilter::Civilization(job) => {
                        state.civilizations = job.take_ready_result().map_err(|_| F::Invariant)?
                    }
                    QueuedRemovalFilter::Deposit(job) => {
                        state.deposits = job.take_ready_result().map_err(|_| F::Invariant)?
                    }
                    QueuedRemovalFilter::Facility(job) => {
                        state.facilities = job.take_ready_result().map_err(|_| F::Invariant)?
                    }
                    QueuedRemovalFilter::ConstructionJob(job) => {
                        state.construction_jobs =
                            job.take_ready_result().map_err(|_| F::Invariant)?
                    }
                    QueuedRemovalFilter::HullJob(job) => {
                        state.hull_jobs = job.take_ready_result().map_err(|_| F::Invariant)?
                    }
                    QueuedRemovalFilter::Fleet(job) => {
                        state.fleets = job.take_ready_result().map_err(|_| F::Invariant)?
                    }
                    QueuedRemovalFilter::Route(job) => {
                        state.routes = job.take_ready_result().map_err(|_| F::Invariant)?
                    }
                    QueuedRemovalFilter::Shipment(job) => {
                        state.shipments = job.take_ready_result().map_err(|_| F::Invariant)?
                    }
                    QueuedRemovalFilter::Hazard(job) => {
                        state.hazards = job.take_ready_result().map_err(|_| F::Invariant)?
                    }
                    QueuedRemovalFilter::Colony(job) => {
                        state.colonies = job.take_ready_result().map_err(|_| F::Invariant)?
                    }
                    QueuedRemovalFilter::Observation(job) => {
                        state.observations = job.take_ready_result().map_err(|_| F::Invariant)?
                    }
                    QueuedRemovalFilter::Relation(job) => {
                        state.relations = job.take_ready_result().map_err(|_| F::Invariant)?
                    }
                    QueuedRemovalFilter::Agreement(job) => {
                        state.agreements = job.take_ready_result().map_err(|_| F::Invariant)?
                    }
                    QueuedRemovalFilter::War(job) => {
                        state.wars = job.take_ready_result().map_err(|_| F::Invariant)?
                    }
                    QueuedRemovalFilter::SettlementClaim(job) => {
                        state.settlement_claims =
                            job.take_ready_result().map_err(|_| F::Invariant)?
                    }
                    QueuedRemovalFilter::Occupation(job) => {
                        state.occupations = job.take_ready_result().map_err(|_| F::Invariant)?
                    }
                    QueuedRemovalFilter::Hull(job) => {
                        state
                            .fleets
                            .record_at_mut(self.nested_fleet)
                            .ok_or(F::Invariant)?
                            .hulls = job.take_ready_result().map_err(|_| F::Invariant)?
                    }
                }
                self.retirement.start(self.filter.take());
                if self.nested {
                    self.nested = false;
                    self.nested_fleet += 1;
                } else if self.registry == 20 {
                    self.registry = 10;
                } else if self.registry == 19 {
                    self.registry = 21;
                } else {
                    self.registry += 1;
                }
                self.row = 0;
                self.hull = 0;
                self.stage = S::FilterPlan;
            }
            S::Ready => return Ok(false),
        }
        Ok(true)
    }
}

/// Finite causes of this concrete cursor. No recursive diagnostic is copied.
#[derive(Clone, Copy)]
enum QueuedFleetStructuralFault {
    Missing(&'static str),
    Referenced,
    Validation(LivingValidationErrorV2),
    KeyedOrder,
    PreparationCapacity,
    Invariant,
}
impl QueuedFleetStructuralFault {
    fn storage(error: super::step_job::LivingStorageJobErrorV2) -> Self {
        use super::step_job::LivingStorageJobErrorV2 as E;
        match error {
            E::CapacityOverflow => Self::PreparationCapacity,
            E::KeyedOrder => Self::KeyedOrder,
            E::Budget | E::Cancelled | E::NotReady => Self::Invariant,
        }
    }
    fn insertion(error: super::step_job::InsertionFault) -> Self {
        use super::step_job::InsertionFault as E;
        match error {
            E::Duplicate => Self::Validation(LivingValidationErrorV2::DuplicateIdentity {
                context: "creator batch",
            }),
            E::KeyedOrder => Self::KeyedOrder,
            E::Capacity => Self::PreparationCapacity,
            E::Preparation => Self::Invariant,
        }
    }
    fn public(self, sequence: u64, operation: usize) -> super::step_job::LivingLifecycleJobErrorV2 {
        use super::step_job::LivingLifecycleJobErrorV2 as E;
        let reason = match self {
            Self::Missing(reference) => LivingCommandRejectionV2::MissingTarget {
                operation,
                reference,
            },
            Self::Referenced => LivingCommandRejectionV2::ReferencedTarget,
            Self::Validation(reason) => LivingCommandRejectionV2::Projection(reason),
            Self::KeyedOrder => {
                LivingCommandRejectionV2::Projection(LivingWireErrorV2::KeyedOrder.into())
            }
            Self::PreparationCapacity => return E::PreparationCapacity,
            Self::Invariant => return E::Fault(LivingDeterministicFaultV2::Invariant),
        };
        E::Fault(LivingDeterministicFaultV2::Replay {
            accepted_sequence: sequence,
            reason: Box::new(reason),
        })
    }
}
#[derive(Clone, Copy, PartialEq, Eq)]
enum QueuedFleetStructuralStage {
    Resolve,
    Reservations { target: bool, index: usize },
    SourceGuard,
    Colocation,
    SelectionPrepare,
    SelectionRead,
    SelectionEmit,
    Membership { index: usize },
    Capacity { index: usize, reserved: usize },
    SelectedExists { index: usize },
    DetachSource,
    DetachTarget,
    HullPrepare { moved: bool },
    Partition,
    Merge,
    Assemble,
    Identity,
    PlanRemoval,
    FilterStart,
    Filter,
    ReplaceStart,
    Replace,
    InsertStart,
    Insert,
    Ready,
}
/// Original owners remain in this variant through all nested storage/filter/
/// replacement work. The creator never replaces the variant with a child job.
struct QueuedFleetStructuralWork {
    stage: QueuedFleetStructuralStage,
    split: bool,
    source: Option<LivingEntityIdV2>,
    target: Option<LivingEntityIdV2>,
    source_index: usize,
    target_index: usize,
    source_count: usize,
    target_count: usize,
    selected_index: usize,
    selected: Vec<LivingEntityIdV2>,
    pending_id: Option<LivingEntityIdV2>,
    selection_storage: Option<super::step_job::LivingVecStorageJobV2<'static, LivingEntityIdV2>>,
    source_parent: Option<super::model::LivingFleetV2>,
    target_parent: Option<super::model::LivingFleetV2>,
    source_hulls: Option<std::vec::IntoIter<super::model::LivingHullV2>>,
    target_hulls: Option<std::vec::IntoIter<super::model::LivingHullV2>>,
    pending_source: Option<super::model::LivingHullV2>,
    pending_target: Option<super::model::LivingHullV2>,
    kept: LivingSortedVecV2<super::model::LivingHullV2>,
    moved: LivingSortedVecV2<super::model::LivingHullV2>,
    hull_storage:
        Option<super::step_job::LivingSortedStorageJobV2<'static, super::model::LivingHullV2>>,
    source_order: Option<super::model::LivingFleetOrderV2>,
    target_order: Option<super::model::LivingFleetOrderV2>,
    plan: std::collections::BTreeSet<usize>,
    filter: Option<super::step_job::LivingPlannedFilterJobV2<super::model::LivingFleetV2>>,
    replacement: Option<super::step_job::LivingSortedInsertionJobV2<super::model::LivingFleetV2>>,
    retirement: super::step_job::PhaseRetirementLane<'static>,
    fault: Option<QueuedFleetStructuralFault>,
}
impl QueuedFleetStructuralWork {
    fn begin(split: bool) -> Self {
        Self {
            stage: QueuedFleetStructuralStage::Resolve,
            split,
            source: None,
            target: None,
            source_index: 0,
            target_index: 0,
            source_count: 0,
            target_count: 0,
            selected_index: 0,
            selected: Vec::new(),
            pending_id: None,
            selection_storage: None,
            source_parent: None,
            target_parent: None,
            source_hulls: None,
            target_hulls: None,
            pending_source: None,
            pending_target: None,
            kept: LivingSortedVecV2::default(),
            moved: LivingSortedVecV2::default(),
            hull_storage: None,
            source_order: None,
            target_order: None,
            plan: std::collections::BTreeSet::new(),
            filter: None,
            replacement: None,
            retirement: super::step_job::PhaseRetirementLane::default(),
            fault: None,
        }
    }
    fn resolve(
        target: &LivingCommandTargetV2,
        locals: &std::collections::BTreeMap<u16, LivingEntityIdV2>,
    ) -> Result<LivingEntityIdV2, QueuedFleetStructuralFault> {
        match target {
            LivingCommandTargetV2::Existing { id } => Ok(*id),
            LivingCommandTargetV2::Local { local } => locals
                .get(local)
                .copied()
                .ok_or(QueuedFleetStructuralFault::Invariant),
        }
    }
    fn guard(
        state: &LivingGalaxyStateV2,
        id: LivingEntityIdV2,
    ) -> Result<usize, QueuedFleetStructuralFault> {
        let index = state
            .fleets
            .as_slice()
            .binary_search_by_key(&id, |row| row.id)
            .map_err(|_| QueuedFleetStructuralFault::Missing("fleet"))?;
        if !matches!(
            state.fleets.as_slice()[index].location,
            super::model::LivingFleetLocationV2::Docked { .. }
        ) {
            return Err(QueuedFleetStructuralFault::Referenced);
        }
        Ok(index)
    }
    fn detach(
        state: &mut LivingGalaxyStateV2,
        index: usize,
        id: LivingEntityIdV2,
    ) -> Result<super::model::LivingFleetV2, QueuedFleetStructuralFault> {
        let row = state
            .fleets
            .record_at_mut(index)
            .filter(|row| row.id == id)
            .ok_or(QueuedFleetStructuralFault::Missing("fleet"))?;
        let shell = super::model::LivingFleetV2 {
            id: row.id,
            owner: row.owner,
            location: row.location,
            order: None,
            hulls: LivingSortedVecV2::default(),
        };
        Ok(std::mem::replace(row, shell))
    }
    fn ready(&self) -> bool {
        self.stage == QueuedFleetStructuralStage::Ready
            && self.fault.is_none()
            && self.retirement.is_empty()
    }
    fn visit(
        &mut self,
        state: &mut LivingGalaxyStateV2,
        operation: &LivingCreatorOperationV2,
        locals: &std::collections::BTreeMap<u16, LivingEntityIdV2>,
    ) -> Result<bool, QueuedFleetStructuralFault> {
        if let Some(fault) = self.fault {
            return Err(fault);
        }
        if self.retirement.visit() {
            return Ok(true);
        }
        let result = self.visit_record(state, operation, locals);
        if let Err(fault) = result {
            self.fault = Some(fault);
        }
        result
    }
    fn visit_record(
        &mut self,
        state: &mut LivingGalaxyStateV2,
        operation: &LivingCreatorOperationV2,
        locals: &std::collections::BTreeMap<u16, LivingEntityIdV2>,
    ) -> Result<bool, QueuedFleetStructuralFault> {
        use super::model::{LIVING_MAX_FLEETS_V2, LIVING_MAX_HULLS_PER_FLEET_V2};
        use super::step_job::{
            LivingSortedInsertionJobV2 as Insert, LivingSortedStorageJobV2 as Sorted,
            LivingVecStorageJobV2 as Vector,
        };
        use QueuedFleetStructuralFault as F;
        use QueuedFleetStructuralStage as S;
        match self.stage {
            S::Resolve => {
                match operation {
                    LivingCreatorOperationV2::SplitFleet { source, .. } if self.split => {
                        let id = Self::resolve(source, locals)?;
                        self.source = Some(id);
                        self.source_index = Self::guard(state, id)?;
                    }
                    LivingCreatorOperationV2::MergeFleets { target, source } if !self.split => {
                        let target = Self::resolve(target, locals)?;
                        let source = Self::resolve(source, locals)?;
                        self.source = Some(source);
                        self.target = Some(target);
                        if source == target {
                            return Err(F::Validation(LivingValidationErrorV2::Range {
                                field: "merge fleet identity",
                            }));
                        }
                        self.target_index = Self::guard(state, target)?;
                    }
                    _ => return Err(F::Invariant),
                }
                self.stage = S::Reservations {
                    target: !self.split,
                    index: 0,
                };
            }
            S::Reservations { target, index } => {
                if let Some(job) = state.hull_jobs.as_slice().get(index) {
                    if job.target_fleet == if target { self.target } else { self.source } {
                        return Err(F::Referenced);
                    }
                    self.stage = S::Reservations {
                        target,
                        index: index + 1,
                    };
                } else {
                    self.stage = if target {
                        S::SourceGuard
                    } else if self.split {
                        S::SelectionPrepare
                    } else {
                        S::Colocation
                    };
                    return Ok(false);
                }
            }
            S::SourceGuard => {
                self.source_index = Self::guard(state, self.source.ok_or(F::Invariant)?)?;
                self.stage = S::Reservations {
                    target: false,
                    index: 0,
                };
            }
            S::Colocation => {
                let source = &state.fleets.as_slice()[self.source_index];
                let target = &state.fleets.as_slice()[self.target_index];
                if source.owner != target.owner || source.location != target.location {
                    return Err(F::Validation(LivingValidationErrorV2::Range {
                        field: "merge fleet colocation",
                    }));
                }
                self.source_count = source.hulls.as_slice().len();
                self.target_count = target.hulls.as_slice().len();
                let found = self
                    .source_count
                    .checked_add(self.target_count)
                    .ok_or(F::PreparationCapacity)?;
                if found > LIVING_MAX_HULLS_PER_FLEET_V2 {
                    return Err(F::Validation(LivingValidationErrorV2::Capacity {
                        collection: "fleet hulls",
                        limit: LIVING_MAX_HULLS_PER_FLEET_V2,
                        found,
                    }));
                }
                self.stage = S::DetachSource;
            }
            S::SelectionPrepare => {
                let LivingCreatorOperationV2::SplitFleet { hulls, .. } = operation else {
                    return Err(F::Invariant);
                };
                let Some(child) = &mut self.selection_storage else {
                    self.selection_storage = Some(Vector::begin_empty(hulls.len()));
                    return Ok(true);
                };
                if !child.ready() {
                    child.poll(1).map_err(F::storage)?;
                    return Ok(child.last_poll_visits() != 0);
                }
                self.selected = child.take_ready_result().map_err(F::storage)?;
                self.retirement.start(self.selection_storage.take());
                self.source_count = state.fleets.as_slice()[self.source_index]
                    .hulls
                    .as_slice()
                    .len();
                self.stage = S::SelectionRead;
            }
            S::SelectionRead => {
                let LivingCreatorOperationV2::SplitFleet { hulls, .. } = operation else {
                    return Err(F::Invariant);
                };
                if let Some(target) = hulls.get(self.selected_index) {
                    self.pending_id = Some(Self::resolve(target, locals)?);
                    self.stage = S::SelectionEmit;
                } else {
                    self.stage = S::Membership { index: 0 };
                    return Ok(false);
                }
            }
            S::SelectionEmit => {
                Vector::commit_one(&mut self.selected, &mut self.pending_id).map_err(F::storage)?;
                self.selected_index += 1;
                self.stage = S::SelectionRead;
            }
            S::Membership { index } => {
                if self.selected.is_empty()
                    || self.selected.len() >= self.source_count
                    || (index > 0 && self.selected[index - 1] >= self.selected[index])
                {
                    return Err(F::Validation(LivingValidationErrorV2::Range {
                        field: "split hull membership",
                    }));
                }
                self.stage = if index + 1 < self.selected.len() {
                    S::Membership { index: index + 1 }
                } else {
                    S::Capacity {
                        index: 0,
                        reserved: 0,
                    }
                };
            }
            S::Capacity { index, reserved } => {
                if let Some(job) = state.hull_jobs.as_slice().get(index) {
                    self.stage = S::Capacity {
                        index: index + 1,
                        reserved: reserved + usize::from(job.target_fleet.is_none()),
                    };
                } else {
                    if state.fleets.as_slice().len().saturating_add(reserved)
                        >= LIVING_MAX_FLEETS_V2
                    {
                        return Err(F::Validation(LivingValidationErrorV2::Capacity {
                            collection: "fleets",
                            limit: LIVING_MAX_FLEETS_V2,
                            found: LIVING_MAX_FLEETS_V2 + 1,
                        }));
                    }
                    self.stage = S::SelectedExists { index: 0 };
                }
            }
            S::SelectedExists { index } => {
                let id = self.selected[index];
                if state.fleets.as_slice()[self.source_index]
                    .hulls
                    .as_slice()
                    .binary_search_by_key(&id, |row| row.id)
                    .is_err()
                {
                    return Err(F::Missing("split hull"));
                }
                self.stage = if index + 1 < self.selected.len() {
                    S::SelectedExists { index: index + 1 }
                } else {
                    S::DetachSource
                };
            }
            S::DetachSource => {
                let mut parent =
                    Self::detach(state, self.source_index, self.source.ok_or(F::Invariant)?)?;
                self.source_hulls = Some(std::mem::take(&mut parent.hulls).into_vec().into_iter());
                self.source_parent = Some(parent);
                self.stage = if self.split {
                    S::HullPrepare { moved: false }
                } else {
                    S::DetachTarget
                };
            }
            S::DetachTarget => {
                let mut parent =
                    Self::detach(state, self.target_index, self.target.ok_or(F::Invariant)?)?;
                self.target_hulls = Some(std::mem::take(&mut parent.hulls).into_vec().into_iter());
                self.target_parent = Some(parent);
                self.stage = S::HullPrepare { moved: false };
            }
            S::HullPrepare { moved } => {
                let Some(child) = &mut self.hull_storage else {
                    let count = if self.split {
                        if moved {
                            self.selected.len()
                        } else {
                            self.source_count - self.selected.len()
                        }
                    } else {
                        self.source_count + self.target_count
                    };
                    self.hull_storage = Some(Sorted::begin_empty(count));
                    return Ok(true);
                };
                if !child.ready() {
                    child.poll(1).map_err(F::storage)?;
                    return Ok(child.last_poll_visits() != 0);
                }
                let output = child.take_ready_result().map_err(F::storage)?;
                if moved {
                    self.moved = output;
                } else {
                    self.kept = output;
                }
                self.retirement.start(self.hull_storage.take());
                self.stage = if self.split && !moved {
                    S::HullPrepare { moved: true }
                } else if self.split {
                    S::Partition
                } else {
                    S::Merge
                };
            }
            S::Partition => {
                if let Some(hull) = &self.pending_source {
                    let output = if self.selected.binary_search(&hull.id).is_ok() {
                        &mut self.moved
                    } else {
                        &mut self.kept
                    };
                    Sorted::commit_one(output, &mut self.pending_source).map_err(F::storage)?;
                } else if !self
                    .source_hulls
                    .as_ref()
                    .ok_or(F::Invariant)?
                    .as_slice()
                    .is_empty()
                {
                    self.pending_source = self.source_hulls.as_mut().unwrap().next();
                } else {
                    self.stage = S::Assemble;
                    return Ok(false);
                }
            }
            S::Merge => {
                if self.pending_target.is_none()
                    && !self
                        .target_hulls
                        .as_ref()
                        .ok_or(F::Invariant)?
                        .as_slice()
                        .is_empty()
                {
                    self.pending_target = self.target_hulls.as_mut().unwrap().next();
                } else if self.pending_source.is_none()
                    && !self
                        .source_hulls
                        .as_ref()
                        .ok_or(F::Invariant)?
                        .as_slice()
                        .is_empty()
                {
                    self.pending_source = self.source_hulls.as_mut().unwrap().next();
                } else {
                    let target_first = match (&self.pending_target, &self.pending_source) {
                        (Some(target), Some(source)) => target.id <= source.id,
                        (Some(_), None) => true,
                        (None, Some(_)) => false,
                        (None, None) => {
                            self.stage = S::Assemble;
                            return Ok(false);
                        }
                    };
                    let pending = if target_first {
                        &mut self.pending_target
                    } else {
                        &mut self.pending_source
                    };
                    Sorted::commit_one(&mut self.kept, pending).map_err(F::storage)?;
                }
            }
            S::Assemble => {
                let source = self.source_parent.as_mut().ok_or(F::Invariant)?;
                if self.split {
                    source.hulls = std::mem::take(&mut self.kept);
                    self.source_order = source.order.take();
                    let LivingCreatorOperationV2::SplitFleet { local, .. } = operation else {
                        return Err(F::Invariant);
                    };
                    let id =
                        Self::resolve(&LivingCommandTargetV2::Local { local: *local }, locals)?;
                    self.target = Some(id);
                    self.target_parent = Some(super::model::LivingFleetV2 {
                        id,
                        owner: source.owner,
                        location: source.location,
                        order: None,
                        hulls: std::mem::take(&mut self.moved),
                    });
                    self.stage = S::Identity;
                } else {
                    let target = self.target_parent.as_mut().ok_or(F::Invariant)?;
                    target.hulls = std::mem::take(&mut self.kept);
                    self.target_order = target.order.take();
                    self.source_order = source.order.take();
                    self.stage = S::PlanRemoval;
                }
            }
            S::Identity => {
                if state
                    .fleets
                    .as_slice()
                    .binary_search_by_key(&self.target.ok_or(F::Invariant)?, |row| row.id)
                    .is_ok()
                {
                    return Err(F::Validation(LivingValidationErrorV2::Range {
                        field: "split fleet identity",
                    }));
                }
                self.stage = S::ReplaceStart;
            }
            S::PlanRemoval => {
                self.plan.insert(self.source_index);
                self.stage = S::FilterStart;
            }
            S::FilterStart => {
                self.filter = Some(super::step_job::LivingPlannedFilterJobV2::begin(
                    std::mem::take(&mut state.fleets),
                    std::mem::take(&mut self.plan),
                ));
                self.stage = S::Filter;
            }
            S::Filter => {
                let child = self.filter.as_mut().ok_or(F::Invariant)?;
                if !child.ready() {
                    return child.visit().map_err(|_| {
                        F::insertion(
                            child
                                .failure()
                                .expect("concrete failed filter retains its cause"),
                        )
                    });
                }
                state.fleets = child.take_ready_result().map_err(|error| match error {
                    super::step_job::LivingLifecycleJobErrorV2::NotReady => F::Invariant,
                    _ => unreachable!(
                        "ready concrete child can only refuse an already delivered output"
                    ),
                })?;
                self.retirement.start(self.filter.take());
                self.stage = S::ReplaceStart;
            }
            S::ReplaceStart => {
                let id = if self.split { self.source } else { self.target }.ok_or(F::Invariant)?;
                // Source removal shifts target positions. Always re-find by identity.
                if state
                    .fleets
                    .as_slice()
                    .binary_search_by_key(&id, |row| row.id)
                    .is_err()
                {
                    return Err(F::Missing("fleet"));
                }
                let parent = if self.split {
                    self.source_parent.take()
                } else {
                    self.target_parent.take()
                }
                .ok_or(F::Invariant)?;
                self.replacement = Some(Insert::begin_replacement(
                    std::mem::take(&mut state.fleets),
                    parent,
                ));
                self.stage = S::Replace;
            }
            S::Replace | S::Insert => {
                let child = self.replacement.as_mut().ok_or(F::Invariant)?;
                if !child.ready() {
                    return child.visit().map_err(|_| {
                        F::insertion(
                            child
                                .failure()
                                .expect("concrete failed insertion retains its cause"),
                        )
                    });
                }
                state.fleets = child.take_ready_result().map_err(|error| match error {
                    super::step_job::LivingLifecycleJobErrorV2::NotReady => F::Invariant,
                    _ => unreachable!(
                        "ready concrete child can only refuse an already delivered output"
                    ),
                })?;
                self.retirement.start(self.replacement.take());
                self.stage = if self.split && self.stage == S::Replace {
                    S::InsertStart
                } else {
                    S::Ready
                };
            }
            S::InsertStart => {
                let row = self.target_parent.take().ok_or(F::Invariant)?;
                self.replacement = Some(Insert::begin(std::mem::take(&mut state.fleets), row));
                self.stage = S::Insert;
            }
            S::Ready => return Ok(false),
        }
        Ok(true)
    }
}

enum QueuedDiplomacyFilter {
    Agreement(super::step_job::LivingPlannedFilterJobV2<super::model::LivingAgreementV2>),
    War(super::step_job::LivingPlannedFilterJobV2<super::model::LivingWarV2>),
    Occupation(super::step_job::LivingPlannedFilterJobV2<super::model::LivingOccupationV2>),
}
/// Operation-local ownership; the original creator, state and accepted command
/// remain anchored throughout planning, filtering and standing-order replacement.
#[derive(Default)]
struct QueuedDiplomacyWork {
    pair: Option<(LivingEntityIdV2, LivingEntityIdV2)>,
    plan: std::collections::BTreeSet<usize>,
    pending_plan: Option<usize>,
    occupation: Option<super::model::LivingOccupationV2>,
    filter: Option<QueuedDiplomacyFilter>,
    fleet: Option<LivingEntityIdV2>,
    fleet_index: usize,
    path_storage: Option<super::step_job::LivingVecStorageJobV2<'static, LivingEntityIdV2>>,
    pending_path: Option<LivingEntityIdV2>,
    order: Option<super::model::LivingFleetOrderV2>,
    retired_order: Option<super::model::LivingFleetOrderV2>,
}
enum QueuedCancellationFilter {
    Construction(super::step_job::LivingPlannedFilterJobV2<super::model::LivingConstructionJobV2>),
    Hull(super::step_job::LivingPlannedFilterJobV2<super::model::LivingHullJobV2>),
    Hazard(super::step_job::LivingPlannedFilterJobV2<super::model::LivingHazardV2>),
}
#[derive(Clone, Copy)]
enum QueuedCancellationKind {
    Construction,
    Hull,
    Hazard,
}
/// The operation owns its exact original-position plan and its replacement row.
/// Refund metadata is fixed-size; effects belong to the revision, not this owner.
#[derive(Default)]
struct QueuedCancellationWork {
    kind: Option<QueuedCancellationKind>,
    position: usize,
    refund: Option<(LivingEntityIdV2, u32)>,
    plan: std::collections::BTreeSet<usize>,
    filter: Option<QueuedCancellationFilter>,
    route: Option<super::model::LivingFreightRouteV2>,
}
struct QueuedCreatorEffects {
    values: Vec<LivingEventKindV2>,
    storage: Option<super::step_job::LivingVecStorageJobV2<'static, LivingEventKindV2>>,
    pending: Option<LivingEventKindV2>,
    input: Option<std::vec::IntoIter<LivingEventKindV2>>,
    suffix: Vec<LivingEventKindV2>,
    suffix_input: Option<std::vec::IntoIter<LivingEventKindV2>>,
    resume: QueuedCreatorStage,
}
impl Default for QueuedCreatorEffects {
    fn default() -> Self {
        Self {
            values: Vec::new(),
            storage: None,
            pending: None,
            input: None,
            suffix: Vec::new(),
            suffix_input: None,
            resume: QueuedCreatorStage::Operation,
        }
    }
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
    diplomacy_work: Option<QueuedDiplomacyWork>,
    cancellation_work: Option<QueuedCancellationWork>,
    effect_work: Option<QueuedCreatorEffects>,
    colony_operation: Option<QueuedColonyOperation>,
    colony_create: Option<(LivingEntityIdV2, LivingEntityIdV2, [LivingTickV2; 3])>,
    colony_removal: Option<super::step_job::LivingColonyRemovalJobV2>,
    colony_changed: bool,
    colony_owners: std::collections::BTreeSet<LivingEntityIdV2>,
    recovery_storage:
        Option<super::step_job::LivingVecStorageJobV2<'static, LivingCreatorRecoveryCandidateV2>>,
    pending_recovery: Option<LivingCreatorRecoveryCandidateV2>,
    hull_parent: Option<super::model::LivingFleetV2>,
    hull_output: Option<LivingSortedVecV2<super::model::LivingHullV2>>,
    hull_index: usize,
    hull_id: Option<LivingEntityIdV2>,
    append: Option<super::receipt::LivingPendingEventAppendJobV2>,
    retirement: super::step_job::PhaseRetirementLane<'static>,
    last_poll_visits: usize,
    delivered: bool,
}
impl LivingQueuedCreatorJobV2 {
    fn structural_failure(&self) -> Option<super::step_job::LivingLifecycleJobErrorV2> {
        let sequence = self
            .queue
            .get(self.revision_index)?
            .accepted
            .accepted_sequence;
        match &self.insertion {
            Some(QueuedCreatorInsertion::FleetStructural(work)) => work
                .fault
                .map(|cause| cause.public(sequence, self.operation_index)),
            Some(QueuedCreatorInsertion::Removal(work)) => work
                .fault
                .map(|cause| cause.public(sequence, self.operation_index)),
            _ => None,
        }
    }
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
            diplomacy_work: None,
            cancellation_work: None,
            effect_work: None,
            colony_operation: None,
            colony_create: None,
            colony_removal: None,
            colony_changed: false,
            colony_owners: std::collections::BTreeSet::new(),
            recovery_storage: None,
            pending_recovery: None,
            hull_parent: None,
            hull_output: None,
            hull_index: 0,
            hull_id: None,
            append: None,
            retirement: super::step_job::PhaseRetirementLane::default(),
            last_poll_visits: 0,
        }
    }
    fn poll(
        &mut self,
        budget: usize,
    ) -> Result<LivingStepProgressV2, super::step_job::LivingLifecycleJobErrorV2> {
        self.last_poll_visits = 0;
        if budget == 0 || budget > 1024 {
            return Err(super::step_job::LivingLifecycleJobErrorV2::Budget);
        }
        if let Some(fault) = self.structural_failure() {
            return Err(fault);
        }
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
                        super::step_job::LivingLifecycleJobErrorV2::Fault(fault) => fault,
                        other => return Err(other),
                    };
                    let fault = match fault {
                        LivingDeterministicFaultV2::Validation(reason) => {
                            self.replay_fault(LivingCommandRejectionV2::Projection(reason))
                        }
                        LivingDeterministicFaultV2::Wire(reason) => {
                            self.replay_fault(LivingCommandRejectionV2::Projection(reason.into()))
                        }
                        other => other,
                    };
                    return Err(fault.into());
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
    fn append_error(
        error: super::receipt::LivingPendingEventAppendErrorV2,
    ) -> super::step_job::LivingLifecycleJobErrorV2 {
        use super::receipt::LivingPendingEventAppendErrorV2 as E;
        use super::step_job::{LivingLifecycleJobErrorV2 as J, LivingStorageJobErrorV2 as S};
        match error {
            E::Storage(S::CapacityOverflow) | E::Capacity => J::PreparationCapacity,
            E::TooManyEvents => J::Fault(error.receipt_error().expect("receipt error").into()),
            E::Storage(S::KeyedOrder) => J::Fault(LivingWireErrorV2::KeyedOrder.into()),
            E::Json => J::Fault(LivingWireErrorV2::Json.into()),
            E::Budget | E::Cancelled | E::NotReady | E::Facts | E::Storage(_) => {
                J::Fault(LivingDeterministicFaultV2::Invariant)
            }
        }
    }
    fn recovery_storage_error(
        error: super::step_job::LivingStorageJobErrorV2,
    ) -> super::step_job::LivingLifecycleJobErrorV2 {
        use super::step_job::{LivingLifecycleJobErrorV2 as J, LivingStorageJobErrorV2 as S};
        match error {
            S::CapacityOverflow => J::PreparationCapacity,
            S::KeyedOrder => J::Fault(LivingWireErrorV2::KeyedOrder.into()),
            S::Budget | S::Cancelled | S::NotReady => {
                J::Fault(LivingDeterministicFaultV2::Invariant)
            }
        }
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
    fn resolve_pair(
        &self,
        a: &LivingCommandTargetV2,
        b: &LivingCommandTargetV2,
    ) -> Result<(LivingEntityIdV2, LivingEntityIdV2), LivingDeterministicFaultV2> {
        let a = self.resolve(a)?;
        let b = self.resolve(b)?;
        Ok(if a < b { (a, b) } else { (b, a) })
    }
    fn check_diplomacy_participants(
        &self,
        pair: (LivingEntityIdV2, LivingEntityIdV2),
    ) -> Result<(), LivingDeterministicFaultV2> {
        if self
            .state
            .civilizations
            .as_slice()
            .binary_search_by_key(&pair.0, |row| row.id)
            .is_err()
            || self
                .state
                .civilizations
                .as_slice()
                .binary_search_by_key(&pair.1, |row| row.id)
                .is_err()
        {
            return Err(self.replay_fault(LivingCommandRejectionV2::MissingTarget {
                operation: self.operation_index,
                reference: "civilization",
            }));
        }
        Ok(())
    }
    fn replay_fault(&self, reason: LivingCommandRejectionV2) -> LivingDeterministicFaultV2 {
        LivingDeterministicFaultV2::Replay {
            accepted_sequence: self.queue[self.revision_index].accepted.accepted_sequence,
            reason: Box::new(reason),
        }
    }
    // Metadata edits deliberately use missing -> Docked -> reservation order.
    // SetFleetOrder has a different guard and must not share this entry point.
    fn editable_fleet(&self, id: LivingEntityIdV2) -> Result<usize, LivingDeterministicFaultV2> {
        let index = self
            .state
            .fleets
            .as_slice()
            .binary_search_by_key(&id, |row| row.id)
            .map_err(|_| {
                self.replay_fault(LivingCommandRejectionV2::MissingTarget {
                    operation: self.operation_index,
                    reference: "fleet",
                })
            })?;
        if !matches!(
            self.state.fleets.as_slice()[index].location,
            super::model::LivingFleetLocationV2::Docked { .. }
        ) {
            return Err(self.replay_fault(LivingCommandRejectionV2::ReferencedTarget));
        }
        Ok(index)
    }
    fn prepare_effect(&mut self, kind: LivingEventKindV2, resume: QueuedCreatorStage) {
        let work = self.effect_work.get_or_insert_with(Default::default);
        debug_assert!(work.pending.is_none() && work.storage.is_none());
        work.pending = Some(kind);
        work.storage = Some(super::step_job::LivingVecStorageJobV2::begin(
            std::mem::take(&mut work.values),
            1,
        ));
        work.resume = resume;
        self.stage = QueuedCreatorStage::EffectPrepare;
    }
    fn route_permission_fault(&self) -> LivingDeterministicFaultV2 {
        self.replay_fault(LivingCommandRejectionV2::Projection(
            LivingValidationErrorV2::Range {
                field: "route permission",
            },
        ))
    }
    fn visit(&mut self) -> Result<bool, super::step_job::LivingLifecycleJobErrorV2> {
        use super::model::*;
        use QueuedCreatorStage::*;
        match self.stage {
            Revision => {
                if self.revision_index == self.queue.len() {
                    self.stage = Ready;
                    return Ok(false);
                }
                self.operation_index = 0;
                self.colony_changed = false;
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
                    if self.colony_changed {
                        self.recovery_storage =
                            Some(super::step_job::LivingVecStorageJobV2::begin(
                                std::mem::take(&mut self.events.creator_recoveries),
                                self.dormant.len(),
                            ));
                        self.stage = RecoveryPrepare;
                        return Ok(true);
                    }
                    self.stage = EmitCreator;
                    return Ok(false);
                };
                use super::step_job::LivingSortedInsertionJobV2 as Insert;
                let accepted_sequence = self.queue[self.revision_index].accepted.accepted_sequence;
                let command_fault = |reason| LivingDeterministicFaultV2::Replay {
                    accepted_sequence,
                    reason: Box::new(reason),
                };
                let insertion = match operation {
                    LivingCreatorOperationV2::CancelConstructionJob { target }
                    | LivingCreatorOperationV2::CancelHullJob { target }
                    | LivingCreatorOperationV2::CancelHazard { target } => {
                        let target = self.resolve(target)?;
                        let (kind, position, refund) = match operation {
                            LivingCreatorOperationV2::CancelConstructionJob { .. } => {
                                let position = self
                                    .state
                                    .construction_jobs
                                    .as_slice()
                                    .binary_search_by_key(&target, |row| row.id)
                                    .map_err(|_| {
                                        command_fault(LivingCommandRejectionV2::MissingTarget {
                                            operation: self.operation_index,
                                            reference: "construction job",
                                        })
                                    })?;
                                let row = &self.state.construction_jobs.as_slice()[position];
                                (
                                    QueuedCancellationKind::Construction,
                                    position,
                                    Some((row.world, row.paid_alloy)),
                                )
                            }
                            LivingCreatorOperationV2::CancelHullJob { .. } => {
                                let position = self
                                    .state
                                    .hull_jobs
                                    .as_slice()
                                    .binary_search_by_key(&target, |row| row.id)
                                    .map_err(|_| {
                                        command_fault(LivingCommandRejectionV2::MissingTarget {
                                            operation: self.operation_index,
                                            reference: "hull job",
                                        })
                                    })?;
                                let row = &self.state.hull_jobs.as_slice()[position];
                                (
                                    QueuedCancellationKind::Hull,
                                    position,
                                    Some((row.world, row.paid_alloy)),
                                )
                            }
                            _ => {
                                let position = self
                                    .state
                                    .hazards
                                    .as_slice()
                                    .binary_search_by_key(&target, |row| row.id)
                                    .map_err(|_| {
                                        command_fault(LivingCommandRejectionV2::MissingTarget {
                                            operation: self.operation_index,
                                            reference: "hazard",
                                        })
                                    })?;
                                (QueuedCancellationKind::Hazard, position, None)
                            }
                        };
                        self.cancellation_work = Some(QueuedCancellationWork {
                            kind: Some(kind),
                            position,
                            refund,
                            ..Default::default()
                        });
                        self.stage = if refund.is_some() {
                            CancelRefund
                        } else {
                            CancelPlan
                        };
                        return Ok(true);
                    }
                    LivingCreatorOperationV2::EditRoute { target, .. }
                    | LivingCreatorOperationV2::SetRouteSuspended { target, .. } => {
                        let id = self.resolve(target)?;
                        let position = self
                            .state
                            .routes
                            .as_slice()
                            .binary_search_by_key(&id, |row| row.id)
                            .map_err(|_| {
                                command_fault(LivingCommandRejectionV2::MissingTarget {
                                    operation: self.operation_index,
                                    reference: "route",
                                })
                            })?;
                        let mut route = self.state.routes.as_slice()[position].clone();
                        let suspend = match operation {
                            LivingCreatorOperationV2::EditRoute {
                                kind,
                                source_world,
                                destination_world,
                                resource,
                                batch_size,
                                cadence_ticks,
                                source_reserve,
                                source_owner,
                                receiver_owner,
                                ..
                            } => {
                                if *batch_size == 0
                                    || *batch_size > LIVING_ROUTE_BATCH_LIMIT_V2
                                    || *cadence_ticks != LIVING_ROUTE_CADENCE_TICKS_V2
                                    || *source_reserve < LIVING_ROUTE_RESERVE_FLOOR_V2
                                {
                                    return Err(command_fault(
                                        LivingCommandRejectionV2::Projection(
                                            LivingValidationErrorV2::Range {
                                                field: "route bounds",
                                            },
                                        ),
                                    )
                                    .into());
                                }
                                route.kind = *kind;
                                route.source_world = self.resolve(source_world)?;
                                route.destination_world = self.resolve(destination_world)?;
                                route.resource = *resource;
                                route.batch_size = *batch_size;
                                route.cadence_ticks = *cadence_ticks;
                                route.source_reserve = *source_reserve;
                                route.source_owner = self.resolve(source_owner)?;
                                route.receiver_owner = self.resolve(receiver_owner)?;
                                false
                            }
                            LivingCreatorOperationV2::SetRouteSuspended { suspended, .. } => {
                                *suspended
                            }
                            _ => unreachable!(),
                        };
                        if suspend {
                            route.suspended = true;
                        }
                        self.cancellation_work = Some(QueuedCancellationWork {
                            route: Some(route),
                            ..Default::default()
                        });
                        self.stage = if suspend { RouteInstall } else { RouteSource };
                        return Ok(true);
                    }
                    LivingCreatorOperationV2::SetRelationBase {
                        from,
                        to,
                        base_disposition,
                    } => {
                        let from = self.resolve(from)?;
                        let to = self.resolve(to)?;
                        let reasons = self
                            .state
                            .relations
                            .as_slice()
                            .binary_search_by_key(&(from, to), LivingKeyedV2::living_key)
                            .ok()
                            .map(|i| self.state.relations.as_slice()[i].reasons)
                            .unwrap_or_default();
                        Some(QueuedCreatorInsertion::Relation(Insert::begin_replacement(
                            std::mem::take(&mut self.state.relations),
                            LivingRelationV2 {
                                from,
                                to,
                                base_disposition: *base_disposition,
                                reasons,
                            },
                        )))
                    }
                    LivingCreatorOperationV2::SetAgreement {
                        kind,
                        participant_a,
                        participant_b,
                        start_tick,
                        end_tick,
                    } => {
                        let pair = self.resolve_pair(participant_a, participant_b)?;
                        Some(QueuedCreatorInsertion::Agreement(
                            Insert::begin_replacement(
                                std::mem::take(&mut self.state.agreements),
                                LivingAgreementV2 {
                                    kind: *kind,
                                    participant_a: pair.0,
                                    participant_b: pair.1,
                                    start_tick: *start_tick,
                                    end_tick: *end_tick,
                                },
                            ),
                        ))
                    }
                    LivingCreatorOperationV2::RemoveAgreement {
                        kind,
                        participant_a,
                        participant_b,
                    } => {
                        let pair = self.resolve_pair(participant_a, participant_b)?;
                        self.check_diplomacy_participants(pair)?;
                        let position = self
                            .state
                            .agreements
                            .as_slice()
                            .binary_search_by_key(
                                &(pair.0, pair.1, kind.as_wire_str()),
                                LivingKeyedV2::living_key,
                            )
                            .map_err(|_| {
                                command_fault(LivingCommandRejectionV2::MissingTarget {
                                    operation: self.operation_index,
                                    reference: "agreement",
                                })
                            })?;
                        let mut work = QueuedDiplomacyWork::default();
                        work.plan.insert(position);
                        work.filter = Some(QueuedDiplomacyFilter::Agreement(
                            super::step_job::LivingPlannedFilterJobV2::begin(
                                std::mem::take(&mut self.state.agreements),
                                std::mem::take(&mut work.plan),
                            ),
                        ));
                        self.diplomacy_work = Some(work);
                        self.stage = DiplomacyFilter;
                        return Ok(true);
                    }
                    LivingCreatorOperationV2::ForceWar {
                        participant_a,
                        participant_b,
                        ..
                    } => {
                        self.diplomacy_work = Some(QueuedDiplomacyWork {
                            pair: Some(self.resolve_pair(participant_a, participant_b)?),
                            ..Default::default()
                        });
                        self.index = 0;
                        self.stage = AgreementPlan;
                        return Ok(true);
                    }
                    LivingCreatorOperationV2::ForcePeace {
                        participant_a,
                        participant_b,
                    } => {
                        let pair = self.resolve_pair(participant_a, participant_b)?;
                        self.check_diplomacy_participants(pair)?;
                        let mut work = QueuedDiplomacyWork {
                            pair: Some(pair),
                            ..Default::default()
                        };
                        if let Ok(position) = self
                            .state
                            .wars
                            .as_slice()
                            .binary_search_by_key(&pair, LivingKeyedV2::living_key)
                        {
                            work.plan.insert(position);
                        }
                        work.filter = Some(QueuedDiplomacyFilter::War(
                            super::step_job::LivingPlannedFilterJobV2::begin(
                                std::mem::take(&mut self.state.wars),
                                std::mem::take(&mut work.plan),
                            ),
                        ));
                        self.diplomacy_work = Some(work);
                        self.stage = DiplomacyFilter;
                        return Ok(true);
                    }
                    LivingCreatorOperationV2::SetFleetOrder { fleet, .. } => {
                        self.diplomacy_work = Some(QueuedDiplomacyWork {
                            fleet: Some(self.resolve(fleet)?),
                            ..Default::default()
                        });
                        self.index = 0;
                        self.stage = OrderReservations;
                        return Ok(true);
                    }

                    LivingCreatorOperationV2::CreateColony { world, owner } => {
                        let world = self.resolve(world)?;
                        let owner = self.resolve(owner)?;
                        self.state
                            .worlds
                            .find_mut(&world)
                            .ok_or_else(|| {
                                command_fault(LivingCommandRejectionV2::MissingTarget {
                                    operation: self.operation_index,
                                    reference: "world",
                                })
                            })?
                            .owner = Some(owner);
                        let deadline = |period| {
                            revision
                                .tick
                                .0
                                .checked_add(period)
                                .map(LivingTickV2)
                                .ok_or_else(|| command_fault(LivingCommandRejectionV2::Exhausted))
                        };
                        self.colony_create = Some((
                            world,
                            owner,
                            [
                                deadline(LIVING_HUB_ENERGY_PERIOD_TICKS_V2)?,
                                deadline(LIVING_HUB_ORE_PERIOD_TICKS_V2)?,
                                deadline(LIVING_HUB_FALLBACK_PERIOD_TICKS_V2)?,
                            ],
                        ));
                        self.stage = ColonyCreateInsertion;
                        return Ok(true);
                    }
                    LivingCreatorOperationV2::SetWorldOwner { world, owner } => {
                        self.colony_operation = Some(QueuedColonyOperation::SetOwner {
                            world: self.resolve(world)?,
                            owner: owner
                                .as_ref()
                                .map(|owner| self.resolve(owner))
                                .transpose()?,
                        });
                        self.index = 0;
                        self.stage = ColonyConstructionReservations;
                        return Ok(true);
                    }
                    LivingCreatorOperationV2::RemoveColony { world } => {
                        let world = self.resolve(world)?;
                        if self
                            .state
                            .colonies
                            .as_slice()
                            .binary_search_by_key(&world, |row| row.world)
                            .is_err()
                        {
                            return Err(command_fault(LivingCommandRejectionV2::MissingTarget {
                                operation: self.operation_index,
                                reference: "colony",
                            })
                            .into());
                        }
                        self.colony_operation = Some(QueuedColonyOperation::Remove { world });
                        self.index = 0;
                        self.stage = ColonyConstructionReservations;
                        return Ok(true);
                    }
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
                    LivingCreatorOperationV2::CreateFacility {
                        local,
                        world,
                        definition,
                        status,
                        paid_alloy,
                        hit_points,
                        next_due,
                    } => {
                        let row = LivingFacilityV2 {
                            id: self.locals[local],
                            world: self.resolve(world)?,
                            definition: definition.clone(),
                            status: *status,
                            paid_alloy: *paid_alloy,
                            hit_points: *hit_points,
                            next_due: *next_due,
                            next_repair: None,
                            blocked_reasons: LivingSortedVecV2::default(),
                            disabled: false,
                        };
                        Some(QueuedCreatorInsertion::Facility(Insert::begin_replacement(
                            std::mem::take(&mut self.state.facilities),
                            row,
                        )))
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
                        let row = LivingConstructionJobV2 {
                            id: self.locals[local],
                            world: self.resolve(world)?,
                            owner: self.resolve(owner)?,
                            definition: definition.clone(),
                            accepted_tick: *accepted_tick,
                            completion_tick: *completion_tick,
                            paid_alloy: *paid_alloy,
                        };
                        Some(QueuedCreatorInsertion::ConstructionJob(
                            Insert::begin_replacement(
                                std::mem::take(&mut self.state.construction_jobs),
                                row,
                            ),
                        ))
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
                        let row = LivingHullJobV2 {
                            id: self.locals[local],
                            world: self.resolve(world)?,
                            owner: self.resolve(owner)?,
                            definition: definition.clone(),
                            accepted_tick: *accepted_tick,
                            completion_tick: *completion_tick,
                            paid_alloy: *paid_alloy,
                            target_fleet: target_fleet
                                .as_ref()
                                .map(|target| self.resolve(target))
                                .transpose()?,
                        };
                        Some(QueuedCreatorInsertion::HullJob(Insert::begin_replacement(
                            std::mem::take(&mut self.state.hull_jobs),
                            row,
                        )))
                    }
                    LivingCreatorOperationV2::CreateFleet {
                        local,
                        owner,
                        world,
                    } => {
                        let row = LivingFleetV2 {
                            id: self.locals[local],
                            owner: self.resolve(owner)?,
                            location: LivingFleetLocationV2::Docked {
                                world: self.resolve(world)?,
                            },
                            order: None,
                            hulls: LivingSortedVecV2::default(),
                        };
                        Some(QueuedCreatorInsertion::Fleet(Insert::begin_replacement(
                            std::mem::take(&mut self.state.fleets),
                            row,
                        )))
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
                        let row = LivingFreightRouteV2 {
                            id: self.locals[local],
                            kind: *kind,
                            source_world: self.resolve(source_world)?,
                            destination_world: self.resolve(destination_world)?,
                            resource: *resource,
                            batch_size: *batch_size,
                            cadence_ticks: *cadence_ticks,
                            source_reserve: *source_reserve,
                            source_owner: self.resolve(source_owner)?,
                            receiver_owner: self.resolve(receiver_owner)?,
                            next_due: *next_due,
                            suspended: false,
                        };
                        Some(QueuedCreatorInsertion::Route(Insert::begin_replacement(
                            std::mem::take(&mut self.state.routes),
                            row,
                        )))
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
                        let row = LivingShipmentV2 {
                            id: self.locals[local],
                            dispatch_owner: self.resolve(dispatch_owner)?,
                            intended_receiver: self.resolve(intended_receiver)?,
                            source_world: self.resolve(source_world)?,
                            destination_world: self.resolve(destination_world)?,
                            resource: *resource,
                            units: *units,
                            departure_tick: *departure_tick,
                            arrival_tick: *arrival_tick,
                            disposition: *disposition,
                            dispatch_kind: None,
                            returning_to_source: false,
                        };
                        Some(QueuedCreatorInsertion::Shipment(Insert::begin_replacement(
                            std::mem::take(&mut self.state.shipments),
                            row,
                        )))
                    }
                    LivingCreatorOperationV2::CreateHazard {
                        local,
                        definition,
                        lane,
                        start_tick,
                        end_tick,
                    } => {
                        let row = LivingHazardV2 {
                            id: self.locals[local],
                            definition: definition.clone(),
                            lane: self.resolve(lane)?,
                            start_tick: *start_tick,
                            end_tick: *end_tick,
                        };
                        Some(QueuedCreatorInsertion::Hazard(Insert::begin_replacement(
                            std::mem::take(&mut self.state.hazards),
                            row,
                        )))
                    }
                    LivingCreatorOperationV2::CreateHull {
                        local,
                        fleet,
                        definition,
                        hit_points,
                        return_credit,
                    } => {
                        let id = self.resolve(fleet)?;
                        let index = self
                            .state
                            .fleets
                            .as_slice()
                            .binary_search_by_key(&id, |row| row.id)
                            .map_err(|_| {
                                self.replay_fault(LivingCommandRejectionV2::MissingTarget {
                                    operation: self.operation_index,
                                    reference: "fleet",
                                })
                            })?;
                        let fleet = self
                            .state
                            .fleets
                            .record_at_mut(index)
                            .expect("located fleet");
                        let shell = LivingFleetV2 {
                            id: fleet.id,
                            owner: fleet.owner,
                            location: fleet.location,
                            order: None,
                            hulls: LivingSortedVecV2::default(),
                        };
                        let mut parent = std::mem::replace(fleet, shell);
                        let source = std::mem::take(&mut parent.hulls);
                        self.hull_parent = Some(parent);
                        self.hull_index = index;
                        self.hull_id = Some(id);
                        Some(QueuedCreatorInsertion::Hull(Insert::begin_replacement(
                            source,
                            LivingHullV2 {
                                id: self.locals[local],
                                definition: definition.clone(),
                                hit_points: *hit_points,
                                return_credit: *return_credit,
                                next_repair: None,
                            },
                        )))
                    }
                    LivingCreatorOperationV2::RemoveEntity { cascade, .. }
                    | LivingCreatorOperationV2::ScrapFacility { cascade, .. } => {
                        if matches!(
                            cascade,
                            super::command::LivingCascadeDispositionV2::RemoveReviewed { .. }
                        ) {
                            self.stage = NeedsSynchronous;
                            return Ok(true);
                        }
                        Some(QueuedCreatorInsertion::Removal(QueuedRemovalWork::begin()))
                    }
                    LivingCreatorOperationV2::SplitFleet { .. } => {
                        Some(QueuedCreatorInsertion::FleetStructural(
                            QueuedFleetStructuralWork::begin(true),
                        ))
                    }
                    LivingCreatorOperationV2::MergeFleets { .. } => {
                        Some(QueuedCreatorInsertion::FleetStructural(
                            QueuedFleetStructuralWork::begin(false),
                        ))
                    }
                    LivingCreatorOperationV2::EditFleet { target, .. } => {
                        let id = self.resolve(target)?;
                        let fleet = self.editable_fleet(id)?;
                        self.stage = FleetEditReservations {
                            fleet,
                            id,
                            hull: None,
                            index: 0,
                        };
                        return Ok(true);
                    }
                    LivingCreatorOperationV2::EditHull { target, .. } => {
                        self.stage = HullDefinition {
                            hull: self.resolve(target)?,
                            index: 0,
                        };
                        return Ok(true);
                    }
                    LivingCreatorOperationV2::EditWorld { target, system, .. } => {
                        let id = self.resolve(target)?;
                        let world = self
                            .state
                            .worlds
                            .as_slice()
                            .binary_search_by_key(&id, |row| row.id)
                            .map_err(|_| {
                                command_fault(LivingCommandRejectionV2::MissingTarget {
                                    operation: self.operation_index,
                                    reference: "world",
                                })
                            })?;
                        self.stage = if self.state.worlds.as_slice()[world].system
                            == self.resolve(system)?
                        {
                            WorldInstall { world }
                        } else {
                            WorldShipments { world, index: 0 }
                        };
                        return Ok(true);
                    }
                    LivingCreatorOperationV2::EditLane {
                        target,
                        system_a,
                        system_b,
                        distance_units,
                    } => {
                        let id = self.resolve(target)?;
                        let a = self.resolve(system_a)?;
                        let b = self.resolve(system_b)?;
                        if a == b || *distance_units == 0 {
                            return Err(command_fault(LivingCommandRejectionV2::Projection(
                                LivingValidationErrorV2::Range {
                                    field: "lane endpoints/distance",
                                },
                            ))
                            .into());
                        }
                        let lane = self
                            .state
                            .lanes
                            .as_slice()
                            .binary_search_by_key(&id, |row| row.id)
                            .map_err(|_| {
                                command_fault(LivingCommandRejectionV2::MissingTarget {
                                    operation: self.operation_index,
                                    reference: "lane",
                                })
                            })?;
                        let existing = &self.state.lanes.as_slice()[lane];
                        self.stage = if existing.system_a == a.min(b)
                            && existing.system_b == a.max(b)
                            && existing.distance_units == *distance_units
                        {
                            LaneInstall { lane }
                        } else {
                            LaneFleets { lane, index: 0 }
                        };
                        return Ok(true);
                    }
                    LivingCreatorOperationV2::EditSystem { target, name } => {
                        let id = self.resolve(target)?;
                        edit(&mut self.state.systems, id, self.operation_index, "system")
                            .map_err(command_fault)?
                            .name = name.clone();
                        None
                    }
                    LivingCreatorOperationV2::EditStar {
                        target,
                        system,
                        archetype,
                        name,
                    } => {
                        let id = self.resolve(target)?;
                        // Resolve the new parent only after the original target's missing check.
                        if self
                            .state
                            .stars
                            .as_slice()
                            .binary_search_by_key(&id, |row| row.id)
                            .is_err()
                        {
                            return Err(command_fault(LivingCommandRejectionV2::MissingTarget {
                                operation: self.operation_index,
                                reference: "star",
                            })
                            .into());
                        }
                        let system = self.resolve(system)?;
                        let star = edit(&mut self.state.stars, id, self.operation_index, "star")
                            .map_err(command_fault)?;
                        star.system = system;
                        star.archetype = archetype.clone();
                        star.name = name.clone();
                        None
                    }
                    LivingCreatorOperationV2::EditDeposit {
                        target,
                        resource,
                        remaining_units,
                    } => {
                        let id = self.resolve(target)?;
                        let row = edit(
                            &mut self.state.deposits,
                            id,
                            self.operation_index,
                            "deposit",
                        )
                        .map_err(command_fault)?;
                        row.resource = *resource;
                        row.remaining_units = *remaining_units;
                        None
                    }
                    LivingCreatorOperationV2::EditCivilization {
                        target,
                        name,
                        policy,
                    } => {
                        let id = self.resolve(target)?;
                        let row = edit(
                            &mut self.state.civilizations,
                            id,
                            self.operation_index,
                            "civilization",
                        )
                        .map_err(command_fault)?;
                        row.name = name.clone();
                        row.policy = policy.clone();
                        None
                    }
                    LivingCreatorOperationV2::SetFacilityEnabled { target, enabled } => {
                        let id = self.resolve(target)?;
                        edit(
                            &mut self.state.facilities,
                            id,
                            self.operation_index,
                            "facility",
                        )
                        .map_err(command_fault)?
                        .disabled = !*enabled;
                        None
                    }
                    LivingCreatorOperationV2::SetShipmentDisposition {
                        shipment,
                        disposition,
                    } => {
                        let id = self.resolve(shipment)?;
                        let row = edit(
                            &mut self.state.shipments,
                            id,
                            self.operation_index,
                            "shipment",
                        )
                        .map_err(command_fault)?;
                        row.returning_to_source = *disposition
                            == LivingShipmentDispositionV2::WaitingForStorage
                            && (row.disposition == LivingShipmentDispositionV2::Returning
                                || row.returning_to_source);
                        row.disposition = *disposition;
                        None
                    }
                    LivingCreatorOperationV2::EditHazard {
                        start_tick,
                        end_tick,
                        ..
                    } => {
                        if *start_tick < revision.tick || end_tick <= start_tick {
                            return Err(command_fault(LivingCommandRejectionV2::Projection(
                                LivingValidationErrorV2::Range {
                                    field: "hazard interval",
                                },
                            ))
                            .into());
                        }
                        self.index = 0;
                        self.stage = HazardDefinition;
                        return Ok(true);
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
                if let Some(QueuedCreatorInsertion::Removal(work)) = &mut self.insertion {
                    if !work.ready() {
                        let queued = &self.queue[self.revision_index];
                        let operations = queued.revision.command.operations();
                        return work
                            .visit(
                                &mut self.state,
                                &operations[self.operation_index],
                                &self.locals,
                            )
                            .map_err(|cause| {
                                cause
                                    .public(queued.accepted.accepted_sequence, self.operation_index)
                            });
                    }
                    self.retirement.start(self.insertion.take());
                    self.colony_changed = true;
                    self.index = 0;
                    self.stage = StatusOwners;
                    return Ok(true);
                }
                if let Some(QueuedCreatorInsertion::FleetStructural(work)) = &mut self.insertion {
                    if !work.ready() {
                        let queued = &self.queue[self.revision_index];
                        let operations = queued.revision.command.operations();
                        return work
                            .visit(
                                &mut self.state,
                                &operations[self.operation_index],
                                &self.locals,
                            )
                            .map_err(|fault| {
                                fault
                                    .public(queued.accepted.accepted_sequence, self.operation_index)
                            });
                    }
                    self.retirement.start(self.insertion.take());
                    self.operation_index += 1;
                    self.stage = Operation;
                    return Ok(true);
                }
                let Some(insertion) = &mut self.insertion else {
                    return Err(LivingDeterministicFaultV2::Invariant.into());
                };
                let ready = match insertion {
                    QueuedCreatorInsertion::Removal(_)
                    | QueuedCreatorInsertion::FleetStructural(_) => {
                        unreachable!("handled structural work")
                    }
                    QueuedCreatorInsertion::System(job) => job.ready(),
                    QueuedCreatorInsertion::Star(job) => job.ready(),
                    QueuedCreatorInsertion::World(job) => job.ready(),
                    QueuedCreatorInsertion::Lane(job) => job.ready(),
                    QueuedCreatorInsertion::Civilization(job) => job.ready(),
                    QueuedCreatorInsertion::Deposit(job) => job.ready(),
                    QueuedCreatorInsertion::Facility(job) => job.ready(),
                    QueuedCreatorInsertion::ConstructionJob(job) => job.ready(),
                    QueuedCreatorInsertion::HullJob(job) => job.ready(),
                    QueuedCreatorInsertion::Fleet(job) => job.ready(),
                    QueuedCreatorInsertion::Hull(job) => job.ready(),
                    QueuedCreatorInsertion::Route(job) => job.ready(),
                    QueuedCreatorInsertion::Shipment(job) => job.ready(),
                    QueuedCreatorInsertion::Hazard(job) => job.ready(),
                    QueuedCreatorInsertion::Relation(job) => job.ready(),
                    QueuedCreatorInsertion::Agreement(job) => job.ready(),
                    QueuedCreatorInsertion::War(job) => job.ready(),
                    QueuedCreatorInsertion::Colony(job) => job.ready(),
                };
                if !ready {
                    return match insertion {
                        QueuedCreatorInsertion::Removal(_)
                        | QueuedCreatorInsertion::FleetStructural(_) => {
                            unreachable!("handled structural work")
                        }
                        QueuedCreatorInsertion::System(job) => job.visit(),
                        QueuedCreatorInsertion::Star(job) => job.visit(),
                        QueuedCreatorInsertion::World(job) => job.visit(),
                        QueuedCreatorInsertion::Lane(job) => job.visit(),
                        QueuedCreatorInsertion::Civilization(job) => job.visit(),
                        QueuedCreatorInsertion::Deposit(job) => job.visit(),
                        QueuedCreatorInsertion::Facility(job) => job.visit(),
                        QueuedCreatorInsertion::ConstructionJob(job) => job.visit(),
                        QueuedCreatorInsertion::HullJob(job) => job.visit(),
                        QueuedCreatorInsertion::Fleet(job) => job.visit(),
                        QueuedCreatorInsertion::Hull(job) => job.visit(),
                        QueuedCreatorInsertion::Route(job) => job.visit(),
                        QueuedCreatorInsertion::Shipment(job) => job.visit(),
                        QueuedCreatorInsertion::Hazard(job) => job.visit(),
                        QueuedCreatorInsertion::Relation(job) => job.visit(),
                        QueuedCreatorInsertion::Agreement(job) => job.visit(),
                        QueuedCreatorInsertion::War(job) => job.visit(),
                        QueuedCreatorInsertion::Colony(job) => job.visit(),
                    };
                }
                match insertion {
                    QueuedCreatorInsertion::Removal(_)
                    | QueuedCreatorInsertion::FleetStructural(_) => {
                        unreachable!("handled structural work")
                    }
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
                    QueuedCreatorInsertion::Facility(job) => {
                        self.state.facilities = job
                            .take_ready_result()
                            .map_err(|_| LivingDeterministicFaultV2::Invariant)?;
                    }
                    QueuedCreatorInsertion::ConstructionJob(job) => {
                        self.state.construction_jobs = job
                            .take_ready_result()
                            .map_err(|_| LivingDeterministicFaultV2::Invariant)?;
                    }
                    QueuedCreatorInsertion::HullJob(job) => {
                        self.state.hull_jobs = job
                            .take_ready_result()
                            .map_err(|_| LivingDeterministicFaultV2::Invariant)?;
                    }
                    QueuedCreatorInsertion::Fleet(job) => {
                        self.state.fleets = job
                            .take_ready_result()
                            .map_err(|_| LivingDeterministicFaultV2::Invariant)?;
                    }
                    QueuedCreatorInsertion::Route(job) => {
                        self.state.routes = job
                            .take_ready_result()
                            .map_err(|_| LivingDeterministicFaultV2::Invariant)?;
                    }
                    QueuedCreatorInsertion::Shipment(job) => {
                        self.state.shipments = job
                            .take_ready_result()
                            .map_err(|_| LivingDeterministicFaultV2::Invariant)?;
                    }
                    QueuedCreatorInsertion::Hazard(job) => {
                        self.state.hazards = job
                            .take_ready_result()
                            .map_err(|_| LivingDeterministicFaultV2::Invariant)?;
                    }
                    QueuedCreatorInsertion::Relation(job) => {
                        self.state.relations = job
                            .take_ready_result()
                            .map_err(|_| LivingDeterministicFaultV2::Invariant)?;
                    }
                    QueuedCreatorInsertion::Agreement(job) => {
                        self.state.agreements = job
                            .take_ready_result()
                            .map_err(|_| LivingDeterministicFaultV2::Invariant)?;
                    }
                    QueuedCreatorInsertion::War(job) => {
                        self.state.wars = job
                            .take_ready_result()
                            .map_err(|_| LivingDeterministicFaultV2::Invariant)?;
                    }
                    QueuedCreatorInsertion::Colony(job) => {
                        self.state.colonies = job
                            .take_ready_result()
                            .map_err(|_| LivingDeterministicFaultV2::Invariant)?;
                        self.retirement.start(self.insertion.take());
                        self.colony_changed = true;
                        self.index = 0;
                        self.stage = StatusOwners;
                        return Ok(true);
                    }
                    QueuedCreatorInsertion::Hull(job) => {
                        self.hull_output = Some(
                            job.take_ready_result()
                                .map_err(|_| LivingDeterministicFaultV2::Invariant)?,
                        );
                        self.stage = HullReassemble;
                        return Ok(true);
                    }
                }
                self.retirement.start(self.insertion.take());
                if self.diplomacy_work.is_some() {
                    self.stage = DiplomacyFinish;
                } else if self.cancellation_work.is_some() {
                    self.stage = CancellationFinish;
                } else {
                    self.operation_index += 1;
                    self.stage = Operation;
                }
                return Ok(true);
            }
            CancelRefund => {
                let (world, paid) = self
                    .cancellation_work
                    .as_ref()
                    .and_then(|work| work.refund)
                    .ok_or(LivingDeterministicFaultV2::Invariant)?;
                let Some(row) = self
                    .state
                    .worlds
                    .as_slice()
                    .binary_search_by_key(&world, |row| row.id)
                    .ok()
                    .map(|i| &self.state.worlds.as_slice()[i])
                else {
                    // Original private malformed-source behavior: no world, no refund.
                    self.cancellation_work.as_mut().unwrap().refund = None;
                    self.stage = CancelPlan;
                    return Ok(true);
                };
                let refunded = row
                    .inventory
                    .alloy
                    .checked_add(paid / 2)
                    .ok_or_else(|| self.replay_fault(LivingCommandRejectionV2::Exhausted))?;
                self.cancellation_work.as_mut().unwrap().refund = Some((world, refunded));
                if refunded > super::catalog::LIVING_RESOURCE_STORAGE_LIMIT_V2 {
                    self.prepare_effect(LivingEventKindV2::RefundDiscarded, CancelRefundWrite);
                } else {
                    self.stage = CancelRefundWrite;
                }
                return Ok(true);
            }
            CancelRefundWrite => {
                let (world, refunded) = self
                    .cancellation_work
                    .as_mut()
                    .and_then(|work| work.refund.take())
                    .ok_or(LivingDeterministicFaultV2::Invariant)?;
                self.state
                    .worlds
                    .find_mut(&world)
                    .ok_or(LivingDeterministicFaultV2::Invariant)?
                    .inventory
                    .alloy = refunded.min(super::catalog::LIVING_RESOURCE_STORAGE_LIMIT_V2);
                self.stage = CancelPlan;
                return Ok(true);
            }
            CancelPlan => {
                let work = self
                    .cancellation_work
                    .as_mut()
                    .ok_or(LivingDeterministicFaultV2::Invariant)?;
                work.plan.insert(work.position);
                self.stage = CancelFilterStart;
                return Ok(true);
            }
            CancelFilterStart => {
                let work = self
                    .cancellation_work
                    .as_mut()
                    .ok_or(LivingDeterministicFaultV2::Invariant)?;
                let plan = std::mem::take(&mut work.plan);
                use super::step_job::LivingPlannedFilterJobV2 as Filter;
                work.filter = Some(
                    match work.kind.ok_or(LivingDeterministicFaultV2::Invariant)? {
                        QueuedCancellationKind::Construction => {
                            QueuedCancellationFilter::Construction(Filter::begin(
                                std::mem::take(&mut self.state.construction_jobs),
                                plan,
                            ))
                        }
                        QueuedCancellationKind::Hull => QueuedCancellationFilter::Hull(
                            Filter::begin(std::mem::take(&mut self.state.hull_jobs), plan),
                        ),
                        QueuedCancellationKind::Hazard => QueuedCancellationFilter::Hazard(
                            Filter::begin(std::mem::take(&mut self.state.hazards), plan),
                        ),
                    },
                );
                self.stage = CancelFilter;
                return Ok(true);
            }
            CancelFilter => {
                let work = self
                    .cancellation_work
                    .as_mut()
                    .ok_or(LivingDeterministicFaultV2::Invariant)?;
                match work
                    .filter
                    .as_mut()
                    .ok_or(LivingDeterministicFaultV2::Invariant)?
                {
                    QueuedCancellationFilter::Construction(child) => {
                        if !child.ready() {
                            return child.visit();
                        }
                        self.state.construction_jobs = child
                            .take_ready_result()
                            .map_err(|_| LivingDeterministicFaultV2::Invariant)?;
                    }
                    QueuedCancellationFilter::Hull(child) => {
                        if !child.ready() {
                            return child.visit();
                        }
                        self.state.hull_jobs = child
                            .take_ready_result()
                            .map_err(|_| LivingDeterministicFaultV2::Invariant)?;
                    }
                    QueuedCancellationFilter::Hazard(child) => {
                        if !child.ready() {
                            return child.visit();
                        }
                        self.state.hazards = child
                            .take_ready_result()
                            .map_err(|_| LivingDeterministicFaultV2::Invariant)?;
                    }
                }
                self.retirement.start(work.filter.take());
                self.stage = if matches!(work.kind, Some(QueuedCancellationKind::Hazard)) {
                    CancellationFinish
                } else {
                    CancelEffect
                };
                return Ok(true);
            }
            CancelEffect => {
                self.prepare_effect(LivingEventKindV2::ConstructionCancelled, CancellationFinish);
                return Ok(true);
            }
            CancellationFinish => {
                self.retirement.start(self.cancellation_work.take());
                self.operation_index += 1;
                self.stage = Operation;
                return Ok(true);
            }
            RouteSource | RouteDestination | RouteWar | RouteKind | RouteTrade => {
                let route = self
                    .cancellation_work
                    .as_ref()
                    .and_then(|work| work.route.as_ref())
                    .ok_or(LivingDeterministicFaultV2::Invariant)?;
                let tick = self.queue[self.revision_index].revision.tick;
                let pair = (
                    route.source_owner.min(route.receiver_owner),
                    route.source_owner.max(route.receiver_owner),
                );
                match self.stage {
                    RouteSource | RouteDestination => {
                        let (world, owner) = if matches!(self.stage, RouteSource) {
                            (route.source_world, route.source_owner)
                        } else {
                            (route.destination_world, route.receiver_owner)
                        };
                        let current = self
                            .state
                            .worlds
                            .as_slice()
                            .binary_search_by_key(&world, |row| row.id)
                            .ok()
                            .and_then(|i| self.state.worlds.as_slice()[i].owner);
                        if current != Some(owner) {
                            return Err(self.route_permission_fault().into());
                        }
                        self.stage = if matches!(self.stage, RouteSource) {
                            RouteDestination
                        } else {
                            RouteWar
                        };
                    }
                    RouteWar => {
                        if self
                            .state
                            .wars
                            .as_slice()
                            .binary_search_by_key(&pair, LivingKeyedV2::living_key)
                            .ok()
                            .is_some_and(|i| {
                                let war = &self.state.wars.as_slice()[i];
                                war.start_tick <= tick && tick < war.end_tick
                            })
                        {
                            return Err(self.route_permission_fault().into());
                        }
                        self.stage = RouteKind;
                    }
                    RouteKind => match route.kind {
                        LivingRouteKindV2::Internal
                            if route.source_owner != route.receiver_owner =>
                        {
                            return Err(self.route_permission_fault().into());
                        }
                        LivingRouteKindV2::Trade => self.stage = RouteTrade,
                        LivingRouteKindV2::Internal | LivingRouteKindV2::Aid => {
                            self.stage = RouteDeadline
                        }
                    },
                    RouteTrade => {
                        if !self
                            .state
                            .agreements
                            .as_slice()
                            .binary_search_by_key(
                                &(pair.0, pair.1, LivingAgreementKindV2::Trade.as_wire_str()),
                                LivingKeyedV2::living_key,
                            )
                            .ok()
                            .is_some_and(|i| {
                                let agreement = &self.state.agreements.as_slice()[i];
                                agreement.start_tick <= tick && tick < agreement.end_tick
                            })
                        {
                            return Err(self.route_permission_fault().into());
                        }
                        self.stage = RouteDeadline;
                    }
                    _ => unreachable!(),
                }
                return Ok(true);
            }
            RouteDeadline => {
                let tick = self.queue[self.revision_index].revision.tick;
                let accepted_sequence = self.queue[self.revision_index].accepted.accepted_sequence;
                let route = self
                    .cancellation_work
                    .as_mut()
                    .and_then(|work| work.route.as_mut())
                    .ok_or(LivingDeterministicFaultV2::Invariant)?;
                route.suspended = false;
                if route.next_due <= tick {
                    route.next_due = LivingTickV2(
                        tick.0
                            .checked_add(u64::from(route.cadence_ticks))
                            .ok_or_else(|| LivingDeterministicFaultV2::Replay {
                                accepted_sequence,
                                reason: Box::new(LivingCommandRejectionV2::Exhausted),
                            })?,
                    );
                }
                self.stage = RouteInstall;
                return Ok(true);
            }
            WorldShipments { world, index } => {
                if let Some(shipment) = self.state.shipments.as_slice().get(index) {
                    let target = self.state.worlds.as_slice()[world].id;
                    if shipment.source_world == target || shipment.destination_world == target {
                        return Err(self
                            .replay_fault(LivingCommandRejectionV2::ReferencedTarget)
                            .into());
                    }
                    self.stage = WorldShipments {
                        world,
                        index: index + 1,
                    };
                    return Ok(true);
                }
                self.stage = WorldFleets { world, index: 0 };
            }
            WorldFleets { world, index } => {
                if let Some(fleet) = self.state.fleets.as_slice().get(index) {
                    let target = self.state.worlds.as_slice()[world].id;
                    let blocked = match fleet.location {
                        super::model::LivingFleetLocationV2::Travelling {
                            origin_world,
                            destination_world,
                            ..
                        } => origin_world == target || destination_world == target,
                        super::model::LivingFleetLocationV2::Docked { world } => {
                            world == target && fleet.order.is_some()
                        }
                    };
                    if blocked {
                        return Err(self
                            .replay_fault(LivingCommandRejectionV2::ReferencedTarget)
                            .into());
                    }
                    self.stage = WorldFleets {
                        world,
                        index: index + 1,
                    };
                    return Ok(true);
                }
                self.stage = WorldInstall { world };
            }
            WorldInstall { world } => {
                // Proposed bounded leaves stay in the original accepted queue until install.
                let LivingCreatorOperationV2::EditWorld {
                    system,
                    archetype,
                    name,
                    ..
                } = &self.queue[self.revision_index]
                    .revision
                    .command
                    .operations()[self.operation_index]
                else {
                    return Err(LivingDeterministicFaultV2::Invariant.into());
                };
                let system = self.resolve(system)?;
                let row = self
                    .state
                    .worlds
                    .record_at_mut(world)
                    .ok_or(LivingDeterministicFaultV2::Invariant)?;
                row.system = system;
                row.archetype = archetype.clone();
                row.name = name.clone();
                self.operation_index += 1;
                self.stage = Operation;
                return Ok(true);
            }
            LaneFleets { lane, index } => {
                if let Some(fleet) = self.state.fleets.as_slice().get(index) {
                    self.stage = if fleet.order.is_some() {
                        LanePath {
                            lane,
                            fleet: index,
                            path: 0,
                        }
                    } else {
                        LaneFleets {
                            lane,
                            index: index + 1,
                        }
                    };
                    return Ok(true);
                }
                self.stage = LaneInstall { lane };
            }
            LanePath { lane, fleet, path } => {
                let order = self.state.fleets.as_slice()[fleet]
                    .order
                    .as_ref()
                    .ok_or(LivingDeterministicFaultV2::Invariant)?;
                if let Some(id) = order.lane_path.get(path) {
                    if *id == self.state.lanes.as_slice()[lane].id {
                        return Err(self
                            .replay_fault(LivingCommandRejectionV2::ReferencedTarget)
                            .into());
                    }
                    self.stage = LanePath {
                        lane,
                        fleet,
                        path: path + 1,
                    };
                    return Ok(true);
                }
                self.stage = LaneFleets {
                    lane,
                    index: fleet + 1,
                };
            }
            LaneInstall { lane } => {
                let LivingCreatorOperationV2::EditLane {
                    system_a,
                    system_b,
                    distance_units,
                    ..
                } = &self.queue[self.revision_index]
                    .revision
                    .command
                    .operations()[self.operation_index]
                else {
                    return Err(LivingDeterministicFaultV2::Invariant.into());
                };
                let a = self.resolve(system_a)?;
                let b = self.resolve(system_b)?;
                let row = self
                    .state
                    .lanes
                    .record_at_mut(lane)
                    .ok_or(LivingDeterministicFaultV2::Invariant)?;
                row.system_a = a.min(b);
                row.system_b = a.max(b);
                row.distance_units = *distance_units;
                self.operation_index += 1;
                self.stage = Operation;
                return Ok(true);
            }
            HullDefinition { hull, index } => {
                let LivingCreatorOperationV2::EditHull {
                    definition,
                    hit_points,
                    ..
                } = &self.queue[self.revision_index]
                    .revision
                    .command
                    .operations()[self.operation_index]
                else {
                    return Err(LivingDeterministicFaultV2::Invariant.into());
                };
                let Some(row) = self.catalog.hulls().get(index) else {
                    return Err(self
                        .replay_fault(LivingCommandRejectionV2::Projection(
                            LivingValidationErrorV2::Range {
                                field: "hull definition",
                            },
                        ))
                        .into());
                };
                if row.id == *definition {
                    if *hit_points == 0 || *hit_points > row.hit_points {
                        return Err(self
                            .replay_fault(LivingCommandRejectionV2::Projection(
                                LivingValidationErrorV2::Range {
                                    field: "hull hit points",
                                },
                            ))
                            .into());
                    }
                    self.stage = HullParent { hull, index: 0 };
                } else {
                    self.stage = HullDefinition {
                        hull,
                        index: index + 1,
                    };
                }
                return Ok(true);
            }
            HullParent { hull, index } => {
                let Some(row) = self.state.fleets.as_slice().get(index) else {
                    return Err(self
                        .replay_fault(LivingCommandRejectionV2::MissingTarget {
                            operation: self.operation_index,
                            reference: "hull",
                        })
                        .into());
                };
                // One parent per visit, fixed-key lookup inside its original sorted Hulls.
                if let Ok(position) = row.hulls.as_slice().binary_search_by_key(&hull, |h| h.id) {
                    let id = row.id;
                    let fleet = self.editable_fleet(id)?;
                    self.stage = FleetEditReservations {
                        fleet,
                        id,
                        hull: Some((position, hull)),
                        index: 0,
                    };
                } else {
                    self.stage = HullParent {
                        hull,
                        index: index + 1,
                    };
                }
                return Ok(true);
            }
            FleetEditReservations {
                fleet,
                id,
                hull,
                index,
            } => {
                if let Some(row) = self.state.hull_jobs.as_slice().get(index) {
                    if row.target_fleet == Some(id) {
                        return Err(self
                            .replay_fault(LivingCommandRejectionV2::ReferencedTarget)
                            .into());
                    }
                    self.stage = FleetEditReservations {
                        fleet,
                        id,
                        hull,
                        index: index + 1,
                    };
                    return Ok(true);
                }
                self.stage = FleetEditInstall { fleet, id, hull };
            }
            FleetEditInstall { fleet, id, hull } => {
                let operation = &self.queue[self.revision_index]
                    .revision
                    .command
                    .operations()[self.operation_index];
                // All guarded lookups precede semantic mutation and bounded leaf cloning.
                let row = self
                    .state
                    .fleets
                    .as_slice()
                    .get(fleet)
                    .filter(|row| row.id == id)
                    .ok_or_else(|| {
                        self.replay_fault(LivingCommandRejectionV2::MissingTarget {
                            operation: self.operation_index,
                            reference: "fleet",
                        })
                    })?;
                if let Some((position, target)) = hull
                    && row
                        .hulls
                        .as_slice()
                        .get(position)
                        .is_none_or(|h| h.id != target)
                {
                    return Err(self
                        .replay_fault(LivingCommandRejectionV2::MissingTarget {
                            operation: self.operation_index,
                            reference: "hull",
                        })
                        .into());
                }
                match operation {
                    LivingCreatorOperationV2::EditFleet { owner, world, .. } if hull.is_none() => {
                        let owner = self.resolve(owner)?;
                        let world = self.resolve(world)?;
                        let row = self.state.fleets.record_at_mut(fleet).unwrap();
                        row.owner = owner;
                        row.location = super::model::LivingFleetLocationV2::Docked { world };
                    }
                    LivingCreatorOperationV2::EditHull {
                        definition,
                        hit_points,
                        return_credit,
                        ..
                    } => {
                        let (position, _) = hull.ok_or(LivingDeterministicFaultV2::Invariant)?;
                        let row = self
                            .state
                            .fleets
                            .record_at_mut(fleet)
                            .unwrap()
                            .hulls
                            .record_at_mut(position)
                            .unwrap();
                        // Slug is an owned <=64-byte Box leaf: replacement/release is
                        // part of this counted bounded-row action, never Copy metadata.
                        row.definition = definition.clone();
                        row.hit_points = *hit_points;
                        row.return_credit = *return_credit;
                    }
                    _ => return Err(LivingDeterministicFaultV2::Invariant.into()),
                }
                let order = self.state.fleets.record_at_mut(fleet).unwrap().order.take();
                self.retirement.start(order);
                self.operation_index += 1;
                self.stage = Operation;
                return Ok(true);
            }
            RouteInstall => {
                let route = self
                    .cancellation_work
                    .as_mut()
                    .and_then(|work| work.route.take())
                    .ok_or(LivingDeterministicFaultV2::Invariant)?;
                self.insertion = Some(QueuedCreatorInsertion::Route(
                    super::step_job::LivingSortedInsertionJobV2::begin_replacement(
                        std::mem::take(&mut self.state.routes),
                        route,
                    ),
                ));
                self.stage = Insertion;
                return Ok(true);
            }
            EffectPrepare => {
                let work = self
                    .effect_work
                    .as_mut()
                    .ok_or(LivingDeterministicFaultV2::Invariant)?;
                let child = work
                    .storage
                    .as_mut()
                    .ok_or(LivingDeterministicFaultV2::Invariant)?;
                if !child.ready() {
                    let result = child.poll(1);
                    let visits = child.last_poll_visits();
                    result.map_err(Self::recovery_storage_error)?;
                    return Ok(visits != 0);
                }
                work.values = child
                    .take_ready_result()
                    .map_err(Self::recovery_storage_error)?;
                self.retirement.start(work.storage.take());
                self.stage = EffectCommit;
                return Ok(true);
            }
            EffectCommit => {
                let work = self
                    .effect_work
                    .as_mut()
                    .ok_or(LivingDeterministicFaultV2::Invariant)?;
                super::step_job::LivingVecStorageJobV2::commit_one(
                    &mut work.values,
                    &mut work.pending,
                )
                .map_err(Self::recovery_storage_error)?;
                self.stage = work.resume;
                return Ok(true);
            }
            EffectsStart => {
                let work = self
                    .effect_work
                    .as_mut()
                    .ok_or(LivingDeterministicFaultV2::Invariant)?;
                work.input = Some(std::mem::take(&mut work.values).into_iter());
                self.stage = EffectsNext;
                return Ok(true);
            }
            EffectsNext => {
                let work = self
                    .effect_work
                    .as_mut()
                    .ok_or(LivingDeterministicFaultV2::Invariant)?;
                work.pending = work
                    .input
                    .as_mut()
                    .ok_or(LivingDeterministicFaultV2::Invariant)?
                    .next();
                if work.pending.is_some() {
                    self.stage = EffectAppendStart;
                } else {
                    self.retirement.start(self.effect_work.take());
                    self.stage = FinishRevision;
                }
                return Ok(true);
            }
            EffectAppendStart => {
                let kind = self
                    .effect_work
                    .as_mut()
                    .and_then(|work| work.pending.take())
                    .ok_or(LivingDeterministicFaultV2::Invariant)?;
                self.append = Some(
                    super::receipt::LivingPendingEventAppendJobV2::begin_ordinary(
                        std::mem::take(&mut self.events),
                        LivingEventProvenanceV2::Creator {
                            revision: self.queue[self.revision_index].revision.id,
                        },
                        kind,
                    ),
                );
                self.stage = EffectAppend;
                return Ok(true);
            }
            EffectAppend => {
                let child = self
                    .append
                    .as_mut()
                    .ok_or(LivingDeterministicFaultV2::Invariant)?;
                if child.status() != super::receipt::LivingPendingEventAppendStatusV2::Ready {
                    let result = child.poll(1);
                    let visits = child.last_poll_visits();
                    result.map_err(Self::append_error)?;
                    return Ok(visits != 0);
                }
                self.events = child.take_ready_result().map_err(Self::append_error)?.0;
                self.retirement.start(self.append.take());
                self.stage = EffectsNext;
                return Ok(true);
            }
            BridgeEffectsStart => {
                let work = self
                    .effect_work
                    .as_mut()
                    .ok_or(LivingDeterministicFaultV2::Invariant)?;
                work.suffix_input = Some(std::mem::take(&mut work.suffix).into_iter());
                self.stage = BridgeEffectsNext;
                return Ok(true);
            }
            BridgeEffectsNext => {
                let work = self
                    .effect_work
                    .as_mut()
                    .ok_or(LivingDeterministicFaultV2::Invariant)?;
                if let Some(kind) = work
                    .suffix_input
                    .as_mut()
                    .ok_or(LivingDeterministicFaultV2::Invariant)?
                    .next()
                {
                    self.prepare_effect(kind, BridgeEffectsNext);
                } else {
                    self.retirement.start(work.suffix_input.take());
                    self.operation_index += 1;
                    self.stage = Operation;
                }
                return Ok(true);
            }
            AgreementPlan => {
                let work = self
                    .diplomacy_work
                    .as_mut()
                    .ok_or(LivingDeterministicFaultV2::Invariant)?;
                let pair = work.pair.ok_or(LivingDeterministicFaultV2::Invariant)?;
                if let Some(row) = self.state.agreements.as_slice().get(self.index) {
                    if (row.participant_a, row.participant_b) == pair {
                        work.pending_plan = Some(self.index);
                        self.stage = DiplomacyPlanEmit;
                    }
                    self.index += 1;
                    return Ok(true);
                }
                work.filter = Some(QueuedDiplomacyFilter::Agreement(
                    super::step_job::LivingPlannedFilterJobV2::begin(
                        std::mem::take(&mut self.state.agreements),
                        std::mem::take(&mut work.plan),
                    ),
                ));
                self.stage = DiplomacyFilter;
                return Ok(true);
            }
            OccupationPlan => {
                let work = self
                    .diplomacy_work
                    .as_mut()
                    .ok_or(LivingDeterministicFaultV2::Invariant)?;
                if let Some(row) = self.state.occupations.as_slice().get(self.index) {
                    work.occupation = Some(*row);
                    self.stage = OccupationOwner;
                    return Ok(true);
                }
                work.filter = Some(QueuedDiplomacyFilter::Occupation(
                    super::step_job::LivingPlannedFilterJobV2::begin(
                        std::mem::take(&mut self.state.occupations),
                        std::mem::take(&mut work.plan),
                    ),
                ));
                self.stage = DiplomacyFilter;
                return Ok(true);
            }
            OccupationOwner => {
                let work = self
                    .diplomacy_work
                    .as_mut()
                    .ok_or(LivingDeterministicFaultV2::Invariant)?;
                let pair = work.pair.ok_or(LivingDeterministicFaultV2::Invariant)?;
                let row = work
                    .occupation
                    .take()
                    .ok_or(LivingDeterministicFaultV2::Invariant)?;
                let owner = self
                    .state
                    .worlds
                    .as_slice()
                    .binary_search_by_key(&row.world, |world| world.id)
                    .ok()
                    .and_then(|i| self.state.worlds.as_slice()[i].owner);
                self.stage = OccupationPlan;
                if (owner == Some(pair.0) && row.claimant == pair.1)
                    || (owner == Some(pair.1) && row.claimant == pair.0)
                {
                    work.pending_plan = Some(self.index);
                    self.stage = DiplomacyPlanEmit;
                }
                self.index += 1;
                return Ok(true);
            }
            DiplomacyPlanEmit => {
                let work = self
                    .diplomacy_work
                    .as_mut()
                    .ok_or(LivingDeterministicFaultV2::Invariant)?;
                work.plan.insert(
                    work.pending_plan
                        .take()
                        .ok_or(LivingDeterministicFaultV2::Invariant)?,
                );
                self.stage = if matches!(
                    self.queue[self.revision_index]
                        .revision
                        .command
                        .operations()[self.operation_index],
                    LivingCreatorOperationV2::ForceWar { .. }
                ) {
                    AgreementPlan
                } else {
                    OccupationPlan
                };
                return Ok(true);
            }
            DiplomacyFilter => {
                let work = self
                    .diplomacy_work
                    .as_mut()
                    .ok_or(LivingDeterministicFaultV2::Invariant)?;
                let filter = work
                    .filter
                    .as_mut()
                    .ok_or(LivingDeterministicFaultV2::Invariant)?;
                match filter {
                    QueuedDiplomacyFilter::Agreement(job) => {
                        if !job.ready() {
                            return job.visit();
                        }
                        self.state.agreements = job
                            .take_ready_result()
                            .map_err(|_| LivingDeterministicFaultV2::Invariant)?;
                        self.stage = if work.pair.is_some() {
                            WarInsertion
                        } else {
                            DiplomacyFinish
                        };
                    }
                    QueuedDiplomacyFilter::War(job) => {
                        if !job.ready() {
                            return job.visit();
                        }
                        self.state.wars = job
                            .take_ready_result()
                            .map_err(|_| LivingDeterministicFaultV2::Invariant)?;
                        self.index = 0;
                        self.stage = OccupationPlan;
                    }
                    QueuedDiplomacyFilter::Occupation(job) => {
                        if !job.ready() {
                            return job.visit();
                        }
                        self.state.occupations = job
                            .take_ready_result()
                            .map_err(|_| LivingDeterministicFaultV2::Invariant)?;
                        self.stage = DiplomacyFinish;
                    }
                }
                self.retirement.start(work.filter.take());
                return Ok(true);
            }
            WarInsertion => {
                let pair = self
                    .diplomacy_work
                    .as_ref()
                    .and_then(|work| work.pair)
                    .ok_or(LivingDeterministicFaultV2::Invariant)?;
                let LivingCreatorOperationV2::ForceWar {
                    declarer,
                    start_tick,
                    end_tick,
                    ..
                } = &self.queue[self.revision_index]
                    .revision
                    .command
                    .operations()[self.operation_index]
                else {
                    return Err(LivingDeterministicFaultV2::Invariant.into());
                };
                let row = LivingWarV2 {
                    participant_a: pair.0,
                    participant_b: pair.1,
                    declarer: self.resolve(declarer)?,
                    start_tick: *start_tick,
                    end_tick: *end_tick,
                };
                self.insertion = Some(QueuedCreatorInsertion::War(
                    super::step_job::LivingSortedInsertionJobV2::begin_replacement(
                        std::mem::take(&mut self.state.wars),
                        row,
                    ),
                ));
                self.stage = Insertion;
                return Ok(true);
            }
            OrderReservations => {
                let work = self
                    .diplomacy_work
                    .as_mut()
                    .ok_or(LivingDeterministicFaultV2::Invariant)?;
                let fleet = work.fleet.ok_or(LivingDeterministicFaultV2::Invariant)?;
                if let Some(row) = self.state.hull_jobs.as_slice().get(self.index) {
                    self.index += 1;
                    if row.target_fleet == Some(fleet) {
                        return Err(self
                            .replay_fault(LivingCommandRejectionV2::ReferencedTarget)
                            .into());
                    }
                    return Ok(true);
                }
                let Some(position) = self
                    .state
                    .fleets
                    .as_slice()
                    .binary_search_by_key(&fleet, |row| row.id)
                    .ok()
                else {
                    return Err(self
                        .replay_fault(LivingCommandRejectionV2::MissingTarget {
                            operation: self.operation_index,
                            reference: "fleet",
                        })
                        .into());
                };
                work.fleet_index = position;
                self.stage = OrderPrepare;
                return Ok(true);
            }
            OrderPrepare => {
                let LivingCreatorOperationV2::SetFleetOrder {
                    kind,
                    target_world,
                    not_before_tick,
                    lane_path,
                    ..
                } = &self.queue[self.revision_index]
                    .revision
                    .command
                    .operations()[self.operation_index]
                else {
                    return Err(LivingDeterministicFaultV2::Invariant.into());
                };
                if let Some(kind) = kind {
                    let target_world = target_world
                        .as_ref()
                        .map(|target| self.resolve(target))
                        .transpose()?;
                    let work = self
                        .diplomacy_work
                        .as_mut()
                        .ok_or(LivingDeterministicFaultV2::Invariant)?;
                    work.order = Some(LivingFleetOrderV2 {
                        kind: *kind,
                        target_world,
                        not_before_tick: *not_before_tick,
                        lane_path: Vec::new(),
                    });
                    work.path_storage = Some(super::step_job::LivingVecStorageJobV2::begin_empty(
                        lane_path.len(),
                    ));
                    self.index = 0;
                    self.stage = OrderPath;
                } else {
                    self.stage = OrderInstall;
                }
                return Ok(true);
            }
            OrderPath => {
                let work = self
                    .diplomacy_work
                    .as_mut()
                    .ok_or(LivingDeterministicFaultV2::Invariant)?;
                if let Some(storage) = &mut work.path_storage {
                    if !storage.ready() {
                        let result = storage.poll(1);
                        let visits = storage.last_poll_visits();
                        result.map_err(Self::recovery_storage_error)?;
                        return Ok(visits != 0);
                    }
                    work.order
                        .as_mut()
                        .ok_or(LivingDeterministicFaultV2::Invariant)?
                        .lane_path = storage
                        .take_ready_result()
                        .map_err(Self::recovery_storage_error)?;
                    self.retirement.start(work.path_storage.take());
                    return Ok(true);
                }
                if work.pending_path.is_some() {
                    super::step_job::LivingVecStorageJobV2::commit_one(
                        &mut work
                            .order
                            .as_mut()
                            .ok_or(LivingDeterministicFaultV2::Invariant)?
                            .lane_path,
                        &mut work.pending_path,
                    )
                    .map_err(Self::recovery_storage_error)?;
                    return Ok(true);
                }
                let LivingCreatorOperationV2::SetFleetOrder { lane_path, .. } = &self.queue
                    [self.revision_index]
                    .revision
                    .command
                    .operations()[self.operation_index]
                else {
                    return Err(LivingDeterministicFaultV2::Invariant.into());
                };
                if let Some(target) = lane_path.get(self.index) {
                    let id = self.resolve(target)?;
                    self.diplomacy_work
                        .as_mut()
                        .ok_or(LivingDeterministicFaultV2::Invariant)?
                        .pending_path = Some(id);
                    self.index += 1;
                    return Ok(true);
                }
                self.stage = OrderInstall;
            }
            OrderInstall => {
                let work = self
                    .diplomacy_work
                    .as_mut()
                    .ok_or(LivingDeterministicFaultV2::Invariant)?;
                let row = self
                    .state
                    .fleets
                    .record_at_mut(work.fleet_index)
                    .filter(|row| Some(row.id) == work.fleet)
                    .ok_or(LivingDeterministicFaultV2::Invariant)?;
                // Prepare the complete new order before replacing, retaining only
                // the displaced order. Unrelated original hull storage stays put.
                work.retired_order = std::mem::replace(&mut row.order, work.order.take());
                self.stage = DiplomacyFinish;
                return Ok(true);
            }
            DiplomacyFinish => {
                self.retirement.start(self.diplomacy_work.take());
                self.operation_index += 1;
                self.stage = Operation;
                return Ok(true);
            }
            HullReassemble => {
                let id = self.hull_id.ok_or(LivingDeterministicFaultV2::Invariant)?;
                let slot = self
                    .state
                    .fleets
                    .record_at_mut(self.hull_index)
                    .filter(|row| row.id == id)
                    .ok_or(LivingDeterministicFaultV2::Invariant)?;
                let parent = self
                    .hull_parent
                    .as_mut()
                    .ok_or(LivingDeterministicFaultV2::Invariant)?;
                let output = self
                    .hull_output
                    .as_mut()
                    .ok_or(LivingDeterministicFaultV2::Invariant)?;
                std::mem::swap(&mut parent.hulls, output);
                std::mem::swap(slot, parent);
                self.retirement.start((
                    self.insertion.take(),
                    self.hull_parent.take(),
                    self.hull_output.take(),
                ));
                self.hull_id = None;
                self.operation_index += 1;
                self.stage = Operation;
                return Ok(true);
            }
            HazardDefinition => {
                let operation = &self.queue[self.revision_index]
                    .revision
                    .command
                    .operations()[self.operation_index];
                let LivingCreatorOperationV2::EditHazard { definition, .. } = operation else {
                    return Err(LivingDeterministicFaultV2::Invariant.into());
                };
                let Some(row) = self.catalog.hazards().get(self.index) else {
                    return Err(self
                        .replay_fault(LivingCommandRejectionV2::Projection(
                            LivingValidationErrorV2::Range {
                                field: "hazard definition",
                            },
                        ))
                        .into());
                };
                self.index += 1;
                if row.id == *definition {
                    self.stage = HazardEdit;
                }
                return Ok(true);
            }
            HazardEdit => {
                let operation = &self.queue[self.revision_index]
                    .revision
                    .command
                    .operations()[self.operation_index];
                let LivingCreatorOperationV2::EditHazard {
                    target,
                    lane,
                    definition,
                    start_tick,
                    end_tick,
                } = operation
                else {
                    return Err(LivingDeterministicFaultV2::Invariant.into());
                };
                let id = self.resolve(target)?;
                if self
                    .state
                    .hazards
                    .as_slice()
                    .binary_search_by_key(&id, |row| row.id)
                    .is_err()
                {
                    return Err(self
                        .replay_fault(LivingCommandRejectionV2::MissingTarget {
                            operation: self.operation_index,
                            reference: "hazard",
                        })
                        .into());
                }
                let lane = self.resolve(lane)?;
                let row = self.state.hazards.find_mut(&id).expect("located hazard");
                row.lane = lane;
                row.definition = definition.clone();
                row.start_tick = *start_tick;
                row.end_tick = *end_tick;
                self.operation_index += 1;
                self.stage = Operation;
                return Ok(true);
            }
            ColonyConstructionReservations | ColonyHullReservations => {
                let operation = self
                    .colony_operation
                    .ok_or(LivingDeterministicFaultV2::Invariant)?;
                let row = if matches!(self.stage, ColonyConstructionReservations) {
                    self.state
                        .construction_jobs
                        .as_slice()
                        .get(self.index)
                        .map(|row| (row.world, row.owner))
                } else {
                    self.state
                        .hull_jobs
                        .as_slice()
                        .get(self.index)
                        .map(|row| (row.world, row.owner))
                };
                if let Some((world, owner)) = row {
                    self.index += 1;
                    if operation.blocks(world, owner) {
                        return Err(self.replay_fault(operation.rejection()).into());
                    }
                    return Ok(true);
                }
                self.index = 0;
                self.stage = if matches!(self.stage, ColonyConstructionReservations) {
                    ColonyHullReservations
                } else {
                    ColonyMutation
                };
            }
            ColonyMutation => {
                let operation = self
                    .colony_operation
                    .ok_or(LivingDeterministicFaultV2::Invariant)?;
                match operation {
                    QueuedColonyOperation::SetOwner { world, owner } => {
                        let accepted_sequence =
                            self.queue[self.revision_index].accepted.accepted_sequence;
                        self.state
                            .worlds
                            .find_mut(&world)
                            .ok_or_else(|| LivingDeterministicFaultV2::Replay {
                                accepted_sequence,
                                reason: Box::new(LivingCommandRejectionV2::MissingTarget {
                                    operation: self.operation_index,
                                    reference: "world",
                                }),
                            })?
                            .owner = owner;
                        self.stage = ColonyOwnerEdit;
                    }
                    QueuedColonyOperation::Remove { world } => {
                        self.colony_removal =
                            Some(super::step_job::LivingColonyRemovalJobV2::begin(
                                std::mem::take(&mut self.state.colonies),
                                world,
                            ));
                        self.colony_operation = None;
                        self.colony_changed = true;
                        self.index = 0;
                        self.stage = ColonyRemoval;
                    }
                }
                return Ok(true);
            }
            ColonyOwnerEdit => {
                let Some(QueuedColonyOperation::SetOwner { world, owner }) = self.colony_operation
                else {
                    return Err(LivingDeterministicFaultV2::Invariant.into());
                };
                if let Some(owner) = owner {
                    if let Some(colony) = self.state.colonies.find_mut(&world) {
                        colony.owner = owner;
                    }
                    self.stage = StatusOwners;
                } else {
                    self.colony_removal = Some(super::step_job::LivingColonyRemovalJobV2::begin(
                        std::mem::take(&mut self.state.colonies),
                        world,
                    ));
                    self.stage = ColonyRemoval;
                }
                self.colony_operation = None;
                self.colony_changed = true;
                self.index = 0;
                return Ok(true);
            }
            ColonyCreateInsertion => {
                let (world, owner, [energy_next_due, ore_next_due, fallback_next_due]) = self
                    .colony_create
                    .ok_or(LivingDeterministicFaultV2::Invariant)?;
                let colony = LivingColonyV2 {
                    world,
                    owner,
                    hub: LivingHubV2 {
                        energy_next_due,
                        ore_next_due,
                        fallback_next_due,
                        blocked_reasons: LivingSortedVecV2::default(),
                    },
                };
                self.insertion = Some(QueuedCreatorInsertion::Colony(
                    super::step_job::LivingSortedInsertionJobV2::begin_replacement(
                        std::mem::take(&mut self.state.colonies),
                        colony,
                    ),
                ));
                self.colony_create = None;
                self.stage = Insertion;
                return Ok(true);
            }
            ColonyRemoval => {
                let child = self
                    .colony_removal
                    .as_mut()
                    .ok_or(LivingDeterministicFaultV2::Invariant)?;
                if !child.ready() {
                    return child.visit();
                }
                self.state.colonies = child
                    .take_ready_result()
                    .map_err(|_| LivingDeterministicFaultV2::Invariant)?;
                self.retirement.start(self.colony_removal.take());
                self.stage = StatusOwners;
                return Ok(true);
            }
            StatusOwners | RecoveryOwners => {
                if let Some(row) = self.state.colonies.as_slice().get(self.index) {
                    self.index += 1;
                    self.colony_owners.insert(row.owner);
                    return Ok(true);
                }
                self.index = 0;
                self.stage = if matches!(self.stage, StatusOwners) {
                    StatusCivilizations
                } else {
                    RecoveryActor
                };
            }
            StatusCivilizations => {
                if let Some(row) = self.state.civilizations.record_at_mut(self.index) {
                    self.index += 1;
                    row.status = if self.colony_owners.contains(&row.id) {
                        LivingCivilizationStatusV2::Active
                    } else {
                        LivingCivilizationStatusV2::Dormant
                    };
                    return Ok(true);
                }
                self.retirement
                    .start(std::mem::take(&mut self.colony_owners));
                self.operation_index += 1;
                self.stage = Operation;
                return Ok(true);
            }
            RecoveryPrepare => {
                let child = self
                    .recovery_storage
                    .as_mut()
                    .ok_or(LivingDeterministicFaultV2::Invariant)?;
                if !child.ready() {
                    let result = child.poll(1);
                    let visits = child.last_poll_visits();
                    result.map_err(Self::recovery_storage_error)?;
                    return Ok(visits != 0);
                }
                self.events.creator_recoveries = child
                    .take_ready_result()
                    .map_err(Self::recovery_storage_error)?;
                self.retirement.start(self.recovery_storage.take());
                self.index = 0;
                self.stage = RecoveryOwners;
                return Ok(true);
            }
            RecoveryActor => {
                if let Some(actor) = self.dormant.pop_first() {
                    if self.colony_owners.contains(&actor)
                        && self
                            .state
                            .civilizations
                            .as_slice()
                            .binary_search_by_key(&actor, |row| row.id)
                            .ok()
                            .is_some_and(|index| {
                                self.state.civilizations.as_slice()[index].status
                                    == LivingCivilizationStatusV2::Active
                            })
                    {
                        self.pending_recovery = Some(LivingCreatorRecoveryCandidateV2 {
                            revision: self.queue[self.revision_index].revision.id,
                            actor,
                        });
                        self.stage = RecoveryEmit;
                    }
                    return Ok(true);
                }
                self.retirement
                    .start(std::mem::take(&mut self.colony_owners));
                self.stage = EmitCreator;
                return Ok(true);
            }
            RecoveryEmit => {
                super::step_job::LivingVecStorageJobV2::commit_one(
                    &mut self.events.creator_recoveries,
                    &mut self.pending_recovery,
                )
                .map_err(Self::recovery_storage_error)?;
                self.stage = RecoveryActor;
                return Ok(true);
            }
            EmitCreator => {
                self.append = Some(
                    super::receipt::LivingPendingEventAppendJobV2::begin_ordinary(
                        std::mem::take(&mut self.events),
                        LivingEventProvenanceV2::Creator {
                            revision: self.queue[self.revision_index].revision.id,
                        },
                        LivingEventKindV2::CreatorIntervention,
                    ),
                );
                self.stage = AppendCreator;
                return Ok(true);
            }
            AppendCreator => {
                let child = self
                    .append
                    .as_mut()
                    .ok_or(LivingDeterministicFaultV2::Invariant)?;
                if child.status() != super::receipt::LivingPendingEventAppendStatusV2::Ready {
                    let result = child.poll(1);
                    // The outer visit debit covers exactly this child's one actual visit.
                    let visits = child.last_poll_visits();
                    result.map_err(Self::append_error)?;
                    return Ok(visits != 0);
                }
                let (events, _) = child.take_ready_result().map_err(Self::append_error)?;
                self.events = events;
                self.retirement.start(self.append.take());
                self.state.accepted_sequence = self.queue[self.revision_index].revision.ordinal;
                self.stage = if self.effect_work.is_some() {
                    EffectsStart
                } else {
                    FinishRevision
                };
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
    ) -> Result<
        (LivingGalaxyStateV2, LivingPendingEventsV2),
        super::step_job::LivingLifecycleJobErrorV2,
    > {
        if self.delivered || !self.retirement.is_empty() {
            return Err(LivingDeterministicFaultV2::Invariant.into());
        }
        if matches!(self.stage, QueuedCreatorStage::Ready) {
            return self.take_ready_result().map_err(Into::into);
        }
        if !matches!(self.stage, QueuedCreatorStage::NeedsSynchronous) {
            return Err(LivingDeterministicFaultV2::Invariant.into());
        }
        // Explicit compatibility bridge. Unsupported gameplay remains synchronous;
        // original prefix/suffix effects and every bounded child remain parent-owned.
        // Resume the cursor between unsupported operations so prefix work is never replayed.
        loop {
            if matches!(self.stage, QueuedCreatorStage::NeedsSynchronous) {
                let queued = &self.queue[self.revision_index];
                let revision = &queued.revision;
                let resolve = |target: &LivingCommandTargetV2| match *target {
                    LivingCommandTargetV2::Existing { id } => id,
                    LivingCommandTargetV2::Local { local } => revision
                        .command
                        .operations()
                        .iter()
                        .find_map(|operation| {
                            (operation.declared_local() == Some(local))
                                .then(|| {
                                    operation.created_kind().map(|kind| {
                                        super::ids::creator_entity_id_v2(
                                            revision.id,
                                            kind.ordinal(),
                                            local,
                                        )
                                    })
                                })
                                .flatten()
                        })
                        .unwrap_or(LivingEntityIdV2([0; 16])),
                };
                let work = self.effect_work.get_or_insert_with(Default::default);
                rules::apply_creator_operation(
                    &mut self.state,
                    revision.tick,
                    self.operation_index,
                    &revision.command.operations()[self.operation_index],
                    &resolve,
                    &mut work.suffix,
                    &self.catalog,
                )
                .map_err(|reason| LivingDeterministicFaultV2::Replay {
                    accepted_sequence: queued.accepted.accepted_sequence,
                    reason: Box::new(reason),
                })?;
                // Unsupported operations can change colonies/status; the original
                // fallback stages candidates from the original dormant set regardless.
                self.colony_changed = true;
                self.stage = QueuedCreatorStage::BridgeEffectsStart;
            }
            match self.poll(1024) {
                Ok(LivingStepProgressV2::NeedsSynchronousCompletion) => {
                    return self.take_ready_result().map_err(Into::into);
                }
                Ok(_) => {}
                Err(error) => return Err(error),
            }
        }
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
            preparation_capacity: false,
            production_fault: None,
            production_handoff: false,
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
            repair_exclusions: Default::default(),
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
pub(crate) mod tests {
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
    fn staged_step_fleet_creator_prefix_preserves_local_ids_and_recovery() {
        let (mut base, actor, world) = recovery_fixture();
        let fleet = base
            .state
            .fleets
            .as_slice()
            .iter()
            .find(|fleet| {
                matches!(
                    fleet.location,
                    super::super::model::LivingFleetLocationV2::Docked { .. }
                ) && !base
                    .state
                    .hull_jobs
                    .as_slice()
                    .iter()
                    .any(|job| job.target_fleet == Some(fleet.id))
            })
            .unwrap()
            .clone();
        let super::super::model::LivingFleetLocationV2::Docked { world: dock } = fleet.location
        else {
            unreachable!()
        };
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
                LivingCreatorOperationV2::EditFleet {
                    target: LivingCommandTargetV2::Existing { id: fleet.id },
                    owner: LivingCommandTargetV2::Existing { id: fleet.owner },
                    world: LivingCommandTargetV2::Existing { id: dock },
                },
            ],
        );
        let mut reference = base.clone();
        reference.step().unwrap();
        for budget in [1, 64, 1024] {
            let mut authority = base.clone();
            let mut job = authority.begin_step().unwrap();
            let mut reached_completion = false;
            for _ in 0..100_000 {
                let progress = job.poll(budget).unwrap();
                assert!(job.last_poll_visits() <= budget);
                if progress == LivingStepProgressV2::NeedsSynchronousCompletion {
                    reached_completion = true;
                    break;
                }
                assert_ne!(
                    progress,
                    LivingStepProgressV2::NeedsSynchronousCreatorOperation
                );
            }
            assert!(reached_completion, "finite creator completion guard");
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
    fn staged_step_keeps_scheduled_box_and_resident_on_arrival_checked_failure() {
        let mut authority = scheduled_staged_fixture();
        let state = Arc::make_mut(&mut authority.state);
        state.wars.record_at_mut(0).unwrap().end_tick = LivingTickV2(100);
        let receiver = state.civilizations.as_slice()[1].id;
        let destination = state.worlds.as_slice()[1].id;
        let cargo = state.shipments.record_at_mut(0).unwrap();
        cargo.intended_receiver = receiver;
        cargo.destination_world = destination;
        cargo.dispatch_kind = Some(super::super::model::LivingRouteKindV2::Aid);
        cargo.disposition = super::super::model::LivingShipmentDispositionV2::WaitingForStorage;
        cargo.departure_tick = LivingTickV2(10);
        cargo.arrival_tick = LivingTickV2(10);
        let original = authority.state.as_ref().clone();
        let mut job = Box::new(authority.begin_step().unwrap());
        let step_box = &*job as *const _;
        let mut scheduled_box = None;
        let error = loop {
            match job.poll(1) {
                Ok(LivingStepProgressV2::Pending) => {
                    if let Some(scheduled) = &job.scheduled {
                        scheduled_box.get_or_insert(&**scheduled as *const _);
                    }
                }
                Err(error) => break error,
                other => panic!("unexpected invalid Arrival progress: {other:?}"),
            }
        };
        assert_eq!(
            error,
            LivingStepJobErrorV2::Fault(LivingDeterministicFaultV2::Arithmetic)
        );
        assert!(job.last_poll_visits() > 0);
        assert_eq!(&*job as *const _, step_box);
        assert_eq!(
            job.scheduled.as_ref().map(|child| &**child as *const _),
            scheduled_box
        );
        assert_eq!(authority.state.as_ref(), &original);
        assert!(authority.fault.is_none());
        assert!(authority.queue.is_empty());
        assert_eq!(job.poll(0), Err(LivingStepJobErrorV2::Budget));
        assert_eq!(job.poll(1), Err(error));
        assert_eq!(job.last_poll_visits(), 0);
        let mut retirement = job.into_boxed_retirement(
            super::super::simulation::LivingPublicationRetirementPolicyV2::ReleaseHandle,
        );
        while retirement.poll(64).unwrap()
            != super::super::simulation::LivingPublicationRetirementProgressV2::Ready
        {}
    }

    #[test]
    fn staged_arrival_failures_reach_each_trigger_budget_with_original_step_owners() {
        let mut invalid = scheduled_staged_fixture();
        {
            let state = Arc::make_mut(&mut invalid.state);
            state.wars.record_at_mut(0).unwrap().end_tick = LivingTickV2(100);
            let receiver = state.civilizations.as_slice()[1].id;
            let destination = state.worlds.as_slice()[1].id;
            let cargo = state.shipments.record_at_mut(0).unwrap();
            cargo.intended_receiver = receiver;
            cargo.destination_world = destination;
            cargo.dispatch_kind = Some(super::super::model::LivingRouteKindV2::Aid);
            cargo.disposition = super::super::model::LivingShipmentDispositionV2::WaitingForStorage;
            cargo.departure_tick = LivingTickV2(10);
            cargo.arrival_tick = LivingTickV2(10);
        }
        let mut probe = invalid.begin_step().unwrap();
        loop {
            if probe.poll(1).is_err() {
                break;
            }
        }
        let arithmetic_position = probe.total_visits();
        for category in 0..3 {
            let mut totals = Vec::new();
            for (trigger, retirement_schedule) in [
                (1, &[1][..]),
                (64, &[64][..]),
                (7, &[1, 64, 7, 1024][..]),
                (1024, &[1024][..]),
            ] {
                let authority = if category == 0 {
                    invalid.clone()
                } else {
                    scheduled_staged_fixture()
                };
                let original = authority.state.as_ref().clone();
                let mut job = Box::new(authority.begin_step().unwrap());
                let job_pointer = std::ptr::from_ref(job.as_ref());
                if category == 0 {
                    for _ in 1..arithmetic_position {
                        assert_eq!(job.poll(1), Ok(LivingStepProgressV2::Pending));
                    }
                } else {
                    let mut guard = 0;
                    while !job.retirement.is_empty()
                        || !job.scheduled.as_mut().is_some_and(|child| {
                            child.inject_arrival_capacity_failure(category == 2)
                        })
                    {
                        assert_eq!(job.poll(1), Ok(LivingStepProgressV2::Pending));
                        guard += 1;
                        assert!(guard < 20000);
                    }
                }
                let scheduled_pointer =
                    std::ptr::from_ref(job.scheduled.as_ref().unwrap().as_ref());
                let before = job.total_visits();
                let expected = if category == 0 {
                    LivingStepJobErrorV2::Fault(LivingDeterministicFaultV2::Arithmetic)
                } else {
                    LivingStepJobErrorV2::PreparationCapacity
                };
                assert_eq!(job.poll(trigger), Err(expected.clone()));
                assert_eq!(job.last_poll_visits(), 1);
                assert_eq!(job.total_visits(), before + 1);
                assert_eq!(std::ptr::from_ref(job.as_ref()), job_pointer);
                assert_eq!(
                    std::ptr::from_ref(job.scheduled.as_ref().unwrap().as_ref()),
                    scheduled_pointer
                );
                assert_eq!(job.poll(trigger), Err(expected));
                assert_eq!(job.last_poll_visits(), 0);
                assert_eq!(authority.state.as_ref(), &original);
                assert!(authority.fault.is_none());
                assert!(authority.queue.is_empty());
                let mut retirement =
                    job.into_boxed_retirement(LivingPublicationRetirementPolicyV2::ReleaseHandle);
                let mut i = 0;
                loop {
                    let budget = retirement_schedule[i % retirement_schedule.len()];
                    i += 1;
                    if retirement.poll(budget).unwrap()
                        == LivingPublicationRetirementProgressV2::Ready
                    {
                        break;
                    }
                    assert!(retirement.last_poll_visits() > 0);
                }
                totals.push(retirement.total_visits());
            }
            assert!(totals.iter().all(|n| *n == totals[0]));
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
            repair_exclusions: Default::default(),
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
    pub(crate) fn combat_cursor_fixture() -> LivingGalaxyAuthorityV2 {
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
            repair_exclusions: Default::default(),
        })
        .unwrap();
        let literal_id = |byte: u8| {
            let mut bytes = [0; 16];
            bytes[0] = byte;
            LivingEntityIdV2(bytes)
        };
        let source_hulls = authority.state.as_ref().fleets.as_slice()[0]
            .hulls
            .as_slice();
        let selected_escort = source_hulls[0].id;
        let later_escort = source_hulls[2].id;
        assert_eq!(selected_escort, literal_id(15));
        assert_eq!(later_escort, literal_id(17));
        assert_eq!(source_hulls[0].definition.as_str(), "escort");
        assert_eq!(source_hulls[2].definition.as_str(), "escort");
        assert_eq!(source_hulls[0].hit_points, 1);
        assert_eq!(source_hulls[2].hit_points, 1);
        assert!(
            reference.fleets.as_slice()[0]
                .hulls
                .as_slice()
                .iter()
                .all(|hull| hull.id != selected_escort)
        );
        assert_eq!(
            reference.fleets.as_slice()[0]
                .hulls
                .as_slice()
                .iter()
                .find(|hull| hull.id == later_escort)
                .unwrap()
                .hit_points,
            1,
        );
        assert_eq!(
            reference.fleets.as_slice()[1].hulls.as_slice()[0].next_repair,
            Some(LivingTickV2(20)),
        );
        // The escort killed by the hostile 2-damage hit already fired. Three
        // living-at-round-start escorts contribute 2 each and the battery 4:
        // 500 - (2 + 2 + 2 + 4) = 490 simultaneous remaining HP.
        assert_eq!(
            reference.fleets.as_slice()[1].hulls.as_slice()[0].id,
            literal_id(19)
        );
        assert_eq!(
            reference.fleets.as_slice()[1].hulls.as_slice()[0].hit_points,
            490
        );
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
            assert_eq!(events, reference_events);
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
        // Axes mutate the unpublished candidate directly; missing definitions
        // are a private malformed-state control, not an admitted public state.
        for axis in 0..5 {
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
                3 => {
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
                _ => {
                    state
                        .fleets
                        .record_at_mut(0)
                        .unwrap()
                        .hulls
                        .record_at_mut(0)
                        .unwrap()
                        .definition = LivingSlugV2::new("missing_hull_definition").unwrap();
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
                repair_exclusions: Default::default(),
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
            assert_eq!(events, reference_events, "axis {axis}");
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
    #[test]
    fn combat_private_near_overflow_fault_matches_partial_reference_in_every_partition() {
        use crate::living::model::LivingFleetLocationV2;
        use crate::living::step_job::{LivingCombatJobV2, LivingLifecycleJobErrorV2};
        let authority = combat_cursor_fixture();
        let tick = LivingTickV2(u64::MAX - 5); // Divisible by ten; tick + ten overflows.
        let mut source = authority.state.as_ref().clone();
        source.wars.record_at_mut(0).unwrap().end_tick = LivingTickV2(u64::MAX);
        let other_world = source.worlds.as_slice()[1].id;
        assert_eq!(
            source.worlds.as_slice()[1].owner,
            Some(source.civilizations.as_slice()[1].id)
        );
        source.fleets.record_at_mut(2).unwrap().location =
            LivingFleetLocationV2::Docked { world: other_world };
        source.facilities.record_at_mut(0).unwrap().world = other_world;
        let old_repair = Some(LivingTickV2(20));
        source
            .fleets
            .record_at_mut(1)
            .unwrap()
            .hulls
            .record_at_mut(0)
            .unwrap()
            .next_repair = old_repair;

        let mut reference = source.clone();
        let mut reference_events = LivingPendingEventsV2::default();
        let reference_fault = rules::reference_combat_for_cursor_test(&mut LivingStepContextV2 {
            catalog: &authority.catalog,
            genesis_seed: authority.genesis_seed,
            branch: authority.branch,
            tick,
            state: &mut reference,
            events: &mut reference_events,
            arrivals: Vec::new(),
            repair_exclusions: Default::default(),
        });
        assert_eq!(reference_fault, Err(LivingDeterministicFaultV2::Arithmetic));
        assert_eq!(reference_events.payloads().len(), 1);
        assert_eq!(
            reference_events.payloads()[0].kind,
            LivingEventKindV2::CombatRound
        );
        // Fixed algebra from the built-in definitions: the destroyed first
        // escort fired for 2; the second escort fired for 2; the B escort's
        // saved clock bypasses arithmetic at 500 - 4 = 496; its 2-damage
        // return kills A's one-HP first escort. The other-world escort takes
        // the B battery from 40 to 38 before its first repair tick overflows.
        assert_eq!(
            reference.fleets.as_slice()[1].hulls.as_slice()[0].hit_points,
            496
        );
        assert_eq!(
            reference.fleets.as_slice()[1].hulls.as_slice()[0].next_repair,
            old_repair
        );
        assert_eq!(
            reference.fleets.as_slice()[0].hulls.as_slice()[0].hit_points,
            0
        );
        assert_eq!(reference.facilities.as_slice()[0].hit_points, 38);
        assert_eq!(reference.facilities.as_slice()[0].next_repair, None);
        assert_eq!(
            reference.fleets.as_slice()[2].hulls.as_slice()[0].hit_points,
            1
        );

        for budgets in [&[1][..], &[64][..], &[1024][..], &[1, 7, 64, 1024][..]] {
            let mut job = LivingCombatJobV2::begin(
                &authority.catalog,
                source.clone(),
                LivingPendingEventsV2::default(),
                tick,
            );
            let mut failed = false;
            for turn in 0..10_000 {
                let budget = budgets[turn % budgets.len()];
                match job.poll(budget) {
                    Err(LivingLifecycleJobErrorV2::Fault(
                        LivingDeterministicFaultV2::Arithmetic,
                    )) => {
                        assert!((1..=budget).contains(&job.last_poll_visits()));
                        failed = true;
                        break;
                    }
                    Ok(crate::living::step_job::LivingLifecycleProgressV2::Pending) => {
                        assert!(job.last_poll_visits() <= budget);
                    }
                    other => panic!("unexpected combat fault partition outcome: {other:?}"),
                }
            }
            assert!(failed, "fault not reached");
            let (partial, collector) = job.fault_snapshot_for_test();
            assert_eq!(partial, &reference);
            assert_eq!(collector, &reference_events);
            assert_eq!(
                job.poll(1),
                Err(LivingLifecycleJobErrorV2::Fault(
                    LivingDeterministicFaultV2::Arithmetic
                ))
            );
            assert_eq!(job.last_poll_visits(), 0);
        }
    }
}

#[cfg(test)]
mod preparation_capacity_tests {
    use super::*;
    fn enqueue(authority: &mut LivingGalaxyAuthorityV2, label: &str) {
        let cursor = authority.published_cursor();
        authority
            .submit(LivingCommandEnvelopeV2 {
                expected_committed_revision: cursor.committed_revision,
                expected_tick: cursor.tick,
                expected_pending_sequence: cursor.pending_sequence,
                mode: LivingCommandModeV2::Running,
                command: LivingCommandV2::CreatorBatch {
                    operations: vec![LivingCreatorOperationV2::CreateSystem {
                        local: 0,
                        name: super::super::catalog::LivingNameV2::new(label).unwrap(),
                    }],
                },
            })
            .unwrap();
    }
    fn drain(mut job: LivingPublicationRetirementJobV2, schedule: &[usize]) -> u64 {
        let mut n = 0;
        loop {
            let budget = schedule[n % schedule.len()];
            let result = job.poll(budget).unwrap();
            assert!(job.last_poll_visits() <= budget);
            n += 1;
            if result == LivingPublicationRetirementProgressV2::Ready {
                break;
            }
            assert_ne!(
                result,
                LivingPublicationRetirementProgressV2::AwaitingExternalOwners
            );
            assert!(n < 100000);
        }
        let total = job.total_visits();
        assert_eq!(
            job.poll(1),
            Ok(LivingPublicationRetirementProgressV2::Ready)
        );
        assert_eq!(job.last_poll_visits(), 0);
        assert_eq!(job.total_visits(), total);
        total
    }
    #[test]
    fn preparation_capacity_original_step_scheduled_lifecycle_boxes_and_authority_are_retained() {
        for event_storage in [false, true] {
            let mut totals = Vec::new();
            for schedule in [&[1][..], &[64][..], &[1024][..], &[1, 64, 7, 1024][..]] {
                let mut authority = tests::scheduled_staged_fixture();
                enqueue(&mut authority, "Accepted before capacity failure");
                let before = authority.state.canonical_bytes().unwrap();
                let queue = Arc::clone(&authority.queue);
                let capture = authority.capture_boundary().unwrap();
                let tick = authority.state.tick;
                let mut job = Box::new(authority.begin_step().unwrap());
                let step_pointer = &*job as *const _;
                let mut injection_reached = false;
                for _ in 0..100_000 {
                    if job
                        .scheduled
                        .as_mut()
                        .is_some_and(|child| child.inject_capacity_failure(event_storage))
                    {
                        injection_reached = true;
                        break;
                    }
                    assert_eq!(job.poll(1).unwrap(), LivingStepProgressV2::Pending);
                }
                assert!(
                    injection_reached,
                    "finite scheduled capacity injection guard"
                );
                let scheduled_pointer = &**job.scheduled.as_ref().unwrap() as *const _;
                let lifecycle_pointer = job
                    .scheduled
                    .as_ref()
                    .unwrap()
                    .lifecycle_identity()
                    .unwrap();
                // Output injection occurs before the delivered creator is retired.
                // Original map: Creator fixed23 + state41 + events16 + locals1 +
                // dormant1 + insertion1 + lane7 + hull parent1 + hull output1 +
                // append1 =93. Four absent E2 fields each add action1 + payload1 +
                // node Box1, giving105. E3 adds3 and E4 adds6, giving114.
                // Some3 + lane Box1 + failed storage1 =119.
                // Event injection follows cleanup, so only failure1. Independent
                // scheduled-capacity-addendum-v1 pins both before this correction.
                let expected_failure_visits = if event_storage { 1 } else { 119 };
                let failure_start = job.total_visits();
                let mut failure_reached = false;
                for _ in 0..100_000 {
                    let previous = job.total_visits();
                    let result = job.poll(64);
                    assert_eq!(job.total_visits() - previous, job.last_poll_visits() as u64);
                    assert!(job.last_poll_visits() >= 1 && job.last_poll_visits() <= 64);
                    assert!(job.total_visits() - failure_start <= expected_failure_visits);
                    assert_eq!(&*job as *const _, step_pointer);
                    assert_eq!(
                        &**job.scheduled.as_ref().unwrap() as *const _,
                        scheduled_pointer
                    );
                    assert_eq!(
                        job.scheduled.as_ref().unwrap().lifecycle_identity(),
                        Some(lifecycle_pointer)
                    );
                    match result {
                        Ok(LivingStepProgressV2::Pending) => {}
                        Err(LivingStepJobErrorV2::PreparationCapacity) => {
                            failure_reached = true;
                            break;
                        }
                        _ => panic!("capacity failure must precede any suffix"),
                    }
                }
                assert!(failure_reached, "finite scheduled capacity failure guard");
                assert_eq!(job.total_visits() - failure_start, expected_failure_visits);
                assert!(job.preparation_capacity);
                assert!(job.fault.is_none());
                assert!(!job.needs_synchronous_completion());
                assert!(job.conflict.is_none());
                assert!(job.completed_output.is_none());
                assert_eq!(
                    job.scheduled.as_ref().unwrap().lifecycle_identity(),
                    Some(lifecycle_pointer)
                );
                assert_eq!(
                    &**job.scheduled.as_ref().unwrap() as *const _,
                    scheduled_pointer
                );
                assert_eq!(&*job as *const _, step_pointer);
                for budget in [1, 64, 1024] {
                    assert_eq!(
                        job.poll(budget),
                        Err(LivingStepJobErrorV2::PreparationCapacity)
                    );
                    assert_eq!(job.last_poll_visits(), 0);
                }
                for bad in [0, 1025] {
                    assert_eq!(job.poll(bad), Err(LivingStepJobErrorV2::Budget));
                    assert_eq!(job.last_poll_visits(), 0);
                }
                assert!(matches!(
                    job.take_synchronous_preparation(&authority),
                    Err(LivingStepJobErrorV2::PreparationCapacity)
                ));
                assert!(!job.delivered);
                assert!(job.fault.is_none());
                assert!(job.capture.is_present());
                assert_eq!(authority.state.canonical_bytes().unwrap(), before);
                assert!(Arc::ptr_eq(&authority.queue, &queue));
                assert_eq!(authority.queue.len(), 1);
                assert_eq!(authority.state.tick, tick);
                assert!(authority.fault.is_none());
                assert!(authority.boundary_capture_is_current(&capture));
                assert!(authority.last_receipt.is_none());
                enqueue(&mut authority, "Accepted stale tail");
                assert!(matches!(
                    job.take_synchronous_preparation(&authority),
                    Err(LivingStepJobErrorV2::Stale)
                ));
                job.cancel();
                assert!(matches!(
                    job.take_synchronous_preparation(&authority),
                    Err(LivingStepJobErrorV2::Cancelled)
                ));
                for budget in [0, 1, 1025] {
                    assert_eq!(
                        job.poll(budget),
                        Err(LivingStepJobErrorV2::PreparationCapacity)
                    );
                    assert_eq!(job.last_poll_visits(), 0);
                }
                assert_eq!(
                    job.scheduled.as_ref().unwrap().lifecycle_identity(),
                    Some(lifecycle_pointer)
                );
                totals.push(drain(
                    job.into_boxed_retirement(LivingPublicationRetirementPolicyV2::ReleaseHandle),
                    schedule,
                ));
                assert_eq!(authority.queue.len(), 2);
                assert_eq!(authority.state.tick, tick);
                assert!(authority.fault.is_none());
            }
            assert!(totals.iter().all(|total| *total == totals[0]));
        }
    }
    #[test]
    fn conflict_borrowed_extraction_preserves_local_capacity_in_step() {
        let authority = crate::living::builtin_living_sandbox_v2().unwrap();
        let before = authority.state().canonical_bytes().unwrap();
        let mut job = authority.begin_step().unwrap();
        for _ in 0..10_000 {
            if job.conflict.is_some() {
                break;
            }
            assert_eq!(job.poll(64).unwrap(), LivingStepProgressV2::Pending);
        }
        let original = &**job.conflict.as_ref().expect("conflict child") as *const _;
        // Private injected handoff control: real local capacity is exercised
        // through the Combat/Conflict poll tests before this borrowed call.
        job.conflict
            .as_mut()
            .unwrap()
            .inject_local_capacity_for_extraction();
        assert_eq!(
            job.install_production_from_conflict(),
            Err(LivingStepJobErrorV2::PreparationCapacity)
        );
        assert!(job.preparation_capacity);
        assert!(job.fault.is_none());
        assert!(job.production.is_none());
        assert_eq!(&**job.conflict.as_ref().unwrap() as *const _, original);
        assert_eq!(job.poll(64), Err(LivingStepJobErrorV2::PreparationCapacity));
        assert_eq!(job.last_poll_visits(), 0);
        assert_eq!(authority.state().canonical_bytes().unwrap(), before);
    }
    #[test]
    fn combat_child_capacity_reaches_step_with_current_and_stale_captures() {
        for (trigger, stale) in [(1, false), (64, false), (7, true), (1024, true)] {
            let mut totals = Vec::new();
            for cleanup in [&[1][..], &[64][..], &[1024][..], &[1, 7, 64, 1024][..]] {
                let mut authority = tests::combat_cursor_fixture();
                let state = Arc::make_mut(&mut authority.state);
                state.tick = LivingTickV2(9);
                for fleet_index in 0..state.fleets.as_slice().len() {
                    let fleet = state.fleets.record_at_mut(fleet_index).unwrap();
                    for hull_index in 0..fleet.hulls.as_slice().len() {
                        fleet.hulls.record_at_mut(hull_index).unwrap().hit_points = 1;
                    }
                }
                let before = authority.state().canonical_bytes().unwrap();
                let capture = authority.capture_boundary().unwrap();
                let mut job = Box::new(authority.begin_step().unwrap());
                let step_ptr = std::ptr::from_ref(&*job);
                let mut injected = false;
                for _ in 0..100_000 {
                    if job
                        .conflict
                        .as_mut()
                        .is_some_and(|parent| parent.inject_combat_piece_capacity_failure())
                    {
                        injected = true;
                        break;
                    }
                    assert_eq!(job.poll(1).unwrap(), LivingStepProgressV2::Pending);
                }
                assert!(injected, "combat child not reached through step");
                let conflict_ptr = std::ptr::from_ref(&**job.conflict.as_ref().unwrap());
                let mut before_visits = job.total_visits();
                let mut failed = false;
                for _ in 0..100_000 {
                    match job.poll(trigger) {
                        Ok(LivingStepProgressV2::Pending) => before_visits = job.total_visits(),
                        Err(LivingStepJobErrorV2::PreparationCapacity) => {
                            failed = true;
                            break;
                        }
                        other => panic!("unexpected combat continuation: {other:?}"),
                    }
                }
                assert!(failed, "combat child capacity was not propagated");
                assert_eq!(
                    job.total_visits() - before_visits,
                    job.last_poll_visits() as u64
                );
                assert!((1..=trigger).contains(&job.last_poll_visits()));
                assert!(job.preparation_capacity);
                assert!(job.fault.is_none());
                assert_eq!(std::ptr::from_ref(&*job), step_ptr);
                assert_eq!(
                    std::ptr::from_ref(&**job.conflict.as_ref().unwrap()),
                    conflict_ptr
                );
                assert_eq!(authority.state().canonical_bytes().unwrap(), before);
                assert!(authority.boundary_capture_is_current(&capture));
                if stale {
                    enqueue(&mut authority, "Accepted combat tail");
                    assert!(matches!(
                        job.take_synchronous_preparation(&authority),
                        Err(LivingStepJobErrorV2::Stale)
                    ));
                    assert_eq!(authority.capture_boundary().unwrap().pending_count(), 1);
                } else {
                    assert!(matches!(
                        job.take_synchronous_preparation(&authority),
                        Err(LivingStepJobErrorV2::PreparationCapacity)
                    ));
                }
                assert_eq!(job.poll(1), Err(LivingStepJobErrorV2::PreparationCapacity));
                assert_eq!(job.last_poll_visits(), 0);
                totals.push(drain(
                    job.into_boxed_retirement(LivingPublicationRetirementPolicyV2::ReleaseHandle),
                    cleanup,
                ));
                assert_eq!(authority.state().canonical_bytes().unwrap(), before);
                assert_eq!(
                    authority.capture_boundary().unwrap().pending_count(),
                    if stale { 1 } else { 0 }
                );
            }
            assert_eq!(totals[0], 295, "literal failed Step owner total");
            assert!(totals.iter().all(|total| *total == totals[0]));
        }
    }
    #[test]
    fn retreat_child_storage_failures_reach_step_current_stale_cancelled_and_borrowed() {
        for which in 0..3 {
            for trigger in [1, 64, 7, 1024] {
                for cleanup in [&[1][..], &[64][..], &[1024][..], &[1, 7, 64, 1024][..]] {
                    let mut authority = crate::living::builtin_living_sandbox_v2().unwrap();
                    enqueue(&mut authority, "Accepted before Retreat");
                    let state = authority.state().canonical_bytes().unwrap();
                    let queue = Arc::clone(&authority.queue);
                    let capture = authority.capture_boundary().unwrap();
                    let mut job = Box::new(authority.begin_step().unwrap());
                    let step = std::ptr::from_ref(&*job);
                    let mut injected = false;
                    for _ in 0..100_000 {
                        if job
                            .conflict
                            .as_mut()
                            .is_some_and(|c| c.inject_retreat_storage_capacity_failure(which))
                        {
                            injected = true;
                            break;
                        }
                        assert_eq!(job.poll(1).unwrap(), LivingStepProgressV2::Pending);
                    }
                    assert!(injected, "actual Retreat storage boundary");
                    let conflict = std::ptr::from_ref(&**job.conflict.as_ref().unwrap());
                    let retreat = job.conflict.as_ref().unwrap().retreat_identity().unwrap();
                    let before = job.total_visits();
                    assert_eq!(
                        job.poll(trigger),
                        Err(LivingStepJobErrorV2::PreparationCapacity)
                    );
                    assert_eq!(job.last_poll_visits(), 1);
                    assert_eq!(job.total_visits(), before + 1);
                    assert_eq!(std::ptr::from_ref(&*job), step);
                    assert_eq!(
                        std::ptr::from_ref(&**job.conflict.as_ref().unwrap()),
                        conflict
                    );
                    assert_eq!(
                        job.conflict.as_ref().unwrap().retreat_identity(),
                        Some(retreat)
                    );
                    assert!(job.production.is_none());
                    // The same actual failed child must reject borrowed handoff,
                    // without substituting an injected broad parent fault.
                    assert_eq!(
                        job.install_production_from_conflict(),
                        Err(LivingStepJobErrorV2::PreparationCapacity)
                    );
                    assert_eq!(
                        std::ptr::from_ref(&**job.conflict.as_ref().unwrap()),
                        conflict
                    );
                    assert!(job.production.is_none());
                    assert!(matches!(
                        job.take_synchronous_preparation(&authority),
                        Err(LivingStepJobErrorV2::PreparationCapacity)
                    ));
                    assert!(authority.boundary_capture_is_current(&capture));
                    assert!(Arc::ptr_eq(&authority.queue, &queue));
                    assert_eq!(authority.state().canonical_bytes().unwrap(), state);
                    enqueue(&mut authority, "Accepted after Retreat");
                    assert!(matches!(
                        job.take_synchronous_preparation(&authority),
                        Err(LivingStepJobErrorV2::Stale)
                    ));
                    job.cancel();
                    assert!(matches!(
                        job.take_synchronous_preparation(&authority),
                        Err(LivingStepJobErrorV2::Cancelled)
                    ));
                    for b in [0, 1, 64, 1025] {
                        assert_eq!(job.poll(b), Err(LivingStepJobErrorV2::PreparationCapacity));
                        assert_eq!(job.last_poll_visits(), 0);
                    }
                    drain(
                        job.into_boxed_retirement(
                            LivingPublicationRetirementPolicyV2::ReleaseHandle,
                        ),
                        cleanup,
                    );
                    assert_eq!(authority.state().canonical_bytes().unwrap(), state);
                    assert_eq!(authority.capture_boundary().unwrap().pending_count(), 2);
                    assert!(authority.fault.is_none());
                    assert!(authority.last_receipt.is_none());
                }
            }
        }
    }
    #[test]
    fn occupation_child_storage_failures_reach_step_current_stale_cancelled_and_borrowed() {
        for which in 0..3 {
            for trigger in [1, 64, 7, 1024] {
                for cleanup in [&[1][..], &[64][..], &[1024][..], &[1, 7, 64, 1024][..]] {
                    let mut authority = crate::living::builtin_living_sandbox_v2().unwrap();
                    enqueue(&mut authority, "Accepted before Occupation");
                    let state = authority.state().canonical_bytes().unwrap();
                    let queue = Arc::clone(&authority.queue);
                    let capture = authority.capture_boundary().unwrap();
                    let mut job = Box::new(authority.begin_step().unwrap());
                    let step = std::ptr::from_ref(&*job);
                    let mut injected = false;
                    for _ in 0..100_000 {
                        if job
                            .conflict
                            .as_mut()
                            .is_some_and(|c| c.inject_occupation_storage_capacity_failure(which))
                        {
                            injected = true;
                            break;
                        }
                        assert_eq!(job.poll(1).unwrap(), LivingStepProgressV2::Pending);
                    }
                    assert!(injected, "actual Occupation storage boundary");
                    let conflict = std::ptr::from_ref(&**job.conflict.as_ref().unwrap());
                    let occupation = job
                        .conflict
                        .as_ref()
                        .unwrap()
                        .occupation_identity()
                        .unwrap();
                    let before = job.total_visits();
                    assert_eq!(
                        job.poll(trigger),
                        Err(LivingStepJobErrorV2::PreparationCapacity)
                    );
                    assert_eq!(job.last_poll_visits(), 1);
                    assert_eq!(job.total_visits(), before + 1);
                    assert_eq!(std::ptr::from_ref(&*job), step);
                    assert_eq!(
                        std::ptr::from_ref(&**job.conflict.as_ref().unwrap()),
                        conflict
                    );
                    assert_eq!(
                        job.conflict.as_ref().unwrap().occupation_identity(),
                        Some(occupation)
                    );
                    assert!(job.production.is_none());
                    // The same actual failed child must reject borrowed handoff,
                    // without substituting an injected broad parent fault.
                    assert_eq!(
                        job.install_production_from_conflict(),
                        Err(LivingStepJobErrorV2::PreparationCapacity)
                    );
                    assert_eq!(
                        std::ptr::from_ref(&**job.conflict.as_ref().unwrap()),
                        conflict
                    );
                    assert!(job.production.is_none());
                    assert!(matches!(
                        job.take_synchronous_preparation(&authority),
                        Err(LivingStepJobErrorV2::PreparationCapacity)
                    ));
                    assert!(authority.boundary_capture_is_current(&capture));
                    assert!(Arc::ptr_eq(&authority.queue, &queue));
                    assert_eq!(authority.state().canonical_bytes().unwrap(), state);
                    enqueue(&mut authority, "Accepted after Occupation");
                    assert!(matches!(
                        job.take_synchronous_preparation(&authority),
                        Err(LivingStepJobErrorV2::Stale)
                    ));
                    job.cancel();
                    assert!(matches!(
                        job.take_synchronous_preparation(&authority),
                        Err(LivingStepJobErrorV2::Cancelled)
                    ));
                    for b in [0, 1, 64, 1025] {
                        assert_eq!(job.poll(b), Err(LivingStepJobErrorV2::PreparationCapacity));
                        assert_eq!(job.last_poll_visits(), 0);
                    }
                    drain(
                        job.into_boxed_retirement(
                            LivingPublicationRetirementPolicyV2::ReleaseHandle,
                        ),
                        cleanup,
                    );
                    assert_eq!(authority.state().canonical_bytes().unwrap(), state);
                    assert_eq!(authority.capture_boundary().unwrap().pending_count(), 2);
                    assert!(authority.fault.is_none());
                    assert!(authority.last_receipt.is_none());
                }
            }
        }
    }
    #[test]
    fn settlement_child_storage_failures_reach_step_current_stale_cancelled_and_borrowed() {
        for which in 0..2 {
            for trigger in [1, 64, 7, 1024] {
                for cleanup in [&[1][..], &[64][..], &[1024][..], &[1, 7, 64, 1024][..]] {
                    let mut authority = crate::living::builtin_living_sandbox_v2().unwrap();
                    enqueue(&mut authority, "Accepted before Settlement");
                    let state = authority.state().canonical_bytes().unwrap();
                    let queue = Arc::clone(&authority.queue);
                    let capture = authority.capture_boundary().unwrap();
                    let mut job = Box::new(authority.begin_step().unwrap());
                    let step = std::ptr::from_ref(&*job);
                    let mut injected = false;
                    // A finite search guard for this built-in small-state control;
                    // not an observed work total or performance qualification.
                    const BOUNDARY_SEARCH_LIMIT: usize = 100_000;
                    for _ in 0..BOUNDARY_SEARCH_LIMIT {
                        if job
                            .conflict
                            .as_mut()
                            .is_some_and(|c| c.inject_settlement_storage_capacity_failure(which))
                        {
                            injected = true;
                            break;
                        }
                        assert_eq!(job.poll(1).unwrap(), LivingStepProgressV2::Pending);
                    }
                    assert!(
                        injected,
                        "Settlement storage boundary not reached within finite search guard"
                    );
                    let conflict = std::ptr::from_ref(&**job.conflict.as_ref().unwrap());
                    let settlement = job
                        .conflict
                        .as_ref()
                        .unwrap()
                        .settlement_identity()
                        .unwrap();
                    let before = job.total_visits();
                    assert_eq!(
                        job.poll(trigger),
                        Err(LivingStepJobErrorV2::PreparationCapacity)
                    );
                    assert_eq!(job.last_poll_visits(), 1);
                    assert_eq!(job.total_visits(), before + 1);
                    assert_eq!(std::ptr::from_ref(&*job), step);
                    assert_eq!(
                        std::ptr::from_ref(&**job.conflict.as_ref().unwrap()),
                        conflict
                    );
                    assert_eq!(
                        job.conflict.as_ref().unwrap().settlement_identity(),
                        Some(settlement)
                    );
                    assert!(job.production.is_none());
                    // The same actual failed child must reject borrowed handoff,
                    // without substituting an injected broad parent fault.
                    assert_eq!(
                        job.install_production_from_conflict(),
                        Err(LivingStepJobErrorV2::PreparationCapacity)
                    );
                    assert_eq!(
                        std::ptr::from_ref(&**job.conflict.as_ref().unwrap()),
                        conflict
                    );
                    assert!(job.production.is_none());
                    assert!(matches!(
                        job.take_synchronous_preparation(&authority),
                        Err(LivingStepJobErrorV2::PreparationCapacity)
                    ));
                    assert!(authority.boundary_capture_is_current(&capture));
                    assert!(Arc::ptr_eq(&authority.queue, &queue));
                    assert_eq!(authority.state().canonical_bytes().unwrap(), state);
                    enqueue(&mut authority, "Accepted after Settlement");
                    assert!(matches!(
                        job.take_synchronous_preparation(&authority),
                        Err(LivingStepJobErrorV2::Stale)
                    ));
                    job.cancel();
                    assert!(matches!(
                        job.take_synchronous_preparation(&authority),
                        Err(LivingStepJobErrorV2::Cancelled)
                    ));
                    for b in [0, 1, 64, 1025] {
                        assert_eq!(job.poll(b), Err(LivingStepJobErrorV2::PreparationCapacity));
                        assert_eq!(job.last_poll_visits(), 0);
                    }
                    drain(
                        job.into_boxed_retirement(
                            LivingPublicationRetirementPolicyV2::ReleaseHandle,
                        ),
                        cleanup,
                    );
                    assert_eq!(authority.state().canonical_bytes().unwrap(), state);
                    assert_eq!(authority.capture_boundary().unwrap().pending_count(), 2);
                    assert!(authority.fault.is_none());
                    assert!(authority.last_receipt.is_none());
                }
            }
        }
    }
    #[test]
    fn preparation_capacity_recipe_keeps_original_inline_child_production_and_step_boxes() {
        let mut totals = Vec::new();
        for retirement_schedule in [&[1][..], &[64][..], &[1024][..], &[1, 64, 7, 1024][..]] {
            // One-visit prelude captures setup; actual failing forward budgets
            // cover constants and every mixed position independently of cleanup.
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
                let mut authority = tests::scheduled_staged_fixture();
                enqueue(&mut authority, "Accepted before recipe capacity failure");
                let before = authority.state.canonical_bytes().unwrap();
                let queue = Arc::clone(&authority.queue);
                let capture = authority.capture_boundary().unwrap();
                let tick = authority.state.tick;
                let mut job = Box::new(authority.begin_step().unwrap());
                let step_pointer = std::ptr::from_ref(job.as_ref());
                let mut guard = 0;
                while !job
                    .production
                    .as_mut()
                    .is_some_and(|child| child.inject_recipe_capacity_failure())
                {
                    assert_eq!(job.poll(1).unwrap(), LivingStepProgressV2::Pending);
                    guard += 1;
                    assert!(guard < 20000);
                }
                let parent_pointer = std::ptr::from_ref(job.production.as_ref().unwrap().as_ref());
                let recipe_pointer = job.production.as_ref().unwrap().recipe_identity();
                let previous = job.total_visits();
                let parent_previous = job.production.as_ref().unwrap().total_visits();
                let child_previous = job.production.as_ref().unwrap().recipe_visits().unwrap().1;
                assert_eq!(
                    job.poll(trigger_budget),
                    Err(LivingStepJobErrorV2::PreparationCapacity)
                );
                assert!(job.last_poll_visits() <= trigger_budget);
                let parent = job.production.as_ref().unwrap();
                assert_eq!(parent.last_poll_visits(), 1);
                assert_eq!(parent.total_visits(), parent_previous + 1);
                assert_eq!(parent.recipe_visits(), Some((1, child_previous + 1)));
                assert_eq!(job.last_poll_visits(), 1);
                assert_eq!(job.total_visits(), previous + 1);
                assert!(job.preparation_capacity);
                assert!(job.fault.is_none());
                assert!(job.production.as_ref().unwrap().recipe_preparation_failed());
                assert_eq!(std::ptr::from_ref(job.as_ref()), step_pointer);
                assert_eq!(
                    std::ptr::from_ref(job.production.as_ref().unwrap().as_ref()),
                    parent_pointer
                );
                assert_eq!(
                    job.production.as_ref().unwrap().recipe_identity(),
                    recipe_pointer
                );
                assert!(job.observation.is_none());
                assert!(job.completed_output.is_none());
                assert!(!job.needs_synchronous_completion());
                for bad in [0, 1025] {
                    assert_eq!(job.poll(bad), Err(LivingStepJobErrorV2::Budget));
                    assert_eq!(job.last_poll_visits(), 0);
                }
                for budget in [1, 64, 1024] {
                    assert_eq!(
                        job.poll(budget),
                        Err(LivingStepJobErrorV2::PreparationCapacity)
                    );
                    assert_eq!(job.last_poll_visits(), 0);
                }
                assert!(matches!(
                    job.take_synchronous_preparation(&authority),
                    Err(LivingStepJobErrorV2::PreparationCapacity)
                ));
                assert!(!job.delivered);
                assert!(job.capture.is_present());
                assert!(job.fault.is_none());
                assert_eq!(authority.state.canonical_bytes().unwrap(), before);
                assert!(Arc::ptr_eq(&authority.queue, &queue));
                assert_eq!(authority.queue.len(), 1);
                assert_eq!(authority.state.tick, tick);
                assert!(authority.fault.is_none());
                assert!(authority.boundary_capture_is_current(&capture));
                assert!(authority.last_receipt.is_none());
                enqueue(&mut authority, "Accepted stale recipe tail");
                assert!(matches!(
                    job.take_synchronous_preparation(&authority),
                    Err(LivingStepJobErrorV2::Stale)
                ));
                job.cancel();
                assert!(matches!(
                    job.take_synchronous_preparation(&authority),
                    Err(LivingStepJobErrorV2::Cancelled)
                ));
                for budget in [0, 1, 1025] {
                    assert_eq!(
                        job.poll(budget),
                        Err(LivingStepJobErrorV2::PreparationCapacity)
                    );
                    assert_eq!(job.last_poll_visits(), 0);
                }
                assert_eq!(
                    job.production.as_ref().unwrap().recipe_identity(),
                    recipe_pointer
                );
                assert_eq!(
                    std::ptr::from_ref(job.production.as_ref().unwrap().as_ref()),
                    parent_pointer
                );
                assert_eq!(std::ptr::from_ref(job.as_ref()), step_pointer);
                totals.push(drain(
                    job.into_boxed_retirement(LivingPublicationRetirementPolicyV2::ReleaseHandle),
                    retirement_schedule,
                ));
                assert_eq!(authority.queue.len(), 2);
                assert_eq!(authority.state.tick, tick);
                assert!(authority.fault.is_none());
            }
        }
        assert!(totals.iter().all(|n| *n == totals[0]));
    }
    #[test]
    fn preparation_capacity_does_not_change_authentic_deterministic_failure_tokens() {
        let mut authority = tests::scheduled_staged_fixture();
        Arc::make_mut(&mut authority.state).tick = LivingTickV2(u64::MAX);
        let mut job = authority.begin_step().unwrap();
        let error = LivingStepJobErrorV2::Fault(LivingDeterministicFaultV2::Arithmetic);
        assert_eq!(job.poll(1), Err(error.clone()));
        assert_eq!(job.last_poll_visits(), 0);
        let preparation = job.take_synchronous_preparation(&authority).unwrap();
        let LivingStepPreparationV2::Failed(failure) = preparation else {
            panic!("authentic fault token expected");
        };
        assert_eq!(failure.fault(), &LivingDeterministicFaultV2::Arithmetic);
        drain(
            failure.into_retirement(LivingPublicationRetirementPolicyV2::ReleaseHandle),
            &[64],
        );
        drain(
            job.into_retirement(LivingPublicationRetirementPolicyV2::ReleaseHandle),
            &[64],
        );
        let mut cancelled = authority.begin_step().unwrap();
        cancelled.cancel();
        for budget in [0, 1, 1025] {
            assert_eq!(cancelled.poll(budget), Err(error.clone()));
            assert_eq!(cancelled.last_poll_visits(), 0);
        }
        assert!(matches!(
            cancelled.take_synchronous_preparation(&authority),
            Err(LivingStepJobErrorV2::Cancelled)
        ));
        assert!(cancelled.fault.is_some());
        drain(
            cancelled.into_retirement(LivingPublicationRetirementPolicyV2::ReleaseHandle),
            &[1, 64, 1024],
        );
        assert!(authority.fault.is_none());
    }
}

#[cfg(test)]
mod corrected_repair_public_tests {
    use super::*;
    use crate::living::*;

    fn id(n: u8) -> LivingEntityIdV2 {
        let mut a = [0; 16];
        a[0] = n;
        LivingEntityIdV2(a)
    }
    fn base() -> LivingGalaxyStateV2 {
        let mut s = builtin_living_sandbox_v2().unwrap().state().clone();
        s.fleets = Default::default();
        s.facilities = Default::default();
        s.routes = Default::default();
        s.shipments = Default::default();
        s
    }
    fn hull(fleet: u8, hull: u8, owner: u8, world: u8, hp: u32, due: Option<u64>) -> LivingFleetV2 {
        LivingFleetV2 {
            id: id(fleet),
            owner: id(owner),
            location: LivingFleetLocationV2::Docked { world: id(world) },
            order: None,
            hulls: LivingSortedVecV2::new(vec![LivingHullV2 {
                id: id(hull),
                definition: LivingSlugV2::new("escort").unwrap(),
                hit_points: hp,
                return_credit: false,
                next_repair: due.map(LivingTickV2),
            }])
            .unwrap(),
        }
    }
    fn battery(n: u8, world: u8, hp: u32, due: u64, disabled: bool) -> LivingFacilityV2 {
        LivingFacilityV2 {
            id: id(n),
            world: id(world),
            definition: LivingSlugV2::new("defense_battery").unwrap(),
            status: LivingFacilityStatusV2::Operational,
            paid_alloy: 40,
            hit_points: hp,
            next_due: None,
            next_repair: Some(LivingTickV2(due)),
            blocked_reasons: Default::default(),
            disabled,
        }
    }
    fn from_state(state: LivingGalaxyStateV2) -> LivingGalaxyAuthorityV2 {
        let catalog = living_core_pack_v2().unwrap();
        let genesis = validate_living_genesis_manifest_v2(
            LivingGenesisManifestV2 {
                kind: LIVING_GENESIS_KIND_V2.into(),
                format_version: LIVING_GENESIS_FORMAT_VERSION_V2,
                rules_version: 2,
                state,
            },
            &catalog,
        )
        .unwrap();
        LivingGalaxyAuthorityV2::from_genesis(
            catalog,
            [0xD2; 32],
            LivingGenesisGeneratorV2 {
                generator_id: LivingSlugV2::new("corrected_repair_public").unwrap(),
                generator_version: 1,
            },
            genesis,
        )
        .unwrap()
    }
    fn next_all_schedules(base: &LivingGalaxyAuthorityV2) -> LivingGalaxyAuthorityV2 {
        let mut reference = base.clone();
        reference.step().unwrap();
        for schedule in [
            &[1][..],
            &[64][..],
            &[1024][..],
            &[1, 64, 7, 1024][..],
            &[64, 7, 1024, 1][..],
            &[7, 1024, 1, 64][..],
            &[1024, 1, 64, 7][..],
        ] {
            let mut authority = base.clone();
            let mut job = authority.begin_step().unwrap();
            let mut poll = 0;
            loop {
                let budget = schedule[poll % schedule.len()];
                poll += 1;
                let before = job.total_visits();
                let progress = job.poll(budget).unwrap();
                assert_eq!(job.total_visits() - before, job.last_poll_visits() as u64);
                assert!(job.last_poll_visits() <= budget);
                if progress == LivingStepProgressV2::NeedsSynchronousCompletion {
                    break;
                }
                assert!(poll < 100000);
            }
            let LivingStepPreparationV2::Ready(prepared) =
                job.finish_synchronously(&authority).unwrap()
            else {
                panic!("unexpected repair failure")
            };
            drop(authority.commit_step(prepared).unwrap());
            assert_eq!(authority.state(), reference.state());
            assert_eq!(authority.last_receipt(), reference.last_receipt());
            assert_eq!(authority.last_event_facts(), reference.last_event_facts());
            assert_eq!(authority.last_decision(), reference.last_decision());
        }
        reference
    }
    #[test]
    fn corrected_repair_step_actual_target_failure_current_stale_and_cancelled() {
        for trigger in [1, 64, 1024, 1, 64, 7, 1024] {
            for stale in [false, true] {
                let mut s = base();
                s.worlds.find_mut(&id(5)).unwrap().inventory.energy = 1;
                s.fleets = LivingSortedVecV2::new(vec![hull(80, 140, 3, 5, 5, Some(0))]).unwrap();
                let mut authority = from_state(s);
                let original = authority.state().canonical_bytes().unwrap();
                let capture = authority.capture_boundary().unwrap();
                let mut job = Box::new(authority.begin_step().unwrap());
                let step_ptr = std::ptr::from_ref(&*job);
                let mut guard = 0;
                while !job
                    .production
                    .as_mut()
                    .is_some_and(|p| p.inject_repair_target_failure())
                {
                    job.poll(1).unwrap();
                    guard += 1;
                    assert!(guard < 100000);
                }
                let parent_ptr = std::ptr::from_ref(&**job.production.as_ref().unwrap());
                let before = job.total_visits();
                assert_eq!(
                    job.poll(trigger),
                    Err(LivingStepJobErrorV2::Fault(
                        LivingDeterministicFaultV2::Invariant
                    ))
                );
                assert_eq!(job.total_visits() - before, 1);
                assert_eq!(job.last_poll_visits(), 1);
                assert_eq!(job.production.as_ref().unwrap().last_poll_visits(), 1);
                assert_eq!(std::ptr::from_ref(&*job), step_ptr);
                assert_eq!(
                    std::ptr::from_ref(&**job.production.as_ref().unwrap()),
                    parent_ptr
                );
                assert_eq!(authority.state().canonical_bytes().unwrap(), original);
                assert!(authority.boundary_capture_is_current(&capture));
                if stale {
                    let c = authority.published_cursor();
                    authority
                        .submit(LivingCommandEnvelopeV2 {
                            expected_committed_revision: c.committed_revision,
                            expected_tick: c.tick,
                            expected_pending_sequence: c.pending_sequence,
                            mode: LivingCommandModeV2::Running,
                            command: LivingCommandV2::CreatorBatch {
                                operations: vec![LivingCreatorOperationV2::CreateSystem {
                                    local: 0,
                                    name: LivingNameV2::new("Accepted repair tail").unwrap(),
                                }],
                            },
                        })
                        .unwrap();
                    assert!(matches!(
                        job.take_synchronous_preparation(&authority),
                        Err(LivingStepJobErrorV2::Stale)
                    ));
                    assert!(!job.delivered);
                    job.cancel();
                    assert!(matches!(
                        job.take_synchronous_preparation(&authority),
                        Err(LivingStepJobErrorV2::Cancelled)
                    ));
                    assert_eq!(authority.capture_boundary().unwrap().pending_count(), 1);
                } else {
                    let LivingStepPreparationV2::Failed(failure) =
                        job.take_synchronous_preparation(&authority).unwrap()
                    else {
                        panic!("expected repair failure token");
                    };
                    assert_eq!(failure.fault(), &LivingDeterministicFaultV2::Invariant);
                    let mut r =
                        failure.into_retirement(LivingPublicationRetirementPolicyV2::ReleaseHandle);
                    while r.poll(1).unwrap() != LivingPublicationRetirementProgressV2::Ready {}
                }
                let mut r =
                    job.into_boxed_retirement(LivingPublicationRetirementPolicyV2::ReleaseHandle);
                let mut n = 0;
                loop {
                    let b = [1, 64, 7, 1024][n % 4];
                    n += 1;
                    let p = r.poll(b).unwrap();
                    assert!(r.last_poll_visits() <= b);
                    if p == LivingPublicationRetirementProgressV2::Ready {
                        break;
                    }
                    assert!(n < 100000);
                }
                assert_eq!(
                    r.poll(1).unwrap(),
                    LivingPublicationRetirementProgressV2::Ready
                );
                assert_eq!(r.last_poll_visits(), 0);
                assert_eq!(authority.state().canonical_bytes().unwrap(), original);
                assert!(authority.fault.is_none());
            }
        }
    }
    #[test]
    fn corrected_repair_public_battery_disabled_unowned_and_zero_hp() {
        for disabled in [false, true] {
            let mut s = base();
            s.worlds.find_mut(&id(5)).unwrap().inventory.energy = 1;
            s.facilities = LivingSortedVecV2::new(vec![
                battery(100, 5, 39, 0, disabled),
                battery(101, 7, 39, 0, disabled),
                battery(102, 8, 0, 0, disabled),
            ])
            .unwrap();
            let a = next_all_schedules(&from_state(s));
            let rows = a.state().facilities.as_slice();
            assert_eq!((rows[0].hit_points, rows[0].next_repair), (40, None));
            assert_eq!(
                (rows[1].hit_points, rows[1].next_repair),
                (39, Some(LivingTickV2(0)))
            );
            assert_eq!(
                (rows[2].hit_points, rows[2].next_repair),
                (0, Some(LivingTickV2(0)))
            );
            assert_eq!(a.state().worlds.as_slice()[0].inventory.energy, 0);
        }
    }
    #[test]
    fn corrected_repair_public_overdue_combined_id_and_fleet_id_order() {
        let mut s = base();
        s.worlds.find_mut(&id(5)).unwrap().inventory.energy = 0;
        s.fleets = LivingSortedVecV2::new(vec![hull(80, 140, 3, 5, 1, Some(0))]).unwrap();
        let a = next_all_schedules(&from_state(s));
        assert_eq!(
            a.state().fleets.as_slice()[0].hulls.as_slice()[0].next_repair,
            Some(LivingTickV2(10))
        );
        let a = next_all_schedules(&a);
        assert_eq!(
            a.state().fleets.as_slice()[0].hulls.as_slice()[0].next_repair,
            Some(LivingTickV2(10))
        );
        for battery_first in [false, true] {
            let mut s = base();
            s.worlds.find_mut(&id(5)).unwrap().inventory.energy = 1;
            let (bid, hid) = if battery_first {
                (100, 140)
            } else {
                (140, 100)
            };
            s.facilities = LivingSortedVecV2::new(vec![battery(bid, 5, 38, 0, true)]).unwrap();
            s.fleets = LivingSortedVecV2::new(vec![hull(80, hid, 3, 5, 5, Some(0))]).unwrap();
            let a = next_all_schedules(&from_state(s));
            assert_eq!(
                a.state().facilities.as_slice()[0].hit_points,
                38 + u32::from(battery_first)
            );
            assert_eq!(
                a.state().fleets.as_slice()[0].hulls.as_slice()[0].hit_points,
                5 + u32::from(!battery_first)
            );
            assert_eq!(
                a.state().facilities.as_slice()[0].next_repair,
                Some(LivingTickV2(10))
            );
            assert_eq!(
                a.state().fleets.as_slice()[0].hulls.as_slice()[0].next_repair,
                Some(LivingTickV2(10))
            );
        }
        let mut s = base();
        s.worlds.find_mut(&id(5)).unwrap().inventory.energy = 1;
        s.facilities = LivingSortedVecV2::new(vec![battery(60, 5, 40, 0, true)]).unwrap();
        s.fleets = LivingSortedVecV2::new(vec![
            hull(80, 150, 3, 5, 5, Some(0)),
            hull(90, 140, 3, 5, 5, Some(0)),
        ])
        .unwrap();
        let a = next_all_schedules(&from_state(s));
        assert_eq!(a.state().facilities.as_slice()[0].next_repair, None);
        assert_eq!(
            a.state().fleets.as_slice()[0].hulls.as_slice()[0].hit_points,
            5
        );
        assert_eq!(
            a.state().fleets.as_slice()[1].hulls.as_slice()[0].hit_points,
            6
        );
    }
    fn war(s: &mut LivingGalaxyStateV2) {
        s.wars = LivingSortedVecV2::new(vec![LivingWarV2 {
            participant_a: id(3),
            participant_b: id(4),
            declarer: id(3),
            start_tick: LivingTickV2(0),
            end_tick: LivingTickV2(100),
        }])
        .unwrap();
    }
    #[test]
    fn corrected_repair_public_combat_excludes_nonparticipants_and_preserves_peace_world() {
        let mut s = base();
        war(&mut s);
        s.worlds.find_mut(&id(5)).unwrap().inventory.energy = 0;
        s.worlds.find_mut(&id(8)).unwrap().inventory.energy = 0;
        let mut spectator = hull(82, 142, 3, 5, 1, Some(10));
        spectator.hulls.record_at_mut(0).unwrap().definition = LivingSlugV2::new("scout").unwrap();
        s.fleets = LivingSortedVecV2::new(vec![
            hull(80, 140, 3, 5, 9, Some(10)),
            hull(81, 141, 4, 5, 10, None),
            spectator,
            hull(83, 143, 3, 8, 5, Some(10)),
        ])
        .unwrap();
        s.facilities = LivingSortedVecV2::new(vec![battery(100, 5, 39, 10, true)]).unwrap();
        let mut a = from_state(s);
        for _ in 0..9 {
            a.step().unwrap();
        }
        let a = next_all_schedules(&a);
        let fleets = a.state().fleets.as_slice();
        assert_eq!(
            (
                fleets[0].hulls.as_slice()[0].hit_points,
                fleets[0].hulls.as_slice()[0].next_repair
            ),
            (7, Some(LivingTickV2(10)))
        );
        assert_eq!(
            (
                fleets[1].hulls.as_slice()[0].hit_points,
                fleets[1].hulls.as_slice()[0].next_repair
            ),
            (8, Some(LivingTickV2(20)))
        );
        assert_eq!(
            (
                fleets[2].hulls.as_slice()[0].hit_points,
                fleets[2].hulls.as_slice()[0].next_repair
            ),
            (1, Some(LivingTickV2(10)))
        );
        assert_eq!(a.state().facilities.as_slice()[0].hit_points, 39);
        assert_eq!(
            a.state().facilities.as_slice()[0].next_repair,
            Some(LivingTickV2(10))
        );
        assert_eq!(fleets[3].hulls.as_slice()[0].hit_points, 6);
        assert_eq!(a.state().worlds.as_slice()[0].inventory.energy, 2);
    }
    #[test]
    fn corrected_repair_public_retreat_nonperiod_origin_only_and_failed_credit() {
        for credit in [false, true] {
            let mut s = base();
            war(&mut s);
            s.worlds.find_mut(&id(5)).unwrap().inventory.energy = 1;
            s.worlds.find_mut(&id(6)).unwrap().inventory.energy = 1;
            let mut retreat = hull(80, 140, 4, 5, 2, None);
            let h = retreat.hulls.record_at_mut(0).unwrap();
            h.definition = LivingSlugV2::new("scout").unwrap();
            h.return_credit = credit;
            s.fleets = LivingSortedVecV2::new(vec![retreat]).unwrap();
            s.facilities = LivingSortedVecV2::new(vec![
                battery(100, 5, 39, 0, true),
                battery(101, 6, 39, 0, true),
            ])
            .unwrap();
            let a = next_all_schedules(&from_state(s));
            let rows = a.state().facilities.as_slice();
            assert_eq!(rows[0].hit_points, if credit { 39 } else { 40 });
            assert_eq!(
                rows[0].next_repair,
                if credit { Some(LivingTickV2(0)) } else { None }
            );
            assert_eq!(rows[1].hit_points, 40);
            assert_eq!(
                matches!(a.state().fleets.as_slice()[0].location,LivingFleetLocationV2::Travelling {origin_world,..} if origin_world==id(5)),
                credit
            );
        }
    }
    #[test]
    fn corrected_repair_public_retreat_after_combat_and_failed_route() {
        let mut s = base();
        war(&mut s);
        s.worlds.find_mut(&id(5)).unwrap().inventory.energy = 0;
        let mut foreign = hull(81, 141, 4, 5, 1, None);
        foreign.hulls.record_at_mut(0).unwrap().return_credit = true;
        foreign
            .hulls
            .push_sorted(LivingHullV2 {
                id: id(142),
                definition: LivingSlugV2::new("scout").unwrap(),
                hit_points: 2,
                return_credit: true,
                next_repair: None,
            })
            .unwrap();
        s.fleets = LivingSortedVecV2::new(vec![hull(80, 140, 3, 5, 10, None), foreign]).unwrap();
        s.facilities = LivingSortedVecV2::new(vec![battery(100, 5, 39, 11, true)]).unwrap();
        let mut a = from_state(s);
        for _ in 0..9 {
            a.step().unwrap();
        }
        let a = next_all_schedules(&a);
        assert_eq!(a.state().fleets.as_slice()[1].hulls.as_slice().len(), 1);
        let a = next_all_schedules(&a);
        assert!(
            matches!(a.state().fleets.as_slice()[1].location,LivingFleetLocationV2::Travelling {origin_world,..} if origin_world==id(5))
        );
        assert_eq!(a.state().facilities.as_slice()[0].hit_points, 39);
        assert_eq!(
            a.state().facilities.as_slice()[0].next_repair,
            Some(LivingTickV2(11))
        );
        // Publicly validated second system with no lane: prepaid retreat fails.
        let mut s = base();
        war(&mut s);
        let mut system = s.systems.as_slice()[0].clone();
        system.id = id(70);
        s.systems.push_sorted(system).unwrap();
        s.worlds.find_mut(&id(6)).unwrap().system = id(70);
        s.worlds.find_mut(&id(5)).unwrap().inventory.energy = 1;
        let mut f = hull(80, 140, 4, 5, 2, None);
        let h = f.hulls.record_at_mut(0).unwrap();
        h.definition = LivingSlugV2::new("scout").unwrap();
        h.return_credit = true;
        s.fleets = LivingSortedVecV2::new(vec![f]).unwrap();
        s.facilities = LivingSortedVecV2::new(vec![battery(100, 5, 39, 0, true)]).unwrap();
        let a = next_all_schedules(&from_state(s));
        assert!(matches!(
            a.state().fleets.as_slice()[0].location,
            LivingFleetLocationV2::Docked { .. }
        ));
        assert_eq!(a.state().facilities.as_slice()[0].hit_points, 40);
    }
}

#[cfg(test)]
mod general_production_parent_tests {
    use super::super::step_job::{LivingLifecycleProgressV2 as Progress, ProductionFailure};
    use super::*;
    fn production_ready(authority: &LivingGalaxyAuthorityV2) -> LivingStepJobV2 {
        let mut job = authority.begin_step().unwrap();
        for _ in 0..100_000 {
            if job.production.is_some() && job.retirement.is_empty() {
                break;
            }
            assert_eq!(job.poll(1).unwrap(), LivingStepProgressV2::Pending);
        }
        let child = job.production.as_mut().expect("actual Step production");
        for _ in 0..100_000 {
            if child.poll(64).unwrap() == Progress::Ready {
                return job;
            }
        }
        panic!("production did not become ready");
    }
    fn retire(job: LivingStepJobV2) {
        let mut retirement =
            job.into_retirement(LivingPublicationRetirementPolicyV2::ReleaseHandle);
        for _ in 0..100_000 {
            if retirement.poll(64).unwrap() == LivingPublicationRetirementProgressV2::Ready {
                return;
            }
        }
        panic!("Step retirement exceeded fixture bound");
    }
    #[test]
    fn general_production_step_retires_original_box_before_observation_setup() {
        let authority = super::super::rules::builtin_living_sandbox_v2().unwrap();
        let mut job = production_ready(&authority);
        assert_eq!(job.poll(1).unwrap(), LivingStepProgressV2::Pending);
        assert!(job.production.is_none() && job.production_handoff);
        assert!(
            job.observation.is_none()
                && job.candidate.is_some()
                && job.observation_events.is_some()
        );
        let original = job.candidate.as_ref().unwrap().worlds.as_slice().as_ptr();
        for _ in 0..100_000 {
            if job.retirement.is_empty() {
                break;
            }
            assert_eq!(job.poll(1).unwrap(), LivingStepProgressV2::Pending);
            assert!(job.observation.is_none());
            assert_eq!(
                job.candidate.as_ref().unwrap().worlds.as_slice().as_ptr(),
                original
            );
        }
        assert!(job.retirement.is_empty());
        assert_eq!(job.poll(1).unwrap(), LivingStepProgressV2::Pending);
        assert!(!job.production_handoff && job.observation.is_some() && job.candidate.is_none());
        job.cancel();
        retire(job);
    }
    #[test]
    fn general_production_step_rejected_ready_extraction_debits_and_keeps_box() {
        let authority = super::super::rules::builtin_living_sandbox_v2().unwrap();
        for trigger in [1, 64, 7, 1024] {
            let mut job = production_ready(&authority);
            // Private protocol seam: retain the already extracted output locally.
            let output = job
                .production
                .as_mut()
                .unwrap()
                .take_ready_result()
                .unwrap();
            let original = std::ptr::from_ref(job.production.as_ref().unwrap().as_ref());
            let before = job.total_visits();
            assert_eq!(
                job.poll(trigger),
                Err(LivingStepJobErrorV2::Fault(
                    LivingDeterministicFaultV2::Invariant
                ))
            );
            assert_eq!(job.total_visits() - before, 1);
            assert_eq!(job.last_poll_visits(), 1);
            assert_eq!(job.production_fault, Some(ProductionFailure::Invariant));
            assert!(
                job.fault.is_none()
                    && job.observation.is_none()
                    && job.candidate.is_none()
                    && job.observation_events.is_none()
            );
            assert_eq!(
                std::ptr::from_ref(job.production.as_ref().unwrap().as_ref()),
                original
            );
            assert_eq!(job.poll(0), Err(LivingStepJobErrorV2::Budget));
            assert_eq!(job.last_poll_visits(), 0);
            assert_eq!(
                job.poll(1),
                Err(LivingStepJobErrorV2::Fault(
                    LivingDeterministicFaultV2::Invariant
                ))
            );
            assert_eq!(job.last_poll_visits(), 0);
            assert_eq!(output.0.tick, authority.state.tick);
            job.cancel();
            assert!(matches!(
                job.take_synchronous_preparation(&authority),
                Err(LivingStepJobErrorV2::Cancelled)
            ));
            retire(job);
        }
    }
}

#[cfg(test)]
mod creator_batch_cursor_tests {
    use super::*;
    use crate::living::model::*;
    use crate::living::{LivingNameV2, LivingSlugV2};
    const SCHEDULES: &[&[usize]] = &[&[1], &[64], &[1024], &[1, 7, 64, 2, 1024]];
    fn id(n: u8) -> LivingEntityIdV2 {
        LivingEntityIdV2([n; 16])
    }
    fn t(n: u8) -> LivingCommandTargetV2 {
        LivingCommandTargetV2::Existing { id: id(n) }
    }
    fn local(n: u16) -> LivingCommandTargetV2 {
        LivingCommandTargetV2::Local { local: n }
    }
    fn name() -> LivingNameV2 {
        LivingNameV2::new("N".repeat(64)).unwrap()
    }
    fn slug(s: &str) -> LivingSlugV2 {
        LivingSlugV2::new(s).unwrap()
    }
    pub(super) fn queue(operations: Vec<LivingCreatorOperationV2>) -> Arc<Vec<QueuedCommandV2>> {
        let catalog = crate::living::living_core_pack_v2().unwrap();
        let accepted = LivingAcceptedCommandV2 {
            accepted_sequence: 1,
            application_tick: LivingTickV2(1),
            envelope: LivingCommandEnvelopeV2 {
                expected_committed_revision: None,
                expected_tick: LivingTickV2(0),
                expected_pending_sequence: 0,
                mode: LivingCommandModeV2::Running,
                command: LivingCommandV2::CreatorBatch { operations },
            },
        };
        let revision =
            LivingRevisionV2::seal(catalog.catalog_hash(), [0x51; 32], None, &accepted).unwrap();
        Arc::new(vec![QueuedCommandV2 {
            accepted: Arc::new(accepted).into(),
            revision: Arc::new(revision).into(),
        }])
    }
    pub(super) fn base() -> LivingGalaxyStateV2 {
        LivingGalaxyStateV2 {
            systems: LivingSortedVecV2::new(vec![LivingSystemV2 {
                id: id(1),
                name: name(),
            }])
            .unwrap(),
            stars: LivingSortedVecV2::new(vec![LivingStarV2 {
                id: id(2),
                system: id(1),
                archetype: slug("main_sequence"),
                name: name(),
            }])
            .unwrap(),
            deposits: LivingSortedVecV2::new(vec![LivingDepositV2 {
                id: id(3),
                world: id(9),
                resource: LivingResourceV2::Ore,
                remaining_units: 99,
            }])
            .unwrap(),
            civilizations: LivingSortedVecV2::new(vec![LivingCivilizationV2 {
                id: id(4),
                name: name(),
                policy: LivingPolicyV2(slug("neutral")),
                status: LivingCivilizationStatusV2::Dormant,
            }])
            .unwrap(),
            ..Default::default()
        }
    }
    pub(super) fn operations() -> Vec<LivingCreatorOperationV2> {
        use LivingCreatorOperationV2::*;
        let catalog = crate::living::living_core_pack_v2().unwrap();
        let hazard = catalog.hazards()[0].id.clone();
        vec![
            CreateFacility {
                local: 0,
                world: t(9),
                definition: slug("solar"),
                status: LivingFacilityStatusV2::Operational,
                paid_alloy: 20,
                hit_points: 50,
                next_due: Some(LivingTickV2(10)),
            },
            CreateConstructionJob {
                local: 1,
                world: t(9),
                owner: t(4),
                definition: slug("solar"),
                accepted_tick: LivingTickV2(0),
                completion_tick: LivingTickV2(100),
                paid_alloy: 20,
            },
            CreateHullJob {
                local: 2,
                world: t(9),
                owner: t(4),
                definition: slug("scout"),
                accepted_tick: LivingTickV2(0),
                completion_tick: LivingTickV2(100),
                paid_alloy: 20,
                target_fleet: None,
            },
            CreateFleet {
                local: 3,
                owner: t(4),
                world: t(9),
            },
            CreateHull {
                local: 4,
                fleet: local(3),
                definition: slug("scout"),
                hit_points: 50,
                return_credit: false,
            },
            CreateRoute {
                local: 5,
                kind: LivingRouteKindV2::Internal,
                source_world: t(9),
                destination_world: t(10),
                resource: LivingResourceV2::Ore,
                batch_size: 3,
                cadence_ticks: 10,
                source_reserve: 2,
                source_owner: t(4),
                receiver_owner: t(4),
                next_due: LivingTickV2(10),
            },
            CreateShipment {
                local: 6,
                dispatch_owner: t(4),
                intended_receiver: t(4),
                source_world: t(9),
                destination_world: t(10),
                resource: LivingResourceV2::Ore,
                units: 3,
                departure_tick: LivingTickV2(1),
                arrival_tick: LivingTickV2(10),
                disposition: LivingShipmentDispositionV2::Outbound,
            },
            CreateHazard {
                local: 7,
                definition: hazard.clone(),
                lane: t(8),
                start_tick: LivingTickV2(1),
                end_tick: LivingTickV2(9),
            },
            EditSystem {
                target: t(1),
                name: name(),
            },
            EditStar {
                target: t(2),
                system: t(1),
                archetype: slug("main_sequence"),
                name: name(),
            },
            EditDeposit {
                target: t(3),
                resource: LivingResourceV2::Ore,
                remaining_units: 123,
            },
            EditCivilization {
                target: t(4),
                name: name(),
                policy: LivingPolicyV2(slug("neutral")),
            },
            SetFacilityEnabled {
                target: local(0),
                enabled: false,
            },
            SetShipmentDisposition {
                shipment: local(6),
                disposition: LivingShipmentDispositionV2::WaitingForStorage,
            },
            EditHazard {
                target: local(7),
                lane: t(8),
                definition: hazard,
                start_tick: LivingTickV2(2),
                end_tick: LivingTickV2(12),
            },
        ]
    }
    pub(super) fn reference(
        state: &mut LivingGalaxyStateV2,
        queue: &[QueuedCommandV2],
        catalog: &ValidatedLivingCatalogPackV2,
    ) -> LivingPendingEventsV2 {
        let mut events = LivingPendingEventsV2::new();
        for q in queue {
            let (effects, recoveries) =
                apply_revision_with_recoveries(state, &q.revision, catalog).unwrap();
            assert!(effects.is_empty());
            assert!(recoveries.is_empty());
            events
                .record(
                    LivingEventProvenanceV2::Creator {
                        revision: q.revision.id,
                    },
                    LivingEventKindV2::CreatorIntervention,
                )
                .unwrap();
        }
        events
    }
    fn run(
        state: LivingGalaxyStateV2,
        q: Arc<Vec<QueuedCommandV2>>,
        catalog: Arc<ValidatedLivingCatalogPackV2>,
        schedule: &[usize],
    ) -> (LivingGalaxyStateV2, LivingPendingEventsV2) {
        let mut job = LivingQueuedCreatorJobV2::begin(state, q, catalog);
        let mut dormant = false;
        for i in 0..100000 {
            let budget = schedule[i % schedule.len()];
            let progress = job.poll(budget).unwrap();
            dormant |= !job.dormant.is_empty();
            assert!(job.last_poll_visits <= budget);
            assert_ne!(
                progress,
                LivingStepProgressV2::NeedsSynchronousCreatorOperation
            );
            if progress == LivingStepProgressV2::NeedsSynchronousCompletion {
                if schedule == [1] {
                    assert!(dormant);
                }
                assert!(
                    job.locals.is_empty() && job.dormant.is_empty() && job.retirement.is_empty()
                );
                let output = job.take_ready_result().unwrap();
                assert!(job.take_ready_result().is_err());
                return output;
            }
        }
        panic!("creator did not complete")
    }
    #[test]
    fn creator_batch_fifteen_operations_match_unchanged_rules_with_replacements_and_dormant() {
        // Private cursor fixture: intentionally no gameplay validation claim.
        let q = queue(operations());
        let catalog = Arc::new(crate::living::living_core_pack_v2().unwrap());
        for replacement in [false, true] {
            let mut input = base();
            if replacement {
                reference(&mut input, &q, &catalog);
                input.accepted_sequence = 0;
                input.facilities.record_at_mut(0).unwrap().blocked_reasons =
                    LivingSortedVecV2::new(vec![LivingBlockedReasonV2::Disabled]).unwrap();
                let fleet = input.fleets.record_at_mut(0).unwrap();
                fleet.order = Some(LivingFleetOrderV2 {
                    kind: LivingFleetOrderKindV2::Move,
                    target_world: Some(id(10)),
                    not_before_tick: LivingTickV2(1),
                    lane_path: vec![id(8), id(9)],
                });
                fleet.hulls.record_at_mut(0).unwrap().next_repair = Some(LivingTickV2(200));
            }
            let mut expected = input.clone();
            let events = reference(&mut expected, &q, &catalog);
            for schedule in SCHEDULES {
                let (actual, actual_events) = run(
                    input.clone(),
                    Arc::clone(&q),
                    Arc::clone(&catalog),
                    schedule,
                );
                assert_eq!(
                    actual.canonical_bytes().unwrap(),
                    expected.canonical_bytes().unwrap()
                );
                assert_eq!(actual_events, events);
                assert_eq!(
                    actual.civilizations.as_slice()[0].status,
                    LivingCivilizationStatusV2::Dormant
                );
                assert!(actual.fleets.as_slice()[0].order.is_none());
                assert!(
                    actual.fleets.as_slice()[0].hulls.as_slice()[0]
                        .next_repair
                        .is_none()
                );
                assert!(
                    actual.facilities.as_slice()[0]
                        .blocked_reasons
                        .as_slice()
                        .is_empty()
                );
            }
        }
    }
    #[test]
    fn creator_batch_hazard_precedence_and_all_definition_positions() {
        let catalog = Arc::new(crate::living::living_core_pack_v2().unwrap());
        for definition in catalog
            .hazards()
            .iter()
            .map(|row| row.id.clone())
            .chain([slug("absent")])
        {
            for invalid_interval in [false, true] {
                let q = queue(vec![LivingCreatorOperationV2::EditHazard {
                    target: t(99),
                    lane: t(88),
                    definition: definition.clone(),
                    start_tick: LivingTickV2(if invalid_interval { 0 } else { 1 }),
                    end_tick: LivingTickV2(2),
                }]);
                let mut oracle = base();
                let expected =
                    apply_revision_range(&mut oracle, &q[0].revision, &catalog, 0).unwrap_err();
                for schedule in SCHEDULES {
                    let mut job = LivingQueuedCreatorJobV2::begin(
                        base(),
                        Arc::clone(&q),
                        Arc::clone(&catalog),
                    );
                    let mut index = 0;
                    let error = loop {
                        let budget = schedule[index % schedule.len()];
                        index += 1;
                        assert!(index < 10000, "hazard fault did not arrive");
                        if let Err(error) = job.poll(budget) {
                            assert!(job.last_poll_visits <= budget);
                            break error;
                        }
                    };
                    assert_eq!(
                        error,
                        super::super::step_job::LivingLifecycleJobErrorV2::Fault(
                            LivingDeterministicFaultV2::Replay {
                                accepted_sequence: 1,
                                reason: Box::new(expected.clone())
                            }
                        )
                    );
                    assert!(Arc::ptr_eq(&job.catalog, &catalog));
                    assert_eq!(job.operation_index, 0);
                    assert!(job.append.is_none());
                }
            }
        }
    }
    #[test]
    fn creator_batch_shipment_disposition_preserves_old_direction_and_dispatch_kind() {
        let catalog = Arc::new(crate::living::living_core_pack_v2().unwrap());
        for old in [
            LivingShipmentDispositionV2::Outbound,
            LivingShipmentDispositionV2::Returning,
            LivingShipmentDispositionV2::WaitingForStorage,
        ] {
            for flag in [false, true] {
                for new in [
                    LivingShipmentDispositionV2::Outbound,
                    LivingShipmentDispositionV2::Returning,
                    LivingShipmentDispositionV2::WaitingForStorage,
                ] {
                    let mut state = base();
                    state.shipments = LivingSortedVecV2::new(vec![LivingShipmentV2 {
                        id: id(9),
                        dispatch_owner: id(4),
                        intended_receiver: id(4),
                        source_world: id(1),
                        destination_world: id(2),
                        resource: LivingResourceV2::Ore,
                        units: 3,
                        departure_tick: LivingTickV2(1),
                        arrival_tick: LivingTickV2(9),
                        disposition: old,
                        dispatch_kind: Some(LivingRouteKindV2::Internal),
                        returning_to_source: flag,
                    }])
                    .unwrap();
                    let q = queue(vec![LivingCreatorOperationV2::SetShipmentDisposition {
                        shipment: t(9),
                        disposition: new,
                    }]);
                    let mut expected = state.clone();
                    let events = reference(&mut expected, &q, &catalog);
                    for schedule in SCHEDULES {
                        let (actual, actual_events) = run(
                            state.clone(),
                            Arc::clone(&q),
                            Arc::clone(&catalog),
                            schedule,
                        );
                        assert_eq!(actual, expected);
                        assert_eq!(actual_events, events);
                        let row = &actual.shipments.as_slice()[0];
                        assert_eq!(row.dispatch_kind, Some(LivingRouteKindV2::Internal));
                        assert_eq!(
                            row.returning_to_source,
                            new == LivingShipmentDispositionV2::WaitingForStorage
                                && (old == LivingShipmentDispositionV2::Returning || flag)
                        );
                    }
                }
            }
        }
    }
    fn error_at(
        mut job: LivingQueuedCreatorJobV2,
        schedule: &[usize],
    ) -> (
        LivingQueuedCreatorJobV2,
        super::super::step_job::LivingLifecycleJobErrorV2,
    ) {
        for poll in 0..10000 {
            let b = schedule[poll % schedule.len()];
            match job.poll(b) {
                Err(error) => {
                    assert!(job.last_poll_visits > 0 && job.last_poll_visits <= b);
                    return (job, error);
                }
                Ok(progress) => {
                    assert_eq!(progress, LivingStepProgressV2::Pending);
                    assert!(job.last_poll_visits > 0 && job.last_poll_visits <= b);
                }
            }
        }
        panic!("expected creator fault")
    }
    #[test]
    fn creator_batch_all_edit_misses_preserve_prefix_and_accepted_error_identity() {
        let catalog = Arc::new(crate::living::living_core_pack_v2().unwrap());
        for mut edit in operations().into_iter().skip(8) {
            match &mut edit {
                LivingCreatorOperationV2::EditSystem { target, .. }
                | LivingCreatorOperationV2::EditStar { target, .. }
                | LivingCreatorOperationV2::EditDeposit { target, .. }
                | LivingCreatorOperationV2::EditCivilization { target, .. }
                | LivingCreatorOperationV2::SetFacilityEnabled { target, .. }
                | LivingCreatorOperationV2::EditHazard { target, .. } => *target = t(99),
                LivingCreatorOperationV2::SetShipmentDisposition { shipment, .. } => {
                    *shipment = t(99)
                }
                _ => unreachable!(),
            }
            let original = queue(vec![operations()[0].clone(), edit]);
            let mut accepted = (*original[0].accepted).as_ref().clone();
            accepted.accepted_sequence = 9;
            // Private mismatched envelope control: real admission seals equal sequence/ordinal.
            let q = Arc::new(vec![QueuedCommandV2 {
                accepted: Arc::new(accepted).into(),
                revision: original[0].revision.clone(),
            }]);
            let mut expected = base();
            let reason =
                apply_revision_range(&mut expected, &q[0].revision, &catalog, 0).unwrap_err();
            for schedule in SCHEDULES {
                let (job, error) = error_at(
                    LivingQueuedCreatorJobV2::begin(base(), Arc::clone(&q), Arc::clone(&catalog)),
                    schedule,
                );
                assert_eq!(
                    error,
                    super::super::step_job::LivingLifecycleJobErrorV2::Fault(
                        LivingDeterministicFaultV2::Replay {
                            accepted_sequence: 9,
                            reason: Box::new(reason.clone())
                        }
                    )
                );
                assert_eq!(job.operation_index, 1);
                assert_eq!(job.state, expected);
                assert_eq!(job.events, LivingPendingEventsV2::new());
                assert!(job.append.is_none());
                assert!(!job.locals.is_empty());
                assert!(!job.dormant.is_empty());
            }
        }
    }
    #[test]
    fn creator_batch_unsupported_suffix_retains_prefix_locals_dormant_and_index() {
        let catalog = Arc::new(crate::living::living_core_pack_v2().unwrap());
        let prefix = operations()[0].clone();
        let q = queue(vec![
            prefix,
            LivingCreatorOperationV2::ScrapFacility {
                target: local(0),
                cascade: super::super::command::LivingCascadeDispositionV2::RemoveReviewed {
                    dependents: Vec::new(),
                    derived: Vec::new(),
                },
            },
        ]);
        for schedule in SCHEDULES {
            let mut job =
                LivingQueuedCreatorJobV2::begin(base(), Arc::clone(&q), Arc::clone(&catalog));
            for poll in 0..10000 {
                assert!(poll < 9999, "fallback did not arrive");
                let b = schedule[poll % schedule.len()];
                let progress = job.poll(b).unwrap();
                assert!(job.last_poll_visits <= b);
                if progress == LivingStepProgressV2::NeedsSynchronousCreatorOperation {
                    assert_eq!(job.operation_index, 1);
                    assert_eq!(job.state.facilities.as_slice().len(), 1);
                    assert_eq!(job.locals.len(), 1);
                    assert!(job.dormant.contains(&id(4)));
                    assert_eq!(job.events, LivingPendingEventsV2::new());
                    assert!(job.append.is_none());
                    assert_eq!(job.state.accepted_sequence, 0);
                    assert!(job.take_ready_result().is_err());
                    for b in [1, 64, 1024] {
                        assert_eq!(job.poll(b).unwrap(), progress);
                        assert_eq!(job.last_poll_visits, 0);
                    }
                    break;
                }
                assert_eq!(progress, LivingStepProgressV2::Pending);
            }
        }
    }
    #[test]
    fn creator_batch_hazard_success_every_catalog_position_matches_rules() {
        let catalog = Arc::new(crate::living::living_core_pack_v2().unwrap());
        for definition in catalog.hazards() {
            let mut state = base();
            state.hazards = LivingSortedVecV2::new(vec![LivingHazardV2 {
                id: id(7),
                definition: catalog.hazards()[0].id.clone(),
                lane: id(8),
                start_tick: LivingTickV2(1),
                end_tick: LivingTickV2(9),
            }])
            .unwrap();
            let q = queue(vec![LivingCreatorOperationV2::EditHazard {
                target: t(7),
                lane: t(10),
                definition: definition.id.clone(),
                start_tick: LivingTickV2(3),
                end_tick: LivingTickV2(10),
            }]);
            let mut expected = state.clone();
            let events = reference(&mut expected, &q, &catalog);
            for schedule in SCHEDULES {
                let (actual, actual_events) = run(
                    state.clone(),
                    Arc::clone(&q),
                    Arc::clone(&catalog),
                    schedule,
                );
                assert_eq!(actual, expected);
                assert_eq!(actual_events, events);
            }
        }
    }
    #[test]
    fn creator_batch_later_operation_fault_precedes_full_marker_and_hull_parent_missing() {
        let catalog = Arc::new(crate::living::living_core_pack_v2().unwrap());
        let missing = LivingCreatorOperationV2::CreateHull {
            local: 1,
            fleet: t(99),
            definition: slug("scout"),
            hit_points: 1,
            return_credit: false,
        };
        let q = queue(vec![operations()[0].clone(), missing]);
        let mut original = LivingPendingEventsV2::new();
        for _ in 0..=u16::MAX {
            original
                .record(
                    LivingEventProvenanceV2::Creator {
                        revision: q[0].revision.id,
                    },
                    LivingEventKindV2::CreatorIntervention,
                )
                .unwrap();
        }
        let mut oracle = base();
        let reason = apply_revision_range(&mut oracle, &q[0].revision, &catalog, 0).unwrap_err();
        for schedule in SCHEDULES {
            let mut job =
                LivingQueuedCreatorJobV2::begin(base(), Arc::clone(&q), Arc::clone(&catalog));
            job.events = original.clone();
            for bad in [0, 1025] {
                assert!(job.poll(bad).is_err());
                assert_eq!(job.last_poll_visits, 0);
                assert_eq!(job.state, base());
            }
            assert!(job.take_ready_result().is_err());
            let (job, error) = error_at(job, schedule);
            assert_eq!(
                error,
                super::super::step_job::LivingLifecycleJobErrorV2::Fault(
                    LivingDeterministicFaultV2::Replay {
                        accepted_sequence: 1,
                        reason: Box::new(reason.clone())
                    }
                )
            );
            assert!(
                job.hull_parent.is_none()
                    && job.hull_output.is_none()
                    && job.insertion.is_none()
                    && job.append.is_none()
            );
            assert_eq!(job.events, original);
            assert_eq!(job.state, oracle);
        }
    }

    fn enqueue(authority: &mut LivingGalaxyAuthorityV2, operations: Vec<LivingCreatorOperationV2>) {
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
    #[test]
    fn creator_batch_real_step_marker_capacity_keeps_current_stale_cancelled_original_owners() {
        for trigger in [1, 64, 1024] {
            for stale in [false, true] {
                for cleanup in SCHEDULES {
                    let mut authority = crate::living::builtin_living_sandbox_v2().unwrap();
                    let system = authority.state().systems.as_slice()[0].id;
                    enqueue(
                        &mut authority,
                        vec![LivingCreatorOperationV2::EditSystem {
                            target: LivingCommandTargetV2::Existing { id: system },
                            name: name(),
                        }],
                    );
                    let capture = authority.capture_boundary().unwrap();
                    let before = authority.state().canonical_bytes().unwrap();
                    let mut job = Box::new(authority.begin_step().unwrap());
                    let step_pointer = std::ptr::from_ref(&*job);
                    let mut injected = false;
                    for _ in 0..10000 {
                        if job
                            .creator
                            .as_mut()
                            .and_then(|creator| creator.append.as_mut())
                            .is_some_and(|child| child.inject_entry_capacity_failure())
                        {
                            injected = true;
                            break;
                        }
                        assert_eq!(job.poll(1).unwrap(), LivingStepProgressV2::Pending);
                    }
                    assert!(injected);
                    let creator_pointer = std::ptr::from_ref(&**job.creator.as_ref().unwrap());
                    let before_visits = job.total_visits();
                    assert_eq!(
                        job.poll(trigger),
                        Err(LivingStepJobErrorV2::PreparationCapacity)
                    );
                    assert_eq!(job.last_poll_visits(), 1);
                    assert_eq!(job.total_visits(), before_visits + 1);
                    assert!(job.fault.is_none() && job.scheduled.is_none());
                    assert_eq!(authority.state().canonical_bytes().unwrap(), before);
                    assert!(authority.boundary_capture_is_current(&capture));
                    if stale {
                        enqueue(
                            &mut authority,
                            vec![LivingCreatorOperationV2::EditSystem {
                                target: LivingCommandTargetV2::Existing { id: system },
                                name: LivingNameV2::new("new tail").unwrap(),
                            }],
                        );
                    }
                    let current = authority.capture_boundary().unwrap();
                    assert!(
                        matches!(job.take_synchronous_preparation(&authority),Err(error) if error==if stale {LivingStepJobErrorV2::Stale}else{LivingStepJobErrorV2::PreparationCapacity})
                    );
                    assert!(!job.delivered && job.capture.is_present());
                    job.cancel();
                    assert!(matches!(
                        job.take_synchronous_preparation(&authority),
                        Err(LivingStepJobErrorV2::Cancelled)
                    ));
                    for budget in [0, 1, 1025] {
                        assert_eq!(
                            job.poll(budget),
                            Err(LivingStepJobErrorV2::PreparationCapacity)
                        );
                        assert_eq!(job.last_poll_visits(), 0);
                    }
                    assert_eq!(std::ptr::from_ref(&*job), step_pointer);
                    assert_eq!(
                        std::ptr::from_ref(&**job.creator.as_ref().unwrap()),
                        creator_pointer
                    );
                    let mut retirement = job
                        .into_boxed_retirement(LivingPublicationRetirementPolicyV2::ReleaseHandle);
                    for poll in 0..10000 {
                        assert!(poll < 9999);
                        let b = cleanup[poll % cleanup.len()];
                        let progress = retirement.poll(b).unwrap();
                        assert!(retirement.last_poll_visits() <= b);
                        if progress == LivingPublicationRetirementProgressV2::Ready {
                            break;
                        }
                    }
                    assert!(authority.boundary_capture_is_current(&current));
                    assert!(authority.fault.is_none());
                    assert_eq!(authority.queue.len(), if stale { 2 } else { 1 });
                }
            }
        }
    }
}

#[cfg(test)]
mod colony_cursor_tests {
    use super::*;
    use crate::living::model::*;
    use crate::living::{LivingNameV2, LivingSlugV2};
    pub(super) const SCHEDULES: &[&[usize]] = &[&[1], &[64], &[1024], &[1, 7, 64, 2, 1024]];
    pub(super) fn id(n: u8) -> LivingEntityIdV2 {
        LivingEntityIdV2([n; 16])
    }
    fn t(n: u8) -> LivingCommandTargetV2 {
        LivingCommandTargetV2::Existing { id: id(n) }
    }
    // Deliberately private populated projection: no system/catalog/genesis claim.
    // Three worlds/civilizations/colonies, two original reason rows per colony.
    pub(super) fn source(
        kind: usize,
    ) -> (
        Box<LivingQueuedCreatorJobV2>,
        Arc<Vec<QueuedCommandV2>>,
        Arc<ValidatedLivingCatalogPackV2>,
    ) {
        let catalog = Arc::new(crate::living::living_core_pack_v2().unwrap());
        let colonies = (1..=3)
            .map(|n| LivingColonyV2 {
                world: id(n),
                owner: id(5),
                hub: LivingHubV2 {
                    energy_next_due: LivingTickV2(40),
                    ore_next_due: LivingTickV2(400),
                    fallback_next_due: LivingTickV2(400),
                    blocked_reasons: LivingSortedVecV2::new(vec![
                        LivingBlockedReasonV2::Disabled,
                        LivingBlockedReasonV2::MissingDeposit {
                            required: 2,
                            available: 1,
                        },
                    ])
                    .unwrap(),
                },
            })
            .collect();
        let state = LivingGalaxyStateV2 {
            worlds: LivingSortedVecV2::new(
                (1..=3)
                    .map(|n| LivingWorldV2 {
                        id: id(n),
                        system: id(9),
                        archetype: LivingSlugV2::new("barren").unwrap(),
                        name: LivingNameV2::new("World").unwrap(),
                        owner: Some(id(5)),
                        inventory: Default::default(),
                    })
                    .collect(),
            )
            .unwrap(),
            civilizations: LivingSortedVecV2::new(
                (4..=6)
                    .map(|n| LivingCivilizationV2 {
                        id: id(n),
                        name: LivingNameV2::new("Actor").unwrap(),
                        policy: LivingPolicyV2(LivingSlugV2::new("neutral").unwrap()),
                        status: if n == 6 {
                            LivingCivilizationStatusV2::Active
                        } else {
                            LivingCivilizationStatusV2::Dormant
                        },
                    })
                    .collect(),
            )
            .unwrap(),
            colonies: LivingSortedVecV2::new(colonies).unwrap(),
            ..Default::default()
        };
        let operation = match kind {
            0 | 4 => LivingCreatorOperationV2::CreateColony {
                world: t(2),
                owner: t(4),
            },
            1 => LivingCreatorOperationV2::SetWorldOwner {
                world: t(2),
                owner: Some(t(4)),
            },
            2 => LivingCreatorOperationV2::SetWorldOwner {
                world: t(2),
                owner: None,
            },
            3 => LivingCreatorOperationV2::RemoveColony { world: t(2) },
            _ => unreachable!(),
        };
        let queue = super::creator_batch_cursor_tests::queue(vec![operation]);
        let mut job = Box::new(LivingQueuedCreatorJobV2::begin(
            state,
            Arc::clone(&queue),
            Arc::clone(&catalog),
        ));
        job.events
            .record(
                LivingEventProvenanceV2::Autonomous {
                    phase: LivingPhaseV2::ResolveArrivals,
                },
                LivingEventKindV2::FleetArrived,
            )
            .unwrap();
        if kind == 4 {
            job.events.creator_recoveries = (50..53)
                .map(|n| LivingCreatorRecoveryCandidateV2 {
                    revision: queue[0].revision.id,
                    actor: id(n),
                })
                .collect();
        }
        job.events = job.events.clone(); // Private fixture: full one-entry backing before ownership begins.
        (job, queue, catalog)
    }
    fn run(
        job: &mut LivingQueuedCreatorJobV2,
        schedule: &[usize],
    ) -> Result<usize, super::super::step_job::LivingLifecycleJobErrorV2> {
        let mut total = 0;
        for attempt in 0..10_000 {
            let budget = schedule[attempt % schedule.len()];
            let result = job.poll(budget);
            total += job.last_poll_visits;
            assert!(job.last_poll_visits <= budget);
            match result? {
                LivingStepProgressV2::NeedsSynchronousCompletion => return Ok(total),
                LivingStepProgressV2::Pending => assert!(job.last_poll_visits > 0),
                other => panic!("unexpected colony progress {other:?}"),
            }
        }
        panic!("finite colony cursor guard exhausted")
    }
    #[test]
    fn colony_cursor_reference_status_recovery_and_once_only_extraction() {
        for kind in 0..5 {
            for schedule in SCHEDULES {
                let (mut job, queue, catalog) = source(kind);
                let mut expected = job.state.clone();
                let mut events = job.events.clone();
                let (effects, candidates) =
                    apply_revision_with_recoveries(&mut expected, &queue[0].revision, &catalog)
                        .unwrap();
                assert!(effects.is_empty());
                events.creator_recoveries.extend(candidates);
                events
                    .record(
                        LivingEventProvenanceV2::Creator {
                            revision: queue[0].revision.id,
                        },
                        LivingEventKindV2::CreatorIntervention,
                    )
                    .unwrap();
                assert!(job.take_ready_result().is_err());
                run(&mut job, schedule).unwrap();
                assert_eq!(job.state, expected);
                assert_eq!(job.events, events);
                assert_eq!(
                    job.events.creator_recoveries.len(),
                    if kind == 4 { 4 } else { usize::from(kind < 2) }
                );
                if kind == 0 || kind == 4 {
                    let hub = &job.state.colonies.as_slice()[1].hub;
                    assert_eq!(
                        (
                            hub.energy_next_due.0,
                            hub.ore_next_due.0,
                            hub.fallback_next_due.0
                        ),
                        (11, 101, 101)
                    );
                    assert!(hub.blocked_reasons.as_slice().is_empty());
                }
                assert_eq!(
                    job.poll(1).unwrap(),
                    LivingStepProgressV2::NeedsSynchronousCompletion
                );
                assert_eq!(job.last_poll_visits, 0);
                let result = job.take_ready_result().unwrap();
                assert_eq!(result, (expected, events));
                assert!(job.take_ready_result().is_err());
            }
        }
    }
    #[test]
    fn colony_cursor_private_deadline_precedence_and_unpublished_owner() {
        for tick in [u64::MAX - 10, u64::MAX - 9] {
            for missing in [false, true] {
                let (mut job, _, _) = source(0);
                let queue = Arc::make_mut(&mut job.queue);
                Arc::make_mut(&mut queue[0].revision).tick = LivingTickV2(tick);
                if missing {
                    job.state.worlds = Default::default();
                }
                let old_colonies = job.state.colonies.clone();
                let result = run(&mut job, &[1]);
                let reason = if missing {
                    LivingCommandRejectionV2::MissingTarget {
                        operation: 0,
                        reference: "world",
                    }
                } else {
                    LivingCommandRejectionV2::Exhausted
                };
                assert_eq!(
                    result,
                    Err(super::super::step_job::LivingLifecycleJobErrorV2::Fault(
                        job.replay_fault(reason)
                    ))
                );
                assert_eq!(job.state.colonies, old_colonies);
                assert!(job.events.creator_recoveries.is_empty());
                assert_eq!(job.events.len(), 1);
                if !missing {
                    assert_eq!(job.state.worlds.as_slice()[1].owner, Some(id(4)));
                }
            }
        }
    }
    #[test]
    fn colony_cursor_each_operation_refresh_and_later_fault_before_marker() {
        for fail in [false, true] {
            let (mut job, _, _) = source(0);
            let operations = vec![
                LivingCreatorOperationV2::CreateColony {
                    world: t(2),
                    owner: t(4),
                },
                LivingCreatorOperationV2::RemoveColony {
                    world: t(if fail { 99 } else { 2 }),
                },
            ];
            job.queue = super::creator_batch_cursor_tests::queue(operations).into();
            let mut stopped = false;
            for _ in 0..10_000 {
                if job.operation_index == 1
                    && matches!(job.stage, QueuedCreatorStage::Operation)
                    && job.retirement.is_empty()
                {
                    stopped = true;
                    break;
                }
                assert_eq!(job.poll(1).unwrap(), LivingStepProgressV2::Pending);
            }
            assert!(stopped);
            assert_eq!(
                job.state.civilizations.as_slice()[0].status,
                LivingCivilizationStatusV2::Active
            );
            assert_eq!(
                job.state.civilizations.as_slice()[1].status,
                LivingCivilizationStatusV2::Active
            );
            assert_eq!(
                job.state.civilizations.as_slice()[2].status,
                LivingCivilizationStatusV2::Dormant
            );
            assert!(job.events.creator_recoveries.is_empty());
            if fail {
                for _ in 1..=u16::MAX {
                    job.events
                        .record(
                            LivingEventProvenanceV2::Autonomous {
                                phase: LivingPhaseV2::ResolveArrivals,
                            },
                            LivingEventKindV2::FleetArrived,
                        )
                        .unwrap();
                }
                let fault = run(&mut job, &[1]).unwrap_err();
                assert_eq!(
                    fault,
                    super::super::step_job::LivingLifecycleJobErrorV2::Fault(job.replay_fault(
                        LivingCommandRejectionV2::MissingTarget {
                            operation: 1,
                            reference: "colony"
                        }
                    ))
                );
                assert!(job.events.creator_recoveries.is_empty());
                assert!(job.append.is_none());
            } else {
                run(&mut job, &[1]).unwrap();
                assert_eq!(
                    job.state.civilizations.as_slice()[0].status,
                    LivingCivilizationStatusV2::Dormant
                );
                assert!(job.events.creator_recoveries.is_empty());
                assert_eq!(job.state.worlds.as_slice()[1].owner, Some(id(4)));
            }
        }
    }
    #[test]
    fn colony_cursor_private_reservation_before_missing_world_and_colony_before_reservations() {
        let catalog = crate::living::living_core_pack_v2().unwrap();
        let q = super::creator_batch_cursor_tests::queue(
            super::creator_batch_cursor_tests::operations(),
        );
        let mut templates = super::creator_batch_cursor_tests::base();
        super::creator_batch_cursor_tests::reference(&mut templates, &q, &catalog);
        for kind in [1, 2, 3] {
            for hull_only in [false, true] {
                let (mut job, _, _) = source(kind);
                let mut construction = templates.construction_jobs.as_slice()[0].clone();
                construction.world = id(2);
                construction.owner = id(5);
                let mut hull = templates.hull_jobs.as_slice()[0].clone();
                hull.world = id(2);
                hull.owner = id(5);
                job.state.construction_jobs = if hull_only {
                    Default::default()
                } else {
                    LivingSortedVecV2::new(vec![construction]).unwrap()
                };
                job.state.hull_jobs = LivingSortedVecV2::new(vec![hull]).unwrap();
                job.state.worlds = Default::default();
                if kind == 3 {
                    job.state.colonies = Default::default();
                }
                let reason = if kind == 3 {
                    LivingCommandRejectionV2::MissingTarget {
                        operation: 0,
                        reference: "colony",
                    }
                } else {
                    LivingCommandRejectionV2::Projection(LivingValidationErrorV2::Range {
                        field: "foreign_jobs_on_world",
                    })
                };
                assert_eq!(
                    run(&mut job, &[1]),
                    Err(super::super::step_job::LivingLifecycleJobErrorV2::Fault(
                        job.replay_fault(reason)
                    ))
                );
                assert!(job.events.creator_recoveries.is_empty());
                assert_eq!(job.events.len(), 1);
            }
        }
    }

    #[test]
    fn colony_cursor_private_current_and_stale_failed_tokens_preserve_accepted_tail() {
        for stale in [false, true] {
            let mut authority = crate::living::builtin_living_sandbox_v2().unwrap();
            let row = authority.state.colonies.as_slice()[0].clone();
            let cursor = authority.published_cursor();
            authority
                .submit(LivingCommandEnvelopeV2 {
                    expected_committed_revision: cursor.committed_revision,
                    expected_tick: cursor.tick,
                    expected_pending_sequence: cursor.pending_sequence,
                    mode: LivingCommandModeV2::Running,
                    command: LivingCommandV2::CreatorBatch {
                        operations: vec![LivingCreatorOperationV2::CreateColony {
                            world: LivingCommandTargetV2::Existing { id: row.world },
                            owner: LivingCommandTargetV2::Existing { id: row.owner },
                        }],
                    },
                })
                .unwrap();
            // Explicit post-admission metadata fault injection, not reachable genesis.
            Arc::make_mut(&mut Arc::make_mut(&mut authority.queue)[0].revision).tick =
                LivingTickV2(u64::MAX - 9);
            let state = authority.state.clone();
            let high_water = authority.accepted_sequence_high_water;
            let queue = Arc::clone(&authority.queue);
            let mut job = Box::new(authority.begin_step().unwrap());
            let mut failed = false;
            for _ in 0..100_000 {
                match job.poll(64) {
                    Err(LivingStepJobErrorV2::Fault(LivingDeterministicFaultV2::Replay {
                        accepted_sequence: 1,
                        reason,
                    })) => {
                        assert_eq!(*reason, LivingCommandRejectionV2::Exhausted);
                        failed = true;
                        break;
                    }
                    Ok(LivingStepProgressV2::Pending) => assert!(job.last_poll_visits() > 0),
                    other => panic!("unexpected private injected result {other:?}"),
                }
            }
            assert!(failed);
            let LivingStepPreparationV2::Failed(token) =
                job.take_synchronous_preparation(&authority).unwrap()
            else {
                panic!("failed token")
            };
            if stale {
                authority.accepted_sequence_high_water += 1;
            }
            let result = authority.commit_step_failure(token);
            if stale {
                assert_eq!(result.unwrap_err().reason(), &LivingStepJobErrorV2::Stale);
                assert!(authority.fault.is_none());
            } else {
                drop(result.unwrap());
                assert!(authority.fault.is_some());
            }
            assert!(Arc::ptr_eq(&authority.state, &state));
            assert!(Arc::ptr_eq(&authority.queue, &queue));
            assert_eq!(
                authority.accepted_sequence_high_water,
                high_water + u64::from(stale)
            );
            assert!(authority.revisions.is_empty());
            assert!(authority.last_receipt.is_none());
            assert!(authority.last_creator_recoveries.is_empty());
            let mut retirement =
                job.into_boxed_retirement(LivingPublicationRetirementPolicyV2::ReleaseHandle);
            let mut done = false;
            for _ in 0..100_000 {
                if retirement.poll(64).unwrap() == LivingPublicationRetirementProgressV2::Ready {
                    done = true;
                    break;
                }
            }
            assert!(done);
        }
    }

    #[test]
    fn colony_cursor_private_divergent_accepted_metadata_keeps_original_fault_sequence() {
        for kind in 0..4 {
            let (mut job, _, _) = source(if kind == 3 { 0 } else { kind });
            let queue = Arc::make_mut(&mut job.queue);
            Arc::make_mut(&mut queue[0].accepted).accepted_sequence = 77;
            assert_eq!(queue[0].revision.ordinal, 1);
            let reason = if kind == 3 {
                Arc::make_mut(&mut queue[0].revision).tick = LivingTickV2(u64::MAX - 9);
                LivingCommandRejectionV2::Exhausted
            } else {
                job.state.worlds = Default::default();
                LivingCommandRejectionV2::MissingTarget {
                    operation: 0,
                    reference: "world",
                }
            };
            assert_eq!(
                run(&mut job, &[1]),
                Err(super::super::step_job::LivingLifecycleJobErrorV2::Fault(
                    LivingDeterministicFaultV2::Replay {
                        accepted_sequence: 77,
                        reason: Box::new(reason)
                    }
                ))
            );
        }
        // Existing covered SetWorldInventory's private ordinal discrepancy is
        // unchanged; this malformed metadata is not public replay evidence.
        let (mut job, _, _) = source(0);
        job.queue = super::creator_batch_cursor_tests::queue(vec![
            LivingCreatorOperationV2::SetWorldInventory {
                world: t(99),
                inventory: Default::default(),
            },
        ])
        .into();
        Arc::make_mut(&mut Arc::make_mut(&mut job.queue)[0].accepted).accepted_sequence = 77;
        assert_eq!(
            run(&mut job, &[1]),
            Err(super::super::step_job::LivingLifecycleJobErrorV2::Fault(
                LivingDeterministicFaultV2::Replay {
                    accepted_sequence: 1,
                    reason: Box::new(LivingCommandRejectionV2::MissingTarget {
                        operation: 0,
                        reference: "world"
                    })
                }
            ))
        );
    }
}

#[cfg(test)]
mod diplomacy_creator_tests {
    use super::*;
    use crate::living::model::*;
    use crate::living::{LivingNameV2, LivingSlugV2};
    pub(super) const SCHEDULES: &[&[usize]] = &[&[1], &[64], &[1024], &[1, 7, 64, 2, 1024]];
    pub(super) fn id(n: u8) -> LivingEntityIdV2 {
        LivingEntityIdV2([n; 16])
    }
    pub(super) fn t(n: u8) -> LivingCommandTargetV2 {
        LivingCommandTargetV2::Existing { id: id(n) }
    }
    pub(super) fn operation(kind: usize) -> LivingCreatorOperationV2 {
        match kind {
            0 => LivingCreatorOperationV2::SetRelationBase {
                from: t(4),
                to: t(5),
                base_disposition: 17,
            },
            1 => LivingCreatorOperationV2::SetAgreement {
                kind: LivingAgreementKindV2::Trade,
                participant_a: t(5),
                participant_b: t(4),
                start_tick: LivingTickV2(2),
                end_tick: LivingTickV2(80),
            },
            2 => LivingCreatorOperationV2::RemoveAgreement {
                kind: LivingAgreementKindV2::Trade,
                participant_a: t(5),
                participant_b: t(4),
            },
            3 => LivingCreatorOperationV2::ForceWar {
                participant_a: t(5),
                participant_b: t(4),
                declarer: t(5),
                start_tick: LivingTickV2(3),
                end_tick: LivingTickV2(90),
            },
            4 => LivingCreatorOperationV2::ForcePeace {
                participant_a: t(5),
                participant_b: t(4),
            },
            5..=7 => LivingCreatorOperationV2::SetFleetOrder {
                fleet: t(20),
                kind: (kind != 6).then_some(LivingFleetOrderKindV2::Return),
                target_world: Some(t(2)),
                not_before_tick: LivingTickV2(81),
                lane_path: if kind == 7 {
                    vec![]
                } else {
                    vec![t(32), t(31), t(32), t(30)]
                },
            },
            _ => unreachable!(),
        }
    }
    // Private populated projection: authored source references/catalog topology
    // intentionally absent. Public validated/admitted controls are integration tests.
    pub(super) fn source(
        kind: usize,
    ) -> (
        Box<LivingQueuedCreatorJobV2>,
        Arc<Vec<QueuedCommandV2>>,
        Arc<ValidatedLivingCatalogPackV2>,
    ) {
        let catalog = Arc::new(crate::living::living_core_pack_v2().unwrap());
        let mut state = LivingGalaxyStateV2 {
            worlds: LivingSortedVecV2::new(
                (1..=3)
                    .map(|n| LivingWorldV2 {
                        id: id(n),
                        system: id(9),
                        archetype: LivingSlugV2::new("barren").unwrap(),
                        name: LivingNameV2::new("World").unwrap(),
                        owner: match n {
                            1 => Some(id(4)),
                            2 => Some(id(5)),
                            _ => None,
                        },
                        inventory: Default::default(),
                    })
                    .collect(),
            )
            .unwrap(),
            civilizations: LivingSortedVecV2::new(
                (4..=6)
                    .map(|n| LivingCivilizationV2 {
                        id: id(n),
                        name: LivingNameV2::new("Actor").unwrap(),
                        policy: LivingPolicyV2(LivingSlugV2::new("neutral").unwrap()),
                        status: LivingCivilizationStatusV2::Active,
                    })
                    .collect(),
            )
            .unwrap(),
            relations: LivingSortedVecV2::new(
                [(4, 5), (5, 4), (5, 6)]
                    .into_iter()
                    .map(|(a, b)| LivingRelationV2 {
                        from: id(a),
                        to: id(b),
                        base_disposition: 1,
                        reasons: LivingRelationReasonsV2 {
                            delivered_aid: 10,
                            successful_trade: 3,
                            aid_units_remainder: 7,
                            trade_units_remainder: 9,
                            ..Default::default()
                        },
                    })
                    .collect(),
            )
            .unwrap(),
            wars: LivingSortedVecV2::new(
                [(4, 5), (4, 6), (5, 6)]
                    .into_iter()
                    .map(|(a, b)| LivingWarV2 {
                        participant_a: id(a),
                        participant_b: id(b),
                        declarer: id(a),
                        start_tick: LivingTickV2(0),
                        end_tick: LivingTickV2(100),
                    })
                    .collect(),
            )
            .unwrap(),
            occupations: LivingSortedVecV2::new(
                (1..=3)
                    .map(|n| LivingOccupationV2 {
                        world: id(n),
                        claimant: id(if n == 1 { 5 } else { 4 }),
                        progress_ticks: 7,
                    })
                    .collect(),
            )
            .unwrap(),
            fleets: LivingSortedVecV2::new(vec![LivingFleetV2 {
                id: id(20),
                owner: id(4),
                location: LivingFleetLocationV2::Docked { world: id(1) },
                order: Some(LivingFleetOrderV2 {
                    kind: LivingFleetOrderKindV2::Move,
                    target_world: Some(id(1)),
                    not_before_tick: LivingTickV2(70),
                    lane_path: vec![id(30), id(31), id(30)],
                }),
                hulls: LivingSortedVecV2::new(
                    (21..=22)
                        .map(|n| LivingHullV2 {
                            id: id(n),
                            definition: LivingSlugV2::new("scout").unwrap(),
                            hit_points: 7,
                            return_credit: n == 21,
                            next_repair: Some(LivingTickV2(40)),
                        })
                        .collect(),
                )
                .unwrap(),
            }])
            .unwrap(),
            ..Default::default()
        };
        let mut agreements = vec![];
        for kind in [
            LivingAgreementKindV2::Nonaggression,
            LivingAgreementKindV2::Trade,
            LivingAgreementKindV2::Truce,
        ] {
            agreements.push(LivingAgreementV2 {
                kind,
                participant_a: id(4),
                participant_b: id(5),
                start_tick: LivingTickV2(0),
                end_tick: LivingTickV2(100),
            });
        }
        agreements.push(LivingAgreementV2 {
            kind: LivingAgreementKindV2::Trade,
            participant_a: id(5),
            participant_b: id(6),
            start_tick: LivingTickV2(200),
            end_tick: LivingTickV2(300),
        });
        state.agreements = LivingSortedVecV2::new(agreements).unwrap();
        let queue = super::creator_batch_cursor_tests::queue(vec![operation(kind)]);
        let mut job = Box::new(LivingQueuedCreatorJobV2::begin(
            state,
            Arc::clone(&queue),
            Arc::clone(&catalog),
        ));
        job.events
            .record(
                LivingEventProvenanceV2::Autonomous {
                    phase: LivingPhaseV2::ResolveArrivals,
                },
                LivingEventKindV2::FleetArrived,
            )
            .unwrap();
        job.events = job.events.clone();
        (job, queue, catalog)
    }
    pub(super) fn run(
        job: &mut LivingQueuedCreatorJobV2,
        schedule: &[usize],
    ) -> Result<usize, super::super::step_job::LivingLifecycleJobErrorV2> {
        let mut total = 0;
        for attempt in 0..10_000 {
            let budget = schedule[attempt % schedule.len()];
            let result = job.poll(budget);
            total += job.last_poll_visits;
            assert!(job.last_poll_visits <= budget);
            match result? {
                LivingStepProgressV2::NeedsSynchronousCompletion => return Ok(total),
                LivingStepProgressV2::Pending => assert!(job.last_poll_visits > 0),
                other => panic!("unexpected diplomacy creator progress {other:?}"),
            }
        }
        panic!("finite diplomacy creator guard")
    }
    #[test]
    fn diplomacy_creator_reference_and_exact_order_fields() {
        for kind in 0..8 {
            for schedule in SCHEDULES {
                let (mut job, queue, catalog) = source(kind);
                let mut expected = job.state.clone();
                let mut events = job.events.clone();
                let (effects, recoveries) =
                    apply_revision_with_recoveries(&mut expected, &queue[0].revision, &catalog)
                        .unwrap();
                assert!(effects.is_empty() && recoveries.is_empty());
                events
                    .record(
                        LivingEventProvenanceV2::Creator {
                            revision: queue[0].revision.id,
                        },
                        LivingEventKindV2::CreatorIntervention,
                    )
                    .unwrap();
                let hull_pointer = job.state.fleets.as_slice()[0].hulls.as_slice().as_ptr();
                assert!(job.take_ready_result().is_err());
                run(&mut job, schedule).unwrap();
                assert_eq!(job.state, expected);
                assert_eq!(job.events, events);
                assert_eq!(
                    job.state.fleets.as_slice()[0].hulls.as_slice().as_ptr(),
                    hull_pointer
                );
                match kind {
                    0 => {
                        assert_eq!(job.state.relations.as_slice()[0].base_disposition, 17);
                        assert_eq!(
                            job.state.relations.as_slice()[0]
                                .reasons
                                .aid_units_remainder,
                            7
                        );
                        assert_eq!(job.state.relations.as_slice()[1].base_disposition, 1);
                    }
                    3 => {
                        assert_eq!(job.state.agreements.as_slice().len(), 1);
                        assert_eq!(job.state.wars.as_slice()[0].declarer, id(5));
                    }
                    4 => {
                        assert_eq!(job.state.wars.as_slice().len(), 2);
                        assert_eq!(
                            job.state.occupations.as_slice(),
                            &[LivingOccupationV2 {
                                world: id(3),
                                claimant: id(4),
                                progress_ticks: 7
                            }]
                        );
                    }
                    5 => {
                        let order = job.state.fleets.as_slice()[0].order.as_ref().unwrap();
                        assert_eq!(order.lane_path, vec![id(32), id(31), id(32), id(30)]);
                        assert_eq!(order.not_before_tick, LivingTickV2(81));
                    }
                    6 => assert!(job.state.fleets.as_slice()[0].order.is_none()),
                    7 => assert!(
                        job.state.fleets.as_slice()[0]
                            .order
                            .as_ref()
                            .unwrap()
                            .lane_path
                            .is_empty()
                    ),
                    _ => {}
                }
                assert_eq!(
                    job.poll(1).unwrap(),
                    LivingStepProgressV2::NeedsSynchronousCompletion
                );
                assert_eq!(job.last_poll_visits, 0);
                assert_eq!(job.take_ready_result().unwrap(), (expected, events));
                assert!(job.take_ready_result().is_err());
            }
        }
    }
    #[test]
    fn diplomacy_creator_private_precedence_clear_mapping_and_current_owner() {
        for schedule in SCHEDULES {
            for control in 0..6 {
                let (mut job, _, catalog) = source(if control < 3 { 2 } else { 5 });
                let expected = match control {
                    0 => {
                        job.state.civilizations = Default::default();
                        LivingCommandRejectionV2::MissingTarget {
                            operation: 0,
                            reference: "civilization",
                        }
                    }
                    1 => {
                        job.state.agreements = Default::default();
                        LivingCommandRejectionV2::MissingTarget {
                            operation: 0,
                            reference: "agreement",
                        }
                    }
                    2 => {
                        job.queue = super::creator_batch_cursor_tests::queue(vec![
                            LivingCreatorOperationV2::RemoveAgreement {
                                kind: LivingAgreementKindV2::Trade,
                                participant_a: t(4),
                                participant_b: t(4),
                            },
                        ])
                        .into();
                        LivingCommandRejectionV2::MissingTarget {
                            operation: 0,
                            reference: "agreement",
                        }
                    }
                    3 => {
                        job.state.fleets = Default::default();
                        LivingCommandRejectionV2::MissingTarget {
                            operation: 0,
                            reference: "fleet",
                        }
                    }
                    _ => {
                        let mut templates = super::creator_batch_cursor_tests::base();
                        let all = super::creator_batch_cursor_tests::queue(
                            super::creator_batch_cursor_tests::operations(),
                        );
                        super::creator_batch_cursor_tests::reference(
                            &mut templates,
                            &all,
                            &catalog,
                        );
                        let mut row = templates.hull_jobs.as_slice()[0].clone();
                        row.target_fleet = Some(id(20));
                        job.state.hull_jobs = LivingSortedVecV2::new(vec![row]).unwrap();
                        if control == 4 {
                            job.state.fleets = Default::default();
                        }
                        if control == 5 {
                            job.queue =
                                super::creator_batch_cursor_tests::queue(vec![operation(6)]).into();
                        }
                        LivingCommandRejectionV2::ReferencedTarget
                    }
                };
                // Private accepted metadata mismatch: diagnostic uses accepted
                // sequence, never revision ordinal or operation count.
                Arc::make_mut(&mut Arc::make_mut(&mut job.queue)[0].accepted).accepted_sequence =
                    77;
                assert_eq!(
                    run(&mut job, schedule),
                    Err(super::super::step_job::LivingLifecycleJobErrorV2::Fault(
                        LivingDeterministicFaultV2::Replay {
                            accepted_sequence: 77,
                            reason: Box::new(expected)
                        }
                    ))
                );
                assert_eq!(job.events.len(), 1);
                assert!(job.events.creator_recoveries.is_empty());
                assert!(job.append.is_none());
            }
            let (mut job, _, _) = source(6);
            // None execution ignores unresolved locals in a deliberately private
            // injected queue; public admission rejects these separately.
            let mut clear = operation(6);
            if let LivingCreatorOperationV2::SetFleetOrder {
                target_world,
                lane_path,
                ..
            } = &mut clear
            {
                *target_world = Some(LivingCommandTargetV2::Local { local: 77 });
                *lane_path = vec![LivingCommandTargetV2::Local { local: 78 }];
            }
            // Seal a valid original first; this is a private post-sealing
            // execution-only mutation, never an admitted malformed command.
            job.queue = super::creator_batch_cursor_tests::queue(vec![operation(6)]).into();
            Arc::make_mut(&mut Arc::make_mut(&mut job.queue)[0].revision).command =
                LivingCommandV2::CreatorBatch {
                    operations: vec![clear],
                };
            run(&mut job, schedule).unwrap();
            assert!(job.state.fleets.as_slice()[0].order.is_none());
            let (mut job, _, _) = source(4);
            job.queue = super::creator_batch_cursor_tests::queue(vec![
                LivingCreatorOperationV2::SetWorldOwner {
                    world: t(1),
                    owner: Some(t(6)),
                },
                operation(4),
            ])
            .into();
            run(&mut job, schedule).unwrap();
            assert_eq!(
                job.state
                    .occupations
                    .as_slice()
                    .iter()
                    .map(|r| r.world)
                    .collect::<Vec<_>>(),
                vec![id(1), id(3)]
            );
            // Missing world is a private malformed-state None lookup, not a new error.
            let (mut job, _, _) = source(4);
            job.state.worlds = Default::default();
            run(&mut job, schedule).unwrap();
            assert_eq!(job.state.occupations.as_slice().len(), 3);
        }
    }
}

#[cfg(test)]
mod cancellation_creator_tests {
    use super::*;
    use crate::living::model::*;
    use crate::living::{LivingNameV2, LivingSlugV2};
    pub(super) const SCHEDULES: &[&[usize]] = &[&[1], &[64], &[1024], &[1, 7, 64, 2, 1024]];
    pub(super) fn id(n: u8) -> LivingEntityIdV2 {
        LivingEntityIdV2([n; 16])
    }
    fn t(n: u8) -> LivingCommandTargetV2 {
        LivingCommandTargetV2::Existing { id: id(n) }
    }
    pub(super) fn operation(kind: usize) -> LivingCreatorOperationV2 {
        match kind {
            0 => LivingCreatorOperationV2::CancelConstructionJob { target: t(11) },
            1 => LivingCreatorOperationV2::CancelHullJob { target: t(21) },
            2 => LivingCreatorOperationV2::EditRoute {
                target: t(31),
                kind: LivingRouteKindV2::Trade,
                source_world: t(1),
                destination_world: t(2),
                resource: LivingResourceV2::Alloy,
                batch_size: 7,
                cadence_ticks: 10,
                source_reserve: 20,
                source_owner: t(4),
                receiver_owner: t(5),
            },
            3 | 4 => LivingCreatorOperationV2::SetRouteSuspended {
                target: t(31),
                suspended: kind == 3,
            },
            5 => LivingCreatorOperationV2::CancelHazard { target: t(41) },
            _ => unreachable!(),
        }
    }
    // Private populated projection: absent system/catalog topology is deliberate.
    // Public validated/admitted fixtures are in living_creator_additions.rs.
    pub(super) fn source(
        kind: usize,
    ) -> (
        Box<LivingQueuedCreatorJobV2>,
        Arc<Vec<QueuedCommandV2>>,
        Arc<ValidatedLivingCatalogPackV2>,
    ) {
        if kind == 6 {
            // Private populated bridge-owner seam, not an admitted unsupported effect.
            // Original prefix/suffix allocations exist before any counted bridge visit.
            let (mut job, queue, catalog) = source(5);
            job.stage = QueuedCreatorStage::BridgeEffectsStart;
            job.colony_changed = true;
            job.effect_work = Some(QueuedCreatorEffects {
                values: vec![
                    LivingEventKindV2::RefundDiscarded,
                    LivingEventKindV2::ConstructionCancelled,
                ],
                suffix: vec![
                    LivingEventKindV2::RefundDiscarded,
                    LivingEventKindV2::ConstructionCancelled,
                ],
                ..Default::default()
            });
            return (job, queue, catalog);
        }
        let catalog = Arc::new(crate::living::living_core_pack_v2().unwrap());
        let state = LivingGalaxyStateV2 {
            worlds: LivingSortedVecV2::new(
                (1..=2)
                    .map(|n| LivingWorldV2 {
                        id: id(n),
                        system: id(9),
                        archetype: LivingSlugV2::new("barren").unwrap(),
                        name: LivingNameV2::new("World").unwrap(),
                        owner: Some(id(n + 3)),
                        inventory: LivingInventoryV2 {
                            energy: 9,
                            ore: 8,
                            alloy: 10_000,
                        },
                    })
                    .collect(),
            )
            .unwrap(),
            civilizations: LivingSortedVecV2::new(
                (4..=5)
                    .map(|n| LivingCivilizationV2 {
                        id: id(n),
                        name: LivingNameV2::new("Actor").unwrap(),
                        policy: LivingPolicyV2(LivingSlugV2::new("neutral").unwrap()),
                        status: LivingCivilizationStatusV2::Active,
                    })
                    .collect(),
            )
            .unwrap(),
            construction_jobs: LivingSortedVecV2::new(
                (10..=12)
                    .map(|n| LivingConstructionJobV2 {
                        id: id(n),
                        world: id(1),
                        owner: id(4),
                        definition: LivingSlugV2::new("defense_battery").unwrap(),
                        accepted_tick: LivingTickV2(0),
                        completion_tick: LivingTickV2(100),
                        paid_alloy: 5,
                    })
                    .collect(),
            )
            .unwrap(),
            hull_jobs: LivingSortedVecV2::new(
                (20..=22)
                    .map(|n| LivingHullJobV2 {
                        id: id(n),
                        world: id(1),
                        owner: id(4),
                        definition: LivingSlugV2::new("scout").unwrap(),
                        accepted_tick: LivingTickV2(0),
                        completion_tick: LivingTickV2(100),
                        paid_alloy: 5,
                        target_fleet: Some(id(60)),
                    })
                    .collect(),
            )
            .unwrap(),
            routes: LivingSortedVecV2::new(
                (30..=32)
                    .map(|n| LivingFreightRouteV2 {
                        id: id(n),
                        kind: LivingRouteKindV2::Trade,
                        source_world: id(1),
                        destination_world: id(2),
                        resource: LivingResourceV2::Ore,
                        batch_size: 3,
                        cadence_ticks: 10,
                        source_reserve: 20,
                        source_owner: id(4),
                        receiver_owner: id(5),
                        next_due: LivingTickV2(if kind == 4 { 50 } else { 1 }),
                        suspended: true,
                    })
                    .collect(),
            )
            .unwrap(),
            hazards: LivingSortedVecV2::new(
                (40..=42)
                    .map(|n| LivingHazardV2 {
                        id: id(n),
                        lane: id(9),
                        definition: LivingSlugV2::new("ion_storm").unwrap(),
                        start_tick: LivingTickV2(100),
                        end_tick: LivingTickV2(200),
                    })
                    .collect(),
            )
            .unwrap(),
            wars: LivingSortedVecV2::new(vec![LivingWarV2 {
                participant_a: id(4),
                participant_b: id(5),
                declarer: id(4),
                start_tick: LivingTickV2(0),
                end_tick: LivingTickV2(1),
            }])
            .unwrap(),
            agreements: LivingSortedVecV2::new(vec![LivingAgreementV2 {
                kind: LivingAgreementKindV2::Trade,
                participant_a: id(4),
                participant_b: id(5),
                start_tick: LivingTickV2(1),
                end_tick: LivingTickV2(100),
            }])
            .unwrap(),
            shipments: LivingSortedVecV2::new(vec![LivingShipmentV2 {
                id: id(50),
                dispatch_owner: id(4),
                intended_receiver: id(5),
                source_world: id(1),
                destination_world: id(2),
                resource: LivingResourceV2::Ore,
                units: 3,
                departure_tick: LivingTickV2(0),
                arrival_tick: LivingTickV2(100),
                disposition: LivingShipmentDispositionV2::Outbound,
                dispatch_kind: Some(LivingRouteKindV2::Trade),
                returning_to_source: false,
            }])
            .unwrap(),
            fleets: LivingSortedVecV2::new(vec![LivingFleetV2 {
                id: id(60),
                owner: id(4),
                location: LivingFleetLocationV2::Docked { world: id(1) },
                order: Some(LivingFleetOrderV2 {
                    kind: LivingFleetOrderKindV2::Move,
                    target_world: Some(id(2)),
                    not_before_tick: LivingTickV2(70),
                    lane_path: vec![id(8), id(9), id(8)],
                }),
                hulls: LivingSortedVecV2::new(
                    (61..=62)
                        .map(|n| LivingHullV2 {
                            id: id(n),
                            definition: LivingSlugV2::new("scout").unwrap(),
                            hit_points: 7,
                            return_credit: true,
                            next_repair: Some(LivingTickV2(40)),
                        })
                        .collect(),
                )
                .unwrap(),
            }])
            .unwrap(),
            ..Default::default()
        };
        let queue = super::creator_batch_cursor_tests::queue(vec![operation(kind)]);
        let mut job = Box::new(LivingQueuedCreatorJobV2::begin(
            state,
            Arc::clone(&queue),
            Arc::clone(&catalog),
        ));
        job.events
            .record(
                LivingEventProvenanceV2::Autonomous {
                    phase: LivingPhaseV2::ResolveArrivals,
                },
                LivingEventKindV2::FleetArrived,
            )
            .unwrap();
        job.events = job.events.clone(); // Exact one-entry backing for the immutable fixture.
        (job, queue, catalog)
    }
    pub(super) fn run(
        job: &mut LivingQueuedCreatorJobV2,
        schedule: &[usize],
    ) -> Result<usize, super::super::step_job::LivingLifecycleJobErrorV2> {
        let mut total = 0;
        for attempt in 0..100_000 {
            let b = schedule[attempt % schedule.len()];
            let result = job.poll(b);
            total += job.last_poll_visits;
            assert!(job.last_poll_visits <= b);
            match result? {
                LivingStepProgressV2::Pending => assert!(job.last_poll_visits > 0),
                _ => return Ok(total),
            }
        }
        panic!("finite cancellation creator guard")
    }
    fn reference(
        job: &LivingQueuedCreatorJobV2,
    ) -> Result<(LivingGalaxyStateV2, LivingPendingEventsV2), LivingCommandRejectionV2> {
        let mut state = job.state.clone();
        let mut events = job.events.clone();
        for queued in job.queue.iter() {
            let (effects, candidates) =
                apply_revision_with_recoveries(&mut state, &queued.revision, &job.catalog)?;
            events.creator_recoveries.extend(candidates);
            events
                .record(
                    LivingEventProvenanceV2::Creator {
                        revision: queued.revision.id,
                    },
                    LivingEventKindV2::CreatorIntervention,
                )
                .unwrap();
            for kind in effects {
                events
                    .record(
                        LivingEventProvenanceV2::Creator {
                            revision: queued.revision.id,
                        },
                        kind,
                    )
                    .unwrap();
            }
        }
        Ok((state, events))
    }
    #[test]
    fn cancellation_creator_reference_refund_order_and_unchanged_owners() {
        for kind in 0..6 {
            for schedule in SCHEDULES {
                let (mut job, _, _) = source(kind);
                let expected = reference(&job).unwrap();
                let shipments = job.state.shipments.as_slice().as_ptr();
                let hulls = job.state.fleets.as_slice()[0].hulls.as_slice().as_ptr();
                assert!(job.take_ready_result().is_err());
                for invalid in [0, 1025] {
                    assert!(job.poll(invalid).is_err());
                    assert_eq!(job.last_poll_visits, 0);
                }
                run(&mut job, schedule).unwrap();
                assert!(matches!(job.stage, QueuedCreatorStage::Ready));
                assert_eq!(job.state, expected.0);
                assert_eq!(job.events, expected.1);
                assert_eq!(job.state.shipments.as_slice().as_ptr(), shipments);
                assert_eq!(
                    job.state.fleets.as_slice()[0].hulls.as_slice().as_ptr(),
                    hulls
                );
                assert_eq!(
                    job.events
                        .payloads()
                        .iter()
                        .map(|e| e.kind)
                        .collect::<Vec<_>>(),
                    if kind < 2 {
                        vec![
                            LivingEventKindV2::FleetArrived,
                            LivingEventKindV2::CreatorIntervention,
                            LivingEventKindV2::RefundDiscarded,
                            LivingEventKindV2::ConstructionCancelled,
                        ]
                    } else {
                        vec![
                            LivingEventKindV2::FleetArrived,
                            LivingEventKindV2::CreatorIntervention,
                        ]
                    }
                );
                assert_eq!(
                    job.poll(64).unwrap(),
                    LivingStepProgressV2::NeedsSynchronousCompletion
                );
                assert_eq!(job.last_poll_visits, 0);
                assert_eq!(job.take_ready_result().unwrap(), expected);
                assert!(job.take_ready_result().is_err());
            }
        }
        for kind in 0..2 {
            for paid in [0, 1, 2, 5, 6, u32::MAX] {
                for alloy in [0, 9997, 9998, 9999, 10000] {
                    let (mut job, _, _) = source(kind);
                    job.state.worlds.find_mut(&id(1)).unwrap().inventory.alloy = alloy;
                    if kind == 0 {
                        job.state
                            .construction_jobs
                            .find_mut(&id(11))
                            .unwrap()
                            .paid_alloy = paid;
                    } else {
                        job.state.hull_jobs.find_mut(&id(21)).unwrap().paid_alloy = paid;
                    }
                    let expected = reference(&job).unwrap();
                    run(&mut job, SCHEDULES[3]).unwrap();
                    assert_eq!((job.state, job.events), expected);
                }
            }
        }
    }
    #[test]
    fn cancellation_creator_private_missing_world_overflow_missing_target_and_late_fault() {
        for kind in 0..2 {
            for control in 0..4 {
                for schedule in SCHEDULES {
                    let (mut job, _, _) = source(kind);
                    match control {
                        0 => {
                            job.state.worlds = Default::default();
                        }
                        1 => {
                            job.state.worlds.find_mut(&id(1)).unwrap().inventory.alloy = u32::MAX;
                        }
                        2 => {
                            if kind == 0 {
                                job.state.construction_jobs = Default::default();
                            } else {
                                job.state.hull_jobs = Default::default();
                            }
                        }
                        _ => {
                            job.queue = super::creator_batch_cursor_tests::queue(vec![
                                operation(kind),
                                operation(kind),
                            ])
                            .into();
                        }
                    }
                    let expected = reference(&job);
                    let original_events = job.events.clone();
                    match expected {
                        Ok(expected) => {
                            run(&mut job, schedule).unwrap();
                            assert_eq!((job.state, job.events), expected);
                        }
                        Err(reason) => {
                            assert_eq!(
                                run(&mut job, schedule),
                                Err(super::super::step_job::LivingLifecycleJobErrorV2::Fault(
                                    LivingDeterministicFaultV2::Replay {
                                        accepted_sequence: 1,
                                        reason: Box::new(reason)
                                    }
                                ))
                            );
                            assert_eq!(job.events, original_events);
                            if control == 3 {
                                assert_eq!(job.effect_work.as_ref().unwrap().values.len(), 2);
                            }
                        }
                    }
                }
            }
        }
    }
    #[test]
    fn cancellation_creator_route_permission_half_open_current_owners_and_deadlines() {
        for kind in [2, 3, 4] {
            for control in 0..13 {
                for schedule in SCHEDULES {
                    let (mut job, _, _) = source(kind);
                    match control {
                        0 => job.state.worlds.find_mut(&id(1)).unwrap().owner = None,
                        1 => job.state.worlds.find_mut(&id(2)).unwrap().owner = Some(id(4)),
                        2 => job.state.wars.record_at_mut(0).unwrap().end_tick = LivingTickV2(2),
                        3 => job.state.wars.record_at_mut(0).unwrap().start_tick = LivingTickV2(2),
                        4 => {
                            job.state.agreements.record_at_mut(0).unwrap().end_tick =
                                LivingTickV2(1)
                        }
                        5 => {
                            job.state.agreements.record_at_mut(0).unwrap().start_tick =
                                LivingTickV2(2)
                        }
                        6 => job.state.routes.find_mut(&id(31)).unwrap().next_due = LivingTickV2(0),
                        7 => job.state.routes.find_mut(&id(31)).unwrap().next_due = LivingTickV2(1),
                        8 => {
                            job.state.routes.find_mut(&id(31)).unwrap().next_due = LivingTickV2(40)
                        }
                        9 => job.state.routes.find_mut(&id(31)).unwrap().suspended = false,
                        10 => {
                            job.queue = super::creator_batch_cursor_tests::queue(vec![
                                LivingCreatorOperationV2::ForceWar {
                                    participant_a: t(4),
                                    participant_b: t(5),
                                    declarer: t(4),
                                    start_tick: LivingTickV2(1),
                                    end_tick: LivingTickV2(100),
                                },
                                operation(kind),
                            ])
                            .into();
                        }
                        11 => {
                            job.queue = super::creator_batch_cursor_tests::queue(vec![
                                LivingCreatorOperationV2::SetWorldOwner {
                                    world: t(2),
                                    owner: Some(t(4)),
                                },
                                operation(kind),
                            ])
                            .into();
                        }
                        _ => {
                            job.state.routes = Default::default();
                        }
                    }
                    let expected = reference(&job);
                    match expected {
                        Ok(expected) => {
                            run(&mut job, schedule).unwrap();
                            assert_eq!((job.state, job.events), expected);
                        }
                        Err(reason) => assert_eq!(
                            run(&mut job, schedule),
                            Err(super::super::step_job::LivingLifecycleJobErrorV2::Fault(
                                LivingDeterministicFaultV2::Replay {
                                    accepted_sequence: 1,
                                    reason: Box::new(reason)
                                }
                            ))
                        ),
                    }
                }
            }
        }
        // Private revision clock injection. Permission precedes deadline overflow;
        // suspension=true preserves due even at the maximum private tick.
        for control in 0..3 {
            let (mut job, _, _) = source(if control == 2 { 3 } else { 4 });
            let mut queue = job.queue.as_ref().clone();
            Arc::make_mut(&mut queue[0].revision).tick = LivingTickV2(u64::MAX);
            job.queue = Arc::new(queue).into();
            job.state.agreements.record_at_mut(0).unwrap().end_tick = LivingTickV2(u64::MAX);
            job.state.routes.find_mut(&id(31)).unwrap().kind = LivingRouteKindV2::Aid;
            if control == 1 {
                job.state.worlds.find_mut(&id(1)).unwrap().owner = None;
            }
            let expected = reference(&job);
            match expected {
                Ok(expected) => {
                    run(&mut job, SCHEDULES[3]).unwrap();
                    assert_eq!((job.state, job.events), expected);
                }
                Err(reason) => assert_eq!(
                    run(&mut job, SCHEDULES[3]),
                    Err(super::super::step_job::LivingLifecycleJobErrorV2::Fault(
                        LivingDeterministicFaultV2::Replay {
                            accepted_sequence: 1,
                            reason: Box::new(reason)
                        }
                    ))
                ),
            }
        }
    }
    #[test]
    fn cancellation_creator_prefix_unsupported_suffix_retains_effects_and_fault_order() {
        for fail in [false, true] {
            for schedule in SCHEDULES {
                let (mut job, _, _) = source(0);
                let suffix = LivingCreatorOperationV2::RemoveEntity {
                    target: t(if fail { 99 } else { 41 }),
                    cascade: super::super::command::LivingCascadeDispositionV2::RemoveReviewed {
                        dependents: Vec::new(),
                        derived: Vec::new(),
                    },
                };
                job.queue = super::creator_batch_cursor_tests::queue(vec![
                    operation(0),
                    suffix,
                    operation(1),
                ])
                .into();
                let expected = reference(&job);
                run(&mut job, schedule).unwrap();
                assert!(matches!(job.stage, QueuedCreatorStage::NeedsSynchronous));
                let prefix = job.effect_work.as_ref().unwrap().values.as_ptr();
                assert_eq!(job.effect_work.as_ref().unwrap().values.len(), 2);
                let result = job.take_synchronous_result();
                match expected {
                    Ok(expected) => assert_eq!(result.unwrap(), expected),
                    Err(reason) => {
                        assert_eq!(
                            result,
                            Err(super::super::step_job::LivingLifecycleJobErrorV2::Fault(
                                LivingDeterministicFaultV2::Replay {
                                    accepted_sequence: 1,
                                    reason: Box::new(reason)
                                }
                            ))
                        );
                        assert_eq!(job.effect_work.as_ref().unwrap().values.as_ptr(), prefix);
                        assert_eq!(job.events.payloads().len(), 1);
                    }
                }
            }
        }
    }
    #[test]
    fn cancellation_creator_private_receipt_precedence_and_recovery_before_effects() {
        for control in 0..4 {
            let (mut job, _, _) = source(0);
            job.events = LivingPendingEventsV2::new();
            let len = if control < 2 {
                65_536
            } else {
                65_537 - control
            };
            for _ in 0..len {
                job.events
                    .record(
                        LivingEventProvenanceV2::Autonomous {
                            phase: LivingPhaseV2::ResolveArrivals,
                        },
                        LivingEventKindV2::FleetArrived,
                    )
                    .unwrap();
            }
            if control == 0 {
                job.queue =
                    super::creator_batch_cursor_tests::queue(vec![operation(0), operation(0)])
                        .into();
            }
            let error = run(&mut job, SCHEDULES[3]).unwrap_err();
            if control == 0 {
                assert!(matches!(
                    error,
                    super::super::step_job::LivingLifecycleJobErrorV2::Fault(
                        LivingDeterministicFaultV2::Replay { .. }
                    )
                ));
                assert_eq!(job.events.payloads().len(), len);
            } else {
                assert!(matches!(
                    error,
                    super::super::step_job::LivingLifecycleJobErrorV2::Fault(
                        LivingDeterministicFaultV2::Receipt(_)
                    )
                ));
                let collector = job.append.as_ref().unwrap().retained_for_test().0.unwrap();
                assert_eq!(collector.payloads().len(), 65_536);
                assert_eq!(
                    job.effect_work.as_ref().unwrap().values.len(),
                    if control == 1 { 2 } else { 0 }
                );
            }
        }
        for schedule in SCHEDULES {
            let (mut job, _, _) = source(0);
            job.state.civilizations.find_mut(&id(5)).unwrap().status =
                LivingCivilizationStatusV2::Dormant;
            job.state.worlds.find_mut(&id(2)).unwrap().owner = None;
            job.queue = super::creator_batch_cursor_tests::queue(vec![
                operation(0),
                LivingCreatorOperationV2::CreateColony {
                    world: t(2),
                    owner: t(5),
                },
            ])
            .into();
            job.events
                .creator_recoveries
                .push(LivingCreatorRecoveryCandidateV2 {
                    revision: job.queue[0].revision.id,
                    actor: id(4),
                });
            let expected = reference(&job).unwrap();
            run(&mut job, schedule).unwrap();
            assert_eq!((job.state, job.events), expected);
        }
    }

    #[test]
    fn cancellation_creator_private_populated_suffix_owner_preserves_order() {
        for schedule in SCHEDULES {
            let (mut job, _, _) = source(6);
            run(&mut job, schedule).unwrap();
            assert_eq!(
                job.events
                    .payloads()
                    .iter()
                    .map(|e| e.kind)
                    .collect::<Vec<_>>(),
                vec![
                    LivingEventKindV2::FleetArrived,
                    LivingEventKindV2::CreatorIntervention,
                    LivingEventKindV2::RefundDiscarded,
                    LivingEventKindV2::ConstructionCancelled,
                    LivingEventKindV2::RefundDiscarded,
                    LivingEventKindV2::ConstructionCancelled
                ]
            );
            assert!(job.effect_work.is_none());
            assert_eq!(job.state.accepted_sequence, 1);
        }
    }

    #[test]
    fn cancellation_creator_private_candidate_fault_current_stale_and_accepted_tail() {
        for stale in [false, true] {
            for schedule in SCHEDULES {
                let mut authority = crate::living::builtin_first_expansion_v2().unwrap();
                let colony = authority.state.colonies.as_slice()[0].clone();
                let build = authority
                    .catalog
                    .industry("defense_battery")
                    .unwrap()
                    .clone();
                let envelope = |authority: &LivingGalaxyAuthorityV2, mode, operations| {
                    let cursor = authority.published_cursor();
                    LivingCommandEnvelopeV2 {
                        expected_committed_revision: cursor.committed_revision,
                        expected_tick: cursor.tick,
                        expected_pending_sequence: cursor.pending_sequence,
                        mode,
                        command: LivingCommandV2::CreatorBatch { operations },
                    }
                };
                authority
                    .apply_paused(envelope(
                        &authority,
                        LivingCommandModeV2::Paused,
                        vec![LivingCreatorOperationV2::CreateConstructionJob {
                            local: 0,
                            world: LivingCommandTargetV2::Existing { id: colony.world },
                            owner: LivingCommandTargetV2::Existing { id: colony.owner },
                            definition: build.id,
                            accepted_tick: LivingTickV2(0),
                            completion_tick: LivingTickV2(u64::from(build.build_ticks)),
                            paid_alloy: build.alloy_cost,
                        }],
                    ))
                    .unwrap();
                let target = authority.state.construction_jobs.as_slice()[0].id;
                authority
                    .submit(envelope(
                        &authority,
                        LivingCommandModeV2::Running,
                        vec![LivingCreatorOperationV2::CancelConstructionJob {
                            target: LivingCommandTargetV2::Existing { id: target },
                        }],
                    ))
                    .unwrap();
                let state = Arc::clone(&authority.state);
                let revisions = Arc::clone(&authority.revisions);
                let receipts = (
                    authority.last_receipt.clone(),
                    authority.last_decision.clone(),
                    authority.last_event_facts.clone(),
                );
                let mut job = Box::new(authority.begin_step().unwrap());
                for _ in 0..100_000 {
                    if job.creator.is_some() {
                        break;
                    }
                    assert_eq!(job.poll(1).unwrap(), LivingStepProgressV2::Pending);
                }
                let creator = job
                    .creator
                    .as_mut()
                    .expect("finite creator acquisition guard");
                let pointer = &**creator as *const _;
                // Private candidate-only mutation after genuine admission. Authority and
                // its accepted revision remain valid so the stale tail is actually admitted.
                creator
                    .state
                    .worlds
                    .find_mut(&colony.world)
                    .unwrap()
                    .inventory
                    .alloy = u32::MAX;
                let mut failed = false;
                for attempt in 0..100_000 {
                    let budget = schedule[attempt % schedule.len()];
                    match job.poll(budget) {
                        Err(LivingStepJobErrorV2::Fault(LivingDeterministicFaultV2::Replay {
                            reason,
                            ..
                        })) => {
                            assert_eq!(*reason, LivingCommandRejectionV2::Exhausted);
                            failed = true;
                            break;
                        }
                        Ok(LivingStepProgressV2::Pending) => assert!(job.last_poll_visits() > 0),
                        result => panic!("private cancellation fault {result:?}"),
                    }
                }
                assert!(failed, "finite private failure guard");
                assert!(std::ptr::eq(&**job.creator.as_ref().unwrap(), pointer));
                let LivingStepPreparationV2::Failed(token) =
                    job.take_synchronous_preparation(&authority).unwrap()
                else {
                    panic!("failed token")
                };
                if stale {
                    authority
                        .submit(envelope(
                            &authority,
                            LivingCommandModeV2::Running,
                            vec![LivingCreatorOperationV2::CreateSystem {
                                local: 0,
                                name: LivingNameV2::new("Cancellation failure tail").unwrap(),
                            }],
                        ))
                        .unwrap();
                }
                let queue = Arc::clone(&authority.queue);
                let high = authority.accepted_sequence_high_water;
                let cursor = authority.published_cursor();
                let result = authority.commit_step_failure(token);
                if stale {
                    assert_eq!(result.unwrap_err().reason(), &LivingStepJobErrorV2::Stale);
                    assert!(authority.fault.is_none());
                } else {
                    drop(result.unwrap());
                    assert!(matches!(
                        authority.fault,
                        Some(LivingDeterministicFaultV2::Replay { .. })
                    ));
                }
                assert!(Arc::ptr_eq(&authority.state, &state));
                assert!(Arc::ptr_eq(&authority.revisions, &revisions));
                assert!(Arc::ptr_eq(&authority.queue, &queue));
                assert_eq!(authority.published_cursor(), cursor);
                assert_eq!(authority.accepted_sequence_high_water, high);
                assert_eq!(
                    (
                        authority.last_receipt.clone(),
                        authority.last_decision.clone(),
                        authority.last_event_facts.clone()
                    ),
                    receipts
                );
                assert_eq!(authority.queue.len(), if stale { 2 } else { 1 });
                let mut retirement =
                    job.into_boxed_retirement(LivingPublicationRetirementPolicyV2::ReleaseHandle);
                let mut done = false;
                for attempt in 0..100_000 {
                    if retirement.poll(schedule[attempt % schedule.len()]).unwrap()
                        == LivingPublicationRetirementProgressV2::Ready
                    {
                        done = true;
                        break;
                    }
                }
                assert!(done, "finite failed parent retirement guard");
                if stale {
                    authority.step().unwrap();
                    assert!(authority.state.construction_jobs.as_slice().is_empty());
                    assert!(authority.queue.is_empty());
                }
            }
        }
    }

    #[test]
    fn cancellation_creator_private_bridge_original_local_resolver_and_metadata() {
        for declared in [false, true] {
            let (mut job, _, _) = source(0);
            let revision_id = job.queue[0].revision.id;
            let mut operations = Vec::new();
            if declared {
                operations.push(LivingCreatorOperationV2::CreateSystem {
                    local: 7,
                    name: LivingNameV2::new("First declaration").unwrap(),
                });
                operations.push(LivingCreatorOperationV2::CreateCivilization {
                    local: 7,
                    name: LivingNameV2::new("Private duplicate declaration").unwrap(),
                    policy: super::super::model::LivingPolicyV2(
                        LivingSlugV2::new("neutral").unwrap(),
                    ),
                    status: LivingCivilizationStatusV2::Dormant,
                });
                let first_id = super::super::ids::creator_entity_id_v2(
                    revision_id,
                    super::super::ids::LivingEntityKindV2::System.ordinal(),
                    7,
                );
                job.state.systems = LivingSortedVecV2::new(vec![LivingSystemV2 {
                    id: first_id,
                    name: LivingNameV2::new("Original created ID").unwrap(),
                }])
                .unwrap();
            }
            job.operation_index = operations.len();
            operations.push(LivingCreatorOperationV2::RemoveEntity {
                target: LivingCommandTargetV2::Local { local: 7 },
                cascade: super::super::command::LivingCascadeDispositionV2::RejectIfReferenced,
            });
            let queue = Arc::make_mut(&mut job.queue);
            Arc::make_mut(&mut queue[0].revision).command =
                LivingCommandV2::CreatorBatch { operations };
            Arc::make_mut(&mut queue[0].accepted).accepted_sequence = 77;
            job.stage = QueuedCreatorStage::NeedsSynchronous;
            let mut expected_state = job.state.clone();
            let expected = apply_revision_range(
                &mut expected_state,
                &job.queue[0].revision,
                &job.catalog,
                job.operation_index,
            );
            match expected {
                Ok(effects) => {
                    assert!(effects.is_empty());
                    let actual = job.take_synchronous_result().unwrap();
                    assert_eq!(actual.0, expected_state);
                }
                Err(reason) => {
                    assert_eq!(
                        job.take_synchronous_result(),
                        Err(super::super::step_job::LivingLifecycleJobErrorV2::Fault(
                            LivingDeterministicFaultV2::Replay {
                                accepted_sequence: 77,
                                reason: Box::new(reason)
                            }
                        ))
                    );
                    let effects = job
                        .effect_work
                        .as_ref()
                        .expect("failed empty suffix owner retained");
                    assert!(effects.suffix.is_empty());
                    assert!(effects.values.is_empty());
                    assert_eq!(job.events.payloads().len(), 1);
                }
            }
        }
    }
}

#[cfg(test)]
mod geometry_creator_tests {
    use super::*;
    use crate::living::model::*;
    use crate::living::{LivingNameV2, LivingSlugV2};
    pub(super) const SCHEDULES: &[&[usize]] = &[&[1], &[64], &[1024], &[1, 7, 64, 2, 1024]];
    fn id(n: u8) -> LivingEntityIdV2 {
        LivingEntityIdV2([n; 16])
    }
    fn t(n: u8) -> LivingCommandTargetV2 {
        LivingCommandTargetV2::Existing { id: id(n) }
    }
    pub(super) fn operation(kind: usize) -> LivingCreatorOperationV2 {
        if kind < 2 {
            LivingCreatorOperationV2::EditWorld {
                target: t(1),
                system: t(if kind == 0 { 11 } else { 10 }),
                archetype: LivingSlugV2::new("barren").unwrap(),
                name: LivingNameV2::new("N".repeat(64)).unwrap(),
            }
        } else {
            LivingCreatorOperationV2::EditLane {
                target: t(8),
                system_a: t(11),
                system_b: t(10),
                distance_units: if kind == 2 { 20 } else { 10 },
            }
        }
    }
    // Private populated ownership fixture. Deliberately absent systems/actors are
    // not admitted genesis evidence. Empty allocated paths remain original owners.
    pub(super) fn source(
        kind: usize,
    ) -> (
        Box<LivingQueuedCreatorJobV2>,
        Arc<Vec<QueuedCommandV2>>,
        Arc<ValidatedLivingCatalogPackV2>,
    ) {
        let catalog = Arc::new(crate::living::living_core_pack_v2().unwrap());
        let state = LivingGalaxyStateV2 {
            worlds: LivingSortedVecV2::new(
                (1..=3)
                    .map(|n| LivingWorldV2 {
                        id: id(n),
                        system: id(10),
                        archetype: LivingSlugV2::new("barren").unwrap(),
                        name: LivingNameV2::new("World").unwrap(),
                        owner: None,
                        inventory: Default::default(),
                    })
                    .collect(),
            )
            .unwrap(),
            lanes: LivingSortedVecV2::new(
                (8..=9)
                    .map(|n| LivingLaneV2 {
                        id: id(n),
                        system_a: id(10),
                        system_b: id(11),
                        distance_units: 10,
                    })
                    .collect(),
            )
            .unwrap(),
            shipments: LivingSortedVecV2::new(
                (20..=22)
                    .map(|n| LivingShipmentV2 {
                        id: id(n),
                        dispatch_owner: id(4),
                        intended_receiver: id(4),
                        source_world: id(2),
                        destination_world: id(3),
                        resource: LivingResourceV2::Ore,
                        units: 1,
                        departure_tick: LivingTickV2(0),
                        arrival_tick: LivingTickV2(9),
                        disposition: LivingShipmentDispositionV2::Outbound,
                        dispatch_kind: Some(LivingRouteKindV2::Internal),
                        returning_to_source: false,
                    })
                    .collect(),
            )
            .unwrap(),
            fleets: LivingSortedVecV2::new(
                (30..=32)
                    .map(|n| LivingFleetV2 {
                        id: id(n),
                        owner: id(4),
                        location: LivingFleetLocationV2::Docked { world: id(2) },
                        order: (n != 30).then(|| LivingFleetOrderV2 {
                            kind: LivingFleetOrderKindV2::Move,
                            target_world: Some(id(1)),
                            not_before_tick: LivingTickV2(99),
                            lane_path: if n == 31 {
                                Vec::with_capacity(7)
                            } else {
                                vec![id(9); 3]
                            },
                        }),
                        hulls: LivingSortedVecV2::new(
                            (0..2)
                                .map(|i| LivingHullV2 {
                                    id: id(40 + (n - 30) * 2 + i),
                                    definition: LivingSlugV2::new("scout").unwrap(),
                                    hit_points: 7,
                                    return_credit: true,
                                    next_repair: Some(LivingTickV2(40)),
                                })
                                .collect(),
                        )
                        .unwrap(),
                    })
                    .collect(),
            )
            .unwrap(),
            ..Default::default()
        };
        let queue = super::creator_batch_cursor_tests::queue(vec![operation(kind)]);
        let mut job = Box::new(LivingQueuedCreatorJobV2::begin(
            state,
            Arc::clone(&queue),
            Arc::clone(&catalog),
        ));
        job.events
            .record(
                LivingEventProvenanceV2::Autonomous {
                    phase: LivingPhaseV2::ResolveArrivals,
                },
                LivingEventKindV2::FleetArrived,
            )
            .unwrap();
        job.events = job.events.clone();
        (job, queue, catalog)
    }
    pub(super) fn run(
        job: &mut LivingQueuedCreatorJobV2,
        schedule: &[usize],
    ) -> Result<usize, super::super::step_job::LivingLifecycleJobErrorV2> {
        super::cancellation_creator_tests::run(job, schedule)
    }
    #[test]
    fn geometry_creator_private_guard_matrix_matches_independent_additions() {
        // Mutable fixture cases are explicitly private replay controls, not admission.
        for kind in 0..4 {
            for case in 0..18 {
                for schedule in SCHEDULES {
                    let (mut job, queue, catalog) = source(kind);
                    match case {
                        0 => {}
                        1..=3 => {
                            job.state
                                .shipments
                                .record_at_mut(case - 1)
                                .unwrap()
                                .source_world = id(1)
                        }
                        4..=6 => {
                            job.state
                                .shipments
                                .record_at_mut(case - 4)
                                .unwrap()
                                .destination_world = id(1)
                        }
                        7 => {
                            job.state.fleets.record_at_mut(0).unwrap().location =
                                LivingFleetLocationV2::Travelling {
                                    origin_world: id(1),
                                    destination_world: id(3),
                                    departure_tick: LivingTickV2(0),
                                    arrival_tick: LivingTickV2(9),
                                }
                        }
                        8 => {
                            job.state.fleets.record_at_mut(0).unwrap().location =
                                LivingFleetLocationV2::Travelling {
                                    origin_world: id(3),
                                    destination_world: id(1),
                                    departure_tick: LivingTickV2(0),
                                    arrival_tick: LivingTickV2(9),
                                }
                        }
                        9..=11 => {
                            job.state.fleets.record_at_mut(case - 9).unwrap().location =
                                LivingFleetLocationV2::Docked { world: id(1) }
                        }
                        12..=14 => {
                            job.state
                                .fleets
                                .record_at_mut(2)
                                .unwrap()
                                .order
                                .as_mut()
                                .unwrap()
                                .lane_path[case - 12] = id(8)
                        }
                        15 => {
                            job.state.shipments.record_at_mut(0).unwrap().source_world = id(1);
                            job.state.fleets.record_at_mut(1).unwrap().location =
                                LivingFleetLocationV2::Docked { world: id(1) };
                        }
                        16 => {
                            job.state.fleets.record_at_mut(0).unwrap().location =
                                LivingFleetLocationV2::Travelling {
                                    origin_world: id(2),
                                    destination_world: id(3),
                                    departure_tick: LivingTickV2(0),
                                    arrival_tick: LivingTickV2(9),
                                }
                        }
                        17 => {
                            job.state
                                .fleets
                                .record_at_mut(2)
                                .unwrap()
                                .order
                                .as_mut()
                                .unwrap()
                                .lane_path = vec![id(8); 3]
                        }
                        _ => unreachable!(),
                    }
                    let before = job.state.clone();
                    let mut expected = before.clone();
                    let reference =
                        apply_revision_range(&mut expected, &queue[0].revision, &catalog, 0);
                    let result = run(&mut job, schedule);
                    match reference {
                        Ok(effects) => {
                            assert!(effects.is_empty());
                            result.unwrap();
                            assert_eq!(job.state, expected, "kind {kind} case {case}");
                            assert_eq!(job.events.len(), 2);
                            assert_eq!(job.state.shipments, before.shipments);
                            assert_eq!(job.state.fleets, before.fleets);
                        }
                        Err(reason) => {
                            assert_eq!(reason, LivingCommandRejectionV2::ReferencedTarget);
                            assert_eq!(
                                result,
                                Err(super::super::step_job::LivingLifecycleJobErrorV2::Fault(
                                    LivingDeterministicFaultV2::Replay {
                                        accepted_sequence: 1,
                                        reason: Box::new(reason)
                                    }
                                ))
                            );
                            assert_eq!(job.state, before);
                            assert_eq!(job.events.len(), 1);
                            assert_eq!(job.operation_index, 0);
                            assert!(job.last_poll_visits > 0);
                        }
                    }
                }
            }
        }
    }
    #[test]
    fn geometry_creator_private_missing_range_and_late_fault_precedence() {
        for case in 0..5 {
            for schedule in SCHEDULES {
                let (original, _old_queue, catalog) = source(0);
                let mut operation = operation(if case == 0 { 0 } else { 2 });
                match &mut operation {
                    LivingCreatorOperationV2::EditWorld { target, system, .. } => {
                        *target = t(99);
                        *system = t(99);
                    }
                    LivingCreatorOperationV2::EditLane {
                        target,
                        system_a,
                        distance_units,
                        ..
                    } => {
                        *target = t(99);
                        if case == 1 {
                            *system_a = t(10);
                        }
                        if case == 2 {
                            *distance_units = 0;
                        }
                    }
                    _ => unreachable!(),
                }
                let prefix = operation_for_prefix();
                let mut queue = super::creator_batch_cursor_tests::queue(vec![prefix, operation]);
                if case == 0 {
                    // Explicit private post-seal mutation: public admission rejects this Local.
                    let revision = Arc::make_mut(&mut Arc::make_mut(&mut queue)[0].revision);
                    let LivingCommandV2::CreatorBatch { operations } = &mut revision.command;
                    let LivingCreatorOperationV2::EditWorld { system, .. } = &mut operations[1]
                    else {
                        unreachable!()
                    };
                    *system = LivingCommandTargetV2::Local { local: 99 };
                }
                let mut job = LivingQueuedCreatorJobV2::begin(
                    original.state.clone(),
                    Arc::clone(&queue),
                    Arc::clone(&catalog),
                );
                let expected = if case == 1 || case == 2 {
                    LivingCommandRejectionV2::Projection(LivingValidationErrorV2::Range {
                        field: "lane endpoints/distance",
                    })
                } else {
                    LivingCommandRejectionV2::MissingTarget {
                        operation: 1,
                        reference: if case == 0 { "world" } else { "lane" },
                    }
                };
                assert_eq!(
                    run(&mut job, schedule),
                    Err(super::super::step_job::LivingLifecycleJobErrorV2::Fault(
                        LivingDeterministicFaultV2::Replay {
                            accepted_sequence: 1,
                            reason: Box::new(expected)
                        }
                    ))
                );
                assert_eq!(job.operation_index, 1);
                assert_eq!(
                    job.state.worlds.as_slice()[0].name,
                    LivingNameV2::new("N".repeat(64)).unwrap()
                );
                assert_eq!(job.state.accepted_sequence, 0);
                assert_eq!(job.events, LivingPendingEventsV2::new());
            }
        }
    }
    fn operation_for_prefix() -> LivingCreatorOperationV2 {
        operation(1)
    }

    #[test]
    fn geometry_creator_source_derived_forward_and_once_only_extraction() {
        for kind in 0..4 {
            for schedule in SCHEDULES {
                let (mut job, queue, catalog) = source(kind);
                let pointer = &*job as *const _;
                let expected_visits = [79, 73, 79, 73][kind];
                assert!(job.take_ready_result().is_err());
                assert_eq!(run(&mut job, schedule).unwrap(), expected_visits);
                assert!(std::ptr::eq(&*job, pointer));
                assert!(Arc::ptr_eq(&job.queue, &queue));
                assert!(Arc::ptr_eq(&job.catalog, &catalog));
                let mut expected = source(kind).0.state.clone();
                apply_revision_range(&mut expected, &queue[0].revision, &catalog, 0).unwrap();
                let (actual, events) = job.take_ready_result().unwrap();
                assert_eq!(actual, expected);
                assert_eq!(events.len(), 2);
                assert!(job.take_ready_result().is_err());
            }
        }
    }
    #[test]
    fn geometry_creator_private_candidate_fault_current_and_genuine_stale_tail() {
        for stale in [false, true] {
            for schedule in SCHEDULES {
                let mut authority = crate::living::builtin_first_expansion_v2().unwrap();
                let world = authority.state.worlds.as_slice()[1].clone();
                let system = authority
                    .state
                    .systems
                    .as_slice()
                    .iter()
                    .find(|r| r.id != world.system)
                    .unwrap()
                    .id;
                let envelope = |a: &LivingGalaxyAuthorityV2, operations| {
                    let c = a.published_cursor();
                    LivingCommandEnvelopeV2 {
                        expected_committed_revision: c.committed_revision,
                        expected_tick: c.tick,
                        expected_pending_sequence: c.pending_sequence,
                        mode: LivingCommandModeV2::Running,
                        command: LivingCommandV2::CreatorBatch { operations },
                    }
                };
                authority
                    .submit(envelope(
                        &authority,
                        vec![LivingCreatorOperationV2::EditWorld {
                            target: LivingCommandTargetV2::Existing { id: world.id },
                            system: LivingCommandTargetV2::Existing { id: system },
                            archetype: world.archetype.clone(),
                            name: world.name.clone(),
                        }],
                    ))
                    .unwrap();
                let state = Arc::clone(&authority.state);
                let revisions = Arc::clone(&authority.revisions);
                let receipts = (
                    authority.last_receipt.clone(),
                    authority.last_decision.clone(),
                    authority.last_event_facts.clone(),
                );
                let mut job = Box::new(authority.begin_step().unwrap());
                let mut reached = false;
                for _ in 0..100000 {
                    if job.creator.as_ref().is_some_and(|c| {
                        matches!(c.stage, QueuedCreatorStage::WorldShipments { .. })
                    }) {
                        reached = true;
                        break;
                    }
                    assert_eq!(job.poll(1).unwrap(), LivingStepProgressV2::Pending);
                }
                assert!(reached);
                // Private candidate-only corruption after genuine unchanged admission.
                // The public authority remains valid for a genuinely admitted stale tail.
                let creator = job.creator.as_mut().unwrap();
                let pointer = &**creator as *const _;
                let fleet = creator.state.fleets.record_at_mut(0).unwrap();
                fleet.location = LivingFleetLocationV2::Docked { world: world.id };
                fleet.order = Some(LivingFleetOrderV2 {
                    kind: LivingFleetOrderKindV2::Move,
                    target_world: Some(world.id),
                    not_before_tick: LivingTickV2(99),
                    lane_path: Vec::new(),
                });
                let mut failed = false;
                for attempt in 0..10000 {
                    match job.poll(schedule[attempt % schedule.len()]) {
                        Err(LivingStepJobErrorV2::Fault(LivingDeterministicFaultV2::Replay {
                            reason,
                            ..
                        })) => {
                            assert_eq!(*reason, LivingCommandRejectionV2::ReferencedTarget);
                            failed = true;
                            break;
                        }
                        Ok(LivingStepProgressV2::Pending) => {}
                        other => panic!("private geometry fault {other:?}"),
                    }
                }
                assert!(failed);
                assert!(std::ptr::eq(&**job.creator.as_ref().unwrap(), pointer));
                let LivingStepPreparationV2::Failed(token) =
                    job.take_synchronous_preparation(&authority).unwrap()
                else {
                    panic!("private failed token")
                };
                assert!(job.take_synchronous_preparation(&authority).is_err());
                if stale {
                    authority
                        .submit(envelope(
                            &authority,
                            vec![LivingCreatorOperationV2::CreateSystem {
                                local: 0,
                                name: LivingNameV2::new("Accepted geometry fault tail").unwrap(),
                            }],
                        ))
                        .unwrap();
                }
                let queue = Arc::clone(&authority.queue);
                let cursor = authority.published_cursor();
                let high = authority.accepted_sequence_high_water;
                let result = authority.commit_step_failure(token);
                if stale {
                    assert_eq!(result.unwrap_err().reason(), &LivingStepJobErrorV2::Stale);
                    assert!(authority.fault.is_none());
                } else {
                    drop(result.unwrap());
                    assert!(authority.fault.is_some());
                }
                assert!(Arc::ptr_eq(&authority.state, &state));
                assert!(Arc::ptr_eq(&authority.revisions, &revisions));
                assert!(Arc::ptr_eq(&authority.queue, &queue));
                assert_eq!(authority.published_cursor(), cursor);
                assert_eq!(authority.accepted_sequence_high_water, high);
                assert_eq!(
                    (
                        authority.last_receipt.clone(),
                        authority.last_decision.clone(),
                        authority.last_event_facts.clone()
                    ),
                    receipts
                );
                let mut retired =
                    job.into_boxed_retirement(LivingPublicationRetirementPolicyV2::ReleaseHandle);
                let mut done = false;
                for attempt in 0..100000 {
                    if retired.poll(schedule[attempt % schedule.len()]).unwrap()
                        == LivingPublicationRetirementProgressV2::Ready
                    {
                        done = true;
                        break;
                    }
                }
                assert!(done);
                if stale {
                    authority.step().unwrap();
                    assert!(authority.queue.is_empty());
                }
            }
        }
    }

    #[test]
    fn geometry_creator_private_late_command_precedes_marker_limit_and_retains_effects() {
        for failed in [false, true] {
            let (mut job, _original_queue, _catalog) = source(1);
            let mut operations = vec![operation(1)];
            if failed {
                operations.push(LivingCreatorOperationV2::EditLane {
                    target: t(99),
                    system_a: t(10),
                    system_b: t(11),
                    distance_units: 10,
                });
            }
            job.queue = super::creator_batch_cursor_tests::queue(operations).into();
            for _ in 1..=u16::MAX {
                job.events
                    .record(
                        LivingEventProvenanceV2::Autonomous {
                            phase: LivingPhaseV2::ResolveArrivals,
                        },
                        LivingEventKindV2::FleetArrived,
                    )
                    .unwrap();
            }
            let result = run(&mut job, SCHEDULES[3]);
            if failed {
                assert_eq!(
                    result,
                    Err(super::super::step_job::LivingLifecycleJobErrorV2::Fault(
                        LivingDeterministicFaultV2::Replay {
                            accepted_sequence: 1,
                            reason: Box::new(LivingCommandRejectionV2::MissingTarget {
                                operation: 1,
                                reference: "lane"
                            })
                        }
                    ))
                );
                assert!(job.append.is_none());
            } else {
                assert!(matches!(
                    result,
                    Err(super::super::step_job::LivingLifecycleJobErrorV2::Fault(
                        LivingDeterministicFaultV2::Receipt(_)
                    ))
                ));
            }
            assert_eq!(job.state.accepted_sequence, 0);
        }
        let (mut job, _q, catalog) = super::cancellation_creator_tests::source(0);
        let cancellation = job.queue[0].revision.command.operations()[0].clone();
        let world = &job.state.worlds.as_slice()[0];
        let edit = LivingCreatorOperationV2::EditWorld {
            target: LivingCommandTargetV2::Existing { id: world.id },
            system: LivingCommandTargetV2::Existing { id: world.system },
            archetype: world.archetype.clone(),
            name: LivingNameV2::new("After refund").unwrap(),
        };
        let q = super::creator_batch_cursor_tests::queue(vec![cancellation, edit]);
        job.queue = Arc::clone(&q).into();
        let mut expected = job.state.clone();
        let effects = apply_revision_range(&mut expected, &q[0].revision, &catalog, 0).unwrap();
        assert_eq!(
            effects,
            vec![
                LivingEventKindV2::RefundDiscarded,
                LivingEventKindV2::ConstructionCancelled
            ]
        );
        run(&mut job, SCHEDULES[3]).unwrap();
        assert_eq!(job.state, expected);
        let kinds: Vec<_> = job.events.payloads().iter().map(|p| p.kind).collect();
        assert_eq!(
            kinds,
            vec![
                LivingEventKindV2::FleetArrived,
                LivingEventKindV2::CreatorIntervention,
                LivingEventKindV2::RefundDiscarded,
                LivingEventKindV2::ConstructionCancelled
            ]
        );
    }
}

#[cfg(test)]
mod fleet_hull_creator_tests {
    pub(super) use super::geometry_creator_tests::SCHEDULES;
    use super::*;
    use crate::living::model::*;
    use crate::living::{LivingNameV2, LivingSlugV2};
    fn id(n: u8) -> LivingEntityIdV2 {
        LivingEntityIdV2([n; 16])
    }
    fn t(n: u8) -> LivingCommandTargetV2 {
        LivingCommandTargetV2::Existing { id: id(n) }
    }
    pub(super) fn operation(kind: usize) -> LivingCreatorOperationV2 {
        if kind < 3 {
            LivingCreatorOperationV2::EditFleet {
                target: t(30 + kind as u8),
                owner: t(5),
                world: t(3),
            }
        } else {
            LivingCreatorOperationV2::EditHull {
                target: t(40 + (kind as u8 - 3) * 2),
                definition: LivingSlugV2::new(["colony_ark", "escort", "scout"][kind - 3]).unwrap(),
                hit_points: 1,
                return_credit: false,
            }
        }
    }
    // Private original-owner fixture, not validated/admitted genesis. Related
    // reservations are injected only in explicit rejection cases below.
    pub(super) fn source(
        kind: usize,
    ) -> (
        Box<LivingQueuedCreatorJobV2>,
        Arc<Vec<QueuedCommandV2>>,
        Arc<ValidatedLivingCatalogPackV2>,
    ) {
        let (mut job, _, catalog) = super::geometry_creator_tests::source(0);
        job.state
            .fleets
            .record_at_mut(2)
            .unwrap()
            .order
            .as_mut()
            .unwrap()
            .lane_path = vec![id(9); 33];
        job.state.hull_jobs = LivingSortedVecV2::new(
            (60..63)
                .map(|n| LivingHullJobV2 {
                    id: id(n),
                    world: id(2),
                    owner: id(4),
                    definition: LivingSlugV2::new("scout").unwrap(),
                    accepted_tick: LivingTickV2(0),
                    completion_tick: LivingTickV2(50),
                    paid_alloy: 10,
                    target_fleet: None,
                })
                .collect(),
        )
        .unwrap();
        let queue = super::creator_batch_cursor_tests::queue(vec![operation(kind)]);
        job.queue = Arc::clone(&queue).into();
        (job, queue, catalog)
    }
    fn run(
        job: &mut LivingQueuedCreatorJobV2,
        schedule: &[usize],
    ) -> Result<usize, super::super::step_job::LivingLifecycleJobErrorV2> {
        super::geometry_creator_tests::run(job, schedule)
    }
    #[test]
    fn fleet_hull_creator_private_guard_matrix_and_exact_original_hulls() {
        for kind in 0..6 {
            for case in 0..13 {
                for schedule in SCHEDULES {
                    let (mut job, _, catalog) = source(kind);
                    let parent = kind % 3;
                    let mut op = operation(kind);
                    match case {
                        0 => {}
                        1..=3 => {
                            job.state
                                .hull_jobs
                                .record_at_mut(case - 1)
                                .unwrap()
                                .target_fleet = Some(id(30 + parent as u8))
                        }
                        4 => {
                            job.state.fleets.record_at_mut(parent).unwrap().location =
                                LivingFleetLocationV2::Travelling {
                                    origin_world: id(2),
                                    destination_world: id(3),
                                    departure_tick: LivingTickV2(0),
                                    arrival_tick: LivingTickV2(20),
                                }
                        }
                        5 => {
                            match &mut op {
                                LivingCreatorOperationV2::EditFleet { target, .. }
                                | LivingCreatorOperationV2::EditHull { target, .. } => {
                                    *target = t(99)
                                }
                                _ => unreachable!(),
                            }
                            job.state.hull_jobs.record_at_mut(0).unwrap().target_fleet =
                                Some(id(99));
                        }
                        6..=9 => {
                            if let LivingCreatorOperationV2::EditHull {
                                definition,
                                hit_points,
                                ..
                            } = &mut op
                            {
                                match case {
                                    6 => {
                                        *definition = LivingSlugV2::new("missing").unwrap();
                                        *hit_points = 0;
                                    }
                                    7 => *hit_points = 0,
                                    8 => *hit_points = catalog.hulls()[parent].hit_points,
                                    9 => *hit_points = catalog.hulls()[parent].hit_points + 1,
                                    _ => unreachable!(),
                                }
                            }
                        }
                        10 => {
                            if let LivingCreatorOperationV2::EditFleet { owner, world, .. } =
                                &mut op
                            {
                                *owner = t(98);
                                *world = t(99);
                            }
                        }
                        11 => {
                            job.state.hull_jobs.record_at_mut(0).unwrap().target_fleet =
                                Some(id(30 + parent as u8));
                            job.state.hull_jobs.record_at_mut(2).unwrap().target_fleet =
                                Some(id(30 + parent as u8));
                        }
                        12 => {
                            if let LivingCreatorOperationV2::EditHull { target, .. } = &mut op {
                                *target = t(99);
                                // Missing hull must remain after valid definition/HP checks.
                            }
                        }
                        _ => unreachable!(),
                    }
                    let queue = super::creator_batch_cursor_tests::queue(vec![op]);
                    job.queue = Arc::clone(&queue).into();
                    let before = job.state.clone();
                    let pointers: Vec<_> = job
                        .state
                        .fleets
                        .as_slice()
                        .iter()
                        .map(|f| f.hulls.as_slice().as_ptr())
                        .collect();
                    let mut expected = before.clone();
                    let reference =
                        apply_revision_range(&mut expected, &queue[0].revision, &catalog, 0);
                    let actual = run(&mut job, schedule);
                    match reference {
                        Ok(effects) => {
                            assert!(effects.is_empty());
                            actual.unwrap();
                            assert_eq!(job.state, expected);
                            for (i, f) in job.state.fleets.as_slice().iter().enumerate() {
                                assert_eq!(f.hulls.as_slice().as_ptr(), pointers[i]);
                                for (h, old) in f
                                    .hulls
                                    .as_slice()
                                    .iter()
                                    .zip(before.fleets.as_slice()[i].hulls.as_slice())
                                {
                                    assert_eq!(h.id, old.id);
                                    assert_eq!(h.next_repair, old.next_repair);
                                    if kind < 3 || i != parent || h.id != id(40 + parent as u8 * 2)
                                    {
                                        assert_eq!(h, old);
                                    }
                                }
                            }
                            assert!(job.retirement.is_empty());
                            let output = job.take_ready_result().unwrap();
                            assert_eq!(output.0, expected);
                            assert!(job.take_ready_result().is_err());
                        }
                        Err(reason) => {
                            assert_eq!(
                                actual,
                                Err(super::super::step_job::LivingLifecycleJobErrorV2::Fault(
                                    LivingDeterministicFaultV2::Replay {
                                        accepted_sequence: 1,
                                        reason: Box::new(reason)
                                    }
                                ))
                            );
                            assert_eq!(job.state, before);
                            assert_eq!(job.operation_index, 0);
                            assert_eq!(job.events.len(), 1);
                            assert!(job.last_poll_visits > 0);
                        }
                    }
                }
            }
        }
    }
    #[test]
    fn fleet_hull_creator_private_relookup_and_first_parent_precedence() {
        for missing_parent in [false, true] {
            let (mut job, _, _) = source(5);
            // A duplicate across otherwise sorted parents is private malformed input.
            let duplicate = job.state.fleets.as_slice()[2].hulls.as_slice()[0].clone();
            job.state.fleets.record_at_mut(0).unwrap().hulls =
                LivingSortedVecV2::new(vec![duplicate]).unwrap();
            for _ in 0..1000 {
                if matches!(job.stage, QueuedCreatorStage::FleetEditReservations { .. }) {
                    break;
                }
                job.poll(1).unwrap();
            }
            assert!(matches!(
                job.stage,
                QueuedCreatorStage::FleetEditReservations { fleet: 0, .. }
            ));
            if missing_parent {
                job.state.fleets.record_at_mut(0).unwrap().id = id(29);
            } else {
                job.state.fleets.record_at_mut(0).unwrap().hulls = LivingSortedVecV2::default();
            }
            let before = job.state.clone();
            assert_eq!(
                run(&mut job, SCHEDULES[3]),
                Err(super::super::step_job::LivingLifecycleJobErrorV2::Fault(
                    LivingDeterministicFaultV2::Replay {
                        accepted_sequence: 1,
                        reason: Box::new(LivingCommandRejectionV2::MissingTarget {
                            operation: 0,
                            reference: if missing_parent { "fleet" } else { "hull" }
                        }),
                    }
                ))
            );
            assert_eq!(job.state, before);
        }
    }
    #[test]
    fn fleet_hull_creator_private_late_error_precedes_marker_and_retains_prefix() {
        for failed in [false, true] {
            let (mut job, _, _) = source(2);
            let mut ops = vec![operation(2)];
            if failed {
                ops.push(LivingCreatorOperationV2::EditHull {
                    target: t(99),
                    definition: LivingSlugV2::new("scout").unwrap(),
                    hit_points: 1,
                    return_credit: false,
                });
            }
            job.queue = super::creator_batch_cursor_tests::queue(ops).into();
            for _ in 1..=u16::MAX {
                job.events
                    .record(
                        LivingEventProvenanceV2::Autonomous {
                            phase: LivingPhaseV2::ResolveArrivals,
                        },
                        LivingEventKindV2::FleetArrived,
                    )
                    .unwrap();
            }
            let result = run(&mut job, SCHEDULES[3]);
            if failed {
                assert_eq!(
                    result,
                    Err(super::super::step_job::LivingLifecycleJobErrorV2::Fault(
                        LivingDeterministicFaultV2::Replay {
                            accepted_sequence: 1,
                            reason: Box::new(LivingCommandRejectionV2::MissingTarget {
                                operation: 1,
                                reference: "hull"
                            })
                        }
                    ))
                );
                assert!(job.append.is_none());
            } else {
                assert!(matches!(
                    result,
                    Err(super::super::step_job::LivingLifecycleJobErrorV2::Fault(
                        LivingDeterministicFaultV2::Receipt(_)
                    ))
                ));
            }
            assert_eq!(job.state.fleets.as_slice()[2].owner, id(5));
            assert!(job.state.fleets.as_slice()[2].order.is_none());
            assert_eq!(job.state.accepted_sequence, 0);
        }
    }
    #[test]
    fn fleet_hull_creator_private_candidate_fault_current_and_genuine_stale_tail() {
        for stale in [false, true] {
            for schedule in SCHEDULES {
                let mut authority = crate::living::builtin_first_expansion_v2().unwrap();
                let fleet = authority
                    .state
                    .fleets
                    .as_slice()
                    .iter()
                    .find(|f| {
                        matches!(f.location, LivingFleetLocationV2::Docked { .. })
                            && !authority
                                .state
                                .hull_jobs
                                .as_slice()
                                .iter()
                                .any(|j| j.target_fleet == Some(f.id))
                    })
                    .unwrap()
                    .clone();
                let LivingFleetLocationV2::Docked { world } = fleet.location else {
                    unreachable!()
                };
                let envelope = |a: &LivingGalaxyAuthorityV2, operations| {
                    let c = a.published_cursor();
                    LivingCommandEnvelopeV2 {
                        expected_committed_revision: c.committed_revision,
                        expected_tick: c.tick,
                        expected_pending_sequence: c.pending_sequence,
                        mode: LivingCommandModeV2::Running,
                        command: LivingCommandV2::CreatorBatch { operations },
                    }
                };
                authority
                    .submit(envelope(
                        &authority,
                        vec![LivingCreatorOperationV2::EditFleet {
                            target: LivingCommandTargetV2::Existing { id: fleet.id },
                            owner: LivingCommandTargetV2::Existing { id: fleet.owner },
                            world: LivingCommandTargetV2::Existing { id: world },
                        }],
                    ))
                    .unwrap();
                let state = Arc::clone(&authority.state);
                let revisions = Arc::clone(&authority.revisions);
                let receipts = (
                    authority.last_receipt.clone(),
                    authority.last_decision.clone(),
                    authority.last_event_facts.clone(),
                );
                let mut job = Box::new(authority.begin_step().unwrap());
                let mut reached = false;
                for _ in 0..100000 {
                    if job.creator.as_ref().is_some_and(|c| {
                        matches!(c.stage, QueuedCreatorStage::Declarations) && c.index == 1
                    }) {
                        reached = true;
                        break;
                    }
                    assert_eq!(job.poll(1).unwrap(), LivingStepProgressV2::Pending);
                }
                assert!(reached);
                // Private candidate-only corruption after genuine unchanged admission.
                // The public authority remains valid for a genuinely admitted stale tail.
                let creator = job.creator.as_mut().unwrap();
                let pointer = &**creator as *const _;
                creator.state.fleets.find_mut(&fleet.id).unwrap().location =
                    LivingFleetLocationV2::Travelling {
                        origin_world: world,
                        destination_world: world,
                        departure_tick: LivingTickV2(0),
                        arrival_tick: LivingTickV2(99),
                    };
                let mut failed = false;
                for attempt in 0..10000 {
                    match job.poll(schedule[attempt % schedule.len()]) {
                        Err(LivingStepJobErrorV2::Fault(LivingDeterministicFaultV2::Replay {
                            reason,
                            ..
                        })) => {
                            assert_eq!(*reason, LivingCommandRejectionV2::ReferencedTarget);
                            failed = true;
                            break;
                        }
                        Ok(LivingStepProgressV2::Pending) => {}
                        other => panic!("private fleet hull fault {other:?}"),
                    }
                }
                assert!(failed);
                assert!(std::ptr::eq(&**job.creator.as_ref().unwrap(), pointer));
                let LivingStepPreparationV2::Failed(token) =
                    job.take_synchronous_preparation(&authority).unwrap()
                else {
                    panic!("private failed token")
                };
                assert!(job.take_synchronous_preparation(&authority).is_err());
                if stale {
                    authority
                        .submit(envelope(
                            &authority,
                            vec![LivingCreatorOperationV2::CreateSystem {
                                local: 0,
                                name: LivingNameV2::new("Accepted fleet hull fault tail").unwrap(),
                            }],
                        ))
                        .unwrap();
                }
                let queue = Arc::clone(&authority.queue);
                let cursor = authority.published_cursor();
                let high = authority.accepted_sequence_high_water;
                let result = authority.commit_step_failure(token);
                if stale {
                    assert_eq!(result.unwrap_err().reason(), &LivingStepJobErrorV2::Stale);
                    assert!(authority.fault.is_none());
                } else {
                    drop(result.unwrap());
                    assert!(authority.fault.is_some());
                }
                assert!(Arc::ptr_eq(&authority.state, &state));
                assert!(Arc::ptr_eq(&authority.revisions, &revisions));
                assert!(Arc::ptr_eq(&authority.queue, &queue));
                assert_eq!(authority.published_cursor(), cursor);
                assert_eq!(authority.accepted_sequence_high_water, high);
                assert_eq!(
                    (
                        authority.last_receipt.clone(),
                        authority.last_decision.clone(),
                        authority.last_event_facts.clone()
                    ),
                    receipts
                );
                let mut retired =
                    job.into_boxed_retirement(LivingPublicationRetirementPolicyV2::ReleaseHandle);
                let mut done = false;
                for attempt in 0..100000 {
                    if retired.poll(schedule[attempt % schedule.len()]).unwrap()
                        == LivingPublicationRetirementProgressV2::Ready
                    {
                        done = true;
                        break;
                    }
                }
                assert!(done);
                if stale {
                    authority.step().unwrap();
                    assert!(authority.queue.is_empty());
                }
            }
        }
    }

    #[test]
    fn fleet_hull_creator_source_derived_forward_and_once_only_extraction() {
        for kind in 0..6 {
            for schedule in SCHEDULES {
                let (mut job, queue, catalog) = source(kind);
                let pointer = &*job as *const _;
                let expected_visits = [78, 84, 183, 80, 88, 189][kind];
                assert!(job.take_ready_result().is_err());
                assert_eq!(run(&mut job, schedule).unwrap(), expected_visits);
                assert!(std::ptr::eq(&*job, pointer));
                assert!(Arc::ptr_eq(&job.queue, &queue));
                assert!(Arc::ptr_eq(&job.catalog, &catalog));
                let mut expected = source(kind).0.state.clone();
                apply_revision_range(&mut expected, &queue[0].revision, &catalog, 0).unwrap();
                let (actual, events) = job.take_ready_result().unwrap();
                assert_eq!(actual, expected);
                assert_eq!(events.len(), 2);
                assert!(job.take_ready_result().is_err());
            }
        }
    }

    #[test]
    fn fleet_hull_creator_private_catalog_hp_missing_precedence_and_exact_debit() {
        for case in 0..4 {
            for schedule in SCHEDULES {
                let (mut job, _, _) = source(5);
                let op = LivingCreatorOperationV2::EditHull {
                    target: t(99),
                    definition: LivingSlugV2::new(if case == 0 { "missing" } else { "scout" })
                        .unwrap(),
                    hit_points: if case == 1 {
                        0
                    } else if case == 2 {
                        11
                    } else {
                        1
                    },
                    return_credit: false,
                };
                job.queue = super::creator_batch_cursor_tests::queue(vec![op]).into();
                let before = job.state.clone();
                let mut visits = 0;
                let result = loop {
                    assert!(visits < 1000, "finite metadata rejection");
                    let b = schedule[visits % schedule.len()];
                    let result = job.poll(b);
                    visits += job.last_poll_visits;
                    assert!(job.last_poll_visits <= b);
                    if let Err(e) = result {
                        break e;
                    }
                };
                let reason = if case < 3 {
                    LivingCommandRejectionV2::Projection(LivingValidationErrorV2::Range {
                        field: if case == 0 {
                            "hull definition"
                        } else {
                            "hull hit points"
                        },
                    })
                } else {
                    LivingCommandRejectionV2::MissingTarget {
                        operation: 0,
                        reference: "hull",
                    }
                };
                assert_eq!(
                    result,
                    super::super::step_job::LivingLifecycleJobErrorV2::Fault(
                        LivingDeterministicFaultV2::Replay {
                            accepted_sequence: 1,
                            reason: Box::new(reason)
                        }
                    )
                );
                assert_eq!(visits, [7, 6, 6, 10][case]);
                assert_eq!(job.state, before);
            }
        }
    }
    #[test]
    fn fleet_hull_creator_private_cancellation_effects_follow_metadata_marker() {
        let (mut job, _, catalog) = super::cancellation_creator_tests::source(0);
        // Private source has three reservations plus an order: explicitly clear
        // reservations in setup, never claim this fixture as public admission.
        for i in 0..3 {
            job.state.hull_jobs.record_at_mut(i).unwrap().target_fleet = None;
        }
        let cancel = job.queue[0].revision.command.operations()[0].clone();
        let q = super::creator_batch_cursor_tests::queue(vec![
            cancel,
            LivingCreatorOperationV2::EditHull {
                target: t(61),
                definition: LivingSlugV2::new("escort").unwrap(),
                hit_points: 1,
                return_credit: false,
            },
            LivingCreatorOperationV2::EditFleet {
                target: t(60),
                owner: t(4),
                world: t(1),
            },
        ]);
        job.queue = Arc::clone(&q).into();
        let mut expected = job.state.clone();
        let effects = apply_revision_range(&mut expected, &q[0].revision, &catalog, 0).unwrap();
        assert_eq!(
            effects,
            vec![
                LivingEventKindV2::RefundDiscarded,
                LivingEventKindV2::ConstructionCancelled
            ]
        );
        run(&mut job, SCHEDULES[3]).unwrap();
        assert_eq!(job.state, expected);
        assert_eq!(
            job.events
                .payloads()
                .iter()
                .map(|p| p.kind)
                .collect::<Vec<_>>(),
            vec![
                LivingEventKindV2::FleetArrived,
                LivingEventKindV2::CreatorIntervention,
                LivingEventKindV2::RefundDiscarded,
                LivingEventKindV2::ConstructionCancelled,
            ]
        );
    }
}

#[cfg(test)]
mod fleet_structural_creator_tests {
    pub(super) use super::geometry_creator_tests::SCHEDULES;
    use super::*;
    use crate::living::LivingNameV2;
    use crate::living::model::*;
    fn id(n: u8) -> LivingEntityIdV2 {
        LivingEntityIdV2([n; 16])
    }
    fn t(n: u8) -> LivingCommandTargetV2 {
        LivingCommandTargetV2::Existing { id: id(n) }
    }
    pub(super) fn parents(kind: usize) -> (usize, usize) {
        if kind < 3 {
            (kind, kind)
        } else {
            [(0, 1), (0, 2), (1, 0), (1, 2), (2, 0), (2, 1)][kind - 3]
        }
    }
    pub(super) fn operation(kind: usize) -> LivingCreatorOperationV2 {
        let (source, target) = parents(kind);
        if kind < 3 {
            LivingCreatorOperationV2::SplitFleet {
                local: 0,
                source: t(30 + source as u8),
                hulls: vec![t(40 + 2 * source as u8)],
            }
        } else {
            LivingCreatorOperationV2::MergeFleets {
                source: t(30 + source as u8),
                target: t(30 + target as u8),
            }
        }
    }
    // Private owning fixture, with a deliberately fixed post-seal revision ID.
    // Independent SHA-256 domain formula fixes new Fleet key78.. after all30..32.
    // No authenticated admission or wire-vector claim is made for this fixture.
    pub(super) fn source(
        kind: usize,
    ) -> (
        Box<LivingQueuedCreatorJobV2>,
        Arc<Vec<QueuedCommandV2>>,
        Arc<ValidatedLivingCatalogPackV2>,
    ) {
        let (mut job, _, catalog) = super::fleet_hull_creator_tests::source(0);
        let mut queue = super::creator_batch_cursor_tests::queue(vec![operation(kind)]);
        Arc::make_mut(&mut Arc::make_mut(&mut queue)[0].revision).id =
            LivingRevisionIdV2([0x67; 32]);
        job.queue = Arc::clone(&queue).into();
        (job, queue, catalog)
    }
    pub(super) fn run(
        job: &mut LivingQueuedCreatorJobV2,
        schedule: &[usize],
    ) -> Result<usize, super::super::step_job::LivingLifecycleJobErrorV2> {
        super::geometry_creator_tests::run(job, schedule)
    }
    #[test]
    fn structural_creator_private_reference_guard_diagnostic_and_sticky_matrix() {
        for kind in 0..9 {
            for case in 0..11 {
                for schedule in SCHEDULES {
                    let (mut job, old_queue, catalog) = source(kind);
                    let (source, target) = parents(kind);
                    let mut op = operation(kind);
                    match case {
                        0 => {}
                        1 => {
                            job.state.fleets.record_at_mut(source).unwrap().location =
                                LivingFleetLocationV2::Travelling {
                                    origin_world: id(2),
                                    destination_world: id(3),
                                    departure_tick: LivingTickV2(0),
                                    arrival_tick: LivingTickV2(10),
                                }
                        }
                        2 => {
                            job.state.hull_jobs.record_at_mut(0).unwrap().target_fleet =
                                Some(id(30 + source as u8))
                        }
                        3 => {
                            job.state.hull_jobs.record_at_mut(1).unwrap().target_fleet =
                                Some(id(30 + source as u8))
                        }
                        4 => {
                            job.state.hull_jobs.record_at_mut(2).unwrap().target_fleet =
                                Some(id(30 + source as u8))
                        }
                        5 => {
                            if let LivingCreatorOperationV2::SplitFleet { hulls, .. } = &mut op {
                                hulls.clear();
                            } else {
                                job.state.fleets.record_at_mut(target).unwrap().owner = id(5);
                            }
                        }
                        6 => {
                            if let LivingCreatorOperationV2::SplitFleet { hulls, .. } = &mut op {
                                hulls.push(hulls[0]);
                            } else {
                                job.state.fleets.record_at_mut(target).unwrap().location =
                                    LivingFleetLocationV2::Docked { world: id(3) };
                            }
                        }
                        7 => {
                            if let LivingCreatorOperationV2::SplitFleet { hulls, .. } = &mut op {
                                *hulls = vec![t(99)];
                            } else if let LivingCreatorOperationV2::MergeFleets { target, .. } =
                                &mut op
                            {
                                *target = t(99);
                            }
                        }
                        8 => {
                            if let LivingCreatorOperationV2::SplitFleet { source, .. }
                            | LivingCreatorOperationV2::MergeFleets { source, .. } = &mut op
                            {
                                *source = t(99);
                            }
                        }
                        9 => {
                            if let LivingCreatorOperationV2::MergeFleets { target, source } =
                                &mut op
                            {
                                *source = *target;
                            } else {
                                let mut rows = job.state.fleets.clone().into_vec();
                                let mut row = rows[0].clone();
                                row.id = LivingEntityIdV2([
                                    0x78, 0x50, 0x64, 0x36, 0x32, 0x8f, 0x88, 0x29, 0x89, 0x6d,
                                    0xca, 0x4e, 0xfb, 0x91, 0x7e, 0x36,
                                ]);
                                rows.push(row);
                                job.state.fleets = LivingSortedVecV2::new(rows).unwrap();
                            }
                        }
                        10 => {
                            if kind >= 3 {
                                let mut rows =
                                    job.state.fleets.as_slice()[source].hulls.clone().into_vec();
                                rows[0] =
                                    job.state.fleets.as_slice()[target].hulls.as_slice()[0].clone();
                                rows.sort_by_key(|row| row.id);
                                job.state.fleets.record_at_mut(source).unwrap().hulls =
                                    LivingSortedVecV2::new(rows).unwrap();
                            }
                        }
                        _ => unreachable!(),
                    }
                    let mut queue = super::creator_batch_cursor_tests::queue(vec![op]);
                    Arc::make_mut(&mut Arc::make_mut(&mut queue)[0].revision).id =
                        old_queue[0].revision.id;
                    job.queue = Arc::clone(&queue).into();
                    let mut expected = job.state.clone();
                    let reference =
                        apply_revision_range(&mut expected, &queue[0].revision, &catalog, 0);
                    let result = run(&mut job, schedule);
                    match reference {
                        Ok(effects) => {
                            assert!(effects.is_empty());
                            result.unwrap();
                            assert_eq!(job.state, expected);
                            assert_eq!(job.events.len(), 2);
                            let actual = job.take_ready_result().unwrap();
                            assert_eq!(actual.0, expected);
                            assert!(job.take_ready_result().is_err());
                        }
                        Err(reason) => {
                            let expected = super::super::step_job::LivingLifecycleJobErrorV2::Fault(
                                LivingDeterministicFaultV2::Replay {
                                    accepted_sequence: 1,
                                    reason: Box::new(reason),
                                },
                            );
                            assert_eq!(result, Err(expected.clone()));
                            assert!(job.last_poll_visits > 0);
                            assert_eq!(job.events.len(), 1);
                            assert_eq!(job.state.accepted_sequence, 0);
                            assert!(job.take_ready_result().is_err());
                            let Some(QueuedCreatorInsertion::FleetStructural(work)) =
                                &job.insertion
                            else {
                                panic!("original structural owner");
                            };
                            assert!(work.fault.is_some());
                            if kind >= 3 && case == 10 {
                                assert!(work.pending_source.is_some());
                                assert!(work.source_hulls.is_some());
                                assert!(work.target_hulls.is_some());
                                assert!(work.source_parent.is_some());
                                assert!(work.target_parent.is_some());
                            }
                            for budget in [1, 64, 1024] {
                                assert_eq!(job.poll(budget), Err(expected.clone()));
                                assert_eq!(job.last_poll_visits, 0);
                            }
                        }
                    }
                }
            }
        }
    }
    #[test]
    fn structural_creator_private_late_fault_precedes_marker() {
        let (mut job, _, _) = source(2);
        job.queue = super::creator_batch_cursor_tests::queue(vec![
            operation(2),
            LivingCreatorOperationV2::MergeFleets {
                target: t(99),
                source: t(98),
            },
        ])
        .into();
        for _ in 1..=u16::MAX {
            job.events
                .record(
                    LivingEventProvenanceV2::Autonomous {
                        phase: LivingPhaseV2::ResolveArrivals,
                    },
                    LivingEventKindV2::FleetArrived,
                )
                .unwrap();
        }
        let error = run(&mut job, SCHEDULES[3]).unwrap_err();
        assert_eq!(
            error,
            super::super::step_job::LivingLifecycleJobErrorV2::Fault(
                LivingDeterministicFaultV2::Replay {
                    accepted_sequence: 1,
                    reason: Box::new(LivingCommandRejectionV2::MissingTarget {
                        operation: 1,
                        reference: "fleet"
                    })
                }
            )
        );
        assert!(job.append.is_none());
        assert_eq!(job.operation_index, 1);
        assert_eq!(job.state.accepted_sequence, 0);
        assert_eq!(job.state.fleets.as_slice().len(), 4);
    }

    #[test]
    fn structural_creator_source_derived_forward_once_only_extraction() {
        let forward = [410, 416, 515, 371, 470, 371, 476, 470, 476];
        for (kind, expected_visits) in forward.into_iter().enumerate() {
            for schedule in SCHEDULES {
                let (mut job, queue, catalog) = source(kind);
                let pointer = &*job as *const _;
                let mut expected = job.state.clone();
                apply_revision_range(&mut expected, &queue[0].revision, &catalog, 0).unwrap();
                assert!(job.take_ready_result().is_err());
                assert_eq!(run(&mut job, schedule).unwrap(), expected_visits);
                assert!(std::ptr::eq(&*job, pointer));
                assert!(Arc::ptr_eq(&job.queue, &queue));
                assert!(Arc::ptr_eq(&job.catalog, &catalog));
                let (actual, events) = job.take_ready_result().unwrap();
                assert_eq!(actual, expected);
                assert_eq!(events.len(), 2);
                assert!(job.take_ready_result().is_err());
            }
        }
    }
    #[test]
    fn structural_creator_private_candidate_fault_current_and_genuine_stale_tail() {
        for stale in [false, true] {
            for schedule in SCHEDULES {
                let mut authority = crate::living::builtin_first_expansion_v2().unwrap();
                let fleet = authority
                    .state
                    .fleets
                    .as_slice()
                    .iter()
                    .find(|f| {
                        matches!(f.location, LivingFleetLocationV2::Docked { .. })
                            && !authority
                                .state
                                .hull_jobs
                                .as_slice()
                                .iter()
                                .any(|j| j.target_fleet == Some(f.id))
                    })
                    .unwrap()
                    .clone();
                let LivingFleetLocationV2::Docked { world } = fleet.location else {
                    unreachable!()
                };
                let envelope = |a: &LivingGalaxyAuthorityV2, operations| {
                    let c = a.published_cursor();
                    LivingCommandEnvelopeV2 {
                        expected_committed_revision: c.committed_revision,
                        expected_tick: c.tick,
                        expected_pending_sequence: c.pending_sequence,
                        mode: LivingCommandModeV2::Running,
                        command: LivingCommandV2::CreatorBatch { operations },
                    }
                };
                authority
                    .submit(envelope(
                        &authority,
                        vec![
                            LivingCreatorOperationV2::CreateHull {
                                local: 0,
                                fleet: LivingCommandTargetV2::Existing { id: fleet.id },
                                definition: crate::living::LivingSlugV2::new("scout").unwrap(),
                                hit_points: 7,
                                return_credit: false,
                            },
                            LivingCreatorOperationV2::SplitFleet {
                                local: 1,
                                source: LivingCommandTargetV2::Existing { id: fleet.id },
                                hulls: vec![LivingCommandTargetV2::Existing {
                                    id: fleet.hulls.as_slice()[0].id,
                                }],
                            },
                        ],
                    ))
                    .unwrap();
                let state = Arc::clone(&authority.state);
                let revisions = Arc::clone(&authority.revisions);
                let receipts = (
                    authority.last_receipt.clone(),
                    authority.last_decision.clone(),
                    authority.last_event_facts.clone(),
                );
                let mut job = Box::new(authority.begin_step().unwrap());
                let mut reached = false;
                for _ in 0..100000 {
                    if job.creator.as_ref().is_some_and(|c| {
                        matches!(&c.insertion, Some(QueuedCreatorInsertion::FleetStructural(work)) if work.stage == QueuedFleetStructuralStage::Resolve)
                    }) {
                        reached = true;
                        break;
                    }
                    assert_eq!(job.poll(1).unwrap(), LivingStepProgressV2::Pending);
                }
                assert!(reached);
                // Private candidate-only corruption after genuine unchanged admission.
                // The public authority remains valid for a genuinely admitted stale tail.
                let creator = job.creator.as_mut().unwrap();
                let pointer = &**creator as *const _;
                creator.state.fleets.find_mut(&fleet.id).unwrap().location =
                    LivingFleetLocationV2::Travelling {
                        origin_world: world,
                        destination_world: world,
                        departure_tick: LivingTickV2(0),
                        arrival_tick: LivingTickV2(99),
                    };
                let mut failed = false;
                for attempt in 0..10000 {
                    match job.poll(schedule[attempt % schedule.len()]) {
                        Err(LivingStepJobErrorV2::Fault(LivingDeterministicFaultV2::Replay {
                            reason,
                            ..
                        })) => {
                            assert_eq!(*reason, LivingCommandRejectionV2::ReferencedTarget);
                            failed = true;
                            break;
                        }
                        Ok(LivingStepProgressV2::Pending) => {}
                        other => panic!("private structural fault {other:?}"),
                    }
                }
                assert!(failed);
                assert!(
                    job.fault.is_none(),
                    "closed structural cause stays in original child"
                );
                for budget in [1, 64, 1024] {
                    assert!(matches!(
                        job.poll(budget),
                        Err(LivingStepJobErrorV2::Fault(_))
                    ));
                    assert_eq!(job.last_poll_visits(), 0);
                }
                assert!(std::ptr::eq(&**job.creator.as_ref().unwrap(), pointer));
                let LivingStepPreparationV2::Failed(token) =
                    job.take_synchronous_preparation(&authority).unwrap()
                else {
                    panic!("private failed token")
                };
                assert!(job.take_synchronous_preparation(&authority).is_err());
                if stale {
                    authority
                        .submit(envelope(
                            &authority,
                            vec![LivingCreatorOperationV2::CreateSystem {
                                local: 0,
                                name: LivingNameV2::new("Accepted structural fault tail").unwrap(),
                            }],
                        ))
                        .unwrap();
                }
                let queue = Arc::clone(&authority.queue);
                let cursor = authority.published_cursor();
                let high = authority.accepted_sequence_high_water;
                let result = authority.commit_step_failure(token);
                if stale {
                    assert_eq!(result.unwrap_err().reason(), &LivingStepJobErrorV2::Stale);
                    assert!(authority.fault.is_none());
                } else {
                    drop(result.unwrap());
                    assert!(authority.fault.is_some());
                }
                assert!(Arc::ptr_eq(&authority.state, &state));
                assert!(Arc::ptr_eq(&authority.revisions, &revisions));
                assert!(Arc::ptr_eq(&authority.queue, &queue));
                assert_eq!(authority.published_cursor(), cursor);
                assert_eq!(authority.accepted_sequence_high_water, high);
                assert_eq!(
                    (
                        authority.last_receipt.clone(),
                        authority.last_decision.clone(),
                        authority.last_event_facts.clone()
                    ),
                    receipts
                );
                let mut retired =
                    job.into_boxed_retirement(LivingPublicationRetirementPolicyV2::ReleaseHandle);
                let mut done = false;
                for attempt in 0..100000 {
                    if retired.poll(schedule[attempt % schedule.len()]).unwrap()
                        == LivingPublicationRetirementProgressV2::Ready
                    {
                        done = true;
                        break;
                    }
                }
                assert!(done);
                if stale {
                    authority.step().unwrap();
                    assert!(authority.queue.is_empty());
                }
            }
        }
    }

    #[test]
    fn structural_creator_private_cancellation_effects_follow_structural_marker() {
        let (mut job, _, catalog) = super::cancellation_creator_tests::source(0);
        // Private source has three reservations plus an order: explicitly clear
        // reservations in setup, never claim this fixture as public admission.
        for i in 0..3 {
            job.state.hull_jobs.record_at_mut(i).unwrap().target_fleet = None;
        }
        let cancel = job.queue[0].revision.command.operations()[0].clone();
        let q = super::creator_batch_cursor_tests::queue(vec![
            cancel,
            LivingCreatorOperationV2::SplitFleet {
                local: 0,
                source: t(60),
                hulls: vec![t(61)],
            },
            LivingCreatorOperationV2::MergeFleets {
                target: t(60),
                source: LivingCommandTargetV2::Local { local: 0 },
            },
        ]);
        job.queue = Arc::clone(&q).into();
        let mut expected = job.state.clone();
        let effects = apply_revision_range(&mut expected, &q[0].revision, &catalog, 0).unwrap();
        assert_eq!(
            effects,
            vec![
                LivingEventKindV2::RefundDiscarded,
                LivingEventKindV2::ConstructionCancelled
            ]
        );
        run(&mut job, SCHEDULES[3]).unwrap();
        assert_eq!(job.state, expected);
        assert_eq!(
            job.events
                .payloads()
                .iter()
                .map(|p| p.kind)
                .collect::<Vec<_>>(),
            vec![
                LivingEventKindV2::FleetArrived,
                LivingEventKindV2::CreatorIntervention,
                LivingEventKindV2::RefundDiscarded,
                LivingEventKindV2::ConstructionCancelled,
            ]
        );
    }
    #[test]
    fn structural_creator_private_missing_beats_dangling_reservation() {
        for kind in [0, 3] {
            let (mut job, _, _) = source(kind);
            let mut op = operation(kind);
            match &mut op {
                LivingCreatorOperationV2::SplitFleet { source, .. } => *source = t(99),
                LivingCreatorOperationV2::MergeFleets { target, .. } => *target = t(99),
                _ => unreachable!(),
            }
            job.state.hull_jobs.record_at_mut(0).unwrap().target_fleet = Some(id(99));
            job.queue = super::creator_batch_cursor_tests::queue(vec![op]).into();
            assert_eq!(
                run(&mut job, SCHEDULES[0]),
                Err(super::super::step_job::LivingLifecycleJobErrorV2::Fault(
                    LivingDeterministicFaultV2::Replay {
                        accepted_sequence: 1,
                        reason: Box::new(LivingCommandRejectionV2::MissingTarget {
                            operation: 0,
                            reference: "fleet"
                        })
                    }
                ))
            );
        }
    }
}

#[cfg(test)]
mod removal_creator_tests {
    pub(super) use super::geometry_creator_tests::SCHEDULES;
    use super::*;
    use crate::living::model::*;
    use crate::living::{LivingNameV2, LivingSlugV2};
    pub(super) fn id(n: u8) -> LivingEntityIdV2 {
        LivingEntityIdV2([n; 16])
    }
    fn t(n: u8) -> LivingCommandTargetV2 {
        LivingCommandTargetV2::Existing { id: id(n) }
    }
    fn slug(s: &str) -> LivingSlugV2 {
        LivingSlugV2::new(s).unwrap()
    }
    fn reasons(n: u8) -> LivingSortedVecV2<LivingBlockedReasonV2> {
        let mut values = Vec::with_capacity(7);
        if n != 1 {
            values.push(LivingBlockedReasonV2::Disabled);
        }
        if n == 2 {
            values.push(LivingBlockedReasonV2::MissingDeposit {
                required: 2,
                available: 0,
            });
        }
        LivingSortedVecV2::new(values).unwrap()
    }
    /// Private malformed-state ownership fixture. Every collection and ordinary
    /// nested allocation is populated; no authored-genesis claim is made here.
    pub(super) fn state() -> LivingGalaxyStateV2 {
        let (job, _, _) = super::fleet_hull_creator_tests::source(0);
        let mut state = job.state.clone();
        state.systems = LivingSortedVecV2::new(
            (10..13)
                .map(|n| LivingSystemV2 {
                    id: id(n),
                    name: LivingNameV2::new("System").unwrap(),
                })
                .collect(),
        )
        .unwrap();
        state.stars = LivingSortedVecV2::new(
            (0..3)
                .map(|n| LivingStarV2 {
                    id: id(13 + n),
                    system: id(10 + n),
                    archetype: slug("yellow"),
                    name: LivingNameV2::new("Star").unwrap(),
                })
                .collect(),
        )
        .unwrap();
        state.civilizations = LivingSortedVecV2::new(
            (4..7)
                .map(|n| LivingCivilizationV2 {
                    id: id(n),
                    name: LivingNameV2::new("Actor").unwrap(),
                    policy: LivingPolicyV2(slug("neutral")),
                    status: LivingCivilizationStatusV2::Active,
                })
                .collect(),
        )
        .unwrap();
        for n in 0..3 {
            state.worlds.record_at_mut(n).unwrap().system = id(10 + n as u8);
            state.worlds.record_at_mut(n).unwrap().owner = Some(id(4 + n as u8));
            state.fleets.record_at_mut(n).unwrap().location = LivingFleetLocationV2::Docked {
                world: id(1 + n as u8),
            };
        }
        // HullJob precedes Fleet in the reference scan: lane9 -> Fleet32 on the
        // first growth pass, then job60 on the next, then a complete no-growth pass.
        state.hull_jobs.record_at_mut(0).unwrap().target_fleet = Some(id(32));
        state.deposits = LivingSortedVecV2::new(
            (0..3)
                .map(|n| LivingDepositV2 {
                    id: id(70 + n),
                    world: id(1 + n),
                    resource: LivingResourceV2::Ore,
                    remaining_units: 99,
                })
                .collect(),
        )
        .unwrap();
        state.facilities = LivingSortedVecV2::new(
            (0..3)
                .map(|n| LivingFacilityV2 {
                    id: id(73 + n),
                    world: id(1 + n),
                    definition: slug("mine"),
                    status: LivingFacilityStatusV2::Blocked,
                    paid_alloy: 40,
                    hit_points: 0,
                    next_due: Some(LivingTickV2(100)),
                    next_repair: None,
                    blocked_reasons: reasons(n),
                    disabled: true,
                })
                .collect(),
        )
        .unwrap();
        state.construction_jobs = LivingSortedVecV2::new(
            (0..3)
                .map(|n| LivingConstructionJobV2 {
                    id: id(76 + n),
                    world: id(1 + n),
                    owner: id(4 + n),
                    definition: slug("mine"),
                    accepted_tick: LivingTickV2(0),
                    completion_tick: LivingTickV2(100),
                    paid_alloy: 40,
                })
                .collect(),
        )
        .unwrap();
        state.routes = LivingSortedVecV2::new(
            (0..3)
                .map(|n| LivingFreightRouteV2 {
                    id: id(79 + n),
                    kind: LivingRouteKindV2::Trade,
                    source_world: id(1 + n),
                    destination_world: id(3),
                    resource: LivingResourceV2::Ore,
                    batch_size: 3,
                    cadence_ticks: 10,
                    source_reserve: 20,
                    source_owner: id(4 + n),
                    receiver_owner: id(6),
                    next_due: LivingTickV2(100),
                    suspended: true,
                })
                .collect(),
        )
        .unwrap();
        state.hazards = LivingSortedVecV2::new(
            (0..3)
                .map(|n| LivingHazardV2 {
                    id: id(82 + n),
                    lane: id(if n == 0 { 8 } else { 9 }),
                    definition: slug("ion_storm"),
                    start_tick: LivingTickV2(100),
                    end_tick: LivingTickV2(200),
                })
                .collect(),
        )
        .unwrap();
        state.colonies = LivingSortedVecV2::new(
            (0..3)
                .map(|n| LivingColonyV2 {
                    world: id(1 + n),
                    owner: id(4 + n),
                    hub: LivingHubV2 {
                        energy_next_due: LivingTickV2(100),
                        ore_next_due: LivingTickV2(100),
                        fallback_next_due: LivingTickV2(100),
                        blocked_reasons: reasons(n),
                    },
                })
                .collect(),
        )
        .unwrap();
        state.observations = LivingSortedVecV2::new(
            (0..3)
                .map(|n| LivingObservationV2 {
                    civilization: id(4 + n),
                    world: id(1 + n),
                    observed_tick: LivingTickV2(0),
                    owner: Some(id(6)),
                    remaining_deposit: 99,
                    facilities: LivingSortedVecV2::new(if n == 1 {
                        Vec::with_capacity(7)
                    } else {
                        vec![LivingObservedFacilityV2 {
                            definition: slug("mine"),
                            count: 1,
                        }]
                    })
                    .unwrap(),
                    stationed_hulls: LivingSortedVecV2::new(vec![LivingObservedHullV2 {
                        definition: slug("scout"),
                        count: 2,
                    }])
                    .unwrap(),
                    operational_defense_batteries: None,
                })
                .collect(),
        )
        .unwrap();
        state.relations = LivingSortedVecV2::new(
            (4..7)
                .map(|n| LivingRelationV2 {
                    from: id(n),
                    to: id(9),
                    base_disposition: 0,
                    reasons: Default::default(),
                })
                .collect(),
        )
        .unwrap();
        state.agreements = LivingSortedVecV2::new(
            (4..7)
                .map(|n| LivingAgreementV2 {
                    kind: LivingAgreementKindV2::Trade,
                    participant_a: id(n),
                    participant_b: id(9),
                    start_tick: LivingTickV2(0),
                    end_tick: LivingTickV2(100),
                })
                .collect(),
        )
        .unwrap();
        state.wars = LivingSortedVecV2::new(
            (4..7)
                .map(|n| LivingWarV2 {
                    participant_a: id(n),
                    participant_b: id(9),
                    declarer: id(n),
                    start_tick: LivingTickV2(0),
                    end_tick: LivingTickV2(100),
                })
                .collect(),
        )
        .unwrap();
        state.settlement_claims = LivingSortedVecV2::new(
            (0..3)
                .map(|n| LivingSettlementClaimV2 {
                    world: id(1 + n),
                    civilization: id(4 + n),
                    arrival_tick: LivingTickV2(0),
                    close_tick: LivingTickV2(50),
                })
                .collect(),
        )
        .unwrap();
        state.occupations = LivingSortedVecV2::new(
            (0..3)
                .map(|n| LivingOccupationV2 {
                    world: id(1 + n),
                    claimant: id(4 + n),
                    progress_ticks: 1,
                })
                .collect(),
        )
        .unwrap();
        // Preserve fresh allocated-empty originals, instead of cloning the fixture
        // after these allocations (Vec::clone does not preserve spare capacity).
        state
            .fleets
            .record_at_mut(1)
            .unwrap()
            .order
            .as_mut()
            .unwrap()
            .lane_path = Vec::with_capacity(7);
        state
    }
    pub(super) fn operation(kind: usize) -> LivingCreatorOperationV2 {
        use crate::living::LivingCascadeDispositionV2 as C;
        let (target, dependents): (u8, &[u8]) = match kind {
            0 => (9, &[32, 44, 45, 60, 83, 84]),
            1 => (
                10,
                &[
                    1, 8, 9, 13, 30, 31, 32, 40, 41, 42, 43, 44, 45, 60, 70, 73, 76, 79, 82, 83, 84,
                ],
            ),
            2 => (
                11,
                &[
                    2, 8, 9, 14, 31, 32, 20, 21, 22, 42, 43, 44, 45, 60, 61, 62, 71, 74, 77, 80,
                    82, 83, 84,
                ],
            ),
            3 => (
                12,
                &[3, 15, 32, 20, 21, 22, 44, 45, 60, 72, 75, 78, 79, 80, 81],
            ),
            4 => (73, &[]),
            5 => (40, &[]),
            _ => panic!("removal fixture kind"),
        };
        LivingCreatorOperationV2::RemoveEntity {
            target: t(target),
            cascade: if dependents.is_empty() {
                C::RejectIfReferenced
            } else {
                C::RemoveListed {
                    dependents: dependents.iter().rev().map(|n| t(*n)).collect(),
                }
            },
        }
    }
    pub(super) fn source(
        kind: usize,
    ) -> (
        Box<LivingQueuedCreatorJobV2>,
        Arc<Vec<QueuedCommandV2>>,
        Arc<ValidatedLivingCatalogPackV2>,
    ) {
        let catalog = Arc::new(crate::living::living_core_pack_v2().unwrap());
        let queue = super::creator_batch_cursor_tests::queue(vec![operation(kind)]);
        let mut job = Box::new(LivingQueuedCreatorJobV2::begin(
            state(),
            Arc::clone(&queue),
            Arc::clone(&catalog),
        ));
        job.events
            .record(
                LivingEventProvenanceV2::Autonomous {
                    phase: LivingPhaseV2::ResolveArrivals,
                },
                LivingEventKindV2::FleetArrived,
            )
            .unwrap();
        job.events = job.events.clone();
        (job, queue, catalog)
    }
    pub(super) fn run(
        job: &mut LivingQueuedCreatorJobV2,
        schedule: &[usize],
    ) -> Result<usize, super::super::step_job::LivingLifecycleJobErrorV2> {
        super::geometry_creator_tests::run(job, schedule)
    }
    #[test]
    fn removal_creator_private_all_registry_reference_and_unchanged_memories() {
        for kind in 0..6 {
            for schedule in SCHEDULES {
                let (mut job, queue, catalog) = source(kind);
                let mut expected = job.state.clone();
                let (effects, recovery) =
                    apply_revision_with_recoveries(&mut expected, &queue[0].revision, &catalog)
                        .unwrap();
                assert!(effects.is_empty() && recovery.is_empty());
                let memories = job.state.observations.clone();
                run(&mut job, schedule).unwrap();
                assert!(matches!(job.stage, QueuedCreatorStage::Ready));
                assert_eq!(job.state, expected);
                for actual in job.state.observations.as_slice() {
                    let original = memories
                        .as_slice()
                        .iter()
                        .find(|row| {
                            row.civilization == actual.civilization && row.world == actual.world
                        })
                        .unwrap();
                    assert_eq!(actual, original);
                }
                if kind == 5 {
                    assert_eq!(job.state.fleets.as_slice()[0].hulls.as_slice().len(), 1);
                }
                assert_eq!(job.events.payloads().len(), 2);
                assert!(job.events.creator_recoveries.is_empty());
                assert!(job.insertion.is_none() && job.retirement.is_empty());
            }
        }
    }
    pub(super) fn failure_source(
        case: usize,
    ) -> (
        Box<LivingQueuedCreatorJobV2>,
        Arc<Vec<QueuedCommandV2>>,
        Arc<ValidatedLivingCatalogPackV2>,
    ) {
        use crate::living::LivingCascadeDispositionV2 as C;
        let (mut job, _, catalog) = source(0);
        let operation = match case {
            0 => LivingCreatorOperationV2::ScrapFacility {
                target: t(10),
                cascade: C::RejectIfReferenced,
            },
            1 => LivingCreatorOperationV2::RemoveEntity {
                target: t(99),
                cascade: C::RejectIfReferenced,
            },
            2 => LivingCreatorOperationV2::RemoveEntity {
                target: t(9),
                cascade: C::RejectIfReferenced,
            },
            3 => LivingCreatorOperationV2::RemoveEntity {
                target: t(9),
                cascade: C::RemoveListed {
                    dependents: vec![t(32)],
                },
            },
            4 => {
                job.state.civilizations.record_at_mut(2).unwrap().id = id(90);
                job.state.relations.record_at_mut(2).unwrap().from = id(90);
                LivingCreatorOperationV2::RemoveEntity {
                    target: t(90),
                    cascade: C::RejectIfReferenced,
                }
            }
            5 => LivingCreatorOperationV2::RemoveEntity {
                target: t(9),
                cascade: C::RemoveListed {
                    dependents: vec![t(32), t(32)],
                },
            },
            6 => LivingCreatorOperationV2::RemoveEntity {
                target: t(9),
                cascade: C::RemoveListed {
                    dependents: vec![t(32), t(44), t(45), t(60), t(83), t(99)],
                },
            },
            _ => LivingCreatorOperationV2::ScrapFacility {
                target: t(73),
                cascade: C::RemoveListed {
                    dependents: vec![t(73)],
                },
            },
        };
        // Case 5 deliberately corrupts the already-sealed private command;
        // public identical syntactic duplicates fail structural admission.
        let mut q = super::creator_batch_cursor_tests::queue(vec![if case == 5 {
            self::operation(0)
        } else {
            operation.clone()
        }]);
        if case == 5 {
            Arc::make_mut(&mut Arc::make_mut(&mut q)[0].revision).command =
                LivingCommandV2::CreatorBatch {
                    operations: vec![operation],
                };
        }
        job.queue = Arc::clone(&q).into();
        (job, q, catalog)
    }
    #[test]
    fn removal_creator_private_error_precedence_aliases_legacy_and_sticky() {
        for case in 0..8 {
            for schedule in SCHEDULES {
                let (mut job, q, catalog) = failure_source(case);
                let mut expected = job.state.clone();
                let reason =
                    apply_revision_range(&mut expected, &q[0].revision, &catalog, 0).unwrap_err();
                let failure = super::super::step_job::LivingLifecycleJobErrorV2::Fault(
                    LivingDeterministicFaultV2::Replay {
                        accepted_sequence: 1,
                        reason: Box::new(reason),
                    },
                );
                assert_eq!(run(&mut job, schedule), Err(failure.clone()));
                assert_eq!(job.state, expected);
                let Some(QueuedCreatorInsertion::Removal(work)) = &job.insertion else {
                    panic!("failed original retained")
                };
                assert!(work.fault.is_some());
                for budget in [1, 64, 1024] {
                    assert_eq!(job.poll(budget), Err(failure.clone()));
                    assert_eq!(job.last_poll_visits, 0);
                }
                for budget in [0, 1025] {
                    assert_eq!(
                        job.poll(budget),
                        Err(super::super::step_job::LivingLifecycleJobErrorV2::Budget)
                    );
                    assert_eq!(job.last_poll_visits, 0);
                }
                assert!(job.take_ready_result().is_err());
            }
        }
    }
    #[test]
    fn removal_creator_private_candidate_fault_current_and_genuine_stale_tail() {
        for stale in [false, true] {
            for schedule in SCHEDULES {
                let mut authority = crate::living::builtin_first_expansion_v2().unwrap();
                let facility = authority.state.facilities.as_slice()[0].id;
                let envelope = |a: &LivingGalaxyAuthorityV2, operations| {
                    let c = a.published_cursor();
                    LivingCommandEnvelopeV2 {
                        expected_committed_revision: c.committed_revision,
                        expected_tick: c.tick,
                        expected_pending_sequence: c.pending_sequence,
                        mode: LivingCommandModeV2::Running,
                        command: LivingCommandV2::CreatorBatch { operations },
                    }
                };
                authority
                    .submit(envelope(
                        &authority,
                        vec![
                            LivingCreatorOperationV2::CreateSystem {
                                local: 0,
                                name: LivingNameV2::new("Accepted removal prefix").unwrap(),
                            },
                            LivingCreatorOperationV2::ScrapFacility {
                                target: LivingCommandTargetV2::Existing { id: facility },
                                cascade:
                                    crate::living::LivingCascadeDispositionV2::RejectIfReferenced,
                            },
                        ],
                    ))
                    .unwrap();
                let state = Arc::clone(&authority.state);
                let revisions = Arc::clone(&authority.revisions);
                let receipts = (
                    authority.last_receipt.clone(),
                    authority.last_decision.clone(),
                    authority.last_event_facts.clone(),
                );
                let mut job = Box::new(authority.begin_step().unwrap());
                let mut reached = false;
                for _ in 0..100000 {
                    if job.creator.as_ref().is_some_and(|c| {
                        matches!(&c.insertion, Some(QueuedCreatorInsertion::Removal(work)) if work.stage == QueuedRemovalStage::Resolve)
                    }) {
                        reached = true;
                        break;
                    }
                    assert_eq!(job.poll(1).unwrap(), LivingStepProgressV2::Pending);
                }
                assert!(reached);
                // Private candidate-only corruption after genuine unchanged admission.
                // The public authority remains valid for a genuinely admitted stale tail.
                let creator = job.creator.as_mut().unwrap();
                let pointer = &**creator as *const _;
                creator.state.facilities.find_mut(&facility).unwrap().id =
                    LivingEntityIdV2([0xfe; 16]);
                let mut failed = false;
                for attempt in 0..10000 {
                    match job.poll(schedule[attempt % schedule.len()]) {
                        Err(LivingStepJobErrorV2::Fault(LivingDeterministicFaultV2::Replay {
                            reason,
                            ..
                        })) => {
                            assert_eq!(
                                *reason,
                                LivingCommandRejectionV2::MissingTarget {
                                    operation: 1,
                                    reference: "facility"
                                }
                            );
                            assert_eq!(job.last_poll_visits(), 1);
                            failed = true;
                            break;
                        }
                        Ok(LivingStepProgressV2::Pending) => {}
                        other => panic!("private removal fault {other:?}"),
                    }
                }
                assert!(failed);
                assert!(
                    job.fault.is_none(),
                    "closed removal cause stays in original child"
                );
                for budget in [1, 64, 1024] {
                    assert!(matches!(
                        job.poll(budget),
                        Err(LivingStepJobErrorV2::Fault(_))
                    ));
                    assert_eq!(job.last_poll_visits(), 0);
                }
                assert!(std::ptr::eq(&**job.creator.as_ref().unwrap(), pointer));
                let LivingStepPreparationV2::Failed(token) =
                    job.take_synchronous_preparation(&authority).unwrap()
                else {
                    panic!("private failed token")
                };
                assert!(job.take_synchronous_preparation(&authority).is_err());
                if stale {
                    authority
                        .submit(envelope(
                            &authority,
                            vec![LivingCreatorOperationV2::CreateSystem {
                                local: 0,
                                name: LivingNameV2::new("Accepted removal fault tail").unwrap(),
                            }],
                        ))
                        .unwrap();
                }
                let queue = Arc::clone(&authority.queue);
                let cursor = authority.published_cursor();
                let high = authority.accepted_sequence_high_water;
                let result = authority.commit_step_failure(token);
                if stale {
                    assert_eq!(result.unwrap_err().reason(), &LivingStepJobErrorV2::Stale);
                    assert!(authority.fault.is_none());
                } else {
                    drop(result.unwrap());
                    assert!(authority.fault.is_some());
                }
                assert!(Arc::ptr_eq(&authority.state, &state));
                assert!(Arc::ptr_eq(&authority.revisions, &revisions));
                assert!(Arc::ptr_eq(&authority.queue, &queue));
                assert_eq!(authority.published_cursor(), cursor);
                assert_eq!(authority.accepted_sequence_high_water, high);
                assert_eq!(
                    (
                        authority.last_receipt.clone(),
                        authority.last_decision.clone(),
                        authority.last_event_facts.clone()
                    ),
                    receipts
                );
                let mut retired =
                    job.into_boxed_retirement(LivingPublicationRetirementPolicyV2::ReleaseHandle);
                let mut done = false;
                for attempt in 0..100000 {
                    if retired.poll(schedule[attempt % schedule.len()]).unwrap()
                        == LivingPublicationRetirementProgressV2::Ready
                    {
                        done = true;
                        break;
                    }
                }
                assert!(done);
                if stale {
                    authority.step().unwrap();
                    assert!(authority.queue.is_empty());
                }
            }
        }
    }

    pub(super) fn alias_source() -> (
        Box<LivingQueuedCreatorJobV2>,
        Arc<Vec<QueuedCommandV2>>,
        Arc<ValidatedLivingCatalogPackV2>,
    ) {
        let (mut job, _, catalog) = source(0);
        let mut operation = operation(0);
        let LivingCreatorOperationV2::RemoveEntity {
            cascade: crate::living::LivingCascadeDispositionV2::RemoveListed { dependents },
            ..
        } = &mut operation
        else {
            unreachable!()
        };
        dependents.push(LivingCommandTargetV2::Local { local: 7 });
        let mut queue = super::creator_batch_cursor_tests::queue(vec![self::operation(0)]);
        Arc::make_mut(&mut Arc::make_mut(&mut queue)[0].revision).command =
            LivingCommandV2::CreatorBatch {
                operations: vec![operation],
            };
        // Deliberate private Local alias injection, never a public admission claim.
        job.queue = Arc::clone(&queue).into();
        job.locals.insert(7, id(32));
        job.stage = QueuedCreatorStage::Operation;
        (job, queue, catalog)
    }
    #[test]
    fn removal_creator_private_resolved_local_alias_rejects_declared_length() {
        for schedule in SCHEDULES {
            let (mut job, queue, _) = alias_source();
            let before = job.state.clone();
            let failure = run(&mut job, schedule).unwrap_err();
            assert_eq!(
                failure,
                super::super::step_job::LivingLifecycleJobErrorV2::Fault(
                    LivingDeterministicFaultV2::Replay {
                        accepted_sequence: 1,
                        reason: Box::new(LivingCommandRejectionV2::CascadeMismatch)
                    }
                )
            );
            let mut reference = before.clone();
            let LivingCreatorOperationV2::RemoveEntity { cascade, .. } =
                &queue[0].revision.command.operations()[0]
            else {
                unreachable!()
            };
            assert_eq!(
                crate::living::rules::remove_creator_entity(
                    &mut reference,
                    id(9),
                    cascade,
                    0,
                    &|reference| match reference {
                        LivingCommandTargetV2::Existing { id } => *id,
                        LivingCommandTargetV2::Local { local } => job.locals[local],
                    },
                    &mut Vec::new()
                ),
                Err(LivingCommandRejectionV2::CascadeMismatch)
            );
            assert_eq!(job.state, before);
        }
    }

    #[test]
    fn removal_creator_private_every_registry_target_position_reference_matrix() {
        use crate::living::LivingCascadeDispositionV2 as C;
        let source = state();
        let mut groups = Vec::new();
        groups.push(
            source
                .systems
                .as_slice()
                .iter()
                .map(|r| r.id)
                .collect::<Vec<_>>(),
        );
        groups.push(
            source
                .stars
                .as_slice()
                .iter()
                .map(|r| r.id)
                .collect::<Vec<_>>(),
        );
        groups.push(
            source
                .worlds
                .as_slice()
                .iter()
                .map(|r| r.id)
                .collect::<Vec<_>>(),
        );
        groups.push(
            source
                .lanes
                .as_slice()
                .iter()
                .map(|r| r.id)
                .collect::<Vec<_>>(),
        );
        groups.push(
            source
                .civilizations
                .as_slice()
                .iter()
                .map(|r| r.id)
                .collect::<Vec<_>>(),
        );
        groups.push(
            source
                .deposits
                .as_slice()
                .iter()
                .map(|r| r.id)
                .collect::<Vec<_>>(),
        );
        groups.push(
            source
                .facilities
                .as_slice()
                .iter()
                .map(|r| r.id)
                .collect::<Vec<_>>(),
        );
        groups.push(
            source
                .construction_jobs
                .as_slice()
                .iter()
                .map(|r| r.id)
                .collect::<Vec<_>>(),
        );
        groups.push(
            source
                .hull_jobs
                .as_slice()
                .iter()
                .map(|r| r.id)
                .collect::<Vec<_>>(),
        );
        groups.push(
            source
                .fleets
                .as_slice()
                .iter()
                .map(|r| r.id)
                .collect::<Vec<_>>(),
        );
        groups.push(
            source
                .routes
                .as_slice()
                .iter()
                .map(|r| r.id)
                .collect::<Vec<_>>(),
        );
        groups.push(
            source
                .shipments
                .as_slice()
                .iter()
                .map(|r| r.id)
                .collect::<Vec<_>>(),
        );
        groups.push(
            source
                .hazards
                .as_slice()
                .iter()
                .map(|r| r.id)
                .collect::<Vec<_>>(),
        );

        groups.push(
            source
                .fleets
                .as_slice()
                .iter()
                .flat_map(|f| f.hulls.as_slice().iter().map(|h| h.id))
                .collect(),
        );
        for ids in groups {
            for target in [ids[0], ids[ids.len() / 2], ids[ids.len() - 1]] {
                for reject in [false, true] {
                    for schedule in SCHEDULES {
                        let (mut job, _, catalog) = self::source(0);
                        let preview =
                            crate::living::preview_living_removal_v2(&job.state, target).unwrap();
                        let cascade = if reject || preview.dependents.is_empty() {
                            C::RejectIfReferenced
                        } else {
                            C::RemoveListed {
                                dependents: preview
                                    .dependents
                                    .iter()
                                    .rev()
                                    .map(|id| LivingCommandTargetV2::Existing { id: *id })
                                    .collect(),
                            }
                        };
                        let q = super::creator_batch_cursor_tests::queue(vec![
                            LivingCreatorOperationV2::RemoveEntity {
                                target: LivingCommandTargetV2::Existing { id: target },
                                cascade,
                            },
                        ]);
                        job.queue = Arc::clone(&q).into();
                        let mut reference = job.state.clone();
                        let expected = apply_revision_with_recoveries(
                            &mut reference,
                            &q[0].revision,
                            &catalog,
                        );
                        match expected {
                            Ok((effects, _)) => {
                                assert!(effects.is_empty());
                                run(&mut job, schedule).unwrap();
                                assert_eq!(job.state, reference);
                                assert!(matches!(job.stage, QueuedCreatorStage::Ready));
                            }
                            Err(reason) => assert_eq!(
                                run(&mut job, schedule),
                                Err(super::super::step_job::LivingLifecycleJobErrorV2::Fault(
                                    LivingDeterministicFaultV2::Replay {
                                        accepted_sequence: 1,
                                        reason: Box::new(reason)
                                    }
                                ))
                            ),
                        }
                    }
                }
            }
        }
    }

    #[test]
    fn removal_creator_prior_effects_marker_order_and_later_error_precede_overflow() {
        for fail in [false, true] {
            for schedule in SCHEDULES {
                let (mut job, _, catalog) = super::cancellation_creator_tests::source(0);
                let operations = vec![
                    LivingCreatorOperationV2::CancelConstructionJob { target: t(10) },
                    LivingCreatorOperationV2::RemoveEntity {
                        target: t(if fail { 99 } else { 41 }),
                        cascade: crate::living::LivingCascadeDispositionV2::RejectIfReferenced,
                    },
                ];
                let q = super::creator_batch_cursor_tests::queue(operations);
                job.queue = Arc::clone(&q).into();
                let mut reference = job.state.clone();
                let expected =
                    apply_revision_with_recoveries(&mut reference, &q[0].revision, &catalog);
                if fail {
                    job.events = LivingPendingEventsV2::new();
                    for _ in 0..65_536 {
                        job.events
                            .record(
                                LivingEventProvenanceV2::Autonomous {
                                    phase: LivingPhaseV2::ResolveArrivals,
                                },
                                LivingEventKindV2::FleetArrived,
                            )
                            .unwrap();
                    }
                    assert_eq!(
                        run(&mut job, schedule),
                        Err(super::super::step_job::LivingLifecycleJobErrorV2::Fault(
                            LivingDeterministicFaultV2::Replay {
                                accepted_sequence: 1,
                                reason: Box::new(expected.unwrap_err())
                            }
                        ))
                    );
                    assert_eq!(job.events.payloads().len(), 65_536);
                    assert_eq!(job.effect_work.as_ref().unwrap().values.len(), 2);
                    assert!(job.append.is_none());
                    assert_eq!(job.operation_index, 1);
                } else {
                    let (effects, _) = expected.unwrap();
                    run(&mut job, schedule).unwrap();
                    assert_eq!(job.state, reference);
                    let (_, events) = job.take_ready_result().unwrap();
                    let reference_events = {
                        let mut expected_events = LivingPendingEventsV2::new();
                        expected_events
                            .record(
                                LivingEventProvenanceV2::Autonomous {
                                    phase: LivingPhaseV2::ResolveArrivals,
                                },
                                LivingEventKindV2::FleetArrived,
                            )
                            .unwrap();
                        expected_events
                            .record(
                                LivingEventProvenanceV2::Creator {
                                    revision: q[0].revision.id,
                                },
                                LivingEventKindV2::CreatorIntervention,
                            )
                            .unwrap();
                        for effect in effects {
                            expected_events
                                .record(
                                    LivingEventProvenanceV2::Creator {
                                        revision: q[0].revision.id,
                                    },
                                    effect,
                                )
                                .unwrap();
                        }
                        expected_events
                    };
                    assert_eq!(events, reference_events);
                }
            }
        }
    }
}
