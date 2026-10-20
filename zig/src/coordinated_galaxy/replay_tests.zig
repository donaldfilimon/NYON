const std = @import("std");
const api = @import("root.zig");
const t = @import("types.zig");
const view = @import("view.zig");
const command = @import("command.zig");
const replay = @import("replay.zig");
const creator: t.SessionPolicy = .{ .role = .creator, .civilization = null };
const config: t.Config = .{ .seed = 2, .planets = .{ .{ .side = 4 }, .{ .side = 4 }, .{}, .{} } };
fn create(cfg: t.Config) !api.Galaxy {
    var b = try api.beginCreate(std.testing.allocator, cfg);
    for (0..500000) |_| {
        var budget: t.WorkBudget = .{ .remaining = 4096 };
        switch (api.pollCreate(&b, &budget)) {
            .ready => return api.takeReady(&b),
            .pending => {},
            else => return error.CreateFailed,
        }
    }
    return error.Timeout;
}
fn tick(g: *api.Galaxy, policy: t.SessionPolicy, inputs: []const t.RoutedEnvelope) !void {
    const v = try api.acquireView(g);
    const s = try api.summary(g, v);
    try api.releaseView(g, v);
    const id = try api.beginTick(g, s.revision, @fromBackingInt(@backingInt(s.tick) + 1), policy, inputs);
    for (0..500000) |_| {
        var b: t.WorkBudget = .{ .remaining = 4096 };
        switch (api.poll(g, id, &b)) {
            .idle => return,
            .pending, .published => {},
            else => return error.TickFailed,
        }
    }
    return error.Timeout;
}
fn finish(c: *replay.Cursor, units: u16) !void {
    for (0..2000000) |_| {
        var zero: t.WorkBudget = .{ .remaining = 0 };
        try std.testing.expect(replay.poll(c, &zero) == .pending);
        var b: t.WorkBudget = .{ .remaining = units };
        switch (replay.poll(c, &b)) {
            .complete => return,
            .pending => {},
            .failed => |e| return e,
            .canceled => return error.Canceled,
        }
    }
    return error.Timeout;
}
fn rejected(c: *replay.Cursor) !struct { cause: t.Failure } {
    for (0..500000) |_| {
        var b: t.WorkBudget = .{ .remaining = 4096 };
        switch (replay.poll(c, &b)) {
            .failed => |e| return .{ .cause = e },
            .pending => {},
            else => return error.UnexpectedReplay,
        }
    }
    return error.Timeout;
}
test "cg/replay/all-inputs-and-policy" {
    var g = try create(config);
    defer api.deinit(&g) catch unreachable;
    var bytes: [2][256]u8 = undefined;
    var inputs: [2]t.RoutedEnvelope = undefined;
    for (0..2) |i| {
        const p: t.PlanetId = @fromBackingInt(@as(u32, @intCast(i)) + 1);
        inputs[i] = try command.encodeEnvelope(p, .{ .sequence = @fromBackingInt(@as(u64, @intCast(i)) + 1), .provenance = .creator, .actor = null }, .{ .author_settlement = .{ .owner = @fromBackingInt(@as(u32, @intCast(i)) + 1), .anchor = .{ .planet = p, .local = 70 }, .population = 2, .food = 12, .material = 20, .economic_bypass = true } }, &bytes[i]);
    }
    try tick(&g, creator, &inputs);
    for (0..2) |i| {
        const p: t.PlanetId = @fromBackingInt(@as(u32, @intCast(i)) + 1);
        inputs[i] = try command.encodeEnvelope(p, .{ .sequence = @fromBackingInt(@as(u64, @intCast(i)) + 3), .provenance = .ruler, .actor = @fromBackingInt(1) }, .{ .build = .{ .settlement = @fromBackingInt(1), .kind = .house, .anchor = .{ .planet = p, .local = 69 }, .facing = null } }, &bytes[i]);
    }
    try tick(&g, .{ .role = .ruler, .civilization = @fromBackingInt(1) }, &inputs);
    try tick(&g, creator, &.{});
    const token = try api.acquireView(&g);
    defer api.releaseView(&g, token) catch unreachable;
    var work: ?u64 = null;
    for ([_]u16{ 1, 7, 256, 4096 }) |units| {
        var allocator = std.testing.FailingAllocator.init(std.testing.allocator, .{ .fail_index = 3 });
        var c = try replay.begin(allocator.allocator(), &g, token);
        try std.testing.expectError(error.NotReady, replay.takeVerified(&c));
        try finish(&c, units);
        if (work) |expected| try std.testing.expectEqual(expected, c.work_units) else work = c.work_units;
        try std.testing.expectEqual(3, allocator.alloc_index);
        try std.testing.expect(!allocator.has_induced_failure);
        var target = try replay.takeVerified(&c);
        defer api.deinit(&target) catch unreachable;
        try replay.deinit(&c);
        const tv = try api.acquireView(&target);
        defer api.releaseView(&target, tv) catch unreachable;
        try std.testing.expectEqualDeep(try api.summary(&g, token), try api.summary(&target, tv));
        try std.testing.expectEqualDeep((try view.storage(&g)).config, (try view.storage(&target)).config);
        try std.testing.expect(tv.owner_generation != token.owner_generation);
    }
}
test "cg/replay/adversarial-and-lifetime" {
    var g = try create(config);
    defer api.deinit(&g) catch unreachable;
    try tick(&g, creator, &.{});
    const token = try api.acquireView(&g);
    defer api.releaseView(&g, token) catch unreachable;
    const s = try view.storage(&g);
    const d = &s.metadata().descriptors[token.descriptor_slot];
    const count = d.history_count;
    d.history_count -= 1;
    var c = try replay.begin(std.testing.allocator, &g, token);
    try std.testing.expectEqual(error.InvalidEnvelope, (try rejected(&c)).cause);
    try std.testing.expectError(error.NotReady, replay.takeVerified(&c));
    try replay.deinit(&c);
    d.history_count = count;
    try std.testing.expectEqual(1, s.metadata().descriptor_leases[token.descriptor_slot]);
    try std.testing.expectEqual(1, @backingInt((try api.summary(&g, token)).tick));
    c = try replay.begin(std.testing.allocator, &g, token);
    replay.cancel(&c);
    for (0..500000) |_| {
        var b: t.WorkBudget = .{ .remaining = 4096 };
        const p = replay.poll(&c, &b);
        if (p == .canceled) break;
        try std.testing.expect(p == .pending);
    } else return error.Timeout;
    try replay.deinit(&c);
    try std.testing.expectEqual(1, s.metadata().descriptor_leases[token.descriptor_slot]);
}

