mod common;

use std::collections::BTreeMap;

use nyon_workshop_core::{
    CatalogId, CreatorBatchV1, CreatorOpV1, DeterministicFault, EntityId, GalaxyCoord,
    GalaxyPointV1, HistoryError, TickEventV1, WorkshopHistory, WorkshopStateV1, WorkshopTick,
    decode_catalog_pack,
    ids::GALAXY_COORDINATE_LIMIT,
    model::{MAX_INDUSTRIES, MAX_ROUTES, MAX_SHIPMENTS, WorldV1},
};

#[test]
fn exact_phase_order_delivers_then_produces_then_dispatches() {
    let pack = common::pack();
    let mut history = WorkshopHistory::from_seed_u64(pack.clone(), 21);
    let forge = common::create_forge(&mut history);
    let tick_zero = history.step().unwrap();
    assert_eq!(tick_zero.tick, WorkshopTick(0));
    assert!(history.state().shipments.values().all(|shipment| {
        shipment.dispatched_tick == WorkshopTick(0) && shipment.arrival_tick == WorkshopTick(1)
    }));

    let tick_one = history.step().unwrap();
    let last_delivery = tick_one
        .events
        .iter()
        .rposition(|event| matches!(event, TickEventV1::ShipmentDelivered { .. }))
        .unwrap();
    let first_industry = tick_one
        .events
        .iter()
        .position(|event| matches!(event, TickEventV1::IndustryRan { .. }))
        .unwrap();
    assert!(last_delivery < first_industry);

    let mut foundry_receipt = None;
    for _ in 0..5 {
        let receipt = history.step().unwrap();
        if receipt.events.iter().any(
            |event| matches!(event, TickEventV1::IndustryRan { industry } if *industry == forge.foundry),
        ) {
            foundry_receipt = Some(receipt);
            break;
        }
    }
    let foundry_receipt = foundry_receipt.expect("foundry must run after routed inputs arrive");
    let last_delivery = foundry_receipt
        .events
        .iter()
        .rposition(|event| matches!(event, TickEventV1::ShipmentDelivered { .. }))
        .unwrap();
    let foundry_run = foundry_receipt
        .events
        .iter()
        .position(|event| {
            matches!(event, TickEventV1::IndustryRan { industry } if *industry == forge.foundry)
        })
        .unwrap();
    assert!(last_delivery < foundry_run);
    assert_eq!(
        history.state().worlds[&forge.world_b].inventory[&common::catalog("alloy")],
        1
    );
}

#[test]
fn ion_storm_is_half_open_and_halves_capacity_with_floor() {
    let pack = common::pack();
    let mut history = WorkshopHistory::from_seed_u64(pack.clone(), 22);
    let forge = common::create_forge(&mut history);
    history
        .submit(CreatorBatchV1 {
            expected_cursor: history.active_revision(),
            expected_tick: WorkshopTick(0),
            operations: vec![CreatorOpV1::ScheduleHazard {
                local: common::local(90),
                lane: common::existing(forge.lane),
                hazard_id: common::catalog("ion-storm"),
                start_tick: WorkshopTick(0),
                duration_ticks: 1,
            }],
        })
        .unwrap();
    let active = history.step().unwrap();
    assert!(matches!(
        active.events.first(),
        Some(TickEventV1::HazardActivated { .. })
    ));
    let dispatched: Vec<(nyon_workshop_core::EntityId, u64)> = active
        .events
        .iter()
        .filter_map(|event| match event {
            TickEventV1::RouteDispatched { route, units, .. } => Some((*route, *units)),
            _ => None,
        })
        .collect();
    assert!(dispatched.contains(&(forge.energy_route, 1)));
    assert!(dispatched.iter().all(|(_, units)| *units <= 2));

    let expired = history.step().unwrap();
    assert!(matches!(
        expired.events.first(),
        Some(TickEventV1::HazardExpired { .. })
    ));
    assert!(expired.events.iter().any(|event| {
        matches!(event, TickEventV1::RouteDispatched { route, units: 2, .. } if *route == forge.energy_route)
    }));
}

#[test]
fn pacing_grouping_does_not_change_authoritative_result() {
    let mut history = WorkshopHistory::from_seed_u64(common::pack(), 24);
    common::create_forge(&mut history);
    let mut direct = history.clone();
    let mut grouped = history;
    direct.advance_ticks(100).unwrap();
    for group in [20_u64, 4, 1, 20, 20, 20, 15] {
        grouped.advance_ticks(group).unwrap();
    }
    assert_eq!(
        direct.state().canonical_bytes(),
        grouped.state().canonical_bytes()
    );
    assert_eq!(direct.active_state_digest(), grouped.active_state_digest());
}

