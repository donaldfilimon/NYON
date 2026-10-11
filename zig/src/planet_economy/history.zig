//! Bounded canonical receipt serialization and invisible history publication.
const std = @import("std");
const types = @import("types.zig");
const state = @import("state.zig");
const ids = @import("ids.zig");
const wire_vectors = @import("wire_vectors.zig");
const rules = @import("rules.zig");

const Sha256 = std.crypto.hash.sha2.Sha256;
const domain = "NYON-PE0-RECEIPT";

pub const Encoder = struct {
    receipt: *const types.Receipt,
    context: rules.AlgorithmContext = .{},
    out: []u8,
    written: usize = 0,
    pc: usize = 0,
    tmp: [12]u8 = undefined,
    finished: bool = false,

    pub fn init(receipt: *const types.Receipt, out: []u8) Encoder {
        return .{ .receipt = receipt, .out = out };
    }

    pub fn initContext(receipt: *const types.Receipt, out: []u8, context: rules.AlgorithmContext) Encoder {
        return .{ .receipt = receipt, .out = out, .context = context };
    }

    pub fn len(self: *const Encoder) usize {
        return self.written;
    }

    fn number(self: *Encoder, comptime T: type, value: T) []const u8 {
        std.mem.writeInt(T, self.tmp[0..@sizeOf(T)], value, .little);
        return self.tmp[0..@sizeOf(T)];
    }

    fn id(self: *Encoder, value: anytype) []const u8 {
        return self.number(u64, @backingInt(value));
    }

    fn cell(self: *Encoder, value: ids.CellId) []const u8 {
        std.mem.writeInt(u32, self.tmp[0..4], @backingInt(value.planet), .little);
        std.mem.writeInt(u64, self.tmp[4..12], value.local, .little);
        return self.tmp[0..12];
    }

    fn optional(self: *Encoder, value: anytype) []const u8 {
        const T = @TypeOf(value);
        const Child = @typeInfo(T).optional.child;
        self.tmp[0] = if (value == null) 0 else 1;
        if (value) |present| {
            switch (@typeInfo(Child)) {
                .@"enum" => std.mem.writeInt(@typeInfo(Child).@"enum".tag_type, self.tmp[1 .. 1 + @sizeOf(@typeInfo(Child).@"enum".tag_type)], @backingInt(present), .little),
                .int => std.mem.writeInt(Child, self.tmp[1 .. 1 + @sizeOf(Child)], present, .little),
                else => unreachable,
            }
            return self.tmp[0 .. 1 + @sizeOf(Child)];
        }
        return self.tmp[0..1];
    }

    fn policy(self: *Encoder, value: types.SessionPolicy) []const u8 {
        self.tmp[0] = @backingInt(value.role);
        self.tmp[1] = if (value.civilization == null) 0 else 1;
        if (value.civilization) |civilization| {
            std.mem.writeInt(u32, self.tmp[2..6], @backingInt(civilization), .little);
            return self.tmp[0..6];
        }
        return self.tmp[0..2];
    }

    fn previousPolicy(self: *Encoder, value: ?types.SessionPolicy) []const u8 {
        if (value) |policy_value| {
            const bytes = self.policy(policy_value);
            std.mem.copyBackwards(u8, self.tmp[1 .. bytes.len + 1], bytes);
            self.tmp[0] = 1;
            return self.tmp[0 .. bytes.len + 1];
        }
        self.tmp[0] = 0;
        return self.tmp[0..1];
    }

    fn footprint(self: *Encoder, n: usize, value: types.Footprint) ?[]const u8 {
        if (n == 0) return self.number(u16, value.len);
        if (n <= value.len) return self.cell(value.cells[n - 1]);
        return null;
    }

    fn effect(self: *Encoder, n: usize, value: types.AcceptedEffect) ?[]const u8 {
        return switch (value) {
            .author_settlement => |v| switch (n) {
                0 => self.id(v.settlement),
                1 => self.number(u32, v.food_minted),
                2 => self.number(u32, v.material_minted),
                3 => self.number(u8, v.population_minted),
                else => null,
            },
            .grant => |v| switch (n) {
                0 => self.number(u32, v.food_minted),
                1 => self.number(u32, v.material_minted),
                2 => self.number(u8, v.population_minted),
                else => null,
            },
            .author_build => |v| blk: {
                if (n == 0) break :blk self.id(v.building);
                if (self.footprint(n - 1, v.footprint)) |bytes| break :blk bytes;
                const tail = n - 2 - v.footprint.len;
                break :blk switch (tail) {
                    0 => self.number(u32, v.material_waived),
                    1 => self.number(u8, v.ticks_waived),
                    else => null,
                };
            },
            .author_ship => |v| switch (n) {
                0 => self.id(v.shipment),
                1 => self.number(u16, v.transferred),
                2 => self.number(u16, v.minted),
                3 => self.id(v.departure_tick),
                4 => self.id(v.arrival_tick),
                5 => self.number(u16, v.reserved),
                else => null,
            },
            .found => |v| switch (n) {
                0 => self.id(v.settlement),
                1 => self.number(u32, v.food_moved),
                2 => self.number(u32, v.material_spent),
                3 => self.number(u8, v.population_moved),
                else => null,
            },
            .build => |v| blk: {
                if (n == 0) break :blk self.id(v.building);
                if (self.footprint(n - 1, v.footprint)) |bytes| break :blk bytes;
                const tail = n - 2 - v.footprint.len;
                break :blk switch (tail) {
                    0 => self.number(u32, v.material_spent),
                    1 => self.number(u8, v.remaining_ticks),
                    else => null,
                };
            },
            .ship => |v| switch (n) {
                0 => self.id(v.shipment),
                1 => self.number(u16, v.transferred),
                2 => self.number(u16, v.reserved),
                3 => self.id(v.departure_tick),
                4 => self.id(v.arrival_tick),
                else => null,
            },
        };
    }

    fn envelope(self: *Encoder, n: usize, value: types.Envelope) ?[]const u8 {
        if (n == 0) return self.number(u16, @intCast(value.bytes.len));
        const chunks = (value.bytes.len + 255) / 256;
        if (n <= chunks) {
            const start = (n - 1) * 256;
            return value.bytes[start..@min(start + 256, value.bytes.len)];
        }
        return null;
    }

    fn payload(self: *Encoder, n: usize) ?[]const u8 {
        return switch (self.receipt.event) {
            .genesis => |v| if (self.context.civilization_count) |count| switch (n) {
                0 => self.number(u32, @backingInt(self.context.planet)),
                1 => self.number(u32, self.context.system),
                2 => self.number(u64, v.seed),
                3 => self.number(u16, v.side),
                4 => self.number(u16, v.capacities.settlements),
                5 => self.number(u16, v.capacities.buildings),
                6 => self.number(u16, v.capacities.shipments),
                7 => self.number(u16, v.capacities.history_receipts),
                8 => self.number(u8, count),
                else => if (n - 9 < count) self.number(u32, @intCast(n - 8)) else null,
            } else switch (n) {
                0 => self.number(u16, types.rules_identity.len),
                1 => types.rules_identity,
                2 => self.number(u64, v.seed),
                3 => self.number(u16, v.side),
                4 => self.number(u16, v.capacities.settlements),
                5 => self.number(u16, v.capacities.buildings),
                6 => self.number(u16, v.capacities.shipments),
                7 => self.number(u16, v.capacities.history_receipts),
                8 => self.optional(v.startup),
                9 => self.number(u32, 1),
                10 => self.number(u32, 7),
                else => null,
            },
            .tick_input => |v| switch (n) {
                0 => self.id(v.target_tick),
                1 => self.id(v.expected_revision),
                2 => self.previousPolicy(v.previous_policy),
                3 => self.policy(v.current_policy),
                4 => self.number(u16, v.envelope_count),
                else => null,
            },
            .accepted => |v| blk: {
                if (self.envelope(n, v.envelope)) |bytes| break :blk bytes;
                break :blk self.effect(n - 1 - (v.envelope.bytes.len + 255) / 256, v.effect);
            },
            .rejected => |v| blk: {
                if (self.envelope(n, v.envelope)) |bytes| break :blk bytes;
                const tail = n - 1 - (v.envelope.bytes.len + 255) / 256;
                break :blk switch (tail) {
                    0 => self.number(u16, @backingInt(v.reason)),
                    1 => self.optional(v.offending_index),
                    else => null,
                };
            },
            .arrived => |v| switch (n) {
                0 => self.id(v.shipment),
                1 => self.id(v.source),
                2 => self.id(v.destination),
                3 => self.number(u8, @backingInt(v.resource)),
                4 => self.number(u16, v.quantity),
                5 => self.number(u32, v.destination_stock_after),
                6 => self.number(u32, v.destination_reservation_after),
                else => null,
            },
            .completed => |v| switch (n) {
                0 => self.id(v.building),
                1 => self.id(v.settlement),
                2 => self.number(u8, @backingInt(v.kind)),
                else => null,
            },
            .produced => |v| switch (n) {
                0 => self.id(v.building),
                1 => self.id(v.settlement),
                2 => self.number(u8, v.workers),
                3 => self.number(u8, @backingInt(v.resource)),
                4 => self.number(u32, v.quantity),
                5 => self.number(u32, v.stock_after),
                else => null,
            },
            .idle => |v| switch (n) {
                0 => self.id(v.building),
                1 => self.id(v.settlement),
                2 => self.number(u8, @backingInt(v.reason)),
                3 => self.number(u8, v.workers_needed),
                4 => self.number(u8, v.workers_available),
                5 => self.number(u8, @backingInt(v.resource)),
                6 => self.number(u32, v.quantity),
                7 => self.number(u32, v.stock_before),
                8 => self.number(u32, v.incoming_reserved),
                else => null,
            },
            .supplied => |v| switch (n) {
                0 => self.id(v.settlement),
                1 => self.number(u8, v.needed),
                2 => self.number(u8, v.consumed),
                3 => self.number(u8, v.shortage),
                4 => self.number(u32, v.food_after),
                else => null,
            },
            .demography => |v| switch (n) {
                0 => self.id(v.settlement),
                1 => self.number(u8, @backingInt(v.kind)),
                2 => self.number(u8, v.population_before),
                3 => self.number(u8, v.population_after),
                4 => self.number(u8, v.housing),
                5 => self.number(u8, v.hunger_after),
                6 => self.number(u8, v.fed_streak_after),
                else => null,
            },
            .tick_complete => |v| switch (n) {
                0 => self.id(v.revision),
                1 => self.id(v.last_sequence),
                2 => self.number(u16, v.accepted),
                3 => self.number(u16, v.rejected),
                4 => self.number(u16, v.settlements),
                5 => self.number(u16, v.buildings),
                6 => self.number(u16, v.active_shipments),
                else => null,
            },
        };
    }

    fn next(self: *Encoder) ?[]const u8 {
        const h = self.receipt.header;
        const integrated = self.context.civilization_count != null;
        if (integrated and self.pc == 1) return self.number(u32, @backingInt(self.context.planet));
        const pc = if (integrated and self.pc > 1) self.pc - 1 else self.pc;
        return switch (pc) {
            0 => if (integrated) "NYON-CG0-PLANET-RECEIPT" else domain,
            1 => self.number(u8, @backingInt(std.meta.activeTag(self.receipt.event))),
            2 => self.id(h.tick),
            3 => self.number(u16, h.ordinal),
            4 => self.number(u8, @backingInt(h.provenance)),
            5 => self.optional(h.sequence),
            else => self.payload(pc - 6),
        };
    }

    fn validate(self: *const Encoder) types.Failure!void {
        switch (self.receipt.event) {
            .accepted => |v| {
                try checkCommandHeader(self.receipt.header, v.envelope.bytes);
                if (self.receipt.header.provenance == .autonomous) return error.InvalidEnvelope;
                const tag_offset: usize = if (v.envelope.bytes[9] == 0) 10 else 14;
                if (v.envelope.bytes[tag_offset] != @backingInt(std.meta.activeTag(v.effect))) return error.InvalidEnvelope;
                switch (v.effect) {
                    .author_build => |e| if (e.footprint.len > 2) return error.InvariantViolation,
                    .build => |e| if (e.footprint.len > 2) return error.InvariantViolation,
                    else => {},
                }
            },
            .rejected => |v| try checkCommandHeader(self.receipt.header, v.envelope.bytes),
            else => if (self.receipt.header.provenance != .autonomous or self.receipt.header.sequence != null) return error.InvariantViolation,
        }
    }

    pub fn advance(self: *Encoder, budget: *types.WorkBudget) types.Failure!types.Step {
        if (budget.remaining > 4096) return error.InvalidBudget;
        if (self.finished) return .done;
        while (budget.remaining > 0) {
            budget.remaining -= 1;
            if (self.pc == 0) try self.validate();
            const bytes = self.next() orelse {
                budget.remaining += 1;
                self.finished = true;
                return .done;
            };
            if (bytes.len > 256 or self.written + bytes.len > self.out.len or self.written + bytes.len > 1024) return error.EnvelopeTooLarge;
            @memcpy(self.out[self.written..][0..bytes.len], bytes);
            self.written += bytes.len;
            self.pc += 1;
        }
        return .pending;
    }
};

