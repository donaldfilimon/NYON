//! Exact recovery observations do not turn creator changes into autonomous events.
use nyon_workshop_core::living::*;

fn target(id: LivingEntityIdV2) -> LivingCommandTargetV2 {
    LivingCommandTargetV2::Existing { id }
}

fn fixture() -> (LivingGalaxyAuthorityV2, LivingEntityIdV2, LivingEntityIdV2) {
    fixture_with_status(LivingCivilizationStatusV2::Dormant)
}

fn fixture_with_status(
    status: LivingCivilizationStatusV2,
) -> (LivingGalaxyAuthorityV2, LivingEntityIdV2, LivingEntityIdV2) {
    fixture_with_status_and_colonies(status, false)
}

fn fixture_with_status_and_colonies(
    status: LivingCivilizationStatusV2,
    retain_colonies: bool,
) -> (LivingGalaxyAuthorityV2, LivingEntityIdV2, LivingEntityIdV2) {
    let baseline = builtin_first_expansion_v2().unwrap();
    let mut state = baseline.state().clone();
    let actor = state.civilizations.as_slice()[0].id;
    let world = if retain_colonies {
        state
            .worlds
            .as_slice()
            .iter()
            .find(|world| {
                !state
                    .colonies
                    .as_slice()
                    .iter()
                    .any(|colony| colony.world == world.id)
            })
            .unwrap()
            .id
    } else {
        state.worlds.as_slice()[0].id
    };
    let mut civilizations = state.civilizations.as_slice().to_vec();
    civilizations
        .iter_mut()
        .find(|row| row.id == actor)
        .unwrap()
        .status = status;
    state.civilizations = LivingSortedVecV2::new(civilizations).unwrap();
    let mut worlds = state.worlds.as_slice().to_vec();
    for row in &mut worlds {
        if !retain_colonies && row.owner == Some(actor) {
            row.owner = None;
        }
    }
    state.worlds = LivingSortedVecV2::new(worlds).unwrap();
    state.colonies = LivingSortedVecV2::new(
        state
            .colonies
            .as_slice()
            .iter()
            .filter(|row| retain_colonies || row.owner != actor)
            .cloned()
            .collect(),
    )
    .unwrap();
    let catalog = living_core_pack_v2().unwrap();
    let genesis = validate_living_genesis_manifest_v2(
        LivingGenesisManifestV2 {
            kind: LIVING_GENESIS_KIND_V2.into(),
            format_version: LIVING_GENESIS_FORMAT_VERSION_V2,
            rules_version: 2,
            state,
        },
        &catalog,
    )
    .unwrap();
    let authority = LivingGalaxyAuthorityV2::from_genesis(
        catalog,
        [61; 32],
        LivingGenesisGeneratorV2 {
            generator_id: LivingSlugV2::new("recovery_evidence_fixture").unwrap(),
            generator_version: 1,
        },
        genesis,
    )
    .unwrap();
    (authority, actor, world)
}

fn apply(
    authority: &mut LivingGalaxyAuthorityV2,
    mode: LivingCommandModeV2,
    operations: Vec<LivingCreatorOperationV2>,
) -> LivingTickReceiptV2 {
    let cursor = authority.published_cursor();
    let envelope = LivingCommandEnvelopeV2 {
        expected_committed_revision: cursor.committed_revision,
        expected_tick: cursor.tick,
        expected_pending_sequence: cursor.pending_sequence,
        mode,
        command: LivingCommandV2::CreatorBatch { operations },
    };
    if mode == LivingCommandModeV2::Paused {
        authority.apply_paused(envelope).unwrap()
    } else {
        authority.submit(envelope).unwrap();
        authority.step().unwrap()
    }
}

fn recovery(actor: LivingEntityIdV2, world: LivingEntityIdV2) -> Vec<LivingCreatorOperationV2> {
    vec![
        LivingCreatorOperationV2::SetWorldOwner {
            world: target(world),
            owner: Some(target(actor)),
        },
        LivingCreatorOperationV2::CreateColony {
            world: target(world),
            owner: target(actor),
        },
    ]
}

