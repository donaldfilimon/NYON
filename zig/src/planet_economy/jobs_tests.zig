const std = @import("std");
const pe = @import("root.zig");

pub fn create(config: pe.Config) !pe.Authority {
    var builder = try pe.beginCreate(std.testing.allocator, config);
    while (true) {
        var budget = pe.WorkBudget{ .remaining = 4096 };
        switch (pe.pollCreate(&builder, &budget)) {
            .ready => return pe.takeAuthority(&builder),
            .pending => {},
            else => return error.UnexpectedCreate,
        }
    }
}

test "pe/jobs/genesis ready includes encoded and hashed genesis" {
    var authority = try create(.{ .seed = 2, .side = 4 });
    defer authority.deinit();
    try std.testing.expectEqual(@as(u16, 1), authority.stats().history_used);
    try std.testing.expectEqualSlices(u8, &pe.wire_vectors.G0.chain, &authority.summary().chain_head);
}

pub fn author(out: []u8, seq: u64) !pe.Envelope {
    return pe.encodeEnvelope(.{ .sequence = @fromBackingInt(seq), .provenance = .creator, .actor = null }, .{ .author_settlement = .{ .owner = pe.civilization_id, .anchor = .{ .planet = pe.planet_id, .local = 70 }, .population = 4, .food = 12, .material = 20, .economic_bypass = true } }, out);
}

pub fn tick(authority: *pe.Authority, policy: pe.SessionPolicy, batch: []const pe.Envelope, units: u16) !pe.TickSummary {
    const id = try authority.beginTick(authority.summary().revision, policy, batch);
    var result: ?pe.TickSummary = null;
    while (true) {
        var budget = pe.WorkBudget{ .remaining = units };
        switch (authority.poll(id, &budget)) {
            .published => |value| result = value,
            .pending => {},
            .idle => return result orelse error.MissingPublication,
            .failed, .invalid_request => |err| return err,
            .canceled => return error.UnexpectedCancel,
        }
    }
}

test "pe/jobs/copied-input survives caller overwrite and publication stays atomic" {
    var authority = try create(.{ .seed = 2, .side = 4 });
    defer authority.deinit();
    var bytes: [1024]u8 = undefined;
    var envelopes = [_]pe.Envelope{try author(&bytes, 1)};
    const policy = try pe.SessionPolicy.init(.creator, null);
    const id = try authority.beginTick(@fromBackingInt(0), policy, &envelopes);
    @memset(&bytes, 0xff);
    envelopes[0] = .{ .bytes = &.{} };
    var rows: [1]pe.ReceiptView = undefined;
    while (true) {
        try std.testing.expectError(error.NotFound, authority.settlement(@fromBackingInt(1)));
        try std.testing.expectEqual(@as(u16, 1), authority.stats().history_used);
        try std.testing.expectEqual(@as(u16, 1), (try authority.receipts(@fromBackingInt(0), 0, &rows)).written);
        var budget = pe.WorkBudget{ .remaining = 1 };
        switch (authority.poll(id, &budget)) {
            .published => |value| {
                try std.testing.expectEqual(@as(u16, 1), value.accepted);
                break;
            },
            .pending => {},
            else => return error.UnexpectedProgress,
        }
    }
    try std.testing.expectEqual(@as(u32, 8), (try authority.settlement(@fromBackingInt(1))).food);
    try std.testing.expectError(error.StaleView, authority.receipts(@fromBackingInt(0), 0, &rows));
    try std.testing.expectEqual(pe.CancelResult.already_published, authority.cancel(id));
    try std.testing.expectError(error.Busy, authority.beginTick(@fromBackingInt(1), policy, &.{}));
    while (true) {
        var budget = pe.WorkBudget{ .remaining = 7 };
        switch (authority.poll(id, &budget)) {
            .idle => break,
            .pending => {},
            else => return error.DuplicateTerminal,
        }
    }
    const next = try authority.beginTick(@fromBackingInt(1), policy, &.{});
    var budget = pe.WorkBudget{ .remaining = 1 };
    try std.testing.expectEqual(error.StaleJob, authority.poll(id, &budget).invalid_request);
    try std.testing.expectEqual(@as(u16, 1), budget.remaining);
    try std.testing.expectEqual(pe.CancelResult.stale_job, authority.cancel(id));
    _ = authority.cancel(next);
    while (true) {
        budget.remaining = 4096;
        if (authority.poll(next, &budget) == .idle) break;
    }
}

