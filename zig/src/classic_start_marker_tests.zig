const std = @import("std");
const p = @import("classic_start_marker.zig");
const m = @import("model.zig");
const math = @import("classic_math.zig");
const camera = @import("classic_camera.zig");
const scene = @import("classic_scene.zig");
const generator = @import("generator.zig");
const testing = std.testing;
const Fixture = struct {
    worlds: [7]m.World,
    spheres: [7]scene.SceneWorld,
    fn init() Fixture {
        var c = generator.generate(m.DEFAULT_SEED, .{});
        const cam = camera.CameraState.init(.{ .x = 1440, .y = 900 });
        const f = scene.buildScene(.{ .worlds = c.worlds, .fleets = &.{}, .active_hazard = null }, 0, 0, .{}, null, 0, &cam, .{});
        var out = Fixture{ .worlds = c.worlds, .spheres = f.worlds };
        c.deinit(testing.allocator);
        for (&out.spheres, 0..) |*s, i| {
            s.position = .{ .x = if (i == 0) 0 else @as(f32, @floatFromInt(i)) * 10, .y = 0, .z = 0.8 };
            s.radius = 0.1;
        }
        return out;
    }
    fn input(f: *const Fixture) p.Input {
        const id = math.Mat4.identity();
        return .{ .step = .select_union_world, .playing = true, .viewport = .{ .x = 1440, .y = 900 }, .camera = .{ .view = id, .projection = id, .view_projection = id, .inverse_view_projection = id }, .scene_worlds = &f.spheres, .worlds = &f.worlds, .safe = .{ .min = .{ 0, 0 }, .max = .{ 1440, 900 } }, .edges = .{ .left = 0, .right = 0, .top = 0, .bottom = 0 }, .settings = false, .zones = &.{} };
    }
};
fn marker(i: p.Input) !p.Marker {
    const r = try p.project(i);
    try testing.expect(r == .marker);
    return r.marker;
}
test "marker selection pick occlusion first source and reordered campaign" {
    var f = Fixture.init();
    var i = f.input();
    var value = try marker(i);
    try testing.expectEqual(p.Mode.scene_linked, value.mode);
    try testing.expectEqual(@as(f32, 630), value.label.min[0]);
    try testing.expectEqual(@as(f32, 358), value.label.min[1]);
    f.spheres[1].position = .{ .x = 0, .y = 0, .z = 0.3 };
    value = try marker(i);
    try testing.expectEqual(p.Mode.action_only, value.mode);
    try testing.expect(value.connector_start == null and value.ring_radius == null);
    f.worlds[2].owner = .union_faction;
    f.spheres[2].position = .{ .x = 0.4, .y = 0, .z = 0.8 };
    try testing.expectEqual(@as(u8, 0), (try marker(i)).world);
    std.mem.swap(scene.SceneWorld, &f.spheres[0], &f.spheres[2]);
    try testing.expectEqual(@as(u8, 2), (try marker(i)).world);
    std.mem.swap(m.World, &f.worlds[0], &f.worlds[6]);
    try testing.expectEqual(@as(u8, 2), (try marker(i)).world);
    i.camera.inverse_view_projection.cols[2].z = 2;
    try testing.expectEqual(p.Mode.scene_linked, (try marker(i)).mode);
    f.spheres[2].position.z = 0;
    std.mem.swap(scene.SceneWorld, &f.spheres[0], &f.spheres[2]);
    f.worlds[2].owner = null;
    f.spheres[1].position.x = 10;
    try testing.expectEqual(p.Mode.scene_linked, (try marker(i)).mode);
}
test "marker projection depth flags and finite singular camera" {
    var f = Fixture.init();
    var i = f.input();
    f.spheres[0].position.z = 50;
    try testing.expectEqual(@as(u8, 0), (try marker(i)).world);
    i.camera.view_projection.cols[3].w = 0;
    try testing.expectEqual(p.Absent.no_label_candidate, (try p.project(i)).absent);
    i.camera.view_projection.cols[3].w = -1;
    try testing.expectEqual(p.Absent.no_label_candidate, (try p.project(i)).absent);
    i = f.input();
    i.camera.inverse_view_projection.cols[2].z = 0;
    try testing.expectEqual(p.Mode.action_only, (try marker(i)).mode);
    i = f.input();
    i.step = null;
    try testing.expectEqual(p.Absent.not_initial, (try p.project(i)).absent);
    i.step = .select_union_world;
    i.settings = true;
    try testing.expectEqual(p.Absent.not_playing, (try p.project(i)).absent);
    i.settings = false;
    i.playing = false;
    try testing.expectEqual(p.Absent.not_playing, (try p.project(i)).absent);
    i = f.input();
    f.spheres[0].position.x = 2;
    try testing.expectEqual(p.Absent.no_label_candidate, (try p.project(i)).absent);
    f.spheres[0].position.x = -1;
    try testing.expectEqual(p.Mode.action_only, (try marker(i)).mode);
    i = f.input();
    i.camera.view_projection.cols[0].x = std.math.floatMax(f32);
    f.spheres[0].position.x = 2;
    try testing.expectEqual(p.Absent.no_label_candidate, (try p.project(i)).absent);
}
test "marker clamp exact touch tiny inset and ring policy" {
    var f = Fixture.init();
    var i = f.input();
    const touch = p.Rect{ .min = .{ 810, 358 }, .max = .{ 820, 412 } };
    var zones = [_]p.Zone{.{ .kind = .control, .bounds = touch, .enabled = true }};
    i.zones = &zones;
    try testing.expectEqual(p.Absent.no_label_candidate, (try p.project(i)).absent);
    zones[0].enabled = false;
    _ = try marker(i);
    zones[0].bounds.min[0] = 809.999;
    try testing.expectEqual(p.Absent.no_label_candidate, (try p.project(i)).absent);
    zones[0] = .{ .kind = .guidance, .bounds = touch, .enabled = true };
    _ = try marker(i);
    zones[0].kind = .tooltip;
    _ = try marker(i);
    zones[0].kind = .tray;
    try testing.expectEqual(p.Absent.no_label_candidate, (try p.project(i)).absent);
    zones[0] = .{ .kind = .control, .bounds = .{ .min = .{ 742, 450 }, .max = .{ 750, 460 } }, .enabled = true };
    try testing.expectEqual(p.Mode.action_only, (try marker(i)).mode);
    i = f.input();
    i.edges.left = 720;
    try testing.expectEqual(p.Mode.action_only, (try marker(i)).mode);
    i = f.input();
    i.safe = .{ .min = .{ 0, 0 }, .max = .{ 182, 56 } };
    const value = try marker(i);
    try testing.expectEqual(@as(f32, 1), value.label.min[0]);
    try testing.expectEqual(@as(f32, 181), value.label.max[0]);
    try testing.expectEqual(@as(f32, 55), value.label.max[1]);
    i.safe.max[0] = 181;
    try testing.expectEqual(p.Absent.no_label_candidate, (try p.project(i)).absent);
    i.safe = .{ .min = .{ 2000, 2000 }, .max = .{ 2000, 2000 } };
    try testing.expectEqual(p.Absent.no_label_candidate, (try p.project(i)).absent);
}
test "marker typed admission validates inactive input and owns names" {
    var f = Fixture.init();
    var i = f.input();
    const before = f;
    const value = try marker(i);
    try testing.expectEqualDeep(before, f);
    f.worlds[0].name = try m.WorldName.init("REUSED");
    try testing.expectEqualStrings("ASTER VALE", value.name.asSlice());
    f = before;
    i.step = null;
    i.viewport.x = 0;
    try testing.expectError(error.InvalidViewport, p.project(i));
    try testing.expectEqualDeep(before, f);
    i = f.input();
    i.viewport.y = -1;
    try testing.expectError(error.InvalidViewport, p.project(i));
    i.viewport.y = std.math.inf(f32);
    try testing.expectError(error.InvalidViewport, p.project(i));
    i = f.input();
    f.worlds[0].id = 7;
    try testing.expectError(error.InvalidWorldSet, p.project(i));
    f = before;
    f.worlds[0].id = 1;
    try testing.expectError(error.InvalidWorldSet, p.project(i));
    f = before;
    f.spheres[0].id = 1;
    try testing.expectError(error.InvalidWorldSet, p.project(i));
    f = before;
    f.spheres[0].id = 7;
    try testing.expectError(error.InvalidWorldSet, p.project(i));
    f = before;
    f.worlds[0].name.len = 25;
    try testing.expectError(error.InvalidWorldName, p.project(i));
    f = before;
    f.worlds[0].name.bytes[0] = 0xff;
    try testing.expectError(error.InvalidWorldName, p.project(i));
    f = before;
    f.worlds[0].name.bytes[0] = ' ';
    try testing.expectError(error.InvalidWorldName, p.project(i));
    f = before;
    for ([_]f32{ 0, -1, std.math.inf(f32), std.math.floatMax(f32) }) |radius| {
        f.spheres[0].radius = radius;
        try testing.expectError(error.InvalidWorldSet, p.project(i));
    }
    f = before;
    i.safe.max[0] = std.math.nan(f32);
    try testing.expectError(error.InvalidBounds, p.project(i));
    i = f.input();
    i.safe.min[0] = 2;
    i.safe.max[0] = 1;
    try testing.expectError(error.InvalidBounds, p.project(i));
    i = f.input();
    i.camera.view.cols[0].x = std.math.nan(f32);
    try testing.expectError(error.InvalidCamera, p.project(i));
    i = f.input();
    i.edges.left = -1;
    try testing.expectError(error.InvalidEdges, p.project(i));
    var zones: [27]p.Zone = @splat(.{ .kind = .tooltip, .bounds = .{ .min = .{ 0, 0 }, .max = .{ 0, 0 } }, .enabled = false });
    i = f.input();
    i.zones = zones[0..26];
    _ = try marker(i);
    zones[26].bounds.min[0] = std.math.nan(f32);
    i.zones = &zones;
    try testing.expectError(error.ZoneLimit, p.project(i));
}
fn canonical(x: f32, y: f32) p.Marker {
    @setFloatMode(.strict);
    return .{ .world = 0, .name = m.WorldName.init("A") catch unreachable, .point = .{ .x = 0, .y = 0 }, .label = .{ .min = .{ x, y }, .max = .{ x + 180, y + 54 } }, .mode = .action_only, .connector_start = null, .ring_radius = null };
}
test "marker canonical fractional endpoints option combinations and lost extent" {
    var v = canonical(173.8, 225.64);
    try testing.expectEqual(@as(f32, 353.79998779296875), v.label.max[0]);
    try testing.expectEqual(@as(f32, 279.6400146484375), v.label.max[1]);
    try testing.expect(v.label.max[0] - v.label.min[0] != 180);
    try testing.expect(v.label.max[1] - v.label.min[1] != 54);
    try p.validateMarkerGeometry(v);
    v = canonical(225.64, 225.64);
    try testing.expectEqual(@as(f32, 405.6400146484375), v.label.max[0]);
    try p.validateMarkerGeometry(v);
    v.mode = .scene_linked;
    try testing.expectError(error.InvalidMarkerGeometry, p.validateMarkerGeometry(v));
    v.connector_start = .{ .x = v.label.min[0] + 90, .y = v.label.min[1] + 54 };
    try testing.expectError(error.InvalidMarkerGeometry, p.validateMarkerGeometry(v));
    v.ring_radius = 22;
    try p.validateMarkerGeometry(v);
    v.mode = .action_only;
    try testing.expectError(error.InvalidMarkerGeometry, p.validateMarkerGeometry(v));
    v.connector_start = null;
    try testing.expectError(error.InvalidMarkerGeometry, p.validateMarkerGeometry(v));
    v.mode = .scene_linked;
    v.connector_start = .{ .x = 0, .y = 0 };
    try testing.expectError(error.InvalidMarkerGeometry, p.validateMarkerGeometry(v));
    v = canonical(0, 0);
    v.mode = .scene_linked;
    v.connector_start = .{ .x = 90, .y = 54 };
    v.ring_radius = 21;
    try testing.expectError(error.InvalidMarkerGeometry, p.validateMarkerGeometry(v));
    v.ring_radius = std.math.nan(f32);
    try testing.expectError(error.NonFinite, p.validateMarkerGeometry(v));
    v = canonical(173.8, 0);
    v.label.max[0] = @bitCast(@as(u32, @bitCast(v.label.max[0])) + 1);
    try testing.expectError(error.InvalidMarkerGeometry, p.validateMarkerGeometry(v));
    v = canonical(0, 0);
    v.label.max[0] = -1;
    try testing.expectError(error.InvalidMarkerGeometry, p.validateMarkerGeometry(v));
    v = canonical(1e30, 0);
    try testing.expectError(error.ArithmeticOverflow, p.validateMarkerGeometry(v));
    v = canonical(0, 0);
    v.point.x = std.math.inf(f32);
    try testing.expectError(error.NonFinite, p.validateMarkerGeometry(v));
}
// Independent accepted HUD formulas, not a product UI import or projector oracle.
fn hudZones(v: math.Vec2, s: f32) ![18]p.Zone {
    @setFloatMode(.strict);
    var out: [18]p.Zone = undefined;
    const columns: usize = if (v.x >= 1100) 9 else 5;
    const rows: usize = if (columns == 9) 1 else 2;
    const extent = @max(@as(f32, 48) * s, @as(f32, 44));
    const width = @min(if (columns == 9) @max(@as(f32, 92) * s, @as(f32, 72)) else @max(@as(f32, 124) * s, @as(f32, 104)), (v.x - 16 - @as(f32, @floatFromInt(columns - 1)) * 6) / @as(f32, @floatFromInt(columns)));
    for (0..9) |j| {
        const row = j / columns;
        const col = j % columns;
        const count = @min(9 - row * columns, columns);
        const rw = @as(f32, @floatFromInt(count)) * width + @as(f32, @floatFromInt(count - 1)) * 6;
        const start = @max((v.x - rw) * 0.5, @as(f32, 4));
        const y = @max(v.y - @as(f32, @floatFromInt(rows)) * extent - @as(f32, @floatFromInt(rows - 1)) * 6 - 4, @as(f32, 0));
        out[j] = .{ .kind = .control, .enabled = j >= 5, .bounds = try p.Rect.fromXYWH(start + @as(f32, @floatFromInt(col)) * (width + 6), y + @as(f32, @floatFromInt(row)) * (extent + 6), width, extent) };
    }
    out[9] = .{ .kind = .control, .enabled = true, .bounds = try p.Rect.fromXYWH(v.x - 116, 8, 108, 44) };
    const fw = @max(@as(f32, 72) * s, @as(f32, 44));
    const fh = @max(@as(f32, 44) * s, @as(f32, 44));
    const fx = @max(v.x - 268 * s, @as(f32, 4));
    for (0..3) |j| out[10 + j] = .{ .kind = .control, .enabled = true, .bounds = try p.Rect.fromXYWH(fx + @as(f32, @floatFromInt(j)) * (fw + 4), 236 * s, fw, fh) };
    out[13] = .{ .kind = .control, .enabled = false, .bounds = try p.Rect.fromXYWH(fx, 286 * s, fw * 1.5, fh) };
    out[14] = .{ .kind = .control, .enabled = false, .bounds = try p.Rect.fromXYWH(fx + fw * 1.5 + 4, 286 * s, fw * 1.5, fh) };
    out[15] = .{ .kind = .control, .enabled = true, .bounds = try p.Rect.fromXYWH(@max(v.x - 276 * s, @as(f32, 0)), 380 * s, @max(@as(f32, 268) * s, @as(f32, 44)), @max(@as(f32, 150) * s, @as(f32, 44))) };
    const tw = @min(@max(@as(f32, 448) * s, @as(f32, 344)), @max(v.x - 16, @as(f32, 1)));
    out[16] = .{ .kind = .tray, .enabled = true, .bounds = try p.Rect.fromXYWH((v.x - tw) * 0.5, @max(@as(f32, 70) * s, @as(f32, 52)), tw, @max(@as(f32, 108) * s, @as(f32, 92))) };
    const gw = @min(@max(@as(f32, 448) * s, @as(f32, 344)), @max(v.x - 132, @as(f32, 1)));
    out[17] = .{ .kind = .guidance, .enabled = true, .bounds = try p.Rect.fromXYWH((v.x - gw) * 0.5, 8, gw, @max(@as(f32, 52) * s, @as(f32, 44))) };
    return out;
}
test "marker independent twelve default generator camera HUD policies" {
    @setFloatMode(.strict);
    var c = generator.generate(m.DEFAULT_SEED, .{});
    defer c.deinit(testing.allocator);
    try testing.expectEqual(@as(i32, 1037), c.worlds[0].position.x);
    try testing.expectEqual(@as(i32, 5027), c.worlds[0].position.y);
    try testing.expectEqual(@as(u64, 80000), c.worlds[0].energy);
    try testing.expectEqual(@as(u64, 43735), c.worlds[0].defense);
    const positions = [7][2]i32{ .{ 1037, 5027 }, .{ 8793, 2818 }, .{ 8517, 7827 }, .{ 4811, 1320 }, .{ 4905, 8955 }, .{ 3746, 3709 }, .{ 5925, 6136 } };
    for (c.worlds, positions) |w, xy| {
        try testing.expectEqual(xy[0], w.position.x);
        try testing.expectEqual(xy[1], w.position.y);
    }
    const viewports = [_]math.Vec2{ .{ .x = 723, .y = 802 }, .{ .x = 960, .y = 600 }, .{ .x = 1440, .y = 900 } };
    const points = [_]math.Vec2{ .{ .x = 105.464, .y = 402.429 }, .{ .x = 288.452, .y = 301.069 }, .{ .x = 432.678, .y = 451.604 } };
    const label_x = [3][4]f32{ .{ 173.8, 173.8, 199.72, 225.64 }, .{ 198.452, 198.452, 199.72, 225.64 }, .{ 342.678, 342.678, 342.678, 342.678 } };
    const label_y = [_]f32{ 310.429, 209.069, 359.604 };
    for (viewports, 0..) |v, vi| {
        const cam = camera.CameraState.init(v);
        const frame = scene.buildScene(.{ .worlds = c.worlds, .fleets = &.{}, .active_hazard = null }, 0, 0, .{}, null, 0, &cam, .{});
        try testing.expectApproxEqAbs(@as(f32, -3.963), frame.worlds[0].position.x, 0.00001);
        try testing.expectApproxEqAbs(@as(f32, 0.027), frame.worlds[0].position.z, 0.00001);
        try testing.expectApproxEqAbs(@as(f32, 0.368241), frame.worlds[0].radius, 0.000001);
        for ([_]f32{ 0.85, 1, 1.15, 1.30 }, 0..) |factor, fi| {
            const s = std.math.clamp(std.math.clamp(@min(v.x / 1440, v.y / 900), @as(f32, 0.72), @as(f32, 1.15)) * factor, @as(f32, 0.60), @as(f32, 1.50));
            const legacy = @max(s, @as(f32, 0.72));
            const edges = p.OpaqueEdges{ .left = 240 * legacy, .right = 280 * legacy, .top = 64 * legacy, .bottom = if (v.x < 1100) 102 else 56 * legacy };
            const zones = try hudZones(v, s);
            const i = p.Input{ .step = .select_union_world, .playing = true, .viewport = v, .camera = cam.current_matrices, .scene_worlds = &frame.worlds, .worlds = &c.worlds, .safe = .{ .min = .{ edges.left, edges.top }, .max = .{ v.x - edges.right, v.y - edges.bottom } }, .edges = edges, .settings = false, .zones = &zones };
            const value = try marker(i);
            try testing.expectEqual(@as(u8, 0), value.world);
            try testing.expectEqualStrings("ASTER VALE", value.name.asSlice());
            try testing.expectApproxEqAbs(points[vi].x, value.point.x, 0.02);
            try testing.expectApproxEqAbs(points[vi].y, value.point.y, 0.02);
            try testing.expectApproxEqAbs(label_x[vi][fi], value.label.min[0], 0.02);
            try testing.expectApproxEqAbs(label_y[vi], value.label.min[1], 0.02);
            try testing.expectEqual(if (vi == 0) p.Mode.action_only else p.Mode.scene_linked, value.mode);
            try p.validateMarkerGeometry(value);
            if (vi == 0) {
                try testing.expect(value.connector_start == null and value.ring_radius == null);
            } else {
                try testing.expectEqual(@as(f32, 22), value.ring_radius.?);
                try testing.expectEqual(value.label.min[0] + 90, value.connector_start.?.x);
                try testing.expectEqual(value.label.min[1] + 54, value.connector_start.?.y);
                const pick = @import("classic_picking.zig");
                var spheres: [7]pick.PickWorld = undefined;
                for (frame.worlds, 0..) |w, j| spheres[j] = .{ .id = w.id, .position = w.position, .radius = w.radius };
                const ray = pick.Ray.fromLogicalPointer(value.point, v, cam.current_matrices.inverse_view_projection).?;
                try testing.expectEqual(@as(u8, 0), pick.pickWorld(ray, &spheres).?.world);
            }
        }
    }
}
test "marker native named value sizes" {
    const builtin = @import("builtin");
    std.debug.print("TARGET arch={s} os={s} mode={s}\n", .{ @tagName(builtin.cpu.arch), @tagName(builtin.os.tag), @tagName(builtin.mode) });
    std.debug.print("TARGET sizes Marker={d} Result={d} Input={d} CameraMatrices={d} PickWorld7={d} Zone26={d} WorldName={d}\n", .{ @sizeOf(p.Marker), @sizeOf(p.Result), @sizeOf(p.Input), @sizeOf(camera.CameraMatrices), @sizeOf([7]@import("classic_picking.zig").PickWorld), @sizeOf([26]p.Zone), @sizeOf(m.WorldName) });
    try testing.expect(@sizeOf(p.Marker) <= 128);
}
test "marker reads used lanes only immutable shared source and malformed zones" {
    var f = Fixture.init();
    var i = f.input();
    f.worlds[0].name.bytes[10..].* = undefined;
    f.spheres[0].ownership_color = undefined;
    f.spheres[0].fields = undefined;
    f.spheres[0].energy = undefined;
    const a = try marker(i);
    const b = try marker(i);
    try testing.expectEqualDeep(a, b);
    var z = [_]p.Zone{.{ .kind = .help, .enabled = false, .bounds = .{ .min = .{ 0, 0 }, .max = .{ std.math.inf(f32), 1 } } }};
    i.zones = &z;
    try testing.expectError(error.InvalidBounds, p.project(i));
    i = f.input();
    i.edges.top = std.math.nan(f32);
    try testing.expectError(error.InvalidEdges, p.project(i));
    i = f.input();
    f.spheres[1].position.x = std.math.nan(f32);
    try testing.expectError(error.InvalidWorldSet, p.project(i));
}
test "marker reachable projection lost label extent typed overflow" {
    var f = Fixture.init();
    var i = f.input();
    i.viewport.x = 1e30;
    i.safe.max[0] = 1e30;
    try testing.expectError(error.ArithmeticOverflow, p.project(i));
}
test "marker complete optional presence matrix and exact positive clip w" {
    for ([_]p.Mode{ .action_only, .scene_linked }) |mode| {
        for ([_]bool{ false, true }) |connector| {
            for ([_]bool{ false, true }) |radius| {
                var value = canonical(173.8, 225.64);
                value.mode = mode;
                value.connector_start = if (connector) .{ .x = value.label.min[0] + 90, .y = value.label.min[1] + 54 } else null;
                value.ring_radius = if (radius) 22 else null;
                if ((mode == .action_only and !connector and !radius) or (mode == .scene_linked and connector and radius)) {
                    try p.validateMarkerGeometry(value);
                } else try testing.expectError(error.InvalidMarkerGeometry, p.validateMarkerGeometry(value));
            }
        }
    }
    var value = canonical(0, 0);
    value.label.max[0] = 0;
    try testing.expectError(error.InvalidMarkerGeometry, p.validateMarkerGeometry(value));
    value = canonical(-0.0, -0.0);
    try p.validateMarkerGeometry(value);
    var f = Fixture.init();
    var i = f.input();
    i.camera.view_projection.cols[3].w = std.math.floatEps(f32) * 0.5;
    try testing.expectEqual(@as(u8, 0), (try marker(i)).world);
    for (&f.worlds) |*w| w.owner = null;
    try testing.expectEqual(p.Absent.no_label_candidate, (try p.project(i)).absent);
}
fn hugeCollapsed(vertical: bool) !void {
    var f = Fixture.init();
    const original = f;
    var i = f.input();
    if (vertical) {
        i.viewport.y = 1e30;
        i.safe.min[1] = 1e30;
        i.safe.max[1] = 1e30;
    } else {
        i.viewport.x = 1e30;
        i.safe.min[0] = 1e30;
        i.safe.max[0] = 1e30;
    }
    try testing.expectEqual(p.Absent.no_label_candidate, (try p.project(i)).absent);
    try testing.expectEqualDeep(original, f);
    i.step = null;
    i.playing = false;
    i.settings = true;
    try testing.expectEqual(p.Absent.not_initial, (try p.project(i)).absent);
    i.step = .select_union_world;
    try testing.expectEqual(p.Absent.not_playing, (try p.project(i)).absent);
    i.settings = false;
    try testing.expectEqual(p.Absent.not_playing, (try p.project(i)).absent);
    i.playing = true;
    i.settings = true;
    try testing.expectEqual(p.Absent.not_playing, (try p.project(i)).absent);
    i.step = null;
    i.viewport.x = 0;
    try testing.expectError(error.InvalidViewport, p.project(i));
    try testing.expectEqualDeep(original, f);
    i = f.input();
    i.safe.min[0] = 1e30;
    i.safe.max[0] = 1e30;
    i.step = null;
    f.worlds[0].name.len = 25;
    try testing.expectError(error.InvalidWorldName, p.project(i));
    f = original;
    i.camera.view.cols[0].x = std.math.nan(f32);
    try testing.expectError(error.InvalidCamera, p.project(i));
    try testing.expectEqualDeep(original, f);
}

test "marker huge finite collapsed x safe absence validates before flags" {
    try hugeCollapsed(false);
}
test "marker huge finite collapsed y safe absence validates before flags" {
    try hugeCollapsed(true);
}
