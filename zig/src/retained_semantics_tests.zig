const std = @import("std");
const s = @import("retained_semantics.zig");
test "token live requested bytes and owner optional fields" {
    var budget = s.LiveTokenBudget.init(.{ .max_live_objects = 2, .max_live_requested_bytes = 4096 });
    defer budget.deinit();
    const r = try s.TokenRef.create(std.testing.allocator, &budget);
    defer r.release();
    try std.testing.expectEqual(@as(usize, 1), budget.usage().live_objects);
    const a = s.OwnerSpec{ .resident = r, .hold = null, .context = "guide", .draft = null };
    var b = a;
    b.hold = 0;
    try std.testing.expect(!a.matches(b));
}
test "actual ledger allocation records population and peak" {
    var budget = s.PreparationBudget.init(4);
    defer budget.deinit();
    var reservation = try budget.reserve(.staged_frame, 4);
    const bytes = try std.testing.allocator.alloc(u8, 4);
    var charge = reservation.commit();
    defer {
        std.testing.allocator.free(bytes);
        charge.release();
    }
    try std.testing.expectEqual(@as(usize, 4), budget.ledger().total);
    try std.testing.expectEqual(@as(usize, 4), budget.ledger().staged_frame);
    try std.testing.expectEqual(@as(usize, 4), budget.ledger().peak);
}

test "tracked token lifecycle stale identity and borrowed owner matrix" {
    var tracking = std.testing.FailingAllocator.init(std.testing.allocator, .{});
    var budget = s.LiveTokenBudget.init(.{ .max_live_objects = 3, .max_live_requested_bytes = 4096 });
    const r = try s.TokenRef.create(tracking.allocator(), &budget);
    const borrowed = r;
    try std.testing.expect(r.sameIdentity(borrowed));
    const stale = try r.retain();
    r.release();
    try std.testing.expectEqual(@as(usize, 0), tracking.deallocations);
    const other = try s.TokenRef.create(tracking.allocator(), &budget);
    const draft = try s.TokenRef.create(tracking.allocator(), &budget);
    try std.testing.expect(!stale.sameIdentity(other));
    const a = s.OwnerSpec{ .resident = stale, .hold = null, .context = "guide", .draft = null };
    try std.testing.expect(a.matches(a));
    var b = a;
    b.resident = other;
    try std.testing.expect(!a.matches(b));
    b = a;
    b.hold = 0;
    try std.testing.expect(!a.matches(b));
    b = a;
    b.context = "classic";
    try std.testing.expect(!a.matches(b));
    b = a;
    b.draft = draft;
    try std.testing.expect(!a.matches(b));
    const c = b;
    try std.testing.expect(b.matches(c));
    b.draft = other;
    try std.testing.expect(!b.matches(c));
    b.draft = null;
    try std.testing.expect(!c.matches(b));
    // Two actual ownership slots, with no imaginary per-retain allocation.
    const resident_slot = try stale.retain();
    const draft_slot = try stale.retain();
    try std.testing.expectEqual(@as(usize, 3), tracking.allocations);
    resident_slot.release();
    draft_slot.release();
    stale.release();
    other.release();
    draft.release();
    try std.testing.expectEqual(@as(usize, 3), tracking.deallocations);
    try std.testing.expectEqual(tracking.allocated_bytes, tracking.freed_bytes);
    try std.testing.expectEqual(@as(usize, 0), budget.usage().live_objects);
    budget.deinit();
}

test "token creation all actual OOM ordinals preserves resident and peak" {
    // Token constructor makes exactly one allocation; ordinal 1 fails after
    // the preexisting resident allocation at ordinal 0.
    var tracking = std.testing.FailingAllocator.init(std.testing.allocator, .{});
    var budget = s.LiveTokenBudget.init(.{ .max_live_objects = 2, .max_live_requested_bytes = 4096 });
    const resident = try s.TokenRef.create(tracking.allocator(), &budget);
    const before = budget.usage();
    tracking.fail_index = 1;
    try std.testing.expectError(error.OutOfMemory, s.TokenRef.create(tracking.allocator(), &budget));
    try std.testing.expectEqualDeep(before, budget.usage());
    try std.testing.expectEqual(@as(usize, 1), tracking.allocations);
    try std.testing.expect(resident.sameIdentity(resident));
    resident.release();
    budget.deinit();
    try std.testing.expectEqual(tracking.allocated_bytes, tracking.freed_bytes);
    var failing = std.testing.FailingAllocator.init(std.testing.allocator, .{ .fail_index = 0 });
    var empty = s.LiveTokenBudget.init(.{ .max_live_objects = 1, .max_live_requested_bytes = 4096 });
    try std.testing.expectError(error.OutOfMemory, s.TokenRef.create(failing.allocator(), &empty));
    try std.testing.expectEqualDeep(s.TokenUsage{}, empty.usage());
    empty.deinit();
}

test "pending admission cancellation exact ceiling and reclassification" {
    var b = s.PreparationBudget.init(7);
    var pending = try b.reserve(.frame_scratch, 7);
    try std.testing.expectEqualDeep(s.RequestedBytes{}, b.ledger());
    try std.testing.expectError(error.LimitExceeded, b.reserve(.staged_frame, 1));
    pending.cancel();
    var track = std.testing.FailingAllocator.init(std.testing.allocator, .{});
    var r = try b.reserve(.staged_frame, 7);
    const data = try track.allocator().alloc(u8, 7);
    var c = r.commit();
    c.reclassify(.resident_frame);
    try std.testing.expectEqualDeep(s.RequestedBytes{ .resident_frame = 7, .total = 7, .peak = 7 }, b.ledger());
    c.reclassify(.resident_frame);
    track.allocator().free(data);
    c.release();
    try std.testing.expectEqualDeep(s.RequestedBytes{ .peak = 7 }, b.ledger());
    b.deinit();
    var less = s.PreparationBudget.init(6);
    try std.testing.expectError(error.LimitExceeded, less.reserve(.staged_frame, 7));
    less.deinit();
    try std.testing.expectEqual(@as(usize, 1), track.allocations);
    try std.testing.expectEqual(@as(usize, 1), track.deallocations);
}

test "six simultaneous populations account literal allocation sizes" {
    var b = s.PreparationBudget.init(21);
    var tracking = std.testing.FailingAllocator.init(std.testing.allocator, .{});
    const populations = [_]s.Population{ .resident_frame, .prepared_focus, .staged_frame, .frame_scratch, .staged_focus, .focus_scratch };
    var buffers: [6][]u8 = undefined;
    var charges: [6]s.Charge = undefined;
    for (populations, 0..) |population, i| {
        var r = try b.reserve(population, i + 1);
        buffers[i] = try tracking.allocator().alloc(u8, i + 1);
        charges[i] = r.commit();
    }
    try std.testing.expectEqualDeep(s.RequestedBytes{ .resident_frame = 1, .prepared_focus = 2, .staged_frame = 3, .frame_scratch = 4, .staged_focus = 5, .focus_scratch = 6, .total = 21, .peak = 21 }, b.ledger());
    try std.testing.expectEqual(@as(usize, 21), tracking.allocated_bytes);
    for (&charges, buffers) |*charge, buffer| {
        tracking.allocator().free(buffer);
        charge.release();
    }
    b.deinit();
    try std.testing.expectEqual(@as(usize, 21), tracking.freed_bytes);
}

fn allocationHarness(allocator: std.mem.Allocator, b: *s.PreparationBudget) !void {
    var buffers: [3][]u8 = undefined;
    var charges: [3]s.Charge = undefined;
    var count: usize = 0;
    defer {
        while (count > 0) {
            count -= 1;
            allocator.free(buffers[count]);
            charges[count].release();
        }
    }
    for ([_]usize{ 3, 5, 7 }) |size| {
        var reservation = try b.reserve(.staged_focus, size);
        const data = allocator.alloc(u8, size) catch |err| {
            reservation.cancel();
            return err;
        };
        buffers[count] = data;
        charges[count] = reservation.commit();
        count += 1;
    }
}

test "test-local ledger harness sweeps each of three allocation OOM ordinals" {
    for (0..4) |ordinal| {
        var tracking = std.testing.FailingAllocator.init(std.testing.allocator, .{});
        var b = s.PreparationBudget.init(32);
        var r = try b.reserve(.resident_frame, 2);
        const resident = try tracking.allocator().alloc(u8, 2);
        @memcpy(resident, "ok");
        var c = r.commit();
        tracking.fail_index = ordinal + 1;
        if (ordinal < 3) try std.testing.expectError(error.OutOfMemory, allocationHarness(tracking.allocator(), &b)) else try allocationHarness(tracking.allocator(), &b);
        try std.testing.expectEqual(@as(usize, 2), b.ledger().total);
        try std.testing.expectEqual(@as(usize, 2), b.ledger().resident_frame);
        try std.testing.expectEqual(@as(usize, 0), b.ledger().staged_focus);
        try std.testing.expectEqualStrings("ok", resident);
        const expected_peak = [_]usize{ 2, 5, 10, 17 };
        try std.testing.expectEqual(expected_peak[ordinal], b.ledger().peak);
        try std.testing.expectEqual(@as(usize, 2), tracking.allocated_bytes - tracking.freed_bytes);
        tracking.allocator().free(resident);
        c.release();
        b.deinit();
        try std.testing.expectEqual(tracking.allocated_bytes, tracking.freed_bytes);
    }
}

