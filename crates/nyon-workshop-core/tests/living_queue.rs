//! Living Galaxy V2 creator queue and atomic boundary orchestration.
//!
//! Section 2 of `docs/superpowers/specs/2026-09-04-nyon-living-galaxy-rules.md`
//! is the contract under test: three submission expectations, same-boundary
//! chaining against a private projection, document-global sequences that a
//! rejection never consumes, synchronous paused application, and a boundary
//! that commits the whole candidate or nothing.
//!
//! No expected hash is written down in this file. Every digest assertion is a
//! structural identity between two things the crate derives.

use nyon_workshop_core::living::{
    LIVING_MAX_PENDING_QUEUE_DEPTH_V2, LivingAuthorityErrorV2, LivingCivilizationStatusV2,
    LivingCommandCursorV2, LivingCommandEnvelopeV2, LivingCommandErrorV2, LivingCommandModeV2,
    LivingCommandRejectionV2, LivingCommandTargetV2, LivingCommandV2, LivingCreatorOperationV2,
    LivingEntityIdV2, LivingEventKindV2, LivingEventProvenanceV2, LivingGalaxyAuthorityV2,
    LivingGalaxyStateV2, LivingGenesisGeneratorV2, LivingGenesisManifestV2, LivingInventoryV2,
    LivingNameV2, LivingPolicyV2, LivingRevisionIdV2, LivingSlugV2, LivingTickV2,
    LivingValidationErrorV2, ValidatedLivingCatalogPackV2, decode_living_catalog_pack_v2,
    living_core_pack_v2, validate_living_genesis_manifest_v2,
};

const SEED: [u8; 32] = [0x51; 32];

fn catalog() -> ValidatedLivingCatalogPackV2 {
    living_core_pack_v2().expect("the built-in pack validates")
}

fn name(text: &str) -> LivingNameV2 {
    LivingNameV2::new(text).expect("valid name")
}

fn slug(text: &str) -> LivingSlugV2 {
    LivingSlugV2::new(text).expect("valid slug")
}

fn generator() -> LivingGenesisGeneratorV2 {
    LivingGenesisGeneratorV2 {
        generator_id: slug("empty_test_galaxy"),
        generator_version: 1,
    }
}

/// An empty tick-zero galaxy: section 6 lets zero civilizations keep stepping.
fn authority() -> LivingGalaxyAuthorityV2 {
    let catalog = catalog();
    let genesis = validate_living_genesis_manifest_v2(
        LivingGenesisManifestV2 {
            kind: nyon_workshop_core::living::LIVING_GENESIS_KIND_V2.to_owned(),
            format_version: nyon_workshop_core::living::LIVING_GENESIS_FORMAT_VERSION_V2,
            rules_version: nyon_workshop_core::living::LIVING_RULES_VERSION,
            state: LivingGalaxyStateV2::default(),
        },
        &catalog,
    )
    .expect("the empty galaxy is a valid genesis");
    LivingGalaxyAuthorityV2::from_genesis(catalog, SEED, generator(), genesis)
        .expect("a validated genesis starts an authority")
}

fn batch(operations: Vec<LivingCreatorOperationV2>) -> LivingCommandV2 {
    LivingCommandV2::CreatorBatch { operations }
}

fn system(local: u16, text: &str) -> LivingCreatorOperationV2 {
    LivingCreatorOperationV2::CreateSystem {
        local,
        name: name(text),
    }
}

fn envelope(
    cursor: LivingCommandCursorV2,
    mode: LivingCommandModeV2,
    command: LivingCommandV2,
) -> LivingCommandEnvelopeV2 {
    LivingCommandEnvelopeV2 {
        expected_committed_revision: cursor.committed_revision,
        expected_tick: cursor.tick,
        expected_pending_sequence: cursor.pending_sequence,
        mode,
        command,
    }
}

fn running(
    authority: &LivingGalaxyAuthorityV2,
    command: LivingCommandV2,
) -> LivingCommandEnvelopeV2 {
    envelope(
        authority.published_cursor(),
        LivingCommandModeV2::Running,
        command,
    )
}