fn checkCommandHeader(header: types.ReceiptHeader, bytes: []const u8) types.Failure!void {
    if (bytes.len < 11 or bytes.len > 1024 or header.sequence == null) return error.InvalidEnvelope;
    if (std.mem.readInt(u64, bytes[0..8], .little) != @backingInt(header.sequence.?)) return error.InvalidEnvelope;
    if (bytes[8] != @backingInt(header.provenance)) return error.InvalidEnvelope;
    const actor_size: usize = switch (bytes[9]) {
        0 => 0,
        1 => 4,
        else => return error.InvalidEnvelope,
    };
    if (bytes.len < 11 + actor_size) return error.InvalidEnvelope;
}

pub const Chain = struct {
    visible: [32]u8 = wire_vectors.h0,
    receipt: ?[]const u8 = null,
    hash: Sha256 = Sha256.init(.{}),
    cursor: usize = 0,
    first_padding_block_done: bool = false,

    pub fn init() Chain {
        return .{};
    }

    pub fn initContext(context: rules.AlgorithmContext) Chain {
        if (context.civilization_count == null) return .{};
        var input: ["NYON-CG0-PLANET-HISTORY".len + 4]u8 = undefined;
        @memcpy(input[0.."NYON-CG0-PLANET-HISTORY".len], "NYON-CG0-PLANET-HISTORY");
        std.mem.writeInt(u32, input["NYON-CG0-PLANET-HISTORY".len..], @backingInt(context.planet), .little);
        var head_value: [32]u8 = undefined;
        Sha256.hash(&input, &head_value, .{});
        return .{ .visible = head_value };
    }

    pub fn head(self: *const Chain) [32]u8 {
        return self.visible;
    }

    pub fn begin(self: *Chain, receipt: []const u8) types.Failure!void {
        if (self.receipt != null) return error.Busy;
        if (receipt.len > 1024) return error.EnvelopeTooLarge;
        self.receipt = receipt;
        self.cursor = 0;
        self.first_padding_block_done = false;
        self.hash = Sha256.init(.{});
    }

    pub fn advance(self: *Chain, budget: *types.WorkBudget) types.Failure!types.Step {
        if (budget.remaining > 4096) return error.InvalidBudget;
        const receipt = self.receipt orelse return .done;
        while (budget.remaining > 0) {
            const total = 36 + receipt.len;
            if (self.cursor < total) {
                budget.remaining -= 1;
                var length: [4]u8 = undefined;
                std.mem.writeInt(u32, &length, @intCast(receipt.len), .little);
                const segment: []const u8 = if (self.cursor < 32) &self.visible else if (self.cursor < 36) &length else receipt;
                const offset: usize = if (self.cursor < 32) self.cursor else if (self.cursor < 36) self.cursor - 32 else self.cursor - 36;
                const n = @min(@as(usize, 64), segment.len - offset);
                self.hash.update(segment[offset..][0..n]);
                self.cursor += n;
                continue;
            }
            // Feed one padding block per charged unit. Calling Sha256.final
            // here could compress two blocks in one unit for lengths >55 mod 64.
            if (self.hash.buf_len > 55 and !self.first_padding_block_done) {
                budget.remaining -= 1;
                var padding: [64]u8 = @splat(0);
                padding[0] = 0x80;
                self.hash.update(padding[0 .. 64 - self.hash.buf_len]);
                self.first_padding_block_done = true;
                continue;
            }
            budget.remaining -= 1;
            var padding: [64]u8 = @splat(0);
            const pad_len: usize = 64 - self.hash.buf_len;
            if (!self.first_padding_block_done) padding[0] = 0x80;
            std.mem.writeInt(u64, padding[pad_len - 8 ..][0..8], @as(u64, @intCast(total)) * 8, .big);
            self.hash.update(padding[0..pad_len]);
            for (self.hash.s, 0..) |word, index| {
                std.mem.writeInt(u32, self.visible[index * 4 ..][0..4], word, .big);
            }
            self.receipt = null;
            return .done;
        }
        return .pending;
    }
};

