mod common;

use nyon_workshop_core::{
    BatchLocalId, CatalogId, CreatorBatchV1, CreatorOpV1, CreatorRejectionV1, EntityId,
    GalaxyPointV1, ObjectName, ObjectRefV1, RevisionId, StateDigest, WorkshopAuthority,
    WorkshopHistory, WorkshopTick,
    command::{MAX_BATCH_BYTES, MAX_BATCH_OPERATIONS},
    ids::GALAXY_COORDINATE_LIMIT,
    model::{
        EntityKind, MAX_DEPOSITS, MAX_FACTIONS, MAX_HAZARDS, MAX_INDUSTRIES, MAX_LANES, MAX_ROUTES,
        MAX_STARS, MAX_SYSTEMS, MAX_WORLDS,
    },
};

#[test]
fn identical_inputs_produce_identical_revision_entities_and_digest() {
    let pack = common::pack();
    let mut left = WorkshopAuthority::from_seed_u64(pack.clone(), 7);
    let mut right = WorkshopAuthority::from_seed_u64(pack, 7);
    let batch = CreatorBatchV1 {
        expected_cursor: None,
        expected_tick: WorkshopTick(0),
        operations: vec![CreatorOpV1::CreateSystem {
            local: common::local(1),
            name: common::name("Sol"),
            position: GalaxyPointV1::new(0, 0).unwrap(),
        }],
    };
    let left_receipt = left.submit(batch.clone()).unwrap();
    let right_receipt = right.submit(batch).unwrap();
    assert_eq!(left_receipt, right_receipt);
    assert_eq!(
        left.state().canonical_bytes(),
        right.state().canonical_bytes()
    );
}

#[test]
fn backward_local_references_are_required_and_kind_checked() {
    let mut authority = WorkshopAuthority::from_seed_u64(common::pack(), 9);
    let before = authority.encode_authority_for_test();
    let rejection = authority.submit(CreatorBatchV1 {
        expected_cursor: None,
        expected_tick: WorkshopTick(0),
        operations: vec![
            CreatorOpV1::CreateStar {
                local: common::local(0),
                system: common::local_ref(1),
                name: common::name("Early Star"),
                archetype_id: common::catalog("yellow-dwarf"),
            },
            CreatorOpV1::CreateSystem {
                local: common::local(1),
                name: common::name("Late System"),
                position: GalaxyPointV1::new(0, 0).unwrap(),
            },
        ],
    });
    assert_eq!(rejection, Err(CreatorRejectionV1::InvalidLocalReference));
    assert_eq!(authority.encode_authority_for_test(), before);
}

#[test]
fn duplicate_local_and_stale_expectations_are_atomic() {
    let mut authority = WorkshopAuthority::from_seed_u64(common::pack(), 11);
    let before = authority.encode_authority_for_test();
    let duplicate = authority.submit(CreatorBatchV1 {
        expected_cursor: None,
        expected_tick: WorkshopTick(0),
        operations: vec![
            CreatorOpV1::CreateSystem {
                local: common::local(0),
                name: common::name("First"),
                position: GalaxyPointV1::new(0, 0).unwrap(),
            },
            CreatorOpV1::CreateSystem {
                local: common::local(0),
                name: common::name("Second"),
                position: GalaxyPointV1::new(1, 0).unwrap(),
            },
        ],
    });
    assert_eq!(duplicate, Err(CreatorRejectionV1::DuplicateLocalId));
    assert_eq!(authority.encode_authority_for_test(), before);

    assert_eq!(
        authority.submit(CreatorBatchV1 {
            expected_cursor: None,
            expected_tick: WorkshopTick(1),
            operations: Vec::new(),
        }),
        Err(CreatorRejectionV1::StaleTick)
    );
    assert_eq!(authority.encode_authority_for_test(), before);
}

#[test]
fn non_cascading_remove_reports_every_current_dependency() {
    let mut history = WorkshopHistory::from_seed_u64(common::pack(), 13);
    let forge = common::create_forge(&mut history);
    let before = history.state().canonical_bytes();
    let result = history.submit(CreatorBatchV1 {
        expected_cursor: history.active_revision(),
        expected_tick: history.state().tick,
        operations: vec![CreatorOpV1::RemoveObject {
            target: ObjectRefV1::Existing(forge.system_a),
        }],
    });
    match result {
        Err(nyon_workshop_core::HistoryError::Creator(
            CreatorRejectionV1::BlockingDependencies { entities },
        )) => {
            assert!(entities.contains(&forge.star_a));
            assert!(entities.contains(&forge.world_a));
            assert!(entities.contains(&forge.lane));
            assert!(entities.windows(2).all(|pair| pair[0] < pair[1]));
        }
        other => panic!("unexpected result: {other:?}"),
    }
    assert_eq!(history.state().canonical_bytes(), before);
    assert_eq!(history.revision_count(), 1);
}

