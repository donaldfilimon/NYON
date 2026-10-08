//! Living Galaxy V2 tick receipts, their ordered events, and the distinct
//! record `receipt_digest` is actually computed over.
//!
//! Section 10 of
//! `docs/superpowers/specs/2026-09-04-nyon-living-galaxy-rules.md` is binding
//! here, and it resolves a circularity that would otherwise be unimplementable:
//! an event's identity comes from the receipt digest, so the digest cannot be
//! taken over a document that already contains those identities. The spec's
//! answer is a second record. `canonical_receipt_bytes` encodes
//! [`LivingReceiptPayloadV2`], whose event entries carry `ordinal`,
//! `provenance` and `kind` and deliberately have no identity field at all.
//!
//! The derivation order is normative:
//!
//! 1. compute `receipt_digest` over the payload,
//! 2. derive each event identity from that digest and the event's ordinal,
//! 3. assemble the public [`LivingTickReceiptV2`].
//!
//! An implementation that hashes a receipt containing its own digest, or that
//! substitutes zeroed identities to break the cycle, is not conformant.
//! [`LivingPendingEventV2`] exists so that step is not merely a convention: a
//! pending event has no identity field to fill in, correctly or otherwise.
//!
//! # What this module decides rather than transcribes
//!
//! The spec names `provenance` and `kind` without fixing their shape, so
//! [`LivingEventProvenanceV2`] and [`LivingEventKindV2`] are decisions of this
//! task, recorded here rather than left implicit. Both follow the canonical
//! wire contract for tagged enums, and both are pinned by
//! `tests/living_receipt.rs`. No hash depends on the kind list's order: the
//! spec deliberately leaves that enum's discriminant ordinal unassigned,
//! because a tagged enum travels the wire as a lowercase-snake-case string and
//! `event_digest` consumes an emission ordinal rather than a kind. Adding a
//! kind is therefore not a format break; changing the *field order* of either
//! record is, which is why that order is pinned too.

use serde::{Deserialize, Serialize};

use super::ids::{
    LivingEventIdV2, LivingPhaseV2, LivingReceiptDigestV2, LivingRevisionIdV2, LivingStateDigestV2,
    LivingTickV2, event_id_v2, receipt_digest_v2,
};
use super::wire::{LIVING_MAX_ARCHIVE_BYTES_V2, LivingWireErrorV2, encode_canonical_v2};

/// The largest number of events one tick boundary may emit.
///
/// The event ordinal is a `u16` framed into `event_digest`, so the identity
/// space of a single boundary is exactly this many events. Reaching it is a
/// typed refusal rather than a wrapped ordinal, because a wrapped ordinal would
/// silently issue one identity to two events.
pub const LIVING_MAX_BOUNDARY_EVENTS_V2: usize = u16::MAX as usize + 1;

/// Why an event exists.
///
/// Section 8 requires creator overrides to "produce Creator intervention events
/// rather than pretending the civilizations negotiated", so provenance is the
/// field that keeps an authored change distinguishable from an emergent one
/// forever, including after replay. A creator event names the revision that
/// caused it; an autonomous event names the published phase ordinal of the step
/// that produced it. Creator interventions are applied in phase 1 by
/// definition, so that variant carries no phase of its own.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(tag = "type", rename_all = "snake_case", deny_unknown_fields)]
pub enum LivingEventProvenanceV2 {
    /// Produced by a simulation phase.
    Autonomous {
        /// The published phase ordinal of the producing step.
        phase: LivingPhaseV2,
    },
    /// Produced by applying one validated creator revision.
    Creator {
        /// The revision whose application produced the event.
        revision: LivingRevisionIdV2,
    },
}

/// What an event reports.
///
/// Every kind below is something the rules spec says a boundary must be able to
/// report; the list is alphabetical, like the crate's other wire enums, and its
/// order carries no meaning.
#[derive(Clone, Copy, Debug, Deserialize, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum LivingEventKindV2 {
    /// A bilateral agreement reached its end-exclusive boundary.
    AgreementExpired,
    /// A mutually eligible agreement was formed at a diplomacy boundary.
    AgreementFormed,
    /// A civilization lost its last colony and became dormant.
    CivilizationDormant,
    /// A dormant civilization settled again and became active.
    CivilizationReactivated,
    /// Occupation completed and a colony changed owner.
    ColonyCaptured,
    /// One due combat round resolved.
    CombatRound,
    /// A construction job was cancelled, with its half-alloy refund.
    ConstructionCancelled,
    /// A construction job completed into a facility.
    ConstructionCompleted,
    /// A creator intervention was applied.
    CreatorIntervention,
    /// A due facility attempt produced nothing and recorded its reasons.
    FacilityBlocked,
    /// A successful enabled facility recipe consumed and produced its actual batch.
    FacilityProduced,
    /// A fleet arrived at its destination world.
    FleetArrived,
    /// A fleet departed on a committed leg.
    FleetDeparted,
    /// A hazard reached its end-exclusive boundary.
    HazardEnded,
    /// A hazard reached its start boundary.
    HazardStarted,
    /// A hull job completed into a hull.
    HullCompleted,
    /// A hull was destroyed in combat.
    HullDestroyed,
    /// Refunded alloy above the storage cap was discarded, which section 3
    /// requires to be receipted rather than silently dropped.
    RefundDiscarded,
    /// A first eligible arrival opened a settlement claim window.
    SettlementClaimOpened,
    /// A settlement claim window closed and arbitration chose a claimant.
    SettlementClaimResolved,
    /// Cargo was taken by the destination's new owner.
    ShipmentCaptured,
    /// Cargo was delivered into its destination world.
    ShipmentDelivered,
    /// A due freight route dispatched cargo.
    ShipmentDispatched,
    /// Cargo returned to its source after a war began mid-flight.
    ShipmentReturned,
    /// Arrived cargo waited because the recipient's storage was full.
    ShipmentWaiting,
    /// A war was declared at a diplomacy boundary.
    WarDeclared,
    /// A war reached its end-exclusive boundary and became a truce.
    WarEnded,
}

impl LivingEventKindV2 {
    /// The stable wire discriminant.
    pub const fn as_wire_str(self) -> &'static str {
        match self {
            Self::AgreementExpired => "agreement_expired",
            Self::AgreementFormed => "agreement_formed",
            Self::CivilizationDormant => "civilization_dormant",
            Self::CivilizationReactivated => "civilization_reactivated",
            Self::ColonyCaptured => "colony_captured",
            Self::CombatRound => "combat_round",
            Self::ConstructionCancelled => "construction_cancelled",
            Self::ConstructionCompleted => "construction_completed",
            Self::CreatorIntervention => "creator_intervention",
            Self::FacilityBlocked => "facility_blocked",
            Self::FacilityProduced => "facility_produced",
            Self::FleetArrived => "fleet_arrived",
            Self::FleetDeparted => "fleet_departed",
            Self::HazardEnded => "hazard_ended",
            Self::HazardStarted => "hazard_started",
            Self::HullCompleted => "hull_completed",
            Self::HullDestroyed => "hull_destroyed",
            Self::RefundDiscarded => "refund_discarded",
            Self::SettlementClaimOpened => "settlement_claim_opened",
            Self::SettlementClaimResolved => "settlement_claim_resolved",
            Self::ShipmentCaptured => "shipment_captured",
            Self::ShipmentDelivered => "shipment_delivered",
            Self::ShipmentDispatched => "shipment_dispatched",
            Self::ShipmentReturned => "shipment_returned",
            Self::ShipmentWaiting => "shipment_waiting",
            Self::WarDeclared => "war_declared",
            Self::WarEnded => "war_ended",
        }
    }
}

/// One entry of `event_payloads`, which is what the receipt digest is taken
/// over.
///
/// Its three fields are the spec's, in the spec's order:
/// `{ordinal_u16, provenance, kind}`. There is no identity field, by design.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct LivingEventPayloadV2 {
    /// Position in emission order, starting at zero.
    pub ordinal: u16,
    /// Why the event exists.
    pub provenance: LivingEventProvenanceV2,
    /// What the event reports.
    pub kind: LivingEventKindV2,
}

/// An event held inside a step before the boundary commits.
///
/// It carries no identity and no placeholder for one, because its identity does
/// not exist yet: it is derived from a digest taken over the payload this
/// contributes to. Only [`seal_living_tick_receipt_v2`] can turn one of these
/// into a [`LivingEventV2`].
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct LivingPendingEventV2 {
    /// Why the event exists.
    pub provenance: LivingEventProvenanceV2,
    /// What the event reports.
    pub kind: LivingEventKindV2,
}

/// One committed, identified event of a tick receipt.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct LivingEventV2 {
    /// Stable identity, the first 16 bytes of
    /// `SHA256(domain || receipt_digest_32 || event_ordinal_u16)`.
    pub id: LivingEventIdV2,
    /// Position in emission order, starting at zero.
    pub ordinal: u16,
    /// Why the event exists.
    pub provenance: LivingEventProvenanceV2,
    /// What the event reports.
    pub kind: LivingEventKindV2,
}

