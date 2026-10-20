const std = @import("std");
const api = @import("root.zig");
const job = @import("job.zig");
const view = @import("view.zig");
const state = @import("state.zig");
const creator: api.SessionPolicy = .{ .role = .creator, .civilization = null };
const config: api.Config = .{ .seed = 2, .planet_count = 4, .planets = @splat(.{ .side = 4 }) };

fn create(allocator: std.mem.Allocator) !api.Galaxy {
    var b = try api.beginCreate(allocator, config);
    for (0..500_000) |_| {
        var budget: api.WorkBudget = .{ .remaining = 4096 };
        switch (api.pollCreate(&b, &budget)) {
            .pending => {},
            .ready => return api.takeReady(&b),
            else => return error.UnexpectedCreate,
        }
    }
    return error.Timeout;
}
fn reach(g: *api.Galaxy, id: api.JobId, published: bool) !void {
    for (0..500_000) |_| {
        var b: api.WorkBudget = .{ .remaining = 1 };
        const result = api.poll(g, id, &b);
        if (published and result == .published) return;
        if (!published and result == .pending and result.pending.installed_planets == 1) return;
        try std.testing.expect(result == .pending);
    }
    return error.Timeout;
}
fn fault(g: *api.Galaxy, id: api.JobId) !void {
    for (0..500_000) |_| {
        var b: api.WorkBudget = .{ .remaining = 256 };
        const result = api.poll(g, id, &b);
        if (result == .faulted) {
            try std.testing.expectEqual(id, result.faulted.job);
            try std.testing.expectEqual(error.CleanupFault, result.faulted.cause);
            return;
        }
        try std.testing.expect(result == .pending or result == .canceled);
    }
    return error.Timeout;
}
// Observe all mutable traversal/work cursors without reading undefined controls.
fn expectPermanent(g: *api.Galaxy, id: api.JobId) !void {
    const s = try view.storage(g);
    const work = job.control(s).work;
    const lane = job.control(s).tick;
    var local_work: [4]u64 = undefined;
    var discard: [4]u16 = undefined;
    for (s.metadata().planets[0..4], 0..) |p, i| {
        local_work[i] = p.?.storage().work_units;
        const l = &p.?.control.lane.?;
        discard[i] = if (l.retirement) |r| r.discard_index else l.candidate.discard_index;
    }
    for ([_]u16{ 0, 1, 7, 256, 4096 }) |units| {
        var b: api.WorkBudget = .{ .remaining = units };
        for (0..3) |_| {
            const result = api.poll(g, id, &b);
            try std.testing.expect(result == .faulted);
            try std.testing.expectEqual(id, result.faulted.job);
            try std.testing.expectEqual(error.CleanupFault, result.faulted.cause);
            try std.testing.expectEqual(units, b.remaining);
            try std.testing.expectEqual(.faulted, api.cancel(g, id));
            const stale = api.poll(g, @fromBackingInt(@backingInt(id) + 1), &b);
            try std.testing.expectEqual(error.StaleJob, stale.invalid_request);
            try std.testing.expectEqual(.stale_job, api.cancel(g, @fromBackingInt(@backingInt(id) + 1)));
        }
    }
    try std.testing.expectEqual(work, job.control(s).work);
    try std.testing.expectEqual(lane.step, job.control(s).tick.step);
    try std.testing.expectEqual(lane.index, job.control(s).tick.index);
    for (s.metadata().planets[0..4], 0..) |p, i| {
        try std.testing.expectEqual(local_work[i], p.?.storage().work_units);
        const l = &p.?.control.lane.?;
        try std.testing.expectEqual(discard[i], if (l.retirement) |r| r.discard_index else l.candidate.discard_index);
    }
}

