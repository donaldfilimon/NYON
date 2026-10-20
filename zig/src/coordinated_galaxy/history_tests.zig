const std = @import("std");
const t = @import("types.zig");
const history = @import("history.zig");
const vectors = @import("wire_vectors.zig");
const state = @import("state.zig");
const view = @import("view.zig");
const helpers = @import("planet_tests.zig");

fn encode(receipt: *const t.Receipt, expected: vectors.Vector) !void {
    var bytes: [2048]u8 = undefined;
    var encoder = history.Encoder.init(receipt, &bytes);
    while (true) {
        var budget = t.WorkBudget{ .remaining = 1 };
        if (try encoder.advance(&budget) == .done) break;
    }
    try std.testing.expectEqualSlices(u8, expected.bytes, bytes[0..encoder.len()]);
}
fn chain(previous: [32]u8, expected: vectors.Vector) !void {
    var c = history.Chain{ .visible = previous };
    try c.begin(expected.bytes);
    var zero = t.WorkBudget{ .remaining = 0 };
    try std.testing.expectEqual(t.Step.pending, try c.advance(&zero));
    try std.testing.expectEqualSlices(u8, &previous, &c.visible);
    while (true) {
        var budget = t.WorkBudget{ .remaining = 1 };
        if (try c.advance(&budget) == .done) break;
    }
    try std.testing.expectEqualSlices(u8, &expected.chain, &c.visible);
}

test "cg/history/independent-vectors" {
    const policy = try t.SessionPolicy.init(.creator, null);
    const input = t.Receipt{ .header = .{ .tick = @fromBackingInt(1), .ordinal = 0 }, .event = .{ .tick_input = .{ .target = @fromBackingInt(1), .expected_revision = @fromBackingInt(0), .previous_policy = null, .current_policy = policy, .count = 0 } } };
    try encode(&input, vectors.one_global_input1);
    const prepared = t.Receipt{ .header = .{ .tick = @fromBackingInt(1), .ordinal = 1 }, .event = .{ .planet_prepared = .{ .planet = @fromBackingInt(1), .local_history_count = 3, .local_head = vectors.one_local_commit1.chain, .accepted = 0, .rejected = 0 } } };
    try encode(&prepared, vectors.one_global_prepared1);
    var config = t.Config{ .seed = 2, .planet_count = 1, .civilization_count = 1, .global_history_capacity = 4 };
    config.planets[0] = .{ .side = 4, .capacities = .{ .settlements = 1, .buildings = 1, .freight = 1, .history_receipts = 3 } };
    var entries: [4]t.PlanetEntry = undefined;
    entries[0] = .{ .planet = @fromBackingInt(1), .system = @fromBackingInt(1), .derived_seed = 2, .side = 4, .capacities = config.planets[0].capacities, .local_genesis_head = vectors.one_local_genesis.chain };
    const genesis = t.Receipt{ .header = .{ .tick = @fromBackingInt(0), .ordinal = 0 }, .event = .{ .genesis = .{ .config = config, .planets = entries } } };
    try encode(&genesis, vectors.one_global_genesis);
    var commits: [4]t.CommitEntry = undefined;
    commits[0] = .{ .planet = @fromBackingInt(1), .tick = @fromBackingInt(1), .revision = @fromBackingInt(1), .local_history_count = 3, .local_head = vectors.one_local_commit1.chain };
    const commit = t.Receipt{ .header = .{ .tick = @fromBackingInt(1), .ordinal = 2 }, .event = .{ .tick_commit = .{ .revision = @fromBackingInt(1), .last_sequence = @fromBackingInt(0), .accepted = 0, .rejected = 0, .planet_count = 1, .planets = commits } } };
    try encode(&commit, vectors.one_global_commit1);
    const ruler = try t.SessionPolicy.init(.ruler, @fromBackingInt(1));
    const input2 = t.Receipt{ .header = .{ .tick = @fromBackingInt(2), .ordinal = 0 }, .event = .{ .tick_input = .{ .target = @fromBackingInt(2), .expected_revision = @fromBackingInt(1), .previous_policy = policy, .current_policy = ruler, .count = 1 } } };
    try encode(&input2, vectors.two_g_i2);
    const routed = t.Receipt{ .header = .{ .tick = @fromBackingInt(2), .ordinal = 1 }, .event = .{ .routed_input = .{ .planet = @fromBackingInt(2), .body = &.{ 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1, 1, 0, 0, 0, 0, 0, 0, 0, 1, 0, 0, 0, 0, 0, 0, 0, 0, 1 } } } };
    try encode(&routed, vectors.two_g_r2);
    var two_commits: [4]t.CommitEntry = undefined;
    two_commits[0] = .{ .planet = @fromBackingInt(1), .tick = @fromBackingInt(2), .revision = @fromBackingInt(2), .local_history_count = 5, .local_head = vectors.two_l1_c2.chain };
    two_commits[1] = .{ .planet = @fromBackingInt(2), .tick = @fromBackingInt(2), .revision = @fromBackingInt(2), .local_history_count = 6, .local_head = vectors.two_l2_c2.chain };
    const two_commit = t.Receipt{ .header = .{ .tick = @fromBackingInt(2), .ordinal = 4 }, .event = .{ .tick_commit = .{ .revision = @fromBackingInt(2), .last_sequence = @fromBackingInt(1), .accepted = 0, .rejected = 1, .planet_count = 2, .planets = two_commits } } };
    try encode(&two_commit, vectors.two_g_c2);
}

