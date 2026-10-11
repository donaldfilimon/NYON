//! Player command admission; economic checks still follow role checks.
const std = @import("std");
pub const Role = enum(u8) { creator = 0, ruler = 1, combined = 2 };
pub const Provenance = enum(u8) { creator = 0, ruler = 1, autonomous = 2 };
pub const CivilizationId = enum(u32) { _ };
pub const SessionPolicy = struct {
    role: Role,
    civilization: ?CivilizationId,
    pub fn init(role: Role, civilization: ?CivilizationId) !SessionPolicy {
        if (role != .creator and civilization == null) return error.CivilizationRequired;
        return .{ .role = role, .civilization = civilization };
    }
    pub fn admit(self: SessionPolicy, provenance: Provenance, actor: ?CivilizationId) !void {
        switch (provenance) {
            .creator => {
                if (self.role == .ruler) return error.CreatorForbidden;
                if (actor != null) return error.InvalidActor;
            },
            .ruler => {
                if (self.role == .creator) return error.RulerForbidden;
                const selected = self.civilization orelse return error.CivilizationRequired;
                if (actor == null or actor.? != selected) return error.WrongCivilization;
            },
            .autonomous => return error.InternalCommandOnly,
        }
    }
};
test "roles reject authority escalation and preserve explicit provenance" {
    const faction: CivilizationId = @fromBackingInt(@intCast(7));
    const other: CivilizationId = @fromBackingInt(@intCast(8));
    const creator = try SessionPolicy.init(.creator, null);
    try creator.admit(.creator, null);
    try std.testing.expectError(error.RulerForbidden, creator.admit(.ruler, faction));
    const ruler = try SessionPolicy.init(.ruler, faction);
    try ruler.admit(.ruler, faction);
    try std.testing.expectError(error.CreatorForbidden, ruler.admit(.creator, null));
    const combined = try SessionPolicy.init(.combined, faction);
    try combined.admit(.creator, null);
    try combined.admit(.ruler, faction);
    for ([_]SessionPolicy{ creator, ruler, combined }) |policy|
        try std.testing.expectError(error.InternalCommandOnly, policy.admit(.autonomous, faction));
    try std.testing.expectError(error.WrongCivilization, combined.admit(.ruler, other));
    try std.testing.expectError(error.WrongCivilization, combined.admit(.ruler, null));
    try std.testing.expectError(error.InvalidActor, combined.admit(.creator, faction));
    try std.testing.expectError(error.CivilizationRequired, SessionPolicy.init(.ruler, null));
    try std.testing.expectError(error.CivilizationRequired, SessionPolicy.init(.combined, null));
}
