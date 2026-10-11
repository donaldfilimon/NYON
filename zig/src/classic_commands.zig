//! ID-only Classic intent controller. Platform picking and onboarding consume
//! these value effects; the controller retains no session pointers or state.
const app = @import("classic_app.zig");
const m = @import("model.zig");
const sim = @import("simulation.zig");

pub const Observation = enum { union_world_selected, launch_previewed, fleet_launched, field_tuned };
pub const Effects = struct { clear_ui_focus: bool = false, observation: ?Observation = null };
pub const Interaction = struct { handled: bool = false, effects: Effects = .{} };
pub const PickingError = error{InvalidWorld};
pub fn commandTarget(core: *const app.AppCore) ?m.WorldId {
    return core.hovered_world orelse core.touch_command_target;
}
fn world(core: *const app.AppCore, id: m.WorldId) ?m.World {
    for (core.campaign().worlds) |resident| if (resident.id == id) return resident;
    return null;
}
fn admitHit(core: *const app.AppCore, hit: ?m.WorldId) PickingError!void {
    if (hit) |id| if (world(core, id) == null) return error.InvalidWorld;
}
fn unionOwned(core: *const app.AppCore, hit: ?m.WorldId) bool {
    const resident = world(core, hit orelse return false) orelse return false;
    return resident.owner == .union_faction;
}
fn previewValid(core: *const app.AppCore) bool {
    const source = core.selected_world orelse return false;
    const destination = commandTarget(core) orelse return false;
    _ = core.previewCommand(.{ .launch = .{ .source = source, .destination = destination } }) catch return false;
    return true;
}
pub fn selectSceneWorld(core: *app.AppCore, hit: ?m.WorldId, hud_consumed: bool) PickingError!Interaction {
    if (core.mode() != .playing or hud_consumed) return .{};
    try admitHit(core, hit);
    core.selected_world = hit;
    core.touch_command_target = null;
    return .{ .handled = true, .effects = .{ .clear_ui_focus = true } };
}
pub fn hoverSceneWorld(core: *app.AppCore, hit: ?m.WorldId, hud_consumed: bool) PickingError!Interaction {
    if (core.mode() != .playing) return .{};
    const admitted = if (hud_consumed) null else hit;
    try admitHit(core, admitted);
    core.hovered_world = admitted;
    return .{ .handled = true };
}
pub fn cursorHover(core: *app.AppCore, hit: ?m.WorldId, hud_consumed: bool) PickingError!Interaction {
    var result = try hoverSceneWorld(core, hit, hud_consumed);
    if (result.handled and previewValid(core)) result.effects.observation = .launch_previewed;
    return result;
}
pub fn clearHover(core: *app.AppCore) void {
    core.hovered_world = null;
}
pub fn primaryTap(core: *app.AppCore, hit: ?m.WorldId, touch: bool) PickingError!Interaction {
    if (core.mode() != .playing) return .{};
    try admitHit(core, hit);
    var effects: Effects = .{ .clear_ui_focus = true };
    if (!touch or hit == null or unionOwned(core, hit)) {
        core.selected_world = hit;
        core.hovered_world = hit;
        core.touch_command_target = null;
        if (unionOwned(core, hit)) effects.observation = .union_world_selected;
    } else if (unionOwned(core, core.selected_world)) {
        core.hovered_world = hit;
        core.touch_command_target = hit;
        if (previewValid(core)) effects.observation = .launch_previewed;
    } else {
        core.selected_world = null;
        core.hovered_world = hit;
        core.touch_command_target = null;
    }
    return .{ .handled = true, .effects = effects };
}
pub fn clearSelection(core: *app.AppCore) Interaction {
    core.selected_world = null;
    core.hovered_world = null;
    core.touch_command_target = null;
    return .{ .handled = true, .effects = .{ .clear_ui_focus = true } };
}
pub fn cancel(core: *app.AppCore) Interaction {
    if (core.mode() != .playing) return .{};
    return clearSelection(core);
}
pub fn selectField(core: *app.AppCore, field: m.FieldKind) bool {
    if (core.mode() != .playing) return false;
    core.selected_field = field;
    return true;
}

