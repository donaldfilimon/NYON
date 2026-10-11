const std = @import("std");
const t = std.testing;
const R = @import("classic_router.zig");
const S = @import("classic_session.zig");
const Store = @import("classic_store.zig");
const Scenario = @import("scenario.zig");
const M = @import("model.zig");
const Layout = @import("classic_editor_layout.zig");
test "I3.A callable real Session initial projection and teardown" {
    var raw = Store.MemoryStore.init(t.allocator, .preferences);
    defer raw.deinit();
    const candidate = try Scenario.generatedValidated(M.DEFAULT_SEED, .{});
    var session = try S.Session.init(t.allocator, &candidate, raw.raw());
    defer session.deinit();
    var workspace = Layout.Workspace{};
    var router: R.Coordinator = undefined;
    try router.init(t.allocator, &session, &workspace);
    defer router.deinit();
}

const H1 = @import("retained_semantics.zig");
const UI = @import("classic_ui.zig");
const Core = @import("classic_app.zig");
const Prefs = @import("preferences.zig");
fn newSession(raw: Store.RawStore) !S.Session {
    const candidate = try Scenario.generatedValidated(M.DEFAULT_SEED, .{});
    return S.Session.init(t.allocator, &candidate, raw);
}
fn expectFocus(router: *const R.Coordinator, expected: ?[]const u8) !void {
    const actual = router.view().?.focus.focused;
    if (expected) |value| try t.expectEqualStrings(value, actual.?.bytes) else try t.expect(actual == null);
}
fn expectGuideCycle(router: *R.Coordinator) !void {
    const revision = router.current().revision;
    const base = router.focus;
    const source = router.source_focus;
    const context = router.context;
    _ = try router.guide(.open);
    try t.expectEqual(revision + 1, router.current().revision);
    try t.expectEqual(revision + 1, router.current().owner.hold.?);
    try t.expect(router.view().?.owner.matches(router.current().owner));
    try t.expectEqual(base, router.saved_focus);
    try t.expectEqual(source, router.saved_source_focus);
    try t.expectEqualStrings("guide.previous", router.view().?.focus.base[0].bytes);
    const opened = router.current().revision;
    _ = try router.guide(.open);
    try t.expectEqual(opened, router.current().revision);
    _ = try router.guide(.{ .show = .{ .topic = 5, .sheet = 0 } });
    _ = try router.guide(.close);
    try t.expectEqualStrings(&context, router.current().owner.context);
    try t.expectEqual(source, router.source_focus);
    try t.expect(!router.saved_present);
    try t.expectEqual(@as(?u64, null), router.guide_publication.revision);
}
test "I3.A Settings literal eleven-row catalog indexes 0 5 10 survive guide" {
    for ([_]usize{ 0, 5, 10 }) |index| {
        var raw = Store.MemoryStore.init(t.allocator, .preferences);
        defer raw.deinit();
        var session = try newSession(raw.raw());
        defer session.deinit();
        _ = session.handleUiAction(.open_settings, raw.raw());
        session.settings.focus(index);
        var workspace = Layout.Workspace{};
        var router: R.Coordinator = undefined;
        try router.init(t.allocator, &session, &workspace);
        defer router.deinit();
        const expected = [_][]const u8{ "a/settings/scale/85", "a/settings/scale/100", "a/settings/scale/115", "a/settings/scale/130", "a/settings/motion", "a/settings/contrast", "a/settings/quality/auto", "a/settings/quality/low", "a/settings/quality/high", "a/settings/reset_guidance", "a/settings/close" };
        try t.expectEqual(@as(usize, 11), router.view().?.focus.base.len);
        for (router.view().?.focus.base, expected) |id, wanted| try t.expectEqualStrings(wanted, id.bytes);
        try expectFocus(&router, expected[index]);
        try expectGuideCycle(&router);
        try expectFocus(&router, expected[index]);
        try t.expectEqual(index, session.settings.focusIndex());
        try t.expectEqualStrings("C|S|-|-|-|0|0", router.context[0..]);
        try t.expect(router.geometry.?.settings);
    }
}
test "I3.A HUD null hidden disabled-present and marker3 histories survive guide" {
    const actions = [_]?S.UiAction{ null, .reset_onboarding, .launch, .{ .select_guidance_world = 3 }, .skip_onboarding };
    const expected = [_]?[]const u8{ null, null, "a/hud/launch", null, null };
    for (actions, expected) |action, wanted| {
        var raw = Store.MemoryStore.init(t.allocator, .preferences);
        defer raw.deinit();
        var session = try newSession(raw.raw());
        defer session.deinit();
        _ = session.handleUiAction(.skip_onboarding, raw.raw());
        session.hud_focus = action;
        var workspace = Layout.Workspace{};
        var router: R.Coordinator = undefined;
        try router.init(t.allocator, &session, &workspace);
        defer router.deinit();
        try expectFocus(&router, wanted);
        try expectGuideCycle(&router);
        try expectFocus(&router, wanted);
        try t.expectEqualDeep(action, session.hud_focus);
        try t.expectEqualStrings("C|P|-|-|-|0|0", router.context[0..]);
        if (wanted != null) try t.expectError(error.DisabledAction, UI.validateAction(router.view().?, router.published_revision.?, .{ .revision = router.current().revision, .owner = router.current().owner }, .{ .bytes = wanted.? }));
    }
}
test "I3.A Editor literal worlds rules validation and historical readonly focus" {
    for ([_]@import("classic_editor.zig").Section{ .worlds, .rules, .validation }) |section| {
        var raw = Store.MemoryStore.init(t.allocator, .preferences);
        defer raw.deinit();
        var session = try newSession(raw.raw());
        defer session.deinit();
        try t.expect(session.openEditor().handled);
        session.editor.session.?.section = section;
        // Valid producer historical observation, not normal readonly focusField.
        session.editor.session.?.focus = .{ .field = .{ .global = .tick_hz } };
        var workspace = Layout.Workspace{};
        var router: R.Coordinator = undefined;
        try router.init(t.allocator, &session, &workspace);
        defer router.deinit();
        try t.expectEqual(@as(u8, 1), router.source_focus);
        try expectFocus(&router, null);
        try t.expect(router.current().owner.draft != null);
        try t.expect(router.geometry == null);
        const expected_len: usize = switch (section) {
            .worlds => 28,
            .rules => 22,
            .validation => 10,
        };
        try t.expectEqual(expected_len, router.view().?.focus.base.len);
        try t.expectEqualStrings("a/editor/section.worlds", router.view().?.focus.base[0].bytes);
        try expectGuideCycle(&router);
        try expectFocus(&router, null);
        try t.expectEqual(@as(u8, 1), router.source_focus);
    }
}
test "I3.A Guide over actual confirmation preserves full base modal and source history" {
    var raw = Store.MemoryStore.init(t.allocator, .preferences);
    defer raw.deinit();
    var session = try newSession(raw.raw());
    defer session.deinit();
    _ = session.openEditor();
    session.editor.session.?.focus = .{ .field = .{ .global = .tick_hz } };
    session.editor.session.?.modal.begin(.revert, null, null);
    var workspace = Layout.Workspace{};
    var router: R.Coordinator = undefined;
    try router.init(t.allocator, &session, &workspace);
    defer router.deinit();
    const modal_hold = router.current().owner.hold;
    const before = router.focus;
    try t.expectEqual(@as(u8, 1), before.restore);
    try t.expectEqualStrings("a/editor/confirmation.dismiss", router.view().?.focus.modal.?[0].bytes);
    try t.expectEqualStrings("a/editor/confirmation.confirm", router.view().?.focus.modal.?[1].bytes);
    try expectGuideCycle(&router);
    try t.expectEqual(before, router.focus);
    try t.expectEqual(modal_hold, router.current().owner.hold);
    try t.expectEqual(@import("classic_editor.zig").ModalKind.revert, session.editor.session.?.modal.kind.?);
}
test "I3.A MainMenu exactly one typed effect then inert retained display" {
    var raw = Store.MemoryStore.init(t.allocator, .preferences);
    defer raw.deinit();
    var session = try newSession(raw.raw());
    defer session.deinit();
    var workspace = Layout.Workspace{};
    var router: R.Coordinator = undefined;
    try router.init(t.allocator, &session, &workspace);
    defer router.deinit();
    _ = try router.guide(.open);
    const old_nodes = router.view().?.nodes.ptr;
    const result = try router.guide(.main_menu);
    try t.expectEqual(R.ShellEffect.return_to_main_menu, result.shell);
    try t.expectEqual(old_nodes, router.view().?.nodes.ptr);
    try t.expectEqual(R.Projection.stale, router.projection);
    try t.expectEqualStrings("C|P|-|-|-|0|1", router.context[0..]);
    try t.expectError(error.LeavingClassic, router.guide(.main_menu));
    try t.expectError(error.LeavingClassic, router.redraw(100));
    try t.expectEqual(@as(usize, 0), raw.save_calls);
}
test "I3.A whole guide holds preserve real fractional timing all four speeds" {
    for ([_]Core.GameSpeed{ .paused, .normal, .fast, .very_fast }, [_]u128{ 0, 1, 2, 4 }) |speed, multiplier| {
        var raw = Store.MemoryStore.init(t.allocator, .preferences);
        defer raw.deinit();
        var session = try newSession(raw.raw());
        defer session.deinit();
        _ = session.handleUiAction(.skip_onboarding, raw.raw());
        _ = session.core.setSpeed(speed);
        if (speed != .paused) _ = session.core.advance(3_000_001);
        var workspace = Layout.Workspace{};
        var router: R.Coordinator = undefined;
        try router.init(t.allocator, &session, &workspace);
        defer router.deinit();
        const alpha = session.core.clockAlpha();
        if (speed != .paused) try t.expect(alpha > 0);
        const fp = session.core.canonicalFingerprint();
        const camera = session.core.camera();
        const visual = session.core.visual_time;
        const selected = session.core.speed();
        const resume_speed = session.core.resumeSpeed();
        _ = try router.guide(.open);
        for ([_]u128{ 0, 5_000_000_000, 900_000_000_000 }) |now| {
            const frame = try router.redraw(now);
            try t.expect(frame.held);
            try t.expect(frame.outcome == null);
            try t.expect(!frame.permit_advisory_poll_submit);
            try t.expectEqual(alpha, session.core.clockAlpha());
            try t.expectEqual(fp, session.core.canonicalFingerprint());
            try t.expectEqualDeep(camera, session.core.camera());
            try t.expectEqual(visual, session.core.visual_time);
        }
        _ = try router.guide(.close);
        const first = try router.redraw(901_000_000_000);
        try t.expect(!first.held);
        try t.expect(first.permit_advisory_poll_submit);
        try t.expectEqual(alpha, session.core.clockAlpha());
        try t.expectEqual(fp, session.core.canonicalFingerprint());
        try t.expectEqual(visual, session.core.visual_time);
        const next = try router.redraw(901_003_000_001);
        try t.expect(next.permit_advisory_poll_submit);
        if (speed != .paused) {
            const expected_alpha = @as(f32, @floatFromInt((6_000_002 * 60 * multiplier) % 1_000_000_000)) / 1_000_000_000.0;
            try t.expectEqual(expected_alpha, session.core.clockAlpha());
        }
        try t.expect(session.core.visual_time > visual);
        try t.expectEqual(selected, session.core.speed());
        try t.expectEqual(resume_speed, session.core.resumeSpeed());
    }
}

