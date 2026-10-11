const std = @import("std");
const m = @import("classic_math.zig");
test "classic math identity has four diagonal ones" {
    const a = m.Mat4.identity();
    try std.testing.expectEqual(@as(f32, 1), a.cols[0].x);
    try std.testing.expectEqual(@as(f32, 1), a.cols[1].y);
    try std.testing.expectEqual(@as(f32, 1), a.cols[2].z);
    try std.testing.expectEqual(@as(f32, 1), a.cols[3].w);
}
fn vec(x: f32, y: f32, z: f32, w: f32) m.Vec4 {
    return .{ .x = x, .y = y, .z = z, .w = w };
}
fn matrixNear(expected: m.Mat4, actual: m.Mat4, tolerance: f32) !void {
    for (expected.cols, actual.cols) |e, a| {
        inline for (.{ "x", "y", "z", "w" }) |field| try std.testing.expectApproxEqAbs(@field(e, field), @field(a, field), tolerance);
    }
}
const translation = m.Mat4.fromColumns(.{ vec(1, 0, 0, 0), vec(0, 1, 0, 0), vec(0, 0, 1, 0), vec(3, -2, 5, 1) });
const rotation = m.Mat4.fromColumns(.{ vec(0, 1, 0, 0), vec(-1, 0, 0, 0), vec(0, 0, 1, 0), vec(0, 0, 0, 1) });
test "classic math identity translation direction and composition" {
    const point = vec(2, -3, 4, 1);
    try std.testing.expectEqualDeep(point, m.Mat4.identity().mulVec4(point));
    try std.testing.expectEqualDeep(vec(5, -5, 9, 1), translation.mulVec4(point));
    try std.testing.expectEqualDeep(vec(2, -3, 4, 0), translation.mulVec4(vec(2, -3, 4, 0)));
    try std.testing.expectEqualDeep(vec(3, 2, 4, 1), rotation.mulVec4(point));
    try std.testing.expectEqualDeep(vec(6, 0, 9, 1), translation.mulMat4(rotation).mulVec4(point));
    try std.testing.expectEqualDeep(vec(5, 5, 9, 1), rotation.mulMat4(translation).mulVec4(point));
    try std.testing.expectEqualDeep(translation.cols, translation.toColumns());
}
test "classic math inverse translation quarter turn diagonal and dense literals" {
    try matrixNear(m.Mat4.fromColumns(.{ vec(1, 0, 0, 0), vec(0, 1, 0, 0), vec(0, 0, 1, 0), vec(-3, 2, -5, 1) }), translation.inverseUnchecked(), 0);
    try matrixNear(m.Mat4.fromColumns(.{ vec(0, -1, 0, 0), vec(1, 0, 0, 0), vec(0, 0, 1, 0), vec(0, 0, 0, 1) }), rotation.inverseUnchecked(), 0);
    const diagonal = m.Mat4.fromColumns(.{ vec(2, 0, 0, 0), vec(0, 4, 0, 0), vec(0, 0, 8, 0), vec(0, 0, 0, 1) });
    try matrixNear(m.Mat4.fromColumns(.{ vec(0.5, 0, 0, 0), vec(0, 0.25, 0, 0), vec(0, 0, 0.125, 0), vec(0, 0, 0, 1) }), diagonal.inverseUnchecked(), 0);
    // Explicit column conversion of independently reviewed rational row literals.
    const a = m.Mat4.fromColumns(.{ vec(1, 0, 2, 1), vec(2, 1, 0, 1), vec(3, 2, 1, 0), vec(4, 3, 1, 1) });
    const b = m.Mat4.fromColumns(.{ vec(0, 1, 1, -1), vec(-0.2, -1.2, -1, 1.4), vec(0.4, -0.6, 0, 0.2), vec(0.2, 0.2, -1, 0.6) });
    try matrixNear(b, a.inverseUnchecked(), 1e-6);
    try matrixNear(m.Mat4.identity(), a.mulMat4(a.inverseUnchecked()), 1e-6);
    try matrixNear(m.Mat4.identity(), a.inverseUnchecked().mulMat4(a), 1e-6);
}
test "classic math unchecked inverse singular and determinant overflow classification" {
    const zero = m.Mat4.fromColumns(@splat(vec(0, 0, 0, 0)));
    try std.testing.expect(!zero.inverseUnchecked().isFinite());
    var singular = m.Mat4.identity();
    singular.cols[1] = singular.cols[0];
    try std.testing.expect(!singular.inverseUnchecked().isFinite());
    const huge = m.Mat4.fromColumns(.{ vec(1e10, 0, 0, 0), vec(0, 1e10, 0, 0), vec(0, 0, 1e10, 0), vec(0, 0, 0, 1e10) });
    try std.testing.expect(huge.inverseUnchecked().isFinite());
    try matrixNear(zero, huge.inverseUnchecked(), 0);
}
test "classic math normalize and vector witnesses" {
    const n = m.tryNormalize(.{ .x = 3, .y = 4, .z = 0 }).?;
    try std.testing.expectApproxEqAbs(@as(f32, 0.6), n.x, 1e-6);
    try std.testing.expectApproxEqAbs(@as(f32, 0.8), n.y, 1e-6);
    try std.testing.expectEqual(@as(f32, 0), n.z);
    try std.testing.expect(m.tryNormalize(m.Vec3.zero) == null);
    try std.testing.expect(m.tryNormalize(.{ .x = std.math.inf(f32), .y = 0, .z = 0 }) == null);
    try std.testing.expect(m.tryNormalize(.{ .x = std.math.nan(f32), .y = 0, .z = 0 }) == null);
    try std.testing.expectEqual(@as(f32, 25), m.lengthSquared(.{ .x = 3, .y = 4, .z = 0 }));
    try std.testing.expectEqual(@as(f32, 5), m.distance(m.Vec3.zero, .{ .x = 3, .y = 4, .z = 0 }));
    try std.testing.expectEqualDeep(m.Vec3{ .x = 0, .y = 0, .z = 1 }, m.cross(.{ .x = 1, .y = 0, .z = 0 }, .{ .x = 0, .y = 1, .z = 0 }));
}
test "classic math raw bit total order includes signed zero infinities NaN payloads" {
    const bits = [_]u32{ 0xffc00002, 0xffc00001, 0xff800000, 0xbf800000, 0x80000000, 0, 0x3f800000, 0x7f800000, 0x7fc00001, 0x7fc00002 };
    for (bits[0 .. bits.len - 1], bits[1..]) |a, b| {
        try std.testing.expectEqual(std.math.Order.lt, m.totalOrder(@bitCast(a), @bitCast(b)));
        try std.testing.expectEqual(std.math.Order.gt, m.totalOrder(@bitCast(b), @bitCast(a)));
        try std.testing.expectEqual(std.math.Order.eq, m.totalOrder(@bitCast(a), @bitCast(a)));
    }
}
test "classic math lookAt and projection independent depth witnesses" {
    try matrixNear(m.Mat4.fromColumns(.{ vec(1, 0, 0, 0), vec(0, 1, 0, 0), vec(0, 0, 1, 0), vec(0, 0, -5, 1) }), m.lookAtRH(.{ .x = 0, .y = 0, .z = 5 }, m.Vec3.zero, .{ .x = 0, .y = 1, .z = 0 }), 0);
    const analytic = m.Mat4.fromColumns(.{ vec(0.5, 0, 0, 0), vec(0, 1, 0, 0), vec(0, 0, -2, -1), vec(0, 0, -2, 0) });
    const public = m.perspectiveRHZO(@as(f32, std.math.pi / 2.0), 2, 1, 2);
    try matrixNear(analytic, public, 1e-6);
    for ([_]f32{ 1, 2 }, [_]f32{ 0, 1 }) |z, expected| {
        const clip = public.mulVec4(vec(0, 0, -z, 1));
        try std.testing.expectApproxEqAbs(expected, clip.z / clip.w, 1e-6);
    }
    const production = m.perspectiveRHZO(m.radians(45), 1.6, 0.1, 100);
    for ([_]f32{ 0.1, 100 }, [_]f32{ 0, 1 }) |z, expected| {
        const clip = production.mulVec4(vec(0, 0, -z, 1));
        try std.testing.expectApproxEqAbs(expected, clip.z / clip.w, 1e-5);
    }
    try std.testing.expect(!m.lookAtRH(m.Vec3.zero, m.Vec3.zero, .{ .x = 0, .y = 1, .z = 0 }).isFinite());
}
