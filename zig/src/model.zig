//! Classic RulesV1 authority state, ported from src/game/model.rs.
//! Widths and tagged payloads follow the Rust model. World names are inline
//! values; only the fleet list owns allocator-backed memory.
const std = @import("std");

pub const WORLD_COUNT: usize = 7;
pub const DEFAULT_SEED: u64 = 0x4947_5731_2026_0902;

pub const WorldNameError = error{ Length, Spacing, Character };

pub const WorldName = struct {
    pub const MAX_BYTES: usize = 24;

    bytes: [MAX_BYTES]u8 = @splat(0),
    len: u8 = 0,

    pub fn init(value: []const u8) WorldNameError!WorldName {
        if (value.len == 0 or value.len > MAX_BYTES) return error.Length;
        if (value[0] == ' ' or value[value.len - 1] == ' ' or
            std.mem.indexOf(u8, value, "  ") != null) return error.Spacing;
        for (value) |byte| {
            if (!std.ascii.isUpper(byte) and !std.ascii.isDigit(byte) and
                byte != ' ' and byte != '.' and byte != '-' and byte != '/')
            {
                return error.Character;
            }
        }
        var result = WorldName{ .len = @intCast(value.len) };
        @memcpy(result.bytes[0..value.len], value);
        return result;
    }

    pub fn asSlice(self: *const WorldName) []const u8 {
        return self.bytes[0..self.len];
    }
};

// Zig's plain integer aliases retain the exact storage widths used by the
// reference's newtypes. Integer operations remain explicit at their call sites.
pub const Tick = u64;
pub const WorldId = u8;
pub const FleetId = u64;
pub const CommandSequence = u64;
pub const Energy = u64;
pub const Strength = u64;

pub const Faction = enum { union_faction, helix, choir };
pub const FieldKind = enum { atmosphere, hydrosphere, topology };
pub const FieldAdjustment = enum { increase, decrease };

pub const SectorPoint = struct { x: i32, y: i32 };

pub const FieldLevels = struct {
    atmosphere: u8,
    hydrosphere: u8,
    topology: u8,

    pub fn init(atmosphere: u8, hydrosphere: u8, topology: u8) FieldLevels {
        return .{ .atmosphere = atmosphere, .hydrosphere = hydrosphere, .topology = topology };
    }

    pub fn get(self: FieldLevels, field: FieldKind) u8 {
        return switch (field) {
            .atmosphere => self.atmosphere,
            .hydrosphere => self.hydrosphere,
            .topology => self.topology,
        };
    }

    pub fn set(self: *FieldLevels, field: FieldKind, value: u8) void {
        switch (field) {
            .atmosphere => self.atmosphere = value,
            .hydrosphere => self.hydrosphere = value,
            .topology => self.topology = value,
        }
    }
};

/// Immutable RulesV1 defaults. Changing frozen behavior requires a versioned
/// decision; tests must never replace expected vectors to conceal a change.
pub const RulesV1 = struct {
    tick_hz: u32 = 60,
    win_world_count: u8 = 5,
    ai_period_ticks: u64 = 60,
    hazard_first_tick: u64 = 2700,
    hazard_period_ticks: u64 = 2700,
    hazard_duration_ticks: u64 = 720,
    maximum_energy: Energy = 250_000,
    maximum_defense: Strength = 100_000,
    minimum_launch: Energy = 10_000,
    field_raise_cost: Energy = 20_000,
    field_lower_refund: Energy = 10_000,
    base_fleet_speed: u64 = 23,
};

pub const World = struct {
    id: WorldId,
    name: WorldName,
    position: SectorPoint,
    owner: ?Faction,
    defense: Strength,
    energy: Energy,
    base_output_per_second: Energy,
    base_regeneration_per_second: Strength,
    fields: FieldLevels,
    production_remainder: u64,
    regeneration_remainder: u64,
};

pub const Fleet = struct {
    id: FleetId,
    faction: Faction,
    source: WorldId,
    destination: WorldId,
    strength: Strength,
    route_length: u64,
    progress: u64,
    speed_per_second: u64,
    hydrosphere_level: u8,
    movement_remainder: u64,
};

pub const HazardKind = enum { ion_storm, gravity_tide };

pub const ActiveHazard = struct {
    event_index: u64,
    kind: HazardKind,
    affected_field: FieldKind,
    start: Tick,
    end: Tick,
};

pub const Outcome = union(enum) {
    faction_victory: struct { winner: Faction },
    player_eliminated,
};

pub const Phase = union(enum) {
    running,
    finished: struct { at: Tick, outcome: Outcome },
};

pub const GameCommand = union(enum) {
    launch: struct { source: WorldId, destination: WorldId },
    tune_field: struct { world: WorldId, field: FieldKind, adjustment: FieldAdjustment },
};

pub const CommandEnvelope = struct {
    tick: Tick,
    sequence: CommandSequence,
    command: GameCommand,
};

