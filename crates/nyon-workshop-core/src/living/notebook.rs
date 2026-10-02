//! Portable evidence metadata, attached to exact archive bytes. Replay outputs
//! are checked before a notebook is exposed; no cache is serialized.
use super::archive::{
    LivingArchiveErrorV2, LivingArchiveReplayStatusV2, LivingArchiveReplayV2,
    ValidatedLivingArchiveV2, begin_restore_living_view_v2,
};
use super::catalog::LivingNameV2;
use super::decision::LivingDecisionScoreV2;
use super::history::LivingHistoryViewV2;
use super::ids::{
    LivingArchiveIntegrityV2, LivingCatalogHashV2, LivingEntityIdV2, LivingEventIdV2,
    LivingReceiptDigestV2, LivingStateDigestV2, LivingTickV2,
};
use super::model::LivingGalaxyStateV2;
use super::receipt::{LivingEventKindV2, LivingEventProvenanceV2, LivingTickReceiptV2};
use super::wire::{LivingWireErrorV2, decode_canonical_v2, encode_canonical_v2};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

/// Maximum canonical notebook size.
pub const LIVING_MAX_NOTEBOOK_BYTES_V2: usize = 1_048_576;
/// Independent integrity domain; gameplay identity is unaffected.
pub const LIVING_NOTEBOOK_DOMAIN_V2: &[u8] = b"NYON-LIVING-NOTEBOOK-V2\0";
/// Named exact state reference.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct LivingNamedBookmarkV2 {
    /// Strictly increasing, never reused notebook ordinal.
    pub ordinal: u64,
    /// Validated display label.
    pub label: LivingNameV2,
    /// Exact branch, creator cursor and completed tick.
    pub view: LivingHistoryViewV2,
    /// Replay witness.
    pub digest: LivingStateDigestV2,
}
/// Accepted suggested experiment category; a record proves an observation only.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum LivingNotebookExperimentKindV2 {
    SupplyChain,
    AutonomousSettlement,
    TradeDelivery,
    CorridorImpact,
    WarAlternative,
    DormantRecovery,
    PortableReproduction,
}
/// Exact ordinary event identity; no explanation or outcome is inferred.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct LivingNotebookEventReferenceV2 {
    pub ordinal: u16,
    pub id: LivingEventIdV2,
    pub kind: LivingEventKindV2,
}
/// Locate an actual replay-regenerated hazard-restricted route attempt.
/// Quantities remain authoritative replay facts, never persisted assertions.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct LivingNotebookRouteAttemptReferenceV2 {
    pub route: LivingEntityIdV2,
    pub hazard: LivingEntityIdV2,
}
/// Replay-checked observation linked to an exact named state.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct LivingNotebookEvidenceRecordV2 {
    pub ordinal: u64,
    pub bookmark: u64,
    pub experiment: LivingNotebookExperimentKindV2,
    pub receipt_digest: LivingReceiptDigestV2,
    pub events: Vec<LivingNotebookEventReferenceV2>,
    /// Absent preserves historical bytes; only Corridor evidence may contain it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub route_attempts: Option<Vec<LivingNotebookRouteAttemptReferenceV2>>,
}
/// Persistent allocation mark remains after evidence records are removed.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct LivingNotebookEvidenceV2 {
    pub next_ordinal: u64,
    pub records: Vec<LivingNotebookEvidenceRecordV2>,
}
/// Metrics whose complete scope is defined in the approved notebook review.
#[derive(Clone, Copy, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum LivingNotebookMetricV2 {
    /// World stockpile energy.
    InventoryEnergy,
    /// World stockpile ore.
    InventoryOre,
    /// World stockpile alloy.
    InventoryAlloy,
    /// World owner.
    Owner,
    /// Galaxy total or civilization-owned colonies.
    ColonyCount,
    /// Galaxy total or civilization-owned facilities.
    FacilityCount,
    /// Galaxy total or civilization-owned hulls.
    HullCount,
    /// Completed tick, with no entity scope.
    Tick,
}
/// Typed measured output; integers use canonical signed decimal strings.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(
    tag = "type",
    content = "value",
    rename_all = "snake_case",
    deny_unknown_fields
)]
pub enum LivingNotebookValueV2 {
    /// Measured quantity.
    Integer(LivingDecisionScoreV2),
    /// Nullable world owner.
    Entity(Option<LivingEntityIdV2>),
    /// Scoped entity is absent from this view.
    Absent,
}
/// Matching-tick observation, sorted by metric then nullable scope.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct LivingNotebookObservationV2 {
    /// Metric registry entry.
    pub metric: LivingNotebookMetricV2,
    /// World ID for inventory/owner; civilization ID or null for counts.
    pub entity: Option<LivingEntityIdV2>,
    /// Measured left value.
    pub left: LivingNotebookValueV2,
    /// Measured right value.
    pub right: LivingNotebookValueV2,
}
/// Falsifiable comparison of two retained histories.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct LivingNotebookExperimentV2 {
    /// Strictly increasing experiment ordinal.
    pub ordinal: u64,
    /// Validated display label.
    pub label: LivingNameV2,
    /// Left bookmark ordinal.
    pub left: u64,
    /// Right bookmark ordinal.
    pub right: u64,
    /// Both histories are measured here.
    pub comparison_tick: LivingTickV2,
    /// Ordered, unique measured outputs.
    pub observations: Vec<LivingNotebookObservationV2>,
}
/// Optional third portable file. Existing archive bytes remain unchanged.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct LivingNotebookV2 {
    /// NYON_LIVING_NOTEBOOK.
    pub kind: String,
    /// Two.
    pub format_version: u32,
    /// Two.
    pub rules_version: u32,
    /// Exact catalog attachment.
    pub catalog_hash: LivingCatalogHashV2,
    /// Validated embedded genesis digest.
    pub genesis_digest: LivingStateDigestV2,
    /// Exact exported archive attachment.
    pub archive_integrity_sha256: LivingArchiveIntegrityV2,
    /// At most 64 named bookmarks.
    pub bookmarks: Vec<LivingNamedBookmarkV2>,
    /// At most 64 comparisons.
    pub experiments: Vec<LivingNotebookExperimentV2>,
    /// Optional durable observations; absent preserves historical canonical bytes.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub evidence: Option<LivingNotebookEvidenceV2>,
    /// Independent integrity over the ordered payload.
    pub integrity_sha256: LivingArchiveIntegrityV2,
}
#[derive(Serialize)]
struct Payload<'a> {
    kind: &'a str,
    format_version: u32,
    rules_version: u32,
    catalog_hash: LivingCatalogHashV2,
    genesis_digest: LivingStateDigestV2,
    archive_integrity_sha256: LivingArchiveIntegrityV2,
    bookmarks: &'a [LivingNamedBookmarkV2],
    experiments: &'a [LivingNotebookExperimentV2],
    #[serde(skip_serializing_if = "Option::is_none")]
    evidence: Option<&'a LivingNotebookEvidenceV2>,
}
impl LivingNotebookV2 {
    fn integrity(&self) -> Result<LivingArchiveIntegrityV2, LivingNotebookErrorV2> {
        let payload = Payload {
            kind: &self.kind,
            format_version: self.format_version,
            rules_version: self.rules_version,
            catalog_hash: self.catalog_hash,
            genesis_digest: self.genesis_digest,
            archive_integrity_sha256: self.archive_integrity_sha256,
            bookmarks: &self.bookmarks,
            experiments: &self.experiments,
            evidence: self.evidence.as_ref(),
        };
        let mut hash = Sha256::new();
        hash.update(LIVING_NOTEBOOK_DOMAIN_V2);
        hash.update(encode_canonical_v2(&payload, LIVING_MAX_NOTEBOOK_BYTES_V2)?);
        Ok(LivingArchiveIntegrityV2(hash.finalize().into()))
    }
    /// Seal a metadata candidate. Installation still requires bounded validation.
    pub fn seal(
        archive: &ValidatedLivingArchiveV2,
        bookmarks: Vec<LivingNamedBookmarkV2>,
        experiments: Vec<LivingNotebookExperimentV2>,
    ) -> Result<Self, LivingNotebookErrorV2> {
        Self::seal_with_evidence(archive, bookmarks, experiments, None)
    }
    /// Seal additive durable observations without changing absent-evidence bytes.
    pub fn seal_with_evidence(
        archive: &ValidatedLivingArchiveV2,
        bookmarks: Vec<LivingNamedBookmarkV2>,
        experiments: Vec<LivingNotebookExperimentV2>,
        evidence: Option<LivingNotebookEvidenceV2>,
    ) -> Result<Self, LivingNotebookErrorV2> {
        let mut notebook = Self {
            kind: "NYON_LIVING_NOTEBOOK".into(),
            format_version: 2,
            rules_version: 2,
            catalog_hash: archive.archive().catalog_hash,
            genesis_digest: archive.archive().genesis_manifest.state.digest()?,
            archive_integrity_sha256: archive.archive().integrity_sha256,
            bookmarks,
            experiments,
            evidence,
            integrity_sha256: LivingArchiveIntegrityV2([0; 32]),
        };
        notebook.integrity_sha256 = notebook.integrity()?;
        notebook.validate_shape(archive)?;
        notebook.canonical_bytes()?;
        Ok(notebook)
    }
    /// Canonical portable bytes.
    pub fn canonical_bytes(&self) -> Result<Vec<u8>, LivingNotebookErrorV2> {
        Ok(encode_canonical_v2(self, LIVING_MAX_NOTEBOOK_BYTES_V2)?)
    }
    fn validate_shape(
        &self,
        archive: &ValidatedLivingArchiveV2,
    ) -> Result<(), LivingNotebookErrorV2> {
        if self.kind != "NYON_LIVING_NOTEBOOK"
            || self.format_version != 2
            || self.rules_version != 2
            || self.catalog_hash != archive.archive().catalog_hash
            || self.genesis_digest != archive.archive().genesis_manifest.state.digest()?
            || self.archive_integrity_sha256 != archive.archive().integrity_sha256
            || self.integrity_sha256 != self.integrity()?
        {
            return Err(LivingNotebookErrorV2::Attachment);
        }
        if self.bookmarks.len() > 64
            || self.experiments.len() > 64
            || self.bookmarks.first().is_some_and(|x| x.ordinal == 0)
            || self
                .bookmarks
                .windows(2)
                .any(|x| x[0].ordinal >= x[1].ordinal)
            || self.experiments.first().is_some_and(|x| x.ordinal == 0)
            || self
                .experiments
                .windows(2)
                .any(|x| x[0].ordinal >= x[1].ordinal)
        {
            return Err(LivingNotebookErrorV2::Shape);
        }
        if let Some(evidence) = &self.evidence {
            if evidence.next_ordinal < 2
                || evidence.records.len() > 128
                || evidence
                    .records
                    .windows(2)
                    .any(|pair| pair[0].ordinal >= pair[1].ordinal)
            {
                return Err(LivingNotebookErrorV2::Shape);
            }
            for record in &evidence.records {
                let attempts = record.route_attempts.as_deref().unwrap_or_default();
                if record.ordinal == 0
                    || record.ordinal >= evidence.next_ordinal
                    || !self
                        .bookmarks
                        .iter()
                        .any(|bookmark| bookmark.ordinal == record.bookmark)
                    || (record.events.is_empty() && attempts.is_empty())
                    || record.events.len() + attempts.len() > 32
                    || (record.route_attempts.is_some()
                        && (record.experiment != LivingNotebookExperimentKindV2::CorridorImpact
                            || attempts.is_empty()))
                    || attempts.windows(2).any(|pair| {
                        (pair[0].route, pair[0].hazard) >= (pair[1].route, pair[1].hazard)
                    })
                    || record
                        .events
                        .windows(2)
                        .any(|pair| pair[0].ordinal >= pair[1].ordinal)
                    || record
                        .events
                        .iter()
                        .map(|event| event.id)
                        .collect::<std::collections::BTreeSet<_>>()
                        .len()
                        != record.events.len()
                {
                    return Err(LivingNotebookErrorV2::Shape);
                }
            }
        }
        for experiment in &self.experiments {
            if !self.bookmarks.iter().any(|x| x.ordinal == experiment.left)
                || !self.bookmarks.iter().any(|x| x.ordinal == experiment.right)
                || experiment.observations.is_empty()
                || experiment.observations.len() > 128
                || experiment
                    .observations
                    .windows(2)
                    .any(|x| (x[0].metric, x[0].entity) >= (x[1].metric, x[1].entity))
            {
                return Err(LivingNotebookErrorV2::Shape);
            }
        }
        Ok(())
    }
}
/// No failure exposes an unvalidated notebook or changes authority.
#[derive(Clone, Debug, Eq, PartialEq, thiserror::Error)]
pub enum LivingNotebookErrorV2 {
    /// Wrong document attachment or integrity.
    #[error("Living notebook attachment or integrity differs")]
    Attachment,
    /// Invalid scope, ordering, capacity or typed value.
    #[error("Living notebook schema or metric scope is invalid")]
    Shape,
    /// Replay witness differs.
    #[error("Living notebook evidence differs from replay")]
    Evidence,
    /// Validation cancelled.
    #[error("Living notebook validation cancelled")]
    Cancelled,
    /// Canonical encoding failure.
    #[error(transparent)]
    Wire(#[from] LivingWireErrorV2),
    /// History reconstruction failure.
    #[error(transparent)]
    Replay(#[from] LivingArchiveErrorV2),
}
/// Validated metadata with all referenced states and observations recomputed.
pub struct ValidatedLivingNotebookV2 {
    notebook: LivingNotebookV2,
    source_revisions: Vec<super::history::LivingRecordedRevisionV2>,
    source_branches: Vec<super::history::LivingHistoryBranchV2>,
    trade_completions: Vec<LivingNotebookTradeCompletionV2>,
    supply_completions: Vec<LivingNotebookSupplyCompletionV2>,
    corridor_observations: Vec<LivingNotebookCorridorObservationPairV2>,
}
/// Which observed resource connection supports alloy production.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum LivingNotebookSupplyTopologyV2 {
    AlloyDelivered,
    OreDeliveredToAlloyProducer,
}
/// Replay-derived production and inter-system transfer, without fungible-unit causation.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LivingNotebookSupplyCompletionV2 {
    production: SupplyProduction,
    delivered: EvidenceReference,
    delivery: super::event_facts::LivingDeliveryFactV2,
    topology: LivingNotebookSupplyTopologyV2,
}
#[derive(Clone, Debug, Eq, PartialEq)]
struct SupplyProduction {
    reference: EvidenceReference,
    actor: Option<LivingEntityIdV2>,
    facility: LivingEntityIdV2,
    world: LivingEntityIdV2,
    system: LivingEntityIdV2,
    ore_input: u32,
    alloy_output: u32,
}
struct SupplyDelivery {
    reference: EvidenceReference,
    delivery: super::event_facts::LivingDeliveryFactV2,
}
impl LivingNotebookSupplyCompletionV2 {
    pub const fn production_record(&self) -> u64 {
        self.production.reference.record
    }
    pub const fn delivery_record(&self) -> u64 {
        self.delivered.record
    }
    pub const fn production_bookmark(&self) -> u64 {
        self.production.reference.bookmark
    }
    pub const fn delivery_bookmark(&self) -> u64 {
        self.delivered.bookmark
    }
    pub const fn production_view(&self) -> LivingHistoryViewV2 {
        self.production.reference.view
    }
    pub const fn delivery_view(&self) -> LivingHistoryViewV2 {
        self.delivered.view
    }
    pub fn production_event(&self) -> &LivingNotebookEventReferenceV2 {
        &self.production.reference.event
    }
    pub fn delivery_event(&self) -> &LivingNotebookEventReferenceV2 {
        &self.delivered.event
    }
    pub const fn production_actor(&self) -> Option<LivingEntityIdV2> {
        self.production.actor
    }
    pub const fn production_facility(&self) -> LivingEntityIdV2 {
        self.production.facility
    }
    pub const fn production_world(&self) -> LivingEntityIdV2 {
        self.production.world
    }
    pub const fn production_system(&self) -> LivingEntityIdV2 {
        self.production.system
    }
    pub const fn ore_input(&self) -> u32 {
        self.production.ore_input
    }
    pub const fn alloy_output(&self) -> u32 {
        self.production.alloy_output
    }
    pub const fn delivery(&self) -> &super::event_facts::LivingDeliveryFactV2 {
        &self.delivery
    }
    pub const fn topology(&self) -> LivingNotebookSupplyTopologyV2 {
        self.topology
    }
}
/// Replay-derived agreement and matching delivery observations; never a causal dispatch claim.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LivingNotebookTradeCompletionV2 {
    formation: EvidenceReference,
    delivered: EvidenceReference,
    agreement: super::model::LivingAgreementV2,
    delivery: super::event_facts::LivingDeliveryFactV2,
}
#[derive(Clone, Debug, Eq, PartialEq)]
struct EvidenceReference {
    record: u64,
    bookmark: u64,
    view: LivingHistoryViewV2,
    event: LivingNotebookEventReferenceV2,
    revision_depth: usize,
}
#[derive(Clone, Debug, Eq, PartialEq)]
struct CorridorAttemptReference {
    record: u64,
    bookmark: u64,
    view: LivingHistoryViewV2,
    revision_depth: usize,
    locator: LivingNotebookRouteAttemptReferenceV2,
}
/// Compact replay-validated restriction anchor; never serialized.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LivingNotebookCorridorRestrictionObservationV2 {
    reference: CorridorAttemptReference,
    receipt_digest: LivingReceiptDigestV2,
    restriction: super::event_facts::LivingCorridorRestrictionV2,
}
/// Compact replay-validated MissingInput observation, with catalog recipe checks.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LivingNotebookCorridorWaitObservationV2 {
    reference: EvidenceReference,
    facility: LivingEntityIdV2,
    actor: LivingEntityIdV2,
    world: LivingEntityIdV2,
    system: LivingEntityIdV2,
    resource: super::model::LivingResourceV2,
    required: u32,
    available: u32,
}
impl LivingNotebookCorridorWaitObservationV2 {
    /// One pure candidate comparison. This establishes observation correspondence,
    /// not sole causation or completion of the full Corridor experiment.
    pub fn matches_restriction(
        &self,
        observation: &LivingNotebookCorridorRestrictionObservationV2,
    ) -> bool {
        let r = &observation.restriction;
        observation.reference.view.branch == self.reference.view.branch
            && observation.reference.revision_depth <= self.reference.revision_depth
            && observation.reference.view.tick == r.tick()
            && observation.receipt_digest == r.receipt_digest()
            && r.tick() < self.reference.view.tick
            && r.source_world() != r.destination_world()
            && r.source_system() != r.destination_system()
            && r.eligible_batch() > 0
            && r.eligible_batch() <= r.requested_batch()
            && r.restricted_batch() < r.eligible_batch()
            && r.actual_dispatched_units() == r.restricted_batch()
            && r.hazard().start_tick <= r.tick()
            && r.tick() < r.hazard().end_tick
            && r.receiver() == self.actor
            && r.destination_world() == self.world
            && r.destination_system() == self.system
            && r.resource() == self.resource
            && self.required > self.available
    }
}
/// Exact replay-derived restriction and later matching input wait observations.
/// This is neither a serialized completion flag nor a claim of sole causation.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LivingNotebookCorridorObservationPairV2 {
    attempt: LivingNotebookCorridorRestrictionObservationV2,
    wait: LivingNotebookCorridorWaitObservationV2,
}
impl LivingNotebookCorridorObservationPairV2 {
    pub const fn attempt_record(&self) -> u64 {
        self.attempt.reference.record
    }
    pub const fn attempt_bookmark(&self) -> u64 {
        self.attempt.reference.bookmark
    }
    pub const fn attempt_view(&self) -> LivingHistoryViewV2 {
        self.attempt.reference.view
    }
    pub const fn attempt_locator(&self) -> &LivingNotebookRouteAttemptReferenceV2 {
        &self.attempt.reference.locator
    }
    pub const fn restriction(&self) -> &super::event_facts::LivingCorridorRestrictionV2 {
        &self.attempt.restriction
    }
    pub const fn wait_record(&self) -> u64 {
        self.wait.reference.record
    }
    pub const fn wait_bookmark(&self) -> u64 {
        self.wait.reference.bookmark
    }
    pub const fn wait_view(&self) -> LivingHistoryViewV2 {
        self.wait.reference.view
    }
    pub const fn wait_event(&self) -> &LivingNotebookEventReferenceV2 {
        &self.wait.reference.event
    }
    pub const fn facility(&self) -> LivingEntityIdV2 {
        self.wait.facility
    }
    pub const fn world(&self) -> LivingEntityIdV2 {
        self.wait.world
    }
    pub const fn system(&self) -> LivingEntityIdV2 {
        self.wait.system
    }
    pub const fn actor(&self) -> LivingEntityIdV2 {
        self.wait.actor
    }
    pub const fn resource(&self) -> super::model::LivingResourceV2 {
        self.wait.resource
    }
    pub const fn required(&self) -> u32 {
        self.wait.required
    }
    pub const fn available(&self) -> u32 {
        self.wait.available
    }
}
struct CorridorCollection {
    authority: super::simulation::LivingGalaxyAuthorityV2,
    record: usize,
    view: LivingHistoryViewV2,
    locator: usize,
    witness: usize,
    event: usize,
    reason: usize,
}
/// Bounded pure join cursor, selecting one stable first restriction per wait.
/// Inputs must come from authenticated replay; this does not admit generic events.
pub struct LivingNotebookCorridorObservationJoinV2 {
    restrictions: Vec<LivingNotebookCorridorRestrictionObservationV2>,
    waits: Vec<LivingNotebookCorridorWaitObservationV2>,
    restriction: usize,
    wait: usize,
    pairs: Vec<(usize, usize)>,
}
impl LivingNotebookCorridorObservationJoinV2 {
    pub fn new(
        restrictions: Vec<LivingNotebookCorridorRestrictionObservationV2>,
        waits: Vec<LivingNotebookCorridorWaitObservationV2>,
    ) -> Result<Self, LivingNotebookErrorV2> {
        if restrictions.len() > 128 * 32 || waits.len() > 3 * 128 * 32 {
            return Err(LivingNotebookErrorV2::Shape);
        }
        Ok(Self {
            restrictions,
            waits,
            restriction: 0,
            wait: 0,
            pairs: Vec::new(),
        })
    }
    /// Perform at most one candidate comparison; true means the join is finished.
    pub fn advance(&mut self) -> bool {
        if self.wait == self.waits.len() || self.restrictions.is_empty() {
            return true;
        }
        if self.waits[self.wait].matches_restriction(&self.restrictions[self.restriction]) {
            self.pairs.push((self.restriction, self.wait));
            self.wait += 1;
            self.restriction = 0;
        } else {
            self.restriction += 1;
            if self.restriction == self.restrictions.len() {
                self.wait += 1;
                self.restriction = 0;
            }
        }
        self.wait == self.waits.len()
    }
    pub fn pairs(&self) -> &[(usize, usize)] {
        &self.pairs
    }
}
impl LivingNotebookTradeCompletionV2 {
    pub const fn formation_record(&self) -> u64 {
        self.formation.record
    }
    pub const fn delivery_record(&self) -> u64 {
        self.delivered.record
    }
    pub const fn formation_bookmark(&self) -> u64 {
        self.formation.bookmark
    }
    pub const fn delivery_bookmark(&self) -> u64 {
        self.delivered.bookmark
    }
    pub const fn formation_view(&self) -> LivingHistoryViewV2 {
        self.formation.view
    }
    pub const fn delivery_view(&self) -> LivingHistoryViewV2 {
        self.delivered.view
    }
    pub fn formation_event(&self) -> &LivingNotebookEventReferenceV2 {
        &self.formation.event
    }
    pub fn delivery_event(&self) -> &LivingNotebookEventReferenceV2 {
        &self.delivered.event
    }
    pub const fn agreement(&self) -> &super::model::LivingAgreementV2 {
        &self.agreement
    }
    pub const fn delivery(&self) -> &super::event_facts::LivingDeliveryFactV2 {
        &self.delivery
    }
}
struct TradeFormation {
    reference: EvidenceReference,
    agreement: super::model::LivingAgreementV2,
}
struct TradeDelivery {
    reference: EvidenceReference,
    delivery: super::event_facts::LivingDeliveryFactV2,
    agreement: Option<super::model::LivingAgreementV2>,
}
impl ValidatedLivingNotebookV2 {
    /// Exact restriction and matching later input wait, without a causal completion claim.
    pub fn corridor_observations(&self) -> &[LivingNotebookCorridorObservationPairV2] {
        &self.corridor_observations
    }
    /// Actual alloy production plus a matching inter-system resource transfer.
    pub fn supply_completions(&self) -> &[LivingNotebookSupplyCompletionV2] {
        &self.supply_completions
    }
    /// Exact linked observations derived from this validated retained history.
    /// This does not prove the agreement caused cargo dispatch.
    pub fn trade_completions(&self) -> &[LivingNotebookTradeCompletionV2] {
        &self.trade_completions
    }
    /// Replay-validated portable metadata.
    pub fn notebook(&self) -> &LivingNotebookV2 {
        &self.notebook
    }
    /// Verify the retained immutable document, independently of its selected view.
    /// A historical Continue view can share this proof with its newer retained head.
    /// An extended or different document requires fresh notebook validation.
    pub fn matches_retained_history(
        &self,
        authority: &super::simulation::LivingGalaxyAuthorityV2,
    ) -> Result<bool, LivingNotebookErrorV2> {
        if authority.published_cursor().pending_sequence != 0 {
            return Ok(false);
        }
        let (revisions, branches) = authority
            .canonical_history()
            .map_err(LivingArchiveErrorV2::from)?;
        Ok(revisions == self.source_revisions && branches == self.source_branches)
    }
    /// Transfer validated metadata to a resident presentation owner.
    pub fn into_notebook(self) -> LivingNotebookV2 {
        self.notebook
    }
}
struct Task {
    view: LivingHistoryViewV2,
    digest: Option<LivingStateDigestV2>,
    evidence: Option<usize>,
}
/// Bounded, cancellable reconstruction. Each poll advances one archive task.
pub struct LivingNotebookReplayV2 {
    notebook: LivingNotebookV2,
    archive: ValidatedLivingArchiveV2,
    tasks: Vec<Task>,
    completed: Vec<LivingGalaxyStateV2>,
    task_cursor: usize,
    replay: Option<LivingArchiveReplayV2>,
    cancelled: bool,
    failure: Option<LivingNotebookErrorV2>,
    ready: bool,
    experiment_cursor: usize,
    observation_cursor: usize,
    trade_formations: Vec<TradeFormation>,
    trade_deliveries: Vec<TradeDelivery>,
    trade_completions: Vec<LivingNotebookTradeCompletionV2>,
    trade_delivery_cursor: usize,
    trade_formation_cursor: usize,
    supply_production: Vec<SupplyProduction>,
    supply_deliveries: Vec<SupplyDelivery>,
    supply_completions: Vec<LivingNotebookSupplyCompletionV2>,
    corridor_observations: Vec<LivingNotebookCorridorObservationPairV2>,
    supply_delivery_cursor: usize,
    supply_production_cursor: usize,
    corridor_collection: Option<CorridorCollection>,
    corridor_restrictions: Vec<LivingNotebookCorridorRestrictionObservationV2>,
    corridor_waits: Vec<LivingNotebookCorridorWaitObservationV2>,
    corridor_join: Option<LivingNotebookCorridorObservationJoinV2>,
    corridor_materialized: usize,
    corridor_join_complete: bool,
    corridor_done: bool,
}
/// Decode canonical metadata and check its exact archive attachment before replay.
pub fn begin_decode_living_notebook_v2(
    bytes: &[u8],
    archive: ValidatedLivingArchiveV2,
) -> Result<LivingNotebookReplayV2, LivingNotebookErrorV2> {
    let notebook: LivingNotebookV2 = decode_canonical_v2(bytes, LIVING_MAX_NOTEBOOK_BYTES_V2)?;
    notebook.validate_shape(&archive)?;
    let mut tasks: Vec<_> = notebook
        .bookmarks
        .iter()
        .map(|x| Task {
            view: x.view,
            digest: Some(x.digest),
            evidence: None,
        })
        .collect();
    for experiment in &notebook.experiments {
        for ordinal in [experiment.left, experiment.right] {
            let bookmark = notebook
                .bookmarks
                .iter()
                .find(|x| x.ordinal == ordinal)
                .ok_or(LivingNotebookErrorV2::Shape)?;
            tasks.push(Task {
                view: LivingHistoryViewV2 {
                    tick: experiment.comparison_tick,
                    ..bookmark.view
                },
                digest: None,
                evidence: None,
            });
        }
    }
    if let Some(section) = &notebook.evidence {
        for (index, record) in section.records.iter().enumerate() {
            let bookmark = notebook
                .bookmarks
                .iter()
                .find(|bookmark| bookmark.ordinal == record.bookmark)
                .ok_or(LivingNotebookErrorV2::Shape)?;
            tasks.push(Task {
                view: bookmark.view,
                digest: Some(bookmark.digest),
                evidence: Some(index),
            });
        }
    }
    Ok(LivingNotebookReplayV2 {
        notebook,
        archive,
        tasks,
        completed: Vec::new(),
        task_cursor: 0,
        replay: None,
        cancelled: false,
        failure: None,
        ready: false,
        experiment_cursor: 0,
        observation_cursor: 0,
        trade_formations: Vec::new(),
        trade_deliveries: Vec::new(),
        trade_completions: Vec::new(),
        trade_delivery_cursor: 0,
        trade_formation_cursor: 0,
        supply_production: Vec::new(),
        supply_deliveries: Vec::new(),
        supply_completions: Vec::new(),
        supply_delivery_cursor: 0,
        supply_production_cursor: 0,
        corridor_observations: Vec::new(),
        corridor_collection: None,
        corridor_restrictions: Vec::new(),
        corridor_waits: Vec::new(),
        corridor_join: None,
        corridor_materialized: 0,
        corridor_join_complete: false,
        corridor_done: false,
    })
}
impl LivingNotebookReplayV2 {
    /// Cancel without touching the source authority.
    pub fn cancel(&mut self) {
        self.cancelled = true;
        self.corridor_collection = None;
        self.replay = None;
        self.completed.clear();
        self.trade_formations.clear();
        self.trade_deliveries.clear();
        self.trade_completions.clear();
        self.supply_production.clear();
        self.supply_deliveries.clear();
        self.supply_completions.clear();
    }
    /// Uses the existing bounded archive replay work contract.
    pub fn poll(
        &mut self,
        budget: usize,
    ) -> Result<LivingArchiveReplayStatusV2, LivingNotebookErrorV2> {
        if self.cancelled {
            return Err(LivingNotebookErrorV2::Cancelled);
        }
        if let Some(error) = &self.failure {
            return Err(error.clone());
        }
        let result = self.poll_work(budget);
        if let Err(error) = &result
            && *error != LivingNotebookErrorV2::Replay(LivingArchiveErrorV2::Budget)
        {
            self.failure = Some(error.clone());
            self.corridor_collection = None;
            self.replay = None;
            self.completed.clear();
            self.trade_formations.clear();
            self.trade_deliveries.clear();
            self.trade_completions.clear();
            self.supply_production.clear();
            self.supply_deliveries.clear();
            self.supply_completions.clear();
        }
        result
    }
    fn poll_work(
        &mut self,
        budget: usize,
    ) -> Result<LivingArchiveReplayStatusV2, LivingNotebookErrorV2> {
        if budget == 0 || budget > super::archive::LIVING_MAX_REPLAY_POLL_WORK_V2 {
            return Err(LivingArchiveErrorV2::Budget.into());
        }
        if self.ready {
            return Ok(LivingArchiveReplayStatusV2::Ready);
        }
        if self.corridor_collection.is_some() {
            for _ in 0..budget {
                if self.collect_corridor_unit()? {
                    break;
                }
            }
            return Ok(LivingArchiveReplayStatusV2::Pending);
        }
        if let Some(task) = self.tasks.get(self.task_cursor) {
            if self.replay.is_none() {
                self.replay = Some(begin_restore_living_view_v2(
                    self.archive.authority(),
                    task.view,
                )?);
            }
            let replay = self.replay.as_mut().ok_or(LivingNotebookErrorV2::Shape)?;
            if replay.poll(budget)? == LivingArchiveReplayStatusV2::Pending {
                return Ok(LivingArchiveReplayStatusV2::Pending);
            }
            let authority = self
                .replay
                .take()
                .ok_or(LivingNotebookErrorV2::Shape)?
                .finish()?
                .into_authority();
            if task
                .digest
                .is_some_and(|digest| authority.state().digest().ok() != Some(digest))
            {
                return Err(LivingNotebookErrorV2::Evidence);
            }
            if let Some(index) = task.evidence {
                let record = &self
                    .notebook
                    .evidence
                    .as_ref()
                    .ok_or(LivingNotebookErrorV2::Shape)?
                    .records[index];
                if record.experiment == LivingNotebookExperimentKindV2::AutonomousSettlement
                    && !authority.revisions().is_empty()
                {
                    return Err(LivingNotebookErrorV2::Evidence);
                }
                let receipt = authority
                    .last_receipt()
                    .ok_or(LivingNotebookErrorV2::Evidence)?;
                verify_notebook_event_reference(
                    record,
                    receipt,
                    task.view.tick,
                    task.digest.ok_or(LivingNotebookErrorV2::Shape)?,
                    &authority,
                )?;
                if record.experiment == LivingNotebookExperimentKindV2::CorridorImpact {
                    self.corridor_collection = Some(CorridorCollection {
                        authority,
                        record: index,
                        view: task.view,
                        locator: 0,
                        witness: 0,
                        event: 0,
                        reason: 0,
                    });
                    return Ok(LivingArchiveReplayStatusV2::Pending);
                }
                if record.experiment == LivingNotebookExperimentKindV2::TradeDelivery {
                    let sidecar = authority
                        .last_event_facts()
                        .ok_or(LivingNotebookErrorV2::Evidence)?;
                    for event in &record.events {
                        let facts = &sidecar
                            .events
                            .iter()
                            .find(|entry| entry.ordinal == event.ordinal)
                            .ok_or(LivingNotebookErrorV2::Evidence)?
                            .facts;
                        let reference = EvidenceReference {
                            record: record.ordinal,
                            bookmark: record.bookmark,
                            view: task.view,
                            event: event.clone(),
                            revision_depth: authority.revisions().len(),
                        };
                        if let Some(agreement) = facts.agreement {
                            if authority.state().agreements.as_slice().contains(&agreement) {
                                self.trade_formations.push(TradeFormation {
                                    reference,
                                    agreement,
                                });
                            }
                        } else if let Some(delivery) = &facts.delivery {
                            let a = delivery.dispatch_owner.min(delivery.recipient);
                            let b = delivery.dispatch_owner.max(delivery.recipient);
                            let agreement = authority
                                .state()
                                .agreements
                                .as_slice()
                                .iter()
                                .find(|agreement| {
                                    agreement.kind == super::model::LivingAgreementKindV2::Trade
                                        && agreement.participant_a == a
                                        && agreement.participant_b == b
                                        && agreement.start_tick <= task.view.tick
                                        && task.view.tick < agreement.end_tick
                                })
                                .copied();
                            self.trade_deliveries.push(TradeDelivery {
                                reference,
                                delivery: delivery.clone(),
                                agreement,
                            });
                        }
                    }
                }
                if record.experiment == LivingNotebookExperimentKindV2::SupplyChain {
                    let sidecar = authority
                        .last_event_facts()
                        .ok_or(LivingNotebookErrorV2::Evidence)?;
                    for event in &record.events {
                        let facts = &sidecar
                            .events
                            .iter()
                            .find(|entry| entry.ordinal == event.ordinal)
                            .ok_or(LivingNotebookErrorV2::Evidence)?
                            .facts;
                        let reference = EvidenceReference {
                            record: record.ordinal,
                            bookmark: record.bookmark,
                            view: task.view,
                            event: event.clone(),
                            revision_depth: authority.revisions().len(),
                        };
                        if event.kind == LivingEventKindV2::FacilityProduced {
                            self.supply_production.push(
                                supply_production(&authority, facts, reference)
                                    .ok_or(LivingNotebookErrorV2::Evidence)?,
                            );
                        } else if let Some(delivery) = &facts.delivery {
                            self.supply_deliveries.push(SupplyDelivery {
                                reference,
                                delivery: delivery.clone(),
                            });
                        }
                    }
                }
            }
            if task.evidence.is_none() {
                self.completed.push(authority.state().clone());
            }
            self.task_cursor += 1;
            return Ok(LivingArchiveReplayStatusV2::Pending);
        }
        for _ in 0..budget {
            if let Some(delivered) = self.trade_deliveries.get(self.trade_delivery_cursor) {
                if let Some(formation) = self.trade_formations.get(self.trade_formation_cursor) {
                    let matched = formation.reference.view.branch
                        == delivered.reference.view.branch
                        && formation.reference.view.tick < delivered.reference.view.tick
                        && formation.reference.revision_depth <= delivered.reference.revision_depth
                        && delivered.agreement == Some(formation.agreement);
                    self.trade_formation_cursor += 1;
                    if matched {
                        self.trade_completions
                            .push(LivingNotebookTradeCompletionV2 {
                                formation: formation.reference.clone(),
                                delivered: delivered.reference.clone(),
                                agreement: formation.agreement,
                                delivery: delivered.delivery.clone(),
                            });
                        self.trade_delivery_cursor += 1;
                        self.trade_formation_cursor = 0;
                    }
                } else {
                    self.trade_delivery_cursor += 1;
                    self.trade_formation_cursor = 0;
                }
                continue;
            }
            if let Some(delivered) = self.supply_deliveries.get(self.supply_delivery_cursor) {
                if let Some(production) = self.supply_production.get(self.supply_production_cursor)
                {
                    self.supply_production_cursor += 1;
                    if let Some(topology) = supply_link(production, delivered) {
                        self.supply_completions
                            .push(LivingNotebookSupplyCompletionV2 {
                                production: production.clone(),
                                delivered: delivered.reference.clone(),
                                delivery: delivered.delivery.clone(),
                                topology,
                            });
                        self.supply_delivery_cursor += 1;
                        self.supply_production_cursor = 0;
                    }
                } else {
                    self.supply_delivery_cursor += 1;
                    self.supply_production_cursor = 0;
                }
                continue;
            }
            if !self.corridor_done {
                self.advance_corridor_join()?;
                continue;
            }
            let Some(experiment) = self.notebook.experiments.get(self.experiment_cursor) else {
                self.ready = true;
                return Ok(LivingArchiveReplayStatusV2::Ready);
            };
            let left = &self.completed[self.notebook.bookmarks.len() + self.experiment_cursor * 2];
            let right =
                &self.completed[self.notebook.bookmarks.len() + self.experiment_cursor * 2 + 1];
            let observation = &experiment.observations[self.observation_cursor];
            if measure(left, observation.metric, observation.entity)? != observation.left
                || measure(right, observation.metric, observation.entity)? != observation.right
            {
                return Err(LivingNotebookErrorV2::Evidence);
            }
            self.observation_cursor += 1;
            if self.observation_cursor == experiment.observations.len() {
                self.observation_cursor = 0;
                self.experiment_cursor += 1;
            }
        }
        if self.experiment_cursor != self.notebook.experiments.len() {
            return Ok(LivingArchiveReplayStatusV2::Pending);
        }
        if self.trade_delivery_cursor != self.trade_deliveries.len() {
            return Ok(LivingArchiveReplayStatusV2::Pending);
        }
        if self.supply_delivery_cursor != self.supply_deliveries.len() {
            return Ok(LivingArchiveReplayStatusV2::Pending);
        }
        if !self.corridor_done {
            return Ok(LivingArchiveReplayStatusV2::Pending);
        }
        self.ready = true;
        Ok(LivingArchiveReplayStatusV2::Ready)
    }
    // One witness comparison or one blocked-reason extraction per call.
    fn collect_corridor_unit(&mut self) -> Result<bool, LivingNotebookErrorV2> {
        let c = self
            .corridor_collection
            .as_mut()
            .ok_or(LivingNotebookErrorV2::Shape)?;
        let record = &self
            .notebook
            .evidence
            .as_ref()
            .ok_or(LivingNotebookErrorV2::Shape)?
            .records[c.record];
        if let Some(locator) = record
            .route_attempts
            .as_deref()
            .unwrap_or_default()
            .get(c.locator)
        {
            let w = c
                .authority
                .last_corridor_restrictions()
                .get(c.witness)
                .ok_or(LivingNotebookErrorV2::Evidence)?;
            c.witness += 1;
            if w.tick() == c.view.tick
                && w.receipt_digest() == record.receipt_digest
                && w.route() == locator.route
                && w.hazard().id == locator.hazard
                && w.eligible_batch() > 0
                && w.restricted_batch() < w.eligible_batch()
                && w.actual_dispatched_units() == w.restricted_batch()
            {
                self.corridor_restrictions
                    .push(LivingNotebookCorridorRestrictionObservationV2 {
                        reference: CorridorAttemptReference {
                            record: record.ordinal,
                            bookmark: record.bookmark,
                            view: c.view,
                            revision_depth: c.authority.revisions().len(),
                            locator: locator.clone(),
                        },
                        receipt_digest: record.receipt_digest,
                        restriction: w.clone(),
                    });
                c.locator += 1;
                c.witness = 0;
            }
            return Ok(false);
        }
        if let Some(reference) = record.events.get(c.event) {
            let sidecar = c
                .authority
                .last_event_facts()
                .ok_or(LivingNotebookErrorV2::Evidence)?;
            let index = sidecar
                .events
                .binary_search_by_key(&reference.ordinal, |entry| entry.ordinal)
                .map_err(|_| LivingNotebookErrorV2::Evidence)?;
            let facts = &sidecar.events[index].facts;
            let facility_id = facts.entity.ok_or(LivingNotebookErrorV2::Evidence)?;
            let facilities = c.authority.state().facilities.as_slice();
            let facility = &facilities[facilities
                .binary_search_by_key(&facility_id, |row| row.id)
                .map_err(|_| LivingNotebookErrorV2::Evidence)?];
            let worlds = c.authority.state().worlds.as_slice();
            let world = &worlds[worlds
                .binary_search_by_key(&facility.world, |row| row.id)
                .map_err(|_| LivingNotebookErrorV2::Evidence)?];
            let definition = c
                .authority
                .catalog()
                .industry(facility.definition.as_str())
                .ok_or(LivingNotebookErrorV2::Evidence)?;
            let super::catalog::LivingIndustryEffectV2::Recipe { inputs, .. } = &definition.effect
            else {
                return Err(LivingNotebookErrorV2::Evidence);
            };
            if let Some(reason) = facts.blocked_reasons.get(c.reason) {
                c.reason += 1;
                if let super::model::LivingBlockedReasonV2::MissingInput {
                    resource,
                    required,
                    available,
                } = reason
                    && required > available
                    && inputs.as_slice().iter().any(|input| {
                        input.resource.as_str() == resource.as_wire_str()
                            && input.quantity == *required
                    })
                {
                    // At most three catalog-confirmed resource waits per event; each reason is counted.
                    self.corridor_waits
                        .push(LivingNotebookCorridorWaitObservationV2 {
                            reference: EvidenceReference {
                                record: record.ordinal,
                                bookmark: record.bookmark,
                                view: c.view,
                                event: reference.clone(),
                                revision_depth: c.authority.revisions().len(),
                            },
                            facility: facility_id,
                            actor: facts.actor.ok_or(LivingNotebookErrorV2::Evidence)?,
                            world: world.id,
                            system: world.system,
                            resource: *resource,
                            required: *required,
                            available: *available,
                        });
                }
                return Ok(false);
            }
            c.event += 1;
            c.reason = 0;
            return Ok(false);
        }
        // The authority is temporary, not an additional retained full evidence state.
        self.corridor_collection = None;
        self.task_cursor += 1;
        Ok(true)
    }
    fn advance_corridor_join(&mut self) -> Result<(), LivingNotebookErrorV2> {
        if self.corridor_join.is_none() {
            self.corridor_join = Some(LivingNotebookCorridorObservationJoinV2::new(
                std::mem::take(&mut self.corridor_restrictions),
                std::mem::take(&mut self.corridor_waits),
            )?);
            return Ok(());
        }
        let join = self
            .corridor_join
            .as_mut()
            .ok_or(LivingNotebookErrorV2::Shape)?;
        if !self.corridor_join_complete {
            self.corridor_join_complete = join.advance();
            return Ok(());
        }
        if let Some(&(restriction, wait)) = join.pairs.get(self.corridor_materialized) {
            self.corridor_observations
                .push(LivingNotebookCorridorObservationPairV2 {
                    attempt: join.restrictions[restriction].clone(),
                    wait: join.waits[wait].clone(),
                });
            self.corridor_materialized += 1;
            return Ok(());
        }
        // Count each compact temporary element's retirement after all proof indices are consumed.
        if join.pairs.pop().is_some() {
            return Ok(());
        }
        if join.restrictions.pop().is_some() {
            return Ok(());
        }
        if join.waits.pop().is_some() {
            return Ok(());
        }
        self.corridor_join = None;
        self.corridor_done = true;
        Ok(())
    }
    /// Only succeeds after every exact reference and observation validates.
    pub fn finish(self) -> Result<ValidatedLivingNotebookV2, LivingNotebookErrorV2> {
        if self.cancelled {
            return Err(LivingNotebookErrorV2::Cancelled);
        }
        if let Some(error) = self.failure {
            return Err(error);
        }
        if !self.ready {
            return Err(LivingNotebookErrorV2::Evidence);
        }
        Ok(ValidatedLivingNotebookV2 {
            notebook: self.notebook,
            source_revisions: self.archive.archive().revisions.as_slice().to_vec(),
            source_branches: self.archive.archive().branches.as_slice().to_vec(),
            trade_completions: self.trade_completions,
            supply_completions: self.supply_completions,
            corridor_observations: self.corridor_observations,
        })
    }
}
fn integer(value: u64) -> Result<LivingNotebookValueV2, LivingNotebookErrorV2> {
    let value = i64::try_from(value).map_err(|_| LivingNotebookErrorV2::Shape)?;
    Ok(LivingNotebookValueV2::Integer(LivingDecisionScoreV2(value)))
}
/// Measure one accepted metric with explicit scope against a private replay output.
pub fn measure_living_notebook_v2(
    state: &LivingGalaxyStateV2,
    metric: LivingNotebookMetricV2,
    entity: Option<LivingEntityIdV2>,
) -> Result<LivingNotebookValueV2, LivingNotebookErrorV2> {
    measure(state, metric, entity)
}
fn measure(
    state: &LivingGalaxyStateV2,
    metric: LivingNotebookMetricV2,
    entity: Option<LivingEntityIdV2>,
) -> Result<LivingNotebookValueV2, LivingNotebookErrorV2> {
    use LivingNotebookMetricV2 as M;
    match metric {
        M::InventoryEnergy | M::InventoryOre | M::InventoryAlloy | M::Owner => {
            let id = entity.ok_or(LivingNotebookErrorV2::Shape)?;
            let Some(world) = state.worlds.as_slice().iter().find(|x| x.id == id) else {
                return Ok(LivingNotebookValueV2::Absent);
            };
            match metric {
                M::InventoryEnergy => integer(u64::from(world.inventory.energy)),
                M::InventoryOre => integer(u64::from(world.inventory.ore)),
                M::InventoryAlloy => integer(u64::from(world.inventory.alloy)),
                M::Owner => Ok(LivingNotebookValueV2::Entity(world.owner)),
                _ => Err(LivingNotebookErrorV2::Shape),
            }
        }
        M::Tick => {
            if entity.is_some() {
                return Err(LivingNotebookErrorV2::Shape);
            }
            integer(state.tick.0)
        }
        M::ColonyCount | M::FacilityCount | M::HullCount => {
            if entity.is_some_and(|id| !state.civilizations.as_slice().iter().any(|x| x.id == id)) {
                return Ok(LivingNotebookValueV2::Absent);
            }
            let count = match metric {
                M::ColonyCount => state
                    .colonies
                    .as_slice()
                    .iter()
                    .filter(|x| entity.is_none_or(|id| x.owner == id))
                    .count(),
                M::FacilityCount => state
                    .facilities
                    .as_slice()
                    .iter()
                    .filter(|x| {
                        entity.is_none_or(|id| {
                            state
                                .worlds
                                .as_slice()
                                .iter()
                                .any(|world| world.id == x.world && world.owner == Some(id))
                        })
                    })
                    .count(),
                M::HullCount => state
                    .fleets
                    .as_slice()
                    .iter()
                    .filter(|x| entity.is_none_or(|id| x.owner == id))
                    .map(|x| x.hulls.as_slice().len())
                    .sum(),
                _ => return Err(LivingNotebookErrorV2::Shape),
            };
            integer(count as u64)
        }
    }
}

