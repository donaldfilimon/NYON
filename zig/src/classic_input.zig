const std = @import("std");
const retained = @import("retained_semantics.zig");
pub const Revision = u64;
pub const Context = struct { bytes: [128]u8, len: u8 };
pub const CurrentView = struct { revision: Revision, owner: retained.OwnerSpec };

/// One coordinator lifetime. Do not copy, restore or reset after producing a
/// CurrentView. Close only after every published/current/event domain expires.
/// External source changes and hidden borrowed copies are caller obligations.
pub const RevisionClock = struct {
    value: Revision = 0,
    closed: bool = false,
    pub fn next(self: *const RevisionClock) error{ RevisionExhausted, ClockClosed }!Revision {
        if (self.closed) return error.ClockClosed;
        return std.math.add(Revision, self.value, 1) catch error.RevisionExhausted;
    }
    pub fn commit(self: *RevisionClock, expected: Revision, next_value: Revision) error{ RevisionConflict, ClockClosed }!void {
        if (self.closed) return error.ClockClosed;
        if (expected != self.value or next_value != (std.math.add(Revision, expected, 1) catch return error.RevisionConflict)) return error.RevisionConflict;
        self.value = next_value;
    }
    pub fn deinit(self: *RevisionClock) void {
        self.closed = true;
    }
};
pub const SlotPurpose = enum { root_resident, root_draft, resident_frame, staged_frame, modal_hold, guide_hold, queued_event, transition };
pub const SlotTicket = struct { registry: *const ReferenceRegistry, index: u8, generation: u64 };
const EntryState = enum { free, reserved, owned };
const RegistryEntry = struct { state: EntryState = .free, purpose: SlotPurpose = .transition, generation: u64 = 0 };
/// UI-thread only. Pin before first reserve; thereafter no independent copy,
/// move, reinit, reset or restoration. Tickets cannot escape terminal lifetime.
/// Address binding rejects distinct live instances, not arbitrary address reuse.
/// Entry fields are implementation state, not a caller mutation API.
pub const ReferenceRegistry = struct {
    entries: [32]RegistryEntry = @splat(.{}),
    closed: bool = false,
    pub fn init() ReferenceRegistry {
        return .{};
    }
    pub fn reserve(self: *ReferenceRegistry, purpose: SlotPurpose) error{ TokenReferenceLimit, TicketExhausted, RegistryClosed }!SlotTicket {
        if (self.closed) return error.RegistryClosed;
        for (&self.entries, 0..) |*entry, index| {
            if (entry.state != .free) continue;
            const generation = std.math.add(u64, entry.generation, 1) catch return error.TicketExhausted;
            entry.* = .{ .state = .reserved, .purpose = purpose, .generation = generation };
            return .{ .registry = self, .index = @intCast(index), .generation = generation };
        }
        return error.TokenReferenceLimit;
    }
    fn checked(self: *ReferenceRegistry, ticket: SlotTicket, state: EntryState) error{InvalidTicket}!*RegistryEntry {
        if (self.closed or ticket.registry != self or ticket.index >= self.entries.len) return error.InvalidTicket;
        const entry = &self.entries[ticket.index];
        if (entry.generation != ticket.generation or entry.state != state) return error.InvalidTicket;
        return entry;
    }
    pub fn markOwned(self: *ReferenceRegistry, ticket: SlotTicket) error{InvalidTicket}!void {
        (try self.checked(ticket, .reserved)).state = .owned;
    }
    pub fn cancel(self: *ReferenceRegistry, ticket: SlotTicket) error{InvalidTicket}!void {
        (try self.checked(ticket, .reserved)).state = .free;
    }
    pub fn release(self: *ReferenceRegistry, ticket: SlotTicket) error{InvalidTicket}!void {
        (try self.checked(ticket, .owned)).state = .free;
    }
    pub fn deinit(self: *ReferenceRegistry) void {
        // Unconditional in every optimization mode; check before closed mutation.
        for (self.entries) |entry| if (entry.state != .free) @panic("classic input registry shutdown with occupied references");
        self.closed = true;
    }
};
pub const SemanticKind = enum { activate, focus, edit_utf8 };
pub const EventSource = struct { current: CurrentView, kind: SemanticKind, action: []const u8, text: []const u8 = "" };
pub const QueueError = error{ OutOfMemory, QueueFull, EventLimit, InvalidUtf8, InvalidPayload, TokenReferenceLimit, TicketExhausted, RegistryClosed, ReferenceOverflow, InvalidTicket };
pub const EventView = struct { current: CurrentView, kind: SemanticKind, action: []const u8, text: []const u8 };
const EventSlot = struct {
    kind: SemanticKind = .activate,
    revision: Revision = 0,
    resident: retained.TokenRef = undefined,
    draft: ?retained.TokenRef = null,
    hold: ?u64 = null,
    resident_ticket: ?SlotTicket = null,
    draft_ticket: ?SlotTicket = null,
    context_len: u16 = 0,
    action_len: u16 = 0,
    text_len: u16 = 0,
    occupied: bool = false,
    bytes: [3840]u8 = @splat(0),
    fn view(self: *const EventSlot) EventView {
        const action_start: usize = self.context_len;
        const text_start = action_start + self.action_len;
        return .{ .current = .{ .revision = self.revision, .owner = .{ .resident = self.resident, .draft = self.draft, .hold = self.hold, .context = self.bytes[0..action_start] } }, .kind = self.kind, .action = self.bytes[action_start..text_start], .text = self.bytes[text_start .. text_start + self.text_len] };
    }
};
const QueueHeader = struct { allocator: std.mem.Allocator, registry: *ReferenceRegistry, slots: [8]EventSlot = @splat(.{}), head: u8 = 0, count: u8 = 0 };
comptime {
    if (@sizeOf(EventSlot) > 4096 or @sizeOf(EventSlot) - 3840 > 256) @compileError("semantic event slot exceeds accepted storage profile");
}
/// Move-only owning handle by convention. Header has one stable allocation.
/// Allocator, registry and token budgets outlive it. front borrows end before
/// any enqueue/drain/clear/deinit. No host delivery or retry policy is implied.
pub const SemanticQueue = struct {
    header: ?*QueueHeader = null,
    pub fn init(allocator: std.mem.Allocator, registry: *ReferenceRegistry) error{OutOfMemory}!SemanticQueue {
        const header = try allocator.create(QueueHeader);
        header.* = .{ .allocator = allocator, .registry = registry };
        return .{ .header = header };
    }
    pub fn enqueue(self: *SemanticQueue, source: EventSource) QueueError!void {
        const h = self.header orelse @panic("classic input enqueue after queue shutdown");
        if (h.count == h.slots.len) return error.QueueFull;
        const owner = source.current.owner;
        // Validate complete producer strings before reserving or retaining.
        if (!std.unicode.utf8ValidateSlice(owner.context) or !std.unicode.utf8ValidateSlice(source.action) or !std.unicode.utf8ValidateSlice(source.text)) return error.InvalidUtf8;
        if (source.action.len == 0 or (source.kind != .edit_utf8 and source.text.len != 0)) return error.InvalidPayload;
        if (owner.context.len > 128 or source.action.len > 64 or source.text.len > 3840) return error.EventLimit;
        const text_start = owner.context.len + source.action.len;
        if (source.text.len > 3840 - text_start) return error.EventLimit;
        const rt = try h.registry.reserve(.queued_event);
        errdefer h.registry.cancel(rt) catch @panic("classic input resident reservation unwind");
        const dt: ?SlotTicket = if (owner.draft != null) try h.registry.reserve(.queued_event) else null;
        errdefer if (dt) |ticket| h.registry.cancel(ticket) catch @panic("classic input draft reservation unwind");
        const resident = try owner.resident.retain();
        errdefer resident.release();
        const draft: ?retained.TokenRef = if (owner.draft) |token| try token.retain() else null;
        errdefer if (draft) |token| token.release();
        // Tickets are known reserved in this single-threaded lifetime. No
        // recoverable operation remains after ownership transfer begins.
        h.registry.markOwned(rt) catch @panic("classic input resident ticket transition");
        if (dt) |ticket| h.registry.markOwned(ticket) catch @panic("classic input draft ticket transition");
        const slot = &h.slots[(@as(usize, h.head) + h.count) % h.slots.len];
        slot.kind = source.kind;
        slot.revision = source.current.revision;
        slot.resident = resident;
        slot.draft = draft;
        slot.hold = owner.hold;
        slot.resident_ticket = rt;
        slot.draft_ticket = dt;
        slot.context_len = @intCast(owner.context.len);
        slot.action_len = @intCast(source.action.len);
        slot.text_len = @intCast(source.text.len);
        @memcpy(slot.bytes[0..owner.context.len], owner.context);
        @memcpy(slot.bytes[owner.context.len..text_start], source.action);
        @memcpy(slot.bytes[text_start .. text_start + source.text.len], source.text);
        slot.occupied = true;
        h.count += 1;
    }
    pub fn front(self: *const SemanticQueue) ?EventView {
        const h = self.header orelse return null;
        return if (h.count == 0) null else h.slots[h.head].view();
    }
    pub fn finishFront(self: *SemanticQueue) void {
        const h = self.header orelse return;
        if (h.count == 0) return;
        const slot = &h.slots[h.head];
        if (slot.draft) |token| token.release();
        if (slot.draft_ticket) |ticket| h.registry.release(ticket) catch @panic("classic input draft ticket release");
        slot.resident.release();
        h.registry.release(slot.resident_ticket.?) catch @panic("classic input resident ticket release");
        slot.occupied = false;
        slot.draft = null;
        slot.resident_ticket = null;
        slot.draft_ticket = null;
        h.count -= 1;
        h.head = @intCast((@as(usize, h.head) + 1) % h.slots.len);
    }
    pub fn clear(self: *SemanticQueue) void {
        while (self.front() != null) self.finishFront();
    }
    pub fn deinit(self: *SemanticQueue) void {
        const h = self.header orelse return;
        self.clear();
        const allocator = h.allocator;
        allocator.destroy(h);
        self.header = null;
    }
};
pub fn matchesCurrent(event: EventView, current: CurrentView) bool {
    return event.current.revision == current.revision and event.current.owner.matches(current.owner);
}

