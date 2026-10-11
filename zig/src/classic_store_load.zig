//! Validated Classic candidate loading; editor draft policy is separate.
const std = @import("std");
const classic_store = @import("classic_store.zig");
const scenario = @import("scenario.zig");
const simulation = @import("simulation.zig");

/// Detached checked candidate. Move its simulation only after successful load;
/// deinit owns returned store diagnostics and uninstalled simulation allocations.
pub const Outcome = union(enum) {
    absent,
    candidate: struct { scenario: scenario.ScenarioV1, simulation: simulation.Simulation },
    failure: union(enum) {
        store: classic_store.StoreFailure,
        codec: scenario.CodecFailure,
        admission: scenario.AdmissionError,
    },

    pub fn deinit(self: *Outcome, allocator: std.mem.Allocator) void {
        switch (self.*) {
            .candidate => |*value| value.simulation.deinit(),
            .failure => |*failure| switch (failure.*) {
                .store => |*value| value.deinit(allocator),
                else => {},
            },
            .absent => {},
        }
        self.* = undefined;
    }
};

/// The adapter is borrowed at a stable address for this call. No live state is
/// touched, and OutOfMemory stays distinct from recoverable record failures.
pub fn load(allocator: std.mem.Allocator, store: classic_store.RawStore) std.mem.Allocator.Error!Outcome {
    if (!store.matchesRecord(.scenario)) return .{ .failure = .{ .store = .{ .record = .scenario, .kind = .io, .cause = error.RecordMismatch } } };
    const loaded = try store.load(allocator);
    switch (loaded) {
        .absent => return .absent,
        .failure => |failure| return .{ .failure = .{ .store = failure } },
        .payload => |payload| {
            defer allocator.free(payload);
            const decoded = switch (try scenario.decodeDetailed(allocator, payload)) {
                .scenario => |value| value,
                .failure => |failure| return .{ .failure = .{ .codec = failure } },
            };
            const candidate = simulation.Simulation.fromScenario(allocator, &decoded) catch |failure|
                return .{ .failure = .{ .admission = failure } };
            return .{ .candidate = .{ .scenario = decoded, .simulation = candidate } };
        },
    }
}
