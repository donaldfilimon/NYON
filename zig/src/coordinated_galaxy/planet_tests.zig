const std = @import("std");
const planet = @import("planet.zig");
const t = @import("types.zig");
const command = @import("../planet_economy/command.zig");
const it = @import("../planet_economy/types.zig");
const vectors = @import("wire_vectors.zig");
const creator: t.SessionPolicy = .{ .role = .creator, .civilization = null };
const tick_phases = [_]t.Phase{ .prepare, .commands, .arrivals, .construction, .production, .population, .local_finalize };

pub fn create(allocator: t.Allocator, id: u32, count: u8, units: u16) !planet.PlanetAuthority {
    var builder = try planet.createPlanet(allocator, .{ .side = 4 }, @fromBackingInt(id), 2, count);
    defer builder.deinit();
    while (true) {
        var budget: t.WorkBudget = .{ .remaining = units };
        if (try planet.advancePlanetCreate(&builder, &budget) == .done) break;
    }
    return planet.takePlanet(&builder);
}
pub fn advance(p: *planet.PlanetAuthority, phase: t.Phase, units: u16) !void {
    while (true) {
        var budget: t.WorkBudget = .{ .remaining = units };
        if (try p.advancePhase(phase, &budget) == .done) return;
    }
}
pub fn freeze(p: *planet.PlanetAuthority, units: u16) !t.PreparedRoot {
    while (true) {
        var budget: t.WorkBudget = .{ .remaining = units };
        if (try p.freezePrepared(&budget)) |root| return root;
    }
}
pub fn retire(p: *planet.PlanetAuthority, root: t.RootHandle, units: u16) !void {
    while (true) {
        var budget: t.WorkBudget = .{ .remaining = units };
        if (try p.retire(root, &budget) == .done) return;
    }
}
pub fn demo(p: *planet.PlanetAuthority, id: u32, owner: u32) !void {
    const header: it.CommandHeader = .{ .sequence = @fromBackingInt(1), .provenance = .creator, .actor = null };
    const settlement = try command.encodeEnvelope(header, .{ .author_settlement = .{
        .owner = @fromBackingInt(owner),
        .anchor = .{ .planet = @fromBackingInt(id), .local = 70 },
        .population = 2,
        .food = 8,
        .material = 20,
        .economic_bypass = true,
    } }, try p.envelopeBuffer(0));
    const farm = try command.encodeEnvelope(.{ .sequence = @fromBackingInt(2), .provenance = .creator, .actor = null }, .{ .author_build = .{
        .settlement = @fromBackingInt(1),
        .kind = .farm,
        .anchor = .{ .planet = @fromBackingInt(id), .local = 71 },
        .facing = .east,
        .economic_bypass = true,
    } }, try p.envelopeBuffer(1));
    try p.beginPrepared(@fromBackingInt(0), @fromBackingInt(1), creator, &.{ @intCast(settlement.bytes.len), @intCast(farm.bytes.len) }, @fromBackingInt(9));
}
fn install(p: *planet.PlanetAuthority) !void {
    var one: t.WorkBudget = .{ .remaining = 1 };
    try std.testing.expectEqual(.done, try p.installHidden(&one));
    try std.testing.expectEqual(@as(u16, 0), one.remaining);
}
fn restore(p: *planet.PlanetAuthority) !void {
    var one: t.WorkBudget = .{ .remaining = 1 };
    try std.testing.expectEqual(.done, try p.restoreHidden(&one));
}

