const std = @import("std");
pub const BudgetError = error{LimitExceeded};
pub const TokenCreateError = error{ OutOfMemory, LimitExceeded };
pub const TokenRetainError = error{ReferenceOverflow};
pub const TokenLimits = struct { max_live_objects: usize, max_live_requested_bytes: usize };
pub const TokenUsage = struct { live_objects: usize = 0, live_requested_bytes: usize = 0, peak_objects: usize = 0, peak_requested_bytes: usize = 0 };
fn contract(ok: bool) void {
    if (!ok) @panic("retained ownership contract violation");
}
fn add(a: usize, b: usize) BudgetError!usize {
    return std.math.add(usize, a, b) catch error.LimitExceeded;
}
fn subtract(a: usize, b: usize) usize {
    contract(a >= b);
    return a - b;
}
/// UI-thread capability. Its address and allocator context must stay stable
/// until every token is released. No defaults or consumer admission policy.
pub const LiveTokenBudget = struct {
    limits: TokenLimits,
    current: TokenUsage = .{},
    pending_objects: usize = 0,
    pending_bytes: usize = 0,
    pub fn init(limits: TokenLimits) LiveTokenBudget {
        return .{ .limits = limits };
    }
    pub fn usage(self: *const LiveTokenBudget) TokenUsage {
        return self.current;
    }
    pub fn deinit(self: *LiveTokenBudget) void {
        contract(self.current.live_objects == 0 and self.current.live_requested_bytes == 0 and self.pending_objects == 0 and self.pending_bytes == 0);
    }
    fn reserve(self: *LiveTokenBudget, bytes: usize) BudgetError!void {
        const objects = try add(try add(self.current.live_objects, self.pending_objects), 1);
        const total = try add(try add(self.current.live_requested_bytes, self.pending_bytes), bytes);
        if (objects > self.limits.max_live_objects or total > self.limits.max_live_requested_bytes) return error.LimitExceeded;
        const pending_objects = try add(self.pending_objects, 1);
        const pending_bytes = try add(self.pending_bytes, bytes);
        self.pending_objects = pending_objects;
        self.pending_bytes = pending_bytes;
    }
    fn cancel(self: *LiveTokenBudget, bytes: usize) void {
        const objects = subtract(self.pending_objects, 1);
        const total = subtract(self.pending_bytes, bytes);
        self.pending_objects = objects;
        self.pending_bytes = total;
    }
    fn commit(self: *LiveTokenBudget, bytes: usize) void {
        const objects = add(self.current.live_objects, 1) catch @panic("token count overflow");
        const total = add(self.current.live_requested_bytes, bytes) catch @panic("token byte overflow");
        self.cancel(bytes);
        self.current.live_objects = objects;
        self.current.live_requested_bytes = total;
        self.current.peak_objects = @max(self.current.peak_objects, objects);
        self.current.peak_requested_bytes = @max(self.current.peak_requested_bytes, total);
    }
    fn refund(self: *LiveTokenBudget, bytes: usize) void {
        const objects = subtract(self.current.live_objects, 1);
        const total = subtract(self.current.live_requested_bytes, bytes);
        self.current.live_objects = objects;
        self.current.live_requested_bytes = total;
    }
};
const Token = struct { allocator: std.mem.Allocator, refs: usize, budget: *LiveTokenBudget, charged_bytes: usize };
const TokenIdentity = opaque {};
/// Value copies borrow; each owning slot explicitly retains once. UI-thread only.
/// The opaque identity exposes no refcount or raw mutable token storage.
pub const TokenRef = struct {
    identity: *TokenIdentity,
    fn storage(self: TokenRef) *Token {
        return @ptrCast(@alignCast(self.identity));
    }
    pub fn create(allocator: std.mem.Allocator, budget: *LiveTokenBudget) TokenCreateError!TokenRef {
        const bytes = @sizeOf(Token);
        try budget.reserve(bytes);
        const token = allocator.create(Token) catch {
            budget.cancel(bytes);
            return error.OutOfMemory;
        };
        token.* = .{ .allocator = allocator, .refs = 1, .budget = budget, .charged_bytes = bytes };
        budget.commit(bytes);
        return .{ .identity = @ptrCast(token) };
    }
    pub fn retain(self: TokenRef) TokenRetainError!TokenRef {
        const token = self.storage();
        contract(token.refs != 0);
        if (token.refs == std.math.maxInt(usize)) return error.ReferenceOverflow;
        token.refs += 1;
        return self;
    }
    pub fn release(self: TokenRef) void {
        const token = self.storage();
        contract(token.refs != 0);
        token.refs -= 1;
        if (token.refs == 0) {
            const allocator = token.allocator;
            const budget = token.budget;
            const bytes = token.charged_bytes;
            allocator.destroy(token);
            budget.refund(bytes);
        }
    }
    pub fn sameIdentity(self: TokenRef, other: TokenRef) bool {
        return self.identity == other.identity;
    }
};
/// Borrowed metadata: no allocation, retention, UTF-8 validation or owned context.
pub const OwnerSpec = struct {
    resident: TokenRef,
    hold: ?u64,
    context: []const u8,
    draft: ?TokenRef,
    pub fn matches(self: OwnerSpec, other: OwnerSpec) bool {
        if (!self.resident.sameIdentity(other.resident) or self.hold != other.hold or !std.mem.eql(u8, self.context, other.context)) return false;
        if (self.draft) |draft| return if (other.draft) |other_draft| draft.sameIdentity(other_draft) else false;
        return other.draft == null;
    }
};
test "private token reference overflow and slot unwind" {
    var budget = LiveTokenBudget.init(.{ .max_live_objects = 1, .max_live_requested_bytes = @sizeOf(Token) });
    defer budget.deinit();
    const r = try TokenRef.create(std.testing.allocator, &budget);
    defer r.release();
    r.storage().refs = std.math.maxInt(usize) - 1;
    const first = try r.retain();
    try std.testing.expectEqual(std.math.maxInt(usize), r.storage().refs);
    try std.testing.expectError(error.ReferenceOverflow, r.retain());
    first.release();
    try std.testing.expectEqual(std.math.maxInt(usize) - 1, r.storage().refs);
    r.storage().refs = 1;
}
test "private token requested layout ceilings and synthetic arithmetic" {
    const bytes = @sizeOf(Token);
    var exact = LiveTokenBudget.init(.{ .max_live_objects = 1, .max_live_requested_bytes = bytes });
    const r = try TokenRef.create(std.testing.allocator, &exact);
    try std.testing.expectEqual(bytes, exact.usage().live_requested_bytes);
    try std.testing.expectError(error.LimitExceeded, TokenRef.create(std.testing.allocator, &exact));
    r.release();
    exact.deinit();
    var small = LiveTokenBudget.init(.{ .max_live_objects = 1, .max_live_requested_bytes = bytes - 1 });
    try std.testing.expectError(error.LimitExceeded, TokenRef.create(std.testing.allocator, &small));
    small.deinit();
    var zero = LiveTokenBudget.init(.{ .max_live_objects = 0, .max_live_requested_bytes = 0 });
    try std.testing.expectError(error.LimitExceeded, TokenRef.create(std.testing.allocator, &zero));
    zero.deinit();
    var synthetic = LiveTokenBudget.init(.{ .max_live_objects = std.math.maxInt(usize), .max_live_requested_bytes = std.math.maxInt(usize) });
    synthetic.current.live_objects = std.math.maxInt(usize);
    try std.testing.expectError(error.LimitExceeded, synthetic.reserve(bytes));
    synthetic.current.live_objects = 0;
    synthetic.current.live_requested_bytes = std.math.maxInt(usize);
    try std.testing.expectError(error.LimitExceeded, synthetic.reserve(bytes));
    synthetic.current.live_requested_bytes = 0;
    synthetic.deinit();
}
pub const Population = enum { resident_frame, prepared_focus, staged_frame, frame_scratch, staged_focus, focus_scratch };
pub const RequestedBytes = struct { resident_frame: usize = 0, prepared_focus: usize = 0, staged_frame: usize = 0, frame_scratch: usize = 0, staged_focus: usize = 0, focus_scratch: usize = 0, total: usize = 0, peak: usize = 0 };
/// Stable UI-thread capability. Pending admission is separate from actual bytes.
/// Handle fields are implementation state: Zig has no private struct fields;
/// callers must use methods and move ownership exactly once by convention.
pub const PreparationBudget = struct {
    max_live_requested_bytes: usize,
    current: RequestedBytes = .{},
    pending_bytes: usize = 0,
    active_reservations: usize = 0,
    active_charges: usize = 0,
    pub fn init(max_live_requested_bytes: usize) PreparationBudget {
        return .{ .max_live_requested_bytes = max_live_requested_bytes };
    }
    pub fn ledger(self: *const PreparationBudget) RequestedBytes {
        return self.current;
    }
    fn populationBytes(self: *PreparationBudget, population: Population) *usize {
        return switch (population) {
            .resident_frame => &self.current.resident_frame,
            .prepared_focus => &self.current.prepared_focus,
            .staged_frame => &self.current.staged_frame,
            .frame_scratch => &self.current.frame_scratch,
            .staged_focus => &self.current.staged_focus,
            .focus_scratch => &self.current.focus_scratch,
        };
    }
    pub fn reserve(self: *PreparationBudget, population: Population, bytes: usize) BudgetError!Reservation {
        const reservations = try add(self.active_reservations, 1);
        _ = try add(self.active_charges, reservations);
        _ = try add(self.populationBytes(population).*, bytes);
        const pending = try add(self.pending_bytes, bytes);
        const total = try add(self.current.total, pending);
        if (total > self.max_live_requested_bytes) return error.LimitExceeded;
        self.pending_bytes = pending;
        self.active_reservations = reservations;
        return .{ .state = .{ .budget = self, .population = population, .bytes = bytes, .active = true } };
    }
    pub fn deinit(self: *PreparationBudget) void {
        contract(self.active_reservations == 0 and self.active_charges == 0 and self.pending_bytes == 0 and self.current.total == 0);
    }
};
const ReceiptState = struct { budget: *PreparationBudget, population: Population, bytes: usize, active: bool };
/// Move-only by convention; commit only immediately after successful allocation.
pub const Reservation = struct {
    state: ReceiptState,
    pub fn commit(self: *Reservation) Charge {
        contract(self.state.active);
        const state = self.state;
        const b = state.budget;
        const charges = add(b.active_charges, 1) catch @panic("charge count overflow");
        const reservations = subtract(b.active_reservations, 1);
        const pending = subtract(b.pending_bytes, state.bytes);
        const population = b.populationBytes(state.population);
        const population_bytes = add(population.*, state.bytes) catch @panic("population overflow");
        const total = add(b.current.total, state.bytes) catch @panic("ledger total overflow");
        contract(total <= b.max_live_requested_bytes);
        b.active_charges = charges;
        b.active_reservations = reservations;
        b.pending_bytes = pending;
        population.* = population_bytes;
        b.current.total = total;
        b.current.peak = @max(b.current.peak, total);
        self.state.active = false;
        return .{ .state = state };
    }
    pub fn cancel(self: *Reservation) void {
        contract(self.state.active);
        const b = self.state.budget;
        const reservations = subtract(b.active_reservations, 1);
        const pending = subtract(b.pending_bytes, self.state.bytes);
        b.active_reservations = reservations;
        b.pending_bytes = pending;
        self.state.active = false;
    }
};
/// Free corresponding storage before release. Reclassification allocates nothing.
pub const Charge = struct {
    state: ReceiptState,
    pub fn reclassify(self: *Charge, population: Population) void {
        contract(self.state.active);
        const b = self.state.budget;
        contract(b.active_charges > 0);
        if (population == self.state.population) {
            _ = subtract(b.populationBytes(population).*, self.state.bytes);
            return;
        }
        const old = b.populationBytes(self.state.population);
        const next = b.populationBytes(population);
        const old_bytes = subtract(old.*, self.state.bytes);
        const next_bytes = add(next.*, self.state.bytes) catch @panic("population overflow");
        old.* = old_bytes;
        next.* = next_bytes;
        self.state.population = population;
    }
    pub fn release(self: *Charge) void {
        contract(self.state.active);
        const b = self.state.budget;
        const charges = subtract(b.active_charges, 1);
        const population = b.populationBytes(self.state.population);
        const population_bytes = subtract(population.*, self.state.bytes);
        const total = subtract(b.current.total, self.state.bytes);
        b.active_charges = charges;
        population.* = population_bytes;
        b.current.total = total;
        self.state.active = false;
    }
};
test "private ledger count and byte synthetic overflow admission" {
    var b = PreparationBudget.init(std.math.maxInt(usize));
    b.active_reservations = std.math.maxInt(usize);
    try std.testing.expectError(error.LimitExceeded, b.reserve(.staged_frame, 0));
    try std.testing.expectEqual(std.math.maxInt(usize), b.active_reservations);
    b.active_reservations = 0;
    b.active_charges = std.math.maxInt(usize);
    try std.testing.expectError(error.LimitExceeded, b.reserve(.staged_frame, 0));
    b.active_charges = 0;
    b.current.total = std.math.maxInt(usize);
    try std.testing.expectError(error.LimitExceeded, b.reserve(.staged_frame, 1));
    b.current.total = 0;
    b.current.staged_frame = std.math.maxInt(usize);
    try std.testing.expectError(error.LimitExceeded, b.reserve(.staged_frame, 1));
    b.current.staged_frame = 0;
    b.pending_bytes = std.math.maxInt(usize);
    try std.testing.expectError(error.LimitExceeded, b.reserve(.staged_frame, 1));
    b.pending_bytes = 0;
    b.deinit();
}

