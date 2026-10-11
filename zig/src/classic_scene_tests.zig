const std = @import("std");
const scene = @import("classic_scene.zig");
test "scene sector literal endpoints and integer clamp" {
    try std.testing.expectEqual(@as(f32, -5), scene.sectorToCommandPlane(-1, 10001).x);
    try std.testing.expectEqual(@as(f32, 5), scene.sectorToCommandPlane(-1, 10001).z);
}
const m = @import("model.zig");
const math = @import("classic_math.zig");
const camera = @import("classic_camera.zig");
const prefs = @import("preferences.zig");
const generator = @import("generator.zig");
const ctl = @import("classic_advisory_controller.zig");
fn campaign(fleets: []const m.Fleet) scene.SceneCampaign {
    const generated = generator.generate(m.DEFAULT_SEED, .{});
    return .{ .worlds = generated.worlds, .fleets = fleets, .active_hazard = null };
}
fn fleet(id: u64) m.Fleet {
    return .{ .id = id, .faction = .union_faction, .source = 0, .destination = 1, .strength = 0, .route_length = 2, .progress = 1, .speed_per_second = 23, .hydrosphere_level = 0, .movement_remainder = 0 };
}
fn build(c: scene.SceneCampaign, alpha: f32) scene.SceneFrame {
    const cam = camera.CameraState.init(.{ .x = 1280, .y = 800 });
    return scene.buildScene(c, 0, alpha, .{}, null, 12, &cam, .{});
}
fn vec(expected: math.Vec3, actual: math.Vec3) !void {
    try std.testing.expectApproxEqAbs(expected.x, actual.x, 0.000001);
    try std.testing.expectApproxEqAbs(expected.y, actual.y, 0.000001);
    try std.testing.expectApproxEqAbs(expected.z, actual.z, 0.000001);
}
test "scene empty worlds literal properties stored IDs hazard and all ownership branches" {
    var c = campaign(&.{});
    const owners = [_]?m.Faction{ .union_faction, .helix, .choir, null };
    const colors = [_][4]f32{ .{ 0.12, 0.82, 1, 1 }, .{ 1, 0.28, 0.12, 1 }, .{ 0.76, 0.32, 1, 1 }, .{ 0.48, 0.56, 0.66, 1 } };
    for (&c.worlds, 0..) |*w, i| {
        w.id = @intCast(100 + i);
        w.owner = owners[i % 4];
        w.energy = if (i == 0) 0 else 300000;
        w.defense = if (i == 0) 0 else 200000;
        w.fields = .{ .atmosphere = 0, .hydrosphere = 10, .topology = 255 };
    }
    c.worlds[0].position = .{ .x = -999, .y = 11000 };
    c.worlds[1].position = .{ .x = 5000, .y = 5000 };
    c.active_hazard = .{ .event_index = 1, .kind = .gravity_tide, .affected_field = .topology, .start = 1, .end = 2 };
    const before = c;
    const cam = camera.CameraState.init(.{ .x = 800, .y = 600 });
    const frame = scene.buildScene(c, 0, 0, .{ .selected_world = 101, .hovered_world = 100 }, null, 0, &cam, .{ .high_contrast = true });
    try std.testing.expectEqualDeep(before.worlds, c.worlds);
    try std.testing.expectEqual(@as(usize, 0), frame.fleets().len);
    try std.testing.expectEqual(@as(usize, 0), frame.routes().len);
    try std.testing.expectEqualDeep(math.Vec3{ .x = -5, .y = 0, .z = 5 }, frame.worlds[0].position);
    try std.testing.expectEqualDeep(math.Vec3.zero, frame.worlds[1].position);
    for (frame.worlds, 0..) |w, i| {
        try std.testing.expectEqual(@as(u8, @intCast(100 + i)), w.id);
        try std.testing.expectEqualDeep(colors[i % 4], w.ownership_color);
        try std.testing.expectEqual(@as(u32, @intCast(i % 4 + 1)), w.ownership_pattern);
        try std.testing.expectEqualDeep([3]f32{ 0, 1, 25.5 }, w.fields);
        try std.testing.expectEqual(@as(?m.HazardKind, .gravity_tide), w.hazard);
        try std.testing.expectEqual(@as(?m.FieldKind, .topology), w.hazard_field);
        try std.testing.expectEqual(i == 0, w.hovered);
        try std.testing.expectEqual(i == 1, w.selected);
        try std.testing.expectEqual(@as(f32, if (i == 0) 0 else 1), w.energy);
        try std.testing.expectEqual(@as(f32, if (i == 0) 0 else 1), w.defense);
        try std.testing.expectEqual(@as(f32, if (i == 0) 0.31 else 0.31 + 0.10 + 0.06), w.radius);
    }
}
test "scene 520 reverse IDs late lower replacement and invalid filtering after truncation" {
    var input: [520]m.Fleet = undefined;
    for (&input, 0..) |*f, i| f.* = fleet(519 - i);
    const before = input;
    var frame = build(campaign(&input), 0);
    try std.testing.expectEqualDeep(before, input);
    try std.testing.expectEqual(@as(usize, 512), frame.fleet_count);
    try std.testing.expectEqual(@as(usize, 1), frame.route_count);
    for (frame.fleets(), 0..) |f, i| try std.testing.expectEqual(@as(u64, @intCast(i)), f.id);
    try std.testing.expectEqualDeep(scene.routeControl(frame.routes()[0].start, frame.routes()[0].end, 0), frame.routes()[0].control);
    input[519].source = 7;
    frame = build(campaign(&input), 0);
    try std.testing.expectEqual(@as(usize, 511), frame.fleet_count);
    try std.testing.expectEqual(@as(u64, 511), frame.fleets()[510].id);
    input[519].source = 0;
    input[519].destination = 255;
    frame = build(campaign(&input), 0);
    try std.testing.expectEqual(@as(usize, 511), frame.fleet_count);
    for (&input) |*f| {
        if (f.id < 512) f.source = 255;
    }
    frame = build(campaign(&input), 0);
    try std.testing.expectEqual(@as(usize, 0), frame.fleet_count);
    try std.testing.expectEqual(@as(usize, 0), frame.route_count);
}
test "scene equal ID stable properties and tie at capacity" {
    var input = [_]m.Fleet{ fleet(8), fleet(1), fleet(8) };
    input[0].strength = 20000;
    input[0].destination = 2;
    input[2].strength = 30000;
    var frame = build(campaign(&input), 0);
    try std.testing.expectEqual(@as(f32, 0.2), frame.fleets()[1].strength);
    try std.testing.expectEqual(@as(u8, 2), frame.fleets()[1].destination);
    try std.testing.expectEqual(@as(f32, 0.3), frame.fleets()[2].strength);
    std.mem.swap(m.Fleet, &input[0], &input[2]);
    frame = build(campaign(&input), 0);
    try std.testing.expectEqual(@as(f32, 0.3), frame.fleets()[1].strength);
    var boundary: [514]m.Fleet = undefined;
    for (boundary[0..511], 0..) |*f, i| f.* = fleet(i % 8);
    for (boundary[511..], 0..) |*f, i| {
        f.* = fleet(8);
        f.strength = @intCast(20000 + 10000 * i);
    }
    frame = build(campaign(&boundary), 0);
    try std.testing.expectEqual(@as(usize, 512), frame.fleet_count);
    try std.testing.expectEqual(@as(f32, 0.2), frame.fleets()[511].strength);
}
test "scene unique permutation invariant populated prefixes encounter routes and first control" {
    var input = [_]m.Fleet{ fleet(4), fleet(2), fleet(1), fleet(3) };
    input[0].source = 6;
    input[0].destination = 0;
    input[2].source = 6;
    input[2].destination = 0;
    input[3].faction = .helix;
    const a = build(campaign(&input), 0.5);
    std.mem.reverse(m.Fleet, &input);
    const b = build(campaign(&input), 0.5);
    try std.testing.expectEqualDeep(a.fleets(), b.fleets());
    try std.testing.expectEqualDeep(a.routes(), b.routes());
    try std.testing.expectEqual(@as(u8, 6), a.routes()[0].source);
    try std.testing.expectEqual(@as(u8, 0), a.routes()[0].destination);
    try std.testing.expectEqualDeep(scene.routeControl(a.routes()[0].start, a.routes()[0].end, 1), a.routes()[0].control);
    var all: [147]m.Fleet = undefined;
    var n: usize = 0;
    for (0..7) |src| for (0..7) |dst| for ([_]m.Faction{ .union_faction, .helix, .choir }) |faction| {
        all[n] = fleet(n);
        all[n].source = @intCast(src);
        all[n].destination = @intCast(dst);
        all[n].faction = faction;
        n += 1;
    };
    const full = build(campaign(&all), 0);
    try std.testing.expectEqual(@as(usize, 147), full.route_count);
    // 7*7*3 keys make 256 saturation unreachable; do not fabricate worlds.
}
test "scene independent analytic curve and indexed endpoints" {
    const start = math.Vec3.zero;
    const end = math.Vec3{ .x = 3, .y = 0, .z = 4 };
    const control = scene.routeControl(start, end, 0);
    try vec(.{ .x = 1.5, .y = 1.15, .z = 2 }, control);
    try vec(start, scene.quadraticBezier(start, control, end, 0));
    try vec(end, scene.quadraticBezier(start, control, end, 1));
    try vec(.{ .x = 1.5, .y = 0.575, .z = 2 }, scene.quadraticBezier(start, control, end, 0.5));
    try std.testing.expectApproxEqAbs(@as(f32, 0.025), scene.routeControl(start, end, 1).y - control.y, 0.000001);
    var input = [_]m.Fleet{fleet(0)};
    var c = campaign(&input);
    c.worlds[0].id = 99;
    c.worlds[1].id = 98;
    c.worlds[0].position = .{ .x = 5000, .y = 5000 };
    c.worlds[1].position = .{ .x = 8000, .y = 9000 };
    const frame = build(c, 0);
    try vec(.{ .x = 1.5, .y = 0.575, .z = 2 }, frame.fleets()[0].position);
}
test "scene progress rational conversion interpolation strength exceptional alpha and time" {
    var input = [_]m.Fleet{fleet(0)};
    var c = campaign(&input);
    c.worlds[0].position = .{ .x = 5000, .y = 5000 };
    c.worlds[1].position = .{ .x = 8000, .y = 9000 };
    input[0].route_length = 0;
    input[0].progress = 0;
    var frame = build(c, 0.5);
    try std.testing.expectEqual(@as(f32, 0), frame.fleets()[0].progress);
    try vec(.{ .x = 1.5, .y = 0.575, .z = 2 }, frame.fleets()[0].position);
    try std.testing.expectEqual(@as(f32, 0.08), frame.fleets()[0].strength);
    input[0].route_length = 2;
    input[0].progress = 1;
    input[0].strength = 200000;
    frame = build(c, 1);
    try std.testing.expectEqual(@as(f32, 0.5), frame.fleets()[0].progress);
    try vec(.{ .x = 3, .y = 0, .z = 4 }, frame.fleets()[0].position);
    try std.testing.expectEqual(@as(f32, 1), frame.fleets()[0].strength);
    for ([_]f32{ std.math.nan(f32), std.math.inf(f32), -1 }) |alpha| {
        frame = build(c, alpha);
        try vec(.{ .x = 1.5, .y = 0.575, .z = 2 }, frame.fleets()[0].position);
    }
    frame = build(c, 2);
    try vec(.{ .x = 3, .y = 0, .z = 4 }, frame.fleets()[0].position);
    input[0].progress = 3;
    frame = build(c, 0);
    try std.testing.expectEqual(@as(f32, 1), frame.fleets()[0].progress);
    input[0].route_length = std.math.maxInt(u64);
    input[0].progress = 9223372036854775808;
    frame = build(c, 0);
    // Independent rational: 2^63/(2^64-1) rounds to 0.5 in binary32.
    try std.testing.expectEqual(@as(f32, 0.5), frame.fleets()[0].progress);
    input[0].route_length = 11891602650592941854;
    input[0].progress = 4386542788665436018;
    frame = build(c, 0);
    // Independent exact rational rounded through binary64, contrasted with the
    // premature binary32 operand conversion; this pair separates the paths.
    const ratio: f32 = @floatCast(@as(f64, @floatFromInt(@as(u64, 4386542788665436018))) / @as(f64, @floatFromInt(@as(u64, 11891602650592941854))));
    const premature = @as(f32, @floatFromInt(input[0].progress)) / @as(f32, @floatFromInt(input[0].route_length));
    try std.testing.expect(ratio != premature);
    try std.testing.expectEqual(ratio, frame.fleets()[0].progress);
    const cam = camera.CameraState.init(.{ .x = 800, .y = 600 });
    for ([_]f32{ std.math.nan(f32), std.math.inf(f32), -1 }) |time| {
        frame = scene.buildScene(c, 0, 0, .{}, null, time, &cam, .{});
        try std.testing.expectEqual(@as(f32, 0), frame.visual_time);
    }
    input[0].route_length = std.math.maxInt(u64);
    input[0].progress = 9223372036854775808;
    frame = build(c, -@as(f32, 0));
    try std.testing.expectEqual(@as(f32, 0.5), frame.fleets()[0].progress);
}
test "scene preference grid copies CURRENT matrices parallax and arbitrary advisory score" {
    var cam = camera.CameraState.init(.{ .x = 800, .y = 600 });
    cam.yaw = 0.3;
    cam.pitch = 0.7;
    cam.target_matrices.view = math.Mat4.identity();
    const before = cam;
    const c = campaign(&.{});
    for ([_]prefs.MotionPreference{ .full, .reduced }) |motion| for ([_]prefs.GraphicsQuality{ .auto, .low, .high }) |quality| for ([_]bool{ false, true }) |contrast| {
        const frame = scene.buildScene(c, 0, 0, .{}, .{ .priority = 2, .score = -1 }, 12, &cam, .{ .motion = motion, .graphics_quality = quality, .high_contrast = contrast });
        const reduced = motion == .reduced;
        const high = quality != .low;
        try std.testing.expectEqualDeep(cam.current_matrices, frame.camera);
        try std.testing.expectEqualDeep(before, cam);
        try std.testing.expectEqual(reduced, frame.background.static_background);
        try std.testing.expectEqual(!reduced, frame.effects.animate_halos);
        try std.testing.expectEqual(high, frame.effects.bloom_requested);
        try std.testing.expectEqual(contrast, frame.effects.high_contrast);
        try std.testing.expectEqual(@as(u32, if (reduced) 0 else if (high) 512 else 96), frame.effects.particle_budget);
        try std.testing.expectEqual(@as(f32, if (reduced) 0 else 12), frame.visual_time);
        try std.testing.expectApproxEqAbs(@as(f32, if (reduced) 0 else @sin(@as(f32, 0.3)) * 0.025), frame.background.parallax.x, 0.000001);
        try std.testing.expectApproxEqAbs(@as(f32, if (reduced) 0 else @cos(@as(f32, 0.7)) * 0.025), frame.background.parallax.y, 0.000001);
        if (!contrast) for (frame.worlds) |w| {
            try std.testing.expectEqual(@as(u32, 0), w.ownership_pattern);
        };
        try std.testing.expectEqual(@as(f32, -1), frame.advisory.?.score);
    };
    for ([_]f32{ std.math.nan(f32), std.math.inf(f32) }) |score| {
        const frame = scene.buildScene(c, 0, 0, .{}, .{ .priority = 2, .score = score }, 0, &cam, .{});
        try std.testing.expect(if (std.math.isNan(score)) std.math.isNan(frame.advisory.?.score) else std.math.isPositiveInf(frame.advisory.?.score));
    }
    const snapshot = ctl.Snapshot{ .scores = .{ 0, 0.1, 0.2, 0.3, 0.4, 0.5, 0.6 }, .priority = 4, .metadata = .{ .device_epoch = 0, .request_id = 0, .model_version = 1, .source_tick = 0, .source_fingerprint = 0 }, .backend = .Cpu };
    try std.testing.expectEqualDeep(scene.SceneAdvisory{ .priority = 4, .score = 0.4 }, scene.SceneAdvisory.fromSnapshot(&snapshot));
}
test "scene independent integer seed source formula wrapping and deep frame value ownership" {
    // Reference translates scene.rs:94-100 with widened modulo arithmetic,
    // independently from the production wrapping operator sequence.
    for ([_]u64{ 0, 1, std.math.maxInt(u64), m.DEFAULT_SEED }) |fingerprint| {
        const mask: u128 = std.math.maxInt(u64);
        var v: u128 = ((fingerprint ^ @as(u64, 0x56495355414C5F31)) + @as(u128, 0x9E3779B97F4A7C15)) & mask;
        v = ((v ^ (v >> 30)) * 0xBF58476D1CE4E5B9) & mask;
        v = ((v ^ (v >> 27)) * 0x94D049BB133111EB) & mask;
        const expected: u64 = @intCast(v ^ (v >> 31));
        try std.testing.expectEqual(expected, scene.visualSeed(fingerprint));
    }
    const input = try std.testing.allocator.alloc(m.Fleet, 1);
    input[0] = fleet(7);
    var c = campaign(input);
    const a = build(c, 0);
    const b = a;
    input[0] = fleet(99);
    c.worlds[0].energy = 0;
    std.testing.allocator.free(input);
    try std.testing.expectEqual(@as(u64, 7), b.fleets()[0].id);
    try std.testing.expectEqualDeep(a.fleets(), b.fleets());
    try std.testing.expectEqualDeep(a.routes(), b.routes());
    try std.testing.expectEqualDeep(a.worlds, b.worlds);
}
