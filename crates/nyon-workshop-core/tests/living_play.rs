//! Living Galaxy V2 play loop on the shipped boundary.
//!
//! These tests construct a built-in-pack authority and call `step` and
//! `apply_paused`. They do not reimplement production, freight, or construction.

use nyon_workshop_core::living::{
    LIVING_GENESIS_FORMAT_VERSION_V2, LIVING_GENESIS_KIND_V2, LIVING_RULES_VERSION,
    LivingBlockedReasonV2, LivingCommandEnvelopeV2, LivingCommandModeV2, LivingCommandTargetV2,
    LivingCommandV2, LivingCreatorOperationV2, LivingEventKindV2, LivingFacilityStatusV2,
    LivingGalaxyAuthorityV2, LivingGalaxyStateV2, LivingGenesisGeneratorV2,
    LivingGenesisManifestV2, LivingInventoryV2, LivingNameV2, LivingPolicyV2, LivingResourceV2,
    LivingSlugV2, LivingTickV2, builtin_living_sandbox_v2, living_core_pack_v2,
    validate_living_genesis_manifest_v2,
};

fn step_to(authority: &mut LivingGalaxyAuthorityV2, tick: u64) {
    while authority.state().tick.0 < tick {
        let next = authority.state().tick.0 + 1;
        authority
            .step()
            .unwrap_or_else(|fault| panic!("boundary {next} faulted: {fault:?}"));
    }
}

fn world_named<'a>(
    authority: &'a LivingGalaxyAuthorityV2,
    label: &str,
) -> &'a nyon_workshop_core::living::LivingWorldV2 {
    authority
        .state()
        .worlds
        .as_slice()
        .iter()
        .find(|world| world.name.as_str() == label)
        .unwrap_or_else(|| panic!("missing world {label}"))
}

#[test]
fn builtin_boundary_changes_economy_construction_and_freight() {
    let mut first = builtin_living_sandbox_v2().expect("built-in sandbox");
    let mut second = builtin_living_sandbox_v2().expect("built-in sandbox");
    let energy_before = world_named(&first, "Home A").inventory.energy;
    let ore_before = world_named(&first, "Home A").inventory.ore;
    let alloy_before = world_named(&first, "Home A").inventory.alloy;
    assert!(first.state().shipments.as_slice().is_empty());
    assert!(first.state().hull_jobs.as_slice().is_empty());
    assert_eq!(first.state().tick, LivingTickV2(0));

    let mut dispatched = false;
    for _ in 0..10 {
        let receipt = first.step().expect("boundary");
        dispatched |= receipt
            .events
            .iter()
            .any(|event| event.kind == LivingEventKindV2::ShipmentDispatched);
    }
    step_to(&mut second, 10);

    assert_eq!(first.state().tick, LivingTickV2(10));
    assert_ne!(
        world_named(&first, "Home A").inventory.energy,
        energy_before
    );
    assert_ne!(world_named(&first, "Home A").inventory.ore, ore_before);
    assert!(dispatched);
    assert_eq!(first.state().shipments.as_slice().len(), 1);
    assert_eq!(
        first.state().digest().expect("digest"),
        second.state().digest().expect("digest")
    );

    step_to(&mut first, 50);
    step_to(&mut second, 50);
    let jobs: Vec<_> = first
        .state()
        .hull_jobs
        .as_slice()
        .iter()
        .filter(|job| job.definition.as_str() == "colony_ark")
        .collect();
    assert_eq!(jobs.len(), 1);
    assert_eq!(jobs[0].paid_alloy, 60);
    assert_eq!(jobs[0].accepted_tick, LivingTickV2(50));
    assert_eq!(jobs[0].completion_tick, LivingTickV2(250));
    assert_ne!(world_named(&first, "Home A").inventory.alloy, alloy_before);
    assert_eq!(
        first.state().digest().expect("digest"),
        second.state().digest().expect("digest")
    );
    assert!(first.fault().is_none());
}

#[test]
fn an_expansionist_ark_settles_an_unowned_world() {
    let mut first = settlement_galaxy();
    let mut second = settlement_galaxy();
    assert!(world_named(&first, "Open").owner.is_none());
    assert!(first.state().colonies.as_slice().len() == 1);
    step_to(&mut first, 311);
    step_to(&mut second, 311);
    let open = world_named(&first, "Open");
    assert!(open.owner.is_some());
    assert!(
        first
            .state()
            .colonies
            .as_slice()
            .iter()
            .any(|colony| colony.world == open.id && colony.owner == open.owner.unwrap())
    );
    assert_eq!(
        first.state().digest().expect("digest"),
        second.state().digest().expect("digest")
    );
    first.step().expect("settlement does not end the session");
    assert!(first.fault().is_none());
}

