//! Coordinator-private staged adapter. Intentionally absent from root.zig exports.
const std = @import("std");
const t = @import("types.zig");
const it = @import("../planet_economy/types.zig");
const state = @import("../planet_economy/state.zig");
const rules = @import("../planet_economy/rules.zig");
const creation = @import("../planet_economy/job.zig");
const command = @import("../planet_economy/command.zig");
const phases = @import("../planet_economy/phases.zig");
const history = @import("../planet_economy/history.zig");
const invariants = @import("../planet_economy/invariants.zig");
const view = @import("../planet_economy/view.zig");

pub const RootHandle = t.RootHandle;
pub const AlgorithmContext = rules.AlgorithmContext;

const Lane = struct {
    candidate: state.Candidate,
    old_root: RootHandle,
    new_root: RootHandle,
    target: it.Tick,
    expected: it.Revision,
    policy: t.SessionPolicy,
    watermark: it.CommandSequence,
    phase: t.Phase = .prepare,
    step: enum { candidate, descriptors, input, structural, economy, complete, metadata, history, invariants, frozen } = .candidate,
    lengths: [64]u16 = undefined,
    envelopes: [64]it.Envelope = undefined,
    count: u16,
    input_index: u16 = 0,
    commands: command.Cursor = undefined,
    economic: phases.Cursor = undefined,
    metadata: ?state.MetaWriter = null,
    finalizer: history.Finalizer = undefined,
    checks: invariants.Cursor = .{},
    installed: bool = false,
    restored: bool = false,
    retirement: ?state.Candidate = null,
    retiring: ?RootHandle = null,
};

const Control = struct {
    builder: creation.Builder,
    context: AlgorithmContext,
    planet: t.PlanetId,
    ready: bool = false,
    create_init: usize = 0,
    create_drained: bool = false,
    generations: [2]u64 = .{ 1, 1 },
    readable: [2]bool = .{ false, false },
    lane: ?Lane = null,
};
comptime {
    std.debug.assert(@sizeOf(Control) + @alignOf(Control) - 1 <= 65_536);
    std.debug.assert(@sizeOf(state.Reservation) == 8_642_560);
    std.debug.assert(@sizeOf(state.RootMeta) <= 256);
    std.debug.assert(@sizeOf([1024]it.Receipt) <= 2_097_152);
}

pub const PlanetBuilder = struct {
    control: ?*Control,

    // Allocation-ledger destruction is also valid for incomplete setup; it
    // never traverses potentially incomplete or damaged page references.
    pub fn deinit(self: *PlanetBuilder) void {
        const control = self.control orelse return;
        const storage = control.builder.storage.?;
        self.control = null;
        storage.allocator.destroy(storage.reservation);
    }
};

pub fn createPlanet(allocator: t.Allocator, config: t.PlanetConfig, planet: t.PlanetId, seed: u64, civilization_count: u8) t.Failure!PlanetBuilder {
    if (@backingInt(planet) < 1 or @backingInt(planet) > 4 or @backingInt(config.system) < 1 or @backingInt(config.system) > 4 or civilization_count < 1 or civilization_count > 4) return error.InvalidConfig;
    const local: it.Config = .{ .seed = t.planetSeed(seed, planet), .side = config.side, .capacities = .{
        .settlements = config.capacities.settlements,
        .buildings = config.capacities.buildings,
        .shipments = config.capacities.freight,
        .history_receipts = config.capacities.history_receipts,
    } };
    rules.validateConfig(local) catch return error.InvalidConfig;
    const builder = try allocatePlanet(allocator, config, planet, seed, civilization_count);
    // Preserve the existing private constructor's initialized-cursor contract.
    builder.control.?.builder.cursor = .{};
    builder.control.?.create_init = @sizeOf(creation.CreateCursor);
    return builder;
}

