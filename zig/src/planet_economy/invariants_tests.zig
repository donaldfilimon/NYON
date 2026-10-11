const std = @import("std");
const pe = @import("root.zig");
const helpers = @import("jobs_tests.zig");

test "pe/invariants/food70000 before finalize discards and retries unchanged input" {
    var authority = try helpers.create(.{ .seed = 2, .side = 4 });
    defer authority.deinit();
    const before = authority.storage.rootMeta(authority.storage.active);
    const active = authority.storage.active;
    var bytes: [1024]u8 = undefined;
    const envelope = try helpers.author(&bytes, 1);
    const policy = try pe.SessionPolicy.init(.creator, null);
    const id = try authority.beginTick(authority.summary().revision, policy, &.{envelope});
    while (authority.lane.?.step != .finalize) {
        var budget: pe.WorkBudget = .{ .remaining = 1 };
        try std.testing.expect(authority.poll(id, &budget) == .pending);
    }
    var record = (try authority.lane.?.candidate.readRecord(.settlement, 0)).settlement;
    record.food = 70_000;
    var writer = pe.state.RecordWriter.init(0, .{ .settlement = record });
    while (true) {
        var budget: pe.WorkBudget = .{ .remaining = 1 };
        if (try writer.advance(&authority.lane.?.candidate, &budget) == .done) break;
    }
    var published = false;
    var failures: u8 = 0;
    while (true) {
        var budget: pe.WorkBudget = .{ .remaining = 1 };
        switch (authority.poll(id, &budget)) {
            .pending => {},
            .published => published = true,
            .failed => |failure| {
                try std.testing.expectEqual(error.InvariantViolation, failure);
                failures += 1;
            },
            .idle => break,
            else => return error.UnexpectedProgress,
        }
    }
    try std.testing.expect(!published);
    try std.testing.expectEqual(@as(u8, 1), failures);
    try std.testing.expectEqual(active, authority.storage.active);
    try std.testing.expectEqualDeep(before, authority.storage.rootMeta(authority.storage.active));
    try std.testing.expectError(error.NotFound, authority.settlement(@fromBackingInt(1)));
    try std.testing.expectEqual(@as(u16, 1216), authority.stats().live_pages);
    _ = try helpers.tick(&authority, policy, &.{envelope}, 1);
    try std.testing.expectEqual(@as(u32, 8), (try authority.settlement(@fromBackingInt(1))).food);
}

const Fault = enum { food, reservation, material, population, housing, hunger, streak, mixed_counters, owner, hall, orphan_claim, claim_building, building_ticks, footprint, building_owner, shipment_quantity, shipment_destination, shipment_time, route_length, route_repeat, route_edge, route_endpoint, erased_claim, erased_building, erased_settlement, counts };