#[test]
fn a_short_foundry_blocks_without_consuming_and_advances_its_clock() {
    let catalog = living_core_pack_v2().expect("pack");
    let genesis = validate_living_genesis_manifest_v2(
        LivingGenesisManifestV2 {
            kind: LIVING_GENESIS_KIND_V2.to_owned(),
            format_version: LIVING_GENESIS_FORMAT_VERSION_V2,
            rules_version: LIVING_RULES_VERSION,
            state: LivingGalaxyStateV2::default(),
        },
        &catalog,
    )
    .expect("empty genesis");
    let mut authority = LivingGalaxyAuthorityV2::from_genesis(
        catalog,
        [0x53; 32],
        LivingGenesisGeneratorV2 {
            generator_id: LivingSlugV2::new("shortage").expect("slug"),
            generator_version: 1,
        },
        genesis,
    )
    .expect("authority");

    let cursor = authority.published_cursor();
    authority
        .apply_paused(LivingCommandEnvelopeV2 {
            expected_committed_revision: cursor.committed_revision,
            expected_tick: cursor.tick,
            expected_pending_sequence: cursor.pending_sequence,
            mode: LivingCommandModeV2::Paused,
            command: LivingCommandV2::CreatorBatch {
                operations: vec![
                    LivingCreatorOperationV2::CreateSystem {
                        local: 0,
                        name: LivingNameV2::new("Forge").expect("name"),
                    },
                    LivingCreatorOperationV2::CreateCivilization {
                        local: 1,
                        name: LivingNameV2::new("Mill").expect("name"),
                        policy: LivingPolicyV2(LivingSlugV2::new("industrialist").expect("slug")),
                        status: nyon_workshop_core::living::LivingCivilizationStatusV2::Active,
                    },
                    LivingCreatorOperationV2::CreateWorld {
                        local: 2,
                        system: LivingCommandTargetV2::Local { local: 0 },
                        archetype: LivingSlugV2::new("barren").expect("slug"),
                        name: LivingNameV2::new("Lean").expect("name"),
                        owner: Some(LivingCommandTargetV2::Local { local: 1 }),
                        inventory: LivingInventoryV2 {
                            energy: 3,
                            ore: 4,
                            alloy: 0,
                        },
                    },
                    LivingCreatorOperationV2::CreateFacility {
                        local: 3,
                        world: LivingCommandTargetV2::Local { local: 2 },
                        definition: LivingSlugV2::new("foundry").expect("slug"),
                        status: LivingFacilityStatusV2::Operational,
                        paid_alloy: 40,
                        hit_points: 0,
                        next_due: Some(LivingTickV2(1)),
                    },
                ],
            },
        })
        .expect("paused foundry");

    assert_eq!(authority.state().tick, LivingTickV2(0));
    let before = world_named(&authority, "Lean").inventory;
    authority.step().expect("shortage boundary");
    let after = world_named(&authority, "Lean").inventory;
    assert_eq!(after, before);
    let foundry = authority
        .state()
        .facilities
        .as_slice()
        .iter()
        .find(|facility| facility.definition.as_str() == "foundry")
        .expect("foundry");
    assert_eq!(foundry.next_due, Some(LivingTickV2(21)));
    assert!(foundry.blocked_reasons.as_slice().iter().any(|reason| {
        matches!(
            reason,
            LivingBlockedReasonV2::MissingInput {
                resource: LivingResourceV2::Energy,
                required: 4,
                available: 3,
            }
        )
    }));
    assert_eq!(authority.state().tick, LivingTickV2(1));
}

#[test]
fn builtin_boundary_runs_diplomacy_and_fleet_conflict_without_ending() {
    let mut first = builtin_living_sandbox_v2().expect("sandbox");
    let mut second = builtin_living_sandbox_v2().expect("sandbox");
    let mut saw_war = false;
    let mut saw_departure = false;
    let mut saw_combat = false;
    for _ in 0..120 {
        let receipt = first.step().expect("boundary");
        for event in &receipt.events {
            saw_war |= event.kind == LivingEventKindV2::WarDeclared;
            saw_departure |= event.kind == LivingEventKindV2::FleetDeparted;
            saw_combat |= event.kind == LivingEventKindV2::CombatRound;
        }
    }
    step_to(&mut second, 120);
    assert_eq!(first.state().tick, LivingTickV2(120));
    assert!(saw_war);
    assert!(saw_departure);
    assert!(saw_combat);
    assert!(!first.state().wars.as_slice().is_empty());
    assert_eq!(first.state().civilizations.as_slice().len(), 2);
    assert_eq!(
        first.state().digest().expect("digest"),
        second.state().digest().expect("digest")
    );
    first.step().expect("the session continues after conflict");
    assert_eq!(first.state().tick, LivingTickV2(121));
    assert!(first.fault().is_none());
}

#[test]
fn conquest_and_an_empty_galaxy_keep_stepping() {
    let mut authority = empty_authority();
    authority
        .apply_paused(paused(
            &authority,
            vec![
                LivingCreatorOperationV2::CreateSystem {
                    local: 0,
                    name: LivingNameV2::new("Ring").expect("name"),
                },
                LivingCreatorOperationV2::CreateCivilization {
                    local: 1,
                    name: LivingNameV2::new("Alder").expect("name"),
                    policy: policy("neutral"),
                    status: nyon_workshop_core::living::LivingCivilizationStatusV2::Active,
                },
                LivingCreatorOperationV2::CreateCivilization {
                    local: 2,
                    name: LivingNameV2::new("Birch").expect("name"),
                    policy: policy("neutral"),
                    status: nyon_workshop_core::living::LivingCivilizationStatusV2::Active,
                },
                LivingCreatorOperationV2::CreateWorld {
                    local: 3,
                    system: local(0),
                    archetype: LivingSlugV2::new("barren").expect("slug"),
                    name: LivingNameV2::new("Held").expect("name"),
                    owner: Some(local(1)),
                    inventory: stock(100, 100, 100),
                },
                LivingCreatorOperationV2::CreateWorld {
                    local: 4,
                    system: local(0),
                    archetype: LivingSlugV2::new("barren").expect("slug"),
                    name: LivingNameV2::new("Taken").expect("name"),
                    owner: Some(local(2)),
                    inventory: stock(100, 100, 100),
                },
                LivingCreatorOperationV2::CreateColony {
                    world: local(3),
                    owner: local(1),
                },
                LivingCreatorOperationV2::CreateColony {
                    world: local(4),
                    owner: local(2),
                },
                LivingCreatorOperationV2::CreateFleet {
                    local: 5,
                    owner: local(1),
                    world: local(4),
                },
                LivingCreatorOperationV2::CreateHull {
                    local: 6,
                    fleet: local(5),
                    definition: LivingSlugV2::new("escort").expect("slug"),
                    hit_points: 10,
                    return_credit: false,
                },
                LivingCreatorOperationV2::ForceWar {
                    participant_a: local(1),
                    participant_b: local(2),
                    declarer: local(1),
                    start_tick: LivingTickV2(0),
                    end_tick: LivingTickV2(1_200),
                },
            ],
        ))
        .expect("occupation setup");
    step_to(&mut authority, 100);
    let birch = authority
        .state()
        .civilizations
        .as_slice()
        .iter()
        .find(|civilization| civilization.name.as_str() == "Birch")
        .expect("birch");
    assert_eq!(
        birch.status,
        nyon_workshop_core::living::LivingCivilizationStatusV2::Dormant
    );
    assert_eq!(authority.state().civilizations.as_slice().len(), 2);
    authority.step().expect("conquest does not end the session");

    let mut empty = empty_authority();
    step_to(&mut empty, 3);
    assert!(empty.state().civilizations.as_slice().is_empty());
    assert_eq!(empty.state().tick, LivingTickV2(3));
    empty.step().expect("an empty galaxy keeps stepping");
}

