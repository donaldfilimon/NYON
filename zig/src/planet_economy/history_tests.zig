const std = @import("std");
const types = @import("types.zig");
const history = @import("history.zig");
const wire_vectors = @import("wire_vectors.zig");
const state = @import("state.zig");
const command = @import("command.zig");

fn eventReceipt(event: types.ReceiptEvent) types.Receipt {
    return .{ .header = .{ .tick = @fromBackingInt(3), .ordinal = 0, .provenance = .autonomous, .sequence = null }, .event = event };
}

fn expectPayload(event: types.ReceiptEvent, expected: []const u8) !void {
    var output: [1024]u8 = undefined;
    const value = eventReceipt(event);
    const length = try encode(&value, &output);
    try std.testing.expectEqual(@as(u8, @backingInt(std.meta.activeTag(event))), output[16]);
    try std.testing.expectEqual(@as(usize, 29) + expected.len, length);
    try std.testing.expectEqualSlices(u8, expected, output[29..length]);
}

fn encode(receipt: *const types.Receipt, output: []u8) !usize {
    var encoder = history.Encoder.init(receipt, output);
    while (true) {
        var budget: types.WorkBudget = .{ .remaining = 1 };
        if (try encoder.advance(&budget) == .done) return encoder.len();
    }
}

fn chained(bytes: []const u8) ![32]u8 {
    var chain = history.Chain.init();
    try chain.begin(bytes);
    while (true) {
        var budget: types.WorkBudget = .{ .remaining = 1 };
        if (try chain.advance(&budget) == .done) return chain.head();
    }
}

test "pe/history/independent-wire" {
    var output: [1024]u8 = undefined;
    const genesis = types.Receipt{ .header = .{ .tick = @fromBackingInt(0), .ordinal = 0, .provenance = .autonomous, .sequence = null }, .event = .{ .genesis = .{ .seed = 2, .side = 4 } } };
    const genesis_len = try encode(&genesis, &output);
    try std.testing.expectEqual(@as(usize, 80), genesis_len);
    try std.testing.expectEqualSlices(u8, wire_vectors.G0.bytes, output[0..genesis_len]);
    try std.testing.expectEqualDeep(wire_vectors.G0.chain, try chained(output[0..genesis_len]));

    const input = types.Receipt{ .header = .{ .tick = @fromBackingInt(1), .ordinal = 0, .provenance = .autonomous, .sequence = null }, .event = .{ .tick_input = .{ .target_tick = @fromBackingInt(1), .expected_revision = @fromBackingInt(0), .previous_policy = null, .current_policy = .{ .role = .creator, .civilization = null }, .envelope_count = 1 } } };
    const input_len = try encode(&input, &output);
    try std.testing.expectEqual(@as(usize, 50), input_len);
    try std.testing.expectEqualSlices(u8, wire_vectors.I1.bytes, output[0..input_len]);
    try std.testing.expectEqualDeep(wire_vectors.I1.chain, try chained(output[0..input_len]));

    const envelope = wire_vectors.A2.bytes[39..68];
    const accepted = types.Receipt{ .header = .{ .tick = @fromBackingInt(2), .ordinal = 1, .provenance = .creator, .sequence = @fromBackingInt(2) }, .event = .{ .accepted = .{ .envelope = .{ .bytes = envelope }, .effect = .{ .grant = .{ .food_minted = 1, .material_minted = 0, .population_minted = 0 } } } } };
    const accepted_len = try encode(&accepted, &output);
    try std.testing.expectEqual(@as(usize, 77), accepted_len);
    try std.testing.expectEqualSlices(u8, wire_vectors.A2.bytes, output[0..accepted_len]);
    try std.testing.expectEqualDeep(wire_vectors.A2.chain, try chained(output[0..accepted_len]));

    const rejected = types.Receipt{ .header = accepted.header, .event = .{ .rejected = .{ .envelope = .{ .bytes = envelope }, .reason = .CreatorForbidden, .offending_index = null } } };
    const rejected_len = try encode(&rejected, &output);
    try std.testing.expectEqual(@as(usize, 71), rejected_len);
    try std.testing.expectEqualSlices(u8, wire_vectors.R2.bytes, output[0..rejected_len]);
    try std.testing.expectEqualDeep(wire_vectors.R2.chain, try chained(output[0..rejected_len]));
}

