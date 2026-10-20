const std = @import("std");
const vectors = @import("wire_vectors.zig");
const role = @import("../play_role.zig");
const types = @import("types.zig");
const global_history = @import("history.zig");
const island_history = @import("../planet_economy/history.zig");
const island_types = @import("../planet_economy/types.zig");
const island_command = @import("../planet_economy/command.zig");

// Reference field assembly lives only in these tests. It never calls an authority encoder.
const Witness = struct {
    storage: [2048]u8 = undefined,
    len: usize = 0,

    fn bytes(self: *Witness, value: []const u8) void {
        @memcpy(self.storage[self.len..][0..value.len], value);
        self.len += value.len;
    }
    fn int(self: *Witness, comptime T: type, value: T) void {
        var raw: [@sizeOf(T)]u8 = undefined;
        std.mem.writeInt(T, &raw, value, .little);
        self.bytes(&raw);
    }
    fn string(self: *Witness, value: []const u8) void {
        self.int(u16, @intCast(value.len));
        self.bytes(value);
    }
    fn local(self: *Witness, planet: u32, tag: u8, tick: u64, ordinal: u16, provenance: u8, sequence: ?u64) void {
        self.bytes("NYON-CG0-PLANET-RECEIPT");
        self.int(u32, planet);
        self.int(u8, tag);
        self.int(u64, tick);
        self.int(u16, ordinal);
        self.int(u8, provenance);
        if (sequence) |n| {
            self.int(u8, 1);
            self.int(u64, n);
        } else self.int(u8, 0);
    }
    fn global(self: *Witness, tag: u8, tick: u64, ordinal: u16) void {
        self.bytes("NYON-CG0-RECEIPT");
        self.int(u8, tag);
        self.int(u64, tick);
        self.int(u16, ordinal);
    }
    fn policy(self: *Witness, tag: u8, civilization: ?u32) void {
        self.int(u8, tag);
        if (civilization) |id| {
            self.int(u8, 1);
            self.int(u32, id);
        } else self.int(u8, 0);
    }
    fn slice(self: *const Witness) []const u8 {
        return self.storage[0..self.len];
    }
};

