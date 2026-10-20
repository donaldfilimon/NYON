const std = @import("std");
const pe = @import("root.zig");
const surface = @import("../surface.zig");

const Vec = [3]i32;
const Frame = struct { normal: Vec, right: Vec, up: Vec };
// Independent six-face basis oracle: expected neighbors come from these
// literals and coordinate projection, never from Topology.neighbor.
const frames = [_]Frame{
    .{ .normal = .{ 1, 0, 0 }, .right = .{ 0, 0, -1 }, .up = .{ 0, 1, 0 } },
    .{ .normal = .{ -1, 0, 0 }, .right = .{ 0, 0, 1 }, .up = .{ 0, 1, 0 } },
    .{ .normal = .{ 0, 1, 0 }, .right = .{ 1, 0, 0 }, .up = .{ 0, 0, -1 } },
    .{ .normal = .{ 0, -1, 0 }, .right = .{ 1, 0, 0 }, .up = .{ 0, 0, 1 } },
    .{ .normal = .{ 0, 0, 1 }, .right = .{ 1, 0, 0 }, .up = .{ 0, 1, 0 } },
    .{ .normal = .{ 0, 0, -1 }, .right = .{ -1, 0, 0 }, .up = .{ 0, 1, 0 } },
};

fn dot(a: Vec, b: Vec) i32 {
    return a[0] * b[0] + a[1] * b[1] + a[2] * b[2];
}

fn oracleNeighbor(side: u16, id: u64, direction: surface.Direction) u64 {
    const area = @as(u64, side) * side;
    const face: usize = @intCast(id / area);
    const x: i32 = @intCast(id % area % side);
    const y: i32 = @intCast(id % area / side);
    const span: i32 = @as(i32, side) - 1;
    const dx: i32 = switch (direction) {
        .east => 1,
        .west => -1,
        else => 0,
    };
    const dy: i32 = switch (direction) {
        .north => 1,
        .south => -1,
        else => 0,
    };
    const nx = x + dx;
    const ny = y + dy;
    if (nx >= 0 and nx <= span and ny >= 0 and ny <= span)
        return @as(u64, face) * area + @as(u64, @intCast(ny)) * side + @as(u64, @intCast(nx));

    const from = frames[face];
    var new_normal: Vec = undefined;
    var position: Vec = undefined;
    for (0..3) |axis| {
        new_normal[axis] = from.right[axis] * dx + from.up[axis] * dy;
        position[axis] = from.normal[axis] * span + from.right[axis] * (2 * x - span) + from.up[axis] * (2 * y - span);
    }
    for (frames, 0..) |to, target| {
        if (dot(to.normal, new_normal) != 1) continue;
        const target_x: u64 = @intCast(@divExact(dot(position, to.right) + span, 2));
        const target_y: u64 = @intCast(@divExact(dot(position, to.up) + span, 2));
        return @as(u64, target) * area + target_y * side + target_x;
    }
    unreachable;
}

