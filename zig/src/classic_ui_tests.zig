const std = @import("std");
const ui = @import("classic_ui.zig");
const r = @import("retained_semantics.zig");
test "H3a contemporary pure projection admission" {
    var tokens = r.LiveTokenBudget.init(.{ .max_live_objects = 1, .max_live_requested_bytes = 1280 });
    defer tokens.deinit();
    const owner = try r.TokenRef.create(std.testing.allocator, &tokens);
    defer owner.release();
    var budget = r.PreparationBudget.init(196608);
    defer budget.deinit();
    var p = try ui.prepareHudSettings(std.testing.allocator, &budget, .{}, .{ .revision = 4, .owner = .{ .resident = owner, .hold = null, .context = "classic", .draft = null } });
    defer p.deinit();
    try std.testing.expectEqualStrings("NYON", p.input().nodes[0].name);
}
fn stamp(owner: r.TokenRef, revision: u64) ui.CurrentStamp {
    return .{ .revision = revision, .owner = .{ .resident = owner, .hold = null, .context = "classic", .draft = null } };
}
test "H3a contemporary publication and actual frame action admission" {
    var tokens = r.LiveTokenBudget.init(.{ .max_live_objects = 1, .max_live_requested_bytes = 1280 });
    defer tokens.deinit();
    const owner = try r.TokenRef.create(std.testing.allocator, &tokens);
    defer owner.release();
    var budget = r.PreparationBudget.init(196608);
    defer budget.deinit();
    var p = try ui.prepareHudSettings(std.testing.allocator, &budget, .{ .onboarding_step = .select_union_world, .hud_focus = .speed_down }, stamp(owner, 4));
    defer p.deinit();
    var slot = r.FrameSlot.init(std.testing.allocator, &budget);
    defer slot.deinit();
    var published: ?u64 = null;
    try ui.publishHudSettings(&slot, &published, &p, stamp(owner, 4));
    try std.testing.expectEqual(@as(?u64, 4), published);
    try std.testing.expectError(error.DisabledAction, ui.validateAction(slot.view().?, published.?, stamp(owner, 4), .{ .bytes = "a/hud/speed_down" }));
    try std.testing.expectEqualDeep(@import("classic_session.zig").UiAction.open_settings, try ui.validateAction(slot.view().?, published.?, stamp(owner, 4), .{ .bytes = "a/hud/settings" }));
    const old = slot.view().?.nodes.ptr;
    try std.testing.expectError(error.StaleProjection, ui.publishHudSettings(&slot, &published, &p, stamp(owner, 5)));
    try std.testing.expectEqual(old, slot.view().?.nodes.ptr);
    try std.testing.expectEqual(@as(?u64, 4), published);
    try std.testing.expectError(error.StaleProjection, ui.validateAction(slot.view().?, published.?, stamp(owner, 5), .{ .bytes = "a/hud/settings" }));
}
test "H3a contemporary closed resource recovery and immutable Session adapter" {
    var scratch: [256]u8 = undefined;
    try std.testing.expectEqualStrings("PREFERENCE MEMORY ALLOCATION FAILED | EDITOR OPERATION FAILED | COMMAND MEMORY ALLOCATION FAILED", try ui.recoveryText(.{ .resource = .{ .preferences = error.OutOfMemory, .editor = error.TestEditorDetail, .command = error.OutOfMemory } }, &scratch));
    const store = @import("classic_store.zig");
    var raw = store.MemoryStore.init(std.testing.allocator, .preferences);
    defer raw.deinit();
    const candidate = try @import("scenario.zig").generatedValidated(@import("model.zig").DEFAULT_SEED, .{});
    var session = try @import("classic_session.zig").Session.init(std.testing.allocator, &candidate, raw.raw());
    defer session.deinit();
    var tokens = r.LiveTokenBudget.init(.{ .max_live_objects = 1, .max_live_requested_bytes = 1280 });
    defer tokens.deinit();
    const owner = try r.TokenRef.create(std.testing.allocator, &tokens);
    defer owner.release();
    var budget = r.PreparationBudget.init(196608);
    defer budget.deinit();
    var p = try ui.prepareHudSettingsFromSession(std.testing.allocator, &budget, &session, null, stamp(owner, 4));
    defer p.deinit();
    try std.testing.expectEqualStrings("PLAY", p.input().nodes[5].name);
    try std.testing.expect(session.onboarding.active());
    try std.testing.expectEqual(@as(usize, 0), raw.save_calls);
}
test "H3a contemporary legacy opaque pointer chrome" {
    var tokens = r.LiveTokenBudget.init(.{ .max_live_objects = 1, .max_live_requested_bytes = 1280 });
    defer tokens.deinit();
    const owner = try r.TokenRef.create(std.testing.allocator, &tokens);
    defer owner.release();
    var budget = r.PreparationBudget.init(196608);
    defer budget.deinit();
    var p = try ui.prepareHudSettings(std.testing.allocator, &budget, .{ .viewport = .{ .x = 960, .y = 600 }, .preferences = .{ .ui_scale = .percent85 } }, stamp(owner, 1));
    defer p.deinit();
    try std.testing.expect(ui.consumesPointer(&p, .{ .x = std.math.nan(f32), .y = 0 }));
    try std.testing.expect(ui.consumesPointer(&p, .{ .x = 171, .y = 300 }));
    try std.testing.expect(ui.consumesPointer(&p, .{ .x = 500, .y = 498 }));
    try std.testing.expect(!ui.consumesPointer(&p, .{ .x = 500, .y = 300 }));
}
const t = std.testing;
fn find(input: r.FrameInput, id: []const u8) !r.NodeSpec {
    for (input.nodes) |n| if (std.mem.eql(u8, n.id.bytes, id)) return n;
    return error.MissingFixtureNode;
}
fn expectBounds(input: r.FrameInput, id: []const u8, min: [2]f32, max: [2]f32) !void {
    const rect = (try find(input, id)).bounds.?;
    inline for (0..2) |i| {
        try t.expectApproxEqAbs(min[i], rect.min[i], 0.0001);
        try t.expectApproxEqAbs(max[i], rect.max[i], 0.0001);
    }
}
const PlayingFixture = struct { id: []const u8, name: []const u8, tip: []const u8, action: @import("classic_session.zig").UiAction, role: r.NodeRole = .button };
const playing_fixtures = [_]PlayingFixture{
    .{ .id = "a/hud/speed_down", .name = "SPEED -", .tip = "Decrease game speed ([)", .action = .speed_down },
    .{ .id = "a/hud/pause", .name = "PLAY", .tip = "Pause or resume (P)", .action = .toggle_pause },
    .{ .id = "a/hud/speed_up", .name = "SPEED +", .tip = "Increase game speed (])", .action = .speed_up },
    .{ .id = "a/hud/launch", .name = "LAUNCH", .tip = "Launch the command tray preview", .action = .launch },
    .{ .id = "a/hud/clear_selection", .name = "CLEAR", .tip = "Clear source and destination (Escape)", .action = .clear_selection },
    .{ .id = "a/hud/reset_camera", .name = "RESET", .tip = "Reset tactical camera (C)", .action = .reset_camera },
    .{ .id = "a/hud/settings", .name = "SETTINGS", .tip = "Open local settings", .action = .open_settings },
    .{ .id = "a/hud/help", .name = "HELP", .tip = "Show controls and guidance", .action = .toggle_help },
    .{ .id = "a/hud/scenario", .name = "SCENARIO", .tip = "Open detached scenario editor (F4)", .action = .open_scenario },
    .{ .id = "a/hud/skip", .name = "SKIP", .tip = "Skip tutorial and play this match now", .action = .skip_onboarding },
    .{ .id = "a/hud/field/atmosphere", .name = "ATM", .tip = "Select field channel (1/2/3)", .action = .{ .select_field = .atmosphere }, .role = .option },
    .{ .id = "a/hud/field/hydrosphere", .name = "HYD", .tip = "Select field channel (1/2/3)", .action = .{ .select_field = .hydrosphere }, .role = .option },
    .{ .id = "a/hud/field/topology", .name = "TOP", .tip = "Select field channel (1/2/3)", .action = .{ .select_field = .topology }, .role = .option },
    .{ .id = "a/hud/field/decrease", .name = "FIELD -", .tip = "Decrease selected field (Q)", .action = .decrease_field },
    .{ .id = "a/hud/field/increase", .name = "FIELD +", .tip = "Increase selected field (E)", .action = .increase_field },
    .{ .id = "a/hud/advisory_ack", .name = "ADVISORY", .tip = "Read the advisory status", .action = .acknowledge_advisory },
    .{ .id = "a/hud/reset_guidance", .name = "RESTART GUIDANCE", .tip = "Restart first-run guidance", .action = .reset_onboarding },
};
fn topology(input: r.FrameInput) !void {
    try t.expectEqual(input.nodes.len - 1, input.child_indices.len);
    for (input.nodes, 0..) |n, i| {
        var last: ?usize = null;
        for (input.child_indices[n.child_start..][0..n.child_count]) |child| {
            try t.expect(child < input.nodes.len);
            try t.expectEqual(@as(?usize, i), input.nodes[child].parent);
            if (last) |previous| try t.expect(child > previous);
            last = child;
        }
        if (i != 0) {
            const parent = input.nodes[n.parent.?];
            var matches: usize = 0;
            for (input.child_indices[parent.child_start..][0..parent.child_count]) |child| {
                if (child == i) matches += 1;
            }
            try t.expectEqual(@as(usize, 1), matches);
        }
    }
}
test "H3a independent complete HUD literal action role IDs topology and disabled focus" {
    var tokens = r.LiveTokenBudget.init(.{ .max_live_objects = 1, .max_live_requested_bytes = 1280 });
    defer tokens.deinit();
    const owner = try r.TokenRef.create(t.allocator, &tokens);
    defer owner.release();
    var budget = r.PreparationBudget.init(196608);
    defer budget.deinit();
    var p = try ui.prepareHudSettings(t.allocator, &budget, .{ .speed_multiplier = 0, .onboarding_step = .select_union_world, .help_visible = true, .source_name = "ALPHA", .target_name = "BETA", .launch_enabled = true, .strength = 0, .decrease_enabled = true, .increase_enabled = true, .decrease_tooltip = "Decrease selected field (Q)", .increase_tooltip = "Increase selected field (E)", .pointer = .{ .x = 285, .y = 850 }, .recoverable = "α雪", .hud_focus = .speed_down }, stamp(owner, 2));
    defer p.deinit();
    var slot = r.FrameSlot.init(t.allocator, &budget);
    defer slot.deinit();
    var published: ?u64 = null;
    try ui.publishHudSettings(&slot, &published, &p, stamp(owner, 2));
    const input = p.input();
    try t.expectEqual(@as(usize, 64), input.nodes.len);
    try t.expectEqual(@as(usize, 17), input.focus.base.len);
    try t.expect(input.focus.modal == null and input.focus.restore == null);
    try t.expectEqualStrings("a/hud/speed_down", input.focus.focused.?.bytes);
    try topology(input);
    for (playing_fixtures, 0..) |expected, i| {
        try t.expectEqualStrings(expected.id, input.focus.base[i].bytes);
        var got: ?r.NodeSpec = null;
        for (input.nodes) |n| if (n.action) |a| {
            if (std.mem.eql(u8, a.bytes, expected.id)) got = n;
        };
        const n = got.?;
        try t.expectEqualStrings(expected.name, n.name);
        try t.expectEqualStrings(expected.tip, n.description);
        try t.expectEqual(expected.role, n.role);
        try t.expect(n.focusable);
        try t.expectEqualStrings("", n.value.?);
        try t.expectEqualStrings(expected.name, input.nodes[input.child_indices[n.child_start]].name);
        try t.expectEqualStrings("", input.nodes[input.child_indices[n.child_start + 1]].name);
        try t.expectEqual(@as(usize, 2), n.child_count);
        try t.expectEqual(i >= 3, n.enabled);
        const view = slot.view().?;
        if (n.enabled) try t.expectEqualDeep(expected.action, try ui.validateAction(view, published.?, stamp(owner, 2), .{ .bytes = expected.id }));
    }
    try t.expectEqualStrings("SOURCE ALPHA", (try find(input, "c/hud/tray/source")).name);
    try t.expectEqualStrings("TARGET BETA", (try find(input, "c/hud/tray/target")).name);
    try t.expectEqualStrings("STRENGTH 0", (try find(input, "c/hud/tray/strength")).name);
    try t.expectEqualStrings("TUTORIAL 1/6 // PAUSED", (try find(input, "c/guidance/progress")).name);
    try t.expectEqualStrings("SELECT A UNION WORLD", (try find(input, "c/guidance/title")).name);
    try t.expectEqualStrings("Decrease game speed ([)", (try find(input, "c/tooltip")).name);
    try t.expectEqualStrings("α雪", (try find(input, "c/recoverable")).name);
    try t.expectEqualStrings("α雪", (try find(input, "c/recoverable")).value.?);
    try t.expectEqualStrings("ATM", (try find(input, "c/selected-field")).value.?);
    // Accepted H1 focus keeps disabled actions in its order and wraps.
    var focus = try r.OwnedFocus.init(t.allocator, input.focus, ui.limits, &budget);
    defer focus.deinit();
    _ = focus.next();
    try t.expectEqualStrings("a/hud/pause", focus.view().focused.?.bytes);
    _ = focus.previous();
    try t.expectEqualStrings("a/hud/speed_down", focus.view().focused.?.bytes);
}
test "H3a actual Settings publication obeys H1 DFS preorder" {
    var tokens = r.LiveTokenBudget.init(.{ .max_live_objects = 1, .max_live_requested_bytes = 1280 });
    defer tokens.deinit();
    const owner = try r.TokenRef.create(t.allocator, &tokens);
    defer owner.release();
    var budget = r.PreparationBudget.init(196608);
    defer budget.deinit();
    var p = try ui.prepareHudSettings(t.allocator, &budget, .{ .mode = .settings }, stamp(owner, 2));
    defer p.deinit();
    var slot = r.FrameSlot.init(t.allocator, &budget);
    defer slot.deinit();
    var revision: ?u64 = null;
    try ui.publishHudSettings(&slot, &revision, &p, stamp(owner, 2));
    try t.expectEqual(@as(?u64, 2), revision);
    try t.expectEqual(@as(usize, 39), slot.view().?.nodes.len);
}
const SettingFixture = struct { id: []const u8, label: []const u8, role: r.NodeRole, action: @import("classic_settings.zig").Action };
const setting_fixtures = [_]SettingFixture{
    .{ .id = "a/settings/scale/85", .label = "85%", .role = .option, .action = .{ .set_ui_scale = .percent85 } },
    .{ .id = "a/settings/scale/100", .label = "100%", .role = .option, .action = .{ .set_ui_scale = .percent100 } },
    .{ .id = "a/settings/scale/115", .label = "115%", .role = .option, .action = .{ .set_ui_scale = .percent115 } },
    .{ .id = "a/settings/scale/130", .label = "130%", .role = .option, .action = .{ .set_ui_scale = .percent130 } },
    .{ .id = "a/settings/motion", .label = "REDUCED MOTION", .role = .checkbox, .action = .{ .set_motion = .reduced } },
    .{ .id = "a/settings/contrast", .label = "HIGH CONTRAST", .role = .checkbox, .action = .{ .set_high_contrast = true } },
    .{ .id = "a/settings/quality/auto", .label = "AUTO", .role = .option, .action = .{ .set_graphics_quality = .auto } },
    .{ .id = "a/settings/quality/low", .label = "LOW", .role = .option, .action = .{ .set_graphics_quality = .low } },
    .{ .id = "a/settings/quality/high", .label = "HIGH", .role = .option, .action = .{ .set_graphics_quality = .high } },
    .{ .id = "a/settings/reset_guidance", .label = "RESET GUIDANCE", .role = .button, .action = .reset_onboarding },
    .{ .id = "a/settings/close", .label = "CLOSE", .role = .button, .action = .close },
};
test "H3a all11 Settings source choices roles inverse toggles replace underlay" {
    var tokens = r.LiveTokenBudget.init(.{ .max_live_objects = 1, .max_live_requested_bytes = 1280 });
    defer tokens.deinit();
    const owner = try r.TokenRef.create(t.allocator, &tokens);
    defer owner.release();
    var budget = r.PreparationBudget.init(196608);
    defer budget.deinit();
    var slot = r.FrameSlot.init(t.allocator, &budget);
    defer slot.deinit();
    var revision: ?u64 = null;
    {
        var p = try ui.prepareHudSettings(t.allocator, &budget, .{}, stamp(owner, 1));
        defer p.deinit();
        try ui.publishHudSettings(&slot, &revision, &p, stamp(owner, 1));
    }
    inline for ([_]@import("preferences.zig").UiScale{ .percent85, .percent100, .percent115, .percent130 }, 0..) |scale, scale_index| {
        var p = try ui.prepareHudSettings(t.allocator, &budget, .{ .mode = .settings, .settings_focus = 99, .preferences = .{ .ui_scale = scale } }, stamp(owner, 2 + scale_index));
        defer p.deinit();
        try ui.publishHudSettings(&slot, &revision, &p, stamp(owner, 2 + scale_index));
        const input = p.input();
        const view = slot.view().?;
        try topology(input);
        try t.expectEqualStrings("c/settings/root", view.nodes[0].id.bytes);
        try t.expect(view.node(.{ .bytes = "c/hud/toolbar" }) == null);
        try t.expect(view.node(.{ .bytes = "c/hud/tray" }) == null);
        try t.expectEqualStrings("SETTINGS", view.nodes[1].name);
        try t.expectEqualStrings("a/settings/close", view.focus.focused.?.bytes);
        try t.expect(view.focus.modal == null);
        try t.expect(ui.consumesPointer(&p, .{ .x = 600, .y = 200 }));
        for (setting_fixtures, 0..) |expected, i| {
            try t.expectEqualStrings(expected.id, view.focus.base[i].bytes);
            var n: ?r.NodeSpec = null;
            for (view.nodes) |value| if (value.action) |a| {
                if (std.mem.eql(u8, a.bytes, expected.id)) n = value;
            };
            try t.expectEqualStrings(expected.label, n.?.name);
            try t.expectEqualStrings("Local offline preference", n.?.description);
            try t.expectEqual(expected.role, n.?.role);
            try t.expect(n.?.enabled and n.?.focusable);
            if (i < 4) try t.expectEqual(i == scale_index, n.?.selected);
            try t.expectEqualDeep(@import("classic_session.zig").UiAction{ .settings = expected.action }, try ui.validateAction(view, revision.?, stamp(owner, 2 + scale_index), .{ .bytes = expected.id }));
        }
    }
    {
        var p = try ui.prepareHudSettings(t.allocator, &budget, .{ .mode = .settings, .preferences = .{ .motion = .reduced, .high_contrast = true, .graphics_quality = .high } }, stamp(owner, 9));
        defer p.deinit();
        try ui.publishHudSettings(&slot, &revision, &p, stamp(owner, 9));
        try t.expectEqualDeep(@import("classic_session.zig").UiAction{ .settings = .{ .set_motion = .full } }, try ui.validateAction(slot.view().?, revision.?, stamp(owner, 9), .{ .bytes = "a/settings/motion" }));
        try t.expectEqualDeep(@import("classic_session.zig").UiAction{ .settings = .{ .set_high_contrast = false } }, try ui.validateAction(slot.view().?, revision.?, stamp(owner, 9), .{ .bytes = "a/settings/contrast" }));
        try t.expect((try find(p.input(), "c/settings/quality/high/primary")).selected);
    }
}
test "H3a independent source geometry 1440 960 breakpoint scales nonfinite tiny" {
    var tokens = r.LiveTokenBudget.init(.{ .max_live_objects = 1, .max_live_requested_bytes = 1280 });
    defer tokens.deinit();
    const owner = try r.TokenRef.create(t.allocator, &tokens);
    defer owner.release();
    var budget = r.PreparationBudget.init(196608);
    defer budget.deinit();
    const fixtures = [_]struct { source: ui.ProjectionSource, id: []const u8, min: [2]f32, max: [2]f32 }{
        .{ .source = .{}, .id = "c/hud/speed_down/primary", .min = .{ 282, 848 }, .max = .{ 374, 896 } },
        .{ .source = .{ .viewport = .{ .x = 960, .y = 600 } }, .id = "c/hud/speed_down/primary", .min = .{ 208, 502 }, .max = .{ 312, 546 } },
        .{ .source = .{ .mode = .settings }, .id = "c/settings/scale/85/primary", .min = .{ 408, 320 }, .max = .{ 558, 368 } },
        .{ .source = .{ .viewport = .{ .x = 1099, .y = 900 } }, .id = "c/hud/speed_down/primary", .min = .{ 277.5, 802 }, .max = .{ 381.5, 846 } },
        .{ .source = .{ .viewport = .{ .x = 1100, .y = 900 } }, .id = "c/hud/speed_down/primary", .min = .{ 202, 852 }, .max = .{ 274, 896 } },
        .{ .source = .{ .preferences = .{ .ui_scale = .percent85 } }, .id = "c/hud/speed_down/primary", .min = .{ 344.1, 852 }, .max = .{ 422.3, 896 } },
        .{ .source = .{ .preferences = .{ .ui_scale = .percent115 } }, .id = "c/hud/speed_down/primary", .min = .{ 219.9, 840.8 }, .max = .{ 325.7, 896 } },
        .{ .source = .{ .preferences = .{ .ui_scale = .percent130 } }, .id = "c/hud/speed_down/primary", .min = .{ 157.8, 833.6 }, .max = .{ 277.4, 896 } },
        .{ .source = .{ .viewport = .{ .x = std.math.nan(f32), .y = 900 } }, .id = "c/hud/speed_down/primary", .min = .{ 8, 0 }, .max = .{ 8, 44 } },
        .{ .source = .{ .viewport = .{ .x = 0.2, .y = 0.3 } }, .id = "c/hud/speed_down/primary", .min = .{ 8, 0 }, .max = .{ 8, 44 } },
    };
    for (fixtures) |fixture| {
        var p = try ui.prepareHudSettings(t.allocator, &budget, fixture.source, stamp(owner, 1));
        defer p.deinit();
        try expectBounds(p.input(), fixture.id, fixture.min, fixture.max);
    }
}
test "H3a exhaustive rejection title decimal field and mode combinations" {
    var tokens = r.LiveTokenBudget.init(.{ .max_live_objects = 1, .max_live_requested_bytes = 1280 });
    defer tokens.deinit();
    const owner = try r.TokenRef.create(t.allocator, &tokens);
    defer owner.release();
    var budget = r.PreparationBudget.init(196608);
    defer budget.deinit();
    const rejections = [_][]const u8{ "REJECTED - WRONG TICK", "REJECTED - DUPLICATE COMMAND", "REJECTED - CAMPAIGN FINISHED", "REJECTED - INVALID WORLD", "REJECTED - CHOOSE ANOTHER TARGET", "REJECTED - SOURCE IS NOT UNION", "REJECTED - INSUFFICIENT ENERGY", "REJECTED - FIELD AT LIMIT" };
    for (rejections) |text| {
        var p = try ui.prepareHudSettings(t.allocator, &budget, .{ .tray_status = text, .decrease_tooltip = text, .increase_tooltip = text }, stamp(owner, 1));
        defer p.deinit();
        try t.expectEqualStrings(text, (try find(p.input(), "c/hud/tray/status")).name);
        try t.expectEqualStrings(text, (try find(p.input(), "c/hud/field/decrease/primary")).description);
        try t.expectEqualStrings(text, (try find(p.input(), "c/hud/field/increase/primary")).description);
    }
    const steps = [_]@import("classic_onboarding.zig").Step{ .select_union_world, .preview_launch, .launch_fleet, .tune_field, .read_advisory, .open_scenario_editor };
    const titles = [_][]const u8{ "SELECT A UNION WORLD", "PREVIEW A DESTINATION", "LAUNCH THE FLEET", "TUNE A FIELD", "READ THE ADVISORY", "OPEN THE SCENARIO EDITOR" };
    const progress = [_][]const u8{ "TUTORIAL 1/6 // PAUSED", "TUTORIAL 2/6 // PAUSED", "TUTORIAL 3/6 // PAUSED", "TUTORIAL 4/6 // PAUSED", "TUTORIAL 5/6 // PAUSED", "TUTORIAL 6/6 // PAUSED" };
    for (steps, titles, progress) |step, title, line| {
        var p = try ui.prepareHudSettings(t.allocator, &budget, .{ .onboarding_step = step, .strength = std.math.maxInt(u64), .source_name = "ABCDEFGHIJKLMNOPQRSTUVWX", .target_name = "ABCDEFGHIJKLMNOPQRSTUVWXYZ"[0..24], .tray_status = "COMMAND READY - EXPLICIT LAUNCH REQUIRED" }, stamp(owner, 1));
        defer p.deinit();
        try t.expectEqualStrings(title, (try find(p.input(), "c/guidance/title")).name);
        try t.expectEqualStrings(line, (try find(p.input(), "c/guidance/progress")).name);
        try t.expectEqualStrings("STRENGTH 18446744073709551615", (try find(p.input(), "c/hud/tray/strength")).name);
        try t.expectEqualStrings("SOURCE ABCDEFGHIJKLMNOPQRSTUVWX", (try find(p.input(), "c/hud/tray/source")).name);
        try t.expectEqualStrings("COMMAND READY - EXPLICIT LAUNCH REQUIRED", (try find(p.input(), "c/hud/tray/status")).name);
    }
    for ([_]bool{ false, true }) |paused| for ([_]bool{ false, true }) |help| for ([_]bool{ false, true }) |tutorial| {
        var p = try ui.prepareHudSettings(t.allocator, &budget, .{ .speed_multiplier = if (paused) 0 else 4, .help_visible = help, .onboarding_step = if (tutorial) .tune_field else null }, stamp(owner, 1));
        defer p.deinit();
        const input = p.input();
        try t.expectEqual(@as(usize, 15) + @intFromBool(help) + @intFromBool(tutorial), input.focus.base.len);
        const pause = try find(input, "c/hud/pause/primary");
        try t.expectEqualStrings(if (paused) "PLAY" else "PAUSE", pause.name);
        try t.expectEqual(paused, pause.selected);
        try t.expectEqual(!tutorial, pause.enabled);
        try t.expectEqual(help, (try find(input, "c/hud/help/primary")).selected);
    };
    for ([_]@import("model.zig").FieldKind{ .atmosphere, .hydrosphere, .topology }, [_][]const u8{ "ATM", "HYD", "TOP" }, [_][]const u8{ "c/hud/field/atmosphere/primary", "c/hud/field/hydrosphere/primary", "c/hud/field/topology/primary" }) |field, label, id| {
        var p = try ui.prepareHudSettings(t.allocator, &budget, .{ .selected_field = field }, stamp(owner, 1));
        defer p.deinit();
        try t.expect((try find(p.input(), id)).selected);
        try t.expectEqualStrings(label, (try find(p.input(), "c/selected-field")).value.?);
    }
}
test "H3a owned snapshot poisons all borrowed bytes and exact source admission" {
    var tokens = r.LiveTokenBudget.init(.{ .max_live_objects = 1, .max_live_requested_bytes = 1280 });
    defer tokens.deinit();
    const owner = try r.TokenRef.create(t.allocator, &tokens);
    defer owner.release();
    var budget = r.PreparationBudget.init(196608);
    defer budget.deinit();
    var context: [128]u8 = @splat('C');
    var recovery = try t.allocator.alloc(u8, 16385);
    defer t.allocator.free(recovery);
    @memset(recovery, 'x');
    recovery[0] = 0xce;
    recovery[1] = 0xb1;
    var source_name = "ALPHA".*;
    var target_name = "BETA".*;
    var hovered_name = "GAMMA".*;
    var status = "COMMAND READY - EXPLICIT LAUNCH REQUIRED".*;
    var decrease = "Decrease selected field (Q)".*;
    var increase = "Increase selected field (E)".*;
    const current = ui.CurrentStamp{ .revision = 1, .owner = .{ .resident = owner, .hold = 99, .context = &context, .draft = null } };
    var p = try ui.prepareHudSettings(t.allocator, &budget, .{ .source_name = &source_name, .target_name = &target_name, .hovered_name = &hovered_name, .pointer = .{ .x = 600, .y = 200 }, .recoverable = recovery[0..16384], .tray_status = &status, .decrease_tooltip = &decrease, .increase_tooltip = &increase }, current);
    // Move the handle; the Header and all borrowed input addresses stay stable.
    const old_nodes = p.input().nodes.ptr;
    var moved = p;
    p.header = null;
    defer moved.deinit();
    @memset(&source_name, '!');
    @memset(&target_name, '!');
    @memset(&hovered_name, '!');
    @memset(&status, '!');
    @memset(&decrease, '!');
    @memset(&increase, '!');
    @memset(&context, '!');
    @memset(recovery, '!');
    try t.expectEqual(old_nodes, moved.input().nodes.ptr);
    try t.expectEqualStrings("SOURCE ALPHA", (try find(moved.input(), "c/hud/tray/source")).name);
    try t.expectEqualStrings("TARGET BETA", (try find(moved.input(), "c/hud/tray/target")).name);
    try t.expectEqualStrings("WORLD GAMMA - SELECT OR PREVIEW DESTINATION", (try find(moved.input(), "c/tooltip")).name);
    try t.expectEqualStrings("COMMAND READY - EXPLICIT LAUNCH REQUIRED", (try find(moved.input(), "c/hud/tray/status")).name);
    try t.expectEqualStrings("Decrease selected field (Q)", (try find(moved.input(), "c/hud/field/decrease/primary")).description);
    try t.expectEqualStrings("Increase selected field (E)", (try find(moved.input(), "c/hud/field/increase/primary")).description);
    for (moved.input().owner.context) |byte| try t.expectEqual(@as(u8, 'C'), byte);
    const alert = try find(moved.input(), "c/recoverable");
    try t.expectEqual(@as(usize, 16384), alert.name.len);
    try t.expectEqualStrings("α", alert.name[0..2]);
    try t.expectEqualSlices(u8, alert.name, alert.value.?);
    for (alert.name[2..]) |byte| try t.expectEqual(@as(u8, 'x'), byte);
    var slot = r.FrameSlot.init(t.allocator, &budget);
    defer slot.deinit();
    var revision: ?u64 = null;
    const frozen_current = ui.CurrentStamp{ .revision = 1, .owner = moved.input().owner };
    try ui.publishHudSettings(&slot, &revision, &moved, frozen_current);
    const frame_name_pointer = slot.view().?.nodes[0].name.ptr;
    moved.deinit();
    try t.expectEqual(frame_name_pointer, slot.view().?.nodes[0].name.ptr);
    try t.expectEqualStrings("α", slot.view().?.node(.{ .bytes = "c/recoverable" }).?.name[0..2]);
    // Invalid full inputs fail before allocation and preserve existing frame.
    const before = budget.ledger();
    try t.expectError(error.InputLimit, ui.prepareHudSettings(t.allocator, &budget, .{ .recoverable = recovery }, stamp(owner, 2)));
    try t.expectError(error.InvalidUtf8, ui.prepareHudSettings(t.allocator, &budget, .{ .recoverable = &.{0xff} }, stamp(owner, 2)));
    for ([_][]const u8{ "", "é", "alpha", " ALPHA", "ALPHA ", "A  B", "ABCDEFGHIJKLMNOPQRSTUVWXY" }) |bad| try t.expectError(error.InvalidWorldName, ui.prepareHudSettings(t.allocator, &budget, .{ .source_name = bad }, stamp(owner, 2)));
    try t.expectEqualDeep(before, budget.ledger());
    try t.expectEqual(@as(usize, 0), budget.pending_bytes);
}
const session_module = @import("classic_session.zig");
const store_module = @import("classic_store.zig");
fn checkRecovery(failure: session_module.RecoveryFailure, expected: []const u8) !void {
    var scratch: [256]u8 = undefined;
    const got = try ui.recoveryText(failure, &scratch);
    try t.expectEqualStrings(expected, got);
    try t.expect(got.len <= 128);
}
test "H3a complete closed recovery category and every numeric width" {
    const rejection_errors = [_]@import("simulation.zig").CommandRejection{ error.WrongTick, error.DuplicateSequence, error.CampaignFinished, error.InvalidWorld, error.SameSourceAndTarget, error.SourceNotOwnedByIssuer, error.InsufficientEnergy, error.FieldAtLimit };
    const rejection_text = [_][]const u8{ "REJECTED - WRONG TICK", "REJECTED - DUPLICATE COMMAND", "REJECTED - CAMPAIGN FINISHED", "REJECTED - INVALID WORLD", "REJECTED - CHOOSE ANOTHER TARGET", "REJECTED - SOURCE IS NOT UNION", "REJECTED - INSUFFICIENT ENERGY", "REJECTED - FIELD AT LIMIT" };
    for (rejection_errors, rejection_text) |cause, expected| {
        const general = session_module.GeneralFailure{ .command = cause };
        try checkRecovery(.{ .general = &general }, expected);
    }
    const command_errors = [_]@import("classic_commands.zig").DispatchError{ error.ModeRefused, error.OutOfMemory, error.MissingSource, error.MissingTarget };
    for (command_errors, [_][]const u8{ "COMMAND UNAVAILABLE IN CURRENT MODE", "COMMAND MEMORY ALLOCATION FAILED", "SELECT A SOURCE WORLD", "SELECT A TARGET WORLD" }) |cause, expected| {
        const general = session_module.GeneralFailure{ .command = cause };
        try checkRecovery(.{ .general = &general }, expected);
    }
    for ([_]@import("scenario.zig").AdmissionError{ error.Validation, error.InvalidInitialState, error.StaleFingerprint }, [_][]const u8{ "SCENARIO VALIDATION FAILED", "SCENARIO INITIAL STATE INVALID", "SCENARIO FINGERPRINT IS STALE" }) |cause, expected| {
        const general = session_module.GeneralFailure{ .admission = cause };
        try checkRecovery(.{ .general = &general }, expected);
    }
    try checkRecovery(.{ .resource = .{ .preferences = error.OutOfMemory } }, "PREFERENCE MEMORY ALLOCATION FAILED");
    try checkRecovery(.{ .resource = .{ .editor = error.UnboundedNewEditorError } }, "EDITOR OPERATION FAILED");
    try checkRecovery(.{ .resource = .{ .command = error.OutOfMemory } }, "COMMAND MEMORY ALLOCATION FAILED");
    try checkRecovery(.{ .resource = .{ .preferences = error.OutOfMemory, .editor = error.SecondEditorError, .command = error.OutOfMemory } }, "PREFERENCE MEMORY ALLOCATION FAILED | EDITOR OPERATION FAILED | COMMAND MEMORY ALLOCATION FAILED");
    try checkRecovery(.{ .tick = .{ .cause = error.OutOfMemory, .tick = 0 } }, "SIMULATION MEMORY ALLOCATION FAILED AT TICK 0");
    try checkRecovery(.{ .tick = .{ .cause = error.OutOfMemory, .tick = std.math.maxInt(u64) } }, "SIMULATION MEMORY ALLOCATION FAILED AT TICK 18446744073709551615");
    const codecs = [_]@import("preferences.zig").CodecFailure{ .too_large, .version, .{ .json = error.UnknownJsonDetail }, .{ .ui_scale = 0 }, .{ .ui_scale = 65535 } };
    for (codecs, [_][]const u8{ "preferences JSON exceeds the 16384-byte limit", "unsupported preferences format version", "invalid preferences JSON", "unsupported UI scale percentage 0", "unsupported UI scale percentage 65535" }) |codec, expected| {
        const failure = store_module.PreferencesFailure{ .codec = codec };
        try checkRecovery(.{ .preferences = &failure }, expected);
    }
    var raw_detail = "raw UTF8 α retained".*;
    for ([_]store_module.FailureKind{ .io, .before_persist, .browser, .unavailable_path }, [_][]const u8{ "PREFERENCE STORAGE FAILED", "PREFERENCE STORAGE FAILED BEFORE ATOMIC PERSIST", "BROWSER PREFERENCE STORAGE FAILED", "PREFERENCE STORAGE PATH UNAVAILABLE" }) |kind, expected| {
        const failure = store_module.PreferencesFailure{ .store = .{ .record = .preferences, .kind = kind, .cause = error.SourceCause, .cleanup_error = error.CleanupCause, .message = &raw_detail } };
        try checkRecovery(.{ .preferences = &failure }, expected);
        try t.expectEqualStrings("raw UTF8 α retained", failure.store.message.?);
        try t.expectEqual(error.SourceCause, failure.store.cause.?);
        try t.expectEqual(error.CleanupCause, failure.store.cleanup_error.?);
    }
    const zero = store_module.PreferencesFailure{ .store = .{ .record = .preferences, .kind = .oversized_slot, .max_bytes = 0 } };
    try checkRecovery(.{ .preferences = &zero }, "preference storage slot exceeds the 0-byte limit");
    const maximum = store_module.PreferencesFailure{ .store = .{ .record = .preferences, .kind = .oversized_slot, .max_bytes = std.math.maxInt(usize) } };
    try checkRecovery(.{ .preferences = &maximum }, "preference storage slot exceeds the 18446744073709551615-byte limit");
    const inconsistent = store_module.PreferencesFailure{ .store = .{ .record = .scenario, .kind = .io } };
    var scratch: [256]u8 = undefined;
    try t.expectError(error.InvalidRecoveryRecord, ui.recoveryText(.{ .preferences = &inconsistent }, &scratch));
}
test "H3a Session adapter source purity priority raw details full copied locals and repeat status" {
    var raw = store_module.MemoryStore.init(t.allocator, .preferences);
    defer raw.deinit();
    const candidate = try @import("scenario.zig").generatedValidated(@import("model.zig").DEFAULT_SEED, .{});
    var session = try session_module.Session.init(t.allocator, &candidate, raw.raw());
    defer session.deinit();
    // Source setup occurs before the read-only projection boundary.
    session.core.selected_world = 0;
    session.core.hovered_world = 1;
    session.general_failure = .{ .command = error.WrongTick };
    session.resource_errors = .{ .preferences = error.OutOfMemory, .editor = error.EditorRaw, .command = error.OutOfMemory };
    session.preference_failure = .{ .store = .{ .record = .preferences, .kind = .io, .cause = error.RawCause, .cleanup_error = error.RawCleanup, .message = try t.allocator.dupe(u8, "owned raw failure") } };
    const identity = session.core.canonicalFingerprint();
    const prefs = session.core.preferences();
    const clock = session.core.clockAlpha();
    const pending = session.core.pendingCommandCount();
    const sequence = session.core.nextSequence();
    const world = session.core.selected_world;
    const hovered = session.core.hovered_world;
    const focus = session.hud_focus;
    const guidance = session.onboarding;
    const settings = session.settings.focusIndex();
    const loads = raw.load_calls;
    const saves = raw.save_calls;
    var tokens = r.LiveTokenBudget.init(.{ .max_live_objects = 1, .max_live_requested_bytes = 1280 });
    defer tokens.deinit();
    const owner = try r.TokenRef.create(t.allocator, &tokens);
    defer owner.release();
    var budget = r.PreparationBudget.init(196608);
    defer budget.deinit();
    var slot = r.FrameSlot.init(t.allocator, &budget);
    defer slot.deinit();
    var revision: ?u64 = null;
    for (0..2) |_| {
        var p = try ui.prepareHudSettingsFromSession(t.allocator, &budget, &session, .{ .x = 600, .y = 200 }, stamp(owner, 2));
        defer p.deinit();
        try ui.publishHudSettings(&slot, &revision, &p, stamp(owner, 2));
        try t.expectEqualStrings("REJECTED - WRONG TICK", slot.view().?.node(.{ .bytes = "c/recoverable" }).?.name);
    }
    try t.expectEqual(identity, session.core.canonicalFingerprint());
    try t.expectEqualDeep(prefs, session.core.preferences());
    try t.expectEqual(clock, session.core.clockAlpha());
    try t.expectEqual(pending, session.core.pendingCommandCount());
    try t.expectEqual(sequence, session.core.nextSequence());
    try t.expectEqual(world, session.core.selected_world);
    try t.expectEqual(hovered, session.core.hovered_world);
    try t.expectEqualDeep(focus, session.hud_focus);
    try t.expectEqualDeep(guidance, session.onboarding);
    try t.expectEqual(settings, session.settings.focusIndex());
    try t.expectEqual(loads, raw.load_calls);
    try t.expectEqual(saves, raw.save_calls);
    try t.expectEqualStrings("owned raw failure", session.preference_failure.?.store.message.?);
    try t.expectEqual(error.RawCause, session.preference_failure.?.store.cause.?);
    try t.expectEqual(error.RawCleanup, session.preference_failure.?.store.cleanup_error.?);
    try t.expectEqual(error.WrongTick, session.general_failure.?.command);
    try t.expectEqual(error.EditorRaw, session.resource_errors.editor.?);
    session.general_failure = null;
    {
        var p = try ui.prepareHudSettingsFromSession(t.allocator, &budget, &session, null, stamp(owner, 3));
        defer p.deinit();
        try t.expectEqualStrings("PREFERENCE MEMORY ALLOCATION FAILED | EDITOR OPERATION FAILED | COMMAND MEMORY ALLOCATION FAILED", (try find(p.input(), "c/recoverable")).name);
    }
    session.resource_errors = .{};
    {
        var p = try ui.prepareHudSettingsFromSession(t.allocator, &budget, &session, null, stamp(owner, 4));
        defer p.deinit();
        try t.expectEqualStrings("PREFERENCE STORAGE FAILED", (try find(p.input(), "c/recoverable")).name);
    }
    // Poison original storage after a complete synchronous copy; Header survives.
    {
        var p = try ui.prepareHudSettingsFromSession(t.allocator, &budget, &session, null, stamp(owner, 5));
        defer p.deinit();
        @memset(session.preference_failure.?.store.message.?, '!');
        session.core.hovered_world = null;
        try t.expectEqualStrings("PREFERENCE STORAGE FAILED", (try find(p.input(), "c/recoverable")).name);
    }
    session.core.setMode(.editing_scenario);
    try t.expectError(error.WrongProjectionMode, ui.prepareHudSettingsFromSession(t.allocator, &budget, &session, null, stamp(owner, 6)));
}
// Qualification only: mirror the frozen Token's 40-byte target layout to inject
// its otherwise unreachable checked overflow. No production accessor is added.
const TokenLayout = struct { allocator: std.mem.Allocator, refs: usize, budget: *r.LiveTokenBudget, charged_bytes: usize };
fn tokenStorage(token: r.TokenRef) *TokenLayout {
    return @ptrCast(@alignCast(token.identity));
}
const Witness = struct { nodes: [*]const r.NodeSpec, children: [*]const usize, focus: [*]const r.ActionId, context: [*]const u8, owner: r.OwnerSpec, hash: u64, revision: ?u64 };
fn frameHash(view: r.FrameView) u64 {
    var hash = std.hash.Wyhash.init(0);
    hash.update(std.mem.sliceAsBytes(view.nodes));
    hash.update(std.mem.sliceAsBytes(view.child_indices));
    hash.update(std.mem.sliceAsBytes(view.announcements));
    for (view.nodes) |n| {
        hash.update(n.id.bytes);
        hash.update(n.name);
        hash.update(n.description);
        if (n.value) |value| hash.update(value);
        if (n.action) |action| hash.update(action.bytes);
    }
    for (view.announcements) |announcement| {
        hash.update(announcement.code);
        hash.update(announcement.message);
    }
    for (view.focus.base) |action| hash.update(action.bytes);
    if (view.focus.focused) |action| hash.update(action.bytes);
    hash.update(view.owner.context);
    return hash.final();
}
fn witness(slot: *const r.FrameSlot, revision: ?u64) Witness {
    const view = slot.view().?;
    return .{ .nodes = view.nodes.ptr, .children = view.child_indices.ptr, .focus = view.focus.base.ptr, .context = view.owner.context.ptr, .owner = view.owner, .hash = frameHash(view), .revision = revision };
}
fn unchanged(slot: *const r.FrameSlot, revision: ?u64, old: Witness) !void {
    const current = witness(slot, revision);
    try t.expectEqual(old.nodes, current.nodes);
    try t.expectEqual(old.children, current.children);
    try t.expectEqual(old.focus, current.focus);
    try t.expectEqual(old.context, current.context);
    try t.expect(old.owner.matches(current.owner));
    try t.expectEqual(old.hash, current.hash);
    try t.expectEqual(old.revision, revision);
}
const ObservedAllocator = struct {
    backing: std.mem.Allocator,
    budget: *r.PreparationBudget,
    requests: [16]usize = undefined,
    ledgers: [16]r.RequestedBytes = undefined,
    pending: [16]usize = undefined,
    count: usize = 0,
    free_calls: usize = 0,
    header_free_seen: bool = false,
    fn allocator(self: *ObservedAllocator) std.mem.Allocator {
        return .{ .ptr = self, .vtable = &.{ .alloc = alloc, .resize = resize, .remap = remap, .free = free } };
    }
    fn alloc(ctx: *anyopaque, length: usize, alignment: std.mem.Alignment, address: usize) ?[*]u8 {
        const self: *ObservedAllocator = @ptrCast(@alignCast(ctx));
        std.debug.assert(self.count < 16);
        self.requests[self.count] = length;
        self.ledgers[self.count] = self.budget.ledger();
        self.pending[self.count] = self.budget.pending_bytes;
        self.count += 1;
        // All actual requests must have been admitted before allocator entry.
        std.debug.assert(self.budget.pending_bytes >= length and self.budget.active_reservations != 0);
        return self.backing.rawAlloc(length, alignment, address);
    }
    fn resize(_: *anyopaque, _: []u8, _: std.mem.Alignment, _: usize, _: usize) bool {
        @panic("H3a unexpected resize");
    }
    fn remap(_: *anyopaque, _: []u8, _: std.mem.Alignment, _: usize, _: usize) ?[*]u8 {
        @panic("H3a unexpected remap");
    }
    fn free(ctx: *anyopaque, memory: []u8, alignment: std.mem.Alignment, address: usize) void {
        const self: *ObservedAllocator = @ptrCast(@alignCast(ctx));
        std.debug.assert(self.budget.ledger().total >= memory.len and self.budget.active_charges > 0);
        if (memory.len == ui.HEADER_BYTES) {
            // Header charge is still live when storage is destroyed.
            std.debug.assert(self.budget.ledger().frame_scratch >= ui.HEADER_BYTES);
            self.header_free_seen = true;
        }
        self.free_calls += 1;
        self.backing.rawFree(memory, alignment, address);
    }
};
fn richSource(mode: ui.ProjectionMode, recovery: []const u8) ui.ProjectionSource {
    return .{ .mode = mode, .onboarding_step = .select_union_world, .help_visible = true, .recoverable = recovery, .source_name = "ABCDEFGHIJKLMNOPQRSTUVWX", .target_name = "ABCDEFGHIJKLMNOPQRSTUVWX", .hovered_name = "ABCDEFGHIJKLMNOPQRSTUVWX", .strength = std.math.maxInt(u64), .tray_status = "COMMAND READY - EXPLICIT LAUNCH REQUIRED", .pointer = .{ .x = 600, .y = 200 }, .hud_focus = .speed_down, .settings_focus = 5, .announcements = &.{ .ready, .preferences_changed } };
}
test "H3a actual Header plus all frame allocation ordinal OOM sweeps two mode transitions" {
    try t.expectEqual(@as(usize, 40), @sizeOf(TokenLayout));
    var tokens = r.LiveTokenBudget.init(.{ .max_live_objects = 1, .max_live_requested_bytes = 1280 });
    defer tokens.deinit();
    const owner = try r.TokenRef.create(t.allocator, &tokens);
    defer owner.release();
    const recovery = try t.allocator.alloc(u8, 16384);
    defer t.allocator.free(recovery);
    @memset(recovery, 'x');
    for ([_]ui.ProjectionMode{ .playing, .settings }) |target_mode| {
        var actual_count: usize = 0;
        // First run is successful, measuring every actual allocation ordinal.
        {
            var budget = r.PreparationBudget.init(196608);
            defer budget.deinit();
            var slot = r.FrameSlot.init(t.allocator, &budget);
            defer slot.deinit();
            var revision: ?u64 = null;
            {
                var old = try ui.prepareHudSettings(t.allocator, &budget, richSource(if (target_mode == .playing) .settings else .playing, recovery), stamp(owner, 1));
                defer old.deinit();
                try ui.publishHudSettings(&slot, &revision, &old, stamp(owner, 1));
            }
            const old_bytes = budget.ledger().resident_frame;
            var observed = ObservedAllocator{ .backing = t.allocator, .budget = &budget };
            slot.allocator = observed.allocator();
            var prepared = try ui.prepareHudSettings(observed.allocator(), &budget, richSource(target_mode, recovery), stamp(owner, 2));
            try t.expectEqual(ui.HEADER_BYTES, budget.ledger().frame_scratch);
            try t.expectEqual(@as(usize, 3), tokenStorage(owner).refs);
            try ui.publishHudSettings(&slot, &revision, &prepared, stamp(owner, 2));
            actual_count = observed.count;
            try t.expectEqual(ui.HEADER_BYTES, observed.requests[0]);
            try t.expectEqual(old_bytes, observed.ledgers[0].resident_frame);
            try t.expectEqual(@as(usize, 0), observed.ledgers[0].frame_scratch);
            try t.expectEqual(ui.HEADER_BYTES, observed.pending[0]);
            for (observed.ledgers[1..actual_count]) |ledger| {
                try t.expectEqual(old_bytes, ledger.resident_frame);
                try t.expect(ledger.frame_scratch >= ui.HEADER_BYTES);
                try t.expectEqual(@as(usize, 0), ledger.prepared_focus);
                try t.expectEqual(@as(usize, 0), ledger.staged_focus);
                try t.expectEqual(@as(usize, 0), ledger.focus_scratch);
                try t.expect(ledger.total <= 186448);
            }
            std.debug.print("H3a target={s} actual requests={any}; old R={d} candidate R={d} Header={d} observed peak={d}; populations 0 prepared_focus/staged_focus/focus_scratch\n", .{ @tagName(target_mode), observed.requests[0..actual_count], old_bytes, budget.ledger().resident_frame, ui.HEADER_BYTES, budget.ledger().peak });
            prepared.deinit();
            try t.expect(observed.header_free_seen);
            try t.expectEqual(@as(usize, 0), budget.ledger().frame_scratch);
            try t.expectEqual(budget.ledger().resident_frame, budget.ledger().total);
            slot.deinit();
            slot = r.FrameSlot.init(t.allocator, &budget);
            try t.expectEqual(@as(usize, 1), tokenStorage(owner).refs);
        }
        for (0..actual_count) |ordinal| {
            var budget = r.PreparationBudget.init(196608);
            defer budget.deinit();
            var slot = r.FrameSlot.init(t.allocator, &budget);
            defer slot.deinit();
            var revision: ?u64 = null;
            {
                var old = try ui.prepareHudSettings(t.allocator, &budget, richSource(if (target_mode == .playing) .settings else .playing, recovery), stamp(owner, 1));
                defer old.deinit();
                try ui.publishHudSettings(&slot, &revision, &old, stamp(owner, 1));
            }
            const old = witness(&slot, revision);
            const old_ledger = budget.ledger();
            const token_usage = tokens.usage();
            var failing = std.testing.FailingAllocator.init(t.allocator, .{ .fail_index = ordinal });
            var observed = ObservedAllocator{ .backing = failing.allocator(), .budget = &budget };
            slot.allocator = observed.allocator();
            var prepared: ?ui.PreparedProjection = null;
            if (ui.prepareHudSettings(observed.allocator(), &budget, richSource(target_mode, recovery), stamp(owner, 2))) |value| {
                prepared = value;
                try t.expectError(error.OutOfMemory, ui.publishHudSettings(&slot, &revision, &prepared.?, stamp(owner, 2)));
                try t.expectEqual(@as(usize, 3), tokenStorage(owner).refs);
                const ledger = budget.ledger();
                try t.expectEqual(old_ledger.resident_frame, ledger.resident_frame);
                try t.expectEqual(ui.HEADER_BYTES, ledger.frame_scratch);
                try t.expectEqual(old_ledger.total + ui.HEADER_BYTES, ledger.total);
                try t.expectEqual(@as(usize, 0), ledger.staged_frame);
                try t.expectEqual(@as(usize, 0), ledger.prepared_focus);
                try t.expectEqual(@as(usize, 0), ledger.staged_focus);
                try t.expectEqual(@as(usize, 0), ledger.focus_scratch);
                try t.expectEqualStrings("NYON", prepared.?.input().nodes[0].name);
            } else |failure| {
                try t.expectEqual(error.OutOfMemory, failure);
                try t.expectEqual(@as(usize, 0), ordinal);
                try t.expectEqualDeep(old_ledger, budget.ledger());
                try t.expectEqual(@as(usize, 2), tokenStorage(owner).refs);
            }
            try t.expect(failing.has_induced_failure);
            try t.expectEqual(@as(usize, 0), budget.pending_bytes);
            try t.expectEqual(@as(usize, 0), budget.active_reservations);
            try unchanged(&slot, revision, old);
            try t.expectEqualDeep(token_usage, tokens.usage());
            if (prepared) |*value| {
                failing.fail_index = std.math.maxInt(usize);
                // Reset trace capacity for retry; original failure ordinal already
                // witnessed, all actual retry requests are admitted separately.
                observed.count = 0;
                try ui.publishHudSettings(&slot, &revision, value, stamp(owner, 2));
                try t.expectEqual(@as(?u64, 2), revision);
                value.deinit();
                try t.expectEqual(@as(usize, 0), budget.ledger().frame_scratch);
            } else {
                failing.fail_index = std.math.maxInt(usize);
                observed.count = 0;
                var retry = try ui.prepareHudSettings(observed.allocator(), &budget, richSource(target_mode, recovery), stamp(owner, 2));
                try ui.publishHudSettings(&slot, &revision, &retry, stamp(owner, 2));
                retry.deinit();
                try t.expectEqual(@as(?u64, 2), revision);
            }
            for (recovery) |byte| try t.expectEqual(@as(u8, 'x'), byte);
            slot.deinit();
            slot = r.FrameSlot.init(t.allocator, &budget);
            try t.expectEqual(@as(usize, 1), tokenStorage(owner).refs);
            try t.expectEqual(failing.allocated_bytes, failing.freed_bytes);
            try t.expectEqual(@as(usize, 0), budget.ledger().total);
        }
        std.debug.print("H3a actual OOM ordinals 0..{d} target={s}: full old frame/scalar/focus/backing/token/source preservation, retry and cleanup passed\n", .{ actual_count - 1, @tagName(target_mode) });
    }
}
test "H3a budget identity revision every owner field and checked same-owner retain unwind" {
    var tokens = r.LiveTokenBudget.init(.{ .max_live_objects = 2, .max_live_requested_bytes = 1280 });
    defer tokens.deinit();
    const owner = try r.TokenRef.create(t.allocator, &tokens);
    defer owner.release();
    const other_owner = try r.TokenRef.create(t.allocator, &tokens);
    defer other_owner.release();
    var budget = r.PreparationBudget.init(196608);
    defer budget.deinit();
    var other_budget = r.PreparationBudget.init(196608);
    defer other_budget.deinit();
    var slot = r.FrameSlot.init(t.allocator, &budget);
    defer slot.deinit();
    var revision: ?u64 = null;
    {
        var old = try ui.prepareHudSettings(t.allocator, &budget, .{ .hud_focus = .toggle_help }, stamp(owner, 1));
        defer old.deinit();
        try ui.publishHudSettings(&slot, &revision, &old, stamp(owner, 1));
    }
    const old = witness(&slot, revision);
    {
        var p = try ui.prepareHudSettings(t.allocator, &other_budget, .{ .mode = .settings }, stamp(owner, 2));
        defer p.deinit();
        try t.expectError(error.BudgetMismatch, ui.publishHudSettings(&slot, &revision, &p, stamp(owner, 2)));
        try unchanged(&slot, revision, old);
        try t.expectEqual(ui.HEADER_BYTES, other_budget.ledger().frame_scratch);
    }
    const before = budget.ledger();
    tokenStorage(owner).refs = std.math.maxInt(usize);
    try t.expectError(error.ReferenceOverflow, ui.prepareHudSettings(t.allocator, &budget, .{}, stamp(owner, 2)));
    try t.expectEqual(std.math.maxInt(usize), tokenStorage(owner).refs);
    tokenStorage(owner).refs = 2;
    try t.expectEqual(before.total, budget.ledger().total);
    try t.expectEqual(@as(usize, 0), budget.ledger().frame_scratch);
    try t.expectEqual(@as(usize, 0), budget.pending_bytes);
    try unchanged(&slot, revision, old);
    var p = try ui.prepareHudSettings(t.allocator, &budget, .{ .mode = .settings }, stamp(owner, 2));
    defer p.deinit();
    const candidates = [_]ui.CurrentStamp{
        stamp(owner, 3),                                                                                               stamp(other_owner, 2),
        .{ .revision = 2, .owner = .{ .resident = owner, .hold = 1, .context = "classic", .draft = null } },           .{ .revision = 2, .owner = .{ .resident = owner, .hold = null, .context = "other", .draft = null } },
        .{ .revision = 2, .owner = .{ .resident = owner, .hold = null, .context = "classic", .draft = other_owner } },
    };
    for (candidates) |current| {
        try t.expectError(error.StaleProjection, ui.publishHudSettings(&slot, &revision, &p, current));
        try unchanged(&slot, revision, old);
        try t.expectEqual(ui.HEADER_BYTES, budget.ledger().frame_scratch);
    }
    // The old HUD remains published, but caller-current Settings revision blocks
    // stale HUD activation even when a failed projection leaves it visible.
    try t.expectError(error.StaleProjection, ui.validateAction(slot.view().?, revision.?, stamp(owner, 2), .{ .bytes = "a/hud/help" }));
    tokenStorage(owner).refs = std.math.maxInt(usize);
    try t.expectError(error.ReferenceOverflow, ui.publishHudSettings(&slot, &revision, &p, stamp(owner, 2)));
    try t.expectEqual(std.math.maxInt(usize), tokenStorage(owner).refs);
    tokenStorage(owner).refs = 3;
    try unchanged(&slot, revision, old);
    try t.expectEqual(@as(usize, 0), budget.ledger().staged_frame);
    try t.expectEqual(ui.HEADER_BYTES, budget.ledger().frame_scratch);
    for (0..3) |_| {
        try ui.publishHudSettings(&slot, &revision, &p, stamp(owner, 2));
        try t.expectEqual(@as(usize, 3), tokenStorage(owner).refs);
        try t.expectEqual(@as(?u64, 2), revision);
        try t.expectError(error.UnknownAction, ui.validateAction(slot.view().?, revision.?, stamp(owner, 2), .{ .bytes = "a/hud/help" }));
        try t.expectError(error.UnknownAction, ui.validateAction(slot.view().?, revision.?, stamp(owner, 2), .{ .bytes = "a/settings/nonexistent" }));
    }
    // A separately retained old identity cannot become current by token reuse.
    const stale = try owner.retain();
    defer stale.release();
    try t.expectError(error.StaleProjection, ui.validateAction(slot.view().?, revision.?, stamp(other_owner, 2), .{ .bytes = "a/settings/close" }));
    var empty: ui.PreparedProjection = .{};
    try t.expectError(error.EmptyProjection, ui.publishHudSettings(&slot, &revision, &empty, stamp(owner, 2)));
}
test "H3a exact and one-less all eight candidate limits and Header admission" {
    var tokens = r.LiveTokenBudget.init(.{ .max_live_objects = 1, .max_live_requested_bytes = 1280 });
    defer tokens.deinit();
    const owner = try r.TokenRef.create(t.allocator, &tokens);
    defer owner.release();
    var budget = r.PreparationBudget.init(196608);
    defer budget.deinit();
    var small = r.PreparationBudget.init(ui.HEADER_BYTES - 1);
    defer small.deinit();
    try t.expectError(error.LimitExceeded, ui.prepareHudSettings(t.allocator, &small, .{}, stamp(owner, 1)));
    try t.expectEqual(@as(usize, 0), small.pending_bytes);
    try t.expectEqual(@as(usize, 0), small.active_charges);
    var exact_header = r.PreparationBudget.init(ui.HEADER_BYTES);
    defer exact_header.deinit();
    {
        var p = try ui.prepareHudSettings(t.allocator, &exact_header, .{}, stamp(owner, 1));
        defer p.deinit();
        try t.expectEqual(ui.HEADER_BYTES, exact_header.ledger().total);
    }
    try t.expectError(error.InputLimit, ui.prepareHudSettings(t.allocator, &budget, .{ .announcements = &.{ .ready, .ready, .preferences_changed } }, stamp(owner, 1)));
    var too_long: [129]u8 = @splat('C');
    try t.expectError(error.InputLimit, ui.prepareHudSettings(t.allocator, &budget, .{}, .{ .revision = 1, .owner = .{ .resident = owner, .hold = null, .context = &too_long, .draft = null } }));
    try t.expectError(error.InvalidUtf8, ui.prepareHudSettings(t.allocator, &budget, .{}, .{ .revision = 1, .owner = .{ .resident = owner, .hold = null, .context = &.{0xff}, .draft = null } }));
    const recovery = try t.allocator.alloc(u8, 16384);
    defer t.allocator.free(recovery);
    @memset(recovery, 'x');
    var p = try ui.prepareHudSettings(t.allocator, &budget, richSource(.playing, recovery), stamp(owner, 1));
    defer p.deinit();
    const input = p.input();
    var exact = ui.limits;
    exact.max_nodes = input.nodes.len;
    exact.max_child_indices = input.child_indices.len;
    exact.max_announcements = input.announcements.len;
    exact.max_focus_actions = input.focus.base.len;
    exact.max_owner_context_bytes = input.owner.context.len;
    exact.max_id_bytes = 0;
    exact.max_text_bytes = 0;
    for (input.nodes) |n| {
        exact.max_id_bytes += n.id.bytes.len;
        if (n.action) |action| exact.max_id_bytes += action.bytes.len;
        exact.max_text_bytes += n.name.len + n.description.len;
        if (n.value) |value| exact.max_text_bytes += value.len;
    }
    for (input.focus.base) |action| exact.max_id_bytes += action.bytes.len;
    for (input.announcements) |announcement| exact.max_text_bytes += announcement.code.len + announcement.message.len;
    var slot = r.FrameSlot.init(t.allocator, &budget);
    defer slot.deinit();
    var revision: ?u64 = null;
    try ui.publishHudSettings(&slot, &revision, &p, stamp(owner, 1));
    exact.max_owned_bytes = slot.frame.?.requested + input.nodes.len * 8 + (input.focus.base.len + @intFromBool(input.focus.focused != null)) * 16;
    try slot.replace(input, exact);
    const old = witness(&slot, revision);
    inline for (.{ "max_nodes", "max_child_indices", "max_id_bytes", "max_text_bytes", "max_focus_actions", "max_announcements", "max_owner_context_bytes", "max_owned_bytes" }) |field| {
        var low = exact;
        @field(low, field) -= 1;
        const old_bytes = budget.ledger().total;
        try t.expectError(error.LimitExceeded, slot.replace(input, low));
        try unchanged(&slot, revision, old);
        try t.expectEqual(old_bytes, budget.ledger().total);
        try t.expectEqual(@as(usize, 0), budget.pending_bytes);
        try t.expectEqual(@as(usize, 0), budget.ledger().staged_frame);
    }
    try t.expectEqual(@as(usize, 72), ui.limits.max_nodes);
    try t.expectEqual(@as(usize, 71), ui.limits.max_child_indices);
    try t.expectEqual(@as(usize, 12288), ui.limits.max_id_bytes);
    try t.expectEqual(@as(usize, 49152), ui.limits.max_text_bytes);
    try t.expectEqual(@as(usize, 18), ui.limits.max_focus_actions);
    try t.expectEqual(@as(usize, 2), ui.limits.max_announcements);
    try t.expectEqual(@as(usize, 128), ui.limits.max_owner_context_bytes);
    try t.expectEqual(@as(usize, 81920), ui.limits.max_owned_bytes);
    std.debug.print("H3a actual exact eight limits={any}; each one-less rejects transactionally\n", .{exact});
}
test "H3a actual Session tick priority preserves hold and local name lifetimes" {
    var raw = store_module.MemoryStore.init(t.allocator, .preferences);
    defer raw.deinit();
    var failing = std.testing.FailingAllocator.init(t.allocator, .{});
    const candidate = try @import("scenario.zig").generatedValidated(@import("model.zig").DEFAULT_SEED, .{});
    var session = try session_module.Session.init(failing.allocator(), &candidate, raw.raw());
    defer session.deinit();
    session.core.setOnboardingHold(false);
    _ = session.core.setSpeed(.normal);
    _ = try session.queueGameCommand(.{ .launch = .{ .source = 0, .destination = 1 } });
    failing.fail_index = failing.alloc_index;
    _ = session.core.advance(1_000_000_000);
    const tick = session.core.recoverableStatus().?;
    try t.expectEqual(@as(u64, 0), tick.tick);
    session.core.selected_world = 0;
    session.core.hovered_world = 1;
    session.general_failure = .{ .command = error.WrongTick };
    session.resource_errors.editor = error.EditorPriority;
    session.preference_failure = .{ .codec = .version };
    var tokens = r.LiveTokenBudget.init(.{ .max_live_objects = 1, .max_live_requested_bytes = 1280 });
    defer tokens.deinit();
    const owner = try r.TokenRef.create(t.allocator, &tokens);
    defer owner.release();
    var budget = r.PreparationBudget.init(196608);
    defer budget.deinit();
    const fingerprint = session.core.canonicalFingerprint();
    const pending = session.core.pendingCommandCount();
    {
        var p = try ui.prepareHudSettingsFromSession(t.allocator, &budget, &session, null, stamp(owner, 1));
        defer p.deinit();
        try t.expectEqualStrings("REJECTED - WRONG TICK", (try find(p.input(), "c/recoverable")).name);
    }
    session.general_failure = null;
    {
        var p = try ui.prepareHudSettingsFromSession(t.allocator, &budget, &session, null, stamp(owner, 1));
        defer p.deinit();
        try t.expectEqualStrings("EDITOR OPERATION FAILED", (try find(p.input(), "c/recoverable")).name);
    }
    session.resource_errors = .{};
    var p = try ui.prepareHudSettingsFromSession(t.allocator, &budget, &session, .{ .x = 600, .y = 200 }, stamp(owner, 1));
    defer p.deinit();
    try t.expectEqualStrings("SIMULATION MEMORY ALLOCATION FAILED AT TICK 0", (try find(p.input(), "c/recoverable")).name);
    try t.expectEqualStrings("SOURCE ASTER VALE", (try find(p.input(), "c/hud/tray/source")).name);
    try t.expectEqualStrings("TARGET KHEPRI", (try find(p.input(), "c/hud/tray/target")).name);
    try t.expectEqualStrings("WORLD KHEPRI - SELECT OR PREVIEW DESTINATION", (try find(p.input(), "c/tooltip")).name);
    try t.expectEqualDeep(tick, session.core.recoverableStatus().?);
    try t.expectEqual(fingerprint, session.core.canonicalFingerprint());
    try t.expectEqual(pending, session.core.pendingCommandCount());
    try t.expectEqual(@as(usize, 0), raw.save_calls);
    // Mutating source only AFTER the read-only call demonstrates expired local
    // Controls/Tray/value WorldNames do not back the returned stable Header.
    session.core.selected_world = null;
    session.core.hovered_world = null;
    try t.expectEqualStrings("SOURCE ASTER VALE", (try find(p.input(), "c/hud/tray/source")).name);
    try t.expectEqualStrings("TARGET KHEPRI", (try find(p.input(), "c/hud/tray/target")).name);
    try t.expectEqualStrings("WORLD KHEPRI - SELECT OR PREVIEW DESTINATION", (try find(p.input(), "c/tooltip")).name);
}
test "H3a tooltip disabled hit fallback hidden focus and missing-current navigation" {
    var tokens = r.LiveTokenBudget.init(.{ .max_live_objects = 1, .max_live_requested_bytes = 1280 });
    defer tokens.deinit();
    const owner = try r.TokenRef.create(t.allocator, &tokens);
    defer owner.release();
    var budget = r.PreparationBudget.init(196608);
    defer budget.deinit();
    {
        var p = try ui.prepareHudSettings(t.allocator, &budget, .{ .onboarding_step = .select_union_world, .pointer = .{ .x = 285, .y = 850 }, .hovered_name = "KHEPRI" }, stamp(owner, 1));
        defer p.deinit();
        try t.expect(ui.hitTest(&p, .{ .x = 285, .y = 850 }) == null);
        try t.expectEqualStrings("Decrease game speed ([)", (try find(p.input(), "c/tooltip")).name);
        try t.expect(ui.consumesPointer(&p, .{ .x = 285, .y = 850 }));
        try t.expectEqualStrings("a/hud/settings", ui.hitTest(&p, .{ .x = 900, .y = 860 }).?.bytes);
    }
    var p = try ui.prepareHudSettings(t.allocator, &budget, .{ .hud_focus = .skip_onboarding }, stamp(owner, 1));
    defer p.deinit();
    try t.expect(p.input().focus.focused == null);
    var next = try r.OwnedFocus.init(t.allocator, p.input().focus, ui.limits, &budget);
    defer next.deinit();
    var previous = try r.OwnedFocus.init(t.allocator, p.input().focus, ui.limits, &budget);
    defer previous.deinit();
    // H1 defaults an absent focused action to its first action. Session
    // separately owns missing-current source navigation (qualified below).
    try t.expectEqualStrings("a/hud/pause", next.next().?.bytes);
    try t.expectEqualStrings("a/hud/advisory_ack", previous.previous().?.bytes);
    try t.expect(ui.hitTest(&p, .{ .x = std.math.inf(f32), .y = 0 }) == null);
    var raw = store_module.MemoryStore.init(t.allocator, .preferences);
    defer raw.deinit();
    const candidate = try @import("scenario.zig").generatedValidated(@import("model.zig").DEFAULT_SEED, .{});
    var session = try session_module.Session.init(t.allocator, &candidate, raw.raw());
    defer session.deinit();
    _ = session.onboarding.skip();
    const controls = session.controls(null);
    session.hud_focus = .skip_onboarding;
    try t.expect(session.focusIndex(controls.items()) == null);
    session.focusNext(false, controls.items());
    try t.expectEqualDeep(session_module.UiAction.speed_down, session.hud_focus.?);
    session.hud_focus = .skip_onboarding;
    session.focusNext(true, controls.items());
    try t.expectEqualDeep(session_module.UiAction.acknowledge_advisory, session.hud_focus.?);
    try t.expectEqual(@as(usize, 0), session.core.pendingCommandCount());
    try t.expectEqual(@as(usize, 0), raw.save_calls);
}