#[test]
fn coordinate_extremes_and_integer_square_root_floor_are_exact() {
    assert!(GalaxyCoord::new(-GALAXY_COORDINATE_LIMIT).is_ok());
    assert!(GalaxyCoord::new(GALAXY_COORDINATE_LIMIT).is_ok());
    assert!(GalaxyCoord::new(-GALAXY_COORDINATE_LIMIT - 1).is_err());
    assert!(GalaxyCoord::new(GALAXY_COORDINATE_LIMIT + 1).is_err());

    let cases = [
        ((0, 0), (0, 0), 0),
        ((0, 0), (1, 1), 1),
        ((0, 0), (2, 2), 2),
        ((0, 0), (3, 4), 5),
        ((0, 0), (5, 5), 7),
        (
            (-GALAXY_COORDINATE_LIMIT, -GALAXY_COORDINATE_LIMIT),
            (GALAXY_COORDINATE_LIMIT, GALAXY_COORDINATE_LIMIT),
            379_625_062,
        ),
    ];
    for (left, right, expected) in cases {
        let left = GalaxyPointV1::new(left.0, left.1).unwrap();
        let right = GalaxyPointV1::new(right.0, right.1).unwrap();
        assert_eq!(left.distance_units(right).unwrap(), expected);
        assert_eq!(right.distance_units(left).unwrap(), expected);
    }
}

#[test]
fn route_travel_uses_exact_integer_distance_and_ceiling_division() {
    let cases = [
        ((0, 0), (255, 0), 255, 1),
        ((0, 0), (256, 0), 256, 1),
        ((0, 0), (257, 0), 257, 2),
        ((0, 0), (257, 257), 363, 2),
        (
            (-GALAXY_COORDINATE_LIMIT, -GALAXY_COORDINATE_LIMIT),
            (GALAXY_COORDINATE_LIMIT, GALAXY_COORDINATE_LIMIT),
            379_625_062,
            1_482_911,
        ),
    ];

    for (index, (left, right, expected_distance, expected_travel)) in cases.into_iter().enumerate()
    {
        let fixture = routed_solar_fixture(
            common::pack(),
            200 + u64::try_from(index).unwrap(),
            left,
            right,
            1,
        );
        assert_eq!(
            fixture.history.state().lanes[&fixture.lane].distance_units,
            expected_distance
        );
        let mut history = fixture.history;
        let receipt = history.step().unwrap();
        assert!(receipt.events.iter().any(|event| {
            matches!(
                event,
                TickEventV1::RouteDispatched {
                    route,
                    units: 1,
                    arrival_tick,
                    ..
                } if *route == fixture.route && *arrival_tick == WorkshopTick(expected_travel)
            )
        }));
    }
}

#[test]
fn maximum_route_quantity_is_dispatched_and_delivered_without_loss() {
    let pack = pack_with_solar_quantity(1_000_000_000);
    let fixture = routed_solar_fixture(pack, 220, (0, 0), (257, 0), 1_000_000_000);
    let mut history = fixture.history;

    let dispatched = history.step().unwrap();
    assert!(dispatched.events.iter().any(|event| {
        matches!(
            event,
            TickEventV1::RouteDispatched {
                route,
                units: 1_000_000_000,
                arrival_tick: WorkshopTick(2),
                ..
            } if *route == fixture.route
        )
    }));
    assert_eq!(
        history.state().worlds[&fixture.source]
            .inventory
            .get(&common::catalog("energy"))
            .copied()
            .unwrap_or(0),
        0
    );

    history.step().unwrap();
    let delivered = history.step().unwrap();
    assert!(delivered.events.iter().any(|event| {
        matches!(
            event,
            TickEventV1::ShipmentDelivered {
                destination,
                resource_id,
                units: 1_000_000_000,
                ..
            } if *destination == fixture.destination && resource_id == &common::catalog("energy")
        )
    }));
    assert_eq!(
        history.state().worlds[&fixture.destination].inventory[&common::catalog("energy")],
        1_000_000_000
    );
}

