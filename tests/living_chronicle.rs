use nyon::living_chronicle::{
    ChronicleCategory, LIVING_CHRONICLE_LIMIT, LivingChronicle, LivingChronicleFilter,
};
use nyon_workshop_core::living::{
    LivingEventKindV2 as Kind, LivingEventProvenanceV2 as Provenance, LivingPendingEventsV2,
    LivingPhaseV2, LivingRevisionIdV2, LivingStateDigestV2, LivingTickReceiptV2, LivingTickV2,
    builtin_first_expansion_v2, seal_living_tick_receipt_v2,
};
fn receipt(tick: u64, kinds: &[Kind], creator: bool) -> LivingTickReceiptV2 {
    let mut pending = LivingPendingEventsV2::new();
    let provenance = if creator {
        Provenance::Creator {
            revision: LivingRevisionIdV2([1; 32]),
        }
    } else {
        Provenance::Autonomous {
            phase: LivingPhaseV2::RunProduction,
        }
    };
    for kind in kinds {
        pending.record(provenance, *kind).unwrap();
    }
    seal_living_tick_receipt_v2(
        LivingTickV2(tick),
        if creator {
            vec![LivingRevisionIdV2([1; 32])]
        } else {
            vec![]
        },
        &pending,
        LivingStateDigestV2([2; 32]),
    )
    .unwrap()
}
#[test]
fn repetitive_counts_use_exact_windows_and_bounded_first_last_references() {
    let mut chronicle = LivingChronicle::default();
    for tick in [1, 99, 100, 199, 200] {
        chronicle
            .record(
                &receipt(
                    tick,
                    &[Kind::FacilityBlocked, Kind::ShipmentDelivered],
                    false,
                ),
                None,
            )
            .unwrap();
    }
    let rows = chronicle.rows(&LivingChronicleFilter {
        category: Some(ChronicleCategory::Production),
        ..Default::default()
    });
    assert_eq!(rows.len(), 3);
    assert_eq!(
        (rows[0].tick_first, rows[0].tick_last, rows[0].count),
        (200, 200, 1)
    );
    assert_eq!(
        (rows[1].tick_first, rows[1].tick_last, rows[1].count),
        (100, 199, 2)
    );
    assert_eq!(
        (rows[2].tick_first, rows[2].tick_last, rows[2].count),
        (1, 99, 2)
    );
    assert_eq!(
        rows[2].first_receipt_digest,
        receipt(1, &[Kind::FacilityBlocked, Kind::ShipmentDelivered], false).digest
    );
    assert_eq!(
        rows[2].receipt_digest,
        receipt(99, &[Kind::FacilityBlocked, Kind::ShipmentDelivered], false).digest
    );
    assert_eq!(
        rows[2].event_id_first,
        Some(receipt(1, &[Kind::FacilityBlocked, Kind::ShipmentDelivered], false).events[0].id)
    );
    assert_eq!(
        rows[2].event_id_last,
        Some(receipt(99, &[Kind::FacilityBlocked, Kind::ShipmentDelivered], false).events[0].id)
    );
    assert!(rows.iter().all(|r| r.actor.is_none() && r.entity.is_none()));
}
#[test]
fn important_events_and_creator_overrides_remain_individual() {
    let mut chronicle = LivingChronicle::default();
    let kinds = [
        Kind::SettlementClaimResolved,
        Kind::AgreementFormed,
        Kind::WarDeclared,
        Kind::ColonyCaptured,
        Kind::CivilizationDormant,
    ];
    for tick in [1, 2] {
        chronicle
            .record(&receipt(tick, &kinds, false), None)
            .unwrap();
        chronicle
            .record(
                &receipt(
                    tick,
                    &[Kind::FacilityBlocked, Kind::ShipmentDelivered],
                    true,
                ),
                None,
            )
            .unwrap();
    }
    assert_eq!(chronicle.rows(&Default::default()).len(), 14);
    let creator = chronicle.rows(&LivingChronicleFilter {
        category: Some(ChronicleCategory::Creator),
        ..Default::default()
    });
    assert_eq!(creator.len(), 4);
    assert!(
        creator
            .iter()
            .all(|r| r.count == 1 && matches!(r.provenance, Some(Provenance::Creator { .. })))
    );
}
#[test]
fn latest_cache_is_capped_and_exact_duplicate_receipts_are_ignored() {
    let mut chronicle = LivingChronicle::default();
    for tick in 1..=600 {
        chronicle
            .record(&receipt(tick, &[Kind::SettlementClaimOpened], false), None)
            .unwrap();
    }
    let rows = chronicle.rows(&Default::default());
    assert_eq!(rows.len(), LIVING_CHRONICLE_LIMIT);
    assert_eq!(rows.last().unwrap().tick_first, 101);
    assert_eq!(
        chronicle
            .record(&receipt(600, &[Kind::SettlementClaimOpened], false), None)
            .unwrap(),
        None
    );
    assert_eq!(chronicle.rows(&Default::default()).len(), 500);
    assert!(
        chronicle
            .record(&receipt(100, &[Kind::SettlementClaimOpened], false), None)
            .is_err()
    );
    chronicle.clear();
    assert!(chronicle.rows(&Default::default()).is_empty());
    chronicle
        .record(&receipt(100, &[Kind::SettlementClaimOpened], false), None)
        .unwrap();
}
#[test]
fn invalid_pair_and_receipt_rejection_is_atomic_and_follow_links_are_honest() {
    let mut authority = builtin_first_expansion_v2().unwrap();
    let (receipt, decision) = (0..100)
        .find_map(|_| {
            let receipt = authority.step().unwrap();
            let decision = authority.last_decision().unwrap().clone();
            (!decision.intents.is_empty()).then_some((receipt, decision))
        })
        .expect("fixture must exercise actual linked decisions");
    let mut chronicle = LivingChronicle::default();
    chronicle.record(&receipt, Some(&decision)).unwrap();
    let before: Vec<_> = chronicle
        .rows(&Default::default())
        .into_iter()
        .cloned()
        .collect();
    let mut invalid = receipt.clone();
    invalid.state_digest = LivingStateDigestV2([9; 32]);
    assert!(chronicle.record(&invalid, Some(&decision)).is_err());
    let other = authority.step().unwrap();
    assert!(chronicle.record(&other, Some(&decision)).is_err());
    assert_eq!(
        chronicle
            .rows(&Default::default())
            .into_iter()
            .cloned()
            .collect::<Vec<_>>(),
        before
    );
    for intent in &decision.intents {
        let filter = LivingChronicleFilter {
            category: Some(ChronicleCategory::Decision),
            actor: Some(intent.actor),
            entity: intent.target,
        };
        let rows = chronicle.rows(&filter);
        assert!(
            rows.iter()
                .any(|r| r.score == Some(intent.score.0) && r.outcome == Some(intent.outcome))
        );
    }
    assert!(
        chronicle
            .rows(&Default::default())
            .iter()
            .filter(|r| r.category != ChronicleCategory::Decision)
            .all(|r| r.actor.is_none() && r.entity.is_none())
    );
}
#[test]
fn pause_matches_only_actual_completed_receipt_categories() {
    let mut chronicle = LivingChronicle::default();
    chronicle.set_pause_categories([ChronicleCategory::Settlement, ChronicleCategory::Creator]);
    assert_eq!(
        chronicle
            .record(&receipt(1, &[Kind::FacilityBlocked], false), None)
            .unwrap(),
        None
    );
    assert_eq!(
        chronicle
            .record(&receipt(2, &[Kind::SettlementClaimOpened], false), None)
            .unwrap(),
        Some(ChronicleCategory::Settlement)
    );
    assert_eq!(chronicle.rows(&Default::default())[0].tick_last, 2);
    assert_eq!(
        chronicle
            .record(&receipt(3, &[Kind::ShipmentWaiting], true), None)
            .unwrap(),
        Some(ChronicleCategory::Creator)
    );
}

