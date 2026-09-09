//! Reviewed exact-byte tests for Living Galaxy V2 tick receipts and events.
//!
//! The expected canonical bytes, receipt digest and event identities come from
//! the `receipt_payload` section of `tests/fixtures/living-v2/vectors.json`,
//! which was derived by `tools/living-v2-vectors.py` from section 10 of
//! `docs/superpowers/specs/2026-09-04-nyon-living-galaxy-rules.md` rather than
//! from this crate's output. That independence is the whole point: a corpus
//! regenerated from the implementation would prove only that the code agrees
//! with itself.

use nyon_workshop_core::living::receipt::LivingPendingEventsV2;
use nyon_workshop_core::living::{
    LIVING_MAX_ARCHIVE_BYTES_V2, LIVING_MAX_BOUNDARY_EVENTS_V2, LivingEventKindV2,
    LivingEventPayloadV2, LivingEventProvenanceV2, LivingEventV2, LivingPhaseV2,
    LivingReceiptDigestV2, LivingReceiptErrorV2, LivingReceiptPayloadV2, LivingRevisionIdV2,
    LivingStateDigestV2, LivingTickReceiptV2, LivingTickV2, LivingWireErrorV2, decode_canonical_v2,
    event_id_v2, receipt_digest_v2, seal_living_tick_receipt_v2,
};
use serde_json::Value;

const VECTORS: &str = include_str!("fixtures/living-v2/vectors.json");

fn vectors() -> Value {
    serde_json::from_str(VECTORS).expect("reviewed Living V2 vectors parse")
}

fn hex32(value: &Value) -> [u8; 32] {
    let text = value.as_str().expect("a 32-byte hexadecimal string");
    let bytes = hex_bytes(text);
    <[u8; 32]>::try_from(bytes.as_slice()).expect("32 bytes")
}

fn hex16(value: &Value) -> [u8; 16] {
    let text = value.as_str().expect("a 16-byte hexadecimal string");
    let bytes = hex_bytes(text);
    <[u8; 16]>::try_from(bytes.as_slice()).expect("16 bytes")
}

fn hex_bytes(text: &str) -> Vec<u8> {
    assert!(text.len().is_multiple_of(2), "hexadecimal is byte aligned");
    (0..text.len() / 2)
        .map(|index| {
            u8::from_str_radix(&text[index * 2..index * 2 + 2], 16).expect("lowercase hexadecimal")
        })
        .collect()
}

fn revision(index: usize) -> LivingRevisionIdV2 {
    LivingRevisionIdV2(hex32(&vectors()["revisions"][index]["revision_id"]))
}

/// The exact boundary the reviewed `receipt_payload` vector describes: two
/// applied revisions, a creator event and an autonomous dispatch event.
fn reviewed_boundary() -> (
    LivingTickV2,
    Vec<LivingRevisionIdV2>,
    LivingPendingEventsV2,
    LivingStateDigestV2,
) {
    let mut pending = LivingPendingEventsV2::new();
    pending
        .record(
            LivingEventProvenanceV2::Creator {
                revision: revision(0),
            },
            LivingEventKindV2::CreatorIntervention,
        )
        .expect("the creator event records");
    pending
        .record(
            LivingEventProvenanceV2::Autonomous {
                phase: LivingPhaseV2::DispatchOrders,
            },
            LivingEventKindV2::ShipmentDispatched,
        )
        .expect("the dispatch event records");

    let state_digest = LivingStateDigestV2(hex32(&vectors()["receipt_payload"]["state_digest"]));
    (
        LivingTickV2(50),
        vec![revision(0), revision(1)],
        pending,
        state_digest,
    )
}

// ------------------------------------------------------------ exact vectors

/// The payload record must encode to the exact bytes the reviewed corpus
/// carries. This is the assertion that pins the four field names, their order,
/// the `{ordinal, provenance, kind}` entry order, the `type`-first tagged
/// provenance, and the phase travelling as its published ordinal -- all at
/// once, because any one of them changes these bytes.
#[test]
fn the_receipt_payload_encodes_to_the_reviewed_canonical_bytes() {
    let (tick, applied_revisions, pending, state_digest) = reviewed_boundary();
    let payload = LivingReceiptPayloadV2 {
        tick,
        applied_revisions,
        event_payloads: pending.payloads().to_vec(),
        state_digest,
    };

    let expected = hex_bytes(
        vectors()["receipt_payload"]["canonical_bytes_hex"]
            .as_str()
            .expect("canonical bytes"),
    );
    let encoded = payload.canonical_bytes().expect("the payload encodes");
    assert_eq!(
        encoded,
        expected,
        "encoded as {}",
        String::from_utf8_lossy(&encoded)
    );

    assert_eq!(
        payload.digest().expect("the payload hashes").0,
        hex32(&vectors()["receipt_payload"]["receipt_digest"])
    );
}