fn expectRecord(comptime vector: vectors.Vector, actual: *const Witness, expected_len: usize) !void {
    try std.testing.expectEqual(expected_len, vector.bytes.len);
    try std.testing.expectEqualSlices(u8, vector.bytes, actual.slice());
}
fn next(head: [32]u8, vector: vectors.Vector) ![32]u8 {
    var digest = std.crypto.hash.sha2.Sha256.init(.{});
    digest.update(&head);
    var raw: [4]u8 = undefined;
    std.mem.writeInt(u32, &raw, @intCast(vector.bytes.len), .little);
    digest.update(&raw);
    digest.update(vector.bytes);
    var out: [32]u8 = undefined;
    digest.final(&out);
    try std.testing.expectEqualSlices(u8, &vector.chain, &out);
    return out;
}
fn production(receipt: *const types.Receipt, vector: vectors.Vector) !void {
    var bytes: [2048]u8 = undefined;
    var encoder = global_history.Encoder.init(receipt, &bytes);
    while (true) {
        var budget = types.WorkBudget{ .remaining = 1 };
        if (try encoder.advance(&budget) == .done) break;
    }
    try std.testing.expectEqualSlices(u8, vector.bytes, bytes[0..encoder.len()]);
}
fn localProduction(receipt: *const island_types.Receipt, witness: *const Witness) !void {
    var bytes: [1024]u8 = undefined;
    var encoder = island_history.Encoder.initContext(receipt, &bytes, .{ .planet = @fromBackingInt(2), .system = 1, .civilization_count = 2 });
    while (true) {
        var budget = island_types.WorkBudget{ .remaining = 1 };
        if (try encoder.advance(&budget) == .done) break;
    }
    try std.testing.expectEqualSlices(u8, witness.slice(), bytes[0..encoder.len()]);
}
fn localGenesis(planet: u32, seed: u64, settlements: u16, buildings: u16, freight: u16, history: u16, civilizations: u8) Witness {
    var w = Witness{};
    w.local(planet, 0, 0, 0, 2, null);
    w.int(u32, planet);
    w.int(u32, 1);
    w.int(u64, seed);
    w.int(u16, 4);
    w.int(u16, settlements);
    w.int(u16, buildings);
    w.int(u16, freight);
    w.int(u16, history);
    w.int(u8, civilizations);
    for (1..@as(usize, civilizations) + 1) |id| w.int(u32, @intCast(id));
    return w;
}
fn globalGenesis(seed: u64, civilizations: u8, capacity: u16, local_heads: []const [32]u8, settlements: u16, buildings: u16, freight: u16, history: u16) Witness {
    var w = Witness{};
    w.global(0, 0, 0);
    w.string("nyon.coordinated-galaxy.v0");
    w.int(u64, seed);
    w.int(u8, 1);
    w.int(u8, civilizations);
    w.int(u8, @intCast(local_heads.len));
    w.int(u16, capacity);
    for (local_heads, 0..) |head, index| {
        w.int(u32, @intCast(index + 1));
        w.int(u32, 1);
        const derived = seed ^ (@as(u64, @intCast(index)) *% 0x9E3779B97F4A7C15);
        w.int(u64, derived);
        w.int(u16, 4);
        w.int(u16, settlements);
        w.int(u16, buildings);
        w.int(u16, freight);
        w.int(u16, history);
        w.bytes(&head);
    }
    return w;
}
fn input(local_planet: ?u32, tick: u64, previous: ?u8, current: u8, civilization: ?u32, count: u16) Witness {
    var w = Witness{};
    if (local_planet) |planet| w.local(planet, 1, tick, 0, 2, null) else w.global(1, tick, 0);
    w.int(u64, tick);
    w.int(u64, tick - 1);
    if (previous) |tag| {
        w.int(u8, 1);
        w.policy(tag, null);
    } else w.int(u8, 0);
    w.policy(current, civilization);
    w.int(u16, count);
    return w;
}
fn complete(planet: u32, tick: u64, ordinal: u16, sequence: u64, rejected: u16) Witness {
    var w = Witness{};
    w.local(planet, 10, tick, ordinal, 2, null);
    w.int(u64, tick);
    w.int(u64, sequence);
    for ([_]u16{ 0, rejected, 0, 0, 0 }) |value| w.int(u16, value);
    return w;
}
fn prepared(planet: u32, tick: u64, ordinal: u16, count: u16, head: [32]u8, rejected: u16) Witness {
    var w = Witness{};
    w.global(3, tick, ordinal);
    w.int(u32, planet);
    w.int(u16, count);
    w.bytes(&head);
    w.int(u16, 0);
    w.int(u16, rejected);
    return w;
}
fn commit(tick: u64, ordinal: u16, sequence: u64, local_counts: []const u16, local_heads: []const [32]u8, rejected: u16) Witness {
    var w = Witness{};
    w.global(4, tick, ordinal);
    w.int(u64, tick);
    w.int(u64, sequence);
    w.int(u16, 0);
    w.int(u16, rejected);
    w.int(u8, @intCast(local_counts.len));
    for (local_counts, local_heads, 0..) |count, head, index| {
        w.int(u32, @intCast(index + 1));
        w.int(u64, tick);
        w.int(u64, tick);
        w.int(u16, count);
        w.bytes(&head);
    }
    return w;
}

test "cg/wire/role-rejection-witness" {
    const ruler = try role.SessionPolicy.init(.ruler, @fromBackingInt(1));
    try std.testing.expectError(error.CreatorForbidden, ruler.admit(.creator, null));
    const record = vectors.two_l2_r2.bytes;
    try std.testing.expectEqual(@as(u8, @truncate(@backingInt(types.RejectReason.CreatorForbidden))), record[record.len - 3]);
    try std.testing.expectEqual(@as(u8, 0), record[record.len - 2]);
    try std.testing.expectEqual(@as(u8, 0), record[record.len - 1]);
}

