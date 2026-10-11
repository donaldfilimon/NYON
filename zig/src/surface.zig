//! Integer cube-surface topology. Each cell belongs to exactly one face.
const std = @import("std");

pub const Face = enum(u3) { positive_x, negative_x, positive_y, negative_y, positive_z, negative_z };
pub const Direction = enum { east, west, north, south };
pub const Cell = struct { face: Face, x: u16, y: u16 };
const Vector = [3]i32;
const Basis = struct { normal: Vector, right: Vector, up: Vector };
const bases = [_]Basis{
    .{ .normal = .{ 1, 0, 0 }, .right = .{ 0, 0, -1 }, .up = .{ 0, 1, 0 } },
    .{ .normal = .{ -1, 0, 0 }, .right = .{ 0, 0, 1 }, .up = .{ 0, 1, 0 } },
    .{ .normal = .{ 0, 1, 0 }, .right = .{ 1, 0, 0 }, .up = .{ 0, 0, -1 } },
    .{ .normal = .{ 0, -1, 0 }, .right = .{ 1, 0, 0 }, .up = .{ 0, 0, 1 } },
    .{ .normal = .{ 0, 0, 1 }, .right = .{ 1, 0, 0 }, .up = .{ 0, 1, 0 } },
    .{ .normal = .{ 0, 0, -1 }, .right = .{ -1, 0, 0 }, .up = .{ 0, 1, 0 } },
};

fn dot(a: Vector, b: Vector) i32 {
    return a[0] * b[0] + a[1] * b[1] + a[2] * b[2];
}

pub const Topology = struct {
    side: u16,

    pub fn init(side: u16) !Topology {
        if (side == 0) return error.InvalidSide;
        return .{ .side = side };
    }

    pub fn cellId(self: Topology, cell: Cell) !u64 {
        try self.validate(cell);
        return @as(u64, @backingInt(cell.face)) * self.side * self.side + @as(u64, cell.y) * self.side + cell.x;
    }

    pub fn fromId(self: Topology, id: u64) !Cell {
        if (self.side == 0) return error.InvalidSide;
        const area = @as(u64, self.side) * self.side;
        if (id >= 6 * area) return error.InvalidCell;
        return .{ .face = @fromBackingInt(@intCast(id / area)), .x = @intCast(id % area % self.side), .y = @intCast(id % area / self.side) };
    }

    fn validate(self: Topology, cell: Cell) !void {
        if (self.side == 0) return error.InvalidSide;
        if (cell.x >= self.side or cell.y >= self.side) return error.InvalidCell;
    }

    pub fn neighbor(self: Topology, cell: Cell, direction: Direction) !Cell {
        try self.validate(cell);
        var result = cell;
        switch (direction) {
            .east => if (cell.x + 1 < self.side) {
                result.x += 1;
                return result;
            },
            .west => if (cell.x > 0) {
                result.x -= 1;
                return result;
            },
            .north => if (cell.y + 1 < self.side) {
                result.y += 1;
                return result;
            },
            .south => if (cell.y > 0) {
                result.y -= 1;
                return result;
            },
        }
        const source = bases[@backingInt(cell.face)];
        var normal = switch (direction) {
            .east, .west => source.right,
            .north, .south => source.up,
        };
        if (direction == .west or direction == .south) for (&normal) |*v| {
            v.* = -v.*;
        };
        const extent = @as(i32, self.side) - 1;
        const x = 2 * @as(i32, cell.x) - extent;
        const y = 2 * @as(i32, cell.y) - extent;
        var position: Vector = undefined;
        for (0..3) |i| position[i] = source.normal[i] * extent + source.right[i] * x + source.up[i] * y;
        for (bases, 0..) |target, index| {
            if (dot(target.normal, normal) != 1) continue;
            return .{ .face = @fromBackingInt(@intCast(index)), .x = @intCast(@divExact(dot(position, target.right) + extent, 2)), .y = @intCast(@divExact(dot(position, target.up) + extent, 2)) };
        }
        unreachable;
    }
};

test "all surface neighbors are reciprocal and IDs round trip including corners" {
    for ([_]u16{ 1, 2, 3, 8 }) |side| {
        const topology = try Topology.init(side);
        for (0..@as(usize, 6) * side * side) |id| {
            const cell = try topology.fromId(id);
            try std.testing.expectEqual(@as(u64, id), try topology.cellId(cell));
            for (std.enums.values(Direction)) |direction| {
                const next = try topology.neighbor(cell, direction);
                _ = try topology.cellId(next);
                var reverse_count: usize = 0;
                for (std.enums.values(Direction)) |reverse| {
                    if (std.meta.eql(cell, try topology.neighbor(next, reverse))) reverse_count += 1;
                }
                try std.testing.expectEqual(@as(usize, 1), reverse_count);
            }
        }
    }
}

test "independent positive Z seam witnesses and input limits" {
    const topology = try Topology.init(4);
    try std.testing.expectEqual(Cell{ .face = .positive_x, .x = 0, .y = 1 }, try topology.neighbor(.{ .face = .positive_z, .x = 3, .y = 1 }, .east));
    try std.testing.expectEqual(Cell{ .face = .positive_y, .x = 2, .y = 0 }, try topology.neighbor(.{ .face = .positive_z, .x = 2, .y = 3 }, .north));
    try std.testing.expectError(error.InvalidSide, Topology.init(0));
    try std.testing.expectError(error.InvalidCell, topology.fromId(96));
    try std.testing.expectError(error.InvalidCell, topology.neighbor(.{ .face = .positive_z, .x = 4, .y = 0 }, .east));
    const maximum = try Topology.init(65535);
    const last = 6 * @as(u64, 65535) * 65535 - 1;
    try std.testing.expectEqual(last, try maximum.cellId(try maximum.fromId(last)));
}