/// The payload entries carry no identity field at all. A `null` or a zeroed
/// placeholder would still be a field, would still be hashed, and would still
/// break the derivation order the spec fixes.
#[test]
fn no_payload_entry_carries_an_identity_field() {
    let (_, _, pending, _) = reviewed_boundary();
    let encoded = serde_json::to_string(pending.payloads()).expect("entries encode");
    assert!(!encoded.contains("\"id\""), "{encoded}");
    assert!(!encoded.contains("null"), "{encoded}");
}

/// The sealed receipt's event identities must be the ones the corpus derives
/// from the receipt digest, in emission order.
#[test]
fn sealing_derives_the_reviewed_event_identities() {
    let (tick, applied_revisions, pending, state_digest) = reviewed_boundary();
    let receipt = seal_living_tick_receipt_v2(tick, applied_revisions, &pending, state_digest)
        .expect("the boundary seals");

    let corpus = vectors();
    assert_eq!(
        receipt.digest.0,
        hex32(&corpus["receipt_payload"]["receipt_digest"])
    );
    let rows = corpus["receipt_payload"]["events"]
        .as_array()
        .expect("event rows");
    assert_eq!(receipt.events.len(), rows.len());
    for (event, row) in receipt.events.iter().zip(rows) {
        assert_eq!(
            u64::from(event.ordinal),
            row["ordinal"].as_u64().expect("ordinal")
        );
        assert_eq!(event.id.0, hex16(&row["event_id"]));
    }
    assert_eq!(
        receipt.events[0].kind,
        LivingEventKindV2::CreatorIntervention
    );
    assert_eq!(
        receipt.events[1].provenance,
        LivingEventProvenanceV2::Autonomous {
            phase: LivingPhaseV2::DispatchOrders
        }
    );
    receipt.verify().expect("a freshly sealed receipt verifies");
}

/// The derivation order is normative and this is the test that would catch its
/// inversion. An implementation that hashed the public receipt -- the one that
/// already contains the event identities -- would produce a different digest
/// for the same boundary, so the corpus digest and the corpus identities can
/// never both be reproduced by that mistake.
#[test]
fn hashing_the_public_receipt_instead_of_the_payload_yields_a_different_digest() {
    let (tick, applied_revisions, pending, state_digest) = reviewed_boundary();
    let receipt = seal_living_tick_receipt_v2(tick, applied_revisions, &pending, state_digest)
        .expect("the boundary seals");

    let public_bytes = serde_json::to_vec(&receipt).expect("the public receipt encodes");
    let wrong = receipt_digest_v2(&public_bytes);
    assert_ne!(
        wrong, receipt.digest,
        "the payload and the public receipt must not hash alike, or the record split is cosmetic"
    );

    // And the identities really do come from the payload digest, not the other.
    for event in &receipt.events {
        assert_eq!(event.id, event_id_v2(receipt.digest, event.ordinal));
        assert_ne!(event.id, event_id_v2(wrong, event.ordinal));
    }
}

// ------------------------------------------------------------ ordinal rules

#[test]
fn ordinals_are_assigned_in_emission_order_from_zero() {
    let mut pending = LivingPendingEventsV2::new();
    assert!(pending.is_empty());
    for expected in 0_u16..5 {
        let ordinal = pending
            .record(
                LivingEventProvenanceV2::Autonomous {
                    phase: LivingPhaseV2::RunProduction,
                },
                LivingEventKindV2::FacilityBlocked,
            )
            .expect("an event records");
        assert_eq!(ordinal, expected);
    }
    assert_eq!(pending.len(), 5);
    let ordinals: Vec<u16> = pending
        .payloads()
        .iter()
        .map(|entry| entry.ordinal)
        .collect();
    assert_eq!(ordinals, vec![0, 1, 2, 3, 4]);
}

