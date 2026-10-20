//! Leases and copied queries. Coordinator mutation helpers are not facade exports.
const std = @import("std");
const t = @import("types.zig");
const state = @import("state.zig");

pub const Galaxy = struct { handle: ?*anyopaque };
// Lifecycle identity only: never hashed, serialized, or used by gameplay.
var owner_generation: std.atomic.Value(u64) = .init(0);

fn nextOwnerGeneration(counter: *std.atomic.Value(u64)) t.Failure!u64 {
    var previous = counter.load(.monotonic);
    while (true) {
        const next = std.math.add(u64, previous, 1) catch return error.IdExhausted;
        if (counter.cmpxchgWeak(previous, next, .monotonic, .monotonic)) |actual| {
            previous = actual;
        } else return next;
    }
}

// Anchor before any creation cursor retains Storage. This is ownership only;
// process lifetime identity is still acquired exclusively at takeGalaxy.
pub fn anchorStorage(s: *state.Storage) *state.Storage {
    const m = s.metadata();
    std.debug.assert(m.owner == null);
    m.owner = s.*;
    s.reservation = null;
    s.owned_planets = 0;
    return &m.owner.?;
}

// Creation's final handoff: move the ledger into stable reserved storage.
// On failure the caller still owns s, including an already anchored builder.
pub fn takeGalaxy(s: *state.Storage) t.Failure!Galaxy {
    return takeGalaxyWithCounter(s, &owner_generation);
}
fn takeGalaxyWithCounter(s: *state.Storage, counter: *std.atomic.Value(u64)) t.Failure!Galaxy {
    const m = s.metadata();
    if (m.committed_descriptor == null or m.preparing_descriptor != null or m.obsolete_descriptor != null or m.owner_generation != 0) return error.NotReady;
    if (m.owner != null and s != &m.owner.?) return error.NotReady;
    const generation = try nextOwnerGeneration(counter);
    if (m.owner == null) {
        m.owner = s.*;
        s.reservation = null;
        s.owned_planets = 0;
    }
    m.owner_generation = generation;
    return .{ .handle = &m.owner.? };
}
pub fn storage(g: *const Galaxy) t.Failure!*state.Storage {
    return @ptrCast(@alignCast(g.handle orelse return error.Shutdown));
}
pub fn deinit(g: *Galaxy) t.Failure!void {
    const s = try storage(g);
    const m = s.metadata();
    if (m.lane_live or m.faulted or m.preparing_descriptor != null or m.obsolete_descriptor != null or m.descriptor_leases[0] != 0 or m.descriptor_leases[1] != 0) return error.Busy;
    g.handle = null;
    s.deinit();
}

// Closing admission does not invalidate existing leases or restart cleanup.
// Replay acquires its source through acquireView and observes the same closure.
pub fn beginFaultShutdown(g: *Galaxy) t.Failure!void {
    const s = try storage(g);
    const m = s.metadata();
    if (!m.faulted) return error.NotReady;
    m.acquisition_closed = true;
}

// Explicit destruction is outside poll's work/latency contract. The independent
// creation ledger frees whole reservations; no damaged page graph is traversed.
// Ledger/allocator corruption is outside the supported semantic fault model.
pub fn destroyFaulted(g: *Galaxy) t.Failure!void {
    const s = try storage(g);
    const m = s.metadata();
    if (!m.faulted or !m.acquisition_closed or m.descriptor_leases[0] != 0 or m.descriptor_leases[1] != 0) return error.Busy;
    // Storage itself is anchored inside the coordinator allocation. Clear the
    // external owner before freeing it and never dereference s afterwards.
    g.handle = null;
    s.deinit();
}