fn fixture() !pe.Authority {
    var authority = try helpers.create(.{ .seed = 2, .side = 4 });
    const policy = try pe.SessionPolicy.init(.combined, pe.civilization_id);
    var bytes: [4][1024]u8 = undefined;
    const path = [_]pe.CellId{ cell(70), cell(71), cell(4), cell(5) };
    const batch = [_]pe.Envelope{
        try helpers.author(&bytes[0], 1),
        try pe.encodeEnvelope(.{ .sequence = @fromBackingInt(2), .provenance = .creator, .actor = null }, .{ .author_settlement = .{ .owner = pe.civilization_id, .anchor = cell(5), .population = 2, .food = 6, .material = 0, .economic_bypass = true } }, &bytes[1]),
        try pe.encodeEnvelope(.{ .sequence = @fromBackingInt(3), .provenance = .creator, .actor = null }, .{ .author_build = .{ .settlement = @fromBackingInt(1), .kind = .farm, .anchor = cell(71), .facing = .east, .economic_bypass = true } }, &bytes[2]),
        try pe.encodeEnvelope(.{ .sequence = @fromBackingInt(4), .provenance = .ruler, .actor = pe.civilization_id }, .{ .ship = .{ .source = @fromBackingInt(1), .destination = @fromBackingInt(2), .resource = .food, .amount = 4, .path = &path } }, &bytes[3]),
    };
    _ = try helpers.tick(&authority, policy, &batch, 256);
    return authority;
}
fn cell(local: u64) pe.CellId {
    return .{ .planet = pe.planet_id, .local = local };
}
fn atFinalize(authority: *pe.Authority) !pe.JobId {
    const id = try authority.beginTick(authority.summary().revision, try pe.SessionPolicy.init(.ruler, pe.civilization_id), &.{});
    while (authority.lane.?.step != .finalize) {
        var one: pe.WorkBudget = .{ .remaining = 1 };
        try std.testing.expect(authority.poll(id, &one) == .pending);
    }
    return id;
}
fn write(candidate: *pe.state.Candidate, index: u16, value: pe.state.Record) !void {
    var writer = pe.state.RecordWriter.init(index, value);
    while (true) {
        var one: pe.WorkBudget = .{ .remaining = 1 };
        if (try writer.advance(candidate, &one) == .done) return;
    }
}
fn inject(candidate: *pe.state.Candidate, fault: Fault) !void {
    switch (fault) {
        .food, .reservation, .material, .population, .housing, .hunger, .streak, .mixed_counters, .owner, .hall => {
            var value = (try candidate.readRecord(.settlement, 1)).settlement;
            switch (fault) {
                .food => value.food = 65_535,
                .reservation => value.incoming_food = 3,
                .material => value.incoming_material = 65_536,
                .population => value.population = 33,
                .housing => value.housing = 8,
                .hunger => value.hunger = 3,
                .streak => value.fed_streak = 8,
                .mixed_counters => {
                    value.hunger = 1;
                    value.fed_streak = 1;
                },
                .owner => value.owner = @fromBackingInt(8),
                .hall => value.hall = cell(6),
                else => unreachable,
            }
            try write(candidate, 1, .{ .settlement = value });
        },
        .orphan_claim, .claim_building => {
            var value = (try candidate.readRecord(.claim, 71)).claim;
            if (fault == .orphan_claim) value.settlement = null else value.building = @fromBackingInt(99);
            try write(candidate, 71, .{ .claim = value });
        },
        .building_ticks, .footprint, .building_owner => {
            var value = (try candidate.readRecord(.building, 0)).building;
            switch (fault) {
                .building_ticks => value.remaining_ticks = 2,
                .footprint => value.footprint.cells[1] = cell(6),
                .building_owner => value.settlement = @fromBackingInt(99),
                else => unreachable,
            }
            try write(candidate, 0, .{ .building = value });
        },
        .shipment_quantity, .shipment_destination, .shipment_time, .route_length => {
            var value = (try candidate.readRecord(.shipment, 0)).shipment;
            switch (fault) {
                .shipment_quantity => value.quantity = 257,
                .shipment_destination => value.destination = @fromBackingInt(99),
                .shipment_time => value.arrival_tick = @fromBackingInt(2),
                .route_length => value.route_len = 66,
                else => unreachable,
            }
            try write(candidate, 0, .{ .shipment = value });
        },
        .route_repeat => try write(candidate, 0, .{ .route_cell = .{ .index = 1, .cell = cell(70) } }),
        .route_edge => try write(candidate, 0, .{ .route_cell = .{ .index = 1, .cell = cell(90) } }),
        .route_endpoint => try write(candidate, 0, .{ .route_cell = .{ .index = 3, .cell = cell(6) } }),
        .erased_claim => try write(candidate, 71, .{ .claim = .{} }),
        .erased_building => try write(candidate, 0, .{ .building = std.mem.zeroes(pe.state.Building) }),
        .erased_settlement => try write(candidate, 1, .{ .settlement = std.mem.zeroes(pe.state.Settlement) }),
        .counts => {
            var value = candidate.meta();
            value.buildings = 0;
            var writer = pe.state.MetaWriter.init(value);
            while (true) {
                var one: pe.WorkBudget = .{ .remaining = 1 };
                if (try writer.advance(candidate, &one) == .done) break;
            }
        },
    }
}

