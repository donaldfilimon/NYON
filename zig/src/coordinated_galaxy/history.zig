//! Coordinator-private canonical global receipts and invisible history tail.
const std = @import("std");
const t = @import("types.zig");
const state = @import("state.zig");

const Sha256 = std.crypto.hash.sha2.Sha256;
const domain = "NYON-CG0-RECEIPT";
pub const initial_head: [32]u8 = .{ 0xcb, 0x0a, 0x4f, 0x7a, 0x9e, 0x85, 0x3e, 0xd2, 0x9f, 0x1d, 0xb4, 0xdd, 0x67, 0x33, 0x69, 0xc2, 0x4a, 0xbc, 0x5c, 0x08, 0x4d, 0xaf, 0x44, 0x04, 0xce, 0x21, 0x68, 0x74, 0x68, 0x36, 0x7b, 0x78 };

pub fn requiredTickReceipts(planet_count: u8, command_count: u16) t.Failure!u16 {
    if (planet_count < 1 or planet_count > 4) return error.InvalidConfig;
    if (command_count > 256) return error.TooManyCommands;
    return 2 + @as(u16, planet_count) + command_count;
}

pub fn hasCapacity(used: u16, capacity: u16, required: u16) bool {
    return @as(u32, used) + required <= capacity;
}

