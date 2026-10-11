//! Pure offline guide. Caller owns guide activation and current owner lease.
const std = @import("std");
const H1 = @import("retained_semantics.zig");
const math = @import("classic_math.zig");
pub const Topic = enum(u3) { choose_game, classic_first_fleet, workshop_first_energy, workshop_ore_alloy, living_first_simulation, controls, save_exit_graphics };
pub const TopicText = struct { title: []const u8, paragraphs: []const []const u8 };
pub const topics = [_]TopicText{
    .{ .title = "Choose your game", .paragraphs = &.{
        "Classic Sector is a strategy battle. You command the Union: capture five of seven worlds before a rival does. Galaxy Workshop is a separate creative sandbox: build a galaxy, run production, inspect it and revise. It has no enemy or victory screen.",
        "Living Galaxy lets you observe autonomous societies and inspect their inventories and recorded events. First time? Use Next for a first fleet in Classic, continue to Workshop for your first 4 energy, or read Living Galaxy for a paused simulation experiment. No account, server or agent commands are needed.",
        "Open this guide with F1, the main-menu Player guide button, Workshop's Guide button, or Classic's HELP / H. Simulation time is held while reading. Close or Escape returns to the same session and speed; no save or scenario is replaced.",
    } },
    .{ .title = "Classic // First fleet", .paragraphs = &.{
        "GOAL: You are Union. Control five of seven worlds to win. Losing every Union world and fleet ends your campaign. Energy funds orders and fleets; defense is the force protecting a world.",
        "FIRST SUCCESS: If the old six-step tutorial is active, click SKIP to unlock the speed controls. Press P to pause if needed. Click a Union world, then hover a different world to preview the launch in the command tray. Do not left-click the destination: that changes your selection.",
        "Read source, destination, strength and any rejection in the tray. A launch sends half the source's current energy as fleet strength and consumes that energy. Source defense stays unchanged. The fleet snapshots the source Hydrosphere for its travel speed.",
        "Press Space with keyboard focus off the HUD, or right-click the destination, to issue the order. Press P to run. Your first result is a moving fleet and lower source energy, not an instant capture. Friendly arrivals reinforce defense; hostile arrivals fight defense.",
        "If an order is rejected, select your own world with enough energy and a different destination. Space activates a focused HUD button instead of launching. Clicking your source clears HUD focus. Use the tray's rejection text as the reason to fix.",
    } },
    .{ .title = "Workshop // First energy", .paragraphs = &.{
        "GOAL: Make a working galaxy, not a battle fleet. Start with New Workshop for a blank galaxy, or Continue for your save. These instructions also work with new objects in an existing galaxy. Do not replace your save just to learn.",
        "Keep time paused. Click Create system and Apply (defaults are fine). Select that system in the left hierarchy. Create star and Apply. Select that star, then Create world and Apply. Factions and ownership are optional for this first production result.",
        "Select your new world in the hierarchy. Click Place industry. Check World is your new world and Industry is solar-array. Apply. New industries start enabled: do NOT toggle them off. Click Step once while paused.",
        "FIRST SUCCESS: The tick increases by one. Select the world again: the visible Inventory line now shows 4 energy. A solar array makes 4 energy per tick. Save Workshop and wait for Saved and selected for Continue. If the bottom controls are missing, widen the window.",
        "Forms are drafts until Apply. Tab / Shift+Tab moves focus; Enter or Space activates. For a choice, activate repeatedly to cycle. Focus a text or number field and type to replace its value; Backspace edits. Escape cancels the form without applying it.",
    } },
    .{ .title = "Workshop // Ore to alloy", .paragraphs = &.{
        "On the same world, Create deposit: choose ore and a positive reserve. Place industry again: choose extractor, that world and that linked deposit. Keep your solar-array enabled. Step once: extraction uses 1 energy to make 2 ore, reducing the reserve. Deposits alone do not mine anything.",
        "Place a foundry on that world with no linked deposit. Each operating tick consumes 2 energy and 3 ore to make 1 alloy. With one solar array and extractor, ore needs time to accumulate; not every tick can run the foundry. Select the world and watch Inventory for alloy.",
        "For shipping, build a second system, star and world. Connect lane joins the SYSTEMS. Connect route joins the WORLDS: choose source, destination, ore, and batch units 1. A lane by itself ships nothing. Run 1x until ore reaches the destination; travel is not instantaneous.",
        "Put a solar-array and foundry on the receiving world to turn delivered ore into alloy there. If nothing happens: check speed, industry Enabled, matching deposit, reserves, local energy/ore, lane, source/destination and route batch size. An ion storm reduces lane capacity; leave hazards out of your first factory.",
    } },
    .{ .title = "Living Galaxy // First simulation", .paragraphs = &.{
        "1 / SELECTION: Open Living Galaxy to start paused in Vale Confluence. Select a world in the diorama, or open Navigator to search names, full identities or entity types. Expand a system and inspect a world, civilization, facility, fleet, route or hazard. Selection changes presentation only; Inspector reads current authority fields and recorded recipe causes.",
        "2 / TIME: Step or the period key advances one boundary while paused. Check the tick and selected inventory before and after. Run/Pause or P toggles time outside text fields. Start at 1x: ten ticks per second. 4x and 20x request faster transport; the visible effective speed can be lower under load. Pause, modal activity and suspension discard timing debt.",
        "3 / CREATION: Pause, choose Creator tools, select System and give it a name. Fields are drafts. Preview validates a private candidate and explains its changes; Apply commits only that current preview. Cancel preserves the galaxy. Removals require the displayed dependent objects and derived references to be reviewed. Editing retained history requires Branch here.",
        "FIRST SUCCESS: Find a committed inventory change or event you can explain. Chronicle shows newest verified rows first; repetitive production and freight are summarized in 100-tick windows. Open a row for exact receipt references. Decision rows show recorded actor, score, outcome and reasons. Ordinary events without actor/entity payloads explicitly retain unknown links.",
        "CAMERA: Galaxy, System and World buttons change scope. Empty-space drag pans; modified drag or the Camera tool orbits. Wheel zooms toward the pointer. F focuses selection and Home fits the current scope when scene focus is active. Camera offers visible equivalents. Removing a focused object returns to its surviving parent. Navigator and Inspector remain available when scene objects overlap.",
        "EXPERIMENT: In History, name a bookmark Before the intervention. Preview a temporary lane hazard, Apply it, then observe receipts rather than predict an unrecorded result. Branch preserves the sibling future. Matching-tick comparison evaluates both histories at one exact tick. Record an experiment only from the completed measured comparison. Notebook labels and observations are separate session metadata.",
        "LEARNING GALAXIES: New galaxy offers paused Vale Confluence, Commerce, Frontier Friction, Interrupted Corridor, First Expansion, Blank and reproducible Seeded starts. Main Vale has no guaranteed first war. Commerce, Frontier and Interrupted are deterministic fixture situations; their rules are unchanged while running. Save your current session before replacing it.",
        "PRESERVE: Save and wait for its durable status. Save & leave completes the archive and notebook save before leaving. Library opens exact saved identities and supports rename, archive, restore and deleting archived saves. Continue resolves its recorded selection; invalid data enters recovery. Transfer first prepares an immutable export snapshot, then exports Galaxy, Catalog or Notebook separately. Browser download confirmation is separate from download handoff. Import displays a validated candidate for explicit Apply or Cancel.",
        "ACCESS: Tab / Shift+Tab focuses controls; Enter or Space activates the focused action. Text fields support cursor movement, selection and editing. PageUp / PageDown reads inspector pages. Open F1 to read with time held, then Close to return to the same speed. A fault stops transport at the last committed authority boundary; read its message. Audio settings expose mute and separate volume channels.",
    } },
    .{ .title = "Controls // Mouse and keyboard", .paragraphs = &.{
        "MENUS / WORKSHOP: Click a visible control, or Tab / Shift+Tab (also arrow keys) to move focus, then Enter / Space. Select objects in the left hierarchy; the Workshop map is a display, not a direct placement surface. P toggles pause / 1x outside forms. Cmd/Ctrl+S saves. Use Pause, Step once, 1x, 4x, 20x on the timeline.",
        "CLASSIC: Left-click selects. Hover previews a destination; right-click issues launch. Space launches unless a HUD button has focus. Drag empty space or middle-drag to orbit; wheel / pinch zooms; C resets camera. P pauses; [ / ] changes speed. 1 / 2 / 3 selects atmosphere / hydrosphere / topology; Q / E lowers / raises it at an energy cost on your own world.",
        "CLASSIC: S opens Settings, F4 opens the detached Scenario editor, R restarts and loses current battle progress, Escape clears selection. The old observed-action tutorial can be reset in Settings. The guide's Main menu button leaves Classic; it does not save a battle.",
        "CONTROLLERS: No native gamepad bindings are implemented in this build. A controller requires an external mouse/keyboard mapping; there is no supported A/B or stick layout to promise. Classic world selection still needs a pointer. Workshop and this guide expose keyboard controls.",
    } },
    .{ .title = "Save, exit and graphics", .paragraphs = &.{
        "WORKSHOP: Save Workshop (Cmd/Ctrl+S) persists the current galaxy. Wait for Saved and selected for Continue, with no DIRTY marker, before quitting. Main menu / Escape requests a save; New Workshop or Classic can remain disabled while a save is pending. Continue reopens the selected Workshop save, not a Classic battle. Library, on the main menu and in the Workshop, opens, renames and archives saves, and picks the Continue save.",
        "Undo moves to an earlier recorded creator revision. Editing history creates a branch; it does not erase later work. Select a branch to return to it. Save after choosing the branch you want to continue. Library's Export, Export this galaxy and Export content pack prepare portable copies, but this build cannot write a copy to a file or import one, so Save copy and both import controls stay disabled.",
        "CLASSIC: Scenario SAVE stores the editor's setup, not the current battle. LOAD changes the detached draft; APPLY AND RESTART replaces the battle. CANCEL leaves the running scenario intact. R also restarts the battle. To exit the app, use the window close control or Main menu > Quit (native).",
        "GRAPHICS: The retro map is intentional. GPU/Metal/WebGPU labels describe the rendering backend; they do not mean ray tracing. No ray-tracing option is implemented. Classic's neural advisory is only a scoring hint and cannot play for you. Settings offers contrast and motion controls; Classic also has UI scale and graphics-quality choices.",
    } },
};

