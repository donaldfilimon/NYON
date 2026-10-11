const std = @import("std");
const t = std.testing;
const a = t.allocator;
const app = @import("classic_app.zig");
const scenario = @import("scenario.zig");
const m = @import("model.zig");
const sim = @import("simulation.zig");
const prefs = @import("preferences.zig");

fn fixture() !scenario.ScenarioV1 {
    return scenario.generatedValidated(m.DEFAULT_SEED, .{});
}
fn ready(allocator: std.mem.Allocator) !app.AppCore {
    const candidate = try fixture();
    return app.AppCore.init(allocator, &candidate, .{ .onboarding_completed = true });
}
// Controller work is Task3B. These fixtures seed the existing owned authority
// queue and session sequence solely to qualify Task3A lifecycle preservation.
fn queue(core: *app.AppCore) !void {
    try core._simulation.enqueueNext(core._next_sequence, .{ .launch = .{ .source = 0, .destination = 3 } });
    core._next_sequence +%= 1;
    try core._simulation.enqueueNext(core._next_sequence, .{ .tune_field = .{ .world = 0, .field = .atmosphere, .adjustment = .increase } });
    core._next_sequence +%= 1;
}

test "Classic session speed ordering and 100ms long-frame canonical traces" {
    const speeds = [_]app.GameSpeed{ .paused, .normal, .fast, .very_fast };
    const faster = [_]app.GameSpeed{ .normal, .fast, .very_fast, .very_fast };
    const slower = [_]app.GameSpeed{ .paused, .paused, .normal, .fast };
    for (speeds, faster, slower) |speed, up, down| {
        try t.expectEqual(up, speed.faster());
        try t.expectEqual(down, speed.slower());
        for ([_]u128{ 100_000_000, 2_000_000_000 }) |delta| {
            var core = try ready(a);
            defer core.deinit();
            _ = core.setSpeed(speed);
            const count: usize = @as(usize, speed.multiplier()) * @as(usize, if (delta == 100_000_000) 6 else 15);
            const frame = core.advance(delta);
            try t.expectEqual(count, frame.completed_steps);
            try t.expectEqual(count, frame.ticks().len);
            const candidate = try fixture();
            var direct = try sim.Simulation.fromScenario(a, &candidate);
            defer direct.deinit();
            try direct.stepN(@intCast(count));
            try t.expectEqual(direct.canonicalFingerprint(), core.canonicalFingerprint());
            try t.expectEqualDeep(direct.replayIdentity(), core.replayIdentity());
            for (frame.ticks(), 0..) |summary, index| try t.expectEqual(@as(u64, @intCast(index)), summary.tick);
        }
    }
}

test "Classic session no-op speed retains residue and transitions discard it" {
    var core = try ready(a);
    defer core.deinit();
    const before = core.canonicalFingerprint();
    _ = core.advance(1_000_000);
    const alpha = core.clockAlpha();
    try t.expect(alpha > 0);
    try t.expectEqual(app.SpeedSelection.unchanged, core.setSpeed(.normal));
    try t.expectEqual(alpha, core.clockAlpha());
    try t.expectEqual(app.SpeedSelection.changed, core.setSpeed(.fast));
    try t.expectEqual(@as(f32, 0), core.clockAlpha());
    try t.expectEqual(app.GameSpeed.fast, core.resumeSpeed());
    _ = core.setSpeed(.paused);
    try t.expectEqual(app.GameSpeed.fast, core.resumeSpeed());
    try t.expect(core.advance(2_000_000_000).status == .paused);
    _ = core.setSpeed(.very_fast);
    for ([_]app.AppMode{ .settings, .editing_scenario }) |mode| {
        _ = core.advance(1_000_000);
        try t.expect(core.clockAlpha() > 0);
        core.setMode(mode);
        try t.expectEqual(@as(f32, 0), core.clockAlpha());
        try t.expect(core.advance(2_000_000_000).status == .mode_hold);
        try t.expectEqual(app.GameSpeed.very_fast, core.speed());
        try t.expectEqual(app.GameSpeed.very_fast, core.resumeSpeed());
        core.setMode(.playing);
        try t.expectEqual(@as(f32, 0), core.clockAlpha());
    }
    _ = core.advance(1_000_000);
    core.onSuspend();
    try t.expectEqual(@as(f32, 0), core.clockAlpha());
    _ = core.advance(1_000_000);
    core.onResume();
    try t.expectEqual(@as(f32, 0), core.clockAlpha());
    try t.expectEqual(before, core.canonicalFingerprint());
}