test "pe/history/all-tags autonomous event payload fields" {
    try expectPayload(.{ .arrived = .{ .shipment = @fromBackingInt(1), .source = @fromBackingInt(2), .destination = @fromBackingInt(3), .resource = .material, .quantity = 4, .destination_stock_after = 5, .destination_reservation_after = 6 } }, &.{ 1, 0, 0, 0, 0, 0, 0, 0, 2, 0, 0, 0, 0, 0, 0, 0, 3, 0, 0, 0, 0, 0, 0, 0, 1, 4, 0, 5, 0, 0, 0, 6, 0, 0, 0 });
    try expectPayload(.{ .completed = .{ .building = @fromBackingInt(1), .settlement = @fromBackingInt(2), .kind = .quarry } }, &.{ 1, 0, 0, 0, 0, 0, 0, 0, 2, 0, 0, 0, 0, 0, 0, 0, 1 });
    try expectPayload(.{ .produced = .{ .building = @fromBackingInt(1), .settlement = @fromBackingInt(2), .workers = 3, .resource = .food, .quantity = 4, .stock_after = 5 } }, &.{ 1, 0, 0, 0, 0, 0, 0, 0, 2, 0, 0, 0, 0, 0, 0, 0, 3, 0, 4, 0, 0, 0, 5, 0, 0, 0 });
    try expectPayload(.{ .idle = .{ .building = @fromBackingInt(1), .settlement = @fromBackingInt(2), .reason = .output_capacity, .workers_needed = 3, .workers_available = 4, .resource = .material, .quantity = 5, .stock_before = 6, .incoming_reserved = 7 } }, &.{ 1, 0, 0, 0, 0, 0, 0, 0, 2, 0, 0, 0, 0, 0, 0, 0, 1, 3, 4, 1, 5, 0, 0, 0, 6, 0, 0, 0, 7, 0, 0, 0 });
    try expectPayload(.{ .supplied = .{ .settlement = @fromBackingInt(1), .needed = 2, .consumed = 3, .shortage = 4, .food_after = 5 } }, &.{ 1, 0, 0, 0, 0, 0, 0, 0, 2, 3, 4, 5, 0, 0, 0 });
    try expectPayload(.{ .demography = .{ .settlement = @fromBackingInt(1), .kind = .housing_blocked, .population_before = 2, .population_after = 3, .housing = 4, .hunger_after = 5, .fed_streak_after = 6 } }, &.{ 1, 0, 0, 0, 0, 0, 0, 0, 2, 2, 3, 4, 5, 6 });
    try expectPayload(.{ .tick_complete = .{ .revision = @fromBackingInt(1), .last_sequence = @fromBackingInt(2), .accepted = 3, .rejected = 4, .settlements = 5, .buildings = 6, .active_shipments = 7 } }, &.{ 1, 0, 0, 0, 0, 0, 0, 0, 2, 0, 0, 0, 0, 0, 0, 0, 3, 0, 4, 0, 5, 0, 6, 0, 7, 0 });
}