fn descriptor(count: u16, head: [32]u8) t.Descriptor {
    return .{ .tick = @fromBackingInt(0), .revision = @fromBackingInt(0), .last_sequence = @fromBackingInt(0), .policy = null, .history_count = count, .history_head = head, .planet_count = 1, .planets = undefined };
}
fn stageRecord(f: *history.Finalizer, receipt: *const t.Receipt) !void {
    while (true) {
        var budget = t.WorkBudget{ .remaining = 1 };
        if (try f.stage(receipt, &budget) == .done) break;
    }
}
fn finish(f: *history.Finalizer) !void {
    while (true) {
        var budget = t.WorkBudget{ .remaining = 1 };
        if (try f.advance(&budget) == .done) break;
    }
}
test "cg/history/capacity-exact-fit and retained-prefix" {
    const policy = try t.SessionPolicy.init(.creator, null);
    try std.testing.expectEqual(@as(u16, 3), try history.requiredTickReceipts(1, 0));
    try std.testing.expectEqual(@as(u16, 262), try history.requiredTickReceipts(4, 256));
    try std.testing.expectError(error.TooManyCommands, history.requiredTickReceipts(4, 257));
    for ([_]u16{ 1, 3, 4 }) |capacity| {
        var config = t.Config{ .seed = 2, .planet_count = 1, .civilization_count = 1, .global_history_capacity = capacity };
        config.planets[0] = .{ .side = 4, .capacities = .{ .settlements = 1, .buildings = 1, .freight = 1, .history_receipts = 3 } };
        var storage = try state.Storage.init(std.testing.allocator, config);
        defer storage.deinit();
        var first = descriptor(0, history.initial_head);
        var planets: [4]t.PlanetEntry = undefined;
        planets[0] = .{ .planet = @fromBackingInt(1), .system = @fromBackingInt(1), .derived_seed = 2, .side = 4, .capacities = config.planets[0].capacities, .local_genesis_head = vectors.one_local_genesis.chain };
        const genesis = t.Receipt{ .header = .{ .tick = @fromBackingInt(0), .ordinal = 0 }, .event = .{ .genesis = .{ .config = config, .planets = planets } } };
        var f = history.Finalizer.init(&storage, &first);
        var zero = t.WorkBudget{ .remaining = 0 };
        try std.testing.expectEqual(t.Step.pending, try f.stage(&genesis, &zero));
        try std.testing.expectEqual(@as(u16, 0), f.staged_count);
        try stageRecord(&f, &genesis);
        try finish(&f);
        try std.testing.expectEqual(@as(u16, 1), f.historyCount());
        const prefix_len = storage.metadata().history_lengths[0];
        if (capacity == 4) {
            try std.testing.expectEqualSlices(u8, vectors.one_global_genesis.bytes, storage.reservation.?.history[0][0..prefix_len]);
            try std.testing.expectEqualSlices(u8, &vectors.one_global_genesis.chain, &f.head());
        }
        var prefix: [2048]u8 = undefined;
        @memcpy(prefix[0..prefix_len], storage.reservation.?.history[0][0..prefix_len]);
        var previous = descriptor(1, f.head());
        var tick = history.Finalizer.init(&storage, &previous);
        const input = t.Receipt{ .header = .{ .tick = @fromBackingInt(1), .ordinal = 0 }, .event = .{ .tick_input = .{ .target = @fromBackingInt(1), .expected_revision = @fromBackingInt(0), .previous_policy = null, .current_policy = policy, .count = 0 } } };
        const prepared = t.Receipt{ .header = .{ .tick = @fromBackingInt(1), .ordinal = 1 }, .event = .{ .planet_prepared = .{ .planet = @fromBackingInt(1), .local_history_count = 3, .local_head = vectors.one_local_commit1.chain, .accepted = 0, .rejected = 0 } } };
        var entries: [4]t.CommitEntry = undefined;
        entries[0] = .{ .planet = @fromBackingInt(1), .tick = @fromBackingInt(1), .revision = @fromBackingInt(1), .local_history_count = 3, .local_head = vectors.one_local_commit1.chain };
        const commit = t.Receipt{ .header = .{ .tick = @fromBackingInt(1), .ordinal = 2 }, .event = .{ .tick_commit = .{ .revision = @fromBackingInt(1), .last_sequence = @fromBackingInt(0), .accepted = 0, .rejected = 0, .planet_count = 1, .planets = entries } } };
        for ([_]*const t.Receipt{ &input, &prepared, &commit }) |receipt| try stageRecord(&tick, receipt);
        var zero_tick = t.WorkBudget{ .remaining = 0 };
        try std.testing.expectEqual(t.Step.pending, try tick.advance(&zero_tick));
        try std.testing.expectEqual(@as(u16, 0), tick.count);
        var budget = t.WorkBudget{ .remaining = 1 };
        if (capacity == 4) {
            try finish(&tick);
            try std.testing.expectEqual(@as(u16, 4), tick.historyCount());
            var next = descriptor(4, tick.head());
            next.tick = @fromBackingInt(1);
            next.revision = @fromBackingInt(1);
            next.policy = policy;
            var exhausted = history.Finalizer.init(&storage, &next);
            const next_input = t.Receipt{ .header = .{ .tick = @fromBackingInt(2), .ordinal = 0 }, .event = .{ .tick_input = .{ .target = @fromBackingInt(2), .expected_revision = @fromBackingInt(1), .previous_policy = policy, .current_policy = policy, .count = 0 } } };
            const next_prepared = t.Receipt{ .header = .{ .tick = @fromBackingInt(2), .ordinal = 1 }, .event = .{ .planet_prepared = .{ .planet = @fromBackingInt(1), .local_history_count = 5, .local_head = vectors.one_local_commit1.chain, .accepted = 0, .rejected = 0 } } };
            var next_entries: [4]t.CommitEntry = undefined;
            next_entries[0] = .{ .planet = @fromBackingInt(1), .tick = @fromBackingInt(2), .revision = @fromBackingInt(2), .local_history_count = 5, .local_head = vectors.one_local_commit1.chain };
            const next_commit = t.Receipt{ .header = .{ .tick = @fromBackingInt(2), .ordinal = 2 }, .event = .{ .tick_commit = .{ .revision = @fromBackingInt(2), .last_sequence = @fromBackingInt(0), .accepted = 0, .rejected = 0, .planet_count = 1, .planets = next_entries } } };
            for ([_]*const t.Receipt{ &next_input, &next_prepared, &next_commit }) |receipt| try stageRecord(&exhausted, receipt);
            try std.testing.expectEqual(@as(u16, 3), exhausted.staged_count);
            try std.testing.expectError(error.HistoryFull, exhausted.advance(&budget));
        } else try std.testing.expectError(error.HistoryFull, tick.advance(&budget));
        try std.testing.expectEqualSlices(u8, prefix[0..prefix_len], storage.reservation.?.history[0][0..prefix_len]);
    }
}

