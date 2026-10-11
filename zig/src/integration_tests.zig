const std = @import("std");
const m = @import("model.zig");
const scenario = @import("scenario.zig");
const Simulation = @import("simulation.zig").Simulation;

test "validated scenario supplies the same Classic state and exact replay identity" {
    const allocator = std.testing.allocator;
    const validated = try scenario.generatedValidated(m.DEFAULT_SEED, .{});
    var sim = try Simulation.fromScenario(allocator, &validated);
    defer sim.deinit();
    try std.testing.expectEqual(@as(u64, 0xD728015A9A3D8835), sim.scenario_fingerprint.?);
    try std.testing.expectEqual(@as(u64, 0x67D96E983D6C9330), sim.canonicalFingerprint());
    const initial = sim.replayIdentity().?;
    _ = try sim.previewCommand(.{ .launch = .{ .source = 0, .destination = 3 } });
    try std.testing.expectEqualDeep(initial, sim.replayIdentity().?);
    try sim.enqueueNext(0, .{ .launch = .{ .source = 0, .destination = 3 } });
    try std.testing.expectEqualDeep(initial, sim.replayIdentity().?);
    var report = try sim.step();
    defer report.deinit(allocator);
    const next = sim.replayIdentity().?;
    try std.testing.expectEqual(initial.scenario, next.scenario);
    try std.testing.expect(initial.state != next.state);
}

test "generation metadata is absent for invalid rules and controlled fixtures" {
    const allocator = std.testing.allocator;
    var invalid = Simulation.generated(allocator, m.DEFAULT_SEED, .{ .tick_hz = 59 });
    defer invalid.deinit();
    try std.testing.expect(invalid.replayIdentity() == null);
    var generated = Simulation.generated(allocator, m.DEFAULT_SEED, .{});
    defer generated.deinit();
    try std.testing.expect(generated.replayIdentity() != null);
    var fixture = Simulation.fromCampaign(allocator, try generated.state.clone(allocator), .{});
    defer fixture.deinit();
    try std.testing.expect(fixture.replayIdentity() == null);
    try std.testing.expectEqual(generated.canonicalFingerprint(), fixture.canonicalFingerprint());
}

test "scenario draft editing and rejected validation preserve resident authority" {
    const allocator = std.testing.allocator;
    var draft = try scenario.ScenarioDraft.factoryDefault(allocator);
    defer draft.deinit();
    const validated = try draft.validated();
    var sim = try Simulation.fromScenario(allocator, &validated);
    defer sim.deinit();
    const before = sim.replayIdentity().?;
    draft.worlds[0].name = "INVALID?";
    draft.worlds[0].energy = 1;
    try std.testing.expectError(error.Validation, draft.validated());
    try std.testing.expectEqualDeep(before, sim.replayIdentity().?);
    try std.testing.expectEqualStrings("ASTER VALE", sim.state.worlds[0].name.asSlice());
    try std.testing.expectEqual(@as(u64, 80_000), sim.state.worlds[0].energy);
}

test "scenario admission rejects stale identity and preserves resident authority" {
    const allocator = std.testing.allocator;
    const validated = try scenario.generatedValidated(m.DEFAULT_SEED, .{});
    var resident = try Simulation.fromScenario(allocator, &validated);
    defer resident.deinit();
    const before = resident.replayIdentity().?;
    var candidate = validated;
    candidate.worlds[0].energy += 1;
    try std.testing.expectError(error.StaleFingerprint, Simulation.fromScenario(allocator, &candidate));
    try std.testing.expectEqualDeep(before, resident.replayIdentity().?);
    try std.testing.expectEqual(@as(u64, 80_000), resident.state.worlds[0].energy);
    candidate.fingerprint = scenario.computeFingerprint(&candidate);
    var accepted = try Simulation.fromScenario(allocator, &candidate);
    defer accepted.deinit();
    try std.testing.expectEqual(candidate.fingerprint, accepted.replayIdentity().?.scenario);
    try std.testing.expectEqual(@as(u64, 80_001), accepted.state.worlds[0].energy);
    try std.testing.expectEqualDeep(before, resident.replayIdentity().?);
}

test "scenario admission rejects invalid state before installation" {
    const allocator = std.testing.allocator;
    const validated = try scenario.generatedValidated(m.DEFAULT_SEED, .{});
    var candidate = validated;
    candidate.rules.tick_hz = 59;
    try std.testing.expectError(error.Validation, Simulation.fromScenario(allocator, &candidate));
    candidate = validated;
    candidate.worlds[1].id = 0;
    try std.testing.expectError(error.InvalidInitialState, Simulation.fromScenario(allocator, &candidate));
    inline for (.{ "production_remainder", "regeneration_remainder" }) |field| {
        candidate = validated;
        @field(candidate.worlds[0], field) = 1;
        try std.testing.expectError(error.InvalidInitialState, Simulation.fromScenario(allocator, &candidate));
    }
    candidate = validated;
    candidate.worlds[0].name.len = 255;
    try std.testing.expectError(error.Validation, Simulation.fromScenario(allocator, &candidate));
    candidate = validated;
    candidate.warnings.len = std.math.maxInt(usize);
    var accepted = try Simulation.fromScenario(allocator, &candidate);
    defer accepted.deinit();
    try std.testing.expectEqual(@as(u64, 0xD728015A9A3D8835), accepted.replayIdentity().?.scenario);
}