fn paused(
    authority: &LivingGalaxyAuthorityV2,
    command: LivingCommandV2,
) -> LivingCommandEnvelopeV2 {
    envelope(
        authority.published_cursor(),
        LivingCommandModeV2::Paused,
        command,
    )
}

fn existing(id: LivingEntityIdV2) -> LivingCommandTargetV2 {
    LivingCommandTargetV2::Existing { id }
}

fn only_system(authority: &LivingGalaxyAuthorityV2) -> LivingEntityIdV2 {
    let systems = authority.state().systems.as_slice();
    assert_eq!(systems.len(), 1);
    systems[0].id
}

// ------------------------------------------------------------ the cursor

#[test]
fn a_fresh_authority_publishes_tick_zero_with_no_revision_and_an_empty_tail() {
    let authority = authority();
    assert_eq!(
        authority.published_cursor(),
        LivingCommandCursorV2 {
            committed_revision: None,
            tick: LivingTickV2(0),
            pending_sequence: 0,
        }
    );
    assert_eq!(authority.state().tick, LivingTickV2(0));
    assert!(authority.revisions().is_empty());
    assert!(authority.fault().is_none());
    assert_eq!(authority.accepted_sequence_high_water(), 0);
}

#[test]
fn a_genesis_validated_under_another_catalog_is_refused() {
    let catalog = catalog();
    let genesis = validate_living_genesis_manifest_v2(
        LivingGenesisManifestV2 {
            kind: nyon_workshop_core::living::LIVING_GENESIS_KIND_V2.to_owned(),
            format_version: nyon_workshop_core::living::LIVING_GENESIS_FORMAT_VERSION_V2,
            rules_version: nyon_workshop_core::living::LIVING_RULES_VERSION,
            state: LivingGalaxyStateV2::default(),
        },
        &catalog,
    )
    .unwrap();
    let other =
        decode_living_catalog_pack_v2(include_bytes!("fixtures/living-v2/minimal-pack.json"))
            .expect("the minimal pack validates");
    assert_ne!(other.catalog_hash(), catalog.catalog_hash());
    assert!(matches!(
        LivingGalaxyAuthorityV2::from_genesis(other, SEED, generator(), genesis),
        Err(LivingAuthorityErrorV2::CatalogMismatch)
    ));
}

// ------------------------------------------------- tail-aware submission

#[test]
fn two_running_commands_chain_against_successive_pending_tails() {
    let mut authority = authority();

    let first = authority
        .submit(running(&authority, batch(vec![system(0, "Vale")])))
        .expect("first submission is accepted");
    assert_eq!(first.accepted_sequence, 1);
    assert_eq!(first.application_tick, LivingTickV2(1));
    assert_eq!(authority.published_cursor().pending_sequence, 1);
    assert_eq!(
        authority.published_cursor().committed_revision,
        None,
        "queued work is not committed"
    );

    let second = authority
        .submit(running(&authority, batch(vec![system(0, "Confluence")])))
        .expect("the second command appends to the new tail");
    assert_eq!(second.accepted_sequence, 2);
    assert_eq!(second.application_tick, LivingTickV2(1));
    assert_eq!(authority.published_cursor().pending_sequence, 2);

    // Nothing is applied before the boundary.
    assert!(authority.state().systems.as_slice().is_empty());

    let receipt = authority.step().expect("the boundary commits");
    assert_eq!(receipt.tick, LivingTickV2(1));
    assert_eq!(receipt.applied_revisions.len(), 2);
    assert_eq!(authority.state().systems.as_slice().len(), 2);

    let revisions = authority.revisions();
    assert_eq!(revisions.len(), 2);
    assert_eq!(revisions[0].parent, None);
    assert_eq!(revisions[1].parent, Some(revisions[0].id));
    assert_eq!(revisions[0].ordinal, 1);
    assert_eq!(revisions[1].ordinal, 2);
    assert!(
        revisions
            .iter()
            .all(|revision| revision.tick == LivingTickV2(1))
    );

    assert_eq!(
        authority.published_cursor(),
        LivingCommandCursorV2 {
            committed_revision: Some(revisions[1].id),
            tick: LivingTickV2(1),
            pending_sequence: 0,
        },
        "a successful boundary publishes its final revision, tick, and a zero tail"
    );
    assert_eq!(authority.state().accepted_sequence, 2);
}

