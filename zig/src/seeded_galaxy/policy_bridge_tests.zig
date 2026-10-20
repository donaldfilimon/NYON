const std = @import("std");
const sg = @import("root.zig");
const h = @import("test_support.zig");
const f = @import("fixtures.zig");
const cg = @import("../coordinated_galaxy/types.zig");
const role = @import("../play_role.zig");

test "maximum metadata cannot masquerade as four world simulation" {
    var setup = try sg.Setup.init(std.testing.allocator, 20);
    defer setup.deinit();
    const token = try setup.beginCreate(.{ .systems = 64, .planets = 512, .civilizations = 16, .side = 8 });
    try h.finish(&setup, token, 256);
    var manifest = try setup.take(token);
    defer manifest.deinit();
    const before = try manifest.digest();
    const caps: [4]cg.PlanetCapacities = @splat(.{});
    try std.testing.expectError(error.CapacityUnsupported, sg.bridge.toCoordinatedConfig(&manifest, .{
        .planet_capacities = &caps,
        .global_history_capacity = 4096,
    }));
    const after = try manifest.digest();
    try std.testing.expectEqualSlices(u8, &before, &after);
}

test "bridge preserves explicitly supplied capacities" {
    var setup = try sg.Setup.init(std.testing.allocator, 21);
    defer setup.deinit();
    const token = try setup.beginCreate(f.small_config);
    try h.finish(&setup, token, 7);
    var manifest = try setup.take(token);
    defer manifest.deinit();
    const caps = [_]cg.PlanetCapacities{
        .{ .settlements = 1, .buildings = 2, .freight = 3, .history_receipts = 4 },
        .{ .settlements = 5, .buildings = 6, .freight = 7, .history_receipts = 8 },
        .{ .settlements = 9, .buildings = 10, .freight = 11, .history_receipts = 12 },
        .{ .settlements = 13, .buildings = 14, .freight = 15, .history_receipts = 16 },
    };
    const config = try sg.bridge.toCoordinatedConfig(&manifest, .{
        .planet_capacities = &caps,
        .global_history_capacity = 17,
    });
    try std.testing.expectEqual(@as(u16, 17), config.global_history_capacity);
    try std.testing.expectEqual(@as(u8, 4), config.planet_count);
    for (caps, 0..) |cap, index| {
        try std.testing.expectEqualDeep(cap, config.planets[index].capacities);
        try std.testing.expectEqual(@as(u32, 1 + @as(u32, @intCast(index % 2))), @backingInt(config.planets[index].system));
        try std.testing.expectEqual(@as(u16, 8), config.planets[index].side);
    }
    try cg.validateConfig(config);
}

test "bridge returns the independently specified v0 config without allocating" {
    var counter = h.Counter.init(std.testing.allocator);
    var setup = try sg.Setup.init(counter.allocator(), 22);
    defer setup.deinit();
    const token = try setup.beginCreate(f.small_config);
    try h.finish(&setup, token, 8);
    var manifest = try setup.take(token);
    defer manifest.deinit();
    const before = try manifest.digest();
    const attempts = counter.attempts;
    const caps = [_]cg.PlanetCapacities{
        .{ .settlements = 1, .buildings = 2, .freight = 3, .history_receipts = 4 },
        .{ .settlements = 5, .buildings = 6, .freight = 7, .history_receipts = 8 },
        .{ .settlements = 9, .buildings = 10, .freight = 11, .history_receipts = 12 },
        .{ .settlements = 13, .buildings = 14, .freight = 15, .history_receipts = 16 },
    };
    const actual = try sg.bridge.toCoordinatedConfig(&manifest, .{ .planet_capacities = &caps, .global_history_capacity = 17 });
    const expected: cg.Config = .{
        .seed = 0,
        .system_count = 2,
        .civilization_count = 2,
        .planet_count = 4,
        .global_history_capacity = 17,
        .planets = .{
            .{ .system = @fromBackingInt(1), .side = 8, .capacities = .{ .settlements = 1, .buildings = 2, .freight = 3, .history_receipts = 4 } },
            .{ .system = @fromBackingInt(2), .side = 8, .capacities = .{ .settlements = 5, .buildings = 6, .freight = 7, .history_receipts = 8 } },
            .{ .system = @fromBackingInt(1), .side = 8, .capacities = .{ .settlements = 9, .buildings = 10, .freight = 11, .history_receipts = 12 } },
            .{ .system = @fromBackingInt(2), .side = 8, .capacities = .{ .settlements = 13, .buildings = 14, .freight = 15, .history_receipts = 16 } },
        },
    };
    try std.testing.expectEqualDeep(expected, actual);
    try cg.validateConfig(actual);
    try std.testing.expectEqual(attempts, counter.attempts);
    try std.testing.expectEqualSlices(u8, &before, &(try manifest.digest()));
}