// Allocation/ownership only. The coordinator validates before advancing creation.
pub fn allocatePlanet(allocator: t.Allocator, config: t.PlanetConfig, planet: t.PlanetId, seed: u64, civilization_count: u8) t.Failure!PlanetBuilder {
    const local: it.Config = .{ .seed = t.planetSeed(seed, planet), .side = config.side, .capacities = .{
        .settlements = config.capacities.settlements,
        .buildings = config.capacities.buildings,
        .shipments = config.capacities.freight,
        .history_receipts = config.capacities.history_receipts,
    } };
    var storage = try state.Storage.init(allocator, local);
    const context: AlgorithmContext = .{ .planet = @fromBackingInt(@backingInt(planet)), .system = @backingInt(config.system), .civilization_count = civilization_count };
    storage.context = context;
    const bytes = &storage.reservation.control;
    const aligned = std.mem.alignForward(usize, @intFromPtr(bytes), @alignOf(Control));
    const control: *Control = @ptrFromInt(aligned);
    control.builder.storage = storage;
    control.builder.canceled = false;
    control.builder.failure = null;
    control.context = context;
    control.planet = planet;
    control.ready = false;
    control.create_init = 0;
    control.create_drained = false;
    control.generations = .{ 1, 1 };
    control.readable = .{ false, false };
    control.lane = null;
    return .{ .control = control };
}

pub fn advancePlanetCreate(builder: *PlanetBuilder, budget: *t.WorkBudget) t.Failure!t.Step {
    if (budget.remaining > 4096) return error.InvalidBudget;
    const control = builder.control orelse return error.NotReady;
    if (control.ready) return .done;
    if (budget.remaining == 0) return .pending;
    // Copy fixed cursor defaults incrementally; allocation never initializes worlds.
    const initial: creation.CreateCursor = .{};
    const source = std.mem.asBytes(&initial);
    const destination = std.mem.asBytes(&control.builder.cursor);
    while (control.create_init < source.len) {
        if (budget.remaining == 0) return .pending;
        state.spend(&control.builder.storage.?, budget);
        const count = @min(@as(usize, 256), source.len - control.create_init);
        @memcpy(destination[control.create_init..][0..count], source[control.create_init..][0..count]);
        control.create_init += count;
    }
    const result = try creation.advanceCreateContext(&control.builder, budget, control.context);
    switch (result) {
        .ready => {
            control.ready = true;
            control.readable[0] = true;
            return .done;
        },
        .pending => return .pending,
        else => return error.InvariantViolation,
    }
}

// Drains even a ready genesis; no published owner exists during builder abort.
fn drainControl(control: *Control, budget: *t.WorkBudget) t.Failure!t.Step {
    if (budget.remaining == 0) return .pending;
    const b = &control.builder;
    if (control.create_drained) return .done;
    if (control.create_init < @sizeOf(creation.CreateCursor)) {
        state.spend(&b.storage.?, budget);
        control.create_drained = true;
        return .done;
    }
    if (b.cursor.phase == .drained) return .done;
    if (b.cursor.phase != .discard) {
        state.spend(&b.storage.?, budget);
        b.cursor.phase = .discard;
        control.ready = false;
        control.readable = .{ false, false };
    }
    // Invalid unvalidated sides must never enter the terrain arithmetic.
    if (b.cursor.allocated_pages == 0 and b.cursor.pending_page_ref == 0) {
        if (budget.remaining == 0) return .pending;
        state.spend(&b.storage.?, budget);
        b.cursor.phase = .drained;
        return .done;
    }
    const result = try creation.advanceCreateContext(b, budget, control.context);
    return if (result == .canceled or result == .failed) .done else .pending;
}
pub fn drainPlanetCreate(builder: *PlanetBuilder, budget: *t.WorkBudget) t.Failure!t.Step {
    return drainControl(builder.control orelse return error.NotReady, budget);
}

pub fn takePlanet(builder: *PlanetBuilder) t.Failure!PlanetAuthority {
    const control = builder.control orelse return error.NotReady;
    if (!control.ready) return error.NotReady;
    builder.control = null;
    return .{ .control = control };
}

