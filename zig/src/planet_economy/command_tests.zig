const std = @import("std");
const pe = @import("root.zig");
const command = @import("command.zig");

fn makeAuthority() !pe.Authority {
    return makeAuthorityWithSide(4);
}

fn makeAuthorityWithSide(side: u16) !pe.Authority {
    return makeAuthorityWithConfig(.{ .seed = 2, .side = side });
}

fn makeAuthorityWithConfig(config: pe.Config) !pe.Authority {
    var builder = try pe.beginCreate(std.testing.allocator, config);
    defer builder.deinit();
    while (true) {
        var budget: pe.WorkBudget = .{ .remaining = 256 };
        switch (pe.pollCreate(&builder, &budget)) {
            .pending => {},
            .ready => return pe.takeAuthority(&builder),
            .failed => |failure| return failure,
            else => return error.UnexpectedProgress,
        }
    }
}

fn prepare(candidate: *pe.state.Candidate) !void {
    while (true) {
        var budget: pe.WorkBudget = .{ .remaining = 256 };
        if (try candidate.advancePrepare(&budget) == .done) return;
    }
}

fn discard(candidate: *pe.state.Candidate) !void {
    while (true) {
        var budget: pe.WorkBudget = .{ .remaining = 256 };
        if (try candidate.advanceDiscard(&budget) == .done) return;
    }
}

fn run(candidate: *pe.state.Candidate, policy: pe.SessionPolicy, envelopes: []const pe.Envelope) !command.Cursor {
    return runWithBudget(candidate, policy, envelopes, 256);
}

fn runWithBudget(candidate: *pe.state.Candidate, policy: pe.SessionPolicy, envelopes: []const pe.Envelope, units: u16) !command.Cursor {
    var cursor = command.Cursor.init(policy, envelopes, @fromBackingInt(0), @fromBackingInt(1));
    while (true) {
        var budget: pe.WorkBudget = .{ .remaining = units };
        if (try cursor.advance(candidate, &budget) == .done) return cursor;
    }
}

fn authoredSettlement(sequence: u64, cell: u64, population: u8, food: u32, material: u32, bytes: []u8) !pe.Envelope {
    return command.encodeEnvelope(.{ .sequence = @fromBackingInt(sequence), .provenance = .creator, .actor = null }, .{ .author_settlement = .{ .owner = pe.civilization_id, .anchor = .{ .planet = pe.planet_id, .local = cell }, .population = population, .food = food, .material = material, .economic_bypass = true } }, bytes);
}

test "pe/commands/envelope canonical grant encoding" {
    var bytes: [1024]u8 = undefined;
    const envelope = try command.encodeEnvelope(.{
        .sequence = @fromBackingInt(2),
        .provenance = .creator,
        .actor = null,
    }, .{ .grant = .{
        .settlement = @fromBackingInt(1),
        .food = 1,
        .material = 0,
        .population = 0,
        .economic_bypass = true,
    } }, &bytes);
    try std.testing.expectEqualSlices(u8, &.{
        2, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1,
        1, 0, 0, 0, 0, 0, 0, 0, 1, 0, 0,
        0, 0, 0, 0, 0, 0, 1,
    }, envelope.bytes);
    try std.testing.expectEqual(@as(usize, 29), envelope.bytes.len);
    _ = pe;
}

test "pe/commands/role-tag-bypass preserves admission precedence" {
    var builder = try pe.beginCreate(std.testing.allocator, .{ .seed = 2, .side = 4 });
    defer builder.deinit();
    while (true) {
        var budget: pe.WorkBudget = .{ .remaining = 256 };
        switch (pe.pollCreate(&builder, &budget)) {
            .pending => {},
            .ready => break,
            .failed => |failure| return failure,
            else => return error.UnexpectedProgress,
        }
    }
    var authority = try pe.takeAuthority(&builder);
    defer authority.deinit();
    var candidate = pe.state.Candidate.begin(&authority.storage);
    while (true) {
        var budget: pe.WorkBudget = .{ .remaining = 256 };
        if (try candidate.advancePrepare(&budget) == .done) break;
    }
    var bytes: [3][1024]u8 = undefined;
    var envelopes: [3]pe.Envelope = undefined;
    envelopes[0] = try command.encodeEnvelope(.{ .sequence = @fromBackingInt(1), .provenance = .creator, .actor = null }, .{ .grant = .{ .settlement = @fromBackingInt(99), .food = 1, .material = 0, .population = 0, .economic_bypass = true } }, &bytes[0]);
    envelopes[1] = try command.encodeEnvelope(.{ .sequence = @fromBackingInt(2), .provenance = .ruler, .actor = pe.civilization_id }, .{ .grant = .{ .settlement = @fromBackingInt(99), .food = 1, .material = 0, .population = 0, .economic_bypass = true } }, &bytes[1]);
    envelopes[2] = try command.encodeEnvelope(.{ .sequence = @fromBackingInt(3), .provenance = .creator, .actor = null }, .{ .grant = .{ .settlement = @fromBackingInt(99), .food = 1, .material = 0, .population = 0, .economic_bypass = false } }, &bytes[2]);
    var cursor = command.Cursor.init(try pe.SessionPolicy.init(.ruler, pe.civilization_id), &envelopes, @fromBackingInt(0), @fromBackingInt(1));
    while (true) {
        var budget: pe.WorkBudget = .{ .remaining = 256 };
        if (try cursor.advance(&candidate, &budget) == .done) break;
    }
    try std.testing.expectEqual(@as(u16, 3), authority.storage.staged_count);
    try std.testing.expectEqual(pe.RejectReason.CreatorForbidden, authority.storage.logicalReceipt(0).event.rejected.reason);
    try std.testing.expectEqual(pe.RejectReason.CommandProvenanceMismatch, authority.storage.logicalReceipt(1).event.rejected.reason);
    try std.testing.expectEqual(pe.RejectReason.CreatorForbidden, authority.storage.logicalReceipt(2).event.rejected.reason);
    while (true) {
        var budget: pe.WorkBudget = .{ .remaining = 256 };
        if (try candidate.advanceDiscard(&budget) == .done) break;
    }
}

test "pe/commands/atomic-costs paid farm debits six material after authored founding" {
    var builder = try pe.beginCreate(std.testing.allocator, .{ .seed = 2, .side = 4 });
    defer builder.deinit();
    while (true) {
        var budget: pe.WorkBudget = .{ .remaining = 256 };
        switch (pe.pollCreate(&builder, &budget)) {
            .pending => {},
            .ready => break,
            .failed => |failure| return failure,
            else => return error.UnexpectedProgress,
        }
    }
    var authority = try pe.takeAuthority(&builder);
    defer authority.deinit();
    var candidate = pe.state.Candidate.begin(&authority.storage);
    while (true) {
        var budget: pe.WorkBudget = .{ .remaining = 256 };
        if (try candidate.advancePrepare(&budget) == .done) break;
    }
    var bytes: [2][1024]u8 = undefined;
    const envelopes = [_]pe.Envelope{
        try command.encodeEnvelope(.{ .sequence = @fromBackingInt(1), .provenance = .creator, .actor = null }, .{ .author_settlement = .{ .owner = pe.civilization_id, .anchor = .{ .planet = pe.planet_id, .local = 70 }, .population = 2, .food = 0, .material = 8, .economic_bypass = true } }, &bytes[0]),
        try command.encodeEnvelope(.{ .sequence = @fromBackingInt(2), .provenance = .ruler, .actor = pe.civilization_id }, .{ .build = .{ .settlement = @fromBackingInt(1), .kind = .farm, .anchor = .{ .planet = pe.planet_id, .local = 71 }, .facing = .east } }, &bytes[1]),
    };
    var cursor = command.Cursor.init(try pe.SessionPolicy.init(.combined, pe.civilization_id), &envelopes, @fromBackingInt(0), @fromBackingInt(1));
    while (true) {
        var budget: pe.WorkBudget = .{ .remaining = 256 };
        if (try cursor.advance(&candidate, &budget) == .done) break;
    }
    try std.testing.expectEqual(@as(u16, 2), cursor.accepted);
    try std.testing.expectEqual(@as(u32, 6), authority.storage.logicalReceipt(1).event.accepted.effect.build.material_spent);
    try std.testing.expectEqual(@as(u32, 2), (try candidate.readRecord(.settlement, 0)).settlement.material);
    while (true) {
        var budget: pe.WorkBudget = .{ .remaining = 256 };
        if (try candidate.advanceDiscard(&budget) == .done) break;
    }
}

