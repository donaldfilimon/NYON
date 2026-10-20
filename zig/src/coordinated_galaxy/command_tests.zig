const std = @import("std");
const t = @import("types.zig");
const state = @import("state.zig");
const command = @import("command.zig");

fn setup(storage: *state.Storage) !void {
    while (true) {
        var budget: t.WorkBudget = .{ .remaining = 4096 };
        if (try storage.advanceSetup(&budget) == .done) return;
    }
}

fn finish(cursor: *command.Cursor, allowance: u16) !void {
    while (true) {
        var budget: t.WorkBudget = .{ .remaining = allowance };
        if (try cursor.advance(&budget) == .done) return;
    }
}

fn phase(p: anytype, which: t.Phase, allowance: u16) !void {
    while (true) {
        var budget: t.WorkBudget = .{ .remaining = allowance };
        if (try p.advancePhase(which, &budget) == .done) return;
    }
}

fn prepared(p: anytype, expected: u64, target: u64, policy: t.SessionPolicy, cursor: *const command.Cursor, allowance: u16) !void {
    try p.beginPrepared(@fromBackingInt(expected), @fromBackingInt(target), policy, cursor.ownedLengths(@fromBackingInt(1)), cursor.globalWatermark());
    for ([_]t.Phase{ .prepare, .commands, .arrivals, .construction, .production, .population, .local_finalize }) |which| try phase(p, which, allowance);
    while (true) {
        var budget: t.WorkBudget = .{ .remaining = allowance };
        if (try p.freezePrepared(&budget) != null) return;
    }
}

test "cg/commands/global-order-and-ownership" {
    var storage = try state.Storage.init(std.testing.allocator, .{ .seed = 2, .planet_count = 2, .civilization_count = 4 });
    defer storage.deinit();
    try setup(&storage);
    var bytes: [4][1024]u8 = undefined;
    var routed: [4]t.RoutedEnvelope = undefined;
    for ([_]u64{ 4, 2, 3, 1 }, 0..) |sequence, i| {
        const planet: u32 = if (sequence % 2 == 0) 2 else 1;
        const encoded = try command.encodeEnvelope(@fromBackingInt(planet), .{ .sequence = @fromBackingInt(sequence), .provenance = .creator, .actor = null }, .{ .author_settlement = .{
            .owner = @fromBackingInt(@intCast(sequence)),
            .anchor = .{ .planet = @fromBackingInt(planet), .local = if (sequence <= 2) 70 else 5 },
            .population = 2,
            .food = 8,
            .material = 20,
            .economic_bypass = true,
        } }, &bytes[i]);
        routed[i] = encoded;
    }
    var cursor = try command.Cursor.init(&storage, .{ .role = .creator, .civilization = null }, &routed);
    routed[0].planet = @fromBackingInt(4);
    for (&bytes) |*row| @memset(row, 0xFF);
    try finish(&cursor, 7);
    try std.testing.expectEqual(@as(u64, 4), @backingInt(cursor.globalWatermark()));
    try std.testing.expectEqual(cursor.copied_bytes_at_begin, cursor.routed_bytes + 4 * @sizeOf(t.PlanetId));
    try std.testing.expectEqual(@as(usize, 2), cursor.ownedLengths(@fromBackingInt(1)).len);
    try std.testing.expectEqual(@as(usize, 2), cursor.ownedLengths(@fromBackingInt(2)).len);
    for (0..4) |i| try std.testing.expectEqual(@as(u64, @intCast(i + 1)), std.mem.readInt(u64, (try cursor.routed(@intCast(i))).body[0..8], .little));
    try std.testing.expectEqual(@as(u32, 2), @backingInt((try cursor.routed(3)).planet));
    for (storage.metadata().planets[0..2], 0..) |*maybe, i| {
        const p = &maybe.*.?;
        const old = p.activeRoot();
        const prior = try p.metadata(old);
        try p.beginPrepared(@fromBackingInt(0), @fromBackingInt(1), .{ .role = .creator, .civilization = null }, cursor.ownedLengths(@fromBackingInt(@as(u32, @intCast(i)) + 1)), cursor.globalWatermark());
        try phase(p, .prepare, 7);
        try phase(p, .commands, 7);
        for ([_]t.Phase{ .arrivals, .construction, .production, .population, .local_finalize }) |which| try phase(p, which, 7);
        while (true) {
            var budget: t.WorkBudget = .{ .remaining = 7 };
            if (try p.freezePrepared(&budget) != null) break;
        }
        try std.testing.expectEqual(@as(u16, 2), (try p.counts()).accepted);
        try std.testing.expectEqual(@as(u16, 0), (try p.counts()).rejected);
        try std.testing.expectEqualDeep(prior, try p.metadata(old));
    }
}

