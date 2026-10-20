const std = @import("std");
const api = @import("root.zig");
const t = @import("types.zig");
const view = @import("view.zig");
const command = @import("command.zig");
const local_state = @import("../planet_economy/state.zig");
const creator: t.SessionPolicy = .{ .role = .creator, .civilization = null };
const config: t.Config = .{ .seed = 2, .planets = .{ .{ .side = 4 }, .{ .side = 4 }, .{}, .{} } };

fn create(cfg: t.Config, units: u16) !api.Galaxy {
    var builder = try api.beginCreate(std.testing.allocator, cfg);
    var n: usize = 0;
    while (n < 500_000) : (n += 1) {
        var budget: t.WorkBudget = .{ .remaining = units };
        switch (api.pollCreate(&builder, &budget)) {
            .ready => return api.takeReady(&builder),
            .pending => {},
            else => return error.UnexpectedCreate,
        }
    }
    return error.Timeout;
}
fn finish(g: *api.Galaxy, id: t.JobId, units: u16) !t.Summary {
    var result: ?t.Summary = null;
    for (0..500_000) |_| {
        var budget: t.WorkBudget = .{ .remaining = units };
        switch (api.poll(g, id, &budget)) {
            .published => |s| {
                try std.testing.expect(result == null);
                result = s;
            },
            .pending => {},
            .idle => return result orelse error.MissingPublication,
            else => return error.UnexpectedProgress,
        }
    }
    return error.Timeout;
}
fn drain(g: *api.Galaxy, id: t.JobId) !void {
    for (0..500_000) |_| {
        var budget: t.WorkBudget = .{ .remaining = 4096 };
        switch (api.poll(g, id, &budget)) {
            .idle => return,
            .pending, .canceled, .failed => {},
            else => return error.UnexpectedDrain,
        }
    }
    return error.Timeout;
}
fn envelope(out: []u8, planet: u32, sequence: u64) !t.RoutedEnvelope {
    return command.encodeEnvelope(@fromBackingInt(planet), .{ .sequence = @fromBackingInt(sequence), .provenance = .creator, .actor = null }, .{ .author_settlement = .{ .owner = @fromBackingInt(1), .anchor = .{ .planet = @fromBackingInt(planet), .local = 70 }, .population = 2, .food = 12, .material = 8, .economic_bypass = true } }, out);
}

// Two distinct, nonempty worlds: two settlements, one house and one in-flight
// shipment per world. All encoded bodies borrow only the caller's owned buffers.
fn twoWorldInputs(buffers: *[8][256]u8) ![8]t.RoutedEnvelope {
    var result: [8]t.RoutedEnvelope = undefined;
    for (0..2) |i| {
        const id: t.PlanetId = @fromBackingInt(@as(u32, @intCast(i)) + 1);
        const owner: t.CivilizationId = @fromBackingInt(@as(u32, @intCast(i)) + 1);
        const path = [_]t.CellId{ .{ .planet = id, .local = 70 }, .{ .planet = id, .local = 71 }, .{ .planet = id, .local = 4 }, .{ .planet = id, .local = 5 } };
        const payloads = [_]t.Command{
            .{ .author_settlement = .{ .owner = owner, .anchor = .{ .planet = id, .local = 70 }, .population = @intCast(2 + i), .food = @intCast(20 + i), .material = @intCast(8 + i), .economic_bypass = true } },
            .{ .author_settlement = .{ .owner = owner, .anchor = .{ .planet = id, .local = 5 }, .population = 1, .food = @intCast(6 + i), .material = 2, .economic_bypass = true } },
            .{ .author_build = .{ .settlement = @fromBackingInt(1), .kind = .house, .anchor = .{ .planet = id, .local = 69 }, .facing = null, .economic_bypass = true } },
            .{ .author_ship = .{ .source = @fromBackingInt(1), .destination = @fromBackingInt(2), .resource = .food, .amount = @intCast(1 + i), .path = &path, .cargo_source = .transfer, .economic_bypass = true } },
        };
        for (payloads, 0..) |payload, j| {
            const slot = i * 4 + j;
            result[slot] = try command.encodeEnvelope(id, .{ .sequence = @fromBackingInt(@as(u64, @intCast(slot)) + 1), .provenance = .creator, .actor = null }, payload, &buffers[slot]);
        }
    }
    return result;
}

