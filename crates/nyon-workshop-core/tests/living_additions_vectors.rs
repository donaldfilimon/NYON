//! Independently specified framing for additive identity domains.
use nyon_workshop_core::living::ids::{
    LIVING_GENESIS_FORK_DOMAIN_V2, genesis_fork_branch_digest_v2, genesis_fork_branch_id_v2,
};
use nyon_workshop_core::living::{
    LivingBranchIdV2, LivingStateDigestV2, LivingTickV2, decode_canonical_v2, encode_canonical_v2,
};
use serde::{Deserialize, Serialize};
#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct Row {
    parent: LivingBranchIdV2,
    genesis_digest: LivingStateDigestV2,
    tick: LivingTickV2,
    ordinal: u64,
    digest: LivingStateDigestV2,
    branch_id: LivingBranchIdV2,
}
#[test]
fn genesis_only_fork_domain_full_digest_and_truncation_match_independent_formula() {
    const BYTES: &[u8] = include_bytes!("fixtures/living-v2-additions/genesis-forks.json");
    assert_eq!(
        LIVING_GENESIS_FORK_DOMAIN_V2,
        b"NYON-LIVING-GENESIS-FORK-V2\0"
    );
    let rows: Vec<Row> = decode_canonical_v2(BYTES, 65536).unwrap();
    assert_eq!(rows.len(), 4);
    assert_eq!(encode_canonical_v2(&rows, 65536).unwrap(), BYTES);
    for row in rows {
        assert_eq!(
            genesis_fork_branch_digest_v2(row.parent, row.genesis_digest, row.tick, row.ordinal),
            row.digest.0
        );
        assert_eq!(
            genesis_fork_branch_id_v2(row.parent, row.genesis_digest, row.tick, row.ordinal),
            row.branch_id
        );
        assert_eq!(&row.digest.0[..16], &row.branch_id.0);
    }
}
#[test]
fn every_genesis_fork_input_changes_full_digest() {
    let parent = LivingBranchIdV2([0x11; 16]);
    let genesis = LivingStateDigestV2([0x22; 32]);
    let tick = LivingTickV2(50);
    let ordinal = 2;
    let base = genesis_fork_branch_digest_v2(parent, genesis, tick, ordinal);
    for value in [
        genesis_fork_branch_digest_v2(LivingBranchIdV2([0x12; 16]), genesis, tick, ordinal),
        genesis_fork_branch_digest_v2(parent, LivingStateDigestV2([0x23; 32]), tick, ordinal),
        genesis_fork_branch_digest_v2(parent, genesis, LivingTickV2(51), ordinal),
        genesis_fork_branch_digest_v2(parent, genesis, tick, 3),
    ] {
        assert_ne!(value, base);
    }
}

#[test]
fn sparse_operational_defense_observation_matches_independent_bytes_and_hashes() {
    use nyon_workshop_core::living::LivingObservationV2;
    use sha2::{Digest, Sha256};
    #[derive(Deserialize, Serialize)]
    #[serde(deny_unknown_fields)]
    struct ObservationRow {
        observation: LivingObservationV2,
        sha256: LivingStateDigestV2,
    }
    const BYTES: &[u8] = include_bytes!("fixtures/living-v2-additions/observations.json");
    let rows: Vec<ObservationRow> = decode_canonical_v2(BYTES, 65536).unwrap();
    assert_eq!(encode_canonical_v2(&rows, 65536).unwrap(), BYTES);
    assert_eq!(rows.len(), 3);
    assert_eq!(rows[0].observation.operational_defense_batteries, None);
    assert_eq!(rows[1].observation.operational_defense_batteries, Some(0));
    assert_eq!(rows[2].observation.operational_defense_batteries, Some(1));
    for row in rows {
        let bytes = encode_canonical_v2(&row.observation, 65536).unwrap();
        assert_eq!(<[u8; 32]>::from(Sha256::digest(&bytes)), row.sha256.0);
    }
}

