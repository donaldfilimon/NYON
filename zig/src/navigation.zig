const std = @import("std");
const model = @import("model.zig");

/// Exact floor square root without overflowing an intermediate product.
pub fn integerSqrt(value: u128) u128 {
    if (value < 2) return value;
    var low: u128 = 1;
    var high: u128 = @min(value, @as(u128, std.math.maxInt(u64)));
    high += 1;
    while (low + 1 < high) {
        const middle = low + (high - low) / 2;
        if (middle <= value / middle) {
            low = middle;
        } else {
            high = middle;
        }
    }
    return low;
}

/// RulesV1 route length is floor(sqrt(dx² + dy²)), in sector units.
pub fn routeLength(source: model.SectorPoint, destination: model.SectorPoint) u64 {
    const dx = @as(i128, destination.x) - @as(i128, source.x);
    const dy = @as(i128, destination.y) - @as(i128, source.y);
    return @intCast(integerSqrt(@intCast(dx * dx + dy * dy)));
}

/// The hydrosphere hazard halves the field bonus before the integer speed
/// truncation, retaining the base speed in full.
pub fn fleetSpeed(base_speed: u64, hydrosphere_level: u8, hazard_active: bool) u64 {
    var bonus_basis_points = @as(u64, hydrosphere_level) * 400;
    if (hazard_active) bonus_basis_points = bonus_basis_points * 5_000 / 10_000;
    return @truncate(@as(u128, base_speed) * @as(u128, 10_000 + bonus_basis_points) / 10_000);
}

/// Advance an existing fleet by one fixed tick. The simulation skips fleets
/// launched during this tick before calling this helper and resolves arrivals
/// afterward. Outside a hydrosphere hazard, retain the fleet's launch speed.
pub fn advanceFleet(fleet: *model.Fleet, rules: model.RulesV1, hydrosphere_hazard: bool) void {
    if (rules.tick_hz == 0) @panic("RulesV1.tick_hz must be nonzero");
    const divisor = @as(u128, rules.tick_hz) * 10_000;
    const rate = if (hydrosphere_hazard)
        fleetSpeed(rules.base_fleet_speed, fleet.hydrosphere_level, true)
    else
        fleet.speed_per_second;
    const numerator = @as(u128, fleet.movement_remainder) + @as(u128, rate) * 10_000;
    const gain: u64 = @truncate(numerator / divisor);
    fleet.movement_remainder = @intCast(numerator % divisor);
    fleet.progress +|= gain;
}

pub const ProgressRatio = struct { numerator: u64, denominator: u64 };

pub fn fleetProgressRatio(fleet: *const model.Fleet) ProgressRatio {
    return .{
        .numerator = @min(fleet.progress, fleet.route_length),
        .denominator = @max(fleet.route_length, 1),
    };
}

fn fixture() model.Fleet {
    return .{
        .id = 0,
        .faction = .union_faction,
        .source = 0,
        .destination = 1,
        .strength = 40_000,
        .route_length = 8064,
        .progress = 0,
        .speed_per_second = 29,
        .hydrosphere_level = 7,
        .movement_remainder = 0,
    };
}

test "integer square root floors boundary values across full u128 domain" {
    const roots = [_]u128{ 0, 1, 2, 3, 4, 255, 256, 65_535, 4_294_967_295, std.math.maxInt(u64) };
    for (roots) |root| {
        const squared = root * root;
        try std.testing.expectEqual(root, integerSqrt(squared));
        if (root > 0) try std.testing.expectEqual(root - 1, integerSqrt(squared - 1));
        if (root > 0) try std.testing.expectEqual(root, integerSqrt(squared + 1));
    }
    try std.testing.expectEqual(@as(u128, std.math.maxInt(u64)), integerSqrt(std.math.maxInt(u128)));
}