test "cg/jobs/all-world-phases" {
    var g = try create(config, 4096);
    defer api.deinit(&g) catch unreachable;
    try std.testing.expectError(error.WrongRevision, api.beginTick(&g, @fromBackingInt(1), @fromBackingInt(1), creator, &.{}));
    try std.testing.expectError(error.WrongTick, api.beginTick(&g, @fromBackingInt(0), @fromBackingInt(2), creator, &.{}));
    const id = try api.beginTick(&g, @fromBackingInt(0), @fromBackingInt(1), creator, &.{});
    var seen: [7][2]bool = @splat(@splat(false));
    var published = false;
    for (0..500_000) |_| {
        var budget: t.WorkBudget = .{ .remaining = 1 };
        const result = api.poll(&g, id, &budget);
        if (result == .pending) if (result.pending.phase) |phase| {
            if (@backingInt(phase) <= @backingInt(t.Phase.local_finalize)) {
                const storage = try view.storage(&g);
                for (storage.metadata().planets[0..2]) |p| if (p.?.control.lane) |local| {
                    const rank = @backingInt(local.phase);
                    const global = @backingInt(phase);
                    const next: usize = if (phase == .local_finalize) @backingInt(t.Phase.install) else global + 1;
                    try std.testing.expect(rank == global or rank == next);
                };
            }
        };
        switch (result) {
            .pending => |stats| if (stats.phase) |phase| {
                if (@backingInt(phase) < 7) {
                    if (stats.planet) |p| seen[@backingInt(phase)][@backingInt(p) - 1] = true;
                }
            },
            .published => |s| {
                published = true;
                try std.testing.expectEqual(1, @backingInt(s.tick));
            },
            .idle => break,
            else => return error.UnexpectedProgress,
        }
    }
    try std.testing.expect(published);
    for (seen) |row| for (row) |yes| try std.testing.expect(yes);
    const s = try view.storage(&g);
    for (s.metadata().planets[0..2]) |p| {
        const meta = try p.?.metadata(p.?.activeRoot());
        try std.testing.expectEqual(1, @backingInt(meta.tick));
        try std.testing.expectEqual(1, @backingInt(meta.revision));
    }
}

test "cg/jobs/partial-install-abort-and-retry" {
    var g = try create(config, 4096);
    defer api.deinit(&g) catch unreachable;
    const old = try api.acquireView(&g);
    const original = try snapshot(&g, old);
    var bytes: [8][256]u8 = undefined;
    const inputs = try twoWorldInputs(&bytes);
    const id = try api.beginTick(&g, @fromBackingInt(0), @fromBackingInt(1), creator, &inputs);
    var reached = false;
    for (0..500_000) |_| {
        var budget: t.WorkBudget = .{ .remaining = 1 };
        const progress = api.poll(&g, id, &budget);
        if (progress == .pending and progress.pending.phase == .install and progress.pending.cursor == 1) {
            reached = true;
            break;
        }
        try std.testing.expect(progress == .pending);
    }
    try std.testing.expect(reached);
    // Old and newly acquired views still resolve both complete old worlds even
    // while world 1's internal marker has already installed its candidate.
    const during = try api.acquireView(&g);
    try std.testing.expectEqualDeep(original, try snapshot(&g, old));
    try std.testing.expectEqualDeep(original, try snapshot(&g, during));
    try api.releaseView(&g, during);
    try std.testing.expectEqual(.requested, api.cancel(&g, id));
    try drain(&g, id);
    try std.testing.expectEqualDeep(original, try snapshot(&g, old));
    const fresh = try api.acquireView(&g);
    try std.testing.expectEqualDeep(original, try snapshot(&g, fresh));
    try api.releaseView(&g, fresh);
    try api.releaseView(&g, old);
    const retry = try api.beginTick(&g, @fromBackingInt(0), @fromBackingInt(1), creator, &inputs);
    const published = try finish(&g, retry, 7);
    try std.testing.expectEqual(8, @backingInt(published.last_sequence));
    try std.testing.expectEqual(8, published.accepted);
    const current = try api.acquireView(&g);
    const values = try snapshot(&g, current);
    try expectPopulated(values);
    try api.releaseView(&g, current);
    try std.testing.expectEqual(.already_published, api.cancel(&g, retry));
}