#[test]
fn construction_scrap_action_has_independent_additive_integrity_and_preserves_none_bytes() {
    use nyon_workshop_core::living::{
        LivingEntityIdV2, LivingIntentActionV2, decode_living_decision_receipt_v2,
    };
    const LEGACY: &[u8] = include_bytes!("fixtures/living-v2-additions/decision.json");
    const ADDITIVE: &[u8] = include_bytes!("fixtures/living-v2-additions/decision-scrap.json");
    let legacy = decode_living_decision_receipt_v2(LEGACY).unwrap();
    assert_eq!(legacy.canonical_bytes().unwrap(), LEGACY);
    assert!(matches!(
        legacy.intents[0].action,
        LivingIntentActionV2::Construction { scrap: None, .. }
    ));
    let additive = decode_living_decision_receipt_v2(ADDITIVE).unwrap();
    assert_eq!(additive.canonical_bytes().unwrap(), ADDITIVE);
    assert!(
        matches!(additive.intents[0].action, LivingIntentActionV2::Construction { scrap: Some(id), .. } if id == LivingEntityIdV2([0x99;16]))
    );
    let mut explicit_null: serde_json::Value = serde_json::from_slice(LEGACY).unwrap();
    explicit_null["intents"][0]["action"]["scrap"] = serde_json::Value::Null;
    assert!(
        decode_living_decision_receipt_v2(&serde_json::to_vec(&explicit_null).unwrap()).is_err()
    );
}

#[test]
fn notebook_ordered_wire_and_separate_integrity_domain_match_independent_formula() {
    use nyon_workshop_core::living::{
        LIVING_NOTEBOOK_DOMAIN_V2, LivingNotebookV2, LivingNotebookValueV2,
    };
    use sha2::{Digest, Sha256};
    const BYTES: &[u8] = include_bytes!("fixtures/living-v2-additions/notebook.json");
    let notebook: LivingNotebookV2 = decode_canonical_v2(BYTES, 1048576).unwrap();
    assert_eq!(notebook.canonical_bytes().unwrap(), BYTES);
    assert_eq!(LIVING_NOTEBOOK_DOMAIN_V2, b"NYON-LIVING-NOTEBOOK-V2\0");
    let field = b",\"integrity_sha256\":";
    let boundary = BYTES
        .windows(field.len())
        .position(|bytes| bytes == field)
        .unwrap();
    let mut payload = BYTES[..boundary].to_vec();
    payload.push(b'}');
    let mut hash = Sha256::new();
    hash.update(LIVING_NOTEBOOK_DOMAIN_V2);
    hash.update(payload);
    assert_eq!(
        <[u8; 32]>::from(hash.finalize()),
        notebook.integrity_sha256.0
    );
    assert_eq!(notebook.bookmarks.len(), 2);
    let observations = &notebook.experiments[0].observations;
    assert!(matches!(observations[0].left, LivingNotebookValueV2::Integer(value) if value.0 == 20));
    assert!(matches!(
        observations[1].right,
        LivingNotebookValueV2::Entity(None)
    ));
    assert!(matches!(
        observations[3].left,
        LivingNotebookValueV2::Absent
    ));
    // This independent codec fixture has synthetic attachment IDs. It proves
    // wire/formula parity; the notebook replay suite proves real attachment and
    // measured-value validation against a canonical retained archive.
}

#[test]
fn immutable_shipment_dispatch_kind_matches_independent_legacy_and_additive_bytes() {
    use nyon_workshop_core::living::{LivingRouteKindV2, LivingShipmentV2};
    use sha2::{Digest, Sha256};
    #[derive(Deserialize, Serialize)]
    #[serde(deny_unknown_fields)]
    struct ShipmentRow {
        shipment: LivingShipmentV2,
        sha256: LivingStateDigestV2,
    }
    const BYTES: &[u8] = include_bytes!("fixtures/living-v2-additions/shipments.json");
    let rows: Vec<ShipmentRow> = decode_canonical_v2(BYTES, 65536).unwrap();
    assert_eq!(encode_canonical_v2(&rows, 65536).unwrap(), BYTES);
    assert_eq!(rows.len(), 4);
    assert_eq!(rows[0].shipment.dispatch_kind, None);
    assert_eq!(rows[1].shipment.dispatch_kind, Some(LivingRouteKindV2::Aid));
    assert_eq!(
        rows[2].shipment.dispatch_kind,
        Some(LivingRouteKindV2::Trade)
    );
    assert!(rows[3].shipment.returning_to_source);
    assert_eq!(
        rows[3].shipment.disposition,
        nyon_workshop_core::living::LivingShipmentDispositionV2::WaitingForStorage
    );
    for row in rows {
        let bytes = encode_canonical_v2(&row.shipment, 65536).unwrap();
        assert_eq!(<[u8; 32]>::from(Sha256::digest(&bytes)), row.sha256.0);
    }
    let canonical: serde_json::Value = serde_json::from_slice(BYTES).unwrap();
    let legacy: LivingShipmentV2 =
        serde_json::from_value(canonical[0]["shipment"].clone()).unwrap();
    let bytes = encode_canonical_v2(&legacy, 65536).unwrap();
    for suffix in [
        b",\"dispatch_kind\":null}".as_slice(),
        b",\"returning_to_source\":false}".as_slice(),
    ] {
        let mut invalid = bytes[..bytes.len() - 1].to_vec();
        invalid.extend_from_slice(suffix);
        assert!(decode_canonical_v2::<LivingShipmentV2>(&invalid, 65536).is_err());
    }
}

