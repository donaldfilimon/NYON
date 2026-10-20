const std = @import("std");
const ledger = @import("allocator_ledger.zig");
const types = @import("types.zig");

const first_bytes: [112]u8 = blk: {
    var bytes: [112]u8 = undefined;
    _ = std.fmt.hexToBytes(&bytes, "4e594f4e414c3031010000000000000000000000000000000000000000000000000000000000000000000000000000000100000000000000010000000000000001000000000000000100000000000000b13f204fbf670042e8a3c940e0d965a9c669da3c01a9b4b64111d2578f1efec7") catch unreachable;
    break :blk bytes;
};

fn first() ledger.LedgerRecord {
    return ledger.decodeRecord(&first_bytes) catch unreachable;
}

fn rehash(record: *ledger.LedgerRecord) void {
    var bytes: [80]u8 = undefined;
    @memcpy(bytes[0..8], "NYONAL01");
    std.mem.writeInt(u64, bytes[8..16], record.ordinal, .little);
    @memcpy(bytes[16..48], &record.previous_digest);
    std.mem.writeInt(u64, bytes[48..56], record.next_generation, .little);
    std.mem.writeInt(u64, bytes[56..64], record.next_slot_id, .little);
    std.mem.writeInt(u64, bytes[64..72], record.next_branch_id, .little);
    std.mem.writeInt(u64, bytes[72..80], record.next_request_id, .little);
    std.crypto.hash.sha2.Sha256.hash(&bytes, &record.digest, .{});
}

fn next(current: ledger.LedgerRecord, kind: ledger.AllocationKind) !ledger.LedgerRecord {
    return (try ledger.reserve(current, kind)).record;
}

fn emptyCatalog(generation: u64, watermark: u64) types.Catalog {
    return .{ .generation = generation, .allocator_watermark = watermark, .slots = &.{}, .selection = null };
}

fn checkRejectedOrigin(record: ledger.LedgerRecord) !void {
    var chain = ledger.Chain.init(1);
    try std.testing.expectError(error.Invalid, chain.accept(record));
    try std.testing.expectEqual(@as(usize, 0), chain.count);
    try std.testing.expect(chain.head == null);
}

test "literal_first_record" {
    try std.testing.expectEqual(@as(usize, 112), first_bytes.len);
    try std.testing.expectEqualStrings("NYONAL01", first_bytes[0..8]);
    try std.testing.expectEqual(@as(u64, 1), std.mem.readInt(u64, first_bytes[8..16], .little));
    const zero_digest: [32]u8 = @splat(0);
    try std.testing.expectEqualSlices(u8, &zero_digest, first_bytes[16..48]);
    inline for ([_]usize{ 48, 56, 64, 72 }) |offset| {
        try std.testing.expectEqual(@as(u64, 1), std.mem.readInt(u64, first_bytes[offset..][0..8], .little));
    }
    var hash: types.Digest = undefined;
    std.crypto.hash.sha2.Sha256.hash(first_bytes[0..80], &hash, .{});
    try std.testing.expectEqualSlices(u8, &hash, first_bytes[80..112]);
    const record = first();
    try std.testing.expectEqualSlices(u8, &first_bytes, &(try ledger.encodeRecord(record)));
    var chain = ledger.Chain.init(1);
    try chain.accept(record);
    const binding = try chain.finish();
    try std.testing.expectEqual(@as(u64, 1), binding.matched.ordinal);
    try std.testing.expectEqual(@as(u64, 0), binding.predecessor.ordinal);
    try std.testing.expectEqual(@as(u64, 1), binding.head.?.ordinal);
}

test "partial_record" {
    try std.testing.expectError(error.Invalid, ledger.decodeRecord(first_bytes[0..111]));
    var longer: [113]u8 = @splat(0);
    @memcpy(longer[0..112], &first_bytes);
    try std.testing.expectError(error.Invalid, ledger.decodeRecord(&longer));
    longer[0] = 'X';
    try std.testing.expectError(error.Invalid, ledger.decodeRecord(longer[0..112]));
    var corrupted = first_bytes;
    corrupted[56] ^= 1;
    try std.testing.expectError(error.Invalid, ledger.decodeRecord(&corrupted));
}

