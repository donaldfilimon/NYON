const std = @import("std");
const types = @import("types.zig");
const catalog = @import("catalog.zig");
const ledger = @import("allocator_ledger.zig");
const metadata = @import("metadata.zig");
const t = std.testing;

const anchor_bytes = blk: {
    var bytes: [112]u8 = undefined;
    _ = std.fmt.hexToBytes(&bytes, "4e594f4e414c3031010000000000000000000000000000000000000000000000000000000000000000000000000000000100000000000000010000000000000001000000000000000100000000000000b13f204fbf670042e8a3c940e0d965a9c669da3c01a9b4b64111d2578f1efec7") catch unreachable;
    break :blk bytes;
};

fn anchor() ledger.LedgerRecord {
    return ledger.decodeRecord(&anchor_bytes) catch unreachable;
}

fn sample(allocator: types.Allocator) !types.Catalog {
    var slot_label = [_]u8{'S'};
    var other_label = [_]u8{'T'};
    var branch_label = [_]u8{'B'};
    var other_branch_label = [_]u8{'C'};
    var branches = [_]types.Branch{.{ .id = 1, .label = .{ .bytes = &branch_label }, .tip_digest = @splat(2), .cursor_digest = @splat(3) }};
    var other_branches = [_]types.Branch{.{ .id = 2, .label = .{ .bytes = &other_branch_label }, .tip_digest = @splat(4), .cursor_digest = @splat(5) }};
    var slots = [_]types.Slot{
        .{ .id = 1, .archived = false, .label = .{ .bytes = &slot_label }, .manifest_digest = @splat(1), .branches = &branches },
        .{ .id = 2, .archived = false, .label = .{ .bytes = &other_label }, .manifest_digest = @splat(6), .branches = &other_branches },
    };
    const value = types.Catalog{ .generation = 1, .allocator_watermark = 6, .slots = &slots, .selection = .{
        .slot_id = 1,
        .branch_id = 1,
        .cursor_digest = @splat(3),
        .manifest_digest = @splat(1),
    } };
    const bytes = try catalog.encode(allocator, &value);
    defer allocator.free(bytes);
    return catalog.decode(allocator, bytes);
}

fn encoded(allocator: types.Allocator, value: *const types.Catalog) ![]u8 {
    return catalog.encode(allocator, value);
}

test "stale_expected and invalid reservation preserve current" {
    var current = try sample(t.allocator);
    defer current.deinit(t.allocator);
    const before = try encoded(t.allocator, &current);
    defer t.allocator.free(before);
    const select = metadata.MetadataMutation{ .select = current.selection.? };
    try t.expectError(error.Conflict, metadata.prepare(t.allocator, &current, 0, 2, 7, select));
    try t.expectError(error.Invalid, metadata.prepare(t.allocator, &current, 1, 1, 7, select));
    try t.expectError(error.Invalid, metadata.prepare(t.allocator, &current, 1, 2, 6, select));
    const after = try encoded(t.allocator, &current);
    defer t.allocator.free(after);
    try t.expectEqualSlices(u8, before, after);
}

test "select_exact and malformed selection" {
    var current = try sample(t.allocator);
    defer current.deinit(t.allocator);
    const chosen = types.Selection{ .slot_id = 2, .branch_id = 2, .cursor_digest = @splat(5), .manifest_digest = @splat(6) };
    var proposal = try metadata.prepare(t.allocator, &current, 1, 2, 7, .{ .select = chosen });
    defer proposal.deinit(t.allocator);
    try t.expectEqualDeep(chosen, proposal.selection.?);
    try t.expectEqualDeep(@as(u64, 2), proposal.generation);
    try t.expectEqualDeep(@as(u64, 7), proposal.allocator_watermark);
    try t.expectEqualDeep(@as(u64, 1), current.selection.?.slot_id);
    var bad = chosen;
    bad.cursor_digest[0] ^= 1;
    try t.expectError(error.Invalid, metadata.prepare(t.allocator, &current, 1, 2, 7, .{ .select = bad }));
    bad = chosen;
    bad.branch_id = 1;
    try t.expectError(error.Invalid, metadata.prepare(t.allocator, &current, 1, 2, 7, .{ .select = bad }));
}

test "remove_selected and unselected" {
    var current = try sample(t.allocator);
    defer current.deinit(t.allocator);
    var selected_removed = try metadata.prepare(t.allocator, &current, 1, 2, 7, .{ .remove_slot = 1 });
    defer selected_removed.deinit(t.allocator);
    try t.expectEqual(@as(usize, 1), selected_removed.slots.len);
    try t.expectEqual(@as(u64, 2), selected_removed.slots[0].id);
    try t.expect(selected_removed.selection == null);
    var other_removed = try metadata.prepare(t.allocator, &current, 1, 2, 7, .{ .remove_slot = 2 });
    defer other_removed.deinit(t.allocator);
    try t.expectEqual(@as(usize, 1), other_removed.slots.len);
    try t.expectEqualDeep(current.selection, other_removed.selection);
    try t.expectError(error.Invalid, metadata.prepare(t.allocator, &current, 1, 2, 7, .{ .remove_slot = 3 }));
    try t.expectEqual(@as(usize, 2), current.slots.len);
}

