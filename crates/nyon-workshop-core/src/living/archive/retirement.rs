//! Retirement of one finite archive graph. Authority aliases join one existing
//! publication census; archive shells outlive all admitted descendants.
use super::*;
use crate::living::simulation::{
    LivingPublicationRetirementJobV2, LivingPublicationRetirementPolicyV2,
    LivingPublicationRetirementProgressV2,
};
use crate::living::step_job::{
    LivingStateRetirementPolicyV2, RetirementFields, RetirementPoll, RetirementStep,
    retirement_node,
};
use std::collections::LinkedList;

#[derive(Clone, Debug)]
pub(super) struct ArchiveSlot<T>(Option<T>);
impl<T> From<T> for ArchiveSlot<T> {
    fn from(v: T) -> Self {
        Self(Some(v))
    }
}
impl<T> ArchiveSlot<T> {
    pub(super) const fn get(&self) -> &T {
        self.0.as_ref().expect("live archive owner")
    }
    pub(super) fn take(&mut self) -> Option<T> {
        self.0.take()
    }
}
impl<T> std::ops::Deref for ArchiveSlot<T> {
    type Target = T;
    fn deref(&self) -> &T {
        self.get()
    }
}
impl<T> std::ops::DerefMut for ArchiveSlot<T> {
    fn deref_mut(&mut self) -> &mut T {
        self.0.as_mut().expect("live archive owner")
    }
}