const start_marker = @import("classic_start_marker.zig");
fn richMarker() start_marker.Marker {
    return .{ .world = 0, .name = @import("model.zig").WorldName.init("ABCDEFGHIJKLMNOPQRSTUVWX") catch unreachable, .point = .{ .x = 400, .y = 400 }, .label = .{ .min = .{ 173.8, 225.64 }, .max = .{ 173.8 + @as(f32, 180), 225.64 + @as(f32, 54) } }, .mode = .action_only, .connector_start = null, .ring_radius = null };
}
test "H3c actual retained marker OOM every ordinal and exact shared peak boundary" {
    var tokens = r.LiveTokenBudget.init(.{ .max_live_objects = 1, .max_live_requested_bytes = 1280 });
    defer tokens.deinit();
    const owner = try r.TokenRef.create(t.allocator, &tokens);
    defer owner.release();
    var recovery: [16384]u8 = @splat('x');
    for ([_]ui.ProjectionMode{ .playing, .settings }) |old_mode| {
        var count: usize = 0;
        var peak: usize = 0;
        {
            var budget = r.PreparationBudget.init(1179648);
            defer budget.deinit();
            var slot = r.FrameSlot.init(t.allocator, &budget);
            defer slot.deinit();
            var revision: ?u64 = null;
            {
                var old = try ui.prepareHudSettings(t.allocator, &budget, richSource(old_mode, &recovery), stamp(owner, 1));
                defer old.deinit();
                try ui.publishHudSettings(&slot, &revision, &old, stamp(owner, 1));
            }
            var observed = ObservedAllocator{ .backing = t.allocator, .budget = &budget };
            slot.allocator = observed.allocator();
            var candidate = try ui.prepareHudSettings(observed.allocator(), &budget, richSource(.playing, &recovery), stamp(owner, 2));
            const ledger_before = budget.ledger();
            const refs_before = tokenStorage(owner).refs;
            try ui.attachStartMarker(&candidate, .{ .marker = richMarker() });
            try t.expectEqualDeep(ledger_before, budget.ledger());
            try t.expectEqual(refs_before, tokenStorage(owner).refs);
            try t.expectEqual(@as(usize, 1), observed.count);
            try ui.publishHudSettings(&slot, &revision, &candidate, stamp(owner, 2));
            count = observed.count;
            peak = budget.ledger().peak;
            try t.expectEqual(@as(usize, 67), slot.view().?.nodes.len);
            std.debug.print("H3c REAL old={s} requests={any} P={d} resident={d} Header={d}\n", .{ @tagName(old_mode), observed.requests[0..count], peak, budget.ledger().resident_frame, ui.HEADER_BYTES });
            for (observed.requests[0..count], observed.ledgers[0..count], observed.pending[0..count], 0..) |request, ledger, pending, ordinal| std.debug.print("H3c TRACE ordinal={d} request={d} live={d} resident={d} staged={d} scratch={d} pending={d}\n", .{ ordinal, request, ledger.total, ledger.resident_frame, ledger.staged_frame, ledger.frame_scratch, pending });
            candidate.deinit();
            slot.deinit();
            slot = r.FrameSlot.init(t.allocator, &budget);
            try t.expectEqual(@as(usize, 0), budget.ledger().total);
            try t.expectEqual(@as(usize, 1), tokenStorage(owner).refs);
        }
        for (0..count) |ordinal| {
            var budget = r.PreparationBudget.init(1179648);
            defer budget.deinit();
            var slot = r.FrameSlot.init(t.allocator, &budget);
            defer slot.deinit();
            var revision: ?u64 = null;
            {
                var old = try ui.prepareHudSettings(t.allocator, &budget, richSource(old_mode, &recovery), stamp(owner, 1));
                defer old.deinit();
                try ui.publishHudSettings(&slot, &revision, &old, stamp(owner, 1));
            }
            const old = witness(&slot, revision);
            const old_ledger = budget.ledger();
            const token_usage = tokens.usage();
            var failing = t.FailingAllocator.init(t.allocator, .{ .fail_index = ordinal });
            var observed = ObservedAllocator{ .backing = failing.allocator(), .budget = &budget };
            slot.allocator = observed.allocator();
            var candidate: ?ui.PreparedProjection = null;
            if (ui.prepareHudSettings(observed.allocator(), &budget, richSource(.playing, &recovery), stamp(owner, 2))) |prepared| {
                candidate = prepared;
                try ui.attachStartMarker(&candidate.?, .{ .marker = richMarker() });
                try t.expectError(error.OutOfMemory, ui.publishHudSettings(&slot, &revision, &candidate.?, stamp(owner, 2)));
                try t.expectEqual(old_ledger.total + ui.HEADER_BYTES, budget.ledger().total);
                try t.expectEqual(@as(usize, 3), tokenStorage(owner).refs);
            } else |err| {
                try t.expectEqual(error.OutOfMemory, err);
                try t.expectEqual(@as(usize, 0), ordinal);
                try t.expectEqualDeep(old_ledger, budget.ledger());
            }
            try t.expect(failing.has_induced_failure);
            try unchanged(&slot, revision, old);
            try t.expectEqualDeep(token_usage, tokens.usage());
            try t.expectEqual(@as(usize, 0), budget.pending_bytes);
            try t.expectEqual(@as(usize, 0), budget.active_reservations);
            failing.fail_index = std.math.maxInt(usize);
            observed.count = 0;
            if (candidate == null) {
                candidate = try ui.prepareHudSettings(observed.allocator(), &budget, richSource(.playing, &recovery), stamp(owner, 2));
                try ui.attachStartMarker(&candidate.?, .{ .marker = richMarker() });
            }
            try ui.publishHudSettings(&slot, &revision, &candidate.?, stamp(owner, 2));
            candidate.?.deinit();
            try t.expectEqual(@as(?u64, 2), revision);
            slot.deinit();
            slot = r.FrameSlot.init(t.allocator, &budget);
            try t.expectEqual(@as(usize, 0), budget.ledger().total);
            try t.expectEqual(@as(usize, 1), tokenStorage(owner).refs);
            try t.expectEqual(failing.allocated_bytes, failing.freed_bytes);
            for (recovery) |byte| try t.expectEqual(@as(u8, 'x'), byte);
        }
        for ([_]usize{ peak, peak - 1 }) |cap| {
            var budget = r.PreparationBudget.init(cap);
            defer budget.deinit();
            var slot = r.FrameSlot.init(t.allocator, &budget);
            defer slot.deinit();
            var revision: ?u64 = null;
            {
                var old = try ui.prepareHudSettings(t.allocator, &budget, richSource(old_mode, &recovery), stamp(owner, 1));
                defer old.deinit();
                try ui.publishHudSettings(&slot, &revision, &old, stamp(owner, 1));
            }
            const old = witness(&slot, revision);
            var candidate = try ui.prepareHudSettings(t.allocator, &budget, richSource(.playing, &recovery), stamp(owner, 2));
            defer candidate.deinit();
            try ui.attachStartMarker(&candidate, .{ .marker = richMarker() });
            if (cap == peak) {
                try ui.publishHudSettings(&slot, &revision, &candidate, stamp(owner, 2));
                try t.expectEqual(peak, budget.ledger().peak);
            } else {
                try t.expectError(error.LimitExceeded, ui.publishHudSettings(&slot, &revision, &candidate, stamp(owner, 2)));
                try unchanged(&slot, revision, old);
                try t.expectEqual(@as(usize, 0), budget.pending_bytes);
            }
        }
        std.debug.print("H3c every real OOM ordinal 0..{d} old={s} preserved/retry/cleanup; P={d} pass P-1={d} fail\n", .{ count - 1, @tagName(old_mode), peak, peak - 1 });
    }
}
test "H3c failed publication permits attach and real retain overflow unwind" {
    var tokens = r.LiveTokenBudget.init(.{ .max_live_objects = 1, .max_live_requested_bytes = 1280 });
    defer tokens.deinit();
    const owner = try r.TokenRef.create(t.allocator, &tokens);
    defer owner.release();
    var budget = r.PreparationBudget.init(1179648);
    defer budget.deinit();
    var slot = r.FrameSlot.init(t.allocator, &budget);
    defer slot.deinit();
    var revision: ?u64 = null;
    {
        var old = try ui.prepareHudSettings(t.allocator, &budget, .{}, stamp(owner, 1));
        defer old.deinit();
        try ui.publishHudSettings(&slot, &revision, &old, stamp(owner, 1));
    }
    const old = witness(&slot, revision);
    var candidate = try ui.prepareHudSettings(t.allocator, &budget, .{}, stamp(owner, 2));
    defer candidate.deinit();
    var failing = t.FailingAllocator.init(t.allocator, .{ .fail_index = 0 });
    slot.allocator = failing.allocator();
    try t.expectError(error.OutOfMemory, ui.publishHudSettings(&slot, &revision, &candidate, stamp(owner, 2)));
    try unchanged(&slot, revision, old);
    try ui.attachStartMarker(&candidate, .{ .marker = richMarker() });
    failing.fail_index = std.math.maxInt(usize);
    const refs = tokenStorage(owner).refs;
    tokenStorage(owner).refs = std.math.maxInt(usize);
    try t.expectError(error.ReferenceOverflow, ui.publishHudSettings(&slot, &revision, &candidate, stamp(owner, 2)));
    try t.expectEqual(std.math.maxInt(usize), tokenStorage(owner).refs);
    tokenStorage(owner).refs = refs;
    try unchanged(&slot, revision, old);
    try t.expectEqual(@as(usize, 0), budget.pending_bytes);
    try ui.publishHudSettings(&slot, &revision, &candidate, stamp(owner, 2));
    for (0..20) |_| try ui.publishHudSettings(&slot, &revision, &candidate, stamp(owner, 2));
    try t.expectEqual(@as(usize, 3), tokenStorage(owner).refs);
    candidate.deinit();
    slot.deinit();
    slot = r.FrameSlot.init(t.allocator, &budget);
    try t.expectEqual(@as(usize, 0), budget.ledger().total);
    try t.expectEqual(@as(usize, 1), tokenStorage(owner).refs);
}