#[test]
fn streaming_empty_state_matches_independent_complete_canonical_domain_witness() {
    use nyon_workshop_core::living::{
        LivingGalaxyStateV2,
        step_job::{LivingLifecycleProgressV2, LivingStateHashJobV2},
    };
    #[derive(Deserialize, Serialize)]
    struct Witness {
        state: LivingGalaxyStateV2,
        state_digest: LivingStateDigestV2,
    }
    let bytes = include_bytes!("fixtures/living-v2-additions/empty-state.json");
    let expected: Witness = decode_canonical_v2(bytes, 65536).unwrap();
    assert_eq!(encode_canonical_v2(&expected, 65536).unwrap(), bytes);
    assert_eq!(expected.state, LivingGalaxyStateV2::default());
    let source = std::sync::Arc::new(expected.state);
    for budget in [1, 64, 1024] {
        let mut job = LivingStateHashJobV2::begin(std::sync::Arc::clone(&source));
        while job.poll(budget).unwrap() != LivingLifecycleProgressV2::Ready {
            assert!(job.last_poll_visits() <= budget);
        }
        assert_eq!(job.finish().unwrap(), expected.state_digest);
    }
    assert_eq!(source.digest().unwrap(), expected.state_digest);
}

#[test]
fn route_adjustment_optional_identity_preserves_establishment_and_independent_integrity() {
    use nyon_workshop_core::living::{
        LivingEntityIdV2, LivingIntentActionV2, decode_living_decision_receipt_v2,
    };
    const ORIGINAL: &[u8] = include_bytes!("fixtures/living-v2-additions/decision-route.json");
    const ADJUSTED: &[u8] =
        include_bytes!("fixtures/living-v2-additions/decision-route-adjust.json");
    let original = decode_living_decision_receipt_v2(ORIGINAL).unwrap();
    assert_eq!(original.canonical_bytes().unwrap(), ORIGINAL);
    assert!(matches!(
        original.intents[0].action,
        LivingIntentActionV2::Route {
            existing_route: None,
            ..
        }
    ));
    let adjusted = decode_living_decision_receipt_v2(ADJUSTED).unwrap();
    assert_eq!(adjusted.canonical_bytes().unwrap(), ADJUSTED);
    assert!(
        matches!(adjusted.intents[0].action, LivingIntentActionV2::Route { existing_route: Some(id), .. } if id == LivingEntityIdV2([0xaa;16]))
    );
    assert_eq!(adjusted.intents[0].action.ordinal(), 2);
    let mut explicit_null: serde_json::Value = serde_json::from_slice(ORIGINAL).unwrap();
    explicit_null["intents"][0]["action"]["existing_route"] = serde_json::Value::Null;
    assert!(
        decode_living_decision_receipt_v2(&serde_json::to_vec(&explicit_null).unwrap()).is_err()
    );
    let mut changed: serde_json::Value = serde_json::from_slice(ADJUSTED).unwrap();
    changed["intents"][0]["action"]["existing_route"] = serde_json::Value::String("bb".repeat(16));
    assert!(decode_living_decision_receipt_v2(&serde_json::to_vec(&changed).unwrap()).is_err());
}