fn validate_observation(
    authority: &LivingGalaxyAuthorityV2,
    receipt: &LivingTickReceiptV2,
    kind: LivingNotebookExperimentKindV2,
) -> Result<ValidatedLivingNotebookV2, LivingNotebookErrorV2> {
    let bookmark = authority.bookmark().unwrap();
    let archive = decode_living_archive_v2(
        &encode_living_archive_v2(authority).unwrap(),
        authority.catalog(),
    )
    .unwrap();
    let event = receipt
        .events
        .iter()
        .find(|event| event.kind == LivingEventKindV2::CreatorIntervention)
        .unwrap();
    let notebook = LivingNotebookV2::seal_with_evidence(
        &archive,
        vec![LivingNamedBookmarkV2 {
            ordinal: 1,
            label: LivingNameV2::new("Creator recovery observation").unwrap(),
            view: bookmark.view,
            digest: bookmark.digest,
        }],
        vec![],
        Some(LivingNotebookEvidenceV2 {
            next_ordinal: 2,
            records: vec![LivingNotebookEvidenceRecordV2 {
                ordinal: 1,
                bookmark: 1,
                experiment: kind,
                receipt_digest: receipt.digest,
                route_attempts: None,
                events: vec![LivingNotebookEventReferenceV2 {
                    ordinal: event.ordinal,
                    id: event.id,
                    kind: event.kind,
                }],
            }],
        }),
    )?;
    let mut replay = begin_decode_living_notebook_v2(&notebook.canonical_bytes()?, archive)?;
    while replay.poll(64)? == LivingArchiveReplayStatusV2::Pending {}
    replay.finish()
}

#[test]
fn real_creator_recovery_is_portable_evidence_without_an_autonomous_reactivation_event() {
    for mode in [LivingCommandModeV2::Paused, LivingCommandModeV2::Running] {
        let (mut authority, actor, world) = fixture();
        let receipt = apply(&mut authority, mode, recovery(actor, world));
        assert_eq!(
            authority
                .state()
                .civilizations
                .as_slice()
                .iter()
                .find(|row| row.id == actor)
                .unwrap()
                .status,
            LivingCivilizationStatusV2::Active
        );
        assert_eq!(
            authority
                .state()
                .colonies
                .as_slice()
                .iter()
                .find(|row| row.world == world)
                .unwrap()
                .owner,
            actor
        );
        assert!(
            !receipt
                .events
                .iter()
                .any(|event| event.kind == LivingEventKindV2::CivilizationReactivated)
        );
        assert!(
            receipt
                .events
                .iter()
                .any(|event| event.kind == LivingEventKindV2::CreatorIntervention
                    && matches!(event.provenance, LivingEventProvenanceV2::Creator { .. }))
        );
        let before = authority.bookmark().unwrap();
        let bytes = encode_living_archive_v2(&authority).unwrap();
        validate_observation(
            &authority,
            &receipt,
            LivingNotebookExperimentKindV2::DormantRecovery,
        )
        .unwrap();
        assert!(
            validate_observation(
                &authority,
                &receipt,
                LivingNotebookExperimentKindV2::AutonomousSettlement
            )
            .is_err()
        );
        assert_eq!(authority.bookmark().unwrap(), before);
        assert_eq!(encode_living_archive_v2(&authority).unwrap(), bytes);
    }
}

#[test]
fn transient_recovery_and_unrelated_inventory_interventions_are_not_recovery_evidence() {
    for case in 0..4 {
        let (mut authority, actor, world) = fixture();
        let operations = if case == 1 {
            let mut operations = recovery(actor, world);
            operations.push(LivingCreatorOperationV2::RemoveColony {
                world: target(world),
            });
            operations
        } else if case == 2 {
            vec![LivingCreatorOperationV2::SetWorldOwner {
                world: target(world),
                owner: Some(target(actor)),
            }]
        } else if case == 3 {
            vec![LivingCreatorOperationV2::EditCivilization {
                target: target(actor),
                name: LivingNameV2::new("Renamed dormant actor").unwrap(),
                policy: authority.state().civilizations.as_slice()[0].policy.clone(),
            }]
        } else {
            vec![LivingCreatorOperationV2::SetWorldInventory {
                world: target(world),
                inventory: LivingInventoryV2 {
                    energy: 23,
                    ore: 17,
                    alloy: 5,
                },
            }]
        };
        let receipt = apply(&mut authority, LivingCommandModeV2::Paused, operations);
        assert_eq!(
            authority
                .state()
                .civilizations
                .as_slice()
                .iter()
                .find(|row| row.id == actor)
                .unwrap()
                .status,
            LivingCivilizationStatusV2::Dormant
        );
        assert!(
            validate_observation(
                &authority,
                &receipt,
                LivingNotebookExperimentKindV2::DormantRecovery
            )
            .is_err()
        );
    }
}