#[test]
fn hazard_capacity_counts_only_scheduled_or_active_hazards() {
    let mut history = WorkshopHistory::from_seed_u64(common::pack(), 17);
    let forge = common::create_forge(&mut history);
    let operations = (0..MAX_HAZARDS)
        .map(|index| CreatorOpV1::ScheduleHazard {
            local: common::local(u16::try_from(index).unwrap()),
            lane: common::existing(forge.lane),
            hazard_id: common::catalog("ion-storm"),
            start_tick: WorkshopTick(0),
            duration_ticks: 1,
        })
        .collect();
    history
        .submit(CreatorBatchV1 {
            expected_cursor: history.active_revision(),
            expected_tick: history.state().tick,
            operations,
        })
        .unwrap();

    let at_capacity = history.submit(CreatorBatchV1 {
        expected_cursor: history.active_revision(),
        expected_tick: history.state().tick,
        operations: vec![CreatorOpV1::ScheduleHazard {
            local: common::local(200),
            lane: common::existing(forge.lane),
            hazard_id: common::catalog("ion-storm"),
            start_tick: WorkshopTick(0),
            duration_ticks: 1,
        }],
    });
    assert_eq!(
        at_capacity,
        Err(nyon_workshop_core::HistoryError::Creator(
            CreatorRejectionV1::Capacity {
                kind: nyon_workshop_core::model::EntityKind::Hazard,
            },
        ))
    );

    history.step().unwrap();
    history
        .submit(CreatorBatchV1 {
            expected_cursor: history.active_revision(),
            expected_tick: history.state().tick,
            operations: vec![CreatorOpV1::ScheduleHazard {
                local: common::local(201),
                lane: common::existing(forge.lane),
                hazard_id: common::catalog("ion-storm"),
                start_tick: WorkshopTick(1),
                duration_ticks: 1,
            }],
        })
        .unwrap();
    assert_eq!(history.state().hazards.len(), MAX_HAZARDS + 1);
}