pub const Encoder = struct {
    receipt: *const t.Receipt,
    out: []u8,
    written: usize = 0,
    pc: usize = 0,
    tmp: [12]u8 = undefined,
    finished: bool = false,

    pub fn init(receipt: *const t.Receipt, out: []u8) Encoder {
        return .{ .receipt = receipt, .out = out };
    }
    pub fn len(self: *const Encoder) usize {
        return self.written;
    }
    fn number(self: *Encoder, comptime T: type, value: T) []const u8 {
        std.mem.writeInt(T, self.tmp[0..@sizeOf(T)], value, .little);
        return self.tmp[0..@sizeOf(T)];
    }
    fn id(self: *Encoder, comptime T: type, value: anytype) []const u8 {
        return self.number(T, @backingInt(value));
    }
    fn policy(self: *Encoder, value: t.SessionPolicy) []const u8 {
        self.tmp[0] = @backingInt(value.role);
        self.tmp[1] = if (value.civilization == null) 0 else 1;
        if (value.civilization) |c| {
            std.mem.writeInt(u32, self.tmp[2..6], @backingInt(c), .little);
            return self.tmp[0..6];
        }
        return self.tmp[0..2];
    }
    fn previous(self: *Encoder, value: ?t.SessionPolicy) []const u8 {
        if (value) |p| {
            const bytes = self.policy(p);
            std.mem.copyBackwards(u8, self.tmp[1 .. bytes.len + 1], bytes);
            self.tmp[0] = 1;
            return self.tmp[0 .. bytes.len + 1];
        }
        self.tmp[0] = 0;
        return self.tmp[0..1];
    }
    fn genesis(self: *Encoder, n: usize, v: *const @FieldType(t.ReceiptEvent, "genesis")) ?[]const u8 {
        const c = &v.config;
        if (n < 7) return switch (n) {
            0 => self.number(u16, t.rules_identity.len),
            1 => t.rules_identity,
            2 => self.number(u64, c.seed),
            3 => self.number(u8, c.system_count),
            4 => self.number(u8, c.civilization_count),
            5 => self.number(u8, c.planet_count),
            6 => self.number(u16, c.global_history_capacity),
            else => unreachable,
        };
        const index = (n - 7) / 9;
        if (index >= c.planet_count) return null;
        const p = &v.planets[index];
        return switch ((n - 7) % 9) {
            0 => self.id(u32, p.planet),
            1 => self.id(u32, p.system),
            2 => self.number(u64, p.derived_seed),
            3 => self.number(u16, p.side),
            4 => self.number(u16, p.capacities.settlements),
            5 => self.number(u16, p.capacities.buildings),
            6 => self.number(u16, p.capacities.freight),
            7 => self.number(u16, p.capacities.history_receipts),
            8 => &self.receipt.event.genesis.planets[index].local_genesis_head,
            else => unreachable,
        };
    }
    fn commit(self: *Encoder, n: usize, v: *const @FieldType(t.ReceiptEvent, "tick_commit")) ?[]const u8 {
        if (n < 5) return switch (n) {
            0 => self.id(u64, v.revision),
            1 => self.id(u64, v.last_sequence),
            2 => self.number(u16, v.accepted),
            3 => self.number(u16, v.rejected),
            4 => self.number(u8, v.planet_count),
            else => unreachable,
        };
        const index = (n - 5) / 5;
        if (index >= v.planet_count) return null;
        const p = &v.planets[index];
        return switch ((n - 5) % 5) {
            0 => self.id(u32, p.planet),
            1 => self.id(u64, p.tick),
            2 => self.id(u64, p.revision),
            3 => self.number(u16, p.local_history_count),
            4 => &self.receipt.event.tick_commit.planets[index].local_head,
            else => unreachable,
        };
    }
    fn payload(self: *Encoder, n: usize) ?[]const u8 {
        return switch (std.meta.activeTag(self.receipt.event)) {
            .genesis => self.genesis(n, &self.receipt.event.genesis),
            .tick_input => switch (n) {
                0 => self.id(u64, self.receipt.event.tick_input.target),
                1 => self.id(u64, self.receipt.event.tick_input.expected_revision),
                2 => self.previous(self.receipt.event.tick_input.previous_policy),
                3 => self.policy(self.receipt.event.tick_input.current_policy),
                4 => self.number(u16, self.receipt.event.tick_input.count),
                else => null,
            },
            .routed_input => blk: {
                const v = &self.receipt.event.routed_input;
                if (n == 0) break :blk self.id(u32, v.planet);
                if (n == 1) break :blk self.number(u16, @intCast(v.body.len));
                const start = (n - 2) * 256;
                if (start >= v.body.len) break :blk null;
                break :blk v.body[start..@min(start + 256, v.body.len)];
            },
            .planet_prepared => switch (n) {
                0 => self.id(u32, self.receipt.event.planet_prepared.planet),
                1 => self.number(u16, self.receipt.event.planet_prepared.local_history_count),
                2 => &self.receipt.event.planet_prepared.local_head,
                3 => self.number(u16, self.receipt.event.planet_prepared.accepted),
                4 => self.number(u16, self.receipt.event.planet_prepared.rejected),
                else => null,
            },
            .tick_commit => self.commit(n, &self.receipt.event.tick_commit),
        };
    }
    fn next(self: *Encoder) ?[]const u8 {
        return switch (self.pc) {
            0 => domain,
            1 => self.number(u8, @backingInt(std.meta.activeTag(self.receipt.event))),
            2 => self.id(u64, self.receipt.header.tick),
            3 => self.number(u16, self.receipt.header.ordinal),
            else => self.payload(self.pc - 4),
        };
    }
    pub fn advance(self: *Encoder, budget: *t.WorkBudget) t.Failure!t.Step {
        if (budget.remaining > 4096) return error.InvalidBudget;
        while (budget.remaining > 0 and !self.finished) {
            const bytes = self.next() orelse {
                self.finished = true;
                return .done;
            };
            if (bytes.len > 256 or self.written + bytes.len > self.out.len or self.written + bytes.len > 2048) return error.EnvelopeTooLarge;
            budget.remaining -= 1;
            @memcpy(self.out[self.written..][0..bytes.len], bytes);
            self.written += bytes.len;
            self.pc += 1;
        }
        return if (self.finished) .done else .pending;
    }
};