test "cg/planet/staged-no-publication" {
    var p = try create(std.testing.allocator, 2, 2, 1);
    defer p.deinit();
    const old = p.activeRoot();
    const old_meta = try p.metadata(old);
    var genesis: [1]t.LocalReceiptValue = undefined;
    _ = try p.receipts(old, 0, &genesis);
    try std.testing.expectEqualSlices(u8, vectors.two_l2_g0.bytes, genesis[0].bytes[0..genesis[0].len]);
    try std.testing.expectEqualSlices(u8, &vectors.two_l2_g0.chain, &old_meta.chain_head);
    const old_cell = try p.cell(old, .{ .planet = @fromBackingInt(2), .local = 71 });
    try demo(&p, 2, 2);
    var budget: t.WorkBudget = .{ .remaining = 1 };
    try std.testing.expectError(error.NotReady, p.advancePhase(.production, &budget));
    try std.testing.expectError(error.NotReady, p.freezePrepared(&budget));
    try std.testing.expectError(error.NotReady, p.installHidden(&budget));
    for (tick_phases) |phase| {
        try advance(&p, phase, 1);
        try std.testing.expectEqualDeep(old, p.activeRoot());
        try std.testing.expectEqualDeep(old_meta, try p.metadata(old));
        try std.testing.expectEqualDeep(old_cell, try p.cell(old, old_cell.id));
        try std.testing.expectError(error.NotFound, p.settlement(old, @fromBackingInt(1)));
    }
    const prepared = try freeze(&p, 1);
    try std.testing.expectEqual(@as(u64, 1), @backingInt(prepared.tick));
    try std.testing.expect(!std.mem.eql(u8, &old_meta.chain_head, &prepared.history_head));
    const city = try p.settlement(prepared.root, @fromBackingInt(1));
    try std.testing.expectEqual(@as(u32, 2), @backingInt(city.owner));
    try std.testing.expectEqual(@as(u32, 12), city.food);
    try std.testing.expectEqual(@as(u64, 9), @backingInt((try p.metadata(prepared.root)).last_sequence));
    for ([_]u64{ 71, 4 }) |local| {
        const cell = try p.cell(prepared.root, .{ .planet = @fromBackingInt(2), .local = local });
        try std.testing.expectEqual(@as(u64, 1), @backingInt(cell.building.?.local));
    }
    try install(&p);
    try std.testing.expectEqualDeep(prepared.root, p.activeRoot());
    try std.testing.expectEqualDeep(old_meta, try p.metadata(old));
    try std.testing.expectEqualDeep(old_cell, try p.cell(old, old_cell.id));
    try restore(&p);
    try std.testing.expectEqualDeep(old, p.activeRoot());
    try retire(&p, prepared.root, 1);
    try std.testing.expectEqual(@as(u16, 1216), p.storage().live_pages);
    try std.testing.expectEqualDeep(old_meta, try p.metadata(old));
    try std.testing.expectError(error.StaleView, p.metadata(prepared.root));
    try std.testing.expect(!@hasDecl(@import("root.zig"), "PlanetAuthority"));
    try std.testing.expect(!@hasDecl(@import("root.zig"), "beginPrepared"));
}

test "cg/planet/zero-budget stable through every phase install restore and retirement" {
    var builder = try planet.createPlanet(std.testing.allocator, .{ .side = 4 }, @fromBackingInt(2), 2, 2);
    defer builder.deinit();
    const before_creation = builder.control.?.builder.cursor;
    var zero: t.WorkBudget = .{ .remaining = 0 };
    try std.testing.expectEqual(.pending, try planet.advancePlanetCreate(&builder, &zero));
    try std.testing.expectEqualDeep(before_creation, builder.control.?.builder.cursor);
    var p = try create(std.testing.allocator, 2, 2, 256);
    defer p.deinit();
    try demo(&p, 2, 2);
    for (tick_phases) |phase| {
        const before_phase = p.control.lane.?.phase;
        const before_step = p.control.lane.?.step;
        const before_index = p.control.lane.?.candidate.prepare_index;
        const before_count = p.storage().staged_count;
        const work = p.storage().work_units;
        try std.testing.expectEqual(.pending, try p.advancePhase(phase, &zero));
        try std.testing.expectEqual(before_phase, p.control.lane.?.phase);
        try std.testing.expectEqual(before_step, p.control.lane.?.step);
        try std.testing.expectEqual(before_index, p.control.lane.?.candidate.prepare_index);
        try std.testing.expectEqual(before_count, p.storage().staged_count);
        try std.testing.expectEqual(work, p.storage().work_units);
        try advance(&p, phase, 7);
    }
    try std.testing.expectEqual(null, try p.freezePrepared(&zero));
    const root = (try freeze(&p, 7)).root;
    try std.testing.expectEqual(.pending, try p.installHidden(&zero));
    try std.testing.expect(!p.control.lane.?.installed);
    try install(&p);
    try std.testing.expectEqual(.pending, try p.restoreHidden(&zero));
    try std.testing.expect(p.control.lane.?.installed);
    try restore(&p);
    try std.testing.expectEqual(.pending, try p.retire(root, &zero));
    try std.testing.expectEqual(null, p.control.lane.?.retiring);
    try retire(&p, root, 7);
}

