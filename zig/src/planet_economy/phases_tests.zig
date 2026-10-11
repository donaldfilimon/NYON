const std = @import("std");
const pe = @import("root.zig");
const phases = @import("phases.zig");
const command = @import("command.zig");

fn authority() !pe.Authority {
    var builder = try pe.beginCreate(std.testing.allocator, .{ .seed = 2, .side = 4 });
    defer builder.deinit();
    while (true) {
        var budget: pe.WorkBudget = .{ .remaining = 256 };
        switch (pe.pollCreate(&builder, &budget)) {
            .pending => {},
            .ready => return pe.takeAuthority(&builder),
            .failed => |err| return err,
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

fn write(candidate: *pe.state.Candidate, index: u16, record: pe.state.Record) !void {
    var writer = pe.state.RecordWriter.init(index, record);
    while (true) {
        var budget: pe.WorkBudget = .{ .remaining = 256 };
        if (try writer.advance(candidate, &budget) == .done) return;
    }
}

fn metadata(candidate: *pe.state.Candidate, meta: pe.state.RootMeta) !void {
    var writer = pe.state.MetaWriter.init(meta);
    while (true) {
        var budget: pe.WorkBudget = .{ .remaining = 256 };
        if (try writer.advance(candidate, &budget) == .done) return;
    }
}

fn run(candidate: *pe.state.Candidate, phase: pe.Phase, tick: u64, units: u16) !void {
    var cursor = phases.Cursor.init(phase, @fromBackingInt(tick));
    while (true) {
        var budget: pe.WorkBudget = .{ .remaining = units };
        if (try cursor.advance(candidate, &budget) == .done) return;
    }
}

fn settlement(id: u64, population: u8, food: u32, material: u32) pe.state.Settlement {
    return .{
        .id = @fromBackingInt(id),
        .owner = pe.civilization_id,
        .hall = .{ .planet = pe.planet_id, .local = 70 + id },
        .population = population,
        .food = food,
        .material = material,
        .incoming_food = 0,
        .incoming_material = 0,
        .housing = 4,
        .hunger = 0,
        .fed_streak = 0,
    };
}

fn building(id: u64, owner: u64, kind: pe.BuildingKind, remaining: u8) pe.state.Building {
    const cell: pe.CellId = .{ .planet = pe.planet_id, .local = 72 + id };
    return .{
        .id = @fromBackingInt(id),
        .settlement = @fromBackingInt(owner),
        .kind = kind,
        .anchor = cell,
        .facing = null,
        .footprint = .{ .len = 1, .cells = .{ cell, cell } },
        .remaining_ticks = remaining,
    };
}

test "pe/phases/construction-and-work paid project completes at T+1 and produces then" {
    var owner = try authority();
    defer owner.deinit();
    var candidate = pe.state.Candidate.begin(&owner.storage);
    try prepare(&candidate);
    try write(&candidate, 0, .{ .settlement = settlement(1, 2, 0, 0) });
    try write(&candidate, 0, .{ .building = building(1, 1, .farm, 2) });
    var meta = candidate.meta();
    meta.settlements = 1;
    meta.buildings = 1;
    try metadata(&candidate, meta);
    try run(&candidate, .construction, 1, 1);
    try std.testing.expectEqual(@as(u8, 1), (try candidate.readRecord(.building, 0)).building.remaining_ticks);
    try run(&candidate, .production, 1, 7);
    try std.testing.expectEqual(@as(u32, 0), (try candidate.readRecord(.settlement, 0)).settlement.food);
    try run(&candidate, .construction, 2, 1);
    try run(&candidate, .production, 2, 7);
    try std.testing.expectEqual(@as(u8, 0), (try candidate.readRecord(.building, 0)).building.remaining_ticks);
    try std.testing.expectEqual(@as(u32, 6), (try candidate.readRecord(.settlement, 0)).settlement.food);
    try std.testing.expectEqual(@as(u16, 2), owner.storage.staged_count);
    try std.testing.expectEqual(pe.EventTag.completed, std.meta.activeTag(owner.storage.logicalReceipt(0).event));
    try std.testing.expectEqual(pe.EventTag.produced, std.meta.activeTag(owner.storage.logicalReceipt(1).event));
}

test "pe/phases/command-created authored and paid buildings first produce on their activation ticks" {
    var owner = try authority();
    defer owner.deinit();
    var candidate = pe.state.Candidate.begin(&owner.storage);
    try prepare(&candidate);
    try write(&candidate, 0, .{ .settlement = settlement(1, 4, 0, 4) });
    try write(&candidate, 71, .{ .claim = .{ .settlement = @fromBackingInt(1), .is_hall = true } });
    var meta = candidate.meta();
    meta.settlements = 1;
    meta.next_settlement = 2;
    try metadata(&candidate, meta);

    var authored_bytes: [1024]u8 = undefined;
    const authored = try command.encodeEnvelope(.{
        .sequence = @fromBackingInt(1),
        .provenance = .creator,
        .actor = null,
    }, .{ .author_build = .{
        .settlement = @fromBackingInt(1),
        .kind = .farm,
        .anchor = .{ .planet = pe.planet_id, .local = 75 },
        .facing = .west,
        .economic_bypass = true,
    } }, &authored_bytes);
    var creator = command.Cursor.init(try pe.SessionPolicy.init(.creator, null), &.{authored}, @fromBackingInt(0), @fromBackingInt(1));
    while (true) {
        var budget: pe.WorkBudget = .{ .remaining = 7 };
        if (try creator.advance(&candidate, &budget) == .done) break;
    }
    try std.testing.expectEqual(@as(u16, 1), creator.accepted);
    try std.testing.expectEqual(@as(u8, 0), (try candidate.readRecord(.building, 0)).building.remaining_ticks);
    try run(&candidate, .construction, 1, 1);
    try run(&candidate, .production, 1, 7);
    try std.testing.expectEqual(@as(u32, 6), (try candidate.readRecord(.settlement, 0)).settlement.food);
    try std.testing.expectEqual(@as(u16, 2), owner.storage.staged_count);
    try std.testing.expectEqual(@as(u64, 1), @backingInt(owner.storage.logicalReceipt(1).event.produced.building));

    var paid_bytes: [1024]u8 = undefined;
    const paid = try command.encodeEnvelope(.{
        .sequence = @fromBackingInt(2),
        .provenance = .ruler,
        .actor = pe.civilization_id,
    }, .{ .build = .{
        .settlement = @fromBackingInt(1),
        .kind = .quarry,
        .anchor = .{ .planet = pe.planet_id, .local = 70 },
        .facing = null,
    } }, &paid_bytes);
    var ruler = command.Cursor.init(try pe.SessionPolicy.init(.ruler, pe.civilization_id), &.{paid}, @fromBackingInt(1), @fromBackingInt(2));
    while (true) {
        var budget: pe.WorkBudget = .{ .remaining = 7 };
        if (try ruler.advance(&candidate, &budget) == .done) break;
    }
    try std.testing.expectEqual(@as(u16, 1), ruler.accepted);
    try std.testing.expectEqual(@as(u32, 0), (try candidate.readRecord(.settlement, 0)).settlement.material);
    try run(&candidate, .construction, 2, 1);
    try std.testing.expectEqual(@as(u8, 1), (try candidate.readRecord(.building, 1)).building.remaining_ticks);
    try run(&candidate, .production, 2, 7);
    try std.testing.expectEqual(@as(u32, 0), (try candidate.readRecord(.settlement, 0)).settlement.material);
    try run(&candidate, .construction, 3, 1);
    try run(&candidate, .production, 3, 7);
    try std.testing.expectEqual(@as(u8, 0), (try candidate.readRecord(.building, 1)).building.remaining_ticks);
    try std.testing.expectEqual(@as(u32, 2), (try candidate.readRecord(.settlement, 0)).settlement.material);
    try std.testing.expectEqual(@as(u64, 2), @backingInt(owner.storage.logicalReceipt(owner.storage.staged_count - 1).event.produced.building));
}

test "pe/phases/supply-counters partial shortage starves and full supply grows or housing-blocks" {
    var owner = try authority();
    defer owner.deinit();
    var candidate = pe.state.Candidate.begin(&owner.storage);
    try prepare(&candidate);
    var hungry = settlement(1, 2, 1, 0);
    hungry.hunger = 2;
    try write(&candidate, 0, .{ .settlement = hungry });
    var fed = settlement(2, 3, 3, 0);
    fed.fed_streak = 7;
    try write(&candidate, 1, .{ .settlement = fed });
    var full = settlement(3, 4, 4, 0);
    full.fed_streak = 7;
    try write(&candidate, 2, .{ .settlement = full });
    var zero = settlement(4, 0, 0, 0);
    zero.hunger = 2;
    zero.fed_streak = 7;
    try write(&candidate, 3, .{ .settlement = zero });
    var meta = candidate.meta();
    meta.settlements = 4;
    try metadata(&candidate, meta);
    try run(&candidate, .population, 1, 1);
    const starving_settlement = (try candidate.readRecord(.settlement, 0)).settlement;
    try std.testing.expectEqual(@as(u8, 1), starving_settlement.population);
    try std.testing.expectEqual(@as(u8, 0), starving_settlement.hunger);
    try std.testing.expectEqual(@as(u8, 4), (try candidate.readRecord(.settlement, 1)).settlement.population);
    try std.testing.expectEqual(@as(u8, 4), (try candidate.readRecord(.settlement, 2)).settlement.population);
    const empty = (try candidate.readRecord(.settlement, 3)).settlement;
    try std.testing.expectEqual(@as(u8, 0), empty.hunger);
    try std.testing.expectEqual(@as(u8, 0), empty.fed_streak);
    try std.testing.expectEqual(@as(u16, 7), owner.storage.staged_count);
}

test "pe/phases/production reserves workers only after full output fits" {
    var first_events: [5]pe.ReceiptEvent = undefined;
    for ([_]u16{ 1, 7, 256 }) |units| {
        var owner = try authority();
        defer owner.deinit();
        var candidate = pe.state.Candidate.begin(&owner.storage);
        try prepare(&candidate);
        var first = settlement(1, 2, 65_530, 0);
        first.incoming_food = 4;
        try write(&candidate, 0, .{ .settlement = first });
        var second = settlement(2, 4, 65_528, 0);
        second.incoming_food = 1;
        try write(&candidate, 1, .{ .settlement = second });
        try write(&candidate, 0, .{ .building = building(1, 1, .farm, 0) });
        try write(&candidate, 1, .{ .building = building(2, 1, .quarry, 0) });
        try write(&candidate, 2, .{ .building = building(3, 2, .farm, 0) });
        try write(&candidate, 3, .{ .building = building(4, 2, .quarry, 0) });
        try write(&candidate, 4, .{ .building = building(5, 2, .farm, 0) });
        var meta = candidate.meta();
        meta.settlements = 2;
        meta.buildings = 5;
        try metadata(&candidate, meta);
        try run(&candidate, .production, 1, units);
        const a = (try candidate.readRecord(.settlement, 0)).settlement;
        const b = (try candidate.readRecord(.settlement, 1)).settlement;
        try std.testing.expectEqual(@as(u32, 65_530), a.food);
        try std.testing.expectEqual(@as(u32, 2), a.material);
        try std.testing.expectEqual(@as(u32, 65_534), b.food);
        try std.testing.expectEqual(@as(u32, 2), b.material);
        try std.testing.expectEqual(@as(u16, 5), owner.storage.staged_count);
        try std.testing.expectEqual(pe.IdleReason.output_capacity, owner.storage.logicalReceipt(0).event.idle.reason);
        try std.testing.expectEqual(pe.EventTag.produced, std.meta.activeTag(owner.storage.logicalReceipt(1).event));
        try std.testing.expectEqual(pe.IdleReason.workforce, owner.storage.logicalReceipt(4).event.idle.reason);
        try std.testing.expectEqual(@as(u8, 0), owner.storage.logicalReceipt(4).event.idle.workers_available);
        try std.testing.expectEqual(@as(u32, 65_530), owner.storage.logicalReceipt(0).event.idle.stock_before);
        try std.testing.expectEqual(@as(u32, 4), owner.storage.logicalReceipt(0).event.idle.incoming_reserved);
        try std.testing.expectEqual(@as(u32, 2), owner.storage.logicalReceipt(1).event.produced.stock_after);
        try std.testing.expectEqual(@as(u32, 65_534), owner.storage.logicalReceipt(2).event.produced.stock_after);
        try std.testing.expectEqual(@as(u32, 2), owner.storage.logicalReceipt(3).event.produced.stock_after);
        try std.testing.expectEqual(@as(u32, 1), owner.storage.logicalReceipt(4).event.idle.incoming_reserved);
        for (0..5) |i| {
            const event = owner.storage.logicalReceipt(@intCast(i)).event;
            if (units == 1) first_events[i] = event else try std.testing.expectEqualDeep(first_events[i], event);
        }
    }
}

test "pe/phases/production uses growth only from the next tick" {
    var owner = try authority();
    defer owner.deinit();
    var candidate = pe.state.Candidate.begin(&owner.storage);
    try prepare(&candidate);
    var growing = settlement(1, 1, 2, 0);
    growing.fed_streak = 7;
    try write(&candidate, 0, .{ .settlement = growing });
    try write(&candidate, 0, .{ .building = building(1, 1, .farm, 0) });
    var meta = candidate.meta();
    meta.settlements = 1;
    meta.buildings = 1;
    try metadata(&candidate, meta);
    var zero: pe.WorkBudget = .{ .remaining = 0 };
    var cursor = phases.Cursor.init(.production, @fromBackingInt(1));
    const before = cursor;
    try std.testing.expectEqual(pe.Step.pending, try cursor.advance(&candidate, &zero));
    try std.testing.expectEqual(before.step, cursor.step);
    try std.testing.expectEqual(before.index, cursor.index);
    try std.testing.expectEqual(before.building_index, cursor.building_index);
    try std.testing.expectEqual(@as(u16, 0), owner.storage.staged_count);
    try run(&candidate, .production, 1, 1);
    try std.testing.expectEqual(pe.IdleReason.workforce, owner.storage.logicalReceipt(0).event.idle.reason);
    try run(&candidate, .population, 1, 7);
    try std.testing.expectEqual(@as(u8, 2), (try candidate.readRecord(.settlement, 0)).settlement.population);
    try std.testing.expectEqual(@as(u32, 1), (try candidate.readRecord(.settlement, 0)).settlement.food);
    try run(&candidate, .production, 2, 256);
    try std.testing.expectEqual(@as(u32, 7), (try candidate.readRecord(.settlement, 0)).settlement.food);
    try std.testing.expectEqual(pe.EventTag.produced, std.meta.activeTag(owner.storage.logicalReceipt(3).event));
}

test "pe/phases/current-tick grant supplies production workers" {
    var owner = try authority();
    defer owner.deinit();
    var candidate = pe.state.Candidate.begin(&owner.storage);
    try prepare(&candidate);
    try write(&candidate, 0, .{ .settlement = settlement(1, 0, 0, 0) });
    try write(&candidate, 0, .{ .building = building(1, 1, .farm, 0) });
    var meta = candidate.meta();
    meta.settlements = 1;
    meta.buildings = 1;
    meta.next_settlement = 2;
    meta.next_building = 2;
    try metadata(&candidate, meta);
    var bytes: [1024]u8 = undefined;
    const envelope = try command.encodeEnvelope(.{
        .sequence = @fromBackingInt(1),
        .provenance = .creator,
        .actor = null,
    }, .{ .grant = .{
        .settlement = @fromBackingInt(1),
        .food = 0,
        .material = 0,
        .population = 2,
        .economic_bypass = true,
    } }, &bytes);
    var cursor = command.Cursor.init(try pe.SessionPolicy.init(.creator, null), &.{envelope}, @fromBackingInt(0), @fromBackingInt(1));
    while (true) {
        var budget: pe.WorkBudget = .{ .remaining = 7 };
        if (try cursor.advance(&candidate, &budget) == .done) break;
    }
    try std.testing.expectEqual(@as(u16, 1), cursor.accepted);
    try run(&candidate, .production, 1, 1);
    try std.testing.expectEqual(@as(u32, 6), (try candidate.readRecord(.settlement, 0)).settlement.food);
    try std.testing.expectEqual(@as(u32, 6), owner.storage.logicalReceipt(1).event.produced.stock_after);
}

test "pe/phases/current-tick founding consumes food at both settlements" {
    var owner = try authority();
    defer owner.deinit();
    var candidate = pe.state.Candidate.begin(&owner.storage);
    try prepare(&candidate);
    try write(&candidate, 0, .{ .settlement = settlement(1, 2, 6, 20) });
    try write(&candidate, 71, .{ .claim = .{ .settlement = @fromBackingInt(1), .is_hall = true } });
    var meta = candidate.meta();
    meta.settlements = 1;
    meta.next_settlement = 2;
    try metadata(&candidate, meta);
    var bytes: [1024]u8 = undefined;
    const envelope = try command.encodeEnvelope(.{
        .sequence = @fromBackingInt(1),
        .provenance = .ruler,
        .actor = pe.civilization_id,
    }, .{ .found = .{
        .source = @fromBackingInt(1),
        .anchor = .{ .planet = pe.planet_id, .local = 5 },
    } }, &bytes);
    var cursor = command.Cursor.init(try pe.SessionPolicy.init(.ruler, pe.civilization_id), &.{envelope}, @fromBackingInt(0), @fromBackingInt(1));
    while (true) {
        var budget: pe.WorkBudget = .{ .remaining = 7 };
        if (try cursor.advance(&candidate, &budget) == .done) break;
    }
    try std.testing.expectEqual(@as(u16, 1), cursor.accepted);
    try run(&candidate, .population, 1, 1);
    const source = (try candidate.readRecord(.settlement, 0)).settlement;
    const founded = (try candidate.readRecord(.settlement, 1)).settlement;
    try std.testing.expectEqual(@as(u8, 0), source.population);
    try std.testing.expectEqual(@as(u32, 2), source.food);
    try std.testing.expectEqual(@as(u8, 2), founded.population);
    try std.testing.expectEqual(@as(u32, 2), founded.food);
    try std.testing.expectEqual(@as(u8, 0), owner.storage.logicalReceipt(1).event.supplied.needed);
    try std.testing.expectEqual(@as(u8, 2), owner.storage.logicalReceipt(2).event.supplied.consumed);
}

test "pe/phases/construction house activates housing once up to 32" {
    var owner = try authority();
    defer owner.deinit();
    var candidate = pe.state.Candidate.begin(&owner.storage);
    try prepare(&candidate);
    var home = settlement(1, 4, 0, 0);
    home.housing = 28;
    try write(&candidate, 0, .{ .settlement = home });
    try write(&candidate, 0, .{ .building = building(1, 1, .house, 1) });
    try write(&candidate, 1, .{ .building = building(2, 1, .house, 1) });
    var meta = candidate.meta();
    meta.settlements = 1;
    meta.buildings = 2;
    try metadata(&candidate, meta);
    try run(&candidate, .construction, 1, 1);
    try std.testing.expectEqual(@as(u8, 32), (try candidate.readRecord(.settlement, 0)).settlement.housing);
    try std.testing.expectEqual(@as(u16, 2), owner.storage.staged_count);
    try run(&candidate, .construction, 2, 7);
    try std.testing.expectEqual(@as(u8, 32), (try candidate.readRecord(.settlement, 0)).settlement.housing);
    try std.testing.expectEqual(@as(u16, 2), owner.storage.staged_count);
}

test "pe/phases/freight-reservation arrival credits stock and releases reservation before supply" {
    var owner = try authority();
    defer owner.deinit();
    var candidate = pe.state.Candidate.begin(&owner.storage);
    try prepare(&candidate);
    try write(&candidate, 0, .{ .settlement = settlement(1, 2, 0, 0) });
    var destination = settlement(2, 2, 0, 0);
    destination.incoming_food = 5;
    try write(&candidate, 1, .{ .settlement = destination });
    try write(&candidate, 0, .{ .shipment = .{
        .id = @fromBackingInt(1),
        .source = @fromBackingInt(1),
        .destination = @fromBackingInt(2),
        .resource = .food,
        .quantity = 5,
        .departure_tick = @fromBackingInt(1),
        .arrival_tick = @fromBackingInt(4),
        .route_len = 4,
    } });
    var meta = candidate.meta();
    meta.settlements = 2;
    meta.shipments = 1;
    meta.next_shipment = 2;
    try metadata(&candidate, meta);
    try run(&candidate, .arrivals, 3, 1);
    try std.testing.expectEqual(@as(u32, 0), (try candidate.readRecord(.settlement, 1)).settlement.food);
    try run(&candidate, .arrivals, 4, 7);
    try run(&candidate, .population, 4, 256);
    const arrived = (try candidate.readRecord(.settlement, 1)).settlement;
    try std.testing.expectEqual(@as(u32, 3), arrived.food);
    try std.testing.expectEqual(@as(u32, 0), arrived.incoming_food);
    try std.testing.expectEqual(@as(u16, 0), candidate.meta().shipments);
    try std.testing.expectEqual(@as(u64, 0), @backingInt((try candidate.readRecord(.shipment, 0)).shipment.id));
}

test "pe/phases/admitted transfer conserves source transit destination and consumed food" {
    var owner = try authority();
    defer owner.deinit();
    var candidate = pe.state.Candidate.begin(&owner.storage);
    try prepare(&candidate);
    try write(&candidate, 0, .{ .settlement = settlement(1, 0, 10, 0) });
    var destination = settlement(2, 2, 0, 0);
    destination.hall.local = 75;
    try write(&candidate, 1, .{ .settlement = destination });
    try write(&candidate, 71, .{ .claim = .{ .settlement = @fromBackingInt(1), .is_hall = true } });
    try write(&candidate, 75, .{ .claim = .{ .settlement = @fromBackingInt(2), .is_hall = true } });
    var meta = candidate.meta();
    meta.settlements = 2;
    meta.next_settlement = 3;
    try metadata(&candidate, meta);
    const path = [_]pe.CellId{
        .{ .planet = pe.planet_id, .local = 71 },
        .{ .planet = pe.planet_id, .local = 75 },
    };
    var bytes: [1024]u8 = undefined;
    const envelope = try command.encodeEnvelope(.{
        .sequence = @fromBackingInt(1),
        .provenance = .ruler,
        .actor = pe.civilization_id,
    }, .{ .ship = .{
        .source = @fromBackingInt(1),
        .destination = @fromBackingInt(2),
        .resource = .food,
        .amount = 3,
        .path = &path,
    } }, &bytes);
    var cursor = command.Cursor.init(try pe.SessionPolicy.init(.ruler, pe.civilization_id), &.{envelope}, @fromBackingInt(0), @fromBackingInt(1));
    while (true) {
        var budget: pe.WorkBudget = .{ .remaining = 7 };
        if (try cursor.advance(&candidate, &budget) == .done) break;
    }
    try std.testing.expectEqual(@as(u16, 1), cursor.accepted);
    try std.testing.expectEqual(@as(u32, 7), (try candidate.readRecord(.settlement, 0)).settlement.food);
    try std.testing.expectEqual(@as(u32, 3), (try candidate.readRecord(.settlement, 1)).settlement.incoming_food);
    try std.testing.expectEqual(@as(u16, 3), (try candidate.readRecord(.shipment, 0)).shipment.quantity);
    try std.testing.expectEqual(@as(u32, 10), (try candidate.readRecord(.settlement, 0)).settlement.food + (try candidate.readRecord(.shipment, 0)).shipment.quantity + (try candidate.readRecord(.settlement, 1)).settlement.food);

    try run(&candidate, .arrivals, 1, 1);
    try run(&candidate, .population, 1, 7);
    try std.testing.expectEqual(@as(u32, 10), (try candidate.readRecord(.settlement, 0)).settlement.food + (try candidate.readRecord(.shipment, 0)).shipment.quantity + (try candidate.readRecord(.settlement, 1)).settlement.food);
    try std.testing.expectEqual(@as(u8, 2), owner.storage.logicalReceipt(owner.storage.staged_count - 1).event.supplied.shortage);

    try run(&candidate, .arrivals, 2, 1);
    try std.testing.expectEqual(@as(u32, 3), (try candidate.readRecord(.settlement, 1)).settlement.food);
    try std.testing.expectEqual(@as(u32, 0), (try candidate.readRecord(.settlement, 1)).settlement.incoming_food);
    try std.testing.expectEqual(@as(u64, 0), @backingInt((try candidate.readRecord(.shipment, 0)).shipment.id));
    try std.testing.expectEqual(@as(u32, 10), (try candidate.readRecord(.settlement, 0)).settlement.food + (try candidate.readRecord(.settlement, 1)).settlement.food);
    try run(&candidate, .population, 2, 7);
    try std.testing.expectEqual(@as(u32, 1), (try candidate.readRecord(.settlement, 1)).settlement.food);
    try std.testing.expectEqual(@as(u32, 10), (try candidate.readRecord(.settlement, 0)).settlement.food + (try candidate.readRecord(.settlement, 1)).settlement.food + @as(u32, 2));
}

test "pe/phases/arrivals stage shipments in ID order across reused slots" {
    var owner = try authority();
    defer owner.deinit();
    var candidate = pe.state.Candidate.begin(&owner.storage);
    try prepare(&candidate);
    try write(&candidate, 0, .{ .settlement = settlement(1, 0, 0, 0) });
    var destination = settlement(2, 0, 0, 0);
    destination.incoming_material = 5;
    try write(&candidate, 1, .{ .settlement = destination });
    for ([_]struct { slot: u16, id: u64, quantity: u16 }{
        .{ .slot = 0, .id = 9, .quantity = 2 },
        .{ .slot = 1, .id = 3, .quantity = 3 },
    }) |entry| try write(&candidate, entry.slot, .{ .shipment = .{
        .id = @fromBackingInt(entry.id),
        .source = @fromBackingInt(1),
        .destination = @fromBackingInt(2),
        .resource = .material,
        .quantity = entry.quantity,
        .departure_tick = @fromBackingInt(1),
        .arrival_tick = @fromBackingInt(2),
        .route_len = 2,
    } });
    var meta = candidate.meta();
    meta.settlements = 2;
    meta.shipments = 2;
    meta.next_shipment = 10;
    try metadata(&candidate, meta);
    try run(&candidate, .arrivals, 2, 1);
    try std.testing.expectEqual(@as(u64, 3), @backingInt(owner.storage.logicalReceipt(0).event.arrived.shipment));
    try std.testing.expectEqual(@as(u32, 3), owner.storage.logicalReceipt(0).event.arrived.destination_stock_after);
    try std.testing.expectEqual(@as(u64, 9), @backingInt(owner.storage.logicalReceipt(1).event.arrived.shipment));
    try std.testing.expectEqual(@as(u32, 5), owner.storage.logicalReceipt(1).event.arrived.destination_stock_after);
    try std.testing.expectEqual(@as(u32, 0), (try candidate.readRecord(.settlement, 1)).settlement.incoming_material);
}

test "pe/phases/four-phase ordered effects and final records match at budgets 1 7 256" {
    var first_receipts: [9]pe.Receipt = undefined;
    var first_settlements: [2]pe.state.Settlement = undefined;
    var first_buildings: [2]pe.state.Building = undefined;
    var first_shipments: [2]pe.state.Shipment = undefined;
    var first_meta: pe.state.RootMeta = undefined;
    for ([_]u16{ 1, 7, 256 }) |units| {
        var owner = try authority();
        defer owner.deinit();
        var candidate = pe.state.Candidate.begin(&owner.storage);
        try prepare(&candidate);
        var first = settlement(1, 4, 1, 0);
        first.incoming_food = 3;
        first.fed_streak = 7;
        try write(&candidate, 0, .{ .settlement = first });
        var second = settlement(2, 2, 0, 0);
        second.hunger = 2;
        try write(&candidate, 1, .{ .settlement = second });
        try write(&candidate, 0, .{ .building = building(1, 1, .quarry, 1) });
        try write(&candidate, 1, .{ .building = building(2, 1, .farm, 0) });
        for ([_]struct { slot: u16, id: u64, quantity: u16 }{
            .{ .slot = 0, .id = 9, .quantity = 2 },
            .{ .slot = 1, .id = 3, .quantity = 1 },
        }) |entry| try write(&candidate, entry.slot, .{ .shipment = .{
            .id = @fromBackingInt(entry.id),
            .source = @fromBackingInt(2),
            .destination = @fromBackingInt(1),
            .resource = .food,
            .quantity = entry.quantity,
            .departure_tick = @fromBackingInt(1),
            .arrival_tick = @fromBackingInt(2),
            .route_len = 2,
        } });
        var meta = candidate.meta();
        meta.settlements = 2;
        meta.buildings = 2;
        meta.shipments = 2;
        meta.next_shipment = 10;
        try metadata(&candidate, meta);
        try run(&candidate, .arrivals, 2, units);
        try run(&candidate, .construction, 2, units);
        try run(&candidate, .production, 2, units);
        try run(&candidate, .population, 2, units);

        try std.testing.expectEqual(@as(u16, 9), owner.storage.staged_count);
        try std.testing.expectEqual(@as(u64, 3), @backingInt(owner.storage.logicalReceipt(0).event.arrived.shipment));
        try std.testing.expectEqual(@as(u32, 2), owner.storage.logicalReceipt(0).event.arrived.destination_stock_after);
        try std.testing.expectEqual(@as(u64, 9), @backingInt(owner.storage.logicalReceipt(1).event.arrived.shipment));
        try std.testing.expectEqual(@as(u32, 4), owner.storage.logicalReceipt(1).event.arrived.destination_stock_after);
        try std.testing.expectEqual(pe.EventTag.completed, std.meta.activeTag(owner.storage.logicalReceipt(2).event));
        try std.testing.expectEqual(@as(u32, 2), owner.storage.logicalReceipt(3).event.produced.stock_after);
        try std.testing.expectEqual(@as(u32, 10), owner.storage.logicalReceipt(4).event.produced.stock_after);
        try std.testing.expectEqual(@as(u8, 4), owner.storage.logicalReceipt(5).event.supplied.consumed);
        try std.testing.expectEqual(pe.DemographyKind.housing_blocked, owner.storage.logicalReceipt(6).event.demography.kind);
        try std.testing.expectEqual(@as(u8, 2), owner.storage.logicalReceipt(7).event.supplied.shortage);
        try std.testing.expectEqual(pe.DemographyKind.starvation, owner.storage.logicalReceipt(8).event.demography.kind);
        const settled_a = (try candidate.readRecord(.settlement, 0)).settlement;
        const settled_b = (try candidate.readRecord(.settlement, 1)).settlement;
        try std.testing.expectEqual(@as(u32, 6), settled_a.food);
        try std.testing.expectEqual(@as(u32, 2), settled_a.material);
        try std.testing.expectEqual(@as(u32, 0), settled_a.incoming_food);
        try std.testing.expectEqual(@as(u8, 1), settled_b.population);
        try std.testing.expectEqual(@as(u8, 0), settled_b.hunger);
        try std.testing.expectEqual(@as(u16, 0), candidate.meta().shipments);
        for (0..9) |i| {
            const receipt = owner.storage.logicalReceipt(@intCast(i)).*;
            try std.testing.expectEqual(@as(u16, @intCast(i)), receipt.header.ordinal);
            if (units == 1) first_receipts[i] = receipt else try std.testing.expectEqualDeep(first_receipts[i], receipt);
        }
        for (0..2) |i| {
            const slot: u16 = @intCast(i);
            const current_settlement = (try candidate.readRecord(.settlement, slot)).settlement;
            const current_building = (try candidate.readRecord(.building, slot)).building;
            const current_shipment = (try candidate.readRecord(.shipment, slot)).shipment;
            if (units == 1) {
                first_settlements[i] = current_settlement;
                first_buildings[i] = current_building;
                first_shipments[i] = current_shipment;
            } else {
                try std.testing.expectEqualDeep(first_settlements[i], current_settlement);
                try std.testing.expectEqualDeep(first_buildings[i], current_building);
                try std.testing.expectEqualDeep(first_shipments[i], current_shipment);
            }
        }
        if (units == 1) first_meta = candidate.meta() else try std.testing.expectEqualDeep(first_meta, candidate.meta());
    }
}

test "pe/phases/shipment-slot-reuse retired slot accepts fresh ID and route" {
    var owner = try authority();
    defer owner.deinit();
    var candidate = pe.state.Candidate.begin(&owner.storage);
    try prepare(&candidate);
    try write(&candidate, 0, .{ .settlement = settlement(1, 2, 0, 0) });
    var destination = settlement(2, 2, 0, 0);
    destination.incoming_material = 3;
    destination.hall.local = 75;
    try write(&candidate, 1, .{ .settlement = destination });
    try write(&candidate, 71, .{ .claim = .{ .settlement = @fromBackingInt(1), .is_hall = true } });
    try write(&candidate, 75, .{ .claim = .{ .settlement = @fromBackingInt(2), .is_hall = true } });
    try write(&candidate, 0, .{ .shipment = .{
        .id = @fromBackingInt(7),
        .source = @fromBackingInt(1),
        .destination = @fromBackingInt(2),
        .resource = .material,
        .quantity = 3,
        .departure_tick = @fromBackingInt(1),
        .arrival_tick = @fromBackingInt(2),
        .route_len = 2,
    } });
    var meta = candidate.meta();
    meta.settlements = 2;
    meta.shipments = 1;
    meta.next_shipment = 8;
    try metadata(&candidate, meta);
    try run(&candidate, .arrivals, 2, 1);
    const old_shipment_id: pe.ShipmentId = @fromBackingInt(7);
    try std.testing.expectEqual(@as(u64, 0), @backingInt((try candidate.readRecord(.shipment, 0)).shipment.id));
    const path = [_]pe.CellId{
        .{ .planet = pe.planet_id, .local = 71 },
        .{ .planet = pe.planet_id, .local = 75 },
    };
    var bytes: [1024]u8 = undefined;
    const envelope = try command.encodeEnvelope(.{
        .sequence = @fromBackingInt(1),
        .provenance = .creator,
        .actor = null,
    }, .{ .author_ship = .{
        .source = @fromBackingInt(1),
        .destination = @fromBackingInt(2),
        .resource = .material,
        .amount = 2,
        .path = &path,
        .cargo_source = .mint,
        .economic_bypass = true,
    } }, &bytes);
    var cursor = command.Cursor.init(try pe.SessionPolicy.init(.creator, null), &.{envelope}, @fromBackingInt(0), @fromBackingInt(3));
    while (true) {
        var budget: pe.WorkBudget = .{ .remaining = 7 };
        if (try cursor.advance(&candidate, &budget) == .done) break;
    }
    try std.testing.expectEqual(@as(u16, 1), cursor.accepted);
    const new_shipment = (try candidate.readRecord(.shipment, 0)).shipment;
    const new_shipment_id = new_shipment.id;
    try std.testing.expect(old_shipment_id != new_shipment_id);
    try std.testing.expectEqual(@as(u64, 8), @backingInt(new_shipment_id));
    try std.testing.expectEqual(@as(u16, 2), new_shipment.quantity);
    try std.testing.expectEqual(@as(u64, 4), @backingInt(new_shipment.arrival_tick));
    try std.testing.expectEqualDeep(path[0], try candidate.routeCell(0, 0));
    try std.testing.expectEqualDeep(path[1], try candidate.routeCell(0, 1));
    try std.testing.expectEqual(@as(u32, 2), (try candidate.readRecord(.settlement, 1)).settlement.incoming_material);
    try std.testing.expectEqual(@as(u32, 3), (try candidate.readRecord(.settlement, 1)).settlement.material);
    try run(&candidate, .arrivals, 4, 1);
    try std.testing.expectEqual(@as(u64, 8), @backingInt(owner.storage.logicalReceipt(owner.storage.staged_count - 1).event.arrived.shipment));
    try std.testing.expectEqual(@as(u32, 5), (try candidate.readRecord(.settlement, 1)).settlement.material);
    try std.testing.expectEqual(@as(u32, 0), (try candidate.readRecord(.settlement, 1)).settlement.incoming_material);
    try std.testing.expectEqual(@as(u64, 0), @backingInt((try candidate.readRecord(.shipment, 0)).shipment.id));
    try std.testing.expectEqual(@as(u16, 0), candidate.meta().shipments);
}
