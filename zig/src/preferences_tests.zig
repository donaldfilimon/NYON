//! Pure preferences model/codec regressions; no native or browser store coverage.
const std = @import("std");
const p = @import("preferences.zig");
const allocator = std.testing.allocator;

const canonical_default = "{\"format_version\":1,\"ui_scale_percent\":100,\"motion\":\"FULL\",\"high_contrast\":false,\"graphics_quality\":\"AUTO\",\"onboarding_completed\":false}";

fn changed(comptime old: []const u8, comptime new: []const u8) []const u8 {
    @setEvalBranchQuota(10_000);
    const position = comptime std.mem.indexOf(u8, canonical_default, old).?;
    return canonical_default[0..position] ++ new ++ canonical_default[position + old.len ..];
}

test "conservative defaults preserve every preference and presentation projection" {
    const value: p.UserPreferencesV1 = .{};
    try std.testing.expectEqual(p.UiScale.percent100, value.ui_scale);
    try std.testing.expectEqual(@as(u16, 100), value.ui_scale.percent());
    try std.testing.expectEqual(@as(f32, 1), value.ui_scale.factor());
    try std.testing.expectEqual(p.MotionPreference.full, value.motion);
    try std.testing.expectEqual(p.GraphicsQuality.auto, value.graphics_quality);
    try std.testing.expect(!value.high_contrast and !value.onboarding_completed);
    try std.testing.expectEqualDeep(p.PresentationPreferences{}, value.presentation());
    const full: p.UserPreferencesV1 = .{ .ui_scale = .percent130, .motion = .reduced, .graphics_quality = .low, .high_contrast = true, .onboarding_completed = true };
    try std.testing.expectEqualDeep(p.PresentationPreferences{ .motion = .reduced, .graphics_quality = .low, .high_contrast = true }, full.presentation());
    const scales = [_]p.UiScale{ .percent85, .percent100, .percent115, .percent130 };
    for (scales, [_]u16{ 85, 100, 115, 130 }) |scale, percentage| {
        try std.testing.expectEqual(percentage, scale.percent());
        try std.testing.expectEqual(scale, try p.UiScale.fromPercent(percentage));
        try std.testing.expectEqual(@as(f32, @floatFromInt(percentage)) / 100.0, scale.factor());
    }
    try std.testing.expectError(error.UiScale, p.UiScale.fromPercent(99));
}

test "canonical encoded JSON preserves Rust field order and enum spelling" {
    const encoded = try p.encode(allocator, .{});
    defer allocator.free(encoded);
    try std.testing.expectEqualStrings(canonical_default, encoded);
    try std.testing.expectEqualDeep(p.UserPreferencesV1{}, try p.decode(allocator, canonical_default));
}

test "all 96 supported preference combinations round trip independently" {
    for ([_]p.UiScale{ .percent85, .percent100, .percent115, .percent130 }) |scale| {
        for ([_]p.MotionPreference{ .full, .reduced }) |motion| {
            for ([_]p.GraphicsQuality{ .auto, .low, .high }) |quality| {
                for ([_]bool{ false, true }) |contrast| {
                    for ([_]bool{ false, true }) |onboarding| {
                        const original: p.UserPreferencesV1 = .{ .ui_scale = scale, .motion = motion, .graphics_quality = quality, .high_contrast = contrast, .onboarding_completed = onboarding };
                        const encoded = try p.encode(allocator, original);
                        defer allocator.free(encoded);
                        try std.testing.expect(encoded.len <= p.MAX_JSON_BYTES);
                        try std.testing.expectEqualDeep(original, try p.decode(allocator, encoded));
                    }
                }
            }
        }
    }
}

test "codec caps raw UTF8 bytes before parsing and accepts exact size whitespace padding" {
    const padded = try allocator.alloc(u8, p.MAX_JSON_BYTES + 1);
    defer allocator.free(padded);
    @memcpy(padded[0..canonical_default.len], canonical_default);
    @memset(padded[canonical_default.len..], ' ');
    try std.testing.expectEqualDeep(p.UserPreferencesV1{}, try p.decode(allocator, padded[0..p.MAX_JSON_BYTES]));
    try std.testing.expectError(error.TooLarge, p.decode(allocator, padded));
    @memset(padded, 'X');
    try std.testing.expectError(error.TooLarge, p.decode(allocator, padded));
    try std.testing.expectError(error.Json, p.decode(allocator, padded[0..p.MAX_JSON_BYTES]));
}

test "version and scale failures retain typed classification and validation precedence" {
    try std.testing.expectError(error.Version, p.decode(allocator, changed("\"format_version\":1", "\"format_version\":7")));
    try std.testing.expectError(error.UiScale, p.decode(allocator, changed("\"ui_scale_percent\":100", "\"ui_scale_percent\":101")));
    const outcome = try p.decodeDetailed(allocator, changed("\"ui_scale_percent\":100", "\"ui_scale_percent\":65535"));
    try std.testing.expectEqual(@as(u16, 65535), outcome.failure.ui_scale);
    try std.testing.expectError(error.Version, p.decode(allocator, "[7,99,\"FULL\",false,\"AUTO\",false]"));
    try std.testing.expectError(error.Json, p.decode(allocator, "[7,99,\"invalid\",false,\"AUTO\",false]"));
}

