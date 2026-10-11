const std = @import("std");
const ids = @import("ids.zig");
const role = @import("../play_role.zig");
const surface = @import("../surface.zig");

pub const Allocator = std.mem.Allocator;
pub const PlanetId = ids.PlanetId;
pub const SettlementId = ids.SettlementId;
pub const BuildingId = ids.BuildingId;
pub const ShipmentId = ids.ShipmentId;
pub const Tick = ids.Tick;
pub const Revision = ids.Revision;
pub const CommandSequence = ids.CommandSequence;
pub const JobId = ids.JobId;
pub const CellId = ids.CellId;
pub const CivilizationId = role.CivilizationId;
pub const SessionPolicy = role.SessionPolicy;
pub const Provenance = role.Provenance;
pub const Direction = surface.Direction;

pub const rules_identity = "nyon.planet-economy.v0";
pub const Capacities = struct {
    settlements: u16 = 64,
    buildings: u16 = 256,
    shipments: u16 = 64,
    history_receipts: u16 = 4096,
};
pub const Startup = enum(u8) { ruler_start = 0 };
pub const Config = struct {
    seed: u64,
    side: u16 = 8,
    capacities: Capacities = .{},
    startup: ?Startup = null,
};
pub const Resource = enum(u8) { food = 0, material = 1 };
pub const Terrain = enum(u8) { fertile = 0, rocky = 1 };
pub const BuildingKind = enum(u8) { farm = 0, quarry = 1, house = 2 };
pub const CargoSource = enum(u8) { transfer = 0, mint = 1 };
pub const Footprint = struct { len: u8, cells: [2]CellId };

pub const CommandTag = enum(u8) {
    author_settlement = 0,
    grant = 1,
    author_build = 2,
    author_ship = 3,
    found = 4,
    build = 5,
    ship = 6,
};
pub const Command = union(CommandTag) {
    author_settlement: struct { owner: CivilizationId, anchor: CellId, population: u8, food: u32, material: u32, economic_bypass: bool },
    grant: struct { settlement: SettlementId, food: u32, material: u32, population: u8, economic_bypass: bool },
    author_build: struct { settlement: SettlementId, kind: BuildingKind, anchor: CellId, facing: ?Direction, economic_bypass: bool },
    author_ship: struct { source: SettlementId, destination: SettlementId, resource: Resource, amount: u16, path: []const CellId, cargo_source: CargoSource, economic_bypass: bool },
    found: struct { source: SettlementId, anchor: CellId },
    build: struct { settlement: SettlementId, kind: BuildingKind, anchor: CellId, facing: ?Direction },
    ship: struct { source: SettlementId, destination: SettlementId, resource: Resource, amount: u16, path: []const CellId },
};
pub const CommandHeader = struct { sequence: CommandSequence, provenance: Provenance, actor: ?CivilizationId };
pub const Envelope = struct { bytes: []const u8 };
pub const WorkBudget = struct { remaining: u16 };
pub const Step = enum { pending, done };
pub const Phase = enum { prepare, commands, arrivals, construction, production, population, finalize, publish, cleanup, discard };
pub const PollStats = struct { phase: Phase, units_used: u16 };
pub const TickSummary = struct { tick: Tick, revision: Revision, accepted: u16, rejected: u16, chain_head: [32]u8 };
pub const Failure = error{
    OutOfMemory,
    CapacityUnsupported,
    InvalidEnvelope,
    InvalidPolicy,
    TooManyCommands,
    EnvelopeTooLarge,
    DuplicateSequence,
    SequenceAlreadyPublished,
    WrongRevision,
    Busy,
    InvalidBudget,
    StaleJob,
    StaleView,
    NotFound,
    InvalidPlanet,
    InvalidCell,
    HistoryFull,
    TickHorizon,
    IdExhausted,
    InvariantViolation,
    NotReady,
};
pub const Progress = union(enum) { pending: PollStats, published: TickSummary, canceled, failed: Failure, invalid_request: Failure, idle };
pub const CancelResult = enum { requested, already_published, stale_job };

