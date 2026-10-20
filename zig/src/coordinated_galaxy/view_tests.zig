const std = @import("std");
const t = @import("types.zig");
const state = @import("state.zig");
const view = @import("view.zig");
const api = @import("root.zig");
const helpers = @import("planet_tests.zig");
const creator: t.SessionPolicy = .{ .role = .creator, .civilization = null };

fn create() !api.Galaxy {
    return createWithAllocator(std.testing.allocator);
}
fn createWithAllocator(allocator: t.Allocator) !api.Galaxy {
    var s = try state.Storage.init(allocator, .{ .seed = 2, .planets = .{ .{ .side = 4 }, .{ .side = 4 }, .{}, .{} } });
    errdefer s.deinit();
    while (true) {
        var budget: t.WorkBudget = .{ .remaining = 4096 };
        if (try s.advanceSetup(&budget) == .done) break;
    }
    try view.reserveDescriptor(&s, .{});
    for (0..2) |i| {
        const p = &s.metadata().planets[i].?;
        s.metadata().prepared_roots[i] = try p.prepared(p.activeRoot());
    }
    // Global history encoding belongs Task5; these sentinel bytes test bounds/copies.
    s.metadata().history_lengths[0] = 3;
    @memcpy(s.reservation.?.history[0][0..3], "old");
    s.metadata().descriptor_header.history_count = 1;
    s.metadata().descriptor_header.history_head = @splat(7);
    try prepare(&s, 1);
    view.publishDescriptor(&s);
    return view.takeGalaxy(&s);
}
fn prepare(s: *state.Storage, units: u16) !void {
    while (true) {
        const cursor = s.metadata().descriptor_cursor;
        const work = s.metadata().descriptor_work;
        var zero: t.WorkBudget = .{ .remaining = 0 };
        try std.testing.expectEqual(.pending, try view.prepareDescriptor(s, &zero));
        try std.testing.expectEqual(cursor, s.metadata().descriptor_cursor);
        try std.testing.expectEqual(work, s.metadata().descriptor_work);
        var budget: t.WorkBudget = .{ .remaining = units };
        if (try view.prepareDescriptor(s, &budget) == .done) break;
    }
}
fn candidate(s: *state.Storage, units: u16) !void {
    try view.reserveDescriptor(s, .{ .tick = @fromBackingInt(1), .revision = @fromBackingInt(1), .last_sequence = @fromBackingInt(9), .policy = creator, .history_count = 2, .history_head = @splat(8), .accepted = 4 });
    for (0..2) |i| {
        const p = &s.metadata().planets[i].?;
        try helpers.demo(p, @as(u32, @intCast(i)) + 1, @as(u32, @intCast(i)) + 1);
        for ([_]t.Phase{ .prepare, .commands, .arrivals, .construction, .production, .population, .local_finalize }) |phase| try helpers.advance(p, phase, units);
        s.metadata().prepared_roots[i] = try helpers.freeze(p, units);
    }
    s.metadata().history_lengths[1] = 3;
    @memcpy(s.reservation.?.history[1][0..3], "new");
    try prepare(s, units);
}
fn install(s: *state.Storage, index: usize) !void {
    var one: t.WorkBudget = .{ .remaining = 1 };
    try std.testing.expectEqual(.done, try s.metadata().planets[index].?.installHidden(&one));
}
fn cleanOld(s: *state.Storage) !void {
    const slot = s.metadata().obsolete_descriptor.?;
    try std.testing.expect(view.canRetire(s, slot));
    for (s.metadata().descriptors[slot].planets[0..2], 0..) |p, i| try helpers.retire(&s.metadata().planets[i].?, p.root, 256);
    try view.finishRetirement(s, slot);
}

