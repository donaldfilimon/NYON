const std = @import("std");
const ids = @import("ids.zig");
const role = @import("../play_role.zig");
const island = @import("../planet_economy/types.zig");

pub const Allocator = std.mem.Allocator;
pub const SystemId = ids.SystemId;
pub const PlanetId = ids.PlanetId;
pub const SettlementId = ids.SettlementId;
pub const BuildingId = ids.BuildingId;
pub const ShipmentId = ids.ShipmentId;
pub const Tick = ids.Tick;
pub const Revision = ids.Revision;
pub const CommandSequence = ids.CommandSequence;
pub const JobId = ids.JobId;
pub const DescriptorGeneration = ids.DescriptorGeneration;
pub const CellId = ids.CellId;
pub const SettlementRef = ids.SettlementRef;
pub const BuildingRef = ids.BuildingRef;
pub const ShipmentRef = ids.ShipmentRef;
pub const CivilizationId = role.CivilizationId;
pub const SessionPolicy = role.SessionPolicy;
pub const Provenance = role.Provenance;
pub const Role = role.Role;

pub const rules_identity = "nyon.coordinated-galaxy.v0";
pub const PlanetCapacities = struct {
    settlements: u16 = 64,
    buildings: u16 = 256,
    freight: u16 = 64,
    history_receipts: u16 = 4096,
};
pub const PlanetConfig = struct {
    system: SystemId = @fromBackingInt(1),
    side: u16 = 8,
    capacities: PlanetCapacities = .{},
};
pub const Config = struct {
    seed: u64,
    system_count: u8 = 1,
    civilization_count: u8 = 2,
    planet_count: u8 = 2,
    global_history_capacity: u16 = 4096,
    planets: [4]PlanetConfig = .{ .{}, .{}, .{}, .{} },
};

pub const Failure = error{
    OutOfMemory,
    CapacityUnsupported,
    InvalidConfig,
    InvalidEnvelope,
    InvalidPolicy,
    InvalidBudget,
    TooManyCommands,
    EnvelopeTooLarge,
    DuplicateSequence,
    SequenceAlreadyPublished,
    WrongRevision,
    WrongTick,
    Busy,
    NotReady,
    StaleJob,
    StaleView,
    ViewCapacity,
    Shutdown,
    CleanupFault,
    NotFound,
    InvalidPlanet,
    InvalidCell,
    HistoryFull,
    TickHorizon,
    IdExhausted,
    InvariantViolation,
};
pub const WorkBudget = island.WorkBudget;
pub const Step = enum { pending, done };
pub const Phase = enum {
    prepare,
    commands,
    arrivals,
    construction,
    production,
    population,
    local_finalize,
    global_finalize,
    install,
    retire,
};
pub const RoutedEnvelope = struct { planet: PlanetId, body: []const u8 };
pub const EnvelopeHeader = struct { sequence: CommandSequence, provenance: Provenance, actor: ?CivilizationId };
pub const RootHandle = struct { slot: u8, generation: u64 };
pub const PreparedRoot = struct {
    planet: PlanetId,
    root: RootHandle,
    tick: Tick,
    revision: Revision,
    history_count: u16,
    history_head: [32]u8,
};
pub const PlanetDescriptor = struct {
    planet: PlanetId,
    root: RootHandle,
    history_count: u16,
    history_head: [32]u8,
};
pub const Descriptor = struct {
    accepted: u16 = 0,
    rejected: u16 = 0,
    tick: Tick,
    revision: Revision,
    last_sequence: CommandSequence,
    policy: ?SessionPolicy,
    history_count: u16,
    history_head: [32]u8,
    planet_count: u8,
    planets: [4]PlanetDescriptor,
};
pub const ViewToken = struct {
    // Noncanonical process-local owner lifetime, independent of slot reuse.
    owner_generation: u64 = 0,
    descriptor_slot: u8,
    descriptor_generation: DescriptorGeneration,
    lease_slot: u8,
    lease_generation: u64,
};