// Constructors move roots without allocating/reboxing the original wrapper.
#[allow(clippy::large_enum_variant)]
enum Original<T> {
    Inline(T),
    Boxed(Box<T>),
}
impl<T> Original<T> {
    fn value(&mut self) -> &mut T {
        match self {
            Self::Inline(v) => v,
            Self::Boxed(v) => v,
        }
    }
}
#[allow(clippy::large_enum_variant)]
enum Owner {
    Replay(Original<LivingArchiveReplayV2>),
    Comparison(Original<LivingComparisonReplayV2>),
    Validated(Original<ValidatedLivingArchiveV2>),
    Document(LivingArchiveV2),
    Output(LivingComparisonV2),
}
struct Frame {
    owner: Owner,
    field: usize,
}
#[allow(clippy::large_enum_variant)]
enum Action {
    Child(Owner),
    Node(Box<dyn RetirementPoll>),
    Authority(LivingGalaxyAuthorityV2),
    Replay(LivingReplayBoundaryV2),
    Fault(LivingDeterministicFaultV2),
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
fn error(v: LivingArchiveErrorV2) -> Action {
    match v {
        LivingArchiveErrorV2::Replay(v) => Action::Fault(v),
        LivingArchiveErrorV2::History(v) => match v {
            LivingHistoryErrorV2::Replay(v) => Action::Fault(v),
            LivingHistoryErrorV2::PendingQueue
            | LivingHistoryErrorV2::UnknownView
            | LivingHistoryErrorV2::Empty
            | LivingHistoryErrorV2::DigestMismatch
            | LivingHistoryErrorV2::Wire(_) => Action::Work,
        },
        LivingArchiveErrorV2::Kind
        | LivingArchiveErrorV2::FormatVersion { found: _ }
        | LivingArchiveErrorV2::RulesVersion { found: _ }
        | LivingArchiveErrorV2::CatalogMismatch
        | LivingArchiveErrorV2::Integrity
        | LivingArchiveErrorV2::HistoryGraph
        | LivingArchiveErrorV2::HighWater
        | LivingArchiveErrorV2::FinalDigest
        | LivingArchiveErrorV2::PendingQueue
        | LivingArchiveErrorV2::Cancelled
        | LivingArchiveErrorV2::NotReady
        | LivingArchiveErrorV2::Budget
        | LivingArchiveErrorV2::Wire(_)
        | LivingArchiveErrorV2::Genesis(_)
        | LivingArchiveErrorV2::Command(_)
        | LivingArchiveErrorV2::Authority(_) => Action::Work,
    }
}
impl Frame {
    fn visit(&mut self) -> Action {
        let n = self.field;
        self.field += 1;
        match &mut self.owner {
            Owner::Replay(v) => {
                let v = v.value();
                match n {
                    0 => optional(v.origin.take(), Action::Authority),
                    1 => optional(v.working.take(), Action::Authority),
                    2 => {
                        if let Some(a) = v.completed.pop() {
                            self.field -= 1;
                            Action::Authority(a)
                        } else {
                            Action::Work
                        }
                    }
                    3 => optional(v.validated.take(), |v| {
                        Action::Child(Owner::Validated(Original::Inline(v)))
                    }),
                    4 => optional(v.staged.take(), Action::Replay),
                    5 => optional(v.archive.take(), |v| Action::Child(Owner::Document(v))),
                    6 => node(Bytes(std::mem::take(&mut v.canonical_bytes))),
                    7 => node(Bytes(std::mem::take(&mut v.retired_bytes))),
                    8 => node(std::mem::take(&mut v.tasks)),
                    9 => node(std::mem::take(&mut v.redo_after)),
                    10 => optional(v.failure.take(), error),
                    _ => Action::Done,
                }
            }
            Owner::Validated(v) => {
                let v = v.value();
                match n {
                    0 => optional(v.authority.take(), Action::Authority),
                    1 => optional(v.archive.take(), |v| Action::Child(Owner::Document(v))),
                    2 => node(Bytes(std::mem::take(&mut v.canonical_bytes))),
                    _ => Action::Done,
                }
            }
            Owner::Comparison(v) => {
                let v = v.value();
                match n {
                    0 => optional(v.left.take(), |v| {
                        Action::Child(Owner::Replay(Original::Inline(v)))
                    }),
                    1 => optional(v.right.take(), |v| {
                        Action::Child(Owner::Replay(Original::Inline(v)))
                    }),
                    2 => optional(v.left_ready.take(), |v| {
                        Action::Child(Owner::Validated(Original::Inline(v)))
                    }),
                    3 => optional(v.right_ready.take(), |v| {
                        Action::Child(Owner::Validated(Original::Inline(v)))
                    }),
                    4 => optional(v.comparison.take(), |v| Action::Child(Owner::Output(v))),
                    5 => optional(v.failure.take(), error),
                    _ => Action::Done,
                }
            }
            Owner::Document(v) => match n {
                0 => node(std::mem::take(&mut v.kind)),
                1 => node(std::mem::take(&mut v.genesis_manifest.kind)),
                2 => node(std::mem::take(&mut v.genesis_manifest.state)),
                3 => node(std::mem::take(&mut v.revisions)),
                4 => node(std::mem::take(&mut v.branches)),
                _ => Action::Done,
            },
            Owner::Output(v) => match n {
                0 => node(std::mem::take(&mut v.left_state)),
                1 => node(std::mem::take(&mut v.right_state)),
                _ => Action::Done,
            },
        }
    }
}
impl Owner {
    fn release_storage(&mut self) -> bool {
        match self {
            Self::Replay(v) => {
                if v.value().completed.capacity() != 0 {
                    v.value().completed = Vec::new();
                    true
                } else {
                    false
                }
            }
            _ => false,
        }
    }
}
/// A byte buffer owns storage, not per-byte destructor obligations.
struct Bytes(Vec<u8>);
impl<'a> RetirementFields<'a> for Bytes {
    const LEAF: bool = true;
    fn retirement_child(
        &mut self,
        _: usize,
        _: LivingStateRetirementPolicyV2,
    ) -> Option<Box<dyn RetirementPoll + 'a>> {
        let _ = &self.0;
        None
    }
}
impl<'a> RetirementFields<'a> for LivingHistoryBranchV2 {
    const LEAF: bool = true;
    fn retirement_child(
        &mut self,
        _: usize,
        _: LivingStateRetirementPolicyV2,
    ) -> Option<Box<dyn RetirementPoll + 'a>> {
        None
    }
}
impl<'a> RetirementFields<'a> for ReplayTask {
    fn retirement_child(
        &mut self,
        index: usize,
        p: LivingStateRetirementPolicyV2,
    ) -> Option<Box<dyn RetirementPoll + 'a>> {
        (index == 0).then(|| retirement_node(std::mem::take(&mut self.actions), p))
    }
}
impl<'a> RetirementFields<'a> for ReplayAction {
    fn retirement_child(
        &mut self,
        index: usize,
        p: LivingStateRetirementPolicyV2,
    ) -> Option<Box<dyn RetirementPoll + 'a>> {
        match self {
            Self::Advance(_)
            | Self::Fork {
                branch: _,
                ordinal: _,
            } => None,
            Self::Paused(record) => record.retirement_child(index, p),
            Self::Running { tick: _, records } => {
                (index == 0).then(|| retirement_node(std::mem::take(records), p))
            }
        }
    }
}

