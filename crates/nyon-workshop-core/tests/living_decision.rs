//! Separate canonical explanation wire, pinned by independent Python formulas.
use nyon_workshop_core::living::*;
const GOLDEN: &[u8] = include_bytes!("fixtures/living-v2-additions/decision.json");
fn sample() -> LivingDecisionReceiptV2 {
    decode_living_decision_receipt_v2(GOLDEN).unwrap()
}
#[test]
fn independently_derived_full_bytes_are_exact_and_bind_ordinary_receipt() {
    let record = sample();
    assert_eq!(record.canonical_bytes().unwrap(), GOLDEN);
    assert_eq!(record.intents[0].score, LivingDecisionScoreV2(-10));
    let ordinary = LivingTickReceiptV2 {
        tick: record.tick,
        applied_revisions: vec![],
        events: vec![],
        state_digest: record.state_after,
        digest: record.receipt_digest,
    };
    record.verify_receipt(&ordinary).unwrap();
    let sealed = seal_living_decision_receipt_v2(
        record.catalog_hash,
        record.branch,
        record.committed_state_before,
        &ordinary,
        record.intents.clone(),
    )
    .unwrap();
    assert_eq!(sealed.canonical_bytes().unwrap(), GOLDEN);
    let mut wrong = ordinary;
    wrong.tick = LivingTickV2(51);
    assert_eq!(
        record.verify_receipt(&wrong),
        Err(LivingDecisionErrorV2::ReceiptBinding)
    );
}
#[test]
fn strict_wire_rejects_unknown_reordered_duplicate_and_noncanonical_scores() {
    let text = std::str::from_utf8(GOLDEN).unwrap();
    for changed in [
        text.replace("\"score\":\"-10\"", "\"score\":\"-010\""),
        text.replace("\"score\":\"-10\"", "\"score\":-10"),
        text.replace("\"ordinal\":0", "\"ordinal\":0,\"ordinal\":0"),
        text.replace("\"ordinal\":0", "\"unknown\":0,\"ordinal\":0"),
        text.replace(
            "\"format_version\":2,\"rules_version\":2",
            "\"rules_version\":2,\"format_version\":2",
        ),
        format!(" {text}"),
    ] {
        assert!(
            decode_living_decision_receipt_v2(changed.as_bytes()).is_err(),
            "{changed}"
        );
    }
    assert!(
        decode_living_decision_receipt_v2(&vec![b' '; LIVING_MAX_DECISION_BYTES_V2 + 1]).is_err()
    );
}
#[test]
fn sealing_refuses_invalid_clock_phase_dense_ordinals_and_reason_ranges() {
    let original = sample();
    let ordinary = LivingTickReceiptV2 {
        tick: original.tick,
        applied_revisions: vec![],
        events: vec![],
        state_digest: original.state_after,
        digest: original.receipt_digest,
    };
    let seal = |intent| {
        seal_living_decision_receipt_v2(
            original.catalog_hash,
            original.branch,
            original.committed_state_before,
            &ordinary,
            vec![intent],
        )
    };
    let mut i = original.intents[0].clone();
    i.ordinal = 1;
    assert_eq!(seal(i), Err(LivingDecisionErrorV2::Candidate));
    let mut i = original.intents[0].clone();
    i.phase = LivingPhaseV2::RunProduction;
    assert_eq!(seal(i), Err(LivingDecisionErrorV2::Candidate));
    let mut i = original.intents[0].clone();
    i.snapshot_tick = LivingTickV2(49);
    assert_eq!(seal(i), Err(LivingDecisionErrorV2::Candidate));
    let mut i = original.intents[0].clone();
    i.observed_tick = Some(LivingTickV2(51));
    assert_eq!(seal(i), Err(LivingDecisionErrorV2::Candidate));
    let mut i = original.intents[0].clone();
    i.reasons.clear();
    assert_eq!(seal(i), Err(LivingDecisionErrorV2::Candidate));
    let mut i = original.intents[0].clone();
    i.reasons = vec![LivingDecisionReasonV2::InsufficientResource {
        world: LivingEntityIdV2([1; 16]),
        resource: LivingResourceV2::Energy,
        required: 10001,
        available: 0,
        reserve: 0,
    }];
    assert_eq!(seal(i), Err(LivingDecisionErrorV2::Candidate));
    let mut corrupt = original;
    corrupt.intents[0].score = LivingDecisionScoreV2(1);
    assert_eq!(corrupt.verify(), Err(LivingDecisionErrorV2::Integrity));
}
#[test]
fn score_extremes_are_canonical_signed_strings() {
    for value in [i64::MIN, -1, 0, 1, i64::MAX] {
        let score = LivingDecisionScoreV2(value);
        let encoded = serde_json::to_vec(&score).unwrap();
        assert_eq!(encoded, format!("\"{value}\"").as_bytes());
        assert_eq!(
            serde_json::from_slice::<LivingDecisionScoreV2>(&encoded).unwrap(),
            score
        );
    }
    for value in [
        "\"-0\"",
        "\"+1\"",
        "\"01\"",
        "\"9223372036854775808\"",
        "1",
        "null",
    ] {
        assert!(serde_json::from_str::<LivingDecisionScoreV2>(value).is_err());
    }
}
#[test]
fn phase_snapshot_normalization_is_private_and_exact() {
    let snapshot = LivingGalaxyStateV2::default();
    let before = snapshot.digest().unwrap();
    let digest = living_decision_snapshot_digest_v2(&snapshot, LivingTickV2(1)).unwrap();
    assert_eq!(snapshot.tick, LivingTickV2(0));
    assert_eq!(snapshot.digest().unwrap(), before);
    let mut normalized = snapshot.clone();
    normalized.tick = LivingTickV2(1);
    assert_eq!(digest, normalized.digest().unwrap());
    assert_eq!(
        living_decision_snapshot_digest_v2(&snapshot, LivingTickV2(0)),
        Err(LivingDecisionErrorV2::Snapshot)
    );
    assert_eq!(
        living_decision_snapshot_digest_v2(&snapshot, LivingTickV2(2)),
        Err(LivingDecisionErrorV2::Snapshot)
    );
}