test "zero actual requests carry independent obligations and mixed charges" {
    var b = s.PreparationBudget.init(0);
    var a = try b.reserve(.resident_frame, 0);
    var z = try b.reserve(.prepared_focus, 0);
    a.cancel();
    z.cancel();
    b.deinit();
    const allocator = std.testing.allocator;
    var x = try b.reserve(.staged_frame, 0);
    const empty = try allocator.alloc(u8, 0);
    var xc = x.commit();
    var y = try b.reserve(.staged_focus, 0);
    const empty2 = try allocator.alloc(u8, 0);
    var yc = y.commit();
    xc.reclassify(.resident_frame);
    try std.testing.expectEqualDeep(s.RequestedBytes{}, b.ledger());
    allocator.free(empty);
    xc.release();
    allocator.free(empty2);
    yc.release();
    b.deinit();
    var mixed = s.PreparationBudget.init(1);
    var zr = try mixed.reserve(.focus_scratch, 0);
    const zero = try allocator.alloc(u8, 0);
    var zc = zr.commit();
    var nr = try mixed.reserve(.frame_scratch, 1);
    const one = try allocator.alloc(u8, 1);
    var nc = nr.commit();
    allocator.free(one);
    nc.release();
    allocator.free(zero);
    zc.release();
    mixed.deinit();
    try std.testing.expectEqualDeep(s.RequestedBytes{ .peak = 1 }, mixed.ledger());
}

test "moving owning handles into another allocation keeps stable capabilities" {
    var tracking = std.testing.FailingAllocator.init(std.testing.allocator, .{});
    const allocator = tracking.allocator();
    var tokens = s.LiveTokenBudget.init(.{ .max_live_objects = 1, .max_live_requested_bytes = 4096 });
    var ledger = s.PreparationBudget.init(64);
    var token: ?s.TokenRef = try s.TokenRef.create(allocator, &tokens);
    var reservation = try ledger.reserve(.prepared_focus, 9);
    const data = try allocator.alloc(u8, 9);
    var charge: ?s.Charge = reservation.commit();
    const Owner = struct { token: s.TokenRef, charge: s.Charge, data: []u8 };
    const moved = try allocator.create(Owner);
    moved.* = .{ .token = token.?, .charge = charge.?, .data = data };
    token = null;
    charge = null;
    moved.token.release();
    allocator.free(moved.data);
    moved.charge.release();
    allocator.destroy(moved);
    tokens.deinit();
    ledger.deinit();
    try std.testing.expectEqual(tracking.allocated_bytes, tracking.freed_bytes);
    try std.testing.expectEqual(tracking.allocations, tracking.deallocations);
}

fn focusLimits() s.Limits {
    return .{ .max_nodes = 0, .max_id_bytes = 4096, .max_text_bytes = 0, .max_focus_actions = 64, .max_announcements = 0, .max_child_indices = 0, .max_owner_context_bytes = 0, .max_owned_bytes = 32768 };
}
fn id(bytes: []const u8) s.ActionId {
    return .{ .bytes = bytes };
}
fn expectIds(expected: []const []const u8, actual: []const s.ActionId) !void {
    try std.testing.expectEqual(expected.len, actual.len);
    for (expected, actual) |e, a| try std.testing.expectEqualStrings(e, a.bytes);
}
test "focus import ownership stable uniqueness null and UTF8" {
    var budget = s.PreparationBudget.init(32768);
    defer budget.deinit();
    var a = [_]u8{'A'};
    var unicode = [_]u8{ 0xc3, 0xa9 };
    var focus = try s.OwnedFocus.fresh(std.testing.allocator, &.{ id(&a), id("B"), id(&a), id(&unicode), id("e\u{301}"), id("") }, focusLimits(), &budget);
    defer focus.deinit();
    @memset(&a, '!');
    @memset(&unicode, '!');
    try expectIds(&.{ "A", "B", "é", "e\u{301}", "" }, focus.view().base);
    try std.testing.expectEqualStrings("A", focus.view().focused.?.bytes);
    var complete = try s.OwnedFocus.init(std.testing.allocator, .{ .base = &.{ id("A"), id("B"), id("C") }, .modal = &.{ id("X"), id("Y"), id("X") }, .focused = null, .restore = id("historical") }, focusLimits(), &budget);
    defer complete.deinit();
    try expectIds(&.{ "X", "Y" }, complete.view().modal.?);
    try std.testing.expect(complete.view().focused == null);
    try std.testing.expectEqualStrings("historical", complete.view().restore.?.bytes);
    var nullfocus = try s.OwnedFocus.init(std.testing.allocator, .{ .base = &.{id("A")}, .modal = null, .focused = null, .restore = null }, focusLimits(), &budget);
    defer nullfocus.deinit();
    try std.testing.expect(nullfocus.view().focused == null);
    try std.testing.expectError(error.InvalidFocus, s.OwnedFocus.init(std.testing.allocator, .{ .base = &.{id("A")}, .modal = null, .focused = id("B"), .restore = null }, focusLimits(), &budget));
    try std.testing.expectError(error.InvalidFocus, s.OwnedFocus.init(std.testing.allocator, .{ .base = &.{}, .modal = null, .focused = null, .restore = id("A") }, focusLimits(), &budget));
    try std.testing.expectError(error.InvalidUtf8, s.OwnedFocus.fresh(std.testing.allocator, &.{id("\xff")}, focusLimits(), &budget));
}

test "focus nonallocating source transitions null empty and singleton" {
    var budget = s.PreparationBudget.init(32768);
    defer budget.deinit();
    var track = std.testing.FailingAllocator.init(std.testing.allocator, .{});
    var a = try s.OwnedFocus.init(track.allocator(), .{ .base = &.{ id("A"), id("B"), id("C") }, .modal = null, .focused = null, .restore = null }, focusLimits(), &budget);
    defer a.deinit();
    const allocations = track.allocations;
    const first_next = a.next();
    try std.testing.expect(first_next != null);
    try std.testing.expectEqualStrings("B", first_next.?.bytes);
    try std.testing.expectEqualStrings("A", a.previous().?.bytes);
    try std.testing.expect(!a.requestFocus(id("unknown")));
    try std.testing.expect(a.requestFocus(id("C")));
    try std.testing.expectEqualStrings("A", a.next().?.bytes);
    try std.testing.expectEqualStrings("C", a.previous().?.bytes);
    try std.testing.expect(a.closeModal() == null);
    try std.testing.expectEqualStrings("C", a.view().focused.?.bytes);
    try std.testing.expectEqual(allocations, track.allocations);
    var b = try s.OwnedFocus.init(track.allocator(), .{ .base = &.{ id("A"), id("B"), id("C") }, .modal = null, .focused = null, .restore = null }, focusLimits(), &budget);
    defer b.deinit();
    try std.testing.expectEqualStrings("C", b.previous().?.bytes);
    var empty = try s.OwnedFocus.fresh(track.allocator(), &.{}, focusLimits(), &budget);
    defer empty.deinit();
    try std.testing.expect(empty.next() == null and empty.previous() == null and empty.closeModal() == null);
    var single = try s.OwnedFocus.fresh(track.allocator(), &.{id("one")}, focusLimits(), &budget);
    defer single.deinit();
    try std.testing.expectEqualStrings("one", single.next().?.bytes);
    try std.testing.expectEqualStrings("one", single.previous().?.bytes);
    var modal = try s.OwnedFocus.init(track.allocator(), .{ .base = &.{ id("A"), id("C") }, .modal = &.{ id("X"), id("Y") }, .focused = null, .restore = id("B") }, focusLimits(), &budget);
    defer modal.deinit();
    const before = track.allocations;
    try std.testing.expectEqualStrings("Y", modal.next().?.bytes);
    try std.testing.expectEqualStrings("A", modal.closeModal().?.bytes);
    try std.testing.expect(modal.view().modal == null and modal.view().restore == null);
    try std.testing.expectEqual(before, track.allocations);
}

test "focus staged source transitions aliasing and compact rebuild" {
    var budget = s.PreparationBudget.init(32768);
    defer budget.deinit();
    var track = std.testing.FailingAllocator.init(std.testing.allocator, .{});
    var focus = try s.OwnedFocus.fresh(track.allocator(), &.{ id("A"), id("B"), id("C") }, focusLimits(), &budget);
    defer focus.deinit();
    try std.testing.expectEqualStrings("C", focus.previous().?.bytes);
    try std.testing.expectEqualStrings("A", focus.next().?.bytes);
    try std.testing.expect(focus.requestFocus(id("B")));
    var x = [_]u8{'X'};
    var y = [_]u8{'Y'};
    try std.testing.expectEqualStrings("X", (try focus.openModal(&.{ id(&x), id(&y), id(&x) })).bytes);
    @memset(&x, '!');
    @memset(&y, '!');
    try expectIds(&.{ "X", "Y" }, focus.view().modal.?);
    try std.testing.expectEqualStrings("Y", focus.next().?.bytes);
    try focus.replaceBase(&.{ id("A"), id("C") });
    try std.testing.expectEqualStrings("Y", focus.view().focused.?.bytes);
    try std.testing.expectEqualStrings("B", focus.view().restore.?.bytes);
    const before = budget.ledger().total;
    try std.testing.expectEqualStrings("A", focus.closeModal().?.bytes);
    try std.testing.expectEqual(before, budget.ledger().total);
    try focus.replaceBase(focus.view().base);
    try expectIds(&.{ "A", "C" }, focus.view().base);
    const compact = budget.ledger().total;
    try std.testing.expect(compact < before);
    try focus.replaceBase(&.{ id("D"), id("E") });
    try focus.replaceBase(&.{ id("F"), id("G") });
    try std.testing.expectEqual(compact, budget.ledger().total);
    try std.testing.expect(focus.requestFocus(id("G")));
    _ = try focus.openModal(focus.view().base);
    try focus.replaceActive(&.{id("Z")});
    try std.testing.expectEqualStrings("G", focus.view().restore.?.bytes);
    try std.testing.expectEqualStrings("Z", focus.view().focused.?.bytes);
    const old = focus.view();
    const allocs = track.allocations;
    try std.testing.expectError(error.ModalAlreadyOpen, focus.openModal(&.{id("\xff")}));
    try std.testing.expectError(error.EmptyModal, focus.replaceActive(&.{}));
    try std.testing.expectEqual(allocs, track.allocations);
    try std.testing.expect(old.base.ptr == focus.view().base.ptr and old.modal.?.ptr == focus.view().modal.?.ptr);
    try std.testing.expectEqualStrings("G", focus.closeModal().?.bytes);
    try std.testing.expectError(error.EmptyModal, focus.openModal(&.{}));
    try focus.replaceActive(&.{});
    try std.testing.expect(focus.view().focused == null);
}

