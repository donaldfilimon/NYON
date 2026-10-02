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
        | LivingStepJobErrorV2::Stale => Action::Work,
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
    fn begin(root: Owner, policy: LivingPublicationRetirementPolicyV2) -> Self {
        Self {
            root: Some(root),
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
        while self.last < budget && !self.ready && !self.waiting {
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
fields!(LivingTickReceiptV2;applied_revisions,events);
fields!(super::super::decision::LivingDecisionReceiptV2;kind,intents);
fields!(super::super::event_facts::LivingEventFactsReceiptV2;events,omitted);
fields!(super::super::event_facts::LivingEventFactsOmittedV2;ranges);
impl<'a> RetirementFields<'a> for super::super::event_facts::LivingExplainedEventV2 {
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
            Self::ObservationsComplete | Self::DiplomacyComplete | Self::DispatchComplete => None,
        }
    }
}
impl<'a> RetirementFields<'a> for QueuedCreatorInsertion {
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
        }
    }
}
impl RetirementPoll for LivingPublicationRetirementJobV2 {
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
audited_fields!(LivingStepJobV2;capture,copy,hash,candidate,source_digest,creator,scheduled,conflict,production,observation,observation_events,diplomacy,dispatch,completed_output,fault,cancelled,delivered,retirement,last_poll_visits,total_visits);
audited_fields!(LivingQueuedCreatorJobV2;state,queue,catalog,events,stage,revision_index,operation_index,index,locals,dormant,insertion,retirement,last_poll_visits,delivered);
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
                    QueuedCreatorInsertion::System(_) => 10,
                    QueuedCreatorInsertion::Star(_) => 11,
                    QueuedCreatorInsertion::World(_) => 12,
                    QueuedCreatorInsertion::Lane(_) => 13,
                    QueuedCreatorInsertion::Civilization(_) => 14,
                    QueuedCreatorInsertion::Deposit(_) => 15,
                };
            }
            return 20 + c.stage as u8;
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
            QueuedCreatorStage::EmitCreator,
            QueuedCreatorStage::FinishRevision,
            QueuedCreatorStage::NeedsSynchronous,
            QueuedCreatorStage::Ready,
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
