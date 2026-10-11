const std = @import("std");
const pe = @import("root.zig");
const helpers = @import("jobs_tests.zig");

test "pe/replay/empty-handoff preserves trusted policy rejected input and empty tick" {
    var source = try helpers.create(.{ .seed = 2, .side = 4 });
    defer source.deinit();
    var bytes: [1024]u8 = undefined;
    const creator = try pe.SessionPolicy.init(.creator, null);
    const ruler = try pe.SessionPolicy.init(.ruler, pe.civilization_id);
    _ = try helpers.tick(&source, creator, &.{try helpers.author(&bytes, 1)}, 7);
    const grant = try pe.encodeEnvelope(.{ .sequence = @fromBackingInt(2), .provenance = .creator, .actor = null }, .{ .grant = .{ .settlement = @fromBackingInt(1), .food = 1, .material = 0, .population = 0, .economic_bypass = true } }, &bytes);
    const rejected = try helpers.tick(&source, ruler, &.{grant}, 7);
    try std.testing.expectEqual(@as(u16, 1), rejected.rejected);
    _ = try helpers.tick(&source, ruler, &.{}, 7);
    try std.testing.expectEqual(@as(u32, 0), (try source.settlement(@fromBackingInt(1))).food);
    var target = try helpers.create(.{ .seed = 2, .side = 4 });
    defer target.deinit();
    var replay = pe.replay.Cursor.init(&source);
    while (true) {
        var zero = pe.WorkBudget{ .remaining = 0 };
        try std.testing.expectEqual(pe.Step.pending, try replay.advance(&target, &zero));
        var one = pe.WorkBudget{ .remaining = 1 };
        if (try replay.advance(&target, &one) == .done) break;
    }
    try std.testing.expectEqualDeep(source.summary(), target.summary());
    try std.testing.expectEqualDeep(try source.settlement(@fromBackingInt(1)), try target.settlement(@fromBackingInt(1)));
    var a: [1]pe.ReceiptView = undefined;
    var b: [1]pe.ReceiptView = undefined;
    for (0..source.stats().history_used) |index| {
        _ = try source.receipts(source.summary().revision, @intCast(index), &a);
        _ = try target.receipts(target.summary().revision, @intCast(index), &b);
        try std.testing.expectEqual(a[0].len, b[0].len);
        try std.testing.expectEqualSlices(u8, a[0].bytes[0..a[0].len], b[0].bytes[0..b[0].len]);
    }
    var stale = pe.replay.Cursor.init(&source);
    _ = try helpers.tick(&source, ruler, &.{}, 256);
    var budget = pe.WorkBudget{ .remaining = 1 };
    try std.testing.expectError(error.StaleView, stale.advance(&target, &budget));
}

