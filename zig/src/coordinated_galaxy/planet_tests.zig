const std = @import("std");
const planet = @import("planet.zig");
test "cg/planet/staged-no-publication" {
    var builder = try planet.createPlanet(std.testing.allocator, .{ .side = 4 }, @fromBackingInt(2), 2, 2);
    defer builder.deinit();
    var zero = try @import("types.zig").WorkBudget.init(0);
    try std.testing.expectEqual(.pending, try planet.advancePlanetCreate(&builder, &zero));
}