#[test]
fn genesis_and_first_creation_identifiers_and_digests_match_frozen_goldens() {
    let pack = common::pack();
    let empty = WorkshopHistory::from_seed_u64(pack.clone(), 0x4652_4745);
    assert_eq!(
        empty.state().canonical_bytes(),
        br#"{"tick":0,"factions":{},"systems":{},"stars":{},"worlds":{},"lanes":{},"deposits":{},"industries":{},"routes":{},"shipments":{},"hazards":{},"owners":{}}"#
    );
    assert_eq!(
        empty.active_state_digest(),
        StateDigest([
            206, 75, 30, 39, 46, 141, 19, 137, 28, 79, 142, 241, 117, 199, 155, 180, 106, 232, 251,
            94, 188, 210, 50, 65, 112, 26, 250, 183, 236, 101, 79, 13,
        ])
    );
    assert_eq!(
        empty.active_view().selected_branch.0,
        [
            179, 154, 124, 160, 224, 209, 211, 97, 203, 104, 239, 95, 134, 79, 209, 166,
        ]
    );

    let alternate = WorkshopHistory::from_seed_u64(pack.clone(), 0x0bad_f00d);
    assert_eq!(
        alternate.active_view().selected_branch.0,
        [
            20, 151, 246, 246, 36, 104, 125, 253, 234, 122, 52, 132, 152, 40, 30, 210,
        ]
    );

    let mut authority = WorkshopAuthority::from_seed_u64(pack.clone(), 7);
    let receipt = authority
        .submit(CreatorBatchV1 {
            expected_cursor: None,
            expected_tick: WorkshopTick(0),
            operations: vec![CreatorOpV1::CreateSystem {
                local: common::local(1),
                name: common::name("Sol"),
                position: GalaxyPointV1::new(0, 0).unwrap(),
            }],
        })
        .unwrap();
    assert_eq!(
        receipt.revision,
        RevisionId([
            26, 152, 145, 212, 189, 89, 140, 115, 247, 77, 217, 155, 86, 73, 157, 152, 41, 90, 196,
            198, 131, 190, 183, 112, 147, 0, 29, 90, 130, 218, 188, 22,
        ])
    );
    assert_eq!(
        receipt.entities[&common::local(1)],
        EntityId([
            137, 141, 98, 139, 50, 247, 21, 155, 211, 27, 156, 149, 101, 93, 69, 121,
        ])
    );
    assert_eq!(
        receipt.post_batch_digest,
        StateDigest([
            111, 174, 182, 124, 219, 168, 174, 240, 220, 73, 58, 82, 163, 143, 110, 86, 148, 5,
            204, 164, 227, 219, 118, 26, 111, 204, 136, 12, 142, 124, 137, 122,
        ])
    );

    let mut alternate_authority = WorkshopAuthority::from_seed_u64(pack, 0x0bad_f00d);
    let alternate_receipt = alternate_authority
        .submit(CreatorBatchV1 {
            expected_cursor: None,
            expected_tick: WorkshopTick(0),
            operations: vec![CreatorOpV1::CreateSystem {
                local: common::local(1),
                name: common::name("Sol"),
                position: GalaxyPointV1::new(0, 0).unwrap(),
            }],
        })
        .unwrap();
    assert_eq!(
        alternate_receipt.revision,
        RevisionId([
            59, 189, 75, 236, 112, 215, 19, 246, 254, 219, 234, 212, 206, 250, 153, 82, 64, 211,
            72, 37, 34, 115, 185, 240, 222, 254, 75, 137, 126, 212, 255, 149,
        ])
    );
    assert_eq!(
        alternate_receipt.entities[&common::local(1)],
        EntityId([
            66, 193, 133, 44, 251, 11, 39, 63, 77, 79, 210, 74, 189, 33, 184, 116,
        ])
    );
    assert_eq!(
        alternate_receipt.post_batch_digest,
        StateDigest([
            229, 68, 195, 81, 155, 36, 136, 166, 202, 170, 122, 190, 36, 184, 158, 215, 49, 137,
            116, 16, 219, 107, 85, 3, 160, 105, 139, 50, 237, 92, 13, 16,
        ])
    );
}

#[test]
fn stale_cursor_rejection_preserves_the_entire_authority() {
    let mut authority = WorkshopAuthority::from_seed_u64(common::pack(), 29);
    authority
        .submit(CreatorBatchV1 {
            expected_cursor: None,
            expected_tick: WorkshopTick(0),
            operations: vec![CreatorOpV1::CreateSystem {
                local: common::local(0),
                name: common::name("Current"),
                position: GalaxyPointV1::new(0, 0).unwrap(),
            }],
        })
        .unwrap();
    let before = authority.encode_authority_for_test();

    let result = authority.submit(CreatorBatchV1 {
        expected_cursor: None,
        expected_tick: WorkshopTick(0),
        operations: Vec::new(),
    });

    assert_eq!(result, Err(CreatorRejectionV1::StaleCursor));
    assert_eq!(authority.encode_authority_for_test(), before);
}

