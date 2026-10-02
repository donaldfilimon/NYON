//! Exact additive bytes are independently calculated; codecs do not certify gameplay.
use nyon_workshop_core::living::event_facts::LivingDeliveryFactV2;
use nyon_workshop_core::living::*;

fn delivery() -> LivingDeliveryFactV2 {
    LivingDeliveryFactV2 {
        source_world: LivingEntityIdV2([0x33; 16]),
        destination_world: LivingEntityIdV2([0x44; 16]),
        dispatch_owner: LivingEntityIdV2([0x11; 16]),
        intended_receiver: LivingEntityIdV2([0x22; 16]),
        recipient: LivingEntityIdV2([0x22; 16]),
        resource: LivingResourceV2::Ore,
        units: 10,
        dispatch_kind: Some(LivingRouteKindV2::Trade),
        source_system: Some(LivingEntityIdV2([0x55; 16])),
        destination_system: Some(LivingEntityIdV2([0x66; 16])),
    }
}
fn agreement() -> LivingAgreementV2 {
    LivingAgreementV2 {
        kind: LivingAgreementKindV2::Trade,
        participant_a: LivingEntityIdV2([0x11; 16]),
        participant_b: LivingEntityIdV2([0x22; 16]),
        start_tick: LivingTickV2(50),
        end_tick: LivingTickV2(1250),
    }
}
fn pair(unknown: bool) -> (LivingTickReceiptV2, LivingEventFactsReceiptV2) {
    let mut delivered = delivery();
    if unknown {
        delivered.dispatch_kind = None;
        delivered.source_system = None;
        delivered.destination_system = None;
    }
    let mut pending = LivingPendingEventsV2::new();
    pending
        .record_explained(
            LivingEventProvenanceV2::Autonomous {
                phase: LivingPhaseV2::ResolveArrivals,
            },
            LivingEventKindV2::ShipmentDelivered,
            LivingEventFactsV2 {
                delivery: Some(delivered),
                ..Default::default()
            },
        )
        .unwrap();
    pending
        .record_explained(
            LivingEventProvenanceV2::Autonomous {
                phase: LivingPhaseV2::UpdateDiplomacy,
            },
            LivingEventKindV2::AgreementFormed,
            LivingEventFactsV2 {
                agreement: Some(agreement()),
                ..Default::default()
            },
        )
        .unwrap();
    let receipt = seal_living_tick_receipt_v2(
        LivingTickV2(50),
        vec![],
        &pending,
        LivingStateDigestV2([0x44; 32]),
    )
    .unwrap();
    let facts = seal_living_event_facts_receipt_v2(&receipt, &pending).unwrap();
    (receipt, facts)
}

#[test]
fn populated_and_unknown_trade_fact_bytes_match_independent_formula() {
    for (unknown, expected) in [
        (
            false,
            include_bytes!("fixtures/living-v2-additions/trade-facts.json").as_slice(),
        ),
        (
            true,
            include_bytes!("fixtures/living-v2-additions/trade-facts-unknown.json").as_slice(),
        ),
    ] {
        let (receipt, facts) = pair(unknown);
        assert_eq!(facts.canonical_bytes().unwrap(), expected);
        assert_eq!(
            LivingEventFactsReceiptV2::decode(expected, &receipt).unwrap(),
            facts
        );
        assert_eq!(
            facts.events[0]
                .facts
                .delivery
                .as_ref()
                .unwrap()
                .dispatch_kind
                .is_none(),
            unknown
        );
    }
}

#[test]
fn sparse_absence_keeps_old_bytes_and_explicit_null_is_not_canonical() {
    let old: LivingEventFactsReceiptV2 = decode_canonical_v2(
        include_bytes!("fixtures/living-v2-additions/event-facts.json"),
        LIVING_MAX_EVENT_FACTS_BYTES_V2,
    )
    .unwrap();
    assert!(
        old.events
            .iter()
            .all(|event| event.facts.delivery.is_none() && event.facts.agreement.is_none())
    );
    assert_eq!(
        old.canonical_bytes().unwrap(),
        include_bytes!("fixtures/living-v2-additions/event-facts.json")
    );
    for field in ["agreement", "delivery"] {
        let mut bytes = old.canonical_bytes().unwrap();
        let needle = b"\"affected\":[]}";
        let position = bytes
            .windows(needle.len())
            .position(|window| window == needle)
            .unwrap()
            + needle.len()
            - 1;
        bytes.splice(position..position, format!(",\"{field}\":null").bytes());
        assert!(
            decode_canonical_v2::<LivingEventFactsReceiptV2>(
                &bytes,
                LIVING_MAX_EVENT_FACTS_BYTES_V2
            )
            .is_err()
        );
    }
}

