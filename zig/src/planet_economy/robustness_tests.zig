const std = @import("std");
const pe = @import("root.zig");
const helpers = @import("jobs_tests.zig");

const CountingAllocator = struct {
    backing: std.mem.Allocator,
    attempts: usize = 0,

    fn allocator(self: *CountingAllocator) std.mem.Allocator {
        return .{ .ptr = self, .vtable = &.{ .alloc = alloc, .resize = resize, .remap = remap, .free = free } };
    }
    fn alloc(ctx: *anyopaque, len: usize, alignment: std.mem.Alignment, address: usize) ?[*]u8 {
        const self: *CountingAllocator = @ptrCast(@alignCast(ctx));
        self.attempts += 1;
        return self.backing.rawAlloc(len, alignment, address);
    }
    fn resize(ctx: *anyopaque, memory: []u8, alignment: std.mem.Alignment, new_len: usize, address: usize) bool {
        const self: *CountingAllocator = @ptrCast(@alignCast(ctx));
        self.attempts += 1;
        return self.backing.rawResize(memory, alignment, new_len, address);
    }
    fn remap(ctx: *anyopaque, memory: []u8, alignment: std.mem.Alignment, new_len: usize, address: usize) ?[*]u8 {
        const self: *CountingAllocator = @ptrCast(@alignCast(ctx));
        self.attempts += 1;
        return self.backing.rawRemap(memory, alignment, new_len, address);
    }
    fn free(ctx: *anyopaque, memory: []u8, alignment: std.mem.Alignment, address: usize) void {
        const self: *CountingAllocator = @ptrCast(@alignCast(ctx));
        self.backing.rawFree(memory, alignment, address);
    }
};

fn createWith(allocator: std.mem.Allocator, config: pe.Config) !pe.Authority {
    var builder = try pe.beginCreate(allocator, config);
    defer builder.deinit();
    while (true) {
        var budget: pe.WorkBudget = .{ .remaining = 4096 };
        switch (pe.pollCreate(&builder, &budget)) {
            .pending => {},
            .ready => return pe.takeAuthority(&builder),
            .failed => |err| return err,
            else => return error.UnexpectedCreate,
        }
    }
}

fn drainCanceled(authority: *pe.Authority, id: pe.JobId) !void {
    try std.testing.expectEqual(pe.CancelResult.requested, authority.cancel(id));
    var terminals: usize = 0;
    while (true) {
        var budget: pe.WorkBudget = .{ .remaining = 256 };
        switch (authority.poll(id, &budget)) {
            .pending => {},
            .canceled => terminals += 1,
            .idle => break,
            else => return error.UnexpectedProgress,
        }
    }
    try std.testing.expectEqual(@as(usize, 1), terminals);
}

fn receiptBytes(authority: *const pe.Authority, index: u16) !pe.ReceiptView {
    var rows: [1]pe.ReceiptView = undefined;
    try std.testing.expectEqual(@as(u16, 1), (try authority.receipts(authority.summary().revision, index, &rows)).written);
    return rows[0];
}

fn expectSameWorld(expected: *const pe.Authority, actual: *const pe.Authority) !void {
    try std.testing.expectEqualDeep(expected.storage.config, actual.storage.config);
    try std.testing.expectEqualDeep(expected.summary(), actual.summary());
    try std.testing.expectEqualDeep(
        expected.storage.rootMeta(expected.storage.active),
        actual.storage.rootMeta(actual.storage.active),
    );
    const cells: usize = @as(usize, 6) * expected.storage.config.side * expected.storage.config.side;
    for (0..cells) |local| {
        const id: pe.CellId = .{ .planet = pe.planet_id, .local = @intCast(local) };
        try std.testing.expectEqualDeep(try expected.cell(id), try actual.cell(id));
    }
    for (0..expected.storage.config.capacities.settlements) |slot| {
        try std.testing.expectEqualDeep(
            try expected.storage.readRecord(expected.storage.active, .settlement, @intCast(slot)),
            try actual.storage.readRecord(actual.storage.active, .settlement, @intCast(slot)),
        );
    }
    for (0..expected.storage.config.capacities.buildings) |slot| {
        try std.testing.expectEqualDeep(
            try expected.storage.readRecord(expected.storage.active, .building, @intCast(slot)),
            try actual.storage.readRecord(actual.storage.active, .building, @intCast(slot)),
        );
    }
    for (0..expected.storage.config.capacities.shipments) |slot| {
        const left = (try expected.storage.readRecord(expected.storage.active, .shipment, @intCast(slot))).shipment;
        const right = (try actual.storage.readRecord(actual.storage.active, .shipment, @intCast(slot))).shipment;
        try std.testing.expectEqualDeep(left, right);
        if (@backingInt(left.id) != 0) for (0..left.route_len) |index| {
            try std.testing.expectEqualDeep(
                try expected.storage.routeCell(expected.storage.active, @intCast(slot), @intCast(index)),
                try actual.storage.routeCell(actual.storage.active, @intCast(slot), @intCast(index)),
            );
        };
    }
    try std.testing.expectEqual(expected.stats().history_used, actual.stats().history_used);
    for (0..expected.stats().history_used) |index| {
        const left = try receiptBytes(expected, @intCast(index));
        const right = try receiptBytes(actual, @intCast(index));
        try std.testing.expectEqual(left.len, right.len);
        try std.testing.expectEqualSlices(u8, left.bytes[0..left.len], right.bytes[0..right.len]);
    }
}

fn write(candidate: *pe.state.Candidate, index: u16, record: pe.state.Record) !void {
    var writer = pe.state.RecordWriter.init(index, record);
    while (true) {
        var budget: pe.WorkBudget = .{ .remaining = 256 };
        if (try writer.advance(candidate, &budget) == .done) return;
    }
}

fn mutationBaseline() !pe.Authority {
    var authority = try helpers.create(.{ .seed = 2, .side = 4 });
    const policy = try pe.SessionPolicy.init(.combined, pe.civilization_id);
    const path = [_]pe.CellId{
        .{ .planet = pe.planet_id, .local = 70 },
        .{ .planet = pe.planet_id, .local = 69 },
    };
    var bytes: [5][1024]u8 = undefined;
    const batch = [_]pe.Envelope{
        try pe.encodeEnvelope(.{ .sequence = @fromBackingInt(1), .provenance = .creator, .actor = null }, .{ .author_settlement = .{
            .owner = pe.civilization_id,
            .anchor = .{ .planet = pe.planet_id, .local = 70 },
            .population = 4,
            .food = 12,
            .material = 20,
            .economic_bypass = true,
        } }, &bytes[0]),
        try pe.encodeEnvelope(.{ .sequence = @fromBackingInt(2), .provenance = .creator, .actor = null }, .{ .author_settlement = .{
            .owner = pe.civilization_id,
            .anchor = .{ .planet = pe.planet_id, .local = 69 },
            .population = 0,
            .food = 0,
            .material = 0,
            .economic_bypass = true,
        } }, &bytes[1]),
        try pe.encodeEnvelope(.{ .sequence = @fromBackingInt(3), .provenance = .ruler, .actor = pe.civilization_id }, .{ .build = .{
            .settlement = @fromBackingInt(1),
            .kind = .farm,
            .anchor = .{ .planet = pe.planet_id, .local = 71 },
            .facing = .east,
        } }, &bytes[2]),
        try pe.encodeEnvelope(.{ .sequence = @fromBackingInt(4), .provenance = .ruler, .actor = pe.civilization_id }, .{ .build = .{
            .settlement = @fromBackingInt(1),
            .kind = .quarry,
            .anchor = .{ .planet = pe.planet_id, .local = 74 },
            .facing = null,
        } }, &bytes[3]),
        try pe.encodeEnvelope(.{ .sequence = @fromBackingInt(5), .provenance = .ruler, .actor = pe.civilization_id }, .{ .ship = .{
            .source = @fromBackingInt(1),
            .destination = @fromBackingInt(2),
            .resource = .food,
            .amount = 1,
            .path = &path,
        } }, &bytes[4]),
    };
    const result = try helpers.tick(&authority, policy, &batch, 256);
    try std.testing.expectEqual(@as(u16, 5), result.accepted);
    return authority;
}

