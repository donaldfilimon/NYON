const std = @import("std");
const t = std.testing;
const app = @import("classic_app.zig");
const scenario = @import("scenario.zig");
const m = @import("model.zig");
const prefs = @import("preferences.zig");
const camera = @import("classic_camera.zig");
fn ready() !app.AppCore {
    const candidate = try scenario.generatedValidated(m.DEFAULT_SEED, .{});
    return app.AppCore.init(t.allocator, &candidate, .{ .onboarding_completed = true });
}
test "G3 core effective presentation controls held-frame raw time and camera exactly once" {
    try t.expect(@hasDecl(app.AppCore, "setPresentationPreferences"));
    if (comptime @hasDecl(app.AppCore, "setPresentationPreferences")) {
        for ([_]u128{ 0, 250_000_000, 1_000_000_000, 16_666_666, 16_666_667 }) |ns| {
            var core = try ready();
            defer core.deinit();
            _ = core.setSpeed(.paused);
            core.orbitCamera(0.5, 0);
            var durable = core.preferences();
            durable.motion = .reduced;
            core.setPreferences(durable);
            core.setPresentationPreferences(.{ .motion = .full, .graphics_quality = .low, .high_contrast = true });
            _ = core.advance(ns);
            const seconds = @as(f32, @floatFromInt(ns / 1_000_000_000)) + @as(f32, @floatFromInt(ns % 1_000_000_000)) / @as(f32, 1_000_000_000);
            try t.expectEqual(seconds, core.visual_time);
            const expected = 0.5 * (1 - @exp(-12 * @min(seconds, 0.25)));
            try t.expectApproxEqAbs(expected, core.camera().yaw, 0.000001);
            try t.expectEqual(prefs.MotionPreference.reduced, core.preferences().motion);
            core.setPresentationPreferences(.{ .motion = .reduced });
            try t.expectEqual(core.camera().target_yaw, core.camera().yaw);
            _ = core.advance(1_000_000_000);
            try t.expectEqual(seconds, core.visual_time);
            core.setPresentationPreferences(.{ .motion = .full });
            core.visual_time = 4095.75;
            _ = core.advance(250_000_000);
            try t.expectEqual(@as(f32, 0), core.visual_time);
        }
    }
}
test "G3 session override is immediately effective survives completion clears on failed settings save and UI reset executes" {
    const session = @import("classic_session.zig");
    const store = @import("classic_store.zig");
    var memory = store.MemoryStore.init(t.allocator, .preferences);
    defer memory.deinit();
    const candidate = try scenario.generatedValidated(m.DEFAULT_SEED, .{});
    var value = try session.Session.init(t.allocator, &candidate, memory.raw());
    defer value.deinit();
    _ = value.setPresentationPreferences(.{ .motion = .reduced, .graphics_quality = .low, .high_contrast = true });
    try t.expectEqual(prefs.MotionPreference.reduced, value.core.camera().motion);
    try t.expectEqualDeep(value.presentation(), value.core._presentation);
    _ = value.handleUiAction(.skip_onboarding, memory.raw());
    try t.expectEqual(prefs.MotionPreference.reduced, value.core.camera().motion);
    value.core.orbitCamera(0.3, 0.1);
    value.core.visual_time = 4;
    const reset = value.handleUiAction(.reset_camera, memory.raw());
    try t.expect(reset.effects.reset_camera);
    try t.expectEqual(camera.DEFAULT_YAW_RADIANS, value.core.camera().yaw);
    try t.expectEqual(@as(f32, 4), value.core.visual_time);
    _ = value.handleUiAction(.open_settings, memory.raw());
    try memory.failNextSave("denied");
    const result = value.handleUiAction(.{ .settings = .{ .set_ui_scale = .percent115 } }, memory.raw());
    try t.expectEqual(session.Persistence.failed, result.persistence);
    try t.expectEqualDeep(value.core.preferences().presentation(), value.presentation());
    try t.expectEqualDeep(value.presentation(), value.core._presentation);
    try t.expectEqual(prefs.MotionPreference.full, value.core.camera().motion);
}
test "G3 replacement resets targets retains Full current camera and effective state and invalid replacement is atomic" {
    var core = try ready();
    defer core.deinit();
    core.orbitCamera(0.4, 0.1);
    core._camera.zoom(100);
    core._camera.setViewport(.{ .x = 777, .y = 555 });
    core._camera.target = .{ .x = 1, .y = 0, .z = 2 };
    _ = core.advance(50_000_000);
    core.setPresentationPreferences(.{ .motion = .full, .graphics_quality = .low, .high_contrast = true });
    _ = try core.enqueueGameCommand(.{ .launch = .{ .source = 0, .destination = 3 } });
    _ = try core.enqueueGameCommand(.{ .tune_field = .{ .world = 0, .field = .topology, .adjustment = .decrease } });
    const before = core.camera();
    const raw_time = core.visual_time;
    const authority = core.canonicalFingerprint();
    const pending = try t.allocator.dupe(m.CommandEnvelope, core._simulation.pending.items);
    defer t.allocator.free(pending);
    var invalid = core.activeScenario().*;
    invalid.rules.tick_hz = 0;
    try t.expectError(error.Validation, core.replaceScenario(&invalid));
    try t.expectEqualDeep(before, core.camera());
    try t.expectEqual(raw_time, core.visual_time);
    try t.expectEqual(authority, core.canonicalFingerprint());
    try t.expectEqualDeep(pending, core._simulation.pending.items);
    try t.expectEqual(@as(u64, 2), core.nextSequence());
    const generation = core.advisorySnapshot().?.metadata.request_id;
    try core.restart();
    try t.expectEqual(camera.DEFAULT_YAW_RADIANS, core.camera().target_yaw);
    try t.expectEqual(camera.DEFAULT_PITCH_RADIANS, core.camera().target_pitch);
    try t.expectEqual(camera.DEFAULT_DISTANCE, core.camera().target_distance);
    try t.expectEqual(before.yaw, core.camera().yaw);
    try t.expectEqual(before.distance, core.camera().distance);
    try t.expectEqualDeep(before.logical_viewport, core.camera().logical_viewport);
    try t.expectEqualDeep(before.target, core.camera().target);
    try t.expectEqualDeep(prefs.PresentationPreferences{ .motion = .full, .graphics_quality = .low, .high_contrast = true }, core.presentation());
    try t.expectEqual(@as(f32, 0), core.visual_time);
    try t.expectEqual(@as(usize, 0), core.pendingCommandCount());
    try t.expectEqual(@as(u64, 0), core.nextSequence());
    try t.expectEqual(generation + 1, core.advisorySnapshot().?.metadata.request_id);
}
test "G3 editor open close retain camera and applied reset is consumed at install" {
    const session = @import("classic_session.zig");
    const store = @import("classic_store.zig");
    const tx = @import("classic_editor_transactions.zig");
    var raw = store.MemoryStore.init(t.allocator, .preferences);
    defer raw.deinit();
    var scenario_raw = store.MemoryStore.init(t.allocator, .scenario);
    defer scenario_raw.deinit();
    const candidate = try scenario.generatedValidated(m.DEFAULT_SEED, .{});
    var value = try session.Session.init(t.allocator, &candidate, raw.raw());
    defer value.deinit();
    value.core.orbitCamera(0.3, 0.1);
    const before = value.core.camera();
    try t.expectEqual(@as(?tx.Outcome, .opened), value.openEditor().editor_outcome);
    try t.expectEqualDeep(before, value.core.camera());
    try t.expectEqual(@as(?tx.Outcome, .closed), value.dispatchEditor(.cancel, scenario_raw.raw()).editor_outcome);
    try t.expectEqualDeep(before, value.core.camera());
    _ = value.openEditor();
    const request_id = value.core.advisorySnapshot().?.metadata.request_id;
    try t.expectEqual(@as(?tx.Outcome, .modal_opened), value.dispatchEditor(.apply_and_restart, scenario_raw.raw()).editor_outcome);
    const applied = value.dispatchEditor(.confirm, scenario_raw.raw());
    try t.expectEqual(@as(?tx.Outcome, .applied), applied.editor_outcome);
    try t.expect(applied.effects.reset_camera);
    try t.expect(!value.editor.effects.reset_camera);
    try t.expect(!value.editor.effects.restart_advisory);
    try t.expectEqual(camera.DEFAULT_YAW_RADIANS, value.core.camera().target_yaw);
    try t.expectEqual(before.yaw, value.core.camera().yaw);
    try t.expectEqual(request_id + 1, value.core.advisorySnapshot().?.metadata.request_id);
}
test "G3 scene copies active scenario provenance exact published advisory current camera effective effects and owned populated fleets" {
    const scene = @import("classic_scene.zig");
    try t.expect(@hasDecl(app.AppCore, "sceneFrame"));
    if (comptime @hasDecl(app.AppCore, "sceneFrame")) {
        var core = try ready();
        var live = true;
        defer if (live) core.deinit();
        core.selected_world = 0;
        core.hovered_world = 3;
        core.setPresentationPreferences(.{ .motion = .full, .graphics_quality = .low, .high_contrast = true });
        core.orbitCamera(0.4, 0.1);
        _ = try core.enqueueGameCommand(.{ .launch = .{ .source = 0, .destination = 3 } });
        _ = core.advance(25_000_000);
        const snapshot = core.advisorySnapshot().?.*;
        const saved = core.sceneFrame();
        try t.expect(saved.worlds[0].selected);
        try t.expect(saved.worlds[3].hovered);
        try t.expectEqual(scene.visualSeed(core.activeScenario().fingerprint), saved.background.visual_seed);
        try t.expect(core.activeScenario().fingerprint != core.canonicalFingerprint());
        try t.expectEqual(core.canonicalFingerprint(), snapshot.metadata.source_fingerprint);
        try t.expectEqual(core.campaign().next_tick, snapshot.metadata.source_tick);
        try t.expectEqual(snapshot.priority, saved.advisory.?.priority);
        try t.expectEqual(snapshot.scores[snapshot.priority], saved.advisory.?.score);
        try t.expectEqualDeep(core.camera().current_matrices, saved.camera);
        try t.expectEqual(core.visual_time, saved.visual_time);
        try t.expectEqual(scene.LOW_PARTICLE_BUDGET, saved.effects.particle_budget);
        try t.expect(saved.effects.high_contrast);
        try t.expectEqual(@as(usize, 1), saved.fleets().len);
        try t.expectEqual(@as(usize, 1), saved.routes().len);
        const fleet = saved.fleets()[0];
        const route = saved.routes()[0];
        const worlds = saved.worlds;
        const matrices = saved.camera;
        const advice = saved.advisory;
        var moved = core;
        live = false;
        core = undefined;
        try moved.restart();
        moved.deinit();
        try t.expectEqualDeep(fleet, saved.fleets()[0]);
        try t.expectEqualDeep(route, saved.routes()[0]);
        try t.expectEqualDeep(worlds, saved.worlds);
        try t.expectEqualDeep(matrices, saved.camera);
        try t.expectEqualDeep(advice, saved.advisory);
    }
}
test "G3 logical viewport rejection floor and projected picking preserve authority queue and advisory identities" {
    const math = @import("classic_math.zig");
    try t.expect(@hasDecl(app.AppCore, "setViewport"));
    if (comptime @hasDecl(app.AppCore, "setViewport")) {
        var failing = t.FailingAllocator.init(t.allocator, .{});
        const candidate = try scenario.generatedValidated(m.DEFAULT_SEED, .{});
        var core = try app.AppCore.init(failing.allocator(), &candidate, .{ .onboarding_completed = true });
        defer core.deinit();
        _ = try core.enqueueGameCommand(.{ .launch = .{ .source = 0, .destination = 3 } });
        _ = try core.enqueueGameCommand(.{ .tune_field = .{ .world = 0, .field = .atmosphere, .adjustment = .increase } });
        const pending = try t.allocator.dupe(m.CommandEnvelope, core._simulation.pending.items);
        defer t.allocator.free(pending);
        const campaign = core.campaign();
        const replay = core.replayIdentity();
        const fingerprint = core.canonicalFingerprint();
        const metadata = core.advisorySnapshot().?.metadata;
        failing.fail_index = failing.alloc_index;
        core.setViewport(.{ .x = std.math.nan(f32), .y = 88 });
        try t.expectEqualDeep(math.Vec2{ .x = 1440, .y = 900 }, core.viewport());
        core.setViewport(.{ .x = -5, .y = 0 });
        try t.expectEqualDeep(math.Vec2{ .x = 0, .y = 0 }, core.viewport());
        try t.expectEqualDeep(math.Vec2{ .x = 1, .y = 1 }, core.camera().logical_viewport);
        try t.expectEqual(@as(?m.WorldId, null), core.pickSceneWorld(.{ .x = 0, .y = 0 }, false));
        core.setViewport(.{ .x = 1440, .y = 900 });
        core.setPresentationPreferences(.{ .motion = .reduced });
        core.orbitCamera(0.1, 0.02);
        core.zoomCamera(10);
        core.resetCamera();
        const frame = core.sceneFrame();
        const p = frame.camera.view_projection.mulVec4(.{ .x = frame.worlds[0].position.x, .y = frame.worlds[0].position.y, .z = frame.worlds[0].position.z, .w = 1 });
        const pointer = math.Vec2{ .x = (p.x / p.w + 1) * 720, .y = (1 - p.y / p.w) * 450 };
        try t.expectEqual(@as(?m.WorldId, 0), core.pickSceneWorld(pointer, false));
        try t.expectEqual(@as(?m.WorldId, null), core.pickSceneWorld(pointer, true));
        try t.expectEqualDeep(campaign, core.campaign());
        try t.expectEqualDeep(replay, core.replayIdentity());
        try t.expectEqual(fingerprint, core.canonicalFingerprint());
        try t.expectEqual(@as(u64, 2), core.nextSequence());
        try t.expectEqualDeep(pending, core._simulation.pending.items);
        try t.expectEqualDeep(metadata, core.advisorySnapshot().?.metadata);
    }
}
// Additional qualification after the chunked RED/GREEN witnesses above. These
// are regression coverage, not retroactive claims of test-first development.
test "G3 all holds update presentation once before hold and never request advisory" {
    for (0..5) |kind| {
        var core = try ready();
        defer core.deinit();
        switch (kind) {
            0 => {
                _ = core.setSpeed(.paused);
            },
            1 => core.setMode(.settings),
            2 => core.setOnboardingHold(true),
            3 => core._simulation.state.phase = .{ .finished = .{ .at = 0, .outcome = .player_eliminated } },
            4 => core._failure = .{ .cause = error.OutOfMemory, .tick = 0 },
            else => unreachable,
        }
        core.orbitCamera(0.5, 0.1);
        const metadata = core.advisorySnapshot().?.metadata;
        const speed = core.speed();
        const frame = core.advance(1_000_000_000);
        try t.expectEqual(@as(usize, 0), frame.completed_steps);
        try t.expectEqual(@as(f32, 1), core.visual_time);
        try t.expectApproxEqAbs(@as(f32, 0.5) * (1 - @exp(@as(f32, -3))), core.camera().yaw, 0.000001);
        try t.expectEqual(@as(f32, 0), core.clockAlpha());
        try t.expectEqual(speed, core.speed());
        try t.expectEqualDeep(metadata, core.advisorySnapshot().?.metadata);
        switch (kind) {
            0 => try t.expect(frame.status == .paused),
            1 => try t.expect(frame.status == .mode_hold),
            2 => try t.expect(frame.status == .onboarding_hold),
            3 => try t.expect(frame.status == .gameplay_finished),
            4 => try t.expect(frame.status == .failure_hold),
            else => unreachable,
        }
    }
    // Explicit precedence when all other holds are simultaneously eligible.
    var core = try ready();
    defer core.deinit();
    core.setMode(.settings);
    core.setOnboardingHold(true);
    core._simulation.state.phase = .{ .finished = .{ .at = 0, .outcome = .player_eliminated } };
    core._failure = .{ .cause = error.OutOfMemory, .tick = 0 };
    try t.expect(core.advance(0).status == .failure_hold);
    core._failure = null;
    try t.expect(core.advance(0).status == .mode_hold);
    core.setMode(.playing);
    try t.expect(core.advance(0).status == .onboarding_hold);
    core.setOnboardingHold(false);
    _ = core.setSpeed(.paused);
    try t.expect(core.advance(0).status == .paused);
    _ = core.setSpeed(.normal);
    try t.expect(core.advance(0).status == .gameplay_finished);
}
test "G3 speed passes differ only in gameplay with direct authority parity and zero step no advisory" {
    const sim = @import("simulation.zig");
    const candidate = try scenario.generatedValidated(m.DEFAULT_SEED, .{});
    for ([_]app.GameSpeed{ .normal, .fast, .very_fast }) |speed| {
        var core = try ready();
        defer core.deinit();
        _ = core.setSpeed(speed);
        core.orbitCamera(0.5, 0);
        const metadata = core.advisorySnapshot().?.metadata;
        const zero = core.advance(0);
        try t.expectEqual(@as(usize, 0), zero.completed_steps);
        try t.expectEqualDeep(metadata, core.advisorySnapshot().?.metadata);
        const frame = core.advance(250_000_000);
        try t.expectEqual(@as(usize, 15) * speed.multiplier(), frame.completed_steps);
        try t.expectEqual(@as(f32, 0.25), core.visual_time);
        try t.expectApproxEqAbs(@as(f32, 0.5) * (1 - @exp(@as(f32, -3))), core.camera().yaw, 0.000001);
        var direct = try sim.Simulation.fromScenario(t.allocator, &candidate);
        defer direct.deinit();
        try direct.stepN(@intCast(frame.completed_steps));
        try t.expectEqual(direct.canonicalFingerprint(), core.canonicalFingerprint());
        try t.expectEqualDeep(direct.replayIdentity(), core.replayIdentity());
    }
}
test "G3 actual early and late tick OOM update camera time once and sticky hold updates without advisory or authority mutation" {
    var early = false;
    var late = false;
    for (0..128) |ordinal| {
        var failing = t.FailingAllocator.init(t.allocator, .{});
        const candidate = try scenario.generatedValidated(m.DEFAULT_SEED, .{});
        var core = try app.AppCore.init(failing.allocator(), &candidate, .{ .onboarding_completed = true });
        defer core.deinit();
        _ = try core.enqueueGameCommand(.{ .launch = .{ .source = 0, .destination = 3 } });
        core.orbitCamera(0.5, 0);
        _ = core.setSpeed(.very_fast);
        failing.fail_index = failing.alloc_index + ordinal;
        const frame = core.advance(100_000_000);
        try t.expectEqual(@as(f32, 0.1), core.visual_time);
        try t.expectApproxEqAbs(@as(f32, 0.5) * (1 - @exp(@as(f32, -1.2))), core.camera().yaw, 0.000001);
        if (frame.status == .failure_hold) {
            if (frame.completed_steps == 0) early = true else late = true;
            const metadata = core.advisorySnapshot().?.metadata;
            try t.expect(metadata.source_tick <= frame.completed_steps);
            const fingerprint = core.canonicalFingerprint();
            const pending = try t.allocator.dupe(m.CommandEnvelope, core._simulation.pending.items);
            defer t.allocator.free(pending);
            const before = core.camera().yaw;
            try t.expect(core.advance(1_000_000_000).status == .failure_hold);
            try t.expectEqual(@as(f32, 1.1), core.visual_time);
            try t.expectApproxEqAbs(before + (0.5 - before) * (1 - @exp(@as(f32, -3))), core.camera().yaw, 0.000001);
            try t.expectEqual(fingerprint, core.canonicalFingerprint());
            try t.expectEqualDeep(pending, core._simulation.pending.items);
            try t.expectEqualDeep(metadata, core.advisorySnapshot().?.metadata);
            try t.expectEqual(@as(f32, 0), core.clockAlpha());
            try t.expectEqual(app.GameSpeed.very_fast, core.speed());
        }
        if (early and late) break;
    }
    try t.expect(early and late);
}
test "G3 both opposite durable overrides retain raw time and moved Session keeps one effective owner" {
    const session = @import("classic_session.zig");
    const store = @import("classic_store.zig");
    for ([_]prefs.MotionPreference{ .full, .reduced }) |durable_motion| {
        const durable = prefs.UserPreferencesV1{ .motion = durable_motion, .onboarding_completed = true };
        const bytes = try prefs.encode(t.allocator, durable);
        defer t.allocator.free(bytes);
        var raw = try store.MemoryStore.withSlot(t.allocator, .preferences, bytes);
        defer raw.deinit();
        const candidate = try scenario.generatedValidated(m.DEFAULT_SEED, .{});
        var original = try session.Session.init(t.allocator, &candidate, raw.raw());
        const override_motion: prefs.MotionPreference = if (durable_motion == .full) .reduced else .full;
        _ = original.setPresentationPreferences(.{ .motion = override_motion, .graphics_quality = .low, .high_contrast = true });
        original.core.visual_time = 7;
        original.core.orbitCamera(0.3, 0.1);
        _ = original.core.advance(1_000_000_000);
        try t.expectEqual(@as(f32, if (override_motion == .full) 8 else 7), original.core.visual_time);
        const frame = original.core.sceneFrame();
        try t.expectEqual(@as(f32, if (override_motion == .full) 8 else 0), frame.visual_time);
        try t.expectEqual(@as(u32, if (override_motion == .full) 96 else 0), frame.effects.particle_budget);
        try t.expect(frame.effects.high_contrast);
        try t.expect(!frame.effects.bloom_requested);
        var moved = original;
        original = undefined;
        defer moved.deinit();
        try t.expectEqualDeep(moved.presentation(), moved.core.presentation());
        try t.expectEqual(override_motion, moved.core.camera().motion);
        try t.expectEqual(durable_motion, moved.core.preferences().motion);
        _ = moved.setPresentationPreferences(.{ .motion = .full });
        _ = moved.core.advance(250_000_000);
        try t.expectEqual(@as(f32, if (override_motion == .full) 8.25 else 7.25), moved.core.visual_time);
    }
}
test "G3 UI reset full current and clock retained while Reduced snaps with no advisory work" {
    const session = @import("classic_session.zig");
    const store = @import("classic_store.zig");
    for ([_]prefs.MotionPreference{ .full, .reduced }) |motion| {
        var raw = store.MemoryStore.init(t.allocator, .preferences);
        defer raw.deinit();
        const candidate = try scenario.generatedValidated(m.DEFAULT_SEED, .{});
        var value = try session.Session.init(t.allocator, &candidate, raw.raw());
        defer value.deinit();
        _ = value.handleUiAction(.skip_onboarding, raw.raw());
        _ = value.setPresentationPreferences(.{ .motion = motion });
        value.core.orbitCamera(0.3, 0.1);
        _ = value.core.advance(50_000_000);
        value.core._clock.pushFrame(1_000_000);
        const before = value.core.camera();
        const alpha = value.core.clockAlpha();
        const time = value.core.visual_time;
        const request = value.core.advisorySnapshot().?.metadata;
        try t.expect(value.handleUiAction(.reset_camera, raw.raw()).effects.reset_camera);
        try t.expectEqual(camera.DEFAULT_YAW_RADIANS, value.core.camera().target_yaw);
        try t.expectEqual(if (motion == .full) before.yaw else camera.DEFAULT_YAW_RADIANS, value.core.camera().yaw);
        try t.expectEqual(alpha, value.core.clockAlpha());
        try t.expectEqual(time, value.core.visual_time);
        try t.expectEqualDeep(request, value.core.advisorySnapshot().?.metadata);
    }
}
test "G3 failed editor apply retains effective camera time queued contents and session queries are authority isolated" {
    const session = @import("classic_session.zig");
    const store = @import("classic_store.zig");
    const tx = @import("classic_editor_transactions.zig");
    var raw = store.MemoryStore.init(t.allocator, .preferences);
    defer raw.deinit();
    var scenario_raw = store.MemoryStore.init(t.allocator, .scenario);
    defer scenario_raw.deinit();
    const candidate = try scenario.generatedValidated(m.DEFAULT_SEED, .{});
    var value = try session.Session.init(t.allocator, &candidate, raw.raw());
    defer value.deinit();
    _ = value.handleUiAction(.skip_onboarding, raw.raw());
    _ = try value.queueGameCommand(.{ .launch = .{ .source = 0, .destination = 3 } });
    _ = try value.queueGameCommand(.{ .tune_field = .{ .world = 0, .field = .hydrosphere, .adjustment = .decrease } });
    const pending = try t.allocator.dupe(m.CommandEnvelope, value.core._simulation.pending.items);
    defer t.allocator.free(pending);
    const campaign = value.core.campaign();
    const replay = value.core.replayIdentity();
    const fingerprint = value.core.canonicalFingerprint();
    const metadata = value.core.advisorySnapshot().?.metadata;
    _ = try value.cursorHover(3, false);
    _ = try value.selectSceneWorld(0, true);
    const controls = value.controls(0);
    _ = controls.items();
    _ = value.presentation();
    _ = value.core.sceneFrame();
    _ = value.setPresentationPreferences(.{ .motion = .full, .high_contrast = true });
    value.core.orbitCamera(0.3, 0.1);
    value.core.visual_time = 9;
    const camera_before = value.core.camera();
    _ = value.openEditor();
    _ = value.dispatchEditor(.{ .select_section = .rules }, scenario_raw.raw());
    try value.editor.session.?.focusField("scenario.seed");
    try value.editor.session.?.replaceFocusedText("invalid seed");
    const failed = value.dispatchEditor(.apply_and_restart, scenario_raw.raw());
    try t.expectEqual(@as(?tx.Outcome, .failed), failed.editor_outcome);
    try t.expectEqualDeep(session.Effects{}, failed.effects);
    try t.expect(value.editor.action_failure.? == .invalid_text);
    try t.expectEqualDeep(camera_before, value.core.camera());
    try t.expectEqual(@as(f32, 9), value.core.visual_time);
    try t.expectEqualDeep(prefs.PresentationPreferences{ .motion = .full, .high_contrast = true }, value.presentation());
    try t.expectEqualDeep(campaign, value.core.campaign());
    try t.expectEqualDeep(replay, value.core.replayIdentity());
    try t.expectEqual(fingerprint, value.core.canonicalFingerprint());
    try t.expectEqualDeep(pending, value.core._simulation.pending.items);
    try t.expectEqual(@as(u64, 2), value.core.nextSequence());
    try t.expectEqualDeep(metadata, value.core.advisorySnapshot().?.metadata);
}
