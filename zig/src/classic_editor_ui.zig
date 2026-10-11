//! Pure retained Classic editor projection; no transaction or host effects.
const std = @import("std");
const editor_model = @import("classic_editor.zig");
pub const Target = union(enum) { field: editor_model.FieldId, action: editor_model.Action };
pub const Identity = struct { id: []const u8, target: Target, editable: bool };
pub const catalog = blk: {
    var result: [109]Identity = undefined;
    for (editor_model.catalog, 0..) |field, i| result[i] = .{ .id = "a/editor/" ++ field.text_id, .target = .{ .field = field.id }, .editable = field.editable };
    for (std.meta.tags(editor_model.Section), 0..) |section, i| result[90 + i] = .{ .id = "a/editor/section." ++ @tagName(section), .target = .{ .action = .{ .select_section = section } }, .editable = true };
    for (0..7) |i| result[93 + i] = .{ .id = std.fmt.comptimePrint("a/editor/world.select.{d}", .{i}), .target = .{ .action = .{ .select_world = i } }, .editable = true };
    const keys = .{ "revert", "factory_defaults", "regenerate_from_seed", "load", "save", "cancel", "apply_and_restart" };
    const actions = [_]editor_model.Action{ .revert, .factory_defaults, .regenerate_from_seed, .load, .save, .cancel, .apply_and_restart };
    for (keys, actions, 0..) |key, action, i| result[100 + i] = .{ .id = "a/editor/footer." ++ key, .target = .{ .action = action }, .editable = true };
    result[107] = .{ .id = "a/editor/confirmation.dismiss", .target = .{ .action = .dismiss_confirmation }, .editable = true };
    result[108] = .{ .id = "a/editor/confirmation.confirm", .target = .{ .action = .confirm }, .editable = true };
    break :blk result;
};
const H1 = @import("retained_semantics.zig");
const layout = @import("classic_editor_layout.zig");
const scenario = @import("scenario.zig");
const transactions = @import("classic_editor_transactions.zig");
const session_model = @import("classic_session.zig");
const Allocator = std.mem.Allocator;
pub const CurrentStamp = struct { revision: u64, owner: H1.OwnerSpec };
pub const SavedBaseFocus = struct { order: []const layout.FocusKey, restore: ?layout.FocusKey };
pub const FocusInput = struct { current: ?layout.FocusKey, saved_base: ?SavedBaseFocus = null };
pub const Source = struct {
    editor: *const editor_model.EditorState,
    report: *const scenario.ValidationReport,
    parse_flags: *const layout.ParseFlags,
    viewport: [2]f32,
    store_status: *const transactions.Status,
    action_failure: ?*const transactions.Failure,
    focus: FocusInput,
    raw_status: ?[]const u8 = null,
    raw_action_error: ?[]const u8 = null,
    recovery: ?session_model.RecoveryFailure = null,
    raw_recovery: ?[]const u8 = null,
};
pub const PrepareError = H1.FrameError || layout.LayoutError || error{ NoEditor, WrongProjectionMode, InvalidOwner, InvalidSource, InputLimit, TextLimit, SnapshotLimit, InvalidRecoveryRecord };
pub const PublishError = H1.FrameError || error{ EmptyProjection, BudgetMismatch, StaleProjection };
pub const ActionError = error{ InvalidUtf8, StaleProjection, UnknownAction, DisabledAction, InactiveModal, InvalidSource };
pub const limits = H1.Limits{ .max_nodes = 480, .max_child_indices = 479, .max_id_bytes = 49152, .max_text_bytes = 262144, .max_focus_actions = 64, .max_announcements = 8, .max_owner_context_bytes = 128, .max_owned_bytes = 442368 };
pub const EditorGeometryView = struct { revision: u64, viewport: [2]f32, section: editor_model.Section, selected_world: u3, modal: ?editor_model.ModalKind, content_rect: layout.Rect, maximum_scroll: f32, scroll: f32, remembered_focus: ?layout.FocusKey };
const Metadata = struct {
    allocator: Allocator,
    budget: *H1.PreparationBudget,
    charge: H1.Charge,
    resident: H1.TokenRef,
    draft: H1.TokenRef,
    resident_retained: bool = false,
    draft_retained: bool = false,
    revision: u64,
    hold: ?u64,
    context_len: usize = 0,
    node_count: usize = 0,
    child_count: usize = 0,
    focus_count: usize = 0,
    announcement_count: usize = 0,
    used: usize = 0,
    base_count: usize = 0,
    modal_count: usize = 0,
    focused: ?H1.ActionId = null,
    restore: ?H1.ActionId = null,
    remembered_focus: ?layout.FocusKey = null,
    historical_restore: ?layout.FocusKey = null,
    viewport: [2]f32,
    content_rect: layout.Rect,
    maximum_scroll: f32,
    scroll: f32,
    section: editor_model.Section,
    selected_world: u3,
    modal: ?editor_model.ModalKind,
};
const Header = struct { m: Metadata, nodes: [480]H1.NodeSpec = undefined, children: [479]usize = undefined, focus: [32]H1.ActionId = undefined, announcements: [8]H1.AnnouncementSpec = undefined, context: [128]u8 = undefined, arena: [196608]u8 = undefined };
pub const HEADER_BYTES = @sizeOf(Header);
pub const METADATA_BYTES = @sizeOf(Metadata);
comptime {
    if (HEADER_BYTES > 294912 or METADATA_BYTES > 4096 or @sizeOf(PreparedEditor) != 8) @compileError("H3b target layout exceeds accepted caps");
}
/// Move-only. input/geometry borrow stable Header storage until deinit.
/// Source/backing leases end on prepare return; budget and tokens outlive owners.
pub const PreparedEditor = struct {
    header: ?*Header = null,
    pub fn deinit(self: *PreparedEditor) void {
        const h = self.header orelse return;
        const allocator = h.m.allocator;
        var charge = h.m.charge;
        if (h.m.draft_retained) h.m.draft.release();
        if (h.m.resident_retained) h.m.resident.release();
        allocator.destroy(h);
        charge.release();
        self.header = null;
    }
    pub fn input(self: *const PreparedEditor) error{EmptyProjection}!H1.FrameInput {
        const h = self.header orelse return error.EmptyProjection;
        return .{ .nodes = h.nodes[0..h.m.node_count], .child_indices = h.children[0..h.m.child_count], .announcements = h.announcements[0..h.m.announcement_count], .owner = .{ .resident = h.m.resident, .draft = h.m.draft, .hold = h.m.hold, .context = h.context[0..h.m.context_len] }, .focus = .{ .base = h.focus[0..h.m.base_count], .modal = if (h.m.modal != null) h.focus[h.m.base_count..h.m.focus_count] else null, .focused = h.m.focused, .restore = if (h.m.modal != null) h.m.restore else null } };
    }
    pub fn geometry(self: *const PreparedEditor) error{EmptyProjection}!EditorGeometryView {
        const m = &(self.header orelse return error.EmptyProjection).m;
        return .{ .revision = m.revision, .viewport = m.viewport, .section = m.section, .selected_world = m.selected_world, .modal = m.modal, .content_rect = m.content_rect, .maximum_scroll = m.maximum_scroll, .scroll = m.scroll, .remembered_focus = m.remembered_focus };
    }
};
fn checked(a: usize, b: usize) PrepareError!usize {
    return std.math.add(usize, a, b) catch error.InputLimit;
}
fn valid(text: []const u8) PrepareError!void {
    if (!std.unicode.utf8ValidateSlice(text)) return error.InvalidUtf8;
}
fn keyIndex(key: layout.FocusKey) PrepareError!usize {
    switch (key) {
        .field => |field| switch (field) {
            .world => |world| if (world.index > 6) return error.InvalidFocus,
            else => {},
        },
        .action => |a| switch (a) {
            .select_world => |i| if (i > 6) return error.InvalidFocus,
            else => {},
        },
    }
    const target: Target = switch (key) {
        .field => |f| .{ .field = f },
        .action => |a| .{ .action = a },
    };
    for (catalog, 0..) |entry, i| if (std.meta.eql(entry.target, target)) return i;
    return error.InvalidFocus;
}
fn sourceFocus(editor: *const editor_model.EditorState) PrepareError!?layout.FocusKey {
    const key: ?layout.FocusKey = if (editor.focus) |f| switch (f) {
        .field => |id| .{ .field = id },
        .action => |a| .{ .action = a },
    } else null;
    if (key) |k| _ = try keyIndex(k);
    return key;
}
fn descriptorIndex(d: layout.Descriptor) ?usize {
    return switch (d.key) {
        .field => |f| f.slot(),
        .section => |section| 90 + @as(usize, @backingInt(section)),
        .world_choice => |i| 93 + @as(usize, i),
        .footer => |f| 100 + @as(usize, @backingInt(f)),
        .dismiss_confirmation => 107,
        .confirm => 108,
        else => null,
    };
}
fn nodeId(index: usize, comptime suffix: []const u8) []const u8 {
    inline for (catalog, 0..) |entry, i| if (index == i) return "c/editor/" ++ entry.id[9..] ++ "/" ++ suffix;
    unreachable;
}
const Lane = struct {
    bytes: []const u8,
    len: usize,
    fn literal(bytes: []const u8) Lane {
        return .{ .bytes = bytes, .len = bytes.len };
    }
    fn component(self: Lane, start: usize, len: usize) PrepareError!Lane {
        if (start > self.len or len > self.len - start) return error.InvalidSource;
        return .{ .bytes = if (self.bytes.len == self.len) self.bytes[start..][0..len] else "", .len = len };
    }
};
const Sink = struct {
    h: ?*Header,
    nodes: usize = 0,
    unique: usize = 0,
    text: usize = 0,
    ids: usize = 0,
    fn copy(self: *Sink, parts: []const []const u8, uppercase: bool) PrepareError!Lane {
        var length: usize = 0;
        for (parts) |part| {
            try valid(part);
            length = try checked(length, part.len);
        }
        const start = self.unique;
        self.unique = try checked(start, length);
        if (self.unique > 196608) return error.SnapshotLimit;
        if (self.h) |h| {
            var offset = start;
            for (parts) |part| {
                for (part) |byte| {
                    h.arena[offset] = if (uppercase) std.ascii.toUpper(byte) else byte;
                    offset += 1;
                }
            }
            return .{ .bytes = h.arena[start..self.unique], .len = length };
        }
        return .{ .bytes = "", .len = length };
    }
    fn node(self: *Sink, id: []const u8, role: H1.NodeRole, name: Lane, description: Lane, value: Lane, parent: ?usize, d: ?layout.Descriptor, enabled: bool, action: ?H1.ActionId) PrepareError!usize {
        if (self.nodes == 480) return error.InputLimit;
        const index = self.nodes;
        self.nodes += 1;
        self.ids = try checked(self.ids, id.len);
        if (action) |a| self.ids = try checked(self.ids, a.bytes.len);
        self.text = try checked(self.text, try checked(name.len, try checked(description.len, value.len)));
        if (self.text > 262144) return error.TextLimit;
        if (self.ids > 49152) return error.InputLimit;
        if (self.h) |h| h.nodes[index] = .{ .id = .{ .bytes = id }, .role = role, .name = name.bytes, .description = description.bytes, .value = value.bytes, .enabled = if (d) |descriptor| descriptor.enabled else enabled, .selected = if (d) |descriptor| descriptor.selected else false, .focusable = if (d) |descriptor| descriptor.focusable else false, .hierarchy_level = null, .expanded = null, .action = action, .bounds = if (d) |descriptor| descriptor.bounds else null, .visible = if (d) |descriptor| descriptor.visible else true, .parent = parent, .child_start = 0, .child_count = 0 };
        return index;
    }
    fn container(self: *Sink, id: []const u8, role: H1.NodeRole, name: []const u8, parent: ?usize, enabled: bool) PrepareError!usize {
        return self.node(id, role, Lane.literal(name), Lane.literal(""), Lane.literal(""), parent, null, enabled, null);
    }
};
fn build(sink: *Sink, source: Source, view: layout.LayoutView) PrepareError!void {
    const enabled = source.editor.modal.kind == null;
    const root = try sink.container("c/editor/root", .application, "NYON", null, enabled);
    const dialog = try sink.container("c/editor/dialog", .region, "SCENARIO EDITOR", root, enabled);
    const groups = [_]Group{ .status, .navigation, .preview, .content, .footer, .confirmation };
    var attached_preedit = false;
    var scratch: [512]u8 = undefined;
    for (groups) |group| {
        // Recovery precedes the overlay, after all complete underlay groups.
        if (group == .confirmation) {
            const has_action = source.action_failure != null or source.raw_action_error != null;
            const has_recovery = source.recovery != null or source.raw_recovery != null;
            if (has_action or has_recovery) {
                const region = try sink.container("c/editor/recovery_region", .region, "", dialog, enabled);
                if (has_action) {
                    const category = if (source.action_failure) |failure| try actionText(failure, &scratch) else "";
                    try alert(sink, "action_failure", source.raw_action_error orelse category, region, enabled);
                }
                if (has_recovery) {
                    const category = if (source.recovery) |failure| try recoveryText(failure, &scratch) else "";
                    try alert(sink, "recovery", source.raw_recovery orelse category, region, enabled);
                }
            }
        }
        var present = false;
        for (view.widgets) |d| if (groupOf(d.key) == group) {
            present = true;
            break;
        };
        if (!present) continue;
        const parent = try sink.container(groupId(group), if (group == .footer) .toolbar else if (group == .confirmation) .dialog else .region, if (group == .confirmation) "CONFIRM ACTION" else "", dialog, enabled or group == .confirmation);
        const widgets_parent = if (group == .content and source.editor.section == .validation) try sink.container("c/editor/validation_list", .list, "", parent, enabled) else parent;
        for (view.widgets) |d| if (groupOf(d.key) == group) {
            try widget(sink, source, d, widgets_parent, &scratch, &attached_preedit);
        };
        if (group == .content and !attached_preedit and source.editor.preedit().len != 0) {
            const value = try sink.copy(&.{source.editor.preedit()}, false);
            _ = try sink.node("c/editor/preedit", .status, Lane.literal("IME PREEDIT"), Lane.literal(""), value, parent, null, enabled, null);
        }
    }
}
/// Synchronous immutable source lease, including every pointed-to backing.
/// One admitted Header and two token reference slots; no epoch mutation in flight.
pub fn prepareEditor(allocator: Allocator, budget: *H1.PreparationBudget, workspace: *layout.Workspace, source: Source, current: CurrentStamp) PrepareError!PreparedEditor {
    const draft = current.owner.draft orelse return error.InvalidOwner;
    if (current.owner.context.len > 128) return error.InvalidOwner;
    try valid(current.owner.context);
    try layout.validateWorkspaceBorrowing(workspace, source.report, source.parse_flags);
    try validateSourceBorrowing(workspace, source, current.owner.context);
    try layout.validateReport(source.report);
    if (source.editor.selected_world > 6) return error.InvalidWorld;
    const actual = try sourceFocus(source.editor);
    if (source.focus.current) |key| _ = try keyIndex(key);
    if (source.editor.modal.kind == null) {
        if (!std.meta.eql(actual, source.focus.current)) return error.InvalidSource;
    } else {
        if (source.focus.current) |key| {
            const i = try keyIndex(key);
            if (i < 107) return error.InvalidFocus;
        }
        const saved = source.focus.saved_base orelse return error.InvalidSource;
        if (saved.order.len > 28) return error.InputLimit;
        for (saved.order, 0..) |key, i| {
            if (try keyIndex(key) >= 107) return error.InvalidFocus;
            for (saved.order[0..i]) |other| if (std.meta.eql(key, other)) return error.InvalidFocus;
        }
        if (saved.restore) |key| _ = try keyIndex(key);
    }
    if (source.focus.saved_base) |saved| {
        if (saved.restore) |key| _ = try keyIndex(key);
    }
    for (editor_model.catalog) |field| try valid(source.editor.fieldText(field.text_id) orelse return error.InvalidSource);
    try valid(source.editor.preedit());
    const current_draft = source.editor.current();
    for (current_draft.worlds) |world| try valid(world.name);
    const view = try layout.layout(.{ .viewport = source.viewport, .section = source.editor.section, .selected_world = @intCast(source.editor.selected_world), .focused = source.focus.current, .scroll = source.editor.scroll, .modal = source.editor.modal.kind, .can_apply = !source.editor.hasParseErrors() and source.report.isValid(), .has_store_status = source.store_status.* != .none or source.raw_status != null, .issue_count = source.report.len }, workspace);
    defer layout.releaseView(workspace);
    var preflight = Sink{ .h = null };
    try build(&preflight, source, view);
    if (try checked(preflight.ids, try focusByteCount(source, view)) > 49152) return error.InputLimit;
    var reservation = try budget.reserve(.frame_scratch, HEADER_BYTES);
    const h = allocator.create(Header) catch {
        reservation.cancel();
        return error.OutOfMemory;
    };
    const charge = reservation.commit();
    h.* = .{ .m = .{ .allocator = allocator, .budget = budget, .charge = charge, .resident = current.owner.resident, .draft = draft, .revision = current.revision, .hold = current.owner.hold, .viewport = view.viewport, .content_rect = view.content_rect, .maximum_scroll = view.maximum_scroll, .scroll = view.scroll, .section = source.editor.section, .selected_world = @intCast(source.editor.selected_world), .modal = source.editor.modal.kind, .remembered_focus = source.focus.current } };
    var result = PreparedEditor{ .header = h };
    errdefer result.deinit();
    h.m.context_len = current.owner.context.len;
    @memcpy(h.context[0..h.m.context_len], current.owner.context);
    var replay = Sink{ .h = h };
    try build(&replay, source, view);
    h.m.node_count = replay.nodes;
    h.m.used = replay.unique;
    for (h.nodes[0..h.m.node_count], 0..) |*parent, i| {
        parent.child_start = h.m.child_count;
        for (h.nodes[0..h.m.node_count], 0..) |child, j| if (child.parent == i) {
            if (h.m.child_count == h.children.len) return error.InputLimit;
            h.children[h.m.child_count] = j;
            h.m.child_count += 1;
            parent.child_count += 1;
        };
    }
    if (source.focus.saved_base) |saved| {
        h.m.historical_restore = saved.restore;
        if (saved.restore) |key| h.m.restore = .{ .bytes = catalog[try keyIndex(key)].id };
    }
    if (source.editor.modal.kind != null) {
        for (source.focus.saved_base.?.order) |key| {
            h.focus[h.m.focus_count] = .{ .bytes = catalog[try keyIndex(key)].id };
            h.m.focus_count += 1;
        }
        h.m.base_count = h.m.focus_count;
    }
    for (view.widgets) |descriptor| if (descriptor.enabled and descriptor.focusable) {
        if (descriptorIndex(descriptor)) |i| {
            if (h.m.focus_count >= h.focus.len) return error.InputLimit;
            h.focus[h.m.focus_count] = .{ .bytes = catalog[i].id };
            h.m.focus_count += 1;
            if (source.focus.current) |key| if (try keyIndex(key) == i) {
                h.m.focused = .{ .bytes = catalog[i].id };
            };
        }
    };
    if (source.editor.modal.kind == null) h.m.base_count = h.m.focus_count else h.m.modal_count = h.m.focus_count - h.m.base_count;
    _ = try h.m.resident.retain();
    h.m.resident_retained = true;
    _ = try h.m.draft.retain();
    h.m.draft_retained = true;
    return result;
}
pub fn publishEditor(slot: *H1.FrameSlot, published_revision: *?u64, prepared: *const PreparedEditor, current: CurrentStamp) PublishError!void {
    const h = prepared.header orelse return error.EmptyProjection;
    if (h.m.budget != slot.budget) return error.BudgetMismatch;
    const input = try prepared.input();
    if (h.m.revision != current.revision or !input.owner.matches(current.owner)) return error.StaleProjection;
    try slot.replace(input, limits);
    published_revision.* = current.revision;
}
const Group = enum { status, navigation, preview, content, footer, confirmation };
fn groupOf(key: layout.WidgetKey) Group {
    return switch (key) {
        .status => .status,
        .section, .world_choice => .navigation,
        .preview => .preview,
        .field, .validation_clean, .validation_issue => .content,
        .footer => .footer,
        .confirmation_message, .dismiss_confirmation, .confirm => .confirmation,
    };
}
fn groupId(group: Group) []const u8 {
    return switch (group) {
        .status => "c/editor/status_region",
        .navigation => "c/editor/navigation",
        .preview => "c/editor/preview_region",
        .content => "c/editor/content",
        .footer => "c/editor/footer_region",
        .confirmation => "c/editor/confirmation_dialog",
    };
}
const global_labels = [_][]const u8{ "SEED", "TICK RATE", "WIN WORLDS", "AI PERIOD", "HAZARD FIRST", "HAZARD PERIOD", "HAZARD DURATION", "MAX ENERGY", "MAX DEFENSE", "MIN LAUNCH", "RAISE COST", "LOWER REFUND", "BASE SPEED" };
const world_labels = [_][]const u8{ "NAME", "X", "Y", "OWNER", "ENERGY", "DEFENSE", "BASE OUTPUT", "BASE REGEN", "ATMOSPHERE", "HYDROSPHERE", "TOPOLOGY" };
const section_labels = [_][]const u8{ "WORLDS", "RULES", "VALIDATION" };
const footer_labels = [_][]const u8{ "REVERT", "FACTORY DEFAULTS", "REGENERATE FROM SEED", "LOAD", "SAVE", "CANCEL", "APPLY AND RESTART" };
const modal_messages = [_][]const u8{ "DISCARD DRAFT CHANGES AND REVERT?", "REPLACE THE DRAFT WITH FACTORY DEFAULTS?", "REPLACE ALL WORLD EDITS FROM THE SEED?", "DISCARD DRAFT CHANGES AND LOAD THE SAVED SLOT?", "REPLACE THE OCCUPIED SAVE SLOT?", "DISCARD DRAFT CHANGES AND CLOSE THE EDITOR?", "DISCARD THE RUNNING SESSION AND START THIS SCENARIO?" };
fn issueCode(code: scenario.ScenarioIssueCode) []const u8 {
    return switch (code) {
        .tick_rate => "TICKRATE",
        .win_world_count => "WINWORLDCOUNT",
        .ai_period => "AIPERIOD",
        .hazard_period => "HAZARDPERIOD",
        .hazard_duration => "HAZARDDURATION",
        .hazard_boundary_arithmetic => "HAZARDBOUNDARYARITHMETIC",
        .energy_cap => "ENERGYCAP",
        .defense_cap => "DEFENSECAP",
        .minimum_launch => "MINIMUMLAUNCH",
        .field_cost_relation => "FIELDCOSTRELATION",
        .missing_union => "MISSINGUNION",
        .initial_victory => "INITIALVICTORY",
        .missing_helix => "MISSINGHELIX",
        .missing_choir => "MISSINGCHOIR",
        .no_launchable_union => "NOLAUNCHABLEUNION",
        .fleet_speed_arithmetic => "FLEETSPEEDARITHMETIC",
        .world_name => "WORLDNAME",
        .world_coordinate => "WORLDCOORDINATE",
        .world_field_level => "WORLDFIELDLEVEL",
        .world_energy => "WORLDENERGY",
        .world_defense => "WORLDDEFENSE",
        .production_arithmetic => "PRODUCTIONARITHMETIC",
        .regeneration_arithmetic => "REGENERATIONARITHMETIC",
        .duplicate_position => "DUPLICATEPOSITION",
        .duplicate_name => "DUPLICATENAME",
        .close_position => "CLOSEPOSITION",
    };
}
fn nonactionId(key: layout.WidgetKey, comptime suffix: []const u8) []const u8 {
    return switch (key) {
        .status => |status| switch (status) {
            .header => "c/editor/status.header/" ++ suffix,
            .store => "c/editor/status.store/" ++ suffix,
        },
        .preview => "c/editor/preview.world/" ++ suffix,
        .validation_clean => "c/editor/validation.clean/" ++ suffix,
        .validation_issue => |index| blk: {
            inline for (0..98) |i| if (index == i) break :blk std.fmt.comptimePrint("c/editor/validation.{d}/" ++ suffix, .{i});
            unreachable;
        },
        .confirmation_message => "c/editor/confirmation.message/" ++ suffix,
        else => unreachable,
    };
}
fn widgetId(key: layout.WidgetKey, comptime suffix: []const u8) []const u8 {
    const d = layout.Descriptor{ .key = key, .kind = .status, .bounds = undefined, .enabled = false, .selected = false, .focusable = false, .visible = false };
    return if (descriptorIndex(d)) |i| nodeId(i, suffix) else nonactionId(key, suffix);
}
fn fieldRole(id: editor_model.FieldId) H1.NodeRole {
    return switch (id) {
        .global => |f| if (f == .tick_hz) .text else .spin_button,
        .world => |w| switch (w.field) {
            .name => .text_input,
            .owner => .option,
            else => .spin_button,
        },
    };
}
fn formatted(scratch: *[512]u8, comptime format: []const u8, args: anytype) PrepareError![]const u8 {
    return std.fmt.bufPrint(scratch, format, args) catch error.InvalidSource;
}
fn widget(sink: *Sink, source: Source, d: layout.Descriptor, parent: usize, scratch: *[512]u8, attached_preedit: *bool) PrepareError!void {
    var name = Lane.literal("");
    var value = Lane.literal("");
    var description = Lane.literal("");
    var role: H1.NodeRole = .text;
    switch (d.key) {
        .status => |status| {
            role = .status;
            if (status == .header) {
                name = Lane.literal("SCENARIO EDITOR");
                value = try sink.copy(&.{try formatted(scratch, "{s} | {d} ERRORS | {d} WARNINGS", .{ section_labels[@intCast(@backingInt(source.editor.section))], source.report.errorCount(), source.report.warningCount() })}, false);
            } else {
                name = Lane.literal("STORAGE");
                const category = try statusText(source.store_status, scratch);
                value = try sink.copy(&.{source.raw_status orelse category}, false);
            }
        },
        .section => |section| {
            role = .button;
            name = Lane.literal(section_labels[@intCast(@backingInt(section))]);
            value = Lane.literal(if (d.selected) "SELECTED" else "");
        },
        .world_choice => |index| {
            role = .option;
            inline for (0..7) |i| if (index == i) {
                name = Lane.literal(std.fmt.comptimePrint("WORLD {d}", .{i}));
            };
            const draft = source.editor.current();
            value = try sink.copy(&.{ draft.worlds[index].name, if (d.selected) " | SELECTED" else "" }, false);
        },
        .preview => {
            const draft = source.editor.current();
            const index = source.editor.selected_world;
            const world = draft.worlds[index];
            const prefix = try formatted(scratch, "TACTICAL PREVIEW | WORLD {d} | ", .{index});
            name = try sink.copy(&.{ prefix, world.name }, false);
            const owner = if (world.owner) |f| switch (f) {
                .union_faction => "UNION",
                .helix => "HELIX",
                .choir => "CHOIR",
            } else "NEUTRAL";
            const numeric = try formatted(scratch, "{s} | X {d} Y {d} | ENERGY {d} | DEFENSE {d} | FIELDS {d}/{d}/{d}", .{ owner, world.x, world.y, world.energy, world.defense, world.atmosphere, world.hydrosphere, world.topology });
            value = try sink.copy(&.{numeric}, false);
        },
        .field => |id| {
            role = fieldRole(id);
            name = Lane.literal(switch (id) {
                .global => |f| global_labels[@intCast(@backingInt(f))],
                .world => |w| world_labels[@intCast(@backingInt(w.field))],
            });
            const raw = source.editor.fieldText(editor_model.catalog[id.slot()].text_id) orelse return error.InvalidSource;
            const actual = try sourceFocus(source.editor);
            var suffix: []const u8 = "";
            if (actual) |focus| switch (focus) {
                .field => |focused| if (std.meta.eql(id, focused)) {
                    suffix = source.editor.preedit();
                    attached_preedit.* = true;
                },
                .action => {},
            };
            value = try sink.copy(&.{ raw, suffix }, false);
            if (try layout.fieldError(source.report, source.parse_flags, id)) |issue| description = try sink.copy(&.{try layout.issueText(source.report, issue)}, false);
        },
        .validation_clean => {
            role = .status;
            name = Lane.literal("VALIDATION");
            value = Lane.literal("NO ERRORS OR WARNINGS");
        },
        .validation_issue => |i| {
            const issue = &source.report.entries[i];
            role = .list_item;
            name = Lane.literal(issueCode(issue.code));
            description = try sink.copy(&.{issue.field.asSlice()}, false);
            const severity = if (issue.severity == .err) "ERROR | " else "WARNING | ";
            value = try sink.copy(&.{ severity, issue.message.asSlice() }, true);
        },
        .footer => |footer| {
            role = .button;
            name = Lane.literal(footer_labels[@intCast(@backingInt(footer))]);
        },
        .confirmation_message => {
            name = Lane.literal("CONFIRM ACTION");
            value = Lane.literal(modal_messages[@intCast(@backingInt(source.editor.modal.kind.?))]);
        },
        .dismiss_confirmation => {
            role = .button;
            name = Lane.literal("KEEP EDITING");
        },
        .confirm => {
            role = .button;
            name = Lane.literal("CONFIRM");
        },
    }
    const action: ?H1.ActionId = if (d.focusable) if (descriptorIndex(d)) |i| .{ .bytes = catalog[i].id } else null else null;
    const primary = try sink.node(widgetId(d.key, "primary"), role, name, description, value, parent, d, true, action);
    var child = d;
    child.focusable = false;
    child.selected = false;
    if (d.key == .validation_issue) {
        const severity = if (source.report.entries[d.key.validation_issue].severity == .err) "ERROR | " else "WARNING | ";
        _ = try sink.node(widgetId(d.key, "code"), .text, name, Lane.literal(""), Lane.literal(severity), primary, child, true, null);
        _ = try sink.node(widgetId(d.key, "message"), .text, try value.component(severity.len, value.len - severity.len), Lane.literal(""), Lane.literal(severity), primary, child, true, null);
        _ = try sink.node(widgetId(d.key, "field"), .text, description, Lane.literal(""), Lane.literal(""), primary, child, true, null);
    } else {
        _ = try sink.node(widgetId(d.key, "label"), .text, name, Lane.literal(""), Lane.literal(""), primary, child, true, null);
        _ = try sink.node(widgetId(d.key, "value"), .text, Lane.literal(""), Lane.literal(""), value, primary, child, true, null);
        if (d.key == .field and description.len != 0) _ = try sink.node(widgetId(d.key, "error"), .alert, description, Lane.literal(""), Lane.literal(""), primary, child, true, null);
    }
}
fn alert(sink: *Sink, comptime key: []const u8, text: []const u8, parent: usize, enabled: bool) PrepareError!void {
    const name = try sink.copy(&.{text}, false);
    const primary = try sink.node("c/editor/" ++ key ++ "/primary", .alert, name, Lane.literal(""), Lane.literal(""), parent, null, enabled, null);
    _ = try sink.node("c/editor/" ++ key ++ "/text", .text, name, Lane.literal(""), Lane.literal(""), primary, null, enabled, null);
}
fn admissionText(cause: scenario.AdmissionError) []const u8 {
    return switch (cause) {
        error.Validation => "SCENARIO VALIDATION FAILED",
        error.InvalidInitialState => "SCENARIO INITIAL STATE INVALID",
        error.StaleFingerprint => "SCENARIO FINGERPRINT IS STALE",
    };
}
fn codecText(codec: *const scenario.CodecFailure) []const u8 {
    return switch (codec.*) {
        .too_large => "SCENARIO JSON EXCEEDS THE 65536-BYTE LIMIT",
        .json => "INVALID SCENARIO JSON",
        .version => "UNSUPPORTED SCENARIO FORMAT VERSION",
        .canonical_number => "INVALID CANONICAL SCENARIO NUMBER",
        .validation => "SCENARIO VALIDATION FAILED",
    };
}
fn statusText(status: *const transactions.Status, scratch: *[512]u8) PrepareError![]const u8 {
    return switch (status.*) {
        .none => "",
        .no_saved_scenario => "NO SAVED SCENARIO",
        .loaded => "SCENARIO LOADED INTO DRAFT",
        .saved => "SCENARIO SAVED",
        .failure => |*failure| switch (failure.*) {
            .codec => |*codec| codecText(codec),
            .storage => |store| blk: {
                if (store.record != .scenario) return error.InvalidRecoveryRecord;
                break :blk switch (store.kind) {
                    .io => "SCENARIO STORAGE FAILED",
                    .oversized_slot => try formatted(scratch, "SCENARIO STORAGE SLOT EXCEEDS THE {d}-BYTE LIMIT", .{store.max_bytes}),
                    .before_persist => "SCENARIO STORAGE FAILED BEFORE ATOMIC PERSIST",
                    .browser => "BROWSER SCENARIO STORAGE FAILED",
                    .unavailable_path => "SCENARIO STORAGE PATH UNAVAILABLE",
                };
            },
        },
    };
}
fn actionText(failure: *const transactions.Failure, scratch: *[512]u8) PrepareError![]const u8 {
    _ = scratch;
    return switch (failure.*) {
        .invalid_text => "INVALID TEXT",
        .confirmation_active => "CONFIRMATION ACTIVE",
        .no_confirmation => "NO CONFIRMATION",
        .stale_confirmation => "STALE CONFIRMATION",
        .codec => |*codec| codecText(codec),
        .validation => "SCENARIO VALIDATION FAILED",
        .admission => |cause| admissionText(cause),
    };
}
fn recoveryText(failure: session_model.RecoveryFailure, scratch: *[512]u8) PrepareError![]const u8 {
    const text = switch (failure) {
        .general => |general| switch (general.*) {
            .admission => |cause| admissionText(cause),
            .command => |cause| switch (cause) {
                error.ModeRefused => "COMMAND UNAVAILABLE IN CURRENT MODE",
                error.OutOfMemory => "COMMAND MEMORY ALLOCATION FAILED",
                error.MissingSource => "SELECT A SOURCE WORLD",
                error.MissingTarget => "SELECT A TARGET WORLD",
                else => |reason| @import("classic_commands.zig").rejectionLabel(reason),
            },
        },
        .resource => |resource| blk: {
            const literals = .{ "PREFERENCE MEMORY ALLOCATION FAILED", "EDITOR OPERATION FAILED", "COMMAND MEMORY ALLOCATION FAILED" };
            const present = [_]bool{ resource.preferences != null, resource.editor != null, resource.command != null };
            var length: usize = 0;
            inline for (literals, 0..) |literal, i| if (present[i]) {
                if (length != 0) {
                    @memcpy(scratch[length..][0..3], " | ");
                    length += 3;
                }
                @memcpy(scratch[length..][0..literal.len], literal);
                length += literal.len;
            };
            if (length == 0) return error.InvalidSource;
            break :blk scratch[0..length];
        },
        .tick => |tick| try formatted(scratch, "SIMULATION MEMORY ALLOCATION FAILED AT TICK {d}", .{tick.tick}),
        .preferences => |failure_value| switch (failure_value.*) {
            .codec => |codec| switch (codec) {
                .too_large => "preferences JSON exceeds the 16384-byte limit",
                .version => "unsupported preferences format version",
                .ui_scale => |scale| try formatted(scratch, "unsupported UI scale percentage {d}", .{scale}),
                .json => "invalid preferences JSON",
            },
            .store => |store| blk: {
                if (store.record != .preferences) return error.InvalidRecoveryRecord;
                break :blk switch (store.kind) {
                    .io => "PREFERENCE STORAGE FAILED",
                    .oversized_slot => try formatted(scratch, "preference storage slot exceeds the {d}-byte limit", .{store.max_bytes}),
                    .before_persist => "PREFERENCE STORAGE FAILED BEFORE ATOMIC PERSIST",
                    .browser => "BROWSER PREFERENCE STORAGE FAILED",
                    .unavailable_path => "PREFERENCE STORAGE PATH UNAVAILABLE",
                };
            },
        },
    };
    if (text.len > 128) return error.InvalidSource;
    return text;
}
pub fn validateEditorTarget(view: H1.FrameView, published_revision: u64, current: CurrentStamp, current_modal: ?editor_model.ModalKind, id: H1.ActionId) ActionError!Target {
    if (!std.unicode.utf8ValidateSlice(id.bytes)) return error.InvalidUtf8;
    if (published_revision != current.revision or !view.owner.matches(current.owner)) return error.StaleProjection;
    if (current.owner.draft == null) return error.InvalidSource;
    for (catalog, 0..) |entry, i| if (std.mem.eql(u8, entry.id, id.bytes)) {
        if (!entry.editable) return error.UnknownAction;
        if ((current_modal != null) != (i >= 107)) return error.InactiveModal;
        for (view.nodes) |node| if (node.action) |actual| if (std.mem.eql(u8, actual.bytes, id.bytes)) {
            if (!node.enabled) return error.DisabledAction;
            if (!node.focusable) return error.UnknownAction;
            return entry.target;
        };
        return error.UnknownAction;
    };
    return error.UnknownAction;
}
pub fn prepareEditorFromSession(allocator: Allocator, budget: *H1.PreparationBudget, workspace: *layout.Workspace, session: *const session_model.Session, focus: FocusInput, current: CurrentStamp) PrepareError!PreparedEditor {
    if (session.core.mode() != .editing_scenario) return error.WrongProjectionMode;
    const editor = if (session.editor.session) |*value| value else return error.NoEditor;
    const report = editor.validationReport();
    var flags: layout.ParseFlags = undefined;
    for (editor_model.catalog) |field| flags[field.id.slot()] = try editor.fieldHasParseError(field.id);
    const viewport = session.core.viewport();
    return prepareEditor(allocator, budget, workspace, .{ .editor = editor, .report = &report, .parse_flags = &flags, .viewport = .{ viewport.x, viewport.y }, .store_status = &session.editor.status, .action_failure = if (session.editor.action_failure) |*failure| failure else null, .focus = focus, .recovery = session.recoveryFailure() }, current);
}
fn rejectOverlap(workspace: *const layout.Workspace, address: usize, length: usize) PrepareError!void {
    if (length == 0) return;
    const first = @intFromPtr(workspace);
    const last = std.math.add(usize, first, @sizeOf(layout.Workspace)) catch return error.ArithmeticOverflow;
    const end = std.math.add(usize, address, length) catch return error.ArithmeticOverflow;
    if (address < last and first < end) return error.AliasedWorkspace;
}
fn guardObject(workspace: *const layout.Workspace, pointer: anytype) PrepareError!void {
    try rejectOverlap(workspace, @intFromPtr(pointer), @sizeOf(@TypeOf(pointer.*)));
}
fn guardBytes(workspace: *const layout.Workspace, bytes: []const u8) PrepareError!void {
    try rejectOverlap(workspace, @intFromPtr(bytes.ptr), bytes.len);
}
fn guardCodec(workspace: *const layout.Workspace, codec: *const scenario.CodecFailure) PrepareError!void {
    switch (codec.*) {
        .canonical_number => |detail| {
            try guardBytes(workspace, detail.field);
            try guardBytes(workspace, detail.kind);
        },
        else => {},
    }
}
fn guardStore(workspace: *const layout.Workspace, store: @import("classic_store.zig").StoreFailure) PrepareError!void {
    if (store.message) |message| try guardBytes(workspace, message);
}
fn validateSourceBorrowing(workspace: *const layout.Workspace, source: Source, context: []const u8) PrepareError!void {
    try guardObject(workspace, source.editor);
    try guardObject(workspace, source.store_status);
    try guardBytes(workspace, context);
    for ([_]?[]const u8{ source.raw_status, source.raw_action_error, source.raw_recovery }) |maybe| if (maybe) |bytes| {
        try guardBytes(workspace, bytes);
    };
    if (source.focus.saved_base) |base| try rejectOverlap(workspace, @intFromPtr(base.order.ptr), std.math.mul(usize, base.order.len, @sizeOf(layout.FocusKey)) catch return error.ArithmeticOverflow);
    for (editor_model.catalog) |field| try guardBytes(workspace, source.editor.fieldText(field.text_id) orelse return error.InvalidSource);
    try guardBytes(workspace, source.editor.preedit());
    const draft = source.editor.current();
    for (draft.worlds) |world| try guardBytes(workspace, world.name);
    switch (source.store_status.*) {
        .failure => |*failure| switch (failure.*) {
            .codec => |*codec| try guardCodec(workspace, codec),
            .storage => |store| try guardStore(workspace, store),
        },
        else => {},
    }
    if (source.action_failure) |failure| {
        try guardObject(workspace, failure);
        switch (failure.*) {
            .codec => |*codec| try guardCodec(workspace, codec),
            else => {},
        }
    }
    if (source.recovery) |failure| switch (failure) {
        .general => |general| try guardObject(workspace, general),
        .preferences => |preferences| {
            try guardObject(workspace, preferences);
            switch (preferences.*) {
                .store => |store| try guardStore(workspace, store),
                else => {},
            }
        },
        else => {},
    };
}
test "H3b UI static literal ID inventory and checked envelopes" {
    var action_bytes: usize = 0;
    var typed_node_bytes: usize = 0;
    var nonaction_bytes: usize = 0;
    for (catalog, 0..) |entry, i| {
        try std.testing.expect(entry.id.len <= 48);
        action_bytes += entry.id.len;
        inline for (.{ "primary", "label", "value" }) |suffix| {
            const id = nodeId(i, suffix);
            try std.testing.expect(id.len <= 64);
            typed_node_bytes += id.len;
        }
        if (i < 90) {
            const id = nodeId(i, "error");
            try std.testing.expect(id.len <= 64);
            typed_node_bytes += id.len;
        }
    }
    for (0..98) |i| inline for (.{ "primary", "code", "message", "field" }) |suffix| {
        const id = nonactionId(.{ .validation_issue = @intCast(i) }, suffix);
        try std.testing.expect(id.len <= 64);
        nonaction_bytes += id.len;
    };
    inline for (.{ layout.WidgetKey{ .status = .header }, layout.WidgetKey{ .status = .store }, layout.WidgetKey.preview, layout.WidgetKey.validation_clean, layout.WidgetKey.confirmation_message }) |key| inline for (.{ "primary", "label", "value" }) |suffix| {
        nonaction_bytes += nonactionId(key, suffix).len;
    };
    for ([_][]const u8{ "c/editor/root", "c/editor/dialog", "c/editor/status_region", "c/editor/navigation", "c/editor/preview_region", "c/editor/content", "c/editor/footer_region", "c/editor/validation_list", "c/editor/recovery_region", "c/editor/confirmation_dialog", "c/editor/preedit", "c/editor/action_failure/primary", "c/editor/action_failure/text", "c/editor/recovery/primary", "c/editor/recovery/text" }) |id| nonaction_bytes += id.len;
    std.debug.print("H3b static ID literal inventories action={d} typednodes={d} nonactionnodes={d} total={d}; linker pooling/section footprint UNKNOWN; static109records={d}\n", .{ action_bytes, typed_node_bytes, nonaction_bytes, action_bytes + typed_node_bytes + nonaction_bytes, @sizeOf(@TypeOf(catalog)) });
}

fn focusByteCount(source: Source, view: layout.LayoutView) PrepareError!usize {
    var bytes: usize = 0;
    var count: usize = 0;
    if (source.editor.modal.kind != null) {
        const saved = source.focus.saved_base.?;
        for (saved.order) |key| {
            bytes = try checked(bytes, catalog[try keyIndex(key)].id.len);
            count += 1;
        }
        if (saved.restore) |restore| {
            const index = try keyIndex(restore);
            var found = index >= 107;
            for (saved.order) |key| if (try keyIndex(key) == index) {
                found = true;
            };
            if (!found) bytes = try checked(bytes, catalog[index].id.len);
        }
    }
    for (view.widgets) |d| if (d.enabled and d.focusable) {
        if (descriptorIndex(d)) |index| {
            bytes = try checked(bytes, catalog[index].id.len);
            count += 1;
        }
    };
    if (count > 32) return error.InputLimit;
    return bytes;
}
