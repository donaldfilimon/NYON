const std = @import("std");
const ids = @import("ids.zig");
const types = @import("types.zig");
const rules = @import("rules.zig");
const surface = @import("../surface.zig");
const state = @import("state.zig");
const view = @import("view.zig");
const command = @import("command.zig");
const phases = @import("phases.zig");
const history = @import("history.zig");
const invariants = @import("invariants.zig");

pub const CreateProgress = union(enum) { pending, ready, canceled, failed: types.Failure, invalid_request: types.Failure, idle };
const CreatePhase = enum { init_pool, init_directory, allocate_pages, terrain, roundtrip, neighbors, reciprocity, receipt, encode, meta_hash, meta_write, ready, discard, drained };
pub const CreateCursor = struct {
    phase: CreatePhase = .init_pool,
    index: u32 = 0,
    substep: u8 = 0,
    direction: u8 = 0,
    reverse: u8 = 0,
    forward_cell: ?surface.Cell = null,
    return_count: u8 = 0,
    edge_checks: u64 = 0,
    neighbors: [4]u64 = .{ 0, 0, 0, 0 },
    allocated_pages: u16 = 0,
    pending_page_ref: u64 = 0,
    encoder: ?history.Encoder = null,
    chain: history.Chain = history.Chain.init(),
    discard_index: u16 = 0,
    discard_released: bool = false,
};

pub const Builder = struct {
    storage: ?state.Storage,
    cursor: CreateCursor = .{},
    canceled: bool = false,
    failure: ?types.Failure = null,

    pub fn deinit(self: *Builder) void {
        std.debug.assert(self.storage == null or self.cursor.phase == .drained);
        if (self.storage) |*storage| storage.deinit();
        self.storage = null;
    }

    pub fn stats(self: *const Builder) types.Stats {
        if (self.storage) |storage| return statsFor(&storage, .prepare);
        return .{ .requested_bytes = 0, .peak_pages = 0, .live_pages = 0, .pages_copied = 0, .work_units = 0, .history_used = 0, .phase = null };
    }
};

const TickJob = struct {
    id: ids.JobId,
    candidate: state.Candidate,
    phase: types.Phase = .prepare,
    step: enum { candidate, descriptors, input, commands, phases, complete, metadata, finalize, invariants, publish, cleanup } = .candidate,
    policy: types.SessionPolicy,
    target: ids.Tick,
    expected: ids.Revision,
    count: u16,
    input_lengths: [64]u16 = undefined,
    input_index: u16 = 0,
    envelopes: [64]types.Envelope = undefined,
    commands: command.Cursor = undefined,
    economic: phases.Cursor = undefined,
    finalizer: history.Finalizer = undefined,
    metadata: ?state.MetaWriter = null,
    invariants: invariants.Cursor = .{},
    canceled: bool = false,
    failure: ?types.Failure = null,
    terminal_delivered: bool = false,
};

comptime {
    std.debug.assert(@sizeOf(TickJob) + @alignOf(TickJob) - 1 <= 65_536);
}

