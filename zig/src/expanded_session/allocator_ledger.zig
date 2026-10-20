const std = @import("std");
const types = @import("types.zig");
const catalog_codec = @import("catalog.zig");

const record_len: usize = 112;
const body_len: usize = 80;
const max_records: usize = 1_048_576;
const zero_digest: types.Digest = @splat(0);

pub const LedgerRecord = struct {
    ordinal: u64,
    previous_digest: types.Digest,
    next_generation: u64,
    next_slot_id: u64,
    next_branch_id: u64,
    next_request_id: u64,
    digest: types.Digest,
};

pub const AllocationKind = enum { generation, slot, branch, request };
/// A candidate next record; persistence and allocation commit belong to the store.
pub const Reservation = struct { allocated: u64, record: LedgerRecord };
/// Use only the unchanged result of Chain.finish after a complete trusted scan.
/// These public fields do not independently prove the chain from matched to head.
pub const LedgerBinding = struct {
    head: ?LedgerRecord,
    matched: LedgerRecord,
    predecessor: LedgerRecord,
    watermark: u64,
};

fn virtualOrigin() LedgerRecord {
    return .{
        .ordinal = 0,
        .previous_digest = zero_digest,
        .next_generation = 1,
        .next_slot_id = 1,
        .next_branch_id = 1,
        .next_request_id = 1,
        .digest = zero_digest,
    };
}

fn body(record: LedgerRecord) [body_len]u8 {
    var bytes: [body_len]u8 = undefined;
    @memcpy(bytes[0..8], "NYONAL01");
    std.mem.writeInt(u64, bytes[8..16], record.ordinal, .little);
    @memcpy(bytes[16..48], &record.previous_digest);
    std.mem.writeInt(u64, bytes[48..56], record.next_generation, .little);
    std.mem.writeInt(u64, bytes[56..64], record.next_slot_id, .little);
    std.mem.writeInt(u64, bytes[64..72], record.next_branch_id, .little);
    std.mem.writeInt(u64, bytes[72..80], record.next_request_id, .little);
    return bytes;
}

fn validCounters(record: LedgerRecord) bool {
    return record.ordinal != 0 and record.next_generation != 0 and
        record.next_slot_id != 0 and record.next_branch_id != 0 and record.next_request_id != 0;
}

pub fn encodeRecord(record: LedgerRecord) types.Error![record_len]u8 {
    if (!validCounters(record)) return error.Invalid;
    var bytes: [record_len]u8 = undefined;
    const prefix = body(record);
    @memcpy(bytes[0..body_len], &prefix);
    var digest: types.Digest = undefined;
    std.crypto.hash.sha2.Sha256.hash(&prefix, &digest, .{});
    if (!std.mem.eql(u8, &digest, &record.digest)) return error.Invalid;
    @memcpy(bytes[body_len..], &record.digest);
    return bytes;
}

pub fn decodeRecord(bytes: []const u8) types.Error!LedgerRecord {
    if (bytes.len != record_len or !std.mem.eql(u8, bytes[0..8], "NYONAL01")) return error.Invalid;
    var digest: types.Digest = undefined;
    std.crypto.hash.sha2.Sha256.hash(bytes[0..body_len], &digest, .{});
    if (!std.mem.eql(u8, &digest, bytes[body_len..])) return error.Invalid;
    const record = LedgerRecord{
        .ordinal = std.mem.readInt(u64, bytes[8..16], .little),
        .previous_digest = bytes[16..48].*,
        .next_generation = std.mem.readInt(u64, bytes[48..56], .little),
        .next_slot_id = std.mem.readInt(u64, bytes[56..64], .little),
        .next_branch_id = std.mem.readInt(u64, bytes[64..72], .little),
        .next_request_id = std.mem.readInt(u64, bytes[72..80], .little),
        .digest = digest,
    };
    if (!validCounters(record)) return error.Invalid;
    return record;
}

/// Computes a candidate reservation; callers must durably bind it before use.
pub fn reserve(current: LedgerRecord, kind: AllocationKind) types.Error!Reservation {
    _ = try encodeRecord(current);
    var record = current;
    record.ordinal = std.math.add(u64, current.ordinal, 1) catch return error.IdExhausted;
    record.previous_digest = current.digest;
    const counter = switch (kind) {
        .generation => &record.next_generation,
        .slot => &record.next_slot_id,
        .branch => &record.next_branch_id,
        .request => &record.next_request_id,
    };
    const allocated = counter.*;
    counter.* = std.math.add(u64, allocated, 1) catch return error.IdExhausted;
    const prefix = body(record);
    std.crypto.hash.sha2.Sha256.hash(&prefix, &record.digest, .{});
    return .{ .allocated = allocated, .record = record };
}

