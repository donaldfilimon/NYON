const std = @import("std");
const catalog = @import("catalog.zig");
const types = @import("types.zig");
const limits = @import("limits.zig");
const t = std.testing;

fn nibble(byte: u8) u8 {
    return if (byte <= '9') byte - '0' else byte - 'a' + 10;
}

fn hexInto(dest: []u8, hex: []const u8) void {
    std.debug.assert(dest.len * 2 == hex.len);
    for (dest, 0..) |*byte, index| byte.* = nibble(hex[index * 2]) * 16 + nibble(hex[index * 2 + 1]);
}

// Handwritten offsets from the spec: header 0..48, body generation 48,
// watermark 56, slot count 64, selection presence 66. SHA is of body 48..67.
fn emptyFixture() [67]u8 {
    var bytes: [67]u8 = @splat(0);
    @memcpy(bytes[0..8], "NYONCAT1");
    std.mem.writeInt(u16, bytes[8..10], 1, .little);
    std.mem.writeInt(u32, bytes[12..16], 19, .little);
    hexInto(bytes[16..48], "d6fd62f5ce537d90ea3ea45841b17f34d727bcbc4128748cba14fb87c0ffd9d1");
    return bytes;
}

// One slot and branch: body is 220 bytes. Offsets are reviewed field positions,
// and this digest was computed from those literal fields using Python hashlib,
// independently of catalog.encode.
fn selectedFixture() [268]u8 {
    var bytes: [268]u8 = @splat(0);
    @memcpy(bytes[0..8], "NYONCAT1");
    std.mem.writeInt(u16, bytes[8..10], 1, .little);
    std.mem.writeInt(u32, bytes[12..16], 220, .little);
    hexInto(bytes[16..48], "ffc5f111417e1085bbd2f805803c9606da2b08197758a2b594b64ca5a31fcd09");
    std.mem.writeInt(u64, bytes[48..56], 1, .little);
    std.mem.writeInt(u64, bytes[56..64], 1, .little);
    std.mem.writeInt(u16, bytes[64..66], 1, .little);
    std.mem.writeInt(u64, bytes[66..74], 2, .little);
    std.mem.writeInt(u16, bytes[75..77], 1, .little);
    bytes[77] = 'S';
    @memset(bytes[78..110], 0x11);
    std.mem.writeInt(u16, bytes[110..112], 1, .little);
    std.mem.writeInt(u64, bytes[112..120], 3, .little);
    std.mem.writeInt(u16, bytes[120..122], 1, .little);
    bytes[122] = 'B';
    @memset(bytes[123..155], 0x22);
    @memset(bytes[155..187], 0x33);
    bytes[187] = 1;
    std.mem.writeInt(u64, bytes[188..196], 2, .little);
    std.mem.writeInt(u64, bytes[196..204], 3, .little);
    @memset(bytes[204..236], 0x33);
    @memset(bytes[236..268], 0x11);
    return bytes;
}

// Extend the literal slot through its first branch, then write a second branch
// at offsets 187..262. The selection is absent so ordering is the only defect
// in the descending variant.
fn twoBranchFixture() [263]u8 {
    const selected = selectedFixture();
    var bytes: [263]u8 = @splat(0);
    @memcpy(bytes[0..187], selected[0..187]);
    std.mem.writeInt(u16, bytes[110..112], 2, .little);
    std.mem.writeInt(u64, bytes[187..195], 4, .little);
    std.mem.writeInt(u16, bytes[195..197], 1, .little);
    bytes[197] = 'C';
    @memset(bytes[198..230], 0x44);
    @memset(bytes[230..262], 0x55);
    bytes[262] = 0;
    reframe(&bytes);
    return bytes;
}

fn rehash(bytes: []u8) void {
    std.crypto.hash.sha2.Sha256.hash(bytes[48..], bytes[16..48], .{});
}

fn reframe(bytes: []u8) void {
    std.mem.writeInt(u32, bytes[12..16], @intCast(bytes.len - 48), .little);
    rehash(bytes);
}

