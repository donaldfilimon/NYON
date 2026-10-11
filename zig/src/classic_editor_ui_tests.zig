const std = @import("std");
const t = std.testing;
const ui = @import("classic_editor_ui.zig");
test "H3b UI canonical 109 identities include all real fields and actions" {
    try t.expectEqual(@as(usize, 109), ui.catalog.len);
}
const e = @import("classic_editor.zig");
const sc = @import("scenario.zig");
const l = @import("classic_editor_layout.zig");
const H1 = @import("retained_semantics.zig");
const tx = @import("classic_editor_transactions.zig");
test "H3b UI compact Rules full literal node population" {
    var draft = try sc.ScenarioDraft.factoryDefault(t.allocator);
    defer draft.deinit();
    var editor = try e.EditorState.init(t.allocator, &draft);
    defer editor.deinit();
    editor.section = .rules;
    var report = editor.validationReport();
    var flags: l.ParseFlags = @splat(false);
    var w: l.Workspace = .{};
    const status: tx.Status = .none;
    var tokens = H1.LiveTokenBudget.init(.{ .max_live_objects = 2, .max_live_requested_bytes = 4096 });
    defer tokens.deinit();
    const resident = try H1.TokenRef.create(t.allocator, &tokens);
    defer resident.release();
    const token_draft = try H1.TokenRef.create(t.allocator, &tokens);
    defer token_draft.release();
    var budget = H1.PreparationBudget.init(1179648);
    defer budget.deinit();
    var prepared = try ui.prepareEditor(t.allocator, &budget, &w, .{ .editor = &editor, .report = &report, .parse_flags = &flags, .viewport = .{ 960, 600 }, .store_status = &status, .action_failure = null, .focus = .{ .current = null } }, .{ .revision = 1, .owner = .{ .resident = resident, .draft = token_draft, .hold = null, .context = "rules" } });
    defer prepared.deinit();
    try t.expectEqual(@as(usize, 78), (try prepared.input()).nodes.len);
}
const Fixture = struct {
    editor: e.EditorState = undefined,
    report: sc.ValidationReport = .{},
    flags: l.ParseFlags = @splat(false),
    workspace: l.Workspace = .{},
    tokens: H1.LiveTokenBudget = H1.LiveTokenBudget.init(.{ .max_live_objects = 2, .max_live_requested_bytes = 4096 }),
    resident: H1.TokenRef = undefined,
    draft: H1.TokenRef = undefined,
    budget: H1.PreparationBudget = H1.PreparationBudget.init(1179648),
    status: tx.Status = .none,
    editor_live: bool = false,
    fn init(self: *Fixture) !void {
        var draft = try sc.ScenarioDraft.factoryDefault(t.allocator);
        defer draft.deinit();
        self.editor = try e.EditorState.init(t.allocator, &draft);
        self.editor_live = true;
        self.resident = try H1.TokenRef.create(t.allocator, &self.tokens);
        self.draft = try H1.TokenRef.create(t.allocator, &self.tokens);
        self.refresh();
    }
    fn refresh(self: *Fixture) void {
        self.report = self.editor.validationReport();
        for (e.catalog) |field| self.flags[field.id.slot()] = self.editor.fieldHasParseError(field.id) catch unreachable;
    }
    fn deinit(self: *Fixture) void {
        if (self.editor_live) self.editor.deinit();
        self.draft.release();
        self.resident.release();
        self.budget.deinit();
        self.tokens.deinit();
    }
    fn stamp(self: *const Fixture, revision: u64) ui.CurrentStamp {
        return .{ .revision = revision, .owner = .{ .resident = self.resident, .draft = self.draft, .hold = 7, .context = "editor" } };
    }
    fn source(self: *Fixture, viewport: [2]f32) ui.Source {
        const focus: ?l.FocusKey = if (self.editor.focus) |f| switch (f) {
            .field => |id| .{ .field = id },
            .action => |a| .{ .action = a },
        } else null;
        return .{ .editor = &self.editor, .report = &self.report, .parse_flags = &self.flags, .viewport = viewport, .store_status = &self.status, .action_failure = null, .focus = .{ .current = focus } };
    }
};
test "H3b UI focus exact order current remembered absence and readonly identity" {
    var f: Fixture = .{};
    try f.init();
    defer f.deinit();
    f.editor.section = .rules;
    try f.editor.focusField("scenario.seed");
    var p = try ui.prepareEditor(t.allocator, &f.budget, &f.workspace, f.source(.{ 960, 600 }), f.stamp(1));
    defer p.deinit();
    const input = try p.input();
    try t.expectEqual(@as(usize, 22), input.focus.base.len);
    try t.expectEqualStrings("a/editor/scenario.seed", input.focus.focused.?.bytes);
}
test "H3b UI publication authorizes only actual enabled focusable capabilities" {
    var f: Fixture = .{};
    try f.init();
    defer f.deinit();
    var p = try ui.prepareEditor(t.allocator, &f.budget, &f.workspace, f.source(.{ 0, 0 }), f.stamp(1));
    defer p.deinit();
    var slot = H1.FrameSlot.init(t.allocator, &f.budget);
    defer slot.deinit();
    var revision: ?u64 = null;
    try ui.publishEditor(&slot, &revision, &p, f.stamp(1));
    try t.expectError(error.UnknownAction, ui.validateEditorTarget(slot.view().?, revision.?, f.stamp(1), null, .{ .bytes = "a/editor/section.rules" }));
}
fn node(input: H1.FrameInput, id: []const u8) H1.NodeSpec {
    for (input.nodes) |n| if (std.mem.eql(u8, n.id.bytes, id)) return n;
    @panic("missing literal node");
}
fn textBytes(input: H1.FrameInput) usize {
    var n: usize = 0;
    for (input.nodes) |v| {
        n += v.name.len + v.description.len;
        if (v.value) |value| n += value.len;
    }
    return n;
}
fn put(report: *sc.ValidationReport, code: sc.ScenarioIssueCode, severity: sc.IssueSeverity, path: []const u8, message: []const u8) void {
    const issue = &report.entries[report.len];
    issue.* = .{ .code = code, .severity = severity, .field = .{}, .message = .{} };
    issue.field.len = path.len;
    @memcpy(issue.field.bytes[0..path.len], path);
    issue.message.len = message.len;
    @memcpy(issue.message.bytes[0..message.len], message);
    report.len += 1;
}
const base_rules = [_]l.FocusKey{
    .{ .action = .{ .select_section = .worlds } },   .{ .action = .{ .select_section = .rules } },      .{ .action = .{ .select_section = .validation } },
    .{ .field = .{ .global = .seed } },              .{ .field = .{ .global = .win_world_count } },     .{ .field = .{ .global = .ai_period_ticks } },
    .{ .field = .{ .global = .hazard_first_tick } }, .{ .field = .{ .global = .hazard_period_ticks } }, .{ .field = .{ .global = .hazard_duration_ticks } },
    .{ .field = .{ .global = .maximum_energy } },    .{ .field = .{ .global = .maximum_defense } },     .{ .field = .{ .global = .minimum_launch } },
    .{ .field = .{ .global = .field_raise_cost } },  .{ .field = .{ .global = .field_lower_refund } },  .{ .field = .{ .global = .base_fleet_speed } },
    .{ .action = .revert },                          .{ .action = .factory_defaults },                  .{ .action = .regenerate_from_seed },
    .{ .action = .load },                            .{ .action = .save },                              .{ .action = .cancel },
    .{ .action = .apply_and_restart },
};
test "H3b UI literal complete compact Rules DFS order roles and child values" {
    var f: Fixture = .{};
    try f.init();
    defer f.deinit();
    f.editor.section = .rules;
    var p = try ui.prepareEditor(t.allocator, &f.budget, &f.workspace, f.source(.{ 960, 600 }), f.stamp(2));
    defer p.deinit();
    const input = try p.input();
    const prefix = [_][]const u8{ "c/editor/root", "c/editor/dialog", "c/editor/status_region", "c/editor/status.header/primary", "c/editor/status.header/label", "c/editor/status.header/value", "c/editor/navigation", "c/editor/section.worlds/primary", "c/editor/section.worlds/label", "c/editor/section.worlds/value", "c/editor/section.rules/primary", "c/editor/section.rules/label", "c/editor/section.rules/value", "c/editor/section.validation/primary", "c/editor/section.validation/label", "c/editor/section.validation/value", "c/editor/content", "c/editor/scenario.seed/primary", "c/editor/scenario.seed/label", "c/editor/scenario.seed/value" };
    for (prefix, 0..) |id, i| try t.expectEqualStrings(id, input.nodes[i].id.bytes);
    inline for (.{ "scenario.seed", "rule.tick_hz", "rule.win_world_count", "rule.ai_period_ticks", "rule.hazard_first_tick", "rule.hazard_period_ticks", "rule.hazard_duration_ticks", "rule.maximum_energy", "rule.maximum_defense", "rule.minimum_launch", "rule.field_raise_cost", "rule.field_lower_refund", "rule.base_fleet_speed" }, 0..) |key, i| {
        const primary = 17 + 3 * i;
        inline for (.{ "primary", "label", "value" }, 0..) |suffix, j| try t.expectEqualStrings("c/editor/" ++ key ++ "/" ++ suffix, input.nodes[primary + j].id.bytes);
        try t.expectEqual(@as(?usize, 16), input.nodes[primary].parent);
        try t.expectEqual(if (i == 1) H1.NodeRole.text else H1.NodeRole.spin_button, input.nodes[primary].role);
        for (input.nodes[primary + 1 ..][0..2]) |child| {
            try t.expectEqual(@as(?usize, primary), child.parent);
            try t.expectEqual(H1.NodeRole.text, child.role);
        }
    }
    try t.expectEqualStrings("c/editor/footer_region", input.nodes[56].id.bytes);
    try t.expectEqual(H1.NodeRole.toolbar, input.nodes[56].role);
    inline for (.{ "revert", "factory_defaults", "regenerate_from_seed", "load", "save", "cancel", "apply_and_restart" }, 0..) |key, i| {
        const primary = 57 + 3 * i;
        inline for (.{ "primary", "label", "value" }, 0..) |suffix, j| try t.expectEqualStrings("c/editor/footer." ++ key ++ "/" ++ suffix, input.nodes[primary + j].id.bytes);
        try t.expectEqual(@as(?usize, 56), input.nodes[primary].parent);
        try t.expectEqual(H1.NodeRole.button, input.nodes[primary].role);
        for (input.nodes[primary + 1 ..][0..2]) |child| {
            try t.expectEqual(@as(?usize, primary), child.parent);
            try t.expectEqual(H1.NodeRole.text, child.role);
        }
    }
    try t.expectEqualStrings("RULES | 0 ERRORS | 0 WARNINGS", node(input, "c/editor/status.header/value").value.?);
    const readonly = node(input, "c/editor/rule.tick_hz/primary");
    try t.expectEqual(H1.NodeRole.text, readonly.role);
    try t.expect(readonly.action == null and !readonly.focusable);
    try t.expectEqualStrings("60", readonly.value.?);
    try t.expectEqualStrings("TICK RATE", readonly.name);
    var slot = H1.FrameSlot.init(t.allocator, &f.budget);
    defer slot.deinit();
    var revision: ?u64 = null;
    try ui.publishEditor(&slot, &revision, &p, f.stamp(2));
    try t.expectEqual(@as(usize, input.nodes.len - 1), input.child_indices.len);
    for (slot.view().?.nodes, 0..) |n, i| {
        try t.expectEqualStrings(input.nodes[i].id.bytes, n.id.bytes);
        for (input.child_indices[n.child_start..][0..n.child_count]) |c| try t.expectEqual(@as(?usize, i), input.nodes[c].parent);
    }
}
test "H3b UI all seven modal reasons preserve saved backing restore outside base and IME" {
    const expected = [_][]const u8{ "DISCARD DRAFT CHANGES AND REVERT?", "REPLACE THE DRAFT WITH FACTORY DEFAULTS?", "REPLACE ALL WORLD EDITS FROM THE SEED?", "DISCARD DRAFT CHANGES AND LOAD THE SAVED SLOT?", "REPLACE THE OCCUPIED SAVE SLOT?", "DISCARD DRAFT CHANGES AND CLOSE THE EDITOR?", "DISCARD THE RUNNING SESSION AND START THIS SCENARIO?" };
    var f: Fixture = .{};
    try f.init();
    defer f.deinit();
    f.editor.section = .rules;
    try f.editor.focusField("scenario.seed");
    try f.editor.setPreedit("é語");
    for (std.meta.tags(e.ModalKind), 0..) |kind, index| {
        f.editor.modal.begin(kind, null, null);
        var saved = base_rules;
        var source = f.source(.{ 960, 600 });
        source.focus = .{ .current = .{ .action = .confirm }, .saved_base = .{ .order = &saved, .restore = .{ .field = .{ .world = .{ .index = 6, .field = .name } } } } };
        var p = try ui.prepareEditor(t.allocator, &f.budget, &f.workspace, source, f.stamp(index + 10));
        defer p.deinit();
        saved[0] = .{ .action = .cancel };
        const input = try p.input();
        try t.expectEqualStrings("a/editor/section.worlds", input.focus.base[0].bytes);
        try t.expectEqual(@as(usize, 22), input.focus.base.len);
        try t.expectEqual(@as(usize, 2), input.focus.modal.?.len);
        try t.expectEqualStrings("a/editor/world.6.name", input.focus.restore.?.bytes);
        try t.expectEqualStrings(expected[index], node(input, "c/editor/confirmation.message/primary").value.?);
        try t.expect(std.mem.endsWith(u8, node(input, "c/editor/scenario.seed/primary").value.?, "é語"));
        var actions: usize = 0;
        var overlay = false;
        for (input.nodes) |n| {
            if (std.mem.eql(u8, n.id.bytes, "c/editor/confirmation_dialog")) overlay = true;
            if (!overlay) try t.expect(!n.enabled and !n.focusable and n.action == null);
            if (n.action != null) actions += 1;
        }
        try t.expectEqual(@as(usize, 2), actions);
        var slot = H1.FrameSlot.init(t.allocator, &f.budget);
        defer slot.deinit();
        var revision: ?u64 = null;
        try ui.publishEditor(&slot, &revision, &p, f.stamp(index + 10));
        try t.expectEqualStrings("a/editor/world.6.name", slot.view().?.focus.restore.?.bytes);
        try t.expectError(error.InactiveModal, ui.validateEditorTarget(slot.view().?, revision.?, f.stamp(index + 10), kind, .{ .bytes = "a/editor/scenario.seed" }));
        try t.expectEqualDeep(ui.Target{ .action = .confirm }, try ui.validateEditorTarget(slot.view().?, revision.?, f.stamp(index + 10), kind, .{ .bytes = "a/editor/confirmation.confirm" }));
        f.editor.modal.dismiss();
    }
}
test "H3b UI remembered absence source focus mismatch orphan IME invalid keys modal lease" {
    var f: Fixture = .{};
    try f.init();
    defer f.deinit();
    f.editor.section = .rules;
    f.editor.focus = .{ .field = .{ .global = .tick_hz } };
    try f.editor.setPreedit("suffix");
    var source = f.source(.{ 0, 0 });
    var p = try ui.prepareEditor(t.allocator, &f.budget, &f.workspace, source, f.stamp(1));
    defer p.deinit();
    const input = try p.input();
    try t.expect(input.focus.focused == null);
    try t.expectEqualStrings("60suffix", node(input, "c/editor/rule.tick_hz/primary").value.?);
    try t.expectEqualDeep(@as(?l.FocusKey, .{ .field = .{ .global = .tick_hz } }), (try p.geometry()).remembered_focus);
    source.focus.current = null;
    try t.expectError(error.InvalidSource, ui.prepareEditor(t.allocator, &f.budget, &f.workspace, source, f.stamp(1)));
    f.editor.focus = .{ .field = .{ .world = .{ .index = 7, .field = .name } } };
    source = f.source(.{ 960, 600 });
    try t.expectError(error.InvalidFocus, ui.prepareEditor(t.allocator, &f.budget, &f.workspace, source, f.stamp(1)));
    f.editor.focus = null;
    source = f.source(.{ 960, 600 });
    var orphan = try ui.prepareEditor(t.allocator, &f.budget, &f.workspace, source, f.stamp(2));
    defer orphan.deinit();
    try t.expectEqualStrings("suffix", node(try orphan.input(), "c/editor/preedit").value.?);
    f.editor.modal.begin(.revert, null, null);
    source = f.source(.{ 960, 600 });
    try t.expectError(error.InvalidSource, ui.prepareEditor(t.allocator, &f.budget, &f.workspace, source, f.stamp(2)));
}
test "H3b UI tiny publication then resize restores actions and disabled Apply remains typed" {
    var f: Fixture = .{};
    try f.init();
    defer f.deinit();
    var slot = H1.FrameSlot.init(t.allocator, &f.budget);
    defer slot.deinit();
    var revision: ?u64 = null;
    var p = try ui.prepareEditor(t.allocator, &f.budget, &f.workspace, f.source(.{ 0, 0 }), f.stamp(1));
    try ui.publishEditor(&slot, &revision, &p, f.stamp(1));
    p.deinit();
    try t.expectError(error.UnknownAction, ui.validateEditorTarget(slot.view().?, 1, f.stamp(1), null, .{ .bytes = "a/editor/world.select.0" }));
    p = try ui.prepareEditor(t.allocator, &f.budget, &f.workspace, f.source(.{ 1440, 900 }), f.stamp(2));
    try ui.publishEditor(&slot, &revision, &p, f.stamp(2));
    p.deinit();
    try t.expectEqualDeep(ui.Target{ .action = .{ .select_world = 0 } }, try ui.validateEditorTarget(slot.view().?, 2, f.stamp(2), null, .{ .bytes = "a/editor/world.select.0" }));
    try f.editor.focusField("world.0.energy");
    try f.editor.replaceFocusedText("invalid");
    f.refresh();
    p = try ui.prepareEditor(t.allocator, &f.budget, &f.workspace, f.source(.{ 1440, 900 }), f.stamp(3));
    defer p.deinit();
    const input = try p.input();
    const apply = node(input, "c/editor/footer.apply_and_restart/primary");
    try t.expect(apply.focusable and apply.action != null and !apply.enabled);
    for (input.focus.base) |id| try t.expect(!std.mem.eql(u8, id.bytes, "a/editor/footer.apply_and_restart"));
    try ui.publishEditor(&slot, &revision, &p, f.stamp(3));
    try t.expectError(error.DisabledAction, ui.validateEditorTarget(slot.view().?, 3, f.stamp(3), null, .{ .bytes = "a/editor/footer.apply_and_restart" }));
    try t.expectError(error.StaleProjection, ui.validateEditorTarget(slot.view().?, 3, f.stamp(4), null, .{ .bytes = "a/editor/footer.cancel" }));
}
const IssueFixture = struct { code: sc.ScenarioIssueCode, severity: sc.IssueSeverity, path: []const u8, label: []const u8 };
const issue_fixtures = [_]IssueFixture{
    .{ .code = .tick_rate, .severity = .err, .path = "rules.tick_hz", .label = "TICKRATE" },                                   .{ .code = .win_world_count, .severity = .err, .path = "rules.win_world_count", .label = "WINWORLDCOUNT" }, .{ .code = .ai_period, .severity = .err, .path = "rules.ai_period_ticks", .label = "AIPERIOD" },            .{ .code = .hazard_period, .severity = .err, .path = "rules.hazard_period_ticks", .label = "HAZARDPERIOD" }, .{ .code = .hazard_duration, .severity = .err, .path = "rules.hazard_duration_ticks", .label = "HAZARDDURATION" }, .{ .code = .hazard_boundary_arithmetic, .severity = .err, .path = "rules.hazard_first_tick", .label = "HAZARDBOUNDARYARITHMETIC" }, .{ .code = .energy_cap, .severity = .err, .path = "rules.maximum_energy", .label = "ENERGYCAP" }, .{ .code = .defense_cap, .severity = .err, .path = "rules.maximum_defense", .label = "DEFENSECAP" }, .{ .code = .minimum_launch, .severity = .err, .path = "rules.minimum_launch", .label = "MINIMUMLAUNCH" },                .{ .code = .field_cost_relation, .severity = .err, .path = "rules.field_raise_cost", .label = "FIELDCOSTRELATION" },
    .{ .code = .world_name, .severity = .err, .path = "worlds[6].name", .label = "WORLDNAME" },                                .{ .code = .world_coordinate, .severity = .err, .path = "worlds[6].position", .label = "WORLDCOORDINATE" }, .{ .code = .world_field_level, .severity = .err, .path = "worlds[6].fields", .label = "WORLDFIELDLEVEL" },  .{ .code = .world_energy, .severity = .err, .path = "worlds[6].energy", .label = "WORLDENERGY" },            .{ .code = .world_defense, .severity = .err, .path = "worlds[6].defense", .label = "WORLDDEFENSE" },               .{ .code = .duplicate_position, .severity = .err, .path = "worlds[6].position", .label = "DUPLICATEPOSITION" },                     .{ .code = .missing_union, .severity = .err, .path = "worlds", .label = "MISSINGUNION" },         .{ .code = .initial_victory, .severity = .err, .path = "worlds", .label = "INITIALVICTORY" },        .{ .code = .production_arithmetic, .severity = .err, .path = "worlds[6].base_output", .label = "PRODUCTIONARITHMETIC" }, .{ .code = .regeneration_arithmetic, .severity = .err, .path = "worlds[6].base_regeneration", .label = "REGENERATIONARITHMETIC" },
    .{ .code = .fleet_speed_arithmetic, .severity = .err, .path = "rules.base_fleet_speed", .label = "FLEETSPEEDARITHMETIC" }, .{ .code = .duplicate_name, .severity = .warning, .path = "worlds[6].name", .label = "DUPLICATENAME" },     .{ .code = .close_position, .severity = .warning, .path = "worlds[6].position", .label = "CLOSEPOSITION" }, .{ .code = .missing_helix, .severity = .warning, .path = "worlds", .label = "MISSINGHELIX" },                .{ .code = .missing_choir, .severity = .warning, .path = "worlds", .label = "MISSINGCHOIR" },                      .{ .code = .no_launchable_union, .severity = .warning, .path = "worlds", .label = "NOLAUNCHABLEUNION" },
};
test "H3b UI all 26 validation families literal quartet two plus three copies" {
    var f: Fixture = .{};
    try f.init();
    defer f.deinit();
    f.editor.section = .validation;
    for (issue_fixtures) |fixture| {
        f.report = .{};
        put(&f.report, fixture.code, fixture.severity, fixture.path, "raw é message");
        var p = try ui.prepareEditor(t.allocator, &f.budget, &f.workspace, f.source(.{ 1440, 900 }), f.stamp(1));
        defer p.deinit();
        const input = try p.input();
        const primary = node(input, "c/editor/validation.0/primary");
        const code = node(input, "c/editor/validation.0/code");
        const message = node(input, "c/editor/validation.0/message");
        const field = node(input, "c/editor/validation.0/field");
        const severity = if (fixture.severity == .err) "ERROR | " else "WARNING | ";
        try t.expectEqual(H1.NodeRole.list_item, primary.role);
        try t.expectEqualStrings(fixture.label, primary.name);
        try t.expectEqualStrings(fixture.label, code.name);
        try t.expectEqualStrings("RAW é MESSAGE", message.name);
        try t.expectEqualStrings(fixture.path, primary.description);
        try t.expectEqualStrings(fixture.path, field.name);
        try t.expectEqualStrings(severity, code.value.?);
        try t.expectEqualStrings(severity, message.value.?);
        try t.expectEqualStrings(severity, primary.value.?[0..severity.len]);
        try t.expect(message.name.ptr == primary.value.?.ptr + severity.len and field.name.ptr == primary.description.ptr);
        const expected = 2 * fixture.label.len + 2 * fixture.path.len + 2 * "RAW é MESSAGE".len + 3 * severity.len;
        const actual = primary.name.len + primary.description.len + primary.value.?.len + code.name.len + code.value.?.len + message.name.len + message.value.?.len + field.name.len;
        try t.expectEqual(expected, actual);
    }
}
test "H3b UI crafted 98 report full 455 node DFS profile and physical schema rejection" {
    var f: Fixture = .{};
    try f.init();
    defer f.deinit();
    f.editor.section = .validation;
    try f.editor.setPreedit("p");
    f.report = .{};
    for (0..98) |_| put(&f.report, .tick_rate, .err, "rules.tick_hz", "tick rate must remain 60");
    f.editor.modal.begin(.revert, null, null);
    f.status = .saved;
    const failure: tx.Failure = .invalid_text;
    var source = f.source(.{ 1440, 900 });
    source.action_failure = &failure;
    source.recovery = .{ .resource = .{ .editor = error.OutOfMemory } };
    source.focus = .{ .current = null, .saved_base = .{ .order = &base_rules, .restore = .{ .field = .{ .global = .seed } } } };
    var p = try ui.prepareEditor(t.allocator, &f.budget, &f.workspace, source, f.stamp(1));
    defer p.deinit();
    const input = try p.input();
    try t.expectEqual(@as(usize, 455), input.nodes.len);
    try t.expectEqual(@as(usize, 454), input.child_indices.len);
    var slot = H1.FrameSlot.init(t.allocator, &f.budget);
    defer slot.deinit();
    var revision: ?u64 = null;
    try ui.publishEditor(&slot, &revision, &p, f.stamp(1));
    f.report.len = 99;
    try t.expectError(error.InvalidReport, ui.prepareEditor(t.allocator, &f.budget, &f.workspace, source, f.stamp(1)));
    f.report.len = 98;
    f.report.entries[0].message.len = 97;
    try t.expectError(error.InvalidReport, ui.prepareEditor(t.allocator, &f.budget, &f.workspace, source, f.stamp(1)));
    f.report.entries[0].message.len = 74;
    try t.expectError(error.InvalidReport, ui.prepareEditor(t.allocator, &f.budget, &f.workspace, source, f.stamp(1)));
    f.report.entries[0].message.len = 23;
    f.report.entries[0].field.len = 28;
    try t.expectError(error.InvalidReport, ui.prepareEditor(t.allocator, &f.budget, &f.workspace, source, f.stamp(1)));
    // This reaches the conservative count using a crafted report; no validator attainability claim.
}
test "H3b UI full invalid numeric UTF8 preedit world names no wire truncation stable after source mutation" {
    var f: Fixture = .{};
    try f.init();
    defer f.deinit();
    try f.editor.focusField("world.0.energy");
    const raw = try t.allocator.alloc(u8, 70002);
    defer t.allocator.free(raw);
    @memset(raw, 'z');
    raw[70000] = 0xc3;
    raw[70001] = 0xa9;
    try f.editor.replaceFocusedText(raw);
    try f.editor.setPreedit("語");
    f.refresh();
    var p = try ui.prepareEditor(t.allocator, &f.budget, &f.workspace, f.source(.{ 1440, 900 }), f.stamp(1));
    defer p.deinit();
    const input = try p.input();
    const primary = node(input, "c/editor/world.0.energy/primary");
    try t.expectEqual(@as(usize, 70005), primary.value.?.len);
    try t.expectEqualStrings(raw, primary.value.?[0..raw.len]);
    try t.expect(std.mem.endsWith(u8, primary.value.?, "語"));
    try t.expectEqualStrings("INVALID VALUE", primary.description);
    try t.expect(primary.value.?.ptr == node(input, "c/editor/world.0.energy/value").value.?.ptr);
    try f.editor.replaceFocusedText("1");
    try t.expectEqual(@as(usize, 70005), primary.value.?.len);
    try t.expectEqualStrings(raw, primary.value.?[0..raw.len]);
    try f.editor.focusField("world.0.name");
    try f.editor.replaceFocusedText("lower é world");
    f.refresh();
    var names = try ui.prepareEditor(t.allocator, &f.budget, &f.workspace, f.source(.{ 1440, 900 }), f.stamp(2));
    defer names.deinit();
    const named = try names.input();
    try t.expectEqualStrings("lower é world | SELECTED", node(named, "c/editor/world.select.0/primary").value.?);
    try t.expectEqualStrings("TACTICAL PREVIEW | WORLD 0 | lower é world", node(named, "c/editor/preview.world/primary").name);
}
const store_model = @import("classic_store.zig");
const session_model = @import("classic_session.zig");
test "H3b UI store and action categories preserve full typed failure and raw independent lanes" {
    var f: Fixture = .{};
    try f.init();
    defer f.deinit();
    const codecs = [_]sc.CodecFailure{ .too_large, .{ .json = error.BadJson }, .version, .{ .canonical_number = .{ .field = "originalfield", .kind = "u64" } }, .{ .validation = .{} } };
    const labels = [_][]const u8{ "SCENARIO JSON EXCEEDS THE 65536-BYTE LIMIT", "INVALID SCENARIO JSON", "UNSUPPORTED SCENARIO FORMAT VERSION", "INVALID CANONICAL SCENARIO NUMBER", "SCENARIO VALIDATION FAILED" };
    for (&codecs, labels) |*codec, label| {
        f.status = .{ .failure = .{ .codec = codec.* } };
        const failure: tx.Failure = .{ .codec = codec.* };
        var source = f.source(.{ 960, 600 });
        source.action_failure = &failure;
        var p = try ui.prepareEditor(t.allocator, &f.budget, &f.workspace, source, f.stamp(1));
        defer p.deinit();
        try t.expectEqualStrings(label, node(try p.input(), "c/editor/status.store/primary").value.?);
        try t.expectEqualStrings(label, node(try p.input(), "c/editor/action_failure/primary").name);
    }
    const action_failures = [_]tx.Failure{ .invalid_text, .confirmation_active, .no_confirmation, .stale_confirmation, .{ .validation = .{} }, .{ .admission = error.Validation }, .{ .admission = error.InvalidInitialState }, .{ .admission = error.StaleFingerprint } };
    const action_labels = [_][]const u8{ "INVALID TEXT", "CONFIRMATION ACTIVE", "NO CONFIRMATION", "STALE CONFIRMATION", "SCENARIO VALIDATION FAILED", "SCENARIO VALIDATION FAILED", "SCENARIO INITIAL STATE INVALID", "SCENARIO FINGERPRINT IS STALE" };
    for (&action_failures, action_labels) |*failure, label| {
        var source = f.source(.{ 960, 600 });
        source.action_failure = failure;
        var p = try ui.prepareEditor(t.allocator, &f.budget, &f.workspace, source, f.stamp(1));
        defer p.deinit();
        try t.expectEqualStrings(label, node(try p.input(), "c/editor/action_failure/text").name);
    }
    const store_labels = [_][]const u8{ "SCENARIO STORAGE FAILED", "SCENARIO STORAGE SLOT EXCEEDS THE 18446744073709551615-BYTE LIMIT", "SCENARIO STORAGE FAILED BEFORE ATOMIC PERSIST", "BROWSER SCENARIO STORAGE FAILED", "SCENARIO STORAGE PATH UNAVAILABLE" };
    var cause = [_]u8{ 'c', 'a', 'u', 's', 'e' };
    for (std.meta.tags(store_model.FailureKind), store_labels) |kind, label| {
        f.status = .{ .failure = .{ .storage = .{ .record = .scenario, .kind = kind, .cause = error.Original, .message = &cause, .max_bytes = std.math.maxInt(usize), .cleanup_error = error.Secondary } } };
        var p = try ui.prepareEditor(t.allocator, &f.budget, &f.workspace, f.source(.{ 960, 600 }), f.stamp(1));
        defer p.deinit();
        try t.expectEqualStrings(label, node(try p.input(), "c/editor/status.store/value").value.?);
        try t.expect(f.status.failure.storage.message.?.ptr == &cause and f.status.failure.storage.cause.? == error.Original and f.status.failure.storage.cleanup_error.? == error.Secondary);
    }
    const failure: tx.Failure = .invalid_text;
    var source = f.source(.{ 960, 600 });
    source.action_failure = &failure;
    source.raw_status = "raw é storage";
    source.raw_action_error = "raw 語 action";
    source.raw_recovery = "raw é recovery";
    source.recovery = .{ .resource = .{ .editor = error.Original } };
    var p = try ui.prepareEditor(t.allocator, &f.budget, &f.workspace, source, f.stamp(1));
    defer p.deinit();
    try t.expectEqualStrings("raw é storage", node(try p.input(), "c/editor/status.store/value").value.?);
    try t.expectEqualStrings("raw 語 action", node(try p.input(), "c/editor/action_failure/text").name);
    try t.expectEqualStrings("raw é recovery", node(try p.input(), "c/editor/recovery/text").name);
    f.status.failure.storage.record = .preferences;
    try t.expectError(error.InvalidRecoveryRecord, ui.prepareEditor(t.allocator, &f.budget, &f.workspace, source, f.stamp(1)));
    f.status = .none;
    var wrong: store_model.PreferencesFailure = .{ .store = .{ .record = .scenario, .kind = .io } };
    source.recovery = .{ .preferences = &wrong };
    try t.expectError(error.InvalidRecoveryRecord, ui.prepareEditor(t.allocator, &f.budget, &f.workspace, source, f.stamp(1)));
    source.recovery = .{ .resource = .{} };
    try t.expectError(error.InvalidSource, ui.prepareEditor(t.allocator, &f.budget, &f.workspace, source, f.stamp(1)));
}
test "H3b UI all closed recovery command admission resource tick preference categories" {
    var f: Fixture = .{};
    try f.init();
    defer f.deinit();
    var source = f.source(.{ 960, 600 });
    const commands = [_]@import("classic_commands.zig").DispatchError{ error.ModeRefused, error.OutOfMemory, error.MissingSource, error.MissingTarget, error.WrongTick, error.DuplicateSequence, error.CampaignFinished, error.InvalidWorld, error.SameSourceAndTarget, error.SourceNotOwnedByIssuer, error.InsufficientEnergy, error.FieldAtLimit };
    const command_labels = [_][]const u8{ "COMMAND UNAVAILABLE IN CURRENT MODE", "COMMAND MEMORY ALLOCATION FAILED", "SELECT A SOURCE WORLD", "SELECT A TARGET WORLD", "REJECTED - WRONG TICK", "REJECTED - DUPLICATE COMMAND", "REJECTED - CAMPAIGN FINISHED", "REJECTED - INVALID WORLD", "REJECTED - CHOOSE ANOTHER TARGET", "REJECTED - SOURCE IS NOT UNION", "REJECTED - INSUFFICIENT ENERGY", "REJECTED - FIELD AT LIMIT" };
    for (commands, command_labels) |command, label| {
        const failure: session_model.GeneralFailure = .{ .command = command };
        source.recovery = .{ .general = &failure };
        var p = try ui.prepareEditor(t.allocator, &f.budget, &f.workspace, source, f.stamp(1));
        defer p.deinit();
        try t.expectEqualStrings(label, node(try p.input(), "c/editor/recovery/text").name);
    }
    const admissions = [_]sc.AdmissionError{ error.Validation, error.InvalidInitialState, error.StaleFingerprint };
    const admission_labels = [_][]const u8{ "SCENARIO VALIDATION FAILED", "SCENARIO INITIAL STATE INVALID", "SCENARIO FINGERPRINT IS STALE" };
    for (admissions, admission_labels) |admission, label| {
        const failure: session_model.GeneralFailure = .{ .admission = admission };
        source.recovery = .{ .general = &failure };
        var p = try ui.prepareEditor(t.allocator, &f.budget, &f.workspace, source, f.stamp(1));
        defer p.deinit();
        try t.expectEqualStrings(label, node(try p.input(), "c/editor/recovery/text").name);
    }
    const resources = [_]session_model.ResourceErrors{ .{ .preferences = error.OutOfMemory }, .{ .editor = error.AnyEditorFailure }, .{ .command = error.OutOfMemory }, .{ .preferences = error.OutOfMemory, .editor = error.AnyEditorFailure, .command = error.OutOfMemory } };
    const resource_labels = [_][]const u8{ "PREFERENCE MEMORY ALLOCATION FAILED", "EDITOR OPERATION FAILED", "COMMAND MEMORY ALLOCATION FAILED", "PREFERENCE MEMORY ALLOCATION FAILED | EDITOR OPERATION FAILED | COMMAND MEMORY ALLOCATION FAILED" };
    for (resources, resource_labels) |resource, label| {
        source.recovery = .{ .resource = resource };
        var p = try ui.prepareEditor(t.allocator, &f.budget, &f.workspace, source, f.stamp(1));
        defer p.deinit();
        const text = node(try p.input(), "c/editor/recovery/text").name;
        try t.expectEqualStrings(label, text);
        try t.expect(text.len <= 128);
    }
    source.recovery = .{ .tick = .{ .cause = error.OutOfMemory, .tick = std.math.maxInt(u64) } };
    var tick = try ui.prepareEditor(t.allocator, &f.budget, &f.workspace, source, f.stamp(1));
    try t.expectEqualStrings("SIMULATION MEMORY ALLOCATION FAILED AT TICK 18446744073709551615", node(try tick.input(), "c/editor/recovery/text").name);
    tick.deinit();
    const codecs = [_]@import("preferences.zig").CodecFailure{ .too_large, .version, .{ .ui_scale = 65535 }, .{ .json = error.BadJson } };
    const codec_labels = [_][]const u8{ "preferences JSON exceeds the 16384-byte limit", "unsupported preferences format version", "unsupported UI scale percentage 65535", "invalid preferences JSON" };
    for (codecs, codec_labels) |codec, label| {
        const failure: store_model.PreferencesFailure = .{ .codec = codec };
        source.recovery = .{ .preferences = &failure };
        var p = try ui.prepareEditor(t.allocator, &f.budget, &f.workspace, source, f.stamp(1));
        defer p.deinit();
        try t.expectEqualStrings(label, node(try p.input(), "c/editor/recovery/text").name);
    }
    const store_labels = [_][]const u8{ "PREFERENCE STORAGE FAILED", "preference storage slot exceeds the 18446744073709551615-byte limit", "PREFERENCE STORAGE FAILED BEFORE ATOMIC PERSIST", "BROWSER PREFERENCE STORAGE FAILED", "PREFERENCE STORAGE PATH UNAVAILABLE" };
    for (std.meta.tags(store_model.FailureKind), store_labels) |kind, label| {
        const failure: store_model.PreferencesFailure = .{ .store = .{ .record = .preferences, .kind = kind, .max_bytes = std.math.maxInt(usize) } };
        source.recovery = .{ .preferences = &failure };
        var p = try ui.prepareEditor(t.allocator, &f.budget, &f.workspace, source, f.stamp(1));
        defer p.deinit();
        try t.expectEqualStrings(label, node(try p.input(), "c/editor/recovery/text").name);
    }
}
test "H3b UI actual initialized raw backing overlapping Workspace rejects before writes" {
    var f: Fixture = .{};
    try f.init();
    defer f.deinit();
    const bytes = std.mem.asBytes(&f.workspace.descriptors);
    @memset(bytes, 'x');
    f.workspace.borrowed = false;
    var source = f.source(.{ 960, 600 });
    source.raw_status = bytes[0..32];
    try t.expectError(error.AliasedWorkspace, ui.prepareEditor(t.allocator, &f.budget, &f.workspace, source, f.stamp(1)));
    try t.expectEqualSlices(u8, &@as([32]u8, @splat('x')), bytes[0..32]);
}
test "H3b UI exact U arena and T copies boundaries full input preserve retry" {
    var f: Fixture = .{};
    try f.init();
    defer f.deinit();
    f.editor.section = .validation;
    var source = f.source(.{ 960, 600 });
    var baseline = try ui.prepareEditor(t.allocator, &f.budget, &f.workspace, source, f.stamp(1));
    defer baseline.deinit();
    const used = baseline.header.?.m.used;
    const total = textBytes(try baseline.input());
    baseline.deinit();
    const unique_length = 196608 - used;
    const preedit = try t.allocator.alloc(u8, unique_length + 1);
    defer t.allocator.free(preedit);
    @memset(preedit, 'p');
    try f.editor.setPreedit(preedit[0..unique_length]);
    source = f.source(.{ 960, 600 });
    var exact = try ui.prepareEditor(t.allocator, &f.budget, &f.workspace, source, f.stamp(2));
    defer exact.deinit();
    try t.expectEqual(@as(usize, 196608), exact.header.?.m.used);
    exact.deinit();
    try f.editor.setPreedit(preedit);
    source = f.source(.{ 960, 600 });
    try t.expectError(error.SnapshotLimit, ui.prepareEditor(t.allocator, &f.budget, &f.workspace, source, f.stamp(3)));
    try t.expectEqualStrings(preedit, f.editor.preedit());
    try t.expect(!f.workspace.borrowed);
    try f.editor.setPreedit("");
    source = f.source(.{ 960, 600 });
    source.raw_status = "";
    baseline = try ui.prepareEditor(t.allocator, &f.budget, &f.workspace, source, f.stamp(4));
    const status_total = textBytes(try baseline.input());
    baseline.deinit();
    const odd = (262144 - status_total) % 2;
    if (odd != 0) try f.editor.setPreedit("pp");
    source = f.source(.{ 960, 600 });
    source.raw_status = "";
    baseline = try ui.prepareEditor(t.allocator, &f.budget, &f.workspace, source, f.stamp(5));
    const adjusted = textBytes(try baseline.input());
    baseline.deinit();
    const text_length = (262144 - adjusted) / 2;
    const status = try t.allocator.alloc(u8, text_length + 1);
    defer t.allocator.free(status);
    @memset(status, 's');
    source.raw_status = status[0..text_length];
    exact = try ui.prepareEditor(t.allocator, &f.budget, &f.workspace, source, f.stamp(6));
    try t.expectEqual(@as(usize, 262144), textBytes(try exact.input()));
    try t.expect(exact.header.?.m.used < 196608);
    exact.deinit();
    source.raw_status = status;
    try t.expectError(error.TextLimit, ui.prepareEditor(t.allocator, &f.budget, &f.workspace, source, f.stamp(7)));
    try t.expectEqual(@as(usize, 0), f.budget.ledger().total);
    std.debug.print("H3b U exact=196608 baselineU={d} baselineT={d} orphan={d}; T exact=262144 baselineT={d} rawstatus={d}; one-less admission tested via one extra byte\n", .{ used, total, unique_length, adjusted, text_length });
}
const Observer = struct {
    backing: std.mem.Allocator = t.allocator,
    budget: ?*H1.PreparationBudget = null,
    lengths: [128]usize = undefined,
    pointers: [128][*]u8 = undefined,
    alignments: [128]std.mem.Alignment = undefined,
    count: usize = 0,
    frees: usize = 0,
    live: usize = 0,
    peak: usize = 0,
    header_free: bool = false,
    failed: bool = false,
    fn allocator(self: *Observer) std.mem.Allocator {
        return .{ .ptr = self, .vtable = &.{ .alloc = allocate, .resize = resize, .remap = remap, .free = free } };
    }
    fn allocate(ctx: *anyopaque, len: usize, alignment: std.mem.Alignment, ra: usize) ?[*]u8 {
        const self: *Observer = @ptrCast(@alignCast(ctx));
        if (self.budget) |budget| {
            if (budget.ledger().total != self.live) self.failed = true;
            if (budget.pending_bytes != len or budget.active_reservations != 1) self.failed = true;
        }
        const data = self.backing.rawAlloc(len, alignment, ra) orelse return null;
        std.debug.assert(self.count < self.lengths.len);
        self.lengths[self.count] = len;
        self.pointers[self.count] = data;
        self.alignments[self.count] = alignment;
        self.count += 1;
        self.live += len;
        self.peak = @max(self.peak, self.live);
        return data;
    }
    fn resize(_: *anyopaque, _: []u8, _: std.mem.Alignment, _: usize, _: usize) bool {
        @panic("unexpected resize");
    }
    fn remap(_: *anyopaque, _: []u8, _: std.mem.Alignment, _: usize, _: usize) ?[*]u8 {
        @panic("unexpected remap");
    }
    fn free(ctx: *anyopaque, bytes: []u8, alignment: std.mem.Alignment, ra: usize) void {
        const self: *Observer = @ptrCast(@alignCast(ctx));
        if (self.budget) |budget| {
            if (budget.ledger().total != self.live) self.failed = true;
            if (bytes.len == ui.HEADER_BYTES) {
                if (budget.ledger().frame_scratch < ui.HEADER_BYTES) self.failed = true;
                self.header_free = true;
            }
        }
        self.backing.rawFree(bytes, alignment, ra);
        self.live -= bytes.len;
        self.frees += 1;
    }
};
const hud = @import("classic_ui.zig");
fn hudStamp(f: *Fixture, revision: u64) hud.CurrentStamp {
    return .{ .revision = revision, .owner = .{ .resident = f.resident, .draft = null, .hold = 7, .context = "hud" } };
}
fn fullHash(view: H1.FrameView) u64 {
    var hash = std.hash.Wyhash.init(0);
    for (view.nodes) |n| {
        hash.update(n.id.bytes);
        hash.update(n.name);
        hash.update(n.description);
        if (n.value) |value| hash.update(value);
        if (n.action) |a| hash.update(a.bytes);
    }
    for (view.focus.base) |id| hash.update(id.bytes);
    if (view.focus.modal) |order| for (order) |id| {
        hash.update(id.bytes);
    };
    if (view.focus.focused) |id| hash.update(id.bytes);
    if (view.focus.restore) |id| hash.update(id.bytes);
    hash.update(view.owner.context);
    return hash.final();
}
fn installHud(f: *Fixture, allocator: std.mem.Allocator, slot: *H1.FrameSlot, revision: *?u64) !void {
    var p = try hud.prepareHudSettings(allocator, &f.budget, .{ .source_name = "SOURCE", .target_name = "TARGET", .help_visible = true, .recoverable = "FULL HUD FAILURE" }, hudStamp(f, 1));
    defer p.deinit();
    try hud.publishHudSettings(slot, revision, &p, hudStamp(f, 1));
}
fn installEditor(f: *Fixture, allocator: std.mem.Allocator, slot: *H1.FrameSlot, revision: *?u64) !void {
    var source = f.source(.{ 1440, 900 });
    source.raw_status = "FULL STORAGE é";
    source.raw_action_error = "FULL ACTION 語";
    if (f.editor.modal.kind != null) source.focus = .{ .current = .{ .action = .confirm }, .saved_base = .{ .order = &base_rules, .restore = .{ .field = .{ .global = .seed } } } };
    var p = try ui.prepareEditor(allocator, &f.budget, &f.workspace, source, f.stamp(2));
    defer p.deinit();
    try ui.publishEditor(slot, revision, &p, f.stamp(2));
}
fn richEditor(f: *Fixture) !void {
    f.editor.section = .rules;
    try f.editor.focusField("scenario.seed");
    try f.editor.setPreedit("語");
    const capsule = try t.allocator.dupe(u8, "EXACT SAVED ONE SHOT CAPSULE");
    f.editor.modal.begin(.overwrite_save, capsule, null);
    f.refresh();
}
fn sweepTransition(to_editor: bool) !void {
    var requests: usize = 0;
    var measured_peak: usize = 0;
    {
        var f: Fixture = .{};
        try f.init();
        defer f.deinit();
        try richEditor(&f);
        var observer = Observer{ .budget = &f.budget };
        const allocator = observer.allocator();
        var slot = H1.FrameSlot.init(allocator, &f.budget);
        var revision: ?u64 = null;
        if (to_editor) try installHud(&f, allocator, &slot, &revision) else try installEditor(&f, allocator, &slot, &revision);
        const before = observer.count;
        const initial_peak = observer.peak;
        observer.peak = observer.live;
        if (to_editor) try installEditor(&f, allocator, &slot, &revision) else try installHud(&f, allocator, &slot, &revision);
        requests = observer.count - before;
        measured_peak = observer.peak;
        try t.expectEqual(@max(initial_peak, measured_peak), f.budget.ledger().peak);
        std.debug.print("H3b {s} requests={d} lengths={any} peakP={d}\n", .{ if (to_editor) "HUD->Editor" else "Editor->HUD", requests, observer.lengths[before..observer.count], measured_peak });
        slot.deinit();
        try t.expectEqual(@as(usize, 0), observer.live);
        try t.expect(observer.header_free and !observer.failed);
        try t.expectEqual(observer.count, observer.frees);
    }
    for (0..requests) |ordinal| {
        var f: Fixture = .{};
        try f.init();
        defer f.deinit();
        try richEditor(&f);
        var failure = std.testing.FailingAllocator.init(t.allocator, .{});
        var observer = Observer{ .budget = &f.budget, .backing = failure.allocator() };
        const allocator = observer.allocator();
        var slot = H1.FrameSlot.init(allocator, &f.budget);
        defer slot.deinit();
        var revision: ?u64 = null;
        if (to_editor) try installHud(&f, allocator, &slot, &revision) else try installEditor(&f, allocator, &slot, &revision);
        const old = slot.view().?;
        const hash = fullHash(old);
        const old_revision = revision;
        const live = f.budget.ledger().total;
        const preedit = f.editor.preedit().ptr;
        const capsule = f.editor.modal.save_bytes.?.ptr;
        failure.fail_index = failure.alloc_index + ordinal;
        if (to_editor) try t.expectError(error.OutOfMemory, installEditor(&f, allocator, &slot, &revision)) else try t.expectError(error.OutOfMemory, installHud(&f, allocator, &slot, &revision));
        try t.expect(slot.view().?.nodes.ptr == old.nodes.ptr and slot.view().?.child_indices.ptr == old.child_indices.ptr and slot.view().?.owner.matches(old.owner));
        try t.expectEqual(hash, fullHash(slot.view().?));
        try t.expectEqual(old_revision, revision);
        try t.expectEqual(live, f.budget.ledger().total);
        try t.expectEqual(@as(usize, 0), f.budget.pending_bytes);
        try t.expectEqual(preedit, f.editor.preedit().ptr);
        try t.expect(!f.workspace.borrowed);
        try t.expectEqual(capsule, f.editor.modal.save_bytes.?.ptr);
        try t.expectEqualStrings("EXACT SAVED ONE SHOT CAPSULE", f.editor.modal.save_bytes.?);
        failure.fail_index = std.math.maxInt(usize);
        if (to_editor) try installEditor(&f, allocator, &slot, &revision) else try installHud(&f, allocator, &slot, &revision);
    }
    for ([_]usize{ measured_peak, measured_peak - 1 }) |cap| {
        var f: Fixture = .{};
        try f.init();
        defer f.deinit();
        try richEditor(&f);
        var observer = Observer{ .budget = &f.budget };
        const allocator = observer.allocator();
        var slot = H1.FrameSlot.init(allocator, &f.budget);
        defer slot.deinit();
        var revision: ?u64 = null;
        if (to_editor) try installHud(&f, allocator, &slot, &revision) else try installEditor(&f, allocator, &slot, &revision);
        f.budget.max_live_requested_bytes = cap;
        const old = slot.view().?;
        const hash = fullHash(old);
        const old_revision = revision;
        if (cap == measured_peak) {
            if (to_editor) try installEditor(&f, allocator, &slot, &revision) else try installHud(&f, allocator, &slot, &revision);
        } else {
            if (to_editor) try t.expectError(error.LimitExceeded, installEditor(&f, allocator, &slot, &revision)) else try t.expectError(error.LimitExceeded, installHud(&f, allocator, &slot, &revision));
            try t.expect(slot.view().?.nodes.ptr == old.nodes.ptr);
            try t.expectEqual(hash, fullHash(slot.view().?));
            try t.expectEqual(old_revision, revision);
        }
    }
}
test "H3b UI all actual allocation ordinal OOM HUD Editor populated overlap exact peak and one less" {
    try sweepTransition(true);
    try sweepTransition(false);
}
test "H3b UI actual Session wrapper recovery priority and mode policy" {
    var f: Fixture = .{};
    try f.init();
    defer f.deinit();
    var draft = try sc.ScenarioDraft.factoryDefault(t.allocator);
    defer draft.deinit();
    const candidate = try draft.validated();
    var store = store_model.MemoryStore.init(t.allocator, .preferences);
    defer store.deinit();
    var session = try session_model.Session.init(t.allocator, &candidate, store.raw());
    defer session.deinit();
    try t.expectError(error.WrongProjectionMode, ui.prepareEditorFromSession(t.allocator, &f.budget, &f.workspace, &session, .{ .current = null }, f.stamp(1)));
    _ = try session.editor.open(&session.core);
    session.core.setViewport(.{ .x = 960, .y = 600 });
    session.general_failure = .{ .command = error.ModeRefused };
    session.resource_errors = .{ .editor = error.Example };
    session.preference_failure = .{ .codec = .version };
    var p = try ui.prepareEditorFromSession(t.allocator, &f.budget, &f.workspace, &session, .{ .current = null }, f.stamp(1));
    try t.expectEqualStrings("COMMAND UNAVAILABLE IN CURRENT MODE", node(try p.input(), "c/editor/recovery/text").name);
    p.deinit();
    session.general_failure = null;
    p = try ui.prepareEditorFromSession(t.allocator, &f.budget, &f.workspace, &session, .{ .current = null }, f.stamp(2));
    try t.expectEqualStrings("EDITOR OPERATION FAILED", node(try p.input(), "c/editor/recovery/text").name);
    p.deinit();
    session.resource_errors = .{};
    p = try ui.prepareEditorFromSession(t.allocator, &f.budget, &f.workspace, &session, .{ .current = null }, f.stamp(3));
    try t.expectEqualStrings("unsupported preferences format version", node(try p.input(), "c/editor/recovery/text").name);
    p.deinit();
    try t.expect(session.preference_failure != null);
}
const MirrorToken = struct { allocator: std.mem.Allocator, refs: usize, budget: *H1.LiveTokenBudget, charged_bytes: usize };
fn observedToken(observer: *Observer, index: usize, expected_budget: *H1.LiveTokenBudget) !*MirrorToken {
    try t.expectEqual(@sizeOf(MirrorToken), observer.lengths[index]);
    try t.expect(observer.alignments[index].toByteUnits() >= @alignOf(MirrorToken));
    const pointer: *MirrorToken = @ptrCast(@alignCast(observer.pointers[index]));
    try t.expect(pointer.allocator.ptr == observer.allocator().ptr and pointer.allocator.vtable == observer.allocator().vtable);
    try t.expect(pointer.budget == expected_budget);
    try t.expectEqual(observer.lengths[index], pointer.charged_bytes);
    try t.expectEqual(@as(usize, 1), pointer.refs);
    return pointer;
}
test "H3b UI synthetic test only observed token ABI first second and same token retain overflow unwind" {
    var f: Fixture = .{};
    try f.init();
    defer f.deinit();
    f.resident.release();
    f.draft.release();
    var observer = Observer{};
    f.resident = try H1.TokenRef.create(observer.allocator(), &f.tokens);
    f.draft = try H1.TokenRef.create(observer.allocator(), &f.tokens);
    const resident = try observedToken(&observer, 0, &f.tokens);
    const draft = try observedToken(&observer, 1, &f.tokens);
    for (0..3) |case| {
        var current = f.stamp(1);
        const original_resident = resident.refs;
        const original_draft = draft.refs;
        defer {
            resident.refs = original_resident;
            draft.refs = original_draft;
        }
        if (case == 0) resident.refs = std.math.maxInt(usize) else if (case == 1) {
            resident.refs = std.math.maxInt(usize) - 1;
            current.owner.draft = f.resident;
        } else draft.refs = std.math.maxInt(usize);
        const injected_resident = resident.refs;
        const injected_draft = draft.refs;
        const source = f.source(.{ 960, 600 });
        const text = f.editor.fieldText("world.0.name").?.ptr;
        try t.expectError(error.ReferenceOverflow, ui.prepareEditor(t.allocator, &f.budget, &f.workspace, source, current));
        try t.expectEqual(injected_resident, resident.refs);
        try t.expectEqual(injected_draft, draft.refs);
        try t.expectEqual(@as(usize, 0), f.budget.ledger().total);
        try t.expectEqual(@as(usize, 0), f.budget.pending_bytes);
        try t.expectEqual(text, f.editor.fieldText("world.0.name").?.ptr);
        try t.expect(!f.workspace.borrowed);
    }
    // Restore before legitimate releases; this is fault injection, not reachable live admission.
}
test "H3b UI stable source independent Header move deinit and captured context" {
    var f: Fixture = .{};
    try f.init();
    defer f.deinit();
    var context = [_]u8{ 'c', 'o', 'p', 'y' };
    var current = f.stamp(1);
    current.owner.context = &context;
    var p = try ui.prepareEditor(t.allocator, &f.budget, &f.workspace, f.source(.{ 1440, 900 }), current);
    var moved = p;
    p.header = null;
    defer moved.deinit();
    @memset(&context, 'x');
    f.report = .{};
    @memset(&f.flags, true);
    try f.editor.focusField("world.0.name");
    try f.editor.replaceFocusedText("changed");
    f.editor.deinit();
    f.editor_live = false;
    const input = try moved.input();
    try t.expectEqualStrings("copy", input.owner.context);
    try t.expect(!std.mem.startsWith(u8, node(input, "c/editor/world.0.name/primary").value.?, "changed"));
    try t.expectError(error.EmptyProjection, p.input());
    try t.expectError(error.EmptyProjection, p.geometry());
    moved.deinit();
    try t.expectError(error.EmptyProjection, moved.input());
    try t.expectEqual(@as(usize, 0), f.budget.ledger().total);
}
test "H3b UI all active source profiles and actual target named populations" {
    var f: Fixture = .{};
    try f.init();
    defer f.deinit();
    for ([_][2]f32{ .{ 0, 0 }, .{ 320, 220 }, .{ 960, 600 }, .{ 1099, 900 }, .{ 1100, 900 }, .{ 1440, 900 }, .{ std.math.nan(f32), std.math.inf(f32) } }) |viewport| for (std.meta.tags(e.Section)) |section| {
        f.editor.section = section;
        f.editor.focus = null;
        var p = try ui.prepareEditor(t.allocator, &f.budget, &f.workspace, f.source(viewport), f.stamp(1));
        defer p.deinit();
        const input = try p.input();
        try t.expect(input.nodes.len <= 480 and input.child_indices.len <= 479);
        try t.expect(textBytes(input) <= 262144 and p.header.?.m.used <= 196608);
        var slot = H1.FrameSlot.init(t.allocator, &f.budget);
        defer slot.deinit();
        var revision: ?u64 = null;
        try ui.publishEditor(&slot, &revision, &p, f.stamp(1));
    };
    std.debug.print("H3b Header={d} Metadata={d} Handle={d} Source={d} CurrentStamp={d} FocusInput={d} SavedBase={d} DraftView={d} Geometry={d} Workspace={d} report={d} flags={d} formatter=512 optionalrevision={d} static109table={d}; stack UNKNOWN\n", .{ ui.HEADER_BYTES, ui.METADATA_BYTES, @sizeOf(ui.PreparedEditor), @sizeOf(ui.Source), @sizeOf(ui.CurrentStamp), @sizeOf(ui.FocusInput), @sizeOf(ui.SavedBaseFocus), @sizeOf(e.DraftView), @sizeOf(ui.EditorGeometryView), @sizeOf(l.Workspace), @sizeOf(sc.ValidationReport), @sizeOf(l.ParseFlags), @sizeOf(?u64), @sizeOf(@TypeOf(ui.catalog)) });
}
test "H3b UI full stamp stale same draft new buffers budget mismatch and empty handles preserve publication" {
    var f: Fixture = .{};
    try f.init();
    defer f.deinit();
    var slot = H1.FrameSlot.init(t.allocator, &f.budget);
    defer slot.deinit();
    var revision: ?u64 = null;
    var p = try ui.prepareEditor(t.allocator, &f.budget, &f.workspace, f.source(.{ 960, 600 }), f.stamp(1));
    defer p.deinit();
    try ui.publishEditor(&slot, &revision, &p, f.stamp(1));
    const old = slot.view().?;
    const hash = fullHash(old);
    try f.editor.focusField("world.0.energy");
    try f.editor.replaceFocusedText("123");
    var current = f.stamp(2);
    try t.expectError(error.StaleProjection, ui.publishEditor(&slot, &revision, &p, current));
    try t.expectError(error.StaleProjection, ui.validateEditorTarget(slot.view().?, revision.?, current, null, .{ .bytes = "a/editor/footer.save" }));
    current = f.stamp(1);
    current.owner.hold = 8;
    try t.expectError(error.StaleProjection, ui.publishEditor(&slot, &revision, &p, current));
    current = f.stamp(1);
    current.owner.context = "different full context";
    try t.expectError(error.StaleProjection, ui.publishEditor(&slot, &revision, &p, current));
    current = f.stamp(1);
    current.owner.resident = f.draft;
    try t.expectError(error.StaleProjection, ui.publishEditor(&slot, &revision, &p, current));
    current = f.stamp(1);
    current.owner.draft = f.resident;
    try t.expectError(error.StaleProjection, ui.publishEditor(&slot, &revision, &p, current));
    var other_budget = H1.PreparationBudget.init(1179648);
    defer other_budget.deinit();
    var other_slot = H1.FrameSlot.init(t.allocator, &other_budget);
    defer other_slot.deinit();
    try t.expectError(error.BudgetMismatch, ui.publishEditor(&other_slot, &revision, &p, f.stamp(1)));
    const empty: ui.PreparedEditor = .{};
    try t.expectError(error.EmptyProjection, ui.publishEditor(&slot, &revision, &empty, f.stamp(1)));
    try t.expect(slot.view().?.nodes.ptr == old.nodes.ptr);
    try t.expectEqual(hash, fullHash(slot.view().?));
    try t.expectEqual(@as(?u64, 1), revision);
    current = f.stamp(1);
    current.owner.draft = null;
    try t.expectError(error.InvalidOwner, ui.prepareEditor(t.allocator, &f.budget, &f.workspace, f.source(.{ 960, 600 }), current));
    current = f.stamp(1);
    const long_context: [129]u8 = @splat('c');
    current.owner.context = &long_context;
    try t.expectError(error.InvalidOwner, ui.prepareEditor(t.allocator, &f.budget, &f.workspace, f.source(.{ 960, 600 }), current));
    var source = f.source(.{ 960, 600 });
    source.raw_recovery = &.{0xff};
    try t.expectError(error.InvalidUtf8, ui.prepareEditor(t.allocator, &f.budget, &f.workspace, source, f.stamp(2)));
}
test "H3b UI modal saved order malformed and duplicate keys reject before allocation" {
    var f: Fixture = .{};
    try f.init();
    defer f.deinit();
    f.editor.modal.begin(.revert, null, null);
    var source = f.source(.{ 960, 600 });
    const duplicate = [_]l.FocusKey{ .{ .action = .save }, .{ .action = .save } };
    source.focus = .{ .current = null, .saved_base = .{ .order = &duplicate, .restore = null } };
    try t.expectError(error.InvalidFocus, ui.prepareEditor(t.allocator, &f.budget, &f.workspace, source, f.stamp(1)));
    const modal_key = [_]l.FocusKey{.{ .action = .confirm }};
    source.focus.saved_base.?.order = &modal_key;
    try t.expectError(error.InvalidFocus, ui.prepareEditor(t.allocator, &f.budget, &f.workspace, source, f.stamp(1)));
    const malformed = [_]l.FocusKey{.{ .action = .{ .select_world = 7 } }};
    source.focus.saved_base.?.order = &malformed;
    try t.expectError(error.InvalidFocus, ui.prepareEditor(t.allocator, &f.budget, &f.workspace, source, f.stamp(1)));
    source.focus.saved_base = .{ .order = &base_rules, .restore = .{ .field = .{ .world = .{ .index = 7, .field = .name } } } };
    try t.expectError(error.InvalidFocus, ui.prepareEditor(t.allocator, &f.budget, &f.workspace, source, f.stamp(1)));
    try t.expectEqual(@as(usize, 0), f.budget.ledger().total);
}
test "H3b UI all 90 real typed field targets and readonly tick deny mutation" {
    var f: Fixture = .{};
    try f.init();
    defer f.deinit();
    for (0..8) |profile| {
        if (profile == 7) f.editor.section = .rules else {
            f.editor.section = .worlds;
            f.editor.selected_world = profile;
        }
        var p = try ui.prepareEditor(t.allocator, &f.budget, &f.workspace, f.source(.{ 1440, 900 }), f.stamp(profile + 1));
        defer p.deinit();
        var slot = H1.FrameSlot.init(t.allocator, &f.budget);
        defer slot.deinit();
        var revision: ?u64 = null;
        try ui.publishEditor(&slot, &revision, &p, f.stamp(profile + 1));
        for (e.catalog, 0..) |field, i| {
            const active = switch (field.id) {
                .global => profile == 7,
                .world => |world| world.index == profile,
            };
            if (!active) continue;
            try t.expectEqualDeep(ui.Target{ .field = field.id }, ui.catalog[i].target);
            try t.expectEqual(field.editable, ui.catalog[i].editable);
            if (field.editable) try t.expectEqualDeep(ui.Target{ .field = field.id }, try ui.validateEditorTarget(slot.view().?, revision.?, f.stamp(profile + 1), null, .{ .bytes = ui.catalog[i].id })) else try t.expectError(error.UnknownAction, ui.validateEditorTarget(slot.view().?, revision.?, f.stamp(profile + 1), null, .{ .bytes = ui.catalog[i].id }));
            try t.expect(f.editor.focus == null);
        }
    }
}