test "cg/views/old-new-and-stale" {
    var g = try create();
    defer api.deinit(&g) catch unreachable;
    const s = try view.storage(&g);
    const old = try api.acquireView(&g);
    const old_summary = try api.summary(&g, old);
    const old_cell = try api.cell(&g, old, .{ .planet = @fromBackingInt(1), .local = 70 });
    const initial_work = s.metadata().descriptor_work;
    try candidate(s, 1);
    try std.testing.expectEqual(initial_work + 4, s.metadata().descriptor_work);
    try install(s, 0);
    for (1..3) |id| try std.testing.expectError(error.NotFound, api.settlement(&g, old, .{ .planet = @fromBackingInt(@as(u32, @intCast(id))), .local = @fromBackingInt(1) }));
    try install(s, 1);
    try std.testing.expectEqualDeep(old_cell, try api.cell(&g, old, old_cell.id));
    view.publishDescriptor(s);
    const new = try api.acquireView(&g);
    for (1..3) |id| {
        const value = try api.settlement(&g, new, .{ .planet = @fromBackingInt(@as(u32, @intCast(id))), .local = @fromBackingInt(1) });
        try std.testing.expectEqual(@as(u32, @intCast(id)), @backingInt(value.owner));
        try std.testing.expectEqual(@as(u64, 1), @backingInt(value.revision));
    }
    try std.testing.expectEqualDeep(old_summary, try api.summary(&g, old));
    try std.testing.expectEqual(@as(u16, 4), (try api.summary(&g, new)).accepted);
    try std.testing.expectEqual(@as(u64, 0), @backingInt(s.metadata().published_sequence));
    var local: [64]t.LocalReceiptValue = @splat(.{ .len = 0, .bytes = @splat(0) });
    const old_page = try api.planetReceipts(&g, old, @fromBackingInt(1), 0, &local);
    try std.testing.expectEqual(@as(u16, 1), old_page.written);
    try std.testing.expectEqual(null, old_page.next);
    const saved_local = local[0];
    const new_page = try api.planetReceipts(&g, new, @fromBackingInt(1), 0, &local);
    try std.testing.expect(new_page.written > 1);
    var global: [2]t.ReceiptValue = @splat(.{ .len = 0, .bytes = @splat(0) });
    try std.testing.expectEqual(@as(u16, 1), (try api.receipts(&g, old, 0, &global)).written);
    const saved = global[0];
    try std.testing.expectEqual(@as(u16, 2), (try api.receipts(&g, new, 0, &global)).written);
    try std.testing.expectEqualSlices(u8, "new", global[1].bytes[0..global[1].len]);
    try std.testing.expect(!view.canRetire(s, old.descriptor_slot));
    try std.testing.expectError(error.Busy, view.reserveDescriptor(s, .{}));
    try std.testing.expectError(error.Busy, api.deinit(&g));
    var forged = old;
    forged.lease_generation += 1;
    const before = global;
    try std.testing.expectError(error.StaleView, api.receipts(&g, forged, 0, &global));
    try std.testing.expectEqualDeep(before, global);
    const before_local = local;
    try std.testing.expectError(error.StaleView, api.planetReceipts(&g, forged, @fromBackingInt(1), 0, &local));
    try std.testing.expectEqualDeep(before_local, local);
    try std.testing.expectError(error.StaleView, api.releaseView(&g, forged));
    try api.releaseView(&g, old);
    try std.testing.expectError(error.StaleView, api.releaseView(&g, old));
    try cleanOld(s);
    try std.testing.expectError(error.StaleView, api.summary(&g, old));
    try std.testing.expectEqualSlices(u8, "old", saved.bytes[0..saved.len]);
    try std.testing.expect(saved_local.len > 0);
    try std.testing.expectEqual(@as(u64, 0), @backingInt(old_cell.revision));
    try std.testing.expectEqual(@as(u64, 1), @backingInt((try api.summary(&g, new)).revision));
    try api.releaseView(&g, new);
}

test "cg/views/exhaustion-and-backpressure" {
    var g = try create();
    defer api.deinit(&g) catch unreachable;
    const s = try view.storage(&g);
    var tokens: [64]t.ViewToken = undefined;
    for (&tokens) |*token| token.* = try api.acquireView(&g);
    try std.testing.expectError(error.ViewCapacity, api.acquireView(&g));
    try std.testing.expectError(error.Busy, api.deinit(&g));
    for (tokens) |token| try api.releaseView(&g, token);
    const slot = s.metadata().free_leases[s.metadata().free_lease_count - 1];
    s.metadata().leases[slot].generation = std.math.maxInt(u64) - 1;
    const last = try api.acquireView(&g);
    try std.testing.expectEqual(std.math.maxInt(u64), last.lease_generation);
    try api.releaseView(&g, last);
    for (tokens[0..63]) |*token| {
        token.* = try api.acquireView(&g);
        try std.testing.expect(token.lease_slot != last.lease_slot);
    }
    try std.testing.expectError(error.ViewCapacity, api.acquireView(&g));
    for (tokens[0..63]) |token| try api.releaseView(&g, token);
    s.metadata().descriptor_generations[1] = std.math.maxInt(u64);
    try std.testing.expectError(error.IdExhausted, view.reserveDescriptor(s, .{}));
    try std.testing.expectEqual(null, s.metadata().preparing_descriptor);
    try std.testing.expect(!s.metadata().lane_live);
    const token = try api.acquireView(&g);
    var forged = token;
    forged.descriptor_generation = @fromBackingInt(2);
    try std.testing.expectError(error.StaleView, api.summary(&g, forged));
    forged = token;
    forged.lease_slot = 255;
    try std.testing.expectError(error.StaleView, api.releaseView(&g, forged));
    forged = token;
    forged.descriptor_slot = 255;
    try std.testing.expectError(error.StaleView, api.summary(&g, forged));
    var out: [65]t.ReceiptValue = @splat(.{ .len = 17, .bytes = @splat(99) });
    try std.testing.expectError(error.CapacityUnsupported, api.receipts(&g, token, 0, &out));
    try std.testing.expectError(error.NotFound, api.receipts(&g, token, 2, out[0..1]));
    try std.testing.expectEqual(@as(u8, 99), out[0].bytes[0]);
    try std.testing.expectEqual(@as(u16, 17), out[64].len);
    s.metadata().acquisition_closed = true;
    try std.testing.expectError(error.Shutdown, api.acquireView(&g));
    _ = try api.summary(&g, token);
    try api.releaseView(&g, token);
}

