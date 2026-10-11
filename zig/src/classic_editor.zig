//! Detached Classic text editor. Move-only owner; deinit exactly once.
const std = @import("std");
const scenario = @import("scenario.zig");
const model = @import("model.zig");
pub const Section = enum { worlds, rules, validation };
pub const GlobalField = enum { seed, tick_hz, win_world_count, ai_period_ticks, hazard_first_tick, hazard_period_ticks, hazard_duration_ticks, maximum_energy, maximum_defense, minimum_launch, field_raise_cost, field_lower_refund, base_fleet_speed };
pub const WorldField = enum { name, x, y, owner, energy, defense, base_output, base_regeneration, atmosphere, hydrosphere, topology };
pub const FieldId = union(enum) {
    global: GlobalField,
    world: struct { index: u3, field: WorldField },
    pub fn slot(self: FieldId) usize {
        return switch (self) {
            .global => |f| @backingInt(f),
            .world => |w| 13 + @as(usize, w.index) * 11 + @backingInt(w.field),
        };
    }
};
pub const Field = struct { id: FieldId, text_id: []const u8, editable: bool };
pub const catalog: [90]Field = blk: {
    @setEvalBranchQuota(100000);
    var fields: [90]Field = undefined;
    for (std.meta.tags(GlobalField), 0..) |f, i| fields[i] = .{ .id = .{ .global = f }, .text_id = if (f == .seed) "scenario.seed" else "rule." ++ @tagName(f), .editable = f != .tick_hz };
    for (0..7) |i| for (std.meta.tags(WorldField), 0..) |f, j| {
        fields[13 + i * 11 + j] = .{ .id = .{ .world = .{ .index = i, .field = f } }, .text_id = std.fmt.comptimePrint("world.{d}.{s}", .{ i, @tagName(f) }), .editable = true };
    };
    break :blk fields;
};
pub fn lookup(text_id: []const u8) ?Field {
    for (catalog) |field| if (std.mem.eql(u8, field.text_id, text_id)) return field;
    return null;
}
pub const Action = union(enum) { select_section: Section, select_world: usize, revert, factory_defaults, regenerate_from_seed, load, save, cancel, apply_and_restart, confirm, dismiss_confirmation };
pub const Focus = union(enum) { field: FieldId, action: Action };
/// Pure view. ORIGINAL names survive until deinit; current names/buffers/preedit
/// survive until the next successful content mutation. No allocator is exposed.
pub const DraftView = struct { seed: u64, rules: model.RulesV1, worlds: [7]scenario.WorldDraft };
const Epoch = struct {
    draft: scenario.ScenarioDraft,
    buffers: [90][]const u8,
    errors: [90]bool = @splat(false),
    preedit: []const u8 = "",
    fn deinit(self: *Epoch) void {
        self.draft.deinit();
    }
};
pub const ModalKind = enum { revert, factory_defaults, regenerate_from_seed, load_dirty, overwrite_save, cancel_dirty, apply_and_restart };
/// Move-only captures, independent of current epochs. begin adopts save bytes
/// allocated with allocator; staged 3D preparation must succeed before begin.
pub const ModalState = struct {
    allocator: std.mem.Allocator,
    kind: ?ModalKind = null,
    save_bytes: ?[]u8 = null,
    apply_candidate: ?scenario.ScenarioV1 = null,
    pub fn begin(self: *ModalState, kind: ModalKind, owned_save: ?[]u8, candidate_value: ?scenario.ScenarioV1) void {
        self.deinit();
        self.kind = kind;
        self.save_bytes = owned_save;
        self.apply_candidate = candidate_value;
    }
    pub fn dismiss(self: *ModalState) void {
        self.deinit();
    }
    pub fn deinit(self: *ModalState) void {
        if (self.save_bytes) |bytes| self.allocator.free(bytes);
        self.save_bytes = null;
        self.apply_candidate = null;
        self.kind = null;
    }
};
pub const EditorState = struct {
    modal: ModalState,
    base_allocator: std.mem.Allocator,
    _original: scenario.ScenarioDraft,
    _current: Epoch,
    section: Section = .worlds,
    selected_world: usize = 0,
    focus: ?Focus = null,
    scroll: f32 = 0,
    pub fn init(allocator: std.mem.Allocator, borrowed: *const scenario.ScenarioDraft) !EditorState {
        try checkNames(borrowed);
        var initial = try borrowed.clone(allocator);
        errdefer initial.deinit();
        const live = try buildEpoch(allocator, borrowed);
        return .{ .modal = .{ .allocator = allocator }, .base_allocator = allocator, ._original = initial, ._current = live };
    }
    pub fn deinit(self: *EditorState) void {
        self.modal.deinit();
        self._original.deinit();
        self._current.deinit();
        self.* = undefined;
    }
    pub fn current(self: *const EditorState) DraftView {
        return .{ .seed = self._current.draft.seed, .rules = self._current.draft.rules, .worlds = self._current.draft.worlds };
    }
    pub fn original(self: *const EditorState) DraftView {
        return .{ .seed = self._original.seed, .rules = self._original.rules, .worlds = self._original.worlds };
    }
    pub fn fieldText(self: *const EditorState, id: []const u8) ?[]const u8 {
        const f = lookup(id) orelse return null;
        return self._current.buffers[f.id.slot()];
    }
    /// Low-level 3D publication seam: no validation or request guard. It retains
    /// ORIGINAL, focus, section/world, preedit, scroll and modal captures.
    /// 3D owns confirmation policy and may regenerate semantic-invalid drafts.
    pub fn replaceCurrent(self: *EditorState, borrowed: *const scenario.ScenarioDraft) !void {
        const prepared = try prepareEpoch(self.base_allocator, borrowed, @splat(false), @splat(""), self._current.preedit);
        self._current.deinit();
        self._current = prepared;
    }
    /// Independent owned copies for staged 3D preparation; caller deinits once.
    pub fn copyOriginal(self: *const EditorState, allocator: std.mem.Allocator) !scenario.ScenarioDraft {
        return self._original.clone(allocator);
    }
    pub fn copyCurrent(self: *const EditorState, allocator: std.mem.Allocator) !scenario.ScenarioDraft {
        return self._current.draft.clone(allocator);
    }
    pub fn selectSection(self: *EditorState, section: Section) void {
        if (self.modal.kind != null or self.section == section) return;
        self.section = section;
        self.focus = null;
        self.scroll = 0;
    }
    pub fn selectWorld(self: *EditorState, index: usize) void {
        if (self.modal.kind != null or index >= 7) return;
        self.section = .worlds;
        self.selected_world = index;
        self.focus = null;
        self.scroll = 0;
    }
    /// Caller admits enabled real action widgets in 3H; no traversal is invented.
    pub fn focusAction(self: *EditorState, action: Action) void {
        if (self.modal.kind == null) self.focus = .{ .action = action };
    }
    pub fn focusedAcceptsText(self: *const EditorState) bool {
        return self.focusedField() != null;
    }
    pub fn focusedText(self: *const EditorState) ?[]const u8 {
        const f = self.focusedField() orelse return null;
        return self._current.buffers[f.slot()];
    }
    pub fn preedit(self: *const EditorState) []const u8 {
        return self._current.preedit;
    }
    pub fn insertFocusedText(self: *EditorState, value: []const u8) !void {
        if (self.modal.kind != null) return;
        const id = self.focusedField() orelse return;
        try self.append(id, value, self._current.preedit);
    }
    fn append(self: *EditorState, id: FieldId, value: []const u8, next_preedit: []const u8) !void {
        if (!std.unicode.utf8ValidateSlice(value)) return error.InvalidUtf8;
        // Bounded temporary text is released whether epoch staging succeeds or fails.
        const joined = try std.mem.concat(self.base_allocator, u8, &.{ self._current.buffers[id.slot()], value });
        defer self.base_allocator.free(joined);
        try self.edit(id, joined, next_preedit);
    }
    pub fn backspaceFocused(self: *EditorState) !void {
        if (self.modal.kind != null) return;
        const id = self.focusedField() orelse return;
        const text = self._current.buffers[id.slot()];
        var end = text.len;
        if (end > 0) {
            end -= 1;
            while (end > 0 and text[end] & 0xc0 == 0x80) end -= 1;
        }
        try self.edit(id, text[0..end], self._current.preedit);
    }
    pub fn setPreedit(self: *EditorState, value: []const u8) !void {
        if (self.modal.kind != null) return;
        if (!std.unicode.utf8ValidateSlice(value)) return error.InvalidUtf8;
        try self.replacePreedit(value);
    }
    fn replacePreedit(self: *EditorState, value: []const u8) !void {
        const prepared = try prepareEpoch(self.base_allocator, &self._current.draft, self._current.errors, self._current.buffers, value);
        self._current.deinit();
        self._current = prepared;
    }
    pub fn commitIme(self: *EditorState, value: []const u8) !void {
        if (self.modal.kind != null) return;
        if (self.focusedField()) |id| try self.append(id, value, "") else try self.replacePreedit("");
    }
    pub fn replaceFocusedText(self: *EditorState, value: []const u8) !void {
        if (self.modal.kind != null) {
            self.modal.dismiss();
            return;
        }
        const id = self.focusedField() orelse return;
        if (!std.unicode.utf8ValidateSlice(value)) return error.InvalidUtf8;
        try self.edit(id, value, self._current.preedit);
    }
    fn focusedField(self: *const EditorState) ?FieldId {
        return if (self.focus) |focus| switch (focus) {
            .field => |field| field,
            .action => null,
        } else null;
    }
    fn edit(self: *EditorState, id: FieldId, value: []const u8, next_preedit: []const u8) !void {
        var borrowed = self._current.draft;
        var flags = self._current.errors;
        var buffers = self._current.buffers;
        flags[id.slot()] = !assign(&borrowed, id, value);
        buffers[id.slot()] = value;
        const prepared = try prepareEpoch(self.base_allocator, &borrowed, flags, buffers, next_preedit);
        self._current.deinit();
        self._current = prepared;
    }
    pub fn fieldHasParseError(self: *const EditorState, id: FieldId) error{InvalidField}!bool {
        switch (id) {
            .world => |w| if (w.index > 6) return error.InvalidField,
            .global => {},
        }
        return self._current.errors[id.slot()];
    }
    pub fn hasParseErrors(self: *const EditorState) bool {
        for (self._current.errors) |flag| if (flag) return true;
        return false;
    }
    pub fn isDirty(self: *const EditorState) bool {
        return self.hasParseErrors() or !draftEqual(&self._original, &self._current.draft);
    }
    pub fn validationReport(self: *const EditorState) scenario.ValidationReport {
        return self._current.draft.validationReport();
    }
    pub fn canApply(self: *const EditorState) bool {
        const report = self.validationReport();
        return !self.hasParseErrors() and report.isValid();
    }
    pub fn candidate(self: *const EditorState) error{ InvalidText, Validation }!scenario.ScenarioV1 {
        if (self.hasParseErrors()) return error.InvalidText;
        return self._current.draft.validated();
    }
    pub fn focusField(self: *EditorState, id: []const u8) error{UnknownWidget}!void {
        if (self.modal.kind != null) return error.UnknownWidget;
        const f = lookup(id) orelse return error.UnknownWidget;
        if (!f.editable) return error.UnknownWidget;
        switch (f.id) {
            .global => if (self.section != .rules) return error.UnknownWidget,
            .world => |w| if (self.section != .worlds or self.selected_world != w.index) return error.UnknownWidget,
        }
        self.focus = .{ .field = f.id };
    }
};