test "pe/commands/creator-actions active farm waives cost and permits later grant" {
    var authority = try makeAuthority();
    defer authority.deinit();
    var candidate = pe.state.Candidate.begin(&authority.storage);
    try prepare(&candidate);
    var bytes: [3][1024]u8 = undefined;
    const envelopes = [_]pe.Envelope{
        try authoredSettlement(1, 70, 2, 0, 0, &bytes[0]),
        try command.encodeEnvelope(.{ .sequence = @fromBackingInt(2), .provenance = .creator, .actor = null }, .{ .author_build = .{ .settlement = @fromBackingInt(1), .kind = .farm, .anchor = .{ .planet = pe.planet_id, .local = 71 }, .facing = .east, .economic_bypass = true } }, &bytes[1]),
        try command.encodeEnvelope(.{ .sequence = @fromBackingInt(3), .provenance = .creator, .actor = null }, .{ .grant = .{ .settlement = @fromBackingInt(1), .food = 5, .material = 0, .population = 1, .economic_bypass = true } }, &bytes[2]),
    };
    const cursor = try runWithBudget(&candidate, try pe.SessionPolicy.init(.creator, null), &envelopes, 1);
    try std.testing.expectEqual(@as(u16, 3), cursor.accepted);
    const effect = authority.storage.logicalReceipt(1).event.accepted.effect.author_build;
    try std.testing.expectEqual(@as(u32, 6), effect.material_waived);
    try std.testing.expectEqual(@as(u8, 2), effect.ticks_waived);
    try std.testing.expectEqual(@as(u8, 0), (try candidate.readRecord(.building, 0)).building.remaining_ticks);
    const settlement = (try candidate.readRecord(.settlement, 0)).settlement;
    try std.testing.expectEqual(@as(u32, 0), settlement.material);
    try std.testing.expectEqual(@as(u32, 5), settlement.food);
    try std.testing.expectEqual(@as(u8, 3), settlement.population);
    try discard(&candidate);
}

test "pe/commands/seam-loop rejects repeated cell at index three" {
    var authority = try makeAuthority();
    defer authority.deinit();
    var candidate = pe.state.Candidate.begin(&authority.storage);
    try prepare(&candidate);
    var bytes: [3][1024]u8 = undefined;
    const path = [_]pe.CellId{
        .{ .planet = pe.planet_id, .local = 70 }, .{ .planet = pe.planet_id, .local = 71 },
        .{ .planet = pe.planet_id, .local = 4 },  .{ .planet = pe.planet_id, .local = 71 },
        .{ .planet = pe.planet_id, .local = 4 },  .{ .planet = pe.planet_id, .local = 5 },
    };
    const envelopes = [_]pe.Envelope{
        try authoredSettlement(1, 70, 0, 0, 0, &bytes[0]),
        try authoredSettlement(2, 5, 0, 0, 0, &bytes[1]),
        try command.encodeEnvelope(.{ .sequence = @fromBackingInt(3), .provenance = .creator, .actor = null }, .{ .author_ship = .{ .source = @fromBackingInt(1), .destination = @fromBackingInt(2), .resource = .food, .amount = 1, .path = &path, .cargo_source = .mint, .economic_bypass = true } }, &bytes[2]),
    };
    const cursor = try run(&candidate, try pe.SessionPolicy.init(.creator, null), &envelopes);
    try std.testing.expectEqual(@as(u16, 2), cursor.accepted);
    try std.testing.expectEqual(@as(u16, 1), cursor.rejected);
    const rejected = authority.storage.logicalReceipt(2).event.rejected;
    try std.testing.expectEqual(pe.RejectReason.RepeatedCell, rejected.reason);
    try std.testing.expectEqual(@as(?u16, 3), rejected.offending_index);
    try std.testing.expectEqual(@as(u64, 1), candidate.meta().next_shipment);
    try discard(&candidate);
}

test "pe/commands/batch-order sorts before effects and duplicate aborts before effects" {
    var authority = try makeAuthority();
    defer authority.deinit();
    var candidate = pe.state.Candidate.begin(&authority.storage);
    try prepare(&candidate);
    var bytes: [3][1024]u8 = undefined;
    const envelopes = [_]pe.Envelope{
        try command.encodeEnvelope(.{ .sequence = @fromBackingInt(3), .provenance = .creator, .actor = null }, .{ .grant = .{ .settlement = @fromBackingInt(1), .food = 3, .material = 0, .population = 0, .economic_bypass = true } }, &bytes[0]),
        try authoredSettlement(1, 70, 0, 0, 0, &bytes[1]),
        try command.encodeEnvelope(.{ .sequence = @fromBackingInt(2), .provenance = .creator, .actor = null }, .{ .grant = .{ .settlement = @fromBackingInt(1), .food = 2, .material = 0, .population = 0, .economic_bypass = true } }, &bytes[2]),
    };
    const cursor = try run(&candidate, try pe.SessionPolicy.init(.creator, null), &envelopes);
    try std.testing.expectEqual(@as(u16, 3), cursor.accepted);
    try std.testing.expectEqual(@as(u32, 5), (try candidate.readRecord(.settlement, 0)).settlement.food);
    try std.testing.expectEqual(@as(u64, 1), @backingInt(authority.storage.logicalReceipt(0).header.sequence.?));
    try std.testing.expectEqual(@as(u64, 2), @backingInt(authority.storage.logicalReceipt(1).header.sequence.?));
    try std.testing.expectEqual(@as(u64, 3), @backingInt(authority.storage.logicalReceipt(2).header.sequence.?));
    try discard(&candidate);

    var retry = pe.state.Candidate.begin(&authority.storage);
    try prepare(&retry);
    var duplicate = envelopes;
    duplicate[2] = try authoredSettlement(1, 71, 0, 0, 0, &bytes[2]);
    var dup_cursor = command.Cursor.init(try pe.SessionPolicy.init(.creator, null), &duplicate, @fromBackingInt(0), @fromBackingInt(1));
    var failure: ?pe.Failure = null;
    while (true) {
        var budget: pe.WorkBudget = .{ .remaining = 256 };
        const step = dup_cursor.advance(&retry, &budget) catch |err| {
            failure = err;
            break;
        };
        if (step == .done) break;
    }
    try std.testing.expectEqual(error.DuplicateSequence, failure.?);
    try std.testing.expectEqual(@as(u16, 0), authority.storage.staged_count);
    try std.testing.expectEqual(@as(u16, 0), retry.meta().settlements);
    const already_published = [_]pe.Envelope{envelopes[1]};
    var old_cursor = command.Cursor.init(try pe.SessionPolicy.init(.creator, null), &already_published, @fromBackingInt(1), @fromBackingInt(1));
    var old_failure: ?pe.Failure = null;
    while (true) {
        var budget: pe.WorkBudget = .{ .remaining = 7 };
        const step = old_cursor.advance(&retry, &budget) catch |err| {
            old_failure = err;
            break;
        };
        if (step == .done) break;
    }
    try std.testing.expectEqual(error.SequenceAlreadyPublished, old_failure.?);
    try std.testing.expectEqual(@as(u16, 0), authority.storage.staged_count);
    try std.testing.expectEqual(@as(u16, 0), retry.meta().settlements);
    try discard(&retry);
}