const Snapshot = struct { result: pe.stories.StoryResult, bytes: std.ArrayList(u8) };
fn storySnapshot(story: pe.stories.Story, units: u16, zeroes: bool) !Snapshot {
    var runner = try pe.stories.Runner.init(std.testing.allocator, story);
    defer runner.deinit();
    var bytes = std.ArrayList(u8).empty;
    errdefer bytes.deinit(std.testing.allocator);
    var accepted: [4]u16 = @splat(0);
    var tick_count: u64 = 0;
    while (true) {
        if (zeroes) {
            var zero = pe.WorkBudget{ .remaining = 0 };
            const result = try runner.poll(&zero);
            try std.testing.expect(result == .pending or result == .complete);
        }
        var budget = pe.WorkBudget{ .remaining = units };
        switch (try runner.poll(&budget)) {
            .pending => {},
            .awaiting_observer => return error.MissingPublication,
            .published => |observation| {
                tick_count += 1;
                const a = observation.a.?;
                const freight_tick = @backingInt(observation.summary.tick);
                const expected_freight: pe.FreightView = if (story == .combined or story == .creator) switch (freight_tick) {
                    1 => .{ .transit = .{ .planet = pe.planet_id, .local = 70 } },
                    2 => .{ .transit = .{ .planet = pe.planet_id, .local = 71 } },
                    3 => .{ .transit = .{ .planet = pe.planet_id, .local = 4 } },
                    4 => .{ .delivered = .{ .planet = pe.planet_id, .local = 5 } },
                    else => .none,
                } else .none;
                try std.testing.expectEqualDeep(expected_freight, observation.freight);
                const tick_number = @backingInt(observation.summary.tick);
                switch (story) {
                    .combined => {
                        const tuples = [_][5]u32{ .{ 4, 10, 2, 4, 0 }, .{ 6, 12, 2, 2, 0 }, .{ 8, 14, 2, 0, 0 }, .{ 10, 16, 2, 2, 0 }, .{ 12, 18, 2, 0, 0 }, .{ 14, 20, 2, 0, 1 }, .{ 16, 22, 2, 0, 2 }, .{ 18, 24, 1, 0, 0 } };
                        const b = observation.b.?;
                        try std.testing.expectEqualDeep(tuples[tick_number - 1], [5]u32{ a.food, a.material, b.population, b.food, b.hunger });
                        try std.testing.expectEqual(@as(u8, 4), a.population);
                        if (tick_number == 8) {
                            const final_a = a;
                            const final_b = b;
                            try std.testing.expectEqual(@as(u32, 18), final_a.food);
                            try std.testing.expectEqual(@as(u32, 24), final_a.material);
                            try std.testing.expectEqual(@as(u8, 1), final_b.population);
                        }
                    },
                    .creator => {
                        const tuples = [_][3]u32{ .{ 2, 2, 4 }, .{ 4, 4, 2 }, .{ 6, 6, 0 }, .{ 8, 8, 2 } };
                        try std.testing.expectEqualDeep(tuples[tick_number - 1], [3]u32{ a.food, a.material, observation.b.?.food });
                        try std.testing.expectEqual(@as(u8, 8), a.housing);
                    },
                    .ruler_start => {
                        const tuples = [_][2]u32{ .{ 8, 20 }, .{ 4, 14 }, .{ 6, 14 } };
                        try std.testing.expectEqualDeep(tuples[tick_number - 1], [2]u32{ a.food, a.material });
                        try std.testing.expectEqual(@as(u8, @intCast(tick_number)), a.fed_streak);
                    },
                    .policy_replay => {
                        const foods = if (observation.case_index % 2 == 0) [3]u32{ 8, 5, 1 } else [3]u32{ 8, 4, 0 };
                        try std.testing.expectEqual(foods[tick_number - 1], a.food);
                    },
                }
                accepted[observation.case_index] += observation.summary.accepted;
                if (story == .creator) {
                    const instance = &runner.authorities[observation.case_index].?;
                    for (0..3) |slot| {
                        const building = (try instance.storage.readRecord(instance.storage.active, .building, @intCast(slot))).building;
                        try std.testing.expectEqual(@as(u8, 0), building.remaining_ticks);
                    }
                    if (tick_number == 1) try std.testing.expectEqual(@as(u32, 4), observation.b.?.incoming_food);
                }
                var rows: [4]pe.ReceiptView = undefined;
                var cursor: pe.ReceiptCursor = 0;
                while (true) {
                    const page = try runner.receipts(observation.summary.revision, cursor, &rows);
                    for (rows[0..page.written]) |row| {
                        try bytes.appendSlice(std.testing.allocator, row.bytes[0..row.len]);
                        if (integer(u64, &row.bytes, 17) == tick_number) try verifyReceipt(story, observation, row.bytes[0..row.len]);
                    }
                    cursor = page.next orelse break;
                }
                var pause = pe.WorkBudget{ .remaining = units };
                try std.testing.expect(try runner.poll(&pause) == .awaiting_observer);
                try std.testing.expectEqual(units, pause.remaining);
                try std.testing.expectError(error.StaleView, runner.acknowledge(@fromBackingInt(99)));
                try runner.acknowledge(observation.summary.revision);
            },
            .complete => |result| {
                try std.testing.expectEqual(tick_count, result.ticks);
                switch (story) {
                    .combined => try std.testing.expectEqual(@as(u16, 5), accepted[0]),
                    .creator => try std.testing.expectEqual(@as(u16, 6), accepted[0]),
                    .ruler_start => try std.testing.expectEqual(@as(u16, 2), accepted[0]),
                    .policy_replay => try std.testing.expectEqualDeep([4]u16{ 2, 1, 2, 1 }, accepted),
                }
                // Every schedule is independently replayed at a fixed budget;
                // Cursor compares every claim/entity/route and root metadata,
                // as well as complete receipt bytes. This ties transcript
                // parity across schedules to full logical-state parity.
                const originals: usize = if (story == .policy_replay) 2 else 1;
                for (0..originals) |index| {
                    const source = &runner.authorities[index].?;
                    var target = try helpers.create(source.storage.config);
                    defer target.deinit();
                    var cursor = pe.replay.Cursor.init(source);
                    while (true) {
                        var fixed = pe.WorkBudget{ .remaining = 256 };
                        if (try cursor.advance(&target, &fixed) == .done) break;
                    }
                }
                return .{ .result = result, .bytes = bytes };
            },
        }
    }
}
fn schedules(story: pe.stories.Story) !void {
    var baseline = try storySnapshot(story, 256, false);
    defer baseline.bytes.deinit(std.testing.allocator);
    for ([_]u16{ 1, 7, 256, 4096 }) |units| {
        for ([_]bool{ false, true }) |zeroes| {
            var actual = try storySnapshot(story, units, zeroes);
            defer actual.bytes.deinit(std.testing.allocator);
            try std.testing.expectEqualDeep(baseline.result, actual.result);
            try std.testing.expectEqualSlices(u8, baseline.bytes.items, actual.bytes.items);
        }
    }
}
test "pe/stories/combined" {
    try schedules(.combined);
}
test "pe/stories/creator" {
    try schedules(.creator);
}
test "pe/stories/ruler-start" {
    try schedules(.ruler_start);
}
test "pe/stories/policy-replay" {
    try schedules(.policy_replay);
}

