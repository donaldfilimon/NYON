//! Stable caller-owned single lane. Never copy a live Setup or reuse its namespace.
const std = @import("std");
const t = @import("types.zig");
const storage = @import("storage.zig");
const generate = @import("generate.zig");
const wire = @import("wire.zig");
const hash = @import("hash.zig");
const Manifest = @import("manifest.zig");
const Self = @This();
allocator: std.mem.Allocator,
owner_namespace: u64,
next_id: u64 = 1,
next_generation: u64 = 1,
active: ?t.Token = null,
output: ?*storage.Output = null,
scratch: ?*storage.Scratch = null,
closed: bool = false,
cause: ?t.Failure = null,
state: t.Stats = .{ .phase = .taken, .cursor = 0, .units_used = 0, .total_units = 0, .requested_bytes = 0, .admission_copy_bytes = 0 },

comptime {
    storage.assertControlsFit(Self, struct { token: t.Token, manifest: Manifest, progress: t.Progress });
}

pub fn init(allocator: std.mem.Allocator, owner_namespace: u64) t.Failure!Self {
    if (owner_namespace == 0) return error.InvalidConfig;
    return .{ .allocator = allocator, .owner_namespace = owner_namespace };
}
pub fn beginCreate(self: *Self, config: t.Config) t.Failure!t.Token {
    if (self.closed) return error.Shutdown;
    if (self.active != null) return error.Busy;
    try t.validateConfig(config);
    return self.admit(config, null);
}
pub fn beginDecode(self: *Self, input: []const u8) t.Failure!t.Token {
    if (self.closed) return error.Shutdown;
    if (self.active != null) return error.Busy;
    if (input.len > 16384) return error.InvalidWire;
    return self.admit(.{}, input);
}
fn admit(self: *Self, config: t.Config, input: ?[]const u8) t.Failure!t.Token {
    const next_id = std.math.add(u64, self.next_id, 1) catch return error.IdExhausted;
    const next_generation = std.math.add(u64, self.next_generation, 1) catch return error.IdExhausted;
    const token: t.Token = .{ .owner_namespace = self.owner_namespace, .id = self.next_id, .generation = self.next_generation };
    self.next_id = next_id;
    self.next_generation = next_generation;
    const output = try self.allocator.create(storage.Output);
    errdefer self.allocator.destroy(output);
    const scratch = try self.allocator.create(storage.Scratch);
    output.config = config;
    output.wire_len = 0;
    scratch.sha = std.crypto.hash.sha2.Sha256.init(.{});
    scratch.input_len = 0;
    scratch.parsed_cursor = 0;
    scratch.record_cursor = 0;
    scratch.block_cursor = 0;
    scratch.phase = 0;
    scratch.mode = 0;
    scratch.total_units = 0;
    if (input) |bytes| {
        @memcpy(scratch.input[0..bytes.len], bytes);
        scratch.input_len = @intCast(bytes.len);
        scratch.mode = 1;
    }
    self.output = output;
    self.scratch = scratch;
    self.active = token;
    self.cause = null;
    self.state = .{ .phase = if (input != null) .parse_header else .validate_config, .cursor = 0, .units_used = 0, .total_units = 0, .requested_bytes = storage.candidateReservation(), .admission_copy_bytes = scratch.input_len };
    return token;
}
fn check(self: *const Self, token: t.Token) t.Failure!void {
    if (self.closed) return error.Shutdown;
    const active = self.active orelse return error.StaleJob;
    if (!std.meta.eql(active, token)) return error.StaleJob;
}
fn progress(self: *const Self) t.Progress {
    return switch (self.state.phase) {
        .ready => .{ .ready = self.state },
        .canceled => .canceled,
        .failed => .{ .failed = self.cause orelse error.InvariantViolation },
        else => .{ .pending = self.state },
    };
}
pub fn poll(self: *Self, token: t.Token, budget: u16) t.Failure!t.Progress {
    try self.check(token);
    if (budget > 4096) return error.InvalidBudget;
    if (budget == 0) return self.progress();
    self.state.units_used = 0;
    while (self.state.units_used < budget) {
        switch (self.state.phase) {
            .ready, .canceled, .failed => break,
            else => {},
        }
        // All admitted jobs have fewer than 2,000 units, including cleanup.
        self.state.units_used += 1;
        self.state.total_units += 1;
        self.step() catch |cause| self.cleanup(cause);
    }
    return self.progress();
}
fn transition(self: *Self, phase: t.Phase) void {
    self.state.phase = phase;
    self.state.cursor = 0;
}
fn cleanup(self: *Self, cause: ?t.Failure) void {
    self.cause = cause;
    self.transition(.retire_output);
}
fn step(self: *Self) t.Failure!void {
    // These phases remain valid after either owned pointer has retired.
    switch (self.state.phase) {
        .retire_output => {
            if (self.output) |output| self.allocator.destroy(output);
            self.output = null;
            self.transition(.retire_scratch);
            return;
        },
        .retire_scratch => {
            if (self.scratch) |scratch| self.allocator.destroy(scratch);
            self.scratch = null;
            self.transition(if (self.cause != null) .failed else .canceled);
            return;
        },
        else => {},
    }
    const output = self.output orelse return error.InvariantViolation;
    const scratch = self.scratch orelse return error.InvariantViolation;
    const c = output.config;
    const cursor = self.state.cursor;
    switch (self.state.phase) {
        .parse_header => {
            const header = try wire.parseHeader(scratch.input[0..scratch.input_len]);
            output.config = header.config;
            scratch.parsed_cursor = header.consumed;
            self.transition(.validate_config);
        },
        .validate_config => {
            try t.validateConfig(c);
            if (scratch.mode == 1 and scratch.input_len != try wire.expectedLength(c)) return error.InvalidWire;
            self.transition(.systems);
        },
        .systems => {
            output.systems[cursor] = if (scratch.mode == 1) try wire.parseSystem(try nextRecord(scratch, 16)) else try generate.system(c, @fromBackingInt(@as(u32, cursor) + 1));
            self.state.cursor += 1;
            if (self.state.cursor == c.systems) self.transition(.planets);
        },
        .planets => {
            output.planets[cursor] = if (scratch.mode == 1) try wire.parsePlanet(try nextRecord(scratch, 20)) else try generate.planet(c, @fromBackingInt(@as(u32, cursor) + 1));
            self.state.cursor += 1;
            if (self.state.cursor == c.planets) self.transition(.civilizations);
        },
        .civilizations => {
            output.civilizations[cursor] = if (scratch.mode == 1) try wire.parseCivilization(try nextRecord(scratch, 9)) else try generate.civilization(c, @fromBackingInt(@as(u32, cursor) + 1));
            self.state.cursor += 1;
            if (self.state.cursor == c.civilizations) self.transition(.validate_records);
        },
        .validate_records => {
            validateRecord(output, cursor) catch |cause| {
                if (scratch.mode == 1 and cause == error.InvariantViolation) return error.InvalidWire;
                return cause;
            };
            self.state.cursor += 1;
            if (self.state.cursor == c.systems + c.planets + c.civilizations) self.transition(.encode_header);
        },
        .encode_header => {
            output.wire_len = try wire.encodeHeader(c, &output.wire);
            self.transition(.encode_records);
        },
        .encode_records => {
            if (output.wire_len > output.wire.len) return error.InvariantViolation;
            const dest = output.wire[output.wire_len..];
            const len = if (cursor < c.systems)
                try wire.encodeSystem(output.systems[cursor], dest)
            else if (cursor < c.systems + c.planets)
                try wire.encodePlanet(output.planets[cursor - c.systems], dest)
            else
                try wire.encodeCivilization(output.civilizations[cursor - c.systems - c.planets], dest);
            output.wire_len = std.math.add(u16, output.wire_len, len) catch return error.ArithmeticOverflow;
            self.state.cursor += 1;
            if (self.state.cursor == c.systems + c.planets + c.civilizations) self.transition(.hash_blocks);
        },
        .hash_blocks => {
            const done = try hash.advance(scratch, output.wire[0..output.wire_len]);
            self.state.cursor = scratch.block_cursor;
            if (done) self.transition(.finalize_digest);
        },
        .finalize_digest => {
            output.digest = hash.finish(scratch);
            self.transition(.ready);
        },
        else => return error.InvariantViolation,
    }
}
fn nextRecord(scratch: *storage.Scratch, len: u16) t.Failure![]const u8 {
    const end = std.math.add(u16, scratch.parsed_cursor, len) catch return error.ArithmeticOverflow;
    if (end > scratch.input_len) return error.InvalidWire;
    const record = scratch.input[scratch.parsed_cursor..end];
    scratch.parsed_cursor = end;
    return record;
}
fn validateRecord(output: *const storage.Output, cursor: u16) t.Failure!void {
    const c = output.config;
    const valid = if (cursor < c.systems)
        std.meta.eql(output.systems[cursor], try generate.system(c, @fromBackingInt(@as(u32, cursor) + 1)))
    else if (cursor < c.systems + c.planets)
        std.meta.eql(output.planets[cursor - c.systems], try generate.planet(c, @fromBackingInt(@as(u32, cursor - c.systems) + 1)))
    else if (cursor < c.systems + c.planets + c.civilizations)
        std.meta.eql(output.civilizations[cursor - c.systems - c.planets], try generate.civilization(c, @fromBackingInt(@as(u32, cursor - c.systems - c.planets) + 1)))
    else
        return error.InvariantViolation;
    if (!valid) return error.InvariantViolation;
}
pub fn cancel(self: *Self, token: t.Token) t.Failure!void {
    try self.check(token);
    switch (self.state.phase) {
        .retire_output, .retire_scratch, .canceled, .failed => {},
        else => self.cleanup(null),
    }
}
pub fn take(self: *Self, token: t.Token) t.Failure!Manifest {
    try self.check(token);
    if (self.state.phase != .ready) return error.NotReady;
    const result: Manifest = .{ .allocator = self.allocator, .output = self.output orelse return error.InvariantViolation };
    self.allocator.destroy(self.scratch orelse return error.InvariantViolation);
    self.scratch = null;
    self.output = null;
    self.active = null;
    self.transition(.taken);
    return result;
}
pub fn release(self: *Self, token: t.Token) t.Failure!void {
    try self.check(token);
    switch (self.state.phase) {
        .ready, .failed, .canceled => {},
        else => return error.NotReady,
    }
    self.freeOwned();
    self.active = null;
    self.transition(.taken);
}
pub fn stats(self: *const Self, token: t.Token) t.Failure!t.Stats {
    try self.check(token);
    return self.state;
}
fn freeOwned(self: *Self) void {
    if (self.output) |output| self.allocator.destroy(output);
    if (self.scratch) |scratch| self.allocator.destroy(scratch);
    self.output = null;
    self.scratch = null;
}
pub fn deinit(self: *Self) void {
    if (self.closed) return;
    self.freeOwned();
    self.active = null;
    self.closed = true;
}