test "pe/commands/zero-budget leaves cursor and candidate unchanged" {
    var authority = try makeAuthority();
    defer authority.deinit();
    var candidate = pe.state.Candidate.begin(&authority.storage);
    try prepare(&candidate);
    var bytes: [1024]u8 = undefined;
    const envelope = [_]pe.Envelope{try authoredSettlement(1, 70, 0, 0, 0, &bytes)};
    var cursor = command.Cursor.init(try pe.SessionPolicy.init(.creator, null), &envelope, @fromBackingInt(0), @fromBackingInt(1));
    var budget: pe.WorkBudget = .{ .remaining = 0 };
    try std.testing.expectEqual(pe.Step.pending, try cursor.advance(&candidate, &budget));
    try std.testing.expectEqual(@as(u16, 0), cursor.index);
    try std.testing.expectEqual(@as(u16, 0), cursor.offset);
    try std.testing.expectEqual(@as(u32, 0), cursor.parse_units);
    try std.testing.expectEqual(@as(u16, 0), authority.storage.staged_count);
    try std.testing.expectEqual(@as(u16, 0), candidate.meta().settlements);
    try discard(&candidate);
}

test "pe/commands/found and freight distinguish transferred and minted cargo" {
    var authority = try makeAuthority();
    defer authority.deinit();
    var candidate = pe.state.Candidate.begin(&authority.storage);
    try prepare(&candidate);
    var bytes: [5][1024]u8 = undefined;
    const route = [_]pe.CellId{
        .{ .planet = pe.planet_id, .local = 70 }, .{ .planet = pe.planet_id, .local = 71 },
        .{ .planet = pe.planet_id, .local = 4 },  .{ .planet = pe.planet_id, .local = 5 },
    };
    const envelopes = [_]pe.Envelope{
        try authoredSettlement(1, 70, 4, 8, 20, &bytes[0]),
        try command.encodeEnvelope(.{ .sequence = @fromBackingInt(2), .provenance = .ruler, .actor = pe.civilization_id }, .{ .found = .{ .source = @fromBackingInt(1), .anchor = .{ .planet = pe.planet_id, .local = 5 } } }, &bytes[1]),
        try command.encodeEnvelope(.{ .sequence = @fromBackingInt(3), .provenance = .ruler, .actor = pe.civilization_id }, .{ .ship = .{ .source = @fromBackingInt(1), .destination = @fromBackingInt(2), .resource = .food, .amount = 2, .path = &route } }, &bytes[2]),
        try command.encodeEnvelope(.{ .sequence = @fromBackingInt(4), .provenance = .creator, .actor = null }, .{ .author_ship = .{ .source = @fromBackingInt(1), .destination = @fromBackingInt(2), .resource = .material, .amount = 3, .path = &route, .cargo_source = .mint, .economic_bypass = true } }, &bytes[3]),
        try command.encodeEnvelope(.{ .sequence = @fromBackingInt(5), .provenance = .creator, .actor = null }, .{ .author_ship = .{ .source = @fromBackingInt(1), .destination = @fromBackingInt(2), .resource = .food, .amount = 1, .path = &route, .cargo_source = .transfer, .economic_bypass = true } }, &bytes[4]),
    };
    const cursor = try run(&candidate, try pe.SessionPolicy.init(.combined, pe.civilization_id), &envelopes);
    try std.testing.expectEqual(@as(u16, 5), cursor.accepted);
    const found = authority.storage.logicalReceipt(1).event.accepted.effect.found;
    try std.testing.expectEqual(@as(u32, 4), found.food_moved);
    try std.testing.expectEqual(@as(u32, 20), found.material_spent);
    try std.testing.expectEqual(@as(u8, 2), found.population_moved);
    const shipment = authority.storage.logicalReceipt(2).event.accepted.effect.ship;
    try std.testing.expectEqual(@as(u16, 2), shipment.transferred);
    try std.testing.expectEqual(@as(u64, 4), @backingInt(shipment.arrival_tick));
    const authored = authority.storage.logicalReceipt(3).event.accepted.effect.author_ship;
    try std.testing.expectEqual(@as(u16, 0), authored.transferred);
    try std.testing.expectEqual(@as(u16, 3), authored.minted);
    const authored_transfer = authority.storage.logicalReceipt(4).event.accepted.effect.author_ship;
    try std.testing.expectEqual(@as(u16, 1), authored_transfer.transferred);
    try std.testing.expectEqual(@as(u16, 0), authored_transfer.minted);
    const source = (try candidate.readRecord(.settlement, 0)).settlement;
    const destination = (try candidate.readRecord(.settlement, 1)).settlement;
    try std.testing.expectEqual(@as(u32, 1), source.food);
    try std.testing.expectEqual(@as(u32, 0), source.material);
    try std.testing.expectEqual(@as(u8, 2), source.population);
    try std.testing.expectEqual(@as(u32, 4), destination.food);
    try std.testing.expectEqual(@as(u32, 3), destination.incoming_food);
    try std.testing.expectEqual(@as(u32, 3), destination.incoming_material);
    try std.testing.expectEqual(@as(u16, 3), candidate.meta().shipments);
    try discard(&candidate);
}

test "pe/commands/atomic-costs second footprint cell overlap keeps cost and ID" {
    var authority = try makeAuthority();
    defer authority.deinit();
    var candidate = pe.state.Candidate.begin(&authority.storage);
    try prepare(&candidate);
    var bytes: [3][1024]u8 = undefined;
    const envelopes = [_]pe.Envelope{
        try authoredSettlement(1, 70, 2, 0, 20, &bytes[0]),
        try command.encodeEnvelope(.{ .sequence = @fromBackingInt(2), .provenance = .ruler, .actor = pe.civilization_id }, .{ .build = .{ .settlement = @fromBackingInt(1), .kind = .farm, .anchor = .{ .planet = pe.planet_id, .local = 71 }, .facing = .east } }, &bytes[1]),
        try command.encodeEnvelope(.{ .sequence = @fromBackingInt(3), .provenance = .ruler, .actor = pe.civilization_id }, .{ .build = .{ .settlement = @fromBackingInt(1), .kind = .farm, .anchor = .{ .planet = pe.planet_id, .local = 5 }, .facing = .west } }, &bytes[2]),
    };
    const cursor = try run(&candidate, try pe.SessionPolicy.init(.combined, pe.civilization_id), &envelopes);
    try std.testing.expectEqual(@as(u16, 2), cursor.accepted);
    try std.testing.expectEqual(@as(u16, 1), cursor.rejected);
    const rejected = authority.storage.logicalReceipt(2).event.rejected;
    try std.testing.expectEqual(pe.RejectReason.OccupiedCell, rejected.reason);
    try std.testing.expectEqual(@as(?u16, 1), rejected.offending_index);
    try std.testing.expectEqual(@as(u32, 14), (try candidate.readRecord(.settlement, 0)).settlement.material);
    try std.testing.expectEqual(@as(u64, 2), candidate.meta().next_building);
    try discard(&candidate);
}

test "pe/commands/structural-invalid aborts whole batch before first effect" {
    var authority = try makeAuthority();
    defer authority.deinit();
    var candidate = pe.state.Candidate.begin(&authority.storage);
    try prepare(&candidate);
    var bytes: [2][1024]u8 = undefined;
    const first = try authoredSettlement(1, 70, 0, 0, 0, &bytes[0]);
    const second_valid = try command.encodeEnvelope(.{ .sequence = @fromBackingInt(2), .provenance = .creator, .actor = null }, .{ .grant = .{ .settlement = @fromBackingInt(1), .food = 1, .material = 0, .population = 0, .economic_bypass = true } }, &bytes[1]);
    bytes[1][second_valid.bytes.len - 1] = 2;
    const envelopes = [_]pe.Envelope{ first, second_valid };
    var cursor = command.Cursor.init(try pe.SessionPolicy.init(.creator, null), &envelopes, @fromBackingInt(0), @fromBackingInt(1));
    var failure: ?pe.Failure = null;
    while (true) {
        var budget: pe.WorkBudget = .{ .remaining = 1 };
        const step = cursor.advance(&candidate, &budget) catch |err| {
            failure = err;
            break;
        };
        if (step == .done) break;
    }
    try std.testing.expectEqual(error.InvalidEnvelope, failure.?);
    try std.testing.expectEqual(@as(u16, 0), authority.storage.staged_count);
    try std.testing.expectEqual(@as(u16, 0), candidate.meta().settlements);
    try std.testing.expectEqual(@as(u64, 1), candidate.meta().next_settlement);
    try discard(&candidate);
}

