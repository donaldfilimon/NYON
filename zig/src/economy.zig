const std = @import("std");
const model = @import("model.zig");

/// Advance one owned world's production and defense by a fixed RulesV1 tick.
/// A hazard halves only its field's bonus. Integer remainders preserve the
/// fractional rate until the corresponding resource reaches its cap.
pub fn accrueWorld(world: *model.World, rules: model.RulesV1, affected_field: ?model.FieldKind) void {
    if (rules.tick_hz == 0) @panic("RulesV1.tick_hz must be nonzero");
    if (world.owner == null) return;
    const divisor = @as(u128, rules.tick_hz) * 10_000;

    var atmosphere_bonus = @as(u64, world.fields.atmosphere) * 500;
    if (affected_field == .atmosphere) atmosphere_bonus = atmosphere_bonus * 5_000 / 10_000;
    const production_numerator = @as(u128, world.production_remainder) +
        @as(u128, world.base_output_per_second) * @as(u128, 10_000 + atmosphere_bonus);
    // Rust's u128 as u64 truncates. Preserve that behavior even for authored
    // states outside the canonical resource range.
    const production_gain: u64 = @truncate(production_numerator / divisor);
    if (world.energy >= rules.maximum_energy or world.energy +| production_gain >= rules.maximum_energy) {
        world.energy = rules.maximum_energy;
        world.production_remainder = 0;
    } else {
        world.energy += production_gain;
        world.production_remainder = @intCast(production_numerator % divisor);
    }

    var topology_bonus = @as(u64, world.fields.topology) * 100;
    if (affected_field == .topology) topology_bonus = topology_bonus * 5_000 / 10_000;
    const regeneration_rate = world.base_regeneration_per_second +| topology_bonus;
    const regeneration_numerator = @as(u128, world.regeneration_remainder) +
        @as(u128, regeneration_rate) * 10_000;
    const regeneration_gain: u64 = @truncate(regeneration_numerator / divisor);
    if (world.defense >= rules.maximum_defense or world.defense +| regeneration_gain >= rules.maximum_defense) {
        world.defense = rules.maximum_defense;
        world.regeneration_remainder = 0;
    } else {
        world.defense += regeneration_gain;
        world.regeneration_remainder = @intCast(regeneration_numerator % divisor);
    }
}

fn fixture() !model.World {
    return .{
        .id = 0,
        .name = try model.WorldName.init("ASTER VALE"),
        .position = .{ .x = 1037, .y = 5027 },
        .owner = .union_faction,
        .defense = 43_735,
        .energy = 80_000,
        .base_output_per_second = 2904,
        .base_regeneration_per_second = 488,
        .fields = .{ .atmosphere = 6, .hydrosphere = 7, .topology = 3 },
        .production_remainder = 0,
        .regeneration_remainder = 0,
    };
}

test "sixty fixed ticks preserve fractional production and regeneration" {
    var world = try fixture();
    for (0..60) |_| accrueWorld(&world, .{}, null);
    try std.testing.expectEqual(@as(u64, 83_775), world.energy);
    try std.testing.expectEqual(@as(u64, 120_000), world.production_remainder);
    try std.testing.expectEqual(@as(u64, 44_523), world.defense);
    try std.testing.expectEqual(@as(u64, 0), world.regeneration_remainder);
}

test "atmosphere hazard halves only the production field bonus" {
    var ordinary = try fixture();
    ordinary.energy = 0;
    ordinary.base_output_per_second = 600;
    ordinary.fields.atmosphere = 10;
    var hazardous = ordinary;
    accrueWorld(&ordinary, .{}, null);
    accrueWorld(&hazardous, .{}, .atmosphere);
    try std.testing.expectEqual(@as(u64, 15), ordinary.energy);
    try std.testing.expectEqual(@as(u64, 12), hazardous.energy);
    try std.testing.expectEqual(@as(u64, 300_000), hazardous.production_remainder);
    try std.testing.expectEqual(ordinary.defense, hazardous.defense);
    try std.testing.expectEqual(ordinary.regeneration_remainder, hazardous.regeneration_remainder);
}