#[test]
fn stale_and_invalid_commands_leave_the_boundary_unchanged() {
    let mut authority = builtin_living_sandbox_v2().expect("sandbox");
    let before = authority.state().clone();
    let revisions = authority.revisions().len();
    let cursor = authority.published_cursor();
    let stale = authority.submit(LivingCommandEnvelopeV2 {
        expected_committed_revision: cursor.committed_revision,
        expected_tick: LivingTickV2(7),
        expected_pending_sequence: cursor.pending_sequence,
        mode: LivingCommandModeV2::Running,
        command: LivingCommandV2::CreatorBatch {
            operations: vec![relation(
                before.civilizations.as_slice()[0].id,
                before.civilizations.as_slice()[1].id,
                -30,
            )],
        },
    });
    assert!(matches!(
        stale,
        Err(nyon_workshop_core::living::LivingCommandRejectionV2::StaleTick { .. })
    ));
    assert_eq!(authority.state(), &before);
    assert_eq!(authority.revisions().len(), revisions);
    assert_eq!(authority.published_cursor().pending_sequence, 0);

    let invalid = authority.apply_paused(paused(
        &authority,
        vec![relation(
            before.civilizations.as_slice()[0].id,
            before.civilizations.as_slice()[1].id,
            1_000,
        )],
    ));
    assert!(matches!(
        invalid,
        Err(nyon_workshop_core::living::LivingCommandRejectionV2::Projection(_))
    ));
    assert_eq!(authority.state(), &before);
    assert!(authority.revisions().is_empty());
    assert!(authority.fault().is_none());
}

#[test]
fn an_over_capacity_batch_is_rejected_without_mutation() {
    let mut authority = builtin_living_sandbox_v2().expect("sandbox");
    let before = authority.state().clone();
    let from = before.civilizations.as_slice()[0].id;
    let to = before.civilizations.as_slice()[1].id;
    let mut operations = Vec::with_capacity(1_025);
    for _ in 0..1_025 {
        operations.push(relation(from, to, -30));
    }
    let rejected = authority.apply_paused(paused(&authority, operations));
    assert!(matches!(
        rejected,
        Err(
            nyon_workshop_core::living::LivingCommandRejectionV2::Command(
                nyon_workshop_core::living::LivingCommandErrorV2::BatchTooLarge {
                    operations: 1_025,
                    limit: 1_024,
                }
            )
        )
    ));
    assert_eq!(authority.state(), &before);
    assert!(authority.revisions().is_empty());
    assert_eq!(authority.published_cursor().pending_sequence, 0);
    assert_eq!(authority.state().tick, LivingTickV2(0));
    assert!(authority.fault().is_none());
}

#[test]
fn two_same_owner_jobs_finishing_together_both_remain() {
    let mut first = empty_authority();
    let mut second = empty_authority();
    let operations = vec![
        LivingCreatorOperationV2::CreateSystem {
            local: 0,
            name: LivingNameV2::new("Yard").expect("name"),
        },
        LivingCreatorOperationV2::CreateCivilization {
            local: 1,
            name: LivingNameV2::new("Mason").expect("name"),
            policy: policy("neutral"),
            status: nyon_workshop_core::living::LivingCivilizationStatusV2::Active,
        },
        LivingCreatorOperationV2::CreateWorld {
            local: 2,
            system: local(0),
            archetype: LivingSlugV2::new("barren").expect("slug"),
            name: LivingNameV2::new("Slip").expect("name"),
            owner: Some(local(1)),
            inventory: stock(100, 100, 100),
        },
        LivingCreatorOperationV2::CreateWorld {
            local: 3,
            system: local(0),
            archetype: LivingSlugV2::new("barren").expect("slug"),
            name: LivingNameV2::new("Other Slip").expect("name"),
            owner: Some(local(1)),
            inventory: stock(100, 100, 100),
        },
        LivingCreatorOperationV2::CreateConstructionJob {
            local: 4,
            world: local(2),
            owner: local(1),
            definition: LivingSlugV2::new("solar_array").expect("slug"),
            accepted_tick: LivingTickV2(0),
            completion_tick: LivingTickV2(1),
            paid_alloy: 20,
        },
        LivingCreatorOperationV2::CreateConstructionJob {
            local: 5,
            world: local(3),
            owner: local(1),
            definition: LivingSlugV2::new("solar_array").expect("slug"),
            accepted_tick: LivingTickV2(0),
            completion_tick: LivingTickV2(1),
            paid_alloy: 20,
        },
        LivingCreatorOperationV2::CreateHullJob {
            local: 6,
            world: local(2),
            owner: local(1),
            definition: LivingSlugV2::new("escort").expect("slug"),
            accepted_tick: LivingTickV2(0),
            completion_tick: LivingTickV2(1),
            paid_alloy: 10,
            target_fleet: None,
        },
        LivingCreatorOperationV2::CreateHullJob {
            local: 7,
            world: local(3),
            owner: local(1),
            definition: LivingSlugV2::new("escort").expect("slug"),
            accepted_tick: LivingTickV2(0),
            completion_tick: LivingTickV2(1),
            paid_alloy: 10,
            target_fleet: None,
        },
    ];
    first
        .apply_paused(paused(&first, operations.clone()))
        .expect("paused jobs");
    second
        .apply_paused(paused(&second, operations))
        .expect("paused jobs");
    assert_eq!(first.state().facilities.as_slice().len(), 0);
    assert_eq!(first.state().fleets.as_slice().len(), 0);
    assert_eq!(first.state().construction_jobs.as_slice().len(), 2);
    assert_eq!(first.state().hull_jobs.as_slice().len(), 2);

    first.step().expect("completion boundary");
    second.step().expect("completion boundary");

    assert_eq!(first.state().tick, LivingTickV2(1));
    assert!(first.state().construction_jobs.as_slice().is_empty());
    assert!(first.state().hull_jobs.as_slice().is_empty());
    let facilities = first.state().facilities.as_slice();
    assert_eq!(facilities.len(), 2);
    assert_ne!(facilities[0].id, facilities[1].id);
    assert!(
        facilities
            .iter()
            .all(|facility| facility.definition.as_str() == "solar_array")
    );
    let fleets = first.state().fleets.as_slice();
    assert_eq!(fleets.len(), 2);
    assert_ne!(fleets[0].id, fleets[1].id);
    let hulls: Vec<_> = fleets
        .iter()
        .flat_map(|fleet| fleet.hulls.as_slice())
        .collect();
    assert_eq!(hulls.len(), 2);
    assert_ne!(hulls[0].id, hulls[1].id);
    assert!(
        hulls
            .iter()
            .all(|hull| hull.definition.as_str() == "escort")
    );
    assert_eq!(
        first.state().digest().expect("digest"),
        second.state().digest().expect("digest")
    );
    assert!(first.fault().is_none());
}