test "required fields unknown fields and duplicate decoded names reject" {
    @setEvalBranchQuota(10_000);
    try std.testing.expectError(error.Json, p.decode(allocator, "{}"));
    try std.testing.expectError(error.Json, p.decode(allocator, "{\"unknown\":true," ++ canonical_default[1..]));
    inline for (.{ "format_version", "ui_scale_percent", "motion", "high_contrast", "graphics_quality", "onboarding_completed" }) |name| {
        const needle = "\"" ++ name ++ "\":";
        const begin = comptime std.mem.indexOf(u8, canonical_default, needle).?;
        const after = comptime (std.mem.indexOfScalarPos(u8, canonical_default, begin, ',') orelse canonical_default.len - 1);
        const field = canonical_default[begin..after];
        const duplicate = "{" ++ field ++ "," ++ canonical_default[1..];
        try std.testing.expectError(error.Json, p.decode(allocator, duplicate));
        const missing = if (begin == 1) "{" ++ canonical_default[after + 1 ..] else canonical_default[0 .. begin - 1] ++ canonical_default[after..];
        try std.testing.expectError(error.Json, p.decode(allocator, missing));
    }
    try std.testing.expectError(error.Json, p.decode(allocator, "{\"format\\u005fversion\":1," ++ canonical_default[1..]));
}

test "wire integer fields reject quoted float exponent negative and out of range inputs" {
    inline for (.{ "\"1\"", "1.0", "1e0", "-0", "-1", "4294967296", "true", "null" }) |number| {
        try std.testing.expectError(error.Json, p.decode(allocator, changed("\"format_version\":1", "\"format_version\":" ++ number)));
    }
    inline for (.{ "\"100\"", "100.0", "1e2", "-0", "-1", "65536", "false", "null" }) |number| {
        try std.testing.expectError(error.Json, p.decode(allocator, changed("\"ui_scale_percent\":100", "\"ui_scale_percent\":" ++ number)));
    }
}

test "wire enums and booleans reject numeric and case coercions" {
    inline for (.{ "0", "1", "\"0\"", "\"Full\"", "\"full\"", "null", "true", "{\"FULL\":true}", "{\"FULL\":null,\"REDUCED\":null}" }) |motion| {
        try std.testing.expectError(error.Json, p.decode(allocator, changed("\"motion\":\"FULL\"", "\"motion\":" ++ motion)));
    }
    inline for (.{ "0", "\"0\"", "\"auto\"", "\"MEDIUM\"", "null" }) |quality| {
        try std.testing.expectError(error.Json, p.decode(allocator, changed("\"graphics_quality\":\"AUTO\"", "\"graphics_quality\":" ++ quality)));
    }
    inline for (.{ "0", "\"false\"", "null" }) |value| {
        try std.testing.expectError(error.Json, p.decode(allocator, changed("\"high_contrast\":false", "\"high_contrast\":" ++ value)));
        try std.testing.expectError(error.Json, p.decode(allocator, changed("\"onboarding_completed\":false", "\"onboarding_completed\":" ++ value)));
    }
}

test "serde-compatible sequences and unit variant objects retain canonical encoding" {
    const sequence = "[1,100,\"FULL\",false,\"AUTO\",false]";
    const enum_objects = "{\"format_version\":1,\"ui_scale_percent\":100,\"motion\":{\"FULL\":null},\"high_contrast\":false,\"graphics_quality\":{\"AUTO\":null},\"onboarding_completed\":false}";
    for ([_][]const u8{ sequence, enum_objects }) |input| {
        const decoded = try p.decode(allocator, input);
        try std.testing.expectEqualDeep(p.UserPreferencesV1{}, decoded);
        const encoded = try p.encode(allocator, decoded);
        defer allocator.free(encoded);
        try std.testing.expectEqualStrings(canonical_default, encoded);
    }
    try std.testing.expectError(error.Json, p.decode(allocator, "[1,100,\"FULL\",false,\"AUTO\"]"));
    try std.testing.expectError(error.Json, p.decode(allocator, "[1,100,\"FULL\",false,\"AUTO\",false,true]"));
}

test "decoded preferences detach from input and accept escaped names and reordered fields" {
    const input = try allocator.dupe(u8, "{\"onboarding_completed\":true,\"graphics_quality\":\"L\\u004fW\",\"high_contrast\":true,\"motion\":\"REDUCED\",\"ui_scale_percent\":130,\"format\\u005fversion\":1}");
    const value = try p.decode(allocator, input);
    @memset(input, 'X');
    allocator.free(input);
    try std.testing.expectEqualDeep(p.UserPreferencesV1{ .ui_scale = .percent130, .motion = .reduced, .high_contrast = true, .graphics_quality = .low, .onboarding_completed = true }, value);
}

test "malformed trailing and invalid UTF8 data reject without partial results" {
    for ([_][]const u8{ "not-json", canonical_default ++ "{}", canonical_default ++ " true", canonical_default[0 .. canonical_default.len - 1] ++ ",}", "{\"motion\":\"\xff\"}", "null", "true" }) |input| {
        try std.testing.expectError(error.Json, p.decode(allocator, input));
    }
}

fn allocationRoundTrip(test_allocator: std.mem.Allocator) !void {
    const encoded = try p.encode(test_allocator, .{});
    defer test_allocator.free(encoded);
    try std.testing.expectEqualDeep(p.UserPreferencesV1{}, try p.decode(test_allocator, encoded));
}

test "encode decode allocation failures remain recoverable and leak free" {
    try std.testing.checkAllAllocationFailures(allocator, allocationRoundTrip, .{});
}