test "pe/jobs/publication invalid budgets are nonterminal and cancellation drains" {
    var authority = try create(.{ .seed = 2, .side = 4 });
    defer authority.deinit();
    const prior = authority.summary();
    const policy = try pe.SessionPolicy.init(.creator, null);
    const id = try authority.beginTick(prior.revision, policy, &.{});
    var budget = pe.WorkBudget{ .remaining = 4097 };
    try std.testing.expectEqual(error.InvalidBudget, authority.poll(id, &budget).invalid_request);
    budget.remaining = 0;
    const work = authority.stats().work_units;
    try std.testing.expect(authority.poll(id, &budget) == .pending);
    try std.testing.expectEqual(work, authority.stats().work_units);
    _ = authority.cancel(id);
    var canceled: u8 = 0;
    while (true) {
        budget.remaining = 1;
        switch (authority.poll(id, &budget)) {
            .canceled => canceled += 1,
            .idle => break,
            .pending => {},
            else => return error.UnexpectedProgress,
        }
    }
    try std.testing.expectEqual(@as(u8, 1), canceled);
    try std.testing.expectEqualDeep(prior, authority.summary());
    _ = try tick(&authority, policy, &.{}, 7);
    try std.testing.expectEqual(@as(u16, 3), authority.stats().history_used);
}

test "pe/jobs/create invalid budget resumes and canceled terminal is delivered once" {
    var builder = try pe.beginCreate(std.testing.allocator, .{ .seed = 2, .side = 4 });
    defer builder.deinit();
    var budget = pe.WorkBudget{ .remaining = 4097 };
    try std.testing.expectEqual(error.InvalidBudget, pe.pollCreate(&builder, &budget).invalid_request);
    try std.testing.expectEqual(@as(u64, 0), builder.stats().work_units);
    budget.remaining = 1;
    try std.testing.expect(pe.pollCreate(&builder, &budget) == .pending);
    pe.cancelCreate(&builder);
    while (true) {
        budget.remaining = 1;
        if (pe.pollCreate(&builder, &budget) == .canceled) break;
    }
    try std.testing.expect(pe.pollCreate(&builder, &budget) == .idle);
}

test "pe/jobs/trusted-policy malformed policy fails once and preserves genesis" {
    var authority = try create(.{ .seed = 2, .side = 4 });
    defer authority.deinit();
    const before = authority.summary();
    const id = try authority.beginTick(before.revision, .{ .role = .ruler, .civilization = null }, &.{});
    var failures: u8 = 0;
    while (true) {
        var budget = pe.WorkBudget{ .remaining = 7 };
        switch (authority.poll(id, &budget)) {
            .pending => {},
            .failed => |err| {
                try std.testing.expectEqual(error.InvalidPolicy, err);
                failures += 1;
            },
            .idle => break,
            else => return error.UnexpectedProgress,
        }
    }
    try std.testing.expectEqual(@as(u8, 1), failures);
    try std.testing.expectEqualDeep(before, authority.summary());
    try std.testing.expectEqual(@as(u16, 1), authority.stats().history_used);
    var rows: [65]pe.ReceiptView = undefined;
    try std.testing.expectError(error.CapacityUnsupported, authority.receipts(before.revision, 0, &rows));
    try std.testing.expectEqualDeep(pe.ReceiptPage{ .written = 0, .next = 0 }, try authority.receipts(before.revision, 0, rows[0..0]));
    try std.testing.expectError(error.NotFound, authority.receipts(before.revision, 2, rows[0..1]));
}
