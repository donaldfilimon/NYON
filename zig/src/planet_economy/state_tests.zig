const std = @import("std");
const pe = @import("root.zig");

fn finishCreate(builder: *pe.Builder, units: u16) !void {
    while (true) {
        var budget: pe.WorkBudget = .{ .remaining = units };
        switch (pe.pollCreate(builder, &budget)) {
            .pending => {},
            .ready => return,
            .failed => |failure| return failure,
            else => return error.UnexpectedProgress,
        }
    }
}

fn finishWrite(candidate: *pe.state.Candidate, index: u16, record: pe.state.Record) !void {
    var writer = pe.state.RecordWriter.init(index, record);
    while (true) {
        var budget: pe.WorkBudget = .{ .remaining = 256 };
        if (try writer.advance(candidate, &budget) == .done) return;
    }
}

test "pe/state/max-layout reserves the full advertised bound" {
    var builder = try pe.beginCreate(std.testing.allocator, .{ .seed = 2, .side = 32 });
    defer builder.deinit();
    try std.testing.expectEqual(@as(usize, 8_642_560), builder.stats().requested_bytes);
    try std.testing.expectEqualSlices(u16, &.{ 24, 768, 64, 128, 32, 192, 8 }, &pe.state.region_pages);
    try std.testing.expectEqual(@as(usize, 1216), pe.state.addressed_pages);
    try finishCreate(&builder, 256);
    var authority = try pe.takeAuthority(&builder);
    defer authority.deinit();
    try std.testing.expectEqual(@as(usize, 8_642_560), authority.stats().requested_bytes);
    try std.testing.expectEqual(@as(u16, 1), authority.stats().history_used);
    try std.testing.expectError(error.NotFound, authority.settlement(@fromBackingInt(1)));
    const cell = try authority.cell(.{ .planet = pe.planet_id, .local = 6143 });
    try std.testing.expectEqual(pe.Terrain.fertile, cell.terrain);
}

test "pe/state/genesis-cursor zero budget leaves cursor and world unchanged" {
    var builder = try pe.beginCreate(std.testing.allocator, .{ .seed = 2, .side = 4 });
    defer builder.deinit();
    const before = builder.cursor;
    var zero: pe.WorkBudget = .{ .remaining = 0 };
    try std.testing.expectEqual(pe.CreateProgress.pending, pe.pollCreate(&builder, &zero));
    try std.testing.expectEqualDeep(before, builder.cursor);
    try finishCreate(&builder, 1);
    var authority = try pe.takeAuthority(&builder);
    defer authority.deinit();
    const cell71: pe.CellId = .{ .planet = pe.planet_id, .local = 71 };
    const before_cell = try authority.cell(cell71);
    try std.testing.expectEqual(pe.Terrain.fertile, before_cell.terrain);
    try std.testing.expectEqualDeep(before_cell, try authority.cell(cell71));
    try std.testing.expectEqual(@as(u64, 0), @backingInt(authority.summary().tick));
    var fast_builder = try pe.beginCreate(std.testing.allocator, .{ .seed = 2, .side = 4 });
    defer fast_builder.deinit();
    try finishCreate(&fast_builder, 256);
    var fast = try pe.takeAuthority(&fast_builder);
    defer fast.deinit();
    for (0..96) |local| {
        const id: pe.CellId = .{ .planet = pe.planet_id, .local = local };
        try std.testing.expectEqualDeep(try authority.cell(id), try fast.cell(id));
    }
    try std.testing.expectEqual(authority.stats().work_units, fast.stats().work_units);
}

