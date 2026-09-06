use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::{
    WORKSHOP_RULES_VERSION,
    ids::{
        BatchLocalId, CatalogId, EntityId, GalaxyPointV1, ObjectName, ObjectRefV1, RevisionId,
        StateDigest, WorkshopTick,
    },
    model::{
        DepositV1, EntityKind, FactionV1, HazardV1, IndustryV1, LaneV1, MAX_DEPOSITS, MAX_FACTIONS,
        MAX_HAZARDS, MAX_INDUSTRIES, MAX_LANES, MAX_ROUTES, MAX_STARS, MAX_SYSTEMS, MAX_WORLDS,
        RouteV1, StarV1, SystemV1, WorkshopStateV1, WorldV1,
    },
    pack::{CatalogDefinitionKind, ValidatedCatalogPackV1},
};

pub const MAX_BATCH_OPERATIONS: usize = 128;
pub const MAX_BATCH_BYTES: usize = 65_536;

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct CreatorBatchV1 {
    pub expected_cursor: Option<RevisionId>,
    pub expected_tick: WorkshopTick,
    pub operations: Vec<CreatorOpV1>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum CreatorOpV1 {
    CreateFaction {
        local: BatchLocalId,
        name: ObjectName,
        color_rgb: [u8; 3],
    },
    CreateSystem {
        local: BatchLocalId,
        name: ObjectName,
        position: GalaxyPointV1,
    },
    CreateStar {
        local: BatchLocalId,
        system: ObjectRefV1,
        name: ObjectName,
        archetype_id: CatalogId,
    },
    CreateWorld {
        local: BatchLocalId,
        system: ObjectRefV1,
        primary: ObjectRefV1,
        name: ObjectName,
        archetype_id: CatalogId,
        orbit_radius_milli_au: u32,
        orbit_period_ticks: u64,
        phase_millidegrees: u32,
    },
    ConnectLane {
        local: BatchLocalId,
        a: ObjectRefV1,
        b: ObjectRefV1,
    },
    CreateDeposit {
        local: BatchLocalId,
        world: ObjectRefV1,
        resource_id: CatalogId,
        reserve_units: u64,
    },
    SetOwner {
        target: ObjectRefV1,
        faction: Option<ObjectRefV1>,
    },
    PlaceIndustry {
        local: BatchLocalId,
        world: ObjectRefV1,
        definition_id: CatalogId,
        linked_deposit: Option<ObjectRefV1>,
    },
    ConnectRoute {
        local: BatchLocalId,
        source: ObjectRefV1,
        destination: ObjectRefV1,
        resource_id: CatalogId,
        batch_units: u64,
    },
    SetIndustryEnabled {
        industry: ObjectRefV1,
        enabled: bool,
    },
    RenameObject {
        target: ObjectRefV1,
        name: ObjectName,
    },
    RemoveObject {
        target: ObjectRefV1,
    },
    ScheduleHazard {
        local: BatchLocalId,
        lane: ObjectRefV1,
        hazard_id: CatalogId,
        start_tick: WorkshopTick,
        duration_ticks: u64,
    },
    CancelHazard {
        hazard: ObjectRefV1,
    },
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct CreatorReceiptV1 {
    pub revision: RevisionId,
    pub tick: WorkshopTick,
    pub ordinal: u64,
    pub entities: BTreeMap<BatchLocalId, EntityId>,
    pub post_batch_digest: StateDigest,
}

#[derive(Clone, Debug, Eq, PartialEq, thiserror::Error)]
pub enum CreatorRejectionV1 {
    #[error("creator batch contains {actual} operations; the limit is {limit}")]
    TooManyOperations { actual: usize, limit: usize },
    #[error("canonical creator batch has {actual} bytes; the limit is {limit}")]
    BatchTooLarge { actual: usize, limit: usize },
    #[error("creator batch cursor is stale")]
    StaleCursor,
    #[error("creator batch tick is stale")]
    StaleTick,
    #[error("authority ordinal space is exhausted")]
    OrdinalExhausted,
    #[error("batch local identifier is duplicated")]
    DuplicateLocalId,
    #[error("batch local reference is forward, absent, or of the wrong entity kind")]
    InvalidLocalReference,
    #[error("referenced entity does not exist")]
    UnknownEntity,
    #[error("referenced entity has the wrong kind")]
    WrongEntityKind,
    #[error("referenced catalog definition does not exist or has the wrong kind")]
    UnknownCatalogDefinition,
    #[error("entity capacity is exhausted for {kind:?}")]
    Capacity { kind: EntityKind },
    #[error("creator operation contains an invalid value")]
    InvalidValue,
    #[error("creator operation would duplicate an existing relationship or position")]
    DuplicateRelationship,
    #[error("object removal is blocked by current dependents")]
    BlockingDependencies { entities: Vec<EntityId> },
    #[error("a deterministic entity identifier collided")]
    IdentityCollision,
    #[error("checked creator arithmetic failed")]
    Arithmetic,
    #[error("creator batch encoding failed")]
    Encoding,
}

#[derive(Clone, Debug)]
pub struct WorkshopAuthority {
    catalog: ValidatedCatalogPackV1,
    genesis_seed: [u8; 32],
    cursor: Option<RevisionId>,
    state: WorkshopStateV1,
    ordinal_tick: WorkshopTick,
    next_ordinal: u64,
}

impl WorkshopAuthority {
    pub fn new(catalog: ValidatedCatalogPackV1, genesis_seed: [u8; 32]) -> Self {
        Self {
            catalog,
            genesis_seed,
            cursor: None,
            state: WorkshopStateV1::default(),
            ordinal_tick: WorkshopTick(0),
            next_ordinal: 0,
        }
    }

    pub fn from_seed_u64(catalog: ValidatedCatalogPackV1, seed: u64) -> Self {
        let mut genesis_seed = [0_u8; 32];
        genesis_seed[..8].copy_from_slice(&seed.to_le_bytes());
        Self::new(catalog, genesis_seed)
    }

    pub fn catalog(&self) -> &ValidatedCatalogPackV1 {
        &self.catalog
    }

    pub const fn genesis_seed(&self) -> [u8; 32] {
        self.genesis_seed
    }

    pub const fn cursor(&self) -> Option<RevisionId> {
        self.cursor
    }

    pub fn state(&self) -> &WorkshopStateV1 {
        &self.state
    }

    pub fn submit(
        &mut self,
        batch: CreatorBatchV1,
    ) -> Result<CreatorReceiptV1, CreatorRejectionV1> {
        let (ordinal_tick, ordinal) = if self.ordinal_tick == self.state.tick {
            (self.ordinal_tick, self.next_ordinal)
        } else {
            (self.state.tick, 0)
        };
        let (state, receipt) = apply_batch(
            &self.catalog,
            self.genesis_seed,
            self.cursor,
            self.state.tick,
            ordinal,
            &batch,
            &self.state,
        )?;
        self.next_ordinal = ordinal
            .checked_add(1)
            .ok_or(CreatorRejectionV1::OrdinalExhausted)?;
        self.ordinal_tick = ordinal_tick;
        self.cursor = Some(receipt.revision);
        self.state = state;
        Ok(receipt)
    }

    pub fn encode_authority_for_test(&self) -> Vec<u8> {
        serde_json::to_vec(&(
            self.genesis_seed,
            self.cursor,
            &self.state,
            self.ordinal_tick,
            self.next_ordinal,
        ))
        .expect("authority test encoding is infallible")
    }

    pub(crate) fn from_parts(
        catalog: ValidatedCatalogPackV1,
        genesis_seed: [u8; 32],
        cursor: Option<RevisionId>,
        state: WorkshopStateV1,
        next_ordinal: u64,
    ) -> Self {
        Self {
            catalog,
            genesis_seed,
            cursor,
            ordinal_tick: state.tick,
            state,
            next_ordinal,
        }
    }

    pub(crate) const fn next_ordinal(&self) -> u64 {
        self.next_ordinal
    }
}

pub(crate) fn apply_recorded_batch(
    catalog: &ValidatedCatalogPackV1,
    genesis_seed: [u8; 32],
    parent: Option<RevisionId>,
    tick: WorkshopTick,
    ordinal: u64,
    batch: &CreatorBatchV1,
    state: &WorkshopStateV1,
) -> Result<(WorkshopStateV1, CreatorReceiptV1), CreatorRejectionV1> {
    apply_batch(catalog, genesis_seed, parent, tick, ordinal, batch, state)
}

fn apply_batch(
    catalog: &ValidatedCatalogPackV1,
    genesis_seed: [u8; 32],
    parent: Option<RevisionId>,
    tick: WorkshopTick,
    ordinal: u64,
    batch: &CreatorBatchV1,
    state: &WorkshopStateV1,
) -> Result<(WorkshopStateV1, CreatorReceiptV1), CreatorRejectionV1> {
    if batch.operations.len() > MAX_BATCH_OPERATIONS {
        return Err(CreatorRejectionV1::TooManyOperations {
            actual: batch.operations.len(),
            limit: MAX_BATCH_OPERATIONS,
        });
    }
    let batch_bytes = serde_json::to_vec(batch).map_err(|_| CreatorRejectionV1::Encoding)?;
    if batch_bytes.len() > MAX_BATCH_BYTES {
        return Err(CreatorRejectionV1::BatchTooLarge {
            actual: batch_bytes.len(),
            limit: MAX_BATCH_BYTES,
        });
    }
    if batch.expected_cursor != parent {
        return Err(CreatorRejectionV1::StaleCursor);
    }
    if batch.expected_tick != tick || state.tick != tick {
        return Err(CreatorRejectionV1::StaleTick);
    }

    let revision = revision_id(catalog, genesis_seed, parent, tick, ordinal, &batch_bytes);
    let mut candidate = state.clone();
    let mut locals = BTreeMap::<BatchLocalId, (EntityId, EntityKind)>::new();
    let mut entities = BTreeMap::new();
    for operation in &batch.operations {
        apply_operation(
            catalog,
            revision,
            operation,
            &mut candidate,
            &mut locals,
            &mut entities,
        )?;
    }
    let post_batch_digest = candidate.digest();
    Ok((
        candidate,
        CreatorReceiptV1 {
            revision,
            tick,
            ordinal,
            entities,
            post_batch_digest,
        },
    ))
}

fn apply_operation(
    catalog: &ValidatedCatalogPackV1,
    revision: RevisionId,
    operation: &CreatorOpV1,
    state: &mut WorkshopStateV1,
    locals: &mut BTreeMap<BatchLocalId, (EntityId, EntityKind)>,
    entities: &mut BTreeMap<BatchLocalId, EntityId>,
) -> Result<(), CreatorRejectionV1> {
    match operation {
        CreatorOpV1::CreateFaction {
            local,
            name,
            color_rgb,
        } => {
            ensure_capacity(state.factions.len(), MAX_FACTIONS, EntityKind::Faction)?;
            let id = allocate(revision, EntityKind::Faction, *local, state, locals)?;
            state.factions.insert(
                id,
                FactionV1 {
                    name: name.clone(),
                    color_rgb: *color_rgb,
                },
            );
            register(*local, id, EntityKind::Faction, locals, entities);
        }
        CreatorOpV1::CreateSystem {
            local,
            name,
            position,
        } => {
            ensure_capacity(state.systems.len(), MAX_SYSTEMS, EntityKind::System)?;
            if state
                .systems
                .values()
                .any(|item| item.position == *position)
            {
                return Err(CreatorRejectionV1::DuplicateRelationship);
            }
            let id = allocate(revision, EntityKind::System, *local, state, locals)?;
            state.systems.insert(
                id,
                SystemV1 {
                    name: name.clone(),
                    position: *position,
                },
            );
            register(*local, id, EntityKind::System, locals, entities);
        }
        CreatorOpV1::CreateStar {
            local,
            system,
            name,
            archetype_id,
        } => {
            ensure_capacity(state.stars.len(), MAX_STARS, EntityKind::Star)?;
            let system = resolve(state, locals, *system, &[EntityKind::System])?;
            if !catalog.has_star_archetype(archetype_id) {
                return Err(CreatorRejectionV1::UnknownCatalogDefinition);
            }
            let id = allocate(revision, EntityKind::Star, *local, state, locals)?;
            state.stars.insert(
                id,
                StarV1 {
                    system,
                    name: name.clone(),
                    archetype_id: archetype_id.clone(),
                },
            );
            register(*local, id, EntityKind::Star, locals, entities);
        }
        CreatorOpV1::CreateWorld {
            local,
            system,
            primary,
            name,
            archetype_id,
            orbit_radius_milli_au,
            orbit_period_ticks,
            phase_millidegrees,
        } => {
            ensure_capacity(state.worlds.len(), MAX_WORLDS, EntityKind::World)?;
            let system = resolve(state, locals, *system, &[EntityKind::System])?;
            let primary = resolve(state, locals, *primary, &[EntityKind::Star])?;
            if state.stars.get(&primary).map(|star| star.system) != Some(system) {
                return Err(CreatorRejectionV1::WrongEntityKind);
            }
            if !catalog.has_world_archetype(archetype_id) {
                return Err(CreatorRejectionV1::UnknownCatalogDefinition);
            }
            if *orbit_radius_milli_au == 0
                || *orbit_period_ticks == 0
                || *phase_millidegrees >= 360_000
            {
                return Err(CreatorRejectionV1::InvalidValue);
            }
            let id = allocate(revision, EntityKind::World, *local, state, locals)?;
            state.worlds.insert(
                id,
                WorldV1 {
                    system,
                    primary,
                    name: name.clone(),
                    archetype_id: archetype_id.clone(),
                    orbit_radius_milli_au: *orbit_radius_milli_au,
                    orbit_period_ticks: *orbit_period_ticks,
                    phase_millidegrees: *phase_millidegrees,
                    inventory: BTreeMap::new(),
                },
            );
            register(*local, id, EntityKind::World, locals, entities);
        }
        CreatorOpV1::ConnectLane { local, a, b } => {
            ensure_capacity(state.lanes.len(), MAX_LANES, EntityKind::Lane)?;
            let a = resolve(state, locals, *a, &[EntityKind::System])?;
            let b = resolve(state, locals, *b, &[EntityKind::System])?;
            if a == b || find_lane(state, a, b).is_some() {
                return Err(CreatorRejectionV1::DuplicateRelationship);
            }
            let distance_units = state.systems[&a]
                .position
                .distance_units(state.systems[&b].position)
                .map_err(|_| CreatorRejectionV1::Arithmetic)?;
            let id = allocate(revision, EntityKind::Lane, *local, state, locals)?;
            state.lanes.insert(
                id,
                LaneV1 {
                    a,
                    b,
                    distance_units,
                },
            );
            register(*local, id, EntityKind::Lane, locals, entities);
        }
        CreatorOpV1::CreateDeposit {
            local,
            world,
            resource_id,
            reserve_units,
        } => {
            ensure_capacity(state.deposits.len(), MAX_DEPOSITS, EntityKind::Deposit)?;
            let world = resolve(state, locals, *world, &[EntityKind::World])?;
            if !catalog.has_resource(resource_id) {
                return Err(CreatorRejectionV1::UnknownCatalogDefinition);
            }
            if !(1..=1_000_000_000).contains(reserve_units) {
                return Err(CreatorRejectionV1::InvalidValue);
            }
            let id = allocate(revision, EntityKind::Deposit, *local, state, locals)?;
            state.deposits.insert(
                id,
                DepositV1 {
                    world,
                    resource_id: resource_id.clone(),
                    reserve_units: *reserve_units,
                },
            );
            register(*local, id, EntityKind::Deposit, locals, entities);
        }
        CreatorOpV1::SetOwner { target, faction } => {
            let target = resolve(state, locals, *target, &OWNABLE_KINDS)?;
            if let Some(faction) = faction {
                let faction = resolve(state, locals, *faction, &[EntityKind::Faction])?;
                state.owners.insert(target, faction);
            } else {
                state.owners.remove(&target);
            }
        }
        CreatorOpV1::PlaceIndustry {
            local,
            world,
            definition_id,
            linked_deposit,
        } => {
            ensure_capacity(state.industries.len(), MAX_INDUSTRIES, EntityKind::Industry)?;
            let world = resolve(state, locals, *world, &[EntityKind::World])?;
            let definition = catalog
                .industry(definition_id)
                .ok_or(CreatorRejectionV1::UnknownCatalogDefinition)?;
            let linked_deposit = linked_deposit
                .map(|reference| resolve(state, locals, reference, &[EntityKind::Deposit]))
                .transpose()?;
            match definition.kind() {
                CatalogDefinitionKind::Extractor => {
                    let deposit_id = linked_deposit.ok_or(CreatorRejectionV1::InvalidValue)?;
                    let deposit = &state.deposits[&deposit_id];
                    if deposit.world != world
                        || Some(&deposit.resource_id) != definition.extractor_resource()
                    {
                        return Err(CreatorRejectionV1::InvalidValue);
                    }
                }
                CatalogDefinitionKind::Solar | CatalogDefinitionKind::Processor => {
                    if linked_deposit.is_some() {
                        return Err(CreatorRejectionV1::InvalidValue);
                    }
                }
            }
            let id = allocate(revision, EntityKind::Industry, *local, state, locals)?;
            state.industries.insert(
                id,
                IndustryV1 {
                    world,
                    definition_id: definition_id.clone(),
                    linked_deposit,
                    enabled: true,
                },
            );
            register(*local, id, EntityKind::Industry, locals, entities);
        }
        CreatorOpV1::ConnectRoute {
            local,
            source,
            destination,
            resource_id,
            batch_units,
        } => {
            ensure_capacity(state.routes.len(), MAX_ROUTES, EntityKind::Route)?;
            let source = resolve(state, locals, *source, &[EntityKind::World])?;
            let destination = resolve(state, locals, *destination, &[EntityKind::World])?;
            if source == destination || !(1..=1_000_000_000).contains(batch_units) {
                return Err(CreatorRejectionV1::InvalidValue);
            }
            if !catalog.has_resource(resource_id) {
                return Err(CreatorRejectionV1::UnknownCatalogDefinition);
            }
            if state.routes.values().any(|route| {
                route.source == source
                    && route.destination == destination
                    && route.resource_id == *resource_id
            }) {
                return Err(CreatorRejectionV1::DuplicateRelationship);
            }
            let source_system = state.worlds[&source].system;
            let destination_system = state.worlds[&destination].system;
            if source_system != destination_system
                && find_lane(state, source_system, destination_system).is_none()
            {
                return Err(CreatorRejectionV1::InvalidValue);
            }
            let id = allocate(revision, EntityKind::Route, *local, state, locals)?;
            state.routes.insert(
                id,
                RouteV1 {
                    source,
                    destination,
                    resource_id: resource_id.clone(),
                    batch_units: *batch_units,
                },
            );
            register(*local, id, EntityKind::Route, locals, entities);
        }
        CreatorOpV1::SetIndustryEnabled { industry, enabled } => {
            let industry = resolve(state, locals, *industry, &[EntityKind::Industry])?;
            state
                .industries
                .get_mut(&industry)
                .ok_or(CreatorRejectionV1::UnknownEntity)?
                .enabled = *enabled;
        }
        CreatorOpV1::RenameObject { target, name } => {
            let target = resolve(state, locals, *target, &NAMED_KINDS)?;
            match state.entity_kind(target) {
                Some(EntityKind::Faction) => {
                    state.factions.get_mut(&target).unwrap().name = name.clone()
                }
                Some(EntityKind::System) => {
                    state.systems.get_mut(&target).unwrap().name = name.clone()
                }
                Some(EntityKind::Star) => state.stars.get_mut(&target).unwrap().name = name.clone(),
                Some(EntityKind::World) => {
                    state.worlds.get_mut(&target).unwrap().name = name.clone()
                }
                _ => return Err(CreatorRejectionV1::WrongEntityKind),
            }
        }
        CreatorOpV1::RemoveObject { target } => {
            let target = resolve(state, locals, *target, &REMOVABLE_KINDS)?;
            let dependencies = dependencies(state, target);
            if !dependencies.is_empty() {
                return Err(CreatorRejectionV1::BlockingDependencies {
                    entities: dependencies,
                });
            }
            state.remove_unreferenced(target);
        }
        CreatorOpV1::ScheduleHazard {
            local,
            lane,
            hazard_id,
            start_tick,
            duration_ticks,
        } => {
            ensure_capacity(
                scheduled_or_active_hazard_count(state)?,
                MAX_HAZARDS,
                EntityKind::Hazard,
            )?;
            let lane = resolve(state, locals, *lane, &[EntityKind::Lane])?;
            let definition = catalog
                .hazard(hazard_id)
                .ok_or(CreatorRejectionV1::UnknownCatalogDefinition)?;
            if *start_tick < state.tick
                || *duration_ticks == 0
                || *duration_ticks > definition.duration_ticks()
                || start_tick.0.checked_add(*duration_ticks).is_none()
            {
                return Err(CreatorRejectionV1::InvalidValue);
            }
            let id = allocate(revision, EntityKind::Hazard, *local, state, locals)?;
            state.hazards.insert(
                id,
                HazardV1 {
                    lane,
                    hazard_id: hazard_id.clone(),
                    start_tick: *start_tick,
                    duration_ticks: *duration_ticks,
                    cancelled: false,
                },
            );
            register(*local, id, EntityKind::Hazard, locals, entities);
        }
        CreatorOpV1::CancelHazard { hazard } => {
            let hazard = resolve(state, locals, *hazard, &[EntityKind::Hazard])?;
            state
                .hazards
                .get_mut(&hazard)
                .ok_or(CreatorRejectionV1::UnknownEntity)?
                .cancelled = true;
        }
    }
    Ok(())
}

const OWNABLE_KINDS: [EntityKind; 8] = [
    EntityKind::System,
    EntityKind::Star,
    EntityKind::World,
    EntityKind::Lane,
    EntityKind::Deposit,
    EntityKind::Industry,
    EntityKind::Route,
    EntityKind::Hazard,
];
const NAMED_KINDS: [EntityKind; 4] = [
    EntityKind::Faction,
    EntityKind::System,
    EntityKind::Star,
    EntityKind::World,
];
const REMOVABLE_KINDS: [EntityKind; 9] = [
    EntityKind::Faction,
    EntityKind::System,
    EntityKind::Star,
    EntityKind::World,
    EntityKind::Lane,
    EntityKind::Deposit,
    EntityKind::Industry,
    EntityKind::Route,
    EntityKind::Hazard,
];

fn register(
    local: BatchLocalId,
    id: EntityId,
    kind: EntityKind,
    locals: &mut BTreeMap<BatchLocalId, (EntityId, EntityKind)>,
    entities: &mut BTreeMap<BatchLocalId, EntityId>,
) {
    locals.insert(local, (id, kind));
    entities.insert(local, id);
}

fn allocate(
    revision: RevisionId,
    kind: EntityKind,
    local: BatchLocalId,
    state: &WorkshopStateV1,
    locals: &BTreeMap<BatchLocalId, (EntityId, EntityKind)>,
) -> Result<EntityId, CreatorRejectionV1> {
    if locals.contains_key(&local) {
        return Err(CreatorRejectionV1::DuplicateLocalId);
    }
    let id = entity_id(revision, kind, local);
    if state.contains_entity(id) || locals.values().any(|(other, _)| *other == id) {
        return Err(CreatorRejectionV1::IdentityCollision);
    }
    Ok(id)
}

fn resolve(
    state: &WorkshopStateV1,
    locals: &BTreeMap<BatchLocalId, (EntityId, EntityKind)>,
    reference: ObjectRefV1,
    expected: &[EntityKind],
) -> Result<EntityId, CreatorRejectionV1> {
    match reference {
        ObjectRefV1::Existing(id) => match state.entity_kind(id) {
            None => Err(CreatorRejectionV1::UnknownEntity),
            Some(kind) if expected.contains(&kind) => Ok(id),
            Some(_) => Err(CreatorRejectionV1::WrongEntityKind),
        },
        ObjectRefV1::Local(local) => match locals.get(&local) {
            Some((id, kind))
                if expected.contains(kind) && state.entity_kind(*id) == Some(*kind) =>
            {
                Ok(*id)
            }
            _ => Err(CreatorRejectionV1::InvalidLocalReference),
        },
    }
}

fn ensure_capacity(
    current: usize,
    maximum: usize,
    kind: EntityKind,
) -> Result<(), CreatorRejectionV1> {
    if current >= maximum {
        Err(CreatorRejectionV1::Capacity { kind })
    } else {
        Ok(())
    }
}

fn scheduled_or_active_hazard_count(state: &WorkshopStateV1) -> Result<usize, CreatorRejectionV1> {
    state.hazards.values().try_fold(0_usize, |count, hazard| {
        let end = hazard
            .start_tick
            .0
            .checked_add(hazard.duration_ticks)
            .ok_or(CreatorRejectionV1::Arithmetic)?;
        if hazard.cancelled || end <= state.tick.0 {
            Ok(count)
        } else {
            count.checked_add(1).ok_or(CreatorRejectionV1::Arithmetic)
        }
    })
}

pub(crate) fn find_lane(
    state: &WorkshopStateV1,
    a: EntityId,
    b: EntityId,
) -> Option<(EntityId, &LaneV1)> {
    state
        .lanes
        .iter()
        .find(|(_, lane)| (lane.a == a && lane.b == b) || (lane.a == b && lane.b == a))
        .map(|(id, lane)| (*id, lane))
}

fn dependencies(state: &WorkshopStateV1, target: EntityId) -> Vec<EntityId> {
    let mut found = BTreeSet::new();
    if state.factions.contains_key(&target) {
        found.extend(
            state
                .owners
                .iter()
                .filter_map(|(entity, owner)| (*owner == target).then_some(*entity)),
        );
    }
    for (id, star) in &state.stars {
        if star.system == target {
            found.insert(*id);
        }
    }
    for (id, world) in &state.worlds {
        if world.system == target || world.primary == target {
            found.insert(*id);
        }
    }
    for (id, lane) in &state.lanes {
        if lane.a == target || lane.b == target {
            found.insert(*id);
        }
    }
    for (id, deposit) in &state.deposits {
        if deposit.world == target {
            found.insert(*id);
        }
    }
    for (id, industry) in &state.industries {
        if industry.world == target || industry.linked_deposit == Some(target) {
            found.insert(*id);
        }
    }
    for (id, route) in &state.routes {
        if route.source == target || route.destination == target {
            found.insert(*id);
        }
    }
    if let Some(lane) = state.lanes.get(&target) {
        for (id, route) in &state.routes {
            let Some(source) = state.worlds.get(&route.source) else {
                continue;
            };
            let Some(destination) = state.worlds.get(&route.destination) else {
                continue;
            };
            if (source.system == lane.a && destination.system == lane.b)
                || (source.system == lane.b && destination.system == lane.a)
            {
                found.insert(*id);
            }
        }
    }
    for (id, shipment) in &state.shipments {
        if shipment.route == target || shipment.source == target || shipment.destination == target {
            found.insert(*id);
        }
    }
    for (id, hazard) in &state.hazards {
        if hazard.lane == target {
            found.insert(*id);
        }
    }
    found.into_iter().collect()
}

pub(crate) fn revision_id(
    catalog: &ValidatedCatalogPackV1,
    genesis_seed: [u8; 32],
    parent: Option<RevisionId>,
    tick: WorkshopTick,
    ordinal: u64,
    batch_bytes: &[u8],
) -> RevisionId {
    let mut hash = Sha256::new();
    hash.update(b"NYON-WORKSHOP-REVISION-V1\0");
    hash.update(WORKSHOP_RULES_VERSION.to_le_bytes());
    hash.update(catalog.catalog_hash().0);
    hash.update(genesis_seed);
    match parent {
        Some(parent) => {
            hash.update([1]);
            hash.update(parent.0);
        }
        None => hash.update([0]),
    }
    hash.update(tick.0.to_le_bytes());
    hash.update(ordinal.to_le_bytes());
    hash.update((batch_bytes.len() as u64).to_le_bytes());
    hash.update(batch_bytes);
    RevisionId(hash.finalize().into())
}

pub(crate) fn entity_id(revision: RevisionId, kind: EntityKind, local: BatchLocalId) -> EntityId {
    let mut hash = Sha256::new();
    hash.update(b"NYON-WORKSHOP-ENTITY-V1\0");
    hash.update(revision.0);
    hash.update([kind as u8]);
    hash.update(local.0.to_le_bytes());
    let bytes: [u8; 32] = hash.finalize().into();
    let mut id = [0_u8; 16];
    id.copy_from_slice(&bytes[..16]);
    EntityId(id)
}
