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
fn removal_of_a_missing_colony_is_refused_without_spending_sequence() {
    let mut authority = authority();
    let unsupported = batch(vec![LivingCreatorOperationV2::RemoveColony {
        world: existing(LivingEntityIdV2([0x45; 16])),
    }]);
    assert!(matches!(
        authority.submit(running(&authority, unsupported)),
        Err(LivingCommandRejectionV2::MissingTarget {
            operation: 0,
            reference: "colony"
        })
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

#[test]
fn creator_replacement_is_publicly_admitted_from_authored_genesis_and_matches_staged_step() {
    use nyon_workshop_core::living::*;
    fn sorted<T: LivingKeyedV2>(row: T) -> LivingSortedVecV2<T> {
        LivingSortedVecV2::new(vec![row]).unwrap()
    }
    let template = creator_fixture_with_lane();
    let world = template.state().colonies.as_slice()[0].world;
    let owner = template.state().colonies.as_slice()[0].owner;
    let destination = template
        .state()
        .worlds
        .as_slice()
        .iter()
        .find(|row| row.id != world)
        .unwrap()
        .id;
    let fleet = template.state().fleets.as_slice()[0].clone();
    let hull = fleet.hulls.as_slice()[0].clone();
    let facility = template.state().facilities.as_slice()[0].clone();
    let lane = template.state().lanes.as_slice()[0].id;
    for kind in 0..8 {
        let operation = match kind {
            0 => LivingCreatorOperationV2::CreateFacility {
                local: 0,
                world: existing(facility.world),
                definition: facility.definition.clone(),
                status: facility.status,
                paid_alloy: facility.paid_alloy,
                hit_points: facility.hit_points,
                next_due: facility.next_due,
            },
            1 => LivingCreatorOperationV2::CreateConstructionJob {
                local: 0,
                world: existing(world),
                owner: existing(owner),
                definition: slug("solar_array"),
                accepted_tick: LivingTickV2(0),
                completion_tick: LivingTickV2(100),
                paid_alloy: 20,
            },
            2 => LivingCreatorOperationV2::CreateHullJob {
                local: 0,
                world: existing(world),
                owner: existing(owner),
                definition: hull.definition.clone(),
                accepted_tick: LivingTickV2(0),
                completion_tick: LivingTickV2(100),
                paid_alloy: 20,
                target_fleet: None,
            },
            3 => LivingCreatorOperationV2::CreateFleet {
                local: 0,
                owner: existing(owner),
                world: existing(world),
            },
            4 => LivingCreatorOperationV2::CreateHull {
                local: 0,
                fleet: existing(fleet.id),
                definition: hull.definition.clone(),
                hit_points: hull.hit_points,
                return_credit: false,
            },
            5 => LivingCreatorOperationV2::CreateRoute {
                local: 0,
                kind: LivingRouteKindV2::Internal,
                source_world: existing(world),
                destination_world: existing(destination),
                resource: LivingResourceV2::Ore,
                batch_size: 2,
                cadence_ticks: 10,
                source_reserve: 20,
                source_owner: existing(owner),
                receiver_owner: existing(owner),
                next_due: LivingTickV2(10),
            },
            6 => LivingCreatorOperationV2::CreateShipment {
                local: 0,
                dispatch_owner: existing(owner),
                intended_receiver: existing(owner),
                source_world: existing(world),
                destination_world: existing(destination),
                resource: LivingResourceV2::Ore,
                units: 2,
                departure_tick: LivingTickV2(0),
                arrival_tick: LivingTickV2(10),
                disposition: LivingShipmentDispositionV2::Outbound,
            },
            _ => LivingCreatorOperationV2::CreateHazard {
                local: 0,
                definition: template.catalog().hazards()[0].id.clone(),
                lane: existing(lane),
                start_tick: LivingTickV2(1),
                end_tick: LivingTickV2(10),
            },
        };
        let mut operations = vec![operation];
        if kind == 3 {
            operations.push(LivingCreatorOperationV2::CreateHull {
                local: 1,
                fleet: LivingCommandTargetV2::Local { local: 0 },
                definition: hull.definition.clone(),
                hit_points: hull.hit_points,
                return_credit: false,
            });
        }
        let envelope = LivingCommandEnvelopeV2 {
            expected_committed_revision: None,
            expected_tick: LivingTickV2(0),
            expected_pending_sequence: 0,
            mode: LivingCommandModeV2::Running,
            command: batch(operations),
        };
        let accepted = LivingAcceptedCommandV2 {
            accepted_sequence: 1,
            application_tick: LivingTickV2(1),
            envelope: envelope.clone(),
        };
        let revision =
            LivingRevisionV2::seal(template.catalog().catalog_hash(), SEED, None, &accepted)
                .unwrap();
        let (_, entity_kind, derived) = revision.created_entities()[0];
        assert_eq!(
            derived,
            creator_entity_id_v2(revision.id, entity_kind.ordinal(), 0)
        );
        let mut state = template.state().clone();
        match kind {
            0 => {
                let mut old = facility.clone();
                old.id = derived;
                old.disabled = true;
                state.facilities = sorted(old);
            }
            1 => {
                state.construction_jobs = sorted(LivingConstructionJobV2 {
                    id: derived,
                    world,
                    owner,
                    definition: slug("solar_array"),
                    accepted_tick: LivingTickV2(0),
                    completion_tick: LivingTickV2(90),
                    paid_alloy: 40,
                })
            }
            2 => {
                state.hull_jobs = sorted(LivingHullJobV2 {
                    id: derived,
                    world,
                    owner,
                    definition: hull.definition.clone(),
                    accepted_tick: LivingTickV2(0),
                    completion_tick: LivingTickV2(90),
                    paid_alloy: 40,
                    target_fleet: None,
                })
            }
            3 => {
                let mut old = fleet.clone();
                old.id = derived;
                state.fleets = sorted(old);
            }
            4 => {
                let mut parent = fleet.clone();
                let mut old = hull.clone();
                old.id = derived;
                old.return_credit = true;
                parent.hulls = sorted(old);
                state.fleets = sorted(parent);
            }
            5 => {
                state.routes = sorted(LivingFreightRouteV2 {
                    id: derived,
                    kind: LivingRouteKindV2::Internal,
                    source_world: world,
                    destination_world: destination,
                    resource: LivingResourceV2::Ore,
                    batch_size: 3,
                    cadence_ticks: 10,
                    source_reserve: 20,
                    source_owner: owner,
                    receiver_owner: owner,
                    next_due: LivingTickV2(10),
                    suspended: true,
                })
            }
            6 => {
                state.shipments = sorted(LivingShipmentV2 {
                    id: derived,
                    dispatch_owner: owner,
                    intended_receiver: owner,
                    source_world: world,
                    destination_world: destination,
                    resource: LivingResourceV2::Ore,
                    units: 3,
                    departure_tick: LivingTickV2(0),
                    arrival_tick: LivingTickV2(10),
                    disposition: LivingShipmentDispositionV2::WaitingForStorage,
                    dispatch_kind: Some(LivingRouteKindV2::Internal),
                    returning_to_source: true,
                })
            }
            _ => {
                state.hazards = sorted(LivingHazardV2 {
                    id: derived,
                    definition: template.catalog().hazards()[0].id.clone(),
                    lane,
                    start_tick: LivingTickV2(0),
                    end_tick: LivingTickV2(9),
                })
            }
        }
        let genesis = validate_living_genesis_manifest_v2(
            LivingGenesisManifestV2 {
                kind: LIVING_GENESIS_KIND_V2.to_owned(),
                format_version: LIVING_GENESIS_FORMAT_VERSION_V2,
                rules_version: LIVING_RULES_VERSION,
                state,
            },
            template.catalog(),
        )
        .unwrap();
        let mut admitted = LivingGalaxyAuthorityV2::from_genesis(
            template.catalog().clone(),
            SEED,
            generator(),
            genesis,
        )
        .unwrap();
        assert_eq!(admitted.submit(envelope).unwrap(), accepted);
        let mut reference = admitted.clone();
        reference.step().unwrap();
        assert_eq!(reference.revisions()[0], revision);
        assert_eq!(reference.revisions()[0].created_entities()[0].2, derived);
        for schedule in [&[1][..], &[64], &[1024], &[1, 7, 64, 2, 1024]] {
            let mut actual = admitted.clone();
            let capture = actual.capture_boundary().unwrap();
            let mut job = actual.begin_step().unwrap();
            let mut completed = false;
            for attempt in 0..100_000 {
                let budget = schedule[attempt % schedule.len()];
                let progress = job.poll(budget).unwrap();
                assert!(job.last_poll_visits() <= budget);
                assert!(actual.boundary_capture_is_current(&capture));
                assert_ne!(
                    progress,
                    LivingStepProgressV2::NeedsSynchronousCreatorOperation
                );
                if progress == LivingStepProgressV2::NeedsSynchronousCompletion {
                    completed = true;
                    break;
                }
                assert_eq!(progress, LivingStepProgressV2::Pending);
                assert!(
                    job.last_poll_visits() > 0,
                    "public Step Pending without work"
                );
            }
            assert!(completed, "public replacement Step exceeded attempt bound");
            let LivingStepPreparationV2::Ready(prepared) =
                job.finish_synchronously(&actual).unwrap()
            else {
                panic!("expected completed step")
            };
            let retired = actual.commit_step(prepared).unwrap();
            drop(retired);
            assert_eq!(actual.state(), reference.state());
            assert_eq!(actual.last_receipt(), reference.last_receipt());
            assert_eq!(actual.last_decision(), reference.last_decision());
            assert_eq!(actual.last_event_facts(), reference.last_event_facts());
            assert!(actual.last_creator_recoveries().is_empty());
        }
    }
}

#[test]
fn six_original_creator_constructor_collisions_reject_without_consuming_admission() {
    use nyon_workshop_core::living::*;
    fn add<T: LivingKeyedV2 + Clone>(
        source: &LivingSortedVecV2<T>,
        row: T,
    ) -> LivingSortedVecV2<T> {
        let mut rows = source.as_slice().to_vec();
        rows.push(row);
        rows.sort_by_key(LivingKeyedV2::living_key);
        LivingSortedVecV2::new(rows).unwrap()
    }
    let base = creator_fixture_with_lane();
    for kind in 0..6 {
        let state = base.state();
        let system = &state.systems.as_slice()[0];
        let star = &state.stars.as_slice()[0];
        let world = &state.worlds.as_slice()[0];
        let lane = &state.lanes.as_slice()[0];
        let civilization = &state.civilizations.as_slice()[0];
        let deposit = &state.deposits.as_slice()[0];
        let op = match kind {
            0 => LivingCreatorOperationV2::CreateSystem {
                local: 0,
                name: name("Collision"),
            },
            1 => LivingCreatorOperationV2::CreateStar {
                local: 0,
                system: existing(star.system),
                archetype: star.archetype.clone(),
                name: name("Collision"),
            },
            2 => LivingCreatorOperationV2::CreateWorld {
                local: 0,
                system: existing(world.system),
                archetype: world.archetype.clone(),
                name: name("Collision"),
                owner: None,
                inventory: LivingInventoryV2::default(),
            },
            3 => LivingCreatorOperationV2::CreateLane {
                local: 0,
                system_a: existing(lane.system_a),
                system_b: existing(lane.system_b),
                distance_units: lane.distance_units,
            },
            4 => LivingCreatorOperationV2::CreateCivilization {
                local: 0,
                name: name("Collision"),
                policy: civilization.policy.clone(),
                status: LivingCivilizationStatusV2::Dormant,
            },
            _ => LivingCreatorOperationV2::CreateDeposit {
                local: 0,
                world: existing(deposit.world),
                resource: deposit.resource,
                remaining_units: deposit.remaining_units,
            },
        };
        let envelope = LivingCommandEnvelopeV2 {
            expected_committed_revision: None,
            expected_tick: LivingTickV2(0),
            expected_pending_sequence: 0,
            mode: LivingCommandModeV2::Running,
            command: batch(vec![op]),
        };
        let accepted = LivingAcceptedCommandV2 {
            accepted_sequence: 1,
            application_tick: LivingTickV2(1),
            envelope: envelope.clone(),
        };
        let revision =
            LivingRevisionV2::seal(base.catalog().catalog_hash(), SEED, None, &accepted).unwrap();
        let derived = revision.created_entities()[0].2;
        let mut state = state.clone();
        match kind {
            0 => {
                let mut row = system.clone();
                row.id = derived;
                state.systems = add(&state.systems, row);
            }
            1 => {
                let mut row = star.clone();
                row.id = derived;
                state.stars = add(&state.stars, row);
            }
            2 => {
                let mut row = world.clone();
                row.id = derived;
                row.owner = None;
                state.worlds = add(&state.worlds, row);
            }
            3 => {
                let mut row = lane.clone();
                row.id = derived;
                state.lanes = add(&state.lanes, row);
            }
            4 => {
                let mut row = civilization.clone();
                row.id = derived;
                row.status = LivingCivilizationStatusV2::Dormant;
                state.civilizations = add(&state.civilizations, row);
            }
            _ => {
                let mut row = deposit.clone();
                row.id = derived;
                state.deposits = add(&state.deposits, row);
            }
        }
        let genesis = validate_living_genesis_manifest_v2(
            LivingGenesisManifestV2 {
                kind: LIVING_GENESIS_KIND_V2.to_owned(),
                format_version: LIVING_GENESIS_FORMAT_VERSION_V2,
                rules_version: LIVING_RULES_VERSION,
                state,
            },
            base.catalog(),
        )
        .unwrap();
        let mut actual = LivingGalaxyAuthorityV2::from_genesis(
            base.catalog().clone(),
            SEED,
            generator(),
            genesis,
        )
        .unwrap();
        let before = actual.state().clone();
        let cursor = actual.published_cursor();
        assert_eq!(
            actual.submit(envelope),
            Err(LivingCommandRejectionV2::Projection(
                LivingValidationErrorV2::DuplicateIdentity {
                    context: "creator batch"
                }
            ))
        );
        assert_eq!(actual.state(), &before);
        assert_eq!(actual.published_cursor(), cursor);
        assert_eq!(actual.accepted_sequence_high_water(), 0);
        assert!(actual.revisions().is_empty());
    }
}

fn creator_fixture_with_lane() -> LivingGalaxyAuthorityV2 {
    use nyon_workshop_core::living::*;
    let original = builtin_living_sandbox_v2().unwrap();
    let mut state = original.state().clone();
    let first = state.systems.as_slice()[0].id;
    let second = LivingEntityIdV2([0xf4; 16]);
    let mut systems = state.systems.into_vec();
    systems.push(LivingSystemV2 {
        id: second,
        name: name("Second system"),
    });
    systems.sort_by_key(|row| row.id);
    state.systems = LivingSortedVecV2::new(systems).unwrap();
    state.lanes = LivingSortedVecV2::new(vec![LivingLaneV2 {
        id: LivingEntityIdV2([0xf5; 16]),
        system_a: first.min(second),
        system_b: first.max(second),
        distance_units: 100,
    }])
    .unwrap();
    let genesis = validate_living_genesis_manifest_v2(
        LivingGenesisManifestV2 {
            kind: LIVING_GENESIS_KIND_V2.to_owned(),
            format_version: LIVING_GENESIS_FORMAT_VERSION_V2,
            rules_version: LIVING_RULES_VERSION,
            state,
        },
        original.catalog(),
    )
    .unwrap();
    LivingGalaxyAuthorityV2::from_genesis(original.catalog().clone(), SEED, generator(), genesis)
        .unwrap()
}

#[test]
fn colony_public_stale_success_keeps_newly_accepted_tail_and_restarts() {
    use nyon_workshop_core::living::{
        LivingStepJobErrorV2, LivingStepPreparationV2, LivingStepProgressV2,
        builtin_living_sandbox_v2,
    };
    for schedule in [&[1][..], &[64], &[1024], &[1, 7, 64, 2, 1024]] {
        let mut authority = builtin_living_sandbox_v2().unwrap();
        let colony = authority.state().colonies.as_slice()[0].clone();
        let operation = LivingCreatorOperationV2::CreateColony {
            world: LivingCommandTargetV2::Existing { id: colony.world },
            owner: LivingCommandTargetV2::Existing { id: colony.owner },
        };
        let request = running(&authority, batch(vec![operation]));
        authority.submit(request).unwrap();
        let mut job = authority.begin_step().unwrap();
        let mut done = false;
        for attempt in 0..100_000 {
            let budget = schedule[attempt % schedule.len()];
            let progress = job.poll(budget).unwrap();
            assert!(job.last_poll_visits() <= budget);
            assert_ne!(
                progress,
                LivingStepProgressV2::NeedsSynchronousCreatorOperation
            );
            if progress == LivingStepProgressV2::NeedsSynchronousCompletion {
                done = true;
                break;
            }
            assert!(job.last_poll_visits() > 0);
        }
        assert!(done);
        let LivingStepPreparationV2::Ready(prepared) =
            job.finish_synchronously(&authority).unwrap()
        else {
            panic!("colony success")
        };
        let request = running(
            &authority,
            batch(vec![LivingCreatorOperationV2::RemoveColony {
                world: LivingCommandTargetV2::Existing { id: colony.world },
            }]),
        );
        authority.submit(request).unwrap();
        let before = authority.state().canonical_bytes().unwrap();
        let cursor = authority.published_cursor();
        let high_water = authority.accepted_sequence_high_water();
        let rejected = authority.commit_step(prepared).unwrap_err();
        assert_eq!(rejected.reason(), &LivingStepJobErrorV2::Stale);
        assert_eq!(authority.state().canonical_bytes().unwrap(), before);
        assert_eq!(authority.published_cursor(), cursor);
        assert_eq!(authority.accepted_sequence_high_water(), high_water);
        assert_eq!(authority.capture_boundary().unwrap().pending_count(), 2);
        assert!(authority.fault().is_none());
        authority.step().unwrap();
        assert_eq!(authority.capture_boundary().unwrap().pending_count(), 0);
        assert!(
            !authority
                .state()
                .colonies
                .as_slice()
                .iter()
                .any(|row| row.world == colony.world)
        );
        assert_eq!(
            authority
                .state()
                .worlds
                .as_slice()
                .iter()
                .find(|row| row.id == colony.world)
                .unwrap()
                .owner,
            Some(colony.owner)
        );
    }
}

#[test]
fn diplomacy_public_stale_success_preserves_accepted_tail() {
    use nyon_workshop_core::living::{
        LivingStepJobErrorV2, LivingStepPreparationV2, LivingStepProgressV2,
        builtin_living_sandbox_v2,
    };
    for schedule in [&[1][..], &[64], &[1024], &[1, 7, 64, 2, 1024]] {
        let mut authority = builtin_living_sandbox_v2().unwrap();
        let owner = authority.state().colonies.as_slice()[0].owner;
        authority
            .submit(running(
                &authority,
                batch(vec![LivingCreatorOperationV2::ForcePeace {
                    participant_a: LivingCommandTargetV2::Existing { id: owner },
                    participant_b: LivingCommandTargetV2::Existing { id: owner },
                }]),
            ))
            .unwrap();
        let mut job = authority.begin_step().unwrap();
        let mut done = false;
        for poll in 0..100_000 {
            let budget = schedule[poll % schedule.len()];
            let progress = job.poll(budget).unwrap();
            assert!(job.last_poll_visits() <= budget);
            assert_ne!(
                progress,
                LivingStepProgressV2::NeedsSynchronousCreatorOperation
            );
            if progress == LivingStepProgressV2::NeedsSynchronousCompletion {
                done = true;
                break;
            }
        }
        assert!(done);
        let LivingStepPreparationV2::Ready(prepared) =
            job.finish_synchronously(&authority).unwrap()
        else {
            panic!("diplomacy success")
        };
        authority
            .submit(running(
                &authority,
                batch(vec![LivingCreatorOperationV2::CreateSystem {
                    local: 0,
                    name: name("Accepted diplomacy tail"),
                }]),
            ))
            .unwrap();
        let before = authority.state().canonical_bytes().unwrap();
        let cursor = authority.published_cursor();
        let high = authority.accepted_sequence_high_water();
        let rejected = authority.commit_step(prepared).unwrap_err();
        assert_eq!(rejected.reason(), &LivingStepJobErrorV2::Stale);
        assert_eq!(authority.state().canonical_bytes().unwrap(), before);
        assert_eq!(authority.published_cursor(), cursor);
        assert_eq!(authority.accepted_sequence_high_water(), high);
        assert_eq!(authority.capture_boundary().unwrap().pending_count(), 2);
        assert!(authority.fault().is_none());
        authority.step().unwrap();
        assert_eq!(authority.capture_boundary().unwrap().pending_count(), 0);
        assert!(
            authority
                .state()
                .systems
                .as_slice()
                .iter()
                .any(|r| r.name.as_str() == "Accepted diplomacy tail")
        );
    }
}

#[test]
fn cancellation_public_stale_success_preserves_accepted_tail_and_restarts() {
    use nyon_workshop_core::living::*;
    for schedule in [&[1][..], &[64], &[1024], &[1, 7, 64, 2, 1024]] {
        let mut a = builtin_first_expansion_v2().unwrap();
        let colony = a.state().colonies.as_slice()[0].clone();
        let build = a.catalog().industry("defense_battery").unwrap().clone();
        let cursor = a.published_cursor();
        a.apply_paused(LivingCommandEnvelopeV2 {
            expected_committed_revision: cursor.committed_revision,
            expected_tick: cursor.tick,
            expected_pending_sequence: cursor.pending_sequence,
            mode: LivingCommandModeV2::Paused,
            command: batch(vec![LivingCreatorOperationV2::CreateConstructionJob {
                local: 0,
                world: existing(colony.world),
                owner: existing(colony.owner),
                definition: build.id,
                accepted_tick: LivingTickV2(0),
                completion_tick: LivingTickV2(u64::from(build.build_ticks)),
                paid_alloy: build.alloy_cost,
            }]),
        })
        .unwrap();
        let target = a.state().construction_jobs.as_slice()[0].id;
        a.submit(running(
            &a,
            batch(vec![LivingCreatorOperationV2::CancelConstructionJob {
                target: existing(target),
            }]),
        ))
        .unwrap();
        let mut job = a.begin_step().unwrap();
        let mut done = false;
        for attempt in 0..100_000 {
            let b = schedule[attempt % schedule.len()];
            let progress = job.poll(b).unwrap();
            assert!(job.last_poll_visits() <= b);
            assert_ne!(
                progress,
                LivingStepProgressV2::NeedsSynchronousCreatorOperation
            );
            if progress == LivingStepProgressV2::NeedsSynchronousCompletion {
                done = true;
                break;
            }
        }
        assert!(done, "finite cancellation public Step guard");
        let LivingStepPreparationV2::Ready(prepared) = job.finish_synchronously(&a).unwrap() else {
            panic!("cancellation success")
        };
        a.submit(running(
            &a,
            batch(vec![system(0, "Accepted cancellation tail")]),
        ))
        .unwrap();
        let before = a.state().canonical_bytes().unwrap();
        let cursor = a.published_cursor();
        let high = a.accepted_sequence_high_water();
        let rejected = a.commit_step(prepared).unwrap_err();
        assert_eq!(rejected.reason(), &LivingStepJobErrorV2::Stale);
        assert_eq!(a.state().canonical_bytes().unwrap(), before);
        assert_eq!(a.published_cursor(), cursor);
        assert_eq!(a.accepted_sequence_high_water(), high);
        assert_eq!(a.capture_boundary().unwrap().pending_count(), 2);
        assert!(a.fault().is_none());
        a.step().unwrap();
        assert!(a.state().construction_jobs.as_slice().is_empty());
        assert_eq!(a.capture_boundary().unwrap().pending_count(), 0);
    }
}

#[test]
fn geometry_public_local_prefix_reviewed_suffix_and_genuine_stale_tail() {
    use nyon_workshop_core::living::*;
    for reviewed_suffix in [false, true] {
        for stale in [false, true] {
            for schedule in [&[1][..], &[64], &[1024], &[1, 7, 64, 2, 1024]] {
                let mut a = authority();
                let local = |local| LivingCommandTargetV2::Local { local };
                let mut ops = vec![
                    system(0, "A"),
                    system(1, "B"),
                    LivingCreatorOperationV2::CreateWorld {
                        local: 2,
                        system: local(0),
                        archetype: slug("barren"),
                        name: name("World"),
                        owner: None,
                        inventory: Default::default(),
                    },
                    LivingCreatorOperationV2::CreateLane {
                        local: 3,
                        system_a: local(0),
                        system_b: local(1),
                        distance_units: 10,
                    },
                    LivingCreatorOperationV2::EditWorld {
                        target: local(2),
                        system: local(1),
                        archetype: slug("barren"),
                        name: name("Moved"),
                    },
                    LivingCreatorOperationV2::EditLane {
                        target: local(3),
                        system_a: local(1),
                        system_b: local(0),
                        distance_units: 20,
                    },
                ];
                if reviewed_suffix {
                    ops.push(system(4, "Removed suffix target"));
                    ops.push(LivingCreatorOperationV2::RemoveEntity {
                        target: local(4),
                        cascade: LivingCascadeDispositionV2::RemoveReviewed {
                            dependents: Vec::new(),
                            derived: Vec::new(),
                        },
                    });
                }
                a.submit(running(&a, batch(ops))).unwrap();
                let mut reference = a.clone();
                reference.step().unwrap();
                let mut job = a.begin_step().unwrap();
                let mut ready = false;
                for attempt in 0..100000 {
                    let b = schedule[attempt % schedule.len()];
                    let progress = job.poll(b).unwrap();
                    assert!(job.last_poll_visits() <= b);
                    if progress != LivingStepProgressV2::Pending {
                        assert_eq!(progress, LivingStepProgressV2::NeedsSynchronousCompletion);
                        ready = true;
                        break;
                    }
                    assert!(job.last_poll_visits() > 0);
                }
                assert!(ready);
                let LivingStepPreparationV2::Ready(token) =
                    job.take_synchronous_preparation(&a).unwrap()
                else {
                    panic!("geometry local preparation")
                };
                assert!(job.take_synchronous_preparation(&a).is_err());
                if stale {
                    a.submit(running(
                        &a,
                        batch(vec![system(0, "Geometry accepted tail")]),
                    ))
                    .unwrap();
                    let before = a.state().canonical_bytes().unwrap();
                    let cursor = a.published_cursor();
                    let high = a.accepted_sequence_high_water();
                    let rejected = a.commit_step(token).unwrap_err();
                    assert_eq!(rejected.reason(), &LivingStepJobErrorV2::Stale);
                    assert_eq!(a.state().canonical_bytes().unwrap(), before);
                    assert_eq!(a.published_cursor(), cursor);
                    assert_eq!(a.accepted_sequence_high_water(), high);
                    assert_eq!(a.capture_boundary().unwrap().pending_count(), 2);
                    assert!(a.fault().is_none());
                    reference = a.clone();
                    reference.step().unwrap();
                    a.step().unwrap();
                } else {
                    drop(a.commit_step(token).unwrap());
                }
                assert_eq!(
                    a.state().canonical_bytes().unwrap(),
                    reference.state().canonical_bytes().unwrap()
                );
                assert_eq!(a.last_receipt(), reference.last_receipt());
                assert_eq!(a.last_decision(), reference.last_decision());
                assert_eq!(a.last_event_facts(), reference.last_event_facts());
                assert_eq!(
                    a.last_creator_recoveries(),
                    reference.last_creator_recoveries()
                );
                assert_eq!(a.revisions(), reference.revisions());
                assert_eq!(a.published_cursor(), reference.published_cursor());
                assert_eq!(
                    a.accepted_sequence_high_water(),
                    reference.accepted_sequence_high_water()
                );
                assert_eq!(a.capture_boundary().unwrap().pending_count(), 0);
                assert!(a.fault().is_none());
                let bytes = encode_living_archive_v2(&a).unwrap();
                let loaded = decode_living_archive_v2(&bytes, a.catalog())
                    .unwrap()
                    .into_authority();
                assert_eq!(loaded.bookmark().unwrap(), a.bookmark().unwrap());
            }
        }
    }
}

#[test]
fn fleet_hull_public_local_prefix_reviewed_suffix_and_genuine_stale_tail() {
    use nyon_workshop_core::living::*;
    for reviewed_suffix in [false, true] {
        for stale in [false, true] {
            for schedule in [&[1][..], &[64], &[1024], &[1, 7, 64, 2, 1024]] {
                let mut a = builtin_first_expansion_v2().unwrap();
                let world = a.state().colonies.as_slice()[0].world;
                let owner = a.state().colonies.as_slice()[0].owner;
                let local = |local| LivingCommandTargetV2::Local { local };
                let mut ops = vec![
                    LivingCreatorOperationV2::CreateFleet {
                        local: 0,
                        owner: existing(owner),
                        world: existing(world),
                    },
                    LivingCreatorOperationV2::CreateHull {
                        local: 1,
                        fleet: local(0),
                        definition: slug("scout"),
                        hit_points: 1,
                        return_credit: false,
                    },
                    LivingCreatorOperationV2::EditHull {
                        target: local(1),
                        definition: slug("escort"),
                        hit_points: 1,
                        return_credit: true,
                    },
                    LivingCreatorOperationV2::EditFleet {
                        target: local(0),
                        owner: existing(owner),
                        world: existing(world),
                    },
                ];
                if reviewed_suffix {
                    ops.push(system(2, "Removed metadata suffix target"));
                    ops.push(LivingCreatorOperationV2::RemoveEntity {
                        target: local(2),
                        cascade: LivingCascadeDispositionV2::RemoveReviewed {
                            dependents: Vec::new(),
                            derived: Vec::new(),
                        },
                    });
                }
                a.submit(running(&a, batch(ops))).unwrap();
                let mut reference = a.clone();
                reference.step().unwrap();
                let mut job = a.begin_step().unwrap();
                let mut ready = false;
                for attempt in 0..100000 {
                    let b = schedule[attempt % schedule.len()];
                    let progress = job.poll(b).unwrap();
                    assert!(job.last_poll_visits() <= b);
                    if progress != LivingStepProgressV2::Pending {
                        assert_eq!(progress, LivingStepProgressV2::NeedsSynchronousCompletion);
                        ready = true;
                        break;
                    }
                    assert!(job.last_poll_visits() > 0);
                }
                assert!(ready);
                let LivingStepPreparationV2::Ready(token) =
                    job.take_synchronous_preparation(&a).unwrap()
                else {
                    panic!("fleet hull local preparation")
                };
                assert!(job.take_synchronous_preparation(&a).is_err());
                if stale {
                    a.submit(running(
                        &a,
                        batch(vec![system(0, "Fleet hull accepted tail")]),
                    ))
                    .unwrap();
                    let before = a.state().canonical_bytes().unwrap();
                    let cursor = a.published_cursor();
                    let high = a.accepted_sequence_high_water();
                    let rejected = a.commit_step(token).unwrap_err();
                    assert_eq!(rejected.reason(), &LivingStepJobErrorV2::Stale);
                    assert_eq!(a.state().canonical_bytes().unwrap(), before);
                    assert_eq!(a.published_cursor(), cursor);
                    assert_eq!(a.accepted_sequence_high_water(), high);
                    assert_eq!(a.capture_boundary().unwrap().pending_count(), 2);
                    assert!(a.fault().is_none());
                    reference = a.clone();
                    reference.step().unwrap();
                    a.step().unwrap();
                } else {
                    drop(a.commit_step(token).unwrap());
                }
                assert_eq!(
                    a.state().canonical_bytes().unwrap(),
                    reference.state().canonical_bytes().unwrap()
                );
                assert_eq!(a.last_receipt(), reference.last_receipt());
                assert_eq!(a.last_decision(), reference.last_decision());
                assert_eq!(a.last_event_facts(), reference.last_event_facts());
                assert_eq!(
                    a.last_creator_recoveries(),
                    reference.last_creator_recoveries()
                );
                assert_eq!(a.revisions(), reference.revisions());
                assert_eq!(a.published_cursor(), reference.published_cursor());
                assert_eq!(
                    a.accepted_sequence_high_water(),
                    reference.accepted_sequence_high_water()
                );
                assert_eq!(a.capture_boundary().unwrap().pending_count(), 0);
                assert!(a.fault().is_none());
                let bytes = encode_living_archive_v2(&a).unwrap();
                let loaded = decode_living_archive_v2(&bytes, a.catalog())
                    .unwrap()
                    .into_authority();
                assert_eq!(loaded.bookmark().unwrap(), a.bookmark().unwrap());
            }
        }
    }
}

#[test]
fn structural_public_local_successive_split_merge_reviewed_suffix_and_genuine_stale_tail() {
    use nyon_workshop_core::living::*;
    for reviewed_suffix in [false, true] {
        for stale in [false, true] {
            for schedule in [&[1][..], &[64], &[1024], &[1, 7, 64, 2, 1024]] {
                let mut a = builtin_first_expansion_v2().unwrap();
                let world = a.state().colonies.as_slice()[0].world;
                let owner = a.state().colonies.as_slice()[0].owner;
                let local = |local| LivingCommandTargetV2::Local { local };
                let mut ops = vec![
                    LivingCreatorOperationV2::CreateFleet {
                        local: 0,
                        owner: existing(owner),
                        world: existing(world),
                    },
                    LivingCreatorOperationV2::CreateHull {
                        local: 1,
                        fleet: local(0),
                        definition: slug("scout"),
                        hit_points: 1,
                        return_credit: false,
                    },
                    LivingCreatorOperationV2::CreateHull {
                        local: 2,
                        fleet: local(0),
                        definition: slug("scout"),
                        hit_points: 7,
                        return_credit: true,
                    },
                    LivingCreatorOperationV2::CreateHull {
                        local: 3,
                        fleet: local(0),
                        definition: slug("escort"),
                        hit_points: 7,
                        return_credit: true,
                    },
                    LivingCreatorOperationV2::SplitFleet {
                        local: 4,
                        source: local(0),
                        hulls: vec![local(1)],
                    },
                    LivingCreatorOperationV2::MergeFleets {
                        target: local(0),
                        source: local(4),
                    },
                    LivingCreatorOperationV2::SplitFleet {
                        local: 5,
                        source: local(0),
                        hulls: vec![local(2)],
                    },
                    LivingCreatorOperationV2::MergeFleets {
                        target: local(5),
                        source: local(0),
                    },
                ];
                if reviewed_suffix {
                    ops.push(system(6, "Removed structural suffix target"));
                    ops.push(LivingCreatorOperationV2::RemoveEntity {
                        target: local(6),
                        cascade: LivingCascadeDispositionV2::RemoveReviewed {
                            dependents: Vec::new(),
                            derived: Vec::new(),
                        },
                    });
                }
                a.submit(running(&a, batch(ops))).unwrap();
                let mut reference = a.clone();
                reference.step().unwrap();
                let mut job = a.begin_step().unwrap();
                let mut ready = false;
                for attempt in 0..100000 {
                    let b = schedule[attempt % schedule.len()];
                    let progress = job.poll(b).unwrap();
                    assert!(job.last_poll_visits() <= b);
                    if progress != LivingStepProgressV2::Pending {
                        assert_eq!(progress, LivingStepProgressV2::NeedsSynchronousCompletion);
                        ready = true;
                        break;
                    }
                    assert!(job.last_poll_visits() > 0);
                }
                assert!(ready);
                let LivingStepPreparationV2::Ready(token) =
                    job.take_synchronous_preparation(&a).unwrap()
                else {
                    panic!("structural local preparation")
                };
                assert!(job.take_synchronous_preparation(&a).is_err());
                if stale {
                    a.submit(running(
                        &a,
                        batch(vec![system(0, "Structural accepted tail")]),
                    ))
                    .unwrap();
                    let before = a.state().canonical_bytes().unwrap();
                    let cursor = a.published_cursor();
                    let high = a.accepted_sequence_high_water();
                    let rejected = a.commit_step(token).unwrap_err();
                    assert_eq!(rejected.reason(), &LivingStepJobErrorV2::Stale);
                    assert_eq!(a.state().canonical_bytes().unwrap(), before);
                    assert_eq!(a.published_cursor(), cursor);
                    assert_eq!(a.accepted_sequence_high_water(), high);
                    assert_eq!(a.capture_boundary().unwrap().pending_count(), 2);
                    assert!(a.fault().is_none());
                    reference = a.clone();
                    reference.step().unwrap();
                    a.step().unwrap();
                } else {
                    drop(a.commit_step(token).unwrap());
                }
                assert_eq!(
                    a.state().canonical_bytes().unwrap(),
                    reference.state().canonical_bytes().unwrap()
                );
                assert_eq!(a.last_receipt(), reference.last_receipt());
                assert_eq!(a.last_decision(), reference.last_decision());
                assert_eq!(a.last_event_facts(), reference.last_event_facts());
                assert_eq!(
                    a.last_creator_recoveries(),
                    reference.last_creator_recoveries()
                );
                assert_eq!(a.revisions(), reference.revisions());
                assert_eq!(a.published_cursor(), reference.published_cursor());
                assert_eq!(
                    a.accepted_sequence_high_water(),
                    reference.accepted_sequence_high_water()
                );
                assert_eq!(a.capture_boundary().unwrap().pending_count(), 0);
                assert!(a.fault().is_none());
                let bytes = encode_living_archive_v2(&a).unwrap();
                assert_eq!(bytes, encode_living_archive_v2(&reference).unwrap());
                let loaded = decode_living_archive_v2(&bytes, a.catalog())
                    .unwrap()
                    .into_authority();
                assert_eq!(loaded.bookmark().unwrap(), a.bookmark().unwrap());
            }
        }
    }
}

#[test]
fn removal_public_local_multipass_prefix_suffix_current_and_genuine_stale_tail() {
    use nyon_workshop_core::living::*;
    for stale in [false, true] {
        for schedule in [&[1][..], &[64], &[1024], &[1, 7, 64, 2, 1024]] {
            let template = builtin_first_expansion_v2().unwrap();
            let mut state = template.state().clone();
            let world = state.colonies.as_slice()[0].world;
            let owner = state.colonies.as_slice()[0].owner;
            let frontier = state
                .worlds
                .as_slice()
                .iter()
                .find(|w| w.id != world)
                .unwrap()
                .clone();
            let lane = state.lanes.as_slice()[0].id;
            let mut fleets = state.fleets.into_vec();
            let fleet = fleets[0].id;
            let mut dependents = fleets[0]
                .hulls
                .as_slice()
                .iter()
                .map(|h| existing(h.id))
                .collect::<Vec<_>>();
            dependents.push(existing(fleet));
            dependents.push(LivingCommandTargetV2::Local { local: 0 });
            fleets[0].order = Some(LivingFleetOrderV2 {
                kind: LivingFleetOrderKindV2::Move,
                target_world: Some(frontier.id),
                not_before_tick: LivingTickV2(100),
                lane_path: vec![lane; 33],
            });
            state.fleets = LivingSortedVecV2::new(fleets).unwrap();
            let genesis = validate_living_genesis_manifest_v2(
                LivingGenesisManifestV2 {
                    kind: LIVING_GENESIS_KIND_V2.into(),
                    format_version: LIVING_GENESIS_FORMAT_VERSION_V2,
                    rules_version: LIVING_RULES_VERSION,
                    state,
                },
                template.catalog(),
            )
            .unwrap();
            let mut a = LivingGalaxyAuthorityV2::from_genesis(
                template.catalog().clone(),
                SEED,
                generator(),
                genesis,
            )
            .unwrap();
            let local = |local| LivingCommandTargetV2::Local { local };
            a.submit(running(
                &a,
                batch(vec![
                    LivingCreatorOperationV2::CreateHullJob {
                        local: 0,
                        world: existing(world),
                        owner: existing(owner),
                        definition: slug("scout"),
                        accepted_tick: LivingTickV2(0),
                        completion_tick: LivingTickV2(100),
                        paid_alloy: 40,
                        target_fleet: Some(existing(fleet)),
                    },
                    LivingCreatorOperationV2::RemoveEntity {
                        target: existing(lane),
                        cascade: LivingCascadeDispositionV2::RemoveListed { dependents },
                    },
                    system(1, "Transient removal system"),
                    LivingCreatorOperationV2::EditWorld {
                        target: existing(frontier.id),
                        system: local(1),
                        archetype: frontier.archetype.clone(),
                        name: frontier.name.clone(),
                    },
                    LivingCreatorOperationV2::RemoveEntity {
                        target: local(1),
                        cascade: LivingCascadeDispositionV2::RemoveListed {
                            dependents: vec![existing(frontier.id)],
                        },
                    },
                    system(2, "Surviving removal suffix"),
                ]),
            ))
            .unwrap();
            let mut reference = a.clone();
            reference.step().unwrap();
            let mut job = a.begin_step().unwrap();
            let mut done = false;
            for attempt in 0..100000 {
                let b = schedule[attempt % schedule.len()];
                let p = job.poll(b).unwrap();
                assert!(job.last_poll_visits() <= b);
                assert_ne!(p, LivingStepProgressV2::NeedsSynchronousCreatorOperation);
                if p == LivingStepProgressV2::NeedsSynchronousCompletion {
                    done = true;
                    break;
                }
                assert!(job.last_poll_visits() > 0);
            }
            assert!(done);
            let LivingStepPreparationV2::Ready(token) =
                job.take_synchronous_preparation(&a).unwrap()
            else {
                panic!("removal preparation")
            };
            assert!(job.take_synchronous_preparation(&a).is_err());
            if stale {
                a.submit(running(&a, batch(vec![system(0, "Genuine removal tail")])))
                    .unwrap();
                // Archives deliberately refuse authorities with queued commands.
                assert!(matches!(
                    encode_living_archive_v2(&a),
                    Err(LivingArchiveErrorV2::PendingQueue)
                ));
                let before = a.state().canonical_bytes().unwrap();
                let cursor = a.published_cursor();
                let high = a.accepted_sequence_high_water();
                assert_eq!(
                    a.commit_step(token).unwrap_err().reason(),
                    &LivingStepJobErrorV2::Stale
                );
                assert_eq!(a.state().canonical_bytes().unwrap(), before);
                assert_eq!(a.published_cursor(), cursor);
                assert_eq!(a.accepted_sequence_high_water(), high);
                assert_eq!(a.capture_boundary().unwrap().pending_count(), 2);
                assert!(a.fault().is_none());
                reference = a.clone();
                reference.step().unwrap();
                let mut fresh = a.begin_step().unwrap();
                loop {
                    if fresh.poll(64).unwrap() != LivingStepProgressV2::Pending {
                        break;
                    }
                }
                let LivingStepPreparationV2::Ready(token) =
                    fresh.take_synchronous_preparation(&a).unwrap()
                else {
                    panic!("fresh removal")
                };
                drop(a.commit_step(token).unwrap());
            } else {
                drop(a.commit_step(token).unwrap());
            }
            assert_eq!(a.state(), reference.state());
            assert_eq!(a.last_receipt(), reference.last_receipt());
            assert_eq!(a.last_decision(), reference.last_decision());
            assert_eq!(a.last_event_facts(), reference.last_event_facts());
            assert_eq!(
                a.last_creator_recoveries(),
                reference.last_creator_recoveries()
            );
            assert_eq!(a.revisions(), reference.revisions());
            assert_eq!(a.published_cursor(), reference.published_cursor());
            assert_eq!(
                a.accepted_sequence_high_water(),
                reference.accepted_sequence_high_water()
            );
            assert_eq!(a.fault(), reference.fault());
            assert_eq!(a.capture_boundary().unwrap().pending_count(), 0);
            assert_eq!(
                encode_living_archive_v2(&a).unwrap(),
                encode_living_archive_v2(&reference).unwrap()
            );
        }
    }
}