test "cg/history/independent-chain-heads and two-block padding" {
    var domain_head: [32]u8 = undefined;
    std.crypto.hash.sha2.Sha256.hash("NYON-CG0-HISTORY", &domain_head, .{});
    try std.testing.expectEqualSlices(u8, &domain_head, &history.initial_head);
    try chain(vectors.global_h0, vectors.one_global_genesis);
    try chain(vectors.one_global_genesis.chain, vectors.one_global_input1);
    try chain(vectors.one_global_input1.chain, vectors.one_global_prepared1);
    try chain(vectors.one_global_prepared1.chain, vectors.one_global_commit1);
    try chain(vectors.global_h0, vectors.two_g0);
    try chain(vectors.two_g0.chain, vectors.two_g_i1);
    try chain(vectors.two_g_i1.chain, vectors.two_g_p1_1);
    try chain(vectors.two_g_p1_1.chain, vectors.two_g_p2_1);
    try chain(vectors.two_g_p2_1.chain, vectors.two_g_c1);
    try chain(vectors.two_g_c1.chain, vectors.two_g_i2);
    try chain(vectors.two_g_i2.chain, vectors.two_g_r2);
    try chain(vectors.two_g_r2.chain, vectors.two_g_p1_2);
    try chain(vectors.two_g_p1_2.chain, vectors.two_g_p2_2);
    try chain(vectors.two_g_p2_2.chain, vectors.two_g_c2);
    const lengths = [_]usize{ 19, 20, 83, 84, 147, 148, 1024, 2048 };
    for (lengths) |length| {
        var bytes: [2048]u8 = undefined;
        for (bytes[0..length], 0..) |*byte, index| byte.* = @truncate(index *% 37);
        var c = history.Chain{ .visible = vectors.global_h0 };
        try c.begin(bytes[0..length]);
        while (true) {
            var budget = t.WorkBudget{ .remaining = 1 };
            if (try c.advance(&budget) == .done) break;
        }
        var digest = std.crypto.hash.sha2.Sha256.init(.{});
        digest.update(&vectors.global_h0);
        var raw: [4]u8 = undefined;
        std.mem.writeInt(u32, &raw, @intCast(length), .little);
        digest.update(&raw);
        digest.update(bytes[0..length]);
        var expected: [32]u8 = undefined;
        digest.final(&expected);
        try std.testing.expectEqualSlices(u8, &expected, &c.visible);
    }
}

