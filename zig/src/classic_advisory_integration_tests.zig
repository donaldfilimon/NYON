const std = @import("std");
const t = std.testing;
const app = @import("classic_app.zig");
const scenario = @import("scenario.zig");
const m = @import("model.zig");
const projection = @import("classic_advisory_projection.zig");
fn ready() !app.AppCore {
    const candidate = try scenario.generatedValidated(m.DEFAULT_SEED, .{});
    return app.AppCore.init(t.allocator, &candidate, .{ .onboarding_completed = true });
}
test "F3 initialization publishes exact factory CPU immediately" {
    var core = try ready();
    defer core.deinit();
    try t.expect(core.advisorySnapshot() != null);
    const snapshot = core.advisorySnapshot().?.*;
    try t.expectEqual(@as(u64, 0), snapshot.metadata.request_id);
    try t.expectEqual(@as(u64, 0), snapshot.metadata.device_epoch);
    try t.expectEqual(core.canonicalFingerprint(), snapshot.metadata.source_fingerprint);
    try t.expectEqual(.Cpu, snapshot.backend);
    for (snapshot.scores, [_]u32{ 1058173887, 1060329379, 1059226841, 1054688186, 1056209868, 1054665716, 1057420253 }) |score, bits| try t.expectEqual(bits, @as(u32, @bitCast(score)));
    try t.expect(!core.advisoryUpdating());
    try t.expect(core.takeGpuRequest() == null);
}
test "F3 cadence and material observe actual intermediate successful steps" {
    var core = try ready();
    defer core.deinit();
    core.setGpuEpoch(2, true);
    try t.expect(core.takeGpuRequest() != null);
    _ = try core.enqueueGameCommand(.{ .tune_field = .{ .world = 0, .field = .atmosphere, .adjustment = .increase } });
    try t.expectEqual(@as(u64, 1), core.advisorySnapshot().?.metadata.request_id);
    const frame = core.advance(100_000_000);
    try t.expectEqual(@as(usize, 6), frame.completed_steps);
    try t.expect(core.advisorySnapshot().?.metadata.request_id > 1);
    try t.expect(core.advisorySnapshot().?.metadata.source_tick < core.campaign().next_tick);
    try t.expect(core.advisorySnapshot().?.metadata.source_fingerprint != core.canonicalFingerprint());
}
test "F3 replacement retains request history and uses installed canonical state" {
    var core = try ready();
    defer core.deinit();
    const candidate = try scenario.generatedValidated(m.DEFAULT_SEED + 1, .{});
    try core.replaceScenario(&candidate);
    try t.expect(core.advisorySnapshot() != null);
    try t.expectEqual(@as(u64, 1), core.advisorySnapshot().?.metadata.request_id);
    try t.expectEqual(core.canonicalFingerprint(), core.advisorySnapshot().?.metadata.source_fingerprint);
}

test "F3 sixty intermediate steps equal independent one-step metadata journey" {
    var batch = try ready();
    defer batch.deinit();
    var single = try ready();
    defer single.deinit();
    batch.setGpuEpoch(5, true);
    single.setGpuEpoch(5, true);
    _ = batch.takeGpuRequest();
    _ = single.takeGpuRequest();
    _ = try batch.enqueueGameCommand(.{ .tune_field = .{ .world = 0, .field = .atmosphere, .adjustment = .increase } });
    _ = try single.enqueueGameCommand(.{ .tune_field = .{ .world = 0, .field = .atmosphere, .adjustment = .increase } });
    _ = batch.setSpeed(.very_fast);
    const frame = batch.advance(250_000_000);
    try t.expectEqual(@as(usize, 60), frame.completed_steps);
    var last_id: u64 = 1;
    var observations: usize = 0;
    var prior_fingerprint = single.canonicalFingerprint();
    for (0..60) |index| {
        const one = single.advance(16_666_667);
        try t.expectEqual(@as(usize, 1), one.completed_steps);
        const snapshot = single.advisorySnapshot().?.*;
        const due = one.material_steps != 0 or (index + 1) % 30 == 0;
        if (due) {
            last_id += 1;
            observations += 1;
            try t.expectEqual(last_id, snapshot.metadata.request_id);
            try t.expectEqual(@as(u64, index + 1), snapshot.metadata.source_tick);
            try t.expectEqual(single.canonicalFingerprint(), snapshot.metadata.source_fingerprint);
            try t.expect(snapshot.metadata.source_fingerprint != prior_fingerprint);
            prior_fingerprint = snapshot.metadata.source_fingerprint;
            try t.expect(single._advisory.queued != null);
            try t.expect(single._advisory.queued.?.metadata.eql(snapshot.metadata));
        } else try t.expectEqual(last_id, snapshot.metadata.request_id);
    }
    try t.expect(observations >= 3);
    try t.expectEqualDeep(single.advisorySnapshot().?.*, batch.advisorySnapshot().?.*);
    try t.expectEqualDeep(single._advisory.queued, batch._advisory.queued);
    try t.expectEqual(single.canonicalFingerprint(), batch.canonicalFingerprint());
    try t.expectEqual(single._advisory.last_cadence_tick, batch._advisory.last_cadence_tick);
}

