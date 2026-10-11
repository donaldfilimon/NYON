//! Synchronous borrowed HUD/Core adaptation. Caller owns the coherent lease.
const std = @import("std");
const ui = @import("classic_ui.zig");
const marker = @import("classic_start_marker.zig");
const app = @import("classic_app.zig");
const retained = @import("retained_semantics.zig");
const math = @import("classic_math.zig");
const geometry = @import("retained_geometry.zig");
const onboarding = @import("classic_onboarding.zig");
pub const CurrentLease = struct { core: *const app.AppCore, step: ?onboarding.Step, stamp: ui.CurrentStamp };
/// Borrowed resident; zero retains. Context/zones are fixed copied values.
/// Must not outlive, alias or license mutation of the prepared source.
pub const BorrowedHudSnapshot = struct {
    revision: u64,
    resident: retained.TokenRef,
    hold: ?u64,
    context: [128]u8,
    context_len: usize,
    viewport: math.Vec2,
    safe: geometry.Rect,
    edges: marker.OpaqueEdges,
    settings: bool,
    zones: [26]marker.Zone,
    zone_count: usize,
};
pub const SnapshotError = error{ EmptyProjection, InvalidHudGeometry, ZoneLimit, SourceMismatch };
pub const AdapterError = marker.InputError || error{SourceMismatch};
const hud_keys = .{ "speed_down", "pause", "speed_up", "launch", "clear_selection", "reset_camera", "settings", "help", "scenario", "skip", "field/atmosphere", "field/hydrosphere", "field/topology", "field/decrease", "field/increase", "advisory_ack", "reset_guidance" };
const settings_keys = .{ "scale/85", "scale/100", "scale/115", "scale/130", "motion", "contrast", "quality/auto", "quality/low", "quality/high", "reset_guidance", "close" };
fn eql(a: []const u8, b: []const u8) bool {
    return std.mem.eql(u8, a, b);
}
fn rect(r: geometry.Rect) SnapshotError!void {
    _ = geometry.Rect.fromBounds(r.min, r.max) catch return error.InvalidHudGeometry;
}
fn actionOrdinal(settings: bool, id: []const u8) ?usize {
    if (settings) {
        inline for (settings_keys, 0..) |key, i| if (eql(id, "a/settings/" ++ key)) return i;
    } else {
        inline for (hud_keys, 0..) |key, i| if (eql(id, "a/hud/" ++ key)) return i;
    }
    return null;
}
fn scalarText(n: retained.NodeSpec) bool {
    return std.unicode.utf8ValidateSlice(n.name) and std.unicode.utf8ValidateSlice(n.description) and (if (n.value) |v| std.unicode.utf8ValidateSlice(v) else true);
}
fn group(input: retained.FrameInput, index: usize, role: retained.NodeRole, parent: usize, child_names: []const []const u8, bounded: bool) SnapshotError!void {
    const n = input.nodes[index];
    if (n.role != role or n.parent != parent or n.action != null or n.focusable or n.child_count != child_names.len or (n.bounds != null) != bounded) return error.InvalidHudGeometry;
    for (child_names, 0..) |id, offset| {
        const j = index + offset + 1;
        if (j >= input.nodes.len) return error.InvalidHudGeometry;
        const child = input.nodes[j];
        if (!eql(child.id.bytes, id) or child.parent != index or child.role != .text or child.action != null or child.focusable or child.bounds != null or child.child_count != 0) return error.InvalidHudGeometry;
    }
}
/// All counters/topology/schema validated before second-pass output writes.
fn validateTree(input: retained.FrameInput, g: ui.GeometryView) SnapshotError!usize {
    if (input.nodes.len < 2 or input.nodes.len > 72 or input.child_indices.len != input.nodes.len - 1 or input.child_indices.len > 71 or input.focus.base.len > 18 or input.announcements.len > 2 or input.focus.modal != null or input.focus.restore != null) return error.InvalidHudGeometry;
    const root = input.nodes[0];
    if (!eql(root.id.bytes, if (g.settings) "c/settings/root" else "c/hud/root") or root.role != .application or !eql(root.name, "NYON") or root.parent != null or root.action != null or root.focusable or root.bounds != null) return error.InvalidHudGeometry;
    const deck = input.nodes[1];
    if (!eql(deck.id.bytes, if (g.settings) "c/settings/dialog" else "c/hud/toolbar") or deck.role != (if (g.settings) retained.NodeRole.dialog else .toolbar) or deck.parent != 0 or deck.bounds != null or deck.action != null or deck.focusable) return error.InvalidHudGeometry;
    var edges: usize = 0;
    var actions: usize = 0;
    var action_mask: u32 = 0;
    var last_action: ?usize = null;
    var zones: usize = 0;
    var tray = false;
    var guidance = false;
    var status = false;
    var settings_groups: u8 = 0;
    for (input.nodes, 0..) |n, i| {
        if (std.mem.startsWith(u8, n.id.bytes, "c/hud/marker/") or (if (n.action) |a| std.mem.startsWith(u8, a.bytes, "a/hud/marker/") else false)) return error.SourceMismatch;
        if (!scalarText(n) or !n.visible) return error.InvalidHudGeometry;
        for (input.nodes[0..i]) |old| if (eql(old.id.bytes, n.id.bytes)) return error.InvalidHudGeometry;
        if (i != 0) {
            const parent = n.parent orelse return error.InvalidHudGeometry;
            if (parent >= i) return error.InvalidHudGeometry;
            var ancestor = i - 1;
            while (ancestor != parent) ancestor = input.nodes[ancestor].parent orelse return error.InvalidHudGeometry;
        }
        if (n.child_start != edges) return error.InvalidHudGeometry;
        var child_count: usize = 0;
        for (input.nodes, 0..) |child, j| if (child.parent == i) {
            if (edges >= input.child_indices.len or input.child_indices[edges] != j) return error.InvalidHudGeometry;
            edges += 1;
            child_count += 1;
        };
        if (n.child_count != child_count) return error.InvalidHudGeometry;
        if (n.bounds) |bounds| try rect(bounds);
        if (n.action) |action| {
            const ordinal = actionOrdinal(g.settings, action.bytes) orelse return error.InvalidHudGeometry;
            if (last_action) |last| if (ordinal <= last) return error.InvalidHudGeometry;
            last_action = ordinal;
            action_mask |= @as(u32, 1) << @as(u5, @intCast(ordinal));
            if (!n.focusable or n.bounds == null or n.child_count != 2 or i + 2 >= input.nodes.len or !n.enabled and g.settings) return error.InvalidHudGeometry;
            const prefix = if (g.settings) "a/settings/" else "a/hud/";
            const key = action.bytes[prefix.len..];
            const node_prefix = if (g.settings) "c/settings/" else "c/hud/";
            if (n.id.bytes.len < node_prefix.len + 8 or !std.mem.startsWith(u8, n.id.bytes, node_prefix) or !std.mem.endsWith(u8, n.id.bytes, "/primary") or !eql(n.id.bytes[node_prefix.len .. n.id.bytes.len - 8], key)) return error.InvalidHudGeometry;
            const expected_role: retained.NodeRole = if (!g.settings) (if (ordinal >= 10 and ordinal <= 12) .option else .button) else (if (ordinal < 4 or ordinal >= 6 and ordinal < 9) .option else if (ordinal < 6) .checkbox else .button);
            if (n.role != expected_role) return error.InvalidHudGeometry;
            if (!g.settings and n.parent != 1) return error.InvalidHudGeometry;
            if (g.settings) {
                const parent_id = if (ordinal < 4) "c/settings/scales" else if (ordinal < 6) "c/settings/accessibility" else if (ordinal < 9) "c/settings/quality" else "c/settings/dialog";
                if (!eql(input.nodes[n.parent.?].id.bytes, parent_id)) return error.InvalidHudGeometry;
            }
            for (input.nodes[i + 1 .. i + 3], 0..) |child, offset| {
                const suffix = if (offset == 0) "/label" else "/value";
                if (child.id.bytes.len < node_prefix.len + suffix.len or child.parent != i or child.role != .text or child.action != null or child.focusable or child.child_count != 0 or child.bounds == null or !std.meta.eql(child.bounds.?, n.bounds.?) or !std.mem.startsWith(u8, child.id.bytes, node_prefix) or !std.mem.endsWith(u8, child.id.bytes, suffix) or !eql(child.id.bytes[node_prefix.len .. child.id.bytes.len - suffix.len], key)) return error.InvalidHudGeometry;
            }
            if (actions >= input.focus.base.len or !eql(input.focus.base[actions].bytes, action.bytes)) return error.InvalidHudGeometry;
            actions += 1;
            zones += 1;
        } else if (i > 1) {
            const parent = input.nodes[n.parent.?];
            if (parent.action != null) continue;
            if (eql(n.id.bytes, "c/hud/tray")) {
                if (g.settings or tray) return error.InvalidHudGeometry;
                try group(input, i, .region, 0, &.{ "c/hud/tray/source", "c/hud/tray/target", "c/hud/tray/strength", "c/hud/tray/status" }, true);
                tray = true;
                zones += 1;
            } else if (eql(n.id.bytes, "c/guidance")) {
                try group(input, i, .region, 0, &.{ "c/guidance/progress", "c/guidance/title" }, true);
                guidance = true;
                zones += 1;
            } else if (eql(n.id.bytes, "c/tooltip")) {
                if (n.role != .text or n.parent != 0 or n.bounds == null or n.focusable or n.child_count != 0) return error.InvalidHudGeometry;
                zones += 1;
            } else if (eql(parent.id.bytes, "c/hud/tray") or eql(parent.id.bytes, "c/guidance")) {
                // Parent validation checked the exact canonical children.
            } else if (eql(n.id.bytes, "c/selected-field")) {
                if (n.role != .status or n.parent != 0 or n.bounds != null or n.focusable or n.child_count != 0 or n.value == null or !(eql(n.value.?, "ATM") or eql(n.value.?, "HYD") or eql(n.value.?, "TOP"))) return error.InvalidHudGeometry;
                status = true;
            } else if (eql(n.id.bytes, "c/recoverable")) {
                if (n.role != .alert or n.parent != 0 or n.bounds != null or n.focusable or n.child_count != 0 or n.value == null or !eql(n.value.?, n.name)) return error.InvalidHudGeometry;
            } else if (g.settings and (eql(n.id.bytes, "c/settings/scales") or eql(n.id.bytes, "c/settings/accessibility") or eql(n.id.bytes, "c/settings/quality"))) {
                if (n.role != .group or n.parent != 1 or n.bounds != null or n.focusable) return error.InvalidHudGeometry;
                settings_groups += 1;
            } else return error.InvalidHudGeometry;
        }
    }
    const required: u32 = if (g.settings) 2047 else 65535 & ~@as(u32, 1 << 9);
    if (action_mask & required != required or actions != input.focus.base.len or edges != input.child_indices.len or !status or (!g.settings and !tray) or (g.settings and settings_groups != 3)) return error.InvalidHudGeometry;
    if (!g.settings and (((action_mask & (1 << 16)) != 0) != g.help_visible or ((action_mask & (1 << 9)) != 0) != guidance)) return error.InvalidHudGeometry;
    if (input.focus.focused) |focused| {
        var found = false;
        for (input.focus.base) |action| if (eql(action.bytes, focused.bytes)) {
            found = true;
        };
        if (!found) return error.InvalidHudGeometry;
    }
    if (!g.settings and g.help_visible) zones += 1;
    if (zones > 26) return error.ZoneLimit;
    return zones;
}
fn finite(v: f32) SnapshotError!f32 {
    return if (std.math.isFinite(v)) v else error.InvalidHudGeometry;
}
pub fn snapshotFromPrepared(prepared: *const ui.PreparedProjection, current: ui.CurrentStamp, out: *BorrowedHudSnapshot) SnapshotError!void {
    const g = try prepared.geometry();
    return snapshotFromBorrowed(prepared.input(), g, current, out);
}
fn snapshotFromBorrowed(input: retained.FrameInput, g: ui.GeometryView, current: ui.CurrentStamp, out: *BorrowedHudSnapshot) SnapshotError!void {
    @setFloatMode(.strict);
    if (current.owner.draft != null or current.owner.context.len > 128 or !std.unicode.utf8ValidateSlice(current.owner.context) or g.revision != current.revision or !input.owner.matches(current.owner)) return error.SourceMismatch;
    if (!g.viewport.isFinite() or g.viewport.x < 1 or g.viewport.y < 1 or !std.math.isFinite(g.scale) or g.scale <= 0) return error.InvalidHudGeometry;
    const count = try validateTree(input, g);
    const scale = @max(g.scale, 0.72);
    const edges = marker.OpaqueEdges{ .left = try finite(240 * scale), .right = try finite(280 * scale), .top = try finite(64 * scale), .bottom = if (g.viewport.x < 1100) 102 else try finite(56 * scale) };
    var safe = geometry.Rect{ .min = .{ edges.left, edges.top }, .max = .{ try finite(g.viewport.x - edges.right), try finite(g.viewport.y - edges.bottom) } };
    for (&safe.min, &safe.max) |*lo, *hi| if (lo.* > hi.*) {
        const midpoint = try finite(lo.* + (try finite(hi.* - lo.*)) / 2);
        lo.* = midpoint;
        hi.* = midpoint;
    };
    try rect(safe);
    const help = if (!g.settings and g.help_visible) geometry.Rect.fromXYWH(g.viewport.x / 2 - 190, g.viewport.y / 2 - 90, 380, 180) catch return error.InvalidHudGeometry else null;
    // Infallible second pass. Uninitialized unused context/zone tails untouched.
    out.revision = g.revision;
    out.resident = input.owner.resident;
    out.hold = input.owner.hold;
    out.context_len = input.owner.context.len;
    @memcpy(out.context[0..out.context_len], input.owner.context);
    out.viewport = g.viewport;
    out.safe = safe;
    out.edges = edges;
    out.settings = g.settings;
    out.zone_count = count;
    var index: usize = 0;
    for (input.nodes) |n| {
        const kind: ?marker.ZoneKind = if (n.action != null) .control else if (eql(n.id.bytes, "c/hud/tray")) .tray else if (eql(n.id.bytes, "c/guidance")) .guidance else if (eql(n.id.bytes, "c/tooltip")) .tooltip else null;
        if (kind) |k| {
            out.zones[index] = .{ .kind = k, .bounds = n.bounds.?, .enabled = n.enabled };
            index += 1;
        }
    }
    if (help) |bounds| out.zones[index] = .{ .kind = .help, .bounds = bounds, .enabled = true };
}
/// Caller associates step/stamp/Core with one immutable epoch. No runtime
/// provenance getter or inverse-matrix consistency proof is implied.
pub fn projectFromCore(lease: *const CurrentLease, hud: *const BorrowedHudSnapshot) AdapterError!marker.Result {
    if (hud.context_len > 128 or hud.zone_count > 26 or lease.stamp.owner.context.len > 128 or lease.stamp.owner.draft != null or hud.revision != lease.stamp.revision or !hud.resident.sameIdentity(lease.stamp.owner.resident) or hud.hold != lease.stamp.owner.hold or !std.unicode.utf8ValidateSlice(hud.context[0..hud.context_len]) or !eql(hud.context[0..hud.context_len], lease.stamp.owner.context)) return error.SourceMismatch;
    const raw = lease.core.viewport();
    if (!raw.isFinite()) return error.InvalidViewport;
    const normalized = math.Vec2{ .x = @max(raw.x, 1), .y = @max(raw.y, 1) };
    const camera_state = lease.core.camera();
    if (!std.meta.eql(camera_state.logical_viewport, normalized) or !std.meta.eql(hud.viewport, normalized)) return error.SourceMismatch;
    const campaign = lease.core.campaign();
    const scene_frame = lease.core.sceneFrame();
    return marker.project(.{ .step = lease.step, .playing = lease.core.mode() == .playing, .viewport = normalized, .camera = scene_frame.camera, .scene_worlds = &scene_frame.worlds, .worlds = &campaign.worlds, .safe = hud.safe, .edges = hud.edges, .settings = hud.settings, .zones = hud.zones[0..hud.zone_count] });
}
test "H3c adapter target payload measurements" {
    std.debug.print("H3c Snapshot={d} Lease={d} Geometry={d} FrameInput={d} CameraState={d} CampaignView={d} SceneFrame={d} Marker={d} Result={d} Header={d} OptionalMarker={d} AttachStage={d}; constructor locals and transitive call stack UNKNOWN\n", .{ @sizeOf(BorrowedHudSnapshot), @sizeOf(CurrentLease), @sizeOf(ui.GeometryView), @sizeOf(retained.FrameInput), @sizeOf(@import("classic_camera.zig").CameraState), @sizeOf(app.CampaignView), @sizeOf(@import("classic_scene.zig").SceneFrame), @sizeOf(marker.Marker), @sizeOf(marker.Result), ui.HEADER_BYTES, @sizeOf(?marker.Marker), ui.ATTACH_STAGE_BYTES });
}