test "false_origin_each_field" {
    var changed = first();
    changed.next_generation = 2;
    rehash(&changed);
    try checkRejectedOrigin(changed);
    changed = first();
    changed.next_slot_id = 2;
    rehash(&changed);
    try checkRejectedOrigin(changed);
    changed = first();
    changed.next_branch_id = 2;
    rehash(&changed);
    try checkRejectedOrigin(changed);
    changed = first();
    changed.next_request_id = 2;
    rehash(&changed);
    try checkRejectedOrigin(changed);
    changed = first();
    changed.ordinal = 2;
    rehash(&changed);
    try checkRejectedOrigin(changed);
    changed = first();
    changed.previous_digest[0] = 1;
    rehash(&changed);
    try checkRejectedOrigin(changed);
}

test "broken_chain" {
    const origin = first();
    const second = try next(origin, .generation);
    var chain = ledger.Chain.init(2);
    try chain.accept(origin);
    var bad = second;
    bad.ordinal = 3;
    rehash(&bad);
    try std.testing.expectError(error.Invalid, chain.accept(bad));
    try std.testing.expectEqual(@as(usize, 1), chain.count);
    bad = second;
    bad.ordinal = 1;
    rehash(&bad);
    try std.testing.expectError(error.Invalid, chain.accept(bad));
    bad = second;
    bad.previous_digest[0] ^= 1;
    rehash(&bad);
    try std.testing.expectError(error.Invalid, chain.accept(bad));
    bad = second;
    bad.next_slot_id = 0;
    rehash(&bad);
    try std.testing.expectError(error.Invalid, chain.accept(bad));
    bad = second;
    bad.next_generation = 1;
    rehash(&bad);
    try std.testing.expectError(error.Invalid, chain.accept(bad));
    try std.testing.expectEqualDeep(origin, chain.head.?);
    try chain.accept(second);
    bad = try next(second, .slot);
    bad.next_generation = 1;
    rehash(&bad);
    try std.testing.expectError(error.Invalid, chain.accept(bad));
    const third = try next(second, .slot);
    inline for ([_]ledger.AllocationKind{ .generation, .slot }) |kind| {
        bad = try next(third, .request);
        switch (kind) {
            .generation => bad.next_generation = 1,
            .slot => bad.next_slot_id = 1,
            else => unreachable,
        }
        rehash(&bad);
        var three = ledger.Chain.init(4);
        try three.accept(origin);
        try three.accept(second);
        try three.accept(third);
        try std.testing.expectError(error.Invalid, three.accept(bad));
        try std.testing.expectEqualDeep(third, three.head.?);
    }
    try std.testing.expectEqualDeep(second, chain.head.?);
}

test "independent_branch_request_rewinds" {
    const origin = first();
    const generation = try next(origin, .generation);
    const branch = try next(generation, .branch);
    const request = try next(branch, .request);
    var chain = ledger.Chain.init(4);
    for ([_]ledger.LedgerRecord{ origin, generation, branch, request }) |record| try chain.accept(record);
    const before = chain;
    var bad = try next(request, .slot);
    bad.next_branch_id = 1;
    rehash(&bad);
    try std.testing.expectError(error.Invalid, chain.accept(bad));
    try std.testing.expectEqualDeep(before, chain);
    bad = try next(request, .slot);
    bad.next_request_id = 1;
    rehash(&bad);
    try std.testing.expectError(error.Invalid, chain.accept(bad));
    try std.testing.expectEqualDeep(before, chain);
}

