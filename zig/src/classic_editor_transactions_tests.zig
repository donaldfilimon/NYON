const std = @import("std");
const t = std.testing;
const a = t.allocator;
const tx = @import("classic_editor_transactions.zig");
const app = @import("classic_app.zig");
const scenario = @import("scenario.zig");
const model = @import("model.zig");
const store = @import("classic_store.zig");
fn ready() !app.AppCore {
    const value = try scenario.generatedValidated(model.DEFAULT_SEED, .{});
    return app.AppCore.init(a, &value, .{ .onboarding_completed = true });
}
// Break: opening from live campaign or closing by restarting loses initial draft/queue/speed.
test "3D opening stages initial scenario and clean cancel preserves campaign" {
    var core = try ready();
    defer core.deinit();
    _ = core.setSpeed(.fast);
    _ = core.advance(100_000_000);
    _ = try core.enqueueGameCommand(.{ .launch = .{ .source = 0, .destination = 3 } });
    core.selected_world = 2;
    const fingerprint = core.canonicalFingerprint();
    const sequence = core.nextSequence();
    var memory = store.MemoryStore.init(a, .scenario);
    defer memory.deinit();
    var owner = tx.Transactions.init(a);
    defer owner.deinit();
    try t.expectEqual(tx.Outcome.opened, try owner.open(&core));
    try t.expectEqual(app.AppMode.editing_scenario, core.mode());
    try t.expectEqual(@as(u64, model.DEFAULT_SEED), owner.session.?.current().seed);
    try t.expectEqual(core.activeScenario().worlds[0].energy, owner.session.?.current().worlds[0].energy);
    try t.expectEqual(tx.Outcome.unchanged, try owner.open(&core));
    core._failure = .{ .cause = error.OutOfMemory, .tick = 12 };
    try t.expectEqual(tx.Outcome.closed, try owner.dispatch(&core, memory.raw(), .cancel));
    try t.expect(owner.session == null and core.recoverableStatus() == null);
    try t.expectEqual(app.AppMode.playing, core.mode());
    try t.expectEqual(fingerprint, core.canonicalFingerprint());
    try t.expectEqual(sequence, core.nextSequence());
    try t.expectEqual(@as(usize, 1), core.pendingCommandCount());
    try t.expectEqual(app.GameSpeed.fast, core.speed());
    try t.expectEqual(@as(?u8, 2), core.selected_world);
    try t.expectEqual(tx.Outcome.unchanged, try owner.dispatch(&core, memory.raw(), .save));
}
fn edit(owner: *tx.Transactions, id: []const u8, text: []const u8) !void {
    const s = &owner.session.?;
    if (std.mem.startsWith(u8, id, "rule.") or std.mem.eql(u8, id, "scenario.seed")) s.selectSection(.rules) else s.selectWorld(0);
    try s.focusField(id);
    try s.replaceFocusedText(text);
}
fn setSlot(memory: *store.MemoryStore, bytes: []const u8) !void {
    var saved = try memory.raw().save(a, bytes);
    defer saved.deinit(a);
    try t.expect(saved == .saved);
}
// Break: reading a dirty request, capturing its slot too soon, or replacing ORIGINAL on load.
test "3D clean absence and dirty load read then current slot only at confirmation" {
    var core = try ready();
    defer core.deinit();
    var memory = store.MemoryStore.init(a, .scenario);
    defer memory.deinit();
    var owner = tx.Transactions.init(a);
    defer owner.deinit();
    _ = try owner.open(&core);
    try t.expectEqual(tx.Outcome.absent, try owner.dispatch(&core, memory.raw(), .load));
    try t.expect(owner.status == .no_saved_scenario);
    try t.expect(!owner.session.?.isDirty());
    try edit(&owner, "world.0.name", "EDITED");
    try t.expectEqual(tx.Outcome.modal_opened, try owner.dispatch(&core, memory.raw(), .load));
    try t.expectEqual(@as(usize, 1), memory.load_calls);
    var draft = try scenario.ScenarioDraft.factoryDefault(a);
    defer draft.deinit();
    draft.worlds[0].name = "THEN CURRENT";
    const candidate = try draft.validated();
    const bytes = try scenario.encode(a, &candidate);
    defer a.free(bytes);
    try setSlot(&memory, bytes);
    try t.expectEqual(tx.Outcome.loaded, try owner.dispatch(&core, memory.raw(), .confirm));
    try t.expectEqual(@as(usize, 2), memory.load_calls);
    try t.expectEqualStrings("THEN CURRENT", owner.session.?.current().worlds[0].name);
    try t.expectEqualStrings("ASTER VALE", owner.session.?.original().worlds[0].name);
    try t.expect(owner.session.?.isDirty());
    try t.expectEqual(model.DEFAULT_SEED, core.activeScenario().seed);
    try t.expectEqual(tx.Outcome.failed, try owner.dispatch(&core, memory.raw(), .confirm));
    try t.expect(owner.action_failure.? == .no_confirmation);
}
// Break: replacing current before detailed decoding/semantic validation or collapsing raw errors.
test "3D load detailed malformed and semantic errors preserve draft and original" {
    var core = try ready();
    defer core.deinit();
    var memory = store.MemoryStore.init(a, .scenario);
    defer memory.deinit();
    var owner = tx.Transactions.init(a);
    defer owner.deinit();
    _ = try owner.open(&core);
    for ([_][]const u8{ "", "{", "{\"version\":9}" }) |bad| {
        try setSlot(&memory, bad);
        try t.expectEqual(tx.Outcome.failed, try owner.dispatch(&core, memory.raw(), .load));
        try t.expect(owner.status.failure == .codec);
        try t.expect(!owner.session.?.isDirty());
    }
    const initial = try scenario.encode(a, core.activeScenario());
    defer a.free(initial);
    const sem = try std.mem.replaceOwned(u8, a, initial, "\"hazard_period_ticks\":\"2700\"", "\"hazard_period_ticks\":\"0\"");
    defer a.free(sem);
    try setSlot(&memory, sem);
    try t.expectEqual(tx.Outcome.failed, try owner.dispatch(&core, memory.raw(), .load));
    try t.expect(owner.status.failure.codec == .validation);
    try t.expect(!owner.session.?.isDirty());
    try memory.denyLoad("RAW READ FAILED");
    try t.expectEqual(tx.Outcome.failed, try owner.dispatch(&core, memory.raw(), .load));
    try t.expect(owner.status.failure == .storage);
    try t.expectEqualStrings("RAW READ FAILED", owner.status.failure.storage.message.?);
    try t.expectEqualStrings("ASTER VALE", owner.session.?.current().worlds[0].name);
}
// Break: save probes before text/semantic admission, treats returned arbitrary payload as absent,
// reencodes current at confirm, or rereads occupancy instead of honoring the owned capture.
test "3D save validates before callbacks and overwrite uses independent bytes across moves" {
    var core = try ready();
    defer core.deinit();
    var memory = store.MemoryStore.init(a, .scenario);
    defer memory.deinit();
    var owner = tx.Transactions.init(a);
    var owner_live = true;
    defer if (owner_live) owner.deinit();
    _ = try owner.open(&core);
    try edit(&owner, "rule.hazard_period_ticks", "0");
    try edit(&owner, "scenario.seed", "invalid");
    try t.expectEqual(tx.Outcome.failed, try owner.dispatch(&core, memory.raw(), .save));
    try t.expect(owner.action_failure.? == .invalid_text);
    try t.expectEqual(@as(usize, 0), memory.load_calls);
    try t.expectEqual(@as(usize, 0), memory.save_calls);
    try edit(&owner, "scenario.seed", "4947573120260902");
    try t.expectEqual(tx.Outcome.failed, try owner.dispatch(&core, memory.raw(), .save));
    try t.expect(owner.action_failure.?.codec == .validation);
    try t.expectEqual(@as(usize, 0), memory.load_calls);
    try edit(&owner, "rule.hazard_period_ticks", "2700");
    try edit(&owner, "world.0.name", "CAPTURED");
    try t.expectEqual(tx.Outcome.saved, try owner.dispatch(&core, memory.raw(), .save));
    try t.expect(owner.status == .saved);
    try t.expect(owner.session.?.isDirty());
    var first = try scenario.decodeDetailed(a, memory.slot.?);
    try t.expect(first == .scenario);
    try t.expectEqualStrings("CAPTURED", first.scenario.worlds[0].name.asSlice());
    try setSlot(&memory, "\xff");
    try t.expectEqual(tx.Outcome.modal_opened, try owner.dispatch(&core, memory.raw(), .save));
    const captured = try a.dupe(u8, owner.session.?.modal.save_bytes.?);
    defer a.free(captured);
    var next = try scenario.ScenarioDraft.factoryDefault(a);
    defer next.deinit();
    next.worlds[0].name = "NEW CURRENT";
    try owner.session.?.replaceCurrent(&next);
    var moved = owner;
    owner = undefined;
    owner_live = false;
    defer moved.deinit();
    try setSlot(&memory, "changed after request");
    try memory.denyLoad("NO REREAD");
    const reads = memory.load_calls;
    try t.expectEqual(tx.Outcome.saved, try moved.dispatch(&core, memory.raw(), .confirm));
    try t.expectEqual(reads, memory.load_calls);
    try t.expectEqualSlices(u8, captured, memory.slot.?);
    try t.expectEqualStrings("NEW CURRENT", moved.session.?.current().worlds[0].name);
    try t.expectEqualStrings("ASTER VALE", moved.session.?.original().worlds[0].name);
    try t.expect(moved.session.?.modal.kind == null);
    try t.expectEqual(tx.Outcome.failed, try moved.dispatch(&core, memory.raw(), .confirm));
}
// Break: failed overwrite retains reusable confirmation or destroys durable bytes/details.
test "3D failed overwrite consumes capture and retains owned storage detail" {
    var core = try ready();
    defer core.deinit();
    var memory = try store.MemoryStore.withSlot(a, .scenario, "");
    defer memory.deinit();
    var owner = tx.Transactions.init(a);
    defer owner.deinit();
    _ = try owner.open(&core);
    try t.expectEqual(tx.Outcome.modal_opened, try owner.dispatch(&core, memory.raw(), .save));
    try memory.failNextSave("EXACT SAVE FAILURE");
    try t.expectEqual(tx.Outcome.failed, try owner.dispatch(&core, memory.raw(), .confirm));
    try t.expectEqualStrings("", memory.slot.?);
    try t.expectEqualStrings("EXACT SAVE FAILURE", owner.status.failure.storage.message.?);
    try t.expect(owner.session.?.modal.kind == null and owner.session.?.modal.save_bytes == null);
    const writes = memory.save_calls;
    try t.expectEqual(tx.Outcome.failed, try owner.dispatch(&core, memory.raw(), .confirm));
    try t.expectEqual(writes, memory.save_calls);
    try t.expectEqual(tx.Outcome.modal_opened, try owner.dispatch(&core, memory.raw(), .save));
    _ = try owner.dispatch(&core, memory.raw(), .dismiss_confirmation);
    try t.expect(owner.session.?.modal.kind == null);
    try t.expectEqual(writes, memory.save_calls);
    try t.expectEqualStrings("", memory.slot.?);
}
// Break: a scenario facade calls the preferences adapter or occupied store read errors enable writes.
test "3D wrong record refuses before callbacks and raw read failure blocks save" {
    var core = try ready();
    defer core.deinit();
    var memory = store.MemoryStore.init(a, .preferences);
    defer memory.deinit();
    var owner = tx.Transactions.init(a);
    defer owner.deinit();
    _ = try owner.open(&core);
    for ([_]@import("classic_editor.zig").Action{ .load, .save }) |action| {
        try t.expectEqual(tx.Outcome.failed, try owner.dispatch(&core, memory.raw(), action));
        try t.expectEqual(error.RecordMismatch, owner.status.failure.storage.cause.?);
    }
    try t.expectEqual(@as(usize, 0), memory.load_calls);
    try t.expectEqual(@as(usize, 0), memory.save_calls);
    var blocked = try store.MemoryStore.withSlot(a, .scenario, "old");
    defer blocked.deinit();
    try blocked.denyLoad("cannot probe");
    try t.expectEqual(tx.Outcome.failed, try owner.dispatch(&core, blocked.raw(), .save));
    try t.expect(owner.session.?.modal.kind == null);
    try t.expectEqual(@as(usize, 0), blocked.save_calls);
    try t.expectEqualStrings("old", blocked.slot.?);
}
// Break: apply trusts fingerprint alone, fails to revalidate, or slices malformed capture lengths.
test "3D apply refuses invalid and stale full value captures without changing authority" {
    var core = try ready();
    defer core.deinit();
    var memory = store.MemoryStore.init(a, .scenario);
    defer memory.deinit();
    var owner = tx.Transactions.init(a);
    defer owner.deinit();
    _ = try owner.open(&core);
    const before = core.canonicalFingerprint();
    try edit(&owner, "rule.hazard_period_ticks", "0");
    try edit(&owner, "scenario.seed", "bad");
    try t.expectEqual(tx.Outcome.failed, try owner.dispatch(&core, memory.raw(), .apply_and_restart));
    try t.expect(owner.action_failure.? == .invalid_text);
    try edit(&owner, "scenario.seed", "4947573120260902");
    try t.expectEqual(tx.Outcome.failed, try owner.dispatch(&core, memory.raw(), .apply_and_restart));
    try t.expect(owner.action_failure.? == .validation);
    try edit(&owner, "rule.hazard_period_ticks", "2700");
    for (0..7) |variant| {
        try t.expectEqual(tx.Outcome.modal_opened, try owner.dispatch(&core, memory.raw(), .apply_and_restart));
        const capsule = &owner.session.?.modal;
        switch (variant) {
            0 => capsule.apply_candidate = null,
            1 => capsule.apply_candidate.?.worlds[0].energy += 1, // same fingerprint, unequal meaning
            2 => capsule.apply_candidate.?.worlds[0].name.len = 255,
            3 => capsule.apply_candidate.?.warnings.len = 101,
            4 => {
                capsule.apply_candidate.?.warnings.len = 1;
                capsule.apply_candidate.?.warnings.entries[0].field.len = 49;
            },
            5 => {
                var draft = try owner.session.?.copyCurrent(a);
                defer draft.deinit();
                draft.rules.hazard_period_ticks = 0;
                try owner.session.?.replaceCurrent(&draft);
            },
            6 => {
                var draft = try owner.session.?.copyCurrent(a);
                defer draft.deinit();
                draft.worlds[0].name = "CHANGED";
                try owner.session.?.replaceCurrent(&draft);
            },
            else => unreachable,
        }
        try t.expectEqual(tx.Outcome.failed, try owner.dispatch(&core, memory.raw(), .confirm));
        if (variant == 5) try t.expect(owner.action_failure.? == .validation) else try t.expect(owner.action_failure.? == .stale_confirmation);
        try t.expectEqual(before, core.canonicalFingerprint());
        try t.expectEqual(app.AppMode.editing_scenario, core.mode());
        try t.expect(owner.session.?.modal.kind == null);
        var original = try owner.session.?.copyOriginal(a);
        defer original.deinit();
        try owner.session.?.replaceCurrent(&original);
    }
}
// Break: apply compares unused padding, drops editor before admission, or omits core reset/effects.
test "3D warning only apply installs captured scenario resets lifecycle and retains preferences" {
    var core = try ready();
    defer core.deinit();
    var memory = store.MemoryStore.init(a, .scenario);
    defer memory.deinit();
    _ = core.setSpeed(.very_fast);
    _ = core.advance(100_000_000);
    _ = try core.enqueueGameCommand(.{ .launch = .{ .source = 0, .destination = 3 } });
    core.selected_world = 3;
    core.hovered_world = 4;
    core.touch_command_target = 5;
    core.help_visible = true;
    var preferences = core.preferences();
    preferences.onboarding_completed = true;
    preferences.ui_scale = .percent85;
    core.setPreferences(preferences);
    var owner = tx.Transactions.init(a);
    defer owner.deinit();
    _ = try owner.open(&core);
    var draft = try owner.session.?.copyCurrent(a);
    defer draft.deinit();
    draft.worlds[0].name = "NEW ACTIVE";
    draft.worlds[1].name = "NEW ACTIVE";
    try owner.session.?.replaceCurrent(&draft);
    try t.expect(owner.session.?.validationReport().warningCount() > 0);
    try t.expectEqual(tx.Outcome.modal_opened, try owner.dispatch(&core, memory.raw(), .apply_and_restart));
    // Unused bytes have no meaning; a full backing-array comparison would spuriously reject.
    owner.session.?.modal.apply_candidate.?.worlds[0].name.bytes[23] = 99;
    owner.session.?.modal.apply_candidate.?.warnings.entries[99].message.bytes[90] = 17;
    try t.expectEqual(tx.Outcome.applied, try owner.dispatch(&core, memory.raw(), .confirm));
    try t.expect(owner.session == null);
    try t.expectEqual(app.AppMode.playing, core.mode());
    try t.expectEqualStrings("NEW ACTIVE", core.activeScenario().worlds[0].name.asSlice());
    try t.expectEqual(@as(u64, 0), core.campaign().next_tick);
    try t.expectEqual(@as(usize, 0), core.pendingCommandCount());
    try t.expectEqual(@as(u64, 0), core.nextSequence());
    try t.expect(core.selected_world == null and core.hovered_world == null and core.touch_command_target == null and !core.help_visible);
    try t.expectEqual(app.GameSpeed.normal, core.speed());
    try t.expectEqual(@as(f32, 0), core.visual_time);
    try t.expectEqualDeep(preferences, core.preferences());
    try t.expect(owner.effects.reset_camera and owner.effects.clear_focus and owner.effects.restart_advisory);
    const identity = core.activeScenario().fingerprint;
    _ = core.advance(100_000_000);
    try core.restart();
    try t.expectEqual(identity, core.activeScenario().fingerprint);
    try t.expectEqualStrings("NEW ACTIVE", core.campaign().worlds[0].name.asSlice());
}
// Break: reset bypasses confirmation, validates regeneration too early, or loses presentation/message.
test "3D confirmed reset regeneration uses typed seed invalid rules and preserves presentation" {
    var core = try ready();
    defer core.deinit();
    var memory = store.MemoryStore.init(a, .scenario);
    defer memory.deinit();
    var owner = tx.Transactions.init(a);
    defer owner.deinit();
    _ = try owner.open(&core);
    try t.expectEqual(tx.Outcome.absent, try owner.dispatch(&core, memory.raw(), .load));
    try t.expectEqual(tx.Outcome.modal_opened, try owner.dispatch(&core, memory.raw(), .revert));
    try t.expectEqual(tx.Outcome.unchanged, try owner.dispatch(&core, memory.raw(), .factory_defaults));
    try t.expect(owner.status == .no_saved_scenario);
    _ = try owner.dispatch(&core, memory.raw(), .dismiss_confirmation);
    try edit(&owner, "scenario.seed", "0000000000000001");
    try edit(&owner, "scenario.seed", "invalid");
    try edit(&owner, "rule.hazard_period_ticks", "0");
    try edit(&owner, "rule.maximum_energy", "1");
    try owner.session.?.setPreedit("visual IME");
    owner.session.?.selected_world = 4;
    owner.session.?.scroll = 17;
    const focus = owner.session.?.focus;
    try t.expectEqual(tx.Outcome.modal_opened, try owner.dispatch(&core, memory.raw(), .regenerate_from_seed));
    try t.expectEqual(tx.Outcome.replaced, try owner.dispatch(&core, memory.raw(), .confirm));
    try t.expectEqual(@as(u64, 1), owner.session.?.current().seed);
    try t.expectEqual(@as(u64, 0), owner.session.?.current().rules.hazard_period_ticks);
    try t.expectEqual(@as(u64, 1), owner.session.?.current().rules.maximum_energy);
    try t.expect(!owner.session.?.hasParseErrors());
    try t.expectEqualStrings("0000000000000001", owner.session.?.fieldText("scenario.seed").?);
    try t.expectEqualStrings("ASTER VALE", owner.session.?.current().worlds[0].name);
    try t.expectEqualDeep(focus, owner.session.?.focus);
    try t.expectEqual(@as(usize, 4), owner.session.?.selected_world);
    try t.expectEqual(@as(f32, 17), owner.session.?.scroll);
    try t.expectEqualStrings("visual IME", owner.session.?.preedit());
    try t.expect(owner.status == .no_saved_scenario);
    try t.expectEqual(model.DEFAULT_SEED, owner.session.?.original().seed);
    try t.expectEqual(tx.Outcome.failed, try owner.dispatch(&core, memory.raw(), .save));
    try t.expectEqual(@as(usize, 0), memory.save_calls);
    try t.expectEqual(tx.Outcome.failed, try owner.dispatch(&core, memory.raw(), .apply_and_restart));
    try t.expectEqual(tx.Outcome.modal_opened, try owner.dispatch(&core, memory.raw(), .factory_defaults));
    try t.expectEqual(tx.Outcome.replaced, try owner.dispatch(&core, memory.raw(), .confirm));
    try t.expect(!owner.session.?.isDirty());
    try edit(&owner, "world.0.name", "UNDO THIS");
    try t.expectEqual(tx.Outcome.modal_opened, try owner.dispatch(&core, memory.raw(), .revert));
    try t.expectEqual(tx.Outcome.replaced, try owner.dispatch(&core, memory.raw(), .confirm));
    try t.expectEqualStrings("ASTER VALE", owner.session.?.current().worlds[0].name);
    try t.expectEqualStrings("visual IME", owner.session.?.preedit());
}
// Break: direct Cancel respects a background-modal guard instead of replacing/releasing the capsule.
test "3D dirty cancel overrides overwrite and dismisses without writes then closes" {
    var core = try ready();
    defer core.deinit();
    var memory = try store.MemoryStore.withSlot(a, .scenario, "old");
    defer memory.deinit();
    var owner = tx.Transactions.init(a);
    defer owner.deinit();
    _ = try owner.open(&core);
    try edit(&owner, "world.0.name", "DIRTY");
    _ = try owner.dispatch(&core, memory.raw(), .save);
    try t.expect(owner.session.?.modal.save_bytes != null);
    try t.expectEqual(tx.Outcome.modal_opened, try owner.dispatch(&core, memory.raw(), .cancel));
    try t.expect(owner.session.?.modal.kind == .cancel_dirty and owner.session.?.modal.save_bytes == null);
    _ = try owner.dispatch(&core, memory.raw(), .dismiss_confirmation);
    try t.expectEqualStrings("DIRTY", owner.session.?.current().worlds[0].name);
    _ = try owner.dispatch(&core, memory.raw(), .apply_and_restart);
    try t.expect(owner.session.?.modal.apply_candidate != null);
    _ = try owner.dispatch(&core, memory.raw(), .cancel);
    try t.expect(owner.session.?.modal.apply_candidate == null);
    try t.expectEqual(tx.Outcome.closed, try owner.dispatch(&core, memory.raw(), .confirm));
    try t.expect(owner.session == null);
    try t.expectEqual(@as(usize, 0), memory.save_calls);
    try t.expectEqualStrings("old", memory.slot.?);
}
// Break: dispatch does not route existing selection state, or silently acts across inconsistent owners.
test "3D dispatch selection and mode correspondence guard" {
    var core = try ready();
    defer core.deinit();
    var memory = store.MemoryStore.init(a, .scenario);
    defer memory.deinit();
    var owner = tx.Transactions.init(a);
    defer owner.deinit();
    _ = try owner.open(&core);
    _ = try owner.dispatch(&core, memory.raw(), .{ .select_section = .rules });
    try t.expect(owner.session.?.section == .rules);
    _ = try owner.dispatch(&core, memory.raw(), .{ .select_world = 2 });
    try t.expectEqual(@as(usize, 2), owner.session.?.selected_world);
    core.setMode(.settings);
    try t.expectError(error.ModeSessionMismatch, owner.dispatch(&core, memory.raw(), .cancel));
    try t.expect(owner.session != null);
    core.setMode(.editing_scenario);
    _ = try owner.dispatch(&core, memory.raw(), .cancel);
    core.setMode(.editing_scenario);
    try t.expectError(error.ModeSessionMismatch, owner.open(&core));
    core.setMode(.playing);
}
const io = t.io;
const Fixture = struct {
    root: []u8,
    dir: std.Io.Dir,
    fn init() !Fixture {
        var random: [16]u8 = undefined;
        io.random(&random);
        const root = try std.fmt.allocPrint(a, "/private/tmp/nyon-task3d-{s}", .{std.fmt.bytesToHex(random, .lower)});
        errdefer a.free(root);
        const dir = try std.Io.Dir.cwd().createDirPathOpen(io, root, .{ .open_options = .{ .iterate = true } });
        return .{ .root = root, .dir = dir };
    }
    fn deinit(self: *Fixture) void {
        self.dir.close(io);
        std.Io.Dir.cwd().deleteTree(io, self.root) catch @panic("fixture cleanup failed");
        a.free(self.root);
    }
    fn path(self: Fixture, name: []const u8) ![]u8 {
        return std.fs.path.join(a, &.{ self.root, name });
    }
    fn write(self: Fixture, name: []const u8, bytes: []const u8) !void {
        try self.dir.writeFile(io, .{ .sub_path = name, .data = bytes });
    }
    fn read(self: Fixture, name: []const u8) ![]u8 {
        return self.dir.readFileAlloc(io, name, a, .limited(70_000));
    }
    fn noTemps(self: Fixture) !void {
        var iter = self.dir.iterate();
        while (try iter.next(io)) |entry| try t.expect(!std.mem.startsWith(u8, entry.name, ".nyon-classic-"));
    }
};
// Supplementary composition assertions: catches wrong native fallback/occupancy/error routing.
test "3D native primary legacy raw presence caps errors exact readback and atomic failure" {
    var f = try Fixture.init();
    defer f.deinit();
    const primary = try f.path("scenario-v1.json");
    defer a.free(primary);
    const legacy = try f.path("legacy.json");
    defer a.free(legacy);
    var native = try store.NativeStore.atPaths(a, io, .scenario, primary, legacy);
    defer native.deinit();
    var core = try ready();
    defer core.deinit();
    var owner = tx.Transactions.init(a);
    defer owner.deinit();
    _ = try owner.open(&core);
    const original = try scenario.encode(a, core.activeScenario());
    defer a.free(original);
    try f.write("legacy.json", original);
    try f.write("preferences-v1.json", "PREFERENCES SENTINEL");
    try t.expectEqual(tx.Outcome.loaded, try owner.dispatch(&core, native.raw(), .load));
    try t.expectError(error.FileNotFound, f.dir.openFile(io, "scenario-v1.json", .{}));
    for ([_][]const u8{ "", "{", "\xff" }) |bytes| {
        try f.write("scenario-v1.json", bytes);
        try t.expectEqual(tx.Outcome.failed, try owner.dispatch(&core, native.raw(), .load));
        if (bytes.len == 1 and bytes[0] == 255) {
            try t.expect(owner.status.failure == .storage);
            try t.expectEqual(error.InvalidUtf8, owner.status.failure.storage.cause.?);
            try t.expectEqual(tx.Outcome.failed, try owner.dispatch(&core, native.raw(), .save));
            try t.expect(owner.session.?.modal.kind == null);
        } else {
            try t.expect(owner.status.failure == .codec);
            try t.expectEqual(tx.Outcome.modal_opened, try owner.dispatch(&core, native.raw(), .save));
            _ = try owner.dispatch(&core, native.raw(), .dismiss_confirmation);
        }
        try expectFile(&f, "legacy.json", original);
    }
    try f.dir.deleteFile(io, "scenario-v1.json");
    try f.dir.createDir(io, "scenario-v1.json", .default_dir);
    try t.expectEqual(tx.Outcome.failed, try owner.dispatch(&core, native.raw(), .save));
    try t.expectEqual(error.IsDir, owner.status.failure.storage.cause.?);
    try t.expect(owner.session.?.modal.kind == null);
    try f.dir.deleteTree(io, "scenario-v1.json");
    const cap = try a.alloc(u8, 65_537);
    defer a.free(cap);
    @memset(cap, 'x');
    try f.write("scenario-v1.json", cap[0..65_536]);
    try t.expectEqual(tx.Outcome.modal_opened, try owner.dispatch(&core, native.raw(), .save));
    _ = try owner.dispatch(&core, native.raw(), .dismiss_confirmation);
    try f.write("scenario-v1.json", cap);
    try t.expectEqual(tx.Outcome.failed, try owner.dispatch(&core, native.raw(), .save));
    try t.expectEqual(store.FailureKind.oversized_slot, owner.status.failure.storage.kind);
    try t.expectEqual(@as(usize, 65_536), owner.status.failure.storage.max_bytes);
    try t.expect(owner.session.?.modal.kind == null);
    try f.dir.deleteFile(io, "scenario-v1.json"); // actual absence permits legacy fallback and overwrite
    try edit(&owner, "world.0.name", "NATIVE SAVED");
    try t.expectEqual(tx.Outcome.modal_opened, try owner.dispatch(&core, native.raw(), .save));
    const captured = try a.dupe(u8, owner.session.?.modal.save_bytes.?);
    defer a.free(captured);
    try t.expectEqual(tx.Outcome.saved, try owner.dispatch(&core, native.raw(), .confirm));
    const saved = try f.read("scenario-v1.json");
    defer a.free(saved);
    try t.expectEqualSlices(u8, captured, saved);
    var decoded = try scenario.decodeDetailed(a, saved);
    try t.expect(decoded == .scenario);
    try t.expectEqualStrings("NATIVE SAVED", decoded.scenario.worlds[0].name.asSlice());
    try expectFile(&f, "legacy.json", original);
    // Supplementary FIX1 proof: reopen through an independently owned native facade.
    {
        var reopened = try store.NativeStore.atPaths(a, io, .scenario, primary, legacy);
        defer reopened.deinit();
        var readback = try reopened.raw().load(a);
        defer readback.deinit(a);
        try t.expect(readback == .payload);
        try t.expectEqualSlices(u8, captured, readback.payload);
        var reopened_decoded = try scenario.decodeDetailed(a, readback.payload);
        try t.expect(reopened_decoded == .scenario);
        try t.expectEqualStrings("NATIVE SAVED", reopened_decoded.scenario.worlds[0].name.asSlice());
        try expectFile(&f, "legacy.json", original);
        try expectFile(&f, "preferences-v1.json", "PREFERENCES SENTINEL");
        try f.noTemps();
    }
    for ([_]store.FaultStage{ .create, .write, .sync, .persist }) |stage| {
        try edit(&owner, "world.0.name", "REJECTED NEW");
        _ = try owner.dispatch(&core, native.raw(), .save);
        try native.failNext(stage, "ATOMIC REJECTION");
        try t.expectEqual(tx.Outcome.failed, try owner.dispatch(&core, native.raw(), .confirm));
        try t.expectEqual(error.InjectedFailure, owner.status.failure.storage.cause.?);
        try t.expectEqualStrings("ATOMIC REJECTION", owner.status.failure.storage.message.?);
        try t.expect(owner.session.?.modal.kind == null);
        try expectFile(&f, "scenario-v1.json", saved);
        try expectFile(&f, "legacy.json", original);
        try expectFile(&f, "preferences-v1.json", "PREFERENCES SENTINEL");
        try f.noTemps();
    }
}
fn expectFile(f: *Fixture, name: []const u8, expected: []const u8) !void {
    const bytes = try f.read(name);
    defer a.free(bytes);
    try t.expectEqualSlices(u8, expected, bytes);
}