#[test]
fn oversized_reason_summaries_keep_all_500_labels_within_one_mib() {
    use nyon::living_chronicle::LIVING_CHRONICLE_LABEL_LIMIT;
    use nyon_workshop_core::living::{
        LivingDecisionReasonV2, LivingDecisionScoreV2, LivingResourceV2,
        seal_living_decision_receipt_v2,
    };
    let mut authority = builtin_first_expansion_v2().unwrap();
    let decision = (0..100)
        .find_map(|_| {
            authority.step().unwrap();
            let decision = authority.last_decision().unwrap().clone();
            (!decision.intents.is_empty()).then_some(decision)
        })
        .unwrap();
    let mut intent = decision.intents[0].clone();
    intent.ordinal = 0;
    intent.reasons = vec![
        LivingDecisionReasonV2::Forecast {
            world: intent.actor,
            horizon: LivingTickV2(200),
            resource: LivingResourceV2::Energy,
            predicted: LivingDecisionScoreV2(i64::MIN),
            demand: LivingDecisionScoreV2(i64::MAX)
        };
        64
    ];
    let mut chronicle = LivingChronicle::default();
    for tick in decision.tick.0..decision.tick.0 + 500 {
        let receipt = receipt(tick, &[], false);
        intent.snapshot_tick = receipt.tick;
        let sidecar = seal_living_decision_receipt_v2(
            decision.catalog_hash,
            decision.branch,
            decision.committed_state_before,
            &receipt,
            vec![intent.clone()],
        )
        .unwrap();
        chronicle.record(&receipt, Some(&sidecar)).unwrap();
    }
    let rows = chronicle.rows(&Default::default());
    assert_eq!(rows.len(), 500);
    assert!(
        rows.iter()
            .all(|r| r.label.len() <= LIVING_CHRONICLE_LABEL_LIMIT
                && r.label.bytes().all(|b| (0x20..=0x7e).contains(&b))
                && r.label.contains("summary omitted")
                && r.decision_ordinal == Some(0)
                && r.decision_phase == Some(intent.phase)
                && r.first_receipt_digest == r.receipt_digest)
    );
    assert!(
        rows.iter().map(|r| r.label.len()).sum::<usize>() <= 500 * LIVING_CHRONICLE_LABEL_LIMIT
    );
}
#[test]
fn verified_ordinary_facts_enable_honest_follow_and_invalid_pair_is_atomic() {
    use nyon_workshop_core::living::{
        LivingBlockedReasonV2, LivingEntityIdV2, LivingEventFactsV2, LivingResourceV2,
        seal_living_event_facts_receipt_v2,
    };
    let actor = LivingEntityIdV2([3; 16]);
    let entity = LivingEntityIdV2([4; 16]);
    let mut p = LivingPendingEventsV2::new();
    p.record_explained(
        Provenance::Autonomous {
            phase: LivingPhaseV2::RunProduction,
        },
        Kind::FacilityBlocked,
        LivingEventFactsV2 {
            actor: Some(actor),
            entity: Some(entity),
            blocked_reasons: vec![LivingBlockedReasonV2::MissingInput {
                resource: LivingResourceV2::Energy,
                required: 4,
                available: 0,
            }],
            ..Default::default()
        },
    )
    .unwrap();
    let r = seal_living_tick_receipt_v2(LivingTickV2(10), vec![], &p, LivingStateDigestV2([2; 32]))
        .unwrap();
    let facts = seal_living_event_facts_receipt_v2(&r, &p).unwrap();
    let mut c = LivingChronicle::default();
    c.set_pause_categories([ChronicleCategory::Production]);
    assert_eq!(
        c.record_with_facts(&r, None, Some(&facts)).unwrap(),
        Some(ChronicleCategory::Production)
    );
    let rows = c.rows(&LivingChronicleFilter {
        actor: Some(actor),
        entity: Some(entity),
        ..Default::default()
    });
    assert_eq!(rows.len(), 1);
    assert!(rows[0].label.contains("required: 4"));
    assert_eq!(rows[0].facts.as_ref().unwrap().blocked_reasons.len(), 1);
    let snapshot = c
        .rows(&LivingChronicleFilter::default())
        .into_iter()
        .cloned()
        .collect::<Vec<_>>();
    let mut bad = facts.clone();
    bad.events[0].facts.actor = Some(LivingEntityIdV2([9; 16]));
    assert!(c.record_with_facts(&r, None, Some(&bad)).is_err());
    assert_eq!(
        c.rows(&LivingChronicleFilter::default())
            .into_iter()
            .cloned()
            .collect::<Vec<_>>(),
        snapshot
    );
    let mut unknown = LivingChronicle::default();
    unknown.record(&r, None).unwrap();
    assert!(
        unknown
            .rows(&LivingChronicleFilter {
                actor: Some(actor),
                ..Default::default()
            })
            .is_empty()
    );
    assert!(
        unknown.rows(&LivingChronicleFilter::default())[0]
            .facts
            .is_none()
    );
}

