//! Compile-only freestanding probe. The caller supplies all allocator memory.
const std = @import("std");
const pe = @import("planet_economy/root.zig");

export fn planetEconomyCompileProbe(buffer: [*]u8, len: usize) u64 {
    var fixed = std.heap.FixedBufferAllocator.init(buffer[0..len]);
    const allocator = fixed.allocator();
    if (len & 1 == 1) {
        var canceled_builder = pe.beginCreate(allocator, .{ .seed = 2, .side = 4 }) catch return 0;
        _ = canceled_builder.stats();
        pe.cancelCreate(&canceled_builder);
        while (true) {
            var budget = pe.WorkBudget{ .remaining = 256 };
            switch (pe.pollCreate(&canceled_builder, &budget)) {
                .pending => {},
                .canceled, .idle => break,
                else => return 0,
            }
        }
        canceled_builder.deinit();
    }
    var source = create(allocator) catch return 0;
    defer source.deinit();
    var target = create(allocator) catch return 0;
    defer target.deinit();
    _ = source.stats();
    _ = source.cell(.{ .planet = pe.planet_id, .local = 70 }) catch return 0;
    var bytes: [1024]u8 = undefined;
    const envelope = pe.encodeEnvelope(.{ .sequence = @fromBackingInt(1), .provenance = .creator, .actor = null }, .{ .author_settlement = .{
        .owner = pe.civilization_id,
        .anchor = .{ .planet = pe.planet_id, .local = 70 },
        .population = 4,
        .food = 12,
        .material = 20,
        .economic_bypass = true,
    } }, &bytes) catch return 0;
    const policy = pe.SessionPolicy.init(.creator, null) catch return 0;
    const id = source.beginTick(source.summary().revision, policy, &.{envelope}) catch return 0;
    while (true) {
        var budget = pe.WorkBudget{ .remaining = 256 };
        switch (source.poll(id, &budget)) {
            .pending, .published => {},
            .idle => break,
            else => return 0,
        }
    }
    _ = source.settlement(@fromBackingInt(1)) catch return 0;
    var rows: [2]pe.ReceiptView = undefined;
    _ = source.receipts(source.summary().revision, 0, &rows) catch return 0;
    var replay = pe.replay.Cursor.init(&source);
    while (true) {
        var budget = pe.WorkBudget{ .remaining = 256 };
        if ((replay.advance(&target, &budget) catch return 0) == .done) break;
    }
    const canceled = target.beginTick(target.summary().revision, policy, &.{}) catch return 0;
    _ = target.cancel(canceled);
    while (true) {
        var budget = pe.WorkBudget{ .remaining = 256 };
        switch (target.poll(canceled, &budget)) {
            .pending, .canceled => {},
            .idle => break,
            else => return 0,
        }
    }
    const owned = target.beginOwned(target.summary().revision, policy, &.{}) catch return 0;
    _ = target.cancel(owned);
    while (true) {
        var budget = pe.WorkBudget{ .remaining = 256 };
        switch (target.poll(owned, &budget)) {
            .pending, .canceled => {},
            .idle => break,
            else => return 0,
        }
    }
    if (len & 2 == 2) {
        var story = pe.stories.Runner.init(allocator, .combined) catch return 0;
        while (true) {
            var budget = pe.WorkBudget{ .remaining = 256 };
            switch (story.poll(&budget) catch return 0) {
                .pending => {},
                .published => |value| {
                    _ = story.receipts(value.summary.revision, 0, &rows) catch return 0;
                    story.acknowledge(value.summary.revision) catch return 0;
                },
                .awaiting_observer => return 0,
                .complete => break,
            }
        }
        story.deinit();
        _ = pe.stories.run(allocator, .combined, 256) catch return 0;
    }
    return @backingInt(target.summary().revision);
}

fn create(allocator: std.mem.Allocator) pe.Failure!pe.Authority {
    var builder = try pe.beginCreate(allocator, .{ .seed = 2, .side = 4 });
    while (true) {
        var budget = pe.WorkBudget{ .remaining = 256 };
        switch (pe.pollCreate(&builder, &budget)) {
            .pending => {},
            .ready => return pe.takeAuthority(&builder),
            else => return error.InvariantViolation,
        }
    }
}