test "pe/history/all-tags seven accepted effect layouts" {
    const cell_a: types.CellId = .{ .planet = @fromBackingInt(1), .local = 9 };
    const cell_b: types.CellId = .{ .planet = @fromBackingInt(1), .local = 10 };
    const footprint: types.Footprint = .{ .len = 2, .cells = .{ cell_a, cell_b } };
    const cases = .{
        .{ types.AcceptedEffect{ .author_settlement = .{ .settlement = @fromBackingInt(1), .food_minted = 2, .material_minted = 3, .population_minted = 4 } }, &[_]u8{ 1, 0, 0, 0, 0, 0, 0, 0, 2, 0, 0, 0, 3, 0, 0, 0, 4 } },
        .{ types.AcceptedEffect{ .grant = .{ .food_minted = 2, .material_minted = 3, .population_minted = 4 } }, &[_]u8{ 2, 0, 0, 0, 3, 0, 0, 0, 4 } },
        .{ types.AcceptedEffect{ .author_build = .{ .building = @fromBackingInt(1), .footprint = footprint, .material_waived = 2, .ticks_waived = 3 } }, &[_]u8{ 1, 0, 0, 0, 0, 0, 0, 0, 2, 0, 1, 0, 0, 0, 9, 0, 0, 0, 0, 0, 0, 0, 1, 0, 0, 0, 10, 0, 0, 0, 0, 0, 0, 0, 2, 0, 0, 0, 3 } },
        .{ types.AcceptedEffect{ .author_ship = .{ .shipment = @fromBackingInt(1), .transferred = 2, .minted = 3, .departure_tick = @fromBackingInt(4), .arrival_tick = @fromBackingInt(5), .reserved = 6 } }, &[_]u8{ 1, 0, 0, 0, 0, 0, 0, 0, 2, 0, 3, 0, 4, 0, 0, 0, 0, 0, 0, 0, 5, 0, 0, 0, 0, 0, 0, 0, 6, 0 } },
        .{ types.AcceptedEffect{ .found = .{ .settlement = @fromBackingInt(1), .food_moved = 2, .material_spent = 3, .population_moved = 4 } }, &[_]u8{ 1, 0, 0, 0, 0, 0, 0, 0, 2, 0, 0, 0, 3, 0, 0, 0, 4 } },
        .{ types.AcceptedEffect{ .build = .{ .building = @fromBackingInt(1), .footprint = footprint, .material_spent = 2, .remaining_ticks = 3 } }, &[_]u8{ 1, 0, 0, 0, 0, 0, 0, 0, 2, 0, 1, 0, 0, 0, 9, 0, 0, 0, 0, 0, 0, 0, 1, 0, 0, 0, 10, 0, 0, 0, 0, 0, 0, 0, 2, 0, 0, 0, 3 } },
        .{ types.AcceptedEffect{ .ship = .{ .shipment = @fromBackingInt(1), .transferred = 2, .reserved = 3, .departure_tick = @fromBackingInt(4), .arrival_tick = @fromBackingInt(5) } }, &[_]u8{ 1, 0, 0, 0, 0, 0, 0, 0, 2, 0, 3, 0, 4, 0, 0, 0, 0, 0, 0, 0, 5, 0, 0, 0, 0, 0, 0, 0 } },
    };
    inline for (cases) |case| {
        const effect = case[0];
        var envelope: [11]u8 = .{ 2, 0, 0, 0, 0, 0, 0, 0, 0, 0, @backingInt(std.meta.activeTag(effect)) };
        const value = types.Receipt{ .header = .{ .tick = @fromBackingInt(2), .ordinal = 1, .provenance = .creator, .sequence = @fromBackingInt(2) }, .event = .{ .accepted = .{ .envelope = .{ .bytes = &envelope }, .effect = effect } } };
        var output: [1024]u8 = undefined;
        const length = try encode(&value, &output);
        try std.testing.expectEqualSlices(u8, case[1], output[50..length]);
    }
}