pub const Finalizer = struct {
    candidate: *state.Candidate,
    context: rules.AlgorithmContext = .{},
    index: u16 = 0,
    count: u16 = 0,
    chain: Chain = Chain.init(),
    encoder: ?Encoder = null,
    writer: ?state.MetaWriter = null,
    copy_offset: usize = 0,
    step: enum { start, encode, hash, copy, meta, done } = .start,

    pub fn init(candidate: *state.Candidate) Finalizer {
        return .{ .candidate = candidate };
    }

    pub fn initContext(candidate: *state.Candidate, context: rules.AlgorithmContext) Finalizer {
        return .{ .candidate = candidate, .context = context };
    }

    pub fn advance(self: *Finalizer, budget: *types.WorkBudget) types.Failure!types.Step {
        if (budget.remaining > 4096) return error.InvalidBudget;
        if (!self.candidate.prepared) return error.NotReady;
        while (budget.remaining > 0) {
            const storage = self.candidate.storage;
            switch (self.step) {
                .start => {
                    const meta = self.candidate.meta();
                    if (@as(u32, meta.history_used) + storage.staged_count > storage.config.capacities.history_receipts) return error.HistoryFull;
                    state.spend(storage, budget);
                    self.count = storage.staged_count;
                    self.chain.visible = meta.chain_head;
                    self.step = if (self.count == 0) .meta else .encode;
                },
                .encode => {
                    if (self.encoder == null) self.encoder = Encoder.initContext(storage.logicalReceipt(self.index), &storage.reservation.staged[self.index], self.context);
                    const before = budget.remaining;
                    defer storage.work_units += @as(u64, before - budget.remaining);
                    if (try self.encoder.?.advance(budget) == .pending) return .pending;
                    const length = self.encoder.?.len();
                    storage.reservation.staged_meta[self.index] = @intCast(length);
                    try self.chain.begin(storage.reservation.staged[self.index][0..length]);
                    self.step = .hash;
                },
                .hash => {
                    const before = budget.remaining;
                    defer storage.work_units += @as(u64, before - budget.remaining);
                    if (try self.chain.advance(budget) == .pending) return .pending;
                    self.step = .copy;
                },
                .copy => {
                    const length: usize = storage.reservation.staged_meta[self.index];
                    if (self.copy_offset < length) {
                        const n = @min(@as(usize, 256), length - self.copy_offset);
                        const slot = @as(usize, self.candidate.meta().history_used) + self.index;
                        state.spend(storage, budget);
                        @memcpy(storage.reservation.history[slot][self.copy_offset..][0..n], storage.reservation.staged[self.index][self.copy_offset..][0..n]);
                        self.copy_offset += n;
                        continue;
                    }
                    state.spend(storage, budget);
                    const slot = @as(usize, self.candidate.meta().history_used) + self.index;
                    storage.reservation.history_meta[slot] = @intCast(length);
                    self.index += 1;
                    self.copy_offset = 0;
                    self.encoder = null;
                    self.step = if (self.index == self.count) .meta else .encode;
                },
                .meta => {
                    if (self.writer == null) {
                        var meta = self.candidate.meta();
                        meta.history_used += self.count;
                        meta.chain_head = self.chain.head();
                        self.writer = state.MetaWriter.init(meta);
                    }
                    if (try self.writer.?.advance(self.candidate, budget) == .pending) return .pending;
                    self.step = .done;
                    return .done;
                },
                .done => return .done,
            }
        }
        return if (self.step == .done) .done else .pending;
    }
};