test "pe/state/cow-isolation keeps the published claim unchanged" {
    var builder = try pe.beginCreate(std.testing.allocator, .{ .seed = 2, .side = 4 });
    defer builder.deinit();
    try finishCreate(&builder, 256);
    var authority = try pe.takeAuthority(&builder);
    defer authority.deinit();
    const cell71: pe.CellId = .{ .planet = pe.planet_id, .local = 71 };
    const before_cell = try authority.cell(cell71);
    var candidate = pe.state.Candidate.begin(&authority.storage);
    var budget: pe.WorkBudget = .{ .remaining = 256 };
    while (try candidate.advancePrepare(&budget) == .pending) budget.remaining = 256;
    var writer = pe.state.RecordWriter.init(71, .{ .claim = .{ .settlement = @fromBackingInt(1), .building = null, .is_hall = true } });
    budget.remaining = 1;
    while (try writer.advance(&candidate, &budget) == .pending) budget.remaining = 1;
    try std.testing.expectEqualDeep(before_cell, try authority.cell(cell71));
    try std.testing.expectEqual(@as(u64, 1), @backingInt((try candidate.readRecord(.claim, 71)).claim.settlement.?));
    budget.remaining = 256;
    while (try candidate.advanceDiscard(&budget) == .pending) budget.remaining = 256;
    try std.testing.expectEqualDeep(before_cell, try authority.cell(cell71));
    try std.testing.expectEqual(@as(u16, 1216), authority.stats().live_pages);
}

test "pe/state/setup-oom releases partial reservation" {
    var storage: [128]u8 = undefined;
    var fba = std.heap.FixedBufferAllocator.init(&storage);
    try std.testing.expectError(error.OutOfMemory, pe.beginCreate(fba.allocator(), .{ .seed = 2 }));
}

test "pe/state/partial-retain discard releases a page retained before directory copy" {
    var builder = try pe.beginCreate(std.testing.allocator, .{ .seed = 2, .side = 4 });
    defer builder.deinit();
    try finishCreate(&builder, 256);
    var authority = try pe.takeAuthority(&builder);
    defer authority.deinit();
    const ref = authority.storage.reservation.directories[authority.storage.active][0];
    var candidate = pe.state.Candidate.begin(&authority.storage);
    var one: pe.WorkBudget = .{ .remaining = 1 };
    try std.testing.expectEqual(pe.Step.pending, try candidate.advancePrepare(&one));
    try std.testing.expectEqual(@as(u16, 2), authority.storage.reservation.pool[ref - 1].references);
    one.remaining = 1;
    try std.testing.expectEqual(pe.Step.pending, try candidate.advanceDiscard(&one));
    one.remaining = 1;
    try std.testing.expectEqual(pe.Step.done, try candidate.advanceDiscard(&one));
    try std.testing.expectEqual(@as(u16, 1), authority.storage.reservation.pool[ref - 1].references);
}

test "pe/state/cancel-create drains allocated pages before deinit" {
    var builder = try pe.beginCreate(std.testing.allocator, .{ .seed = 2, .side = 32 });
    var budget: pe.WorkBudget = .{ .remaining = 256 };
    while (builder.cursor.allocated_pages < 3) {
        budget.remaining = 256;
        try std.testing.expectEqual(pe.CreateProgress.pending, pe.pollCreate(&builder, &budget));
    }
    pe.cancelCreate(&builder);
    while (true) {
        budget.remaining = 1;
        switch (pe.pollCreate(&builder, &budget)) {
            .pending => {},
            .canceled => break,
            else => return error.UnexpectedProgress,
        }
    }
    try std.testing.expectEqual(@as(u16, 0), builder.stats().live_pages);
    builder.deinit();
}

test "pe/state/cancel-create drains a page allocated before directory insertion" {
    var builder = try pe.beginCreate(std.testing.allocator, .{ .seed = 2, .side = 4 });
    while (builder.cursor.phase != .allocate_pages or builder.cursor.substep != 0) {
        var budget: pe.WorkBudget = .{ .remaining = 256 };
        _ = pe.pollCreate(&builder, &budget);
    }
    const before_pages = builder.stats().live_pages;
    var one: pe.WorkBudget = .{ .remaining = 1 };
    try std.testing.expectEqual(pe.CreateProgress.pending, pe.pollCreate(&builder, &one));
    try std.testing.expectEqual(before_pages + 1, builder.stats().live_pages);
    try std.testing.expectEqual(@as(u64, 0), builder.storage.?.reservation.directories[0][builder.cursor.index]);
    pe.cancelCreate(&builder);
    var zero: pe.WorkBudget = .{ .remaining = 0 };
    const before = builder.cursor;
    try std.testing.expectEqual(pe.CreateProgress.pending, pe.pollCreate(&builder, &zero));
    try std.testing.expectEqualDeep(before, builder.cursor);
    while (true) {
        one.remaining = 1;
        switch (pe.pollCreate(&builder, &one)) {
            .pending => {},
            .canceled => break,
            else => return error.UnexpectedProgress,
        }
    }
    try std.testing.expectEqual(@as(u16, 0), builder.stats().live_pages);
    builder.deinit();
}