test "pe/robustness/every-cursor-cancel leaves active root and retry unchanged" {
    const policy = try pe.SessionPolicy.init(.creator, null);
    var reference = try helpers.create(.{ .seed = 2, .side = 4 });
    defer reference.deinit();
    var bytes: [1024]u8 = undefined;
    const envelope = try helpers.author(&bytes, 1);
    const reference_id = try reference.beginTick(reference.summary().revision, policy, &.{envelope});
    var publish_units: usize = 0;
    var phases_seen = std.mem.zeroes([10]bool);
    var expected: pe.TickSummary = undefined;
    while (true) {
        phases_seen[@backingInt(reference.lane.?.phase)] = true;
        var one: pe.WorkBudget = .{ .remaining = 1 };
        const progress = reference.poll(reference_id, &one);
        publish_units += 1 - one.remaining;
        switch (progress) {
            .pending => {},
            .published => |value| {
                expected = value;
                break;
            },
            else => return error.UnexpectedProgress,
        }
    }
    for ([_]pe.Phase{ .prepare, .commands, .arrivals, .construction, .production, .population, .finalize, .publish }) |phase| {
        try std.testing.expect(phases_seen[@backingInt(phase)]);
    }
    std.debug.print("pe/robustness/every-cursor-cancel publish_units={d} cancel_points={d}\n", .{ publish_units, publish_units });
    try std.testing.expect(reference.stats().pages_copied > 0);
    try std.testing.expectEqual(pe.CancelResult.already_published, reference.cancel(reference_id));
    while (true) {
        var budget: pe.WorkBudget = .{ .remaining = 256 };
        if (reference.poll(reference_id, &budget) == .idle) break;
    }
    var clean = try helpers.create(.{ .seed = 2, .side = 4 });
    defer clean.deinit();
    // Each candidate is canceled after exactly `limit` charged primitives.
    // Publish the identical retry, compare the whole root/history, then reset.
    for (0..publish_units) |limit| {
        var authority = try helpers.create(.{ .seed = 2, .side = 4 });
        defer authority.deinit();
        const id = try authority.beginTick(clean.summary().revision, policy, &.{envelope});
        var consumed: usize = 0;
        while (consumed < limit) {
            var budget: pe.WorkBudget = .{ .remaining = @intCast(@min(limit - consumed, 4096)) };
            const available = budget.remaining;
            switch (authority.poll(id, &budget)) {
                .pending => {},
                else => return error.PublishedBeforeBoundary,
            }
            const used = available - budget.remaining;
            try std.testing.expect(used > 0);
            consumed += used;
        }
        var zero: pe.WorkBudget = .{ .remaining = 0 };
        const charged = authority.stats().work_units;
        try std.testing.expect(authority.poll(id, &zero) == .pending);
        try std.testing.expectEqual(charged, authority.stats().work_units);
        try drainCanceled(&authority, id);
        try expectSameWorld(&clean, &authority);
        try std.testing.expectEqual(@as(u16, 1216), authority.stats().live_pages);
        const actual = try helpers.tick(&authority, policy, &.{envelope}, 1);
        try std.testing.expectEqualDeep(expected, actual);
        try expectSameWorld(&reference, &authority);
    }
}

test "pe/robustness/history-tail-cancel discards invisible bytes before identical retry" {
    var authority = try helpers.create(.{ .seed = 2, .side = 4 });
    defer authority.deinit();
    const policy = try pe.SessionPolicy.init(.creator, null);
    const before_summary = authority.summary();
    const before_meta = authority.storage.rootMeta(authority.storage.active);
    var bytes: [1024]u8 = undefined;
    const envelope = try helpers.author(&bytes, 1);
    const id = try authority.beginTick(before_summary.revision, policy, &.{envelope});
    while (true) {
        var budget: pe.WorkBudget = .{ .remaining = 1 };
        switch (authority.poll(id, &budget)) {
            .pending => {},
            else => return error.PublishedBeforeTail,
        }
        if (authority.lane.?.step == .finalize and authority.lane.?.finalizer.index > 0) break;
    }
    try std.testing.expect(authority.storage.reservation.history_meta[before_meta.history_used] > 0);
    try std.testing.expectEqualDeep(before_summary, authority.summary());
    var row: [1]pe.ReceiptView = undefined;
    try std.testing.expectEqualDeep(pe.ReceiptPage{ .written = 0, .next = null }, try authority.receipts(before_summary.revision, before_meta.history_used, &row));
    try drainCanceled(&authority, id);
    try std.testing.expectEqualDeep(before_meta, authority.storage.rootMeta(authority.storage.active));
    var reference = try helpers.create(.{ .seed = 2, .side = 4 });
    defer reference.deinit();
    const expected = try helpers.tick(&reference, policy, &.{envelope}, 256);
    try std.testing.expectEqualDeep(expected, try helpers.tick(&authority, policy, &.{envelope}, 1));
    for (0..reference.stats().history_used) |i| {
        const lhs = try receiptBytes(&reference, @intCast(i));
        const rhs = try receiptBytes(&authority, @intCast(i));
        try std.testing.expectEqualSlices(u8, lhs.bytes[0..lhs.len], rhs.bytes[0..rhs.len]);
    }
}

test "pe/robustness/postpublication-cleanup keeps published root visible at every disposal unit" {
    const policy = try pe.SessionPolicy.init(.creator, null);
    var bytes: [1024]u8 = undefined;
    const envelope = try helpers.author(&bytes, 1);
    var reference = try helpers.create(.{ .seed = 2, .side = 4 });
    defer reference.deinit();
    _ = try helpers.tick(&reference, policy, &.{envelope}, 256);
    var authority = try helpers.create(.{ .seed = 2, .side = 4 });
    defer authority.deinit();
    const id = try authority.beginTick(authority.summary().revision, policy, &.{envelope});
    while (true) {
        var budget: pe.WorkBudget = .{ .remaining = 256 };
        switch (authority.poll(id, &budget)) {
            .pending => {},
            .published => break,
            else => return error.UnexpectedProgress,
        }
    }
    try expectSameWorld(&reference, &authority);
    var cleanup_units: usize = 0;
    while (true) {
        try std.testing.expectEqual(pe.CancelResult.already_published, authority.cancel(id));
        try std.testing.expectError(error.Busy, authority.beginTick(authority.summary().revision, policy, &.{}));
        try expectSameWorld(&reference, &authority);
        var budget: pe.WorkBudget = .{ .remaining = 1 };
        const before = authority.stats().work_units;
        const progress = authority.poll(id, &budget);
        cleanup_units += 1 - budget.remaining;
        try std.testing.expectEqual(@as(u64, 1 - budget.remaining), authority.stats().work_units - before);
        try expectSameWorld(&reference, &authority);
        switch (progress) {
            .pending => {},
            .idle => break,
            else => return error.UnexpectedProgress,
        }
    }
    try std.testing.expect(cleanup_units > 0);
    std.debug.print("pe/robustness/postpublication-cleanup cleanup_units={d}\n", .{cleanup_units});
    try std.testing.expectEqual(pe.CancelResult.already_published, authority.cancel(id));
    const next = try authority.beginTick(authority.summary().revision, policy, &.{});
    try drainCanceled(&authority, next);
    try expectSameWorld(&reference, &authority);
}