fn checkNames(draft: *const scenario.ScenarioDraft) error{InvalidUtf8}!void {
    for (draft.worlds) |world| if (!std.unicode.utf8ValidateSlice(world.name)) return error.InvalidUtf8;
}
fn buildEpoch(allocator: std.mem.Allocator, draft: *const scenario.ScenarioDraft) !Epoch {
    return prepareEpoch(allocator, draft, @splat(false), @splat(""), "");
}
fn formatted(draft: *scenario.ScenarioDraft, id: FieldId) ![]const u8 {
    const allocator = draft.arena.allocator();
    return switch (id) {
        .global => |f| switch (f) {
            .seed => try std.fmt.allocPrint(allocator, "{X:0>16}", .{draft.seed}),
            inline else => |tag| try std.fmt.allocPrint(allocator, "{d}", .{@field(draft.rules, @tagName(tag))}),
        },
        .world => |w| switch (w.field) {
            .name => draft.worlds[w.index].name,
            .owner => try allocator.dupe(u8, if (draft.worlds[w.index].owner) |owner| switch (owner) {
                .union_faction => "UNION",
                .helix => "HELIX",
                .choir => "CHOIR",
            } else "NEUTRAL"),
            inline else => |tag| try std.fmt.allocPrint(allocator, "{d}", .{@field(draft.worlds[w.index], @tagName(tag))}),
        },
    };
}

