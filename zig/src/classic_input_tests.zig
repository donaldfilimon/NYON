const std = @import("std");
const input = @import("classic_input.zig");
test "foundation monotonic revisions preserve failure and close terminally" {
    var clock = input.RevisionClock{};
    try std.testing.expectEqual(@as(u64, 1), try clock.next());
    try clock.commit(0, 1);
    try std.testing.expectEqual(@as(u64, 2), try clock.next());
    try std.testing.expectError(error.RevisionConflict, clock.commit(0, 1));
    try std.testing.expectEqual(@as(u64, 1), clock.value);
    clock.deinit();
    try std.testing.expectError(error.ClockClosed, clock.next());
}
const H1 = @import("retained_semantics.zig");
const a = std.testing.allocator;
const TokenMirror = struct { allocator: std.mem.Allocator, refs: usize, budget: *H1.LiveTokenBudget, charged_bytes: usize };
const Trace = struct {
    lengths: [128]usize = undefined,
    pointers: [128][*]u8 = undefined,
    alignments: [128]std.mem.Alignment = undefined,
    calls: usize = 0,
    frees: usize = 0,
    live: usize = 0,
    peak: usize = 0,
    budget: ?*H1.LiveTokenBudget = null,
    registry: ?*input.ReferenceRegistry = null,
    pending_observed: usize = 0,
    fn alloc(ctx: *anyopaque, len: usize, alignment: std.mem.Alignment, ra: usize) ?[*]u8 {
        const self: *Trace = @ptrCast(@alignCast(ctx));
        if (self.budget) |budget| {
            if (budget.pending_objects != 1 or budget.pending_bytes != len) @panic("fixture pending token admission");
            if (self.registry) |registry| {
                if (used(registry) != budget.usage().live_objects + 1) @panic("fixture pending external slot admission");
            }
            self.pending_observed += 1;
        }
        const ptr = a.rawAlloc(len, alignment, ra) orelse return null;
        if (self.calls >= self.lengths.len) @panic("fixture trace capacity");
        self.lengths[self.calls] = len;
        self.pointers[self.calls] = ptr;
        self.alignments[self.calls] = alignment;
        self.calls += 1;
        self.live += len;
        self.peak = @max(self.peak, self.live);
        return ptr;
    }
    fn free(ctx: *anyopaque, memory: []u8, alignment: std.mem.Alignment, ra: usize) void {
        const self: *Trace = @ptrCast(@alignCast(ctx));
        self.live -= memory.len;
        self.frees += 1;
        a.rawFree(memory, alignment, ra);
    }
    fn allocator(self: *Trace) std.mem.Allocator {
        return .{ .ptr = self, .vtable = &.{ .alloc = alloc, .resize = std.mem.Allocator.noResize, .remap = std.mem.Allocator.noRemap, .free = free } };
    }
    fn token(self: *Trace, index: usize, budget: *H1.LiveTokenBudget) !*TokenMirror {
        try std.testing.expectEqual(@sizeOf(TokenMirror), self.lengths[index]);
        try std.testing.expect(self.alignments[index].toByteUnits() >= @alignOf(TokenMirror));
        const result: *TokenMirror = @ptrCast(@alignCast(self.pointers[index]));
        const handle = self.allocator();
        try std.testing.expect(result.allocator.ptr == handle.ptr and result.allocator.vtable == handle.vtable);
        try std.testing.expect(result.budget == budget);
        try std.testing.expectEqual(self.lengths[index], result.charged_bytes);
        try std.testing.expectEqual(@as(usize, 1), result.refs);
        return result;
    }
};
fn used(registry: *const input.ReferenceRegistry) usize {
    var count: usize = 0;
    for (registry.entries) |entry| if (entry.state != .free) {
        count += 1;
    };
    return count;
}
const Root = struct {
    ticket: input.SlotTicket,
    token: H1.TokenRef,
    fn create(registry: *input.ReferenceRegistry, budget: *H1.LiveTokenBudget, alloc: std.mem.Allocator) !Root {
        const ticket = try registry.reserve(.root_resident);
        errdefer registry.cancel(ticket) catch unreachable;
        const token = try H1.TokenRef.create(alloc, budget);
        errdefer token.release();
        try registry.markOwned(ticket);
        return .{ .ticket = ticket, .token = token };
    }
    fn release(self: Root, registry: *input.ReferenceRegistry) !void {
        self.token.release();
        try registry.release(self.ticket);
    }
    fn current(self: Root) input.CurrentView {
        return .{ .revision = 7, .owner = .{ .resident = self.token, .hold = null, .context = "classic", .draft = null } };
    }
};
fn source(current: input.CurrentView) input.EventSource {
    return .{ .current = current, .kind = .activate, .action = "launch" };
}
test "foundation registry 32 slots exact transitions stale copies cross instance and terminal close" {
    var registry = input.ReferenceRegistry.init();
    var other = input.ReferenceRegistry.init();
    var tickets: [32]input.SlotTicket = undefined;
    for (&tickets, 0..) |*ticket, index| {
        ticket.* = try registry.reserve(std.meta.tags(input.SlotPurpose)[index % 8]);
        try std.testing.expectEqual(@as(u8, @intCast(index)), ticket.index);
        if (index % 2 == 0) try registry.markOwned(ticket.*);
    }
    try std.testing.expectError(error.TokenReferenceLimit, registry.reserve(.transition));
    try std.testing.expectEqual(@as(usize, 32), used(&registry));
    for (tickets, 0..) |ticket, index| {
        if (index % 2 == 0) {
            try std.testing.expectError(error.InvalidTicket, registry.cancel(ticket));
            try std.testing.expectError(error.InvalidTicket, registry.markOwned(ticket));
            try registry.release(ticket);
        } else {
            try std.testing.expectError(error.InvalidTicket, registry.release(ticket));
            try registry.cancel(ticket);
        }
    }
    const fresh = try registry.reserve(.transition);
    try registry.markOwned(fresh);
    const other_ticket = try other.reserve(.transition);
    try other.markOwned(other_ticket);
    for (tickets) |old| {
        try std.testing.expectError(error.InvalidTicket, registry.markOwned(old));
        try std.testing.expectError(error.InvalidTicket, registry.cancel(old));
        try std.testing.expectError(error.InvalidTicket, registry.release(old));
    }
    try std.testing.expectError(error.InvalidTicket, other.release(fresh));
    try std.testing.expectError(error.InvalidTicket, registry.release(other_ticket));
    var invalid = fresh;
    invalid.index = 255;
    try std.testing.expectError(error.InvalidTicket, registry.release(invalid));
    try std.testing.expectEqual(@as(usize, 1), used(&registry));
    try registry.release(fresh);
    try other.release(other_ticket);
    registry.deinit();
    other.deinit();
    try std.testing.expectError(error.RegistryClosed, registry.reserve(.transition));
    try std.testing.expectError(error.InvalidTicket, registry.release(fresh));
    try std.testing.expectError(error.InvalidTicket, registry.cancel(fresh));
    try std.testing.expectError(error.InvalidTicket, registry.markOwned(fresh));
}
test "foundation clock exact successor overflow same token revision and terminal preservation" {
    var clock = input.RevisionClock{};
    for (0..5) |i| {
        try clock.commit(@intCast(i), try clock.next());
        try std.testing.expectError(error.RevisionConflict, clock.commit(@intCast(i + 1), @intCast(i + 3)));
    }
    clock.value = std.math.maxInt(u64);
    try std.testing.expectError(error.RevisionExhausted, clock.next());
    try std.testing.expectError(error.RevisionConflict, clock.commit(clock.value, 0));
    clock.deinit();
    try std.testing.expectError(error.ClockClosed, clock.commit(clock.value, 0));
    try std.testing.expectEqual(std.math.maxInt(u64), clock.value);
}
test "foundation actual token32 byte1280 and one less creation cleanup current preservation" {
    var registry = input.ReferenceRegistry.init();
    var budget = H1.LiveTokenBudget.init(.{ .max_live_objects = 32, .max_live_requested_bytes = 1280 });
    var trace = Trace{};
    trace.budget = &budget;
    trace.registry = &registry;
    var roots: [32]Root = undefined;
    for (&roots, 0..) |*root, i| {
        root.* = try Root.create(&registry, &budget, trace.allocator());
        _ = try trace.token(i, &budget);
    }
    try std.testing.expectEqual(@as(usize, 1280), budget.usage().live_requested_bytes);
    try std.testing.expectEqual(@as(usize, 32), trace.calls);
    try std.testing.expectEqual(@as(usize, 32), trace.pending_observed);
    try std.testing.expectEqual(@as(usize, 1280), trace.peak);
    try std.testing.expectError(error.TokenReferenceLimit, Root.create(&registry, &budget, trace.allocator()));
    try std.testing.expectError(error.LimitExceeded, H1.TokenRef.create(trace.allocator(), &budget));
    try std.testing.expectEqual(@as(usize, 32), trace.calls);
    try std.testing.expect(roots[0].current().owner.resident.sameIdentity(roots[0].token));
    for (roots) |root| try root.release(&registry);
    budget.deinit();
    registry.deinit();
    try std.testing.expectEqual(@as(usize, 0), trace.live);
    var r = input.ReferenceRegistry.init();
    var b = H1.LiveTokenBudget.init(.{ .max_live_objects = 32, .max_live_requested_bytes = 1279 });
    for (roots[0..31]) |*root| root.* = try Root.create(&r, &b, a);
    try std.testing.expectError(error.LimitExceeded, Root.create(&r, &b, a));
    try std.testing.expectEqual(@as(usize, 31), used(&r));
    for (roots[0..31]) |root| try root.release(&r);
    b.deinit();
    r.deinit();
    std.debug.print("foundation token trace count=32 request={d} peak=1280 frees={d}\n", .{ trace.lengths[0], trace.frees });
}
test "foundation actual constructor OOM pending creation and allocation free enqueue" {
    var registry = input.ReferenceRegistry.init();
    var budget = H1.LiveTokenBudget.init(.{ .max_live_objects = 32, .max_live_requested_bytes = 1280 });
    var fail = std.testing.FailingAllocator.init(a, .{ .fail_index = 0 });
    try std.testing.expectError(error.OutOfMemory, Root.create(&registry, &budget, fail.allocator()));
    try std.testing.expectEqual(@as(usize, 0), used(&registry));
    try std.testing.expectEqual(@as(usize, 0), budget.pending_bytes + budget.pending_objects + budget.usage().live_objects);
    try std.testing.expectError(error.OutOfMemory, input.SemanticQueue.init(fail.allocator(), &registry));
    try std.testing.expectEqual(@as(usize, 0), used(&registry));
    var trace = Trace{};
    const root = try Root.create(&registry, &budget, a);
    var queue = try input.SemanticQueue.init(trace.allocator(), &registry);
    try queue.enqueue(source(root.current()));
    queue.finishFront();
    for (0..64) |_| {
        try queue.enqueue(source(root.current()));
        queue.finishFront();
    }
    try std.testing.expectEqual(@as(usize, 1), trace.calls);
    std.debug.print("foundation queue allocation calls=1 request={d} live={d} peak={d}\n", .{ trace.lengths[0], trace.live, trace.peak });
    queue.deinit();
    try std.testing.expectEqual(@as(usize, 0), trace.live);
    try root.release(&registry);
    budget.deinit();
    registry.deinit();
}
test "foundation eight distinct stale pairs ninth failure FIFO retained identity then final release" {
    var registry = input.ReferenceRegistry.init();
    var budget = H1.LiveTokenBudget.init(.{ .max_live_objects = 32, .max_live_requested_bytes = 1280 });
    var roots: [16]Root = undefined;
    for (&roots) |*root| root.* = try Root.create(&registry, &budget, a);
    var queue = try input.SemanticQueue.init(a, &registry);
    for (0..8) |i| {
        var current = roots[2 * i].current();
        current.revision = @intCast(i);
        current.owner.draft = roots[2 * i + 1].token;
        try queue.enqueue(source(current));
    }
    try std.testing.expectEqual(@as(usize, 32), used(&registry));
    const before = queue.front().?;
    try std.testing.expectError(error.QueueFull, queue.enqueue(source(roots[0].current())));
    try std.testing.expect(input.matchesCurrent(queue.front().?, before.current));
    for (roots) |root| try root.release(&registry);
    try std.testing.expectEqual(@as(usize, 16), budget.usage().live_objects);
    for (0..8) |i| {
        {
            const event = queue.front().?;
            try std.testing.expectEqual(@as(u64, @intCast(i)), event.current.revision);
            try std.testing.expect(event.current.owner.resident.sameIdentity(roots[2 * i].token));
            try std.testing.expect(event.current.owner.draft.?.sameIdentity(roots[2 * i + 1].token));
        }
        queue.finishFront();
        try std.testing.expectEqual(@as(usize, 16 - 2 * (i + 1)), budget.usage().live_objects);
    }
    try std.testing.expect(queue.front() == null);
    queue.deinit();
    budget.deinit();
    registry.deinit();
}
test "foundation exact full UTF8 payload copied poisoning boundary and malformed before retain" {
    var registry = input.ReferenceRegistry.init();
    var budget = H1.LiveTokenBudget.init(.{ .max_live_objects = 32, .max_live_requested_bytes = 1280 });
    const root = try Root.create(&registry, &budget, a);
    var queue = try input.SemanticQueue.init(a, &registry);
    var context: [129]u8 = @splat('c');
    var action: [65]u8 = @splat('a');
    var text: [3649]u8 = undefined;
    for (text[0..3648], 0..) |*byte, i| byte.* = if (i % 2 == 0) 0xc3 else 0xa9;
    text[3648] = 'x';
    var current = root.current();
    current.owner.context = context[0..128];
    var event = input.EventSource{ .current = current, .kind = .edit_utf8, .action = action[0..64], .text = text[0..3648] };
    try queue.enqueue(event);
    const count = used(&registry);
    event.text = &text;
    try std.testing.expectError(error.EventLimit, queue.enqueue(event));
    try std.testing.expectEqual(@as(usize, 3649), event.text.len);
    event.text = "";
    event.current.owner.context = &context;
    try std.testing.expectError(error.EventLimit, queue.enqueue(event));
    event.current.owner.context = current.owner.context;
    event.action = &action;
    try std.testing.expectError(error.EventLimit, queue.enqueue(event));
    event.action = "";
    try std.testing.expectError(error.InvalidPayload, queue.enqueue(event));
    event.action = "a";
    event.kind = .activate;
    event.text = "text";
    try std.testing.expectError(error.InvalidPayload, queue.enqueue(event));
    for (0..3) |case| {
        event = source(current);
        switch (case) {
            0 => event.current.owner.context = "\xff",
            1 => event.action = "\xc3",
            2 => {
                event.kind = .edit_utf8;
                event.text = "\xc3x";
            },
            else => unreachable,
        }
        try std.testing.expectError(error.InvalidUtf8, queue.enqueue(event));
    }
    try std.testing.expectEqual(count, used(&registry));
    @memset(&context, 0xff);
    @memset(&action, 0xff);
    @memset(&text, 0xff);
    {
        const borrowed = queue.front().?;
        try std.testing.expectEqual(@as(usize, 128), borrowed.current.owner.context.len);
        try std.testing.expectEqual(@as(usize, 64), borrowed.action.len);
        try std.testing.expectEqual(@as(usize, 3648), borrowed.text.len);
        try std.testing.expect(std.unicode.utf8ValidateSlice(borrowed.text));
        try std.testing.expectEqual(@as(u8, 'c'), borrowed.current.owner.context[127]);
        try std.testing.expectEqual(@as(u8, 'a'), borrowed.action[63]);
    }
    queue.clear();
    queue.deinit();
    try root.release(&registry);
    budget.deinit();
    registry.deinit();
}
test "foundation full owner hold context draft resident and source revision stale independently" {
    var registry = input.ReferenceRegistry.init();
    var budget = H1.LiveTokenBudget.init(.{ .max_live_objects = 32, .max_live_requested_bytes = 1280 });
    const root = try Root.create(&registry, &budget, a);
    const draft = try Root.create(&registry, &budget, a);
    const other = try Root.create(&registry, &budget, a);
    var queue = try input.SemanticQueue.init(a, &registry);
    var current = root.current();
    current.owner.draft = draft.token;
    current.owner.hold = 3;
    try queue.enqueue(source(current));
    try queue.enqueue(source(current));
    {
        const event = queue.front().?;
        try std.testing.expect(input.matchesCurrent(event, current));
        for (0..6) |case| {
            var stale = current;
            switch (case) {
                0 => stale.revision += 1,
                1 => stale.owner.context = "Classic",
                2 => stale.owner.hold = null,
                3 => stale.owner.resident = other.token,
                4 => stale.owner.draft = null,
                5 => stale.owner.draft = other.token,
                else => unreachable,
            }
            try std.testing.expect(!input.matchesCurrent(event, stale));
        }
    }
    queue.finishFront();
    current.revision += 1;
    try std.testing.expect(!input.matchesCurrent(queue.front().?, current));
    queue.clear();
    queue.deinit();
    try root.release(&registry);
    try draft.release(&registry);
    try other.release(&registry);
    budget.deinit();
    registry.deinit();
}
test "foundation observed token overflow each unwind registry full second ticket preserve old event" {
    var registry = input.ReferenceRegistry.init();
    var budget = H1.LiveTokenBudget.init(.{ .max_live_objects = 32, .max_live_requested_bytes = 1280 });
    var trace = Trace{};
    const root = try Root.create(&registry, &budget, trace.allocator());
    const draft = try Root.create(&registry, &budget, trace.allocator());
    const rm = try trace.token(0, &budget);
    const dm = try trace.token(1, &budget);
    var queue = try input.SemanticQueue.init(a, &registry);
    try queue.enqueue(source(root.current()));
    const original_r = rm.refs;
    const original_d = dm.refs;
    const old = queue.front().?.current;
    for (0..3) |case| {
        var current = root.current();
        current.owner.draft = if (case == 2) root.token else draft.token;
        rm.refs = if (case == 0) std.math.maxInt(usize) else if (case == 2) std.math.maxInt(usize) - 1 else original_r;
        dm.refs = if (case == 1) std.math.maxInt(usize) else original_d;
        const r = rm.refs;
        const d = dm.refs;
        try std.testing.expectError(error.ReferenceOverflow, queue.enqueue(source(current)));
        try std.testing.expectEqual(r, rm.refs);
        try std.testing.expectEqual(d, dm.refs);
        try std.testing.expectEqual(@as(usize, 3), used(&registry));
        try std.testing.expect(input.matchesCurrent(queue.front().?, old));
        rm.refs = original_r;
        dm.refs = original_d;
    }
    var reserved: [28]input.SlotTicket = undefined;
    for (&reserved) |*ticket| ticket.* = try registry.reserve(.transition);
    var current = root.current();
    current.owner.draft = draft.token;
    try std.testing.expectError(error.TokenReferenceLimit, queue.enqueue(source(current)));
    try std.testing.expectEqual(@as(usize, 31), used(&registry));
    try std.testing.expectEqual(original_r, rm.refs);
    try std.testing.expectEqual(original_d, dm.refs);
    for (reserved) |ticket| try registry.cancel(ticket);
    queue.deinit();
    try root.release(&registry);
    try draft.release(&registry);
    budget.deinit();
    registry.deinit();
}
test "foundation empty closed registry queue admission deterministic and independent fresh domain" {
    var registry = input.ReferenceRegistry.init();
    var budget = H1.LiveTokenBudget.init(.{ .max_live_objects = 32, .max_live_requested_bytes = 1280 });
    // Synthetic token belongs to a separate still-live owner domain; this closed
    // registry never owned it. No ticket crosses shutdown or registry address reuse.
    const token = try H1.TokenRef.create(a, &budget);
    registry.deinit();
    var queue = try input.SemanticQueue.init(a, &registry);
    const current = input.CurrentView{ .revision = 1, .owner = .{ .resident = token, .draft = null, .hold = null, .context = "" } };
    try std.testing.expectError(error.RegistryClosed, queue.enqueue(source(current)));
    try std.testing.expect(queue.front() == null);
    queue.deinit();
    token.release();
    budget.deinit();
    var fresh_registry = input.ReferenceRegistry.init();
    var fresh_clock = input.RevisionClock{};
    const ticket = try fresh_registry.reserve(.transition);
    try fresh_registry.cancel(ticket);
    try fresh_clock.commit(0, try fresh_clock.next());
    fresh_clock.deinit();
    fresh_registry.deinit();
}
test "foundation multibyte context action empty edit and failure preserve every occupied record" {
    var registry = input.ReferenceRegistry.init();
    var budget = H1.LiveTokenBudget.init(.{ .max_live_objects = 32, .max_live_requested_bytes = 1280 });
    var trace = Trace{};
    const root = try Root.create(&registry, &budget, trace.allocator());
    const mirror = try trace.token(0, &budget);
    var queue = try input.SemanticQueue.init(a, &registry);
    var context: [128]u8 = undefined;
    var action: [64]u8 = undefined;
    for (&context, 0..) |*byte, i| byte.* = if (i % 2 == 0) 0xc3 else 0xa9;
    for (&action, 0..) |*byte, i| byte.* = if (i % 2 == 0) 0xc3 else 0xa9;
    var current = root.current();
    current.owner.context = &context;
    try queue.enqueue(.{ .current = current, .kind = .focus, .action = &action });
    try queue.enqueue(.{ .current = current, .kind = .edit_utf8, .action = &action });
    const header = queue.header.?;
    const snapshot = try a.dupe(u8, std.mem.sliceAsBytes(header.slots[0..header.count]));
    defer a.free(snapshot);
    const refs = mirror.refs;
    try std.testing.expectError(error.InvalidUtf8, queue.enqueue(.{ .current = current, .kind = .edit_utf8, .action = &action, .text = "\xff" }));
    try std.testing.expectEqualSlices(u8, snapshot, std.mem.sliceAsBytes(header.slots[0..header.count]));
    try std.testing.expectEqual(@as(u8, 2), header.count);
    try std.testing.expectEqual(refs, mirror.refs);
    {
        const view = queue.front().?;
        try std.testing.expectEqualSlices(u8, &context, view.current.owner.context);
        try std.testing.expectEqualSlices(u8, &action, view.action);
    }
    queue.finishFront();
    try std.testing.expectEqual(input.SemanticKind.edit_utf8, queue.front().?.kind);
    try std.testing.expectEqual(@as(usize, 0), queue.front().?.text.len);
    queue.deinit();
    try root.release(&registry);
    budget.deinit();
    registry.deinit();
}
test "foundation source buffer freed queue moved and retained allocation unavailable until drain" {
    var registry = input.ReferenceRegistry.init();
    var budget = H1.LiveTokenBudget.init(.{ .max_live_objects = 32, .max_live_requested_bytes = 1280 });
    var trace = Trace{};
    const root = try Root.create(&registry, &budget, trace.allocator());
    var context = try a.dupe(u8, "settings");
    var action = try a.dupe(u8, "name");
    var text = try a.dupe(u8, "complete é buffer");
    var current = root.current();
    current.owner.context = context;
    var queue = try input.SemanticQueue.init(a, &registry);
    try queue.enqueue(.{ .current = current, .kind = .edit_utf8, .action = action, .text = text });
    @memset(context, 0xff);
    a.free(context);
    context = undefined;
    @memset(action, 0xff);
    a.free(action);
    action = undefined;
    @memset(text, 0xff);
    a.free(text);
    text = undefined;
    current = undefined;
    const holder = try a.create(input.SemanticQueue);
    holder.* = queue;
    queue = undefined;
    try root.release(&registry);
    try std.testing.expectEqual(@as(usize, 0), trace.frees);
    try std.testing.expectEqual(@as(usize, 1), budget.usage().live_objects);
    const next = try Root.create(&registry, &budget, trace.allocator());
    try std.testing.expect(!next.token.sameIdentity(holder.front().?.current.owner.resident));
    {
        const view = holder.front().?;
        try std.testing.expectEqualStrings("settings", view.current.owner.context);
        try std.testing.expectEqualStrings("name", view.action);
        try std.testing.expectEqualStrings("complete é buffer", view.text);
    }
    holder.clear();
    try std.testing.expectEqual(@as(usize, 1), trace.frees);
    holder.deinit();
    a.destroy(holder);
    try next.release(&registry);
    budget.deinit();
    registry.deinit();
}
test "foundation canceled slot reused generation rejects old ticket in reserved and owned states" {
    var registry = input.ReferenceRegistry.init();
    const canceled = try registry.reserve(.transition);
    const old = canceled;
    try registry.cancel(canceled);
    const fresh = try registry.reserve(.guide_hold);
    try std.testing.expectEqual(old.index, fresh.index);
    try std.testing.expect(fresh.generation > old.generation);
    try std.testing.expectEqual(@as(usize, 1), used(&registry));
    for (0..2) |stage| {
        if (stage == 1) try registry.markOwned(fresh);
        const before = registry.entries[fresh.index];
        try std.testing.expectError(error.InvalidTicket, registry.cancel(old));
        try std.testing.expectEqualDeep(before, registry.entries[fresh.index]);
        try std.testing.expectError(error.InvalidTicket, registry.markOwned(old));
        try std.testing.expectEqualDeep(before, registry.entries[fresh.index]);
        try std.testing.expectError(error.InvalidTicket, registry.release(old));
        try std.testing.expectEqualDeep(before, registry.entries[fresh.index]);
        try std.testing.expectEqual(@as(usize, 1), used(&registry));
    }
    try registry.release(fresh);
    try std.testing.expectEqual(@as(usize, 0), used(&registry));
    registry.deinit();
}