/// Explicit consumer ceilings; no production defaults.
pub const Limits = struct { max_nodes: usize, max_id_bytes: usize, max_text_bytes: usize, max_focus_actions: usize, max_announcements: usize, max_child_indices: usize, max_owner_context_bytes: usize, max_owned_bytes: usize };
pub const ActionId = struct { bytes: []const u8 };
pub const FocusSpec = struct { base: []const ActionId, modal: ?[]const ActionId, focused: ?ActionId, restore: ?ActionId };
pub const FocusError = error{ OutOfMemory, LimitExceeded, InvalidUtf8, InvalidFocus, EmptyModal, ModalAlreadyOpen };
/// Owns all returned bytes. Move by convention; allocator and budget stay stable.
/// Views borrow until a successful mutation or deinit; failed rebuilds preserve them.
pub const OwnedFocus = struct {
    allocator: std.mem.Allocator,
    limits: Limits,
    budget: *PreparationBudget,
    table: []ActionId = &.{},
    base: []ActionId = &.{},
    modal: []ActionId = &.{},
    bytes: []u8 = &.{},
    charges: [4]?Charge = .{ null, null, null, null },
    requested: usize = 0,
    focused: ?usize = null,
    restore: ?usize = null,
    modal_present: bool = false,

    pub fn init(allocator: std.mem.Allocator, spec: FocusSpec, limits: Limits, budget: *PreparationBudget) FocusError!OwnedFocus {
        return build(allocator, spec, limits, budget, .prepared_focus, 0, 0);
    }
    pub fn cloneFromFrame(allocator: std.mem.Allocator, frame: FrameView, limits: Limits, budget: *PreparationBudget) FocusError!OwnedFocus {
        return init(allocator, frame.focus, limits, budget);
    }
    pub fn fresh(allocator: std.mem.Allocator, base: []const ActionId, limits: Limits, budget: *PreparationBudget) FocusError!OwnedFocus {
        return init(allocator, .{ .base = base, .modal = null, .focused = if (base.len == 0) null else base[0], .restore = null }, limits, budget);
    }
    pub fn deinit(self: *OwnedFocus) void {
        // Reverse actual allocation order; discharge only after allocator free.
        self.allocator.free(self.bytes);
        if (self.charges[3]) |*c| c.release();
        self.allocator.free(self.modal);
        if (self.charges[2]) |*c| c.release();
        self.allocator.free(self.base);
        if (self.charges[1]) |*c| c.release();
        self.allocator.free(self.table);
        if (self.charges[0]) |*c| c.release();
        self.* = undefined;
    }
    pub fn view(self: *const OwnedFocus) FocusSpec {
        return .{ .base = self.base, .modal = if (self.modal_present) self.modal else null, .focused = if (self.focused) |i| self.table[i] else null, .restore = if (self.restore) |i| self.table[i] else null };
    }
    fn eql(a: ActionId, b: ActionId) bool {
        return std.mem.eql(u8, a.bytes, b.bytes);
    }
    fn find(order: []const ActionId, target: ActionId) ?usize {
        for (order, 0..) |a, i| if (eql(a, target)) return i;
        return null;
    }
    fn uniqueCount(order: []const ActionId) usize {
        var count: usize = 0;
        for (order, 0..) |a, i| if (find(order[0..i], a) == null) {
            count += 1;
        };
        return count;
    }
    fn validate(order: []const ActionId) FocusError!void {
        for (order) |a| if (!std.unicode.utf8ValidateSlice(a.bytes)) return error.InvalidUtf8;
    }
    fn alloc(self: *OwnedFocus, comptime T: type, count: usize, slot: usize, population: Population, old: usize, scratch: usize) FocusError![]T {
        if (count == 0) return &.{};
        const size = std.math.mul(usize, count, @sizeOf(T)) catch return error.LimitExceeded;
        const next_requested = try add(self.requested, size);
        if (try add(try add(old, scratch), next_requested) > self.limits.max_owned_bytes) return error.LimitExceeded;
        var reservation = try self.budget.reserve(population, size);
        const data = self.allocator.alloc(T, count) catch {
            reservation.cancel();
            return error.OutOfMemory;
        };
        self.charges[slot] = reservation.commit();
        self.requested = next_requested;
        return data;
    }
    fn build(allocator: std.mem.Allocator, spec: FocusSpec, limits: Limits, budget: *PreparationBudget, population: Population, old: usize, old_ids: usize) FocusError!OwnedFocus {
        return buildCharged(allocator, spec, limits, budget, population, .focus_scratch, old, old_ids);
    }
    fn buildCharged(allocator: std.mem.Allocator, spec: FocusSpec, limits: Limits, budget: *PreparationBudget, population: Population, scratch_population: Population, old: usize, old_ids: usize) FocusError!OwnedFocus {
        try validate(spec.base);
        const modal = spec.modal orelse &.{};
        try validate(modal);
        if (spec.focused) |a| try validate(&.{a});
        if (spec.restore) |a| try validate(&.{a});
        if (spec.modal != null and modal.len == 0) return error.EmptyModal;
        if (spec.modal == null and spec.restore != null) return error.InvalidFocus;
        if (spec.focused) |a| if (find(if (spec.modal != null) modal else spec.base, a) == null) return error.InvalidFocus;
        const bcount = uniqueCount(spec.base);
        const mcount = uniqueCount(modal);
        if (try add(bcount, mcount) > limits.max_focus_actions) return error.LimitExceeded;
        const q = try add(try add(spec.base.len, modal.len), try add(@intFromBool(spec.focused != null), @intFromBool(spec.restore != null)));
        const scratch_size = std.math.mul(usize, q, @sizeOf(ActionId)) catch return error.LimitExceeded;
        if (try add(old, scratch_size) > limits.max_owned_bytes) return error.LimitExceeded;
        var scratch_charge: ?Charge = null;
        const scratch: []ActionId = if (q == 0) &.{} else blk: {
            var r = try budget.reserve(scratch_population, scratch_size);
            const data = allocator.alloc(ActionId, q) catch {
                r.cancel();
                return error.OutOfMemory;
            };
            scratch_charge = r.commit();
            break :blk data;
        };
        defer {
            allocator.free(scratch);
            if (scratch_charge) |*c| c.release();
        }
        var used: usize = 0;
        for (spec.base) |a| {
            if (find(scratch[0..used], a) == null) {
                scratch[used] = a;
                used += 1;
            }
        }
        for (modal) |a| {
            if (find(scratch[0..used], a) == null) {
                scratch[used] = a;
                used += 1;
            }
        }
        for ([_]?ActionId{ spec.focused, spec.restore }) |maybe| if (maybe) |a| {
            if (find(scratch[0..used], a) == null) {
                scratch[used] = a;
                used += 1;
            }
        };
        var length: usize = 0;
        for (scratch[0..used]) |a| length = try add(length, a.bytes.len);
        if (try add(old_ids, length) > limits.max_id_bytes) return error.LimitExceeded;
        var result = OwnedFocus{ .allocator = allocator, .limits = limits, .budget = budget, .modal_present = spec.modal != null };
        errdefer result.deinit();
        result.table = try result.alloc(ActionId, used, 0, population, old, scratch_size);
        result.base = try result.alloc(ActionId, bcount, 1, population, old, scratch_size);
        result.modal = try result.alloc(ActionId, mcount, 2, population, old, scratch_size);
        result.bytes = try result.alloc(u8, length, 3, population, old, scratch_size);
        var offset: usize = 0;
        for (scratch[0..used], result.table) |a, *dest| {
            const end = offset + a.bytes.len;
            @memcpy(result.bytes[offset..end], a.bytes);
            dest.* = .{ .bytes = result.bytes[offset..end] };
            offset = end;
        }
        var n: usize = 0;
        for (spec.base, 0..) |a, i| if (find(spec.base[0..i], a) == null) {
            result.base[n] = result.table[find(scratch[0..used], a).?];
            n += 1;
        };
        n = 0;
        for (modal, 0..) |a, i| if (find(modal[0..i], a) == null) {
            result.modal[n] = result.table[find(scratch[0..used], a).?];
            n += 1;
        };
        if (spec.focused) |a| result.focused = find(result.table, a);
        if (spec.restore) |a| result.restore = find(result.table, a);
        return result;
    }
    fn active(self: *const OwnedFocus) []const ActionId {
        return if (self.modal_present) self.modal else self.base;
    }
    pub fn requestFocus(self: *OwnedFocus, target: ActionId) bool {
        if (find(self.active(), target) == null) return false;
        self.focused = find(self.table, target).?;
        return true;
    }
    fn advance(self: *OwnedFocus, forward: bool) ?ActionId {
        const order = self.active();
        if (order.len == 0) {
            self.focused = null;
            return null;
        }
        const current = if (self.focused) |i| find(order, self.table[i]) orelse 0 else 0;
        const selected = if (forward) (if (current == order.len - 1) 0 else current + 1) else (if (current == 0) order.len - 1 else current - 1);
        const target = order[selected];
        self.focused = find(self.table, target).?;
        return target;
    }
    pub fn next(self: *OwnedFocus) ?ActionId {
        return self.advance(true);
    }
    pub fn previous(self: *OwnedFocus) ?ActionId {
        return self.advance(false);
    }
    pub fn closeModal(self: *OwnedFocus) ?ActionId {
        if (!self.modal_present) return null;
        const saved = if (self.restore) |i| self.table[i] else null;
        const target: ?ActionId = if (saved != null and find(self.base, saved.?) != null) saved else if (self.base.len != 0) self.base[0] else null;
        self.focused = if (target) |a| find(self.table, a).? else null;
        self.restore = null;
        self.modal_present = false;
        return target;
    }
    fn choose(order: []const ActionId, focused: ?ActionId) ?ActionId {
        if (focused) |a| if (find(order, a) != null) return a;
        return if (order.len == 0) null else order[0];
    }
    fn rebuild(self: *OwnedFocus, spec: FocusSpec) FocusError!void {
        const candidate = try build(self.allocator, spec, self.limits, self.budget, .staged_focus, self.requested, self.bytes.len);
        var old = self.*;
        self.* = candidate;
        for (&self.charges) |*maybe| if (maybe.*) |*c| c.reclassify(.prepared_focus);
        old.deinit();
    }
    pub fn replaceBase(self: *OwnedFocus, base: []const ActionId) FocusError!void {
        var spec = self.view();
        spec.base = base;
        if (!self.modal_present) spec.focused = choose(base, spec.focused);
        try self.rebuild(spec);
    }
    pub fn replaceActive(self: *OwnedFocus, order: []const ActionId) FocusError!void {
        if (!self.modal_present) return self.replaceBase(order);
        if (order.len == 0) return error.EmptyModal;
        var spec = self.view();
        spec.modal = order;
        spec.focused = choose(order, spec.focused);
        try self.rebuild(spec);
    }
    pub fn openModal(self: *OwnedFocus, modal: []const ActionId) FocusError!ActionId {
        if (self.modal_present) return error.ModalAlreadyOpen;
        if (modal.len == 0) return error.EmptyModal;
        var spec = self.view();
        spec.restore = spec.focused;
        spec.modal = modal;
        spec.focused = modal[0];
        try self.rebuild(spec);
        return self.view().focused.?;
    }
};