#[test]
fn quantity_and_hazard_arithmetic_upper_bounds_reject_atomically() {
    let mut authority = WorkshopAuthority::from_seed_u64(common::pack(), 30);
    let receipt = authority
        .submit(CreatorBatchV1 {
            expected_cursor: None,
            expected_tick: WorkshopTick(0),
            operations: vec![
                CreatorOpV1::CreateSystem {
                    local: common::local(0),
                    name: common::name("Left"),
                    position: GalaxyPointV1::new(0, 0).unwrap(),
                },
                CreatorOpV1::CreateSystem {
                    local: common::local(1),
                    name: common::name("Right"),
                    position: GalaxyPointV1::new(1, 0).unwrap(),
                },
                CreatorOpV1::CreateStar {
                    local: common::local(2),
                    system: common::local_ref(0),
                    name: common::name("Left Star"),
                    archetype_id: common::catalog("yellow-dwarf"),
                },
                CreatorOpV1::CreateStar {
                    local: common::local(3),
                    system: common::local_ref(1),
                    name: common::name("Right Star"),
                    archetype_id: common::catalog("yellow-dwarf"),
                },
                CreatorOpV1::CreateWorld {
                    local: common::local(4),
                    system: common::local_ref(0),
                    primary: common::local_ref(2),
                    name: common::name("Left World"),
                    archetype_id: common::catalog("rocky-world"),
                    orbit_radius_milli_au: 1,
                    orbit_period_ticks: 1,
                    phase_millidegrees: 0,
                },
                CreatorOpV1::CreateWorld {
                    local: common::local(5),
                    system: common::local_ref(1),
                    primary: common::local_ref(3),
                    name: common::name("Right World"),
                    archetype_id: common::catalog("rocky-world"),
                    orbit_radius_milli_au: 1,
                    orbit_period_ticks: 1,
                    phase_millidegrees: 0,
                },
                CreatorOpV1::ConnectLane {
                    local: common::local(6),
                    a: common::local_ref(0),
                    b: common::local_ref(1),
                },
                CreatorOpV1::CreateDeposit {
                    local: common::local(7),
                    world: common::local_ref(4),
                    resource_id: common::catalog("ore"),
                    reserve_units: 1_000_000_000,
                },
                CreatorOpV1::ConnectRoute {
                    local: common::local(8),
                    source: common::local_ref(4),
                    destination: common::local_ref(5),
                    resource_id: common::catalog("energy"),
                    batch_units: 1_000_000_000,
                },
            ],
        })
        .unwrap();
    let left_world = receipt.entities[&common::local(4)];
    let right_world = receipt.entities[&common::local(5)];
    let lane = receipt.entities[&common::local(6)];

    let invalid_operations = [
        CreatorOpV1::CreateDeposit {
            local: common::local(20),
            world: common::existing(left_world),
            resource_id: common::catalog("ore"),
            reserve_units: 1_000_000_001,
        },
        CreatorOpV1::ConnectRoute {
            local: common::local(21),
            source: common::existing(left_world),
            destination: common::existing(right_world),
            resource_id: common::catalog("ore"),
            batch_units: 1_000_000_001,
        },
        CreatorOpV1::ScheduleHazard {
            local: common::local(22),
            lane: common::existing(lane),
            hazard_id: common::catalog("ion-storm"),
            start_tick: WorkshopTick(u64::MAX),
            duration_ticks: 1,
        },
    ];
    for operation in invalid_operations {
        let before = authority.encode_authority_for_test();
        assert_eq!(
            authority.submit(CreatorBatchV1 {
                expected_cursor: authority.cursor(),
                expected_tick: authority.state().tick,
                operations: vec![operation],
            }),
            Err(CreatorRejectionV1::InvalidValue)
        );
        assert_eq!(authority.encode_authority_for_test(), before);
    }
}

#[test]
fn creator_operation_count_accepts_128_and_atomically_rejects_129() {
    let mut authority = WorkshopAuthority::from_seed_u64(common::pack(), 31);
    let created = authority
        .submit(CreatorBatchV1 {
            expected_cursor: None,
            expected_tick: WorkshopTick(0),
            operations: vec![CreatorOpV1::CreateSystem {
                local: common::local(0),
                name: common::name("Initial"),
                position: GalaxyPointV1::new(0, 0).unwrap(),
            }],
        })
        .unwrap();
    let system = created.entities[&common::local(0)];
    let rename = CreatorOpV1::RenameObject {
        target: common::existing(system),
        name: ObjectName::new("\"".repeat(64)).unwrap(),
    };
    let accepted = authority
        .submit(CreatorBatchV1 {
            expected_cursor: authority.cursor(),
            expected_tick: WorkshopTick(0),
            operations: vec![rename.clone(); MAX_BATCH_OPERATIONS],
        })
        .unwrap();
    assert_eq!(accepted.ordinal, 1);
    assert_eq!(
        authority.state().systems[&system].name,
        match &rename {
            CreatorOpV1::RenameObject { name, .. } => name.clone(),
            _ => unreachable!(),
        }
    );

    let before = authority.encode_authority_for_test();
    let rejected = authority.submit(CreatorBatchV1 {
        expected_cursor: authority.cursor(),
        expected_tick: WorkshopTick(0),
        operations: vec![rename; MAX_BATCH_OPERATIONS + 1],
    });
    assert_eq!(
        rejected,
        Err(CreatorRejectionV1::TooManyOperations {
            actual: MAX_BATCH_OPERATIONS + 1,
            limit: MAX_BATCH_OPERATIONS,
        })
    );
    assert_eq!(authority.encode_authority_for_test(), before);
}

