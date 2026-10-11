const std = @import("std");
const ids = @import("ids.zig");
const types = @import("types.zig");
const state = @import("state.zig");
const rules = @import("rules.zig");
const surface = @import("../surface.zig");

const Failure = types.Failure;
const Command = types.Command;
const CellId = ids.CellId;

const Encoder = struct {
    out: []u8,
    offset: usize = 0,

    fn bytes(self: *Encoder, source: []const u8) Failure!void {
        if (self.offset + source.len > self.out.len or self.offset + source.len > 1024) return error.EnvelopeTooLarge;
        @memcpy(self.out[self.offset..][0..source.len], source);
        self.offset += source.len;
    }
    fn int(self: *Encoder, comptime T: type, value: T) Failure!void {
        var data: [@sizeOf(T)]u8 = undefined;
        std.mem.writeInt(T, &data, value, .little);
        try self.bytes(&data);
    }
    fn boolean(self: *Encoder, value: bool) Failure!void {
        try self.int(u8, @intFromBool(value));
    }
    fn cell(self: *Encoder, value: CellId) Failure!void {
        try self.int(u32, @backingInt(value.planet));
        try self.int(u64, value.local);
    }
    fn facing(self: *Encoder, value: ?surface.Direction) Failure!void {
        if (value) |direction| {
            try self.int(u8, 1);
            try self.int(u8, directionCode(direction));
        } else try self.int(u8, 0);
    }
    fn path(self: *Encoder, cells: []const CellId) Failure!void {
        if (cells.len > 65) return error.InvalidEnvelope;
        try self.int(u16, @intCast(cells.len));
        for (cells) |cell_id| try self.cell(cell_id);
    }
};

fn directionCode(direction: surface.Direction) u8 {
    return switch (direction) {
        .east => 0,
        .west => 1,
        .north => 2,
        .south => 3,
    };
}

pub fn encodeEnvelope(header: types.CommandHeader, command: Command, out: []u8) Failure!types.Envelope {
    var writer = Encoder{ .out = out };
    try writer.int(u64, @backingInt(header.sequence));
    try writer.int(u8, @backingInt(header.provenance));
    if (header.actor) |actor| {
        try writer.int(u8, 1);
        try writer.int(u32, @backingInt(actor));
    } else try writer.int(u8, 0);
    try writer.int(u8, @backingInt(std.meta.activeTag(command)));
    switch (command) {
        .author_settlement => |p| {
            try writer.int(u32, @backingInt(p.owner));
            try writer.cell(p.anchor);
            try writer.int(u8, p.population);
            try writer.int(u32, p.food);
            try writer.int(u32, p.material);
            try writer.boolean(p.economic_bypass);
        },
        .grant => |p| {
            try writer.int(u64, @backingInt(p.settlement));
            try writer.int(u32, p.food);
            try writer.int(u32, p.material);
            try writer.int(u8, p.population);
            try writer.boolean(p.economic_bypass);
        },
        .author_build => |p| {
            try writer.int(u64, @backingInt(p.settlement));
            try writer.int(u8, @backingInt(p.kind));
            try writer.cell(p.anchor);
            try writer.facing(p.facing);
            try writer.boolean(p.economic_bypass);
        },
        .author_ship => |p| {
            try writer.int(u64, @backingInt(p.source));
            try writer.int(u64, @backingInt(p.destination));
            try writer.int(u8, @backingInt(p.resource));
            try writer.int(u16, p.amount);
            try writer.path(p.path);
            try writer.int(u8, @backingInt(p.cargo_source));
            try writer.boolean(p.economic_bypass);
        },
        .found => |p| {
            try writer.int(u64, @backingInt(p.source));
            try writer.cell(p.anchor);
        },
        .build => |p| {
            try writer.int(u64, @backingInt(p.settlement));
            try writer.int(u8, @backingInt(p.kind));
            try writer.cell(p.anchor);
            try writer.facing(p.facing);
        },
        .ship => |p| {
            try writer.int(u64, @backingInt(p.source));
            try writer.int(u64, @backingInt(p.destination));
            try writer.int(u8, @backingInt(p.resource));
            try writer.int(u16, p.amount);
            try writer.path(p.path);
        },
    }
    return .{ .bytes = out[0..writer.offset] };
}

const Decoded = struct { header: types.CommandHeader, command: Command };

const Entry = struct {
    header: types.CommandHeader,
    tag: types.CommandTag,
    owner: types.CivilizationId,
    source: ids.SettlementId,
    destination: ids.SettlementId,
    anchor: CellId,
    population: u8,
    food: u32,
    material: u32,
    kind: types.BuildingKind,
    facing: ?surface.Direction,
    resource: types.Resource,
    amount: u16,
    path_len: u16,
    path: [65]CellId,
    cargo_source: types.CargoSource,
    bypass: bool,

    fn decoded(self: *const Entry) Decoded {
        const payload: Command = switch (self.tag) {
            .author_settlement => .{ .author_settlement = .{
                .owner = self.owner,
                .anchor = self.anchor,
                .population = self.population,
                .food = self.food,
                .material = self.material,
                .economic_bypass = self.bypass,
            } },
            .grant => .{ .grant = .{
                .settlement = self.source,
                .food = self.food,
                .material = self.material,
                .population = self.population,
                .economic_bypass = self.bypass,
            } },
            .author_build => .{ .author_build = .{
                .settlement = self.source,
                .kind = self.kind,
                .anchor = self.anchor,
                .facing = self.facing,
                .economic_bypass = self.bypass,
            } },
            .author_ship => .{ .author_ship = .{
                .source = self.source,
                .destination = self.destination,
                .resource = self.resource,
                .amount = self.amount,
                .path = self.path[0..self.path_len],
                .cargo_source = self.cargo_source,
                .economic_bypass = self.bypass,
            } },
            .found => .{ .found = .{ .source = self.source, .anchor = self.anchor } },
            .build => .{ .build = .{
                .settlement = self.source,
                .kind = self.kind,
                .anchor = self.anchor,
                .facing = self.facing,
            } },
            .ship => .{ .ship = .{
                .source = self.source,
                .destination = self.destination,
                .resource = self.resource,
                .amount = self.amount,
                .path = self.path[0..self.path_len],
            } },
        };
        return .{ .header = self.header, .command = payload };
    }
};