test "pe/robustness/create-cancel retries every charged creation boundary" {
    const config = pe.Config{ .seed = 2, .side = 4 };
    var reference_builder = try pe.beginCreate(std.testing.allocator, config);
    defer reference_builder.deinit();
    var create_units: usize = 0;
    while (true) {
        var budget: pe.WorkBudget = .{ .remaining = 1 };
        const progress = pe.pollCreate(&reference_builder, &budget);
        create_units += 1 - budget.remaining;
        switch (progress) {
            .pending => {},
            .ready => break,
            else => return error.UnexpectedCreate,
        }
    }
    var reference = try pe.takeAuthority(&reference_builder);
    defer reference.deinit();
    for (0..create_units) |limit| {
        var builder = try pe.beginCreate(std.testing.allocator, config);
        defer builder.deinit();
        try std.testing.expectEqual(@as(u16, 0), builder.stats().history_used);
        var consumed: usize = 0;
        while (consumed < limit) {
            var budget: pe.WorkBudget = .{ .remaining = @intCast(@min(limit - consumed, 4096)) };
            const available = budget.remaining;
            try std.testing.expect(pe.pollCreate(&builder, &budget) == .pending);
            const used = available - budget.remaining;
            try std.testing.expect(used > 0);
            consumed += used;
            try std.testing.expectEqual(@as(u16, 0), builder.stats().history_used);
        }
        var zero: pe.WorkBudget = .{ .remaining = 0 };
        const charged = builder.stats().work_units;
        try std.testing.expect(pe.pollCreate(&builder, &zero) == .pending);
        try std.testing.expectEqual(charged, builder.stats().work_units);
        pe.cancelCreate(&builder);
        var canceled: usize = 0;
        while (true) {
            var budget: pe.WorkBudget = .{ .remaining = 256 };
            const progress = pe.pollCreate(&builder, &budget);
            try std.testing.expectEqual(@as(u16, 0), builder.stats().history_used);
            switch (progress) {
                .pending => {},
                .canceled => canceled += 1,
                .idle => break,
                else => return error.UnexpectedCreate,
            }
        }
        try std.testing.expectEqual(@as(usize, 1), canceled);
        try std.testing.expectEqual(@as(u16, 0), builder.stats().live_pages);
        var retry = try createWith(std.testing.allocator, config);
        defer retry.deinit();
        try expectSameWorld(&reference, &retry);
    }
    std.debug.print("pe/robustness/create-cancel create_units={d} cancel_points={d}\n", .{ create_units, create_units });
}

test "pe/robustness/nonempty-arrival-construction-production cancels every charged tick boundary" {
    const policy = try pe.SessionPolicy.init(.combined, pe.civilization_id);
    var clean = try mutationBaseline();
    defer clean.deinit();
    var reference = try mutationBaseline();
    defer reference.deinit();
    const reference_id = try reference.beginTick(reference.summary().revision, policy, &.{});
    var publish_units: usize = 0;
    while (true) {
        var budget: pe.WorkBudget = .{ .remaining = 1 };
        const progress = reference.poll(reference_id, &budget);
        publish_units += 1 - budget.remaining;
        switch (progress) {
            .pending => {},
            .published => break,
            else => return error.UnexpectedProgress,
        }
    }
    while (true) {
        var budget: pe.WorkBudget = .{ .remaining = 256 };
        if (reference.poll(reference_id, &budget) == .idle) break;
    }
    try std.testing.expectEqual(@as(u16, 0), reference.storage.rootMeta(reference.storage.active).shipments);
    try std.testing.expectEqual(@as(u32, 1), (try reference.settlement(@fromBackingInt(2))).food);
    try std.testing.expectEqual(@as(u32, 12), (try reference.settlement(@fromBackingInt(1))).material);
    for (0..publish_units) |limit| {
        var authority = try mutationBaseline();
        defer authority.deinit();
        const id = try authority.beginTick(authority.summary().revision, policy, &.{});
        var consumed: usize = 0;
        while (consumed < limit) {
            var budget: pe.WorkBudget = .{ .remaining = @intCast(@min(limit - consumed, 4096)) };
            const available = budget.remaining;
            try std.testing.expect(authority.poll(id, &budget) == .pending);
            const used = available - budget.remaining;
            try std.testing.expect(used > 0);
            consumed += used;
        }
        try drainCanceled(&authority, id);
        try expectSameWorld(&clean, &authority);
        _ = try helpers.tick(&authority, policy, &.{}, 1);
        try expectSameWorld(&reference, &authority);
    }
    std.debug.print("pe/robustness/nonempty-arrival-construction-production publish_units={d} cancel_points={d}\n", .{ publish_units, publish_units });
}

test "pe/robustness/no-poll-allocation after setup" {
    var counter = CountingAllocator{ .backing = std.testing.allocator };
    var authority = try createWith(counter.allocator(), .{ .seed = 2, .side = 32 });
    defer authority.deinit();
    try std.testing.expectEqual(@as(usize, 1), counter.attempts);
    const setup_attempts = counter.attempts;
    const before_summary = authority.summary();
    const policy = try pe.SessionPolicy.init(.creator, null);
    const id = try authority.beginTick(before_summary.revision, policy, &.{});
    var published = false;
    while (true) {
        var budget: pe.WorkBudget = .{ .remaining = 1 };
        const before_work = authority.stats().work_units;
        const before_copies = authority.stats().pages_copied;
        const before_pages = authority.stats().live_pages;
        switch (authority.poll(id, &budget)) {
            .pending => {},
            .published => published = true,
            .idle => break,
            else => return error.UnexpectedProgress,
        }
        const used = 1 - budget.remaining;
        try std.testing.expectEqual(@as(u64, used), authority.stats().work_units - before_work);
        try std.testing.expect(authority.stats().pages_copied - before_copies <= used);
        if (authority.stats().live_pages > before_pages) {
            try std.testing.expect(authority.stats().live_pages - before_pages <= used);
        }
    }
    try std.testing.expect(published);
    try std.testing.expectEqual(@as(usize, 0), counter.attempts - setup_attempts);
    try std.testing.expect(authority.stats().peak_pages <= 4096);
    var bytes: [1024]u8 = undefined;
    const envelope = try helpers.author(&bytes, 1);
    const next = try authority.beginTick(authority.summary().revision, policy, &.{envelope});
    var nonempty_published = false;
    while (true) {
        var budget: pe.WorkBudget = .{ .remaining = 1 };
        const before_work = authority.stats().work_units;
        const before_copies = authority.stats().pages_copied;
        const progress = authority.poll(next, &budget);
        const used = 1 - budget.remaining;
        try std.testing.expectEqual(@as(u64, used), authority.stats().work_units - before_work);
        try std.testing.expect(authority.stats().pages_copied - before_copies <= used);
        switch (progress) {
            .pending => {},
            .published => nonempty_published = true,
            .idle => break,
            else => return error.UnexpectedProgress,
        }
    }
    try std.testing.expect(nonempty_published);
    try std.testing.expectEqual(@as(usize, 0), counter.attempts - setup_attempts);
}

test "pe/robustness/setup-oom does not leave a reservation" {
    var failing = std.testing.FailingAllocator.init(std.testing.allocator, .{ .fail_index = 0 });
    try std.testing.expectError(error.OutOfMemory, pe.beginCreate(failing.allocator(), .{ .seed = 2, .side = 32 }));
}