test "cg/planet/local-genesis-empty-tick matches independent fixtures" {
    var builder = try planet.createPlanet(std.testing.allocator, .{ .side = 4, .capacities = .{ .settlements = 1, .buildings = 1, .freight = 1, .history_receipts = 3 } }, @fromBackingInt(1), 2, 1);
    defer builder.deinit();
    var budget: t.WorkBudget = .{ .remaining = 256 };
    while (try planet.advancePlanetCreate(&builder, &budget) == .pending) budget.remaining = 256;
    var p = try planet.takePlanet(&builder);
    defer p.deinit();
    const old = p.activeRoot();
    var rows: [3]t.LocalReceiptValue = undefined;
    _ = try p.receipts(old, 0, &rows);
    try std.testing.expectEqualSlices(u8, vectors.one_local_genesis.bytes, rows[0].bytes[0..rows[0].len]);
    try std.testing.expectEqualSlices(u8, &vectors.one_local_genesis.chain, &(try p.metadata(old)).chain_head);
    try p.beginPrepared(@fromBackingInt(0), @fromBackingInt(1), creator, &.{}, @fromBackingInt(0));
    for (tick_phases) |phase| try advance(&p, phase, 7);
    const ready = try freeze(&p, 1);
    try std.testing.expectEqualSlices(u8, &vectors.one_local_commit1.chain, &ready.history_head);
    _ = try p.receipts(ready.root, 0, &rows);
    try std.testing.expectEqualSlices(u8, vectors.one_local_input1.bytes, rows[1].bytes[0..rows[1].len]);
    try std.testing.expectEqualSlices(u8, vectors.one_local_commit1.bytes, rows[2].bytes[0..rows[2].len]);
    try install(&p);
    try retire(&p, old, 1);
    try std.testing.expectEqual(@as(u16, 1216), p.storage().live_pages);
    try std.testing.expectError(error.StaleView, p.metadata(old));
    try std.testing.expectEqual(@as(u64, 1), @backingInt((try p.metadata(ready.root)).tick));
}

test "cg/planet/budget scheduling and empty-world global-watermark" {
    var expected: ?[32]u8 = null;
    var expected_work: ?u64 = null;
    for ([_]u16{ 1, 7, 256, 4096 }) |units| {
        var p = try create(std.testing.allocator, 2, 2, units);
        defer p.deinit();
        const old = p.activeRoot();
        try demo(&p, 2, 2);
        for (tick_phases) |phase| try advance(&p, phase, units);
        const ready = try freeze(&p, units);
        try install(&p);
        try retire(&p, old, units);
        if (expected) |head| try std.testing.expectEqualSlices(u8, &head, &ready.history_head) else expected = ready.history_head;
        if (expected_work) |work| try std.testing.expectEqual(work, p.storage().work_units) else expected_work = p.storage().work_units;
        try p.beginPrepared(@fromBackingInt(1), @fromBackingInt(2), creator, &.{}, @fromBackingInt(99));
        for (tick_phases) |phase| try advance(&p, phase, units);
        const second = try freeze(&p, units);
        try std.testing.expectEqual(@as(u64, 99), @backingInt((try p.metadata(second.root)).last_sequence));
        try std.testing.expectEqual(@as(u32, 16), (try p.settlement(second.root, @fromBackingInt(1))).food);
    }
}