pub const Authority = struct {
    storage: state.Storage,
    lane: ?*TickJob = null,
    next_job: u64 = 1,
    last_job: ?ids.JobId = null,
    last_published: bool = false,
    accepted: u16 = 0,
    rejected: u16 = 0,

    pub fn deinit(self: *Authority) void {
        std.debug.assert(self.lane == null);
        self.storage.deinit();
    }
    pub fn summary(self: *const Authority) types.TickSummary {
        const root = self.storage.rootMeta(self.storage.active);
        return .{ .tick = root.tick, .revision = root.revision, .accepted = self.accepted, .rejected = self.rejected, .chain_head = root.chain_head };
    }
    pub fn stats(self: *const Authority) types.Stats {
        return statsFor(&self.storage, if (self.lane) |lane| lane.phase else null);
    }
    pub fn settlement(self: *const Authority, id: ids.SettlementId) types.Failure!types.SettlementView {
        return view.settlement(&self.storage, id);
    }
    pub fn cell(self: *const Authority, id: ids.CellId) types.Failure!types.CellView {
        return view.cell(&self.storage, id);
    }
    pub fn receipts(self: *const Authority, revision: ids.Revision, cursor: types.ReceiptCursor, out: []types.ReceiptView) types.Failure!types.ReceiptPage {
        return view.receipts(&self.storage, revision, cursor, out);
    }

    // The sole synchronous bounded-copy allowance: at most 64 KiB. No world
    // traversal, decoding or sorting occurs here; borrowed bytes expire now.
    pub fn beginTick(self: *Authority, expected: ids.Revision, policy: types.SessionPolicy, batch: []const types.Envelope) types.Failure!ids.JobId {
        try self.checkRequest(expected, batch.len);
        for (batch) |envelope| if (envelope.bytes.len > 1024) return error.EnvelopeTooLarge;
        var lengths: [64]u16 = undefined;
        for (batch, 0..) |envelope, index| {
            @memmove(self.storage.reservation.envelopes[index][0..envelope.bytes.len], envelope.bytes);
            lengths[index] = @intCast(envelope.bytes.len);
        }
        return self.beginOwned(expected, policy, lengths[0..batch.len]);
    }

    fn checkRequest(self: *const Authority, expected: ids.Revision, count: usize) types.Failure!void {
        if (self.lane != null) return error.Busy;
        const meta = self.storage.rootMeta(self.storage.active);
        if (expected != meta.revision) return error.WrongRevision;
        if (count > 64) return error.TooManyCommands;
        if (@backingInt(meta.tick) >= 1_000_000) return error.TickHorizon;
        if (self.next_job == std.math.maxInt(u64)) return error.IdExhausted;
    }

    // Internal integration entry point, deliberately absent from the facade.
    // Replay/story cursors must finish metered writes to owned slots first.
    // Admission and publication use exactly the same lane as beginTick.
    pub fn beginOwned(self: *Authority, expected: ids.Revision, policy: types.SessionPolicy, lengths: []const u16) types.Failure!ids.JobId {
        try self.checkRequest(expected, lengths.len);
        for (lengths) |len| if (len > 1024) return error.EnvelopeTooLarge;
        const meta = self.storage.rootMeta(self.storage.active);
        const id: ids.JobId = @fromBackingInt(self.next_job);
        self.next_job += 1;
        self.last_job = id;
        self.last_published = false;
        // SHA state may require stricter alignment than Reservation itself.
        // Round inside the already reserved control block, including slack.
        const control = @intFromPtr(&self.storage.reservation.control);
        self.lane = @ptrFromInt(std.mem.alignForward(usize, control, @alignOf(TickJob)));
        self.lane.?.* = .{ .id = id, .candidate = state.Candidate.begin(&self.storage), .policy = policy, .expected = expected, .target = @fromBackingInt(@backingInt(meta.tick) + 1), .count = @intCast(lengths.len) };
        @memcpy(self.lane.?.input_lengths[0..lengths.len], lengths);
        return id;
    }

    pub fn cancel(self: *Authority, id: ids.JobId) types.CancelResult {
        if (self.last_job == null or self.last_job.? != id) return .stale_job;
        if (self.last_published) return .already_published;
        if (self.lane) |lane| {
            lane.canceled = true;
            return .requested;
        }
        return .stale_job;
    }

    pub fn poll(self: *Authority, id: ids.JobId, budget: *types.WorkBudget) types.Progress {
        if (self.last_job == null or self.last_job.? != id) return .{ .invalid_request = error.StaleJob };
        if (budget.remaining > 4096) return .{ .invalid_request = error.InvalidBudget };
        if (self.lane == null) return .idle;
        const before = budget.remaining;
        const lane = self.lane.?;
        if (budget.remaining == 0) return .{ .pending = .{ .phase = lane.phase, .units_used = 0 } };
        if ((lane.canceled or lane.failure != null) and !lane.candidate.published) {
            lane.phase = .discard;
            lane.step = .cleanup;
            if (!lane.terminal_delivered) {
                state.spend(&self.storage, budget);
                lane.terminal_delivered = true;
                return if (lane.failure) |failure| .{ .failed = failure } else .canceled;
            }
        }
        return self.advanceTick(budget) catch |failure| {
            lane.failure = failure;
            lane.phase = .discard;
            lane.step = .cleanup;
            return .{ .pending = .{ .phase = .discard, .units_used = before - budget.remaining } };
        };
    }

    fn advanceTick(self: *Authority, budget: *types.WorkBudget) types.Failure!types.Progress {
        const before = budget.remaining;
        const lane = self.lane.?;
        const candidate = &lane.candidate;
        while (budget.remaining > 0) {
            switch (lane.step) {
                .candidate => {
                    if (!candidate.prepared) {
                        if (try candidate.advancePrepare(budget) == .pending) break;
                    }
                    if (budget.remaining == 0) break;
                    state.spend(&self.storage, budget);
                    _ = types.SessionPolicy.init(lane.policy.role, lane.policy.civilization) catch return error.InvalidPolicy;
                    lane.commands = command.Cursor.init(lane.policy, lane.envelopes[0..lane.count], candidate.meta().last_sequence, lane.target);
                    lane.commands.phase = .parse;
                    lane.step = .descriptors;
                },
                .descriptors => {
                    if (lane.input_index == lane.count) {
                        lane.step = .input;
                        continue;
                    }
                    state.spend(&self.storage, budget);
                    const index = lane.input_index;
                    const len = lane.input_lengths[index];
                    lane.envelopes[index] = .{ .bytes = self.storage.reservation.envelopes[index][0..len] };
                    lane.commands.lengths[index] = len;
                    lane.commands.order[index] = @intCast(index);
                    lane.input_index += 1;
                },
                .input => {
                    if (try candidate.stage(.{ .header = .{ .tick = lane.target, .ordinal = 0, .provenance = .autonomous, .sequence = null }, .event = .{ .tick_input = .{ .target_tick = lane.target, .expected_revision = lane.expected, .previous_policy = candidate.meta().policy, .current_policy = lane.policy, .envelope_count = lane.count } } }, budget) == .pending) break;
                    lane.step = .commands;
                    lane.phase = .commands;
                },
                .commands => {
                    if (try lane.commands.advance(candidate, budget) == .pending) break;
                    lane.phase = .arrivals;
                    lane.economic = phases.Cursor.init(.arrivals, lane.target);
                    lane.step = .phases;
                },
                .phases => {
                    if (try lane.economic.advance(candidate, budget) == .pending) break;
                    lane.phase = switch (lane.phase) {
                        .arrivals => .construction,
                        .construction => .production,
                        .production => .population,
                        .population => .finalize,
                        else => unreachable,
                    };
                    if (lane.phase == .finalize) lane.step = .complete else lane.economic = phases.Cursor.init(lane.phase, lane.target);
                },
                .complete => {
                    const meta = candidate.meta();
                    if (try candidate.stage(.{ .header = .{ .tick = lane.target, .ordinal = self.storage.staged_count, .provenance = .autonomous, .sequence = null }, .event = .{ .tick_complete = .{ .revision = @fromBackingInt(@backingInt(lane.expected) + 1), .last_sequence = lane.commands.highest_sequence, .accepted = lane.commands.accepted, .rejected = lane.commands.rejected, .settlements = meta.settlements, .buildings = meta.buildings, .active_shipments = meta.shipments } } }, budget) == .pending) break;
                    var updated = meta;
                    updated.tick = lane.target;
                    updated.revision = @fromBackingInt(@backingInt(lane.expected) + 1);
                    updated.policy = lane.policy;
                    updated.last_sequence = lane.commands.highest_sequence;
                    lane.metadata = state.MetaWriter.init(updated);
                    lane.step = .metadata;
                },
                .metadata => {
                    if (try lane.metadata.?.advance(candidate, budget) == .pending) break;
                    lane.finalizer = history.Finalizer.init(candidate);
                    lane.step = .finalize;
                },
                .finalize => {
                    if (try lane.finalizer.advance(budget) == .pending) break;
                    lane.step = .invariants;
                },
                .invariants => {
                    if (try lane.invariants.advance(candidate, budget) == .pending) break;
                    lane.step = .publish;
                    lane.phase = .publish;
                },
                .publish => {
                    state.spend(&self.storage, budget);
                    candidate.publish();
                    self.accepted = lane.commands.accepted;
                    self.rejected = lane.commands.rejected;
                    self.last_published = true;
                    lane.terminal_delivered = true;
                    lane.step = .cleanup;
                    lane.phase = .cleanup;
                    return .{ .published = self.summary() };
                },
                .cleanup => {
                    if (try candidate.advanceDiscard(budget) == .pending) break;
                    self.lane = null;
                    return .idle;
                },
            }
        }
        return .{ .pending = .{ .phase = lane.phase, .units_used = before - budget.remaining } };
    }
};