test "foundation private target layout and registry generation exhaustion" {
    try std.testing.expect(@sizeOf(EventSlot) <= 4096);
    try std.testing.expect(@sizeOf(EventSlot) - 3840 <= 256);
    std.debug.print("foundation layout ticket={d} entry={d} registry={d} clock={d} slot={d} metadata={d} slots={d} header={d} overhead={d}\n", .{ @sizeOf(SlotTicket), @sizeOf(RegistryEntry), @sizeOf(ReferenceRegistry), @sizeOf(RevisionClock), @sizeOf(EventSlot), @sizeOf(EventSlot) - 3840, 8 * @sizeOf(EventSlot), @sizeOf(QueueHeader), @sizeOf(QueueHeader) - 8 * @sizeOf(EventSlot) });
    var registry = ReferenceRegistry.init();
    registry.entries[0].generation = std.math.maxInt(u64);
    const before = registry;
    try std.testing.expectError(error.TicketExhausted, registry.reserve(.transition));
    try std.testing.expectEqualDeep(before, registry);
    registry.deinit();
}
test "foundation private second ticket exhaustion unwind preserves occupied slots" {
    var registry = ReferenceRegistry.init();
    var budget = retained.LiveTokenBudget.init(.{ .max_live_objects = 32, .max_live_requested_bytes = 1280 });
    const root_ticket = try registry.reserve(.root_resident);
    const resident = try retained.TokenRef.create(std.testing.allocator, &budget);
    try registry.markOwned(root_ticket);
    var queue = try SemanticQueue.init(std.testing.allocator, &registry);
    const current = CurrentView{ .revision = 1, .owner = .{ .resident = resident, .draft = null, .hold = null, .context = "" } };
    try queue.enqueue(.{ .current = current, .kind = .activate, .action = "first" });
    // First event owns entry1, failed candidate reserves entry2; entry3 cannot
    // wrap. The canceled entry2 generation may advance but usage must unwind.
    registry.entries[3].generation = std.math.maxInt(u64);
    var pair = current;
    pair.owner.draft = resident;
    const before = queue.header.?.slots[queue.header.?.head];
    try std.testing.expectError(error.TicketExhausted, queue.enqueue(.{ .current = pair, .kind = .activate, .action = "second" }));
    try std.testing.expectEqualDeep(before, queue.header.?.slots[queue.header.?.head]);
    try std.testing.expectEqual(EntryState.free, registry.entries[2].state);
    try std.testing.expectEqual(@as(u64, 1), registry.entries[2].generation);
    try std.testing.expectEqual(@as(u64, std.math.maxInt(u64)), registry.entries[3].generation);
    queue.deinit();
    resident.release();
    try registry.release(root_ticket);
    budget.deinit();
    registry.deinit();
}

