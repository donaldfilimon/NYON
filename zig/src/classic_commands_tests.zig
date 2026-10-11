const std = @import("std");
const t = std.testing;
const app = @import("classic_app.zig");
const c = @import("classic_commands.zig");
const m = @import("model.zig");
const scenario = @import("scenario.zig");
const sim = @import("simulation.zig");
fn ready(allocator: std.mem.Allocator) !app.AppCore {
    const candidate = try scenario.generatedValidated(m.DEFAULT_SEED, .{});
    return app.AppCore.init(allocator, &candidate, .{ .onboarding_completed = true });
}
fn ids(core: *const app.AppCore, s: ?u8, h: ?u8, latch: ?u8) !void {
    try t.expectEqual(s, core.selected_world);
    try t.expectEqual(h, core.hovered_world);
    try t.expectEqual(latch, core.touch_command_target);
}
fn seed(core: *app.AppCore) void {
    core.selected_world = 0;
    core.hovered_world = 3;
    core.touch_command_target = 4;
    core.selected_field = .topology;
}
// Catches implicit enqueue, lost latch, wrong mouse ownership restrictions,
// nonfriendly source retention and incorrect observation/focus transitions.
test "Classic commands picking transition table never queues" {
    const Op = enum { helper, hover, cursor, mouse, touch };
    const Row = struct { op: Op, hit: ?u8, s: ?u8, h: ?u8, latch: ?u8, focus: bool, event: ?c.Observation };
    const rows = [_]Row{
        .{ .op = .helper, .hit = 1, .s = 1, .h = 3, .latch = null, .focus = true, .event = null },
        .{ .op = .helper, .hit = null, .s = null, .h = 3, .latch = null, .focus = true, .event = null },
        .{ .op = .hover, .hit = null, .s = 0, .h = null, .latch = 4, .focus = false, .event = null },
        .{ .op = .hover, .hit = 0, .s = 0, .h = 0, .latch = 4, .focus = false, .event = null },
        .{ .op = .cursor, .hit = 3, .s = 0, .h = 3, .latch = 4, .focus = false, .event = .launch_previewed },
        .{ .op = .cursor, .hit = 0, .s = 0, .h = 0, .latch = 4, .focus = false, .event = null },
        .{ .op = .cursor, .hit = null, .s = 0, .h = null, .latch = 4, .focus = false, .event = .launch_previewed },
        .{ .op = .mouse, .hit = 1, .s = 1, .h = 1, .latch = null, .focus = true, .event = null },
        .{ .op = .mouse, .hit = 0, .s = 0, .h = 0, .latch = null, .focus = true, .event = .union_world_selected },
        .{ .op = .mouse, .hit = null, .s = null, .h = null, .latch = null, .focus = true, .event = null },
        .{ .op = .touch, .hit = 3, .s = 0, .h = 3, .latch = 3, .focus = true, .event = .launch_previewed },
        .{ .op = .touch, .hit = 0, .s = 0, .h = 0, .latch = null, .focus = true, .event = .union_world_selected },
        .{ .op = .touch, .hit = null, .s = null, .h = null, .latch = null, .focus = true, .event = null },
    };
    for (rows) |row| {
        var core = try ready(t.allocator);
        defer core.deinit();
        seed(&core);
        const before = core.canonicalFingerprint();
        const result = try switch (row.op) {
            .helper => c.selectSceneWorld(&core, row.hit, false),
            .hover => c.hoverSceneWorld(&core, row.hit, false),
            .cursor => c.cursorHover(&core, row.hit, false),
            .mouse => c.primaryTap(&core, row.hit, false),
            .touch => c.primaryTap(&core, row.hit, true),
        };
        try t.expect(result.handled);
        try ids(&core, row.s, row.h, row.latch);
        try t.expectEqual(row.focus, result.effects.clear_ui_focus);
        try t.expectEqual(row.event, result.effects.observation);
        try t.expectEqual(before, core.canonicalFingerprint());
        try t.expectEqual(@as(usize, 0), core._simulation.pending.items.len);
        try t.expectEqual(@as(u64, 0), core.nextSequence());
        try t.expectEqual(m.FieldKind.topology, core.selected_field);
    }
}
// Catches wrong gate ordering and partial mutation on invalid picking admission.
test "Classic commands mode HUD and invalid picking gates preserve state atomically" {
    var core = try ready(t.allocator);
    defer core.deinit();
    seed(&core);
    for ([_]app.AppMode{ .editing_scenario, .settings }) |mode| {
        core.setMode(mode);
        try t.expect(!(try c.selectSceneWorld(&core, 255, false)).handled);
        try t.expect(!(try c.hoverSceneWorld(&core, 255, false)).handled);
        try t.expect(!(try c.cursorHover(&core, 255, false)).handled);
        for ([_]bool{ false, true }) |touch| try t.expect(!(try c.primaryTap(&core, 255, touch)).handled);
        try t.expect(!c.selectField(&core, .hydrosphere));
        try t.expect(!c.cancel(&core).handled);
        try ids(&core, 0, 3, 4);
    }
    core.setMode(.playing);
    try t.expect(!(try c.selectSceneWorld(&core, 255, true)).handled);
    try ids(&core, 0, 3, 4);
    const hover = try c.hoverSceneWorld(&core, 255, true);
    try t.expect(hover.handled);
    try ids(&core, 0, null, 4);
    seed(&core);
    try t.expectError(error.InvalidWorld, c.selectSceneWorld(&core, 255, false));
    try ids(&core, 0, 3, 4);
    try t.expectError(error.InvalidWorld, c.hoverSceneWorld(&core, 255, false));
    try ids(&core, 0, 3, 4);
    try t.expectError(error.InvalidWorld, c.cursorHover(&core, 255, false));
    try ids(&core, 0, 3, 4);
    for ([_]bool{ false, true }) |touch| {
        try t.expectError(error.InvalidWorld, c.primaryTap(&core, 255, touch));
        try ids(&core, 0, 3, 4);
    }
    core.setMode(.settings);
    c.clearHover(&core);
    try ids(&core, 0, null, 4);
    const cleared = c.clearSelection(&core);
    try t.expect(cleared.effects.clear_ui_focus);
    try ids(&core, null, null, null);
    try t.expectEqual(m.FieldKind.topology, core.selected_field);
    core.setMode(.playing);
    try t.expect(c.selectField(&core, .hydrosphere));
    seed(&core);
    try t.expect(c.cancel(&core).effects.clear_ui_focus);
    try ids(&core, null, null, null);
}
// Catches source ownership cached at selection and hover collision bypass.
test "Classic commands touch observes current ownership and hover has priority" {
    var core = try ready(t.allocator);
    defer core.deinit();
    _ = try c.primaryTap(&core, 0, true);
    _ = try c.primaryTap(&core, 3, true);
    c.clearHover(&core);
    try t.expectEqual(@as(?u8, 3), c.commandTarget(&core));
    _ = try c.hoverSceneWorld(&core, 0, false);
    try t.expectEqual(@as(?u8, 0), c.commandTarget(&core));
    core._simulation.state.worlds[1].owner = .union_faction;
    const replaced = try c.primaryTap(&core, 1, true);
    try ids(&core, 1, 1, null);
    try t.expectEqual(c.Observation.union_world_selected, replaced.effects.observation.?);
    core._simulation.state.worlds[1].owner = .helix;
    const captured = try c.primaryTap(&core, 3, true);
    try ids(&core, null, 3, null);
    try t.expect(captured.effects.observation == null);
    _ = try c.primaryTap(&core, 1, false);
    c.clearHover(&core);
    try t.expect(c.commandTarget(&core) == null);
    core.selected_world = 0;
    core._simulation.state.worlds[0].energy = 19_999;
    const insufficient = try c.primaryTap(&core, 3, true);
    try ids(&core, 0, 3, 3);
    try t.expect(insufficient.effects.observation == null);
    core._simulation.state.worlds[0].energy = 80_000;
    const hud_cursor = try c.cursorHover(&core, 255, true);
    try ids(&core, 0, null, 3);
    try t.expectEqual(c.Observation.launch_previewed, hud_cursor.effects.observation.?);
}

