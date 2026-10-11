const std = @import("std");
const root = @import("root.zig");
test "H3d guide retained feature availability" {
    // Runnable baseline feature contract; absence is limited API evidence.
    try std.testing.expect(@hasDecl(root, "classic_guide"));
}
const G = @import("classic_guide.zig");
const H1 = @import("retained_semantics.zig");
const allocator = std.testing.allocator;
const Fixture = struct {
    tokens: H1.LiveTokenBudget,
    budget: H1.PreparationBudget,
    token: H1.TokenRef,
    fn init(self: *Fixture) !void {
        self.tokens = H1.LiveTokenBudget.init(.{ .max_live_objects = 2, .max_live_requested_bytes = 4096 });
        self.budget = H1.PreparationBudget.init(1179648);
        self.token = try H1.TokenRef.create(allocator, &self.tokens);
    }
    fn deinit(self: *Fixture) void {
        self.token.release();
        self.budget.deinit();
        self.tokens.deinit();
    }
    fn current(self: *const Fixture) G.CurrentStamp {
        return .{ .revision = 7, .owner = .{ .resident = self.token, .draft = null, .hold = 19, .context = "guide" } };
    }
    fn input(self: *const Fixture, topic: u8, sheet: u16, width: f32, height: f32) G.Input {
        return .{ .current = self.current(), .source = .{ .location = .{ .topic = topic, .sheet = sheet }, .viewport = .{ .x = width, .y = height }, .high_contrast = true } };
    }
};
const digests = [_][]const u8{
    "bad44dedc44e86e597b7bd52c27b12f01118c2d1fda81309edcb011f8e45be1d",
    "414023a844ac91eda7e9715ba3989acef0e8019fbc66a1e6e76aaa9ec5ed73bc",
    "91ee81434df53f34ba11c9ba96e94229f7475ac60be447640c867827d0e4da5c",
    "b20469cff510cb2d5efb91ed7ba947a9da74ed804d45a09cad7cd75e3e972065",
    "f9d55420aec90ad907a34730eb6ac9c0d26fcf2ab915b4c3f5ac2d7687f5dcbf",
    "3079105e03a893fe0215aa6073d2f0e2da37d699630608f567008a5e45125136",
    "9d9c75154db348ccaa1dbbb2b196a6056e05ff57d2081740bf5b808e59f6a821",
};
test "H3d guide independent corpus identities and exact literal inventories" {
    var paragraphs: usize = 0;
    var bytes: usize = 0;
    var titles: usize = 0;
    const expected_counts = [_]usize{ 3, 5, 5, 4, 9, 4, 4 };
    const expected_bytes = [_]usize{ 787, 1229, 1204, 1138, 3548, 1225, 1460 };
    const expected_max = [_]usize{ 306, 280, 265, 313, 532, 359, 429 };
    for (G.topics, 0..) |topic, i| {
        var sha = std.crypto.hash.sha2.Sha256.init(.{});
        sha.update(topic.title);
        sha.update(&.{0});
        var topic_bytes: usize = 0;
        var max: usize = 0;
        for (topic.paragraphs, 0..) |p, j| {
            if (j > 0) sha.update(&.{0});
            sha.update(p);
            topic_bytes += p.len;
            max = @max(max, p.len);
        }
        var actual: [32]u8 = undefined;
        sha.final(&actual);
        var expected: [32]u8 = undefined;
        _ = try std.fmt.hexToBytes(&expected, digests[i]);
        try std.testing.expectEqualSlices(u8, &expected, &actual);
        try std.testing.expectEqual(expected_counts[i], topic.paragraphs.len);
        try std.testing.expectEqual(expected_bytes[i], topic_bytes);
        try std.testing.expectEqual(expected_max[i], max);
        paragraphs += topic.paragraphs.len;
        bytes += topic_bytes;
        titles += topic.title.len;
    }
    try std.testing.expectEqual(@as(usize, 34), paragraphs);
    try std.testing.expectEqual(@as(usize, 10591), bytes);
    try std.testing.expectEqual(@as(usize, 172), titles);
    try std.testing.expectEqual(@as(usize, 20), "NYON // Player guide".len);
    try std.testing.expectEqual(@as(usize, 41), "Time held // Tab + Enter // Escape closes".len);
    try std.testing.expectEqual(@as(usize, 104), 4 * ("Previous".len + "Next".len + "Main menu".len + "Close".len));
    try std.testing.expectEqual(@as(usize, 7440), 7130 + 12 + 17 + 33 + 20 + 43 + 41 + 144);
    try std.testing.expectEqual(@as(usize, 7173), 7130 + 43);
    std.debug.print("H3d Header={d} Metadata={d} Word={d} Line={d} Publication={d} Handle={d} TopicDescriptor={d} descriptors={d} literal payload=10763; mapped footprint/stack UNKNOWN\n", .{ G.HEADER_BYTES, G.METADATA_BYTES, @sizeOf(G.WordSpan), @sizeOf(G.LineSpan), @sizeOf(G.GuidePublication), @sizeOf(G.PreparedGuide), @sizeOf(G.TopicText), @sizeOf(@TypeOf(G.topics)) });
}
test "H3d guide geometry footer normalized endpoints topology and focus" {
    var f: Fixture = undefined;
    try f.init();
    defer f.deinit();
    var input = f.input(0, 0, 320, 320);
    input.source.focused = .{ .bytes = "guide.previous" };
    var p = try G.prepareGuide(allocator, &f.budget, input);
    defer p.deinit();
    const g = try p.geometry();
    const tree = try p.input();
    try std.testing.expectEqual(@as(u16, 29), g.effective_columns);
    try std.testing.expectEqual(@as(u16, 4), g.effective_rows);
    try std.testing.expectEqual(@as(usize, 16), tree.nodes.len);
    try std.testing.expectEqual(@as(usize, 15), tree.child_indices.len);
    try std.testing.expectEqual(@as(usize, 3), tree.focus.base.len);
    try std.testing.expect(tree.focus.focused == null);
    try std.testing.expectEqualStrings("guide.body.value", tree.nodes[6].id.bytes);
    try std.testing.expectEqual(@as(?usize, 5), tree.nodes[6].parent);
    try std.testing.expectEqualStrings("guide.next.node", tree.nodes[7].id.bytes);
    try std.testing.expectEqualSlices(f32, &.{ 86, 258 }, &tree.nodes[7].bounds.?.min);
    try std.testing.expectEqualSlices(f32, &.{ 156, 306 }, &tree.nodes[7].bounds.?.max);
    try std.testing.expectEqual(@as(usize, 0), tree.announcements.len);
    var edges: usize = 0;
    for (tree.nodes, 0..) |n, i| {
        try std.testing.expectEqual(edges, n.child_start);
        for (tree.child_indices[n.child_start..][0..n.child_count]) |child| try std.testing.expectEqual(@as(?usize, i), tree.nodes[child].parent);
        edges += n.child_count;
    }
    try std.testing.expectEqual(tree.nodes.len - 1, edges);
    p.deinit();
    input = f.input(255, 65535, 640, 480);
    p = try G.prepareGuide(allocator, &f.budget, input);
    const last = try p.geometry();
    try std.testing.expectEqual(@as(u8, 6), last.location.topic);
    try std.testing.expectEqual(last.sheets - 1, last.location.sheet);
    try std.testing.expectEqual(@as(u16, 62), last.effective_columns);
    try std.testing.expectEqual(@as(u16, 11), last.effective_rows);
    const lt = try p.input();
    for (lt.focus.base) |a| try std.testing.expect(!std.mem.eql(u8, a.bytes, "guide.next"));
    p.deinit();
    input = f.input(4, 0, 640, 480);
    input.source.focused = .{ .bytes = "guide.next" };
    p = try G.prepareGuide(allocator, &f.budget, input);
    try std.testing.expectEqual(@as(usize, 19), (try p.input()).nodes.len);
    try std.testing.expectEqualStrings("guide.next", (try p.input()).focus.focused.?.bytes);
    const line = try p.line(0);
    try std.testing.expectEqual(@as(f32, 18), line.position.x);
    try std.testing.expectEqual(@as(f32, 134), line.position.y);
    try std.testing.expectEqual(@as(f32, 1.6), line.scale);
    try std.testing.expectError(error.InvalidLine, p.line((try p.bodyLines()).len));
}
// Independent source wrapper: owns transient test strings, uses source text only.
fn oracle(alloc: std.mem.Allocator, paragraphs: []const []const u8, columns: usize) ![][]const u8 {
    var lines: std.array_list.Managed([]const u8) = .init(alloc);
    errdefer {
        for (lines.items) |s| alloc.free(s);
        lines.deinit();
    }
    for (paragraphs) |paragraph| {
        var current: std.array_list.Managed(u8) = .init(alloc);
        defer current.deinit();
        var words = std.mem.tokenizeAny(u8, paragraph, " \t\r\n\x0b\x0c");
        while (words.next()) |word| {
            if (current.items.len > 0 and current.items.len + 1 + word.len > columns) {
                try lines.append(try alloc.dupe(u8, current.items));
                current.clearRetainingCapacity();
            }
            if (current.items.len > 0) try current.append(' ');
            try current.appendSlice(word);
        }
        if (current.items.len > 0) try lines.append(try alloc.dupe(u8, current.items));
        try lines.append(try alloc.dupe(u8, ""));
    }
    return lines.toOwnedSlice();
}
fn freeOracle(lines: [][]const u8) void {
    for (lines) |l| allocator.free(l);
    allocator.free(lines);
}
test "H3d guide every topic page independent wrap preserved uppercase and blank spans" {
    var f: Fixture = undefined;
    try f.init();
    defer f.deinit();
    for (G.topics, 0..) |topic, ti| {
        for ([_][2]f32{ .{ 320, 320 }, .{ 640, 480 }, .{ 8000, 16000 } }) |v| {
            const cols: usize = if (v[0] == 320) 29 else if (v[0] == 640) 62 else 532;
            const rows: usize = if (v[1] == 320) 4 else if (v[1] == 480) 11 else 530;
            const expected = try oracle(allocator, topic.paragraphs, cols);
            defer freeOracle(expected);
            const sheets = (expected.len + rows - 1) / rows;
            for (0..sheets) |sheet| {
                var p = try G.prepareGuide(allocator, &f.budget, f.input(@intCast(ti), @intCast(sheet), v[0], v[1]));
                defer p.deinit();
                const from = sheet * rows;
                const selected = expected[from..@min(from + rows, expected.len)];
                const joined = try std.mem.join(allocator, " ", selected);
                defer allocator.free(joined);
                try std.testing.expectEqualStrings(joined, (try p.input()).nodes[5].description);
                const upper = try allocator.dupe(u8, joined);
                defer allocator.free(upper);
                for (upper) |*byte| byte.* = std.ascii.toUpper(byte.*);
                try std.testing.expectEqualStrings(upper, (try p.input()).nodes[6].value.?);
                try std.testing.expectEqual(selected.len, (try p.bodyLines()).len);
                for (selected, 0..) |s, i| {
                    const actual = try p.line(i);
                    const u = try allocator.dupe(u8, s);
                    defer allocator.free(u);
                    for (u) |*byte| byte.* = std.ascii.toUpper(byte.*);
                    try std.testing.expectEqualStrings(u, actual.text);
                }
                try std.testing.expect(joined.len <= 3565);
            }
        }
    }
}
test "H3d guide durable publication rollback trusted actions and empty accessors" {
    var f: Fixture = undefined;
    try f.init();
    defer f.deinit();
    var slot = H1.FrameSlot.init(allocator, &f.budget);
    defer slot.deinit();
    var publication = G.GuidePublication{};
    var input = f.input(1, 0, 640, 480);
    var p = try G.prepareGuide(allocator, &f.budget, input);
    defer p.deinit();
    try G.publishGuide(&slot, &publication, &p, f.current());
    const saved = publication;
    const old_nodes = slot.view().?.nodes.ptr;
    p.deinit();
    input.source.location = .{ .topic = 6, .sheet = 100 };
    const action = try G.validateGuideAction(slot.view().?, &publication, f.current(), .{ .bytes = "guide.next" });
    try std.testing.expectEqual(saved.next.?.topic, action.show.topic);
    try std.testing.expectEqual(saved.next.?.sheet, action.show.sheet);
    try std.testing.expectError(error.EmptyProjection, p.input());
    try std.testing.expectError(error.EmptyProjection, p.geometry());
    try std.testing.expectError(error.EmptyProjection, p.bodyLines());
    try std.testing.expectError(error.EmptyProjection, p.line(0));
    p = try G.prepareGuide(allocator, &f.budget, input);
    var stale = f.current();
    stale.revision += 1;
    try std.testing.expectError(error.CurrentMismatch, G.publishGuide(&slot, &publication, &p, stale));
    try std.testing.expect(std.meta.eql(saved, publication));
    try std.testing.expectEqual(old_nodes, slot.view().?.nodes.ptr);
    var other = H1.PreparationBudget.init(1179648);
    defer other.deinit();
    var other_slot = H1.FrameSlot.init(allocator, &other);
    defer other_slot.deinit();
    try std.testing.expectError(error.BudgetMismatch, G.publishGuide(&other_slot, &publication, &p, f.current()));
    try std.testing.expectError(error.InvalidUtf8, G.validateGuideAction(slot.view().?, &publication, f.current(), .{ .bytes = &.{0xff} }));
    try std.testing.expectError(error.UnknownAction, G.validateGuideAction(slot.view().?, &publication, f.current(), .{ .bytes = "guide.next/2" }));
    try std.testing.expectError(error.CurrentMismatch, G.validateGuideAction(slot.view().?, &publication, stale, .{ .bytes = "guide.close" }));
    var bad = saved;
    bad.next = .{ .topic = 7, .sheet = 0 };
    try std.testing.expectError(error.InvalidPublication, G.validateGuideAction(slot.view().?, &bad, f.current(), .{ .bytes = "guide.next" }));
    bad = .{};
    try std.testing.expectError(error.NoPublication, G.validateGuideAction(slot.view().?, &bad, f.current(), .{ .bytes = "guide.close" }));
    f.budget.max_live_requested_bytes = f.budget.ledger().total;
    try std.testing.expectError(error.LimitExceeded, G.publishGuide(&slot, &publication, &p, f.current()));
    try std.testing.expect(std.meta.eql(saved, publication));
    try std.testing.expectEqual(old_nodes, slot.view().?.nodes.ptr);
    f.budget.max_live_requested_bytes = 1179648;
    try G.publishGuide(&slot, &publication, &p, f.current());
}
test "H3d guide geometry UTF8 ownership borrowed focus budget and real OOM" {
    var f: Fixture = undefined;
    try f.init();
    defer f.deinit();
    var input = f.input(0, 0, -9, 0);
    var p = try G.prepareGuide(allocator, &f.budget, input);
    defer p.deinit();
    try std.testing.expectEqual(@as(f32, 320), (try p.geometry()).viewport.x);
    p.deinit();
    input.source.viewport = .{ .x = std.math.nan(f32), .y = 800 };
    p = try G.prepareGuide(allocator, &f.budget, input);
    try std.testing.expectEqual(@as(f32, 640), (try p.geometry()).viewport.x);
    p.deinit();
    input.source.viewport = .{ .x = std.math.floatMax(f32), .y = std.math.floatMax(f32) };
    try std.testing.expectError(error.InvalidGeometry, G.prepareGuide(allocator, &f.budget, input));
    input = f.input(0, 0, 640, 480);
    input.source.focused = .{ .bytes = "guide.unknown" };
    p = try G.prepareGuide(allocator, &f.budget, input);
    try std.testing.expect((try p.input()).focus.focused == null);
    p.deinit();
    input.source.viewport = .{ .x = 1000, .y = std.math.inf(f32) };
    p = try G.prepareGuide(allocator, &f.budget, input);
    try std.testing.expectEqual(@as(f32, 640), (try p.geometry()).viewport.x);
    try std.testing.expectEqual(@as(f32, 480), (try p.geometry()).viewport.y);
    p.deinit();
    input = f.input(0, 0, 640, 480);
    input.source.focused = .{ .bytes = &.{0xff} };
    try std.testing.expectError(error.InvalidUtf8, G.prepareGuide(allocator, &f.budget, input));
    input.source.focused = null;
    input.current.owner.context = &.{0xff};
    try std.testing.expectError(error.InvalidUtf8, G.prepareGuide(allocator, &f.budget, input));
    var context: [129]u8 = @splat('a');
    input.current.owner.context = &context;
    try std.testing.expectError(error.InvalidOwner, G.prepareGuide(allocator, &f.budget, input));
    input = f.input(1, 0, 640, 480);
    var focus = try allocator.dupe(u8, "guide.close");
    input.source.focused = .{ .bytes = focus };
    p = try G.prepareGuide(allocator, &f.budget, input);
    allocator.free(focus);
    focus = &.{};
    try std.testing.expectEqualStrings("guide.close", (try p.input()).focus.focused.?.bytes);
    p.deinit();
    input = f.input(1, 0, 640, 480);
    var fail = std.testing.FailingAllocator.init(allocator, .{ .fail_index = 0 });
    try std.testing.expectError(error.OutOfMemory, G.prepareGuide(fail.allocator(), &f.budget, input));
    try std.testing.expectEqual(@as(usize, 0), f.budget.ledger().total);
    f.budget.max_live_requested_bytes = G.HEADER_BYTES - 1;
    try std.testing.expectError(error.LimitExceeded, G.prepareGuide(allocator, &f.budget, input));
    f.budget.max_live_requested_bytes = G.HEADER_BYTES;
    p = try G.prepareGuide(allocator, &f.budget, input);
    try std.testing.expectEqual(G.HEADER_BYTES, f.budget.ledger().total);
    p.deinit();
    f.budget.max_live_requested_bytes = 1179648;
    const draft = try H1.TokenRef.create(allocator, &f.tokens);
    defer draft.release();
    input.current.owner.draft = draft;
    p = try G.prepareGuide(allocator, &f.budget, input);
    try std.testing.expect((try p.input()).owner.draft.?.sameIdentity(draft));
    p.deinit();
}
const Trace = struct {
    budget: *H1.PreparationBudget,
    events: [128]Event = undefined,
    count: usize = 0,
    live: usize = 0,
    peak: usize = 0,
    fail_at: ?usize = null,
    const Event = struct { bytes: usize, charged: usize, pending: usize, admitted: usize, failed: bool };
    fn alloc(ctx: *anyopaque, len: usize, alignment: std.mem.Alignment, ra: usize) ?[*]u8 {
        const self: *Trace = @ptrCast(@alignCast(ctx));
        const total = self.budget.ledger().total + self.budget.pending_bytes;
        std.debug.assert(self.count < self.events.len);
        std.debug.assert(self.live == self.budget.ledger().total);
        const fail = if (self.fail_at) |ordinal| ordinal == self.count else false;
        self.events[self.count] = .{ .bytes = len, .charged = self.budget.ledger().total, .pending = self.budget.pending_bytes, .admitted = total, .failed = fail };
        self.count += 1;
        self.peak = @max(self.peak, total);
        if (fail) return null;
        const ptr = allocator.rawAlloc(len, alignment, ra) orelse return null;
        self.live += len;
        return ptr;
    }
    fn free(ctx: *anyopaque, memory: []u8, alignment: std.mem.Alignment, ra: usize) void {
        const self: *Trace = @ptrCast(@alignCast(ctx));
        allocator.rawFree(memory, alignment, ra);
        self.live -= memory.len;
    }
    fn allocatorHandle(self: *Trace) std.mem.Allocator {
        return .{ .ptr = self, .vtable = &.{ .alloc = alloc, .resize = std.mem.Allocator.noResize, .remap = std.mem.Allocator.noRemap, .free = free } };
    }
};
const Consumer = enum { guide, hud, editor };
const Transitions = [_][2]Consumer{ .{ .guide, .guide }, .{ .hud, .guide }, .{ .editor, .guide }, .{ .guide, .hud }, .{ .guide, .editor } };
const TransitionResult = struct { peak: usize, events: [128]Trace.Event, count: usize, destination_start: usize, failed: bool };
fn transition(old: Consumer, destination: Consumer, limit: usize, fail_destination: ?usize) !TransitionResult {
    const UI = @import("classic_ui.zig");
    const EU = @import("classic_editor_ui.zig");
    const EL = @import("classic_editor_layout.zig");
    const EM = @import("classic_editor.zig");
    const SC = @import("scenario.zig");
    const TX = @import("classic_editor_transactions.zig");
    var f: Fixture = undefined;
    try f.init();
    defer f.deinit();
    const draft_token = try H1.TokenRef.create(allocator, &f.tokens);
    defer draft_token.release();
    var draft = try SC.ScenarioDraft.factoryDefault(allocator);
    defer draft.deinit();
    var editor = try EM.EditorState.init(allocator, &draft);
    defer editor.deinit();
    editor.section = .rules;
    var report = editor.validationReport();
    var flags: EL.ParseFlags = @splat(false);
    var workspace: EL.Workspace = .{};
    const status: TX.Status = .none;
    const esource = EU.Source{ .editor = &editor, .report = &report, .parse_flags = &flags, .viewport = .{ 960, 600 }, .store_status = &status, .action_failure = null, .focus = .{ .current = null } };
    const ec = EU.CurrentStamp{ .revision = 7, .owner = .{ .resident = f.token, .draft = draft_token, .hold = 19, .context = "editor" } };
    const hc = UI.CurrentStamp{ .revision = 7, .owner = .{ .resident = f.token, .draft = null, .hold = 19, .context = "hud" } };
    var trace = Trace{ .budget = &f.budget };
    const a = trace.allocatorHandle();
    var slot = H1.FrameSlot.init(a, &f.budget);
    defer {
        slot.deinit();
        std.debug.assert(trace.live == 0);
    }
    var publication = G.GuidePublication{};
    var revision: ?u64 = null;
    switch (old) {
        .guide => {
            var p = try G.prepareGuide(a, &f.budget, f.input(4, 0, 8000, 16000));
            defer p.deinit();
            try G.publishGuide(&slot, &publication, &p, f.current());
        },
        .hud => {
            var p = try UI.prepareHudSettings(a, &f.budget, .{ .mode = .playing, .help_visible = true, .onboarding_step = .select_union_world }, hc);
            defer p.deinit();
            try UI.publishHudSettings(&slot, &revision, &p, hc);
        },
        .editor => {
            var p = try EU.prepareEditor(a, &f.budget, &workspace, esource, ec);
            defer p.deinit();
            try EU.publishEditor(&slot, &revision, &p, ec);
        },
    }
    const backing = slot.view().?.nodes.ptr;
    const old_publication = publication;
    const old_revision = revision;
    const old_total = f.budget.ledger().total;
    const start = trace.count;
    trace.peak = trace.live;
    f.budget.max_live_requested_bytes = limit;
    if (fail_destination) |ordinal| trace.fail_at = start + ordinal;
    const result: anyerror!void = blk: {
        switch (destination) {
            .guide => {
                var p = G.prepareGuide(a, &f.budget, f.input(4, 0, 8000, 16000)) catch |err| break :blk err;
                defer p.deinit();
                G.publishGuide(&slot, &publication, &p, f.current()) catch |err| break :blk err;
            },
            .hud => {
                var p = UI.prepareHudSettings(a, &f.budget, .{ .mode = .playing, .help_visible = true, .onboarding_step = .select_union_world }, hc) catch |err| break :blk err;
                defer p.deinit();
                UI.publishHudSettings(&slot, &revision, &p, hc) catch |err| break :blk err;
                publication = .{};
            },
            .editor => {
                var p = EU.prepareEditor(a, &f.budget, &workspace, esource, ec) catch |err| break :blk err;
                defer p.deinit();
                EU.publishEditor(&slot, &revision, &p, ec) catch |err| break :blk err;
                publication = .{};
            },
        }
        break :blk {};
    };
    var failed = false;
    result catch |err| {
        try std.testing.expect(err == error.OutOfMemory or err == error.LimitExceeded);
        failed = true;
        try std.testing.expectEqual(backing, slot.view().?.nodes.ptr);
        try std.testing.expect(std.meta.eql(old_publication, publication));
        try std.testing.expectEqual(old_revision, revision);
        try std.testing.expectEqual(old_total, f.budget.ledger().total);
    };
    try std.testing.expectEqual(@as(usize, 0), f.budget.pending_bytes);
    try std.testing.expectEqual(@as(usize, 0), f.budget.active_reservations);
    return .{ .peak = trace.peak, .events = trace.events, .count = trace.count, .destination_start = start, .failed = failed };
}
test "H3d guide five actual old frame transitions exact requested byte P boundary and every OOM ordinal" {
    for (Transitions) |pair| {
        const observed = try transition(pair[0], pair[1], 1179648, null);
        try std.testing.expect(!observed.failed);
        try std.testing.expect(observed.peak > 0);
        std.debug.print("H3d transition {s}->{s} P={d} requests={d} destination_start={d}\n", .{ @tagName(pair[0]), @tagName(pair[1]), observed.peak, observed.count, observed.destination_start });
        for (observed.events[0..observed.count], 0..) |event, i| std.debug.print(" request[{d}] bytes={d} committed={d} pending={d} admitted={d}\n", .{ i, event.bytes, event.charged, event.pending, event.admitted });
        const exact = try transition(pair[0], pair[1], observed.peak, null);
        try std.testing.expect(!exact.failed);
        try std.testing.expectEqual(observed.count, exact.count);
        try std.testing.expectEqualDeep(observed.events[0..observed.count], exact.events[0..exact.count]);
        const short = try transition(pair[0], pair[1], observed.peak - 1, null);
        try std.testing.expect(short.failed);
        for (0..observed.count - observed.destination_start) |ordinal| {
            const failed = try transition(pair[0], pair[1], 1179648, ordinal);
            try std.testing.expect(failed.failed);
        }
    }
}
test "H3d guide full copied context source purity exact copies raw q5 and repeat publication" {
    var f: Fixture = undefined;
    try f.init();
    defer f.deinit();
    var trace = Trace{ .budget = &f.budget };
    const a = trace.allocatorHandle();
    var slot = H1.FrameSlot.init(a, &f.budget);
    defer slot.deinit();
    var publication = G.GuidePublication{};
    var context: [128]u8 = @splat('x');
    var input = f.input(4, 0, 8000, 16000);
    input.current.owner.context = &context;
    input.source.focused = .{ .bytes = "guide.close" };
    const before = input;
    var p = try G.prepareGuide(a, &f.budget, input);
    defer p.deinit();
    try std.testing.expect(std.meta.eql(before, input));
    const tree = try p.input();
    var button_bytes: usize = 0;
    var copied_text: usize = 0;
    for (tree.nodes) |n| {
        copied_text += n.name.len + n.description.len + (if (n.value) |v| v.len else 0);
        if (n.role == .button) {
            try std.testing.expectEqualStrings(n.name, n.description);
            button_bytes += n.name.len * 4;
        }
    }
    try std.testing.expectEqual(@as(usize, 104), button_bytes);
    try std.testing.expect(copied_text <= 7400);
    try std.testing.expectEqual(@as(usize, 4), tree.focus.base.len);
    try std.testing.expectEqual(@as(u16, 532), (try p.geometry()).effective_columns);
    try std.testing.expectEqual(@as(u16, 530), (try p.geometry()).effective_rows);
    const original_words = p.header.?.words[0..p.header.?.m.word_count];
    for (original_words) |word| {
        try std.testing.expect(word.paragraph < 9);
        try std.testing.expect(@as(usize, word.start) + word.len <= G.topics[4].paragraphs[word.paragraph].len);
    }
    try G.publishGuide(&slot, &publication, &p, input.current);
    p.deinit();
    var found_q5 = false;
    for (trace.events[0..trace.count]) |event| if (event.bytes == 80) {
        found_q5 = true;
    };
    try std.testing.expect(found_q5);
    @memset(&context, 'z');
    try std.testing.expectError(error.CurrentMismatch, G.validateGuideAction(slot.view().?, &publication, input.current, .{ .bytes = "guide.close" }));
    try std.testing.expectEqual(@as(u8, 'x'), slot.view().?.owner.context[0]);
    var current = input.current;
    var saved_context: [128]u8 = @splat('x');
    current.owner.context = &saved_context;
    try std.testing.expectEqualDeep(G.Action.close, try G.validateGuideAction(slot.view().?, &publication, current, .{ .bytes = "guide.close" }));
    const other = try H1.TokenRef.create(allocator, &f.tokens);
    defer other.release();
    var wrong = current;
    wrong.owner.resident = other;
    try std.testing.expectError(error.CurrentMismatch, G.validateGuideAction(slot.view().?, &publication, wrong, .{ .bytes = "guide.close" }));
    wrong = current;
    wrong.owner.hold = null;
    try std.testing.expectError(error.CurrentMismatch, G.validateGuideAction(slot.view().?, &publication, wrong, .{ .bytes = "guide.close" }));
    // Repetition uses normal allocator after ending trace-owned slot lifetime.
    slot.deinit();
    slot = H1.FrameSlot.init(allocator, &f.budget);
    try std.testing.expectEqual(@as(usize, 0), trace.live);
    try std.testing.expectEqual(@as(usize, 0), f.budget.ledger().total);
    for (0..64) |i| {
        p = try G.prepareGuide(allocator, &f.budget, f.input(@intCast(i % 7), @intCast(i), 640, 480));
        try G.publishGuide(&slot, &publication, &p, f.current());
        p.deinit();
        try std.testing.expectEqual(@as(usize, 0), f.budget.ledger().frame_scratch);
        try std.testing.expectEqual(f.budget.ledger().resident_frame, f.budget.ledger().total);
    }
}
test "H3d guide cross topic navigation exact previous sheets resize endpoint inactive and no publication" {
    var f: Fixture = undefined;
    try f.init();
    defer f.deinit();
    var slot = H1.FrameSlot.init(allocator, &f.budget);
    defer slot.deinit();
    var publication = G.GuidePublication{};
    for (1..7) |topic| {
        const expected = try oracle(allocator, G.topics[topic - 1].paragraphs, 29);
        defer freeOracle(expected);
        var p = try G.prepareGuide(allocator, &f.budget, f.input(@intCast(topic), 0, 320, 320));
        defer p.deinit();
        try G.publishGuide(&slot, &publication, &p, f.current());
        const action = try G.validateGuideAction(slot.view().?, &publication, f.current(), .{ .bytes = "guide.previous" });
        try std.testing.expectEqual(@as(u8, @intCast(topic - 1)), action.show.topic);
        try std.testing.expectEqual(@as(u16, @intCast((expected.len + 3) / 4 - 1)), action.show.sheet);
    }
    var p = try G.prepareGuide(allocator, &f.budget, f.input(0, 0, 640, 480));
    defer p.deinit();
    try G.publishGuide(&slot, &publication, &p, f.current());
    publication.previous = .{ .topic = 0, .sheet = 0 };
    try std.testing.expectError(error.InactiveAction, G.validateGuideAction(slot.view().?, &publication, f.current(), .{ .bytes = "guide.previous" }));
    p.deinit();
    p = try G.prepareGuide(allocator, &f.budget, f.input(6, 65535, 8000, 16000));
    try G.publishGuide(&slot, &publication, &p, f.current());
    try std.testing.expectEqual(@as(u16, 0), (try p.geometry()).location.sheet);
    try std.testing.expect(publication.next == null);
    publication.next = .{ .topic = 6, .sheet = 1 };
    try std.testing.expectError(error.InactiveAction, G.validateGuideAction(slot.view().?, &publication, f.current(), .{ .bytes = "guide.next" }));
    try std.testing.expectEqualDeep(G.Action.main_menu, try G.validateGuideAction(slot.view().?, &publication, f.current(), .{ .bytes = "guide.menu" }));
}
// Ruling 36 qualification technique: observe actual token allocations, validate
// the frozen ABI, then inject unreachable checked overflow only in this test.
const TokenMirror = struct { allocator: std.mem.Allocator, refs: usize, budget: *H1.LiveTokenBudget, charged_bytes: usize };
const TokenObserver = struct {
    pointers: [2][*]u8 = undefined,
    lengths: [2]usize = undefined,
    alignments: [2]std.mem.Alignment = undefined,
    count: usize = 0,
    fn alloc(ctx: *anyopaque, len: usize, alignment: std.mem.Alignment, ra: usize) ?[*]u8 {
        const self: *TokenObserver = @ptrCast(@alignCast(ctx));
        const ptr = allocator.rawAlloc(len, alignment, ra) orelse return null;
        std.debug.assert(self.count < 2);
        self.pointers[self.count] = ptr;
        self.lengths[self.count] = len;
        self.alignments[self.count] = alignment;
        self.count += 1;
        return ptr;
    }
    fn free(_: *anyopaque, memory: []u8, alignment: std.mem.Alignment, ra: usize) void {
        allocator.rawFree(memory, alignment, ra);
    }
    fn allocatorHandle(self: *TokenObserver) std.mem.Allocator {
        return .{ .ptr = self, .vtable = &.{ .alloc = alloc, .resize = std.mem.Allocator.noResize, .remap = std.mem.Allocator.noRemap, .free = free } };
    }
    fn observed(self: *TokenObserver, index: usize, budget: *H1.LiveTokenBudget) !*TokenMirror {
        try std.testing.expectEqual(@sizeOf(TokenMirror), self.lengths[index]);
        try std.testing.expect(self.alignments[index].toByteUnits() >= @alignOf(TokenMirror));
        const token: *TokenMirror = @ptrCast(@alignCast(self.pointers[index]));
        const a = self.allocatorHandle();
        try std.testing.expect(token.allocator.ptr == a.ptr and token.allocator.vtable == a.vtable);
        try std.testing.expect(token.budget == budget);
        try std.testing.expectEqual(self.lengths[index], token.charged_bytes);
        try std.testing.expectEqual(@as(usize, 1), token.refs);
        return token;
    }
};
test "H3d guide synthetic observed token ABI resident draft same token overflow and publication rollback" {
    var tokens = H1.LiveTokenBudget.init(.{ .max_live_objects = 2, .max_live_requested_bytes = 4096 });
    defer tokens.deinit();
    var observer = TokenObserver{};
    const resident = try H1.TokenRef.create(observer.allocatorHandle(), &tokens);
    defer resident.release();
    const draft = try H1.TokenRef.create(observer.allocatorHandle(), &tokens);
    defer draft.release();
    const rm = try observer.observed(0, &tokens);
    const dm = try observer.observed(1, &tokens);
    var budget = H1.PreparationBudget.init(1179648);
    defer budget.deinit();
    var slot = H1.FrameSlot.init(allocator, &budget);
    defer slot.deinit();
    var publication = G.GuidePublication{};
    var input = G.Input{ .source = .{ .location = .{ .topic = 1, .sheet = 0 }, .viewport = .{ .x = 640, .y = 480 }, .high_contrast = false }, .current = .{ .revision = 8, .owner = .{ .resident = resident, .draft = null, .hold = 3, .context = "guide" } } };
    {
        var p = try G.prepareGuide(allocator, &budget, input);
        defer p.deinit();
        try G.publishGuide(&slot, &publication, &p, input.current);
    }
    const backing = slot.view().?.nodes.ptr;
    const saved = publication;
    const total = budget.ledger().total;
    for (0..3) |case| {
        const original_r = rm.refs;
        const original_d = dm.refs;
        defer {
            rm.refs = original_r;
            dm.refs = original_d;
        }
        input.current.owner.draft = draft;
        if (case == 0) rm.refs = std.math.maxInt(usize) else if (case == 1) {
            rm.refs = std.math.maxInt(usize) - 1;
            input.current.owner.draft = resident;
        } else dm.refs = std.math.maxInt(usize);
        const injected_r = rm.refs;
        const injected_d = dm.refs;
        try std.testing.expectError(error.ReferenceOverflow, G.prepareGuide(allocator, &budget, input));
        try std.testing.expectEqual(injected_r, rm.refs);
        try std.testing.expectEqual(injected_d, dm.refs);
        try std.testing.expectEqual(total, budget.ledger().total);
        try std.testing.expectEqual(@as(usize, 0), budget.pending_bytes);
        try std.testing.expectEqual(@as(usize, 0), budget.ledger().frame_scratch);
        try std.testing.expectEqual(backing, slot.view().?.nodes.ptr);
        try std.testing.expect(std.meta.eql(saved, publication));
    }
    input.current.owner.draft = draft;
    var prepared = try G.prepareGuide(allocator, &budget, input);
    defer prepared.deinit();
    const prepared_total = budget.ledger().total;
    for (0..2) |case| {
        const original_r = rm.refs;
        const original_d = dm.refs;
        defer {
            rm.refs = original_r;
            dm.refs = original_d;
        }
        if (case == 0) rm.refs = std.math.maxInt(usize) else dm.refs = std.math.maxInt(usize);
        const ir = rm.refs;
        const id = dm.refs;
        try std.testing.expectError(error.ReferenceOverflow, G.publishGuide(&slot, &publication, &prepared, input.current));
        try std.testing.expectEqual(ir, rm.refs);
        try std.testing.expectEqual(id, dm.refs);
        try std.testing.expectEqual(prepared_total, budget.ledger().total);
        try std.testing.expectEqual(@as(usize, 0), budget.pending_bytes);
        try std.testing.expectEqual(backing, slot.view().?.nodes.ptr);
        try std.testing.expect(std.meta.eql(saved, publication));
    }
    try G.publishGuide(&slot, &publication, &prepared, input.current);
}