/// Initialize with Chain.init, then feed every decoded journal record through
/// accept, propagating all decode and accept failures.
/// A rejected record leaves the accepted prefix intact; it does not authorize
/// ignoring a corrupt or partial suffix.
pub const Chain = struct {
    target_watermark: u64,
    count: usize = 0,
    head: ?LedgerRecord = null,
    matched: ?LedgerRecord = null,
    predecessor: ?LedgerRecord = null,

    pub fn init(target_watermark: u64) Chain {
        return .{ .target_watermark = target_watermark };
    }

    pub fn accept(self: *Chain, record: LedgerRecord) types.Error!void {
        if (self.count == max_records) return error.TooLarge;
        _ = try encodeRecord(record);
        if (self.head) |head| {
            const expected = std.math.add(u64, head.ordinal, 1) catch return error.IdExhausted;
            if (record.ordinal != expected or !std.mem.eql(u8, &record.previous_digest, &head.digest)) return error.Invalid;
            if (record.next_generation < head.next_generation or
                record.next_slot_id < head.next_slot_id or
                record.next_branch_id < head.next_branch_id or
                record.next_request_id < head.next_request_id) return error.Invalid;
            if (record.next_generation == head.next_generation and
                record.next_slot_id == head.next_slot_id and
                record.next_branch_id == head.next_branch_id and
                record.next_request_id == head.next_request_id) return error.Invalid;
        } else {
            if (record.ordinal != 1 or !std.mem.eql(u8, &record.previous_digest, &zero_digest) or
                record.next_generation != 1 or record.next_slot_id != 1 or
                record.next_branch_id != 1 or record.next_request_id != 1) return error.Invalid;
        }
        if (record.ordinal == self.target_watermark) {
            self.matched = record;
            self.predecessor = self.head orelse virtualOrigin();
        }
        self.head = record;
        self.count += 1;
    }

    /// Call only after the transport establishes exact journal EOF, including
    /// no unread or partial suffix. This method cannot establish EOF itself.
    pub fn finish(self: *const Chain) types.Error!LedgerBinding {
        if (self.target_watermark == 0) return .{
            .head = self.head,
            .matched = virtualOrigin(),
            .predecessor = virtualOrigin(),
            .watermark = 0,
        };
        return .{
            .head = self.head,
            .matched = self.matched orelse return error.Invalid,
            .predecessor = self.predecessor orelse return error.Invalid,
            .watermark = self.target_watermark,
        };
    }
};

/// Requires an unchanged Chain.finish binding from a complete trusted scan.
/// Checks exact allocation provenance, not graph closure, persistence, or CAS.
pub fn validateAgainstLedger(catalog: *const types.Catalog, binding: *const LedgerBinding) types.Error!void {
    try catalog_codec.validate(catalog);
    if (catalog.allocator_watermark != binding.watermark or
        binding.matched.ordinal != binding.watermark) return error.Invalid;
    if (binding.head) |head| {
        if (binding.watermark > head.ordinal) return error.Invalid;
    } else if (binding.watermark != 0) return error.Invalid;
    if (binding.watermark == 0) {
        if (catalog.generation != 0 or catalog.slots.len != 0 or catalog.selection != null) return error.Invalid;
        return;
    }
    _ = try encodeRecord(binding.matched);
    if (binding.predecessor.ordinal == 0) {
        if (!std.meta.eql(binding.predecessor, virtualOrigin())) return error.Invalid;
    } else _ = try encodeRecord(binding.predecessor);
    if (binding.predecessor.ordinal != binding.watermark - 1 or
        !std.mem.eql(u8, &binding.matched.previous_digest, &binding.predecessor.digest)) return error.Invalid;
    const expected_generation = std.math.add(u64, binding.predecessor.next_generation, 1) catch return error.Invalid;
    if (binding.matched.next_generation != expected_generation or
        catalog.generation != binding.matched.next_generation - 1) return error.Invalid;
    for (catalog.slots) |slot| {
        if (slot.id >= binding.matched.next_slot_id) return error.Invalid;
        for (slot.branches) |branch| {
            if (branch.id >= binding.matched.next_branch_id) return error.Invalid;
        }
    }
}
