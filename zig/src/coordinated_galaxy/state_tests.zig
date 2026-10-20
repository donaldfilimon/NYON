const std = @import("std");
const state = @import("state.zig");
const t = @import("types.zig");
const helpers = @import("planet_tests.zig");

test "cg/state/reservation-and-setup-unwind" {
    try std.testing.expectEqual(@as(usize, 12_582_912), @sizeOf(state.CoordinatorReservation));
    var storage = try state.Storage.init(std.testing.allocator, .{ .seed = 2, .planet_count = 4 });
    defer storage.deinit();
    try std.testing.expectEqual(@as(usize, 47_153_152), storage.requestedBytes());
    var zero: t.WorkBudget = .{ .remaining = 0 };
    try std.testing.expectEqual(.pending, try storage.advanceSetup(&zero));
    try std.testing.expectEqual(@as(u8, 0), storage.setup_index);
    for (storage.metadata().builders) |builder| try std.testing.expectEqual(@as(u64, 0), builder.?.control.?.builder.storage.?.work_units);
    for (0..5) |index| {
        var failing = std.testing.FailingAllocator.init(std.testing.allocator, .{ .fail_index = index });
        try std.testing.expectError(error.OutOfMemory, state.Storage.init(failing.allocator(), .{ .seed = 2, .planet_count = 4 }));
        try std.testing.expectEqual(failing.allocated_bytes, failing.freed_bytes);
        try std.testing.expectEqual(index, failing.deallocations);
    }
    storage.deinit();
    try std.testing.expectEqual(@as(usize, 0), storage.requestedBytes());
}

test "cg/state/independent-worlds local-one qualified-identities and zero-poll-allocation" {
    var failing = std.testing.FailingAllocator.init(std.testing.allocator, .{ .fail_index = 3 });
    var storage = try state.Storage.init(failing.allocator(), .{ .seed = 2, .planets = .{ .{ .side = 4 }, .{ .side = 4 }, .{}, .{} } });
    defer storage.deinit();
    var budget: t.WorkBudget = .{ .remaining = 7 };
    while (try storage.advanceSetup(&budget) == .pending) budget.remaining = 7;
    const one = &storage.metadata().planets[0].?;
    const two = &storage.metadata().planets[1].?;
    try std.testing.expect(one.storage().reservation != two.storage().reservation);
    try helpers.demo(one, 1, 1);
    try helpers.demo(two, 2, 2);
    for ([_]t.Phase{ .prepare, .commands, .arrivals, .construction, .production, .population, .local_finalize }) |phase| {
        try helpers.advance(one, phase, 7);
        try helpers.advance(two, phase, 7);
    }
    const first = try one.settlement((try helpers.freeze(one, 7)).root, @fromBackingInt(1));
    const second = try two.settlement((try helpers.freeze(two, 7)).root, @fromBackingInt(1));
    try std.testing.expect(first.id.planet != second.id.planet);
    try std.testing.expectEqual(first.id.local, second.id.local);
    try std.testing.expect(!failing.has_induced_failure);
    try std.testing.expectEqual(@as(usize, 3), failing.alloc_index);
}

test "cg/state/partial-setup ledger destruction frees exactly once" {
    for ([_]u16{ 1, 7, 256, 4096 }) |units| {
        var failing = std.testing.FailingAllocator.init(std.testing.allocator, .{ .fail_index = 5 });
        var storage = try state.Storage.init(failing.allocator(), .{ .seed = 2, .planet_count = 4 });
        var budget: t.WorkBudget = .{ .remaining = units };
        _ = try storage.advanceSetup(&budget);
        storage.deinit();
        storage.deinit();
        try std.testing.expectEqual(failing.allocated_bytes, failing.freed_bytes);
        try std.testing.expectEqual(@as(usize, 5), failing.deallocations);
    }
}
