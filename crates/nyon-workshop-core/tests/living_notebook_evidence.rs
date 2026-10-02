//! Portable observations retain exact receipt witnesses without certifying outcomes.
use nyon_workshop_core::living::*;
fn archive(a: &LivingGalaxyAuthorityV2) -> ValidatedLivingArchiveV2 {
    decode_living_archive_v2(&encode_living_archive_v2(a).unwrap(), a.catalog()).unwrap()
}
fn edit(
    a: &LivingGalaxyAuthorityV2,
    mode: LivingCommandModeV2,
    value: i32,
) -> LivingCommandEnvelopeV2 {
    let c = a.published_cursor();
    let rows = a.state().civilizations.as_slice();
    LivingCommandEnvelopeV2 {
        expected_committed_revision: c.committed_revision,
        expected_tick: c.tick,
        expected_pending_sequence: c.pending_sequence,
        mode,
        command: LivingCommandV2::CreatorBatch {
            operations: vec![LivingCreatorOperationV2::SetRelationBase {
                from: LivingCommandTargetV2::Existing { id: rows[0].id },
                to: LivingCommandTargetV2::Existing { id: rows[1].id },
                base_disposition: value,
            }],
        },
    }
}
fn named(a: &LivingGalaxyAuthorityV2, ordinal: u64) -> LivingNamedBookmarkV2 {
    let b = a.bookmark().unwrap();
    LivingNamedBookmarkV2 {
        ordinal,
        label: LivingNameV2::new("Receipt observation").unwrap(),
        view: b.view,
        digest: b.digest,
    }
}
fn record(receipt: &LivingTickReceiptV2, bookmark: u64) -> LivingNotebookEvidenceRecordV2 {
    let event = &receipt.events[0];
    LivingNotebookEvidenceRecordV2 {
        ordinal: 1,
        bookmark,
        experiment: LivingNotebookExperimentKindV2::WarAlternative,
        receipt_digest: receipt.digest,
        route_attempts: None,
        events: vec![LivingNotebookEventReferenceV2 {
            ordinal: event.ordinal,
            id: event.id,
            kind: event.kind,
        }],
    }
}
fn validate(
    n: &LivingNotebookV2,
    a: &LivingGalaxyAuthorityV2,
) -> Result<ValidatedLivingNotebookV2, LivingNotebookErrorV2> {
    let mut job = begin_decode_living_notebook_v2(&n.canonical_bytes()?, archive(a))?;
    while job.poll(128)? == LivingArchiveReplayStatusV2::Pending {}
    job.finish()
}
fn section(record: LivingNotebookEvidenceRecordV2) -> LivingNotebookEvidenceV2 {
    LivingNotebookEvidenceV2 {
        next_ordinal: 2,
        records: vec![record],
    }
}
#[test]
fn mixed_running_paused_receipt_witness_round_trip_and_wrong_cursor_rejection() {
    let mut a = builtin_living_sandbox_v2().unwrap();
    a.submit(edit(&a, LivingCommandModeV2::Running, -30))
        .unwrap();
    let receipt = a.step().unwrap();
    let before = named(&a, 1);
    a.apply_paused(edit(&a, LivingCommandModeV2::Paused, -80))
        .unwrap();
    let after = named(&a, 2);
    let n = LivingNotebookV2::seal_with_evidence(
        &archive(&a),
        vec![before, after],
        vec![],
        Some(section(record(&receipt, 1))),
    )
    .unwrap();
    let validated = validate(&n, &a).unwrap();
    assert_eq!(validated.notebook().evidence, n.evidence);
    assert_eq!(
        validated.notebook().canonical_bytes().unwrap(),
        n.canonical_bytes().unwrap()
    );
    let mut wrong = n.evidence.clone().unwrap();
    wrong.records[0].bookmark = 2;
    let wrong = LivingNotebookV2::seal_with_evidence(
        &archive(&a),
        n.bookmarks.clone(),
        vec![],
        Some(wrong),
    )
    .unwrap();
    assert!(matches!(
        validate(&wrong, &a),
        Err(LivingNotebookErrorV2::Evidence)
    ));
}
#[test]
fn forged_receipt_event_kind_id_and_provenance_reject_without_authority_mutation() {
    let mut a = builtin_living_sandbox_v2().unwrap();
    a.submit(edit(&a, LivingCommandModeV2::Running, -30))
        .unwrap();
    let receipt = a.step().unwrap();
    let before = a.bookmark().unwrap();
    for defect in 0..7 {
        let mut r = record(&receipt, 1);
        match defect {
            0 => r.receipt_digest = LivingReceiptDigestV2([7; 32]),
            1 => r.events[0].id = LivingEventIdV2([8; 16]),
            2 => r.events[0].kind = LivingEventKindV2::ShipmentDelivered,
            3 => r.events[0].ordinal = u16::MAX,
            4 => r.experiment = LivingNotebookExperimentKindV2::AutonomousSettlement,
            5 => r.experiment = LivingNotebookExperimentKindV2::DormantRecovery,
            _ => r.experiment = LivingNotebookExperimentKindV2::TradeDelivery,
        }
        let n = LivingNotebookV2::seal_with_evidence(
            &archive(&a),
            vec![named(&a, 1)],
            vec![],
            Some(section(r)),
        )
        .unwrap();
        assert!(matches!(
            validate(&n, &a),
            Err(LivingNotebookErrorV2::Evidence)
        ));
        assert_eq!(a.bookmark().unwrap(), before);
    }
}
#[test]
fn evidence_caps_ordering_allocation_and_cancellation_are_explicit() {
    let mut a = builtin_living_sandbox_v2().unwrap();
    let receipt = a
        .apply_paused(edit(&a, LivingCommandModeV2::Paused, -30))
        .unwrap();
    let r = record(&receipt, 1);
    let arc = archive(&a);
    let bookmarks = vec![named(&a, 1)];
    for malformed in [
        LivingNotebookEvidenceV2 {
            next_ordinal: 1,
            records: vec![],
        },
        LivingNotebookEvidenceV2 {
            next_ordinal: 2,
            records: vec![r.clone(), r.clone()],
        },
        LivingNotebookEvidenceV2 {
            next_ordinal: 130,
            records: (1..=129)
                .map(|ordinal| {
                    let mut r = r.clone();
                    r.ordinal = ordinal;
                    r
                })
                .collect(),
        },
    ] {
        assert!(matches!(
            LivingNotebookV2::seal_with_evidence(&arc, bookmarks.clone(), vec![], Some(malformed)),
            Err(LivingNotebookErrorV2::Shape)
        ));
    }
    let mut many = r.clone();
    many.events = vec![r.events[0].clone(); 33];
    assert!(matches!(
        LivingNotebookV2::seal_with_evidence(&arc, bookmarks.clone(), vec![], Some(section(many))),
        Err(LivingNotebookErrorV2::Shape)
    ));
    let full = LivingNotebookV2::seal_with_evidence(
        &arc,
        bookmarks.clone(),
        vec![],
        Some(LivingNotebookEvidenceV2 {
            next_ordinal: 129,
            records: (1..=128)
                .map(|ordinal| {
                    let mut record = r.clone();
                    record.ordinal = ordinal;
                    record
                })
                .collect(),
        }),
    )
    .unwrap();
    validate(&full, &a).unwrap();
    let empty = LivingNotebookV2::seal_with_evidence(
        &arc,
        bookmarks.clone(),
        vec![],
        Some(LivingNotebookEvidenceV2 {
            next_ordinal: 2,
            records: vec![],
        }),
    )
    .unwrap();
    validate(&empty, &a).unwrap();
    assert_eq!(empty.evidence.unwrap().next_ordinal, 2);
    let n =
        LivingNotebookV2::seal_with_evidence(&arc, bookmarks, vec![], Some(section(r))).unwrap();
    let before = a.bookmark().unwrap();
    let mut job = begin_decode_living_notebook_v2(&n.canonical_bytes().unwrap(), arc).unwrap();
    job.cancel();
    assert!(matches!(
        job.poll(128),
        Err(LivingNotebookErrorV2::Cancelled)
    ));
    assert!(matches!(
        job.finish(),
        Err(LivingNotebookErrorV2::Cancelled)
    ));
    assert_eq!(a.bookmark().unwrap(), before);
}
#[test]
fn independent_populated_wire_preserves_absent_old_bytes() {
    use sha2::{Digest, Sha256};
    let old = include_bytes!("fixtures/living-v2-additions/notebook.json");
    let old: LivingNotebookV2 = decode_canonical_v2(old, LIVING_MAX_NOTEBOOK_BYTES_V2).unwrap();
    assert!(old.evidence.is_none());
    assert_eq!(
        old.canonical_bytes().unwrap(),
        include_bytes!("fixtures/living-v2-additions/notebook.json")
    );
    let bytes = include_bytes!("fixtures/living-v2-additions/notebook-evidence.json");
    let n: LivingNotebookV2 = decode_canonical_v2(bytes, LIVING_MAX_NOTEBOOK_BYTES_V2).unwrap();
    assert_eq!(n.canonical_bytes().unwrap(), bytes);
    let delimiter = b",\"integrity_sha256\":";
    let boundary = bytes
        .windows(delimiter.len())
        .position(|part| part == delimiter)
        .unwrap();
    let mut payload = bytes[..boundary].to_vec();
    payload.push(b'}');
    let mut hash = Sha256::new();
    hash.update(LIVING_NOTEBOOK_DOMAIN_V2);
    hash.update(payload);
    assert_eq!(<[u8; 32]>::from(hash.finalize()), n.integrity_sha256.0);
    let null = std::str::from_utf8(include_bytes!("fixtures/living-v2-additions/notebook.json"))
        .unwrap()
        .replace(
            ",\"integrity_sha256\":",
            ",\"evidence\":null,\"integrity_sha256\":",
        );
    assert!(
        decode_canonical_v2::<LivingNotebookV2>(null.as_bytes(), LIVING_MAX_NOTEBOOK_BYTES_V2)
            .is_err()
    );
}

#[test]
fn actual_autonomous_settlement_receipt_is_a_portable_observation() {
    let mut authority = builtin_first_expansion_v2().unwrap();
    for _ in 0..400 {
        let receipt = authority.step().unwrap();
        if let Some(event) = receipt
            .events
            .iter()
            .find(|event| event.kind == LivingEventKindV2::SettlementClaimResolved)
        {
            let evidence = LivingNotebookEvidenceRecordV2 {
                ordinal: 1,
                bookmark: 1,
                experiment: LivingNotebookExperimentKindV2::AutonomousSettlement,
                receipt_digest: receipt.digest,
                route_attempts: None,
                events: vec![LivingNotebookEventReferenceV2 {
                    ordinal: event.ordinal,
                    id: event.id,
                    kind: event.kind,
                }],
            };
            let notebook = LivingNotebookV2::seal_with_evidence(
                &archive(&authority),
                vec![named(&authority, 1)],
                vec![],
                Some(section(evidence)),
            )
            .unwrap();
            validate(&notebook, &authority).unwrap();
            return;
        }
    }
    panic!("First Expansion did not supply a settlement observation within the 400-tick probe");
}