/// One admitted archive/comparison graph. Retain through Ready; dropping a
/// partial job is ordinary synchronous Drop, not a bounded cleanup operation.
#[must_use]
pub struct LivingArchiveRetirementJobV2 {
    root: Option<Owner>,
    outer: Option<Owner>,
    adopted_shell: Option<Box<LivingArchiveRetirementJobV2>>,
    frames: LinkedList<Frame>,
    shells: LinkedList<Owner>,
    active: Option<Box<dyn RetirementPoll>>,
    coordinator: Option<LivingPublicationRetirementJobV2>,
    closed: bool,
    waiting: bool,
    ready: bool,
    last: usize,
    total: u64,
}
impl std::fmt::Debug for LivingArchiveRetirementJobV2 {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("LivingArchiveRetirementJobV2")
            .field("ready", &self.ready)
            .field("waiting", &self.waiting)
            .field("total", &self.total)
            .finish_non_exhaustive()
    }
}
impl LivingArchiveRetirementJobV2 {
    fn begin(mut root: Owner, policy: LivingPublicationRetirementPolicyV2) -> Self {
        let previous = match &mut root {
            Owner::Comparison(v) => v.value().cleanup.take(),
            _ => None,
        };
        if let Some(mut previous) = previous {
            assert!(
                previous.adopted_shell.is_none() && previous.outer.is_none(),
                "normal child cleanup is not composed recursively"
            );
            let mut coordinator = previous.coordinator.take().unwrap_or_else(|| {
                LivingPublicationRetirementJobV2::begin_open(
                    LivingPublicationRetirementPolicyV2::ReleaseHandle,
                )
            });
            assert!(
                coordinator.request_archive_reopen(policy),
                "archive-created ReleaseHandle provenance"
            );
            return Self {
                root: previous.root.take(),
                outer: Some(root),
                frames: std::mem::take(&mut previous.frames),
                shells: std::mem::take(&mut previous.shells),
                active: previous.active.take(),
                coordinator: Some(coordinator),
                closed: false,
                waiting: false,
                ready: false,
                last: 0,
                total: 0,
                adopted_shell: Some(previous),
            };
        }
        Self {
            root: Some(root),
            outer: None,
            adopted_shell: None,
            frames: LinkedList::new(),
            shells: LinkedList::new(),
            active: None,
            coordinator: Some(LivingPublicationRetirementJobV2::begin_open(policy)),
            closed: false,
            waiting: false,
            ready: false,
            last: 0,
            total: 0,
        }
    }
    /// Actual ownership transitions used by the latest poll.
    pub fn last_poll_visits(&self) -> usize {
        self.last
    }
    /// Actual ownership transitions since this task was admitted.
    pub fn total_visits(&self) -> u64 {
        self.total
    }
    /// Explicitly retry retained external owners without reinventory or cloning.
    pub fn retry_external_owners(&mut self) {
        if self.waiting {
            self.waiting = false;
            if let Some(c) = &mut self.coordinator {
                c.retry_external_owners();
            }
        }
    }
    /// Count every field, child, admission, close, storage and shell transition.
    pub fn poll(
        &mut self,
        budget: usize,
    ) -> Result<LivingPublicationRetirementProgressV2, LivingArchiveErrorV2> {
        self.last = 0;
        if !(1..=1024).contains(&budget) {
            return Err(LivingArchiveErrorV2::Budget);
        }
        while self.last < budget && !self.waiting && !self.ready {
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
    fn visit(&mut self) -> bool {
        if let Some(owner) = self.outer.take() {
            self.frames.push_back(Frame { owner, field: 0 });
            return true;
        }
        if let Some(child) = &mut self.active {
            match child.retirement_step() {
                RetirementStep::Work => {}
                RetirementStep::Done => {
                    self.active.take();
                }
                RetirementStep::Deferred(_) => {
                    unreachable!("archive independent children have no shared owners")
                }
            }
            return true;
        }
        if let Some(c) = &mut self.coordinator {
            let status = c.poll(1).expect("unit budget");
            if c.last_poll_visits() != 0 {
                return true;
            }
            if status == LivingPublicationRetirementProgressV2::AwaitingExternalOwners {
                self.waiting = true;
                return false;
            }
            if self.closed && status == LivingPublicationRetirementProgressV2::Ready {
                self.coordinator.take();
                return true;
            }
        }
        if let Some(owner) = self.root.take() {
            self.frames.push_front(Frame { owner, field: 0 });
            return true;
        }
        if let Some(frame) = self.frames.front_mut() {
            let action = frame.visit();
            match action {
                Action::Child(owner) => self.frames.push_front(Frame { owner, field: 0 }),
                Action::Node(n) => self.active = Some(n),
                Action::Authority(v) => self
                    .coordinator
                    .as_mut()
                    .expect("open")
                    .try_admit_authority(v)
                    .expect("idle producer"),
                Action::Replay(v) => self
                    .coordinator
                    .as_mut()
                    .expect("open")
                    .try_admit_replay(v)
                    .expect("idle producer"),
                Action::Fault(v) => self
                    .coordinator
                    .as_mut()
                    .expect("open")
                    .try_admit_fault(v)
                    .expect("idle producer"),
                Action::Work => {}
                Action::Done => {
                    let f = self.frames.pop_front().expect("frame");
                    self.shells.push_back(f.owner);
                }
            }
            return true;
        }
        if !self.closed {
            let c = self.coordinator.as_mut().expect("open");
            assert!(c.close_admission());
            self.closed = true;
            c.poll(1).expect("close budget");
            assert_eq!(c.last_poll_visits(), 1);
            return true;
        }
        if let Some(shell) = self.shells.front_mut() {
            if matches!(shell, Owner::Comparison(_)) && self.adopted_shell.take().is_some() {
                return true;
            }
            if !shell.release_storage() {
                self.shells.pop_front();
            }
            return true;
        }
        if self.adopted_shell.take().is_some() {
            return true;
        }
        self.ready = true;
        false
    }
}
macro_rules! handoff {
    ($ty:ty,$variant:ident) => {
        impl $ty {
            /// Move this exact graph into counted retirement without cloning.
            pub fn into_retirement(
                self,
                policy: LivingPublicationRetirementPolicyV2,
            ) -> LivingArchiveRetirementJobV2 {
                LivingArchiveRetirementJobV2::begin(Owner::$variant(Original::Inline(self)), policy)
            }
            /// Retain this original Box until all descendants and storage finish.
            pub fn into_boxed_retirement(
                self: Box<Self>,
                policy: LivingPublicationRetirementPolicyV2,
            ) -> LivingArchiveRetirementJobV2 {
                LivingArchiveRetirementJobV2::begin(Owner::$variant(Original::Boxed(self)), policy)
            }
        }
    };
}
handoff!(LivingArchiveReplayV2, Replay);
handoff!(LivingComparisonReplayV2, Comparison);
handoff!(ValidatedLivingArchiveV2, Validated);
impl LivingArchiveV2 {
    /// Retire document records, manifest state and original storage.
    pub fn into_retirement(
        self,
        policy: LivingPublicationRetirementPolicyV2,
    ) -> LivingArchiveRetirementJobV2 {
        LivingArchiveRetirementJobV2::begin(Owner::Document(self), policy)
    }
}
impl LivingComparisonV2 {
    /// Retire both materialized states through the reviewed state adapter.
    pub fn into_retirement(
        self,
        policy: LivingPublicationRetirementPolicyV2,
    ) -> LivingArchiveRetirementJobV2 {
        LivingArchiveRetirementJobV2::begin(Owner::Output(self), policy)
    }
}

// Exhaustive patterns prevent new retained fields silently bypassing retirement.
macro_rules! audit {($ty:path;$($field:ident),+)=>{const _:fn(&$ty)=|value|{let $ty{$($field:_),+}=value;};}}
audit!(LivingArchiveV2;kind,format_version,rules_version,catalog_hash,genesis_seed,genesis_generator,genesis_manifest,revisions,branches,active_view,final_tick,final_digest,accepted_sequence_high_water,branch_sequence_high_water,integrity_sha256);
audit!(LivingGenesisManifestV2;kind,format_version,rules_version,state);
audit!(ValidatedLivingArchiveV2;archive,authority,canonical_bytes);
audit!(LivingArchiveReplayV2;archive,canonical_bytes,retired_bytes,origin,tasks,task,action,working,completed,validated,cancelled,staged,staged_record,verify_final,redo_after,failure,delivered);
audit!(LivingComparisonReplayV2;left,right,left_ready,right_ready,comparison,cleanup,stage,cancelled,delivered,failure);
audit!(LivingComparisonV2;tick,left,right,left_state,right_state);
audit!(ReplayTask;view,actions);
audit!(LivingHistoryBranchV2;id,parent,fork_revision,fork_tick,ordinal,head,tip_tick);
audit!(LivingArchiveRetirementJobV2;root,outer,adopted_shell,frames,shells,active,coordinator,closed,waiting,ready,last,total);
audit!(Frame;owner,field);

#[cfg(test)]
mod tests {
    use super::*;
    use crate::living::step_job::{LivingLifecycleProgressV2, LivingPhaseRetirementJobV2};
    use crate::living::*;
    const MIXED: &[usize] = &[1, 64, 3, 1024, 2, 9];
    fn replay() -> LivingArchiveReplayV2 {
        let a = builtin_living_sandbox_v2().unwrap();
        begin_decode_living_archive_v2(&encode_living_archive_v2(&a).unwrap(), a.catalog()).unwrap()
    }
    fn ready(mut r: LivingArchiveReplayV2) -> LivingArchiveReplayV2 {
        for _ in 0..1000 {
            if r.poll(64).unwrap() == LivingArchiveReplayStatusV2::Ready {
                return r;
            }
        }
        panic!("ready replay");
    }
    fn compare() -> LivingComparisonReplayV2 {
        let a = builtin_living_sandbox_v2().unwrap();
        begin_compare_living_v2(&a, a.branch(), a.branch(), LivingTickV2(0)).unwrap()
    }
    fn drain(
        job: &mut LivingArchiveRetirementJobV2,
        budgets: &[usize],
    ) -> LivingPublicationRetirementProgressV2 {
        for i in 0..1_000_000 {
            let b = budgets[i % budgets.len()];
            let before = job.total_visits();
            let status = job.poll(b).unwrap();
            assert!(job.last_poll_visits() <= b);
            assert_eq!(job.total_visits() - before, job.last_poll_visits() as u64);
            if status != LivingPublicationRetirementProgressV2::Pending {
                return status;
            }
        }
        panic!("finite retirement");
    }
    fn terminal(
        job: &mut LivingArchiveRetirementJobV2,
        status: LivingPublicationRetirementProgressV2,
    ) {
        let before = job.total_visits();
        for b in [0, 1025] {
            assert_eq!(job.poll(b), Err(LivingArchiveErrorV2::Budget));
            assert_eq!(job.last_poll_visits(), 0);
        }
        for _ in 0..3 {
            assert_eq!(job.poll(1024).unwrap(), status);
            assert_eq!(job.last_poll_visits(), 0);
            assert_eq!(job.total_visits(), before);
        }
    }
    #[test]
    fn archive_retirement_all_budgets_have_same_actual_count() {
        for policy in [
            LivingPublicationRetirementPolicyV2::ReleaseHandle,
            LivingPublicationRetirementPolicyV2::DeferUntilUnique,
        ] {
            let mut expected = None;
            for budgets in [&[1][..], &[64][..], &[1024][..], MIXED] {
                let mut r = ready(replay());
                r.working = Some(r.origin.get().clone());
                r.completed.push(r.origin.get().clone());
                let mut job = r.into_retirement(policy);
                assert_eq!(job.total_visits(), 0);
                assert_eq!(job.poll(0), Err(LivingArchiveErrorV2::Budget));
                assert_eq!(
                    drain(&mut job, budgets),
                    LivingPublicationRetirementProgressV2::Ready
                );
                if let Some(n) = expected {
                    assert_eq!(job.total_visits(), n)
                } else {
                    expected = Some(job.total_visits())
                }
                terminal(&mut job, LivingPublicationRetirementProgressV2::Ready);
            }
        }
    }
    #[test]
    fn archive_retirement_original_box_and_empty_buffer_survive_external_wait() {
        for budgets in [&[1][..], &[64][..], &[1024][..], MIXED] {
            let mut r = Box::new(replay());
            r.completed = Vec::with_capacity(7);
            r.completed.push(r.origin.get().clone());
            let buffer = r.completed.as_ptr();
            let capacity = r.completed.capacity();
            let pointer = &*r as *const _;
            let outside = r.origin.state_snapshot();
            let mut job =
                r.into_boxed_retirement(LivingPublicationRetirementPolicyV2::DeferUntilUnique);
            assert_eq!(
                drain(&mut job, budgets),
                LivingPublicationRetirementProgressV2::AwaitingExternalOwners
            );
            let shell = job
                .shells
                .iter()
                .find_map(|s| match s {
                    Owner::Replay(Original::Boxed(v)) => Some(v),
                    _ => None,
                })
                .unwrap();
            assert_eq!(&**shell as *const _, pointer);
            assert_eq!(shell.completed.as_ptr(), buffer);
            assert_eq!(shell.completed.capacity(), capacity);
            assert!(shell.completed.is_empty());
            terminal(
                &mut job,
                LivingPublicationRetirementProgressV2::AwaitingExternalOwners,
            );
            let shells = job.shells.len();
            for _ in 0..3 {
                job.retry_external_owners();
                assert_eq!(
                    drain(&mut job, budgets),
                    LivingPublicationRetirementProgressV2::AwaitingExternalOwners
                );
                assert_eq!(job.shells.len(), shells);
            }
            drop(outside);
            job.retry_external_owners();
            assert_eq!(
                drain(&mut job, budgets),
                LivingPublicationRetirementProgressV2::Ready
            );
        }
    }
    #[test]
    fn archive_retirement_borrowed_once_outputs_retain_original_wrappers() {
        let mut r = Box::new(replay());
        let ptr = &*r as *const _;
        let origin = r.origin.state_snapshot();
        assert!(matches!(
            r.take_ready_result(),
            Err(LivingArchiveErrorV2::NotReady)
        ));
        assert_eq!(&*r as *const _, ptr);
        assert!(std::sync::Arc::ptr_eq(&origin, &r.origin.state_snapshot()));
        while r.poll(64).unwrap() != LivingArchiveReplayStatusV2::Ready {}
        let mut output = Box::new(r.take_ready_result().unwrap());
        let output_ptr = &*output as *const _;
        assert!(matches!(
            r.take_ready_result(),
            Err(LivingArchiveErrorV2::NotReady)
        ));
        assert_eq!(r.poll(1), Err(LivingArchiveErrorV2::NotReady));
        let authority = output.take_authority().unwrap();
        assert!(matches!(
            output.take_authority(),
            Err(LivingArchiveErrorV2::NotReady)
        ));
        assert_eq!(&*output as *const _, output_ptr);
        assert!(!output.canonical_bytes().is_empty());
        let mut residual =
            r.into_boxed_retirement(LivingPublicationRetirementPolicyV2::ReleaseHandle);
        assert_eq!(
            drain(&mut residual, &[1]),
            LivingPublicationRetirementProgressV2::Ready
        );
        let mut wrapper =
            output.into_boxed_retirement(LivingPublicationRetirementPolicyV2::DeferUntilUnique);
        assert_eq!(
            drain(&mut wrapper, &[64]),
            LivingPublicationRetirementProgressV2::Ready
        );
        drop(origin);
        let mut a = authority.into_retirement(LivingPublicationRetirementPolicyV2::ReleaseHandle);
        while a.poll(1024).unwrap() != LivingPublicationRetirementProgressV2::Ready {}
    }
    #[test]
    fn archive_retirement_comparison_normal_cleanup_and_exact_output() {
        let mut c = compare();
        let mut seen = 0u16;
        assert!(matches!(
            c.take_ready_result(),
            Err(LivingArchiveErrorV2::NotReady)
        ));
        for _ in 0..200_000 {
            seen |= 1 << c.stage;
            if c.poll(1).unwrap() == LivingArchiveReplayStatusV2::Ready {
                break;
            }
        }
        assert_eq!(c.stage, 9);
        assert!(
            c.left.is_none()
                && c.right.is_none()
                && c.left_ready.is_none()
                && c.right_ready.is_none()
                && c.cleanup.is_none()
        );
        assert_eq!(seen & 0x3ff, 0x3ff);
        let out = c.take_ready_result().unwrap();
        assert_eq!(out.left, out.right);
        assert_eq!(out.left_state, out.right_state);
        assert!(matches!(
            c.take_ready_result(),
            Err(LivingArchiveErrorV2::NotReady)
        ));
        assert_eq!(c.poll(1), Err(LivingArchiveErrorV2::NotReady));
        let mut job = c.into_retirement(LivingPublicationRetirementPolicyV2::DeferUntilUnique);
        assert_eq!(
            drain(&mut job, &[1]),
            LivingPublicationRetirementProgressV2::Ready
        );
        let mut output = out.into_retirement(LivingPublicationRetirementPolicyV2::ReleaseHandle);
        assert_eq!(
            drain(&mut output, &[64]),
            LivingPublicationRetirementProgressV2::Ready
        );
    }
    #[test]
    fn archive_retirement_comparison_cancel_adopts_partial_cleanup() {
        for policy in [
            LivingPublicationRetirementPolicyV2::ReleaseHandle,
            LivingPublicationRetirementPolicyV2::DeferUntilUnique,
        ] {
            for budgets in [&[1][..], &[64][..], &[1024][..], MIXED] {
                for offset in [0, 1, 7, 31, 127, 513] {
                    let mut c = Box::new(compare());
                    let pointer = &*c as *const _;
                    while c.cleanup.is_none() {
                        c.poll(1).unwrap();
                    }
                    for _ in 0..offset {
                        c.poll(1).unwrap();
                    }
                    c.cancel();
                    assert_eq!(c.poll(1), Err(LivingArchiveErrorV2::Cancelled));
                    assert!(matches!(
                        c.take_ready_result(),
                        Err(LivingArchiveErrorV2::Cancelled)
                    ));
                    let mut job = c.into_boxed_retirement(policy);
                    assert_eq!(job.total_visits(), 0);
                    // Original outer Box is in the fixed pending slot, not reboxed.
                    let owner = job.outer.as_ref().or(job.root.as_ref()).unwrap();
                    match owner {
                        Owner::Comparison(Original::Boxed(v)) => {
                            assert_eq!(&**v as *const _, pointer)
                        }
                        _ => panic!("original comparison"),
                    }
                    assert_eq!(
                        drain(&mut job, budgets),
                        LivingPublicationRetirementProgressV2::Ready
                    );
                    terminal(&mut job, LivingPublicationRetirementProgressV2::Ready);
                }
            }
        }
    }
    #[test]
    fn archive_retirement_partial_cleanup_external_blocker_is_retained() {
        let mut c = compare();
        while c.cleanup.is_none() {
            c.poll(1).unwrap();
        }
        let outside = c.left_ready.as_ref().unwrap().authority().state_snapshot();
        for _ in 0..31 {
            c.poll(1).unwrap();
        }
        let original_cleanup = &**c.cleanup.as_ref().unwrap() as *const _;
        c.cancel();
        let mut job = c.into_retirement(LivingPublicationRetirementPolicyV2::DeferUntilUnique);
        assert_eq!(
            &**job.adopted_shell.as_ref().unwrap() as *const _,
            original_cleanup
        );
        assert_eq!(
            drain(&mut job, &[1]),
            LivingPublicationRetirementProgressV2::AwaitingExternalOwners
        );
        assert!(job.adopted_shell.is_some());
        terminal(
            &mut job,
            LivingPublicationRetirementProgressV2::AwaitingExternalOwners,
        );
        drop(outside);
        job.retry_external_owners();
        assert_eq!(
            drain(&mut job, MIXED),
            LivingPublicationRetirementProgressV2::Ready
        );
    }
    #[test]
    fn archive_retirement_comparison_fault_retains_both_ready_sides() {
        let mut c = compare();
        while c.stage != 6 {
            c.poll(1).unwrap();
            if c.stage > 6 {
                panic!("use exact stage")
            }
        }
        // A sticky owned replay fault retains existing ready wrappers on repeated reporting.
        c.failure = Some(LivingArchiveErrorV2::Replay(
            LivingDeterministicFaultV2::Replay {
                accepted_sequence: 1,
                reason: Box::new(LivingCommandRejectionV2::Seal(
                    LivingDeterministicFaultV2::Invariant,
                )),
            },
        ));
        assert!(c.poll(1).is_err());
        assert!(c.take_ready_result().is_err());
        assert!(c.left_ready.is_some() && c.right_ready.is_some());
        let mut job = c.into_retirement(LivingPublicationRetirementPolicyV2::DeferUntilUnique);
        assert_eq!(
            drain(&mut job, MIXED),
            LivingPublicationRetirementProgressV2::Ready
        );
    }
    fn record(n: usize) -> LivingRecordedRevisionV2 {
        LivingRecordedRevisionV2 {
            revision: LivingRevisionV2 {
                id: LivingRevisionIdV2([1; 32]),
                parent: None,
                tick: LivingTickV2(0),
                ordinal: 1,
                command: LivingCommandV2::CreatorBatch {
                    operations: vec![LivingCreatorOperationV2::SetFleetOrder {
                        kind: None,
                        target_world: None,
                        not_before_tick: LivingTickV2(0),
                        fleet: LivingCommandTargetV2::Existing {
                            id: LivingEntityIdV2([2; 16]),
                        },
                        lane_path: vec![
                            LivingCommandTargetV2::Existing {
                                id: LivingEntityIdV2([3; 16])
                            };
                            n
                        ],
                    }],
                },
            },
            mode: LivingCommandModeV2::Paused,
        }
    }
    fn node_visits<T: RetirementFields<'static>>(v: T) -> u64 {
        let mut job =
            LivingPhaseRetirementJobV2::begin(v, LivingStateRetirementPolicyV2::ReleaseHandle);
        while job.poll(1).unwrap() != LivingLifecycleProgressV2::Ready {}
        job.total_visits()
    }
    #[test]
    fn archive_retirement_byte_storage_and_action_counts_are_independent() {
        for n in [0, 1, 256, 10000] {
            assert_eq!(node_visits(Bytes(Vec::with_capacity(n))), 1);
            assert_eq!(node_visits(Bytes(vec![9; n])), 1);
        }
        assert_eq!(node_visits(ReplayAction::Advance(LivingTickV2(0))), 1);
        assert_eq!(
            node_visits(ReplayAction::Fork {
                branch: LivingBranchIdV2([0; 16]),
                ordinal: 2
            }),
            1
        );
        // SetFleetOrder: 3n+4. operations Vec adds 3; record adds 3.
        for n in [0, 1, 17] {
            let r = record(n);
            assert_eq!(
                node_visits(ReplayAction::Paused(r.clone())),
                3 * n as u64 + 10
            );
            // Running adds a records Vec + one wrapper: six transitions.
            assert_eq!(
                node_visits(ReplayAction::Running {
                    tick: LivingTickV2(0),
                    records: vec![r]
                }),
                3 * n as u64 + 16
            );
        }
    }
    #[test]
    fn archive_retirement_all_actions_and_recursive_history_error() {
        let mut r = replay();
        r.tasks[0].actions = vec![
            ReplayAction::Advance(LivingTickV2(0)),
            ReplayAction::Fork {
                branch: r.origin.branch(),
                ordinal: 7,
            },
            ReplayAction::Paused(record(17)),
            ReplayAction::Running {
                tick: LivingTickV2(1),
                records: vec![record(31)],
            },
        ];
        let mut fault = LivingDeterministicFaultV2::Invariant;
        for _ in 0..3000 {
            fault = LivingDeterministicFaultV2::Replay {
                accepted_sequence: 1,
                reason: Box::new(LivingCommandRejectionV2::Seal(fault)),
            };
        }
        r.failure = Some(LivingArchiveErrorV2::History(LivingHistoryErrorV2::Replay(
            fault,
        )));
        let mut job = r.into_retirement(LivingPublicationRetirementPolicyV2::DeferUntilUnique);
        assert_eq!(
            drain(&mut job, &[1]),
            LivingPublicationRetirementProgressV2::Ready
        );
    }
    #[test]
    fn archive_retirement_empty_residual_has_independent_shell_storage_formula() {
        for policy in [
            LivingPublicationRetirementPolicyV2::ReleaseHandle,
            LivingPublicationRetirementPolicyV2::DeferUntilUnique,
        ] {
            for allocated in [false, true] {
                let mut r = replay();
                r.archive.take();
                r.origin.take();
                r.tasks.clear();
                r.tasks = Vec::new();
                r.canonical_bytes = Vec::new();
                if allocated {
                    r.canonical_bytes = Vec::with_capacity(17);
                    r.retired_bytes = Vec::with_capacity(31);
                    r.tasks = Vec::with_capacity(5);
                    r.redo_after = Vec::with_capacity(7);
                    r.completed = Vec::with_capacity(11);
                }
                let mut job = r.into_retirement(policy);
                assert_eq!(
                    drain(&mut job, &[1]),
                    LivingPublicationRetirementProgressV2::Ready
                );
                // Root 1 + eleven field transitions + four empty child roots
                // and their node releases (8) + frame 1 + close 1 + coordinator
                // release 1 + shell 1 = 24. Defer adds two empty registry roots;
                // only completed Vec storage is held separately behind census.
                assert_eq!(
                    job.total_visits(),
                    24 + u64::from(allocated)
                        + if policy == LivingPublicationRetirementPolicyV2::DeferUntilUnique {
                            2
                        } else {
                            0
                        }
                );
            }
        }
    }
    #[test]
    fn archive_retirement_comparison_censuses_both_sides_before_external_wait() {
        let mut c = Box::new(compare());
        let mut shared = builtin_living_sandbox_v2().unwrap();
        shared.step().unwrap();
        for r in [c.left.as_mut().unwrap(), c.right.as_mut().unwrap()] {
            r.origin = shared.clone().into();
            r.working = Some(shared.clone());
            r.completed.push(shared.clone());
        }
        let external = shared.catalog_snapshot();
        // The authority itself is another internal root, not an external blocker.
        c.left.as_mut().unwrap().completed.push(shared);
        let outer = &*c as *const _;
        let mut job =
            c.into_boxed_retirement(LivingPublicationRetirementPolicyV2::DeferUntilUnique);
        assert_eq!(
            drain(&mut job, MIXED),
            LivingPublicationRetirementProgressV2::AwaitingExternalOwners
        );
        assert!(job.frames.is_empty() && job.root.is_none() && job.closed);
        let mut children = 0;
        for shell in &job.shells {
            match shell {
                Owner::Replay(_) => children += 1,
                Owner::Comparison(Original::Boxed(v)) => assert_eq!(&**v as *const _, outer),
                _ => {}
            }
        }
        assert_eq!(children, 2);
        terminal(
            &mut job,
            LivingPublicationRetirementProgressV2::AwaitingExternalOwners,
        );
        drop(external);
        job.retry_external_owners();
        assert_eq!(
            drain(&mut job, &[1]),
            LivingPublicationRetirementProgressV2::Ready
        );
    }
}