test "cg/wire/independent-domain-and-genesis" {
    var h: [32]u8 = undefined;
    std.crypto.hash.sha2.Sha256.hash("NYON-CG0-HISTORY", &h, .{});
    try std.testing.expectEqualSlices(u8, &vectors.global_h0, &h);
    var local_h0: [32]u8 = undefined;
    std.crypto.hash.sha2.Sha256.hash("NYON-CG0-PLANET-HISTORY\x01\x00\x00\x00", &local_h0, .{});
    try std.testing.expectEqualSlices(u8, &vectors.planet1_h0, &local_h0);
    std.crypto.hash.sha2.Sha256.hash("NYON-CG0-PLANET-HISTORY\x02\x00\x00\x00", &local_h0, .{});
    try std.testing.expectEqualSlices(u8, &vectors.planet2_h0, &local_h0);

    try expectRecord(vectors.one_local_genesis, &localGenesis(1, 2, 1, 1, 1, 3, 1), 71);
    try expectRecord(vectors.one_global_genesis, &globalGenesis(2, 1, 4, &.{vectors.one_local_genesis.chain}, 1, 1, 1, 3), 126);
    try expectRecord(vectors.one_local_input1, &input(1, 1, null, 0, null, 0), 61);
    try expectRecord(vectors.one_local_commit1, &complete(1, 1, 1, 0, 0), 66);
    try expectRecord(vectors.one_global_input1, &input(null, 1, null, 0, null, 0), 48);
    try expectRecord(vectors.one_global_prepared1, &prepared(1, 1, 1, 3, vectors.one_local_commit1.chain, 0), 69);
    try expectRecord(vectors.one_global_commit1, &commit(1, 2, 0, &.{3}, &.{vectors.one_local_commit1.chain}, 0), 102);

    _ = try next(vectors.planet1_h0, vectors.one_local_genesis);
    _ = try next(vectors.one_local_genesis.chain, vectors.one_local_input1);
    _ = try next(vectors.one_local_input1.chain, vectors.one_local_commit1);
    _ = try next(vectors.global_h0, vectors.one_global_genesis);
    _ = try next(vectors.one_global_genesis.chain, vectors.one_global_input1);
    _ = try next(vectors.one_global_input1.chain, vectors.one_global_prepared1);
    _ = try next(vectors.one_global_prepared1.chain, vectors.one_global_commit1);

    try expectRecord(vectors.two_l1_g0, &localGenesis(1, 2, 64, 256, 64, 4096, 2), 75);
    try expectRecord(vectors.two_l2_g0, &localGenesis(2, 0x9E3779B97F4A7C17, 64, 256, 64, 4096, 2), 75);
    try expectRecord(vectors.two_g0, &globalGenesis(2, 2, 4096, &.{ vectors.two_l1_g0.chain, vectors.two_l2_g0.chain }, 64, 256, 64, 4096), 184);
    _ = try next(vectors.planet1_h0, vectors.two_l1_g0);
    _ = try next(vectors.planet2_h0, vectors.two_l2_g0);
    _ = try next(vectors.global_h0, vectors.two_g0);

    try expectRecord(vectors.two_l1_i1, &input(1, 1, null, 0, null, 0), 61);
    try expectRecord(vectors.two_l2_i1, &input(2, 1, null, 0, null, 0), 61);
    try expectRecord(vectors.two_l1_c1, &complete(1, 1, 1, 0, 0), 66);
    try expectRecord(vectors.two_l2_c1, &complete(2, 1, 1, 0, 0), 66);
    try expectRecord(vectors.two_g_i1, &input(null, 1, null, 0, null, 0), 48);
    try expectRecord(vectors.two_g_p1_1, &prepared(1, 1, 1, 3, vectors.two_l1_c1.chain, 0), 69);
    try expectRecord(vectors.two_g_p2_1, &prepared(2, 1, 2, 3, vectors.two_l2_c1.chain, 0), 69);
    try expectRecord(vectors.two_g_c1, &commit(1, 3, 0, &.{ 3, 3 }, &.{ vectors.two_l1_c1.chain, vectors.two_l2_c1.chain }, 0), 156);
    _ = try next(vectors.two_l1_g0.chain, vectors.two_l1_i1);
    _ = try next(vectors.two_l1_i1.chain, vectors.two_l1_c1);
    _ = try next(vectors.two_l2_g0.chain, vectors.two_l2_i1);
    _ = try next(vectors.two_l2_i1.chain, vectors.two_l2_c1);
    _ = try next(vectors.two_g0.chain, vectors.two_g_i1);
    _ = try next(vectors.two_g_i1.chain, vectors.two_g_p1_1);
    _ = try next(vectors.two_g_p1_1.chain, vectors.two_g_p2_1);
    _ = try next(vectors.two_g_p2_1.chain, vectors.two_g_c1);

    try expectRecord(vectors.two_l1_i2, &input(1, 2, 0, 1, 1, 0), 67);
    try expectRecord(vectors.two_l2_i2, &input(2, 2, 0, 1, 1, 1), 67);
    try expectRecord(vectors.two_l1_c2, &complete(1, 2, 1, 1, 0), 66);
    try expectRecord(vectors.two_l2_c2, &complete(2, 2, 2, 1, 1), 66);
    try expectRecord(vectors.two_g_i2, &input(null, 2, 0, 1, 1, 1), 54);
    try expectRecord(vectors.two_g_p1_2, &prepared(1, 2, 2, 5, vectors.two_l1_c2.chain, 0), 69);
    try expectRecord(vectors.two_g_p2_2, &prepared(2, 2, 3, 6, vectors.two_l2_c2.chain, 1), 69);
    try expectRecord(vectors.two_g_c2, &commit(2, 4, 1, &.{ 5, 6 }, &.{ vectors.two_l1_c2.chain, vectors.two_l2_c2.chain }, 1), 156);
    _ = try next(vectors.two_l1_c1.chain, vectors.two_l1_i2);
    _ = try next(vectors.two_l1_i2.chain, vectors.two_l1_c2);
    _ = try next(vectors.two_l2_c1.chain, vectors.two_l2_i2);
    _ = try next(vectors.two_l2_i2.chain, vectors.two_l2_r2);
    _ = try next(vectors.two_l2_r2.chain, vectors.two_l2_c2);
    _ = try next(vectors.two_g_c1.chain, vectors.two_g_i2);
    _ = try next(vectors.two_g_i2.chain, vectors.two_g_r2);
    _ = try next(vectors.two_g_r2.chain, vectors.two_g_p1_2);
    _ = try next(vectors.two_g_p1_2.chain, vectors.two_g_p2_2);
    _ = try next(vectors.two_g_p2_2.chain, vectors.two_g_c2);

    var body = Witness{};
    body.int(u64, 1); // global sequence
    body.int(u8, 0); // creator provenance, rejected under ruler policy
    body.int(u8, 0); // absent actor
    body.int(u8, 1); // grant tag
    body.int(u64, 1); // local settlement
    body.int(u32, 1);
    body.int(u32, 0);
    body.int(u8, 0);
    body.int(u8, 1); // explicit bypass
    try std.testing.expectEqual(@as(usize, 29), body.len);
    var routed = Witness{};
    routed.global(2, 2, 1);
    routed.int(u32, 2);
    routed.string(body.slice());
    try expectRecord(vectors.two_g_r2, &routed, 62);
    var rejected = Witness{};
    rejected.local(2, 3, 2, 1, 0, 1);
    rejected.string(body.slice());
    rejected.int(u16, 1); // CreatorForbidden: creator provenance under ruler policy
    rejected.int(u8, 0); // no offending index
    try expectRecord(vectors.two_l2_r2, &rejected, 82);
}