test "cg/jobs/view-lease-and-zero" {
    var g = try create(config, 4096);
    defer api.deinit(&g) catch unreachable;
    const old = try api.acquireView(&g);
    const original = try api.summary(&g, old);
    const id = try api.beginTick(&g, @fromBackingInt(0), @fromBackingInt(1), creator, &.{});
    var zero: t.WorkBudget = .{ .remaining = 0 };
    const first = api.poll(&g, id, &zero);
    try std.testing.expectEqualDeep(first, api.poll(&g, id, &zero));
    for (0..500_000) |_| {
        var budget: t.WorkBudget = .{ .remaining = 4096 };
        const result = api.poll(&g, id, &budget);
        if (result == .published) break;
        try std.testing.expect(result == .pending);
    }
    var budget: t.WorkBudget = .{ .remaining = 7 };
    try std.testing.expectEqual(.waiting_for_views, api.poll(&g, id, &budget));
    try std.testing.expectEqual(7, budget.remaining);
    try std.testing.expectEqualDeep(original, try api.summary(&g, old));
    try std.testing.expectError(error.Busy, api.beginTick(&g, @fromBackingInt(1), @fromBackingInt(2), creator, &.{}));
    const fresh = try api.acquireView(&g);
    try api.releaseView(&g, old);
    try drain(&g, id);
    const next = try api.beginTick(&g, @fromBackingInt(1), @fromBackingInt(2), creator, &.{});
    try std.testing.expectEqual(.stale_job, api.cancel(&g, id));
    try std.testing.expectEqual(error.StaleJob, api.poll(&g, id, &budget).invalid_request);
    try std.testing.expectEqual(.requested, api.cancel(&g, next));
    try drain(&g, next);
    try api.releaseView(&g, fresh);
}

fn drainBuilder(builder: *api.Builder) !void {
    for (0..500_000) |_| {
        var budget: t.WorkBudget = .{ .remaining = 256 };
        const result = api.pollCreate(builder, &budget);
        if (result == .pending and result.pending.phase == null) {
            try api.deinitBuilder(builder);
            return;
        }
    }
    return error.Timeout;
}

test "cg/jobs/creation-paid-validation-move-cancel-and-allocation-ownership" {
    for (0..5) |index| {
        var allocator = std.testing.FailingAllocator.init(std.testing.allocator, .{ .fail_index = index });
        try std.testing.expectError(error.OutOfMemory, api.beginCreate(allocator.allocator(), .{ .seed = 2, .planet_count = 4 }));
        try std.testing.expectEqual(allocator.allocated_bytes, allocator.freed_bytes);
        try std.testing.expectEqual(index, allocator.deallocations);
    }
    var invalid = config;
    invalid.planets[0].side = 65535;
    var builder = try api.beginCreate(std.testing.allocator, invalid);
    try std.testing.expectError(error.Busy, api.deinitBuilder(&builder));
    var zero: t.WorkBudget = .{ .remaining = 0 };
    const untouched = api.pollCreate(&builder, &zero);
    try std.testing.expectEqualDeep(untouched, api.pollCreate(&builder, &zero));
    var failed = false;
    for (0..100) |_| {
        var budget: t.WorkBudget = .{ .remaining = 1 };
        const result = api.pollCreate(&builder, &budget);
        if (result == .failed) {
            try std.testing.expectEqual(error.InvalidConfig, result.failed);
            failed = true;
            break;
        }
    }
    try std.testing.expect(failed);
    try drainBuilder(&builder);

    for ([_]u16{ 0, 1, 256, 4096 }) |units| {
        var allocator = std.testing.FailingAllocator.init(std.testing.allocator, .{ .fail_index = 3 });
        var original = try api.beginCreate(allocator.allocator(), config);
        var budget: t.WorkBudget = .{ .remaining = units };
        _ = api.pollCreate(&original, &budget);
        // Move the owning handle; retained cursors must not point into original.
        var moved = original;
        original = .{ .handle = null };
        api.cancelCreate(&moved);
        const frozen = api.pollCreate(&moved, &zero);
        try std.testing.expectEqualDeep(frozen, api.pollCreate(&moved, &zero));
        try drainBuilder(&moved);
        try std.testing.expectEqual(allocator.allocated_bytes, allocator.freed_bytes);
        try std.testing.expectEqual(3, allocator.alloc_index);
        try std.testing.expect(!allocator.has_induced_failure);
    }
    // Cancel after world 1 has finished and world 2 is partially initialized.
    builder = try api.beginCreate(std.testing.allocator, config);
    for (0..500_000) |_| {
        var budget: t.WorkBudget = .{ .remaining = 256 };
        _ = api.pollCreate(&builder, &budget);
        if (@import("job.zig").builderStorage(&builder).?.setup_index == 1) break;
    }
    api.cancelCreate(&builder);
    try drainBuilder(&builder);
}