pub const Tray = struct { source: ?m.WorldName = null, destination: ?m.WorldName = null, launch_strength: ?m.Strength = null, status: []const u8 = "SELECT SOURCE AND TARGET", launch_enabled: bool = false };
pub const FieldControl = struct { enabled: bool = false, tooltip: []const u8 = "Select a Union world" };
pub fn previewContextualLaunch(core: *const app.AppCore) ?sim.CommandRejection!sim.CommandPreview {
    const source = core.selected_world orelse return null;
    const destination = commandTarget(core) orelse return null;
    return core.previewCommand(.{ .launch = .{ .source = source, .destination = destination } });
}
fn name(core: *const app.AppCore, id: ?m.WorldId) ?m.WorldName {
    return (world(core, id orelse return null) orelse return null).name;
}
pub fn commandTray(core: *const app.AppCore) Tray {
    var tray: Tray = .{ .source = name(core, core.selected_world), .destination = name(core, commandTarget(core)) };
    const preview = previewContextualLaunch(core) orelse return tray;
    const valid = preview catch |reason| {
        tray.status = rejectionLabel(reason);
        return tray;
    };
    tray.launch_strength = valid.launch_strength;
    tray.status = "COMMAND READY - EXPLICIT LAUNCH REQUIRED";
    tray.launch_enabled = true;
    return tray;
}
pub fn fieldControl(core: *const app.AppCore, adjustment: m.FieldAdjustment) FieldControl {
    const id = core.selected_world orelse return .{};
    _ = core.previewCommand(.{ .tune_field = .{ .world = id, .field = core.selected_field, .adjustment = adjustment } }) catch |reason| return .{ .tooltip = rejectionLabel(reason) };
    return .{ .enabled = true, .tooltip = switch (adjustment) {
        .decrease => "Decrease selected field (Q)",
        .increase => "Increase selected field (E)",
    } };
}
pub fn rejectionLabel(reason: sim.CommandRejection) []const u8 {
    return switch (reason) {
        error.WrongTick => "REJECTED - WRONG TICK",
        error.DuplicateSequence => "REJECTED - DUPLICATE COMMAND",
        error.CampaignFinished => "REJECTED - CAMPAIGN FINISHED",
        error.InvalidWorld => "REJECTED - INVALID WORLD",
        error.SameSourceAndTarget => "REJECTED - CHOOSE ANOTHER TARGET",
        error.SourceNotOwnedByIssuer => "REJECTED - SOURCE IS NOT UNION",
        error.InsufficientEnergy => "REJECTED - INSUFFICIENT ENERGY",
        error.FieldAtLimit => "REJECTED - FIELD AT LIMIT",
    };
}

pub const DispatchError = app.EnqueueError || error{ MissingSource, MissingTarget };
pub const Dispatch = struct { handled: bool = false, queued: ?app.Enqueued = null, effects: Effects = .{} };
pub const CommandAction = enum { launch, increase, decrease };
pub fn queueGameCommand(core: *app.AppCore, command: m.GameCommand) app.EnqueueError!Dispatch {
    const queued = try core.enqueueGameCommand(command);
    var effects: Effects = .{};
    if (queued.preview_valid) switch (command) {
        .launch => {
            core.touch_command_target = null;
            effects.observation = .fleet_launched;
        },
        .tune_field => effects.observation = .field_tuned,
    };
    return .{ .handled = true, .queued = queued, .effects = effects };
}
pub fn keyboardAction(core: *app.AppCore, action: CommandAction) DispatchError!Dispatch {
    if (core.mode() != .playing) return error.ModeRefused;
    return switch (action) {
        .launch => launchTo(core, core.hovered_world),
        .increase => adjustField(core, .increase),
        .decrease => adjustField(core, .decrease),
    };
}
fn launchTo(core: *app.AppCore, target: ?m.WorldId) DispatchError!Dispatch {
    const source = core.selected_world orelse return error.MissingSource;
    const destination = target orelse return error.MissingTarget;
    return queueGameCommand(core, .{ .launch = .{ .source = source, .destination = destination } });
}
pub fn contextualLaunch(core: *app.AppCore) DispatchError!Dispatch {
    if (core.mode() != .playing) return error.ModeRefused;
    return launchTo(core, commandTarget(core));
}
pub fn adjustField(core: *app.AppCore, adjustment: m.FieldAdjustment) DispatchError!Dispatch {
    if (core.mode() != .playing) return error.ModeRefused;
    const selected = core.selected_world orelse return error.MissingSource;
    return queueGameCommand(core, .{ .tune_field = .{ .world = selected, .field = core.selected_field, .adjustment = adjustment } });
}
pub fn rightClick(core: *app.AppCore, hit: ?m.WorldId) DispatchError!Dispatch {
    if (core.mode() != .playing or hit == null or core.selected_world == null) return .{};
    try admitHit(core, hit);
    core.hovered_world = hit;
    core.touch_command_target = null;
    return launchTo(core, hit);
}