test "cg/commands/borrowed-snapshot-and-boundaries" {
    var storage = try state.Storage.init(std.testing.allocator, .{ .seed = 2, .planet_count = 4 });
    defer storage.deinit();
    var bodies: [256][1024]u8 = undefined;
    var routed: [256]t.RoutedEnvelope = undefined;
    for (&bodies, 0..) |*body, i| {
        @memset(body, @intCast(i));
        routed[i] = .{ .planet = @fromBackingInt(@as(u32, @intCast(i / 64)) + 1), .body = body };
    }
    const creator: t.SessionPolicy = .{ .role = .creator, .civilization = null };
    const max_copy = try command.Cursor.init(&storage, creator, &routed);
    try std.testing.expectEqual(@as(u32, 263_168), max_copy.copied_bytes_at_begin);
    @memset(&bodies[0], 0xFF);
    try std.testing.expectEqual(@as(u8, 0), storage.metadata().commands[0][0]);
    try std.testing.expectEqual(@as(u8, 255), storage.metadata().commands[255][1023]);
    try setup(&storage);
    const p = &storage.metadata().planets[0].?;
    const root = p.activeRoot();
    const committed = try p.metadata(root);

    var bytes: [1024]u8 = undefined;
    const valid = try command.encodeEnvelope(@fromBackingInt(1), .{ .sequence = @fromBackingInt(1), .provenance = .creator, .actor = null }, .{ .grant = .{
        .settlement = @fromBackingInt(1),
        .food = 1,
        .material = 0,
        .population = 0,
        .economic_bypass = true,
    } }, &bytes);
    const original = bytes;
    const Case = enum { empty, truncated, trailing, exact_max, actor, tag, bypass };
    for ([_]Case{ .empty, .truncated, .trailing, .exact_max, .actor, .tag, .bypass }) |case| {
        bytes = original;
        var size = valid.body.len;
        switch (case) {
            .empty => size = 0,
            .truncated => size -= 1,
            .trailing => {
                bytes[size] = 0;
                size += 1;
            },
            .exact_max => {
                @memset(bytes[size..], 0);
                size = 1024;
            },
            .actor => bytes[9] = 2,
            .tag => bytes[10] = 7,
            .bypass => bytes[size - 1] = 2,
        }
        const one = [_]t.RoutedEnvelope{.{ .planet = @fromBackingInt(1), .body = bytes[0..size] }};
        var cursor = try command.Cursor.init(&storage, creator, &one);
        try std.testing.expectError(error.InvalidEnvelope, finish(&cursor, 1));
        try std.testing.expectEqualDeep(root, p.activeRoot());
        try std.testing.expectEqualDeep(committed, try p.metadata(root));
        try std.testing.expectEqual(@as(u16, 0), p.storage().staged_count);
    }
    const build = try command.encodeEnvelope(@fromBackingInt(1), .{ .sequence = @fromBackingInt(1), .provenance = .creator, .actor = null }, .{ .author_build = .{
        .settlement = @fromBackingInt(1),
        .kind = .house,
        .anchor = .{ .planet = @fromBackingInt(1), .local = 70 },
        .facing = null,
        .economic_bypass = true,
    } }, &bytes);
    bytes[19] = 3;
    var bad_kind = try command.Cursor.init(&storage, creator, &.{.{ .planet = @fromBackingInt(1), .body = bytes[0..build.body.len] }});
    try std.testing.expectError(error.InvalidEnvelope, finish(&bad_kind, 1));
    bytes[19] = 2;
    bytes[32] = 2;
    var bad_optional = try command.Cursor.init(&storage, creator, &.{.{ .planet = @fromBackingInt(1), .body = bytes[0..build.body.len] }});
    try std.testing.expectError(error.InvalidEnvelope, finish(&bad_optional, 1));
    const ship = try command.encodeEnvelope(@fromBackingInt(1), .{ .sequence = @fromBackingInt(1), .provenance = .creator, .actor = null }, .{ .author_ship = .{
        .source = @fromBackingInt(1),
        .destination = @fromBackingInt(2),
        .resource = .food,
        .amount = 1,
        .path = &.{},
        .cargo_source = .mint,
        .economic_bypass = true,
    } }, &bytes);
    bytes[30] = 66;
    var bad_path = try command.Cursor.init(&storage, creator, &.{.{ .planet = @fromBackingInt(1), .body = bytes[0..ship.body.len] }});
    try std.testing.expectError(error.InvalidEnvelope, finish(&bad_path, 1));
    bytes[30] = 0;
    bytes[32] = 2;
    var bad_cargo = try command.Cursor.init(&storage, creator, &.{.{ .planet = @fromBackingInt(1), .body = bytes[0..ship.body.len] }});
    try std.testing.expectError(error.InvalidEnvelope, finish(&bad_cargo, 1));
    try std.testing.expectEqualDeep(committed, try p.metadata(root));
    var unknown_route = try command.Cursor.init(&storage, creator, &.{.{ .planet = @fromBackingInt(5), .body = valid.body }});
    try std.testing.expectError(error.InvalidPlanet, finish(&unknown_route, 1));
    var invalid_policy = try command.Cursor.init(&storage, .{ .role = .ruler, .civilization = @fromBackingInt(5) }, &.{});
    try std.testing.expectError(error.InvalidPolicy, finish(&invalid_policy, 1));
    for (routed[0..65]) |*entry| entry.planet = @fromBackingInt(1);
    var overflowing_route = try command.Cursor.init(&storage, creator, routed[0..65]);
    try std.testing.expectError(error.TooManyCommands, finish(&overflowing_route, 1));
    for (&bodies, 0..) |*body, i| {
        const planet: t.PlanetId = @fromBackingInt(@as(u32, @intCast(i / 64)) + 1);
        routed[i] = try command.encodeEnvelope(planet, .{ .sequence = @fromBackingInt(@as(u64, @intCast(256 - i))), .provenance = .creator, .actor = null }, .{ .grant = .{
            .settlement = @fromBackingInt(99),
            .food = 1,
            .material = 0,
            .population = 0,
            .economic_bypass = true,
        } }, body);
    }
    var full = try command.Cursor.init(&storage, creator, &routed);
    try finish(&full, 4096);
    try std.testing.expectEqual(@as(u32, 32_640), full.comparisons);
    try std.testing.expectEqual(@as(u64, 256), @backingInt(full.globalWatermark()));
    for (1..5) |i| try std.testing.expectEqual(@as(usize, 64), full.ownedLengths(@fromBackingInt(@intCast(i))).len);
    try std.testing.expectEqual(@as(u64, 1), std.mem.readInt(u64, (try full.routed(0)).body[0..8], .little));
}