pub const Chain = struct {
    visible: [32]u8 = initial_head,
    receipt: ?[]const u8 = null,
    hash: Sha256 = Sha256.init(.{}),
    cursor: usize = 0,
    first_padding_block_done: bool = false,

    pub fn begin(self: *Chain, receipt: []const u8) t.Failure!void {
        if (self.receipt != null) return error.Busy;
        if (receipt.len > 2048) return error.EnvelopeTooLarge;
        self.receipt = receipt;
        self.cursor = 0;
        self.first_padding_block_done = false;
        self.hash = Sha256.init(.{});
    }
    pub fn advance(self: *Chain, budget: *t.WorkBudget) t.Failure!t.Step {
        if (budget.remaining > 4096) return error.InvalidBudget;
        const receipt = self.receipt orelse return .done;
        const total = 36 + receipt.len;
        while (budget.remaining > 0) {
            if (self.cursor < total) {
                var length: [4]u8 = undefined;
                std.mem.writeInt(u32, &length, @intCast(receipt.len), .little);
                const segment: []const u8 = if (self.cursor < 32) &self.visible else if (self.cursor < 36) &length else receipt;
                const offset: usize = if (self.cursor < 32) self.cursor else if (self.cursor < 36) self.cursor - 32 else self.cursor - 36;
                const n = @min(@as(usize, 64), segment.len - offset);
                budget.remaining -= 1;
                self.hash.update(segment[offset..][0..n]);
                self.cursor += n;
                continue;
            }
            if (self.hash.buf_len > 55 and !self.first_padding_block_done) {
                var padding: [64]u8 = @splat(0);
                padding[0] = 0x80;
                budget.remaining -= 1;
                self.hash.update(padding[0 .. 64 - self.hash.buf_len]);
                self.first_padding_block_done = true;
                continue;
            }
            var padding: [64]u8 = @splat(0);
            const pad_len: usize = 64 - self.hash.buf_len;
            if (!self.first_padding_block_done) padding[0] = 0x80;
            std.mem.writeInt(u64, padding[pad_len - 8 ..][0..8], @as(u64, @intCast(total)) * 8, .big);
            budget.remaining -= 1;
            self.hash.update(padding[0..pad_len]);
            for (self.hash.s, 0..) |word, index| std.mem.writeInt(u32, self.visible[index * 4 ..][0..4], word, .big);
            self.receipt = null;
            return .done;
        }
        return .pending;
    }
};