#[test]
fn a_second_command_sees_the_first_commands_entities_in_its_projection() {
    let mut authority = authority();
    authority
        .submit(running(&authority, batch(vec![system(0, "Vale")])))
        .unwrap();
    authority.step().unwrap();
    let vale = only_system(&authority);

    // Queue a star into an existing system, then a second star that only
    // validates because the projection holds the first queued command.
    authority
        .submit(running(
            &authority,
            batch(vec![LivingCreatorOperationV2::CreateStar {
                local: 0,
                system: existing(vale),
                archetype: slug("main_sequence"),
                name: name("Vale Prime"),
            }]),
        ))
        .unwrap();
    let queued_star_system = authority
        .submit(running(&authority, batch(vec![system(0, "Outer")])))
        .unwrap();
    assert_eq!(queued_star_system.accepted_sequence, 3);

    let receipt = authority.step().unwrap();
    assert_eq!(receipt.applied_revisions.len(), 2);
    assert_eq!(authority.state().stars.as_slice().len(), 1);
    assert_eq!(authority.state().systems.as_slice().len(), 2);
}

#[test]
fn each_stale_expectation_is_rejected_without_consuming_a_sequence() {
    let mut authority = authority();
    authority
        .submit(running(&authority, batch(vec![system(0, "Vale")])))
        .unwrap();
    let cursor = authority.published_cursor();

    let mut stale_revision = running(&authority, batch(vec![system(0, "A")]));
    stale_revision.expected_committed_revision = Some(LivingRevisionIdV2([0x77; 32]));
    assert!(matches!(
        authority.submit(stale_revision),
        Err(LivingCommandRejectionV2::StaleRevision { .. })
    ));

    let mut past_tick = running(&authority, batch(vec![system(0, "A")]));
    past_tick.expected_tick = LivingTickV2(7);
    assert!(matches!(
        authority.submit(past_tick),
        Err(LivingCommandRejectionV2::StaleTick { .. })
    ));

    let mut future_tick = running(&authority, batch(vec![system(0, "A")]));
    future_tick.expected_tick = LivingTickV2(cursor.tick.0 + 1);
    assert!(matches!(
        authority.submit(future_tick),
        Err(LivingCommandRejectionV2::StaleTick { .. })
    ));

    for tail in [0, 2, 9] {
        let mut stale_tail = running(&authority, batch(vec![system(0, "A")]));
        stale_tail.expected_pending_sequence = tail;
        assert!(matches!(
            authority.submit(stale_tail),
            Err(LivingCommandRejectionV2::StalePendingSequence { .. })
        ));
    }

    assert_eq!(
        authority.published_cursor(),
        cursor,
        "the queue is unchanged"
    );
    assert_eq!(authority.accepted_sequence_high_water(), 1);

    let next = authority
        .submit(running(&authority, batch(vec![system(0, "B")])))
        .unwrap();
    assert_eq!(next.accepted_sequence, 2, "rejections consumed nothing");
}

