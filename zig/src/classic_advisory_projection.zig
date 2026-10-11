//! Fixed value panel data. Text layout/percent formatting belong to the UI owner.
const m = @import("model.zig");
const advisory = @import("classic_advisory_controller.zig");
pub const Panel = struct {
    title: []const u8 = "ADVISORY ONLY",
    initializing: bool,
    priority_name: ?m.WorldName = null,
    priority: ?m.WorldId = null,
    scores: ?[7]f32 = null,
    priority_score: ?f32 = null,
    source_tick: m.Tick = 0,
    backend_label: ?[]const u8 = null,
    state_label: ?[]const u8 = null,
};
pub fn project(worlds: []const m.World, snapshot: ?*const advisory.Snapshot, updating: bool) Panel {
    const value = snapshot orelse return .{ .initializing = true };
    var name = m.WorldName.init("UNKNOWN") catch unreachable;
    for (worlds) |world| if (world.id == value.priority) {
        name = world.name;
        break;
    };
    return .{ .initializing = false, .priority_name = name, .priority = value.priority, .scores = value.scores, .priority_score = value.scores[value.priority], .source_tick = value.metadata.source_tick, .backend_label = switch (value.backend) {
        .Cpu => "CPU",
        .WebGpu => "WEBGPU",
        .CpuFallback => "CPU FALLBACK",
    }, .state_label = if (updating) "UPDATING" else "CURRENT" };
}