// Called before lane creation. A consumed generation is never restored on abort.
// Input header contains only copied scalars and a head; prepared roots are filled
// by the metered builder/finalizer before prepareDescriptor advances.
pub fn reserveDescriptor(s: *state.Storage, header: state.DescriptorHeader) t.Failure!void {
    const m = s.metadata();
    if (m.preparing_descriptor != null or m.obsolete_descriptor != null or m.lane_live) return error.Busy;
    const slot: u1 = if (m.committed_descriptor) |current| 1 - current else 0;
    if (m.committed_descriptor != null) {
        const generation = std.math.add(u64, m.descriptor_generations[slot], 1) catch return error.IdExhausted;
        m.descriptor_generations[slot] = generation;
    }
    m.preparing_descriptor = slot;
    m.descriptor_header = header;
    m.prepared_roots = .{ null, null, null, null };
    m.descriptor_cursor = 0;
    m.descriptor_ready = false;
}
pub fn prepareDescriptor(s: *state.Storage, budget: *t.WorkBudget) t.Failure!t.Step {
    if (budget.remaining > 4096) return error.InvalidBudget;
    if (budget.remaining == 0) return .pending;
    const m = s.metadata();
    const slot = m.preparing_descriptor orelse return error.NotReady;
    const d = &m.descriptors[slot];
    while (budget.remaining > 0) {
        if (m.descriptor_ready) return .done;
        budget.remaining -= 1;
        m.descriptor_work += 1;
        if (m.descriptor_cursor == 0) {
            const h = m.descriptor_header;
            if (h.history_count > s.config.global_history_capacity) return error.HistoryFull;
            d.tick = h.tick;
            d.revision = h.revision;
            d.last_sequence = h.last_sequence;
            d.policy = h.policy;
            d.history_count = h.history_count;
            d.history_head = h.history_head;
            d.accepted = h.accepted;
            d.rejected = h.rejected;
            d.planet_count = s.config.planet_count;
        } else if (m.descriptor_cursor <= s.config.planet_count) {
            const index = m.descriptor_cursor - 1;
            const root = m.prepared_roots[index] orelse return error.NotReady;
            if (@backingInt(root.planet) != @as(u32, index) + 1 or root.tick != d.tick or root.revision != d.revision) return error.InvariantViolation;
            d.planets[index] = .{ .planet = root.planet, .root = root.root, .history_count = root.history_count, .history_head = root.history_head };
        } else {
            m.descriptor_ready = true;
            return .done;
        }
        m.descriptor_cursor += 1;
    }
    return .pending;
}
// Task6 spends the publication unit and verifies the prepared/installed barrier
// before calling this infallible O(1) switch. Its publication also updates the
// admission watermark; this descriptor-only primitive does not publish jobs.
pub fn publishDescriptor(s: *state.Storage) void {
    const m = s.metadata();
    std.debug.assert(m.descriptor_ready and m.preparing_descriptor != null and m.obsolete_descriptor == null);
    m.obsolete_descriptor = m.committed_descriptor;
    m.committed_descriptor = m.preparing_descriptor.?;
    m.preparing_descriptor = null;
    m.descriptor_ready = false;
}
pub fn discardDescriptor(s: *state.Storage) t.Failure!void {
    const m = s.metadata();
    if (m.preparing_descriptor == null) return error.NotReady;
    m.preparing_descriptor = null;
    m.descriptor_ready = false;
}
pub fn canRetire(s: *const state.Storage, slot: u8) bool {
    const m = s.metadata();
    return slot < 2 and m.obsolete_descriptor != null and m.obsolete_descriptor.? == slot and m.descriptor_leases[slot] == 0;
}
// Task6 calls only after every old planet root has drained.
pub fn finishRetirement(s: *state.Storage, slot: u8) t.Failure!void {
    if (!canRetire(s, slot)) return error.Busy;
    s.metadata().obsolete_descriptor = null;
}

pub fn acquireView(g: *Galaxy) t.Failure!t.ViewToken {
    const s = try storage(g);
    const m = s.metadata();
    if (m.acquisition_closed) return error.Shutdown;
    const slot = m.committed_descriptor orelse return error.NotReady;
    return acquireSlot(s, slot);
}