test "cg/commands/sequence-exhaustion-and-budget-invariance" {
    var storage = try state.Storage.init(std.testing.allocator, .{ .seed = 2, .planet_count = 1 });
    defer storage.deinit();
    try setup(&storage);
    const creator: t.SessionPolicy = .{ .role = .creator, .civilization = null };
    var bytes: [3][1024]u8 = undefined;
    var routed: [3]t.RoutedEnvelope = undefined;
    for ([_]u64{ 3, 1, 2 }, 0..) |sequence, i| routed[i] = try command.encodeEnvelope(@fromBackingInt(1), .{ .sequence = @fromBackingInt(sequence), .provenance = .creator, .actor = null }, .{ .grant = .{
        .settlement = @fromBackingInt(99),
        .food = 1,
        .material = 0,
        .population = 0,
        .economic_bypass = true,
    } }, &bytes[i]);
    var reference = try command.Cursor.init(&storage, creator, &routed);
    try finish(&reference, 4096);
    const expected_work = reference.work_units;
    try std.testing.expectEqual(@as(u64, reference.parse_units + reference.comparisons + 1 + 3 + 3 + 3 * 3), expected_work);
    var expected_order: [3]u8 = undefined;
    @memcpy(&expected_order, storage.metadata().command_order[0..3]);
    for ([_]u16{ 1, 7, 256 }) |allowance| {
        var cursor = try command.Cursor.init(&storage, creator, &routed);
        while (true) {
            var zero: t.WorkBudget = .{ .remaining = 0 };
            const before = cursor.work_units;
            const phase_before = cursor.phase;
            const index_before = cursor.index;
            const sort_before = cursor.sort_index;
            const route_before = cursor.route_offset;
            try std.testing.expectEqual(t.Step.pending, try cursor.advance(&zero));
            try std.testing.expectEqual(before, cursor.work_units);
            try std.testing.expectEqual(phase_before, cursor.phase);
            try std.testing.expectEqual(index_before, cursor.index);
            try std.testing.expectEqual(sort_before, cursor.sort_index);
            try std.testing.expectEqual(route_before, cursor.route_offset);
            var budget: t.WorkBudget = .{ .remaining = allowance };
            if (try cursor.advance(&budget) == .done) break;
        }
        try std.testing.expectEqual(expected_work, cursor.work_units);
        try std.testing.expectEqualSlices(u8, &expected_order, storage.metadata().command_order[0..3]);
        try std.testing.expect(cursor.comparisons <= 32_640);
        try std.testing.expect(cursor.max_parse_bytes_per_unit <= 12);
    }
    var duplicate = routed;
    duplicate[0] = routed[2];
    var dup = try command.Cursor.init(&storage, creator, &duplicate);
    try std.testing.expectError(error.DuplicateSequence, finish(&dup, 7));
    storage.metadata().published_sequence = @fromBackingInt(1);
    var old = try command.Cursor.init(&storage, creator, &routed);
    try std.testing.expectError(error.SequenceAlreadyPublished, finish(&old, 7));
    storage.metadata().published_sequence = @fromBackingInt(std.math.maxInt(u64) - 1);
    const maximum = try command.encodeEnvelope(@fromBackingInt(1), .{ .sequence = @fromBackingInt(std.math.maxInt(u64)), .provenance = .creator, .actor = null }, .{ .grant = .{
        .settlement = @fromBackingInt(99),
        .food = 1,
        .material = 0,
        .population = 0,
        .economic_bypass = true,
    } }, &bytes[0]);
    var once = try command.Cursor.init(&storage, creator, &.{maximum});
    try finish(&once, 1);
    try std.testing.expectEqual(std.math.maxInt(u64), @backingInt(once.globalWatermark()));
    storage.metadata().published_sequence = once.globalWatermark();
    var empty = try command.Cursor.init(&storage, creator, &.{});
    try finish(&empty, 1);
    try std.testing.expectEqual(std.math.maxInt(u64), @backingInt(empty.globalWatermark()));
    var later = try command.Cursor.init(&storage, creator, &.{maximum});
    try std.testing.expectError(error.SequenceAlreadyPublished, finish(&later, 1));
    @memset(bytes[0][0..8], 0);
    storage.metadata().published_sequence = @fromBackingInt(0);
    var zero = try command.Cursor.init(&storage, creator, &.{maximum});
    try std.testing.expectError(error.InvalidEnvelope, finish(&zero, 1));
}

