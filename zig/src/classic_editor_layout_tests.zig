const std = @import("std");
const t = std.testing;
const l = @import("classic_editor_layout.zig");
const e = @import("classic_editor.zig");
const sc = @import("scenario.zig");
fn input(viewport: [2]f32, section: e.Section) l.LayoutInput {
    return .{ .viewport = viewport, .section = section, .selected_world = 0, .focused = null, .scroll = 0, .modal = null, .can_apply = true, .has_store_status = false, .issue_count = 0 };
}
test "H3b source geometry wide and compact literal fixtures" {
    var w: l.Workspace = .{};
    const wide = try l.layout(input(.{ 1440, 900 }, .worlds), &w);
    try t.expectEqual(@as(usize, 30), wide.widgets.len);
    try t.expectEqual(@as(f32, 1004), wide.content_rect.min[0]);
    try t.expectEqual(@as(f32, 56), wide.content_rect.min[1]);
    try t.expectEqual(@as(f32, 1424), wide.content_rect.max[0]);
    try t.expectEqual(@as(f32, 856), wide.content_rect.max[1]);
}
test "H3b focus traverses clipped fields and reveals exact source scroll" {
    var w: l.Workspace = .{};
    const v = try l.layout(input(.{ 960, 600 }, .worlds), &w);
    defer l.releaseView(&w);
    try t.expect(std.meta.eql(l.FocusKey{ .action = .{ .select_section = .worlds } }, l.first(v).?));
    const last_field: l.FocusKey = .{ .field = .{ .world = .{ .index = 0, .field = .topology } } };
    try t.expectEqual(@as(f32, 308), try l.reveal(v, last_field));
    try t.expectEqual(@as(f32, 316), try l.scrollTo(v, .end));
    try t.expect(std.meta.eql(l.FocusKey{ .action = .apply_and_restart }, l.next(v, null, true).?));
    try t.expectEqual(@as(?e.Action, null), l.actionAt(v, last_field));
    try t.expect(std.meta.eql(l.FocusKey{ .action = .{ .select_world = 0 } }, l.hit(v, .{ 0, 108 }).?));
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
test "H3b ordered errors exact grouped field and borrowed report text" {
    var report: sc.ValidationReport = .{};
    put(&report, .world_coordinate, .err, "worlds[0].position", "first coordinate");
    put(&report, .duplicate_position, .err, "worlds[0].position", "later duplicate");
    var flags: l.ParseFlags = @splat(false);
    const x: e.FieldId = .{ .world = .{ .index = 0, .field = .x } };
    const err = try l.fieldError(&report, &flags, x);
    try t.expect(err != null);
    try t.expectEqualStrings("first coordinate", try l.issueText(&report, err.?));
    try t.expect((try l.issueText(&report, err.?)).ptr == report.entries[0].message.bytes[0..].ptr);
    flags[x.slot()] = true;
    try t.expectEqualStrings("INVALID VALUE", try l.issueText(&report, (try l.fieldError(&report, &flags, x)).?));
    try t.expectEqual(@as(?l.IssueText, null), try l.fieldError(&report, &flags, .{ .world = .{ .index = 0, .field = .y } }));
}
const Family = struct { code: sc.ScenarioIssueCode, severity: sc.IssueSeverity, path: []const u8, target: ?e.FieldId };
const families = [_]Family{
    .{ .code = .tick_rate, .severity = .err, .path = "rules.tick_hz", .target = .{ .global = .tick_hz } },
    .{ .code = .win_world_count, .severity = .err, .path = "rules.win_world_count", .target = .{ .global = .win_world_count } },
    .{ .code = .ai_period, .severity = .err, .path = "rules.ai_period_ticks", .target = .{ .global = .ai_period_ticks } },
    .{ .code = .hazard_period, .severity = .err, .path = "rules.hazard_period_ticks", .target = .{ .global = .hazard_period_ticks } },
    .{ .code = .hazard_duration, .severity = .err, .path = "rules.hazard_duration_ticks", .target = .{ .global = .hazard_duration_ticks } },
    .{ .code = .hazard_boundary_arithmetic, .severity = .err, .path = "rules.hazard_first_tick", .target = .{ .global = .hazard_first_tick } },
    .{ .code = .energy_cap, .severity = .err, .path = "rules.maximum_energy", .target = .{ .global = .maximum_energy } },
    .{ .code = .defense_cap, .severity = .err, .path = "rules.maximum_defense", .target = .{ .global = .maximum_defense } },
    .{ .code = .minimum_launch, .severity = .err, .path = "rules.minimum_launch", .target = .{ .global = .minimum_launch } },
    .{ .code = .field_cost_relation, .severity = .err, .path = "rules.field_raise_cost", .target = .{ .global = .field_raise_cost } },
    .{ .code = .fleet_speed_arithmetic, .severity = .err, .path = "rules.base_fleet_speed", .target = .{ .global = .base_fleet_speed } },
    .{ .code = .world_name, .severity = .err, .path = "worlds[6].name", .target = .{ .world = .{ .index = 6, .field = .name } } },
    .{ .code = .world_coordinate, .severity = .err, .path = "worlds[6].position", .target = .{ .world = .{ .index = 6, .field = .x } } },
    .{ .code = .world_field_level, .severity = .err, .path = "worlds[6].fields", .target = .{ .world = .{ .index = 6, .field = .atmosphere } } },
    .{ .code = .world_energy, .severity = .err, .path = "worlds[6].energy", .target = .{ .world = .{ .index = 6, .field = .energy } } },
    .{ .code = .world_defense, .severity = .err, .path = "worlds[6].defense", .target = .{ .world = .{ .index = 6, .field = .defense } } },
    .{ .code = .production_arithmetic, .severity = .err, .path = "worlds[6].base_output", .target = .{ .world = .{ .index = 6, .field = .base_output } } },
    .{ .code = .regeneration_arithmetic, .severity = .err, .path = "worlds[6].base_regeneration", .target = .{ .world = .{ .index = 6, .field = .base_regeneration } } },
    .{ .code = .duplicate_position, .severity = .err, .path = "worlds[6].position", .target = .{ .world = .{ .index = 6, .field = .x } } },
    .{ .code = .duplicate_name, .severity = .warning, .path = "worlds[6].name", .target = .{ .world = .{ .index = 6, .field = .name } } },
    .{ .code = .close_position, .severity = .warning, .path = "worlds[6].position", .target = .{ .world = .{ .index = 6, .field = .x } } },
    .{ .code = .missing_union, .severity = .err, .path = "worlds", .target = null },
    .{ .code = .initial_victory, .severity = .err, .path = "worlds", .target = null },
    .{ .code = .missing_helix, .severity = .warning, .path = "worlds", .target = null },
    .{ .code = .missing_choir, .severity = .warning, .path = "worlds", .target = null },
    .{ .code = .no_launchable_union, .severity = .warning, .path = "worlds", .target = null },
};
test "H3b all twenty six independent code schema and severity mappings" {
    var flags: l.ParseFlags = @splat(false);
    for (families) |family| {
        var report: sc.ValidationReport = .{};
        put(&report, family.code, family.severity, family.path, "literal message");
        const mapped = try l.mapIssueField(&report, 0);
        try t.expect(std.meta.eql(family.target, mapped));
        if (family.target) |id| {
            const err = try l.fieldError(&report, &flags, id);
            try t.expect((err != null) == (family.severity == .err));
            if (family.severity == .err) try t.expectEqualStrings("literal message", try l.issueText(&report, err.?));
        }
    }
    var report: sc.ValidationReport = .{};
    put(&report, .world_field_level, .err, "worlds[0].fields", "field group");
    put(&report, .missing_union, .err, "worlds", "global issue");
    try t.expectEqual(@as(?l.IssueText, null), try l.fieldError(&report, &flags, .{ .world = .{ .index = 0, .field = .hydrosphere } }));
    try t.expectEqual(@as(?l.IssueText, null), try l.fieldError(&report, &flags, .{ .world = .{ .index = 0, .field = .topology } }));
    try t.expectEqual(@as(?l.IssueText, null), try l.fieldError(&report, &flags, .{ .world = .{ .index = 1, .field = .atmosphere } }));
    try t.expectEqual(@as(?l.IssueText, null), try l.fieldError(&report, &flags, .{ .global = .field_lower_refund }));
    flags[13 + 8] = true;
    try t.expectEqualStrings("INVALID VALUE", try l.issueText(&report, (try l.fieldError(&report, &flags, .{ .world = .{ .index = 0, .field = .atmosphere } })).?));
}
test "H3b malformed reports physical logical UTF8 paths and indices reject whole" {
    const paths = [_][]const u8{ "worlds[7].position", "worlds[00].position", "worlds[-1].position", "worlds[0]..position", "worlds[0].x", "worlds[0].position.extra", "worlds[0].energy", "rules.tick_hz" };
    for (paths) |path| {
        var report: sc.ValidationReport = .{};
        put(&report, .world_coordinate, .err, path, "message");
        try t.expectError(error.InvalidIssuePath, l.validateReport(&report));
    }
    var report: sc.ValidationReport = .{};
    put(&report, .tick_rate, .err, "rules.tick_hz", "message");
    const good = report.entries[0];
    for ([_]usize{ 28, 49, std.math.maxInt(usize) }) |len| {
        report.entries[0] = good;
        report.entries[0].field.len = len;
        try t.expectError(error.InvalidReport, l.validateReport(&report));
    }
    for ([_]usize{ 74, 97, std.math.maxInt(usize) }) |len| {
        report.entries[0] = good;
        report.entries[0].message.len = len;
        try t.expectError(error.InvalidReport, l.validateReport(&report));
    }
    report.entries[0] = good;
    report.entries[0].message.bytes[0] = 0xff;
    try t.expectError(error.InvalidReport, l.validateReport(&report));
    report.entries[0] = good;
    report.entries[0].field.bytes[0] = 0xff;
    try t.expectError(error.InvalidReport, l.validateReport(&report));
    report.entries[0] = good;
    report.entries[0].severity = .warning;
    try t.expectError(error.InvalidReport, l.validateReport(&report));
    report.entries[0] = good;
    var flags: l.ParseFlags = @splat(false);
    try t.expectError(error.InvalidField, l.fieldError(&report, &flags, .{ .world = .{ .index = 7, .field = .name } }));
    try t.expectError(error.InvalidReport, l.mapIssueField(&report, 1));
    try t.expectError(error.InvalidReport, l.issueText(&report, .{ .issue_index = 1 }));
    for ([_]usize{ 99, 100, 101, std.math.maxInt(usize) }) |len| {
        report.len = len;
        try t.expectError(error.InvalidReport, l.validateReport(&report));
    }
    // Malformed later record invalidates whole report even when the first matches or parse overrides.
    report.len = 1;
    put(&report, .world_coordinate, .err, "worlds[7].position", "later invalid");
    flags[1] = true;
    try t.expectError(error.InvalidIssuePath, l.fieldError(&report, &flags, .{ .global = .tick_hz }));
    try t.expectError(error.InvalidIssuePath, l.issueText(&report, .invalid_value));
}
fn descriptor(view: l.LayoutView, key: l.WidgetKey) l.Descriptor {
    for (view.widgets) |d| if (std.meta.eql(d.key, key)) return d;
    unreachable;
}
fn bounds(d: l.Descriptor, xywh: [4]f32) !void {
    try t.expectEqual(xywh[0], d.bounds.min[0]);
    try t.expectEqual(xywh[1], d.bounds.min[1]);
    try t.expectEqual(xywh[0] + xywh[2], d.bounds.max[0]);
    try t.expectEqual(xywh[1] + xywh[3], d.bounds.max[1]);
}
test "H3b literal all sections footer modal order and maximum population" {
    var w: l.Workspace = .{};
    var in = input(.{ 1440, 900 }, .worlds);
    in.has_store_status = true;
    var v = try l.layout(in, &w);
    try t.expectEqual(@as(usize, 31), v.widgets.len);
    try bounds(descriptor(v, .{ .section = .worlds }), .{ 16, 56, 208, 44 });
    try bounds(descriptor(v, .{ .world_choice = 0 }), .{ 16, 220, 208, 44 });
    try bounds(descriptor(v, .{ .world_choice = 6 }), .{ 16, 532, 208, 44 });
    try bounds(descriptor(v, .preview), .{ 240, 56, 748, 800 });
    try bounds(descriptor(v, .{ .footer = .revert }), .{ 0, 856, 1440.0 / 7.0, 44 });
    try t.expectEqual(@as(f32, 0), v.maximum_scroll);
    l.releaseView(&w);
    in.section = .rules;
    in.has_store_status = false;
    v = try l.layout(in, &w);
    try t.expectEqual(@as(usize, 25), v.widgets.len);
    const tick = descriptor(v, .{ .field = .{ .global = .tick_hz } });
    try t.expect(!tick.enabled and !tick.focusable);
    try t.expectEqual(@as(f32, 0), v.maximum_scroll);
    l.releaseView(&w);
    in.section = .validation;
    in.issue_count = 98;
    in.has_store_status = true;
    in.modal = .revert;
    v = try l.layout(in, &w);
    try t.expectEqual(@as(usize, 114), v.widgets.len);
    try t.expectEqual(@as(f32, 4296), v.maximum_scroll);
    try t.expect(std.meta.eql(l.WidgetKey{ .validation_issue = 0 }, v.widgets[6].key));
    try t.expect(std.meta.eql(l.WidgetKey{ .validation_issue = 97 }, v.widgets[103].key));
    try t.expect(v.widgets[111].key == .confirmation_message and v.widgets[112].key == .dismiss_confirmation and v.widgets[113].key == .confirm);
    try bounds(v.widgets[111], .{ 420, 362, 600, 132 });
    try bounds(v.widgets[112], .{ 420, 494, 300, 44 });
    try bounds(v.widgets[113], .{ 720, 494, 300, 44 });
    for (v.widgets[0..111]) |d| try t.expect(!d.enabled and !d.focusable);
    l.releaseView(&w);
    in = input(.{ 960, 600 }, .worlds);
    v = try l.layout(in, &w);
    try bounds(descriptor(v, .{ .section = .worlds }), .{ 0, 56, 320, 44 });
    try bounds(descriptor(v, .{ .world_choice = 0 }), .{ 0, 108, 960.0 / 7.0, 44 });
    try bounds(descriptor(v, .preview), .{ 16, 160, 928, 88 });
    try bounds(descriptor(v, .{ .field = .{ .world = .{ .index = 0, .field = .name } } }), .{ 16, 256, 928, 44 });
    try t.expectEqual(@as(f32, 512), v.content_rect.max[1]);
    try t.expectEqual(@as(f32, 316), v.maximum_scroll);
    try bounds(descriptor(v, .{ .footer = .revert }), .{ 0, 512, 160, 44 });
    try bounds(descriptor(v, .{ .footer = .apply_and_restart }), .{ 0, 556, 160, 44 });
    l.releaseView(&w);
    in = input(.{ 320, 220 }, .worlds);
    v = try l.layout(in, &w);
    try t.expectEqual(@as(f32, 56), v.content_rect.min[1]);
    try t.expectEqual(@as(f32, 56), v.content_rect.max[1]);
    try bounds(descriptor(v, .{ .footer = .revert }), .{ 0, 44, 160, 44 });
    try bounds(descriptor(v, .{ .footer = .apply_and_restart }), .{ 0, 176, 160, 44 });
    try t.expect(!descriptor(v, .{ .section = .worlds }).focusable);
    try t.expect(!descriptor(v, .{ .world_choice = 0 }).focusable);
    l.releaseView(&w);
}
test "H3b viewport cutoff nonfinite zero tiny source capability and resize" {
    var w: l.Workspace = .{};
    for ([_][2]f32{ .{ 1099, 600 }, .{ 1100, 600 }, .{ -1, -2 }, .{ std.math.nan(f32), std.math.inf(f32) }, .{ 0, 0 }, .{ 1, 1 } }) |viewport| {
        const v = try l.layout(input(viewport, .worlds), &w);
        try t.expect(v.wide == (viewport[0] >= 1100));
        if (!(viewport[0] > 0)) {
            try t.expectEqual(@as(f32, 0), v.viewport[0]);
            try t.expectEqual(@as(f32, 0), v.viewport[1]);
        }
        if (v.viewport[0] <= 1) {
            const section = descriptor(v, .{ .section = .worlds });
            const choice = descriptor(v, .{ .world_choice = 0 });
            try t.expect(section.enabled and !section.focusable and choice.enabled and !choice.focusable);
            try t.expectEqual(@as(?e.Action, null), l.actionAt(v, .{ .action = .{ .select_section = .worlds } }));
            try t.expectEqual(@as(?e.Action, null), l.actionAt(v, .{ .action = .{ .select_world = 0 } }));
            // Typed identity remains present. Future retained builder must set action=null.
            try t.expect(!descriptor(v, .{ .field = .{ .world = .{ .index = 0, .field = .name } } }).visible);
        }
        l.releaseView(&w);
    }
    const v = try l.layout(input(.{ 960, 600 }, .worlds), &w);
    defer l.releaseView(&w);
    try t.expect(std.meta.eql(e.Action{ .select_section = .worlds }, l.actionAt(v, .{ .action = .{ .select_section = .worlds } }).?));
    try t.expectEqual(@as(?l.FocusKey, null), l.hit(v, .{ std.math.nan(f32), 0 }));
    try t.expectEqual(@as(?l.FocusKey, null), l.hit(v, .{ 0, std.math.inf(f32) }));
}
test "H3b all seven modal reasons navigation disabled apply and invisible reachability" {
    var w: l.Workspace = .{};
    for (std.meta.tags(e.ModalKind)) |reason| {
        var in = input(.{ 960, 600 }, .worlds);
        in.modal = reason;
        const v = try l.layout(in, &w);
        try t.expect(std.meta.eql(l.FocusKey{ .action = .dismiss_confirmation }, l.first(v).?));
        try t.expect(std.meta.eql(l.FocusKey{ .action = .confirm }, l.next(v, null, true).?));
        try t.expect(std.meta.eql(l.FocusKey{ .action = .confirm }, l.next(v, l.first(v), false).?));
        try t.expect(std.meta.eql(l.FocusKey{ .action = .dismiss_confirmation }, l.next(v, .{ .action = .confirm }, false).?));
        try t.expectEqual(@as(?e.Action, null), l.actionAt(v, .{ .action = .{ .select_world = 0 } }));
        l.releaseView(&w);
    }
    var in = input(.{ 960, 600 }, .worlds);
    in.can_apply = false;
    const v = try l.layout(in, &w);
    defer l.releaseView(&w);
    const apply = descriptor(v, .{ .footer = .apply_and_restart });
    try t.expect(!apply.enabled and apply.focusable);
    try t.expectEqual(@as(?e.Action, null), l.actionAt(v, .{ .action = .apply_and_restart }));
    try t.expect(std.meta.eql(l.FocusKey{ .action = .cancel }, l.next(v, null, true).?));
    var key = l.first(v);
    var visited: usize = 0;
    var saw_topology = false;
    while (visited < 27) : (visited += 1) {
        if (std.meta.eql(key.?, l.FocusKey{ .field = .{ .world = .{ .index = 0, .field = .topology } } })) saw_topology = true;
        key = l.next(v, key, false);
    }
    try t.expect(saw_topology);
    try t.expect(std.meta.eql(l.FocusKey{ .action = .{ .select_section = .worlds } }, l.next(v, .{ .field = .{ .world = .{ .index = 6, .field = .name } } }, false).?));
    try t.expectEqual(v.scroll, try l.reveal(v, .{ .field = .{ .world = .{ .index = 6, .field = .name } } }));
    try t.expectEqual(v.scroll, try l.reveal(v, .{ .action = .cancel }));
}
fn zeroWorkspace() l.Workspace {
    var w: l.Workspace = undefined;
    @memset(std.mem.asBytes(&w), 0);
    return w;
}
fn rejectUnchanged(w: *l.Workspace, in: l.LayoutInput, expected: anyerror) !void {
    var before: [@sizeOf(l.Workspace)]u8 = undefined;
    @memcpy(&before, std.mem.asBytes(w));
    try t.expectError(expected, l.layout(in, w));
    try t.expectEqualSlices(u8, &before, std.mem.asBytes(w));
}
test "H3b entire workspace and borrowed flag preserved for every rejected input" {
    var w = zeroWorkspace();
    var in = input(.{ 960, 600 }, .worlds);
    in.selected_world = 7;
    try rejectUnchanged(&w, in, error.InvalidWorld);
    in.selected_world = 0;
    in.issue_count = 99;
    try rejectUnchanged(&w, in, error.InvalidReport);
    in.issue_count = 0;
    for ([_]f32{ -1, std.math.nan(f32), std.math.inf(f32), -std.math.inf(f32) }) |scroll| {
        in.scroll = scroll;
        try rejectUnchanged(&w, in, error.InvalidScroll);
    }
    in.scroll = 0;
    in.focused = .{ .field = .{ .world = .{ .index = 7, .field = .name } } };
    try rejectUnchanged(&w, in, error.InvalidFocus);
    in.focused = .{ .action = .{ .select_world = 7 } };
    try rejectUnchanged(&w, in, error.InvalidFocus);
    in.focused = null;
    const v = try l.layout(in, &w);
    try t.expect(w.borrowed);
    try t.expect(v.widgets.ptr == w.descriptors[0..].ptr);
    try rejectUnchanged(&w, in, error.WorkspaceBusy);
    l.releaseView(&w);
    // Release permits owner movement, then new borrows use moved storage.
    var moved = w;
    const moved_view = try l.layout(in, &moved);
    try t.expect(moved_view.widgets.ptr == moved.descriptors[0..].ptr);
    l.releaseView(&moved);
    const huge = try l.layout(input(.{ std.math.floatMax(f32), std.math.floatMax(f32) }, .worlds), &w);
    for (huge.widgets) |d| for (d.bounds.min, d.bounds.max) |lo, hi| try t.expect(std.math.isFinite(lo) and std.math.isFinite(hi));
    l.releaseView(&w);
}
test "H3b alias preflight checks complete workspace physical byte ranges" {
    var w = zeroWorkspace();
    var report: sc.ValidationReport = .{};
    var flags: l.ParseFlags = @splat(false);
    try l.validateWorkspaceBorrowing(&w, &report, &flags);
    var bytes: [@sizeOf(sc.ValidationReport) + @sizeOf(l.Workspace) + 64]u8 align(64) = @splat(0);
    const wp: *const l.Workspace = @ptrCast(&bytes[64]);
    const rp: *const sc.ValidationReport = @ptrCast(&bytes[0]);
    try t.expectError(error.AliasedWorkspace, l.validateWorkspaceBorrowing(wp, rp, &flags));
    const fp: *const l.ParseFlags = @ptrCast(&bytes[64 + @sizeOf(l.Workspace) - 1]);
    try t.expectError(error.AliasedWorkspace, l.validateWorkspaceBorrowing(wp, &report, fp));
    const read_alias: *const l.ParseFlags = @ptrCast(&report);
    try l.validateWorkspaceBorrowing(&w, &report, read_alias);
    const overflowing: *const l.ParseFlags = @ptrFromInt(std.math.maxInt(usize) - 1);
    try t.expectError(error.ArithmeticOverflow, l.validateWorkspaceBorrowing(&w, &report, overflowing));
}
test "H3b pure source inputs report flags layout bytes and scroll checked arithmetic" {
    var w = zeroWorkspace();
    var report: sc.ValidationReport = .{};
    put(&report, .tick_rate, .err, "rules.tick_hz", "real immutable report");
    var flags: l.ParseFlags = @splat(false);
    var before_report: [@sizeOf(sc.ValidationReport)]u8 = undefined;
    @memcpy(&before_report, std.mem.asBytes(&report));
    const before_flags = flags;
    var in = input(.{ 960, 600 }, .rules);
    in.scroll = 1000;
    const before_in = in;
    try l.validateWorkspaceBorrowing(&w, &report, &flags);
    const v = try l.layout(in, &w);
    defer l.releaseView(&w);
    var before_w: [@sizeOf(l.Workspace)]u8 = undefined;
    @memcpy(&before_w, std.mem.asBytes(&w));
    _ = try l.fieldError(&report, &flags, .{ .global = .tick_hz });
    _ = try l.issueText(&report, .{ .issue_index = 0 });
    _ = l.first(v);
    _ = l.next(v, null, true);
    _ = l.hit(v, .{ 16, 108 });
    _ = l.actionAt(v, .{ .action = .cancel });
    _ = try l.reveal(v, .{ .field = .{ .global = .seed } });
    try t.expectEqual(@as(f32, 0), try l.scrollTo(v, .home));
    try t.expectEqual(v.maximum_scroll, try l.scrollTo(v, .end));
    try t.expectEqual(v.scroll, try l.scrollTo(v, .{ .delta = std.math.nan(f32) }));
    try t.expectEqual(v.scroll, try l.scrollTo(v, .{ .delta = std.math.inf(f32) }));
    try t.expectEqual(@as(f32, 0), try l.scrollTo(v, .page_up));
    try t.expectEqual(v.maximum_scroll, try l.scrollTo(v, .page_down));
    var overflow_view = v;
    overflow_view.scroll = std.math.floatMax(f32);
    overflow_view.maximum_scroll = std.math.floatMax(f32);
    try t.expectError(error.InvalidScroll, l.scrollTo(overflow_view, .{ .delta = std.math.floatMax(f32) }));
    try t.expectEqualSlices(u8, &before_w, std.mem.asBytes(&w));
    try t.expectEqualSlices(u8, &before_report, std.mem.asBytes(&report));
    try t.expectEqualSlices(bool, &before_flags, &flags);
    try t.expect(std.meta.eql(before_in, in));
}
test "H3b target storage measurements no allocator parameter or heap requests" {
    std.debug.print("H3b target bytes Descriptor={d} Workspace={d} overhead={d} Input={d} View={d} Report={d} ParseFlags={d} catalog_static={d} fixture_local_named={d}; transitive stack UNKNOWN\n", .{ @sizeOf(l.Descriptor), @sizeOf(l.Workspace), @sizeOf(l.Workspace) - 114 * @sizeOf(l.Descriptor), @sizeOf(l.LayoutInput), @sizeOf(l.LayoutView), @sizeOf(sc.ValidationReport), @sizeOf(l.ParseFlags), @sizeOf(@TypeOf(e.catalog)), @sizeOf(l.Workspace) + @sizeOf(sc.ValidationReport) + @sizeOf(l.ParseFlags) + @sizeOf(l.LayoutInput) + @sizeOf(l.LayoutView) });
    var w: l.Workspace = .{};
    const v = try l.layout(input(.{ 960, 600 }, .worlds), &w);
    _ = l.first(v);
    l.releaseView(&w);
    try t.expect(@sizeOf(l.Descriptor) <= 96 and @sizeOf(l.Workspace) - 114 * @sizeOf(l.Descriptor) <= 16);
}
test "H3b reveal rejects finite intermediate overflow before clamp" {
    const d = l.Descriptor{ .key = .{ .field = .{ .global = .seed } }, .kind = .field, .bounds = .{ .min = .{ 0, std.math.floatMax(f32) }, .max = .{ 1, std.math.floatMax(f32) } }, .enabled = true, .selected = false, .focusable = true, .visible = false };
    const v = l.LayoutView{ .widgets = &.{d}, .viewport = .{ 1, 1 }, .wide = false, .content_rect = .{ .min = .{ 0, 0 }, .max = .{ 1, 1 } }, .maximum_scroll = std.math.floatMax(f32), .scroll = std.math.floatMax(f32) };
    try t.expectError(error.InvalidScroll, l.reveal(v, .{ .field = .{ .global = .seed } }));
}
test "H3b reverse inclusive overlap source order field ordering and zero modal" {
    var w: l.Workspace = .{};
    var in = input(.{ 960, 600 }, .worlds);
    in.scroll = std.math.floatMax(f32);
    var v = try l.layout(in, &w);
    try t.expectEqual(@as(f32, 316), v.scroll);
    try t.expect(std.meta.eql(l.FocusKey{ .action = .{ .select_section = .rules } }, l.hit(v, .{ 320, 100 }).?));
    try t.expect(std.meta.eql(l.FocusKey{ .action = .{ .select_world = 0 } }, l.hit(v, .{ 0, 152 }).?));
    const field_order = [_]e.WorldField{ .name, .x, .y, .owner, .energy, .defense, .base_output, .base_regeneration, .atmosphere, .hydrosphere, .topology };
    for (field_order, 0..) |field, i| try t.expect(std.meta.eql(l.WidgetKey{ .field = .{ .world = .{ .index = 0, .field = field } } }, v.widgets[12 + i].key));
    const footers = [_]l.Footer{ .revert, .factory_defaults, .regenerate_from_seed, .load, .save, .cancel, .apply_and_restart };
    for (footers, 0..) |footer, i| try t.expect(std.meta.eql(l.WidgetKey{ .footer = footer }, v.widgets[23 + i].key));
    l.releaseView(&w);
    in = input(.{ 0, 0 }, .worlds);
    in.modal = .cancel_dirty;
    v = try l.layout(in, &w);
    defer l.releaseView(&w);
    const dismiss = descriptor(v, .dismiss_confirmation);
    const confirm = descriptor(v, .confirm);
    try bounds(dismiss, .{ 0, -44, 0, 0 });
    try bounds(confirm, .{ 0, -44, 0, 0 });
    try t.expect(dismiss.enabled and dismiss.focusable and !dismiss.visible);
    try t.expect(std.meta.eql(l.FocusKey{ .action = .dismiss_confirmation }, l.first(v).?));
    try t.expectEqual(@as(?l.FocusKey, null), l.hit(v, .{ 0, -44 }));
}