test "empty_literal" {
    const expected = emptyFixture();
    const empty = types.Catalog{
        .generation = 0,
        .allocator_watermark = 0,
        .slots = &.{},
        .selection = null,
    };
    const encoded = try catalog.encode(t.allocator, &empty);
    defer t.allocator.free(encoded);
    try t.expectEqualSlices(u8, &expected, encoded);
    var decoded = try catalog.decode(t.allocator, &expected);
    defer decoded.deinit(t.allocator);
    try t.expectEqual(@as(u64, 0), decoded.generation);
    try t.expectEqual(@as(u64, 0), decoded.allocator_watermark);
    try t.expectEqual(@as(usize, 0), decoded.slots.len);
    try t.expect(decoded.selection == null);
}

test "selected_literal and owned labels" {
    var input = selectedFixture();
    var decoded = try catalog.decode(t.allocator, &input);
    defer decoded.deinit(t.allocator);
    const encoded = try catalog.encode(t.allocator, &decoded);
    defer t.allocator.free(encoded);
    try t.expectEqualSlices(u8, &input, encoded);
    @memset(&input, 0);
    try t.expectEqualSlices(u8, "S", decoded.slots[0].label.bytes);
    try t.expectEqualSlices(u8, "B", decoded.slots[0].branches[0].label.bytes);
    try t.expectEqual(@as(u64, 2), decoded.selection.?.slot_id);
}

test "semantic_rejection" {
    const original = selectedFixture();
    const cases = [_]struct { offset: usize, value: u8 }{
        .{ .offset = 48, .value = 0 }, // zero generation with metadata
        .{ .offset = 56, .value = 0 }, // noninitial zero watermark
        .{ .offset = 66, .value = 0 }, // zero slot ID
        .{ .offset = 74, .value = 2 }, // archived tag
        .{ .offset = 75, .value = 0 }, // empty label
        .{ .offset = 75, .value = 129 }, // oversized label
        .{ .offset = 77, .value = 0xc0 }, // invalid UTF-8
        .{ .offset = 77, .value = 0x1f }, // C0 control
        .{ .offset = 112, .value = 0 }, // zero branch ID
        .{ .offset = 120, .value = 0 }, // empty branch label
        .{ .offset = 120, .value = 129 }, // oversized branch label
        .{ .offset = 122, .value = 0xc0 }, // invalid branch UTF-8
        .{ .offset = 122, .value = 0x1f }, // branch C0 control
        .{ .offset = 187, .value = 2 }, // selection tag
        .{ .offset = 188, .value = 4 }, // absent selected slot
        .{ .offset = 196, .value = 4 }, // absent branch
        .{ .offset = 204, .value = 4 }, // cursor mismatch
        .{ .offset = 236, .value = 4 }, // manifest mismatch
        .{ .offset = 64, .value = 65 }, // excessive slot count
        .{ .offset = 110, .value = 65 }, // excessive branch count
    };
    for (cases) |case| {
        var bytes = original;
        bytes[case.offset] = case.value;
        rehash(&bytes);
        try t.expectError(if (case.offset == 64 or case.offset == 110) error.TooLarge else error.Invalid, catalog.decode(t.allocator, &bytes));
    }
    var initial_bad = emptyFixture();
    initial_bad[56] = 1;
    rehash(&initial_bad);
    try t.expectError(error.Invalid, catalog.decode(t.allocator, &initial_bad));
    var bad_hash = original;
    bad_hash[77] = 'x';
    try t.expectError(error.Invalid, catalog.decode(t.allocator, &bad_hash));
    var bad_header = original;
    bad_header[8] = 2;
    try t.expectError(error.Invalid, catalog.decode(t.allocator, &bad_header));
    bad_header = original;
    bad_header[10] = 1;
    try t.expectError(error.Invalid, catalog.decode(t.allocator, &bad_header));
    try t.expectError(error.Invalid, catalog.decode(t.allocator, original[0 .. original.len - 1]));
    var trailing: [269]u8 = undefined;
    @memcpy(trailing[0..268], &original);
    trailing[268] = 0;
    try t.expectError(error.Invalid, catalog.decode(t.allocator, &trailing));
    const huge = try t.allocator.alloc(u8, limits.max_catalog_bytes + 1);
    defer t.allocator.free(huge);
    try t.expectError(error.TooLarge, catalog.decode(t.allocator, huge));
}

