//! Stateless strategic metadata formulas. Each field is keyed independently.
const std = @import("std");
const t = @import("types.zig");

pub fn mix(input: u64) u64 {
    var value = input +% 0x9E3779B97F4A7C15;
    value = (value ^ (value >> 30)) *% 0xBF58476D1CE4E5B9;
    value = (value ^ (value >> 27)) *% 0x94D049BB133111EB;
    return value ^ (value >> 31);
}

pub fn draw(seed: u64, domain: u8, id: u32, field: u16) u64 {
    return mix(seed ^ (@as(u64, domain) << 56) ^ (@as(u64, id) << 16) ^ @as(u64, field));
}

pub fn system(config: t.Config, id: t.SystemId) t.Failure!t.System {
    try t.validateConfig(config);
    const value = @backingInt(id);
    if (value == 0 or value > config.systems) return error.InvalidConfig;
    const slot = value - 1;
    const x: u32 = (slot % 8) * 4096 + @as(u32, @intCast(draw(config.seed, 1, value, 0) % 1024));
    const y: u32 = (slot / 8) * 4096 + @as(u32, @intCast(draw(config.seed, 1, value, 1) % 1024));
    return .{ .id = id, .x = @intCast(x), .y = @intCast(y), .z = 0 };
}

pub fn planet(config: t.Config, id: t.PlanetId) t.Failure!t.Planet {
    try t.validateConfig(config);
    const value = @backingInt(id);
    if (value == 0 or value > config.planets) return error.InvalidConfig;
    const index = value - 1;
    return .{
        .id = id,
        .system = @fromBackingInt(1 + index % config.systems),
        .orbit_index = @intCast(index / config.systems),
        .terrain_seed = config.seed ^ (index *% @as(u64, 0x9E3779B97F4A7C15)),
        .side = config.side,
    };
}

pub fn civilization(config: t.Config, id: t.CivilizationId) t.Failure!t.Civilization {
    try t.validateConfig(config);
    const value = @backingInt(id);
    if (value == 0 or value > config.civilizations) return error.InvalidConfig;
    return .{ .id = id, .founding_planet = @fromBackingInt(value), .kind = .uninitialized };
}

pub fn distance(a: t.System, b: t.System) t.Failure!u64 {
    const dx = @as(i128, a.x) - @as(i128, b.x);
    const dy = @as(i128, a.y) - @as(i128, b.y);
    const square_x: u128 = @intCast(dx * dx);
    const square_y: u128 = @intCast(dy * dy);
    const squared = std.math.add(u128, square_x, square_y) catch return error.ArithmeticOverflow;
    // All i32 pairs fit below 2^33; division avoids a squared midpoint overflow.
    var low: u128 = 0;
    var high: u128 = @as(u128, 1) << 33;
    while (low + 1 < high) {
        const middle = low + (high - low) / 2;
        if (middle <= squared / middle) low = middle else high = middle;
    }
    if (low > std.math.maxInt(u64)) return error.ArithmeticOverflow;
    return @intCast(low);
}