test "pe/contracts/limits-and-types" {
    try pe.rules.validateConfig(.{ .seed = 2, .side = 32 });
    try pe.rules.validateConfig(.{ .seed = 2, .side = 4, .capacities = .{ .settlements = 1, .buildings = 1, .shipments = 1, .history_receipts = 1 } });
    for ([_]u16{ 0, 1, 3, 33, 65535 }) |side|
        try std.testing.expectError(error.CapacityUnsupported, pe.rules.validateConfig(.{ .seed = 2, .side = side }));
    try std.testing.expectError(error.CapacityUnsupported, pe.rules.validateConfig(.{ .seed = 2, .capacities = .{ .settlements = 0 } }));
    try std.testing.expectError(error.CapacityUnsupported, pe.rules.validateConfig(.{ .seed = 2, .capacities = .{ .buildings = 0 } }));
    try std.testing.expectError(error.CapacityUnsupported, pe.rules.validateConfig(.{ .seed = 2, .capacities = .{ .shipments = 0 } }));
    try std.testing.expectError(error.CapacityUnsupported, pe.rules.validateConfig(.{ .seed = 2, .capacities = .{ .history_receipts = 0 } }));
    try std.testing.expectError(error.CapacityUnsupported, pe.rules.validateConfig(.{ .seed = 2, .capacities = .{ .settlements = 65 } }));
    try std.testing.expectError(error.CapacityUnsupported, pe.rules.validateConfig(.{ .seed = 2, .capacities = .{ .buildings = 257 } }));
    try std.testing.expectError(error.CapacityUnsupported, pe.rules.validateConfig(.{ .seed = 2, .capacities = .{ .shipments = 65 } }));
    try std.testing.expectError(error.CapacityUnsupported, pe.rules.validateConfig(.{ .seed = 2, .capacities = .{ .history_receipts = 4097 } }));
    try std.testing.expect(pe.ids.SettlementId != pe.ids.BuildingId);
    try std.testing.expect(pe.ids.PlanetId != pe.play_role.CivilizationId);
    try std.testing.expectEqual(pe.Terrain.fertile, pe.rules.terrain(2, .{ .planet = @fromBackingInt(@as(u32, 1)), .local = 71 }));
    try std.testing.expectEqual(pe.Terrain.rocky, pe.rules.terrain(2, .{ .planet = @fromBackingInt(@as(u32, 1)), .local = 74 }));
}

test "pe/contracts/seams" {
    const topology = try surface.Topology.init(4);
    const anchor = pe.CellId{ .planet = pe.planet_id, .local = 71 };
    const farm = try pe.rules.footprint(topology, .farm, anchor, .east);
    try std.testing.expectEqual(@as(u8, 2), farm.len);
    try std.testing.expectEqual(@as(u64, 71), farm.cells[0].local);
    try std.testing.expectEqual(@as(u64, 4), farm.cells[1].local);
    try std.testing.expectEqual(@as(u64, 34), try topology.cellId(try topology.neighbor(try topology.fromId(78), .north)));
    const across = try topology.neighbor(try topology.fromId(14), .north);
    try std.testing.expectEqual(@as(u64, 43), try topology.cellId(across));
    try std.testing.expectEqual(@as(u64, 14), try topology.cellId(try topology.neighbor(across, .east)));
    try std.testing.expectError(error.InvalidEnvelope, pe.rules.footprint(topology, .farm, anchor, null));
    try std.testing.expectError(error.InvalidEnvelope, pe.rules.footprint(topology, .house, anchor, .east));
    try std.testing.expectError(error.InvalidPlanet, pe.rules.footprint(topology, .farm, .{ .planet = @fromBackingInt(@as(u32, 2)), .local = 71 }, .east));
    try std.testing.expectError(error.InvalidCell, pe.rules.footprint(topology, .farm, .{ .planet = pe.planet_id, .local = 96 }, .east));
    try std.testing.expectError(error.CapacityUnsupported, pe.rules.footprint(.{ .side = 0 }, .farm, anchor, .east));

    var directed_seams: [6][4]bool = @splat(@splat(false));
    for (4..33) |side_usize| {
        const side: u16 = @intCast(side_usize);
        const t = try surface.Topology.init(side);
        const area = @as(u64, side) * side;
        for (0..6 * area) |id| {
            const cell = try t.fromId(id);
            const here = pe.CellId{ .planet = pe.planet_id, .local = id };
            for (std.enums.values(surface.Direction), 0..) |direction, dir_index| {
                const expected = oracleNeighbor(side, id, direction);
                const actual = try t.cellId(try t.neighbor(cell, direction));
                try std.testing.expectEqual(expected, actual);
                const fp = try pe.rules.footprint(t, .farm, here, direction);
                try std.testing.expectEqual(expected, fp.cells[1].local);
                if (id / area != expected / area) directed_seams[@intCast(id / area)][dir_index] = true;
            }
        }
    }
    for (directed_seams) |face| for (face) |seen| {
        try std.testing.expect(seen);
    };
}