test "monotonic_high_water" {
    const origin = first();
    var gap = origin;
    gap.ordinal = 2;
    gap.previous_digest = origin.digest;
    gap.next_slot_id = 13;
    gap.next_request_id = 8;
    rehash(&gap);
    var chain = ledger.Chain.init(2);
    try chain.accept(origin);
    try chain.accept(gap);
    const bound = try chain.finish();
    try std.testing.expectEqual(@as(u64, 13), bound.matched.next_slot_id);
    try std.testing.expectEqualDeep(origin, bound.predecessor);
    try std.testing.expectEqual(@as(u64, 13), (try ledger.reserve(gap, .slot)).allocated);
    const reservation = try ledger.reserve(gap, .branch);
    try std.testing.expectEqual(@as(u64, 1), reservation.allocated);
    try std.testing.expectEqual(@as(u64, 3), reservation.record.ordinal);
    try std.testing.expectEqualSlices(u8, &gap.digest, &reservation.record.previous_digest);
    try std.testing.expectEqual(@as(u64, 2), reservation.record.next_branch_id);
    try std.testing.expectEqualDeep(gap, chain.head.?);
}

test "exhaustion_preserves_record" {
    const origin = first();
    inline for ([_]ledger.AllocationKind{ .generation, .slot, .branch, .request }) |kind| {
        var exhausted = origin;
        switch (kind) {
            .generation => exhausted.next_generation = std.math.maxInt(u64),
            .slot => exhausted.next_slot_id = std.math.maxInt(u64),
            .branch => exhausted.next_branch_id = std.math.maxInt(u64),
            .request => exhausted.next_request_id = std.math.maxInt(u64),
        }
        rehash(&exhausted);
        const before = exhausted;
        try std.testing.expectError(error.IdExhausted, ledger.reserve(exhausted, kind));
        try std.testing.expectEqualDeep(before, exhausted);
    }
    var ordinal_max = origin;
    ordinal_max.ordinal = std.math.maxInt(u64);
    rehash(&ordinal_max);
    try std.testing.expectError(error.IdExhausted, ledger.reserve(ordinal_max, .slot));
}

test "journal_capacity" {
    var chain = ledger.Chain.init(0);
    var current = first();
    try chain.accept(current);
    var ordinal: usize = 1;
    while (ordinal < 1_048_576) : (ordinal += 1) {
        current = try next(current, .request);
        try chain.accept(current);
    }
    const before = chain.head.?;
    const too_many = try next(current, .request);
    try std.testing.expectError(error.TooLarge, chain.accept(too_many));
    try std.testing.expectEqual(@as(usize, 1_048_576), chain.count);
    try std.testing.expectEqualDeep(before, chain.head.?);
}

test "watermark_binding_and_complete_suffix" {
    const origin = first();
    const generation = try next(origin, .generation);
    const suffix = try next(generation, .request);
    var chain = ledger.Chain.init(2);
    try std.testing.expectError(error.Invalid, chain.finish());
    try chain.accept(origin);
    try std.testing.expectError(error.Invalid, chain.finish());
    try chain.accept(generation);
    try chain.accept(suffix);
    const bound = try chain.finish();
    try std.testing.expectEqualDeep(suffix, bound.head.?);
    try std.testing.expectEqualDeep(generation, bound.matched);
    try std.testing.expectEqualDeep(origin, bound.predecessor);
    var corrupt = try next(suffix, .slot);
    corrupt.digest[0] ^= 1;
    try std.testing.expectError(error.Invalid, chain.accept(corrupt));
    try std.testing.expectEqualDeep(suffix, chain.head.?);
    var missing = ledger.Chain.init(4);
    try missing.accept(origin);
    try missing.accept(generation);
    try missing.accept(suffix);
    try std.testing.expectError(error.Invalid, missing.finish());
    var zero = ledger.Chain.init(0);
    try zero.accept(origin);
    try zero.accept(generation);
    const zero_bound = try zero.finish();
    try std.testing.expectEqual(@as(u64, 0), zero_bound.matched.ordinal);
    try std.testing.expectEqualDeep(generation, zero_bound.head.?);
    const empty = emptyCatalog(0, 0);
    try ledger.validateAgainstLedger(&empty, &zero_bound);
}

