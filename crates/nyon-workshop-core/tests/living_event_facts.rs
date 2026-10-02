use nyon_workshop_core::living::*;
fn provenance() -> LivingEventProvenanceV2 {
    LivingEventProvenanceV2::Autonomous {
        phase: LivingPhaseV2::RunProduction,
    }
}
fn facts() -> LivingEventFactsV2 {
    LivingEventFactsV2 {
        actor: Some(LivingEntityIdV2([0x11; 16])),
        entity: Some(LivingEntityIdV2([0x22; 16])),
        blocked_reasons: vec![LivingBlockedReasonV2::MissingInput {
            resource: LivingResourceV2::Energy,
            required: 4,
            available: 0,
        }],
        ..Default::default()
    }
}
fn pair() -> (LivingTickReceiptV2, LivingEventFactsReceiptV2) {
    let mut p = LivingPendingEventsV2::new();
    p.record_explained(provenance(), LivingEventKindV2::FacilityBlocked, facts())
        .unwrap();
    p.record_explained(
        provenance(),
        LivingEventKindV2::ConstructionCompleted,
        LivingEventFactsV2 {
            actor: Some(LivingEntityIdV2([0x11; 16])),
            entity: Some(LivingEntityIdV2([0x33; 16])),
            resources: vec![LivingEventResourceFactV2 {
                role: LivingEventResourceRoleV2::Paid,
                resource: LivingResourceV2::Alloy,
                quantity: 60,
            }],
            affected: vec![LivingEntityIdV2([0x55; 16])],
            ..Default::default()
        },
    )
    .unwrap();
    let r = seal_living_tick_receipt_v2(
        LivingTickV2(50),
        vec![],
        &p,
        LivingStateDigestV2([0x44; 32]),
    )
    .unwrap();
    let f = seal_living_event_facts_receipt_v2(&r, &p).unwrap();
    (r, f)
}
#[test]
fn ordinary_event_facts_match_independent_full_bytes_and_domain_hash() {
    let (r, f) = pair();
    let bytes = include_bytes!("fixtures/living-v2-additions/event-facts.json");
    assert_eq!(f.canonical_bytes().unwrap(), bytes);
    assert_eq!(LivingEventFactsReceiptV2::decode(bytes, &r).unwrap(), f);
    assert_eq!(
        LIVING_EVENT_FACTS_DOMAIN_V2,
        b"NYON-LIVING-EVENT-FACTS-V2\0"
    );
}
#[test]
fn rejected_explained_events_preserve_ordinary_collector_and_ordinals() {
    let mut p = LivingPendingEventsV2::new();
    p.record(provenance(), LivingEventKindV2::HullCompleted)
        .unwrap();
    let before = p.clone();
    let mut bad = facts();
    bad.affected = vec![LivingEntityIdV2([1; 16]); 2];
    assert!(
        p.record_explained(provenance(), LivingEventKindV2::FacilityBlocked, bad)
            .is_err()
    );
    assert_eq!(p, before);
    assert!(
        p.record_explained(provenance(), LivingEventKindV2::FleetArrived, facts())
            .is_err()
    );
    assert_eq!(p, before);
    assert_eq!(
        p.record_explained(provenance(), LivingEventKindV2::FacilityBlocked, facts())
            .unwrap(),
        1
    );
    let r = seal_living_tick_receipt_v2(LivingTickV2(1), vec![], &p, LivingStateDigestV2([2; 32]))
        .unwrap();
    let f = seal_living_event_facts_receipt_v2(&r, &p).unwrap();
    assert_eq!(f.events.len(), 1);
    assert_eq!(f.events[0].ordinal, 1);
    assert_eq!(f.events[0].event_id, r.events[1].id);
}
#[test]
fn entire_event_facts_binding_rejects_digest_identity_order_and_wrong_boundary() {
    let (r, f) = pair();
    for variant in 0..6 {
        let mut bad = f.clone();
        match variant {
            0 => bad.tick.0 += 1,
            1 => bad.state_digest.0[0] ^= 1,
            2 => bad.events[0].event_id.0[0] ^= 1,
            3 => bad.events.swap(0, 1),
            4 => bad.events.push(bad.events[1].clone()),
            _ => bad.integrity_sha256.0[0] ^= 1,
        }
        assert!(bad.verify_receipt(&r).is_err());
    }
    let mut wrong = r.clone();
    wrong.digest.0[0] ^= 1;
    assert!(f.verify_receipt(&wrong).is_err());
}
#[test]
fn facts_resource_and_reason_bounds_reject_before_emission() {
    let mut p = LivingPendingEventsV2::new();
    for variant in 0..4 {
        let mut bad = facts();
        match variant {
            0 => {
                bad.resources = vec![LivingEventResourceFactV2 {
                    role: LivingEventResourceRoleV2::Input,
                    resource: LivingResourceV2::Energy,
                    quantity: 10_001,
                }]
            }
            1 => bad.blocked_reasons.push(bad.blocked_reasons[0]),
            2 => {
                bad.blocked_reasons = vec![LivingBlockedReasonV2::MissingInput {
                    resource: LivingResourceV2::Energy,
                    required: 4,
                    available: 4,
                }]
            }
            _ => bad.affected = (0..65).map(|n| LivingEntityIdV2([n; 16])).collect(),
        }
        assert!(
            p.record_explained(provenance(), LivingEventKindV2::FacilityBlocked, bad)
                .is_err()
        );
        assert!(p.is_empty());
    }
}
#[test]
fn explanation_capacity_omits_details_and_legacy_receipt_bytes_stay_exact() {
    let mut pending = LivingPendingEventsV2::new();
    let facts = LivingEventFactsV2 {
        affected: (0..64).map(|n| LivingEntityIdV2([n; 16])).collect(),
        ..Default::default()
    };
    for _ in 0..4096 {
        pending
            .record_explained(provenance(), LivingEventKindV2::FleetArrived, facts.clone())
            .unwrap();
    }
    let mut legacy = LivingPendingEventsV2::new();
    for event in pending.payloads() {
        legacy.record(event.provenance, event.kind).unwrap();
    }
    let ordinary = seal_living_tick_receipt_v2(
        LivingTickV2(u64::MAX),
        vec![],
        &pending,
        LivingStateDigestV2([1; 32]),
    )
    .unwrap();
    let explanations = seal_living_event_facts_receipt_v2(&ordinary, &pending).unwrap();
    assert!(explanations.omitted.is_some());
    assert!(explanations.events.len() < pending.len());
    assert!(explanations.canonical_bytes().unwrap().len() <= LIVING_MAX_EVENT_FACTS_BYTES_V2);
    let old = seal_living_tick_receipt_v2(
        LivingTickV2(u64::MAX),
        vec![],
        &legacy,
        LivingStateDigestV2([1; 32]),
    )
    .unwrap();
    assert_eq!(ordinary, old);
    let sidecar = seal_living_event_facts_receipt_v2(&ordinary, &pending).unwrap();
    assert!(sidecar.canonical_bytes().unwrap().len() <= LIVING_MAX_EVENT_FACTS_BYTES_V2);
}
#[test]
fn successful_recipe_new_tag_and_sparse_deposit_draw_match_independent_witness() {
    use nyon_workshop_core::living::event_facts::LivingDepositInputFactV2;
    let mut pending = LivingPendingEventsV2::new();
    let facts = LivingEventFactsV2 {
        actor: Some(LivingEntityIdV2([0x11; 16])),
        entity: Some(LivingEntityIdV2([0x22; 16])),
        resources: vec![
            LivingEventResourceFactV2 {
                role: LivingEventResourceRoleV2::Input,
                resource: LivingResourceV2::Energy,
                quantity: 2,
            },
            LivingEventResourceFactV2 {
                role: LivingEventResourceRoleV2::Output,
                resource: LivingResourceV2::Ore,
                quantity: 4,
            },
        ],
        deposit_input: Some(LivingDepositInputFactV2 {
            world: LivingEntityIdV2([0x55; 16]),
            units: 4,
        }),
        ..Default::default()
    };
    assert_eq!(
        LivingEventKindV2::FacilityProduced.as_wire_str(),
        "facility_produced"
    );
    pending
        .record_explained(
            provenance(),
            LivingEventKindV2::FacilityProduced,
            facts.clone(),
        )
        .unwrap();
    let ordinary = seal_living_tick_receipt_v2(
        LivingTickV2(10),
        vec![],
        &pending,
        LivingStateDigestV2([0x44; 32]),
    )
    .unwrap();
    let sidecar = seal_living_event_facts_receipt_v2(&ordinary, &pending).unwrap();
    let bytes = include_bytes!("fixtures/living-v2-additions/production-facts.json");
    assert_eq!(sidecar.canonical_bytes().unwrap(), bytes);
    assert_eq!(
        LivingEventFactsReceiptV2::decode(bytes, &ordinary).unwrap(),
        sidecar
    );
    let mut rejected = LivingPendingEventsV2::new();
    assert!(
        rejected
            .record_explained(
                provenance(),
                LivingEventKindV2::HullCompleted,
                facts.clone()
            )
            .is_err()
    );
    assert!(rejected.is_empty());
    for units in [0, 10_001] {
        let mut invalid = facts.clone();
        invalid.deposit_input.as_mut().unwrap().units = units;
        assert!(
            rejected
                .record_explained(provenance(), LivingEventKindV2::FacilityProduced, invalid)
                .is_err()
        );
        assert!(rejected.is_empty());
    }
    let explicit_null = std::str::from_utf8(bytes).unwrap().replace(
        "\"deposit_input\":{\"world\":\"55555555555555555555555555555555\",\"units\":4}",
        "\"deposit_input\":null",
    );
    assert_ne!(explicit_null.as_bytes(), bytes);
    assert!(LivingEventFactsReceiptV2::decode(explicit_null.as_bytes(), &ordinary).is_err());
}
