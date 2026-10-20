//! In-memory replay owns an independent target and a retained source lease.
//! Cursor moves safely: no helper retains a pointer into it or a decoded value.
const std = @import("std");
const t = @import("types.zig");
const view = @import("view.zig");
const job = @import("job.zig");
const state = @import("state.zig");
const local = @import("../planet_economy/state.zig");
const domain = "NYON-CG0-RECEIPT";
const header_len = domain.len + 11;

pub const maximum_requested_bytes = 2 * state.maximum_requested_bytes;
pub const Cursor = struct {
    source: view.Galaxy, // non-owning handle; own lease prevents source destruction
    lease: ?t.ViewToken,
    builder: job.Builder,
    target: ?view.Galaxy = null,
    step: enum { create, genesis, input, routes, copy, boundaries, begin_tick, drive, descriptor, planet_meta, global_history, local_history, records, release, done, cleanup, drained } = .create,
    canceled: bool = false,
    failure: ?t.Failure = null,
    work_units: u64 = 0,
    requested_bytes: usize,
    index: u16 = 1,
    count: u16 = 0,
    command_index: u16 = 0,
    offset: usize = 0,
    length: u16 = 0,
    planet_index: u8 = 0,
    expected: t.Revision = @fromBackingInt(0),
    tick: t.Tick = @fromBackingInt(0),
    policy: t.SessionPolicy = undefined,
    live_job: ?t.JobId = null,
    compare_index: u16 = 0,
    record_kind: u8 = 0,
    record_index: u16 = 0,
    route_index: u16 = 0,
    route_length: u16 = 0,
    cached: bool = false,
    record: local.Record = undefined,
    terrain: t.Terrain = undefined,
    cell: t.CellId = undefined,
    meta: local.RootMeta = undefined,
    discard: ?local.Candidate = null,
    cleanup_started: bool = false,
    cleanup_fault: bool = false,
};