test "pe/robustness/simultaneous-max dirties every mutable entity page without touching active root" {
    var authority = try helpers.create(.{ .seed = 2, .side = 32 });
    defer authority.deinit();
    const before_summary = authority.summary();
    const before_meta = authority.storage.rootMeta(authority.storage.active);
    const before_cell = try authority.cell(.{ .planet = pe.planet_id, .local = 70 });
    var candidate = pe.state.Candidate.begin(&authority.storage);
    while (true) {
        var budget: pe.WorkBudget = .{ .remaining = 256 };
        if (try candidate.advancePrepare(&budget) == .done) break;
    }
    // One record per claim page, every settlement/building/shipment slot,
    // and all three route pages for each shipment. The remaining metadata
    // pages are reserved slack; only the first stores RootMeta.
    for (0..768) |page| try write(&candidate, @intCast(page * 8), .{ .claim = .{ .settlement = @fromBackingInt(1) } });
    for (0..64) |slot| try write(&candidate, @intCast(slot), .{ .settlement = .{
        .id = @fromBackingInt(slot + 1),
        .owner = pe.civilization_id,
        .hall = .{ .planet = pe.planet_id, .local = @intCast(slot) },
        .population = 0,
        .food = 0,
        .material = 0,
        .incoming_food = 0,
        .incoming_material = 0,
        .housing = 4,
        .hunger = 0,
        .fed_streak = 0,
    } });
    for (0..256) |slot| try write(&candidate, @intCast(slot), .{ .building = .{
        .id = @fromBackingInt(slot + 1),
        .settlement = @fromBackingInt(slot / 4 + 1),
        .kind = .house,
        .anchor = .{ .planet = pe.planet_id, .local = @intCast(slot) },
        .facing = null,
        .footprint = .{ .len = 1, .cells = .{
            .{ .planet = pe.planet_id, .local = @intCast(slot) },
            .{ .planet = pe.planet_id, .local = @intCast(slot) },
        } },
        .remaining_ticks = 0,
    } });
    for (0..64) |slot| {
        try write(&candidate, @intCast(slot), .{ .shipment = .{
            .id = @fromBackingInt(slot + 1),
            .source = @fromBackingInt(1),
            .destination = @fromBackingInt(2),
            .resource = .food,
            .quantity = 1,
            .departure_tick = @fromBackingInt(1),
            .arrival_tick = @fromBackingInt(2),
            .route_len = 65,
        } });
        for ([_]u16{ 0, 32, 64 }) |route_index| {
            try write(&candidate, @intCast(slot), .{ .route_cell = .{
                .index = route_index,
                .cell = .{ .planet = pe.planet_id, .local = 70 },
            } });
        }
    }
    var meta = candidate.meta();
    meta.settlements = 64;
    meta.buildings = 256;
    meta.shipments = 64;
    var writer = pe.state.MetaWriter.init(meta);
    while (true) {
        var budget: pe.WorkBudget = .{ .remaining = 256 };
        if (try writer.advance(&candidate, &budget) == .done) break;
    }
    try std.testing.expectEqual(@as(u16, 64), candidate.meta().settlements);
    try std.testing.expectEqual(@as(u16, 256), candidate.meta().buildings);
    try std.testing.expectEqual(@as(u16, 64), candidate.meta().shipments);
    try std.testing.expectEqualDeep(before_summary, authority.summary());
    try std.testing.expectEqualDeep(before_meta, authority.storage.rootMeta(authority.storage.active));
    try std.testing.expectEqualDeep(before_cell, try authority.cell(.{ .planet = pe.planet_id, .local = 70 }));
    try std.testing.expect(authority.stats().peak_pages <= 4096);
    try std.testing.expectEqual(@as(usize, 8_642_560), authority.stats().requested_bytes);
    try std.testing.expectEqual(@as(u64, 1185), authority.stats().pages_copied);
    while (true) {
        var budget: pe.WorkBudget = .{ .remaining = 256 };
        if (try candidate.advanceDiscard(&budget) == .done) break;
    }
    try std.testing.expectEqual(@as(u16, 1216), authority.stats().live_pages);
    try std.testing.expectEqualDeep(before_summary, authority.summary());
}

test "pe/robustness/staging-saturation failure drains and preserves active history" {
    var authority = try helpers.create(.{ .seed = 2, .side = 4 });
    defer authority.deinit();
    const policy = try pe.SessionPolicy.init(.creator, null);
    const before_summary = authority.summary();
    const before_meta = authority.storage.rootMeta(authority.storage.active);
    var bytes: [1024]u8 = undefined;
    const envelope = try helpers.author(&bytes, 1);
    const id = try authority.beginTick(before_summary.revision, policy, &.{envelope});
    while (authority.lane.?.step != .commands) {
        var budget: pe.WorkBudget = .{ .remaining = 1 };
        try std.testing.expect(authority.poll(id, &budget) == .pending);
    }
    authority.storage.staged_count = 1024;
    var failures: usize = 0;
    while (true) {
        var budget: pe.WorkBudget = .{ .remaining = 1 };
        switch (authority.poll(id, &budget)) {
            .pending => {},
            .failed => |err| {
                try std.testing.expectEqual(error.HistoryFull, err);
                failures += 1;
            },
            .idle => break,
            else => return error.UnexpectedProgress,
        }
    }
    try std.testing.expectEqual(@as(usize, 1), failures);
    try std.testing.expectEqualDeep(before_summary, authority.summary());
    try std.testing.expectEqualDeep(before_meta, authority.storage.rootMeta(authority.storage.active));
    try std.testing.expectEqual(@as(u16, 1216), authority.stats().live_pages);
    _ = try helpers.tick(&authority, policy, &.{envelope}, 256);
}

test "pe/robustness/injected prepublication invariant fault preserves active root" {
    var authority = try helpers.create(.{ .seed = 2, .side = 4 });
    defer authority.deinit();
    var clean = try helpers.create(.{ .seed = 2, .side = 4 });
    defer clean.deinit();
    const policy = try pe.SessionPolicy.init(.creator, null);
    var bytes: [1024]u8 = undefined;
    const envelope = try helpers.author(&bytes, 1);
    const id = try authority.beginTick(authority.summary().revision, policy, &.{envelope});
    while (authority.lane.?.step != .commands) {
        var budget: pe.WorkBudget = .{ .remaining = 1 };
        try std.testing.expect(authority.poll(id, &budget) == .pending);
    }
    const spare = authority.lane.?.candidate.root;
    const page = pe.state.region_start[1] + 70 / 8;
    const saved = authority.storage.reservation.directories[spare][page];
    try std.testing.expect(saved != 0);
    authority.storage.reservation.directories[spare][page] = 0;
    var failures: usize = 0;
    while (authority.lane.?.failure == null) {
        var budget: pe.WorkBudget = .{ .remaining = 1 };
        try std.testing.expect(authority.poll(id, &budget) == .pending);
    }
    authority.storage.reservation.directories[spare][page] = saved;
    while (true) {
        var budget: pe.WorkBudget = .{ .remaining = 1 };
        switch (authority.poll(id, &budget)) {
            .pending => {},
            .failed => |err| {
                try std.testing.expectEqual(error.InvariantViolation, err);
                failures += 1;
            },
            .idle => break,
            else => return error.UnexpectedProgress,
        }
    }
    try std.testing.expectEqual(@as(usize, 1), failures);
    try expectSameWorld(&clean, &authority);
    _ = try helpers.tick(&authority, policy, &.{envelope}, 1);
    _ = try helpers.tick(&clean, policy, &.{envelope}, 1);
    try expectSameWorld(&clean, &authority);
}