#[test]
fn shipment_capacity_fault_rolls_back_the_complete_tick() {
    let pack = pack_with_solar_quantity(1_000_000_000);
    let mut history = WorkshopHistory::from_seed_u64(pack, 221);
    let setup = history
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
                    position: GalaxyPointV1::new(1_024, 0).unwrap(),
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
                CreatorOpV1::ConnectLane {
                    local: common::local(4),
                    a: common::local_ref(0),
                    b: common::local_ref(1),
                },
            ],
        })
        .unwrap();
    let left_system = setup.entities[&common::local(0)];
    let right_system = setup.entities[&common::local(1)];
    let left_star = setup.entities[&common::local(2)];
    let right_star = setup.entities[&common::local(3)];

    let mut worlds = Vec::with_capacity(64);
    let world_operations: Vec<_> = (0..64)
        .map(|index| {
            let (system, star) = if index < 32 {
                (left_system, left_star)
            } else {
                (right_system, right_star)
            };
            CreatorOpV1::CreateWorld {
                local: common::local(index),
                system: common::existing(system),
                primary: common::existing(star),
                name: common::name("Transit World"),
                archetype_id: common::catalog("rocky-world"),
                orbit_radius_milli_au: 1,
                orbit_period_ticks: 1,
                phase_millidegrees: 0,
            }
        })
        .collect();
    let receipt = history
        .submit(CreatorBatchV1 {
            expected_cursor: history.active_revision(),
            expected_tick: history.state().tick,
            operations: world_operations,
        })
        .unwrap();
    for index in 0..64 {
        worlds.push(receipt.entities[&common::local(index)]);
    }
    let (left_worlds, right_worlds) = worlds.split_at(32);

    submit_history_operations(
        &mut history,
        worlds
            .iter()
            .flat_map(|world| std::iter::repeat_n(*world, 32))
            .enumerate()
            .map(|(index, world)| CreatorOpV1::PlaceIndustry {
                local: common::local(u16::try_from(index).unwrap()),
                world: common::existing(world),
                definition_id: common::catalog("solar-array"),
                linked_deposit: None,
            })
            .collect(),
    );
    assert_eq!(history.state().industries.len(), MAX_INDUSTRIES);

    let route_pairs = left_worlds
        .iter()
        .flat_map(|left| right_worlds.iter().map(move |right| (*left, *right)))
        .chain(
            right_worlds
                .iter()
                .flat_map(|right| left_worlds.iter().map(move |left| (*right, *left))),
        );
    submit_history_operations(
        &mut history,
        route_pairs
            .enumerate()
            .map(|(index, (source, destination))| CreatorOpV1::ConnectRoute {
                local: common::local(u16::try_from(index).unwrap()),
                source: common::existing(source),
                destination: common::existing(destination),
                resource_id: common::catalog("energy"),
                batch_units: 1_000_000_000,
            })
            .collect(),
    );
    assert_eq!(history.state().routes.len(), MAX_ROUTES);

    history.step().unwrap();
    history.step().unwrap();
    assert_eq!(history.state().shipments.len(), MAX_SHIPMENTS);
    let before_state = history.state().canonical_bytes();
    let before_view = history.active_view().clone();
    let before_branches = history.branches().clone();

    assert_eq!(
        history.step(),
        Err(HistoryError::Simulation(
            DeterministicFault::ShipmentCapacity
        ))
    );
    assert_eq!(history.state().canonical_bytes(), before_state);
    assert_eq!(history.active_view(), &before_view);
    assert_eq!(history.branches(), &before_branches);
}

#[test]
fn ordered_state_encoding_ignores_map_insertion_order() {
    let system_a = EntityId([1; 16]);
    let system_b = EntityId([2; 16]);
    let star = EntityId([3; 16]);
    let world = EntityId([4; 16]);
    let energy = CatalogId::new("energy").unwrap();
    let ore = CatalogId::new("ore").unwrap();

    let mut first = WorkshopStateV1::default();
    first.systems.insert(
        system_a,
        nyon_workshop_core::model::SystemV1 {
            name: common::name("A"),
            position: GalaxyPointV1::new(0, 0).unwrap(),
        },
    );
    first.systems.insert(
        system_b,
        nyon_workshop_core::model::SystemV1 {
            name: common::name("B"),
            position: GalaxyPointV1::new(1, 0).unwrap(),
        },
    );
    first.worlds.insert(
        world,
        WorldV1 {
            system: system_a,
            primary: star,
            name: common::name("World"),
            archetype_id: common::catalog("rocky-world"),
            orbit_radius_milli_au: 1,
            orbit_period_ticks: 1,
            phase_millidegrees: 0,
            inventory: BTreeMap::from([(energy.clone(), 2), (ore.clone(), 3)]),
        },
    );

    let mut second = WorkshopStateV1::default();
    second.systems.insert(
        system_b,
        nyon_workshop_core::model::SystemV1 {
            name: common::name("B"),
            position: GalaxyPointV1::new(1, 0).unwrap(),
        },
    );
    second.systems.insert(
        system_a,
        nyon_workshop_core::model::SystemV1 {
            name: common::name("A"),
            position: GalaxyPointV1::new(0, 0).unwrap(),
        },
    );
    let mut reverse_inventory = BTreeMap::new();
    reverse_inventory.insert(ore, 3);
    reverse_inventory.insert(energy, 2);
    second.worlds.insert(
        world,
        WorldV1 {
            system: system_a,
            primary: star,
            name: common::name("World"),
            archetype_id: common::catalog("rocky-world"),
            orbit_radius_milli_au: 1,
            orbit_period_ticks: 1,
            phase_millidegrees: 0,
            inventory: reverse_inventory,
        },
    );

    assert_eq!(first.canonical_bytes(), second.canonical_bytes());
    assert_eq!(first.digest(), second.digest());
}