#[test]
fn initially_active_actor_gaining_a_colony_is_not_dormant_recovery() {
    let (mut authority, actor, world) = fixture_with_status(LivingCivilizationStatusV2::Active);
    let receipt = apply(
        &mut authority,
        LivingCommandModeV2::Paused,
        recovery(actor, world),
    );
    assert!(
        validate_observation(
            &authority,
            &receipt,
            LivingNotebookExperimentKindV2::DormantRecovery
        )
        .is_err()
    );
}

#[test]
fn dormant_status_with_an_existing_colony_is_not_colony_loss_recovery() {
    let (mut authority, actor, world) =
        fixture_with_status_and_colonies(LivingCivilizationStatusV2::Dormant, true);
    assert!(
        authority
            .state()
            .colonies
            .as_slice()
            .iter()
            .any(|colony| colony.owner == actor)
    );
    let receipt = apply(
        &mut authority,
        LivingCommandModeV2::Paused,
        recovery(actor, world),
    );
    assert!(
        validate_observation(
            &authority,
            &receipt,
            LivingNotebookExperimentKindV2::DormantRecovery
        )
        .is_err()
    );
}

#[test]
fn creating_a_new_active_civilization_with_a_colony_is_not_existing_actor_recovery() {
    let (mut authority, _, world) = fixture();
    let new_actor = LivingCommandTargetV2::Local { local: 0 };
    let receipt = apply(
        &mut authority,
        LivingCommandModeV2::Paused,
        vec![
            LivingCreatorOperationV2::CreateCivilization {
                local: 0,
                name: LivingNameV2::new("New civilization").unwrap(),
                policy: LivingPolicyV2(LivingSlugV2::new("neutral").unwrap()),
                status: LivingCivilizationStatusV2::Active,
            },
            LivingCreatorOperationV2::SetWorldOwner {
                world: target(world),
                owner: Some(new_actor),
            },
            LivingCreatorOperationV2::CreateColony {
                world: target(world),
                owner: new_actor,
            },
        ],
    );
    assert!(
        validate_observation(
            &authority,
            &receipt,
            LivingNotebookExperimentKindV2::DormantRecovery
        )
        .is_err()
    );
}

#[test]
fn running_receipt_cannot_borrow_another_revision_proof_or_a_lost_final_colony() {
    for loses_colony in [false, true] {
        let (mut authority, actor, world) = fixture();
        let batches = if loses_colony {
            vec![
                recovery(actor, world),
                vec![LivingCreatorOperationV2::RemoveColony {
                    world: target(world),
                }],
            ]
        } else {
            vec![
                vec![LivingCreatorOperationV2::SetWorldInventory {
                    world: target(world),
                    inventory: LivingInventoryV2 {
                        energy: 23,
                        ore: 17,
                        alloy: 5,
                    },
                }],
                recovery(actor, world),
            ]
        };
        for operations in batches {
            let cursor = authority.published_cursor();
            authority
                .submit(LivingCommandEnvelopeV2 {
                    expected_committed_revision: cursor.committed_revision,
                    expected_tick: cursor.tick,
                    expected_pending_sequence: cursor.pending_sequence,
                    mode: LivingCommandModeV2::Running,
                    command: LivingCommandV2::CreatorBatch { operations },
                })
                .unwrap();
        }
        let receipt = authority.step().unwrap();
        assert_eq!(authority.last_creator_recoveries().is_empty(), loses_colony);
        // The first event refers to either the lost recovery or the unrelated edit.
        assert!(
            validate_observation(
                &authority,
                &receipt,
                LivingNotebookExperimentKindV2::DormantRecovery
            )
            .is_err()
        );
    }
}