pub const Resource = island.Resource;
pub const Terrain = island.Terrain;
pub const BuildingKind = island.BuildingKind;
pub const CargoSource = island.CargoSource;
pub const Direction = island.Direction;
pub const CommandTag = island.CommandTag;
pub const RejectReason = island.RejectReason;
pub const IdleReason = island.IdleReason;
pub const DemographyKind = island.DemographyKind;
pub const Footprint = struct { len: u8, cells: [2]CellId };
pub const Command = union(CommandTag) {
    author_settlement: struct { owner: CivilizationId, anchor: CellId, population: u8, food: u32, material: u32, economic_bypass: bool },
    grant: struct { settlement: SettlementId, food: u32, material: u32, population: u8, economic_bypass: bool },
    author_build: struct { settlement: SettlementId, kind: BuildingKind, anchor: CellId, facing: ?Direction, economic_bypass: bool },
    author_ship: struct { source: SettlementId, destination: SettlementId, resource: Resource, amount: u16, path: []const CellId, cargo_source: CargoSource, economic_bypass: bool },
    found: struct { source: SettlementId, anchor: CellId },
    build: struct { settlement: SettlementId, kind: BuildingKind, anchor: CellId, facing: ?Direction },
    ship: struct { source: SettlementId, destination: SettlementId, resource: Resource, amount: u16, path: []const CellId },
};

pub const SettlementValue = struct {
    revision: Revision,
    id: SettlementRef,
    owner: CivilizationId,
    hall: CellId,
    population: u8,
    food: u32,
    material: u32,
    incoming_food: u32,
    incoming_material: u32,
    housing: u8,
    hunger: u8,
    fed_streak: u8,
};
pub const CellValue = struct {
    revision: Revision,
    id: CellId,
    terrain: Terrain,
    settlement: ?SettlementRef,
    building: ?BuildingRef,
    is_hall: bool,
};
pub const ReceiptValue = struct { len: u16, bytes: [2048]u8 };
pub const LocalReceiptValue = struct { len: u16, bytes: [1024]u8 };
pub const ReceiptCursor = u16;
pub const ReceiptPage = struct { written: u16, next: ?ReceiptCursor };

pub const EventTag = enum(u8) { genesis = 0, tick_input = 1, routed_input = 2, planet_prepared = 3, tick_commit = 4 };
pub const ReceiptHeader = struct { tick: Tick, ordinal: u16 };
pub const PlanetEntry = struct {
    planet: PlanetId,
    system: SystemId,
    derived_seed: u64,
    side: u16,
    capacities: PlanetCapacities,
    local_genesis_head: [32]u8,
};
pub const CommitEntry = struct { planet: PlanetId, tick: Tick, revision: Revision, local_history_count: u16, local_head: [32]u8 };
pub const ReceiptEvent = union(EventTag) {
    genesis: struct { config: Config, planets: [4]PlanetEntry },
    tick_input: struct { target: Tick, expected_revision: Revision, previous_policy: ?SessionPolicy, current_policy: SessionPolicy, count: u16 },
    routed_input: RoutedEnvelope,
    planet_prepared: struct { planet: PlanetId, local_history_count: u16, local_head: [32]u8, accepted: u16, rejected: u16 },
    tick_commit: struct { revision: Revision, last_sequence: CommandSequence, accepted: u16, rejected: u16, planet_count: u8, planets: [4]CommitEntry },
};
pub const Receipt = struct { header: ReceiptHeader, event: ReceiptEvent };

