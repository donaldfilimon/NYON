const std = @import("std");
const t = std.testing;
const a = @import("classic_start_marker_adapter.zig");
const ui = @import("classic_ui.zig");
const r = @import("retained_semantics.zig");
const m = @import("model.zig");
const math = @import("classic_math.zig");
const p = @import("classic_start_marker.zig");
const prefs = @import("preferences.zig");
const store = @import("classic_store.zig");
const sessions = @import("classic_session.zig");
fn stamp(owner: r.TokenRef, revision: u64) ui.CurrentStamp {
    return .{ .revision = revision, .owner = .{ .resident = owner, .hold = 12, .context = "classic/α", .draft = null } };
}
fn canonical(world: u8) p.Marker {
    return .{ .world = world, .name = m.WorldName.init("ASTER VALE") catch unreachable, .point = .{ .x = 200, .y = 200 }, .label = .{ .min = .{ 173.8, 225.64 }, .max = .{ 173.8 + @as(f32, 180), 225.64 + @as(f32, 54) } }, .mode = .action_only, .connector_start = null, .ring_radius = null };
}
fn countKind(snapshot: *const a.BorrowedHudSnapshot, kind: p.ZoneKind) usize {
    var n: usize = 0;
    for (snapshot.zones[0..snapshot.zone_count]) |z| if (z.kind == kind) {
        n += 1;
    };
    return n;
}
fn verifyTree(view: r.FrameView) !void {
    try t.expectEqual(view.nodes.len - 1, view.child_indices.len);
    var edges: usize = 0;
    for (view.nodes, 0..) |node, i| {
        try t.expectEqual(edges, node.child_start);
        var count: usize = 0;
        for (view.nodes, 0..) |child, j| if (child.parent == i) {
            try t.expectEqual(j, view.child_indices[edges]);
            edges += 1;
            count += 1;
        };
        try t.expectEqual(count, node.child_count);
        if (i != 0) {
            var ancestor = i - 1;
            const parent = node.parent.?;
            while (ancestor != parent) ancestor = view.nodes[ancestor].parent.?;
        }
    }
}
test "H3c actual Session twelve default source snapshot attachment publication" {
    @setFloatMode(.strict);
    const viewports = [_]math.Vec2{ .{ .x = 723, .y = 802 }, .{ .x = 960, .y = 600 }, .{ .x = 1440, .y = 900 } };
    const points = [_]math.Vec2{ .{ .x = 105.464, .y = 402.429 }, .{ .x = 288.452, .y = 301.069 }, .{ .x = 432.678, .y = 451.604 } };
    const xs = [3][4]f32{ .{ 173.8, 173.8, 199.72, 225.64 }, .{ 198.452, 198.452, 199.72, 225.64 }, .{ 342.678, 342.678, 342.678, 342.678 } };
    const ys = [_]f32{ 310.429, 209.069, 359.604 };
    for (viewports, 0..) |viewport, vi| for ([_]prefs.UiScale{ .percent85, .percent100, .percent115, .percent130 }, 0..) |scale, si| {
        var raw = store.MemoryStore.init(t.allocator, .preferences);
        defer raw.deinit();
        const scenario = try @import("scenario.zig").generatedValidated(m.DEFAULT_SEED, .{});
        var session = try sessions.Session.init(t.allocator, &scenario, raw.raw());
        defer session.deinit();
        session.core.setViewport(viewport);
        session.core.setPreferences(.{ .ui_scale = scale });
        var tokens = r.LiveTokenBudget.init(.{ .max_live_objects = 1, .max_live_requested_bytes = 1280 });
        defer tokens.deinit();
        const owner = try r.TokenRef.create(t.allocator, &tokens);
        defer owner.release();
        var budget = r.PreparationBudget.init(1179648);
        defer budget.deinit();
        var prepared = try ui.prepareHudSettingsFromSession(t.allocator, &budget, &session, null, stamp(owner, 1));
        defer prepared.deinit();
        const fingerprint = session.core.canonicalFingerprint();
        const camera = session.core.camera();
        const value = blk: {
            var snapshot: a.BorrowedHudSnapshot = undefined;
            try a.snapshotFromPrepared(&prepared, stamp(owner, 1), &snapshot);
            try t.expectEqual(@as(usize, 18), snapshot.zone_count);
            try t.expectEqual(@as(usize, 16), countKind(&snapshot, .control));
            try t.expectEqual(@as(usize, 1), countKind(&snapshot, .tray));
            try t.expectEqual(@as(usize, 1), countKind(&snapshot, .guidance));
            try t.expect(!snapshot.zones[13].enabled and !snapshot.zones[14].enabled);
            const s = std.math.clamp(std.math.clamp(@min(viewport.x / 1440, viewport.y / 900), @as(f32, 0.72), @as(f32, 1.15)) * scale.factor(), @as(f32, 0.60), @as(f32, 1.50));
            const legacy = @max(s, @as(f32, 0.72));
            try t.expectEqual(@as(f32, 240) * legacy, snapshot.safe.min[0]);
            try t.expectEqual(viewport.x - 280 * legacy, snapshot.safe.max[0]);
            try t.expectEqual(@as(f32, 64) * legacy, snapshot.safe.min[1]);
            try t.expectEqual(viewport.y - (if (viewport.x < 1100) @as(f32, 102) else 56 * legacy), snapshot.safe.max[1]);
            // Independently source-derived first-row and field control geometry.
            const columns: usize = if (viewport.x >= 1100) 9 else 5;
            const available = (viewport.x - 16 - @as(f32, @floatFromInt(columns - 1)) * 6) / @as(f32, @floatFromInt(columns));
            const width = @min(if (columns == 9) @max(92 * s, 72) else @max(124 * s, 104), available);
            const row_width = @as(f32, @floatFromInt(columns)) * width + @as(f32, @floatFromInt(columns - 1)) * 6;
            try t.expectEqual(@max((viewport.x - row_width) / 2, 4), snapshot.zones[0].bounds.min[0]);
            try t.expectEqual(@max(viewport.x - 268 * s, 4), snapshot.zones[10].bounds.min[0]);
            try t.expectEqual(236 * s, snapshot.zones[10].bounds.min[1]);
            const lease = a.CurrentLease{ .core = &session.core, .step = session.onboarding.step(), .stamp = stamp(owner, 1) };
            break :blk try a.projectFromCore(&lease, &snapshot);
        }; // Every input/snapshot lease ends here, before mutable attachment.
        const marker = value.marker;
        try t.expectEqual(@as(u8, 0), marker.world);
        try t.expectEqualStrings("ASTER VALE", marker.name.asSlice());
        try t.expectApproxEqAbs(points[vi].x, marker.point.x, 0.02);
        try t.expectApproxEqAbs(points[vi].y, marker.point.y, 0.02);
        try t.expectApproxEqAbs(xs[vi][si], marker.label.min[0], 0.02);
        try t.expectApproxEqAbs(ys[vi], marker.label.min[1], 0.02);
        try t.expectEqual(if (vi == 0) p.Mode.action_only else p.Mode.scene_linked, marker.mode);
        if (vi != 0) {
            try t.expectEqual(@as(?f32, 22), marker.ring_radius);
            try t.expectEqual(@as(?u8, 0), session.core.pickSceneWorld(marker.point, false));
        }
        const before = budget.ledger();
        try ui.attachStartMarker(&prepared, value);
        try t.expectEqualDeep(before, budget.ledger());
        try t.expectEqualDeep(marker, (try prepared.startMarker()).?);
        _ = try prepared.geometry();
        var slot = r.FrameSlot.init(t.allocator, &budget);
        defer slot.deinit();
        var revision: ?u64 = null;
        try ui.publishHudSettings(&slot, &revision, &prepared, stamp(owner, 1));
        try verifyTree(slot.view().?);
        try t.expectEqual(@as(usize, 62), slot.view().?.nodes.len);
        try t.expectEqualDeep(sessions.UiAction{ .select_guidance_world = 0 }, try ui.validateAction(slot.view().?, revision.?, stamp(owner, 1), .{ .bytes = "a/hud/marker/0" }));
        prepared.deinit(); // H1 copies and inline result survive source Header destruction.
        try t.expectEqualStrings("ASTER VALE", marker.name.asSlice());
        try t.expectEqualDeep(camera, session.core.camera());
        try t.expectEqual(fingerprint, session.core.canonicalFingerprint());
        try t.expectEqual(@as(usize, 0), raw.save_calls);
        std.debug.print("H3c DEFAULT viewport={d}x{d} scale={s} zones=18 controls=16 world=0 mode={s} label=({d},{d}) nodes=62\n", .{ viewport.x, viewport.y, @tagName(scale), @tagName(marker.mode), marker.label.min[0], marker.label.min[1] });
    };
}
test "H3c source Settings eleven twelve thirteen ancillary and absence order" {
    for ([_]bool{ false, true }) |guidance| for ([_]bool{ false, true }) |tooltip| {
        var raw = store.MemoryStore.init(t.allocator, .preferences);
        defer raw.deinit();
        const scenario = try @import("scenario.zig").generatedValidated(m.DEFAULT_SEED, .{});
        var session = try sessions.Session.init(t.allocator, &scenario, raw.raw());
        defer session.deinit();
        if (!guidance) _ = session.handleUiAction(.skip_onboarding, raw.raw());
        session.core.setMode(.settings);
        session.core.help_visible = true;
        const initial_saves = raw.save_calls;
        var tokens = r.LiveTokenBudget.init(.{ .max_live_objects = 1, .max_live_requested_bytes = 1280 });
        defer tokens.deinit();
        const owner = try r.TokenRef.create(t.allocator, &tokens);
        defer owner.release();
        var budget = r.PreparationBudget.init(1179648);
        defer budget.deinit();
        // Default 1440x900 four-column scale85 control source x408,y320.
        const pointer: ?math.Vec2 = if (tooltip) .{ .x = 410, .y = 322 } else null;
        var prepared = try ui.prepareHudSettingsFromSession(t.allocator, &budget, &session, pointer, stamp(owner, 1));
        defer prepared.deinit();
        const result = blk: {
            var snapshot: a.BorrowedHudSnapshot = undefined;
            try a.snapshotFromPrepared(&prepared, stamp(owner, 1), &snapshot);
            try t.expectEqual(@as(usize, 11) + @intFromBool(guidance) + @intFromBool(tooltip), snapshot.zone_count);
            try t.expectEqual(@as(usize, 0), countKind(&snapshot, .help));
            try t.expectEqual(@as(usize, 0), countKind(&snapshot, .tray));
            try t.expectEqual(@as(usize, 11), countKind(&snapshot, .control));
            try t.expectEqual(@as(f32, 408), snapshot.zones[0].bounds.min[0]);
            try t.expectEqual(@as(f32, 320), snapshot.zones[0].bounds.min[1]);
            const input = prepared.input();
            for (snapshot.zones[11..snapshot.zone_count]) |zone| {
                const id = if (zone.kind == .guidance) "c/guidance" else "c/tooltip";
                var found = false;
                for (input.nodes) |n| if (std.mem.eql(u8, n.id.bytes, id)) {
                    try t.expectEqualDeep(n.bounds.?, zone.bounds);
                    found = true;
                };
                try t.expect(found);
            }
            const lease = a.CurrentLease{ .core = &session.core, .step = session.onboarding.step(), .stamp = stamp(owner, 1) };
            break :blk try a.projectFromCore(&lease, &snapshot);
        };
        try t.expectEqual(if (guidance) p.Absent.not_playing else p.Absent.not_initial, result.absent);
        try t.expectError(error.WrongProjectionMode, ui.attachStartMarker(&prepared, result));
        var slot = r.FrameSlot.init(t.allocator, &budget);
        defer slot.deinit();
        var revision: ?u64 = null;
        try ui.publishHudSettings(&slot, &revision, &prepared, stamp(owner, 1));
        try t.expect(session.core.help_visible);
        try t.expectEqual(guidance, session.onboarding.active());
        try t.expectEqual(initial_saves, raw.save_calls);
    };
}
test "H3c seven actual published capabilities and inline fractional geometry" {
    var tokens = r.LiveTokenBudget.init(.{ .max_live_objects = 1, .max_live_requested_bytes = 1280 });
    defer tokens.deinit();
    const owner = try r.TokenRef.create(t.allocator, &tokens);
    defer owner.release();
    var budget = r.PreparationBudget.init(1179648);
    defer budget.deinit();
    for (0..7) |world| {
        var prepared = try ui.prepareHudSettings(t.allocator, &budget, .{ .onboarding_step = .select_union_world, .help_visible = true, .recoverable = "RECOVERY", .pointer = .{ .x = 800, .y = 850 } }, stamp(owner, 1));
        defer prepared.deinit();
        var before_nodes: [72]NodeWitness = undefined;
        const old_count = blk: {
            const input = prepared.input();
            for (input.nodes, 0..) |n, i| before_nodes[i] = .{ .semantic = semanticHash(n), .parent = n.parent };
            break :blk input.nodes.len;
        };
        try ui.attachStartMarker(&prepared, .{ .marker = canonical(@intCast(world)) });
        var slot = r.FrameSlot.init(t.allocator, &budget);
        defer slot.deinit();
        var revision: ?u64 = null;
        try ui.publishHudSettings(&slot, &revision, &prepared, stamp(owner, 1));
        const view = slot.view().?;
        try verifyTree(view);
        try t.expectEqual(old_count + 3, view.nodes.len);
        try t.expectEqual(@as(usize, 18), view.focus.base.len);
        const ids = [_][]const u8{ "a/hud/marker/0", "a/hud/marker/1", "a/hud/marker/2", "a/hud/marker/3", "a/hud/marker/4", "a/hud/marker/5", "a/hud/marker/6" };
        try t.expectEqualDeep(sessions.UiAction{ .select_guidance_world = @intCast(world) }, try ui.validateAction(view, revision.?, stamp(owner, 1), .{ .bytes = ids[world] }));
        try t.expectError(error.UnknownAction, ui.validateAction(view, revision.?, stamp(owner, 1), .{ .bytes = "a/hud/marker/7" }));
        try t.expectError(error.UnknownAction, ui.validateAction(view, revision.?, stamp(owner, 1), .{ .bytes = ids[(world + 1) % 7] }));
        try t.expectError(error.StaleProjection, ui.validateAction(view, revision.?, stamp(owner, 2), .{ .bytes = ids[world] }));
        const primary = view.nodes[53];
        try t.expectEqualStrings("START HERE", primary.name);
        try t.expectEqualStrings("Select your Union world to begin the tutorial", primary.description);
        try t.expectEqualStrings("ASTER VALE", primary.value.?);
        try t.expectEqualStrings(ids[world], view.focus.base[17].bytes);
        for (before_nodes[53..old_count], 56..) |old, index| {
            const fresh = view.nodes[index];
            try t.expectEqual(old.semantic, semanticHash(fresh));
            try t.expectEqual(if (old.parent.? >= 53) old.parent.? + 3 else old.parent.?, fresh.parent.?);
        }
        try t.expectError(error.AlreadyPublished, ui.attachStartMarker(&prepared, .{ .absent = .not_initial }));
        try ui.publishHudSettings(&slot, &revision, &prepared, stamp(owner, 1));
        slot.deinit();
        slot = r.FrameSlot.init(t.allocator, &budget);
        try t.expectError(error.AlreadyPublished, ui.attachStartMarker(&prepared, .{ .absent = .not_initial }));
    }
}