pub fn begin(allocator: t.Allocator, source: *view.Galaxy, token: t.ViewToken) t.Failure!Cursor {
    const lease = try view.duplicateView(source, token);
    errdefer view.releaseView(source, lease) catch unreachable;
    const s = try view.storage(source);
    // Allocation is the only eager creation work; setup/validation is polled.
    const builder = try job.beginCreate(allocator, s.config);
    return .{ .source = source.*, .lease = lease, .builder = builder, .requested_bytes = 2 * s.requestedBytes() };
}
pub fn cancel(c: *Cursor) void {
    if (c.step != .drained) c.canceled = true;
}
pub fn takeVerified(c: *Cursor) t.Failure!view.Galaxy {
    if (c.step != .done or c.canceled or c.failure != null) return error.NotReady;
    const result = c.target orelse return error.NotReady;
    c.target = null;
    return result;
}
pub fn deinit(c: *Cursor) t.Failure!void {
    if (c.step != .done and c.step != .drained) return error.Busy;
    if (c.builder.handle != null) {
        if (c.cleanup_fault) try job.destroyFaultedBuilder(&c.builder) else try job.deinitBuilder(&c.builder);
    }
    if (c.target) |*g| {
        if (c.cleanup_fault) {
            try view.beginFaultShutdown(g);
            try view.destroyFaulted(g);
        } else try job.deinit(g);
        c.target = null;
    }
    std.debug.assert(c.lease == null);
}
fn stats(c: *const Cursor, used: u16) t.Stats {
    return .{ .phase = if (c.step == .done or c.step == .drained) null else .prepare, .cursor = c.index, .units_used = used, .work_units = c.work_units, .requested_bytes = c.requested_bytes };
}
fn spend(b: *t.WorkBudget) void {
    std.debug.assert(b.remaining > 0);
    b.remaining -= 1;
}
pub fn poll(c: *Cursor, budget: *t.WorkBudget) t.ReplayProgress {
    if (budget.remaining > 4096) return .{ .failed = error.InvalidBudget };
    if (budget.remaining == 0) return .{ .pending = stats(c, 0) };
    const before = budget.remaining;
    if ((c.canceled or c.failure != null) and c.step != .drained and c.step != .cleanup and !(c.step == .release and c.cleanup_started)) {
        spend(budget);
        c.step = .cleanup;
        c.planet_index = 0;
    }
    advance(c, budget) catch |err| {
        if (c.step == .cleanup) {
            // Undrainable semantic state: retain the original allocation ledger
            // for explicit deinit, which uses fault destruction without traversal.
            c.cleanup_fault = true;
            if (c.target) |*g| (view.storage(g) catch unreachable).metadata().faulted = true;
            c.failure = error.CleanupFault;
            c.step = .release;
        } else c.failure = err;
    };
    c.work_units += before - budget.remaining;
    if (c.step == .drained) return if (c.failure) |err| .{ .failed = err } else .canceled;
    if (c.step == .done) return .complete;
    return .{ .pending = stats(c, before - budget.remaining) };
}
fn sourceStorage(c: *const Cursor) *state.Storage {
    return view.storage(&c.source) catch unreachable;
}
fn sourceDescriptor(c: *const Cursor) *const t.Descriptor {
    return &sourceStorage(c).metadata().descriptors[c.lease.?.descriptor_slot];
}
fn targetStorage(c: *const Cursor) *state.Storage {
    return view.storage(&c.target.?) catch unreachable;
}
fn targetDescriptor(c: *const Cursor) *const t.Descriptor {
    const s = targetStorage(c);
    return &s.metadata().descriptors[s.metadata().committed_descriptor.?];
}
fn receipt(c: *const Cursor, index: u16) t.Failure![]const u8 {
    const s = sourceStorage(c);
    if (index >= sourceDescriptor(c).history_count) return error.InvalidEnvelope;
    const len = s.metadata().history_lengths[index];
    if (len < header_len or len > 2048) return error.InvalidEnvelope;
    return s.reservation.?.history[index][0..len];
}
fn number(comptime T: type, bytes: []const u8, offset: usize) t.Failure!T {
    if (offset > bytes.len or @sizeOf(T) > bytes.len - offset) return error.InvalidEnvelope;
    return std.mem.readInt(T, bytes[offset..][0..@sizeOf(T)], .little);
}
fn header(bytes: []const u8, tag: u8, tick: t.Tick, ordinal: u16) t.Failure!void {
    if (!std.mem.eql(u8, bytes[0..domain.len], domain) or bytes[domain.len] != tag or try number(u64, bytes, domain.len + 1) != @backingInt(tick) or try number(u16, bytes, domain.len + 9) != ordinal) return error.InvalidEnvelope;
}
fn policy(bytes: []const u8, offset: *usize) t.Failure!t.SessionPolicy {
    const role = try number(u8, bytes, offset.*);
    offset.* += 1;
    if (role > 2) return error.InvalidPolicy;
    const present = try number(u8, bytes, offset.*);
    offset.* += 1;
    const civ: ?t.CivilizationId = switch (present) {
        0 => null,
        1 => blk: {
            const value = try number(u32, bytes, offset.*);
            offset.* += 4;
            break :blk @fromBackingInt(value);
        },
        else => return error.InvalidPolicy,
    };
    return t.SessionPolicy.init(@fromBackingInt(role), civ) catch error.InvalidPolicy;
}
fn same(comptime T: type, a: T, b: T) t.Failure!void {
    if (!std.meta.eql(a, b)) return error.InvariantViolation;
}
fn advance(c: *Cursor, b: *t.WorkBudget) t.Failure!void {
    while (b.remaining > 0) switch (c.step) {
        .create => switch (job.pollCreate(&c.builder, b)) {
            .ready => {
                if (b.remaining == 0) return;
                spend(b);
                c.target = try job.takeReady(&c.builder);
                c.step = .genesis;
            },
            .pending => return,
            .failed => |err| return err,
            .canceled => return error.InvariantViolation,
        },
        .genesis => {
            // Exact config, derived seeds and all local genesis heads are bound
            // by byte comparison, including a four-world genesis >256 bytes.
            spend(b);
            const bytes = try receipt(c, 0);
            const s = targetStorage(c);
            if (s.metadata().history_lengths[0] != bytes.len) return error.InvalidEnvelope;
            const n = @min(@as(usize, 256), bytes.len - c.offset);
            if (!std.mem.eql(u8, bytes[c.offset..][0..n], s.reservation.?.history[0][c.offset..][0..n])) return error.InvalidEnvelope;
            c.offset += n;
            if (c.offset == bytes.len) {
                c.offset = 0;
                c.step = .input;
            }
        },
        .input => {
            spend(b);
            if (c.index == sourceDescriptor(c).history_count) {
                c.step = .descriptor;
                continue;
            }
            const bytes = try receipt(c, c.index);
            const d = targetDescriptor(c);
            const next = std.math.add(u64, @backingInt(d.tick), 1) catch return error.TickHorizon;
            c.tick = @fromBackingInt(next);
            c.expected = d.revision;
            try header(bytes, 1, c.tick, 0);
            if (try number(u64, bytes, header_len) != next or try number(u64, bytes, header_len + 8) != @backingInt(c.expected)) return error.WrongRevision;
            var pos: usize = header_len + 16;
            const present = try number(u8, bytes, pos);
            pos += 1;
            const previous: ?t.SessionPolicy = switch (present) {
                0 => null,
                1 => try policy(bytes, &pos),
                else => return error.InvalidPolicy,
            };
            if (!std.meta.eql(previous, d.policy)) return error.InvalidPolicy;
            c.policy = try policy(bytes, &pos);
            c.count = try number(u16, bytes, pos);
            if (pos + 2 != bytes.len or c.count > 256) return error.InvalidEnvelope;
            c.command_index = 0;
            c.index += 1;
            c.step = .routes;
        },
        .routes => {
            spend(b);
            if (c.command_index == c.count) {
                c.planet_index = 0;
                c.step = .boundaries;
                continue;
            }
            const bytes = try receipt(c, c.index);
            try header(bytes, 2, c.tick, c.command_index + 1);
            const planet = try number(u32, bytes, header_len);
            if (planet == 0 or planet > targetStorage(c).config.planet_count) return error.InvalidPlanet;
            c.length = try number(u16, bytes, header_len + 4);
            if (c.length > 1024 or header_len + 6 + c.length != bytes.len) return error.InvalidEnvelope;
            const m = targetStorage(c).metadata();
            m.command_planets[c.command_index] = @fromBackingInt(planet);
            m.command_lengths[c.command_index] = c.length;
            c.offset = 0;
            c.step = .copy;
        },
        .copy => {
            spend(b);
            const bytes = try receipt(c, c.index);
            const n = @min(@as(usize, 256), c.length - c.offset);
            @memcpy(targetStorage(c).metadata().commands[c.command_index][c.offset..][0..n], bytes[header_len + 6 + c.offset ..][0..n]);
            c.offset += n;
            if (c.offset == c.length) {
                c.command_index += 1;
                c.index += 1;
                c.offset = 0;
                c.step = .routes;
            }
        },
        .boundaries => {
            spend(b);
            const bytes = try receipt(c, c.index);
            const planets = targetStorage(c).config.planet_count;
            if (c.planet_index < planets) {
                try header(bytes, 3, c.tick, 1 + c.count + c.planet_index);
                if (bytes.len != header_len + 42 or try number(u32, bytes, header_len) != @as(u32, c.planet_index) + 1) return error.InvalidEnvelope;
                c.planet_index += 1;
            } else {
                try header(bytes, 4, c.tick, 1 + c.count + planets);
                if (bytes.len != header_len + 21 + @as(usize, planets) * 54) return error.InvalidEnvelope;
                c.step = .begin_tick;
            }
            c.index += 1;
        },
        .begin_tick => {
            spend(b);
            c.live_job = try job.beginOwnedTick(&c.target.?, c.expected, c.tick, c.policy, c.count);
            c.step = .drive;
        },
        .drive => switch (job.poll(&c.target.?, c.live_job.?, b)) {
            .pending, .published => return,
            .idle => {
                if (b.remaining == 0) return;
                spend(b);
                c.live_job = null;
                c.step = .input;
            },
            .failed, .invalid_request => |err| return err,
            .faulted => return error.CleanupFault,
            else => return error.InvariantViolation,
        },
        .descriptor => {
            spend(b);
            const a = sourceDescriptor(c);
            const z = targetDescriptor(c);
            try same(t.Summary, .{ .tick = a.tick, .revision = a.revision, .last_sequence = a.last_sequence, .accepted = a.accepted, .rejected = a.rejected, .chain_head = a.history_head }, .{ .tick = z.tick, .revision = z.revision, .last_sequence = z.last_sequence, .accepted = z.accepted, .rejected = z.rejected, .chain_head = z.history_head });
            try same(?t.SessionPolicy, a.policy, z.policy);
            if (a.history_count != z.history_count or a.planet_count != z.planet_count) return error.InvariantViolation;
            c.planet_index = 0;
            c.cached = false;
            c.step = .planet_meta;
        },
        .planet_meta => {
            spend(b);
            if (c.planet_index == sourceDescriptor(c).planet_count) {
                c.compare_index = 0;
                c.step = .global_history;
                continue;
            }
            const i = c.planet_index;
            const a = sourceDescriptor(c).planets[i];
            const z = targetDescriptor(c).planets[i];
            if (!c.cached) {
                c.meta = try sourceStorage(c).metadata().planets[i].?.metadata(a.root);
                c.cached = true;
            } else {
                try same(local.RootMeta, c.meta, try targetStorage(c).metadata().planets[i].?.metadata(z.root));
                if (a.planet != z.planet or a.history_count != z.history_count or !std.mem.eql(u8, &a.history_head, &z.history_head)) return error.InvariantViolation;
                c.planet_index += 1;
                c.cached = false;
            }
        },
        .global_history => {
            spend(b);
            if (c.compare_index == sourceDescriptor(c).history_count) {
                c.compare_index = 0;
                c.planet_index = 0;
                c.step = .local_history;
                continue;
            }
            const bytes = try receipt(c, c.compare_index);
            const s = targetStorage(c);
            if (s.metadata().history_lengths[c.compare_index] != bytes.len) return error.InvariantViolation;
            const n = @min(@as(usize, 256), bytes.len - c.offset);
            if (!std.mem.eql(u8, bytes[c.offset..][0..n], s.reservation.?.history[c.compare_index][c.offset..][0..n])) return error.InvariantViolation;
            c.offset += n;
            if (c.offset == bytes.len) {
                c.offset = 0;
                c.compare_index += 1;
            }
        },
        .local_history => {
            spend(b);
            if (c.planet_index == sourceDescriptor(c).planet_count) {
                c.planet_index = 0;
                c.step = .records;
                continue;
            }
            const i = c.planet_index;
            if (c.compare_index == sourceDescriptor(c).planets[i].history_count) {
                c.planet_index += 1;
                c.compare_index = 0;
                continue;
            }
            const a = sourceStorage(c).metadata().planets[i].?.storage().reservation;
            const z = targetStorage(c).metadata().planets[i].?.storage().reservation;
            const len = a.history_meta[c.compare_index];
            if (len > 1024 or len != z.history_meta[c.compare_index]) return error.InvariantViolation;
            const n = @min(@as(usize, 256), len - c.offset);
            if (!std.mem.eql(u8, a.history[c.compare_index][c.offset..][0..n], z.history[c.compare_index][c.offset..][0..n])) return error.InvariantViolation;
            c.offset += n;
            if (c.offset == len) {
                c.offset = 0;
                c.compare_index += 1;
            }
        },
        .records => try compareRecord(c, b),
        .release => {
            spend(b);
            if (c.lease) |lease| {
                try view.releaseView(&c.source, lease);
                c.lease = null;
            }
            c.step = if (c.canceled or c.failure != null) .drained else .done;
        },
        .cleanup => try cleanup(c, b),
        .done, .drained => return,
    };
}