test "cg/views/hidden-restore-keeps-descriptor-and-buffers" {
    var g = try create();
    defer api.deinit(&g) catch unreachable;
    const s = try view.storage(&g);
    const token = try api.acquireView(&g);
    const summary = try api.summary(&g, token);
    try candidate(s, 7);
    try install(s, 0);
    for (0..2) |i| {
        var one: t.WorkBudget = .{ .remaining = 1 };
        const p = &s.metadata().planets[i].?;
        try std.testing.expectEqual(.done, try p.restoreHidden(&one));
        try helpers.retire(p, try p.candidateRoot(), 7);
    }
    try view.discardDescriptor(s);
    try std.testing.expectEqualDeep(summary, try api.summary(&g, token));
    var out: [2]t.LocalReceiptValue = @splat(.{ .len = 17, .bytes = @splat(99) });
    try std.testing.expectEqual(@as(u16, 1), (try api.planetReceipts(&g, token, @fromBackingInt(1), 0, &out)).written);
    try std.testing.expectEqual(@as(u16, 17), out[1].len);
    try std.testing.expectError(error.NotFound, api.planetReceipts(&g, token, @fromBackingInt(1), 2, &out));
    try std.testing.expectError(error.NotFound, api.settlement(&g, token, .{ .planet = @fromBackingInt(1), .local = @fromBackingInt(1) }));
    try api.releaseView(&g, token);
    try std.testing.expectEqual(@as(u64, 2), s.metadata().descriptor_generations[1]);
    try view.reserveDescriptor(s, .{});
    try std.testing.expectEqual(@as(u64, 3), s.metadata().descriptor_generations[1]);
    try view.discardDescriptor(s);
}

test "cg/views/owner-identity-moves-and-copied-value-lifetime" {
    var first = try create();
    var second = try create();
    defer api.deinit(&second) catch unreachable;
    const a = try api.acquireView(&first);
    const b = try api.acquireView(&second);
    try std.testing.expectEqual(a.descriptor_slot, b.descriptor_slot);
    try std.testing.expectEqual(a.descriptor_generation, b.descriptor_generation);
    try std.testing.expectEqual(a.lease_slot, b.lease_slot);
    try std.testing.expectEqual(a.lease_generation, b.lease_generation);
    try std.testing.expect(a.owner_generation != b.owner_generation);
    try std.testing.expectError(error.StaleView, api.summary(&second, a));
    try std.testing.expectError(error.StaleView, api.releaseView(&second, a));
    var rows: [2]t.ReceiptValue = @splat(.{ .len = 17, .bytes = @splat(99) });
    const unchanged = rows;
    try std.testing.expectError(error.StaleView, api.receipts(&second, a, 0, &rows));
    try std.testing.expectEqualDeep(unchanged, rows);
    const stable = (try view.storage(&first)).metadata();
    var moved = first;
    first.handle = null;
    try std.testing.expect(stable == (try view.storage(&moved)).metadata());
    try std.testing.expectEqual(@as(u16, 1), (try api.receipts(&moved, a, 0, &rows)).written);
    const copied_summary = try api.summary(&moved, a);
    const copied_cell = try api.cell(&moved, a, .{ .planet = @fromBackingInt(1), .local = 70 });
    try api.releaseView(&moved, a);
    try api.deinit(&moved);
    // A recreated owner has fresh identity even when allocator addresses recycle.
    var recreated = try create();
    defer api.deinit(&recreated) catch unreachable;
    const c = try api.acquireView(&recreated);
    try std.testing.expect(c.owner_generation != a.owner_generation);
    try std.testing.expectError(error.StaleView, api.summary(&recreated, a));
    try std.testing.expectError(error.StaleView, api.releaseView(&recreated, a));
    try std.testing.expectEqualSlices(u8, "old", rows[0].bytes[0..rows[0].len]);
    try std.testing.expectEqual(@as(u16, 17), rows[1].len);
    try std.testing.expectEqual(@as(u64, 0), @backingInt(copied_summary.revision));
    try std.testing.expectEqual(@as(u64, 0), @backingInt(copied_cell.revision));
    try api.releaseView(&recreated, c);
    try api.releaseView(&second, b);
}

