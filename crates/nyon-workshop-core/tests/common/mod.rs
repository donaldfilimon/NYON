#![allow(dead_code)]

use nyon_workshop_core::{
    BatchLocalId, CatalogId, CreatorBatchV1, CreatorOpV1, EntityId, GalaxyPointV1, ObjectName,
    ObjectRefV1, ValidatedCatalogPackV1, WorkshopHistory, WorkshopTick, decode_catalog_pack,
};

pub const CORE_PACK: &[u8] = include_bytes!("../../../../assets/workshop/core-pack-v1.json");

pub fn pack() -> ValidatedCatalogPackV1 {
    decode_catalog_pack(CORE_PACK).expect("shipped Workshop pack must validate")
}

pub fn name(value: &str) -> ObjectName {
    ObjectName::new(value).unwrap()
}

pub fn catalog(value: &str) -> CatalogId {
    CatalogId::new(value).unwrap()
}

pub fn local(value: u16) -> BatchLocalId {
    BatchLocalId(value)
}

pub fn local_ref(value: u16) -> ObjectRefV1 {
    ObjectRefV1::Local(local(value))
}

pub fn existing(value: EntityId) -> ObjectRefV1 {
    ObjectRefV1::Existing(value)
}

#[derive(Clone, Copy, Debug)]
pub struct ForgeIds {
    pub faction_a: EntityId,
    pub faction_b: EntityId,
    pub system_a: EntityId,
    pub system_b: EntityId,
    pub star_a: EntityId,
    pub star_b: EntityId,
    pub world_a: EntityId,
    pub world_b: EntityId,
    pub lane: EntityId,
    pub deposit: EntityId,
    pub solar: EntityId,
    pub extractor: EntityId,
    pub foundry: EntityId,
    pub energy_route: EntityId,
    pub ore_route: EntityId,
}

pub fn create_forge(history: &mut WorkshopHistory) -> ForgeIds {
    let operations = vec![
        CreatorOpV1::CreateFaction {
            local: local(0),
            name: name("Gold Union"),
            color_rgb: [240, 190, 40],
        },
        CreatorOpV1::CreateFaction {
            local: local(1),
            name: name("Blue Union"),
            color_rgb: [40, 120, 240],
        },
        CreatorOpV1::CreateSystem {
            local: local(2),
            name: name("Anvil"),
            position: GalaxyPointV1::new(0, 0).unwrap(),
        },
        CreatorOpV1::CreateSystem {
            local: local(3),
            name: name("Relay"),
            position: GalaxyPointV1::new(256, 0).unwrap(),
        },
        CreatorOpV1::CreateStar {
            local: local(4),
            system: local_ref(2),
            name: name("Anvil Sun"),
            archetype_id: catalog("yellow-dwarf"),
        },
        CreatorOpV1::CreateStar {
            local: local(5),
            system: local_ref(3),
            name: name("Relay Sun"),
            archetype_id: catalog("yellow-dwarf"),
        },
        CreatorOpV1::CreateWorld {
            local: local(6),
            system: local_ref(2),
            primary: local_ref(4),
            name: name("Anvil Prime"),
            archetype_id: catalog("rocky-world"),
            orbit_radius_milli_au: 1_000,
            orbit_period_ticks: 1_000,
            phase_millidegrees: 0,
        },
        CreatorOpV1::CreateWorld {
            local: local(7),
            system: local_ref(3),
            primary: local_ref(5),
            name: name("Relay Prime"),
            archetype_id: catalog("rocky-world"),
            orbit_radius_milli_au: 1_000,
            orbit_period_ticks: 1_000,
            phase_millidegrees: 0,
        },
        CreatorOpV1::SetOwner {
            target: local_ref(6),
            faction: Some(local_ref(0)),
        },
        CreatorOpV1::SetOwner {
            target: local_ref(7),
            faction: Some(local_ref(1)),
        },
        CreatorOpV1::ConnectLane {
            local: local(8),
            a: local_ref(2),
            b: local_ref(3),
        },
        CreatorOpV1::CreateDeposit {
            local: local(9),
            world: local_ref(6),
            resource_id: catalog("ore"),
            reserve_units: 100,
        },
        CreatorOpV1::PlaceIndustry {
            local: local(10),
            world: local_ref(6),
            definition_id: catalog("solar-array"),
            linked_deposit: None,
        },
        CreatorOpV1::PlaceIndustry {
            local: local(11),
            world: local_ref(6),
            definition_id: catalog("extractor"),
            linked_deposit: Some(local_ref(9)),
        },
        CreatorOpV1::PlaceIndustry {
            local: local(12),
            world: local_ref(7),
            definition_id: catalog("foundry"),
            linked_deposit: None,
        },
        CreatorOpV1::ConnectRoute {
            local: local(13),
            source: local_ref(6),
            destination: local_ref(7),
            resource_id: catalog("energy"),
            batch_units: 2,
        },
        CreatorOpV1::ConnectRoute {
            local: local(14),
            source: local_ref(6),
            destination: local_ref(7),
            resource_id: catalog("ore"),
            batch_units: 4,
        },
    ];
    let receipt = history
        .submit(CreatorBatchV1 {
            expected_cursor: history.active_revision(),
            expected_tick: WorkshopTick(0),
            operations,
        })
        .unwrap();
    let id = |value| receipt.entities[&local(value)];
    ForgeIds {
        faction_a: id(0),
        faction_b: id(1),
        system_a: id(2),
        system_b: id(3),
        star_a: id(4),
        star_b: id(5),
        world_a: id(6),
        world_b: id(7),
        lane: id(8),
        deposit: id(9),
        solar: id(10),
        extractor: id(11),
        foundry: id(12),
        energy_route: id(13),
        ore_route: id(14),
    }
}