test "cg/commands/zero-boundaries-and-metered-policy-route-inspection" {
    const creator: t.SessionPolicy = .{ .role = .creator, .civilization = null };
    var empty_storage = try state.Storage.init(std.testing.allocator, .{ .seed = 2, .planet_count = 1 });
    defer empty_storage.deinit();
    var empty = try command.Cursor.init(&empty_storage, creator, &.{});
    var zero: t.WorkBudget = .{ .remaining = 0 };
    try std.testing.expectEqual(t.Step.pending, try empty.advance(&zero));
    try std.testing.expectEqual(@as(u64, 0), empty.work_units);
    var one: t.WorkBudget = .{ .remaining = 1 };
    try std.testing.expectEqual(t.Step.done, try empty.advance(&one));
    try std.testing.expectEqual(@as(u64, 1), empty.work_units);

    var bad_policy = try command.Cursor.init(&empty_storage, .{ .role = .ruler, .civilization = null }, &.{});
    zero = .{ .remaining = 0 };
    try std.testing.expectEqual(t.Step.pending, try bad_policy.advance(&zero));
    one = .{ .remaining = 1 };
    try std.testing.expectError(error.InvalidPolicy, bad_policy.advance(&one));
    try std.testing.expectEqual(@as(u16, 0), one.remaining);

    var bytes: [1024]u8 = undefined;
    const valid = try command.encodeEnvelope(@fromBackingInt(1), .{ .sequence = @fromBackingInt(1), .provenance = .creator, .actor = null }, .{ .grant = .{
        .settlement = @fromBackingInt(99),
        .food = 1,
        .material = 0,
        .population = 0,
        .economic_bypass = true,
    } }, &bytes);
    var bad_route = try command.Cursor.init(&empty_storage, creator, &.{.{ .planet = @fromBackingInt(2), .body = valid.body }});
    zero = .{ .remaining = 0 };
    try std.testing.expectEqual(t.Step.pending, try bad_route.advance(&zero));
    one = .{ .remaining = 1 };
    try std.testing.expectEqual(t.Step.pending, try bad_route.advance(&one));
    zero = .{ .remaining = 0 };
    try std.testing.expectEqual(t.Step.pending, try bad_route.advance(&zero));
    one = .{ .remaining = 1 };
    try std.testing.expectError(error.InvalidPlanet, bad_route.advance(&one));
    try std.testing.expectEqual(@as(u64, 2), bad_route.work_units);
    var no_world = try command.Cursor.init(&empty_storage, creator, &.{valid});
    while (!(no_world.phase == .sequence and no_world.index == no_world.count)) {
        one = .{ .remaining = 1 };
        try std.testing.expectEqual(t.Step.pending, try no_world.advance(&one));
    }
    zero = .{ .remaining = 0 };
    const before = no_world.work_units;
    try std.testing.expectEqual(t.Step.pending, try no_world.advance(&zero));
    try std.testing.expectEqual(before, no_world.work_units);
    one = .{ .remaining = 1 };
    try std.testing.expectError(error.NotReady, no_world.advance(&one));
    try std.testing.expectEqual(before + 1, no_world.work_units);

    var busy_storage = try state.Storage.init(std.testing.allocator, .{ .seed = 2, .planet_count = 1 });
    defer busy_storage.deinit();
    try setup(&busy_storage);
    try busy_storage.metadata().planets[0].?.beginPrepared(@fromBackingInt(0), @fromBackingInt(1), creator, &.{}, @fromBackingInt(0));
    var busy = try command.Cursor.init(&busy_storage, creator, &.{valid});
    while (!(busy.phase == .sequence and busy.index == busy.count)) {
        one = .{ .remaining = 1 };
        try std.testing.expectEqual(t.Step.pending, try busy.advance(&one));
    }
    zero = .{ .remaining = 0 };
    const busy_work = busy.work_units;
    try std.testing.expectEqual(t.Step.pending, try busy.advance(&zero));
    try std.testing.expectEqual(busy_work, busy.work_units);
    one = .{ .remaining = 1 };
    try std.testing.expectError(error.Busy, busy.advance(&one));
    try std.testing.expectEqual(busy_work + 1, busy.work_units);
}

