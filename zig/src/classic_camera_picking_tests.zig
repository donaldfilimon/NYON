const std = @import("std");
const math = @import("classic_math.zig");
const camera = @import("classic_camera.zig");
test "classic camera reduced orbit snaps to yaw clamp" {
    var c = camera.CameraState.init(.{ .x = 960, .y = 600 });
    c.setMotion(.reduced);
    c.orbit(100, 100);
    try std.testing.expectEqual(camera.MAX_YAW_RADIANS, c.yaw);
}
fn expectInverse(m: camera.CameraMatrices) !void {
    try std.testing.expect(m.isFinite());
    const product = m.view_projection.mulMat4(m.inverse_view_projection);
    for (product.cols, math.Mat4.identity().cols) |a, e| {
        inline for (.{ "x", "y", "z", "w" }) |field| try std.testing.expectApproxEqAbs(@field(e, field), @field(a, field), 1e-4);
    }
}
test "classic camera source viewport extrema retain finite inverse matrices" {
    const viewports = [_]math.Vec2{ .{ .x = 1, .y = 1 }, .{ .x = 960, .y = 600 }, .{ .x = 7680, .y = 4320 } };
    for (viewports) |viewport| {
        var c = camera.CameraState.init(viewport);
        try expectInverse(c.current_matrices);
        c.setMotion(.reduced);
        for ([_]f32{ -100, 100 }) |delta| {
            c.orbit(delta, delta);
            c.zoom(delta * 1000);
            try expectInverse(c.current_matrices);
            try expectInverse(c.target_matrices);
        }
    }
}
test "classic camera repeated clamps nonfinite controls and reduced reset" {
    var c = camera.CameraState.init(.{ .x = 960, .y = 600 });
    c.setMotion(.reduced);
    for (0..100) |_| {
        c.orbit(100, 100);
        c.zoom(100000);
    }
    try std.testing.expectEqual(camera.MAX_YAW_RADIANS, c.yaw);
    try std.testing.expectEqual(camera.MAX_PITCH_RADIANS, c.pitch);
    try std.testing.expectEqual(camera.MIN_DISTANCE, c.distance);
    for (0..100) |_| {
        c.orbit(-100, -100);
        c.zoom(-100000);
    }
    try std.testing.expectEqual(camera.MIN_YAW_RADIANS, c.yaw);
    try std.testing.expectEqual(camera.MIN_PITCH_RADIANS, c.pitch);
    try std.testing.expectEqual(camera.MAX_DISTANCE, c.distance);
    const before = c;
    c.orbit(std.math.nan(f32), std.math.inf(f32));
    c.zoom(-std.math.inf(f32));
    try std.testing.expectEqualDeep(before, c);
    c.target = .{ .x = 1, .y = 2, .z = 3 };
    c.reset();
    try std.testing.expectEqualDeep(math.Vec3{ .x = 1, .y = 2, .z = 3 }, c.target);
    try std.testing.expectEqual(camera.DEFAULT_YAW_RADIANS, c.yaw);
    try std.testing.expectEqual(camera.DEFAULT_PITCH_RADIANS, c.pitch);
    try std.testing.expectEqual(camera.DEFAULT_DISTANCE, c.distance);
    try expectInverse(c.current_matrices);
}
test "classic camera full reset keeps current until smoothing and motion transition" {
    var c = camera.CameraState.init(.{ .x = 960, .y = 600 });
    c.setMotion(.reduced);
    c.orbit(100, 100);
    c.zoom(100000);
    c.setMotion(.full);
    const prior = c.current_matrices;
    const yaw = c.yaw;
    c.reset();
    try std.testing.expectEqual(yaw, c.yaw);
    try std.testing.expectEqualDeep(prior, c.current_matrices);
    try std.testing.expectEqual(camera.DEFAULT_YAW_RADIANS, c.target_yaw);
    c.update(0);
    try std.testing.expectEqual(yaw, c.yaw);
    c.setMotion(.reduced);
    try std.testing.expectEqual(camera.DEFAULT_YAW_RADIANS, c.yaw);
    try std.testing.expectEqualDeep(c.target_matrices, c.current_matrices);
}
test "classic camera analytic duration blend and quarter second saturation" {
    const durations = [_]u128{ 0, 16_666_666, 16_666_667, 250_000_000 };
    for (durations) |ns| {
        var c = camera.CameraState.init(.{ .x = 960, .y = 600 });
        c.orbit(0.5, 0.1);
        c.zoom(100);
        const old = c;
        c.update(ns);
        const seconds: f64 = @as(f64, @floatFromInt(ns)) / 1_000_000_000.0;
        const blend: f32 = @floatCast(1 - @exp(-12 * seconds));
        try std.testing.expectApproxEqAbs(old.yaw + (old.target_yaw - old.yaw) * blend, c.yaw, 1e-6);
        try std.testing.expectApproxEqAbs(old.pitch + (old.target_pitch - old.pitch) * blend, c.pitch, 1e-6);
        try std.testing.expectApproxEqAbs(old.distance + (old.target_distance - old.distance) * blend, c.distance, 1e-6);
    }
    var a = camera.CameraState.init(.{ .x = 960, .y = 600 });
    a.orbit(0.5, 0.1);
    a.zoom(100);
    var b = a;
    var extended = a;
    a.update(250_000_000);
    b.update(1_000_000_000);
    extended.update(std.math.maxInt(u128));
    try std.testing.expectEqualDeep(a, b);
    try std.testing.expectEqualDeep(a, extended);
    try std.testing.expectApproxEqAbs(@as(f32, 0.9502129316) * 0.5, a.yaw, 1e-6);
}
test "classic camera viewport components sanitize independently and eye uses target" {
    var c = camera.CameraState.init(.{ .x = std.math.nan(f32), .y = 600 });
    try std.testing.expectEqualDeep(math.Vec2{ .x = 1, .y = 600 }, c.logical_viewport);
    c.orbit(0.3, 0.1);
    c.setViewport(.{ .x = 960, .y = -std.math.inf(f32) });
    try std.testing.expectEqualDeep(math.Vec2{ .x = 960, .y = 1 }, c.logical_viewport);
    try expectInverse(c.current_matrices);
    try expectInverse(c.target_matrices);
    c.setViewport(.{ .x = -4, .y = 0 });
    try std.testing.expectEqualDeep(math.Vec2{ .x = 1, .y = 1 }, c.logical_viewport);
    c.target = .{ .x = 2, .y = 3, .z = 4 };
    c.yaw = 0;
    c.pitch = 0;
    c.distance = 5;
    try std.testing.expectEqualDeep(math.Vec3{ .x = 2, .y = 3, .z = 9 }, c.eye());
    c.yaw = std.math.nan(f32);
    c.setViewport(.{ .x = 960, .y = 600 });
    try std.testing.expect(!c.current_matrices.isFinite());
}
const picking = @import("classic_picking.zig");
test "classic picking identity center unprojects origin and positive Z" {
    const r = picking.Ray.fromLogicalPointer(.{ .x = 480, .y = 300 }, .{ .x = 960, .y = 600 }, math.Mat4.identity());
    try std.testing.expect(r != null);
    try std.testing.expectEqualDeep(math.Vec3.zero, r.?.origin);
    try std.testing.expectEqualDeep(math.Vec3{ .x = 0, .y = 0, .z = 1 }, r.?.direction);
}
fn vec(x: f32, y: f32, z: f32, w: f32) math.Vec4 {
    return .{ .x = x, .y = y, .z = z, .w = w };
}
const forward = picking.Ray{ .origin = .{ .x = 0, .y = 0, .z = 5 }, .direction = .{ .x = 0, .y = 0, .z = -1 } };
test "classic picking analytic projection near far and outside viewport admission" {
    const projection = math.Mat4.fromColumns(.{ vec(0.5, 0, 0, 0), vec(0, 1, 0, 0), vec(0, 0, -2, -1), vec(0, 0, -2, 0) });
    const r = picking.Ray.fromLogicalPointer(.{ .x = 480, .y = 300 }, .{ .x = 960, .y = 600 }, projection.inverseUnchecked()).?;
    try std.testing.expectEqualDeep(math.Vec3{ .x = 0, .y = 0, .z = -1 }, r.origin);
    try std.testing.expectEqualDeep(math.Vec3{ .x = 0, .y = 0, .z = -1 }, r.direction);
    const outside = picking.Ray.fromLogicalPointer(.{ .x = -480, .y = 900 }, .{ .x = 960, .y = 600 }, math.Mat4.identity()).?;
    try std.testing.expectEqualDeep(math.Vec3{ .x = -2, .y = -2, .z = 0 }, outside.origin);
    try std.testing.expectEqualDeep(math.Vec3{ .x = 0, .y = 0, .z = 1 }, outside.direction);
}
test "classic picking invalid pointer viewport homogeneous and divided coordinates" {
    const pointer = math.Vec2{ .x = 0, .y = 0 };
    const viewport = math.Vec2{ .x = 960, .y = 600 };
    const identity = math.Mat4.identity();
    try std.testing.expect(picking.Ray.fromLogicalPointer(.{ .x = std.math.nan(f32), .y = 0 }, viewport, identity) == null);
    try std.testing.expect(picking.Ray.fromLogicalPointer(.{ .x = 0, .y = std.math.inf(f32) }, viewport, identity) == null);
    for ([_]math.Vec2{ .{ .x = 0, .y = 600 }, .{ .x = 960, .y = -1 }, .{ .x = std.math.inf(f32), .y = 600 }, .{ .x = 960, .y = std.math.nan(f32) } }) |v|
        try std.testing.expect(picking.Ray.fromLogicalPointer(pointer, v, identity) == null);
    const zero = math.Mat4.fromColumns(@splat(vec(0, 0, 0, 0)));
    try std.testing.expect(picking.Ray.fromLogicalPointer(pointer, viewport, zero) == null);
    try std.testing.expect(picking.Ray.fromLogicalPointer(pointer, viewport, zero.inverseUnchecked()) == null);
    var invalid = identity;
    invalid.cols[0].x = std.math.inf(f32);
    try std.testing.expect(picking.Ray.fromLogicalPointer(pointer, viewport, invalid) == null);
    var epsilon = identity;
    epsilon.cols[3].w = std.math.floatEps(f32);
    try std.testing.expect(picking.Ray.fromLogicalPointer(pointer, viewport, epsilon) == null);
    epsilon.cols[3].w = -std.math.floatEps(f32);
    try std.testing.expect(picking.Ray.fromLogicalPointer(pointer, viewport, epsilon) == null);
    epsilon.cols[3].w = 2 * std.math.floatEps(f32);
    epsilon.cols[3].x = std.math.floatMax(f32);
    try std.testing.expect(picking.Ray.fromLogicalPointer(pointer, viewport, epsilon) == null);
    const huge = math.Mat4.fromColumns(.{ vec(1e10, 0, 0, 0), vec(0, 1e10, 0, 0), vec(0, 0, 1e10, 0), vec(0, 0, 0, 1e10) });
    try std.testing.expect(huge.inverseUnchecked().isFinite());
    try std.testing.expect(picking.Ray.fromLogicalPointer(pointer, viewport, huge.inverseUnchecked()) == null);
}
test "classic picking sphere nearest tangent inside behind and exact zero roots" {
    try std.testing.expectEqual(@as(?f32, 4), forward.sphereDistance(math.Vec3.zero, 1));
    try std.testing.expectEqual(@as(?f32, 5), forward.sphereDistance(.{ .x = 1, .y = 0, .z = 0 }, 1));
    try std.testing.expectEqual(@as(?f32, null), forward.sphereDistance(.{ .x = 2, .y = 0, .z = 0 }, 1));
    try std.testing.expectEqual(@as(?f32, null), forward.sphereDistance(.{ .x = 0, .y = 0, .z = 8 }, 1));
    const inside = picking.Ray{ .origin = math.Vec3.zero, .direction = .{ .x = 0, .y = 0, .z = 1 } };
    try std.testing.expectEqual(@as(?f32, 1), inside.sphereDistance(math.Vec3.zero, 1));
    const surface_in = picking.Ray{ .origin = .{ .x = 0, .y = 0, .z = 1 }, .direction = .{ .x = 0, .y = 0, .z = -1 } };
    const surface_out = picking.Ray{ .origin = .{ .x = 0, .y = 0, .z = 1 }, .direction = .{ .x = 0, .y = 0, .z = 1 } };
    try std.testing.expectEqual(@as(?f32, 2), surface_in.sphereDistance(math.Vec3.zero, 1));
    try std.testing.expectEqual(@as(?f32, null), surface_out.sphereDistance(math.Vec3.zero, 1));
    const tangent_zero = picking.Ray{ .origin = .{ .x = 1, .y = 0, .z = 0 }, .direction = .{ .x = 0, .y = 0, .z = 1 } };
    try std.testing.expectEqual(@as(?f32, null), tangent_zero.sphereDistance(math.Vec3.zero, 1));
}
test "classic picking sphere finite input admission preserves raw direction and overflow" {
    for ([_]f32{ 0, -1, std.math.inf(f32), std.math.nan(f32) }) |radius|
        try std.testing.expect(forward.sphereDistance(math.Vec3.zero, radius) == null);
    try std.testing.expect(forward.sphereDistance(.{ .x = std.math.nan(f32), .y = 0, .z = 0 }, 1) == null);
    var invalid = forward;
    invalid.origin.x = std.math.inf(f32);
    try std.testing.expect(invalid.sphereDistance(math.Vec3.zero, 1) == null);
    invalid = forward;
    invalid.direction.z = std.math.nan(f32);
    try std.testing.expect(invalid.sphereDistance(math.Vec3.zero, 1) == null);
    // Source assumes a unit ray without imposing admission on public raw rays.
    const stationary = picking.Ray{ .origin = math.Vec3.zero, .direction = math.Vec3.zero };
    try std.testing.expectEqual(@as(?f32, 1), stationary.sphereDistance(math.Vec3.zero, 1));
    const doubled = picking.Ray{ .origin = .{ .x = 0, .y = 0, .z = 5 }, .direction = .{ .x = 0, .y = 0, .z = -2 } };
    try std.testing.expectApproxEqAbs(@as(f32, 10 - @sqrt(@as(f32, 76))), doubled.sphereDistance(math.Vec3.zero, 1).?, 1e-6);
    // Finite radius overflow produces a positive-infinite far root; retain it.
    try std.testing.expectEqual(@as(?f32, std.math.inf(f32)), stationary.sphereDistance(math.Vec3.zero, 1e30));
    const extreme = picking.Ray{ .origin = .{ .x = 1e30, .y = 0, .z = 0 }, .direction = .{ .x = -1, .y = 0, .z = 0 } };
    try std.testing.expect(extreme.sphereDistance(math.Vec3.zero, 1) == null);
}
test "classic picking nearest world total order then numeric ID and HUD precedence" {
    const worlds = [_]picking.PickWorld{ .{ .id = 4, .position = math.Vec3.zero, .radius = 1 }, .{ .id = 2, .position = math.Vec3.zero, .radius = 1 }, .{ .id = 1, .position = .{ .x = 0, .y = 0, .z = -3 }, .radius = 1 } };
    try std.testing.expect(picking.pickWorld(forward, &worlds) != null);
    const hit = picking.pickWorld(forward, &worlds).?;
    try std.testing.expectEqual(@as(u8, 2), hit.world);
    try std.testing.expectApproxEqAbs(@as(f32, 4), hit.distance, std.math.floatEps(f32));
    const reverse = [_]picking.PickWorld{ worlds[2], worlds[1], worlds[0] };
    try std.testing.expectEqualDeep(hit, picking.pickWorld(forward, &reverse).?);
    try std.testing.expect(picking.pickAfterHud(true, forward, &worlds) == null);
    try std.testing.expectEqualDeep(hit, picking.pickAfterHud(false, forward, &worlds).?);
    try std.testing.expect(picking.pickWorld(forward, &.{}) == null);
    const overflow = [_]picking.PickWorld{ .{ .id = 4, .position = math.Vec3.zero, .radius = 1e30 }, .{ .id = 2, .position = math.Vec3.zero, .radius = 1e30 } };
    try std.testing.expectEqual(@as(u8, 2), picking.pickWorld(forward, &overflow).?.world);
}