// Catches projection mode gates, wrong rejection precedence, wrong literal UI
// status/strength policy and borrowed temporary names.
test "Classic commands tray and field controls expose exact immutable policy" {
    var core = try ready(t.allocator);
    defer core.deinit();
    core._simulation.state.worlds[0].energy = 80_000;
    core._simulation.state.worlds[0].name = try m.WorldName.init("SOURCE");
    core._simulation.state.worlds[3].name = try m.WorldName.init("TARGET");
    core._simulation.state.worlds[0].fields.atmosphere = 5;
    const missing = c.commandTray(&core);
    try t.expect(!missing.launch_enabled);
    try t.expect(missing.source == null);
    try t.expect(missing.destination == null);
    try t.expectEqualStrings("SELECT SOURCE AND TARGET", missing.status);
    try t.expect(missing.launch_strength == null);
    try t.expect(c.previewContextualLaunch(&core) == null);
    for ([_]m.FieldAdjustment{ .increase, .decrease }) |adjustment| {
        const control = c.fieldControl(&core, adjustment);
        try t.expect(!control.enabled);
        try t.expectEqualStrings("Select a Union world", control.tooltip);
    }
    core.selected_world = 0;
    core.hovered_world = 3;
    core.setMode(.settings);
    const valid = c.commandTray(&core);
    try t.expect(valid.launch_enabled);
    try t.expectEqual(@as(?u64, 40_000), valid.launch_strength);
    try t.expectEqualStrings("COMMAND READY - EXPLICIT LAUNCH REQUIRED", valid.status);
    try t.expectEqualStrings("SOURCE", valid.source.?.asSlice());
    try t.expectEqualStrings("TARGET", valid.destination.?.asSlice());
    try t.expectEqual(@as(?u64, 40_000), (try c.previewContextualLaunch(&core).?).launch_strength);
    try t.expect(c.fieldControl(&core, .increase).enabled);
    try t.expect(c.fieldControl(&core, .decrease).enabled);
    try t.expectEqualStrings("Increase selected field (E)", c.fieldControl(&core, .increase).tooltip);
    try t.expectEqualStrings("Decrease selected field (Q)", c.fieldControl(&core, .decrease).tooltip);
    core._simulation.state.worlds[0].name = try m.WorldName.init("CHANGED");
    try t.expectEqualStrings("SOURCE", valid.source.?.asSlice());
    const Row = struct { source: u8, target: u8, energy: u64, level: u8, finished: bool = false, enabled: bool = false, strength: ?u64 = null, up_enabled: bool = false, down_enabled: bool = false, status: []const u8, up: []const u8, down: []const u8 };
    const rows = [_]Row{
        .{ .source = 0, .target = 0, .energy = 80_000, .level = 5, .up_enabled = true, .down_enabled = true, .status = "REJECTED - CHOOSE ANOTHER TARGET", .up = "Increase selected field (E)", .down = "Decrease selected field (Q)" },
        .{ .source = 1, .target = 3, .energy = 80_000, .level = 5, .status = "REJECTED - SOURCE IS NOT UNION", .up = "REJECTED - SOURCE IS NOT UNION", .down = "REJECTED - SOURCE IS NOT UNION" },
        .{ .source = 0, .target = 3, .energy = 19_999, .level = 5, .down_enabled = true, .status = "REJECTED - INSUFFICIENT ENERGY", .up = "REJECTED - INSUFFICIENT ENERGY", .down = "Decrease selected field (Q)" },
        .{ .source = 0, .target = 3, .energy = 80_000, .level = 10, .enabled = true, .strength = 40_000, .down_enabled = true, .status = "COMMAND READY - EXPLICIT LAUNCH REQUIRED", .up = "REJECTED - FIELD AT LIMIT", .down = "Decrease selected field (Q)" },
        .{ .source = 0, .target = 3, .energy = 80_000, .level = 0, .enabled = true, .strength = 40_000, .up_enabled = true, .status = "COMMAND READY - EXPLICIT LAUNCH REQUIRED", .up = "Increase selected field (E)", .down = "REJECTED - FIELD AT LIMIT" },
        .{ .source = 255, .target = 3, .energy = 80_000, .level = 5, .status = "REJECTED - INVALID WORLD", .up = "REJECTED - INVALID WORLD", .down = "REJECTED - INVALID WORLD" },
        .{ .source = 0, .target = 3, .energy = 80_000, .level = 5, .finished = true, .status = "REJECTED - CAMPAIGN FINISHED", .up = "REJECTED - CAMPAIGN FINISHED", .down = "REJECTED - CAMPAIGN FINISHED" },
    };
    for (rows) |row| {
        core.selected_world = row.source;
        core.hovered_world = row.target;
        core._simulation.state.worlds[0].energy = row.energy;
        core._simulation.state.worlds[0].fields.atmosphere = row.level;
        core._simulation.state.phase = if (row.finished) .{ .finished = .{ .at = 0, .outcome = .player_eliminated } } else .running;
        const tray = c.commandTray(&core);
        try t.expectEqualStrings(row.status, tray.status);
        try t.expectEqual(row.enabled, tray.launch_enabled);
        try t.expectEqual(row.strength, tray.launch_strength);
        const up = c.fieldControl(&core, .increase);
        const down = c.fieldControl(&core, .decrease);
        try t.expectEqualStrings(row.up, up.tooltip);
        try t.expectEqualStrings(row.down, down.tooltip);
        try t.expectEqual(row.up_enabled, up.enabled);
        try t.expectEqual(row.down_enabled, down.enabled);
        if (row.source == 255) try t.expect(tray.source == null);
    }
    const reasons = [_]sim.CommandRejection{ error.WrongTick, error.DuplicateSequence, error.CampaignFinished, error.InvalidWorld, error.SameSourceAndTarget, error.SourceNotOwnedByIssuer, error.InsufficientEnergy, error.FieldAtLimit };
    const labels = [_][]const u8{ "REJECTED - WRONG TICK", "REJECTED - DUPLICATE COMMAND", "REJECTED - CAMPAIGN FINISHED", "REJECTED - INVALID WORLD", "REJECTED - CHOOSE ANOTHER TARGET", "REJECTED - SOURCE IS NOT UNION", "REJECTED - INSUFFICIENT ENERGY", "REJECTED - FIELD AT LIMIT" };
    for (reasons, labels) |reason, label| try t.expectEqualStrings(label, c.rejectionLabel(reason));
}
// Catches reservation, sequence consumption, mutation and allocator use in reads.
test "Classic commands repeated projections preserve complete pending envelopes and authority" {
    var failing = std.testing.FailingAllocator.init(t.allocator, .{});
    var core = try ready(failing.allocator());
    defer core.deinit();
    seed(&core);
    const envelopes = [_]m.CommandEnvelope{
        .{ .tick = 0, .sequence = 7, .command = .{ .launch = .{ .source = 0, .destination = 3 } } },
        .{ .tick = 0, .sequence = 4, .command = .{ .tune_field = .{ .world = 0, .field = .hydrosphere, .adjustment = .decrease } } },
    };
    for (envelopes) |envelope| try core._simulation.enqueue(envelope);
    core._next_sequence = 9;
    const fingerprint = core.canonicalFingerprint();
    const allocations = failing.allocations;
    const resizes = failing.resize_index;
    failing.fail_index = failing.alloc_index;
    for (0..128) |_| {
        try t.expect(c.previewContextualLaunch(&core) != null);
        _ = c.commandTray(&core);
        _ = c.fieldControl(&core, .increase);
        _ = c.fieldControl(&core, .decrease);
    }
    try t.expectEqual(fingerprint, core.canonicalFingerprint());
    try t.expectEqualDeep(&envelopes, core._simulation.pending.items);
    try t.expectEqual(allocations, failing.allocations);
    try t.expectEqual(resizes, failing.resize_index);
    try t.expect(!failing.has_induced_failure);
    try t.expectEqual(@as(u64, 9), core.nextSequence());
    try ids(&core, 0, 3, 4);
    try t.expectEqual(m.FieldKind.topology, core.selected_field);
    try t.expectEqual(@as(usize, 2), core.pendingCommandCount());
}

