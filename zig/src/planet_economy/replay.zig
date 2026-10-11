//! Borrows one immutable published transcript. Extraction, copies, target jobs,
//! and verification all share the caller's remaining budget.
const std = @import("std");
const types = @import("types.zig");
const ids = @import("ids.zig");
const job = @import("job.zig");
const state = @import("state.zig");

pub const Cursor = struct {
    source: *const job.Authority,
    revision: ids.Revision,
    step: enum { genesis, input, scan, copy, begin, drive, compare, records, done } = .genesis,
    index: u16 = 1,
    count: u16 = 0,
    expected_count: u16 = 0,
    policy: types.SessionPolicy = undefined,
    expected: ids.Revision = @fromBackingInt(0),
    lengths: [64]u16 = undefined,
    source_offset: usize = 0,
    copy_offset: usize = 0,
    copy_length: usize = 0,
    live_job: ?ids.JobId = null,
    compare_index: u16 = 0,
    record_kind: u8 = 0,
    record_index: u16 = 0,
    route_index: u16 = 0,
    source_cached: bool = false,
    cached_record: state.Record = undefined,
    cached_cell: ids.CellId = undefined,
    cached_terrain: types.Terrain = undefined,

    pub fn init(source: *const job.Authority) Cursor {
        return .{ .source = source, .revision = source.summary().revision };
    }

    fn receipt(self: *const Cursor, index: u16) types.Failure![]const u8 {
        const meta = self.source.storage.rootMeta(self.source.storage.active);
        if (index >= meta.history_used) return error.InvariantViolation;
        const len = self.source.storage.reservation.history_meta[index];
        if (len < 29 or len > 1024) return error.InvariantViolation;
        return self.source.storage.reservation.history[index][0..len];
    }

    pub fn advance(self: *Cursor, target: *job.Authority, budget: *types.WorkBudget) types.Failure!types.Step {
        if (self.source.summary().revision != self.revision) return error.StaleView;
        if (budget.remaining > 4096) return error.InvalidBudget;
        if (self.step == .done) return .done;
        if (self.step != .drive and target.lane != null) return error.Busy;
        while (budget.remaining > 0) {
            switch (self.step) {
                .genesis => {
                    state.spend(&target.storage, budget);
                    if (target.lane != null) return error.Busy;
                    if (@backingInt(target.summary().revision) != 0 or target.stats().history_used != 1) return error.WrongRevision;
                    const source = try self.receipt(0);
                    const length = target.storage.reservation.history_meta[0];
                    // Genesis is at most 81 bytes. Its exact encoding pins all
                    // configuration fields, startup identity and chain seed.
                    if (source[16] != 0 or source.len > 81 or length != source.len or !std.mem.eql(u8, source, target.storage.reservation.history[0][0..length])) return error.InvalidEnvelope;
                    self.step = .input;
                },
                .input => {
                    state.spend(&target.storage, budget);
                    const source_meta = self.source.storage.rootMeta(self.source.storage.active);
                    if (self.index == source_meta.history_used) {
                        if (!std.meta.eql(self.source.summary(), target.summary())) return error.InvariantViolation;
                        const target_meta = target.storage.rootMeta(target.storage.active);
                        if (!std.meta.eql(source_meta, target_meta)) return error.InvariantViolation;
                        self.step = .compare;
                        self.compare_index = 0;
                        self.copy_offset = 0;
                        continue;
                    }
                    const bytes = try self.receipt(self.index);
                    if (bytes[16] != 1 or bytes[28] != 0) return error.InvalidEnvelope;
                    const tick = read(u64, bytes, 29);
                    self.expected = @fromBackingInt(read(u64, bytes, 37));
                    if (tick != @backingInt(target.summary().tick) + 1 or self.expected != target.summary().revision) return error.WrongRevision;
                    var offset: usize = 45;
                    const previous = if (bytes[offset] == 1) blk: {
                        offset += 1;
                        break :blk try policy(bytes, &offset);
                    } else blk: {
                        if (bytes[offset] != 0) return error.InvalidEnvelope;
                        offset += 1;
                        break :blk null;
                    };
                    if (!std.meta.eql(previous, target.storage.rootMeta(target.storage.active).policy)) return error.InvalidPolicy;
                    self.policy = try policy(bytes, &offset);
                    self.expected_count = read(u16, bytes, offset);
                    if (self.expected_count > 64 or offset + 2 != bytes.len) return error.InvalidEnvelope;
                    self.count = 0;
                    self.index += 1;
                    self.step = .scan;
                },
                .scan => {
                    state.spend(&target.storage, budget);
                    const bytes = try self.receipt(self.index);
                    switch (bytes[16]) {
                        2, 3 => {
                            if (self.count >= self.expected_count or bytes[28] != 1 or bytes.len < 39) return error.InvalidEnvelope;
                            self.copy_length = read(u16, bytes, 37);
                            self.source_offset = 39;
                            self.copy_offset = 0;
                            if (self.copy_length > 1024 or self.source_offset + self.copy_length > bytes.len) return error.InvalidEnvelope;
                            self.step = .copy;
                        },
                        10 => {
                            if (self.count != self.expected_count or read(u64, bytes, 29) != @backingInt(self.expected) + 1) return error.InvalidEnvelope;
                            self.index += 1;
                            self.step = .begin;
                        },
                        4...9 => self.index += 1,
                        else => return error.InvalidEnvelope,
                    }
                },
                .copy => {
                    state.spend(&target.storage, budget);
                    const bytes = try self.receipt(self.index);
                    const n = @min(@as(usize, 256), self.copy_length - self.copy_offset);
                    @memcpy(target.storage.reservation.envelopes[self.count][self.copy_offset..][0..n], bytes[self.source_offset + self.copy_offset ..][0..n]);
                    self.copy_offset += n;
                    if (self.copy_offset == self.copy_length) {
                        self.lengths[self.count] = @intCast(self.copy_length);
                        self.count += 1;
                        self.index += 1;
                        self.step = .scan;
                    }
                },
                .begin => {
                    state.spend(&target.storage, budget);
                    self.live_job = try target.beginOwned(self.expected, self.policy, self.lengths[0..self.count]);
                    self.step = .drive;
                },
                .drive => switch (target.poll(self.live_job.?, budget)) {
                    .pending => return .pending,
                    .published => return .pending,
                    .idle => {
                        self.live_job = null;
                        self.step = .input;
                    },
                    .failed, .invalid_request => |err| return err,
                    .canceled => return error.InvariantViolation,
                },
                .compare => {
                    state.spend(&target.storage, budget);
                    const total = self.source.stats().history_used;
                    if (self.compare_index == total) {
                        self.step = .records;
                        continue;
                    }
                    const bytes = try self.receipt(self.compare_index);
                    if (target.storage.reservation.history_meta[self.compare_index] != bytes.len) return error.InvariantViolation;
                    const n = @min(@as(usize, 256), bytes.len - self.copy_offset);
                    if (!std.mem.eql(u8, bytes[self.copy_offset..][0..n], target.storage.reservation.history[self.compare_index][self.copy_offset..][0..n])) return error.InvariantViolation;
                    self.copy_offset += n;
                    if (self.copy_offset == bytes.len) {
                        self.copy_offset = 0;
                        self.compare_index += 1;
                    }
                },
                .records => {
                    state.spend(&target.storage, budget);
                    const source = &self.source.storage;
                    const count: u16 = switch (self.record_kind) {
                        0, 1 => 6 * source.config.side * source.config.side,
                        2 => source.config.capacities.settlements,
                        3 => source.config.capacities.buildings,
                        4, 5 => source.config.capacities.shipments,
                        else => {
                            self.step = .done;
                            return .done;
                        },
                    };
                    if (self.record_index == count) {
                        self.record_index = 0;
                        self.record_kind += 1;
                        continue;
                    }
                    // A source read and its target read each consume their own
                    // unit. No capacity-wide query is hidden in verification.
                    if (self.record_kind == 0) {
                        if (!self.source_cached) self.cached_terrain = try source.terrainAt(source.active, self.record_index) else {
                            if (self.cached_terrain != try target.storage.terrainAt(target.storage.active, self.record_index)) return error.InvariantViolation;
                            self.record_index += 1;
                        }
                    } else if (self.record_kind == 5) {
                        if (!self.source_cached) self.cached_cell = try source.routeCell(source.active, self.record_index, self.route_index) else {
                            const value = try target.storage.routeCell(target.storage.active, self.record_index, self.route_index);
                            if (!std.meta.eql(self.cached_cell, value)) return error.InvariantViolation;
                            self.route_index += 1;
                            if (self.route_index == 65) {
                                self.route_index = 0;
                                self.record_index += 1;
                            }
                        }
                    } else {
                        const kind: state.RecordKind = switch (self.record_kind) {
                            1 => .claim,
                            2 => .settlement,
                            3 => .building,
                            4 => .shipment,
                            else => unreachable,
                        };
                        if (!self.source_cached) self.cached_record = try source.readRecord(source.active, kind, self.record_index) else {
                            const value = try target.storage.readRecord(target.storage.active, kind, self.record_index);
                            if (!std.meta.eql(self.cached_record, value)) return error.InvariantViolation;
                            self.record_index += 1;
                        }
                    }
                    self.source_cached = !self.source_cached;
                },
                .done => return .done,
            }
        }
        return .pending;
    }
};

fn read(comptime T: type, bytes: []const u8, offset: usize) T {
    return std.mem.readInt(T, bytes[offset..][0..@sizeOf(T)], .little);
}

fn policy(bytes: []const u8, offset: *usize) types.Failure!types.SessionPolicy {
    if (offset.* + 2 > bytes.len or bytes[offset.*] > 2) return error.InvalidPolicy;
    const role: @import("../play_role.zig").Role = @fromBackingInt(bytes[offset.*]);
    offset.* += 1;
    const present = bytes[offset.*];
    offset.* += 1;
    const civilization: ?types.CivilizationId = switch (present) {
        0 => null,
        1 => blk: {
            if (offset.* + 4 > bytes.len) return error.InvalidPolicy;
            const value: types.CivilizationId = @fromBackingInt(read(u32, bytes, offset.*));
            offset.* += 4;
            break :blk value;
        },
        else => return error.InvalidPolicy,
    };
    return types.SessionPolicy.init(role, civilization) catch error.InvalidPolicy;
}