comptime {
    std.debug.assert(@sizeOf([64]Entry) <= @sizeOf(@FieldType(state.Reservation, "scratch")));
    std.debug.assert(@offsetOf(state.Reservation, "scratch") % @alignOf(Entry) == 0);
}

fn scratchEntry(candidate: *state.Candidate, index: u16) *Entry {
    const entries: *[64]Entry = @ptrCast(@alignCast(&candidate.storage.reservation.scratch));
    return &entries[index];
}

const Read = struct {
    bytes: []const u8,
    offset: usize,

    fn int(self: *Read, comptime T: type) Failure!T {
        if (self.offset + @sizeOf(T) > self.bytes.len) return error.InvalidEnvelope;
        const value = std.mem.readInt(T, self.bytes[self.offset..][0..@sizeOf(T)], .little);
        self.offset += @sizeOf(T);
        return value;
    }
    fn boolean(self: *Read) Failure!bool {
        return switch (try self.int(u8)) {
            0 => false,
            1 => true,
            else => error.InvalidEnvelope,
        };
    }
    fn cell(self: *Read) Failure!CellId {
        return .{ .planet = @fromBackingInt(try self.int(u32)), .local = try self.int(u64) };
    }
    fn facing(self: *Read) Failure!?surface.Direction {
        return switch (try self.int(u8)) {
            0 => null,
            1 => switch (try self.int(u8)) {
                0 => .east,
                1 => .west,
                2 => .north,
                3 => .south,
                else => error.InvalidEnvelope,
            },
            else => error.InvalidEnvelope,
        };
    }
    fn kind(self: *Read) Failure!types.BuildingKind {
        return switch (try self.int(u8)) {
            0 => .farm,
            1 => .quarry,
            2 => .house,
            else => error.InvalidEnvelope,
        };
    }
    fn resource(self: *Read) Failure!types.Resource {
        return switch (try self.int(u8)) {
            0 => .food,
            1 => .material,
            else => error.InvalidEnvelope,
        };
    }
};

const ParseCursor = struct {
    offset: usize = 0,
    field: u8 = 0,
    route_index: u16 = 0,
    done: bool = false,
    max_bytes_in_step: u8 = 0,

    fn advance(self: *ParseCursor, candidate: *state.Candidate, entry: *Entry, bytes: []const u8, budget: *types.WorkBudget) Failure!types.Step {
        if (self.done) return .done;
        if (budget.remaining == 0) return .pending;
        state.spend(candidate.storage, budget);
        var reader = Read{ .bytes = bytes, .offset = self.offset };
        if (self.field < 5) {
            switch (self.field) {
                0 => entry.header.sequence = @fromBackingInt(try reader.int(u64)),
                1 => entry.header.provenance = switch (try reader.int(u8)) {
                    0 => .creator,
                    1 => .ruler,
                    2 => .autonomous,
                    else => return error.InvalidEnvelope,
                },
                2 => entry.header.actor = switch (try reader.int(u8)) {
                    0 => null,
                    1 => @fromBackingInt(try reader.int(u32)),
                    else => return error.InvalidEnvelope,
                },
                3 => entry.tag = switch (try reader.int(u8)) {
                    0 => .author_settlement,
                    1 => .grant,
                    2 => .author_build,
                    3 => .author_ship,
                    4 => .found,
                    5 => .build,
                    6 => .ship,
                    else => return error.InvalidEnvelope,
                },
                else => unreachable,
            }
            self.field += 1;
            if (self.field == 4) self.field = 5;
        } else switch (entry.tag) {
            .author_settlement => switch (self.field) {
                5 => {
                    entry.owner = @fromBackingInt(try reader.int(u32));
                    self.field = 6;
                },
                6 => {
                    entry.anchor = try reader.cell();
                    self.field = 7;
                },
                7 => {
                    entry.population = try reader.int(u8);
                    self.field = 8;
                },
                8 => {
                    entry.food = try reader.int(u32);
                    self.field = 9;
                },
                9 => {
                    entry.material = try reader.int(u32);
                    self.field = 10;
                },
                10 => {
                    entry.bypass = try reader.boolean();
                    self.done = true;
                },
                else => unreachable,
            },
            .grant => switch (self.field) {
                5 => {
                    entry.source = @fromBackingInt(try reader.int(u64));
                    self.field = 6;
                },
                6 => {
                    entry.food = try reader.int(u32);
                    self.field = 7;
                },
                7 => {
                    entry.material = try reader.int(u32);
                    self.field = 8;
                },
                8 => {
                    entry.population = try reader.int(u8);
                    self.field = 9;
                },
                9 => {
                    entry.bypass = try reader.boolean();
                    self.done = true;
                },
                else => unreachable,
            },
            .author_build, .build => switch (self.field) {
                5 => {
                    entry.source = @fromBackingInt(try reader.int(u64));
                    self.field = 6;
                },
                6 => {
                    entry.kind = try reader.kind();
                    self.field = 7;
                },
                7 => {
                    entry.anchor = try reader.cell();
                    self.field = 8;
                },
                8 => {
                    entry.facing = try reader.facing();
                    self.field = 9;
                    if (entry.tag == .build) self.done = true;
                },
                9 => {
                    entry.bypass = try reader.boolean();
                    self.done = true;
                },
                else => unreachable,
            },
            .author_ship, .ship => switch (self.field) {
                5 => {
                    entry.source = @fromBackingInt(try reader.int(u64));
                    self.field = 6;
                },
                6 => {
                    entry.destination = @fromBackingInt(try reader.int(u64));
                    self.field = 7;
                },
                7 => {
                    entry.resource = try reader.resource();
                    self.field = 8;
                },
                8 => {
                    entry.amount = try reader.int(u16);
                    self.field = 9;
                },
                9 => {
                    entry.path_len = try reader.int(u16);
                    if (entry.path_len > 65) return error.InvalidEnvelope;
                    self.route_index = 0;
                    self.field = 10;
                },
                10 => {
                    if (self.route_index < entry.path_len) {
                        entry.path[self.route_index] = try reader.cell();
                        self.route_index += 1;
                    }
                    if (self.route_index == entry.path_len) {
                        self.field = 11;
                        if (entry.tag == .ship) self.done = true;
                    }
                },
                11 => {
                    entry.cargo_source = switch (try reader.int(u8)) {
                        0 => .transfer,
                        1 => .mint,
                        else => return error.InvalidEnvelope,
                    };
                    self.field = 12;
                },
                12 => {
                    entry.bypass = try reader.boolean();
                    self.done = true;
                },
                else => unreachable,
            },
            .found => switch (self.field) {
                5 => {
                    entry.source = @fromBackingInt(try reader.int(u64));
                    self.field = 6;
                },
                6 => {
                    entry.anchor = try reader.cell();
                    self.done = true;
                },
                else => unreachable,
            },
        }
        const consumed = reader.offset - self.offset;
        std.debug.assert(consumed <= 12);
        self.max_bytes_in_step = @max(self.max_bytes_in_step, @as(u8, @intCast(consumed)));
        self.offset = reader.offset;
        if (self.done) {
            if (self.offset != bytes.len) return error.InvalidEnvelope;
            return .done;
        }
        return .pending;
    }
};

