pub const SystemId = enum(u32) { _ };
pub const PlanetId = enum(u32) { _ };
pub const CivilizationId = @import("../play_role.zig").CivilizationId;

pub const rules_identity = "nyon.seeded-galaxy.v0";

pub const Config = struct {
    seed: u64 = 0,
    systems: u16 = 12,
    planets: u16 = 24,
    civilizations: u16 = 4,
    side: u16 = 8,
    version: u16 = 1,
};
pub const System = struct { id: SystemId, x: i32, y: i32, z: i32 };
pub const Planet = struct {
    id: PlanetId,
    system: SystemId,
    orbit_index: u16,
    terrain_seed: u64,
    side: u16,
};
pub const Civilization = struct {
    id: CivilizationId,
    founding_planet: PlanetId,
    kind: enum(u8) { uninitialized = 0 },
};
pub const Failure = error{
    InvalidVersion,
    CapacityUnsupported,
    InvalidConfig,
    InvalidPolicy,
    OutOfMemory,
    InvalidBudget,
    Busy,
    StaleJob,
    NotReady,
    Shutdown,
    ArithmeticOverflow,
    InvariantViolation,
    IdExhausted,
    InvalidWire,
    UnsupportedWireVersion,
};
pub const Phase = enum(u8) {
    validate_config,
    parse_header,
    systems,
    planets,
    civilizations,
    validate_records,
    encode_header,
    encode_records,
    hash_blocks,
    finalize_digest,
    retire_output,
    retire_scratch,
    ready,
    canceled,
    failed,
    taken,
};
pub const Stats = struct {
    phase: Phase,
    cursor: u16,
    units_used: u16,
    total_units: u64,
    requested_bytes: usize,
    admission_copy_bytes: u16,
};
pub const Progress = union(enum) {
    pending: Stats,
    ready: Stats,
    canceled,
    failed: Failure,
};
pub const Page = struct { written: u8, next: ?u16 };
pub const WireRecord = struct { len: u16, bytes: [254]u8 };

pub fn validateConfig(config: Config) Failure!void {
    if (config.version != 1) return error.InvalidVersion;
    if (config.systems < 1 or config.systems > 64 or
        config.planets < 1 or config.planets > 512 or
        config.civilizations < 1 or config.civilizations > 16 or
        config.side < 4 or config.side > 32) return error.CapacityUnsupported;
    if (config.systems > config.planets or config.civilizations > config.planets)
        return error.InvalidConfig;
}

pub const Token = struct { owner_namespace: u64, id: u64, generation: u64 };