const Snapshot = struct {
    value: t.Summary,
    global_count: u16,
    local_count: [2]u16,
    global_hash: [32]u8,
    local_hash: [2][32]u8,
    policy: ?t.SessionPolicy,
    metadata: [2]local_state.RootMeta = undefined,
    cells: [2][96]t.CellValue = undefined,
    settlements: [2][2]?t.SettlementValue = @splat(@splat(null)),
    buildings: [2]?local_state.Building = @splat(null),
    freight: [2]?local_state.Shipment = @splat(null),
    routes: [2][65]?t.CellId = @splat(@splat(null)),
};
// These fixtures have side 4, at most two settlements and one building/shipment
// per world. Compare every visible cell and every instantiated logical record;
// normalize unused footprint storage, never padding or spare pool bytes.
fn expectPopulated(value: Snapshot) !void {
    for (0..2) |i| {
        try std.testing.expectEqual(2, value.metadata[i].settlements);
        try std.testing.expectEqual(1, value.metadata[i].buildings);
        try std.testing.expectEqual(1, value.metadata[i].shipments);
        try std.testing.expectEqual(i + 1, @backingInt(value.settlements[i][0].?.owner));
        try std.testing.expectEqual(2 + i, value.settlements[i][0].?.population);
        try std.testing.expectEqual(i + 1, value.freight[i].?.quantity);
        try std.testing.expectEqual(.house, value.buildings[i].?.kind);
        try std.testing.expectEqual(70, value.routes[i][0].?.local);
        try std.testing.expectEqual(5, value.routes[i][3].?.local);
    }
}
fn snapshot(g: *api.Galaxy, token: t.ViewToken) !Snapshot {
    const s = try view.storage(g);
    const d = &s.metadata().descriptors[token.descriptor_slot];
    var result: Snapshot = .{ .value = try api.summary(g, token), .global_count = d.history_count, .local_count = undefined, .global_hash = undefined, .local_hash = undefined, .policy = d.policy };
    var global: [64]t.ReceiptValue = undefined;
    const rows = try api.receipts(g, token, 0, &global);
    var hash = std.crypto.hash.sha2.Sha256.init(.{});
    for (global[0..rows.written]) |row| hash.update(row.bytes[0..row.len]);
    hash.final(&result.global_hash);
    for (0..2) |i| {
        var local: [64]t.LocalReceiptValue = undefined;
        const page = try api.planetReceipts(g, token, @fromBackingInt(@as(u32, @intCast(i)) + 1), 0, &local);
        var h = std.crypto.hash.sha2.Sha256.init(.{});
        for (local[0..page.written]) |row| h.update(row.bytes[0..row.len]);
        h.final(&result.local_hash[i]);
        result.local_count[i] = d.planets[i].history_count;
        const p = &s.metadata().planets[i].?;
        const root = d.planets[i].root;
        const meta = try p.metadata(root);
        result.metadata[i] = meta;
        try std.testing.expectEqual(4, s.config.planets[i].side);
        try std.testing.expect(meta.settlements <= 2 and meta.buildings <= 1 and meta.shipments <= 1);
        for (&result.cells[i], 0..) |*cell, index| cell.* = try api.cell(g, token, .{ .planet = d.planets[i].planet, .local = index });
        for (0..meta.settlements) |index| result.settlements[i][index] = try api.settlement(g, token, .{ .planet = d.planets[i].planet, .local = @fromBackingInt(@as(u64, @intCast(index)) + 1) });
        var budget: t.WorkBudget = .{ .remaining = 4096 };
        if (meta.buildings != 0) {
            var building = (try p.readRecord(root, .building, 0, &budget)).?.building;
            for (building.footprint.cells[building.footprint.len..]) |*cell| cell.* = .{ .planet = @fromBackingInt(0), .local = 0 };
            result.buildings[i] = building;
        }
        if (meta.shipments != 0) {
            const freight = (try p.readRecord(root, .shipment, 0, &budget)).?.shipment;
            result.freight[i] = freight;
            for (0..freight.route_len) |index| result.routes[i][index] = (try p.readRouteCell(root, 0, @intCast(index), &budget)).?;
        }
    }
    return result;
}