fn prepareEpoch(allocator: std.mem.Allocator, borrowed: *const scenario.ScenarioDraft, flags: [90]bool, buffers: [90][]const u8, preedit: []const u8) !Epoch {
    try checkNames(borrowed);
    var epoch = Epoch{ .draft = try borrowed.clone(allocator), .buffers = undefined, .errors = flags };
    errdefer epoch.deinit();
    for (catalog) |field| {
        const i = field.id.slot();
        epoch.buffers[i] = if (flags[i]) try epoch.draft.arena.allocator().dupe(u8, buffers[i]) else try formatted(&epoch.draft, field.id);
    }
    epoch.preedit = try epoch.draft.arena.allocator().dupe(u8, preedit);
    return epoch;
}
fn draftEqual(a: *const scenario.ScenarioDraft, b: *const scenario.ScenarioDraft) bool {
    if (a.seed != b.seed or !std.meta.eql(a.rules, b.rules)) return false;
    for (a.worlds, b.worlds) |x, y| {
        inline for (comptime std.meta.fieldNames(scenario.WorldDraft)) |field| {
            if (comptime std.mem.eql(u8, field, "name")) {
                if (!std.mem.eql(u8, x.name, y.name)) return false;
            } else if (!std.meta.eql(@field(x, field), @field(y, field))) return false;
        }
    }
    return true;
}
fn unsigned(value: []const u8, radix: u64, limit: u64) ?u64 {
    if (value.len == 0) return null;
    const digits = if (value[0] == '+') value[1..] else value;
    if (digits.len == 0) return null;
    var result: u64 = 0;
    for (digits) |byte| {
        const digit: u64 = switch (byte) {
            '0'...'9' => byte - '0',
            'a'...'f' => if (radix == 16) byte - 'a' + 10 else return null,
            'A'...'F' => if (radix == 16) byte - 'A' + 10 else return null,
            else => return null,
        };
        if (digit >= radix or digit > limit or result > (limit - digit) / radix) return null;
        result = result * radix + digit;
    }
    return result;
}
fn decimal(comptime T: type, value: []const u8) ?T {
    if (T == i32) {
        const negative = value.len > 0 and value[0] == '-';
        const digits = if (negative) value[1..] else value;
        if (negative and digits.len > 0 and digits[0] == '+') return null;
        const magnitude = unsigned(digits, 10, if (negative) 2147483648 else 2147483647) orelse return null;
        const signed: i64 = @intCast(magnitude);
        return @intCast(if (negative) -signed else signed);
    }
    return @intCast(unsigned(value, 10, std.math.maxInt(T)) orelse return null);
}
fn assign(draft: *scenario.ScenarioDraft, id: FieldId, text: []const u8) bool {
    switch (id) {
        .global => |field| switch (field) {
            .seed => {
                if (text.len != 16) return false;
                draft.seed = unsigned(text, 16, std.math.maxInt(u64)) orelse return false;
            },
            .tick_hz => return false,
            inline else => |f| {
                const T = @TypeOf(@field(draft.rules, @tagName(f)));
                @field(draft.rules, @tagName(f)) = decimal(T, text) orelse return false;
            },
        },
        .world => |w| switch (w.field) {
            .name => draft.worlds[w.index].name = text,
            .owner => {
                const owner: ?model.Faction = if (std.mem.eql(u8, text, "NEUTRAL")) null else if (std.mem.eql(u8, text, "UNION")) .union_faction else if (std.mem.eql(u8, text, "HELIX")) .helix else if (std.mem.eql(u8, text, "CHOIR")) .choir else return false;
                draft.worlds[w.index].owner = owner;
            },
            inline else => |f| {
                const T = @TypeOf(@field(draft.worlds[w.index], @tagName(f)));
                @field(draft.worlds[w.index], @tagName(f)) = decimal(T, text) orelse return false;
            },
        },
    }
    return true;
}