test "topology hazard halves additive defense bonus and leaves production intact" {
    var ordinary = try fixture();
    ordinary.defense = 0;
    ordinary.base_regeneration_per_second = 600;
    ordinary.fields.topology = 10;
    var hazardous = ordinary;
    for (0..60) |_| {
        accrueWorld(&ordinary, .{}, null);
        accrueWorld(&hazardous, .{}, .topology);
    }
    try std.testing.expectEqual(@as(u64, 1600), ordinary.defense);
    try std.testing.expectEqual(@as(u64, 1100), hazardous.defense);
    try std.testing.expectEqual(ordinary.energy, hazardous.energy);
    try std.testing.expectEqual(ordinary.production_remainder, hazardous.production_remainder);
}

test "hydrosphere hazard leaves world growth intact" {
    var ordinary = try fixture();
    var hazardous = ordinary;
    accrueWorld(&ordinary, .{}, null);
    accrueWorld(&hazardous, .{}, .hydrosphere);
    try std.testing.expectEqualDeep(ordinary, hazardous);
}

test "neutral worlds retain resource values and fractional remainders" {
    var world = try fixture();
    world.owner = null;
    world.production_remainder = 552_000;
    world.regeneration_remainder = 80_000;
    const before = world;
    accrueWorld(&world, .{}, .atmosphere);
    try std.testing.expectEqualDeep(before, world);
}

test "reaching resource caps clears fractions and repairs above-cap authored states" {
    var world = try fixture();
    world.energy = 249_999;
    world.defense = 99_999;
    world.production_remainder = 599_999;
    world.regeneration_remainder = 599_999;
    accrueWorld(&world, .{}, null);
    try std.testing.expectEqual(@as(u64, 250_000), world.energy);
    try std.testing.expectEqual(@as(u64, 100_000), world.defense);
    try std.testing.expectEqual(@as(u64, 0), world.production_remainder);
    try std.testing.expectEqual(@as(u64, 0), world.regeneration_remainder);
    world.energy = std.math.maxInt(u64);
    world.defense = std.math.maxInt(u64);
    accrueWorld(&world, .{}, null);
    try std.testing.expectEqual(@as(u64, 250_000), world.energy);
    try std.testing.expectEqual(@as(u64, 100_000), world.defense);
}

test "resource numerators retain u128 precision and defense rate saturates" {
    var world = try fixture();
    world.energy = 0;
    world.defense = 0;
    world.base_output_per_second = std.math.maxInt(u64);
    world.base_regeneration_per_second = std.math.maxInt(u64);
    world.fields.atmosphere = 0;
    world.fields.topology = 10;
    const rules: model.RulesV1 = .{
        .tick_hz = 60,
        .maximum_energy = std.math.maxInt(u64),
        .maximum_defense = std.math.maxInt(u64),
    };
    accrueWorld(&world, rules, null);
    try std.testing.expectEqual(std.math.maxInt(u64) / 60, world.energy);
    try std.testing.expectEqual(std.math.maxInt(u64) / 60, world.defense);
    try std.testing.expectEqual(@as(u64, 150_000), world.production_remainder);
    try std.testing.expectEqual(@as(u64, 150_000), world.regeneration_remainder);
}

test "nondefault tick rates preserve whole-second rates exactly" {
    var world = try fixture();
    const rules: model.RulesV1 = .{ .tick_hz = 7 };
    for (0..7) |_| accrueWorld(&world, rules, null);
    try std.testing.expectEqual(@as(u64, 83_775), world.energy);
    try std.testing.expectEqual(@as(u64, 14_000), world.production_remainder);
    try std.testing.expectEqual(@as(u64, 44_523), world.defense);
    try std.testing.expectEqual(@as(u64, 0), world.regeneration_remainder);
}

test "production narrowing retains Rust u128 as u64 semantics" {
    var world = try fixture();
    world.energy = 0;
    world.base_output_per_second = std.math.maxInt(u64);
    world.fields.atmosphere = 10;
    const rules: model.RulesV1 = .{ .tick_hz = 1, .maximum_energy = std.math.maxInt(u64) };
    accrueWorld(&world, rules, null);
    try std.testing.expectEqual(@as(u64, 9_223_372_036_854_775_806), world.energy);
    try std.testing.expectEqual(@as(u64, 5000), world.production_remainder);
}
