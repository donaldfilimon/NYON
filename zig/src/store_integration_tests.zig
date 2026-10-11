//! Tests of the real validated candidate boundary and authority isolation.
const std = @import("std");
const store = @import("classic_store.zig");
const loader = @import("classic_store_load.zig");
const scenario = @import("scenario.zig");
const model = @import("model.zig");
const Simulation = @import("simulation.zig").Simulation;
const t = std.testing;
const a = t.allocator;

fn resident() !Simulation {
    var sim = Simulation.generated(a, model.DEFAULT_SEED, .{});
    errdefer sim.deinit();
    try sim.enqueueNext(42, .{ .launch = .{ .source = 0, .destination = 1 } });
    return sim;
}
fn unchanged(sim: *const Simulation, identity: scenario.ReplayIdentity, pending: model.CommandEnvelope) !void {
    try t.expectEqualDeep(identity, sim.replayIdentity().?);
    try t.expectEqual(@as(usize, 1), sim.pending.items.len);
    try t.expectEqualDeep(pending, sim.pending.items[0]);
}

test "store integration native save fresh readback creates checked candidate without resident mutation" {
    var random: [16]u8 = undefined;
    t.io.random(&random);
    const profile = try std.fmt.allocPrint(a, "/private/tmp/nyon-task2b-{s}", .{std.fmt.bytesToHex(random, .lower)});
    defer a.free(profile);
    defer std.Io.Dir.cwd().deleteTree(t.io, profile) catch @panic("owned profile cleanup failed");
    var sim = try resident();
    defer sim.deinit();
    const identity = sim.replayIdentity().?;
    const pending = sim.pending.items[0];
    const value = try scenario.generatedValidated(1, .{});
    const bytes = try scenario.encode(a, &value);
    defer a.free(bytes);
    var native = try store.NativeStore.fromProfile(a, t.io, .scenario, .macos, profile);
    defer native.deinit();
    var saved = try native.raw().save(a, bytes);
    defer saved.deinit(a);
    try t.expect(saved == .saved);
    var reopened = try store.NativeStore.fromProfile(a, t.io, .scenario, .macos, profile);
    defer reopened.deinit();
    var loaded = try loader.load(a, reopened.raw());
    defer loaded.deinit(a);
    try t.expect(loaded == .candidate);
    var expected = try Simulation.fromScenario(a, &value);
    defer expected.deinit();
    try t.expectEqualDeep(expected.replayIdentity(), loaded.candidate.simulation.replayIdentity());
    try t.expectEqual(value.fingerprint, loaded.candidate.scenario.fingerprint);
    try t.expectEqual(@as(usize, 0), loaded.candidate.simulation.pending.items.len);
    try unchanged(&sim, identity, pending);
    // Explicitly install only this successful checked candidate.
    sim.deinit();
    sim = loaded.candidate.simulation;
    loaded = .absent; // ownership moved; the outcome must not deinit it again
    try t.expectEqualDeep(expected.replayIdentity(), sim.replayIdentity());
}

test "store integration absent store codec and semantic load failures preserve resident and queue" {
    var sim = try resident();
    defer sim.deinit();
    const identity = sim.replayIdentity().?;
    const pending = sim.pending.items[0];
    var memory = store.MemoryStore.init(a, .scenario);
    defer memory.deinit();
    var absent = try loader.load(a, memory.raw());
    defer absent.deinit(a);
    try t.expect(absent == .absent);
    try memory.denyLoad("integration read denial");
    var denied = try loader.load(a, memory.raw());
    defer denied.deinit(a);
    try t.expect(denied == .failure and denied.failure == .store);
    try unchanged(&sim, identity, pending);
    const valid = try scenario.generatedValidated(model.DEFAULT_SEED, .{});
    const encoded = try scenario.encode(a, &valid);
    defer a.free(encoded);
    const invalid = try std.mem.replaceOwned(u8, a, encoded, "\"tick_hz\":60", "\"tick_hz\":0");
    defer a.free(invalid);
    for ([_][]const u8{ "", "{}", invalid }) |bytes| {
        var primary = try store.MemoryStore.withSlot(a, .scenario, bytes);
        defer primary.deinit();
        var loaded = try loader.load(a, primary.raw());
        defer loaded.deinit(a);
        try t.expect(loaded == .failure and loaded.failure == .codec);
        if (std.mem.eql(u8, bytes, invalid)) {
            try t.expect(loaded.failure.codec == .validation);
            try t.expect(loaded.failure.codec.validation.errorCount() > 0);
        }
        try t.expectEqualStrings(bytes, primary.slot.?);
        try unchanged(&sim, identity, pending);
    }
}