test "cg/jobs/local-and-global-HistoryFull-preserve-whole-snapshot" {
    for ([_]bool{ false, true }) |local_failure| {
        var cfg = config;
        if (local_failure) cfg.planets[1].capacities.history_receipts = 1 else cfg.global_history_capacity = 1;
        var g = try create(cfg, 4096);
        defer api.deinit(&g) catch unreachable;
        const old = try api.acquireView(&g);
        const before = try snapshot(&g, old);
        var bytes: [128]u8 = undefined;
        const input = try envelope(&bytes, 1, 7);
        const id = try api.beginTick(&g, @fromBackingInt(0), @fromBackingInt(1), creator, &.{input});
        var saw_failure = false;
        for (0..500_000) |_| {
            var budget: t.WorkBudget = .{ .remaining = 7 };
            const result = api.poll(&g, id, &budget);
            if (result == .failed) {
                try std.testing.expect(!saw_failure);
                saw_failure = true;
                try std.testing.expectEqual(error.HistoryFull, result.failed);
            } else if (result == .idle) break else try std.testing.expect(result == .pending);
        }
        try std.testing.expect(saw_failure);
        try std.testing.expectEqualDeep(before, try snapshot(&g, old));
        const fresh = try api.acquireView(&g);
        try std.testing.expectEqualDeep(before, try snapshot(&g, fresh));
        try std.testing.expectEqual(0, @backingInt((try view.storage(&g)).metadata().published_sequence));
        for (1..3) |p| try std.testing.expectError(error.NotFound, api.settlement(&g, fresh, .{ .planet = @fromBackingInt(@as(u32, @intCast(p))), .local = @fromBackingInt(1) }));
        try api.releaseView(&g, old);
        try api.releaseView(&g, fresh);
        const retry = try api.beginTick(&g, @fromBackingInt(0), @fromBackingInt(1), creator, &.{input});
        try std.testing.expectEqual(.requested, api.cancel(&g, retry));
        try drain(&g, retry);
    }
}

test "cg/jobs/budget-schedules-caller-lifetime-and-no-poll-allocation" {
    var reference: ?Snapshot = null;
    var reference_work: ?u64 = null;
    for ([_]u16{ 1, 7, 256, 4096 }) |units| {
        var allocator = std.testing.FailingAllocator.init(std.testing.allocator, .{ .fail_index = 3 });
        var builder = try api.beginCreate(allocator.allocator(), config);
        while (true) {
            var budget: t.WorkBudget = .{ .remaining = units };
            const result = api.pollCreate(&builder, &budget);
            if (result == .ready) break;
            try std.testing.expect(result == .pending);
        }
        var g = try api.takeReady(&builder);
        var bytes: [8][256]u8 = undefined;
        const inputs = try twoWorldInputs(&bytes);
        const id = try api.beginTick(&g, @fromBackingInt(0), @fromBackingInt(1), creator, &inputs);
        for (&bytes) |*body| @memset(body, 0);
        var publications: u8 = 0;
        for (0..500_000) |_| {
            var zero: t.WorkBudget = .{ .remaining = 0 };
            const untouched = api.poll(&g, id, &zero);
            try std.testing.expectEqualDeep(untouched, api.poll(&g, id, &zero));
            var budget: t.WorkBudget = .{ .remaining = units };
            const result = api.poll(&g, id, &budget);
            if (result == .pending) try std.testing.expectEqual(units - budget.remaining, result.pending.units_used);
            if (result == .published) publications += 1 else if (result == .idle) break else try std.testing.expect(result == .pending);
        }
        try std.testing.expectEqual(1, publications);
        const token = try api.acquireView(&g);
        const current = try snapshot(&g, token);
        try expectPopulated(current);
        if (reference) |expected| try std.testing.expectEqualDeep(expected, current) else reference = current;
        const work = @import("job.zig").control(try view.storage(&g)).work;
        if (reference_work) |expected| try std.testing.expectEqual(expected, work) else reference_work = work;
        for (1..3) |p| {
            const value = try api.settlement(&g, token, .{ .planet = @fromBackingInt(@as(u32, @intCast(p))), .local = @fromBackingInt(1) });
            try std.testing.expectEqual(1, @backingInt(value.revision));
        }
        try api.releaseView(&g, token);
        try api.deinit(&g);
        try std.testing.expectEqual(allocator.allocated_bytes, allocator.freed_bytes);
        try std.testing.expectEqual(3, allocator.alloc_index);
        try std.testing.expect(!allocator.has_induced_failure);
    }
}