pub const Cursor = struct {
    policy: types.SessionPolicy,
    envelopes: []const types.Envelope,
    last_sequence: ids.CommandSequence,
    target: ids.Tick,
    highest_sequence: ids.CommandSequence,
    accepted: u16 = 0,
    rejected: u16 = 0,
    phase: enum { copy, parse, sort, sequence, command, validate, prepare_writes, write, meta, stage, done } = .copy,
    index: u16 = 0,
    offset: u16 = 0,
    lengths: [64]u16 = undefined,
    order: [64]u8 = undefined,
    sequences: [64]u64 = undefined,
    sort_index: u16 = 1,
    sort_cursor: u16 = 1,
    parser: ParseCursor = .{},
    parse_units: u32 = 0,
    parsed_bytes: u32 = 0,
    max_parse_bytes_per_unit: u8 = 0,
    receipt: types.Receipt = undefined,
    current: Decoded = undefined,
    eval: enum { first, second, owner, range, topology_init, footprint_terrain, footprint_cell, connection_edge, connection_claim, path_cell, path_duplicate, path_edge, path_claim, path_owner, capacity, building_count, shipment_slot, resources, ready } = .first,
    source: state.Settlement = undefined,
    destination: state.Settlement = undefined,
    source_slot: u16 = 0,
    destination_slot: u16 = 0,
    meta_value: state.RootMeta = undefined,
    action: enum { settlement, grant, building, shipment } = .grant,
    authored: bool = false,
    cargo_source: types.CargoSource = .transfer,
    kind: types.BuildingKind = .farm,
    resource: types.Resource = .food,
    amount: u16 = 0,
    anchor: CellId = undefined,
    footprint: types.Footprint = undefined,
    path_cells: []const CellId = &.{},
    scan_i: u16 = 0,
    scan_j: u16 = 0,
    scan_direction: u8 = 0,
    neighbor: CellId = undefined,
    connected: bool = false,
    per_settlement_buildings: u16 = 0,
    shipment_slot_index: u16 = 0,
    chosen_shipment_slot: ?u16 = null,
    fixed_writes: [4]Write = undefined,
    fixed_count: u8 = 0,
    prepared_fixed_writes: u32 = 0,
    write_index: u16 = 0,
    writer: ?state.RecordWriter = null,
    meta_writer: ?state.MetaWriter = null,
    effect: types.AcceptedEffect = undefined,
    accepted_receipt: bool = false,

    const Write = struct { index: u16, record: state.Record };

    fn unit(candidate: *state.Candidate, budget: *types.WorkBudget) bool {
        if (budget.remaining == 0) return false;
        state.spend(candidate.storage, budget);
        return true;
    }

    fn settlementAt(candidate: *state.Candidate, id: ids.SettlementId) Failure!?struct { slot: u16, value: state.Settlement } {
        const raw = @backingInt(id);
        if (raw == 0 or raw > candidate.storage.config.capacities.settlements) return null;
        const slot: u16 = @intCast(raw - 1);
        const value = (try candidate.readRecord(.settlement, slot)).settlement;
        if (value.id != id) return null;
        return .{ .slot = slot, .value = value };
    }

    fn firstId(self: *const Cursor) ?ids.SettlementId {
        return switch (self.current.command) {
            .author_settlement => null,
            .grant => |p| p.settlement,
            .author_build => |p| p.settlement,
            .author_ship => |p| p.source,
            .found => |p| p.source,
            .build => |p| p.settlement,
            .ship => |p| p.source,
        };
    }

    fn secondId(self: *const Cursor) ?ids.SettlementId {
        return switch (self.current.command) {
            .author_ship => |p| p.destination,
            .ship => |p| p.destination,
            else => null,
        };
    }

    fn evaluate(self: *Cursor, candidate: *state.Candidate, budget: *types.WorkBudget, context: rules.AlgorithmContext) Failure!types.Step {
        const config = candidate.storage.config;
        const topology = surface.Topology.init(config.side) catch unreachable;
        switch (self.eval) {
            .first => {
                if (!unit(candidate, budget)) return .pending;
                if (self.firstId()) |id| {
                    const result = try settlementAt(candidate, id) orelse {
                        self.reject(candidate, .UnknownSettlement, null);
                        return .done;
                    };
                    self.source_slot = result.slot;
                    self.source = result.value;
                }
                self.eval = .second;
            },
            .second => {
                if (!unit(candidate, budget)) return .pending;
                if (self.secondId()) |id| {
                    const result = try settlementAt(candidate, id) orelse {
                        self.reject(candidate, .UnknownSettlement, null);
                        return .done;
                    };
                    self.destination_slot = result.slot;
                    self.destination = result.value;
                }
                self.eval = .owner;
            },
            .owner => {
                if (!unit(candidate, budget)) return .pending;
                switch (self.current.command) {
                    .author_settlement => |p| if (!context.validOwner(p.owner)) {
                        self.reject(candidate, .UnknownCivilization, null);
                        return .done;
                    },
                    else => {
                        const owner = if (self.current.header.provenance == .creator) (if (context.civilization_count == null) ids.civilization_id else self.source.owner) else self.current.header.actor.?;
                        if (self.source.owner != owner or (self.secondId() != null and self.destination.owner != owner)) {
                            self.reject(candidate, .WrongOwner, null);
                            return .done;
                        }
                    },
                }
                self.eval = .range;
            },
            .range => {
                if (!unit(candidate, budget)) return .pending;
                var shipment_source: ?ids.SettlementId = null;
                var shipment_destination: ids.SettlementId = undefined;
                switch (self.current.command) {
                    .author_settlement => |p| {
                        self.action = .settlement;
                        self.authored = true;
                        self.anchor = p.anchor;
                        if (p.population > 4) {
                            self.reject(candidate, .InvalidPopulation, null);
                            return .done;
                        }
                    },
                    .grant => |p| {
                        self.action = .grant;
                        self.authored = true;
                        if (p.food == 0 and p.material == 0 and p.population == 0) {
                            self.reject(candidate, .InvalidGrant, null);
                            return .done;
                        }
                    },
                    .author_build => |p| {
                        self.action = .building;
                        self.authored = true;
                        self.kind = p.kind;
                        self.anchor = p.anchor;
                        if ((p.kind == .farm) != (p.facing != null)) {
                            self.reject(candidate, .InvalidFacing, null);
                            return .done;
                        }
                    },
                    .author_ship => |p| {
                        self.action = .shipment;
                        self.authored = true;
                        self.resource = p.resource;
                        self.amount = p.amount;
                        self.path_cells = p.path;
                        self.cargo_source = p.cargo_source;
                        shipment_source = p.source;
                        shipment_destination = p.destination;
                    },
                    .found => |p| {
                        self.action = .settlement;
                        self.authored = false;
                        self.anchor = p.anchor;
                    },
                    .build => |p| {
                        self.action = .building;
                        self.authored = false;
                        self.kind = p.kind;
                        self.anchor = p.anchor;
                        if ((p.kind == .farm) != (p.facing != null)) {
                            self.reject(candidate, .InvalidFacing, null);
                            return .done;
                        }
                    },
                    .ship => |p| {
                        self.action = .shipment;
                        self.authored = false;
                        self.resource = p.resource;
                        self.amount = p.amount;
                        self.path_cells = p.path;
                        self.cargo_source = .transfer;
                        shipment_source = p.source;
                        shipment_destination = p.destination;
                    },
                }
                if (shipment_source) |source| {
                    if (source == shipment_destination) {
                        self.reject(candidate, .SameEndpoint, null);
                        return .done;
                    }
                    if (self.amount == 0 or self.amount > 256) {
                        self.reject(candidate, .InvalidAmount, null);
                        return .done;
                    }
                    if (self.path_cells.len < 2) {
                        self.reject(candidate, .InvalidPathLength, null);
                        return .done;
                    }
                }
                self.eval = .topology_init;
            },
            .topology_init => {
                if (!unit(candidate, budget)) return .pending;
                switch (self.action) {
                    .grant => self.eval = .capacity,
                    .settlement => {
                        if (self.anchor.planet != context.planet) {
                            self.reject(candidate, .InvalidPlanet, 0);
                            return .done;
                        }
                        _ = topology.fromId(self.anchor.local) catch {
                            self.reject(candidate, .InvalidCell, 0);
                            return .done;
                        };
                        self.scan_i = 0;
                        self.eval = .footprint_cell;
                    },
                    .building => {
                        if (self.anchor.planet != context.planet) {
                            self.reject(candidate, .InvalidPlanet, 0);
                            return .done;
                        }
                        _ = topology.fromId(self.anchor.local) catch {
                            self.reject(candidate, .InvalidCell, 0);
                            return .done;
                        };
                        const facing = switch (self.current.command) {
                            .author_build => |p| p.facing,
                            .build => |p| p.facing,
                            else => unreachable,
                        };
                        self.footprint = rules.footprint(topology, self.kind, self.anchor, facing) catch return error.InvariantViolation;
                        self.scan_i = 0;
                        self.eval = .footprint_terrain;
                    },
                    .shipment => {
                        self.scan_i = 0;
                        self.eval = .path_cell;
                    },
                }
            },
            .footprint_terrain => {
                if (!unit(candidate, budget)) return .pending;
                const terrain = try candidate.storage.terrainAt(candidate.root, @intCast(self.anchor.local));
                if ((self.kind == .farm and terrain != .fertile) or (self.kind == .quarry and terrain != .rocky)) {
                    self.reject(candidate, .TerrainMismatch, 0);
                    return .done;
                }
                self.eval = .footprint_cell;
            },
            .footprint_cell => {
                if (self.action == .building and self.scan_i >= self.footprint.len) {
                    self.scan_i = 0;
                    self.scan_direction = 0;
                    self.eval = .connection_edge;
                    return .done;
                }
                if (self.action == .settlement and self.scan_i != 0) {
                    self.eval = .capacity;
                    return .done;
                }
                if (!unit(candidate, budget)) return .pending;
                const cell = if (self.action == .building) self.footprint.cells[self.scan_i] else self.anchor;
                if (cell.planet != context.planet) {
                    self.reject(candidate, .InvalidPlanet, self.scan_i);
                    return .done;
                }
                if (cell.local >= @as(u64, 6) * config.side * config.side) {
                    self.reject(candidate, .InvalidCell, self.scan_i);
                    return .done;
                }
                const claim = (try candidate.readRecord(.claim, @intCast(cell.local))).claim;
                if (claim.settlement != null) {
                    self.reject(candidate, .OccupiedCell, self.scan_i);
                    return .done;
                }
                self.scan_i += 1;
            },
            .connection_edge => {
                if (self.connected) {
                    self.eval = .capacity;
                    return .done;
                }
                if (self.scan_i >= self.footprint.len) {
                    self.reject(candidate, .DisconnectedFootprint, 0);
                    return .done;
                }
                if (!unit(candidate, budget)) return .pending;
                const origin = topology.fromId(self.footprint.cells[self.scan_i].local) catch return error.InvariantViolation;
                const direction: surface.Direction = switch (self.scan_direction) {
                    0 => .east,
                    1 => .west,
                    2 => .north,
                    3 => .south,
                    else => unreachable,
                };
                const neighbor_cell = topology.neighbor(origin, direction) catch return error.InvariantViolation;
                self.neighbor = .{ .planet = context.planet, .local = topology.cellId(neighbor_cell) catch return error.InvariantViolation };
                self.eval = .connection_claim;
            },
            .connection_claim => {
                if (!unit(candidate, budget)) return .pending;
                const claim = (try candidate.readRecord(.claim, @intCast(self.neighbor.local))).claim;
                if (claim.settlement == self.source.id) self.connected = true;
                self.scan_direction += 1;
                if (self.scan_direction == 4) {
                    self.scan_direction = 0;
                    self.scan_i += 1;
                }
                self.eval = .connection_edge;
            },
            .path_cell => {
                if (self.scan_i >= self.path_cells.len) {
                    self.eval = .capacity;
                    return .done;
                }
                if (!unit(candidate, budget)) return .pending;
                const cell = self.path_cells[self.scan_i];
                if (cell.planet != context.planet) {
                    self.reject(candidate, .InvalidPlanet, self.scan_i);
                    return .done;
                }
                if (cell.local >= @as(u64, 6) * config.side * config.side) {
                    self.reject(candidate, .InvalidCell, self.scan_i);
                    return .done;
                }
                if (self.scan_i == 0 and !std.meta.eql(cell, self.source.hall)) {
                    self.reject(candidate, .PathEndpointMismatch, 0);
                    return .done;
                }
                if (self.scan_i == self.path_cells.len - 1 and !std.meta.eql(cell, self.destination.hall)) {
                    self.reject(candidate, .PathEndpointMismatch, self.scan_i);
                    return .done;
                }
                self.scan_j = 0;
                self.eval = .path_duplicate;
            },
            .path_duplicate => {
                if (self.scan_j >= self.scan_i) {
                    self.scan_direction = 0;
                    self.eval = if (self.scan_i == 0) .path_claim else .path_edge;
                    return .done;
                }
                if (!unit(candidate, budget)) return .pending;
                if (std.meta.eql(self.path_cells[self.scan_j], self.path_cells[self.scan_i])) {
                    self.reject(candidate, .RepeatedCell, self.scan_i);
                    return .done;
                }
                self.scan_j += 1;
            },
            .path_edge => {
                if (self.scan_direction == 4) {
                    self.reject(candidate, .NonAdjacentPath, self.scan_i);
                    return .done;
                }
                if (!unit(candidate, budget)) return .pending;
                const origin = topology.fromId(self.path_cells[self.scan_i - 1].local) catch return error.InvariantViolation;
                const direction: surface.Direction = switch (self.scan_direction) {
                    0 => .east,
                    1 => .west,
                    2 => .north,
                    3 => .south,
                    else => unreachable,
                };
                const next = topology.neighbor(origin, direction) catch return error.InvariantViolation;
                if ((topology.cellId(next) catch return error.InvariantViolation) == self.path_cells[self.scan_i].local) {
                    self.eval = .path_claim;
                } else self.scan_direction += 1;
            },
            .path_claim => {
                if (!unit(candidate, budget)) return .pending;
                if (self.scan_i > 0 and self.scan_i + 1 < self.path_cells.len) {
                    const claim = (try candidate.readRecord(.claim, @intCast(self.path_cells[self.scan_i].local))).claim;
                    if (claim.settlement) |settlement_id| {
                        self.scan_j = @intCast(@backingInt(settlement_id) - 1);
                        self.eval = .path_owner;
                        return .done;
                    }
                }
                self.scan_i += 1;
                self.eval = .path_cell;
            },
            .path_owner => {
                if (!unit(candidate, budget)) return .pending;
                const claimed = (try candidate.readRecord(.settlement, self.scan_j)).settlement;
                if (claimed.owner != self.source.owner) {
                    self.reject(candidate, .ForeignPathCell, self.scan_i);
                    return .done;
                }
                self.scan_i += 1;
                self.eval = .path_cell;
            },
            .capacity => {
                if (!unit(candidate, budget)) return .pending;
                switch (self.action) {
                    .settlement => {
                        if (self.meta_value.settlements >= config.capacities.settlements) {
                            self.reject(candidate, .SettlementCapacity, null);
                            return .done;
                        }
                        if (self.authored) {
                            const p = self.current.command.author_settlement;
                            if (p.food > 65_535 or p.material > 65_535) {
                                self.reject(candidate, .InventoryCapacity, null);
                                return .done;
                            }
                        }
                        if (self.meta_value.next_settlement == std.math.maxInt(u64)) {
                            self.reject(candidate, .IdExhausted, null);
                            return .done;
                        }
                        self.eval = .resources;
                    },
                    .grant => {
                        const p = self.current.command.grant;
                        if (@as(u64, self.source.population) + p.population > 32 or @as(u64, self.source.population) + p.population > self.source.housing) {
                            self.reject(candidate, .PopulationCapacity, null);
                            return .done;
                        }
                        if (@as(u64, self.source.food) + self.source.incoming_food + p.food > 65_535 or
                            @as(u64, self.source.material) + self.source.incoming_material + p.material > 65_535)
                        {
                            self.reject(candidate, .InventoryCapacity, null);
                            return .done;
                        }
                        self.eval = .resources;
                    },
                    .building => {
                        if (self.meta_value.buildings >= config.capacities.buildings) {
                            self.reject(candidate, .BuildingCapacity, null);
                            return .done;
                        }
                        self.scan_i = 0;
                        self.per_settlement_buildings = 0;
                        self.eval = .building_count;
                    },
                    .shipment => {
                        if (self.meta_value.shipments >= config.capacities.shipments) {
                            self.reject(candidate, .FreightCapacity, null);
                            return .done;
                        }
                        self.shipment_slot_index = 0;
                        self.eval = .shipment_slot;
                    },
                }
            },
            .building_count => {
                if (self.scan_i >= self.meta_value.buildings) {
                    if (self.per_settlement_buildings >= 32) {
                        self.reject(candidate, .SettlementBuildingCapacity, null);
                        return .done;
                    }
                    if (self.meta_value.next_building == std.math.maxInt(u64)) {
                        self.reject(candidate, .IdExhausted, null);
                        return .done;
                    }
                    self.eval = .resources;
                    return .done;
                }
                if (!unit(candidate, budget)) return .pending;
                const building = (try candidate.readRecord(.building, self.scan_i)).building;
                if (building.settlement == self.source.id) self.per_settlement_buildings += 1;
                self.scan_i += 1;
            },
            .shipment_slot => {
                if (self.chosen_shipment_slot != null) {
                    const reserved = if (self.resource == .food) @as(u64, self.destination.food) + self.destination.incoming_food else @as(u64, self.destination.material) + self.destination.incoming_material;
                    if (reserved + self.amount > 65_535) {
                        self.reject(candidate, .InventoryCapacity, null);
                        return .done;
                    }
                    if (self.meta_value.next_shipment == std.math.maxInt(u64)) {
                        self.reject(candidate, .IdExhausted, null);
                        return .done;
                    }
                    const arrival = std.math.add(u64, @backingInt(self.target), self.path_cells.len - 1) catch {
                        self.reject(candidate, .TickHorizon, null);
                        return .done;
                    };
                    if (arrival > 1_000_000) {
                        self.reject(candidate, .TickHorizon, null);
                        return .done;
                    }
                    self.eval = .resources;
                    return .done;
                }
                if (self.shipment_slot_index >= config.capacities.shipments) return error.InvariantViolation;
                if (!unit(candidate, budget)) return .pending;
                const shipment = (try candidate.readRecord(.shipment, self.shipment_slot_index)).shipment;
                if (@backingInt(shipment.id) == 0) self.chosen_shipment_slot = self.shipment_slot_index else self.shipment_slot_index += 1;
            },
            .resources => {
                if (!unit(candidate, budget)) return .pending;
                switch (self.action) {
                    .settlement => if (!self.authored) {
                        if (self.source.food < 4) {
                            self.reject(candidate, .InsufficientFood, null);
                            return .done;
                        }
                        if (self.source.material < 20) {
                            self.reject(candidate, .InsufficientMaterial, null);
                            return .done;
                        }
                        if (self.source.population < 2) {
                            self.reject(candidate, .InsufficientPopulation, null);
                            return .done;
                        }
                    },
                    .building => if (!self.authored and self.source.material < buildingCost(self.kind)) {
                        self.reject(candidate, .InsufficientMaterial, null);
                        return .done;
                    },
                    .shipment => if (self.cargo_source == .transfer and (if (self.resource == .food) self.source.food else self.source.material) < self.amount) {
                        self.reject(candidate, if (self.resource == .food) .InsufficientFood else .InsufficientMaterial, null);
                        return .done;
                    },
                    .grant => {},
                }
                self.eval = .ready;
            },
            .ready => {
                if (!unit(candidate, budget)) return .pending;
                self.prepareEffects();
                self.fixed_count = 0;
                self.write_index = 0;
                self.writer = null;
                self.phase = .prepare_writes;
            },
        }
        return .done;
    }

    fn fixedWriteCount(self: *const Cursor) u8 {
        return switch (self.action) {
            .settlement => if (self.authored) 2 else 3,
            .grant => 1,
            .building => 2 + self.footprint.len,
            .shipment => if (self.cargo_source == .transfer) 3 else 2,
        };
    }

    fn prepareEffects(self: *Cursor) void {
        switch (self.action) {
            .settlement => {
                const id: ids.SettlementId = @fromBackingInt(self.meta_value.next_settlement);
                self.meta_value.next_settlement += 1;
                self.meta_value.settlements += 1;
                if (self.authored) {
                    const p = self.current.command.author_settlement;
                    self.effect = .{ .author_settlement = .{
                        .settlement = id,
                        .food_minted = p.food,
                        .material_minted = p.material,
                        .population_minted = p.population,
                    } };
                } else {
                    self.source.population -= 2;
                    self.source.food -= 4;
                    self.source.material -= 20;
                    self.effect = .{ .found = .{
                        .settlement = id,
                        .food_moved = 4,
                        .material_spent = 20,
                        .population_moved = 2,
                    } };
                }
            },
            .grant => {
                const p = self.current.command.grant;
                self.source.food += p.food;
                self.source.material += p.material;
                self.source.population += p.population;
                self.effect = .{ .grant = .{ .food_minted = p.food, .material_minted = p.material, .population_minted = p.population } };
            },
            .building => {
                const id: ids.BuildingId = @fromBackingInt(self.meta_value.next_building);
                self.meta_value.next_building += 1;
                self.meta_value.buildings += 1;
                if (self.authored) {
                    if (self.kind == .house) self.source.housing = @min(@as(u8, 32), self.source.housing + 4);
                    self.effect = .{ .author_build = .{ .building = id, .footprint = self.footprint, .material_waived = buildingCost(self.kind), .ticks_waived = 2 } };
                } else {
                    self.source.material -= buildingCost(self.kind);
                    self.effect = .{ .build = .{ .building = id, .footprint = self.footprint, .material_spent = buildingCost(self.kind), .remaining_ticks = 2 } };
                }
            },
            .shipment => {
                const id: ids.ShipmentId = @fromBackingInt(self.meta_value.next_shipment);
                self.meta_value.next_shipment += 1;
                self.meta_value.shipments += 1;
                const departure = self.target;
                const arrival: ids.Tick = @fromBackingInt(@backingInt(self.target) + self.path_cells.len - 1);
                if (self.cargo_source == .transfer) {
                    if (self.resource == .food) self.source.food -= self.amount else self.source.material -= self.amount;
                }
                if (self.resource == .food) self.destination.incoming_food += self.amount else self.destination.incoming_material += self.amount;
                if (self.authored) {
                    self.effect = .{ .author_ship = .{
                        .shipment = id,
                        .transferred = if (self.cargo_source == .transfer) self.amount else 0,
                        .minted = if (self.cargo_source == .mint) self.amount else 0,
                        .departure_tick = departure,
                        .arrival_tick = arrival,
                        .reserved = self.amount,
                    } };
                } else self.effect = .{ .ship = .{
                    .shipment = id,
                    .transferred = self.amount,
                    .reserved = self.amount,
                    .departure_tick = departure,
                    .arrival_tick = arrival,
                } };
            },
        }
    }

    fn prepareFixedWrite(self: *const Cursor, position: u8) Write {
        switch (self.action) {
            .settlement => {
                const id: ids.SettlementId = @fromBackingInt(self.meta_value.next_settlement - 1);
                if (!self.authored and position == 0) return .{ .index = self.source_slot, .record = .{ .settlement = self.source } };
                const ordinal = position - @as(u8, if (self.authored) 0 else 1);
                if (ordinal == 0) {
                    const settlement: state.Settlement = if (self.authored) blk: {
                        const p = self.current.command.author_settlement;
                        break :blk .{ .id = id, .owner = p.owner, .hall = p.anchor, .population = p.population, .food = p.food, .material = p.material, .incoming_food = 0, .incoming_material = 0, .housing = 4, .hunger = 0, .fed_streak = 0 };
                    } else .{ .id = id, .owner = self.source.owner, .hall = self.anchor, .population = 2, .food = 4, .material = 0, .incoming_food = 0, .incoming_material = 0, .housing = 4, .hunger = 0, .fed_streak = 0 };
                    return .{ .index = self.meta_value.settlements - 1, .record = .{ .settlement = settlement } };
                }
                return .{ .index = @intCast(self.anchor.local), .record = .{ .claim = .{ .settlement = id, .is_hall = true } } };
            },
            .grant => return .{ .index = self.source_slot, .record = .{ .settlement = self.source } },
            .building => {
                const id: ids.BuildingId = @fromBackingInt(self.meta_value.next_building - 1);
                if (position == 0) return .{ .index = self.source_slot, .record = .{ .settlement = self.source } };
                if (position == 1) {
                    const facing = switch (self.current.command) {
                        .author_build => |p| p.facing,
                        .build => |p| p.facing,
                        else => unreachable,
                    };
                    return .{ .index = self.meta_value.buildings - 1, .record = .{ .building = .{ .id = id, .settlement = self.source.id, .kind = self.kind, .anchor = self.anchor, .facing = facing, .footprint = self.footprint, .remaining_ticks = if (self.authored) 0 else 2 } } };
                }
                const cell = self.footprint.cells[position - 2];
                return .{ .index = @intCast(cell.local), .record = .{ .claim = .{ .settlement = self.source.id, .building = id } } };
            },
            .shipment => {
                const id: ids.ShipmentId = @fromBackingInt(self.meta_value.next_shipment - 1);
                if (self.cargo_source == .transfer and position == 0) return .{ .index = self.source_slot, .record = .{ .settlement = self.source } };
                const ordinal = position - @as(u8, if (self.cargo_source == .transfer) 1 else 0);
                if (ordinal == 0) return .{ .index = self.destination_slot, .record = .{ .settlement = self.destination } };
                return .{ .index = self.chosen_shipment_slot.?, .record = .{ .shipment = .{
                    .id = id,
                    .source = self.source.id,
                    .destination = self.destination.id,
                    .resource = self.resource,
                    .quantity = self.amount,
                    .departure_tick = self.target,
                    .arrival_tick = @fromBackingInt(@backingInt(self.target) + self.path_cells.len - 1),
                    .route_len = @intCast(self.path_cells.len),
                } } };
            },
        }
    }

    pub fn init(policy: types.SessionPolicy, envelopes: []const types.Envelope, last_sequence: ids.CommandSequence, target: ids.Tick) Cursor {
        return .{ .policy = policy, .envelopes = envelopes, .last_sequence = last_sequence, .highest_sequence = last_sequence, .target = target };
    }

    fn ownedEnvelope(self: *const Cursor, candidate: *state.Candidate, source_index: u16) types.Envelope {
        return .{ .bytes = candidate.storage.reservation.envelopes[source_index][0..self.lengths[source_index]] };
    }

    fn nextReceipt(self: *Cursor, candidate: *state.Candidate, source_index: u16, header: types.CommandHeader, event: types.ReceiptEvent) void {
        self.receipt = .{ .header = .{
            .tick = self.target,
            .ordinal = candidate.storage.staged_count,
            .provenance = header.provenance,
            .sequence = header.sequence,
        }, .event = event };
        _ = source_index;
        self.phase = .stage;
    }

    fn reject(self: *Cursor, candidate: *state.Candidate, reason: types.RejectReason, offending_index: ?u16) void {
        const source_index = self.order[self.index];
        self.accepted_receipt = false;
        self.nextReceipt(candidate, source_index, self.current.header, .{ .rejected = .{
            .envelope = self.ownedEnvelope(candidate, source_index),
            .reason = reason,
            .offending_index = offending_index,
        } });
    }

    pub fn advance(self: *Cursor, candidate: *state.Candidate, budget: *types.WorkBudget) Failure!types.Step {
        return self.advanceContext(candidate, budget, .{}, false);
    }

    // prepare_only stops before gameplay so a coordinator can complete routing.
    pub fn advanceContext(self: *Cursor, candidate: *state.Candidate, budget: *types.WorkBudget, context: rules.AlgorithmContext, prepare_only: bool) Failure!types.Step {
        if (!candidate.prepared) return error.NotReady;
        if (budget.remaining > 4096) return error.InvalidBudget;
        if (self.envelopes.len > 64) return error.TooManyCommands;
        if (self.phase == .done) return .done;
        if (budget.remaining == 0) return .pending;
        while (true) {
            switch (self.phase) {
                .copy => {
                    if (self.index == self.envelopes.len) {
                        self.index = 0;
                        self.phase = .parse;
                        continue;
                    }
                    const source = self.envelopes[self.index].bytes;
                    if (source.len > 1024) return error.EnvelopeTooLarge;
                    if (self.offset == source.len) {
                        self.lengths[self.index] = @intCast(source.len);
                        self.order[self.index] = @intCast(self.index);
                        self.index += 1;
                        self.offset = 0;
                        continue;
                    }
                    if (budget.remaining == 0) return .pending;
                    const n = @min(@as(usize, 256), source.len - self.offset);
                    state.spend(candidate.storage, budget);
                    @memcpy(candidate.storage.reservation.envelopes[self.index][self.offset..][0..n], source[self.offset..][0..n]);
                    self.offset += @intCast(n);
                },
                .parse => {
                    if (self.index == self.envelopes.len) {
                        self.phase = .sort;
                        continue;
                    }
                    const envelope = self.ownedEnvelope(candidate, self.index);
                    const entry = scratchEntry(candidate, self.index);
                    const before = self.parser.offset;
                    const before_budget = budget.remaining;
                    const result = try self.parser.advance(candidate, entry, envelope.bytes, budget);
                    if (budget.remaining < before_budget) {
                        self.parse_units += 1;
                        self.parsed_bytes += @intCast(self.parser.offset - before);
                        self.max_parse_bytes_per_unit = @max(self.max_parse_bytes_per_unit, self.parser.max_bytes_in_step);
                    }
                    if (result == .pending) {
                        if (budget.remaining == 0) return .pending;
                        continue;
                    }
                    self.sequences[self.index] = @backingInt(entry.header.sequence);
                    self.index += 1;
                    self.parser = .{};
                },
                .sort => {
                    if (self.sort_index >= self.envelopes.len) {
                        self.index = 0;
                        self.phase = .sequence;
                        continue;
                    }
                    if (budget.remaining == 0) return .pending;
                    state.spend(candidate.storage, budget);
                    const left = self.sort_cursor - 1;
                    const right = self.sort_cursor;
                    if (self.sequences[self.order[left]] > self.sequences[self.order[right]]) {
                        std.mem.swap(u8, &self.order[left], &self.order[right]);
                        if (left != 0) {
                            self.sort_cursor -= 1;
                            continue;
                        }
                    }
                    self.sort_index += 1;
                    self.sort_cursor = self.sort_index;
                },
                .sequence => {
                    if (self.index == self.envelopes.len) {
                        self.index = 0;
                        self.phase = .command;
                        continue;
                    }
                    if (budget.remaining == 0) return .pending;
                    state.spend(candidate.storage, budget);
                    const sequence = self.sequences[self.order[self.index]];
                    if (sequence == 0) return error.InvalidEnvelope;
                    if (sequence <= @backingInt(self.last_sequence)) return error.SequenceAlreadyPublished;
                    if (self.index != 0 and sequence == self.sequences[self.order[self.index - 1]]) return error.DuplicateSequence;
                    self.highest_sequence = @fromBackingInt(sequence);
                    self.index += 1;
                },
                .command => {
                    if (prepare_only) return .done;
                    if (self.index == self.envelopes.len) {
                        self.phase = .done;
                        continue;
                    }
                    if (budget.remaining == 0) return .pending;
                    state.spend(candidate.storage, budget);
                    const source_index = self.order[self.index];
                    const decoded = scratchEntry(candidate, source_index).decoded();
                    self.current = decoded;
                    if (admission(self.policy, decoded.header, decoded.command)) |reason| {
                        self.reject(candidate, reason, null);
                    } else {
                        self.meta_value = candidate.meta();
                        self.eval = .first;
                        self.scan_i = 0;
                        self.scan_j = 0;
                        self.scan_direction = 0;
                        self.connected = false;
                        self.per_settlement_buildings = 0;
                        self.chosen_shipment_slot = null;
                        self.phase = .validate;
                    }
                },
                .validate => {
                    if (try self.evaluate(candidate, budget, context) == .pending) return .pending;
                },
                .prepare_writes => {
                    if (self.fixed_count == self.fixedWriteCount()) {
                        self.phase = .write;
                        continue;
                    }
                    if (budget.remaining == 0) return .pending;
                    state.spend(candidate.storage, budget);
                    self.fixed_writes[self.fixed_count] = self.prepareFixedWrite(self.fixed_count);
                    self.fixed_count += 1;
                    self.prepared_fixed_writes += 1;
                },
                .write => {
                    if (self.write_index >= self.fixed_count + (if (self.action == .shipment) @as(u16, @intCast(self.path_cells.len)) else 0)) {
                        self.meta_writer = state.MetaWriter.init(self.meta_value);
                        self.phase = .meta;
                        continue;
                    }
                    if (self.writer == null) {
                        const write = if (self.write_index < self.fixed_count)
                            self.fixed_writes[self.write_index]
                        else
                            Write{ .index = self.chosen_shipment_slot.?, .record = .{ .route_cell = .{
                                .index = @intCast(self.write_index - self.fixed_count),
                                .cell = self.path_cells[self.write_index - self.fixed_count],
                            } } };
                        self.writer = state.RecordWriter.init(write.index, write.record);
                    }
                    if (try self.writer.?.advance(candidate, budget) == .pending) return .pending;
                    self.writer = null;
                    self.write_index += 1;
                },
                .meta => {
                    if (try self.meta_writer.?.advance(candidate, budget) == .pending) return .pending;
                    const source_index = self.order[self.index];
                    self.accepted_receipt = true;
                    self.nextReceipt(candidate, source_index, self.current.header, .{ .accepted = .{
                        .envelope = self.ownedEnvelope(candidate, source_index),
                        .effect = self.effect,
                    } });
                },
                .stage => {
                    if (try candidate.stage(self.receipt, budget) == .pending) return .pending;
                    if (self.accepted_receipt) self.accepted += 1 else self.rejected += 1;
                    self.index += 1;
                    self.phase = .command;
                },
                .done => return .done,
            }
        }
    }
};