// Synthetic arithmetic only: these counts do not describe actual allocations.
test "private focus allocation checked overflow without giant slices" {
    var budget = PreparationBudget.init(std.math.maxInt(usize));
    var failing = std.testing.FailingAllocator.init(std.testing.allocator, .{});
    const limits = Limits{ .max_nodes = 0, .max_id_bytes = 4096, .max_text_bytes = 0, .max_focus_actions = 64, .max_announcements = 0, .max_child_indices = 0, .max_owner_context_bytes = 0, .max_owned_bytes = std.math.maxInt(usize) };
    var candidate = OwnedFocus{ .allocator = failing.allocator(), .limits = limits, .budget = &budget };
    try std.testing.expectError(error.LimitExceeded, candidate.alloc(ActionId, std.math.maxInt(usize), 0, .staged_focus, 0, 0));
    try std.testing.expectError(error.LimitExceeded, candidate.alloc(ActionId, 1, 0, .staged_focus, std.math.maxInt(usize), 0));
    try std.testing.expectError(error.LimitExceeded, candidate.alloc(ActionId, 1, 0, .staged_focus, 0, std.math.maxInt(usize)));
    candidate.requested = std.math.maxInt(usize);
    try std.testing.expectError(error.LimitExceeded, candidate.alloc(u8, 1, 3, .staged_focus, 0, 0));
    candidate.requested = 0;
    // Checked addition is shared by raw union counts and copied-byte sums.
    try std.testing.expectError(error.LimitExceeded, add(std.math.maxInt(usize), 1));
    try std.testing.expectEqual(@as(usize, 0), failing.allocations);
    try std.testing.expectEqualDeep(RequestedBytes{}, budget.ledger());
    candidate.deinit();
    budget.deinit();
}