pub const Finalizer = struct {
    storage: *state.Storage,
    descriptor: ?*const t.Descriptor,
    base_count: u16 = 0,
    count: u16 = 0,
    index: u16 = 0,
    copy_offset: usize = 0,
    encoder: ?Encoder = null,
    chain: Chain = .{},
    staged_count: u16 = 0,
    mode: enum { initial, genesis, routed, prepared, committed } = .initial,
    routed_count: u16 = 0,
    expected_routes: u16 = 0,
    prepared_count: u8 = 0,
    step: enum { start, hash, copy, next, done } = .start,

    pub fn init(storage: *state.Storage, descriptor: *const t.Descriptor) Finalizer {
        return .{ .storage = storage, .descriptor = descriptor };
    }
    fn validateStage(self: *const Finalizer, receipt: *const t.Receipt) t.Failure!void {
        if (receipt.header.ordinal != self.staged_count) return error.InvariantViolation;
        const tag = std.meta.activeTag(receipt.event);
        const next_tick = std.math.add(u64, @backingInt(self.descriptor.?.tick), 1) catch return error.TickHorizon;
        switch (tag) {
            .genesis => if (self.mode != .initial or self.descriptor.?.history_count != 0 or @backingInt(receipt.header.tick) != 0) return error.InvariantViolation,
            .tick_input => if (self.mode != .initial or self.descriptor.?.history_count == 0 or @backingInt(receipt.header.tick) != next_tick or receipt.event.tick_input.count > 256) return error.InvariantViolation,
            .routed_input => if (self.mode != .routed or self.routed_count >= self.expected_routes or receipt.event.routed_input.body.len > 1024 or @backingInt(receipt.event.routed_input.planet) < 1 or @backingInt(receipt.event.routed_input.planet) > self.storage.config.planet_count) return error.InvariantViolation,
            .planet_prepared => if ((self.mode != .routed and self.mode != .prepared) or self.routed_count != self.expected_routes or self.prepared_count >= self.storage.config.planet_count or @backingInt(receipt.event.planet_prepared.planet) != @as(u32, self.prepared_count) + 1) return error.InvariantViolation,
            .tick_commit => if (self.mode != .prepared or self.prepared_count != self.storage.config.planet_count or receipt.event.tick_commit.planet_count != self.storage.config.planet_count) return error.InvariantViolation,
        }
        if (tag != .genesis and @backingInt(receipt.header.tick) != next_tick) return error.InvariantViolation;
    }
    fn finishStage(self: *Finalizer, receipt: *const t.Receipt) void {
        switch (std.meta.activeTag(receipt.event)) {
            .genesis => self.mode = .genesis,
            .tick_input => {
                self.mode = .routed;
                self.expected_routes = receipt.event.tick_input.count;
            },
            .routed_input => self.routed_count += 1,
            .planet_prepared => {
                self.mode = .prepared;
                self.prepared_count += 1;
            },
            .tick_commit => self.mode = .committed,
        }
    }
    // The source and routed body remain stable while this encoder is pending.
    // Completed bytes, rather than a padded union, live in reserved staging.
    pub fn stage(self: *Finalizer, receipt: *const t.Receipt, budget: *t.WorkBudget) t.Failure!t.Step {
        if (budget.remaining > 4096) return error.InvalidBudget;
        if (self.step != .start) return error.Busy;
        const m = self.storage.metadata();
        if (self.staged_count >= 512) return error.TooManyCommands;
        if (budget.remaining == 0) return .pending;
        if (self.encoder == null) {
            budget.remaining -= 1;
            try self.validateStage(receipt);
            self.encoder = Encoder.init(receipt, &self.storage.reservation.?.staged[self.staged_count]);
            if (budget.remaining == 0) return .pending;
        }
        if (self.encoder.?.receipt != receipt) return error.Busy;
        if (try self.encoder.?.advance(budget) == .pending) return .pending;
        m.staged_lengths[self.staged_count] = @intCast(self.encoder.?.len());
        self.finishStage(receipt);
        self.staged_count += 1;
        self.encoder = null;
        return .done;
    }
    pub fn historyCount(self: *const Finalizer) u16 {
        return self.base_count + self.count;
    }
    pub fn head(self: *const Finalizer) [32]u8 {
        return self.chain.visible;
    }
    pub fn advance(self: *Finalizer, budget: *t.WorkBudget) t.Failure!t.Step {
        if (budget.remaining > 4096) return error.InvalidBudget;
        if (budget.remaining == 0) return if (self.step == .done) .done else .pending;
        const m = self.storage.metadata();
        while (budget.remaining > 0) switch (self.step) {
            .start => {
                if (self.encoder != null) return error.NotReady;
                const count = self.staged_count;
                if (count == 0 or count > 512) return error.NotReady;
                const descriptor = self.descriptor orelse return error.NotReady;
                if (!hasCapacity(descriptor.history_count, self.storage.config.global_history_capacity, count)) return error.HistoryFull;
                if (self.mode != .genesis and self.mode != .committed) return error.NotReady;
                budget.remaining -= 1;
                self.base_count = descriptor.history_count;
                self.chain.visible = descriptor.history_head;
                self.descriptor = null;
                self.count = count;
                self.step = .hash;
            },
            .hash => {
                if (self.chain.receipt == null) {
                    const length = m.staged_lengths[self.index];
                    try self.chain.begin(self.storage.reservation.?.staged[self.index][0..length]);
                }
                if (try self.chain.advance(budget) == .pending) return .pending;
                self.step = .copy;
            },
            .copy => {
                const length = m.staged_lengths[self.index];
                if (self.copy_offset < length) {
                    const n = @min(@as(usize, 256), length - self.copy_offset);
                    budget.remaining -= 1;
                    @memcpy(self.storage.reservation.?.history[@as(usize, self.base_count) + self.index][self.copy_offset..][0..n], self.storage.reservation.?.staged[self.index][self.copy_offset..][0..n]);
                    self.copy_offset += n;
                } else self.step = .next;
            },
            .next => {
                budget.remaining -= 1;
                m.history_lengths[@as(usize, self.base_count) + self.index] = m.staged_lengths[self.index];
                self.index += 1;
                self.copy_offset = 0;
                self.step = if (self.index == self.count) .done else .hash;
            },
            .done => return .done,
        };
        return if (self.step == .done) .done else .pending;
    }
};