test "F3 rejected command nonmaterial hold zero frames and queries consume no request" {
    var core = try ready();
    defer core.deinit();
    const initial = core.advisorySnapshot().?.*;
    _ = try core.enqueueGameCommand(.{ .launch = .{ .source = 0, .destination = 0 } });
    _ = core.previewCommand(.{ .launch = .{ .source = 0, .destination = 0 } }) catch {};
    core.hovered_world = 2;
    _ = core.advance(0);
    core.setMode(.settings);
    _ = core.advance(1_000_000_000);
    core.setMode(.editing_scenario);
    _ = core.advance(1_000_000_000);
    core.setMode(.playing);
    core.setOnboardingHold(true);
    _ = core.advance(1_000_000_000);
    core.setOnboardingHold(false);
    const frame = core.advance(16_666_667);
    try t.expectEqual(@as(usize, 1), frame.completed_steps);
    try t.expectEqual(@as(usize, 0), frame.material_steps);
    try t.expectEqualDeep(initial, core.advisorySnapshot().?.*);
    const fingerprint = core.canonicalFingerprint();
    const sequence = core.nextSequence();
    const queue_len = core.pendingCommandCount();
    _ = projection.project(&core.campaign().worlds, core.advisorySnapshot(), core.advisoryUpdating());
    try t.expectEqual(fingerprint, core.canonicalFingerprint());
    try t.expectEqual(sequence, core.nextSequence());
    try t.expectEqual(queue_len, core.pendingCommandCount());
}

test "F3 later real tick OOM preserves prior publication failing queue and sticky hold" {
    var failing = t.FailingAllocator.init(t.allocator, .{});
    const candidate = try scenario.generatedValidated(m.DEFAULT_SEED, .{});
    var core = try app.AppCore.init(failing.allocator(), &candidate, .{ .onboarding_completed = true });
    defer core.deinit();
    // Measure the first step allocations independently; the next report allocation fails.
    var measure = t.FailingAllocator.init(t.allocator, .{});
    var direct = try app.AppCore.init(measure.allocator(), &candidate, .{ .onboarding_completed = true });
    defer direct.deinit();
    _ = try core.enqueueGameCommand(.{ .tune_field = .{ .world = 0, .field = .atmosphere, .adjustment = .increase } });
    _ = try direct.enqueueGameCommand(.{ .tune_field = .{ .world = 0, .field = .atmosphere, .adjustment = .increase } });
    const one = direct.advance(16_666_667);
    try t.expectEqual(@as(usize, 1), one.completed_steps);
    failing.fail_index = measure.alloc_index;
    const frame = core.advance(100_000_000);
    try t.expectEqual(@as(usize, 1), frame.completed_steps);
    try t.expect(frame.status == .failure_hold);
    try t.expectEqual(@as(u64, 1), core.campaign().next_tick);
    try t.expectEqual(direct.canonicalFingerprint(), core.canonicalFingerprint());
    try t.expectEqualDeep(direct.advisorySnapshot().?.*, core.advisorySnapshot().?.*);
    try t.expectEqualDeep(direct._simulation.pending.items, core._simulation.pending.items);
    const snapshot = core.advisorySnapshot().?.*;
    try t.expectEqual(@as(usize, 0), core.advance(100_000_000).completed_steps);
    try t.expectEqualDeep(snapshot, core.advisorySnapshot().?.*);
    failing.fail_index = std.math.maxInt(usize);
    try t.expect(core.recover());
    try t.expectEqual(@as(usize, 0), core.advance(0).completed_steps);
    try t.expectEqual(@as(usize, 1), core.advance(16_666_667).completed_steps);
}