test "pe/history/bounded-tail zero budget and 65-cell envelope" {
    const value = eventReceipt(.{ .genesis = .{ .seed = 2, .side = 4 } });
    var output: [1024]u8 = @splat(0xA5);
    var encoder = history.Encoder.init(&value, &output);
    var zero: types.WorkBudget = .{ .remaining = 0 };
    try std.testing.expectEqual(types.Step.pending, try encoder.advance(&zero));
    try std.testing.expectEqual(@as(usize, 0), encoder.len());
    try std.testing.expectEqual(@as(usize, 0), encoder.pc);
    try std.testing.expectEqual(@as(u8, 0xA5), output[0]);
    var chain = history.Chain.init();
    try chain.begin(wire_vectors.G0.bytes);
    const old_head = chain.head();
    try std.testing.expectEqual(types.Step.pending, try chain.advance(&zero));
    try std.testing.expectEqual(@as(usize, 0), chain.cursor);
    try std.testing.expectEqual(@as(u64, 0), chain.hash.total_len);
    try std.testing.expectEqualDeep(old_head, chain.head());

    var cells: [65]types.CellId = undefined;
    for (&cells, 0..) |*cell, index| cell.* = .{ .planet = @fromBackingInt(1), .local = index };
    var envelope_bytes: [1024]u8 = undefined;
    const envelope = try command.encodeEnvelope(.{ .sequence = @fromBackingInt(2), .provenance = .creator, .actor = null }, .{ .author_ship = .{ .source = @fromBackingInt(1), .destination = @fromBackingInt(2), .resource = .food, .amount = 1, .path = &cells, .cargo_source = .mint, .economic_bypass = true } }, &envelope_bytes);
    try std.testing.expectEqual(@as(usize, 814), envelope.bytes.len);
    const large = types.Receipt{ .header = .{ .tick = @fromBackingInt(2), .ordinal = 1, .provenance = .creator, .sequence = @fromBackingInt(2) }, .event = .{ .accepted = .{ .envelope = envelope, .effect = .{ .author_ship = .{ .shipment = @fromBackingInt(1), .transferred = 0, .minted = 1, .departure_tick = @fromBackingInt(2), .arrival_tick = @fromBackingInt(66), .reserved = 1 } } } } };
    const length = try encode(&large, &output);
    try std.testing.expectEqual(@as(usize, 883), length);
    try std.testing.expectEqualSlices(u8, envelope.bytes, output[39..853]);
}

fn makeAuthority(history_capacity: u16) !@import("job.zig").Authority {
    const job = @import("job.zig");
    var builder = try job.beginCreate(std.testing.allocator, .{ .seed = 2, .side = 4, .capacities = .{ .history_receipts = history_capacity } });
    defer builder.deinit();
    while (true) {
        var budget: types.WorkBudget = .{ .remaining = 256 };
        switch (job.pollCreate(&builder, &budget)) {
            .pending => {},
            .ready => return job.takeAuthority(&builder),
            .failed => |failure| return failure,
            else => return error.UnexpectedProgress,
        }
    }
}

test "pe/history/bounded-tail HistoryFull preserves visible metadata" {
    var authority = try makeAuthority(1);
    defer authority.deinit();
    var candidate = state.Candidate.begin(&authority.storage);
    while (true) {
        var budget: types.WorkBudget = .{ .remaining = 256 };
        if (try candidate.advancePrepare(&budget) == .done) break;
    }
    var one: types.WorkBudget = .{ .remaining = 1 };
    const genesis = authority.storage.genesis.?;
    try std.testing.expectEqual(types.Step.done, try candidate.stage(genesis, &one));
    one.remaining = 1;
    try std.testing.expectEqual(types.Step.done, try candidate.stage(genesis, &one));
    const before = authority.storage.rootMeta(authority.storage.active);
    const work_before = authority.storage.work_units;
    var finalizer = history.Finalizer.init(&candidate);
    var zero: types.WorkBudget = .{ .remaining = 0 };
    try std.testing.expectEqual(types.Step.pending, try finalizer.advance(&zero));
    try std.testing.expectEqual(@as(u16, 0), finalizer.index);
    try std.testing.expectEqual(.start, finalizer.step);
    try std.testing.expectEqual(work_before, authority.storage.work_units);
    var budget: types.WorkBudget = .{ .remaining = 1 };
    try std.testing.expectError(error.HistoryFull, finalizer.advance(&budget));
    try std.testing.expectEqualDeep(before, authority.storage.rootMeta(authority.storage.active));
}