test "H3c private copied schema malformed ancillary and primary rejection" {
    var tokens = retained.LiveTokenBudget.init(.{ .max_live_objects = 1, .max_live_requested_bytes = 1280 });
    defer tokens.deinit();
    const owner = try retained.TokenRef.create(std.testing.allocator, &tokens);
    defer owner.release();
    var budget = retained.PreparationBudget.init(1179648);
    defer budget.deinit();
    const current = ui.CurrentStamp{ .revision = 1, .owner = .{ .resident = owner, .hold = null, .context = "classic", .draft = null } };
    var prepared = try ui.prepareHudSettings(std.testing.allocator, &budget, .{ .mode = .settings, .onboarding_step = .select_union_world, .help_visible = true, .pointer = .{ .x = 410, .y = 322 } }, current);
    defer prepared.deinit();
    const g = try prepared.geometry();
    const original = prepared.input();
    var nodes: [72]retained.NodeSpec = undefined;
    @memcpy(nodes[0..original.nodes.len], original.nodes);
    var input = original;
    input.nodes = nodes[0..original.nodes.len];
    try std.testing.expectEqual(@as(usize, 13), try validateTree(input, g));
    for (input.nodes, 0..) |n, i| {
        if (eql(n.id.bytes, "c/guidance") or eql(n.id.bytes, "c/tooltip")) {
            nodes[i].bounds = null;
            try expectSnapshotRejected(input, g, current, error.InvalidHudGeometry);
            nodes[i] = n;
            nodes[i].bounds.?.min[0] = std.math.nan(f32);
            try expectSnapshotRejected(input, g, current, error.InvalidHudGeometry);
            nodes[i] = n;
            nodes[i].role = .button;
            try expectSnapshotRejected(input, g, current, error.InvalidHudGeometry);
            nodes[i] = n;
            nodes[i].id = .{ .bytes = "unknown" };
            try expectSnapshotRejected(input, g, current, error.InvalidHudGeometry);
            nodes[i] = n;
            nodes[i].id = nodes[0].id;
            try expectSnapshotRejected(input, g, current, error.InvalidHudGeometry);
            nodes[i] = n;
        }
        if (n.action != null) {
            nodes[i].id = .{ .bytes = "c/settings/primary" };
            try expectSnapshotRejected(input, g, current, error.InvalidHudGeometry);
            nodes[i] = n;
            nodes[i].action = .{ .bytes = "a/settings/unknown" };
            try expectSnapshotRejected(input, g, current, error.InvalidHudGeometry);
            nodes[i] = n;
        }
    }
    nodes[2].parent = 72;
    try expectSnapshotRejected(input, g, current, error.InvalidHudGeometry);
    nodes[2] = original.nodes[2];
    nodes[2].id = .{ .bytes = "c/hud/marker/0/primary" };
    try expectSnapshotRejected(input, g, current, error.SourceMismatch);
}

fn expectSnapshotRejected(input: retained.FrameInput, g: ui.GeometryView, current: ui.CurrentStamp, expected: SnapshotError) !void {
    var out: BorrowedHudSnapshot = undefined;
    @memset(std.mem.asBytes(&out), 0x5a);
    var original: [@sizeOf(BorrowedHudSnapshot)]u8 = undefined;
    @memcpy(&original, std.mem.asBytes(&out));
    try std.testing.expectError(expected, snapshotFromBorrowed(input, g, current, &out));
    try std.testing.expectEqualSlices(u8, &original, std.mem.asBytes(&out));
}