test "I3.A actual init allocation ordinals unwind stable budgets and registry" {
    var raw = Store.MemoryStore.init(t.allocator, .preferences);
    defer raw.deinit();
    var session = try newSession(raw.raw());
    defer session.deinit();
    var failures: usize = 0;
    for (0..32) |ordinal| {
        var failing = std.testing.FailingAllocator.init(t.allocator, .{ .fail_index = ordinal });
        var workspace = Layout.Workspace{};
        var router: R.Coordinator = undefined;
        if (router.init(failing.allocator(), &session, &workspace)) |_| {
            router.deinit();
            try t.expectEqual(failing.allocated_bytes, failing.freed_bytes);
            break;
        } else |failure| {
            try t.expectEqual(error.OutOfMemory, failure);
            failures += 1;
            try t.expect(router.closed);
            try t.expect(router.registry.closed);
            try t.expectEqual(@as(usize, 0), router.preparation.ledger().total);
            try t.expectEqual(@as(usize, 0), router.tokens.usage().live_objects);
            try t.expectEqual(failing.allocated_bytes, failing.freed_bytes);
        }
    }
    try t.expect(failures > 4);
    std.debug.print("I3.A Playing init actual failure ordinals={d}\n", .{failures});
}
test "I3.A Open Show Close every actual projection allocation fails stale then same revision retry" {
    var counts: [3]usize = @splat(0);
    for (0..3) |transition| for (0..32) |ordinal| {
        var raw = Store.MemoryStore.init(t.allocator, .preferences);
        defer raw.deinit();
        var session = try newSession(raw.raw());
        defer session.deinit();
        var failing = std.testing.FailingAllocator.init(t.allocator, .{});
        var workspace = Layout.Workspace{};
        var router: R.Coordinator = undefined;
        try router.init(failing.allocator(), &session, &workspace);
        defer router.deinit();
        if (transition != 0) _ = try router.guide(.open);
        router.input.setAction(.launch, true);
        try router.contacts.begin(42, .{ .x = 600, .y = 400 }, .mouse, .primary, .empty_scene);
        const old_nodes = router.view().?.nodes.ptr;
        const old_publication = router.guide_publication;
        const old_revision = router.current().revision;
        const old_display_revision = router.published_revision;
        const old_geometry = router.geometry;
        const request: R.GuideRequest = switch (transition) {
            0 => .open,
            1 => .{ .show = .{ .topic = 5, .sheet = 0 } },
            else => .close,
        };
        failing.fail_index = failing.alloc_index + ordinal;
        if (router.guide(request)) |_| {
            try t.expect(!failing.has_induced_failure);
            break;
        } else |failure| {
            try t.expectEqual(error.OutOfMemory, failure);
            counts[transition] += 1;
            const preadmission = transition == 1 and ordinal == 0;
            try t.expectEqual(old_revision + @as(u64, if (preadmission) 0 else 1), router.current().revision);
            try t.expectEqual(old_display_revision, router.published_revision);
            try t.expectEqual(old_nodes, router.view().?.nodes.ptr);
            try t.expectEqual(old_publication, router.guide_publication);
            try t.expectEqualDeep(old_geometry, router.geometry);
            try t.expectEqual(if (preadmission) R.Projection.current else R.Projection.stale, router.projection);
            try t.expect(router.saved_present);
            if (transition == 0) {
                try t.expect(!router.input.isHeld(.launch));
                try t.expectEqual(@as(usize, 0), router.contacts.contactCount());
            } else {
                try t.expect(router.input.isHeld(.launch));
                try t.expectEqual(@as(usize, 1), router.contacts.contactCount());
            }
            if (transition != 2) {
                const held = try router.redraw(99_000_000_000);
                try t.expect(held.held);
                try t.expect(!held.permit_advisory_poll_submit);
            } else try t.expectError(error.StaleProjection, router.guide(.open));
            const stamp = router.current();
            failing.fail_index = std.math.maxInt(usize);
            _ = try router.recover(.{ .revision = stamp.revision, .owner = stamp.owner }, .retry_projection);
            try t.expectEqual(stamp.revision, router.current().revision);
            try t.expectEqual(R.Projection.current, router.projection);
            try t.expectEqual(transition != 2, router.saved_present);
        }
    };
    for (counts) |count| try t.expect(count > 4);
    std.debug.print("I3.A actual transition failure ordinals Open={d} Show={d} Close={d}\n", .{ counts[0], counts[1], counts[2] });
}
test "I3.A checked revision exhaustion admission preserves every source input contact frame" {
    var raw = Store.MemoryStore.init(t.allocator, .preferences);
    defer raw.deinit();
    var session = try newSession(raw.raw());
    defer session.deinit();
    var workspace = Layout.Workspace{};
    var router: R.Coordinator = undefined;
    try router.init(t.allocator, &session, &workspace);
    defer router.deinit();
    router.clock.value = std.math.maxInt(u64); // Explicit terminal clock fixture.
    router.input.setAction(.launch, true);
    try router.contacts.begin(1, .{ .x = 700, .y = 400 }, .mouse, .primary, .empty_scene);
    const fp = session.core.canonicalFingerprint();
    const old_nodes = router.view().?.nodes.ptr;
    const viewport = session.core.viewport();
    try t.expectError(error.RevisionExhausted, router.guide(.open));
    try t.expectError(error.RevisionExhausted, router.viewport(.{ .x = 900, .y = 700 }));
    try t.expectError(error.RevisionExhausted, router.redraw(1234));
    try t.expectEqualDeep(viewport, session.core.viewport());
    try t.expectEqual(fp, session.core.canonicalFingerprint());
    try t.expect(router.input.isHeld(.launch));
    try t.expectEqual(@as(usize, 1), router.contacts.contactCount());
    try t.expect(router.guide_state == null);
    try t.expect(!router.saved_present);
    try t.expectEqual(old_nodes, router.view().?.nodes.ptr);
}
test "I3.A editor queue eight real pairs reach26 upper bound then teardown zero" {
    var raw = Store.MemoryStore.init(t.allocator, .preferences);
    defer raw.deinit();
    var session = try newSession(raw.raw());
    defer session.deinit();
    _ = session.openEditor();
    var workspace = Layout.Workspace{};
    var router: R.Coordinator = undefined;
    try router.init(t.allocator, &session, &workspace);
    for (0..8) |_| try router.queue.enqueue(.{ .current = router.current(), .kind = .activate, .action = "a/editor/footer.cancel" });
    try t.expectError(error.QueueFull, router.queue.enqueue(.{ .current = router.current(), .kind = .activate, .action = "a/editor/footer.cancel" }));
    _ = try router.guide(.open);
    try t.expectEqual(@as(usize, 26), router.reference_peak);
    try t.expectEqualStrings("C|E|W|0|-|0|0", router.queue.front().?.current.owner.context);
    _ = try router.guide(.close);
    router.deinit();
    try t.expectEqual(@as(usize, 0), router.preparation.ledger().total);
    try t.expectEqual(@as(usize, 0), router.tokens.usage().live_objects);
    try t.expect(router.registry.closed);
    std.debug.print("I3.A actual real reachable reference peak=26; full creation overlap28 deferred B/C\n", .{});
}
test "I3.A hidden live detached draft root projects Playing draft null" {
    var raw = Store.MemoryStore.init(t.allocator, .preferences);
    defer raw.deinit();
    var session = try newSession(raw.raw());
    defer session.deinit();
    _ = session.openEditor();
    session.core.setMode(.playing);
    var workspace = Layout.Workspace{};
    var router: R.Coordinator = undefined;
    try router.init(t.allocator, &session, &workspace);
    defer router.deinit();
    try t.expect(router.roots.draft != null);
    try t.expect(router.current().owner.draft == null);
    try t.expect(router.view().?.owner.draft == null);
    try expectGuideCycle(&router);
    try t.expect(router.roots.draft != null);
}
test "I3.A source mismatch rejects restoration and unsupported routes explicit" {
    var raw = Store.MemoryStore.init(t.allocator, .preferences);
    defer raw.deinit();
    var session = try newSession(raw.raw());
    defer session.deinit();
    var workspace = Layout.Workspace{};
    var router: R.Coordinator = undefined;
    try router.init(t.allocator, &session, &workspace);
    defer router.deinit();
    _ = try router.guide(.open);
    const old_nodes = router.view().?.nodes.ptr;
    session.hud_focus = .launch; // Deliberate violating caller for fail-closed witness.
    try t.expectError(error.SourceChanged, router.guide(.close));
    try t.expectEqual(old_nodes, router.view().?.nodes.ptr);
    try t.expect(router.guide_state != null);
    try t.expectError(error.PendingSlice, router.key(.{ .key = .{ .named = .f1 }, .state = .pressed, .repeat = false, .modifiers = .{} }, raw.raw()));
    try t.expectError(error.PendingSlice, router.ime(.{ .commit = "abc" }));
    try t.expectError(error.PendingSlice, router.drain(raw.raw()));
}