fn settlementInput(out: []u8, planet: u32, sequence: u64) !t.RoutedEnvelope {
    const p: t.PlanetId = @fromBackingInt(planet);
    return command.encodeEnvelope(p, .{ .sequence = @fromBackingInt(sequence), .provenance = .creator, .actor = null }, .{ .author_settlement = .{ .owner = @fromBackingInt(planet), .anchor = .{ .planet = p, .local = 70 }, .population = 2, .food = 20, .material = 20, .economic_bypass = true } }, out);
}
fn expectRejected(g: *api.Galaxy, token: t.ViewToken, expected: t.Failure) !void {
    var c = try replay.begin(std.testing.allocator, g, token);
    defer replay.deinit(&c) catch unreachable;
    try std.testing.expectEqual(expected, (try rejected(&c)).cause);
    try std.testing.expectError(error.NotReady, replay.takeVerified(&c));
    _ = try api.summary(g, token);
}
test "cg/replay/malformed-transcripts-and-local-head" {
    var g = try create(config);
    defer api.deinit(&g) catch unreachable;
    var buffers: [2][256]u8 = undefined;
    const inputs = [_]t.RoutedEnvelope{ try settlementInput(&buffers[0], 1, 1), try settlementInput(&buffers[1], 2, 2) };
    try tick(&g, creator, &inputs);
    const token = try api.acquireView(&g);
    defer api.releaseView(&g, token) catch unreachable;
    const s = try view.storage(&g);
    const rows = &s.reservation.?.history;
    const h = "NYON-CG0-RECEIPT".len + 11;
    rows[1][h + 17] = 1; // ruler without required civilization
    try expectRejected(&g, token, error.InvalidPolicy);
    rows[1][h + 17] = 0;
    const first = rows[2];
    const second = rows[3];
    rows[2] = second;
    rows[3] = first;
    try expectRejected(&g, token, error.InvalidEnvelope); // ordinal order
    rows[2] = first;
    rows[3] = second;
    @memcpy(rows[2][h..], second[h..]);
    @memcpy(rows[3][h..], first[h..]);
    try expectRejected(&g, token, error.InvariantViolation); // same ordinals, noncanonical input order
    rows[2] = first;
    rows[3] = second;
    rows[2][h] = 3; // unknown planet
    try expectRejected(&g, token, error.InvalidPlanet);
    rows[2] = first;
    // Body begins with sequence:u64. Preserve the second route.
    std.mem.writeInt(u64, rows[3][h + 6 ..][0..8], 1, .little);
    try expectRejected(&g, token, error.DuplicateSequence);
    rows[3] = second;
    s.metadata().descriptors[token.descriptor_slot].planets[0].history_head[0] ^= 1;
    try expectRejected(&g, token, error.InvariantViolation);
    s.metadata().descriptors[token.descriptor_slot].planets[0].history_head[0] ^= 1;
    const local_res = s.metadata().planets[0].?.storage().reservation;
    local_res.history[1][0] ^= 1; // Global heads alone cannot catch local byte tampering.
    try expectRejected(&g, token, error.InvariantViolation);
    local_res.history[1][0] ^= 1;
}
test "cg/replay/first-null-max-sequence-empty-and-exact-fit" {
    var cfg = config;
    cfg.planet_count = 1;
    cfg.global_history_capacity = 4;
    var g = try create(cfg);
    try tick(&g, creator, &.{});
    var token = try api.acquireView(&g);
    var c = try replay.begin(std.testing.allocator, &g, token);
    try finish(&c, 7);
    var target = try replay.takeVerified(&c);
    try replay.deinit(&c);
    try std.testing.expectEqual(4, (try view.storage(&target)).config.global_history_capacity);
    const tv = try api.acquireView(&target);
    try std.testing.expectEqualDeep(try api.summary(&g, token), try api.summary(&target, tv));
    try api.releaseView(&target, tv);
    try api.deinit(&target);
    try api.releaseView(&g, token);
    try api.deinit(&g);
    cfg.global_history_capacity = 8;
    g = try create(cfg);
    defer api.deinit(&g) catch unreachable;
    var buf: [256]u8 = undefined;
    const input = try settlementInput(&buf, 1, std.math.maxInt(u64));
    try tick(&g, creator, &.{input});
    try tick(&g, creator, &.{});
    token = try api.acquireView(&g);
    defer api.releaseView(&g, token) catch unreachable;
    c = try replay.begin(std.testing.allocator, &g, token);
    try finish(&c, 1);
    target = try replay.takeVerified(&c);
    defer api.deinit(&target) catch unreachable;
    try replay.deinit(&c);
    try std.testing.expectEqual(8, (try view.storage(&target)).metadata().descriptors[(try view.storage(&target)).metadata().committed_descriptor.?].history_count);
}
test "cg/replay/retained-descriptor-move-and-source-next-publication" {
    var g = try create(config);
    defer api.deinit(&g) catch unreachable;
    try tick(&g, creator, &.{});
    const old = try api.acquireView(&g);
    const original = try api.summary(&g, old);
    const id = try api.beginTick(&g, original.revision, @fromBackingInt(2), creator, &.{});
    for (0..500000) |_| {
        var b: t.WorkBudget = .{ .remaining = 4096 };
        if (api.poll(&g, id, &b) == .published) break;
    } else return error.Timeout;
    // Start after source advanced; duplicate the retained old descriptor.
    var initial = try replay.begin(std.testing.allocator, &g, old);
    var b: t.WorkBudget = .{ .remaining = 7 };
    _ = replay.poll(&initial, &b);
    var c = initial;
    initial = undefined;
    try api.releaseView(&g, old); // replay owns its own lease
    b = .{ .remaining = 4096 };
    try std.testing.expect(api.poll(&g, id, &b) == .waiting_for_views);
    try finish(&c, 256);
    var target = try replay.takeVerified(&c);
    defer api.deinit(&target) catch unreachable;
    try replay.deinit(&c);
    const tv = try api.acquireView(&target);
    try std.testing.expectEqualDeep(original, try api.summary(&target, tv));
    try api.releaseView(&target, tv);
    for (0..500000) |_| {
        b = .{ .remaining = 4096 };
        if (api.poll(&g, id, &b) == .idle) break;
    } else return error.Timeout;
}
test "cg/replay/allocation-prefix-maximum-and-no-poll-allocations" {
    var cfg = config;
    cfg.planet_count = 4;
    for (&cfg.planets) |*p| p.side = 32;
    var g = try create(cfg);
    defer api.deinit(&g) catch unreachable;
    const token = try api.acquireView(&g);
    defer api.releaseView(&g, token) catch unreachable;
    const s = try view.storage(&g);
    for (0..5) |fail| {
        var a = std.testing.FailingAllocator.init(std.testing.allocator, .{ .fail_index = fail });
        try std.testing.expectError(error.OutOfMemory, replay.begin(a.allocator(), &g, token));
        try std.testing.expectEqual(a.allocated_bytes, a.freed_bytes);
        try std.testing.expectEqual(1, s.metadata().descriptor_leases[token.descriptor_slot]);
    }
    var a = std.testing.FailingAllocator.init(std.testing.allocator, .{ .fail_index = 5 });
    var c = try replay.begin(a.allocator(), &g, token);
    try std.testing.expectEqual(94_306_304, c.requested_bytes);
    try std.testing.expectEqual(47_153_152, a.allocated_bytes);
    try finish(&c, 4096);
    try replay.deinit(&c); // an untaken verified target stays owned
    try std.testing.expectEqual(5, a.alloc_index);
    try std.testing.expect(!a.has_induced_failure);
    try std.testing.expectEqual(a.allocated_bytes, a.freed_bytes);
}
test "cg/replay/source-shutdown-and-stale-admission" {
    var g = try create(config);
    const token = try api.acquireView(&g);
    var c = try replay.begin(std.testing.allocator, &g, token);
    const s = try view.storage(&g);
    s.metadata().faulted = true;
    try api.beginFaultShutdown(&g);
    try std.testing.expectError(error.Shutdown, replay.begin(std.testing.allocator, &g, token));
    try std.testing.expectError(error.Busy, api.destroyFaulted(&g));
    try api.releaseView(&g, token);
    try std.testing.expectError(error.StaleView, replay.begin(std.testing.allocator, &g, token));
    try finish(&c, 256);
    try replay.deinit(&c);
    try api.destroyFaulted(&g);
}

