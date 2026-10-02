//! Corridor locators certify replayed attempts without inventing ordinary events.
use nyon_workshop_core::living::notebook::{
    LivingNotebookRouteAttemptReferenceV2, supports_living_corridor_wait_observation_v2,
};
use nyon_workshop_core::living::*;
fn fixture() -> LivingGalaxyAuthorityV2 {
    fixture_with_energy(100)
}
fn fixture_with_energy(destination_energy: u32) -> LivingGalaxyAuthorityV2 {
    let base = builtin_first_expansion_v2().unwrap();
    let mut state = base.state().clone();
    let actor = state.civilizations.as_slice()[0].id;
    let source = state.colonies.as_slice()[0].world;
    let destination = state
        .worlds
        .as_slice()
        .iter()
        .find(|w| w.id != source)
        .unwrap()
        .id;
    let mut worlds = state.worlds.as_slice().to_vec();
    for w in &mut worlds {
        w.owner = Some(actor);
        w.inventory = LivingInventoryV2 {
            energy: if w.id == source {
                100
            } else {
                destination_energy
            },
            ore: if w.id == source { 100 } else { 0 },
            alloy: 0,
        };
    }
    state.worlds = LivingSortedVecV2::new(worlds).unwrap();
    let mut colony = state.colonies.as_slice()[0].clone();
    colony.world = destination;
    state.colonies =
        LivingSortedVecV2::new(vec![state.colonies.as_slice()[0].clone(), colony]).unwrap();
    let other = builtin_living_sandbox_v2().unwrap();
    let mut foundry = other
        .state()
        .facilities
        .as_slice()
        .iter()
        .find(|f| f.definition.as_str() == "foundry")
        .unwrap()
        .clone();
    foundry.id = LivingEntityIdV2([0xa1; 16]);
    foundry.world = destination;
    foundry.next_due = Some(LivingTickV2(20));
    state.facilities = LivingSortedVecV2::new(vec![foundry]).unwrap();
    state.fleets = Default::default();
    state.routes = LivingSortedVecV2::new(vec![LivingFreightRouteV2 {
        id: LivingEntityIdV2([0xa2; 16]),
        kind: LivingRouteKindV2::Internal,
        source_world: source,
        destination_world: destination,
        resource: LivingResourceV2::Ore,
        batch_size: 1,
        cadence_ticks: 10,
        source_reserve: 20,
        source_owner: actor,
        receiver_owner: actor,
        next_due: LivingTickV2(11),
        suspended: false,
    }])
    .unwrap();
    state.hazards = LivingSortedVecV2::new(vec![LivingHazardV2 {
        id: LivingEntityIdV2([0xa3; 16]),
        definition: LivingSlugV2::new("ion_storm").unwrap(),
        lane: state.lanes.as_slice()[0].id,
        start_tick: LivingTickV2(10),
        end_tick: LivingTickV2(150),
    }])
    .unwrap();
    let catalog = living_core_pack_v2().unwrap();
    let genesis = validate_living_genesis_manifest_v2(
        LivingGenesisManifestV2 {
            kind: LIVING_GENESIS_KIND_V2.into(),
            format_version: 2,
            rules_version: 2,
            state,
        },
        &catalog,
    )
    .unwrap();
    LivingGalaxyAuthorityV2::from_genesis(
        catalog,
        [0xa8; 32],
        LivingGenesisGeneratorV2 {
            generator_id: LivingSlugV2::new("corridor_notebook_fixture").unwrap(),
            generator_version: 1,
        },
        genesis,
    )
    .unwrap()
}
fn archive(a: &LivingGalaxyAuthorityV2) -> ValidatedLivingArchiveV2 {
    decode_living_archive_v2(&encode_living_archive_v2(a).unwrap(), a.catalog()).unwrap()
}
fn note(
    a: &LivingGalaxyAuthorityV2,
    record: LivingNotebookEvidenceRecordV2,
) -> Result<LivingNotebookV2, LivingNotebookErrorV2> {
    let b = a.bookmark().unwrap();
    LivingNotebookV2::seal_with_evidence(
        &archive(a),
        vec![LivingNamedBookmarkV2 {
            ordinal: 1,
            label: LivingNameV2::new("Attempt").unwrap(),
            view: b.view,
            digest: b.digest,
        }],
        vec![],
        Some(LivingNotebookEvidenceV2 {
            next_ordinal: 2,
            records: vec![record],
        }),
    )
}
fn validate(
    a: &LivingGalaxyAuthorityV2,
    n: &LivingNotebookV2,
) -> Result<ValidatedLivingNotebookV2, LivingNotebookErrorV2> {
    let mut job = begin_decode_living_notebook_v2(&n.canonical_bytes()?, archive(a))?;
    while job.poll(1)? == LivingArchiveReplayStatusV2::Pending {}
    job.finish()
}
fn record(a: &LivingGalaxyAuthorityV2) -> LivingNotebookEvidenceRecordV2 {
    let r = a.last_receipt().unwrap();
    let w = &a.last_corridor_restrictions()[0];
    LivingNotebookEvidenceRecordV2 {
        ordinal: 1,
        bookmark: 1,
        experiment: LivingNotebookExperimentKindV2::CorridorImpact,
        receipt_digest: r.digest,
        events: vec![],
        route_attempts: Some(vec![LivingNotebookRouteAttemptReferenceV2 {
            route: w.route(),
            hazard: w.hazard().id,
        }]),
    }
}
#[test]
fn zero_dispatch_empty_receipt_locator_replays_and_preserves_canonical_bytes() {
    let mut a = fixture();
    for _ in 0..11 {
        a.step().unwrap();
    }
    assert!(a.last_receipt().unwrap().events.is_empty());
    assert_eq!(
        a.last_corridor_restrictions()[0].actual_dispatched_units(),
        0
    );
    let n = note(&a, record(&a)).unwrap();
    let bytes = n.canonical_bytes().unwrap();
    let validated = validate(&a, &n).unwrap();
    assert_eq!(validated.notebook().canonical_bytes().unwrap(), bytes);
    assert_eq!(a.state().tick, LivingTickV2(11));
}
#[test]
fn malformed_and_forged_locators_fail_without_authority_mutation() {
    let mut a = fixture();
    for _ in 0..11 {
        a.step().unwrap();
    }
    let before = a.bookmark().unwrap();
    for axis in 0..7 {
        let mut r = record(&a);
        let duplicate = r.route_attempts.as_ref().unwrap()[0].clone();
        match axis {
            0 => r.route_attempts = Some(vec![]),
            1 => r.experiment = LivingNotebookExperimentKindV2::SupplyChain,
            2 => r.route_attempts.as_mut().unwrap().push(duplicate),
            3 => r.receipt_digest = LivingReceiptDigestV2([4; 32]),
            4 => r.route_attempts.as_mut().unwrap()[0].hazard = LivingEntityIdV2([4; 16]),
            5 => r.route_attempts.as_mut().unwrap()[0].route = LivingEntityIdV2([4; 16]),
            _ => r.route_attempts = None,
        }
        if let Ok(n) = note(&a, r) {
            assert!(validate(&a, &n).is_err(), "axis {axis}");
        }
        assert_eq!(a.bookmark().unwrap(), before);
    }
}
#[test]
fn typed_downstream_wait_admitted_but_generic_storm_event_rejected() {
    let mut a = fixture();
    for _ in 0..10 {
        a.step().unwrap();
    }
    let receipt = a.last_receipt().unwrap();
    let storm = receipt
        .events
        .iter()
        .find(|e| e.kind == LivingEventKindV2::HazardStarted)
        .unwrap();
    let generic = LivingNotebookEvidenceRecordV2 {
        ordinal: 1,
        bookmark: 1,
        experiment: LivingNotebookExperimentKindV2::CorridorImpact,
        receipt_digest: receipt.digest,
        events: vec![LivingNotebookEventReferenceV2 {
            ordinal: storm.ordinal,
            id: storm.id,
            kind: storm.kind,
        }],
        route_attempts: None,
    };
    assert!(validate(&a, &note(&a, generic).unwrap()).is_err());
    for _ in 10..20 {
        a.step().unwrap();
    }
    let receipt = a.last_receipt().unwrap();
    let wait = receipt
        .events
        .iter()
        .find(|e| e.kind == LivingEventKindV2::FacilityBlocked)
        .unwrap();
    assert!(supports_living_corridor_wait_observation_v2(
        &a, receipt, wait
    ));
    let r = LivingNotebookEvidenceRecordV2 {
        ordinal: 1,
        bookmark: 1,
        experiment: LivingNotebookExperimentKindV2::CorridorImpact,
        receipt_digest: receipt.digest,
        events: vec![LivingNotebookEventReferenceV2 {
            ordinal: wait.ordinal,
            id: wait.id,
            kind: wait.kind,
        }],
        route_attempts: None,
    };
    validate(&a, &note(&a, r).unwrap()).unwrap();
}
#[test]
fn independent_locator_wire_and_absent_legacy_bytes_are_exact() {
    for bytes in [
        include_bytes!("fixtures/living-v2-additions/notebook-corridor-attempts.json").as_slice(),
        include_bytes!("fixtures/living-v2-additions/notebook-evidence.json").as_slice(),
    ] {
        let n: LivingNotebookV2 = serde_json::from_slice(bytes).unwrap();
        assert_eq!(n.canonical_bytes().unwrap(), bytes);
    }
}

