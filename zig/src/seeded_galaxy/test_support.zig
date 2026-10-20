const sg = @import("root.zig");
pub fn collectWire(manifest: *const sg.Manifest, out: []u8) sg.Failure!u16 {
    var row: [1]sg.WireRecord = undefined;
    var cursor: u16 = 0;
    var written: usize = 0;
    while (true) {
        const page = try manifest.wireRecords(cursor, &row);
        if (page.written == 1) {
            const len: usize = row[0].len;
            if (written > out.len or len > out.len - written) return error.InvalidConfig;
            @memcpy(out[written..][0..len], row[0].bytes[0..len]);
            written += len;
        }
        cursor = page.next orelse return @intCast(written);
    }
}
pub fn finish(setup: *sg.Setup, token: sg.Token, budget: u16) sg.Failure!void {
    if (budget == 0) return error.InvalidBudget;
    for (0..10_000) |_| switch (try setup.poll(token, budget)) {
        .pending => |stats| if (stats.units_used > budget) return error.InvariantViolation,
        .ready => return,
        .failed => |cause| return cause,
        .canceled => return error.NotReady,
    };
    return error.InvariantViolation;
}

const std = @import("std");
/// Counts attempts, including rejected requests, around the allocator under test.
pub const Counter = struct {
    child: std.mem.Allocator,
    attempts: usize = 0,
    frees: usize = 0,
    live: usize = 0,
    peak: usize = 0,
    pub fn init(child: std.mem.Allocator) Counter {
        return .{ .child = child };
    }
    pub fn allocator(self: *Counter) std.mem.Allocator {
        return .{ .ptr = self, .vtable = &.{ .alloc = alloc, .resize = resize, .remap = remap, .free = free } };
    }
    fn alloc(ctx: *anyopaque, len: usize, alignment: std.mem.Alignment, ra: usize) ?[*]u8 {
        const self: *Counter = @ptrCast(@alignCast(ctx));
        self.attempts += 1;
        const ptr = self.child.rawAlloc(len, alignment, ra) orelse return null;
        self.live += len;
        self.peak = @max(self.peak, self.live);
        return ptr;
    }
    fn resize(ctx: *anyopaque, memory: []u8, alignment: std.mem.Alignment, len: usize, ra: usize) bool {
        const self: *Counter = @ptrCast(@alignCast(ctx));
        self.attempts += 1;
        if (!self.child.rawResize(memory, alignment, len, ra)) return false;
        self.live = self.live - memory.len + len;
        self.peak = @max(self.peak, self.live);
        return true;
    }
    fn remap(ctx: *anyopaque, memory: []u8, alignment: std.mem.Alignment, len: usize, ra: usize) ?[*]u8 {
        const self: *Counter = @ptrCast(@alignCast(ctx));
        self.attempts += 1;
        const ptr = self.child.rawRemap(memory, alignment, len, ra) orelse return null;
        self.live = self.live - memory.len + len;
        self.peak = @max(self.peak, self.live);
        return ptr;
    }
    fn free(ctx: *anyopaque, memory: []u8, alignment: std.mem.Alignment, ra: usize) void {
        const self: *Counter = @ptrCast(@alignCast(ctx));
        self.frees += 1;
        self.live -= memory.len;
        self.child.rawFree(memory, alignment, ra);
    }
};
pub fn pollChecked(setup: *sg.Setup, token: sg.Token, budget: u16, counter: *const Counter) !sg.Progress {
    const attempts = counter.attempts;
    const result = try setup.poll(token, budget);
    try std.testing.expectEqual(attempts, counter.attempts);
    return result;
}