#[derive(Clone, Copy)]
struct SupplyProductionFacts {
    actor: Option<LivingEntityIdV2>,
    facility: LivingEntityIdV2,
    world: LivingEntityIdV2,
    system: LivingEntityIdV2,
    ore_input: u32,
    alloy_output: u32,
}
fn supply_production_facts(
    authority: &super::simulation::LivingGalaxyAuthorityV2,
    facts: &super::event_facts::LivingEventFactsV2,
) -> Option<SupplyProductionFacts> {
    use super::event_facts::LivingEventResourceRoleV2 as Role;
    use super::model::LivingResourceV2 as Resource;
    let facility = facts.entity?;
    let facilities = authority.state().facilities.as_slice();
    let facility_row = &facilities[facilities
        .binary_search_by_key(&facility, |row| row.id)
        .ok()?];
    let worlds = authority.state().worlds.as_slice();
    let world = &worlds[worlds
        .binary_search_by_key(&facility_row.world, |row| row.id)
        .ok()?];
    if facts.actor != world.owner {
        return None;
    }
    let quantity = |role, resource| {
        facts
            .resources
            .iter()
            .find(|fact| fact.role == role && fact.resource == resource)
            .map(|fact| fact.quantity)
            .filter(|quantity| *quantity > 0)
    };
    Some(SupplyProductionFacts {
        actor: facts.actor,
        facility,
        world: world.id,
        system: world.system,
        ore_input: quantity(Role::Input, Resource::Ore)?,
        alloy_output: quantity(Role::Output, Resource::Alloy)?,
    })
}
fn supply_production(
    authority: &super::simulation::LivingGalaxyAuthorityV2,
    facts: &super::event_facts::LivingEventFactsV2,
    reference: EvidenceReference,
) -> Option<SupplyProduction> {
    let facts = supply_production_facts(authority, facts)?;
    Some(SupplyProduction {
        reference,
        actor: facts.actor,
        facility: facts.facility,
        world: facts.world,
        system: facts.system,
        ore_input: facts.ore_input,
        alloy_output: facts.alloy_output,
    })
}
fn supply_delivery_supported(delivery: &super::event_facts::LivingDeliveryFactV2) -> bool {
    delivery.source_world != delivery.destination_world
        && delivery.units > 0
        && delivery.recipient == delivery.intended_receiver
        && matches!(
            delivery.resource,
            super::model::LivingResourceV2::Ore | super::model::LivingResourceV2::Alloy
        )
        && delivery
            .source_system
            .zip(delivery.destination_system)
            .is_some_and(|(a, b)| a != b)
}
fn supply_link(
    production: &SupplyProduction,
    delivered: &SupplyDelivery,
) -> Option<LivingNotebookSupplyTopologyV2> {
    use super::model::LivingResourceV2 as Resource;
    if !supply_delivery_supported(&delivered.delivery)
        || production.reference.view.branch != delivered.reference.view.branch
    {
        return None;
    }
    let production_order = (
        production.reference.view.tick,
        production.reference.event.ordinal,
    );
    let delivery_order = (
        delivered.reference.view.tick,
        delivered.reference.event.ordinal,
    );
    match delivered.delivery.resource {
        Resource::Alloy
            if production_order < delivery_order
                && production.reference.revision_depth <= delivered.reference.revision_depth
                && production.actor == Some(delivered.delivery.dispatch_owner)
                && production.world == delivered.delivery.source_world
                && Some(production.system) == delivered.delivery.source_system =>
        {
            Some(LivingNotebookSupplyTopologyV2::AlloyDelivered)
        }
        Resource::Ore
            if delivery_order < production_order
                && delivered.reference.revision_depth <= production.reference.revision_depth
                && production.actor == Some(delivered.delivery.recipient)
                && production.world == delivered.delivery.destination_world
                && Some(production.system) == delivered.delivery.destination_system =>
        {
            Some(LivingNotebookSupplyTopologyV2::OreDeliveredToAlloyProducer)
        }
        _ => None,
    }
}
/// Validate one actual Supply observation at the authority's current receipt.
/// This does not certify a complete linked workflow or cargo causation.
pub fn supports_living_supply_observation_v2(
    authority: &super::simulation::LivingGalaxyAuthorityV2,
    receipt: &LivingTickReceiptV2,
    event: &super::receipt::LivingEventV2,
) -> bool {
    if authority.last_receipt() != Some(receipt)
        || receipt.events.get(usize::from(event.ordinal)) != Some(event)
        || !matches!(event.provenance, LivingEventProvenanceV2::Autonomous { .. })
    {
        return false;
    }
    let Some(sidecar) = authority.last_event_facts() else {
        return false;
    };
    // Published authority sidecars have already been sealed and verified. The
    // readonly presentation predicate binds that verified boundary without
    // serializing its entire sidecar or hashing the complete state per event.
    if sidecar.tick != receipt.tick
        || sidecar.state_digest != receipt.state_digest
        || sidecar.receipt_digest != receipt.digest
    {
        return false;
    }
    let Ok(index) = sidecar
        .events
        .binary_search_by_key(&event.ordinal, |entry| entry.ordinal)
    else {
        return false;
    };
    let entry = &sidecar.events[index];
    if entry.event_id != event.id {
        return false;
    }
    match event.kind {
        LivingEventKindV2::FacilityProduced => {
            supply_production_facts(authority, &entry.facts).is_some()
        }
        LivingEventKindV2::ShipmentDelivered => entry
            .facts
            .delivery
            .as_ref()
            .is_some_and(supply_delivery_supported),
        _ => false,
    }
}
/// One current authoritative industry input wait; no inferred corridor cause.
pub fn supports_living_corridor_wait_observation_v2(
    authority: &super::simulation::LivingGalaxyAuthorityV2,
    receipt: &LivingTickReceiptV2,
    event: &super::receipt::LivingEventV2,
) -> bool {
    if authority.last_receipt() != Some(receipt)
        || receipt.events.get(usize::from(event.ordinal)) != Some(event)
        || event.kind != LivingEventKindV2::FacilityBlocked
        || !matches!(
            event.provenance,
            LivingEventProvenanceV2::Autonomous {
                phase: super::ids::LivingPhaseV2::RunProduction
            }
        )
    {
        return false;
    }
    let Some(sidecar) = authority.last_event_facts() else {
        return false;
    };
    if sidecar.tick != receipt.tick
        || sidecar.receipt_digest != receipt.digest
        || sidecar.state_digest != receipt.state_digest
    {
        return false;
    }
    let Ok(index) = sidecar
        .events
        .binary_search_by_key(&event.ordinal, |entry| entry.ordinal)
    else {
        return false;
    };
    let entry = &sidecar.events[index];
    if entry.event_id != event.id {
        return false;
    }
    let Some(facility_id) = entry.facts.entity else {
        return false;
    };
    let facilities = authority.state().facilities.as_slice();
    let Ok(index) = facilities.binary_search_by_key(&facility_id, |row| row.id) else {
        return false;
    };
    let facility = &facilities[index];
    let worlds = authority.state().worlds.as_slice();
    let Ok(index) = worlds.binary_search_by_key(&facility.world, |row| row.id) else {
        return false;
    };
    let world = &worlds[index];
    if entry.facts.actor.is_none() || entry.facts.actor != world.owner {
        return false;
    }
    let Some(definition) = authority.catalog().industry(facility.definition.as_str()) else {
        return false;
    };
    let super::catalog::LivingIndustryEffectV2::Recipe { inputs, .. } = &definition.effect else {
        return false;
    };
    entry.facts.blocked_reasons.iter().any(|reason| {
        matches!(reason, super::model::LivingBlockedReasonV2::MissingInput {resource, required, available}
            if required > available && inputs.as_slice().iter().any(|input| input.resource.as_str() == resource.as_wire_str() && input.quantity == *required))
    })
}
fn verify_notebook_event_reference(
    record: &LivingNotebookEvidenceRecordV2,
    receipt: &LivingTickReceiptV2,
    tick: LivingTickV2,
    digest: LivingStateDigestV2,
    authority: &super::simulation::LivingGalaxyAuthorityV2,
) -> Result<(), LivingNotebookErrorV2> {
    if receipt.tick != tick
        || receipt.state_digest != digest
        || receipt.digest != record.receipt_digest
    {
        return Err(LivingNotebookErrorV2::Evidence);
    }
    if matches!(
        record.experiment,
        LivingNotebookExperimentKindV2::SupplyChain
            | LivingNotebookExperimentKindV2::CorridorImpact
    ) && !record.events.is_empty()
    {
        authority
            .last_event_facts()
            .ok_or(LivingNotebookErrorV2::Evidence)?
            .verify_receipt(receipt)
            .map_err(|_| LivingNotebookErrorV2::Evidence)?;
    }
    for reference in &record.events {
        let event = receipt
            .events
            .get(usize::from(reference.ordinal))
            .ok_or(LivingNotebookErrorV2::Evidence)?;
        if event.ordinal != reference.ordinal
            || event.id != reference.id
            || event.kind != reference.kind
        {
            return Err(LivingNotebookErrorV2::Evidence);
        }
        use LivingEventKindV2 as E;
        use LivingNotebookExperimentKindV2 as X;
        let autonomous = matches!(event.provenance, LivingEventProvenanceV2::Autonomous { .. });
        match record.experiment {
            X::CorridorImpact => {
                if !supports_living_corridor_wait_observation_v2(authority, receipt, event) {
                    return Err(LivingNotebookErrorV2::Evidence);
                }
            }
            X::SupplyChain => {
                if !supports_living_supply_observation_v2(authority, receipt, event) {
                    return Err(LivingNotebookErrorV2::Evidence);
                }
            }
            X::AutonomousSettlement
                if !autonomous
                    || event.kind != E::SettlementClaimResolved
                    || !receipt.applied_revisions.is_empty() =>
            {
                return Err(LivingNotebookErrorV2::Evidence);
            }
            X::DormantRecovery => {
                match event.provenance {
                    LivingEventProvenanceV2::Autonomous { .. }
                        if event.kind == E::CivilizationReactivated => {}
                    LivingEventProvenanceV2::Creator { revision }
                        if event.kind == E::CreatorIntervention
                            && authority.last_creator_recoveries().iter().any(|proof| {
                                proof.revision() == revision
                                    && proof.receipt_digest() == receipt.digest
                                    && authority.state().civilizations.as_slice().iter().any(
                                        |actor| {
                                            actor.id == proof.actor()
                                        && actor.status
                                            == super::model::LivingCivilizationStatusV2::Active
                                        },
                                    )
                                    && authority.state().colonies.as_slice().iter().any(|colony| {
                                        colony.world == proof.world()
                                            && colony.owner == proof.actor()
                                    })
                            }) => {}
                    _ => return Err(LivingNotebookErrorV2::Evidence),
                }
            }
            X::TradeDelivery => {
                let sidecar = authority
                    .last_event_facts()
                    .ok_or(LivingNotebookErrorV2::Evidence)?;
                sidecar
                    .verify_receipt(receipt)
                    .map_err(|_| LivingNotebookErrorV2::Evidence)?;
                let facts = &sidecar
                    .events
                    .iter()
                    .find(|entry| entry.ordinal == event.ordinal && entry.event_id == event.id)
                    .ok_or(LivingNotebookErrorV2::Evidence)?
                    .facts;
                let supported = autonomous
                    && match event.kind {
                        E::AgreementFormed => facts.agreement.as_ref().is_some_and(|agreement| {
                            agreement.kind == super::model::LivingAgreementKindV2::Trade
                                && agreement.participant_a < agreement.participant_b
                                && agreement.start_tick == receipt.tick
                                && agreement.end_tick > agreement.start_tick
                        }),
                        E::ShipmentDelivered => facts.delivery.as_ref().is_some_and(|delivery| {
                            delivery.dispatch_kind == Some(super::model::LivingRouteKindV2::Trade)
                                && delivery.source_world != delivery.destination_world
                                && delivery.dispatch_owner != delivery.recipient
                                && delivery.recipient == delivery.intended_receiver
                                && delivery.units > 0
                        }),
                        _ => false,
                    };
                if !supported {
                    return Err(LivingNotebookErrorV2::Evidence);
                }
            }
            _ => {}
        }
    }
    Ok(())
}