test "cg/wire/production-global-records-match-independent-literals" {
    var config = types.Config{ .seed = 2, .planet_count = 1, .civilization_count = 1, .global_history_capacity = 4 };
    config.planets[0] = .{ .side = 4, .capacities = .{ .settlements = 1, .buildings = 1, .freight = 1, .history_receipts = 3 } };
    var planets: [4]types.PlanetEntry = undefined;
    planets[0] = .{ .planet = @fromBackingInt(1), .system = @fromBackingInt(1), .derived_seed = 2, .side = 4, .capacities = config.planets[0].capacities, .local_genesis_head = vectors.one_local_genesis.chain };
    const genesis = types.Receipt{ .header = .{ .tick = @fromBackingInt(0), .ordinal = 0 }, .event = .{ .genesis = .{ .config = config, .planets = planets } } };
    try production(&genesis, vectors.one_global_genesis);
    const creator = try types.SessionPolicy.init(.creator, null);
    const input1 = types.Receipt{ .header = .{ .tick = @fromBackingInt(1), .ordinal = 0 }, .event = .{ .tick_input = .{ .target = @fromBackingInt(1), .expected_revision = @fromBackingInt(0), .previous_policy = null, .current_policy = creator, .count = 0 } } };
    try production(&input1, vectors.one_global_input1);
    const prepared_receipt = types.Receipt{ .header = .{ .tick = @fromBackingInt(1), .ordinal = 1 }, .event = .{ .planet_prepared = .{ .planet = @fromBackingInt(1), .local_history_count = 3, .local_head = vectors.one_local_commit1.chain, .accepted = 0, .rejected = 0 } } };
    try production(&prepared_receipt, vectors.one_global_prepared1);
    var commits: [4]types.CommitEntry = undefined;
    commits[0] = .{ .planet = @fromBackingInt(1), .tick = @fromBackingInt(1), .revision = @fromBackingInt(1), .local_history_count = 3, .local_head = vectors.one_local_commit1.chain };
    const commit_receipt = types.Receipt{ .header = .{ .tick = @fromBackingInt(1), .ordinal = 2 }, .event = .{ .tick_commit = .{ .revision = @fromBackingInt(1), .last_sequence = @fromBackingInt(0), .accepted = 0, .rejected = 0, .planet_count = 1, .planets = commits } } };
    try production(&commit_receipt, vectors.one_global_commit1);
    const routed = types.Receipt{ .header = .{ .tick = @fromBackingInt(2), .ordinal = 1 }, .event = .{ .routed_input = .{ .planet = @fromBackingInt(2), .body = &.{ 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1, 1, 0, 0, 0, 0, 0, 0, 0, 1, 0, 0, 0, 0, 0, 0, 0, 0, 1 } } } };
    try production(&routed, vectors.two_g_r2);
    var two_config = types.Config{ .seed = 2, .planet_count = 2, .civilization_count = 2, .global_history_capacity = 4096 };
    two_config.planets[0].side = 4;
    two_config.planets[1].side = 4;
    var two_planets: [4]types.PlanetEntry = undefined;
    two_planets[0] = .{ .planet = @fromBackingInt(1), .system = @fromBackingInt(1), .derived_seed = 2, .side = 4, .capacities = two_config.planets[0].capacities, .local_genesis_head = vectors.two_l1_g0.chain };
    two_planets[1] = .{ .planet = @fromBackingInt(2), .system = @fromBackingInt(1), .derived_seed = 0x9E3779B97F4A7C17, .side = 4, .capacities = two_config.planets[1].capacities, .local_genesis_head = vectors.two_l2_g0.chain };
    const two_genesis = types.Receipt{ .header = .{ .tick = @fromBackingInt(0), .ordinal = 0 }, .event = .{ .genesis = .{ .config = two_config, .planets = two_planets } } };
    try production(&two_genesis, vectors.two_g0);
    const first_prepared = types.Receipt{ .header = .{ .tick = @fromBackingInt(1), .ordinal = 1 }, .event = .{ .planet_prepared = .{ .planet = @fromBackingInt(1), .local_history_count = 3, .local_head = vectors.two_l1_c1.chain, .accepted = 0, .rejected = 0 } } };
    const second_prepared = types.Receipt{ .header = .{ .tick = @fromBackingInt(1), .ordinal = 2 }, .event = .{ .planet_prepared = .{ .planet = @fromBackingInt(2), .local_history_count = 3, .local_head = vectors.two_l2_c1.chain, .accepted = 0, .rejected = 0 } } };
    try production(&first_prepared, vectors.two_g_p1_1);
    try production(&second_prepared, vectors.two_g_p2_1);
}

