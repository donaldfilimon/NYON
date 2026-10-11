const std = @import("std");
const t = std.testing;
const e = @import("classic_editor.zig");
const s = @import("scenario.zig");
// Catches omitted fields, wrong field mapping and incorrectly accepted IDs.
test "Classic editor builds exact catalog and detached invalid originals" {
    var draft = try s.ScenarioDraft.factoryDefault(t.allocator);
    defer draft.deinit();
    draft.worlds[0].name = "lowercase 🪐 much longer than twenty four bytes";
    var editor = try e.EditorState.init(t.allocator, &draft);
    defer editor.deinit();
    try t.expectEqualStrings("4947573120260902", editor.fieldText("scenario.seed").?);
    try t.expectEqualStrings("60", editor.fieldText("rule.tick_hz").?);
    try t.expectEqualStrings("5", editor.fieldText("rule.win_world_count").?);
    try t.expectEqualStrings("lowercase 🪐 much longer than twenty four bytes", editor.fieldText("world.0.name").?);
    try t.expect(editor.original().worlds[0].name.ptr != draft.worlds[0].name.ptr);
    try t.expect(editor.current().worlds[0].name.ptr != editor.original().worlds[0].name.ptr);
    for ([_][]const u8{ "world.+0.name", "world.00.name", "world.7.name", "world.0.name.extra", "rules.tick_hz", "world.0", "scenario.seed." }) |id| {
        try t.expectEqual(@as(?[]const u8, null), editor.fieldText(id));
        try t.expectError(error.UnknownWidget, editor.focusField(id));
    }
    try editor.focusField("world.0.name");
    try t.expectError(error.UnknownWidget, editor.focusField("rule.tick_hz"));
    try t.expectEqual(@as(usize, 13), editor.focus.?.field.slot());
    try t.expectError(error.UnknownWidget, editor.focusField("world.1.name"));
    try t.expectError(error.UnknownWidget, editor.focusField("rule.maximum_energy"));
}
fn fixture(allocator: std.mem.Allocator) !s.ScenarioDraft {
    var draft = try s.ScenarioDraft.factoryDefault(allocator);
    draft.seed = 0x0123456789ABCDEF;
    draft.rules = .{ .tick_hz = 60, .win_world_count = 3, .ai_period_ticks = 101, .hazard_first_tick = 102, .hazard_period_ticks = 103, .hazard_duration_ticks = 104, .maximum_energy = 105, .maximum_defense = 106, .minimum_launch = 107, .field_raise_cost = 108, .field_lower_refund = 109, .base_fleet_speed = 110 };
    for (&draft.worlds) |*w| w.* = .{ .name = "ASTER", .x = -12, .y = 34, .owner = .choir, .energy = 201, .defense = 202, .base_output = 203, .base_regeneration = 204, .atmosphere = 5, .hydrosphere = 6, .topology = 7 };
    return draft;
}
// Catches cross-wired assignments across all 90 exact IDs, including every world.
test "Classic editor exact field catalog formats and edits all destinations" {
    var draft = try fixture(t.allocator);
    defer draft.deinit();
    var editor = try e.EditorState.init(t.allocator, &draft);
    defer editor.deinit();
    const globals = [_][]const u8{ "scenario.seed", "rule.tick_hz", "rule.win_world_count", "rule.ai_period_ticks", "rule.hazard_first_tick", "rule.hazard_period_ticks", "rule.hazard_duration_ticks", "rule.maximum_energy", "rule.maximum_defense", "rule.minimum_launch", "rule.field_raise_cost", "rule.field_lower_refund", "rule.base_fleet_speed" };
    const values = [_][]const u8{ "0123456789ABCDEF", "60", "3", "101", "102", "103", "104", "105", "106", "107", "108", "109", "110" };
    editor.section = .rules;
    for (globals, values, 0..) |id, value, i| {
        try t.expectEqualStrings(value, editor.fieldText(id).?);
        if (i == 1) {
            try t.expectError(error.UnknownWidget, editor.focusField(id));
            continue;
        }
        try editor.focusField(id);
        try editor.replaceFocusedText(if (i == 0) "0000000000000042" else "+0042");
        try t.expectEqualStrings(if (i == 0) "0000000000000042" else "42", editor.fieldText(id).?);
    }
    const rules = editor.current().rules;
    try t.expectEqual(@as(u64, 0x42), editor.current().seed);
    try t.expectEqual(@as(u32, 60), rules.tick_hz);
    try t.expectEqual(@as(u8, 42), rules.win_world_count);
    try t.expectEqual(@as(u64, 42), rules.ai_period_ticks);
    try t.expectEqual(@as(u64, 42), rules.hazard_first_tick);
    try t.expectEqual(@as(u64, 42), rules.hazard_period_ticks);
    try t.expectEqual(@as(u64, 42), rules.hazard_duration_ticks);
    try t.expectEqual(@as(u64, 42), rules.maximum_energy);
    try t.expectEqual(@as(u64, 42), rules.maximum_defense);
    try t.expectEqual(@as(u64, 42), rules.minimum_launch);
    try t.expectEqual(@as(u64, 42), rules.field_raise_cost);
    try t.expectEqual(@as(u64, 42), rules.field_lower_refund);
    try t.expectEqual(@as(u64, 42), rules.base_fleet_speed);
    const suffixes = [_][]const u8{ "name", "x", "y", "owner", "energy", "defense", "base_output", "base_regeneration", "atmosphere", "hydrosphere", "topology" };
    const initial = [_][]const u8{ "ASTER", "-12", "34", "CHOIR", "201", "202", "203", "204", "5", "6", "7" };
    const replacements = [_][]const u8{ "NEW NAME", "+0043", "-0044", "HELIX", "+0045", "+0046", "+0047", "+0048", "+0008", "+0009", "+0010" };
    const normalized = [_][]const u8{ "NEW NAME", "43", "-44", "HELIX", "45", "46", "47", "48", "8", "9", "10" };
    for (0..7) |i| {
        editor.section = .worlds;
        editor.selected_world = i;
        for (suffixes, initial, replacements, normalized) |suffix, before, input, want| {
            var id_buf: [64]u8 = undefined;
            const id = try std.fmt.bufPrint(&id_buf, "world.{d}.{s}", .{ i, suffix });
            try t.expectEqualStrings(before, editor.fieldText(id).?);
            try editor.focusField(id);
            try editor.replaceFocusedText(input);
            try t.expectEqualStrings(want, editor.fieldText(id).?);
        }
        const w = editor.current().worlds[i];
        try t.expectEqualStrings("NEW NAME", w.name);
        try t.expectEqual(@as(i32, 43), w.x);
        try t.expectEqual(@as(i32, -44), w.y);
        try t.expectEqual(@as(?@import("model.zig").Faction, .helix), w.owner);
        try t.expectEqual(@as(u64, 45), w.energy);
        try t.expectEqual(@as(u64, 46), w.defense);
        try t.expectEqual(@as(u64, 47), w.base_output);
        try t.expectEqual(@as(u64, 48), w.base_regeneration);
        try t.expectEqual(@as(u8, 8), w.atmosphere);
        try t.expectEqual(@as(u8, 9), w.hydrosphere);
        try t.expectEqual(@as(u8, 10), w.topology);
    }
}
// Catches accidentally reusing canonical wire grammar or accepting overflow.
test "Classic editor decimal parser matches Rust plus zeroes signed bounds and ASCII" {
    const Row = struct { id: []const u8, input: []const u8, normalized: ?[]const u8 };
    const rows = [_]Row{
        .{ .id = "world.0.atmosphere", .input = "+0", .normalized = "0" },
        .{ .id = "world.0.atmosphere", .input = "01", .normalized = "1" },
        .{ .id = "world.0.atmosphere", .input = "255", .normalized = "255" },
        .{ .id = "world.0.atmosphere", .input = "256", .normalized = null },
        .{ .id = "world.0.atmosphere", .input = "-0", .normalized = null },
        .{ .id = "world.0.energy", .input = "18446744073709551615", .normalized = "18446744073709551615" },
        .{ .id = "world.0.energy", .input = "18446744073709551616", .normalized = null },
        .{ .id = "world.0.energy", .input = "-00", .normalized = null },
        .{ .id = "world.0.x", .input = "-00", .normalized = "0" },
        .{ .id = "world.0.x", .input = "2147483647", .normalized = "2147483647" },
        .{ .id = "world.0.x", .input = "2147483648", .normalized = null },
        .{ .id = "world.0.x", .input = "-2147483648", .normalized = "-2147483648" },
        .{ .id = "world.0.x", .input = "-2147483649", .normalized = null },
    };
    var draft = try s.ScenarioDraft.factoryDefault(t.allocator);
    defer draft.deinit();
    var editor = try e.EditorState.init(t.allocator, &draft);
    defer editor.deinit();
    for (rows) |row| {
        try editor.focusField(row.id);
        const before = editor.current();
        try editor.replaceFocusedText(row.input);
        try t.expectEqualStrings(row.normalized orelse row.input, editor.fieldText(row.id).?);
        try t.expectEqual(row.normalized == null, editor._current.errors[e.lookup(row.id).?.id.slot()]);
        if (row.normalized == null) {
            try t.expectEqual(before.worlds[0].energy, editor.current().worlds[0].energy);
            try t.expectEqual(before.worlds[0].x, editor.current().worlds[0].x);
            try t.expectEqual(before.worlds[0].atmosphere, editor.current().worlds[0].atmosphere);
        }
    }
    for ([_][]const u8{ "", "+", "-", " 1", "1 ", "\t1", "1\n", "1_0", "0x10", "１", "١" }) |input| for ([_][]const u8{ "world.0.x", "world.0.energy", "world.0.atmosphere" }) |id| {
        try editor.focusField(id);
        try editor.replaceFocusedText(input);
        try t.expect(editor._current.errors[e.lookup(id).?.id.slot()]);
        try t.expectEqualStrings(input, editor.fieldText(id).?);
    };
    var zeros: [4097]u8 = @splat('0');
    zeros[4096] = '1';
    for ([_][]const u8{ "world.0.x", "world.0.energy", "world.0.atmosphere" }) |id| {
        try editor.focusField(id);
        try editor.replaceFocusedText(&zeros);
        try t.expectEqualStrings("1", editor.fieldText(id).?);
    }
    var negative_zeros: [4098]u8 = @splat('0');
    negative_zeros[0] = '-';
    negative_zeros[4097] = '1';
    try editor.focusField("world.0.x");
    try editor.replaceFocusedText(&negative_zeros);
    try t.expectEqualStrings("-1", editor.fieldText("world.0.x").?);
}
// Catches plus+15 hex and lowerhex being incorrectly rejected or left unnormalized.
test "Classic editor seed parsing and exact owner spelling" {
    const Row = struct { input: []const u8, want: ?u64, text: []const u8 };
    const rows = [_]Row{
        .{ .input = "0123456789abcdef", .want = 0x0123456789ABCDEF, .text = "0123456789ABCDEF" },
        .{ .input = "+123456789abcdef", .want = 0x0123456789ABCDEF, .text = "0123456789ABCDEF" },
        .{ .input = "+00000000000000F", .want = 15, .text = "000000000000000F" },
        .{ .input = "ffffffffffffffff", .want = std.math.maxInt(u64), .text = "FFFFFFFFFFFFFFFF" },
        .{ .input = "0000000000000000", .want = 0, .text = "0000000000000000" },
        .{ .input = "000000000000000", .want = null, .text = "000000000000000" },
        .{ .input = "00000000000000000", .want = null, .text = "00000000000000000" },
        .{ .input = "0x00000000000000", .want = null, .text = "0x00000000000000" },
        .{ .input = "-00000000000000F", .want = null, .text = "-00000000000000F" },
        .{ .input = "00000000000000_F", .want = null, .text = "00000000000000_F" },
        .{ .input = " 00000000000000F", .want = null, .text = " 00000000000000F" },
        .{ .input = "００００００００００００００００", .want = null, .text = "００００００００００００００００" },
    };
    var draft = try s.ScenarioDraft.factoryDefault(t.allocator);
    defer draft.deinit();
    var editor = try e.EditorState.init(t.allocator, &draft);
    defer editor.deinit();
    editor.section = .rules;
    try editor.focusField("scenario.seed");
    for (rows) |row| {
        const before = editor.current().seed;
        try editor.replaceFocusedText(row.input);
        try t.expectEqual(row.want orelse before, editor.current().seed);
        try t.expectEqualStrings(row.text, editor.fieldText("scenario.seed").?);
        try t.expectEqual(row.want == null, editor._current.errors[0]);
    }
    editor.section = .worlds;
    try editor.focusField("world.0.owner");
    for ([_][]const u8{ "NEUTRAL", "UNION", "HELIX", "CHOIR" }, [_]?@import("model.zig").Faction{ null, .union_faction, .helix, .choir }) |input, want| {
        try editor.replaceFocusedText(input);
        try t.expectEqual(want, editor.current().worlds[0].owner);
    }
    for ([_][]const u8{ "neutral", "Union", " CHOIR", "HELIX ", "" }) |input| {
        try editor.replaceFocusedText(input);
        try t.expectEqual(@as(?@import("model.zig").Faction, .choir), editor.current().worlds[0].owner);
        try t.expect(editor._current.errors[e.lookup("world.0.owner").?.id.slot()]);
    }
}
// Catches parse/semantic conflation, name truncation and arena-identity dirty tests.
test "Classic editor invalid text semantic names warnings dirty and repair" {
    var draft = try s.ScenarioDraft.factoryDefault(t.allocator);
    defer draft.deinit();
    var editor = try e.EditorState.init(t.allocator, &draft);
    defer editor.deinit();
    try t.expect(!editor.isDirty());
    try t.expect(editor.canApply());
    try editor.focusField("world.0.energy");
    try editor.replaceFocusedText("BROKEN");
    try t.expect(editor.isDirty());
    try t.expect(editor.hasParseErrors());
    var report = editor.validationReport();
    try t.expect(report.isValid());
    try t.expect(!editor.canApply());
    try t.expectError(error.InvalidText, editor.candidate());
    var original_energy: [32]u8 = undefined;
    try editor.replaceFocusedText(try std.fmt.bufPrint(&original_energy, "{d}", .{draft.worlds[0].energy}));
    try t.expect(!editor.isDirty());
    try editor.focusField("world.0.name");
    for ([_][]const u8{ "", "lowercase", "A\nB", "A🪐é", "THIS VALID UTF8 NAME IS MUCH LONGER THAN TWENTY FOUR BYTES" }) |name| {
        try editor.replaceFocusedText(name);
        try t.expectEqualStrings(name, editor.current().worlds[0].name);
        try t.expect(!editor.hasParseErrors());
        try t.expect(editor.isDirty());
        try t.expect(!editor.canApply());
        try t.expectError(error.Validation, editor.candidate());
    }
    try editor.replaceFocusedText(draft.worlds[0].name);
    try t.expect(!editor.isDirty());
    try editor.focusField("world.0.atmosphere");
    try editor.replaceFocusedText("255");
    try t.expect(!editor.hasParseErrors());
    try t.expect(!editor.canApply());
    try editor.focusField("world.0.name");
    try editor.replaceFocusedText(draft.worlds[1].name);
    try editor.focusField("world.0.atmosphere");
    var level: [4]u8 = undefined;
    try editor.replaceFocusedText(try std.fmt.bufPrint(&level, "{d}", .{draft.worlds[0].atmosphere}));
    report = editor.validationReport();
    try t.expect(report.isValid());
    try t.expect(report.warningCount() > 0);
    try t.expect(editor.canApply());
    _ = try editor.candidate();
}
// Catches byte-wise deletion, grapheme deletion, split UTF8 and ignored append.
test "Classic editor Unicode replacement insertion and scalar backspace" {
    var draft = try s.ScenarioDraft.factoryDefault(t.allocator);
    defer draft.deinit();
    var editor = try e.EditorState.init(t.allocator, &draft);
    defer editor.deinit();
    try editor.focusField("world.0.name");
    try editor.replaceFocusedText("A🪐e");
    try editor.insertFocusedText("́");
    try t.expectEqualStrings("A🪐é", editor.focusedText().?);
    for ([_][]const u8{ "A🪐e", "A🪐", "A", "", "" }) |want| {
        try editor.backspaceFocused();
        try t.expectEqualStrings(want, editor.focusedText().?);
        try t.expectEqualStrings(want, editor.current().worlds[0].name);
    }
    const bad = "\xff";
    try t.expectError(error.InvalidUtf8, editor.replaceFocusedText(bad));
    try t.expectError(error.InvalidUtf8, editor.insertFocusedText(bad));
    try t.expectError(error.InvalidUtf8, editor.setPreedit(bad));
    try t.expectError(error.InvalidUtf8, editor.commitIme(bad));
    try t.expectEqualStrings("", editor.current().worlds[0].name);
    editor.focusAction(.save);
    try t.expect(!editor.focusedAcceptsText());
    try editor.replaceFocusedText(bad);
    try editor.insertFocusedText(bad);
    try editor.backspaceFocused();
    try t.expectEqualStrings("", editor.current().worlds[0].name);
    editor.focus = null;
    try editor.replaceFocusedText(bad);
    try editor.insertFocusedText(bad);
    try editor.backspaceFocused();
}
// Catches IME leakage into typed content and invented preedit reset on selection.
test "Classic editor IME is visual and survives focus section world transitions" {
    var draft = try s.ScenarioDraft.factoryDefault(t.allocator);
    defer draft.deinit();
    var editor = try e.EditorState.init(t.allocator, &draft);
    defer editor.deinit();
    try editor.focusField("world.0.name");
    try editor.setPreedit("🪐́");
    try t.expectEqualStrings("🪐́", editor.preedit());
    try t.expect(!editor.isDirty());
    try t.expectEqualStrings(draft.worlds[0].name, editor.focusedText().?);
    editor.scroll = 15;
    editor.selectSection(.worlds);
    try t.expect(editor.focusedAcceptsText());
    try t.expectEqual(@as(f32, 15), editor.scroll);
    editor.selectSection(.rules);
    try t.expectEqual(@as(?e.Focus, null), editor.focus);
    try t.expectEqual(@as(f32, 0), editor.scroll);
    try t.expectEqualStrings("🪐́", editor.preedit());
    try editor.focusField("rule.maximum_energy");
    editor.selectWorld(0);
    try t.expectEqual(e.Section.worlds, editor.section);
    try t.expectEqual(@as(?e.Focus, null), editor.focus);
    try editor.focusField("world.0.name");
    editor.scroll = 22;
    editor.selectWorld(0);
    try t.expectEqual(@as(?e.Focus, null), editor.focus);
    try t.expectEqual(@as(f32, 0), editor.scroll);
    try editor.focusField("world.0.name");
    editor.scroll = 33;
    editor.selectWorld(7);
    try t.expect(editor.focusedAcceptsText());
    try t.expectEqual(@as(f32, 33), editor.scroll);
    try editor.replaceFocusedText("A");
    try editor.commitIme("B");
    try t.expectEqualStrings("AB", editor.current().worlds[0].name);
    try t.expectEqualStrings("", editor.preedit());
    try editor.setPreedit("未");
    editor.focus = null;
    try editor.commitIme("\xff");
    try t.expectEqualStrings("", editor.preedit());
    try t.expectEqualStrings("AB", editor.current().worlds[0].name);
    try editor.setPreedit("未");
    editor.focusAction(.save);
    try editor.commitIme("ignored");
    try t.expectEqualStrings("", editor.preedit());
    try t.expectEqualStrings("AB", editor.current().worlds[0].name);
}
// Catches second modal state, guarded edits consuming bytes and replace applying.
test "Classic editor every modal guards editing and replacement disposes both captures" {
    var draft = try s.ScenarioDraft.factoryDefault(t.allocator);
    defer draft.deinit();
    for (comptime std.meta.tags(e.ModalKind)) |kind| {
        var editor = try e.EditorState.init(t.allocator, &draft);
        defer editor.deinit();
        try editor.focusField("world.0.energy");
        try editor.replaceFocusedText("INVALID");
        try editor.setPreedit("視");
        editor.scroll = 37;
        const candidate = try draft.validated();
        editor.modal.begin(kind, try t.allocator.dupe(u8, "independent save bytes"), candidate);
        editor.selectSection(.rules);
        editor.selectWorld(4);
        try editor.insertFocusedText("\xff");
        try editor.backspaceFocused();
        try editor.setPreedit("\xff");
        try editor.commitIme("\xff");
        try t.expectError(error.UnknownWidget, editor.focusField("world.0.name"));
        try t.expectEqual(e.Section.worlds, editor.section);
        try t.expectEqual(@as(usize, 0), editor.selected_world);
        try t.expectEqual(@as(f32, 37), editor.scroll);
        try t.expectEqualStrings("INVALID", editor.focusedText().?);
        try t.expectEqualStrings("視", editor.preedit());
        try t.expectEqual(@as(?e.ModalKind, kind), editor.modal.kind);
        try editor.replaceFocusedText("\xff");
        try t.expectEqual(@as(?e.ModalKind, null), editor.modal.kind);
        try t.expect(editor.modal.save_bytes == null);
        try t.expect(editor.modal.apply_candidate == null);
        try t.expectEqualStrings("INVALID", editor.focusedText().?);
        try t.expectEqualStrings("視", editor.preedit());
        try t.expect(editor.hasParseErrors());
        editor.modal.dismiss(); // idempotent: test allocator detects double-free.
        editor.focus = null;
        editor.modal.begin(kind, try t.allocator.dupe(u8, "save without focus"), candidate);
        try editor.replaceFocusedText("\xff");
        try t.expectEqual(@as(?e.ModalKind, null), editor.modal.kind);
        try t.expectEqualStrings("INVALID", editor.fieldText("world.0.energy").?);
        try t.expectEqualStrings("視", editor.preedit());
    }
}
// Catches current replacement resetting ORIGINAL or bypassing invalid text repair.
test "Classic editor current replacement remains detached preserves focus preedit and original" {
    const app = @import("classic_app.zig");
    const candidate = try s.generatedValidated(@import("model.zig").DEFAULT_SEED, .{});
    var core = try app.AppCore.init(t.allocator, &candidate, .{ .onboarding_completed = true });
    defer core.deinit();
    _ = try core.enqueueGameCommand(.{ .launch = .{ .source = 0, .destination = 3 } });
    const fingerprint = core.canonicalFingerprint();
    const pending = try t.allocator.dupe(@import("model.zig").CommandEnvelope, core._simulation.pending.items);
    defer t.allocator.free(pending);
    var draft = try core.activeScenario().toDraft(t.allocator);
    defer draft.deinit();
    var editor = try e.EditorState.init(t.allocator, &draft);
    defer editor.deinit();
    try editor.focusField("world.0.energy");
    try editor.replaceFocusedText("INVALID");
    try editor.setPreedit("未");
    editor.scroll = 83;
    var replacement = try s.ScenarioDraft.generated(t.allocator, 77, .{});
    defer replacement.deinit();
    replacement.worlds[1].name = "arbitrary invalid 🪐";
    try editor.replaceCurrent(&replacement);
    try t.expectEqual(@as(u64, 77), editor.current().seed);
    try t.expectEqual(@as(u64, @import("model.zig").DEFAULT_SEED), editor.original().seed);
    try t.expect(!editor.hasParseErrors());
    try t.expectEqualStrings("未", editor.preedit());
    try t.expectEqual(@as(f32, 83), editor.scroll);
    try t.expectEqual(@as(usize, 17), editor.focus.?.field.slot());
    try t.expectEqualStrings("arbitrary invalid 🪐", editor.current().worlds[1].name);
    try t.expect(!editor.canApply());
    try t.expectError(error.Validation, editor.candidate());
    var original_copy = try editor.copyOriginal(t.allocator);
    defer original_copy.deinit();
    try editor.replaceCurrent(&original_copy);
    try t.expect(!editor.isDirty());
    _ = try editor.candidate();
    var current_copy = try editor.copyCurrent(t.allocator);
    defer current_copy.deinit();
    try t.expect(current_copy.worlds[0].name.ptr != editor.current().worlds[0].name.ptr);
    try t.expectEqual(fingerprint, core.canonicalFingerprint());
    try t.expectEqualDeep(pending, core._simulation.pending.items);
    try t.expectEqual(@as(u64, 1), core.nextSequence());
}
// Catches malformed detached names passing the consumed UTF8 boundary.
test "Classic editor rejects invalid UTF8 init and replacement without byte repair" {
    var draft = try s.ScenarioDraft.factoryDefault(t.allocator);
    defer draft.deinit();
    var editor = try e.EditorState.init(t.allocator, &draft);
    defer editor.deinit();
    const old = editor.current().seed;
    draft.worlds[4].name = "\xc0\xaf";
    try t.expectError(error.InvalidUtf8, e.EditorState.init(t.allocator, &draft));
    try t.expectError(error.InvalidUtf8, editor.replaceCurrent(&draft));
    try t.expectEqual(old, editor.current().seed);
    try t.expect(!editor.isDirty());
}
// Test-only deeply owned snapshots distinguish content from moving arena handles.
const Snapshot = struct {
    arena: std.heap.ArenaAllocator,
    original: e.DraftView,
    current: e.DraftView,
    buffers: [90][]const u8,
    buffer_ptrs: [90][*]const u8,
    flags: [90]bool,
    preedit: []const u8,
    preedit_ptr: [*]const u8,
    focus: ?e.Focus,
    section: e.Section,
    world: usize,
    scroll: f32,
    modal_kind: ?e.ModalKind,
    save: ?[]const u8,
    apply: ?s.ScenarioV1,
    save_ptr: ?[*]const u8,
    fn init(editor: *const e.EditorState) !Snapshot {
        var snap: Snapshot = .{ .arena = .init(t.allocator), .original = editor.original(), .current = editor.current(), .buffers = undefined, .buffer_ptrs = undefined, .flags = editor._current.errors, .preedit = undefined, .preedit_ptr = editor.preedit().ptr, .focus = editor.focus, .section = editor.section, .world = editor.selected_world, .scroll = editor.scroll, .modal_kind = editor.modal.kind, .save = null, .apply = editor.modal.apply_candidate, .save_ptr = if (editor.modal.save_bytes) |b| b.ptr else null };
        errdefer snap.arena.deinit();
        const a = snap.arena.allocator();
        for (&snap.original.worlds) |*w| w.name = try a.dupe(u8, w.name);
        for (&snap.current.worlds) |*w| w.name = try a.dupe(u8, w.name);
        for (editor._current.buffers, &snap.buffers, &snap.buffer_ptrs) |b, *copy, *ptr| {
            copy.* = try a.dupe(u8, b);
            ptr.* = b.ptr;
        }
        snap.preedit = try a.dupe(u8, editor.preedit());
        if (editor.modal.save_bytes) |b| snap.save = try a.dupe(u8, b);
        return snap;
    }
    fn deinit(self: *Snapshot) void {
        self.arena.deinit();
    }
    fn expectView(before: e.DraftView, after: e.DraftView) !void {
        try t.expectEqual(before.seed, after.seed);
        try t.expectEqualDeep(before.rules, after.rules);
        for (before.worlds, after.worlds) |x, y| {
            inline for (comptime std.meta.fieldNames(s.WorldDraft)) |f| {
                if (comptime std.mem.eql(u8, f, "name")) try t.expectEqualStrings(x.name, y.name) else try t.expectEqual(@field(x, f), @field(y, f));
            }
        }
    }
    fn expectPreserved(self: *const Snapshot, editor: *const e.EditorState) !void {
        try expectView(self.original, editor.original());
        try expectView(self.current, editor.current());
        for (self.buffers, editor._current.buffers, self.buffer_ptrs) |before, after, ptr| {
            try t.expectEqualStrings(before, after);
            try t.expectEqual(ptr, after.ptr);
        }
        try t.expectEqualDeep(self.flags, editor._current.errors);
        try t.expectEqualStrings(self.preedit, editor.preedit());
        try t.expectEqual(self.preedit_ptr, editor.preedit().ptr);
        try t.expectEqualDeep(self.focus, editor.focus);
        try t.expectEqual(self.section, editor.section);
        try t.expectEqual(self.world, editor.selected_world);
        try t.expectEqual(self.scroll, editor.scroll);
        try t.expectEqual(self.modal_kind, editor.modal.kind);
        try t.expectEqualDeep(self.apply, editor.modal.apply_candidate);
        if (self.save) |bytes| {
            try t.expectEqualStrings(bytes, editor.modal.save_bytes.?);
            try t.expectEqual(self.save_ptr.?, editor.modal.save_bytes.?.ptr);
        } else try t.expect(editor.modal.save_bytes == null);
    }
};
const Mutation = enum { replace_valid, replace_invalid, insert, backspace, preedit, commit, clear_commit, replace_current };
fn mutate(editor: *e.EditorState, op: Mutation, draft: *const s.ScenarioDraft, big: []const u8) !void {
    switch (op) {
        .replace_valid => try editor.replaceFocusedText(big),
        .replace_invalid => try editor.replaceFocusedText(big),
        .insert => try editor.insertFocusedText(big),
        .backspace => try editor.backspaceFocused(),
        .preedit => try editor.setPreedit(big),
        .commit => try editor.commitIme(big),
        .clear_commit => try editor.commitIme("\xff"),
        .replace_current => try editor.replaceCurrent(draft),
    }
}
fn expectMutated(editor: *const e.EditorState, op: Mutation, big: []const u8) !void {
    switch (op) {
        .replace_valid => try t.expectEqualStrings(big, editor.current().worlds[3].name),
        .replace_invalid => {
            try t.expectEqualStrings(big, editor.focusedText().?);
            try t.expect(editor._current.errors[e.lookup("world.3.energy").?.id.slot()]);
        },
        .insert, .commit => {
            const name = editor.current().worlds[3].name;
            try t.expectEqual(@as(usize, 32776), name.len);
            try t.expectEqualStrings("A🪐é", name[0..8]);
            try t.expectEqualStrings(big, name[8..]);
        },
        .backspace => try t.expectEqualStrings("A🪐e", editor.current().worlds[3].name),
        .preedit => try t.expectEqualStrings(big, editor.preedit()),
        .clear_commit => try t.expectEqualStrings("A🪐é", editor.current().worlds[3].name),
        .replace_current => {
            try t.expectEqual(@as(u64, 0x0123456789ABCDEF), editor.current().seed);
            try t.expect(!editor.hasParseErrors());
            try t.expectEqualStrings("owned save", editor.modal.save_bytes.?);
            try t.expectEqual(@as(?e.ModalKind, .load_dirty), editor.modal.kind);
            try t.expect(editor.modal.apply_candidate != null);
        },
    }
    if (op == .commit or op == .clear_commit) try t.expectEqualStrings("", editor.preedit());
}
// Catches partial publication at each allocation ordinal and stale arena pointers.
test "Classic editor exhaustive init allocation failures release partial drafts" {
    var draft = try fixture(t.allocator);
    defer draft.deinit();
    var count: usize = 0;
    var completed = false;
    for (0..512) |index| {
        var failing = t.FailingAllocator.init(t.allocator, .{ .fail_index = index, .resize_fail_index = 0 });
        if (e.EditorState.init(failing.allocator(), &draft)) |value| {
            var editor = value;
            editor.deinit();
            completed = true;
        } else |err| {
            try t.expectEqual(error.OutOfMemory, err);
            try t.expect(failing.has_induced_failure);
            count += 1;
            try t.expectEqual(failing.allocated_bytes, failing.freed_bytes);
            failing.fail_index = std.math.maxInt(usize);
            var retry = try e.EditorState.init(failing.allocator(), &draft);
            try t.expectEqualStrings("ASTER", retry.fieldText("world.0.name").?);
            retry.deinit();
        }
        try t.expectEqual(failing.allocated_bytes, failing.freed_bytes);
        try t.expectEqualStrings("ASTER", draft.worlds[0].name);
        try t.expectEqual(@as(u64, 0x0123456789ABCDEF), draft.seed);
        if (completed) break;
    }
    try t.expect(completed);
    try t.expect(count > 1);
    std.debug.print("init allocation failure ordinals: {d}\n", .{count});
}
// Catches every mutation's OOM corrupting names/text/preedit/focus or modal capture.
test "Classic editor exhaustive epoch mutation allocation failures preserve full state and retry" {
    var draft = try s.ScenarioDraft.factoryDefault(t.allocator);
    defer draft.deinit();
    var replacement = try fixture(t.allocator);
    defer replacement.deinit();
    const big = try t.allocator.alloc(u8, 32768);
    defer t.allocator.free(big);
    @memset(big, 'A');
    for (comptime std.meta.tags(Mutation)) |op| {
        var count: usize = 0;
        var completed = false;
        for (0..512) |index| {
            var failing = t.FailingAllocator.init(t.allocator, .{ .resize_fail_index = 0 });
            {
                var initial = try e.EditorState.init(failing.allocator(), &draft);
                // Move owner by value and invalidate its old location before editing.
                var editor = initial;
                initial = undefined;
                defer editor.deinit();
                try editor.focusField("world.0.energy");
                try editor.replaceFocusedText("INVALID");
                editor.selectWorld(3);
                try editor.focusField("world.3.name");
                try editor.replaceFocusedText("A🪐é");
                try editor.setPreedit("visual 未");
                editor.scroll = 41;
                if (op == .replace_invalid) try editor.focusField("world.3.energy");
                if (op == .clear_commit) editor.focus = null;
                if (op == .replace_current) editor.modal.begin(.load_dirty, try failing.allocator().dupe(u8, "owned save"), try draft.validated());
                var before = try Snapshot.init(&editor);
                defer before.deinit();
                failing.fail_index = failing.alloc_index + index;
                if (mutate(&editor, op, &replacement, big)) |_| {
                    completed = true;
                    try expectMutated(&editor, op, big);
                } else |err| {
                    try t.expectEqual(error.OutOfMemory, err);
                    try t.expect(failing.has_induced_failure);
                    count += 1;
                    try before.expectPreserved(&editor);
                    failing.fail_index = std.math.maxInt(usize);
                    try mutate(&editor, op, &replacement, big);
                    try expectMutated(&editor, op, big);
                }
            }
            try t.expectEqual(failing.allocated_bytes, failing.freed_bytes);
            if (completed) break;
        }
        try t.expect(completed);
        try t.expect(count > 0);
        std.debug.print("{s} allocation failure ordinals: {d}\n", .{ @tagName(op), count });
    }
}
// Catches retaining historical names, invalid buffers or preedit arena allocations.
test "Classic editor repeated long short edits retain only current content after moving" {
    var draft = try s.ScenarioDraft.factoryDefault(t.allocator);
    defer draft.deinit();
    var tracking = t.FailingAllocator.init(t.allocator, .{ .resize_fail_index = 0 });
    {
        var initial = try e.EditorState.init(tracking.allocator(), &draft);
        var editor = initial;
        initial = undefined;
        defer editor.deinit();
        const big = try t.allocator.alloc(u8, 65536);
        defer t.allocator.free(big);
        @memset(big, 'A');
        try editor.focusField("world.0.name");
        try editor.replaceFocusedText("SHORT");
        try editor.setPreedit("");
        const baseline = tracking.allocated_bytes - tracking.freed_bytes;
        for (0..100) |_| {
            try editor.focusField("world.0.name");
            try editor.replaceFocusedText(big);
            try editor.replaceFocusedText("SHORT");
            try editor.setPreedit(big);
            try editor.setPreedit("");
            try editor.focusField("world.0.energy");
            try editor.replaceFocusedText(big);
            try editor.replaceFocusedText("1");
            try t.expectEqual(baseline, tracking.allocated_bytes - tracking.freed_bytes);
        }
        try t.expectEqualStrings("SHORT", editor.current().worlds[0].name);
        try t.expectEqual(@as(u64, 1), editor.current().worlds[0].energy);
    }
    try t.expectEqual(tracking.allocated_bytes, tracking.freed_bytes);
}
// Catches UTF8 admission happening after allocation and inert guards allocating.
test "Classic editor consumed UTF8 validates before allocations and inert guards consume nothing" {
    var draft = try s.ScenarioDraft.factoryDefault(t.allocator);
    defer draft.deinit();
    var failing = t.FailingAllocator.init(t.allocator, .{ .fail_index = 0, .resize_fail_index = 0 });
    draft.worlds[0].name = "\xff";
    try t.expectError(error.InvalidUtf8, e.EditorState.init(failing.allocator(), &draft));
    try t.expectEqual(@as(usize, 0), failing.alloc_index);
    try t.expect(!failing.has_induced_failure);
    draft.worlds[0].name = "ASTER";
    failing.fail_index = std.math.maxInt(usize);
    var editor = try e.EditorState.init(failing.allocator(), &draft);
    defer editor.deinit();
    try editor.focusField("world.0.name");
    try editor.setPreedit("未");
    const epoch_ptr = editor.fieldText("world.0.name").?.ptr;
    const allocations = failing.alloc_index;
    failing.fail_index = allocations;
    try t.expectError(error.InvalidUtf8, editor.replaceFocusedText("\xff"));
    try t.expectError(error.InvalidUtf8, editor.insertFocusedText("\xff"));
    try t.expectError(error.InvalidUtf8, editor.setPreedit("\xff"));
    try t.expectError(error.InvalidUtf8, editor.commitIme("\xff"));
    editor.selectSection(.rules);
    try editor.focusField("rule.maximum_energy");
    editor.selectWorld(2);
    try editor.focusField("world.2.name");
    try t.expectEqual(epoch_ptr, editor.fieldText("world.0.name").?.ptr);
    try t.expectEqualStrings("未", editor.preedit());
    editor.focusAction(.save);
    try editor.insertFocusedText("\xff");
    try editor.replaceFocusedText("\xff");
    try editor.backspaceFocused();
    editor.modal.begin(.cancel_dirty, null, null);
    editor.selectSection(.validation);
    editor.selectWorld(5);
    editor.focusAction(.cancel);
    try editor.insertFocusedText("\xff");
    try editor.setPreedit("\xff");
    try editor.commitIme("\xff");
    try editor.backspaceFocused();
    try editor.replaceFocusedText("\xff");
    try t.expectEqual(allocations, failing.alloc_index);
    try t.expect(!failing.has_induced_failure);
    try t.expectEqual(e.Section.worlds, editor.section);
    try t.expectEqual(@as(usize, 2), editor.selected_world);
    try t.expectEqualStrings("未", editor.preedit());
    try t.expectEqualDeep(@as(?e.Focus, .{ .action = .save }), editor.focus);
}
// Catches candidate extraction allocating or admitting invalid text after semantics.
test "Classic editor candidate extraction is allocation free and invalid text wins semantic errors" {
    var draft = try s.ScenarioDraft.factoryDefault(t.allocator);
    defer draft.deinit();
    var failing = t.FailingAllocator.init(t.allocator, .{});
    var editor = try e.EditorState.init(failing.allocator(), &draft);
    defer editor.deinit();
    try editor.focusField("world.0.name");
    try editor.replaceFocusedText(draft.worlds[1].name);
    const n = failing.alloc_index;
    failing.fail_index = n;
    const value = try editor.candidate();
    try t.expect(value.warnings.warningCount() > 0);
    try t.expectEqual(n, failing.alloc_index);
    failing.fail_index = std.math.maxInt(usize);
    try editor.replaceFocusedText("arbitrary invalid 🪐");
    try editor.focusField("world.0.energy");
    try editor.replaceFocusedText("BAD");
    failing.fail_index = failing.alloc_index;
    try t.expectError(error.InvalidText, editor.candidate());
    const report = editor.validationReport();
    try t.expect(!report.isValid());
}
// Catches borrowed input aliases and capture storage owned by a moving epoch.
test "Classic editor moving owner after releasing borrowed draft retains independent captures" {
    var input = try s.ScenarioDraft.factoryDefault(t.allocator);
    var initial = try e.EditorState.init(t.allocator, &input);
    const captured = try input.validated();
    input.deinit();
    var editor = initial;
    initial = undefined;
    defer editor.deinit();
    try editor.focusField("world.0.name");
    try editor.replaceFocusedText("DETACHED");
    try editor.setPreedit("視");
    editor.modal.begin(.overwrite_save, try t.allocator.dupe(u8, "saved independently"), captured);
    var draft = try fixture(t.allocator);
    defer draft.deinit();
    try editor.replaceCurrent(&draft);
    try t.expectEqualStrings("saved independently", editor.modal.save_bytes.?);
    try t.expectEqualDeep(captured, editor.modal.apply_candidate.?);
    try t.expectEqual(@as(?e.ModalKind, .overwrite_save), editor.modal.kind);
    try t.expectEqualStrings("視", editor.preedit());
    try t.expectEqual(@as(usize, 13), editor.focus.?.field.slot());
    try editor.replaceFocusedText("discarded");
    try t.expectEqualStrings("ASTER", editor.current().worlds[0].name);
    try t.expectEqualStrings("視", editor.preedit());
    try t.expect(editor.modal.save_bytes == null);
    try t.expect(editor.modal.apply_candidate == null);
    try editor.replaceFocusedText("AFTER MOVE");
    try t.expectEqualStrings("AFTER MOVE", editor.current().worlds[0].name);
}