// Supplementary detailed wire tests catch compacted codec errors and MemoryStore/native conflation.
test "3D detailed load version canonical cap UTF8 errors keep detached contents" {
    var core = try ready();
    defer core.deinit();
    var memory = store.MemoryStore.init(a, .scenario);
    defer memory.deinit();
    var owner = tx.Transactions.init(a);
    defer owner.deinit();
    _ = try owner.open(&core);
    const json = try scenario.encode(a, core.activeScenario());
    defer a.free(json);
    const badversion = try std.mem.replaceOwned(u8, a, json, "\"format_version\":1", "\"format_version\":2");
    defer a.free(badversion);
    try setSlot(&memory, badversion);
    _ = try owner.dispatch(&core, memory.raw(), .load);
    try t.expect(owner.status.failure.codec == .version);
    const badnumber = try std.mem.replaceOwned(u8, a, json, "\"hazard_period_ticks\":\"2700\"", "\"hazard_period_ticks\":\"02700\"");
    defer a.free(badnumber);
    try setSlot(&memory, badnumber);
    _ = try owner.dispatch(&core, memory.raw(), .load);
    try t.expect(owner.status.failure.codec == .canonical_number);
    try t.expectEqualStrings("rules.hazard_period_ticks", owner.status.failure.codec.canonical_number.field);
    const huge = try a.alloc(u8, 65_537);
    defer a.free(huge);
    @memset(huge, 'x');
    try setSlot(&memory, huge);
    _ = try owner.dispatch(&core, memory.raw(), .load);
    try t.expect(owner.status.failure.codec == .too_large);
    try t.expectEqual(tx.Outcome.modal_opened, try owner.dispatch(&core, memory.raw(), .save));
    _ = try owner.dispatch(&core, memory.raw(), .dismiss_confirmation);
    try setSlot(&memory, "\xff");
    _ = try owner.dispatch(&core, memory.raw(), .load);
    try t.expect(owner.status.failure.codec == .json);
    try t.expectEqual(error.InvalidUtf8, owner.status.failure.codec.json);
    try t.expect(!owner.session.?.isDirty());
    try t.expectEqualStrings("ASTER VALE", owner.session.?.original().worlds[0].name);
}
const Editor = @import("classic_editor.zig");
const Snapshot = struct {
    current: scenario.ScenarioDraft,
    original: scenario.ScenarioDraft,
    buffers: [90][]u8,
    preedit: []u8,
    errors: [90]bool,
    section: Editor.Section,
    selected: usize,
    focus: ?Editor.Focus,
    scroll: f32,
    status: tx.Status,
    action_failure: ?tx.Failure,
    core: app.AppCore,
    pending: []model.CommandEnvelope,
    fn init(owner: *tx.Transactions, core: *app.AppCore) !Snapshot {
        const s = &owner.session.?;
        var result = Snapshot{ .current = try s.copyCurrent(a), .original = undefined, .buffers = undefined, .preedit = undefined, .errors = s._current.errors, .section = s.section, .selected = s.selected_world, .focus = s.focus, .scroll = s.scroll, .status = owner.status, .action_failure = owner.action_failure, .core = core.*, .pending = undefined };
        errdefer result.current.deinit();
        result.original = try s.copyOriginal(a);
        errdefer result.original.deinit();
        var count: usize = 0;
        errdefer for (result.buffers[0..count]) |bytes| a.free(bytes);
        for (Editor.catalog, 0..) |field, i| {
            result.buffers[i] = try a.dupe(u8, s.fieldText(field.text_id).?);
            count += 1;
        }
        result.preedit = try a.dupe(u8, s.preedit());
        errdefer a.free(result.preedit);
        result.pending = try a.dupe(model.CommandEnvelope, core._simulation.pending.items);
        errdefer a.free(result.pending);
        if (result.status == .failure and result.status.failure == .storage) result.status.failure.storage = try result.status.failure.storage.clone(a);
        return result;
    }
    fn deinit(self: *Snapshot) void {
        self.current.deinit();
        self.original.deinit();
        for (self.buffers) |bytes| a.free(bytes);
        a.free(self.preedit);
        a.free(self.pending);
        if (self.status == .failure and self.status.failure == .storage) self.status.failure.storage.deinit(a);
    }
    fn expectResident(self: *const Snapshot, owner: *tx.Transactions, core: *app.AppCore) !void {
        const s = &owner.session.?;
        const current = s.current();
        const original = s.original();
        try t.expectEqual(self.current.seed, current.seed);
        try t.expectEqualDeep(self.current.rules, current.rules);
        try t.expectEqualDeep(self.current.worlds, current.worlds);
        try t.expectEqual(self.original.seed, original.seed);
        try t.expectEqualDeep(self.original.rules, original.rules);
        try t.expectEqualDeep(self.original.worlds, original.worlds);
        for (Editor.catalog, self.buffers) |field, bytes| try t.expectEqualSlices(u8, bytes, s.fieldText(field.text_id).?);
        try t.expectEqualDeep(self.errors, s._current.errors);
        try t.expectEqualStrings(self.preedit, s.preedit());
        try t.expectEqual(self.section, s.section);
        try t.expectEqual(self.selected, s.selected_world);
        try t.expectEqualDeep(self.focus, s.focus);
        try t.expectEqual(self.scroll, s.scroll);
        try t.expectEqualDeep(self.status, owner.status);
        try t.expectEqualDeep(self.action_failure, owner.action_failure);
        try t.expectEqualDeep(self.core._active_scenario, core._active_scenario);
        try t.expectEqualDeep(self.core._simulation.state, core._simulation.state);
        try t.expectEqualDeep(self.pending, core._simulation.pending.items);
        inline for (comptime std.meta.fieldNames(app.AppCore)) |field| {
            if (comptime !std.mem.eql(u8, field, "_allocator") and !std.mem.eql(u8, field, "_active_scenario") and !std.mem.eql(u8, field, "_simulation")) try t.expectEqualDeep(@field(self.core, field), @field(core, field));
        }
    }
};
fn openAllocations(allocator: std.mem.Allocator) !void {
    var core = try ready();
    defer core.deinit();
    var owner = tx.Transactions.init(allocator);
    defer owner.deinit();
    const before = core.canonicalFingerprint();
    _ = owner.open(&core) catch |cause| {
        try t.expect(owner.session == null and owner.status == .none);
        try t.expectEqual(app.AppMode.playing, core.mode());
        try t.expectEqual(before, core.canonicalFingerprint());
        return cause;
    };
}
// Supplementary exhaustion tests: every allocation ordinal in open must preserve core and leak nothing.
test "3D exhaustive opening allocation failures preserve inactive owner and core" {
    try t.checkAllAllocationFailures(a, openAllocations, .{});
}
const Operation = enum { load_clean, load_confirm, save_absent, save_occupied, save_confirm, save_failure_confirm, load_failure, save_failure, revert, factory, regenerate };
// Supplementary exhaustion tests: a request must not publish before allocation succeeds;
// a confirmation must take its capsule even when the operation subsequently runs out of memory.
test "3D exhaustive operation allocation ordinals preserve full resident state consume confirms and retry" {
    for (std.meta.tags(Operation)) |operation| {
        var ordinal: usize = 0;
        while (ordinal < 128) : (ordinal += 1) {
            var fail = t.FailingAllocator.init(a, .{});
            const allocator = fail.allocator();
            var core = try ready();
            defer core.deinit();
            _ = try core.enqueueGameCommand(.{ .launch = .{ .source = 0, .destination = 3 } });
            var memory = store.MemoryStore.init(allocator, .scenario);
            defer memory.deinit();
            var owner = tx.Transactions.init(allocator);
            defer owner.deinit();
            _ = try owner.open(&core);
            const json = try scenario.encode(a, core.activeScenario());
            defer a.free(json);
            try owner.session.?.setPreedit("OOM visual preedit");
            var action: Editor.Action = undefined;
            var confirmation = false;
            switch (operation) {
                .load_clean => {
                    var saved = try memory.raw().save(a, json);
                    saved.deinit(a);
                    action = .load;
                },
                .load_confirm => {
                    var saved = try memory.raw().save(a, json);
                    saved.deinit(a);
                    try edit(&owner, "world.0.name", "DIRTY");
                    try edit(&owner, "scenario.seed", "INVALID TEXT");
                    _ = try owner.dispatch(&core, memory.raw(), .load);
                    action = .confirm;
                    confirmation = true;
                },
                .save_absent => action = .save,
                .save_occupied => {
                    var saved = try memory.raw().save(a, "old");
                    saved.deinit(a);
                    action = .save;
                },
                .save_confirm, .save_failure_confirm => {
                    var saved = try memory.raw().save(a, "old");
                    saved.deinit(a);
                    _ = try owner.dispatch(&core, memory.raw(), .save);
                    if (operation == .save_failure_confirm) try memory.failNextSave("failure message allocation");
                    action = .confirm;
                    confirmation = true;
                },
                .load_failure => {
                    try memory.denyLoad("load message allocation");
                    action = .load;
                },
                .save_failure => {
                    try memory.failNextSave("save message allocation");
                    action = .save;
                },
                .revert, .factory, .regenerate => {
                    try edit(&owner, "world.0.name", "DIRTY");
                    try edit(&owner, "rule.hazard_period_ticks", "0");
                    try edit(&owner, "scenario.seed", "INVALID TEXT");
                    const request: Editor.Action = switch (operation) {
                        .revert => .revert,
                        .factory => .factory_defaults,
                        .regenerate => .regenerate_from_seed,
                        else => unreachable,
                    };
                    _ = try owner.dispatch(&core, memory.raw(), request);
                    action = .confirm;
                    confirmation = true;
                },
            }
            // Low-level presentation state remains meaningful while the modal is active.
            owner.session.?.section = .rules;
            owner.session.?.selected_world = 5;
            owner.session.?.scroll = 12;
            owner.session.?.focus = .{ .field = .{ .global = .seed } };
            owner.clearStatus();
            owner.status = .{ .failure = .{ .storage = .{ .record = .scenario, .kind = .browser, .message = try allocator.dupe(u8, "prior status") } } };
            owner.action_failure = .stale_confirmation;
            var snapshot = try Snapshot.init(&owner, &core);
            defer snapshot.deinit();
            const oldslot = if (memory.slot) |bytes| try a.dupe(u8, bytes) else null;
            defer if (oldslot) |bytes| a.free(bytes);
            const modal_before = owner.session.?.modal.kind;
            fail.fail_index = fail.alloc_index + ordinal;
            fail.resize_fail_index = fail.resize_index;
            const result = owner.dispatch(&core, memory.raw(), action);
            fail.fail_index = std.math.maxInt(usize);
            fail.resize_fail_index = std.math.maxInt(usize);
            if (result) |_| {
                try t.expect(!fail.has_induced_failure);
                break;
            } else |cause| {
                try t.expectEqual(error.OutOfMemory, cause);
                try t.expect(fail.has_induced_failure);
                try snapshot.expectResident(&owner, &core);
                if (oldslot) |bytes| try t.expectEqualSlices(u8, bytes, memory.slot.?) else try t.expect(memory.slot == null);
                if (confirmation) {
                    try t.expect(owner.session.?.modal.kind == null and owner.session.?.modal.save_bytes == null and owner.session.?.modal.apply_candidate == null);
                    try t.expectEqual(tx.Outcome.failed, try owner.dispatch(&core, memory.raw(), .confirm));
                    const request: Editor.Action = switch (operation) {
                        .load_confirm => .load,
                        .save_confirm, .save_failure_confirm => .save,
                        .revert => .revert,
                        .factory => .factory_defaults,
                        .regenerate => .regenerate_from_seed,
                        else => unreachable,
                    };
                    // Load needs a dirty request after an unchanged failed attempt, which this fixture retains.
                    _ = try owner.dispatch(&core, memory.raw(), request);
                    _ = try owner.dispatch(&core, memory.raw(), .confirm);
                } else {
                    try t.expectEqual(modal_before, owner.session.?.modal.kind);
                    _ = try owner.dispatch(&core, memory.raw(), action);
                }
            }
        }
        try t.expect(ordinal > 0 and ordinal < 128);
        std.debug.print("memory allocation family {s}: {d} failed ordinals then success\n", .{ @tagName(operation), ordinal });
    }
}
fn nativePathAllocations(allocator: std.mem.Allocator) !void {
    var f = try Fixture.init();
    defer f.deinit();
    var profile = try store.NativeStore.fromProfile(allocator, io, .scenario, .macos, f.root);
    defer profile.deinit();
    const primary = try f.path("scenario-v1.json");
    defer a.free(primary);
    const legacy = try f.path("legacy.json");
    defer a.free(legacy);
    var paths = try store.NativeStore.atPaths(allocator, io, .scenario, primary, legacy);
    defer paths.deinit();
}
// Supplementary native allocation composition exercises owned paths without actual user profiles.
test "3D exhaustive native path construction allocations release all owners" {
    try t.checkAllAllocationFailures(a, nativePathAllocations, .{});
}
const NativeOperation = enum { load_clean, load_confirm, save_absent, save_confirm, save_failure_confirm };
test "3D exhaustive native transaction allocations preserve records cleanup temps consume confirms" {
    for (std.meta.tags(NativeOperation)) |operation| {
        var ordinal: usize = 0;
        while (ordinal < 128) : (ordinal += 1) {
            var f = try Fixture.init();
            defer f.deinit();
            var fail = t.FailingAllocator.init(a, .{});
            const allocator = fail.allocator();
            var native = try store.NativeStore.fromProfile(allocator, io, .scenario, .macos, f.root);
            defer native.deinit();
            var core = try ready();
            defer core.deinit();
            _ = try core.enqueueGameCommand(.{ .launch = .{ .source = 0, .destination = 3 } });
            var owner = tx.Transactions.init(allocator);
            defer owner.deinit();
            _ = try owner.open(&core);
            const old = try scenario.encode(a, core.activeScenario());
            defer a.free(old);
            try f.write("preferences-v1.json", "preference sentinel");
            try f.write("legacy.json", "legacy sentinel");
            if (operation != .save_absent) try f.write("scenario-v1.json", old);
            var action: Editor.Action = undefined;
            var confirmation = false;
            switch (operation) {
                .load_clean => action = .load,
                .load_confirm => {
                    try edit(&owner, "world.0.name", "DIRTY");
                    _ = try owner.dispatch(&core, native.raw(), .load);
                    action = .confirm;
                    confirmation = true;
                },
                .save_absent => action = .save,
                .save_confirm, .save_failure_confirm => {
                    try edit(&owner, "world.0.name", "NEW NATIVE");
                    _ = try owner.dispatch(&core, native.raw(), .save);
                    if (operation == .save_failure_confirm) try native.failNext(.persist, "OOM FAULT MESSAGE");
                    action = .confirm;
                    confirmation = true;
                },
            }
            owner.clearStatus();
            owner.status = .{ .failure = .{ .storage = .{ .record = .scenario, .kind = .browser, .message = try allocator.dupe(u8, "prior native status") } } };
            owner.action_failure = .stale_confirmation;
            var snapshot = try Snapshot.init(&owner, &core);
            defer snapshot.deinit();
            fail.fail_index = fail.alloc_index + ordinal;
            fail.resize_fail_index = fail.resize_index;
            const result = owner.dispatch(&core, native.raw(), action);
            fail.fail_index = std.math.maxInt(usize);
            fail.resize_fail_index = std.math.maxInt(usize);
            try f.noTemps();
            try expectFile(&f, "legacy.json", "legacy sentinel");
            try expectFile(&f, "preferences-v1.json", "preference sentinel");
            if (result) |outcome| {
                try t.expect(!fail.has_induced_failure);
                if (operation == .save_absent or operation == .save_confirm) {
                    try t.expectEqual(tx.Outcome.saved, outcome);
                    try t.expect(owner.status == .saved);
                }
                break;
            } else |cause| {
                try t.expectEqual(error.OutOfMemory, cause);
                try t.expect(fail.has_induced_failure);
                try snapshot.expectResident(&owner, &core);
                if (operation == .save_absent) try t.expectError(error.FileNotFound, f.dir.openFile(io, "scenario-v1.json", .{})) else try expectFile(&f, "scenario-v1.json", old);
                try t.expect(owner.session.?.modal.kind == null);
                if (confirmation) {
                    try t.expectEqual(tx.Outcome.failed, try owner.dispatch(&core, native.raw(), .confirm));
                    _ = try owner.dispatch(&core, native.raw(), if (operation == .load_confirm) .load else .save);
                    _ = try owner.dispatch(&core, native.raw(), .confirm);
                } else _ = try owner.dispatch(&core, native.raw(), action);
                try f.noTemps();
            }
        }
        try t.expect(ordinal > 0 and ordinal < 128);
        std.debug.print("native allocation family {s}: {d} failed ordinals then success\n", .{ @tagName(operation), ordinal });
    }
}
// Supplementary owner lifetime test catches stores retained beside their own facade, captured
// slices borrowed from current epochs, and double release under repeated moves/replacements.
test "3D repeated owner store moves reset capture dismiss and confirm lifetimes" {
    var core = try ready();
    defer core.deinit();
    var memory = try store.MemoryStore.withSlot(a, .scenario, "old");
    var memory_live = true;
    defer if (memory_live) memory.deinit();
    var owner = tx.Transactions.init(a);
    var owner_live = true;
    defer if (owner_live) owner.deinit();
    _ = try owner.open(&core);
    for (0..12) |index| {
        try edit(&owner, "world.0.name", "MOVED OWNER");
        _ = try owner.dispatch(&core, memory.raw(), .save);
        const capture = try a.dupe(u8, owner.session.?.modal.save_bytes.?);
        defer a.free(capture);
        var draft = try scenario.ScenarioDraft.factoryDefault(a);
        draft.worlds[0].name = "AFTER CAPTURE";
        try owner.session.?.replaceCurrent(&draft);
        draft.deinit();
        var moved = owner;
        owner = undefined;
        owner_live = false;
        var moved_store = memory;
        memory = undefined;
        memory_live = false; // recreate raw only after move
        if (index % 2 == 0) {
            try t.expectEqual(tx.Outcome.saved, try moved.dispatch(&core, moved_store.raw(), .confirm));
            try t.expectEqualSlices(u8, capture, moved_store.slot.?);
        } else {
            const read_calls = moved_store.load_calls;
            const save_calls = moved_store.save_calls;
            _ = try moved.dispatch(&core, moved_store.raw(), .dismiss_confirmation);
            try t.expectEqual(read_calls, moved_store.load_calls);
            try t.expectEqual(save_calls, moved_store.save_calls);
        }
        _ = try moved.dispatch(&core, moved_store.raw(), .regenerate_from_seed);
        _ = try moved.dispatch(&core, moved_store.raw(), .confirm);
        owner = moved;
        moved = undefined;
        owner_live = true;
        memory = moved_store;
        moved_store = undefined;
        memory_live = true;
        try t.expectEqualStrings("ASTER VALE", owner.session.?.current().worlds[0].name);
        try t.expectEqualStrings("ASTER VALE", owner.session.?.original().worlds[0].name);
    }
    _ = try owner.dispatch(&core, memory.raw(), .apply_and_restart);
    try owner.session.?.replaceFocusedText("discarded");
    try t.expect(owner.session.?.modal.kind == null);
    try t.expectEqual(tx.Outcome.failed, try owner.dispatch(&core, memory.raw(), .confirm));
    try t.expect(owner.action_failure.? == .no_confirmation);
}
// Supplementary comparison coverage uses *live* warning fields with equal report counts,
// so malformed lengths reach the safe comparator instead of a prior count mismatch.
test "3D live warning comparison bounds fields messages and compares complete meaning" {
    var core = try ready();
    defer core.deinit();
    var memory = store.MemoryStore.init(a, .scenario);
    defer memory.deinit();
    var owner = tx.Transactions.init(a);
    defer owner.deinit();
    _ = try owner.open(&core);
    var draft = try owner.session.?.copyCurrent(a);
    defer draft.deinit();
    draft.worlds[1].name = draft.worlds[0].name;
    try owner.session.?.replaceCurrent(&draft);
    try t.expect(owner.session.?.validationReport().warningCount() > 0);
    for (0..7) |variant| {
        _ = try owner.dispatch(&core, memory.raw(), .apply_and_restart);
        const issue = &owner.session.?.modal.apply_candidate.?.warnings.entries[0];
        switch (variant) {
            0 => issue.field.len = 49,
            1 => issue.message.len = 97,
            2 => issue.severity = .err,
            3 => issue.code = .tick_rate,
            4 => issue.field.bytes[0] = '!',
            5 => issue.message.bytes[0] = '!',
            6 => owner.session.?.modal.apply_candidate.?.worlds[0].production_remainder = 1,
            else => unreachable,
        }
        try t.expectEqual(tx.Outcome.failed, try owner.dispatch(&core, memory.raw(), .confirm));
        try t.expect(owner.action_failure.? == .stale_confirmation);
        try t.expect(owner.session.?.modal.kind == null);
        try t.expectEqual(model.DEFAULT_SEED, core.activeScenario().seed);
        try t.expectEqual(@as(u64, 0), core.campaign().next_tick);
    }
}
// Supplementary absence/failure behavior must consume a LoadDirty attempt while leaving
// parse spelling, ORIGINAL and IME presentation unchanged.
test "3D dirty confirmed absent malformed and dismissed load preserve raw editor contents" {
    var core = try ready();
    defer core.deinit();
    var memory = store.MemoryStore.init(a, .scenario);
    defer memory.deinit();
    var owner = tx.Transactions.init(a);
    defer owner.deinit();
    _ = try owner.open(&core);
    try edit(&owner, "scenario.seed", "bad spelling");
    try owner.session.?.setPreedit("PREEDIT");
    _ = try owner.dispatch(&core, memory.raw(), .load);
    const reads = memory.load_calls;
    _ = try owner.dispatch(&core, memory.raw(), .dismiss_confirmation);
    try t.expectEqual(reads, memory.load_calls);
    _ = try owner.dispatch(&core, memory.raw(), .load);
    try t.expectEqual(tx.Outcome.absent, try owner.dispatch(&core, memory.raw(), .confirm));
    try t.expectEqualStrings("bad spelling", owner.session.?.fieldText("scenario.seed").?);
    try t.expectEqualStrings("PREEDIT", owner.session.?.preedit());
    try t.expect(owner.session.?.hasParseErrors() and owner.session.?.modal.kind == null);
    try setSlot(&memory, "not json");
    _ = try owner.dispatch(&core, memory.raw(), .load);
    try t.expectEqual(tx.Outcome.failed, try owner.dispatch(&core, memory.raw(), .confirm));
    try t.expect(owner.status.failure == .codec);
    try t.expectEqualStrings("bad spelling", owner.session.?.fieldText("scenario.seed").?);
    try t.expectEqualStrings("PREEDIT", owner.session.?.preedit());
    try t.expectEqual(model.DEFAULT_SEED, owner.session.?.original().seed);
}
// Supplementary revert/factory distinction uses an independently named nondefault INITIAL.
test "3D revert restores custom original while factory retains original and prior message" {
    var initial = try scenario.ScenarioDraft.factoryDefault(a);
    defer initial.deinit();
    initial.seed = 9;
    initial.worlds[0].name = "CUSTOM ORIGINAL";
    const value = try initial.validated();
    var core = try app.AppCore.init(a, &value, .{ .onboarding_completed = true });
    defer core.deinit();
    var memory = store.MemoryStore.init(a, .scenario);
    defer memory.deinit();
    var owner = tx.Transactions.init(a);
    defer owner.deinit();
    _ = try owner.open(&core);
    _ = try owner.dispatch(&core, memory.raw(), .load);
    try edit(&owner, "world.0.name", "CURRENT");
    _ = try owner.dispatch(&core, memory.raw(), .factory_defaults);
    _ = try owner.dispatch(&core, memory.raw(), .confirm);
    try t.expectEqual(model.DEFAULT_SEED, owner.session.?.current().seed);
    try t.expectEqual(@as(u64, 9), owner.session.?.original().seed);
    try t.expect(owner.session.?.isDirty());
    try t.expect(owner.status == .no_saved_scenario);
    _ = try owner.dispatch(&core, memory.raw(), .revert);
    _ = try owner.dispatch(&core, memory.raw(), .confirm);
    try t.expectEqual(@as(u64, 9), owner.session.?.current().seed);
    try t.expectEqualStrings("CUSTOM ORIGINAL", owner.session.?.current().worlds[0].name);
    try t.expect(!owner.session.?.isDirty());
    try t.expect(owner.status == .no_saved_scenario);
}
// Supplementary allocation-free capture/install: zero remaining allocation capacity cannot
// fail apply. Move the transaction before confirmation, with inline names no borrowed epoch.
test "3D apply capture install allocate nothing after staging and survive owner movement" {
    var fail = t.FailingAllocator.init(a, .{});
    var core = try ready();
    defer core.deinit();
    var memory = store.MemoryStore.init(a, .scenario);
    defer memory.deinit();
    var owner = tx.Transactions.init(fail.allocator());
    var live = true;
    defer if (live) owner.deinit();
    _ = try owner.open(&core);
    try edit(&owner, "world.0.name", "INLINE CAPTURE");
    fail.fail_index = fail.alloc_index;
    try t.expectEqual(tx.Outcome.modal_opened, try owner.dispatch(&core, memory.raw(), .apply_and_restart));
    var moved = owner;
    owner = undefined;
    live = false;
    defer moved.deinit();
    try t.expectEqual(tx.Outcome.applied, try moved.dispatch(&core, memory.raw(), .confirm));
    try t.expect(!fail.has_induced_failure);
    try t.expectEqualStrings("INLINE CAPTURE", core.activeScenario().worlds[0].name.asSlice());
}
// Break: reporting an action-only error replaces the independently owned store-message channel.
test "3D action failures retain prior absent saved loaded and owned store failure status" {
    for (0..4) |prior| {
        var core = try ready();
        defer core.deinit();
        var memory = store.MemoryStore.init(a, .scenario);
        defer memory.deinit();
        var owner = tx.Transactions.init(a);
        defer owner.deinit();
        _ = try owner.open(&core);
        switch (prior) {
            0 => {
                _ = try owner.dispatch(&core, memory.raw(), .load);
            },
            1 => {
                _ = try owner.dispatch(&core, memory.raw(), .save);
            },
            2 => {
                _ = try owner.dispatch(&core, memory.raw(), .save);
                _ = try owner.dispatch(&core, memory.raw(), .load);
            },
            3 => {
                try memory.denyLoad("PERSISTENT PRIOR STORE ERROR");
                _ = try owner.dispatch(&core, memory.raw(), .load);
            },
            else => unreachable,
        }
        var expected = owner.status;
        if (expected == .failure and expected.failure == .storage) expected.failure.storage = try expected.failure.storage.clone(a);
        defer if (expected == .failure and expected.failure == .storage) expected.failure.storage.deinit(a);
        try edit(&owner, "scenario.seed", "invalid text");
        for ([_]Editor.Action{ .save, .apply_and_restart }) |action| {
            try t.expectEqual(tx.Outcome.failed, try owner.dispatch(&core, memory.raw(), action));
            try t.expectEqualDeep(expected, owner.status);
            try t.expect(owner.action_failure.? == .invalid_text);
        }
        try edit(&owner, "scenario.seed", "4947573120260902");
        try edit(&owner, "rule.hazard_period_ticks", "0");
        for ([_]Editor.Action{ .save, .apply_and_restart }) |action| {
            try t.expectEqual(tx.Outcome.failed, try owner.dispatch(&core, memory.raw(), action));
            try t.expectEqualDeep(expected, owner.status);
            if (action == .save) try t.expect(owner.action_failure.?.codec == .validation) else try t.expect(owner.action_failure.? == .validation);
        }
        try t.expectEqual(tx.Outcome.failed, try owner.dispatch(&core, memory.raw(), .confirm));
        try t.expect(owner.action_failure.? == .no_confirmation);
        try t.expectEqualDeep(expected, owner.status);
        _ = try owner.dispatch(&core, memory.raw(), .revert);
        for ([_]Editor.Action{ .load, .save, .apply_and_restart }) |action| {
            try t.expectEqual(tx.Outcome.failed, try owner.dispatch(&core, memory.raw(), action));
            try t.expectEqualDeep(expected, owner.status);
            try t.expect(owner.action_failure.? == .confirmation_active);
        }
        _ = try owner.dispatch(&core, memory.raw(), .confirm);
        try t.expect(owner.action_failure == null);
        try t.expectEqualDeep(expected, owner.status);
        _ = try owner.dispatch(&core, memory.raw(), .apply_and_restart);
        owner.session.?.modal.apply_candidate.?.worlds[0].energy += 1;
        try t.expectEqual(tx.Outcome.failed, try owner.dispatch(&core, memory.raw(), .confirm));
        try t.expectEqualDeep(expected, owner.status);
        try t.expect(owner.action_failure.? == .stale_confirmation);
        for ([_]Editor.Action{ .factory_defaults, .regenerate_from_seed }) |action| {
            _ = try owner.dispatch(&core, memory.raw(), action);
            _ = try owner.dispatch(&core, memory.raw(), .confirm);
            try t.expectEqualDeep(expected, owner.status);
            try t.expect(owner.action_failure == null);
        }
        _ = try owner.dispatch(&core, memory.raw(), .confirm);
        try t.expect(owner.action_failure.? == .no_confirmation);
        try setSlot(&memory, "malformed saved JSON");
        try t.expectEqual(tx.Outcome.failed, try owner.dispatch(&core, memory.raw(), .load));
        try t.expect(owner.action_failure == null);
        if (prior == 3) try t.expect(owner.status.failure == .storage) else try t.expect(owner.status.failure == .codec);
    }
}
