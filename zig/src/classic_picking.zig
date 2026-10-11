//! Immutable geometry projection only. No campaign admission or input mutation.
const std = @import("std");
const math = @import("classic_math.zig");
const model = @import("model.zig");
pub const Ray = struct {
    origin: math.Vec3,
    direction: math.Vec3,
    pub fn fromLogicalPointer(pointer: math.Vec2, viewport: math.Vec2, inverse: math.Mat4) ?Ray {
        @setFloatMode(.strict);
        if (!pointer.isFinite() or !viewport.isFinite() or viewport.x <= 0 or viewport.y <= 0) return null;
        const x = pointer.x * 2 / viewport.x - 1;
        const y = 1 - pointer.y * 2 / viewport.y;
        const near = unproject(inverse, .{ .x = x, .y = y, .z = 0 }) orelse return null;
        const far = unproject(inverse, .{ .x = x, .y = y, .z = 1 }) orelse return null;
        const direction = math.tryNormalize(math.sub(far, near)) orelse return null;
        return .{ .origin = near, .direction = direction };
    }
    pub fn sphereDistance(ray: Ray, center: math.Vec3, radius: f32) ?f32 {
        @setFloatMode(.strict);
        if (!ray.origin.isFinite() or !ray.direction.isFinite() or !center.isFinite() or !std.math.isFinite(radius) or radius <= 0) return null;
        const offset = math.sub(ray.origin, center);
        const half_b = math.dot3(offset, ray.direction);
        const c = math.lengthSquared(offset) - radius * radius;
        const discriminant = half_b * half_b - c;
        if (discriminant < 0) return null;
        const root = @sqrt(discriminant);
        const near = -half_b - root;
        const far = -half_b + root;
        return if (near > 0) near else if (far > 0) far else null;
    }
};
pub const PickWorld = struct { id: model.WorldId, position: math.Vec3, radius: f32 };
pub const WorldHit = struct { world: model.WorldId, distance: f32 };
pub fn pickWorld(ray: Ray, worlds: []const PickWorld) ?WorldHit {
    var best: ?WorldHit = null;
    for (worlds) |world| {
        const distance = ray.sphereDistance(world.position, world.radius) orelse continue;
        const candidate = WorldHit{ .world = world.id, .distance = distance };
        if (best) |prior| {
            const order = math.totalOrder(distance, prior.distance);
            if (order == .lt or (order == .eq and world.id < prior.world)) best = candidate;
        } else best = candidate;
    }
    return best;
}
pub fn pickAfterHud(consumed: bool, ray: Ray, worlds: []const PickWorld) ?WorldHit {
    return if (consumed) null else pickWorld(ray, worlds);
}
fn unproject(inverse: math.Mat4, point: math.Vec3) ?math.Vec3 {
    @setFloatMode(.strict);
    const h = inverse.mulVec4(.{ .x = point.x, .y = point.y, .z = point.z, .w = 1 });
    if (!h.isFinite() or @abs(h.w) <= std.math.floatEps(f32)) return null;
    const world: math.Vec3 = .{ .x = h.x / h.w, .y = h.y / h.w, .z = h.z / h.w };
    return if (world.isFinite()) world else null;
}