test "I3 HUD focus setter canonical marker from mutable caller bytes" {
    var tokens = r.LiveTokenBudget.init(.{ .max_live_objects = 1, .max_live_requested_bytes = 1280 });
    defer tokens.deinit();
    const owner = try r.TokenRef.create(t.allocator, &tokens);
    defer owner.release();
    var budget = r.PreparationBudget.init(196608);
    defer budget.deinit();
    var p = try ui.prepareHudSettings(t.allocator, &budget, .{}, stamp(owner, 1));
    defer p.deinit();
    var m = richMarker();
    m.world = 3;
    try ui.attachStartMarker(&p, .{ .marker = m });
    var caller = "a/hud/marker/3".*;
    try p.setHudPublishedFocus(.{ .bytes = &caller });
    try t.expect(p.input().focus.focused != null);
    const focused = p.input().focus.focused.?;
    try t.expectEqualStrings("a/hud/marker/3", focused.bytes);
    try t.expect(focused.bytes.ptr == p.input().focus.base[p.input().focus.base.len - 1].bytes.ptr);
    try t.expect(focused.bytes.ptr != &caller);
    @memset(&caller, 'x');
    try t.expectEqualStrings("a/hud/marker/3", p.input().focus.focused.?.bytes);
}

test "I3 HUD focus setter zero heap ref budget delta disabled null and exact marker payload" {
    var tokens = r.LiveTokenBudget.init(.{ .max_live_objects = 1, .max_live_requested_bytes = 1280 });
    defer tokens.deinit();
    const owner = try r.TokenRef.create(t.allocator, &tokens);
    defer owner.release();
    var budget = r.PreparationBudget.init(196608);
    defer budget.deinit();
    var observed = ObservedAllocator{ .backing = t.allocator, .budget = &budget };
    var p = try ui.prepareHudSettings(observed.allocator(), &budget, .{}, stamp(owner, 1));
    defer p.deinit();
    var m = richMarker();
    m.world = 3;
    try ui.attachStartMarker(&p, .{ .marker = m });
    const ledger = budget.ledger();
    const pending = budget.pending_bytes;
    const reservations = budget.active_reservations;
    const charges = budget.active_charges;
    const usage = tokens.usage();
    const refs = tokenStorage(owner).refs;
    const free_calls = observed.free_calls;
    const nodes = p.input().nodes.ptr;
    const children = p.input().child_indices.ptr;
    const base = p.input().focus.base.ptr;
    const node_count = p.input().nodes.len;
    const focus_count = p.input().focus.base.len;
    const geometry = try p.geometry();
    // No input borrow is used across the mutation; these are copied identities.
    try p.setHudPublishedFocus(.{ .bytes = "a/hud/marker/3" });
    try t.expectEqualStrings("a/hud/marker/3", p.input().focus.focused.?.bytes);
    try t.expectError(error.InvalidFocus, p.setHudPublishedFocus(.{ .bytes = "a/hud/marker/4" }));
    try p.setHudPublishedFocus(null);
    try t.expect(p.input().focus.focused == null);
    try p.setHudPublishedFocus(.{ .bytes = "a/hud/launch" });
    try t.expectEqualStrings("a/hud/launch", p.input().focus.focused.?.bytes);
    try t.expectEqual(@as(usize, 33312), ui.HEADER_BYTES);
    try t.expectEqual(@as(usize, 1), observed.count);
    try t.expectEqual(ui.HEADER_BYTES, observed.requests[0]);
    try t.expectEqual(free_calls, observed.free_calls);
    try t.expectEqualDeep(ledger, budget.ledger());
    try t.expectEqual(pending, budget.pending_bytes);
    try t.expectEqual(reservations, budget.active_reservations);
    try t.expectEqual(charges, budget.active_charges);
    try t.expectEqualDeep(usage, tokens.usage());
    try t.expectEqual(refs, tokenStorage(owner).refs);
    try t.expectEqual(nodes, p.input().nodes.ptr);
    try t.expectEqual(children, p.input().child_indices.ptr);
    try t.expectEqual(base, p.input().focus.base.ptr);
    try t.expectEqual(node_count, p.input().nodes.len);
    try t.expectEqual(focus_count, p.input().focus.base.len);
    try t.expectEqualDeep(geometry, try p.geometry());
    std.debug.print("I3 HUD setter Header={d} requests={any} calls_delta=0 refs_delta=0 requested_delta=0 pending_delta=0; transitive stack UNKNOWN\n", .{ ui.HEADER_BYTES, observed.requests[0..observed.count] });
    var slot = r.FrameSlot.init(t.allocator, &budget);
    defer slot.deinit();
    var revision: ?u64 = null;
    try ui.publishHudSettings(&slot, &revision, &p, stamp(owner, 1));
    try t.expectEqualStrings("a/hud/launch", slot.view().?.focus.focused.?.bytes);
    try t.expectError(error.DisabledAction, ui.validateAction(slot.view().?, revision.?, stamp(owner, 1), .{ .bytes = "a/hud/launch" }));
    try t.expectEqualDeep(@import("classic_session.zig").UiAction{ .select_guidance_world = 3 }, try ui.validateAction(slot.view().?, revision.?, stamp(owner, 1), .{ .bytes = "a/hud/marker/3" }));
}