test "cg/history/max-legal-global-staging fits 512 slots and 2048-byte records" {
    var config = t.Config{ .seed = 2, .planet_count = 4, .system_count = 4, .global_history_capacity = 4096 };
    for (0..4) |index| config.planets[index].system = @fromBackingInt(@as(u32, @intCast(index)) + 1);
    var storage = try state.Storage.init(std.testing.allocator, config);
    defer storage.deinit();
    var previous = descriptor(1, vectors.two_g0.chain);
    var f = history.Finalizer.init(&storage, &previous);
    const policy = try t.SessionPolicy.init(.creator, null);
    const input = t.Receipt{ .header = .{ .tick = @fromBackingInt(1), .ordinal = 0 }, .event = .{ .tick_input = .{ .target = @fromBackingInt(1), .expected_revision = @fromBackingInt(0), .previous_policy = null, .current_policy = policy, .count = 256 } } };
    try stageRecord(&f, &input);
    const body: [1024]u8 = @splat(0x5a);
    for (0..256) |index| {
        const receipt = t.Receipt{ .header = .{ .tick = @fromBackingInt(1), .ordinal = @intCast(index + 1) }, .event = .{ .routed_input = .{ .planet = @fromBackingInt(@as(u32, @intCast(index / 64)) + 1), .body = &body } } };
        try stageRecord(&f, &receipt);
    }
    for (0..4) |index| {
        const receipt = t.Receipt{ .header = .{ .tick = @fromBackingInt(1), .ordinal = @intCast(index + 257) }, .event = .{ .planet_prepared = .{ .planet = @fromBackingInt(@as(u32, @intCast(index)) + 1), .local_history_count = 1, .local_head = vectors.one_local_genesis.chain, .accepted = 0, .rejected = 0 } } };
        try stageRecord(&f, &receipt);
    }
    var planets: [4]t.CommitEntry = undefined;
    for (&planets, 0..) |*entry, index| entry.* = .{ .planet = @fromBackingInt(@as(u32, @intCast(index)) + 1), .tick = @fromBackingInt(1), .revision = @fromBackingInt(1), .local_history_count = 1, .local_head = vectors.one_local_genesis.chain };
    const commit = t.Receipt{ .header = .{ .tick = @fromBackingInt(1), .ordinal = 261 }, .event = .{ .tick_commit = .{ .revision = @fromBackingInt(1), .last_sequence = @fromBackingInt(256), .accepted = 0, .rejected = 0, .planet_count = 4, .planets = planets } } };
    try stageRecord(&f, &commit);
    try std.testing.expectEqual(@as(u16, 262), f.staged_count);
    try std.testing.expect(storage.metadata().staged_lengths[1] < 2048);
    try finish(&f);
    try std.testing.expectEqual(@as(u16, 263), f.historyCount());
    try std.testing.expectEqual(storage.metadata().staged_lengths[261], storage.metadata().history_lengths[262]);
}

