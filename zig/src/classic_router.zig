//! Serialized, headless Classic projection/guide coordinator. Pin before init;
//! Session/Workspace and all returned borrows end before any mutation/deinit.
const std = @import("std");
const H1 = @import("retained_semantics.zig");
const Input = @import("classic_input.zig");
const Session = @import("classic_session.zig");
const Core = @import("classic_app.zig");
const UI = @import("classic_ui.zig");
const EditorUI = @import("classic_editor_ui.zig");
const Editor = @import("classic_editor.zig");
const Layout = @import("classic_editor_layout.zig");
const Guide = @import("classic_guide.zig");
const Marker = @import("classic_start_marker_adapter.zig");
const Math = @import("classic_math.zig");
const Interaction = @import("classic_interaction.zig");
const Store = @import("classic_store.zig");
pub const Controller = Interaction.InteractionController(Session.UiAction, 8);
pub const none: u8 = 255;
pub const FocusDomain = enum(u8) { hud, settings, editor, guide };
pub const FocusSnapshot = extern struct {
    domain: u8 = 0,
    base_len: u8 = 0,
    base: [28]u8 = @splat(none),
    modal_present: u8 = 0,
    modal_len: u8 = 0,
    modal: [2]u8 = @splat(none),
    focused: u8 = none,
    restore: u8 = none,
};
pub const ContextKey = struct {
    surface: enum(u8) { classic, leaving_classic } = .classic,
    base: enum(u8) { playing, settings, editor },
    section: enum(u8) { none, worlds, rules, validation } = .none,
    world: u8 = none,
    confirmation: u8 = none,
    guide: bool = false,
    pub fn encode(self: ContextKey) error{InvalidContext}![13]u8 {
        if (self.surface == .leaving_classic and self.guide) return error.InvalidContext;
        if (self.base != .editor) {
            if (self.section != .none or self.world != none or self.confirmation != none) return error.InvalidContext;
        } else {
            if (self.section == .none or (self.confirmation != none and self.confirmation > 6)) return error.InvalidContext;
            if (self.section == .worlds) {
                if (self.world > 6) return error.InvalidContext;
            } else if (self.world != none) return error.InvalidContext;
        }
        return .{ 'C', '|', switch (self.base) { .playing => 'P', .settings => 'S', .editor => 'E' }, '|', switch (self.section) { .none => '-', .worlds => 'W', .rules => 'R', .validation => 'V' }, '|', if (self.world == none) '-' else '0' + self.world, '|', if (self.confirmation == none) '-' else '0' + self.confirmation, '|', if (self.guide) '1' else '0', '|', if (self.surface == .leaving_classic) '1' else '0' };
    }
};
pub const ShellEffect = enum { none, return_to_main_menu };
pub const Projection = enum { stale, current };
pub const RouteResult = struct { handled: bool = false, shell: ShellEffect = .none, session: ?Session.Result = null, projection: enum { unchanged, current, stale } = .unchanged };
pub const GuideRequest = union(enum) { open, show: Guide.Location, close, main_menu };
pub const RecoveryStamp = struct { revision: u64, owner: H1.OwnerSpec };
pub const Recovery = enum { close_current_guide, retry_projection };
pub const Redraw = struct { held: bool, stale: bool, outcome: ?Core.FrameOutcome, permit_advisory_poll_submit: bool };
pub const RouterError = UI.PrepareError || UI.PublishError || UI.AttachError || UI.SetHudFocusError || EditorUI.PrepareError || EditorUI.PublishError || Guide.PrepareError || Guide.PublishError || Marker.SnapshotError || Marker.AdapterError || Input.QueueError || H1.TokenCreateError || error{ RevisionExhausted, ClockClosed, RevisionConflict, InvalidContext, InvalidFocusSnapshot, SourceChanged, CoordinatorClosed, LeavingClassic, SourceBusy, PendingSlice, StaleProjection };
const Tickets = struct { resident: ?Input.SlotTicket = null, draft: ?Input.SlotTicket = null };
const OwnedPair = struct { resident: ?H1.TokenRef = null, draft: ?H1.TokenRef = null, tickets: Tickets = .{} };
const GuideState = struct { location: Guide.Location, hold: u64 };
const hud_ids = [_][]const u8{ "a/hud/speed_down", "a/hud/pause", "a/hud/speed_up", "a/hud/launch", "a/hud/clear_selection", "a/hud/reset_camera", "a/hud/settings", "a/hud/help", "a/hud/scenario", "a/hud/skip", "a/hud/field/atmosphere", "a/hud/field/hydrosphere", "a/hud/field/topology", "a/hud/field/decrease", "a/hud/field/increase", "a/hud/advisory_ack", "a/hud/reset_guidance", "a/hud/marker/0", "a/hud/marker/1", "a/hud/marker/2", "a/hud/marker/3", "a/hud/marker/4", "a/hud/marker/5", "a/hud/marker/6" };
const hud_actions = [_]Session.UiAction{ .speed_down, .toggle_pause, .speed_up, .launch, .clear_selection, .reset_camera, .open_settings, .toggle_help, .open_scenario, .skip_onboarding, .{ .select_field = .atmosphere }, .{ .select_field = .hydrosphere }, .{ .select_field = .topology }, .decrease_field, .increase_field, .acknowledge_advisory, .reset_onboarding, .{ .select_guidance_world = 0 }, .{ .select_guidance_world = 1 }, .{ .select_guidance_world = 2 }, .{ .select_guidance_world = 3 }, .{ .select_guidance_world = 4 }, .{ .select_guidance_world = 5 }, .{ .select_guidance_world = 6 } };
const settings_ids = [_][]const u8{ "a/settings/scale/85", "a/settings/scale/100", "a/settings/scale/115", "a/settings/scale/130", "a/settings/motion", "a/settings/contrast", "a/settings/quality/auto", "a/settings/quality/low", "a/settings/quality/high", "a/settings/reset_guidance", "a/settings/close" };
const guide_ids = [_][]const u8{ "guide.previous", "guide.next", "guide.menu", "guide.close" };
comptime {
    if (@sizeOf(FocusSnapshot) != 36 or @sizeOf(Layout.FocusKey) > 24) @compileError("router focus layout exceeds reviewed ceiling");
}
fn domainOf(key: ContextKey) FocusDomain {
    if (key.guide) return .guide;
    return switch (key.base) { .playing => .hud, .settings => .settings, .editor => .editor };
}
fn catalogId(domain: FocusDomain, code: u8) error{InvalidFocusSnapshot}!H1.ActionId {
    const bytes = switch (domain) {
        .hud => if (code < hud_ids.len) hud_ids[code] else return error.InvalidFocusSnapshot,
        .settings => if (code < settings_ids.len) settings_ids[code] else return error.InvalidFocusSnapshot,
        .editor => if (code < EditorUI.catalog.len) EditorUI.catalog[code].id else return error.InvalidFocusSnapshot,
        .guide => if (code < guide_ids.len) guide_ids[code] else return error.InvalidFocusSnapshot,
    };
    return .{ .bytes = bytes };
}
fn codeOf(domain: FocusDomain, id: ?H1.ActionId) error{InvalidFocusSnapshot}!u8 {
    const value = id orelse return none;
    const length: usize = switch (domain) { .hud => 24, .settings => 11, .editor => 109, .guide => 4 };
    for (0..length) |i| if (std.mem.eql(u8, value.bytes, (try catalogId(domain, @intCast(i))).bytes)) return @intCast(i);
    return error.InvalidFocusSnapshot;
}
fn optionalId(domain: FocusDomain, code: u8) error{InvalidFocusSnapshot}!?H1.ActionId {
    return if (code == none) null else try catalogId(domain, code);
}
fn editorKey(code: u8) error{InvalidFocusSnapshot}!?Layout.FocusKey {
    if (code == none) return null;
    if (code >= 109) return error.InvalidFocusSnapshot;
    return switch (EditorUI.catalog[code].target) { .field => |id| .{ .field = id }, .action => |action| .{ .action = action } };
}
fn keyCode(key: ?Layout.FocusKey) error{InvalidFocusSnapshot}!u8 {
    const k = key orelse return none;
    const target: EditorUI.Target = switch (k) { .field => |id| .{ .field = id }, .action => |a| .{ .action = a } };
    for (EditorUI.catalog, 0..) |entry, i| if (std.meta.eql(entry.target, target)) return @intCast(i);
    return error.InvalidFocusSnapshot;
}
fn snapshot(domain: FocusDomain, focus: H1.FocusSpec) error{InvalidFocusSnapshot}!FocusSnapshot {
    if (focus.base.len > 28) return error.InvalidFocusSnapshot;
    var result = FocusSnapshot{ .domain = @intFromEnum(domain), .base_len = @intCast(focus.base.len), .focused = try codeOf(domain, focus.focused), .restore = try codeOf(domain, focus.restore) };
    for (focus.base, 0..) |id, i| {
        const code = try codeOf(domain, id);
        if (domain == .editor and code >= 107) return error.InvalidFocusSnapshot;
        for (result.base[0..i]) |previous| if (previous == code) return error.InvalidFocusSnapshot;
        result.base[i] = code;
    }
    if (focus.modal) |modal| {
        if (domain != .editor or modal.len != 2) return error.InvalidFocusSnapshot;
        result.modal_present = 1;
        result.modal_len = 2;
        for (modal, 0..) |id, i| {
            const code = try codeOf(domain, id);
            if (code < 107) return error.InvalidFocusSnapshot;
            result.modal[i] = code;
        }
        if (result.modal[0] == result.modal[1]) return error.InvalidFocusSnapshot;
    }
    const active = if (result.modal_present == 1) result.modal[0..result.modal_len] else result.base[0..result.base_len];
    if (result.focused != none) {
        var found = false;
        for (active) |code| if (code == result.focused) { found = true; };
        if (!found) return error.InvalidFocusSnapshot;
    }
    return result;
}

