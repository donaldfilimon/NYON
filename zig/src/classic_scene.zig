//! Synchronous allocation-free presentation extraction. Inputs borrow fleets for
//! this call only; frames own all populated values. No render/GPU ABI contract.
const std = @import("std");
const m = @import("model.zig");
const math = @import("classic_math.zig");
const camera = @import("classic_camera.zig");
const prefs = @import("preferences.zig");
const scenario = @import("scenario.zig");
const advisory = @import("classic_advisory_controller.zig");
pub const MAX_SCENE_FLEETS = 512;
pub const MAX_SCENE_ROUTES = 256;
pub const HIGH_PARTICLE_BUDGET: u32 = 512;
pub const LOW_PARTICLE_BUDGET: u32 = 96;
/// Field-copy CampaignView.worlds/fleets/active_hazard here. Finish extraction
/// before authority mutation, restart, move or deinit. No AppCore dependency.
pub const SceneCampaign = struct { worlds: [m.WORLD_COUNT]m.World, fleets: []const m.Fleet, active_hazard: ?m.ActiveHazard };
pub const SceneView = struct { selected_world: ?m.WorldId = null, hovered_world: ?m.WorldId = null };
pub const SceneWorld = struct { id: m.WorldId, position: math.Vec3, radius: f32, ownership_color: [4]f32, ownership_pattern: u32, fields: [3]f32, energy: f32, defense: f32, hovered: bool, selected: bool, hazard: ?m.HazardKind, hazard_field: ?m.FieldKind };
pub const SceneRoute = struct { source: m.WorldId, destination: m.WorldId, faction: m.Faction, start: math.Vec3, control: math.Vec3, end: math.Vec3 };
pub const SceneFleet = struct { id: m.FleetId, faction: m.Faction, source: m.WorldId, destination: m.WorldId, position: math.Vec3, strength: f32, progress: f32 };
pub const SceneBackground = struct { visual_seed: u64, parallax: math.Vec2, static_background: bool };
pub const SceneEffects = struct { particle_budget: u32, animate_halos: bool, bloom_requested: bool, high_contrast: bool };
pub const SceneAdvisory = struct {
    priority: m.WorldId,
    score: f32,
    /// F3 owns priority admission, freshness and score publication.
    pub fn fromSnapshot(snapshot: *const advisory.Snapshot) SceneAdvisory {
        return .{ .priority = snapshot.priority, .score = snapshot.scores[snapshot.priority] };
    }
};
pub const SceneFrame = struct {
    worlds: [m.WORLD_COUNT]SceneWorld,
    fleet_storage: [MAX_SCENE_FLEETS]SceneFleet,
    route_storage: [MAX_SCENE_ROUTES]SceneRoute,
    fleet_count: usize,
    route_count: usize,
    background: SceneBackground,
    effects: SceneEffects,
    advisory: ?SceneAdvisory,
    camera: camera.CameraMatrices,
    visual_time: f32,
    /// Borrow these prefixes only from a live, stable frame. Tails undefined.
    pub fn fleets(self: *const SceneFrame) []const SceneFleet {
        return self.fleet_storage[0..self.fleet_count];
    }
    pub fn routes(self: *const SceneFrame) []const SceneRoute {
        return self.route_storage[0..self.route_count];
    }
};
pub fn visualSeed(fingerprint: scenario.ScenarioFingerprint) u64 {
    var value = fingerprint ^ 0x5649_5355_414C_5F31;
    value +%= 0x9E37_79B9_7F4A_7C15;
    value = (value ^ (value >> 30)) *% 0xBF58_476D_1CE4_E5B9;
    value = (value ^ (value >> 27)) *% 0x94D0_49BB_1331_11EB;
    return value ^ (value >> 31);
}
pub fn sectorToCommandPlane(x: i32, y: i32) math.Vec3 {
    @setFloatMode(.strict);
    return .{ .x = @as(f32, @floatFromInt(std.math.clamp(x, 0, 10000))) / 1000 - 5, .y = 0, .z = @as(f32, @floatFromInt(std.math.clamp(y, 0, 10000))) / 1000 - 5 };
}
pub fn routeControl(start: math.Vec3, end: math.Vec3, tag: m.FleetId) math.Vec3 {
    @setFloatMode(.strict);
    const midpoint = math.scale(math.add(start, end), 0.5);
    const lift = 0.55 + math.distance(start, end) * 0.12 + @as(f32, @floatFromInt(tag % 5)) * 0.025;
    return math.add(midpoint, math.scale(.{ .x = 0, .y = 1, .z = 0 }, lift));
}
pub fn quadraticBezier(start: math.Vec3, control: math.Vec3, end: math.Vec3, t: f32) math.Vec3 {
    @setFloatMode(.strict);
    const one_minus = 1 - t;
    return math.add(math.add(math.scale(math.scale(start, one_minus), one_minus), math.scale(control, 2 * one_minus * t)), math.scale(math.scale(end, t), t));
}
fn factionColor(faction: ?m.Faction) [4]f32 {
    return if (faction) |f| switch (f) {
        .union_faction => .{ 0.12, 0.82, 1, 1 },
        .helix => .{ 1, 0.28, 0.12, 1 },
        .choir => .{ 0.76, 0.32, 1, 1 },
    } else .{ 0.48, 0.56, 0.66, 1 };
}
fn factionPattern(faction: ?m.Faction, contrast: bool) u32 {
    return if (!contrast) 0 else if (faction) |f| switch (f) {
        .union_faction => 1,
        .helix => 2,
        .choir => 3,
    } else 4;
}
pub fn buildScene(campaign: SceneCampaign, fingerprint: scenario.ScenarioFingerprint, interpolation_alpha: f32, view: SceneView, advice: ?SceneAdvisory, visual_time: f32, cam: *const camera.CameraState, preferences: prefs.PresentationPreferences) SceneFrame {
    @setFloatMode(.strict);
    const alpha = if (std.math.isFinite(interpolation_alpha)) math.clamp(interpolation_alpha, 0, 1) else 0;
    const reduced = preferences.motion == .reduced;
    const high = preferences.graphics_quality != .low;
    var frame: SceneFrame = .{ .worlds = undefined, .fleet_storage = undefined, .route_storage = undefined, .fleet_count = 0, .route_count = 0, .background = .{ .visual_seed = visualSeed(fingerprint), .parallax = if (reduced) .{ .x = 0, .y = 0 } else .{ .x = @sin(cam.yaw) * 0.025, .y = @cos(cam.pitch) * 0.025 }, .static_background = reduced }, .effects = .{ .particle_budget = if (reduced) 0 else if (high) HIGH_PARTICLE_BUDGET else LOW_PARTICLE_BUDGET, .animate_halos = !reduced, .bloom_requested = high, .high_contrast = preferences.high_contrast }, .advisory = advice, .camera = cam.current_matrices, .visual_time = if (reduced) 0 else math.finiteFloor(visual_time, 0) };
    for (campaign.worlds, 0..) |w, i| {
        const energy = @as(f32, @floatFromInt(@min(w.energy, 250000))) / 250000;
        const defense = @as(f32, @floatFromInt(@min(w.defense, 100000))) / 100000;
        frame.worlds[i] = .{ .id = w.id, .position = sectorToCommandPlane(w.position.x, w.position.y), .radius = 0.31 + energy * 0.10 + defense * 0.06, .ownership_color = factionColor(w.owner), .ownership_pattern = factionPattern(w.owner, preferences.high_contrast), .fields = .{ @as(f32, @floatFromInt(w.fields.atmosphere)) / 10, @as(f32, @floatFromInt(w.fields.hydrosphere)) / 10, @as(f32, @floatFromInt(w.fields.topology)) / 10 }, .energy = energy, .defense = defense, .hovered = view.hovered_world == w.id, .selected = view.selected_world == w.id, .hazard = if (campaign.active_hazard) |h| h.kind else null, .hazard_field = if (campaign.active_hazard) |h| h.affected_field else null };
    }
    // Stable bounded insertion across ALL input fleets, before endpoint filtering.
    var indexes: [MAX_SCENE_FLEETS]usize = undefined;
    var count: usize = 0;
    for (campaign.fleets, 0..) |fleet, index| {
        var at: usize = 0;
        while (at < count and campaign.fleets[indexes[at]].id <= fleet.id) : (at += 1) {}
        if (at == MAX_SCENE_FLEETS) continue;
        const new_count = @min(count + 1, MAX_SCENE_FLEETS);
        var shift = new_count - 1;
        while (shift > at) : (shift -= 1) indexes[shift] = indexes[shift - 1];
        indexes[at] = index;
        count = new_count;
    }
    for (indexes[0..count]) |index| {
        const f = campaign.fleets[index];
        if (f.source >= m.WORLD_COUNT or f.destination >= m.WORLD_COUNT) continue;
        const start = frame.worlds[f.source].position;
        const end = frame.worlds[f.destination].position;
        const denominator = @max(f.route_length, 1);
        const progress = math.clamp(@floatCast(@as(f64, @floatFromInt(@min(f.progress, f.route_length))) / @as(f64, @floatFromInt(denominator))), 0, 1);
        const display = math.clamp(progress + alpha / @as(f32, @floatFromInt(denominator)), 0, 1);
        const control = routeControl(start, end, f.id);
        frame.fleet_storage[frame.fleet_count] = .{ .id = f.id, .faction = f.faction, .source = f.source, .destination = f.destination, .position = quadraticBezier(start, control, end, display), .strength = @max(@as(f32, @floatFromInt(@min(f.strength, 100000))) / 100000, 0.08), .progress = progress };
        frame.fleet_count += 1;
        if (frame.route_count < MAX_SCENE_ROUTES) {
            var duplicate = false;
            for (frame.routes()) |r| {
                if (r.source == f.source and r.destination == f.destination and r.faction == f.faction) {
                    duplicate = true;
                    break;
                }
            }
            if (!duplicate) {
                frame.route_storage[frame.route_count] = .{ .source = f.source, .destination = f.destination, .faction = f.faction, .start = start, .control = control, .end = end };
                frame.route_count += 1;
            }
        }
    }
    return frame;
}