test "cg/replay/creation-drain-fault-ownership" {
    const jobs = @import("job.zig");
    var g = try create(config);
    defer api.deinit(&g) catch unreachable;
    const token = try api.acquireView(&g);
    defer api.releaseView(&g, token) catch unreachable;
    var a = std.testing.FailingAllocator.init(std.testing.allocator, .{ .fail_index = 3 });
    var c = try replay.begin(a.allocator(), &g, token);
    try std.testing.expectError(error.NotReady, jobs.destroyFaultedBuilder(&c.builder));
    try std.testing.expectEqual(0, a.deallocations);
    // Diagnostic teardown only; the successful path must free via replay API.
    defer if (c.builder.handle != null) {
        const owned = jobs.builderStorage(&c.builder).?;
        c.builder.handle = null;
        owned.deinit();
        if (c.lease) |lease| {
            api.releaseView(&g, lease) catch unreachable;
            c.lease = null;
        }
    };
    for (0..500000) |_| {
        var b: t.WorkBudget = .{ .remaining = 1 };
        _ = replay.poll(&c, &b);
        const s = jobs.builderStorage(&c.builder).?;
        if (s.setup_index == 1) {
            const p = &s.metadata().planets[0].?;
            p.storage().reservation.directories[p.activeRoot().slot][0] = std.math.maxInt(u64);
            break;
        }
    } else return error.Timeout;
    replay.cancel(&c);
    try std.testing.expectEqual(error.CleanupFault, (try rejected(&c)).cause);
    try std.testing.expectEqual(1, (try view.storage(&g)).metadata().descriptor_leases[token.descriptor_slot]);
    try std.testing.expectEqual(0, a.deallocations);
    try std.testing.expectError(error.NotReady, replay.takeVerified(&c));
    try replay.deinit(&c);
    try std.testing.expectEqual(3, a.deallocations);
    try std.testing.expectEqual(a.allocated_bytes, a.freed_bytes);
    _ = try api.summary(&g, token);
}

