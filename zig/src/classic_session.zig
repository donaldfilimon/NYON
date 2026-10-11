//! Move-only Classic session composition. Raw stores are borrowed only for each call.
const std = @import("std");
const app = @import("classic_app.zig");
const m = @import("model.zig");
const scenario = @import("scenario.zig");
const prefs = @import("preferences.zig");
const store = @import("classic_store.zig");
const commands = @import("classic_commands.zig");
const onboarding = @import("classic_onboarding.zig");
const settings_model = @import("classic_settings.zig");
const transactions = @import("classic_editor_transactions.zig");
const editor_model = @import("classic_editor.zig");
pub const UiAction = union(enum) { select_guidance_world: m.WorldId, launch, clear_selection, speed_down, toggle_pause, speed_up, reset_camera, open_settings, toggle_help, open_scenario, acknowledge_advisory, skip_onboarding, reset_onboarding, select_field: m.FieldKind, decrease_field, increase_field, settings: settings_model.Action };
pub const Persistence = enum { not_requested, saved, failed, resource_error };
/// Informational completed effects: reset_camera is consumed by the producer.
/// Hosts must not replay it. Core owns installation reset; Session owns UI reset.
pub const Effects = struct { reset_camera: bool = false, clear_context_focus: bool = false, restart_advisory: bool = false, presentation_changed: bool = false };
pub const Result = struct { handled: bool = false, persistence: Persistence = .not_requested, editor_outcome: ?transactions.Outcome = null, editor_error: ?anyerror = null, command_error: ?commands.DispatchError = null, queued: ?app.Enqueued = null, effects: Effects = .{} };
pub const ResourceErrors = struct { preferences: ?std.mem.Allocator.Error = null, editor: ?anyerror = null, command: ?std.mem.Allocator.Error = null };
pub const GeneralFailure = union(enum) { command: commands.DispatchError, admission: scenario.AdmissionError };
pub const RecoveryFailure = union(enum) { general: *const GeneralFailure, resource: ResourceErrors, tick: app.TickFailure, preferences: *const store.PreferencesFailure };
pub const Control = struct { action: UiAction, enabled: bool = true };
pub const Controls = struct {
    values: [18]Control = undefined,
    len: usize = 0,
    pub fn items(self: *const Controls) []const Control {
        return self.values[0..self.len];
    }
    fn append(self: *Controls, action: UiAction, enabled: bool) void {
        self.values[self.len] = .{ .action = action, .enabled = enabled };
        self.len += 1;
    }
};
/// Move-only by convention. Never copy and deinitialize both owners.
pub const Session = struct {
    allocator: std.mem.Allocator,
    core: app.AppCore,
    editor: transactions.Transactions,
    onboarding: onboarding.State,
    settings: settings_model.State = .{},
    hud_focus: ?UiAction = null,
    preference_failure: ?store.PreferencesFailure = null,
    general_failure: ?GeneralFailure = null,
    resource_errors: ResourceErrors = .{},
    pub fn init(allocator: std.mem.Allocator, candidate_value: *const scenario.ScenarioV1, raw: store.RawStore) !Session {
        var loaded = try store.loadOrDefault(allocator, raw);
        defer loaded.deinit(allocator);
        const core = try app.AppCore.init(allocator, candidate_value, loaded.preferences);
        const failure = loaded.failure;
        loaded.failure = null;
        return .{ .allocator = allocator, .core = core, .editor = transactions.Transactions.init(allocator), .onboarding = onboarding.State.init(loaded.preferences.onboarding_completed), .preference_failure = failure };
    }
    pub fn deinit(self: *Session) void {
        self.editor.deinit();
        if (self.preference_failure) |*failure| failure.deinit(self.allocator);
        self.core.deinit();
        self.* = undefined;
    }
    pub fn presentation(self: *const Session) prefs.PresentationPreferences {
        return self.core.presentation();
    }
    pub fn recoveryFailure(self: *const Session) ?RecoveryFailure {
        if (self.general_failure) |*failure| return .{ .general = failure };
        if (self.resource_errors.preferences != null or self.resource_errors.editor != null or self.resource_errors.command != null) return .{ .resource = self.resource_errors };
        if (self.core.recoverableStatus()) |failure| return .{ .tick = failure };
        if (self.preference_failure) |*failure| return .{ .preferences = failure };
        return null;
    }
    fn persist(self: *Session, raw: store.RawStore) Persistence {
        var outcome = store.saveOrDefault(self.allocator, raw, self.core.preferences()) catch |failure| {
            self.resource_errors.preferences = failure;
            return .resource_error;
        };
        defer outcome.deinit(self.allocator);
        // Stage the new result before releasing an earlier owned diagnostic.
        if (self.preference_failure) |*failure| failure.deinit(self.allocator);
        self.preference_failure = outcome.failure;
        outcome.failure = null;
        self.resource_errors.preferences = null;
        return if (self.preference_failure == null) .saved else .failed;
    }
    fn complete(self: *Session, raw: store.RawStore) Persistence {
        var value = self.core.preferences();
        value.onboarding_completed = true;
        self.core.setPreferences(value);
        self.core.setOnboardingHold(false);
        return self.persist(raw);
    }
    fn observe(self: *Session, event: onboarding.Event, raw: store.RawStore) Persistence {
        return if (self.onboarding.observe(event)) self.complete(raw) else .not_requested;
    }
    fn consume(self: *Session, effects: commands.Effects) void {
        if (effects.clear_ui_focus) self.hud_focus = null;
        if (effects.observation) |event| {
            // These four events cannot complete the six-step automaton.
            _ = self.onboarding.observe(switch (event) {
                .union_world_selected => .union_world_selected,
                .launch_previewed => .launch_previewed,
                .fleet_launched => .fleet_launched,
                .field_tuned => .field_tuned,
            });
        }
    }
    fn commandResult(self: *Session, outcome: commands.DispatchError!commands.Dispatch) Result {
        const result = outcome catch |failure| {
            switch (failure) {
                error.MissingSource, error.MissingTarget, error.ModeRefused => {},
                else => self.general_failure = .{ .command = failure },
            }
            if (failure == error.OutOfMemory) self.resource_errors.command = error.OutOfMemory;
            return .{ .command_error = failure };
        };
        self.resource_errors.command = null;
        self.consume(result.effects);
        return .{ .handled = result.handled, .queued = result.queued };
    }
    fn applySettings(self: *Session, action: settings_model.Action, raw: store.RawStore) Result {
        if (self.core.mode() != .settings) return .{};
        var value = self.core.preferences();
        switch (action) {
            .set_ui_scale => |scale| value.ui_scale = scale,
            .set_motion => |motion| value.motion = motion,
            .set_high_contrast => |contrast| value.high_contrast = contrast,
            .set_graphics_quality => |quality| value.graphics_quality = quality,
            .reset_onboarding => {
                self.onboarding.reset();
                value.onboarding_completed = false;
                self.core.setOnboardingHold(true);
                _ = self.core.setSpeed(.paused);
            },
            .close => {
                self.core.setMode(.playing);
                return .{ .handled = true };
            },
        }
        self.core.setPreferences(value);
        self.core.setPresentationPreferences(value.presentation());
        return .{ .handled = true, .persistence = self.persist(raw), .effects = .{ .presentation_changed = true } };
    }
    pub fn handleUiAction(self: *Session, action: UiAction, raw: store.RawStore) Result {
        switch (action) {
            .settings => |setting| return self.applySettings(setting, raw),
            .open_settings => {
                if (self.core.mode() != .playing) return .{};
                self.core.setMode(.settings);
                self.settings = .{};
                self.hud_focus = null;
                return .{ .handled = true };
            },
            .launch => return self.commandResult(commands.contextualLaunch(&self.core)),
            .increase_field => return self.commandResult(commands.adjustField(&self.core, .increase)),
            .decrease_field => return self.commandResult(commands.adjustField(&self.core, .decrease)),
            .acknowledge_advisory => return .{ .handled = true, .persistence = self.observe(.advisory_read, raw) },
            .skip_onboarding => return .{ .handled = true, .persistence = if (self.onboarding.skip()) self.complete(raw) else .not_requested },
            .open_scenario => {
                if (self.core.mode() != .playing) return .{};
                const persistence = self.observe(.scenario_editor_opened, raw);
                // Completion/persistence precede preparation. Resource errors never
                // short-circuit opening; the composite result exposes both parts.
                var result = self.openEditor();
                result.persistence = persistence;
                return result;
            },
            .clear_selection => {
                const result = commands.clearSelection(&self.core);
                self.consume(result.effects);
                return .{ .handled = result.handled };
            },
            .reset_onboarding => {
                self.onboarding.reset();
                var value = self.core.preferences();
                value.onboarding_completed = false;
                self.core.setPreferences(value);
                self.core.help_visible = false;
                self.core.setOnboardingHold(true);
                _ = self.core.setSpeed(.paused);
                return .{ .handled = true, .persistence = self.persist(raw) };
            },
            .select_guidance_world => |id| {
                if (self.core.mode() != .playing or self.onboarding.step() != .select_union_world) return .{};
                var owned = false;
                for (self.core.campaign().worlds) |world| if (world.id == id) {
                    owned = world.owner == .union_faction;
                    break;
                };
                if (!owned) return .{};
                self.core.selected_world = id;
                self.core.hovered_world = id;
                self.core.touch_command_target = null;
                return .{ .handled = true, .persistence = self.observe(.union_world_selected, raw) };
            },
            .select_field => |field| return .{ .handled = commands.selectField(&self.core, field) },
            .speed_down, .toggle_pause, .speed_up, .reset_camera, .toggle_help => {
                if (self.core.mode() != .playing) return .{};
                switch (action) {
                    .speed_down => _ = self.core.setSpeed(self.core.speed().slower()),
                    .speed_up => _ = self.core.setSpeed(self.core.speed().faster()),
                    .toggle_pause => _ = self.core.setSpeed(if (self.core.speed() == .paused) self.core.resumeSpeed() else .paused),
                    .toggle_help => self.core.help_visible = !self.core.help_visible,
                    .reset_camera => {
                        self.core.resetCamera();
                        return .{ .handled = true, .effects = .{ .reset_camera = true } };
                    },
                    else => unreachable,
                }
                return .{ .handled = true };
            },
        }
    }
    pub fn primaryTap(self: *Session, hit: ?m.WorldId, touch: bool) commands.PickingError!commands.Interaction {
        const result = try commands.primaryTap(&self.core, hit, touch);
        self.consume(result.effects);
        return result;
    }
    pub fn selectSceneWorld(self: *Session, hit: ?m.WorldId, hud_consumed: bool) commands.PickingError!commands.Interaction {
        const result = try commands.selectSceneWorld(&self.core, hit, hud_consumed);
        self.consume(result.effects);
        return result;
    }
    pub fn cursorHover(self: *Session, hit: ?m.WorldId, hud_consumed: bool) commands.PickingError!commands.Interaction {
        const result = try commands.cursorHover(&self.core, hit, hud_consumed);
        self.consume(result.effects);
        return result;
    }
    pub fn queueGameCommand(self: *Session, command: m.GameCommand) app.EnqueueError!commands.Dispatch {
        const result = commands.queueGameCommand(&self.core, command) catch |failure| {
            if (failure == error.OutOfMemory) self.resource_errors.command = error.OutOfMemory;
            return failure;
        };
        self.resource_errors.command = null;
        self.consume(result.effects);
        return result;
    }
    fn editorResult(self: *Session, outcome: anyerror!transactions.Outcome) Result {
        const result = outcome catch |failure| {
            self.resource_errors.editor = failure;
            return .{ .handled = true, .editor_error = failure };
        };
        // An inert dispatch did not retry the failed editor operation.
        if (result != .unchanged) self.resource_errors.editor = null;
        var effects: Effects = .{};
        switch (result) {
            .opened, .closed, .applied => {
                const source = self.editor.effects;
                // Core installation already consumed restart_advisory exactly once.
                if (result == .applied) {
                    self.editor.effects.restart_advisory = false;
                    self.editor.effects.reset_camera = false;
                }
                effects = .{ .reset_camera = source.reset_camera, .clear_context_focus = source.clear_focus, .restart_advisory = false };
                if (result == .applied) self.hud_focus = null;
                if (result == .closed or result == .applied) self.general_failure = null;
            },
            else => {},
        }
        return .{ .handled = result != .unchanged, .editor_outcome = result, .effects = effects };
    }
    pub fn openEditor(self: *Session) Result {
        return self.editorResult(self.editor.open(&self.core));
    }
    pub fn dispatchEditor(self: *Session, action: editor_model.Action, raw: store.RawStore) Result {
        return self.editorResult(self.editor.dispatch(&self.core, raw, action));
    }
    /// Pure action/enablement projection. Marker identity comes from the later
    /// scene/widget owner; this module neither chooses nor places its geometry.
    pub fn controls(self: *const Session, guidance_world: ?m.WorldId) Controls {
        var result: Controls = .{};
        switch (self.core.mode()) {
            .editing_scenario => return result,
            .settings => {
                for (settings_model.actions(self.core.preferences())) |action| result.append(.{ .settings = action }, true);
                return result;
            },
            .playing => {},
        }
        const active = self.onboarding.active();
        const tray = commands.commandTray(&self.core);
        result.append(.speed_down, !active);
        result.append(.toggle_pause, !active);
        result.append(.speed_up, !active);
        result.append(.launch, tray.launch_enabled);
        result.append(.clear_selection, tray.source != null or tray.destination != null);
        for ([_]UiAction{ .reset_camera, .open_settings, .toggle_help, .open_scenario }) |action| result.append(action, true);
        if (active) result.append(.skip_onboarding, true);
        for ([_]m.FieldKind{ .atmosphere, .hydrosphere, .topology }) |field| result.append(.{ .select_field = field }, true);
        result.append(.decrease_field, commands.fieldControl(&self.core, .decrease).enabled);
        result.append(.increase_field, commands.fieldControl(&self.core, .increase).enabled);
        result.append(.acknowledge_advisory, true);
        if (self.core.help_visible) result.append(.reset_onboarding, true);
        if (guidance_world) |id| result.append(.{ .select_guidance_world = id }, true);
        return result;
    }
    /// Pure lookup retains a hidden identity until an explicit focus operation.
    pub fn focusIndex(self: *const Session, controls_value: []const Control) ?usize {
        if (self.core.mode() == .settings) return self.settings.focusIndex();
        const action = self.hud_focus orelse return null;
        for (controls_value, 0..) |control, i| if (std.meta.eql(action, control.action)) return i;
        return null;
    }
    pub fn focusNext(self: *Session, backwards: bool, controls_value: []const Control) void {
        if (self.core.mode() == .settings) {
            if (backwards) self.settings.previous() else self.settings.next();
            return;
        }
        const count = controls_value.len;
        if (count == 0) {
            self.hud_focus = null;
            return;
        }
        const current = self.focusIndex(controls_value) orelse if (backwards) 0 else count - 1;
        const next = if (backwards) (if (current == 0) count - 1 else current - 1) else (if (current == count - 1) 0 else current + 1);
        self.hud_focus = controls_value[next].action;
    }
    pub fn focus(self: *Session, index: usize, controls_value: []const Control) void {
        if (self.core.mode() == .settings) self.settings.focus(index) else if (index < controls_value.len) {
            self.hud_focus = controls_value[index].action;
        }
    }
    pub fn activateFocused(self: *Session, controls_value: []const Control, raw: store.RawStore) Result {
        if (self.core.mode() == .settings) {
            const index = self.settings.focusIndex();
            if (index >= controls_value.len or !controls_value[index].enabled) return .{};
            return self.handleUiAction(controls_value[index].action, raw);
        }
        const index = self.focusIndex(controls_value) orelse {
            self.hud_focus = null;
            return .{};
        };
        if (!controls_value[index].enabled) {
            self.hud_focus = null;
            return .{};
        }
        return self.handleUiAction(controls_value[index].action, raw);
    }
    pub fn clearUiFocus(self: *Session) void {
        if (self.core.mode() == .playing) self.hud_focus = null;
    }
    pub fn cancel(self: *Session) commands.Interaction {
        const result = commands.cancel(&self.core);
        self.consume(result.effects);
        return result;
    }
    pub fn restart(self: *Session) scenario.AdmissionError!Effects {
        self.core.restart() catch |failure| {
            self.general_failure = .{ .admission = failure };
            return failure;
        };
        self.hud_focus = null;
        self.general_failure = null;
        return .{ .reset_camera = true, .clear_context_focus = true, .restart_advisory = false };
    }
    /// Transient presentation projection, separate from durable preference truth.
    pub fn setPresentationPreferences(self: *Session, value: prefs.PresentationPreferences) Effects {
        self.core.setPresentationPreferences(value);
        return .{ .presentation_changed = true };
    }
    fn consumedDispatch(self: *Session, outcome: commands.DispatchError!commands.Dispatch) commands.DispatchError!commands.Dispatch {
        const result = outcome catch |failure| {
            if (failure == error.OutOfMemory) self.resource_errors.command = error.OutOfMemory;
            return failure;
        };
        self.resource_errors.command = null;
        self.consume(result.effects);
        return result;
    }
    pub fn keyboardAction(self: *Session, action: commands.CommandAction) commands.DispatchError!commands.Dispatch {
        return self.consumedDispatch(commands.keyboardAction(&self.core, action));
    }
    pub fn rightClick(self: *Session, hit: ?m.WorldId) commands.DispatchError!commands.Dispatch {
        return self.consumedDispatch(commands.rightClick(&self.core, hit));
    }
};