test "pe/state/partial-clone discard frees an unattached page" {
    var builder = try pe.beginCreate(std.testing.allocator, .{ .seed = 2, .side = 4 });
    defer builder.deinit();
    try finishCreate(&builder, 256);
    var authority = try pe.takeAuthority(&builder);
    defer authority.deinit();
    var candidate = pe.state.Candidate.begin(&authority.storage);
    var budget: pe.WorkBudget = .{ .remaining = 256 };
    while (try candidate.advancePrepare(&budget) == .pending) budget.remaining = 256;
    var writer = pe.state.RecordWriter.init(71, .{ .claim = .{ .settlement = @fromBackingInt(1) } });
    budget.remaining = 2;
    try std.testing.expectEqual(pe.Step.pending, try writer.advance(&candidate, &budget));
    try std.testing.expectEqual(@as(u16, 1217), authority.stats().live_pages);
    budget.remaining = 256;
    while (try candidate.advanceDiscard(&budget) == .pending) budget.remaining = 256;
    try std.testing.expectEqual(@as(u16, 1216), authority.stats().live_pages);
}

test "pe/state/swapped-clone discard keeps the published page" {
    var builder = try pe.beginCreate(std.testing.allocator, .{ .seed = 2, .side = 4 });
    defer builder.deinit();
    try finishCreate(&builder, 256);
    var authority = try pe.takeAuthority(&builder);
    defer authority.deinit();
    const cell71: pe.CellId = .{ .planet = pe.planet_id, .local = 71 };
    const before_cell = try authority.cell(cell71);
    const active_ref = authority.storage.reservation.directories[authority.storage.active][24 + 71 / 8];
    var candidate = pe.state.Candidate.begin(&authority.storage);
    var budget: pe.WorkBudget = .{ .remaining = 256 };
    while (try candidate.advancePrepare(&budget) == .pending) budget.remaining = 256;
    var writer = pe.state.RecordWriter.init(71, .{ .claim = .{ .settlement = @fromBackingInt(1) } });
    budget.remaining = 4;
    try std.testing.expectEqual(pe.Step.pending, try writer.advance(&candidate, &budget));
    budget.remaining = 256;
    while (try candidate.advanceDiscard(&budget) == .pending) budget.remaining = 256;
    try std.testing.expectEqualDeep(before_cell, try authority.cell(cell71));
    try std.testing.expectEqual(@as(u16, 1216), authority.stats().live_pages);
    try std.testing.expectEqual(@as(u16, 1), authority.storage.reservation.pool[active_ref - 1].references);
}

