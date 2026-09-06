use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::{
    command::CreatorBatchV1,
    ids::{CatalogId, EntityId, GalaxyPointV1, ObjectName, RevisionId, StateDigest, WorkshopTick},
};

pub const MAX_FACTIONS: usize = 16;
pub const MAX_SYSTEMS: usize = 64;
pub const MAX_STARS: usize = 128;
pub const MAX_WORLDS: usize = 512;
pub const MAX_LANES: usize = 256;
pub const MAX_DEPOSITS: usize = 1_024;
pub const MAX_INDUSTRIES: usize = 2_048;
pub const MAX_ROUTES: usize = 2_048;
pub const MAX_SHIPMENTS: usize = 4_096;
pub const MAX_HAZARDS: usize = 128;

#[derive(Clone, Copy, Debug, Deserialize, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
#[repr(u8)]
#[serde(rename_all = "snake_case")]
pub enum EntityKind {
    Faction,
    System,
    Star,
    World,
    Lane,
    Deposit,
    Industry,
    Route,
    Shipment,
    Hazard,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct FactionV1 {
    pub name: ObjectName,
    pub color_rgb: [u8; 3],
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct SystemV1 {
    pub name: ObjectName,
    pub position: GalaxyPointV1,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct StarV1 {
    pub system: EntityId,
    pub name: ObjectName,
    pub archetype_id: CatalogId,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct WorldV1 {
    pub system: EntityId,
    pub primary: EntityId,
    pub name: ObjectName,
    pub archetype_id: CatalogId,
    pub orbit_radius_milli_au: u32,
    pub orbit_period_ticks: u64,
    pub phase_millidegrees: u32,
    pub inventory: BTreeMap<CatalogId, u64>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct LaneV1 {
    pub a: EntityId,
    pub b: EntityId,
    pub distance_units: u64,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct DepositV1 {
    pub world: EntityId,
    pub resource_id: CatalogId,
    pub reserve_units: u64,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct IndustryV1 {
    pub world: EntityId,
    pub definition_id: CatalogId,
    pub linked_deposit: Option<EntityId>,
    pub enabled: bool,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct RouteV1 {
    pub source: EntityId,
    pub destination: EntityId,
    pub resource_id: CatalogId,
    pub batch_units: u64,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ShipmentV1 {
    pub route: EntityId,
    pub source: EntityId,
    pub destination: EntityId,
    pub resource_id: CatalogId,
    pub units: u64,
    pub dispatched_tick: WorkshopTick,
    pub arrival_tick: WorkshopTick,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct HazardV1 {
    pub lane: EntityId,
    pub hazard_id: CatalogId,
    pub start_tick: WorkshopTick,
    pub duration_ticks: u64,
    pub cancelled: bool,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct WorkshopStateV1 {
    pub tick: WorkshopTick,
    pub factions: BTreeMap<EntityId, FactionV1>,
    pub systems: BTreeMap<EntityId, SystemV1>,
    pub stars: BTreeMap<EntityId, StarV1>,
    pub worlds: BTreeMap<EntityId, WorldV1>,
    pub lanes: BTreeMap<EntityId, LaneV1>,
    pub deposits: BTreeMap<EntityId, DepositV1>,
    pub industries: BTreeMap<EntityId, IndustryV1>,
    pub routes: BTreeMap<EntityId, RouteV1>,
    pub shipments: BTreeMap<EntityId, ShipmentV1>,
    pub hazards: BTreeMap<EntityId, HazardV1>,
    pub owners: BTreeMap<EntityId, EntityId>,
}

impl Default for WorkshopStateV1 {
    fn default() -> Self {
        Self {
            tick: WorkshopTick(0),
            factions: BTreeMap::new(),
            systems: BTreeMap::new(),
            stars: BTreeMap::new(),
            worlds: BTreeMap::new(),
            lanes: BTreeMap::new(),
            deposits: BTreeMap::new(),
            industries: BTreeMap::new(),
            routes: BTreeMap::new(),
            shipments: BTreeMap::new(),
            hazards: BTreeMap::new(),
            owners: BTreeMap::new(),
        }
    }
}

impl WorkshopStateV1 {
    pub fn entity_kind(&self, id: EntityId) -> Option<EntityKind> {
        [
            (self.factions.contains_key(&id), EntityKind::Faction),
            (self.systems.contains_key(&id), EntityKind::System),
            (self.stars.contains_key(&id), EntityKind::Star),
            (self.worlds.contains_key(&id), EntityKind::World),
            (self.lanes.contains_key(&id), EntityKind::Lane),
            (self.deposits.contains_key(&id), EntityKind::Deposit),
            (self.industries.contains_key(&id), EntityKind::Industry),
            (self.routes.contains_key(&id), EntityKind::Route),
            (self.shipments.contains_key(&id), EntityKind::Shipment),
            (self.hazards.contains_key(&id), EntityKind::Hazard),
        ]
        .into_iter()
        .find_map(|(present, kind)| present.then_some(kind))
    }

    pub fn contains_entity(&self, id: EntityId) -> bool {
        self.entity_kind(id).is_some()
    }

    pub fn canonical_bytes(&self) -> Vec<u8> {
        serde_json::to_vec(self).expect("Workshop state has an infallible canonical encoding")
    }

    pub fn digest(&self) -> StateDigest {
        self.digest_with_dynamic(self.tick, &self.worlds, &self.deposits, &self.shipments)
    }

    pub(crate) fn digest_with_dynamic(
        &self,
        tick: WorkshopTick,
        worlds: &BTreeMap<EntityId, WorldV1>,
        deposits: &BTreeMap<EntityId, DepositV1>,
        shipments: &BTreeMap<EntityId, ShipmentV1>,
    ) -> StateDigest {
        #[derive(Serialize)]
        struct CanonicalStateRef<'a> {
            tick: WorkshopTick,
            factions: &'a BTreeMap<EntityId, FactionV1>,
            systems: &'a BTreeMap<EntityId, SystemV1>,
            stars: &'a BTreeMap<EntityId, StarV1>,
            worlds: &'a BTreeMap<EntityId, WorldV1>,
            lanes: &'a BTreeMap<EntityId, LaneV1>,
            deposits: &'a BTreeMap<EntityId, DepositV1>,
            industries: &'a BTreeMap<EntityId, IndustryV1>,
            routes: &'a BTreeMap<EntityId, RouteV1>,
            shipments: &'a BTreeMap<EntityId, ShipmentV1>,
            hazards: &'a BTreeMap<EntityId, HazardV1>,
            owners: &'a BTreeMap<EntityId, EntityId>,
        }

        let mut hash = Sha256::new();
        hash.update(b"NYON-WORKSHOP-STATE-V1\0");
        let state = CanonicalStateRef {
            tick,
            factions: &self.factions,
            systems: &self.systems,
            stars: &self.stars,
            worlds,
            lanes: &self.lanes,
            deposits,
            industries: &self.industries,
            routes: &self.routes,
            shipments,
            hazards: &self.hazards,
            owners: &self.owners,
        };
        hash.update(
            serde_json::to_vec(&state)
                .expect("Workshop state has an infallible canonical encoding"),
        );
        StateDigest(hash.finalize().into())
    }

    pub(crate) fn remove_unreferenced(&mut self, id: EntityId) {
        self.factions.remove(&id);
        self.systems.remove(&id);
        self.stars.remove(&id);
        self.worlds.remove(&id);
        self.lanes.remove(&id);
        self.deposits.remove(&id);
        self.industries.remove(&id);
        self.routes.remove(&id);
        self.shipments.remove(&id);
        self.hazards.remove(&id);
        self.owners.remove(&id);
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct RevisionRecordV1 {
    pub id: RevisionId,
    pub parent: Option<RevisionId>,
    pub tick: WorkshopTick,
    pub ordinal: u64,
    pub batch: CreatorBatchV1,
    pub state_digest: StateDigest,
}