#[cfg(test)]
mod supply_link_tests {
    use super::super::{
        event_facts::LivingDeliveryFactV2, ids::LivingBranchIdV2, model::LivingResourceV2,
    };
    use super::*;
    fn reference(tick: u64, ordinal: u16) -> EvidenceReference {
        EvidenceReference {
            record: 1,
            bookmark: 1,
            view: LivingHistoryViewV2 {
                branch: LivingBranchIdV2([7; 16]),
                revision: None,
                tick: LivingTickV2(tick),
            },
            event: LivingNotebookEventReferenceV2 {
                ordinal,
                id: LivingEventIdV2([8; 16]),
                kind: LivingEventKindV2::FacilityProduced,
            },
            revision_depth: 0,
        }
    }
    #[test]
    fn unknown_systems_and_mismatched_actor_or_world_never_link() {
        let production = SupplyProduction {
            reference: reference(1, 0),
            actor: Some(LivingEntityIdV2([1; 16])),
            facility: LivingEntityIdV2([9; 16]),
            world: LivingEntityIdV2([2; 16]),
            system: LivingEntityIdV2([3; 16]),
            ore_input: 4,
            alloy_output: 2,
        };
        let delivery = LivingDeliveryFactV2 {
            source_world: production.world,
            destination_world: LivingEntityIdV2([4; 16]),
            dispatch_owner: production.actor.unwrap(),
            intended_receiver: LivingEntityIdV2([6; 16]),
            recipient: LivingEntityIdV2([6; 16]),
            resource: LivingResourceV2::Alloy,
            units: 1,
            dispatch_kind: None,
            source_system: Some(production.system),
            destination_system: Some(LivingEntityIdV2([5; 16])),
        };
        let delivered = SupplyDelivery {
            reference: reference(2, 0),
            delivery: delivery.clone(),
        };
        assert_eq!(
            supply_link(&production, &delivered),
            Some(LivingNotebookSupplyTopologyV2::AlloyDelivered)
        );
        for defect in 0..6 {
            let mut changed = delivery.clone();
            match defect {
                0 => changed.source_system = None,
                1 => changed.destination_system = None,
                2 => changed.destination_system = changed.source_system,
                3 => changed.dispatch_owner = changed.recipient,
                4 => changed.source_world = LivingEntityIdV2([10; 16]),
                _ => changed.resource = LivingResourceV2::Energy,
            }
            assert!(
                supply_link(
                    &production,
                    &SupplyDelivery {
                        reference: reference(2, 0),
                        delivery: changed
                    }
                )
                .is_none()
            );
        }
        let mut unknown_actor = production;
        unknown_actor.actor = None;
        assert!(supply_link(&unknown_actor, &delivered).is_none());
    }
}

