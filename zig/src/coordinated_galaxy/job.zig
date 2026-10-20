//! Single-owner, globally atomic coordinator lane. All cursors live in reservations.
const std = @import("std");
const t = @import("types.zig");
const state = @import("state.zig");
const planet = @import("planet.zig");
const view = @import("view.zig");
const command = @import("command.zig");
const history = @import("history.zig");

pub const Builder = struct { handle: ?*anyopaque };
// Private lifecycle bridge; the public facade exposes only an opaque owner.
pub fn builderStorage(builder: *const Builder) ?*state.Storage {
    return @ptrCast(@alignCast(builder.handle orelse return null));
}
const Create = struct {
    step: enum { metadata, header, planets, systems, setup, collect, stage, finalize, descriptor, publish, ready, drain, drained } = .metadata,
    index: u8 = 0,
    systems: [4]bool = @splat(false),
    canceled: bool = false,
    failure: ?t.Failure = null,
    delivered: bool = false,
    fault: bool = false,
    receipt: t.Receipt = undefined,
    finalizer: history.Finalizer = undefined,
};
const Tick = struct {
    id: t.JobId,
    target: t.Tick,
    expected: t.Revision,
    policy: t.SessionPolicy,
    admission: command.Cursor,
    phase: t.Phase = .prepare,
    phase_entered: bool = false,
    step: enum { routing, capacity, input, routes, begin, phases, freeze, prepared, commit, finalize, descriptor, install, publish, retire, restore, discard, clear } = .routing,
    index: u16 = 0,
    started: u8 = 0,
    installed: u8 = 0,
    accepted: u16 = 0,
    rejected: u16 = 0,
    counts: [4]struct { accepted: u16, rejected: u16 } = undefined,
    receipt: t.Receipt = undefined,
    receipt_live: bool = false,
    receipt_index: u8 = 0,
    finalizer: history.Finalizer = undefined,
    canceled: bool = false,
    failure: ?t.Failure = null,
    delivered: bool = false,
    published: bool = false,
};
// Scratch is reserved up front; this is not an allocator-owned cache or job.
const Control = struct {
    create: Create = .{},
    tick: Tick = undefined,
    next_job: u64 = 1,
    last_job: ?t.JobId = null,
    last_published: bool = false,
    fault_cause: t.Failure = error.CleanupFault,
    work: u64 = 0,
};
comptime {
    std.debug.assert(@sizeOf(Control) + @alignOf(Control) - 1 <= 2_097_152);
    std.debug.assert(@sizeOf(t.Config) <= 256);
    std.debug.assert(@sizeOf(t.PlanetEntry) <= 256);
    std.debug.assert(@sizeOf(t.CommitEntry) <= 256);
}
pub fn control(s: *const state.Storage) *Control {
    const address = std.mem.alignForward(usize, @intFromPtr(&s.reservation.?.scratch), @alignOf(Control));
    return @ptrFromInt(address);
}
fn spend(b: *t.WorkBudget) void {
    std.debug.assert(b.remaining > 0);
    b.remaining -= 1;
}
fn pending(s: *state.Storage, phase: ?t.Phase, index: u32, used: u16) t.Stats {
    const m = s.metadata();
    var result: t.Stats = .{ .phase = phase, .cursor = index, .units_used = used, .work_units = control(s).work, .requested_bytes = s.requestedBytes() };
    if (phase != null and index < s.config.planet_count) result.planet = @fromBackingInt(index + 1);
    if (!s.metadata_ready) return result;
    result.live_leases = m.descriptor_leases[0] + m.descriptor_leases[1];
    if (m.lane_live) result.installed_planets = control(s).tick.installed;
    for (m.planets[0..s.owned_planets]) |p| if (p) |authority| {
        result.peak_pages += authority.storage().peak_pages;
        result.copied_pages += authority.storage().pages_copied;
    };
    if (m.committed_descriptor) |slot| {
        const d = &m.descriptors[slot];
        result.global_history_used = d.history_count;
        result.accepted = d.accepted;
        result.rejected = d.rejected;
        for (d.planets[0..d.planet_count], 0..) |p, i| result.local_history_used[i] = p.history_count;
    }
    if (m.obsolete_descriptor) |slot| result.obsolete_leases = m.descriptor_leases[slot];
    return result;
}
fn summary(s: *state.Storage) t.Summary {
    const m = s.metadata();
    const d = &m.descriptors[m.committed_descriptor.?];
    return .{ .tick = d.tick, .revision = d.revision, .last_sequence = d.last_sequence, .accepted = d.accepted, .rejected = d.rejected, .chain_head = d.history_head };
}

