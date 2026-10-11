//! Allocation-free Classic geometry. Views borrow a caller-owned workspace.
const std = @import("std");
const e = @import("classic_editor.zig");
const sc = @import("scenario.zig");
pub const Rect = @import("retained_geometry.zig").Rect;
pub const LayoutError = error{ InvalidWorld, InvalidField, InvalidReport, InvalidIssuePath, InvalidFocus, InvalidScroll, ArithmeticOverflow, AliasedWorkspace, WorkspaceBusy };
pub const StatusKey = enum { header, store };
pub const Footer = enum { revert, factory_defaults, regenerate_from_seed, load, save, cancel, apply_and_restart };
pub const WidgetKey = union(enum) { status: StatusKey, section: e.Section, world_choice: u3, preview, field: e.FieldId, validation_clean, validation_issue: u8, footer: Footer, confirmation_message, dismiss_confirmation, confirm };
pub const WidgetKind = enum { status, action, preview, field, confirmation };
pub const FocusKey = union(enum) { field: e.FieldId, action: e.Action };
pub const Descriptor = struct { key: WidgetKey, kind: WidgetKind, bounds: Rect, enabled: bool, selected: bool, focusable: bool, visible: bool };
/// Move only while unborrowed. A view expires at release or owner destruction.
pub const Workspace = struct { descriptors: [114]Descriptor = undefined, borrowed: bool = false };
pub const LayoutInput = struct { viewport: [2]f32, section: e.Section, selected_world: u3, focused: ?FocusKey, scroll: f32, modal: ?e.ModalKind, can_apply: bool, has_store_status: bool, issue_count: usize };
pub const LayoutView = struct { widgets: []const Descriptor, viewport: [2]f32, wide: bool, content_rect: Rect, maximum_scroll: f32, scroll: f32 };
pub const IssueText = union(enum) { invalid_value, issue_index: u8 };
pub const ParseFlags = [90]bool;
pub const FieldError = LayoutError;
pub const Scroll = union(enum) { delta: f32, page_up, page_down, home, end };
comptime {
    if (@sizeOf(Descriptor) > 96 or @sizeOf(Workspace) - 114 * @sizeOf(Descriptor) > 16) @compileError("layout storage exceeds approved envelope");
}
fn fieldValid(id: e.FieldId) LayoutError!void {
    switch (id) {
        .world => |w| if (w.index > 6) return error.InvalidField,
        .global => {},
    }
}
fn focusValid(key: FocusKey) LayoutError!void {
    switch (key) {
        .field => |id| fieldValid(id) catch return error.InvalidFocus,
        .action => |a| switch (a) {
            .select_world => |i| if (i > 6) return error.InvalidFocus,
            else => {},
        },
    }
}
fn extent(v: f32) f32 {
    return if (std.math.isFinite(v)) @max(v, 0) else 0;
}
fn columns(width: f32, divisor: f32) usize {
    // Bound BEFORE conversion, including arbitrary finite f32 values.
    const n = @floor(width / divisor);
    if (n >= 7) return 7;
    if (n <= 1) return 1;
    return @intFromFloat(n);
}
fn rect(x: f32, y: f32, w: f32, h: f32) LayoutError!Rect {
    return Rect.fromXYWH(x, y, w, h) catch return error.ArithmeticOverflow;
}
const Sink = struct {
    destination: ?*Workspace,
    count: usize = 0,
    modal: bool,
    fn add(self: *Sink, key: WidgetKey, kind: WidgetKind, bounds: Rect, enabled: bool, selected: bool, focusable: bool, visible: bool) LayoutError!void {
        if (self.count >= 114) return error.ArithmeticOverflow;
        if (self.destination) |w| w.descriptors[self.count] = .{ .key = key, .kind = kind, .bounds = bounds, .enabled = enabled and !self.modal, .selected = selected, .focusable = focusable and !self.modal, .visible = visible };
        self.count += 1;
    }
    fn control(self: *Sink, key: WidgetKey, bounds: Rect, selected: bool) LayoutError!void {
        const width = bounds.max[0] - bounds.min[0];
        const height = bounds.max[1] - bounds.min[1];
        try self.add(key, .action, bounds, true, selected, width >= 44 and height >= 44, width > 0 and height > 0);
    }
};
const Metrics = struct { viewport: [2]f32, wide: bool, content: Rect, maximum: f32, scroll: f32 };
fn generate(in: LayoutInput, sink: *Sink) LayoutError!Metrics {
    @setFloatMode(.strict);
    const v: [2]f32 = .{ extent(in.viewport[0]), extent(in.viewport[1]) };
    const width = v[0];
    const height = v[1];
    const wide = width >= 1100;
    const cols = columns(width, 150);
    const rows = (7 + cols - 1) / cols;
    const footer_height = @min(@as(f32, @floatFromInt(rows)) * 44, height);
    const footer_top = @max(height - footer_height, @min(56, height));
    var top = @min(56, footer_top);
    try sink.add(.{ .status = .header }, .status, try rect(0, 0, width, @min(56, height)), true, false, false, true);
    if (in.has_store_status) try sink.add(.{ .status = .store }, .status, try rect(width * 0.5, 0, width * 0.5, @min(56, height)), true, false, false, true);
    var content: Rect = undefined;
    if (wide) {
        const nav_x = @min(16, width);
        const nav_w = @min(208, @max(width - nav_x, 0));
        for (std.meta.tags(e.Section), 0..) |section, i| try sink.control(.{ .section = section }, try rect(nav_x, top + @as(f32, @floatFromInt(i)) * 52, nav_w, 44), section == in.section);
        if (in.section == .worlds) for (0..7) |i| {
            try sink.control(.{ .world_choice = @intCast(i) }, try rect(nav_x, top + 164 + @as(f32, @floatFromInt(i)) * 52, nav_w, 44), i == in.selected_world);
        };
        const preview_x = nav_x + nav_w + 16;
        const form_w = @min(420, width);
        const form_x = @max(width - form_w - 16, preview_x);
        const preview_w = @max(form_x - preview_x - 16, 0);
        const h = @max(footer_top - top, 0);
        try sink.add(.preview, .preview, try rect(preview_x, top, preview_w, h), true, false, false, preview_w > 0 and h > 0);
        content = try rect(form_x, top, form_w, h);
    } else {
        const tab_w = width / 3;
        const tab_h = @min(44, @max(footer_top - top, 0));
        for (std.meta.tags(e.Section), 0..) |section, i| try sink.control(.{ .section = section }, try rect(@as(f32, @floatFromInt(i)) * tab_w, top, @max(tab_w, @min(44, width)), tab_h), section == in.section);
        top = @min(top + 52, footer_top);
        if (in.section == .worlds) {
            const world_cols = columns(width, 80);
            const world_rows = (7 + world_cols - 1) / world_cols;
            const cell_w = width / @as(f32, @floatFromInt(world_cols));
            const cell_h = @min(44, @max(footer_top - top, 0));
            for (0..7) |i| try sink.control(.{ .world_choice = @intCast(i) }, try rect(@as(f32, @floatFromInt(i % world_cols)) * cell_w, top + @as(f32, @floatFromInt(i / world_cols)) * 44, @max(cell_w, @min(44, width)), cell_h), i == in.selected_world);
            top = @min(top + @as(f32, @floatFromInt(world_rows)) * 44 + 8, footer_top);
            const preview_h = @min(88, @max(footer_top - top, 0));
            const preview_w = @max(width - 32, 0);
            try sink.add(.preview, .preview, try rect(16, top, preview_w, preview_h), true, false, false, preview_w > 0 and preview_h > 0);
            top = @min(top + preview_h + 8, footer_top);
        }
        content = try rect(@min(16, width), top, @max(width - 32, 0), @max(footer_top - top, 0));
    }
    const count: usize = switch (in.section) {
        .worlds => 11,
        .rules => 13,
        .validation => @max(in.issue_count, 1),
    };
    const content_h = content.max[1] - content.min[1];
    const maximum = @max(@as(f32, @floatFromInt(count)) * 52 - content_h, 0);
    const scroll = @min(in.scroll, maximum);
    for (0..count) |i| {
        const key: WidgetKey = switch (in.section) {
            .worlds => .{ .field = .{ .world = .{ .index = in.selected_world, .field = @fromBackingInt(@intCast(i)) } } },
            .rules => .{ .field = .{ .global = @fromBackingInt(@intCast(i)) } },
            .validation => if (in.issue_count == 0) .validation_clean else .{ .validation_issue = @intCast(i) },
        };
        const editable = in.section == .worlds or (in.section == .rules and i != 1);
        const y = content.min[1] + @as(f32, @floatFromInt(i)) * 52 - scroll;
        const bounds = try rect(content.min[0], y, content.max[0] - content.min[0], 44);
        try sink.add(key, .field, bounds, editable, false, editable, y >= content.min[1] and bounds.max[1] <= content.max[1]);
    }
    const cell_w = width / @as(f32, @floatFromInt(cols));
    for (std.meta.tags(Footer), 0..) |footer, i| try sink.add(.{ .footer = footer }, .action, try rect(@as(f32, @floatFromInt(i % cols)) * cell_w, height - footer_height + @as(f32, @floatFromInt(i / cols)) * 44, @max(cell_w, @min(44, width)), @min(44, height)), footer != .apply_and_restart or in.can_apply, false, true, true);
    if (in.modal != null) {
        sink.modal = false;
        const w = @min(600, @max(width - 32, 0));
        const h = @min(176, height);
        const x = @max((width - w) * 0.5, 0);
        const y = @max((height - h) * 0.5, 0);
        try sink.add(.confirmation_message, .confirmation, try rect(x, y, w, @max(h - 44, 0)), true, false, false, w > 0 and h > 0);
        const button_w = w * 0.5;
        try sink.add(.dismiss_confirmation, .action, try rect(x, y + h - 44, @max(button_w, @min(44, w)), @min(44, h)), true, false, true, w > 0 and h > 0);
        try sink.add(.confirm, .action, try rect(x + button_w, y + h - 44, @max(button_w, @min(44, w)), @min(44, h)), true, false, true, w > 0 and h > 0);
    }
    for ([_]f32{ maximum, scroll, content_h }) |n| if (!std.math.isFinite(n)) return error.ArithmeticOverflow;
    return .{ .viewport = v, .wide = wide, .content = content, .maximum = maximum, .scroll = scroll };
}
pub fn layout(input: LayoutInput, workspace: *Workspace) LayoutError!LayoutView {
    @setFloatMode(.strict);
    if (workspace.borrowed) return error.WorkspaceBusy;
    if (input.selected_world > 6) return error.InvalidWorld;
    if (input.issue_count > 98) return error.InvalidReport;
    if (!std.math.isFinite(input.scroll) or input.scroll < 0) return error.InvalidScroll;
    if (input.focused) |focus| try focusValid(focus);
    // No descriptor array in this pass: validate EVERY emitted endpoint/count.
    var preflight = Sink{ .destination = null, .modal = input.modal != null };
    const metrics = try generate(input, &preflight);
    // Identical deterministic scalar replay cannot reject after preflight.
    var replay = Sink{ .destination = workspace, .modal = input.modal != null };
    _ = generate(input, &replay) catch unreachable;
    workspace.borrowed = true;
    return .{ .widgets = workspace.descriptors[0..preflight.count], .viewport = metrics.viewport, .wide = metrics.wide, .content_rect = metrics.content, .maximum_scroll = metrics.maximum, .scroll = metrics.scroll };
}
/// Explicit guard survives ReleaseFast; double release is contract misuse.
pub fn releaseView(workspace: *Workspace) void {
    if (!workspace.borrowed) @panic("Classic layout view released twice or never borrowed");
    workspace.borrowed = false;
}
fn focusOf(d: Descriptor) ?FocusKey {
    return switch (d.key) {
        .field => |id| .{ .field = id },
        .section => |section| .{ .action = .{ .select_section = section } },
        .world_choice => |i| .{ .action = .{ .select_world = i } },
        .footer => |f| .{ .action = switch (f) {
            .revert => .revert,
            .factory_defaults => .factory_defaults,
            .regenerate_from_seed => .regenerate_from_seed,
            .load => .load,
            .save => .save,
            .cancel => .cancel,
            .apply_and_restart => .apply_and_restart,
        } },
        .dismiss_confirmation => .{ .action = .dismiss_confirmation },
        .confirm => .{ .action = .confirm },
        else => null,
    };
}
fn eligible(d: Descriptor) ?FocusKey {
    return if (d.enabled and d.focusable) focusOf(d) else null;
}
pub fn first(view: LayoutView) ?FocusKey {
    for (view.widgets) |d| if (eligible(d)) |key| return key;
    return null;
}
pub fn next(view: LayoutView, current: ?FocusKey, reverse: bool) ?FocusKey {
    var current_index: ?usize = null;
    if (current) |key| for (view.widgets, 0..) |d, i| {
        if (eligible(d)) |other| if (std.meta.eql(key, other)) {
            current_index = i;
            break;
        };
    };
    // No temporary focus array. Scan at most the bounded descriptor population.
    const n = view.widgets.len;
    if (n == 0) return null;
    var index = if (current_index) |i| i else if (reverse) @as(usize, 0) else n - 1;
    for (0..n) |_| {
        index = if (reverse) (if (index == 0) n - 1 else index - 1) else (if (index == n - 1) 0 else index + 1);
        if (eligible(view.widgets[index])) |key| return key;
    }
    return null;
}
pub fn hit(view: LayoutView, point: [2]f32) ?FocusKey {
    @setFloatMode(.strict);
    for (point) |v| if (!std.math.isFinite(v)) return null;
    var i = view.widgets.len;
    while (i > 0) {
        i -= 1;
        const d = view.widgets[i];
        if (d.visible and d.bounds.contains(point)) if (eligible(d)) |key| return key;
    }
    return null;
}
pub fn actionAt(view: LayoutView, key: FocusKey) ?e.Action {
    for (view.widgets) |d| if (d.kind == .action) if (eligible(d)) |actual| {
        if (std.meta.eql(actual, key)) return switch (actual) {
            .action => |a| a,
            .field => null,
        };
    };
    return null;
}
pub fn reveal(view: LayoutView, key: FocusKey) LayoutError!f32 {
    @setFloatMode(.strict);
    switch (key) {
        .action => return view.scroll,
        .field => {},
    }
    for (view.widgets) |d| if (d.key == .field) {
        const other = focusOf(d).?;
        if (!std.meta.eql(key, other)) continue;
        if (d.bounds.min[1] < view.content_rect.min[1]) {
            const distance = view.content_rect.min[1] - d.bounds.min[1];
            const value = view.scroll - distance;
            if (!std.math.isFinite(distance) or !std.math.isFinite(value)) return error.InvalidScroll;
            return @max(value, 0);
        }
        if (d.bounds.max[1] > view.content_rect.max[1]) {
            const sum = view.scroll + d.bounds.min[1];
            const height = d.bounds.max[1] - d.bounds.min[1];
            const bottom = sum + height;
            const value = bottom - view.content_rect.max[1];
            if (!std.math.isFinite(sum) or !std.math.isFinite(height) or !std.math.isFinite(bottom) or !std.math.isFinite(value)) return error.InvalidScroll;
            return @min(value, view.maximum_scroll);
        }
        return view.scroll;
    };
    return view.scroll;
}
pub fn scrollTo(view: LayoutView, operation: Scroll) LayoutError!f32 {
    @setFloatMode(.strict);
    const delta: f32 = switch (operation) {
        .home => return 0,
        .end => return view.maximum_scroll,
        .page_up => -@max(view.viewport[1], 44),
        .page_down => @max(view.viewport[1], 44),
        .delta => |d| if (std.math.isFinite(d)) d else return view.scroll,
    };
    const value = view.scroll + delta;
    if (!std.math.isFinite(value)) return error.InvalidScroll;
    return std.math.clamp(value, 0, view.maximum_scroll);
}
const Schema = union(enum) { global: e.GlobalField, world: struct { suffix: []const u8, field: e.WorldField }, aggregate };
fn schema(code: sc.ScenarioIssueCode) Schema {
    return switch (code) {
        .tick_rate => .{ .global = .tick_hz },
        .win_world_count => .{ .global = .win_world_count },
        .ai_period => .{ .global = .ai_period_ticks },
        .hazard_period => .{ .global = .hazard_period_ticks },
        .hazard_duration => .{ .global = .hazard_duration_ticks },
        .hazard_boundary_arithmetic => .{ .global = .hazard_first_tick },
        .energy_cap => .{ .global = .maximum_energy },
        .defense_cap => .{ .global = .maximum_defense },
        .minimum_launch => .{ .global = .minimum_launch },
        .field_cost_relation => .{ .global = .field_raise_cost },
        .fleet_speed_arithmetic => .{ .global = .base_fleet_speed },
        .world_name, .duplicate_name => .{ .world = .{ .suffix = "name", .field = .name } },
        .world_coordinate, .duplicate_position, .close_position => .{ .world = .{ .suffix = "position", .field = .x } },
        .world_field_level => .{ .world = .{ .suffix = "fields", .field = .atmosphere } },
        .world_energy => .{ .world = .{ .suffix = "energy", .field = .energy } },
        .world_defense => .{ .world = .{ .suffix = "defense", .field = .defense } },
        .production_arithmetic => .{ .world = .{ .suffix = "base_output", .field = .base_output } },
        .regeneration_arithmetic => .{ .world = .{ .suffix = "base_regeneration", .field = .base_regeneration } },
        .missing_union, .initial_victory, .missing_helix, .missing_choir, .no_launchable_union => .aggregate,
    };
}
fn expectedSeverity(code: sc.ScenarioIssueCode) sc.IssueSeverity {
    return switch (code) {
        .duplicate_name, .close_position, .missing_helix, .missing_choir, .no_launchable_union => .warning,
        else => .err,
    };
}
fn mapped(issue: *const sc.ScenarioIssue) LayoutError!?e.FieldId {
    const path = issue.field.asSlice();
    switch (schema(issue.code)) {
        .aggregate => {
            if (!std.mem.eql(u8, path, "worlds")) return error.InvalidIssuePath;
            return null;
        },
        .global => |g| {
            const suffix = if (std.mem.startsWith(u8, path, "rules.")) path[6..] else return error.InvalidIssuePath;
            if (!std.mem.eql(u8, suffix, @tagName(g))) return error.InvalidIssuePath;
            return .{ .global = g };
        },
        .world => |w| {
            if (path.len < 10 or !std.mem.startsWith(u8, path, "worlds[") or path[7] < '0' or path[7] > '6' or path[8] != ']' or path[9] != '.') return error.InvalidIssuePath;
            if (!std.mem.eql(u8, path[10..], w.suffix)) return error.InvalidIssuePath;
            return .{ .world = .{ .index = @intCast(path[7] - '0'), .field = w.field } };
        },
    }
}
pub fn validateReport(report: *const sc.ValidationReport) LayoutError!void {
    if (report.len > 98 or report.len > report.entries.len) return error.InvalidReport;
    for (report.entries[0..report.len]) |*issue| {
        // Public Text lengths are untrusted. Check physical capacity before ANY slice.
        if (issue.field.len > issue.field.bytes.len or issue.message.len > issue.message.bytes.len) return error.InvalidReport;
        if (issue.field.len > 27 or issue.message.len > 73) return error.InvalidReport;
        if (!std.unicode.utf8ValidateSlice(issue.field.asSlice()) or !std.unicode.utf8ValidateSlice(issue.message.asSlice())) return error.InvalidReport;
        if (issue.severity != expectedSeverity(issue.code)) return error.InvalidReport;
        _ = try mapped(issue);
    }
}
pub fn mapIssueField(report: *const sc.ValidationReport, index: usize) LayoutError!?e.FieldId {
    try validateReport(report);
    if (index >= report.len) return error.InvalidReport;
    return mapped(&report.entries[index]);
}
pub fn fieldError(report: *const sc.ValidationReport, flags: *const ParseFlags, id: e.FieldId) FieldError!?IssueText {
    try fieldValid(id);
    try validateReport(report);
    if (flags[id.slot()]) return .invalid_value;
    for (report.entries[0..report.len], 0..) |*issue, i| {
        if (issue.severity != .err) continue;
        if (try mapped(issue)) |field| if (std.meta.eql(field, id)) return .{ .issue_index = @intCast(i) };
    }
    return null;
}
/// The returned message borrows the ORIGINAL immutable report, never a copied issue.
pub fn issueText(report: *const sc.ValidationReport, text: IssueText) LayoutError![]const u8 {
    try validateReport(report);
    return switch (text) {
        .invalid_value => "INVALID VALUE",
        .issue_index => |i| if (i < report.len) report.entries[i].message.asSlice() else error.InvalidReport,
    };
}
const Range = struct { start: usize, end: usize };
fn range(pointer: anytype) LayoutError!Range {
    const start = @intFromPtr(pointer);
    const end = std.math.add(usize, start, @sizeOf(@typeInfo(@TypeOf(pointer)).pointer.child)) catch return error.ArithmeticOverflow;
    return .{ .start = start, .end = end };
}
fn overlap(a: Range, b: Range) bool {
    return a.start < b.end and b.start < a.end;
}
/// Must precede integration layout writes while report and parse flags are live.
/// Read/read report-flags aliasing is permitted; both full Workspace ranges reject.
pub fn validateWorkspaceBorrowing(workspace: *const Workspace, report: *const sc.ValidationReport, flags: *const ParseFlags) LayoutError!void {
    const workspace_range = try range(workspace);
    const report_range = try range(report);
    const flags_range = try range(flags);
    if (overlap(workspace_range, report_range) or overlap(workspace_range, flags_range)) return error.AliasedWorkspace;
}
