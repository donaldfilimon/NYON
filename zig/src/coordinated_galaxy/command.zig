//! Coordinator-private global command admission. No gameplay runs in this pass.
const std = @import("std");
const t = @import("types.zig");
const state = @import("state.zig");
const island = @import("../planet_economy/command.zig");
const it = @import("../planet_economy/types.zig");

fn localCell(cell: t.CellId) it.CellId {
    return .{ .planet = @fromBackingInt(@backingInt(cell.planet)), .local = cell.local };
}

// The outer planet is routing metadata; the body is exactly the island codec.
pub fn encodeEnvelope(planet: t.PlanetId, header: t.EnvelopeHeader, payload: t.Command, out: []u8) t.Failure!t.RoutedEnvelope {
    var local_path: [65]it.CellId = undefined;
    const command: it.Command = switch (payload) {
        .author_settlement => |p| .{ .author_settlement = .{ .owner = p.owner, .anchor = localCell(p.anchor), .population = p.population, .food = p.food, .material = p.material, .economic_bypass = p.economic_bypass } },
        .grant => |p| .{ .grant = .{ .settlement = @fromBackingInt(@backingInt(p.settlement)), .food = p.food, .material = p.material, .population = p.population, .economic_bypass = p.economic_bypass } },
        .author_build => |p| .{ .author_build = .{ .settlement = @fromBackingInt(@backingInt(p.settlement)), .kind = p.kind, .anchor = localCell(p.anchor), .facing = p.facing, .economic_bypass = p.economic_bypass } },
        .author_ship => |p| blk: {
            if (p.path.len > local_path.len) return error.InvalidEnvelope;
            for (p.path, 0..) |cell, i| local_path[i] = localCell(cell);
            break :blk .{ .author_ship = .{ .source = @fromBackingInt(@backingInt(p.source)), .destination = @fromBackingInt(@backingInt(p.destination)), .resource = p.resource, .amount = p.amount, .path = local_path[0..p.path.len], .cargo_source = p.cargo_source, .economic_bypass = p.economic_bypass } };
        },
        .found => |p| .{ .found = .{ .source = @fromBackingInt(@backingInt(p.source)), .anchor = localCell(p.anchor) } },
        .build => |p| .{ .build = .{ .settlement = @fromBackingInt(@backingInt(p.settlement)), .kind = p.kind, .anchor = localCell(p.anchor), .facing = p.facing } },
        .ship => |p| blk: {
            if (p.path.len > local_path.len) return error.InvalidEnvelope;
            for (p.path, 0..) |cell, i| local_path[i] = localCell(cell);
            break :blk .{ .ship = .{ .source = @fromBackingInt(@backingInt(p.source)), .destination = @fromBackingInt(@backingInt(p.destination)), .resource = p.resource, .amount = p.amount, .path = local_path[0..p.path.len] } };
        },
    };
    const encoded = try island.encodeEnvelope(.{ .sequence = @fromBackingInt(@backingInt(header.sequence)), .provenance = header.provenance, .actor = header.actor }, command, out);
    return .{ .planet = planet, .body = encoded.bytes };
}