test "I3.A all ContextKey combinations canonical13 or exhaustive rejection" {
    const surfaces = [_]@FieldType(R.ContextKey, "surface"){ .classic, .leaving_classic };
    const bases = [_]@FieldType(R.ContextKey, "base"){ .playing, .settings, .editor };
    const sections = [_]@FieldType(R.ContextKey, "section"){ .none, .worlds, .rules, .validation };
    for (surfaces) |surface| for (bases) |base| for (sections) |section| for ([_]u8{ 0, 6, 7, 255 }) |world| for ([_]u8{ 0, 6, 7, 255 }) |confirmation| for ([_]bool{ false, true }) |guide| {
        const valid_base = if (base == .editor) section != .none and (if (section == .worlds) world <= 6 else world == 255) and (confirmation <= 6 or confirmation == 255) else section == .none and world == 255 and confirmation == 255;
        const valid = valid_base and !(surface == .leaving_classic and guide);
        const context = R.ContextKey{ .surface = surface, .base = base, .section = section, .world = world, .confirmation = confirmation, .guide = guide };
        if (valid) {
            const bytes = try context.encode();
            try t.expectEqual(@as(usize, 13), bytes.len);
            try t.expectEqual(@as(u8, 'C'), bytes[0]);
            for ([_]usize{ 1, 3, 5, 7, 9, 11 }) |separator| try t.expectEqual(@as(u8, '|'), bytes[separator]);
        } else try t.expectError(error.InvalidContext, context.encode());
    };
}
test "I3.A full independent editor109 source catalog current publication and copied guide history" {
    const E = @import("classic_editor.zig");
    const globals = [_]E.GlobalField{ .seed, .tick_hz, .win_world_count, .ai_period_ticks, .hazard_first_tick, .hazard_period_ticks, .hazard_duration_ticks, .maximum_energy, .maximum_defense, .minimum_launch, .field_raise_cost, .field_lower_refund, .base_fleet_speed };
    const fields = [_]E.WorldField{ .name, .x, .y, .owner, .energy, .defense, .base_output, .base_regeneration, .atmosphere, .hydrosphere, .topology };
    const sections = [_]E.Section{ .worlds, .rules, .validation };
    const footers = [_]E.Action{ .revert, .factory_defaults, .regenerate_from_seed, .load, .save, .cancel, .apply_and_restart };
    var sources: [109]E.Focus = undefined;
    for (globals, 0..) |field, i| sources[i] = .{ .field = .{ .global = field } };
    for (0..7) |world| for (fields, 0..) |field, j| {
        sources[13 + world * 11 + j] = .{ .field = .{ .world = .{ .index = @intCast(world), .field = field } } };
    };
    for (sections, 0..) |section, i| sources[90 + i] = .{ .action = .{ .select_section = section } };
    for (0..7) |world| sources[93 + world] = .{ .action = .{ .select_world = world } };
    for (footers, 0..) |action, i| sources[100 + i] = .{ .action = action };
    sources[107] = .{ .action = .dismiss_confirmation };
    sources[108] = .{ .action = .confirm };
    for (sources, 0..) |source, i| {
        var raw = Store.MemoryStore.init(t.allocator, .preferences);
        defer raw.deinit();
        var session = try newSession(raw.raw());
        defer session.deinit();
        _ = session.openEditor();
        session.editor.session.?.focus = source;
        var workspace = Layout.Workspace{};
        var router: R.Coordinator = undefined;
        try router.init(t.allocator, &session, &workspace);
        defer router.deinit();
        try t.expectEqual(@as(u8, @intCast(i)), router.source_focus);
        const eligible = (i >= 13 and i < 24) or (i >= 90 and i < 107);
        try t.expectEqual(eligible, router.view().?.focus.focused != null);
        try expectGuideCycle(&router);
        try t.expectEqualDeep(@as(?E.Focus, source), session.editor.session.?.focus);
        try t.expectEqual(@as(u8, @intCast(i)), router.source_focus);
    }
}
test "I3.A actual marker3 restored with canonical setter after real adapter" {
    var raw = Store.MemoryStore.init(t.allocator, .preferences);
    defer raw.deinit();
    const generated = try Scenario.generatedValidated(M.DEFAULT_SEED, .{});
    var draft = try generated.toDraft(t.allocator);
    defer draft.deinit();
    for (&draft.worlds, 0..) |*world, i| world.owner = if (i == 3) .union_faction else .helix;
    const candidate = try Scenario.fromDraft(&draft);
    var session = try S.Session.init(t.allocator, &candidate, raw.raw());
    defer session.deinit();
    session.hud_focus = .{ .select_guidance_world = 3 };
    var workspace = Layout.Workspace{};
    var router: R.Coordinator = undefined;
    try router.init(t.allocator, &session, &workspace);
    defer router.deinit();
    try expectFocus(&router, "a/hud/marker/3");
    try t.expectEqual(@as(u8, 20), router.source_focus);
    try expectGuideCycle(&router);
    try expectFocus(&router, "a/hud/marker/3");
    try t.expectEqualDeep(S.UiAction{ .select_guidance_world = 3 }, try UI.validateAction(router.view().?, router.published_revision.?, .{ .revision = router.current().revision, .owner = router.current().owner }, .{ .bytes = "a/hud/marker/3" }));
}
test "I3.A editor modal and Settings initialization each ordinal closes all owners" {
    var counts: [3]usize = @splat(0);
    for (0..3) |mode| {
        var raw = Store.MemoryStore.init(t.allocator, .preferences);
        defer raw.deinit();
        var session = try newSession(raw.raw());
        defer session.deinit();
        if (mode == 0) _ = session.handleUiAction(.open_settings, raw.raw()) else {
            _ = session.openEditor();
            if (mode == 2) session.editor.session.?.modal.begin(.apply_and_restart, null, null);
        }
        for (0..32) |ordinal| {
            var failing = std.testing.FailingAllocator.init(t.allocator, .{ .fail_index = ordinal });
            var workspace = Layout.Workspace{};
            var router: R.Coordinator = undefined;
            if (router.init(failing.allocator(), &session, &workspace)) |_| {
                router.deinit();
                try t.expectEqual(failing.allocated_bytes, failing.freed_bytes);
                break;
            } else |failure| {
                try t.expectEqual(error.OutOfMemory, failure);
                counts[mode] += 1;
                try t.expect(router.closed);
                try t.expectEqual(@as(usize, 0), router.preparation.ledger().total);
                try t.expectEqual(@as(usize, 0), router.tokens.usage().live_objects);
                try t.expectEqual(failing.allocated_bytes, failing.freed_bytes);
            }
        }
    }
    std.debug.print("I3.A actual init failure ordinals Settings={d} Editor={d} Modal={d}\n", .{ counts[0], counts[1], counts[2] });
}
test "I3.A modal Open Show Close actual allocator unwind and saved historical restore" {
    var counts: [3]usize = @splat(0);
    for (0..3) |transition| for (0..32) |ordinal| {
        var raw = Store.MemoryStore.init(t.allocator, .preferences);
        defer raw.deinit();
        var session = try newSession(raw.raw());
        defer session.deinit();
        _ = session.openEditor();
        session.editor.session.?.focus = .{ .field = .{ .global = .tick_hz } };
        session.editor.session.?.modal.begin(.revert, null, null);
        var failing = std.testing.FailingAllocator.init(t.allocator, .{});
        var workspace = Layout.Workspace{};
        var router: R.Coordinator = undefined;
        try router.init(failing.allocator(), &session, &workspace);
        defer router.deinit();
        const original = router.focus;
        const hold = router.confirmation_hold;
        if (transition != 0) _ = try router.guide(.open);
        const old = router.view().?.nodes.ptr;
        const old_revision = router.published_revision;
        failing.fail_index = failing.alloc_index + ordinal;
        const request: R.GuideRequest = switch (transition) {
            0 => .open,
            1 => .{ .show = .{ .topic = 6, .sheet = 0 } },
            else => .close,
        };
        if (router.guide(request)) |_| {
            try t.expect(!failing.has_induced_failure);
            break;
        } else |failure| {
            try t.expectEqual(error.OutOfMemory, failure);
            counts[transition] += 1;
            try t.expectEqual(old, router.view().?.nodes.ptr);
            try t.expectEqual(old_revision, router.published_revision);
            try t.expectEqual(original, router.saved_focus);
            try t.expectEqual(hold, router.confirmation_hold);
            try t.expect(router.saved_present);
            failing.fail_index = std.math.maxInt(usize);
            const stamp = router.current();
            _ = try router.recover(.{ .revision = stamp.revision, .owner = stamp.owner }, .retry_projection);
            if (transition == 2) {
                try t.expectEqual(original, router.focus);
                try t.expectEqual(hold, router.current().owner.hold);
            }
        }
    };
    std.debug.print("I3.A Modal actual transition failure ordinals Open={d} Show={d} Close={d}\n", .{ counts[0], counts[1], counts[2] });
}
test "I3.A measured actual combined P and ordinary target layouts" {
    var peaks: [3]usize = @splat(0);
    for (0..3) |mode| {
        var raw = Store.MemoryStore.init(t.allocator, .preferences);
        defer raw.deinit();
        var session = try newSession(raw.raw());
        defer session.deinit();
        if (mode == 1) _ = session.handleUiAction(.open_settings, raw.raw());
        if (mode == 2) {
            _ = session.openEditor();
            session.editor.session.?.modal.begin(.revert, null, null);
        }
        var workspace = Layout.Workspace{};
        var router: R.Coordinator = undefined;
        try router.init(t.allocator, &session, &workspace);
        defer router.deinit();
        _ = try router.guide(.open);
        _ = try router.guide(.{ .show = .{ .topic = 4, .sheet = 0 } });
        _ = try router.guide(.close);
        peaks[mode] = router.preparation.ledger().peak;
    }
    std.debug.print("I3.A actual lifecycle P Playing={d} Settings={d} Modal={d}; Coordinator={d} align={d} FocusSnapshot={d} sourcebytes=2 nativekey={d} scratch={d} Geometry={d} OptionalGeometry={d} Controller={d}; stack/process UNKNOWN\n", .{ peaks[0], peaks[1], peaks[2], @sizeOf(R.Coordinator), @alignOf(R.Coordinator), @sizeOf(R.FocusSnapshot), @sizeOf(Layout.FocusKey), @sizeOf([28]Layout.FocusKey), @sizeOf(UI.GeometryView), @sizeOf(?UI.GeometryView), @sizeOf(R.Controller) });
}
