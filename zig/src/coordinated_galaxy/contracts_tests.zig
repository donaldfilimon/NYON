const std = @import("std");
const types = @import("types.zig");

test "cg/contracts/seed-witness" {
    try std.testing.expectEqual(@as(u64, 2), types.planetSeed(2, @fromBackingInt(1)));
    try std.testing.expectEqual(@as(u64, 0x9E3779B97F4A7C17), types.planetSeed(2, @fromBackingInt(2)));
}

test "cg/contracts/limits-and-seed" {
    var config = types.Config{ .seed = 2 };
    try types.validateConfig(config);
    try std.testing.expectEqual(@as(u64, 2), types.planetSeed(2, @fromBackingInt(1)));
    try std.testing.expectEqual(@as(u64, 0x9E3779B97F4A7C17), types.planetSeed(2, @fromBackingInt(2)));
    try std.testing.expectEqual(types.Terrain.fertile, types.terrain(2, .{ .planet = @fromBackingInt(1), .local = 71 }));
    try std.testing.expectEqual(types.Terrain.rocky, types.terrain(2, .{ .planet = @fromBackingInt(1), .local = 74 }));

    for ([_]u8{ 0, 5 }) |count| {
        config.system_count = count;
        try std.testing.expectError(error.InvalidConfig, types.validateConfig(config));
        config.system_count = 1;
        config.civilization_count = count;
        try std.testing.expectError(error.InvalidConfig, types.validateConfig(config));
        config.civilization_count = 2;
        config.planet_count = count;
        try std.testing.expectError(error.InvalidConfig, types.validateConfig(config));
        config.planet_count = 2;
    }
    config.system_count = 2;
    try std.testing.expectError(error.InvalidConfig, types.validateConfig(config));
    config.planets[1].system = @fromBackingInt(2);
    try types.validateConfig(config);
    config.system_count = 1;
    config.planets[1].system = @fromBackingInt(1);

    for ([_]u16{ 3, 33 }) |side| {
        config.planets[0].side = side;
        try std.testing.expectError(error.InvalidConfig, types.validateConfig(config));
    }
    config.planets[0].side = 4;
    for ([_]u16{ 0, 4097 }) |capacity| {
        config.global_history_capacity = capacity;
        try std.testing.expectError(error.InvalidConfig, types.validateConfig(config));
    }
    config.global_history_capacity = 1;
    config.planets[0].capacities = .{ .settlements = 1, .buildings = 1, .freight = 1, .history_receipts = 1 };
    try types.validateConfig(config);
    config.planets[0].capacities.settlements = 65;
    try std.testing.expectError(error.InvalidConfig, types.validateConfig(config));

    var partial = types.Config{ .seed = 2, .planet_count = 1, .planets = undefined };
    partial.planets[0] = .{ .system = @fromBackingInt(1), .side = 4 };
    try types.validateConfig(partial);

    try std.testing.expect(types.PlanetId != types.SystemId);
    try std.testing.expect(types.PlanetId != types.CivilizationId);
    try std.testing.expect(types.SettlementRef != types.SettlementId);
}