pub const NodeId = struct { bytes: []const u8 };
pub const NodeRole = enum { application, toolbar, button, tree, tree_item, region, group, heading, text, status, alert, dialog, list, list_item, option, checkbox, text_input, spin_button };
pub const Rect = @import("retained_geometry.zig").Rect;
pub const NodeSpec = struct {
    id: NodeId,
    role: NodeRole,
    name: []const u8,
    description: []const u8,
    value: ?[]const u8,
    enabled: bool,
    selected: bool,
    focusable: bool,
    hierarchy_level: ?u32,
    expanded: ?bool,
    action: ?ActionId,
    bounds: ?Rect,
    visible: bool,
    parent: ?usize,
    child_start: usize,
    child_count: usize,
};
pub const AnnouncementSpec = struct { kind: enum { status, @"error" }, code: []const u8, message: []const u8 };
pub const FrameInput = struct { nodes: []const NodeSpec, child_indices: []const usize, announcements: []const AnnouncementSpec, owner: OwnerSpec, focus: FocusSpec };
pub const FrameError = FocusError || TokenRetainError || error{ InvalidBounds, RootIsNotApplication, DuplicateNodeId, DuplicateActionId, ActionIsNotFocusable, InvalidTopology };
/// Immutable borrowed snapshot; expires on successful publication or deinit.
pub const FrameView = struct {
    nodes: []const NodeSpec,
    child_indices: []const usize,
    announcements: []const AnnouncementSpec,
    owner: OwnerSpec,
    focus: FocusSpec,
    pub fn node(self: FrameView, id: NodeId) ?NodeSpec {
        for (self.nodes) |n| if (std.mem.eql(u8, n.id.bytes, id.bytes)) return n;
        return null;
    }
    pub fn visibleNodes(self: FrameView, clip: Rect, output: []usize) error{CapacityExceeded}![]const usize {
        var count: usize = 0;
        for (self.nodes) |n| if (n.visible) {
            if (n.bounds) |bounds| if (bounds.intersection(clip) != null) {
                count += 1;
            };
        };
        if (count > output.len) return error.CapacityExceeded;
        var used: usize = 0;
        for (self.nodes, 0..) |n, i| if (n.visible) {
            if (n.bounds) |bounds| if (bounds.intersection(clip) != null) {
                output[used] = i;
                used += 1;
            };
        };
        return output[0..used];
    }
};
fn validBytes(bytes: []const u8) FrameError!void {
    if (!std.unicode.utf8ValidateSlice(bytes)) return error.InvalidUtf8;
}
const FrameLengths = struct { ids: usize = 0, text: usize = 0, context: usize = 0 };
fn preflight(input: FrameInput, limits: Limits) FrameError!FrameLengths {
    const nodes = input.nodes;
    if (nodes.len == 0) return error.InvalidTopology;
    if (nodes.len > limits.max_nodes or input.child_indices.len > limits.max_child_indices or input.announcements.len > limits.max_announcements) return error.LimitExceeded;
    if (nodes[0].role != .application) return error.RootIsNotApplication;
    if (nodes[0].parent != null or input.child_indices.len != nodes.len - 1) return error.InvalidTopology;
    var lengths = FrameLengths{ .context = input.owner.context.len };
    try validBytes(input.owner.context);
    if (lengths.context > limits.max_owner_context_bytes) return error.LimitExceeded;
    var edges: usize = 0;
    for (nodes, 0..) |n, i| {
        try validBytes(n.id.bytes);
        lengths.ids = try add(lengths.ids, n.id.bytes.len);
        try validBytes(n.name);
        try validBytes(n.description);
        lengths.text = try add(lengths.text, try add(n.name.len, n.description.len));
        if (n.value) |v| {
            try validBytes(v);
            lengths.text = try add(lengths.text, v.len);
        }
        if (n.action) |a| {
            try validBytes(a.bytes);
            lengths.ids = try add(lengths.ids, a.bytes.len);
            if (!n.focusable) return error.ActionIsNotFocusable;
        }
        if (n.bounds) |bounds| {
            _ = Rect.fromBounds(bounds.min, bounds.max) catch |err| switch (err) {
                error.NonFinite, error.InvalidBounds, error.Overflow => return error.InvalidBounds,
            };
        }
        if (i != 0) {
            const parent = n.parent orelse return error.InvalidTopology;
            if (parent >= nodes.len or parent == i) return error.InvalidTopology;
        }
        const end = try add(n.child_start, n.child_count);
        if (end > input.child_indices.len) return error.InvalidTopology;
        edges = try add(edges, n.child_count);
        for (nodes[0..i]) |previous| {
            if (std.mem.eql(u8, n.id.bytes, previous.id.bytes)) return error.DuplicateNodeId;
            if (n.action) |a| if (previous.action) |b| if (std.mem.eql(u8, a.bytes, b.bytes)) return error.DuplicateActionId;
            const previous_end = try add(previous.child_start, previous.child_count);
            if (n.child_count != 0 and previous.child_count != 0 and n.child_start < previous_end and previous.child_start < end) return error.InvalidTopology;
        }
        for (input.child_indices[n.child_start..end], 0..) |child, j| {
            if (child == 0 or child >= nodes.len or nodes[child].parent != i) return error.InvalidTopology;
            for (input.child_indices[n.child_start .. n.child_start + j]) |previous| if (child == previous) return error.InvalidTopology;
        }
    }
    if (edges != input.child_indices.len) return error.InvalidTopology;
    for (input.announcements) |a| {
        try validBytes(a.code);
        try validBytes(a.message);
        lengths.text = try add(lengths.text, try add(a.code.len, a.message.len));
    }
    if (lengths.ids > limits.max_id_bytes or lengths.text > limits.max_text_bytes) return error.LimitExceeded;
    return lengths;
}
fn copyBytes(storage: []u8, offset: *usize, bytes: []const u8) []const u8 {
    const start = offset.*;
    offset.* += bytes.len;
    @memcpy(storage[start..offset.*], bytes);
    return storage[start..offset.*];
}
/// Move by convention; no duplicated owning copies. Public methods provide no
/// mutable focus escape. Zig fields are accessible; hostile mutation is unsupported.
pub const OwnedFrame = struct {
    allocator: std.mem.Allocator,
    budget: *PreparationBudget,
    nodes: []NodeSpec = &.{},
    children: []usize = &.{},
    announcements: []AnnouncementSpec = &.{},
    bytes: []u8 = &.{},
    charges: [4]?Charge = .{ null, null, null, null },
    requested: usize = 0,
    owner: OwnerSpec,
    resident_retained: bool = false,
    draft_retained: bool = false,
    focus: ?OwnedFocus = null,

    pub fn init(allocator: std.mem.Allocator, budget: *PreparationBudget, input: FrameInput, limits: Limits) FrameError!OwnedFrame {
        var result = try build(allocator, budget, input, limits);
        result.reclassify(.resident_frame);
        return result;
    }
    fn allocate(self: *OwnedFrame, comptime T: type, count: usize, slot: usize, scratch: usize, limits: Limits) FrameError![]T {
        if (count == 0) return &.{};
        const size = std.math.mul(usize, count, @sizeOf(T)) catch return error.LimitExceeded;
        const requested = try add(self.requested, size);
        if (try add(requested, scratch) > limits.max_owned_bytes) return error.LimitExceeded;
        var reservation = try self.budget.reserve(.staged_frame, size);
        const data = self.allocator.alloc(T, count) catch {
            reservation.cancel();
            return error.OutOfMemory;
        };
        self.charges[slot] = reservation.commit();
        self.requested = requested;
        return data;
    }
    fn build(allocator: std.mem.Allocator, budget: *PreparationBudget, input: FrameInput, limits: Limits) FrameError!OwnedFrame {
        const lengths = try preflight(input, limits);
        const scratch_size = std.math.mul(usize, input.nodes.len, @sizeOf(usize)) catch return error.LimitExceeded;
        if (scratch_size > limits.max_owned_bytes) return error.LimitExceeded;
        var reservation = try budget.reserve(.frame_scratch, scratch_size);
        const stack = allocator.alloc(usize, input.nodes.len) catch {
            reservation.cancel();
            return error.OutOfMemory;
        };
        var scratch_charge = reservation.commit();
        defer {
            allocator.free(stack);
            scratch_charge.release();
        }
        // Iterative DFS must consume the exact flat preorder. A repeated node,
        // cycle, orphan or nonpreorder edge cannot match the next expected index.
        var pending: usize = 1;
        stack[0] = 0;
        var expected: usize = 0;
        while (pending != 0) {
            pending -= 1;
            const index = stack[pending];
            if (index != expected or expected >= input.nodes.len) return error.InvalidTopology;
            expected += 1;
            const n = input.nodes[index];
            if (n.child_count > stack.len - pending) return error.InvalidTopology;
            var j = n.child_count;
            while (j != 0) {
                j -= 1;
                stack[pending] = input.child_indices[n.child_start + j];
                pending += 1;
            }
        }
        if (expected != input.nodes.len) return error.InvalidTopology;
        var result = OwnedFrame{ .allocator = allocator, .budget = budget, .owner = input.owner };
        errdefer result.deinit();
        result.nodes = try result.allocate(NodeSpec, input.nodes.len, 0, scratch_size, limits);
        result.children = try result.allocate(usize, input.child_indices.len, 1, scratch_size, limits);
        result.announcements = try result.allocate(AnnouncementSpec, input.announcements.len, 2, scratch_size, limits);
        result.bytes = try result.allocate(u8, try add(try add(lengths.ids, lengths.text), lengths.context), 3, scratch_size, limits);
        var offset: usize = 0;
        for (input.nodes, result.nodes) |source, *dest| {
            dest.* = source;
            dest.id.bytes = copyBytes(result.bytes, &offset, source.id.bytes);
            dest.name = copyBytes(result.bytes, &offset, source.name);
            dest.description = copyBytes(result.bytes, &offset, source.description);
            if (source.value) |v| dest.value = copyBytes(result.bytes, &offset, v);
            if (source.action) |a| dest.action = .{ .bytes = copyBytes(result.bytes, &offset, a.bytes) };
        }
        @memcpy(result.children, input.child_indices);
        for (input.announcements, result.announcements) |source, *dest| {
            dest.* = .{ .kind = source.kind, .code = copyBytes(result.bytes, &offset, source.code), .message = copyBytes(result.bytes, &offset, source.message) };
        }
        result.owner.context = copyBytes(result.bytes, &offset, input.owner.context);
        result.focus = try buildFrameFocus(allocator, input.focus, limits, budget, try add(result.requested, scratch_size), lengths.ids);
        result.requested = try add(result.requested, result.focus.?.requested);
        result.owner.resident = try input.owner.resident.retain();
        result.resident_retained = true;
        if (input.owner.draft) |draft| {
            result.owner.draft = try draft.retain();
            result.draft_retained = true;
        }
        return result;
    }
    fn reclassify(self: *OwnedFrame, population: Population) void {
        for (&self.charges) |*maybe| if (maybe.*) |*c| c.reclassify(population);
        if (self.focus) |*focus| for (&focus.charges) |*maybe| if (maybe.*) |*c| c.reclassify(population);
    }
    pub fn view(self: *const OwnedFrame) FrameView {
        return .{ .nodes = self.nodes, .child_indices = self.children, .announcements = self.announcements, .owner = self.owner, .focus = self.focus.?.view() };
    }
    pub fn deinit(self: *OwnedFrame) void {
        if (self.draft_retained) self.owner.draft.?.release();
        if (self.resident_retained) self.owner.resident.release();
        if (self.focus) |*focus| focus.deinit();
        self.allocator.free(self.bytes);
        if (self.charges[3]) |*c| c.release();
        self.allocator.free(self.announcements);
        if (self.charges[2]) |*c| c.release();
        self.allocator.free(self.children);
        if (self.charges[1]) |*c| c.release();
        self.allocator.free(self.nodes);
        if (self.charges[0]) |*c| c.release();
        self.* = undefined;
    }
};
/// One complete publication. UI-thread serialization and stable capabilities are
/// caller obligations; do not overlap candidates or helper mutation.
pub const FrameSlot = struct {
    allocator: std.mem.Allocator,
    budget: *PreparationBudget,
    frame: ?OwnedFrame = null,
    pub fn init(allocator: std.mem.Allocator, budget: *PreparationBudget) FrameSlot {
        return .{ .allocator = allocator, .budget = budget };
    }
    pub fn replace(self: *FrameSlot, input: FrameInput, limits: Limits) FrameError!void {
        var candidate = try OwnedFrame.build(self.allocator, self.budget, input, limits);
        candidate.reclassify(.resident_frame);
        var old = self.frame;
        self.frame = candidate;
        if (old) |*frame| frame.deinit();
    }
    pub fn view(self: *const FrameSlot) ?FrameView {
        return if (self.frame) |*frame| frame.view() else null;
    }
    pub fn deinit(self: *FrameSlot) void {
        if (self.frame) |*frame| frame.deinit();
        self.* = undefined;
    }
};