test "Classic session onboarding hold seam rejects speed without mutation" {
    const candidate = try fixture();
    var core = try app.AppCore.init(a, &candidate, .{});
    defer core.deinit();
    try t.expect(core.onboardingHeld());
    try t.expectEqual(app.GameSpeed.paused, core.speed());
    try t.expectEqual(app.GameSpeed.normal, core.resumeSpeed());
    const identity = core.replayIdentity();
    // Deliberately seed clock residue to prove rejected and no-op selections
    // themselves do not mutate clock or authority.
    core._clock.pushFrame(1_000_000);
    const alpha = core.clockAlpha();
    for ([_]app.GameSpeed{ .normal, .fast, .very_fast }) |speed| {
        try t.expectEqual(app.SpeedSelection.onboarding_rejected, core.setSpeed(speed));
        try t.expectEqual(alpha, core.clockAlpha());
    }
    try t.expectEqual(app.SpeedSelection.unchanged, core.setSpeed(.paused));
    try t.expectEqual(alpha, core.clockAlpha());
    try t.expect(core.advance(2_000_000_000).status == .onboarding_hold);
    try t.expectEqualDeep(identity, core.replayIdentity());
    core.setOnboardingHold(false);
    try t.expectEqual(app.GameSpeed.normal, core.speed());
    try t.expectEqual(@as(f32, 0), core.clockAlpha());
    try t.expect(!core.preferences().onboarding_completed);
    _ = core.setSpeed(.fast);
    core.setOnboardingHold(true);
    try t.expectEqual(app.GameSpeed.paused, core.speed());
    try t.expectEqual(app.GameSpeed.fast, core.resumeSpeed());
    core.setOnboardingHold(false);
    try t.expectEqual(app.GameSpeed.fast, core.speed());
}

test "Classic session material summaries own values after reports are released" {
    var core = try ready(a);
    defer core.deinit();
    try queue(&core);
    const frame = core.advance(100_000_000);
    try t.expectEqual(@as(usize, 6), frame.completed_steps);
    try t.expectEqual(@as(usize, 2), frame.accepted_commands);
    try t.expectEqual(@as(usize, 1), frame.material_steps);
    try t.expectEqual(@as(usize, 1), frame.event_count);
    try t.expect(frame.ticks()[0].material());
    try core.restart();
    try t.expectEqual(@as(usize, 2), frame.ticks()[0].accepted_commands);
    try t.expectEqual(@as(usize, 1), frame.ticks()[0].event_count);
    try core._simulation.enqueueNext(0, .{ .launch = .{ .source = 99, .destination = 1 } });
    const rejected = core.advance(16_666_667);
    try t.expectEqual(@as(usize, 1), rejected.completed_steps);
    try t.expectEqual(@as(usize, 0), rejected.accepted_commands);
    try t.expectEqual(@as(usize, 0), rejected.event_count);
    try t.expectEqual(@as(usize, 0), rejected.material_steps);
    try t.expect(!rejected.ticks()[0].material());
}

test "Classic session public initialization rejects stale and invalid authority in every build" {
    var candidate = try fixture();
    candidate.seed ^= 1;
    try t.expectError(error.StaleFingerprint, app.AppCore.init(a, &candidate, .{}));
    candidate = try fixture();
    candidate.rules.tick_hz = 0;
    try t.expectError(error.Validation, app.AppCore.init(a, &candidate, .{}));
    candidate = try fixture();
    candidate.worlds[0].production_remainder = 1;
    try t.expectError(error.InvalidInitialState, app.AppCore.init(a, &candidate, .{}));
}

fn seedPresentation(core: *app.AppCore) void {
    core.selected_world = 0;
    core.hovered_world = 3;
    core.touch_command_target = 2;
    core.selected_field = .topology;
    core.help_visible = true;
    core.visual_time = 123;
}
fn expectReset(core: *const app.AppCore) !void {
    try t.expectEqual(@as(u64, 0), core.campaign().next_tick);
    try t.expectEqual(@as(usize, 0), core._simulation.pending.items.len);
    try t.expectEqual(@as(u64, 0), core.nextSequence());
    try t.expectEqual(app.AppMode.playing, core.mode());
    try t.expect(core.selected_world == null and core.hovered_world == null and core.touch_command_target == null);
    try t.expectEqual(m.FieldKind.atmosphere, core.selected_field);
    try t.expect(!core.help_visible);
    try t.expectEqual(@as(f32, 0), core.visual_time);
    try t.expectEqual(@as(f32, 0), core.clockAlpha());
    try t.expect(core.recoverableStatus() == null);
    try t.expectEqual(app.GameSpeed.normal, core.resumeSpeed());
}

