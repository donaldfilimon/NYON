use std::collections::BTreeMap;

use nyon::{
    presentation::workshop::{
        WorkshopSemanticRole, WorkshopStarInstance, WorkshopSystemInstance, WorkshopWorldInstance,
        build_workshop_scene_frame,
    },
    workshop::{
        CatalogId, EntityId, GalaxyPointV1, ObjectName, WorkshopStateV1, WorkshopTick,
        model::{LaneV1, RouteV1, ShipmentV1, StarV1, SystemV1, WorldV1},
    },
};

fn id(value: u8) -> EntityId {
    EntityId([value; 16])
}

fn star_a_id() -> EntityId {
    EntityId([
        0x01, 0x23, 0x45, 0x67, 0x89, 0xAB, 0xCD, 0xEF, 0x10, 0x32, 0x54, 0x76, 0x98, 0xBA, 0xDC,
        0xFE,
    ])
}

fn star_b_id() -> EntityId {
    EntityId([
        0x11, 0x22, 0x33, 0x44, 0x55, 0x66, 0x77, 0x88, 0x99, 0xAA, 0xBB, 0xCC, 0xDD, 0xEE, 0xF0,
        0x0F,
    ])
}

fn catalog(value: &str) -> CatalogId {
    CatalogId::new(value).unwrap()
}

fn name(value: &str) -> ObjectName {
    ObjectName::new(value).unwrap()
}

fn populated_state(reverse_insertion: bool) -> WorkshopStateV1 {
    let system_a = id(1);
    let system_b = id(2);
    let world_a = id(3);
    let world_b = id(4);
    let route = id(5);
    let shipment = id(6);
    let lane = id(7);
    let star_a = star_a_id();
    let star_b = star_b_id();

    let mut state = WorkshopStateV1 {
        tick: WorkshopTick(3),
        ..WorkshopStateV1::default()
    };
    let systems = [
        (
            system_a,
            SystemV1 {
                name: name("Alpha"),
                position: GalaxyPointV1::new(0, 0).unwrap(),
            },
        ),
        (
            system_b,
            SystemV1 {
                name: name("Beta"),
                position: GalaxyPointV1::new(10_240, 0).unwrap(),
            },
        ),
    ];
    let worlds = [
        (
            world_a,
            WorldV1 {
                system: system_a,
                primary: star_a,
                name: name("Forge A"),
                archetype_id: catalog("rocky-world"),
                orbit_radius_milli_au: 1_000,
                orbit_period_ticks: 100,
                phase_millidegrees: 0,
                inventory: BTreeMap::new(),
            },
        ),
        (
            world_b,
            WorldV1 {
                system: system_b,
                primary: star_b,
                name: name("Forge B"),
                archetype_id: catalog("rocky-world"),
                orbit_radius_milli_au: 1_000,
                orbit_period_ticks: 100,
                phase_millidegrees: 0,
                inventory: BTreeMap::new(),
            },
        ),
    ];

    let system_order: Box<dyn Iterator<Item = _>> = if reverse_insertion {
        Box::new(systems.into_iter().rev())
    } else {
        Box::new(systems.into_iter())
    };
    for (entity, system) in system_order {
        state.systems.insert(entity, system);
    }
    let stars = [
        (
            star_a,
            StarV1 {
                system: system_a,
                name: name("Alpha Prime"),
                archetype_id: catalog("yellow-dwarf"),
            },
        ),
        (
            star_b,
            StarV1 {
                system: system_b,
                name: name("Beta Prime"),
                archetype_id: catalog("red-dwarf"),
            },
        ),
    ];
    let star_order: Box<dyn Iterator<Item = _>> = if reverse_insertion {
        Box::new(stars.into_iter().rev())
    } else {
        Box::new(stars.into_iter())
    };
    for (entity, star) in star_order {
        state.stars.insert(entity, star);
    }
    let world_order: Box<dyn Iterator<Item = _>> = if reverse_insertion {
        Box::new(worlds.into_iter().rev())
    } else {
        Box::new(worlds.into_iter())
    };
    for (entity, world) in world_order {
        state.worlds.insert(entity, world);
    }
    state.lanes.insert(
        lane,
        LaneV1 {
            a: system_a,
            b: system_b,
            distance_units: 10_240,
        },
    );
    state.routes.insert(
        route,
        RouteV1 {
            source: world_a,
            destination: world_b,
            resource_id: catalog("ore"),
            batch_units: 3,
        },
    );
    state.shipments.insert(
        shipment,
        ShipmentV1 {
            route,
            source: world_a,
            destination: world_b,
            resource_id: catalog("ore"),
            units: 3,
            dispatched_tick: WorkshopTick(2),
            arrival_tick: WorkshopTick(4),
        },
    );
    state
}

#[test]
fn extraction_is_one_way_and_map_insertion_order_independent() {
    let forward = populated_state(false);
    let reverse = populated_state(true);
    let before = forward.canonical_bytes();

    let forward_frame = build_workshop_scene_frame(&forward, Some(id(3)), false);
    let reverse_frame = build_workshop_scene_frame(&reverse, Some(id(3)), false);

    assert_eq!(forward.canonical_bytes(), before);
    assert_eq!(forward.digest(), reverse.digest());
    assert_eq!(forward_frame, reverse_frame);
    assert_eq!(forward_frame.state_digest, forward.digest());
}