test "pe/robustness/limits invalid budgets horizon and paging preserve active state" {
    var authority = try helpers.create(.{ .seed = 2, .side = 4, .capacities = .{ .history_receipts = 3 } });
    defer authority.deinit();
    const policy = try pe.SessionPolicy.init(.creator, null);
    const before_summary = authority.summary();
    const id = try authority.beginTick(before_summary.revision, policy, &.{});
    var over: pe.WorkBudget = .{ .remaining = 4097 };
    try std.testing.expectEqual(error.InvalidBudget, authority.poll(id, &over).invalid_request);
    try std.testing.expectEqual(@as(u16, 4097), over.remaining);
    try std.testing.expectEqualDeep(before_summary, authority.summary());
    try drainCanceled(&authority, id);
    _ = try helpers.tick(&authority, policy, &.{}, 256);
    const full = authority.summary();
    try std.testing.expectEqual(@as(u16, 3), authority.stats().history_used);
    const failed_id = try authority.beginTick(full.revision, policy, &.{});
    var failures: usize = 0;
    while (true) {
        var budget: pe.WorkBudget = .{ .remaining = 256 };
        switch (authority.poll(failed_id, &budget)) {
            .pending => {},
            .failed => |err| {
                try std.testing.expectEqual(error.HistoryFull, err);
                failures += 1;
            },
            .idle => break,
            else => return error.UnexpectedProgress,
        }
    }
    try std.testing.expectEqual(@as(usize, 1), failures);
    try std.testing.expectEqualDeep(full, authority.summary());
    var rows: [2]pe.ReceiptView = undefined;
    const first = try authority.receipts(full.revision, 0, rows[0..2]);
    try std.testing.expectEqual(@as(u16, 2), first.written);
    try std.testing.expectEqual(@as(?u16, 2), first.next);
    const last = try authority.receipts(full.revision, first.next.?, rows[0..2]);
    try std.testing.expectEqual(@as(u16, 1), last.written);
    try std.testing.expectEqual(@as(?u16, null), last.next);
    try std.testing.expectError(error.StaleView, authority.receipts(before_summary.revision, 0, rows[0..1]));
    var meta = authority.storage.rootMeta(authority.storage.active);
    meta.tick = @fromBackingInt(1_000_000);
    authority.storage.writeRootMeta(authority.storage.active, meta);
    try std.testing.expectError(error.TickHorizon, authority.beginTick(meta.revision, policy, &.{}));
    meta.tick = @fromBackingInt(1);
    authority.storage.writeRootMeta(authority.storage.active, meta);
    authority.next_job = std.math.maxInt(u64);
    try std.testing.expectError(error.IdExhausted, authority.beginTick(meta.revision, policy, &.{}));
}

test "pe/robustness/limits exact and over entity and inventory bounds" {
    const policy = try pe.SessionPolicy.init(.creator, null);
    var authority = try helpers.create(.{ .seed = 2, .side = 32 });
    defer authority.deinit();
    var buffers: [64][1024]u8 = undefined;
    var batch: [64]pe.Envelope = undefined;
    for (0..64) |index| {
        batch[index] = try pe.encodeEnvelope(.{
            .sequence = @fromBackingInt(index + 1),
            .provenance = .creator,
            .actor = null,
        }, .{ .author_settlement = .{
            .owner = pe.civilization_id,
            .anchor = .{ .planet = pe.planet_id, .local = @intCast(index) },
            .population = 0,
            .food = 0,
            .material = 0,
            .economic_bypass = true,
        } }, &buffers[index]);
    }
    const full = try helpers.tick(&authority, policy, &batch, 256);
    try std.testing.expectEqual(@as(u16, 64), full.accepted);
    try std.testing.expectEqual(@as(u16, 0), full.rejected);
    try std.testing.expectEqual(@as(u16, 64), authority.storage.rootMeta(authority.storage.active).settlements);
    const before_id = authority.storage.rootMeta(authority.storage.active).next_settlement;
    var more_bytes: [1024]u8 = undefined;
    const more = try pe.encodeEnvelope(.{
        .sequence = @fromBackingInt(65),
        .provenance = .creator,
        .actor = null,
    }, .{ .author_settlement = .{
        .owner = pe.civilization_id,
        .anchor = .{ .planet = pe.planet_id, .local = 64 },
        .population = 0,
        .food = 0,
        .material = 0,
        .economic_bypass = true,
    } }, &more_bytes);
    const over = try helpers.tick(&authority, policy, &.{more}, 256);
    try std.testing.expectEqual(@as(u16, 0), over.accepted);
    try std.testing.expectEqual(@as(u16, 1), over.rejected);
    try std.testing.expectEqual(before_id, authority.storage.rootMeta(authority.storage.active).next_settlement);
    try std.testing.expectEqual(@as(u16, 64), authority.storage.rootMeta(authority.storage.active).settlements);
    try std.testing.expectEqual(@as(?pe.SettlementId, null), (try authority.cell(.{ .planet = pe.planet_id, .local = 64 })).settlement);

    var exact = try helpers.create(.{ .seed = 2, .side = 4 });
    defer exact.deinit();
    var exact_bytes: [1024]u8 = undefined;
    const exact_envelope = try pe.encodeEnvelope(.{
        .sequence = @fromBackingInt(1),
        .provenance = .creator,
        .actor = null,
    }, .{ .author_settlement = .{
        .owner = pe.civilization_id,
        .anchor = .{ .planet = pe.planet_id, .local = 70 },
        .population = 0,
        .food = 65_535,
        .material = 65_535,
        .economic_bypass = true,
    } }, &exact_bytes);
    try std.testing.expectEqual(@as(u16, 1), (try helpers.tick(&exact, policy, &.{exact_envelope}, 256)).accepted);
    const stock = try exact.settlement(@fromBackingInt(1));
    try std.testing.expectEqual(@as(u32, 65_535), stock.food);
    try std.testing.expectEqual(@as(u32, 65_535), stock.material);
    var over_bytes: [1024]u8 = undefined;
    const over_envelope = try pe.encodeEnvelope(.{
        .sequence = @fromBackingInt(2),
        .provenance = .creator,
        .actor = null,
    }, .{ .grant = .{
        .settlement = @fromBackingInt(1),
        .food = 1,
        .material = 0,
        .population = 0,
        .economic_bypass = true,
    } }, &over_bytes);
    const rejected = try helpers.tick(&exact, policy, &.{over_envelope}, 256);
    try std.testing.expectEqual(@as(u16, 1), rejected.rejected);
    try std.testing.expectEqual(@as(u32, 65_535), (try exact.settlement(@fromBackingInt(1))).food);
}