test "cg/history/retained-global-and-local-pages-switch-with-descriptor" {
    var config = t.Config{ .seed = 2, .planet_count = 1, .civilization_count = 1, .global_history_capacity = 4 };
    config.planets[0] = .{ .side = 4, .capacities = .{ .settlements = 1, .buildings = 1, .freight = 1, .history_receipts = 3 } };
    var storage = try state.Storage.init(std.testing.allocator, config);
    errdefer storage.deinit();
    while (true) {
        var budget = t.WorkBudget{ .remaining = 256 };
        if (try storage.advanceSetup(&budget) == .done) break;
    }
    const p = &storage.metadata().planets[0].?;
    const old_root = p.activeRoot();
    var planets: [4]t.PlanetEntry = undefined;
    planets[0] = .{ .planet = @fromBackingInt(1), .system = @fromBackingInt(1), .derived_seed = 2, .side = 4, .capacities = config.planets[0].capacities, .local_genesis_head = (try p.metadata(old_root)).chain_head };
    const genesis = t.Receipt{ .header = .{ .tick = @fromBackingInt(0), .ordinal = 0 }, .event = .{ .genesis = .{ .config = config, .planets = planets } } };
    var empty = descriptor(0, history.initial_head);
    var first = history.Finalizer.init(&storage, &empty);
    try stageRecord(&first, &genesis);
    try finish(&first);
    try view.reserveDescriptor(&storage, .{ .history_count = first.historyCount(), .history_head = first.head() });
    storage.metadata().prepared_roots[0] = try p.prepared(old_root);
    while (true) {
        var budget = t.WorkBudget{ .remaining = 1 };
        if (try view.prepareDescriptor(&storage, &budget) == .done) break;
    }
    view.publishDescriptor(&storage);
    var g = try view.takeGalaxy(&storage);
    defer view.deinit(&g) catch unreachable;
    const s = try view.storage(&g);
    const old = try view.acquireView(&g);
    var global: [4]t.ReceiptValue = undefined;
    var local: [3]t.LocalReceiptValue = undefined;
    try std.testing.expectEqual(@as(u16, 1), (try view.receipts(&g, old, 0, &global)).written);
    try std.testing.expectEqualSlices(u8, vectors.one_global_genesis.bytes, global[0].bytes[0..global[0].len]);
    try std.testing.expectEqual(@as(u16, 1), (try view.planetReceipts(&g, old, @fromBackingInt(1), 0, &local)).written);
    try std.testing.expectEqualSlices(u8, vectors.one_local_genesis.bytes, local[0].bytes[0..local[0].len]);
    const live_planet = &s.metadata().planets[0].?;
    const policy = try t.SessionPolicy.init(.creator, null);
    try live_planet.beginPrepared(@fromBackingInt(0), @fromBackingInt(1), policy, &.{}, @fromBackingInt(0));
    for ([_]t.Phase{ .prepare, .commands, .arrivals, .construction, .production, .population, .local_finalize }) |phase| try helpers.advance(live_planet, phase, 7);
    const ready = try helpers.freeze(live_planet, 1);
    const committed = &s.metadata().descriptors[s.metadata().committed_descriptor.?];
    var tick = history.Finalizer.init(s, committed);
    const input = t.Receipt{ .header = .{ .tick = @fromBackingInt(1), .ordinal = 0 }, .event = .{ .tick_input = .{ .target = @fromBackingInt(1), .expected_revision = @fromBackingInt(0), .previous_policy = null, .current_policy = policy, .count = 0 } } };
    const prepared = t.Receipt{ .header = .{ .tick = @fromBackingInt(1), .ordinal = 1 }, .event = .{ .planet_prepared = .{ .planet = @fromBackingInt(1), .local_history_count = ready.history_count, .local_head = ready.history_head, .accepted = 0, .rejected = 0 } } };
    var commits: [4]t.CommitEntry = undefined;
    commits[0] = .{ .planet = @fromBackingInt(1), .tick = @fromBackingInt(1), .revision = @fromBackingInt(1), .local_history_count = ready.history_count, .local_head = ready.history_head };
    const commit = t.Receipt{ .header = .{ .tick = @fromBackingInt(1), .ordinal = 2 }, .event = .{ .tick_commit = .{ .revision = @fromBackingInt(1), .last_sequence = @fromBackingInt(0), .accepted = 0, .rejected = 0, .planet_count = 1, .planets = commits } } };
    for ([_]*const t.Receipt{ &input, &prepared, &commit }) |receipt| try stageRecord(&tick, receipt);
    try finish(&tick);
    try std.testing.expectEqualSlices(u8, &vectors.one_global_commit1.chain, &tick.head());
    try std.testing.expectEqual(@as(u16, 1), (try view.receipts(&g, old, 0, &global)).written);
    try std.testing.expectEqual(@as(u16, 1), (try view.planetReceipts(&g, old, @fromBackingInt(1), 0, &local)).written);
    const still_old = try view.acquireView(&g);
    try std.testing.expectEqual(@as(u16, 1), (try view.receipts(&g, still_old, 0, &global)).written);
    try view.reserveDescriptor(s, .{ .tick = @fromBackingInt(1), .revision = @fromBackingInt(1), .policy = policy, .history_count = tick.historyCount(), .history_head = tick.head() });
    s.metadata().prepared_roots[0] = ready;
    while (true) {
        var budget = t.WorkBudget{ .remaining = 1 };
        if (try view.prepareDescriptor(s, &budget) == .done) break;
    }
    var install_budget = t.WorkBudget{ .remaining = 1 };
    try std.testing.expectEqual(t.Step.done, try live_planet.installHidden(&install_budget));
    view.publishDescriptor(s);
    const newest = try view.acquireView(&g);
    try std.testing.expectEqual(@as(u16, 4), (try view.receipts(&g, newest, 0, &global)).written);
    try std.testing.expectEqualSlices(u8, vectors.one_global_commit1.bytes, global[3].bytes[0..global[3].len]);
    try std.testing.expectEqual(@as(u16, 3), (try view.planetReceipts(&g, newest, @fromBackingInt(1), 0, &local)).written);
    try std.testing.expectEqualSlices(u8, vectors.one_local_commit1.bytes, local[2].bytes[0..local[2].len]);
    try std.testing.expectEqual(@as(u16, 1), (try view.receipts(&g, old, 0, &global)).written);
    try view.releaseView(&g, old);
    try view.releaseView(&g, still_old);
    const obsolete = s.metadata().obsolete_descriptor.?;
    try helpers.retire(live_planet, old_root, 1);
    try view.finishRetirement(s, obsolete);
    try view.releaseView(&g, newest);
}