test "pe/commands/structural codec rejects malformed option enum bool and trailing bytes" {
    var authority = try makeAuthority();
    defer authority.deinit();
    var candidate = pe.state.Candidate.begin(&authority.storage);
    try prepare(&candidate);
    var base: [1025]u8 = undefined;
    const valid = try command.encodeEnvelope(.{ .sequence = @fromBackingInt(1), .provenance = .creator, .actor = null }, .{ .grant = .{ .settlement = @fromBackingInt(1), .food = 1, .material = 0, .population = 0, .economic_bypass = true } }, base[0..1024]);
    const len = valid.bytes.len;
    for ([_]struct { index: usize, value: u8 }{
        .{ .index = 8, .value = 3 }, .{ .index = 9, .value = 2 }, .{ .index = len - 1, .value = 2 },
    }) |mutation| {
        var malformed = base;
        malformed[mutation.index] = mutation.value;
        const envelope = [_]pe.Envelope{.{ .bytes = malformed[0..len] }};
        var cursor = command.Cursor.init(try pe.SessionPolicy.init(.creator, null), &envelope, @fromBackingInt(0), @fromBackingInt(1));
        var budget: pe.WorkBudget = .{ .remaining = 256 };
        try std.testing.expectError(error.InvalidEnvelope, cursor.advance(&candidate, &budget));
    }
    base[len] = 0;
    const trailing = [_]pe.Envelope{.{ .bytes = base[0 .. len + 1] }};
    var trailing_cursor = command.Cursor.init(try pe.SessionPolicy.init(.creator, null), &trailing, @fromBackingInt(0), @fromBackingInt(1));
    var budget: pe.WorkBudget = .{ .remaining = 256 };
    try std.testing.expectError(error.InvalidEnvelope, trailing_cursor.advance(&candidate, &budget));
    const oversized = [_]pe.Envelope{.{ .bytes = base[0..1025] }};
    var large_cursor = command.Cursor.init(try pe.SessionPolicy.init(.creator, null), &oversized, @fromBackingInt(0), @fromBackingInt(1));
    budget.remaining = 256;
    try std.testing.expectError(error.EnvelopeTooLarge, large_cursor.advance(&candidate, &budget));
    try std.testing.expectEqual(@as(u16, 0), authority.storage.staged_count);
    try discard(&candidate);
}

test "pe/commands/role-tag-bypass and facing reject before target lookup" {
    var authority = try makeAuthority();
    defer authority.deinit();
    var candidate = pe.state.Candidate.begin(&authority.storage);
    try prepare(&candidate);
    var bytes: [3][1024]u8 = undefined;
    const envelopes = [_]pe.Envelope{
        try command.encodeEnvelope(.{ .sequence = @fromBackingInt(1), .provenance = .creator, .actor = null }, .{ .grant = .{ .settlement = @fromBackingInt(99), .food = 1, .material = 0, .population = 0, .economic_bypass = false } }, &bytes[0]),
        try authoredSettlement(2, 70, 0, 0, 0, &bytes[1]),
        try command.encodeEnvelope(.{ .sequence = @fromBackingInt(3), .provenance = .creator, .actor = null }, .{ .author_build = .{ .settlement = @fromBackingInt(1), .kind = .farm, .anchor = .{ .planet = pe.planet_id, .local = 71 }, .facing = null, .economic_bypass = true } }, &bytes[2]),
    };
    const cursor = try run(&candidate, try pe.SessionPolicy.init(.creator, null), &envelopes);
    try std.testing.expectEqual(@as(u16, 1), cursor.accepted);
    try std.testing.expectEqual(pe.RejectReason.ExplicitBypassRequired, authority.storage.logicalReceipt(0).event.rejected.reason);
    try std.testing.expectEqual(pe.RejectReason.InvalidFacing, authority.storage.logicalReceipt(2).event.rejected.reason);
    try std.testing.expectEqual(@as(u64, 1), candidate.meta().next_building);
    try discard(&candidate);
}

test "pe/commands/budget-one and seven preserve accepted receipts" {
    var first = try makeAuthority();
    defer first.deinit();
    var second = try makeAuthority();
    defer second.deinit();
    var candidate1 = pe.state.Candidate.begin(&first.storage);
    var candidate7 = pe.state.Candidate.begin(&second.storage);
    try prepare(&candidate1);
    try prepare(&candidate7);
    var bytes: [2][1024]u8 = undefined;
    const envelopes = [_]pe.Envelope{
        try authoredSettlement(1, 70, 2, 0, 8, &bytes[0]),
        try command.encodeEnvelope(.{ .sequence = @fromBackingInt(2), .provenance = .ruler, .actor = pe.civilization_id }, .{ .build = .{ .settlement = @fromBackingInt(1), .kind = .farm, .anchor = .{ .planet = pe.planet_id, .local = 71 }, .facing = .east } }, &bytes[1]),
    };
    const policy = try pe.SessionPolicy.init(.combined, pe.civilization_id);
    const cursor1 = try runWithBudget(&candidate1, policy, &envelopes, 1);
    const cursor7 = try runWithBudget(&candidate7, policy, &envelopes, 7);
    try std.testing.expectEqual(cursor1.accepted, cursor7.accepted);
    try std.testing.expectEqualDeep(candidate1.meta(), candidate7.meta());
    try std.testing.expectEqualDeep((try candidate1.readRecord(.settlement, 0)).settlement, (try candidate7.readRecord(.settlement, 0)).settlement);
    for (0..2) |index| try std.testing.expectEqualDeep(first.storage.logicalReceipt(@intCast(index)).*, second.storage.logicalReceipt(@intCast(index)).*);
    try discard(&candidate1);
    try discard(&candidate7);
}

test "pe/commands/budget-one prepares at most one fixed record descriptor" {
    var authority = try makeAuthority();
    defer authority.deinit();
    var candidate = pe.state.Candidate.begin(&authority.storage);
    try prepare(&candidate);
    var bytes: [2][1024]u8 = undefined;
    const envelopes = [_]pe.Envelope{
        try authoredSettlement(1, 70, 2, 0, 8, &bytes[0]),
        try command.encodeEnvelope(.{ .sequence = @fromBackingInt(2), .provenance = .ruler, .actor = pe.civilization_id }, .{ .build = .{ .settlement = @fromBackingInt(1), .kind = .farm, .anchor = .{ .planet = pe.planet_id, .local = 71 }, .facing = .east } }, &bytes[1]),
    };
    var cursor = command.Cursor.init(try pe.SessionPolicy.init(.combined, pe.civilization_id), &envelopes, @fromBackingInt(0), @fromBackingInt(1));
    while (true) {
        const before = cursor.prepared_fixed_writes;
        var budget: pe.WorkBudget = .{ .remaining = 1 };
        const step = try cursor.advance(&candidate, &budget);
        try std.testing.expect(cursor.prepared_fixed_writes <= before + 1);
        if (cursor.prepared_fixed_writes != before) try std.testing.expectEqual(@as(u16, 0), budget.remaining);
        if (step == .done) break;
    }
    try std.testing.expectEqual(@as(u32, 6), cursor.prepared_fixed_writes);
    try std.testing.expectEqual(@as(u16, 2), cursor.accepted);
    try discard(&candidate);
}

