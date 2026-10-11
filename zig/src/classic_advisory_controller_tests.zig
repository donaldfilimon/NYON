const std = @import("std");
const c = @import("classic_advisory_controller.zig");
const t = @import("classic_advisory_types.zig");
const m = @import("model.zig");
const g = @import("generator.zig");
fn obs(tick: u64) t.Observation {
    const campaign = g.generate(m.DEFAULT_SEED, .{});
    return .{ .next_tick = tick, .worlds = campaign.worlds, .fleets = &.{}, .active_hazard = campaign.active_hazard };
}
fn request(owner: *c.Controller, tick: u64, fp: u64, trigger: c.Trigger) !c.RequestOutcome {
    const result = owner.requestIfDue(obs(tick), fp, trigger);
    try std.testing.expect(result != null);
    return result.?;
}
test "controller cadence metadata wrapping and unconditional triggers" {
    var owner = c.Controller.init(7, false);
    try std.testing.expect(owner.published() == null);
    const initial = try request(&owner, 0, 41, .Initialization);
    try std.testing.expectEqual(c.Metadata{ .device_epoch = 7, .request_id = 0, .model_version = 1, .source_tick = 0, .source_fingerprint = 41 }, initial.snapshot.metadata);
    try std.testing.expectEqual(c.Backend.Cpu, initial.snapshot.backend);
    const before = owner;
    try std.testing.expect(owner.requestIfDue(obs(0), 42, .Tick) == null);
    try std.testing.expect(owner.requestIfDue(obs(29), 43, .Tick) == null);
    try std.testing.expectEqualDeep(before, owner);
    _ = try request(&owner, 30, 44, .Tick);
    try std.testing.expectEqual(@as(?u64, 30), owner.last_cadence_tick);
    const cadence = owner;
    try std.testing.expect(owner.requestIfDue(obs(30), 45, .Tick) == null);
    try std.testing.expectEqualDeep(cadence, owner);
    _ = try request(&owner, 60, 46, .MaterialEvent);
    try std.testing.expect(owner.requestIfDue(obs(60), 47, .Tick) == null);
    owner.next_request_id = std.math.maxInt(u64);
    const wrap = try request(&owner, 60, 48, .Restart);
    try std.testing.expectEqual(std.math.maxInt(u64), wrap.snapshot.metadata.request_id);
    owner.setDeviceEpoch(8, true);
    const next = try request(&owner, 61, 49, .Restart);
    try std.testing.expectEqual(@as(u64, 0), next.snapshot.metadata.request_id);
    try std.testing.expectEqual(@as(?u64, 30), owner.last_cadence_tick);
}
test "controller bounded queue metadata mismatches stale promotion acceptance" {
    var owner = c.Controller.init(2, true);
    const first = (try request(&owner, 0, 10, .Initialization)).gpu_request.?;
    _ = try request(&owner, 1, 11, .MaterialEvent);
    const newest = try request(&owner, 2, 12, .MaterialEvent);
    try std.testing.expectEqual(@as(u64, 2), owner.queued.?.metadata.request_id);
    try std.testing.expectEqualDeep(first, owner.in_flight.?);
    for (0..5) |field| {
        var wrong = first.metadata;
        switch (field) {
            0 => wrong.device_epoch += 1,
            1 => wrong.request_id += 1,
            2 => wrong.model_version += 1,
            3 => wrong.source_tick += 1,
            else => wrong.source_fingerprint += 1,
        }
        const before = owner;
        try std.testing.expectEqual(c.Disposition.Stale, owner.completeGpu(wrong, error.MapFailure).disposition);
        try std.testing.expectEqualDeep(before, owner);
    }
    const stale = owner.completeGpu(first.metadata, first.cpu_scores);
    try std.testing.expectEqual(c.Disposition.Stale, stale.disposition);
    try std.testing.expect(stale.snapshot == null);
    try std.testing.expectEqualDeep(newest.snapshot, owner.published().?.*);
    const dispatch = stale.gpu_request.?;
    try std.testing.expectEqual(@as(u64, 2), dispatch.metadata.request_id);
    const accepted = owner.completeGpu(dispatch.metadata, dispatch.cpu_scores);
    try std.testing.expectEqual(c.Disposition.Accepted, accepted.disposition);
    try std.testing.expectEqual(c.Backend.WebGpu, accepted.snapshot.?.backend);
    try std.testing.expectEqualDeep(dispatch.cpu_scores, accepted.snapshot.?.scores);
    try std.testing.expectEqual(@as(u8, 1), accepted.snapshot.?.priority);
    try std.testing.expect(!owner.hasInFlightRequest() and !owner.hasQueuedRequest());
    const before = owner;
    try std.testing.expectEqual(c.Disposition.Stale, owner.completeGpu(dispatch.metadata, error.DeviceFailure).disposition);
    try std.testing.expectEqualDeep(before, owner);
}
test "controller matching old faults precede stale check sticky fallback" {
    for (0..6) |fault| {
        var owner = c.Controller.init(3, true);
        const first = (try request(&owner, 0, 1, .Initialization)).gpu_request.?;
        const newest = try request(&owner, 1, 2, .MaterialEvent);
        var scores = first.cpu_scores;
        switch (fault) {
            0 => {},
            1 => scores[0] = std.math.nan(f32),
            2 => scores[0] = std.math.inf(f32),
            3 => scores[0] = -0.01,
            4 => scores[0] = 1.01,
            else => scores[0] += 0.01,
        }
        const result = owner.completeGpu(first.metadata, if (fault == 0) error.MapFailure else scores);
        try std.testing.expectEqual(c.Disposition.Fallback, result.disposition);
        try std.testing.expectEqualDeep(newest.snapshot.metadata, result.snapshot.?.metadata);
        try std.testing.expectEqual(c.Backend.CpuFallback, result.snapshot.?.backend);
        try std.testing.expect(!owner.gpuEnabled() and !owner.hasInFlightRequest() and !owner.hasQueuedRequest());
        owner.setDeviceEpoch(3, false);
        owner.setDeviceEpoch(3, true);
        try std.testing.expect(!owner.gpuEnabled());
        try std.testing.expectEqual(c.Backend.CpuFallback, (try request(&owner, 2, 3, .Restart)).snapshot.backend);
        const historical = owner.publication;
        const id = owner.next_request_id;
        owner.setDeviceEpoch(4, true);
        try std.testing.expect(owner.gpuEnabled());
        try std.testing.expectEqualDeep(historical, owner.publication);
        try std.testing.expectEqual(id, owner.next_request_id);
    }
}
test "controller availability false retains ownership until valid completion" {
    for ([_]bool{ false, true }) |obsolete| {
        var owner = c.Controller.init(5, true);
        const first = (try request(&owner, 0, 1, .Initialization)).gpu_request.?;
        if (obsolete) _ = try request(&owner, 1, 2, .MaterialEvent);
        owner.setDeviceEpoch(5, false);
        try std.testing.expect(owner.hasInFlightRequest());
        try std.testing.expectEqual(obsolete, owner.hasQueuedRequest());
        const publication = owner.publication;
        const result = owner.completeGpu(first.metadata, first.cpu_scores);
        try std.testing.expectEqual(if (obsolete) c.Disposition.Stale else c.Disposition.Accepted, result.disposition);
        try std.testing.expect(!owner.hasInFlightRequest() and !owner.hasQueuedRequest());
        try std.testing.expect(result.gpu_request == null and owner.disabled_epoch == null);
        if (obsolete) try std.testing.expectEqualDeep(publication, owner.publication) else try std.testing.expectEqual(c.Backend.WebGpu, owner.published().?.backend);
    }
}
test "controller explicit device failure without flight or latest" {
    var empty = c.Controller.init(0, false);
    const result = empty.failDeviceEpoch(error.PollFailure);
    try std.testing.expectEqual(c.Disposition.Fallback, result.disposition);
    try std.testing.expect(result.snapshot == null and empty.published() == null);
    var owner = c.Controller.init(0, false);
    const cpu = try request(&owner, 0, 99, .Initialization);
    const fallback = owner.failDeviceEpoch(error.DeviceFailure);
    try std.testing.expectEqualDeep(cpu.snapshot.scores, fallback.snapshot.?.scores);
    try std.testing.expectEqual(c.Backend.CpuFallback, fallback.snapshot.?.backend);
}
test "controller request snapshot move copies and delayed source preserve authority" {
    var observation = obs(0);
    const original = observation;
    var owner = c.Controller.init(9, true);
    const maybe_outcome = owner.requestIfDue(observation, 777, .Initialization);
    try std.testing.expect(maybe_outcome != null);
    const outcome = maybe_outcome.?;
    try std.testing.expect(outcome.gpu_request != null and owner.published() != null);
    const request_copy = outcome.gpu_request.?;
    const snapshot_copy = owner.published().?.*;
    try std.testing.expectEqualDeep(original, observation);
    observation.next_tick = 1;
    observation.worlds[0].defense = 0;
    try std.testing.expect(owner.requestIfDue(observation, 778, .Tick) == null);
    var moved = owner;
    owner = c.Controller.init(99, false);
    const result = moved.completeGpu(request_copy.metadata, request_copy.cpu_scores);
    try std.testing.expectEqual(c.Disposition.Accepted, result.disposition);
    try std.testing.expectEqual(@as(u64, 0), result.snapshot.?.metadata.source_tick);
    try std.testing.expectEqual(@as(u64, 777), snapshot_copy.metadata.source_fingerprint);
    try std.testing.expectEqual(@as(u64, 777), request_copy.metadata.source_fingerprint);
    try std.testing.expectEqualDeep(snapshot_copy.scores, request_copy.cpu_scores);
}
test "controller valid near tie priority change accepted" {
    var observation = obs(0);
    for (&observation.worlds) |*world| {
        world.owner = null;
        world.defense = 0;
        world.energy = 0;
        world.base_output_per_second = 0;
        world.base_regeneration_per_second = 0;
        world.fields = .init(0, 0, 0);
    }
    var owner = c.Controller.init(1, true);
    const maybe_outcome = owner.requestIfDue(observation, 1, .Initialization);
    try std.testing.expect(maybe_outcome != null);
    try std.testing.expect(maybe_outcome.?.gpu_request != null);
    const first = maybe_outcome.?.gpu_request.?;
    var scores = first.cpu_scores;
    scores[1] += 0.000001;
    try std.testing.expectEqual(@as(u8, 0), owner.published().?.priority);
    const result = owner.completeGpu(first.metadata, scores);
    try std.testing.expectEqual(c.Disposition.Accepted, result.disposition);
    try std.testing.expectEqual(@as(u8, 1), result.snapshot.?.priority);
}
test "controller queries requests completions preserve simulation digest and pending queue" {
    const simulation = @import("simulation.zig");
    var authority = simulation.Simulation.generated(std.testing.allocator, m.DEFAULT_SEED, .{});
    defer authority.deinit();
    try authority.enqueueNext(0, .{ .launch = .{ .source = 0, .destination = 3 } });
    const pending_copy = authority.pending.items[0];
    const fingerprint = authority.canonicalFingerprint();
    const pending = authority.pending.items.len;
    const observation: t.Observation = .{ .next_tick = authority.state.next_tick, .worlds = authority.state.worlds, .fleets = authority.state.fleets.items, .active_hazard = authority.state.active_hazard };
    var owner = c.Controller.init(1, true);
    const first = owner.requestIfDue(observation, fingerprint, .Initialization);
    try std.testing.expect(first != null);
    try std.testing.expect(first.?.gpu_request != null);
    _ = owner.published();
    _ = owner.gpuEnabled();
    _ = owner.hasInFlightRequest();
    _ = owner.hasQueuedRequest();
    const dispatch = first.?.gpu_request.?;
    _ = owner.completeGpu(dispatch.metadata, dispatch.cpu_scores);
    _ = owner.failDeviceEpoch(error.MapFailure);
    try std.testing.expectEqual(fingerprint, authority.canonicalFingerprint());
    try std.testing.expectEqual(pending, authority.pending.items.len);
    try std.testing.expectEqualDeep(pending_copy, authority.pending.items[0]);
    try std.testing.expectEqualDeep(observation.worlds, authority.state.worlds);
}
test "controller latest requested tuple remains eligible after real nonmaterial steps" {
    const simulation = @import("simulation.zig");
    var authority = simulation.Simulation.generated(std.testing.allocator, m.DEFAULT_SEED, .{ .ai_period_ticks = 0, .hazard_first_tick = std.math.maxInt(u64) });
    defer authority.deinit();
    var owner = c.Controller.init(1, true);
    const initial: t.Observation = .{ .next_tick = authority.state.next_tick, .worlds = authority.state.worlds, .fleets = authority.state.fleets.items, .active_hazard = authority.state.active_hazard };
    const result = owner.requestIfDue(initial, authority.canonicalFingerprint(), .Initialization);
    try std.testing.expect(result != null and result.?.gpu_request != null);
    const dispatch = result.?.gpu_request.?;
    for (0..3) |_| {
        var report = try authority.step();
        defer report.deinit(std.testing.allocator);
        try std.testing.expectEqual(@as(usize, 0), report.events.items.len);
        const current: t.Observation = .{ .next_tick = authority.state.next_tick, .worlds = authority.state.worlds, .fleets = authority.state.fleets.items, .active_hazard = authority.state.active_hazard };
        try std.testing.expect(owner.requestIfDue(current, authority.canonicalFingerprint(), .Tick) == null);
    }
    const before = authority.canonicalFingerprint();
    const accepted = owner.completeGpu(dispatch.metadata, dispatch.cpu_scores);
    try std.testing.expectEqual(c.Disposition.Accepted, accepted.disposition);
    try std.testing.expectEqual(@as(u64, 0), accepted.snapshot.?.metadata.source_tick);
    try std.testing.expectEqual(@as(u64, 3), authority.state.next_tick);
    try std.testing.expectEqual(before, authority.canonicalFingerprint());
}
