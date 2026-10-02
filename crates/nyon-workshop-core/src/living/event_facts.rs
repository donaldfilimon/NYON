//! Additive, separately sealed facts emitted by ordinary-event producers.
//! Ordinary receipt payloads and identities remain unchanged. Missing facts
//! mean unknown; consumers never reconstruct them from a later state.
use super::{
    ids::*,
    model::{LivingAgreementV2, LivingBlockedReasonV2, LivingResourceV2, LivingRouteKindV2},
    receipt::{LivingEventKindV2, LivingPendingEventsV2, LivingTickReceiptV2},
    wire::{LivingWireErrorV2, decode_canonical_v2, encode_canonical_v2},
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
/// Independent explanation domain.
pub const LIVING_EVENT_FACTS_DOMAIN_V2: &[u8] = b"NYON-LIVING-EVENT-FACTS-V2\0";
/// Independent explanation byte bound.
pub const LIVING_MAX_EVENT_FACTS_BYTES_V2: usize = 2 * 1024 * 1024;
/// Ephemeral replay-regenerated route restriction; excluded from canonical bytes.
/// This observes a restriction, not the cause of a later industry's shortage.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LivingCorridorRestrictionV2 {
    pub(crate) tick: LivingTickV2,
    pub(crate) receipt_digest: LivingReceiptDigestV2,
    pub(crate) route: LivingEntityIdV2,
    pub(crate) source_world: LivingEntityIdV2,
    pub(crate) destination_world: LivingEntityIdV2,
    pub(crate) source_system: LivingEntityIdV2,
    pub(crate) destination_system: LivingEntityIdV2,
    pub(crate) sender: LivingEntityIdV2,
    pub(crate) receiver: LivingEntityIdV2,
    pub(crate) resource: LivingResourceV2,
    pub(crate) requested_batch: u32,
    pub(crate) eligible_batch: u32,
    pub(crate) restricted_batch: u32,
    pub(crate) actual_dispatched_units: u32,
    pub(crate) hazard: super::model::LivingHazardV2,
}
impl LivingCorridorRestrictionV2 {
    pub const fn tick(&self) -> LivingTickV2 {
        self.tick
    }
    pub const fn receipt_digest(&self) -> LivingReceiptDigestV2 {
        self.receipt_digest
    }
    pub const fn route(&self) -> LivingEntityIdV2 {
        self.route
    }
    pub const fn source_world(&self) -> LivingEntityIdV2 {
        self.source_world
    }
    pub const fn destination_world(&self) -> LivingEntityIdV2 {
        self.destination_world
    }
    pub const fn source_system(&self) -> LivingEntityIdV2 {
        self.source_system
    }
    pub const fn destination_system(&self) -> LivingEntityIdV2 {
        self.destination_system
    }
    pub const fn sender(&self) -> LivingEntityIdV2 {
        self.sender
    }
    pub const fn receiver(&self) -> LivingEntityIdV2 {
        self.receiver
    }
    pub const fn resource(&self) -> LivingResourceV2 {
        self.resource
    }
    pub const fn requested_batch(&self) -> u32 {
        self.requested_batch
    }
    pub const fn eligible_batch(&self) -> u32 {
        self.eligible_batch
    }
    pub const fn restricted_batch(&self) -> u32 {
        self.restricted_batch
    }
    /// Accepted shipment units; zero also represents a legitimate restriction to zero.
    pub const fn actual_dispatched_units(&self) -> u32 {
        self.actual_dispatched_units
    }
    pub fn hazard(&self) -> &super::model::LivingHazardV2 {
        &self.hazard
    }
}
/// What an actual resource quantity describes; order is canonical wire order.
#[derive(Clone, Copy, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum LivingEventResourceRoleV2 {
    Discarded,
    Input,
    Output,
    Paid,
    Refund,
    Transferred,
}
/// A measured quantity, never a forecast or inferred inventory delta.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct LivingEventResourceFactV2 {
    pub role: LivingEventResourceRoleV2,
    pub resource: LivingResourceV2,
    pub quantity: u32,
}
/// Successful aggregate deposit consumption; no individual deposit identity is inferred.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct LivingDepositInputFactV2 {
    pub world: LivingEntityIdV2,
    pub units: u32,
}
/// Actual accepted destination inventory transfer, recorded at delivery.
/// Sender/endpoints belong to the immutable manifest; systems describe the
/// available world records at this boundary, never the historical departure.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct LivingDeliveryFactV2 {
    pub source_world: LivingEntityIdV2,
    pub destination_world: LivingEntityIdV2,
    pub dispatch_owner: LivingEntityIdV2,
    pub intended_receiver: LivingEntityIdV2,
    pub recipient: LivingEntityIdV2,
    pub resource: LivingResourceV2,
    pub units: u32,
    /// None retains the honest unknown classification of legacy manifests.
    pub dispatch_kind: Option<LivingRouteKindV2>,
    /// The source world's current system, when the world still exists.
    pub source_system: Option<LivingEntityIdV2>,
    /// The destination world's system at the delivery boundary, when known.
    pub destination_system: Option<LivingEntityIdV2>,
}
/// Producer-supplied facts, with bounded sorted lists and no freeform motives.
#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct LivingEventFactsV2 {
    pub actor: Option<LivingEntityIdV2>,
    pub entity: Option<LivingEntityIdV2>,
    pub resources: Vec<LivingEventResourceFactV2>,
    pub blocked_reasons: Vec<LivingBlockedReasonV2>,
    pub affected: Vec<LivingEntityIdV2>,
    /// Sparse additive success witness, omitted when absent to preserve old bytes.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub deposit_input: Option<LivingDepositInputFactV2>,
    /// The exact bilateral agreement accepted at this formation boundary.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub agreement: Option<LivingAgreementV2>,
    /// The actual full inventory transfer, never an attempted delivery.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub delivery: Option<LivingDeliveryFactV2>,
}
impl LivingEventFactsV2 {
    /// Validate before appending the ordinary event, preserving rejected collectors.
    pub fn validate(&self, kind: LivingEventKindV2) -> Result<(), LivingEventFactsErrorV2> {
        if self.agreement.as_ref().is_some_and(|agreement| {
            kind != LivingEventKindV2::AgreementFormed
                || agreement.participant_a >= agreement.participant_b
                || agreement.start_tick >= agreement.end_tick
        }) || self.delivery.as_ref().is_some_and(|delivery| {
            kind != LivingEventKindV2::ShipmentDelivered
                || delivery.recipient != delivery.intended_receiver
                || delivery.units == 0
                || delivery.units > 10_000
                || (delivery.dispatch_kind == Some(LivingRouteKindV2::Internal)
                    && delivery.dispatch_owner != delivery.intended_receiver)
        }) {
            return Err(LivingEventFactsErrorV2::Facts);
        }
        if self.deposit_input.as_ref().is_some_and(|draw| {
            kind != LivingEventKindV2::FacilityProduced || draw.units == 0 || draw.units > 10_000
        }) {
            return Err(LivingEventFactsErrorV2::Facts);
        }
        if self.resources.len() > 18
            || self.blocked_reasons.len() > 8
            || self.affected.len() > 64
            || self.resources.iter().any(|r| r.quantity > 10_000)
            || !self
                .resources
                .windows(2)
                .all(|p| (p[0].role, p[0].resource) < (p[1].role, p[1].resource))
            || !self.affected.windows(2).all(|p| p[0] < p[1])
        {
            return Err(LivingEventFactsErrorV2::Facts);
        }
        if kind != LivingEventKindV2::FacilityBlocked && !self.blocked_reasons.is_empty() {
            return Err(LivingEventFactsErrorV2::Facts);
        }
        if kind == LivingEventKindV2::FacilityBlocked && self.blocked_reasons.is_empty() {
            return Err(LivingEventFactsErrorV2::Facts);
        }
        let key = |r: &LivingBlockedReasonV2| match r {
            LivingBlockedReasonV2::Disabled => (0, None),
            LivingBlockedReasonV2::MissingDeposit { .. } => (1, None),
            LivingBlockedReasonV2::MissingInput { resource, .. } => (2, Some(*resource)),
            LivingBlockedReasonV2::OutputFull { resource } => (3, Some(*resource)),
        };
        if !self
            .blocked_reasons
            .windows(2)
            .all(|p| key(&p[0]) < key(&p[1]))
        {
            return Err(LivingEventFactsErrorV2::Facts);
        }
        for reason in &self.blocked_reasons {
            if let LivingBlockedReasonV2::MissingDeposit {
                required,
                available,
            }
            | LivingBlockedReasonV2::MissingInput {
                required,
                available,
                ..
            } = reason
                && (*required > 10_000 || *available >= *required)
            {
                return Err(LivingEventFactsErrorV2::Facts);
            }
        }
        Ok(())
    }
}
/// One exact ordinary emission ordinal and its derived identity.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct LivingExplainedEventV2 {
    pub ordinal: u16,
    pub event_id: LivingEventIdV2,
    pub facts: LivingEventFactsV2,
}
/// Why requested producer explanations are unavailable.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum LivingEventFactsOmissionReasonV2 {
    Capacity,
}
/// Inclusive ordinary-event ordinal ranges whose requested facts did not fit.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct LivingEventFactsOmittedV2 {
    pub reason: LivingEventFactsOmissionReasonV2,
    pub ranges: Vec<[u16; 2]>,
}
/// Separately authenticated facts bound to one ordinary receipt.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct LivingEventFactsReceiptV2 {
    pub tick: LivingTickV2,
    pub state_digest: LivingStateDigestV2,
    pub receipt_digest: LivingReceiptDigestV2,
    pub events: Vec<LivingExplainedEventV2>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub omitted: Option<LivingEventFactsOmittedV2>,
    pub integrity_sha256: LivingReceiptDigestV2,
}
#[derive(Serialize)]
struct Payload<'a> {
    tick: LivingTickV2,
    state_digest: LivingStateDigestV2,
    receipt_digest: LivingReceiptDigestV2,
    events: &'a [LivingExplainedEventV2],
    #[serde(skip_serializing_if = "Option::is_none")]
    omitted: &'a Option<LivingEventFactsOmittedV2>,
}
/// Typed collector and binding failures.
#[derive(Clone, Copy, Debug, Eq, PartialEq, thiserror::Error)]
pub enum LivingEventFactsErrorV2 {
    #[error("invalid event facts wire: {0}")]
    Wire(#[from] LivingWireErrorV2),
    #[error("ordinary event collector rejected: {0}")]
    Receipt(#[from] super::receipt::LivingReceiptErrorV2),
    #[error("event facts capacity exceeded")]
    Capacity,
    #[error("invalid bounded event facts")]
    Facts,
    #[error("event facts do not bind this ordinary receipt")]
    Binding,
    #[error("event facts integrity mismatch")]
    Integrity,
}
impl LivingEventFactsReceiptV2 {
    fn calculated_digest(&self) -> Result<LivingReceiptDigestV2, LivingEventFactsErrorV2> {
        let bytes = encode_canonical_v2(
            &Payload {
                tick: self.tick,
                state_digest: self.state_digest,
                receipt_digest: self.receipt_digest,
                events: &self.events,
                omitted: &self.omitted,
            },
            LIVING_MAX_EVENT_FACTS_BYTES_V2,
        )?;
        let mut h = Sha256::new();
        h.update(LIVING_EVENT_FACTS_DOMAIN_V2);
        h.update(bytes);
        Ok(LivingReceiptDigestV2(h.finalize().into()))
    }
    /// Validate the entire sidecar and ordinary receipt before publishing any facts.
    pub fn verify_receipt(
        &self,
        receipt: &LivingTickReceiptV2,
    ) -> Result<(), LivingEventFactsErrorV2> {
        self.canonical_bytes()?;
        receipt.verify()?;
        if self.tick != receipt.tick
            || self.state_digest != receipt.state_digest
            || self.receipt_digest != receipt.digest
            || !self.events.windows(2).all(|p| p[0].ordinal < p[1].ordinal)
        {
            return Err(LivingEventFactsErrorV2::Binding);
        }
        for entry in &self.events {
            let e = receipt
                .events
                .get(usize::from(entry.ordinal))
                .ok_or(LivingEventFactsErrorV2::Binding)?;
            if e.ordinal != entry.ordinal || e.id != entry.event_id {
                return Err(LivingEventFactsErrorV2::Binding);
            }
            entry.facts.validate(e.kind)?;
            if entry
                .facts
                .agreement
                .as_ref()
                .is_some_and(|agreement| agreement.start_tick != receipt.tick)
            {
                return Err(LivingEventFactsErrorV2::Binding);
            }
        }
        if let Some(omitted) = &self.omitted {
            if omitted.ranges.is_empty()
                || self
                    .events
                    .last()
                    .is_some_and(|event| omitted.ranges[0][0] <= event.ordinal)
                || omitted
                    .ranges
                    .iter()
                    .any(|[start, end]| start > end || usize::from(*end) >= receipt.events.len())
                || !omitted
                    .ranges
                    .windows(2)
                    .all(|pair| u32::from(pair[0][1]) + 1 < u32::from(pair[1][0]))
            {
                return Err(LivingEventFactsErrorV2::Binding);
            }
            let mut ranges = omitted.ranges.iter().peekable();
            for event in &self.events {
                while ranges.peek().is_some_and(|range| range[1] < event.ordinal) {
                    ranges.next();
                }
                if ranges.peek().is_some_and(|range| range[0] <= event.ordinal) {
                    return Err(LivingEventFactsErrorV2::Binding);
                }
            }
        }
        if self.integrity_sha256 != self.calculated_digest()? {
            return Err(LivingEventFactsErrorV2::Integrity);
        }
        Ok(())
    }
    /// Canonical bytes, bounded independently of ordinary receipts.
    pub fn canonical_bytes(&self) -> Result<Vec<u8>, LivingEventFactsErrorV2> {
        Ok(encode_canonical_v2(self, LIVING_MAX_EVENT_FACTS_BYTES_V2)?)
    }
    /// Strict decoding plus complete pair authentication.
    pub fn decode(
        bytes: &[u8],
        receipt: &LivingTickReceiptV2,
    ) -> Result<Self, LivingEventFactsErrorV2> {
        let result: Self = decode_canonical_v2(bytes, LIVING_MAX_EVENT_FACTS_BYTES_V2)?;
        result.verify_receipt(receipt)?;
        Ok(result)
    }
}
/// Seal actual collector facts after ordinary identities have been derived.
pub fn seal_living_event_facts_receipt_v2(
    receipt: &LivingTickReceiptV2,
    pending: &LivingPendingEventsV2,
) -> Result<LivingEventFactsReceiptV2, LivingEventFactsErrorV2> {
    receipt.verify()?;
    if !pending.explanations_enabled() {
        return Err(LivingEventFactsErrorV2::Binding);
    }
    if pending.payloads().len() != receipt.events.len()
        || pending
            .payloads()
            .iter()
            .zip(&receipt.events)
            .any(|(p, e)| {
                p.ordinal != e.ordinal || p.kind != e.kind || p.provenance != e.provenance
            })
    {
        return Err(LivingEventFactsErrorV2::Binding);
    }
    let candidates = pending
        .explained
        .iter()
        .map(|(ordinal, facts)| {
            let e = receipt
                .events
                .get(usize::from(*ordinal))
                .ok_or(LivingEventFactsErrorV2::Binding)?;
            Ok(LivingExplainedEventV2 {
                ordinal: *ordinal,
                event_id: e.id,
                facts: facts.clone(),
            })
        })
        .collect::<Result<Vec<_>, LivingEventFactsErrorV2>>()?;
    let build = |retained: usize| {
        let mut ranges: Vec<[u16; 2]> = Vec::new();
        for event in &candidates[retained..] {
            if let Some(last) = ranges.last_mut()
                && u32::from(last[1]) + 1 == u32::from(event.ordinal)
            {
                last[1] = event.ordinal;
            } else {
                ranges.push([event.ordinal, event.ordinal]);
            }
        }
        LivingEventFactsReceiptV2 {
            tick: receipt.tick,
            state_digest: receipt.state_digest,
            receipt_digest: receipt.digest,
            events: candidates[..retained].to_vec(),
            omitted: (!ranges.is_empty()).then_some(LivingEventFactsOmittedV2 {
                reason: LivingEventFactsOmissionReasonV2::Capacity,
                ranges,
            }),
            integrity_sha256: LivingReceiptDigestV2([0; 32]),
        }
    };
    // Every detailed entry costs more than its corresponding range boundary;
    // therefore serialized size is monotone in the retained ordinal prefix.
    // Include the complete envelope, fixed-width integrity and omission marker.
    let mut low = 0;
    let mut high = candidates.len();
    while low < high {
        let middle = low + (high - low).div_ceil(2);
        if build(middle).canonical_bytes().is_ok() {
            low = middle;
        } else {
            high = middle - 1;
        }
    }
    let mut r = build(low);
    r.canonical_bytes()?;
    r.integrity_sha256 = r.calculated_digest()?;
    r.verify_receipt(receipt)?;
    Ok(r)
}