fn admission(policy: types.SessionPolicy, header: types.CommandHeader, command: Command) ?types.RejectReason {
    policy.admit(header.provenance, header.actor) catch |err| return switch (err) {
        error.CreatorForbidden => .CreatorForbidden,
        error.RulerForbidden => .RulerForbidden,
        error.CivilizationRequired => .CivilizationRequired,
        error.WrongCivilization => .WrongCivilization,
        error.InvalidActor => .InvalidActor,
        error.InternalCommandOnly => .InternalCommandOnly,
    };
    const creator_tag = switch (std.meta.activeTag(command)) {
        .author_settlement, .grant, .author_build, .author_ship => true,
        .found, .build, .ship => false,
    };
    if (creator_tag != (header.provenance == .creator)) return .CommandProvenanceMismatch;
    if (creator_tag and !switch (command) {
        .author_settlement => |p| p.economic_bypass,
        .grant => |p| p.economic_bypass,
        .author_build => |p| p.economic_bypass,
        .author_ship => |p| p.economic_bypass,
        else => unreachable,
    }) return .ExplicitBypassRequired;
    return null;
}

fn buildingCost(kind: types.BuildingKind) u32 {
    return switch (kind) {
        .farm => 6,
        .quarry => 4,
        .house => 8,
    };
}