test "pe/replay/bounded-copy abandonment before admission leaves target fresh" {
    var source = try helpers.create(.{ .seed = 2, .side = 4 });
    defer source.deinit();
    const creator = try pe.SessionPolicy.init(.creator, null);
    var path: [65]pe.CellId = @splat(.{ .planet = pe.planet_id, .local = 70 });
    var bytes: [1024]u8 = undefined;
    const envelope = try pe.encodeEnvelope(.{ .sequence = @fromBackingInt(1), .provenance = .creator, .actor = null }, .{ .author_ship = .{ .source = @fromBackingInt(1), .destination = @fromBackingInt(2), .resource = .food, .amount = 1, .path = &path, .cargo_source = .mint, .economic_bypass = true } }, &bytes);
    _ = try helpers.tick(&source, creator, &.{envelope}, 256);
    var target = try helpers.create(.{ .seed = 2, .side = 4 });
    defer target.deinit();
    const prior = target.summary();
    var cursor = pe.replay.Cursor.init(&source);
    while (cursor.step != .copy) {
        var one = pe.WorkBudget{ .remaining = 1 };
        try std.testing.expectEqual(pe.Step.pending, try cursor.advance(&target, &one));
    }
    @memset(&target.storage.reservation.envelopes[0], 0xa5);
    var one = pe.WorkBudget{ .remaining = 1 };
    try std.testing.expectEqual(pe.Step.pending, try cursor.advance(&target, &one));
    try std.testing.expectEqual(@as(usize, 256), cursor.copy_offset);
    try std.testing.expectEqualSlices(u8, envelope.bytes[0..256], target.storage.reservation.envelopes[0][0..256]);
    for (target.storage.reservation.envelopes[0][256..]) |byte| try std.testing.expectEqual(@as(u8, 0xa5), byte);
    try std.testing.expectEqualDeep(prior, target.summary());
    try std.testing.expect(target.lane == null);
    // Dropping an extraction cursor has no authority job or candidate to drain.
    // A host can now submit a different request and the abandoned tail is inert.
    _ = try helpers.tick(&target, creator, &.{try helpers.author(&bytes, 1)}, 7);
    try std.testing.expectEqual(@as(u32, 8), (try target.settlement(@fromBackingInt(1))).food);
    var replay_target = try helpers.create(.{ .seed = 2, .side = 4 });
    defer replay_target.deinit();
    cursor = pe.replay.Cursor.init(&source);
    while (true) {
        one.remaining = 1;
        if (try cursor.advance(&replay_target, &one) == .done) break;
    }
    try std.testing.expectEqualDeep(source.summary(), replay_target.summary());
}

test "pe/stories/init failures release earlier reservations and terminal errors drain" {
    for (0..4) |index| {
        var failing = std.testing.FailingAllocator.init(std.testing.allocator, .{ .fail_index = index });
        try std.testing.expectError(error.OutOfMemory, pe.stories.Runner.init(failing.allocator(), .policy_replay));
        try std.testing.expectEqual(failing.allocated_bytes, failing.freed_bytes);
    }
    var runner = try pe.stories.Runner.init(std.testing.allocator, .combined);
    defer runner.deinit();
    // Exhaustion is reached through normal finalization, after economic writes.
    runner.builders[0].?.storage.?.config.capacities.history_receipts = 1;
    var observed_error = false;
    while (!observed_error) {
        var budget = pe.WorkBudget{ .remaining = 7 };
        const progress = runner.poll(&budget) catch |err| {
            try std.testing.expectEqual(error.HistoryFull, err);
            try std.testing.expect(runner.authorities[0].?.lane == null);
            try std.testing.expectEqual(@as(u64, 0), @backingInt(runner.authorities[0].?.summary().revision));
            observed_error = true;
            continue;
        };
        try std.testing.expect(progress == .pending);
    }
}