fn statsFor(storage: *const state.Storage, phase: ?types.Phase) types.Stats {
    const has_meta = if (storage.genesis != null) blk: {
        const meta_ref = storage.reservation.directories[storage.active][state.region_start[6]];
        if (meta_ref == 0 or meta_ref > state.pool_size) break :blk false;
        const meta_index: usize = @intCast(meta_ref - 1);
        break :blk storage.reservation.pool[meta_index].references != 0;
    } else false;
    return .{
        .requested_bytes = state.requested_bytes,
        .peak_pages = storage.peak_pages,
        .live_pages = storage.live_pages,
        .pages_copied = storage.pages_copied,
        .work_units = storage.work_units,
        .history_used = if (has_meta) storage.rootMeta(storage.active).history_used else 0,
        .phase = phase,
    };
}

pub fn beginCreate(allocator: types.Allocator, config: types.Config) types.Failure!Builder {
    try rules.validateConfig(config);
    const cells = std.math.mul(u32, config.side, config.side) catch return error.CapacityUnsupported;
    _ = std.math.mul(u32, cells, 6) catch return error.CapacityUnsupported;
    return .{ .storage = try state.Storage.init(allocator, config) };
}

pub fn cancelCreate(builder: *Builder) void {
    if (builder.cursor.phase != .ready and builder.cursor.phase != .drained) builder.canceled = true;
}