fn freightCase(comptime variant: enum { mixed, foreign_path, same_owner }) !void {
    var storage = try state.Storage.init(std.testing.allocator, .{ .seed = 2, .planet_count = 1, .planets = .{ .{ .side = 4 }, .{}, .{}, .{} } });
    defer storage.deinit();
    try setup(&storage);
    const creator: t.SessionPolicy = .{ .role = .creator, .civilization = null };
    const anchors = if (variant == .foreign_path) [_]u64{ 70, 71, 5 } else [_]u64{ 70, 5 };
    var bytes: [4][1024]u8 = undefined;
    var routed: [4]t.RoutedEnvelope = undefined;
    for (anchors, 0..) |anchor, i| {
        const owner: u32 = if (i == 1 and variant != .same_owner) 2 else 1;
        routed[i] = try command.encodeEnvelope(@fromBackingInt(1), .{ .sequence = @fromBackingInt(@as(u64, @intCast(i)) + 1), .provenance = .creator, .actor = null }, .{ .author_settlement = .{
            .owner = @fromBackingInt(owner),
            .anchor = .{ .planet = @fromBackingInt(1), .local = anchor },
            .population = 2,
            .food = 8,
            .material = 20,
            .economic_bypass = true,
        } }, &bytes[i]);
    }
    const path = [_]t.CellId{ .{ .planet = @fromBackingInt(1), .local = 70 }, .{ .planet = @fromBackingInt(1), .local = 71 }, .{ .planet = @fromBackingInt(1), .local = 4 }, .{ .planet = @fromBackingInt(1), .local = 5 } };
    const ship_index = anchors.len;
    routed[ship_index] = try command.encodeEnvelope(@fromBackingInt(1), .{ .sequence = @fromBackingInt(@as(u64, @intCast(ship_index)) + 1), .provenance = .creator, .actor = null }, .{ .author_ship = .{
        .source = @fromBackingInt(1),
        .destination = @fromBackingInt(if (variant == .foreign_path) 3 else 2),
        .resource = .food,
        .amount = 1,
        .path = &path,
        .cargo_source = .transfer,
        .economic_bypass = true,
    } }, &bytes[ship_index]);
    var cursor = try command.Cursor.init(&storage, creator, routed[0 .. ship_index + 1]);
    try finish(&cursor, 7);
    const p = &storage.metadata().planets[0].?;
    try prepared(p, 0, 1, creator, &cursor, 7);
    const receipt = p.storage().logicalReceipt(@intCast(ship_index + 1));
    switch (variant) {
        .mixed => try std.testing.expectEqual(t.RejectReason.WrongOwner, receipt.event.rejected.reason),
        .foreign_path => {
            try std.testing.expectEqual(t.RejectReason.ForeignPathCell, receipt.event.rejected.reason);
            try std.testing.expectEqual(@as(?u16, 1), receipt.event.rejected.offending_index);
        },
        .same_owner => {
            try std.testing.expectEqual(@as(u16, 3), (try p.counts()).accepted);
            try std.testing.expectEqual(@as(u16, 0), (try p.counts()).rejected);
            try std.testing.expectEqual(@as(u16, 1), receipt.event.accepted.effect.author_ship.transferred);
        },
    }
}