test "cg/planet/abort every prepublication phase and partial retain" {
    for (0..tick_phases.len + 2) |stop| {
        var p = try create(std.testing.allocator, 2, 2, 4096);
        defer p.deinit();
        const old = p.activeRoot();
        const meta = try p.metadata(old);
        try demo(&p, 2, 2);
        if (stop == 0) {
            var one: t.WorkBudget = .{ .remaining = 1 };
            _ = try p.advancePhase(.prepare, &one);
            try std.testing.expect(p.control.lane.?.candidate.prepare_retained);
        } else for (tick_phases[0..@min(stop, tick_phases.len)]) |phase| try advance(&p, phase, 7);
        if (stop > tick_phases.len) try install(&p);
        const doomed = try p.candidateRoot();
        try restore(&p);
        try retire(&p, doomed, 1);
        try std.testing.expectEqualDeep(meta, try p.metadata(old));
        try std.testing.expectEqual(@as(u16, 1216), p.storage().live_pages);
        try std.testing.expectEqual(@as(u16, 0), p.storage().staged_count);
    }
}

test "cg/planet/owner validation and wrong-world cells" {
    for (1..5) |owner| {
        var p = try create(std.testing.allocator, 2, 4, 4096);
        defer p.deinit();
        try demo(&p, 2, @intCast(owner));
        for (tick_phases) |phase| try advance(&p, phase, 256);
        const ready = try freeze(&p, 256);
        try std.testing.expectEqual(@as(u32, @intCast(owner)), @backingInt((try p.settlement(ready.root, @fromBackingInt(1))).owner));
    }
    var p = try create(std.testing.allocator, 2, 2, 4096);
    defer p.deinit();
    try demo(&p, 1, 2);
    for (tick_phases) |phase| try advance(&p, phase, 256);
    const ready = try freeze(&p, 256);
    try std.testing.expectEqual(@as(u16, 0), p.control.lane.?.commands.accepted);
    try std.testing.expectEqual(@as(u16, 2), p.control.lane.?.commands.rejected);
    try std.testing.expectError(error.NotFound, p.settlement(ready.root, @fromBackingInt(1)));
    try std.testing.expectError(error.InvalidPlanet, p.cell(ready.root, .{ .planet = @fromBackingInt(1), .local = 70 }));
}

test "cg/planet/ruler wrong-owner and invalid trusted policy preserve old root" {
    var p = try create(std.testing.allocator, 2, 2, 4096);
    defer p.deinit();
    const old = p.activeRoot();
    try demo(&p, 2, 2);
    for (tick_phases) |phase| try advance(&p, phase, 256);
    const first = try freeze(&p, 256);
    try install(&p);
    try retire(&p, old, 256);
    const envelope = try command.encodeEnvelope(.{ .sequence = @fromBackingInt(10), .provenance = .ruler, .actor = @fromBackingInt(1) }, .{ .build = .{
        .settlement = @fromBackingInt(1),
        .kind = .house,
        .anchor = .{ .planet = @fromBackingInt(2), .local = 69 },
        .facing = null,
    } }, try p.envelopeBuffer(0));
    try p.beginPrepared(@fromBackingInt(1), @fromBackingInt(2), .{ .role = .ruler, .civilization = @fromBackingInt(1) }, &.{@intCast(envelope.bytes.len)}, @fromBackingInt(10));
    for (tick_phases) |phase| try advance(&p, phase, 256);
    const second = try freeze(&p, 256);
    try std.testing.expectEqual(@as(u16, 1), p.control.lane.?.commands.rejected);
    try std.testing.expectEqual(@as(u32, 20), (try p.settlement(second.root, @fromBackingInt(1))).material);
    try std.testing.expectEqual(@as(u32, 16), (try p.settlement(second.root, @fromBackingInt(1))).food);
    try restore(&p);
    try retire(&p, second.root, 256);
    try p.beginPrepared(@fromBackingInt(1), @fromBackingInt(2), .{ .role = .creator, .civilization = @fromBackingInt(7) }, &.{}, @fromBackingInt(10));
    try std.testing.expectError(error.InvalidPolicy, advance(&p, .prepare, 256));
    try restore(&p);
    try retire(&p, try p.candidateRoot(), 256);
    try std.testing.expectEqualSlices(u8, &first.history_head, &(try p.metadata(first.root)).chain_head);
}