test "pe/history/bounded-tail finalizer stages invisible bytes and candidate head" {
    var authority = try makeAuthority(4096);
    defer authority.deinit();
    var candidate = state.Candidate.begin(&authority.storage);
    while (true) {
        var budget: types.WorkBudget = .{ .remaining = 256 };
        if (try candidate.advancePrepare(&budget) == .done) break;
    }
    const visible_before = authority.storage.rootMeta(authority.storage.active);
    try std.testing.expectEqualDeep(wire_vectors.G0.chain, visible_before.chain_head);
    var one: types.WorkBudget = .{ .remaining = 1 };
    try std.testing.expectEqual(types.Step.done, try candidate.stage(authority.storage.genesis.?, &one));
    var finalizer = history.Finalizer.init(&candidate);
    const units_before = authority.storage.work_units;
    var consumed: u64 = 0;
    while (true) {
        one.remaining = 1;
        const step = try finalizer.advance(&one);
        consumed += 1 - one.remaining;
        if (step == .done) break;
    }
    try std.testing.expectEqual(consumed, authority.storage.work_units - units_before);
    try std.testing.expectEqualDeep(visible_before, authority.storage.rootMeta(authority.storage.active));
    try std.testing.expectEqual(visible_before.history_used + 1, candidate.meta().history_used);
    var expected_input: [116]u8 = undefined;
    @memcpy(expected_input[0..32], &wire_vectors.G0.chain);
    std.mem.writeInt(u32, expected_input[32..36], 80, .little);
    @memcpy(expected_input[36..], wire_vectors.G0.bytes);
    var expected_head: [32]u8 = undefined;
    std.crypto.hash.sha2.Sha256.hash(&expected_input, &expected_head, .{});
    try std.testing.expectEqualDeep(expected_head, candidate.meta().chain_head);
    try std.testing.expectEqual(@as(u32, 80), authority.storage.reservation.history_meta[visible_before.history_used]);
    try std.testing.expectEqualSlices(u8, wire_vectors.G0.bytes, authority.storage.reservation.history[visible_before.history_used][0..80]);
    candidate.publish();
    try std.testing.expectEqual(visible_before.history_used + 1, authority.storage.rootMeta(authority.storage.active).history_used);
}

test "pe/history/rejected autonomous command survives real admission and finalization" {
    var authority = try makeAuthority(4096);
    defer authority.deinit();
    var candidate = state.Candidate.begin(&authority.storage);
    while (true) {
        var budget: types.WorkBudget = .{ .remaining = 256 };
        if (try candidate.advancePrepare(&budget) == .done) break;
    }
    var first_bytes: [1024]u8 = undefined;
    var second_bytes: [1024]u8 = undefined;
    const first = try command.encodeEnvelope(.{ .sequence = @fromBackingInt(1), .provenance = .creator, .actor = null }, .{ .author_settlement = .{ .owner = @fromBackingInt(7), .anchor = .{ .planet = @fromBackingInt(1), .local = 70 }, .population = 4, .food = 12, .material = 20, .economic_bypass = true } }, &first_bytes);
    const second = try command.encodeEnvelope(.{ .sequence = @fromBackingInt(2), .provenance = .autonomous, .actor = null }, .{ .grant = .{ .settlement = @fromBackingInt(1), .food = 1, .material = 0, .population = 0, .economic_bypass = true } }, &second_bytes);
    const envelopes = [_]types.Envelope{ first, second };
    var cursor = command.Cursor.init(.{ .role = .creator, .civilization = null }, &envelopes, @fromBackingInt(0), @fromBackingInt(1));
    while (true) {
        var budget: types.WorkBudget = .{ .remaining = 256 };
        if (try cursor.advance(&candidate, &budget) == .done) break;
    }
    try std.testing.expectEqual(@as(u16, 1), cursor.accepted);
    try std.testing.expectEqual(@as(u16, 1), cursor.rejected);
    try std.testing.expectEqual(@as(u16, 2), authority.storage.staged_count);
    try std.testing.expectEqual(types.RejectReason.InternalCommandOnly, authority.storage.logicalReceipt(1).event.rejected.reason);
    var finalizer = history.Finalizer.init(&candidate);
    while (true) {
        var budget: types.WorkBudget = .{ .remaining = 256 };
        if (try finalizer.advance(&budget) == .done) break;
    }
    try std.testing.expectEqual(@as(u16, 3), candidate.meta().history_used);
    const recorded_first = &authority.storage.reservation.history[1];
    const recorded_second = &authority.storage.reservation.history[2];
    try std.testing.expectEqual(@as(u8, 2), recorded_first[16]);
    try std.testing.expectEqual(@as(u8, 3), recorded_second[16]);
    try std.testing.expectEqual(@as(u8, 2), recorded_second[27]);
    try std.testing.expectEqual(@as(u8, 1), recorded_second[28]);
    try std.testing.expectEqual(@as(u64, 2), std.mem.readInt(u64, recorded_second[29..37], .little));
    try std.testing.expectEqual(@as(u16, @intCast(first.bytes.len)), std.mem.readInt(u16, recorded_first[37..39], .little));
    try std.testing.expectEqual(@as(u16, @intCast(second.bytes.len)), std.mem.readInt(u16, recorded_second[37..39], .little));
    try std.testing.expectEqualSlices(u8, first.bytes, recorded_first[39 .. 39 + first.bytes.len]);
    try std.testing.expectEqualSlices(u8, second.bytes, recorded_second[39 .. 39 + second.bytes.len]);
    try std.testing.expectEqual(@as(u16, 6), std.mem.readInt(u16, recorded_second[39 + second.bytes.len ..][0..2], .little));
}

