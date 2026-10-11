const std = @import("std");
const t = @import("classic_advisory_types.zig");
const f = @import("classic_advisory_features.zig");
const m = @import("model.zig");
const W1: [4][12]f32 = .{
    .{ -0.60, 0.55, -0.80, 0, 0.35, -0.20, 0.10, 0.10, -0.10, 0.20, -0.60, -0.40 },
    .{ 0.80, -0.80, -0.70, 0.15, 0.15, 0.05, 0, 0, 0, -0.40, 1, -0.20 },
    .{ 0, 0, -0.10, 0.15, 0.55, 0.25, 0.20, 0.20, 0.20, 0, -0.10, -0.15 },
    .{ -0.20, 0.20, -0.35, 0, 0, 0, 0, 0.25, 0, 0.60, -0.35, -0.90 },
};
const B1: [4]f32 = .{ 0.50, 0.10, -0.25, 0.55 };
const W2: [4]f32 = .{ 0.45, 0.25, 0.20, 0.25 };
const B2: f32 = 0.10;
pub fn packedWeights() [57]f32 {
    var result: [57]f32 = undefined;
    for (W1, 0..) |row, i| @memcpy(result[i * 12 ..][0..12], &row);
    @memcpy(result[48..52], &B1);
    @memcpy(result[52..56], &W2);
    result[56] = B2;
    return result;
}
pub fn evaluateFeatures(features: [12]f32) f32 {
    @setFloatMode(.strict);
    var sanitized: [12]f32 = undefined;
    for (features, 0..) |value, i| sanitized[i] = t.unit(value, 0);
    var hidden: [4]f32 = undefined;
    for (&hidden, 0..) |*value, h| {
        var activation: f32 = B1[h];
        for (sanitized, 0..) |feature, i| activation += W1[h][i] * feature;
        value.* = @max(activation, 0);
    }
    var score: f32 = B2;
    for (hidden, W2) |activation, weight| score += activation * weight;
    return t.unit(score, 0);
}
pub fn cpuScores(observation: t.Observation) [7]f32 {
    var scores: [7]f32 = undefined;
    for (&scores, 0..) |*score, i| score.* = evaluateFeatures(f.featuresForWorld(observation, @intCast(i)));
    return scores;
}
pub fn priorityWorld(scores: [7]f32) m.WorldId {
    var best: m.WorldId = 0;
    var value = t.unit(scores[0], 0);
    for (scores[1..], 1..) |score, i| {
        const candidate = t.unit(score, 0);
        if (candidate > value) {
            best = @intCast(i);
            value = candidate;
        }
    }
    return best;
}
pub fn parityMatches(cpu: [7]f32, candidate: [7]f32) bool {
    @setFloatMode(.strict);
    for (cpu, candidate) |a, b| {
        if (!std.math.isFinite(a) or !std.math.isFinite(b)) return false;
        if (!(@abs(a - b) <= @as(f32, 1e-5) + @as(f32, 5e-5) * @max(@abs(a), @abs(b)))) return false;
    }
    return true;
}
pub fn scoresValid(scores: [7]f32) bool {
    for (scores) |score| if (!std.math.isFinite(score) or score < 0 or score > 1) return false;
    return true;
}