fn buildFrameFocus(allocator: std.mem.Allocator, spec: FocusSpec, limits: Limits, budget: *PreparationBudget, old: usize, old_ids: usize) FocusError!OwnedFocus {
    return OwnedFocus.buildCharged(allocator, spec, limits, budget, .staged_frame, .frame_scratch, old, old_ids);
}
test "frame private focus construction C and S ledger witness" {
    var budget = PreparationBudget.init(65536);
    defer budget.deinit();
    const limits = Limits{ .max_nodes = 1, .max_id_bytes = 100, .max_text_bytes = 100, .max_focus_actions = 10, .max_announcements = 1, .max_child_indices = 1, .max_owner_context_bytes = 100, .max_owned_bytes = 65536 };
    var focus = try buildFrameFocus(std.testing.allocator, .{ .base = &.{.{ .bytes = "A" }}, .modal = null, .focused = null, .restore = null }, limits, &budget, 0, 0);
    defer focus.deinit();
    try std.testing.expectEqual(@as(usize, 0), budget.ledger().prepared_focus);
    try std.testing.expectEqual(@as(usize, 3 * @sizeOf(ActionId) + 1), budget.ledger().peak);
    try std.testing.expectEqual(@as(usize, 2 * @sizeOf(ActionId) + 1), budget.ledger().staged_frame);
}