#[test]
fn wrong_kind_invalid_parties_interval_quantity_and_recipient_preserve_collector() {
    for defect in 0..7 {
        let mut pending = LivingPendingEventsV2::new();
        let before = pending.clone();
        let mut facts = LivingEventFactsV2::default();
        let mut kind = LivingEventKindV2::ShipmentDelivered;
        if defect < 4 {
            let mut value = delivery();
            match defect {
                0 => value.units = 0,
                1 => value.units = 10_001,
                2 => value.recipient = LivingEntityIdV2([0x23; 16]),
                _ => kind = LivingEventKindV2::ShipmentWaiting,
            }
            facts.delivery = Some(value);
        } else {
            let mut value = agreement();
            kind = LivingEventKindV2::AgreementFormed;
            match defect {
                4 => value.participant_a = value.participant_b,
                5 => value.end_tick = value.start_tick,
                _ => kind = LivingEventKindV2::CreatorIntervention,
            }
            facts.agreement = Some(value);
        }
        assert!(
            pending
                .record_explained(
                    LivingEventProvenanceV2::Autonomous {
                        phase: LivingPhaseV2::ResolveArrivals
                    },
                    kind,
                    facts
                )
                .is_err()
        );
        assert_eq!(pending, before);
    }
}

#[test]
fn agreement_formation_tick_and_modified_pair_integrity_are_checked() {
    let (receipt, original) = pair(false);
    for defect in 0..7 {
        let mut bad = original.clone();
        match defect {
            0 => bad.events[0].facts.delivery.as_mut().unwrap().units += 1,
            1 => bad.events[1].facts.agreement.as_mut().unwrap().start_tick.0 -= 1,
            2 => bad.events[0].event_id.0[0] ^= 1,
            3 => bad.events[0].ordinal = 1,
            4 => bad.receipt_digest.0[0] ^= 1,
            5 => bad.integrity_sha256.0[0] ^= 1,
            _ => {
                bad.events[1].facts.agreement.as_mut().unwrap().kind = LivingAgreementKindV2::Truce
            }
        }
        assert!(bad.verify_receipt(&receipt).is_err());
    }
}

#[test]
fn existing_same_world_manifest_and_explicit_unknown_classification_remain_representable() {
    let mut value = delivery();
    value.destination_world = value.source_world;
    value.dispatch_kind = None;
    value.source_system = None;
    value.destination_system = None;
    assert!(
        LivingEventFactsV2 {
            delivery: Some(value),
            ..Default::default()
        }
        .validate(LivingEventKindV2::ShipmentDelivered)
        .is_ok()
    );
}

#[test]
fn omission_bytes_and_strict_ranges_match_independent_formula() {
    use nyon_workshop_core::living::event_facts::{
        LivingEventFactsOmissionReasonV2, LivingEventFactsOmittedV2,
    };
    let (receipt, mut facts) = pair(false);
    facts.events.truncate(1);
    facts.omitted = Some(LivingEventFactsOmittedV2 {
        reason: LivingEventFactsOmissionReasonV2::Capacity,
        ranges: vec![[1, 1]],
    });
    let expected = include_bytes!("fixtures/living-v2-additions/omitted-facts.json");
    let independent: LivingEventFactsReceiptV2 =
        LivingEventFactsReceiptV2::decode(expected, &receipt).unwrap();
    facts.integrity_sha256 = independent.integrity_sha256;
    assert_eq!(facts.canonical_bytes().unwrap(), expected);
    for ranges in [
        vec![],
        vec![[1, 0]],
        vec![[0, 0]],
        vec![[2, 2]],
        vec![[1, 1], [1, 1]],
    ] {
        let mut invalid = facts.clone();
        invalid.omitted.as_mut().unwrap().ranges = ranges;
        assert!(invalid.verify_receipt(&receipt).is_err());
    }
    let mut wire: serde_json::Value = serde_json::from_slice(expected).unwrap();
    wire["omitted"]["reason"] = serde_json::json!("unknown");
    assert!(
        LivingEventFactsReceiptV2::decode(&serde_json::to_vec(&wire).unwrap(), &receipt).is_err()
    );
}