#[test]
fn fork_replays_recorded_inputs_and_keeps_the_parent_addressable() {
    let mut authority = builtin_living_sandbox_v2().expect("sandbox");
    let from = authority.state().civilizations.as_slice()[0].id;
    let to = authority.state().civilizations.as_slice()[1].id;
    authority
        .apply_paused(paused(&authority, vec![relation(from, to, -30)]))
        .expect("parent relation");
    let revision = authority.revisions()[0].id;
    step_to(&mut authority, 15);
    let parent_branch = authority.branch();
    let parent_digest = authority.state().digest().expect("parent digest");
    assert_eq!(authority.replay_digest().expect("replay"), parent_digest);

    let child = authority
        .fork_paused(revision, vec![relation(from, to, -80)])
        .expect("fork");
    assert_eq!(authority.revisions().last().unwrap().parent, Some(revision));
    assert_eq!(authority.revisions()[0].id, revision);
    let child_digest = authority.state().digest().expect("child digest");
    assert_ne!(child_digest, parent_digest);
    assert_eq!(authority.branch(), child);
    assert_eq!(
        authority
            .replay_archived_digest(parent_branch)
            .expect("archived replay"),
        parent_digest
    );
    assert_eq!(
        authority.replay_digest().expect("child replay"),
        child_digest
    );
    assert_eq!(authority.archived().len(), 1);
    assert_eq!(authority.archived()[0].branch(), parent_branch);
    assert_eq!(
        authority.archived()[0]
            .state()
            .digest()
            .expect("stored parent"),
        parent_digest
    );
}

fn settlement_galaxy() -> LivingGalaxyAuthorityV2 {
    let mut authority = empty_authority();
    authority
        .apply_paused(paused(
            &authority,
            vec![
                LivingCreatorOperationV2::CreateSystem {
                    local: 0,
                    name: LivingNameV2::new("Reach").expect("name"),
                },
                LivingCreatorOperationV2::CreateCivilization {
                    local: 1,
                    name: LivingNameV2::new("Planter").expect("name"),
                    policy: policy("expansionist"),
                    status: nyon_workshop_core::living::LivingCivilizationStatusV2::Active,
                },
                LivingCreatorOperationV2::CreateWorld {
                    local: 2,
                    system: local(0),
                    archetype: LivingSlugV2::new("temperate").expect("slug"),
                    name: LivingNameV2::new("Home").expect("name"),
                    owner: Some(local(1)),
                    inventory: stock(100, 100, 120),
                },
                LivingCreatorOperationV2::CreateWorld {
                    local: 3,
                    system: local(0),
                    archetype: LivingSlugV2::new("barren").expect("slug"),
                    name: LivingNameV2::new("Open").expect("name"),
                    owner: None,
                    inventory: stock(0, 0, 0),
                },
                LivingCreatorOperationV2::CreateColony {
                    world: local(2),
                    owner: local(1),
                },
                LivingCreatorOperationV2::CreateFacility {
                    local: 4,
                    world: local(2),
                    definition: LivingSlugV2::new("shipyard").expect("slug"),
                    status: LivingFacilityStatusV2::Operational,
                    paid_alloy: 0,
                    hit_points: 0,
                    next_due: None,
                },
            ],
        ))
        .expect("paused settlement galaxy");
    authority
}

fn empty_authority() -> LivingGalaxyAuthorityV2 {
    let catalog = living_core_pack_v2().expect("pack");
    let genesis = validate_living_genesis_manifest_v2(
        LivingGenesisManifestV2 {
            kind: LIVING_GENESIS_KIND_V2.to_owned(),
            format_version: LIVING_GENESIS_FORMAT_VERSION_V2,
            rules_version: LIVING_RULES_VERSION,
            state: LivingGalaxyStateV2::default(),
        },
        &catalog,
    )
    .expect("empty genesis");
    LivingGalaxyAuthorityV2::from_genesis(
        catalog,
        [0x45; 32],
        LivingGenesisGeneratorV2 {
            generator_id: LivingSlugV2::new("play").expect("slug"),
            generator_version: 1,
        },
        genesis,
    )
    .expect("authority")
}

fn local(local: u16) -> LivingCommandTargetV2 {
    LivingCommandTargetV2::Local { local }
}