// Test-only direct storage access; no public refcount setter or token internals.
test "frame per occupied owner slot retains and every overflow unwinds" {
    var token_budget = LiveTokenBudget.init(.{ .max_live_objects = 2, .max_live_requested_bytes = 4096 });
    defer token_budget.deinit();
    const resident = try TokenRef.create(std.testing.allocator, &token_budget);
    defer resident.release();
    const draft = try TokenRef.create(std.testing.allocator, &token_budget);
    defer draft.release();
    var budget = PreparationBudget.init(65536);
    defer budget.deinit();
    const root = NodeSpec{ .id = .{ .bytes = "r" }, .role = .application, .name = "", .description = "", .value = null, .enabled = true, .selected = false, .focusable = false, .hierarchy_level = null, .expanded = null, .action = null, .bounds = null, .visible = true, .parent = null, .child_start = 0, .child_count = 0 };
    const limits = Limits{ .max_nodes = 1, .max_id_bytes = 10, .max_text_bytes = 10, .max_focus_actions = 10, .max_announcements = 2, .max_child_indices = 1, .max_owner_context_bytes = 10, .max_owned_bytes = 65536 };
    const announcements = [_]AnnouncementSpec{ .{ .kind = .status, .code = "r", .message = "α" }, .{ .kind = .@"error", .code = "b", .message = "β" } };
    const base = [_]ActionId{ .{ .bytes = "A" }, .{ .bytes = "B" } };
    const modal = [_]ActionId{ .{ .bytes = "X" }, .{ .bytes = "Y" } };
    var input = FrameInput{ .nodes = &.{root}, .child_indices = &.{}, .announcements = &announcements, .owner = .{ .resident = resident, .hold = 7, .context = "guide", .draft = resident }, .focus = .{ .base = &base, .modal = &modal, .focused = null, .restore = .{ .bytes = "Z" } } };
    var slot = FrameSlot.init(std.testing.allocator, &budget);
    defer slot.deinit();
    try slot.replace(input, limits);
    try std.testing.expectEqual(@as(usize, 3), resident.storage().refs);
    const old = slot.view().?;
    var helper = try OwnedFocus.cloneFromFrame(std.testing.allocator, old, limits, &budget);
    defer helper.deinit();
    const before = budget.ledger().total;
    const helper_old = helper.view();
    // First retain overflow; same-token second retain overflow; distinct-token
    // second retain overflow, all after actual candidate construction succeeds.
    for (0..3) |case| {
        const real_resident_refs = resident.storage().refs;
        const real_draft_refs = draft.storage().refs;
        switch (case) {
            0 => resident.storage().refs = std.math.maxInt(usize),
            1 => resident.storage().refs = std.math.maxInt(usize) - 1,
            2 => {
                input.owner.draft = draft;
                draft.storage().refs = std.math.maxInt(usize);
            },
            else => unreachable,
        }
        const injected_resident = resident.storage().refs;
        const injected_draft = draft.storage().refs;
        const result = slot.replace(input, limits);
        const after_resident = resident.storage().refs;
        const after_draft = draft.storage().refs;
        // Restore legitimate counts before assertions/deferred real cleanup.
        resident.storage().refs = real_resident_refs;
        draft.storage().refs = real_draft_refs;
        try std.testing.expectError(error.ReferenceOverflow, result);
        try std.testing.expectEqual(injected_resident, after_resident);
        try std.testing.expectEqual(injected_draft, after_draft);
        try std.testing.expect(old.nodes.ptr == slot.view().?.nodes.ptr and old.owner.matches(slot.view().?.owner));
        try std.testing.expectEqualDeep(root, old.node(.{ .bytes = "r" }).?);
        try std.testing.expectEqualDeep(@as([]const AnnouncementSpec, &announcements), old.announcements);
        try std.testing.expectEqualDeep(@as([]const ActionId, &base), old.focus.base);
        try std.testing.expectEqualDeep(@as([]const ActionId, &modal), old.focus.modal.?);
        try std.testing.expect(old.focus.focused == null);
        try std.testing.expectEqualStrings("Z", old.focus.restore.?.bytes);
        try std.testing.expectEqual(@as(?u64, 7), old.owner.hold);
        try std.testing.expectEqualStrings("guide", old.owner.context);
        try std.testing.expect(helper_old.base.ptr == helper.view().base.ptr and helper_old.modal.?.ptr == helper.view().modal.?.ptr);
        try std.testing.expectEqualDeep(old.focus, helper.view());
        try std.testing.expectEqual(before, budget.ledger().total);
        try std.testing.expectEqual(@as(usize, 0), budget.ledger().staged_frame + budget.ledger().frame_scratch + budget.pending_bytes);
    }
    input.owner.draft = draft;
    try slot.replace(input, limits);
    try std.testing.expectEqual(@as(usize, 2), resident.storage().refs);
    try std.testing.expectEqual(@as(usize, 2), draft.storage().refs);
}