test "cg/wire/production-local-rejection-retains-entire-routed-body" {
    const body = [_]u8{ 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1, 1, 0, 0, 0, 0, 0, 0, 0, 1, 0, 0, 0, 0, 0, 0, 0, 0, 1 };
    const receipt = island_types.Receipt{ .header = .{ .tick = @fromBackingInt(2), .ordinal = 1, .provenance = .creator, .sequence = @fromBackingInt(1) }, .event = .{ .rejected = .{ .envelope = .{ .bytes = &body }, .reason = .CreatorForbidden, .offending_index = null } } };
    var bytes: [1024]u8 = undefined;
    var encoder = island_history.Encoder.initContext(&receipt, &bytes, .{ .planet = @fromBackingInt(2), .system = 1, .civilization_count = 2 });
    while (true) {
        var budget = island_types.WorkBudget{ .remaining = 1 };
        if (try encoder.advance(&budget) == .done) break;
    }
    try std.testing.expectEqualSlices(u8, vectors.two_l2_r2.bytes, bytes[0..encoder.len()]);
    var chain_head = island_history.Chain.initContext(.{ .planet = @fromBackingInt(2), .system = 1, .civilization_count = 2 });
    try std.testing.expectEqualSlices(u8, &vectors.planet2_h0, &chain_head.visible);
    chain_head.visible = vectors.two_l2_i2.chain;
    try chain_head.begin(bytes[0..encoder.len()]);
    while (true) {
        var budget = island_types.WorkBudget{ .remaining = 1 };
        if (try chain_head.advance(&budget) == .done) break;
    }
    try std.testing.expectEqualSlices(u8, &vectors.two_l2_r2.chain, &chain_head.visible);
}

