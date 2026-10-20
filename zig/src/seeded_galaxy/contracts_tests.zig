const std = @import("std");
const sg = @import("root.zig");
test "config precedence and reservation" {
    try std.testing.expectError(error.InvalidVersion, sg.validateConfig(.{ .version = 2, .systems = 0, .planets = 0 }));
    try std.testing.expectError(error.CapacityUnsupported, sg.validateConfig(.{ .systems = 65, .planets = 1 }));
    try std.testing.expectError(error.InvalidConfig, sg.validateConfig(.{ .systems = 2, .planets = 1, .civilizations = 1 }));
    try std.testing.expect(sg.storage.candidateReservation() <= 131_072);
}

test "all capacity endpoints and relationship precedence" {
    try sg.validateConfig(.{ .systems = 1, .planets = 1, .civilizations = 1, .side = 4 });
    try sg.validateConfig(.{ .systems = 64, .planets = 512, .civilizations = 16, .side = 32 });
    for ([_]sg.Config{
        .{ .systems = 0 },                .{ .planets = 0 },   .{ .civilizations = 0 },
        .{ .systems = 65, .planets = 1 }, .{ .planets = 513 }, .{ .civilizations = 17 },
        .{ .side = 3 },                   .{ .side = 33 },
    }) |bad| try std.testing.expectError(error.CapacityUnsupported, sg.validateConfig(bad));
    try std.testing.expectError(error.InvalidConfig, sg.validateConfig(.{ .systems = 3, .planets = 2 }));
    try std.testing.expectError(error.InvalidConfig, sg.validateConfig(.{ .systems = 1, .planets = 2, .civilizations = 3 }));
    try std.testing.expectEqualStrings("nyon.seeded-galaxy.v0", sg.rules_identity);
}

test "measured record and allocation layout includes controls and prior resident" {
    try std.testing.expectEqual(@as(usize, 24), @sizeOf(sg.Config));
    try std.testing.expectEqual(@as(usize, 16), @sizeOf(sg.System));
    try std.testing.expectEqual(@as(usize, 24), @sizeOf(sg.Planet));
    try std.testing.expectEqual(@as(usize, 12), @sizeOf(sg.Civilization));
    try std.testing.expectEqual(@as(usize, 256), @sizeOf(sg.WireRecord));
    try std.testing.expectEqual(@as(usize, 29_952), @sizeOf(sg.storage.Output));
    try std.testing.expectEqual(@as(usize, 16_592), @sizeOf(sg.storage.Scratch));
    try std.testing.expectEqual(@as(usize, 8), @alignOf(sg.storage.Output));
    try std.testing.expectEqual(@as(usize, 16), @alignOf(sg.storage.Scratch));
    try std.testing.expectEqual(@as(usize, 47_590), sg.storage.candidateReservation());
    try std.testing.expectEqual(@as(usize, 30_983), sg.storage.detachedReservation());
    try std.testing.expectEqual(@as(usize, 78_573), sg.storage.replacementPeakReservation());
}
