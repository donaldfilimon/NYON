//! Unique owning handle. Do not copy; Setup.take is the ownership transfer.
const std = @import("std");
const t = @import("types.zig");
const storage = @import("storage.zig");
const Self = @This();
allocator: std.mem.Allocator,
output: ?*storage.Output,

pub fn config(self: *const Self) t.Failure!t.Config {
    return (self.output orelse return error.Shutdown).config;
}
pub fn digest(self: *const Self) t.Failure![32]u8 {
    return (self.output orelse return error.Shutdown).digest;
}
fn page(count: u16, start: u16, capacity: usize) t.Failure!t.Page {
    if (start > count or capacity == 0 or capacity > 64) return error.InvalidConfig;
    const written: u8 = @intCast(@min(count - start, capacity));
    const end = start + written;
    return .{ .written = written, .next = if (end < count) end else null };
}
pub fn systems(self: *const Self, start: u16, out: []t.System) t.Failure!t.Page {
    const data = self.output orelse return error.Shutdown;
    const result = try page(data.config.systems, start, out.len);
    @memcpy(out[0..result.written], data.systems[start..][0..result.written]);
    return result;
}
pub fn planets(self: *const Self, start: u16, out: []t.Planet) t.Failure!t.Page {
    const data = self.output orelse return error.Shutdown;
    const result = try page(data.config.planets, start, out.len);
    @memcpy(out[0..result.written], data.planets[start..][0..result.written]);
    return result;
}
pub fn civilizations(self: *const Self, start: u16, out: []t.Civilization) t.Failure!t.Page {
    const data = self.output orelse return error.Shutdown;
    const result = try page(data.config.civilizations, start, out.len);
    @memcpy(out[0..result.written], data.civilizations[start..][0..result.written]);
    return result;
}
pub fn wireRecords(self: *const Self, start: u16, out: []t.WireRecord) t.Failure!t.Page {
    const data = self.output orelse return error.Shutdown;
    const c = data.config;
    const result = try page(1 + c.systems + c.planets + c.civilizations, start, out.len);
    for (out[0..result.written], start..) |*record, index| {
        const offset: usize, const len: u16 = if (index == 0) .{ 0, 47 } else if (index <= c.systems)
            .{ 47 + (index - 1) * 16, 16 }
        else if (index <= @as(usize, c.systems) + c.planets)
            .{ 47 + @as(usize, c.systems) * 16 + (index - 1 - c.systems) * 20, 20 }
        else
            .{ 47 + @as(usize, c.systems) * 16 + @as(usize, c.planets) * 20 + (index - 1 - c.systems - c.planets) * 9, 9 };
        record.* = .{ .len = len, .bytes = @splat(0) };
        @memcpy(record.bytes[0..len], data.wire[offset..][0..len]);
    }
    return result;
}
pub fn deinit(self: *Self) void {
    if (self.output) |output| self.allocator.destroy(output);
    self.output = null;
}