test "bridge rejects count before options and validates every supplied capacity" {
    const four: [4]cg.PlanetCapacities = @splat(.{});
    for ([_]sg.Config{
        .{ .systems = 5, .planets = 5, .civilizations = 1 },
        .{ .systems = 1, .planets = 5, .civilizations = 1 },
        .{ .systems = 1, .planets = 5, .civilizations = 5 },
    }, 0..) |source, index| {
        var setup = try sg.Setup.init(std.testing.allocator, 40 + index);
        defer setup.deinit();
        const token = try setup.beginCreate(source);
        try h.finish(&setup, token, 8);
        var manifest = try setup.take(token);
        defer manifest.deinit();
        try std.testing.expectError(error.CapacityUnsupported, sg.bridge.toCoordinatedConfig(&manifest, .{ .planet_capacities = &four, .global_history_capacity = 0 }));
    }
    var setup = try sg.Setup.init(std.testing.allocator, 45);
    defer setup.deinit();
    const token = try setup.beginCreate(f.small_config);
    try h.finish(&setup, token, 8);
    var manifest = try setup.take(token);
    defer manifest.deinit();
    try std.testing.expectError(error.InvalidConfig, sg.bridge.toCoordinatedConfig(&manifest, .{ .planet_capacities = four[0..3], .global_history_capacity = 1 }));
    const five: [5]cg.PlanetCapacities = @splat(.{});
    try std.testing.expectError(error.InvalidConfig, sg.bridge.toCoordinatedConfig(&manifest, .{ .planet_capacities = &five, .global_history_capacity = 1 }));
    for ([_]u16{ 0, 4097 }) |history| try std.testing.expectError(error.InvalidConfig, sg.bridge.toCoordinatedConfig(&manifest, .{ .planet_capacities = &four, .global_history_capacity = history }));
    for ([_]u16{ 1, 4096 }) |history| _ = try sg.bridge.toCoordinatedConfig(&manifest, .{ .planet_capacities = &four, .global_history_capacity = history });
    for (0..4) |field| {
        const maximum: u16 = switch (field) {
            0, 2 => 64,
            1 => 256,
            else => 4096,
        };
        for ([_]u16{ 0, maximum + 1 }) |bad| {
            var caps = four;
            switch (field) {
                0 => caps[2].settlements = bad,
                1 => caps[2].buildings = bad,
                2 => caps[2].freight = bad,
                else => caps[2].history_receipts = bad,
            }
            try std.testing.expectError(error.InvalidConfig, sg.bridge.toCoordinatedConfig(&manifest, .{ .planet_capacities = &caps, .global_history_capacity = 1 }));
        }
        var caps = four;
        switch (field) {
            0 => caps[2].settlements = maximum,
            1 => caps[2].buildings = maximum,
            2 => caps[2].freight = maximum,
            else => caps[2].history_receipts = maximum,
        }
        const config = try sg.bridge.toCoordinatedConfig(&manifest, .{ .planet_capacities = &caps, .global_history_capacity = 4096 });
        try std.testing.expectEqualDeep(caps[2], config.planets[2].capacities);
    }
}

test "policy uses existing role admission and refuses absent selections" {
    var counter = h.Counter.init(std.testing.allocator);
    var setup = try sg.Setup.init(counter.allocator(), 50);
    defer setup.deinit();
    const token = try setup.beginCreate(f.small_config);
    try h.finish(&setup, token, 8);
    var manifest = try setup.take(token);
    defer manifest.deinit();
    const before = try manifest.digest();
    const attempts = counter.attempts;
    const selected: role.CivilizationId = @fromBackingInt(2);
    const other: role.CivilizationId = @fromBackingInt(1);
    const actors = [_]?role.CivilizationId{ null, selected, other };
    for ([_]role.Role{ .creator, .ruler, .combined }) |kind| {
        for ([_]?role.CivilizationId{ null, selected, @fromBackingInt(0), @fromBackingInt(3), @fromBackingInt(17) }) |civilization| {
            const policy: role.SessionPolicy = .{ .role = kind, .civilization = civilization };
            const valid_selection = civilization == null and kind == .creator or civilization != null and @backingInt(civilization.?) == 2;
            if (valid_selection) try sg.policy.validate(&manifest, policy) else try std.testing.expectError(error.InvalidPolicy, sg.policy.validate(&manifest, policy));
            for ([_]role.Provenance{ .creator, .ruler, .autonomous }) |provenance| {
                for (actors) |actor| {
                    const permitted = valid_selection and switch (provenance) {
                        .creator => kind != .ruler and actor == null,
                        .ruler => kind != .creator and actor != null and @backingInt(actor.?) == 2,
                        .autonomous => false,
                    };
                    if (permitted) try sg.policy.admit(&manifest, policy, provenance, actor) else try std.testing.expectError(error.InvalidPolicy, sg.policy.admit(&manifest, policy, provenance, actor));
                    try std.testing.expectEqualSlices(u8, &before, &(try manifest.digest()));
                }
            }
        }
    }
    try std.testing.expectEqual(attempts, counter.attempts);
}