// Observes requested bytes, rather than allocator backing-block capacity.
// At every allocator boundary the previous commit/release must be complete;
// reserve is pending on alloc, while free precedes its matching release.
const FocusTracking = struct {
    budget: *s.PreparationBudget,
    live: usize = 0,
    peak: usize = 0,
    calls: usize = 0,
    frees: usize = 0,
    lengths: [64]usize = undefined,
    failed: bool = false,
    fn allocator(self: *FocusTracking) std.mem.Allocator {
        return .{ .ptr = self, .vtable = &.{ .alloc = allocate, .resize = resize, .remap = remap, .free = free } };
    }
    fn boundary(self: *FocusTracking) void {
        if (self.budget.ledger().total != self.live) self.failed = true;
    }
    fn allocate(ctx: *anyopaque, len: usize, alignment: std.mem.Alignment, ra: usize) ?[*]u8 {
        const self: *FocusTracking = @ptrCast(@alignCast(ctx));
        self.boundary();
        if (self.budget.pending_bytes != len or self.budget.active_reservations != 1) self.failed = true;
        const data = std.testing.allocator.rawAlloc(len, alignment, ra) orelse return null;
        self.lengths[self.calls] = len;
        self.calls += 1;
        self.live += len;
        self.peak = @max(self.peak, self.live);
        return data;
    }
    fn resize(_: *anyopaque, _: []u8, _: std.mem.Alignment, _: usize, _: usize) bool {
        return false;
    }
    fn remap(_: *anyopaque, _: []u8, _: std.mem.Alignment, _: usize, _: usize) ?[*]u8 {
        return null;
    }
    fn free(ctx: *anyopaque, data: []u8, alignment: std.mem.Alignment, ra: usize) void {
        const self: *FocusTracking = @ptrCast(@alignCast(ctx));
        self.boundary();
        std.testing.allocator.rawFree(data, alignment, ra);
        self.live -= data.len;
        self.frees += 1;
    }
    fn check(self: *FocusTracking) !void {
        self.boundary();
        try std.testing.expect(!self.failed);
        try std.testing.expectEqual(self.peak, self.budget.ledger().peak);
    }
};
test "focus independent literal requested schedule exact and one less overlap" {
    // This target's borrowed slice record is 16 bytes; arithmetic below is
    // independent literal expected requests, not a product accounting helper.
    try std.testing.expectEqual(@as(usize, 16), @sizeOf(s.ActionId));
    var budget = s.PreparationBudget.init(180);
    var tracking = FocusTracking{ .budget = &budget };
    var limits = focusLimits();
    limits.max_owned_bytes = 180;
    limits.max_id_bytes = 4;
    limits.max_focus_actions = 2;
    var focus = try s.OwnedFocus.fresh(tracking.allocator(), &.{ id("A"), id("B") }, limits, &budget);
    try std.testing.expectEqualSlices(usize, &.{ 48, 32, 32, 2 }, tracking.lengths[0..tracking.calls]);
    try std.testing.expectEqual(@as(usize, 66), budget.ledger().prepared_focus);
    try std.testing.expectEqual(@as(usize, 114), budget.ledger().peak);
    try tracking.check();
    try focus.replaceBase(&.{ id("C"), id("D") });
    try tracking.check();
    try std.testing.expectEqualSlices(usize, &.{ 48, 32, 32, 2, 48, 32, 32, 2 }, tracking.lengths[0..tracking.calls]);
    try std.testing.expectEqual(@as(usize, 180), budget.ledger().peak);
    try std.testing.expectEqual(@as(usize, 66), budget.ledger().total);
    focus.deinit();
    try tracking.check();
    budget.deinit();
    try std.testing.expectEqual(tracking.calls, tracking.frees);
    // One less local or aggregate: final 2-byte request is denied, after real
    // scratch/table/base success; actual peak is 178 and rollback leaves H=66.
    for ([_]bool{ false, true }) |aggregate| {
        var b = s.PreparationBudget.init(if (aggregate) 179 else 180);
        var tr = FocusTracking{ .budget = &b };
        var l = limits;
        l.max_owned_bytes = if (aggregate) 180 else 179;
        var f = try s.OwnedFocus.fresh(tr.allocator(), &.{ id("A"), id("B") }, l, &b);
        const old = f.view();
        try std.testing.expectError(error.LimitExceeded, f.replaceBase(&.{ id("C"), id("D") }));
        try std.testing.expect(old.base.ptr == f.view().base.ptr and old.focused.?.bytes.ptr == f.view().focused.?.bytes.ptr);
        try expectIds(&.{ "A", "B" }, old.base);
        try std.testing.expectEqual(@as(usize, 7), tr.calls);
        try std.testing.expectEqual(@as(usize, 178), b.ledger().peak);
        try std.testing.expectEqual(@as(usize, 66), b.ledger().total);
        try std.testing.expectEqual(@as(usize, 0), b.ledger().staged_focus + b.ledger().focus_scratch + b.pending_bytes + b.active_reservations);
        try tr.check();
        f.deinit();
        try tr.check();
        b.deinit();
    }
}
test "focus actual unrelated baseline aggregate independent ceiling" {
    // Test-local unrelated 7-byte storage; this does not construct a frame.
    for ([_]usize{ 187, 186 }) |ceiling| {
        var b = s.PreparationBudget.init(ceiling);
        var tr = FocusTracking{ .budget = &b };
        var reservation = try b.reserve(.resident_frame, 7);
        const unrelated = try tr.allocator().alloc(u8, 7);
        var charge = reservation.commit();
        var l = focusLimits();
        l.max_owned_bytes = 180;
        var f = try s.OwnedFocus.fresh(tr.allocator(), &.{ id("A"), id("B") }, l, &b);
        if (ceiling == 187) {
            try f.replaceBase(&.{ id("C"), id("D") });
            try std.testing.expectEqual(@as(usize, 187), b.ledger().peak);
        } else {
            try std.testing.expectError(error.LimitExceeded, f.replaceBase(&.{ id("C"), id("D") }));
            try std.testing.expectEqual(@as(usize, 185), b.ledger().peak);
            try expectIds(&.{ "A", "B" }, f.view().base);
        }
        try std.testing.expectEqual(@as(usize, 7), b.ledger().resident_frame);
        try std.testing.expectEqual(@as(usize, 73), b.ledger().total);
        try tr.check();
        f.deinit();
        try tr.check();
        tr.allocator().free(unrelated);
        charge.release();
        try tr.check();
        b.deinit();
    }
}
test "focus explicit zero slots simultaneous copied IDs and empty ID" {
    var zero = focusLimits();
    zero.max_owned_bytes = 0;
    zero.max_id_bytes = 0;
    zero.max_focus_actions = 0;
    var b = s.PreparationBudget.init(0);
    var tr = FocusTracking{ .budget = &b };
    var empty = try s.OwnedFocus.fresh(tr.allocator(), &.{}, zero, &b);
    empty.deinit();
    try std.testing.expectEqual(@as(usize, 0), tr.calls);
    b.deinit();
    var budget = s.PreparationBudget.init(32768);
    defer budget.deinit();
    var l = focusLimits();
    l.max_id_bytes = 0;
    l.max_focus_actions = 1;
    var f = try s.OwnedFocus.fresh(std.testing.allocator, &.{id("")}, l, &budget);
    defer f.deinit();
    try std.testing.expectEqualStrings("", f.view().focused.?.bytes);
    try std.testing.expectError(error.LimitExceeded, s.OwnedFocus.fresh(std.testing.allocator, &.{id("A")}, l, &budget));
    l.max_focus_actions = 0;
    try std.testing.expectError(error.LimitExceeded, s.OwnedFocus.fresh(std.testing.allocator, &.{id("")}, l, &budget));
    l = focusLimits();
    l.max_id_bytes = 3;
    var original = try s.OwnedFocus.fresh(std.testing.allocator, &.{ id("A"), id("B") }, l, &budget);
    defer original.deinit();
    const before = budget.ledger().total;
    try std.testing.expectError(error.LimitExceeded, original.replaceBase(&.{ id("C"), id("D") }));
    try std.testing.expectEqual(before, budget.ledger().total);
    try expectIds(&.{ "A", "B" }, original.view().base);
    // Base and modal slots count separately even if their identities coincide.
    l.max_id_bytes = 16;
    l.max_focus_actions = 1;
    try std.testing.expectError(error.LimitExceeded, s.OwnedFocus.init(std.testing.allocator, .{ .base = &.{id("A")}, .modal = &.{id("A")}, .focused = null, .restore = null }, l, &budget));
}