/// Emission order is meaningful and is not a sorted map, so two events with the
/// same provenance and kind still receive different identities.
#[test]
fn two_identical_events_differ_only_by_ordinal_and_still_receive_distinct_identities() {
    let mut pending = LivingPendingEventsV2::new();
    for _ in 0..2 {
        pending
            .record(
                LivingEventProvenanceV2::Autonomous {
                    phase: LivingPhaseV2::ResolveArrivals,
                },
                LivingEventKindV2::ShipmentDelivered,
            )
            .expect("an event records");
    }
    let receipt = seal_living_tick_receipt_v2(
        LivingTickV2(11),
        Vec::new(),
        &pending,
        LivingStateDigestV2([0x22; 32]),
    )
    .expect("the boundary seals");
    assert_ne!(receipt.events[0].id, receipt.events[1].id);
    assert_eq!(receipt.events[0].kind, receipt.events[1].kind);
}

#[test]
fn the_boundary_event_bound_is_the_ordinal_space() {
    assert_eq!(LIVING_MAX_BOUNDARY_EVENTS_V2, 65_536);
}

// -------------------------------------------------------------- rejections

#[test]
fn unsorted_or_repeated_applied_revisions_are_refused_rather_than_sorted() {
    let pending = LivingPendingEventsV2::new();
    for revisions in [
        vec![revision(1), revision(0)],
        vec![revision(0), revision(0)],
    ] {
        assert_eq!(
            seal_living_tick_receipt_v2(
                LivingTickV2(3),
                revisions,
                &pending,
                LivingStateDigestV2([0x33; 32]),
            ),
            Err(LivingReceiptErrorV2::UnsortedRevisions)
        );
    }

    // The ascending pair is accepted, so the rejection is about order and not
    // about the revisions themselves.
    assert!(
        seal_living_tick_receipt_v2(
            LivingTickV2(3),
            vec![revision(0), revision(1)],
            &pending,
            LivingStateDigestV2([0x33; 32]),
        )
        .is_ok()
    );
}

#[test]
fn a_receipt_whose_digest_does_not_seal_its_payload_fails_verification() {
    let (tick, applied_revisions, pending, state_digest) = reviewed_boundary();
    let mut receipt = seal_living_tick_receipt_v2(tick, applied_revisions, &pending, state_digest)
        .expect("the boundary seals");
    receipt.digest = LivingReceiptDigestV2([0x44; 32]);
    assert_eq!(receipt.verify(), Err(LivingReceiptErrorV2::DigestMismatch));
}

/// Every field of the payload reaches the digest, so mutating any one of the
/// four invalidates the seal. Without this, a receipt could carry a state
/// digest or an applied revision the digest never covered.
#[test]
fn every_payload_field_is_covered_by_the_digest() {
    let (tick, applied_revisions, pending, state_digest) = reviewed_boundary();
    let sealed = seal_living_tick_receipt_v2(tick, applied_revisions, &pending, state_digest)
        .expect("the boundary seals");

    let mutations: [fn(&mut LivingTickReceiptV2); 4] = [
        |receipt| receipt.tick = LivingTickV2(51),
        |receipt| receipt.applied_revisions.truncate(1),
        |receipt| receipt.events[1].kind = LivingEventKindV2::ShipmentWaiting,
        |receipt| receipt.state_digest = LivingStateDigestV2([0x55; 32]),
    ];
    for mutate in mutations {
        let mut receipt = sealed.clone();
        mutate(&mut receipt);
        assert_eq!(
            receipt.verify(),
            Err(LivingReceiptErrorV2::DigestMismatch),
            "a mutated payload field must break the seal"
        );
    }
}

#[test]
fn an_event_identity_the_receipt_does_not_derive_fails_verification() {
    let (tick, applied_revisions, pending, state_digest) = reviewed_boundary();
    let sealed = seal_living_tick_receipt_v2(tick, applied_revisions, &pending, state_digest)
        .expect("the boundary seals");

    // Swapping the two identities alone leaves the payload untouched, so the
    // digest still seals it and the identity check is what refuses this.
    let mut swapped_identities = sealed.clone();
    let first = swapped_identities.events[0].id;
    swapped_identities.events[0].id = swapped_identities.events[1].id;
    swapped_identities.events[1].id = first;
    assert_eq!(
        swapped_identities.verify(),
        Err(LivingReceiptErrorV2::EventIdentityMismatch),
        "an identity must belong to its own ordinal"
    );

    // Swapping whole events also permutes the payload entries, so that one is
    // caught one step earlier, by the seal. Both are refusals; the difference
    // says which invariant noticed.
    let mut swapped_events = sealed.clone();
    swapped_events.events.swap(0, 1);
    assert_eq!(
        swapped_events.verify(),
        Err(LivingReceiptErrorV2::DigestMismatch),
        "emission order is inside the hashed payload"
    );

    let mut zeroed = sealed;
    zeroed.events[0].id = nyon_workshop_core::living::LivingEventIdV2([0; 16]);
    assert_eq!(
        zeroed.verify(),
        Err(LivingReceiptErrorV2::EventIdentityMismatch),
        "a zeroed identity is exactly the shortcut the spec forbids"
    );
}