test "pe/robustness/public building and project admission exact and one over" {
    const creator = try pe.SessionPolicy.init(.creator, null);
    var authority = try helpers.create(.{ .seed = 2, .side = 32 });
    defer authority.deinit();
    var hall_bytes: [8][1024]u8 = undefined;
    var halls: [8]pe.Envelope = undefined;
    var sequence: u64 = 1;
    for (0..8) |row| {
        halls[row] = try pe.encodeEnvelope(.{ .sequence = @fromBackingInt(sequence), .provenance = .creator, .actor = null }, .{ .author_settlement = .{
            .owner = pe.civilization_id,
            .anchor = .{ .planet = pe.planet_id, .local = @intCast(4096 + row * 128) },
            .population = 0,
            .food = 0,
            .material = 0,
            .economic_bypass = true,
        } }, &hall_bytes[row]);
        sequence += 1;
    }
    try std.testing.expectEqual(@as(u16, 8), (try helpers.tick(&authority, creator, &halls, 256)).accepted);
    var build_bytes: [32][1024]u8 = undefined;
    var builds: [32]pe.Envelope = undefined;
    for (0..8) |row| {
        for (0..32) |x| {
            builds[x] = try pe.encodeEnvelope(.{ .sequence = @fromBackingInt(sequence), .provenance = .creator, .actor = null }, .{ .author_build = .{
                .settlement = @fromBackingInt(row + 1),
                .kind = .house,
                .anchor = .{ .planet = pe.planet_id, .local = @intCast(4096 + row * 128 + 32 + x) },
                .facing = null,
                .economic_bypass = true,
            } }, &build_bytes[x]);
            sequence += 1;
        }
        const result = try helpers.tick(&authority, creator, &builds, 256);
        try std.testing.expectEqual(@as(u16, 32), result.accepted);
        if (row == 0) {
            const meta = authority.storage.rootMeta(authority.storage.active);
            var over_bytes: [1024]u8 = undefined;
            const extra = try pe.encodeEnvelope(.{ .sequence = @fromBackingInt(sequence), .provenance = .creator, .actor = null }, .{ .author_build = .{
                .settlement = @fromBackingInt(1),
                .kind = .house,
                .anchor = .{ .planet = pe.planet_id, .local = 4160 },
                .facing = null,
                .economic_bypass = true,
            } }, &over_bytes);
            sequence += 1;
            try std.testing.expectEqual(@as(u16, 1), (try helpers.tick(&authority, creator, &.{extra}, 256)).rejected);
            try std.testing.expectEqual(pe.RejectReason.SettlementBuildingCapacity, authority.storage.logicalReceipt(1).event.rejected.reason);
            try std.testing.expectEqual(meta.next_building, authority.storage.rootMeta(authority.storage.active).next_building);
            try std.testing.expectEqual(@as(?pe.SettlementId, null), (try authority.cell(.{ .planet = pe.planet_id, .local = 4160 })).settlement);
        }
    }
    const full = authority.storage.rootMeta(authority.storage.active);
    try std.testing.expectEqual(@as(u16, 256), full.buildings);
    try std.testing.expectEqual(@as(u64, 257), full.next_building);
    var over_bytes: [1024]u8 = undefined;
    const extra = try pe.encodeEnvelope(.{ .sequence = @fromBackingInt(sequence), .provenance = .creator, .actor = null }, .{ .author_build = .{
        .settlement = @fromBackingInt(1),
        .kind = .house,
        .anchor = .{ .planet = pe.planet_id, .local = 4160 },
        .facing = null,
        .economic_bypass = true,
    } }, &over_bytes);
    try std.testing.expectEqual(@as(u16, 1), (try helpers.tick(&authority, creator, &.{extra}, 256)).rejected);
    try std.testing.expectEqual(pe.RejectReason.BuildingCapacity, authority.storage.logicalReceipt(1).event.rejected.reason);
    try std.testing.expectEqual(full.next_building, authority.storage.rootMeta(authority.storage.active).next_building);
    try std.testing.expectEqual(@as(?pe.SettlementId, null), (try authority.cell(.{ .planet = pe.planet_id, .local = 4160 })).settlement);
}

test "pe/robustness/public active project and population housing admission" {
    var authority = try helpers.create(.{ .seed = 2, .side = 4, .capacities = .{ .buildings = 1 } });
    defer authority.deinit();
    const policy = try pe.SessionPolicy.init(.combined, pe.civilization_id);
    var bytes: [3][1024]u8 = undefined;
    const setup = [_]pe.Envelope{
        try pe.encodeEnvelope(.{ .sequence = @fromBackingInt(1), .provenance = .creator, .actor = null }, .{ .author_settlement = .{
            .owner = pe.civilization_id,
            .anchor = .{ .planet = pe.planet_id, .local = 70 },
            .population = 0,
            .food = 0,
            .material = 20,
            .economic_bypass = true,
        } }, &bytes[0]),
        try pe.encodeEnvelope(.{ .sequence = @fromBackingInt(2), .provenance = .ruler, .actor = pe.civilization_id }, .{ .build = .{
            .settlement = @fromBackingInt(1),
            .kind = .house,
            .anchor = .{ .planet = pe.planet_id, .local = 71 },
            .facing = null,
        } }, &bytes[1]),
    };
    try std.testing.expectEqual(@as(u16, 2), (try helpers.tick(&authority, policy, &setup, 256)).accepted);
    const project = (try authority.storage.readRecord(authority.storage.active, .building, 0)).building;
    try std.testing.expect(project.remaining_ticks > 0);
    const before = try authority.settlement(@fromBackingInt(1));
    const before_meta = authority.storage.rootMeta(authority.storage.active);
    const extra = try pe.encodeEnvelope(.{ .sequence = @fromBackingInt(3), .provenance = .ruler, .actor = pe.civilization_id }, .{ .build = .{
        .settlement = @fromBackingInt(1),
        .kind = .house,
        .anchor = .{ .planet = pe.planet_id, .local = 75 },
        .facing = null,
    } }, &bytes[2]);
    try std.testing.expectEqual(@as(u16, 1), (try helpers.tick(&authority, policy, &.{extra}, 256)).rejected);
    try std.testing.expectEqual(pe.RejectReason.BuildingCapacity, authority.storage.logicalReceipt(1).event.rejected.reason);
    try std.testing.expectEqual(before_meta.next_building, authority.storage.rootMeta(authority.storage.active).next_building);
    try std.testing.expectEqual(before.material, (try authority.settlement(@fromBackingInt(1))).material);
    try std.testing.expectEqual(@as(?pe.SettlementId, null), (try authority.cell(.{ .planet = pe.planet_id, .local = 75 })).settlement);

    var people = try helpers.create(.{ .seed = 2, .side = 4 });
    defer people.deinit();
    const creator = try pe.SessionPolicy.init(.creator, null);
    var people_bytes: [3][1024]u8 = undefined;
    const hall = try pe.encodeEnvelope(.{ .sequence = @fromBackingInt(1), .provenance = .creator, .actor = null }, .{ .author_settlement = .{
        .owner = pe.civilization_id,
        .anchor = .{ .planet = pe.planet_id, .local = 70 },
        .population = 4,
        .food = 100,
        .material = 0,
        .economic_bypass = true,
    } }, &people_bytes[0]);
    try std.testing.expectEqual(@as(u16, 1), (try helpers.tick(&people, creator, &.{hall}, 256)).accepted);
    var grant = try pe.encodeEnvelope(.{ .sequence = @fromBackingInt(2), .provenance = .creator, .actor = null }, .{ .grant = .{
        .settlement = @fromBackingInt(1),
        .food = 0,
        .material = 0,
        .population = 1,
        .economic_bypass = true,
    } }, &people_bytes[1]);
    try std.testing.expectEqual(@as(u16, 1), (try helpers.tick(&people, creator, &.{grant}, 256)).rejected);
    try std.testing.expectEqual(pe.RejectReason.PopulationCapacity, people.storage.logicalReceipt(1).event.rejected.reason);
    try std.testing.expectEqual(@as(u8, 4), (try people.settlement(@fromBackingInt(1))).housing);
    try std.testing.expectEqual(@as(u32, 4), (try people.settlement(@fromBackingInt(1))).population);
    // Seven authored houses raise housing from 4 to the hard cap of 32.
    var houses: [7]pe.Envelope = undefined;
    var house_bytes: [7][1024]u8 = undefined;
    for ([_]u64{ 71, 75, 74, 73, 69, 65, 66 }, 0..) |cell, index| houses[index] = try pe.encodeEnvelope(.{
        .sequence = @fromBackingInt(index + 3),
        .provenance = .creator,
        .actor = null,
    }, .{ .author_build = .{
        .settlement = @fromBackingInt(1),
        .kind = .house,
        .anchor = .{ .planet = pe.planet_id, .local = cell },
        .facing = null,
        .economic_bypass = true,
    } }, &house_bytes[index]);
    try std.testing.expectEqual(@as(u16, 7), (try helpers.tick(&people, creator, &houses, 256)).accepted);
    try std.testing.expectEqual(@as(u8, 32), (try people.settlement(@fromBackingInt(1))).housing);
    grant = try pe.encodeEnvelope(.{ .sequence = @fromBackingInt(10), .provenance = .creator, .actor = null }, .{ .grant = .{
        .settlement = @fromBackingInt(1),
        .food = 0,
        .material = 0,
        .population = 28,
        .economic_bypass = true,
    } }, &people_bytes[1]);
    try std.testing.expectEqual(@as(u16, 1), (try helpers.tick(&people, creator, &.{grant}, 256)).accepted);
    try std.testing.expectEqual(@as(u32, 32), (try people.settlement(@fromBackingInt(1))).population);
    const over_grant = try pe.encodeEnvelope(.{ .sequence = @fromBackingInt(11), .provenance = .creator, .actor = null }, .{ .grant = .{
        .settlement = @fromBackingInt(1),
        .food = 0,
        .material = 0,
        .population = 1,
        .economic_bypass = true,
    } }, &people_bytes[2]);
    try std.testing.expectEqual(@as(u16, 1), (try helpers.tick(&people, creator, &.{over_grant}, 256)).rejected);
    try std.testing.expectEqual(pe.RejectReason.PopulationCapacity, people.storage.logicalReceipt(1).event.rejected.reason);
    try std.testing.expectEqual(@as(u32, 32), (try people.settlement(@fromBackingInt(1))).population);
}