const FocusOperation = enum { replace_base, replace_base_modal, replace_active, replace_active_modal, open_modal };
fn initialFocus(a: std.mem.Allocator, b: *s.PreparationBudget, op: FocusOperation) !s.OwnedFocus {
    if (op == .replace_base_modal or op == .replace_active_modal) return s.OwnedFocus.init(a, .{ .base = &.{ id("A"), id("B") }, .modal = &.{ id("X"), id("Y") }, .focused = id("Y"), .restore = id("B") }, focusLimits(), b);
    var f = try s.OwnedFocus.fresh(a, &.{ id("A"), id("B") }, focusLimits(), b);
    _ = f.requestFocus(id("B"));
    return f;
}
fn mutateFocus(f: *s.OwnedFocus, op: FocusOperation) s.FocusError!void {
    // Every mutation uses old owned views as input; failure must preserve them.
    switch (op) {
        .replace_base => try f.replaceBase(f.view().base),
        .replace_base_modal => try f.replaceBase(f.view().modal.?),
        .replace_active => try f.replaceActive(f.view().base),
        .replace_active_modal => try f.replaceActive(f.view().modal.?),
        .open_modal => {
            _ = try f.openModal(f.view().base);
        },
    }
}
fn requestSchedule(op: FocusOperation) []const usize {
    // Explicit independent source-derived literal requests on this target.
    return switch (op) {
        .replace_base, .replace_active => &.{ 48, 32, 32, 2 },
        .replace_base_modal => &.{ 96, 48, 32, 32, 3 },
        .replace_active_modal => &.{ 96, 64, 32, 32, 4 },
        .open_modal => &.{ 96, 32, 32, 32, 2 },
    };
}
fn sameFocusStorage(a: s.FocusSpec, b: s.FocusSpec) !void {
    try std.testing.expect(a.base.ptr == b.base.ptr and a.base.len == b.base.len);
    try std.testing.expectEqual(a.modal != null, b.modal != null);
    if (a.modal) |m| try std.testing.expect(m.ptr == b.modal.?.ptr and m.len == b.modal.?.len);
    try std.testing.expectEqual(a.focused != null, b.focused != null);
    if (a.focused) |f| try std.testing.expect(f.bytes.ptr == b.focused.?.bytes.ptr);
    try std.testing.expectEqual(a.restore != null, b.restore != null);
    if (a.restore) |f| try std.testing.expect(f.bytes.ptr == b.restore.?.bytes.ptr);
}
test "focus every actual init and fresh OOM ordinal real baseline rollback" {
    for ([_]bool{ false, true }) |complete| {
        const schedule: []const usize = if (complete) &.{ 96, 80, 32, 32, 5 } else &.{ 48, 32, 32, 2 };
        // A successful run establishes actual count and each independent size.
        {
            var b = s.PreparationBudget.init(32768);
            var tr = FocusTracking{ .budget = &b };
            var f = if (complete) try s.OwnedFocus.init(tr.allocator(), .{ .base = &.{ id("A"), id("B") }, .modal = &.{ id("X"), id("Y") }, .focused = id("Y"), .restore = id("R") }, focusLimits(), &b) else try s.OwnedFocus.fresh(tr.allocator(), &.{ id("A"), id("B") }, focusLimits(), &b);
            try std.testing.expectEqualSlices(usize, schedule, tr.lengths[0..tr.calls]);
            try tr.check();
            f.deinit();
            try tr.check();
            b.deinit();
        }
        for (0..schedule.len) |ordinal| {
            var b = s.PreparationBudget.init(32768);
            var failing = std.testing.FailingAllocator.init(std.testing.allocator, .{});
            var reservation = try b.reserve(.resident_frame, 7);
            const unrelated = try failing.allocator().alloc(u8, 7);
            @memset(unrelated, 0x5a);
            var charge = reservation.commit();
            failing.fail_index = failing.alloc_index + ordinal;
            if (complete) {
                try std.testing.expectError(error.OutOfMemory, s.OwnedFocus.init(failing.allocator(), .{ .base = &.{ id("A"), id("B") }, .modal = &.{ id("X"), id("Y") }, .focused = id("Y"), .restore = id("R") }, focusLimits(), &b));
            } else {
                try std.testing.expectError(error.OutOfMemory, s.OwnedFocus.fresh(failing.allocator(), &.{ id("A"), id("B") }, focusLimits(), &b));
            }
            try std.testing.expect(failing.has_induced_failure);
            try std.testing.expectEqual(@as(usize, 7), b.ledger().total);
            try std.testing.expectEqual(@as(usize, 7), b.ledger().resident_frame);
            try std.testing.expectEqual(@as(usize, 0), b.ledger().prepared_focus + b.ledger().staged_focus + b.ledger().focus_scratch + b.pending_bytes + b.active_reservations);
            var expected_peak: usize = 7;
            for (schedule[0..ordinal]) |bytes| expected_peak += bytes;
            try std.testing.expectEqual(expected_peak, b.ledger().peak);
            for (unrelated) |byte| try std.testing.expectEqual(@as(u8, 0x5a), byte);
            failing.allocator().free(unrelated);
            charge.release();
            b.deinit();
            try std.testing.expectEqual(failing.allocated_bytes, failing.freed_bytes);
            try std.testing.expectEqual(failing.allocations, failing.deallocations);
        }
    }
}
test "focus every actual staged mutation OOM ordinal aliased views and peaks" {
    for (std.enums.values(FocusOperation)) |op| {
        const schedule = requestSchedule(op);
        {
            var b = s.PreparationBudget.init(32768);
            var tr = FocusTracking{ .budget = &b };
            var f = try initialFocus(tr.allocator(), &b, op);
            const start = tr.calls;
            try mutateFocus(&f, op);
            try std.testing.expectEqualSlices(usize, schedule, tr.lengths[start..tr.calls]);
            try tr.check();
            f.deinit();
            try tr.check();
            b.deinit();
        }
        for (0..schedule.len) |ordinal| {
            var b = s.PreparationBudget.init(32768);
            var failing = std.testing.FailingAllocator.init(std.testing.allocator, .{});
            var reservation = try b.reserve(.resident_frame, 7);
            const unrelated = try failing.allocator().alloc(u8, 7);
            @memset(unrelated, 0x5a);
            var charge = reservation.commit();
            var f = try initialFocus(failing.allocator(), &b, op);
            const before = b.ledger();
            const old = f.view();
            const old_table = f.table.ptr;
            const old_bytes = f.bytes.ptr;
            failing.fail_index = failing.alloc_index + ordinal;
            try std.testing.expectError(error.OutOfMemory, mutateFocus(&f, op));
            try std.testing.expect(failing.has_induced_failure);
            try sameFocusStorage(old, f.view());
            try std.testing.expect(old_table == f.table.ptr and old_bytes == f.bytes.ptr);
            try expectIds(&.{ "A", "B" }, old.base);
            if (old.modal) |m| {
                try expectIds(&.{ "X", "Y" }, m);
                try std.testing.expectEqualStrings("B", old.restore.?.bytes);
                try std.testing.expectEqualStrings("Y", old.focused.?.bytes);
            } else {
                try std.testing.expectEqualStrings("B", old.focused.?.bytes);
            }
            var ledger = b.ledger();
            var overlap_peak = before.total;
            for (schedule[0..ordinal]) |bytes| overlap_peak += bytes;
            try std.testing.expectEqual(@max(before.peak, overlap_peak), ledger.peak);
            ledger.peak = before.peak;
            try std.testing.expectEqualDeep(before, ledger);
            try std.testing.expectEqual(@as(usize, 0), b.pending_bytes + b.active_reservations);
            try std.testing.expectEqual(before.total, failing.allocated_bytes - failing.freed_bytes);
            f.deinit();
            for (unrelated) |byte| try std.testing.expectEqual(@as(u8, 0x5a), byte);
            failing.allocator().free(unrelated);
            charge.release();
            b.deinit();
            try std.testing.expectEqual(failing.allocated_bytes, failing.freed_bytes);
            try std.testing.expectEqual(failing.allocations, failing.deallocations);
        }
    }
}
test "focus moving owner retains stable budget and allocator context" {
    var b = s.PreparationBudget.init(32768);
    var track = FocusTracking{ .budget = &b };
    var original = try s.OwnedFocus.init(track.allocator(), .{ .base = &.{ id("A"), id("B") }, .modal = &.{id("X")}, .focused = null, .restore = id("B") }, focusLimits(), &b);
    const Owner = struct { focus: s.OwnedFocus };
    var moved = Owner{ .focus = original };
    original = undefined;
    try std.testing.expectEqualStrings("X", moved.focus.previous().?.bytes);
    try std.testing.expectEqualStrings("B", moved.focus.closeModal().?.bytes);
    try moved.focus.replaceBase(moved.focus.view().base);
    try track.check();
    moved.focus.deinit();
    try track.check();
    b.deinit();
}