test "cg/planet/no-poll-allocation and stable retirement target" {
    var failing = std.testing.FailingAllocator.init(std.testing.allocator, .{ .fail_index = 1 });
    var p = try create(failing.allocator(), 2, 2, 7);
    defer p.deinit();
    const old = p.activeRoot();
    try demo(&p, 2, 2);
    for (tick_phases) |phase| try advance(&p, phase, 7);
    const ready = try freeze(&p, 7);
    try install(&p);
    var one: t.WorkBudget = .{ .remaining = 1 };
    try std.testing.expectEqual(.pending, try p.retire(old, &one));
    try std.testing.expectError(error.NotReady, p.restoreHidden(&one));
    try std.testing.expectError(error.NotReady, p.retire(ready.root, &one));
    try retire(&p, old, 7);
    try std.testing.expectEqual(@as(usize, 1), failing.alloc_index);
    try std.testing.expect(!failing.has_induced_failure);
}

test "cg/planet/partial COW allocation swap and release abort preserves references" {
    for (0..3) |boundary| {
        var p = try create(std.testing.allocator, 2, 2, 4096);
        defer p.deinit();
        const old = p.activeRoot();
        const meta = try p.metadata(old);
        try demo(&p, 2, 2);
        try advance(&p, .prepare, 256);
        while (true) {
            var one: t.WorkBudget = .{ .remaining = 1 };
            _ = try p.advancePhase(.commands, &one);
            const candidate = &p.control.lane.?.candidate;
            const writer = p.control.lane.?.commands.writer;
            const reached = switch (boundary) {
                0 => candidate.pending_new_ref != 0,
                1 => if (writer) |w| w.step == .swap else false,
                2 => candidate.pending_old_ref != 0,
                else => unreachable,
            };
            if (reached) break;
            try std.testing.expect(p.control.lane.?.phase == .commands);
        }
        try restore(&p);
        try retire(&p, try p.candidateRoot(), 1);
        try std.testing.expectEqualDeep(meta, try p.metadata(old));
        try std.testing.expectEqual(@as(u16, 1216), p.storage().live_pages);
        for (p.storage().reservation.directories[old.slot][0..1216]) |ref| try std.testing.expectEqual(@as(u16, 1), p.storage().reservation.pool[ref - 1].references);
    }
}

test "cg/planet/history exhaustion rejects freeze and cleanly restores null policy" {
    var builder = try planet.createPlanet(std.testing.allocator, .{ .side = 4, .capacities = .{ .history_receipts = 1 } }, @fromBackingInt(2), 2, 2);
    defer builder.deinit();
    var budget: t.WorkBudget = .{ .remaining = 4096 };
    while (try planet.advancePlanetCreate(&builder, &budget) == .pending) budget.remaining = 4096;
    var p = try planet.takePlanet(&builder);
    defer p.deinit();
    const old = p.activeRoot();
    const initial = try p.metadata(old);
    try p.beginPrepared(@fromBackingInt(0), @fromBackingInt(1), creator, &.{}, @fromBackingInt(12));
    for (tick_phases[0..6]) |phase| try advance(&p, phase, 1);
    try std.testing.expectError(error.HistoryFull, freeze(&p, 1));
    try restore(&p);
    try retire(&p, try p.candidateRoot(), 1);
    try std.testing.expectEqualDeep(initial, try p.metadata(old));
    try std.testing.expectEqual(null, (try p.metadata(old)).policy);
}

test "cg/planet/explicit-root budgeted readers reject stale roots and preserve zero allowance" {
    var p = try create(std.testing.allocator, 2, 2, 4096);
    defer p.deinit();
    const root = p.activeRoot();
    var zero: t.WorkBudget = .{ .remaining = 0 };
    const work = p.storage().work_units;
    try std.testing.expectEqual(null, try p.readRecord(root, .settlement, 0, &zero));
    try std.testing.expectEqual(null, try p.readTerrain(root, 71, &zero));
    try std.testing.expectEqual(null, try p.readRouteCell(root, 0, 0, &zero));
    try std.testing.expectEqual(work, p.storage().work_units);
    var one: t.WorkBudget = .{ .remaining = 1 };
    try std.testing.expectEqual(t.Terrain.fertile, (try p.readTerrain(root, 71, &one)).?);
    try std.testing.expectEqual(@as(u16, 0), one.remaining);
    try std.testing.expectError(error.StaleView, p.readRecord(.{ .slot = root.slot, .generation = root.generation + 1 }, .settlement, 0, &zero));
}