fn populatedInputs(buffers: *[8][256]u8) ![8]t.RoutedEnvelope {
    var result: [8]t.RoutedEnvelope = undefined;
    for (0..2) |i| {
        const id: t.PlanetId = @fromBackingInt(@as(u32, @intCast(i)) + 1);
        const owner: t.CivilizationId = @fromBackingInt(@as(u32, @intCast(i)) + 1);
        const path = [_]t.CellId{ .{ .planet = id, .local = 70 }, .{ .planet = id, .local = 71 }, .{ .planet = id, .local = 4 }, .{ .planet = id, .local = 5 } };
        const payloads = [_]t.Command{
            .{ .author_settlement = .{ .owner = owner, .anchor = .{ .planet = id, .local = 70 }, .population = @intCast(2 + i), .food = @intCast(20 + i), .material = @intCast(8 + i), .economic_bypass = true } },
            .{ .author_settlement = .{ .owner = owner, .anchor = .{ .planet = id, .local = 5 }, .population = 1, .food = @intCast(6 + i), .material = 2, .economic_bypass = true } },
            .{ .author_build = .{ .settlement = @fromBackingInt(1), .kind = .house, .anchor = .{ .planet = id, .local = 69 }, .facing = null, .economic_bypass = true } },
            .{ .author_ship = .{ .source = @fromBackingInt(1), .destination = @fromBackingInt(2), .resource = .food, .amount = @intCast(1 + i), .path = &path, .cargo_source = .transfer, .economic_bypass = true } },
        };
        for (payloads, 0..) |payload, j| {
            const slot = i * 4 + j;
            result[slot] = try command.encodeEnvelope(id, .{ .sequence = @fromBackingInt(@as(u64, @intCast(slot)) + 1), .provenance = .creator, .actor = null }, payload, &buffers[slot]);
        }
    }
    return result;
}