test "Classic session restart replacement reset resident state and retain live preferences" {
    var failing = t.FailingAllocator.init(a, .{});
    var core = try ready(failing.allocator());
    defer core.deinit();
    try queue(&core);
    _ = core.advance(100_000_000);
    try queue(&core);
    seedPresentation(&core);
    _ = core.setSpeed(.very_fast);
    core.setMode(.settings);
    const live: prefs.UserPreferencesV1 = .{ .onboarding_completed = true, .motion = .reduced, .ui_scale = .percent130, .high_contrast = true, .graphics_quality = .low };
    core.setPreferences(live);
    // Restart is nonallocating; exhaustion cannot manufacture a failure here.
    failing.fail_index = failing.alloc_index;
    const allocs = failing.alloc_index;
    try core.restart();
    try t.expectEqual(allocs, failing.alloc_index);
    try expectReset(&core);
    try t.expectEqualDeep(live, core.preferences());
    try t.expectEqual(app.GameSpeed.normal, core.speed());
    core.setOnboardingHold(true);
    const replacement = try scenario.generatedValidated(1, .{});
    try core.replaceScenario(&replacement);
    try expectReset(&core);
    try t.expectEqual(app.GameSpeed.paused, core.speed());
    try t.expectEqual(@as(u64, 1), core.activeScenario().seed);
    try t.expectEqualDeep(live, core.preferences());
}

test "Classic session stale invalid replacement preserves resident queue sequence preferences and failure hold" {
    var failing = t.FailingAllocator.init(a, .{});
    var core = try ready(failing.allocator());
    defer core.deinit();
    try queue(&core);
    seedPresentation(&core);
    _ = core.setSpeed(.fast);
    failing.fail_index = failing.alloc_index;
    const failed = core.advance(100_000_000);
    try t.expect(failed.status == .failure_hold);
    core.setMode(.settings);
    const before = core.canonicalFingerprint();
    const held = core.recoverableStatus();
    const preferences = core.preferences();
    const pending = try a.dupe(m.CommandEnvelope, core._simulation.pending.items);
    defer a.free(pending);
    const visual = core.visual_time;
    var stale = try fixture();
    stale.seed ^= 1;
    try t.expectError(error.StaleFingerprint, core.replaceScenario(&stale));
    var invalid = try fixture();
    invalid.rules.tick_hz = 0;
    try t.expectError(error.Validation, core.replaceScenario(&invalid));
    invalid = try fixture();
    invalid.worlds[0].id = 6;
    try t.expectError(error.InvalidInitialState, core.replaceScenario(&invalid));
    try t.expectEqual(before, core.canonicalFingerprint());
    try t.expectEqualDeep(held, core.recoverableStatus());
    try t.expectEqualDeep(preferences, core.preferences());
    try t.expectEqualDeep(pending, core._simulation.pending.items);
    try t.expectEqual(@as(u64, 2), core.nextSequence());
    try t.expectEqual(@as(?m.WorldId, 0), core.selected_world);
    try t.expectEqual(@as(?m.WorldId, 3), core.hovered_world);
    try t.expectEqual(@as(?m.WorldId, 2), core.touch_command_target);
    try t.expectEqual(m.FieldKind.topology, core.selected_field);
    try t.expect(core.help_visible);
    try t.expectEqual(visual, core.visual_time);
    try t.expectEqual(app.GameSpeed.fast, core.speed());
    try t.expectEqual(app.GameSpeed.fast, core.resumeSpeed());
    try t.expectEqual(app.AppMode.settings, core.mode());
    try t.expectEqualDeep((try fixture()), core.activeScenario().*);
}