test "pe/history/command receipt rejects malformed or mismatched headers" {
    var envelope_bytes: [1024]u8 = undefined;
    const envelope = try command.encodeEnvelope(.{ .sequence = @fromBackingInt(2), .provenance = .autonomous, .actor = null }, .{ .grant = .{ .settlement = @fromBackingInt(1), .food = 1, .material = 0, .population = 0, .economic_bypass = true } }, &envelope_bytes);
    var output: [1024]u8 = undefined;
    const matching = types.ReceiptHeader{ .tick = @fromBackingInt(2), .ordinal = 1, .provenance = .autonomous, .sequence = @fromBackingInt(2) };
    const event: types.ReceiptEvent = .{ .rejected = .{ .envelope = envelope, .reason = .InternalCommandOnly, .offending_index = null } };
    var wrong_sequence = types.Receipt{ .header = matching, .event = event };
    wrong_sequence.header.sequence = @fromBackingInt(3);
    try std.testing.expectError(error.InvalidEnvelope, encode(&wrong_sequence, &output));
    var wrong_provenance = types.Receipt{ .header = matching, .event = event };
    wrong_provenance.header.provenance = .creator;
    try std.testing.expectError(error.InvalidEnvelope, encode(&wrong_provenance, &output));
    const malformed = types.Receipt{ .header = matching, .event = .{ .rejected = .{ .envelope = .{ .bytes = envelope.bytes[0..10] }, .reason = .InternalCommandOnly, .offending_index = null } } };
    try std.testing.expectError(error.InvalidEnvelope, encode(&malformed, &output));
    const impossible_accepted = types.Receipt{ .header = matching, .event = .{ .accepted = .{ .envelope = envelope, .effect = .{ .grant = .{ .food_minted = 1, .material_minted = 0, .population_minted = 0 } } } } };
    try std.testing.expectError(error.InvalidEnvelope, encode(&impossible_accepted, &output));
    const impossible_noncommand = types.Receipt{ .header = matching, .event = .{ .supplied = .{ .settlement = @fromBackingInt(1), .needed = 1, .consumed = 1, .shortage = 0, .food_after = 0 } } };
    try std.testing.expectError(error.InvariantViolation, encode(&impossible_noncommand, &output));
}