test "cg/planet/creator freight rejects mixed-owner endpoints and paths retain planet identity" {
    var p = try create(std.testing.allocator, 2, 2, 4096);
    defer p.deinit();
    var lengths: [3]u16 = undefined;
    for ([_]u64{ 70, 5 }, 0..) |local, index| {
        const envelope = try command.encodeEnvelope(.{ .sequence = @fromBackingInt(@as(u64, @intCast(index)) + 1), .provenance = .creator, .actor = null }, .{ .author_settlement = .{
            .owner = @fromBackingInt(@as(u32, @intCast(index)) + 1),
            .anchor = .{ .planet = @fromBackingInt(2), .local = local },
            .population = 2,
            .food = 8,
            .material = 20,
            .economic_bypass = true,
        } }, try p.envelopeBuffer(@intCast(index)));
        lengths[index] = @intCast(envelope.bytes.len);
    }
    const path = [_]it.CellId{ .{ .planet = @fromBackingInt(2), .local = 70 }, .{ .planet = @fromBackingInt(2), .local = 71 }, .{ .planet = @fromBackingInt(2), .local = 4 }, .{ .planet = @fromBackingInt(2), .local = 5 } };
    const ship = try command.encodeEnvelope(.{ .sequence = @fromBackingInt(3), .provenance = .creator, .actor = null }, .{ .author_ship = .{
        .source = @fromBackingInt(1),
        .destination = @fromBackingInt(2),
        .resource = .food,
        .amount = 1,
        .path = &path,
        .cargo_source = .transfer,
        .economic_bypass = true,
    } }, try p.envelopeBuffer(2));
    lengths[2] = @intCast(ship.bytes.len);
    try p.beginPrepared(@fromBackingInt(0), @fromBackingInt(1), creator, &lengths, @fromBackingInt(3));
    for (tick_phases) |phase| try advance(&p, phase, 7);
    _ = try freeze(&p, 7);
    try std.testing.expectEqual(@as(u16, 2), (try p.counts()).accepted);
    try std.testing.expectEqual(@as(u16, 1), (try p.counts()).rejected);
    try std.testing.expectEqual(t.RejectReason.WrongOwner, p.storage().logicalReceipt(3).event.rejected.reason);
    try restore(&p);
    try retire(&p, try p.candidateRoot(), 256);
    // Same endpoint owner is legal, and route storage reconstructs this world.
    const second = try command.encodeEnvelope(.{ .sequence = @fromBackingInt(2), .provenance = .creator, .actor = null }, .{ .author_settlement = .{
        .owner = @fromBackingInt(1),
        .anchor = .{ .planet = @fromBackingInt(2), .local = 5 },
        .population = 2,
        .food = 8,
        .material = 20,
        .economic_bypass = true,
    } }, try p.envelopeBuffer(1));
    lengths[1] = @intCast(second.bytes.len);
    try p.beginPrepared(@fromBackingInt(0), @fromBackingInt(1), creator, &lengths, @fromBackingInt(3));
    for (tick_phases) |phase| try advance(&p, phase, 7);
    const ready = try freeze(&p, 7);
    try std.testing.expectEqual(@as(u16, 3), (try p.counts()).accepted);
    for (path, 0..) |expected, index| {
        var budget: t.WorkBudget = .{ .remaining = 1 };
        const actual = (try p.readRouteCell(ready.root, 0, @intCast(index), &budget)).?;
        try std.testing.expectEqual(@as(u32, 2), @backingInt(actual.planet));
        try std.testing.expectEqual(expected.local, actual.local);
    }
}