#[test]
fn structural_and_projected_failures_are_rejected_at_submission() {
    let mut authority = authority();

    let gap = batch(vec![system(0, "A"), system(2, "B")]);
    assert!(matches!(
        authority.submit(running(&authority, gap)),
        Err(LivingCommandRejectionV2::Command(
            LivingCommandErrorV2::LocalIdentifierOutOfOrder { .. }
        ))
    ));

    let dangling = batch(vec![LivingCreatorOperationV2::CreateStar {
        local: 0,
        system: existing(LivingEntityIdV2([0x42; 16])),
        archetype: slug("main_sequence"),
        name: name("Nowhere"),
    }]);
    assert!(matches!(
        authority.submit(running(&authority, dangling)),
        Err(LivingCommandRejectionV2::Projection(
            LivingValidationErrorV2::DanglingReference { .. }
        ))
    ));

    let missing_world = batch(vec![LivingCreatorOperationV2::SetWorldInventory {
        world: existing(LivingEntityIdV2([0x43; 16])),
        inventory: LivingInventoryV2::default(),
    }]);
    assert!(matches!(
        authority.submit(running(&authority, missing_world)),
        Err(LivingCommandRejectionV2::MissingTarget { operation: 0, .. })
    ));

    assert_eq!(authority.accepted_sequence_high_water(), 0);
    assert_eq!(authority.published_cursor().pending_sequence, 0);
}

#[test]
fn a_projected_conflict_with_an_earlier_queued_command_is_rejected() {
    let mut authority = authority();
    // Alone against the committed state, each command below is valid. Only
    // the projection that already holds the first one can see that the
    // second would exceed section 1's 64-system maximum.
    let full: Vec<_> = (0..64_u16)
        .map(|local| system(local, &format!("S{local}")))
        .collect();
    authority.submit(running(&authority, batch(full))).unwrap();
    assert!(matches!(
        authority.submit(running(&authority, batch(vec![system(0, "One more")]))),
        Err(LivingCommandRejectionV2::Projection(
            LivingValidationErrorV2::Capacity { .. }
        ))
    ));
    assert_eq!(authority.published_cursor().pending_sequence, 1);
    assert_eq!(authority.accepted_sequence_high_water(), 1);
    authority.step().unwrap();
    assert_eq!(authority.state().systems.as_slice().len(), 64);
}

#[test]
fn the_running_queue_is_bounded_by_the_declared_depth() {
    let mut authority = authority();
    for _ in 0..LIVING_MAX_PENDING_QUEUE_DEPTH_V2 {
        authority
            .submit(running(&authority, batch(Vec::new())))
            .expect("the queue accepts up to its depth");
    }
    let high_water = authority.accepted_sequence_high_water();
    assert!(matches!(
        authority.submit(running(&authority, batch(Vec::new()))),
        Err(LivingCommandRejectionV2::QueueFull { .. })
    ));
    assert_eq!(authority.accepted_sequence_high_water(), high_water);

    let receipt = authority.step().unwrap();
    assert_eq!(
        receipt.applied_revisions.len(),
        LIVING_MAX_PENDING_QUEUE_DEPTH_V2
    );
}

#[test]
fn a_paused_envelope_cannot_be_queued_and_a_running_one_cannot_apply_paused() {
    let mut authority = authority();
    assert!(matches!(
        authority.submit(paused(&authority, batch(Vec::new()))),
        Err(LivingCommandRejectionV2::ModeMismatch { .. })
    ));
    assert!(matches!(
        authority.apply_paused(running(&authority, batch(Vec::new()))),
        Err(LivingCommandRejectionV2::ModeMismatch { .. })
    ));
    assert_eq!(authority.accepted_sequence_high_water(), 0);
}

// ------------------------------------------------------ paused application

#[test]
fn paused_application_commits_at_the_current_tick_with_a_revision_and_receipt() {
    let mut authority = authority();
    let receipt = authority
        .apply_paused(paused(&authority, batch(vec![system(0, "Vale")])))
        .expect("paused application commits synchronously");

    assert_eq!(receipt.tick, LivingTickV2(0), "time does not advance");
    assert_eq!(authority.state().tick, LivingTickV2(0));
    assert_eq!(authority.state().systems.as_slice().len(), 1);
    assert_eq!(authority.state().accepted_sequence, 1);

    let revisions = authority.revisions();
    assert_eq!(revisions.len(), 1);
    assert_eq!(revisions[0].tick, LivingTickV2(0));
    assert_eq!(revisions[0].ordinal, 1);
    assert_eq!(receipt.applied_revisions, vec![revisions[0].id]);
    assert_eq!(receipt.state_digest, authority.state().digest().unwrap());
    receipt.verify().expect("the receipt seals its own payload");

    assert_eq!(receipt.events.len(), 1);
    assert_eq!(
        receipt.events[0].kind,
        LivingEventKindV2::CreatorIntervention
    );
    assert_eq!(
        receipt.events[0].provenance,
        LivingEventProvenanceV2::Creator {
            revision: revisions[0].id
        }
    );

    assert_eq!(
        authority.published_cursor(),
        LivingCommandCursorV2 {
            committed_revision: Some(revisions[0].id),
            tick: LivingTickV2(0),
            pending_sequence: 0,
        }
    );
}