test "cg/wire/max-legal-path-local-accepted-rejected-and-global-routed" {
    var path: [65]island_types.CellId = undefined;
    var body = Witness{};
    body.int(u64, 1);
    body.int(u8, 0); // creator
    body.int(u8, 0); // no actor
    body.int(u8, 6); // ship
    body.int(u64, 1); // source
    body.int(u64, 2); // destination
    body.int(u8, 0); // food
    body.int(u16, 1); // amount
    body.int(u16, 65);
    for (&path, 0..) |*cell, index| {
        cell.* = .{ .planet = @fromBackingInt(2), .local = @intCast(index + 1) };
        body.int(u32, 2);
        body.int(u64, cell.local);
    }
    var envelope_bytes: [1024]u8 = undefined;
    const encoded = try island_command.encodeEnvelope(.{ .sequence = @fromBackingInt(1), .provenance = .creator, .actor = null }, .{ .ship = .{ .source = @fromBackingInt(1), .destination = @fromBackingInt(2), .resource = .food, .amount = 1, .path = &path } }, &envelope_bytes);
    try std.testing.expectEqualSlices(u8, body.slice(), encoded.bytes);
    const context: @import("../planet_economy/rules.zig").AlgorithmContext = .{ .planet = @fromBackingInt(2), .system = 1, .civilization_count = 2 };
    const header: island_types.ReceiptHeader = .{ .tick = @fromBackingInt(2), .ordinal = 1, .provenance = .creator, .sequence = @fromBackingInt(1) };
    var witness_accepted = Witness{};
    witness_accepted.local(2, 2, 2, 1, 0, 1);
    witness_accepted.string(body.slice());
    witness_accepted.int(u64, 1); // shipment
    witness_accepted.int(u16, 1); // transferred
    witness_accepted.int(u16, 1); // reserved
    witness_accepted.int(u64, 2); // departure
    witness_accepted.int(u64, 66); // arrival
    const accepted = island_types.Receipt{ .header = header, .event = .{ .accepted = .{ .envelope = encoded, .effect = .{ .ship = .{ .shipment = @fromBackingInt(1), .transferred = 1, .reserved = 1, .departure_tick = @fromBackingInt(2), .arrival_tick = @fromBackingInt(66) } } } } };
    var local_bytes: [1024]u8 = undefined;
    var accepted_encoder = island_history.Encoder.initContext(&accepted, &local_bytes, context);
    while (true) {
        var budget = island_types.WorkBudget{ .remaining = 1 };
        if (try accepted_encoder.advance(&budget) == .done) break;
    }
    try std.testing.expectEqualSlices(u8, witness_accepted.slice(), local_bytes[0..accepted_encoder.len()]);
    var witness_rejected = Witness{};
    witness_rejected.local(2, 3, 2, 1, 0, 1);
    witness_rejected.string(body.slice());
    witness_rejected.int(u16, 1);
    witness_rejected.int(u8, 0);
    const rejected = island_types.Receipt{ .header = header, .event = .{ .rejected = .{ .envelope = encoded, .reason = .CreatorForbidden, .offending_index = null } } };
    var rejected_encoder = island_history.Encoder.initContext(&rejected, &local_bytes, context);
    while (true) {
        var budget = island_types.WorkBudget{ .remaining = 1 };
        if (try rejected_encoder.advance(&budget) == .done) break;
    }
    try std.testing.expectEqualSlices(u8, witness_rejected.slice(), local_bytes[0..rejected_encoder.len()]);
    var global = Witness{};
    global.global(2, 2, 1);
    global.int(u32, 2);
    global.string(body.slice());
    const routed = types.Receipt{ .header = .{ .tick = @fromBackingInt(2), .ordinal = 1 }, .event = .{ .routed_input = .{ .planet = @fromBackingInt(2), .body = encoded.bytes } } };
    var global_bytes: [2048]u8 = undefined;
    var global_encoder = global_history.Encoder.init(&routed, &global_bytes);
    while (true) {
        var budget = types.WorkBudget{ .remaining = 1 };
        if (try global_encoder.advance(&budget) == .done) break;
    }
    try std.testing.expectEqualSlices(u8, global.slice(), global_bytes[0..global_encoder.len()]);
}