test "cg/history/local-HistoryFull-restores-global-and-local-visible-prefix" {
    var config = t.Config{ .seed = 2, .planet_count = 1, .civilization_count = 1, .global_history_capacity = 4 };
    config.planets[0] = .{ .side = 4, .capacities = .{ .settlements = 1, .buildings = 1, .freight = 1, .history_receipts = 1 } };
    var storage = try state.Storage.init(std.testing.allocator, config);
    errdefer storage.deinit();
    while (true) {
        var budget = t.WorkBudget{ .remaining = 256 };
        if (try storage.advanceSetup(&budget) == .done) break;
    }
    const p = &storage.metadata().planets[0].?;
    const old_root = p.activeRoot();
    const old_local = try p.metadata(old_root);
    var planets: [4]t.PlanetEntry = undefined;
    planets[0] = .{ .planet = @fromBackingInt(1), .system = @fromBackingInt(1), .derived_seed = 2, .side = 4, .capacities = config.planets[0].capacities, .local_genesis_head = old_local.chain_head };
    const genesis = t.Receipt{ .header = .{ .tick = @fromBackingInt(0), .ordinal = 0 }, .event = .{ .genesis = .{ .config = config, .planets = planets } } };
    var empty = descriptor(0, history.initial_head);
    var first = history.Finalizer.init(&storage, &empty);
    try stageRecord(&first, &genesis);
    try finish(&first);
    try view.reserveDescriptor(&storage, .{ .history_count = first.historyCount(), .history_head = first.head() });
    storage.metadata().prepared_roots[0] = try p.prepared(old_root);
    while (true) {
        var budget = t.WorkBudget{ .remaining = 1 };
        if (try view.prepareDescriptor(&storage, &budget) == .done) break;
    }
    view.publishDescriptor(&storage);
    var g = try view.takeGalaxy(&storage);
    defer view.deinit(&g) catch unreachable;
    const s = try view.storage(&g);
    const old = try view.acquireView(&g);
    const before = try view.summary(&g, old);
    const d = &s.metadata().descriptors[s.metadata().committed_descriptor.?];
    try std.testing.expectEqual(@as(u16, 1), d.history_count);
    try std.testing.expectEqual(@as(u16, 1), d.planets[0].history_count);
    try std.testing.expectEqual(null, d.policy);
    try std.testing.expectEqual(@as(u64, 0), @backingInt(d.last_sequence));
    var global: [4]t.ReceiptValue = undefined;
    var local: [3]t.LocalReceiptValue = undefined;
    try std.testing.expectEqual(@as(u16, 1), (try view.receipts(&g, old, 0, &global)).written);
    const old_global_record = global[0];
    try std.testing.expectEqual(@as(u16, 1), (try view.planetReceipts(&g, old, @fromBackingInt(1), 0, &local)).written);
    const old_local_record = local[0];

    const policy = try t.SessionPolicy.init(.creator, null);
    var staged = history.Finalizer.init(s, d);
    const input = t.Receipt{ .header = .{ .tick = @fromBackingInt(1), .ordinal = 0 }, .event = .{ .tick_input = .{ .target = @fromBackingInt(1), .expected_revision = @fromBackingInt(0), .previous_policy = null, .current_policy = policy, .count = 0 } } };
    try stageRecord(&staged, &input);
    try std.testing.expectEqual(@as(u16, 1), staged.staged_count);
    try p.beginPrepared(@fromBackingInt(0), @fromBackingInt(1), policy, &.{}, @fromBackingInt(12));
    for ([_]t.Phase{ .prepare, .commands, .arrivals, .construction, .production, .population }) |phase| try helpers.advance(p, phase, 1);
    try std.testing.expectError(error.HistoryFull, helpers.freeze(p, 1));
    try std.testing.expectEqual(t.Phase.local_finalize, p.control.lane.?.phase);
    try std.testing.expect(p.control.lane.?.step == .history);
    const failed_root = try p.candidateRoot();
    var restore_budget = t.WorkBudget{ .remaining = 1 };
    try std.testing.expectEqual(t.Step.done, try p.restoreHidden(&restore_budget));
    try helpers.retire(p, failed_root, 1);
    try std.testing.expectEqualDeep(old_root, p.activeRoot());
    try std.testing.expectEqualDeep(old_local, try p.metadata(old_root));
    try std.testing.expectEqual(@as(u16, 0), p.storage().staged_count);
    try std.testing.expectEqual(@as(u64, 0), @backingInt(s.metadata().published_sequence));

    const fresh = try view.acquireView(&g);
    for ([_]t.ViewToken{ old, fresh }) |token| {
        try std.testing.expectEqualDeep(before, try view.summary(&g, token));
        try std.testing.expectEqual(@as(u16, 1), (try view.receipts(&g, token, 0, &global)).written);
        try std.testing.expectEqual(old_global_record.len, global[0].len);
        try std.testing.expectEqualSlices(u8, old_global_record.bytes[0..old_global_record.len], global[0].bytes[0..global[0].len]);
        try std.testing.expectEqual(@as(u16, 1), (try view.planetReceipts(&g, token, @fromBackingInt(1), 0, &local)).written);
        try std.testing.expectEqual(old_local_record.len, local[0].len);
        try std.testing.expectEqualSlices(u8, old_local_record.bytes[0..old_local_record.len], local[0].bytes[0..local[0].len]);
    }
    try std.testing.expectEqual(@as(u16, 1), d.history_count);
    try std.testing.expectEqualSlices(u8, &before.chain_head, &d.history_head);
    try std.testing.expectEqual(@as(u16, 1), d.planets[0].history_count);
    try std.testing.expectEqualSlices(u8, &old_local.chain_head, &d.planets[0].history_head);
    try std.testing.expectEqual(null, d.policy);
    try std.testing.expectEqual(@as(u64, 0), @backingInt(d.last_sequence));
    try std.testing.expectEqual(@as(u64, 0), @backingInt(old_local.last_sequence));
    try std.testing.expectEqual(null, old_local.policy);
    try view.releaseView(&g, old);
    try view.releaseView(&g, fresh);
}