const launch: m.GameCommand = .{ .launch = .{ .source = 0, .destination = 3 } };
// Catches hover-vs-latch source routing, missing intent invention, mode/hold
// conflation, invalid dispatch filtering and UI enabled policy reapplication.
test "Classic commands explicit dispatch distinguishes keyboard contextual field and holds" {
    var core = try ready(t.allocator);
    defer core.deinit();
    try t.expectError(error.MissingSource, c.contextualLaunch(&core));
    try t.expectError(error.MissingSource, c.adjustField(&core, .increase));
    core.selected_world = 0;
    core.touch_command_target = 3;
    try t.expectError(error.MissingTarget, c.keyboardAction(&core, .launch));
    const queued = try c.contextualLaunch(&core);
    try t.expect(queued.handled);
    try t.expectEqual(@as(u64, 0), queued.queued.?.sequence);
    try t.expect(queued.queued.?.preview_valid);
    try t.expectEqual(c.Observation.fleet_launched, queued.effects.observation.?);
    try ids(&core, 0, null, null);
    core.hovered_world = 0;
    core.touch_command_target = 3;
    const collision = try c.keyboardAction(&core, .launch);
    try t.expect(!collision.queued.?.preview_valid);
    try t.expect(collision.effects.observation == null);
    try ids(&core, 0, 0, 3);
    _ = c.selectField(&core, .hydrosphere);
    const tune = try c.keyboardAction(&core, .decrease);
    try t.expect(tune.queued.?.preview_valid);
    try t.expectEqual(c.Observation.field_tuned, tune.effects.observation.?);
    try ids(&core, 0, 0, 3);
    const expected = [_]m.CommandEnvelope{
        .{ .tick = 0, .sequence = 0, .command = launch },
        .{ .tick = 0, .sequence = 1, .command = .{ .launch = .{ .source = 0, .destination = 0 } } },
        .{ .tick = 0, .sequence = 2, .command = .{ .tune_field = .{ .world = 0, .field = .hydrosphere, .adjustment = .decrease } } },
    };
    try t.expectEqualDeep(&expected, core._simulation.pending.items);
    for ([_]app.AppMode{ .editing_scenario, .settings }) |mode| {
        core.setMode(mode);
        try t.expectError(error.ModeRefused, c.queueGameCommand(&core, launch));
        try t.expectError(error.ModeRefused, c.keyboardAction(&core, .increase));
        try t.expectError(error.ModeRefused, c.contextualLaunch(&core));
        try t.expectError(error.ModeRefused, c.adjustField(&core, .decrease));
        try t.expectEqualDeep(&expected, core._simulation.pending.items);
        try t.expectEqual(@as(u64, 3), core.nextSequence());
    }
    core.setMode(.playing);
    const cleared = c.clearSelection(&core);
    try t.expect(cleared.effects.clear_ui_focus);
    try ids(&core, null, null, null);
    try t.expectEqualDeep(&expected, core._simulation.pending.items);
    try t.expectEqual(@as(u64, 3), core.nextSequence());
    try t.expectEqual(m.FieldKind.hydrosphere, core.selected_field);
    _ = core.setSpeed(.paused);
    _ = try c.queueGameCommand(&core, launch);
    core.setOnboardingHold(true);
    _ = try c.queueGameCommand(&core, launch);
    core._failure = .{ .cause = error.OutOfMemory, .tick = 0 };
    _ = try c.queueGameCommand(&core, launch);
    try t.expectEqual(@as(usize, 6), core.pendingCommandCount());
    try t.expectEqual(@as(u64, 6), core.nextSequence());
}
// Catches invalid picking admitted or raw invalid gameplay dropped before tick.
test "Classic commands raw invalid IDs and disabled direct tune reach canonical rejection" {
    var core = try ready(t.allocator);
    defer core.deinit();
    seed(&core);
    try t.expectError(error.InvalidWorld, c.rightClick(&core, 255));
    try ids(&core, 0, 3, 4);
    const invalid = try c.queueGameCommand(&core, .{ .launch = .{ .source = 255, .destination = 3 } });
    try t.expect(!invalid.queued.?.preview_valid);
    try t.expect(invalid.effects.observation == null);
    try ids(&core, 0, 3, 4);
    core.selected_field = .atmosphere;
    core._simulation.state.worlds[0].fields.atmosphere = 10;
    try t.expect(!c.fieldControl(&core, .increase).enabled);
    const tune = try c.adjustField(&core, .increase);
    try t.expect(!tune.queued.?.preview_valid);
    try t.expect(tune.effects.observation == null);
    var report = try core._simulation.step();
    defer report.deinit(t.allocator);
    try t.expectEqualDeep(&[_]sim.CommandResult{
        .{ .rejected = .{ .sequence = 0, .reason = error.InvalidWorld } },
        .{ .rejected = .{ .sequence = 1, .reason = error.FieldAtLimit } },
    }, report.command_results.items);
    try t.expectEqual(@as(u64, 2), core.nextSequence());
    try t.expectEqual(@as(usize, 0), core.pendingCommandCount());
}
// Catches resource reservation by previews and observation based on future tick.
test "Classic commands competing individually valid previews enqueue then reject canonically" {
    var core = try ready(t.allocator);
    defer core.deinit();
    seed(&core);
    core._simulation.state.worlds[0].energy = 30_000;
    try t.expectEqual(@as(?u64, 15_000), (try c.previewContextualLaunch(&core).?).launch_strength);
    const first = try c.contextualLaunch(&core);
    const second = try c.contextualLaunch(&core);
    for ([_]c.Dispatch{ first, second }) |result| {
        try t.expect(result.queued.?.preview_valid);
        try t.expectEqual(c.Observation.fleet_launched, result.effects.observation.?);
    }
    try t.expectEqual(@as(u64, 30_000), core.campaign().worlds[0].energy);
    var report = try core._simulation.step();
    defer report.deinit(t.allocator);
    try t.expectEqualDeep(&[_]sim.CommandResult{ .{ .accepted = 0 }, .{ .rejected = .{ .sequence = 1, .reason = error.InsufficientEnergy } } }, report.command_results.items);
    try t.expectEqual(@as(usize, 1), core.campaign().fleets.len);
    try t.expectEqual(@as(u64, 15_000), core.campaign().fleets[0].strength);
}
// Catches sequence collision skipping, premature consumption and right-click
// rolling back its source-required pointer transition after enqueue failure.
test "Classic commands wrap collision finished and right click failure retain exact intent" {
    var core = try ready(t.allocator);
    defer core.deinit();
    seed(&core);
    _ = try c.queueGameCommand(&core, launch);
    core._next_sequence = std.math.maxInt(u64);
    const maximum = try c.queueGameCommand(&core, launch);
    try t.expectEqual(std.math.maxInt(u64), maximum.queued.?.sequence);
    try t.expectEqual(@as(u64, 0), core.nextSequence());
    core.touch_command_target = 4;
    try t.expectError(error.DuplicateSequence, c.queueGameCommand(&core, launch));
    try t.expectEqual(@as(?u8, 4), core.touch_command_target);
    try t.expectError(error.DuplicateSequence, c.rightClick(&core, 2));
    try ids(&core, 0, 2, null);
    try t.expectEqualDeep(&[_]m.CommandEnvelope{ .{ .tick = 0, .sequence = 0, .command = launch }, .{ .tick = 0, .sequence = std.math.maxInt(u64), .command = launch } }, core._simulation.pending.items);
    try t.expectEqual(@as(u64, 0), core.nextSequence());
    core._simulation.state.phase = .{ .finished = .{ .at = 0, .outcome = .player_eliminated } };
    core._next_sequence = 9;
    core.touch_command_target = 4;
    try t.expectError(error.CampaignFinished, c.queueGameCommand(&core, launch));
    try t.expectEqual(@as(u64, 9), core.nextSequence());
    try t.expectEqual(@as(?u8, 4), core.touch_command_target);
    try t.expectError(error.CampaignFinished, c.rightClick(&core, 3));
    try ids(&core, 0, 3, null);
    core.selected_world = null;
    core.touch_command_target = 4;
    try t.expect(!(try c.rightClick(&core, 2)).handled);
    try ids(&core, null, 3, 4);
    core.selected_world = 0;
    try t.expect(!(try c.rightClick(&core, null)).handled);
    try ids(&core, 0, 3, 4);
    core.setMode(.settings);
    try t.expect(!(try c.rightClick(&core, 255)).handled);
    try ids(&core, 0, 3, 4);
}
// Catches first append and every growth allocation failure corrupting authority,
// queued bytes/order, sequence or latch. Genuine allocator, no mock queue.
fn everyQueueFailure(allocator: std.mem.Allocator) !void {
    var core = try ready(allocator);
    defer core.deinit();
    seed(&core);
    const before = core.canonicalFingerprint();
    var expected: [96]m.CommandEnvelope = undefined;
    for (0..96) |index| {
        const command: m.GameCommand = .{ .launch = .{ .source = 255, .destination = @intCast(index) } };
        const result = c.queueGameCommand(&core, command) catch |err| {
            try t.expectEqual(error.OutOfMemory, err);
            try t.expectEqual(before, core.canonicalFingerprint());
            try t.expectEqualDeep(expected[0..index], core._simulation.pending.items);
            try t.expectEqual(@as(u64, @intCast(index)), core.nextSequence());
            try ids(&core, 0, 3, 4);
            return error.OutOfMemory;
        };
        try t.expect(!result.queued.?.preview_valid);
        try t.expect(result.effects.observation == null);
        expected[index] = .{ .tick = 0, .sequence = @intCast(index), .command = command };
    }
    try t.expectEqualDeep(&expected, core._simulation.pending.items);
    try t.expectEqual(before, core.canonicalFingerprint());
}
test "Classic commands every queue allocation failure preserves existing commands without leaks" {
    try std.testing.checkAllAllocationFailures(t.allocator, everyQueueFailure, .{});
}
// Catches consume-on-OOM and a retry consuming twice or hiding enqueue effects.
test "Classic commands allocation retry consumes once and right click keeps preenqueue IDs" {
    var failing = std.testing.FailingAllocator.init(t.allocator, .{ .fail_index = 0, .resize_fail_index = 0 });
    var core = try ready(failing.allocator());
    defer core.deinit();
    seed(&core);
    const before = core.canonicalFingerprint();
    try t.expectError(error.OutOfMemory, c.rightClick(&core, 2));
    try ids(&core, 0, 2, null);
    try t.expectEqual(@as(u64, 0), core.nextSequence());
    try t.expectEqual(@as(usize, 0), core.pendingCommandCount());
    try t.expectEqual(before, core.canonicalFingerprint());
    failing.fail_index = std.math.maxInt(usize);
    failing.resize_fail_index = std.math.maxInt(usize);
    const retried = try c.rightClick(&core, 2);
    try t.expectEqual(@as(u64, 0), retried.queued.?.sequence);
    try t.expectEqual(c.Observation.fleet_launched, retried.effects.observation.?);
    while (core.pendingCommandCount() < core._simulation.pending.capacity) _ = try c.queueGameCommand(&core, .{ .launch = .{ .source = 255, .destination = 3 } });
    const count = core.pendingCommandCount();
    const sequence = core.nextSequence();
    var saved: [128]m.CommandEnvelope = undefined;
    @memcpy(saved[0..count], core._simulation.pending.items);
    failing.fail_index = failing.alloc_index;
    failing.resize_fail_index = failing.resize_index;
    core.touch_command_target = 4;
    try t.expectError(error.OutOfMemory, c.queueGameCommand(&core, launch));
    try t.expectEqual(sequence, core.nextSequence());
    try t.expectEqualDeep(saved[0..count], core._simulation.pending.items);
    try t.expectEqual(before, core.canonicalFingerprint());
    try t.expectEqual(@as(?u8, 4), core.touch_command_target);
    failing.fail_index = std.math.maxInt(usize);
    failing.resize_fail_index = std.math.maxInt(usize);
    const growth = try c.queueGameCommand(&core, launch);
    try t.expectEqual(sequence, growth.queued.?.sequence);
    try t.expectEqual(sequence + 1, core.nextSequence());
    try t.expectEqual(count + 1, core.pendingCommandCount());
    try t.expectEqual(c.Observation.fleet_launched, growth.effects.observation.?);
}
// Catches stored self pointers and a controller reset that interferes with 3A
// atomic replacement or leaks a moved owning simulation.
test "Classic commands moved core preserves failed replacement and successful restart resets" {
    var original = try ready(t.allocator);
    var core = original;
    original = undefined;
    defer core.deinit();
    _ = try c.primaryTap(&core, 0, true);
    _ = try c.primaryTap(&core, 3, true);
    _ = c.selectField(&core, .topology);
    _ = try c.adjustField(&core, .increase);
    core.touch_command_target = 3;
    const before = core.canonicalFingerprint();
    const candidate = core.activeScenario().*;
    var stale = candidate;
    stale.fingerprint = 0;
    try t.expectError(error.StaleFingerprint, core.replaceScenario(&stale));
    try ids(&core, 0, 3, 3);
    try t.expectEqual(before, core.canonicalFingerprint());
    try t.expectEqual(@as(u64, 1), core.nextSequence());
    try t.expectEqualDeep(&[_]m.CommandEnvelope{.{ .tick = 0, .sequence = 0, .command = .{ .tune_field = .{ .world = 0, .field = .topology, .adjustment = .increase } } }}, core._simulation.pending.items);
    try t.expectEqual(m.FieldKind.topology, core.selected_field);
    try core.restart();
    try ids(&core, null, null, null);
    try t.expectEqual(m.FieldKind.atmosphere, core.selected_field);
    try t.expectEqual(@as(u64, 0), core.nextSequence());
    try t.expectEqual(@as(usize, 0), core.pendingCommandCount());
    _ = try c.primaryTap(&core, 0, false);
    const click = try c.rightClick(&core, 3);
    try t.expectEqual(c.Observation.fleet_launched, click.effects.observation.?);
}