#[test]
fn routes_lanes_and_shipments_have_stable_derived_presentation() {
    let state = populated_state(false);
    let frame = build_workshop_scene_frame(&state, None, false);

    assert_eq!(frame.lanes.len(), 1);
    assert_eq!(frame.shipments.len(), 1);
    assert_eq!(frame.shipments[0].position_units[3], 0.5);
    assert!(
        frame
            .semantic_entries
            .iter()
            .any(|entry| entry.role == WorkshopSemanticRole::Lane)
    );
    assert!(
        frame
            .semantic_entries
            .iter()
            .any(|entry| entry.role == WorkshopSemanticRole::Route)
    );
}

#[test]
fn every_star_visual_is_paired_with_one_semantic_entry_and_exact_identity() {
    let state = populated_state(false);
    let selected = star_b_id();
    let frame = build_workshop_scene_frame(&state, Some(selected), false);
    let star_semantics = frame
        .semantic_entries
        .iter()
        .filter(|entry| entry.role == WorkshopSemanticRole::Star)
        .collect::<Vec<_>>();

    assert_eq!(frame.stars.len(), state.stars.len());
    assert_eq!(star_semantics.len(), frame.stars.len());
    for (instance, semantic) in frame.stars.iter().zip(star_semantics) {
        assert_eq!(entity_from_words(instance.metadata), semantic.entity);
        assert_eq!(
            [
                semantic.bounds[0] + semantic.bounds[2] * 0.5,
                semantic.bounds[1] + semantic.bounds[3] * 0.5,
            ],
            [instance.position_radius[0], instance.position_radius[1]]
        );
        assert_eq!(instance.flags[0], u32::from(semantic.selected));
        assert_eq!(instance.flags[1..], [0, 0, 0]);
        assert!(
            instance
                .position_radius
                .iter()
                .all(|value| value.is_finite())
        );
        assert!(instance.position_radius[3] > 0.0);
        assert!(instance.color.iter().all(|value| value.is_finite()));
    }
    assert_eq!(
        frame.stars[0].metadata,
        [0x6745_2301, 0xEFCD_AB89, 0x7654_3210, 0xFEDC_BA98]
    );
    assert_eq!(
        frame.stars[1].metadata,
        [0x4433_2211, 0x8877_6655, 0xCCBB_AA99, 0x0FF0_EEDD]
    );
    assert_eq!(entity_from_words(frame.stars[0].metadata), star_a_id());
    assert_eq!(entity_from_words(frame.stars[1].metadata), selected);
    assert_eq!(frame.stars[0].flags, [0, 0, 0, 0]);
    assert_eq!(frame.stars[1].flags, [1, 0, 0, 0]);
    assert_eq!(frame.stars[1].flags[0], 1);
}

#[test]
fn display_preferences_cannot_change_authoritative_identity() {
    let state = populated_state(false);
    let before = state.canonical_bytes();
    let ordinary = build_workshop_scene_frame(&state, None, false);
    let high_contrast = build_workshop_scene_frame(&state, None, true);

    assert_ne!(ordinary.systems, high_contrast.systems);
    assert_ne!(ordinary.stars, high_contrast.stars);
    assert_eq!(ordinary.state_digest, high_contrast.state_digest);
    assert_eq!(ordinary.tick, high_contrast.tick);
    assert_eq!(ordinary.shipments, high_contrast.shipments);
    assert_eq!(state.canonical_bytes(), before);
    assert_eq!(state.digest(), ordinary.state_digest);
}

#[test]
fn gpu_instance_strides_are_explicit() {
    assert_eq!(std::mem::size_of::<WorkshopSystemInstance>(), 48);
    assert_eq!(WorkshopSystemInstance::LAYOUT.array_stride, 48);
    assert_eq!(std::mem::size_of::<WorkshopStarInstance>(), 64);
    assert_eq!(std::mem::align_of::<WorkshopStarInstance>(), 4);
    assert_eq!(std::mem::offset_of!(WorkshopStarInstance, color), 16);
    assert_eq!(std::mem::offset_of!(WorkshopStarInstance, metadata), 32);
    assert_eq!(std::mem::offset_of!(WorkshopStarInstance, flags), 48);
    assert_eq!(WorkshopStarInstance::LAYOUT.array_stride, 64);
    assert_eq!(std::mem::size_of::<WorkshopWorldInstance>(), 64);
    assert_eq!(WorkshopWorldInstance::LAYOUT.array_stride, 64);
}

fn entity_from_words(words: [u32; 4]) -> EntityId {
    let mut bytes = [0_u8; 16];
    for (chunk, word) in bytes.as_chunks_mut::<4>().0.iter_mut().zip(words) {
        chunk.copy_from_slice(&word.to_le_bytes());
    }
    EntityId(bytes)
}
