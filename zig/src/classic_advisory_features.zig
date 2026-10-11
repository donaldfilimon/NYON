const t = @import("classic_advisory_types.zig");
const m = @import("model.zig");
pub fn featuresForWorld(observation: t.Observation, world_id: m.WorldId) [12]f32 {
    @setFloatMode(.strict);
    if (world_id >= m.WORLD_COUNT) return @splat(0);
    const world = observation.worlds[world_id];
    var union_inbound: u64 = 0;
    var hostile_inbound: u64 = 0;
    for (observation.fleets) |item| {
        if (item.destination != world_id) continue;
        if (item.faction == .union_faction) union_inbound +|= item.strength else hostile_inbound +|= item.strength;
    }
    var minimum: ?f64 = null;
    for (observation.worlds) |candidate| {
        if (candidate.owner != .union_faction) continue;
        const dx = @as(i128, world.position.x) - @as(i128, candidate.position.x);
        const dy = @as(i128, world.position.y) - @as(i128, candidate.position.y);
        const distance: f64 = @floatFromInt(dx * dx + dy * dy);
        minimum = if (minimum) |prior| @min(prior, distance) else distance;
    }
    return .{
        if (world.owner == .union_faction) 1 else 0,
        if (world.owner == .helix or world.owner == .choir) 1 else 0,
        t.unit(@as(f32, @floatFromInt(world.defense)) / 100000, 0),
        t.unit(@as(f32, @floatFromInt(world.energy)) / 250000, 0),
        t.unit(@as(f32, @floatFromInt(world.base_output_per_second)) / 3000, 0),
        t.unit(@as(f32, @floatFromInt(world.base_regeneration_per_second)) / 500, 0),
        adjusted(observation.active_hazard, .atmosphere, world.fields.atmosphere),
        adjusted(observation.active_hazard, .hydrosphere, world.fields.hydrosphere),
        adjusted(observation.active_hazard, .topology, world.fields.topology),
        t.unit(@as(f32, @floatFromInt(union_inbound)) / 100000, 0),
        t.unit(@as(f32, @floatFromInt(hostile_inbound)) / 100000, 0),
        t.unit(@floatCast((minimum orelse 200000000) / 200000000), 1),
    };
}
pub fn packedFeatures(observation: t.Observation) [84]f32 {
    var result: [84]f32 = undefined;
    for (0..7) |i| {
        const row = featuresForWorld(observation, @intCast(i));
        @memcpy(result[i * 12 ..][0..12], &row);
    }
    return result;
}

fn adjusted(hazard: ?m.ActiveHazard, field: m.FieldKind, level: u8) f32 {
    @setFloatMode(.strict);
    const multiplier: f32 = if (hazard) |h| (if (h.affected_field == field) @as(f32, 0.5) else 1) else 1;
    return t.unit(@as(f32, @floatFromInt(level)) * multiplier / 10, 0);
}