test "store integration checked admission rejects forged candidates before any resident replacement" {
    var sim = try resident();
    defer sim.deinit();
    const identity = sim.replayIdentity().?;
    const pending = sim.pending.items[0];
    const valid = try scenario.generatedValidated(1, .{});
    var stale = valid;
    stale.fingerprint ^= 1;
    try t.expectError(error.StaleFingerprint, Simulation.fromScenario(a, &stale));
    var invalid_initial = valid;
    invalid_initial.worlds[0].production_remainder = 1;
    try t.expectError(error.InvalidInitialState, Simulation.fromScenario(a, &invalid_initial));
    var invalid_semantics = valid;
    invalid_semantics.rules.tick_hz = 0;
    try t.expectError(error.Validation, Simulation.fromScenario(a, &invalid_semantics));
    try unchanged(&sim, identity, pending);
}

test "store integration preference changes and original failures leave authority and durable prior value" {
    var sim = try resident();
    defer sim.deinit();
    const identity = sim.replayIdentity().?;
    const pending = sim.pending.items[0];
    var memory = store.MemoryStore.init(a, .preferences);
    defer memory.deinit();
    const requested = @import("preferences.zig").UserPreferencesV1{ .ui_scale = .percent130, .motion = .reduced, .high_contrast = true };
    var saved = try store.saveOrDefault(a, memory.raw(), requested);
    defer saved.deinit(a);
    try t.expect(saved.failure == null);
    const prior = try a.dupe(u8, memory.slot.?);
    defer a.free(prior);
    var loaded = try store.loadOrDefault(a, memory.raw());
    defer loaded.deinit(a);
    try t.expectEqualDeep(requested, loaded.preferences);
    try memory.failNextSave("preference persist denial");
    var failed = try store.saveOrDefault(a, memory.raw(), .{});
    defer failed.deinit(a);
    try t.expect(failed.failure.? == .store);
    try t.expectEqualStrings("preference persist denial", failed.failure.?.store.message.?);
    try t.expectEqualStrings(prior, memory.slot.?);
    try memory.denyLoad("preference load denial");
    var denied = try store.loadOrDefault(a, memory.raw());
    defer denied.deinit(a);
    try t.expect(denied.failure.? == .store);
    try unchanged(&sim, identity, pending);
    var malformed = try store.MemoryStore.withSlot(a, .preferences, "{}");
    defer malformed.deinit();
    var defaults = try store.loadOrDefault(a, malformed.raw());
    defer defaults.deinit(a);
    try t.expect(defaults.failure.? == .codec);
    try t.expectEqualStrings("{}", malformed.slot.?);
    try unchanged(&sim, identity, pending);
}

test "store integration candidate allocations propagate OutOfMemory and release owned raw payloads" {
    const valid = try scenario.generatedValidated(model.DEFAULT_SEED, .{});
    const bytes = try scenario.encode(a, &valid);
    defer a.free(bytes);
    var memory = try store.MemoryStore.withSlot(a, .scenario, bytes);
    defer memory.deinit();
    // Fail both the returned raw buffer and the first JSON parsing allocation.
    for ([_]usize{ 0, 1 }) |index| {
        var failing = t.FailingAllocator.init(a, .{ .fail_index = index });
        try t.expectError(error.OutOfMemory, loader.load(failing.allocator(), memory.raw()));
        try t.expectEqual(failing.allocated_bytes, failing.freed_bytes);
    }
    try t.expectEqualStrings(bytes, memory.slot.?);
    var wrong_namespace = store.MemoryStore.init(a, .preferences);
    defer wrong_namespace.deinit();
    var rejected = try loader.load(a, wrong_namespace.raw());
    defer rejected.deinit(a);
    try t.expect(rejected.failure.store.cause.? == error.RecordMismatch);
}