#[test]
fn operation_cap_proves_current_schema_cannot_reach_batch_byte_cap() {
    let maximum_name = ObjectName::new("\"".repeat(64)).unwrap();
    let maximum_catalog = CatalogId::new(format!("a{}", "z".repeat(31))).unwrap();
    let maximum_reference = ObjectRefV1::Existing(EntityId([u8::MAX; 16]));
    let maximum_local = BatchLocalId(u16::MAX);
    let maximum_position =
        GalaxyPointV1::new(-GALAXY_COORDINATE_LIMIT, -GALAXY_COORDINATE_LIMIT).unwrap();

    // One representative with maximum-length legal fields for every V1 variant.
    // If a variant is added or gains a field, this exhaustive schema inventory
    // must be updated before the byte-cap invariant can remain accepted.
    let variants = vec![
        CreatorOpV1::CreateFaction {
            local: maximum_local,
            name: maximum_name.clone(),
            color_rgb: [u8::MAX; 3],
        },
        CreatorOpV1::CreateSystem {
            local: maximum_local,
            name: maximum_name.clone(),
            position: maximum_position,
        },
        CreatorOpV1::CreateStar {
            local: maximum_local,
            system: maximum_reference,
            name: maximum_name.clone(),
            archetype_id: maximum_catalog.clone(),
        },
        CreatorOpV1::CreateWorld {
            local: maximum_local,
            system: maximum_reference,
            primary: maximum_reference,
            name: maximum_name.clone(),
            archetype_id: maximum_catalog.clone(),
            orbit_radius_milli_au: u32::MAX,
            orbit_period_ticks: u64::MAX,
            phase_millidegrees: 359_999,
        },
        CreatorOpV1::ConnectLane {
            local: maximum_local,
            a: maximum_reference,
            b: maximum_reference,
        },
        CreatorOpV1::CreateDeposit {
            local: maximum_local,
            world: maximum_reference,
            resource_id: maximum_catalog.clone(),
            reserve_units: 1_000_000_000,
        },
        CreatorOpV1::SetOwner {
            target: maximum_reference,
            faction: Some(maximum_reference),
        },
        CreatorOpV1::PlaceIndustry {
            local: maximum_local,
            world: maximum_reference,
            definition_id: maximum_catalog.clone(),
            linked_deposit: Some(maximum_reference),
        },
        CreatorOpV1::ConnectRoute {
            local: maximum_local,
            source: maximum_reference,
            destination: maximum_reference,
            resource_id: maximum_catalog.clone(),
            batch_units: 1_000_000_000,
        },
        CreatorOpV1::SetIndustryEnabled {
            industry: maximum_reference,
            enabled: false,
        },
        CreatorOpV1::RenameObject {
            target: maximum_reference,
            name: maximum_name,
        },
        CreatorOpV1::RemoveObject {
            target: maximum_reference,
        },
        CreatorOpV1::ScheduleHazard {
            local: maximum_local,
            lane: maximum_reference,
            hazard_id: maximum_catalog,
            start_tick: WorkshopTick(u64::MAX - 36_000),
            duration_ticks: 36_000,
        },
        CreatorOpV1::CancelHazard {
            hazard: maximum_reference,
        },
    ];
    assert_eq!(variants.len(), 14, "V1 operation inventory changed");
    let largest = variants
        .into_iter()
        .max_by_key(|operation| serde_json::to_vec(operation).unwrap().len())
        .unwrap();
    assert!(matches!(largest, CreatorOpV1::CreateWorld { .. }));

    let empty = CreatorBatchV1 {
        expected_cursor: Some(RevisionId([u8::MAX; 32])),
        expected_tick: WorkshopTick(u64::MAX),
        operations: Vec::new(),
    };
    let maximum_batch = CreatorBatchV1 {
        operations: vec![largest.clone(); MAX_BATCH_OPERATIONS],
        ..empty.clone()
    };
    let empty_bytes = serde_json::to_vec(&empty).unwrap().len();
    let operation_bytes = serde_json::to_vec(&largest).unwrap().len();
    let proven_upper_bound =
        empty_bytes + MAX_BATCH_OPERATIONS * operation_bytes + (MAX_BATCH_OPERATIONS - 1);
    let actual = serde_json::to_vec(&maximum_batch).unwrap().len();

    assert_eq!(actual, proven_upper_bound);
    assert_eq!(actual, 60_298);
    assert!(actual < MAX_BATCH_BYTES);
}