fn linked_note(a: &mut LivingGalaxyAuthorityV2) -> LivingNotebookV2 {
    linked_note_with_reset(a, false)
}
fn linked_note_with_reset(a: &mut LivingGalaxyAuthorityV2, reset_energy: bool) -> LivingNotebookV2 {
    while a.state().tick.0 < 11 {
        a.step().unwrap();
    }
    let attempt = a.bookmark().unwrap();
    let mut first = record(a);
    first.ordinal = 1;
    first.bookmark = 1;
    while a.state().tick.0 < 20 {
        if reset_energy && a.state().tick.0 == 19 {
            let world = a.state().routes.as_slice()[0].destination_world;
            let mut inventory = a
                .state()
                .worlds
                .as_slice()
                .iter()
                .find(|w| w.id == world)
                .unwrap()
                .inventory;
            inventory.energy = 0;
            let cursor = a.published_cursor();
            a.submit(LivingCommandEnvelopeV2 {
                expected_committed_revision: cursor.committed_revision,
                expected_tick: cursor.tick,
                expected_pending_sequence: cursor.pending_sequence,
                mode: LivingCommandModeV2::Running,
                command: LivingCommandV2::CreatorBatch {
                    operations: vec![LivingCreatorOperationV2::SetWorldInventory {
                        world: LivingCommandTargetV2::Existing { id: world },
                        inventory,
                    }],
                },
            })
            .unwrap();
        }
        a.step().unwrap();
    }
    let wait = a.bookmark().unwrap();
    let receipt = a.last_receipt().unwrap();
    let event = receipt
        .events
        .iter()
        .find(|event| supports_living_corridor_wait_observation_v2(a, receipt, event))
        .unwrap();
    LivingNotebookV2::seal_with_evidence(
        &archive(a),
        vec![
            LivingNamedBookmarkV2 {
                ordinal: 1,
                label: LivingNameV2::new("Restriction").unwrap(),
                view: attempt.view,
                digest: attempt.digest,
            },
            LivingNamedBookmarkV2 {
                ordinal: 2,
                label: LivingNameV2::new("Input wait").unwrap(),
                view: wait.view,
                digest: wait.digest,
            },
        ],
        vec![],
        Some(LivingNotebookEvidenceV2 {
            next_ordinal: 3,
            records: vec![
                first,
                LivingNotebookEvidenceRecordV2 {
                    ordinal: 2,
                    bookmark: 2,
                    experiment: LivingNotebookExperimentKindV2::CorridorImpact,
                    receipt_digest: receipt.digest,
                    events: vec![LivingNotebookEventReferenceV2 {
                        ordinal: event.ordinal,
                        id: event.id,
                        kind: event.kind,
                    }],
                    route_attempts: None,
                },
            ],
        }),
    )
    .unwrap()
}
#[test]
fn replay_derives_exact_zero_dispatch_and_later_input_wait_pair_at_all_budgets() {
    let mut a = fixture();
    let note = linked_note(&mut a);
    let canonical = note.canonical_bytes().unwrap();
    let mut expected = None;
    for budget in [1, 2, 17, 1024] {
        let mut job = begin_decode_living_notebook_v2(&canonical, archive(&a)).unwrap();
        assert!(job.finish().is_err());
        job = begin_decode_living_notebook_v2(&canonical, archive(&a)).unwrap();
        while job.poll(budget).unwrap() == LivingArchiveReplayStatusV2::Pending {}
        let validated = job.finish().unwrap();
        let proofs = validated.corridor_observations();
        assert_eq!(proofs.len(), 1);
        let proof = &proofs[0];
        assert_eq!(
            (
                proof.attempt_record(),
                proof.attempt_bookmark(),
                proof.wait_record(),
                proof.wait_bookmark()
            ),
            (1, 1, 2, 2)
        );
        assert_eq!(proof.attempt_view().tick, LivingTickV2(11));
        assert_eq!(proof.wait_view().tick, LivingTickV2(20));
        assert_eq!(proof.restriction().actual_dispatched_units(), 0);
        assert_eq!(proof.actor(), proof.restriction().receiver());
        assert_eq!(proof.world(), proof.restriction().destination_world());
        assert_eq!(proof.system(), proof.restriction().destination_system());
        assert_eq!(proof.resource(), LivingResourceV2::Ore);
        assert!(proof.required() > proof.available());
        assert_eq!(proof.attempt_locator().route, proof.restriction().route());
        assert_eq!(proof.wait_event().kind, LivingEventKindV2::FacilityBlocked);
        if let Some(expected) = &expected {
            assert_eq!(proofs, expected);
        } else {
            expected = Some(proofs.to_vec());
        }
        assert_eq!(
            validated.into_notebook().canonical_bytes().unwrap(),
            canonical
        );
    }
}
#[test]
fn isolated_attempt_or_wait_and_cancelled_validation_never_publish_pairs() {
    let mut a = fixture();
    let note = linked_note(&mut a);
    for remove in [0, 1] {
        let mut partial = note.clone();
        partial.evidence.as_mut().unwrap().records.remove(remove);
        // Reseal metadata after preserving one independently valid observation.
        let partial = LivingNotebookV2::seal_with_evidence(
            &archive(&a),
            partial.bookmarks,
            vec![],
            partial.evidence,
        )
        .unwrap();
        assert!(
            validate(&a, &partial)
                .unwrap()
                .corridor_observations()
                .is_empty()
        );
    }
    let boundary = a.bookmark().unwrap();
    let mut job =
        begin_decode_living_notebook_v2(&note.canonical_bytes().unwrap(), archive(&a)).unwrap();
    for _ in 0..8 {
        assert_eq!(job.poll(1).unwrap(), LivingArchiveReplayStatusV2::Pending);
    }
    job.cancel();
    assert_eq!(job.poll(1), Err(LivingNotebookErrorV2::Cancelled));
    assert!(job.finish().is_err());
    assert_eq!(a.bookmark().unwrap(), boundary);
}

#[test]
fn replay_matches_ore_wait_when_energy_is_the_first_simultaneous_shortage() {
    let mut a = fixture_with_energy(0);
    let n = linked_note_with_reset(&mut a, true);
    let sidecar = a.last_event_facts().unwrap();
    let reasons = &sidecar
        .events
        .iter()
        .find(|entry| !entry.facts.blocked_reasons.is_empty())
        .unwrap()
        .facts
        .blocked_reasons;
    assert!(matches!(
        reasons[0],
        LivingBlockedReasonV2::MissingInput {
            resource: LivingResourceV2::Energy,
            ..
        }
    ));
    assert!(reasons.iter().any(|reason| matches!(
        reason,
        LivingBlockedReasonV2::MissingInput {
            resource: LivingResourceV2::Ore,
            ..
        }
    )));
    let validated = validate(&a, &n).unwrap();
    assert_eq!(validated.corridor_observations().len(), 1);
    assert_eq!(
        validated.corridor_observations()[0].resource(),
        LivingResourceV2::Ore
    );
}
