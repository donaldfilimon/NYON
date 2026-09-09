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
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct LivingPendingEventsV2 {
    entries: Vec<LivingEventPayloadV2>,
}

impl LivingPendingEventsV2 {
    /// An empty boundary.
    pub const fn new() -> Self {
        Self {
            entries: Vec::new(),
        }
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