#[test]
fn all_4096_delivery_events_survive_exact_envelope_admission() {
    let provenance = LivingEventProvenanceV2::Autonomous {
        phase: LivingPhaseV2::ResolveArrivals,
    };
    let mut detailed = LivingPendingEventsV2::new();
    let mut bare = LivingPendingEventsV2::new();
    for _ in 0..4096 {
        detailed
            .record_explained(
                provenance,
                LivingEventKindV2::ShipmentDelivered,
                LivingEventFactsV2 {
                    delivery: Some(delivery()),
                    ..Default::default()
                },
            )
            .unwrap();
        bare.record(provenance, LivingEventKindV2::ShipmentDelivered)
            .unwrap();
    }
    let receipt = seal_living_tick_receipt_v2(
        LivingTickV2(u64::MAX),
        vec![],
        &detailed,
        LivingStateDigestV2([0x44; 32]),
    )
    .unwrap();
    assert_eq!(
        receipt,
        seal_living_tick_receipt_v2(
            LivingTickV2(u64::MAX),
            vec![],
            &bare,
            LivingStateDigestV2([0x44; 32])
        )
        .unwrap()
    );
    let facts = seal_living_event_facts_receipt_v2(&receipt, &detailed).unwrap();
    assert_eq!(receipt.events.len(), 4096);
    assert!(!facts.events.is_empty());
    assert!(facts.events.len() < 4096);
    assert_eq!(
        facts.omitted.as_ref().unwrap().ranges,
        vec![[u16::try_from(facts.events.len()).unwrap(), 4095]]
    );
    let bytes = facts.canonical_bytes().unwrap();
    assert!(bytes.len() <= LIVING_MAX_EVENT_FACTS_BYTES_V2);
    assert!(LIVING_MAX_EVENT_FACTS_BYTES_V2 - bytes.len() < 600);
    assert_eq!(
        LivingEventFactsReceiptV2::decode(&bytes, &receipt).unwrap(),
        facts
    );
    assert_eq!(
        seal_living_event_facts_receipt_v2(&receipt, &detailed).unwrap(),
        facts
    );
}

#[test]
fn worst_alternating_candidate_ordinals_have_bounded_coalesced_omissions() {
    let provenance = LivingEventProvenanceV2::Autonomous {
        phase: LivingPhaseV2::ResolveArrivals,
    };
    let mut pending = LivingPendingEventsV2::new();
    for ordinal in 0..=u16::MAX {
        if ordinal % 2 == 0 {
            pending
                .record_explained(
                    provenance,
                    LivingEventKindV2::ShipmentDelivered,
                    LivingEventFactsV2 {
                        delivery: Some(delivery()),
                        ..Default::default()
                    },
                )
                .unwrap();
        } else {
            pending
                .record(provenance, LivingEventKindV2::ShipmentWaiting)
                .unwrap();
        }
    }
    let receipt = seal_living_tick_receipt_v2(
        LivingTickV2(1),
        vec![],
        &pending,
        LivingStateDigestV2([0x44; 32]),
    )
    .unwrap();
    let facts = seal_living_event_facts_receipt_v2(&receipt, &pending).unwrap();
    let omitted = facts.omitted.as_ref().unwrap();
    assert!(
        omitted
            .ranges
            .iter()
            .all(|[start, end]| start == end && start % 2 == 0)
    );
    assert_eq!(facts.events.len() + omitted.ranges.len(), 32768);
    assert!(facts.canonical_bytes().unwrap().len() <= LIVING_MAX_EVENT_FACTS_BYTES_V2);
    facts.verify_receipt(&receipt).unwrap();
}

#[test]
fn direct_constructed_sidecar_checks_full_envelope_even_when_integrity_payload_fits() {
    use sha2::{Digest, Sha256};
    let provenance = LivingEventProvenanceV2::Autonomous {
        phase: LivingPhaseV2::ResolveArrivals,
    };
    let mut pending = LivingPendingEventsV2::new();
    for _ in 0..4096 {
        pending
            .record_explained(
                provenance,
                LivingEventKindV2::ShipmentDelivered,
                LivingEventFactsV2 {
                    delivery: Some(delivery()),
                    ..Default::default()
                },
            )
            .unwrap();
    }
    let receipt = seal_living_tick_receipt_v2(
        LivingTickV2(u64::MAX),
        vec![],
        &pending,
        LivingStateDigestV2([0x44; 32]),
    )
    .unwrap();
    let mut facts = seal_living_event_facts_receipt_v2(&receipt, &pending).unwrap();
    assert!(facts.omitted.is_some());
    for index in 0..64 {
        facts
            .events
            .last_mut()
            .unwrap()
            .facts
            .affected
            .push(LivingEntityIdV2([index; 16]));
        let bytes = serde_json::to_vec(&facts).unwrap();
        if bytes.len() > LIVING_MAX_EVENT_FACTS_BYTES_V2 {
            let position = bytes
                .windows(b",\"integrity_sha256\"".len())
                .position(|window| window == b",\"integrity_sha256\"")
                .unwrap();
            let mut payload = bytes[..position].to_vec();
            payload.push(b'}');
            assert!(payload.len() <= LIVING_MAX_EVENT_FACTS_BYTES_V2);
            let mut hash = Sha256::new();
            hash.update(LIVING_EVENT_FACTS_DOMAIN_V2);
            hash.update(payload);
            facts.integrity_sha256 = LivingReceiptDigestV2(hash.finalize().into());
            assert!(matches!(
                facts.verify_receipt(&receipt),
                Err(LivingEventFactsErrorV2::Wire(_))
            ));
            return;
        }
    }
    panic!("fixture did not reach complete-envelope boundary");
}