test "key edges whole ASCII command mapping RED jury" {
    try std.testing.expectEqual(@as(?input.Action, .decrease), input.actionForKey(.{ .character = "Q" }));
}

test "key edges literal all command rows uppercase and whole Unicode nonmatches" {
    const Row = struct { text: []const u8, action: input.Action };
    const rows = [_]Row{ .{ .text = "1", .action = .field1 }, .{ .text = "2", .action = .field2 }, .{ .text = "3", .action = .field3 }, .{ .text = "q", .action = .decrease }, .{ .text = "e", .action = .increase }, .{ .text = "p", .action = .pause }, .{ .text = "[", .action = .speed_down }, .{ .text = "]", .action = .speed_up }, .{ .text = "c", .action = .camera_reset }, .{ .text = "s", .action = .settings }, .{ .text = "r", .action = .restart }, .{ .text = "h", .action = .help } };
    for (rows) |row| {
        try std.testing.expectEqual(@as(?input.Action, row.action), input.actionForKey(.{ .character = row.text }));
        var upper = [_]u8{std.ascii.toUpper(row.text[0])};
        try std.testing.expectEqual(@as(?input.Action, row.action), input.actionForKey(.{ .character = &upper }));
        try std.testing.expectEqual(std.ascii.toUpper(row.text[0]), upper[0]);
    }
    try std.testing.expectEqual(@as(?input.Action, .launch), input.actionForKey(.{ .named = .space }));
    try std.testing.expectEqual(@as(?input.Action, .cancel), input.actionForKey(.{ .named = .escape }));
    try std.testing.expectEqual(@as(?input.Action, .scenario), input.actionForKey(.{ .named = .f4 }));
    for ([_][]const u8{ "", " ", "qq", "q\xcc\x81", "\xc3\xa9", "\xef\xbc\xb1", "ab", "\xe2\x84\xaa" }) |text| try std.testing.expectEqual(@as(?input.Action, null), input.actionForKey(.{ .character = text }));
    for ([_]input.NamedKey{ .f1, .enter, .tab, .arrow_up, .arrow_down, .arrow_left, .arrow_right, .page_up, .page_down, .backspace, .delete, .home, .end, .other }) |key| try std.testing.expectEqual(@as(?input.Action, null), input.actionForKey(.{ .named = key }));
    try std.testing.expectEqual(@as(?input.Action, null), input.actionForKey(.{ .unsupported = {} }));
}
test "key edges literal navigation table all modifier snapshots" {
    const Row = struct { key: input.NamedKey, nav: input.NavigationAction };
    const rows = [_]Row{ .{ .key = .tab, .nav = .tab_forward }, .{ .key = .arrow_up, .nav = .up }, .{ .key = .arrow_down, .nav = .down }, .{ .key = .arrow_left, .nav = .left }, .{ .key = .arrow_right, .nav = .right }, .{ .key = .page_up, .nav = .page_up }, .{ .key = .page_down, .nav = .page_down }, .{ .key = .enter, .nav = .activate }, .{ .key = .space, .nav = .activate }, .{ .key = .escape, .nav = .escape }, .{ .key = .backspace, .nav = .backspace }, .{ .key = .delete, .nav = .delete }, .{ .key = .home, .nav = .home }, .{ .key = .end, .nav = .end } };
    for (0..16) |bits| {
        const mods = input.Modifiers{ .shift = bits & 1 != 0, .control = bits & 2 != 0, .alt = bits & 4 != 0, .super = bits & 8 != 0 };
        for (rows) |row| try std.testing.expectEqual(@as(?input.NavigationAction, if (row.key == .tab and mods.shift) .tab_backward else row.nav), input.navigationForKey(.{ .named = row.key }, mods));
        for ([_]input.NamedKey{ .f1, .f4, .other }) |key| try std.testing.expectEqual(@as(?input.NavigationAction, null), input.navigationForKey(.{ .named = key }, mods));
        try std.testing.expectEqual(@as(?input.NavigationAction, null), input.navigationForKey(.{ .character = " " }, mods));
        try std.testing.expectEqual(@as(?input.NavigationAction, null), input.navigationForKey(.{ .unsupported = {} }, mods));
    }
}
test "key edges all fifteen independent sets release pending coalescing and resets" {
    var state = input.InputState.init();
    inline for (std.meta.tags(input.Action)) |action| {
        try std.testing.expect(!state.isHeld(action));
        try std.testing.expect(!state.takePressedOnce(action));
        state.setAction(action, true);
        try std.testing.expect(state.isHeld(action));
        state.setAction(action, false);
        try std.testing.expect(!state.isHeld(action));
        try std.testing.expect(state.takePressedOnce(action));
        try std.testing.expect(!state.takePressedOnce(action));
        state.setAction(action, true);
        try std.testing.expect(state.takePressedOnce(action));
        state.setAction(action, true);
        try std.testing.expect(!state.takePressedOnce(action));
        state.setAction(action, false);
        state.setAction(action, true);
        state.setAction(action, false);
        state.setAction(action, true);
        try std.testing.expect(state.takePressedOnce(action));
        try std.testing.expect(!state.takePressedOnce(action));
    }
    state.clearTransient();
    inline for (std.meta.tags(input.Action)) |action| {
        try std.testing.expect(state.isHeld(action));
        try std.testing.expect(!state.takePressedOnce(action));
    }
    state.finishFrame();
    state.setAction(.launch, true);
    try std.testing.expect(!state.takePressedOnce(.launch));
    state.clearAll();
    inline for (std.meta.tags(input.Action)) |action| {
        try std.testing.expect(!state.isHeld(action));
        try std.testing.expect(!state.takePressedOnce(action));
    }
    state.setAction(.launch, true);
    state.setAction(.pause, true);
    try std.testing.expect(state.takePressedOnce(.launch));
    try std.testing.expect(state.takePressedOnce(.pause));
    state.setAction(.pause, false);
    try std.testing.expect(state.isHeld(.launch));
}
test "key edges aliases repeat metadata and poisoned borrow have no retained state" {
    var bytes = [_]u8{'Q'};
    const event = input.NormalizedKeyEvent{ .key = .{ .character = &bytes }, .state = .pressed, .repeat = true, .modifiers = .{ .control = true, .alt = true, .super = true }, .text = &bytes };
    try input.validateKeyEvent(event);
    var state = input.InputState.init();
    state.setAction(input.actionForKey(event.key).?, event.state == .pressed);
    bytes[0] = 0xff;
    try std.testing.expect(state.isHeld(.decrease));
    try std.testing.expect(state.takePressedOnce(.decrease));
    state.setAction(input.actionForKey(.{ .character = "q" }).?, true);
    try std.testing.expect(!state.takePressedOnce(.decrease));
    state.setAction(input.actionForKey(.{ .character = "Q" }).?, false);
    try std.testing.expect(!state.isHeld(.decrease));
    state.clearAll();
    state.setAction(.decrease, event.state == .pressed);
    try std.testing.expect(state.takePressedOnce(.decrease));
    try std.testing.expectEqual(@as(usize, 30), @sizeOf(input.InputState));
    std.debug.print("key InputState bytes={d} bool={d} actions={d}\n", .{ @sizeOf(input.InputState), @sizeOf(bool), std.meta.tags(input.Action).len });
}
test "key edges full UTF8 validates character and optional text independently" {
    for ([_][]const u8{ "\x80", "ok\xc0\xaf", "\xe2\x82", "\xed\xa0\x80", "\xf4\x90\x80\x80", "ok\xff" }) |bad| {
        try std.testing.expectError(error.InvalidUtf8, input.validateKeyEvent(.{ .key = .{ .character = bad }, .state = .released, .repeat = false, .modifiers = .{}, .text = "valid" }));
        try std.testing.expectError(error.InvalidUtf8, input.validateKeyEvent(.{ .key = .{ .named = .other }, .state = .pressed, .repeat = true, .modifiers = .{}, .text = bad }));
        try std.testing.expectError(error.InvalidUtf8, input.validateIme(.{ .preedit = bad }));
        try std.testing.expectError(error.InvalidUtf8, input.validateIme(.{ .commit = bad }));
        try std.testing.expectError(error.InvalidUtf8, input.editorPrintableText(bad));
    }
    const text = "q\xcc\x81\xf0\x9f\x98\x80";
    try input.validateKeyEvent(.{ .key = .{ .character = text }, .state = .pressed, .repeat = true, .modifiers = .{}, .text = text });
    try std.testing.expectEqualStrings("q\xcc\x81\xf0\x9f\x98\x80", text);
}
test "key edges printable full scalar Cc predicate empty and format allowed" {
    for ([_][]const u8{ "", " ", "a\xcc\x81", "\xf0\x9f\x98\x80", "a\xe2\x80\x8db" }) |text| try std.testing.expect(try input.editorPrintableText(text));
    for ([_][]const u8{ "a\nb", "a\x00b", "a\x7fb", "a\xc2\x85b", "a\xc2\x9fb", "\x1f" }) |text| try std.testing.expect(!try input.editorPrintableText(text));
}
test "key edges IME complete large payload borrowed with no semantic cap or control veto" {
    var bytes: [8195]u8 = undefined;
    @memset(&bytes, 'a');
    @memcpy(bytes[8191..8195], "\xf0\x9f\x98\x80");
    bytes[7] = '\n';
    const preedit = input.NormalizedIme{ .preedit = &bytes };
    const commit = input.NormalizedIme{ .commit = &bytes };
    try input.validateIme(preedit);
    try input.validateIme(commit);
    try std.testing.expectEqual(@as(usize, 8195), preedit.preedit.len);
    try std.testing.expect(preedit.preedit.ptr == bytes[0..].ptr);
    try std.testing.expect(commit.commit.ptr == preedit.preedit.ptr);
    try std.testing.expectEqualSlices(u8, "\xf0\x9f\x98\x80", commit.commit[8191..]);
    try std.testing.expectEqual(@as(u8, '\n'), commit.commit[7]);
    try input.validateKeyEvent(.{ .key = .{ .character = &bytes }, .state = .pressed, .repeat = false, .modifiers = .{}, .text = &bytes });
    try std.testing.expectEqual(@as(?input.Action, null), input.actionForKey(.{ .character = &bytes }));
}