test "cg/views/descriptor-budget-and-invalid-query-buffers" {
    for ([_]u16{ 1, 7, 256, 4096 }) |units| {
        var g = try create();
        defer api.deinit(&g) catch unreachable;
        const s = try view.storage(&g);
        const token = try api.acquireView(&g);
        try view.reserveDescriptor(s, .{ .history_count = 1 });
        const m = s.metadata();
        for (m.descriptors[token.descriptor_slot].planets[0..2], 0..) |pd, i| m.prepared_roots[i] = try m.planets[i].?.prepared(pd.root);
        const start = m.descriptor_work;
        var invalid: t.WorkBudget = .{ .remaining = 4097 };
        try std.testing.expectError(error.InvalidBudget, view.prepareDescriptor(s, &invalid));
        try std.testing.expectEqual(start, m.descriptor_work);
        try prepare(s, units);
        try std.testing.expectEqual(start + 4, m.descriptor_work);
        try view.discardDescriptor(s);
        var globals: [2]t.ReceiptValue = @splat(.{ .len = 17, .bytes = @splat(99) });
        const global_before = globals;
        m.history_lengths[0] = 2049;
        try std.testing.expectError(error.InvariantViolation, api.receipts(&g, token, 0, &globals));
        try std.testing.expectEqualDeep(global_before, globals);
        m.history_lengths[0] = 3;
        var locals: [65]t.LocalReceiptValue = @splat(.{ .len = 17, .bytes = @splat(99) });
        const local_before = locals;
        try std.testing.expectError(error.CapacityUnsupported, api.planetReceipts(&g, token, @fromBackingInt(1), 0, &locals));
        try std.testing.expectError(error.InvalidPlanet, api.planetReceipts(&g, token, @fromBackingInt(0), 0, locals[0..2]));
        try std.testing.expectError(error.NotFound, api.planetReceipts(&g, token, @fromBackingInt(1), 2, locals[0..2]));
        const p = &m.planets[0].?;
        const length = p.storage().reservation.history_meta[0];
        p.storage().reservation.history_meta[0] = 1025;
        try std.testing.expectError(error.InvariantViolation, api.planetReceipts(&g, token, @fromBackingInt(1), 0, locals[0..2]));
        try std.testing.expectEqualDeep(local_before, locals);
        p.storage().reservation.history_meta[0] = length;
        try std.testing.expectEqualDeep(t.ReceiptPage{ .written = 0, .next = 0 }, try api.receipts(&g, token, 0, globals[0..0]));
        try std.testing.expectEqualDeep(t.ReceiptPage{ .written = 0, .next = null }, try api.receipts(&g, token, 1, &globals));
        try std.testing.expectEqualDeep(global_before, globals);
        try api.releaseView(&g, token);
        m.lane_live = true;
        try std.testing.expectError(error.Busy, api.deinit(&g));
        m.lane_live = false;
        m.faulted = true;
        try std.testing.expectError(error.Busy, api.deinit(&g));
        m.faulted = false;
    }
}

test "cg/views/reused-allocation-does-not-revive-token" {
    const bytes = try std.testing.allocator.alloc(u8, state.requested_bytes + 2 * state.planet_requested_bytes + 128);
    defer std.testing.allocator.free(bytes);
    var fixed = std.heap.FixedBufferAllocator.init(bytes);
    var first = try createWithAllocator(fixed.allocator());
    const old_handle = first.handle;
    const old = try api.acquireView(&first);
    try api.releaseView(&first, old);
    try api.deinit(&first);
    fixed.reset();
    var second = try createWithAllocator(fixed.allocator());
    defer api.deinit(&second) catch unreachable;
    try std.testing.expectEqual(old_handle, second.handle);
    const new = try api.acquireView(&second);
    try std.testing.expectEqual(old.lease_slot, new.lease_slot);
    try std.testing.expectEqual(old.lease_generation, new.lease_generation);
    try std.testing.expect(old.owner_generation != new.owner_generation);
    try std.testing.expectError(error.StaleView, api.summary(&second, old));
    try std.testing.expectError(error.StaleView, api.releaseView(&second, old));
    _ = try api.summary(&second, new);
    try api.releaseView(&second, new);
}