#[test]
fn paused_application_requires_an_empty_running_queue() {
    let mut authority = authority();
    authority
        .submit(running(&authority, batch(vec![system(0, "Vale")])))
        .unwrap();
    let before = authority.state().clone();
    assert!(matches!(
        authority.apply_paused(paused(&authority, batch(vec![system(0, "Other")]))),
        Err(LivingCommandRejectionV2::PausedWithRunningQueue)
    ));
    assert_eq!(authority.state(), &before);
    assert_eq!(authority.accepted_sequence_high_water(), 1);
}

#[test]
fn a_rejected_paused_edit_changes_nothing() {
    let mut authority = authority();
    let before = authority.state().clone();
    let bad_policy = batch(vec![LivingCreatorOperationV2::CreateCivilization {
        local: 0,
        name: name("Nobody"),
        policy: LivingPolicyV2(slug("not_a_policy")),
        status: LivingCivilizationStatusV2::Active,
    }]);
    assert!(matches!(
        authority.apply_paused(paused(&authority, bad_policy)),
        Err(LivingCommandRejectionV2::Projection(_))
    ));
    assert_eq!(authority.state(), &before);
    assert!(authority.revisions().is_empty());
    assert_eq!(authority.accepted_sequence_high_water(), 0);
}

// ------------------------------------------------------------- the boundary

#[test]
fn an_empty_galaxy_steps_without_events_or_a_terminal_state() {
    let mut authority = authority();
    for expected in 1..=3_u64 {
        let receipt = authority.step().expect("an empty galaxy keeps stepping");
        assert_eq!(receipt.tick, LivingTickV2(expected));
        assert!(receipt.events.is_empty());
        assert!(receipt.applied_revisions.is_empty());
        assert_eq!(receipt.state_digest, authority.state().digest().unwrap());
        receipt.verify().unwrap();
    }
    assert_eq!(authority.published_cursor().tick, LivingTickV2(3));
    assert_eq!(authority.published_cursor().committed_revision, None);
}

#[test]
fn a_running_command_applies_at_the_boundary_after_its_acceptance() {
    let mut authority = authority();
    authority.step().unwrap();
    authority.step().unwrap();
    let accepted = authority
        .submit(running(&authority, batch(vec![system(0, "Vale")])))
        .unwrap();
    assert_eq!(accepted.application_tick, LivingTickV2(3));
    let receipt = authority.step().unwrap();
    assert_eq!(receipt.tick, LivingTickV2(3));
    assert_eq!(authority.revisions()[0].tick, LivingTickV2(3));
    assert_eq!(receipt.events.len(), 1);
}

#[test]
fn created_entities_take_the_identity_their_revision_derives() {
    let mut authority = authority();
    authority
        .submit(running(
            &authority,
            batch(vec![
                system(0, "Vale"),
                system(1, "Confluence"),
                LivingCreatorOperationV2::CreateLane {
                    local: 2,
                    system_a: LivingCommandTargetV2::Local { local: 0 },
                    system_b: LivingCommandTargetV2::Local { local: 1 },
                    distance_units: 120,
                },
            ]),
        ))
        .unwrap();
    authority.step().unwrap();
    let revision = &authority.revisions()[0];
    let created = revision.created_entities();
    let systems: Vec<_> = authority
        .state()
        .systems
        .as_slice()
        .iter()
        .map(|row| row.id)
        .collect();
    assert!(systems.contains(&created[0].2));
    assert!(systems.contains(&created[1].2));
    let lane = &authority.state().lanes.as_slice()[0];
    assert_eq!(lane.id, created[2].2);
    assert!(
        lane.system_a < lane.system_b,
        "endpoints are stored lower-identified first whatever order the batch named"
    );
}

