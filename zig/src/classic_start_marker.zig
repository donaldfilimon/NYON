//! Pure, allocation-free Classic start-marker projection; caller owns provenance.
const std = @import("std");
const math = @import("classic_math.zig");
const model = @import("model.zig");
const camera_model = @import("classic_camera.zig");
const scene = @import("classic_scene.zig");
const picking = @import("classic_picking.zig");
const onboarding = @import("classic_onboarding.zig");
pub const Rect = @import("retained_geometry.zig").Rect;
pub const ZoneKind = enum { control, tray, help, guidance, tooltip };
pub const Zone = struct { kind: ZoneKind, bounds: Rect, enabled: bool };
pub const OpaqueEdges = struct { left: f32, right: f32, top: f32, bottom: f32 };
pub const Input = struct { step: ?onboarding.Step, playing: bool, viewport: math.Vec2, camera: camera_model.CameraMatrices, scene_worlds: *const [7]scene.SceneWorld, worlds: *const [7]model.World, safe: Rect, edges: OpaqueEdges, settings: bool, zones: []const Zone };
pub const Mode = enum { scene_linked, action_only };
pub const Marker = struct { world: model.WorldId, name: model.WorldName, point: math.Vec2, label: Rect, mode: Mode, connector_start: ?math.Vec2, ring_radius: ?f32 };
pub const Absent = enum { not_initial, not_playing, no_label_candidate };
pub const Result = union(enum) { marker: Marker, absent: Absent };
pub const InputError = error{ InvalidWorldSet, InvalidWorldName, InvalidViewport, InvalidCamera, InvalidBounds, InvalidEdges, ZoneLimit, ArithmeticOverflow };
pub const ProjectError = InputError;
pub const MarkerGeometryError = error{ NonFinite, ArithmeticOverflow, InvalidMarkerGeometry };
comptime {
    if (@sizeOf(Marker) > 128) @compileError("Marker exceeds 128 bytes");
}
pub fn validateMarkerGeometry(value: Marker) MarkerGeometryError!void {
    @setFloatMode(.strict);
    if (!value.point.isFinite()) return error.NonFinite;
    for (value.label.min, value.label.max) |lo, hi| if (!std.math.isFinite(lo) or !std.math.isFinite(hi)) return error.NonFinite;
    if (value.connector_start) |v| {
        if (!v.isFinite()) return error.NonFinite;
    }
    if (value.ring_radius) |r| {
        if (!std.math.isFinite(r)) return error.NonFinite;
    }
    const expected: [2]f32 = .{ value.label.min[0] + 180, value.label.min[1] + 54 };
    for (expected, value.label.min) |hi, lo| if (!std.math.isFinite(hi) or hi <= lo) return error.ArithmeticOverflow;
    if (value.label.max[0] != expected[0] or value.label.max[1] != expected[1]) return error.InvalidMarkerGeometry;
    switch (value.mode) {
        .action_only => if (value.connector_start != null or value.ring_radius != null) return error.InvalidMarkerGeometry,
        .scene_linked => {
            const c = value.connector_start orelse return error.InvalidMarkerGeometry;
            const r = value.ring_radius orelse return error.InvalidMarkerGeometry;
            const x = value.label.min[0] + 90;
            const y = value.label.min[1] + 54;
            if (!std.math.isFinite(x) or !std.math.isFinite(y)) return error.ArithmeticOverflow;
            if (r != 22 or c.x != x or c.y != y) return error.InvalidMarkerGeometry;
        },
    }
}
fn validRect(r: Rect) InputError!void {
    _ = Rect.fromBounds(r.min, r.max) catch return error.InvalidBounds;
}
fn checked(v: f32) InputError!f32 {
    return if (std.math.isFinite(v)) v else error.ArithmeticOverflow;
}
fn blocked(r: Rect, zones: []const Zone) bool {
    for (zones) |z| {
        const closed = z.kind == .tray or z.kind == .help or (z.kind == .control and z.enabled);
        if (closed) {
            if (r.min[0] <= z.bounds.max[0] and r.max[0] >= z.bounds.min[0] and r.min[1] <= z.bounds.max[1] and r.max[1] >= z.bounds.min[1]) return true;
        } else if (r.intersection(z.bounds) != null) return true;
    }
    return false;
}
pub fn project(input: Input) InputError!Result {
    @setFloatMode(.strict);
    if (input.zones.len > 26) return error.ZoneLimit;
    var model_ids: u8 = 0;
    var scene_ids: u8 = 0;
    var names: [7]model.WorldName = undefined;
    for (input.worlds) |*w| {
        if (w.id >= 7) return error.InvalidWorldSet;
        const bit = @as(u8, 1) << @as(u3, @intCast(w.id));
        if (model_ids & bit != 0) return error.InvalidWorldSet;
        model_ids |= bit;
        if (w.name.len == 0 or w.name.len > 24) return error.InvalidWorldName;
        names[w.id] = model.WorldName.init(w.name.asSlice()) catch return error.InvalidWorldName;
    }
    var pick_worlds: [7]picking.PickWorld = undefined;
    for (input.scene_worlds, 0..) |*w, i| {
        if (w.id >= 7) return error.InvalidWorldSet;
        const bit = @as(u8, 1) << @as(u3, @intCast(w.id));
        if (scene_ids & bit != 0) return error.InvalidWorldSet;
        scene_ids |= bit;
        if (!w.position.isFinite() or !std.math.isFinite(w.radius) or w.radius <= 0 or !std.math.isFinite(w.radius * w.radius)) return error.InvalidWorldSet;
        pick_worlds[i] = .{ .id = w.id, .position = w.position, .radius = w.radius };
    }
    if (!input.viewport.isFinite() or input.viewport.x <= 0 or input.viewport.y <= 0) return error.InvalidViewport;
    if (!input.camera.isFinite()) return error.InvalidCamera;
    try validRect(input.safe);
    for (input.safe.min, input.safe.max, [_]f32{ input.viewport.x, input.viewport.y }) |lo, hi, extent| if (lo != hi and (lo < 0 or hi > extent)) return error.InvalidBounds;
    for (input.zones) |z| try validRect(z.bounds);
    for ([_]f32{ input.edges.left, input.edges.right, input.edges.top, input.edges.bottom }) |v| if (!std.math.isFinite(v) or v < 0) return error.InvalidEdges;
    if (input.step != .select_union_world) return .{ .absent = .not_initial };
    if (!input.playing or input.settings) return .{ .absent = .not_playing };
    // A collapsed finite axis has no positive label extent, even when f32
    // rounding erases the inset arithmetic at large coordinates.
    if (input.safe.min[0] == input.safe.max[0] or input.safe.min[1] == input.safe.max[1]) return .{ .absent = .no_label_candidate };
    const inset = Rect{ .min = .{ try checked(input.safe.min[0] + 1), try checked(input.safe.min[1] + 1) }, .max = .{ try checked(input.safe.max[0] - 1), try checked(input.safe.max[1] - 1) } };
    const max_min: [2]f32 = .{ try checked(inset.max[0] - 180), try checked(inset.max[1] - 54) };
    if (max_min[0] < inset.min[0] or max_min[1] < inset.min[1]) return .{ .absent = .no_label_candidate };
    const viewport = Rect{ .min = .{ 0, 0 }, .max = .{ input.viewport.x, input.viewport.y } };
    for (input.scene_worlds) |*w| {
        var union_owned = false;
        for (input.worlds) |*campaign_world| if (campaign_world.id == w.id) {
            union_owned = campaign_world.owner == .union_faction;
            break;
        };
        if (!union_owned) continue;
        const clip = input.camera.view_projection.mulVec4(.{ .x = w.position.x, .y = w.position.y, .z = w.position.z, .w = 1 });
        if (!clip.isFinite() or clip.w <= 0) continue;
        const point = math.Vec2{ .x = (clip.x / clip.w + 1) * input.viewport.x * 0.5, .y = (1 - clip.y / clip.w) * input.viewport.y * 0.5 };
        if (!point.isFinite() or !viewport.contains(.{ point.x, point.y })) continue;
        const source_x = std.math.clamp(try checked(point.x - 90), @as(f32, 8), @max(try checked(input.viewport.x - 188), @as(f32, 8)));
        const source_y = @min(@max(try checked(point.y - 92), @as(f32, 190)), @max(try checked(input.viewport.y - 180), @as(f32, 190)));
        const x = std.math.clamp(source_x, inset.min[0], max_min[0]);
        const y = std.math.clamp(source_y, inset.min[1], max_min[1]);
        const label = Rect{ .min = .{ x, y }, .max = .{ try checked(x + 180), try checked(y + 54) } };
        if (!inset.containsRect(label) or !viewport.containsRect(label) or blocked(label, input.zones)) continue;
        const ring = Rect{ .min = .{ try checked(point.x - 22), try checked(point.y - 22) }, .max = .{ try checked(point.x + 22), try checked(point.y + 22) } };
        var linked = ring.max[0] > ring.min[0] and ring.max[1] > ring.min[1] and input.safe.contains(.{ point.x, point.y }) and input.safe.containsRect(ring) and !blocked(ring, input.zones);
        if (point.x <= input.edges.left or point.x >= input.viewport.x - input.edges.right or point.y <= input.edges.top or point.y >= input.viewport.y - input.edges.bottom) linked = false;
        for (input.zones) |z| if ((z.kind == .tray or z.kind == .help or (z.kind == .control and z.enabled)) and z.bounds.contains(.{ point.x, point.y })) {
            linked = false;
        };
        if (linked) {
            if (picking.Ray.fromLogicalPointer(point, input.viewport, input.camera.inverse_view_projection)) |ray| {
                if (picking.pickWorld(ray, &pick_worlds)) |hit| {
                    linked = hit.world == w.id;
                } else linked = false;
            } else linked = false;
        }
        const marker = Marker{ .world = w.id, .name = names[w.id], .point = point, .label = label, .mode = if (linked) .scene_linked else .action_only, .connector_start = if (linked) .{ .x = try checked(x + 90), .y = try checked(y + 54) } else null, .ring_radius = if (linked) 22 else null };
        validateMarkerGeometry(marker) catch |err| return switch (err) {
            error.ArithmeticOverflow => error.ArithmeticOverflow,
            else => error.InvalidBounds,
        };
        return .{ .marker = marker };
    }
    return .{ .absent = .no_label_candidate };
}