#[cfg(test)]
mod corridor_observation_tests {
    use super::super::{
        catalog::LivingSlugV2,
        event_facts::LivingCorridorRestrictionV2,
        ids::LivingBranchIdV2,
        model::{LivingHazardV2, LivingResourceV2},
    };
    use super::*;
    fn id(n: u8) -> LivingEntityIdV2 {
        LivingEntityIdV2([n; 16])
    }
    fn reference(tick: u64) -> EvidenceReference {
        EvidenceReference {
            record: 1,
            bookmark: 1,
            view: LivingHistoryViewV2 {
                branch: LivingBranchIdV2([1; 16]),
                revision: None,
                tick: LivingTickV2(tick),
            },
            event: LivingNotebookEventReferenceV2 {
                ordinal: 0,
                id: LivingEventIdV2([1; 16]),
                kind: LivingEventKindV2::FacilityBlocked,
            },
            revision_depth: 0,
        }
    }
    fn restriction() -> LivingNotebookCorridorRestrictionObservationV2 {
        LivingNotebookCorridorRestrictionObservationV2 {
            reference: CorridorAttemptReference {
                record: 1,
                bookmark: 1,
                view: reference(10).view,
                revision_depth: 0,
                locator: LivingNotebookRouteAttemptReferenceV2 {
                    route: id(3),
                    hazard: id(10),
                },
            },
            receipt_digest: LivingReceiptDigestV2([2; 32]),
            restriction: LivingCorridorRestrictionV2 {
                tick: LivingTickV2(10),
                receipt_digest: LivingReceiptDigestV2([2; 32]),
                route: id(3),
                source_world: id(4),
                destination_world: id(5),
                source_system: id(6),
                destination_system: id(7),
                sender: id(8),
                receiver: id(9),
                resource: LivingResourceV2::Ore,
                requested_batch: 2,
                eligible_batch: 2,
                restricted_batch: 1,
                actual_dispatched_units: 1,
                hazard: LivingHazardV2 {
                    id: id(10),
                    definition: LivingSlugV2::new("ion_storm").unwrap(),
                    lane: id(11),
                    start_tick: LivingTickV2(10),
                    end_tick: LivingTickV2(150),
                },
            },
        }
    }
    fn wait() -> LivingNotebookCorridorWaitObservationV2 {
        LivingNotebookCorridorWaitObservationV2 {
            reference: reference(20),
            facility: id(12),
            actor: id(9),
            world: id(5),
            system: id(7),
            resource: LivingResourceV2::Ore,
            required: 2,
            available: 0,
        }
    }
    #[test]
    fn later_matching_wait_and_zero_restriction_are_observations_only() {
        assert!(wait().matches_restriction(&restriction()));
        let mut zero = restriction();
        zero.restriction.restricted_batch = 0;
        zero.restriction.actual_dispatched_units = 0;
        assert!(wait().matches_restriction(&zero));
        let mut job = LivingNotebookCorridorObservationJoinV2::new(
            vec![restriction(), zero],
            vec![wait(), wait()],
        )
        .unwrap();
        assert!(!job.advance());
        assert_eq!(job.pairs(), &[(0, 0)]);
        assert!(job.advance());
        assert_eq!(job.pairs(), &[(0, 0), (0, 1)]);
    }
    #[test]
    fn rejects_wrong_boundary_prefix_endpoint_resource_and_unaccepted_dispatch() {
        let mut same_tick = wait();
        same_tick.reference.view.tick = LivingTickV2(10);
        assert!(!same_tick.matches_restriction(&restriction()));
        let mut sibling = wait();
        sibling.reference.view.branch = LivingBranchIdV2([3; 16]);
        assert!(!sibling.matches_restriction(&restriction()));
        let mut reversed = restriction();
        reversed.reference.revision_depth = 1;
        assert!(!wait().matches_restriction(&reversed));
        for axis in 0..10 {
            let mut r = restriction();
            match axis {
                0 => r.receipt_digest = LivingReceiptDigestV2([3; 32]),
                1 => r.restriction.eligible_batch = 0,
                2 => r.restriction.restricted_batch = 2,
                3 => r.restriction.actual_dispatched_units = 0,
                4 => r.restriction.receiver = id(99),
                5 => r.restriction.destination_world = id(99),
                6 => r.restriction.destination_system = id(99),
                7 => r.restriction.resource = LivingResourceV2::Energy,
                8 => r.restriction.hazard.end_tick = LivingTickV2(10),
                _ => r.restriction.source_system = r.restriction.destination_system,
            }
            assert!(!wait().matches_restriction(&r), "axis {axis}");
        }
    }
    #[test]
    fn bounded_join_handles_empty_inputs_and_rejects_over_capacity() {
        let mut empty = LivingNotebookCorridorObservationJoinV2::new(vec![], vec![wait()]).unwrap();
        assert!(empty.advance());
        assert!(empty.pairs().is_empty());
        assert!(matches!(
            LivingNotebookCorridorObservationJoinV2::new(
                vec![],
                std::iter::repeat_with(wait).take(12289).collect()
            ),
            Err(LivingNotebookErrorV2::Shape)
        ));
    }
}