/// The record `canonical_receipt_bytes` encodes.
///
/// This is distinct from the public [`LivingTickReceiptV2`] and is never
/// published on its own; it exists so the digest has something to be taken
/// over that does not contain the identities the digest produces. Its four
/// fields are the spec's, in the spec's order.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct LivingReceiptPayloadV2 {
    /// The committed boundary.
    pub tick: LivingTickV2,
    /// The revisions applied at this boundary, sorted strictly ascending.
    pub applied_revisions: Vec<LivingRevisionIdV2>,
    /// The boundary's events in emission order, without identities.
    pub event_payloads: Vec<LivingEventPayloadV2>,
    /// The digest of the state this boundary committed.
    pub state_digest: LivingStateDigestV2,
}

impl LivingReceiptPayloadV2 {
    /// The canonical bytes this payload hashes as.
    pub fn canonical_bytes(&self) -> Result<Vec<u8>, LivingWireErrorV2> {
        encode_canonical_v2(self, LIVING_MAX_ARCHIVE_BYTES_V2)
    }

    /// `receipt_digest = SHA256("NYON-LIVING-RECEIPT-V2\0" || canonical_bytes)`.
    pub fn digest(&self) -> Result<LivingReceiptDigestV2, LivingWireErrorV2> {
        Ok(receipt_digest_v2(&self.canonical_bytes()?))
    }
}

/// One committed tick receipt.
///
/// `digest` is last because it seals the four fields above it: a verifier
/// rebuilds the payload from them and recomputes it. `events` repeats the
/// payload's ordinals, provenances and kinds and adds the identities derived
/// from `digest`.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct LivingTickReceiptV2 {
    /// The committed boundary.
    pub tick: LivingTickV2,
    /// The revisions applied at this boundary, sorted strictly ascending.
    pub applied_revisions: Vec<LivingRevisionIdV2>,
    /// The boundary's identified events, in emission order.
    pub events: Vec<LivingEventV2>,
    /// The digest of the state this boundary committed.
    pub state_digest: LivingStateDigestV2,
    /// The digest of this receipt's payload.
    pub digest: LivingReceiptDigestV2,
}

/// Every way a receipt can fail to be sealed or verified.
#[derive(Clone, Copy, Debug, Eq, PartialEq, thiserror::Error)]
pub enum LivingReceiptErrorV2 {
    /// `applied_revisions` is the sorted array of revision identities, so a
    /// repeated or out-of-order entry is refused rather than sorted silently:
    /// sorting it here would change the bytes a caller believes it hashed.
    #[error("Living V2 applied revisions are not sorted strictly ascending")]
    UnsortedRevisions,
    /// One boundary cannot emit more events than the `u16` ordinal can name.
    #[error("a Living V2 boundary emits at most {limit} events")]
    TooManyEvents {
        /// The declared bound.
        limit: usize,
    },
    /// The payload could not be encoded as canonical bytes.
    #[error("the Living V2 receipt payload is not canonical")]
    Wire(#[from] LivingWireErrorV2),
    /// A receipt's stored digest is not the digest of its own payload.
    #[error("the Living V2 receipt digest does not seal its own payload")]
    DigestMismatch,
    /// An event's identity is not the one its ordinal and receipt digest
    /// derive, or its ordinals are not emission order from zero.
    #[error("a Living V2 receipt event carries an identity its receipt does not derive")]
    EventIdentityMismatch,
}

/// The events one boundary has emitted so far, in emission order.
///
/// Ordinals are assigned here and nowhere else, so a subsystem cannot choose
/// its own, skip one, or reuse one. Combined with [`LivingPendingEventV2`]
/// having no identity field, this is what makes the normative derivation order
/// the only reachable one.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LivingPendingEventsV2 {
    entries: Vec<LivingEventPayloadV2>,
    pub(crate) decisions: Vec<super::decision::LivingIntentDecisionV2>,
    pub(crate) creator_recoveries: Vec<super::simulation::LivingCreatorRecoveryCandidateV2>,
    pub(crate) corridor_restrictions: Vec<super::event_facts::LivingCorridorRestrictionV2>,
    pub(crate) explained: Vec<(u16, super::event_facts::LivingEventFactsV2)>,
    explained_bytes: usize,
    explanations_enabled: bool,
}

impl Default for LivingPendingEventsV2 {
    fn default() -> Self {
        Self::new()
    }
}

impl LivingPendingEventsV2 {
    /// An empty boundary.
    pub const fn new() -> Self {
        Self {
            entries: Vec::new(),
            decisions: Vec::new(),
            creator_recoveries: Vec::new(),
            corridor_restrictions: Vec::new(),
            explained: Vec::new(),
            explained_bytes: 0,
            explanations_enabled: true,
        }
    }

    /// Private discard-only forecast collector. Ordinary event ordering and
    /// capacity remain exact; no authenticated explanation may be sealed from it.
    pub(crate) fn for_forecast() -> Self {
        Self {
            explanations_enabled: false,
            ..Self::new()
        }
    }
    pub(crate) const fn explanations_enabled(&self) -> bool {
        self.explanations_enabled
    }

    /// Append one event and return the ordinal it was given.
    pub fn record(
        &mut self,
        provenance: LivingEventProvenanceV2,
        kind: LivingEventKindV2,
    ) -> Result<u16, LivingReceiptErrorV2> {
        let ordinal =
            u16::try_from(self.entries.len()).map_err(|_| LivingReceiptErrorV2::TooManyEvents {
                limit: LIVING_MAX_BOUNDARY_EVENTS_V2,
            })?;
        self.entries.push(LivingEventPayloadV2 {
            ordinal,
            provenance,
            kind,
        });
        Ok(ordinal)
    }

    /// Append producer facts atomically with their ordinary event. Existing event
    /// payloads and emission ordinals remain unchanged.
    pub fn record_explained(
        &mut self,
        provenance: LivingEventProvenanceV2,
        kind: LivingEventKindV2,
        facts: super::event_facts::LivingEventFactsV2,
    ) -> Result<u16, super::event_facts::LivingEventFactsErrorV2> {
        if !self.explanations_enabled {
            return self.record(provenance, kind).map_err(Into::into);
        }
        facts.validate(kind)?;
        let expected_ordinal =
            u16::try_from(self.entries.len()).map_err(|_| LivingReceiptErrorV2::TooManyEvents {
                limit: LIVING_MAX_BOUNDARY_EVENTS_V2,
            })?;
        let entry = super::event_facts::LivingExplainedEventV2 {
            ordinal: expected_ordinal,
            event_id: LivingEventIdV2([0; 16]),
            facts: facts.clone(),
        };
        let bytes = serde_json::to_vec(&entry)
            .map_err(|_| LivingWireErrorV2::Json)?
            .len();
        let next_bytes = self
            .explained_bytes
            .checked_add(bytes + usize::from(!self.explained.is_empty()))
            .ok_or(super::event_facts::LivingEventFactsErrorV2::Capacity)?;
        // Canonical sidecar admission happens at sealing, where complete
        // framing and coalesced omissions can be measured exactly. A valid
        // explanation must not fault an otherwise valid ordinary boundary.
        let ordinal = self.record(provenance, kind)?;
        self.explained_bytes = next_bytes;
        self.explained.push((ordinal, facts));
        Ok(ordinal)
    }

    /// The payload entries in emission order.
    pub fn payloads(&self) -> &[LivingEventPayloadV2] {
        &self.entries
    }

    /// How many events the boundary has emitted.
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Whether the boundary has emitted nothing.
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
}

fn strictly_ascending(revisions: &[LivingRevisionIdV2]) -> bool {
    revisions.windows(2).all(|pair| pair[0] < pair[1])
}

use super::event_facts::{
    LivingEventFactsV2, LivingEventFactsValidationCursorV2, LivingExplainedEntryEncodingCursorV2,
};
use super::step_job::{
    LIVING_MAX_PHASE_POLL_VISITS_V2, LivingLifecycleProgressV2, LivingPhaseRetirementJobV2,
    LivingStorageJobErrorV2, LivingVecStorageJobV2, PhaseRetirementLane,
};