test "correctly framed truncated internal fields reach Reader.take" {
    const original = selectedFixture();
    // Label, branch tip, selection presence after a complete branch and slot,
    // selection branch ID, and selection manifest all end inside required fields.
    const lengths = [_]usize{ 77, 146, 187, 195, 267 };
    for (lengths) |length| {
        const bytes = try t.allocator.dupe(u8, original[0..length]);
        defer t.allocator.free(bytes);
        reframe(bytes);
        try t.expectEqual(@as(u32, @intCast(length - 48)), std.mem.readInt(u32, bytes[12..16], .little));
        var independent_digest: types.Digest = undefined;
        std.crypto.hash.sha2.Sha256.hash(bytes[48..], &independent_digest, .{});
        try t.expectEqualSlices(u8, &independent_digest, bytes[16..48]);
        try t.expectError(error.Invalid, catalog.decode(t.allocator, bytes));
    }
    const multi = try multiSlotFixture();
    defer t.allocator.free(multi);
    // First slot and the first branch of slot 2 are complete at offset 383.
    // Slot 2 still declares another branch, so its next ID read must fail.
    const nested = try t.allocator.dupe(u8, multi[0..383]);
    defer t.allocator.free(nested);
    reframe(nested);
    try t.expectError(error.Invalid, catalog.decode(t.allocator, nested));
}

test "correctly framed trailing body byte reaches final position guard" {
    const original = selectedFixture();
    var bytes: [269]u8 = undefined;
    @memcpy(bytes[0..original.len], &original);
    bytes[268] = 0xa5;
    reframe(&bytes);
    try t.expectEqual(@as(u32, 221), std.mem.readInt(u32, bytes[12..16], .little));
    var independent_digest: types.Digest = undefined;
    std.crypto.hash.sha2.Sha256.hash(bytes[48..], &independent_digest, .{});
    try t.expectEqualSlices(u8, &independent_digest, bytes[16..48]);
    try t.expectError(error.Invalid, catalog.decode(t.allocator, &bytes));
}

test "namespace duplicate and descending IDs" {
    var first_label = [_]u8{'A'};
    var second_label = [_]u8{'B'};
    var first_branch = [_]types.Branch{.{
        .id = 3,
        .label = .{ .bytes = &first_label },
        .tip_digest = @splat(1),
        .cursor_digest = @splat(2),
    }};
    var second_branch = [_]types.Branch{.{
        .id = 4,
        .label = .{ .bytes = &second_label },
        .tip_digest = @splat(3),
        .cursor_digest = @splat(4),
    }};
    var slots = [_]types.Slot{
        .{ .id = 2, .archived = false, .label = .{ .bytes = &first_label }, .manifest_digest = @splat(5), .branches = &first_branch },
        .{ .id = 5, .archived = false, .label = .{ .bytes = &second_label }, .manifest_digest = @splat(6), .branches = &second_branch },
    };
    const value = types.Catalog{ .generation = 1, .allocator_watermark = 1, .slots = &slots, .selection = null };
    const encoded = try catalog.encode(t.allocator, &value);
    defer t.allocator.free(encoded);
    // First slot starts at 66 and is 121 bytes; second slot starts at 187.
    // Its branch ID starts at 233.
    var malformed = try t.allocator.dupe(u8, encoded);
    defer t.allocator.free(malformed);
    std.mem.writeInt(u64, malformed[233..241], 3, .little);
    rehash(malformed);
    try t.expectError(error.Invalid, catalog.decode(t.allocator, malformed));
    std.mem.writeInt(u64, malformed[233..241], 4, .little);
    std.mem.writeInt(u64, malformed[187..195], 1, .little);
    rehash(malformed);
    try t.expectError(error.Invalid, catalog.decode(t.allocator, malformed));
    second_branch[0].id = 3;
    try t.expectError(error.Invalid, catalog.encode(t.allocator, &value));
}

