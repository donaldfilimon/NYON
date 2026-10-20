//! Explicit canonical little-endian fields; no native struct representation.
const std = @import("std");
const t = @import("types.zig");

pub fn encodeHeader(config: t.Config, out: []u8) t.Failure!u16 {
    try t.validateConfig(config);
    if (out.len < 47) return error.InvalidConfig;
    @memcpy(out[0..4], "NYSG");
    std.mem.writeInt(u16, out[4..6], 1, .little);
    std.mem.writeInt(u16, out[6..8], 21, .little);
    @memcpy(out[8..29], t.rules_identity);
    std.mem.writeInt(u16, out[29..31], config.version, .little);
    std.mem.writeInt(u64, out[31..39], config.seed, .little);
    std.mem.writeInt(u16, out[39..41], config.systems, .little);
    std.mem.writeInt(u16, out[41..43], config.planets, .little);
    std.mem.writeInt(u16, out[43..45], config.civilizations, .little);
    std.mem.writeInt(u16, out[45..47], config.side, .little);
    return 47;
}
pub fn encodeSystem(value: t.System, out: []u8) t.Failure!u16 {
    if (out.len < 16) return error.InvalidConfig;
    std.mem.writeInt(u32, out[0..4], @backingInt(value.id), .little);
    std.mem.writeInt(u32, out[4..8], @bitCast(value.x), .little);
    std.mem.writeInt(u32, out[8..12], @bitCast(value.y), .little);
    std.mem.writeInt(u32, out[12..16], @bitCast(value.z), .little);
    return 16;
}
pub fn encodePlanet(value: t.Planet, out: []u8) t.Failure!u16 {
    if (out.len < 20) return error.InvalidConfig;
    std.mem.writeInt(u32, out[0..4], @backingInt(value.id), .little);
    std.mem.writeInt(u32, out[4..8], @backingInt(value.system), .little);
    std.mem.writeInt(u16, out[8..10], value.orbit_index, .little);
    std.mem.writeInt(u64, out[10..18], value.terrain_seed, .little);
    std.mem.writeInt(u16, out[18..20], value.side, .little);
    return 20;
}
pub fn encodeCivilization(value: t.Civilization, out: []u8) t.Failure!u16 {
    if (out.len < 9) return error.InvalidConfig;
    std.mem.writeInt(u32, out[0..4], @backingInt(value.id), .little);
    std.mem.writeInt(u32, out[4..8], @backingInt(value.founding_planet), .little);
    out[8] = @backingInt(value.kind);
    return 9;
}

pub const ParsedHeader = struct { config: t.Config, consumed: u16 };
/// The identity length is checked, never used as an index or allocation size.
pub fn parseHeader(input: []const u8) t.Failure!ParsedHeader {
    if (input.len < 47) return error.InvalidWire;
    if (!std.mem.eql(u8, input[0..4], "NYSG")) return error.InvalidWire;
    if (std.mem.readInt(u16, input[4..6], .little) != 1) return error.UnsupportedWireVersion;
    if (std.mem.readInt(u16, input[6..8], .little) != 21) return error.InvalidWire;
    if (!std.mem.eql(u8, input[8..29], t.rules_identity)) return error.InvalidWire;
    const config: t.Config = .{
        .version = std.mem.readInt(u16, input[29..31], .little),
        .seed = std.mem.readInt(u64, input[31..39], .little),
        .systems = std.mem.readInt(u16, input[39..41], .little),
        .planets = std.mem.readInt(u16, input[41..43], .little),
        .civilizations = std.mem.readInt(u16, input[43..45], .little),
        .side = std.mem.readInt(u16, input[45..47], .little),
    };
    try t.validateConfig(config);
    return .{ .config = config, .consumed = 47 };
}
pub fn expectedLength(config: t.Config) t.Failure!u16 {
    try t.validateConfig(config);
    var len: u16 = 47;
    inline for (.{ .{ config.systems, 16 }, .{ config.planets, 20 }, .{ config.civilizations, 9 } }) |record| {
        const size = std.math.mul(u16, record[0], record[1]) catch return error.ArithmeticOverflow;
        len = std.math.add(u16, len, size) catch return error.ArithmeticOverflow;
    }
    return len;
}
pub fn parseSystem(input: []const u8) t.Failure!t.System {
    if (input.len != 16) return error.InvalidWire;
    if (std.mem.readInt(u32, input[12..16], .little) != 0) return error.InvalidWire;
    return .{
        .id = @fromBackingInt(std.mem.readInt(u32, input[0..4], .little)),
        .x = @bitCast(std.mem.readInt(u32, input[4..8], .little)),
        .y = @bitCast(std.mem.readInt(u32, input[8..12], .little)),
        .z = 0,
    };
}
pub fn parsePlanet(input: []const u8) t.Failure!t.Planet {
    if (input.len != 20) return error.InvalidWire;
    return .{
        .id = @fromBackingInt(std.mem.readInt(u32, input[0..4], .little)),
        .system = @fromBackingInt(std.mem.readInt(u32, input[4..8], .little)),
        .orbit_index = std.mem.readInt(u16, input[8..10], .little),
        .terrain_seed = std.mem.readInt(u64, input[10..18], .little),
        .side = std.mem.readInt(u16, input[18..20], .little),
    };
}
pub fn parseCivilization(input: []const u8) t.Failure!t.Civilization {
    if (input.len != 9 or input[8] != 0) return error.InvalidWire;
    return .{
        .id = @fromBackingInt(std.mem.readInt(u32, input[0..4], .little)),
        .founding_planet = @fromBackingInt(std.mem.readInt(u32, input[4..8], .little)),
        .kind = .uninitialized,
    };
}