test "pe/state/publish cleanup releases the old root" {
    var builder = try pe.beginCreate(std.testing.allocator, .{ .seed = 2, .side = 4 });
    defer builder.deinit();
    try finishCreate(&builder, 256);
    var authority = try pe.takeAuthority(&builder);
    defer authority.deinit();
    var candidate = pe.state.Candidate.begin(&authority.storage);
    var budget: pe.WorkBudget = .{ .remaining = 256 };
    while (try candidate.advancePrepare(&budget) == .pending) budget.remaining = 256;
    var writer = pe.state.RecordWriter.init(71, .{ .claim = .{ .settlement = @fromBackingInt(1), .is_hall = true } });
    budget.remaining = 256;
    while (try writer.advance(&candidate, &budget) == .pending) budget.remaining = 256;
    var metadata = candidate.meta();
    metadata.revision = @fromBackingInt(1);
    var meta_writer = pe.state.MetaWriter.init(metadata);
    budget.remaining = 256;
    while (try meta_writer.advance(&candidate, &budget) == .pending) budget.remaining = 256;
    candidate.publish();
    budget.remaining = 256;
    while (try candidate.advanceDiscard(&budget) == .pending) budget.remaining = 256;
    const cell71 = try authority.cell(.{ .planet = pe.planet_id, .local = 71 });
    try std.testing.expect(cell71.is_hall);
    try std.testing.expectEqual(@as(u64, 1), @backingInt(authority.summary().revision));
    try std.testing.expectEqual(@as(u16, 1216), authority.stats().live_pages);
}

test "pe/state/max-layout can clone every entity page within the fixed pool" {
    var builder = try pe.beginCreate(std.testing.allocator, .{ .seed = 2, .side = 32 });
    defer builder.deinit();
    try finishCreate(&builder, 256);
    var authority = try pe.takeAuthority(&builder);
    defer authority.deinit();
    var candidate = pe.state.Candidate.begin(&authority.storage);
    var budget: pe.WorkBudget = .{ .remaining = 256 };
    while (try candidate.advancePrepare(&budget) == .pending) budget.remaining = 256;
    for (0..768) |page| try finishWrite(&candidate, @intCast(page * 8), .{ .claim = .{ .settlement = @fromBackingInt(1) } });
    for (0..64) |slot| try finishWrite(&candidate, @intCast(slot), .{ .settlement = .{
        .id = @fromBackingInt(slot + 1),
        .owner = pe.civilization_id,
        .hall = .{ .planet = pe.planet_id, .local = 0 },
        .population = 0,
        .food = 0,
        .material = 0,
        .incoming_food = 0,
        .incoming_material = 0,
        .housing = 0,
        .hunger = 0,
        .fed_streak = 0,
    } });
    for (0..128) |page| try finishWrite(&candidate, @intCast(page * 2), .{ .building = .{
        .id = @fromBackingInt(page + 1),
        .settlement = @fromBackingInt(1),
        .kind = .house,
        .anchor = .{ .planet = pe.planet_id, .local = 0 },
        .facing = null,
        .footprint = .{ .len = 1, .cells = .{ .{ .planet = pe.planet_id, .local = 0 }, .{ .planet = pe.planet_id, .local = 0 } } },
        .remaining_ticks = 0,
    } });
    for (0..32) |page| try finishWrite(&candidate, @intCast(page * 2), .{ .shipment = .{
        .id = @fromBackingInt(page + 1),
        .source = @fromBackingInt(1),
        .destination = @fromBackingInt(2),
        .resource = .food,
        .quantity = 1,
        .departure_tick = @fromBackingInt(1),
        .arrival_tick = @fromBackingInt(2),
        .route_len = 2,
    } });
    for (0..64) |slot| for ([_]u16{ 0, 32, 64 }) |route_index| {
        try finishWrite(&candidate, @intCast(slot), .{ .route_cell = .{ .index = route_index, .cell = .{ .planet = pe.planet_id, .local = 70 } } });
    };
    try std.testing.expectEqual(@as(u16, 2400), authority.stats().live_pages);
    try std.testing.expectEqual(@as(u64, 1184), authority.stats().pages_copied);
    budget.remaining = 256;
    while (try candidate.advanceDiscard(&budget) == .pending) budget.remaining = 256;
    try std.testing.expectEqual(@as(u16, 1216), authority.stats().live_pages);
}