pub fn beginCreate(allocator: t.Allocator, config: t.Config) t.Failure!Builder {
    var storage = try state.Storage.allocate(allocator, config);
    const s = view.anchorStorage(&storage);
    const c = control(s);
    // Initialize only scalar ownership/control state, not the large unused lanes.
    c.create.step = .metadata;
    c.create.index = 0;
    c.create.systems = @splat(false);
    c.create.canceled = false;
    c.create.failure = null;
    c.create.delivered = false;
    c.create.fault = false;
    c.next_job = 1;
    c.last_job = null;
    c.last_published = false;
    c.fault_cause = error.CleanupFault;
    c.work = 0;
    return .{ .handle = s };
}
pub fn cancelCreate(builder: *Builder) void {
    const s = builderStorage(builder) orelse return;
    const c = &control(s).create;
    if (c.step != .drained) c.canceled = true;
}
pub fn takeReady(builder: *Builder) t.Failure!view.Galaxy {
    const s = builderStorage(builder) orelse return error.NotReady;
    const c = &control(s).create;
    if (c.step != .ready or c.canceled or c.failure != null) return error.NotReady;
    const galaxy = view.takeGalaxy(s) catch |err| {
        c.failure = err;
        return err;
    };
    builder.handle = null;
    return galaxy;
}
pub fn deinitBuilder(builder: *Builder) t.Failure!void {
    const s = builderStorage(builder) orelse return;
    if (control(s).create.step != .drained or control(s).create.fault) return error.Busy;
    builder.handle = null;
    s.deinit();
}
// Private explicit destruction exception for replay-owned creation faults.
// No Galaxy handoff or public lease can exist while Builder retains its handle.
pub fn destroyFaultedBuilder(builder: *Builder) t.Failure!void {
    const s = builderStorage(builder) orelse return error.NotReady;
    if (!control(s).create.fault or s.metadata().owner_generation != 0) return error.NotReady;
    builder.handle = null;
    s.deinit();
}
pub fn pollCreate(builder: *Builder, budget: *t.WorkBudget) t.CreateProgress {
    const s = builderStorage(builder) orelse return .{ .failed = error.NotReady };
    const c = &control(s).create;
    if (budget.remaining > 4096) return .{ .failed = error.InvalidBudget };
    const before = budget.remaining;
    if (budget.remaining == 0) return .{ .pending = pending(s, if (c.step == .drained) null else .prepare, c.index, 0) };
    if (c.fault) return .{ .failed = error.CleanupFault };
    if (c.canceled or c.failure != null) {
        if (!c.delivered) {
            spend(budget);
            control(s).work += 1;
            c.delivered = true;
            c.step = .drain;
            c.index = 0;
            return if (c.failure) |err| .{ .failed = err } else .canceled;
        }
    }
    advanceCreate(s, budget) catch |err| {
        if (c.step == .drain) {
            c.fault = true;
            c.failure = error.CleanupFault;
        } else c.failure = err;
    };
    control(s).work += before - budget.remaining;
    if (c.step == .ready and c.failure == null) return .ready;
    return .{ .pending = pending(s, if (c.step == .drained) null else .prepare, c.index, before - budget.remaining) };
}
fn advanceCreate(s: *state.Storage, budget: *t.WorkBudget) t.Failure!void {
    const m = s.metadata();
    const c = &control(s).create;
    const cfg = &s.config;
    while (budget.remaining > 0) switch (c.step) {
        .metadata => {
            if (try s.advanceMetadata(budget) == .pending) return;
            c.step = .header;
        },
        .header => {
            spend(budget);
            if (cfg.system_count < 1 or cfg.system_count > 4 or cfg.civilization_count < 1 or cfg.civilization_count > 4 or cfg.planet_count < 1 or cfg.system_count > cfg.planet_count or cfg.global_history_capacity < 1 or cfg.global_history_capacity > 4096) return error.InvalidConfig;
            c.step = .planets;
        },
        .planets => {
            if (c.index == cfg.planet_count) {
                c.index = 0;
                c.step = .systems;
                continue;
            }
            spend(budget);
            const p = cfg.planets[c.index];
            const system = @backingInt(p.system);
            const cap = p.capacities;
            if (system < 1 or system > cfg.system_count or p.side < 4 or p.side > 32 or cap.settlements < 1 or cap.settlements > 64 or cap.buildings < 1 or cap.buildings > 256 or cap.freight < 1 or cap.freight > 64 or cap.history_receipts < 1 or cap.history_receipts > 4096) return error.InvalidConfig;
            c.systems[system - 1] = true;
            c.index += 1;
        },
        .systems => {
            if (c.index == cfg.system_count) {
                c.index = 0;
                c.step = .setup;
                continue;
            }
            spend(budget);
            if (!c.systems[c.index]) return error.InvalidConfig;
            c.index += 1;
        },
        .setup => {
            if (try s.advanceSetup(budget) == .pending) return;
            if (budget.remaining == 0) return;
            spend(budget);
            try view.reserveDescriptor(s, .{});
            // Empty seed descriptor is stable until the finalizer captures it.
            const d = &m.descriptors[1];
            d.tick = @fromBackingInt(0);
            d.history_count = 0;
            d.history_head = history.initial_head;
            c.finalizer = history.Finalizer.init(s, d);
            c.receipt = .{ .header = .{ .tick = @fromBackingInt(0), .ordinal = 0 }, .event = .{ .genesis = .{ .config = cfg.*, .planets = undefined } } };
            c.step = .collect;
        },
        .collect => {
            if (c.index == cfg.planet_count) {
                c.step = .stage;
                continue;
            }
            spend(budget);
            const p = &m.planets[c.index].?;
            const prepared = try p.prepared(p.activeRoot());
            m.prepared_roots[c.index] = prepared;
            const pc = cfg.planets[c.index];
            c.receipt.event.genesis.planets[c.index] = .{ .planet = prepared.planet, .system = pc.system, .derived_seed = t.planetSeed(cfg.seed, prepared.planet), .side = pc.side, .capacities = pc.capacities, .local_genesis_head = prepared.history_head };
            c.index += 1;
        },
        .stage => {
            if (try c.finalizer.stage(&c.receipt, budget) == .pending) return;
            c.step = .finalize;
        },
        .finalize => {
            if (try c.finalizer.advance(budget) == .pending) return;
            if (budget.remaining == 0) return;
            spend(budget);
            m.descriptor_header.history_count = c.finalizer.historyCount();
            m.descriptor_header.history_head = c.finalizer.head();
            c.step = .descriptor;
        },
        .descriptor => {
            if (try view.prepareDescriptor(s, budget) == .pending) return;
            c.step = .publish;
        },
        .publish => {
            spend(budget);
            view.publishDescriptor(s);
            c.step = .ready;
            return;
        },
        .ready, .drained => return,
        .drain => {
            if (c.index == s.owned_planets) {
                spend(budget);
                // No public owner/lease exists. Staging and descriptors become unreachable.
                m.committed_descriptor = null;
                m.preparing_descriptor = null;
                m.descriptor_ready = false;
                c.finalizer = undefined;
                c.step = .drained;
                return;
            }
            if (m.planets[c.index]) |*p| {
                if (try p.drainGenesis(budget) == .pending) return;
            } else if (m.builders[c.index]) |*b| {
                if (try planet.drainPlanetCreate(b, budget) == .pending) return;
            }
            c.index += 1;
        },
    };
}