// --------------------------------------------------------------- wire rules

#[test]
fn the_public_receipt_round_trips_through_the_canonical_wire() {
    let (tick, applied_revisions, pending, state_digest) = reviewed_boundary();
    let receipt = seal_living_tick_receipt_v2(tick, applied_revisions, &pending, state_digest)
        .expect("the boundary seals");

    let bytes = serde_json::to_vec(&receipt).expect("the receipt encodes");
    let decoded: LivingTickReceiptV2 =
        decode_canonical_v2(&bytes, LIVING_MAX_ARCHIVE_BYTES_V2).expect("the receipt decodes");
    assert_eq!(decoded, receipt);
    decoded.verify().expect("a decoded receipt verifies");
}

#[test]
fn receipt_records_reject_unknown_and_reordered_fields() {
    let unknown = br#"{"ordinal":0,"provenance":{"type":"autonomous","phase":3},"kind":"fleet_arrived","extra":1}"#;
    assert_eq!(
        decode_canonical_v2::<LivingEventPayloadV2>(unknown, LIVING_MAX_ARCHIVE_BYTES_V2),
        Err(LivingWireErrorV2::Json)
    );

    let reordered =
        br#"{"provenance":{"type":"autonomous","phase":3},"ordinal":0,"kind":"fleet_arrived"}"#;
    assert_eq!(
        decode_canonical_v2::<LivingEventPayloadV2>(reordered, LIVING_MAX_ARCHIVE_BYTES_V2),
        Err(LivingWireErrorV2::NonCanonical)
    );

    let tag_last =
        br#"{"ordinal":0,"provenance":{"phase":3,"type":"autonomous"},"kind":"fleet_arrived"}"#;
    assert_eq!(
        decode_canonical_v2::<LivingEventPayloadV2>(tag_last, LIVING_MAX_ARCHIVE_BYTES_V2),
        Err(LivingWireErrorV2::NonCanonical),
        "a tagged enum puts its type field first"
    );
}

/// An event payload carrying an identity must not decode: that shape is the
/// non-conformant implementation the spec describes, and `deny_unknown_fields`
/// is what refuses it.
#[test]
fn a_payload_entry_with_an_identity_is_refused() {
    let smuggled = br#"{"id":"45cabd078b96ac0bf6a2a4f772131d82","ordinal":0,"provenance":{"type":"autonomous","phase":3},"kind":"fleet_arrived"}"#;
    assert_eq!(
        decode_canonical_v2::<LivingEventPayloadV2>(smuggled, LIVING_MAX_ARCHIVE_BYTES_V2),
        Err(LivingWireErrorV2::Json)
    );
}

#[test]
fn a_provenance_phase_outside_the_published_registry_is_refused() {
    for text in [
        br#"{"id":"45cabd078b96ac0bf6a2a4f772131d82","ordinal":0,"provenance":{"type":"autonomous","phase":0},"kind":"fleet_arrived"}"#.as_slice(),
        br#"{"ordinal":0,"provenance":{"type":"autonomous","phase":11},"kind":"fleet_arrived"}"#.as_slice(),
        br#"{"ordinal":0,"provenance":{"type":"creator","phase":3},"kind":"fleet_arrived"}"#.as_slice(),
    ] {
        assert!(
            decode_canonical_v2::<LivingEventPayloadV2>(text, LIVING_MAX_ARCHIVE_BYTES_V2).is_err()
        );
    }
}

/// The event record and its payload entry must stay in step: the payload is
/// the event minus its identity, which is exactly what the spec says.
#[test]
fn the_payload_entry_is_the_event_without_its_identity() {
    let (tick, applied_revisions, pending, state_digest) = reviewed_boundary();
    let entries = pending.payloads().to_vec();
    let receipt = seal_living_tick_receipt_v2(tick, applied_revisions, &pending, state_digest)
        .expect("the boundary seals");

    let rebuilt: Vec<LivingEventPayloadV2> = receipt
        .events
        .iter()
        .map(|event: &LivingEventV2| LivingEventPayloadV2 {
            ordinal: event.ordinal,
            provenance: event.provenance,
            kind: event.kind,
        })
        .collect();
    assert_eq!(rebuilt, entries);
    assert_eq!(receipt.payload().event_payloads, entries);
}