// Synchronous borrowed normalization helpers. No platform or router effects.
pub const NamedKey = enum { f1, f4, tab, enter, space, escape, arrow_up, arrow_down, arrow_left, arrow_right, page_up, page_down, backspace, delete, home, end, other };
pub const LogicalKey = union(enum) { named: NamedKey, character: []const u8, unsupported: void };
pub const KeyState = enum { pressed, released };
pub const Modifiers = struct { shift: bool = false, control: bool = false, alt: bool = false, super: bool = false };
pub const NormalizedKeyEvent = struct { key: LogicalKey, state: KeyState, repeat: bool, modifiers: Modifiers, text: ?[]const u8 = null };
pub const NormalizedIme = union(enum) { preedit: []const u8, commit: []const u8 };
pub const Action = enum { field1, field2, field3, decrease, increase, launch, pause, speed_down, speed_up, camera_reset, settings, restart, help, cancel, scenario };
pub const NavigationAction = enum { tab_forward, tab_backward, up, down, left, right, page_up, page_down, activate, escape, backspace, delete, home, end };
pub fn validateKeyEvent(event: NormalizedKeyEvent) error{InvalidUtf8}!void {
    switch (event.key) {
        .character => |text| try validateBorrowedUtf8(text),
        else => {},
    }
    if (event.text) |text| try validateBorrowedUtf8(text);
}
pub fn validateIme(event: NormalizedIme) error{InvalidUtf8}!void {
    switch (event) {
        .preedit, .commit => |text| try validateBorrowedUtf8(text),
    }
}
fn validateBorrowedUtf8(text: []const u8) error{InvalidUtf8}!void {
    if (!std.unicode.utf8ValidateSlice(text)) return error.InvalidUtf8;
}
/// Requires validated input; compares the complete string, without retaining it.
pub fn actionForKey(key: LogicalKey) ?Action {
    return switch (key) {
        .named => |named| switch (named) {
            .space => .launch,
            .escape => .cancel,
            .f4 => .scenario,
            else => null,
        },
        .character => |text| blk: {
            if (text.len != 1) break :blk null;
            break :blk switch (std.ascii.toLower(text[0])) {
                '1' => .field1,
                '2' => .field2,
                '3' => .field3,
                'q' => .decrease,
                'e' => .increase,
                'p' => .pause,
                '[' => .speed_down,
                ']' => .speed_up,
                'c' => .camera_reset,
                's' => .settings,
                'r' => .restart,
                'h' => .help,
                else => null,
            };
        },
        .unsupported => null,
    };
}
pub fn navigationForKey(key: LogicalKey, modifiers: Modifiers) ?NavigationAction {
    return switch (key) {
        .named => |named| switch (named) {
            .tab => if (modifiers.shift) .tab_backward else .tab_forward,
            .arrow_up => .up,
            .arrow_down => .down,
            .arrow_left => .left,
            .arrow_right => .right,
            .page_up => .page_up,
            .page_down => .page_down,
            .enter, .space => .activate,
            .escape => .escape,
            .backspace => .backspace,
            .delete => .delete,
            .home => .home,
            .end => .end,
            else => null,
        },
        else => null,
    };
}
pub fn editorPrintableText(text: []const u8) error{InvalidUtf8}!bool {
    try validateBorrowedUtf8(text);
    var iter = (std.unicode.Utf8View.initUnchecked(text)).iterator();
    while (iter.nextCodepoint()) |scalar| {
        if (scalar <= 0x1f or (scalar >= 0x7f and scalar <= 0x9f)) return false;
    }
    return true;
}
/// Action-level sets intentionally coalesce aliases and pending edges.
/// Events and borrowed key/text/IME slices are never stored here.
pub const InputState = struct {
    held: [15]bool = @splat(false),
    pressed: [15]bool = @splat(false),
    pub fn init() InputState {
        return .{};
    }
    pub fn setAction(self: *InputState, action: Action, down: bool) void {
        const index = @backingInt(action);
        if (down and !self.held[index]) self.pressed[index] = true;
        self.held[index] = down;
    }
    pub fn isHeld(self: *const InputState, action: Action) bool {
        return self.held[@backingInt(action)];
    }
    pub fn takePressedOnce(self: *InputState, action: Action) bool {
        const index = @backingInt(action);
        const result = self.pressed[index];
        self.pressed[index] = false;
        return result;
    }
    pub fn clearTransient(self: *InputState) void {
        self.pressed = @splat(false);
    }
    pub fn clearAll(self: *InputState) void {
        self.* = init();
    }
    pub fn finishFrame(self: *InputState) void {
        self.clearTransient();
    }
};
comptime {
    std.debug.assert(@typeInfo(Action).@"enum".field_names.len == 15);
    std.debug.assert(@typeInfo(@FieldType(InputState, "held")).array.len == 15);
    std.debug.assert(@FieldType(InputState, "held") == [15]bool);
    std.debug.assert(@FieldType(InputState, "pressed") == [15]bool);
    std.debug.assert(@typeInfo(InputState).@"struct".field_names.len == 2);
}