test "cg/jobs/cancel-at-descriptor-boundary-and-counter-exhaustion" {
    var g = try create(config, 4096);
    defer api.deinit(&g) catch unreachable;
    const s = try view.storage(&g);
    const c = @import("job.zig").control(s);
    c.next_job = std.math.maxInt(u64);
    try std.testing.expectError(error.IdExhausted, api.beginTick(&g, @fromBackingInt(0), @fromBackingInt(1), creator, &.{}));
    try std.testing.expect(!s.metadata().lane_live);
    c.next_job = 1;
    const slot = s.metadata().committed_descriptor.?;
    s.metadata().descriptor_generations[1 - slot] = std.math.maxInt(u64);
    try std.testing.expectError(error.IdExhausted, api.beginTick(&g, @fromBackingInt(0), @fromBackingInt(1), creator, &.{}));
    s.metadata().descriptor_generations[1 - slot] = 1;
    s.metadata().descriptors[slot].revision = @fromBackingInt(std.math.maxInt(u64));
    try std.testing.expectError(error.IdExhausted, api.beginTick(&g, @fromBackingInt(std.math.maxInt(u64)), @fromBackingInt(1), creator, &.{}));
    s.metadata().descriptors[slot].revision = @fromBackingInt(0);
    try std.testing.expectError(error.TickHorizon, api.beginTick(&g, @fromBackingInt(0), @fromBackingInt(1_000_001), creator, &.{}));
    const old = try api.acquireView(&g);
    const original = try snapshot(&g, old);
    const id = try api.beginTick(&g, @fromBackingInt(0), @fromBackingInt(1), creator, &.{});
    var reached = false;
    for (0..500_000) |_| {
        var budget: t.WorkBudget = .{ .remaining = 1 };
        const result = api.poll(&g, id, &budget);
        try std.testing.expect(result == .pending);
        if (result.pending.phase == .install and result.pending.installed_planets == 2) {
            reached = true;
            break;
        }
    }
    try std.testing.expect(reached);
    try std.testing.expectEqual(.requested, api.cancel(&g, id));
    var zero: t.WorkBudget = .{ .remaining = 0 };
    const unchanged = api.poll(&g, id, &zero);
    try std.testing.expectEqualDeep(unchanged, api.poll(&g, id, &zero));
    try drain(&g, id);
    try std.testing.expectEqualDeep(original, try snapshot(&g, old));
    try api.releaseView(&g, old);
}