test "focus complete import poisons every source and rejects every UTF8 position" {
    var b = s.PreparationBudget.init(32768);
    defer b.deinit();
    var a = [_]u8{'A'};
    var x = [_]u8{'X'};
    var history = [_]u8{ 'R', 0xc3, 0xa9 };
    var f = try s.OwnedFocus.init(std.testing.allocator, .{ .base = &.{id(&a)}, .modal = &.{id(&x)}, .focused = id(&x), .restore = id(&history) }, focusLimits(), &b);
    defer f.deinit();
    @memset(&a, '!');
    @memset(&x, '!');
    @memset(&history, '!');
    try expectIds(&.{"A"}, f.view().base);
    try expectIds(&.{"X"}, f.view().modal.?);
    try std.testing.expectEqualStrings("X", f.view().focused.?.bytes);
    try std.testing.expectEqualStrings("Ré", f.view().restore.?.bytes);
    const invalid = id("\xff");
    const valid = id("A");
    const specs = [_]s.FocusSpec{
        .{ .base = &.{invalid}, .modal = null, .focused = null, .restore = null },
        .{ .base = &.{valid}, .modal = &.{invalid}, .focused = null, .restore = null },
        .{ .base = &.{valid}, .modal = null, .focused = invalid, .restore = null },
        .{ .base = &.{valid}, .modal = &.{valid}, .focused = null, .restore = invalid },
        .{ .base = &.{valid}, .modal = &.{valid}, .focused = id("B"), .restore = null },
        .{ .base = &.{valid}, .modal = &.{}, .focused = null, .restore = null },
    };
    for (specs, 0..) |spec, i| try std.testing.expectError(if (i < 4) error.InvalidUtf8 else if (i == 4) error.InvalidFocus else error.EmptyModal, s.OwnedFocus.init(std.testing.allocator, spec, focusLimits(), &b));
    const old = f.view();
    const before = b.ledger().total;
    try std.testing.expectError(error.InvalidUtf8, f.replaceBase(&.{invalid}));
    try std.testing.expectError(error.InvalidUtf8, f.replaceActive(&.{invalid}));
    try sameFocusStorage(old, f.view());
    try std.testing.expectEqual(before, b.ledger().total);
}
test "focus mutation caller poison restore null and empty base close" {
    var b = s.PreparationBudget.init(32768);
    defer b.deinit();
    var f = try s.OwnedFocus.fresh(std.testing.allocator, &.{id("A")}, focusLimits(), &b);
    defer f.deinit();
    var base = [_]u8{ 0xc3, 0xa9 };
    try f.replaceBase(&.{ id(&base), id(&base) });
    @memset(&base, '!');
    try expectIds(&.{"é"}, f.view().base);
    var modal = [_]u8{'X'};
    _ = try f.openModal(&.{id(&modal)});
    @memset(&modal, '!');
    try std.testing.expectEqualStrings("X", f.view().focused.?.bytes);
    var active = [_]u8{'Y'};
    try f.replaceActive(&.{id(&active)});
    @memset(&active, '!');
    try std.testing.expectEqualStrings("Y", f.view().focused.?.bytes);
    try std.testing.expectEqualStrings("é", f.view().restore.?.bytes);
    try std.testing.expectEqualStrings("é", f.closeModal().?.bytes);
    var nullfocus = try s.OwnedFocus.init(std.testing.allocator, .{ .base = &.{ id("A"), id("B") }, .modal = null, .focused = null, .restore = null }, focusLimits(), &b);
    defer nullfocus.deinit();
    _ = try nullfocus.openModal(&.{id("X")});
    try std.testing.expect(nullfocus.view().restore == null);
    try std.testing.expectEqualStrings("A", nullfocus.closeModal().?.bytes);
    _ = try nullfocus.openModal(&.{id("X")});
    try nullfocus.replaceBase(&.{});
    try std.testing.expect(nullfocus.closeModal() == null);
    try std.testing.expect(nullfocus.view().focused == null);
}
test "focus historical retirement literal accounting and constructor ceilings" {
    var b = s.PreparationBudget.init(32768);
    var tr = FocusTracking{ .budget = &b };
    var f = try s.OwnedFocus.init(tr.allocator(), .{ .base = &.{ id("A"), id("B") }, .modal = &.{ id("X"), id("Y") }, .focused = id("Y"), .restore = id("R") }, focusLimits(), &b);
    // Five canonical IDs (including historical R), 2+2 order slots, 5 bytes.
    try std.testing.expectEqual(@as(usize, 149), b.ledger().prepared_focus);
    try std.testing.expectEqualStrings("A", f.closeModal().?.bytes);
    try std.testing.expectEqual(@as(usize, 149), b.ledger().prepared_focus);
    try f.replaceBase(f.view().base);
    try std.testing.expectEqual(@as(usize, 66), b.ledger().prepared_focus);
    try expectIds(&.{ "A", "B" }, f.table);
    try tr.check();
    f.deinit();
    try tr.check();
    b.deinit();
    for ([_]usize{ 114, 113 }) |ceiling| {
        var budget = s.PreparationBudget.init(32768);
        var l = focusLimits();
        l.max_owned_bytes = ceiling;
        var tracking = std.testing.FailingAllocator.init(std.testing.allocator, .{});
        if (ceiling == 114) {
            var exact = try s.OwnedFocus.fresh(tracking.allocator(), &.{ id("A"), id("B") }, l, &budget);
            exact.deinit();
        } else {
            try std.testing.expectError(error.LimitExceeded, s.OwnedFocus.fresh(tracking.allocator(), &.{ id("A"), id("B") }, l, &budget));
            try std.testing.expectEqual(@as(usize, 3), tracking.allocations);
            try std.testing.expectEqual(@as(usize, 112), budget.ledger().peak);
        }
        budget.deinit();
        try std.testing.expectEqual(tracking.allocated_bytes, tracking.freed_bytes);
    }
    var budget = s.PreparationBudget.init(32768);
    var l = focusLimits();
    l.max_id_bytes = 1;
    try std.testing.expectError(error.LimitExceeded, s.OwnedFocus.fresh(std.testing.allocator, &.{ id("A"), id("B") }, l, &budget));
    budget.deinit();
}
test "focus zero byte identities sweep actual metadata requests" {
    // Empty ID still owns two metadata arrays; zero-byte buffer is skipped.
    const sizes = [_]usize{ 32, 16, 16 };
    var b = s.PreparationBudget.init(32768);
    var tr = FocusTracking{ .budget = &b };
    var f = try s.OwnedFocus.fresh(tr.allocator(), &.{id("")}, focusLimits(), &b);
    try std.testing.expectEqualSlices(usize, &sizes, tr.lengths[0..tr.calls]);
    f.deinit();
    try tr.check();
    b.deinit();
    for (0..sizes.len) |ordinal| {
        var budget = s.PreparationBudget.init(32768);
        var failing = std.testing.FailingAllocator.init(std.testing.allocator, .{ .fail_index = ordinal });
        try std.testing.expectError(error.OutOfMemory, s.OwnedFocus.fresh(failing.allocator(), &.{id("")}, focusLimits(), &budget));
        try std.testing.expect(failing.has_induced_failure);
        budget.deinit();
        try std.testing.expectEqual(failing.allocated_bytes, failing.freed_bytes);
    }
}

// Baseline witness: OwnerSpec currently borrows context; frames must snapshot it.
test "frame ownership context poisoning witness" {
    var tokens = s.LiveTokenBudget.init(.{ .max_live_objects = 1, .max_live_requested_bytes = 4096 });
    defer tokens.deinit();
    const token = try s.TokenRef.create(std.testing.allocator, &tokens);
    defer token.release();
    var context = [_]u8{ 'o', 'l', 'd' };
    const owner = s.OwnerSpec{ .resident = token, .hold = null, .context = &context, .draft = null };
    var budget = s.PreparationBudget.init(65536);
    defer budget.deinit();
    const input = s.FrameInput{ .nodes = &.{frameNode("root", .application, null, 0, 0)}, .child_indices = &.{}, .announcements = &.{}, .owner = owner, .focus = .{ .base = &.{}, .modal = null, .focused = null, .restore = null } };
    var frame = try s.OwnedFrame.init(std.testing.allocator, &budget, input, frameLimits());
    defer frame.deinit();
    @memset(&context, 'x');
    try std.testing.expectEqualStrings("old", frame.view().owner.context);
}

fn frameLimits() s.Limits {
    return .{ .max_nodes = 100, .max_id_bytes = 4096, .max_text_bytes = 4096, .max_focus_actions = 100, .max_announcements = 100, .max_child_indices = 100, .max_owner_context_bytes = 4096, .max_owned_bytes = 65536 };
}

fn frameNode(node_id: []const u8, role: s.NodeRole, parent: ?usize, start: usize, count: usize) s.NodeSpec {
    return .{ .id = .{ .bytes = node_id }, .role = role, .name = "", .description = "", .value = null, .enabled = true, .selected = false, .focusable = false, .hierarchy_level = null, .expanded = null, .action = null, .bounds = null, .visible = true, .parent = parent, .child_start = start, .child_count = count };
}