test "distinct descending branch IDs reject decode and encode" {
    const ascending = twoBranchFixture();
    var decoded = try catalog.decode(t.allocator, &ascending);
    defer decoded.deinit(t.allocator);
    try t.expectEqual(@as(usize, 1), decoded.slots.len);
    try t.expectEqual(@as(usize, 2), decoded.slots[0].branches.len);
    try t.expect(decoded.selection == null);
    try t.expectEqual(@as(u64, 3), decoded.slots[0].branches[0].id);
    try t.expectEqual(@as(u64, 4), decoded.slots[0].branches[1].id);
    const encoded = try catalog.encode(t.allocator, &decoded);
    defer t.allocator.free(encoded);
    try t.expectEqualSlices(u8, &ascending, encoded);

    var descending = ascending;
    std.mem.writeInt(u64, descending[112..120], 4, .little);
    std.mem.writeInt(u64, descending[187..195], 3, .little);
    rehash(&descending);
    try t.expectError(error.Invalid, catalog.decode(t.allocator, &descending));

    decoded.slots[0].branches[0].id = 4;
    decoded.slots[0].branches[1].id = 3;
    try t.expectError(error.Invalid, catalog.encode(t.allocator, &decoded));
    try t.expectEqual(@as(u64, 4), decoded.slots[0].branches[0].id);
    try t.expectEqual(@as(u64, 3), decoded.slots[0].branches[1].id);
}

test "noninitial empty catalog is valid" {
    const value = types.Catalog{ .generation = 1, .allocator_watermark = 1, .slots = &.{}, .selection = null };
    const bytes = try catalog.encode(t.allocator, &value);
    defer t.allocator.free(bytes);
    var decoded = try catalog.decode(t.allocator, bytes);
    defer decoded.deinit(t.allocator);
    try t.expectEqual(@as(usize, 0), decoded.slots.len);
    try t.expectEqual(@as(u64, 1), decoded.generation);
}

test "capacity_edges and ordering" {
    var label128: [128]u8 = @splat('x');
    var branches: [64]types.Branch = undefined;
    var slots: [64]types.Slot = undefined;
    for (&branches, 0..) |*branch, index| branch.* = .{
        .id = index + 1,
        .label = .{ .bytes = &label128 },
        .tip_digest = @splat(1),
        .cursor_digest = @splat(2),
    };
    for (&slots, 0..) |*slot, index| slot.* = .{
        .id = index + 1,
        .archived = false,
        .label = .{ .bytes = &label128 },
        .manifest_digest = @splat(3),
        .branches = if (index == 0) &branches else &.{},
    };
    var value = types.Catalog{ .generation = 1, .allocator_watermark = 1, .slots = &slots, .selection = null };
    const bytes = try catalog.encode(t.allocator, &value);
    defer t.allocator.free(bytes);
    var decoded = try catalog.decode(t.allocator, bytes);
    defer decoded.deinit(t.allocator);
    try t.expectEqual(@as(usize, 64), decoded.slots.len);
    try t.expectEqual(@as(usize, 64), decoded.slots[0].branches.len);
    // Slot 0 starts at 66; its 128-byte label puts branch 0 at 239.
    // Branch records are 202 bytes, so branch 1's ID begins at 441.
    const malformed = try t.allocator.dupe(u8, bytes);
    defer t.allocator.free(malformed);
    std.mem.writeInt(u64, malformed[441..449], 1, .little);
    rehash(malformed);
    try t.expectError(error.Invalid, catalog.decode(t.allocator, malformed));
    slots[1].id = 1;
    try t.expectError(error.Invalid, catalog.encode(t.allocator, &value));
    slots[1].id = 2;
    branches[1].id = 1;
    try t.expectError(error.Invalid, catalog.encode(t.allocator, &value));
    branches[1].id = 2;
    value.allocator_watermark = 0;
    try t.expectError(error.Invalid, catalog.encode(t.allocator, &value));
}