fn policy(id: &str) -> LivingPolicyV2 {
    LivingPolicyV2(LivingSlugV2::new(id).expect("policy"))
}

fn stock(energy: u32, ore: u32, alloy: u32) -> LivingInventoryV2 {
    LivingInventoryV2 { energy, ore, alloy }
}

fn relation(
    from: nyon_workshop_core::living::LivingEntityIdV2,
    to: nyon_workshop_core::living::LivingEntityIdV2,
    base: i32,
) -> LivingCreatorOperationV2 {
    LivingCreatorOperationV2::SetRelationBase {
        from: LivingCommandTargetV2::Existing { id: from },
        to: LivingCommandTargetV2::Existing { id: to },
        base_disposition: base,
    }
}

fn paused(
    authority: &LivingGalaxyAuthorityV2,
    operations: Vec<LivingCreatorOperationV2>,
) -> LivingCommandEnvelopeV2 {
    let cursor = authority.published_cursor();
    LivingCommandEnvelopeV2 {
        expected_committed_revision: cursor.committed_revision,
        expected_tick: cursor.tick,
        expected_pending_sequence: cursor.pending_sequence,
        mode: LivingCommandModeV2::Paused,
        command: LivingCommandV2::CreatorBatch { operations },
    }
}

fn running(
    authority: &LivingGalaxyAuthorityV2,
    operations: Vec<LivingCreatorOperationV2>,
) -> LivingCommandEnvelopeV2 {
    let cursor = authority.published_cursor();
    LivingCommandEnvelopeV2 {
        expected_committed_revision: cursor.committed_revision,
        expected_tick: cursor.tick,
        expected_pending_sequence: cursor.pending_sequence,
        mode: LivingCommandModeV2::Running,
        command: LivingCommandV2::CreatorBatch { operations },
    }
}

#[test]
fn paused_nonzero_edits_and_repeated_forks_replay_exactly() {
    let mut authority = builtin_living_sandbox_v2().unwrap();
    let from = authority.state().civilizations.as_slice()[0].id;
    let to = authority.state().civilizations.as_slice()[1].id;
    step_to(&mut authority, 11);
    authority
        .apply_paused(paused(&authority, vec![relation(from, to, -30)]))
        .unwrap();
    let root = authority.branch();
    let root_digest = authority.state().digest().unwrap();
    assert_eq!(authority.replay_digest().unwrap(), root_digest);
    let revision = authority.revisions()[0].id;
    authority
        .fork_paused(revision, vec![relation(from, to, -80)])
        .unwrap();
    let first = authority.branch();
    let first_digest = authority.state().digest().unwrap();
    assert_eq!(authority.replay_digest().unwrap(), first_digest);
    let revision = authority.revisions()[0].id;
    authority
        .fork_paused(revision, vec![relation(from, to, -60)])
        .unwrap();
    assert_eq!(authority.archived().len(), 2);
    assert_eq!(authority.replay_archived_digest(root).unwrap(), root_digest);
    assert_eq!(
        authority.replay_archived_digest(first).unwrap(),
        first_digest
    );
    assert_eq!(
        authority.replay_digest().unwrap(),
        authority.state().digest().unwrap()
    );
}

#[test]
fn foreign_origin_credits_allow_only_a_return_and_are_consumed() {
    use nyon_workshop_core::living::{LivingFleetLocationV2, LivingFleetOrderKindV2};
    let mut authority = builtin_living_sandbox_v2().unwrap();
    let home = world_named(&authority, "Home A").id;
    let foreign = world_named(&authority, "Home B").id;
    let owner = authority
        .state()
        .colonies
        .as_slice()
        .iter()
        .find(|colony| colony.world == home)
        .unwrap()
        .owner;
    let existing_fleets: Vec<_> = authority
        .state()
        .fleets
        .as_slice()
        .iter()
        .map(|fleet| fleet.id)
        .collect();
    authority
        .apply_paused(paused(
            &authority,
            vec![
                LivingCreatorOperationV2::CreateFleet {
                    local: 0,
                    owner: LivingCommandTargetV2::Existing { id: owner },
                    world: LivingCommandTargetV2::Existing { id: foreign },
                },
                LivingCreatorOperationV2::CreateHull {
                    local: 1,
                    fleet: local(0),
                    definition: LivingSlugV2::new("scout").unwrap(),
                    hit_points: 10,
                    return_credit: true,
                },
                LivingCreatorOperationV2::SetFleetOrder {
                    fleet: local(0),
                    kind: Some(LivingFleetOrderKindV2::Move),
                    target_world: Some(LivingCommandTargetV2::Existing { id: home }),
                    not_before_tick: LivingTickV2(1),
                    lane_path: vec![],
                },
            ],
        ))
        .unwrap();
    let fleet = authority.state().fleets.as_slice().iter().find(|fleet| !existing_fleets.contains(&fleet.id) && fleet.owner == owner && matches!(fleet.location, LivingFleetLocationV2::Docked { world } if world == foreign)).unwrap().id;
    authority.step().unwrap();
    let row = authority
        .state()
        .fleets
        .as_slice()
        .iter()
        .find(|row| row.id == fleet)
        .unwrap();
    assert_eq!(
        row.location,
        LivingFleetLocationV2::Docked { world: foreign }
    );
    assert!(row.hulls.as_slice()[0].return_credit);
    authority
        .apply_paused(paused(
            &authority,
            vec![LivingCreatorOperationV2::SetFleetOrder {
                fleet: LivingCommandTargetV2::Existing { id: fleet },
                kind: Some(LivingFleetOrderKindV2::Return),
                target_world: Some(LivingCommandTargetV2::Existing { id: home }),
                not_before_tick: LivingTickV2(2),
                lane_path: vec![],
            }],
        ))
        .unwrap();
    authority.step().unwrap();
    let row = authority
        .state()
        .fleets
        .as_slice()
        .iter()
        .find(|row| row.id == fleet)
        .unwrap();
    assert!(matches!(
        row.location,
        LivingFleetLocationV2::Travelling { .. }
    ));
    assert!(!row.hulls.as_slice()[0].return_credit);
}