/// The public receipt's field order is declared normative in its doc comment,
/// and a doc comment pins nothing. Decoding a literal in the declared order and
/// then rejecting the same fields in any other order is what makes a reorder
/// fail. Round-tripping a value the encoder produced would not: both sides move
/// together.
#[test]
fn the_public_receipt_field_order_is_pinned_against_literal_bytes() {
    const DECLARED: &[u8] = br#"{"tick":50,"applied_revisions":["5273768014a3a8a6e38d1acdfec57c0e54e61378e7681c579baaf650e02b18ad","f578847e04303eaba9a9656fe43143dfab001a56e508dfc275684060f577d61d"],"events":[{"id":"45cabd078b96ac0bf6a2a4f772131d82","ordinal":0,"provenance":{"type":"creator","revision":"5273768014a3a8a6e38d1acdfec57c0e54e61378e7681c579baaf650e02b18ad"},"kind":"creator_intervention"},{"id":"b5b4c7b20f7e6ca788b27879686d5f19","ordinal":1,"provenance":{"type":"autonomous","phase":9},"kind":"shipment_dispatched"}],"state_digest":"23aea47d6aa816e31f3f5a65dfe7d6d9b6c8889cf6442826f030ab1800bfd683","digest":"e9cc871d91723c670abc871884a42d947992c7fd13096e6a47177cf82f244e71"}"#;

    let receipt: LivingTickReceiptV2 =
        decode_canonical_v2(DECLARED, LIVING_MAX_ARCHIVE_BYTES_V2).expect("the declared order");
    receipt
        .verify()
        .expect("the literal carries the reviewed digest and identities");

    // The literal is the reviewed boundary, so this also ties the field order to
    // the corpus rather than to whatever the encoder happens to emit.
    let (tick, applied_revisions, pending, state_digest) = reviewed_boundary();
    assert_eq!(
        receipt,
        seal_living_tick_receipt_v2(tick, applied_revisions, &pending, state_digest)
            .expect("the boundary seals")
    );

    // `digest` last, `state_digest` fourth: swap them and the document is no
    // longer canonical.
    const SWAPPED: &[u8] = br#"{"tick":50,"applied_revisions":[],"events":[],"digest":"e9cc871d91723c670abc871884a42d947992c7fd13096e6a47177cf82f244e71","state_digest":"23aea47d6aa816e31f3f5a65dfe7d6d9b6c8889cf6442826f030ab1800bfd683"}"#;
    assert_eq!(
        decode_canonical_v2::<LivingTickReceiptV2>(SWAPPED, LIVING_MAX_ARCHIVE_BYTES_V2),
        Err(LivingWireErrorV2::NonCanonical)
    );

    // And an event is `id, ordinal, provenance, kind`, in that order.
    const EVENT_REORDERED: &[u8] = br#"{"ordinal":0,"id":"45cabd078b96ac0bf6a2a4f772131d82","provenance":{"type":"autonomous","phase":9},"kind":"fleet_arrived"}"#;
    assert_eq!(
        decode_canonical_v2::<LivingEventV2>(EVENT_REORDERED, LIVING_MAX_ARCHIVE_BYTES_V2),
        Err(LivingWireErrorV2::NonCanonical)
    );
}

/// The bound is a refusal, not a wrapped ordinal. Filling the ordinal space and
/// then recording one more is the only way to see that.
#[test]
fn one_event_past_the_ordinal_space_is_refused() {
    let mut pending = LivingPendingEventsV2::new();
    for _ in 0..LIVING_MAX_BOUNDARY_EVENTS_V2 {
        pending
            .record(
                LivingEventProvenanceV2::Autonomous {
                    phase: LivingPhaseV2::CommitBoundary,
                },
                LivingEventKindV2::CombatRound,
            )
            .expect("an event inside the bound records");
    }
    assert_eq!(pending.len(), LIVING_MAX_BOUNDARY_EVENTS_V2);
    assert_eq!(
        pending.payloads()[LIVING_MAX_BOUNDARY_EVENTS_V2 - 1].ordinal,
        u16::MAX
    );
    assert_eq!(
        pending.record(
            LivingEventProvenanceV2::Autonomous {
                phase: LivingPhaseV2::CommitBoundary
            },
            LivingEventKindV2::CombatRound
        ),
        Err(LivingReceiptErrorV2::TooManyEvents {
            limit: LIVING_MAX_BOUNDARY_EVENTS_V2
        })
    );
}