const FrameFixture = struct {
    base: [3]s.ActionId = .{ .{ .bytes = "A" }, .{ .bytes = "B" }, .{ .bytes = "A" } },
    modal: [2]s.ActionId = .{ .{ .bytes = "X" }, .{ .bytes = "Y" } },
    nodes: [4]s.NodeSpec,
    children: [3]usize = .{ 1, 3, 2 },
    announcements: [2]s.AnnouncementSpec = .{ .{ .kind = .status, .code = "ready", .message = "α" }, .{ .kind = .@"error", .code = "bad", .message = "β" } },
    fn init() FrameFixture {
        var result = FrameFixture{ .nodes = .{ frameNode("r", .application, null, 0, 2), frameNode("g", .group, 0, 2, 1), frameNode("l", .checkbox, 1, 3, 0), frameNode("s", .option, 0, 3, 0) } };
        for (&result.nodes) |*n| {
            n.name = "name";
            n.description = "desc";
        }
        result.nodes[2].value = "";
        result.nodes[2].enabled = false;
        result.nodes[2].selected = true;
        result.nodes[2].focusable = true;
        result.nodes[2].hierarchy_level = 2;
        result.nodes[2].expanded = false;
        result.nodes[2].action = id("A");
        result.nodes[3].focusable = true;
        result.nodes[3].action = id("B");
        result.nodes[0].bounds = .{ .min = .{ 0, 0 }, .max = .{ 10, 10 } };
        result.nodes[1].bounds = .{ .min = .{ 10, 0 }, .max = .{ 20, 10 } };
        result.nodes[2].bounds = .{ .min = .{ 0, 0 }, .max = .{ 10, 10 } };
        result.nodes[2].visible = false;
        result.nodes[3].bounds = .{ .min = .{ 20, 20 }, .max = .{ 30, 30 } };
        return result;
    }
    fn input(self: *const FrameFixture, token: s.TokenRef) s.FrameInput {
        return .{ .nodes = &self.nodes, .child_indices = &self.children, .announcements = &self.announcements, .owner = .{ .resident = token, .hold = 0, .context = "guide", .draft = token }, .focus = .{ .base = &self.base, .modal = &self.modal, .focused = null, .restore = .{ .bytes = "Z" } } };
    }
};
fn expectFrame(expected: s.FrameInput, actual: s.FrameView) !void {
    try std.testing.expectEqual(expected.nodes.len, actual.nodes.len);
    for (expected.nodes, actual.nodes) |a, b| {
        try std.testing.expectEqualStrings(a.id.bytes, b.id.bytes);
        try std.testing.expectEqual(a.role, b.role);
        try std.testing.expectEqualStrings(a.name, b.name);
        try std.testing.expectEqualStrings(a.description, b.description);
        try std.testing.expectEqual(a.value != null, b.value != null);
        if (a.value) |v| try std.testing.expectEqualStrings(v, b.value.?);
        try std.testing.expectEqual(a.enabled, b.enabled);
        try std.testing.expectEqual(a.selected, b.selected);
        try std.testing.expectEqual(a.focusable, b.focusable);
        try std.testing.expectEqual(a.hierarchy_level, b.hierarchy_level);
        try std.testing.expectEqual(a.expanded, b.expanded);
        try std.testing.expectEqual(a.action != null, b.action != null);
        if (a.action) |v| try std.testing.expectEqualStrings(v.bytes, b.action.?.bytes);
        try std.testing.expectEqualDeep(a.bounds, b.bounds);
        try std.testing.expectEqual(a.visible, b.visible);
        try std.testing.expectEqual(a.parent, b.parent);
        try std.testing.expectEqual(a.child_start, b.child_start);
        try std.testing.expectEqual(a.child_count, b.child_count);
    }
    try std.testing.expectEqualSlices(usize, expected.child_indices, actual.child_indices);
    try std.testing.expectEqual(expected.announcements.len, actual.announcements.len);
    for (expected.announcements, actual.announcements) |a, b| {
        try std.testing.expectEqual(a.kind, b.kind);
        try std.testing.expectEqualStrings(a.code, b.code);
        try std.testing.expectEqualStrings(a.message, b.message);
    }
    try std.testing.expect(expected.owner.matches(actual.owner));
    try expectIds(&.{ "A", "B" }, actual.focus.base);
    try expectIds(&.{ "X", "Y" }, actual.focus.modal.?);
    try std.testing.expect(actual.focus.focused == null);
    try std.testing.expectEqualStrings("Z", actual.focus.restore.?.bytes);
}
fn expectFrameIdentity(old: s.FrameView, actual: s.FrameView) !void {
    try std.testing.expect(old.nodes.ptr == actual.nodes.ptr and old.child_indices.ptr == actual.child_indices.ptr and old.announcements.ptr == actual.announcements.ptr);
    try std.testing.expect(old.owner.context.ptr == actual.owner.context.ptr and old.focus.base.ptr == actual.focus.base.ptr and old.focus.modal.?.ptr == actual.focus.modal.?.ptr and old.focus.restore.?.bytes.ptr == actual.focus.restore.?.bytes.ptr);
    for (old.nodes, actual.nodes) |a, b| {
        try std.testing.expect(a.id.bytes.ptr == b.id.bytes.ptr and a.name.ptr == b.name.ptr and a.description.ptr == b.description.ptr);
        if (a.value) |v| try std.testing.expect(v.ptr == b.value.?.ptr);
        if (a.action) |v| try std.testing.expect(v.bytes.ptr == b.action.?.bytes.ptr);
    }
    for (old.announcements, actual.announcements) |a, b| try std.testing.expect(a.code.ptr == b.code.ptr and a.message.ptr == b.message.ptr);
}
test "frame complete snapshot culling modal clone and aliased atomic replacement" {
    var tokens = s.LiveTokenBudget.init(.{ .max_live_objects = 1, .max_live_requested_bytes = 4096 });
    defer tokens.deinit();
    const token = try s.TokenRef.create(std.testing.allocator, &tokens);
    defer token.release();
    var budget = s.PreparationBudget.init(65536);
    defer budget.deinit();
    var fixture = FrameFixture.init();
    var slot = s.FrameSlot.init(std.testing.allocator, &budget);
    defer slot.deinit();
    try std.testing.expect(slot.view() == null);
    try slot.replace(fixture.input(token), frameLimits());
    const old = slot.view().?;
    try expectFrame(fixture.input(token), old);
    var sentinel = [_]usize{ 999, 998 };
    const clip = try s.Rect.fromBounds(.{ 0, 0 }, .{ 10, 10 });
    try std.testing.expectError(error.CapacityExceeded, old.visibleNodes(clip, sentinel[0..0]));
    try std.testing.expectEqualSlices(usize, &.{ 999, 998 }, &sentinel);
    try std.testing.expectEqualSlices(usize, &.{0}, try old.visibleNodes(clip, &sentinel));
    try std.testing.expect(old.node(.{ .bytes = "s" }) != null and old.node(.{ .bytes = "missing" }) == null);
    var helper = try s.OwnedFocus.cloneFromFrame(std.testing.allocator, old, frameLimits(), &budget);
    defer helper.deinit();
    try std.testing.expect(helper.view().focused == null);
    try std.testing.expectEqualStrings("Y", helper.previous().?.bytes);
    try std.testing.expectEqualStrings("A", helper.closeModal().?.bytes);
    const input = s.FrameInput{ .nodes = old.nodes, .child_indices = old.child_indices, .announcements = old.announcements, .owner = old.owner, .focus = old.focus };
    try slot.replace(input, frameLimits()); // old remains alive through all copying
    try expectFrame(fixture.input(token), slot.view().?);
    try std.testing.expectEqual(@as(usize, 0), budget.ledger().staged_frame + budget.ledger().frame_scratch + budget.ledger().staged_focus + budget.ledger().focus_scratch);
}
test "frame every allocation OOM init replacement with helper and clone" {
    var tokens = s.LiveTokenBudget.init(.{ .max_live_objects = 1, .max_live_requested_bytes = 4096 });
    defer tokens.deinit();
    const token = try s.TokenRef.create(std.testing.allocator, &tokens);
    defer token.release();
    var fixture = FrameFixture.init();
    var allocations: usize = 0;
    {
        var budget = s.PreparationBudget.init(65536);
        var tracking = FocusTracking{ .budget = &budget };
        var frame = try s.OwnedFrame.init(tracking.allocator(), &budget, fixture.input(token), frameLimits());
        allocations = tracking.calls;
        // Each requested record, copied byte, stack, order and duplicate ID copy.
        const expected = [_]usize{ 32, 4 * @sizeOf(s.NodeSpec), 24, 2 * @sizeOf(s.AnnouncementSpec), 55, 96, 80, 32, 32, 5 };
        try std.testing.expectEqualSlices(usize, &expected, tracking.lengths[0..tracking.calls]);
        try std.testing.expectEqual(@as(usize, 10), allocations);
        try std.testing.expectEqual(@as(usize, 0), budget.ledger().prepared_focus + budget.ledger().focus_scratch);
        try tracking.check();
        frame.deinit();
        try tracking.check();
        budget.deinit();
    }
    for (0..allocations) |ordinal| {
        var budget = s.PreparationBudget.init(65536);
        var tracking = FocusTracking{ .budget = &budget };
        var fail = std.testing.FailingAllocator.init(tracking.allocator(), .{ .fail_index = ordinal });
        try std.testing.expectError(error.OutOfMemory, s.OwnedFrame.init(fail.allocator(), &budget, fixture.input(token), frameLimits()));
        try std.testing.expectEqual(@as(usize, 0), budget.ledger().total + budget.pending_bytes + budget.active_reservations);
        try tracking.check();
        budget.deinit();
    }
    for (0..allocations) |ordinal| {
        var budget = s.PreparationBudget.init(65536);
        var tracking = FocusTracking{ .budget = &budget };
        var fail = std.testing.FailingAllocator.init(tracking.allocator(), .{});
        var slot = s.FrameSlot.init(fail.allocator(), &budget);
        try slot.replace(fixture.input(token), frameLimits());
        const old = slot.view().?;
        const clone_start = fail.alloc_index;
        var helper = try s.OwnedFocus.cloneFromFrame(fail.allocator(), old, frameLimits(), &budget);
        try std.testing.expectEqual(@as(usize, 5), fail.alloc_index - clone_start);
        const helper_old = helper.view();
        const before = budget.ledger().total;
        fail.fail_index = fail.alloc_index + ordinal;
        try std.testing.expectError(error.OutOfMemory, slot.replace(.{ .nodes = old.nodes, .child_indices = old.child_indices, .announcements = old.announcements, .owner = old.owner, .focus = helper_old }, frameLimits()));
        try expectFrameIdentity(old, slot.view().?);
        try expectFrame(fixture.input(token), old);
        try std.testing.expect(helper_old.base.ptr == helper.view().base.ptr and helper_old.modal.?.ptr == helper.view().modal.?.ptr);
        try std.testing.expectEqualStrings("Z", helper.view().restore.?.bytes);
        try std.testing.expectEqual(before, budget.ledger().total);
        try std.testing.expectEqual(@as(usize, 0), budget.ledger().staged_frame + budget.ledger().frame_scratch + budget.pending_bytes + budget.active_reservations);
        var sentinel = [_]usize{999};
        try std.testing.expectEqualSlices(usize, &.{0}, try old.visibleNodes(try s.Rect.fromBounds(.{ 0, 0 }, .{ 10, 10 }), &sentinel));
        try tracking.check();
        helper.deinit();
        slot.deinit();
        try tracking.check();
        budget.deinit();
    }
    for (0..5) |ordinal| {
        var budget = s.PreparationBudget.init(65536);
        var tracking = FocusTracking{ .budget = &budget };
        var frame = try s.OwnedFrame.init(tracking.allocator(), &budget, fixture.input(token), frameLimits());
        const old = frame.view();
        const before = budget.ledger().total;
        var fail = std.testing.FailingAllocator.init(tracking.allocator(), .{ .fail_index = ordinal });
        try std.testing.expectError(error.OutOfMemory, s.OwnedFocus.cloneFromFrame(fail.allocator(), old, frameLimits(), &budget));
        try expectFrameIdentity(old, frame.view());
        try expectFrame(fixture.input(token), old);
        try std.testing.expectEqual(before, budget.ledger().total);
        try std.testing.expectEqual(@as(usize, 0), budget.ledger().prepared_focus + budget.ledger().focus_scratch + budget.pending_bytes);
        try tracking.check();
        frame.deinit();
        try tracking.check();
        budget.deinit();
    }
    std.debug.print("frame OOM ordinals init/replacement 0..9 clone 0..4; NodeSpec={d} AnnouncementSpec={d}\n", .{ @sizeOf(s.NodeSpec), @sizeOf(s.AnnouncementSpec) });
}