pub const Location = struct { topic: u8, sheet: u16 };
pub const Source = struct { location: Location, viewport: math.Vec2, high_contrast: bool, focused: ?H1.ActionId = null };
pub const CurrentStamp = struct { revision: u64, owner: H1.OwnerSpec };
pub const Input = struct { source: Source, current: CurrentStamp };
pub const Action = union(enum) { show: Location, close, main_menu };
/// Trusted pointer-free record, assigned only after successful frame replacement.
pub const GuidePublication = struct { revision: ?u64 = null, previous: ?Location = null, next: ?Location = null };
pub const PrepareError = H1.FrameError || error{ InvalidGeometry, ArithmeticOverflow, InvalidOwner };
pub const PublishError = H1.FrameError || error{ CurrentMismatch, BudgetMismatch, InvalidPublication };
pub const ActionError = error{ InvalidUtf8, NoPublication, CurrentMismatch, UnknownAction, InactiveAction, InvalidPublication };
pub const WordSpan = struct { paragraph: u8, start: u16, len: u16 };
pub const LineSpan = struct { first_word: u16, word_count: u16, upper_start: u16, upper_len: u16 };
pub const GuideGeometry = struct { revision: u64, viewport: math.Vec2, topic: Topic, location: Location, sheets: u16, nominal_columns: f32, nominal_rows: f32, effective_columns: u16, effective_rows: u16, high_contrast: bool };
pub const LineView = struct { text: []const u8, position: math.Vec2, scale: f32 };
pub const limits = H1.Limits{ .max_nodes = 24, .max_child_indices = 23, .max_id_bytes = 4096, .max_text_bytes = 16384, .max_focus_actions = 4, .max_announcements = 0, .max_owner_context_bytes = 128, .max_owned_bytes = 32768 };
const Metadata = struct {
    allocator: std.mem.Allocator,
    budget: *H1.PreparationBudget,
    charge: H1.Charge,
    resident: H1.TokenRef,
    draft: ?H1.TokenRef,
    retained_resident: bool = false,
    retained_draft: bool = false,
    hold: ?u64,
    revision: u64,
    context_len: usize,
    geometry: GuideGeometry,
    previous: ?Location,
    next: ?Location,
    node_count: usize = 0,
    child_count: usize = 0,
    focus_count: usize = 0,
    word_count: usize = 0,
    line_count: usize = 0,
    selected_start: usize,
    selected_count: usize,
    arena_used: usize = 0,
    preserved_start: usize = 0,
    preserved_len: usize = 0,
    upper_start: usize = 0,
    upper_len: usize = 0,
    focused: ?H1.ActionId = null,
};
const Header = struct {
    m: Metadata,
    context: [128]u8 = undefined,
    nodes: [24]H1.NodeSpec = undefined,
    children: [23]usize = undefined,
    focus: [4]H1.ActionId = undefined,
    words: [521]WordSpan = undefined,
    lines: [530]LineSpan = undefined,
    arena: [8192]u8 = undefined,
};
pub const HEADER_BYTES = @sizeOf(Header);
pub const METADATA_BYTES = @sizeOf(Metadata);
comptime {
    @setEvalBranchQuota(1000000);
    if (@sizeOf(usize) != 8 or @sizeOf(H1.NodeSpec) != 160 or @sizeOf(H1.ActionId) != 16) @compileError("H3d target differs from reviewed profile");
    var total_words: usize = 0;
    var total_paragraphs: usize = 0;
    var total_bytes: usize = 0;
    var title_bytes: usize = 0;
    for (topics) |topic| {
        title_bytes += topic.title.len;
        var topic_words: usize = 0;
        for (topic.paragraphs) |paragraph| {
            if (paragraph.len > 532) @compileError("H3d paragraph bound exceeded");
            for (paragraph) |byte| if (byte > 127) @compileError("H3d source corpus must remain ASCII");
            var words = std.mem.tokenizeAny(u8, paragraph, " \t\r\n\x0b\x0c");
            while (words.next() != null) topic_words += 1;
            total_paragraphs += 1;
            total_bytes += paragraph.len;
        }
        if (topic_words > 521 or topic_words + topic.paragraphs.len > 530) @compileError("H3d active span capacities exceeded");
        total_words += topic_words;
    }
    if (total_words != 1686 or total_paragraphs != 34 or total_bytes != 10591 or title_bytes != 172) @compileError("H3d independent corpus inventory changed");
    if (HEADER_BYTES > 32768 or METADATA_BYTES > 1024 or @sizeOf(WordSpan) > 16 or @sizeOf(LineSpan) > 16 or @sizeOf(PreparedGuide) != 8 or @sizeOf(GuidePublication) > 64) @compileError("H3d target exceeds reviewed capacity");
}
/// Move-only by convention. All borrowed slices end before move/deinit/publication.
pub const PreparedGuide = struct {
    header: ?*Header = null,
    pub fn deinit(self: *PreparedGuide) void {
        const h = self.header orelse return;
        const allocator = h.m.allocator;
        var charge = h.m.charge;
        if (h.m.retained_draft) h.m.draft.?.release();
        if (h.m.retained_resident) h.m.resident.release();
        allocator.destroy(h);
        charge.release();
        self.header = null;
    }
    pub fn input(self: *const PreparedGuide) error{EmptyProjection}!H1.FrameInput {
        const h = self.header orelse return error.EmptyProjection;
        return .{ .nodes = h.nodes[0..h.m.node_count], .child_indices = h.children[0..h.m.child_count], .announcements = &.{}, .owner = .{ .resident = h.m.resident, .draft = h.m.draft, .hold = h.m.hold, .context = h.context[0..h.m.context_len] }, .focus = .{ .base = h.focus[0..h.m.focus_count], .modal = null, .restore = null, .focused = h.m.focused } };
    }
    pub fn geometry(self: *const PreparedGuide) error{EmptyProjection}!GuideGeometry {
        return (self.header orelse return error.EmptyProjection).m.geometry;
    }
    pub fn bodyLines(self: *const PreparedGuide) error{EmptyProjection}![]const LineSpan {
        const h = self.header orelse return error.EmptyProjection;
        return h.lines[h.m.selected_start..][0..h.m.selected_count];
    }
    pub fn line(self: *const PreparedGuide, index: usize) error{ EmptyProjection, InvalidLine }!LineView {
        const h = self.header orelse return error.EmptyProjection;
        if (index >= h.m.selected_count) return error.InvalidLine;
        const span = h.lines[h.m.selected_start + index];
        return .{ .text = h.arena[span.upper_start..][0..span.upper_len], .position = .{ .x = 18, .y = 134 + @as(f32, @floatFromInt(index)) * 22 }, .scale = 1.6 };
    }
};
const WrapCounts = struct { words: usize = 0, lines: usize = 0, normalized_bytes: usize = 0, selected_bytes: usize = 0 };
/// Scalar pass when h=null; a single active-topic table when h is present.
fn wrap(paragraphs: []const []const u8, columns: usize, h: ?*Header) WrapCounts {
    return scan(paragraphs, columns, h, 0, 530);
}
fn countSelected(c: *WrapCounts, start: usize, end: usize, bytes: usize) void {
    if (c.lines >= start and c.lines < end) c.selected_bytes += bytes + @as(usize, if (c.lines > start) 1 else 0);
}
fn scan(paragraphs: []const []const u8, columns: usize, h: ?*Header, selected_start: usize, selected_end: usize) WrapCounts {
    var c = WrapCounts{};
    for (paragraphs, 0..) |paragraph, pi| {
        var it = std.mem.tokenizeAny(u8, paragraph, " \t\r\n\x0b\x0c");
        var line_start = c.words;
        var line_words: usize = 0;
        var line_bytes: usize = 0;
        var paragraph_words: usize = 0;
        while (it.next()) |word| {
            if (line_words > 0 and line_bytes + 1 + word.len > columns) {
                if (h) |header| header.lines[c.lines] = .{ .first_word = @intCast(line_start), .word_count = @intCast(line_words), .upper_start = 0, .upper_len = 0 };
                countSelected(&c, selected_start, selected_end, line_bytes);
                c.lines += 1;
                line_start = c.words;
                line_words = 0;
                line_bytes = 0;
            }
            if (h) |header| header.words[c.words] = .{ .paragraph = @intCast(pi), .start = @intCast(@intFromPtr(word.ptr) - @intFromPtr(paragraph.ptr)), .len = @intCast(word.len) };
            c.words += 1;
            c.normalized_bytes += word.len + @as(usize, if (paragraph_words > 0) 1 else 0);
            paragraph_words += 1;
            line_bytes += word.len + @as(usize, if (line_words > 0) 1 else 0);
            line_words += 1;
        }
        if (line_words > 0) {
            if (h) |header| header.lines[c.lines] = .{ .first_word = @intCast(line_start), .word_count = @intCast(line_words), .upper_start = 0, .upper_len = 0 };
            countSelected(&c, selected_start, selected_end, line_bytes);
            c.lines += 1;
        }
        if (h) |header| header.lines[c.lines] = .{ .first_word = @intCast(c.words), .word_count = 0, .upper_start = 0, .upper_len = 0 };
        countSelected(&c, selected_start, selected_end, 0);
        c.lines += 1;
    }
    return c;
}
fn footer(v: math.Vec2, index: usize) PrepareError!H1.Rect {
    @setFloatMode(.strict);
    const width = (v.x - 40) / 4;
    const x = 8 + @as(f32, @floatFromInt(index)) * (width + 8);
    const y = v.y - 62;
    const rect = H1.Rect.fromXYWH(x, y, width, 48) catch return error.ArithmeticOverflow;
    _ = H1.Rect.fromBounds(rect.min, rect.max) catch return error.InvalidGeometry;
    if (rect.max[0] <= rect.min[0] or rect.max[1] <= rect.min[1]) return error.InvalidGeometry;
    return rect;
}
fn geometryFor(input: Input) PrepareError!GuideGeometry {
    @setFloatMode(.strict);
    const s = input.source;
    const v: math.Vec2 = if (s.viewport.isFinite()) .{ .x = @max(s.viewport.x, 320), .y = @max(s.viewport.y, 320) } else .{ .x = 640, .y = 480 };
    const columns = @floor((v.x - 36) / (6 * @as(f32, 1.6)));
    const rows = @max(@as(f32, 1), @floor((v.y - 222) / 22));
    if (!std.math.isFinite(columns) or !std.math.isFinite(rows)) return error.ArithmeticOverflow;
    const ec: u16 = @intFromFloat(@min(columns, 532));
    const er: u16 = @intFromFloat(@min(rows, 530));
    const topic: u8 = @min(s.location.topic, 6);
    const counts = wrap(topics[topic].paragraphs, ec, null);
    if (counts.words > 521 or counts.lines > 530 or counts.normalized_bytes + 2 * topics[topic].paragraphs.len - 1 > 3565) return error.LimitExceeded;
    const sheets: u16 = @intCast((counts.lines + er - 1) / er);
    for (0..4) |i| _ = try footer(v, i);
    return .{ .revision = input.current.revision, .viewport = v, .topic = @fromBackingInt(@intCast(topic)), .location = .{ .topic = topic, .sheet = @min(s.location.sheet, sheets - 1) }, .sheets = sheets, .nominal_columns = columns, .nominal_rows = rows, .effective_columns = ec, .effective_rows = er, .high_contrast = s.high_contrast };
}
/// No caller borrow escapes. Budget outlives all Headers and H1 frames.
pub fn prepareGuide(allocator: std.mem.Allocator, budget: *H1.PreparationBudget, input: Input) PrepareError!PreparedGuide {
    if (input.current.owner.context.len > 128) return error.InvalidOwner;
    if (!std.unicode.utf8ValidateSlice(input.current.owner.context)) return error.InvalidUtf8;
    if (input.source.focused) |focus| if (!std.unicode.utf8ValidateSlice(focus.bytes)) return error.InvalidUtf8;
    const g = try geometryFor(input);
    const c = wrap(topics[g.location.topic].paragraphs, g.effective_columns, null);
    const start: usize = @as(usize, g.location.sheet) * g.effective_rows;
    const count = @min(@as(usize, g.effective_rows), c.lines - start);
    const previous: ?Location = if (g.location.sheet > 0) .{ .topic = g.location.topic, .sheet = g.location.sheet - 1 } else if (g.location.topic > 0) .{ .topic = g.location.topic - 1, .sheet = @intCast((wrap(topics[g.location.topic - 1].paragraphs, g.effective_columns, null).lines + g.effective_rows - 1) / g.effective_rows - 1) } else null;
    const next: ?Location = if (g.location.sheet + 1 < g.sheets) .{ .topic = g.location.topic, .sheet = g.location.sheet + 1 } else if (g.location.topic < 6) .{ .topic = g.location.topic + 1, .sheet = 0 } else null;
    // Exact selected-page bytes and every disjoint H1 text copy are preflighted
    // with scalars. No duplicate joined-body buffer or second line table.
    const selected_bytes = scan(topics[g.location.topic].paragraphs, g.effective_columns, null, start, start + count).selected_bytes;
    const location_bytes = topics[g.location.topic].title.len + 4 + digits(g.location.sheet + 1) + digits(g.sheets);
    const dynamic_bytes = selected_bytes * 2 + location_bytes;
    const labels = (if (previous != null) @as(usize, 8) else 0) + (if (next != null) @as(usize, 4) else 0) + 9 + 5;
    const text_bytes = dynamic_bytes + 12 + 17 + topics[g.location.topic].title.len + 20 + 41 + labels * 4;
    if (selected_bytes > 3565 or dynamic_bytes > 7173 or text_bytes > 7440) return error.LimitExceeded;
    var reservation = try budget.reserve(.frame_scratch, HEADER_BYTES);
    const h = allocator.create(Header) catch {
        reservation.cancel();
        return error.OutOfMemory;
    };
    h.* = .{ .m = .{ .allocator = allocator, .budget = budget, .charge = reservation.commit(), .resident = input.current.owner.resident, .draft = input.current.owner.draft, .hold = input.current.owner.hold, .revision = input.current.revision, .context_len = input.current.owner.context.len, .geometry = g, .previous = previous, .next = next, .selected_start = start, .selected_count = count } };
    var prepared = PreparedGuide{ .header = h };
    errdefer prepared.deinit();
    @memcpy(h.context[0..h.m.context_len], input.current.owner.context);
    const counts = wrap(topics[g.location.topic].paragraphs, g.effective_columns, h);
    h.m.word_count = counts.words;
    h.m.line_count = counts.lines;
    try build(h, input.source.focused);
    std.debug.assert(h.m.arena_used == dynamic_bytes);
    _ = try h.m.resident.retain();
    h.m.retained_resident = true;
    if (h.m.draft) |draft| {
        _ = try draft.retain();
        h.m.retained_draft = true;
    }
    return prepared;
}
fn copy(h: *Header, text: []const u8) PrepareError![]const u8 {
    if (text.len > h.arena.len - h.m.arena_used) return error.LimitExceeded;
    const start = h.m.arena_used;
    h.m.arena_used += text.len;
    @memcpy(h.arena[start..h.m.arena_used], text);
    return h.arena[start..h.m.arena_used];
}
fn node(h: *Header, id: []const u8, role: H1.NodeRole, name: []const u8, parent: ?usize, bounds: ?H1.Rect) usize {
    const i = h.m.node_count;
    h.m.node_count += 1;
    h.nodes[i] = .{ .id = .{ .bytes = id }, .role = role, .name = name, .description = "", .value = null, .enabled = true, .selected = false, .focusable = false, .hierarchy_level = null, .expanded = null, .action = null, .bounds = bounds, .visible = true, .parent = parent, .child_start = 0, .child_count = 0 };
    return i;
}
fn button(h: *Header, comptime key: []const u8, label: []const u8, index: usize, focused: ?H1.ActionId) PrepareError!void {
    const action = H1.ActionId{ .bytes = "guide." ++ key };
    const rect = try footer(h.m.geometry.viewport, index);
    const n = node(h, "guide." ++ key ++ ".node", .button, label, 1, rect);
    h.nodes[n].description = label;
    h.nodes[n].value = "";
    h.nodes[n].focusable = true;
    h.nodes[n].action = action;
    _ = node(h, "guide." ++ key ++ ".value", .text, label, n, rect);
    _ = node(h, "guide." ++ key ++ ".description", .text, label, n, rect);
    h.focus[h.m.focus_count] = action;
    h.m.focus_count += 1;
    if (focused) |f| if (std.mem.eql(u8, f.bytes, action.bytes)) {
        h.m.focused = action;
    };
}
fn digits(value: u16) usize {
    return if (value >= 100) 3 else if (value >= 10) 2 else 1;
}
fn decimal(h: *Header, value: u16) PrepareError!void {
    var scratch: [3]u8 = undefined;
    var offset: usize = 3;
    var n = value;
    while (true) {
        offset -= 1;
        scratch[offset] = @intCast('0' + n % 10);
        n /= 10;
        if (n == 0) break;
    }
    _ = try copy(h, scratch[offset..]);
}
fn build(h: *Header, focused: ?H1.ActionId) PrepareError!void {
    const title = topics[h.m.geometry.location.topic].title;
    const location_start = h.m.arena_used;
    _ = try copy(h, title);
    _ = try copy(h, " (");
    try decimal(h, h.m.geometry.location.sheet + 1);
    _ = try copy(h, "/");
    try decimal(h, h.m.geometry.sheets);
    _ = try copy(h, ")");
    const location = h.arena[location_start..h.m.arena_used];
    h.m.preserved_start = h.m.arena_used;
    const selected = h.lines[h.m.selected_start..][0..h.m.selected_count];
    for (selected, 0..) |span, li| {
        if (li > 0) _ = try copy(h, " ");
        for (h.words[span.first_word..][0..span.word_count], 0..) |word, wi| {
            if (wi > 0) _ = try copy(h, " ");
            _ = try copy(h, topics[h.m.geometry.location.topic].paragraphs[word.paragraph][word.start..][0..word.len]);
        }
    }
    h.m.preserved_len = h.m.arena_used - h.m.preserved_start;
    h.m.upper_start = h.m.arena_used;
    const upper = try copy(h, h.arena[h.m.preserved_start..][0..h.m.preserved_len]);
    for (h.arena[h.m.upper_start..h.m.arena_used]) |*byte| byte.* = std.ascii.toUpper(byte.*);
    h.m.upper_len = upper.len;
    var cursor = h.m.upper_start;
    for (selected, 0..) |*span, li| {
        if (li > 0) cursor += 1;
        const begin = cursor;
        for (h.words[span.first_word..][0..span.word_count], 0..) |word, wi| cursor += word.len + @as(usize, if (wi > 0) 1 else 0);
        span.upper_start = @intCast(begin);
        span.upper_len = @intCast(cursor - begin);
    }
    _ = node(h, "guide.application", .application, "NYON player guide", null, null);
    _ = node(h, "guide.dialog", .dialog, title, 0, null);
    _ = node(h, "guide.title", .text, "NYON // Player guide", 1, null);
    _ = node(h, "guide.location", .text, location, 1, null);
    _ = node(h, "guide.held", .text, "Time held // Tab + Enter // Escape closes", 1, null);
    const body = node(h, "guide.body", .text, "Instructions", 1, null);
    h.nodes[body].description = h.arena[h.m.preserved_start..][0..h.m.preserved_len];
    const value = node(h, "guide.body.value", .text, "", body, null);
    h.nodes[value].value = upper;
    if (h.m.previous != null) try button(h, "previous", "Previous", 0, focused);
    if (h.m.next != null) try button(h, "next", "Next", 1, focused);
    try button(h, "menu", "Main menu", 2, focused);
    try button(h, "close", "Close", 3, focused);
    for (h.nodes[0..h.m.node_count], 0..) |*n, i| {
        n.child_start = h.m.child_count;
        for (h.nodes[0..h.m.node_count], 0..) |child, j| if (child.parent == i) {
            h.children[h.m.child_count] = j;
            h.m.child_count += 1;
            n.child_count += 1;
        };
    }
}
pub fn publishGuide(slot: *H1.FrameSlot, publication: *GuidePublication, prepared: *const PreparedGuide, current: CurrentStamp) PublishError!void {
    const h = prepared.header orelse return error.InvalidPublication;
    if (slot.budget != h.m.budget) return error.BudgetMismatch;
    const input = prepared.input() catch return error.InvalidPublication;
    if (h.m.revision != current.revision or !input.owner.matches(current.owner)) return error.CurrentMismatch;
    const candidate = GuidePublication{ .revision = current.revision, .previous = h.m.previous, .next = h.m.next };
    try slot.replace(input, limits);
    publication.* = candidate;
}
fn validLocation(location: Location) bool {
    return location.topic <= 6 and location.sheet < 530;
}
/// Requires current full-shell guide ownership and no higher modal dispatch owner.
pub fn validateGuideAction(view: H1.FrameView, publication: *const GuidePublication, current: CurrentStamp, id: H1.ActionId) ActionError!Action {
    if (!std.unicode.utf8ValidateSlice(id.bytes)) return error.InvalidUtf8;
    const revision = publication.revision orelse return error.NoPublication;
    if (revision != current.revision or !view.owner.matches(current.owner)) return error.CurrentMismatch;
    const tag: u8 = if (std.mem.eql(u8, id.bytes, "guide.previous")) 0 else if (std.mem.eql(u8, id.bytes, "guide.next")) 1 else if (std.mem.eql(u8, id.bytes, "guide.menu")) 2 else if (std.mem.eql(u8, id.bytes, "guide.close")) 3 else return error.UnknownAction;
    var active = false;
    for (view.nodes) |n| if (n.action) |action| if (std.mem.eql(u8, action.bytes, id.bytes) and n.enabled and n.focusable) {
        active = true;
    };
    if (!active) return error.InactiveAction;
    if (tag >= 2) return if (tag == 2) .main_menu else .close;
    const location = (if (tag == 0) publication.previous else publication.next) orelse return error.InvalidPublication;
    if (!validLocation(location)) return error.InvalidPublication;
    return .{ .show = location };
}

test "H3d guide literal no split whitespace paragraph blank span wrapping" {
    var h: Header = undefined;
    const counts = wrap(&.{ "one two", "three" }, 3, &h);
    try std.testing.expectEqual(@as(usize, 3), counts.words);
    try std.testing.expectEqual(@as(usize, 5), counts.lines);
    try std.testing.expectEqual(@as(u16, 1), h.lines[0].word_count);
    try std.testing.expectEqual(@as(u16, 1), h.lines[1].word_count);
    try std.testing.expectEqual(@as(u16, 0), h.lines[2].word_count);
    try std.testing.expectEqual(@as(u16, 1), h.lines[3].word_count);
    try std.testing.expectEqual(@as(u16, 0), h.lines[4].word_count);
    try std.testing.expectEqual(@as(u16, 4), h.words[1].start);
    const long = wrap(&.{"abcdef"}, 3, &h);
    try std.testing.expectEqual(@as(usize, 2), long.lines);
    try std.testing.expectEqual(@as(u16, 6), h.words[0].len);
}
