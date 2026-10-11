const std = @import("std");
const t = std.testing;
const o = @import("classic_onboarding.zig");

// Catches wrong event matching, eager completion, and progress inferred from frames.
test "Classic3E onboarding matches the six literal events and completes only once" {
    const steps = [_]o.Step{ .select_union_world, .preview_launch, .launch_fleet, .tune_field, .read_advisory, .open_scenario_editor };
    const events = [_]o.Event{ .union_world_selected, .launch_previewed, .fleet_launched, .field_tuned, .advisory_read, .scenario_editor_opened };
    var state = o.State.init(false);
    for (steps, events, 0..) |step, event, i| {
        try t.expectEqual(@as(?o.Step, step), state.step());
        try t.expectEqual(o.Progress{ .current = i + 1, .total = 6 }, state.progress());
        for (events) |wrong| if (wrong != event) {
            try t.expect(!state.observe(wrong));
            try t.expectEqual(@as(?o.Step, step), state.step());
        };
        try t.expectEqual(i == 5, state.observe(event));
    }
    try t.expect(!state.active());
    try t.expectEqual(@as(?o.Step, null), state.step());
    try t.expectEqual(o.Progress{ .current = 6, .total = 6 }, state.progress());
    for (events) |event| try t.expect(!state.observe(event));
}

// Catches duplicate saves on inactive skip and reset retaining partial progress.
test "Classic3E onboarding completed startup skip and reset are explicit transitions" {
    var state = o.State.init(true);
    try t.expect(!state.active());
    try t.expect(!state.skip());
    state.reset();
    try t.expectEqual(@as(?o.Step, .select_union_world), state.step());
    try t.expect(!state.observe(.union_world_selected));
    try t.expectEqual(@as(?o.Step, .preview_launch), state.step());
    try t.expect(state.skip());
    try t.expect(!state.skip());
    state.reset();
    try t.expectEqual(o.Progress{ .current = 1, .total = 6 }, state.progress());
}
