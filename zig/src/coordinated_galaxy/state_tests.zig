const std = @import("std");
const state = @import("state.zig");
test "cg/state/reservation-and-setup-unwind" {
    try std.testing.expectEqual(@as(usize, 12582912), @sizeOf(state.CoordinatorReservation));
    var storage = try state.Storage.init(std.testing.allocator, .{ .seed = 2, .planet_count = 4 });
    defer storage.deinit();
    try std.testing.expectEqual(@as(usize, 47153152), storage.requestedBytes());
}