test "cg/fault/undrainable-cleanup" {
    for ([_]bool{ false, true }) |published| {
        var alloc = std.testing.FailingAllocator.init(std.testing.allocator, .{ .fail_index = 5 });
        var g = try create(alloc.allocator());
        // Failure-only teardown keeps diagnostics leak-free; the success path
        // below must destroy through the public API with accounting assertions.
        defer if (g.handle != null) {
            const owned = view.storage(&g) catch unreachable;
            g.handle = null;
            owned.deinit();
        };
        try std.testing.expectEqual(state.maximum_requested_bytes, alloc.allocated_bytes);
        const s = try view.storage(&g);
        var input_bytes: [256]u8 = undefined;
        const input = try @import("command.zig").encodeEnvelope(@fromBackingInt(1), .{ .sequence = @fromBackingInt(1), .provenance = .creator, .actor = null }, .{ .author_settlement = .{ .owner = @fromBackingInt(1), .anchor = .{ .planet = @fromBackingInt(1), .local = 70 }, .population = 2, .food = 12, .material = 8, .economic_bypass = true } }, &input_bytes);
        const id = try api.beginTick(&g, @fromBackingInt(0), @fromBackingInt(1), creator, &.{input});
        try reach(&g, id, published);
        var token = try api.acquireView(&g);
        const before = try api.summary(&g, token);
        const settlement = if (published) try api.settlement(&g, token, .{ .planet = @fromBackingInt(1), .local = @fromBackingInt(1) }) else null;
        var cells: [4]api.CellValue = undefined;
        for (&cells, 0..) |*cell, i| cell.* = try api.cell(&g, token, .{ .planet = @fromBackingInt(@as(u32, @intCast(i)) + 1), .local = 70 });
        const p = &s.metadata().planets[0].?;
        const discarded = if (published) s.metadata().descriptors[s.metadata().obsolete_descriptor.?].planets[0].root else try p.candidateRoot();
        // Damage only the unreachable candidate/obsolete directory. The
        // independent original allocation ledger is outside this fault model.
        p.storage().reservation.directories[discarded.slot][0] = std.math.maxInt(u64);
        if (!published) try std.testing.expectEqual(.requested, api.cancel(&g, id));
        try fault(&g, id);
        if (!published) {
            try api.releaseView(&g, token);
            // Zero leases alone is insufficient: explicit shutdown is required.
            try std.testing.expectError(error.Busy, api.destroyFaulted(&g));
            try std.testing.expectEqual(0, alloc.deallocations);
            token = try api.acquireView(&g);
        }
        try expectPermanent(&g, id);
        try std.testing.expectError(error.Busy, api.deinit(&g));
        try std.testing.expectError(error.Busy, api.destroyFaulted(&g));
        try std.testing.expectError(error.CleanupFault, api.beginTick(&g, before.revision, @fromBackingInt(@backingInt(before.tick) + 1), creator, &.{}));
        const fresh = try api.acquireView(&g);
        const owner_generation = fresh.owner_generation;
        const work = job.control(s).work;
        try api.beginFaultShutdown(&g);
        try api.beginFaultShutdown(&g);
        try std.testing.expectEqual(work, job.control(s).work);
        try std.testing.expectEqual(owner_generation, s.metadata().owner_generation);
        try std.testing.expectError(error.Shutdown, api.acquireView(&g));
        try std.testing.expectError(error.Shutdown, api.beginTick(&g, before.revision, @fromBackingInt(@backingInt(before.tick) + 1), creator, &.{}));
        try std.testing.expectError(error.Busy, api.deinit(&g));
        try std.testing.expectError(error.Busy, api.destroyFaulted(&g));
        try expectPermanent(&g, id);
        for ([_]api.ViewToken{ token, fresh }) |v| {
            try std.testing.expectEqualDeep(before, try api.summary(&g, v));
            if (settlement) |value| try std.testing.expectEqualDeep(value, try api.settlement(&g, v, value.id));
            for (cells) |cell| try std.testing.expectEqualDeep(cell, try api.cell(&g, v, cell.id));
            var rows: [1]api.ReceiptValue = undefined;
            try std.testing.expectEqual(1, (try api.receipts(&g, v, 0, &rows)).written);
            var local_rows: [1]api.LocalReceiptValue = undefined;
            try std.testing.expectEqual(1, (try api.planetReceipts(&g, v, @fromBackingInt(1), 0, &local_rows)).written);
        }
        try api.releaseView(&g, token);
        try std.testing.expectError(error.StaleView, api.releaseView(&g, token));
        try std.testing.expectError(error.Busy, api.destroyFaulted(&g));
        try std.testing.expectEqual(0, alloc.deallocations);
        try api.releaseView(&g, fresh);
        try api.destroyFaulted(&g);
        try std.testing.expect(g.handle == null);
        try std.testing.expectEqual(5, alloc.deallocations);
        try std.testing.expectEqual(alloc.allocated_bytes, alloc.freed_bytes);
        try std.testing.expectEqual(5, alloc.alloc_index);
        try std.testing.expect(!alloc.has_induced_failure);
        // Destruction must not reset the noncanonical process owner sequence.
        var next = try create(std.testing.allocator);
        const next_view = try api.acquireView(&next);
        try std.testing.expect(next_view.owner_generation > owner_generation);
        try std.testing.expectError(error.StaleView, api.summary(&next, fresh));
        try api.releaseView(&next, next_view);
        try api.deinit(&next);
    }
}