#[test]
fn running_and_paused_edits_at_one_boundary_replay_in_phase_order() {
    let mut authority = builtin_living_sandbox_v2().unwrap();
    let from = authority.state().civilizations.as_slice()[0].id;
    let to = authority.state().civilizations.as_slice()[1].id;
    step_to(&mut authority, 9);
    authority
        .submit(running(&authority, vec![relation(from, to, -30)]))
        .unwrap();
    authority.step().unwrap();
    authority
        .apply_paused(paused(&authority, vec![relation(from, to, -60)]))
        .unwrap();
    assert_eq!(
        authority.replay_digest().unwrap(),
        authority.state().digest().unwrap()
    );
}

#[test]
fn branch_capacity_rejects_without_spending_history_or_identity() {
    let mut authority = builtin_living_sandbox_v2().unwrap();
    let from = authority.state().civilizations.as_slice()[0].id;
    let to = authority.state().civilizations.as_slice()[1].id;
    authority
        .apply_paused(paused(&authority, vec![relation(from, to, -30)]))
        .unwrap();
    for _ in 0..63 {
        let revision = authority.revisions()[0].id;
        authority
            .fork_paused(revision, vec![relation(from, to, -30)])
            .unwrap();
    }
    let before = authority.state().clone();
    let cursor = authority.published_cursor();
    let high_water = authority.accepted_sequence_high_water();
    let revision = authority.revisions()[0].id;
    assert!(matches!(
        authority.fork_paused(revision, vec![relation(from, to, -30)]),
        Err(
            nyon_workshop_core::living::LivingCommandRejectionV2::HistoryFull {
                collection: "branches",
                limit: 64
            }
        )
    ));
    assert_eq!(authority.state(), &before);
    assert_eq!(authority.published_cursor(), cursor);
    assert_eq!(authority.accepted_sequence_high_water(), high_water);
    assert_eq!(authority.archived().len(), 63);
}

#[test]
fn a_nonzero_fork_preserves_autonomous_parent_prefix_identities() {
    let mut authority = builtin_living_sandbox_v2().unwrap();
    step_to(&mut authority, 60);
    let from = authority.state().civilizations.as_slice()[0].id;
    let to = authority.state().civilizations.as_slice()[1].id;
    authority
        .apply_paused(paused(&authority, vec![relation(from, to, -30)]))
        .unwrap();
    let jobs = authority.state().hull_jobs.clone();
    let fleets = authority.state().fleets.clone();
    let revision = authority.revisions()[0].id;
    authority
        .fork_paused(revision, vec![relation(from, to, -60)])
        .unwrap();
    assert_eq!(authority.state().hull_jobs, jobs);
    assert_eq!(authority.state().fleets, fleets);
    assert_eq!(
        authority.replay_digest().unwrap(),
        authority.state().digest().unwrap()
    );
    let revision = authority.revisions()[0].id;
    authority
        .fork_paused(revision, vec![relation(from, to, -80)])
        .unwrap();
    assert_eq!(authority.state().hull_jobs, jobs);
    assert_eq!(
        authority.replay_digest().unwrap(),
        authority.state().digest().unwrap()
    );
}

#[test]
fn first_expansion_has_the_approved_autonomous_witness_boundaries() {
    use nyon_workshop_core::living::{
        LivingFleetLocationV2, LivingFleetOrderKindV2, builtin_first_expansion_v2,
    };
    let mut authority = builtin_first_expansion_v2().unwrap();
    let home = world_named(&authority, "Founding World").id;
    let frontier = world_named(&authority, "Frontier World").id;
    step_to(&mut authority, 49);
    assert!(authority.state().construction_jobs.as_slice().is_empty());
    assert!(authority.state().hull_jobs.as_slice().is_empty());
    step_to(&mut authority, 50);
    let foundry = authority
        .state()
        .construction_jobs
        .as_slice()
        .iter()
        .find(|job| job.definition.as_str() == "foundry")
        .unwrap();
    assert_eq!(
        (foundry.accepted_tick, foundry.completion_tick),
        (LivingTickV2(50), LivingTickV2(250))
    );
    let scout = authority
        .state()
        .fleets
        .as_slice()
        .iter()
        .find(|fleet| {
            fleet
                .hulls
                .as_slice()
                .iter()
                .any(|hull| hull.definition.as_str() == "scout")
        })
        .unwrap();
    assert_eq!(scout.hulls.as_slice().len(), 1);
    assert_eq!(
        scout.order.as_ref().unwrap().kind,
        LivingFleetOrderKindV2::Explore
    );
    let scout_id = scout.id;
    authority.step().unwrap();
    assert!(matches!(
        authority
            .state()
            .fleets
            .as_slice()
            .iter()
            .find(|fleet| fleet.id == scout_id)
            .unwrap()
            .location,
        LivingFleetLocationV2::Travelling { .. }
    ));
    step_to(&mut authority, 61);
    assert_eq!(
        authority
            .state()
            .fleets
            .as_slice()
            .iter()
            .find(|fleet| fleet.id == scout_id)
            .unwrap()
            .location,
        LivingFleetLocationV2::Docked { world: frontier }
    );
    assert!(
        authority
            .state()
            .observations
            .as_slice()
            .iter()
            .any(|observation| observation.world == frontier
                && observation.observed_tick == LivingTickV2(61))
    );
    step_to(&mut authority, 100);
    let ark = authority
        .state()
        .hull_jobs
        .as_slice()
        .iter()
        .find(|job| job.definition.as_str() == "colony_ark")
        .unwrap();
    assert_eq!(
        (ark.accepted_tick, ark.completion_tick),
        (LivingTickV2(100), LivingTickV2(300))
    );
    step_to(&mut authority, 250);
    assert!(
        authority
            .state()
            .facilities
            .as_slice()
            .iter()
            .any(|facility| facility.world == home && facility.definition.as_str() == "foundry")
    );
    step_to(&mut authority, 300);
    let ark = authority
        .state()
        .fleets
        .as_slice()
        .iter()
        .find(|fleet| {
            fleet
                .hulls
                .as_slice()
                .iter()
                .any(|hull| hull.definition.as_str() == "colony_ark")
        })
        .unwrap();
    assert_eq!(
        ark.order.as_ref().unwrap().kind,
        LivingFleetOrderKindV2::Settle
    );
    let ark_id = ark.id;
    authority.step().unwrap();
    assert!(matches!(
        authority
            .state()
            .fleets
            .as_slice()
            .iter()
            .find(|fleet| fleet.id == ark_id)
            .unwrap()
            .location,
        LivingFleetLocationV2::Travelling { .. }
    ));
    step_to(&mut authority, 311);
    assert_eq!(
        authority
            .state()
            .fleets
            .as_slice()
            .iter()
            .find(|fleet| fleet.id == ark_id)
            .unwrap()
            .location,
        LivingFleetLocationV2::Docked { world: frontier }
    );
    assert!(
        authority
            .state()
            .settlement_claims
            .as_slice()
            .iter()
            .any(|claim| claim.world == frontier)
    );
    step_to(&mut authority, 361);
    assert!(world_named(&authority, "Frontier World").owner.is_some());
    assert_eq!(
        authority.replay_digest().unwrap(),
        authority.state().digest().unwrap()
    );
}