test "pe/robustness/public freight count quantity and incoming reservation limits" {
    const policy = try pe.SessionPolicy.init(.creator, null);
    const path = [_]pe.CellId{
        .{ .planet = pe.planet_id, .local = 70 },
        .{ .planet = pe.planet_id, .local = 71 },
        .{ .planet = pe.planet_id, .local = 4 },
        .{ .planet = pe.planet_id, .local = 5 },
    };
    var authority = try helpers.create(.{ .seed = 2, .side = 4 });
    defer authority.deinit();
    var setup_bytes: [2][1024]u8 = undefined;
    var setup: [2]pe.Envelope = undefined;
    for ([_]u64{ 70, 5 }, 0..) |cell, index| setup[index] = try pe.encodeEnvelope(.{
        .sequence = @fromBackingInt(index + 1),
        .provenance = .creator,
        .actor = null,
    }, .{ .author_settlement = .{
        .owner = pe.civilization_id,
        .anchor = .{ .planet = pe.planet_id, .local = cell },
        .population = 0,
        .food = 0,
        .material = 0,
        .economic_bypass = true,
    } }, &setup_bytes[index]);
    try std.testing.expectEqual(@as(u16, 2), (try helpers.tick(&authority, policy, &setup, 256)).accepted);
    var ship_bytes: [64][1024]u8 = undefined;
    var ships: [64]pe.Envelope = undefined;
    for (0..64) |index| ships[index] = try pe.encodeEnvelope(.{
        .sequence = @fromBackingInt(index + 3),
        .provenance = .creator,
        .actor = null,
    }, .{ .author_ship = .{
        .source = @fromBackingInt(1),
        .destination = @fromBackingInt(2),
        .resource = .food,
        .amount = 1,
        .path = &path,
        .cargo_source = .mint,
        .economic_bypass = true,
    } }, &ship_bytes[index]);
    try std.testing.expectEqual(@as(u16, 64), (try helpers.tick(&authority, policy, &ships, 256)).accepted);
    const full = authority.storage.rootMeta(authority.storage.active);
    try std.testing.expectEqual(@as(u16, 64), full.shipments);
    try std.testing.expectEqual(@as(u64, 65), full.next_shipment);
    try std.testing.expectEqual(@as(u32, 64), (try authority.settlement(@fromBackingInt(2))).incoming_food);
    var over_bytes: [1024]u8 = undefined;
    const extra = try pe.encodeEnvelope(.{ .sequence = @fromBackingInt(67), .provenance = .creator, .actor = null }, .{ .author_ship = .{
        .source = @fromBackingInt(1),
        .destination = @fromBackingInt(2),
        .resource = .food,
        .amount = 1,
        .path = &path,
        .cargo_source = .mint,
        .economic_bypass = true,
    } }, &over_bytes);
    try std.testing.expectEqual(@as(u16, 1), (try helpers.tick(&authority, policy, &.{extra}, 256)).rejected);
    try std.testing.expectEqual(pe.RejectReason.FreightCapacity, authority.storage.logicalReceipt(1).event.rejected.reason);
    try std.testing.expectEqual(full.next_shipment, authority.storage.rootMeta(authority.storage.active).next_shipment);
    try std.testing.expectEqual(@as(u32, 64), (try authority.settlement(@fromBackingInt(2))).incoming_food);
    try std.testing.expectEqual(@as(u32, 0), (try authority.settlement(@fromBackingInt(1))).food);

    var quantity = try helpers.create(.{ .seed = 2, .side = 4 });
    defer quantity.deinit();
    var quantity_bytes: [4][1024]u8 = undefined;
    const two = [_]pe.Envelope{
        try pe.encodeEnvelope(.{ .sequence = @fromBackingInt(1), .provenance = .creator, .actor = null }, .{ .author_settlement = .{
            .owner = pe.civilization_id,
            .anchor = .{ .planet = pe.planet_id, .local = 70 },
            .population = 0,
            .food = 0,
            .material = 0,
            .economic_bypass = true,
        } }, &quantity_bytes[0]),
        try pe.encodeEnvelope(.{ .sequence = @fromBackingInt(2), .provenance = .creator, .actor = null }, .{ .author_settlement = .{
            .owner = pe.civilization_id,
            .anchor = .{ .planet = pe.planet_id, .local = 5 },
            .population = 0,
            .food = 65_279,
            .material = 0,
            .economic_bypass = true,
        } }, &quantity_bytes[1]),
    };
    try std.testing.expectEqual(@as(u16, 2), (try helpers.tick(&quantity, policy, &two, 256)).accepted);
    const exact = try pe.encodeEnvelope(.{ .sequence = @fromBackingInt(3), .provenance = .creator, .actor = null }, .{ .author_ship = .{
        .source = @fromBackingInt(1),
        .destination = @fromBackingInt(2),
        .resource = .food,
        .amount = 256,
        .path = &path,
        .cargo_source = .mint,
        .economic_bypass = true,
    } }, &quantity_bytes[2]);
    try std.testing.expectEqual(@as(u16, 1), (try helpers.tick(&quantity, policy, &.{exact}, 256)).accepted);
    const at_limit = try quantity.settlement(@fromBackingInt(2));
    try std.testing.expectEqual(@as(u32, 65_279), at_limit.food);
    try std.testing.expectEqual(@as(u32, 256), at_limit.incoming_food);
    const before_meta = quantity.storage.rootMeta(quantity.storage.active);
    const too_many = try pe.encodeEnvelope(.{ .sequence = @fromBackingInt(4), .provenance = .creator, .actor = null }, .{ .author_ship = .{
        .source = @fromBackingInt(1),
        .destination = @fromBackingInt(2),
        .resource = .food,
        .amount = 257,
        .path = &path,
        .cargo_source = .mint,
        .economic_bypass = true,
    } }, &quantity_bytes[2]);
    const over_reserved = try pe.encodeEnvelope(.{ .sequence = @fromBackingInt(5), .provenance = .creator, .actor = null }, .{ .author_ship = .{
        .source = @fromBackingInt(1),
        .destination = @fromBackingInt(2),
        .resource = .food,
        .amount = 1,
        .path = &path,
        .cargo_source = .mint,
        .economic_bypass = true,
    } }, &quantity_bytes[3]);
    try std.testing.expectEqual(@as(u16, 2), (try helpers.tick(&quantity, policy, &.{ too_many, over_reserved }, 256)).rejected);
    try std.testing.expectEqual(pe.RejectReason.InvalidAmount, quantity.storage.logicalReceipt(1).event.rejected.reason);
    try std.testing.expectEqual(pe.RejectReason.InventoryCapacity, quantity.storage.logicalReceipt(2).event.rejected.reason);
    try std.testing.expectEqual(before_meta.next_shipment, quantity.storage.rootMeta(quantity.storage.active).next_shipment);
    const after_limit = try quantity.settlement(@fromBackingInt(2));
    try std.testing.expectEqual(at_limit.food, after_limit.food);
    try std.testing.expectEqual(at_limit.incoming_food, after_limit.incoming_food);
    try std.testing.expectEqual(at_limit.material, after_limit.material);
    try std.testing.expectEqual(at_limit.incoming_material, after_limit.incoming_material);
    try std.testing.expectEqual(@as(u32, 0), (try quantity.settlement(@fromBackingInt(1))).food);
}