test "pe/state/staged-receipt discard respects zero budget" {
    var builder = try pe.beginCreate(std.testing.allocator, .{ .seed = 2, .side = 4 });
    defer builder.deinit();
    try finishCreate(&builder, 256);
    var authority = try pe.takeAuthority(&builder);
    defer authority.deinit();
    var candidate = pe.state.Candidate.begin(&authority.storage);
    var budget: pe.WorkBudget = .{ .remaining = 256 };
    while (try candidate.advancePrepare(&budget) == .pending) budget.remaining = 256;
    budget.remaining = 0;
    try std.testing.expectEqual(pe.Step.pending, try candidate.stage(authority.storage.genesis.?, &budget));
    try std.testing.expectEqual(@as(u16, 0), authority.storage.staged_count);
    budget.remaining = 1;
    try std.testing.expectEqual(pe.Step.done, try candidate.stage(authority.storage.genesis.?, &budget));
    try std.testing.expectEqual(@as(u16, 1), authority.storage.staged_count);
    budget.remaining = 1;
    while (candidate.discard_index < candidate.prepare_index) {
        _ = try candidate.advanceDiscard(&budget);
        budget.remaining = 1;
    }
    budget.remaining = 0;
    try std.testing.expectEqual(pe.Step.pending, try candidate.advanceDiscard(&budget));
    try std.testing.expectEqual(@as(u16, 1), authority.storage.staged_count);
    budget.remaining = 1;
    try std.testing.expectEqual(pe.Step.done, try candidate.advanceDiscard(&budget));
    try std.testing.expectEqual(@as(u16, 0), authority.storage.staged_count);
}

test "pe/state/root-meta COW keeps published revision and chain unchanged" {
    var builder = try pe.beginCreate(std.testing.allocator, .{ .seed = 2, .side = 4 });
    defer builder.deinit();
    try finishCreate(&builder, 256);
    var authority = try pe.takeAuthority(&builder);
    defer authority.deinit();
    const before = authority.summary();
    var candidate = pe.state.Candidate.begin(&authority.storage);
    var budget: pe.WorkBudget = .{ .remaining = 256 };
    while (try candidate.advancePrepare(&budget) == .pending) budget.remaining = 256;
    var updated = candidate.meta();
    updated.revision = @fromBackingInt(1);
    updated.tick = @fromBackingInt(1);
    var writer = pe.state.MetaWriter.init(updated);
    while (true) {
        budget.remaining = 1;
        if (try writer.advance(&candidate, &budget) == .done) break;
    }
    try std.testing.expectEqualDeep(before, authority.summary());
    try std.testing.expectEqual(@as(u64, 1), @backingInt(candidate.meta().revision));
    budget.remaining = 256;
    while (try candidate.advanceDiscard(&budget) == .pending) budget.remaining = 256;
    try std.testing.expectEqualDeep(before, authority.summary());
}

test "pe/state/route-cell rejects foreign or out-of-range IDs before cloning" {
    var builder = try pe.beginCreate(std.testing.allocator, .{ .seed = 2, .side = 4 });
    defer builder.deinit();
    try finishCreate(&builder, 256);
    var authority = try pe.takeAuthority(&builder);
    defer authority.deinit();
    var candidate = pe.state.Candidate.begin(&authority.storage);
    var budget: pe.WorkBudget = .{ .remaining = 256 };
    while (try candidate.advancePrepare(&budget) == .pending) budget.remaining = 256;
    const before = authority.stats().live_pages;
    var foreign = pe.state.RecordWriter.init(0, .{ .route_cell = .{ .index = 0, .cell = .{ .planet = @fromBackingInt(2), .local = 70 } } });
    try std.testing.expectError(error.InvalidPlanet, foreign.advance(&candidate, &budget));
    try std.testing.expectEqual(before, authority.stats().live_pages);
    var outside = pe.state.RecordWriter.init(0, .{ .route_cell = .{ .index = 0, .cell = .{ .planet = pe.planet_id, .local = 96 } } });
    try std.testing.expectError(error.InvalidCell, outside.advance(&candidate, &budget));
    try std.testing.expectEqual(before, authority.stats().live_pages);
    try finishWrite(&candidate, 0, .{ .route_cell = .{ .index = 64, .cell = .{ .planet = pe.planet_id, .local = 71 } } });
    try std.testing.expectEqualDeep(pe.CellId{ .planet = pe.planet_id, .local = 71 }, try candidate.routeCell(0, 64));
    budget.remaining = 256;
    while (try candidate.advanceDiscard(&budget) == .pending) budget.remaining = 256;
}

