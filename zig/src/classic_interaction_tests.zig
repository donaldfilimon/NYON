const std = @import("std");
const interaction = @import("classic_interaction.zig");
const Action = @import("classic_session.zig").UiAction;
const C = interaction.InteractionController(Action, 8);
const V = @import("classic_math.zig").Vec2;
fn v(x: f32, y: f32) V {
    return .{ .x = x, .y = y };
}
test "admitted contact retained and release returns source position" {
    var c = C.init();
    try c.begin(42, v(10, 20), .mouse, .primary, .world);
    try std.testing.expectEqual(@as(usize, 1), c.contactCount());
    try std.testing.expect(c.hasContact(42));
    try std.testing.expect(std.meta.eql(C.Gesture{ .scene_tap = .{ .position = v(11, 20), .source = .mouse } }, c.end(42, v(11, 20), null).?));
}
fn eq(expected: ?C.Gesture, actual: ?C.Gesture) !void {
    try std.testing.expect(std.meta.eql(expected, actual));
}
fn start(c: *C, id: u64, x: f32) !void {
    try c.begin(id, v(x, 0), .touch, .primary, .empty_scene);
}
test "literal ownership matrix all source button and hit branches" {
    const Fixture = struct { source: interaction.PointerSource, button: interaction.PointerButton, hit: C.Hit, release: C.Gesture, move: ?C.Gesture };
    const fixtures = [_]Fixture{
        .{ .source = .mouse, .button = .primary, .hit = .world, .release = .{ .scene_tap = .{ .position = v(1, 2), .source = .mouse } }, .move = null },
        .{ .source = .mouse, .button = .primary, .hit = .empty_scene, .release = .{ .scene_tap = .{ .position = v(1, 2), .source = .mouse } }, .move = .{ .orbit = v(6, 0) } },
        .{ .source = .mouse, .button = .primary, .hit = .{ .hud = .launch }, .release = .{ .hud = .launch }, .move = null },
        .{ .source = .mouse, .button = .secondary, .hit = .world, .release = .{ .immediate_launch = v(1, 2) }, .move = null },
        .{ .source = .mouse, .button = .secondary, .hit = .empty_scene, .release = .{ .immediate_launch = v(1, 2) }, .move = null },
        .{ .source = .mouse, .button = .secondary, .hit = .{ .hud = .launch }, .release = .{ .hud = .launch }, .move = null },
        .{ .source = .mouse, .button = .middle, .hit = .world, .release = .{ .scene_tap = .{ .position = v(1, 2), .source = .mouse } }, .move = .{ .orbit = v(6, 0) } },
        .{ .source = .mouse, .button = .middle, .hit = .empty_scene, .release = .{ .scene_tap = .{ .position = v(1, 2), .source = .mouse } }, .move = .{ .orbit = v(6, 0) } },
        .{ .source = .mouse, .button = .middle, .hit = .{ .hud = .launch }, .release = .{ .hud = .launch }, .move = null },
        .{ .source = .touch, .button = .primary, .hit = .world, .release = .{ .scene_tap = .{ .position = v(1, 2), .source = .touch } }, .move = .{ .orbit = v(6, 0) } },
        .{ .source = .touch, .button = .primary, .hit = .empty_scene, .release = .{ .scene_tap = .{ .position = v(1, 2), .source = .touch } }, .move = .{ .orbit = v(6, 0) } },
        .{ .source = .touch, .button = .primary, .hit = .{ .hud = .launch }, .release = .{ .hud = .launch }, .move = null },
        .{ .source = .touch, .button = .secondary, .hit = .world, .release = .{ .immediate_launch = v(1, 2) }, .move = null },
        .{ .source = .touch, .button = .secondary, .hit = .empty_scene, .release = .{ .immediate_launch = v(1, 2) }, .move = null },
        .{ .source = .touch, .button = .secondary, .hit = .{ .hud = .launch }, .release = .{ .hud = .launch }, .move = null },
        .{ .source = .touch, .button = .middle, .hit = .world, .release = .{ .scene_tap = .{ .position = v(1, 2), .source = .touch } }, .move = .{ .orbit = v(6, 0) } },
        .{ .source = .touch, .button = .middle, .hit = .empty_scene, .release = .{ .scene_tap = .{ .position = v(1, 2), .source = .touch } }, .move = .{ .orbit = v(6, 0) } },
        .{ .source = .touch, .button = .middle, .hit = .{ .hud = .launch }, .release = .{ .hud = .launch }, .move = null },
    };
    for (fixtures) |fixture| {
        var c = C.init();
        try c.begin(1, v(0, 0), fixture.source, fixture.button, fixture.hit);
        try eq(fixture.release, c.end(1, v(1, 2), .launch));
        try c.begin(1, v(0, 0), fixture.source, fixture.button, fixture.hit);
        try eq(fixture.move, c.update(1, v(6, 0)));
        try eq(null, c.end(1, v(0, 0), .launch));
    }
}
test "threshold equality sticky movement last delta and end-only suppression" {
    var c = C.init();
    try c.begin(1, v(0, 0), .mouse, .primary, .empty_scene);
    try eq(null, c.update(1, v(5.9, 0)));
    const step = c.update(1, v(6, 0)).?.orbit;
    try std.testing.expectApproxEqAbs(@as(f32, 0.1), step.x, 0.000001);
    try eq(.{ .orbit = v(-6, 0) }, c.update(1, v(0, 0)));
    try eq(.{ .orbit = v(0, 0) }, c.update(1, v(0, 0)));
    try eq(null, c.end(1, v(0, 0), null));
    for ([_]interaction.PointerButton{ .primary, .secondary, .middle }) |button| {
        try c.begin(1, v(0, 0), .mouse, button, .empty_scene);
        try eq(null, c.end(1, v(6, 0), null));
        try c.begin(1, v(0, 0), .mouse, button, .empty_scene);
        _ = c.update(1, v(6, 0));
        _ = c.update(1, v(0, 0));
        try eq(null, c.end(1, v(0, 0), null));
    }
    try c.begin(1, v(0, 0), .mouse, .secondary, .world);
    try eq(null, c.update(1, v(5.9, 0)));
    try eq(.{ .immediate_launch = v(5.9, 0) }, c.end(1, v(5.9, 0), null));
    try eq(null, c.end(1, v(5.9, 0), null));
    try c.begin(1, v(0, 0), .touch, .primary, .world);
    try eq(null, c.update(1, v(3, 4)));
    const diagonal = c.update(1, v(3.6, 4.8)).?.orbit;
    try std.testing.expectApproxEqAbs(@as(f32, 0.6), diagonal.x, 0.000001);
    try std.testing.expectApproxEqAbs(@as(f32, 0.8), diagonal.y, 0.000001);
}
test "preflight precedence shared ID full state preservation and eight ninth" {
    var c = C.init();
    try start(&c, 90, 0);
    try start(&c, 2, 100);
    for (0..6) |i| try c.begin(@intCast(i + 100), v(20, 30), .mouse, .primary, .{ .hud = .launch });
    const before = c;
    try std.testing.expectEqual(@as(usize, 8), c.contactCount());
    const invalid = [_]V{ v(std.math.nan(f32), 0), v(0, std.math.inf(f32)), v(-std.math.inf(f32), 0) };
    for (invalid) |position| {
        try std.testing.expectError(error.NonFinitePosition, c.begin(90, position, .mouse, .secondary, .world));
        try std.testing.expect(std.meta.eql(before, c));
    }
    try std.testing.expectError(error.DuplicateContact, c.begin(90, v(0, 0), .mouse, .secondary, .world));
    try std.testing.expect(std.meta.eql(before, c));
    try std.testing.expectError(error.ContactLimit, c.begin(999, v(0, 0), .touch, .primary, .empty_scene));
    try std.testing.expect(std.meta.eql(before, c));
    _ = c.end(102, v(20, 30), null);
    try c.begin(999, v(1, 0), .mouse, .middle, .world);
    try std.testing.expect(c.hasContact(90) and c.hasContact(2) and c.hasContact(999));
    try std.testing.expectEqual(@as(usize, 8), c.contactCount());
    c.cancel();
    try std.testing.expectEqual(@as(usize, 0), c.contactCount());
    c.cancel();
    try std.testing.expect(std.meta.eql(C.init(), c));
}
test "nonfinite update unchanged end last finite unknown IDs" {
    var c = C.init();
    try c.begin(1, v(2, 3), .mouse, .secondary, .world);
    const before = c;
    try eq(null, c.update(1, v(std.math.nan(f32), 0)));
    try eq(null, c.update(999, v(0, 0)));
    try std.testing.expect(std.meta.eql(before, c));
    try eq(.{ .immediate_launch = v(2, 3) }, c.end(1, v(std.math.inf(f32), 0), null));
    try eq(null, c.end(1, v(2, 3), null));
    try eq(null, c.end(999, v(2, 3), null));
    try c.begin(1, v(2, 3), .touch, .primary, .world);
    try eq(.{ .scene_tap = .{ .position = v(2, 3), .source = .touch } }, c.end(1, v(0, std.math.nan(f32)), null));
}
test "all actual UiAction variants and settings payload equality" {
    const actions = [_]Action{ .{ .select_guidance_world = 7 }, .launch, .clear_selection, .speed_down, .toggle_pause, .speed_up, .reset_camera, .open_settings, .toggle_help, .open_scenario, .acknowledge_advisory, .skip_onboarding, .reset_onboarding, .{ .select_field = .atmosphere }, .decrease_field, .increase_field, .{ .settings = .{ .set_ui_scale = .percent85 } }, .{ .settings = .{ .set_motion = .reduced } }, .{ .settings = .{ .set_high_contrast = true } }, .{ .settings = .{ .set_graphics_quality = .high } }, .{ .settings = .reset_onboarding }, .{ .settings = .close } };
    var c = C.init();
    for (actions) |action| {
        try c.begin(1, v(0, 0), .touch, .middle, .{ .hud = action });
        try eq(.{ .hud = action }, c.end(1, v(1, 0), action));
        try c.begin(1, v(0, 0), .mouse, .secondary, .{ .hud = action });
        try eq(null, c.end(1, v(0, 0), null));
        try c.begin(1, v(0, 0), .mouse, .primary, .{ .hud = action });
        _ = c.update(1, v(6, 0));
        try eq(null, c.end(1, v(0, 0), action));
    }
    for ([_]struct { a: Action, b: Action }{ .{ .a = .{ .select_guidance_world = 7 }, .b = .{ .select_guidance_world = 8 } }, .{ .a = .{ .settings = .{ .set_high_contrast = true } }, .b = .{ .settings = .{ .set_high_contrast = false } } }, .{ .a = .{ .settings = .{ .set_ui_scale = .percent85 } }, .b = .{ .settings = .{ .set_ui_scale = .percent100 } } } }) |fixture| {
        try c.begin(1, v(0, 0), .mouse, .primary, .{ .hud = fixture.a });
        try eq(null, c.end(1, v(0, 0), fixture.b));
    }
}
test "pinch log signs coincident epsilon and resumed literal values" {
    var c = C.init();
    try start(&c, 2, 100);
    try start(&c, 1, 0);
    try std.testing.expectApproxEqAbs(@as(f32, 415.8883), c.update(2, v(200, 0)).?.zoom, 0.001);
    try std.testing.expectApproxEqAbs(@as(f32, -415.8883), c.update(2, v(100, 0)).?.zoom, 0.001);
    try eq(null, c.update(2, v(0, 0)));
    try eq(null, c.update(2, v(100, 0)));
    try std.testing.expectApproxEqAbs(@as(f32, 109.3929), c.update(2, v(120, 0)).?.zoom, 0.001);
    try eq(null, c.update(2, v(std.math.floatEps(f32), 0)));
    try std.testing.expectEqual(std.math.floatEps(f32), c.pinch_distance.?);
    try eq(null, c.update(2, v(100, 0)));
    try std.testing.expectApproxEqAbs(@as(f32, 109.3929), c.update(2, v(120, 0)).?.zoom, 0.001);
    try eq(null, c.end(2, v(120, 0), null));
    try std.testing.expectEqual(@as(?f32, null), c.pinch_distance);
    try eq(null, c.update(1, v(20, 0)));
    try eq(null, c.end(1, v(0, 0), null));
}
test "third scene touch independent HUD restart survivor no automatic pair" {
    var c = C.init();
    try start(&c, 9, 0);
    try start(&c, 1, 100);
    _ = c.update(1, v(200, 0));
    try start(&c, 3, 300);
    try std.testing.expectEqual(@as(f32, 200), c.pinch_distance.?);
    try eq(.{ .orbit = v(10, 0) }, c.update(3, v(310, 0)));
    try eq(null, c.end(3, v(300, 0), null));
    try std.testing.expectEqual(@as(f32, 200), c.pinch_distance.?);
    try c.begin(50, v(0, 0), .touch, .primary, .{ .hud = .launch });
    try std.testing.expectEqual(@as(f32, 200), c.pinch_distance.?);
    try eq(.{ .hud = .launch }, c.end(50, v(0, 0), .launch));
    try start(&c, 3, 300);
    try eq(.{ .scene_tap = .{ .position = v(300, 0), .source = .touch } }, c.end(3, v(300, 0), null));
    try start(&c, 3, 300);
    try eq(null, c.end(1, v(200, 0), null));
    try std.testing.expectEqual(@as(?f32, null), c.pinch_distance);
    try eq(.{ .orbit = v(6, 0) }, c.update(3, v(306, 0)));
    try eq(null, c.update(9, v(0, 0)));
    try start(&c, 4, 400); // Three scene touches: no pair formed.
    try std.testing.expectEqual(@as(?f32, null), c.pinch_distance);
    _ = c.end(4, v(400, 0), null);
    try c.begin(50, v(0, 0), .touch, .primary, .{ .hud = .launch }); // Exactly two scene touches restart on HUD begin.
    try std.testing.expectEqual(@as(f32, 306), c.pinch_distance.?);
    try std.testing.expectApproxEqAbs(@as(f32, 415.8883), c.update(3, v(612, 0)).?.zoom, 0.001);
    c.cancel();
    try eq(null, c.update(3, v(900, 0)));
    try eq(null, c.end(9, v(0, 0), null));
    try start(&c, 1, 0);
    try start(&c, 2, 100);
    try std.testing.expectApproxEqAbs(@as(f32, 415.8883), c.update(2, v(200, 0)).?.zoom, 0.001);
}
test "mouse HUD exclusion and ID order permutations" {
    var c = C.init();
    try c.begin(0, v(0, 0), .mouse, .primary, .empty_scene);
    try start(&c, 2, 100);
    try std.testing.expectEqual(@as(?f32, null), c.pinch_distance);
    try eq(.{ .orbit = v(6, 0) }, c.update(2, v(106, 0)));
    c.cancel();
    try c.begin(0, v(0, 0), .touch, .primary, .{ .hud = .launch });
    try start(&c, 2, 100);
    try std.testing.expectEqual(@as(?f32, null), c.pinch_distance);
    for ([_][2]u64{ .{ 1, 2 }, .{ 2, 1 }, .{ std.math.maxInt(u64), 0 } }) |ids| {
        c.cancel();
        try start(&c, ids[0], 0);
        try start(&c, ids[1], 100);
        try std.testing.expectApproxEqAbs(@as(f32, 415.8883), c.update(ids[1], v(200, 0)).?.zoom, 0.001);
    }
}
test "finite derived overflow preserved orbit and pinch baseline recovery" {
    var c = C.init();
    const huge = std.math.floatMax(f32);
    try c.begin(1, v(-huge, 0), .mouse, .middle, .empty_scene);
    try std.testing.expect(!c.update(1, v(huge, 0)).?.orbit.isFinite());
    c.cancel();
    try start(&c, 1, -huge);
    try start(&c, 2, huge);
    try std.testing.expect(!std.math.isFinite(c.pinch_distance.?));
    try eq(null, c.update(1, v(0, 0))); // Still infinite derived squared distance.
    try eq(null, c.update(2, v(100, 0))); // Replaces infinite baseline first.
    try std.testing.expectEqual(@as(f32, 100), c.pinch_distance.?);
    try std.testing.expectApproxEqAbs(@as(f32, 109.3929), c.update(2, v(120, 0)).?.zoom, 0.001);
}
// This actual callable witness takes only a controller pointer and numeric/value inputs.
// It cannot obtain an allocator, and production imports only std/math; no OOM simulation.
fn allocatorFreeWitness(c: *C) !void {
    for (0..100) |_| {
        try start(c, 1, 0);
        try start(c, 2, 100);
        _ = c.update(2, v(120, 0));
        _ = c.end(2, v(120, 0), null);
        _ = c.end(1, v(0, 0), null);
        c.cancel();
    }
}
test "actual storage caps and allocator-free callable cycles" {
    try std.testing.expect(@sizeOf(Action) <= 64);
    try std.testing.expect(C.entry_bytes <= 128);
    try std.testing.expect(@sizeOf(C) <= 1088);
    std.debug.print("interaction sizes action={d} entry={d} controller={d}; caller stack unknown\n", .{ @sizeOf(Action), C.entry_bytes, @sizeOf(C) });
    var c = C.init();
    try allocatorFreeWitness(&c);
    try std.testing.expect(std.meta.eql(C.init(), c));
}
test "literal payload value inventory compatible with real Session action" {
    const actions = [_]Action{
        .{ .select_guidance_world = 0 },                    .{ .select_guidance_world = 255 },
        .{ .select_field = .atmosphere },                   .{ .select_field = .hydrosphere },
        .{ .select_field = .topology },                     .{ .settings = .{ .set_ui_scale = .percent85 } },
        .{ .settings = .{ .set_ui_scale = .percent100 } },  .{ .settings = .{ .set_ui_scale = .percent115 } },
        .{ .settings = .{ .set_ui_scale = .percent130 } },  .{ .settings = .{ .set_motion = .full } },
        .{ .settings = .{ .set_motion = .reduced } },       .{ .settings = .{ .set_high_contrast = false } },
        .{ .settings = .{ .set_high_contrast = true } },    .{ .settings = .{ .set_graphics_quality = .auto } },
        .{ .settings = .{ .set_graphics_quality = .low } }, .{ .settings = .{ .set_graphics_quality = .high } },
    };
    var c = C.init();
    for (actions, 0..) |action, i| {
        try c.begin(1, v(0, 0), .mouse, .primary, .{ .hud = action });
        try eq(.{ .hud = action }, c.end(1, v(0, 0), action));
        try c.begin(1, v(0, 0), .touch, .secondary, .{ .hud = action });
        try eq(null, c.end(1, v(0, 0), actions[(i + 1) % actions.len]));
    }
}
test "empty zero-capacity and one-contact unknown/cancel cycles" {
    const Zero = interaction.InteractionController(Action, 0);
    var zero = Zero.init();
    try std.testing.expectError(error.ContactLimit, zero.begin(0, v(0, 0), .touch, .primary, .world));
    try std.testing.expectEqual(@as(usize, 0), zero.contactCount());
    var c = C.init();
    try eq(null, c.update(0, v(0, 0)));
    try eq(null, c.end(0, v(0, 0), null));
    for (0..8) |i| {
        try c.begin(@intCast(i), v(1, 2), .mouse, .middle, .world);
        try std.testing.expectEqual(@as(usize, 1), c.contactCount());
        c.cancel();
        c.cancel();
        try std.testing.expect(std.meta.eql(C.init(), c));
        try eq(null, c.end(@intCast(i), v(1, 2), null));
    }
}
test "all nonfinite components rejected before duplicate preserving existing pinch" {
    var c = C.init();
    try start(&c, 1, 0);
    try start(&c, 2, 100);
    const before = c;
    for ([_]f32{ std.math.nan(f32), std.math.inf(f32), -std.math.inf(f32) }) |invalid| {
        for ([_]V{ v(invalid, 0), v(0, invalid) }) |position| {
            try std.testing.expectError(error.NonFinitePosition, c.begin(1, position, .mouse, .primary, .world));
            try std.testing.expect(std.meta.eql(before, c));
            try eq(null, c.update(2, position));
            try std.testing.expect(std.meta.eql(before, c));
        }
    }
    try eq(null, c.end(2, v(std.math.nan(f32), 0), null));
    try std.testing.expectEqual(@as(?f32, null), c.pinch_distance);
    try eq(null, c.end(1, v(0, 0), null));
}