test "counter exhaustion atomic reservation and input precedence" {
    const fixture = @import("fixtures.zig");
    for ([_]bool{ false, true }) |id_exhausted| {
        var setup = try init(std.testing.allocator, 1);
        defer setup.deinit();
        setup.next_id = if (id_exhausted) std.math.maxInt(u64) else 7;
        setup.next_generation = if (id_exhausted) 8 else std.math.maxInt(u64);
        const before_id = setup.next_id;
        const before_generation = setup.next_generation;
        try std.testing.expectError(error.InvalidVersion, setup.beginCreate(.{ .version = 2 }));
        try std.testing.expectError(error.IdExhausted, setup.beginCreate(fixture.small_config));
        try std.testing.expectEqual(before_id, setup.next_id);
        try std.testing.expectEqual(before_generation, setup.next_generation);
    }
    var setup = try init(std.testing.allocator, 1);
    defer setup.deinit();
    setup.next_id = std.math.maxInt(u64) - 1;
    setup.next_generation = std.math.maxInt(u64) - 1;
    const token = try setup.beginCreate(fixture.small_config);
    try std.testing.expectEqual(std.math.maxInt(u64) - 1, token.id);
    try setup.cancel(token);
    _ = try setup.poll(token, 2);
    try setup.release(token);
    try std.testing.expectError(error.IdExhausted, setup.beginCreate(fixture.small_config));
}