test "I3 HUD focus setter failed first H1 replacement preserves old focus then retry and permanent publication" {
    var tokens = r.LiveTokenBudget.init(.{ .max_live_objects = 1, .max_live_requested_bytes = 1280 });
    defer tokens.deinit();
    const owner = try r.TokenRef.create(t.allocator, &tokens);
    defer owner.release();
    var budget = r.PreparationBudget.init(196608);
    defer budget.deinit();
    var slot = r.FrameSlot.init(t.allocator, &budget);
    defer slot.deinit();
    var revision: ?u64 = null;
    var old = try ui.prepareHudSettings(t.allocator, &budget, .{ .hud_focus = .toggle_pause }, stamp(owner, 1));
    defer old.deinit();
    try ui.publishHudSettings(&slot, &revision, &old, stamp(owner, 1));
    const old_nodes = slot.view().?.nodes.ptr;
    const old_focus = slot.view().?.focus.focused.?.bytes.ptr;
    var candidate = try ui.prepareHudSettings(t.allocator, &budget, .{}, stamp(owner, 2));
    defer candidate.deinit();
    var m = richMarker();
    m.world = 3;
    try ui.attachStartMarker(&candidate, .{ .marker = m });
    try candidate.setHudPublishedFocus(.{ .bytes = "a/hud/marker/3" });
    const ledger = budget.ledger();
    const refs = tokenStorage(owner).refs;
    var failing = t.FailingAllocator.init(t.allocator, .{ .fail_index = 0 });
    slot.allocator = failing.allocator();
    try t.expectError(error.OutOfMemory, ui.publishHudSettings(&slot, &revision, &candidate, stamp(owner, 2)));
    try t.expect(failing.has_induced_failure);
    try t.expectEqualDeep(ledger, budget.ledger());
    try t.expectEqual(@as(usize, 0), budget.pending_bytes);
    try t.expectEqual(@as(usize, 0), budget.active_reservations);
    try t.expectEqual(refs, tokenStorage(owner).refs);
    try t.expectEqual(@as(?u64, 1), revision);
    try t.expectEqual(old_nodes, slot.view().?.nodes.ptr);
    try t.expectEqual(old_focus, slot.view().?.focus.focused.?.bytes.ptr);
    try t.expectEqualStrings("a/hud/pause", slot.view().?.focus.focused.?.bytes);
    try candidate.setHudPublishedFocus(null);
    try t.expect(candidate.input().focus.focused == null);
    try t.expectEqualStrings("a/hud/pause", slot.view().?.focus.focused.?.bytes);
    try candidate.setHudPublishedFocus(.{ .bytes = "a/hud/marker/3" });
    slot.allocator = t.allocator;
    try ui.publishHudSettings(&slot, &revision, &candidate, stamp(owner, 2));
    try t.expectEqual(@as(?u64, 2), revision);
    try t.expectEqualStrings("a/hud/marker/3", slot.view().?.focus.focused.?.bytes);
    try t.expectError(error.AlreadyPublished, candidate.setHudPublishedFocus(null));
    slot.deinit();
    slot = r.FrameSlot.init(t.allocator, &budget);
    try t.expect(slot.view() == null);
    try t.expectError(error.AlreadyPublished, candidate.setHudPublishedFocus(.{ .bytes = "a/hud/launch" }));
}