test "cg/jobs/maximum-sequence-once-empty-ticks-and-gameplay-rejection" {
    var g = try create(config, 4096);
    defer api.deinit(&g) catch unreachable;
    var bytes: [128]u8 = undefined;
    const input = try envelope(&bytes, 1, std.math.maxInt(u64));
    const id = try api.beginTick(&g, @fromBackingInt(0), @fromBackingInt(1), creator, &.{input});
    _ = try finish(&g, id, 256);
    const bad = try api.beginTick(&g, @fromBackingInt(1), @fromBackingInt(2), creator, &.{input});
    var seen = false;
    for (0..1000) |_| {
        var budget: t.WorkBudget = .{ .remaining = 7 };
        const result = api.poll(&g, bad, &budget);
        if (result == .failed) {
            try std.testing.expectEqual(error.SequenceAlreadyPublished, result.failed);
            seen = true;
        }
        if (result == .idle) break;
    }
    try std.testing.expect(seen);
    const empty = try api.beginTick(&g, @fromBackingInt(1), @fromBackingInt(2), creator, &.{});
    const summary2 = try finish(&g, empty, 256);
    try std.testing.expectEqual(std.math.maxInt(u64), @backingInt(summary2.last_sequence));

    var rejected = try create(config, 4096);
    defer api.deinit(&rejected) catch unreachable;
    // Valid structural input with impossible owner is a local gameplay rejection.
    const env = try command.encodeEnvelope(@fromBackingInt(1), .{ .sequence = @fromBackingInt(1), .provenance = .creator, .actor = null }, .{ .author_settlement = .{ .owner = @fromBackingInt(4), .anchor = .{ .planet = @fromBackingInt(1), .local = 70 }, .population = 2, .food = 12, .material = 8, .economic_bypass = true } }, &bytes);
    const job = try api.beginTick(&rejected, @fromBackingInt(0), @fromBackingInt(1), creator, &.{env});
    const done = try finish(&rejected, job, 7);
    try std.testing.expectEqual(1, done.rejected);
    const token = try api.acquireView(&rejected);
    try std.testing.expectEqual(1, @backingInt((try api.cell(&rejected, token, .{ .planet = @fromBackingInt(2), .local = 0 })).revision));
    try api.releaseView(&rejected, token);
}

test "cg/jobs/cleanup-fault-permanent-before-and-after-publication" {
    for ([_]bool{ false, true }) |published| {
        var g = try create(config, 4096);
        const s = try view.storage(&g);
        // Task7 owns the public fault shutdown API; use the allocation ledger
        // for this private injected-fault fixture's teardown only.
        defer {
            g.handle = null;
            s.deinit();
        }
        const id = try api.beginTick(&g, @fromBackingInt(0), @fromBackingInt(1), creator, &.{});
        var reached = false;
        for (0..500_000) |_| {
            var budget: t.WorkBudget = .{ .remaining = 1 };
            const result = api.poll(&g, id, &budget);
            if (published and result == .published) {
                reached = true;
                break;
            }
            if (!published and result == .pending and result.pending.installed_planets == 1) {
                reached = true;
                break;
            }
            try std.testing.expect(result == .pending);
        }
        try std.testing.expect(reached);
        const token = try api.acquireView(&g);
        const before = try api.summary(&g, token);
        const p = &s.metadata().planets[0].?;
        const discarded = if (published) s.metadata().descriptors[s.metadata().obsolete_descriptor.?].planets[0].root else try p.candidateRoot();
        // Corrupt only a directory that no committed view uses.
        p.storage().reservation.directories[discarded.slot][0] = std.math.maxInt(u64);
        if (!published) try std.testing.expectEqual(.requested, api.cancel(&g, id));
        var faulted = false;
        for (0..10000) |_| {
            var budget: t.WorkBudget = .{ .remaining = 256 };
            const result = api.poll(&g, id, &budget);
            if (result == .faulted) {
                try std.testing.expectEqual(error.CleanupFault, result.faulted.cause);
                faulted = true;
                break;
            }
            try std.testing.expect(result == .pending or result == .canceled);
        }
        try std.testing.expect(faulted);
        try std.testing.expectError(error.Busy, api.deinit(&g));
        try std.testing.expectError(error.CleanupFault, api.beginTick(&g, before.revision, @fromBackingInt(@backingInt(before.tick) + 1), creator, &.{}));
        for ([_]u16{ 0, 1, 4096 }) |units| {
            var budget: t.WorkBudget = .{ .remaining = units };
            const result = api.poll(&g, id, &budget);
            try std.testing.expect(result == .faulted);
            try std.testing.expectEqual(units, budget.remaining);
        }
        try std.testing.expectEqual(.faulted, api.cancel(&g, id));
        try std.testing.expectEqual(.stale_job, api.cancel(&g, @fromBackingInt(999)));
        const fresh = try api.acquireView(&g);
        try std.testing.expectEqualDeep(before, try api.summary(&g, fresh));
        try api.releaseView(&g, fresh);
        try api.releaseView(&g, token);
    }
}