test "cg/fault/healthy-shutdown-rejected-without-mutation" {
    var g = try create(std.testing.allocator);
    defer api.deinit(&g) catch unreachable;
    const s = try view.storage(&g);
    try std.testing.expectError(error.NotReady, api.beginFaultShutdown(&g));
    try std.testing.expectError(error.Busy, api.destroyFaulted(&g));
    try std.testing.expect(!s.metadata().acquisition_closed);
    const v = try api.acquireView(&g);
    try api.releaseView(&g, v);
}

test "cg/fault/creation-allocation-ownership" {
    // Failure at coordinator and after each successful prefix of planet
    // allocations; no poll has initialized page semantics at this point.
    for (0..5) |index| {
        var alloc = std.testing.FailingAllocator.init(std.testing.allocator, .{ .fail_index = index });
        try std.testing.expectError(error.OutOfMemory, api.beginCreate(alloc.allocator(), config));
        try std.testing.expectEqual(index, alloc.deallocations);
        try std.testing.expectEqual(alloc.allocated_bytes, alloc.freed_bytes);
        const bytes = if (index == 0) 0 else state.requested_bytes + (index - 1) * state.planet_requested_bytes;
        try std.testing.expectEqual(bytes, alloc.allocated_bytes);
    }
    // Stop precisely between retaining an old page and writing its candidate
    // directory entry. Healthy cancellation must release this extra reference.
    var alloc = std.testing.FailingAllocator.init(std.testing.allocator, .{ .fail_index = 5 });
    var g = try create(alloc.allocator());
    const s = try view.storage(&g);
    const p = &s.metadata().planets[0].?;
    const pages = p.storage().live_pages;
    const id = try api.beginTick(&g, @fromBackingInt(0), @fromBackingInt(1), creator, &.{});
    var retained = false;
    for (0..500_000) |_| {
        var b: api.WorkBudget = .{ .remaining = 1 };
        try std.testing.expect(api.poll(&g, id, &b) == .pending);
        if (p.control.lane) |lane| if (lane.candidate.prepare_retained) {
            retained = true;
            break;
        };
    }
    try std.testing.expect(retained);
    const candidate = &p.control.lane.?.candidate;
    const ref = p.storage().reservation.directories[p.storage().active][candidate.prepare_index];
    try std.testing.expectEqual(2, p.storage().reservation.pool[@intCast(ref - 1)].references);
    try std.testing.expectEqual(.requested, api.cancel(&g, id));
    var canceled: u8 = 0;
    var idle = false;
    for (0..500_000) |_| {
        var b: api.WorkBudget = .{ .remaining = 7 };
        switch (api.poll(&g, id, &b)) {
            .pending => {},
            .canceled => canceled += 1,
            .idle => {
                idle = true;
                break;
            },
            else => return error.UnexpectedDrain,
        }
    }
    try std.testing.expect(idle);
    try std.testing.expectEqual(1, canceled);
    try std.testing.expectEqual(1, p.storage().reservation.pool[@intCast(ref - 1)].references);
    try std.testing.expectEqual(pages, p.storage().live_pages);
    try std.testing.expect(!s.metadata().faulted);
    try api.deinit(&g);
    try std.testing.expectEqual(5, alloc.deallocations);
    try std.testing.expectEqual(alloc.allocated_bytes, alloc.freed_bytes);
    try std.testing.expectEqual(5, alloc.alloc_index);
    try std.testing.expect(!alloc.has_induced_failure);
}