test "pe/commands/max-path admits 65 cells and 256 cargo with candidate-owned envelope" {
    var authority = try makeAuthorityWithSide(8);
    defer authority.deinit();
    var candidate = pe.state.Candidate.begin(&authority.storage);
    try prepare(&candidate);
    var route: [65]pe.CellId = undefined;
    for (0..8) |row| for (0..8) |column| {
        const x = if (row % 2 == 0) column else 7 - column;
        route[row * 8 + column] = .{ .planet = pe.planet_id, .local = 256 + row * 8 + x };
    };
    const topology = try pe.surface.Topology.init(8);
    route[64] = .{ .planet = pe.planet_id, .local = try topology.cellId(try topology.neighbor(try topology.fromId(312), .west)) };
    var bytes: [3][1024]u8 = undefined;
    const envelopes = [_]pe.Envelope{
        try authoredSettlement(1, 256, 0, 0, 0, &bytes[0]),
        try authoredSettlement(2, route[64].local, 0, 0, 0, &bytes[1]),
        try command.encodeEnvelope(.{ .sequence = @fromBackingInt(3), .provenance = .creator, .actor = null }, .{ .author_ship = .{ .source = @fromBackingInt(1), .destination = @fromBackingInt(2), .resource = .food, .amount = 256, .path = &route, .cargo_source = .mint, .economic_bypass = true } }, &bytes[2]),
    };
    try std.testing.expectEqual(@as(usize, 814), envelopes[2].bytes.len);
    const cursor = try runWithBudget(&candidate, try pe.SessionPolicy.init(.creator, null), &envelopes, 1);
    try std.testing.expectEqual(@as(u16, 3), cursor.accepted);
    try std.testing.expectEqual(@as(u32, @intCast(envelopes[0].bytes.len + envelopes[1].bytes.len + envelopes[2].bytes.len)), cursor.parsed_bytes);
    try std.testing.expect(cursor.parse_units >= 65);
    try std.testing.expectEqual(@as(u8, 12), cursor.max_parse_bytes_per_unit);
    const accepted = authority.storage.logicalReceipt(2).event.accepted;
    try std.testing.expectEqual(@as(usize, 814), accepted.envelope.bytes.len);
    bytes[2][0] = 99;
    try std.testing.expectEqual(@as(u8, 3), accepted.envelope.bytes[0]);
    try std.testing.expectEqual(@as(u16, 256), accepted.effect.author_ship.minted);
    try std.testing.expectEqual(@as(u64, 65), @backingInt(accepted.effect.author_ship.arrival_tick));
    try std.testing.expectEqual(route[64], try candidate.routeCell(0, 64));
    try std.testing.expectEqual(@as(u32, 256), (try candidate.readRecord(.settlement, 1)).settlement.incoming_food);
    const raw_overlong = blk: {
        var bytes_copy = bytes[2];
        bytes_copy[30] = 66;
        bytes_copy[31] = 0;
        break :blk bytes_copy;
    };
    const invalid_envelope = [_]pe.Envelope{.{ .bytes = raw_overlong[0..814] }};
    var invalid_cursor = command.Cursor.init(try pe.SessionPolicy.init(.creator, null), &invalid_envelope, @fromBackingInt(0), @fromBackingInt(1));
    var budget: pe.WorkBudget = .{ .remaining = 256 };
    try std.testing.expectError(error.InvalidEnvelope, invalid_cursor.advance(&candidate, &budget));
    var oversized_route: [66]pe.CellId = undefined;
    @memcpy(oversized_route[0..65], &route);
    oversized_route[65] = route[64];
    try std.testing.expectError(error.InvalidEnvelope, command.encodeEnvelope(.{
        .sequence = @fromBackingInt(4),
        .provenance = .creator,
        .actor = null,
    }, .{ .author_ship = .{ .source = @fromBackingInt(1), .destination = @fromBackingInt(2), .resource = .food, .amount = 1, .path = &oversized_route, .cargo_source = .mint, .economic_bypass = true } }, &bytes[2]));
    try discard(&candidate);
}

test "pe/commands/grant exact inventory ceiling and zero delta" {
    var authority = try makeAuthority();
    defer authority.deinit();
    var candidate = pe.state.Candidate.begin(&authority.storage);
    try prepare(&candidate);
    var bytes: [3][1024]u8 = undefined;
    const envelopes = [_]pe.Envelope{
        try authoredSettlement(1, 70, 0, 65_535, 0, &bytes[0]),
        try command.encodeEnvelope(.{ .sequence = @fromBackingInt(2), .provenance = .creator, .actor = null }, .{ .grant = .{ .settlement = @fromBackingInt(1), .food = 0, .material = 0, .population = 0, .economic_bypass = true } }, &bytes[1]),
        try command.encodeEnvelope(.{ .sequence = @fromBackingInt(3), .provenance = .creator, .actor = null }, .{ .grant = .{ .settlement = @fromBackingInt(1), .food = 1, .material = 0, .population = 0, .economic_bypass = true } }, &bytes[2]),
    };
    const cursor = try run(&candidate, try pe.SessionPolicy.init(.creator, null), &envelopes);
    try std.testing.expectEqual(@as(u16, 1), cursor.accepted);
    try std.testing.expectEqual(pe.RejectReason.InvalidGrant, authority.storage.logicalReceipt(1).event.rejected.reason);
    try std.testing.expectEqual(pe.RejectReason.InventoryCapacity, authority.storage.logicalReceipt(2).event.rejected.reason);
    try std.testing.expectEqual(@as(u32, 65_535), (try candidate.readRecord(.settlement, 0)).settlement.food);
    try discard(&candidate);
}

test "pe/commands/creator house changes housing before later grant" {
    var authority = try makeAuthority();
    defer authority.deinit();
    var candidate = pe.state.Candidate.begin(&authority.storage);
    try prepare(&candidate);
    var bytes: [6][1024]u8 = undefined;
    const envelopes = [_]pe.Envelope{
        try authoredSettlement(1, 70, 0, 0, 0, &bytes[0]),
        try command.encodeEnvelope(.{ .sequence = @fromBackingInt(2), .provenance = .creator, .actor = null }, .{ .author_build = .{
            .settlement = @fromBackingInt(1),
            .kind = .quarry,
            .anchor = .{ .planet = pe.planet_id, .local = 71 },
            .facing = null,
            .economic_bypass = true,
        } }, &bytes[1]),
        try command.encodeEnvelope(.{ .sequence = @fromBackingInt(3), .provenance = .creator, .actor = null }, .{ .author_build = .{
            .settlement = @fromBackingInt(1),
            .kind = .house,
            .anchor = .{ .planet = pe.planet_id, .local = 71 },
            .facing = .east,
            .economic_bypass = true,
        } }, &bytes[2]),
        try command.encodeEnvelope(.{ .sequence = @fromBackingInt(4), .provenance = .creator, .actor = null }, .{ .author_build = .{
            .settlement = @fromBackingInt(1),
            .kind = .house,
            .anchor = .{ .planet = pe.planet_id, .local = 71 },
            .facing = null,
            .economic_bypass = true,
        } }, &bytes[3]),
        try command.encodeEnvelope(.{ .sequence = @fromBackingInt(5), .provenance = .creator, .actor = null }, .{ .grant = .{
            .settlement = @fromBackingInt(1),
            .food = 0,
            .material = 0,
            .population = 8,
            .economic_bypass = true,
        } }, &bytes[4]),
        try command.encodeEnvelope(.{ .sequence = @fromBackingInt(6), .provenance = .creator, .actor = null }, .{ .grant = .{
            .settlement = @fromBackingInt(1),
            .food = 0,
            .material = 0,
            .population = 1,
            .economic_bypass = true,
        } }, &bytes[5]),
    };
    const cursor = try run(&candidate, try pe.SessionPolicy.init(.creator, null), &envelopes);
    try std.testing.expectEqual(@as(u16, 3), cursor.accepted);
    try std.testing.expectEqual(@as(u16, 3), cursor.rejected);
    try std.testing.expectEqual(pe.RejectReason.TerrainMismatch, authority.storage.logicalReceipt(1).event.rejected.reason);
    try std.testing.expectEqual(pe.RejectReason.InvalidFacing, authority.storage.logicalReceipt(2).event.rejected.reason);
    try std.testing.expectEqual(pe.RejectReason.PopulationCapacity, authority.storage.logicalReceipt(5).event.rejected.reason);
    const settlement = (try candidate.readRecord(.settlement, 0)).settlement;
    try std.testing.expectEqual(@as(u8, 8), settlement.housing);
    try std.testing.expectEqual(@as(u8, 8), settlement.population);
    try std.testing.expectEqual(@as(u8, 0), (try candidate.readRecord(.building, 0)).building.remaining_ticks);
    try discard(&candidate);
}