test "frame every actual OOM preserves owner refcounts and synthetic arithmetic" {
    var token_budget = LiveTokenBudget.init(.{ .max_live_objects = 1, .max_live_requested_bytes = 4096 });
    defer token_budget.deinit();
    const token = try TokenRef.create(std.testing.allocator, &token_budget);
    defer token.release();
    var budget = PreparationBudget.init(65536);
    defer budget.deinit();
    var root = NodeSpec{ .id = .{ .bytes = "r" }, .role = .application, .name = "", .description = "", .value = null, .enabled = true, .selected = false, .focusable = false, .hierarchy_level = null, .expanded = null, .action = null, .bounds = null, .visible = true, .parent = null, .child_start = 0, .child_count = 0 };
    const limits = Limits{ .max_nodes = 2, .max_id_bytes = 10, .max_text_bytes = 10, .max_focus_actions = 10, .max_announcements = 1, .max_child_indices = 1, .max_owner_context_bytes = 10, .max_owned_bytes = 65536 };
    root.child_count = 1;
    var leaf = root;
    leaf.id.bytes = "l";
    leaf.role = .button;
    leaf.parent = 0;
    leaf.child_start = 1;
    leaf.child_count = 0;
    const nodes = [_]NodeSpec{ root, leaf };
    const base = [_]ActionId{.{ .bytes = "A" }};
    const modal = [_]ActionId{.{ .bytes = "X" }};
    const input = FrameInput{ .nodes = &nodes, .child_indices = &.{1}, .announcements = &.{.{ .kind = .status, .code = "ready", .message = "ok" }}, .owner = .{ .resident = token, .hold = null, .context = "", .draft = token }, .focus = .{ .base = &base, .modal = &modal, .focused = null, .restore = .{ .bytes = "Z" } } };
    var fail = std.testing.FailingAllocator.init(std.testing.allocator, .{});
    var slot = FrameSlot.init(fail.allocator(), &budget);
    defer slot.deinit();
    try slot.replace(input, limits);
    const requests = fail.alloc_index;
    try std.testing.expectEqual(@as(usize, 10), requests);
    var helper = try OwnedFocus.cloneFromFrame(fail.allocator(), slot.view().?, limits, &budget);
    defer helper.deinit();
    const old = slot.view().?;
    for (0..requests) |ordinal| {
        fail.fail_index = fail.alloc_index + ordinal;
        try std.testing.expectError(error.OutOfMemory, slot.replace(input, limits));
        try std.testing.expectEqual(@as(usize, 3), token.storage().refs);
        try std.testing.expect(old.nodes.ptr == slot.view().?.nodes.ptr);
    }
    // Synthetic arithmetic only: no fabricated slices or giant allocator request.
    const alloc_before = fail.alloc_index;
    var synthetic = OwnedFrame{ .allocator = fail.allocator(), .budget = &budget, .owner = input.owner };
    defer synthetic.deinit();
    try std.testing.expectError(error.LimitExceeded, synthetic.allocate(NodeSpec, std.math.maxInt(usize), 0, 0, limits));
    synthetic.requested = std.math.maxInt(usize);
    try std.testing.expectError(error.LimitExceeded, synthetic.allocate(u8, 1, 0, 0, limits));
    synthetic.requested = 0;
    try std.testing.expectError(error.LimitExceeded, synthetic.allocate(u8, 1, 0, std.math.maxInt(usize), limits));
    try std.testing.expectEqual(alloc_before, fail.alloc_index);
}