#[test]
fn receipt_bound_trade_details_distinguish_unknown_dispatch_and_capacity() {
    use nyon_workshop_core::living::{
        LivingDeliveryFactV2, LivingEntityIdV2, LivingEventFactsV2, LivingResourceV2,
        seal_living_event_facts_receipt_v2,
    };
    let mut pending = LivingPendingEventsV2::new();
    let detail = LivingEventFactsV2 {
        delivery: Some(LivingDeliveryFactV2 {
            source_world: LivingEntityIdV2([3; 16]),
            destination_world: LivingEntityIdV2([4; 16]),
            dispatch_owner: LivingEntityIdV2([1; 16]),
            intended_receiver: LivingEntityIdV2([2; 16]),
            recipient: LivingEntityIdV2([2; 16]),
            resource: LivingResourceV2::Ore,
            units: 10,
            dispatch_kind: None,
            source_system: None,
            destination_system: None,
        }),
        ..Default::default()
    };
    for _ in 0..4096 {
        pending
            .record_explained(
                Provenance::Autonomous {
                    phase: LivingPhaseV2::ResolveArrivals,
                },
                Kind::ShipmentDelivered,
                detail.clone(),
            )
            .unwrap();
    }
    let receipt = seal_living_tick_receipt_v2(
        LivingTickV2(50),
        vec![],
        &pending,
        LivingStateDigestV2([2; 32]),
    )
    .unwrap();
    let facts = seal_living_event_facts_receipt_v2(&receipt, &pending).unwrap();
    assert!(facts.omitted.is_some());
    let mut chronicle = LivingChronicle::default();
    chronicle
        .record_with_facts(&receipt, None, Some(&facts))
        .unwrap();
    let rows = chronicle.rows(&LivingChronicleFilter::default());
    assert_eq!(
        rows.len(),
        2,
        "unknown and capacity details cannot aggregate together"
    );
    assert_eq!(rows.iter().map(|row| row.count).sum::<u64>(), 4096);
    let omitted = rows
        .iter()
        .find(|row| row.details_omitted_for_capacity)
        .unwrap();
    assert!(
        omitted
            .label
            .contains("details unavailable: explanation capacity reached")
    );
    assert!(omitted.facts.is_none());
    let detailed = rows.iter().find(|row| row.facts.is_some()).unwrap();
    assert!(!detailed.details_omitted_for_capacity);
    assert!(detailed.label.contains("delivered 10 ore from world"));
    assert!(detailed.label.contains("dispatch unknown"));
    assert!(!detailed.label.contains("capacity reached"));
}

