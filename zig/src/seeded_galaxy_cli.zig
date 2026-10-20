//! Standalone strategic metadata journey; does not install or advance gameplay.
const std = @import("std");
const sg = @import("seeded_galaxy/root.zig");
const role = @import("play_role.zig");
const cg = @import("coordinated_galaxy/types.zig");
const Options = struct {
    config: sg.Config = .{},
    budget: u16 = 256,
    policy: role.SessionPolicy = .{ .role = .creator, .civilization = null },
};

pub fn run(allocator: std.mem.Allocator, args: []const []const u8, out: *std.Io.Writer) !void {
    journey(allocator, args, out) catch |failure| {
        if (failure != error.WriteFailed) try out.print("ERR\t{s}\n", .{@errorName(failure)});
        return failure;
    };
}
fn decimal(comptime T: type, value: []const u8) !T {
    if (value.len == 0) return error.InvalidArguments;
    for (value) |digit| if (digit < '0' or digit > '9') return error.InvalidArguments;
    return std.fmt.parseInt(T, value, 10) catch error.InvalidArguments;
}
fn parse(args: []const []const u8) !Options {
    var result: Options = .{};
    var seen: u8 = 0;
    var index: usize = 0;
    const flags = [_][]const u8{ "--seed", "--systems", "--planets", "--civilizations", "--side", "--budget", "--role", "--civilization" };
    while (index < args.len) : (index += 2) {
        if (index + 1 == args.len) return error.InvalidArguments;
        var selected: ?u3 = null;
        for (flags, 0..) |flag, field| if (std.mem.eql(u8, flag, args[index])) {
            selected = @intCast(field);
            break;
        };
        const field = selected orelse return error.InvalidArguments;
        const bit = @as(u8, 1) << field;
        if (seen & bit != 0) return error.InvalidArguments;
        seen |= bit;
        const value = args[index + 1];
        switch (field) {
            0 => result.config.seed = try decimal(u64, value),
            1 => result.config.systems = try decimal(u16, value),
            2 => result.config.planets = try decimal(u16, value),
            3 => result.config.civilizations = try decimal(u16, value),
            4 => result.config.side = try decimal(u16, value),
            5 => result.budget = try decimal(u16, value),
            6 => result.policy.role = std.meta.stringToEnum(role.Role, value) orelse return error.InvalidArguments,
            7 => {
                const id = try decimal(u32, value);
                if (id < 1 or id > 16) return error.InvalidArguments;
                result.policy.civilization = @fromBackingInt(id);
            },
        }
    }
    if (result.budget == 0 or result.budget > 4096) return error.InvalidArguments;
    sg.validateConfig(result.config) catch return error.InvalidArguments;
    return result;
}
fn finish(setup: *sg.Setup, token: sg.Token, budget: u16) !void {
    // Explicit CLI driver, outside authority; never advances gameplay.
    while (true) switch (try setup.poll(token, budget)) {
        .pending => {},
        .ready => return,
        .failed => |failure| return failure,
        .canceled => return error.NotReady,
    };
}
fn journey(allocator: std.mem.Allocator, args: []const []const u8, out: *std.Io.Writer) !void {
    const options = try parse(args);
    var setup = try sg.Setup.init(allocator, 1);
    defer setup.deinit();
    const token = try setup.beginCreate(options.config);
    try finish(&setup, token, options.budget);
    var manifest = try setup.take(token);
    defer manifest.deinit();
    const config = try manifest.config();
    const digest = try manifest.digest();
    // Caller scratch, outside Setup's reservation. One copied wire record per query.
    var export_bytes: [16384]u8 = undefined;
    var row: [1]sg.WireRecord = undefined;
    var length: usize = 0;
    var cursor: u16 = 0;
    while (true) {
        const page = try manifest.wireRecords(cursor, &row);
        if (page.written != 1 or row[0].len > export_bytes.len - length) return error.InvariantViolation;
        @memcpy(export_bytes[length..][0..row[0].len], row[0].bytes[0..row[0].len]);
        length += row[0].len;
        cursor = page.next orelse break;
    }
    {
        const decode_token = try setup.beginDecode(export_bytes[0..length]);
        try finish(&setup, decode_token, options.budget);
        var decoded = try setup.take(decode_token);
        defer decoded.deinit();
        if (!std.mem.eql(u8, &digest, &(try decoded.digest()))) return error.InvariantViolation;
        cursor = 0;
        var offset: usize = 0;
        while (true) {
            const page = try decoded.wireRecords(cursor, &row);
            if (page.written != 1 or row[0].len > length - offset) return error.InvariantViolation;
            if (!std.mem.eql(u8, export_bytes[offset..][0..row[0].len], row[0].bytes[0..row[0].len])) return error.InvariantViolation;
            offset += row[0].len;
            cursor = page.next orelse break;
        }
        if (offset != length) return error.InvariantViolation;
    } // Destroy the verification owner before any subsequent admission.
    try sg.policy.validate(&manifest, options.policy);
    const capacities: [4]cg.PlanetCapacities = @splat(.{ .settlements = 64, .buildings = 256, .freight = 64, .history_receipts = 4096 });
    const bridge_result = sg.bridge.toCoordinatedConfig(&manifest, .{ .planet_capacities = capacities[0..@min(config.planets, 4)], .global_history_capacity = 4096 });
    const bridge_supported = if (bridge_result) |_| true else |failure| switch (failure) {
        error.CapacityUnsupported => false,
        else => return failure,
    };
    try out.print("IDENTITY\t{s}\nCONFIG\tseed={d}\tsystems={d}\tplanets={d}\tcivilizations={d}\tside={d}\tbudget={d}\n", .{ sg.rules_identity, config.seed, config.systems, config.planets, config.civilizations, config.side, options.budget });
    var system: [1]sg.System = undefined;
    for (0..config.systems) |i| {
        _ = try manifest.systems(@intCast(i), &system);
        const s = system[0];
        try out.print("SYSTEM\t{d}\tSystem {d}\tx={d}\ty={d}\tz={d}\n", .{ @backingInt(s.id), @backingInt(s.id), s.x, s.y, s.z });
    }
    var planet: [1]sg.Planet = undefined;
    for (0..config.planets) |i| {
        _ = try manifest.planets(@intCast(i), &planet);
        const p = planet[0];
        try out.print("PLANET\t{d}\tsystem={d}\torbit={d}\tterrain_seed={d}\tside={d}\n", .{ @backingInt(p.id), @backingInt(p.system), p.orbit_index, p.terrain_seed, p.side });
    }
    var civilization: [1]sg.Civilization = undefined;
    for (0..config.civilizations) |i| {
        _ = try manifest.civilizations(@intCast(i), &civilization);
        const c = civilization[0];
        try out.print("CIVILIZATION\t{d}\tCivilization {d}\tfounding_planet={d}\tkind=uninitialized\n", .{ @backingInt(c.id), @backingInt(c.id), @backingInt(c.founding_planet) });
    }
    try out.print("DIGEST\t{x}\nDECODE\tverified\tbytes={d}\nPOLICY\t{s}\tcivilization=", .{ digest, length, @tagName(options.policy.role) });
    if (options.policy.civilization) |id| try out.print("{d}\n", .{@backingInt(id)}) else try out.writeAll("none\n");
    try out.writeAll(if (bridge_supported) "BRIDGE\tconfig-validated\tlocal=64,256,64,4096\tglobal=4096\n" else "BRIDGE\tCapacityUnsupported\n");
    try out.writeAll("SCOPE\tstrategic-metadata-only\n");
}