test "route length retains integer floors and full i32 coordinate differences" {
    try std.testing.expectEqual(@as(u64, 0), routeLength(.{ .x = 1, .y = 2 }, .{ .x = 1, .y = 2 }));
    try std.testing.expectEqual(@as(u64, 5), routeLength(.{ .x = -1, .y = -2 }, .{ .x = 2, .y = 2 }));
    try std.testing.expectEqual(@as(u64, 1), routeLength(.{ .x = 0, .y = 0 }, .{ .x = 1, .y = 1 }));
    const low: model.SectorPoint = .{ .x = std.math.minInt(i32), .y = std.math.minInt(i32) };
    const high: model.SectorPoint = .{ .x = std.math.maxInt(i32), .y = std.math.maxInt(i32) };
    try std.testing.expectEqual(@as(u64, 6_074_000_998), routeLength(low, high));
    try std.testing.expectEqual(routeLength(low, high), routeLength(high, low));
    try std.testing.expectEqual(@as(u64, 8064), routeLength(.{ .x = 1037, .y = 5027 }, .{ .x = 8793, .y = 2818 }));
}

test "fleet speed halves only hydrosphere bonus before truncation" {
    try std.testing.expectEqual(@as(u64, 23), fleetSpeed(23, 0, false));
    try std.testing.expectEqual(@as(u64, 23), fleetSpeed(23, 0, true));
    try std.testing.expectEqual(@as(u64, 29), fleetSpeed(23, 7, false));
    try std.testing.expectEqual(@as(u64, 26), fleetSpeed(23, 7, true));
    try std.testing.expectEqual(@as(u64, 32), fleetSpeed(23, 10, false));
    try std.testing.expectEqual(@as(u64, 27), fleetSpeed(23, 10, true));
    try std.testing.expectEqual(@as(u64, 0), fleetSpeed(0, 255, true));
    try std.testing.expectEqual(std.math.maxInt(u64), fleetSpeed(std.math.maxInt(u64), 0, false));
    try std.testing.expectEqual(@as(u64, 7_378_697_629_483_820_645), fleetSpeed(std.math.maxInt(u64), 10, false));
}

test "sixty fleet ticks retain movement fractions without drift" {
    var fleet = fixture();
    for (0..60) |_| advanceFleet(&fleet, .{}, false);
    try std.testing.expectEqual(@as(u64, 29), fleet.progress);
    try std.testing.expectEqual(@as(u64, 0), fleet.movement_remainder);
}

test "fleet hazard uses frozen launch hydrosphere and rules base speed" {
    var ordinary = fixture();
    ordinary.speed_per_second = 30;
    var hazardous = ordinary;
    for (0..60) |_| {
        advanceFleet(&ordinary, .{}, false);
        advanceFleet(&hazardous, .{}, true);
    }
    try std.testing.expectEqual(@as(u64, 30), ordinary.progress);
    try std.testing.expectEqual(@as(u64, 26), hazardous.progress);
    try std.testing.expectEqual(@as(u64, 30), hazardous.speed_per_second);
    try std.testing.expectEqual(@as(u8, 7), hazardous.hydrosphere_level);
}

test "fleet movement uses wide numerators and saturates progress" {
    var fleet = fixture();
    fleet.speed_per_second = std.math.maxInt(u64);
    advanceFleet(&fleet, .{}, false);
    try std.testing.expectEqual(std.math.maxInt(u64) / 60, fleet.progress);
    try std.testing.expectEqual(@as(u64, 150_000), fleet.movement_remainder);
    fleet.progress = std.math.maxInt(u64);
    advanceFleet(&fleet, .{}, false);
    try std.testing.expectEqual(std.math.maxInt(u64), fleet.progress);
}

test "fleet progress ratio clamps overshoot and prevents zero denominator" {
    var fleet = fixture();
    fleet.progress = fleet.route_length + 10;
    try std.testing.expectEqualDeep(ProgressRatio{ .numerator = 8064, .denominator = 8064 }, fleetProgressRatio(&fleet));
    fleet.route_length = 0;
    try std.testing.expectEqualDeep(ProgressRatio{ .numerator = 0, .denominator = 1 }, fleetProgressRatio(&fleet));
}