#[test]
fn bilateral_trade_forms_by_mutual_eligibility_and_renews_end_exclusively() {
    use nyon_workshop_core::living::LivingAgreementKindV2;
    let mut authority = builtin_living_sandbox_v2().unwrap();
    let a = authority.state().civilizations.as_slice()[0].id;
    let b = authority.state().civilizations.as_slice()[1].id;
    authority
        .apply_paused(paused(
            &authority,
            vec![relation(a, b, 10), relation(b, a, 10)],
        ))
        .unwrap();
    step_to(&mut authority, 99);
    assert!(authority.state().agreements.as_slice().is_empty());
    let receipt = authority.step().unwrap();
    assert!(
        receipt
            .events
            .iter()
            .any(|event| event.kind == LivingEventKindV2::AgreementFormed)
    );
    let decisions = authority.last_decision().unwrap();
    decisions.verify_receipt(&receipt).unwrap();
    let agreements: Vec<_> = decisions
        .intents
        .iter()
        .filter(|intent| {
            matches!(
                intent.action,
                nyon_workshop_core::living::LivingIntentActionV2::Agreement { .. }
            )
        })
        .collect();
    assert_eq!(agreements.len(), 2);
    assert!(
        agreements
            .iter()
            .all(|intent| intent.observed_tick.is_none()
                && intent.outcome == nyon_workshop_core::living::LivingDecisionOutcomeV2::Accepted
                && intent.reasons.iter().any(|reason| matches!(
                    reason,
                    nyon_workshop_core::living::LivingDecisionReasonV2::BilateralThreshold {
                        forward: 10,
                        reverse: 10,
                        required: 0,
                        ..
                    }
                )))
    );
    let trade = authority
        .state()
        .agreements
        .as_slice()
        .iter()
        .find(|agreement| agreement.kind == LivingAgreementKindV2::Trade)
        .unwrap();
    assert_eq!(
        (trade.start_tick, trade.end_tick),
        (LivingTickV2(100), LivingTickV2(1_300))
    );
    assert!(
        !authority
            .state()
            .agreements
            .as_slice()
            .iter()
            .any(|agreement| agreement.kind == LivingAgreementKindV2::Nonaggression)
    );
    step_to(&mut authority, 1_299);
    let receipt = authority.step().unwrap();
    assert!(
        receipt
            .events
            .iter()
            .any(|event| event.kind == LivingEventKindV2::AgreementExpired)
    );
    assert!(
        receipt
            .events
            .iter()
            .any(|event| event.kind == LivingEventKindV2::AgreementFormed)
    );
    let trade = authority
        .state()
        .agreements
        .as_slice()
        .iter()
        .find(|agreement| agreement.kind == LivingAgreementKindV2::Trade)
        .unwrap();
    assert_eq!(
        (trade.start_tick, trade.end_tick),
        (LivingTickV2(1_300), LivingTickV2(2_500))
    );
    assert!(authority.state().wars.as_slice().is_empty());
    assert_eq!(
        authority.replay_digest().unwrap(),
        authority.state().digest().unwrap()
    );
}
#[test]
fn war_proposals_share_snapshot_even_when_an_earlier_declaration_changes_relations() {
    use nyon_workshop_core::living::{LivingCivilizationStatusV2, LivingCommandTargetV2};
    let mut authority = builtin_living_sandbox_v2().unwrap();
    let a = authority.state().civilizations.as_slice()[0].id;
    let b = authority.state().civilizations.as_slice()[1].id;
    let system = authority.state().systems.as_slice()[0].id;
    authority
        .apply_paused(paused(
            &authority,
            vec![
                LivingCreatorOperationV2::CreateCivilization {
                    local: 0,
                    name: LivingNameV2::new("Cedar").unwrap(),
                    policy: policy("neutral"),
                    status: LivingCivilizationStatusV2::Active,
                },
                LivingCreatorOperationV2::CreateWorld {
                    local: 1,
                    system: LivingCommandTargetV2::Existing { id: system },
                    archetype: LivingSlugV2::new("barren").unwrap(),
                    name: LivingNameV2::new("Cedar Home").unwrap(),
                    owner: Some(local(0)),
                    inventory: stock(100, 100, 100),
                },
                LivingCreatorOperationV2::CreateColony {
                    world: local(1),
                    owner: local(0),
                },
            ],
        ))
        .unwrap();
    let c = authority
        .state()
        .civilizations
        .as_slice()
        .iter()
        .find(|row| row.name.as_str() == "Cedar")
        .unwrap()
        .id;
    authority
        .apply_paused(paused(
            &authority,
            vec![
                relation(a, b, -40),
                relation(b, a, -10),
                relation(b, c, -30),
            ],
        ))
        .unwrap();
    step_to(&mut authority, 100);
    let has_pair = |left, right| {
        authority.state().wars.as_slice().iter().any(|war| {
            (war.participant_a == left && war.participant_b == right)
                || (war.participant_a == right && war.participant_b == left)
        })
    };
    assert!(has_pair(a, b));
    assert!(
        has_pair(b, c),
        "B must retain its snapshot choice C even after A declares against B"
    );
    assert_eq!(authority.state().wars.as_slice().len(), 2);
}

