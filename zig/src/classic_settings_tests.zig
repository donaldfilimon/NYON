const std = @import("std");
const t = std.testing;
const s = @import("classic_settings.zig");
// Catches ordinal drift and stale opposite-value payloads after a live toggle.
test "Classic3E settings literal eleven actions reflect current toggle values" {
    const want = [_]s.Action{
        .{ .set_ui_scale = .percent85 },    .{ .set_ui_scale = .percent100 }, .{ .set_ui_scale = .percent115 },   .{ .set_ui_scale = .percent130 },
        .{ .set_motion = .reduced },        .{ .set_high_contrast = true },   .{ .set_graphics_quality = .auto }, .{ .set_graphics_quality = .low },
        .{ .set_graphics_quality = .high }, .reset_onboarding,                .close,
    };
    const got = s.actions(.{});
    for (want, got) |a, b| try t.expect(std.meta.eql(a, b));
    const toggled = s.actions(.{ .motion = .reduced, .high_contrast = true });
    try t.expect(std.meta.eql(s.Action{ .set_motion = .full }, toggled[4]));
    try t.expect(std.meta.eql(s.Action{ .set_high_contrast = false }, toggled[5]));
}
// Catches out-of-bounds focus, overflow on usize input, and failure to wrap.
test "Classic3E settings focus clamps wraps and survives payload changes" {
    var state: s.State = .{};
    state.focus(std.math.maxInt(usize));
    try t.expectEqual(@as(usize, 10), state.focusIndex());
    state.next();
    try t.expectEqual(@as(usize, 0), state.focusIndex());
    state.previous();
    try t.expectEqual(@as(usize, 10), state.focusIndex());
    for (0..22) |_| state.next();
    try t.expectEqual(@as(usize, 10), state.focusIndex());
    state.focus(4);
    _ = s.actions(.{ .motion = .reduced });
    try t.expectEqual(@as(usize, 4), state.focusIndex());
}