fn integer(comptime T: type, bytes: []const u8, offset: usize) T {
    return std.mem.readInt(T, bytes[offset..][0..@sizeOf(T)], .little);
}
fn verifyReceipt(story: pe.stories.Story, observation: pe.stories.StoryTick, bytes: []const u8) !void {
    const tick_number = @backingInt(observation.summary.tick);
    switch (bytes[16]) {
        1 => {
            try std.testing.expectEqual(tick_number, integer(u64, bytes, 29));
            try std.testing.expectEqual(tick_number - 1, integer(u64, bytes, 37));
            try std.testing.expectEqual(@as(u8, if (tick_number == 1) 0 else 1), bytes[45]);
            const role: u8 = switch (story) {
                .combined => 2,
                .creator => 0,
                .ruler_start => if (tick_number == 1) 0 else 1,
                .policy_replay => if (tick_number == 1 or observation.case_index % 2 == 0) 0 else 1,
            };
            const offset: usize = if (tick_number == 1) 46 else 46 + (if (bytes[47] == 1) @as(usize, 6) else 2);
            try std.testing.expectEqual(role, bytes[offset]);
            if (tick_number == 2 and story == .ruler_start) try std.testing.expectEqual(@as(u8, 0), bytes[46]);
            const count: u16 = if (tick_number == 1) switch (story) {
                .combined => 7,
                .creator => 6,
                else => 1,
            } else if (tick_number == 2 and (story == .ruler_start or story == .policy_replay)) 1 else 0;
            try std.testing.expectEqual(count, integer(u16, bytes, bytes.len - 2));
        },
        2, 3 => {
            const sequence = integer(u64, bytes, 29);
            const length = integer(u16, bytes, 37);
            const effect = 39 + @as(usize, length);
            const envelope = bytes[39..effect];
            try std.testing.expectEqual(sequence, integer(u64, envelope, 0));
            try std.testing.expectEqual(bytes[27], envelope[8]);
            const tag_index: usize = if (envelope[9] == 1) 14 else 10;
            if (bytes[16] == 3) {
                const reason: u16 = if (story == .combined) (if (sequence == 6) 18 else 8) else 1;
                try std.testing.expectEqual(reason, integer(u16, bytes, effect));
                if (story == .combined) try std.testing.expect(sequence == 6 or sequence == 7) else try std.testing.expectEqual(@as(u64, 2), sequence);
                return;
            }
            if (story == .creator) try std.testing.expectEqual(@as(u8, 0), bytes[27]);
            switch (envelope[tag_index]) {
                0 => {
                    if (sequence == 1) {
                        try std.testing.expectEqual(@as(u32, if (story == .creator) 0 else 12), integer(u32, bytes, effect + 8));
                        try std.testing.expectEqual(@as(u32, if (story == .creator) 0 else 20), integer(u32, bytes, effect + 12));
                    }
                },
                2, 5 => {
                    const cells = integer(u16, bytes, effect + 8);
                    const kind = envelope[tag_index + 9];
                    if (kind == 0) {
                        try std.testing.expectEqual(@as(u16, 2), cells);
                        try std.testing.expectEqual(@as(u64, 71), integer(u64, bytes, effect + 14));
                        try std.testing.expectEqual(@as(u64, 4), integer(u64, bytes, effect + 26));
                    } else if (kind == 1) try std.testing.expectEqual(@as(u64, 74), integer(u64, bytes, effect + 14));
                    const tail = effect + 10 + @as(usize, cells) * 12;
                    try std.testing.expectEqual(@as(u32, switch (kind) {
                        0 => 6,
                        1 => 4,
                        2 => 8,
                        else => unreachable,
                    }), integer(u32, bytes, tail));
                    try std.testing.expectEqual(@as(u8, 2), bytes[tail + 4]);
                },
                3 => {
                    try std.testing.expectEqual(@as(u16, 0), integer(u16, bytes, effect + 8));
                    try std.testing.expectEqual(@as(u16, 4), integer(u16, bytes, effect + 10));
                    try std.testing.expectEqual(@as(u64, 4), integer(u64, bytes, effect + 20));
                    try std.testing.expectEqual(@as(u16, 4), integer(u16, bytes, effect + 28));
                },
                6 => try std.testing.expectEqual(@as(u64, 4), integer(u64, bytes, effect + 20)),
                else => {},
            }
        },
        4 => {
            try std.testing.expectEqual(@as(u64, 4), tick_number);
            try std.testing.expectEqual(@as(u16, 4), integer(u16, bytes, 54));
            try std.testing.expectEqual(@as(u32, 0), integer(u32, bytes, 60));
        },
        10 => {
            const watermark: u64 = switch (story) {
                .combined => 7,
                .creator => 6,
                .ruler_start, .policy_replay => if (tick_number == 1) 1 else 2,
            };
            try std.testing.expectEqual(watermark, integer(u64, bytes, 37));
            try std.testing.expectEqual(observation.summary.accepted, integer(u16, bytes, 45));
            try std.testing.expectEqual(observation.summary.rejected, integer(u16, bytes, 47));
            if (story == .combined and tick_number == 1) try std.testing.expectEqual(@as(u16, 2), integer(u16, bytes, 47));
        },
        else => {},
    }
}