test "key edges FIX1 independent reset witnesses begin with every held and pending bit" {
    // Each reset has an independent fresh state; no pending bit is consumed
    // before its reset. Direct reads prove the unconsumed setup for all actions.
    inline for (.{ "clearTransient", "finishFrame", "clearAll" }) |method| {
        var state = input.InputState.init();
        inline for (std.meta.tags(input.Action)) |action| state.setAction(action, true);
        inline for (std.meta.tags(input.Action)) |action| {
            try std.testing.expect(state.isHeld(action));
            try std.testing.expect(state.pressed[@backingInt(action)]);
        }
        @field(input.InputState, method)(&state);
        inline for (std.meta.tags(input.Action)) |action| {
            const transient = !std.mem.eql(u8, method, "clearAll");
            try std.testing.expectEqual(transient, state.isHeld(action));
            try std.testing.expect(!state.takePressedOnce(action));
            state.setAction(action, true);
            try std.testing.expectEqual(!transient, state.takePressedOnce(action));
            if (transient) {
                state.setAction(action, false);
                try std.testing.expect(!state.isHeld(action));
                state.setAction(action, true);
                try std.testing.expect(state.takePressedOnce(action));
            }
            try std.testing.expect(state.isHeld(action));
            try std.testing.expect(!state.takePressedOnce(action));
        }
    }
}