// Independent per-field flags, including malformed typed world indices.
test "H3b parse getter reports exact field and rejects index seven" {
    var draft = try s.ScenarioDraft.factoryDefault(t.allocator);
    defer draft.deinit();
    var editor = try e.EditorState.init(t.allocator, &draft);
    defer editor.deinit();
    try editor.focusField("world.0.x");
    try editor.replaceFocusedText("broken");
    try t.expect(try editor.fieldHasParseError(.{ .world = .{ .index = 0, .field = .x } }));
    try t.expect(!(try editor.fieldHasParseError(.{ .world = .{ .index = 0, .field = .y } })));
    try t.expectError(error.InvalidField, editor.fieldHasParseError(.{ .world = .{ .index = 7, .field = .x } }));
}
test "H3b parse getter readonly epoch flags survive real allocated operation failure" {
    var failing = t.FailingAllocator.init(t.allocator, .{});
    var draft = try s.ScenarioDraft.factoryDefault(t.allocator);
    defer draft.deinit();
    var editor = try e.EditorState.init(failing.allocator(), &draft);
    defer editor.deinit();
    try editor.focusField("world.0.x");
    try editor.replaceFocusedText("invalid x");
    const before_requests = failing.alloc_index;
    const before_text = editor.fieldText("world.0.x").?;
    const before_name = editor.current().worlds[0].name;
    const before_focus = editor.focus;
    const before_scroll = editor.scroll;
    const before_section = editor.section;
    for (e.catalog) |field| try t.expect((try editor.fieldHasParseError(field.id)) == (field.id.slot() == 14));
    try t.expectEqual(before_requests, failing.alloc_index);
    failing.fail_index = failing.alloc_index;
    try t.expectError(error.OutOfMemory, editor.replaceFocusedText("123"));
    try t.expect(try editor.fieldHasParseError(.{ .world = .{ .index = 0, .field = .x } }));
    try t.expectEqualStrings("invalid x", editor.fieldText("world.0.x").?);
    try t.expect(before_text.ptr == editor.fieldText("world.0.x").?.ptr and before_name.ptr == editor.current().worlds[0].name.ptr);
    try t.expect(std.meta.eql(before_focus, editor.focus) and before_scroll == editor.scroll and before_section == editor.section);
}