test "rename_copies_input and all nested bytes" {
    var current = try sample(t.allocator);
    defer current.deinit(t.allocator);
    var slot_label = [_]u8{ 'N', 'e', 'w' };
    var branch_label = [_]u8{ 'N', 'o', 'd', 'e' };
    var renamed = try metadata.prepare(t.allocator, &current, 1, 2, 7, .{ .rename_slot = .{ .slot_id = 1, .label = &slot_label } });
    defer renamed.deinit(t.allocator);
    var branch_renamed = try metadata.prepare(t.allocator, &current, 1, 2, 7, .{ .rename_branch = .{ .slot_id = 1, .branch_id = 1, .label = &branch_label } });
    defer branch_renamed.deinit(t.allocator);
    @memset(&slot_label, 'x');
    @memset(&branch_label, 'x');
    try t.expectEqualStrings("New", renamed.slots[0].label.bytes);
    try t.expectEqualStrings("Node", branch_renamed.slots[0].branches[0].label.bytes);
    try t.expectEqualStrings("S", current.slots[0].label.bytes);
    try t.expectEqualStrings("B", current.slots[0].branches[0].label.bytes);
    try t.expect(@intFromPtr(renamed.slots[1].label.bytes.ptr) != @intFromPtr(current.slots[1].label.bytes.ptr));
    try t.expect(@intFromPtr(renamed.slots[1].branches[0].label.bytes.ptr) != @intFromPtr(current.slots[1].branches[0].label.bytes.ptr));
    try t.expectError(error.Invalid, metadata.prepare(t.allocator, &current, 1, 2, 7, .{ .rename_slot = .{ .slot_id = 1, .label = "" } }));
    try t.expectError(error.Invalid, metadata.prepare(t.allocator, &current, 1, 2, 7, .{ .rename_branch = .{ .slot_id = 2, .branch_id = 1, .label = "X" } }));
}

test "rename rejects invalid labels before allocation" {
    var current = try sample(t.allocator);
    defer current.deinit(t.allocator);
    const before = try encoded(t.allocator, &current);
    defer t.allocator.free(before);
    const label129: [129]u8 = @splat('X');
    const oversized: [1024 * 1024]u8 = @splat('X');
    const cases = [_][]const u8{ "", &label129, &oversized, &[_]u8{0xc0}, &[_]u8{0x1f} };
    for (cases) |label| {
        const mutations = [_]metadata.MetadataMutation{
            .{ .rename_slot = .{ .slot_id = 1, .label = label } },
            .{ .rename_branch = .{ .slot_id = 1, .branch_id = 1, .label = label } },
        };
        for (mutations) |mutation| {
            var failing = t.FailingAllocator.init(t.allocator, .{ .fail_index = 0 });
            try t.expectError(error.Invalid, metadata.prepare(failing.allocator(), &current, 1, 2, 7, mutation));
            try t.expect(!failing.has_induced_failure);
            var buffer: [1024]u8 = undefined;
            var fixed = std.heap.FixedBufferAllocator.init(&buffer);
            try t.expectError(error.Invalid, metadata.prepare(fixed.allocator(), &current, 1, 2, 7, mutation));
        }
    }
    const after = try encoded(t.allocator, &current);
    defer t.allocator.free(after);
    try t.expectEqualSlices(u8, before, after);
}

test "rename accepts 128-byte labels and owns input" {
    var current = try sample(t.allocator);
    defer current.deinit(t.allocator);
    var slot_label: [128]u8 = @splat('S');
    var branch_label: [128]u8 = @splat('B');
    var slot_proposal = try metadata.prepare(t.allocator, &current, 1, 2, 7, .{
        .rename_slot = .{ .slot_id = 1, .label = &slot_label },
    });
    defer slot_proposal.deinit(t.allocator);
    var branch_proposal = try metadata.prepare(t.allocator, &current, 1, 2, 7, .{
        .rename_branch = .{ .slot_id = 1, .branch_id = 1, .label = &branch_label },
    });
    defer branch_proposal.deinit(t.allocator);
    @memset(&slot_label, 'x');
    @memset(&branch_label, 'x');
    try t.expectEqual(@as(usize, 128), slot_proposal.slots[0].label.bytes.len);
    try t.expectEqual(@as(usize, 128), branch_proposal.slots[0].branches[0].label.bytes.len);
    for (slot_proposal.slots[0].label.bytes) |byte| try t.expectEqual(@as(u8, 'S'), byte);
    for (branch_proposal.slots[0].branches[0].label.bytes) |byte| try t.expectEqual(@as(u8, 'B'), byte);
}