#[test]
fn every_creator_entity_capacity_is_exact_and_atomic() {
    verify_faction_capacity();
    verify_system_capacity();
    verify_star_capacity();
    verify_world_capacity();
    verify_lane_capacity();
    verify_deposit_capacity();
    verify_industry_capacity();
    verify_route_capacity();
    verify_hazard_capacity();
}

fn submit_operations(authority: &mut WorkshopAuthority, operations: Vec<CreatorOpV1>) {
    for chunk in operations.chunks(MAX_BATCH_OPERATIONS) {
        authority
            .submit(CreatorBatchV1 {
                expected_cursor: authority.cursor(),
                expected_tick: authority.state().tick,
                operations: chunk.to_vec(),
            })
            .unwrap();
    }
}

fn assert_capacity_rejection(
    authority: &mut WorkshopAuthority,
    kind: EntityKind,
    operation: CreatorOpV1,
) {
    let before = authority.encode_authority_for_test();
    let result = authority.submit(CreatorBatchV1 {
        expected_cursor: authority.cursor(),
        expected_tick: authority.state().tick,
        operations: vec![operation],
    });
    assert_eq!(result, Err(CreatorRejectionV1::Capacity { kind }));
    assert_eq!(authority.encode_authority_for_test(), before);
}

fn verify_faction_capacity() {
    let mut authority = WorkshopAuthority::from_seed_u64(common::pack(), 101);
    submit_operations(
        &mut authority,
        (0..MAX_FACTIONS)
            .map(|index| CreatorOpV1::CreateFaction {
                local: common::local(u16::try_from(index).unwrap()),
                name: common::name("Faction"),
                color_rgb: [0, 0, 0],
            })
            .collect(),
    );
    assert_eq!(authority.state().factions.len(), MAX_FACTIONS);
    assert_capacity_rejection(
        &mut authority,
        EntityKind::Faction,
        CreatorOpV1::CreateFaction {
            local: common::local(1_000),
            name: common::name("Overflow"),
            color_rgb: [0, 0, 0],
        },
    );
}

fn verify_system_capacity() {
    let mut authority = WorkshopAuthority::from_seed_u64(common::pack(), 102);
    submit_operations(
        &mut authority,
        (0..MAX_SYSTEMS)
            .map(|index| CreatorOpV1::CreateSystem {
                local: common::local(u16::try_from(index).unwrap()),
                name: common::name("System"),
                position: GalaxyPointV1::new(i64::try_from(index).unwrap(), 0).unwrap(),
            })
            .collect(),
    );
    assert_eq!(authority.state().systems.len(), MAX_SYSTEMS);
    assert_capacity_rejection(
        &mut authority,
        EntityKind::System,
        CreatorOpV1::CreateSystem {
            local: common::local(1_000),
            name: common::name("Overflow"),
            position: GalaxyPointV1::new(i64::try_from(MAX_SYSTEMS).unwrap(), 0).unwrap(),
        },
    );
}

fn create_system_and_star(authority: &mut WorkshopAuthority) -> (EntityId, EntityId) {
    let receipt = authority
        .submit(CreatorBatchV1 {
            expected_cursor: authority.cursor(),
            expected_tick: authority.state().tick,
            operations: vec![
                CreatorOpV1::CreateSystem {
                    local: common::local(0),
                    name: common::name("System"),
                    position: GalaxyPointV1::new(0, 0).unwrap(),
                },
                CreatorOpV1::CreateStar {
                    local: common::local(1),
                    system: common::local_ref(0),
                    name: common::name("Star"),
                    archetype_id: common::catalog("yellow-dwarf"),
                },
            ],
        })
        .unwrap();
    (
        receipt.entities[&common::local(0)],
        receipt.entities[&common::local(1)],
    )
}

fn create_world(authority: &mut WorkshopAuthority) -> EntityId {
    let (system, star) = create_system_and_star(authority);
    authority
        .submit(CreatorBatchV1 {
            expected_cursor: authority.cursor(),
            expected_tick: authority.state().tick,
            operations: vec![CreatorOpV1::CreateWorld {
                local: common::local(2),
                system: common::existing(system),
                primary: common::existing(star),
                name: common::name("World"),
                archetype_id: common::catalog("rocky-world"),
                orbit_radius_milli_au: 1,
                orbit_period_ticks: 1,
                phase_millidegrees: 0,
            }],
        })
        .unwrap()
        .entities[&common::local(2)]
}