/// Closed retained faults, not a general gameplay/error envelope. Storage is
/// private setup failure and is never translated into a canonical domain fault.
#[allow(dead_code)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum LivingPendingEventAppendErrorV2 {
    Budget,
    Cancelled,
    NotReady,
    Facts,
    TooManyEvents,
    Json,
    Capacity,
    Storage(LivingStorageJobErrorV2),
}
#[allow(dead_code)]
impl LivingPendingEventAppendErrorV2 {
    pub(crate) fn facts_error(self) -> Option<super::event_facts::LivingEventFactsErrorV2> {
        use super::event_facts::LivingEventFactsErrorV2 as E;
        match self {
            Self::Facts => Some(E::Facts),
            Self::TooManyEvents => Some(E::Receipt(LivingReceiptErrorV2::TooManyEvents {
                limit: LIVING_MAX_BOUNDARY_EVENTS_V2,
            })),
            Self::Json => Some(E::Wire(LivingWireErrorV2::Json)),
            Self::Capacity => Some(E::Capacity),
            Self::Budget | Self::Cancelled | Self::NotReady | Self::Storage(_) => None,
        }
    }
    pub(crate) fn receipt_error(self) -> Option<LivingReceiptErrorV2> {
        match self {
            Self::TooManyEvents => Some(LivingReceiptErrorV2::TooManyEvents {
                limit: LIVING_MAX_BOUNDARY_EVENTS_V2,
            }),
            _ => None,
        }
    }
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum LivingPendingEventAppendStatusV2 {
    Pending,
    Ready,
    Faulted,
    Cancelled,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum AppendMode {
    Ordinary,
    Live,
    Forecast,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum AppendStage {
    Mode,
    Validation,
    Ordinal,
    Encoding,
    Accounting,
    PrepareEntries,
    PrepareExplained,
    CommitEntry,
    CommitFacts,
    Publication,
    IgnoredSetup,
    IgnoredRetire,
    Ready,
}

/// An unpublished owning operation. Constructors move original roots only;
/// cancellation/fault never destroys them. Borrowed extraction publishes once,
/// after both logical commits and counted original-storage/ignored-facts cleanup.
#[allow(dead_code)]
pub(crate) struct LivingPendingEventAppendJobV2 {
    events: Option<LivingPendingEventsV2>,
    facts: Option<LivingEventFactsV2>,
    payload: Option<LivingEventPayloadV2>,
    explained_pair: Option<(u16, LivingEventFactsV2)>,
    entries_storage: Option<LivingVecStorageJobV2<'static, LivingEventPayloadV2>>,
    explained_storage: Option<LivingVecStorageJobV2<'static, (u16, LivingEventFactsV2)>>,
    cleanup: PhaseRetirementLane<'static>,
    validation: LivingEventFactsValidationCursorV2,
    encoding: Option<LivingExplainedEntryEncodingCursorV2>,
    provenance: LivingEventProvenanceV2,
    kind: LivingEventKindV2,
    mode: AppendMode,
    stage: AppendStage,
    ordinal: u16,
    entry_len: Option<usize>,
    next_bytes: usize,
    fault: Option<LivingPendingEventAppendErrorV2>,
    cancelled: bool,
    delivered: bool,
    last_poll_visits: usize,
    total_visits: u64,
}
#[allow(dead_code)]
impl LivingPendingEventAppendJobV2 {
    pub(crate) fn begin_ordinary(
        events: LivingPendingEventsV2,
        provenance: LivingEventProvenanceV2,
        kind: LivingEventKindV2,
    ) -> Self {
        Self::begin(events, provenance, kind, None)
    }
    pub(crate) fn begin_explained(
        events: LivingPendingEventsV2,
        provenance: LivingEventProvenanceV2,
        kind: LivingEventKindV2,
        facts: LivingEventFactsV2,
    ) -> Self {
        Self::begin(events, provenance, kind, Some(facts))
    }
    fn begin(
        events: LivingPendingEventsV2,
        provenance: LivingEventProvenanceV2,
        kind: LivingEventKindV2,
        facts: Option<LivingEventFactsV2>,
    ) -> Self {
        Self {
            events: Some(events),
            facts,
            payload: None,
            explained_pair: None,
            entries_storage: None,
            explained_storage: None,
            cleanup: PhaseRetirementLane::default(),
            validation: LivingEventFactsValidationCursorV2::new(kind),
            encoding: None,
            provenance,
            kind,
            mode: AppendMode::Ordinary,
            stage: AppendStage::Mode,
            ordinal: 0,
            entry_len: Some(0),
            next_bytes: 0,
            fault: None,
            cancelled: false,
            delivered: false,
            last_poll_visits: 0,
            total_visits: 0,
        }
    }
    pub(crate) fn poll(
        &mut self,
        budget: usize,
    ) -> Result<LivingLifecycleProgressV2, LivingPendingEventAppendErrorV2> {
        self.last_poll_visits = 0;
        if budget == 0 || budget > LIVING_MAX_PHASE_POLL_VISITS_V2 {
            return Err(LivingPendingEventAppendErrorV2::Budget);
        }
        if let Some(error) = self.fault {
            return Err(error);
        }
        if self.cancelled {
            return Err(LivingPendingEventAppendErrorV2::Cancelled);
        }
        while self.stage != AppendStage::Ready && self.last_poll_visits < budget {
            if let Err(error) = self.advance(budget - self.last_poll_visits) {
                self.fault = Some(error);
                return Err(error);
            }
        }
        Ok(if self.stage == AppendStage::Ready {
            LivingLifecycleProgressV2::Ready
        } else {
            LivingLifecycleProgressV2::Pending
        })
    }
    fn charge(&mut self, visits: usize) {
        self.last_poll_visits += visits;
        self.total_visits += visits as u64;
    }
    fn advance(&mut self, remaining: usize) -> Result<(), LivingPendingEventAppendErrorV2> {
        use AppendStage as S;
        use LivingPendingEventAppendErrorV2 as E;
        debug_assert!(remaining > 0);
        match self.stage {
            S::Mode => {
                self.charge(1);
                self.mode = if self.facts.is_none() {
                    AppendMode::Ordinary
                } else if self.events.as_ref().unwrap().explanations_enabled {
                    AppendMode::Live
                } else {
                    AppendMode::Forecast
                };
                self.stage = if self.mode == AppendMode::Live {
                    S::Validation
                } else {
                    S::Ordinal
                };
            }
            S::Validation => {
                let result = self.validation.visit(self.facts.as_ref().unwrap());
                let worked = self.validation.last_visit_visits();
                self.charge(worked);
                result.map_err(|_| E::Facts)?;
                if self.validation.ready() {
                    self.stage = S::Ordinal;
                }
            }
            S::Ordinal => {
                self.charge(1);
                self.ordinal = u16::try_from(self.events.as_ref().unwrap().entries.len())
                    .map_err(|_| E::TooManyEvents)?;
                self.payload = Some(LivingEventPayloadV2 {
                    ordinal: self.ordinal,
                    provenance: self.provenance,
                    kind: self.kind,
                });
                if self.mode == AppendMode::Live {
                    self.encoding = Some(LivingExplainedEntryEncodingCursorV2::new(
                        self.ordinal,
                        LivingEventIdV2([0; 16]),
                    ));
                    self.stage = S::Encoding;
                } else {
                    self.stage = S::PrepareEntries;
                }
            }
            S::Encoding => {
                let cursor = self.encoding.as_mut().unwrap();
                let result = cursor.next_fragment(self.facts.as_ref().unwrap());
                let worked = cursor.last_visit_visits();
                self.charge(worked);
                match result.map_err(|_| E::Json)? {
                    Some(fragment) => {
                        self.entry_len = self
                            .entry_len
                            .and_then(|len| len.checked_add(fragment.as_bytes().len()))
                    }
                    None => self.stage = S::Accounting,
                }
            }
            S::Accounting => {
                self.charge(1);
                let events = self.events.as_ref().unwrap();
                // Preserve encoding-before-capacity precedence even for the
                // unreachable supported-target entry-length overflow case.
                self.next_bytes = self
                    .entry_len
                    .and_then(|len| len.checked_add(usize::from(!events.explained.is_empty())))
                    .and_then(|len| events.explained_bytes.checked_add(len))
                    .ok_or(E::Capacity)?;
                self.stage = S::PrepareEntries;
            }
            S::PrepareEntries => {
                if self.entries_storage.is_none() {
                    self.entries_storage = Some(LivingVecStorageJobV2::begin(
                        std::mem::take(&mut self.events.as_mut().unwrap().entries),
                        1,
                    ));
                }
                let storage = self.entries_storage.as_mut().unwrap();
                let result = storage.poll(remaining);
                let worked = storage.last_poll_visits();
                self.charge(worked);
                if result.map_err(E::Storage)? == LivingLifecycleProgressV2::Ready {
                    self.events.as_mut().unwrap().entries = self
                        .entries_storage
                        .as_mut()
                        .unwrap()
                        .take_ready_result()
                        .map_err(E::Storage)?;
                    self.stage = if self.mode == AppendMode::Live {
                        S::PrepareExplained
                    } else {
                        S::CommitEntry
                    };
                }
            }
            S::PrepareExplained => {
                if self.explained_storage.is_none() {
                    self.explained_storage = Some(LivingVecStorageJobV2::begin(
                        std::mem::take(&mut self.events.as_mut().unwrap().explained),
                        1,
                    ));
                }
                let storage = self.explained_storage.as_mut().unwrap();
                let result = storage.poll(remaining);
                let worked = storage.last_poll_visits();
                self.charge(worked);
                if result.map_err(E::Storage)? == LivingLifecycleProgressV2::Ready {
                    self.events.as_mut().unwrap().explained = self
                        .explained_storage
                        .as_mut()
                        .unwrap()
                        .take_ready_result()
                        .map_err(E::Storage)?;
                    self.stage = S::CommitEntry;
                }
            }
            S::CommitEntry => {
                self.charge(1);
                LivingVecStorageJobV2::commit_one(
                    &mut self.events.as_mut().unwrap().entries,
                    &mut self.payload,
                )
                .expect("prepared payload Vec has no semantic failure");
                self.stage = if self.mode == AppendMode::Live {
                    S::CommitFacts
                } else {
                    S::Publication
                };
            }
            S::CommitFacts => {
                self.charge(1);
                self.explained_pair = Some((self.ordinal, self.facts.take().unwrap()));
                LivingVecStorageJobV2::commit_one(
                    &mut self.events.as_mut().unwrap().explained,
                    &mut self.explained_pair,
                )
                .expect("prepared facts Vec has no semantic failure");
                self.stage = S::Publication;
            }
            S::Publication => {
                self.charge(1);
                if self.mode == AppendMode::Live {
                    self.events.as_mut().unwrap().explained_bytes = self.next_bytes;
                }
                self.stage = if self.mode == AppendMode::Forecast {
                    S::IgnoredSetup
                } else {
                    S::Ready
                };
            }
            S::IgnoredSetup => {
                self.charge(1);
                // Raw facts, not Option<Facts>: no extra wrapper in D_ignored.
                self.cleanup.start(self.facts.take().unwrap());
                self.stage = S::IgnoredRetire;
            }
            S::IgnoredRetire => {
                let worked = self.cleanup.visit();
                self.charge(usize::from(worked));
                if self.cleanup.is_empty() {
                    self.stage = S::Ready;
                }
            }
            S::Ready => {}
        }
        Ok(())
    }
    /// Read-only inspection of the original collector and its retained storage.
    #[cfg(test)]
    pub(crate) fn retained_for_test(
        &self,
    ) -> (
        Option<&LivingPendingEventsV2>,
        Option<&LivingVecStorageJobV2<'static, LivingEventPayloadV2>>,
    ) {
        (self.events.as_ref(), self.entries_storage.as_ref())
    }
    /// Immutable original facts/storage partitions for production owner checks.
    #[cfg(test)]
    pub(crate) fn retained_explained_for_test(&self) -> (Option<&LivingEventFactsV2>, bool) {
        (self.facts.as_ref(), self.explained_storage.is_some())
    }
    /// Authentic test-only failure at the owned entry-storage preparation stage.
    #[cfg(test)]
    pub(crate) fn inject_entry_capacity_failure(&mut self) -> bool {
        if self.stage != AppendStage::PrepareEntries || self.entries_storage.is_some() {
            return false;
        }
        self.entries_storage = Some(LivingVecStorageJobV2::begin(
            std::mem::take(&mut self.events.as_mut().unwrap().entries),
            usize::MAX,
        ));
        true
    }
    pub(crate) fn status(&self) -> LivingPendingEventAppendStatusV2 {
        use LivingPendingEventAppendStatusV2 as S;
        if self.fault.is_some() {
            S::Faulted
        } else if self.cancelled {
            S::Cancelled
        } else if self.stage == AppendStage::Ready {
            S::Ready
        } else {
            S::Pending
        }
    }
    pub(crate) fn error(&self) -> Option<LivingPendingEventAppendErrorV2> {
        self.fault
    }
    pub(crate) fn cancel(&mut self) {
        self.cancelled = true;
    }
    pub(crate) fn last_poll_visits(&self) -> usize {
        self.last_poll_visits
    }
    pub(crate) fn total_visits(&self) -> u64 {
        self.total_visits
    }
    pub(crate) fn take_ready_result(
        &mut self,
    ) -> Result<(LivingPendingEventsV2, u16), LivingPendingEventAppendErrorV2> {
        if let Some(error) = self.fault {
            return Err(error);
        }
        if self.cancelled {
            return Err(LivingPendingEventAppendErrorV2::Cancelled);
        }
        if self.stage != AppendStage::Ready || self.delivered {
            return Err(LivingPendingEventAppendErrorV2::NotReady);
        }
        self.delivered = true;
        Ok((
            self.events.take().expect("complete unpublished collector"),
            self.ordinal,
        ))
    }
    pub(crate) fn into_retirement(
        self,
        policy: LivingStateRetirementPolicyV2,
    ) -> LivingPhaseRetirementJobV2<'static, Self> {
        LivingPhaseRetirementJobV2::begin(self, policy)
    }
}

impl<'a> RetirementFields<'a> for LivingPendingEventAppendJobV2 {
    #[cfg(test)]
    fn creator_allocations(&self, out: &mut Vec<(usize, usize, Option<usize>, usize)>) {
        self.events.creator_allocations(out);
        self.facts.creator_allocations(out);
        self.payload.creator_allocations(out);
        self.explained_pair.creator_allocations(out);
        self.entries_storage.creator_allocations(out);
        self.explained_storage.creator_allocations(out);
        self.cleanup.creator_allocations(out);
    }
    fn retirement_child(
        &mut self,
        index: usize,
        policy: LivingStateRetirementPolicyV2,
    ) -> Option<Box<dyn RetirementPoll + 'a>> {
        // Closed non-owning cursors have only fixed enums/indices/IDs/arrays;
        // they are not given a blanket leaf adapter. Inventory every field.
        let Self {
            events,
            facts,
            payload,
            explained_pair,
            entries_storage,
            explained_storage,
            cleanup,
            validation,
            encoding,
            provenance: _,
            kind: _,
            mode: _,
            stage: _,
            ordinal: _,
            entry_len: _,
            next_bytes: _,
            fault: _,
            cancelled: _,
            delivered: _,
            last_poll_visits: _,
            total_visits: _,
        } = self;
        validation.assert_non_owning();
        if let Some(cursor) = encoding {
            cursor.assert_non_owning();
        }
        match index {
            0 => Some(retirement_node(events.take(), policy)),
            1 => Some(retirement_node(facts.take(), policy)),
            2 => Some(retirement_node(payload.take(), policy)),
            3 => Some(retirement_node(explained_pair.take(), policy)),
            4 => Some(retirement_node(entries_storage.take(), policy)),
            5 => Some(retirement_node(explained_storage.take(), policy)),
            6 => Some(retirement_node(std::mem::take(cleanup), policy)),
            _ => None,
        }
    }
}

/// Seal a boundary: hash the payload, derive every event identity from that
/// digest, then assemble the public receipt, in that order.
pub fn seal_living_tick_receipt_v2(
    tick: LivingTickV2,
    applied_revisions: Vec<LivingRevisionIdV2>,
    pending: &LivingPendingEventsV2,
    state_digest: LivingStateDigestV2,
) -> Result<LivingTickReceiptV2, LivingReceiptErrorV2> {
    if !strictly_ascending(&applied_revisions) {
        return Err(LivingReceiptErrorV2::UnsortedRevisions);
    }

    // 1. The digest comes first, over a payload that holds no identity.
    let payload = LivingReceiptPayloadV2 {
        tick,
        applied_revisions,
        event_payloads: pending.payloads().to_vec(),
        state_digest,
    };
    let digest = payload.digest()?;

    // 2. Every identity is derived from that digest and an emission ordinal.
    let events = payload
        .event_payloads
        .iter()
        .map(|entry| LivingEventV2 {
            id: event_id_v2(digest, entry.ordinal),
            ordinal: entry.ordinal,
            provenance: entry.provenance,
            kind: entry.kind,
        })
        .collect();

    // 3. Only now does a record exist that carries both.
    Ok(LivingTickReceiptV2 {
        tick: payload.tick,
        applied_revisions: payload.applied_revisions,
        events,
        state_digest: payload.state_digest,
        digest,
    })
}

impl LivingTickReceiptV2 {
    /// Rebuild the payload this receipt's digest was taken over.
    pub fn payload(&self) -> LivingReceiptPayloadV2 {
        LivingReceiptPayloadV2 {
            tick: self.tick,
            applied_revisions: self.applied_revisions.clone(),
            event_payloads: self
                .events
                .iter()
                .map(|event| LivingEventPayloadV2 {
                    ordinal: event.ordinal,
                    provenance: event.provenance,
                    kind: event.kind,
                })
                .collect(),
            state_digest: self.state_digest,
        }
    }

