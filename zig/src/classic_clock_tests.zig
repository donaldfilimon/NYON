const std = @import("std");
const t = std.testing;
const c = @import("classic_clock.zig");

fn drain(clock: *c.FixedClock) usize {
    var count: usize = 0;
    while (clock.takeStep()) count += 1;
    return count;
}

test "Classic clock exact rational carry and source f32 alpha" {
    var clock = try c.FixedClock.init(60, 15);
    clock.pushFrame(16_666_666);
    try t.expect(!clock.takeStep());
    const expected: f32 = @as(f32, @floatFromInt(@as(u128, 999_999_960))) / 1_000_000_000.0;
    try t.expectEqual(@as(u32, @bitCast(expected)), @as(u32, @bitCast(clock.alpha())));
    try t.expectEqual(@as(u32, 0x3f7fffff), @as(u32, @bitCast(clock.alpha())));
    clock.pushFrame(1);
    try t.expect(clock.takeStep());
    try t.expect(!clock.takeStep());
    try t.expectEqual(@as(f32, 20.0 / 1_000_000_000.0), clock.alpha());
}

test "Classic clock clamps delta caps pending drops whole excess and clears" {
    var clock = try c.FixedClock.init(60, 3);
    clock.pushFrame(100_000_001);
    try t.expectEqual(@as(usize, 3), clock.pending_steps);
    try t.expectEqual(@as(u128, 60), clock.scaled_accumulator);
    clock.pushFrame(16_666_667);
    try t.expectEqual(@as(usize, 3), drain(&clock));
    try t.expectEqual(@as(u128, 80), clock.scaled_accumulator);
    try t.expectEqual(@as(usize, 0), drain(&clock));
    clock.pushFrame(2_000_000_000);
    try t.expectEqual(@as(usize, 3), drain(&clock));
    try t.expectEqual(@as(u128, 80), clock.scaled_accumulator);
    clock.pushFrame(100_000_000);
    clock.clear();
    try t.expectEqual(@as(usize, 0), drain(&clock));
    try t.expectEqual(@as(f32, 0), clock.alpha());
}

test "Classic clock all-build invalid configuration zero capacity and integer boundaries" {
    try t.expectError(error.InvalidTickRate, c.FixedClock.init(0, 15));
    try t.expectError(error.InvalidTickRate, c.FixedClock.init(0, 0));
    var zero = try c.FixedClock.init(60, 0);
    zero.pushFrame(100_000_001);
    try t.expectEqual(@as(usize, 0), drain(&zero));
    try t.expectEqual(@as(u128, 60), zero.scaled_accumulator);
    var clock = try c.FixedClock.init(std.math.maxInt(u32), std.math.maxInt(usize));
    clock.pushFrame(std.math.maxInt(u128));
    try t.expectEqual(@as(usize, 1_073_741_823), clock.pending_steps);
    try t.expectEqual(@as(u128, 750_000_000), clock.scaled_accumulator);
    try t.expectEqual(@as(f32, 0.75), clock.alpha());
    clock.clear();
    clock.pushFrame(std.math.maxInt(u64));
    try t.expectEqual(@as(usize, 1_073_741_823), clock.pending_steps);
}