pub const RejectReason = enum(u16) {
    CreatorForbidden = 1,
    RulerForbidden = 2,
    CivilizationRequired = 3,
    WrongCivilization = 4,
    InvalidActor = 5,
    InternalCommandOnly = 6,
    CommandProvenanceMismatch = 7,
    ExplicitBypassRequired = 8,
    UnknownCivilization = 9,
    UnknownSettlement = 10,
    WrongOwner = 11,
    InvalidPlanet = 12,
    InvalidCell = 13,
    InvalidAmount = 14,
    InvalidPopulation = 15,
    InvalidFacing = 16,
    TerrainMismatch = 17,
    OccupiedCell = 18,
    DisconnectedFootprint = 19,
    SameEndpoint = 20,
    InvalidPathLength = 21,
    RepeatedCell = 22,
    NonAdjacentPath = 23,
    ForeignPathCell = 24,
    SettlementCapacity = 25,
    BuildingCapacity = 26,
    SettlementBuildingCapacity = 27,
    FreightCapacity = 28,
    PopulationCapacity = 29,
    InventoryCapacity = 30,
    InsufficientFood = 31,
    InsufficientMaterial = 32,
    InsufficientPopulation = 33,
    TickHorizon = 34,
    IdExhausted = 35,
    InvalidGrant = 36,
    PathEndpointMismatch = 37,
};

pub const SettlementView = struct {
    revision: Revision,
    id: SettlementId,
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
pub const CellView = struct { revision: Revision, id: CellId, terrain: Terrain, settlement: ?SettlementId, building: ?BuildingId, is_hall: bool };
pub const Stats = struct { requested_bytes: usize, peak_pages: u16, live_pages: u16, pages_copied: u64, work_units: u64, history_used: u16, phase: ?Phase };
pub const ReceiptCursor = u16;
pub const ReceiptView = struct { len: u16, bytes: [1024]u8 };
pub const ReceiptPage = struct { written: u16, next: ?ReceiptCursor };

pub const EventTag = enum(u8) {
    genesis = 0,
    tick_input = 1,
    accepted = 2,
    rejected = 3,
    arrived = 4,
    completed = 5,
    produced = 6,
    idle = 7,
    supplied = 8,
    demography = 9,
    tick_complete = 10,
};
pub const IdleReason = enum(u8) { workforce = 0, output_capacity = 1 };
pub const DemographyKind = enum(u8) { growth = 0, starvation = 1, housing_blocked = 2 };
pub const ReceiptHeader = struct { tick: Tick, ordinal: u16, provenance: Provenance, sequence: ?CommandSequence };
pub const AcceptedEffect = union(CommandTag) {
    author_settlement: struct { settlement: SettlementId, food_minted: u32, material_minted: u32, population_minted: u8 },
    grant: struct { food_minted: u32, material_minted: u32, population_minted: u8 },
    author_build: struct { building: BuildingId, footprint: Footprint, material_waived: u32, ticks_waived: u8 },
    author_ship: struct { shipment: ShipmentId, transferred: u16, minted: u16, departure_tick: Tick, arrival_tick: Tick, reserved: u16 },
    found: struct { settlement: SettlementId, food_moved: u32, material_spent: u32, population_moved: u8 },
    build: struct { building: BuildingId, footprint: Footprint, material_spent: u32, remaining_ticks: u8 },
    ship: struct { shipment: ShipmentId, transferred: u16, reserved: u16, departure_tick: Tick, arrival_tick: Tick },
};
pub const ReceiptEvent = union(EventTag) {
    genesis: Config,
    tick_input: struct { target_tick: Tick, expected_revision: Revision, previous_policy: ?SessionPolicy, current_policy: SessionPolicy, envelope_count: u16 },
    accepted: struct { envelope: Envelope, effect: AcceptedEffect },
    rejected: struct { envelope: Envelope, reason: RejectReason, offending_index: ?u16 },
    arrived: struct { shipment: ShipmentId, source: SettlementId, destination: SettlementId, resource: Resource, quantity: u16, destination_stock_after: u32, destination_reservation_after: u32 },
    completed: struct { building: BuildingId, settlement: SettlementId, kind: BuildingKind },
    produced: struct { building: BuildingId, settlement: SettlementId, workers: u8, resource: Resource, quantity: u32, stock_after: u32 },
    idle: struct { building: BuildingId, settlement: SettlementId, reason: IdleReason, workers_needed: u8, workers_available: u8, resource: Resource, quantity: u32, stock_before: u32, incoming_reserved: u32 },
    supplied: struct { settlement: SettlementId, needed: u8, consumed: u8, shortage: u8, food_after: u32 },
    demography: struct { settlement: SettlementId, kind: DemographyKind, population_before: u8, population_after: u8, housing: u8, hunger_after: u8, fed_streak_after: u8 },
    tick_complete: struct { revision: Revision, last_sequence: CommandSequence, accepted: u16, rejected: u16, settlements: u16, buildings: u16, active_shipments: u16 },
};
pub const Receipt = struct { header: ReceiptHeader, event: ReceiptEvent };

// Snapshot for the acceptance stories, which each have at most one freight item.
pub const FreightView = union(enum) { none, transit: ids.CellId, delivered: ids.CellId };
