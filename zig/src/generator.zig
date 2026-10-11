//! Frozen RulesV1 generator, ported from Campaign::new in src/game/model.rs.
const std = @import("std");
const model = @import("model.zig");

pub const WORLD_NAMES: [model.WORLD_COUNT][]const u8 = .{
    "ASTER VALE", "KHEPRI", "MERIDIAN", "VESPER", "ORISON", "NACRE", "UMBRA",
};

const WORLD_ANCHORS: [model.WORLD_COUNT]model.SectorPoint = .{
    .{ .x = 1000, .y = 5000 },
    .{ .x = 9000, .y = 2500 },
    .{ .x = 9000, .y = 7500 },
    .{ .x = 5000, .y = 1000 },
    .{ .x = 5000, .y = 9000 },
    .{ .x = 4000, .y = 4000 },
    .{ .x = 6000, .y = 6000 },
};

const INITIAL_OWNERS: [model.WORLD_COUNT]?model.Faction = .{
    .union_faction, .helix, .choir, null, null, null, null,
};

pub const SplitMix64 = struct {
    state: u64,

    pub fn init(seed: u64) SplitMix64 {
        return .{ .state = seed };
    }

    pub fn next(self: *SplitMix64) u64 {
        self.state +%= 0x9E37_79B9_7F4A_7C15;
        var value = self.state;
        value = (value ^ (value >> 30)) *% 0xBF58_476D_1CE4_E5B9;
        value = (value ^ (value >> 27)) *% 0x94D0_49BB_1331_11EB;
        return value ^ (value >> 31);
    }
};

/// The simulation uses the same single draw to select a hazard's affected field.
pub fn splitmix64Once(seed: u64) u64 {
    var generator = SplitMix64.init(seed);
    return generator.next();
}

/// Rules are intentionally unused in the frozen generator, matching Rust.
/// Changes to simulation rule values must not alter genesis draw order.
pub fn generate(seed: u64, rules: model.RulesV1) model.Campaign {
    _ = rules;
    var generator = SplitMix64.init(seed);
    var worlds: [model.WORLD_COUNT]model.World = undefined;
    for (&worlds, 0..) |*world, index| {
        const anchor = WORLD_ANCHORS[index];
        const x_jitter: i32 = @as(i32, @intCast(generator.next() % 1001)) - 500;
        const y_jitter: i32 = @as(i32, @intCast(generator.next() % 1001)) - 500;
        const base_output_per_second = 1800 + generator.next() % 1201;
        const defense = 40_000 + generator.next() % 20_001;
        const base_regeneration_per_second = 250 + generator.next() % 251;
        // Separate statements preserve the explicit reference draw order.
        const atmosphere: u8 = @intCast(3 + generator.next() % 5);
        const hydrosphere: u8 = @intCast(3 + generator.next() % 5);
        const topology: u8 = @intCast(3 + generator.next() % 5);
        const owner = INITIAL_OWNERS[index];
        world.* = .{
            .id = @intCast(index),
            .name = model.WorldName.init(WORLD_NAMES[index]) catch unreachable,
            .position = .{ .x = anchor.x + x_jitter, .y = anchor.y + y_jitter },
            .owner = owner,
            .defense = defense,
            .energy = if (owner != null) 80_000 else 0,
            .base_output_per_second = base_output_per_second,
            .base_regeneration_per_second = base_regeneration_per_second,
            .fields = model.FieldLevels.init(atmosphere, hydrosphere, topology),
            .production_remainder = 0,
            .regeneration_remainder = 0,
        };
    }
    return model.Campaign.fromWorlds(seed, worlds);
}

