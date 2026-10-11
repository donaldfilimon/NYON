const std = @import("std");
const vectors = @import("wire_vectors.zig");
const role = @import("../play_role.zig");
const types = @import("types.zig");

// Reference field assembly lives only in these tests. It never calls an authority encoder.
const Witness = struct {
    storage: [512]u8 = undefined,
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