test "cg/jobs/creation-invalid-context-capacity-unused-slots-and-ready-cancel" {
    for (0..5) |case| {
        var cfg = config;
        switch (case) {
            0 => cfg.civilization_count = 0,
            1 => cfg.planets[1].capacities.settlements = 0,
            2 => cfg.planets[0].system = @fromBackingInt(4),
            3 => cfg.planet_count = 0,
            4 => cfg.global_history_capacity = 0,
            else => unreachable,
        }
        var builder = try api.beginCreate(std.testing.allocator, cfg);
        var saw = false;
        for (0..100) |_| {
            var budget: t.WorkBudget = .{ .remaining = 7 };
            const result = api.pollCreate(&builder, &budget);
            if (result == .failed) {
                saw = true;
                try std.testing.expectEqual(error.InvalidConfig, result.failed);
                break;
            }
        }
        try std.testing.expect(saw);
        try drainBuilder(&builder);
    }
    var cfg = config;
    cfg.planets[2] = undefined;
    cfg.planets[3] = undefined;
    var builder = try api.beginCreate(std.testing.allocator, cfg);
    while (true) {
        var budget: t.WorkBudget = .{ .remaining = 4096 };
        if (api.pollCreate(&builder, &budget) == .ready) break;
    }
    var zero: t.WorkBudget = .{ .remaining = 0 };
    try std.testing.expect(api.pollCreate(&builder, &zero) == .pending);
    api.cancelCreate(&builder);
    try std.testing.expectError(error.NotReady, api.takeReady(&builder));
    try drainBuilder(&builder);
}

test "cg/jobs/four-world-registry-and-maximum-reservation" {
    var cfg: t.Config = .{ .seed = 99, .planet_count = 4, .system_count = 4, .civilization_count = 4 };
    for (&cfg.planets, 0..) |*p, i| {
        p.side = 32;
        p.system = @fromBackingInt(@as(u32, @intCast(i)) + 1);
    }
    var g = try create(cfg, 4096);
    defer api.deinit(&g) catch unreachable;
    const s = try view.storage(&g);
    try std.testing.expectEqual(47_153_152, s.requestedBytes());
    const id = try api.beginTick(&g, @fromBackingInt(0), @fromBackingInt(1), creator, &.{});
    _ = try finish(&g, id, 4096);
    const token = try api.acquireView(&g);
    for (1..5) |p| {
        const value = try api.cell(&g, token, .{ .planet = @fromBackingInt(@as(u32, @intCast(p))), .local = 6143 });
        try std.testing.expectEqual(1, @backingInt(value.revision));
    }
    try api.releaseView(&g, token);
}

test "cg/jobs/global-capacity-before-local-preparation" {
    for ([_]u16{ 1, 3, 4 }) |capacity| {
        var cfg = config;
        cfg.planet_count = 1;
        cfg.global_history_capacity = capacity;
        var g = try create(cfg, 4096);
        defer api.deinit(&g) catch unreachable;
        const s = try view.storage(&g);
        const old = try api.acquireView(&g);
        const before = try api.summary(&g, old);
        try std.testing.expectEqual(1, s.metadata().descriptors[old.descriptor_slot].history_count);
        const id = try api.beginTick(&g, @fromBackingInt(0), @fromBackingInt(1), creator, &.{});
        var local_started = false;
        var failures: u8 = 0;
        var publications: u8 = 0;
        for (0..500_000) |_| {
            var budget: t.WorkBudget = .{ .remaining = 1 };
            const result = api.poll(&g, id, &budget);
            local_started = local_started or s.metadata().planets[0].?.control.lane != null;
            switch (result) {
                .failed => |err| {
                    try std.testing.expectEqual(error.HistoryFull, err);
                    failures += 1;
                },
                .published => {
                    publications += 1;
                    try api.releaseView(&g, old);
                },
                .idle => break,
                .pending => {},
                else => return error.UnexpectedProgress,
            }
        }
        if (capacity < 4) {
            try std.testing.expectEqual(1, failures);
            try std.testing.expectEqual(0, publications);
            try std.testing.expectEqualDeep(before, try api.summary(&g, old));
            try api.releaseView(&g, old);
            try std.testing.expect(!local_started);
        } else {
            try std.testing.expect(local_started);
            try std.testing.expectEqual(0, failures);
            try std.testing.expectEqual(1, publications);
            try std.testing.expectEqual(4, s.metadata().descriptors[s.metadata().committed_descriptor.?].history_count);
        }
    }
}