/// Move-only by convention; fields are state, not independent mutation APIs.
pub const Coordinator = struct {
    allocator: std.mem.Allocator,
    session: *Session.Session,
    workspace: *Layout.Workspace,
    preparation: H1.PreparationBudget,
    tokens: H1.LiveTokenBudget,
    registry: Input.ReferenceRegistry,
    clock: Input.RevisionClock,
    queue: Input.SemanticQueue = .{},
    frame: H1.FrameSlot,
    roots: OwnedPair = .{},
    frame_tickets: Tickets = .{},
    saved_owners: OwnedPair = .{},
    input: Input.InputState = Input.InputState.init(),
    contacts: Controller = Controller.init(),
    context_key: ContextKey,
    context: [13]u8,
    published_context: [13]u8 = undefined,
    saved_context: [13]u8 = undefined,
    published_key: ?ContextKey = null,
    saved_key: ?ContextKey = null,
    published_revision: ?u64 = null,
    guide_publication: Guide.GuidePublication = .{},
    geometry: ?UI.GeometryView = null,
    focus: FocusSnapshot = .{},
    saved_focus: FocusSnapshot = .{},
    source_focus: u8 = none,
    saved_source_focus: u8 = none,
    editor_scratch: [28]Layout.FocusKey = undefined,
    confirmation_hold: ?u64 = null,
    guide_state: ?GuideState = null,
    saved_present: bool = false,
    projection: Projection = .stale,
    baseline: ?u128 = null,
    resumed_zero: bool = false,
    leased: bool = false,
    closed: bool = false,
    reference_peak: usize = 0,

    pub fn init(self: *Coordinator, allocator: std.mem.Allocator, session: *Session.Session, workspace: *Layout.Workspace) RouterError!void {
        self.* = .{ .allocator = allocator, .session = session, .workspace = workspace, .preparation = H1.PreparationBudget.init(1179648), .tokens = H1.LiveTokenBudget.init(.{ .max_live_objects = 32, .max_live_requested_bytes = 1280 }), .registry = Input.ReferenceRegistry.init(), .clock = .{}, .frame = undefined, .context_key = undefined, .context = undefined, .leased = true };
        self.frame = H1.FrameSlot.init(allocator, &self.preparation);
        errdefer self.deinit();
        self.roots.tickets.resident = try self.reserve(.root_resident);
        self.roots.resident = H1.TokenRef.create(allocator, &self.tokens) catch |failure| {
            self.cancelTickets(&self.roots.tickets);
            return failure;
        };
        self.mark(self.roots.tickets.resident.?);
        if (session.editor.session != null) {
            const ticket = try self.reserve(.root_draft);
            const token = H1.TokenRef.create(allocator, &self.tokens) catch |failure| {
                self.registry.cancel(ticket) catch unreachable;
                return failure;
            };
            self.roots.draft = token;
            self.roots.tickets.draft = ticket;
            self.mark(ticket);
        }
        self.queue = try Input.SemanticQueue.init(allocator, &self.registry);
        self.context_key = try self.sourceKey();
        self.context = try self.context_key.encode();
        const revision = try self.clock.next();
        try self.clock.commit(0, revision);
        if (self.context_key.confirmation != none) self.confirmation_hold = revision;
        self.source_focus = try self.sourceCode();
        if (self.context_key.confirmation != none) try self.initialModalBase();
        try self.project();
        self.leased = false;
    }
    fn reserve(self: *Coordinator, purpose: Input.SlotPurpose) RouterError!Input.SlotTicket {
        const ticket = try self.registry.reserve(purpose);
        // Read-only measurement, not a mutation API into the registry.
        var live: usize = 0;
        for (self.registry.entries) |entry| if (entry.state != .free) { live += 1; };
        self.reference_peak = @max(self.reference_peak, live);
        return ticket;
    }
    fn mark(self: *Coordinator, ticket: Input.SlotTicket) void { self.registry.markOwned(ticket) catch unreachable; }
    fn reserveTickets(self: *Coordinator, purpose: Input.SlotPurpose, draft: bool) RouterError!Tickets {
        var result = Tickets{ .resident = try self.reserve(purpose) };
        errdefer self.cancelTickets(&result);
        if (draft) result.draft = try self.reserve(purpose);
        return result;
    }
    fn cancelTickets(self: *Coordinator, tickets: *Tickets) void {
        if (tickets.draft) |ticket| self.registry.cancel(ticket) catch unreachable;
        if (tickets.resident) |ticket| self.registry.cancel(ticket) catch unreachable;
        tickets.* = .{};
    }
    fn releaseTickets(self: *Coordinator, tickets: *Tickets) void {
        if (tickets.draft) |ticket| self.registry.release(ticket) catch unreachable;
        if (tickets.resident) |ticket| self.registry.release(ticket) catch unreachable;
        tickets.* = .{};
    }
    fn destroyPair(self: *Coordinator, pair: *OwnedPair) void {
        if (pair.draft) |token| token.release();
        if (pair.resident) |token| token.release();
        self.releaseTickets(&pair.tickets);
        pair.* = .{};
    }
    fn retainPair(self: *Coordinator, owner: H1.OwnerSpec) RouterError!OwnedPair {
        var tickets = try self.reserveTickets(.guide_hold, owner.draft != null);
        errdefer self.cancelTickets(&tickets);
        const resident = try owner.resident.retain();
        errdefer resident.release();
        const draft = if (owner.draft) |token| try token.retain() else null;
        self.mark(tickets.resident.?);
        if (tickets.draft) |ticket| self.mark(ticket);
        return .{ .resident = resident, .draft = draft, .tickets = tickets };
    }
    fn enter(self: *Coordinator) RouterError!void {
        if (self.closed) return error.CoordinatorClosed;
        if (self.context_key.surface == .leaving_classic) return error.LeavingClassic;
        if (self.leased) return error.SourceBusy;
        self.leased = true;
    }
    fn sourceKey(self: *const Coordinator) RouterError!ContextKey {
        var source_key = ContextKey{ .base = switch (self.session.core.mode()) { .playing => .playing, .settings => .settings, .editing_scenario => .editor } };
        if (source_key.base == .editor) {
            const editor = self.session.editor.session orelse return error.NoEditor;
            if (editor.selected_world > 6) return error.InvalidContext;
            source_key.section = switch (editor.section) { .worlds => .worlds, .rules => .rules, .validation => .validation };
            if (editor.section == .worlds) source_key.world = @intCast(editor.selected_world);
            if (editor.modal.kind) |kind| source_key.confirmation = @intCast(@intFromEnum(kind));
        }
        _ = try source_key.encode();
        return source_key;
    }
    fn sourceCode(self: *const Coordinator) RouterError!u8 {
        return switch (self.context_key.base) {
            .playing => blk: {
                const value = self.session.hud_focus orelse break :blk none;
                for (hud_actions, 0..) |action, i| if (std.meta.eql(value, action)) break :blk @intCast(i);
                return error.InvalidFocusSnapshot;
            },
            .settings => blk: { const index = self.session.settings.focusIndex(); if (index > 10) return error.InvalidFocusSnapshot; break :blk @intCast(index); },
            .editor => blk: {
                const editor = self.session.editor.session orelse return error.NoEditor;
                const focus_key: ?Layout.FocusKey = if (editor.focus) |f| switch (f) { .field => |id| .{ .field = id }, .action => |a| .{ .action = a } } else null;
                break :blk try keyCode(focus_key);
            },
        };
    }
    fn checkSource(self: *const Coordinator) RouterError!void {
        var actual = try self.sourceKey();
        actual.guide = self.context_key.guide;
        actual.surface = self.context_key.surface;
        if (!std.meta.eql(actual, self.context_key)) return error.SourceChanged;
        const expected = if (self.saved_present) self.saved_source_focus else self.source_focus;
        if (try self.sourceCode() != expected) return error.SourceChanged;
        if ((self.session.editor.session != null) != (self.roots.draft != null)) return error.SourceChanged;
    }
    fn validateSource(self: *Coordinator) RouterError!void {
        self.checkSource() catch |failure| { self.projection = .stale; return failure; };
    }
    pub fn current(self: *const Coordinator) Input.CurrentView {
        std.debug.assert(!self.closed);
        return .{ .revision = self.clock.value, .owner = .{ .resident = self.roots.resident.?, .draft = if (self.context_key.base == .editor) self.roots.draft else null, .hold = if (self.guide_state) |g| g.hold else self.confirmation_hold, .context = &self.context } };
    }
    pub fn view(self: *const Coordinator) ?H1.FrameView {
        if (self.closed) return null;
        return self.frame.view();
    }
    fn initialModalBase(self: *Coordinator) RouterError!void {
        const editor = &self.session.editor.session.?;
        const report = editor.validationReport();
        const viewport_value = self.session.core.viewport();
        const focus_key = try editorKey(self.source_focus);
        const layout = try Layout.layout(.{ .viewport = .{ viewport_value.x, viewport_value.y }, .section = editor.section, .selected_world = @intCast(editor.selected_world), .focused = focus_key, .scroll = editor.scroll, .modal = null, .can_apply = !editor.hasParseErrors() and report.isValid(), .has_store_status = self.session.editor.status != .none, .issue_count = report.len }, self.workspace);
        defer Layout.releaseView(self.workspace);
        self.focus = .{ .domain = @intFromEnum(FocusDomain.editor), .modal_present = 1, .modal_len = 2, .modal = .{ 107, 108 }, .restore = self.source_focus };
        for (layout.widgets) |d| if (d.enabled and d.focusable) {
            const k: ?Layout.FocusKey = switch (d.key) { .section => |s| .{ .action = .{ .select_section = s } }, .world_choice => |w| .{ .action = .{ .select_world = w } }, .field => |f| .{ .field = f }, .footer => |f| .{ .action = switch (f) { .revert => .revert, .factory_defaults => .factory_defaults, .regenerate_from_seed => .regenerate_from_seed, .load => .load, .save => .save, .cancel => .cancel, .apply_and_restart => .apply_and_restart } }, else => null };
            if (k != null) { if (self.focus.base_len == 28) return error.InvalidFocusSnapshot; self.focus.base[self.focus.base_len] = try keyCode(k); self.focus.base_len += 1; }
        };
    }
    fn editorFocus(self: *Coordinator) RouterError!EditorUI.FocusInput {
        const history = if (self.saved_present) self.saved_focus else self.focus;
        if (self.context_key.confirmation == none) return .{ .current = try editorKey(if (self.saved_present) self.saved_source_focus else self.source_focus) };
        if (history.domain != @intFromEnum(FocusDomain.editor) or history.modal_present != 1 or history.base_len > 28) return error.InvalidFocusSnapshot;
        for (history.base[0..history.base_len], 0..) |code, i| self.editor_scratch[i] = (try editorKey(code)).?;
        return .{ .current = try editorKey(history.focused), .saved_base = .{ .order = self.editor_scratch[0..history.base_len], .restore = try editorKey(history.restore) } };
    }
    fn published(self: *Coordinator, tickets: *Tickets, next_focus: FocusSnapshot, geometry_value: ?UI.GeometryView) void {
        self.mark(tickets.resident.?);
        if (tickets.draft) |ticket| self.mark(ticket);
        self.releaseTickets(&self.frame_tickets);
        self.frame_tickets = tickets.*;
        tickets.* = .{};
        self.focus = next_focus;
        self.published_context = self.context;
        self.published_key = self.context_key;
        self.published_revision = self.clock.value;
        self.geometry = geometry_value;
        self.projection = .current;
        if (self.guide_state == null) {
            self.guide_publication = .{};
            if (self.saved_present) {
                self.source_focus = self.saved_source_focus;
                self.destroyPair(&self.saved_owners);
                self.saved_present = false;
                self.saved_key = null;
                self.saved_focus = .{};
                self.saved_source_focus = none;
            }
        }
    }
    fn project(self: *Coordinator) RouterError!void {
        self.checkSource() catch |failure| { self.projection = .stale; return failure; };
        self.projectOnce() catch |failure| { self.projection = .stale; return failure; };
    }
    fn projectOnce(self: *Coordinator) RouterError!void {
        const current_value = self.current();
        var header_tickets = try self.reserveTickets(.transition, current_value.owner.draft != null);
        var header_owned = false;
        defer if (header_owned) self.releaseTickets(&header_tickets) else self.cancelTickets(&header_tickets);
        if (self.guide_state) |state| {
            var prepared = try Guide.prepareGuide(self.allocator, &self.preparation, .{ .source = .{ .location = state.location, .viewport = self.session.core.viewport(), .high_contrast = self.session.core.preferences().high_contrast, .focused = try optionalId(.guide, self.source_focus) }, .current = .{ .revision = current_value.revision, .owner = current_value.owner } });
            self.mark(header_tickets.resident.?); if (header_tickets.draft) |ticket| self.mark(ticket); header_owned = true;
            defer prepared.deinit();
            const next_focus = try snapshot(.guide, (try prepared.input()).focus);
            const geometry_value = try prepared.geometry();
            var candidate = try self.reserveTickets(.staged_frame, current_value.owner.draft != null);
            errdefer self.cancelTickets(&candidate);
            try self.validateSource();
            try Guide.publishGuide(&self.frame, &self.guide_publication, &prepared, .{ .revision = current_value.revision, .owner = current_value.owner });
            self.guide_state.?.location = geometry_value.location;
            self.published(&candidate, next_focus, null);
        } else if (self.context_key.base == .editor) {
            var prepared = try EditorUI.prepareEditorFromSession(self.allocator, &self.preparation, self.workspace, self.session, try self.editorFocus(), .{ .revision = current_value.revision, .owner = current_value.owner });
            self.mark(header_tickets.resident.?); if (header_tickets.draft) |ticket| self.mark(ticket); header_owned = true;
            defer prepared.deinit();
            const next_focus = try snapshot(.editor, (try prepared.input()).focus);
            var candidate = try self.reserveTickets(.staged_frame, true);
            errdefer self.cancelTickets(&candidate);
            try self.validateSource();
            try EditorUI.publishEditor(&self.frame, &self.published_revision, &prepared, .{ .revision = current_value.revision, .owner = current_value.owner });
            self.published(&candidate, next_focus, null);
        } else {
            var prepared = try UI.prepareHudSettingsFromSession(self.allocator, &self.preparation, self.session, null, .{ .revision = current_value.revision, .owner = current_value.owner });
            self.mark(header_tickets.resident.?); header_owned = true;
            defer prepared.deinit();
            if (self.context_key.base == .playing) {
                var hud: Marker.BorrowedHudSnapshot = undefined;
                const stamp = UI.CurrentStamp{ .revision = current_value.revision, .owner = current_value.owner };
                try Marker.snapshotFromPrepared(&prepared, stamp, &hud);
                const marker = try Marker.projectFromCore(&.{ .core = &self.session.core, .step = self.session.onboarding.step(), .stamp = stamp }, &hud);
                try UI.attachStartMarker(&prepared, marker);
                const code = if (self.saved_present) self.saved_source_focus else self.source_focus;
                const wanted = try optionalId(.hud, code);
                var present: ?H1.ActionId = null;
                { const input_value = prepared.input(); if (wanted) |id| for (input_value.focus.base) |entry| if (std.mem.eql(u8, id.bytes, entry.bytes)) { present = entry; break; }; }
                try prepared.setHudPublishedFocus(present);
            }
            const next_focus = try snapshot(domainOf(self.context_key), prepared.input().focus);
            const geometry_value = try prepared.geometry();
            var candidate = try self.reserveTickets(.staged_frame, false);
            errdefer self.cancelTickets(&candidate);
            try self.validateSource();
            try UI.publishHudSettings(&self.frame, &self.published_revision, &prepared, .{ .revision = current_value.revision, .owner = current_value.owner });
            self.published(&candidate, next_focus, geometry_value);
        }
    }
    pub fn guide(self: *Coordinator, request: GuideRequest) RouterError!RouteResult {
        try self.enter(); defer self.leased = false;
        try self.validateSource();
        switch (request) {
            .open => {
                if (self.guide_state != null) { self.baseline = null; return .{ .handled = true }; }
                if (self.projection != .current) return error.StaleProjection;
                const revision = try self.clock.next();
                const current_value = self.current();
                const old_view = self.frame.view() orelse return error.StaleProjection;
                if (self.published_revision != current_value.revision or !old_view.owner.matches(current_value.owner)) return error.StaleProjection;
                const saved = try snapshot(domainOf(self.context_key), old_view.focus);
                if (self.context_key.base == .settings and (saved.focused != self.source_focus or self.source_focus > 10)) return error.InvalidFocusSnapshot;
                const owners = try self.retainPair(current_value.owner);
                self.saved_owners = owners;
                self.saved_focus = saved;
                self.saved_source_focus = self.source_focus;
                self.saved_context = self.context;
                self.saved_key = self.context_key;
                self.saved_present = true;
                self.guide_state = .{ .location = .{ .topic = 1, .sheet = 0 }, .hold = revision };
                self.context_key.guide = true;
                self.context = try self.context_key.encode();
                self.source_focus = none;
                self.input.clearAll(); self.contacts.cancel();
                self.baseline = null;
                try self.clock.commit(self.clock.value, revision);
            },
            .show => |location| {
                if (self.guide_state == null) return .{};
                if (std.meta.eql(self.guide_state.?.location, location)) { self.baseline = null; return .{ .handled = true }; }
                return self.showGuide(location);
            },
            .close, .main_menu => {
                if (self.guide_state == null) return .{};
                const revision = try self.clock.next();
                if (!self.saved_present or self.saved_key == null or !self.roots.resident.?.sameIdentity(self.saved_owners.resident.?)) return error.SourceChanged;
                if (self.context_key.base == .editor and !self.roots.draft.?.sameIdentity(self.saved_owners.draft.?)) return error.SourceChanged;
                self.guide_state = null;
                self.context_key = self.saved_key.?;
                self.context = self.saved_context;
                self.baseline = null;
                self.resumed_zero = true;
                try self.clock.commit(self.clock.value, revision);
                if (request == .main_menu) {
                    self.context_key.surface = .leaving_classic;
                    self.context = try self.context_key.encode();
                    self.projection = .stale;
                    return .{ .handled = true, .shell = .return_to_main_menu, .projection = .stale };
                }
            },
        }
        try self.project();
        return .{ .handled = true, .projection = .current };
    }
    // The actual producer normalizes before admitting a Show source change.
    // Its one Header becomes the publication source; no second preparation.
    fn showGuide(self: *Coordinator, location: Guide.Location) RouterError!RouteResult {
        const revision = try self.clock.next();
        const owner = self.current().owner;
        var header_tickets = try self.reserveTickets(.transition, owner.draft != null);
        var owned = false;
        defer if (owned) self.releaseTickets(&header_tickets) else self.cancelTickets(&header_tickets);
        var prepared = try Guide.prepareGuide(self.allocator, &self.preparation, .{ .source = .{ .location = location, .viewport = self.session.core.viewport(), .high_contrast = self.session.core.preferences().high_contrast, .focused = try optionalId(.guide, self.source_focus) }, .current = .{ .revision = revision, .owner = owner } });
        self.mark(header_tickets.resident.?); if (header_tickets.draft) |ticket| self.mark(ticket); owned = true;
        defer prepared.deinit();
        const geometry_value = try prepared.geometry();
        const next_focus = try snapshot(.guide, (try prepared.input()).focus);
        if (std.meta.eql(self.guide_state.?.location, geometry_value.location)) {
            self.baseline = null;
            return .{ .handled = true };
        }
        // All borrowed input/source spans have ended before source admission.
        self.guide_state.?.location = geometry_value.location;
        self.baseline = null;
        try self.clock.commit(self.clock.value, revision);
        self.projection = .stale;
        var candidate = try self.reserveTickets(.staged_frame, owner.draft != null);
        errdefer self.cancelTickets(&candidate);
        try self.validateSource();
        try Guide.publishGuide(&self.frame, &self.guide_publication, &prepared, .{ .revision = revision, .owner = self.current().owner });
        self.published(&candidate, next_focus, null);
        return .{ .handled = true, .projection = .current };
    }
    pub fn recover(self: *Coordinator, stamp: RecoveryStamp, operation: Recovery) RouterError!RouteResult {
        if (self.closed) return error.CoordinatorClosed;
        const now = self.current();
        if (stamp.revision != now.revision or !stamp.owner.matches(now.owner)) return error.StaleProjection;
        if (operation == .close_current_guide) return self.guide(.close);
        try self.enter(); defer self.leased = false;
        try self.project();
        return .{ .handled = true, .projection = .current };
    }
    pub fn viewport(self: *Coordinator, value: Math.Vec2) RouterError!void {
        try self.enter(); defer self.leased = false;
        try self.validateSource();
        if (!value.isFinite()) return error.NonFinite;
        const normalized = Math.Vec2{ .x = @max(value.x, 0), .y = @max(value.y, 0) };
        if (std.meta.eql(self.session.core.viewport(), normalized)) return;
        const revision = try self.clock.next();
        self.session.core.setViewport(normalized);
        try self.clock.commit(self.clock.value, revision);
        try self.project();
    }
    pub fn redraw(self: *Coordinator, now_ns: u128) RouterError!Redraw {
        try self.enter(); defer self.leased = false;
        try self.validateSource();
        if (self.guide_state != null) {
            self.baseline = now_ns;
            return .{ .held = true, .stale = self.projection == .stale, .outcome = null, .permit_advisory_poll_submit = false };
        }
        const revision = try self.clock.next();
        const delta = if (self.baseline) |previous| now_ns -| previous else 0;
        const fingerprint = self.session.core.canonicalFingerprint();
        const alpha = self.session.core.clockAlpha();
        const camera = self.session.core.camera();
        const visual_time = self.session.core.visual_time;
        const outcome = self.session.core.advance(delta);
        self.baseline = now_ns;
        self.resumed_zero = false;
        const changed = fingerprint != self.session.core.canonicalFingerprint() or alpha != self.session.core.clockAlpha() or !std.meta.eql(camera, self.session.core.camera()) or visual_time != self.session.core.visual_time or outcome.completed_steps != 0;
        if (changed) {
            try self.clock.commit(self.clock.value, revision);
            try self.project();
        }
        return .{ .held = false, .stale = self.projection == .stale, .outcome = outcome, .permit_advisory_poll_submit = self.projection == .current };
    }
    pub fn key(_: *Coordinator, _: Input.NormalizedKeyEvent, _: Store.RawStore) error{PendingSlice}!RouteResult { return error.PendingSlice; }
    pub fn ime(_: *Coordinator, _: Input.NormalizedIme) error{PendingSlice}!RouteResult { return error.PendingSlice; }
    pub fn drain(_: *Coordinator, _: Store.RawStore) error{PendingSlice}!RouteResult { return error.PendingSlice; }
    pub fn deinit(self: *Coordinator) void {
        if (self.closed) return;
        self.queue.deinit();
        self.contacts.cancel(); self.input.clearAll();
        self.frame.deinit();
        self.releaseTickets(&self.frame_tickets);
        self.destroyPair(&self.saved_owners);
        self.destroyPair(&self.roots);
        self.guide_publication = .{};
        self.clock.deinit(); self.registry.deinit(); self.preparation.deinit(); self.tokens.deinit();
        self.closed = true;
    }
};