test "F3 owned host dispatch move completion newest fallback and epoch semantics" {
    var source = try ready();
    source.setGpuEpoch(7, true);
    try t.expect(source._pending_gpu_request != null);
    const copied = source._pending_gpu_request.?;
    var core = source;
    source = undefined; // transfer move-only authority ownership
    defer core.deinit();
    const request = core.takeGpuRequest();
    try t.expect(request != null);
    try t.expectEqualDeep(copied, request.?);
    try t.expect(core.takeGpuRequest() == null);
    try core.restart();
    const newest = core.advisorySnapshot().?.*;
    try t.expect(newest.metadata.request_id > copied.metadata.request_id);
    const result = core.completeGpu(copied.metadata, copied.cpu_scores);
    try t.expectEqual(.Stale, result.disposition);
    try t.expect(result.gpu_request != null);
    const promoted = core.takeGpuRequest();
    try t.expect(promoted != null);
    try t.expect(promoted.?.metadata.eql(newest.metadata));
    try t.expect(core.takeGpuRequest() == null);
    const accepted = core.completeGpu(promoted.?.metadata, promoted.?.cpu_scores);
    try t.expectEqual(.Accepted, accepted.disposition);
    try t.expectEqual(.WebGpu, core.advisorySnapshot().?.backend);
    core.setGpuEpoch(8, true);
    const old = core.takeGpuRequest();
    try t.expect(old != null);
    _ = try core.enqueueGameCommand(.{ .tune_field = .{ .world = 0, .field = .hydrosphere, .adjustment = .increase } });
    _ = core.advance(16_666_667);
    const latest_cpu = core.advisorySnapshot().?.*;
    try t.expect(latest_cpu.metadata.request_id > old.?.metadata.request_id);
    const fallback = core.completeGpu(old.?.metadata, error.MapFailure);
    try t.expectEqual(.Fallback, fallback.disposition);
    try t.expectEqualDeep(latest_cpu.scores, core.advisorySnapshot().?.scores);
    try t.expect(core.advisorySnapshot().?.metadata.eql(latest_cpu.metadata));
    try t.expectEqual(.CpuFallback, core.advisorySnapshot().?.backend);
    try t.expect(!core.advisoryUpdating());
    core.setGpuEpoch(8, true);
    try t.expect(core.takeGpuRequest() == null);
    try t.expectEqual(.CpuFallback, core.advisorySnapshot().?.backend);
    core.setGpuEpoch(9, false);
    try t.expectEqual(.Cpu, core.advisorySnapshot().?.backend);
    try t.expect(core.takeGpuRequest() == null);
    core.setGpuEpoch(10, true);
    try t.expect(core._pending_gpu_request != null);
    _ = core.failDeviceEpoch(error.DeviceFailure);
    try t.expect(core.takeGpuRequest() == null);
    try t.expect(!core.advisoryUpdating());
    try t.expectEqual(.CpuFallback, core.advisorySnapshot().?.backend);
}

test "F3 bounded panel owns name lookup by world id and exact source labels" {
    var core = try ready();
    defer core.deinit();
    const absent = projection.project(&core.campaign().worlds, null, true);
    try t.expect(absent.initializing);
    try t.expectEqualStrings("ADVISORY ONLY", absent.title);
    try t.expectEqual(@as(u64, 0), absent.source_tick);
    try t.expect(absent.state_label == null and absent.backend_label == null);
    var snapshot = core.advisorySnapshot().?.*;
    var worlds = core.campaign().worlds;
    const priority = snapshot.priority;
    worlds[0].id = priority;
    worlds[0].name = try m.WorldName.init("ID SEARCH WITNESS");
    var panel = projection.project(&worlds, &snapshot, false);
    try t.expect(panel.priority_name != null);
    try t.expectEqualStrings("ID SEARCH WITNESS", panel.priority_name.?.asSlice());
    worlds[0].name = try m.WorldName.init("REPLACED");
    try t.expectEqualStrings("ID SEARCH WITNESS", panel.priority_name.?.asSlice());
    try t.expectEqual(snapshot.scores[priority], panel.priority_score.?);
    try t.expectEqualStrings("CPU", panel.backend_label.?);
    try t.expectEqualStrings("CURRENT", panel.state_label.?);
    for (&worlds) |*world| world.id = 10;
    snapshot.backend = .WebGpu;
    snapshot.metadata.source_tick = 123;
    panel = projection.project(&worlds, &snapshot, true);
    try t.expectEqualStrings("UNKNOWN", panel.priority_name.?.asSlice());
    try t.expectEqualStrings("WEBGPU", panel.backend_label.?);
    try t.expectEqualStrings("UPDATING", panel.state_label.?);
    try t.expectEqual(@as(u64, 123), panel.source_tick);
    snapshot.backend = .CpuFallback;
    panel = projection.project(&worlds, &snapshot, false);
    try t.expectEqualStrings("CPU FALLBACK", panel.backend_label.?);
}