struct RoutedSolarFixture {
    history: WorkshopHistory,
    lane: EntityId,
    route: EntityId,
    source: EntityId,
    destination: EntityId,
}

fn routed_solar_fixture(
    pack: nyon_workshop_core::ValidatedCatalogPackV1,
    seed: u64,
    left: (i64, i64),
    right: (i64, i64),
    batch_units: u64,
) -> RoutedSolarFixture {
    let mut history = WorkshopHistory::from_seed_u64(pack, seed);
    let receipt = history
        .submit(CreatorBatchV1 {
            expected_cursor: None,
            expected_tick: WorkshopTick(0),
            operations: vec![
                CreatorOpV1::CreateSystem {
                    local: common::local(0),
                    name: common::name("Source System"),
                    position: GalaxyPointV1::new(left.0, left.1).unwrap(),
                },
                CreatorOpV1::CreateSystem {
                    local: common::local(1),
                    name: common::name("Destination System"),
                    position: GalaxyPointV1::new(right.0, right.1).unwrap(),
                },
                CreatorOpV1::CreateStar {
                    local: common::local(2),
                    system: common::local_ref(0),
                    name: common::name("Source Star"),
                    archetype_id: common::catalog("yellow-dwarf"),
                },
                CreatorOpV1::CreateStar {
                    local: common::local(3),
                    system: common::local_ref(1),
                    name: common::name("Destination Star"),
                    archetype_id: common::catalog("yellow-dwarf"),
                },
                CreatorOpV1::CreateWorld {
                    local: common::local(4),
                    system: common::local_ref(0),
                    primary: common::local_ref(2),
                    name: common::name("Source World"),
                    archetype_id: common::catalog("rocky-world"),
                    orbit_radius_milli_au: 1,
                    orbit_period_ticks: 1,
                    phase_millidegrees: 0,
                },
                CreatorOpV1::CreateWorld {
                    local: common::local(5),
                    system: common::local_ref(1),
                    primary: common::local_ref(3),
                    name: common::name("Destination World"),
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
                CreatorOpV1::PlaceIndustry {
                    local: common::local(7),
                    world: common::local_ref(4),
                    definition_id: common::catalog("solar-array"),
                    linked_deposit: None,
                },
                CreatorOpV1::ConnectRoute {
                    local: common::local(8),
                    source: common::local_ref(4),
                    destination: common::local_ref(5),
                    resource_id: common::catalog("energy"),
                    batch_units,
                },
            ],
        })
        .unwrap();
    RoutedSolarFixture {
        lane: receipt.entities[&common::local(6)],
        route: receipt.entities[&common::local(8)],
        source: receipt.entities[&common::local(4)],
        destination: receipt.entities[&common::local(5)],
        history,
    }
}

fn pack_with_solar_quantity(quantity: u64) -> nyon_workshop_core::ValidatedCatalogPackV1 {
    let mut value: serde_json::Value = serde_json::from_slice(common::CORE_PACK).unwrap();
    value["industry_definitions"][0]["outputs"][0]["quantity"] = quantity.into();
    decode_catalog_pack(&serde_json::to_vec(&value).unwrap()).unwrap()
}

fn submit_history_operations(history: &mut WorkshopHistory, operations: Vec<CreatorOpV1>) {
    for chunk in operations.chunks(nyon_workshop_core::command::MAX_BATCH_OPERATIONS) {
        history
            .submit(CreatorBatchV1 {
                expected_cursor: history.active_revision(),
                expected_tick: history.state().tick,
                operations: chunk.to_vec(),
            })
            .unwrap();
    }
}