pub const PlanetAuthority = struct {
    control: *Control,

    pub fn drainGenesis(self: *PlanetAuthority, budget: *t.WorkBudget) t.Failure!t.Step {
        return drainControl(self.control, budget);
    }
    pub fn storage(self: *const PlanetAuthority) *state.Storage {
        return &self.control.builder.storage.?;
    }
    pub fn deinit(self: *PlanetAuthority) void {
        const owned = self.storage().*;
        owned.allocator.destroy(owned.reservation);
        self.* = undefined;
    }
    pub fn activeRoot(self: *const PlanetAuthority) RootHandle {
        const slot = self.storage().active;
        return .{ .slot = slot, .generation = self.control.generations[slot] };
    }
    pub fn candidateRoot(self: *const PlanetAuthority) t.Failure!RootHandle {
        return (self.control.lane orelse return error.NotReady).new_root;
    }
    pub fn envelopeBuffer(self: *PlanetAuthority, index: u16) t.Failure!*[1024]u8 {
        if (index >= 64) return error.TooManyCommands;
        if (self.control.lane != null) return error.Busy;
        return &self.storage().reservation.envelopes[index];
    }
    fn rootSlot(self: *const PlanetAuthority, root: RootHandle) t.Failure!u1 {
        if (root.slot > 1 or !self.control.readable[root.slot] or self.control.generations[root.slot] != root.generation) return error.StaleView;
        return @intCast(root.slot);
    }
    pub fn metadata(self: *const PlanetAuthority, root: RootHandle) t.Failure!state.RootMeta {
        return self.storage().rootMeta(try self.rootSlot(root));
    }
    pub fn prepared(self: *const PlanetAuthority, root: RootHandle) t.Failure!t.PreparedRoot {
        const meta = try self.metadata(root);
        return .{ .planet = self.control.planet, .root = root, .tick = @fromBackingInt(@backingInt(meta.tick)), .revision = @fromBackingInt(@backingInt(meta.revision)), .history_count = meta.history_used, .history_head = meta.chain_head };
    }
    // Jobs/replay use these one-record/edge readers with their shared allowance;
    // public copied-value queries below have bounded page limits instead.
    pub fn readRecord(self: *const PlanetAuthority, root: RootHandle, kind: state.RecordKind, index: u16, budget: *t.WorkBudget) t.Failure!?state.Record {
        if (budget.remaining > 4096) return error.InvalidBudget;
        const slot = try self.rootSlot(root);
        if (budget.remaining == 0) return null;
        state.spend(self.storage(), budget);
        return try self.storage().readRecord(slot, kind, index);
    }
    pub fn readRouteCell(self: *const PlanetAuthority, root: RootHandle, shipment_slot: u16, index: u16, budget: *t.WorkBudget) t.Failure!?t.CellId {
        if (budget.remaining > 4096) return error.InvalidBudget;
        const slot = try self.rootSlot(root);
        if (budget.remaining == 0) return null;
        state.spend(self.storage(), budget);
        const value = try self.storage().routeCell(slot, shipment_slot, index);
        return .{ .planet = self.control.planet, .local = value.local };
    }
    pub fn readTerrain(self: *const PlanetAuthority, root: RootHandle, local: u16, budget: *t.WorkBudget) t.Failure!?t.Terrain {
        if (budget.remaining > 4096) return error.InvalidBudget;
        const slot = try self.rootSlot(root);
        if (budget.remaining == 0) return null;
        state.spend(self.storage(), budget);
        return try self.storage().terrainAt(slot, local);
    }
    pub fn counts(self: *const PlanetAuthority) t.Failure!struct { accepted: u16, rejected: u16 } {
        const lane = if (self.control.lane) |*l| l else return error.NotReady;
        if (lane.step != .frozen) return error.NotReady;
        return .{ .accepted = lane.commands.accepted, .rejected = lane.commands.rejected };
    }
    pub fn settlement(self: *const PlanetAuthority, root: RootHandle, id: t.SettlementId) t.Failure!t.SettlementValue {
        const value = try view.settlementAt(self.storage(), try self.rootSlot(root), @fromBackingInt(@backingInt(id)));
        return .{ .revision = @fromBackingInt(@backingInt(value.revision)), .id = .{ .planet = self.control.planet, .local = id }, .owner = value.owner, .hall = .{ .planet = self.control.planet, .local = value.hall.local }, .population = value.population, .food = value.food, .material = value.material, .incoming_food = value.incoming_food, .incoming_material = value.incoming_material, .housing = value.housing, .hunger = value.hunger, .fed_streak = value.fed_streak };
    }
    pub fn cell(self: *const PlanetAuthority, root: RootHandle, id: t.CellId) t.Failure!t.CellValue {
        if (id.planet != self.control.planet) return error.InvalidPlanet;
        const value = try view.cellAt(self.storage(), try self.rootSlot(root), .{ .planet = @fromBackingInt(@backingInt(id.planet)), .local = id.local });
        return .{ .revision = @fromBackingInt(@backingInt(value.revision)), .id = id, .terrain = value.terrain, .settlement = if (value.settlement) |v| .{ .planet = id.planet, .local = @fromBackingInt(@backingInt(v)) } else null, .building = if (value.building) |v| .{ .planet = id.planet, .local = @fromBackingInt(@backingInt(v)) } else null, .is_hall = value.is_hall };
    }
    pub fn receipts(self: *const PlanetAuthority, root: RootHandle, cursor: u16, out: []t.LocalReceiptValue) t.Failure!t.ReceiptPage {
        const meta = try self.metadata(root);
        if (out.len > 64) return error.CapacityUnsupported;
        if (cursor > meta.history_used) return error.NotFound;
        const count = @min(out.len, meta.history_used - cursor);
        for (out[0..count], 0..) |*row, index| {
            const slot = cursor + index;
            const len = self.storage().reservation.history_meta[slot];
            if (len > 1024) return error.InvariantViolation;
            row.len = @intCast(len);
            @memcpy(row.bytes[0..len], self.storage().reservation.history[slot][0..len]);
        }
        const next: u16 = cursor + @as(u16, @intCast(count));
        return .{ .written = @intCast(count), .next = if (next == meta.history_used) null else next };
    }
    pub fn beginPrepared(self: *PlanetAuthority, expected: t.Revision, target: t.Tick, policy: t.SessionPolicy, owned_lengths: []const u16, global_watermark: t.CommandSequence) t.Failure!void {
        const c = self.control;
        if (c.lane != null) return error.Busy;
        const s = self.storage();
        const meta = s.rootMeta(s.active);
        if (@backingInt(expected) != @backingInt(meta.revision)) return error.WrongRevision;
        const next_tick = std.math.add(u64, @backingInt(meta.tick), 1) catch return error.TickHorizon;
        if (@backingInt(target) > 1_000_000) return error.TickHorizon;
        if (@backingInt(target) != next_tick) return error.WrongTick;
        _ = std.math.add(u64, @backingInt(expected), 1) catch return error.IdExhausted;
        if (@backingInt(global_watermark) < @backingInt(meta.last_sequence)) return error.SequenceAlreadyPublished;
        if (owned_lengths.len > 64) return error.TooManyCommands;
        for (owned_lengths) |len| if (len > 1024) return error.EnvelopeTooLarge;
        const spare = s.spareRoot();
        if (c.readable[spare]) return error.Busy;
        const generation = std.math.add(u64, c.generations[spare], 1) catch return error.IdExhausted;
        c.generations[spare] = generation;
        c.lane = .{ .candidate = state.Candidate.begin(s), .old_root = self.activeRoot(), .new_root = .{ .slot = spare, .generation = generation }, .expected = @fromBackingInt(@backingInt(expected)), .target = @fromBackingInt(@backingInt(target)), .policy = policy, .watermark = @fromBackingInt(@backingInt(global_watermark)), .count = @intCast(owned_lengths.len) };
        @memcpy(c.lane.?.lengths[0..owned_lengths.len], owned_lengths);
    }
    pub fn advancePhase(self: *PlanetAuthority, phase: t.Phase, budget: *t.WorkBudget) t.Failure!t.Step {
        if (budget.remaining > 4096) return error.InvalidBudget;
        const lane = if (self.control.lane) |*l| l else return error.NotReady;
        if (lane.phase != phase or lane.restored or lane.retiring != null) return error.NotReady;
        if (budget.remaining == 0) return .pending;
        const s = self.storage();
        const context = self.control.context;
        const candidate = &lane.candidate;
        while (budget.remaining > 0) switch (phase) {
            .prepare => switch (lane.step) {
                .candidate => {
                    if (try candidate.advancePrepare(budget) == .pending) return .pending;
                    if (budget.remaining == 0) return .pending;
                    state.spend(s, budget);
                    _ = t.SessionPolicy.init(lane.policy.role, lane.policy.civilization) catch return error.InvalidPolicy;
                    if (lane.policy.civilization) |owner| if (!context.validOwner(owner)) return error.InvalidPolicy;
                    lane.commands = command.Cursor.init(lane.policy, lane.envelopes[0..lane.count], candidate.meta().last_sequence, lane.target);
                    lane.commands.phase = .parse;
                    lane.step = .descriptors;
                },
                .descriptors => {
                    if (lane.input_index == lane.count) {
                        lane.step = .input;
                        continue;
                    }
                    state.spend(s, budget);
                    const index = lane.input_index;
                    lane.envelopes[index] = .{ .bytes = s.reservation.envelopes[index][0..lane.lengths[index]] };
                    lane.commands.lengths[index] = lane.lengths[index];
                    lane.commands.order[index] = @intCast(index);
                    lane.input_index += 1;
                },
                .input => {
                    if (try candidate.stage(.{ .header = .{ .tick = lane.target, .ordinal = 0, .provenance = .autonomous, .sequence = null }, .event = .{ .tick_input = .{ .target_tick = lane.target, .expected_revision = lane.expected, .previous_policy = candidate.meta().policy, .current_policy = lane.policy, .envelope_count = lane.count } } }, budget) == .pending) return .pending;
                    lane.step = .structural;
                },
                .structural => {
                    if (try lane.commands.advanceContext(candidate, budget, context, true) == .pending) return .pending;
                    if (@backingInt(lane.commands.highest_sequence) > @backingInt(lane.watermark)) return error.InvalidEnvelope;
                    lane.phase = .commands;
                    return .done;
                },
                else => return error.NotReady,
            },
            .commands => {
                if (try lane.commands.advanceContext(candidate, budget, context, false) == .pending) return .pending;
                lane.phase = .arrivals;
                lane.economic = phases.Cursor.init(.arrivals, lane.target);
                lane.step = .economy;
                return .done;
            },
            .arrivals, .construction, .production, .population => {
                if (try lane.economic.advance(candidate, budget) == .pending) return .pending;
                lane.phase = switch (phase) {
                    .arrivals => .construction,
                    .construction => .production,
                    .production => .population,
                    .population => .local_finalize,
                    else => unreachable,
                };
                if (lane.phase == .local_finalize) lane.step = .complete else lane.economic = phases.Cursor.init(switch (lane.phase) {
                    .construction => .construction,
                    .production => .production,
                    .population => .population,
                    else => unreachable,
                }, lane.target);
                return .done;
            },
            .local_finalize => switch (lane.step) {
                .complete => {
                    var meta = candidate.meta();
                    if (try candidate.stage(.{ .header = .{ .tick = lane.target, .ordinal = s.staged_count, .provenance = .autonomous, .sequence = null }, .event = .{ .tick_complete = .{ .revision = @fromBackingInt(@backingInt(lane.expected) + 1), .last_sequence = lane.watermark, .accepted = lane.commands.accepted, .rejected = lane.commands.rejected, .settlements = meta.settlements, .buildings = meta.buildings, .active_shipments = meta.shipments } } }, budget) == .pending) return .pending;
                    meta.tick = lane.target;
                    meta.revision = @fromBackingInt(@backingInt(lane.expected) + 1);
                    meta.last_sequence = lane.watermark;
                    meta.policy = lane.policy;
                    lane.metadata = state.MetaWriter.init(meta);
                    lane.step = .metadata;
                },
                .metadata => {
                    if (try lane.metadata.?.advance(candidate, budget) == .pending) return .pending;
                    lane.finalizer = history.Finalizer.initContext(candidate, context);
                    lane.step = .history;
                },
                .history => {
                    if (try lane.finalizer.advance(budget) == .pending) return .pending;
                    lane.step = .invariants;
                },
                .invariants => {
                    if (try lane.checks.advanceContext(candidate, budget, context) == .pending) return .pending;
                    lane.step = .frozen;
                    lane.phase = .install;
                    self.control.readable[lane.new_root.slot] = true;
                    return .done;
                },
                else => return error.NotReady,
            },
            else => return error.NotReady,
        };
        return .pending;
    }
    pub fn freezePrepared(self: *PlanetAuthority, budget: *t.WorkBudget) t.Failure!?t.PreparedRoot {
        if (budget.remaining > 4096) return error.InvalidBudget;
        const lane = if (self.control.lane) |*l| l else return error.NotReady;
        if (lane.restored or lane.retiring != null) return error.NotReady;
        if (budget.remaining == 0) return null;
        if (lane.phase == .local_finalize) {
            if (try self.advancePhase(.local_finalize, budget) == .pending) return null;
        }
        if (lane.step != .frozen) return error.NotReady;
        if (budget.remaining == 0) return null;
        state.spend(self.storage(), budget);
        return try self.prepared(lane.new_root);
    }
    pub fn installHidden(self: *PlanetAuthority, budget: *t.WorkBudget) t.Failure!t.Step {
        if (budget.remaining > 4096) return error.InvalidBudget;
        const lane = if (self.control.lane) |*l| l else return error.NotReady;
        if (lane.step != .frozen or lane.restored or lane.retiring != null) return error.NotReady;
        if (lane.installed) return .done;
        if (budget.remaining == 0) return .pending;
        state.spend(self.storage(), budget);
        self.storage().active = @intCast(lane.new_root.slot);
        lane.installed = true;
        return .done;
    }
    pub fn restoreHidden(self: *PlanetAuthority, budget: *t.WorkBudget) t.Failure!t.Step {
        if (budget.remaining > 4096) return error.InvalidBudget;
        const lane = if (self.control.lane) |*l| l else return error.NotReady;
        if (lane.retiring != null) return error.NotReady;
        if (lane.restored) return .done;
        if (budget.remaining == 0) return .pending;
        state.spend(self.storage(), budget);
        self.storage().active = @intCast(lane.old_root.slot);
        lane.installed = false;
        lane.restored = true;
        return .done;
    }
    pub fn retire(self: *PlanetAuthority, root: RootHandle, budget: *t.WorkBudget) t.Failure!t.Step {
        if (budget.remaining > 4096) return error.InvalidBudget;
        const lane = if (self.control.lane) |*l| l else return error.NotReady;
        const aborting = std.meta.eql(root, lane.new_root);
        if (aborting) {
            if (!lane.restored) return error.NotReady;
        } else if (!std.meta.eql(root, lane.old_root) or !lane.installed) return error.NotReady;
        if (lane.retiring) |target| if (!std.meta.eql(root, target)) return error.NotReady;
        if (budget.remaining == 0) return .pending;
        if (lane.retiring == null) {
            state.spend(self.storage(), budget);
            lane.retiring = root;
            self.control.readable[root.slot] = false;
            if (!aborting) lane.retirement = .{ .storage = self.storage(), .root = @intCast(root.slot), .prepare_index = state.directory_size };
        }
        const cursor = if (aborting) &lane.candidate else &lane.retirement.?;
        if (try cursor.advanceDiscard(budget) == .pending) return .pending;
        self.control.lane = null;
        return .done;
    }
};