test "validation failure retains cause through scratch retirement" {
    const h = @import("test_support.zig");
    var counter = h.Counter.init(std.testing.allocator);
    var setup = try init(counter.allocator(), 1);
    defer setup.deinit();
    const token = try setup.beginCreate(@import("fixtures.zig").small_config);
    _ = try h.pollChecked(&setup, token, 9, &counter);
    setup.output.?.systems[0].x += 1;
    _ = try h.pollChecked(&setup, token, 1, &counter);
    try std.testing.expectEqual(t.Phase.retire_output, (try setup.stats(token)).phase);
    try std.testing.expectError(error.NotReady, setup.release(token));
    try setup.cancel(token); // Cannot turn the recorded failure into cancellation.
    const frees = counter.frees;
    _ = try h.pollChecked(&setup, token, 1, &counter);
    try std.testing.expectEqual(frees + 1, counter.frees);
    const before = try setup.stats(token);
    _ = try h.pollChecked(&setup, token, 0, &counter);
    try std.testing.expectEqualDeep(before, try setup.stats(token));
    const result = try h.pollChecked(&setup, token, 1, &counter);
    try std.testing.expectEqual(error.InvariantViolation, result.failed);
    try std.testing.expectEqual(frees + 2, counter.frees);
    try std.testing.expectEqual(@as(usize, 0), counter.live);
    try std.testing.expectEqual(error.InvariantViolation, (try setup.poll(token, 1)).failed);
    try std.testing.expectError(error.NotReady, setup.take(token));
    try setup.release(token);
    try std.testing.expectError(error.StaleJob, setup.stats(token));
}

test "decode exhaustion admission precedence preserves paired counters" {
    const f = @import("fixtures.zig");
    const h = @import("test_support.zig");
    const oversized: [16385]u8 = @splat(0);
    for ([_]bool{ false, true }) |id_exhausted| {
        var counter = h.Counter.init(std.testing.allocator);
        var setup = try init(counter.allocator(), 1);
        defer setup.deinit();
        setup.next_id = if (id_exhausted) std.math.maxInt(u64) else 7;
        setup.next_generation = if (id_exhausted) 8 else std.math.maxInt(u64);
        const before_id = setup.next_id;
        const before_generation = setup.next_generation;
        try std.testing.expectError(error.InvalidWire, setup.beginDecode(&oversized));
        try std.testing.expectError(error.InvalidVersion, setup.beginCreate(.{ .version = 2 }));
        var invalid_version = f.small_wire;
        invalid_version[29] = 2;
        try std.testing.expectError(error.IdExhausted, setup.beginDecode(&invalid_version));
        try std.testing.expectError(error.IdExhausted, setup.beginDecode(&f.small_wire));
        try std.testing.expectEqual(before_id, setup.next_id);
        try std.testing.expectEqual(before_generation, setup.next_generation);
        try std.testing.expectEqual(@as(usize, 0), counter.attempts);
    }
}
