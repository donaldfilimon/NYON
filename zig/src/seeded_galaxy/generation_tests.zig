const std = @import("std");
const sg = @import("root.zig");
const f = @import("fixtures.zig");
test "independent seed zero records and mixing" {
    try std.testing.expectEqual(@as(u64, 0xE220A8397B1DCDAF), sg.generate.mix(0));
    const c = f.small_config;
    const a = try sg.generate.system(c, @fromBackingInt(1));
    try std.testing.expectEqual(@as(i32, 889), a.x);
    try std.testing.expectEqual(@as(i32, 369), a.y);
    const b = try sg.generate.system(c, @fromBackingInt(2));
    try std.testing.expectEqual(@as(i32, 4399), b.x);
    try std.testing.expectEqual(@as(i32, 764), b.y);
    const p = try sg.generate.planet(c, @fromBackingInt(4));
    try std.testing.expectEqual(@as(u32, 2), @backingInt(p.system));
    try std.testing.expectEqual(@as(u16, 1), p.orbit_index);
    try std.testing.expectEqual(@as(u64, 0xDAA66D2C7DDF743F), p.terrain_seed);
    try std.testing.expectEqual(@as(usize, 177), f.small_wire.len);
}

test "literal wire and digest remain independently pinned" {
    var digest: [32]u8 = undefined;
    std.crypto.hash.sha2.Sha256.hash(&f.small_wire, &digest, .{});
    try std.testing.expectEqualSlices(u8, &f.small_digest, &digest);
    try std.testing.expectEqual(@as(usize, 32), f.maximum_zero_digest.len);
    try std.testing.expectEqual(@as(usize, 32), f.maximum_max_digest.len);
    try std.testing.expectEqual(@as(u64, 0x4AE81809EEB05379), sg.generate.draw(0, 1, 1, 0));
    try std.testing.expectEqual(@as(u64, 0xFF2ED4AFDFE0AD71), sg.generate.draw(0, 1, 1, 1));
}

test "generation rejects zero and over-limit identifiers before subtraction" {
    const c = f.small_config;
    for ([_]u32{ 0, 3, std.math.maxInt(u32) }) |id|
        try std.testing.expectError(error.InvalidConfig, sg.generate.system(c, @fromBackingInt(id)));
    for ([_]u32{ 0, 5, std.math.maxInt(u32) }) |id|
        try std.testing.expectError(error.InvalidConfig, sg.generate.planet(c, @fromBackingInt(id)));
    for ([_]u32{ 0, 3, std.math.maxInt(u32) }) |id|
        try std.testing.expectError(error.InvalidConfig, sg.generate.civilization(c, @fromBackingInt(id)));
}

test "minimum and side endpoints preserve keyed strategic positions" {
    const min: sg.Config = .{ .seed = std.math.maxInt(u64), .systems = 1, .planets = 1, .civilizations = 1, .side = 4 };
    const high_side: sg.Config = .{ .seed = min.seed, .systems = 1, .planets = 1, .civilizations = 1, .side = 32 };
    try std.testing.expectEqualDeep(try sg.generate.system(min, @fromBackingInt(1)), try sg.generate.system(high_side, @fromBackingInt(1)));
    const p = try sg.generate.planet(min, @fromBackingInt(1));
    try std.testing.expectEqual(@as(u32, 1), @backingInt(p.system));
    try std.testing.expectEqual(@as(u16, 0), p.orbit_index);
    try std.testing.expectEqual(min.seed, p.terrain_seed);
    try std.testing.expectEqual(@as(u16, 4), p.side);
    try std.testing.expectEqual(@as(u16, 32), (try sg.generate.planet(high_side, @fromBackingInt(1))).side);
    const home = try sg.generate.civilization(min, @fromBackingInt(1));
    try std.testing.expectEqual(@as(u32, 1), @backingInt(home.founding_planet));
    try std.testing.expectEqual(.uninitialized, home.kind);
}

test "maximum manifest membership orbit home and position uniqueness" {
    for ([_]u64{ 0, std.math.maxInt(u64) }) |seed| {
        const c: sg.Config = .{ .seed = seed, .systems = 64, .planets = 512, .civilizations = 16, .side = 8 };
        var systems: [64]sg.System = undefined;
        for (0..64) |index| {
            const id: u32 = @intCast(index + 1);
            systems[index] = try sg.generate.system(c, @fromBackingInt(id));
            try std.testing.expectEqual(id, @backingInt(systems[index].id));
            try std.testing.expectEqual(@as(i32, 0), systems[index].z);
            try std.testing.expectEqualDeep(systems[index], try sg.generate.system(c, @fromBackingInt(id)));
            for (0..index) |prior| {
                try std.testing.expect(systems[index].x != systems[prior].x or
                    systems[index].y != systems[prior].y);
            }
        }
        const last = systems[63];
        const expected_x: i32 = if (seed == 0) 29_603 else 29_066;
        const expected_y: i32 = if (seed == 0) 29_224 else 29_521;
        try std.testing.expectEqual(expected_x, last.x);
        try std.testing.expectEqual(expected_y, last.y);
        for (0..512) |index| {
            const id: u32 = @intCast(index + 1);
            const p = try sg.generate.planet(c, @fromBackingInt(id));
            try std.testing.expectEqual(id, @backingInt(p.id));
            try std.testing.expectEqual(@as(u32, @intCast(index % 64 + 1)), @backingInt(p.system));
            try std.testing.expectEqual(@as(u16, @intCast(index / 64)), p.orbit_index);
            try std.testing.expectEqual(seed ^ (@as(u64, @intCast(index)) *% 0x9E3779B97F4A7C15), p.terrain_seed);
        }
        for (0..16) |index| {
            const id: u32 = @intCast(index + 1);
            const civ = try sg.generate.civilization(c, @fromBackingInt(id));
            try std.testing.expectEqual(id, @backingInt(civ.id));
            try std.testing.expectEqual(id, @backingInt(civ.founding_planet));
            try std.testing.expectEqual(.uninitialized, civ.kind);
        }
    }
}

test "distance floors Euclidean metric symmetrically over i32 extremes" {
    const a: sg.System = .{ .id = @fromBackingInt(1), .x = 0, .y = 0, .z = 0 };
    const b: sg.System = .{ .id = @fromBackingInt(2), .x = 3, .y = 4, .z = 0 };
    try std.testing.expectEqual(@as(u64, 0), try sg.generate.distance(a, a));
    try std.testing.expectEqual(@as(u64, 5), try sg.generate.distance(a, b));
    try std.testing.expectEqual(try sg.generate.distance(a, b), try sg.generate.distance(b, a));
    const low: sg.System = .{ .id = @fromBackingInt(1), .x = std.math.minInt(i32), .y = std.math.minInt(i32), .z = 0 };
    const high: sg.System = .{ .id = @fromBackingInt(2), .x = std.math.maxInt(i32), .y = std.math.maxInt(i32), .z = 0 };
    try std.testing.expectEqual(@as(u64, 6_074_000_998), try sg.generate.distance(low, high));
    try std.testing.expectEqual(try sg.generate.distance(low, high), try sg.generate.distance(high, low));
}