test "H3c snapshot unchanged full initialized output mismatches tiny safe and result lifetime" {
    var raw = store.MemoryStore.init(t.allocator, .preferences);
    defer raw.deinit();
    const scenario = try @import("scenario.zig").generatedValidated(m.DEFAULT_SEED, .{});
    var session = try sessions.Session.init(t.allocator, &scenario, raw.raw());
    defer session.deinit();
    var tokens = r.LiveTokenBudget.init(.{ .max_live_objects = 2, .max_live_requested_bytes = 1280 });
    defer tokens.deinit();
    const owner = try r.TokenRef.create(t.allocator, &tokens);
    defer owner.release();
    const other = try r.TokenRef.create(t.allocator, &tokens);
    defer other.release();
    var budget = r.PreparationBudget.init(1179648);
    defer budget.deinit();
    var prepared = try ui.prepareHudSettingsFromSession(t.allocator, &budget, &session, null, stamp(owner, 1));
    defer prepared.deinit();
    var snapshot: a.BorrowedHudSnapshot = undefined;
    @memset(std.mem.asBytes(&snapshot), 0x5a);
    var bytes: [@sizeOf(a.BorrowedHudSnapshot)]u8 = undefined;
    @memcpy(&bytes, std.mem.asBytes(&snapshot));
    var empty = ui.PreparedProjection{};
    try t.expectError(error.EmptyProjection, a.snapshotFromPrepared(&empty, stamp(owner, 1), &snapshot));
    try t.expectEqualSlices(u8, &bytes, std.mem.asBytes(&snapshot));
    var bad = stamp(owner, 2);
    try t.expectError(error.SourceMismatch, a.snapshotFromPrepared(&prepared, bad, &snapshot));
    try t.expectEqualSlices(u8, &bytes, std.mem.asBytes(&snapshot));
    bad = stamp(other, 1);
    try t.expectError(error.SourceMismatch, a.snapshotFromPrepared(&prepared, bad, &snapshot));
    try t.expectEqualSlices(u8, &bytes, std.mem.asBytes(&snapshot));
    bad = stamp(owner, 1);
    bad.owner.hold = 13;
    try t.expectError(error.SourceMismatch, a.snapshotFromPrepared(&prepared, bad, &snapshot));
    try t.expectEqualSlices(u8, &bytes, std.mem.asBytes(&snapshot));
    bad = stamp(owner, 1);
    bad.owner.context = "classic/a";
    try t.expectError(error.SourceMismatch, a.snapshotFromPrepared(&prepared, bad, &snapshot));
    try t.expectEqualSlices(u8, &bytes, std.mem.asBytes(&snapshot));
    bad = stamp(owner, 1);
    bad.owner.draft = other;
    try t.expectError(error.SourceMismatch, a.snapshotFromPrepared(&prepared, bad, &snapshot));
    try t.expectEqualSlices(u8, &bytes, std.mem.asBytes(&snapshot));
    bad = stamp(owner, 1);
    bad.owner.context = &.{0xff};
    try t.expectError(error.SourceMismatch, a.snapshotFromPrepared(&prepared, bad, &snapshot));
    try t.expectEqualSlices(u8, &bytes, std.mem.asBytes(&snapshot));
    var long: [129]u8 = @splat('x');
    bad = stamp(owner, 1);
    bad.owner.context = &long;
    try t.expectError(error.SourceMismatch, a.snapshotFromPrepared(&prepared, bad, &snapshot));
    try t.expectEqualSlices(u8, &bytes, std.mem.asBytes(&snapshot));
    const ledger = budget.ledger();
    const refs = tokenReferences(owner);
    const usage = tokens.usage();
    try a.snapshotFromPrepared(&prepared, stamp(owner, 1), &snapshot);
    try t.expectEqualDeep(ledger, budget.ledger());
    try t.expectEqualDeep(usage, tokens.usage());
    try t.expectEqual(refs, tokenReferences(owner));
    var lease = a.CurrentLease{ .core = &session.core, .step = session.onboarding.step(), .stamp = stamp(owner, 1) };
    const result = try a.projectFromCore(&lease, &snapshot);
    try t.expectEqualDeep(usage, tokens.usage());
    try t.expectEqual(refs, tokenReferences(owner));
    snapshot.context_len = 129;
    try t.expectError(error.SourceMismatch, a.projectFromCore(&lease, &snapshot));
    snapshot.context_len = stamp(owner, 1).owner.context.len;
    snapshot.zone_count = 27;
    try t.expectError(error.SourceMismatch, a.projectFromCore(&lease, &snapshot));
    snapshot.zone_count = 18;
    snapshot.zones[0].bounds.min[0] = std.math.nan(f32);
    lease.step = null;
    try t.expectError(error.InvalidBounds, a.projectFromCore(&lease, &snapshot)); // Absence cannot bypass validation.
    prepared.deinit();
    @memset(std.mem.asBytes(&snapshot), 0xa5); // No snapshot lease is used after this point.
    try t.expectEqualStrings("ASTER VALE", result.marker.name.asSlice());
    session.core.setViewport(.{ .x = 1, .y = 1 });
    var tiny = try ui.prepareHudSettingsFromSession(t.allocator, &budget, &session, null, stamp(owner, 2));
    defer tiny.deinit();
    const absent = blk: {
        var small: a.BorrowedHudSnapshot = undefined;
        try a.snapshotFromPrepared(&tiny, stamp(owner, 2), &small);
        try t.expectEqual(small.safe.min[0], small.safe.max[0]);
        try t.expectEqual(small.safe.min[1], small.safe.max[1]);
        lease = .{ .core = &session.core, .step = session.onboarding.step(), .stamp = stamp(owner, 2) };
        break :blk try a.projectFromCore(&lease, &small);
    };
    try t.expectEqual(p.Absent.no_label_candidate, absent.absent);
    try ui.attachStartMarker(&tiny, absent);
    try t.expectError(error.AlreadyAttached, ui.attachStartMarker(&tiny, absent));
    try t.expectEqualStrings("ASTER VALE", result.marker.name.asSlice());
}