pub const Campaign = struct {
    rules_version: u32 = 1,
    generator_version: u32 = 1,
    seed: u64,
    next_tick: Tick = 0,
    phase: Phase = .running,
    worlds: [WORLD_COUNT]World,
    fleets: std.ArrayList(Fleet) = .empty,
    active_hazard: ?ActiveHazard = null,
    next_hazard_index: u64 = 0,
    next_fleet_id: FleetId = 0,

    /// Takes a value copy of validated scenario worlds, resetting campaign
    /// runtime state just as the reference's Campaign::from_scenario does.
    pub fn fromWorlds(seed: u64, worlds: [WORLD_COUNT]World) Campaign {
        return .{ .seed = seed, .worlds = worlds };
    }

    /// Deep copy of the only dynamically owned field. WorldName is already a
    /// value, so a cloned campaign never borrows labels from its source.
    pub fn clone(self: *const Campaign, allocator: std.mem.Allocator) std.mem.Allocator.Error!Campaign {
        var result = self.*;
        result.fleets = .empty;
        errdefer result.deinit(allocator);
        try result.fleets.appendSlice(allocator, self.fleets.items);
        return result;
    }

    pub fn deinit(self: *Campaign, allocator: std.mem.Allocator) void {
        self.fleets.deinit(allocator);
        self.* = undefined;
    }

    pub fn controlledCount(self: *const Campaign, faction: Faction) usize {
        var count: usize = 0;
        for (self.worlds) |world| {
            if (world.owner == faction) count += 1;
        }
        return count;
    }

    pub fn fleetCount(self: *const Campaign, faction: Faction) usize {
        var count: usize = 0;
        for (self.fleets.items) |fleet| {
            if (fleet.faction == faction) count += 1;
        }
        return count;
    }
};

test "world names preserve exact RulesV1 validation and own their bytes" {
    try std.testing.expectError(error.Length, WorldName.init(""));
    try std.testing.expectError(error.Length, WorldName.init("ABCDEFGHIJKLMNOPQRSTUVWXY"));
    try std.testing.expectError(error.Spacing, WorldName.init(" ASTER"));
    try std.testing.expectError(error.Spacing, WorldName.init("ASTER "));
    try std.testing.expectError(error.Spacing, WorldName.init("ASTER  VALE"));
    try std.testing.expectError(error.Spacing, WorldName.init(" ASTER?"));
    for ([_][]const u8{ "Aster", "ASTER?", "A\tB", "A\nB", "\xc3\x89" }) |invalid| {
        try std.testing.expectError(error.Character, WorldName.init(invalid));
    }
    const maximum = try WorldName.init("ABCDEFGHIJKLMNOPQRSTUVWX");
    try std.testing.expectEqual(@as(usize, 24), maximum.asSlice().len);
    const punctuation = try WorldName.init("NYON 123.4-A/B");
    try std.testing.expectEqualStrings("NYON 123.4-A/B", punctuation.asSlice());
    var bytes = [_]u8{ 'A', 'S', 'T', 'E', 'R' };
    const name = try WorldName.init(&bytes);
    bytes[0] = 'V';
    try std.testing.expectEqualStrings("ASTER", name.asSlice());
}

test "field accessors preserve all channels independently" {
    var fields = FieldLevels.init(6, 5, 4);
    try std.testing.expectEqual(@as(u8, 6), fields.get(.atmosphere));
    try std.testing.expectEqual(@as(u8, 5), fields.get(.hydrosphere));
    try std.testing.expectEqual(@as(u8, 4), fields.get(.topology));
    fields.set(.hydrosphere, 10);
    try std.testing.expectEqualDeep(FieldLevels.init(6, 10, 4), fields);
}

test "campaign clone owns independent fleets and names" {
    const allocator = std.testing.allocator;
    const world = World{
        .id = 0,
        .name = try WorldName.init("ASTER VALE"),
        .position = .{ .x = 1000, .y = 5000 },
        .owner = .union_faction,
        .defense = 40_000,
        .energy = 80_000,
        .base_output_per_second = 1800,
        .base_regeneration_per_second = 250,
        .fields = FieldLevels.init(3, 3, 3),
        .production_remainder = 0,
        .regeneration_remainder = 0,
    };
    var original = Campaign.fromWorlds(1, @splat(world));
    defer original.deinit(allocator);
    original.phase = .{ .finished = .{ .at = 12, .outcome = .{ .faction_victory = .{ .winner = .choir } } } };
    original.active_hazard = .{ .event_index = 7, .kind = .ion_storm, .affected_field = .atmosphere, .start = 3, .end = 723 };
    try original.fleets.append(allocator, .{
        .id = 1,
        .faction = .helix,
        .source = 0,
        .destination = 1,
        .strength = 20_000,
        .route_length = 2000,
        .progress = 10,
        .speed_per_second = 23,
        .hydrosphere_level = 5,
        .movement_remainder = 100,
    });
    var copy = try original.clone(allocator);
    defer copy.deinit(allocator);
    try std.testing.expectEqualDeep(original.worlds, copy.worlds);
    try std.testing.expectEqualDeep(original.phase, copy.phase);
    try std.testing.expectEqualDeep(original.active_hazard, copy.active_hazard);
    try std.testing.expectEqualDeep(original.fleets.items, copy.fleets.items);
    original.fleets.items[0].strength = 1;
    original.worlds[0].name = try WorldName.init("VESPER");
    try std.testing.expectEqual(@as(Strength, 20_000), copy.fleets.items[0].strength);
    try std.testing.expectEqualStrings("ASTER VALE", copy.worlds[0].name.asSlice());
    try std.testing.expectEqual(@as(usize, 7), copy.controlledCount(.union_faction));
    try std.testing.expectEqual(@as(usize, 1), copy.fleetCount(.helix));
    try std.testing.expectEqual(@as(usize, 0), copy.fleetCount(.choir));
}