test "default seed matches the frozen Rust generator vector" {
    // Literal values come from src/game/model.rs::default_seed_matches_the_frozen_generator_vector.
    const Expected = struct { position: model.SectorPoint, output: u64, defense: u64, regeneration: u64, fields: model.FieldLevels };
    const expected = [_]Expected{
        .{ .position = .{ .x = 1037, .y = 5027 }, .output = 2904, .defense = 43735, .regeneration = 488, .fields = model.FieldLevels.init(6, 7, 3) },
        .{ .position = .{ .x = 8793, .y = 2818 }, .output = 2678, .defense = 44244, .regeneration = 463, .fields = model.FieldLevels.init(5, 6, 7) },
        .{ .position = .{ .x = 8517, .y = 7827 }, .output = 2246, .defense = 43986, .regeneration = 405, .fields = model.FieldLevels.init(6, 3, 7) },
        .{ .position = .{ .x = 4811, .y = 1320 }, .output = 2591, .defense = 58596, .regeneration = 454, .fields = model.FieldLevels.init(7, 7, 6) },
        .{ .position = .{ .x = 4905, .y = 8955 }, .output = 2848, .defense = 46169, .regeneration = 275, .fields = model.FieldLevels.init(7, 4, 7) },
        .{ .position = .{ .x = 3746, .y = 3709 }, .output = 1944, .defense = 43254, .regeneration = 405, .fields = model.FieldLevels.init(5, 4, 3) },
        .{ .position = .{ .x = 5925, .y = 6136 }, .output = 2998, .defense = 45102, .regeneration = 474, .fields = model.FieldLevels.init(6, 7, 7) },
    };
    var campaign = generate(model.DEFAULT_SEED, .{});
    defer campaign.deinit(std.testing.allocator);
    for (campaign.worlds, expected, 0..) |world, vector, index| {
        try std.testing.expectEqual(@as(model.WorldId, @intCast(index)), world.id);
        try std.testing.expectEqualStrings(WORLD_NAMES[index], world.name.asSlice());
        try std.testing.expectEqualDeep(vector.position, world.position);
        try std.testing.expectEqual(vector.output, world.base_output_per_second);
        try std.testing.expectEqual(vector.defense, world.defense);
        try std.testing.expectEqual(vector.regeneration, world.base_regeneration_per_second);
        try std.testing.expectEqualDeep(vector.fields, world.fields);
        try std.testing.expectEqual(INITIAL_OWNERS[index], world.owner);
        try std.testing.expectEqual(@as(u64, if (index < 3) 80_000 else 0), world.energy);
        try std.testing.expectEqual(@as(u64, 0), world.production_remainder);
        try std.testing.expectEqual(@as(u64, 0), world.regeneration_remainder);
    }
    try std.testing.expectEqual(@as(usize, 1), campaign.controlledCount(.union_faction));
    try std.testing.expectEqual(@as(usize, 1), campaign.controlledCount(.helix));
    try std.testing.expectEqual(@as(usize, 1), campaign.controlledCount(.choir));
    try std.testing.expectEqual(@as(usize, 0), campaign.fleets.items.len);
    try std.testing.expectEqual(@as(u64, 0), campaign.next_tick);
    try std.testing.expectEqualDeep(model.Phase.running, campaign.phase);
    try std.testing.expectEqual(@as(?model.ActiveHazard, null), campaign.active_hazard);
    try std.testing.expectEqual(@as(u64, 0), campaign.next_hazard_index);
    try std.testing.expectEqual(@as(u64, 0), campaign.next_fleet_id);
}

test "SplitMix64 has exact wrapping draws" {
    // Independent standard SplitMix64 seed-zero vector pins wrapping arithmetic.
    var generator = SplitMix64.init(0);
    try std.testing.expectEqual(@as(u64, 0xe220a8397b1dcdaf), generator.next());
    try std.testing.expectEqual(@as(u64, 0x6e789e6aa1b965f4), generator.next());
    try std.testing.expectEqual(@as(u64, 0x06c45d188009454f), generator.next());
    try std.testing.expectEqual(@as(u64, 0xe220a8397b1dcdaf), splitmix64Once(0));
}

test "genesis is value owned and independent of simulation rules" {
    var baseline = generate(model.DEFAULT_SEED, .{});
    defer baseline.deinit(std.testing.allocator);
    var changed_rules = generate(model.DEFAULT_SEED, .{ .tick_hz = 1, .base_fleet_speed = 999, .maximum_energy = 1 });
    defer changed_rules.deinit(std.testing.allocator);
    try std.testing.expectEqualDeep(baseline.worlds, changed_rules.worlds);
    changed_rules.worlds[0].name = try model.WorldName.init("OTHER WORLD");
    try std.testing.expectEqualStrings("ASTER VALE", baseline.worlds[0].name.asSlice());
    for ([_]u64{ 0, 1, std.math.maxInt(u64) }) |seed| {
        var campaign = generate(seed, .{});
        defer campaign.deinit(std.testing.allocator);
        for (campaign.worlds, WORLD_ANCHORS) |world, anchor| {
            try std.testing.expect(world.position.x >= anchor.x - 500 and world.position.x <= anchor.x + 500);
            try std.testing.expect(world.position.y >= anchor.y - 500 and world.position.y <= anchor.y + 500);
            try std.testing.expect(world.base_output_per_second >= 1800 and world.base_output_per_second <= 3000);
            try std.testing.expect(world.defense >= 40_000 and world.defense <= 60_000);
            try std.testing.expect(world.base_regeneration_per_second >= 250 and world.base_regeneration_per_second <= 500);
            for ([_]model.FieldKind{ .atmosphere, .hydrosphere, .topology }) |field| {
                try std.testing.expect(world.fields.get(field) >= 3 and world.fields.get(field) <= 7);
            }
        }
    }
}
