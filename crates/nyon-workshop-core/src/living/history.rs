//! Immutable Living history views. A view names creator inputs and a completed
//! tick; bookmarks additionally pin the materialized digest. Checkpoints never
//! appear in these records or in portable archives.
use super::command::{LivingCommandModeV2, LivingRevisionV2};
use super::ids::{LivingBranchIdV2, LivingRevisionIdV2, LivingStateDigestV2, LivingTickV2};
use super::model::LivingGalaxyStateV2;
use super::simulation::LivingDeterministicFaultV2;
use super::wire::{LivingKeyedV2, LivingWireErrorV2};
use serde::{Deserialize, Serialize};

/// A canonical creator input and its accepted application phase. The mode is
/// stored beside the frozen revision rather than changing its identity bytes.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct LivingRecordedRevisionV2 {
    /// The immutable revision identity and creator command.
    pub revision: LivingRevisionV2,
    /// Running applies before phases; paused applies after the completed tick.
    pub mode: LivingCommandModeV2,
}
impl LivingKeyedV2 for LivingRecordedRevisionV2 {
    type Key = LivingRevisionIdV2;
    fn living_key(&self) -> Self::Key {
        self.revision.id
    }
}

/// Canonical provenance and retained future of one branch. Fields retain this
/// implementation's declared order; no derived state or checkpoint is stored.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct LivingHistoryBranchV2 {
    /// Stable branch identity.
    pub id: LivingBranchIdV2,
    /// The branch from which this branch diverged, null for root.
    pub parent: Option<LivingBranchIdV2>,
    /// Exact parent revision cursor at divergence, null for root.
    pub fork_revision: Option<LivingRevisionIdV2>,
    /// Completed parent boundary at divergence.
    pub fork_tick: LivingTickV2,
    /// Document-global fork allocator value; zero for root.
    pub ordinal: u64,
    /// Latest immutable creator revision of the retained future.
    pub head: Option<LivingRevisionIdV2>,
    /// Latest completed tick of the retained future.
    pub tip_tick: LivingTickV2,
}
impl LivingKeyedV2 for LivingHistoryBranchV2 {
    type Key = LivingBranchIdV2;
    fn living_key(&self) -> Self::Key {
        self.id
    }
}

/// An exact history selection, independent of transport or rendered selection.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct LivingHistoryViewV2 {
    /// Selected branch.
    pub branch: LivingBranchIdV2,
    /// Selected immutable creator cursor, null for genesis.
    pub revision: Option<LivingRevisionIdV2>,
    /// Completed tick at which to materialize those creator inputs.
    pub tick: LivingTickV2,
}
/// A falsifiable bookmark: an exact history view plus its verified digest.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct LivingBookmarkV2 {
    /// Exact view.
    pub view: LivingHistoryViewV2,
    /// Digest of that materialized view.
    pub digest: LivingStateDigestV2,
}
/// Two fully materialized histories compared at the same completed tick.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LivingComparisonV2 {
    /// Shared completed boundary.
    pub tick: LivingTickV2,
    /// Exact left branch/cursor.
    pub left: LivingBookmarkV2,
    /// Exact right branch/cursor.
    pub right: LivingBookmarkV2,
    /// Left materialized state.
    pub left_state: LivingGalaxyStateV2,
    /// Right materialized state.
    pub right_state: LivingGalaxyStateV2,
}
/// A navigation failure never changes the active authority or its queue.
#[derive(Clone, Debug, Eq, PartialEq, thiserror::Error)]
pub enum LivingHistoryErrorV2 {
    /// Uncommitted work must be handled before history navigation.
    #[error("Living history navigation requires an empty creator queue")]
    PendingQueue,
    /// Requested branch, revision, tick, or graph relationship does not exist.
    #[error("the requested Living history view is not retained")]
    UnknownView,
    /// No creator revision remains to undo or redo.
    #[error("there is no Living creator revision to undo or redo")]
    Empty,
    /// Bookmark bytes no longer describe the materialized state.
    #[error("the Living bookmark digest does not match replay")]
    DigestMismatch,
    /// Replay stopped at the last valid boundary.
    #[error("Living history replay failed: {0}")]
    Replay(#[from] LivingDeterministicFaultV2),
    /// A digest could not be encoded.
    #[error("Living history canonical encoding failed: {0}")]
    Wire(#[from] LivingWireErrorV2),
}
