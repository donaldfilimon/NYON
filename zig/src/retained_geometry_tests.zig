const std = @import("std");
const g = @import("retained_geometry.zig");
const Rect = g.Rect;

test "retained geometry constructor literals and degenerate edges" {
    const r = try Rect.fromXYWH(10, 20, -4, 8);
    try std.testing.expectEqualDeep(Rect{ .min = .{ 10, 20 }, .max = .{ 10, 28 } }, r);
    try std.testing.expect(r.contains(.{ 10, 28 }));
    try std.testing.expect(!r.contains(.{ 10.001, 28 }));
    try std.testing.expect(r.intersection(r) == null);
    const empty = try Rect.fromXYWH(-2, -3, -1, -8);
    try std.testing.expectEqualDeep(Rect{ .min = .{ -2, -3 }, .max = .{ -2, -3 } }, empty);
    try std.testing.expect(empty.contains(.{ -2, -3 }));
    try std.testing.expect(!empty.overlaps(empty));
    try std.testing.expectEqualDeep(Rect{ .min = .{ -2, -3 }, .max = .{ 2, 5 } }, try Rect.fromXYWH(-2, -3, 4, 8));
}

test "retained geometry all nonfinite constructor lanes reject before clamping" {
    const bad = [_]f32{ std.math.nan(f32), std.math.inf(f32), -std.math.inf(f32) };
    for (bad) |value| {
        for (0..4) |lane| {
            var xywh: [4]f32 = .{ 1, 2, 3, 4 };
            xywh[lane] = value;
            try std.testing.expectError(error.NonFinite, Rect.fromXYWH(xywh[0], xywh[1], xywh[2], xywh[3]));
            var bounds: [4]f32 = .{ 1, 2, 3, 4 };
            bounds[lane] = value;
            try std.testing.expectError(error.NonFinite, Rect.fromBounds(.{ bounds[0], bounds[1] }, .{ bounds[2], bounds[3] }));
        }
    }
}

test "retained geometry bounds inversion and derived maxima overflow" {
    try std.testing.expectError(error.InvalidBounds, Rect.fromBounds(.{ 3, 2 }, .{ 1, 4 }));
    try std.testing.expectError(error.InvalidBounds, Rect.fromBounds(.{ 1, 4 }, .{ 3, 2 }));
    const max = std.math.floatMax(f32);
    try std.testing.expectError(error.Overflow, Rect.fromXYWH(max, 0, max, 1));
    try std.testing.expectError(error.Overflow, Rect.fromXYWH(0, max, 1, max));
    try std.testing.expectEqualDeep(Rect{ .min = .{ -max, -max }, .max = .{ max, max } }, try Rect.fromBounds(.{ -max, -max }, .{ max, max }));
    try std.testing.expectEqualDeep(Rect{ .min = .{ max, max }, .max = .{ max, max } }, try Rect.fromXYWH(max, max, -1, -1));
}

test "retained geometry inclusive containment and nonfinite points" {
    const r = try Rect.fromBounds(.{ 0, 0 }, .{ 10, 10 });
    for ([_][2]f32{ .{ 0, 0 }, .{ 0, 10 }, .{ 10, 0 }, .{ 10, 10 }, .{ 5, 5 } }) |p| try std.testing.expect(r.contains(p));
    for ([_][2]f32{ .{ -0.001, 5 }, .{ 10.001, 5 }, .{ 5, -0.001 }, .{ 5, 10.001 } }) |p| try std.testing.expect(!r.contains(p));
    for ([_]f32{ std.math.nan(f32), std.math.inf(f32), -std.math.inf(f32) }) |bad| {
        try std.testing.expect(!r.contains(.{ bad, 5 }));
        try std.testing.expect(!r.contains(.{ 5, bad }));
    }
    try std.testing.expect(r.containsRect(r));
    try std.testing.expect(r.containsRect(try Rect.fromBounds(.{ 10, 2 }, .{ 10, 8 })));
    try std.testing.expect(!r.containsRect(try Rect.fromBounds(.{ -1, 2 }, .{ 8, 8 })));
    try std.testing.expect(!r.containsRect(try Rect.fromBounds(.{ 2, -1 }, .{ 8, 8 })));
    try std.testing.expect(!r.containsRect(try Rect.fromBounds(.{ 2, 2 }, .{ 11, 8 })));
    try std.testing.expect(!r.containsRect(try Rect.fromBounds(.{ 2, 2 }, .{ 8, 11 })));
}

test "retained geometry positive intersection literals touching and symmetry" {
    const a = try Rect.fromBounds(.{ 2, 3 }, .{ 8, 9 });
    const b = try Rect.fromBounds(.{ 5, 1 }, .{ 12, 7 });
    const expected = Rect{ .min = .{ 5, 3 }, .max = .{ 8, 7 } };
    try std.testing.expect(a.intersection(b) != null);
    try std.testing.expectEqualDeep(expected, a.intersection(b).?);
    try std.testing.expectEqualDeep(expected, b.intersection(a).?);
    const square = try Rect.fromBounds(.{ 0, 0 }, .{ 10, 10 });
    for ([_]Rect{
        .{ .min = .{ 10, 2 }, .max = .{ 12, 8 } },
        .{ .min = .{ 2, 10 }, .max = .{ 8, 12 } },
        .{ .min = .{ 10, 10 }, .max = .{ 12, 12 } },
        .{ .min = .{ 11, 2 }, .max = .{ 12, 8 } },
        .{ .min = .{ 2, 11 }, .max = .{ 8, 12 } },
    }) |other| {
        try std.testing.expect(square.intersection(other) == null);
        try std.testing.expect(other.intersection(square) == null);
        try std.testing.expect(!square.overlaps(other));
    }
}

test "retained geometry overlap threshold strict on both axes" {
    const square = try Rect.fromBounds(.{ 0, 0 }, .{ 10, 10 });
    const sliver = try Rect.fromBounds(.{ 9.995, 0 }, .{ 12, 10 });
    try std.testing.expect(square.intersection(sliver) != null);
    try std.testing.expect(!square.overlaps(sliver));
    // Origin-based literals keep the exact f32 threshold free of subtraction cancellation.
    for ([_][2]f32{ .{ 0.005, 1 }, .{ 1, 0.005 }, .{ 0.01, 1 }, .{ 1, 0.01 }, .{ 0.01, 0.01 } }) |max| {
        const small = try Rect.fromBounds(.{ 0, 0 }, max);
        try std.testing.expect(square.intersection(small) != null);
        try std.testing.expect(!square.overlaps(small));
        try std.testing.expect(!small.overlaps(square));
    }
    const positive = try Rect.fromBounds(.{ 0, 0 }, .{ 0.011, 0.011 });
    try std.testing.expect(square.overlaps(positive));
    try std.testing.expect(positive.overlaps(square));
}