test "frame exact literal all limits whole nested candidate and shared overlap" {
    try std.testing.expectEqual(@as(usize, 160), @sizeOf(s.NodeSpec));
    try std.testing.expectEqual(@as(usize, 40), @sizeOf(s.AnnouncementSpec));
    var tokens = s.LiveTokenBudget.init(.{ .max_live_objects = 1, .max_live_requested_bytes = 4096 });
    defer tokens.deinit();
    const token = try s.TokenRef.create(std.testing.allocator, &tokens);
    defer token.release();
    var fixture = FrameFixture.init();
    // Payload 640+24+80+55+80+32+32+5=948; topology32+focus96=128.
    const exact = s.Limits{ .max_nodes = 4, .max_id_bytes = 11, .max_text_bytes = 44, .max_focus_actions = 4, .max_announcements = 2, .max_child_indices = 3, .max_owner_context_bytes = 5, .max_owned_bytes = 1076 };
    for (0..9) |case| {
        var budget = s.PreparationBudget.init(65536);
        defer budget.deinit();
        var limits = exact;
        switch (case) {
            0 => {},
            1 => limits.max_nodes -= 1,
            2 => limits.max_id_bytes -= 1,
            3 => limits.max_text_bytes -= 1,
            4 => limits.max_focus_actions -= 1,
            5 => limits.max_announcements -= 1,
            6 => limits.max_child_indices -= 1,
            7 => limits.max_owner_context_bytes -= 1,
            8 => limits.max_owned_bytes -= 1,
            else => unreachable,
        }
        if (case == 0) {
            var tracking = FocusTracking{ .budget = &budget };
            var frame = try s.OwnedFrame.init(tracking.allocator(), &budget, fixture.input(token), limits);
            try std.testing.expectEqual(@as(usize, 948), budget.ledger().resident_frame);
            try std.testing.expectEqual(@as(usize, 1076), tracking.peak);
            try tracking.check();
            frame.deinit();
            try tracking.check();
        } else {
            try std.testing.expectError(error.LimitExceeded, s.OwnedFrame.init(std.testing.allocator, &budget, fixture.input(token), limits));
            try std.testing.expectEqual(@as(usize, 0), budget.ledger().total + budget.pending_bytes);
        }
    }
    for ([_]bool{ false, true }) |one_less| {
        var budget = s.PreparationBudget.init(if (one_less) 2172 else 2173);
        var tracking = FocusTracking{ .budget = &budget };
        var slot = s.FrameSlot.init(tracking.allocator(), &budget);
        try slot.replace(fixture.input(token), exact);
        const old = slot.view().?;
        var helper = try s.OwnedFocus.cloneFromFrame(tracking.allocator(), old, exact, &budget);
        const helper_old = helper.view();
        if (one_less) {
            try std.testing.expectError(error.LimitExceeded, slot.replace(fixture.input(token), exact));
            try expectFrameIdentity(old, slot.view().?);
            try expectFrame(fixture.input(token), old);
            try std.testing.expectEqual(@as(usize, 2168), tracking.peak);
        } else {
            try slot.replace(fixture.input(token), exact);
            try std.testing.expectEqual(@as(usize, 2173), tracking.peak);
            try expectFrame(fixture.input(token), slot.view().?);
        }
        try std.testing.expect(helper_old.base.ptr == helper.view().base.ptr);
        try std.testing.expectEqual(@as(usize, 1097), budget.ledger().total);
        try tracking.check();
        helper.deinit();
        slot.deinit();
        try tracking.check();
        budget.deinit();
    }
    // The frame-local C+S includes its already copied IDs and metadata before
    // embedded focus acquisition; the narrower focus alone would fit 1075.
    var local_budget = s.PreparationBudget.init(65536);
    defer local_budget.deinit();
    var local = exact;
    local.max_owned_bytes = 1075;
    var tracking = FocusTracking{ .budget = &local_budget };
    try std.testing.expectError(error.LimitExceeded, s.OwnedFrame.init(tracking.allocator(), &local_budget, fixture.input(token), local));
    try std.testing.expectEqual(@as(usize, 1071), tracking.peak);
    try std.testing.expectEqual(@as(usize, 9), tracking.calls);
    try tracking.check();
}
test "frame validation failures preserve complete resident helper and old views" {
    var tokens = s.LiveTokenBudget.init(.{ .max_live_objects = 1, .max_live_requested_bytes = 4096 });
    defer tokens.deinit();
    const token = try s.TokenRef.create(std.testing.allocator, &tokens);
    defer token.release();
    var budget = s.PreparationBudget.init(65536);
    defer budget.deinit();
    var fixture = FrameFixture.init();
    var slot = s.FrameSlot.init(std.testing.allocator, &budget);
    defer slot.deinit();
    try slot.replace(fixture.input(token), frameLimits());
    const old = slot.view().?;
    var helper = try s.OwnedFocus.cloneFromFrame(std.testing.allocator, old, frameLimits(), &budget);
    defer helper.deinit();
    const helper_old = helper.view();
    const before = budget.ledger().total;
    for (0..34) |case| {
        var bad = FrameFixture.init();
        var input = bad.input(token);
        const expected: s.FrameError = switch (case) {
            0 => blk: {
                input.nodes = &.{};
                break :blk error.InvalidTopology;
            },
            1 => blk: {
                bad.nodes[0].role = .group;
                break :blk error.RootIsNotApplication;
            },
            2 => blk: {
                bad.nodes[0].parent = 1;
                break :blk error.InvalidTopology;
            },
            3 => blk: {
                bad.nodes[0].child_start = 4;
                break :blk error.InvalidTopology;
            },
            4 => blk: {
                bad.nodes[0].child_start = std.math.maxInt(usize);
                break :blk error.LimitExceeded;
            },
            5 => blk: {
                bad.nodes[1].child_start = 1;
                break :blk error.InvalidTopology;
            },
            6 => blk: {
                input.child_indices = &.{ 1, 3 };
                break :blk error.InvalidTopology;
            },
            7 => blk: {
                bad.children[1] = 1;
                break :blk error.InvalidTopology;
            },
            8 => blk: {
                bad.children[0] = 9;
                break :blk error.InvalidTopology;
            },
            9 => blk: {
                bad.nodes[2].parent = 3;
                break :blk error.InvalidTopology;
            },
            10 => blk: {
                bad.children = .{ 3, 1, 2 };
                break :blk error.InvalidTopology;
            },
            11 => blk: {
                bad.nodes[2].id = bad.nodes[3].id;
                break :blk error.DuplicateNodeId;
            },
            12 => blk: {
                bad.nodes[3].action = id("A");
                break :blk error.DuplicateActionId;
            },
            13 => blk: {
                bad.nodes[2].focusable = false;
                break :blk error.ActionIsNotFocusable;
            },
            14 => blk: {
                bad.nodes[2].id.bytes = "\xff";
                break :blk error.InvalidUtf8;
            },
            15 => blk: {
                bad.nodes[2].name = "\xff";
                break :blk error.InvalidUtf8;
            },
            16 => blk: {
                bad.nodes[2].description = "\xff";
                break :blk error.InvalidUtf8;
            },
            17 => blk: {
                bad.nodes[2].value = "\xff";
                break :blk error.InvalidUtf8;
            },
            18 => blk: {
                bad.nodes[2].action = id("\xff");
                break :blk error.InvalidUtf8;
            },
            19 => blk: {
                input.owner.context = "\xff";
                break :blk error.InvalidUtf8;
            },
            20 => blk: {
                bad.announcements[0].code = "\xff";
                break :blk error.InvalidUtf8;
            },
            21 => blk: {
                bad.announcements[0].message = "\xff";
                break :blk error.InvalidUtf8;
            },
            22 => blk: {
                bad.nodes[2].bounds.?.min[0] = std.math.nan(f32);
                break :blk error.InvalidBounds;
            },
            23 => blk: {
                bad.nodes[2].bounds.?.max[1] = std.math.inf(f32);
                break :blk error.InvalidBounds;
            },
            24 => blk: {
                bad.nodes[2].bounds.?.min[1] = 11;
                break :blk error.InvalidBounds;
            },
            25 => blk: {
                input.focus.modal = &.{};
                break :blk error.EmptyModal;
            },
            26 => blk: {
                bad.nodes[0].child_count = 1;
                bad.nodes[1].parent = 2;
                bad.nodes[1].child_start = 1;
                bad.nodes[2].child_start = 2;
                bad.nodes[2].child_count = 1;
                bad.children = .{ 3, 2, 1 };
                break :blk error.InvalidTopology;
            },
            27 => blk: {
                bad.nodes[2].parent = null;
                break :blk error.InvalidTopology;
            },
            28 => blk: {
                input.focus.focused = id("missing");
                break :blk error.InvalidFocus;
            },
            29 => blk: {
                input.focus.modal = null;
                break :blk error.InvalidFocus;
            },
            30 => blk: {
                input.focus.base = &.{.{ .bytes = "\xff" }};
                break :blk error.InvalidUtf8;
            },
            31 => blk: {
                input.focus.modal = &.{.{ .bytes = "\xff" }};
                break :blk error.InvalidUtf8;
            },
            32 => blk: {
                input.focus.focused = id("\xff");
                break :blk error.InvalidUtf8;
            },
            33 => blk: {
                input.focus.restore = id("\xff");
                break :blk error.InvalidUtf8;
            },
            else => unreachable,
        };
        try std.testing.expectError(expected, slot.replace(input, frameLimits()));
        try expectFrameIdentity(old, slot.view().?);
        try expectFrame(fixture.input(token), old);
        try std.testing.expectEqual(before, budget.ledger().total);
        try std.testing.expect(helper_old.base.ptr == helper.view().base.ptr and helper.view().focused == null);
        try std.testing.expectEqual(@as(usize, 0), budget.ledger().staged_frame + budget.ledger().frame_scratch + budget.pending_bytes + budget.active_reservations);
    }
    for (0..8) |case| {
        var limits = frameLimits();
        switch (case) {
            0 => limits.max_nodes = 3,
            1 => limits.max_id_bytes = 10,
            2 => limits.max_text_bytes = 43,
            3 => limits.max_focus_actions = 3,
            4 => limits.max_announcements = 1,
            5 => limits.max_child_indices = 2,
            6 => limits.max_owner_context_bytes = 4,
            7 => limits.max_owned_bytes = 1075,
            else => unreachable,
        }
        try std.testing.expectError(error.LimitExceeded, slot.replace(fixture.input(token), limits));
        try expectFrameIdentity(old, slot.view().?);
        try expectFrame(fixture.input(token), old);
        try std.testing.expectEqual(before, budget.ledger().total);
        try std.testing.expect(helper_old.base.ptr == helper.view().base.ptr and helper_old.modal.?.ptr == helper.view().modal.?.ptr);
    }
}
fn borrowedCopy(buffers: *std.ArrayList([]u8), bytes: []const u8) ![]const u8 {
    const copy = try std.testing.allocator.dupe(u8, bytes);
    errdefer std.testing.allocator.free(copy);
    try buffers.append(std.testing.allocator, copy);
    return copy;
}
test "frame poison free every caller buffer and move frame slot helper capabilities" {
    var tokens = s.LiveTokenBudget.init(.{ .max_live_objects = 1, .max_live_requested_bytes = 4096 });
    defer tokens.deinit();
    const token = try s.TokenRef.create(std.testing.allocator, &tokens);
    defer token.release();
    var budget = s.PreparationBudget.init(65536);
    defer budget.deinit();
    var tracking = FocusTracking{ .budget = &budget };
    var fixture = FrameFixture.init();
    var buffers: std.ArrayList([]u8) = .empty;
    defer buffers.deinit(std.testing.allocator);
    const nodes = try std.testing.allocator.dupe(s.NodeSpec, &fixture.nodes);
    const children = try std.testing.allocator.dupe(usize, &fixture.children);
    const announcements = try std.testing.allocator.dupe(s.AnnouncementSpec, &fixture.announcements);
    var input = fixture.input(token);
    input.nodes = nodes;
    input.child_indices = children;
    input.announcements = announcements;
    for (nodes) |*n| {
        n.id.bytes = try borrowedCopy(&buffers, n.id.bytes);
        n.name = try borrowedCopy(&buffers, n.name);
        n.description = try borrowedCopy(&buffers, n.description);
        if (n.value) |v| n.value = try borrowedCopy(&buffers, v);
        if (n.action) |v| n.action = id(try borrowedCopy(&buffers, v.bytes));
    }
    for (announcements) |*a| {
        a.code = try borrowedCopy(&buffers, a.code);
        a.message = try borrowedCopy(&buffers, a.message);
    }
    input.owner.context = try borrowedCopy(&buffers, input.owner.context);
    const base = try std.testing.allocator.dupe(s.ActionId, input.focus.base);
    const modal = try std.testing.allocator.dupe(s.ActionId, input.focus.modal.?);
    for (base) |*a| a.bytes = try borrowedCopy(&buffers, a.bytes);
    for (modal) |*a| a.bytes = try borrowedCopy(&buffers, a.bytes);
    input.focus.base = base;
    input.focus.modal = modal;
    input.focus.restore = id(try borrowedCopy(&buffers, "Z"));
    var original = try s.OwnedFrame.init(tracking.allocator(), &budget, input, frameLimits());
    for (buffers.items) |bytes| {
        @memset(bytes, 0xff);
        std.testing.allocator.free(bytes);
    }
    @memset(std.mem.sliceAsBytes(nodes), 0xff);
    std.testing.allocator.free(nodes);
    @memset(children, 999);
    std.testing.allocator.free(children);
    @memset(std.mem.sliceAsBytes(announcements), 0xff);
    std.testing.allocator.free(announcements);
    @memset(std.mem.sliceAsBytes(base), 0xff);
    std.testing.allocator.free(base);
    @memset(std.mem.sliceAsBytes(modal), 0xff);
    std.testing.allocator.free(modal);
    const holder = try std.testing.allocator.create(s.OwnedFrame);
    holder.* = original;
    original = undefined;
    try expectFrame(fixture.input(token), holder.view());
    var helper = try s.OwnedFocus.cloneFromFrame(tracking.allocator(), holder.view(), frameLimits(), &budget);
    const helper_holder = try std.testing.allocator.create(s.OwnedFocus);
    helper_holder.* = helper;
    helper = undefined;
    var slot = s.FrameSlot.init(tracking.allocator(), &budget);
    try slot.replace(fixture.input(token), frameLimits());
    const slot_holder = try std.testing.allocator.create(s.FrameSlot);
    slot_holder.* = slot;
    slot = undefined;
    try expectFrame(fixture.input(token), slot_holder.view().?);
    try std.testing.expect(helper_holder.view().focused == null);
    helper_holder.deinit();
    std.testing.allocator.destroy(helper_holder);
    holder.deinit();
    std.testing.allocator.destroy(holder);
    slot_holder.deinit();
    std.testing.allocator.destroy(slot_holder);
    try tracking.check();
    try std.testing.expectEqual(@as(usize, 0), tracking.live);
}
test "frame all eighteen roles deep iterative tree zero announcements and null focus clone" {
    var tokens = s.LiveTokenBudget.init(.{ .max_live_objects = 1, .max_live_requested_bytes = 4096 });
    defer tokens.deinit();
    const token = try s.TokenRef.create(std.testing.allocator, &tokens);
    defer token.release();
    var budget = s.PreparationBudget.init(262144);
    defer budget.deinit();
    var nodes: [256]s.NodeSpec = undefined;
    var names: [256][2]u8 = undefined;
    var children: [255]usize = undefined;
    const roles = std.meta.tags(s.NodeRole);
    try std.testing.expectEqual(@as(usize, 18), roles.len);
    for (&nodes, &names, 0..) |*n, *name, i| {
        name.* = .{ @intCast(i / 16 + 'A'), @intCast(i % 16 + 'A') };
        n.* = frameNode(name, if (i == 0) .application else roles[i % 18], if (i == 0) null else i - 1, if (i == 255) 255 else i, if (i == 255) 0 else 1);
        if (i != 255) children[i] = i + 1;
    }
    var limits = frameLimits();
    limits.max_nodes = 256;
    limits.max_child_indices = 255;
    limits.max_announcements = 0;
    limits.max_owned_bytes = 262144;
    var frame = try s.OwnedFrame.init(std.testing.allocator, &budget, .{ .nodes = &nodes, .child_indices = &children, .announcements = &.{}, .owner = .{ .resident = token, .hold = null, .context = "", .draft = null }, .focus = .{ .base = &.{ id("A"), id("B"), id("C") }, .modal = null, .focused = null, .restore = null } }, limits);
    defer frame.deinit();
    for (frame.view().nodes, 0..) |n, i| try std.testing.expectEqual(if (i == 0) s.NodeRole.application else roles[i % 18], n.role);
    var helper = try s.OwnedFocus.cloneFromFrame(std.testing.allocator, frame.view(), limits, &budget);
    defer helper.deinit();
    try std.testing.expect(helper.view().focused == null);
    try std.testing.expectEqualStrings("B", helper.next().?.bytes);
}