test "Classic session exhaustive advance allocation failures are atomic sticky recoverable and leak free" {
    var observed_late_failure = false;
    var failure_count: usize = 0;
    // Visit every allocation ordinal until the first uninjected full frame.
    for (0..256) |index| {
        var failing = t.FailingAllocator.init(a, .{});
        {
            var core = try ready(failing.allocator());
            defer core.deinit();
            try queue(&core);
            _ = core.setSpeed(.very_fast);
            failing.fail_index = failing.alloc_index + index;
            const frame = core.advance(100_000_000);
            const candidate = try fixture();
            var direct = try sim.Simulation.fromScenario(a, &candidate);
            defer direct.deinit();
            try direct.enqueueNext(0, .{ .launch = .{ .source = 0, .destination = 3 } });
            try direct.enqueueNext(1, .{ .tune_field = .{ .world = 0, .field = .atmosphere, .adjustment = .increase } });
            try direct.stepN(@intCast(frame.completed_steps));
            try t.expectEqual(direct.canonicalFingerprint(), core.canonicalFingerprint());
            try t.expectEqualDeep(direct.pending.items, core._simulation.pending.items);
            if (frame.status == .failure_hold) {
                failure_count += 1;
                if (frame.completed_steps > 0) observed_late_failure = true;
                try t.expectEqual(error.OutOfMemory, frame.status.failure_hold.cause);
                try t.expectEqual(@as(u64, @intCast(frame.completed_steps)), frame.status.failure_hold.tick);
                try t.expectEqual(@as(f32, 0), core.clockAlpha());
                try t.expectEqual(@as(usize, 0), core._clock.pending_steps);
                const fingerprint = core.canonicalFingerprint();
                const allocations = failing.alloc_index;
                const held = core.advance(2_000_000_000);
                try t.expect(held.status == .failure_hold);
                try t.expectEqual(@as(usize, 0), held.completed_steps);
                try t.expectEqual(allocations, failing.alloc_index);
                try t.expectEqual(fingerprint, core.canonicalFingerprint());
                try t.expectEqual(app.GameSpeed.very_fast, core.speed());
                try t.expectEqual(app.GameSpeed.very_fast, core.resumeSpeed());
                // Mode, speed, preferences and suspend/resume never authorize retry.
                core.setMode(.settings);
                core.setMode(.playing);
                _ = core.setSpeed(.fast);
                core.onSuspend();
                core.onResume();
                core.setPreferences(.{ .onboarding_completed = true, .motion = .reduced });
                try t.expect(core.advance(100_000_000).status == .failure_hold);
                failing.fail_index = std.math.maxInt(usize);
                try t.expect(core.recover());
                try t.expect(!core.recover());
                try t.expectEqual(@as(usize, 0), core.advance(0).completed_steps);
                _ = core.setSpeed(.normal);
                const recovered = core.advance(16_666_667);
                try t.expectEqual(@as(usize, 1), recovered.completed_steps);
                try direct.stepN(1);
                try t.expectEqual(direct.canonicalFingerprint(), core.canonicalFingerprint());
                // A fresh explicit restart also clears a failure hold.
                failing.fail_index = failing.alloc_index;
                try t.expect(core.advance(16_666_667).status == .failure_hold);
                try core.restart();
                try expectReset(&core);
            } else {
                try t.expectEqual(@as(usize, 24), frame.completed_steps);
                try t.expect(!failing.has_induced_failure);
            }
        }
        try t.expectEqual(failing.allocated_bytes, failing.freed_bytes);
        if (!failing.has_induced_failure) break;
        if (index == 255) return error.AllocationSweepDidNotTerminate;
    }
    try t.expect(observed_late_failure);
    try t.expect(failure_count > 3);
}

test "Classic session gameplay finished is absorbing and reports only transitioning steps" {
    var core = try ready(a);
    defer core.deinit();
    // A synthetic authority fixture at the outcome boundary, following the
    // authority tests: the next real step emits campaign_finished exactly once.
    for (core._simulation.state.worlds[0..5]) |*world| world.owner = .union_faction;
    _ = core.setSpeed(.very_fast);
    const frame = core.advance(2_000_000_000);
    try t.expect(frame.status == .gameplay_finished);
    try t.expectEqual(@as(usize, 1), frame.completed_steps);
    try t.expectEqual(@as(usize, 1), frame.material_steps);
    const identity = core.replayIdentity();
    const again = core.advance(2_000_000_000);
    try t.expect(again.status == .gameplay_finished);
    try t.expectEqual(@as(usize, 0), again.completed_steps);
    try t.expectEqualDeep(identity, core.replayIdentity());
    try t.expectEqual(@as(f32, 0), core.clockAlpha());
    try t.expectEqual(app.GameSpeed.very_fast, core.speed());
}

test "Classic session visual time follows live motion preference through holds and integer delta bounds" {
    var core = try ready(a);
    defer core.deinit();
    _ = core.setSpeed(.paused);
    _ = core.advance(1_000_000_000);
    try t.expectEqual(@as(f32, 1), core.visual_time);
    core.setPresentationPreferences(.{ .motion = .reduced });
    _ = core.advance(1_000_000_000);
    try t.expectEqual(@as(f32, 1), core.visual_time);
    core.setPresentationPreferences(.{});
    _ = core.advance(std.math.maxInt(u128));
    try t.expect(std.math.isFinite(core.visual_time));
    try t.expect(core.visual_time >= 0 and core.visual_time < 4096);
}