test "cg/commands/creator-freight-mixed-foreign-and-same-owner" {
    try freightCase(.mixed);
    try freightCase(.foreign_path);
    try freightCase(.same_owner);
}

test "cg/commands/entity-id-exhaustion-rejects-and-later-command-applies" {
    var storage = try state.Storage.init(std.testing.allocator, .{ .seed = 2, .planet_count = 1 });
    defer storage.deinit();
    try setup(&storage);
    const p = &storage.metadata().planets[0].?;
    const creator: t.SessionPolicy = .{ .role = .creator, .civilization = null };
    var bytes: [4][1024]u8 = undefined;
    const first = [_]t.RoutedEnvelope{
        try command.encodeEnvelope(@fromBackingInt(1), .{ .sequence = @fromBackingInt(1), .provenance = .creator, .actor = null }, .{ .author_settlement = .{
            .owner = @fromBackingInt(1),
            .anchor = .{ .planet = @fromBackingInt(1), .local = 70 },
            .population = 2,
            .food = 8,
            .material = 20,
            .economic_bypass = true,
        } }, &bytes[0]),
        try command.encodeEnvelope(@fromBackingInt(1), .{ .sequence = @fromBackingInt(2), .provenance = .creator, .actor = null }, .{ .author_settlement = .{
            .owner = @fromBackingInt(1),
            .anchor = .{ .planet = @fromBackingInt(1), .local = 5 },
            .population = 2,
            .food = 8,
            .material = 20,
            .economic_bypass = true,
        } }, &bytes[1]),
    };
    var initial = try command.Cursor.init(&storage, creator, &first);
    try finish(&initial, 1);
    const old = p.activeRoot();
    try prepared(p, 0, 1, creator, &initial, 7);
    var one: t.WorkBudget = .{ .remaining = 1 };
    try std.testing.expectEqual(t.Step.done, try p.installHidden(&one));
    while (true) {
        var budget: t.WorkBudget = .{ .remaining = 256 };
        if (try p.retire(old, &budget) == .done) break;
    }
    var meta = p.storage().rootMeta(p.storage().active);
    meta.next_settlement = std.math.maxInt(u64);
    p.storage().writeRootMeta(p.storage().active, meta);
    storage.metadata().published_sequence = initial.globalWatermark();
    const path = [_]t.CellId{ .{ .planet = @fromBackingInt(1), .local = 70 }, .{ .planet = @fromBackingInt(1), .local = 71 }, .{ .planet = @fromBackingInt(2), .local = 4 }, .{ .planet = @fromBackingInt(1), .local = 5 } };
    const batch = [_]t.RoutedEnvelope{
        try command.encodeEnvelope(@fromBackingInt(1), .{ .sequence = @fromBackingInt(3), .provenance = .creator, .actor = null }, .{ .author_settlement = .{
            .owner = @fromBackingInt(2),
            .anchor = .{ .planet = @fromBackingInt(1), .local = 69 },
            .population = 0,
            .food = 0,
            .material = 0,
            .economic_bypass = true,
        } }, &bytes[0]),
        try command.encodeEnvelope(@fromBackingInt(1), .{ .sequence = @fromBackingInt(4), .provenance = .creator, .actor = null }, .{ .grant = .{
            .settlement = @fromBackingInt(1),
            .food = 1,
            .material = 0,
            .population = 0,
            .economic_bypass = true,
        } }, &bytes[1]),
        try command.encodeEnvelope(@fromBackingInt(1), .{ .sequence = @fromBackingInt(5), .provenance = .creator, .actor = null }, .{ .author_build = .{
            .settlement = @fromBackingInt(1),
            .kind = .house,
            .anchor = .{ .planet = @fromBackingInt(2), .local = 71 },
            .facing = null,
            .economic_bypass = true,
        } }, &bytes[2]),
        try command.encodeEnvelope(@fromBackingInt(1), .{ .sequence = @fromBackingInt(6), .provenance = .creator, .actor = null }, .{ .author_ship = .{
            .source = @fromBackingInt(1),
            .destination = @fromBackingInt(2),
            .resource = .food,
            .amount = 1,
            .path = &path,
            .cargo_source = .mint,
            .economic_bypass = true,
        } }, &bytes[3]),
    };
    var cursor = try command.Cursor.init(&storage, creator, &batch);
    try finish(&cursor, 1);
    try prepared(p, 1, 2, creator, &cursor, 7);
    try std.testing.expectEqual(@as(u16, 1), (try p.counts()).accepted);
    try std.testing.expectEqual(@as(u16, 3), (try p.counts()).rejected);
    try std.testing.expectEqual(t.RejectReason.IdExhausted, p.storage().logicalReceipt(1).event.rejected.reason);
    try std.testing.expectEqual(t.RejectReason.InvalidPlanet, p.storage().logicalReceipt(3).event.rejected.reason);
    try std.testing.expectEqual(t.RejectReason.InvalidPlanet, p.storage().logicalReceipt(4).event.rejected.reason);
    try std.testing.expectEqual(std.math.maxInt(u64), p.storage().rootMeta(p.storage().active).next_settlement);
    try std.testing.expectEqual(@as(u32, 1), p.storage().logicalReceipt(2).event.accepted.effect.grant.food_minted);

    const second_old = p.activeRoot();
    one = .{ .remaining = 1 };
    try std.testing.expectEqual(t.Step.done, try p.installHidden(&one));
    while (true) {
        var budget: t.WorkBudget = .{ .remaining = 256 };
        if (try p.retire(second_old, &budget) == .done) break;
    }
    storage.metadata().published_sequence = cursor.globalWatermark();
    const ruler: t.SessionPolicy = .{ .role = .ruler, .civilization = @fromBackingInt(2) };
    const wrong = [_]t.RoutedEnvelope{try command.encodeEnvelope(@fromBackingInt(1), .{ .sequence = @fromBackingInt(7), .provenance = .ruler, .actor = @fromBackingInt(2) }, .{ .build = .{
        .settlement = @fromBackingInt(1),
        .kind = .house,
        .anchor = .{ .planet = @fromBackingInt(1), .local = 72 },
        .facing = null,
    } }, &bytes[0])};
    var ownership = try command.Cursor.init(&storage, ruler, &wrong);
    try finish(&ownership, 7);
    try prepared(p, 2, 3, ruler, &ownership, 7);
    try std.testing.expectEqual(t.RejectReason.WrongOwner, p.storage().logicalReceipt(1).event.rejected.reason);
}