// Private replay acquisition: duplicate this retained descriptor, never current by accident.
pub fn duplicateView(g: *Galaxy, token: t.ViewToken) t.Failure!t.ViewToken {
    const s = try resolve(g, token);
    if (s.metadata().acquisition_closed) return error.Shutdown;
    return acquireSlot(s, @intCast(token.descriptor_slot));
}
fn acquireSlot(s: *state.Storage, slot: u1) t.Failure!t.ViewToken {
    const m = s.metadata();
    if (m.free_lease_count == 0) return error.ViewCapacity;
    const index = m.free_leases[m.free_lease_count - 1];
    const lease = &m.leases[index];
    std.debug.assert(!lease.live and lease.generation < std.math.maxInt(u64));
    lease.generation += 1;
    lease.descriptor_slot = slot;
    lease.descriptor_generation = m.descriptor_generations[slot];
    lease.live = true;
    m.free_lease_count -= 1;
    m.descriptor_leases[slot] += 1;
    return .{ .owner_generation = m.owner_generation, .descriptor_slot = slot, .descriptor_generation = @fromBackingInt(lease.descriptor_generation), .lease_slot = index, .lease_generation = lease.generation };
}
fn resolve(g: *const Galaxy, token: t.ViewToken) t.Failure!*state.Storage {
    const s = try storage(g);
    const m = s.metadata();
    if (token.owner_generation != m.owner_generation or token.descriptor_slot > 1 or token.lease_slot >= 64) return error.StaleView;
    const lease = m.leases[token.lease_slot];
    if (!lease.live or lease.generation != token.lease_generation or lease.descriptor_slot != token.descriptor_slot or lease.descriptor_generation != @backingInt(token.descriptor_generation) or m.descriptor_generations[token.descriptor_slot] != lease.descriptor_generation) return error.StaleView;
    return s;
}
pub fn releaseView(g: *Galaxy, token: t.ViewToken) t.Failure!void {
    const s = try resolve(g, token);
    const m = s.metadata();
    const lease = &m.leases[token.lease_slot];
    lease.live = false;
    m.descriptor_leases[token.descriptor_slot] -= 1;
    if (lease.generation < std.math.maxInt(u64)) {
        m.free_leases[m.free_lease_count] = token.lease_slot;
        m.free_lease_count += 1;
    }
}
pub fn summary(g: *const Galaxy, token: t.ViewToken) t.Failure!t.Summary {
    const s = try resolve(g, token);
    const d = &s.metadata().descriptors[token.descriptor_slot];
    return .{ .tick = d.tick, .revision = d.revision, .last_sequence = d.last_sequence, .chain_head = d.history_head, .accepted = d.accepted, .rejected = d.rejected };
}
fn planetIndex(d: *const t.Descriptor, id: t.PlanetId) t.Failure!u8 {
    const value = @backingInt(id);
    if (value == 0 or value > d.planet_count) return error.InvalidPlanet;
    return @intCast(value - 1);
}
pub fn settlement(g: *const Galaxy, token: t.ViewToken, id: t.SettlementRef) t.Failure!t.SettlementValue {
    const s = try resolve(g, token);
    const d = &s.metadata().descriptors[token.descriptor_slot];
    const index = try planetIndex(d, id.planet);
    return s.metadata().planets[index].?.settlement(d.planets[index].root, id.local);
}
pub fn cell(g: *const Galaxy, token: t.ViewToken, id: t.CellId) t.Failure!t.CellValue {
    const s = try resolve(g, token);
    const d = &s.metadata().descriptors[token.descriptor_slot];
    const index = try planetIndex(d, id.planet);
    return s.metadata().planets[index].?.cell(d.planets[index].root, id);
}
pub fn receipts(g: *const Galaxy, token: t.ViewToken, cursor: u16, out: []t.ReceiptValue) t.Failure!t.ReceiptPage {
    const s = try resolve(g, token);
    const m = s.metadata();
    const d = &m.descriptors[token.descriptor_slot];
    if (out.len > 64) return error.CapacityUnsupported;
    if (cursor > d.history_count) return error.NotFound;
    const count = @min(out.len, d.history_count - cursor);
    for (0..count) |index| if (m.history_lengths[cursor + index] > 2048) return error.InvariantViolation;
    for (out[0..count], 0..) |*row, index| {
        const offset = cursor + index;
        const len = m.history_lengths[offset];
        row.len = len;
        @memcpy(row.bytes[0..len], s.reservation.?.history[offset][0..len]);
    }
    const next = cursor + @as(u16, @intCast(count));
    return .{ .written = @intCast(count), .next = if (next == d.history_count) null else next };
}
pub fn planetReceipts(g: *const Galaxy, token: t.ViewToken, id: t.PlanetId, cursor: u16, out: []t.LocalReceiptValue) t.Failure!t.ReceiptPage {
    const s = try resolve(g, token);
    const d = &s.metadata().descriptors[token.descriptor_slot];
    const index = try planetIndex(d, id);
    const pd = d.planets[index];
    if (out.len > 64) return error.CapacityUnsupported;
    if (cursor > pd.history_count) return error.NotFound;
    const p = &s.metadata().planets[index].?;
    const meta = try p.metadata(pd.root);
    if (meta.history_used != pd.history_count or !std.mem.eql(u8, &meta.chain_head, &pd.history_head)) return error.InvariantViolation;
    const count = @min(out.len, pd.history_count - cursor);
    // Preflight every fallible receipt-length check before touching caller output.
    for (0..count) |i| if (p.storage().reservation.history_meta[cursor + i] > 1024) return error.InvariantViolation;
    _ = try p.receipts(pd.root, cursor, out[0..count]);
    const next = cursor + @as(u16, @intCast(count));
    return .{ .written = @intCast(count), .next = if (next == pd.history_count) null else next };
}
comptime {
    std.debug.assert(@sizeOf(state.DescriptorHeader) <= 256);
    std.debug.assert(@sizeOf(t.PlanetDescriptor) <= 256);
}

test "cg/views/owner-generation-exhaustion-preserves-ledger" {
    var counter: std.atomic.Value(u64) = .init(std.math.maxInt(u64) - 1);
    try std.testing.expectEqual(std.math.maxInt(u64), try nextOwnerGeneration(&counter));
    var s = try state.Storage.init(std.testing.allocator, .{ .seed = 2, .planet_count = 1 });
    defer s.deinit();
    // Only the transfer precondition is needed to exercise exhausted handoff;
    // no descriptor/root query is made on this deliberately incomplete builder.
    s.metadata().committed_descriptor = 0;
    const reservation = s.reservation;
    const owned = s.owned_planets;
    try std.testing.expectError(error.IdExhausted, takeGalaxyWithCounter(&s, &counter));
    try std.testing.expectEqual(reservation, s.reservation);
    try std.testing.expectEqual(owned, s.owned_planets);
    try std.testing.expectEqual(null, s.metadata().owner);
    try std.testing.expectEqual(std.math.maxInt(u64), counter.load(.monotonic));
}