test "maximum 64 slots and 4096 namespace branches" {
    var label128: [128]u8 = @splat('x');
    var branches: [64][64]types.Branch = undefined;
    var slots: [64]types.Slot = undefined;
    for (&slots, 0..) |*slot, slot_index| {
        for (&branches[slot_index], 0..) |*branch, branch_index| branch.* = .{
            .id = slot_index * 64 + branch_index + 1,
            .label = .{ .bytes = &label128 },
            .tip_digest = @splat(1),
            .cursor_digest = @splat(2),
        };
        slot.* = .{
            .id = slot_index + 1,
            .archived = false,
            .label = .{ .bytes = &label128 },
            .manifest_digest = @splat(3),
            .branches = &branches[slot_index],
        };
    }
    const value = types.Catalog{
        .generation = 1,
        .allocator_watermark = 1,
        .slots = &slots,
        .selection = .{
            .slot_id = 64,
            .branch_id = 4096,
            .cursor_digest = @splat(2),
            .manifest_digest = @splat(3),
        },
    };
    const bytes = try catalog.encode(t.allocator, &value);
    defer t.allocator.free(bytes);
    try t.expectEqual(@as(usize, 838611), bytes.len);
    var decoded = try catalog.decode(t.allocator, bytes);
    defer decoded.deinit(t.allocator);
    try t.expectEqual(@as(usize, 64), decoded.slots.len);
    for (decoded.slots) |slot| try t.expectEqual(@as(usize, 64), slot.branches.len);
    try t.expectEqual(@as(u64, 4096), decoded.slots[63].branches[63].id);
    try t.expectEqual(@as(u64, 4096), decoded.selection.?.branch_id);
    const roundtrip = try catalog.encode(t.allocator, &decoded);
    defer t.allocator.free(roundtrip);
    try t.expectEqualSlices(u8, bytes, roundtrip);
}

fn multiSlotFixture() ![]u8 {
    var first_label = [_]u8{'A'};
    var second_label = [_]u8{'B'};
    var first_branches = [_]types.Branch{
        .{ .id = 3, .label = .{ .bytes = &first_label }, .tip_digest = @splat(1), .cursor_digest = @splat(2) },
        .{ .id = 4, .label = .{ .bytes = &second_label }, .tip_digest = @splat(3), .cursor_digest = @splat(4) },
    };
    var second_branches = [_]types.Branch{
        .{ .id = 7, .label = .{ .bytes = &first_label }, .tip_digest = @splat(5), .cursor_digest = @splat(6) },
        .{ .id = 8, .label = .{ .bytes = &second_label }, .tip_digest = @splat(7), .cursor_digest = @splat(8) },
    };
    var slots = [_]types.Slot{
        .{ .id = 2, .archived = false, .label = .{ .bytes = &first_label }, .manifest_digest = @splat(9), .branches = &first_branches },
        .{ .id = 5, .archived = true, .label = .{ .bytes = &second_label }, .manifest_digest = @splat(10), .branches = &second_branches },
    };
    const value = types.Catalog{ .generation = 1, .allocator_watermark = 1, .slots = &slots, .selection = null };
    return catalog.encode(t.allocator, &value);
}

fn decodeMultiWithFailures(allocator: types.Allocator, input: []const u8) !void {
    var decoded = try catalog.decode(allocator, input);
    decoded.deinit(allocator);
}

test "decode_allocation_failures clean completed branches and slots" {
    const input = try multiSlotFixture();
    defer t.allocator.free(input);
    const unchanged = try t.allocator.dupe(u8, input);
    defer t.allocator.free(unchanged);
    // 1 slot array + two slot labels + two branch arrays + four branch labels.
    // Failure 4 frees a completed branch; failures 5..8 free a completed slot.
    for (0..9) |ordinal| {
        var failing = t.FailingAllocator.init(t.allocator, .{ .fail_index = ordinal });
        try t.expectError(error.OutOfMemory, catalog.decode(failing.allocator(), input));
        try t.expect(failing.has_induced_failure);
        try t.expectEqualSlices(u8, unchanged, input);
    }
    try t.checkAllAllocationFailures(t.allocator, decodeMultiWithFailures, .{input});
    var decoded = try catalog.decode(t.allocator, input);
    defer decoded.deinit(t.allocator);
    try t.expectEqual(@as(usize, 2), decoded.slots.len);
    try t.expectEqual(@as(usize, 2), decoded.slots[1].branches.len);
}

test "encode allocation failure preserves supplied catalog" {
    const expected = try multiSlotFixture();
    defer t.allocator.free(expected);
    var value = try catalog.decode(t.allocator, expected);
    defer value.deinit(t.allocator);
    var failing = t.FailingAllocator.init(t.allocator, .{ .fail_index = 0 });
    try t.expectError(error.OutOfMemory, catalog.encode(failing.allocator(), &value));
    try t.expect(failing.has_induced_failure);
    const after = try catalog.encode(t.allocator, &value);
    defer t.allocator.free(after);
    try t.expectEqualSlices(u8, expected, after);
}