test "exact_generation_and_watermark" {
    const origin = first();
    const request = try next(origin, .request);
    const generation = try next(request, .generation);
    const newer = try next(generation, .generation);
    var chain = ledger.Chain.init(3);
    try chain.accept(origin);
    try chain.accept(request);
    try chain.accept(generation);
    try chain.accept(newer);
    const binding = try chain.finish();
    var catalog = emptyCatalog(1, 3);
    try ledger.validateAgainstLedger(&catalog, &binding);
    catalog.allocator_watermark = 4;
    try std.testing.expectError(error.Invalid, ledger.validateAgainstLedger(&catalog, &binding));
    catalog.allocator_watermark = 3;
    catalog.generation = 2;
    try std.testing.expectError(error.Invalid, ledger.validateAgainstLedger(&catalog, &binding));
    catalog.generation = 3;
    const excess = catalog;
    try std.testing.expectError(error.Invalid, ledger.validateAgainstLedger(&catalog, &binding));
    try std.testing.expectEqualDeep(excess, catalog);
    var request_chain = ledger.Chain.init(2);
    try request_chain.accept(origin);
    try request_chain.accept(request);
    const request_bound = try request_chain.finish();
    catalog.generation = 1;
    catalog.allocator_watermark = 2;
    try std.testing.expectError(error.Invalid, ledger.validateAgainstLedger(&catalog, &request_bound));
}

test "stale_in_range_generation" {
    const origin = first();
    var chain = ledger.Chain.init(5);
    try chain.accept(origin);
    var current = origin;
    for (0..4) |_| {
        current = try next(current, .generation);
        try chain.accept(current);
    }
    const binding = try chain.finish();
    const stale = emptyCatalog(2, 5);
    try std.testing.expectError(error.Invalid, ledger.validateAgainstLedger(&stale, &binding));
    const exact = emptyCatalog(4, 5);
    try ledger.validateAgainstLedger(&exact, &binding);
    var forged = binding;
    forged.matched.next_generation = 0;
    try std.testing.expectError(error.Invalid, ledger.validateAgainstLedger(&exact, &forged));
}

test "id_bounds_use_matched_not_head" {
    const origin = first();
    const slot = try next(origin, .slot);
    const branch = try next(slot, .branch);
    const generation = try next(branch, .generation);
    const later_slot = try next(generation, .slot);
    const later_branch = try next(later_slot, .branch);
    var chain = ledger.Chain.init(4);
    for ([_]ledger.LedgerRecord{ origin, slot, branch, generation, later_slot, later_branch }) |record| try chain.accept(record);
    const binding = try chain.finish();
    var branches = [_]types.Branch{.{ .id = 1, .label = .{ .bytes = @constCast("branch") }, .tip_digest = @splat(0), .cursor_digest = @splat(0) }};
    var slots = [_]types.Slot{.{ .id = 1, .archived = false, .label = .{ .bytes = @constCast("slot") }, .manifest_digest = @splat(0), .branches = &branches }};
    var catalog = types.Catalog{ .generation = 1, .allocator_watermark = 4, .slots = &slots, .selection = null };
    try ledger.validateAgainstLedger(&catalog, &binding);
    slots[0].id = 2;
    try std.testing.expectError(error.Invalid, ledger.validateAgainstLedger(&catalog, &binding));
    slots[0].id = 3;
    try std.testing.expectError(error.Invalid, ledger.validateAgainstLedger(&catalog, &binding));
    slots[0].id = 1;
    branches[0].id = 2;
    try std.testing.expectError(error.Invalid, ledger.validateAgainstLedger(&catalog, &binding));
    branches[0].id = 3;
    try std.testing.expectError(error.Invalid, ledger.validateAgainstLedger(&catalog, &binding));
}