    /// Check a decoded receipt against the derivation order: sorted revisions,
    /// emission ordinals from zero, a digest that seals its own payload, and an
    /// identity per event that the digest and ordinal derive.
    pub fn verify(&self) -> Result<(), LivingReceiptErrorV2> {
        if !strictly_ascending(&self.applied_revisions) {
            return Err(LivingReceiptErrorV2::UnsortedRevisions);
        }
        if self.events.len() > LIVING_MAX_BOUNDARY_EVENTS_V2 {
            return Err(LivingReceiptErrorV2::TooManyEvents {
                limit: LIVING_MAX_BOUNDARY_EVENTS_V2,
            });
        }
        if self.payload().digest()? != self.digest {
            return Err(LivingReceiptErrorV2::DigestMismatch);
        }
        for (position, event) in self.events.iter().enumerate() {
            let ordinal =
                u16::try_from(position).map_err(|_| LivingReceiptErrorV2::TooManyEvents {
                    limit: LIVING_MAX_BOUNDARY_EVENTS_V2,
                })?;
            if event.ordinal != ordinal || event.id != event_id_v2(self.digest, ordinal) {
                return Err(LivingReceiptErrorV2::EventIdentityMismatch);
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_pending_event_has_no_identity_to_fill_in() {
        let mut pending = LivingPendingEventsV2::new();
        assert!(pending.is_empty());
        let ordinal = pending
            .record(
                LivingEventProvenanceV2::Autonomous {
                    phase: LivingPhaseV2::DispatchOrders,
                },
                LivingEventKindV2::ShipmentDispatched,
            )
            .expect("the first event records");
        assert_eq!(ordinal, 0);
        assert_eq!(pending.len(), 1);
        let encoded =
            serde_json::to_string(&pending.payloads()[0]).expect("a payload entry encodes");
        assert!(
            !encoded.contains("\"id\""),
            "a payload entry must carry no identity: {encoded}"
        );
    }

    #[test]
    fn every_event_kind_round_trips_through_its_wire_string() {
        for kind in [
            LivingEventKindV2::AgreementExpired,
            LivingEventKindV2::CreatorIntervention,
            LivingEventKindV2::ShipmentWaiting,
            LivingEventKindV2::WarEnded,
        ] {
            let encoded = serde_json::to_string(&kind).expect("a kind encodes");
            assert_eq!(encoded, format!("\"{}\"", kind.as_wire_str()));
            assert_eq!(
                serde_json::from_str::<LivingEventKindV2>(&encoded).expect("a kind decodes"),
                kind
            );
        }
    }
}

#[cfg(test)]
mod explained_forecast_tests {
    use super::*;
    #[test]
    fn forecast_keeps_exact_ordinary_ordinals_but_cannot_seal_discarded_facts() {
        let provenance = LivingEventProvenanceV2::Autonomous {
            phase: LivingPhaseV2::RunProduction,
        };
        let mut live = LivingPendingEventsV2::new();
        let mut forecast = LivingPendingEventsV2::for_forecast();
        for _ in 0..32 {
            let facts = super::super::event_facts::LivingEventFactsV2::default();
            live.record_explained(
                provenance,
                LivingEventKindV2::FacilityProduced,
                facts.clone(),
            )
            .unwrap();
            forecast
                .record_explained(provenance, LivingEventKindV2::FacilityProduced, facts)
                .unwrap();
        }
        assert_eq!(live.payloads(), forecast.payloads());
        assert!(forecast.explained.is_empty());
        assert_eq!(forecast.explained_bytes, 0);
        let receipt = seal_living_tick_receipt_v2(
            LivingTickV2(1),
            vec![],
            &live,
            LivingStateDigestV2([1; 32]),
        )
        .unwrap();
        assert_eq!(
            receipt,
            seal_living_tick_receipt_v2(
                LivingTickV2(1),
                vec![],
                &forecast,
                LivingStateDigestV2([1; 32])
            )
            .unwrap()
        );
        assert!(
            super::super::event_facts::seal_living_event_facts_receipt_v2(&receipt, &forecast)
                .is_err()
        );
    }
}

// Opaque collector retirement is shared by producers and later sealing owners.
use super::step_job::{
    LivingStateRetirementPolicyV2, RetirementFields, RetirementPoll, retirement_node,
};
impl<'a> RetirementFields<'a> for LivingPendingEventsV2 {
    #[cfg(test)]
    fn creator_allocations(&self, out: &mut Vec<(usize, usize, Option<usize>, usize)>) {
        self.entries.creator_allocations(out);
        self.decisions.creator_allocations(out);
        self.creator_recoveries.creator_allocations(out);
        self.corridor_restrictions.creator_allocations(out);
        self.explained.creator_allocations(out);
    }
    fn retirement_child(
        &mut self,
        index: usize,
        policy: LivingStateRetirementPolicyV2,
    ) -> Option<Box<dyn RetirementPoll + 'a>> {
        match index {
            0 => Some(retirement_node(std::mem::take(&mut self.entries), policy)),
            1 => Some(retirement_node(std::mem::take(&mut self.decisions), policy)),
            2 => Some(retirement_node(
                std::mem::take(&mut self.creator_recoveries),
                policy,
            )),
            3 => Some(retirement_node(
                std::mem::take(&mut self.corridor_restrictions),
                policy,
            )),
            4 => Some(retirement_node(std::mem::take(&mut self.explained), policy)),
            _ => None,
        }
    }
}
impl<'a> RetirementFields<'a> for super::decision::LivingIntentDecisionV2 {
    fn retirement_child(
        &mut self,
        index: usize,
        policy: LivingStateRetirementPolicyV2,
    ) -> Option<Box<dyn RetirementPoll + 'a>> {
        (index == 0).then(|| retirement_node(std::mem::take(&mut self.reasons), policy))
    }
}
impl<'a> RetirementFields<'a> for super::event_facts::LivingEventFactsV2 {
    fn retirement_child(
        &mut self,
        index: usize,
        policy: LivingStateRetirementPolicyV2,
    ) -> Option<Box<dyn RetirementPoll + 'a>> {
        match index {
            0 => Some(retirement_node(std::mem::take(&mut self.resources), policy)),
            1 => Some(retirement_node(
                std::mem::take(&mut self.blocked_reasons),
                policy,
            )),
            2 => Some(retirement_node(std::mem::take(&mut self.affected), policy)),
            _ => None,
        }
    }
}

#[allow(dead_code)]
pub(crate) type LivingPendingEventsRetirementJobV2 =
    super::step_job::LivingPhaseRetirementJobV2<'static, LivingPendingEventsV2>;
impl LivingPendingEventsV2 {
    #[allow(dead_code)]
    pub(crate) fn into_retirement(self) -> LivingPendingEventsRetirementJobV2 {
        super::step_job::LivingPhaseRetirementJobV2::begin(
            self,
            LivingStateRetirementPolicyV2::ReleaseHandle,
        )
    }
}

#[cfg(test)]
mod phase_retirement_tests {
    use super::*;
    use crate::living::LivingEntityIdV2;
    #[test]
    fn phase_retirement_pending_events_drains_every_nested_facts_and_decision_buffer() {
        let mut totals = Vec::new();
        for schedule in [&[1][..], &[64][..], &[1024][..], &[1, 64, 7, 1024][..]] {
            let mut events = LivingPendingEventsV2::new();
            events.entries = Vec::with_capacity(30);
            events
                .decisions
                .push(super::super::decision::LivingIntentDecisionV2 {
                    phase: LivingPhaseV2::UpdateDiplomacy,
                    actor: LivingEntityIdV2([1; 16]),
                    ordinal: 0,
                    action: super::super::decision::LivingIntentActionV2::War {
                        peer: LivingEntityIdV2([2; 16]),
                    },
                    target: None,
                    score: super::super::decision::LivingDecisionScoreV2(0),
                    tier: super::super::decision::LivingDecisionTierV2::Ordinary,
                    snapshot_tick: LivingTickV2(1),
                    snapshot_digest: LivingStateDigestV2([0; 32]),
                    observed_tick: None,
                    outcome: super::super::decision::LivingDecisionOutcomeV2::Accepted,
                    reasons: vec![
                            super::super::decision::LivingDecisionReasonV2::MissingIntelligence {
                                world: LivingEntityIdV2([3; 16])
                            };
                            4
                        ],
                });
            events.explained.push((
                0,
                super::super::event_facts::LivingEventFactsV2 {
                    resources: Vec::with_capacity(20),
                    blocked_reasons: vec![super::super::model::LivingBlockedReasonV2::Disabled; 2],
                    affected: vec![LivingEntityIdV2([4; 16]); 3],
                    ..Default::default()
                },
            ));
            let mut job = events.into_retirement();
            let mut i = 0;
            while job.poll(schedule[i % schedule.len()]).unwrap()
                != super::super::step_job::LivingLifecycleProgressV2::Ready
            {
                i += 1;
                assert!(i < 1000);
            }
            totals.push(job.total_visits());
            assert_eq!(
                job.poll(1).unwrap(),
                super::super::step_job::LivingLifecycleProgressV2::Ready
            );
            assert_eq!(job.last_poll_visits(), 0);
        }
        assert!(totals.iter().all(|total| *total == totals[0]));
        // Empty entries/recoveries/corridors: 3 each; decision: 21;
        // explained tuple/facts tree: 42; final collector root: 1.
        assert_eq!(totals[0], 3 + 21 + 3 + 3 + 42 + 1);
    }
}

#[cfg(test)]
mod bounded_append_tests {
    use super::super::decision::*;
    use super::super::ids::LivingEntityIdV2;
    use super::super::{event_facts::*, model::*};
    use super::*;
    fn id(n: u8) -> LivingEntityIdV2 {
        LivingEntityIdV2([n; 16])
    }
    fn provenance() -> LivingEventProvenanceV2 {
        LivingEventProvenanceV2::Autonomous {
            phase: LivingPhaseV2::UpdateDiplomacy,
        }
    }
    fn facts() -> LivingEventFactsV2 {
        LivingEventFactsV2 {
            resources: vec![LivingEventResourceFactV2 {
                role: LivingEventResourceRoleV2::Input,
                resource: LivingResourceV2::Energy,
                quantity: 100,
            }],
            blocked_reasons: Vec::with_capacity(9),
            affected: vec![id(7)],
            deposit_input: Some(LivingDepositInputFactV2 {
                world: id(8),
                units: 1,
            }),
            ..Default::default()
        }
    }
    fn collector(
        entries_spare: bool,
        explained_spare: bool,
        forecast: bool,
    ) -> LivingPendingEventsV2 {
        let mut e = if forecast {
            LivingPendingEventsV2::for_forecast()
        } else {
            LivingPendingEventsV2::new()
        };
        e.entries = Vec::with_capacity(if entries_spare { 5 } else { 1 });
        e.entries.push(LivingEventPayloadV2 {
            ordinal: 0,
            provenance: provenance(),
            kind: LivingEventKindV2::FacilityBlocked,
        });
        e.explained = Vec::with_capacity(if explained_spare { 5 } else { 1 });
        let prior = LivingEventFactsV2 {
            resources: Vec::with_capacity(17),
            blocked_reasons: vec![LivingBlockedReasonV2::Disabled],
            affected: vec![id(3)],
            ..Default::default()
        };
        let bytes = serde_json::to_vec(&LivingExplainedEventV2 {
            ordinal: 0,
            event_id: LivingEventIdV2([0; 16]),
            facts: prior.clone(),
        })
        .unwrap()
        .len();
        e.explained.push((0, prior));
        e.explained_bytes = bytes;
        e.creator_recoveries = Vec::with_capacity(3);
        e.corridor_restrictions = Vec::with_capacity(4);
        e.decisions.push(LivingIntentDecisionV2 {
            phase: LivingPhaseV2::UpdateDiplomacy,
            actor: id(1),
            ordinal: 0,
            action: LivingIntentActionV2::War { peer: id(2) },
            target: None,
            score: LivingDecisionScoreV2(0),
            tier: LivingDecisionTierV2::Ordinary,
            snapshot_tick: LivingTickV2(1),
            snapshot_digest: LivingStateDigestV2([0; 32]),
            observed_tick: None,
            outcome: LivingDecisionOutcomeV2::Accepted,
            reasons: vec![LivingDecisionReasonV2::MissingIntelligence { world: id(3) }; 2],
        });
        e
    }
    // Independent pre-release outcome and original Serde accounting oracle.
    fn reference(
        e: &mut LivingPendingEventsV2,
        explained: bool,
        f: LivingEventFactsV2,
    ) -> Result<u16, LivingEventFactsErrorV2> {
        if explained && e.explanations_enabled {
            // Input fixture validity is established separately; this reference
            // never uses the new encoder to derive byte expectations.
            let ordinal = u16::try_from(e.entries.len()).map_err(|_| {
                LivingReceiptErrorV2::TooManyEvents {
                    limit: LIVING_MAX_BOUNDARY_EVENTS_V2,
                }
            })?;
            let bytes = serde_json::to_vec(&LivingExplainedEventV2 {
                ordinal,
                event_id: LivingEventIdV2([0; 16]),
                facts: f.clone(),
            })
            .unwrap()
            .len();
            let next = e
                .explained_bytes
                .checked_add(bytes + usize::from(!e.explained.is_empty()))
                .ok_or(LivingEventFactsErrorV2::Capacity)?;
            e.entries.push(LivingEventPayloadV2 {
                ordinal,
                provenance: provenance(),
                kind: LivingEventKindV2::FacilityProduced,
            });
            e.explained.push((ordinal, f));
            e.explained_bytes = next;
            Ok(ordinal)
        } else {
            let ordinal = u16::try_from(e.entries.len()).map_err(|_| {
                LivingReceiptErrorV2::TooManyEvents {
                    limit: LIVING_MAX_BOUNDARY_EVENTS_V2,
                }
            })?;
            e.entries.push(LivingEventPayloadV2 {
                ordinal,
                provenance: provenance(),
                kind: LivingEventKindV2::FacilityProduced,
            });
            Ok(ordinal)
        }
    }
    fn complete(job: &mut LivingPendingEventAppendJobV2, schedule: &[usize]) {
        let mut polls = 0;
        while job.poll(schedule[polls % schedule.len()]).unwrap()
            != LivingLifecycleProgressV2::Ready
        {
            assert!(job.last_poll_visits() <= schedule[polls % schedule.len()]);
            polls += 1;
            assert!(polls < 1_000_000);
        }
        let visits = job.total_visits();
        assert_eq!(job.poll(1), Ok(LivingLifecycleProgressV2::Ready));
        assert_eq!(job.last_poll_visits(), 0);
        assert_eq!(job.total_visits(), visits);
    }
    fn drain(job: LivingPendingEventAppendJobV2, schedule: &[usize]) -> u64 {
        let mut retirement = job.into_retirement(LivingStateRetirementPolicyV2::ReleaseHandle);
        let mut polls = 0;
        while retirement.poll(schedule[polls % schedule.len()]).unwrap()
            != LivingLifecycleProgressV2::Ready
        {
            assert!(retirement.last_poll_visits() <= schedule[polls % schedule.len()]);
            polls += 1;
            assert!(polls < 1_000_000);
        }
        assert!(retirement.take_deferred_owner().is_none());
        let total = retirement.total_visits();
        assert_eq!(retirement.poll(1), Ok(LivingLifecycleProgressV2::Ready));
        assert_eq!(retirement.last_poll_visits(), 0);
        total
    }
    fn r_facts(f: &LivingEventFactsV2) -> u64 {
        3 * (f.resources.len() + f.blocked_reasons.len() + f.affected.len()) as u64 + 10
    }
    fn r_events(e: &LivingPendingEventsV2) -> u64 {
        // Collector1 + five child edges10 + five vector roots5; leaf records3;
        // decision row edge2 + decision root1/reasons edge2/vector root1;
        // explained tuple uses RetiredPair root1/two edges4/two present
        // Options6/ordinal leaf1, plus its facts subtree.
        16 + 3
            * (e.entries.len() + e.creator_recoveries.len() + e.corridor_restrictions.len()) as u64
            + e.decisions
                .iter()
                .map(|d| 6 + 3 * d.reasons.len() as u64)
                .sum::<u64>()
            + e.explained
                .iter()
                .map(|(_, f)| 14 + r_facts(f))
                .sum::<u64>()
    }
    fn r_option<T>(value: &Option<T>, cost: impl FnOnce(&T) -> u64) -> u64 {
        value.as_ref().map_or(1, |v| 3 + cost(v))
    }
    fn r_storage(total: u64, ready: bool, n: u64, nested_delta: u64) -> u64 {
        // Exact Task2A Vec wrapper residual; Ready extraction emptied its four
        // owning Options, retaining only the approved empty lane structure.
        if ready {
            return 22;
        }
        let shell = if total <= 1 {
            25
        } else if total <= n + 2 {
            28
        } else if total == n + 3 {
            25
        } else if total == n + 4 {
            24
        } else {
            panic!("growth cut")
        };
        3 * n + shell + nested_delta
    }
    fn r_job(job: &LivingPendingEventAppendJobV2, ignored_start: Option<u64>) -> u64 {
        let entries = r_option(&job.entries_storage, |s| {
            r_storage(s.total_visits(), s.ready(), 1, 0)
        });
        // Original prior facts has two leaf-list rows: R(facts)=16;
        // pair subtree12+16=28 replaces the assumed leaf R=1.
        let explained = r_option(&job.explained_storage, |s| {
            r_storage(s.total_visits(), s.ready(), 1, 27)
        });
        let lane = if job.cleanup.is_empty() {
            7
        } else {
            let k = job.total_visits() - ignored_start.expect("counted raw facts handoff");
            // New fixture facts has two leaf-list rows: R=16.
            6 + 16 - k
        };
        15 + r_option(&job.events, r_events)
            + r_option(&job.facts, r_facts)
            + r_option(&job.payload, |_| 1)
            + r_option(&job.explained_pair, |(_, f)| 12 + r_facts(f))
            + entries
            + explained
            + lane
    }
    #[test]
    fn append_all_modes_budgets_growth_formulas_and_serde_reference() {
        for entries_spare in [false, true] {
            for explained_spare in [false, true] {
                for mode in 0..3 {
                    for schedule in [&[1][..], &[64][..], &[1024][..], &[1, 64, 7, 1024][..]] {
                        let e = collector(entries_spare, explained_spare, mode == 2);
                        let mut expected = e.clone();
                        let nested = e.explained[0].1.resources.as_ptr();
                        let inputs = e.explained[0].1.blocked_reasons.as_ptr();
                        let input = facts();
                        let resources = input.resources.as_ptr();
                        let affected = input.affected.as_ptr();
                        let ordinal = reference(&mut expected, mode != 0, input.clone()).unwrap();
                        let mut job = if mode == 0 {
                            LivingPendingEventAppendJobV2::begin_ordinary(
                                e,
                                provenance(),
                                LivingEventKindV2::FacilityProduced,
                            )
                        } else {
                            LivingPendingEventAppendJobV2::begin_explained(
                                e,
                                provenance(),
                                LivingEventKindV2::FacilityProduced,
                                input,
                            )
                        };
                        assert_eq!(job.total_visits(), 0);
                        assert_eq!(job.status(), LivingPendingEventAppendStatusV2::Pending);
                        assert_eq!(
                            job.take_ready_result(),
                            Err(LivingPendingEventAppendErrorV2::NotReady)
                        );
                        for bad in [0, 1025] {
                            assert_eq!(job.poll(bad), Err(LivingPendingEventAppendErrorV2::Budget));
                            assert_eq!(job.last_poll_visits(), 0);
                        }
                        complete(&mut job, schedule);
                        let se = if entries_spare { 1 } else { 6 };
                        let sx = if explained_spare { 1 } else { 6 };
                        let count = match mode {
                            0 => 4 + se,
                            1 => 6 + 6 + 62 + se + sx,
                            _ => 4 + se + 18,
                        };
                        assert_eq!(job.total_visits(), count);
                        let (actual, got) = job.take_ready_result().unwrap();
                        assert_eq!(got, ordinal);
                        assert_eq!(actual, expected);
                        assert_eq!(actual.explained[0].1.resources.as_ptr(), nested);
                        assert_eq!(actual.explained[0].1.blocked_reasons.as_ptr(), inputs);
                        if mode == 1 {
                            assert_eq!(actual.explained[1].1.resources.as_ptr(), resources);
                            assert_eq!(actual.explained[1].1.affected.as_ptr(), affected);
                        }
                        assert_eq!(
                            job.take_ready_result(),
                            Err(LivingPendingEventAppendErrorV2::NotReady)
                        );
                        let residual = r_job(&job, None);
                        assert_eq!(drain(job, schedule), residual);
                    }
                }
            }
        }
    }
    #[test]
    fn append_every_forward_cut_cancellation_and_typed_owner_retirement() {
        for mode in 0..3 {
            let total = match mode {
                0 => 10,
                1 => 86,
                _ => 28,
            };
            for cut in 0..=total {
                for schedule in [&[1][..], &[64][..], &[1024][..], &[1, 64, 7, 1024][..]] {
                    let e = collector(false, false, mode == 2);
                    let old_pointer = e.explained[0].1.blocked_reasons.as_ptr();
                    let input = facts();
                    let fact_pointer = input.affected.as_ptr();
                    let mut job = if mode == 0 {
                        LivingPendingEventAppendJobV2::begin_ordinary(
                            e,
                            provenance(),
                            LivingEventKindV2::FacilityProduced,
                        )
                    } else {
                        LivingPendingEventAppendJobV2::begin_explained(
                            e,
                            provenance(),
                            LivingEventKindV2::FacilityProduced,
                            input,
                        )
                    };
                    let mut ignored_start = None;
                    while job.total_visits() < cut {
                        job.poll(1).unwrap();
                        if job.stage == AppendStage::IgnoredRetire && ignored_start.is_none() {
                            ignored_start = Some(job.total_visits());
                        }
                    }
                    assert_eq!(job.total_visits(), cut);
                    if let Some(f) = &job.facts {
                        assert_eq!(f.affected.as_ptr(), fact_pointer);
                    }
                    if let Some(e) = &job.events {
                        if let Some((_, f)) = e.explained.first() {
                            assert_eq!(f.blocked_reasons.as_ptr(), old_pointer);
                        }
                        if let Some((_, f)) = e.explained.get(1) {
                            assert_eq!(f.affected.as_ptr(), fact_pointer);
                        }
                    }
                    let expected = r_job(&job, ignored_start);
                    job.cancel();
                    assert_eq!(
                        job.poll(1024),
                        Err(LivingPendingEventAppendErrorV2::Cancelled)
                    );
                    assert_eq!(job.last_poll_visits(), 0);
                    assert_eq!(job.total_visits(), cut);
                    assert_eq!(
                        job.take_ready_result(),
                        Err(LivingPendingEventAppendErrorV2::Cancelled)
                    );
                    assert_eq!(drain(job, schedule), expected, "mode{mode} cut{cut}");
                }
            }
        }
    }
    fn full(n: usize, forecast: bool) -> LivingPendingEventsV2 {
        let mut e = if forecast {
            LivingPendingEventsV2::for_forecast()
        } else {
            LivingPendingEventsV2::new()
        };
        e.entries = vec![
            LivingEventPayloadV2 {
                ordinal: 0,
                provenance: provenance(),
                kind: LivingEventKindV2::FacilityProduced
            };
            n
        ];
        e
    }
    #[test]
    fn append_ordinal_limits_and_live_forecast_fault_order_retain_oversized_facts() {
        let mut j = LivingPendingEventAppendJobV2::begin_ordinary(
            full(65535, false),
            provenance(),
            LivingEventKindV2::FacilityProduced,
        );
        complete(&mut j, &[1024]);
        assert_eq!(j.total_visits(), 4 + 65535 + 5);
        let (out, ordinal) = j.take_ready_result().unwrap();
        assert_eq!(ordinal, 65535);
        assert_eq!(out.len(), 65536);
        let mut j = LivingPendingEventAppendJobV2::begin_ordinary(
            out,
            provenance(),
            LivingEventKindV2::FacilityProduced,
        );
        assert_eq!(
            j.poll(1024),
            Err(LivingPendingEventAppendErrorV2::TooManyEvents)
        );
        assert_eq!(j.total_visits(), 2);
        assert_eq!(j.events.as_ref().unwrap().len(), 65536);
        assert_eq!(
            j.error().unwrap().receipt_error(),
            Some(LivingReceiptErrorV2::TooManyEvents { limit: 65536 })
        );
        drain(j, &[1024]);
        let mut j = LivingPendingEventAppendJobV2::begin_explained(
            full(65535, false),
            provenance(),
            LivingEventKindV2::FacilityProduced,
            LivingEventFactsV2::default(),
        );
        complete(&mut j, &[64, 1024]);
        assert_eq!(j.total_visits(), 6 + 5 + 36 + 65535 + 5 + 5);
        let (out, ordinal) = j.take_ready_result().unwrap();
        assert_eq!(ordinal, 65535);
        assert_eq!(out.explained[0].0, 65535);
        assert_eq!(
            out.explained_bytes,
            serde_json::to_vec(&LivingExplainedEventV2 {
                ordinal,
                event_id: LivingEventIdV2([0; 16]),
                facts: LivingEventFactsV2::default()
            })
            .unwrap()
            .len()
        );
        let mut j = LivingPendingEventAppendJobV2::begin_explained(
            out,
            provenance(),
            LivingEventKindV2::FacilityProduced,
            LivingEventFactsV2::default(),
        );
        assert_eq!(
            j.poll(64),
            Err(LivingPendingEventAppendErrorV2::TooManyEvents)
        );
        assert_eq!(j.total_visits(), 7);
        assert!(j.facts.is_some());
        drain(j, &[1024]);
        for forecast in [false, true] {
            let f = LivingEventFactsV2 {
                resources: Vec::with_capacity(100),
                blocked_reasons: vec![LivingBlockedReasonV2::Disabled; 100],
                affected: vec![id(9); 100],
                ..Default::default()
            };
            let pointer = f.blocked_reasons.as_ptr();
            let mut j = LivingPendingEventAppendJobV2::begin_explained(
                full(65536, forecast),
                provenance(),
                LivingEventKindV2::FacilityProduced,
                f,
            );
            let error = if forecast {
                LivingPendingEventAppendErrorV2::TooManyEvents
            } else {
                LivingPendingEventAppendErrorV2::Facts
            };
            assert_eq!(j.poll(1024), Err(error));
            assert_eq!(j.total_visits(), if forecast { 2 } else { 5 });
            assert_eq!(j.facts.as_ref().unwrap().blocked_reasons.as_ptr(), pointer);
            assert_eq!(j.events.as_ref().unwrap().len(), 65536);
            assert!(j.cleanup.is_empty());
            j.cancel();
            assert_eq!(j.error(), Some(error));
            assert_eq!(j.poll(1), Err(error));
            assert_eq!(j.last_poll_visits(), 0);
            assert_eq!(j.take_ready_result(), Err(error));
            let expected = 15 + (3 + 16 + 3 * 65536) + (3 + 610) + 1 + 1 + 1 + 1 + 7;
            assert_eq!(drain(j, &[1024]), expected);
        }
    }
    #[test]
    fn append_interspersed_ordinary_events_and_sync_compatibility_account_only_explanation_commas()
    {
        let mut e = LivingPendingEventsV2::new();
        for _ in 0..3 {
            e.record(provenance(), LivingEventKindV2::FacilityProduced)
                .unwrap();
        }
        for ordinal in [3, 5] {
            let mut original = e.clone();
            let mut synchronous = e.clone();
            assert_eq!(reference(&mut original, true, facts()), Ok(ordinal));
            assert_eq!(
                synchronous.record_explained(
                    provenance(),
                    LivingEventKindV2::FacilityProduced,
                    facts()
                ),
                Ok(ordinal)
            );
            let mut j = LivingPendingEventAppendJobV2::begin_explained(
                e,
                provenance(),
                LivingEventKindV2::FacilityProduced,
                facts(),
            );
            complete(&mut j, &[1, 7, 64]);
            let (actual, got) = j.take_ready_result().unwrap();
            assert_eq!(got, ordinal);
            assert_eq!(actual, original);
            assert_eq!(actual, synchronous);
            let expected: usize = actual
                .explained
                .iter()
                .map(|(ordinal, f)| {
                    serde_json::to_vec(&LivingExplainedEventV2 {
                        ordinal: *ordinal,
                        event_id: LivingEventIdV2([0; 16]),
                        facts: f.clone(),
                    })
                    .unwrap()
                    .len()
                })
                .sum();
            assert_eq!(
                actual.explained_bytes,
                expected + actual.explained.len() - 1
            );
            e = actual;
            e.record(provenance(), LivingEventKindV2::FacilityProduced)
                .unwrap();
        }
    }
    #[test]
    fn append_forecast_ignores_invalid_facts_but_counts_even_allocated_empty_cleanup() {
        for n in [0, 1, 100] {
            for schedule in [&[1][..], &[64][..], &[1024][..], &[1, 64, 7, 1024][..]] {
                let f = LivingEventFactsV2 {
                    resources: Vec::with_capacity(20),
                    blocked_reasons: vec![LivingBlockedReasonV2::Disabled; n],
                    affected: Vec::with_capacity(30),
                    ..Default::default()
                };
                let mut j = LivingPendingEventAppendJobV2::begin_explained(
                    LivingPendingEventsV2::for_forecast(),
                    provenance(),
                    LivingEventKindV2::FacilityProduced,
                    f,
                );
                complete(&mut j, schedule);
                assert_eq!(j.total_visits(), 4 + 5 + 3 * n as u64 + 12);
                assert!(j.cleanup.is_empty());
                assert!(j.facts.is_none());
                let (e, ordinal) = j.take_ready_result().unwrap();
                assert_eq!(ordinal, 0);
                assert_eq!(e.len(), 1);
                assert!(e.explained.is_empty());
                assert_eq!(e.explained_bytes, 0);
            }
        }
    }
    #[test]
    fn append_accounting_and_encoding_faults_precede_storage_and_are_retained() {
        let mut e = collector(false, false, false);
        e.explained_bytes = usize::MAX;
        let original = e.clone();
        let f = facts();
        let pointer = f.resources.as_ptr();
        let mut j = LivingPendingEventAppendJobV2::begin_explained(
            e,
            provenance(),
            LivingEventKindV2::FacilityProduced,
            f,
        );
        assert_eq!(j.poll(1024), Err(LivingPendingEventAppendErrorV2::Capacity));
        assert_eq!(j.total_visits(), 1 + 6 + 1 + 62 + 1);
        assert_eq!(j.events.as_ref().unwrap(), &original);
        assert_eq!(j.facts.as_ref().unwrap().resources.as_ptr(), pointer);
        assert!(j.entries_storage.is_none());
        assert!(j.explained_storage.is_none());
        assert_eq!(
            j.error().unwrap().facts_error(),
            Some(LivingEventFactsErrorV2::Capacity)
        );
        let expected = r_job(&j, None);
        assert_eq!(drain(j, &[1, 64, 1024]), expected);
        let mut j = LivingPendingEventAppendJobV2::begin_explained(
            collector(false, false, false),
            provenance(),
            LivingEventKindV2::FacilityProduced,
            facts(),
        );
        while j.stage != AppendStage::Encoding {
            j.poll(1).unwrap();
        }
        j.entry_len = None;
        j.encoding.as_mut().unwrap().reject_next_fragment();
        assert_eq!(j.poll(1024), Err(LivingPendingEventAppendErrorV2::Json));
        assert_eq!(j.total_visits(), 9);
        assert!(j.entries_storage.is_none());
        assert!(j.facts.is_some());
        assert_eq!(
            j.error().unwrap().facts_error(),
            Some(LivingEventFactsErrorV2::Wire(LivingWireErrorV2::Json))
        );
        let count = j.total_visits();
        j.cancel();
        assert_eq!(j.poll(64), Err(LivingPendingEventAppendErrorV2::Json));
        assert_eq!(j.last_poll_visits(), 0);
        assert_eq!(j.total_visits(), count);
        let expected = r_job(&j, None);
        assert_eq!(drain(j, &[64]), expected);
    }
    #[test]
    fn append_private_storage_setup_failure_retains_root_without_domain_translation() {
        let mut j = LivingPendingEventAppendJobV2::begin_explained(
            collector(false, false, false),
            provenance(),
            LivingEventKindV2::FacilityProduced,
            facts(),
        );
        while j.stage != AppendStage::PrepareEntries {
            j.poll(1).unwrap();
        }
        // Test seam replaces only known additional capacity. Original vector
        // moves into the real storage job, which retains it on checked overflow.
        j.entries_storage = Some(LivingVecStorageJobV2::begin(
            std::mem::take(&mut j.events.as_mut().unwrap().entries),
            usize::MAX,
        ));
        let before = j.total_visits();
        let error =
            LivingPendingEventAppendErrorV2::Storage(LivingStorageJobErrorV2::CapacityOverflow);
        assert_eq!(j.poll(64), Err(error));
        assert_eq!(j.total_visits(), before + 1);
        assert!(j.facts.is_some());
        assert!(error.facts_error().is_none());
        assert!(error.receipt_error().is_none());
        assert_eq!(j.poll(1), Err(error));
        assert_eq!(j.last_poll_visits(), 0);
        assert_eq!(j.take_ready_result(), Err(error));
        let expected = r_job(&j, None);
        assert_eq!(drain(j, &[1024]), expected);
    }
}

// Read-only production-parent observations. These enumerate actual retained
// partitions; observer allocations and row copies are outside producer work.
#[cfg(test)]
#[derive(Clone, Copy, Debug)]
pub(crate) struct ProductionBufferObservation {
    pub collection: u8,
    pub partition: u8,
    pub pointer: *const (),
    pub len: usize,
    pub capacity: Option<usize>,
}
#[cfg(test)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct ProductionFactsObservation {
    pub actor: Option<super::ids::LivingEntityIdV2>,
    pub buffers: [(*const (), usize, usize); 3],
}
#[cfg(test)]
#[derive(Default)]
pub(crate) struct ProductionAppendObservation {
    pub buffers: Vec<ProductionBufferObservation>,
    pub facts: Vec<ProductionFactsObservation>,
    pub observed_rows: usize,
}
#[cfg(test)]
impl ProductionAppendObservation {
    fn buffer<T>(&mut self, collection: u8, partition: u8, value: &[T], capacity: Option<usize>) {
        self.buffers.push(ProductionBufferObservation {
            collection,
            partition,
            pointer: value.as_ptr().cast(),
            len: value.len(),
            capacity,
        });
        self.observed_rows += 1;
    }
    pub(crate) fn facts(&mut self, value: &LivingEventFactsV2) {
        self.facts.push(ProductionFactsObservation {
            actor: value.actor,
            buffers: [
                (
                    value.resources.as_ptr().cast(),
                    value.resources.len(),
                    value.resources.capacity(),
                ),
                (
                    value.blocked_reasons.as_ptr().cast(),
                    value.blocked_reasons.len(),
                    value.blocked_reasons.capacity(),
                ),
                (
                    value.affected.as_ptr().cast(),
                    value.affected.len(),
                    value.affected.capacity(),
                ),
            ],
        });
        self.observed_rows += 1;
    }
    pub(crate) fn collector(&mut self, value: &LivingPendingEventsV2) {
        self.buffer(0, 0, &value.entries, Some(value.entries.capacity()));
        self.buffer(1, 0, &value.explained, Some(value.explained.capacity()));
        self.buffer(
            2,
            0,
            &value.creator_recoveries,
            Some(value.creator_recoveries.capacity()),
        );
        self.buffer(
            3,
            0,
            &value.corridor_restrictions,
            Some(value.corridor_restrictions.capacity()),
        );
        for (_, facts) in &value.explained {
            self.facts(facts);
        }
    }
    fn storage<T: RetirementFields<'static>>(
        &mut self,
        collection: u8,
        value: &LivingVecStorageJobV2<'static, T>,
    ) {
        let value = value.production_partitions();
        if let Some(v) = value.original {
            self.buffer(collection, 1, v, Some(v.capacity()));
        }
        if let Some(v) = value.source {
            self.buffer(collection, 2, v.as_slice(), None);
        }
        if let Some(v) = value.destination {
            self.buffer(collection, 3, v, Some(v.capacity()));
        }
    }
}
#[cfg(test)]
impl LivingPendingEventAppendJobV2 {
    pub(crate) fn production_owner_observation(&self) -> ProductionAppendObservation {
        let mut observation = ProductionAppendObservation::default();
        if let Some(events) = &self.events {
            observation.collector(events);
        }
        if let Some(facts) = &self.facts {
            observation.facts(facts);
        }
        if let Some((_, facts)) = &self.explained_pair {
            observation.facts(facts);
        }
        if let Some(storage) = &self.entries_storage {
            observation.storage(0, storage);
        }
        if let Some(storage) = &self.explained_storage {
            observation.storage(1, storage);
            let partitions = storage.production_partitions();
            for (_, facts) in partitions
                .original
                .into_iter()
                .flatten()
                .chain(partitions.source.into_iter().flat_map(|v| v.as_slice()))
                .chain(partitions.destination.into_iter().flatten())
                .chain(partitions.pending)
            {
                observation.facts(facts);
            }
        }
        observation
    }
    pub(crate) fn production_empty_source(
        &self,
        explained: bool,
        pointer: *const (),
    ) -> Option<usize> {
        if explained {
            self.explained_storage
                .as_ref()
                .and_then(|s| s.production_empty_source(pointer))
        } else {
            self.entries_storage
                .as_ref()
                .and_then(|s| s.production_empty_source(pointer))
        }
    }
}
#[cfg(test)]
pub(crate) fn production_append_collector_fixture() -> LivingPendingEventsV2 {
    let mut events = LivingPendingEventsV2::new();
    events.entries = Vec::with_capacity(1);
    events.explained = Vec::with_capacity(1);
    events.creator_recoveries = Vec::with_capacity(3);
    events.corridor_restrictions = Vec::with_capacity(4);
    events
        .record_explained(
            LivingEventProvenanceV2::Autonomous {
                phase: LivingPhaseV2::RunProduction,
            },
            LivingEventKindV2::FacilityBlocked,
            LivingEventFactsV2 {
                actor: Some(super::ids::LivingEntityIdV2([71; 16])),
                resources: vec![super::event_facts::LivingEventResourceFactV2 {
                    role: super::event_facts::LivingEventResourceRoleV2::Input,
                    resource: super::model::LivingResourceV2::Energy,
                    quantity: 1,
                }],
                blocked_reasons: vec![super::model::LivingBlockedReasonV2::Disabled],
                affected: vec![super::ids::LivingEntityIdV2([3; 16])],
                ..Default::default()
            },
        )
        .unwrap();
    assert_eq!(events.entries.capacity(), 1);
    assert_eq!(events.explained.capacity(), 1);
    events
}