test "H3c actual Playing full help tray guidance tooltip zones and strict touch policy" {
    var raw = store.MemoryStore.init(t.allocator, .preferences);
    defer raw.deinit();
    const scenario = try @import("scenario.zig").generatedValidated(m.DEFAULT_SEED, .{});
    var session = try sessions.Session.init(t.allocator, &scenario, raw.raw());
    defer session.deinit();
    session.core.help_visible = true;
    var tokens = r.LiveTokenBudget.init(.{ .max_live_objects = 1, .max_live_requested_bytes = 1280 });
    defer tokens.deinit();
    const owner = try r.TokenRef.create(t.allocator, &tokens);
    defer owner.release();
    var budget = r.PreparationBudget.init(1179648);
    defer budget.deinit();
    var prepared = try ui.prepareHudSettingsFromSession(t.allocator, &budget, &session, .{ .x = 800, .y = 850 }, stamp(owner, 1));
    defer prepared.deinit();
    var snapshot: a.BorrowedHudSnapshot = undefined;
    try a.snapshotFromPrepared(&prepared, stamp(owner, 1), &snapshot);
    try t.expectEqual(@as(usize, 21), snapshot.zone_count);
    try t.expectEqualDeep(r.Rect{ .min = .{ 496, 70 }, .max = .{ 944, 178 } }, snapshot.zones[17].bounds);
    try t.expectEqual(p.ZoneKind.tray, snapshot.zones[17].kind);
    try t.expectEqualDeep(r.Rect{ .min = .{ 496, 8 }, .max = .{ 944, 60 } }, snapshot.zones[18].bounds);
    try t.expectEqual(p.ZoneKind.guidance, snapshot.zones[18].kind);
    try t.expectEqualDeep(r.Rect{ .min = .{ 8, 814 }, .max = .{ 628, 844 } }, snapshot.zones[19].bounds);
    try t.expectEqual(p.ZoneKind.tooltip, snapshot.zones[19].kind);
    try t.expectEqualDeep(r.Rect{ .min = .{ 530, 360 }, .max = .{ 910, 540 } }, snapshot.zones[20].bounds);
    try t.expectEqual(p.ZoneKind.help, snapshot.zones[20].kind);
    const lease = a.CurrentLease{ .core = &session.core, .step = session.onboarding.step(), .stamp = stamp(owner, 1) };
    // Independent policy seam: start with a source-derived snapshot/camera,
    // replace only caller-owned copied zone values for exact edge cases.
    snapshot.zone_count = 0;
    const original = (try a.projectFromCore(&lease, &snapshot)).marker;
    try t.expectEqual(@as(u8, 0), original.world);
    snapshot.zone_count = 1;
    for ([_]p.ZoneKind{ .control, .tray, .help, .guidance, .tooltip }) |kind| for ([_]bool{ false, true }) |enabled| {
        snapshot.zones[0] = .{ .kind = kind, .enabled = enabled, .bounds = .{ .min = .{ original.label.max[0], original.label.min[1] }, .max = .{ original.label.max[0] + 1, original.label.max[1] } } };
        const touched = try a.projectFromCore(&lease, &snapshot);
        const closed = kind == .tray or kind == .help or kind == .control and enabled;
        if (closed) {
            try t.expect(touched == .absent or touched.marker.world != 0);
        } else try t.expectEqual(@as(u8, 0), touched.marker.world);
        snapshot.zones[0].bounds.min[0] -= 0.001;
        const intersected = try a.projectFromCore(&lease, &snapshot);
        try t.expect(intersected == .absent or intersected.marker.world != 0);
    };
}

const TokenLayout = struct { allocator: std.mem.Allocator, refs: usize, budget: *r.LiveTokenBudget, charged_bytes: usize };
fn tokenReferences(token: r.TokenRef) usize {
    const storage: *const TokenLayout = @ptrCast(@alignCast(token.identity));
    return storage.refs;
}

const NodeWitness = struct { semantic: u64, parent: ?usize };
fn semanticHash(n: r.NodeSpec) u64 {
    var hash = std.hash.Wyhash.init(0);
    hash.update(n.id.bytes);
    hash.update(n.name);
    hash.update(n.description);
    if (n.value) |v| hash.update(v);
    if (n.action) |action| hash.update(action.bytes);
    hash.update(&.{ @backingInt(n.role), @intFromBool(n.enabled), @intFromBool(n.selected), @intFromBool(n.focusable), @intFromBool(n.visible), @intFromBool(n.value != null), @intFromBool(n.action != null) });
    if (n.bounds) |bounds| {
        hash.update(std.mem.asBytes(&bounds.min));
        hash.update(std.mem.asBytes(&bounds.max));
    }
    return hash.final();
}