test "pe/history/budget accounting includes delegated partial work before error" {
    var authority = try makeAuthority(4096);
    defer authority.deinit();
    var candidate = state.Candidate.begin(&authority.storage);
    while (true) {
        var budget: types.WorkBudget = .{ .remaining = 256 };
        if (try candidate.advancePrepare(&budget) == .done) break;
    }
    var envelope: [1024]u8 = @splat(0);
    std.mem.writeInt(u64, envelope[0..8], 2, .little);
    envelope[8] = 0; // Creator provenance.
    envelope[9] = 0; // Absent actor.
    envelope[10] = 1; // Grant tag.
    const oversized = types.Receipt{ .header = .{ .tick = @fromBackingInt(2), .ordinal = 1, .provenance = .creator, .sequence = @fromBackingInt(2) }, .event = .{ .accepted = .{ .envelope = .{ .bytes = &envelope }, .effect = .{ .grant = .{ .food_minted = 1, .material_minted = 0, .population_minted = 0 } } } } };
    var one: types.WorkBudget = .{ .remaining = 1 };
    try std.testing.expectEqual(types.Step.done, try candidate.stage(oversized, &one));
    const units_before = authority.storage.work_units;
    var consumed: u64 = 0;
    var finalizer = history.Finalizer.init(&candidate);
    while (true) {
        one.remaining = 1;
        const step = finalizer.advance(&one) catch |failure| {
            consumed += 1 - one.remaining;
            try std.testing.expectEqual(error.EnvelopeTooLarge, failure);
            break;
        };
        consumed += 1 - one.remaining;
        if (step == .done) return error.UnexpectedProgress;
    }
    try std.testing.expect(consumed > 1);
    try std.testing.expectEqual(consumed, authority.storage.work_units - units_before);
    try std.testing.expectEqual(@as(u16, 1), authority.storage.rootMeta(authority.storage.active).history_used);
}

test "pe/history/all-tags optional policy and rejection index" {
    try expectPayload(.{ .tick_input = .{
        .target_tick = @fromBackingInt(3),
        .expected_revision = @fromBackingInt(2),
        .previous_policy = .{ .role = .creator, .civilization = null },
        .current_policy = .{ .role = .ruler, .civilization = @fromBackingInt(7) },
        .envelope_count = 4,
    } }, &.{ 3, 0, 0, 0, 0, 0, 0, 0, 2, 0, 0, 0, 0, 0, 0, 0, 1, 0, 0, 1, 1, 7, 0, 0, 0, 4, 0 });
    const envelope = [_]u8{ 2, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1 };
    const value = types.Receipt{ .header = .{ .tick = @fromBackingInt(2), .ordinal = 1, .provenance = .creator, .sequence = @fromBackingInt(2) }, .event = .{ .rejected = .{ .envelope = .{ .bytes = &envelope }, .reason = .PathEndpointMismatch, .offending_index = 3 } } };
    var output: [1024]u8 = undefined;
    const length = try encode(&value, &output);
    try std.testing.expectEqualSlices(u8, &.{ 37, 0, 1, 3, 0 }, output[length - 5 .. length]);

    const startup = eventReceipt(.{ .genesis = .{ .seed = 2, .side = 4, .startup = .ruler_start } });
    const startup_len = try encode(&startup, &output);
    try std.testing.expectEqual(@as(usize, 81), startup_len);
    try std.testing.expectEqualSlices(u8, &.{ 1, 0, 1, 0, 0, 0, 7, 0, 0, 0 }, output[startup_len - 10 .. startup_len]);
}

test "pe/history/bounded-tail two-block SHA padding charges each block" {
    const bytes: [27]u8 = @splat(0x42); // 32 + 4 + 27 = 63 bytes before padding.
    var chain = history.Chain.init();
    try chain.begin(&bytes);
    const old_head = chain.head();
    while (chain.cursor < 63) {
        var one: types.WorkBudget = .{ .remaining = 1 };
        try std.testing.expectEqual(types.Step.pending, try chain.advance(&one));
    }
    var one: types.WorkBudget = .{ .remaining = 1 };
    try std.testing.expectEqual(types.Step.pending, try chain.advance(&one));
    try std.testing.expectEqual(@as(u8, 0), chain.hash.buf_len);
    try std.testing.expectEqualDeep(old_head, chain.head());
    one.remaining = 1;
    try std.testing.expectEqual(types.Step.done, try chain.advance(&one));
    var expected_input: [63]u8 = undefined;
    @memcpy(expected_input[0..32], &wire_vectors.h0);
    std.mem.writeInt(u32, expected_input[32..36], 27, .little);
    @memcpy(expected_input[36..], &bytes);
    var expected: [32]u8 = undefined;
    std.crypto.hash.sha2.Sha256.hash(&expected_input, &expected, .{});
    try std.testing.expectEqualDeep(expected, chain.head());
}