fn verify_star_capacity() {
    let mut authority = WorkshopAuthority::from_seed_u64(common::pack(), 103);
    let (system, _) = create_system_and_star(&mut authority);
    submit_operations(
        &mut authority,
        (1..MAX_STARS)
            .map(|index| CreatorOpV1::CreateStar {
                local: common::local(u16::try_from(index).unwrap()),
                system: common::existing(system),
                name: common::name("Star"),
                archetype_id: common::catalog("yellow-dwarf"),
            })
            .collect(),
    );
    assert_eq!(authority.state().stars.len(), MAX_STARS);
    assert_capacity_rejection(
        &mut authority,
        EntityKind::Star,
        CreatorOpV1::CreateStar {
            local: common::local(1_000),
            system: common::existing(system),
            name: common::name("Overflow"),
            archetype_id: common::catalog("yellow-dwarf"),
        },
    );
}

fn verify_world_capacity() {
    let mut authority = WorkshopAuthority::from_seed_u64(common::pack(), 104);
    let (system, star) = create_system_and_star(&mut authority);
    submit_operations(
        &mut authority,
        (0..MAX_WORLDS)
            .map(|index| CreatorOpV1::CreateWorld {
                local: common::local(u16::try_from(index).unwrap()),
                system: common::existing(system),
                primary: common::existing(star),
                name: common::name("World"),
                archetype_id: common::catalog("rocky-world"),
                orbit_radius_milli_au: 1,
                orbit_period_ticks: 1,
                phase_millidegrees: 0,
            })
            .collect(),
    );
    assert_eq!(authority.state().worlds.len(), MAX_WORLDS);
    assert_capacity_rejection(
        &mut authority,
        EntityKind::World,
        CreatorOpV1::CreateWorld {
            local: common::local(1_000),
            system: common::existing(system),
            primary: common::existing(star),
            name: common::name("Overflow"),
            archetype_id: common::catalog("rocky-world"),
            orbit_radius_milli_au: 1,
            orbit_period_ticks: 1,
            phase_millidegrees: 0,
        },
    );
}

fn verify_lane_capacity() {
    let mut authority = WorkshopAuthority::from_seed_u64(common::pack(), 105);
    submit_operations(
        &mut authority,
        (0..MAX_SYSTEMS)
            .map(|index| CreatorOpV1::CreateSystem {
                local: common::local(u16::try_from(index).unwrap()),
                name: common::name("System"),
                position: GalaxyPointV1::new(i64::try_from(index).unwrap(), 0).unwrap(),
            })
            .collect(),
    );
    let systems: Vec<_> = authority.state().systems.keys().copied().collect();
    let mut lane_operations = Vec::with_capacity(MAX_LANES + 1);
    'outer: for (left_index, left) in systems.iter().enumerate() {
        for right in &systems[left_index + 1..] {
            let index = lane_operations.len();
            lane_operations.push(CreatorOpV1::ConnectLane {
                local: common::local(u16::try_from(index).unwrap()),
                a: common::existing(*left),
                b: common::existing(*right),
            });
            if lane_operations.len() == MAX_LANES + 1 {
                break 'outer;
            }
        }
    }
    let overflow = lane_operations.pop().unwrap();
    submit_operations(&mut authority, lane_operations);
    assert_eq!(authority.state().lanes.len(), MAX_LANES);
    assert_capacity_rejection(&mut authority, EntityKind::Lane, overflow);
}

fn verify_deposit_capacity() {
    let mut authority = WorkshopAuthority::from_seed_u64(common::pack(), 106);
    let world = create_world(&mut authority);
    submit_operations(
        &mut authority,
        (0..MAX_DEPOSITS)
            .map(|index| CreatorOpV1::CreateDeposit {
                local: common::local(u16::try_from(index).unwrap()),
                world: common::existing(world),
                resource_id: common::catalog("ore"),
                reserve_units: 1,
            })
            .collect(),
    );
    assert_eq!(authority.state().deposits.len(), MAX_DEPOSITS);
    assert_capacity_rejection(
        &mut authority,
        EntityKind::Deposit,
        CreatorOpV1::CreateDeposit {
            local: common::local(2_000),
            world: common::existing(world),
            resource_id: common::catalog("ore"),
            reserve_units: 1,
        },
    );
}