test "frame empty identity byte storage every actual OOM and exact zero capacities" {
    var tokens = s.LiveTokenBudget.init(.{ .max_live_objects = 1, .max_live_requested_bytes = 4096 });
    defer tokens.deinit();
    const token = try s.TokenRef.create(std.testing.allocator, &tokens);
    defer token.release();
    const root = frameNode("", .application, null, 0, 0);
    const base = [_]s.ActionId{.{ .bytes = "" }};
    const input = s.FrameInput{ .nodes = &.{root}, .child_indices = &.{}, .announcements = &.{}, .owner = .{ .resident = token, .hold = null, .context = "", .draft = null }, .focus = .{ .base = &base, .modal = null, .focused = null, .restore = null } };
    const limits = s.Limits{ .max_nodes = 1, .max_id_bytes = 0, .max_text_bytes = 0, .max_focus_actions = 1, .max_announcements = 0, .max_child_indices = 0, .max_owner_context_bytes = 0, .max_owned_bytes = 216 };
    var budget = s.PreparationBudget.init(216);
    var tracking = FocusTracking{ .budget = &budget };
    var frame = try s.OwnedFrame.init(tracking.allocator(), &budget, input, limits);
    try std.testing.expectEqualSlices(usize, &.{ 8, 160, 16, 16, 16 }, tracking.lengths[0..tracking.calls]);
    try std.testing.expectEqual(@as(usize, 192), budget.ledger().total);
    frame.deinit();
    try tracking.check();
    budget.deinit();
    for (0..5) |ordinal| {
        var b = s.PreparationBudget.init(216);
        var tr = FocusTracking{ .budget = &b };
        var fail = std.testing.FailingAllocator.init(tr.allocator(), .{ .fail_index = ordinal });
        try std.testing.expectError(error.OutOfMemory, s.OwnedFrame.init(fail.allocator(), &b, input, limits));
        try std.testing.expectEqual(@as(usize, 0), b.ledger().total + b.pending_bytes + b.active_reservations);
        try tr.check();
        b.deinit();
    }
}