test "pe/commands/configured settlement building and freight capacities reject without IDs" {
    {
        var authority = try makeAuthorityWithConfig(.{ .seed = 2, .side = 4, .capacities = .{ .settlements = 1 } });
        defer authority.deinit();
        var candidate = pe.state.Candidate.begin(&authority.storage);
        try prepare(&candidate);
        var bytes: [2][1024]u8 = undefined;
        const envelopes = [_]pe.Envelope{
            try authoredSettlement(1, 70, 0, 0, 0, &bytes[0]),
            try authoredSettlement(2, 5, 0, 0, 0, &bytes[1]),
        };
        const cursor = try run(&candidate, try pe.SessionPolicy.init(.creator, null), &envelopes);
        try std.testing.expectEqual(@as(u16, 1), cursor.accepted);
        try std.testing.expectEqual(pe.RejectReason.SettlementCapacity, authority.storage.logicalReceipt(1).event.rejected.reason);
        try std.testing.expectEqual(@as(u64, 2), candidate.meta().next_settlement);
        try discard(&candidate);
    }
    {
        var authority = try makeAuthorityWithConfig(.{ .seed = 2, .side = 4, .capacities = .{ .buildings = 1 } });
        defer authority.deinit();
        var candidate = pe.state.Candidate.begin(&authority.storage);
        try prepare(&candidate);
        var bytes: [3][1024]u8 = undefined;
        const envelopes = [_]pe.Envelope{
            try authoredSettlement(1, 70, 0, 0, 0, &bytes[0]),
            try command.encodeEnvelope(.{ .sequence = @fromBackingInt(2), .provenance = .creator, .actor = null }, .{ .author_build = .{
                .settlement = @fromBackingInt(1),
                .kind = .house,
                .anchor = .{ .planet = pe.planet_id, .local = 71 },
                .facing = null,
                .economic_bypass = true,
            } }, &bytes[1]),
            try command.encodeEnvelope(.{ .sequence = @fromBackingInt(3), .provenance = .creator, .actor = null }, .{ .author_build = .{
                .settlement = @fromBackingInt(1),
                .kind = .house,
                .anchor = .{ .planet = pe.planet_id, .local = 69 },
                .facing = null,
                .economic_bypass = true,
            } }, &bytes[2]),
        };
        const cursor = try run(&candidate, try pe.SessionPolicy.init(.creator, null), &envelopes);
        try std.testing.expectEqual(@as(u16, 2), cursor.accepted);
        try std.testing.expectEqual(pe.RejectReason.BuildingCapacity, authority.storage.logicalReceipt(2).event.rejected.reason);
        try std.testing.expectEqual(@as(u64, 2), candidate.meta().next_building);
        try discard(&candidate);
    }
    {
        var authority = try makeAuthorityWithConfig(.{ .seed = 2, .side = 4, .capacities = .{ .shipments = 1 } });
        defer authority.deinit();
        var candidate = pe.state.Candidate.begin(&authority.storage);
        try prepare(&candidate);
        var bytes: [4][1024]u8 = undefined;
        const route = [_]pe.CellId{
            .{ .planet = pe.planet_id, .local = 70 }, .{ .planet = pe.planet_id, .local = 71 },
            .{ .planet = pe.planet_id, .local = 4 },  .{ .planet = pe.planet_id, .local = 5 },
        };
        const envelopes = [_]pe.Envelope{
            try authoredSettlement(1, 70, 0, 0, 0, &bytes[0]),
            try authoredSettlement(2, 5, 0, 0, 0, &bytes[1]),
            try command.encodeEnvelope(.{ .sequence = @fromBackingInt(3), .provenance = .creator, .actor = null }, .{ .author_ship = .{
                .source = @fromBackingInt(1),
                .destination = @fromBackingInt(2),
                .resource = .food,
                .amount = 1,
                .path = &route,
                .cargo_source = .mint,
                .economic_bypass = true,
            } }, &bytes[2]),
            try command.encodeEnvelope(.{ .sequence = @fromBackingInt(4), .provenance = .creator, .actor = null }, .{ .author_ship = .{
                .source = @fromBackingInt(1),
                .destination = @fromBackingInt(2),
                .resource = .food,
                .amount = 1,
                .path = &route,
                .cargo_source = .mint,
                .economic_bypass = true,
            } }, &bytes[3]),
        };
        const cursor = try run(&candidate, try pe.SessionPolicy.init(.creator, null), &envelopes);
        try std.testing.expectEqual(@as(u16, 3), cursor.accepted);
        try std.testing.expectEqual(pe.RejectReason.FreightCapacity, authority.storage.logicalReceipt(3).event.rejected.reason);
        try std.testing.expectEqual(@as(u64, 2), candidate.meta().next_shipment);
        try discard(&candidate);
    }
}

test "pe/commands/per-settlement building limit is 32 with next ID unchanged" {
    var authority = try makeAuthorityWithSide(8);
    defer authority.deinit();
    var candidate = pe.state.Candidate.begin(&authority.storage);
    try prepare(&candidate);
    var bytes: [34][1024]u8 = undefined;
    var envelopes: [34]pe.Envelope = undefined;
    envelopes[0] = try authoredSettlement(1, 256, 0, 0, 0, &bytes[0]);
    for (1..34) |index| {
        const row = index / 8;
        const column = index % 8;
        const x = if (row % 2 == 0) column else 7 - column;
        const cell = 256 + row * 8 + x;
        envelopes[index] = try command.encodeEnvelope(.{ .sequence = @fromBackingInt(index + 1), .provenance = .creator, .actor = null }, .{ .author_build = .{ .settlement = @fromBackingInt(1), .kind = .house, .anchor = .{ .planet = pe.planet_id, .local = cell }, .facing = null, .economic_bypass = true } }, &bytes[index]);
    }
    const cursor = try runWithBudget(&candidate, try pe.SessionPolicy.init(.creator, null), &envelopes, 7);
    try std.testing.expectEqual(@as(u16, 33), cursor.accepted);
    try std.testing.expectEqual(@as(u16, 1), cursor.rejected);
    try std.testing.expectEqual(pe.RejectReason.SettlementBuildingCapacity, authority.storage.logicalReceipt(33).event.rejected.reason);
    try std.testing.expectEqual(@as(u16, 32), candidate.meta().buildings);
    try std.testing.expectEqual(@as(u64, 33), candidate.meta().next_building);
    try std.testing.expectEqual(@as(u8, 32), (try candidate.readRecord(.settlement, 0)).settlement.housing);
    try discard(&candidate);
}