test "I3 HUD focus setter absent marker hidden source and null publish without fallback" {
    var tokens = r.LiveTokenBudget.init(.{ .max_live_objects = 1, .max_live_requested_bytes = 1280 });
    defer tokens.deinit();
    const owner = try r.TokenRef.create(t.allocator, &tokens);
    defer owner.release();
    var budget = r.PreparationBudget.init(196608);
    defer budget.deinit();
    var p = try ui.prepareHudSettings(t.allocator, &budget, .{ .hud_focus = .reset_onboarding }, stamp(owner, 1));
    defer p.deinit();
    try ui.attachStartMarker(&p, .{ .absent = .no_label_candidate });
    try t.expectError(error.InvalidFocus, p.setHudPublishedFocus(.{ .bytes = "a/hud/marker/3" }));
    try t.expectError(error.InvalidFocus, p.setHudPublishedFocus(.{ .bytes = "a/hud/reset_guidance" }));
    // H1 independently rejects direct reuse of hidden history.
    var slot = r.FrameSlot.init(t.allocator, &budget);
    defer slot.deinit();
    {
        var raw = p.input();
        raw.focus.focused = .{ .bytes = "a/hud/reset_guidance" };
        try t.expectError(error.InvalidFocus, slot.replace(raw, ui.limits));
    }
    try t.expect(slot.view() == null);
    try p.setHudPublishedFocus(null);
    try t.expectError(error.AlreadyAttached, ui.attachStartMarker(&p, .{ .absent = .no_label_candidate }));
    var revision: ?u64 = null;
    try ui.publishHudSettings(&slot, &revision, &p, stamp(owner, 1));
    try t.expect(slot.view().?.focus.focused == null);
    try t.expect(slot.view().?.focus.base.len > 0);
}