fn verify_industry_capacity() {
    let mut authority = WorkshopAuthority::from_seed_u64(common::pack(), 107);
    let world = create_world(&mut authority);
    submit_operations(
        &mut authority,
        (0..MAX_INDUSTRIES)
            .map(|index| CreatorOpV1::PlaceIndustry {
                local: common::local(u16::try_from(index).unwrap()),
                world: common::existing(world),
                definition_id: common::catalog("solar-array"),
                linked_deposit: None,
            })
            .collect(),
    );
    assert_eq!(authority.state().industries.len(), MAX_INDUSTRIES);
    assert_capacity_rejection(
        &mut authority,
        EntityKind::Industry,
        CreatorOpV1::PlaceIndustry {
            local: common::local(3_000),
            world: common::existing(world),
            definition_id: common::catalog("solar-array"),
            linked_deposit: None,
        },
    );
}

fn verify_route_capacity() {
    let mut authority = WorkshopAuthority::from_seed_u64(common::pack(), 108);
    let (system, star) = create_system_and_star(&mut authority);
    submit_operations(
        &mut authority,
        (0..27)
            .map(|index| CreatorOpV1::CreateWorld {
                local: common::local(u16::try_from(index).unwrap()),
                system: common::existing(system),
                primary: common::existing(star),
                name: common::name("World"),
                archetype_id: common::catalog("rocky-world"),
                orbit_radius_milli_au: 1,
                orbit_period_ticks: 1,
                phase_millidegrees: 0,
            })
            .collect(),
    );
    let worlds: Vec<_> = authority.state().worlds.keys().copied().collect();
    let resources = ["alloy", "energy", "ore"];
    let mut route_operations = Vec::with_capacity(MAX_ROUTES + 1);
    'outer: for source in &worlds {
        for destination in &worlds {
            if source == destination {
                continue;
            }
            for resource in resources {
                let index = route_operations.len();
                route_operations.push(CreatorOpV1::ConnectRoute {
                    local: common::local(u16::try_from(index).unwrap()),
                    source: common::existing(*source),
                    destination: common::existing(*destination),
                    resource_id: common::catalog(resource),
                    batch_units: 1,
                });
                if route_operations.len() == MAX_ROUTES + 1 {
                    break 'outer;
                }
            }
        }
    }
    let overflow = route_operations.pop().unwrap();
    submit_operations(&mut authority, route_operations);
    assert_eq!(authority.state().routes.len(), MAX_ROUTES);
    assert_capacity_rejection(&mut authority, EntityKind::Route, overflow);
}

fn verify_hazard_capacity() {
    let mut authority = WorkshopAuthority::from_seed_u64(common::pack(), 109);
    let setup = authority
        .submit(CreatorBatchV1 {
            expected_cursor: None,
            expected_tick: WorkshopTick(0),
            operations: vec![
                CreatorOpV1::CreateSystem {
                    local: common::local(0),
                    name: common::name("Left"),
                    position: GalaxyPointV1::new(0, 0).unwrap(),
                },
                CreatorOpV1::CreateSystem {
                    local: common::local(1),
                    name: common::name("Right"),
                    position: GalaxyPointV1::new(1, 0).unwrap(),
                },
                CreatorOpV1::ConnectLane {
                    local: common::local(2),
                    a: common::local_ref(0),
                    b: common::local_ref(1),
                },
            ],
        })
        .unwrap();
    let lane = setup.entities[&common::local(2)];
    submit_operations(
        &mut authority,
        (0..MAX_HAZARDS)
            .map(|index| CreatorOpV1::ScheduleHazard {
                local: common::local(u16::try_from(index).unwrap()),
                lane: common::existing(lane),
                hazard_id: common::catalog("ion-storm"),
                start_tick: WorkshopTick(0),
                duration_ticks: 1,
            })
            .collect(),
    );
    assert_eq!(authority.state().hazards.len(), MAX_HAZARDS);
    assert_capacity_rejection(
        &mut authority,
        EntityKind::Hazard,
        CreatorOpV1::ScheduleHazard {
            local: common::local(1_000),
            lane: common::existing(lane),
            hazard_id: common::catalog("ion-storm"),
            start_tick: WorkshopTick(0),
            duration_ticks: 1,
        },
    );
}