test "pe/invariants/dirty record kinds and references reject semantic faults before publication" {
    var authority = try fixture();
    defer authority.deinit();
    const before = authority.storage.rootMeta(authority.storage.active);
    const a = try authority.settlement(@fromBackingInt(1));
    const b = try authority.settlement(@fromBackingInt(2));
    const pages = authority.stats().live_pages;
    inline for (std.meta.tags(Fault)) |fault| {
        const id = try atFinalize(&authority);
        try inject(&authority.lane.?.candidate, fault);
        var failures: u8 = 0;
        var published = false;
        while (true) {
            const work = authority.stats().work_units;
            const current = &authority.lane.?.invariants;
            const cursor = .{ current.phase, current.page, current.index, current.scan, current.part, current.other, current.direction };
            var zero: pe.WorkBudget = .{ .remaining = 0 };
            try std.testing.expect(authority.poll(id, &zero) == .pending);
            try std.testing.expectEqual(work, authority.stats().work_units);
            const unchanged = &authority.lane.?.invariants;
            try std.testing.expectEqualDeep(cursor, .{ unchanged.phase, unchanged.page, unchanged.index, unchanged.scan, unchanged.part, unchanged.other, unchanged.direction });
            var one: pe.WorkBudget = .{ .remaining = 1 };
            const result = authority.poll(id, &one);
            try std.testing.expect(authority.stats().work_units - work <= 1);
            switch (result) {
                .pending => {},
                .failed => |failure| {
                    try std.testing.expectEqual(error.InvariantViolation, failure);
                    failures += 1;
                },
                .published => published = true,
                .idle => break,
                else => return error.UnexpectedProgress,
            }
        }
        if (published or failures != 1) std.debug.print("missed invariant: {s}\n", .{@tagName(fault)});
        try std.testing.expect(!published);
        try std.testing.expectEqual(@as(u8, 1), failures);
        try std.testing.expectEqualDeep(before, authority.storage.rootMeta(authority.storage.active));
        try std.testing.expectEqualDeep(a, try authority.settlement(@fromBackingInt(1)));
        try std.testing.expectEqualDeep(b, try authority.settlement(@fromBackingInt(2)));
        try std.testing.expectEqual(pages, authority.stats().live_pages);
    }
    var reference = try fixture();
    defer reference.deinit();
    const policy = try pe.SessionPolicy.init(.ruler, pe.civilization_id);
    try std.testing.expectEqualDeep(try helpers.tick(&reference, policy, &.{}, 256), try helpers.tick(&authority, policy, &.{}, 1));
    try std.testing.expectEqualDeep(try reference.settlement(@fromBackingInt(1)), try authority.settlement(@fromBackingInt(1)));
}

test "pe/invariants/exact inventory reservation cap passes and lookup cancellation retries" {
    var authority = try fixture();
    defer authority.deinit();
    const before = authority.storage.rootMeta(authority.storage.active);
    var id = try atFinalize(&authority);
    var record = (try authority.lane.?.candidate.readRecord(.settlement, 1)).settlement;
    record.food = 65_531; // Four incoming food fits the inclusive 65,535 cap.
    try write(&authority.lane.?.candidate, 1, .{ .settlement = record });
    var cursor: @import("invariants.zig").Cursor = .{};
    while (true) {
        var one: pe.WorkBudget = .{ .remaining = 1 };
        if (try cursor.advance(&authority.lane.?.candidate, &one) == .done) break;
    }
    _ = authority.cancel(id);
    while (true) {
        var one: pe.WorkBudget = .{ .remaining = 1 };
        if (authority.poll(id, &one) == .idle) break;
    }
    try std.testing.expectEqualDeep(before, authority.storage.rootMeta(authority.storage.active));
    id = try atFinalize(&authority);
    while (authority.lane.?.step != .invariants or authority.lane.?.invariants.phase != .houses) {
        var one: pe.WorkBudget = .{ .remaining = 1 };
        try std.testing.expect(authority.poll(id, &one) == .pending);
    }
    _ = authority.cancel(id);
    while (true) {
        var one: pe.WorkBudget = .{ .remaining = 1 };
        if (authority.poll(id, &one) == .idle) break;
    }
    try std.testing.expectEqualDeep(before, authority.storage.rootMeta(authority.storage.active));
    _ = try helpers.tick(&authority, try pe.SessionPolicy.init(.ruler, pe.civilization_id), &.{}, 1);
}