pub fn takeAuthority(builder: *Builder) types.Failure!Authority {
    if (builder.cursor.phase != .ready) return error.NotReady;
    const storage = builder.storage orelse return error.NotReady;
    builder.storage = null;
    builder.cursor.phase = .drained;
    return .{ .storage = storage };
}

pub fn pollCreate(builder: *Builder, budget: *types.WorkBudget) CreateProgress {
    if (builder.storage == null) return .idle;
    if (budget.remaining > 4096) return .{ .invalid_request = error.InvalidBudget };
    if (builder.cursor.phase == .ready) return .ready;
    if (builder.cursor.phase == .drained) return .idle;
    if (budget.remaining == 0) return .pending;
    if (builder.canceled and builder.cursor.phase != .discard) {
        builder.cursor.phase = .discard;
        builder.cursor.discard_index = 0;
    }
    const result = advanceCreate(builder, budget) catch |failure| {
        builder.failure = failure;
        builder.cursor.phase = .discard;
        return .pending;
    };
    return result;
}

fn advanceCreate(builder: *Builder, budget: *types.WorkBudget) types.Failure!CreateProgress {
    return advanceCreateContext(builder, budget, .{});
}

// Shared creation mechanics only: callers own lifetime and identity.
pub fn advanceCreateContext(builder: *Builder, budget: *types.WorkBudget, context: rules.AlgorithmContext) types.Failure!CreateProgress {
    const storage = &builder.storage.?;
    const cursor = &builder.cursor;
    const cell_count: u32 = @as(u32, 6) * storage.config.side * storage.config.side;
    const topology = surface.Topology{ .side = storage.config.side };
    while (budget.remaining > 0) {
        switch (cursor.phase) {
            .init_pool => {
                if (cursor.index == state.pool_size) {
                    cursor.index = 0;
                    cursor.phase = .init_directory;
                    continue;
                }
                state.spend(storage, budget);
                const slot = &storage.reservation.pool[cursor.index];
                slot.references = 0;
                slot.next_free = if (cursor.index + 1 == state.pool_size) 0 else @intCast(cursor.index + 2);
                cursor.index += 1;
            },
            .init_directory => {
                if (cursor.index == 2 * state.directory_size) {
                    cursor.index = 0;
                    cursor.phase = .allocate_pages;
                    continue;
                }
                state.spend(storage, budget);
                storage.reservation.directories[cursor.index / state.directory_size][cursor.index % state.directory_size] = 0;
                cursor.index += 1;
            },
            .allocate_pages => {
                if (cursor.index == state.addressed_pages) {
                    cursor.index = 0;
                    cursor.phase = .terrain;
                    continue;
                }
                state.spend(storage, budget);
                switch (cursor.substep) {
                    0 => {
                        cursor.pending_page_ref = try storage.allocatePage();
                        cursor.substep = 1;
                    },
                    1 => {
                        @memset(&storage.reservation.pool[@as(usize, @intCast(cursor.pending_page_ref - 1))].bytes, 0);
                        cursor.substep = 2;
                    },
                    2 => {
                        storage.reservation.directories[storage.active][cursor.index] = cursor.pending_page_ref;
                        cursor.pending_page_ref = 0;
                        cursor.allocated_pages += 1;
                        cursor.index += 1;
                        cursor.substep = 0;
                    },
                    else => unreachable,
                }
            },
            .terrain => {
                if (cursor.index == cell_count) {
                    cursor.index = 0;
                    cursor.phase = .roundtrip;
                    continue;
                }
                state.spend(storage, budget);
                const cell_id: ids.CellId = .{ .planet = context.planet, .local = cursor.index };
                try storage.setTerrainAt(storage.active, @intCast(cursor.index), rules.terrain(storage.config.seed, cell_id));
                cursor.index += 1;
            },
            .roundtrip => {
                if (cursor.index == cell_count) {
                    cursor.index = 0;
                    cursor.phase = .neighbors;
                    continue;
                }
                state.spend(storage, budget);
                const cell = topology.fromId(cursor.index) catch return error.InvariantViolation;
                if ((topology.cellId(cell) catch return error.InvariantViolation) != cursor.index) return error.InvariantViolation;
                cursor.index += 1;
            },
            .neighbors => {
                if (cursor.index == cell_count) {
                    cursor.index = 0;
                    cursor.direction = 0;
                    cursor.phase = .reciprocity;
                    continue;
                }
                state.spend(storage, budget);
                const cell = topology.fromId(cursor.index) catch return error.InvariantViolation;
                cursor.edge_checks += 1;
                const adjacent = topology.neighbor(cell, directions[cursor.direction]) catch return error.InvariantViolation;
                const neighbor_id = topology.cellId(adjacent) catch return error.InvariantViolation;
                if (neighbor_id >= cell_count or neighbor_id == cursor.index) return error.InvariantViolation;
                for (cursor.neighbors[0..cursor.direction]) |prior| if (prior == neighbor_id) return error.InvariantViolation;
                cursor.neighbors[cursor.direction] = neighbor_id;
                cursor.direction += 1;
                if (cursor.direction == 4) {
                    cursor.direction = 0;
                    cursor.index += 1;
                }
            },
            .reciprocity => {
                if (cursor.index == cell_count) {
                    cursor.phase = .receipt;
                    continue;
                }
                if (cursor.forward_cell == null) {
                    state.spend(storage, budget);
                    const cell = topology.fromId(cursor.index) catch return error.InvariantViolation;
                    cursor.edge_checks += 1;
                    cursor.forward_cell = topology.neighbor(cell, directions[cursor.direction]) catch return error.InvariantViolation;
                    continue;
                }
                state.spend(storage, budget);
                cursor.edge_checks += 1;
                const returned = topology.neighbor(cursor.forward_cell.?, directions[cursor.reverse]) catch return error.InvariantViolation;
                if ((topology.cellId(returned) catch return error.InvariantViolation) == cursor.index) cursor.return_count += 1;
                cursor.reverse += 1;
                if (cursor.reverse == 4) {
                    if (cursor.return_count != 1) return error.InvariantViolation;
                    cursor.reverse = 0;
                    cursor.return_count = 0;
                    cursor.forward_cell = null;
                    cursor.direction += 1;
                    if (cursor.direction == 4) {
                        cursor.direction = 0;
                        cursor.index += 1;
                    }
                }
            },
            .receipt => {
                state.spend(storage, budget);
                storage.genesis = .{
                    .header = .{ .tick = @fromBackingInt(0), .ordinal = 0, .provenance = .autonomous, .sequence = null },
                    .event = .{ .genesis = storage.config },
                };
                cursor.encoder = history.Encoder.initContext(&storage.genesis.?, &storage.reservation.history[0], context);
                cursor.chain = history.Chain.initContext(context);
                cursor.phase = .encode;
            },
            .encode => {
                const before = budget.remaining;
                defer storage.work_units += before - budget.remaining;
                if (try cursor.encoder.?.advance(budget) == .pending) return .pending;
                storage.reservation.history_meta[0] = @intCast(cursor.encoder.?.len());
                try cursor.chain.begin(storage.reservation.history[0][0..cursor.encoder.?.len()]);
                cursor.phase = .meta_hash;
            },
            .meta_hash => {
                const before = budget.remaining;
                defer storage.work_units += before - budget.remaining;
                if (try cursor.chain.advance(budget) == .pending) return .pending;
                cursor.phase = .meta_write;
            },
            .meta_write => {
                state.spend(storage, budget);
                var metadata: state.RootMeta = .{};
                metadata.chain_head = cursor.chain.head();
                metadata.history_used = 1;
                storage.writeRootMeta(storage.active, metadata);
                cursor.phase = .ready;
                return .ready;
            },
            .discard => {
                if (cursor.pending_page_ref != 0) {
                    state.spend(storage, budget);
                    try storage.release(cursor.pending_page_ref);
                    cursor.pending_page_ref = 0;
                    continue;
                }
                if (cursor.discard_index == cursor.allocated_pages) {
                    cursor.phase = .drained;
                    return if (builder.failure) |failure| .{ .failed = failure } else .canceled;
                }
                const ref = storage.reservation.directories[storage.active][cursor.discard_index];
                if (!cursor.discard_released) {
                    state.spend(storage, budget);
                    try storage.release(ref);
                    cursor.discard_released = true;
                } else {
                    state.spend(storage, budget);
                    storage.reservation.directories[storage.active][cursor.discard_index] = 0;
                    cursor.discard_index += 1;
                    cursor.discard_released = false;
                }
            },
            .ready, .drained => unreachable,
        }
    }
    return .pending;
}

const directions = [_]surface.Direction{ .east, .west, .north, .south };