#[test]
fn reopened_session_projects_only_the_latest_replay_verified_boundary() {
    use nyon::{living_content::LivingContent, sandbox_session::SandboxSession};
    use nyon_workshop_core::living::{decode_living_archive_v2, encode_living_archive_v2};
    let mut authority = LivingContent::Commerce.open().unwrap();
    for _ in 0..100 {
        authority.step().unwrap();
    }
    let before = authority.bookmark().unwrap();
    let latest = authority.last_receipt().unwrap().clone();
    let decision = authority.last_decision().unwrap().clone();
    let restored = decode_living_archive_v2(
        &encode_living_archive_v2(&authority).unwrap(),
        authority.catalog(),
    )
    .unwrap()
    .into_authority();
    let session = SandboxSession::from_authority(restored);
    assert_eq!(session.bookmark().unwrap(), before);
    assert_eq!(session.tick(), 100);
    assert_eq!(session.receipts().len(), 1);
    assert_eq!(session.receipts().back(), Some(&latest));
    assert_eq!(session.decision_receipts().len(), 1);
    assert_eq!(session.decision_receipts().back().unwrap().1, decision);
    assert!(!session.chronicle_rows().is_empty());
    assert!(session.chronicle_rows().iter().all(|row| {
        row.tick_first == 100 && row.tick_last == 100 && row.receipt_digest == latest.digest
    }));
    assert!(session.paused());
    assert!(SandboxSession::open().unwrap().receipts().is_empty());
}

#[test]
fn restoring_an_exact_bookmark_replaces_chronicle_with_its_authenticated_boundary() {
    use nyon::{
        living_content::LivingContent,
        sandbox_session::{SandboxSession, SandboxUiAction as A},
    };
    let mut session = SandboxSession::from_authority(LivingContent::Commerce.open().unwrap());
    for _ in 0..100 {
        session.step_once().unwrap();
    }
    let bookmark = session.bookmark().unwrap();
    let receipt = session.receipts().back().unwrap().clone();
    session.apply_action(A::RecordBookmark).unwrap();
    for _ in 0..10 {
        session.step_once().unwrap();
    }
    session.apply_action(A::RestoreBookmark(0)).unwrap();
    for _ in 0..1000 {
        session.update(std::time::Duration::ZERO).unwrap();
        if session.tick() == 100 {
            break;
        }
    }
    assert_eq!(session.bookmark().unwrap(), bookmark);
    assert_eq!(session.receipts().len(), 1);
    assert_eq!(session.receipts().back(), Some(&receipt));
    assert!(!session.chronicle_rows().is_empty());
    assert!(session.chronicle_rows().iter().all(|row| {
        row.tick_first == 100 && row.tick_last == 100 && row.receipt_digest == receipt.digest
    }));
}
