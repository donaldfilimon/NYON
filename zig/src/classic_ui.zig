//! Pure retained Classic command-deck/settings projection. No host effects.
const std = @import("std");
const retained = @import("retained_semantics.zig");
const session_model = @import("classic_session.zig");
const preferences = @import("preferences.zig");
const model = @import("model.zig");
const math = @import("classic_math.zig");
const marker = @import("classic_start_marker.zig");
pub const Vec2 = math.Vec2;
pub const GeometryView = struct { revision: u64, viewport: Vec2, scale: f32, settings: bool, help_visible: bool };
pub const AttachError = error{ EmptyProjection, WrongProjectionMode, AlreadyPublished, AlreadyAttached, InvalidMarker, InvalidWorldName, NonFinite, Overflow, InputLimit, InvalidSource };
pub const SetHudFocusError = error{ EmptyProjection, WrongProjectionMode, AlreadyPublished, InvalidUtf8, InvalidFocus };
pub const CurrentStamp = struct { revision: u64, owner: retained.OwnerSpec };
pub const TypedAction = session_model.UiAction;
pub const TransitionAnnouncement = enum { ready, preferences_changed };
pub const ProjectionMode = enum { playing, settings };
pub const ProjectionSource = struct {
    mode: ProjectionMode = .playing,
    viewport: Vec2 = .{ .x = 1440, .y = 900 },
    preferences: preferences.UserPreferencesV1 = .{},
    speed_multiplier: u8 = 1,
    selected_field: model.FieldKind = .atmosphere,
    source_name: ?[]const u8 = null,
    target_name: ?[]const u8 = null,
    strength: ?u64 = null,
    launch_enabled: bool = false,
    tray_status: []const u8 = "SELECT SOURCE AND TARGET",
    decrease_enabled: bool = false,
    increase_enabled: bool = false,
    decrease_tooltip: []const u8 = "Select a Union world",
    increase_tooltip: []const u8 = "Select a Union world",
    help_visible: bool = false,
    onboarding_step: ?@import("classic_onboarding.zig").Step = null,
    pointer: ?Vec2 = null,
    hovered_name: ?[]const u8 = null,
    recoverable: ?[]const u8 = null,
    hud_focus: ?TypedAction = null,
    settings_focus: usize = 0,
    announcements: []const TransitionAnnouncement = &.{},
};
pub const limits = retained.Limits{ .max_nodes = 72, .max_child_indices = 71, .max_id_bytes = 12288, .max_text_bytes = 49152, .max_focus_actions = 18, .max_announcements = 2, .max_owner_context_bytes = 128, .max_owned_bytes = 81920 };
pub const PrepareError = retained.FrameError || error{ InputLimit, InvalidWorldName, InvalidSource, WrongProjectionMode, InvalidRecoveryRecord, NonFinite, Overflow };
pub const PublishError = retained.FrameError || error{ StaleProjection, BudgetMismatch, EmptyProjection };
pub const ActionError = error{ StaleProjection, UnknownAction, DisabledAction };
const Header = struct {
    allocator: std.mem.Allocator,
    budget: *retained.PreparationBudget,
    charge: retained.Charge,
    resident: retained.TokenRef,
    retained_resident: bool = false,
    hold: ?u64,
    revision: u64,
    context: [128]u8 = undefined,
    context_len: usize = 0,
    nodes: [72]retained.NodeSpec = undefined,
    children: [71]usize = undefined,
    focus: [18]retained.ActionId = undefined,
    announcements: [2]retained.AnnouncementSpec = undefined,
    arena: [20480]u8 = undefined,
    node_count: usize = 0,
    child_count: usize = 0,
    focus_count: usize = 0,
    announcement_count: usize = 0,
    used: usize = 0,
    focused: ?retained.ActionId = null,
    viewport: Vec2,
    scale: f32,
    settings: bool,
    help_visible: bool,
    marker_attached: bool = false,
    published: bool = false,
    marker_visual: ?marker.Marker = null,
};
pub const HEADER_BYTES = @sizeOf(Header);
comptime {
    if (@sizeOf(Header) > 36864 or @sizeOf(PreparedProjection) != 8) @compileError("H3a target layout exceeds reviewed cap");
    if (@sizeOf(retained.NodeSpec) != 160 or @sizeOf(retained.AnnouncementSpec) != 40 or @sizeOf(retained.ActionId) != 16 or @sizeOf(usize) != 8) @compileError("H3a target layout differs from reviewed profile");
}
/// Move-only by convention. input() borrows the stable Header until deinit.
pub const PreparedProjection = struct {
    header: ?*Header = null,
    pub fn deinit(self: *PreparedProjection) void {
        const h = self.header orelse return;
        const allocator = h.allocator;
        var charge = h.charge;
        if (h.retained_resident) h.resident.release();
        allocator.destroy(h);
        charge.release();
        self.header = null;
    }
    pub fn geometry(self: *const PreparedProjection) error{EmptyProjection}!GeometryView {
        const h = self.header orelse return error.EmptyProjection;
        return .{ .revision = h.revision, .viewport = h.viewport, .scale = h.scale, .settings = h.settings, .help_visible = h.help_visible };
    }
    /// Inline owned visual metadata. Copy before ending the prepared lease.
    pub fn startMarker(self: *const PreparedProjection) error{EmptyProjection}!?marker.Marker {
        return (self.header orelse return error.EmptyProjection).marker_visual;
    }
    /// End all immutable Header/input leases before calling. Only canonical
    /// current HUD IDs escape this allocation-free prepublication transaction.
    pub fn setHudPublishedFocus(self: *PreparedProjection, focused: ?retained.ActionId) SetHudFocusError!void {
        const h = self.header orelse return error.EmptyProjection;
        if (h.settings) return error.WrongProjectionMode;
        if (h.published) return error.AlreadyPublished;
        var canonical: ?retained.ActionId = null;
        if (focused) |id| {
            if (!std.unicode.utf8ValidateSlice(id.bytes)) return error.InvalidUtf8;
            for (h.focus[0..h.focus_count]) |entry| {
                if (std.mem.eql(u8, entry.bytes, id.bytes)) {
                    canonical = entry;
                    break;
                }
            }
            if (canonical == null) return error.InvalidFocus;
        }
        h.focused = canonical;
    }
    pub fn input(self: *const PreparedProjection) retained.FrameInput {
        const h = self.header.?;
        return .{ .nodes = h.nodes[0..h.node_count], .child_indices = h.children[0..h.child_count], .announcements = h.announcements[0..h.announcement_count], .owner = .{ .resident = h.resident, .hold = h.hold, .context = h.context[0..h.context_len], .draft = null }, .focus = .{ .base = h.focus[0..h.focus_count], .modal = null, .focused = h.focused, .restore = null } };
    }
};
/// Synchronous copy. Caller admits one preparation resident-reference slot
/// before this call and releases/transfers it on failure/success. The stable
/// shared budget outlives every Header/frame. One Header/candidate at a time.
pub fn prepareHudSettings(allocator: std.mem.Allocator, budget: *retained.PreparationBudget, source: ProjectionSource, current: CurrentStamp) PrepareError!PreparedProjection {
    try validateSource(source, current);
    var reservation = try budget.reserve(.frame_scratch, HEADER_BYTES);
    const h = allocator.create(Header) catch {
        reservation.cancel();
        return error.OutOfMemory;
    };
    const charge = reservation.commit();
    const viewport: Vec2 = if (source.viewport.isFinite()) .{ .x = @max(source.viewport.x, 1), .y = @max(source.viewport.y, 1) } else .{ .x = 1, .y = 1 };
    const scale = math.clamp(math.clamp(@min(viewport.x / 1440, viewport.y / 900), 0.72, 1.15) * source.preferences.ui_scale.factor(), 0.60, 1.50);
    h.* = .{ .allocator = allocator, .budget = budget, .charge = charge, .resident = current.owner.resident, .hold = current.owner.hold, .revision = current.revision, .viewport = viewport, .scale = scale, .settings = source.mode == .settings, .help_visible = source.help_visible };
    var result = PreparedProjection{ .header = h };
    errdefer result.deinit();
    h.context_len = current.owner.context.len;
    @memcpy(h.context[0..h.context_len], current.owner.context);
    try build(h, source);
    _ = try h.resident.retain();
    h.retained_resident = true;
    return result;
}
test "H3a actual Header target layout" {
    std.debug.print("H3a Header={d}, handle={d}, optional_revision={d}, Source={d}, Controls={d}, Tray={d}, WorldName={d}, CurrentStamp={d}, FrameInput={d}; transitive stack UNKNOWN\n", .{ HEADER_BYTES, @sizeOf(PreparedProjection), @sizeOf(?u64), @sizeOf(ProjectionSource), @sizeOf(session_model.Controls), @sizeOf(@import("classic_commands.zig").Tray), @sizeOf(model.WorldName), @sizeOf(CurrentStamp), @sizeOf(retained.FrameInput) });
    try std.testing.expect(HEADER_BYTES <= 36864);
}
const commands = @import("classic_commands.zig");
const settings_model = @import("classic_settings.zig");
const Rect = retained.Rect;
const rejection_literals = [_][]const u8{ "REJECTED - WRONG TICK", "REJECTED - DUPLICATE COMMAND", "REJECTED - CAMPAIGN FINISHED", "REJECTED - INVALID WORLD", "REJECTED - CHOOSE ANOTHER TARGET", "REJECTED - SOURCE IS NOT UNION", "REJECTED - INSUFFICIENT ENERGY", "REJECTED - FIELD AT LIMIT" };
const tutorial_titles = [_][]const u8{ "SELECT A UNION WORLD", "PREVIEW A DESTINATION", "LAUNCH THE FLEET", "TUNE A FIELD", "READ THE ADVISORY", "OPEN THE SCENARIO EDITOR" };
fn inLiterals(value: []const u8, literals: []const []const u8) bool {
    for (literals) |literal| if (std.mem.eql(u8, value, literal)) return true;
    return false;
}
fn validateSource(s: ProjectionSource, current: CurrentStamp) PrepareError!void {
    if (current.owner.draft != null) return error.InvalidSource;
    if (s.announcements.len > 2) return error.InputLimit;
    if (current.owner.context.len > 128) return error.InputLimit;
    if (!std.unicode.utf8ValidateSlice(current.owner.context)) return error.InvalidUtf8;
    for ([_]?[]const u8{ s.source_name, s.target_name, s.hovered_name }) |name| if (name) |value| {
        _ = model.WorldName.init(value) catch return error.InvalidWorldName;
    };
    if (s.recoverable) |value| {
        if (value.len > 16384) return error.InputLimit;
        if (!std.unicode.utf8ValidateSlice(value)) return error.InvalidUtf8;
    }
    if (!inLiterals(s.tray_status, &rejection_literals) and !inLiterals(s.tray_status, &.{ "SELECT SOURCE AND TARGET", "COMMAND READY - EXPLICIT LAUNCH REQUIRED" })) return error.InvalidSource;
    if (!inLiterals(s.decrease_tooltip, &rejection_literals) and !inLiterals(s.decrease_tooltip, &.{ "Select a Union world", "Decrease selected field (Q)" })) return error.InvalidSource;
    if (!inLiterals(s.increase_tooltip, &rejection_literals) and !inLiterals(s.increase_tooltip, &.{ "Select a Union world", "Increase selected field (E)" })) return error.InvalidSource;
}
fn copy(h: *Header, value: []const u8) PrepareError![]const u8 {
    if (value.len > h.arena.len - h.used) return error.InputLimit;
    const start = h.used;
    h.used += value.len;
    @memcpy(h.arena[start..h.used], value);
    return h.arena[start..h.used];
}
/// Decimal-only formatter, no std.fmt float or allocator path.
fn decimal(scratch: *[20]u8, value: u64) []const u8 {
    var n = value;
    var start: usize = scratch.len;
    while (true) {
        start -= 1;
        scratch[start] = @intCast('0' + n % 10);
        n /= 10;
        if (n == 0) return scratch[start..];
    }
}
fn concat(h: *Header, prefix: []const u8, value: []const u8, suffix: []const u8) PrepareError![]const u8 {
    const start = h.used;
    _ = try copy(h, prefix);
    _ = try copy(h, value);
    _ = try copy(h, suffix);
    return h.arena[start..h.used];
}
fn node(h: *Header, id: []const u8, role: retained.NodeRole, name: []const u8, parent: ?usize, bounds: ?Rect) PrepareError!usize {
    if (h.node_count == h.nodes.len) return error.InputLimit;
    const index = h.node_count;
    h.node_count += 1;
    h.nodes[index] = .{ .id = .{ .bytes = id }, .role = role, .name = name, .description = "", .value = null, .enabled = true, .selected = false, .focusable = false, .hierarchy_level = null, .expanded = null, .action = null, .bounds = bounds, .visible = true, .parent = parent, .child_start = 0, .child_count = 0 };
    return index;
}
fn control(h: *Header, comptime mode: []const u8, comptime key: []const u8, role: retained.NodeRole, label: []const u8, tooltip: []const u8, action: TypedAction, enabled: bool, selected: bool, parent: usize, bounds: Rect, focused: bool) PrepareError!void {
    const id = "a/" ++ mode ++ "/" ++ key;
    const primary = try node(h, "c/" ++ mode ++ "/" ++ key ++ "/primary", role, label, parent, bounds);
    h.nodes[primary].description = tooltip;
    h.nodes[primary].value = "";
    h.nodes[primary].enabled = enabled;
    h.nodes[primary].selected = selected;
    h.nodes[primary].focusable = true;
    h.nodes[primary].action = .{ .bytes = id };
    _ = try node(h, "c/" ++ mode ++ "/" ++ key ++ "/label", .text, label, primary, bounds);
    _ = try node(h, "c/" ++ mode ++ "/" ++ key ++ "/value", .text, "", primary, bounds);
    if (h.focus_count == h.focus.len) return error.InputLimit;
    h.focus[h.focus_count] = .{ .bytes = id };
    h.focus_count += 1;
    if (focused) h.focused = .{ .bytes = id };
    // Typed actions are identified by immutable IDs, with checkbox state encoded
    // in the node. No source/session pointer or redundant owning action map.
    const mapped = actionForId(id, selected) catch return error.InvalidSource;
    if (!std.meta.eql(mapped, action)) return error.InvalidSource;
}
fn mainBounds(h: *const Header, index: usize) PrepareError!Rect {
    @setFloatMode(.strict);
    const v = h.viewport;
    const s = h.scale;
    const columns: usize = if (v.x >= 1100) 9 else 5;
    const rows: usize = if (columns == 9) 1 else 2;
    const extent = @max(48 * s, 44);
    const available = (v.x - 16 - @as(f32, @floatFromInt(columns - 1)) * 6) / @as(f32, @floatFromInt(columns));
    const width = @min(if (columns == 9) @max(92 * s, 72) else @max(124 * s, 104), available);
    const row = index / columns;
    const column = index % columns;
    const count = @min(9 - row * columns, columns);
    const row_width = @as(f32, @floatFromInt(count)) * width + @as(f32, @floatFromInt(count - 1)) * 6;
    const start = @max((v.x - row_width) * 0.5, 4);
    const y = @max(v.y - @as(f32, @floatFromInt(rows)) * extent - @as(f32, @floatFromInt(rows - 1)) * 6 - 4, 0);
    return Rect.fromXYWH(start + @as(f32, @floatFromInt(column)) * (width + 6), y + @as(f32, @floatFromInt(row)) * (extent + 6), width, extent);
}
fn isFocused(s: ProjectionSource, action: TypedAction) bool {
    return if (s.hud_focus) |focused| std.meta.eql(focused, action) else false;
}
fn build(h: *Header, s: ProjectionSource) PrepareError!void {
    for (s.announcements) |announcement| {
        h.announcements[h.announcement_count] = switch (announcement) {
            .ready => .{ .kind = .status, .code = "ready", .message = "COMMAND READY" },
            .preferences_changed => .{ .kind = .status, .code = "preferences_changed", .message = "PREFERENCES CHANGED" },
        };
        h.announcement_count += 1;
    }
    const root = try node(h, if (h.settings) "c/settings/root" else "c/hud/root", .application, "NYON", null, null);
    if (h.settings) try buildSettings(h, s, root) else try buildPlaying(h, s, root);
    if (s.onboarding_step) |step| {
        const group = try node(h, "c/guidance", .region, "", root, try guidanceRect(h));
        const ordinal: usize = @intCast(@backingInt(step));
        var scratch: [20]u8 = undefined;
        const line = try concat(h, "TUTORIAL ", decimal(&scratch, ordinal + 1), "/6 // PAUSED");
        _ = try node(h, "c/guidance/progress", .text, line, group, null);
        _ = try node(h, "c/guidance/title", .text, tutorial_titles[ordinal], group, null);
    }
    var tooltip: ?[]const u8 = null;
    if (s.pointer) |pointer| {
        for (h.nodes[0..h.node_count]) |n| if (n.action != null) {
            if (n.bounds.?.contains(.{ pointer.x, pointer.y })) {
                tooltip = n.description;
                break;
            }
        };
        if (tooltip == null) if (s.hovered_name) |name| {
            tooltip = try concat(h, "WORLD ", name, " - SELECT OR PREVIEW DESTINATION");
        };
    }
    if (tooltip) |text| _ = try node(h, "c/tooltip", .text, text, root, try tooltipRect(h));
    if (s.recoverable) |raw| {
        const text = try copy(h, raw);
        const index = try node(h, "c/recoverable", .alert, text, root, null);
        h.nodes[index].value = text;
    }
    const field = switch (s.selected_field) {
        .atmosphere => "ATM",
        .hydrosphere => "HYD",
        .topology => "TOP",
    };
    const status = try node(h, "c/selected-field", .status, "", root, null);
    h.nodes[status].value = field;
    // Stable source-order adjacency, no local child table escapes.
    for (h.nodes[0..h.node_count], 0..) |*parent, i| {
        parent.child_start = h.child_count;
        for (h.nodes[0..h.node_count], 0..) |child, j| if (child.parent == i) {
            if (h.child_count == h.children.len) return error.InputLimit;
            h.children[h.child_count] = j;
            h.child_count += 1;
            parent.child_count += 1;
        };
    }
}
fn buildPlaying(h: *Header, s: ProjectionSource, root: usize) PrepareError!void {
    const toolbar = try node(h, "c/hud/toolbar", .toolbar, "", root, null);
    const actions = [_]TypedAction{ .speed_down, .toggle_pause, .speed_up, .launch, .clear_selection, .reset_camera, .open_settings, .toggle_help, .open_scenario };
    const keys = .{ "speed_down", "pause", "speed_up", "launch", "clear_selection", "reset_camera", "settings", "help", "scenario" };
    const labels = .{ "SPEED -", "PAUSE", "SPEED +", "LAUNCH", "CLEAR", "RESET", "SETTINGS", "HELP", "SCENARIO" };
    const tips = .{ "Decrease game speed ([)", "Pause or resume (P)", "Increase game speed (])", "Launch the command tray preview", "Clear source and destination (Escape)", "Reset tactical camera (C)", "Open local settings", "Show controls and guidance", "Open detached scenario editor (F4)" };
    inline for (keys, labels, tips, 0..) |key, label, tip, i| {
        const enabled = switch (i) {
            0, 1, 2 => s.onboarding_step == null,
            3 => s.launch_enabled,
            4 => s.source_name != null or s.target_name != null,
            else => true,
        };
        const selected = switch (i) {
            1 => s.speed_multiplier == 0,
            7 => s.help_visible,
            else => false,
        };
        try control(h, "hud", key, .button, if (i == 1 and s.speed_multiplier == 0) "PLAY" else label, tip, actions[i], enabled, selected, toolbar, try mainBounds(h, i), isFocused(s, actions[i]));
    }
    const v = h.viewport;
    const scale = h.scale;
    if (s.onboarding_step != null) try control(h, "hud", "skip", .button, "SKIP", "Skip tutorial and play this match now", .skip_onboarding, true, false, toolbar, try Rect.fromXYWH(v.x - 116, 8, 108, 44), isFocused(s, .skip_onboarding));
    const width = @max(72 * scale, 44);
    const height = @max(44 * scale, 44);
    const x = @max(v.x - 268 * scale, 4);
    inline for (.{ "atmosphere", "hydrosphere", "topology" }, .{ "ATM", "HYD", "TOP" }, [_]model.FieldKind{ .atmosphere, .hydrosphere, .topology }, 0..) |key, label, field, i| {
        const action = TypedAction{ .select_field = field };
        try control(h, "hud", "field/" ++ key, .option, label, "Select field channel (1/2/3)", action, true, s.selected_field == field, toolbar, try Rect.fromXYWH(x + @as(f32, @floatFromInt(i)) * (width + 4), 236 * scale, width, height), isFocused(s, action));
    }
    try control(h, "hud", "field/decrease", .button, "FIELD -", try copy(h, s.decrease_tooltip), .decrease_field, s.decrease_enabled, false, toolbar, try Rect.fromXYWH(x, 286 * scale, width * 1.5, height), isFocused(s, .decrease_field));
    try control(h, "hud", "field/increase", .button, "FIELD +", try copy(h, s.increase_tooltip), .increase_field, s.increase_enabled, false, toolbar, try Rect.fromXYWH(x + width * 1.5 + 4, 286 * scale, width * 1.5, height), isFocused(s, .increase_field));
    try control(h, "hud", "advisory_ack", .button, "ADVISORY", "Read the advisory status", .acknowledge_advisory, true, false, toolbar, try Rect.fromXYWH(@max(v.x - 276 * scale, 0), 380 * scale, @max(268 * scale, 44), @max(150 * scale, 44)), isFocused(s, .acknowledge_advisory));
    if (s.help_visible) try control(h, "hud", "reset_guidance", .button, "RESTART GUIDANCE", "Restart first-run guidance", .reset_onboarding, true, false, toolbar, try Rect.fromXYWH(v.x * 0.5 - 110, v.y * 0.5 + 54, 220, 44), isFocused(s, .reset_onboarding));
    const tray = try node(h, "c/hud/tray", .region, "", root, try trayRect(h));
    _ = try node(h, "c/hud/tray/source", .text, try concat(h, "SOURCE ", s.source_name orelse "NONE", ""), tray, null);
    _ = try node(h, "c/hud/tray/target", .text, try concat(h, "TARGET ", s.target_name orelse "NONE", ""), tray, null);
    var scratch: [20]u8 = undefined;
    _ = try node(h, "c/hud/tray/strength", .text, try concat(h, "STRENGTH ", if (s.strength) |strength| decimal(&scratch, strength) else "-", ""), tray, null);
    _ = try node(h, "c/hud/tray/status", .text, try copy(h, s.tray_status), tray, null);
}
fn buildSettings(h: *Header, s: ProjectionSource, root: usize) PrepareError!void {
    const dialog = try node(h, "c/settings/dialog", .dialog, "SETTINGS", root, null);
    var parent = dialog;
    const keys = .{ "scale/85", "scale/100", "scale/115", "scale/130", "motion", "contrast", "quality/auto", "quality/low", "quality/high", "reset_guidance", "close" };
    const labels = .{ "85%", "100%", "115%", "130%", "REDUCED MOTION", "HIGH CONTRAST", "AUTO", "LOW", "HIGH", "RESET GUIDANCE", "CLOSE" };
    const actions = settings_model.actions(s.preferences);
    const v = h.viewport;
    const scale = h.scale;
    const columns: usize = if (v.x >= 1100) 4 else 2;
    const width = @min(@max(150 * scale, 118), (v.x - 16 - @as(f32, @floatFromInt(columns - 1)) * 8) / @as(f32, @floatFromInt(columns)));
    const panel = width * @as(f32, @floatFromInt(columns)) + 8 * @as(f32, @floatFromInt(columns - 1));
    const x = (v.x - panel) * 0.5;
    const y = @max(v.y * 0.5 - 130, 8);
    const height = @max(48 * scale, 44);
    inline for (keys, labels, 0..) |key, label, i| {
        // H1 requires flat DFS preorder, not merely a correct adjacency graph.
        if (i == 0) parent = try node(h, "c/settings/scales", .group, "", dialog, null);
        if (i == 4) parent = try node(h, "c/settings/accessibility", .group, "", dialog, null);
        if (i == 6) parent = try node(h, "c/settings/quality", .group, "", dialog, null);
        if (i == 9) parent = dialog;
        const role: retained.NodeRole = if (i < 4 or (i >= 6 and i < 9)) .option else if (i < 6) .checkbox else .button;
        const selected = switch (actions[i]) {
            .set_ui_scale => |value| value == s.preferences.ui_scale,
            .set_motion => s.preferences.motion == .reduced,
            .set_high_contrast => s.preferences.high_contrast,
            .set_graphics_quality => |value| value == s.preferences.graphics_quality,
            else => false,
        };
        try control(h, "settings", key, role, label, "Local offline preference", .{ .settings = actions[i] }, true, selected, parent, try Rect.fromXYWH(x + @as(f32, @floatFromInt(i % columns)) * (width + 8), y + @as(f32, @floatFromInt(i / columns)) * (height + 8), width, height), @min(s.settings_focus, 10) == i);
    }
}
fn panelWidth(desired: f32, minimum: f32, maximum: f32) f32 {
    return if (maximum >= minimum) math.clamp(desired, minimum, maximum) else maximum;
}
fn trayRect(h: *const Header) PrepareError!Rect {
    const width = panelWidth(448 * h.scale, 344, @max(h.viewport.x - 16, 1));
    return Rect.fromXYWH((h.viewport.x - width) * 0.5, @max(70 * h.scale, 52), width, @max(108 * h.scale, 92));
}
fn guidanceRect(h: *const Header) PrepareError!Rect {
    const width = panelWidth(448 * h.scale, 344, @max(h.viewport.x - 132, 1));
    return Rect.fromXYWH((h.viewport.x - width) * 0.5, 8, width, @max(52 * h.scale, 44));
}
fn tooltipRect(h: *const Header) PrepareError!Rect {
    var bottom = h.viewport.y;
    for (h.nodes[0..h.node_count]) |n| if (n.action != null and n.bounds.?.max[1] >= h.viewport.y - 120) {
        bottom = @min(bottom, n.bounds.?.min[1]);
    };
    return Rect.fromXYWH(8, @max(bottom - 34, 0), @min(620 * h.scale, @max(h.viewport.x - 16, 1)), 30);
}
/// UI-thread serialized composite commit. Callers must not interleave observers
/// or source mutation between FrameSlot.replace and trusted revision assignment.
pub fn publishHudSettings(slot: *retained.FrameSlot, published_revision: *?u64, prepared: *const PreparedProjection, current: CurrentStamp) PublishError!void {
    const h = prepared.header orelse return error.EmptyProjection;
    if (h.budget != slot.budget) return error.BudgetMismatch;
    const input = prepared.input();
    if (h.revision != current.revision or !input.owner.matches(current.owner)) return error.StaleProjection;
    try slot.replace(input, limits);
    published_revision.* = current.revision;
    h.published = true;
}
/// published_revision must be the nonnull stored scalar from a successful
/// publication, never a value supplied by an event or fabricated from current.
pub fn validateAction(view: retained.FrameView, published_revision: u64, current: CurrentStamp, id: retained.ActionId) ActionError!TypedAction {
    if (published_revision != current.revision or !view.owner.matches(current.owner)) return error.StaleProjection;
    for (view.nodes) |n| if (n.action) |action| {
        if (std.mem.eql(u8, action.bytes, id.bytes)) {
            if (!n.enabled) return error.DisabledAction;
            if (!n.focusable) return error.UnknownAction;
            return actionForId(id.bytes, n.selected);
        }
    };
    return error.UnknownAction;
}
fn actionForId(id: []const u8, selected: bool) ActionError!TypedAction {
    const keys = .{ "speed_down", "pause", "speed_up", "launch", "clear_selection", "reset_camera", "settings", "help", "scenario", "skip", "field/atmosphere", "field/hydrosphere", "field/topology", "field/decrease", "field/increase", "advisory_ack", "reset_guidance" };
    const actions = [_]TypedAction{ .speed_down, .toggle_pause, .speed_up, .launch, .clear_selection, .reset_camera, .open_settings, .toggle_help, .open_scenario, .skip_onboarding, .{ .select_field = .atmosphere }, .{ .select_field = .hydrosphere }, .{ .select_field = .topology }, .decrease_field, .increase_field, .acknowledge_advisory, .reset_onboarding };
    inline for (keys, 0..) |key, i| if (std.mem.eql(u8, id, "a/hud/" ++ key)) return actions[i];
    const setting_keys = .{ "scale/85", "scale/100", "scale/115", "scale/130", "motion", "contrast", "quality/auto", "quality/low", "quality/high", "reset_guidance", "close" };
    const settings = [_]settings_model.Action{ .{ .set_ui_scale = .percent85 }, .{ .set_ui_scale = .percent100 }, .{ .set_ui_scale = .percent115 }, .{ .set_ui_scale = .percent130 }, .{ .set_motion = if (selected) .full else .reduced }, .{ .set_high_contrast = !selected }, .{ .set_graphics_quality = .auto }, .{ .set_graphics_quality = .low }, .{ .set_graphics_quality = .high }, .reset_onboarding, .close };
    inline for (setting_keys, 0..) |key, i| if (std.mem.eql(u8, id, "a/settings/" ++ key)) return .{ .settings = settings[i] };
    inline for (0..7) |world| if (std.mem.eql(u8, id, "a/hud/marker/" ++ std.fmt.comptimePrint("{d}", .{world}))) return .{ .select_guidance_world = world };
    return error.UnknownAction;
}
fn boundedRecovery(scratch: *[256]u8, prefix: []const u8, number: u64, suffix: []const u8) PrepareError![]const u8 {
    var digits: [20]u8 = undefined;
    const value = decimal(&digits, number);
    const size = prefix.len + value.len + suffix.len;
    if (size > 128) return error.InputLimit;
    @memcpy(scratch[0..prefix.len], prefix);
    @memcpy(scratch[prefix.len..][0..value.len], value);
    @memcpy(scratch[prefix.len + value.len ..][0..suffix.len], suffix);
    return scratch[0..size];
}
/// Closed category display only. Original Session/Store details remain owned
/// and unchanged; this function never clears, allocates or acknowledges them.
pub fn recoveryText(failure: session_model.RecoveryFailure, scratch: *[256]u8) PrepareError![]const u8 {
    return switch (failure) {
        .general => |general| switch (general.*) {
            .command => |cause| switch (cause) {
                error.ModeRefused => "COMMAND UNAVAILABLE IN CURRENT MODE",
                error.OutOfMemory => "COMMAND MEMORY ALLOCATION FAILED",
                error.MissingSource => "SELECT A SOURCE WORLD",
                error.MissingTarget => "SELECT A TARGET WORLD",
                else => |reason| commands.rejectionLabel(reason),
            },
            .admission => |cause| switch (cause) {
                error.Validation => "SCENARIO VALIDATION FAILED",
                error.InvalidInitialState => "SCENARIO INITIAL STATE INVALID",
                error.StaleFingerprint => "SCENARIO FINGERPRINT IS STALE",
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
            break :blk scratch[0..length];
        },
        .tick => |tick| try boundedRecovery(scratch, "SIMULATION MEMORY ALLOCATION FAILED AT TICK ", tick.tick, ""),
        .preferences => |failure_value| switch (failure_value.*) {
            .codec => |codec| switch (codec) {
                .too_large => "preferences JSON exceeds the 16384-byte limit",
                .version => "unsupported preferences format version",
                .ui_scale => |scale| try boundedRecovery(scratch, "unsupported UI scale percentage ", scale, ""),
                .json => "invalid preferences JSON",
            },
            .store => |store| blk: {
                if (store.record != .preferences) return error.InvalidRecoveryRecord;
                break :blk switch (store.kind) {
                    .io => "PREFERENCE STORAGE FAILED",
                    .oversized_slot => try boundedRecovery(scratch, "preference storage slot exceeds the ", @intCast(store.max_bytes), "-byte limit"),
                    .before_persist => "PREFERENCE STORAGE FAILED BEFORE ATOMIC PERSIST",
                    .browser => "BROWSER PREFERENCE STORAGE FAILED",
                    .unavailable_path => "PREFERENCE STORAGE PATH UNAVAILABLE",
                };
            },
        },
    };
}
pub fn prepareHudSettingsFromSession(allocator: std.mem.Allocator, budget: *retained.PreparationBudget, session: *const session_model.Session, pointer: ?Vec2, current: CurrentStamp) PrepareError!PreparedProjection {
    const mode = switch (session.core.mode()) {
        .playing => ProjectionMode.playing,
        .settings => ProjectionMode.settings,
        .editing_scenario => return error.WrongProjectionMode,
    };
    const controls = session.controls(null);
    const focus = session.focusIndex(controls.items());
    const tray = commands.commandTray(&session.core);
    const decrease = commands.fieldControl(&session.core, .decrease);
    const increase = commands.fieldControl(&session.core, .increase);
    var hovered: ?model.WorldName = null;
    if (session.core.hovered_world) |id| for (session.core.campaign().worlds) |world| {
        if (world.id == id) {
            hovered = world.name;
            break;
        }
    };
    var recovery_scratch: [256]u8 = undefined;
    const recovery = if (session.recoveryFailure()) |failure| try recoveryText(failure, &recovery_scratch) else null;
    const source = ProjectionSource{
        .mode = mode,
        .viewport = session.core.viewport(),
        .preferences = session.core.preferences(),
        .speed_multiplier = session.core.speed().multiplier(),
        .selected_field = session.core.selected_field,
        .help_visible = session.core.help_visible,
        .onboarding_step = session.onboarding.step(),
        .source_name = if (tray.source) |*name| name.asSlice() else null,
        .target_name = if (tray.destination) |*name| name.asSlice() else null,
        .strength = tray.launch_strength,
        .launch_enabled = tray.launch_enabled,
        .tray_status = tray.status,
        .decrease_enabled = decrease.enabled,
        .increase_enabled = increase.enabled,
        .decrease_tooltip = decrease.tooltip,
        .increase_tooltip = increase.tooltip,
        .pointer = pointer,
        .hovered_name = if (hovered) |*name| name.asSlice() else null,
        .recoverable = recovery,
        .hud_focus = if (focus) |index| if (index < controls.items().len) controls.items()[index].action else null else null,
        .settings_focus = session.settings.focusIndex(),
    };
    // Every local borrow above is fully copied before this synchronous return.
    return prepareHudSettings(allocator, budget, source, current);
}
pub fn hitTest(prepared: *const PreparedProjection, pointer: Vec2) ?retained.ActionId {
    for (prepared.input().nodes) |n| if (n.enabled and n.action != null) {
        if (n.bounds.?.contains(.{ pointer.x, pointer.y })) return n.action;
    };
    return null;
}
pub fn consumesPointer(prepared: *const PreparedProjection, pointer: Vec2) bool {
    const h = prepared.header.?;
    if (!pointer.isFinite() or h.settings or hitTest(prepared, pointer) != null) return true;
    const scale = @max(h.scale, 0.72);
    const bottom: f32 = if (h.viewport.x < 1100) 102 else 56 * scale;
    if (pointer.x <= 240 * scale or pointer.x >= h.viewport.x - 280 * scale or pointer.y <= 64 * scale or pointer.y >= h.viewport.y - bottom) return true;
    const tray = trayRect(h) catch return true;
    if (tray.contains(.{ pointer.x, pointer.y })) return true;
    if (h.help_visible) {
        const help = Rect.fromXYWH(h.viewport.x * 0.5 - 190, h.viewport.y * 0.5 - 90, 380, 180) catch return true;
        if (help.contains(.{ pointer.x, pointer.y })) return true;
    }
    return false;
}

test "H3c closed marker zero maps to typed guidance action" {
    try std.testing.expectEqualDeep(TypedAction{ .select_guidance_world = 0 }, try actionForId("a/hud/marker/0", false));
}

const marker_tip = "Select your Union world to begin the tutorial";
const marker_ids = blk: {
    var ids: [7]struct { action: []const u8, primary: []const u8, label: []const u8, value: []const u8 } = undefined;
    for (0..7) |i| {
        const digit = std.fmt.comptimePrint("{d}", .{i});
        ids[i] = .{ .action = "a/hud/marker/" ++ digit, .primary = "c/hud/marker/" ++ digit ++ "/primary", .label = "c/hud/marker/" ++ digit ++ "/label", .value = "c/hud/marker/" ++ digit ++ "/value" };
    }
    break :blk ids;
};
const AttachStage = struct { triplet: [3]retained.NodeSpec, visual: marker.Marker, name: model.WorldName, insertion: usize, toolbar: usize, text_bytes: usize, id_bytes: usize };
pub const ATTACH_STAGE_BYTES = @sizeOf(AttachStage);
comptime {
    if (ATTACH_STAGE_BYTES > 736 or @sizeOf(?marker.Marker) > 160 or marker_tip.len != 45) @compileError("H3c attachment target bound");
}
fn attachAdd(a: usize, b: usize) AttachError!usize {
    return std.math.add(usize, a, b) catch error.Overflow;
}
/// Bounded scans validate flat DFS and exact adjacency without a second table.
fn attachTopology(h: *const Header) AttachError!usize {
    if (h.node_count == 0 or h.node_count > h.nodes.len or h.child_count > h.children.len or h.focus_count > h.focus.len or h.announcement_count > h.announcements.len or h.used > h.arena.len or h.context_len > h.context.len) return error.InvalidSource;
    if (h.child_count != h.node_count - 1) return error.InvalidSource;
    const root = h.nodes[0];
    if (!std.mem.eql(u8, root.id.bytes, "c/hud/root") or root.role != .application or !std.mem.eql(u8, root.name, "NYON") or root.parent != null) return error.InvalidSource;
    var toolbar: ?usize = null;
    var edge: usize = 0;
    for (h.nodes[0..h.node_count], 0..) |n, i| {
        if (std.mem.startsWith(u8, n.id.bytes, "c/hud/marker/") or (if (n.action) |a| std.mem.startsWith(u8, a.bytes, "a/hud/marker/") else false)) return error.InvalidSource;
        for (h.nodes[0..i]) |old| if (std.mem.eql(u8, n.id.bytes, old.id.bytes)) return error.InvalidSource;
        if (i != 0) {
            const parent = n.parent orelse return error.InvalidSource;
            if (parent >= i) return error.InvalidSource;
            // The parent must be on the previous node's ancestor chain.
            var ancestor = i - 1;
            while (ancestor != parent) ancestor = h.nodes[ancestor].parent orelse return error.InvalidSource;
        }
        if (std.mem.eql(u8, n.id.bytes, "c/hud/toolbar")) {
            if (toolbar != null or n.role != .toolbar or n.parent != 0) return error.InvalidSource;
            toolbar = i;
        }
        if (n.child_start != edge) return error.InvalidSource;
        var count: usize = 0;
        for (h.nodes[0..h.node_count], 0..) |child, j| if (child.parent == i) {
            if (edge >= h.child_count or h.children[edge] != j) return error.InvalidSource;
            edge += 1;
            count += 1;
        };
        if (count != n.child_count) return error.InvalidSource;
        if (n.bounds) |bounds| _ = Rect.fromBounds(bounds.min, bounds.max) catch return error.InvalidSource;
        if (n.action) |a| {
            if (!n.focusable or n.bounds == null) return error.InvalidSource;
            _ = actionForId(a.bytes, n.selected) catch return error.InvalidSource;
            for (h.nodes[0..i]) |old| if (old.action) |other| if (std.mem.eql(u8, a.bytes, other.bytes)) return error.InvalidSource;
        }
    }
    if (edge != h.child_count) return error.InvalidSource;
    for (h.focus[0..h.focus_count], 0..) |action, i| {
        for (h.focus[0..i]) |old| if (std.mem.eql(u8, old.bytes, action.bytes)) return error.InvalidSource;
        var found = false;
        for (h.nodes[0..h.node_count]) |n| if (n.action) |a| if (std.mem.eql(u8, a.bytes, action.bytes) and n.focusable) {
            found = true;
        };
        if (!found) return error.InvalidSource;
    }
    for (h.nodes[0..h.node_count]) |n| if (n.action) |a| {
        var found = false;
        for (h.focus[0..h.focus_count]) |f| if (std.mem.eql(u8, a.bytes, f.bytes)) {
            found = true;
        };
        if (!found) return error.InvalidSource;
    };
    if (h.focused) |f| {
        var found = false;
        for (h.focus[0..h.focus_count]) |a| if (std.mem.eql(u8, a.bytes, f.bytes)) {
            found = true;
        };
        if (!found) return error.InvalidSource;
    }
    return toolbar orelse error.InvalidSource;
}
fn markerNode(id: []const u8, role: retained.NodeRole, name: []const u8, parent: usize, bounds: Rect) retained.NodeSpec {
    return .{ .id = .{ .bytes = id }, .role = role, .name = name, .description = "", .value = null, .enabled = true, .selected = false, .focusable = false, .hierarchy_level = null, .expanded = null, .action = null, .bounds = bounds, .visible = true, .parent = parent, .child_start = 0, .child_count = 0 };
}
/// Complete nonmutating preflight. text_limit is privately injectable for the
/// exact B+113/B+112 qualification; ordinary source keeps the unchanged cap.
fn preflightMarker(h: *const Header, value: marker.Marker, text_limit: usize) AttachError!AttachStage {
    if (value.world >= 7) return error.InvalidMarker;
    if (value.name.len == 0 or value.name.len > 24) return error.InvalidWorldName;
    const name = model.WorldName.init(value.name.asSlice()) catch return error.InvalidWorldName;
    marker.validateMarkerGeometry(value) catch |err| return switch (err) {
        error.NonFinite => error.NonFinite,
        error.ArithmeticOverflow => error.Overflow,
        error.InvalidMarkerGeometry => error.InvalidMarker,
    };
    const toolbar = try attachTopology(h);
    var insertion = toolbar + 1;
    while (insertion < h.node_count) : (insertion += 1) {
        var ancestor = h.nodes[insertion].parent;
        while (ancestor != null and ancestor.? != toolbar) ancestor = h.nodes[ancestor.?].parent;
        if (ancestor == null) break;
    }
    if (try attachAdd(h.node_count, 3) > h.nodes.len or try attachAdd(h.child_count, 3) > h.children.len or try attachAdd(h.focus_count, 1) > h.focus.len or try attachAdd(h.used, name.len) > h.arena.len) return error.InputLimit;
    var text_bytes: usize = 0;
    var id_bytes: usize = 0;
    for (h.nodes[0..h.node_count]) |n| {
        text_bytes = try attachAdd(text_bytes, n.name.len);
        text_bytes = try attachAdd(text_bytes, n.description.len);
        text_bytes = try attachAdd(text_bytes, if (n.value) |v| v.len else 0);
        id_bytes = try attachAdd(id_bytes, n.id.bytes.len);
        id_bytes = try attachAdd(id_bytes, if (n.action) |a| a.bytes.len else 0);
    }
    for (h.focus[0..h.focus_count]) |action| id_bytes = try attachAdd(id_bytes, action.bytes.len);
    for (h.announcements[0..h.announcement_count]) |a| {
        text_bytes = try attachAdd(text_bytes, a.code.len);
        text_bytes = try attachAdd(text_bytes, a.message.len);
    }
    text_bytes = try attachAdd(text_bytes, 65 + @as(usize, name.len) * 2);
    const ids = marker_ids[value.world];
    id_bytes = try attachAdd(id_bytes, ids.primary.len + ids.label.len + ids.value.len + ids.action.len * 2);
    if (text_bytes > text_limit or id_bytes > limits.max_id_bytes) return error.InputLimit;
    var triplet = [3]retained.NodeSpec{ markerNode(ids.primary, .button, "START HERE", toolbar, value.label), markerNode(ids.label, .text, "START HERE", insertion, value.label), markerNode(ids.value, .text, "", insertion, value.label) };
    triplet[0].description = marker_tip;
    triplet[0].action = .{ .bytes = ids.action };
    triplet[0].focusable = true;
    return .{ .triplet = triplet, .visual = value, .name = name, .insertion = insertion, .toolbar = toolbar, .text_bytes = text_bytes, .id_bytes = id_bytes };
}
/// Caller ends every immutable input/snapshot lease before this transaction.
pub fn attachStartMarker(prepared: *PreparedProjection, result: marker.Result) AttachError!void {
    const h = prepared.header orelse return error.EmptyProjection;
    if (h.settings) return error.WrongProjectionMode;
    if (h.published) return error.AlreadyPublished;
    if (h.marker_attached) return error.AlreadyAttached;
    switch (result) {
        .absent => {
            _ = try attachTopology(h);
            h.marker_attached = true;
        },
        .marker => |value| {
            var stage = try preflightMarker(h, value, limits.max_text_bytes);
            // Everything below is infallible; no allocations, retains or helpers.
            const name_start = h.used;
            @memcpy(h.arena[name_start..][0..stage.name.len], stage.name.asSlice());
            h.used += stage.name.len;
            const copied_name = h.arena[name_start..h.used];
            stage.triplet[0].value = copied_name;
            stage.triplet[2].name = copied_name;
            var i = h.node_count;
            while (i > stage.insertion) {
                i -= 1;
                h.nodes[i + 3] = h.nodes[i];
                if (h.nodes[i + 3].parent) |parent| if (parent >= stage.insertion) {
                    h.nodes[i + 3].parent = parent + 3;
                };
            }
            @memcpy(h.nodes[stage.insertion..][0..3], &stage.triplet);
            h.node_count += 3;
            h.child_count = 0;
            for (h.nodes[0..h.node_count], 0..) |*parent, parent_index| {
                parent.child_start = h.child_count;
                parent.child_count = 0;
                for (h.nodes[0..h.node_count], 0..) |child, j| if (child.parent == parent_index) {
                    h.children[h.child_count] = j;
                    h.child_count += 1;
                    parent.child_count += 1;
                };
            }
            h.focus[h.focus_count] = stage.triplet[0].action.?;
            h.focus_count += 1;
            h.marker_visual = stage.visual;
            h.marker_attached = true;
        },
    }
}

// Private checked fixtures; no production Header or mutation seam is exported.
fn initializedFixture(h: *Header) void {
    for (h.nodes[h.node_count..]) |*n| n.* = markerNode("", .text, "", 0, .{ .min = .{ 0, 0 }, .max = .{ 1, 1 } });
    @memset(h.children[h.child_count..], 0);
    for (h.focus[h.focus_count..]) |*f| f.* = .{ .bytes = "" };
    for (h.announcements[h.announcement_count..]) |*a| a.* = .{ .kind = .status, .code = "", .message = "" };
    @memset(h.context[h.context_len..], 0);
    @memset(h.arena[h.used..], 0);
    const saved = h.*;
    // Zero every padding byte, then restore fields recursively rather than
    // copying aggregate padding back from the prepared source.
    restoreInitialized(Header, h, saved);
}
fn restoreInitialized(comptime T: type, dest: *T, value: T) void {
    @memset(std.mem.asBytes(dest), 0);
    switch (@typeInfo(T)) {
        .@"struct" => inline for (@typeInfo(T).@"struct".field_names) |field| {
            restoreInitialized(@FieldType(T, field), &@field(dest, field), @field(value, field));
        },
        .array => for (dest, value) |*element, old| restoreInitialized(@TypeOf(old), element, old),
        .optional => {
            dest.* = null;
            if (value) |v| {
                dest.* = v;
                if (dest.*) |*payload| restoreInitialized(@TypeOf(v), payload, v);
            }
        },
        else => dest.* = value,
    }
}
fn testMarker() marker.Marker {
    return .{ .world = 0, .name = model.WorldName.init("ABCDEFGHIJKLMNOPQRSTUVWX") catch unreachable, .point = .{ .x = 400, .y = 400 }, .label = .{ .min = .{ 173.8, 225.64 }, .max = .{ 173.8 + @as(f32, 180), 225.64 + @as(f32, 54) } }, .mode = .action_only, .connector_start = null, .ring_radius = null };
}
fn expectAttachUnchanged(p: *PreparedProjection, result: marker.Result, expected: AttachError) !void {
    const h = p.header.?;
    var bytes: [HEADER_BYTES]u8 = undefined;
    @memcpy(&bytes, std.mem.asBytes(h));
    const ledger = h.budget.ledger();
    const token = h.resident;
    try std.testing.expectError(expected, attachStartMarker(p, result));
    try std.testing.expectEqualSlices(u8, &bytes, std.mem.asBytes(h));
    try std.testing.expectEqualDeep(ledger, h.budget.ledger());
    try std.testing.expect(token.sameIdentity(h.resident));
}
test "H3c full initialized Header rollback malformed marker topology counters and capacities" {
    var tokens = retained.LiveTokenBudget.init(.{ .max_live_objects = 1, .max_live_requested_bytes = 1280 });
    defer tokens.deinit();
    const owner = try retained.TokenRef.create(std.testing.allocator, &tokens);
    defer owner.release();
    var budget = retained.PreparationBudget.init(1179648);
    defer budget.deinit();
    const current = CurrentStamp{ .revision = 1, .owner = .{ .resident = owner, .hold = null, .context = "classic", .draft = null } };
    var p = try prepareHudSettings(std.testing.allocator, &budget, .{ .onboarding_step = .select_union_world, .help_visible = true }, current);
    defer p.deinit();
    initializedFixture(p.header.?);
    const h = p.header.?;
    var bad = testMarker();
    bad.world = 7;
    try expectAttachUnchanged(&p, .{ .marker = bad }, error.InvalidMarker);
    bad = testMarker();
    bad.name.len = 25;
    try expectAttachUnchanged(&p, .{ .marker = bad }, error.InvalidWorldName);
    bad = testMarker();
    bad.name.len = 0;
    try expectAttachUnchanged(&p, .{ .marker = bad }, error.InvalidWorldName);
    for ([_][]const u8{ "lower", " A", "A ", "A  B", "é" }) |name| {
        bad = testMarker();
        bad.name.len = @intCast(name.len);
        @memcpy(bad.name.bytes[0..name.len], name);
        try expectAttachUnchanged(&p, .{ .marker = bad }, error.InvalidWorldName);
    }
    bad = testMarker();
    bad.point.x = std.math.nan(f32);
    try expectAttachUnchanged(&p, .{ .marker = bad }, error.NonFinite);
    bad = testMarker();
    bad.label.max[0] += 1;
    try expectAttachUnchanged(&p, .{ .marker = bad }, error.InvalidMarker);
    bad = testMarker();
    bad.label.min[0] = std.math.floatMax(f32);
    bad.label.max[0] = bad.label.min[0];
    try expectAttachUnchanged(&p, .{ .marker = bad }, error.Overflow);
    bad = testMarker();
    bad.connector_start = .{ .x = 1, .y = 1 };
    try expectAttachUnchanged(&p, .{ .marker = bad }, error.InvalidMarker);
    bad = testMarker();
    bad.ring_radius = 22;
    try expectAttachUnchanged(&p, .{ .marker = bad }, error.InvalidMarker);
    bad = testMarker();
    bad.mode = .scene_linked;
    try expectAttachUnchanged(&p, .{ .marker = bad }, error.InvalidMarker);
    bad.connector_start = .{ .x = bad.label.min[0] + 90, .y = bad.label.min[1] + 54 };
    bad.ring_radius = 21;
    try expectAttachUnchanged(&p, .{ .marker = bad }, error.InvalidMarker);
    bad.ring_radius = std.math.inf(f32);
    try expectAttachUnchanged(&p, .{ .marker = bad }, error.NonFinite);
    const old_parent = h.nodes[2].parent;
    h.nodes[2].parent = 72;
    try expectAttachUnchanged(&p, .{ .marker = testMarker() }, error.InvalidSource);
    h.nodes[2].parent = old_parent;
    const child = h.children[0];
    h.children[0] = 71;
    try expectAttachUnchanged(&p, .{ .marker = testMarker() }, error.InvalidSource);
    h.children[0] = child;
    const old_id = h.nodes[1].id;
    h.nodes[1].id = .{ .bytes = "wrong" };
    try expectAttachUnchanged(&p, .{ .marker = testMarker() }, error.InvalidSource);
    h.nodes[1].id = old_id;
    const old_start = h.nodes[1].child_start;
    h.nodes[1].child_start = 71;
    try expectAttachUnchanged(&p, .{ .marker = testMarker() }, error.InvalidSource);
    h.nodes[1].child_start = old_start;
    const old_focus = h.focus[0];
    h.focus[0] = .{ .bytes = "forged" };
    try expectAttachUnchanged(&p, .{ .marker = testMarker() }, error.InvalidSource);
    h.focus[0] = old_focus;
    const old_used = h.used;
    h.used = h.arena.len;
    try expectAttachUnchanged(&p, .{ .marker = testMarker() }, error.InputLimit);
    h.used = old_used;
    const old_count = h.node_count;
    h.node_count = 73;
    try expectAttachUnchanged(&p, .{ .marker = testMarker() }, error.InvalidSource);
    h.node_count = old_count;
    const old_children = h.child_count;
    h.child_count = 72;
    try expectAttachUnchanged(&p, .{ .marker = testMarker() }, error.InvalidSource);
    h.child_count = old_children;
    const old_context = h.context_len;
    h.context_len = 129;
    try expectAttachUnchanged(&p, .{ .marker = testMarker() }, error.InvalidSource);
    h.context_len = old_context;
    const old_announcements = h.announcement_count;
    h.announcement_count = 3;
    try expectAttachUnchanged(&p, .{ .marker = testMarker() }, error.InvalidSource);
    h.announcement_count = old_announcements;
    h.focus_count = 19;
    try expectAttachUnchanged(&p, .{ .marker = testMarker() }, error.InvalidSource);
    h.focus_count = 17;
    h.settings = true;
    try expectAttachUnchanged(&p, .{ .marker = testMarker() }, error.WrongProjectionMode);
    h.settings = false;
    h.published = true;
    try expectAttachUnchanged(&p, .{ .marker = testMarker() }, error.AlreadyPublished);
    h.published = false;
    try attachStartMarker(&p, .{ .absent = .no_label_candidate });
    try std.testing.expect(h.marker_visual == null);
    try std.testing.expectEqual(old_count, h.node_count);
    try expectAttachUnchanged(&p, .{ .marker = testMarker() }, error.AlreadyAttached);
    var empty = PreparedProjection{};
    try std.testing.expectError(error.EmptyProjection, empty.geometry());
    try std.testing.expectError(error.EmptyProjection, attachStartMarker(&empty, .{ .absent = .not_initial }));
}
test "H3c independently measured B plus113 passes plus112 rejects no fabricated source" {
    var tokens = retained.LiveTokenBudget.init(.{ .max_live_objects = 1, .max_live_requested_bytes = 1280 });
    defer tokens.deinit();
    const owner = try retained.TokenRef.create(std.testing.allocator, &tokens);
    defer owner.release();
    var budget = retained.PreparationBudget.init(1179648);
    defer budget.deinit();
    var p = try prepareHudSettings(std.testing.allocator, &budget, .{ .onboarding_step = .select_union_world }, .{ .revision = 1, .owner = .{ .resident = owner, .hold = null, .context = "classic", .draft = null } });
    defer p.deinit();
    initializedFixture(p.header.?);
    var b: usize = 0;
    for (p.input().nodes) |n| b += n.name.len + n.description.len + (if (n.value) |v| v.len else 0);
    for (p.input().announcements) |announcement| b += announcement.code.len + announcement.message.len;
    var original: [HEADER_BYTES]u8 = undefined;
    @memcpy(&original, std.mem.asBytes(p.header.?));
    const accepted = try preflightMarker(p.header.?, testMarker(), b + 113);
    try std.testing.expectEqual(b + 113, accepted.text_bytes);
    try std.testing.expectError(error.InputLimit, preflightMarker(p.header.?, testMarker(), b + 112));
    try std.testing.expectEqualSlices(u8, &original, std.mem.asBytes(p.header.?));
    std.debug.print("H3c independent B={d} marker text=113 ID=90; B+113={d} pass B+112={d} fail\n", .{ b, b + 113, b + 112 });
    try attachStartMarker(&p, .{ .marker = testMarker() });
    var slot = retained.FrameSlot.init(std.testing.allocator, &budget);
    defer slot.deinit();
    var revision: ?u64 = null;
    try publishHudSettings(&slot, &revision, &p, .{ .revision = 1, .owner = p.input().owner });
    var marker_text: usize = 0;
    for (slot.view().?.nodes) |n| if (std.mem.startsWith(u8, n.id.bytes, "c/hud/marker/")) {
        marker_text += n.name.len + n.description.len + (if (n.value) |value| value.len else 0);
    };
    try std.testing.expectEqual(@as(usize, 113), marker_text);
}

fn expectHudFocusUnchanged(p: *PreparedProjection, slot: *const retained.FrameSlot, focused: ?retained.ActionId, expected: SetHudFocusError) !void {
    const h = p.header.?;
    var bytes: [HEADER_BYTES]u8 = undefined;
    @memcpy(&bytes, std.mem.asBytes(h));
    const ledger = h.budget.ledger();
    const pending = h.budget.pending_bytes;
    const reservations = h.budget.active_reservations;
    const old_view = slot.view();
    var old_focus: [64]u8 = undefined;
    var old_focus_len: ?usize = null;
    if (old_view) |view| if (view.focus.focused) |id| {
        @memcpy(old_focus[0..id.bytes.len], id.bytes);
        old_focus_len = id.bytes.len;
    };
    try std.testing.expectError(expected, p.setHudPublishedFocus(focused));
    try std.testing.expectEqualDeep(old_view, slot.view());
    if (old_focus_len) |len| try std.testing.expectEqualStrings(old_focus[0..len], slot.view().?.focus.focused.?.bytes);
    try std.testing.expectEqualSlices(u8, &bytes, std.mem.asBytes(h));
    try std.testing.expectEqualDeep(ledger, h.budget.ledger());
    try std.testing.expectEqual(pending, h.budget.pending_bytes);
    try std.testing.expectEqual(reservations, h.budget.active_reservations);
}
test "I3 HUD focus setter whole initialized Header rejection and flag precedence" {
    var tokens = retained.LiveTokenBudget.init(.{ .max_live_objects = 1, .max_live_requested_bytes = 1280 });
    defer tokens.deinit();
    const owner = try retained.TokenRef.create(std.testing.allocator, &tokens);
    defer owner.release();
    var budget = retained.PreparationBudget.init(196608);
    defer budget.deinit();
    const current = CurrentStamp{ .revision = 1, .owner = .{ .resident = owner, .hold = null, .context = "classic", .draft = null } };
    const invalid = retained.ActionId{ .bytes = "a/hud/launch\xff" };
    var old = try prepareHudSettings(std.testing.allocator, &budget, .{ .hud_focus = .toggle_pause }, current);
    defer old.deinit();
    var slot = retained.FrameSlot.init(std.testing.allocator, &budget);
    defer slot.deinit();
    var revision: ?u64 = null;
    try publishHudSettings(&slot, &revision, &old, current);
    var empty = PreparedProjection{};
    try std.testing.expectError(error.EmptyProjection, empty.setHudPublishedFocus(invalid));
    try std.testing.expect(empty.header == null);
    try std.testing.expectEqualStrings("a/hud/pause", slot.view().?.focus.focused.?.bytes);
    var settings = try prepareHudSettings(std.testing.allocator, &budget, .{ .mode = .settings }, current);
    defer settings.deinit();
    initializedFixture(settings.header.?);
    try expectHudFocusUnchanged(&settings, &slot, invalid, error.WrongProjectionMode);
    try expectHudFocusUnchanged(&settings, &slot, null, error.WrongProjectionMode);
    var p = try prepareHudSettings(std.testing.allocator, &budget, .{ .hud_focus = .toggle_pause }, current);
    defer p.deinit();
    initializedFixture(p.header.?);
    try expectHudFocusUnchanged(&p, &slot, invalid, error.InvalidUtf8);
    // Full-slice UTF8 validation must precede missing membership.
    try expectHudFocusUnchanged(&p, &slot, .{ .bytes = "unrelated\xf0\x80\x80\x80" }, error.InvalidUtf8);
    for ([_][]const u8{ "", "a/hud/marker/3", "a/hud/marker/4", "a/hud/skip", "a/hud/reset_guidance", "a/hud/launch/extra", "a/hud/launc", "a/hud/launch\x00" }) |id| {
        try expectHudFocusUnchanged(&p, &slot, .{ .bytes = id }, error.InvalidFocus);
    }
    var m = testMarker();
    m.world = 3;
    try attachStartMarker(&p, .{ .marker = m });
    initializedFixture(p.header.?);
    try p.setHudPublishedFocus(.{ .bytes = "a/hud/marker/3" });
    try expectHudFocusUnchanged(&p, &slot, .{ .bytes = "a/hud/marker/4" }, error.InvalidFocus);
    try expectAttachUnchanged(&p, .{ .marker = m }, error.AlreadyAttached);
    try p.setHudPublishedFocus(null);
    try std.testing.expect(p.input().focus.focused == null);
    try p.setHudPublishedFocus(.{ .bytes = "a/hud/launch" });
    try publishHudSettings(&slot, &revision, &settings, current);
    try expectHudFocusUnchanged(&settings, &slot, invalid, error.WrongProjectionMode);
    try publishHudSettings(&slot, &revision, &p, current);
    try expectHudFocusUnchanged(&p, &slot, invalid, error.AlreadyPublished);
    try expectHudFocusUnchanged(&p, &slot, null, error.AlreadyPublished);
    try expectHudFocusUnchanged(&p, &slot, .{ .bytes = "a/hud/marker/4" }, error.AlreadyPublished);
    try expectAttachUnchanged(&p, .{ .marker = m }, error.AlreadyPublished);
    slot.deinit();
    slot = retained.FrameSlot.init(std.testing.allocator, &budget);
    try expectHudFocusUnchanged(&p, &slot, null, error.AlreadyPublished);
}