test "pe/state/reciprocity charges at most one topology edge per work unit" {
    var builder = try pe.beginCreate(std.testing.allocator, .{ .seed = 2, .side = 4 });
    defer builder.deinit();
    while (builder.cursor.phase != .reciprocity) {
        var budget: pe.WorkBudget = .{ .remaining = 256 };
        try std.testing.expectEqual(pe.CreateProgress.pending, pe.pollCreate(&builder, &budget));
    }
    while (builder.cursor.forward_cell != null or builder.cursor.reverse != 0) {
        var budget: pe.WorkBudget = .{ .remaining = 1 };
        try std.testing.expectEqual(pe.CreateProgress.pending, pe.pollCreate(&builder, &budget));
    }
    const before_edges = builder.cursor.edge_checks;
    const before_units = builder.stats().work_units;
    var one: pe.WorkBudget = .{ .remaining = 1 };
    try std.testing.expectEqual(pe.CreateProgress.pending, pe.pollCreate(&builder, &one));
    const after_remaining = one.remaining;
    const unit_delta = builder.stats().work_units - before_units;
    const edge_delta = builder.cursor.edge_checks - before_edges;
    const forward_staged = builder.cursor.forward_cell != null and builder.cursor.reverse == 0;
    try finishCreate(&builder, 256);
    var authority = try pe.takeAuthority(&builder);
    defer authority.deinit();
    try std.testing.expectEqual(@as(u16, 0), after_remaining);
    try std.testing.expectEqual(@as(u64, 1), unit_delta);
    try std.testing.expectEqual(@as(u64, 1), edge_delta);
    try std.testing.expect(forward_staged);
    try std.testing.expectEqual(@as(u64, 2_304), builder.cursor.edge_checks);
}

test "pe/state/settlement query returns every populated field" {
    var builder = try pe.beginCreate(std.testing.allocator, .{ .seed = 2, .side = 4 });
    defer builder.deinit();
    try finishCreate(&builder, 256);
    var authority = try pe.takeAuthority(&builder);
    defer authority.deinit();
    var candidate = pe.state.Candidate.begin(&authority.storage);
    var budget: pe.WorkBudget = .{ .remaining = 256 };
    while (try candidate.advancePrepare(&budget) == .pending) budget.remaining = 256;
    const hall: pe.CellId = .{ .planet = pe.planet_id, .local = 70 };
    const settlement: pe.SettlementId = @fromBackingInt(17);
    try finishWrite(&candidate, 0, .{ .settlement = .{
        .id = settlement,
        .owner = pe.civilization_id,
        .hall = hall,
        .population = 9,
        .food = 123,
        .material = 456,
        .incoming_food = 7,
        .incoming_material = 8,
        .housing = 12,
        .hunger = 2,
        .fed_streak = 3,
    } });
    var meta = candidate.meta();
    meta.revision = @fromBackingInt(4);
    var meta_writer = pe.state.MetaWriter.init(meta);
    budget.remaining = 256;
    while (try meta_writer.advance(&candidate, &budget) == .pending) budget.remaining = 256;
    candidate.publish();
    budget.remaining = 256;
    while (try candidate.advanceDiscard(&budget) == .pending) budget.remaining = 256;
    try std.testing.expectEqualDeep(pe.SettlementView{
        .revision = @fromBackingInt(4),
        .id = settlement,
        .owner = pe.civilization_id,
        .hall = hall,
        .population = 9,
        .food = 123,
        .material = 456,
        .incoming_food = 7,
        .incoming_material = 8,
        .housing = 12,
        .hunger = 2,
        .fed_streak = 3,
    }, try authority.settlement(settlement));
    try std.testing.expectError(error.NotFound, authority.settlement(@fromBackingInt(18)));
}