test "cg/history/integrated-empty-world-watermark-matches-independent-local-literals" {
    var config = t.Config{ .seed = 2, .planet_count = 2, .civilization_count = 2 };
    config.planets[0].side = 4;
    config.planets[1].side = 4;
    var storage = try state.Storage.init(std.testing.allocator, config);
    defer storage.deinit();
    while (true) {
        var budget = t.WorkBudget{ .remaining = 256 };
        if (try storage.advanceSetup(&budget) == .done) break;
    }
    const p = &storage.metadata().planets[0].?;
    var local: [5]t.LocalReceiptValue = undefined;
    const old = p.activeRoot();
    try std.testing.expectEqualSlices(u8, &vectors.two_l1_g0.chain, &(try p.metadata(old)).chain_head);
    const creator = try t.SessionPolicy.init(.creator, null);
    try p.beginPrepared(@fromBackingInt(0), @fromBackingInt(1), creator, &.{}, @fromBackingInt(0));
    for ([_]t.Phase{ .prepare, .commands, .arrivals, .construction, .production, .population, .local_finalize }) |phase| try helpers.advance(p, phase, 7);
    const first = try helpers.freeze(p, 1);
    try std.testing.expectEqualSlices(u8, &vectors.two_l1_c1.chain, &first.history_head);
    var one = t.WorkBudget{ .remaining = 1 };
    try std.testing.expectEqual(t.Step.done, try p.installHidden(&one));
    try helpers.retire(p, old, 1);
    const ruler = try t.SessionPolicy.init(.ruler, @fromBackingInt(1));
    try p.beginPrepared(@fromBackingInt(1), @fromBackingInt(2), ruler, &.{}, @fromBackingInt(1));
    for ([_]t.Phase{ .prepare, .commands, .arrivals, .construction, .production, .population, .local_finalize }) |phase| try helpers.advance(p, phase, 7);
    const second = try helpers.freeze(p, 1);
    try std.testing.expectEqual(@as(u16, 5), second.history_count);
    try std.testing.expectEqualSlices(u8, &vectors.two_l1_c2.chain, &second.history_head);
    try std.testing.expectEqual(@as(u16, 5), (try p.receipts(second.root, 0, &local)).written);
    try std.testing.expectEqualSlices(u8, vectors.two_l1_i2.bytes, local[3].bytes[0..local[3].len]);
    try std.testing.expectEqualSlices(u8, vectors.two_l1_c2.bytes, local[4].bytes[0..local[4].len]);
    const meta = try p.metadata(second.root);
    try std.testing.expectEqual(@as(u64, 1), @backingInt(meta.last_sequence));
    try std.testing.expectEqualDeep(ruler, meta.policy.?);
}