test "pe/commands/path validates range endpoints cells and edges before cargo" {
    var authority = try makeAuthority();
    defer authority.deinit();
    var candidate = pe.state.Candidate.begin(&authority.storage);
    try prepare(&candidate);
    const one = [_]pe.CellId{.{ .planet = pe.planet_id, .local = 70 }};
    const wrong_start = [_]pe.CellId{ .{ .planet = pe.planet_id, .local = 71 }, .{ .planet = pe.planet_id, .local = 4 }, .{ .planet = pe.planet_id, .local = 5 } };
    const nonadjacent = [_]pe.CellId{ .{ .planet = pe.planet_id, .local = 70 }, .{ .planet = pe.planet_id, .local = 5 } };
    const foreign_planet = [_]pe.CellId{ .{ .planet = pe.planet_id, .local = 70 }, .{ .planet = @fromBackingInt(2), .local = 71 }, .{ .planet = pe.planet_id, .local = 5 } };
    const invalid_cell = [_]pe.CellId{ .{ .planet = pe.planet_id, .local = 70 }, .{ .planet = pe.planet_id, .local = 96 }, .{ .planet = pe.planet_id, .local = 5 } };
    var bytes: [10][1024]u8 = undefined;
    var envelopes: [10]pe.Envelope = undefined;
    envelopes[0] = try authoredSettlement(1, 70, 0, 0, 0, &bytes[0]);
    envelopes[1] = try authoredSettlement(2, 5, 0, 0, 0, &bytes[1]);
    const paths = [_][]const pe.CellId{ &one, &wrong_start, &nonadjacent, &foreign_planet, &invalid_cell };
    for (paths, 0..) |path, index| {
        envelopes[index + 2] = try command.encodeEnvelope(.{ .sequence = @fromBackingInt(index + 3), .provenance = .creator, .actor = null }, .{ .author_ship = .{ .source = @fromBackingInt(1), .destination = @fromBackingInt(2), .resource = .food, .amount = 1, .path = path, .cargo_source = .mint, .economic_bypass = true } }, &bytes[index + 2]);
    }
    envelopes[7] = try command.encodeEnvelope(.{ .sequence = @fromBackingInt(8), .provenance = .creator, .actor = null }, .{ .author_ship = .{ .source = @fromBackingInt(1), .destination = @fromBackingInt(1), .resource = .food, .amount = 1, .path = &one, .cargo_source = .mint, .economic_bypass = true } }, &bytes[7]);
    for ([_]u16{ 0, 257 }, 0..) |amount, index| {
        envelopes[index + 8] = try command.encodeEnvelope(.{ .sequence = @fromBackingInt(index + 9), .provenance = .creator, .actor = null }, .{ .author_ship = .{ .source = @fromBackingInt(1), .destination = @fromBackingInt(2), .resource = .food, .amount = amount, .path = &one, .cargo_source = .mint, .economic_bypass = true } }, &bytes[index + 8]);
    }
    const cursor = try run(&candidate, try pe.SessionPolicy.init(.creator, null), &envelopes);
    try std.testing.expectEqual(@as(u16, 2), cursor.accepted);
    try std.testing.expectEqual(@as(u16, 8), cursor.rejected);
    const expected = [_]pe.RejectReason{ .InvalidPathLength, .PathEndpointMismatch, .NonAdjacentPath, .InvalidPlanet, .InvalidCell, .SameEndpoint, .InvalidAmount, .InvalidAmount };
    const indexes = [_]?u16{ null, 0, 1, 1, 1, null, null, null };
    for (expected, 0..) |reason, index| {
        const rejected = authority.storage.logicalReceipt(@intCast(index + 2)).event.rejected;
        try std.testing.expectEqual(reason, rejected.reason);
        try std.testing.expectEqual(indexes[index], rejected.offending_index);
    }
    try std.testing.expectEqual(@as(u64, 1), candidate.meta().next_shipment);
    try discard(&candidate);
}

test "pe/commands/found checks food material then population without partial debit" {
    var authority = try makeAuthority();
    defer authority.deinit();
    var candidate = pe.state.Candidate.begin(&authority.storage);
    try prepare(&candidate);
    var bytes: [8][1024]u8 = undefined;
    var envelopes: [8]pe.Envelope = undefined;
    envelopes[0] = try authoredSettlement(1, 70, 1, 3, 19, &bytes[0]);
    for ([_]usize{ 1, 3, 5, 7 }) |index| {
        envelopes[index] = try command.encodeEnvelope(.{ .sequence = @fromBackingInt(index + 1), .provenance = .ruler, .actor = pe.civilization_id }, .{ .found = .{ .source = @fromBackingInt(1), .anchor = .{ .planet = pe.planet_id, .local = 5 } } }, &bytes[index]);
    }
    envelopes[2] = try command.encodeEnvelope(.{ .sequence = @fromBackingInt(3), .provenance = .creator, .actor = null }, .{ .grant = .{ .settlement = @fromBackingInt(1), .food = 1, .material = 0, .population = 0, .economic_bypass = true } }, &bytes[2]);
    envelopes[4] = try command.encodeEnvelope(.{ .sequence = @fromBackingInt(5), .provenance = .creator, .actor = null }, .{ .grant = .{ .settlement = @fromBackingInt(1), .food = 0, .material = 1, .population = 0, .economic_bypass = true } }, &bytes[4]);
    envelopes[6] = try command.encodeEnvelope(.{ .sequence = @fromBackingInt(7), .provenance = .creator, .actor = null }, .{ .grant = .{ .settlement = @fromBackingInt(1), .food = 0, .material = 0, .population = 1, .economic_bypass = true } }, &bytes[6]);
    const cursor = try run(&candidate, try pe.SessionPolicy.init(.combined, pe.civilization_id), &envelopes);
    try std.testing.expectEqual(@as(u16, 5), cursor.accepted);
    try std.testing.expectEqual(@as(u16, 3), cursor.rejected);
    try std.testing.expectEqual(pe.RejectReason.InsufficientFood, authority.storage.logicalReceipt(1).event.rejected.reason);
    try std.testing.expectEqual(pe.RejectReason.InsufficientMaterial, authority.storage.logicalReceipt(3).event.rejected.reason);
    try std.testing.expectEqual(pe.RejectReason.InsufficientPopulation, authority.storage.logicalReceipt(5).event.rejected.reason);
    try std.testing.expectEqual(@as(u64, 2), @backingInt(authority.storage.logicalReceipt(7).event.accepted.effect.found.settlement));
    try std.testing.expectEqual(@as(u64, 3), candidate.meta().next_settlement);
    const source = (try candidate.readRecord(.settlement, 0)).settlement;
    try std.testing.expectEqual(@as(u8, 0), source.population);
    try std.testing.expectEqual(@as(u32, 0), source.food);
    try std.testing.expectEqual(@as(u32, 0), source.material);
    try discard(&candidate);
}

test "pe/commands/maximum 64 envelopes admit and 65 abort structurally" {
    var authority = try makeAuthority();
    defer authority.deinit();
    var candidate = pe.state.Candidate.begin(&authority.storage);
    try prepare(&candidate);
    var bytes: [64][1024]u8 = undefined;
    var envelopes: [64]pe.Envelope = undefined;
    envelopes[0] = try authoredSettlement(1, 70, 0, 0, 0, &bytes[0]);
    for (1..64) |index| envelopes[index] = try command.encodeEnvelope(.{
        .sequence = @fromBackingInt(index + 1),
        .provenance = .creator,
        .actor = null,
    }, .{ .grant = .{ .settlement = @fromBackingInt(1), .food = 1, .material = 0, .population = 0, .economic_bypass = true } }, &bytes[index]);
    const cursor = try runWithBudget(&candidate, try pe.SessionPolicy.init(.creator, null), &envelopes, 7);
    try std.testing.expectEqual(@as(u16, 64), cursor.accepted);
    try std.testing.expectEqual(@as(u32, 63), (try candidate.readRecord(.settlement, 0)).settlement.food);
    try std.testing.expectEqual(@as(u16, 64), authority.storage.staged_count);
    var too_many: [65]pe.Envelope = undefined;
    @memcpy(too_many[0..64], &envelopes);
    too_many[64] = envelopes[0];
    var invalid_cursor = command.Cursor.init(try pe.SessionPolicy.init(.creator, null), &too_many, @fromBackingInt(0), @fromBackingInt(1));
    var budget: pe.WorkBudget = .{ .remaining = 7 };
    try std.testing.expectError(error.TooManyCommands, invalid_cursor.advance(&candidate, &budget));
    try std.testing.expectEqual(@as(u16, 64), authority.storage.staged_count);
    try discard(&candidate);
}