fn reachRecords(c: *replay.Cursor) !void {
    for (0..500000) |_| {
        var b: t.WorkBudget = .{ .remaining = 1 };
        const result = replay.poll(c, &b);
        try std.testing.expect(result == .pending);
        if (c.step == .descriptor) return; // ticks drained; verification has not started
    }
    return error.Timeout;
}
test "cg/replay/full-logical-state-and-routes" {
    const ls = @import("../planet_economy/state.zig");
    var g = try create(config);
    defer api.deinit(&g) catch unreachable;
    var buffers: [8][256]u8 = undefined;
    const inputs = try populatedInputs(&buffers);
    try tick(&g, creator, &inputs);
    const token = try api.acquireView(&g);
    defer api.releaseView(&g, token) catch unreachable;
    var c = try replay.begin(std.testing.allocator, &g, token);
    try finish(&c, 7);
    try replay.deinit(&c);
    for (0..7) |mutation| {
        c = try replay.begin(std.testing.allocator, &g, token);
        try reachRecords(&c);
        const s = try view.storage(&c.target.?);
        const p = &s.metadata().planets[1].?;
        const storage = p.storage();
        const root = storage.active;
        var candidate: ls.Candidate = .{ .storage = storage, .root = root, .prepared = true };
        if (mutation == 0) {
            var meta = storage.rootMeta(root);
            meta.next_building += 1;
            storage.writeRootMeta(root, meta);
        } else if (mutation == 1) {
            const original = try storage.terrainAt(root, 0);
            try storage.setTerrainAt(root, 0, if (original == .rocky) .fertile else .rocky);
        } else {
            var record: ls.Record = switch (mutation) {
                2 => try storage.readRecord(root, .claim, 70),
                3 => try storage.readRecord(root, .settlement, 0),
                4 => try storage.readRecord(root, .building, 0),
                5 => try storage.readRecord(root, .shipment, 0),
                6 => .{ .route_cell = .{ .index = 1, .cell = .{ .planet = @fromBackingInt(2), .local = 0 } } },
                else => unreachable,
            };
            switch (mutation) {
                2 => record.claim.is_hall = !record.claim.is_hall,
                3 => record.settlement.food += 1,
                4 => record.building.remaining_ticks += 1,
                5 => record.shipment.quantity += 1,
                else => {},
            }
            var writer = ls.RecordWriter.init(if (mutation == 2) 70 else 0, record);
            var b: t.WorkBudget = .{ .remaining = 4096 };
            try std.testing.expect(try writer.advance(&candidate, &b) == .done);
        }
        try std.testing.expectEqual(error.InvariantViolation, (try rejected(&c)).cause);
        try replay.deinit(&c);
        try std.testing.expectEqual(1, (try view.storage(&g)).metadata().descriptor_leases[token.descriptor_slot]);
    }
}
fn cancelAndDrain(c: *replay.Cursor) !void {
    replay.cancel(c);
    try std.testing.expectError(error.NotReady, replay.takeVerified(c));
    for (0..500000) |_| {
        var b: t.WorkBudget = .{ .remaining = 7 };
        const result = replay.poll(c, &b);
        if (result == .canceled) return;
        try std.testing.expect(result == .pending);
    }
    return error.Timeout;
}
test "cg/replay/cancellation-at-extraction-drive-comparison-and-release" {
    var g = try create(config);
    defer api.deinit(&g) catch unreachable;
    var buffer: [256]u8 = undefined;
    const input = try settlementInput(&buffer, 1, 1);
    try tick(&g, creator, &.{input});
    const token = try api.acquireView(&g);
    defer api.releaseView(&g, token) catch unreachable;
    const original = try api.summary(&g, token);
    for (0..5) |position| {
        var allocator = std.testing.FailingAllocator.init(std.testing.allocator, .{ .fail_index = 3 });
        var c = try replay.begin(allocator.allocator(), &g, token);
        for (0..500000) |_| {
            var b: t.WorkBudget = .{ .remaining = 1 };
            _ = replay.poll(&c, &b);
            const reached = switch (position) {
                0 => c.step == .create,
                1 => c.step == .copy,
                2 => c.step == .drive,
                3 => c.step == .records,
                4 => c.step == .release,
                else => unreachable,
            };
            if (reached) break;
        } else return error.Timeout;
        try std.testing.expectError(error.Busy, replay.deinit(&c));
        try cancelAndDrain(&c);
        try std.testing.expectEqual(0, allocator.deallocations);
        try std.testing.expectEqual(1, (try view.storage(&g)).metadata().descriptor_leases[token.descriptor_slot]);
        try replay.deinit(&c);
        try std.testing.expectEqual(allocator.allocated_bytes, allocator.freed_bytes);
        try std.testing.expectEqual(3, allocator.alloc_index);
        try std.testing.expect(!allocator.has_induced_failure);
        try std.testing.expectEqualDeep(original, try api.summary(&g, token));
    }
}
test "cg/replay/target-cleanup-fault-ownership" {
    var g = try create(config);
    defer api.deinit(&g) catch unreachable;
    try tick(&g, creator, &.{});
    const token = try api.acquireView(&g);
    defer api.releaseView(&g, token) catch unreachable;
    const original = try api.summary(&g, token);
    for ([_]bool{ false, true }) |postcommit| {
        var allocator = std.testing.FailingAllocator.init(std.testing.allocator, .{ .fail_index = 3 });
        var c = try replay.begin(allocator.allocator(), &g, token);
        if (postcommit) {
            for (0..500000) |_| {
                var b: t.WorkBudget = .{ .remaining = 1 };
                _ = replay.poll(&c, &b);
                if (c.target) |*target| {
                    const s = try view.storage(target);
                    if (s.metadata().obsolete_descriptor) |slot| {
                        const p = &s.metadata().planets[0].?;
                        const root = s.metadata().descriptors[slot].planets[0].root;
                        p.storage().reservation.directories[root.slot][0] = std.math.maxInt(u64);
                        break;
                    }
                }
            } else return error.Timeout;
        } else {
            try reachRecords(&c);
            const p = &(try view.storage(&c.target.?)).metadata().planets[0].?;
            p.storage().reservation.directories[p.activeRoot().slot][0] = std.math.maxInt(u64);
            replay.cancel(&c);
        }
        try std.testing.expectEqual(error.CleanupFault, (try rejected(&c)).cause);
        try std.testing.expectError(error.NotReady, replay.takeVerified(&c));
        try std.testing.expectEqual(1, (try view.storage(&g)).metadata().descriptor_leases[token.descriptor_slot]);
        try std.testing.expectEqual(0, allocator.deallocations);
        try replay.deinit(&c);
        try std.testing.expectEqual(3, allocator.deallocations);
        try std.testing.expectEqual(allocator.allocated_bytes, allocator.freed_bytes);
        try std.testing.expectEqualDeep(original, try api.summary(&g, token));
    }
}