test "pe/robustness/limits shipment arrival beyond tick horizon rejects atomically" {
    var authority = try helpers.create(.{ .seed = 2, .side = 4 });
    defer authority.deinit();
    const policy = try pe.SessionPolicy.init(.creator, null);
    var bytes: [2][1024]u8 = undefined;
    var batch: [2]pe.Envelope = undefined;
    for ([_]u64{ 70, 5 }, 0..) |cell, index| {
        batch[index] = try pe.encodeEnvelope(.{
            .sequence = @fromBackingInt(index + 1),
            .provenance = .creator,
            .actor = null,
        }, .{ .author_settlement = .{
            .owner = pe.civilization_id,
            .anchor = .{ .planet = pe.planet_id, .local = cell },
            .population = 0,
            .food = 0,
            .material = 0,
            .economic_bypass = true,
        } }, &bytes[index]);
    }
    try std.testing.expectEqual(@as(u16, 2), (try helpers.tick(&authority, policy, &batch, 256)).accepted);
    var meta = authority.storage.rootMeta(authority.storage.active);
    meta.tick = @fromBackingInt(999_999);
    authority.storage.writeRootMeta(authority.storage.active, meta);
    const path = [_]pe.CellId{
        .{ .planet = pe.planet_id, .local = 70 },
        .{ .planet = pe.planet_id, .local = 71 },
        .{ .planet = pe.planet_id, .local = 4 },
        .{ .planet = pe.planet_id, .local = 5 },
    };
    var ship_bytes: [1024]u8 = undefined;
    const ship = try pe.encodeEnvelope(.{
        .sequence = @fromBackingInt(3),
        .provenance = .creator,
        .actor = null,
    }, .{ .author_ship = .{
        .source = @fromBackingInt(1),
        .destination = @fromBackingInt(2),
        .resource = .food,
        .amount = 1,
        .path = &path,
        .cargo_source = .mint,
        .economic_bypass = true,
    } }, &ship_bytes);
    const result = try helpers.tick(&authority, policy, &.{ship}, 256);
    try std.testing.expectEqual(@as(u16, 0), result.accepted);
    try std.testing.expectEqual(@as(u16, 1), result.rejected);
    try std.testing.expectEqual(@as(u16, 0), authority.storage.rootMeta(authority.storage.active).shipments);
    try std.testing.expectEqual(@as(u64, 1), authority.storage.rootMeta(authority.storage.active).next_shipment);
    try std.testing.expectEqual(@as(u32, 0), (try authority.settlement(@fromBackingInt(2))).incoming_food);
}

test "pe/robustness/limits exhausted entity IDs do not wrap or consume state" {
    var authority = try helpers.create(.{ .seed = 2, .side = 4 });
    defer authority.deinit();
    const policy = try pe.SessionPolicy.init(.creator, null);
    var setup_bytes: [2][1024]u8 = undefined;
    var setup: [2]pe.Envelope = undefined;
    for ([_]u64{ 70, 5 }, 0..) |cell, index| {
        setup[index] = try pe.encodeEnvelope(.{
            .sequence = @fromBackingInt(index + 1),
            .provenance = .creator,
            .actor = null,
        }, .{ .author_settlement = .{
            .owner = pe.civilization_id,
            .anchor = .{ .planet = pe.planet_id, .local = cell },
            .population = 0,
            .food = 0,
            .material = 0,
            .economic_bypass = true,
        } }, &setup_bytes[index]);
    }
    try std.testing.expectEqual(@as(u16, 2), (try helpers.tick(&authority, policy, &setup, 256)).accepted);
    var meta = authority.storage.rootMeta(authority.storage.active);
    meta.next_settlement = std.math.maxInt(u64);
    meta.next_building = std.math.maxInt(u64);
    meta.next_shipment = std.math.maxInt(u64);
    authority.storage.writeRootMeta(authority.storage.active, meta);
    const before_a = try authority.settlement(@fromBackingInt(1));
    const before_b = try authority.settlement(@fromBackingInt(2));
    const path = [_]pe.CellId{
        .{ .planet = pe.planet_id, .local = 70 },
        .{ .planet = pe.planet_id, .local = 71 },
        .{ .planet = pe.planet_id, .local = 4 },
        .{ .planet = pe.planet_id, .local = 5 },
    };
    var bytes: [3][1024]u8 = undefined;
    const batch = [_]pe.Envelope{
        try pe.encodeEnvelope(.{ .sequence = @fromBackingInt(3), .provenance = .creator, .actor = null }, .{ .author_settlement = .{
            .owner = pe.civilization_id,
            .anchor = .{ .planet = pe.planet_id, .local = 69 },
            .population = 0,
            .food = 0,
            .material = 0,
            .economic_bypass = true,
        } }, &bytes[0]),
        try pe.encodeEnvelope(.{ .sequence = @fromBackingInt(4), .provenance = .creator, .actor = null }, .{ .author_build = .{
            .settlement = @fromBackingInt(1),
            .kind = .house,
            .anchor = .{ .planet = pe.planet_id, .local = 71 },
            .facing = null,
            .economic_bypass = true,
        } }, &bytes[1]),
        try pe.encodeEnvelope(.{ .sequence = @fromBackingInt(5), .provenance = .creator, .actor = null }, .{ .author_ship = .{
            .source = @fromBackingInt(1),
            .destination = @fromBackingInt(2),
            .resource = .food,
            .amount = 1,
            .path = &path,
            .cargo_source = .mint,
            .economic_bypass = true,
        } }, &bytes[2]),
    };
    const result = try helpers.tick(&authority, policy, &batch, 256);
    try std.testing.expectEqual(@as(u16, 0), result.accepted);
    try std.testing.expectEqual(@as(u16, 3), result.rejected);
    for (1..4) |i| try std.testing.expectEqual(pe.RejectReason.IdExhausted, authority.storage.logicalReceipt(@intCast(i)).event.rejected.reason);
    const after = authority.storage.rootMeta(authority.storage.active);
    try std.testing.expectEqual(meta.next_settlement, after.next_settlement);
    try std.testing.expectEqual(meta.next_building, after.next_building);
    try std.testing.expectEqual(meta.next_shipment, after.next_shipment);
    try std.testing.expectEqual(@as(u16, 2), after.settlements);
    try std.testing.expectEqual(@as(u16, 0), after.buildings);
    try std.testing.expectEqual(@as(u16, 0), after.shipments);
    try std.testing.expectEqualDeep(before_a.food, (try authority.settlement(@fromBackingInt(1))).food);
    try std.testing.expectEqualDeep(before_b.incoming_food, (try authority.settlement(@fromBackingInt(2))).incoming_food);
}
