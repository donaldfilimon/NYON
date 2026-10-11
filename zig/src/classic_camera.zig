//! Allocation-free camera owner in logical pixels. No frame-clock or host policy.
const std = @import("std");
const math = @import("classic_math.zig");
const preferences = @import("preferences.zig");
pub const DEFAULT_YAW_RADIANS: f32 = 0;
pub const DEFAULT_PITCH_RADIANS: f32 = math.radians(55);
pub const MIN_PITCH_RADIANS: f32 = math.radians(35);
pub const MAX_PITCH_RADIANS: f32 = math.radians(70);
pub const MIN_YAW_RADIANS: f32 = -math.radians(45);
pub const MAX_YAW_RADIANS: f32 = math.radians(45);
pub const DEFAULT_DISTANCE: f32 = 15;
pub const MIN_DISTANCE: f32 = 8;
pub const MAX_DISTANCE: f32 = 24;
pub const CameraMatrices = struct {
    view: math.Mat4,
    projection: math.Mat4,
    view_projection: math.Mat4,
    inverse_view_projection: math.Mat4,
    pub fn isFinite(m: CameraMatrices) bool {
        return m.view.isFinite() and m.projection.isFinite() and m.view_projection.isFinite() and m.inverse_view_projection.isFinite();
    }
};
pub const CameraState = struct {
    target: math.Vec3,
    yaw: f32,
    pitch: f32,
    distance: f32,
    target_yaw: f32,
    target_pitch: f32,
    target_distance: f32,
    current_matrices: CameraMatrices,
    target_matrices: CameraMatrices,
    logical_viewport: math.Vec2,
    motion: preferences.MotionPreference,
    pub fn init(viewport: math.Vec2) CameraState {
        @setFloatMode(.strict);
        const logical = finiteViewport(viewport);
        const matrices = cameraMatrices(math.Vec3.zero, DEFAULT_YAW_RADIANS, DEFAULT_PITCH_RADIANS, DEFAULT_DISTANCE, logical);
        return .{ .target = math.Vec3.zero, .yaw = DEFAULT_YAW_RADIANS, .pitch = DEFAULT_PITCH_RADIANS, .distance = DEFAULT_DISTANCE, .target_yaw = DEFAULT_YAW_RADIANS, .target_pitch = DEFAULT_PITCH_RADIANS, .target_distance = DEFAULT_DISTANCE, .current_matrices = matrices, .target_matrices = matrices, .logical_viewport = logical, .motion = .full };
    }
    pub fn eye(c: CameraState) math.Vec3 {
        @setFloatMode(.strict);
        return cameraEye(c.target, c.yaw, c.pitch, c.distance);
    }
    pub fn setViewport(c: *CameraState, viewport: math.Vec2) void {
        @setFloatMode(.strict);
        c.logical_viewport = finiteViewport(viewport);
        c.rebuildMatrices();
    }
    pub fn setMotion(c: *CameraState, motion: preferences.MotionPreference) void {
        c.motion = motion;
        if (motion == .reduced) c.snapToTarget();
    }
    pub fn orbit(c: *CameraState, yaw_delta: f32, pitch_delta: f32) void {
        @setFloatMode(.strict);
        if (std.math.isFinite(yaw_delta)) c.target_yaw = math.clamp(c.target_yaw + yaw_delta, MIN_YAW_RADIANS, MAX_YAW_RADIANS);
        if (std.math.isFinite(pitch_delta)) c.target_pitch = math.clamp(c.target_pitch + pitch_delta, MIN_PITCH_RADIANS, MAX_PITCH_RADIANS);
        c.target_matrices = cameraMatrices(c.target, c.target_yaw, c.target_pitch, c.target_distance, c.logical_viewport);
        if (c.motion == .reduced) c.snapToTarget();
    }
    pub fn zoom(c: *CameraState, logical_delta: f32) void {
        @setFloatMode(.strict);
        if (std.math.isFinite(logical_delta)) {
            const factor = @exp(-logical_delta * 0.0015);
            c.target_distance = math.clamp(c.target_distance * factor, MIN_DISTANCE, MAX_DISTANCE);
            c.target_matrices = cameraMatrices(c.target, c.target_yaw, c.target_pitch, c.target_distance, c.logical_viewport);
            if (c.motion == .reduced) c.snapToTarget();
        }
    }
    pub fn reset(c: *CameraState) void {
        c.target_yaw = DEFAULT_YAW_RADIANS;
        c.target_pitch = DEFAULT_PITCH_RADIANS;
        c.target_distance = DEFAULT_DISTANCE;
        c.target_matrices = cameraMatrices(c.target, c.target_yaw, c.target_pitch, c.target_distance, c.logical_viewport);
        if (c.motion == .reduced) c.snapToTarget();
    }
    pub fn update(c: *CameraState, delta_ns: u128) void {
        @setFloatMode(.strict);
        if (c.motion == .reduced) {
            c.snapToTarget();
            return;
        }
        // Duration::as_secs_f32 whole/fraction order; extended u128 saturates
        // camera behavior at the same quarter-second cap, not Rust's range.
        const whole: f32 = @floatFromInt(delta_ns / 1_000_000_000);
        const fraction: f32 = @as(f32, @floatFromInt(delta_ns % 1_000_000_000)) / @as(f32, 1_000_000_000);
        const seconds = math.clamp(whole + fraction, 0, 0.25);
        const blend = 1 - @exp(-12 * seconds);
        c.yaw += (c.target_yaw - c.yaw) * blend;
        c.pitch += (c.target_pitch - c.pitch) * blend;
        c.distance += (c.target_distance - c.distance) * blend;
        c.rebuildMatrices();
    }
    fn snapToTarget(c: *CameraState) void {
        c.yaw = c.target_yaw;
        c.pitch = c.target_pitch;
        c.distance = c.target_distance;
        c.rebuildMatrices();
    }
    fn rebuildMatrices(c: *CameraState) void {
        c.current_matrices = cameraMatrices(c.target, c.yaw, c.pitch, c.distance, c.logical_viewport);
        c.target_matrices = cameraMatrices(c.target, c.target_yaw, c.target_pitch, c.target_distance, c.logical_viewport);
    }
};
fn finiteViewport(v: math.Vec2) math.Vec2 {
    @setFloatMode(.strict);
    return .{ .x = math.finiteFloor(v.x, 1), .y = math.finiteFloor(v.y, 1) };
}
fn cameraEye(target: math.Vec3, yaw: f32, pitch: f32, distance: f32) math.Vec3 {
    @setFloatMode(.strict);
    const horizontal = @cos(pitch) * distance;
    return math.add(target, .{ .x = @sin(yaw) * horizontal, .y = @sin(pitch) * distance, .z = @cos(yaw) * horizontal });
}
fn cameraMatrices(target: math.Vec3, yaw: f32, pitch: f32, distance: f32, viewport: math.Vec2) CameraMatrices {
    @setFloatMode(.strict);
    const view = math.lookAtRH(cameraEye(target, yaw, pitch, distance), target, .{ .x = 0, .y = 1, .z = 0 });
    const aspect = math.clamp(viewport.x / viewport.y, 0.1, 10);
    const projection = math.perspectiveRHZO(math.radians(45), aspect, 0.1, 100);
    const combined = projection.mulMat4(view);
    return .{ .view = view, .projection = projection, .view_projection = combined, .inverse_view_projection = combined.inverseUnchecked() };
}