test "cg/fault/partial-cow-cancel-and-allocation-failure-drain" {
    const command = @import("command.zig");
    // Cancel with an allocated unswapped page, with an unreleased old ref, or
    // fail the next pool allocation after a successful partial COW mutation.
    for (0..3) |case| {
        var alloc = std.testing.FailingAllocator.init(std.testing.allocator, .{ .fail_index = 5 });
        var g = try create(alloc.allocator());
        defer if (g.handle != null) {
            const owned = view.storage(&g) catch unreachable;
            g.handle = null;
            owned.deinit();
        };
        const s = try view.storage(&g);
        const p = &s.metadata().planets[0].?;
        const pages = p.storage().live_pages;
        const old = try api.acquireView(&g);
        const before = try api.summary(&g, old);
        const before_cell = try api.cell(&g, old, .{ .planet = @fromBackingInt(1), .local = 70 });
        var bytes: [256]u8 = undefined;
        const input = try command.encodeEnvelope(@fromBackingInt(1), .{ .sequence = @fromBackingInt(1), .provenance = .creator, .actor = null }, .{ .author_settlement = .{ .owner = @fromBackingInt(1), .anchor = .{ .planet = @fromBackingInt(1), .local = 70 }, .population = 2, .food = 12, .material = 8, .economic_bypass = true } }, &bytes);
        const id = try api.beginTick(&g, @fromBackingInt(0), @fromBackingInt(1), creator, &.{input});
        var reached = false;
        for (0..500_000) |_| {
            var b: api.WorkBudget = .{ .remaining = 1 };
            try std.testing.expect(api.poll(&g, id, &b) == .pending);
            if (p.control.lane) |lane| {
                const candidate = lane.candidate;
                if ((case != 1 and candidate.pending_new_ref != 0) or (case == 1 and candidate.pending_old_ref != 0)) {
                    reached = true;
                    break;
                }
            }
        }
        try std.testing.expect(reached);
        try std.testing.expect(p.storage().live_pages > pages);
        if (case == 2) {
            // Exhaust only the semantic free-page head, never the independent
            // allocation ledger. Restore it before rollback can release pages.
            const free_head = p.storage().free_head;
            p.storage().free_head = 0;
            for (0..500_000) |_| {
                var b: api.WorkBudget = .{ .remaining = 1 };
                try std.testing.expect(api.poll(&g, id, &b) == .pending);
                if (job.control(s).tick.failure != null) break;
            }
            p.storage().free_head = free_head;
            try std.testing.expectEqual(error.InvariantViolation, job.control(s).tick.failure.?);
            try std.testing.expect(p.storage().pages_copied > 0);
        } else try std.testing.expectEqual(.requested, api.cancel(&g, id));
        var terminals: u8 = 0;
        var idle = false;
        for (0..500_000) |_| {
            var zero: api.WorkBudget = .{ .remaining = 0 };
            try std.testing.expect(api.poll(&g, id, &zero) == .pending);
            var b: api.WorkBudget = .{ .remaining = 7 };
            switch (api.poll(&g, id, &b)) {
                .pending => {},
                .canceled => {
                    try std.testing.expect(case != 2);
                    terminals += 1;
                },
                .failed => |err| {
                    try std.testing.expectEqual(2, case);
                    try std.testing.expectEqual(error.InvariantViolation, err);
                    terminals += 1;
                },
                .idle => {
                    idle = true;
                    break;
                },
                else => return error.UnexpectedDrain,
            }
        }
        try std.testing.expect(idle);
        try std.testing.expectEqual(1, terminals);
        try std.testing.expectEqual(pages, p.storage().live_pages);
        try std.testing.expect(!s.metadata().faulted);
        for (p.storage().reservation.directories[p.storage().active]) |ref| {
            if (ref != 0) try std.testing.expectEqual(1, p.storage().reservation.pool[@intCast(ref - 1)].references);
        }
        for (p.storage().reservation.directories[1 - p.storage().active]) |ref| try std.testing.expectEqual(0, ref);
        const fresh = try api.acquireView(&g);
        for ([_]api.ViewToken{ old, fresh }) |v| {
            try std.testing.expectEqualDeep(before, try api.summary(&g, v));
            try std.testing.expectEqualDeep(before_cell, try api.cell(&g, v, before_cell.id));
            try api.releaseView(&g, v);
        }
        // A drained failure/cancel leaves healthy mutation admission available.
        const retry = try api.beginTick(&g, @fromBackingInt(0), @fromBackingInt(1), creator, &.{input});
        var publications: u8 = 0;
        idle = false;
        for (0..500_000) |_| {
            var b: api.WorkBudget = .{ .remaining = 4096 };
            switch (api.poll(&g, retry, &b)) {
                .pending => {},
                .published => publications += 1,
                .idle => {
                    idle = true;
                    break;
                },
                else => return error.UnexpectedRetry,
            }
        }
        try std.testing.expect(idle);
        try std.testing.expectEqual(1, publications);
        try api.deinit(&g);
        try std.testing.expectEqual(5, alloc.deallocations);
        try std.testing.expectEqual(alloc.allocated_bytes, alloc.freed_bytes);
        try std.testing.expectEqual(5, alloc.alloc_index);
        try std.testing.expect(!alloc.has_induced_failure);
    }
}
