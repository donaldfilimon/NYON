//! Bounded retirement of publication roots. ReleaseHandle releases handles;
//! DeferUntilUnique first consolidates this task's aliases. Neither policy
//! changes authority truth. Dropping an unfinished job is not bounded cleanup.
use super::*;
use crate::living::step_job::{
    LivingArcOwnerResolutionV2, LivingLifecycleProgressV2, LivingPhaseDeferredRetirementV2,
    LivingStateRetirementPolicyV2, RetirementFields, RetirementPoll, RetirementStep,
    resolve_living_retirement_arc_v2, retirement_node,
};
use std::collections::{BTreeMap, LinkedList};

/// A private movable slot, preserving its enclosing original allocation.
#[derive(Clone, Debug, PartialEq)]
pub(super) struct OwnerSlot<T>(Option<T>);
impl<T: Default> Default for OwnerSlot<T> {
    fn default() -> Self {
        Self(Some(T::default()))
    }
}
impl<T> From<T> for OwnerSlot<T> {
    fn from(value: T) -> Self {
        Self(Some(value))
    }
}
impl<T> OwnerSlot<T> {
    pub(super) fn is_present(&self) -> bool {
        self.0.is_some()
    }
    pub(super) fn take(&mut self) -> Option<T> {
        self.0.take()
    }
    pub(super) fn into_inner(self) -> T {
        self.0.expect("live owner")
    }
}
impl<T> std::ops::Deref for OwnerSlot<T> {
    type Target = T;
    fn deref(&self) -> &T {
        self.0.as_ref().expect("live owner")
    }
}
impl<T> std::ops::DerefMut for OwnerSlot<T> {
    fn deref_mut(&mut self) -> &mut T {
        self.0.as_mut().expect("live owner")
    }
}

/// Shared ownership contract; handle release is not payload reclamation.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum LivingPublicationRetirementPolicyV2 {
    /// Atomically release this handle, draining only if it is the final owner.
    ReleaseHandle,
    /// Retain exact shared owners until external snapshots are released.
    DeferUntilUnique,
}
impl LivingPublicationRetirementPolicyV2 {
    fn internal(self) -> LivingStateRetirementPolicyV2 {
        match self {
            Self::ReleaseHandle => LivingStateRetirementPolicyV2::ReleaseHandle,
            Self::DeferUntilUnique => LivingStateRetirementPolicyV2::DeferUntilUnique,
        }
    }
}
/// Progress is independent of cancellation/faults of the retired execution.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum LivingPublicationRetirementProgressV2 {
    /// More task-owned work remains.
    Pending,
    /// All independent work finished; exact outside-shared owners are retained.
    AwaitingExternalOwners,
    /// All admitted handles and shells have been released or drained.
    Ready,
}

// Fixed-size inline ownership avoids uncounted allocation at handoff.
#[allow(clippy::large_enum_variant)]
enum AuthorityOwner {
    Inline(LivingGalaxyAuthorityV2),
    Boxed(Box<LivingGalaxyAuthorityV2>),
}
impl AuthorityOwner {
    fn value(&mut self) -> &mut LivingGalaxyAuthorityV2 {
        match self {
            Self::Inline(v) => v,
            Self::Boxed(v) => v,
        }
    }
}
// Fixed-size inline ownership avoids uncounted allocation at handoff.
#[allow(clippy::large_enum_variant)]
enum StepOwner {
    Inline(LivingStepJobV2),
    Boxed(Box<LivingStepJobV2>),
}
impl StepOwner {
    fn value(&mut self) -> &mut LivingStepJobV2 {
        match self {
            Self::Inline(v) => v,
            Self::Boxed(v) => v,
        }
    }
}
// Fixed-size inline ownership avoids uncounted allocation at handoff.
#[allow(clippy::large_enum_variant)]
enum Owner {
    Authority(AuthorityOwner),
    Step(StepOwner),
    Capture(LivingBoundaryCaptureV2),
    Prepared(Box<LivingPreparedBoundaryV2>),
    Success(LivingPreparedBoundaryOwnersV2),
    Failure(Box<LivingPreparedStepFailureV2>),
    FailureOwners(LivingPreparedFailureOwnersV2),
    Retired(LivingRetiredBoundaryV2),
    Rejection(LivingStepCommitRejectionV2),
    Creator(Box<LivingQueuedCreatorJobV2>),
    Completed(
        Box<(
            LivingGalaxyStateV2,
            LivingPendingEventsV2,
            LivingCompletedStepPhasesV2,
        )>,
    ),
    Queue(Vec<QueuedCommandV2>),
    Queued(QueuedCommandV2),
    Fault(LivingDeterministicFaultV2),
    Reason(Box<LivingCommandRejectionV2>),
    Replay(LivingReplayBoundaryV2),
}
impl Owner {
    fn preparation(value: LivingStepPreparationV2) -> Self {
        match value {
            LivingStepPreparationV2::Ready(v) => Self::Prepared(v),
            LivingStepPreparationV2::Failed(v) => Self::Failure(v),
        }
    }
    // Empty archive/queue storage stays behind the child shells until group
    // payloads finish. Count its release separately from the original shell.
    fn release_storage(&mut self) -> bool {
        match self {
            Self::Authority(v) => {
                if v.value().archives.capacity() != 0 {
                    v.value().archives = Vec::new();
                    true
                } else {
                    false
                }
            }
            Self::Queue(v) if v.capacity() != 0 => {
                *v = Vec::new();
                true
            }
            _ => false,
        }
    }
}
struct Frame {
    owner: Owner,
    field: usize,
}
// Fixed-size inline ownership avoids uncounted allocation at handoff.
#[allow(clippy::large_enum_variant)]
enum Action {
    Child(Owner),
    Node(Box<dyn RetirementPoll>),
    Shared(SharedOwner),
    Work,
    Done,
}
fn node<T: RetirementFields<'static>>(v: T) -> Action {
    Action::Node(retirement_node(
        v,
        LivingStateRetirementPolicyV2::ReleaseHandle,
    ))
}
fn optional<T>(v: Option<T>, f: impl FnOnce(T) -> Action) -> Action {
    v.map_or(Action::Work, f)
}
fn fault(v: Option<LivingDeterministicFaultV2>) -> Action {
    optional(v, |v| Action::Child(Owner::Fault(v)))
}
fn job_error(v: LivingStepJobErrorV2) -> Action {
    match v {
        LivingStepJobErrorV2::Fault(v) => Action::Child(Owner::Fault(v)),
        LivingStepJobErrorV2::Budget
        | LivingStepJobErrorV2::Cancelled
        | LivingStepJobErrorV2::NotReady
        | LivingStepJobErrorV2::Stale
        | LivingStepJobErrorV2::PreparationCapacity => Action::Work,
    }
}

impl Frame {
    fn visit(&mut self) -> Action {
        let n = self.field;
        self.field += 1;
        match &mut self.owner {
            Owner::Capture(v) => match n {
                0 => optional(v.state.take(), |v| Action::Shared(SharedOwner::State(v))),
                1 => optional(v.catalog.take(), |v| {
                    Action::Shared(SharedOwner::Catalog(v))
                }),
                2 => optional(v.queue.take(), |v| Action::Shared(SharedOwner::Queue(v))),
                3 => optional(v.revisions.take(), |v| {
                    Action::Shared(SharedOwner::Journal(v))
                }),
                4 => optional(v.revision_modes.take(), |v| {
                    Action::Shared(SharedOwner::Modes(v))
                }),
                _ => Action::Done,
            },
            Owner::Authority(owner) => {
                let v = owner.value();
                match n {
                    0 => optional(v.catalog.take(), |v| {
                        Action::Shared(SharedOwner::Catalog(v))
                    }),
                    1 => optional(v.state.take(), |v| Action::Shared(SharedOwner::State(v))),
                    2 => optional(v.origin.take(), |v| Action::Shared(SharedOwner::State(v))),
                    3 => optional(v.queue.take(), |v| Action::Shared(SharedOwner::Queue(v))),
                    4 => optional(v.revisions.take(), |v| {
                        Action::Shared(SharedOwner::Journal(v))
                    }),
                    5 => optional(v.revision_modes.take(), |v| {
                        Action::Shared(SharedOwner::Modes(v))
                    }),
                    6 => node(v.last_receipt.take()),
                    7 => node(v.last_decision.take()),
                    8 => node(v.last_event_facts.take()),
                    9 => node(std::mem::take(&mut v.last_creator_recoveries)),
                    10 => node(std::mem::take(&mut v.last_corridor_restrictions)),
                    11 => node(std::mem::take(&mut v.redo)),
                    12 => fault(v.fault.take()),
                    13 => {
                        if let Some(child) = v.archives.pop() {
                            self.field -= 1;
                            Action::Child(Owner::Authority(AuthorityOwner::Inline(child)))
                        } else {
                            Action::Work
                        }
                    }
                    14 => optional(v.replay_base.take(), |v| {
                        Action::Child(Owner::Authority(AuthorityOwner::Boxed(v)))
                    }),
                    15 => optional(v.saved_present.take(), |v| {
                        Action::Child(Owner::Authority(AuthorityOwner::Boxed(v)))
                    }),
                    _ => Action::Done,
                }
            }
            Owner::Step(owner) => {
                let v = owner.value();
                match n {
                    // Source anchors are registered before any child alias release.
                    0 => optional(v.capture.take(), |v| Action::Child(Owner::Capture(v))),
                    1 => node(std::mem::take(&mut v.retirement)),
                    2 => node(v.copy.take()),
                    3 => node(v.hash.take()),
                    4 => node(v.candidate.take()),
                    5 => optional(v.creator.take(), |v| Action::Child(Owner::Creator(v))),
                    6 => node(v.scheduled.take()),
                    7 => node(v.conflict.take()),
                    8 => node(v.production.take()),
                    9 => node(v.observation.take()),
                    10 => node(v.observation_events.take()),
                    11 => node(v.diplomacy.take()),
                    12 => node(v.dispatch.take()),
                    13 => optional(v.completed_output.take(), |v| {
                        Action::Child(Owner::Completed(v))
                    }),
                    14 => fault(v.fault.take()),
                    // Preserve the optional-tail convention: absent owners do
                    // not introduce synthetic retirement work. If both exist,
                    // the original intent owner precedes revision preparation.
                    15 if v.intents.is_some() => node(v.intents.take()),
                    15 | 16 if v.applied_revisions.is_some() => node(v.applied_revisions.take()),
                    15..=17 if v.sealing.is_some() => node(v.sealing.take()),
                    15..=18 if v.sealed_receipts.is_some() => node(v.sealed_receipts.take()),
                    _ => Action::Done,
                }
            }
            Owner::Creator(v) => match n {
                0 => optional(v.queue.take(), |v| Action::Shared(SharedOwner::Queue(v))),
                1 => optional(v.catalog.take(), |v| {
                    Action::Shared(SharedOwner::Catalog(v))
                }),
                2 => node(std::mem::take(&mut v.state)),
                3 => node(std::mem::take(&mut v.events)),
                4 => node(std::mem::take(&mut v.locals)),
                5 => node(std::mem::take(&mut v.dormant)),
                6 => node(v.insertion.take()),
                7 => node(std::mem::take(&mut v.retirement)),
                8 => node(v.hull_parent.take()),
                9 => node(v.hull_output.take()),
                10 => node(v.append.take()),
                11 => node(v.colony_removal.take()),
                12 => node(std::mem::take(&mut v.colony_owners)),
                13 => node(v.recovery_storage.take()),
                14 => node(v.pending_recovery.take()),
                15 => node(v.diplomacy_work.take()),
                16 => node(v.cancellation_work.take()),
                17 => node(v.effect_work.take()),
                _ => Action::Done,
            },
            Owner::Prepared(v) => {
                if n == 0 {
                    optional(v.owners.take(), |v| Action::Child(Owner::Success(v)))
                } else {
                    Action::Done
                }
            }
            Owner::Success(v) => match n {
                0 => optional(v.capture.take(), |v| Action::Child(Owner::Capture(v))),
                1 => optional(v.state.take(), |v| Action::Shared(SharedOwner::State(v))),
                2 => optional(v.revisions.take(), |v| {
                    Action::Shared(SharedOwner::Journal(v))
                }),
                3 => optional(v.revision_modes.take(), |v| {
                    Action::Shared(SharedOwner::Modes(v))
                }),
                4 => optional(v.queue.take(), |v| Action::Shared(SharedOwner::Queue(v))),
                5 => node(v.receipt.take()),
                6 => node(v.decision.take()),
                7 => node(v.event_facts.take()),
                8 => node(std::mem::take(&mut v.creator_recoveries)),
                9 => node(std::mem::take(&mut v.corridor_restrictions)),
                _ => Action::Done,
            },
            Owner::Failure(v) => {
                if n == 0 {
                    optional(v.owners.take(), |v| Action::Child(Owner::FailureOwners(v)))
                } else {
                    Action::Done
                }
            }
            Owner::FailureOwners(v) => match n {
                0 => optional(v.capture.take(), |v| Action::Child(Owner::Capture(v))),
                1 => fault(Some(std::mem::replace(
                    &mut v.fault,
                    LivingDeterministicFaultV2::Invariant,
                ))),
                _ => Action::Done,
            },
            Owner::Retired(v) => match n {
                0 => optional(v._capture.take(), |v| Action::Child(Owner::Capture(v))),
                1 => optional(v._token.take(), |v| Action::Child(Owner::preparation(v))),
                2 => node(v._receipt.take()),
                3 => node(v._decision.take()),
                4 => node(v._event_facts.take()),
                5 => node(std::mem::take(&mut v._creator_recoveries)),
                6 => node(std::mem::take(&mut v._corridor_restrictions)),
                _ => Action::Done,
            },
            Owner::Rejection(v) => match n {
                0 => job_error(std::mem::replace(
                    &mut v.reason,
                    LivingStepJobErrorV2::Stale,
                )),
                1 => optional(v._preparation.take(), |v| {
                    Action::Child(Owner::preparation(v))
                }),
                _ => Action::Done,
            },
            Owner::Completed(v) => match n {
                0 => node(std::mem::take(&mut v.0)),
                1 => node(std::mem::take(&mut v.1)),
                2 => node(std::mem::replace(
                    &mut v.2,
                    LivingCompletedStepPhasesV2::DispatchComplete,
                )),
                _ => Action::Done,
            },
            Owner::Queue(v) => {
                if let Some(row) = v.pop() {
                    self.field -= 1;
                    Action::Child(Owner::Queued(row))
                } else {
                    Action::Done
                }
            }
            Owner::Queued(v) => match n {
                0 => optional(v.accepted.take(), |v| {
                    Action::Shared(SharedOwner::Accepted(v))
                }),
                1 => optional(v.revision.take(), |v| {
                    Action::Shared(SharedOwner::Revision(v))
                }),
                _ => Action::Done,
            },
            Owner::Fault(v) => {
                if n == 0 {
                    match std::mem::replace(v, LivingDeterministicFaultV2::Invariant) {
                        LivingDeterministicFaultV2::Replay {
                            reason,
                            accepted_sequence: _,
                        } => Action::Child(Owner::Reason(reason)),
                        LivingDeterministicFaultV2::ViewingHistory
                        | LivingDeterministicFaultV2::Invariant
                        | LivingDeterministicFaultV2::Capacity
                        | LivingDeterministicFaultV2::Arithmetic
                        | LivingDeterministicFaultV2::Validation(_)
                        | LivingDeterministicFaultV2::Wire(_)
                        | LivingDeterministicFaultV2::Receipt(_)
                        | LivingDeterministicFaultV2::Decision(_)
                        | LivingDeterministicFaultV2::EventFacts(_) => Action::Work,
                    }
                } else {
                    Action::Done
                }
            }
            Owner::Reason(v) => {
                if n == 0 {
                    match std::mem::replace(&mut **v, LivingCommandRejectionV2::Faulted) {
                        LivingCommandRejectionV2::Seal(error) => Action::Child(Owner::Fault(error)),
                        LivingCommandRejectionV2::StaleRevision {
                            expected: _,
                            found: _,
                        }
                        | LivingCommandRejectionV2::StaleTick {
                            expected: _,
                            found: _,
                        }
                        | LivingCommandRejectionV2::StalePendingSequence {
                            expected: _,
                            found: _,
                        }
                        | LivingCommandRejectionV2::ModeMismatch {
                            expected: _,
                            found: _,
                        }
                        | LivingCommandRejectionV2::PausedWithRunningQueue
                        | LivingCommandRejectionV2::QueueFull { limit: _ }
                        | LivingCommandRejectionV2::HistoryFull {
                            collection: _,
                            limit: _,
                        }
                        | LivingCommandRejectionV2::ViewingHistory
                        | LivingCommandRejectionV2::Faulted
                        | LivingCommandRejectionV2::Exhausted
                        | LivingCommandRejectionV2::Command(_)
                        | LivingCommandRejectionV2::UnsupportedOperation { operation: _ }
                        | LivingCommandRejectionV2::MissingTarget {
                            operation: _,
                            reference: _,
                        }
                        | LivingCommandRejectionV2::CascadeMismatch
                        | LivingCommandRejectionV2::ReferencedTarget
                        | LivingCommandRejectionV2::Projection(_) => Action::Work,
                    }
                } else {
                    Action::Done
                }
            }
            Owner::Replay(v) => match n {
                0 => node(std::mem::take(&mut v.candidate)),
                1 => node(std::mem::take(&mut v.events)),
                2 => node(std::mem::take(&mut v.records)),
                _ => Action::Done,
            },
        }
    }
}

enum SharedOwner {
    Queue(Arc<Vec<QueuedCommandV2>>),
    State(Arc<LivingGalaxyStateV2>),
    Catalog(Arc<ValidatedLivingCatalogPackV2>),
    Journal(Arc<Vec<LivingRevisionV2>>),
    Modes(Arc<Vec<LivingCommandModeV2>>),
    Accepted(Arc<LivingAcceptedCommandV2>),
    Revision(Arc<LivingRevisionV2>),
}
type Key = (u8, usize);
// Fixed-size inline ownership avoids uncounted allocation at handoff.
#[allow(clippy::large_enum_variant)]
enum Resolution {
    Payload(Action),
    Shared,
    Deferred(SharedOwner),
}
impl SharedOwner {
    fn key(&self) -> Key {
        match self {
            Self::Queue(v) => (0, Arc::as_ptr(v) as usize),
            Self::State(v) => (1, Arc::as_ptr(v) as usize),
            Self::Catalog(v) => (2, Arc::as_ptr(v) as usize),
            Self::Journal(v) => (3, Arc::as_ptr(v) as usize),
            Self::Modes(v) => (4, Arc::as_ptr(v) as usize),
            Self::Accepted(v) => (5, Arc::as_ptr(v) as usize),
            Self::Revision(v) => (6, Arc::as_ptr(v) as usize),
        }
    }
    fn resolve(self, policy: LivingStateRetirementPolicyV2) -> Resolution {
        macro_rules! resolve {
            ($value:expr,$variant:ident,$f:expr) => {
                match resolve_living_retirement_arc_v2($value, policy) {
                    LivingArcOwnerResolutionV2::Owned(v) => Resolution::Payload(($f)(v)),
                    LivingArcOwnerResolutionV2::SharedReleased => Resolution::Shared,
                    LivingArcOwnerResolutionV2::Deferred(v) => {
                        Resolution::Deferred(Self::$variant(v))
                    }
                }
            };
        }
        match self {
            Self::Queue(v) => resolve!(v, Queue, |v| Action::Child(Owner::Queue(v))),
            Self::State(v) => resolve!(v, State, node),
            Self::Catalog(v) => resolve!(v, Catalog, |v| Action::Node(Box::new(CatalogOwned {
                job: super::super::catalog::LivingCatalogRetirementJobV2::begin_owned(v),
                done: false
            }))),
            Self::Journal(v) => resolve!(v, Journal, node),
            Self::Modes(v) => resolve!(v, Modes, node),
            Self::Accepted(v) => resolve!(v, Accepted, node),
            Self::Revision(v) => resolve!(v, Revision, node),
        }
    }
}
struct CatalogOwned {
    job: super::super::catalog::LivingCatalogRetirementJobV2,
    done: bool,
}
impl RetirementPoll for CatalogOwned {
    fn retirement_step(&mut self) -> RetirementStep {
        if self.done {
            return RetirementStep::Done;
        }
        if self.job.poll(1).expect("valid budget") == LivingLifecycleProgressV2::Ready {
            self.done = true;
            match self.job.finish().expect("ready catalog") {
                super::super::catalog::LivingCatalogRetirementDispositionV2::Drained => {}
                _ => unreachable!("owned catalog"),
            }
        }
        RetirementStep::Work
    }
}

/// One admitted root. Constructors move ownership without scanning/allocating.
/// Retain the job through Ready; ordinary Drop of a partial job is not bounded.
#[must_use]
pub struct LivingPublicationRetirementJobV2 {
    root: Option<Owner>,
    admission_open: bool,
    close_requested: bool,
    archive_context: bool,
    recomposed: bool,
    reopen_requested: Option<LivingPublicationRetirementPolicyV2>,
    frames: LinkedList<Frame>,
    shells: LinkedList<Owner>,
    payload_shells: LinkedList<Owner>,
    groups_started: bool,
    registry_storage_released: u8,
    active: Option<Box<dyn RetirementPoll>>,
    unexpected: Option<LivingPhaseDeferredRetirementV2>,
    groups: BTreeMap<Key, SharedOwner>,
    blocked: BTreeMap<Key, SharedOwner>,
    blocked_queues: usize,
    policy: LivingPublicationRetirementPolicyV2,
    waiting: bool,
    ready: bool,
    last: usize,
    total: u64,
    shared: u64,
}
impl LivingPublicationRetirementJobV2 {
    pub(crate) fn begin_fault(value: LivingDeterministicFaultV2) -> Self {
        Self::begin(
            Owner::Fault(value),
            LivingPublicationRetirementPolicyV2::ReleaseHandle,
        )
    }
    fn begin(root: Owner, policy: LivingPublicationRetirementPolicyV2) -> Self {
        let mut job = Self::begin_open(policy);
        job.root = Some(root);
        job.admission_open = false;
        job.archive_context = false;
        job
    }
    /// One finite archive producer; roots share this existing census.
    pub(crate) fn begin_open(policy: LivingPublicationRetirementPolicyV2) -> Self {
        Self {
            root: None,
            admission_open: true,
            close_requested: false,
            archive_context: true,
            recomposed: false,
            reopen_requested: None,
            frames: LinkedList::new(),
            shells: LinkedList::new(),
            payload_shells: LinkedList::new(),
            groups_started: false,
            registry_storage_released: 0,
            active: None,
            unexpected: None,
            groups: BTreeMap::new(),
            blocked: BTreeMap::new(),
            blocked_queues: 0,
            policy,
            waiting: false,
            ready: false,
            last: 0,
            total: 0,
            shared: 0,
        }
    }
    /// True only when the producer may move its next exact root.
    pub(crate) fn admission_idle(&self) -> bool {
        self.admission_open
            && !self.close_requested
            && self.reopen_requested.is_none()
            && self.root.is_none()
            && self.frames.is_empty()
            && self.active.is_none()
    }
    /// Request one counted close transition after all finite holders are exposed.
    pub(crate) fn close_admission(&mut self) -> bool {
        if !self.admission_idle() {
            return false;
        }
        self.close_requested = true;
        true
    }
    #[allow(clippy::result_large_err)] // Refusal must return the exact inline owner without allocating.
    pub(crate) fn try_admit_authority(
        &mut self,
        value: LivingGalaxyAuthorityV2,
    ) -> Result<(), LivingGalaxyAuthorityV2> {
        if !self.admission_idle() {
            return Err(value);
        }
        self.root = Some(Owner::Authority(AuthorityOwner::Inline(value)));
        Ok(())
    }
    #[allow(dead_code)] // Explicit original-Box producer entry; covered by bridge tests.
    pub(crate) fn try_admit_boxed_authority(
        &mut self,
        value: Box<LivingGalaxyAuthorityV2>,
    ) -> Result<(), Box<LivingGalaxyAuthorityV2>> {
        if !self.admission_idle() {
            return Err(value);
        }
        self.root = Some(Owner::Authority(AuthorityOwner::Boxed(value)));
        Ok(())
    }
    #[allow(clippy::result_large_err)] // Refusal preserves the original inline boundary without allocation.
    pub(crate) fn try_admit_replay(
        &mut self,
        value: LivingReplayBoundaryV2,
    ) -> Result<(), LivingReplayBoundaryV2> {
        if !self.admission_idle() {
            return Err(value);
        }
        self.root = Some(Owner::Replay(value));
        Ok(())
    }
    pub(crate) fn try_admit_fault(
        &mut self,
        value: LivingDeterministicFaultV2,
    ) -> Result<(), LivingDeterministicFaultV2> {
        if !self.admission_idle() {
            return Err(value);
        }
        self.root = Some(Owner::Fault(value));
        Ok(())
    }
    /// Adopt remaining archive-only ReleaseHandle owners into one larger finite
    /// graph. Public Step/capture/token constructors cannot acquire this provenance.
    /// Already released handles are prior work, never recovered by this transition.
    pub(crate) fn request_archive_reopen(
        &mut self,
        policy: LivingPublicationRetirementPolicyV2,
    ) -> bool {
        if !self.archive_context
            || self.recomposed
            || self.reopen_requested.is_some()
            || self.policy != LivingPublicationRetirementPolicyV2::ReleaseHandle
            || self.groups_started
            || !self.groups.is_empty()
            || !self.blocked.is_empty()
            || self.unexpected.is_some()
            || self.waiting
        {
            return false;
        }
        self.reopen_requested = Some(policy);
        true
    }
    /// Work consumed by the last poll; invalid/terminal/blocked polls use zero.
    pub fn last_poll_visits(&self) -> usize {
        self.last
    }
    /// Total counted record/ownership/storage transitions.
    pub fn total_visits(&self) -> u64 {
        self.total
    }
    /// Number of shared-handle releases; their contents were not reclaimed here.
    pub fn shared_handle_releases(&self) -> u64 {
        self.shared
    }
    /// Resume after the caller changes external ownership. This moves map roots,
    /// allocates no groups and performs no scan. Repeated calls while active do nothing.
    pub fn retry_external_owners(&mut self) {
        if self.waiting {
            self.waiting = false;
            self.blocked_queues = 0;
            std::mem::swap(&mut self.groups, &mut self.blocked);
            if let Some(ticket) = self.unexpected.take() {
                self.active = Some(retirement_node(
                    ticket,
                    LivingStateRetirementPolicyV2::DeferUntilUnique,
                ));
            }
        }
    }
    /// Poll the single root with 1..=1024 actual visits, independently of faults.
    pub fn poll(
        &mut self,
        budget: usize,
    ) -> Result<LivingPublicationRetirementProgressV2, LivingStepJobErrorV2> {
        self.last = 0;
        if !(1..=1024).contains(&budget) {
            return Err(LivingStepJobErrorV2::Budget);
        }
        while self.last < budget
            && (!self.ready || self.reopen_requested.is_some())
            && !self.waiting
        {
            if !self.visit() {
                break;
            }
            self.last += 1;
            self.total += 1;
        }
        Ok(if self.ready {
            LivingPublicationRetirementProgressV2::Ready
        } else if self.waiting {
            LivingPublicationRetirementProgressV2::AwaitingExternalOwners
        } else {
            LivingPublicationRetirementProgressV2::Pending
        })
    }
    fn action(&mut self, action: Action) {
        match action {
            Action::Node(v) => {
                debug_assert!(self.active.is_none());
                self.active = Some(v)
            }
            Action::Child(owner) => self.frames.push_front(Frame { owner, field: 0 }),
            Action::Shared(v) => {
                if self.policy == LivingPublicationRetirementPolicyV2::DeferUntilUnique {
                    let key = v.key();
                    if let std::collections::btree_map::Entry::Vacant(entry) =
                        self.groups.entry(key)
                    {
                        entry.insert(v);
                    } else {
                        // The exact representative remains alive during this release.
                        self.resolution(v.resolve(LivingStateRetirementPolicyV2::ReleaseHandle));
                    }
                } else {
                    self.resolution(v.resolve(self.policy.internal()))
                }
            }
            Action::Work => {}
            Action::Done => unreachable!("frame completion handled separately"),
        }
    }
    fn resolution(&mut self, value: Resolution) {
        match value {
            Resolution::Payload(action) => self.action(action),
            Resolution::Shared => self.shared += 1,
            Resolution::Deferred(owner) => {
                let key = owner.key();
                if key.0 == 0 {
                    self.blocked_queues += 1
                }
                assert!(self.blocked.insert(key, owner).is_none(), "one exact group");
            }
        }
    }
    fn visit(&mut self) -> bool {
        if let Some(policy) = self.reopen_requested.take() {
            self.policy = policy;
            self.admission_open = true;
            self.close_requested = false;
            self.ready = false;
            self.recomposed = true;
            self.registry_storage_released = 0;
            return true;
        }
        if let Some(node) = &mut self.active {
            match node.retirement_step() {
                RetirementStep::Done => {
                    self.active.take();
                }
                RetirementStep::Work => {}
                RetirementStep::Deferred(ticket) => {
                    self.unexpected = Some(ticket);
                }
            }
            return true;
        }
        if let Some(owner) = self.root.take() {
            self.frames.push_front(Frame { owner, field: 0 });
            return true;
        }
        if let Some(frame) = self.frames.front_mut() {
            let action = frame.visit();
            if matches!(action, Action::Done) {
                let frame = self.frames.pop_front().expect("retained frame");
                if self.groups_started {
                    self.payload_shells.push_back(frame.owner);
                } else {
                    self.shells.push_back(frame.owner);
                }
            } else {
                self.action(action)
            }
            return true;
        }
        if self.admission_open {
            if self.close_requested {
                self.admission_open = false;
                self.close_requested = false;
                return true;
            }
            return false;
        }
        // Fixed scalar keys and fixed handles: standard BTree metadata work is
        // O(log groups), not a nested payload traversal. No populated Vec grows.
        if let Some((key, owner)) = self.groups.pop_first() {
            self.groups_started = true;
            if key.0 >= 5 && self.blocked_queues != 0 {
                assert!(self.blocked.insert(key, owner).is_none());
            } else {
                self.resolution(owner.resolve(self.policy.internal()));
            }
            return true;
        }
        if !self.blocked.is_empty() || self.unexpected.is_some() {
            self.waiting = true;
            return false;
        }
        // pop_first can leave an allocated empty BTree root. Release each
        // exhausted registry allocation in its own counted transition.
        if self.policy == LivingPublicationRetirementPolicyV2::DeferUntilUnique {
            match self.registry_storage_released {
                0 => {
                    self.groups = BTreeMap::new();
                    self.registry_storage_released = 1;
                    return true;
                }
                1 => {
                    self.blocked = BTreeMap::new();
                    self.registry_storage_released = 2;
                    return true;
                }
                _ => {}
            }
        }
        if let Some(shell) = self.payload_shells.front_mut() {
            if !shell.release_storage() {
                self.payload_shells.pop_front();
            }
            return true;
        }
        if let Some(shell) = self.shells.front_mut() {
            if !shell.release_storage() {
                self.shells.pop_front();
            }
            return true;
        }
        self.ready = true;
        false
    }
}

macro_rules! handoff {
    ($ty:ty,$f:expr) => {
        impl $ty {
            /// Move this exact owner into counted retirement. No payload is cloned.
            pub fn into_retirement(
                self,
                policy: LivingPublicationRetirementPolicyV2,
            ) -> LivingPublicationRetirementJobV2 {
                LivingPublicationRetirementJobV2::begin(($f)(self), policy)
            }
        }
    };
}
handoff!(LivingGalaxyAuthorityV2, |v| Owner::Authority(
    AuthorityOwner::Inline(v)
));
handoff!(LivingStepJobV2, |v| Owner::Step(StepOwner::Inline(v)));
handoff!(LivingBoundaryCaptureV2, Owner::Capture);
handoff!(LivingStepPreparationV2, Owner::preparation);
handoff!(LivingStepCommitRejectionV2, Owner::Rejection);
handoff!(LivingRetiredBoundaryV2, Owner::Retired);
impl LivingGalaxyAuthorityV2 {
    /// Preserve an existing authority Box through descendant/storage retirement.
    pub fn into_boxed_retirement(
        self: Box<Self>,
        policy: LivingPublicationRetirementPolicyV2,
    ) -> LivingPublicationRetirementJobV2 {
        LivingPublicationRetirementJobV2::begin(
            Owner::Authority(AuthorityOwner::Boxed(self)),
            policy,
        )
    }
}
impl LivingStepJobV2 {
    /// Preserve an existing executor Box through child retirement.
    pub fn into_boxed_retirement(
        self: Box<Self>,
        policy: LivingPublicationRetirementPolicyV2,
    ) -> LivingPublicationRetirementJobV2 {
        LivingPublicationRetirementJobV2::begin(Owner::Step(StepOwner::Boxed(self)), policy)
    }
}
impl LivingPreparedBoundaryV2 {
    /// Retire an unpublished success while retaining its original token Box.
    pub fn into_retirement(
        self: Box<Self>,
        policy: LivingPublicationRetirementPolicyV2,
    ) -> LivingPublicationRetirementJobV2 {
        LivingPublicationRetirementJobV2::begin(Owner::Prepared(self), policy)
    }
}
impl LivingPreparedStepFailureV2 {
    /// Retire an authentic failure independently of fault publication.
    pub fn into_retirement(
        self: Box<Self>,
        policy: LivingPublicationRetirementPolicyV2,
    ) -> LivingPublicationRetirementJobV2 {
        LivingPublicationRetirementJobV2::begin(Owner::Failure(self), policy)
    }
}
impl LivingReplayBoundaryV2 {
    #[allow(dead_code)] // Archive job handoff is the next released slice.
    pub(crate) fn into_retirement(
        self,
        policy: LivingPublicationRetirementPolicyV2,
    ) -> LivingPublicationRetirementJobV2 {
        LivingPublicationRetirementJobV2::begin(Owner::Replay(self), policy)
    }
}

// These registrations enumerate nested owners; scalar identities and bounded
// strings remain in their original row until all registered children finish.
macro_rules! leaves {($($ty:ty),* $(,)?)=>{$(impl<'a> RetirementFields<'a> for $ty {
 const LEAF:bool=true;
 fn retirement_child(&mut self,_:usize,_:LivingStateRetirementPolicyV2)->Option<Box<dyn RetirementPoll+'a>>{None}
})*}}
leaves!(
    LivingRevisionIdV2,
    LivingCommandModeV2,
    LivingCommandTargetV2,
    super::super::command::LivingDerivedRemovalV2,
    LivingCreatorRecoveryV2,
    super::super::receipt::LivingEventV2,
    LivingHistoryViewV2,
    [u16; 2]
);
macro_rules! fields {($ty:ty; $($field:ident),+)=>{impl<'a> RetirementFields<'a> for $ty {
 fn retirement_child(&mut self,index:usize,policy:LivingStateRetirementPolicyV2)->Option<Box<dyn RetirementPoll+'a>> {
 let mut n=0;$(if index==n{return Some(retirement_node(std::mem::take(&mut self.$field),policy))}n+=1;)+let _=n;None
 }}}}
impl<'a> RetirementFields<'a> for Box<LivingBoundarySealJobV2> {
    #[cfg(test)]
    fn creator_allocations(&self, out: &mut Vec<(usize, usize, Option<usize>, usize)>) {
        self.as_ref().creator_allocations(out);
    }
    fn retirement_child(
        &mut self,
        index: usize,
        policy: LivingStateRetirementPolicyV2,
    ) -> Option<Box<dyn RetirementPoll + 'a>> {
        self.as_mut().retirement_child(index, policy)
    }
}
impl<'a> RetirementFields<'a> for LivingBoundarySealJobV2 {
    #[cfg(test)]
    fn creator_allocations(&self, out: &mut Vec<(usize, usize, Option<usize>, usize)>) {
        self.candidate.creator_allocations(out);
        self.state.creator_allocations(out);
        self.pending.creator_allocations(out);
        self.applied.creator_allocations(out);
        self.validation.creator_allocations(out);
        self.hash.creator_allocations(out);
        self.ordinary.creator_allocations(out);
        self.receipt.creator_allocations(out);
        self.facts.creator_allocations(out);
        self.facts_receipt.creator_allocations(out);
        self.retirement.creator_allocations(out);
    }
    fn retirement_child(
        &mut self,
        index: usize,
        policy: LivingStateRetirementPolicyV2,
    ) -> Option<Box<dyn RetirementPoll + 'a>> {
        let Self {
            candidate,
            state,
            pending,
            applied,
            validation,
            hash,
            ordinary,
            receipt,
            facts,
            facts_receipt,
            retirement,
            digest: _,
            tick: _,
            stage: _,
        } = self;
        match index {
            0 => Some(retirement_node(std::mem::take(candidate), policy)),
            1 => Some(retirement_node(std::mem::take(state), policy)),
            2 => Some(retirement_node(std::mem::take(pending), policy)),
            3 => Some(retirement_node(std::mem::take(applied), policy)),
            4 => Some(retirement_node(std::mem::take(validation), policy)),
            5 => Some(retirement_node(std::mem::take(hash), policy)),
            6 => Some(retirement_node(std::mem::take(ordinary), policy)),
            7 => Some(retirement_node(std::mem::take(receipt), policy)),
            8 => Some(retirement_node(std::mem::take(retirement), policy)),
            9 => Some(retirement_node(facts.take(), policy)),
            10 => Some(retirement_node(facts_receipt.take(), policy)),
            _ => None,
        }
    }
}

fields!(LivingAppliedRevisionPreparationV2;storage,ordered,output,retirement);
impl<'a> RetirementFields<'a> for LivingSealedReceiptsV2 {
    #[cfg(test)]
    fn creator_allocations(&self, out: &mut Vec<(usize, usize, Option<usize>, usize)>) {
        self.ordinary.creator_allocations(out);
        self.facts.creator_allocations(out);
    }
    fn retirement_child(
        &mut self,
        index: usize,
        policy: LivingStateRetirementPolicyV2,
    ) -> Option<Box<dyn RetirementPoll + 'a>> {
        let Self { ordinary, facts } = self;
        // Fixed receipt headers have no nested owners beyond these vectors.
        match index {
            0 => Some(retirement_node(
                std::mem::take(&mut ordinary.applied_revisions),
                policy,
            )),
            1 => Some(retirement_node(
                std::mem::take(&mut ordinary.events),
                policy,
            )),
            2 => Some(retirement_node(std::mem::take(&mut facts.events), policy)),
            3 => Some(retirement_node(facts.omitted.take(), policy)),
            _ => None,
        }
    }
}
impl<'a> RetirementFields<'a> for LivingTickReceiptV2 {
    #[cfg(test)]
    fn creator_allocations(&self, out: &mut Vec<(usize, usize, Option<usize>, usize)>) {
        self.applied_revisions.creator_allocations(out);
        self.events.creator_allocations(out);
    }
    fn retirement_child(
        &mut self,
        index: usize,
        policy: LivingStateRetirementPolicyV2,
    ) -> Option<Box<dyn RetirementPoll + 'a>> {
        match index {
            0 => Some(retirement_node(
                std::mem::take(&mut self.applied_revisions),
                policy,
            )),
            1 => Some(retirement_node(std::mem::take(&mut self.events), policy)),
            _ => None,
        }
    }
}
fields!(super::super::decision::LivingDecisionReceiptV2;kind,intents);
impl<'a> RetirementFields<'a> for super::super::event_facts::LivingEventFactsReceiptV2 {
    #[cfg(test)]
    fn creator_allocations(&self, out: &mut Vec<(usize, usize, Option<usize>, usize)>) {
        self.events.creator_allocations(out);
        self.omitted.creator_allocations(out);
    }
    fn retirement_child(
        &mut self,
        index: usize,
        policy: LivingStateRetirementPolicyV2,
    ) -> Option<Box<dyn RetirementPoll + 'a>> {
        match index {
            0 => Some(retirement_node(std::mem::take(&mut self.events), policy)),
            1 => Some(retirement_node(self.omitted.take(), policy)),
            _ => None,
        }
    }
}
impl<'a> RetirementFields<'a> for super::super::event_facts::LivingEventFactsOmittedV2 {
    #[cfg(test)]
    fn creator_allocations(&self, out: &mut Vec<(usize, usize, Option<usize>, usize)>) {
        self.ranges.creator_allocations(out);
    }
    fn retirement_child(
        &mut self,
        index: usize,
        policy: LivingStateRetirementPolicyV2,
    ) -> Option<Box<dyn RetirementPoll + 'a>> {
        let Self { reason: _, ranges } = self;
        (index == 0).then(|| retirement_node(std::mem::take(ranges), policy))
    }
}
impl<'a> RetirementFields<'a> for super::super::event_facts::LivingExplainedEventV2 {
    #[cfg(test)]
    fn creator_allocations(&self, out: &mut Vec<(usize, usize, Option<usize>, usize)>) {
        self.facts.creator_allocations(out);
    }
    fn retirement_child(
        &mut self,
        i: usize,
        p: LivingStateRetirementPolicyV2,
    ) -> Option<Box<dyn RetirementPoll + 'a>> {
        self.facts.retirement_child(i, p)
    }
}
impl<'a> RetirementFields<'a> for LivingCommandV2 {
    fn retirement_child(
        &mut self,
        i: usize,
        p: LivingStateRetirementPolicyV2,
    ) -> Option<Box<dyn RetirementPoll + 'a>> {
        match self {
            Self::CreatorBatch { operations } => {
                (i == 0).then(|| retirement_node(std::mem::take(operations), p))
            }
        }
    }
}
impl<'a> RetirementFields<'a> for LivingCommandEnvelopeV2 {
    fn retirement_child(
        &mut self,
        i: usize,
        p: LivingStateRetirementPolicyV2,
    ) -> Option<Box<dyn RetirementPoll + 'a>> {
        self.command.retirement_child(i, p)
    }
}
impl<'a> RetirementFields<'a> for LivingAcceptedCommandV2 {
    fn retirement_child(
        &mut self,
        i: usize,
        p: LivingStateRetirementPolicyV2,
    ) -> Option<Box<dyn RetirementPoll + 'a>> {
        self.envelope.retirement_child(i, p)
    }
}
impl<'a> RetirementFields<'a> for LivingRevisionV2 {
    fn retirement_child(
        &mut self,
        i: usize,
        p: LivingStateRetirementPolicyV2,
    ) -> Option<Box<dyn RetirementPoll + 'a>> {
        self.command.retirement_child(i, p)
    }
}
impl<'a> RetirementFields<'a> for LivingRecordedRevisionV2 {
    fn retirement_child(
        &mut self,
        i: usize,
        p: LivingStateRetirementPolicyV2,
    ) -> Option<Box<dyn RetirementPoll + 'a>> {
        self.revision.retirement_child(i, p)
    }
}
impl<'a> RetirementFields<'a> for super::super::command::LivingCascadeDispositionV2 {
    fn retirement_child(
        &mut self,
        i: usize,
        p: LivingStateRetirementPolicyV2,
    ) -> Option<Box<dyn RetirementPoll + 'a>> {
        match self {
            Self::RemoveListed { dependents } => {
                (i == 0).then(|| retirement_node(std::mem::take(dependents), p))
            }
            Self::RemoveReviewed {
                dependents,
                derived,
            } => match i {
                0 => Some(retirement_node(std::mem::take(dependents), p)),
                1 => Some(retirement_node(std::mem::take(derived), p)),
                _ => None,
            },
            Self::RejectIfReferenced => None,
        }
    }
}
impl<'a> RetirementFields<'a> for LivingCreatorOperationV2 {
    fn retirement_child(
        &mut self,
        i: usize,
        p: LivingStateRetirementPolicyV2,
    ) -> Option<Box<dyn RetirementPoll + 'a>> {
        match self {
            Self::SetFleetOrder {
                fleet: _,
                kind: _,
                target_world: _,
                not_before_tick: _,
                lane_path,
            } => (i == 0).then(|| retirement_node(std::mem::take(lane_path), p)),
            Self::SplitFleet {
                local: _,
                source: _,
                hulls,
            } => (i == 0).then(|| retirement_node(std::mem::take(hulls), p)),
            Self::RemoveEntity { target: _, cascade }
            | Self::ScrapFacility { target: _, cascade } => cascade.retirement_child(i, p),
            Self::CreateSystem { local: _, name: _ }
            | Self::CreateStar {
                local: _,
                system: _,
                archetype: _,
                name: _,
            }
            | Self::CreateWorld {
                local: _,
                system: _,
                archetype: _,
                name: _,
                owner: _,
                inventory: _,
            }
            | Self::CreateLane {
                local: _,
                system_a: _,
                system_b: _,
                distance_units: _,
            }
            | Self::CreateCivilization {
                local: _,
                name: _,
                policy: _,
                status: _,
            }
            | Self::CreateDeposit {
                local: _,
                world: _,
                resource: _,
                remaining_units: _,
            }
            | Self::CreateFacility {
                local: _,
                world: _,
                definition: _,
                status: _,
                paid_alloy: _,
                hit_points: _,
                next_due: _,
            }
            | Self::CreateConstructionJob {
                local: _,
                world: _,
                owner: _,
                definition: _,
                accepted_tick: _,
                completion_tick: _,
                paid_alloy: _,
            }
            | Self::CreateHullJob {
                local: _,
                world: _,
                owner: _,
                definition: _,
                accepted_tick: _,
                completion_tick: _,
                paid_alloy: _,
                target_fleet: _,
            }
            | Self::CreateFleet {
                local: _,
                owner: _,
                world: _,
            }
            | Self::CreateHull {
                local: _,
                fleet: _,
                definition: _,
                hit_points: _,
                return_credit: _,
            }
            | Self::CreateRoute {
                local: _,
                kind: _,
                source_world: _,
                destination_world: _,
                resource: _,
                batch_size: _,
                cadence_ticks: _,
                source_reserve: _,
                source_owner: _,
                receiver_owner: _,
                next_due: _,
            }
            | Self::CreateShipment {
                local: _,
                dispatch_owner: _,
                intended_receiver: _,
                source_world: _,
                destination_world: _,
                resource: _,
                units: _,
                departure_tick: _,
                arrival_tick: _,
                disposition: _,
            }
            | Self::CreateHazard {
                local: _,
                definition: _,
                lane: _,
                start_tick: _,
                end_tick: _,
            }
            | Self::SetWorldInventory {
                world: _,
                inventory: _,
            }
            | Self::SetWorldOwner { world: _, owner: _ }
            | Self::CreateColony { world: _, owner: _ }
            | Self::RemoveColony { world: _ }
            | Self::SetDepositRemaining {
                deposit: _,
                remaining_units: _,
            }
            | Self::SetCivilizationPolicy {
                civilization: _,
                policy: _,
            }
            | Self::SetRelationBase {
                from: _,
                to: _,
                base_disposition: _,
            }
            | Self::SetAgreement {
                kind: _,
                participant_a: _,
                participant_b: _,
                start_tick: _,
                end_tick: _,
            }
            | Self::RemoveAgreement {
                kind: _,
                participant_a: _,
                participant_b: _,
            }
            | Self::ForceWar {
                participant_a: _,
                participant_b: _,
                declarer: _,
                start_tick: _,
                end_tick: _,
            }
            | Self::ForcePeace {
                participant_a: _,
                participant_b: _,
            }
            | Self::SetShipmentDisposition {
                shipment: _,
                disposition: _,
            }
            | Self::EditSystem { target: _, name: _ }
            | Self::EditStar {
                target: _,
                system: _,
                archetype: _,
                name: _,
            }
            | Self::EditWorld {
                target: _,
                system: _,
                archetype: _,
                name: _,
            }
            | Self::EditLane {
                target: _,
                system_a: _,
                system_b: _,
                distance_units: _,
            }
            | Self::EditDeposit {
                target: _,
                resource: _,
                remaining_units: _,
            }
            | Self::EditCivilization {
                target: _,
                name: _,
                policy: _,
            }
            | Self::SetFacilityEnabled {
                target: _,
                enabled: _,
            }
            | Self::CancelConstructionJob { target: _ }
            | Self::CancelHullJob { target: _ }
            | Self::EditFleet {
                target: _,
                owner: _,
                world: _,
            }
            | Self::EditHull {
                target: _,
                definition: _,
                hit_points: _,
                return_credit: _,
            }
            | Self::MergeFleets {
                target: _,
                source: _,
            }
            | Self::EditRoute {
                target: _,
                kind: _,
                source_world: _,
                destination_world: _,
                resource: _,
                batch_size: _,
                cadence_ticks: _,
                source_reserve: _,
                source_owner: _,
                receiver_owner: _,
            }
            | Self::SetRouteSuspended {
                target: _,
                suspended: _,
            }
            | Self::EditHazard {
                target: _,
                lane: _,
                definition: _,
                start_tick: _,
                end_tick: _,
            }
            | Self::CancelHazard { target: _ } => None,
        }
    }
}
impl<'a> RetirementFields<'a> for LivingCompletedStepPhasesV2 {
    fn retirement_child(
        &mut self,
        i: usize,
        p: LivingStateRetirementPolicyV2,
    ) -> Option<Box<dyn RetirementPoll + 'a>> {
        match self {
            Self::Scheduled { arrivals } => {
                (i == 0).then(|| retirement_node(std::mem::take(arrivals), p))
            }
            Self::ObservationsComplete
            | Self::DiplomacyComplete
            | Self::IntentsComplete
            | Self::DispatchComplete => None,
        }
    }
}
impl<'a> RetirementFields<'a> for QueuedDiplomacyFilter {
    #[cfg(test)]
    fn creator_allocations(&self, out: &mut Vec<(usize, usize, Option<usize>, usize)>) {
        match self {
            Self::Agreement(job) => job.creator_allocations(out),
            Self::War(job) => job.creator_allocations(out),
            Self::Occupation(job) => job.creator_allocations(out),
        }
    }
    fn retirement_child(
        &mut self,
        i: usize,
        p: LivingStateRetirementPolicyV2,
    ) -> Option<Box<dyn RetirementPoll + 'a>> {
        match self {
            Self::Agreement(job) => job.retirement_child(i, p),
            Self::War(job) => job.retirement_child(i, p),
            Self::Occupation(job) => job.retirement_child(i, p),
        }
    }
}
impl<'a> RetirementFields<'a> for QueuedDiplomacyWork {
    #[cfg(test)]
    fn creator_allocations(&self, out: &mut Vec<(usize, usize, Option<usize>, usize)>) {
        self.plan.creator_allocations(out);
        self.pending_plan.creator_allocations(out);
        self.occupation.creator_allocations(out);
        self.filter.creator_allocations(out);
        self.path_storage.creator_allocations(out);
        self.pending_path.creator_allocations(out);
        self.order.creator_allocations(out);
        self.retired_order.creator_allocations(out);
    }
    fn retirement_child(
        &mut self,
        i: usize,
        p: LivingStateRetirementPolicyV2,
    ) -> Option<Box<dyn RetirementPoll + 'a>> {
        let Self {
            pair: _,
            plan,
            pending_plan,
            occupation,
            filter,
            fleet: _,
            fleet_index: _,
            path_storage,
            pending_path,
            order,
            retired_order,
        } = self;
        match i {
            0 => Some(retirement_node(std::mem::take(plan), p)),
            1 => Some(retirement_node(pending_plan.take(), p)),
            2 => Some(retirement_node(occupation.take(), p)),
            3 => Some(retirement_node(filter.take(), p)),
            4 => Some(retirement_node(path_storage.take(), p)),
            5 => Some(retirement_node(pending_path.take(), p)),
            6 => Some(retirement_node(order.take(), p)),
            7 => Some(retirement_node(retired_order.take(), p)),
            _ => None,
        }
    }
}

impl<'a> RetirementFields<'a> for QueuedCancellationFilter {
    #[cfg(test)]
    fn creator_allocations(&self, out: &mut Vec<(usize, usize, Option<usize>, usize)>) {
        match self {
            Self::Construction(job) => job.creator_allocations(out),
            Self::Hull(job) => job.creator_allocations(out),
            Self::Hazard(job) => job.creator_allocations(out),
        }
    }
    fn retirement_child(
        &mut self,
        i: usize,
        p: LivingStateRetirementPolicyV2,
    ) -> Option<Box<dyn RetirementPoll + 'a>> {
        match self {
            Self::Construction(job) => job.retirement_child(i, p),
            Self::Hull(job) => job.retirement_child(i, p),
            Self::Hazard(job) => job.retirement_child(i, p),
        }
    }
}
impl<'a> RetirementFields<'a> for QueuedCancellationWork {
    #[cfg(test)]
    fn creator_allocations(&self, out: &mut Vec<(usize, usize, Option<usize>, usize)>) {
        self.plan.creator_allocations(out);
        self.filter.creator_allocations(out);
        self.route.creator_allocations(out);
    }
    fn retirement_child(
        &mut self,
        i: usize,
        p: LivingStateRetirementPolicyV2,
    ) -> Option<Box<dyn RetirementPoll + 'a>> {
        let Self {
            kind: _,
            position: _,
            refund: _,
            plan,
            filter,
            route,
        } = self;
        match i {
            0 => Some(retirement_node(std::mem::take(plan), p)),
            1 => Some(retirement_node(filter.take(), p)),
            2 => Some(retirement_node(route.take(), p)),
            _ => None,
        }
    }
}
impl<'a> RetirementFields<'a> for QueuedCreatorEffects {
    #[cfg(test)]
    fn creator_allocations(&self, out: &mut Vec<(usize, usize, Option<usize>, usize)>) {
        self.values.creator_allocations(out);
        self.storage.creator_allocations(out);
        self.pending.creator_allocations(out);
        self.input.creator_allocations(out);
        self.suffix.creator_allocations(out);
        self.suffix_input.creator_allocations(out);
    }
    fn retirement_child(
        &mut self,
        i: usize,
        p: LivingStateRetirementPolicyV2,
    ) -> Option<Box<dyn RetirementPoll + 'a>> {
        let Self {
            values,
            storage,
            pending,
            input,
            suffix,
            suffix_input,
            resume: _,
        } = self;
        match i {
            0 => Some(retirement_node(std::mem::take(values), p)),
            1 => Some(retirement_node(storage.take(), p)),
            2 => Some(retirement_node(pending.take(), p)),
            3 => Some(retirement_node(input.take(), p)),
            4 => Some(retirement_node(std::mem::take(suffix), p)),
            5 => Some(retirement_node(suffix_input.take(), p)),
            _ => None,
        }
    }
}

impl<'a> RetirementFields<'a> for QueuedRemovalWork {
    #[cfg(test)]
    fn creator_allocations(&self, out: &mut Vec<(usize, usize, Option<usize>, usize)>) {
        self.witnesses.creator_allocations(out);
        self.removed.creator_allocations(out);
        self.dependents.creator_allocations(out);
        self.declared.creator_allocations(out);
        self.pending.creator_allocations(out);
        self.storage.creator_allocations(out);
        self.plan.creator_allocations(out);
        self.filter.creator_allocations(out);
        self.retirement.creator_allocations(out);
    }
    fn retirement_child(
        &mut self,
        i: usize,
        p: LivingStateRetirementPolicyV2,
    ) -> Option<Box<dyn RetirementPoll + 'a>> {
        let Self {
            stage: _,
            target: _,
            listed: _,
            reviewed,
            witnesses,
            registry: _,
            row: _,
            hull: _,
            reference: _,
            grew: _,
            legacy: _,
            input_len: _,
            index: _,
            nested_fleet: _,
            nested: _,
            fault: _,
            removed,
            dependents,
            declared,
            pending,
            storage,
            plan,
            filter,
            retirement,
        } = self;
        match i {
            0 => Some(retirement_node(std::mem::take(removed), p)),
            1 => Some(retirement_node(std::mem::take(dependents), p)),
            2 => Some(retirement_node(std::mem::take(declared), p)),
            3 => Some(retirement_node(pending.take(), p)),
            4 => Some(retirement_node(storage.take(), p)),
            5 => Some(retirement_node(std::mem::take(plan), p)),
            6 => Some(retirement_node(filter.take(), p)),
            7 => Some(retirement_node(std::mem::take(retirement), p)),
            8 if *reviewed => Some(retirement_node(std::mem::take(witnesses), p)),
            _ => None,
        }
    }
}
impl<'a> RetirementFields<'a> for QueuedRemovalFilter {
    #[cfg(test)]
    fn creator_allocations(&self, out: &mut Vec<(usize, usize, Option<usize>, usize)>) {
        match self {
            Self::System(job) => job.creator_allocations(out),
            Self::Star(job) => job.creator_allocations(out),
            Self::World(job) => job.creator_allocations(out),
            Self::Lane(job) => job.creator_allocations(out),
            Self::Civilization(job) => job.creator_allocations(out),
            Self::Deposit(job) => job.creator_allocations(out),
            Self::Facility(job) => job.creator_allocations(out),
            Self::ConstructionJob(job) => job.creator_allocations(out),
            Self::HullJob(job) => job.creator_allocations(out),
            Self::Fleet(job) => job.creator_allocations(out),
            Self::Route(job) => job.creator_allocations(out),
            Self::Shipment(job) => job.creator_allocations(out),
            Self::Hazard(job) => job.creator_allocations(out),
            Self::Colony(job) => job.creator_allocations(out),
            Self::Observation(job) => job.creator_allocations(out),
            Self::Relation(job) => job.creator_allocations(out),
            Self::Agreement(job) => job.creator_allocations(out),
            Self::War(job) => job.creator_allocations(out),
            Self::SettlementClaim(job) => job.creator_allocations(out),
            Self::Occupation(job) => job.creator_allocations(out),
            Self::Hull(job) => job.creator_allocations(out),
        }
    }
    fn retirement_child(
        &mut self,
        i: usize,
        p: LivingStateRetirementPolicyV2,
    ) -> Option<Box<dyn RetirementPoll + 'a>> {
        match self {
            Self::System(job) => job.retirement_child(i, p),
            Self::Star(job) => job.retirement_child(i, p),
            Self::World(job) => job.retirement_child(i, p),
            Self::Lane(job) => job.retirement_child(i, p),
            Self::Civilization(job) => job.retirement_child(i, p),
            Self::Deposit(job) => job.retirement_child(i, p),
            Self::Facility(job) => job.retirement_child(i, p),
            Self::ConstructionJob(job) => job.retirement_child(i, p),
            Self::HullJob(job) => job.retirement_child(i, p),
            Self::Fleet(job) => job.retirement_child(i, p),
            Self::Route(job) => job.retirement_child(i, p),
            Self::Shipment(job) => job.retirement_child(i, p),
            Self::Hazard(job) => job.retirement_child(i, p),
            Self::Colony(job) => job.retirement_child(i, p),
            Self::Observation(job) => job.retirement_child(i, p),
            Self::Relation(job) => job.retirement_child(i, p),
            Self::Agreement(job) => job.retirement_child(i, p),
            Self::War(job) => job.retirement_child(i, p),
            Self::SettlementClaim(job) => job.retirement_child(i, p),
            Self::Occupation(job) => job.retirement_child(i, p),
            Self::Hull(job) => job.retirement_child(i, p),
        }
    }
}

impl<'a> RetirementFields<'a> for QueuedFleetStructuralWork {
    #[cfg(test)]
    fn creator_allocations(&self, out: &mut Vec<(usize, usize, Option<usize>, usize)>) {
        self.selected.creator_allocations(out);
        self.pending_id.creator_allocations(out);
        self.selection_storage.creator_allocations(out);
        self.source_parent.creator_allocations(out);
        self.target_parent.creator_allocations(out);
        self.source_hulls.creator_allocations(out);
        self.target_hulls.creator_allocations(out);
        self.pending_source.creator_allocations(out);
        self.pending_target.creator_allocations(out);
        self.kept.creator_allocations(out);
        self.moved.creator_allocations(out);
        self.hull_storage.creator_allocations(out);
        self.source_order.creator_allocations(out);
        self.target_order.creator_allocations(out);
        self.plan.creator_allocations(out);
        self.filter.creator_allocations(out);
        self.replacement.creator_allocations(out);
        self.retirement.creator_allocations(out);
    }
    fn retirement_child(
        &mut self,
        i: usize,
        p: LivingStateRetirementPolicyV2,
    ) -> Option<Box<dyn RetirementPoll + 'a>> {
        let Self {
            stage: _,
            split: _,
            source: _,
            target: _,
            source_index: _,
            target_index: _,
            source_count: _,
            target_count: _,
            selected_index: _,
            fault: _,
            selected,
            pending_id,
            selection_storage,
            source_parent,
            target_parent,
            source_hulls,
            target_hulls,
            pending_source,
            pending_target,
            kept,
            moved,
            hull_storage,
            source_order,
            target_order,
            plan,
            filter,
            replacement,
            retirement,
        } = self;
        match i {
            0 => Some(retirement_node(std::mem::take(selected), p)),
            1 => Some(retirement_node(pending_id.take(), p)),
            2 => Some(retirement_node(selection_storage.take(), p)),
            3 => Some(retirement_node(source_parent.take(), p)),
            4 => Some(retirement_node(target_parent.take(), p)),
            5 => Some(retirement_node(source_hulls.take(), p)),
            6 => Some(retirement_node(target_hulls.take(), p)),
            7 => Some(retirement_node(pending_source.take(), p)),
            8 => Some(retirement_node(pending_target.take(), p)),
            9 => Some(retirement_node(std::mem::take(kept), p)),
            10 => Some(retirement_node(std::mem::take(moved), p)),
            11 => Some(retirement_node(hull_storage.take(), p)),
            12 => Some(retirement_node(source_order.take(), p)),
            13 => Some(retirement_node(target_order.take(), p)),
            14 => Some(retirement_node(std::mem::take(plan), p)),
            15 => Some(retirement_node(filter.take(), p)),
            16 => Some(retirement_node(replacement.take(), p)),
            17 => Some(retirement_node(std::mem::take(retirement), p)),
            _ => None,
        }
    }
}

impl<'a> RetirementFields<'a> for QueuedCreatorInsertion {
    #[cfg(test)]
    fn creator_allocations(&self, out: &mut Vec<(usize, usize, Option<usize>, usize)>) {
        match self {
            Self::System(job) => job.creator_allocations(out),
            Self::Star(job) => job.creator_allocations(out),
            Self::World(job) => job.creator_allocations(out),
            Self::Lane(job) => job.creator_allocations(out),
            Self::Civilization(job) => job.creator_allocations(out),
            Self::Deposit(job) => job.creator_allocations(out),
            Self::Facility(job) => job.creator_allocations(out),
            Self::ConstructionJob(job) => job.creator_allocations(out),
            Self::HullJob(job) => job.creator_allocations(out),
            Self::Removal(job) => job.creator_allocations(out),
            Self::FleetStructural(job) => job.creator_allocations(out),
            Self::Fleet(job) => job.creator_allocations(out),
            Self::Hull(job) => job.creator_allocations(out),
            Self::Route(job) => job.creator_allocations(out),
            Self::Shipment(job) => job.creator_allocations(out),
            Self::Hazard(job) => job.creator_allocations(out),
            Self::Relation(job) => job.creator_allocations(out),
            Self::Agreement(job) => job.creator_allocations(out),
            Self::War(job) => job.creator_allocations(out),
            Self::Colony(job) => job.creator_allocations(out),
        }
    }

    fn retirement_child(
        &mut self,
        i: usize,
        p: LivingStateRetirementPolicyV2,
    ) -> Option<Box<dyn RetirementPoll + 'a>> {
        match self {
            Self::System(v) => v.retirement_child(i, p),
            Self::Star(v) => v.retirement_child(i, p),
            Self::World(v) => v.retirement_child(i, p),
            Self::Lane(v) => v.retirement_child(i, p),
            Self::Civilization(v) => v.retirement_child(i, p),
            Self::Deposit(v) => v.retirement_child(i, p),
            Self::Facility(v) => v.retirement_child(i, p),
            Self::ConstructionJob(v) => v.retirement_child(i, p),
            Self::HullJob(v) => v.retirement_child(i, p),
            Self::Removal(v) => v.retirement_child(i, p),
            Self::FleetStructural(v) => v.retirement_child(i, p),
            Self::Fleet(v) => v.retirement_child(i, p),
            Self::Hull(v) => v.retirement_child(i, p),
            Self::Route(v) => v.retirement_child(i, p),
            Self::Shipment(v) => v.retirement_child(i, p),
            Self::Hazard(v) => v.retirement_child(i, p),
            Self::Relation(v) => v.retirement_child(i, p),
            Self::Agreement(v) => v.retirement_child(i, p),
            Self::War(v) => v.retirement_child(i, p),
            Self::Colony(v) => v.retirement_child(i, p),
        }
    }
}

#[cfg(test)]
fn observe_creator(
    owner: &LivingQueuedCreatorJobV2,
    out: &mut Vec<(usize, usize, Option<usize>, usize)>,
) {
    owner.state.creator_allocations(out);
    owner.insertion.creator_allocations(out);
    owner.diplomacy_work.creator_allocations(out);
    owner.cancellation_work.creator_allocations(out);
    owner.effect_work.creator_allocations(out);
    owner.hull_parent.creator_allocations(out);
    owner.hull_output.creator_allocations(out);
    owner.colony_removal.creator_allocations(out);
    owner.recovery_storage.creator_allocations(out);
    owner.events.creator_allocations(out);
    owner.append.creator_allocations(out);
    owner.pending_recovery.creator_allocations(out);
    owner.retirement.creator_allocations(out);
}
#[cfg(test)]
fn observe_creator_owner(owner: &Owner, out: &mut Vec<(usize, usize, Option<usize>, usize)>) {
    match owner {
        Owner::Creator(owner) => observe_creator(owner, out),
        Owner::Step(step) => {
            let step = match step {
                StepOwner::Inline(step) => step,
                StepOwner::Boxed(step) => step,
            };
            if let Some(creator) = &step.creator {
                observe_creator(creator, out);
            }
            step.retirement.creator_allocations(out);
        }
        _ => {}
    }
}
impl RetirementPoll for LivingPublicationRetirementJobV2 {
    #[cfg(test)]
    fn creator_allocations(&self, out: &mut Vec<(usize, usize, Option<usize>, usize)>) {
        if let Some(root) = &self.root {
            observe_creator_owner(root, out);
        }
        for frame in &self.frames {
            observe_creator_owner(&frame.owner, out);
        }
        for shell in &self.shells {
            observe_creator_owner(shell, out);
        }
        for shell in &self.payload_shells {
            observe_creator_owner(shell, out);
        }
        if let Some(active) = &self.active {
            active.creator_allocations(out);
        }
    }

    fn retirement_step(&mut self) -> RetirementStep {
        match self.poll(1).expect("valid budget") {
            LivingPublicationRetirementProgressV2::Ready if self.last == 0 => RetirementStep::Done,
            LivingPublicationRetirementProgressV2::AwaitingExternalOwners => {
                unreachable!("release-only child")
            }
            _ => RetirementStep::Work,
        }
    }
}
macro_rules! custom_owner {
    ($ty:ty,$variant:ident) => {
        impl<'a> RetirementFields<'a> for $ty {
            const CUSTOM_NODE: bool = true;
            fn retirement_child(
                &mut self,
                _: usize,
                _: LivingStateRetirementPolicyV2,
            ) -> Option<Box<dyn RetirementPoll + 'a>> {
                unreachable!("custom root")
            }
            fn retirement_node(
                self,
                _: LivingStateRetirementPolicyV2,
            ) -> Box<dyn RetirementPoll + 'a> {
                Box::new(LivingPublicationRetirementJobV2::begin(
                    Owner::$variant(self),
                    LivingPublicationRetirementPolicyV2::ReleaseHandle,
                ))
            }
        }
    };
}
custom_owner!(Box<LivingQueuedCreatorJobV2>, Creator);
custom_owner!(
    Box<(
        LivingGalaxyStateV2,
        LivingPendingEventsV2,
        LivingCompletedStepPhasesV2
    )>,
    Completed
);

// Adding a field requires an explicit ownership classification here and in the
// cursor above. These exhaustive patterns cost no runtime work.
macro_rules! audited_fields {($ty:path; $($field:ident),* $(,)?) => {
 const _:fn(&$ty)=|$ty{$($field:_,)*}|{};
}}
audited_fields!(QueuedCommandV2;accepted,revision);
audited_fields!(LivingBoundaryCaptureV2;state,catalog,queue,revisions,revision_modes,genesis_seed,branch,committed_revision,accepted_sequence_high_water,branch_sequence_high_water);
audited_fields!(LivingReplayBoundaryV2;candidate,events,records,tick,parent);
audited_fields!(LivingCreatorRecoveryV2;revision,actor,world,receipt_digest);
audited_fields!(LivingPreparedBoundaryV2;owners);
audited_fields!(LivingPreparedBoundaryOwnersV2;capture,state,revisions,revision_modes,queue,committed_revision,receipt,decision,event_facts,creator_recoveries,corridor_restrictions);
audited_fields!(LivingPreparedStepFailureV2;owners);
audited_fields!(LivingPreparedFailureOwnersV2;capture,fault);
audited_fields!(LivingStepCommitRejectionV2;reason,_preparation);
audited_fields!(LivingRetiredBoundaryV2;_capture,_token,_receipt,_decision,_event_facts,_creator_recoveries,_corridor_restrictions);
audited_fields!(LivingAppliedRevisionPreparationV2;storage,ordered,output,retirement,count,index,ready);
audited_fields!(LivingBoundarySealJobV2;candidate,state,pending,applied,validation,hash,ordinary,receipt,facts,facts_receipt,retirement,digest,tick,stage);
audited_fields!(LivingSealedReceiptsV2;ordinary,facts);
audited_fields!(LivingStepJobV2;sealing,sealed_receipts,applied_revisions,capture,copy,hash,candidate,source_digest,creator,scheduled,conflict,production,observation,observation_events,diplomacy,intents,dispatch,completed_output,preparation_capacity,production_fault,production_handoff,fault,cancelled,delivered,retirement,last_poll_visits,total_visits);
audited_fields!(LivingQueuedCreatorJobV2;state,queue,catalog,events,stage,revision_index,operation_index,index,locals,dormant,insertion,diplomacy_work,cancellation_work,effect_work,colony_operation,colony_create,colony_removal,colony_changed,colony_owners,recovery_storage,pending_recovery,hull_parent,hull_output,hull_index,hull_id,append,retirement,last_poll_visits,delivered);
audited_fields!(LivingGalaxyAuthorityV2;catalog,genesis_seed,generator,branch,state,committed_revision,revisions,revision_modes,queue,accepted_sequence_high_water,fault,origin,root_branch,branch_sequence_high_water,replay_branch_sequence,archives,replay_base,saved_present,redo,last_receipt,last_decision,last_event_facts,last_creator_recoveries,last_corridor_restrictions);
#[cfg(test)]
mod tests {
    use super::*;
    use crate::living::step_job::LivingPhaseRetirementJobV2;
    use crate::living::*;
    const MIXED: &[usize] = &[1, 64, 3, 1024, 2, 9];
    fn name(s: &str) -> LivingNameV2 {
        LivingNameV2::new(s).unwrap()
    }
    fn slug(s: &str) -> LivingSlugV2 {
        LivingSlugV2::new(s).unwrap()
    }
    fn existing(b: u8) -> LivingCommandTargetV2 {
        LivingCommandTargetV2::Existing {
            id: LivingEntityIdV2([b; 16]),
        }
    }
    fn inventory() -> LivingInventoryV2 {
        LivingInventoryV2 {
            energy: 4,
            ore: 5,
            alloy: 6,
        }
    }
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
    fn every_operation_template() -> Vec<LivingCreatorOperationV2> {
        let mut templates = vec![
            LivingCreatorOperationV2::CreateSystem {
                local: 0,
                name: name("Vale"),
            },
            LivingCreatorOperationV2::CreateStar {
                local: 0,
                system: existing(0x01),
                archetype: slug("main_sequence"),
                name: name("Vale Prime"),
            },
            LivingCreatorOperationV2::CreateWorld {
                local: 0,
                system: existing(0x01),
                archetype: slug("terrestrial"),
                name: name("Vale II"),
                owner: Some(existing(0x02)),
                inventory: inventory(),
            },
            LivingCreatorOperationV2::CreateLane {
                local: 0,
                system_a: existing(0x01),
                system_b: existing(0x02),
                distance_units: 101,
            },
            LivingCreatorOperationV2::CreateCivilization {
                local: 0,
                name: name("Concord"),
                policy: LivingPolicyV2(slug("balanced")),
                status: LivingCivilizationStatusV2::Active,
            },
            LivingCreatorOperationV2::CreateDeposit {
                local: 0,
                world: existing(0x01),
                resource: LivingResourceV2::Ore,
                remaining_units: 500,
            },
            LivingCreatorOperationV2::CreateFacility {
                local: 0,
                world: existing(0x01),
                definition: slug("refinery"),
                status: LivingFacilityStatusV2::Operational,
                paid_alloy: 20,
                hit_points: 100,
                next_due: Some(LivingTickV2(9)),
            },
            LivingCreatorOperationV2::CreateConstructionJob {
                local: 0,
                world: existing(0x01),
                owner: existing(0x02),
                definition: slug("refinery"),
                accepted_tick: LivingTickV2(0),
                completion_tick: LivingTickV2(100),
                paid_alloy: 20,
            },
            LivingCreatorOperationV2::CreateHullJob {
                local: 0,
                world: existing(0x01),
                owner: existing(0x02),
                definition: slug("escort"),
                accepted_tick: LivingTickV2(0),
                completion_tick: LivingTickV2(100),
                paid_alloy: 20,
                target_fleet: Some(existing(0x03)),
            },
            LivingCreatorOperationV2::CreateFleet {
                local: 0,
                owner: existing(0x01),
                world: existing(0x02),
            },
            LivingCreatorOperationV2::CreateHull {
                local: 0,
                fleet: existing(0x01),
                definition: slug("escort"),
                hit_points: 50,
                return_credit: false,
            },
            LivingCreatorOperationV2::CreateRoute {
                local: 0,
                kind: LivingRouteKindV2::Internal,
                source_world: existing(0x01),
                destination_world: existing(0x02),
                resource: LivingResourceV2::Ore,
                batch_size: 10,
                cadence_ticks: 5,
                source_reserve: 2,
                source_owner: existing(0x03),
                receiver_owner: existing(0x04),
                next_due: LivingTickV2(12),
            },
            LivingCreatorOperationV2::CreateShipment {
                local: 0,
                dispatch_owner: existing(0x01),
                intended_receiver: existing(0x02),
                source_world: existing(0x03),
                destination_world: existing(0x04),
                resource: LivingResourceV2::Alloy,
                units: 5,
                departure_tick: LivingTickV2(1),
                arrival_tick: LivingTickV2(9),
                disposition: LivingShipmentDispositionV2::Outbound,
            },
            LivingCreatorOperationV2::CreateHazard {
                local: 0,
                definition: slug("ion_storm"),
                lane: existing(0x01),
                start_tick: LivingTickV2(3),
                end_tick: LivingTickV2(11),
            },
            LivingCreatorOperationV2::SetWorldInventory {
                world: existing(0x01),
                inventory: inventory(),
            },
            LivingCreatorOperationV2::SetWorldOwner {
                world: existing(0x01),
                owner: Some(existing(0x02)),
            },
            LivingCreatorOperationV2::CreateColony {
                world: existing(0x01),
                owner: existing(0x02),
            },
            LivingCreatorOperationV2::RemoveColony {
                world: existing(0x01),
            },
            LivingCreatorOperationV2::SetDepositRemaining {
                deposit: existing(0x01),
                remaining_units: 7,
            },
            LivingCreatorOperationV2::SetCivilizationPolicy {
                civilization: existing(0x01),
                policy: LivingPolicyV2(slug("balanced")),
            },
            LivingCreatorOperationV2::SetRelationBase {
                from: existing(0x01),
                to: existing(0x02),
                base_disposition: 10,
            },
            LivingCreatorOperationV2::SetAgreement {
                kind: LivingAgreementKindV2::Trade,
                participant_a: existing(0x01),
                participant_b: existing(0x02),
                start_tick: LivingTickV2(0),
                end_tick: LivingTickV2(600),
            },
            LivingCreatorOperationV2::RemoveAgreement {
                kind: LivingAgreementKindV2::Trade,
                participant_a: existing(0x01),
                participant_b: existing(0x02),
            },
            LivingCreatorOperationV2::ForceWar {
                participant_a: existing(0x01),
                participant_b: existing(0x02),
                declarer: existing(0x03),
                start_tick: LivingTickV2(0),
                end_tick: LivingTickV2(600),
            },
            LivingCreatorOperationV2::ForcePeace {
                participant_a: existing(0x01),
                participant_b: existing(0x02),
            },
            LivingCreatorOperationV2::SetFleetOrder {
                fleet: existing(0x01),
                kind: Some(LivingFleetOrderKindV2::Move),
                target_world: Some(existing(0x02)),
                not_before_tick: LivingTickV2(5),
                lane_path: vec![existing(0x03)],
            },
            LivingCreatorOperationV2::SetShipmentDisposition {
                shipment: existing(0x01),
                disposition: LivingShipmentDispositionV2::Outbound,
            },
            LivingCreatorOperationV2::RemoveEntity {
                target: existing(0x01),
                cascade: LivingCascadeDispositionV2::RemoveListed {
                    dependents: vec![existing(0x02)],
                },
            },
        ];
        let additions: LivingCommandV2 = decode_canonical_v2(
            include_bytes!("../../../tests/fixtures/living-v2-additions/creator-command.json"),
            65536,
        )
        .expect("independent additive command corpus");
        for mut operation in additions.operations()[..18].iter().cloned() {
            if let LivingCreatorOperationV2::ScrapFacility { cascade, .. } = &mut operation {
                *cascade = LivingCascadeDispositionV2::RemoveReviewed {
                    dependents: vec![existing(0x02)],
                    derived: vec![],
                };
            }
            templates.push(operation);
        }
        templates
    }

    fn drain(job: &mut LivingPublicationRetirementJobV2, budgets: &[usize]) -> u64 {
        for i in 0..2_000_000 {
            let b = budgets[i % budgets.len()];
            let p = job.poll(b).unwrap();
            assert!(job.last_poll_visits() <= b);
            match p {
                LivingPublicationRetirementProgressV2::Ready => {
                    let n = job.total_visits();
                    assert_eq!(job.poll(1).unwrap(), p);
                    assert_eq!(job.last_poll_visits(), 0);
                    assert_eq!(job.total_visits(), n);
                    return n;
                }
                LivingPublicationRetirementProgressV2::AwaitingExternalOwners => {
                    panic!("unexpected external wait")
                }
                _ => {}
            }
        }
        panic!("retirement did not finish")
    }
    fn phase<T: RetirementFields<'static>>(v: T, budgets: &[usize]) -> u64 {
        let mut j =
            LivingPhaseRetirementJobV2::begin(v, LivingStateRetirementPolicyV2::ReleaseHandle);
        for i in 0..1_000_000 {
            if j.poll(budgets[i % budgets.len()]).unwrap() == LivingLifecycleProgressV2::Ready {
                return j.total_visits();
            }
        }
        panic!("phase did not finish")
    }
    fn enqueue(a: &mut LivingGalaxyAuthorityV2, name_text: &str) {
        let c = a.published_cursor();
        a.submit(LivingCommandEnvelopeV2 {
            expected_committed_revision: c.committed_revision,
            expected_tick: c.tick,
            expected_pending_sequence: c.pending_sequence,
            mode: LivingCommandModeV2::Running,
            command: LivingCommandV2::CreatorBatch {
                operations: vec![LivingCreatorOperationV2::CreateSystem {
                    local: 0,
                    name: name(name_text),
                }],
            },
        })
        .unwrap();
    }
    fn prepared(a: &LivingGalaxyAuthorityV2) -> (Box<LivingStepJobV2>, LivingStepPreparationV2) {
        let mut j = Box::new(a.begin_step().unwrap());
        while j.poll(64).unwrap() == LivingStepProgressV2::Pending {}
        let p = j.take_synchronous_preparation(a).unwrap();
        assert!(matches!(
            j.take_synchronous_preparation(a),
            Err(LivingStepJobErrorV2::NotReady)
        ));
        (j, p)
    }
    #[test]
    fn publication_retirement_capture_handle_contract_exact_count() {
        for bs in [&[1][..], &[64][..], &[1024][..], MIXED] {
            let a = authority();
            let before = a.state.digest().unwrap();
            let capture = a.capture_boundary().unwrap();
            let pointer = Arc::as_ptr(&capture.state);
            let mut j = capture.into_retirement(LivingPublicationRetirementPolicyV2::ReleaseHandle);
            assert_eq!(j.total_visits(), 0);
            assert!(matches!(&j.root,Some(Owner::Capture(v)) if Arc::as_ptr(&v.state)==pointer));
            for b in [0, 1025] {
                assert_eq!(j.poll(b), Err(LivingStepJobErrorV2::Budget));
                assert_eq!(j.last_poll_visits(), 0)
            }
            // root admission + five atomic shared releases + frame + original shell.
            assert_eq!(drain(&mut j, bs), 1 + 5 + 1 + 1);
            assert_eq!(j.shared_handle_releases(), 5);
            assert_eq!(a.state.digest().unwrap(), before);
        }
    }
    #[test]
    fn publication_retirement_all_command_variants_count_nested_records() {
        let ops = every_operation_template();
        assert_eq!(ops.len(), 46);
        for bs in [&[1][..], &[64][..], &[1024][..], MIXED] {
            for op in &ops {
                let mut expected = 1;
                match op {
                    LivingCreatorOperationV2::SetFleetOrder { lane_path, .. } => {
                        expected += 3 * lane_path.len() as u64 + 3
                    }
                    LivingCreatorOperationV2::SplitFleet { hulls, .. } => {
                        expected += 3 * hulls.len() as u64 + 3
                    }
                    LivingCreatorOperationV2::RemoveEntity { cascade, .. }
                    | LivingCreatorOperationV2::ScrapFacility { cascade, .. } => match cascade {
                        LivingCascadeDispositionV2::RejectIfReferenced => {}
                        LivingCascadeDispositionV2::RemoveListed { dependents } => {
                            expected += 3 * dependents.len() as u64 + 3
                        }
                        LivingCascadeDispositionV2::RemoveReviewed {
                            dependents,
                            derived,
                        } => expected += 3 * (dependents.len() + derived.len()) as u64 + 6,
                    },
                    _ => {}
                };
                assert_eq!(phase(op.clone(), bs), expected);
            }
            let op = LivingCreatorOperationV2::RemoveEntity {
                target: existing(1),
                cascade: LivingCascadeDispositionV2::RemoveReviewed {
                    dependents: vec![existing(2); 17],
                    derived: vec![
                        LivingDerivedRemovalV2::Order {
                            fleet: LivingEntityIdV2([3; 16])
                        };
                        19
                    ],
                },
            };
            assert_eq!(phase(op, bs), 1 + 3 * 36 + 6);
            let op = LivingCreatorOperationV2::SplitFleet {
                source: existing(1),
                local: 2,
                hulls: Vec::with_capacity(10),
            };
            assert_eq!(phase(op, bs), 4);
        }
    }
    #[test]
    fn publication_retirement_defer_queue_barrier_and_retry_no_backlog() {
        let mut a = authority();
        enqueue(&mut a, "one");
        let first = a.clone();
        enqueue(&mut a, "two");
        a.archives.push(first);
        let outer = Arc::clone(&a.archives[0].queue);
        let accepted = Arc::clone(&outer[0].accepted);
        let revision = Arc::clone(&outer[0].revision);
        let outer_ptr = Arc::as_ptr(&outer) as usize;
        let accepted_ptr = Arc::as_ptr(&accepted) as usize;
        let revision_ptr = Arc::as_ptr(&revision) as usize;
        let mut j = a.into_retirement(LivingPublicationRetirementPolicyV2::DeferUntilUnique);
        while j.poll(64).unwrap() != LivingPublicationRetirementProgressV2::AwaitingExternalOwners {
        }
        assert_eq!(j.blocked_queues, 1);
        assert!(j.blocked.contains_key(&(0, outer_ptr)));
        assert!(j.blocked.contains_key(&(5, accepted_ptr)));
        assert!(j.blocked.contains_key(&(6, revision_ptr)));
        let n = j.total_visits();
        let sizes = (j.blocked.len(), j.shells.len(), j.payload_shells.len());
        for _ in 0..10 {
            assert_eq!(
                j.poll(1024).unwrap(),
                LivingPublicationRetirementProgressV2::AwaitingExternalOwners
            );
            assert_eq!(j.last_poll_visits(), 0)
        }
        assert_eq!(j.total_visits(), n);
        j.retry_external_owners();
        while j.poll(1).unwrap() != LivingPublicationRetirementProgressV2::AwaitingExternalOwners {}
        assert_eq!(
            sizes,
            (j.blocked.len(), j.shells.len(), j.payload_shells.len())
        );
        drop(outer);
        j.retry_external_owners();
        while j.poll(64).unwrap() != LivingPublicationRetirementProgressV2::AwaitingExternalOwners {
        }
        assert_eq!(j.blocked_queues, 0);
        assert_eq!(j.blocked.len(), 2);
        match j.blocked.get(&(5, accepted_ptr)).unwrap() {
            SharedOwner::Accepted(v) => assert!(Arc::ptr_eq(v, &accepted)),
            _ => panic!(),
        }
        drop(accepted);
        drop(revision);
        j.retry_external_owners();
        drain(&mut j, MIXED);
    }
    #[test]
    fn publication_retirement_internal_authority_graph_has_no_external_wait() {
        for bs in [&[1][..], &[64][..], &[1024][..], MIXED] {
            let mut a = authority();
            assert!(Arc::ptr_eq(&a.state, &a.origin));
            enqueue(&mut a, "prefix");
            for i in 0..10 {
                let mut child = a.clone();
                child.archives.clear();
                child.saved_present = None;
                child.replay_base = None;
                if i % 2 == 0 {
                    a.archives.push(child)
                } else {
                    a.saved_present = Some(Box::new(child))
                }
            }
            let weak = Arc::downgrade(&a.state);
            let mut j = Box::new(a)
                .into_boxed_retirement(LivingPublicationRetirementPolicyV2::DeferUntilUnique);
            drain(&mut j, bs);
            assert!(weak.upgrade().is_none());
            assert!(j.groups.is_empty() && j.blocked.is_empty());
        }
    }
    #[test]
    fn publication_retirement_recursive_errors_are_iterative_with_exact_count() {
        for bs in [&[1][..], &[64][..], &[1024][..], MIXED] {
            let depth = 3000;
            let mut fault = LivingDeterministicFaultV2::Invariant;
            for _ in 0..depth {
                fault = LivingDeterministicFaultV2::Replay {
                    accepted_sequence: 1,
                    reason: Box::new(LivingCommandRejectionV2::Seal(fault)),
                };
            }
            let mut j = LivingPublicationRetirementJobV2::begin(
                Owner::Fault(fault),
                LivingPublicationRetirementPolicyV2::ReleaseHandle,
            );
            assert_eq!(drain(&mut j, bs), 6 * depth + 4);
        }
    }
    #[test]
    fn publication_retirement_commit_and_rejection_preserve_original_token_boxes() {
        for stale in [false, true] {
            let mut a = authority();
            enqueue(&mut a, "before");
            let (job, p) = prepared(&a);
            let LivingStepPreparationV2::Ready(p) = p else {
                panic!()
            };
            let pointer = &*p as *const _;
            if stale {
                enqueue(&mut a, "after");
                let rejected = a.commit_step(p).unwrap_err();
                assert!(
                    matches!(rejected._preparation.as_ref(),Some(LivingStepPreparationV2::Ready(v)) if std::ptr::eq(&**v,pointer))
                );
                drain(
                    &mut rejected
                        .into_retirement(LivingPublicationRetirementPolicyV2::ReleaseHandle),
                    MIXED,
                );
                assert_eq!(a.queue.len(), 2);
            } else {
                let retired = a.commit_step(p).unwrap();
                assert!(
                    matches!(retired._token.as_ref(),Some(LivingStepPreparationV2::Ready(v)) if std::ptr::eq(&**v,pointer)&&v.owners.is_none())
                );
                drain(
                    &mut retired
                        .into_retirement(LivingPublicationRetirementPolicyV2::ReleaseHandle),
                    MIXED,
                );
                assert!(a.queue.is_empty());
            }
            drain(
                &mut job.into_boxed_retirement(LivingPublicationRetirementPolicyV2::ReleaseHandle),
                MIXED,
            );
        }
    }
    #[test]
    fn publication_retirement_failure_commit_stale_and_fault_independent() {
        for stale in [false, true] {
            let mut a = authority();
            Arc::make_mut(&mut a.state).tick = LivingTickV2(u64::MAX);
            let mut job = Box::new(a.begin_step().unwrap());
            while job.poll(64).is_ok() {}
            let LivingStepPreparationV2::Failed(p) = job.take_synchronous_preparation(&a).unwrap()
            else {
                panic!()
            };
            let pointer = &*p as *const _;
            if stale {
                a.accepted_sequence_high_water += 1;
                let rejected = a.commit_step_failure(p).unwrap_err();
                assert!(
                    matches!(rejected._preparation.as_ref(),Some(LivingStepPreparationV2::Failed(v)) if std::ptr::eq(&**v,pointer))
                );
                drain(
                    &mut rejected
                        .into_retirement(LivingPublicationRetirementPolicyV2::ReleaseHandle),
                    MIXED,
                );
            } else {
                let retired = a.commit_step_failure(p).unwrap();
                assert!(
                    matches!(retired._token.as_ref(),Some(LivingStepPreparationV2::Failed(v)) if std::ptr::eq(&**v,pointer)&&v.owners.is_none())
                );
                drain(
                    &mut retired
                        .into_retirement(LivingPublicationRetirementPolicyV2::ReleaseHandle),
                    MIXED,
                );
                assert!(a.fault.is_some());
            }
            job.cancel();
            drain(
                &mut job.into_boxed_retirement(LivingPublicationRetirementPolicyV2::ReleaseHandle),
                MIXED,
            );
        }
    }
    fn creator_fixture() -> LivingGalaxyAuthorityV2 {
        let mut a = authority();
        let operations = vec![
            LivingCreatorOperationV2::CreateSystem {
                local: 0,
                name: name("one"),
            },
            LivingCreatorOperationV2::CreateSystem {
                local: 1,
                name: name("two"),
            },
            LivingCreatorOperationV2::CreateStar {
                local: 2,
                system: LivingCommandTargetV2::Local { local: 0 },
                archetype: slug("main_sequence"),
                name: name("star"),
            },
            LivingCreatorOperationV2::CreateWorld {
                local: 3,
                system: LivingCommandTargetV2::Local { local: 0 },
                archetype: slug("barren"),
                name: name("world"),
                owner: None,
                inventory: Default::default(),
            },
            LivingCreatorOperationV2::CreateLane {
                local: 4,
                system_a: LivingCommandTargetV2::Local { local: 0 },
                system_b: LivingCommandTargetV2::Local { local: 1 },
                distance_units: 10,
            },
            LivingCreatorOperationV2::CreateCivilization {
                local: 5,
                name: name("civilization"),
                policy: LivingPolicyV2(slug("neutral")),
                status: LivingCivilizationStatusV2::Dormant,
            },
            LivingCreatorOperationV2::CreateDeposit {
                local: 6,
                world: LivingCommandTargetV2::Local { local: 3 },
                resource: LivingResourceV2::Ore,
                remaining_units: 20,
            },
        ];
        let c = a.published_cursor();
        a.submit(LivingCommandEnvelopeV2 {
            expected_committed_revision: c.committed_revision,
            expected_tick: c.tick,
            expected_pending_sequence: c.pending_sequence,
            mode: LivingCommandModeV2::Running,
            command: LivingCommandV2::CreatorBatch { operations },
        })
        .unwrap();
        a
    }
    fn stage(j: &LivingStepJobV2) -> u8 {
        if !j.retirement.is_empty() {
            return 1;
        }
        if j.copy.is_some() {
            return 2;
        }
        if j.hash.is_some() {
            return 3;
        }
        if let Some(c) = &j.creator {
            if !c.retirement.is_empty() {
                return 4;
            }
            if let Some(i) = &c.insertion {
                return match i {
                    QueuedCreatorInsertion::Removal(_) => 90,
                    QueuedCreatorInsertion::FleetStructural(_) => 89,
                    QueuedCreatorInsertion::System(_) => 10,
                    QueuedCreatorInsertion::Star(_) => 11,
                    QueuedCreatorInsertion::World(_) => 12,
                    QueuedCreatorInsertion::Lane(_) => 13,
                    QueuedCreatorInsertion::Civilization(_) => 14,
                    QueuedCreatorInsertion::Deposit(_) => 15,
                    QueuedCreatorInsertion::Facility(_) => 50,
                    QueuedCreatorInsertion::ConstructionJob(_) => 51,
                    QueuedCreatorInsertion::HullJob(_) => 52,
                    QueuedCreatorInsertion::Fleet(_) => 53,
                    QueuedCreatorInsertion::Hull(_) => 54,
                    QueuedCreatorInsertion::Route(_) => 55,
                    QueuedCreatorInsertion::Shipment(_) => 56,
                    QueuedCreatorInsertion::Hazard(_) => 57,
                    QueuedCreatorInsertion::Relation(_) => 59,
                    QueuedCreatorInsertion::Agreement(_) => 60,
                    QueuedCreatorInsertion::War(_) => 61,
                    QueuedCreatorInsertion::Colony(_) => 58,
                };
            }
            return match c.stage {
                QueuedCreatorStage::Revision => 20,
                QueuedCreatorStage::DormantCivilizations => 21,
                QueuedCreatorStage::OwnedColonies => 22,
                QueuedCreatorStage::Declarations => 23,
                QueuedCreatorStage::Operation => 24,
                QueuedCreatorStage::Insertion => 25,
                QueuedCreatorStage::HullReassemble => 26,
                QueuedCreatorStage::HazardDefinition => 27,
                QueuedCreatorStage::HazardEdit => 28,
                QueuedCreatorStage::ColonyConstructionReservations => 29,
                QueuedCreatorStage::ColonyHullReservations => 30,
                QueuedCreatorStage::ColonyMutation => 31,
                QueuedCreatorStage::ColonyOwnerEdit => 32,
                QueuedCreatorStage::ColonyCreateInsertion => 33,
                QueuedCreatorStage::ColonyRemoval => 34,
                QueuedCreatorStage::StatusOwners => 35,
                QueuedCreatorStage::StatusCivilizations => 36,
                QueuedCreatorStage::RecoveryPrepare => 37,
                QueuedCreatorStage::RecoveryOwners => 38,
                QueuedCreatorStage::RecoveryActor => 39,
                QueuedCreatorStage::RecoveryEmit => 40,
                QueuedCreatorStage::EmitCreator => 41,
                QueuedCreatorStage::AppendCreator => 42,
                QueuedCreatorStage::FinishRevision => 43,
                QueuedCreatorStage::NeedsSynchronous => 44,
                QueuedCreatorStage::Ready => 45,
                QueuedCreatorStage::AgreementPlan => 46,
                QueuedCreatorStage::OccupationPlan => 47,
                QueuedCreatorStage::OccupationOwner => 48,
                QueuedCreatorStage::DiplomacyPlanEmit => 49,
                QueuedCreatorStage::DiplomacyFilter => 50,
                QueuedCreatorStage::WarInsertion => 51,
                QueuedCreatorStage::OrderReservations => 52,
                QueuedCreatorStage::OrderPrepare => 53,
                QueuedCreatorStage::OrderPath => 54,
                QueuedCreatorStage::OrderInstall => 55,
                QueuedCreatorStage::DiplomacyFinish => 56,
                QueuedCreatorStage::CancelRefund => 57,
                QueuedCreatorStage::CancelRefundWrite => 58,
                QueuedCreatorStage::CancelPlan => 59,
                QueuedCreatorStage::CancelFilterStart => 60,
                QueuedCreatorStage::CancelFilter => 61,
                QueuedCreatorStage::CancelEffect => 62,
                QueuedCreatorStage::CancellationFinish => 63,
                QueuedCreatorStage::RouteSource => 64,
                QueuedCreatorStage::RouteDestination => 65,
                QueuedCreatorStage::RouteWar => 66,
                QueuedCreatorStage::RouteKind => 67,
                QueuedCreatorStage::RouteTrade => 68,
                QueuedCreatorStage::RouteDeadline => 69,
                QueuedCreatorStage::RouteInstall => 70,
                QueuedCreatorStage::EffectPrepare => 71,
                QueuedCreatorStage::EffectCommit => 72,
                QueuedCreatorStage::EffectsStart => 73,
                QueuedCreatorStage::EffectsNext => 74,
                QueuedCreatorStage::EffectAppendStart => 75,
                QueuedCreatorStage::EffectAppend => 76,
                QueuedCreatorStage::BridgeEffectsStart => 77,
                QueuedCreatorStage::BridgeEffectsNext => 78,
                QueuedCreatorStage::WorldShipments { .. } => 79,
                QueuedCreatorStage::WorldFleets { .. } => 80,
                QueuedCreatorStage::WorldInstall { .. } => 81,
                QueuedCreatorStage::LaneFleets { .. } => 82,
                QueuedCreatorStage::LanePath { .. } => 83,
                QueuedCreatorStage::LaneInstall { .. } => 84,
                QueuedCreatorStage::HullDefinition { .. } => 85,
                QueuedCreatorStage::HullParent { .. } => 86,
                QueuedCreatorStage::FleetEditReservations { .. } => 87,
                QueuedCreatorStage::FleetEditInstall { .. } => 88,
            };
        }
        if j.scheduled.is_some() {
            return 40;
        }
        if j.conflict.is_some() {
            return 41;
        }
        if j.production.is_some() {
            return 42;
        }
        if j.observation.is_some() {
            return 43;
        }
        if j.diplomacy.is_some() {
            return 44;
        }
        if j.dispatch.is_some() {
            return 45;
        }
        if j.completed_output.is_some() {
            return 46;
        }
        0
    }
    #[test]
    fn publication_retirement_every_step_creator_and_insertion_stage_preserves_box() {
        let mut seen = std::collections::BTreeSet::new();
        for due in [false, true] {
            let mut a = creator_fixture();
            if due {
                Arc::make_mut(&mut a.state).tick = LivingTickV2(99);
            }
            // First discover exact checkpoints. Reconstruct independently; executors are
            // never cloned to manufacture a cleanup candidate.
            let mut points = Vec::new();
            let mut j = a.begin_step().unwrap();
            for k in 0..100000 {
                let key = stage(&j);
                if seen.insert(key) {
                    points.push(k)
                }
                if j.poll(1).unwrap() != LivingStepProgressV2::Pending {
                    break;
                }
            }
            drain(
                &mut j.into_retirement(LivingPublicationRetirementPolicyV2::ReleaseHandle),
                MIXED,
            );
            for checkpoint in points {
                for bs in [&[1][..], &[64][..], &[1024][..], MIXED] {
                    let mut j = Box::new(a.begin_step().unwrap());
                    for _ in 0..checkpoint {
                        j.poll(1).unwrap();
                    }
                    let key = stage(&j);
                    let pointer = &*j as *const _;
                    j.cancel();
                    let mut r =
                        j.into_boxed_retirement(LivingPublicationRetirementPolicyV2::ReleaseHandle);
                    assert!(
                        matches!(&r.root,Some(Owner::Step(StepOwner::Boxed(v))) if std::ptr::eq(&**v,pointer))
                    );
                    let n = drain(&mut r, bs);
                    assert!(n > 20, "stage {key}");
                }
            }
        }
        for key in [
            1, 2, 3, 4, 10, 11, 12, 13, 14, 15, 40, 41, 42, 43, 44, 45, 46,
        ] {
            assert!(seen.contains(&key), "missing stage {key}")
        }
    }
    #[test]
    fn publication_retirement_creator_ready_once_and_empty_original_output_box() {
        let a = creator_fixture();
        let mut c = Box::new(LivingQueuedCreatorJobV2::begin(
            a.state.as_ref().clone(),
            Arc::clone(&a.queue),
            Arc::clone(&a.catalog),
        ));
        assert!(c.take_ready_result().is_err());
        while c.poll(1).unwrap() == LivingStepProgressV2::Pending {}
        let (state, events) = c.take_ready_result().unwrap();
        assert!(c.take_ready_result().is_err());
        let pointer = &*c as *const _;
        let mut r = LivingPublicationRetirementJobV2::begin(
            Owner::Creator(c),
            LivingPublicationRetirementPolicyV2::ReleaseHandle,
        );
        assert!(matches!(&r.root,Some(Owner::Creator(v)) if std::ptr::eq(&**v,pointer)));
        drain(&mut r, MIXED);
        let output = Box::new((
            state,
            events,
            LivingCompletedStepPhasesV2::Scheduled {
                arrivals: vec![LivingEntityIdV2([9; 16]); 21],
            },
        ));
        let pointer = &*output as *const _;
        let mut r = LivingPublicationRetirementJobV2::begin(
            Owner::Completed(output),
            LivingPublicationRetirementPolicyV2::ReleaseHandle,
        );
        r.poll(1).unwrap();
        assert!(
            matches!(&r.frames.front().unwrap().owner,Owner::Completed(v) if std::ptr::eq(&**v,pointer))
        );
        drain(&mut r, MIXED);
    }
    #[test]
    fn publication_retirement_original_step_and_authority_shells_survive_external_wait() {
        let a = creator_fixture();
        let mut j = Box::new(a.begin_step().unwrap());
        for _ in 0..100 {
            j.poll(1).unwrap();
        }
        let pointer = &*j as *const _;
        let mut r = j.into_boxed_retirement(LivingPublicationRetirementPolicyV2::DeferUntilUnique);
        while r.poll(64).unwrap() != LivingPublicationRetirementProgressV2::AwaitingExternalOwners {
        }
        assert!(
            r.shells.iter().any(
                |o| matches!(o,Owner::Step(StepOwner::Boxed(v)) if std::ptr::eq(&**v,pointer))
            )
        );
        // Dropping authority here cannot final-drop shared payloads: all live roots
        // are held by this task. Production runtime must hand it off separately.
        drop(a);
        r.retry_external_owners();
        drain(&mut r, MIXED);
        let a = Box::new(creator_fixture());
        let pointer = &*a as *const _;
        let anchor = Arc::clone(&a.queue);
        let mut r = a.into_boxed_retirement(LivingPublicationRetirementPolicyV2::DeferUntilUnique);
        while r.poll(64).unwrap() != LivingPublicationRetirementProgressV2::AwaitingExternalOwners {
        }
        assert!(r.shells.iter().any(
            |o| matches!(o,Owner::Authority(AuthorityOwner::Boxed(v)) if std::ptr::eq(&**v,pointer))
        ));
        drop(anchor);
        r.retry_external_owners();
        drain(&mut r, MIXED);
    }
    #[test]
    fn publication_retirement_deep_authority_spine_releases_children_before_parent() {
        let template = authority();
        let state = Arc::downgrade(&template.state);
        let mut a = template.clone();
        for _ in 0..1500 {
            let mut next = template.clone();
            next.saved_present = Some(Box::new(a));
            a = next;
        }
        drop(template);
        let root = Box::new(a);
        let pointer = &*root as *const _;
        let mut r =
            root.into_boxed_retirement(LivingPublicationRetirementPolicyV2::DeferUntilUnique);
        while r.poll(1).unwrap() != LivingPublicationRetirementProgressV2::Ready {
            if r.frames.is_empty()
                && r.groups.is_empty()
                && r.active.is_none()
                && r.payload_shells.is_empty()
                && r.shells.len() > 1
            {
                assert!(
                    matches!(r.shells.back(),Some(Owner::Authority(AuthorityOwner::Boxed(v))) if std::ptr::eq(&**v,pointer))
                );
            }
        }
        assert!(state.upgrade().is_none());
    }
    #[test]
    fn publication_retirement_sidecar_nested_ranges_and_facts_independent_formula() {
        for bs in [&[1][..], &[64][..], &[1024][..], MIXED] {
            let facts = LivingEventFactsV2 {
                resources: vec![
                    LivingEventResourceFactV2 {
                        role: LivingEventResourceRoleV2::Input,
                        resource: LivingResourceV2::Ore,
                        quantity: 1
                    };
                    7
                ],
                blocked_reasons: vec![
                    LivingBlockedReasonV2::MissingInput {
                        resource: LivingResourceV2::Ore,
                        required: 3,
                        available: 1
                    };
                    5
                ],
                affected: vec![LivingEntityIdV2([1; 16]); 11],
                ..Default::default()
            };
            let explained = LivingExplainedEventV2 {
                ordinal: 0,
                event_id: LivingEventIdV2([2; 16]),
                facts,
            };
            // Three vectors: each child entry + 3 per leaf + storage + child release,
            // then the inline explained/facts record's original shell.
            assert_eq!(phase(explained, bs), 3 * 23 + 3 * 3 + 1);
            let omitted = LivingEventFactsOmittedV2 {
                reason: LivingEventFactsOmissionReasonV2::Capacity,
                ranges: vec![[1, 2]; 17],
            };
            assert_eq!(phase(omitted, bs), 3 * 17 + 4);
        }
    }

    #[test]
    fn publication_retirement_creator_control_states_and_replay_value() {
        for stage in [
            QueuedCreatorStage::Revision,
            QueuedCreatorStage::DormantCivilizations,
            QueuedCreatorStage::OwnedColonies,
            QueuedCreatorStage::Declarations,
            QueuedCreatorStage::Operation,
            QueuedCreatorStage::Insertion,
            QueuedCreatorStage::HullReassemble,
            QueuedCreatorStage::HazardDefinition,
            QueuedCreatorStage::HazardEdit,
            QueuedCreatorStage::AppendCreator,
            QueuedCreatorStage::EmitCreator,
            QueuedCreatorStage::FinishRevision,
            QueuedCreatorStage::NeedsSynchronous,
            QueuedCreatorStage::Ready,
            QueuedCreatorStage::AgreementPlan,
            QueuedCreatorStage::OccupationPlan,
            QueuedCreatorStage::OccupationOwner,
            QueuedCreatorStage::DiplomacyPlanEmit,
            QueuedCreatorStage::DiplomacyFilter,
            QueuedCreatorStage::WarInsertion,
            QueuedCreatorStage::OrderReservations,
            QueuedCreatorStage::OrderPrepare,
            QueuedCreatorStage::OrderPath,
            QueuedCreatorStage::OrderInstall,
            QueuedCreatorStage::DiplomacyFinish,
        ] {
            let a = creator_fixture();
            let mut c = Box::new(LivingQueuedCreatorJobV2::begin(
                a.state.as_ref().clone(),
                Arc::clone(&a.queue),
                Arc::clone(&a.catalog),
            ));
            c.stage = stage;
            c.locals.insert(0, LivingEntityIdV2([1; 16]));
            c.dormant.insert(LivingEntityIdV2([2; 16]));
            drain(
                &mut LivingPublicationRetirementJobV2::begin(
                    Owner::Creator(c),
                    LivingPublicationRetirementPolicyV2::ReleaseHandle,
                ),
                MIXED,
            );
        }
        let a = creator_fixture();
        let replay = LivingReplayBoundaryV2 {
            candidate: a.state.as_ref().clone(),
            events: LivingPendingEventsV2::new(),
            records: vec![LivingRecordedRevisionV2 {
                revision: a.queue[0].revision.as_ref().clone(),
                mode: LivingCommandModeV2::Running,
            }],
            tick: LivingTickV2(1),
            parent: None,
        };
        drain(
            &mut replay.into_retirement(LivingPublicationRetirementPolicyV2::ReleaseHandle),
            MIXED,
        );
    }

    #[test]
    fn publication_retirement_scheduled_conflict_ready_box_and_partial_substages() {
        for conflict in [false, true] {
            let a = if conflict {
                super::super::tests::combat_cursor_fixture()
            } else {
                super::super::tests::scheduled_staged_fixture()
            };
            for offset in [0, 1, 3, 17, 64, 127] {
                let mut step = Box::new(a.begin_step().unwrap());
                while if conflict {
                    step.conflict.is_none()
                } else {
                    step.scheduled.is_none()
                } {
                    step.poll(1).unwrap();
                }
                if conflict {
                    let child = step.conflict.as_mut().unwrap();
                    let pointer = &**child as *const _;
                    for _ in 0..offset {
                        child.poll(1).unwrap();
                    }
                    assert!(std::ptr::eq(&**child, pointer));
                    let mut r = step
                        .into_boxed_retirement(LivingPublicationRetirementPolicyV2::ReleaseHandle);
                    r.poll(1).unwrap();
                    assert!(
                        matches!(&r.frames.front().unwrap().owner,Owner::Step(StepOwner::Boxed(v)) if std::ptr::eq(&**v.conflict.as_ref().unwrap(),pointer))
                    );
                    drain(&mut r, MIXED);
                } else {
                    let child = step.scheduled.as_mut().unwrap();
                    let pointer = &**child as *const _;
                    for _ in 0..offset {
                        child.poll(1).unwrap();
                    }
                    assert!(std::ptr::eq(&**child, pointer));
                    let mut r = step
                        .into_boxed_retirement(LivingPublicationRetirementPolicyV2::ReleaseHandle);
                    r.poll(1).unwrap();
                    assert!(
                        matches!(&r.frames.front().unwrap().owner,Owner::Step(StepOwner::Boxed(v)) if std::ptr::eq(&**v.scheduled.as_ref().unwrap(),pointer))
                    );
                    drain(&mut r, MIXED);
                }
            }
            let mut step = Box::new(a.begin_step().unwrap());
            while if conflict {
                step.conflict.is_none()
            } else {
                step.scheduled.is_none()
            } {
                step.poll(1).unwrap();
            }
            if conflict {
                let child = step.conflict.as_mut().unwrap();
                let pointer = &**child as *const _;
                while child.poll(64).unwrap() != LivingLifecycleProgressV2::Ready {}
                let (state, events) = child.take_ready_result().unwrap();
                assert!(std::ptr::eq(&**child, pointer));
                assert!(child.take_ready_result().is_err());
                phase((state, events), MIXED);
            } else {
                let child = step.scheduled.as_mut().unwrap();
                let pointer = &**child as *const _;
                while child.poll(64).unwrap() != LivingLifecycleProgressV2::Ready {}
                let (state, events, arrivals) = child.take_ready_result().unwrap();
                assert!(!arrivals.is_empty());
                assert!(std::ptr::eq(&**child, pointer));
                assert!(child.take_ready_result().is_err());
                phase((state, events, arrivals), MIXED);
            }
            drain(
                &mut step.into_boxed_retirement(LivingPublicationRetirementPolicyV2::ReleaseHandle),
                MIXED,
            );
        }
    }
    #[test]
    fn publication_retirement_decision_reason_records_are_counted() {
        for bs in [&[1][..], &[64][..], &[1024][..], MIXED] {
            let decision = LivingIntentDecisionV2 {
                phase: LivingPhaseV2::GenerateIntents,
                actor: LivingEntityIdV2([1; 16]),
                ordinal: 0,
                action: LivingIntentActionV2::War {
                    peer: LivingEntityIdV2([2; 16]),
                },
                target: None,
                score: LivingDecisionScoreV2(1),
                tier: LivingDecisionTierV2::Ordinary,
                snapshot_tick: LivingTickV2(1),
                snapshot_digest: LivingStateDigestV2([0; 32]),
                observed_tick: None,
                outcome: LivingDecisionOutcomeV2::Accepted,
                reasons: vec![LivingDecisionReasonV2::Accepted { rule: slug("rule") }; 31],
            };
            assert_eq!(phase(decision, bs), 3 * 31 + 4);
        }
    }
    #[test]
    fn publication_retirement_exhausted_registry_roots_have_counted_release() {
        let mut r =
            authority().into_retirement(LivingPublicationRetirementPolicyV2::DeferUntilUnique);
        while r.registry_storage_released == 0 {
            let before = r.total_visits();
            r.poll(1).unwrap();
            if r.registry_storage_released == 1 {
                assert_eq!(r.total_visits(), before + 1);
                assert_eq!(r.last_poll_visits(), 1);
            }
        }
        assert!(!r.shells.is_empty());
        let before = r.total_visits();
        r.poll(1).unwrap();
        assert_eq!(r.registry_storage_released, 2);
        assert_eq!(r.total_visits(), before + 1);
        assert_eq!(r.last_poll_visits(), 1);
        drain(&mut r, MIXED);
    }
    #[test]
    fn publication_retirement_fleet_creator_scheduled_fault_retains_capture() {
        let mut a = super::super::tests::scheduled_staged_fixture();
        let fleet = a
            .state
            .fleets
            .as_slice()
            .iter()
            .find(|fleet| {
                matches!(fleet.location, LivingFleetLocationV2::Docked { .. })
                    && !a
                        .state
                        .hull_jobs
                        .as_slice()
                        .iter()
                        .any(|job| job.target_fleet == Some(fleet.id))
            })
            .unwrap()
            .clone();
        let LivingFleetLocationV2::Docked { world } = fleet.location else {
            unreachable!()
        };
        let cursor = a.published_cursor();
        a.submit(LivingCommandEnvelopeV2 {
            expected_committed_revision: cursor.committed_revision,
            expected_tick: cursor.tick,
            expected_pending_sequence: cursor.pending_sequence,
            mode: LivingCommandModeV2::Running,
            command: LivingCommandV2::CreatorBatch {
                operations: vec![LivingCreatorOperationV2::EditFleet {
                    target: LivingCommandTargetV2::Existing { id: fleet.id },
                    owner: LivingCommandTargetV2::Existing { id: fleet.owner },
                    world: LivingCommandTargetV2::Existing { id: world },
                }],
            },
        })
        .unwrap();
        // The next boundary exists, but expiring this war cannot represent its
        // mandatory positive-duration truce. Creator replay itself still succeeds.
        let state = Arc::make_mut(&mut a.state);
        state.tick = LivingTickV2(u64::MAX - 1);
        state.wars.record_at_mut(0).unwrap().end_tick = LivingTickV2(u64::MAX);
        let mut reference = a.clone();
        assert_eq!(
            reference.step(),
            Err(LivingDeterministicFaultV2::Arithmetic)
        );
        drop(reference);
        let mut step = Box::new(a.begin_step().unwrap());
        let mut failed = false;
        for _ in 0..100000 {
            let before = step.total_visits();
            match step.poll(64) {
                Err(LivingStepJobErrorV2::Fault(LivingDeterministicFaultV2::Arithmetic)) => {
                    assert!(step.last_poll_visits() > 0 && step.last_poll_visits() <= 64);
                    assert_eq!(step.total_visits() - before, step.last_poll_visits() as u64);
                    failed = true;
                    break;
                }
                Ok(LivingStepProgressV2::Pending) => {}
                other => panic!("expected bounded creator then scheduled arithmetic: {other:?}"),
            }
        }
        assert!(failed);
        assert!(step.creator.is_none());
        let scheduled = &**step.scheduled.as_ref().unwrap() as *const _;
        assert_eq!(step.fault, Some(LivingDeterministicFaultV2::Arithmetic));
        let ids = |capture: &LivingBoundaryCaptureV2| {
            [
                Arc::as_ptr(&capture.state) as usize,
                Arc::as_ptr(&capture.catalog) as usize,
                Arc::as_ptr(&capture.queue) as usize,
                Arc::as_ptr(&capture.revisions) as usize,
                Arc::as_ptr(&capture.revision_modes) as usize,
            ]
        };
        let original = ids(&step.capture);
        let preparation = step.take_synchronous_preparation(&a).unwrap();
        let LivingStepPreparationV2::Failed(token) = &preparation else {
            panic!("expected scheduled failure")
        };
        assert_eq!(token.fault(), &LivingDeterministicFaultV2::Arithmetic);
        assert!(std::ptr::eq(&**step.scheduled.as_ref().unwrap(), scheduled));
        assert!(
            step.delivered,
            "scheduled failure records once-only extraction"
        );
        assert_eq!(ids(&step.capture), original);
        assert_eq!(ids(&token.owners.as_ref().unwrap().capture), original);
        assert!(matches!(
            step.take_synchronous_preparation(&a),
            Err(LivingStepJobErrorV2::NotReady)
        ));
        let visits = step.total_visits();
        assert_eq!(step.poll(64), Err(LivingStepJobErrorV2::NotReady));
        assert_eq!(step.last_poll_visits(), 0);
        assert_eq!(step.total_visits(), visits);
        let mut r =
            step.into_boxed_retirement(LivingPublicationRetirementPolicyV2::DeferUntilUnique);
        while r.poll(64).unwrap() != LivingPublicationRetirementProgressV2::AwaitingExternalOwners {
        }
        drain(
            &mut a.into_retirement(LivingPublicationRetirementPolicyV2::ReleaseHandle),
            MIXED,
        );
        r.retry_external_owners();
        while r.poll(64).unwrap() != LivingPublicationRetirementProgressV2::AwaitingExternalOwners {
        }
        for key in [
            (1, original[0]),
            (2, original[1]),
            (0, original[2]),
            (3, original[3]),
            (4, original[4]),
        ] {
            assert!(
                r.blocked.contains_key(&key),
                "token retains exact capture owner {key:?}"
            );
        }
        let visits = r.total_visits();
        assert_eq!(
            r.poll(1024).unwrap(),
            LivingPublicationRetirementProgressV2::AwaitingExternalOwners
        );
        assert_eq!(r.last_poll_visits(), 0);
        assert_eq!(r.total_visits(), visits);
        drain(
            &mut preparation.into_retirement(LivingPublicationRetirementPolicyV2::ReleaseHandle),
            MIXED,
        );
        r.retry_external_owners();
        drain(&mut r, MIXED);
    }
    #[test]
    fn publication_retirement_borrowed_failure_retains_phase_source_anchors() {
        let a = creator_fixture();
        let mut step = Box::new(a.begin_step().unwrap());
        while step.conflict.is_none() {
            step.poll(1).unwrap();
        }
        step.fault = Some(LivingDeterministicFaultV2::Invariant);
        let preparation = step.take_synchronous_preparation(&a).unwrap();
        let mut r =
            step.into_boxed_retirement(LivingPublicationRetirementPolicyV2::DeferUntilUnique);
        loop {
            match r.poll(64).unwrap() {
                LivingPublicationRetirementProgressV2::Pending => {}
                LivingPublicationRetirementProgressV2::AwaitingExternalOwners => break,
                LivingPublicationRetirementProgressV2::Ready => {
                    panic!("phase aliases released without retaining Defer anchors")
                }
            }
        }
        assert!(
            r.blocked.keys().any(|key| key.0 == 2),
            "catalog owner retained"
        );
        drain(
            &mut a.into_retirement(LivingPublicationRetirementPolicyV2::ReleaseHandle),
            MIXED,
        );
        r.retry_external_owners();
        while r.poll(64).unwrap() != LivingPublicationRetirementProgressV2::AwaitingExternalOwners {
        }
        let LivingStepPreparationV2::Failed(token) = &preparation else {
            panic!()
        };
        let catalog = &token.owners.as_ref().unwrap().capture.catalog;
        match r.blocked.get(&(2, Arc::as_ptr(catalog) as usize)).unwrap() {
            SharedOwner::Catalog(held) => assert!(Arc::ptr_eq(held, catalog)),
            _ => panic!(),
        }
        drain(
            &mut preparation.into_retirement(LivingPublicationRetirementPolicyV2::ReleaseHandle),
            MIXED,
        );
        r.retry_external_owners();
        drain(&mut r, MIXED);
    }

    #[test]
    fn due_intent_step_retirement_optional_tail_preserves_all_four_owner_combinations() {
        for intents in [false, true] {
            for revisions in [false, true] {
                for budgets in [&[1][..], &[64][..], &[1024][..], MIXED] {
                    let authority = authority();
                    let mut step = Box::new(authority.begin_step().unwrap());
                    if intents {
                        step.intents = Some(Box::new(
                            super::super::super::rules::LivingDueIntentsJobV2::begin(
                                Arc::clone(&authority.catalog),
                                LivingGalaxyStateV2::default(),
                                LivingPendingEventsV2::default(),
                                LivingTickV2(20),
                                [0x33; 32],
                                authority.branch,
                            ),
                        ));
                    }
                    if revisions {
                        step.applied_revisions = Some(LivingAppliedRevisionPreparationV2::begin(1));
                    }
                    let mut frame = Frame {
                        owner: Owner::Step(StepOwner::Boxed(step)),
                        field: 15,
                    };
                    let aliases = Arc::strong_count(&authority.catalog);
                    // Each present original owner yields exactly one child.
                    // Neither absent owner produces a placeholder node.
                    for index in 0..usize::from(intents) + usize::from(revisions) {
                        let Action::Node(mut child) = frame.visit() else {
                            panic!("present tail owner omitted")
                        };
                        if intents && index == 0 {
                            assert_eq!(
                                Arc::strong_count(&authority.catalog),
                                aliases,
                                "handoff retains original catalog aliases"
                            );
                        }
                        let Owner::Step(owner) = &mut frame.owner else {
                            unreachable!()
                        };
                        assert!(owner.value().intents.is_none());
                        assert_eq!(
                            owner.value().applied_revisions.is_some(),
                            intents && revisions && index == 0
                        );
                        let mut finished = false;
                        for _ in 0..100_000 {
                            match child.retirement_step() {
                                RetirementStep::Done => {
                                    finished = true;
                                    break;
                                }
                                RetirementStep::Work => {}
                                RetirementStep::Deferred(_) => {
                                    panic!("ReleaseHandle has external anchors")
                                }
                            }
                        }
                        assert!(finished);
                    }
                    assert!(matches!(frame.visit(), Action::Done));
                    let mut rest = LivingPublicationRetirementJobV2::begin(
                        frame.owner,
                        LivingPublicationRetirementPolicyV2::ReleaseHandle,
                    );
                    drain(&mut rest, budgets);
                }
            }
        }
    }

    #[test]
    fn publication_retirement_borrowed_rejection_preserves_capture_and_partial_owners() {
        for mode in 0..3 {
            let mut a = creator_fixture();
            let mut step = Box::new(a.begin_step().unwrap());
            step.poll(1).unwrap();
            let source = Arc::as_ptr(&step.capture.state);
            let queue = Arc::as_ptr(&step.capture.queue);
            let pointer = &*step as *const _;
            if mode == 1 {
                step.cancel();
            } else if mode == 2 {
                enqueue(&mut a, "new tail");
            }
            let result = step.take_synchronous_preparation(&a);
            assert!(matches!(
                result,
                Err(LivingStepJobErrorV2::NotReady
                    | LivingStepJobErrorV2::Cancelled
                    | LivingStepJobErrorV2::Stale)
            ));
            assert!(!step.delivered);
            assert_eq!(Arc::as_ptr(&step.capture.state), source);
            assert_eq!(Arc::as_ptr(&step.capture.queue), queue);
            assert!(std::ptr::eq(&*step, pointer));
            drain(
                &mut step.into_boxed_retirement(LivingPublicationRetirementPolicyV2::ReleaseHandle),
                MIXED,
            );
        }
        let a = creator_fixture();
        let (mut step, token) = prepared(&a);
        let visits = step.total_visits();
        assert!(!step.is_current(&a));
        assert_eq!(step.poll(1), Err(LivingStepJobErrorV2::NotReady));
        assert_eq!(step.last_poll_visits(), 0);
        assert_eq!(step.total_visits(), visits);
        assert!(step.capture.is_present());
        drain(
            &mut step.into_boxed_retirement(LivingPublicationRetirementPolicyV2::ReleaseHandle),
            MIXED,
        );
        drain(
            &mut token.into_retirement(LivingPublicationRetirementPolicyV2::ReleaseHandle),
            MIXED,
        );
    }
}

// All coordinator state is classified explicitly, including composition controls.
audited_fields!(LivingPublicationRetirementJobV2;root,admission_open,close_requested,archive_context,recomposed,reopen_requested,frames,shells,payload_shells,groups_started,registry_storage_released,active,unexpected,groups,blocked,blocked_queues,policy,waiting,ready,last,total,shared);

#[cfg(test)]
mod archive_bridge_tests {
    use super::*;
    use crate::living::builtin_living_sandbox_v2;
    fn enumerate(job: &mut LivingPublicationRetirementJobV2) {
        for _ in 0..1_000_000 {
            if job.admission_idle() {
                return;
            }
            assert_eq!(
                job.poll(1).unwrap(),
                LivingPublicationRetirementProgressV2::Pending
            );
        }
        panic!("finite holder enumeration");
    }
    fn finish(job: &mut LivingPublicationRetirementJobV2) -> LivingPublicationRetirementProgressV2 {
        for _ in 0..1_000_000 {
            let p = job.poll(64).unwrap();
            if p != LivingPublicationRetirementProgressV2::Pending {
                return p;
            }
        }
        panic!("finite completion");
    }
    #[test]
    fn archive_bridge_open_refusal_close_and_shared_roots() {
        let a = builtin_living_sandbox_v2().unwrap();
        let b = Box::new(a.clone());
        let pointer = &*b as *const _;
        let mut c = LivingPublicationRetirementJobV2::begin_open(
            LivingPublicationRetirementPolicyV2::DeferUntilUnique,
        );
        c.try_admit_authority(a).unwrap();
        let b = c.try_admit_boxed_authority(b).unwrap_err();
        assert_eq!(&*b as *const _, pointer);
        assert!(!c.close_admission());
        enumerate(&mut c);
        let groups = c.groups.len();
        assert!(groups > 0);
        for _ in 0..3 {
            assert_eq!(
                c.poll(1024).unwrap(),
                LivingPublicationRetirementProgressV2::Pending
            );
            assert_eq!(c.last_poll_visits(), 0);
            assert_eq!(c.groups.len(), groups);
        }
        c.try_admit_boxed_authority(b).unwrap();
        enumerate(&mut c);
        assert!(c.close_admission());
        assert!(!c.close_admission());
        assert_eq!(c.poll(0), Err(LivingStepJobErrorV2::Budget));
        assert_eq!(c.last_poll_visits(), 0);
        c.poll(1).unwrap();
        assert_eq!(c.last_poll_visits(), 1);
        assert!(!c.admission_open);
        assert!(!c.groups_started);
        assert_eq!(finish(&mut c), LivingPublicationRetirementProgressV2::Ready);
        c.poll(1024).unwrap();
        assert_eq!(c.last_poll_visits(), 0);
    }
    #[test]
    fn archive_bridge_reopen_provenance_and_counted_transition() {
        let a = builtin_living_sandbox_v2().unwrap();
        let mut public = a
            .begin_step()
            .unwrap()
            .into_retirement(LivingPublicationRetirementPolicyV2::ReleaseHandle);
        assert!(
            !public.request_archive_reopen(LivingPublicationRetirementPolicyV2::DeferUntilUnique)
        );
        assert_eq!(public.total_visits(), 0);
        assert_eq!(
            finish(&mut public),
            LivingPublicationRetirementProgressV2::Ready
        );
        let mut c = LivingPublicationRetirementJobV2::begin_open(
            LivingPublicationRetirementPolicyV2::ReleaseHandle,
        );
        c.try_admit_authority(a).unwrap();
        c.poll(1).unwrap();
        assert!(c.request_archive_reopen(LivingPublicationRetirementPolicyV2::DeferUntilUnique));
        assert!(!c.request_archive_reopen(LivingPublicationRetirementPolicyV2::ReleaseHandle));
        let before = c.total_visits();
        c.poll(1).unwrap();
        assert_eq!(c.total_visits(), before + 1);
        assert!(c.admission_open);
        assert!(c.recomposed);
        assert!(!c.request_archive_reopen(LivingPublicationRetirementPolicyV2::ReleaseHandle));
        enumerate(&mut c);
        assert!(c.close_admission());
        assert_eq!(finish(&mut c), LivingPublicationRetirementProgressV2::Ready);
    }
    #[test]
    fn archive_bridge_faults_reuse_exact_iterative_boxes() {
        let mut fault = LivingDeterministicFaultV2::Invariant;
        for _ in 0..3000 {
            fault = LivingDeterministicFaultV2::Replay {
                accepted_sequence: 1,
                reason: Box::new(LivingCommandRejectionV2::Seal(fault)),
            };
        }
        let mut c = LivingPublicationRetirementJobV2::begin_open(
            LivingPublicationRetirementPolicyV2::ReleaseHandle,
        );
        c.try_admit_fault(fault).unwrap();
        enumerate(&mut c);
        assert!(c.close_admission());
        assert_eq!(finish(&mut c), LivingPublicationRetirementProgressV2::Ready);
        // Existing fault formula 6n+4 plus exactly one admission-close visit.
        assert_eq!(c.total_visits(), 6 * 3000 + 5);
    }
    fn enqueue(a: &mut LivingGalaxyAuthorityV2, name: &str) {
        let c = a.published_cursor();
        a.submit(LivingCommandEnvelopeV2 {
            expected_committed_revision: c.committed_revision,
            expected_tick: c.tick,
            expected_pending_sequence: c.pending_sequence,
            mode: LivingCommandModeV2::Running,
            command: LivingCommandV2::CreatorBatch {
                operations: vec![LivingCreatorOperationV2::CreateSystem {
                    local: 0,
                    name: crate::living::LivingNameV2::new(name).unwrap(),
                }],
            },
        })
        .unwrap();
    }
    #[test]
    fn archive_bridge_reopen_at_owned_child_and_queue_row_retains_all_aliases() {
        for queue_cut in [false, true] {
            for policy in [
                LivingPublicationRetirementPolicyV2::ReleaseHandle,
                LivingPublicationRetirementPolicyV2::DeferUntilUnique,
            ] {
                for budget in [1, 64, 1024] {
                    let mut a = builtin_living_sandbox_v2().unwrap();
                    a.step().unwrap();
                    enqueue(&mut a, "one");
                    let mut b = a.clone();
                    enqueue(&mut b, "two");
                    assert!(!Arc::ptr_eq(&a.queue, &b.queue));
                    assert!(Arc::ptr_eq(&a.queue[0].accepted, &b.queue[0].accepted));
                    assert!(Arc::ptr_eq(&a.queue[0].revision, &b.queue[0].revision));
                    let external = Arc::clone(&b.queue[0].accepted);
                    let identity = Arc::as_ptr(&external) as usize;
                    let mut c = LivingPublicationRetirementJobV2::begin_open(
                        LivingPublicationRetirementPolicyV2::ReleaseHandle,
                    );
                    c.try_admit_authority(a).unwrap();
                    for i in 0..100_000 {
                        let found = if queue_cut {
                            c.frames
                                .iter()
                                .any(|f| matches!(&f.owner, Owner::Queued(_)) && f.field == 0)
                        } else {
                            c.active.is_some()
                        };
                        if found {
                            break;
                        }
                        assert!(i < 99_999);
                        c.poll(1).unwrap();
                    }
                    assert!(c.request_archive_reopen(policy));
                    c.poll(1).unwrap();
                    enumerate(&mut c);
                    c.try_admit_authority(b).unwrap();
                    enumerate(&mut c);
                    assert!(c.close_admission());
                    let status = loop {
                        let p = c.poll(budget).unwrap();
                        if p != LivingPublicationRetirementProgressV2::Pending {
                            break p;
                        }
                    };
                    if policy == LivingPublicationRetirementPolicyV2::DeferUntilUnique {
                        assert_eq!(
                            status,
                            LivingPublicationRetirementProgressV2::AwaitingExternalOwners
                        );
                        assert!(c.blocked.contains_key(&(5, identity)));
                        let groups = c.blocked.len();
                        let shells = c.shells.len() + c.payload_shells.len();
                        for _ in 0..3 {
                            c.poll(budget).unwrap();
                            assert_eq!(c.last_poll_visits(), 0);
                            c.retry_external_owners();
                            assert_eq!(
                                finish(&mut c),
                                LivingPublicationRetirementProgressV2::AwaitingExternalOwners
                            );
                            assert_eq!(c.blocked.len(), groups);
                            assert_eq!(c.shells.len() + c.payload_shells.len(), shells);
                        }
                        drop(external);
                        c.retry_external_owners();
                        assert_eq!(finish(&mut c), LivingPublicationRetirementProgressV2::Ready);
                    } else {
                        assert_eq!(status, LivingPublicationRetirementProgressV2::Ready);
                        // This external owner's final Drop is explicitly outside handle-release evidence.
                        drop(external);
                    }
                }
            }
        }
    }
    #[test]
    fn archive_bridge_streamed_deep_history_has_no_recursive_drop() {
        let template = builtin_living_sandbox_v2().unwrap();
        let mut a = template.clone();
        for _ in 0..1500 {
            let mut next = template.clone();
            next.replay_base = Some(Box::new(a));
            a = next;
        }
        let mut c = LivingPublicationRetirementJobV2::begin_open(
            LivingPublicationRetirementPolicyV2::DeferUntilUnique,
        );
        c.try_admit_authority(a).unwrap();
        enumerate(&mut c);
        c.try_admit_authority(template).unwrap();
        enumerate(&mut c);
        assert!(c.close_admission());
        assert_eq!(finish(&mut c), LivingPublicationRetirementProgressV2::Ready);
    }
}

#[cfg(test)]
mod creator_parent_cut_tests {
    use super::*;
    use crate::living::model::*;
    use crate::living::simulation::creator_batch_cursor_tests as fixture;
    const SCHEDULES: &[&[usize]] = &[&[1], &[64], &[1024], &[1, 7, 64, 2, 1024]];
    fn id(n: u8) -> LivingEntityIdV2 {
        LivingEntityIdV2([n; 16])
    }
    fn source(
        kind: usize,
        equal: bool,
    ) -> (
        Box<LivingQueuedCreatorJobV2>,
        Arc<Vec<QueuedCommandV2>>,
        Arc<ValidatedLivingCatalogPackV2>,
    ) {
        source_at(kind, equal, 1, 3)
    }
    fn source_at(
        kind: usize,
        equal: bool,
        position: usize,
        capacity: usize,
    ) -> (
        Box<LivingQueuedCreatorJobV2>,
        Arc<Vec<QueuedCommandV2>>,
        Arc<ValidatedLivingCatalogPackV2>,
    ) {
        // Private malformed projection, natural creator control path. No public
        // replay claim. Each construction is fresh; no stopped owner is repaired.
        let catalog = Arc::new(crate::living::living_core_pack_v2().unwrap());
        let all = fixture::queue(fixture::operations());
        let mut templates = fixture::base();
        fixture::reference(&mut templates, &all, &catalog);
        let mut operation = fixture::operations()[kind].clone();
        match &mut operation {
            LivingCreatorOperationV2::CreateFacility { local, .. }
            | LivingCreatorOperationV2::CreateConstructionJob { local, .. }
            | LivingCreatorOperationV2::CreateHullJob { local, .. }
            | LivingCreatorOperationV2::CreateFleet { local, .. }
            | LivingCreatorOperationV2::CreateHull { local, .. }
            | LivingCreatorOperationV2::CreateRoute { local, .. }
            | LivingCreatorOperationV2::CreateShipment { local, .. }
            | LivingCreatorOperationV2::CreateHazard { local, .. } => *local = 0,
            _ => unreachable!(),
        }
        if let LivingCreatorOperationV2::CreateHull { fleet, .. } = &mut operation {
            *fleet = LivingCommandTargetV2::Existing { id: id(90) };
        }
        if let LivingCreatorOperationV2::CreateHullJob { target_fleet, .. } = &mut operation {
            *target_fleet = None;
        }
        let q = fixture::queue(vec![operation]);
        let new = q[0].revision.created_entities()[0].2;
        let n = u128::from_be_bytes(new.0);
        let keys = match (equal, position) {
            (true, 0) => [n, n + 1, n + 2],
            (true, 1) => [n - 1, n, n + 1],
            (true, 2) => [n - 2, n - 1, n],
            (false, 0) => [n + 1, n + 2, n + 3],
            (false, 1) => [n - 2, n - 1, n + 1],
            (false, 2) => [n - 3, n - 2, n - 1],
            _ => unreachable!(),
        };
        let mut state = LivingGalaxyStateV2 {
            civilizations: templates.civilizations.clone(),
            ..Default::default()
        };
        macro_rules! rows {
            ($field:ident) => {{
                let row = templates.$field.as_slice()[0].clone();
                let values = keys.map(|key| {
                    let mut row = row.clone();
                    row.id = LivingEntityIdV2(key.to_be_bytes());
                    row
                });
                let mut source = Vec::with_capacity(capacity);
                source.extend(values);
                state.$field = LivingSortedVecV2::new(source).unwrap();
            }};
        }
        match kind {
            0 => rows!(facilities),
            1 => rows!(construction_jobs),
            2 => rows!(hull_jobs),
            3 => rows!(fleets),
            4 => {
                let mut fleet = templates.fleets.as_slice()[0].clone();
                fleet.id = id(90);
                let hull = fleet.hulls.as_slice()[0].clone();
                let mut source = Vec::with_capacity(capacity);
                source.extend(keys.map(|key| {
                    let mut row = hull.clone();
                    row.id = LivingEntityIdV2(key.to_be_bytes());
                    row
                }));
                fleet.hulls = LivingSortedVecV2::new(source).unwrap();
                state.fleets = LivingSortedVecV2::new(vec![fleet]).unwrap();
            }
            5 => rows!(routes),
            6 => rows!(shipments),
            7 => rows!(hazards),
            _ => unreachable!(),
        }
        if kind == 0 {
            for index in 0..3 {
                state
                    .facilities
                    .record_at_mut(index)
                    .unwrap()
                    .blocked_reasons = LivingSortedVecV2::new(vec![
                    LivingBlockedReasonV2::Disabled,
                    LivingBlockedReasonV2::MissingDeposit {
                        required: 2,
                        available: 1,
                    },
                ])
                .unwrap();
            }
        }
        if kind == 3 || kind == 4 {
            for index in 0..state.fleets.as_slice().len() {
                let fleet = state.fleets.record_at_mut(index).unwrap();
                fleet.order = Some(LivingFleetOrderV2 {
                    kind: LivingFleetOrderKindV2::Move,
                    target_world: Some(id(9)),
                    not_before_tick: LivingTickV2(1),
                    lane_path: vec![id(8), id(9)],
                });
                if kind == 3 {
                    let hull = fleet.hulls.as_slice()[0].clone();
                    fleet.hulls = LivingSortedVecV2::new(
                        (0..3)
                            .map(|n| {
                                let mut row = hull.clone();
                                row.id = id(100 + n);
                                row
                            })
                            .collect(),
                    )
                    .unwrap();
                }
            }
        }
        let mut creator = Box::new(LivingQueuedCreatorJobV2::begin(
            state,
            Arc::clone(&q),
            Arc::clone(&catalog),
        ));
        let mut events = LivingPendingEventsV2::new();
        events
            .record(
                LivingEventProvenanceV2::Creator {
                    revision: q[0].revision.id,
                },
                LivingEventKindV2::CreatorIntervention,
            )
            .unwrap();
        // Fresh fixture collector clone creates exactly full length-one backing.
        creator.events = events.clone();
        (creator, q, catalog)
    }
    #[test]
    fn creator_parent_all_eight_replacements_all_positions_full_and_spare_match_rules() {
        for kind in 0..8 {
            for equal in [false, true] {
                for position in 0..3 {
                    for capacity in [3, 19] {
                        for schedule in SCHEDULES {
                            let (mut creator, queue, catalog) =
                                source_at(kind, equal, position, capacity);
                            let mut expected = creator.state.clone();
                            let mut events = creator.events.clone();
                            let (effects, recovery) = apply_revision_with_recoveries(
                                &mut expected,
                                &queue[0].revision,
                                &catalog,
                            )
                            .unwrap();
                            assert!(effects.is_empty() && recovery.is_empty());
                            events
                                .record(
                                    LivingEventProvenanceV2::Creator {
                                        revision: queue[0].revision.id,
                                    },
                                    LivingEventKindV2::CreatorIntervention,
                                )
                                .unwrap();
                            for poll in 0..10000 {
                                assert!(poll < 9999, "creator replacement did not finish");
                                let b = schedule[poll % schedule.len()];
                                let progress = creator.poll(b).unwrap();
                                assert!(creator.last_poll_visits <= b);
                                assert_ne!(
                                    progress,
                                    LivingStepProgressV2::NeedsSynchronousCreatorOperation
                                );
                                if progress == LivingStepProgressV2::NeedsSynchronousCompletion {
                                    let (actual, actual_events) =
                                        creator.take_ready_result().unwrap();
                                    assert_eq!(
                                        actual, expected,
                                        "kind {kind} equal {equal} position {position} capacity {capacity}"
                                    );
                                    assert_eq!(actual_events, events);
                                    let retirement = LivingPublicationRetirementJobV2::begin(
                                        Owner::Creator(creator),
                                        LivingPublicationRetirementPolicyV2::ReleaseHandle,
                                    );
                                    drain(retirement, schedule);
                                    break;
                                }
                            }
                        }
                    }
                }
            }
        }
    }
    fn drain(mut job: LivingPublicationRetirementJobV2, schedule: &[usize]) -> u64 {
        for i in 0..10000 {
            let b = schedule[i % schedule.len()];
            let progress = job.poll(b).unwrap();
            assert!(job.last_poll_visits() <= b);
            if progress == LivingPublicationRetirementProgressV2::Ready {
                return job.total_visits();
            }
        }
        panic!("retirement did not finish")
    }
    type Allocation = (usize, usize, Option<usize>, usize);
    fn allocations(owner: &LivingQueuedCreatorJobV2) -> Vec<Allocation> {
        let mut out = Vec::new();
        observe_creator(owner, &mut out);
        out.retain(|row| row.2 != Some(0));
        out
    }
    fn retained(probe: Allocation, current: &[Allocation]) -> bool {
        let (base, len, capacity, size) = probe;
        current.iter().any(|&(pointer, left, cap, element)| {
            if element != size || size == 0 {
                return false;
            }
            if let Some(cap) = cap {
                pointer == base && capacity == Some(cap) && left <= len
            } else {
                pointer >= base
                    && pointer <= base + len * size
                    && (pointer - base) % size == 0
                    && left == len - (pointer - base) / size
            }
        })
    }
    fn released_allocation_reappeared(probe: Allocation, current: &[Allocation]) -> bool {
        current.iter().any(|row| {
            (row.2.is_some() || row.1 != 0) && retained(probe, std::slice::from_ref(row))
        })
    }

    #[test]
    fn released_allocation_observer_rejects_ambiguous_empty_iterator_endpoints() {
        let original = (64, 2, Some(2), 56);
        // Original [64, 176) and an adjacent buffer can share this empty end.
        let empty = (176, 0, None, 56);
        assert!(retained(original, &[empty]));
        assert!(!released_allocation_reappeared(original, &[empty]));
        assert!(released_allocation_reappeared(
            original,
            &[(120, 1, None, 56)]
        ));
        assert!(released_allocation_reappeared(
            original,
            &[(64, 0, Some(2), 56)]
        ));
    }

    #[test]
    fn creator_allocation_observer_distinguishes_iterator_tail_from_vec_base() {
        // Forward storage/filter/replacement converts Vec to IntoIter. Cleanup
        // decomposes each representation; an iterator never becomes a Vec.
        for (probe, current, expected) in [
            ((64, 2, Some(4), 56), (64, 2, Some(4), 56), true),
            ((64, 2, Some(4), 56), (64, 1, Some(4), 56), true),
            ((64, 2, Some(4), 56), (120, 1, None, 56), true),
            ((64, 2, None, 56), (64, 2, None, 56), true),
            ((64, 2, None, 56), (120, 1, None, 56), true),
            ((176, 0, None, 56), (176, 0, None, 56), true),
            ((176, 0, None, 56), (176, 0, Some(4), 56), false),
            ((64, 2, Some(4), 56), (64, 2, Some(8), 56), false),
            ((64, 2, None, 56), (120, 2, None, 56), false),
            ((64, 2, None, 56), (64, 2, None, 64), false),
        ] {
            assert_eq!(
                retained(probe, &[current]),
                expected,
                "{probe:?} -> {current:?}"
            );
        }
    }

    fn probe_deadlines(
        owner: &LivingQueuedCreatorJobV2,
        kind: usize,
        equal: bool,
    ) -> Vec<(Allocation, usize)> {
        let (source_end, first_end, second_end) = allocation_deadlines(kind, equal);
        macro_rules! pointer {
            ($field:ident) => {
                owner.state.$field.as_slice().as_ptr() as usize
            };
        }
        let source = match kind {
            0 => pointer!(facilities),
            1 => pointer!(construction_jobs),
            2 => pointer!(hull_jobs),
            3 => pointer!(fleets),
            4 => owner.state.fleets.as_slice()[0].hulls.as_slice().as_ptr() as usize,
            5 => pointer!(routes),
            6 => pointer!(shipments),
            7 => pointer!(hazards),
            _ => unreachable!(),
        };
        let displaced = if equal && kind == 0 {
            Some((
                owner.state.facilities.as_slice()[1]
                    .blocked_reasons
                    .as_slice()
                    .as_ptr() as usize,
                None,
            ))
        } else if equal && kind == 3 {
            let row = &owner.state.fleets.as_slice()[1];
            Some((
                row.order.as_ref().unwrap().lane_path.as_ptr() as usize,
                Some(row.hulls.as_slice().as_ptr() as usize),
            ))
        } else {
            None
        };
        allocations(owner)
            .into_iter()
            .map(|probe| {
                let end = if probe.0 == owner.events.payloads().as_ptr() as usize {
                    // Existing analytic-v4 append tail: 63 visits follow the
                    // original entry backing release; no forward layout change.
                    cuts(kind, equal).len() - 64
                } else if probe.0 == source {
                    source_end
                } else if displaced.is_some_and(|(first, _)| first == probe.0) {
                    first_end
                } else if displaced.is_some_and(|(_, second)| second == Some(probe.0)) {
                    second_end
                } else {
                    usize::MAX
                };
                (probe, end)
            })
            .collect()
    }
    fn drain_observed(
        mut job: LivingPublicationRetirementJobV2,
        schedule: &[usize],
        probes: &[Allocation],
        case: (usize, bool, usize),
    ) -> u64 {
        let mut live = vec![true; probes.len()];
        for i in 0..10000 {
            let mut current = Vec::new();
            job.creator_allocations(&mut current);
            for (index, &probe) in probes.iter().enumerate() {
                let now = retained(probe, &current);
                if i == 0 {
                    assert!(now, "original allocation missing at cleanup handoff");
                }
                if live[index] {
                    live[index] = now;
                } else {
                    // An exhausted IntoIter exposes only its one-past-end
                    // pointer. A different adjacent allocation can end there
                    // too, so that empty slice cannot identify a released owner.
                    // Keep checking Vec bases and nonempty iterator tails.
                    assert!(
                        !released_allocation_reappeared(probe, &current),
                        "released original allocation reappeared case {case:?} cleanup poll {i} probe {probe:?} current {current:?}"
                    );
                }
            }
            let b = schedule[i % schedule.len()];
            let progress = job.poll(b).unwrap();
            assert!(job.last_poll_visits() <= b);
            if progress == LivingPublicationRetirementProgressV2::Ready {
                let mut current = Vec::new();
                job.creator_allocations(&mut current);
                assert!(probes.iter().all(|&probe| !retained(probe, &current)));
                return job.total_visits();
            }
        }
        panic!("retirement did not finish")
    }
    #[test]
    fn creator_parent_all_constructor_cuts_match_preexecution_literal_ownership() {
        for kind in 0..8 {
            for equal in [false, true] {
                for (cut, &expected) in cuts(kind, equal).iter().enumerate() {
                    for schedule in SCHEDULES {
                        let (creator, queue, catalog) = source(kind, equal);
                        let anchor = crate::living::builtin_living_sandbox_v2().unwrap();
                        let mut step = captured_step(creator, anchor.capture_boundary().unwrap());
                        let step_pointer = &*step as *const _;
                        let creator = step.creator.as_mut().unwrap();
                        let pointer = &**creator as *const LivingQueuedCreatorJobV2;
                        let probes = probe_deadlines(creator, kind, equal);
                        let mut visits = 0;
                        let mut polls = 0;
                        while visits < cut {
                            assert!(polls <= cut + 1, "finite forward cut");
                            let budget = schedule[polls % schedule.len()].min(cut - visits);
                            polls += 1;
                            let progress = creator.poll(budget).unwrap();
                            assert!(
                                creator.last_poll_visits > 0 && creator.last_poll_visits <= budget,
                                "unexpected zero progress at cut {cut}"
                            );
                            visits += creator.last_poll_visits;
                            assert!(progress == LivingStepProgressV2::Pending || visits == cut);
                        }
                        let current = allocations(creator);
                        for &(probe, deadline) in &probes {
                            assert_eq!(
                                retained(probe, &current),
                                cut < deadline,
                                "original allocation kind {kind} equal {equal} cut {cut} deadline {deadline}"
                            );
                        }
                        assert!(Arc::ptr_eq(&creator.queue, &queue));
                        assert!(Arc::ptr_eq(&creator.catalog, &catalog));
                        step.cancel();
                        assert_eq!(step.poll(1), Err(LivingStepJobErrorV2::Cancelled));
                        assert_eq!(step.last_poll_visits(), 0);
                        assert!(matches!(
                            step.take_synchronous_preparation(&anchor),
                            Err(LivingStepJobErrorV2::Cancelled)
                        ));
                        assert!(std::ptr::eq(&*step, step_pointer));
                        assert!(std::ptr::eq(&**step.creator.as_ref().unwrap(), pointer));
                        let retirement = LivingPublicationRetirementJobV2::begin(
                            Owner::Step(StepOwner::Boxed(step)),
                            LivingPublicationRetirementPolicyV2::ReleaseHandle,
                        );
                        assert!(
                            matches!(&retirement.root,Some(Owner::Step(StepOwner::Boxed(owner))) if std::ptr::eq(&**owner,step_pointer)&&std::ptr::eq(&**owner.creator.as_ref().unwrap(),pointer))
                        );
                        assert_eq!(
                            drain_observed(retirement, schedule, &current, (kind, equal, cut)),
                            expected + 52 + 12 + 3 + 6, // E2/E3 retained; E4 two absent owners.
                            "kind {kind} equal {equal} cut {cut}"
                        );
                    }
                }
            }
        }
    }
    fn captured_step(
        creator: Box<LivingQueuedCreatorJobV2>,
        capture: LivingBoundaryCaptureV2,
    ) -> Box<LivingStepJobV2> {
        Box::new(LivingStepJobV2 {
            capture: capture.into(),
            copy: None,
            hash: None,
            candidate: None,
            source_digest: None,
            creator: Some(creator),
            scheduled: None,
            conflict: None,
            production: None,
            observation: None,
            observation_events: None,
            diplomacy: None,
            intents: None,
            dispatch: None,
            completed_output: None,
            applied_revisions: None,
            sealing: None,
            sealed_receipts: None,
            preparation_capacity: false,
            production_fault: None,
            production_handoff: false,
            fault: None,
            cancelled: false,
            delivered: false,
            retirement: Default::default(),
            last_poll_visits: 0,
            total_visits: 0,
        })
    }
    fn insertion_ready(owner: &QueuedCreatorInsertion) -> bool {
        match owner {
            QueuedCreatorInsertion::Removal(job) => job.ready(),
            QueuedCreatorInsertion::FleetStructural(job) => job.ready(),
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
        }
    }
    fn borrow_insertion(owner: &mut QueuedCreatorInsertion) -> Box<dyn RetirementPoll> {
        let policy = LivingStateRetirementPolicyV2::ReleaseHandle;
        match owner {
            QueuedCreatorInsertion::Removal(_) | QueuedCreatorInsertion::FleetStructural(_) => {
                panic!(
                    "structural output installs into its anchored parent, not the constructor helper"
                )
            }
            QueuedCreatorInsertion::System(job) => {
                retirement_node(job.take_ready_result().unwrap(), policy)
            }
            QueuedCreatorInsertion::Star(job) => {
                retirement_node(job.take_ready_result().unwrap(), policy)
            }
            QueuedCreatorInsertion::World(job) => {
                retirement_node(job.take_ready_result().unwrap(), policy)
            }
            QueuedCreatorInsertion::Lane(job) => {
                retirement_node(job.take_ready_result().unwrap(), policy)
            }
            QueuedCreatorInsertion::Civilization(job) => {
                retirement_node(job.take_ready_result().unwrap(), policy)
            }
            QueuedCreatorInsertion::Deposit(job) => {
                retirement_node(job.take_ready_result().unwrap(), policy)
            }
            QueuedCreatorInsertion::Facility(job) => {
                retirement_node(job.take_ready_result().unwrap(), policy)
            }
            QueuedCreatorInsertion::ConstructionJob(job) => {
                retirement_node(job.take_ready_result().unwrap(), policy)
            }
            QueuedCreatorInsertion::HullJob(job) => {
                retirement_node(job.take_ready_result().unwrap(), policy)
            }
            QueuedCreatorInsertion::Fleet(job) => {
                retirement_node(job.take_ready_result().unwrap(), policy)
            }
            QueuedCreatorInsertion::Hull(job) => {
                retirement_node(job.take_ready_result().unwrap(), policy)
            }
            QueuedCreatorInsertion::Route(job) => {
                retirement_node(job.take_ready_result().unwrap(), policy)
            }
            QueuedCreatorInsertion::Shipment(job) => {
                retirement_node(job.take_ready_result().unwrap(), policy)
            }
            QueuedCreatorInsertion::Hazard(job) => {
                retirement_node(job.take_ready_result().unwrap(), policy)
            }
            QueuedCreatorInsertion::Relation(job) => {
                retirement_node(job.take_ready_result().unwrap(), policy)
            }
            QueuedCreatorInsertion::Agreement(job) => {
                retirement_node(job.take_ready_result().unwrap(), policy)
            }
            QueuedCreatorInsertion::War(job) => {
                retirement_node(job.take_ready_result().unwrap(), policy)
            }
            QueuedCreatorInsertion::Colony(job) => {
                retirement_node(job.take_ready_result().unwrap(), policy)
            }
        }
    }
    #[test]
    fn creator_parent_genuine_extraction_and_capacity_refusal_preserve_original_partition() {
        for kind in 0..8 {
            for equal in [false, true] {
                for control in 0..3 {
                    for schedule in SCHEDULES {
                        let (mut creator, _queue, _catalog) = source(kind, equal);
                        let creator_pointer = &*creator as *const _;
                        let mut external: Option<Box<dyn RetirementPoll>> = None;
                        for _ in 0..10000 {
                            let stopped=match control {
                        0=>creator.insertion.as_ref().is_some_and(insertion_ready),
                        1=>creator.append.as_ref().is_some_and(|child|child.status()==crate::living::receipt::LivingPendingEventAppendStatusV2::Ready),
                        _=>creator.append.as_mut().is_some_and(|child|child.inject_entry_capacity_failure()),
                    };
                            if stopped {
                                break;
                            }
                            assert_eq!(creator.poll(1).unwrap(), LivingStepProgressV2::Pending);
                        }
                        match control {
                            0 => {
                                external =
                                    Some(borrow_insertion(creator.insertion.as_mut().unwrap()))
                            }
                            1 => {
                                external = Some(retirement_node(
                                    creator
                                        .append
                                        .as_mut()
                                        .unwrap()
                                        .take_ready_result()
                                        .unwrap()
                                        .0,
                                    LivingStateRetirementPolicyV2::ReleaseHandle,
                                ))
                            }
                            _ => {}
                        }
                        let anchor = crate::living::builtin_living_sandbox_v2().unwrap();
                        // Private captured-stage fixture; capture remains authentic and
                        // externally anchored. Public begin_step integration is separate.
                        let mut step = captured_step(creator, anchor.capture_boundary().unwrap());
                        let step_pointer = &*step as *const _;
                        let expected_error = if control == 2 {
                            LivingStepJobErrorV2::PreparationCapacity
                        } else {
                            LivingStepJobErrorV2::Fault(LivingDeterministicFaultV2::Invariant)
                        };
                        assert_eq!(step.poll(1), Err(expected_error.clone()));
                        assert_eq!(step.last_poll_visits(), 1);
                        assert!(std::ptr::eq(
                            &**step.creator.as_ref().unwrap(),
                            creator_pointer
                        ));
                        assert!(step.scheduled.is_none());
                        for b in [1, 64, 1024] {
                            assert_eq!(step.poll(b), Err(expected_error.clone()));
                            assert_eq!(step.last_poll_visits(), 0);
                        }
                        step.cancel();
                        assert_eq!(step.poll(1), Err(expected_error));
                        assert_eq!(step.last_poll_visits(), 0);
                        let retirement = LivingPublicationRetirementJobV2::begin(
                            Owner::Step(StepOwner::Boxed(step)),
                            LivingPublicationRetirementPolicyV2::ReleaseHandle,
                        );
                        assert!(
                            matches!(&retirement.root,Some(Owner::Step(StepOwner::Boxed(owner))) if std::ptr::eq(&**owner,step_pointer))
                        );
                        let (step_expected, external_expected) =
                            refusal_totals(kind, equal, control);
                        assert_eq!(
                            drain(retirement, schedule),
                            step_expected + 12 + 3 + 6, // E2/E3 retained; E4 two absent owners.
                            "kind {kind} equal {equal} control {control}"
                        );
                        let mut count = 0;
                        if let Some(mut owner) = external {
                            for attempt in 0..10000 {
                                assert!(attempt < 9999, "external original cleanup did not finish");
                                match owner.retirement_step() {
                                    RetirementStep::Work => count += 1,
                                    RetirementStep::Done => {
                                        drop(owner);
                                        count += 1;
                                        break;
                                    }
                                    RetirementStep::Deferred(_) => {
                                        panic!("unexpected deferred output")
                                    }
                                }
                            }
                        }
                        assert_eq!(count, external_expected);
                    }
                }
            }
        }
    }
    fn refusal_totals(kind: usize, equal: bool, control: usize) -> (u64, u64) {
        match (kind, equal, control) {
            (0, false, 0) => (188, 31),
            (0, false, 2) => (255, 0),
            (0, false, 1) => (231, 23),
            (0, true, 0) => (188, 24),
            (0, true, 2) => (249, 0),
            (0, true, 1) => (225, 23),
            (1, false, 0) => (188, 17),
            (1, false, 2) => (237, 0),
            (1, false, 1) => (213, 23),
            (1, true, 0) => (188, 14),
            (1, true, 2) => (236, 0),
            (1, true, 1) => (212, 23),
            (2, false, 0) => (188, 17),
            (2, false, 2) => (237, 0),
            (2, false, 1) => (213, 23),
            (2, true, 0) => (188, 14),
            (2, true, 2) => (236, 0),
            (2, true, 1) => (212, 23),
            (3, false, 0) => (188, 46),
            (3, false, 2) => (270, 0),
            (3, false, 1) => (246, 23),
            (3, true, 0) => (188, 34),
            (3, true, 2) => (259, 0),
            (3, true, 1) => (235, 23),
            (4, false, 0) => (201, 17),
            (4, false, 2) => (245, 0),
            (4, false, 1) => (221, 23),
            (4, true, 0) => (201, 14),
            (4, true, 2) => (244, 0),
            (4, true, 1) => (220, 23),
            (5, false, 0) => (188, 17),
            (5, false, 2) => (237, 0),
            (5, false, 1) => (213, 23),
            (5, true, 0) => (188, 14),
            (5, true, 2) => (236, 0),
            (5, true, 1) => (212, 23),
            (6, false, 0) => (188, 17),
            (6, false, 2) => (237, 0),
            (6, false, 1) => (213, 23),
            (6, true, 0) => (188, 14),
            (6, true, 2) => (236, 0),
            (6, true, 1) => (212, 23),
            (7, false, 0) => (188, 17),
            (7, false, 2) => (237, 0),
            (7, false, 1) => (213, 23),
            (7, true, 0) => (188, 14),
            (7, true, 2) => (236, 0),
            (7, true, 1) => (212, 23),
            _ => unreachable!(),
        }
    }
    fn allocation_deadlines(kind: usize, equal: bool) -> (usize, usize, usize) {
        match (kind, equal) {
            (0, false) => (42, 0, 0),
            (0, true) => (51, 42, 0),
            (1, false) => (42, 0, 0),
            (1, true) => (47, 0, 0),
            (2, false) => (42, 0, 0),
            (2, true) => (47, 0, 0),
            (3, false) => (42, 0, 0),
            (3, true) => (56, 42, 47),
            (4, false) => (42, 0, 0),
            (4, true) => (47, 0, 0),
            (5, false) => (42, 0, 0),
            (5, true) => (47, 0, 0),
            (6, false) => (42, 0, 0),
            (6, true) => (47, 0, 0),
            (7, false) => (42, 0, 0),
            (7, true) => (47, 0, 0),
            _ => unreachable!(),
        }
    }
    #[test]
    fn colony_parent_real_child_extraction_and_capacity_faults_are_sticky_and_retained() {
        use crate::living::simulation::colony_cursor_tests as colony;
        // Existing independent arithmetic-v5 fault ledger; never discovered by polling.
        const FAULT_CUTS: [usize; 7] = [58, 57, 112, 164, 11, 107, 156];
        const FAULT_TOTALS: &[(u64, u64)] = &[
            (202, 24),
            (199, 19),
            (212, 2),
            (236, 26),
            (241, 0),
            (212, 0),
            (263, 0),
        ];
        for (control, &(step_expected, external_expected)) in FAULT_TOTALS.iter().enumerate() {
            for schedule in colony::SCHEDULES {
                let kind = if control == 1 || control == 4 { 3 } else { 0 };
                let (mut creator, _queue, _catalog) = colony::source(kind);
                let pointer = &*creator as *const _;
                let cut = FAULT_CUTS[control];
                let mut forward_visits = 0;
                let mut schedule_index = 0;
                for _attempt in 0..10_000 {
                    if forward_visits == cut {
                        break;
                    }
                    let budget =
                        schedule[schedule_index % schedule.len()].min(cut - forward_visits);
                    schedule_index += 1;
                    assert_eq!(creator.poll(budget).unwrap(), LivingStepProgressV2::Pending);
                    assert!(creator.last_poll_visits > 0 && creator.last_poll_visits <= budget);
                    forward_visits += creator.last_poll_visits;
                }
                assert_eq!(
                    forward_visits, cut,
                    "finite independent fault-boundary guard"
                );
                let stopped = match control {
                    0 => creator.insertion.as_ref().is_some_and(insertion_ready),
                    1 => creator.colony_removal.as_ref().is_some_and(|j| j.ready()),
                    2 => creator.recovery_storage.as_ref().is_some_and(|j| j.ready()),
                    3 => creator.append.as_ref().is_some_and(|j| {
                        j.status()
                            == crate::living::receipt::LivingPendingEventAppendStatusV2::Ready
                    }),
                    4 => creator
                        .colony_removal
                        .as_mut()
                        .is_some_and(|j| j.inject_creator_capacity_failure()),
                    5 => creator
                        .recovery_storage
                        .as_mut()
                        .is_some_and(|j| j.inject_creator_capacity_failure()),
                    _ => creator
                        .append
                        .as_mut()
                        .is_some_and(|j| j.inject_entry_capacity_failure()),
                };
                assert!(
                    stopped,
                    "original child must match the independently pinned boundary"
                );
                let policy = LivingStateRetirementPolicyV2::ReleaseHandle;
                let external = match control {
                    0 => Some(borrow_insertion(creator.insertion.as_mut().unwrap())),
                    1 => Some(retirement_node(
                        creator
                            .colony_removal
                            .as_mut()
                            .unwrap()
                            .take_ready_result()
                            .unwrap(),
                        policy,
                    )),
                    2 => Some(retirement_node(
                        creator
                            .recovery_storage
                            .as_mut()
                            .unwrap()
                            .take_ready_result()
                            .unwrap(),
                        policy,
                    )),
                    3 => Some(retirement_node(
                        creator
                            .append
                            .as_mut()
                            .unwrap()
                            .take_ready_result()
                            .unwrap()
                            .0,
                        policy,
                    )),
                    _ => None,
                };
                let anchor = crate::living::builtin_living_sandbox_v2().unwrap();
                let mut step = captured_step(creator, anchor.capture_boundary().unwrap());
                let expected = if control < 4 {
                    LivingStepJobErrorV2::Fault(LivingDeterministicFaultV2::Invariant)
                } else {
                    LivingStepJobErrorV2::PreparationCapacity
                };
                // Carry the forward schedule cursor into the first failure. A
                // mixed cycle may naturally return to index0; never reset it here.
                let failure_budget = schedule[schedule_index % schedule.len()];
                let failure_start = step.total_visits();
                assert_eq!(step.poll(failure_budget), Err(expected.clone()));
                assert_eq!(step.last_poll_visits(), 1);
                assert_eq!(step.total_visits() - failure_start, 1);
                assert!(std::ptr::eq(&**step.creator.as_ref().unwrap(), pointer));
                let total = step.total_visits();
                for budget in [1, 64, 1024] {
                    assert_eq!(step.poll(budget), Err(expected.clone()));
                    assert_eq!(step.last_poll_visits(), 0);
                    assert_eq!(step.total_visits(), total);
                }
                for budget in [0, 1025] {
                    assert_eq!(step.poll(budget), Err(LivingStepJobErrorV2::Budget));
                    assert_eq!(step.last_poll_visits(), 0);
                    assert_eq!(step.total_visits(), total);
                }
                let current = allocations(step.creator.as_ref().unwrap());
                step.cancel();
                for budget in [0, 1, 64, 1024, 1025] {
                    assert_eq!(step.poll(budget), Err(expected.clone()));
                    assert_eq!(step.last_poll_visits(), 0);
                    assert_eq!(step.total_visits(), total);
                }
                assert_eq!(
                    drain_observed(
                        step.into_boxed_retirement(
                            LivingPublicationRetirementPolicyV2::ReleaseHandle
                        ),
                        schedule,
                        &current,
                        (kind, false, control)
                    ),
                    step_expected + 3 + 6 // E3/E4 absent owners, independently pinned.
                );
                if let Some(mut external) = external {
                    let mut visits = 0;
                    let mut done = false;
                    for _ in 0..10_000 {
                        match external.retirement_step() {
                            RetirementStep::Done => {
                                visits += 1;
                                done = true;
                                break;
                            }
                            RetirementStep::Work => {
                                visits += 1;
                            }
                            RetirementStep::Deferred(_) => panic!("unexpected external wait"),
                        }
                    }
                    assert!(done, "finite original extracted output cleanup");
                    assert_eq!(visits, external_expected);
                }
            }
        }
    }
    // Frozen independent literal arithmetic-v5, prepared before any E2 execution.
    const COLONY_CUTS: &[&[u64]] = &[
        &[
            132, 132, 135, 138, 138, 135, 135, 135, 135, 135, 173, 197, 203, 197, 196, 195, 194,
            193, 192, 191, 190, 189, 188, 187, 186, 185, 184, 183, 182, 181, 180, 179, 178, 177,
            176, 175, 174, 173, 172, 173, 173, 173, 173, 173, 172, 171, 170, 169, 168, 167, 166,
            165, 166, 166, 166, 166, 166, 165, 166, 163, 162, 161, 160, 159, 158, 157, 156, 155,
            154, 153, 152, 151, 150, 149, 148, 147, 146, 145, 144, 143, 142, 141, 140, 139, 138,
            137, 136, 135, 134, 133, 132, 133, 136, 139, 139, 139, 139, 139, 139, 138, 137, 136,
            135, 134, 133, 132, 133, 160, 160, 163, 160, 159, 160, 157, 156, 155, 154, 153, 152,
            151, 150, 149, 148, 147, 146, 145, 144, 143, 142, 141, 140, 139, 138, 137, 136, 135,
            134, 133, 132, 133, 136, 139, 139, 139, 139, 139, 138, 137, 136, 135, 134, 133, 132,
            133, 181, 181, 184, 211, 214, 214, 211, 210, 208, 208, 208, 190, 189, 188, 187, 186,
            185, 184, 183, 182, 181, 180, 179, 178, 177, 176, 175, 174, 173, 172, 171, 170, 169,
            168, 167, 166, 165, 164, 163, 162, 161, 160, 159, 158, 157, 156, 155, 154, 153, 152,
            151, 150, 149, 148, 147, 146, 145, 144, 143, 142, 141, 140, 139, 138, 137, 136, 135,
            136, 136,
        ],
        &[
            132, 132, 135, 138, 138, 135, 135, 135, 135, 135, 135, 135, 138, 141, 141, 141, 141,
            141, 141, 140, 139, 138, 137, 136, 135, 134, 135, 162, 162, 165, 162, 161, 162, 159,
            158, 157, 156, 155, 154, 153, 152, 151, 150, 149, 148, 147, 146, 145, 144, 143, 142,
            141, 140, 139, 138, 137, 136, 135, 134, 135, 138, 141, 141, 141, 141, 141, 140, 139,
            138, 137, 136, 135, 134, 135, 183, 183, 186, 213, 216, 216, 213, 212, 210, 210, 210,
            192, 191, 190, 189, 188, 187, 186, 185, 184, 183, 182, 181, 180, 179, 178, 177, 176,
            175, 174, 173, 172, 171, 170, 169, 168, 167, 166, 165, 164, 163, 162, 161, 160, 159,
            158, 157, 156, 155, 154, 153, 152, 151, 150, 149, 148, 147, 146, 145, 144, 143, 142,
            141, 140, 139, 138, 137, 138, 138,
        ],
        &[
            132, 132, 135, 138, 138, 135, 135, 135, 135, 135, 135, 165, 189, 195, 189, 188, 187,
            186, 185, 184, 183, 182, 181, 180, 179, 178, 177, 176, 175, 174, 173, 172, 171, 170,
            169, 168, 167, 166, 165, 164, 165, 165, 165, 165, 165, 164, 163, 162, 161, 160, 159,
            158, 157, 158, 158, 158, 158, 157, 158, 156, 155, 154, 153, 152, 151, 150, 149, 148,
            147, 146, 145, 144, 143, 142, 141, 140, 139, 138, 137, 136, 135, 134, 133, 132, 131,
            130, 129, 128, 129, 132, 132, 132, 132, 132, 132, 131, 130, 129, 128, 129, 156, 156,
            159, 156, 155, 156, 153, 152, 151, 150, 149, 148, 147, 146, 145, 144, 143, 142, 141,
            140, 139, 138, 137, 136, 135, 134, 133, 132, 131, 130, 129, 128, 129, 132, 132, 129,
            129, 128, 127, 126, 125, 126, 174, 174, 177, 204, 207, 207, 204, 203, 201, 201, 201,
            183, 182, 181, 180, 179, 178, 177, 176, 175, 174, 173, 172, 171, 170, 169, 168, 167,
            166, 165, 164, 163, 162, 161, 160, 159, 158, 157, 156, 155, 154, 153, 152, 151, 150,
            149, 148, 147, 146, 145, 144, 143, 142, 141, 140, 139, 138, 137, 136, 135, 134, 133,
            132, 131, 130, 129, 128, 129, 129,
        ],
        &[
            132, 132, 135, 138, 138, 135, 135, 135, 135, 135, 165, 189, 195, 189, 188, 187, 186,
            185, 184, 183, 182, 181, 180, 179, 178, 177, 176, 175, 174, 173, 172, 171, 170, 169,
            168, 167, 166, 165, 164, 165, 165, 165, 165, 165, 164, 163, 162, 161, 160, 159, 158,
            157, 158, 158, 158, 158, 157, 158, 156, 155, 154, 153, 152, 151, 150, 149, 148, 147,
            146, 145, 144, 143, 142, 141, 140, 139, 138, 137, 136, 135, 134, 133, 132, 131, 130,
            129, 128, 129, 132, 132, 132, 132, 132, 132, 131, 130, 129, 128, 129, 156, 156, 159,
            156, 155, 156, 153, 152, 151, 150, 149, 148, 147, 146, 145, 144, 143, 142, 141, 140,
            139, 138, 137, 136, 135, 134, 133, 132, 131, 130, 129, 128, 129, 132, 132, 129, 129,
            128, 127, 126, 125, 126, 174, 174, 177, 204, 207, 207, 204, 203, 201, 201, 201, 183,
            182, 181, 180, 179, 178, 177, 176, 175, 174, 173, 172, 171, 170, 169, 168, 167, 166,
            165, 164, 163, 162, 161, 160, 159, 158, 157, 156, 155, 154, 153, 152, 151, 150, 149,
            148, 147, 146, 145, 144, 143, 142, 141, 140, 139, 138, 137, 136, 135, 134, 133, 132,
            131, 130, 129, 128, 129, 129,
        ],
        &[
            141, 141, 144, 147, 147, 144, 144, 144, 144, 144, 182, 206, 212, 206, 205, 204, 203,
            202, 201, 200, 199, 198, 197, 196, 195, 194, 193, 192, 191, 190, 189, 188, 187, 186,
            185, 184, 183, 182, 181, 182, 182, 182, 182, 182, 181, 180, 179, 178, 177, 176, 175,
            174, 175, 175, 175, 175, 175, 174, 175, 172, 171, 170, 169, 168, 167, 166, 165, 164,
            163, 162, 161, 160, 159, 158, 157, 156, 155, 154, 153, 152, 151, 150, 149, 148, 147,
            146, 145, 144, 143, 142, 141, 142, 145, 148, 148, 148, 148, 148, 148, 147, 146, 145,
            144, 143, 142, 141, 142, 169, 169, 172, 172, 172, 172, 169, 168, 169, 166, 165, 164,
            163, 162, 161, 160, 159, 158, 157, 156, 155, 154, 153, 152, 151, 150, 149, 148, 147,
            146, 145, 144, 143, 142, 141, 142, 145, 148, 148, 148, 148, 148, 147, 146, 145, 144,
            143, 142, 141, 142, 190, 190, 193, 220, 223, 223, 220, 219, 217, 217, 217, 199, 198,
            197, 196, 195, 194, 193, 192, 191, 190, 189, 188, 187, 186, 185, 184, 183, 182, 181,
            180, 179, 178, 177, 176, 175, 174, 173, 172, 171, 170, 169, 168, 167, 166, 165, 164,
            163, 162, 161, 160, 159, 158, 157, 156, 155, 154, 153, 152, 151, 150, 149, 148, 147,
            146, 145, 144, 145, 145,
        ],
    ];
    #[test]
    fn colony_parent_every_forward_cut_original_boxes_allocations_and_exact_retirement() {
        use crate::living::simulation::colony_cursor_tests as colony;
        for (kind, expected) in COLONY_CUTS.iter().enumerate() {
            for (cut, &count) in expected.iter().enumerate() {
                for schedule in colony::SCHEDULES {
                    let (creator, queue, catalog) = colony::source(kind);
                    let anchor = crate::living::builtin_living_sandbox_v2().unwrap();
                    let mut step = captured_step(creator, anchor.capture_boundary().unwrap());
                    let step_pointer = &*step as *const _;
                    let creator = step.creator.as_mut().unwrap();
                    let creator_pointer = &**creator as *const _;
                    let source_pointer = creator.state.colonies.as_slice().as_ptr() as usize;
                    let reasons_pointer = creator.state.colonies.as_slice()[1]
                        .hub
                        .blocked_reasons
                        .as_slice()
                        .as_ptr() as usize;
                    let entries_pointer = creator.events.payloads().as_ptr() as usize;
                    let recoveries_pointer = creator.events.creator_recoveries.as_ptr() as usize;
                    // Independent analytic-v5: displaced Option handoff + row
                    // child handoff + two reasons + reason backing = five visits.
                    let (source_end, reason_end, entries_end, recovery_end) = match kind {
                        0 => (57, 48, 161, usize::MAX),
                        1 => (usize::MAX, usize::MAX, 81, usize::MAX),
                        2 => (57, 49, 149, usize::MAX),
                        3 => (56, 48, 148, usize::MAX),
                        4 => (57, 48, 164, 114),
                        _ => unreachable!(),
                    };
                    let original: Vec<_> = allocations(creator)
                        .into_iter()
                        .map(|probe| {
                            let end = if probe.0 == source_pointer {
                                source_end
                            } else if probe.0 == reasons_pointer {
                                reason_end
                            } else if probe.0 == entries_pointer {
                                entries_end
                            } else if kind == 4 && probe.0 == recoveries_pointer {
                                recovery_end
                            } else {
                                usize::MAX
                            };
                            (probe, end)
                        })
                        .collect();
                    let mut visited = 0;
                    for attempt in 0..10000 {
                        if visited == cut {
                            break;
                        }
                        let budget = schedule[attempt % schedule.len()].min(cut - visited);
                        let progress = creator.poll(budget).unwrap();
                        assert!(creator.last_poll_visits > 0 && creator.last_poll_visits <= budget);
                        visited += creator.last_poll_visits;
                        assert!(progress == LivingStepProgressV2::Pending || visited == cut);
                    }
                    assert_eq!(visited, cut, "finite independent forward guard");
                    if cut == expected.len() - 1 {
                        assert_eq!(
                            creator.poll(1).unwrap(),
                            LivingStepProgressV2::NeedsSynchronousCompletion
                        );
                        assert_eq!(creator.last_poll_visits, 0);
                    } else {
                        assert!(creator.take_ready_result().is_err());
                    }
                    assert!(Arc::ptr_eq(&creator.queue, &queue));
                    assert!(Arc::ptr_eq(&creator.catalog, &catalog));
                    let current = allocations(creator);
                    for (probe, end) in original {
                        assert_eq!(
                            retained(probe, &current),
                            cut < end,
                            "original colony allocation kind={kind} cut={cut} release={end}"
                        );
                    }
                    step.cancel();
                    assert_eq!(step.poll(1), Err(LivingStepJobErrorV2::Cancelled));
                    assert_eq!(step.last_poll_visits(), 0);
                    assert!(matches!(
                        step.take_synchronous_preparation(&anchor),
                        Err(LivingStepJobErrorV2::Cancelled)
                    ));
                    assert!(std::ptr::eq(&*step, step_pointer));
                    assert!(std::ptr::eq(
                        &**step.creator.as_ref().unwrap(),
                        creator_pointer
                    ));
                    let retirement = step
                        .into_boxed_retirement(LivingPublicationRetirementPolicyV2::ReleaseHandle);
                    assert_eq!(
                        drain_observed(retirement, schedule, &current, (kind, false, cut)),
                        count + 52 + 3 + 6,
                        "independent colony retirement kind={kind} cut={cut}"
                    );
                }
            }
        }
    }
    // Immutable source-free E4 arithmetic-v3; no Rust counter input.
    const CANCELLATION_CUTS: &[&[u64]] = &[
        &[
            147, 147, 147, 147, 147, 159, 210, 210, 213, 210, 209, 210, 207, 206, 205, 204, 203,
            202, 201, 200, 199, 198, 197, 196, 195, 194, 193, 192, 191, 190, 189, 188, 187, 186,
            185, 184, 183, 182, 183, 183, 183, 186, 222, 246, 252, 246, 245, 244, 243, 242, 241,
            240, 239, 238, 237, 236, 235, 234, 233, 232, 231, 230, 229, 228, 227, 226, 225, 224,
            223, 222, 221, 222, 222, 222, 222, 222, 221, 220, 219, 218, 219, 219, 219, 219, 218,
            219, 215, 214, 213, 212, 211, 210, 209, 208, 207, 206, 205, 204, 203, 202, 201, 200,
            199, 198, 197, 196, 195, 194, 193, 192, 191, 190, 189, 188, 187, 186, 185, 184, 183,
            182, 181, 182, 212, 212, 215, 215, 212, 211, 212, 209, 208, 207, 206, 205, 204, 203,
            202, 201, 200, 199, 198, 197, 196, 195, 194, 193, 192, 191, 190, 189, 188, 187, 186,
            185, 184, 185, 185, 185, 184, 183, 182, 181, 180, 179, 178, 177, 176, 175, 174, 173,
            172, 173, 221, 221, 224, 251, 254, 254, 251, 250, 248, 248, 248, 230, 229, 228, 227,
            226, 225, 224, 223, 222, 221, 220, 219, 218, 217, 216, 215, 214, 213, 212, 211, 210,
            209, 208, 207, 206, 205, 204, 203, 202, 201, 200, 199, 198, 197, 196, 195, 194, 193,
            192, 191, 190, 189, 188, 187, 186, 185, 184, 183, 182, 181, 180, 179, 178, 177, 176,
            175, 176, 179, 179, 224, 224, 227, 254, 257, 257, 257, 254, 253, 251, 251, 251, 233,
            232, 231, 230, 229, 228, 227, 226, 225, 224, 223, 222, 221, 220, 219, 218, 217, 216,
            215, 214, 213, 212, 211, 210, 209, 208, 207, 206, 205, 204, 203, 202, 201, 200, 199,
            198, 197, 196, 195, 194, 193, 192, 191, 190, 189, 188, 187, 186, 185, 184, 183, 182,
            181, 180, 179, 178, 179, 179, 224, 224, 227, 251, 251, 251, 233, 232, 231, 230, 229,
            228, 227, 226, 225, 224, 223, 222, 221, 220, 219, 218, 217, 216, 215, 214, 213, 212,
            211, 210, 209, 208, 207, 206, 205, 204, 203, 202, 201, 200, 199, 198, 197, 196, 195,
            194, 193, 192, 191, 190, 189, 188, 187, 186, 185, 184, 183, 182, 181, 180, 179, 178,
            179, 179, 178, 177, 176, 175, 174, 173, 172, 171, 170, 169, 168, 167, 166, 165, 164,
            163, 162, 161, 160, 159, 158, 157, 156, 155, 154, 155, 155,
        ],
        &[
            147, 147, 147, 147, 147, 159, 210, 210, 213, 210, 209, 210, 207, 206, 205, 204, 203,
            202, 201, 200, 199, 198, 197, 196, 195, 194, 193, 192, 191, 190, 189, 188, 187, 186,
            185, 184, 183, 182, 183, 183, 183, 186, 222, 246, 252, 246, 245, 244, 243, 242, 241,
            240, 239, 238, 237, 236, 235, 234, 233, 232, 231, 230, 229, 228, 227, 226, 225, 224,
            223, 222, 221, 222, 222, 222, 222, 222, 221, 220, 219, 218, 219, 219, 219, 219, 218,
            219, 215, 214, 213, 212, 211, 210, 209, 208, 207, 206, 205, 204, 203, 202, 201, 200,
            199, 198, 197, 196, 195, 194, 193, 192, 191, 190, 189, 188, 187, 186, 185, 184, 183,
            182, 181, 182, 212, 212, 215, 215, 212, 211, 212, 209, 208, 207, 206, 205, 204, 203,
            202, 201, 200, 199, 198, 197, 196, 195, 194, 193, 192, 191, 190, 189, 188, 187, 186,
            185, 184, 185, 185, 185, 184, 183, 182, 181, 180, 179, 178, 177, 176, 175, 174, 173,
            172, 173, 221, 221, 224, 251, 254, 254, 251, 250, 248, 248, 248, 230, 229, 228, 227,
            226, 225, 224, 223, 222, 221, 220, 219, 218, 217, 216, 215, 214, 213, 212, 211, 210,
            209, 208, 207, 206, 205, 204, 203, 202, 201, 200, 199, 198, 197, 196, 195, 194, 193,
            192, 191, 190, 189, 188, 187, 186, 185, 184, 183, 182, 181, 180, 179, 178, 177, 176,
            175, 176, 179, 179, 224, 224, 227, 254, 257, 257, 257, 254, 253, 251, 251, 251, 233,
            232, 231, 230, 229, 228, 227, 226, 225, 224, 223, 222, 221, 220, 219, 218, 217, 216,
            215, 214, 213, 212, 211, 210, 209, 208, 207, 206, 205, 204, 203, 202, 201, 200, 199,
            198, 197, 196, 195, 194, 193, 192, 191, 190, 189, 188, 187, 186, 185, 184, 183, 182,
            181, 180, 179, 178, 179, 179, 224, 224, 227, 251, 251, 251, 233, 232, 231, 230, 229,
            228, 227, 226, 225, 224, 223, 222, 221, 220, 219, 218, 217, 216, 215, 214, 213, 212,
            211, 210, 209, 208, 207, 206, 205, 204, 203, 202, 201, 200, 199, 198, 197, 196, 195,
            194, 193, 192, 191, 190, 189, 188, 187, 186, 185, 184, 183, 182, 181, 180, 179, 178,
            179, 179, 178, 177, 176, 175, 174, 173, 172, 171, 170, 169, 168, 167, 166, 165, 164,
            163, 162, 161, 160, 159, 158, 157, 156, 155, 154, 155, 155,
        ],
        &[
            147, 147, 147, 147, 147, 162, 162, 162, 162, 162, 162, 162, 198, 222, 228, 222, 221,
            220, 219, 218, 217, 216, 215, 214, 213, 212, 211, 210, 209, 208, 207, 206, 205, 204,
            203, 202, 201, 200, 199, 198, 197, 198, 198, 198, 198, 198, 197, 196, 195, 194, 195,
            195, 195, 195, 195, 194, 195, 189, 188, 187, 186, 185, 184, 183, 182, 181, 180, 179,
            178, 177, 176, 175, 174, 173, 172, 171, 170, 169, 168, 167, 166, 165, 164, 163, 162,
            161, 160, 159, 158, 159, 159, 158, 157, 156, 155, 154, 153, 152, 151, 150, 149, 148,
            147, 146, 147, 195, 195, 198, 225, 228, 228, 225, 224, 222, 222, 222, 204, 203, 202,
            201, 200, 199, 198, 197, 196, 195, 194, 193, 192, 191, 190, 189, 188, 187, 186, 185,
            184, 183, 182, 181, 180, 179, 178, 177, 176, 175, 174, 173, 172, 171, 170, 169, 168,
            167, 166, 165, 164, 163, 162, 161, 160, 159, 158, 157, 156, 155, 154, 153, 152, 151,
            150, 149, 150, 150,
        ],
        &[
            147, 147, 147, 147, 147, 162, 198, 222, 228, 222, 221, 220, 219, 218, 217, 216, 215,
            214, 213, 212, 211, 210, 209, 208, 207, 206, 205, 204, 203, 202, 201, 200, 199, 198,
            197, 198, 198, 198, 198, 198, 197, 196, 195, 194, 195, 195, 195, 195, 195, 194, 195,
            189, 188, 187, 186, 185, 184, 183, 182, 181, 180, 179, 178, 177, 176, 175, 174, 173,
            172, 171, 170, 169, 168, 167, 166, 165, 164, 163, 162, 161, 160, 159, 158, 159, 159,
            158, 157, 156, 155, 154, 153, 152, 151, 150, 149, 148, 147, 146, 147, 195, 195, 198,
            225, 228, 228, 225, 224, 222, 222, 222, 204, 203, 202, 201, 200, 199, 198, 197, 196,
            195, 194, 193, 192, 191, 190, 189, 188, 187, 186, 185, 184, 183, 182, 181, 180, 179,
            178, 177, 176, 175, 174, 173, 172, 171, 170, 169, 168, 167, 166, 165, 164, 163, 162,
            161, 160, 159, 158, 157, 156, 155, 154, 153, 152, 151, 150, 149, 150, 150,
        ],
        &[
            147, 147, 147, 147, 147, 162, 162, 162, 162, 162, 162, 162, 198, 222, 228, 222, 221,
            220, 219, 218, 217, 216, 215, 214, 213, 212, 211, 210, 209, 208, 207, 206, 205, 204,
            203, 202, 201, 200, 199, 198, 197, 198, 198, 198, 198, 198, 197, 196, 195, 194, 195,
            195, 195, 195, 195, 194, 195, 189, 188, 187, 186, 185, 184, 183, 182, 181, 180, 179,
            178, 177, 176, 175, 174, 173, 172, 171, 170, 169, 168, 167, 166, 165, 164, 163, 162,
            161, 160, 159, 158, 159, 159, 158, 157, 156, 155, 154, 153, 152, 151, 150, 149, 148,
            147, 146, 147, 195, 195, 198, 225, 228, 228, 225, 224, 222, 222, 222, 204, 203, 202,
            201, 200, 199, 198, 197, 196, 195, 194, 193, 192, 191, 190, 189, 188, 187, 186, 185,
            184, 183, 182, 181, 180, 179, 178, 177, 176, 175, 174, 173, 172, 171, 170, 169, 168,
            167, 166, 165, 164, 163, 162, 161, 160, 159, 158, 157, 156, 155, 154, 153, 152, 151,
            150, 149, 150, 150,
        ],
        &[
            147, 147, 147, 147, 147, 159, 162, 198, 222, 228, 222, 221, 220, 219, 218, 217, 216,
            215, 214, 213, 212, 211, 210, 209, 208, 207, 206, 205, 204, 203, 202, 201, 200, 199,
            198, 197, 198, 198, 198, 198, 198, 197, 196, 195, 194, 195, 195, 195, 195, 194, 195,
            191, 190, 189, 188, 187, 186, 185, 184, 183, 182, 181, 180, 179, 178, 177, 176, 175,
            174, 173, 172, 171, 170, 169, 168, 167, 166, 165, 164, 163, 162, 161, 160, 159, 158,
            157, 158, 158, 157, 156, 155, 154, 153, 152, 151, 150, 149, 148, 147, 146, 145, 146,
            194, 194, 197, 224, 227, 227, 224, 223, 221, 221, 221, 203, 202, 201, 200, 199, 198,
            197, 196, 195, 194, 193, 192, 191, 190, 189, 188, 187, 186, 185, 184, 183, 182, 181,
            180, 179, 178, 177, 176, 175, 174, 173, 172, 171, 170, 169, 168, 167, 166, 165, 164,
            163, 162, 161, 160, 159, 158, 157, 156, 155, 154, 153, 152, 151, 150, 149, 148, 149,
            149,
        ],
        &[
            180, 183, 210, 210, 213, 213, 213, 210, 209, 210, 207, 206, 205, 204, 203, 202, 201,
            200, 199, 198, 197, 196, 195, 194, 193, 192, 191, 190, 189, 188, 187, 186, 185, 184,
            183, 182, 183, 183, 210, 210, 207, 206, 205, 204, 203, 202, 201, 200, 199, 198, 197,
            196, 195, 194, 193, 192, 191, 190, 189, 188, 187, 186, 185, 184, 183, 182, 183, 183,
            183, 182, 181, 180, 179, 180, 207, 207, 204, 203, 202, 201, 200, 199, 198, 197, 196,
            195, 194, 193, 192, 191, 190, 189, 188, 187, 186, 185, 184, 183, 182, 181, 180, 179,
            180, 180, 179, 180, 228, 228, 231, 258, 261, 261, 258, 257, 255, 255, 255, 237, 236,
            235, 234, 233, 232, 231, 230, 229, 228, 227, 226, 225, 224, 223, 222, 221, 220, 219,
            218, 217, 216, 215, 214, 213, 212, 211, 210, 209, 208, 207, 206, 205, 204, 203, 202,
            201, 200, 199, 198, 197, 196, 195, 194, 193, 192, 191, 190, 189, 188, 187, 186, 185,
            184, 183, 182, 183, 186, 186, 231, 231, 234, 261, 264, 264, 264, 261, 260, 258, 258,
            258, 240, 239, 238, 237, 236, 235, 234, 233, 232, 231, 230, 229, 228, 227, 226, 225,
            224, 223, 222, 221, 220, 219, 218, 217, 216, 215, 214, 213, 212, 211, 210, 209, 208,
            207, 206, 205, 204, 203, 202, 201, 200, 199, 198, 197, 196, 195, 194, 193, 192, 191,
            190, 189, 188, 187, 186, 185, 186, 186, 231, 231, 234, 258, 258, 258, 240, 239, 238,
            237, 236, 235, 234, 233, 232, 231, 230, 229, 228, 227, 226, 225, 224, 223, 222, 221,
            220, 219, 218, 217, 216, 215, 214, 213, 212, 211, 210, 209, 208, 207, 206, 205, 204,
            203, 202, 201, 200, 199, 198, 197, 196, 195, 194, 193, 192, 191, 190, 189, 188, 187,
            186, 185, 186, 186, 231, 231, 234, 261, 264, 264, 264, 264, 264, 261, 260, 258, 258,
            258, 240, 239, 238, 237, 236, 235, 234, 233, 232, 231, 230, 229, 228, 227, 226, 225,
            224, 223, 222, 221, 220, 219, 218, 217, 216, 215, 214, 213, 212, 211, 210, 209, 208,
            207, 206, 205, 204, 203, 202, 201, 200, 199, 198, 197, 196, 195, 194, 193, 192, 191,
            190, 189, 188, 187, 186, 185, 186, 186, 231, 231, 234, 258, 258, 258, 240, 239, 238,
            237, 236, 235, 234, 233, 232, 231, 230, 229, 228, 227, 226, 225, 224, 223, 222, 221,
            220, 219, 218, 217, 216, 215, 214, 213, 212, 211, 210, 209, 208, 207, 206, 205, 204,
            203, 202, 201, 200, 199, 198, 197, 196, 195, 194, 193, 192, 191, 190, 189, 188, 187,
            186, 185, 186, 186, 185, 184, 183, 182, 181, 180, 179, 178, 177, 176, 175, 174, 173,
            172, 171, 170, 169, 168, 167, 166, 165, 164, 163, 162, 161, 162, 162,
        ],
    ];
    const CANCELLATION_DEADLINES: &[[usize; 4]] = &[
        [84, 179, usize::MAX, usize::MAX],
        [84, 179, usize::MAX, usize::MAX],
        [55, 112, usize::MAX, usize::MAX],
        [49, 106, usize::MAX, usize::MAX],
        [55, 112, usize::MAX, usize::MAX],
        [49, 109, usize::MAX, usize::MAX],
        [usize::MAX, 113, 8, 70],
    ];
    const CANCELLATION_FAULTS: &[(usize, usize, usize, u64, u64)] = &[
        (0, 6, 4, 262, 0),
        (0, 11, 5, 262, 2),
        (0, 43, 0, 298, 0),
        (0, 85, 1, 268, 11),
        (0, 122, 4, 264, 0),
        (0, 128, 5, 261, 5),
        (0, 174, 6, 303, 0),
        (0, 182, 7, 279, 23),
        (0, 244, 6, 306, 0),
        (0, 253, 7, 279, 26),
        (0, 314, 6, 306, 0),
        (0, 317, 7, 276, 29),
        (1, 6, 4, 262, 0),
        (1, 11, 5, 262, 2),
        (1, 43, 0, 298, 0),
        (1, 85, 1, 268, 11),
        (1, 122, 4, 264, 0),
        (1, 128, 5, 261, 5),
        (1, 174, 6, 303, 0),
        (1, 182, 7, 279, 23),
        (1, 244, 6, 306, 0),
        (1, 253, 7, 279, 26),
        (1, 314, 6, 306, 0),
        (1, 317, 7, 276, 29),
        (2, 13, 2, 274, 0),
        (2, 56, 3, 241, 14),
        (2, 107, 6, 277, 0),
        (2, 115, 7, 253, 23),
        (3, 7, 2, 274, 0),
        (3, 50, 3, 241, 14),
        (3, 101, 6, 277, 0),
        (3, 109, 7, 253, 23),
        (4, 13, 2, 274, 0),
        (4, 56, 3, 241, 14),
        (4, 107, 6, 277, 0),
        (4, 115, 7, 253, 23),
        (5, 8, 0, 274, 0),
        (5, 50, 1, 244, 11),
        (5, 104, 6, 276, 0),
        (5, 112, 7, 252, 23),
        (6, 2, 4, 262, 0),
        (6, 9, 5, 256, 8),
        (6, 38, 4, 262, 0),
        (6, 39, 5, 253, 11),
        (6, 108, 6, 310, 0),
        (6, 116, 7, 286, 23),
        (6, 178, 6, 313, 0),
        (6, 187, 7, 286, 26),
        (6, 248, 6, 313, 0),
        (6, 251, 7, 283, 29),
        (6, 312, 6, 313, 0),
        (6, 323, 7, 280, 32),
        (6, 384, 6, 313, 0),
        (6, 387, 7, 277, 35),
    ];
    #[test]
    fn cancellation_creator_every_original_owner_cut_matches_independent_map() {
        use crate::living::simulation::cancellation_creator_tests as fixture;
        for kind in 0..7 {
            for (cut, &expected) in CANCELLATION_CUTS[kind].iter().enumerate() {
                for schedule in fixture::SCHEDULES {
                    let (creator, queue, catalog) = fixture::source(kind);
                    let anchor = crate::living::builtin_living_sandbox_v2().unwrap();
                    let mut step = captured_step(creator, anchor.capture_boundary().unwrap());
                    let step_pointer = &*step as *const _;
                    let creator = step.creator.as_mut().unwrap();
                    let pointer = &**creator as *const _;
                    let state = &creator.state;
                    let source = match kind {
                        0 => state.construction_jobs.as_slice().as_ptr() as usize,
                        1 => state.hull_jobs.as_slice().as_ptr() as usize,
                        5 => state.hazards.as_slice().as_ptr() as usize,
                        _ => state.routes.as_slice().as_ptr() as usize,
                    };
                    let entries = creator.events.payloads().as_ptr() as usize;
                    let effects = creator
                        .effect_work
                        .as_ref()
                        .map(|work| (work.values.as_ptr() as usize, work.suffix.as_ptr() as usize));
                    let probes: Vec<_> = allocations(creator)
                        .into_iter()
                        .map(|probe| {
                            let end = if probe.0 == source {
                                CANCELLATION_DEADLINES[kind][0]
                            } else if probe.0 == entries {
                                CANCELLATION_DEADLINES[kind][1]
                            } else if effects.is_some_and(|pair| probe.0 == pair.0) {
                                CANCELLATION_DEADLINES[kind][2]
                            } else if effects.is_some_and(|pair| probe.0 == pair.1) {
                                CANCELLATION_DEADLINES[kind][3]
                            } else {
                                usize::MAX
                            };
                            (probe, end)
                        })
                        .collect();
                    let mut visits = 0;
                    for attempt in 0..10_000 {
                        if visits == cut {
                            break;
                        }
                        let budget = schedule[attempt % schedule.len()].min(cut - visits);
                        let progress = creator.poll(budget).unwrap();
                        assert!(creator.last_poll_visits > 0 && creator.last_poll_visits <= budget);
                        visits += creator.last_poll_visits;
                        assert!(progress == LivingStepProgressV2::Pending || visits == cut);
                    }
                    assert_eq!(visits, cut, "finite cancellation cut guard");
                    if cut == CANCELLATION_CUTS[kind].len() - 1 {
                        assert_eq!(
                            creator.poll(1).unwrap(),
                            LivingStepProgressV2::NeedsSynchronousCompletion
                        );
                        assert_eq!(creator.last_poll_visits, 0);
                    } else {
                        assert!(creator.take_ready_result().is_err());
                    }
                    assert!(Arc::ptr_eq(&creator.queue, &queue));
                    assert!(Arc::ptr_eq(&creator.catalog, &catalog));
                    let current = allocations(creator);
                    for (probe, end) in probes {
                        assert_eq!(
                            retained(probe, &current),
                            cut < end,
                            "original cancellation source kind={kind} cut={cut} end={end}"
                        );
                    }
                    step.cancel();
                    assert_eq!(step.poll(1), Err(LivingStepJobErrorV2::Cancelled));
                    assert_eq!(step.last_poll_visits(), 0);
                    assert!(matches!(
                        step.take_synchronous_preparation(&anchor),
                        Err(LivingStepJobErrorV2::Cancelled)
                    ));
                    assert!(std::ptr::eq(&*step, step_pointer));
                    assert!(std::ptr::eq(&**step.creator.as_ref().unwrap(), pointer));
                    assert_eq!(
                        drain_observed(
                            step.into_boxed_retirement(
                                LivingPublicationRetirementPolicyV2::ReleaseHandle
                            ),
                            schedule,
                            &current,
                            (kind, false, cut)
                        ),
                        expected + 52,
                        "independent cancellation cut {kind}/{cut}"
                    );
                }
            }
        }
    }
    #[test]
    fn cancellation_creator_real_capacity_borrow_faults_and_carried_budgets() {
        use crate::living::simulation::cancellation_creator_tests as fixture;
        for &(kind, cut, control, expected_total, external_expected) in CANCELLATION_FAULTS {
            for schedule in fixture::SCHEDULES {
                let (mut creator, _queue, _catalog) = fixture::source(kind);
                let pointer = &*creator as *const _;
                let mut visits = 0;
                let mut poll = 0;
                for attempt in 0..10_000 {
                    if visits == cut {
                        break;
                    }
                    let b = schedule[attempt % schedule.len()].min(cut - visits);
                    assert_eq!(creator.poll(b).unwrap(), LivingStepProgressV2::Pending);
                    assert!(creator.last_poll_visits > 0 && creator.last_poll_visits <= b);
                    visits += creator.last_poll_visits;
                    poll = attempt + 1;
                }
                assert_eq!(visits, cut);
                let policy = LivingStateRetirementPolicyV2::ReleaseHandle;
                let external: Option<Box<dyn RetirementPoll>> = match control {
                    0 | 1 => {
                        macro_rules! handle {
                            ($j:ident) => {{
                                if control == 0 {
                                    assert!($j.inject_creator_capacity_failure());
                                    None
                                } else {
                                    Some(retirement_node($j.take_ready_result().unwrap(), policy))
                                }
                            }};
                        }
                        match creator
                            .cancellation_work
                            .as_mut()
                            .unwrap()
                            .filter
                            .as_mut()
                            .unwrap()
                        {
                            QueuedCancellationFilter::Construction(j) => handle!(j),
                            QueuedCancellationFilter::Hull(j) => handle!(j),
                            QueuedCancellationFilter::Hazard(j) => handle!(j),
                        }
                    }
                    2 => {
                        let QueuedCreatorInsertion::Route(j) = creator.insertion.as_mut().unwrap()
                        else {
                            panic!("route insertion")
                        };
                        assert!(j.inject_creator_capacity_failure());
                        None
                    }
                    3 => Some(borrow_insertion(creator.insertion.as_mut().unwrap())),
                    4 => {
                        assert!(
                            creator
                                .effect_work
                                .as_mut()
                                .unwrap()
                                .storage
                                .as_mut()
                                .unwrap()
                                .inject_creator_capacity_failure()
                        );
                        None
                    }
                    5 => Some(retirement_node(
                        creator
                            .effect_work
                            .as_mut()
                            .unwrap()
                            .storage
                            .as_mut()
                            .unwrap()
                            .take_ready_result()
                            .unwrap(),
                        policy,
                    )),
                    6 => {
                        assert!(
                            creator
                                .append
                                .as_mut()
                                .unwrap()
                                .inject_entry_capacity_failure()
                        );
                        None
                    }
                    7 => Some(retirement_node(
                        creator
                            .append
                            .as_mut()
                            .unwrap()
                            .take_ready_result()
                            .unwrap()
                            .0,
                        policy,
                    )),
                    _ => unreachable!(),
                };
                let anchor = crate::living::builtin_living_sandbox_v2().unwrap();
                let mut step = captured_step(creator, anchor.capture_boundary().unwrap());
                let step_pointer = &*step as *const _;
                let expected = if control % 2 == 0 {
                    LivingStepJobErrorV2::PreparationCapacity
                } else {
                    LivingStepJobErrorV2::Fault(LivingDeterministicFaultV2::Invariant)
                };
                let before = step.total_visits();
                assert_eq!(
                    step.poll(schedule[poll % schedule.len()]),
                    Err(expected.clone())
                );
                assert_eq!(step.last_poll_visits(), 1);
                assert_eq!(step.total_visits() - before, 1);
                assert!(std::ptr::eq(&**step.creator.as_ref().unwrap(), pointer));
                let total = step.total_visits();
                for b in [1, 64, 1024] {
                    assert_eq!(step.poll(b), Err(expected.clone()));
                    assert_eq!(step.last_poll_visits(), 0);
                    assert_eq!(step.total_visits(), total);
                }
                let current = allocations(step.creator.as_ref().unwrap());
                step.cancel();
                assert!(std::ptr::eq(&*step, step_pointer));
                assert_eq!(
                    drain_observed(
                        step.into_boxed_retirement(
                            LivingPublicationRetirementPolicyV2::ReleaseHandle
                        ),
                        schedule,
                        &current,
                        (kind, false, cut)
                    ),
                    expected_total
                );
                let mut count = 0;
                if let Some(mut external) = external {
                    let mut done = false;
                    for _ in 0..10_000 {
                        match external.retirement_step() {
                            RetirementStep::Work => count += 1,
                            RetirementStep::Done => {
                                count += 1;
                                done = true;
                                break;
                            }
                            RetirementStep::Deferred(_) => panic!("no shared external"),
                        }
                    }
                    assert!(done, "finite external retirement guard");
                }
                assert_eq!(
                    count, external_expected,
                    "external control {kind}/{cut}/{control}"
                );
            }
        }
    }
    // Independent task-2e3 arithmetic-v1 plus source-derived Vec amendment-v2.
    const DIPLOMACY_CUTS: &[&[u64]] = &[
        &[
            141, 141, 141, 141, 141, 141, 180, 204, 210, 204, 203, 202, 201, 200, 199, 198, 197,
            196, 195, 194, 193, 192, 191, 190, 189, 188, 187, 186, 185, 184, 183, 182, 181, 180,
            179, 180, 180, 180, 179, 178, 177, 176, 177, 177, 177, 177, 177, 177, 177, 176, 177,
            171, 170, 169, 168, 167, 166, 165, 164, 163, 162, 161, 160, 159, 158, 157, 156, 155,
            154, 153, 152, 151, 150, 149, 148, 147, 146, 145, 144, 143, 142, 141, 140, 141, 189,
            189, 192, 219, 222, 222, 219, 218, 216, 216, 216, 198, 197, 196, 195, 194, 193, 192,
            191, 190, 189, 188, 187, 186, 185, 184, 183, 182, 181, 180, 179, 178, 177, 176, 175,
            174, 173, 172, 171, 170, 169, 168, 167, 166, 165, 164, 163, 162, 161, 160, 159, 158,
            157, 156, 155, 154, 153, 152, 151, 150, 149, 148, 147, 146, 145, 144, 143, 144, 144,
        ],
        &[
            141, 141, 141, 141, 141, 141, 182, 206, 212, 206, 205, 204, 203, 202, 201, 200, 199,
            198, 197, 196, 195, 194, 193, 192, 191, 190, 189, 188, 187, 186, 185, 184, 183, 182,
            181, 182, 182, 182, 182, 182, 181, 180, 179, 178, 179, 179, 179, 179, 179, 179, 179,
            178, 179, 171, 170, 169, 168, 167, 166, 165, 164, 163, 162, 161, 160, 159, 158, 157,
            156, 155, 154, 153, 152, 151, 150, 149, 148, 147, 146, 145, 144, 143, 142, 141, 140,
            141, 189, 189, 192, 219, 222, 222, 219, 218, 216, 216, 216, 198, 197, 196, 195, 194,
            193, 192, 191, 190, 189, 188, 187, 186, 185, 184, 183, 182, 181, 180, 179, 178, 177,
            176, 175, 174, 173, 172, 171, 170, 169, 168, 167, 166, 165, 164, 163, 162, 161, 160,
            159, 158, 157, 156, 155, 154, 153, 152, 151, 150, 149, 148, 147, 146, 145, 144, 143,
            144, 144,
        ],
        &[
            141, 141, 141, 141, 141, 141, 209, 233, 239, 233, 232, 231, 230, 229, 228, 227, 226,
            225, 224, 223, 222, 221, 220, 219, 218, 217, 216, 215, 214, 213, 212, 211, 210, 209,
            208, 209, 209, 209, 209, 209, 208, 207, 206, 205, 206, 206, 206, 206, 206, 206, 205,
            206, 200, 199, 198, 197, 196, 195, 194, 193, 192, 191, 190, 189, 188, 187, 186, 185,
            184, 183, 182, 181, 180, 179, 178, 177, 176, 175, 174, 173, 172, 171, 170, 169, 168,
            167, 166, 167, 167, 166, 165, 164, 163, 162, 161, 160, 159, 158, 157, 156, 155, 154,
            153, 152, 151, 150, 149, 148, 147, 146, 145, 144, 143, 142, 141, 140, 139, 140, 188,
            188, 191, 218, 221, 221, 218, 217, 215, 215, 215, 197, 196, 195, 194, 193, 192, 191,
            190, 189, 188, 187, 186, 185, 184, 183, 182, 181, 180, 179, 178, 177, 176, 175, 174,
            173, 172, 171, 170, 169, 168, 167, 166, 165, 164, 163, 162, 161, 160, 159, 158, 157,
            156, 155, 154, 153, 152, 151, 150, 149, 148, 147, 146, 145, 144, 143, 142, 143, 143,
        ],
        &[
            141, 141, 141, 141, 141, 141, 168, 171, 171, 174, 174, 177, 177, 177, 215, 239, 245,
            239, 238, 237, 236, 235, 234, 233, 232, 231, 230, 229, 228, 227, 226, 225, 224, 223,
            222, 221, 220, 219, 218, 217, 216, 215, 214, 215, 215, 215, 214, 213, 212, 211, 212,
            212, 212, 211, 210, 209, 208, 209, 209, 209, 208, 207, 206, 205, 206, 206, 206, 206,
            205, 206, 204, 203, 202, 201, 200, 199, 198, 197, 196, 195, 194, 193, 192, 191, 190,
            189, 188, 187, 186, 185, 184, 183, 182, 181, 180, 179, 178, 177, 176, 175, 174, 173,
            172, 171, 170, 169, 168, 167, 166, 165, 164, 165, 204, 228, 234, 228, 227, 226, 225,
            224, 223, 222, 221, 220, 219, 218, 217, 216, 215, 214, 213, 212, 211, 210, 209, 208,
            207, 206, 205, 204, 203, 204, 204, 204, 203, 202, 201, 200, 201, 201, 201, 201, 201,
            201, 201, 200, 201, 195, 194, 193, 192, 191, 190, 189, 188, 187, 186, 185, 184, 183,
            182, 181, 180, 179, 178, 177, 176, 175, 174, 173, 172, 171, 170, 169, 168, 167, 166,
            165, 164, 165, 165, 164, 163, 162, 161, 160, 159, 158, 157, 156, 155, 154, 153, 152,
            151, 150, 149, 148, 147, 146, 145, 144, 143, 142, 141, 140, 139, 138, 137, 138, 186,
            186, 189, 216, 219, 219, 216, 215, 213, 213, 213, 195, 194, 193, 192, 191, 190, 189,
            188, 187, 186, 185, 184, 183, 182, 181, 180, 179, 178, 177, 176, 175, 174, 173, 172,
            171, 170, 169, 168, 167, 166, 165, 164, 163, 162, 161, 160, 159, 158, 157, 156, 155,
            154, 153, 152, 151, 150, 149, 148, 147, 146, 145, 144, 143, 142, 141, 140, 141, 141,
        ],
        &[
            141, 141, 141, 141, 141, 141, 207, 231, 237, 231, 230, 229, 228, 227, 226, 225, 224,
            223, 222, 221, 220, 219, 218, 217, 216, 215, 214, 213, 212, 211, 210, 209, 208, 207,
            206, 207, 207, 207, 206, 205, 204, 203, 204, 204, 204, 204, 204, 204, 203, 204, 200,
            199, 198, 197, 196, 195, 194, 193, 192, 191, 190, 189, 188, 187, 186, 185, 184, 183,
            182, 181, 180, 179, 178, 177, 176, 175, 174, 173, 172, 171, 170, 169, 168, 167, 166,
            167, 170, 170, 170, 173, 173, 173, 176, 173, 209, 233, 239, 233, 232, 231, 230, 229,
            228, 227, 226, 225, 224, 223, 222, 221, 220, 219, 218, 217, 216, 215, 214, 213, 212,
            211, 210, 209, 208, 209, 209, 209, 208, 207, 206, 205, 206, 206, 206, 205, 204, 203,
            202, 203, 203, 203, 203, 202, 203, 201, 200, 199, 198, 197, 196, 195, 194, 193, 192,
            191, 190, 189, 188, 187, 186, 185, 184, 183, 182, 181, 180, 179, 178, 177, 176, 175,
            174, 173, 172, 171, 170, 169, 168, 167, 166, 165, 164, 165, 165, 164, 163, 162, 161,
            160, 159, 158, 157, 156, 155, 154, 153, 152, 151, 150, 149, 148, 147, 146, 145, 144,
            143, 142, 141, 140, 139, 138, 137, 138, 186, 186, 189, 216, 219, 219, 216, 215, 213,
            213, 213, 195, 194, 193, 192, 191, 190, 189, 188, 187, 186, 185, 184, 183, 182, 181,
            180, 179, 178, 177, 176, 175, 174, 173, 172, 171, 170, 169, 168, 167, 166, 165, 164,
            163, 162, 161, 160, 159, 158, 157, 156, 155, 154, 153, 152, 151, 150, 149, 148, 147,
            146, 145, 144, 143, 142, 141, 140, 141, 141,
        ],
        &[
            141, 141, 141, 141, 141, 141, 168, 168, 198, 201, 198, 197, 196, 195, 194, 193, 192,
            191, 190, 189, 188, 187, 186, 185, 184, 183, 182, 181, 180, 179, 178, 177, 176, 175,
            174, 173, 174, 177, 177, 180, 180, 183, 183, 186, 186, 184, 184, 183, 182, 181, 180,
            179, 178, 177, 176, 175, 174, 173, 172, 171, 170, 169, 168, 167, 166, 165, 164, 163,
            162, 161, 160, 159, 158, 157, 156, 155, 154, 153, 152, 151, 150, 149, 148, 147, 146,
            145, 144, 143, 142, 141, 142, 190, 190, 193, 220, 223, 223, 220, 219, 217, 217, 217,
            199, 198, 197, 196, 195, 194, 193, 192, 191, 190, 189, 188, 187, 186, 185, 184, 183,
            182, 181, 180, 179, 178, 177, 176, 175, 174, 173, 172, 171, 170, 169, 168, 167, 166,
            165, 164, 163, 162, 161, 160, 159, 158, 157, 156, 155, 154, 153, 152, 151, 150, 149,
            148, 147, 146, 145, 144, 145, 145,
        ],
        &[
            141, 141, 141, 141, 141, 141, 168, 168, 168, 178, 178, 177, 176, 175, 174, 173, 172,
            171, 170, 169, 168, 167, 166, 165, 164, 163, 162, 161, 160, 159, 158, 157, 156, 155,
            154, 153, 152, 151, 150, 149, 148, 147, 146, 145, 144, 143, 142, 141, 140, 139, 138,
            137, 136, 135, 136, 184, 184, 187, 214, 217, 217, 214, 213, 211, 211, 211, 193, 192,
            191, 190, 189, 188, 187, 186, 185, 184, 183, 182, 181, 180, 179, 178, 177, 176, 175,
            174, 173, 172, 171, 170, 169, 168, 167, 166, 165, 164, 163, 162, 161, 160, 159, 158,
            157, 156, 155, 154, 153, 152, 151, 150, 149, 148, 147, 146, 145, 144, 143, 142, 141,
            140, 139, 138, 139, 139,
        ],
        &[
            141, 141, 141, 141, 141, 141, 168, 168, 198, 201, 198, 197, 196, 195, 194, 193, 192,
            191, 190, 189, 188, 187, 186, 185, 184, 183, 182, 181, 180, 179, 178, 177, 176, 175,
            174, 173, 174, 180, 180, 179, 178, 177, 176, 175, 174, 173, 172, 171, 170, 169, 168,
            167, 166, 165, 164, 163, 162, 161, 160, 159, 158, 157, 156, 155, 154, 153, 152, 151,
            150, 149, 148, 147, 146, 145, 144, 143, 142, 141, 140, 139, 138, 137, 138, 186, 186,
            189, 216, 219, 219, 216, 215, 213, 213, 213, 195, 194, 193, 192, 191, 190, 189, 188,
            187, 186, 185, 184, 183, 182, 181, 180, 179, 178, 177, 176, 175, 174, 173, 172, 171,
            170, 169, 168, 167, 166, 165, 164, 163, 162, 161, 160, 159, 158, 157, 156, 155, 154,
            153, 152, 151, 150, 149, 148, 147, 146, 145, 144, 143, 142, 141, 140, 141, 141,
        ],
    ];
    const DIPLOMACY_DEADLINES: &[[usize; 6]] = &[
        [49, usize::MAX, usize::MAX, usize::MAX, usize::MAX, 91],
        [usize::MAX, 51, usize::MAX, usize::MAX, usize::MAX, 93],
        [usize::MAX, 50, usize::MAX, usize::MAX, usize::MAX, 125],
        [usize::MAX, 68, 155, usize::MAX, usize::MAX, 227],
        [usize::MAX, usize::MAX, 48, 141, usize::MAX, 219],
        [usize::MAX, usize::MAX, usize::MAX, usize::MAX, 81, 98],
        [usize::MAX, usize::MAX, usize::MAX, usize::MAX, 45, 62],
        [usize::MAX, usize::MAX, usize::MAX, usize::MAX, 73, 90],
    ];
    // Source-only E6 arithmetic-v1, frozen before any Rust execution.
    const FLEET_HULL_CUTS: &[&[u64]] = &[
        &[
            183, 183, 183, 183, 183, 183, 183, 183, 182, 183, 231, 231, 234, 261, 264, 264, 261,
            260, 258, 258, 258, 240, 239, 238, 237, 236, 235, 234, 233, 232, 231, 230, 229, 228,
            227, 226, 225, 224, 223, 222, 221, 220, 219, 218, 217, 216, 215, 214, 213, 212, 211,
            210, 209, 208, 207, 206, 205, 204, 203, 202, 201, 200, 199, 198, 197, 196, 195, 194,
            193, 192, 191, 190, 189, 188, 187, 186, 185, 186, 186,
        ],
        &[
            183, 183, 183, 183, 183, 183, 183, 187, 186, 185, 184, 183, 182, 181, 180, 181, 229,
            229, 232, 259, 262, 262, 259, 258, 256, 256, 256, 238, 237, 236, 235, 234, 233, 232,
            231, 230, 229, 228, 227, 226, 225, 224, 223, 222, 221, 220, 219, 218, 217, 216, 215,
            214, 213, 212, 211, 210, 209, 208, 207, 206, 205, 204, 203, 202, 201, 200, 199, 198,
            197, 196, 195, 194, 193, 192, 191, 190, 189, 188, 187, 186, 185, 184, 183, 184, 184,
        ],
        &[
            183, 183, 183, 183, 183, 183, 183, 253, 252, 251, 250, 249, 248, 247, 246, 245, 244,
            243, 242, 241, 240, 239, 238, 237, 236, 235, 234, 233, 232, 231, 230, 229, 228, 227,
            226, 225, 224, 223, 222, 221, 220, 219, 218, 217, 216, 215, 214, 213, 212, 211, 210,
            209, 208, 207, 206, 205, 204, 203, 202, 201, 200, 199, 198, 197, 196, 195, 194, 193,
            192, 191, 190, 189, 188, 187, 186, 185, 184, 183, 182, 181, 180, 179, 178, 177, 176,
            175, 174, 173, 172, 171, 170, 169, 168, 167, 166, 165, 164, 163, 162, 161, 160, 159,
            158, 157, 156, 155, 154, 153, 152, 151, 150, 149, 148, 147, 148, 196, 196, 199, 226,
            229, 229, 226, 225, 223, 223, 223, 205, 204, 203, 202, 201, 200, 199, 198, 197, 196,
            195, 194, 193, 192, 191, 190, 189, 188, 187, 186, 185, 184, 183, 182, 181, 180, 179,
            178, 177, 176, 175, 174, 173, 172, 171, 170, 169, 168, 167, 166, 165, 164, 163, 162,
            161, 160, 159, 158, 157, 156, 155, 154, 153, 152, 151, 150, 151, 151,
        ],
        &[
            183, 183, 183, 183, 183, 183, 183, 183, 183, 183, 182, 183, 231, 231, 234, 261, 264,
            264, 261, 260, 258, 258, 258, 240, 239, 238, 237, 236, 235, 234, 233, 232, 231, 230,
            229, 228, 227, 226, 225, 224, 223, 222, 221, 220, 219, 218, 217, 216, 215, 214, 213,
            212, 211, 210, 209, 208, 207, 206, 205, 204, 203, 202, 201, 200, 199, 198, 197, 196,
            195, 194, 193, 192, 191, 190, 189, 188, 187, 186, 185, 186, 186,
        ],
        &[
            183, 183, 183, 183, 183, 183, 183, 183, 183, 183, 183, 187, 186, 185, 184, 183, 182,
            181, 180, 181, 229, 229, 232, 259, 262, 262, 259, 258, 256, 256, 256, 238, 237, 236,
            235, 234, 233, 232, 231, 230, 229, 228, 227, 226, 225, 224, 223, 222, 221, 220, 219,
            218, 217, 216, 215, 214, 213, 212, 211, 210, 209, 208, 207, 206, 205, 204, 203, 202,
            201, 200, 199, 198, 197, 196, 195, 194, 193, 192, 191, 190, 189, 188, 187, 186, 185,
            184, 183, 184, 184,
        ],
        &[
            183, 183, 183, 183, 183, 183, 183, 183, 183, 183, 183, 183, 183, 253, 252, 251, 250,
            249, 248, 247, 246, 245, 244, 243, 242, 241, 240, 239, 238, 237, 236, 235, 234, 233,
            232, 231, 230, 229, 228, 227, 226, 225, 224, 223, 222, 221, 220, 219, 218, 217, 216,
            215, 214, 213, 212, 211, 210, 209, 208, 207, 206, 205, 204, 203, 202, 201, 200, 199,
            198, 197, 196, 195, 194, 193, 192, 191, 190, 189, 188, 187, 186, 185, 184, 183, 182,
            181, 180, 179, 178, 177, 176, 175, 174, 173, 172, 171, 170, 169, 168, 167, 166, 165,
            164, 163, 162, 161, 160, 159, 158, 157, 156, 155, 154, 153, 152, 151, 150, 149, 148,
            147, 148, 196, 196, 199, 226, 229, 229, 226, 225, 223, 223, 223, 205, 204, 203, 202,
            201, 200, 199, 198, 197, 196, 195, 194, 193, 192, 191, 190, 189, 188, 187, 186, 185,
            184, 183, 182, 181, 180, 179, 178, 177, 176, 175, 174, 173, 172, 171, 170, 169, 168,
            167, 166, 165, 164, 163, 162, 161, 160, 159, 158, 157, 156, 155, 154, 153, 152, 151,
            150, 151, 151,
        ],
    ];
    #[test]
    fn fleet_hull_creator_every_original_owner_cut_matches_independent_map() {
        use crate::living::simulation::fleet_hull_creator_tests as fixture;
        for (kind, cuts) in FLEET_HULL_CUTS.iter().enumerate() {
            for (cut, &expected) in cuts.iter().enumerate() {
                for schedule in fixture::SCHEDULES {
                    let (creator, queue, catalog) = fixture::source(kind);
                    let anchor = crate::living::builtin_living_sandbox_v2().unwrap();
                    let mut step = captured_step(creator, anchor.capture_boundary().unwrap());
                    let step_pointer = &*step as *const _;
                    let creator = step.creator.as_mut().unwrap();
                    let pointer = &**creator as *const _;
                    let collector = creator.events.payloads().as_ptr() as usize;
                    let probes = allocations(creator);
                    let path = creator.state.fleets.as_slice()[kind % 3]
                        .order
                        .as_ref()
                        .map(|o| o.lane_path.as_ptr() as usize);
                    // Original allocated empty order is observed before any poll.
                    assert!(probes.iter().any(|p| p.1 == 0 && p.2 == Some(7)));
                    let mut visits = 0;
                    for attempt in 0..10000 {
                        if visits == cut {
                            break;
                        }
                        let budget = schedule[attempt % schedule.len()].min(cut - visits);
                        creator.poll(budget).unwrap();
                        assert!(creator.last_poll_visits > 0 && creator.last_poll_visits <= budget);
                        visits += creator.last_poll_visits;
                    }
                    assert_eq!(visits, cut);
                    for budget in [0, 1025] {
                        assert_eq!(
                            creator.poll(budget),
                            Err(crate::living::step_job::LivingLifecycleJobErrorV2::Budget)
                        );
                        assert_eq!(creator.last_poll_visits, 0);
                    }
                    if cut == cuts.len() - 1 {
                        for budget in [1, 64, 1024] {
                            assert_eq!(
                                creator.poll(budget).unwrap(),
                                LivingStepProgressV2::NeedsSynchronousCompletion
                            );
                            assert_eq!(creator.last_poll_visits, 0);
                        }
                    } else {
                        assert!(creator.take_ready_result().is_err());
                    }
                    assert!(Arc::ptr_eq(&creator.queue, &queue));
                    assert!(Arc::ptr_eq(&creator.catalog, &catalog));
                    let current = allocations(creator);
                    for probe in probes {
                        let deadline = if probe.0 == collector {
                            [17, 23, 122, 19, 27, 128][kind]
                        } else if path == Some(probe.0) {
                            [usize::MAX, 10, 109, usize::MAX, 14, 115][kind]
                        } else {
                            usize::MAX
                        };
                        assert_eq!(
                            retained(probe, &current),
                            cut < deadline,
                            "kind {kind} cut {cut} deadline {deadline}"
                        );
                    }
                    step.cancel();
                    assert_eq!(step.poll(1), Err(LivingStepJobErrorV2::Cancelled));
                    assert_eq!(step.last_poll_visits(), 0);
                    assert!(matches!(
                        step.take_synchronous_preparation(&anchor),
                        Err(LivingStepJobErrorV2::Cancelled)
                    ));
                    assert!(std::ptr::eq(&*step, step_pointer));
                    assert!(std::ptr::eq(&**step.creator.as_ref().unwrap(), pointer));
                    assert_eq!(
                        drain_observed(
                            step.into_boxed_retirement(
                                LivingPublicationRetirementPolicyV2::ReleaseHandle
                            ),
                            schedule,
                            &current,
                            (kind, false, cut)
                        ),
                        expected + 52
                    );
                }
            }
        }
    }
    #[test]
    fn geometry_creator_every_original_owner_cut_matches_independent_map() {
        use crate::living::simulation::geometry_creator_tests as fixture;
        for (kind, cuts) in GEOMETRY_CUTS.iter().enumerate() {
            for (cut, &expected) in cuts.iter().enumerate() {
                for schedule in fixture::SCHEDULES {
                    let (creator, queue, catalog) = fixture::source(kind);
                    let anchor = crate::living::builtin_living_sandbox_v2().unwrap();
                    let mut step = captured_step(creator, anchor.capture_boundary().unwrap());
                    let step_pointer = &*step as *const _;
                    let creator = step.creator.as_mut().unwrap();
                    let pointer = &**creator as *const _;
                    let collector = creator.events.payloads().as_ptr() as usize;
                    let probes = allocations(creator);
                    // Original allocated empty order is observed before any poll.
                    assert!(probes.iter().any(|p| p.1 == 0 && p.2 == Some(7)));
                    let mut visits = 0;
                    for attempt in 0..10000 {
                        if visits == cut {
                            break;
                        }
                        let budget = schedule[attempt % schedule.len()].min(cut - visits);
                        creator.poll(budget).unwrap();
                        assert!(creator.last_poll_visits > 0 && creator.last_poll_visits <= budget);
                        visits += creator.last_poll_visits;
                    }
                    assert_eq!(visits, cut);
                    for budget in [0, 1025] {
                        assert_eq!(
                            creator.poll(budget),
                            Err(crate::living::step_job::LivingLifecycleJobErrorV2::Budget)
                        );
                        assert_eq!(creator.last_poll_visits, 0);
                    }
                    if cut == cuts.len() - 1 {
                        for budget in [1, 64, 1024] {
                            assert_eq!(
                                creator.poll(budget).unwrap(),
                                LivingStepProgressV2::NeedsSynchronousCompletion
                            );
                            assert_eq!(creator.last_poll_visits, 0);
                        }
                    } else {
                        assert!(creator.take_ready_result().is_err());
                    }
                    assert!(Arc::ptr_eq(&creator.queue, &queue));
                    assert!(Arc::ptr_eq(&creator.catalog, &catalog));
                    let current = allocations(creator);
                    for probe in probes {
                        let deadline = if probe.0 == collector {
                            [18, 12, 18, 12][kind]
                        } else {
                            usize::MAX
                        };
                        assert_eq!(
                            retained(probe, &current),
                            cut < deadline,
                            "kind {kind} cut {cut} deadline {deadline}"
                        );
                    }
                    step.cancel();
                    assert_eq!(step.poll(1), Err(LivingStepJobErrorV2::Cancelled));
                    assert_eq!(step.last_poll_visits(), 0);
                    assert!(matches!(
                        step.take_synchronous_preparation(&anchor),
                        Err(LivingStepJobErrorV2::Cancelled)
                    ));
                    assert!(std::ptr::eq(&*step, step_pointer));
                    assert!(std::ptr::eq(&**step.creator.as_ref().unwrap(), pointer));
                    assert_eq!(
                        drain_observed(
                            step.into_boxed_retirement(
                                LivingPublicationRetirementPolicyV2::ReleaseHandle
                            ),
                            schedule,
                            &current,
                            (kind, false, cut)
                        ),
                        expected + 52
                    );
                }
            }
        }
    }
    #[test]
    fn fleet_hull_creator_guard_errors_debit_original_step_and_retire_owners() {
        use crate::living::simulation::fleet_hull_creator_tests as fixture;
        for kind in 0..6 {
            for position in 0..4 {
                for schedule in fixture::SCHEDULES {
                    let (mut creator, queue, catalog) = fixture::source(kind);
                    let id = LivingEntityIdV2([30 + (kind % 3) as u8; 16]);
                    if position < 3 {
                        creator
                            .state
                            .hull_jobs
                            .record_at_mut(position)
                            .unwrap()
                            .target_fleet = Some(id);
                    } else {
                        creator
                            .state
                            .fleets
                            .record_at_mut(kind % 3)
                            .unwrap()
                            .location = LivingFleetLocationV2::Travelling {
                            origin_world: LivingEntityIdV2([2; 16]),
                            destination_world: LivingEntityIdV2([3; 16]),
                            departure_tick: LivingTickV2(0),
                            arrival_tick: LivingTickV2(99),
                        };
                    }
                    let unchanged = creator.state.clone();
                    let expected_forward = 3
                        + if kind >= 3 { 2 * (kind % 3 + 1) } else { 0 }
                        + if position < 3 { position + 1 } else { 0 };
                    let anchor = crate::living::builtin_living_sandbox_v2().unwrap();
                    let mut step = captured_step(creator, anchor.capture_boundary().unwrap());
                    let pointer = &**step.creator.as_ref().unwrap() as *const _;
                    let mut used = 0;
                    let mut poll = 0;
                    while used + 1 < expected_forward {
                        assert!(poll < 1000);
                        let budget =
                            schedule[poll % schedule.len()].min(expected_forward - used - 1);
                        poll += 1;
                        assert_eq!(step.poll(budget).unwrap(), LivingStepProgressV2::Pending);
                        used += step.last_poll_visits();
                    }
                    let fault = LivingStepJobErrorV2::Fault(LivingDeterministicFaultV2::Replay {
                        accepted_sequence: 1,
                        reason: Box::new(LivingCommandRejectionV2::ReferencedTarget),
                    });
                    assert_eq!(
                        step.poll(schedule[poll % schedule.len()]),
                        Err(fault.clone())
                    );
                    assert_eq!(step.last_poll_visits(), 1);
                    assert_eq!(step.total_visits(), expected_forward as u64);
                    for budget in [1, 64, 1024] {
                        assert_eq!(step.poll(budget), Err(fault.clone()));
                        assert_eq!(step.last_poll_visits(), 0);
                    }
                    let creator = step.creator.as_ref().unwrap();
                    assert!(std::ptr::eq(&**creator, pointer));
                    assert!(Arc::ptr_eq(&creator.queue, &queue));
                    assert!(Arc::ptr_eq(&creator.catalog, &catalog));
                    assert_eq!(creator.state, unchanged);
                    assert_eq!(creator.events.len(), 1);
                    let current = allocations(creator);
                    assert_eq!(
                        drain_observed(
                            step.into_boxed_retirement(
                                LivingPublicationRetirementPolicyV2::ReleaseHandle
                            ),
                            schedule,
                            &current,
                            (kind, false, position)
                        ),
                        241
                    );
                }
            }
        }
    }
    #[test]
    fn geometry_creator_guard_errors_debit_original_step_and_retire_owners() {
        use crate::living::simulation::geometry_creator_tests as fixture;
        for guard in 0..3 {
            for position in 0..3 {
                for schedule in fixture::SCHEDULES {
                    let (mut creator, queue, catalog) =
                        fixture::source(if guard == 2 { 2 } else { 0 });
                    let id = |n| LivingEntityIdV2([n; 16]);
                    match guard {
                        0 => {
                            creator
                                .state
                                .shipments
                                .record_at_mut(position)
                                .unwrap()
                                .source_world = id(1)
                        }
                        1 => {
                            creator
                                .state
                                .fleets
                                .record_at_mut(position)
                                .unwrap()
                                .location = LivingFleetLocationV2::Travelling {
                                origin_world: id(1),
                                destination_world: id(3),
                                departure_tick: LivingTickV2(0),
                                arrival_tick: LivingTickV2(9),
                            }
                        }
                        _ => {
                            creator
                                .state
                                .fleets
                                .record_at_mut(2)
                                .unwrap()
                                .order
                                .as_mut()
                                .unwrap()
                                .lane_path[position] = id(8)
                        }
                    }
                    let unchanged = creator.state.clone();
                    let expected_forward = 3 + position + 1 + if guard == 0 { 0 } else { 3 };
                    let anchor = crate::living::builtin_living_sandbox_v2().unwrap();
                    let mut step = captured_step(creator, anchor.capture_boundary().unwrap());
                    let pointer = &**step.creator.as_ref().unwrap() as *const _;
                    let mut used = 0;
                    let mut poll = 0;
                    while used + 1 < expected_forward {
                        assert!(poll < 1000);
                        let budget =
                            schedule[poll % schedule.len()].min(expected_forward - used - 1);
                        poll += 1;
                        assert_eq!(step.poll(budget).unwrap(), LivingStepProgressV2::Pending);
                        used += step.last_poll_visits();
                    }
                    let fault = LivingStepJobErrorV2::Fault(LivingDeterministicFaultV2::Replay {
                        accepted_sequence: 1,
                        reason: Box::new(LivingCommandRejectionV2::ReferencedTarget),
                    });
                    assert_eq!(
                        step.poll(schedule[poll % schedule.len()]),
                        Err(fault.clone())
                    );
                    assert_eq!(step.last_poll_visits(), 1);
                    assert_eq!(step.total_visits(), expected_forward as u64);
                    for budget in [1, 64, 1024] {
                        assert_eq!(step.poll(budget), Err(fault.clone()));
                        assert_eq!(step.last_poll_visits(), 0);
                    }
                    let creator = step.creator.as_ref().unwrap();
                    assert!(std::ptr::eq(&**creator, pointer));
                    assert!(Arc::ptr_eq(&creator.queue, &queue));
                    assert!(Arc::ptr_eq(&creator.catalog, &catalog));
                    assert_eq!(creator.state, unchanged);
                    assert_eq!(creator.events.len(), 1);
                    let current = allocations(creator);
                    assert_eq!(
                        drain_observed(
                            step.into_boxed_retirement(
                                LivingPublicationRetirementPolicyV2::ReleaseHandle
                            ),
                            schedule,
                            &current,
                            (guard, false, position)
                        ),
                        208
                    );
                }
            }
        }
    }
    // Immutable source-free E5 arithmetic-v1, pinned before execution.
    const GEOMETRY_CUTS: &[&[u64]] = &[
        &[
            150, 150, 150, 150, 150, 150, 150, 150, 150, 150, 150, 198, 198, 201, 228, 231, 231,
            228, 227, 225, 225, 225, 207, 206, 205, 204, 203, 202, 201, 200, 199, 198, 197, 196,
            195, 194, 193, 192, 191, 190, 189, 188, 187, 186, 185, 184, 183, 182, 181, 180, 179,
            178, 177, 176, 175, 174, 173, 172, 171, 170, 169, 168, 167, 166, 165, 164, 163, 162,
            161, 160, 159, 158, 157, 156, 155, 154, 153, 152, 153, 153,
        ],
        &[
            150, 150, 150, 150, 150, 198, 198, 201, 228, 231, 231, 228, 227, 225, 225, 225, 207,
            206, 205, 204, 203, 202, 201, 200, 199, 198, 197, 196, 195, 194, 193, 192, 191, 190,
            189, 188, 187, 186, 185, 184, 183, 182, 181, 180, 179, 178, 177, 176, 175, 174, 173,
            172, 171, 170, 169, 168, 167, 166, 165, 164, 163, 162, 161, 160, 159, 158, 157, 156,
            155, 154, 153, 152, 153, 153,
        ],
        &[
            150, 150, 150, 150, 150, 150, 150, 150, 150, 150, 150, 198, 198, 201, 228, 231, 231,
            228, 227, 225, 225, 225, 207, 206, 205, 204, 203, 202, 201, 200, 199, 198, 197, 196,
            195, 194, 193, 192, 191, 190, 189, 188, 187, 186, 185, 184, 183, 182, 181, 180, 179,
            178, 177, 176, 175, 174, 173, 172, 171, 170, 169, 168, 167, 166, 165, 164, 163, 162,
            161, 160, 159, 158, 157, 156, 155, 154, 153, 152, 153, 153,
        ],
        &[
            150, 150, 150, 150, 150, 198, 198, 201, 228, 231, 231, 228, 227, 225, 225, 225, 207,
            206, 205, 204, 203, 202, 201, 200, 199, 198, 197, 196, 195, 194, 193, 192, 191, 190,
            189, 188, 187, 186, 185, 184, 183, 182, 181, 180, 179, 178, 177, 176, 175, 174, 173,
            172, 171, 170, 169, 168, 167, 166, 165, 164, 163, 162, 161, 160, 159, 158, 157, 156,
            155, 154, 153, 152, 153, 153,
        ],
    ];
    const DIPLOMACY_FAULTS: &[(usize, usize, usize, u64, u64)] = &[
        (0, 7, 0, 256, 0),
        (0, 50, 1, 223, 14),
        (1, 7, 0, 258, 0),
        (1, 52, 1, 222, 17),
        (2, 7, 2, 285, 0),
        (2, 51, 3, 252, 14),
        (3, 15, 2, 291, 0),
        (3, 69, 3, 258, 8),
        (3, 113, 0, 280, 0),
        (3, 156, 1, 247, 14),
        (4, 7, 2, 283, 0),
        (4, 49, 3, 253, 11),
        (4, 95, 2, 285, 0),
        (4, 142, 3, 255, 8),
        (5, 8, 4, 250, 0),
        (5, 9, 5, 253, 2),
        (7, 8, 4, 250, 0),
        (7, 9, 5, 253, 2),
    ];
    #[test]
    fn diplomacy_creator_every_original_owner_cut_matches_independent_map() {
        use crate::living::simulation::diplomacy_creator_tests as fixture;
        for kind in 0..8 {
            for (cut, &expected) in DIPLOMACY_CUTS[kind].iter().enumerate() {
                for schedule in fixture::SCHEDULES {
                    let (creator, queue, catalog) = fixture::source(kind);
                    let anchor = crate::living::builtin_living_sandbox_v2().unwrap();
                    let mut step = captured_step(creator, anchor.capture_boundary().unwrap());
                    let step_pointer = &*step as *const _;
                    let creator = step.creator.as_mut().unwrap();
                    let pointer = &**creator as *const _;
                    let state = &creator.state;
                    let sources = [
                        state.relations.as_slice().as_ptr() as usize,
                        state.agreements.as_slice().as_ptr() as usize,
                        state.wars.as_slice().as_ptr() as usize,
                        state.occupations.as_slice().as_ptr() as usize,
                        state.fleets.as_slice()[0]
                            .order
                            .as_ref()
                            .unwrap()
                            .lane_path
                            .as_ptr() as usize,
                        creator.events.payloads().as_ptr() as usize,
                    ];
                    let probes: Vec<_> = allocations(creator)
                        .into_iter()
                        .map(|probe| {
                            let end = sources
                                .iter()
                                .position(|&p| p == probe.0)
                                .map(|i| DIPLOMACY_DEADLINES[kind][i])
                                .unwrap_or(usize::MAX);
                            (probe, end)
                        })
                        .collect();
                    let mut visits = 0;
                    for attempt in 0..10_000 {
                        if visits == cut {
                            break;
                        }
                        let budget = schedule[attempt % schedule.len()].min(cut - visits);
                        let progress = creator.poll(budget).unwrap();
                        assert!(creator.last_poll_visits > 0 && creator.last_poll_visits <= budget);
                        visits += creator.last_poll_visits;
                        assert!(progress == LivingStepProgressV2::Pending || visits == cut);
                    }
                    assert_eq!(visits, cut, "finite diplomacy forward guard");
                    if cut == DIPLOMACY_CUTS[kind].len() - 1 {
                        assert_eq!(
                            creator.poll(1).unwrap(),
                            LivingStepProgressV2::NeedsSynchronousCompletion
                        );
                        assert_eq!(creator.last_poll_visits, 0);
                    } else {
                        assert!(creator.take_ready_result().is_err());
                    }
                    assert!(Arc::ptr_eq(&creator.queue, &queue));
                    assert!(Arc::ptr_eq(&creator.catalog, &catalog));
                    let current = allocations(creator);
                    for (probe, end) in probes {
                        assert_eq!(
                            retained(probe, &current),
                            cut < end,
                            "kind {kind} cut {cut} original release {end}"
                        );
                    }
                    step.cancel();
                    assert_eq!(step.poll(1), Err(LivingStepJobErrorV2::Cancelled));
                    assert_eq!(step.last_poll_visits(), 0);
                    assert!(matches!(
                        step.take_synchronous_preparation(&anchor),
                        Err(LivingStepJobErrorV2::Cancelled)
                    ));
                    assert!(std::ptr::eq(&*step, step_pointer));
                    assert!(std::ptr::eq(&**step.creator.as_ref().unwrap(), pointer));
                    assert_eq!(
                        drain_observed(
                            step.into_boxed_retirement(
                                LivingPublicationRetirementPolicyV2::ReleaseHandle
                            ),
                            schedule,
                            &current,
                            (kind, false, cut)
                        ),
                        expected + 52 + 6,
                        "diplomacy cut {kind}/{cut}"
                    );
                }
            }
        }
    }
    #[test]
    fn diplomacy_creator_real_child_refusals_capacity_and_carried_failure_budgets() {
        use crate::living::simulation::diplomacy_creator_tests as fixture;
        // Analytic controls: 0 insertion capacity,1 insertion real borrow,
        // 2 filter capacity,3 filter real borrow,4 path capacity,5 path real borrow.
        for &(kind, cut, control, step_expected, external_expected) in DIPLOMACY_FAULTS {
            for schedule in fixture::SCHEDULES {
                let (mut creator, _queue, _catalog) = fixture::source(kind);
                let creator_pointer = &*creator as *const _;
                let mut visits = 0;
                let mut poll = 0;
                for _ in 0..10_000 {
                    if visits == cut {
                        break;
                    }
                    let budget = schedule[poll % schedule.len()].min(cut - visits);
                    poll += 1;
                    assert_eq!(creator.poll(budget).unwrap(), LivingStepProgressV2::Pending);
                    assert!(creator.last_poll_visits > 0 && creator.last_poll_visits <= budget);
                    visits += creator.last_poll_visits;
                }
                assert_eq!(visits, cut);
                let policy = LivingStateRetirementPolicyV2::ReleaseHandle;
                let external = match control {
                    0 => {
                        assert!(
                            creator
                                .insertion
                                .as_mut()
                                .is_some_and(|insert| match insert {
                                    QueuedCreatorInsertion::Relation(j) =>
                                        j.inject_creator_capacity_failure(),
                                    QueuedCreatorInsertion::Agreement(j) =>
                                        j.inject_creator_capacity_failure(),
                                    QueuedCreatorInsertion::War(j) =>
                                        j.inject_creator_capacity_failure(),
                                    _ => false,
                                })
                        );
                        None
                    }
                    1 => Some(borrow_insertion(creator.insertion.as_mut().unwrap())),
                    2 | 3 => {
                        let filter = creator
                            .diplomacy_work
                            .as_mut()
                            .unwrap()
                            .filter
                            .as_mut()
                            .unwrap();
                        macro_rules! handle {
                            ($j:ident) => {{
                                if control == 2 {
                                    assert!($j.inject_creator_capacity_failure());
                                    None
                                } else {
                                    Some(retirement_node($j.take_ready_result().unwrap(), policy))
                                }
                            }};
                        }
                        match filter {
                            QueuedDiplomacyFilter::Agreement(j) => handle!(j),
                            QueuedDiplomacyFilter::War(j) => handle!(j),
                            QueuedDiplomacyFilter::Occupation(j) => handle!(j),
                        }
                    }
                    4 => {
                        assert!(
                            creator
                                .diplomacy_work
                                .as_mut()
                                .unwrap()
                                .path_storage
                                .as_mut()
                                .unwrap()
                                .inject_creator_empty_capacity_failure()
                        );
                        None
                    }
                    5 => Some(retirement_node(
                        creator
                            .diplomacy_work
                            .as_mut()
                            .unwrap()
                            .path_storage
                            .as_mut()
                            .unwrap()
                            .take_ready_result()
                            .unwrap(),
                        policy,
                    )),
                    _ => unreachable!(),
                };
                let mut anchor = crate::living::builtin_living_sandbox_v2().unwrap();
                let capture = anchor.capture_boundary().unwrap();
                let mut step = captured_step(creator, capture.clone());
                let step_pointer = &*step as *const _;
                let expected = if control % 2 == 0 {
                    LivingStepJobErrorV2::PreparationCapacity
                } else {
                    LivingStepJobErrorV2::Fault(LivingDeterministicFaultV2::Invariant)
                };
                let failure_start = step.total_visits();
                assert_eq!(
                    step.poll(schedule[poll % schedule.len()]),
                    Err(expected.clone())
                );
                assert_eq!(step.last_poll_visits(), 1);
                assert_eq!(step.total_visits() - failure_start, 1);
                assert!(std::ptr::eq(
                    &**step.creator.as_ref().unwrap(),
                    creator_pointer
                ));
                let total = step.total_visits();
                for b in [1, 64, 1024] {
                    assert_eq!(step.poll(b), Err(expected.clone()));
                    assert_eq!(step.last_poll_visits(), 0);
                    assert_eq!(step.total_visits(), total);
                }
                for b in [0, 1025] {
                    assert_eq!(step.poll(b), Err(LivingStepJobErrorV2::Budget));
                    assert_eq!(step.last_poll_visits(), 0);
                }
                // Current failure tokens retain the captured boundary; a newly
                // admitted tail makes subsequent extraction stale without publication.
                if control % 2 == 0 {
                    assert!(matches!(
                        step.take_synchronous_preparation(&anchor),
                        Err(LivingStepJobErrorV2::PreparationCapacity)
                    ));
                }
                let cursor = anchor.published_cursor();
                anchor
                    .submit(LivingCommandEnvelopeV2 {
                        expected_committed_revision: cursor.committed_revision,
                        expected_tick: cursor.tick,
                        expected_pending_sequence: cursor.pending_sequence,
                        mode: LivingCommandModeV2::Running,
                        command: LivingCommandV2::CreatorBatch {
                            operations: vec![LivingCreatorOperationV2::CreateSystem {
                                local: 0,
                                name: crate::living::LivingNameV2::new("Diplomacy tail").unwrap(),
                            }],
                        },
                    })
                    .unwrap();
                let before = anchor.state().canonical_bytes().unwrap();
                assert!(!anchor.boundary_capture_is_current(&capture));
                assert!(matches!(
                    step.take_synchronous_preparation(&anchor),
                    Err(LivingStepJobErrorV2::Stale)
                ));
                assert_eq!(anchor.state().canonical_bytes().unwrap(), before);
                assert_eq!(anchor.capture_boundary().unwrap().pending_count(), 1);
                assert!(anchor.fault().is_none());
                let current = allocations(step.creator.as_ref().unwrap());
                step.cancel();
                for b in [0, 1, 64, 1024, 1025] {
                    assert_eq!(step.poll(b), Err(expected.clone()));
                    assert_eq!(step.last_poll_visits(), 0);
                    assert_eq!(step.total_visits(), total);
                }
                assert!(std::ptr::eq(&*step, step_pointer));
                assert_eq!(
                    drain_observed(
                        step.into_boxed_retirement(
                            LivingPublicationRetirementPolicyV2::ReleaseHandle
                        ),
                        schedule,
                        &current,
                        (kind, false, control)
                    ),
                    step_expected + 6 // E4 two absent optional owners, root-pinned.
                );
                let mut count = 0;
                if let Some(mut external) = external {
                    let mut done = false;
                    for _ in 0..10_000 {
                        match external.retirement_step() {
                            RetirementStep::Work => count += 1,
                            RetirementStep::Done => {
                                count += 1;
                                done = true;
                                break;
                            }
                            RetirementStep::Deferred(_) => panic!("unexpected external owner wait"),
                        }
                    }
                    assert!(done);
                }
                assert_eq!(count, external_expected);
            }
        }
    }
    fn cuts(kind: usize, equal: bool) -> &'static [u64] {
        match (kind, equal) {
            (0, false) => &[
                115, 115, 118, 121, 159, 183, 189, 183, 182, 181, 180, 179, 178, 177, 176, 175,
                174, 173, 172, 171, 170, 169, 168, 167, 166, 165, 164, 163, 162, 161, 160, 159,
                158, 159, 159, 159, 159, 159, 159, 159, 159, 159, 158, 159, 155, 154, 153, 152,
                151, 150, 149, 148, 147, 146, 145, 144, 143, 142, 141, 140, 139, 138, 137, 136,
                135, 134, 133, 132, 131, 130, 129, 128, 127, 126, 125, 124, 125, 173, 173, 176,
                203, 206, 206, 203, 202, 200, 200, 200, 182, 181, 180, 179, 178, 177, 176, 175,
                174, 173, 172, 171, 170, 169, 168, 167, 166, 165, 164, 163, 162, 161, 160, 159,
                158, 157, 156, 155, 154, 153, 152, 151, 150, 149, 148, 147, 146, 145, 144, 143,
                142, 141, 140, 139, 138, 137, 136, 135, 134, 133, 132, 131, 130, 129, 128, 127,
                128, 125, 122, 122,
            ],
            (0, true) => &[
                115, 115, 118, 121, 159, 183, 189, 183, 182, 181, 180, 179, 178, 177, 176, 175,
                174, 173, 172, 171, 170, 169, 168, 167, 166, 165, 164, 163, 162, 161, 160, 159,
                158, 159, 159, 159, 159, 159, 158, 157, 156, 155, 154, 153, 152, 151, 152, 152,
                152, 152, 152, 151, 152, 149, 148, 147, 146, 145, 144, 143, 142, 141, 140, 139,
                138, 137, 136, 135, 134, 133, 132, 131, 130, 129, 128, 127, 126, 125, 124, 123,
                122, 121, 120, 119, 118, 119, 167, 167, 170, 197, 200, 200, 197, 196, 194, 194,
                194, 176, 175, 174, 173, 172, 171, 170, 169, 168, 167, 166, 165, 164, 163, 162,
                161, 160, 159, 158, 157, 156, 155, 154, 153, 152, 151, 150, 149, 148, 147, 146,
                145, 144, 143, 142, 141, 140, 139, 138, 137, 136, 135, 134, 133, 132, 131, 130,
                129, 128, 127, 126, 125, 124, 123, 122, 121, 122, 119, 116, 116,
            ],
            (1, false) => &[
                100, 100, 103, 106, 145, 169, 175, 169, 168, 167, 166, 165, 164, 163, 162, 161,
                160, 159, 158, 157, 156, 155, 154, 153, 152, 151, 150, 149, 148, 147, 146, 145,
                144, 145, 145, 145, 145, 145, 145, 145, 145, 145, 144, 145, 137, 136, 135, 134,
                133, 132, 131, 130, 129, 128, 127, 126, 125, 124, 123, 122, 121, 120, 119, 118,
                117, 116, 115, 114, 113, 112, 111, 110, 109, 108, 107, 106, 107, 155, 155, 158,
                185, 188, 188, 185, 184, 182, 182, 182, 164, 163, 162, 161, 160, 159, 158, 157,
                156, 155, 154, 153, 152, 151, 150, 149, 148, 147, 146, 145, 144, 143, 142, 141,
                140, 139, 138, 137, 136, 135, 134, 133, 132, 131, 130, 129, 128, 127, 126, 125,
                124, 123, 122, 121, 120, 119, 118, 117, 116, 115, 114, 113, 112, 111, 110, 109,
                110, 107, 104, 104,
            ],
            (1, true) => &[
                100, 100, 103, 106, 145, 169, 175, 169, 168, 167, 166, 165, 164, 163, 162, 161,
                160, 159, 158, 157, 156, 155, 154, 153, 152, 151, 150, 149, 148, 147, 146, 145,
                144, 145, 145, 145, 145, 145, 144, 143, 142, 141, 142, 142, 142, 142, 142, 141,
                142, 136, 135, 134, 133, 132, 131, 130, 129, 128, 127, 126, 125, 124, 123, 122,
                121, 120, 119, 118, 117, 116, 115, 114, 113, 112, 111, 110, 109, 108, 107, 106,
                105, 106, 154, 154, 157, 184, 187, 187, 184, 183, 181, 181, 181, 163, 162, 161,
                160, 159, 158, 157, 156, 155, 154, 153, 152, 151, 150, 149, 148, 147, 146, 145,
                144, 143, 142, 141, 140, 139, 138, 137, 136, 135, 134, 133, 132, 131, 130, 129,
                128, 127, 126, 125, 124, 123, 122, 121, 120, 119, 118, 117, 116, 115, 114, 113,
                112, 111, 110, 109, 108, 109, 106, 103, 103,
            ],
            (2, false) => &[
                100, 100, 103, 106, 145, 169, 175, 169, 168, 167, 166, 165, 164, 163, 162, 161,
                160, 159, 158, 157, 156, 155, 154, 153, 152, 151, 150, 149, 148, 147, 146, 145,
                144, 145, 145, 145, 145, 145, 145, 145, 145, 145, 144, 145, 137, 136, 135, 134,
                133, 132, 131, 130, 129, 128, 127, 126, 125, 124, 123, 122, 121, 120, 119, 118,
                117, 116, 115, 114, 113, 112, 111, 110, 109, 108, 107, 106, 107, 155, 155, 158,
                185, 188, 188, 185, 184, 182, 182, 182, 164, 163, 162, 161, 160, 159, 158, 157,
                156, 155, 154, 153, 152, 151, 150, 149, 148, 147, 146, 145, 144, 143, 142, 141,
                140, 139, 138, 137, 136, 135, 134, 133, 132, 131, 130, 129, 128, 127, 126, 125,
                124, 123, 122, 121, 120, 119, 118, 117, 116, 115, 114, 113, 112, 111, 110, 109,
                110, 107, 104, 104,
            ],
            (2, true) => &[
                100, 100, 103, 106, 145, 169, 175, 169, 168, 167, 166, 165, 164, 163, 162, 161,
                160, 159, 158, 157, 156, 155, 154, 153, 152, 151, 150, 149, 148, 147, 146, 145,
                144, 145, 145, 145, 145, 145, 144, 143, 142, 141, 142, 142, 142, 142, 142, 141,
                142, 136, 135, 134, 133, 132, 131, 130, 129, 128, 127, 126, 125, 124, 123, 122,
                121, 120, 119, 118, 117, 116, 115, 114, 113, 112, 111, 110, 109, 108, 107, 106,
                105, 106, 154, 154, 157, 184, 187, 187, 184, 183, 181, 181, 181, 163, 162, 161,
                160, 159, 158, 157, 156, 155, 154, 153, 152, 151, 150, 149, 148, 147, 146, 145,
                144, 143, 142, 141, 140, 139, 138, 137, 136, 135, 134, 133, 132, 131, 130, 129,
                128, 127, 126, 125, 124, 123, 122, 121, 120, 119, 118, 117, 116, 115, 114, 113,
                112, 111, 110, 109, 108, 109, 106, 103, 103,
            ],
            (3, false) => &[
                130, 130, 133, 136, 174, 198, 204, 198, 197, 196, 195, 194, 193, 192, 191, 190,
                189, 188, 187, 186, 185, 184, 183, 182, 181, 180, 179, 178, 177, 176, 175, 174,
                173, 174, 174, 174, 174, 174, 174, 174, 174, 174, 173, 174, 170, 169, 168, 167,
                166, 165, 164, 163, 162, 161, 160, 159, 158, 157, 156, 155, 154, 153, 152, 151,
                150, 149, 148, 147, 146, 145, 144, 143, 142, 141, 140, 139, 140, 188, 188, 191,
                218, 221, 221, 218, 217, 215, 215, 215, 197, 196, 195, 194, 193, 192, 191, 190,
                189, 188, 187, 186, 185, 184, 183, 182, 181, 180, 179, 178, 177, 176, 175, 174,
                173, 172, 171, 170, 169, 168, 167, 166, 165, 164, 163, 162, 161, 160, 159, 158,
                157, 156, 155, 154, 153, 152, 151, 150, 149, 148, 147, 146, 145, 144, 143, 142,
                143, 140, 137, 137,
            ],
            (3, true) => &[
                130, 130, 133, 136, 174, 198, 204, 198, 197, 196, 195, 194, 193, 192, 191, 190,
                189, 188, 187, 186, 185, 184, 183, 182, 181, 180, 179, 178, 177, 176, 175, 174,
                173, 174, 174, 174, 174, 174, 173, 172, 171, 170, 169, 168, 167, 166, 165, 164,
                163, 162, 161, 162, 162, 162, 162, 162, 161, 162, 159, 158, 157, 156, 155, 154,
                153, 152, 151, 150, 149, 148, 147, 146, 145, 144, 143, 142, 141, 140, 139, 138,
                137, 136, 135, 134, 133, 132, 131, 130, 129, 128, 129, 177, 177, 180, 207, 210,
                210, 207, 206, 204, 204, 204, 186, 185, 184, 183, 182, 181, 180, 179, 178, 177,
                176, 175, 174, 173, 172, 171, 170, 169, 168, 167, 166, 165, 164, 163, 162, 161,
                160, 159, 158, 157, 156, 155, 154, 153, 152, 151, 150, 149, 148, 147, 146, 145,
                144, 143, 142, 141, 140, 139, 138, 137, 136, 135, 134, 133, 132, 131, 132, 129,
                126, 126,
            ],
            (4, false) => &[
                108, 108, 111, 114, 158, 182, 188, 182, 181, 180, 179, 178, 177, 176, 175, 174,
                173, 172, 171, 170, 169, 168, 167, 166, 165, 164, 163, 162, 161, 160, 159, 158,
                157, 158, 158, 158, 158, 158, 158, 158, 158, 158, 157, 158, 164, 180, 179, 178,
                177, 176, 175, 174, 173, 172, 171, 170, 169, 168, 167, 166, 165, 164, 163, 162,
                161, 160, 159, 158, 157, 156, 155, 154, 153, 152, 151, 150, 149, 148, 147, 146,
                145, 144, 143, 142, 141, 140, 139, 138, 137, 136, 135, 134, 133, 132, 131, 130,
                129, 128, 127, 126, 125, 124, 123, 122, 121, 120, 119, 118, 117, 116, 115, 114,
                115, 163, 163, 166, 193, 196, 196, 193, 192, 190, 190, 190, 172, 171, 170, 169,
                168, 167, 166, 165, 164, 163, 162, 161, 160, 159, 158, 157, 156, 155, 154, 153,
                152, 151, 150, 149, 148, 147, 146, 145, 144, 143, 142, 141, 140, 139, 138, 137,
                136, 135, 134, 133, 132, 131, 130, 129, 128, 127, 126, 125, 124, 123, 122, 121,
                120, 119, 118, 117, 118, 115, 112, 112,
            ],
            (4, true) => &[
                108, 108, 111, 114, 158, 182, 188, 182, 181, 180, 179, 178, 177, 176, 175, 174,
                173, 172, 171, 170, 169, 168, 167, 166, 165, 164, 163, 162, 161, 160, 159, 158,
                157, 158, 158, 158, 158, 158, 157, 156, 155, 154, 155, 155, 155, 155, 155, 154,
                155, 161, 179, 178, 177, 176, 175, 174, 173, 172, 171, 170, 169, 168, 167, 166,
                165, 164, 163, 162, 161, 160, 159, 158, 157, 156, 155, 154, 153, 152, 151, 150,
                149, 148, 147, 146, 145, 144, 143, 142, 141, 140, 139, 138, 137, 136, 135, 134,
                133, 132, 131, 130, 129, 128, 127, 126, 125, 124, 123, 122, 121, 120, 119, 118,
                117, 116, 115, 114, 113, 114, 162, 162, 165, 192, 195, 195, 192, 191, 189, 189,
                189, 171, 170, 169, 168, 167, 166, 165, 164, 163, 162, 161, 160, 159, 158, 157,
                156, 155, 154, 153, 152, 151, 150, 149, 148, 147, 146, 145, 144, 143, 142, 141,
                140, 139, 138, 137, 136, 135, 134, 133, 132, 131, 130, 129, 128, 127, 126, 125,
                124, 123, 122, 121, 120, 119, 118, 117, 116, 117, 114, 111, 111,
            ],
            (5, false) => &[
                100, 100, 103, 106, 145, 169, 175, 169, 168, 167, 166, 165, 164, 163, 162, 161,
                160, 159, 158, 157, 156, 155, 154, 153, 152, 151, 150, 149, 148, 147, 146, 145,
                144, 145, 145, 145, 145, 145, 145, 145, 145, 145, 144, 145, 137, 136, 135, 134,
                133, 132, 131, 130, 129, 128, 127, 126, 125, 124, 123, 122, 121, 120, 119, 118,
                117, 116, 115, 114, 113, 112, 111, 110, 109, 108, 107, 106, 107, 155, 155, 158,
                185, 188, 188, 185, 184, 182, 182, 182, 164, 163, 162, 161, 160, 159, 158, 157,
                156, 155, 154, 153, 152, 151, 150, 149, 148, 147, 146, 145, 144, 143, 142, 141,
                140, 139, 138, 137, 136, 135, 134, 133, 132, 131, 130, 129, 128, 127, 126, 125,
                124, 123, 122, 121, 120, 119, 118, 117, 116, 115, 114, 113, 112, 111, 110, 109,
                110, 107, 104, 104,
            ],
            (5, true) => &[
                100, 100, 103, 106, 145, 169, 175, 169, 168, 167, 166, 165, 164, 163, 162, 161,
                160, 159, 158, 157, 156, 155, 154, 153, 152, 151, 150, 149, 148, 147, 146, 145,
                144, 145, 145, 145, 145, 145, 144, 143, 142, 141, 142, 142, 142, 142, 142, 141,
                142, 136, 135, 134, 133, 132, 131, 130, 129, 128, 127, 126, 125, 124, 123, 122,
                121, 120, 119, 118, 117, 116, 115, 114, 113, 112, 111, 110, 109, 108, 107, 106,
                105, 106, 154, 154, 157, 184, 187, 187, 184, 183, 181, 181, 181, 163, 162, 161,
                160, 159, 158, 157, 156, 155, 154, 153, 152, 151, 150, 149, 148, 147, 146, 145,
                144, 143, 142, 141, 140, 139, 138, 137, 136, 135, 134, 133, 132, 131, 130, 129,
                128, 127, 126, 125, 124, 123, 122, 121, 120, 119, 118, 117, 116, 115, 114, 113,
                112, 111, 110, 109, 108, 109, 106, 103, 103,
            ],
            (6, false) => &[
                100, 100, 103, 106, 145, 169, 175, 169, 168, 167, 166, 165, 164, 163, 162, 161,
                160, 159, 158, 157, 156, 155, 154, 153, 152, 151, 150, 149, 148, 147, 146, 145,
                144, 145, 145, 145, 145, 145, 145, 145, 145, 145, 144, 145, 137, 136, 135, 134,
                133, 132, 131, 130, 129, 128, 127, 126, 125, 124, 123, 122, 121, 120, 119, 118,
                117, 116, 115, 114, 113, 112, 111, 110, 109, 108, 107, 106, 107, 155, 155, 158,
                185, 188, 188, 185, 184, 182, 182, 182, 164, 163, 162, 161, 160, 159, 158, 157,
                156, 155, 154, 153, 152, 151, 150, 149, 148, 147, 146, 145, 144, 143, 142, 141,
                140, 139, 138, 137, 136, 135, 134, 133, 132, 131, 130, 129, 128, 127, 126, 125,
                124, 123, 122, 121, 120, 119, 118, 117, 116, 115, 114, 113, 112, 111, 110, 109,
                110, 107, 104, 104,
            ],
            (6, true) => &[
                100, 100, 103, 106, 145, 169, 175, 169, 168, 167, 166, 165, 164, 163, 162, 161,
                160, 159, 158, 157, 156, 155, 154, 153, 152, 151, 150, 149, 148, 147, 146, 145,
                144, 145, 145, 145, 145, 145, 144, 143, 142, 141, 142, 142, 142, 142, 142, 141,
                142, 136, 135, 134, 133, 132, 131, 130, 129, 128, 127, 126, 125, 124, 123, 122,
                121, 120, 119, 118, 117, 116, 115, 114, 113, 112, 111, 110, 109, 108, 107, 106,
                105, 106, 154, 154, 157, 184, 187, 187, 184, 183, 181, 181, 181, 163, 162, 161,
                160, 159, 158, 157, 156, 155, 154, 153, 152, 151, 150, 149, 148, 147, 146, 145,
                144, 143, 142, 141, 140, 139, 138, 137, 136, 135, 134, 133, 132, 131, 130, 129,
                128, 127, 126, 125, 124, 123, 122, 121, 120, 119, 118, 117, 116, 115, 114, 113,
                112, 111, 110, 109, 108, 109, 106, 103, 103,
            ],
            (7, false) => &[
                100, 100, 103, 106, 145, 169, 175, 169, 168, 167, 166, 165, 164, 163, 162, 161,
                160, 159, 158, 157, 156, 155, 154, 153, 152, 151, 150, 149, 148, 147, 146, 145,
                144, 145, 145, 145, 145, 145, 145, 145, 145, 145, 144, 145, 137, 136, 135, 134,
                133, 132, 131, 130, 129, 128, 127, 126, 125, 124, 123, 122, 121, 120, 119, 118,
                117, 116, 115, 114, 113, 112, 111, 110, 109, 108, 107, 106, 107, 155, 155, 158,
                185, 188, 188, 185, 184, 182, 182, 182, 164, 163, 162, 161, 160, 159, 158, 157,
                156, 155, 154, 153, 152, 151, 150, 149, 148, 147, 146, 145, 144, 143, 142, 141,
                140, 139, 138, 137, 136, 135, 134, 133, 132, 131, 130, 129, 128, 127, 126, 125,
                124, 123, 122, 121, 120, 119, 118, 117, 116, 115, 114, 113, 112, 111, 110, 109,
                110, 107, 104, 104,
            ],
            (7, true) => &[
                100, 100, 103, 106, 145, 169, 175, 169, 168, 167, 166, 165, 164, 163, 162, 161,
                160, 159, 158, 157, 156, 155, 154, 153, 152, 151, 150, 149, 148, 147, 146, 145,
                144, 145, 145, 145, 145, 145, 144, 143, 142, 141, 142, 142, 142, 142, 142, 141,
                142, 136, 135, 134, 133, 132, 131, 130, 129, 128, 127, 126, 125, 124, 123, 122,
                121, 120, 119, 118, 117, 116, 115, 114, 113, 112, 111, 110, 109, 108, 107, 106,
                105, 106, 154, 154, 157, 184, 187, 187, 184, 183, 181, 181, 181, 163, 162, 161,
                160, 159, 158, 157, 156, 155, 154, 153, 152, 151, 150, 149, 148, 147, 146, 145,
                144, 143, 142, 141, 140, 139, 138, 137, 136, 135, 134, 133, 132, 131, 130, 129,
                128, 127, 126, 125, 124, 123, 122, 121, 120, 119, 118, 117, 116, 115, 114, 113,
                112, 111, 110, 109, 108, 109, 106, 103, 103,
            ],
            _ => unreachable!(),
        }
    }

    // Literal standalone source-only derivation: task-2e7-analytic-v1, never measured.
    const STRUCTURAL_CUTS: &[&[u64]] = &[
        &[
            183, 183, 186, 255, 255, 255, 255, 255, 279, 282, 279, 278, 277, 276, 275, 274, 273,
            272, 271, 270, 269, 268, 267, 266, 265, 264, 263, 262, 261, 260, 259, 258, 257, 256,
            255, 254, 255, 258, 258, 258, 258, 258, 258, 258, 258, 270, 294, 300, 294, 293, 292,
            291, 290, 289, 288, 287, 286, 285, 284, 283, 282, 281, 280, 279, 278, 277, 276, 275,
            274, 273, 272, 271, 270, 269, 270, 294, 300, 294, 293, 292, 291, 290, 289, 288, 287,
            286, 285, 284, 283, 282, 281, 280, 279, 278, 277, 276, 275, 274, 273, 272, 271, 270,
            269, 270, 270, 270, 270, 270, 271, 271, 304, 328, 334, 328, 327, 326, 325, 324, 323,
            322, 321, 320, 319, 318, 317, 316, 315, 314, 313, 312, 311, 310, 309, 308, 307, 306,
            305, 304, 303, 304, 304, 304, 303, 302, 301, 300, 299, 298, 299, 299, 299, 299, 299,
            299, 299, 298, 299, 296, 295, 294, 293, 292, 291, 290, 289, 288, 287, 286, 285, 284,
            283, 282, 281, 280, 279, 278, 277, 276, 275, 274, 273, 272, 271, 270, 269, 268, 267,
            266, 265, 266, 299, 323, 329, 323, 322, 321, 320, 319, 318, 317, 316, 315, 314, 313,
            312, 311, 310, 309, 308, 307, 306, 305, 304, 303, 302, 301, 300, 299, 298, 299, 299,
            299, 299, 299, 299, 299, 299, 299, 298, 299, 295, 294, 293, 292, 291, 290, 289, 288,
            287, 286, 285, 284, 283, 282, 281, 280, 279, 278, 277, 276, 275, 274, 273, 272, 271,
            270, 269, 268, 267, 266, 265, 264, 265, 265, 264, 263, 262, 261, 260, 259, 258, 257,
            256, 255, 254, 253, 252, 251, 250, 249, 248, 247, 246, 245, 244, 243, 242, 241, 240,
            239, 238, 237, 236, 235, 234, 233, 232, 231, 230, 229, 228, 227, 226, 225, 224, 223,
            222, 221, 220, 219, 218, 217, 216, 215, 214, 213, 212, 211, 210, 209, 208, 207, 206,
            205, 204, 203, 202, 201, 200, 199, 198, 197, 196, 195, 194, 193, 192, 191, 190, 189,
            190, 238, 238, 241, 268, 271, 271, 268, 267, 265, 265, 265, 247, 246, 245, 244, 243,
            242, 241, 240, 239, 238, 237, 236, 235, 234, 233, 232, 231, 230, 229, 228, 227, 226,
            225, 224, 223, 222, 221, 220, 219, 218, 217, 216, 215, 214, 213, 212, 211, 210, 209,
            208, 207, 206, 205, 204, 203, 202, 201, 200, 199, 198, 197, 196, 195, 194, 193, 192,
            193, 190, 190,
        ],
        &[
            183, 183, 186, 255, 255, 255, 255, 255, 279, 282, 279, 278, 277, 276, 275, 274, 273,
            272, 271, 270, 269, 268, 267, 266, 265, 264, 263, 262, 261, 260, 259, 258, 257, 256,
            255, 254, 255, 258, 258, 258, 258, 258, 258, 258, 258, 270, 294, 300, 294, 293, 292,
            291, 290, 289, 288, 287, 286, 285, 284, 283, 282, 281, 280, 279, 278, 277, 276, 275,
            274, 273, 272, 271, 270, 269, 270, 294, 300, 294, 293, 292, 291, 290, 289, 288, 287,
            286, 285, 284, 283, 282, 281, 280, 279, 278, 277, 276, 275, 274, 273, 272, 271, 270,
            269, 270, 270, 270, 270, 270, 275, 275, 308, 332, 338, 332, 331, 330, 329, 328, 327,
            326, 325, 324, 323, 322, 321, 320, 319, 318, 317, 316, 315, 314, 313, 312, 311, 310,
            309, 308, 307, 308, 308, 308, 308, 308, 307, 306, 305, 304, 303, 302, 303, 303, 303,
            303, 303, 302, 303, 300, 299, 298, 297, 296, 295, 294, 293, 292, 291, 290, 289, 288,
            287, 286, 285, 284, 283, 282, 281, 280, 279, 278, 277, 276, 275, 274, 273, 272, 271,
            270, 269, 270, 303, 327, 333, 327, 326, 325, 324, 323, 322, 321, 320, 319, 318, 317,
            316, 315, 314, 313, 312, 311, 310, 309, 308, 307, 306, 305, 304, 303, 302, 303, 303,
            303, 303, 303, 303, 303, 303, 303, 302, 303, 299, 298, 297, 296, 295, 294, 293, 292,
            291, 290, 289, 288, 287, 286, 285, 284, 283, 282, 281, 280, 279, 278, 277, 276, 275,
            274, 273, 272, 271, 270, 269, 268, 269, 269, 268, 267, 266, 265, 264, 263, 262, 261,
            260, 259, 258, 257, 256, 255, 254, 253, 252, 251, 250, 249, 248, 247, 246, 245, 244,
            243, 242, 241, 240, 239, 238, 237, 236, 235, 234, 233, 232, 231, 230, 229, 228, 227,
            226, 225, 224, 223, 222, 221, 220, 219, 218, 217, 216, 215, 214, 213, 212, 211, 210,
            209, 208, 207, 206, 205, 204, 203, 202, 201, 200, 199, 198, 197, 196, 195, 194, 193,
            192, 191, 190, 189, 188, 187, 188, 236, 236, 239, 266, 269, 269, 266, 265, 263, 263,
            263, 245, 244, 243, 242, 241, 240, 239, 238, 237, 236, 235, 234, 233, 232, 231, 230,
            229, 228, 227, 226, 225, 224, 223, 222, 221, 220, 219, 218, 217, 216, 215, 214, 213,
            212, 211, 210, 209, 208, 207, 206, 205, 204, 203, 202, 201, 200, 199, 198, 197, 196,
            195, 194, 193, 192, 191, 190, 191, 188, 188,
        ],
        &[
            183, 183, 186, 255, 255, 255, 255, 255, 279, 282, 279, 278, 277, 276, 275, 274, 273,
            272, 271, 270, 269, 268, 267, 266, 265, 264, 263, 262, 261, 260, 259, 258, 257, 256,
            255, 254, 255, 258, 258, 258, 258, 258, 258, 258, 258, 270, 294, 300, 294, 293, 292,
            291, 290, 289, 288, 287, 286, 285, 284, 283, 282, 281, 280, 279, 278, 277, 276, 275,
            274, 273, 272, 271, 270, 269, 270, 294, 300, 294, 293, 292, 291, 290, 289, 288, 287,
            286, 285, 284, 283, 282, 281, 280, 279, 278, 277, 276, 275, 274, 273, 272, 271, 270,
            269, 270, 270, 270, 270, 270, 341, 341, 374, 398, 404, 398, 397, 396, 395, 394, 393,
            392, 391, 390, 389, 388, 387, 386, 385, 384, 383, 382, 381, 380, 379, 378, 377, 376,
            375, 374, 373, 374, 374, 374, 374, 374, 374, 374, 373, 372, 371, 370, 369, 368, 369,
            369, 369, 368, 369, 366, 365, 364, 363, 362, 361, 360, 359, 358, 357, 356, 355, 354,
            353, 352, 351, 350, 349, 348, 347, 346, 345, 344, 343, 342, 341, 340, 339, 338, 337,
            336, 335, 336, 369, 393, 399, 393, 392, 391, 390, 389, 388, 387, 386, 385, 384, 383,
            382, 381, 380, 379, 378, 377, 376, 375, 374, 373, 372, 371, 370, 369, 368, 369, 369,
            369, 369, 369, 369, 369, 369, 369, 368, 369, 365, 364, 363, 362, 361, 360, 359, 358,
            357, 356, 355, 354, 353, 352, 351, 350, 349, 348, 347, 346, 345, 344, 343, 342, 341,
            340, 339, 338, 337, 336, 335, 334, 335, 335, 334, 333, 332, 331, 330, 329, 328, 327,
            326, 325, 324, 323, 322, 321, 320, 319, 318, 317, 316, 315, 314, 313, 312, 311, 310,
            309, 308, 307, 306, 305, 304, 303, 302, 301, 300, 299, 298, 297, 296, 295, 294, 293,
            292, 291, 290, 289, 288, 287, 286, 285, 284, 283, 282, 281, 280, 279, 278, 277, 276,
            275, 274, 273, 272, 271, 270, 269, 268, 267, 266, 265, 264, 263, 262, 261, 260, 259,
            258, 257, 256, 255, 254, 253, 252, 251, 250, 249, 248, 247, 246, 245, 244, 243, 242,
            241, 240, 239, 238, 237, 236, 235, 234, 233, 232, 231, 230, 229, 228, 227, 226, 225,
            224, 223, 222, 221, 220, 219, 218, 217, 216, 215, 214, 213, 212, 211, 210, 209, 208,
            207, 206, 205, 204, 203, 202, 201, 200, 199, 198, 197, 196, 195, 194, 193, 192, 191,
            190, 189, 188, 187, 186, 185, 184, 183, 182, 181, 180, 179, 178, 177, 176, 175, 174,
            173, 172, 171, 170, 169, 168, 167, 166, 165, 164, 163, 162, 161, 160, 159, 158, 157,
            156, 155, 154, 155, 203, 203, 206, 233, 236, 236, 233, 232, 230, 230, 230, 212, 211,
            210, 209, 208, 207, 206, 205, 204, 203, 202, 201, 200, 199, 198, 197, 196, 195, 194,
            193, 192, 191, 190, 189, 188, 187, 186, 185, 184, 183, 182, 181, 180, 179, 178, 177,
            176, 175, 174, 173, 172, 171, 170, 169, 168, 167, 166, 165, 164, 163, 162, 161, 160,
            159, 158, 157, 158, 155, 155,
        ],
        &[
            183, 183, 183, 252, 252, 252, 252, 252, 252, 252, 252, 252, 252, 264, 276, 300, 306,
            300, 299, 298, 297, 296, 295, 294, 293, 292, 291, 290, 289, 288, 287, 286, 285, 284,
            283, 282, 281, 280, 279, 278, 277, 276, 275, 276, 276, 276, 276, 276, 276, 276, 276,
            276, 272, 275, 308, 332, 338, 332, 331, 330, 329, 328, 327, 326, 325, 324, 323, 322,
            321, 320, 319, 318, 317, 316, 315, 314, 313, 312, 311, 310, 309, 308, 307, 308, 308,
            308, 307, 306, 305, 304, 303, 302, 303, 303, 303, 303, 303, 303, 302, 303, 301, 300,
            299, 298, 297, 296, 295, 294, 293, 292, 291, 290, 289, 288, 287, 286, 285, 284, 283,
            282, 281, 280, 279, 278, 277, 276, 275, 274, 273, 272, 271, 270, 269, 268, 267, 268,
            300, 324, 330, 324, 323, 322, 321, 320, 319, 318, 317, 316, 315, 314, 313, 312, 311,
            310, 309, 308, 307, 306, 305, 304, 303, 302, 301, 300, 299, 300, 300, 300, 299, 298,
            297, 296, 295, 294, 295, 295, 295, 295, 295, 294, 295, 293, 292, 291, 290, 289, 288,
            287, 286, 285, 284, 283, 282, 281, 280, 279, 278, 277, 276, 275, 274, 273, 272, 271,
            270, 269, 268, 267, 266, 265, 264, 263, 262, 263, 263, 262, 261, 260, 259, 258, 257,
            256, 255, 254, 253, 252, 251, 250, 249, 248, 247, 246, 245, 244, 243, 242, 241, 240,
            239, 238, 237, 236, 235, 234, 233, 232, 231, 230, 229, 228, 227, 226, 225, 224, 223,
            222, 221, 220, 219, 218, 217, 216, 215, 214, 213, 212, 211, 210, 209, 208, 207, 206,
            205, 204, 203, 202, 201, 200, 199, 198, 197, 196, 195, 194, 193, 192, 191, 190, 189,
            188, 187, 186, 185, 184, 183, 182, 181, 180, 179, 178, 177, 176, 177, 225, 225, 228,
            255, 258, 258, 255, 254, 252, 252, 252, 234, 233, 232, 231, 230, 229, 228, 227, 226,
            225, 224, 223, 222, 221, 220, 219, 218, 217, 216, 215, 214, 213, 212, 211, 210, 209,
            208, 207, 206, 205, 204, 203, 202, 201, 200, 199, 198, 197, 196, 195, 194, 193, 192,
            191, 190, 189, 188, 187, 186, 185, 184, 183, 182, 181, 180, 179, 180, 180,
        ],
        &[
            183, 183, 183, 252, 252, 252, 252, 252, 252, 252, 252, 252, 252, 264, 276, 300, 306,
            300, 299, 298, 297, 296, 295, 294, 293, 292, 291, 290, 289, 288, 287, 286, 285, 284,
            283, 282, 281, 280, 279, 278, 277, 276, 275, 276, 276, 276, 276, 276, 276, 276, 276,
            276, 338, 341, 374, 398, 404, 398, 397, 396, 395, 394, 393, 392, 391, 390, 389, 388,
            387, 386, 385, 384, 383, 382, 381, 380, 379, 378, 377, 376, 375, 374, 373, 374, 374,
            374, 373, 372, 371, 370, 369, 368, 369, 369, 369, 369, 369, 369, 368, 369, 367, 366,
            365, 364, 363, 362, 361, 360, 359, 358, 357, 356, 355, 354, 353, 352, 351, 350, 349,
            348, 347, 346, 345, 344, 343, 342, 341, 340, 339, 338, 337, 336, 335, 334, 333, 334,
            366, 390, 396, 390, 389, 388, 387, 386, 385, 384, 383, 382, 381, 380, 379, 378, 377,
            376, 375, 374, 373, 372, 371, 370, 369, 368, 367, 366, 365, 366, 366, 366, 366, 366,
            365, 364, 363, 362, 361, 360, 361, 361, 361, 360, 361, 359, 358, 357, 356, 355, 354,
            353, 352, 351, 350, 349, 348, 347, 346, 345, 344, 343, 342, 341, 340, 339, 338, 337,
            336, 335, 334, 333, 332, 331, 330, 329, 328, 329, 329, 328, 327, 326, 325, 324, 323,
            322, 321, 320, 319, 318, 317, 316, 315, 314, 313, 312, 311, 310, 309, 308, 307, 306,
            305, 304, 303, 302, 301, 300, 299, 298, 297, 296, 295, 294, 293, 292, 291, 290, 289,
            288, 287, 286, 285, 284, 283, 282, 281, 280, 279, 278, 277, 276, 275, 274, 273, 272,
            271, 270, 269, 268, 267, 266, 265, 264, 263, 262, 261, 260, 259, 258, 257, 256, 255,
            254, 253, 252, 251, 250, 249, 248, 247, 246, 245, 244, 243, 242, 241, 240, 239, 238,
            237, 236, 235, 234, 233, 232, 231, 230, 229, 228, 227, 226, 225, 224, 223, 222, 221,
            220, 219, 218, 217, 216, 215, 214, 213, 212, 211, 210, 209, 208, 207, 206, 205, 204,
            203, 202, 201, 200, 199, 198, 197, 196, 195, 194, 193, 192, 191, 190, 189, 188, 187,
            186, 185, 184, 183, 182, 181, 180, 179, 178, 177, 176, 175, 174, 173, 172, 171, 170,
            169, 168, 167, 166, 165, 164, 163, 162, 161, 160, 159, 158, 157, 156, 155, 154, 153,
            152, 151, 150, 149, 148, 147, 146, 145, 144, 143, 144, 192, 192, 195, 222, 225, 225,
            222, 221, 219, 219, 219, 201, 200, 199, 198, 197, 196, 195, 194, 193, 192, 191, 190,
            189, 188, 187, 186, 185, 184, 183, 182, 181, 180, 179, 178, 177, 176, 175, 174, 173,
            172, 171, 170, 169, 168, 167, 166, 165, 164, 163, 162, 161, 160, 159, 158, 157, 156,
            155, 154, 153, 152, 151, 150, 149, 148, 147, 146, 147, 147,
        ],
        &[
            183, 183, 183, 252, 252, 252, 252, 252, 252, 252, 252, 252, 252, 264, 276, 300, 306,
            300, 299, 298, 297, 296, 295, 294, 293, 292, 291, 290, 289, 288, 287, 286, 285, 284,
            283, 282, 281, 280, 279, 278, 277, 276, 275, 276, 276, 276, 276, 276, 276, 276, 276,
            276, 272, 275, 308, 332, 338, 332, 331, 330, 329, 328, 327, 326, 325, 324, 323, 322,
            321, 320, 319, 318, 317, 316, 315, 314, 313, 312, 311, 310, 309, 308, 307, 308, 308,
            308, 308, 308, 307, 306, 305, 304, 303, 302, 303, 303, 303, 303, 302, 303, 301, 300,
            299, 298, 297, 296, 295, 294, 293, 292, 291, 290, 289, 288, 287, 286, 285, 284, 283,
            282, 281, 280, 279, 278, 277, 276, 275, 274, 273, 272, 271, 270, 269, 268, 267, 268,
            300, 324, 330, 324, 323, 322, 321, 320, 319, 318, 317, 316, 315, 314, 313, 312, 311,
            310, 309, 308, 307, 306, 305, 304, 303, 302, 301, 300, 299, 300, 300, 300, 299, 298,
            297, 296, 295, 294, 295, 295, 295, 295, 295, 294, 295, 293, 292, 291, 290, 289, 288,
            287, 286, 285, 284, 283, 282, 281, 280, 279, 278, 277, 276, 275, 274, 273, 272, 271,
            270, 269, 268, 267, 266, 265, 264, 263, 262, 263, 263, 262, 261, 260, 259, 258, 257,
            256, 255, 254, 253, 252, 251, 250, 249, 248, 247, 246, 245, 244, 243, 242, 241, 240,
            239, 238, 237, 236, 235, 234, 233, 232, 231, 230, 229, 228, 227, 226, 225, 224, 223,
            222, 221, 220, 219, 218, 217, 216, 215, 214, 213, 212, 211, 210, 209, 208, 207, 206,
            205, 204, 203, 202, 201, 200, 199, 198, 197, 196, 195, 194, 193, 192, 191, 190, 189,
            188, 187, 186, 185, 184, 183, 182, 181, 180, 179, 178, 177, 176, 177, 225, 225, 228,
            255, 258, 258, 255, 254, 252, 252, 252, 234, 233, 232, 231, 230, 229, 228, 227, 226,
            225, 224, 223, 222, 221, 220, 219, 218, 217, 216, 215, 214, 213, 212, 211, 210, 209,
            208, 207, 206, 205, 204, 203, 202, 201, 200, 199, 198, 197, 196, 195, 194, 193, 192,
            191, 190, 189, 188, 187, 186, 185, 184, 183, 182, 181, 180, 179, 180, 180,
        ],
        &[
            183, 183, 183, 252, 252, 252, 252, 252, 252, 252, 252, 252, 252, 264, 276, 300, 306,
            300, 299, 298, 297, 296, 295, 294, 293, 292, 291, 290, 289, 288, 287, 286, 285, 284,
            283, 282, 281, 280, 279, 278, 277, 276, 275, 276, 276, 276, 276, 276, 276, 276, 276,
            276, 342, 345, 378, 402, 408, 402, 401, 400, 399, 398, 397, 396, 395, 394, 393, 392,
            391, 390, 389, 388, 387, 386, 385, 384, 383, 382, 381, 380, 379, 378, 377, 378, 378,
            378, 378, 378, 377, 376, 375, 374, 373, 372, 373, 373, 373, 373, 372, 373, 371, 370,
            369, 368, 367, 366, 365, 364, 363, 362, 361, 360, 359, 358, 357, 356, 355, 354, 353,
            352, 351, 350, 349, 348, 347, 346, 345, 344, 343, 342, 341, 340, 339, 338, 337, 338,
            370, 394, 400, 394, 393, 392, 391, 390, 389, 388, 387, 386, 385, 384, 383, 382, 381,
            380, 379, 378, 377, 376, 375, 374, 373, 372, 371, 370, 369, 370, 370, 370, 370, 370,
            369, 368, 367, 366, 365, 364, 365, 365, 365, 364, 365, 363, 362, 361, 360, 359, 358,
            357, 356, 355, 354, 353, 352, 351, 350, 349, 348, 347, 346, 345, 344, 343, 342, 341,
            340, 339, 338, 337, 336, 335, 334, 333, 332, 333, 333, 332, 331, 330, 329, 328, 327,
            326, 325, 324, 323, 322, 321, 320, 319, 318, 317, 316, 315, 314, 313, 312, 311, 310,
            309, 308, 307, 306, 305, 304, 303, 302, 301, 300, 299, 298, 297, 296, 295, 294, 293,
            292, 291, 290, 289, 288, 287, 286, 285, 284, 283, 282, 281, 280, 279, 278, 277, 276,
            275, 274, 273, 272, 271, 270, 269, 268, 267, 266, 265, 264, 263, 262, 261, 260, 259,
            258, 257, 256, 255, 254, 253, 252, 251, 250, 249, 248, 247, 246, 245, 244, 243, 242,
            241, 240, 239, 238, 237, 236, 235, 234, 233, 232, 231, 230, 229, 228, 227, 226, 225,
            224, 223, 222, 221, 220, 219, 218, 217, 216, 215, 214, 213, 212, 211, 210, 209, 208,
            207, 206, 205, 204, 203, 202, 201, 200, 199, 198, 197, 196, 195, 194, 193, 192, 191,
            190, 189, 188, 187, 186, 185, 184, 183, 182, 181, 180, 179, 178, 177, 176, 175, 174,
            173, 172, 171, 170, 169, 168, 167, 166, 165, 164, 163, 162, 161, 160, 159, 158, 157,
            156, 155, 154, 153, 152, 151, 150, 149, 148, 147, 146, 145, 144, 143, 142, 141, 142,
            190, 190, 193, 220, 223, 223, 220, 219, 217, 217, 217, 199, 198, 197, 196, 195, 194,
            193, 192, 191, 190, 189, 188, 187, 186, 185, 184, 183, 182, 181, 180, 179, 178, 177,
            176, 175, 174, 173, 172, 171, 170, 169, 168, 167, 166, 165, 164, 163, 162, 161, 160,
            159, 158, 157, 156, 155, 154, 153, 152, 151, 150, 149, 148, 147, 146, 145, 144, 145,
            145,
        ],
        &[
            183, 183, 183, 252, 252, 252, 252, 252, 252, 252, 252, 252, 252, 264, 276, 300, 306,
            300, 299, 298, 297, 296, 295, 294, 293, 292, 291, 290, 289, 288, 287, 286, 285, 284,
            283, 282, 281, 280, 279, 278, 277, 276, 275, 276, 276, 276, 276, 276, 276, 276, 276,
            276, 338, 341, 374, 398, 404, 398, 397, 396, 395, 394, 393, 392, 391, 390, 389, 388,
            387, 386, 385, 384, 383, 382, 381, 380, 379, 378, 377, 376, 375, 374, 373, 374, 374,
            374, 374, 374, 374, 374, 373, 372, 371, 370, 369, 368, 369, 369, 368, 369, 367, 366,
            365, 364, 363, 362, 361, 360, 359, 358, 357, 356, 355, 354, 353, 352, 351, 350, 349,
            348, 347, 346, 345, 344, 343, 342, 341, 340, 339, 338, 337, 336, 335, 334, 333, 334,
            366, 390, 396, 390, 389, 388, 387, 386, 385, 384, 383, 382, 381, 380, 379, 378, 377,
            376, 375, 374, 373, 372, 371, 370, 369, 368, 367, 366, 365, 366, 366, 366, 365, 364,
            363, 362, 361, 360, 361, 361, 361, 361, 361, 360, 361, 359, 358, 357, 356, 355, 354,
            353, 352, 351, 350, 349, 348, 347, 346, 345, 344, 343, 342, 341, 340, 339, 338, 337,
            336, 335, 334, 333, 332, 331, 330, 329, 328, 329, 329, 328, 327, 326, 325, 324, 323,
            322, 321, 320, 319, 318, 317, 316, 315, 314, 313, 312, 311, 310, 309, 308, 307, 306,
            305, 304, 303, 302, 301, 300, 299, 298, 297, 296, 295, 294, 293, 292, 291, 290, 289,
            288, 287, 286, 285, 284, 283, 282, 281, 280, 279, 278, 277, 276, 275, 274, 273, 272,
            271, 270, 269, 268, 267, 266, 265, 264, 263, 262, 261, 260, 259, 258, 257, 256, 255,
            254, 253, 252, 251, 250, 249, 248, 247, 246, 245, 244, 243, 242, 241, 240, 239, 238,
            237, 236, 235, 234, 233, 232, 231, 230, 229, 228, 227, 226, 225, 224, 223, 222, 221,
            220, 219, 218, 217, 216, 215, 214, 213, 212, 211, 210, 209, 208, 207, 206, 205, 204,
            203, 202, 201, 200, 199, 198, 197, 196, 195, 194, 193, 192, 191, 190, 189, 188, 187,
            186, 185, 184, 183, 182, 181, 180, 179, 178, 177, 176, 175, 174, 173, 172, 171, 170,
            169, 168, 167, 166, 165, 164, 163, 162, 161, 160, 159, 158, 157, 156, 155, 154, 153,
            152, 151, 150, 149, 148, 147, 146, 145, 144, 143, 144, 192, 192, 195, 222, 225, 225,
            222, 221, 219, 219, 219, 201, 200, 199, 198, 197, 196, 195, 194, 193, 192, 191, 190,
            189, 188, 187, 186, 185, 184, 183, 182, 181, 180, 179, 178, 177, 176, 175, 174, 173,
            172, 171, 170, 169, 168, 167, 166, 165, 164, 163, 162, 161, 160, 159, 158, 157, 156,
            155, 154, 153, 152, 151, 150, 149, 148, 147, 146, 147, 147,
        ],
        &[
            183, 183, 183, 252, 252, 252, 252, 252, 252, 252, 252, 252, 252, 264, 276, 300, 306,
            300, 299, 298, 297, 296, 295, 294, 293, 292, 291, 290, 289, 288, 287, 286, 285, 284,
            283, 282, 281, 280, 279, 278, 277, 276, 275, 276, 276, 276, 276, 276, 276, 276, 276,
            276, 342, 345, 378, 402, 408, 402, 401, 400, 399, 398, 397, 396, 395, 394, 393, 392,
            391, 390, 389, 388, 387, 386, 385, 384, 383, 382, 381, 380, 379, 378, 377, 378, 378,
            378, 378, 378, 378, 378, 377, 376, 375, 374, 373, 372, 373, 373, 372, 373, 371, 370,
            369, 368, 367, 366, 365, 364, 363, 362, 361, 360, 359, 358, 357, 356, 355, 354, 353,
            352, 351, 350, 349, 348, 347, 346, 345, 344, 343, 342, 341, 340, 339, 338, 337, 338,
            370, 394, 400, 394, 393, 392, 391, 390, 389, 388, 387, 386, 385, 384, 383, 382, 381,
            380, 379, 378, 377, 376, 375, 374, 373, 372, 371, 370, 369, 370, 370, 370, 370, 370,
            369, 368, 367, 366, 365, 364, 365, 365, 365, 364, 365, 363, 362, 361, 360, 359, 358,
            357, 356, 355, 354, 353, 352, 351, 350, 349, 348, 347, 346, 345, 344, 343, 342, 341,
            340, 339, 338, 337, 336, 335, 334, 333, 332, 333, 333, 332, 331, 330, 329, 328, 327,
            326, 325, 324, 323, 322, 321, 320, 319, 318, 317, 316, 315, 314, 313, 312, 311, 310,
            309, 308, 307, 306, 305, 304, 303, 302, 301, 300, 299, 298, 297, 296, 295, 294, 293,
            292, 291, 290, 289, 288, 287, 286, 285, 284, 283, 282, 281, 280, 279, 278, 277, 276,
            275, 274, 273, 272, 271, 270, 269, 268, 267, 266, 265, 264, 263, 262, 261, 260, 259,
            258, 257, 256, 255, 254, 253, 252, 251, 250, 249, 248, 247, 246, 245, 244, 243, 242,
            241, 240, 239, 238, 237, 236, 235, 234, 233, 232, 231, 230, 229, 228, 227, 226, 225,
            224, 223, 222, 221, 220, 219, 218, 217, 216, 215, 214, 213, 212, 211, 210, 209, 208,
            207, 206, 205, 204, 203, 202, 201, 200, 199, 198, 197, 196, 195, 194, 193, 192, 191,
            190, 189, 188, 187, 186, 185, 184, 183, 182, 181, 180, 179, 178, 177, 176, 175, 174,
            173, 172, 171, 170, 169, 168, 167, 166, 165, 164, 163, 162, 161, 160, 159, 158, 157,
            156, 155, 154, 153, 152, 151, 150, 149, 148, 147, 146, 145, 144, 143, 142, 141, 142,
            190, 190, 193, 220, 223, 223, 220, 219, 217, 217, 217, 199, 198, 197, 196, 195, 194,
            193, 192, 191, 190, 189, 188, 187, 186, 185, 184, 183, 182, 181, 180, 179, 178, 177,
            176, 175, 174, 173, 172, 171, 170, 169, 168, 167, 166, 165, 164, 163, 162, 161, 160,
            159, 158, 157, 156, 155, 154, 153, 152, 151, 150, 149, 148, 147, 146, 145, 144, 145,
            145,
        ],
    ];
    const STRUCTURAL_DEADLINES: &[[usize; 6]] = &[
        [155, 285, usize::MAX, usize::MAX, usize::MAX, 348],
        [155, 285, usize::MAX, 316, usize::MAX, 354],
        [155, 285, usize::MAX, 415, usize::MAX, 453],
        [98, 238, 244, usize::MAX, 275, 310],
        [98, 238, 244, usize::MAX, 374, 409],
        [98, 238, 244, 272, usize::MAX, 310],
        [98, 238, 244, 272, 380, 415],
        [98, 238, 244, 371, usize::MAX, 409],
        [98, 238, 244, 371, 380, 415],
    ];
    #[test]
    fn structural_creator_every_original_owner_cut_matches_independent_map() {
        use crate::living::simulation::fleet_structural_creator_tests as fixture;
        for (kind, cuts) in STRUCTURAL_CUTS.iter().enumerate() {
            for (cut, &expected) in cuts.iter().enumerate() {
                for schedule in fixture::SCHEDULES {
                    let (creator, queue, catalog) = fixture::source(kind);
                    let anchor = crate::living::builtin_living_sandbox_v2().unwrap();
                    let mut step = captured_step(creator, anchor.capture_boundary().unwrap());
                    let step_pointer = &*step as *const _;
                    let creator = step.creator.as_mut().unwrap();
                    let pointer = &**creator as *const _;
                    let (source, target) = fixture::parents(kind);
                    let original_fleets = creator.state.fleets.as_slice().as_ptr() as usize;
                    let source_hulls = creator.state.fleets.as_slice()[source]
                        .hulls
                        .as_slice()
                        .as_ptr() as usize;
                    let target_hulls = creator.state.fleets.as_slice()[target]
                        .hulls
                        .as_slice()
                        .as_ptr() as usize;
                    let source_path = creator.state.fleets.as_slice()[source]
                        .order
                        .as_ref()
                        .map(|order| order.lane_path.as_ptr() as usize);
                    let target_path = creator.state.fleets.as_slice()[target]
                        .order
                        .as_ref()
                        .map(|order| order.lane_path.as_ptr() as usize);
                    let collector = creator.events.payloads().as_ptr() as usize;
                    let probes = allocations(creator);
                    assert!(probes.iter().any(|p| p.1 == 0 && p.2 == Some(7)));
                    let ends = STRUCTURAL_DEADLINES[kind];
                    let probes: Vec<_> = probes
                        .into_iter()
                        .map(|probe| {
                            let deadline = if probe.0 == original_fleets {
                                ends[0]
                            } else if probe.0 == source_hulls {
                                ends[1]
                            } else if kind >= 3 && probe.0 == target_hulls {
                                ends[2]
                            } else if source_path == Some(probe.0) {
                                ends[3]
                            } else if kind >= 3 && target_path == Some(probe.0) {
                                ends[4]
                            } else if probe.0 == collector {
                                ends[5]
                            } else {
                                usize::MAX
                            };
                            (probe, deadline)
                        })
                        .collect();
                    let mut discharged = vec![false; probes.len()];
                    let mut visits = 0;
                    for attempt in 0..10000 {
                        if visits == cut {
                            break;
                        }
                        // A freed address may be reused by a later storage allocation.
                        // Observe each original lifetime at its exact release boundary,
                        // then never use that discharged address as an identity again.
                        let next_deadline = probes
                            .iter()
                            .enumerate()
                            .filter(|(index, (_, deadline))| {
                                !discharged[*index] && *deadline != usize::MAX
                            })
                            .map(|(_, (_, deadline))| *deadline)
                            .min()
                            .unwrap_or(cut);
                        let before = allocations(creator);
                        for (index, &(probe, deadline)) in probes.iter().enumerate() {
                            if !discharged[index] {
                                assert!(visits < deadline);
                                assert!(
                                    retained(probe, &before),
                                    "before release kind {kind} cut {visits} deadline {deadline} probe {probe:?}"
                                );
                            }
                        }
                        let budget = schedule[attempt % schedule.len()]
                            .min(cut - visits)
                            .min(next_deadline - visits);
                        creator.poll(budget).unwrap();
                        assert!(creator.last_poll_visits > 0 && creator.last_poll_visits <= budget);
                        visits += creator.last_poll_visits;
                        let after = allocations(creator);
                        for (index, &(probe, deadline)) in probes.iter().enumerate() {
                            if discharged[index] {
                                continue;
                            }
                            assert!(visits <= deadline);
                            assert_eq!(
                                retained(probe, &after),
                                visits < deadline,
                                "release boundary kind {kind} cut {visits} deadline {deadline} probe {probe:?}"
                            );
                            if visits == deadline {
                                discharged[index] = true;
                            }
                        }
                    }
                    assert_eq!(visits, cut);
                    for budget in [0, 1025] {
                        assert_eq!(
                            creator.poll(budget),
                            Err(crate::living::step_job::LivingLifecycleJobErrorV2::Budget)
                        );
                        assert_eq!(creator.last_poll_visits, 0);
                    }
                    if cut == cuts.len() - 1 {
                        for budget in [1, 64, 1024] {
                            assert_eq!(
                                creator.poll(budget).unwrap(),
                                LivingStepProgressV2::NeedsSynchronousCompletion
                            );
                            assert_eq!(creator.last_poll_visits, 0);
                        }
                    } else {
                        assert!(creator.take_ready_result().is_err());
                    }
                    assert!(Arc::ptr_eq(&creator.queue, &queue));
                    assert!(Arc::ptr_eq(&creator.catalog, &catalog));
                    let current = allocations(creator);
                    for (index, &(probe, deadline)) in probes.iter().enumerate() {
                        assert_eq!(discharged[index], cut >= deadline);
                        if !discharged[index] {
                            assert!(
                                retained(probe, &current),
                                "live original kind {kind} cut {cut} deadline {deadline} probe {probe:?}"
                            );
                        }
                    }
                    step.cancel();
                    assert_eq!(step.poll(1), Err(LivingStepJobErrorV2::Cancelled));
                    assert_eq!(step.last_poll_visits(), 0);
                    assert!(matches!(
                        step.take_synchronous_preparation(&anchor),
                        Err(LivingStepJobErrorV2::Cancelled)
                    ));
                    assert!(std::ptr::eq(&*step, step_pointer));
                    assert!(std::ptr::eq(&**step.creator.as_ref().unwrap(), pointer));
                    assert_eq!(
                        drain_observed(
                            step.into_boxed_retirement(
                                LivingPublicationRetirementPolicyV2::ReleaseHandle
                            ),
                            schedule,
                            &current,
                            (kind, false, cut)
                        ),
                        expected + 52,
                        "kind {kind} cut {cut}"
                    );
                }
            }
        }
    }

    #[test]
    fn structural_creator_nested_capacity_failed_debit_and_original_owners() {
        use crate::living::simulation::fleet_structural_creator_tests as fixture;
        let cases = [
            (0, 8, 0, 331u64),
            (0, 46, 1, 346u64),
            (0, 75, 1, 346u64),
            (0, 111, 3, 380u64),
            (0, 191, 3, 375u64),
            (1, 8, 0, 331u64),
            (1, 46, 1, 346u64),
            (1, 75, 1, 346u64),
            (1, 111, 3, 384u64),
            (1, 191, 3, 379u64),
            (2, 8, 0, 331u64),
            (2, 46, 1, 346u64),
            (2, 75, 1, 346u64),
            (2, 111, 3, 450u64),
            (2, 191, 3, 445u64),
            (3, 15, 1, 352u64),
            (3, 55, 2, 384u64),
            (3, 137, 3, 376u64),
            (4, 15, 1, 352u64),
            (4, 55, 2, 450u64),
            (4, 137, 3, 442u64),
            (5, 15, 1, 352u64),
            (5, 55, 2, 384u64),
            (5, 137, 3, 376u64),
            (6, 15, 1, 352u64),
            (6, 55, 2, 454u64),
            (6, 137, 3, 446u64),
            (7, 15, 1, 352u64),
            (7, 55, 2, 450u64),
            (7, 137, 3, 442u64),
            (8, 15, 1, 352u64),
            (8, 55, 2, 454u64),
            (8, 137, 3, 446u64),
        ];
        for (kind, cut, which, expected) in cases {
            for schedule in fixture::SCHEDULES {
                let (creator, queue, catalog) = fixture::source(kind);
                let anchor = crate::living::builtin_living_sandbox_v2().unwrap();
                let mut step = captured_step(creator, anchor.capture_boundary().unwrap());
                let pointer = &**step.creator.as_ref().unwrap() as *const _;
                for _ in 0..cut {
                    assert_eq!(
                        step.creator.as_mut().unwrap().poll(1).unwrap(),
                        LivingStepProgressV2::Pending
                    );
                }
                let Some(QueuedCreatorInsertion::FleetStructural(work)) =
                    &mut step.creator.as_mut().unwrap().insertion
                else {
                    panic!("structural original");
                };
                let injected = match which {
                    0 => work
                        .selection_storage
                        .as_mut()
                        .unwrap()
                        .inject_structural_empty_capacity_failure(),
                    1 => work
                        .hull_storage
                        .as_mut()
                        .unwrap()
                        .inject_structural_empty_capacity_failure(),
                    2 => work
                        .filter
                        .as_mut()
                        .unwrap()
                        .inject_creator_capacity_failure(),
                    3 => work
                        .replacement
                        .as_mut()
                        .unwrap()
                        .inject_creator_capacity_failure(),
                    _ => unreachable!(),
                };
                assert!(injected);
                let probes = allocations(step.creator.as_ref().unwrap());
                assert_eq!(
                    step.poll(schedule[0]),
                    Err(LivingStepJobErrorV2::PreparationCapacity)
                );
                assert_eq!(step.last_poll_visits(), 1);
                assert_eq!(step.total_visits(), 1);
                for budget in [1, 64, 1024] {
                    assert_eq!(
                        step.poll(budget),
                        Err(LivingStepJobErrorV2::PreparationCapacity)
                    );
                    assert_eq!(step.last_poll_visits(), 0);
                }
                assert!(step.take_synchronous_preparation(&anchor).is_err());
                assert!(step.fault.is_none());
                let creator = step.creator.as_ref().unwrap();
                assert!(std::ptr::eq(&**creator, pointer));
                assert!(Arc::ptr_eq(&creator.queue, &queue));
                assert!(Arc::ptr_eq(&creator.catalog, &catalog));
                let now = allocations(creator);
                assert!(probes.iter().all(|probe| retained(*probe, &now)));
                assert_eq!(
                    drain_observed(
                        step.into_boxed_retirement(
                            LivingPublicationRetirementPolicyV2::ReleaseHandle
                        ),
                        schedule,
                        &now,
                        (kind, false, cut)
                    ),
                    expected
                );
            }
        }
    }
    #[test]
    fn structural_creator_guard_failed_debit_exact_source_retirement() {
        use crate::living::simulation::fleet_structural_creator_tests as fixture;
        for kind in 0..9 {
            for position in 0..4 {
                for schedule in fixture::SCHEDULES {
                    let (mut creator, queue, catalog) = fixture::source(kind);
                    let (source, target) = fixture::parents(kind);
                    let guarded = if kind < 3 { source } else { target };
                    let id = LivingEntityIdV2([30 + guarded as u8; 16]);
                    if position < 3 {
                        creator
                            .state
                            .hull_jobs
                            .record_at_mut(position)
                            .unwrap()
                            .target_fleet = Some(id);
                    } else {
                        creator
                            .state
                            .fleets
                            .record_at_mut(guarded)
                            .unwrap()
                            .location = LivingFleetLocationV2::Travelling {
                            origin_world: LivingEntityIdV2([2; 16]),
                            destination_world: LivingEntityIdV2([3; 16]),
                            departure_tick: LivingTickV2(0),
                            arrival_tick: LivingTickV2(99),
                        };
                    }
                    let before = creator.state.clone();
                    let anchor = crate::living::builtin_living_sandbox_v2().unwrap();
                    let mut step = captured_step(creator, anchor.capture_boundary().unwrap());
                    let expected_forward = 4 + if position < 3 { position + 1 } else { 0 };
                    for _ in 0..expected_forward - 1 {
                        assert_eq!(step.poll(1).unwrap(), LivingStepProgressV2::Pending);
                    }
                    let expected =
                        LivingStepJobErrorV2::Fault(LivingDeterministicFaultV2::Replay {
                            accepted_sequence: 1,
                            reason: Box::new(LivingCommandRejectionV2::ReferencedTarget),
                        });
                    assert_eq!(step.poll(schedule[0]), Err(expected.clone()));
                    assert_eq!(step.last_poll_visits(), 1);
                    assert_eq!(step.total_visits(), expected_forward as u64);
                    for budget in [1, 64, 1024] {
                        assert_eq!(step.poll(budget), Err(expected.clone()));
                        assert_eq!(step.last_poll_visits(), 0);
                    }
                    assert!(step.fault.is_none());
                    let creator = step.creator.as_ref().unwrap();
                    assert_eq!(creator.state, before);
                    assert!(Arc::ptr_eq(&creator.queue, &queue));
                    assert!(Arc::ptr_eq(&creator.catalog, &catalog));
                    let probes = allocations(creator);
                    // 183 initial +69 new variant +3 split Local +52 original Step.
                    assert_eq!(
                        drain_observed(
                            step.into_boxed_retirement(
                                LivingPublicationRetirementPolicyV2::ReleaseHandle
                            ),
                            schedule,
                            &probes,
                            (kind, false, position)
                        ),
                        if kind < 3 { 307 } else { 304 }
                    );
                }
            }
        }
    }

    #[test]
    fn structural_creator_duplicate_retains_exact_target_first_provenance_and_failed_sources() {
        use crate::living::simulation::fleet_structural_creator_tests as fixture;
        for schedule in fixture::SCHEDULES {
            let (mut creator, queue, catalog) = fixture::source(5);
            let target = creator.state.fleets.as_slice()[0].hulls.as_slice()[0].clone();
            let source = creator.state.fleets.record_at_mut(1).unwrap();
            let mut hulls = source.hulls.clone().into_vec();
            hulls[0].id = target.id;
            hulls[0].return_credit = !target.return_credit;
            source.hulls = LivingSortedVecV2::new(hulls).unwrap();
            let anchor = crate::living::builtin_living_sandbox_v2().unwrap();
            let mut step = captured_step(creator, anchor.capture_boundary().unwrap());
            // Source-only cut47: target40/source40 acquired, target40 emitted,
            // then target41 acquired. Next source40 fails strict keyed append.
            for _ in 0..47 {
                step.creator.as_mut().unwrap().poll(1).unwrap();
            }
            let pointer = &**step.creator.as_ref().unwrap() as *const _;
            let probes = allocations(step.creator.as_ref().unwrap());
            let fault = LivingStepJobErrorV2::Fault(LivingDeterministicFaultV2::Replay {
                accepted_sequence: 1,
                reason: Box::new(LivingCommandRejectionV2::Projection(
                    LivingWireErrorV2::KeyedOrder.into(),
                )),
            });
            assert_eq!(step.poll(schedule[0]), Err(fault.clone()));
            assert_eq!(step.last_poll_visits(), 1);
            assert_eq!(step.total_visits(), 1);
            assert!(step.fault.is_none());
            for b in [1, 64, 1024] {
                assert_eq!(step.poll(b), Err(fault.clone()));
                assert_eq!(step.last_poll_visits(), 0);
            }
            let creator = step.creator.as_ref().unwrap();
            assert!(std::ptr::eq(&**creator, pointer));
            assert!(Arc::ptr_eq(&creator.queue, &queue));
            assert!(Arc::ptr_eq(&creator.catalog, &catalog));
            let Some(QueuedCreatorInsertion::FleetStructural(work)) = &creator.insertion else {
                panic!("structural owner");
            };
            assert_eq!(work.kept.as_slice(), std::slice::from_ref(&target));
            assert_eq!(work.pending_source.as_ref().unwrap().id, target.id);
            assert_ne!(
                work.pending_source.as_ref().unwrap().return_credit,
                target.return_credit
            );
            assert!(work.pending_target.is_some());
            assert!(work.source_hulls.is_some());
            assert!(work.target_hulls.is_some());
            assert!(work.source_parent.is_some());
            assert!(work.target_parent.is_some());
            let current = allocations(creator);
            assert!(probes.iter().all(|p| retained(*p, &current)));
            // Initial183 plus the independently enumerated partial variant93;
            // original Step52; copied cause has no owning envelope in the Step.
            assert_eq!(
                drain_observed(
                    step.into_boxed_retirement(LivingPublicationRetirementPolicyV2::ReleaseHandle),
                    schedule,
                    &current,
                    (5, false, 47)
                ),
                328
            );
        }
    }

    #[test]
    fn structural_creator_genuine_nested_extraction_refusal_preserves_partition() {
        use crate::living::simulation::fleet_structural_creator_tests as fixture;
        let cases = [
            (0, 9, 0, 331u64, 2u64),
            (0, 47, 1, 346u64, 5u64),
            (0, 76, 1, 346u64, 5u64),
            (0, 156, 3, 294u64, 62u64),
            (0, 229, 3, 288u64, 68u64),
            (1, 9, 0, 331u64, 2u64),
            (1, 47, 1, 346u64, 5u64),
            (1, 76, 1, 346u64, 5u64),
            (1, 156, 3, 300u64, 60u64),
            (1, 229, 3, 294u64, 66u64),
            (2, 9, 0, 331u64, 2u64),
            (2, 47, 1, 346u64, 5u64),
            (2, 76, 1, 346u64, 5u64),
            (2, 156, 3, 399u64, 27u64),
            (2, 229, 3, 393u64, 33u64),
            (3, 16, 1, 352u64, 5u64),
            (3, 99, 2, 308u64, 52u64),
            (3, 180, 3, 296u64, 56u64),
            (4, 16, 1, 352u64, 5u64),
            (4, 99, 2, 407u64, 19u64),
            (4, 180, 3, 395u64, 23u64),
            (5, 16, 1, 352u64, 5u64),
            (5, 99, 2, 308u64, 52u64),
            (5, 180, 3, 296u64, 56u64),
            (6, 16, 1, 352u64, 5u64),
            (6, 99, 2, 413u64, 17u64),
            (6, 180, 3, 401u64, 21u64),
            (7, 16, 1, 352u64, 5u64),
            (7, 99, 2, 407u64, 19u64),
            (7, 180, 3, 395u64, 23u64),
            (8, 16, 1, 352u64, 5u64),
            (8, 99, 2, 413u64, 17u64),
            (8, 180, 3, 401u64, 21u64),
        ];
        for (kind, cut, which, expected, external_expected) in cases {
            for schedule in fixture::SCHEDULES {
                let (creator, queue, catalog) = fixture::source(kind);
                let anchor = crate::living::builtin_living_sandbox_v2().unwrap();
                let mut step = captured_step(creator, anchor.capture_boundary().unwrap());
                let pointer = &**step.creator.as_ref().unwrap() as *const _;
                for _ in 0..cut {
                    step.creator.as_mut().unwrap().poll(1).unwrap();
                }
                let before = allocations(step.creator.as_ref().unwrap());
                let Some(QueuedCreatorInsertion::FleetStructural(work)) =
                    &mut step.creator.as_mut().unwrap().insertion
                else {
                    panic!("structural original");
                };
                let policy = LivingStateRetirementPolicyV2::ReleaseHandle;
                let mut external: Box<dyn RetirementPoll> = match which {
                    0 => retirement_node(
                        work.selection_storage
                            .as_mut()
                            .unwrap()
                            .take_ready_result()
                            .unwrap(),
                        policy,
                    ),
                    1 => retirement_node(
                        work.hull_storage
                            .as_mut()
                            .unwrap()
                            .take_ready_result()
                            .unwrap(),
                        policy,
                    ),
                    2 => retirement_node(
                        work.filter.as_mut().unwrap().take_ready_result().unwrap(),
                        policy,
                    ),
                    3 => retirement_node(
                        work.replacement
                            .as_mut()
                            .unwrap()
                            .take_ready_result()
                            .unwrap(),
                        policy,
                    ),
                    _ => unreachable!(),
                };
                let mut owners = allocations(step.creator.as_ref().unwrap());
                external.creator_allocations(&mut owners);
                assert!(
                    before.iter().all(|probe| retained(*probe, &owners)),
                    "cut {cut} true extraction lost original owner"
                );
                assert_eq!(
                    step.poll(schedule[0]),
                    Err(LivingStepJobErrorV2::Fault(
                        LivingDeterministicFaultV2::Invariant
                    ))
                );
                assert_eq!(step.last_poll_visits(), 1);
                for budget in [1, 64, 1024] {
                    assert_eq!(
                        step.poll(budget),
                        Err(LivingStepJobErrorV2::Fault(
                            LivingDeterministicFaultV2::Invariant
                        ))
                    );
                    assert_eq!(step.last_poll_visits(), 0);
                }
                let creator = step.creator.as_ref().unwrap();
                assert!(std::ptr::eq(&**creator, pointer));
                assert!(Arc::ptr_eq(&creator.queue, &queue));
                assert!(Arc::ptr_eq(&creator.catalog, &catalog));
                assert!(step.fault.is_none());
                let current = allocations(creator);
                assert_eq!(
                    drain_observed(
                        step.into_boxed_retirement(
                            LivingPublicationRetirementPolicyV2::ReleaseHandle
                        ),
                        schedule,
                        &current,
                        (kind, false, cut)
                    ),
                    expected
                );
                let mut count = 0;
                let mut done = false;
                for _ in 0..10000 {
                    match external.retirement_step() {
                        RetirementStep::Work => count += 1,
                        RetirementStep::Done => {
                            done = true;
                            break;
                        }
                        RetirementStep::Deferred(_) => panic!("unshared output"),
                    }
                }
                assert!(done);
                drop(external);
                count += 1;
                assert_eq!(count, external_expected);
            }
        }
    }

    // Independently derived from immutable source equations, before any product run.
    const REMOVAL_CUTS: &[&[u64]] = &[
        &[
            272, 272, 272, 272, 272, 272, 272, 272, 272, 305, 305, 305, 305, 305, 305, 305, 305,
            305, 305, 305, 305, 305, 305, 305, 305, 308, 311, 308, 311, 308, 311, 308, 308, 311,
            311, 308, 311, 311, 308, 311, 311, 308, 308, 311, 311, 311, 308, 311, 311, 311, 308,
            311, 311, 311, 308, 308, 311, 311, 311, 308, 311, 311, 311, 308, 308, 311, 308, 311,
            308, 311, 308, 308, 311, 311, 308, 311, 311, 308, 311, 311, 308, 308, 311, 311, 308,
            311, 311, 308, 311, 311, 308, 308, 311, 311, 311, 308, 311, 311, 311, 308, 311, 311,
            311, 308, 308, 311, 311, 311, 311, 308, 311, 311, 311, 308, 311, 311, 311, 308, 308,
            311, 311, 311, 308, 311, 311, 308, 311, 311, 308, 311, 311, 311, 311, 308, 311, 311,
            308, 311, 311, 308, 311, 311, 311, 311, 311, 311, 314, 314, 314, 317, 317, 317, 317,
            320, 320, 320, 320, 320, 317, 320, 320, 320, 320, 320, 317, 320, 320, 320, 320, 320,
            317, 317, 320, 320, 320, 320, 320, 317, 320, 320, 320, 320, 320, 317, 320, 320, 320,
            320, 320, 317, 317, 320, 320, 317, 320, 320, 320, 323, 323, 323, 323, 323, 326, 323,
            326, 323, 326, 323, 323, 326, 326, 323, 326, 326, 323, 326, 326, 323, 323, 326, 326,
            326, 323, 326, 326, 326, 323, 326, 326, 326, 323, 323, 326, 326, 326, 323, 326, 326,
            326, 323, 323, 326, 323, 326, 323, 326, 323, 323, 326, 326, 323, 326, 326, 323, 326,
            326, 323, 323, 326, 326, 323, 326, 326, 323, 326, 326, 323, 323, 326, 326, 326, 323,
            326, 326, 326, 323, 326, 326, 326, 323, 323, 326, 326, 326, 326, 326, 329, 329, 329,
            326, 329, 329, 329, 326, 326, 329, 329, 329, 326, 329, 329, 326, 329, 329, 326, 329,
            329, 329, 329, 326, 329, 329, 326, 329, 329, 326, 329, 329, 329, 329, 329, 326, 329,
            329, 326, 329, 329, 326, 326, 329, 329, 329, 329, 329, 326, 329, 329, 329, 329, 329,
            326, 329, 329, 329, 329, 329, 326, 326, 329, 329, 329, 329, 329, 326, 329, 329, 329,
            329, 329, 326, 329, 329, 329, 329, 329, 326, 326, 329, 329, 326, 329, 329, 326, 329,
            329, 326, 326, 326, 329, 326, 329, 326, 329, 326, 326, 329, 329, 326, 329, 329, 326,
            329, 329, 326, 326, 329, 329, 329, 326, 329, 329, 329, 326, 329, 329, 329, 326, 326,
            329, 329, 329, 326, 329, 329, 329, 326, 326, 329, 326, 329, 326, 329, 326, 326, 329,
            329, 326, 329, 329, 326, 329, 329, 326, 326, 329, 329, 326, 329, 329, 326, 329, 329,
            326, 326, 329, 329, 329, 326, 329, 329, 329, 326, 329, 329, 329, 326, 326, 329, 329,
            329, 329, 326, 329, 329, 329, 326, 329, 329, 329, 326, 326, 329, 329, 329, 326, 329,
            329, 326, 329, 329, 326, 329, 329, 329, 329, 326, 329, 329, 326, 329, 329, 326, 329,
            329, 329, 329, 329, 326, 329, 329, 326, 329, 329, 326, 326, 329, 329, 329, 329, 329,
            326, 329, 329, 329, 329, 329, 326, 329, 329, 329, 329, 329, 326, 326, 329, 329, 329,
            329, 329, 326, 329, 329, 329, 329, 329, 326, 329, 329, 329, 329, 329, 326, 326, 329,
            329, 326, 329, 329, 326, 329, 329, 326, 326, 326, 326, 326, 326, 326, 326, 326, 326,
            326, 326, 326, 326, 326, 326, 326, 326, 326, 326, 326, 326, 326, 326, 326, 326, 326,
            326, 326, 326, 326, 326, 350, 353, 350, 349, 348, 347, 346, 345, 344, 343, 342, 341,
            340, 339, 338, 337, 336, 335, 334, 333, 332, 331, 330, 329, 328, 327, 326, 325, 326,
            329, 329, 332, 332, 335, 335, 338, 338, 341, 341, 344, 344, 344, 344, 347, 347, 350,
            350, 353, 353, 356, 356, 359, 359, 362, 362, 362, 362, 362, 362, 362, 362, 362, 362,
            362, 362, 362, 362, 398, 422, 428, 422, 421, 420, 419, 418, 417, 416, 415, 414, 413,
            412, 411, 410, 409, 408, 407, 406, 405, 404, 403, 402, 401, 400, 399, 398, 397, 398,
            398, 398, 398, 398, 398, 398, 398, 397, 398, 392, 391, 390, 389, 388, 387, 386, 385,
            384, 383, 382, 381, 380, 379, 378, 377, 376, 375, 374, 373, 372, 371, 370, 369, 368,
            367, 366, 365, 364, 363, 362, 361, 362, 362, 362, 362, 362, 398, 422, 428, 422, 421,
            420, 419, 418, 417, 416, 415, 414, 413, 412, 411, 410, 409, 408, 407, 406, 405, 404,
            403, 402, 401, 400, 399, 398, 397, 398, 398, 398, 398, 398, 398, 398, 398, 397, 398,
            392, 391, 390, 389, 388, 387, 386, 385, 384, 383, 382, 381, 380, 379, 378, 377, 376,
            375, 374, 373, 372, 371, 370, 369, 368, 367, 366, 365, 364, 363, 362, 361, 362, 362,
            362, 362, 362, 398, 422, 428, 422, 421, 420, 419, 418, 417, 416, 415, 414, 413, 412,
            411, 410, 409, 408, 407, 406, 405, 404, 403, 402, 401, 400, 399, 398, 397, 398, 398,
            398, 398, 398, 398, 398, 398, 397, 398, 392, 391, 390, 389, 388, 387, 386, 385, 384,
            383, 382, 381, 380, 379, 378, 377, 376, 375, 374, 373, 372, 371, 370, 369, 368, 367,
            366, 365, 364, 363, 362, 361, 362, 362, 362, 365, 365, 399, 423, 429, 423, 422, 421,
            420, 419, 418, 417, 416, 415, 414, 413, 412, 411, 410, 409, 408, 407, 406, 405, 404,
            403, 402, 401, 400, 399, 398, 399, 399, 399, 399, 399, 398, 397, 396, 395, 396, 396,
            395, 396, 394, 393, 392, 391, 390, 389, 388, 387, 386, 385, 384, 383, 382, 381, 380,
            379, 378, 377, 376, 375, 374, 373, 372, 371, 370, 369, 368, 367, 366, 365, 364, 363,
            362, 361, 360, 361, 361, 361, 361, 361, 397, 421, 427, 421, 420, 419, 418, 417, 416,
            415, 414, 413, 412, 411, 410, 409, 408, 407, 406, 405, 404, 403, 402, 401, 400, 399,
            398, 397, 396, 397, 397, 397, 397, 397, 397, 397, 397, 396, 397, 391, 390, 389, 388,
            387, 386, 385, 384, 383, 382, 381, 380, 379, 378, 377, 376, 375, 374, 373, 372, 371,
            370, 369, 368, 367, 366, 365, 364, 363, 362, 361, 360, 361, 361, 361, 361, 361, 397,
            421, 427, 421, 420, 419, 418, 417, 416, 415, 414, 413, 412, 411, 410, 409, 408, 407,
            406, 405, 404, 403, 402, 401, 400, 399, 398, 397, 396, 397, 397, 397, 397, 397, 397,
            397, 397, 396, 397, 391, 390, 389, 388, 387, 386, 385, 384, 383, 382, 381, 380, 379,
            378, 377, 376, 375, 374, 373, 372, 371, 370, 369, 368, 367, 366, 365, 364, 363, 362,
            361, 360, 361, 361, 361, 361, 361, 394, 418, 424, 418, 417, 416, 415, 414, 413, 412,
            411, 410, 409, 408, 407, 406, 405, 404, 403, 402, 401, 400, 399, 398, 397, 396, 395,
            394, 393, 394, 394, 394, 394, 394, 394, 394, 394, 393, 394, 391, 390, 389, 388, 387,
            386, 385, 384, 383, 382, 381, 380, 379, 378, 377, 376, 375, 374, 373, 372, 371, 370,
            369, 368, 367, 366, 365, 364, 363, 362, 361, 360, 361, 361, 361, 361, 361, 397, 421,
            427, 421, 420, 419, 418, 417, 416, 415, 414, 413, 412, 411, 410, 409, 408, 407, 406,
            405, 404, 403, 402, 401, 400, 399, 398, 397, 396, 397, 397, 397, 397, 397, 397, 397,
            397, 396, 397, 391, 390, 389, 388, 387, 386, 385, 384, 383, 382, 381, 380, 379, 378,
            377, 376, 375, 374, 373, 372, 371, 370, 369, 368, 367, 366, 365, 364, 363, 362, 361,
            360, 361, 361, 364, 364, 364, 364, 400, 424, 430, 424, 423, 422, 421, 420, 419, 418,
            417, 416, 415, 414, 413, 412, 411, 410, 409, 408, 407, 406, 405, 404, 403, 402, 401,
            400, 399, 400, 400, 400, 399, 398, 397, 396, 397, 397, 397, 397, 397, 397, 396, 397,
            393, 392, 391, 390, 389, 388, 387, 386, 385, 384, 383, 382, 381, 380, 379, 378, 377,
            376, 375, 374, 373, 372, 371, 370, 369, 368, 367, 366, 365, 364, 363, 362, 361, 360,
            359, 360, 360, 360, 360, 360, 394, 418, 424, 418, 417, 416, 415, 414, 413, 412, 411,
            410, 409, 408, 407, 406, 405, 404, 403, 402, 401, 400, 399, 398, 397, 396, 395, 394,
            393, 394, 394, 394, 394, 394, 394, 393, 394, 390, 389, 388, 387, 386, 385, 384, 383,
            382, 381, 380, 379, 378, 377, 376, 375, 374, 373, 372, 371, 370, 369, 368, 367, 366,
            365, 364, 363, 362, 361, 360, 359, 360, 360, 360, 360, 360, 394, 418, 424, 418, 417,
            416, 415, 414, 413, 412, 411, 410, 409, 408, 407, 406, 405, 404, 403, 402, 401, 400,
            399, 398, 397, 396, 395, 394, 393, 394, 394, 394, 394, 394, 394, 393, 394, 390, 389,
            388, 387, 386, 385, 384, 383, 382, 381, 380, 379, 378, 377, 376, 375, 374, 373, 372,
            371, 370, 369, 368, 367, 366, 365, 364, 363, 362, 361, 360, 359, 360, 360, 360, 363,
            363, 366, 366, 400, 424, 430, 424, 423, 422, 421, 420, 419, 418, 417, 416, 415, 414,
            413, 412, 411, 410, 409, 408, 407, 406, 405, 404, 403, 402, 401, 400, 399, 400, 400,
            400, 399, 398, 397, 396, 397, 397, 397, 396, 395, 394, 393, 394, 394, 393, 394, 394,
            393, 392, 391, 390, 389, 388, 387, 386, 385, 384, 383, 382, 381, 380, 379, 378, 377,
            376, 375, 374, 373, 372, 371, 370, 369, 368, 367, 366, 365, 364, 363, 362, 361, 360,
            359, 358, 357, 358, 358, 358, 358, 358, 361, 361, 394, 418, 424, 418, 417, 416, 415,
            414, 413, 412, 411, 410, 409, 408, 407, 406, 405, 404, 403, 402, 401, 400, 399, 398,
            397, 396, 395, 394, 393, 394, 394, 394, 394, 394, 394, 394, 393, 392, 391, 390, 389,
            388, 387, 386, 385, 384, 383, 382, 381, 380, 379, 378, 377, 376, 375, 374, 373, 372,
            371, 370, 369, 368, 367, 366, 365, 364, 363, 362, 361, 360, 359, 358, 357, 356, 355,
            354, 353, 354, 354, 353, 354, 352, 351, 350, 349, 348, 347, 346, 345, 344, 343, 342,
            341, 340, 339, 338, 337, 336, 335, 334, 333, 332, 331, 330, 329, 328, 327, 326, 325,
            324, 323, 322, 321, 320, 319, 318, 319, 319, 319, 319, 319, 355, 379, 385, 379, 378,
            377, 376, 375, 374, 373, 372, 371, 370, 369, 368, 367, 366, 365, 364, 363, 362, 361,
            360, 359, 358, 357, 356, 355, 354, 355, 355, 355, 355, 355, 355, 355, 355, 354, 355,
            349, 348, 347, 346, 345, 344, 343, 342, 341, 340, 339, 338, 337, 336, 335, 334, 333,
            332, 331, 330, 329, 328, 327, 326, 325, 324, 323, 322, 321, 320, 319, 318, 319, 319,
            319, 319, 319, 355, 379, 385, 379, 378, 377, 376, 375, 374, 373, 372, 371, 370, 369,
            368, 367, 366, 365, 364, 363, 362, 361, 360, 359, 358, 357, 356, 355, 354, 355, 355,
            355, 355, 355, 355, 355, 355, 354, 355, 349, 348, 347, 346, 345, 344, 343, 342, 341,
            340, 339, 338, 337, 336, 335, 334, 333, 332, 331, 330, 329, 328, 327, 326, 325, 324,
            323, 322, 321, 320, 319, 318, 319, 319, 319, 322, 322, 325, 325, 361, 385, 391, 385,
            384, 383, 382, 381, 380, 379, 378, 377, 376, 375, 374, 373, 372, 371, 370, 369, 368,
            367, 366, 365, 364, 363, 362, 361, 360, 361, 361, 361, 361, 361, 360, 359, 358, 357,
            358, 358, 358, 357, 356, 355, 354, 355, 355, 354, 355, 353, 352, 351, 350, 349, 348,
            347, 346, 345, 344, 343, 342, 341, 340, 339, 338, 337, 336, 335, 334, 333, 332, 331,
            330, 329, 328, 327, 326, 325, 324, 323, 322, 321, 320, 319, 318, 317, 316, 317, 317,
            317, 317, 317, 350, 374, 380, 374, 373, 372, 371, 370, 369, 368, 367, 366, 365, 364,
            363, 362, 361, 360, 359, 358, 357, 356, 355, 354, 353, 352, 351, 350, 349, 350, 350,
            350, 350, 350, 350, 350, 350, 349, 350, 347, 346, 345, 344, 343, 342, 341, 340, 339,
            338, 337, 336, 335, 334, 333, 332, 331, 330, 329, 328, 327, 326, 325, 324, 323, 322,
            321, 320, 319, 318, 317, 316, 317, 317, 317, 317, 317, 350, 374, 380, 374, 373, 372,
            371, 370, 369, 368, 367, 366, 365, 364, 363, 362, 361, 360, 359, 358, 357, 356, 355,
            354, 353, 352, 351, 350, 349, 350, 350, 350, 350, 350, 350, 350, 350, 349, 350, 347,
            346, 345, 344, 343, 342, 341, 340, 339, 338, 337, 336, 335, 334, 333, 332, 331, 330,
            329, 328, 327, 326, 325, 324, 323, 322, 321, 320, 319, 318, 317, 316, 317, 317, 320,
            320, 323, 323, 326, 326, 362, 386, 392, 386, 385, 384, 383, 382, 381, 380, 379, 378,
            377, 376, 375, 374, 373, 372, 371, 370, 369, 368, 367, 366, 365, 364, 363, 362, 361,
            362, 362, 362, 361, 360, 359, 358, 359, 359, 359, 358, 357, 356, 355, 356, 356, 356,
            355, 354, 353, 352, 353, 353, 352, 353, 353, 352, 351, 350, 349, 348, 347, 346, 345,
            344, 343, 342, 341, 340, 339, 338, 337, 336, 335, 334, 333, 332, 331, 330, 329, 328,
            327, 326, 325, 324, 323, 322, 321, 320, 319, 318, 317, 316, 315, 314, 313, 314, 314,
            317, 317, 320, 320, 323, 323, 359, 383, 389, 383, 382, 381, 380, 379, 378, 377, 376,
            375, 374, 373, 372, 371, 370, 369, 368, 367, 366, 365, 364, 363, 362, 361, 360, 359,
            358, 359, 359, 359, 358, 357, 356, 355, 356, 356, 356, 355, 354, 353, 352, 353, 353,
            353, 352, 351, 350, 349, 350, 350, 349, 350, 350, 349, 348, 347, 346, 345, 344, 343,
            342, 341, 340, 339, 338, 337, 336, 335, 334, 333, 332, 331, 330, 329, 328, 327, 326,
            325, 324, 323, 322, 321, 320, 319, 318, 317, 316, 315, 314, 313, 312, 311, 310, 311,
            311, 314, 314, 317, 317, 320, 320, 356, 380, 386, 380, 379, 378, 377, 376, 375, 374,
            373, 372, 371, 370, 369, 368, 367, 366, 365, 364, 363, 362, 361, 360, 359, 358, 357,
            356, 355, 356, 356, 356, 355, 354, 353, 352, 353, 353, 353, 352, 351, 350, 349, 350,
            350, 350, 349, 348, 347, 346, 347, 347, 346, 347, 347, 346, 345, 344, 343, 342, 341,
            340, 339, 338, 337, 336, 335, 334, 333, 332, 331, 330, 329, 328, 327, 326, 325, 324,
            323, 322, 321, 320, 319, 318, 317, 316, 315, 314, 313, 312, 311, 310, 309, 308, 307,
            308, 308, 308, 308, 308, 344, 368, 374, 368, 367, 366, 365, 364, 363, 362, 361, 360,
            359, 358, 357, 356, 355, 354, 353, 352, 351, 350, 349, 348, 347, 346, 345, 344, 343,
            344, 344, 344, 344, 344, 344, 344, 344, 343, 344, 338, 337, 336, 335, 334, 333, 332,
            331, 330, 329, 328, 327, 326, 325, 324, 323, 322, 321, 320, 319, 318, 317, 316, 315,
            314, 313, 312, 311, 310, 309, 308, 307, 308, 308, 308, 308, 308, 344, 368, 374, 368,
            367, 366, 365, 364, 363, 362, 361, 360, 359, 358, 357, 356, 355, 354, 353, 352, 351,
            350, 349, 348, 347, 346, 345, 344, 343, 344, 344, 344, 344, 344, 344, 344, 344, 343,
            344, 338, 337, 336, 335, 334, 333, 332, 331, 330, 329, 328, 327, 326, 325, 324, 323,
            322, 321, 320, 319, 318, 317, 316, 315, 314, 313, 312, 311, 310, 309, 308, 307, 308,
            308, 308, 307, 306, 305, 304, 303, 302, 301, 300, 299, 298, 297, 296, 295, 294, 293,
            292, 291, 290, 289, 288, 287, 286, 285, 284, 283, 282, 281, 280, 279, 278, 277, 276,
            275, 274, 273, 272, 271, 270, 269, 268, 267, 266, 265, 264, 263, 262, 261, 260, 259,
            258, 257, 256, 255, 254, 253, 252, 251, 250, 249, 248, 247, 246, 245, 244, 243, 242,
            241, 240, 239, 238, 237, 236, 235, 234, 233, 232, 231, 230, 229, 228, 227, 226, 225,
            224, 223, 222, 221, 220, 219, 218, 217, 218, 221, 224, 227, 227, 227, 227, 227, 226,
            225, 224, 223, 222, 221, 220, 219, 218, 217, 218, 245, 245, 242, 241, 240, 239, 238,
            237, 236, 235, 234, 233, 232, 231, 230, 229, 228, 227, 226, 225, 224, 223, 222, 221,
            220, 219, 218, 217, 218, 221, 224, 227, 227, 226, 225, 224, 223, 222, 221, 220, 219,
            218, 217, 218, 266, 266, 269, 296, 299, 299, 296, 295, 293, 293, 293, 275, 274, 273,
            272, 271, 270, 269, 268, 267, 266, 265, 264, 263, 262, 261, 260, 259, 258, 257, 256,
            255, 254, 253, 252, 251, 250, 249, 248, 247, 246, 245, 244, 243, 242, 241, 240, 239,
            238, 237, 236, 235, 234, 233, 232, 231, 230, 229, 228, 227, 226, 225, 224, 223, 222,
            221, 220, 221, 221,
        ],
        &[
            272, 272, 272, 272, 272, 272, 272, 272, 272, 305, 305, 305, 308, 311, 308, 311, 308,
            311, 308, 308, 311, 311, 311, 314, 314, 311, 314, 314, 311, 311, 314, 314, 314, 317,
            317, 317, 314, 317, 317, 317, 314, 314, 317, 317, 317, 320, 320, 320, 320, 323, 320,
            323, 320, 323, 320, 320, 323, 323, 323, 326, 326, 323, 326, 326, 323, 323, 326, 326,
            326, 329, 329, 326, 329, 329, 326, 326, 329, 329, 329, 332, 332, 332, 329, 332, 332,
            332, 329, 329, 332, 332, 332, 332, 329, 332, 332, 332, 329, 332, 332, 332, 329, 329,
            332, 332, 332, 332, 335, 335, 335, 338, 338, 338, 341, 341, 341, 341, 341, 344, 344,
            344, 347, 347, 347, 350, 350, 350, 350, 350, 353, 353, 353, 356, 356, 356, 356, 359,
            359, 359, 362, 362, 362, 362, 362, 359, 362, 362, 362, 362, 362, 359, 359, 362, 362,
            362, 362, 362, 359, 362, 362, 362, 362, 362, 359, 362, 362, 362, 362, 362, 359, 359,
            362, 362, 362, 365, 365, 365, 368, 368, 368, 368, 368, 371, 368, 371, 368, 371, 368,
            368, 371, 371, 368, 371, 371, 368, 371, 371, 368, 368, 371, 371, 368, 371, 371, 371,
            368, 371, 371, 371, 368, 368, 371, 371, 368, 371, 371, 368, 368, 371, 368, 371, 368,
            371, 368, 368, 371, 371, 368, 371, 371, 368, 371, 371, 368, 368, 371, 371, 368, 371,
            371, 368, 371, 371, 368, 368, 371, 371, 368, 371, 371, 371, 368, 371, 371, 371, 368,
            368, 371, 371, 371, 371, 371, 374, 374, 374, 371, 374, 374, 374, 371, 371, 374, 374,
            374, 371, 374, 374, 371, 374, 374, 371, 374, 374, 374, 374, 371, 374, 374, 371, 374,
            374, 371, 374, 374, 374, 374, 371, 374, 374, 371, 374, 374, 371, 371, 374, 374, 371,
            374, 374, 374, 374, 374, 371, 374, 374, 374, 374, 374, 371, 371, 374, 374, 374, 374,
            374, 371, 374, 374, 374, 374, 374, 371, 374, 374, 374, 374, 374, 371, 371, 374, 374,
            371, 374, 374, 371, 374, 374, 371, 371, 371, 374, 371, 374, 371, 374, 371, 371, 374,
            374, 371, 374, 374, 371, 374, 374, 371, 371, 374, 374, 371, 374, 374, 374, 371, 374,
            374, 374, 371, 371, 374, 374, 371, 374, 374, 371, 371, 374, 371, 374, 371, 374, 371,
            371, 374, 374, 371, 374, 374, 371, 374, 374, 371, 371, 374, 374, 371, 374, 374, 371,
            374, 374, 371, 371, 374, 374, 371, 374, 374, 374, 371, 374, 374, 374, 371, 371, 374,
            374, 374, 374, 371, 374, 374, 374, 371, 374, 374, 374, 371, 371, 374, 374, 374, 371,
            374, 374, 371, 374, 374, 371, 374, 374, 374, 374, 371, 374, 374, 371, 374, 374, 371,
            374, 374, 374, 374, 371, 374, 374, 371, 374, 374, 371, 371, 374, 374, 371, 374, 374,
            374, 374, 374, 371, 374, 374, 374, 374, 374, 371, 371, 374, 374, 374, 374, 374, 371,
            374, 374, 374, 374, 374, 371, 374, 374, 374, 374, 374, 371, 371, 374, 374, 371, 374,
            374, 371, 374, 374, 371, 371, 371, 371, 371, 371, 371, 371, 371, 371, 371, 371, 371,
            371, 371, 371, 371, 371, 371, 371, 371, 371, 371, 371, 371, 371, 371, 371, 371, 371,
            371, 371, 395, 398, 395, 394, 393, 392, 391, 390, 389, 388, 387, 386, 385, 384, 383,
            382, 381, 380, 379, 378, 377, 376, 375, 374, 373, 372, 371, 370, 371, 374, 374, 377,
            377, 380, 380, 383, 383, 386, 386, 389, 389, 392, 392, 395, 395, 398, 398, 401, 401,
            404, 404, 407, 407, 410, 410, 413, 413, 416, 416, 419, 419, 422, 422, 425, 425, 428,
            428, 431, 431, 434, 434, 434, 434, 437, 437, 440, 440, 443, 443, 446, 446, 449, 449,
            452, 452, 455, 455, 458, 458, 461, 461, 464, 464, 467, 467, 470, 470, 473, 473, 476,
            476, 479, 479, 482, 482, 485, 485, 488, 488, 491, 491, 494, 494, 497, 497, 497, 497,
            497, 497, 497, 497, 497, 497, 497, 497, 497, 497, 497, 497, 497, 497, 497, 497, 497,
            497, 497, 497, 497, 497, 500, 500, 500, 500, 536, 560, 566, 560, 559, 558, 557, 556,
            555, 554, 553, 552, 551, 550, 549, 548, 547, 546, 545, 544, 543, 542, 541, 540, 539,
            538, 537, 536, 535, 536, 536, 536, 535, 534, 533, 532, 533, 533, 533, 533, 533, 533,
            532, 533, 529, 528, 527, 526, 525, 524, 523, 522, 521, 520, 519, 518, 517, 516, 515,
            514, 513, 512, 511, 510, 509, 508, 507, 506, 505, 504, 503, 502, 501, 500, 499, 498,
            497, 496, 495, 496, 496, 499, 499, 499, 499, 535, 559, 565, 559, 558, 557, 556, 555,
            554, 553, 552, 551, 550, 549, 548, 547, 546, 545, 544, 543, 542, 541, 540, 539, 538,
            537, 536, 535, 534, 535, 535, 535, 534, 533, 532, 531, 532, 532, 532, 532, 532, 532,
            531, 532, 528, 527, 526, 525, 524, 523, 522, 521, 520, 519, 518, 517, 516, 515, 514,
            513, 512, 511, 510, 509, 508, 507, 506, 505, 504, 503, 502, 501, 500, 499, 498, 497,
            496, 495, 494, 495, 495, 498, 498, 498, 498, 534, 558, 564, 558, 557, 556, 555, 554,
            553, 552, 551, 550, 549, 548, 547, 546, 545, 544, 543, 542, 541, 540, 539, 538, 537,
            536, 535, 534, 533, 534, 534, 534, 533, 532, 531, 530, 531, 531, 531, 531, 531, 531,
            530, 531, 527, 526, 525, 524, 523, 522, 521, 520, 519, 518, 517, 516, 515, 514, 513,
            512, 511, 510, 509, 508, 507, 506, 505, 504, 503, 502, 501, 500, 499, 498, 497, 496,
            495, 494, 493, 494, 494, 497, 497, 500, 500, 534, 558, 564, 558, 557, 556, 555, 554,
            553, 552, 551, 550, 549, 548, 547, 546, 545, 544, 543, 542, 541, 540, 539, 538, 537,
            536, 535, 534, 533, 534, 534, 534, 533, 532, 531, 530, 531, 531, 531, 530, 529, 528,
            527, 528, 528, 527, 528, 528, 527, 526, 525, 524, 523, 522, 521, 520, 519, 518, 517,
            516, 515, 514, 513, 512, 511, 510, 509, 508, 507, 506, 505, 504, 503, 502, 501, 500,
            499, 498, 497, 496, 495, 494, 493, 492, 491, 492, 492, 492, 492, 492, 528, 552, 558,
            552, 551, 550, 549, 548, 547, 546, 545, 544, 543, 542, 541, 540, 539, 538, 537, 536,
            535, 534, 533, 532, 531, 530, 529, 528, 527, 528, 528, 528, 528, 528, 528, 528, 528,
            527, 528, 522, 521, 520, 519, 518, 517, 516, 515, 514, 513, 512, 511, 510, 509, 508,
            507, 506, 505, 504, 503, 502, 501, 500, 499, 498, 497, 496, 495, 494, 493, 492, 491,
            492, 492, 495, 495, 495, 495, 531, 555, 561, 555, 554, 553, 552, 551, 550, 549, 548,
            547, 546, 545, 544, 543, 542, 541, 540, 539, 538, 537, 536, 535, 534, 533, 532, 531,
            530, 531, 531, 531, 530, 529, 528, 527, 528, 528, 528, 528, 528, 528, 527, 528, 524,
            523, 522, 521, 520, 519, 518, 517, 516, 515, 514, 513, 512, 511, 510, 509, 508, 507,
            506, 505, 504, 503, 502, 501, 500, 499, 498, 497, 496, 495, 494, 493, 492, 491, 490,
            491, 491, 494, 494, 494, 494, 527, 551, 557, 551, 550, 549, 548, 547, 546, 545, 544,
            543, 542, 541, 540, 539, 538, 537, 536, 535, 534, 533, 532, 531, 530, 529, 528, 527,
            526, 527, 527, 527, 526, 525, 524, 523, 522, 521, 520, 521, 521, 521, 521, 521, 521,
            520, 521, 519, 518, 517, 516, 515, 514, 513, 512, 511, 510, 509, 508, 507, 506, 505,
            504, 503, 502, 501, 500, 499, 498, 497, 496, 495, 494, 493, 492, 491, 490, 489, 488,
            487, 486, 485, 486, 486, 489, 489, 489, 489, 525, 549, 555, 549, 548, 547, 546, 545,
            544, 543, 542, 541, 540, 539, 538, 537, 536, 535, 534, 533, 532, 531, 530, 529, 528,
            527, 526, 525, 524, 525, 525, 525, 524, 523, 522, 521, 522, 522, 522, 522, 522, 522,
            521, 522, 518, 517, 516, 515, 514, 513, 512, 511, 510, 509, 508, 507, 506, 505, 504,
            503, 502, 501, 500, 499, 498, 497, 496, 495, 494, 493, 492, 491, 490, 489, 488, 487,
            486, 485, 484, 485, 485, 488, 488, 488, 488, 524, 548, 554, 548, 547, 546, 545, 544,
            543, 542, 541, 540, 539, 538, 537, 536, 535, 534, 533, 532, 531, 530, 529, 528, 527,
            526, 525, 524, 523, 524, 524, 524, 523, 522, 521, 520, 521, 521, 521, 521, 521, 521,
            520, 521, 517, 516, 515, 514, 513, 512, 511, 510, 509, 508, 507, 506, 505, 504, 503,
            502, 501, 500, 499, 498, 497, 496, 495, 494, 493, 492, 491, 490, 489, 488, 487, 486,
            485, 484, 483, 484, 484, 484, 487, 487, 490, 490, 524, 548, 554, 548, 547, 546, 545,
            544, 543, 542, 541, 540, 539, 538, 537, 536, 535, 534, 533, 532, 531, 530, 529, 528,
            527, 526, 525, 524, 523, 524, 524, 524, 523, 522, 521, 520, 521, 521, 521, 520, 519,
            518, 517, 518, 518, 517, 518, 518, 517, 516, 515, 514, 513, 512, 511, 510, 509, 508,
            507, 506, 505, 504, 503, 502, 501, 500, 499, 498, 497, 496, 495, 494, 493, 492, 491,
            490, 489, 488, 487, 486, 485, 484, 483, 482, 481, 482, 482, 482, 485, 485, 488, 488,
            522, 546, 552, 546, 545, 544, 543, 542, 541, 540, 539, 538, 537, 536, 535, 534, 533,
            532, 531, 530, 529, 528, 527, 526, 525, 524, 523, 522, 521, 522, 522, 522, 521, 520,
            519, 518, 519, 519, 519, 518, 517, 516, 515, 516, 516, 515, 516, 516, 515, 514, 513,
            512, 511, 510, 509, 508, 507, 506, 505, 504, 503, 502, 501, 500, 499, 498, 497, 496,
            495, 494, 493, 492, 491, 490, 489, 488, 487, 486, 485, 484, 483, 482, 481, 480, 479,
            480, 480, 480, 483, 483, 486, 486, 520, 544, 550, 544, 543, 542, 541, 540, 539, 538,
            537, 536, 535, 534, 533, 532, 531, 530, 529, 528, 527, 526, 525, 524, 523, 522, 521,
            520, 519, 520, 520, 520, 519, 518, 517, 516, 517, 517, 517, 516, 515, 514, 513, 514,
            514, 513, 514, 514, 513, 512, 511, 510, 509, 508, 507, 506, 505, 504, 503, 502, 501,
            500, 499, 498, 497, 496, 495, 494, 493, 492, 491, 490, 489, 488, 487, 486, 485, 484,
            483, 482, 481, 480, 479, 478, 477, 478, 478, 478, 481, 481, 484, 484, 487, 487, 520,
            544, 550, 544, 543, 542, 541, 540, 539, 538, 537, 536, 535, 534, 533, 532, 531, 530,
            529, 528, 527, 526, 525, 524, 523, 522, 521, 520, 519, 520, 520, 520, 519, 518, 517,
            516, 515, 514, 515, 515, 515, 514, 513, 512, 511, 510, 509, 508, 507, 508, 508, 508,
            507, 506, 505, 504, 503, 502, 501, 500, 499, 498, 497, 496, 495, 494, 493, 492, 491,
            490, 489, 488, 487, 486, 485, 484, 483, 482, 481, 480, 479, 478, 477, 476, 475, 474,
            473, 472, 471, 470, 469, 468, 467, 468, 468, 467, 468, 468, 467, 466, 465, 464, 463,
            462, 461, 460, 459, 458, 457, 456, 455, 454, 453, 452, 451, 450, 449, 448, 447, 446,
            445, 444, 443, 442, 441, 440, 439, 438, 437, 436, 435, 434, 433, 432, 431, 430, 429,
            428, 429, 429, 432, 432, 432, 432, 468, 492, 498, 492, 491, 490, 489, 488, 487, 486,
            485, 484, 483, 482, 481, 480, 479, 478, 477, 476, 475, 474, 473, 472, 471, 470, 469,
            468, 467, 468, 468, 468, 467, 466, 465, 464, 465, 465, 465, 465, 465, 465, 464, 465,
            461, 460, 459, 458, 457, 456, 455, 454, 453, 452, 451, 450, 449, 448, 447, 446, 445,
            444, 443, 442, 441, 440, 439, 438, 437, 436, 435, 434, 433, 432, 431, 430, 429, 428,
            427, 428, 428, 428, 428, 428, 464, 488, 494, 488, 487, 486, 485, 484, 483, 482, 481,
            480, 479, 478, 477, 476, 475, 474, 473, 472, 471, 470, 469, 468, 467, 466, 465, 464,
            463, 464, 464, 464, 464, 464, 464, 464, 464, 463, 464, 458, 457, 456, 455, 454, 453,
            452, 451, 450, 449, 448, 447, 446, 445, 444, 443, 442, 441, 440, 439, 438, 437, 436,
            435, 434, 433, 432, 431, 430, 429, 428, 427, 428, 428, 431, 431, 434, 434, 437, 437,
            473, 497, 503, 497, 496, 495, 494, 493, 492, 491, 490, 489, 488, 487, 486, 485, 484,
            483, 482, 481, 480, 479, 478, 477, 476, 475, 474, 473, 472, 473, 473, 473, 472, 471,
            470, 469, 470, 470, 470, 469, 468, 467, 466, 467, 467, 467, 466, 465, 464, 463, 464,
            464, 463, 464, 464, 463, 462, 461, 460, 459, 458, 457, 456, 455, 454, 453, 452, 451,
            450, 449, 448, 447, 446, 445, 444, 443, 442, 441, 440, 439, 438, 437, 436, 435, 434,
            433, 432, 431, 430, 429, 428, 427, 426, 425, 424, 425, 425, 428, 428, 428, 428, 461,
            485, 491, 485, 484, 483, 482, 481, 480, 479, 478, 477, 476, 475, 474, 473, 472, 471,
            470, 469, 468, 467, 466, 465, 464, 463, 462, 461, 460, 461, 461, 461, 460, 459, 458,
            457, 456, 455, 454, 455, 455, 455, 455, 455, 455, 454, 455, 453, 452, 451, 450, 449,
            448, 447, 446, 445, 444, 443, 442, 441, 440, 439, 438, 437, 436, 435, 434, 433, 432,
            431, 430, 429, 428, 427, 426, 425, 424, 423, 422, 421, 420, 419, 420, 420, 423, 423,
            423, 423, 456, 480, 486, 480, 479, 478, 477, 476, 475, 474, 473, 472, 471, 470, 469,
            468, 467, 466, 465, 464, 463, 462, 461, 460, 459, 458, 457, 456, 455, 456, 456, 456,
            455, 454, 453, 452, 451, 450, 449, 448, 447, 446, 447, 447, 447, 447, 447, 447, 446,
            447, 445, 444, 443, 442, 441, 440, 439, 438, 437, 436, 435, 434, 433, 432, 431, 430,
            429, 428, 427, 426, 425, 424, 423, 422, 421, 420, 419, 418, 417, 416, 415, 414, 413,
            412, 411, 412, 412, 415, 415, 418, 418, 421, 421, 457, 481, 487, 481, 480, 479, 478,
            477, 476, 475, 474, 473, 472, 471, 470, 469, 468, 467, 466, 465, 464, 463, 462, 461,
            460, 459, 458, 457, 456, 457, 457, 457, 456, 455, 454, 453, 454, 454, 454, 453, 452,
            451, 450, 451, 451, 451, 450, 449, 448, 447, 448, 448, 447, 448, 448, 447, 446, 445,
            444, 443, 442, 441, 440, 439, 438, 437, 436, 435, 434, 433, 432, 431, 430, 429, 428,
            427, 426, 425, 424, 423, 422, 421, 420, 419, 418, 417, 416, 415, 414, 413, 412, 411,
            410, 409, 408, 409, 409, 412, 412, 415, 415, 418, 418, 454, 478, 484, 478, 477, 476,
            475, 474, 473, 472, 471, 470, 469, 468, 467, 466, 465, 464, 463, 462, 461, 460, 459,
            458, 457, 456, 455, 454, 453, 454, 454, 454, 453, 452, 451, 450, 451, 451, 451, 450,
            449, 448, 447, 448, 448, 448, 447, 446, 445, 444, 445, 445, 444, 445, 445, 444, 443,
            442, 441, 440, 439, 438, 437, 436, 435, 434, 433, 432, 431, 430, 429, 428, 427, 426,
            425, 424, 423, 422, 421, 420, 419, 418, 417, 416, 415, 414, 413, 412, 411, 410, 409,
            408, 407, 406, 405, 406, 406, 409, 409, 412, 412, 415, 415, 451, 475, 481, 475, 474,
            473, 472, 471, 470, 469, 468, 467, 466, 465, 464, 463, 462, 461, 460, 459, 458, 457,
            456, 455, 454, 453, 452, 451, 450, 451, 451, 451, 450, 449, 448, 447, 448, 448, 448,
            447, 446, 445, 444, 445, 445, 445, 444, 443, 442, 441, 442, 442, 441, 442, 442, 441,
            440, 439, 438, 437, 436, 435, 434, 433, 432, 431, 430, 429, 428, 427, 426, 425, 424,
            423, 422, 421, 420, 419, 418, 417, 416, 415, 414, 413, 412, 411, 410, 409, 408, 407,
            406, 405, 404, 403, 402, 403, 403, 406, 406, 406, 406, 442, 466, 472, 466, 465, 464,
            463, 462, 461, 460, 459, 458, 457, 456, 455, 454, 453, 452, 451, 450, 449, 448, 447,
            446, 445, 444, 443, 442, 441, 442, 442, 442, 441, 440, 439, 438, 439, 439, 439, 439,
            439, 439, 438, 439, 435, 434, 433, 432, 431, 430, 429, 428, 427, 426, 425, 424, 423,
            422, 421, 420, 419, 418, 417, 416, 415, 414, 413, 412, 411, 410, 409, 408, 407, 406,
            405, 404, 403, 402, 401, 402, 402, 405, 405, 405, 405, 441, 465, 471, 465, 464, 463,
            462, 461, 460, 459, 458, 457, 456, 455, 454, 453, 452, 451, 450, 449, 448, 447, 446,
            445, 444, 443, 442, 441, 440, 441, 441, 441, 440, 439, 438, 437, 438, 438, 438, 438,
            438, 438, 437, 438, 434, 433, 432, 431, 430, 429, 428, 427, 426, 425, 424, 423, 422,
            421, 420, 419, 418, 417, 416, 415, 414, 413, 412, 411, 410, 409, 408, 407, 406, 405,
            404, 403, 402, 401, 400, 401, 401, 401, 400, 399, 398, 397, 396, 395, 394, 393, 392,
            391, 390, 389, 388, 387, 386, 385, 384, 383, 382, 381, 380, 379, 378, 377, 376, 375,
            374, 373, 372, 371, 370, 369, 368, 367, 366, 365, 364, 363, 362, 361, 360, 359, 358,
            357, 356, 355, 354, 353, 352, 351, 350, 349, 348, 347, 346, 345, 344, 343, 342, 341,
            340, 339, 338, 337, 336, 335, 334, 333, 332, 331, 330, 329, 328, 327, 326, 325, 324,
            323, 322, 321, 320, 319, 318, 317, 316, 315, 314, 313, 312, 311, 310, 309, 308, 307,
            306, 305, 304, 303, 302, 301, 300, 299, 298, 297, 296, 295, 294, 293, 292, 291, 290,
            289, 288, 287, 286, 285, 284, 283, 282, 281, 280, 279, 278, 277, 276, 275, 274, 273,
            272, 271, 270, 269, 268, 267, 266, 265, 264, 263, 262, 261, 260, 259, 258, 257, 256,
            255, 254, 253, 252, 251, 250, 249, 248, 247, 246, 245, 244, 243, 242, 241, 240, 239,
            238, 237, 236, 235, 234, 233, 232, 231, 230, 229, 228, 227, 226, 225, 224, 223, 222,
            221, 220, 219, 218, 217, 216, 215, 214, 213, 212, 211, 210, 209, 208, 207, 206, 205,
            204, 203, 202, 201, 200, 199, 198, 197, 196, 195, 194, 193, 192, 191, 190, 189, 188,
            187, 186, 185, 184, 183, 182, 181, 180, 179, 178, 177, 176, 175, 176, 179, 182, 182,
            182, 182, 182, 181, 180, 179, 178, 177, 176, 175, 176, 203, 203, 200, 199, 198, 197,
            196, 195, 194, 193, 192, 191, 190, 189, 188, 187, 186, 185, 184, 183, 182, 181, 180,
            179, 178, 177, 176, 175, 176, 179, 182, 182, 181, 180, 179, 178, 177, 176, 175, 176,
            224, 224, 227, 254, 257, 257, 254, 253, 251, 251, 251, 233, 232, 231, 230, 229, 228,
            227, 226, 225, 224, 223, 222, 221, 220, 219, 218, 217, 216, 215, 214, 213, 212, 211,
            210, 209, 208, 207, 206, 205, 204, 203, 202, 201, 200, 199, 198, 197, 196, 195, 194,
            193, 192, 191, 190, 189, 188, 187, 186, 185, 184, 183, 182, 181, 180, 179, 178, 179,
            179,
        ],
        &[
            272, 272, 272, 272, 272, 272, 272, 272, 272, 305, 305, 305, 305, 308, 311, 308, 311,
            308, 311, 308, 308, 311, 311, 308, 311, 311, 311, 314, 314, 311, 311, 314, 314, 314,
            311, 314, 314, 314, 317, 317, 317, 314, 314, 317, 317, 317, 317, 320, 320, 320, 320,
            320, 323, 320, 323, 320, 323, 320, 320, 323, 323, 320, 323, 323, 323, 326, 326, 323,
            323, 326, 326, 323, 326, 326, 326, 329, 329, 326, 326, 329, 329, 329, 326, 329, 329,
            329, 332, 332, 332, 329, 329, 332, 332, 332, 335, 335, 335, 338, 338, 338, 338, 341,
            341, 341, 338, 341, 341, 338, 341, 341, 338, 341, 341, 341, 341, 344, 344, 344, 347,
            347, 347, 350, 350, 350, 350, 350, 350, 353, 353, 353, 356, 356, 356, 356, 359, 359,
            359, 359, 359, 356, 359, 359, 359, 362, 362, 362, 362, 362, 359, 359, 362, 362, 362,
            365, 365, 365, 368, 368, 368, 368, 371, 371, 371, 374, 374, 374, 377, 377, 377, 377,
            377, 380, 377, 380, 377, 380, 377, 377, 380, 380, 377, 380, 380, 377, 380, 380, 377,
            377, 380, 380, 380, 377, 380, 380, 377, 380, 380, 380, 377, 377, 380, 380, 380, 377,
            380, 380, 380, 377, 377, 380, 377, 380, 377, 380, 377, 377, 380, 380, 377, 380, 380,
            377, 380, 380, 377, 377, 380, 380, 377, 380, 380, 377, 380, 380, 377, 377, 380, 380,
            380, 377, 380, 380, 377, 380, 380, 380, 377, 377, 380, 380, 377, 380, 380, 377, 380,
            380, 377, 377, 380, 380, 380, 377, 380, 380, 377, 380, 380, 377, 380, 380, 380, 377,
            380, 380, 377, 380, 380, 377, 380, 380, 380, 380, 380, 377, 380, 380, 377, 380, 380,
            377, 377, 380, 380, 380, 380, 380, 377, 380, 380, 377, 380, 380, 380, 380, 380, 377,
            377, 380, 380, 377, 380, 380, 377, 380, 380, 377, 377, 380, 380, 377, 380, 380, 377,
            380, 380, 377, 377, 377, 377, 377, 377, 377, 377, 377, 377, 377, 377, 377, 377, 377,
            377, 377, 377, 377, 377, 377, 377, 377, 377, 377, 377, 377, 377, 377, 377, 377, 377,
            401, 404, 401, 400, 399, 398, 397, 396, 395, 394, 393, 392, 391, 390, 389, 388, 387,
            386, 385, 384, 383, 382, 381, 380, 379, 378, 377, 376, 377, 380, 380, 383, 383, 386,
            386, 389, 389, 392, 392, 395, 395, 398, 398, 401, 401, 404, 404, 407, 407, 410, 410,
            413, 413, 416, 416, 419, 419, 422, 422, 425, 425, 428, 428, 431, 431, 434, 434, 437,
            437, 440, 440, 443, 443, 446, 446, 446, 446, 449, 449, 452, 452, 455, 455, 458, 458,
            461, 461, 464, 464, 467, 467, 470, 470, 473, 473, 476, 476, 479, 479, 482, 482, 485,
            485, 488, 488, 491, 491, 494, 494, 497, 497, 500, 500, 503, 503, 506, 506, 509, 509,
            512, 512, 515, 515, 515, 515, 515, 515, 515, 515, 515, 515, 515, 515, 515, 515, 515,
            515, 515, 515, 515, 515, 515, 515, 515, 515, 515, 515, 515, 515, 515, 518, 518, 518,
            554, 578, 584, 578, 577, 576, 575, 574, 573, 572, 571, 570, 569, 568, 567, 566, 565,
            564, 563, 562, 561, 560, 559, 558, 557, 556, 555, 554, 553, 554, 554, 554, 554, 554,
            553, 552, 551, 550, 551, 551, 551, 551, 550, 551, 547, 546, 545, 544, 543, 542, 541,
            540, 539, 538, 537, 536, 535, 534, 533, 532, 531, 530, 529, 528, 527, 526, 525, 524,
            523, 522, 521, 520, 519, 518, 517, 516, 515, 514, 513, 514, 514, 514, 517, 517, 517,
            553, 577, 583, 577, 576, 575, 574, 573, 572, 571, 570, 569, 568, 567, 566, 565, 564,
            563, 562, 561, 560, 559, 558, 557, 556, 555, 554, 553, 552, 553, 553, 553, 553, 553,
            552, 551, 550, 549, 550, 550, 550, 550, 549, 550, 546, 545, 544, 543, 542, 541, 540,
            539, 538, 537, 536, 535, 534, 533, 532, 531, 530, 529, 528, 527, 526, 525, 524, 523,
            522, 521, 520, 519, 518, 517, 516, 515, 514, 513, 512, 513, 513, 513, 516, 516, 516,
            552, 576, 582, 576, 575, 574, 573, 572, 571, 570, 569, 568, 567, 566, 565, 564, 563,
            562, 561, 560, 559, 558, 557, 556, 555, 554, 553, 552, 551, 552, 552, 552, 552, 552,
            551, 550, 549, 548, 549, 549, 549, 549, 548, 549, 545, 544, 543, 542, 541, 540, 539,
            538, 537, 536, 535, 534, 533, 532, 531, 530, 529, 528, 527, 526, 525, 524, 523, 522,
            521, 520, 519, 518, 517, 516, 515, 514, 513, 512, 511, 512, 512, 515, 515, 518, 518,
            552, 576, 582, 576, 575, 574, 573, 572, 571, 570, 569, 568, 567, 566, 565, 564, 563,
            562, 561, 560, 559, 558, 557, 556, 555, 554, 553, 552, 551, 552, 552, 552, 551, 550,
            549, 548, 549, 549, 549, 548, 547, 546, 545, 546, 546, 545, 546, 546, 545, 544, 543,
            542, 541, 540, 539, 538, 537, 536, 535, 534, 533, 532, 531, 530, 529, 528, 527, 526,
            525, 524, 523, 522, 521, 520, 519, 518, 517, 516, 515, 514, 513, 512, 511, 510, 509,
            510, 510, 510, 510, 510, 546, 570, 576, 570, 569, 568, 567, 566, 565, 564, 563, 562,
            561, 560, 559, 558, 557, 556, 555, 554, 553, 552, 551, 550, 549, 548, 547, 546, 545,
            546, 546, 546, 546, 546, 546, 546, 546, 545, 546, 540, 539, 538, 537, 536, 535, 534,
            533, 532, 531, 530, 529, 528, 527, 526, 525, 524, 523, 522, 521, 520, 519, 518, 517,
            516, 515, 514, 513, 512, 511, 510, 509, 510, 510, 510, 513, 513, 513, 549, 573, 579,
            573, 572, 571, 570, 569, 568, 567, 566, 565, 564, 563, 562, 561, 560, 559, 558, 557,
            556, 555, 554, 553, 552, 551, 550, 549, 548, 549, 549, 549, 549, 549, 548, 547, 546,
            545, 546, 546, 546, 546, 545, 546, 542, 541, 540, 539, 538, 537, 536, 535, 534, 533,
            532, 531, 530, 529, 528, 527, 526, 525, 524, 523, 522, 521, 520, 519, 518, 517, 516,
            515, 514, 513, 512, 511, 510, 509, 508, 509, 509, 509, 512, 512, 512, 545, 569, 575,
            569, 568, 567, 566, 565, 564, 563, 562, 561, 560, 559, 558, 557, 556, 555, 554, 553,
            552, 551, 550, 549, 548, 547, 546, 545, 544, 545, 545, 545, 545, 545, 544, 543, 542,
            541, 540, 539, 540, 540, 540, 540, 539, 540, 538, 537, 536, 535, 534, 533, 532, 531,
            530, 529, 528, 527, 526, 525, 524, 523, 522, 521, 520, 519, 518, 517, 516, 515, 514,
            513, 512, 511, 510, 509, 508, 507, 506, 505, 504, 505, 505, 505, 508, 508, 508, 544,
            568, 574, 568, 567, 566, 565, 564, 563, 562, 561, 560, 559, 558, 557, 556, 555, 554,
            553, 552, 551, 550, 549, 548, 547, 546, 545, 544, 543, 544, 544, 544, 544, 544, 543,
            542, 541, 540, 541, 541, 541, 541, 540, 541, 537, 536, 535, 534, 533, 532, 531, 530,
            529, 528, 527, 526, 525, 524, 523, 522, 521, 520, 519, 518, 517, 516, 515, 514, 513,
            512, 511, 510, 509, 508, 507, 506, 505, 504, 503, 504, 504, 507, 507, 510, 510, 513,
            513, 549, 573, 579, 573, 572, 571, 570, 569, 568, 567, 566, 565, 564, 563, 562, 561,
            560, 559, 558, 557, 556, 555, 554, 553, 552, 551, 550, 549, 548, 549, 549, 549, 548,
            547, 546, 545, 546, 546, 546, 545, 544, 543, 542, 543, 543, 543, 542, 541, 540, 539,
            540, 540, 539, 540, 540, 539, 538, 537, 536, 535, 534, 533, 532, 531, 530, 529, 528,
            527, 526, 525, 524, 523, 522, 521, 520, 519, 518, 517, 516, 515, 514, 513, 512, 511,
            510, 509, 508, 507, 506, 505, 504, 503, 502, 501, 500, 501, 501, 501, 501, 501, 535,
            559, 565, 559, 558, 557, 556, 555, 554, 553, 552, 551, 550, 549, 548, 547, 546, 545,
            544, 543, 542, 541, 540, 539, 538, 537, 536, 535, 534, 535, 535, 535, 535, 535, 535,
            534, 535, 531, 530, 529, 528, 527, 526, 525, 524, 523, 522, 521, 520, 519, 518, 517,
            516, 515, 514, 513, 512, 511, 510, 509, 508, 507, 506, 505, 504, 503, 502, 501, 500,
            501, 501, 501, 504, 504, 507, 507, 541, 565, 571, 565, 564, 563, 562, 561, 560, 559,
            558, 557, 556, 555, 554, 553, 552, 551, 550, 549, 548, 547, 546, 545, 544, 543, 542,
            541, 540, 541, 541, 541, 540, 539, 538, 537, 538, 538, 538, 537, 536, 535, 534, 535,
            535, 534, 535, 535, 534, 533, 532, 531, 530, 529, 528, 527, 526, 525, 524, 523, 522,
            521, 520, 519, 518, 517, 516, 515, 514, 513, 512, 511, 510, 509, 508, 507, 506, 505,
            504, 503, 502, 501, 500, 499, 498, 499, 499, 499, 502, 502, 505, 505, 539, 563, 569,
            563, 562, 561, 560, 559, 558, 557, 556, 555, 554, 553, 552, 551, 550, 549, 548, 547,
            546, 545, 544, 543, 542, 541, 540, 539, 538, 539, 539, 539, 538, 537, 536, 535, 536,
            536, 536, 535, 534, 533, 532, 533, 533, 532, 533, 533, 532, 531, 530, 529, 528, 527,
            526, 525, 524, 523, 522, 521, 520, 519, 518, 517, 516, 515, 514, 513, 512, 511, 510,
            509, 508, 507, 506, 505, 504, 503, 502, 501, 500, 499, 498, 497, 496, 497, 497, 497,
            497, 500, 500, 503, 503, 536, 560, 566, 560, 559, 558, 557, 556, 555, 554, 553, 552,
            551, 550, 549, 548, 547, 546, 545, 544, 543, 542, 541, 540, 539, 538, 537, 536, 535,
            536, 536, 536, 536, 536, 535, 534, 533, 532, 531, 530, 529, 528, 529, 529, 529, 528,
            527, 526, 525, 524, 523, 522, 521, 520, 519, 518, 517, 516, 515, 514, 513, 512, 511,
            510, 509, 508, 507, 506, 505, 504, 503, 502, 501, 500, 499, 498, 497, 496, 495, 494,
            493, 492, 491, 490, 489, 488, 489, 489, 488, 489, 488, 487, 486, 485, 484, 483, 482,
            481, 480, 479, 478, 477, 476, 475, 474, 473, 472, 471, 470, 469, 468, 467, 466, 465,
            464, 463, 462, 461, 460, 459, 458, 457, 456, 455, 454, 453, 452, 451, 452, 452, 452,
            455, 455, 455, 491, 515, 521, 515, 514, 513, 512, 511, 510, 509, 508, 507, 506, 505,
            504, 503, 502, 501, 500, 499, 498, 497, 496, 495, 494, 493, 492, 491, 490, 491, 491,
            491, 491, 491, 490, 489, 488, 487, 488, 488, 488, 488, 487, 488, 484, 483, 482, 481,
            480, 479, 478, 477, 476, 475, 474, 473, 472, 471, 470, 469, 468, 467, 466, 465, 464,
            463, 462, 461, 460, 459, 458, 457, 456, 455, 454, 453, 452, 451, 450, 451, 451, 454,
            454, 457, 457, 460, 460, 496, 520, 526, 520, 519, 518, 517, 516, 515, 514, 513, 512,
            511, 510, 509, 508, 507, 506, 505, 504, 503, 502, 501, 500, 499, 498, 497, 496, 495,
            496, 496, 496, 495, 494, 493, 492, 493, 493, 493, 492, 491, 490, 489, 490, 490, 490,
            489, 488, 487, 486, 487, 487, 486, 487, 487, 486, 485, 484, 483, 482, 481, 480, 479,
            478, 477, 476, 475, 474, 473, 472, 471, 470, 469, 468, 467, 466, 465, 464, 463, 462,
            461, 460, 459, 458, 457, 456, 455, 454, 453, 452, 451, 450, 449, 448, 447, 448, 448,
            451, 451, 454, 454, 457, 457, 493, 517, 523, 517, 516, 515, 514, 513, 512, 511, 510,
            509, 508, 507, 506, 505, 504, 503, 502, 501, 500, 499, 498, 497, 496, 495, 494, 493,
            492, 493, 493, 493, 492, 491, 490, 489, 490, 490, 490, 489, 488, 487, 486, 487, 487,
            487, 486, 485, 484, 483, 484, 484, 483, 484, 484, 483, 482, 481, 480, 479, 478, 477,
            476, 475, 474, 473, 472, 471, 470, 469, 468, 467, 466, 465, 464, 463, 462, 461, 460,
            459, 458, 457, 456, 455, 454, 453, 452, 451, 450, 449, 448, 447, 446, 445, 444, 445,
            445, 445, 448, 448, 448, 481, 505, 511, 505, 504, 503, 502, 501, 500, 499, 498, 497,
            496, 495, 494, 493, 492, 491, 490, 489, 488, 487, 486, 485, 484, 483, 482, 481, 480,
            481, 481, 481, 481, 481, 480, 479, 478, 477, 476, 475, 476, 476, 476, 476, 475, 476,
            474, 473, 472, 471, 470, 469, 468, 467, 466, 465, 464, 463, 462, 461, 460, 459, 458,
            457, 456, 455, 454, 453, 452, 451, 450, 449, 448, 447, 446, 445, 444, 443, 442, 441,
            440, 441, 441, 441, 444, 444, 444, 477, 501, 507, 501, 500, 499, 498, 497, 496, 495,
            494, 493, 492, 491, 490, 489, 488, 487, 486, 485, 484, 483, 482, 481, 480, 479, 478,
            477, 476, 477, 477, 477, 477, 477, 476, 475, 474, 473, 472, 471, 470, 469, 468, 469,
            469, 469, 469, 468, 469, 467, 466, 465, 464, 463, 462, 461, 460, 459, 458, 457, 456,
            455, 454, 453, 452, 451, 450, 449, 448, 447, 446, 445, 444, 443, 442, 441, 440, 439,
            438, 437, 436, 435, 434, 433, 434, 434, 437, 437, 440, 440, 443, 443, 479, 503, 509,
            503, 502, 501, 500, 499, 498, 497, 496, 495, 494, 493, 492, 491, 490, 489, 488, 487,
            486, 485, 484, 483, 482, 481, 480, 479, 478, 479, 479, 479, 478, 477, 476, 475, 476,
            476, 476, 475, 474, 473, 472, 473, 473, 473, 472, 471, 470, 469, 470, 470, 469, 470,
            470, 469, 468, 467, 466, 465, 464, 463, 462, 461, 460, 459, 458, 457, 456, 455, 454,
            453, 452, 451, 450, 449, 448, 447, 446, 445, 444, 443, 442, 441, 440, 439, 438, 437,
            436, 435, 434, 433, 432, 431, 430, 431, 431, 434, 434, 437, 437, 440, 440, 476, 500,
            506, 500, 499, 498, 497, 496, 495, 494, 493, 492, 491, 490, 489, 488, 487, 486, 485,
            484, 483, 482, 481, 480, 479, 478, 477, 476, 475, 476, 476, 476, 475, 474, 473, 472,
            473, 473, 473, 472, 471, 470, 469, 470, 470, 470, 469, 468, 467, 466, 467, 467, 466,
            467, 467, 466, 465, 464, 463, 462, 461, 460, 459, 458, 457, 456, 455, 454, 453, 452,
            451, 450, 449, 448, 447, 446, 445, 444, 443, 442, 441, 440, 439, 438, 437, 436, 435,
            434, 433, 432, 431, 430, 429, 428, 427, 428, 428, 431, 431, 434, 434, 437, 437, 473,
            497, 503, 497, 496, 495, 494, 493, 492, 491, 490, 489, 488, 487, 486, 485, 484, 483,
            482, 481, 480, 479, 478, 477, 476, 475, 474, 473, 472, 473, 473, 473, 472, 471, 470,
            469, 470, 470, 470, 469, 468, 467, 466, 467, 467, 467, 466, 465, 464, 463, 464, 464,
            463, 464, 464, 463, 462, 461, 460, 459, 458, 457, 456, 455, 454, 453, 452, 451, 450,
            449, 448, 447, 446, 445, 444, 443, 442, 441, 440, 439, 438, 437, 436, 435, 434, 433,
            432, 431, 430, 429, 428, 427, 426, 425, 424, 425, 425, 425, 428, 428, 428, 464, 488,
            494, 488, 487, 486, 485, 484, 483, 482, 481, 480, 479, 478, 477, 476, 475, 474, 473,
            472, 471, 470, 469, 468, 467, 466, 465, 464, 463, 464, 464, 464, 464, 464, 463, 462,
            461, 460, 461, 461, 461, 461, 460, 461, 457, 456, 455, 454, 453, 452, 451, 450, 449,
            448, 447, 446, 445, 444, 443, 442, 441, 440, 439, 438, 437, 436, 435, 434, 433, 432,
            431, 430, 429, 428, 427, 426, 425, 424, 423, 424, 424, 424, 427, 427, 427, 463, 487,
            493, 487, 486, 485, 484, 483, 482, 481, 480, 479, 478, 477, 476, 475, 474, 473, 472,
            471, 470, 469, 468, 467, 466, 465, 464, 463, 462, 463, 463, 463, 463, 463, 462, 461,
            460, 459, 460, 460, 460, 460, 459, 460, 456, 455, 454, 453, 452, 451, 450, 449, 448,
            447, 446, 445, 444, 443, 442, 441, 440, 439, 438, 437, 436, 435, 434, 433, 432, 431,
            430, 429, 428, 427, 426, 425, 424, 423, 422, 423, 423, 423, 422, 421, 420, 419, 418,
            417, 416, 415, 414, 413, 412, 411, 410, 409, 408, 407, 406, 405, 404, 403, 402, 401,
            400, 399, 398, 397, 396, 395, 394, 393, 392, 391, 390, 389, 388, 387, 386, 385, 384,
            383, 382, 381, 380, 379, 378, 377, 376, 375, 374, 373, 372, 371, 370, 369, 368, 367,
            366, 365, 364, 363, 362, 361, 360, 359, 358, 357, 356, 355, 354, 353, 352, 351, 350,
            349, 348, 347, 346, 345, 344, 343, 342, 341, 340, 339, 338, 337, 336, 335, 334, 333,
            332, 331, 330, 329, 328, 327, 326, 325, 324, 323, 322, 321, 320, 319, 318, 317, 316,
            315, 314, 313, 312, 311, 310, 309, 308, 307, 306, 305, 304, 303, 302, 301, 300, 299,
            298, 297, 296, 295, 294, 293, 292, 291, 290, 289, 288, 287, 286, 285, 284, 283, 282,
            281, 280, 279, 278, 277, 276, 275, 274, 273, 272, 271, 270, 269, 268, 267, 266, 265,
            264, 263, 262, 261, 260, 259, 258, 257, 256, 255, 254, 253, 252, 251, 250, 249, 248,
            247, 246, 245, 244, 243, 242, 241, 240, 239, 238, 237, 236, 235, 234, 233, 232, 231,
            230, 229, 228, 227, 226, 225, 224, 223, 222, 221, 220, 219, 218, 217, 216, 215, 214,
            213, 212, 211, 210, 209, 208, 207, 206, 205, 204, 203, 202, 201, 200, 199, 198, 197,
            196, 195, 194, 193, 192, 191, 190, 189, 188, 187, 186, 185, 184, 183, 182, 181, 180,
            179, 180, 183, 186, 186, 186, 186, 186, 185, 184, 183, 182, 181, 180, 179, 180, 207,
            207, 204, 203, 202, 201, 200, 199, 198, 197, 196, 195, 194, 193, 192, 191, 190, 189,
            188, 187, 186, 185, 184, 183, 182, 181, 180, 179, 180, 183, 186, 186, 185, 184, 183,
            182, 181, 180, 179, 180, 228, 228, 231, 258, 261, 261, 258, 257, 255, 255, 255, 237,
            236, 235, 234, 233, 232, 231, 230, 229, 228, 227, 226, 225, 224, 223, 222, 221, 220,
            219, 218, 217, 216, 215, 214, 213, 212, 211, 210, 209, 208, 207, 206, 205, 204, 203,
            202, 201, 200, 199, 198, 197, 196, 195, 194, 193, 192, 191, 190, 189, 188, 187, 186,
            185, 184, 183, 182, 183, 183,
        ],
        &[
            272, 272, 272, 272, 272, 272, 272, 272, 272, 305, 305, 305, 305, 305, 308, 311, 308,
            311, 308, 311, 308, 308, 311, 311, 308, 311, 311, 308, 311, 311, 311, 311, 314, 314,
            314, 311, 314, 314, 314, 311, 314, 314, 314, 314, 317, 317, 317, 314, 317, 317, 317,
            314, 314, 317, 314, 317, 314, 317, 314, 314, 317, 317, 314, 317, 317, 314, 317, 317,
            317, 317, 320, 320, 317, 320, 320, 317, 320, 320, 320, 320, 323, 323, 323, 320, 323,
            323, 323, 320, 323, 323, 323, 323, 326, 326, 326, 326, 323, 326, 326, 326, 323, 326,
            326, 326, 323, 323, 326, 326, 326, 323, 326, 326, 323, 326, 326, 323, 326, 326, 326,
            326, 323, 326, 326, 323, 326, 326, 323, 326, 326, 326, 326, 329, 329, 329, 332, 332,
            332, 332, 335, 335, 335, 335, 338, 338, 338, 338, 341, 341, 341, 341, 344, 344, 344,
            344, 347, 347, 347, 347, 350, 350, 350, 350, 350, 353, 353, 350, 353, 353, 350, 353,
            353, 350, 350, 350, 353, 350, 353, 350, 353, 350, 350, 353, 353, 350, 353, 353, 350,
            353, 353, 350, 350, 353, 353, 353, 350, 353, 353, 353, 350, 353, 353, 350, 350, 353,
            353, 353, 350, 353, 353, 353, 350, 350, 353, 350, 353, 350, 353, 350, 350, 353, 353,
            350, 353, 353, 350, 353, 353, 350, 350, 353, 353, 350, 353, 353, 350, 353, 353, 350,
            350, 353, 353, 353, 350, 353, 353, 353, 350, 353, 353, 350, 350, 353, 353, 353, 353,
            353, 356, 356, 356, 353, 356, 356, 356, 353, 353, 356, 356, 356, 353, 356, 356, 353,
            356, 356, 353, 356, 356, 356, 356, 353, 356, 356, 353, 356, 356, 353, 356, 356, 356,
            353, 356, 356, 353, 356, 356, 353, 353, 356, 356, 356, 353, 356, 356, 356, 353, 356,
            356, 353, 353, 356, 356, 356, 353, 356, 356, 356, 353, 356, 356, 356, 353, 353, 356,
            356, 353, 356, 356, 353, 356, 356, 353, 353, 353, 356, 353, 356, 353, 356, 353, 353,
            356, 356, 353, 356, 356, 353, 356, 356, 353, 353, 356, 356, 356, 353, 356, 356, 356,
            353, 356, 356, 353, 353, 356, 356, 356, 353, 356, 356, 356, 353, 353, 356, 353, 356,
            353, 356, 353, 353, 356, 356, 353, 356, 356, 353, 356, 356, 353, 353, 356, 356, 353,
            356, 356, 353, 356, 356, 353, 353, 356, 356, 356, 353, 356, 356, 356, 353, 356, 356,
            353, 353, 356, 356, 356, 356, 353, 356, 356, 356, 353, 356, 356, 356, 353, 353, 356,
            356, 356, 353, 356, 356, 353, 356, 356, 353, 356, 356, 356, 356, 353, 356, 356, 353,
            356, 356, 353, 356, 356, 356, 353, 356, 356, 353, 356, 356, 353, 353, 356, 356, 356,
            353, 356, 356, 356, 353, 356, 356, 353, 353, 356, 356, 356, 353, 356, 356, 356, 353,
            356, 356, 356, 353, 353, 356, 356, 353, 356, 356, 353, 356, 356, 353, 353, 353, 353,
            353, 353, 353, 353, 353, 353, 353, 353, 353, 353, 353, 353, 353, 353, 353, 353, 353,
            353, 353, 353, 353, 353, 353, 353, 353, 353, 353, 353, 377, 380, 377, 376, 375, 374,
            373, 372, 371, 370, 369, 368, 367, 366, 365, 364, 363, 362, 361, 360, 359, 358, 357,
            356, 355, 354, 353, 352, 353, 356, 356, 359, 359, 362, 362, 365, 365, 368, 368, 371,
            371, 374, 374, 377, 377, 380, 380, 383, 383, 386, 386, 389, 389, 392, 392, 395, 395,
            398, 398, 398, 398, 401, 401, 404, 404, 407, 407, 410, 410, 413, 413, 416, 416, 419,
            419, 422, 422, 425, 425, 428, 428, 431, 431, 434, 434, 437, 437, 440, 440, 443, 443,
            443, 443, 443, 443, 443, 443, 443, 443, 443, 443, 443, 443, 443, 443, 443, 443, 443,
            443, 443, 443, 446, 446, 482, 506, 512, 506, 505, 504, 503, 502, 501, 500, 499, 498,
            497, 496, 495, 494, 493, 492, 491, 490, 489, 488, 487, 486, 485, 484, 483, 482, 481,
            482, 482, 482, 482, 482, 482, 482, 481, 480, 479, 478, 479, 479, 478, 479, 475, 474,
            473, 472, 471, 470, 469, 468, 467, 466, 465, 464, 463, 462, 461, 460, 459, 458, 457,
            456, 455, 454, 453, 452, 451, 450, 449, 448, 447, 446, 445, 444, 443, 442, 441, 442,
            442, 442, 442, 445, 445, 481, 505, 511, 505, 504, 503, 502, 501, 500, 499, 498, 497,
            496, 495, 494, 493, 492, 491, 490, 489, 488, 487, 486, 485, 484, 483, 482, 481, 480,
            481, 481, 481, 481, 481, 481, 481, 480, 479, 478, 477, 478, 478, 477, 478, 474, 473,
            472, 471, 470, 469, 468, 467, 466, 465, 464, 463, 462, 461, 460, 459, 458, 457, 456,
            455, 454, 453, 452, 451, 450, 449, 448, 447, 446, 445, 444, 443, 442, 441, 440, 441,
            441, 441, 441, 444, 444, 480, 504, 510, 504, 503, 502, 501, 500, 499, 498, 497, 496,
            495, 494, 493, 492, 491, 490, 489, 488, 487, 486, 485, 484, 483, 482, 481, 480, 479,
            480, 480, 480, 480, 480, 480, 480, 479, 478, 477, 476, 477, 477, 476, 477, 473, 472,
            471, 470, 469, 468, 467, 466, 465, 464, 463, 462, 461, 460, 459, 458, 457, 456, 455,
            454, 453, 452, 451, 450, 449, 448, 447, 446, 445, 444, 443, 442, 441, 440, 439, 440,
            440, 440, 440, 474, 498, 504, 498, 497, 496, 495, 494, 493, 492, 491, 490, 489, 488,
            487, 486, 485, 484, 483, 482, 481, 480, 479, 478, 477, 476, 475, 474, 473, 474, 474,
            474, 474, 474, 474, 473, 474, 470, 469, 468, 467, 466, 465, 464, 463, 462, 461, 460,
            459, 458, 457, 456, 455, 454, 453, 452, 451, 450, 449, 448, 447, 446, 445, 444, 443,
            442, 441, 440, 439, 440, 440, 440, 440, 440, 476, 500, 506, 500, 499, 498, 497, 496,
            495, 494, 493, 492, 491, 490, 489, 488, 487, 486, 485, 484, 483, 482, 481, 480, 479,
            478, 477, 476, 475, 476, 476, 476, 476, 476, 476, 476, 476, 475, 476, 470, 469, 468,
            467, 466, 465, 464, 463, 462, 461, 460, 459, 458, 457, 456, 455, 454, 453, 452, 451,
            450, 449, 448, 447, 446, 445, 444, 443, 442, 441, 440, 439, 440, 440, 440, 440, 443,
            443, 479, 503, 509, 503, 502, 501, 500, 499, 498, 497, 496, 495, 494, 493, 492, 491,
            490, 489, 488, 487, 486, 485, 484, 483, 482, 481, 480, 479, 478, 479, 479, 479, 479,
            479, 479, 479, 478, 477, 476, 475, 476, 476, 475, 476, 472, 471, 470, 469, 468, 467,
            466, 465, 464, 463, 462, 461, 460, 459, 458, 457, 456, 455, 454, 453, 452, 451, 450,
            449, 448, 447, 446, 445, 444, 443, 442, 441, 440, 439, 438, 439, 439, 439, 439, 442,
            442, 475, 499, 505, 499, 498, 497, 496, 495, 494, 493, 492, 491, 490, 489, 488, 487,
            486, 485, 484, 483, 482, 481, 480, 479, 478, 477, 476, 475, 474, 475, 475, 475, 475,
            475, 475, 475, 474, 473, 472, 471, 470, 469, 468, 467, 468, 468, 467, 468, 466, 465,
            464, 463, 462, 461, 460, 459, 458, 457, 456, 455, 454, 453, 452, 451, 450, 449, 448,
            447, 446, 445, 444, 443, 442, 441, 440, 439, 438, 437, 436, 435, 434, 433, 432, 433,
            433, 433, 433, 436, 436, 472, 496, 502, 496, 495, 494, 493, 492, 491, 490, 489, 488,
            487, 486, 485, 484, 483, 482, 481, 480, 479, 478, 477, 476, 475, 474, 473, 472, 471,
            472, 472, 472, 472, 472, 472, 472, 471, 470, 469, 468, 469, 469, 468, 469, 465, 464,
            463, 462, 461, 460, 459, 458, 457, 456, 455, 454, 453, 452, 451, 450, 449, 448, 447,
            446, 445, 444, 443, 442, 441, 440, 439, 438, 437, 436, 435, 434, 433, 432, 431, 432,
            432, 435, 435, 435, 435, 471, 495, 501, 495, 494, 493, 492, 491, 490, 489, 488, 487,
            486, 485, 484, 483, 482, 481, 480, 479, 478, 477, 476, 475, 474, 473, 472, 471, 470,
            471, 471, 471, 470, 469, 468, 467, 468, 468, 468, 468, 468, 468, 467, 468, 464, 463,
            462, 461, 460, 459, 458, 457, 456, 455, 454, 453, 452, 451, 450, 449, 448, 447, 446,
            445, 444, 443, 442, 441, 440, 439, 438, 437, 436, 435, 434, 433, 432, 431, 430, 431,
            431, 431, 431, 431, 465, 489, 495, 489, 488, 487, 486, 485, 484, 483, 482, 481, 480,
            479, 478, 477, 476, 475, 474, 473, 472, 471, 470, 469, 468, 467, 466, 465, 464, 465,
            465, 465, 465, 465, 465, 464, 465, 461, 460, 459, 458, 457, 456, 455, 454, 453, 452,
            451, 450, 449, 448, 447, 446, 445, 444, 443, 442, 441, 440, 439, 438, 437, 436, 435,
            434, 433, 432, 431, 430, 431, 431, 431, 431, 431, 465, 489, 495, 489, 488, 487, 486,
            485, 484, 483, 482, 481, 480, 479, 478, 477, 476, 475, 474, 473, 472, 471, 470, 469,
            468, 467, 466, 465, 464, 465, 465, 465, 465, 465, 465, 464, 465, 461, 460, 459, 458,
            457, 456, 455, 454, 453, 452, 451, 450, 449, 448, 447, 446, 445, 444, 443, 442, 441,
            440, 439, 438, 437, 436, 435, 434, 433, 432, 431, 430, 431, 431, 431, 434, 434, 437,
            437, 471, 495, 501, 495, 494, 493, 492, 491, 490, 489, 488, 487, 486, 485, 484, 483,
            482, 481, 480, 479, 478, 477, 476, 475, 474, 473, 472, 471, 470, 471, 471, 471, 470,
            469, 468, 467, 468, 468, 468, 467, 466, 465, 464, 465, 465, 464, 465, 465, 464, 463,
            462, 461, 460, 459, 458, 457, 456, 455, 454, 453, 452, 451, 450, 449, 448, 447, 446,
            445, 444, 443, 442, 441, 440, 439, 438, 437, 436, 435, 434, 433, 432, 431, 430, 429,
            428, 429, 429, 429, 429, 429, 432, 432, 465, 489, 495, 489, 488, 487, 486, 485, 484,
            483, 482, 481, 480, 479, 478, 477, 476, 475, 474, 473, 472, 471, 470, 469, 468, 467,
            466, 465, 464, 465, 465, 465, 465, 465, 465, 465, 464, 463, 462, 461, 460, 459, 458,
            457, 456, 455, 454, 453, 452, 451, 450, 449, 448, 447, 446, 445, 444, 443, 442, 441,
            440, 439, 438, 437, 436, 435, 434, 433, 432, 431, 430, 429, 428, 427, 426, 425, 424,
            425, 425, 424, 425, 423, 422, 421, 420, 419, 418, 417, 416, 415, 414, 413, 412, 411,
            410, 409, 408, 407, 406, 405, 404, 403, 402, 401, 400, 399, 398, 397, 396, 395, 394,
            393, 392, 391, 390, 389, 390, 390, 393, 393, 396, 396, 399, 399, 435, 459, 465, 459,
            458, 457, 456, 455, 454, 453, 452, 451, 450, 449, 448, 447, 446, 445, 444, 443, 442,
            441, 440, 439, 438, 437, 436, 435, 434, 435, 435, 435, 434, 433, 432, 431, 432, 432,
            432, 431, 430, 429, 428, 429, 429, 429, 428, 427, 426, 425, 426, 426, 425, 426, 426,
            425, 424, 423, 422, 421, 420, 419, 418, 417, 416, 415, 414, 413, 412, 411, 410, 409,
            408, 407, 406, 405, 404, 403, 402, 401, 400, 399, 398, 397, 396, 395, 394, 393, 392,
            391, 390, 389, 388, 387, 386, 387, 387, 390, 390, 393, 393, 396, 396, 432, 456, 462,
            456, 455, 454, 453, 452, 451, 450, 449, 448, 447, 446, 445, 444, 443, 442, 441, 440,
            439, 438, 437, 436, 435, 434, 433, 432, 431, 432, 432, 432, 431, 430, 429, 428, 429,
            429, 429, 428, 427, 426, 425, 426, 426, 426, 425, 424, 423, 422, 423, 423, 422, 423,
            423, 422, 421, 420, 419, 418, 417, 416, 415, 414, 413, 412, 411, 410, 409, 408, 407,
            406, 405, 404, 403, 402, 401, 400, 399, 398, 397, 396, 395, 394, 393, 392, 391, 390,
            389, 388, 387, 386, 385, 384, 383, 384, 384, 384, 384, 384, 420, 444, 450, 444, 443,
            442, 441, 440, 439, 438, 437, 436, 435, 434, 433, 432, 431, 430, 429, 428, 427, 426,
            425, 424, 423, 422, 421, 420, 419, 420, 420, 420, 420, 420, 420, 420, 420, 419, 420,
            414, 413, 412, 411, 410, 409, 408, 407, 406, 405, 404, 403, 402, 401, 400, 399, 398,
            397, 396, 395, 394, 393, 392, 391, 390, 389, 388, 387, 386, 385, 384, 383, 384, 384,
            384, 384, 387, 387, 420, 444, 450, 444, 443, 442, 441, 440, 439, 438, 437, 436, 435,
            434, 433, 432, 431, 430, 429, 428, 427, 426, 425, 424, 423, 422, 421, 420, 419, 420,
            420, 420, 420, 420, 420, 420, 419, 418, 417, 416, 415, 414, 413, 412, 413, 413, 412,
            413, 411, 410, 409, 408, 407, 406, 405, 404, 403, 402, 401, 400, 399, 398, 397, 396,
            395, 394, 393, 392, 391, 390, 389, 388, 387, 386, 385, 384, 383, 382, 381, 380, 379,
            378, 377, 378, 378, 378, 378, 381, 381, 414, 438, 444, 438, 437, 436, 435, 434, 433,
            432, 431, 430, 429, 428, 427, 426, 425, 424, 423, 422, 421, 420, 419, 418, 417, 416,
            415, 414, 413, 414, 414, 414, 414, 414, 414, 414, 413, 412, 411, 410, 409, 408, 407,
            406, 405, 404, 405, 405, 404, 405, 403, 402, 401, 400, 399, 398, 397, 396, 395, 394,
            393, 392, 391, 390, 389, 388, 387, 386, 385, 384, 383, 382, 381, 380, 379, 378, 377,
            376, 375, 374, 373, 372, 371, 370, 369, 370, 370, 370, 370, 370, 406, 430, 436, 430,
            429, 428, 427, 426, 425, 424, 423, 422, 421, 420, 419, 418, 417, 416, 415, 414, 413,
            412, 411, 410, 409, 408, 407, 406, 405, 406, 406, 406, 406, 406, 406, 406, 406, 405,
            406, 400, 399, 398, 397, 396, 395, 394, 393, 392, 391, 390, 389, 388, 387, 386, 385,
            384, 383, 382, 381, 380, 379, 378, 377, 376, 375, 374, 373, 372, 371, 370, 369, 370,
            370, 370, 370, 370, 406, 430, 436, 430, 429, 428, 427, 426, 425, 424, 423, 422, 421,
            420, 419, 418, 417, 416, 415, 414, 413, 412, 411, 410, 409, 408, 407, 406, 405, 406,
            406, 406, 406, 406, 406, 406, 406, 405, 406, 400, 399, 398, 397, 396, 395, 394, 393,
            392, 391, 390, 389, 388, 387, 386, 385, 384, 383, 382, 381, 380, 379, 378, 377, 376,
            375, 374, 373, 372, 371, 370, 369, 370, 370, 370, 370, 370, 406, 430, 436, 430, 429,
            428, 427, 426, 425, 424, 423, 422, 421, 420, 419, 418, 417, 416, 415, 414, 413, 412,
            411, 410, 409, 408, 407, 406, 405, 406, 406, 406, 406, 406, 406, 406, 406, 405, 406,
            400, 399, 398, 397, 396, 395, 394, 393, 392, 391, 390, 389, 388, 387, 386, 385, 384,
            383, 382, 381, 380, 379, 378, 377, 376, 375, 374, 373, 372, 371, 370, 369, 370, 370,
            370, 370, 373, 373, 409, 433, 439, 433, 432, 431, 430, 429, 428, 427, 426, 425, 424,
            423, 422, 421, 420, 419, 418, 417, 416, 415, 414, 413, 412, 411, 410, 409, 408, 409,
            409, 409, 409, 409, 409, 409, 408, 407, 406, 405, 406, 406, 405, 406, 402, 401, 400,
            399, 398, 397, 396, 395, 394, 393, 392, 391, 390, 389, 388, 387, 386, 385, 384, 383,
            382, 381, 380, 379, 378, 377, 376, 375, 374, 373, 372, 371, 370, 369, 368, 369, 369,
            369, 369, 372, 372, 408, 432, 438, 432, 431, 430, 429, 428, 427, 426, 425, 424, 423,
            422, 421, 420, 419, 418, 417, 416, 415, 414, 413, 412, 411, 410, 409, 408, 407, 408,
            408, 408, 408, 408, 408, 408, 407, 406, 405, 404, 405, 405, 404, 405, 401, 400, 399,
            398, 397, 396, 395, 394, 393, 392, 391, 390, 389, 388, 387, 386, 385, 384, 383, 382,
            381, 380, 379, 378, 377, 376, 375, 374, 373, 372, 371, 370, 369, 368, 367, 368, 368,
            368, 367, 366, 365, 364, 363, 362, 361, 360, 359, 358, 357, 356, 355, 354, 353, 352,
            351, 350, 349, 348, 347, 346, 345, 344, 343, 342, 341, 340, 339, 338, 337, 336, 335,
            334, 333, 332, 331, 330, 329, 328, 327, 326, 325, 324, 323, 322, 321, 320, 319, 318,
            317, 316, 315, 314, 313, 312, 311, 310, 309, 308, 307, 306, 305, 304, 303, 302, 301,
            300, 299, 298, 297, 296, 295, 294, 293, 292, 291, 290, 289, 288, 287, 286, 285, 284,
            283, 282, 281, 280, 279, 278, 277, 276, 275, 274, 273, 272, 271, 270, 269, 268, 267,
            266, 265, 264, 263, 262, 261, 260, 259, 258, 257, 256, 255, 254, 253, 252, 251, 250,
            249, 248, 247, 246, 245, 244, 243, 242, 241, 240, 239, 238, 237, 236, 235, 234, 233,
            232, 231, 230, 229, 228, 227, 226, 225, 224, 223, 222, 221, 220, 219, 218, 217, 216,
            215, 214, 213, 212, 211, 210, 209, 208, 207, 206, 205, 204, 203, 202, 201, 200, 199,
            198, 197, 196, 197, 200, 203, 203, 203, 203, 203, 202, 201, 200, 199, 198, 197, 196,
            197, 224, 224, 221, 220, 219, 218, 217, 216, 215, 214, 213, 212, 211, 210, 209, 208,
            207, 206, 205, 204, 203, 202, 201, 200, 199, 198, 197, 196, 197, 200, 203, 203, 202,
            201, 200, 199, 198, 197, 196, 197, 245, 245, 248, 275, 278, 278, 275, 274, 272, 272,
            272, 254, 253, 252, 251, 250, 249, 248, 247, 246, 245, 244, 243, 242, 241, 240, 239,
            238, 237, 236, 235, 234, 233, 232, 231, 230, 229, 228, 227, 226, 225, 224, 223, 222,
            221, 220, 219, 218, 217, 216, 215, 214, 213, 212, 211, 210, 209, 208, 207, 206, 205,
            204, 203, 202, 201, 200, 199, 200, 200,
        ],
        &[
            272, 272, 272, 272, 272, 272, 272, 272, 272, 305, 305, 305, 305, 305, 305, 305, 305,
            305, 305, 305, 305, 305, 305, 305, 305, 305, 305, 305, 305, 305, 305, 305, 305, 305,
            305, 308, 311, 308, 311, 308, 311, 308, 308, 311, 311, 308, 311, 311, 308, 311, 311,
            308, 308, 311, 311, 311, 308, 311, 311, 311, 308, 311, 311, 311, 308, 308, 311, 311,
            311, 308, 311, 311, 311, 308, 308, 311, 308, 311, 308, 311, 308, 308, 311, 311, 308,
            311, 311, 308, 311, 311, 308, 308, 311, 311, 308, 311, 311, 308, 311, 311, 308, 308,
            311, 311, 311, 308, 311, 311, 311, 308, 311, 311, 311, 308, 308, 311, 311, 311, 311,
            308, 311, 311, 311, 308, 311, 311, 311, 308, 308, 311, 311, 311, 308, 311, 311, 308,
            311, 311, 308, 311, 311, 311, 311, 308, 311, 311, 308, 311, 311, 308, 311, 311, 311,
            311, 311, 311, 311, 311, 311, 311, 311, 311, 311, 311, 311, 311, 311, 311, 311, 311,
            311, 311, 311, 311, 311, 311, 311, 311, 311, 311, 311, 311, 311, 311, 311, 311, 311,
            308, 311, 311, 308, 311, 311, 308, 308, 311, 311, 311, 311, 311, 308, 311, 311, 311,
            311, 311, 308, 311, 311, 311, 311, 311, 308, 308, 311, 311, 311, 311, 311, 308, 311,
            311, 311, 311, 311, 308, 311, 311, 311, 311, 311, 308, 308, 311, 311, 308, 311, 311,
            308, 311, 311, 308, 308, 308, 308, 308, 308, 308, 308, 308, 308, 308, 308, 308, 308,
            308, 308, 308, 308, 308, 308, 308, 308, 308, 308, 308, 308, 308, 308, 308, 308, 308,
            308, 332, 335, 332, 331, 330, 329, 328, 327, 326, 325, 324, 323, 322, 321, 320, 319,
            318, 317, 316, 315, 314, 313, 312, 311, 310, 309, 308, 307, 308, 308, 308, 308, 308,
            308, 308, 344, 368, 374, 368, 367, 366, 365, 364, 363, 362, 361, 360, 359, 358, 357,
            356, 355, 354, 353, 352, 351, 350, 349, 348, 347, 346, 345, 344, 343, 344, 344, 344,
            344, 344, 344, 344, 344, 343, 344, 338, 337, 336, 335, 334, 333, 332, 331, 330, 329,
            328, 327, 326, 325, 324, 323, 322, 321, 320, 319, 318, 317, 316, 315, 314, 313, 312,
            311, 310, 309, 308, 307, 308, 308, 308, 308, 308, 344, 368, 374, 368, 367, 366, 365,
            364, 363, 362, 361, 360, 359, 358, 357, 356, 355, 354, 353, 352, 351, 350, 349, 348,
            347, 346, 345, 344, 343, 344, 344, 344, 344, 344, 344, 344, 344, 343, 344, 338, 337,
            336, 335, 334, 333, 332, 331, 330, 329, 328, 327, 326, 325, 324, 323, 322, 321, 320,
            319, 318, 317, 316, 315, 314, 313, 312, 311, 310, 309, 308, 307, 308, 308, 308, 308,
            308, 344, 368, 374, 368, 367, 366, 365, 364, 363, 362, 361, 360, 359, 358, 357, 356,
            355, 354, 353, 352, 351, 350, 349, 348, 347, 346, 345, 344, 343, 344, 344, 344, 344,
            344, 344, 344, 344, 343, 344, 338, 337, 336, 335, 334, 333, 332, 331, 330, 329, 328,
            327, 326, 325, 324, 323, 322, 321, 320, 319, 318, 317, 316, 315, 314, 313, 312, 311,
            310, 309, 308, 307, 308, 308, 308, 308, 342, 366, 372, 366, 365, 364, 363, 362, 361,
            360, 359, 358, 357, 356, 355, 354, 353, 352, 351, 350, 349, 348, 347, 346, 345, 344,
            343, 342, 341, 342, 342, 342, 342, 342, 342, 341, 342, 338, 337, 336, 335, 334, 333,
            332, 331, 330, 329, 328, 327, 326, 325, 324, 323, 322, 321, 320, 319, 318, 317, 316,
            315, 314, 313, 312, 311, 310, 309, 308, 307, 308, 308, 308, 308, 308, 344, 368, 374,
            368, 367, 366, 365, 364, 363, 362, 361, 360, 359, 358, 357, 356, 355, 354, 353, 352,
            351, 350, 349, 348, 347, 346, 345, 344, 343, 344, 344, 344, 344, 344, 344, 344, 344,
            343, 344, 338, 337, 336, 335, 334, 333, 332, 331, 330, 329, 328, 327, 326, 325, 324,
            323, 322, 321, 320, 319, 318, 317, 316, 315, 314, 313, 312, 311, 310, 309, 308, 307,
            308, 308, 308, 308, 308, 344, 368, 374, 368, 367, 366, 365, 364, 363, 362, 361, 360,
            359, 358, 357, 356, 355, 354, 353, 352, 351, 350, 349, 348, 347, 346, 345, 344, 343,
            344, 344, 344, 344, 344, 344, 344, 344, 343, 344, 338, 337, 336, 335, 334, 333, 332,
            331, 330, 329, 328, 327, 326, 325, 324, 323, 322, 321, 320, 319, 318, 317, 316, 315,
            314, 313, 312, 311, 310, 309, 308, 307, 308, 308, 311, 311, 311, 311, 344, 368, 374,
            368, 367, 366, 365, 364, 363, 362, 361, 360, 359, 358, 357, 356, 355, 354, 353, 352,
            351, 350, 349, 348, 347, 346, 345, 344, 343, 344, 344, 344, 343, 342, 341, 340, 339,
            338, 337, 338, 338, 338, 338, 338, 338, 337, 338, 336, 335, 334, 333, 332, 331, 330,
            329, 328, 327, 326, 325, 324, 323, 322, 321, 320, 319, 318, 317, 316, 315, 314, 313,
            312, 311, 310, 309, 308, 307, 306, 305, 304, 303, 302, 303, 303, 303, 303, 303, 339,
            363, 369, 363, 362, 361, 360, 359, 358, 357, 356, 355, 354, 353, 352, 351, 350, 349,
            348, 347, 346, 345, 344, 343, 342, 341, 340, 339, 338, 339, 339, 339, 339, 339, 339,
            339, 339, 338, 339, 333, 332, 331, 330, 329, 328, 327, 326, 325, 324, 323, 322, 321,
            320, 319, 318, 317, 316, 315, 314, 313, 312, 311, 310, 309, 308, 307, 306, 305, 304,
            303, 302, 303, 303, 303, 303, 303, 339, 363, 369, 363, 362, 361, 360, 359, 358, 357,
            356, 355, 354, 353, 352, 351, 350, 349, 348, 347, 346, 345, 344, 343, 342, 341, 340,
            339, 338, 339, 339, 339, 339, 339, 339, 339, 339, 338, 339, 333, 332, 331, 330, 329,
            328, 327, 326, 325, 324, 323, 322, 321, 320, 319, 318, 317, 316, 315, 314, 313, 312,
            311, 310, 309, 308, 307, 306, 305, 304, 303, 302, 303, 303, 303, 303, 303, 337, 361,
            367, 361, 360, 359, 358, 357, 356, 355, 354, 353, 352, 351, 350, 349, 348, 347, 346,
            345, 344, 343, 342, 341, 340, 339, 338, 337, 336, 337, 337, 337, 337, 337, 337, 336,
            337, 333, 332, 331, 330, 329, 328, 327, 326, 325, 324, 323, 322, 321, 320, 319, 318,
            317, 316, 315, 314, 313, 312, 311, 310, 309, 308, 307, 306, 305, 304, 303, 302, 303,
            303, 303, 303, 303, 337, 361, 367, 361, 360, 359, 358, 357, 356, 355, 354, 353, 352,
            351, 350, 349, 348, 347, 346, 345, 344, 343, 342, 341, 340, 339, 338, 337, 336, 337,
            337, 337, 337, 337, 337, 336, 337, 333, 332, 331, 330, 329, 328, 327, 326, 325, 324,
            323, 322, 321, 320, 319, 318, 317, 316, 315, 314, 313, 312, 311, 310, 309, 308, 307,
            306, 305, 304, 303, 302, 303, 303, 303, 303, 303, 337, 361, 367, 361, 360, 359, 358,
            357, 356, 355, 354, 353, 352, 351, 350, 349, 348, 347, 346, 345, 344, 343, 342, 341,
            340, 339, 338, 337, 336, 337, 337, 337, 337, 337, 337, 336, 337, 333, 332, 331, 330,
            329, 328, 327, 326, 325, 324, 323, 322, 321, 320, 319, 318, 317, 316, 315, 314, 313,
            312, 311, 310, 309, 308, 307, 306, 305, 304, 303, 302, 303, 303, 303, 303, 303, 303,
            336, 360, 366, 360, 359, 358, 357, 356, 355, 354, 353, 352, 351, 350, 349, 348, 347,
            346, 345, 344, 343, 342, 341, 340, 339, 338, 337, 336, 335, 336, 336, 336, 336, 336,
            336, 336, 336, 335, 336, 333, 332, 331, 330, 329, 328, 327, 326, 325, 324, 323, 322,
            321, 320, 319, 318, 317, 316, 315, 314, 313, 312, 311, 310, 309, 308, 307, 306, 305,
            304, 303, 302, 303, 303, 303, 303, 303, 339, 363, 369, 363, 362, 361, 360, 359, 358,
            357, 356, 355, 354, 353, 352, 351, 350, 349, 348, 347, 346, 345, 344, 343, 342, 341,
            340, 339, 338, 339, 339, 339, 339, 339, 339, 339, 339, 338, 339, 333, 332, 331, 330,
            329, 328, 327, 326, 325, 324, 323, 322, 321, 320, 319, 318, 317, 316, 315, 314, 313,
            312, 311, 310, 309, 308, 307, 306, 305, 304, 303, 302, 303, 303, 303, 303, 303, 339,
            363, 369, 363, 362, 361, 360, 359, 358, 357, 356, 355, 354, 353, 352, 351, 350, 349,
            348, 347, 346, 345, 344, 343, 342, 341, 340, 339, 338, 339, 339, 339, 339, 339, 339,
            339, 339, 338, 339, 333, 332, 331, 330, 329, 328, 327, 326, 325, 324, 323, 322, 321,
            320, 319, 318, 317, 316, 315, 314, 313, 312, 311, 310, 309, 308, 307, 306, 305, 304,
            303, 302, 303, 303, 303, 303, 303, 339, 363, 369, 363, 362, 361, 360, 359, 358, 357,
            356, 355, 354, 353, 352, 351, 350, 349, 348, 347, 346, 345, 344, 343, 342, 341, 340,
            339, 338, 339, 339, 339, 339, 339, 339, 339, 339, 338, 339, 333, 332, 331, 330, 329,
            328, 327, 326, 325, 324, 323, 322, 321, 320, 319, 318, 317, 316, 315, 314, 313, 312,
            311, 310, 309, 308, 307, 306, 305, 304, 303, 302, 303, 303, 303, 302, 301, 300, 299,
            298, 297, 296, 295, 294, 293, 292, 291, 290, 289, 288, 287, 286, 285, 284, 283, 282,
            281, 280, 279, 278, 277, 276, 275, 274, 273, 272, 271, 270, 269, 268, 267, 266, 267,
            270, 273, 276, 276, 276, 276, 276, 275, 274, 273, 272, 271, 270, 269, 268, 267, 266,
            267, 294, 294, 291, 290, 289, 288, 287, 286, 285, 284, 283, 282, 281, 280, 279, 278,
            277, 276, 275, 274, 273, 272, 271, 270, 269, 268, 267, 266, 267, 270, 273, 276, 276,
            275, 274, 273, 272, 271, 270, 269, 268, 267, 266, 267, 315, 315, 318, 345, 348, 348,
            345, 344, 342, 342, 342, 324, 323, 322, 321, 320, 319, 318, 317, 316, 315, 314, 313,
            312, 311, 310, 309, 308, 307, 306, 305, 304, 303, 302, 301, 300, 299, 298, 297, 296,
            295, 294, 293, 292, 291, 290, 289, 288, 287, 286, 285, 284, 283, 282, 281, 280, 279,
            278, 277, 276, 275, 274, 273, 272, 271, 270, 269, 270, 270,
        ],
        &[
            272, 272, 272, 272, 272, 272, 272, 272, 272, 305, 305, 305, 305, 305, 305, 305, 305,
            305, 305, 305, 305, 305, 305, 305, 305, 305, 305, 305, 305, 305, 305, 305, 305, 305,
            305, 305, 305, 305, 305, 305, 305, 305, 305, 305, 305, 305, 305, 305, 308, 311, 308,
            311, 308, 311, 308, 308, 311, 311, 308, 311, 311, 308, 311, 311, 308, 308, 311, 311,
            311, 308, 311, 311, 311, 308, 311, 311, 311, 308, 308, 311, 311, 311, 308, 311, 311,
            311, 308, 308, 311, 308, 311, 308, 311, 308, 308, 311, 311, 308, 311, 311, 308, 311,
            311, 308, 308, 311, 311, 308, 311, 311, 308, 311, 311, 308, 308, 311, 311, 311, 308,
            311, 311, 311, 308, 311, 311, 311, 308, 308, 311, 311, 311, 311, 308, 311, 311, 311,
            308, 311, 311, 311, 308, 308, 311, 311, 311, 308, 311, 311, 308, 311, 311, 308, 311,
            311, 311, 311, 308, 311, 311, 308, 311, 311, 308, 311, 311, 311, 311, 311, 311, 311,
            311, 311, 311, 311, 311, 311, 311, 311, 311, 311, 311, 311, 311, 311, 311, 311, 311,
            311, 311, 311, 311, 311, 311, 311, 311, 311, 311, 311, 311, 311, 308, 311, 311, 308,
            311, 311, 308, 308, 311, 311, 311, 311, 311, 308, 311, 311, 311, 311, 311, 308, 311,
            311, 311, 311, 311, 308, 308, 311, 311, 311, 311, 311, 308, 311, 311, 311, 311, 311,
            308, 311, 311, 311, 311, 311, 308, 308, 311, 311, 308, 311, 311, 308, 311, 311, 308,
            308, 308, 308, 308, 308, 308, 308, 308, 308, 308, 308, 308, 308, 308, 308, 308, 308,
            308, 308, 308, 308, 308, 308, 308, 308, 308, 308, 308, 308, 308, 308, 332, 335, 332,
            331, 330, 329, 328, 327, 326, 325, 324, 323, 322, 321, 320, 319, 318, 317, 316, 315,
            314, 313, 312, 311, 310, 309, 308, 307, 308, 308, 308, 308, 308, 308, 308, 344, 368,
            374, 368, 367, 366, 365, 364, 363, 362, 361, 360, 359, 358, 357, 356, 355, 354, 353,
            352, 351, 350, 349, 348, 347, 346, 345, 344, 343, 344, 344, 344, 344, 344, 344, 344,
            344, 343, 344, 338, 337, 336, 335, 334, 333, 332, 331, 330, 329, 328, 327, 326, 325,
            324, 323, 322, 321, 320, 319, 318, 317, 316, 315, 314, 313, 312, 311, 310, 309, 308,
            307, 308, 308, 308, 308, 308, 344, 368, 374, 368, 367, 366, 365, 364, 363, 362, 361,
            360, 359, 358, 357, 356, 355, 354, 353, 352, 351, 350, 349, 348, 347, 346, 345, 344,
            343, 344, 344, 344, 344, 344, 344, 344, 344, 343, 344, 338, 337, 336, 335, 334, 333,
            332, 331, 330, 329, 328, 327, 326, 325, 324, 323, 322, 321, 320, 319, 318, 317, 316,
            315, 314, 313, 312, 311, 310, 309, 308, 307, 308, 308, 308, 308, 308, 344, 368, 374,
            368, 367, 366, 365, 364, 363, 362, 361, 360, 359, 358, 357, 356, 355, 354, 353, 352,
            351, 350, 349, 348, 347, 346, 345, 344, 343, 344, 344, 344, 344, 344, 344, 344, 344,
            343, 344, 338, 337, 336, 335, 334, 333, 332, 331, 330, 329, 328, 327, 326, 325, 324,
            323, 322, 321, 320, 319, 318, 317, 316, 315, 314, 313, 312, 311, 310, 309, 308, 307,
            308, 308, 308, 308, 342, 366, 372, 366, 365, 364, 363, 362, 361, 360, 359, 358, 357,
            356, 355, 354, 353, 352, 351, 350, 349, 348, 347, 346, 345, 344, 343, 342, 341, 342,
            342, 342, 342, 342, 342, 341, 342, 338, 337, 336, 335, 334, 333, 332, 331, 330, 329,
            328, 327, 326, 325, 324, 323, 322, 321, 320, 319, 318, 317, 316, 315, 314, 313, 312,
            311, 310, 309, 308, 307, 308, 308, 308, 308, 308, 344, 368, 374, 368, 367, 366, 365,
            364, 363, 362, 361, 360, 359, 358, 357, 356, 355, 354, 353, 352, 351, 350, 349, 348,
            347, 346, 345, 344, 343, 344, 344, 344, 344, 344, 344, 344, 344, 343, 344, 338, 337,
            336, 335, 334, 333, 332, 331, 330, 329, 328, 327, 326, 325, 324, 323, 322, 321, 320,
            319, 318, 317, 316, 315, 314, 313, 312, 311, 310, 309, 308, 307, 308, 308, 308, 308,
            308, 344, 368, 374, 368, 367, 366, 365, 364, 363, 362, 361, 360, 359, 358, 357, 356,
            355, 354, 353, 352, 351, 350, 349, 348, 347, 346, 345, 344, 343, 344, 344, 344, 344,
            344, 344, 344, 344, 343, 344, 338, 337, 336, 335, 334, 333, 332, 331, 330, 329, 328,
            327, 326, 325, 324, 323, 322, 321, 320, 319, 318, 317, 316, 315, 314, 313, 312, 311,
            310, 309, 308, 307, 308, 308, 308, 308, 308, 341, 365, 371, 365, 364, 363, 362, 361,
            360, 359, 358, 357, 356, 355, 354, 353, 352, 351, 350, 349, 348, 347, 346, 345, 344,
            343, 342, 341, 340, 341, 341, 341, 341, 341, 341, 341, 341, 340, 341, 338, 337, 336,
            335, 334, 333, 332, 331, 330, 329, 328, 327, 326, 325, 324, 323, 322, 321, 320, 319,
            318, 317, 316, 315, 314, 313, 312, 311, 310, 309, 308, 307, 308, 308, 308, 308, 308,
            344, 368, 374, 368, 367, 366, 365, 364, 363, 362, 361, 360, 359, 358, 357, 356, 355,
            354, 353, 352, 351, 350, 349, 348, 347, 346, 345, 344, 343, 344, 344, 344, 344, 344,
            344, 344, 344, 343, 344, 338, 337, 336, 335, 334, 333, 332, 331, 330, 329, 328, 327,
            326, 325, 324, 323, 322, 321, 320, 319, 318, 317, 316, 315, 314, 313, 312, 311, 310,
            309, 308, 307, 308, 308, 308, 308, 308, 344, 368, 374, 368, 367, 366, 365, 364, 363,
            362, 361, 360, 359, 358, 357, 356, 355, 354, 353, 352, 351, 350, 349, 348, 347, 346,
            345, 344, 343, 344, 344, 344, 344, 344, 344, 344, 344, 343, 344, 338, 337, 336, 335,
            334, 333, 332, 331, 330, 329, 328, 327, 326, 325, 324, 323, 322, 321, 320, 319, 318,
            317, 316, 315, 314, 313, 312, 311, 310, 309, 308, 307, 308, 308, 308, 311, 311, 311,
            345, 369, 375, 369, 368, 367, 366, 365, 364, 363, 362, 361, 360, 359, 358, 357, 356,
            355, 354, 353, 352, 351, 350, 349, 348, 347, 346, 345, 344, 345, 345, 345, 344, 343,
            342, 341, 342, 342, 342, 342, 341, 342, 340, 339, 338, 337, 336, 335, 334, 333, 332,
            331, 330, 329, 328, 327, 326, 325, 324, 323, 322, 321, 320, 319, 318, 317, 316, 315,
            314, 313, 312, 311, 310, 309, 308, 307, 306, 307, 307, 307, 307, 307, 341, 365, 371,
            365, 364, 363, 362, 361, 360, 359, 358, 357, 356, 355, 354, 353, 352, 351, 350, 349,
            348, 347, 346, 345, 344, 343, 342, 341, 340, 341, 341, 341, 341, 341, 341, 340, 341,
            337, 336, 335, 334, 333, 332, 331, 330, 329, 328, 327, 326, 325, 324, 323, 322, 321,
            320, 319, 318, 317, 316, 315, 314, 313, 312, 311, 310, 309, 308, 307, 306, 307, 307,
            307, 307, 307, 341, 365, 371, 365, 364, 363, 362, 361, 360, 359, 358, 357, 356, 355,
            354, 353, 352, 351, 350, 349, 348, 347, 346, 345, 344, 343, 342, 341, 340, 341, 341,
            341, 341, 341, 341, 340, 341, 337, 336, 335, 334, 333, 332, 331, 330, 329, 328, 327,
            326, 325, 324, 323, 322, 321, 320, 319, 318, 317, 316, 315, 314, 313, 312, 311, 310,
            309, 308, 307, 306, 307, 307, 307, 307, 307, 307, 340, 364, 370, 364, 363, 362, 361,
            360, 359, 358, 357, 356, 355, 354, 353, 352, 351, 350, 349, 348, 347, 346, 345, 344,
            343, 342, 341, 340, 339, 340, 340, 340, 340, 340, 340, 340, 340, 339, 340, 337, 336,
            335, 334, 333, 332, 331, 330, 329, 328, 327, 326, 325, 324, 323, 322, 321, 320, 319,
            318, 317, 316, 315, 314, 313, 312, 311, 310, 309, 308, 307, 306, 307, 307, 307, 307,
            307, 343, 367, 373, 367, 366, 365, 364, 363, 362, 361, 360, 359, 358, 357, 356, 355,
            354, 353, 352, 351, 350, 349, 348, 347, 346, 345, 344, 343, 342, 343, 343, 343, 343,
            343, 343, 343, 343, 342, 343, 337, 336, 335, 334, 333, 332, 331, 330, 329, 328, 327,
            326, 325, 324, 323, 322, 321, 320, 319, 318, 317, 316, 315, 314, 313, 312, 311, 310,
            309, 308, 307, 306, 307, 307, 307, 307, 307, 343, 367, 373, 367, 366, 365, 364, 363,
            362, 361, 360, 359, 358, 357, 356, 355, 354, 353, 352, 351, 350, 349, 348, 347, 346,
            345, 344, 343, 342, 343, 343, 343, 343, 343, 343, 343, 343, 342, 343, 337, 336, 335,
            334, 333, 332, 331, 330, 329, 328, 327, 326, 325, 324, 323, 322, 321, 320, 319, 318,
            317, 316, 315, 314, 313, 312, 311, 310, 309, 308, 307, 306, 307, 307, 307, 307, 307,
            343, 367, 373, 367, 366, 365, 364, 363, 362, 361, 360, 359, 358, 357, 356, 355, 354,
            353, 352, 351, 350, 349, 348, 347, 346, 345, 344, 343, 342, 343, 343, 343, 343, 343,
            343, 343, 343, 342, 343, 337, 336, 335, 334, 333, 332, 331, 330, 329, 328, 327, 326,
            325, 324, 323, 322, 321, 320, 319, 318, 317, 316, 315, 314, 313, 312, 311, 310, 309,
            308, 307, 306, 307, 307, 307, 306, 305, 304, 303, 302, 301, 300, 299, 298, 297, 296,
            295, 294, 293, 292, 291, 290, 289, 288, 287, 286, 285, 284, 283, 282, 281, 280, 279,
            278, 277, 276, 275, 274, 273, 272, 271, 270, 271, 274, 277, 280, 280, 280, 280, 280,
            279, 278, 277, 276, 275, 274, 273, 272, 271, 270, 271, 298, 298, 295, 294, 293, 292,
            291, 290, 289, 288, 287, 286, 285, 284, 283, 282, 281, 280, 279, 278, 277, 276, 275,
            274, 273, 272, 271, 270, 271, 274, 277, 280, 280, 279, 278, 277, 276, 275, 274, 273,
            272, 271, 270, 271, 319, 319, 322, 349, 352, 352, 349, 348, 346, 346, 346, 328, 327,
            326, 325, 324, 323, 322, 321, 320, 319, 318, 317, 316, 315, 314, 313, 312, 311, 310,
            309, 308, 307, 306, 305, 304, 303, 302, 301, 300, 299, 298, 297, 296, 295, 294, 293,
            292, 291, 290, 289, 288, 287, 286, 285, 284, 283, 282, 281, 280, 279, 278, 277, 276,
            275, 274, 273, 274, 274,
        ],
    ];
    const REMOVAL_DEADLINES: &[&[(&str, usize)]] = &[
        &[
            ("registry-0-source", 687),
            ("registry-1-source", 763),
            ("registry-2-source", 839),
            ("registry-3-source", 918),
            ("registry-4-source", 997),
            ("registry-5-source", 1073),
            ("registry-6-source", 1149),
            ("registry-7-source", 1225),
            ("registry-8-source", 1307),
            ("hulls-0-source", 1384),
            ("hulls-1-source", 1458),
            ("hulls-2-source", 1544),
            ("registry-9-32-path", 1662),
            ("registry-9-source", 1670),
            ("registry-10-source", 1749),
            ("registry-11-source", 1825),
            ("registry-12-source", 1913),
            ("registry-13-source", 1995),
            ("registry-14-source", 2071),
            ("registry-15-source", 2165),
            ("registry-16-source", 2268),
            ("registry-17-source", 2371),
            ("registry-18-source", 2456),
            ("registry-19-source", 2532),
            ("collector", 2730),
        ],
        &[
            ("registry-0-source", 731),
            ("registry-1-source", 816),
            ("registry-2-source", 901),
            ("registry-3-source", 989),
            ("registry-4-source", 1071),
            ("registry-5-source", 1153),
            ("registry-6-73-reasons", 1231),
            ("registry-6-source", 1241),
            ("registry-7-source", 1326),
            ("registry-8-source", 1411),
            ("hulls-0-source", 1500),
            ("hulls-1-source", 1592),
            ("hulls-2-source", 1684),
            ("registry-9-31-path", 1776),
            ("registry-9-32-path", 1820),
            ("registry-9-source", 1828),
            ("registry-10-source", 1919),
            ("registry-11-source", 1998),
            ("registry-12-source", 2092),
            ("registry-13-1-reasons", 2176),
            ("registry-13-source", 2186),
            ("registry-14-4-facilities", 2264),
            ("registry-14-4-memhulls", 2267),
            ("registry-14-source", 2277),
            ("registry-15-source", 2374),
            ("registry-16-source", 2477),
            ("registry-17-source", 2580),
            ("registry-18-source", 2671),
            ("registry-19-source", 2756),
            ("collector", 3084),
        ],
        &[
            ("registry-0-source", 552),
            ("registry-1-source", 637),
            ("registry-2-source", 722),
            ("registry-3-source", 810),
            ("registry-4-source", 892),
            ("registry-5-source", 974),
            ("registry-6-74-reasons", 1053),
            ("registry-6-source", 1061),
            ("registry-7-source", 1146),
            ("registry-8-source", 1243),
            ("hulls-0-source", 1326),
            ("hulls-1-source", 1412),
            ("hulls-2-source", 1504),
            ("registry-9-31-path", 1588),
            ("registry-9-32-path", 1632),
            ("registry-9-source", 1640),
            ("registry-10-source", 1728),
            ("registry-11-source", 1825),
            ("registry-12-source", 1928),
            ("registry-13-2-reasons", 2013),
            ("registry-13-source", 2021),
            ("registry-14-5-facilities", 2100),
            ("registry-14-5-memhulls", 2103),
            ("registry-14-source", 2111),
            ("registry-15-source", 2208),
            ("registry-16-source", 2311),
            ("registry-17-source", 2414),
            ("registry-18-source", 2505),
            ("registry-19-source", 2590),
            ("collector", 2936),
        ],
        &[
            ("registry-0-source", 676),
            ("registry-1-source", 761),
            ("registry-2-source", 846),
            ("registry-3-source", 922),
            ("registry-4-source", 998),
            ("registry-5-source", 1080),
            ("registry-6-75-reasons", 1163),
            ("registry-6-source", 1169),
            ("registry-7-source", 1254),
            ("registry-8-source", 1339),
            ("hulls-0-source", 1416),
            ("hulls-1-source", 1490),
            ("hulls-2-source", 1576),
            ("registry-9-32-path", 1694),
            ("registry-9-source", 1702),
            ("registry-10-source", 1799),
            ("registry-11-source", 1902),
            ("registry-12-source", 1987),
            ("registry-13-3-reasons", 2067),
            ("registry-13-source", 2073),
            ("registry-14-6-facilities", 2155),
            ("registry-14-6-memhulls", 2158),
            ("registry-14-source", 2164),
            ("registry-15-source", 2243),
            ("registry-16-source", 2319),
            ("registry-17-source", 2395),
            ("registry-18-source", 2477),
            ("registry-19-source", 2562),
            ("collector", 2836),
        ],
        &[
            ("registry-0-source", 345),
            ("registry-1-source", 421),
            ("registry-2-source", 497),
            ("registry-3-source", 570),
            ("registry-4-source", 646),
            ("registry-5-source", 722),
            ("registry-6-73-reasons", 797),
            ("registry-6-source", 807),
            ("registry-7-source", 886),
            ("registry-8-source", 962),
            ("hulls-0-source", 1036),
            ("hulls-1-source", 1110),
            ("hulls-2-source", 1184),
            ("registry-9-source", 1261),
            ("registry-10-source", 1337),
            ("registry-11-source", 1413),
            ("registry-12-source", 1489),
            ("collector", 1633),
        ],
        &[
            ("registry-0-source", 358),
            ("registry-1-source", 434),
            ("registry-2-source", 510),
            ("registry-3-source", 583),
            ("registry-4-source", 659),
            ("registry-5-source", 735),
            ("registry-6-source", 811),
            ("registry-7-source", 887),
            ("registry-8-source", 963),
            ("hulls-0-source", 1043),
            ("hulls-1-source", 1120),
            ("hulls-2-source", 1194),
            ("registry-9-source", 1271),
            ("registry-10-source", 1347),
            ("registry-11-source", 1423),
            ("registry-12-source", 1499),
            ("collector", 1643),
        ],
    ];
    fn removal_original_names(owner: &LivingQueuedCreatorJobV2) -> Vec<(usize, String)> {
        let s = &owner.state;
        let mut names = vec![
            (
                s.systems.as_slice().as_ptr() as usize,
                "registry-0-source".into(),
            ),
            (
                s.stars.as_slice().as_ptr() as usize,
                "registry-1-source".into(),
            ),
            (
                s.worlds.as_slice().as_ptr() as usize,
                "registry-2-source".into(),
            ),
            (
                s.lanes.as_slice().as_ptr() as usize,
                "registry-3-source".into(),
            ),
            (
                s.civilizations.as_slice().as_ptr() as usize,
                "registry-4-source".into(),
            ),
            (
                s.deposits.as_slice().as_ptr() as usize,
                "registry-5-source".into(),
            ),
            (
                s.facilities.as_slice().as_ptr() as usize,
                "registry-6-source".into(),
            ),
            (
                s.construction_jobs.as_slice().as_ptr() as usize,
                "registry-7-source".into(),
            ),
            (
                s.hull_jobs.as_slice().as_ptr() as usize,
                "registry-8-source".into(),
            ),
            (
                s.fleets.as_slice().as_ptr() as usize,
                "registry-9-source".into(),
            ),
            (
                s.routes.as_slice().as_ptr() as usize,
                "registry-10-source".into(),
            ),
            (
                s.shipments.as_slice().as_ptr() as usize,
                "registry-11-source".into(),
            ),
            (
                s.hazards.as_slice().as_ptr() as usize,
                "registry-12-source".into(),
            ),
            (
                s.colonies.as_slice().as_ptr() as usize,
                "registry-13-source".into(),
            ),
            (
                s.observations.as_slice().as_ptr() as usize,
                "registry-14-source".into(),
            ),
            (
                s.relations.as_slice().as_ptr() as usize,
                "registry-15-source".into(),
            ),
            (
                s.agreements.as_slice().as_ptr() as usize,
                "registry-16-source".into(),
            ),
            (
                s.wars.as_slice().as_ptr() as usize,
                "registry-17-source".into(),
            ),
            (
                s.settlement_claims.as_slice().as_ptr() as usize,
                "registry-18-source".into(),
            ),
            (
                s.occupations.as_slice().as_ptr() as usize,
                "registry-19-source".into(),
            ),
        ];

        for (i, row) in s.fleets.as_slice().iter().enumerate() {
            names.push((
                row.hulls.as_slice().as_ptr() as usize,
                format!("hulls-{i}-source"),
            ));
            if let Some(order) = &row.order {
                names.push((
                    order.lane_path.as_ptr() as usize,
                    format!("registry-9-{}-path", row.id.0[0]),
                ));
            }
        }
        for row in s.facilities.as_slice() {
            names.push((
                row.blocked_reasons.as_slice().as_ptr() as usize,
                format!("registry-6-{}-reasons", row.id.0[0]),
            ));
        }
        for row in s.colonies.as_slice() {
            names.push((
                row.hub.blocked_reasons.as_slice().as_ptr() as usize,
                format!("registry-13-{}-reasons", row.world.0[0]),
            ));
        }
        for row in s.observations.as_slice() {
            names.push((
                row.facilities.as_slice().as_ptr() as usize,
                format!("registry-14-{}-facilities", row.civilization.0[0]),
            ));
            names.push((
                row.stationed_hulls.as_slice().as_ptr() as usize,
                format!("registry-14-{}-memhulls", row.civilization.0[0]),
            ));
        }
        names.push((
            owner.events.payloads().as_ptr() as usize,
            "collector".into(),
        ));
        names
    }
    #[test]
    fn reviewed_removal_every_stage_refusal_retains_original_owners() {
        use crate::living::simulation::removal_creator_tests as removal;
        for kind in 0..3 {
            let authority = removal::admitted_reviewed_source(kind);
            let make_creator = |a: &LivingGalaxyAuthorityV2| {
                Box::new(LivingQueuedCreatorJobV2::begin(
                    a.state.as_ref().clone(),
                    Arc::clone(&a.queue),
                    Arc::clone(&a.catalog),
                ))
            };
            let mut scan = make_creator(&authority);
            let mut cuts = Vec::new();
            let mut seen = std::collections::BTreeSet::new();
            for cut in 0..100000 {
                if let Some(QueuedCreatorInsertion::Removal(work)) = &scan.insertion {
                    // Every actual witness-validation visit, plus every surrounding
                    // stage/registry. Existing legacy tests cover all filter cuts.
                    let first =
                        seen.insert((work.stage as usize, work.registry, work.witnesses.len()));
                    if first
                        || matches!(
                            work.stage,
                            QueuedRemovalStage::ReviewedStable
                                | QueuedRemovalStage::ReviewedDerived
                                | QueuedRemovalStage::ReviewedCompare
                        )
                    {
                        cuts.push(cut);
                    }
                }
                if scan.poll(1).unwrap() != LivingStepProgressV2::Pending {
                    break;
                }
                assert!(cut < 99999);
            }
            assert!(
                seen.iter()
                    .any(|(stage, _, _)| *stage == QueuedRemovalStage::ReviewedStable as usize)
            );
            assert!(
                seen.iter()
                    .any(|(stage, _, _)| *stage == QueuedRemovalStage::ReviewedDerived as usize)
            );
            assert!(
                seen.iter()
                    .any(|(stage, _, _)| *stage == QueuedRemovalStage::ReviewedCompare as usize)
            );
            for cut in cuts {
                for stale in [false, true] {
                    let mut counts = Vec::new();
                    for schedule in SCHEDULES {
                        let mut anchor = removal::admitted_reviewed_source(kind);
                        let mut step = captured_step(
                            make_creator(&anchor),
                            anchor.capture_boundary().unwrap(),
                        );
                        let creator = step.creator.as_mut().unwrap();
                        let pointer = &**creator as *const _;
                        for _ in 0..cut {
                            assert_eq!(creator.poll(1).unwrap(), LivingStepProgressV2::Pending);
                        }
                        let current = allocations(creator);
                        let child_pointer = match &creator.insertion {
                            Some(QueuedCreatorInsertion::Removal(work)) => work as *const _,
                            _ => panic!("original removal"),
                        };
                        if stale {
                            let c = anchor.published_cursor();
                            anchor
                                .submit(LivingCommandEnvelopeV2 {
                                    expected_committed_revision: c.committed_revision,
                                    expected_tick: c.tick,
                                    expected_pending_sequence: c.pending_sequence,
                                    mode: LivingCommandModeV2::Running,
                                    command: LivingCommandV2::CreatorBatch {
                                        operations: vec![LivingCreatorOperationV2::CreateSystem {
                                            local: 0,
                                            name: crate::living::LivingNameV2::new(
                                                "Stale reviewed tail",
                                            )
                                            .unwrap(),
                                        }],
                                    },
                                })
                                .unwrap();
                        } else {
                            step.cancel();
                        }
                        let accepted = Arc::clone(&anchor.queue);
                        let state = Arc::clone(&anchor.state);
                        let cursor = anchor.published_cursor();
                        assert!(step.take_synchronous_preparation(&anchor).is_err());
                        assert!(std::ptr::eq(&**step.creator.as_ref().unwrap(), pointer));
                        let Some(QueuedCreatorInsertion::Removal(work)) =
                            &step.creator.as_ref().unwrap().insertion
                        else {
                            unreachable!()
                        };
                        assert!(std::ptr::eq(work, child_pointer));
                        assert_eq!(allocations(step.creator.as_ref().unwrap()), current);
                        assert!(
                            Arc::ptr_eq(&anchor.queue, &accepted)
                                && Arc::ptr_eq(&anchor.state, &state)
                        );
                        assert_eq!(anchor.published_cursor(), cursor);
                        counts.push(drain_observed(
                            step.into_boxed_retirement(
                                LivingPublicationRetirementPolicyV2::ReleaseHandle,
                            ),
                            schedule,
                            &current,
                            (kind, stale, cut),
                        ));
                    }
                    assert!(counts.iter().all(|n| *n == counts[0]));
                }
            }
        }
    }
    #[test]
    fn reviewed_removal_witness_tree_has_independent_counted_retirement() {
        use crate::living::LivingDerivedRemovalV2 as D;
        for n in [0, 1, 7, 65] {
            let mut work = QueuedRemovalWork::begin();
            work.reviewed = true;
            for id in 0..n {
                work.witnesses.insert(D::Order {
                    fleet: LivingEntityIdV2([id; 16]),
                });
            }
            let mut original = work
                .retirement_child(8, LivingStateRetirementPolicyV2::ReleaseHandle)
                .unwrap();
            assert!(work.witnesses.is_empty());
            let mut count = 0;
            let mut done = false;
            for _ in 0..1000 {
                match original.retirement_step() {
                    RetirementStep::Work => count += 1,
                    RetirementStep::Done => {
                        done = true;
                        break;
                    }
                    RetirementStep::Deferred(_) => panic!("unshared witnesses"),
                }
            }
            assert!(done);
            // Each scalar witness: select, release payload, release child Box.
            // The exhausted BTree root has its own final release visit.
            assert_eq!(count, 3 * u64::from(n) + 1);
            let mut legacy = QueuedRemovalWork::begin();
            assert!(
                legacy
                    .retirement_child(8, LivingStateRetirementPolicyV2::ReleaseHandle)
                    .is_none()
            );
        }
    }
    #[test]
    fn removal_creator_every_natural_cut_original_lifetimes_and_owner_arithmetic() {
        use crate::living::simulation::removal_creator_tests as removal;
        for kind in 0..6 {
            for (cut, &expected) in REMOVAL_CUTS[kind].iter().enumerate() {
                for schedule in SCHEDULES {
                    let (creator, queue, catalog) = removal::source(kind);
                    let anchor = crate::living::builtin_living_sandbox_v2().unwrap();
                    let mut step = captured_step(creator, anchor.capture_boundary().unwrap());
                    let step_pointer = &*step as *const _;
                    let creator = step.creator.as_mut().unwrap();
                    let pointer = &**creator as *const _;
                    let names = removal_original_names(creator);
                    let probes: Vec<_> = allocations(creator)
                        .into_iter()
                        .map(|probe| {
                            let name = names
                                .iter()
                                .find(|(ptr, _)| *ptr == probe.0)
                                .map(|(_, name)| name.as_str())
                                .expect("named original allocation");
                            let deadline = REMOVAL_DEADLINES[kind]
                                .iter()
                                .find(|(key, _)| *key == name)
                                .map(|(_, n)| *n)
                                .unwrap_or(usize::MAX);
                            (probe, deadline)
                        })
                        .collect();
                    let mut discharged = vec![false; probes.len()];
                    let mut visits = 0;
                    for attempt in 0..20000 {
                        if visits == cut {
                            break;
                        }
                        let next = probes
                            .iter()
                            .enumerate()
                            .filter(|(i, (_, deadline))| !discharged[*i] && *deadline != usize::MAX)
                            .map(|(_, (_, deadline))| *deadline)
                            .min()
                            .unwrap_or(cut);
                        let before = allocations(creator);
                        for (i, &(probe, deadline)) in probes.iter().enumerate() {
                            if !discharged[i] {
                                assert!(visits < deadline);
                                assert!(
                                    retained(probe, &before),
                                    "before kind {kind} cut {visits} deadline {deadline} probe {probe:?}"
                                );
                            }
                        }
                        let budget = schedule[attempt % schedule.len()]
                            .min(cut - visits)
                            .min(next - visits);
                        creator.poll(budget).unwrap();
                        assert!(creator.last_poll_visits > 0 && creator.last_poll_visits <= budget);
                        visits += creator.last_poll_visits;
                        let after = allocations(creator);
                        for (i, &(probe, deadline)) in probes.iter().enumerate() {
                            if !discharged[i] {
                                assert_eq!(
                                    retained(probe, &after),
                                    visits < deadline,
                                    "kind {kind} cut {visits} deadline {deadline} probe {probe:?}"
                                );
                                if visits == deadline {
                                    discharged[i] = true;
                                }
                            }
                        }
                    }
                    assert_eq!(visits, cut);
                    for budget in [0, 1025] {
                        assert_eq!(
                            creator.poll(budget),
                            Err(crate::living::step_job::LivingLifecycleJobErrorV2::Budget)
                        );
                        assert_eq!(creator.last_poll_visits, 0);
                    }
                    if cut + 1 == REMOVAL_CUTS[kind].len() {
                        for budget in [1, 64, 1024] {
                            assert_eq!(
                                creator.poll(budget).unwrap(),
                                LivingStepProgressV2::NeedsSynchronousCompletion
                            );
                            assert_eq!(creator.last_poll_visits, 0);
                        }
                    } else {
                        assert!(creator.take_ready_result().is_err());
                    }
                    assert!(
                        Arc::ptr_eq(&creator.queue, &queue)
                            && Arc::ptr_eq(&creator.catalog, &catalog)
                    );
                    let current = allocations(creator);
                    for (i, &(probe, deadline)) in probes.iter().enumerate() {
                        assert_eq!(discharged[i], cut >= deadline);
                        if !discharged[i] {
                            assert!(retained(probe, &current));
                        }
                    }
                    step.cancel();
                    assert_eq!(step.poll(1), Err(LivingStepJobErrorV2::Cancelled));
                    assert_eq!(step.last_poll_visits(), 0);
                    assert!(step.take_synchronous_preparation(&anchor).is_err());
                    assert!(
                        std::ptr::eq(&*step, step_pointer)
                            && std::ptr::eq(&**step.creator.as_ref().unwrap(), pointer)
                    );
                    assert_eq!(
                        drain_observed(
                            step.into_boxed_retirement(
                                LivingPublicationRetirementPolicyV2::ReleaseHandle
                            ),
                            schedule,
                            &current,
                            (kind, false, cut)
                        ),
                        expected + 52,
                        "kind {kind} cut {cut}"
                    );
                }
            }
        }
    }
    #[test]
    fn removal_creator_every_storage_capacity_failed_visit_and_sticky_owners() {
        use crate::living::simulation::removal_creator_tests as removal;
        let cases = &[
            (0, 583, true, 402u64),
            (0, 651, false, 474u64),
            (0, 727, false, 474u64),
            (0, 803, false, 474u64),
            (0, 879, false, 475u64),
            (0, 961, false, 473u64),
            (0, 1037, false, 473u64),
            (0, 1113, false, 470u64),
            (0, 1189, false, 473u64),
            (0, 1266, false, 476u64),
            (0, 1350, false, 470u64),
            (0, 1424, false, 470u64),
            (0, 1500, false, 476u64),
            (0, 1592, false, 470u64),
            (0, 1713, false, 431u64),
            (0, 1789, false, 431u64),
            (0, 1867, false, 437u64),
            (0, 1959, false, 426u64),
            (0, 2035, false, 426u64),
            (0, 2114, false, 438u64),
            (0, 2217, false, 435u64),
            (0, 2320, false, 432u64),
            (0, 2420, false, 420u64),
            (0, 2496, false, 420u64),
            (1, 546, true, 447u64),
            (1, 690, false, 612u64),
            (1, 775, false, 611u64),
            (1, 860, false, 610u64),
            (1, 945, false, 610u64),
            (1, 1035, false, 604u64),
            (1, 1112, false, 607u64),
            (1, 1197, false, 603u64),
            (1, 1285, false, 601u64),
            (1, 1370, false, 600u64),
            (1, 1456, false, 600u64),
            (1, 1548, false, 598u64),
            (1, 1640, false, 596u64),
            (1, 1734, false, 596u64),
            (1, 1878, false, 544u64),
            (1, 1962, false, 540u64),
            (1, 2041, false, 549u64),
            (1, 2142, false, 537u64),
            (1, 2230, false, 532u64),
            (1, 2323, false, 533u64),
            (1, 2426, false, 530u64),
            (1, 2529, false, 527u64),
            (1, 2630, false, 518u64),
            (1, 2715, false, 517u64),
            (2, 357, true, 453u64),
            (2, 511, false, 630u64),
            (2, 596, false, 629u64),
            (2, 681, false, 628u64),
            (2, 766, false, 628u64),
            (2, 856, false, 622u64),
            (2, 933, false, 625u64),
            (2, 1018, false, 621u64),
            (2, 1105, false, 620u64),
            (2, 1192, false, 625u64),
            (2, 1292, false, 611u64),
            (2, 1368, false, 617u64),
            (2, 1460, false, 615u64),
            (2, 1553, false, 612u64),
            (2, 1687, false, 567u64),
            (2, 1774, false, 572u64),
            (2, 1877, false, 569u64),
            (2, 1978, false, 557u64),
            (2, 2065, false, 553u64),
            (2, 2157, false, 555u64),
            (2, 2260, false, 552u64),
            (2, 2363, false, 549u64),
            (2, 2464, false, 540u64),
            (2, 2549, false, 539u64),
            (3, 521, true, 429u64),
            (3, 635, false, 558u64),
            (3, 720, false, 557u64),
            (3, 805, false, 556u64),
            (3, 888, false, 550u64),
            (3, 962, false, 552u64),
            (3, 1039, false, 555u64),
            (3, 1124, false, 551u64),
            (3, 1213, false, 548u64),
            (3, 1298, false, 547u64),
            (3, 1382, false, 541u64),
            (3, 1456, false, 541u64),
            (3, 1532, false, 547u64),
            (3, 1624, false, 541u64),
            (3, 1748, false, 511u64),
            (3, 1851, false, 508u64),
            (3, 1951, false, 496u64),
            (3, 2028, false, 496u64),
            (3, 2117, false, 490u64),
            (3, 2207, false, 482u64),
            (3, 2283, false, 482u64),
            (3, 2359, false, 482u64),
            (3, 2436, false, 485u64),
            (3, 2521, false, 484u64),
            (4, 273, true, 384u64),
            (4, 309, false, 420u64),
            (4, 385, false, 420u64),
            (4, 461, false, 420u64),
            (4, 536, false, 418u64),
            (4, 610, false, 420u64),
            (4, 686, false, 420u64),
            (4, 763, false, 420u64),
            (4, 850, false, 415u64),
            (4, 926, false, 415u64),
            (4, 1002, false, 413u64),
            (4, 1076, false, 413u64),
            (4, 1150, false, 413u64),
            (4, 1225, false, 412u64),
            (4, 1301, false, 415u64),
            (4, 1377, false, 415u64),
            (4, 1453, false, 415u64),
            (5, 286, true, 384u64),
            (5, 322, false, 420u64),
            (5, 398, false, 420u64),
            (5, 474, false, 420u64),
            (5, 549, false, 418u64),
            (5, 623, false, 420u64),
            (5, 699, false, 420u64),
            (5, 775, false, 417u64),
            (5, 851, false, 420u64),
            (5, 927, false, 420u64),
            (5, 1004, false, 421u64),
            (5, 1086, false, 417u64),
            (5, 1160, false, 417u64),
            (5, 1235, false, 416u64),
            (5, 1311, false, 419u64),
            (5, 1387, false, 419u64),
            (5, 1463, false, 419u64),
        ];
        for &(kind, cut, storage, expected) in cases {
            for schedule in SCHEDULES {
                let (creator, queue, catalog) = removal::source(kind);
                let anchor = crate::living::builtin_living_sandbox_v2().unwrap();
                let mut step = captured_step(creator, anchor.capture_boundary().unwrap());
                for _ in 0..cut {
                    step.creator.as_mut().unwrap().poll(1).unwrap();
                }
                let creator = step.creator.as_mut().unwrap();
                let pointer = &**creator as *const _;
                let Some(QueuedCreatorInsertion::Removal(work)) = &mut creator.insertion else {
                    panic!("removal owner")
                };
                assert!(if storage {
                    work.storage
                        .as_mut()
                        .unwrap()
                        .inject_structural_empty_capacity_failure()
                } else {
                    work.filter.as_mut().unwrap().inject_capacity()
                });
                assert_eq!(step.poll(1), Err(LivingStepJobErrorV2::PreparationCapacity));
                assert_eq!(step.last_poll_visits(), 1);
                for budget in [1, 64, 1024] {
                    assert_eq!(
                        step.poll(budget),
                        Err(LivingStepJobErrorV2::PreparationCapacity)
                    );
                    assert_eq!(step.last_poll_visits(), 0);
                }
                assert!(step.take_synchronous_preparation(&anchor).is_err());
                let creator = step.creator.as_ref().unwrap();
                assert!(std::ptr::eq(&**creator, pointer));
                assert!(
                    Arc::ptr_eq(&creator.queue, &queue) && Arc::ptr_eq(&creator.catalog, &catalog)
                );
                let current = allocations(creator);
                assert_eq!(
                    drain_observed(
                        step.into_boxed_retirement(
                            LivingPublicationRetirementPolicyV2::ReleaseHandle
                        ),
                        schedule,
                        &current,
                        (kind, false, cut)
                    ),
                    expected
                );
            }
        }
    }
    #[test]
    fn removal_creator_genuine_child_extraction_then_refusal_partitions_owners() {
        use crate::living::simulation::removal_creator_tests as removal;
        let cases = &[
            (0, 584, 0, 402u64, 2u64),
            (0, 652, 1, 474u64, 5u64),
            (0, 688, 2, 441u64, 14u64),
            (0, 728, 1, 474u64, 5u64),
            (0, 764, 2, 441u64, 14u64),
            (0, 804, 1, 474u64, 5u64),
            (0, 840, 2, 441u64, 14u64),
            (0, 880, 1, 475u64, 5u64),
            (0, 919, 2, 445u64, 8u64),
            (0, 962, 1, 473u64, 5u64),
            (0, 998, 2, 440u64, 14u64),
            (0, 1038, 1, 473u64, 5u64),
            (0, 1074, 2, 440u64, 14u64),
            (0, 1114, 1, 470u64, 5u64),
            (0, 1150, 2, 428u64, 23u64),
            (0, 1190, 1, 473u64, 5u64),
            (0, 1226, 2, 440u64, 14u64),
            (0, 1267, 1, 476u64, 5u64),
            (0, 1308, 2, 443u64, 11u64),
            (0, 1351, 1, 470u64, 5u64),
            (0, 1385, 2, 440u64, 11u64),
            (0, 1425, 1, 470u64, 5u64),
            (0, 1459, 2, 440u64, 11u64),
            (0, 1501, 1, 476u64, 5u64),
            (0, 1545, 2, 446u64, 5u64),
            (0, 1593, 1, 470u64, 5u64),
            (0, 1671, 2, 390u64, 21u64),
            (0, 1714, 1, 431u64, 5u64),
            (0, 1750, 2, 398u64, 14u64),
            (0, 1790, 1, 431u64, 5u64),
            (0, 1826, 2, 398u64, 14u64),
            (0, 1868, 1, 437u64, 5u64),
            (0, 1914, 2, 404u64, 8u64),
            (0, 1960, 1, 426u64, 5u64),
            (0, 1996, 2, 384u64, 23u64),
            (0, 2036, 1, 426u64, 5u64),
            (0, 2072, 2, 376u64, 31u64),
            (0, 2115, 1, 438u64, 5u64),
            (0, 2166, 2, 405u64, 5u64),
            (0, 2218, 1, 435u64, 5u64),
            (0, 2269, 2, 402u64, 5u64),
            (0, 2321, 1, 432u64, 5u64),
            (0, 2372, 2, 399u64, 5u64),
            (0, 2421, 1, 420u64, 5u64),
            (0, 2457, 2, 387u64, 14u64),
            (0, 2497, 1, 420u64, 5u64),
            (0, 2533, 2, 387u64, 14u64),
            (1, 547, 0, 447u64, 2u64),
            (1, 691, 1, 612u64, 5u64),
            (1, 732, 2, 579u64, 11u64),
            (1, 776, 1, 611u64, 5u64),
            (1, 817, 2, 578u64, 11u64),
            (1, 861, 1, 610u64, 5u64),
            (1, 902, 2, 577u64, 11u64),
            (1, 946, 1, 610u64, 5u64),
            (1, 990, 2, 580u64, 5u64),
            (1, 1036, 1, 604u64, 5u64),
            (1, 1072, 2, 571u64, 14u64),
            (1, 1113, 1, 607u64, 5u64),
            (1, 1154, 2, 574u64, 11u64),
            (1, 1198, 1, 603u64, 5u64),
            (1, 1242, 2, 561u64, 17u64),
            (1, 1286, 1, 601u64, 5u64),
            (1, 1327, 2, 568u64, 11u64),
            (1, 1371, 1, 600u64, 5u64),
            (1, 1412, 2, 567u64, 11u64),
            (1, 1457, 1, 600u64, 5u64),
            (1, 1501, 2, 570u64, 5u64),
            (1, 1549, 1, 598u64, 5u64),
            (1, 1593, 2, 568u64, 5u64),
            (1, 1641, 1, 596u64, 5u64),
            (1, 1685, 2, 566u64, 5u64),
            (1, 1735, 1, 596u64, 5u64),
            (1, 1829, 2, 520u64, 5u64),
            (1, 1879, 1, 544u64, 5u64),
            (1, 1920, 2, 511u64, 11u64),
            (1, 1963, 1, 540u64, 5u64),
            (1, 1999, 2, 507u64, 14u64),
            (1, 2042, 1, 549u64, 5u64),
            (1, 2093, 2, 516u64, 5u64),
            (1, 2143, 1, 537u64, 5u64),
            (1, 2187, 2, 495u64, 17u64),
            (1, 2231, 1, 532u64, 5u64),
            (1, 2278, 2, 482u64, 22u64),
            (1, 2324, 1, 533u64, 5u64),
            (1, 2375, 2, 500u64, 5u64),
            (1, 2427, 1, 530u64, 5u64),
            (1, 2478, 2, 497u64, 5u64),
            (1, 2530, 1, 527u64, 5u64),
            (1, 2581, 2, 494u64, 5u64),
            (1, 2631, 1, 518u64, 5u64),
            (1, 2672, 2, 485u64, 11u64),
            (1, 2716, 1, 517u64, 5u64),
            (1, 2757, 2, 484u64, 11u64),
            (2, 358, 0, 453u64, 2u64),
            (2, 512, 1, 630u64, 5u64),
            (2, 553, 2, 597u64, 11u64),
            (2, 597, 1, 629u64, 5u64),
            (2, 638, 2, 596u64, 11u64),
            (2, 682, 1, 628u64, 5u64),
            (2, 723, 2, 595u64, 11u64),
            (2, 767, 1, 628u64, 5u64),
            (2, 811, 2, 598u64, 5u64),
            (2, 857, 1, 622u64, 5u64),
            (2, 893, 2, 589u64, 14u64),
            (2, 934, 1, 625u64, 5u64),
            (2, 975, 2, 592u64, 11u64),
            (2, 1019, 1, 621u64, 5u64),
            (2, 1062, 2, 579u64, 18u64),
            (2, 1106, 1, 620u64, 5u64),
            (2, 1147, 2, 587u64, 11u64),
            (2, 1193, 1, 625u64, 5u64),
            (2, 1244, 2, 592u64, 5u64),
            (2, 1293, 1, 611u64, 5u64),
            (2, 1327, 2, 581u64, 11u64),
            (2, 1369, 1, 617u64, 5u64),
            (2, 1413, 2, 587u64, 5u64),
            (2, 1461, 1, 615u64, 5u64),
            (2, 1505, 2, 585u64, 5u64),
            (2, 1554, 1, 612u64, 5u64),
            (2, 1641, 2, 534u64, 12u64),
            (2, 1688, 1, 567u64, 5u64),
            (2, 1729, 2, 534u64, 11u64),
            (2, 1775, 1, 572u64, 5u64),
            (2, 1826, 2, 539u64, 5u64),
            (2, 1878, 1, 569u64, 5u64),
            (2, 1929, 2, 536u64, 5u64),
            (2, 1979, 1, 557u64, 5u64),
            (2, 2022, 2, 515u64, 18u64),
            (2, 2066, 1, 553u64, 5u64),
            (2, 2112, 2, 503u64, 23u64),
            (2, 2158, 1, 555u64, 5u64),
            (2, 2209, 2, 522u64, 5u64),
            (2, 2261, 1, 552u64, 5u64),
            (2, 2312, 2, 519u64, 5u64),
            (2, 2364, 1, 549u64, 5u64),
            (2, 2415, 2, 516u64, 5u64),
            (2, 2465, 1, 540u64, 5u64),
            (2, 2506, 2, 507u64, 11u64),
            (2, 2550, 1, 539u64, 5u64),
            (2, 2591, 2, 506u64, 11u64),
            (3, 522, 0, 429u64, 2u64),
            (3, 636, 1, 558u64, 5u64),
            (3, 677, 2, 525u64, 11u64),
            (3, 721, 1, 557u64, 5u64),
            (3, 762, 2, 524u64, 11u64),
            (3, 806, 1, 556u64, 5u64),
            (3, 847, 2, 523u64, 11u64),
            (3, 889, 1, 550u64, 5u64),
            (3, 923, 2, 520u64, 11u64),
            (3, 963, 1, 552u64, 5u64),
            (3, 999, 2, 519u64, 14u64),
            (3, 1040, 1, 555u64, 5u64),
            (3, 1081, 2, 522u64, 11u64),
            (3, 1125, 1, 551u64, 5u64),
            (3, 1170, 2, 509u64, 16u64),
            (3, 1214, 1, 548u64, 5u64),
            (3, 1255, 2, 515u64, 11u64),
            (3, 1299, 1, 547u64, 5u64),
            (3, 1340, 2, 514u64, 11u64),
            (3, 1383, 1, 541u64, 5u64),
            (3, 1417, 2, 511u64, 11u64),
            (3, 1457, 1, 541u64, 5u64),
            (3, 1491, 2, 511u64, 11u64),
            (3, 1533, 1, 547u64, 5u64),
            (3, 1577, 2, 517u64, 5u64),
            (3, 1625, 1, 541u64, 5u64),
            (3, 1703, 2, 461u64, 21u64),
            (3, 1749, 1, 511u64, 5u64),
            (3, 1800, 2, 478u64, 5u64),
            (3, 1852, 1, 508u64, 5u64),
            (3, 1903, 2, 475u64, 5u64),
            (3, 1952, 1, 496u64, 5u64),
            (3, 1988, 2, 463u64, 14u64),
            (3, 2029, 1, 496u64, 5u64),
            (3, 2074, 2, 454u64, 16u64),
            (3, 2118, 1, 490u64, 5u64),
            (3, 2165, 2, 440u64, 22u64),
            (3, 2208, 1, 482u64, 5u64),
            (3, 2244, 2, 449u64, 14u64),
            (3, 2284, 1, 482u64, 5u64),
            (3, 2320, 2, 449u64, 14u64),
            (3, 2360, 1, 482u64, 5u64),
            (3, 2396, 2, 449u64, 14u64),
            (3, 2437, 1, 485u64, 5u64),
            (3, 2478, 2, 452u64, 11u64),
            (3, 2522, 1, 484u64, 5u64),
            (3, 2563, 2, 451u64, 11u64),
            (4, 274, 0, 384u64, 2u64),
            (4, 310, 1, 420u64, 5u64),
            (4, 346, 2, 387u64, 14u64),
            (4, 386, 1, 420u64, 5u64),
            (4, 422, 2, 387u64, 14u64),
            (4, 462, 1, 420u64, 5u64),
            (4, 498, 2, 387u64, 14u64),
            (4, 537, 1, 418u64, 5u64),
            (4, 571, 2, 388u64, 11u64),
            (4, 611, 1, 420u64, 5u64),
            (4, 647, 2, 387u64, 14u64),
            (4, 687, 1, 420u64, 5u64),
            (4, 723, 2, 387u64, 14u64),
            (4, 764, 1, 420u64, 5u64),
            (4, 808, 2, 378u64, 17u64),
            (4, 851, 1, 415u64, 5u64),
            (4, 887, 2, 382u64, 14u64),
            (4, 927, 1, 415u64, 5u64),
            (4, 963, 2, 382u64, 14u64),
            (4, 1003, 1, 413u64, 5u64),
            (4, 1037, 2, 383u64, 11u64),
            (4, 1077, 1, 413u64, 5u64),
            (4, 1111, 2, 383u64, 11u64),
            (4, 1151, 1, 413u64, 5u64),
            (4, 1185, 2, 383u64, 11u64),
            (4, 1226, 1, 412u64, 5u64),
            (4, 1262, 2, 330u64, 63u64),
            (4, 1302, 1, 415u64, 5u64),
            (4, 1338, 2, 382u64, 14u64),
            (4, 1378, 1, 415u64, 5u64),
            (4, 1414, 2, 382u64, 14u64),
            (4, 1454, 1, 415u64, 5u64),
            (4, 1490, 2, 382u64, 14u64),
            (5, 287, 0, 384u64, 2u64),
            (5, 323, 1, 420u64, 5u64),
            (5, 359, 2, 387u64, 14u64),
            (5, 399, 1, 420u64, 5u64),
            (5, 435, 2, 387u64, 14u64),
            (5, 475, 1, 420u64, 5u64),
            (5, 511, 2, 387u64, 14u64),
            (5, 550, 1, 418u64, 5u64),
            (5, 584, 2, 388u64, 11u64),
            (5, 624, 1, 420u64, 5u64),
            (5, 660, 2, 387u64, 14u64),
            (5, 700, 1, 420u64, 5u64),
            (5, 736, 2, 387u64, 14u64),
            (5, 776, 1, 417u64, 5u64),
            (5, 812, 2, 375u64, 23u64),
            (5, 852, 1, 420u64, 5u64),
            (5, 888, 2, 387u64, 14u64),
            (5, 928, 1, 420u64, 5u64),
            (5, 964, 2, 387u64, 14u64),
            (5, 1005, 1, 421u64, 5u64),
            (5, 1044, 2, 391u64, 8u64),
            (5, 1087, 1, 417u64, 5u64),
            (5, 1121, 2, 387u64, 11u64),
            (5, 1161, 1, 417u64, 5u64),
            (5, 1195, 2, 387u64, 11u64),
            (5, 1236, 1, 416u64, 5u64),
            (5, 1272, 2, 335u64, 62u64),
            (5, 1312, 1, 419u64, 5u64),
            (5, 1348, 2, 386u64, 14u64),
            (5, 1388, 1, 419u64, 5u64),
            (5, 1424, 2, 386u64, 14u64),
            (5, 1464, 1, 419u64, 5u64),
            (5, 1500, 2, 386u64, 14u64),
        ];
        for &(kind, cut, which, expected, external_expected) in cases {
            for schedule in SCHEDULES {
                let (creator, queue, catalog) = removal::source(kind);
                let anchor = crate::living::builtin_living_sandbox_v2().unwrap();
                let mut step = captured_step(creator, anchor.capture_boundary().unwrap());
                for _ in 0..cut {
                    step.creator.as_mut().unwrap().poll(1).unwrap();
                }
                let creator = step.creator.as_mut().unwrap();
                let Some(QueuedCreatorInsertion::Removal(work)) = &mut creator.insertion else {
                    panic!("removal owner")
                };
                let mut external = match which {
                    0 => {
                        let child = work.storage.as_mut().unwrap();
                        let output = child.take_ready_result().unwrap();
                        assert!(child.take_ready_result().is_err());
                        retirement_node(output, LivingStateRetirementPolicyV2::ReleaseHandle)
                    }
                    1 => work.filter.as_mut().unwrap().take_test_preparation(),
                    2 => work.filter.as_mut().unwrap().take_test_output(),
                    _ => unreachable!(),
                };
                assert!(
                    Arc::ptr_eq(&creator.queue, &queue) && Arc::ptr_eq(&creator.catalog, &catalog)
                );
                let current = allocations(creator);
                step.cancel();
                assert!(step.take_synchronous_preparation(&anchor).is_err());
                assert_eq!(
                    drain_observed(
                        step.into_boxed_retirement(
                            LivingPublicationRetirementPolicyV2::ReleaseHandle
                        ),
                        schedule,
                        &current,
                        (kind, false, cut)
                    ),
                    expected
                );
                let mut count = 0;
                let mut done = false;
                for _ in 0..10000 {
                    match external.retirement_step() {
                        RetirementStep::Work => count += 1,
                        RetirementStep::Done => {
                            done = true;
                            break;
                        }
                        RetirementStep::Deferred(_) => panic!("unshared"),
                    }
                }
                assert!(done);
                drop(external);
                count += 1;
                assert_eq!(count, external_expected);
            }
        }
    }

    #[test]
    fn removal_creator_all_finite_failures_retain_and_retire_original_parent() {
        use crate::living::simulation::removal_creator_tests as removal;
        let expected = &[
            357u64, 357u64, 396u64, 399u64, 360u64, 399u64, 414u64, 363u64, 417u64,
        ];
        for (case, &expected) in expected.iter().enumerate() {
            for schedule in SCHEDULES {
                let (creator, queue, catalog) = if case == 8 {
                    removal::alias_source()
                } else {
                    removal::failure_source(case)
                };
                let anchor = crate::living::builtin_living_sandbox_v2().unwrap();
                let mut step = captured_step(creator, anchor.capture_boundary().unwrap());
                let pointer = &**step.creator.as_ref().unwrap() as *const _;
                let mut failure = None;
                for attempt in 0..20000 {
                    let b = schedule[attempt % schedule.len()];
                    match step.poll(b) {
                        Err(e) => {
                            assert!(step.last_poll_visits() > 0 && step.last_poll_visits() <= b);
                            failure = Some(e);
                            break;
                        }
                        Ok(p) => assert_eq!(p, LivingStepProgressV2::Pending),
                    }
                }
                let failure = failure.expect("finite cause");
                assert!(matches!(
                    failure,
                    LivingStepJobErrorV2::Fault(LivingDeterministicFaultV2::Replay { .. })
                ));
                assert!(step.fault.is_none());
                for b in [1, 64, 1024] {
                    assert_eq!(step.poll(b), Err(failure.clone()));
                    assert_eq!(step.last_poll_visits(), 0);
                }
                let creator = step.creator.as_ref().unwrap();
                assert!(std::ptr::eq(&**creator, pointer));
                assert!(
                    Arc::ptr_eq(&creator.queue, &queue) && Arc::ptr_eq(&creator.catalog, &catalog)
                );
                let current = allocations(creator);
                assert_eq!(
                    drain_observed(
                        step.into_boxed_retirement(
                            LivingPublicationRetirementPolicyV2::ReleaseHandle
                        ),
                        schedule,
                        &current,
                        (case, false, 0)
                    ),
                    expected
                );
            }
        }
    }
    #[test]
    fn removal_creator_private_keyed_filter_failure_preserves_pending_and_output() {
        use crate::living::simulation::removal_creator_tests as removal;
        for schedule in SCHEDULES {
            let (mut creator, queue, catalog) = removal::source(4);
            creator.state.stars.record_at_mut(1).unwrap().id = removal::id(13);
            let anchor = crate::living::builtin_living_sandbox_v2().unwrap();
            let mut step = captured_step(creator, anchor.capture_boundary().unwrap());
            for _ in 0..416 {
                step.creator.as_mut().unwrap().poll(1).unwrap();
            }
            let before = allocations(step.creator.as_ref().unwrap());
            assert_eq!(
                step.poll(1),
                Err(LivingStepJobErrorV2::Fault(
                    LivingDeterministicFaultV2::Replay {
                        accepted_sequence: 1,
                        reason: Box::new(LivingCommandRejectionV2::Projection(
                            LivingWireErrorV2::KeyedOrder.into()
                        ))
                    }
                ))
            );
            assert_eq!(step.last_poll_visits(), 1);
            let creator = step.creator.as_ref().unwrap();
            let after = allocations(creator);
            assert!(before.iter().all(|p| retained(*p, &after)));
            assert!(Arc::ptr_eq(&creator.queue, &queue) && Arc::ptr_eq(&creator.catalog, &catalog));
            assert_eq!(
                drain_observed(
                    step.into_boxed_retirement(LivingPublicationRetirementPolicyV2::ReleaseHandle),
                    schedule,
                    &after,
                    (4, false, 416)
                ),
                396
            );
        }
    }
}