pub const Cursor = struct {
    storage: *state.Storage,
    policy: t.SessionPolicy,
    count: u16,
    last_sequence: t.CommandSequence,
    watermark: t.CommandSequence,
    phase: enum { policy, inspect, parse, sort, sequence, route, done } = .policy,
    index: u16 = 0,
    sort_index: u16 = 1,
    sort_cursor: u16 = 1,
    route_offset: u16 = 0,
    parser: island.StructuralParser = .{},
    route_counts: [4]u16 = @splat(0),
    owned_counts: [4]u16 = @splat(0),
    owned_lengths: [4][64]u16 = undefined,
    route_buffer: ?*[1024]u8 = null,
    work_units: u64 = 0,
    parse_units: u32 = 0,
    parsed_bytes: u32 = 0,
    max_parse_bytes_per_unit: u8 = 0,
    comparisons: u32 = 0,
    copied_bytes_at_begin: u32 = 0,
    routed_bytes: u32 = 0,

    pub fn init(storage: *state.Storage, policy: t.SessionPolicy, envelopes: []const t.RoutedEnvelope) t.Failure!Cursor {
        if (envelopes.len > 256) return error.TooManyCommands;
        const m = storage.metadata();
        var copied: u32 = 0;
        for (envelopes, 0..) |envelope, i| {
            if (envelope.body.len > 1024) return error.EnvelopeTooLarge;
            m.command_planets[i] = envelope.planet;
            m.command_lengths[i] = @intCast(envelope.body.len);
            @memcpy(m.commands[i][0..envelope.body.len], envelope.body);
            copied += @intCast(envelope.body.len + @sizeOf(t.PlanetId));
        }
        return .{ .storage = storage, .policy = policy, .count = @intCast(envelopes.len), .last_sequence = m.published_sequence, .watermark = m.published_sequence, .copied_bytes_at_begin = copied };
    }

    fn spend(self: *Cursor, budget: *t.WorkBudget) bool {
        if (budget.remaining == 0) return false;
        budget.remaining -= 1;
        self.work_units += 1;
        return true;
    }

    pub fn globalWatermark(self: *const Cursor) t.CommandSequence {
        return self.watermark;
    }

    pub fn ownedLengths(self: *const Cursor, planet: t.PlanetId) []const u16 {
        const id = @backingInt(planet);
        if (id == 0 or id > self.storage.config.planet_count) return &.{};
        return self.owned_lengths[id - 1][0..self.owned_counts[id - 1]];
    }

    pub fn routed(self: *const Cursor, sorted_index: u16) t.Failure!t.RoutedEnvelope {
        if (self.phase != .done) return error.NotReady;
        if (sorted_index >= self.count) return error.NotFound;
        const m = self.storage.metadata();
        const source = m.command_order[sorted_index];
        return .{ .planet = m.command_planets[source], .body = m.commands[source][0..m.command_lengths[source]] };
    }

    pub fn advance(self: *Cursor, budget: *t.WorkBudget) t.Failure!t.Step {
        if (budget.remaining > 4096) return error.InvalidBudget;
        if (self.phase == .done) return .done;
        if (budget.remaining == 0) return .pending;
        const m = self.storage.metadata();
        while (true) {
            if (budget.remaining == 0) return .pending;
            switch (self.phase) {
                .policy => {
                    _ = self.spend(budget);
                    _ = t.SessionPolicy.init(self.policy.role, self.policy.civilization) catch return error.InvalidPolicy;
                    if (self.policy.civilization) |owner| if (@backingInt(owner) < 1 or @backingInt(owner) > self.storage.config.civilization_count) return error.InvalidPolicy;
                    if (self.count == 0) {
                        self.phase = .done;
                        return .done;
                    }
                    self.phase = .inspect;
                },
                .inspect => {
                    if (self.index == self.count) {
                        self.phase = .parse;
                        self.index = 0;
                        continue;
                    }
                    _ = self.spend(budget);
                    const planet = @backingInt(m.command_planets[self.index]);
                    if (planet == 0 or planet > self.storage.config.planet_count) return error.InvalidPlanet;
                    if (self.route_counts[planet - 1] == 64) return error.TooManyCommands;
                    self.route_counts[planet - 1] += 1;
                    self.index += 1;
                },
                .parse => {
                    if (self.index == self.count) {
                        self.phase = .sort;
                        self.index = 0;
                        continue;
                    }
                    const len = m.command_lengths[self.index];
                    const before = budget.remaining;
                    const bytes_before = self.parser.cursor.offset;
                    const result = self.parser.advance(m.commands[self.index][0..len], budget) catch |err| {
                        if (budget.remaining < before) self.recordParse(bytes_before);
                        return err;
                    };
                    if (budget.remaining < before) self.recordParse(bytes_before);
                    if (result == .pending) {
                        if (budget.remaining == 0) return .pending;
                        continue;
                    }
                    m.command_sequences[self.index] = @backingInt(self.parser.sequence());
                    m.command_order[self.index] = @intCast(self.index);
                    self.index += 1;
                    self.parser = .{};
                },
                .sort => {
                    if (self.sort_index >= self.count) {
                        self.phase = .sequence;
                        self.index = 0;
                        continue;
                    }
                    if (!self.spend(budget)) return .pending;
                    self.comparisons += 1;
                    const left = self.sort_cursor - 1;
                    const right = self.sort_cursor;
                    if (m.command_sequences[m.command_order[left]] > m.command_sequences[m.command_order[right]]) {
                        std.mem.swap(u8, &m.command_order[left], &m.command_order[right]);
                        if (left != 0) {
                            self.sort_cursor -= 1;
                            continue;
                        }
                    }
                    self.sort_index += 1;
                    self.sort_cursor = self.sort_index;
                },
                .sequence => {
                    if (self.index == self.count) {
                        self.phase = .route;
                        self.index = 0;
                        continue;
                    }
                    if (!self.spend(budget)) return .pending;
                    const sequence = m.command_sequences[m.command_order[self.index]];
                    if (sequence == 0) return error.InvalidEnvelope;
                    if (sequence <= @backingInt(self.last_sequence)) return error.SequenceAlreadyPublished;
                    if (self.index != 0 and sequence == m.command_sequences[m.command_order[self.index - 1]]) return error.DuplicateSequence;
                    self.watermark = @fromBackingInt(sequence);
                    self.index += 1;
                },
                .route => {
                    if (self.index == self.count) {
                        self.phase = .done;
                        return .done;
                    }
                    if (!self.spend(budget)) return .pending;
                    const source = m.command_order[self.index];
                    const planet = @backingInt(m.command_planets[source]);
                    const local_index = self.owned_counts[planet - 1];
                    const len = m.command_lengths[source];
                    if (self.route_buffer == null) {
                        const authority = if (m.planets[planet - 1]) |*p| p else return error.NotReady;
                        self.route_buffer = try authority.envelopeBuffer(local_index);
                        continue;
                    }
                    if (self.route_offset < len) {
                        const amount: u16 = @intCast(@min(@as(usize, 256), len - self.route_offset));
                        @memcpy(self.route_buffer.?[self.route_offset..][0..amount], m.commands[source][self.route_offset..][0..amount]);
                        self.route_offset += amount;
                        self.routed_bytes += amount;
                        continue;
                    }
                    self.owned_lengths[planet - 1][local_index] = len;
                    self.owned_counts[planet - 1] += 1;
                    self.index += 1;
                    self.route_offset = 0;
                    self.route_buffer = null;
                    if (self.index == self.count) {
                        self.phase = .done;
                        return .done;
                    }
                },
                .done => return .done,
            }
        }
    }

    fn recordParse(self: *Cursor, bytes_before: usize) void {
        self.work_units += 1;
        self.parse_units += 1;
        self.parsed_bytes += @intCast(self.parser.cursor.offset - bytes_before);
        self.max_parse_bytes_per_unit = @max(self.max_parse_bytes_per_unit, self.parser.cursor.max_bytes_in_step);
    }
};

comptime {
    std.debug.assert(@sizeOf(Cursor) <= 16_384);
    std.debug.assert(@sizeOf(Cursor) + @sizeOf(state.Metadata) + @alignOf(Cursor) - 1 < 1_048_576);
}