pub fn beginTick(g: *view.Galaxy, expected: t.Revision, target: t.Tick, policy: t.SessionPolicy, batch: []const t.RoutedEnvelope) t.Failure!t.JobId {
    if (batch.len > 256) return error.TooManyCommands;
    return admitTick(g, expected, target, policy, batch, @intCast(batch.len));
}
// Private replay seam: caller has paid every copy and initialized count slots,
// including lengths <=1024. Admission and all structural parsing stay shared.
pub fn beginOwnedTick(g: *view.Galaxy, expected: t.Revision, target: t.Tick, policy: t.SessionPolicy, count: u16) t.Failure!t.JobId {
    return admitTick(g, expected, target, policy, null, count);
}
fn admitTick(g: *view.Galaxy, expected: t.Revision, target: t.Tick, policy: t.SessionPolicy, batch: ?[]const t.RoutedEnvelope, count: u16) t.Failure!t.JobId {
    const s = try view.storage(g);
    const m = s.metadata();
    const c = control(s);
    if (m.acquisition_closed) return error.Shutdown;
    if (m.faulted) return error.CleanupFault;
    if (m.lane_live or m.obsolete_descriptor != null or m.preparing_descriptor != null) return error.Busy;
    const d = &m.descriptors[m.committed_descriptor orelse return error.NotReady];
    if (d.revision != expected) return error.WrongRevision;
    const next = std.math.add(u64, @backingInt(d.tick), 1) catch return error.TickHorizon;
    if (next > 1_000_000 or @backingInt(target) > 1_000_000) return error.TickHorizon;
    if (@backingInt(target) != next) return error.WrongTick;
    const revision = std.math.add(u64, @backingInt(expected), 1) catch return error.IdExhausted;
    if (c.next_job == std.math.maxInt(u64)) return error.IdExhausted;
    if (count > 256) return error.TooManyCommands;
    if (batch) |envelopes| for (envelopes) |envelope| {
        if (envelope.body.len > 1024) return error.EnvelopeTooLarge;
    };
    try view.reserveDescriptor(s, .{ .tick = target, .revision = @fromBackingInt(revision), .policy = policy });
    const admission: command.Cursor = if (batch) |envelopes| try command.Cursor.init(s, policy, envelopes) else .{ .storage = s, .policy = policy, .count = count, .last_sequence = m.published_sequence, .watermark = m.published_sequence };
    const id: t.JobId = @fromBackingInt(c.next_job);
    c.next_job += 1;
    c.last_job = id;
    c.last_published = false;
    c.tick = .{ .id = id, .target = target, .expected = expected, .policy = policy, .admission = admission, .finalizer = history.Finalizer.init(s, d) };
    m.lane_live = true;
    return id;
}
pub fn cancel(g: *view.Galaxy, id: t.JobId) t.CancelResult {
    const s = view.storage(g) catch return .stale_job;
    const c = control(s);
    if (c.last_job == null or c.last_job.? != id) return .stale_job;
    if (s.metadata().faulted) return .faulted;
    if (c.last_published) return .already_published;
    if (!s.metadata().lane_live) return .stale_job;
    c.tick.canceled = true;
    return .requested;
}
pub const deinit = view.deinit;
pub fn poll(g: *view.Galaxy, id: t.JobId, budget: *t.WorkBudget) t.Progress {
    const s = view.storage(g) catch |err| return .{ .invalid_request = err };
    const c = control(s);
    const m = s.metadata();
    if (c.last_job == null or c.last_job.? != id) return .{ .invalid_request = error.StaleJob };
    if (m.faulted) return .{ .faulted = .{ .job = id, .cause = c.fault_cause } };
    if (budget.remaining > 4096) return .{ .invalid_request = error.InvalidBudget };
    if (!m.lane_live) return .idle;
    const lane = &c.tick;
    if (budget.remaining == 0) return .{ .pending = pending(s, lane.phase, lane.index, 0) };
    if (lane.step == .retire and !view.canRetire(s, m.obsolete_descriptor.?)) return .waiting_for_views;
    const before = budget.remaining;
    if (!lane.published and (lane.canceled or lane.failure != null) and !lane.delivered) {
        spend(budget);
        c.work += 1;
        lane.delivered = true;
        lane.step = .restore;
        lane.phase = .retire;
        lane.index = 0;
        return if (lane.failure) |err| .{ .failed = err } else .canceled;
    }
    const result = advanceTick(s, budget) catch |err| blk: {
        if (lane.published or lane.delivered) {
            m.faulted = true;
            c.fault_cause = error.CleanupFault;
            break :blk t.Progress{ .faulted = .{ .job = id, .cause = error.CleanupFault } };
        }
        lane.failure = err;
        break :blk t.Progress{ .pending = undefined };
    };
    c.work += before - budget.remaining;
    if (result == .pending) return .{ .pending = pending(s, lane.phase, lane.index, before - budget.remaining) };
    return result;
}
fn stage(lane: *Tick, budget: *t.WorkBudget) t.Failure!bool {
    if (try lane.finalizer.stage(&lane.receipt, budget) == .pending) return false;
    lane.receipt_live = false;
    return true;
}
fn advanceTick(s: *state.Storage, budget: *t.WorkBudget) t.Failure!t.Progress {
    const m = s.metadata();
    const lane = &control(s).tick;
    while (budget.remaining > 0) switch (lane.step) {
        .routing => {
            if (try lane.admission.advance(budget) == .pending) break;
            lane.step = .capacity;
        },
        .capacity => {
            spend(budget);
            const committed = &m.descriptors[m.committed_descriptor.?];
            const required = try history.requiredTickReceipts(s.config.planet_count, lane.admission.count);
            if (!history.hasCapacity(committed.history_count, s.config.global_history_capacity, required)) return error.HistoryFull;
            lane.step = .input;
        },
        .input => {
            if (!lane.receipt_live) {
                spend(budget);
                const d = &m.descriptors[m.committed_descriptor.?];
                lane.receipt = .{ .header = .{ .tick = lane.target, .ordinal = 0 }, .event = .{ .tick_input = .{ .target = lane.target, .expected_revision = lane.expected, .previous_policy = d.policy, .current_policy = lane.policy, .count = lane.admission.count } } };
                lane.receipt_live = true;
            }
            if (!try stage(lane, budget)) break;
            lane.step = .routes;
        },
        .routes => {
            if (lane.index == lane.admission.count) {
                lane.index = 0;
                lane.step = .begin;
                continue;
            }
            if (!lane.receipt_live) {
                spend(budget);
                lane.receipt = .{ .header = .{ .tick = lane.target, .ordinal = lane.finalizer.staged_count }, .event = .{ .routed_input = try lane.admission.routed(lane.index) } };
                lane.receipt_live = true;
            }
            if (!try stage(lane, budget)) break;
            lane.index += 1;
        },
        .begin => {
            if (lane.index == s.config.planet_count) {
                lane.index = 0;
                lane.step = .phases;
                continue;
            }
            spend(budget);
            const id: t.PlanetId = @fromBackingInt(@as(u32, lane.index) + 1);
            try m.planets[lane.index].?.beginPrepared(lane.expected, lane.target, lane.policy, lane.admission.ownedLengths(id), lane.admission.globalWatermark());
            lane.started += 1;
            lane.index += 1;
        },
        .phases => {
            if (lane.index == s.config.planet_count) {
                lane.index = 0;
                if (lane.phase == .local_finalize) lane.step = .freeze else lane.phase = @fromBackingInt(@intCast(@backingInt(lane.phase) + 1));
                continue;
            }
            if (!lane.phase_entered) {
                spend(budget);
                lane.phase_entered = true;
                if (budget.remaining == 0) break;
            }
            if (try m.planets[lane.index].?.advancePhase(lane.phase, budget) == .pending) break;
            lane.phase_entered = false;
            lane.index += 1;
        },
        .freeze => {
            if (lane.index == s.config.planet_count) {
                lane.index = 0;
                lane.step = .prepared;
                lane.phase = .global_finalize;
                continue;
            }
            const p = &m.planets[lane.index].?;
            if (m.prepared_roots[lane.index] == null) {
                m.prepared_roots[lane.index] = try p.freezePrepared(budget);
                if (m.prepared_roots[lane.index] == null) break;
            }
            if (budget.remaining == 0) break;
            spend(budget);
            const counts = try p.counts();
            lane.counts[lane.index] = .{ .accepted = counts.accepted, .rejected = counts.rejected };
            lane.accepted += counts.accepted;
            lane.rejected += counts.rejected;
            lane.index += 1;
        },
        .prepared => {
            if (lane.index == s.config.planet_count) {
                lane.index = 0;
                lane.step = .commit;
                continue;
            }
            if (!lane.receipt_live) {
                spend(budget);
                const root = m.prepared_roots[lane.index].?;
                const counts = lane.counts[lane.index];
                lane.receipt = .{ .header = .{ .tick = lane.target, .ordinal = lane.finalizer.staged_count }, .event = .{ .planet_prepared = .{ .planet = root.planet, .local_history_count = root.history_count, .local_head = root.history_head, .accepted = counts.accepted, .rejected = counts.rejected } } };
                lane.receipt_live = true;
            }
            if (!try stage(lane, budget)) break;
            lane.index += 1;
        },
        .commit => {
            if (!lane.receipt_live) {
                spend(budget);
                lane.receipt = .{ .header = .{ .tick = lane.target, .ordinal = lane.finalizer.staged_count }, .event = .{ .tick_commit = .{ .revision = m.descriptor_header.revision, .last_sequence = lane.admission.globalWatermark(), .accepted = lane.accepted, .rejected = lane.rejected, .planet_count = s.config.planet_count, .planets = undefined } } };
                lane.receipt_live = true;
            }
            if (lane.receipt_index < s.config.planet_count) {
                if (budget.remaining == 0) break;
                spend(budget);
                const root = m.prepared_roots[lane.receipt_index].?;
                if (root.tick != lane.target or root.revision != m.descriptor_header.revision) return error.InvariantViolation;
                lane.receipt.event.tick_commit.planets[lane.receipt_index] = .{ .planet = root.planet, .tick = root.tick, .revision = root.revision, .local_history_count = root.history_count, .local_head = root.history_head };
                lane.receipt_index += 1;
                continue;
            }
            if (!try stage(lane, budget)) break;
            lane.step = .finalize;
        },
        .finalize => {
            if (try lane.finalizer.advance(budget) == .pending) break;
            if (budget.remaining == 0) break;
            spend(budget);
            m.descriptor_header.history_count = lane.finalizer.historyCount();
            m.descriptor_header.history_head = lane.finalizer.head();
            m.descriptor_header.last_sequence = lane.admission.globalWatermark();
            m.descriptor_header.accepted = lane.accepted;
            m.descriptor_header.rejected = lane.rejected;
            lane.step = .descriptor;
        },
        .descriptor => {
            if (try view.prepareDescriptor(s, budget) == .pending) break;
            lane.step = .install;
            lane.phase = .install;
            lane.index = 0;
        },
        .install => {
            if (lane.index == s.config.planet_count) {
                lane.step = .publish;
                continue;
            }
            if (try m.planets[lane.index].?.installHidden(budget) == .pending) break;
            lane.installed += 1;
            lane.index += 1;
        },
        .publish => {
            spend(budget);
            // Every O(P) preparation/check/install already completed above.
            std.debug.assert(lane.installed == s.config.planet_count and m.descriptor_ready);
            view.publishDescriptor(s);
            m.published_sequence = m.descriptor_header.last_sequence;
            lane.published = true;
            control(s).last_published = true;
            lane.step = .retire;
            lane.phase = .retire;
            lane.index = 0;
            return .{ .published = summary(s) };
        },
        .retire => {
            if (lane.index == s.config.planet_count) {
                spend(budget);
                try view.finishRetirement(s, m.obsolete_descriptor.?);
                lane.step = .clear;
                continue;
            }
            const old = &m.descriptors[m.obsolete_descriptor.?];
            if (try m.planets[lane.index].?.retire(old.planets[lane.index].root, budget) == .pending) break;
            lane.index += 1;
        },
        .restore => {
            if (lane.index == lane.started) {
                lane.index = 0;
                lane.step = .discard;
                continue;
            }
            if (try m.planets[lane.index].?.restoreHidden(budget) == .pending) break;
            if (lane.installed > 0) lane.installed -= 1;
            lane.index += 1;
        },
        .discard => {
            if (lane.index == lane.started) {
                lane.step = .clear;
                continue;
            }
            const p = &m.planets[lane.index].?;
            if (try p.retire(try p.candidateRoot(), budget) == .pending) break;
            lane.index += 1;
        },
        .clear => {
            spend(budget);
            if (!lane.published) try view.discardDescriptor(s);
            m.prepared_roots = .{ null, null, null, null };
            // Drop the whole finalizer, including a partially staged receipt.
            lane.finalizer = undefined;
            lane.receipt_live = false;
            m.lane_live = false;
            return .idle;
        },
    };
    return .{ .pending = undefined };
}