pub const LocalEventTag = island.EventTag;
pub const LocalReceiptHeader = struct { tick: Tick, ordinal: u16, provenance: Provenance, sequence: ?CommandSequence };
pub const AcceptedEffect = union(CommandTag) {
    author_settlement: struct { settlement: SettlementId, food_minted: u32, material_minted: u32, population_minted: u8 },
    grant: struct { food_minted: u32, material_minted: u32, population_minted: u8 },
    author_build: struct { building: BuildingId, footprint: Footprint, material_waived: u32, ticks_waived: u8 },
    author_ship: struct { shipment: ShipmentId, transferred: u16, minted: u16, departure_tick: Tick, arrival_tick: Tick, reserved: u16 },
    found: struct { settlement: SettlementId, food_moved: u32, material_spent: u32, population_moved: u8 },
    build: struct { building: BuildingId, footprint: Footprint, material_spent: u32, remaining_ticks: u8 },
    ship: struct { shipment: ShipmentId, transferred: u16, reserved: u16, departure_tick: Tick, arrival_tick: Tick },
};
pub const LocalReceiptEvent = union(LocalEventTag) {
    genesis: struct { planet: PlanetId, system: SystemId, seed: u64, side: u16, capacities: PlanetCapacities, civilization_count: u8 },
    tick_input: struct { target_tick: Tick, expected_revision: Revision, previous_policy: ?SessionPolicy, current_policy: SessionPolicy, envelope_count: u16 },
    accepted: struct { envelope: []const u8, effect: AcceptedEffect },
    rejected: struct { envelope: []const u8, reason: RejectReason, offending_index: ?u16 },
    arrived: struct { shipment: ShipmentId, source: SettlementId, destination: SettlementId, resource: Resource, quantity: u16, destination_stock_after: u32, destination_reservation_after: u32 },
    completed: struct { building: BuildingId, settlement: SettlementId, kind: BuildingKind },
    produced: struct { building: BuildingId, settlement: SettlementId, workers: u8, resource: Resource, quantity: u32, stock_after: u32 },
    idle: struct { building: BuildingId, settlement: SettlementId, reason: IdleReason, workers_needed: u8, workers_available: u8, resource: Resource, quantity: u32, stock_before: u32, incoming_reserved: u32 },
    supplied: struct { settlement: SettlementId, needed: u8, consumed: u8, shortage: u8, food_after: u32 },
    demography: struct { settlement: SettlementId, kind: DemographyKind, population_before: u8, population_after: u8, housing: u8, hunger_after: u8, fed_streak_after: u8 },
    tick_complete: struct { revision: Revision, last_sequence: CommandSequence, accepted: u16, rejected: u16, settlements: u16, buildings: u16, active_shipments: u16 },
};
pub const LocalReceipt = struct { header: LocalReceiptHeader, event: LocalReceiptEvent };

pub const Stats = struct {
    phase: ?Phase,
    planet: ?PlanetId = null,
    cursor: u32 = 0,
    units_used: u16 = 0,
    work_units: u64 = 0,
    requested_bytes: usize = 0,
    peak_pages: u32 = 0,
    copied_pages: u64 = 0,
    global_history_used: u16 = 0,
    local_history_used: [4]u16 = @splat(0),
    obsolete_leases: u8 = 0,
    live_leases: u8 = 0,
    installed_planets: u8 = 0,
    accepted: u16 = 0,
    rejected: u16 = 0,
};
pub const Summary = struct {
    tick: Tick,
    revision: Revision,
    last_sequence: CommandSequence,
    accepted: u16,
    rejected: u16,
    chain_head: [32]u8,
};
pub const Progress = union(enum) {
    pending: Stats,
    published: Summary,
    waiting_for_views,
    canceled,
    failed: Failure,
    faulted: struct { job: JobId, cause: Failure },
    invalid_request: Failure,
    idle,
};
pub const CancelResult = enum { requested, already_published, stale_job, faulted };
pub const CreateProgress = union(enum) { pending: Stats, ready, canceled, failed: Failure };
pub const ReplayProgress = union(enum) { pending: Stats, complete, canceled, failed: Failure };
pub const StoryProgress = union(enum) { pending: Stats, complete, canceled, failed: Failure };

pub fn validateConfig(config: Config) Failure!void {
    if (config.system_count < 1 or config.system_count > 4 or
        config.civilization_count < 1 or config.civilization_count > 4 or
        config.planet_count < 1 or config.planet_count > 4 or
        config.global_history_capacity < 1 or config.global_history_capacity > 4096)
        return error.InvalidConfig;
    if (config.system_count > config.planet_count) return error.InvalidConfig;
    var seen: [4]bool = @splat(false);
    for (config.planets[0..config.planet_count]) |planet| {
        const system = @backingInt(planet.system);
        if (system < 1 or system > config.system_count) return error.InvalidConfig;
        seen[system - 1] = true;
        const cap = planet.capacities;
        if (planet.side < 4 or planet.side > 32 or
            cap.settlements < 1 or cap.settlements > 64 or
            cap.buildings < 1 or cap.buildings > 256 or
            cap.freight < 1 or cap.freight > 64 or
            cap.history_receipts < 1 or cap.history_receipts > 4096)
            return error.InvalidConfig;
    }
    for (seen[0..config.system_count]) |present| if (!present) return error.InvalidConfig;
}

pub fn planetSeed(seed: u64, planet: PlanetId) u64 {
    return seed ^ ((@as(u64, @backingInt(planet)) - 1) *% 0x9E3779B97F4A7C15);
}

pub fn terrain(seed: u64, cell: CellId) Terrain {
    return if (((planetSeed(seed, cell.planet) % 4) + (cell.local % 4)) % 4 == 0) .rocky else .fertile;
}