test "archive_retains_capacity" {
    var current = try sample(t.allocator);
    defer current.deinit(t.allocator);
    var proposal = try metadata.prepare(t.allocator, &current, 1, 2, 7, .{ .archive_slot = .{ .slot_id = 1, .archived = true } });
    defer proposal.deinit(t.allocator);
    try t.expect(proposal.slots[0].archived);
    try t.expectEqual(@as(usize, current.slots.len), proposal.slots.len);
    try t.expectEqualDeep(current.selection, proposal.selection);
    try t.expect(!current.slots[0].archived);
    try t.expectError(error.Invalid, metadata.prepare(t.allocator, &current, 1, 2, 7, .{ .archive_slot = .{ .slot_id = 3, .archived = true } }));
}

fn prepareWithFailures(allocator: types.Allocator, current: *const types.Catalog) !void {
    var proposal = try metadata.prepare(allocator, current, 1, 2, 7, .{ .rename_branch = .{ .slot_id = 2, .branch_id = 2, .label = "Changed" } });
    proposal.deinit(allocator);
}

test "prepare_allocation_failures" {
    var current = try sample(t.allocator);
    defer current.deinit(t.allocator);
    const before = try encoded(t.allocator, &current);
    defer t.allocator.free(before);
    // Slot array, two slot labels, two branch arrays, two branch labels.
    for (0..7) |failure_index| {
        var failing = t.FailingAllocator.init(t.allocator, .{ .fail_index = failure_index });
        try t.expectError(error.OutOfMemory, metadata.prepare(failing.allocator(), &current, 1, 2, 7, .{
            .rename_branch = .{ .slot_id = 2, .branch_id = 2, .label = "Changed" },
        }));
        try t.expect(failing.has_induced_failure);
    }
    try t.checkAllAllocationFailures(t.allocator, prepareWithFailures, .{&current});
    const after = try encoded(t.allocator, &current);
    defer t.allocator.free(after);
    try t.expectEqualSlices(u8, before, after);
}

test "forged_reservation_binding" {
    var current = try sample(t.allocator);
    defer current.deinit(t.allocator);
    const mutation = metadata.MetadataMutation{ .archive_slot = .{ .slot_id = 1, .archived = true } };
    var proposal = try metadata.prepare(t.allocator, &current, 1, 2, 7, mutation);
    defer proposal.deinit(t.allocator);

    var request_chain = ledger.Chain.init(7);
    var record = anchor();
    try request_chain.accept(record);
    for ([_]ledger.AllocationKind{ .slot, .branch, .slot, .branch, .generation }) |kind| {
        record = (try ledger.reserve(record, kind)).record;
        try request_chain.accept(record);
    }
    const request = try ledger.reserve(record, .request);
    try request_chain.accept(request.record);
    const forged_binding = try request_chain.finish();
    try t.expectError(error.Invalid, ledger.validateAgainstLedger(&proposal, &forged_binding));

    var generation_chain = ledger.Chain.init(7);
    record = anchor();
    try generation_chain.accept(record);
    for ([_]ledger.AllocationKind{ .slot, .branch, .slot, .branch, .generation }) |kind| {
        record = (try ledger.reserve(record, kind)).record;
        try generation_chain.accept(record);
    }
    const reservation = try ledger.reserve(record, .generation);
    try t.expectEqual(@as(u64, 2), reservation.allocated);
    try generation_chain.accept(reservation.record);
    const exact = try generation_chain.finish();
    try ledger.validateAgainstLedger(&proposal, &exact);
    var unallocated_generation = proposal;
    unallocated_generation.generation = 3;
    try t.expectError(error.Invalid, ledger.validateAgainstLedger(&unallocated_generation, &exact));
    var unallocated_watermark = proposal;
    unallocated_watermark.allocator_watermark = 8;
    try t.expectError(error.Invalid, ledger.validateAgainstLedger(&unallocated_watermark, &exact));
    var unallocated_slot = proposal;
    var forged_slots = [_]types.Slot{ proposal.slots[0], proposal.slots[1] };
    forged_slots[1].id = 3;
    unallocated_slot.slots = &forged_slots;
    try t.expectError(error.Invalid, ledger.validateAgainstLedger(&unallocated_slot, &exact));
    var unallocated_branch = proposal;
    var forged_branches = [_]types.Branch{proposal.slots[1].branches[0]};
    forged_branches[0].id = 3;
    forged_slots[1] = proposal.slots[1];
    forged_slots[1].branches = &forged_branches;
    unallocated_branch.slots = &forged_slots;
    try t.expectError(error.Invalid, ledger.validateAgainstLedger(&unallocated_branch, &exact));
    try t.expectEqual(@as(u64, 1), current.generation);
}