#[test]
fn real_ranked_intents_seal_actual_attempt_explanations_replayably() {
    use nyon_workshop_core::living::{
        LivingDecisionOutcomeV2, LivingIntentActionV2, builtin_first_expansion_v2,
        decode_living_archive_v2, encode_living_archive_v2,
    };
    let mut authority = builtin_first_expansion_v2().unwrap();
    step_to(&mut authority, 50);
    let decision = authority
        .last_decision()
        .expect("running boundary emits paired sidecar");
    decision
        .verify_receipt(authority.last_receipt().unwrap())
        .unwrap();
    let foundry = decision.intents.iter().find(|intent| matches!(&intent.action, LivingIntentActionV2::Construction { definition, .. } if definition.as_str() == "foundry")).unwrap();
    assert_eq!(foundry.outcome, LivingDecisionOutcomeV2::Accepted);
    assert_eq!(foundry.score.0, 50);
    assert!(
        decision
            .intents
            .iter()
            .all(|intent| intent.outcome == LivingDecisionOutcomeV2::Accepted
                && !intent.reasons.is_empty())
    );
    assert!(decision.intents.iter().any(|intent| matches!(
        intent.action,
        LivingIntentActionV2::Explore { .. }
    ) && intent.outcome
        == LivingDecisionOutcomeV2::Accepted));
    let canonical = decision.canonical_bytes().unwrap();
    let archive = encode_living_archive_v2(&authority).unwrap();
    let decoded = decode_living_archive_v2(&archive, authority.catalog()).unwrap();
    assert_eq!(
        decoded
            .authority()
            .last_decision()
            .unwrap()
            .canonical_bytes()
            .unwrap(),
        canonical,
        "portable replay reconstructs authoritative explanations without exporting caches"
    );
}

#[test]
fn null_cursor_forks_preserve_autonomous_prefix_and_retain_siblings() {
    for tick in [0, 61] {
        let mut authority = builtin_living_sandbox_v2().unwrap();
        step_to(&mut authority, tick);
        let parent = authority.branch();
        let parent_digest = authority.state().digest().unwrap();
        let worlds = authority.state().worlds.clone();
        let fleets = authority.state().fleets.clone();
        let sequence = authority.accepted_sequence_high_water();
        let child = authority.fork_view_paused(Vec::new()).unwrap();
        assert_ne!(child, parent);
        assert_eq!(authority.state().worlds, worlds);
        assert_eq!(authority.state().fleets, fleets);
        assert_eq!(authority.accepted_sequence_high_water(), sequence);
        assert!(authority.revisions().is_empty());
        assert_eq!(
            authority.replay_archived_digest(parent).unwrap(),
            parent_digest
        );
        assert_eq!(
            authority.replay_digest().unwrap(),
            authority.state().digest().unwrap()
        );
        let sibling = authority.fork_view_paused(Vec::new()).unwrap();
        assert_ne!(sibling, child);
        assert_eq!(authority.archived().len(), 2);
        assert_eq!(
            authority.replay_archived_digest(parent).unwrap(),
            parent_digest
        );
        assert_eq!(
            authority.replay_digest().unwrap(),
            authority.state().digest().unwrap()
        );
    }
}

#[test]
fn null_cursor_branch_capacity_is_atomic_without_allocating_revisions() {
    let mut authority = builtin_living_sandbox_v2().unwrap();
    for _ in 0..63 {
        authority.fork_view_paused(Vec::new()).unwrap();
    }
    let state = authority.state().clone();
    let branch = authority.branch();
    let high_water = authority.branch_sequence_high_water();
    assert!(matches!(
        authority.fork_view_paused(Vec::new()),
        Err(
            nyon_workshop_core::living::LivingCommandRejectionV2::HistoryFull {
                collection: "branches",
                limit: 64
            }
        )
    ));
    assert_eq!(authority.state(), &state);
    assert_eq!(authority.branch(), branch);
    assert_eq!(authority.branch_sequence_high_water(), high_water);
    assert_eq!(authority.accepted_sequence_high_water(), 0);
    assert!(authority.revisions().is_empty());
}

#[test]
fn fork_view_preserves_autonomous_history_after_a_nonnull_cursor() {
    let mut authority = builtin_living_sandbox_v2().unwrap();
    let a = authority.state().civilizations.as_slice()[0].id;
    let b = authority.state().civilizations.as_slice()[1].id;
    authority
        .apply_paused(paused(&authority, vec![relation(a, b, 10)]))
        .unwrap();
    step_to(&mut authority, 61);
    let before = authority.state().clone();
    let revision = authority.published_cursor().committed_revision;
    authority.fork_view_paused(Vec::new()).unwrap();
    assert_eq!(authority.state().tick, before.tick);
    assert_eq!(
        authority.state().construction_jobs,
        before.construction_jobs
    );
    assert_eq!(authority.state().hull_jobs, before.hull_jobs);
    assert_eq!(authority.published_cursor().committed_revision, revision);
    assert_eq!(
        authority.replay_digest().unwrap(),
        authority.state().digest().unwrap()
    );
}