fn drainStoryForTest(runner: *pe.stories.Runner) void {
    while (runner.mode != .complete) {
        var budget = pe.WorkBudget{ .remaining = 4096 };
        switch (runner.poll(&budget) catch unreachable) {
            .published => |value| runner.acknowledge(value.summary.revision) catch unreachable,
            .awaiting_observer => runner.acknowledge(runner.pending_summary.?.revision) catch unreachable,
            .pending, .complete => {},
        }
    }
    runner.deinit();
}

test "pe/stories/observation budget one cannot read both settlements" {
    var runner = try pe.stories.Runner.init(std.testing.allocator, .combined);
    defer drainStoryForTest(&runner);
    while (runner.mode != .observe) {
        var budget = pe.WorkBudget{ .remaining = 1 };
        try std.testing.expect(try runner.poll(&budget) == .pending);
    }
    const instance = &runner.authorities[0].?;
    const before = instance.stats().work_units;
    var one = pe.WorkBudget{ .remaining = 1 };
    // Reading A must not also scan A+B and publish under this one unit.
    try std.testing.expect(try runner.poll(&one) == .pending);
    try std.testing.expectEqual(@as(u16, 0), one.remaining);
    try std.testing.expectEqual(before + 1, instance.stats().work_units);
    try std.testing.expectEqual(@as(u16, 1), runner.observation_index);
    try std.testing.expectEqual(@as(u64, 1), @backingInt(runner.observation_a.?.id));
    try std.testing.expect(runner.observation_b == null);
    const cached_a = runner.observation_a;
    var zero = pe.WorkBudget{ .remaining = 0 };
    try std.testing.expect(try runner.poll(&zero) == .pending);
    try std.testing.expectEqual(before + 1, instance.stats().work_units);
    try std.testing.expectEqual(@as(u16, 1), runner.observation_index);
    try std.testing.expectEqualDeep(cached_a, runner.observation_a);
    try std.testing.expect(runner.observation_b == null);
    one.remaining = 1;
    try std.testing.expect(try runner.poll(&one) == .pending);
    try std.testing.expectEqual(@as(u16, 2), runner.observation_index);
    try std.testing.expect(runner.observed_shipment == null);
    one.remaining = 1;
    try std.testing.expect(try runner.poll(&one) == .pending);
    try std.testing.expect(runner.observed_shipment != null);
    try std.testing.expect(runner.observation_freight == .none);
    const cached_shipment = runner.observed_shipment;
    try std.testing.expect(try runner.poll(&zero) == .pending);
    try std.testing.expectEqualDeep(cached_shipment, runner.observed_shipment);
    one.remaining = 1;
    try std.testing.expect(try runner.poll(&one) == .pending);
    try std.testing.expectEqual(@as(u64, 70), runner.observation_freight.transit.local);
    try std.testing.expect(runner.arrival == null);
    one.remaining = 1;
    try std.testing.expect(try runner.poll(&one) == .pending);
    try std.testing.expectEqual(@as(u64, 5), runner.arrival.?.cell.local);
    one.remaining = 1;
    const observation = (try runner.poll(&one)).published;
    try std.testing.expectEqual(@as(u64, 1), @backingInt(observation.summary.tick));
    try std.testing.expectEqual(@as(u32, 4), observation.a.?.food);
    try std.testing.expectEqual(@as(u32, 4), observation.b.?.food);
    try std.testing.expectEqual(before + 6, instance.stats().work_units);
    try std.testing.expectEqual(@as(u16, 2), runner.observation_index);
    try std.testing.expectEqual(@as(u64, 2), @backingInt(runner.observation_b.?.id));
    one.remaining = 1;
    try std.testing.expect(try runner.poll(&one) == .awaiting_observer);
    try std.testing.expectEqual(@as(u16, 1), one.remaining);
    try runner.acknowledge(observation.summary.revision);
}
