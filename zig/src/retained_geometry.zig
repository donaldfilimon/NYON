const std = @import("std");

pub const GeometryError = error{ NonFinite, Overflow, InvalidBounds };

/// Logical-pixel bounds. Direct struct construction requires finite lanes and
/// min <= max on both axes; validate externally supplied bounds with fromBounds.
/// Generic geometry owns no viewport, scale or device-pixel policy.
pub const Rect = struct {
    min: [2]f32,
    max: [2]f32,

    pub fn fromXYWH(x: f32, y: f32, w: f32, h: f32) GeometryError!Rect {
        @setFloatMode(.strict);
        for ([_]f32{ x, y, w, h }) |value| {
            if (!std.math.isFinite(value)) return error.NonFinite;
        }
        const max: [2]f32 = .{ x + @max(w, 0), y + @max(h, 0) };
        for (max) |value| {
            if (!std.math.isFinite(value)) return error.Overflow;
        }
        return .{ .min = .{ x, y }, .max = max };
    }

    pub fn fromBounds(min: [2]f32, max: [2]f32) GeometryError!Rect {
        @setFloatMode(.strict);
        for (min, max) |lo, hi| {
            if (!std.math.isFinite(lo) or !std.math.isFinite(hi)) return error.NonFinite;
        }
        if (min[0] > max[0] or min[1] > max[1]) return error.InvalidBounds;
        return .{ .min = min, .max = max };
    }

    pub fn contains(self: Rect, point: [2]f32) bool {
        @setFloatMode(.strict);
        for (point, self.min, self.max) |value, lo, hi| {
            if (!std.math.isFinite(value) or value < lo or value > hi) return false;
        }
        return true;
    }

    pub fn containsRect(self: Rect, other: Rect) bool {
        @setFloatMode(.strict);
        return other.min[0] >= self.min[0] and other.min[1] >= self.min[1] and
            other.max[0] <= self.max[0] and other.max[1] <= self.max[1];
    }

    pub fn intersection(self: Rect, other: Rect) ?Rect {
        @setFloatMode(.strict);
        const result = Rect{
            .min = .{ @max(self.min[0], other.min[0]), @max(self.min[1], other.min[1]) },
            .max = .{ @min(self.max[0], other.max[0]), @min(self.max[1], other.max[1]) },
        };
        if (result.max[0] > result.min[0] and result.max[1] > result.min[1]) return result;
        return null;
    }

    pub fn overlaps(self: Rect, other: Rect) bool {
        @setFloatMode(.strict);
        const result = self.intersection(other) orelse return false;
        return result.max[0] - result.min[0] > @as(f32, 0.01) and
            result.max[1] - result.min[1] > @as(f32, 0.01);
    }
};