fn compareRecord(c: *Cursor, b: *t.WorkBudget) t.Failure!void {
    if (c.planet_index == sourceDescriptor(c).planet_count) {
        spend(b);
        c.step = .release;
        return;
    }
    const i = c.planet_index;
    const a = &sourceStorage(c).metadata().planets[i].?;
    const z = &targetStorage(c).metadata().planets[i].?;
    const ar = sourceDescriptor(c).planets[i].root;
    const zr = targetDescriptor(c).planets[i].root;
    const cfg = sourceStorage(c).config.planets[i];
    const count: u16 = switch (c.record_kind) {
        0, 1 => 6 * cfg.side * cfg.side,
        2 => cfg.capacities.settlements,
        3 => cfg.capacities.buildings,
        4 => cfg.capacities.freight,
        else => {
            spend(b);
            c.planet_index += 1;
            c.record_kind = 0;
            return;
        },
    };
    if (c.record_index == count) {
        spend(b);
        c.record_kind += 1;
        c.record_index = 0;
        return;
    }
    if (c.route_length > 0) {
        if (!c.cached) c.cell = (try a.readRouteCell(ar, c.record_index, c.route_index, b)).? else {
            try same(t.CellId, c.cell, (try z.readRouteCell(zr, c.record_index, c.route_index, b)).?);
            c.route_index += 1;
            if (c.route_index == c.route_length) {
                c.route_length = 0;
                c.record_index += 1;
            }
        }
    } else if (c.record_kind == 0) {
        if (!c.cached) c.terrain = (try a.readTerrain(ar, c.record_index, b)).? else {
            try same(t.Terrain, c.terrain, (try z.readTerrain(zr, c.record_index, b)).?);
            c.record_index += 1;
        }
    } else {
        const kind: local.RecordKind = switch (c.record_kind) {
            1 => .claim,
            2 => .settlement,
            3 => .building,
            4 => .shipment,
            else => unreachable,
        };
        if (!c.cached) c.record = (try a.readRecord(ar, kind, c.record_index, b)).? else {
            const r = (try z.readRecord(zr, kind, c.record_index, b)).?;
            switch (kind) {
                .claim => try same(local.Claim, c.record.claim, r.claim),
                .settlement => {
                    if (c.record.settlement.id != r.settlement.id) return error.InvariantViolation;
                    if (@backingInt(r.settlement.id) != 0) try same(local.Settlement, c.record.settlement, r.settlement);
                },
                .building => {
                    var left = c.record.building;
                    var right = r.building;
                    if (left.id != right.id) return error.InvariantViolation;
                    if (@backingInt(right.id) != 0) {
                        if (left.footprint.len > 2 or right.footprint.len > 2) return error.InvariantViolation;
                        // Unused footprint slots are not logical state.
                        if (left.footprint.len < 2) left.footprint.cells[1] = .{ .planet = @fromBackingInt(0), .local = 0 };
                        if (right.footprint.len < 2) right.footprint.cells[1] = .{ .planet = @fromBackingInt(0), .local = 0 };
                        try same(local.Building, left, right);
                    }
                },
                .shipment => {
                    if (c.record.shipment.id != r.shipment.id) return error.InvariantViolation;
                    if (@backingInt(r.shipment.id) != 0) {
                        try same(local.Shipment, c.record.shipment, r.shipment);
                        if (r.shipment.route_len > 65) return error.InvariantViolation;
                        c.route_length = r.shipment.route_len;
                        c.route_index = 0;
                    }
                },
                else => unreachable,
            }
            if (c.route_length == 0) c.record_index += 1;
        }
    }
    c.cached = !c.cached;
}
fn cleanup(c: *Cursor, b: *t.WorkBudget) t.Failure!void {
    if (!c.cleanup_started) {
        spend(b);
        c.cleanup_started = true;
        if (c.builder.handle != null) job.cancelCreate(&c.builder);
        if (c.live_job) |id| _ = job.cancel(&c.target.?, id);
        return;
    }
    if (c.builder.handle != null) {
        switch (job.pollCreate(&c.builder, b)) {
            .failed => |err| if (err == error.CleanupFault) return err,
            else => {},
        }
        const s = job.builderStorage(&c.builder).?;
        if (job.control(s).create.step == .drained) c.step = .release;
        return;
    }
    if (c.live_job) |id| {
        switch (job.poll(&c.target.?, id, b)) {
            .idle => {
                if (b.remaining == 0) return;
                spend(b);
                c.live_job = null;
            },
            .faulted => return error.CleanupFault,
            .invalid_request => |err| return err,
            else => {},
        }
        return;
    }
    if (c.target) |*g| {
        const s = try view.storage(g);
        if (s.metadata().faulted) return error.CleanupFault;
        if (c.planet_index < s.config.planet_count) {
            if (c.discard == null) {
                spend(b);
                const p = &s.metadata().planets[c.planet_index].?;
                c.discard = .{ .storage = p.storage(), .root = @intCast(p.activeRoot().slot), .prepare_index = local.directory_size };
                return;
            }
            if (try c.discard.?.advanceDiscard(b) == .done) {
                c.discard = null;
                c.planet_index += 1;
            }
            return;
        }
    }
    spend(b);
    c.step = .release;
}
