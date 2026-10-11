const std = @import("std");
const t = std.testing;
const s = @import("classic_session.zig");
const store = @import("classic_store.zig");
const prefs = @import("preferences.zig");
const m = @import("model.zig");
const scenario = @import("scenario.zig");
const app = @import("classic_app.zig");
const o = @import("classic_onboarding.zig");
const tx = @import("classic_editor_transactions.zig");
fn candidate() !scenario.ScenarioV1 {
    return scenario.generatedValidated(m.DEFAULT_SEED, .{});
}
fn init(allocator: std.mem.Allocator, raw: store.RawStore) !s.Session {
    const value = try candidate();
    return s.Session.init(allocator, &value, raw);
}
fn memory(value: prefs.UserPreferencesV1) !store.MemoryStore {
    const bytes = try prefs.encode(t.allocator, value);
    defer t.allocator.free(bytes);
    return store.MemoryStore.withSlot(t.allocator, .preferences, bytes);
}
fn act(session: *s.Session, raw: store.RawStore, action: s.UiAction) s.Result {
    return session.handleUiAction(action, raw);
}
// Catches ignoring persisted completion, startup autosave and corrupt-record repair.
test "Classic3E session startup defaults or loads completion without saving" {
    var absent = store.MemoryStore.init(t.allocator, .preferences);
    defer absent.deinit();
    var fresh = try init(t.allocator, absent.raw());
    defer fresh.deinit();
    try t.expectEqual(app.GameSpeed.paused, fresh.core.speed());
    try t.expectEqual(app.GameSpeed.normal, fresh.core.resumeSpeed());
    try t.expect(fresh.onboarding.active());
    try t.expectEqual(@as(usize, 1), absent.load_calls);
    try t.expectEqual(@as(usize, 0), absent.save_calls);
    var stored = try memory(.{ .onboarding_completed = true, .ui_scale = .percent130, .motion = .reduced, .graphics_quality = .high, .high_contrast = true });
    defer stored.deinit();
    var loaded = try init(t.allocator, stored.raw());
    defer loaded.deinit();
    try t.expectEqual(app.GameSpeed.normal, loaded.core.speed());
    try t.expect(!loaded.onboarding.active());
    try t.expectEqual(prefs.UiScale.percent130, loaded.core.preferences().ui_scale);
    try t.expectEqual(@as(usize, 0), stored.save_calls);
    const invalid = [_][]const u8{
        "broken",
        "{\"format_version\":2,\"ui_scale_percent\":100,\"motion\":\"FULL\",\"high_contrast\":false,\"graphics_quality\":\"AUTO\",\"onboarding_completed\":true}",
        "{\"format_version\":1,\"ui_scale_percent\":99,\"motion\":\"FULL\",\"high_contrast\":false,\"graphics_quality\":\"AUTO\",\"onboarding_completed\":true}",
    };
    for (invalid, 0..) |bytes, i| {
        var bad = try store.MemoryStore.withSlot(t.allocator, .preferences, bytes);
        defer bad.deinit();
        var recovered = try init(t.allocator, bad.raw());
        defer recovered.deinit();
        try t.expectEqual(prefs.UserPreferencesV1{}, recovered.core.preferences());
        const failure = recovered.preference_failure orelse return error.ExpectedFailure;
        try t.expect(failure == .codec);
        try t.expectEqual(i == 0, failure.codec == .json);
        try t.expectEqual(i == 1, failure.codec == .version);
        try t.expectEqual(i == 2, failure.codec == .ui_scale);
        try t.expectEqualStrings(bytes, bad.slot.?);
        try t.expectEqual(@as(usize, 0), bad.save_calls);
    }
}
// Catches default-outcome overwriting live settings, error loss and post-save formatting.
test "Classic3E settings saves every accepted action retains live values on failure" {
    var raw = try memory(.{ .onboarding_completed = true });
    defer raw.deinit();
    var session = try init(t.allocator, raw.raw());
    defer session.deinit();
    const fp = session.core.canonicalFingerprint();
    const before = try t.allocator.dupe(u8, raw.slot.?);
    defer t.allocator.free(before);
    session.hud_focus = .launch;
    session.settings.focus(9);
    try t.expect(act(&session, raw.raw(), .open_settings).handled);
    try t.expectEqual(app.AppMode.settings, session.core.mode());
    try t.expectEqual(@as(usize, 0), session.settings.focusIndex());
    try t.expectEqual(@as(?s.UiAction, null), session.hud_focus);
    try raw.failNextSave("quota denied");
    const failed = act(&session, raw.raw(), .{ .settings = .{ .set_ui_scale = .percent115 } });
    try t.expect(failed.handled);
    try t.expectEqual(s.Persistence.failed, failed.persistence);
    try t.expectEqual(prefs.UiScale.percent115, session.core.preferences().ui_scale);
    try t.expectEqualStrings(before, raw.slot.?);
    try t.expectEqualStrings("quota denied", session.preference_failure.?.store.message.?);
    session.general_failure = .{ .command = error.MissingSource };
    try t.expect(session.recoveryFailure().? == .general);
    try t.expectEqualStrings("quota denied", session.preference_failure.?.store.message.?);
    const accepted = [_]s.UiAction{
        .{ .settings = .{ .set_ui_scale = .percent115 } },
        .{ .settings = .{ .set_ui_scale = .percent85 } },
        .{ .settings = .{ .set_ui_scale = .percent100 } },
        .{ .settings = .{ .set_ui_scale = .percent130 } },
        .{ .settings = .{ .set_motion = .reduced } },
        .{ .settings = .{ .set_high_contrast = true } },
        .{ .settings = .{ .set_graphics_quality = .low } },
        .{ .settings = .{ .set_graphics_quality = .high } },
        .{ .settings = .{ .set_graphics_quality = .auto } },
    };
    for (accepted, 0..) |action, i| {
        const result = act(&session, raw.raw(), action);
        try t.expectEqual(s.Persistence.saved, result.persistence);
        try t.expect(result.effects.presentation_changed);
        try t.expectEqual(@as(usize, i + 2), raw.save_calls);
        try t.expectEqual(@as(?store.PreferencesFailure, null), session.preference_failure);
        try t.expect(session.recoveryFailure().? == .general);
    }
    try t.expectEqual(prefs.UiScale.percent130, session.core.preferences().ui_scale);
    try t.expectEqual(prefs.PresentationPreferences{ .motion = .reduced, .high_contrast = true, .graphics_quality = .auto }, session.presentation());
    const decoded = try prefs.decode(t.allocator, raw.slot.?);
    try t.expectEqual(session.core.preferences(), decoded);
    session.settings.focus(5);
    session.hud_focus = .toggle_help;
    const count = raw.save_calls;
    try t.expect(act(&session, raw.raw(), .{ .settings = .close }).handled);
    try t.expectEqual(count, raw.save_calls);
    try t.expectEqual(@as(usize, 5), session.settings.focusIndex());
    try t.expect(std.meta.eql(s.UiAction.toggle_help, session.hud_focus.?));
    try t.expectEqual(app.AppMode.playing, session.core.mode());
    try t.expectEqual(fp, session.core.canonicalFingerprint());
    try t.expect(!act(&session, raw.raw(), .{ .settings = .{ .set_ui_scale = .percent85 } }).handled);
    try t.expectEqual(count, raw.save_calls);
}
// Catches failure ownership not surviving move, OOM during replacement erasing old detail.
test "Classic3E preference denial ownership survives moves and OOM replacement" {
    var raw = store.MemoryStore.init(t.allocator, .preferences);
    defer raw.deinit();
    try raw.denyLoad("read denied");
    var original = try init(t.allocator, raw.raw());
    var moved = original;
    original = undefined;
    defer moved.deinit();
    try t.expect(moved.preference_failure != null);
    try t.expectEqualStrings("read denied", moved.preference_failure.?.store.message.?);
    try t.expect(moved.recoveryFailure().? == .preferences);
    _ = act(&moved, raw.raw(), .open_settings);
    var failing = std.testing.FailingAllocator.init(t.allocator, .{});
    const v = try candidate();
    var session = try s.Session.init(failing.allocator(), &v, raw.raw());
    defer session.deinit();
    _ = act(&session, raw.raw(), .open_settings);
    try raw.failNextSave("write denied");
    failing.fail_index = failing.alloc_index;
    const result = act(&session, raw.raw(), .{ .settings = .{ .set_high_contrast = true } });
    try t.expectEqual(s.Persistence.resource_error, result.persistence);
    try t.expect(session.core.preferences().high_contrast);
    try t.expectEqualStrings("read denied", session.preference_failure.?.store.message.?);
    try t.expectEqual(@as(?std.mem.Allocator.Error, error.OutOfMemory), session.resource_errors.preferences);
    failing.fail_index = std.math.maxInt(usize);
    const typed = act(&session, raw.raw(), .{ .settings = .{ .set_motion = .reduced } });
    try t.expectEqual(s.Persistence.failed, typed.persistence);
    try t.expectEqualStrings("write denied", session.preference_failure.?.store.message.?);
}
fn toEditorStep(session: *s.Session, raw: store.RawStore) !void {
    _ = try session.primaryTap(0, true);
    _ = try session.primaryTap(3, true);
    try t.expect(act(session, raw, .launch).handled);
    try t.expect(act(session, raw, .increase_field).handled);
    try t.expect(act(session, raw, .acknowledge_advisory).handled);
    try t.expectEqual(@as(?o.Step, .open_scenario_editor), session.onboarding.step());
}
// Catches frame polling instead of immediate 3B consumption or tick-acceptance gating.
test "Classic3E real first-run intent completes before editor hold with tick zero" {
    var raw = store.MemoryStore.init(t.allocator, .preferences);
    defer raw.deinit();
    var session = try init(t.allocator, raw.raw());
    defer session.deinit();
    const fp = session.core.canonicalFingerprint();
    try t.expectEqual(app.SpeedSelection.onboarding_rejected, session.core.setSpeed(.very_fast));
    try t.expectEqual(@as(usize, 0), session.core.advance(250_000_000).completed_steps);
    session.hud_focus = .launch;
    _ = try session.selectSceneWorld(0, false);
    try t.expectEqual(@as(?o.Step, .select_union_world), session.onboarding.step());
    try t.expectEqual(@as(?s.UiAction, null), session.hud_focus);
    try toEditorStep(&session, raw.raw());
    try t.expectEqual(@as(usize, 0), raw.save_calls);
    try t.expectEqual(@as(u64, 0), session.core.campaign().next_tick);
    try t.expectEqual(@as(usize, 2), session.core.pendingCommandCount());
    try t.expectEqual(@as(u64, 2), session.core.nextSequence());
    const envelopes = try t.allocator.dupe(m.CommandEnvelope, session.core._simulation.pending.items);
    defer t.allocator.free(envelopes);
    session.hud_focus = .open_scenario;
    const completed = act(&session, raw.raw(), .open_scenario);
    try t.expectEqual(s.Persistence.saved, completed.persistence);
    try t.expectEqual(@as(?tx.Outcome, .opened), completed.editor_outcome);
    try t.expect(completed.effects.clear_context_focus);
    try t.expect(!completed.effects.reset_camera and !completed.effects.restart_advisory);
    try t.expect(!session.onboarding.active() and session.core.preferences().onboarding_completed);
    try t.expect(!session.core.onboardingHeld());
    try t.expectEqual(app.GameSpeed.normal, session.core.speed());
    try t.expectEqual(app.AppMode.editing_scenario, session.core.mode());
    try t.expect(std.meta.eql(s.UiAction.open_scenario, session.hud_focus.?));
    try t.expectEqualSlices(m.CommandEnvelope, envelopes, session.core._simulation.pending.items);
    try t.expectEqual(fp, session.core.canonicalFingerprint());
    try t.expectEqual(@as(usize, 0), session.core.advance(250_000_000).completed_steps);
    const inactive = act(&session, raw.raw(), .skip_onboarding);
    try t.expect(inactive.handled);
    try t.expectEqual(s.Persistence.not_requested, inactive.persistence);
    try t.expectEqual(@as(usize, 1), raw.save_calls);
    var scenario_store = store.MemoryStore.init(t.allocator, .scenario);
    defer scenario_store.deinit();
    const closed = session.dispatchEditor(.cancel, scenario_store.raw());
    try t.expectEqual(@as(?tx.Outcome, .closed), closed.editor_outcome);
    try t.expect(std.meta.eql(s.UiAction.open_scenario, session.hud_focus.?));
    try t.expectEqual(@as(usize, 0), session.core.advance(0).completed_steps);
    try t.expectEqual(@as(usize, 1), raw.save_calls);
}
// Catches failed/invalid enqueue observing guidance, duplicate sequence consumption and polling old selection.
test "Classic3E command failures and read-only projections cannot advance guidance" {
    var raw = store.MemoryStore.init(t.allocator, .preferences);
    defer raw.deinit();
    var failing = t.FailingAllocator.init(t.allocator, .{});
    var session = try init(failing.allocator(), raw.raw());
    defer session.deinit();
    _ = try session.primaryTap(0, true);
    _ = try session.cursorHover(3, false);
    try t.expectEqual(@as(?o.Step, .launch_fleet), session.onboarding.step());
    failing.fail_index = failing.alloc_index;
    try t.expectError(error.OutOfMemory, session.queueGameCommand(.{ .launch = .{ .source = 0, .destination = 3 } }));
    try t.expectEqual(@as(?o.Step, .launch_fleet), session.onboarding.step());
    try t.expectEqual(@as(usize, 0), session.core.pendingCommandCount());
    try t.expectEqual(@as(u64, 0), session.core.nextSequence());
    failing.fail_index = std.math.maxInt(usize);
    const invalid = try session.queueGameCommand(.{ .launch = .{ .source = 0, .destination = 0 } });
    try t.expect(!invalid.queued.?.preview_valid);
    try t.expectEqual(@as(?o.Step, .launch_fleet), session.onboarding.step());
    try t.expectEqual(@as(u64, 1), session.core.nextSequence());
    session.core._next_sequence = 0;
    try t.expectError(error.DuplicateSequence, session.queueGameCommand(.{ .launch = .{ .source = 0, .destination = 3 } }));
    try t.expectEqual(@as(?o.Step, .launch_fleet), session.onboarding.step());
    try t.expectEqual(@as(usize, 1), session.core.pendingCommandCount());
    try t.expectEqual(@as(usize, 0), raw.save_calls);
}
// Catches observing direct open and stale lifecycle effects replayed after unrelated editor actions.
test "Classic3E direct editor open does not observe and effects bind to this outcome" {
    var raw = store.MemoryStore.init(t.allocator, .preferences);
    defer raw.deinit();
    var scenario_store = store.MemoryStore.init(t.allocator, .scenario);
    defer scenario_store.deinit();
    var session = try init(t.allocator, raw.raw());
    defer session.deinit();
    try toEditorStep(&session, raw.raw());
    session.hud_focus = .reset_camera;
    try t.expectEqual(@as(?tx.Outcome, .opened), session.openEditor().editor_outcome);
    try t.expectEqual(@as(?o.Step, .open_scenario_editor), session.onboarding.step());
    try t.expectEqual(@as(usize, 0), raw.save_calls);
    try t.expect(session.core.onboardingHeld());
    const absent = session.dispatchEditor(.load, scenario_store.raw());
    try t.expectEqual(@as(?tx.Outcome, .absent), absent.editor_outcome);
    try t.expectEqual(s.Effects{}, absent.effects);
    try t.expect(std.meta.eql(s.UiAction.reset_camera, session.hud_focus.?));
    const missing = session.dispatchEditor(.confirm, scenario_store.raw());
    try t.expectEqual(@as(?tx.Outcome, .failed), missing.editor_outcome);
    try t.expect(session.editor.action_failure.? == .no_confirmation);
    try t.expect(session.editor.status == .no_saved_scenario);
    try t.expectEqual(s.Effects{}, missing.effects);
    _ = session.dispatchEditor(.cancel, scenario_store.raw());
    try t.expect(session.core.onboardingHeld());
    try t.expect(std.meta.eql(s.UiAction.reset_camera, session.hud_focus.?));
}
fn markFinal(session: *s.Session) void {
    for ([_]o.Event{ .union_world_selected, .launch_previewed, .fleet_launched, .field_tuned, .advisory_read }) |event| _ = session.onboarding.observe(event);
}
// Real store callback arms the allocator only AFTER atomic MemoryStore save succeeds.
// This exposes the completion/persist/editor boundary without fabricating an OOM.
const ArmEditorAllocator = struct {
    memory_store: *store.MemoryStore,
    failing: *t.FailingAllocator,
    fn raw(self: *@This()) store.RawStore {
        return .{ .record = .preferences, .context = self, .record_fn = record, .load_fn = load, .save_fn = save };
    }
    fn record(_: *anyopaque) store.Record {
        return .preferences;
    }
    fn load(ctx: *anyopaque, allocator: std.mem.Allocator) std.mem.Allocator.Error!store.LoadOutcome {
        const self: *@This() = @ptrCast(@alignCast(ctx));
        return self.memory_store.raw().load(allocator);
    }
    fn save(ctx: *anyopaque, allocator: std.mem.Allocator, bytes: []const u8) std.mem.Allocator.Error!store.SaveOutcome {
        const self: *@This() = @ptrCast(@alignCast(ctx));
        const result = try self.memory_store.raw().save(allocator, bytes);
        if (result == .saved) self.failing.fail_index = self.failing.alloc_index;
        return result;
    }
};
// Catches an incidental try on persistence stopping editor preparation, or flattening mixed success.
test "Classic3E completion preference-store OOM still opens editor and exposes both results" {
    var failing_store = t.FailingAllocator.init(t.allocator, .{ .fail_index = 0 });
    var raw = store.MemoryStore.init(failing_store.allocator(), .preferences);
    defer raw.deinit();
    var session = try init(t.allocator, raw.raw());
    defer session.deinit();
    _ = try session.queueGameCommand(.{ .launch = .{ .source = 0, .destination = 3 } });
    const fp = session.core.canonicalFingerprint();
    markFinal(&session);
    session.hud_focus = .open_scenario;
    const result = act(&session, raw.raw(), .open_scenario);
    try t.expectEqual(s.Persistence.resource_error, result.persistence);
    try t.expectEqual(@as(?tx.Outcome, .opened), result.editor_outcome);
    try t.expectEqual(@as(?anyerror, null), result.editor_error);
    try t.expectEqual(@as(?std.mem.Allocator.Error, error.OutOfMemory), session.resource_errors.preferences);
    try t.expect(session.recoveryFailure().? == .resource);
    try t.expect(session.core.preferences().onboarding_completed and !session.onboarding.active());
    try t.expect(!session.core.onboardingHeld());
    try t.expectEqual(app.GameSpeed.normal, session.core.speed());
    try t.expectEqual(app.AppMode.editing_scenario, session.core.mode());
    try t.expect(std.meta.eql(s.UiAction.open_scenario, session.hud_focus.?));
    try t.expectEqual(@as(usize, 1), raw.save_calls);
    try t.expectEqual(@as(?[]u8, null), raw.slot);
    try t.expectEqual(fp, session.core.canonicalFingerprint());
    try t.expectEqual(@as(usize, 1), session.core.pendingCommandCount());
    try t.expectEqual(@as(u64, 1), session.core.nextSequence());
    try t.expectEqual(@as(u64, 0), session.core._simulation.pending.items[0].sequence);
    try t.expectEqual(m.GameCommand{ .launch = .{ .source = 0, .destination = 3 } }, session.core._simulation.pending.items[0].command);
}
// Catches whole-session rollback after a persisted completion and losing simultaneous failures.
test "Classic3E editor-only and combined completion OOM preserve completed live state" {
    for ([_]bool{ false, true }) |both| {
        var raw = store.MemoryStore.init(t.allocator, .preferences);
        defer raw.deinit();
        var failing = t.FailingAllocator.init(t.allocator, .{});
        var session = try init(failing.allocator(), raw.raw());
        defer session.deinit();
        _ = try session.queueGameCommand(.{ .launch = .{ .source = 0, .destination = 3 } });
        const fp = session.core.canonicalFingerprint();
        markFinal(&session);
        session.hud_focus = .open_scenario;
        var arm: ArmEditorAllocator = .{ .memory_store = &raw, .failing = &failing };
        if (both) failing.fail_index = failing.alloc_index;
        const result = act(&session, if (both) raw.raw() else arm.raw(), .open_scenario);
        try t.expectEqual(if (both) s.Persistence.resource_error else .saved, result.persistence);
        try t.expectEqual(@as(?tx.Outcome, null), result.editor_outcome);
        try t.expectEqual(@as(?anyerror, error.OutOfMemory), result.editor_error);
        try t.expectEqual(@as(?anyerror, error.OutOfMemory), session.resource_errors.editor);
        try t.expectEqual(both, session.resource_errors.preferences != null);
        try t.expect(session.core.preferences().onboarding_completed and !session.onboarding.active());
        try t.expect(!session.core.onboardingHeld());
        try t.expectEqual(app.GameSpeed.normal, session.core.speed());
        try t.expectEqual(app.AppMode.playing, session.core.mode());
        try t.expect(std.meta.eql(s.UiAction.open_scenario, session.hud_focus.?));
        try t.expectEqual(fp, session.core.canonicalFingerprint());
        try t.expectEqual(@as(usize, 1), session.core.pendingCommandCount());
        try t.expectEqual(@as(u64, 1), session.core.nextSequence());
        try t.expectEqual(m.GameCommand{ .launch = .{ .source = 0, .destination = 3 } }, session.core._simulation.pending.items[0].command);
        if (!both) try t.expect((try prefs.decode(t.allocator, raw.slot.?)).onboarding_completed);
        failing.fail_index = std.math.maxInt(usize);
        try t.expectEqual(@as(?tx.Outcome, .opened), session.openEditor().editor_outcome);
        try t.expectEqual(@as(?anyerror, null), session.resource_errors.editor);
        try t.expectEqual(both, session.resource_errors.preferences != null);
    }
}
// Catches treating a recoverable store failure as a stop before editor open.
test "Classic3E completion typed preference failure still opens editor" {
    var raw = store.MemoryStore.init(t.allocator, .preferences);
    defer raw.deinit();
    var session = try init(t.allocator, raw.raw());
    defer session.deinit();
    markFinal(&session);
    try raw.failNextSave("cannot persist");
    const result = act(&session, raw.raw(), .open_scenario);
    try t.expectEqual(s.Persistence.failed, result.persistence);
    try t.expectEqual(@as(?tx.Outcome, .opened), result.editor_outcome);
    try t.expectEqualStrings("cannot persist", session.preference_failure.?.store.message.?);
    try t.expect(session.core.preferences().onboarding_completed);
}
fn expectActions(got: []const s.Control, want: []const s.UiAction) !void {
    try t.expectEqual(want.len, got.len);
    for (got, want) |control, action| try t.expect(std.meta.eql(action, control.action));
}
// Catches dropped/retargeted identity when optional controls are inserted or hidden.
test "Classic3E literal Playing controls and typed HUD identity survive pure rebuilds" {
    var raw = store.MemoryStore.init(t.allocator, .preferences);
    defer raw.deinit();
    var session = try init(t.allocator, raw.raw());
    defer session.deinit();
    session.core.help_visible = true;
    var full = session.controls(0);
    const want = [_]s.UiAction{ .speed_down, .toggle_pause, .speed_up, .launch, .clear_selection, .reset_camera, .open_settings, .toggle_help, .open_scenario, .skip_onboarding, .{ .select_field = .atmosphere }, .{ .select_field = .hydrosphere }, .{ .select_field = .topology }, .decrease_field, .increase_field, .acknowledge_advisory, .reset_onboarding, .{ .select_guidance_world = 0 } };
    try expectActions(full.items(), &want);
    for (full.items()[0..5]) |control| try t.expect(!control.enabled);
    try t.expect(!full.items()[13].enabled and !full.items()[14].enabled);
    session.focus(12, full.items());
    try t.expect(std.meta.eql(s.UiAction{ .select_field = .topology }, session.hud_focus.?));
    session.core.help_visible = false;
    _ = act(&session, raw.raw(), .skip_onboarding);
    var smaller = session.controls(null);
    const fewer = [_]s.UiAction{ .speed_down, .toggle_pause, .speed_up, .launch, .clear_selection, .reset_camera, .open_settings, .toggle_help, .open_scenario, .{ .select_field = .atmosphere }, .{ .select_field = .hydrosphere }, .{ .select_field = .topology }, .decrease_field, .increase_field, .acknowledge_advisory };
    try expectActions(smaller.items(), &fewer);
    try t.expectEqual(@as(?usize, 11), session.focusIndex(smaller.items()));
    session.hud_focus = .skip_onboarding;
    for (0..20) |_| {
        smaller = session.controls(null);
        try t.expectEqual(@as(?usize, null), session.focusIndex(smaller.items()));
    }
    try t.expect(std.meta.eql(s.UiAction.skip_onboarding, session.hud_focus.?));
    session.focus(std.math.maxInt(usize), smaller.items());
    try t.expect(std.meta.eql(s.UiAction.skip_onboarding, session.hud_focus.?));
    try t.expect(!session.activateFocused(smaller.items(), raw.raw()).handled);
    try t.expectEqual(@as(?s.UiAction, null), session.hud_focus);
    try t.expectEqual(@as(usize, 1), raw.save_calls);
    session.hud_focus = .{ .select_guidance_world = 0 };
    const other = [_]s.Control{.{ .action = .{ .select_guidance_world = 1 } }};
    try t.expectEqual(@as(?usize, null), session.focusIndex(&other));
    session.hud_focus = .{ .settings = .{ .set_high_contrast = true } };
    const changed = [_]s.Control{.{ .action = .{ .settings = .{ .set_high_contrast = false } } }};
    try t.expectEqual(@as(?usize, null), session.focusIndex(&changed));
}
// Catches skipping disabled entries, missing-identity off-by-one and failed Settings activation reset.
test "Classic3E focus navigation includes disabled controls and Settings keeps its index" {
    var raw = try memory(.{ .onboarding_completed = true });
    defer raw.deinit();
    var session = try init(t.allocator, raw.raw());
    defer session.deinit();
    const controls = [_]s.Control{ .{ .action = .launch, .enabled = false }, .{ .action = .toggle_help }, .{ .action = .open_settings } };
    session.focusNext(false, &controls);
    try t.expectEqual(@as(?usize, 0), session.focusIndex(&controls));
    try t.expect(!session.activateFocused(&controls, raw.raw()).handled);
    try t.expectEqual(@as(?s.UiAction, null), session.hud_focus);
    session.focusNext(true, &controls);
    try t.expectEqual(@as(?usize, 2), session.focusIndex(&controls));
    session.focusNext(true, &controls);
    try t.expectEqual(@as(?usize, 1), session.focusIndex(&controls));
    try t.expect(session.activateFocused(&controls, raw.raw()).handled);
    try t.expect(session.core.help_visible);
    session.focusNext(false, &controls);
    try t.expect(session.activateFocused(&controls, raw.raw()).handled);
    var settings = session.controls(null);
    const want = [_]s.UiAction{ .{ .settings = .{ .set_ui_scale = .percent85 } }, .{ .settings = .{ .set_ui_scale = .percent100 } }, .{ .settings = .{ .set_ui_scale = .percent115 } }, .{ .settings = .{ .set_ui_scale = .percent130 } }, .{ .settings = .{ .set_motion = .reduced } }, .{ .settings = .{ .set_high_contrast = true } }, .{ .settings = .{ .set_graphics_quality = .auto } }, .{ .settings = .{ .set_graphics_quality = .low } }, .{ .settings = .{ .set_graphics_quality = .high } }, .{ .settings = .reset_onboarding }, .{ .settings = .close } };
    try expectActions(settings.items(), &want);
    session.focus(4, settings.items());
    try t.expect(session.activateFocused(settings.items(), raw.raw()).handled);
    settings = session.controls(null);
    try t.expectEqual(@as(?usize, 4), session.focusIndex(settings.items()));
    try t.expect(std.meta.eql(s.UiAction{ .settings = .{ .set_motion = .full } }, settings.items()[4].action));
    settings.values[4].enabled = false;
    try t.expect(!session.activateFocused(settings.items(), raw.raw()).handled);
    try t.expectEqual(@as(usize, 4), session.settings.focusIndex());
    session.focusNext(true, &.{});
    try t.expectEqual(@as(usize, 3), session.settings.focusIndex());
    session.focus(999, &.{});
    try t.expectEqual(@as(usize, 10), session.settings.focusIndex());
    _ = act(&session, raw.raw(), .{ .settings = .close });
    session.hud_focus = .launch;
    session.focusNext(false, &.{});
    try t.expectEqual(@as(?s.UiAction, null), session.hud_focus);
}
// Catches over-broad Playing gates and conflating direct clear with keyboard/helper clearing.
test "Classic3E direct action guards preserve source exceptions and guidance ownership" {
    var raw = store.MemoryStore.init(t.allocator, .preferences);
    defer raw.deinit();
    var session = try init(t.allocator, raw.raw());
    defer session.deinit();
    session.hud_focus = .reset_camera;
    try t.expect(!act(&session, raw.raw(), .{ .select_guidance_world = 1 }).handled);
    try t.expect(!act(&session, raw.raw(), .{ .select_guidance_world = 255 }).handled);
    try t.expectEqual(@as(?u8, null), session.core.selected_world);
    try t.expect(act(&session, raw.raw(), .{ .select_guidance_world = 0 }).handled);
    try t.expectEqual(@as(?u8, 0), session.core.selected_world);
    try t.expectEqual(@as(?u8, 0), session.core.hovered_world);
    try t.expect(std.meta.eql(s.UiAction.reset_camera, session.hud_focus.?));
    try t.expect(!act(&session, raw.raw(), .{ .select_guidance_world = 0 }).handled);
    _ = act(&session, raw.raw(), .open_settings);
    session.hud_focus = .launch;
    session.core.touch_command_target = 3;
    session.clearUiFocus();
    try t.expect(std.meta.eql(s.UiAction.launch, session.hud_focus.?));
    try t.expect(!session.cancel().handled);
    try t.expect(act(&session, raw.raw(), .clear_selection).handled);
    try t.expectEqual(@as(?s.UiAction, null), session.hud_focus);
    try t.expectEqual(@as(?u8, null), session.core.selected_world);
    try t.expectEqual(@as(?u8, null), session.core.hovered_world);
    try t.expectEqual(@as(?u8, null), session.core.touch_command_target);
    for ([_]s.UiAction{ .speed_down, .toggle_pause, .speed_up, .reset_camera, .toggle_help, .open_scenario, .{ .select_field = .topology } }) |action| try t.expect(!act(&session, raw.raw(), action).handled);
    session.onboarding.reset();
    for ([_]o.Event{ .union_world_selected, .launch_previewed, .fleet_launched, .field_tuned }) |event| _ = session.onboarding.observe(event);
    try t.expect(act(&session, raw.raw(), .acknowledge_advisory).handled);
    try t.expectEqual(@as(?o.Step, .open_scenario_editor), session.onboarding.step());
    _ = act(&session, raw.raw(), .{ .settings = .close });
    session.hud_focus = .launch;
    session.clearUiFocus();
    try t.expectEqual(@as(?s.UiAction, null), session.hud_focus);
    session.hud_focus = .launch;
    try t.expect(session.cancel().handled);
    try t.expectEqual(@as(?s.UiAction, null), session.hud_focus);
}
// Catches accidental restart on guidance reset, lost remembered speed, and conflating help policy.
test "Classic3E skip and two reset paths retain authority queue identities and remembered speed" {
    var raw = try memory(.{ .onboarding_completed = true });
    defer raw.deinit();
    var session = try init(t.allocator, raw.raw());
    defer session.deinit();
    _ = session.core.setSpeed(.very_fast);
    _ = try session.primaryTap(0, false);
    _ = try session.cursorHover(3, false);
    _ = try session.keyboardAction(.launch);
    const fp = session.core.canonicalFingerprint();
    const envelopes = try t.allocator.dupe(m.CommandEnvelope, session.core._simulation.pending.items);
    defer t.allocator.free(envelopes);
    session.core.help_visible = true;
    session.hud_focus = .reset_onboarding;
    try t.expectEqual(s.Persistence.saved, act(&session, raw.raw(), .reset_onboarding).persistence);
    try t.expect(!session.core.help_visible);
    try t.expect(session.onboarding.active() and !session.core.preferences().onboarding_completed);
    try t.expectEqual(app.GameSpeed.paused, session.core.speed());
    try t.expectEqual(app.GameSpeed.very_fast, session.core.resumeSpeed());
    try t.expectEqual(@as(?o.Step, .select_union_world), session.onboarding.step());
    try t.expectEqual(@as(?u8, 0), session.core.selected_world);
    try t.expectEqual(@as(?u8, 3), session.core.hovered_world);
    try t.expect(std.meta.eql(s.UiAction.reset_onboarding, session.hud_focus.?));
    _ = session.controls(null);
    _ = session.core.advance(250_000_000);
    try t.expectEqual(@as(?o.Step, .select_union_world), session.onboarding.step());
    try t.expectEqual(s.Persistence.saved, act(&session, raw.raw(), .skip_onboarding).persistence);
    try t.expectEqual(app.GameSpeed.very_fast, session.core.speed());
    try t.expectEqual(s.Persistence.not_requested, act(&session, raw.raw(), .skip_onboarding).persistence);
    session.core.help_visible = true;
    _ = act(&session, raw.raw(), .open_settings);
    session.hud_focus = .reset_onboarding;
    session.settings.focus(9);
    try t.expectEqual(s.Persistence.saved, act(&session, raw.raw(), .{ .settings = .reset_onboarding }).persistence);
    try t.expect(session.core.help_visible);
    try t.expectEqual(app.AppMode.settings, session.core.mode());
    try t.expectEqual(@as(usize, 9), session.settings.focusIndex());
    try t.expect(std.meta.eql(s.UiAction.reset_onboarding, session.hud_focus.?));
    try t.expectEqual(fp, session.core.canonicalFingerprint());
    try t.expectEqual(@as(u64, 1), session.core.nextSequence());
    try t.expectEqualSlices(m.CommandEnvelope, envelopes, session.core._simulation.pending.items);
    try t.expectEqual(@as(usize, 3), raw.save_calls);
}
// Catches focus resets before successful apply and loss of guidance/preferences on restart.
test "Classic3E successful apply and restart clear HUD while failed apply retains it" {
    var raw = try memory(.{ .onboarding_completed = true, .ui_scale = .percent115 });
    defer raw.deinit();
    var scenario_store = store.MemoryStore.init(t.allocator, .scenario);
    defer scenario_store.deinit();
    var session = try init(t.allocator, raw.raw());
    defer session.deinit();
    _ = act(&session, raw.raw(), .reset_onboarding);
    _ = try session.primaryTap(0, false);
    const progress = session.onboarding.step();
    const preferences = session.core.preferences();
    session.hud_focus = .open_scenario;
    _ = session.openEditor();
    _ = session.dispatchEditor(.{ .select_section = .rules }, scenario_store.raw());
    try session.editor.session.?.focusField("scenario.seed");
    try session.editor.session.?.replaceFocusedText("invalid");
    const bad = session.dispatchEditor(.apply_and_restart, scenario_store.raw());
    try t.expectEqual(@as(?tx.Outcome, .failed), bad.editor_outcome);
    try t.expectEqual(s.Effects{}, bad.effects);
    try t.expect(std.meta.eql(s.UiAction.open_scenario, session.hud_focus.?));
    try session.editor.session.?.replaceFocusedText("4947573120260902");
    try t.expectEqual(@as(?tx.Outcome, .modal_opened), session.dispatchEditor(.apply_and_restart, scenario_store.raw()).editor_outcome);
    const applied = session.dispatchEditor(.confirm, scenario_store.raw());
    try t.expectEqual(@as(?tx.Outcome, .applied), applied.editor_outcome);
    try t.expect(applied.effects.reset_camera and !applied.effects.restart_advisory and applied.effects.clear_context_focus);
    try t.expectEqual(@as(?s.UiAction, null), session.hud_focus);
    try t.expectEqual(progress, session.onboarding.step());
    try t.expectEqual(preferences, session.core.preferences());
    try t.expectEqual(app.GameSpeed.paused, session.core.speed());
    try t.expectEqual(app.GameSpeed.normal, session.core.resumeSpeed());
    session.hud_focus = .launch;
    const restarted = try session.restart();
    try t.expect(restarted.reset_camera and !restarted.restart_advisory and restarted.clear_context_focus);
    try t.expectEqual(@as(?s.UiAction, null), session.hud_focus);
    try t.expectEqual(progress, session.onboarding.step());
    try t.expectEqual(preferences, session.core.preferences());
    try t.expectEqual(@as(usize, 1), raw.save_calls);
}
// Catches wrapper consumers dropping launch/tune observations after 3B returns.
test "Classic3E keyboard and right-click wrappers consume intent effects immediately" {
    var raw = store.MemoryStore.init(t.allocator, .preferences);
    defer raw.deinit();
    var session = try init(t.allocator, raw.raw());
    defer session.deinit();
    _ = try session.primaryTap(0, false);
    _ = try session.cursorHover(3, false);
    _ = try session.rightClick(3);
    try t.expectEqual(@as(?o.Step, .tune_field), session.onboarding.step());
    _ = try session.keyboardAction(.increase);
    try t.expectEqual(@as(?o.Step, .read_advisory), session.onboarding.step());
    try t.expectEqual(@as(u64, 0), session.core.campaign().next_tick);
    try t.expectEqual(@as(usize, 2), session.core.pendingCommandCount());
    try t.expectEqual(@as(usize, 0), raw.save_calls);
}
// Catches durable mutations through a transient presentation projection and clock catch-up on close.
test "Classic3E transient presentation and mode holds preserve durable settings and clock policy" {
    var raw = try memory(.{ .onboarding_completed = true });
    defer raw.deinit();
    var session = try init(t.allocator, raw.raw());
    defer session.deinit();
    const before = session.core.preferences();
    const effects = session.setPresentationPreferences(.{ .motion = .reduced, .high_contrast = true, .graphics_quality = .low });
    try t.expect(effects.presentation_changed);
    try t.expectEqual(prefs.MotionPreference.reduced, session.presentation().motion);
    try t.expectEqual(before, session.core.preferences());
    try t.expectEqual(@as(usize, 0), raw.save_calls);
    try t.expect(act(&session, raw.raw(), .speed_up).handled);
    try t.expectEqual(app.GameSpeed.fast, session.core.speed());
    _ = session.core.advance(8_000_000);
    try t.expect(session.core.clockAlpha() > 0);
    _ = act(&session, raw.raw(), .open_settings);
    try t.expectEqual(@as(f32, 0), session.core.clockAlpha());
    try t.expectEqual(@as(usize, 0), session.core.advance(250_000_000).completed_steps);
    _ = act(&session, raw.raw(), .{ .settings = .{ .set_motion = .full } });
    try t.expectEqual(before, session.core.preferences());
    try t.expectEqual(prefs.MotionPreference.full, session.presentation().motion);
    _ = act(&session, raw.raw(), .{ .settings = .close });
    try t.expectEqual(@as(usize, 0), session.core.advance(0).completed_steps);
    try t.expectEqual(app.GameSpeed.fast, session.core.speed());
    try t.expect(act(&session, raw.raw(), .toggle_pause).handled);
    try t.expectEqual(app.GameSpeed.paused, session.core.speed());
    try t.expect(act(&session, raw.raw(), .toggle_pause).handled);
    try t.expectEqual(app.GameSpeed.fast, session.core.speed());
    try t.expect(act(&session, raw.raw(), .speed_down).handled);
    try t.expectEqual(app.GameSpeed.normal, session.core.speed());
    try t.expect(act(&session, raw.raw(), .{ .select_field = .topology }).handled);
    try t.expectEqual(m.FieldKind.topology, session.core.selected_field);
    try t.expect(act(&session, raw.raw(), .reset_camera).effects.reset_camera);
}
// Catches mode-wide gating of the four explicit exceptions in both non-Playing modes.
test "Classic3E ungated reset skip clear and advisory branches work in Settings and editor" {
    for ([_]app.AppMode{ .settings, .editing_scenario }) |mode| {
        var raw = try memory(.{ .onboarding_completed = true });
        defer raw.deinit();
        var session = try init(t.allocator, raw.raw());
        defer session.deinit();
        if (mode == .settings) _ = act(&session, raw.raw(), .open_settings) else _ = session.openEditor();
        session.core.help_visible = true;
        session.core.selected_world = 0;
        session.core.hovered_world = 3;
        session.hud_focus = .reset_onboarding;
        try t.expectEqual(s.Persistence.saved, act(&session, raw.raw(), .reset_onboarding).persistence);
        try t.expectEqual(mode, session.core.mode());
        try t.expect(!session.core.help_visible);
        try t.expect(std.meta.eql(s.UiAction.reset_onboarding, session.hud_focus.?));
        try t.expect(!act(&session, raw.raw(), .{ .select_guidance_world = 0 }).handled);
        try t.expectEqual(@as(?u8, 0), session.core.selected_world);
        try t.expectEqual(s.Persistence.saved, act(&session, raw.raw(), .skip_onboarding).persistence);
        try t.expectEqual(mode, session.core.mode());
        try t.expectEqual(app.GameSpeed.normal, session.core.speed());
        try t.expectEqual(@as(usize, 2), raw.save_calls);
        session.onboarding.reset();
        for ([_]o.Event{ .union_world_selected, .launch_previewed, .fleet_launched, .field_tuned }) |event| _ = session.onboarding.observe(event);
        try t.expect(act(&session, raw.raw(), .acknowledge_advisory).handled);
        try t.expectEqual(@as(?o.Step, .open_scenario_editor), session.onboarding.step());
        session.clearUiFocus();
        try t.expect(std.meta.eql(s.UiAction.reset_onboarding, session.hud_focus.?));
        try t.expect(!session.cancel().handled);
        try t.expect(act(&session, raw.raw(), .clear_selection).handled);
        try t.expectEqual(@as(?s.UiAction, null), session.hud_focus);
        try t.expectEqual(@as(?u8, null), session.core.selected_world);
        try t.expectEqual(mode, session.core.mode());
    }
}
// Catches flattened editor/preference channels and repeated lifecycle effects on content actions.
test "Classic3E editor diagnostics retain separate owned status action and preference failures" {
    var raw = store.MemoryStore.init(t.allocator, .preferences);
    defer raw.deinit();
    try raw.denyLoad("preferences denied");
    var scenario_store = store.MemoryStore.init(t.allocator, .scenario);
    defer scenario_store.deinit();
    try scenario_store.denyLoad("scenario denied");
    var session = try init(t.allocator, raw.raw());
    defer session.deinit();
    session.hud_focus = .open_scenario;
    _ = session.openEditor();
    try t.expectEqual(@as(?tx.Outcome, .failed), session.dispatchEditor(.load, scenario_store.raw()).editor_outcome);
    try t.expectEqualStrings("scenario denied", session.editor.status.failure.storage.message.?);
    try t.expectEqualStrings("preferences denied", session.preference_failure.?.store.message.?);
    _ = session.dispatchEditor(.{ .select_section = .rules }, scenario_store.raw());
    try session.editor.session.?.focusField("scenario.seed");
    try session.editor.session.?.replaceFocusedText("bad seed");
    const failed = session.dispatchEditor(.apply_and_restart, scenario_store.raw());
    try t.expectEqual(@as(?tx.Outcome, .failed), failed.editor_outcome);
    try t.expect(session.editor.action_failure.? == .invalid_text);
    try t.expectEqualStrings("scenario denied", session.editor.status.failure.storage.message.?);
    try t.expectEqual(s.Effects{}, failed.effects);
    try session.editor.session.?.replaceFocusedText("4947573120260902");
    var writable = store.MemoryStore.init(t.allocator, .scenario);
    defer writable.deinit();
    const saved = session.dispatchEditor(.save, writable.raw());
    try t.expectEqual(@as(?tx.Outcome, .saved), saved.editor_outcome);
    try t.expectEqual(s.Effects{}, saved.effects);
    try t.expectEqualStrings("preferences denied", session.preference_failure.?.store.message.?);
    try t.expectEqual(@as(?tx.Outcome, .closed), session.dispatchEditor(.cancel, scenario_store.raw()).editor_outcome);
    try t.expectEqualStrings("preferences denied", session.preference_failure.?.store.message.?);
    try t.expect(std.meta.eql(s.UiAction.open_scenario, session.hud_focus.?));
    try t.expectEqual(@as(usize, 0), raw.save_calls);
}
// Catches a no-op editor call silently hiding a prior preparation resource failure.
test "Classic3E inactive editor dispatch retains resource error until actual retry" {
    var raw = store.MemoryStore.init(t.allocator, .preferences);
    defer raw.deinit();
    var scenario_store = store.MemoryStore.init(t.allocator, .scenario);
    defer scenario_store.deinit();
    var failing = t.FailingAllocator.init(t.allocator, .{});
    var session = try init(failing.allocator(), raw.raw());
    defer session.deinit();
    failing.fail_index = failing.alloc_index;
    try t.expectEqual(@as(?anyerror, error.OutOfMemory), session.openEditor().editor_error);
    const inert = session.dispatchEditor(.load, scenario_store.raw());
    try t.expectEqual(@as(?tx.Outcome, .unchanged), inert.editor_outcome);
    try t.expectEqual(s.Effects{}, inert.effects);
    try t.expectEqual(@as(?anyerror, error.OutOfMemory), session.resource_errors.editor);
    try t.expect(session.recoveryFailure().? == .resource);
    failing.fail_index = std.math.maxInt(usize);
    try t.expectEqual(@as(?tx.Outcome, .opened), session.openEditor().editor_outcome);
    try t.expectEqual(@as(?anyerror, null), session.resource_errors.editor);
}
// This is a deliberately malformed test-owned producer fixture, not a normal reachable campaign.
// Catches clearing focus before AppCore admission succeeds on restart.
test "Classic3E malformed restart admission failure preserves session focus and progress" {
    var raw = store.MemoryStore.init(t.allocator, .preferences);
    defer raw.deinit();
    var session = try init(t.allocator, raw.raw());
    defer session.deinit();
    _ = try session.primaryTap(0, true);
    session.hud_focus = .launch;
    const before = session.core.canonicalFingerprint();
    const preferences = session.core.preferences();
    session.core._active_scenario.worlds[0].id = 255;
    try t.expectError(error.InvalidInitialState, session.restart());
    try t.expectEqual(before, session.core.canonicalFingerprint());
    try t.expectEqual(preferences, session.core.preferences());
    try t.expectEqual(@as(?o.Step, .preview_launch), session.onboarding.step());
    try t.expect(std.meta.eql(s.UiAction.launch, session.hud_focus.?));
    try t.expect(session.general_failure.? == .admission);
    try t.expectEqual(@as(usize, 0), raw.save_calls);
}
// Catches allowing direct speed UI requests to release first-run hold even when disabled widgets are bypassed.
test "Classic3E direct speed requests during guidance leave speed resume and residue unchanged" {
    var raw = store.MemoryStore.init(t.allocator, .preferences);
    defer raw.deinit();
    var session = try init(t.allocator, raw.raw());
    defer session.deinit();
    for ([_]s.UiAction{ .speed_up, .toggle_pause, .speed_down }) |action| {
        try t.expect(act(&session, raw.raw(), action).handled);
        try t.expectEqual(app.GameSpeed.paused, session.core.speed());
        try t.expectEqual(app.GameSpeed.normal, session.core.resumeSpeed());
        try t.expectEqual(@as(f32, 0), session.core.clockAlpha());
        try t.expect(session.core.onboardingHeld());
    }
    try t.expectEqual(@as(usize, 0), raw.save_calls);
}
// Catches borrowed interior pointers surviving relocation of all three owned epochs.
test "Classic3E moved session preserves open editor failures and future raw stores" {
    var raw = store.MemoryStore.init(t.allocator, .preferences);
    defer raw.deinit();
    try raw.denyLoad("older preference failure");
    var scenario_store = store.MemoryStore.init(t.allocator, .scenario);
    defer scenario_store.deinit();
    try scenario_store.denyLoad("older editor failure");
    for (0..12) |_| {
        var original = try init(t.allocator, raw.raw());
        _ = original.openEditor();
        _ = original.dispatchEditor(.load, scenario_store.raw());
        original.hud_focus = .{ .select_field = .topology };
        var moved = original;
        original = undefined;
        defer moved.deinit();
        try t.expectEqualStrings("older editor failure", moved.editor.status.failure.storage.message.?);
        try t.expectEqualStrings("older preference failure", moved.preference_failure.?.store.message.?);
        _ = moved.dispatchEditor(.{ .select_section = .rules }, scenario_store.raw());
        try moved.editor.session.?.focusField("scenario.seed");
        try moved.editor.session.?.replaceFocusedText("invalid");
        try t.expectEqual(@as(?tx.Outcome, .failed), moved.dispatchEditor(.save, scenario_store.raw()).editor_outcome);
        try t.expectEqualStrings("older editor failure", moved.editor.status.failure.storage.message.?);
        _ = moved.dispatchEditor(.cancel, scenario_store.raw());
        _ = moved.dispatchEditor(.confirm, scenario_store.raw());
        _ = act(&moved, raw.raw(), .open_settings);
        try t.expectEqual(s.Persistence.saved, act(&moved, raw.raw(), .{ .settings = .{ .set_motion = .reduced } }).persistence);
        try t.expectEqual(@as(?store.PreferencesFailure, null), moved.preference_failure);
    }
}
fn initAllocation(allocator: std.mem.Allocator, kind: u8) !void {
    var raw = if (kind == 0) try memory(.{ .onboarding_completed = true }) else try store.MemoryStore.withSlot(t.allocator, .preferences, "malformed");
    defer raw.deinit();
    if (kind >= 2) try raw.denyLoad("owned startup denial");
    var value = try candidate();
    if (kind == 3) value.worlds[0].id = 255;
    var session = s.Session.init(allocator, &value, raw.raw()) catch |failure| {
        if (failure == error.InvalidInitialState and kind == 3) return;
        return failure;
    };
    defer session.deinit();
    try t.expectEqual(@as(usize, 0), raw.save_calls);
}
fn settingsAllocation(allocator: std.mem.Allocator, denied_save: bool) !void {
    var raw = store.MemoryStore.init(t.allocator, .preferences);
    defer raw.deinit();
    try raw.denyLoad("old owned denial");
    var session = try init(allocator, raw.raw());
    defer session.deinit();
    _ = act(&session, raw.raw(), .open_settings);
    if (denied_save) try raw.failNextSave("new owned denial");
    const result = act(&session, raw.raw(), .{ .settings = .{ .set_high_contrast = true } });
    try t.expect(session.core.preferences().high_contrast);
    if (result.persistence == .resource_error) {
        try t.expectEqualStrings("old owned denial", session.preference_failure.?.store.message.?);
        try t.expectEqual(@as(?[]u8, null), raw.slot);
        return error.OutOfMemory;
    }
    try t.expectEqual(if (denied_save) s.Persistence.failed else .saved, result.persistence);
    if (denied_save) try t.expectEqualStrings("new owned denial", session.preference_failure.?.store.message.?) else try t.expectEqual(@as(?store.PreferencesFailure, null), session.preference_failure);
}
fn completionAllocation(allocator: std.mem.Allocator) !void {
    var raw = store.MemoryStore.init(t.allocator, .preferences);
    defer raw.deinit();
    var session = try init(allocator, raw.raw());
    defer session.deinit();
    markFinal(&session);
    session.hud_focus = .open_scenario;
    const result = act(&session, raw.raw(), .open_scenario);
    try t.expect(session.core.preferences().onboarding_completed and !session.core.onboardingHeld());
    try t.expect(!session.onboarding.active());
    try t.expect(std.meta.eql(s.UiAction.open_scenario, session.hud_focus.?));
    if (result.persistence == .resource_error or result.editor_error != null) {
        try t.expectEqual(@as(?tx.Outcome, null), result.editor_outcome);
        try t.expectEqual(@as(?anyerror, error.OutOfMemory), result.editor_error);
        try t.expectEqual(app.AppMode.playing, session.core.mode());
        if (result.persistence == .saved) try t.expect((try prefs.decode(t.allocator, raw.slot.?)).onboarding_completed);
        return error.OutOfMemory;
    }
    try t.expectEqual(s.Persistence.saved, result.persistence);
    try t.expectEqual(@as(?tx.Outcome, .opened), result.editor_outcome);
}
// Catches leaked transferred failure payloads on failed admission and every alloc ordinal.
test "Classic3E initialization settings failure replacement and completion enumerate allocation failures" {
    var counts: [7]usize = undefined;
    for (0..4) |i| {
        var measure = t.FailingAllocator.init(t.allocator, .{});
        try initAllocation(measure.allocator(), @intCast(i));
        counts[i] = measure.alloc_index;
        try t.checkAllAllocationFailures(t.allocator, initAllocation, .{@as(u8, @intCast(i))});
    }
    for ([_]bool{ false, true }, 0..) |denied, i| {
        var measure = t.FailingAllocator.init(t.allocator, .{});
        try settingsAllocation(measure.allocator(), denied);
        counts[4 + i] = measure.alloc_index;
        try t.checkAllAllocationFailures(t.allocator, settingsAllocation, .{denied});
    }
    // Arena growth must use deterministic new allocations: backing allocator
    // remap availability varies by optimization mode and resident heap layout.
    var measure = t.FailingAllocator.init(t.allocator, .{ .resize_fail_index = 0 });
    try completionAllocation(measure.allocator());
    counts[6] = measure.alloc_index;
    for (0..counts[6]) |ordinal| {
        var failing = t.FailingAllocator.init(t.allocator, .{ .fail_index = ordinal, .resize_fail_index = 0 });
        try t.expectError(error.OutOfMemory, completionAllocation(failing.allocator()));
        try t.expect(failing.has_induced_failure);
        try t.expectEqual(failing.allocated_bytes, failing.freed_bytes);
    }
    std.debug.print("\nClassic3E allocation ordinals: init-valid={d}, corrupt={d}, denial={d}, denied-invalid-admission={d}, settings-save={d}, settings-denial={d}, completion-open={d}\n", .{ counts[0], counts[1], counts[2], counts[3], counts[4], counts[5], counts[6] });
}