test "pe/contracts/wire-witnesses" {
    try std.testing.expectEqual(@as(usize, 80), pe.wire_vectors.G0.bytes.len);
    try std.testing.expectEqual(@as(usize, 50), pe.wire_vectors.I1.bytes.len);
    try std.testing.expectEqual(@as(usize, 77), pe.wire_vectors.A2.bytes.len);
    try std.testing.expectEqual(@as(usize, 71), pe.wire_vectors.R2.bytes.len);
    const command_tags = [_]pe.types.CommandTag{ .author_settlement, .grant, .author_build, .author_ship, .found, .build, .ship };
    for (command_tags, 0..) |tag, value| try std.testing.expectEqual(@as(u8, @intCast(value)), @backingInt(tag));
    const event_tags = [_]pe.types.EventTag{ .genesis, .tick_input, .accepted, .rejected, .arrived, .completed, .produced, .idle, .supplied, .demography, .tick_complete };
    for (event_tags, 0..) |tag, value| try std.testing.expectEqual(@as(u8, @intCast(value)), @backingInt(tag));
    try std.testing.expectEqual(@as(u8, 0), @backingInt(pe.IdleReason.workforce));
    try std.testing.expectEqual(@as(u8, 1), @backingInt(pe.IdleReason.output_capacity));
    try std.testing.expectEqual(@as(u8, 0), @backingInt(pe.DemographyKind.growth));
    try std.testing.expectEqual(@as(u8, 1), @backingInt(pe.DemographyKind.starvation));
    try std.testing.expectEqual(@as(u8, 2), @backingInt(pe.DemographyKind.housing_blocked));
    const reject_tags = [_]pe.RejectReason{ .CreatorForbidden, .RulerForbidden, .CivilizationRequired, .WrongCivilization, .InvalidActor, .InternalCommandOnly, .CommandProvenanceMismatch, .ExplicitBypassRequired, .UnknownCivilization, .UnknownSettlement, .WrongOwner, .InvalidPlanet, .InvalidCell, .InvalidAmount, .InvalidPopulation, .InvalidFacing, .TerrainMismatch, .OccupiedCell, .DisconnectedFootprint, .SameEndpoint, .InvalidPathLength, .RepeatedCell, .NonAdjacentPath, .ForeignPathCell, .SettlementCapacity, .BuildingCapacity, .SettlementBuildingCapacity, .FreightCapacity, .PopulationCapacity, .InventoryCapacity, .InsufficientFood, .InsufficientMaterial, .InsufficientPopulation, .TickHorizon, .IdExhausted, .InvalidGrant, .PathEndpointMismatch };
    for (reject_tags, 0..) |tag, index| try std.testing.expectEqual(@as(u16, @intCast(index + 1)), @backingInt(tag));
    const sha = std.crypto.hash.sha2.Sha256;
    var h0: [32]u8 = undefined;
    sha.hash("NYON-PE0-HISTORY", &h0, .{});
    try std.testing.expectEqualSlices(u8, &pe.wire_vectors.h0, &h0);
    inline for (.{ pe.wire_vectors.G0, pe.wire_vectors.I1, pe.wire_vectors.A2, pe.wire_vectors.R2 }) |vector| {
        var hash = sha.init(.{});
        hash.update(&h0);
        const len: u32 = @intCast(vector.bytes.len);
        const le = [_]u8{ @truncate(len), @truncate(len >> 8), @truncate(len >> 16), @truncate(len >> 24) };
        hash.update(&le);
        hash.update(vector.bytes);
        var digest: [32]u8 = undefined;
        hash.final(&digest);
        try std.testing.expectEqualSlices(u8, &vector.chain, &digest);
    }
}

test "pe/contracts/shared algorithm defaults remain fixed island identity" {
    const context: pe.rules.AlgorithmContext = .{};
    try std.testing.expectEqual(pe.planet_id, context.planet);
    try std.testing.expect(context.validOwner(pe.civilization_id));
    for ([_]u32{ 0, 1, 2, 3, 4, 8 }) |owner| try std.testing.expect(!context.validOwner(@fromBackingInt(owner)));
    try std.testing.expectEqual(@as(usize, 8_642_560), @sizeOf(pe.state.Reservation));
}
