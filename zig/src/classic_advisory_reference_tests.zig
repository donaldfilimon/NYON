const std = @import("std");
const m = @import("model.zig");
const g = @import("generator.zig");
const t = @import("classic_advisory_types.zig");
const f = @import("classic_advisory_features.zig");
fn observation() t.Observation {
    const c = g.generate(m.DEFAULT_SEED, .{});
    return .{ .next_tick = c.next_tick, .worlds = c.worlds, .fleets = &.{}, .active_hazard = c.active_hazard };
}
fn fleet(faction: m.Faction, destination: u8, strength: u64) m.Fleet {
    return .{ .id = 0, .faction = faction, .source = 6, .destination = destination, .strength = strength, .route_length = 1, .progress = 0, .speed_per_second = 1, .hydrosphere_level = 0, .movement_remainder = 0 };
}
test "advisory source twelve feature fixture" {
    var o = observation();
    o.worlds[0].defense = 50000;
    o.worlds[0].energy = 125000;
    o.worlds[0].base_output_per_second = 1500;
    o.worlds[0].base_regeneration_per_second = 250;
    o.worlds[0].fields = .init(8, 6, 4);
    o.active_hazard = .{ .event_index = 0, .kind = .ion_storm, .affected_field = .hydrosphere, .start = 0, .end = 10 };
    const fleets = [_]m.Fleet{ fleet(.union_faction, 0, 10000), fleet(.union_faction, 0, 15000), fleet(.helix, 0, 20000), fleet(.choir, 0, 30000) };
    o.fleets = &fleets;
    const expected = [_]f32{ 1, 0, 0.5, 0.5, 0.5, 0.5, 0.8, 0.3, 0.4, 0.25, 0.5, 0 };
    try std.testing.expectEqualSlices(f32, &expected, &f.featuresForWorld(o, 0));
}
const neural = @import("classic_advisory_model.zig");
test "advisory factory independent Rust source score bits" {
    const expected = [_]u32{ 1058173887, 1060329379, 1059226841, 1054688186, 1056209868, 1054665716, 1057420253 };
    for (neural.cpuScores(observation()), expected) |score, bits| try std.testing.expectEqual(bits, @as(u32, @bitCast(score)));
}
// Literal source weight array, independent of the production packing loops.
test "advisory all weight bits and offsets" {
    const expected = [_]f32{ -0.60, 0.55, -0.80, 0, 0.35, -0.20, 0.10, 0.10, -0.10, 0.20, -0.60, -0.40, 0.80, -0.80, -0.70, 0.15, 0.15, 0.05, 0, 0, 0, -0.40, 1, -0.20, 0, 0, -0.10, 0.15, 0.55, 0.25, 0.20, 0.20, 0.20, 0, -0.10, -0.15, -0.20, 0.20, -0.35, 0, 0, 0, 0, 0.25, 0, 0.60, -0.35, -0.90, 0.50, 0.10, -0.25, 0.55, 0.45, 0.25, 0.20, 0.25, 0.10 };
    for (neural.packedWeights(), expected) |actual, value| try std.testing.expectEqual(@as(u32, @bitCast(value)), @as(u32, @bitCast(actual)));
    try std.testing.expectEqual(@as(usize, 336), @sizeOf([t.PACKED_FEATURE_COUNT]f32));
    try std.testing.expectEqual(@as(usize, 228), @sizeOf([t.PACKED_WEIGHT_COUNT]f32));
    try std.testing.expectEqual(@as(usize, 28), @sizeOf([t.SCORE_COUNT]f32));
    try std.testing.expectEqual(@as(u32, 1), t.MODEL_VERSION);
    try std.testing.expectEqual(@as(u64, 30), t.CADENCE_TICKS);
}
test "advisory literal seven packing rows and array indexing" {
    var o = observation();
    for (&o.worlds, 0..) |*w, i| {
        w.id = 255; // extraction indexes array; never searches World.id
        w.owner = null;
        w.defense = @as(u64, @intCast(i)) * 10000;
        w.energy = 0;
        w.base_output_per_second = 0;
        w.base_regeneration_per_second = 0;
        w.fields = .init(0, 0, 0);
    }
    const expected = [_]f32{
        0, 0, 0,   0, 0, 0, 0, 0, 0, 0, 0, 1,
        0, 0, 0.1, 0, 0, 0, 0, 0, 0, 0, 0, 1,
        0, 0, 0.2, 0, 0, 0, 0, 0, 0, 0, 0, 1,
        0, 0, 0.3, 0, 0, 0, 0, 0, 0, 0, 0, 1,
        0, 0, 0.4, 0, 0, 0, 0, 0, 0, 0, 0, 1,
        0, 0, 0.5, 0, 0, 0, 0, 0, 0, 0, 0, 1,
        0, 0, 0.6, 0, 0, 0, 0, 0, 0, 0, 0, 1,
    };
    try std.testing.expectEqualSlices(f32, &expected, &f.packedFeatures(o));
    for ([_]u8{ 7, 255 }) |id| try std.testing.expectEqual(@as([12]f32, @splat(0)), f.featuresForWorld(o, id));
}
test "advisory owners hazard fields and literal resource clamps" {
    var o = observation();
    for ([_]?m.Faction{ null, .union_faction, .helix, .choir }, [_][2]f32{ .{ 0, 0 }, .{ 1, 0 }, .{ 0, 1 }, .{ 0, 1 } }) |owner, expected| {
        o.worlds[0].owner = owner;
        const values = f.featuresForWorld(o, 0);
        try std.testing.expectEqualSlices(f32, &expected, values[0..2]);
    }
    o.worlds[0].fields = .init(11, 11, 11);
    for ([_]m.HazardKind{ .ion_storm, .gravity_tide }) |kind| {
        for ([_]m.FieldKind{ .atmosphere, .hydrosphere, .topology }, 0..) |field, index| {
            o.active_hazard = .{ .event_index = 99, .kind = kind, .affected_field = field, .start = 999, .end = 0 };
            const values = f.featuresForWorld(o, 0);
            for (values[6..9], 0..) |v, i| try std.testing.expectEqual(@as(f32, if (i == index) 0.55 else 1), v);
        }
    }
    o.worlds[0].defense = std.math.maxInt(u64);
    o.worlds[0].energy = std.math.maxInt(u64);
    o.worlds[0].base_output_per_second = 3001;
    o.worlds[0].base_regeneration_per_second = 501;
    const values = f.featuresForWorld(o, 0);
    try std.testing.expectEqualSlices(f32, &.{ 1, 1, 1, 1 }, values[2..6]);
}
test "advisory distance no Union own Union and widened extremes" {
    var o = observation();
    for (&o.worlds) |*w| w.owner = null;
    o.worlds[0].position = .{ .x = 10000, .y = 0 };
    try std.testing.expectEqual(@as(f32, 1), f.featuresForWorld(o, 0)[11]);
    o.worlds[1].owner = .union_faction;
    o.worlds[1].position = .{ .x = 0, .y = 0 };
    try std.testing.expectEqual(@as(f32, 0.5), f.featuresForWorld(o, 0)[11]);
    try std.testing.expectEqual(@as(f32, 0), f.featuresForWorld(o, 1)[11]);
    o.worlds[0].position = .{ .x = std.math.minInt(i32), .y = std.math.minInt(i32) };
    o.worlds[1].position = .{ .x = std.math.maxInt(i32), .y = std.math.maxInt(i32) };
    try std.testing.expectEqual(@as(f32, 1), f.featuresForWorld(o, 0)[11]);
}
test "advisory complete fleets beyond scene capacity independent saturation" {
    var o = observation();
    var fleets: [514]m.Fleet = undefined;
    for (fleets[0..513]) |*item| item.* = fleet(.union_faction, 0, 100);
    fleets[513] = fleet(.union_faction, 1, std.math.maxInt(u64));
    o.fleets = &fleets;
    try std.testing.expectEqual(@as(f32, 0.513), f.featuresForWorld(o, 0)[9]);
    const saturated = [_]m.Fleet{ fleet(.union_faction, 0, std.math.maxInt(u64)), fleet(.union_faction, 0, 1), fleet(.helix, 0, std.math.maxInt(u64)), fleet(.choir, 0, 1) };
    o.fleets = &saturated;
    const values = f.featuresForWorld(o, 0);
    try std.testing.expectEqualSlices(f32, &.{ 1, 1 }, values[9..11]);
}
test "advisory sanitized evaluator inputs priority validity and separate parity" {
    const nan = std.math.nan(f32);
    const inf = std.math.inf(f32);
    try std.testing.expectEqual(@as(f32, 0), t.unit(nan, 0));
    try std.testing.expectEqual(@as(f32, 1), t.unit(inf, 1));
    try std.testing.expectEqual(@as(f32, 0), t.unit(-1, 1));
    try std.testing.expectEqual(@as(f32, 1), t.unit(2, 0));
    const zeros: [12]f32 = @splat(0);
    // Independent real-number fold of biases: .1+.5*.45+.1*.25+.55*.25=.4875.
    try std.testing.expectApproxEqAbs(@as(f32, 0.4875), neural.evaluateFeatures(zeros), 0.000001);
    const ones: [12]f32 = @splat(1);
    try std.testing.expectEqual(neural.evaluateFeatures(zeros), neural.evaluateFeatures(@splat(nan)));
    try std.testing.expectEqual(neural.evaluateFeatures(zeros), neural.evaluateFeatures(@splat(inf)));
    try std.testing.expectEqual(neural.evaluateFeatures(zeros), neural.evaluateFeatures(@splat(-1)));
    try std.testing.expectEqual(neural.evaluateFeatures(ones), neural.evaluateFeatures(@splat(2)));
    try std.testing.expectEqual(@as(u8, 0), neural.priorityWorld(@splat(0.5)));
    try std.testing.expectEqual(@as(u8, 4), neural.priorityWorld(.{ nan, 0.5, 0.5, -inf, 2, 1, -1 }));
    try std.testing.expect(neural.scoresValid(.{ 0, 1, 0, 1, 0, 1, 0 }));
    for ([_]f32{ nan, inf, -inf, -0.01, 1.01 }) |invalid| {
        var scores: [7]f32 = @splat(0.5);
        scores[6] = invalid;
        try std.testing.expect(!neural.scoresValid(scores));
        if (!std.math.isFinite(invalid)) try std.testing.expect(!neural.parityMatches(scores, scores));
    }
    try std.testing.expect(neural.parityMatches(@splat(0.5), @splat(0.50002)));
    try std.testing.expect(neural.parityMatches(@splat(0.5), @splat(0.500034)));
    try std.testing.expect(!neural.parityMatches(@splat(0.5), @splat(0.500036)));
    try std.testing.expect(!neural.parityMatches(@splat(0.5), @splat(0.6)));
    try std.testing.expect(neural.parityMatches(@splat(2), @splat(2))); // no range policy in parity
    const cpu: [7]f32 = .{ 0.50001, 0.5, 0, 0, 0, 0, 0 };
    const gpu: [7]f32 = .{ 0.5, 0.50001, 0, 0, 0, 0, 0 };
    try std.testing.expect(neural.parityMatches(cpu, gpu));
    try std.testing.expect(neural.priorityWorld(cpu) != neural.priorityWorld(gpu));
    // Independently derive output clamp: vector [0,1,0,0,1,0,1,1,1,0,0,0]
    // hidden = [1.5,0,.9,1]; output=.1+.675+0+.18+.25=1.205 ->1.
    try std.testing.expectEqual(@as(f32, 1), neural.evaluateFeatures(.{ 0, 1, 0, 0, 1, 0, 1, 1, 1, 0, 0, 0 }));
}
test "advisory authority queue counters isolation owned results survive replacement" {
    const sim = @import("simulation.zig");
    var authority = sim.Simulation.generated(std.testing.allocator, m.DEFAULT_SEED, .{});
    defer authority.deinit();
    try authority.enqueueNext(42, .{ .launch = .{ .source = 0, .destination = 3 } });
    try authority.enqueueNext(43, .{ .tune_field = .{ .world = 0, .field = .topology, .adjustment = .increase } });
    try authority.state.fleets.append(std.testing.allocator, fleet(.union_faction, 0, 25000));
    const before = authority.canonicalFingerprint();
    const pending = [_]m.CommandEnvelope{ authority.pending.items[0], authority.pending.items[1] };
    const seed = authority.state.seed;
    const hazard = authority.state.next_hazard_index;
    const fleet_id = authority.state.next_fleet_id;
    const o = t.Observation{ .next_tick = authority.state.next_tick, .worlds = authority.state.worlds, .fleets = authority.state.fleets.items, .active_hazard = authority.state.active_hazard };
    const feature_values = f.packedFeatures(o);
    const scores = neural.cpuScores(o);
    const packed_copy = feature_values;
    const scores_copy = scores;
    try std.testing.expectEqual(before, authority.canonicalFingerprint());
    try std.testing.expectEqualDeep(pending, authority.pending.items[0..2].*);
    try std.testing.expectEqual(seed, authority.state.seed);
    try std.testing.expectEqual(hazard, authority.state.next_hazard_index);
    try std.testing.expectEqual(fleet_id, authority.state.next_fleet_id);
    var moved = authority;
    authority = undefined;
    moved.deinit();
    authority = sim.Simulation.generated(std.testing.allocator, 123, .{});
    try std.testing.expectEqualSlices(f32, &packed_copy, &feature_values);
    try std.testing.expectEqualSlices(f32, &scores_copy, &scores);
}