test "cg/wire/production-integrated-local-autonomous-tags" {
    const header: island_types.ReceiptHeader = .{ .tick = @fromBackingInt(2), .ordinal = 1, .provenance = .autonomous, .sequence = null };
    var w = Witness{};
    w.local(2, 4, 2, 1, 2, null);
    w.int(u64, 1); // shipment
    w.int(u64, 2); // source
    w.int(u64, 3); // destination
    w.int(u8, 1); // material
    w.int(u16, 5);
    w.int(u32, 7);
    w.int(u32, 9);
    const arrived = island_types.Receipt{ .header = header, .event = .{ .arrived = .{ .shipment = @fromBackingInt(1), .source = @fromBackingInt(2), .destination = @fromBackingInt(3), .resource = .material, .quantity = 5, .destination_stock_after = 7, .destination_reservation_after = 9 } } };
    try localProduction(&arrived, &w);
    w = .{};
    w.local(2, 5, 2, 1, 2, null);
    w.int(u64, 4);
    w.int(u64, 5);
    w.int(u8, 2);
    const completed = island_types.Receipt{ .header = header, .event = .{ .completed = .{ .building = @fromBackingInt(4), .settlement = @fromBackingInt(5), .kind = .house } } };
    try localProduction(&completed, &w);
    w = .{};
    w.local(2, 6, 2, 1, 2, null);
    w.int(u64, 4);
    w.int(u64, 5);
    w.int(u8, 3);
    w.int(u8, 0);
    w.int(u32, 7);
    w.int(u32, 9);
    const produced = island_types.Receipt{ .header = header, .event = .{ .produced = .{ .building = @fromBackingInt(4), .settlement = @fromBackingInt(5), .workers = 3, .resource = .food, .quantity = 7, .stock_after = 9 } } };
    try localProduction(&produced, &w);
    w = .{};
    w.local(2, 7, 2, 1, 2, null);
    w.int(u64, 4);
    w.int(u64, 5);
    w.int(u8, 1);
    w.int(u8, 3);
    w.int(u8, 2);
    w.int(u8, 1);
    w.int(u32, 7);
    w.int(u32, 9);
    w.int(u32, 11);
    const idle = island_types.Receipt{ .header = header, .event = .{ .idle = .{ .building = @fromBackingInt(4), .settlement = @fromBackingInt(5), .reason = .output_capacity, .workers_needed = 3, .workers_available = 2, .resource = .material, .quantity = 7, .stock_before = 9, .incoming_reserved = 11 } } };
    try localProduction(&idle, &w);
    w = .{};
    w.local(2, 8, 2, 1, 2, null);
    w.int(u64, 5);
    w.int(u8, 3);
    w.int(u8, 2);
    w.int(u8, 1);
    w.int(u32, 9);
    const supplied = island_types.Receipt{ .header = header, .event = .{ .supplied = .{ .settlement = @fromBackingInt(5), .needed = 3, .consumed = 2, .shortage = 1, .food_after = 9 } } };
    try localProduction(&supplied, &w);
    w = .{};
    w.local(2, 9, 2, 1, 2, null);
    w.int(u64, 5);
    w.int(u8, 1);
    w.int(u8, 3);
    w.int(u8, 2);
    w.int(u8, 4);
    w.int(u8, 5);
    w.int(u8, 6);
    const demography = island_types.Receipt{ .header = header, .event = .{ .demography = .{ .settlement = @fromBackingInt(5), .kind = .starvation, .population_before = 3, .population_after = 2, .housing = 4, .hunger_after = 5, .fed_streak_after = 6 } } };
    try localProduction(&demography, &w);
}