test "bridge preserves side endpoints and leaves inactive v0 slots at defaults" {
    for ([_]u16{ 4, 32 }, 0..) |side, index| {
        var setup = try sg.Setup.init(std.testing.allocator, 60 + index);
        defer setup.deinit();
        const token = try setup.beginCreate(.{ .seed = 99, .systems = 1, .planets = 1, .civilizations = 1, .side = side });
        try h.finish(&setup, token, 8);
        var manifest = try setup.take(token);
        defer manifest.deinit();
        const caps = [_]cg.PlanetCapacities{.{ .settlements = 64, .buildings = 256, .freight = 64, .history_receipts = 4096 }};
        const config = try sg.bridge.toCoordinatedConfig(&manifest, .{ .planet_capacities = &caps, .global_history_capacity = 4096 });
        try std.testing.expectEqual(@as(u64, 99), config.seed);
        try std.testing.expectEqual(@as(u8, 1), config.system_count);
        try std.testing.expectEqual(@as(u8, 1), config.civilization_count);
        try std.testing.expectEqual(@as(u8, 1), config.planet_count);
        try std.testing.expectEqual(side, config.planets[0].side);
        try std.testing.expectEqualDeep([_]cg.PlanetConfig{ .{}, .{}, .{} }, config.planets[1..4].*);
        try cg.validateConfig(config);
    }
}

test "maximum copied pages enumerate 512 planets and 593 wire records" {
    var setup = try sg.Setup.init(std.testing.allocator, 70);
    const token = try setup.beginCreate(.{ .systems = 64, .planets = 512, .civilizations = 16 });
    try h.finish(&setup, token, 256);
    var manifest = try setup.take(token);
    setup.deinit();
    defer manifest.deinit();
    var planets: [64]sg.Planet = undefined;
    var cursor: u16 = 0;
    var count: u16 = 0;
    while (true) {
        const page = try manifest.planets(cursor, &planets);
        try std.testing.expect(page.written <= 64);
        for (planets[0..page.written]) |planet| {
            count += 1;
            try std.testing.expectEqual(@as(u32, count), @backingInt(planet.id));
        }
        cursor = page.next orelse break;
    }
    try std.testing.expectEqual(@as(u16, 512), count);
    try std.testing.expectEqualDeep(sg.Page{ .written = 0, .next = null }, try manifest.planets(512, planets[0..1]));
    try std.testing.expectError(error.InvalidConfig, manifest.planets(513, planets[0..1]));
    try std.testing.expectError(error.InvalidConfig, manifest.planets(0, planets[0..0]));
    var too_many: [65]sg.Planet = undefined;
    try std.testing.expectError(error.InvalidConfig, manifest.planets(0, &too_many));

    var wires: [64]sg.WireRecord = undefined;
    cursor = 0;
    count = 0;
    while (true) {
        const page = try manifest.wireRecords(cursor, &wires);
        for (wires[0..page.written]) |wire| {
            try std.testing.expectEqual(@as(u16, if (count == 0) 47 else if (count <= 64) 16 else if (count <= 576) 20 else 9), wire.len);
            count += 1;
        }
        cursor = page.next orelse break;
    }
    try std.testing.expectEqual(@as(u16, 593), count);
    try std.testing.expectEqualDeep(sg.Page{ .written = 0, .next = null }, try manifest.wireRecords(593, wires[0..1]));
    try std.testing.expectError(error.InvalidConfig, manifest.wireRecords(594, wires[0..1]));

    var civs: [16]sg.Civilization = undefined;
    _ = try manifest.civilizations(0, &civs);
    for (civs) |civ| try std.testing.expectEqual(.uninitialized, civ.kind);
    const independent = planets[0];
    _ = try manifest.planets(0, planets[0..1]);
    try std.testing.expectEqual(@as(u32, 449), @backingInt(independent.id));
    try std.testing.expectEqual(@as(u32, 1), @backingInt(planets[0].id));
}

test "closed manifest rejects bridge and policy after admission ordering" {
    var setup = try sg.Setup.init(std.testing.allocator, 71);
    defer setup.deinit();
    const token = try setup.beginCreate(f.small_config);
    try h.finish(&setup, token, 8);
    var manifest = try setup.take(token);
    manifest.deinit();
    const caps: [4]cg.PlanetCapacities = @splat(.{});
    try std.testing.expectError(error.Shutdown, sg.bridge.toCoordinatedConfig(&manifest, .{ .planet_capacities = &caps, .global_history_capacity = 1 }));
    const creator: role.SessionPolicy = .{ .role = .creator, .civilization = null };
    try std.testing.expectError(error.Shutdown, sg.policy.validate(&manifest, creator));
    try std.testing.expectError(error.Shutdown, sg.policy.admit(&manifest, creator, .creator, null));
    try std.testing.expectError(error.InvalidPolicy, sg.policy.admit(&manifest, creator, .autonomous, null));
}
