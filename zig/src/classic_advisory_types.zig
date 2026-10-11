//! Fixed Classic advisory values. Fleets borrow authority only during a synchronous call.
const m = @import("model.zig");
pub const MODEL_VERSION: u32 = 1;
pub const FEATURE_COUNT: usize = 12;
pub const HIDDEN_COUNT: usize = 4;
pub const SCORE_COUNT: usize = m.WORLD_COUNT;
pub const PACKED_FEATURE_COUNT: usize = 84;
pub const PACKED_WEIGHT_COUNT: usize = 57;
pub const CADENCE_TICKS: u64 = 30;
/// Worlds/hazard are copies; fleets must remain valid throughout the call.
/// Never retain this observation across authority mutation, move or deinit.
pub const Observation = struct {
    next_tick: m.Tick,
    worlds: [m.WORLD_COUNT]m.World,
    fleets: []const m.Fleet,
    active_hazard: ?m.ActiveHazard,
};
pub fn unit(value: f32, fallback: f32) f32 {
    const std = @import("std");
    if (!std.math.isFinite(value)) return fallback;
    // Comparisons preserve finite signed zero, matching Rust clamp.
    if (value < 0) return 0;
    if (value > 1) return 1;
    return value;
}