test "F3 failed install preserves current advisory and nonempty failing tick queue" {
    var failing = t.FailingAllocator.init(t.allocator, .{});
    var candidate = try scenario.generatedValidated(m.DEFAULT_SEED, .{});
    var core = try app.AppCore.init(failing.allocator(), &candidate, .{ .onboarding_completed = true });
    defer core.deinit();
    _ = core.advance(16_666_667);
    _ = try core.enqueueGameCommand(.{ .launch = .{ .source = 0, .destination = 3 } });
    const snapshot = core.advisorySnapshot().?.*;
    const fingerprint = core.canonicalFingerprint();
    const pending = try t.allocator.dupe(m.CommandEnvelope, core._simulation.pending.items);
    defer t.allocator.free(pending);
    failing.fail_index = failing.alloc_index;
    const failed = core.advance(16_666_667);
    try t.expect(failed.status == .failure_hold);
    try t.expectEqualDeep(pending, core._simulation.pending.items);
    try t.expectEqual(fingerprint, core.canonicalFingerprint());
    try t.expectEqualDeep(snapshot, core.advisorySnapshot().?.*);
    candidate.seed ^= 1;
    try t.expectError(error.StaleFingerprint, core.replaceScenario(&candidate));
    try t.expectEqualDeep(snapshot, core.advisorySnapshot().?.*);
    try t.expect(core.recoverableStatus() != null);
}

test "F3 moved Session open close failed apply and successful install consume advisory once" {
    const session_module = @import("classic_session.zig");
    const store = @import("classic_store.zig");
    var raw = store.MemoryStore.init(t.allocator, .preferences);
    defer raw.deinit();
    var scenario_store = store.MemoryStore.init(t.allocator, .scenario);
    defer scenario_store.deinit();
    const candidate = try scenario.generatedValidated(m.DEFAULT_SEED, .{});
    var source = try session_module.Session.init(t.allocator, &candidate, raw.raw());
    var session = source;
    source = undefined;
    defer session.deinit();
    const initial = session.core.advisorySnapshot().?.*;
    try t.expect(session.core.onboardingHeld());
    _ = session.handleUiAction(.acknowledge_advisory, raw.raw());
    _ = session.controls(null);
    try t.expectEqualDeep(initial, session.core.advisorySnapshot().?.*);
    const opened = session.openEditor();
    try t.expect(!opened.effects.restart_advisory);
    try t.expectEqualDeep(initial, session.core.advisorySnapshot().?.*);
    const closed = session.dispatchEditor(.cancel, scenario_store.raw());
    try t.expect(!closed.effects.restart_advisory);
    try t.expectEqualDeep(initial, session.core.advisorySnapshot().?.*);
    _ = session.openEditor();
    try t.expect(session.editor.session != null);
    _ = session.dispatchEditor(.{ .select_section = .rules }, scenario_store.raw());
    try session.editor.session.?.focusField("scenario.seed");
    try session.editor.session.?.replaceFocusedText("invalid");
    const bad = session.dispatchEditor(.apply_and_restart, scenario_store.raw());
    try t.expectEqual(@as(?@import("classic_editor_transactions.zig").Outcome, .failed), bad.editor_outcome);
    try t.expectEqualDeep(initial, session.core.advisorySnapshot().?.*);
    try session.editor.session.?.replaceFocusedText("4947573120260902");
    const confirmation = session.dispatchEditor(.apply_and_restart, scenario_store.raw());
    try t.expectEqual(@as(?@import("classic_editor_transactions.zig").Outcome, .modal_opened), confirmation.editor_outcome);
    try t.expectEqualDeep(initial, session.core.advisorySnapshot().?.*);
    const good = session.dispatchEditor(.confirm, scenario_store.raw());
    try t.expectEqual(@as(?@import("classic_editor_transactions.zig").Outcome, .applied), good.editor_outcome);
    try t.expect(!good.effects.restart_advisory);
    try t.expect(!session.editor.effects.restart_advisory);
    try t.expectEqual(@as(u64, 1), session.core.advisorySnapshot().?.metadata.request_id);
    try t.expectEqual(session.core.canonicalFingerprint(), session.core.advisorySnapshot().?.metadata.source_fingerprint);
    const preferences = session.core.preferences();
    const progress = session.onboarding.step();
    const restart = try session.restart();
    try t.expect(!restart.restart_advisory);
    try t.expectEqual(@as(u64, 2), session.core.advisorySnapshot().?.metadata.request_id);
    try t.expectEqualDeep(preferences, session.core.preferences());
    try t.expectEqual(progress, session.onboarding.step());
    try t.expect(session.core.onboardingHeld());
}