#[test]
fn two_authorities_fed_the_same_history_agree_byte_for_byte() {
    let script = |authority: &mut LivingGalaxyAuthorityV2| {
        authority
            .apply_paused(paused(authority, batch(vec![system(0, "Vale")])))
            .unwrap();
        authority
            .submit(running(authority, batch(vec![system(0, "Outer")])))
            .unwrap();
        let mut receipts = Vec::new();
        for _ in 0..3 {
            receipts.push(authority.step().unwrap());
        }
        receipts
    };
    let mut left = authority();
    let mut right = authority();
    let left_receipts = script(&mut left);
    let right_receipts = script(&mut right);
    assert_eq!(left_receipts, right_receipts);
    assert_eq!(
        left.state().canonical_bytes().unwrap(),
        right.state().canonical_bytes().unwrap()
    );
    assert_eq!(left.revisions(), right.revisions());
}

#[test]
fn operations_this_slice_does_not_apply_are_refused_rather_than_skipped() {
    let mut authority = authority();
    let unsupported = batch(vec![LivingCreatorOperationV2::RemoveColony {
        world: existing(LivingEntityIdV2([0x45; 16])),
    }]);
    assert!(matches!(
        authority.submit(running(&authority, unsupported)),
        Err(LivingCommandRejectionV2::UnsupportedOperation { operation: 0 })
    ));
    assert_eq!(authority.accepted_sequence_high_water(), 0);
}

#[test]
fn edits_apply_to_existing_records() {
    let mut authority = authority();
    authority
        .apply_paused(paused(
            &authority,
            batch(vec![
                system(0, "Vale"),
                LivingCreatorOperationV2::CreateWorld {
                    local: 1,
                    system: LivingCommandTargetV2::Local { local: 0 },
                    archetype: slug("temperate"),
                    name: name("Hearth"),
                    owner: None,
                    inventory: LivingInventoryV2::default(),
                },
                LivingCreatorOperationV2::CreateDeposit {
                    local: 2,
                    world: LivingCommandTargetV2::Local { local: 1 },
                    resource: nyon_workshop_core::living::LivingResourceV2::Ore,
                    remaining_units: 500,
                },
                LivingCreatorOperationV2::CreateCivilization {
                    local: 3,
                    name: name("Kin"),
                    policy: LivingPolicyV2(slug("neutral")),
                    status: LivingCivilizationStatusV2::Dormant,
                },
            ]),
        ))
        .unwrap();
    let world = authority.state().worlds.as_slice()[0].id;
    let deposit = authority.state().deposits.as_slice()[0].id;
    let civilization = authority.state().civilizations.as_slice()[0].id;
    let stock = LivingInventoryV2 {
        energy: 10,
        ore: 20,
        alloy: 30,
    };
    authority
        .apply_paused(paused(
            &authority,
            batch(vec![
                LivingCreatorOperationV2::SetWorldInventory {
                    world: existing(world),
                    inventory: stock,
                },
                LivingCreatorOperationV2::SetDepositRemaining {
                    deposit: existing(deposit),
                    remaining_units: 7,
                },
                LivingCreatorOperationV2::SetCivilizationPolicy {
                    civilization: existing(civilization),
                    policy: LivingPolicyV2(slug("trader")),
                },
            ]),
        ))
        .unwrap();
    let state = authority.state();
    assert_eq!(state.worlds.as_slice()[0].inventory, stock);
    assert_eq!(state.deposits.as_slice()[0].remaining_units, 7);
    assert_eq!(
        state.civilizations.as_slice()[0].policy,
        LivingPolicyV2(slug("trader"))
    );
    assert_eq!(authority.revisions().len(), 2);
    assert_eq!(
        authority.revisions()[1].parent,
        Some(authority.revisions()[0].id)
    );
}