test "pe/commands/freight rejects arrival beyond tick horizon" {
    var authority = try makeAuthority();
    defer authority.deinit();
    var candidate = pe.state.Candidate.begin(&authority.storage);
    try prepare(&candidate);
    var bytes: [3][1024]u8 = undefined;
    const route = [_]pe.CellId{
        .{ .planet = pe.planet_id, .local = 70 }, .{ .planet = pe.planet_id, .local = 71 },
        .{ .planet = pe.planet_id, .local = 4 },  .{ .planet = pe.planet_id, .local = 5 },
    };
    const envelopes = [_]pe.Envelope{
        try authoredSettlement(1, 70, 0, 0, 0, &bytes[0]),
        try authoredSettlement(2, 5, 0, 0, 0, &bytes[1]),
        try command.encodeEnvelope(.{ .sequence = @fromBackingInt(3), .provenance = .creator, .actor = null }, .{ .author_ship = .{ .source = @fromBackingInt(1), .destination = @fromBackingInt(2), .resource = .food, .amount = 1, .path = &route, .cargo_source = .mint, .economic_bypass = true } }, &bytes[2]),
    };
    var cursor = command.Cursor.init(try pe.SessionPolicy.init(.creator, null), &envelopes, @fromBackingInt(0), @fromBackingInt(1_000_000));
    while (true) {
        var budget: pe.WorkBudget = .{ .remaining = 7 };
        if (try cursor.advance(&candidate, &budget) == .done) break;
    }
    try std.testing.expectEqual(@as(u16, 2), cursor.accepted);
    try std.testing.expectEqual(pe.RejectReason.TickHorizon, authority.storage.logicalReceipt(2).event.rejected.reason);
    try std.testing.expectEqual(@as(u64, 1), candidate.meta().next_shipment);
    try discard(&candidate);
}

test "pe/commands/author settlement hall range reports index zero without writes" {
    var authority = try makeAuthority();
    defer authority.deinit();
    var candidate = pe.state.Candidate.begin(&authority.storage);
    try prepare(&candidate);
    const original_claim = try candidate.readRecord(.claim, 70);
    var bytes: [2][1024]u8 = undefined;
    const envelopes = [_]pe.Envelope{
        try command.encodeEnvelope(.{ .sequence = @fromBackingInt(1), .provenance = .creator, .actor = null }, .{ .author_settlement = .{ .owner = pe.civilization_id, .anchor = .{ .planet = @fromBackingInt(2), .local = 70 }, .population = 0, .food = 0, .material = 0, .economic_bypass = true } }, &bytes[0]),
        try command.encodeEnvelope(.{ .sequence = @fromBackingInt(2), .provenance = .creator, .actor = null }, .{ .author_settlement = .{ .owner = pe.civilization_id, .anchor = .{ .planet = pe.planet_id, .local = 96 }, .population = 0, .food = 0, .material = 0, .economic_bypass = true } }, &bytes[1]),
    };
    const cursor = try run(&candidate, try pe.SessionPolicy.init(.creator, null), &envelopes);
    try std.testing.expectEqual(@as(u16, 0), cursor.accepted);
    try std.testing.expectEqual(@as(u16, 2), cursor.rejected);
    try std.testing.expectEqual(pe.RejectReason.InvalidPlanet, authority.storage.logicalReceipt(0).event.rejected.reason);
    try std.testing.expectEqual(pe.RejectReason.InvalidCell, authority.storage.logicalReceipt(1).event.rejected.reason);
    for (0..2) |i| try std.testing.expectEqual(@as(?u16, 0), authority.storage.logicalReceipt(@intCast(i)).event.rejected.offending_index);
    try std.testing.expectEqual(@as(u64, 1), candidate.meta().next_settlement);
    try std.testing.expectEqual(@as(u16, 0), candidate.meta().settlements);
    try std.testing.expectEqualDeep(original_claim, try candidate.readRecord(.claim, 70));
    try discard(&candidate);
}

test "pe/commands/paid found hall range reports index zero without debit" {
    var authority = try makeAuthority();
    defer authority.deinit();
    var candidate = pe.state.Candidate.begin(&authority.storage);
    try prepare(&candidate);
    var bytes: [3][1024]u8 = undefined;
    const envelopes = [_]pe.Envelope{
        try authoredSettlement(1, 70, 4, 8, 20, &bytes[0]),
        try command.encodeEnvelope(.{ .sequence = @fromBackingInt(2), .provenance = .ruler, .actor = pe.civilization_id }, .{ .found = .{ .source = @fromBackingInt(1), .anchor = .{ .planet = @fromBackingInt(2), .local = 71 } } }, &bytes[1]),
        try command.encodeEnvelope(.{ .sequence = @fromBackingInt(3), .provenance = .ruler, .actor = pe.civilization_id }, .{ .found = .{ .source = @fromBackingInt(1), .anchor = .{ .planet = pe.planet_id, .local = 96 } } }, &bytes[2]),
    };
    const cursor = try run(&candidate, try pe.SessionPolicy.init(.combined, pe.civilization_id), &envelopes);
    try std.testing.expectEqual(@as(u16, 1), cursor.accepted);
    try std.testing.expectEqual(@as(u16, 2), cursor.rejected);
    try std.testing.expectEqual(pe.RejectReason.InvalidPlanet, authority.storage.logicalReceipt(1).event.rejected.reason);
    try std.testing.expectEqual(pe.RejectReason.InvalidCell, authority.storage.logicalReceipt(2).event.rejected.reason);
    for (1..3) |i| try std.testing.expectEqual(@as(?u16, 0), authority.storage.logicalReceipt(@intCast(i)).event.rejected.offending_index);
    const source = (try candidate.readRecord(.settlement, 0)).settlement;
    try std.testing.expectEqual(@as(u32, 8), source.food);
    try std.testing.expectEqual(@as(u32, 20), source.material);
    try std.testing.expectEqual(@as(u8, 4), source.population);
    try std.testing.expectEqual(@as(u64, 2), candidate.meta().next_settlement);
    try std.testing.expectEqual(@as(u16, 1), candidate.meta().settlements);
    try std.testing.expectEqual(@as(?pe.SettlementId, null), (try candidate.readRecord(.claim, 71)).claim.settlement);
    try std.testing.expectEqual(@as(?pe.SettlementId, @fromBackingInt(1)), (try candidate.readRecord(.claim, 70)).claim.settlement);
    try discard(&candidate);
}

test "pe/commands/authored and paid freight share range precedence" {
    var authority = try makeAuthority();
    defer authority.deinit();
    var candidate = pe.state.Candidate.begin(&authority.storage);
    try prepare(&candidate);
    var bytes: [8][1024]u8 = undefined;
    const path = [_]pe.CellId{ .{ .planet = pe.planet_id, .local = 70 }, .{ .planet = pe.planet_id, .local = 71 } };
    const short_path = path[0..1];
    var envelopes: [8]pe.Envelope = undefined;
    envelopes[0] = try authoredSettlement(1, 70, 0, 8, 0, &bytes[0]);
    envelopes[1] = try authoredSettlement(2, 71, 0, 0, 0, &bytes[1]);
    for (0..6) |i| {
        const same = i % 3 == 0;
        const invalid_amount = i % 3 == 1;
        const route = if (i % 3 == 2) short_path else &path;
        const destination: pe.SettlementId = @fromBackingInt(if (same) 1 else 2);
        const amount: u16 = if (invalid_amount) 0 else 1;
        envelopes[i + 2] = if (i < 3)
            try command.encodeEnvelope(.{ .sequence = @fromBackingInt(i + 3), .provenance = .creator, .actor = null }, .{ .author_ship = .{ .source = @fromBackingInt(1), .destination = destination, .resource = .food, .amount = amount, .path = route, .cargo_source = .mint, .economic_bypass = true } }, &bytes[i + 2])
        else
            try command.encodeEnvelope(.{ .sequence = @fromBackingInt(i + 3), .provenance = .ruler, .actor = pe.civilization_id }, .{ .ship = .{ .source = @fromBackingInt(1), .destination = destination, .resource = .food, .amount = amount, .path = route } }, &bytes[i + 2]);
    }
    const cursor = try run(&candidate, try pe.SessionPolicy.init(.combined, pe.civilization_id), &envelopes);
    try std.testing.expectEqual(@as(u16, 2), cursor.accepted);
    try std.testing.expectEqual(@as(u16, 6), cursor.rejected);
    const reasons = [_]pe.RejectReason{ .SameEndpoint, .InvalidAmount, .InvalidPathLength };
    for (0..6) |i| try std.testing.expectEqual(reasons[i % 3], authority.storage.logicalReceipt(@intCast(i + 2)).event.rejected.reason);
    try std.testing.expectEqual(@as(u64, 1), candidate.meta().next_shipment);
    try std.testing.expectEqual(@as(u32, 8), (try candidate.readRecord(.settlement, 0)).settlement.food);
    try discard(&candidate);
}
