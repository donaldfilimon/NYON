const std = @import("std");
const ids = @import("ids.zig");
const types = @import("types.zig");
const rules = @import("rules.zig");

pub const region_pages = [_]u16{ 24, 768, 64, 128, 32, 192, 8 };
pub const addressed_pages = 1216;
pub const page_size = 256;
pub const directory_size = 2048;
pub const pool_size = 4096;
pub const requested_bytes = 8_642_560;
pub const region_start = [_]u16{ 0, 24, 792, 856, 984, 1016, 1208 };

const PageSlot = extern struct {
    bytes: [page_size]u8,
    references: u16,
    next_free: u16,
    padding: [12]u8,
};

// Every field is part of the fixed reservation, including future job scratch.
pub const Reservation = extern struct {
    pool: [pool_size]PageSlot,
    directories: [2][directory_size]u64,
    history: [4096][1024]u8,
    history_meta: [4096]u32,
    staged: [1024][1024]u8,
    staged_meta: [1024]u32,
    envelopes: [64][1024]u8,
    command_meta: [4096]u8,
    logical_receipts: [@sizeOf([1024]types.Receipt)]u8 align(@alignOf(types.Receipt)),
    scratch: [2_097_152 - @sizeOf([1024]types.Receipt)]u8,
    control: [65_536]u8,
};

comptime {
    std.debug.assert(@sizeOf(PageSlot) == 272);
    std.debug.assert(@sizeOf(Reservation) == requested_bytes);
    std.debug.assert(region_start[6] + region_pages[6] == addressed_pages);
    std.debug.assert(@sizeOf([2][directory_size]u64) == 32_768);
    std.debug.assert(@sizeOf([1024]types.Receipt) <= 2_097_152);
    std.debug.assert(@sizeOf(types.Receipt) <= page_size);
    std.debug.assert(@offsetOf(Reservation, "pool") == 0);
    std.debug.assert(@offsetOf(Reservation, "directories") == 1_114_112);
    std.debug.assert(@offsetOf(Reservation, "history") == 1_146_880);
    std.debug.assert(@offsetOf(Reservation, "history_meta") == 5_341_184);
    std.debug.assert(@offsetOf(Reservation, "staged") == 5_357_568);
    std.debug.assert(@offsetOf(Reservation, "staged_meta") == 6_406_144);
    std.debug.assert(@offsetOf(Reservation, "envelopes") == 6_410_240);
    std.debug.assert(@offsetOf(Reservation, "command_meta") == 6_475_776);
    std.debug.assert(@offsetOf(Reservation, "logical_receipts") == 6_479_872);
    std.debug.assert(@offsetOf(Reservation, "scratch") == 6_479_872 + @sizeOf([1024]types.Receipt));
    std.debug.assert(@offsetOf(Reservation, "control") == 8_577_024);
}

pub const Claim = struct { settlement: ?ids.SettlementId = null, building: ?ids.BuildingId = null, is_hall: bool = false };
pub const Settlement = struct {
    id: ids.SettlementId,
    owner: types.CivilizationId,
    hall: ids.CellId,
    population: u8,
    food: u32,
    material: u32,
    incoming_food: u32,
    incoming_material: u32,
    housing: u8,
    hunger: u8,
    fed_streak: u8,
};
pub const Building = struct {
    id: ids.BuildingId,
    settlement: ids.SettlementId,
    kind: types.BuildingKind,
    anchor: ids.CellId,
    facing: ?types.Direction,
    footprint: types.Footprint,
    remaining_ticks: u8,
};
pub const Shipment = struct {
    id: ids.ShipmentId,
    source: ids.SettlementId,
    destination: ids.SettlementId,
    resource: types.Resource,
    quantity: u16,
    departure_tick: ids.Tick,
    arrival_tick: ids.Tick,
    route_len: u16,
};
pub const RouteCell = struct { index: u16, cell: ids.CellId };
pub const RecordKind = enum { claim, settlement, building, shipment, route_cell };
pub const Record = union(RecordKind) {
    claim: Claim,
    settlement: Settlement,
    building: Building,
    shipment: Shipment,
    route_cell: RouteCell,
};

const ClaimSlot = extern struct { settlement: u64, building: u64, flags: u8, padding: [15]u8 };
comptime {
    std.debug.assert(@sizeOf(ClaimSlot) == 32);
    std.debug.assert(@sizeOf(Settlement) <= 256);
    std.debug.assert(@sizeOf(Building) <= 128);
    std.debug.assert(@sizeOf(Shipment) <= 128);
    std.debug.assert(@sizeOf(RootMeta) <= 256);
    std.debug.assert(@sizeOf(u64) == 8);
}

pub const RootMeta = struct {
    revision: ids.Revision = @fromBackingInt(0),
    tick: ids.Tick = @fromBackingInt(0),
    last_sequence: ids.CommandSequence = @fromBackingInt(0),
    next_settlement: u64 = 1,
    next_building: u64 = 1,
    next_shipment: u64 = 1,
    settlements: u16 = 0,
    buildings: u16 = 0,
    shipments: u16 = 0,
    history_used: u16 = 0,
    chain_head: [32]u8 = std.mem.zeroes([32]u8),
    policy: ?types.SessionPolicy = null,
};

pub const Storage = struct {
    allocator: types.Allocator,
    reservation: *Reservation,
    config: types.Config,
    context: rules.AlgorithmContext = .{},
    active: u1 = 0,
    free_head: u16 = 1,
    live_pages: u16 = 0,
    peak_pages: u16 = 0,
    pages_copied: u64 = 0,
    work_units: u64 = 0,
    staged_count: u16 = 0,
    genesis: ?types.Receipt = null,

    pub fn init(allocator: types.Allocator, config: types.Config) types.Failure!Storage {
        const reservation = allocator.create(Reservation) catch return error.OutOfMemory;
        return .{ .allocator = allocator, .reservation = reservation, .config = config };
    }

    pub fn deinit(self: *Storage) void {
        self.allocator.destroy(self.reservation);
        self.* = undefined;
    }

    pub fn activeRoot(self: *const Storage) u1 {
        return self.active;
    }
    pub fn spareRoot(self: *const Storage) u1 {
        return self.active ^ 1;
    }
    pub fn logicalReceipt(self: *Storage, index: u16) *types.Receipt {
        const entries: *[1024]types.Receipt = @ptrCast(@alignCast(&self.reservation.logical_receipts));
        return &entries[index];
    }
    pub fn rootMeta(self: *const Storage, root: u1) RootMeta {
        const page_bytes = self.bytes(root, region_start[6]) catch unreachable;
        return readValue(RootMeta, page_bytes);
    }

    // The caller pays the work unit; creation writes the initial unshared page.
    pub fn writeRootMeta(self: *Storage, root: u1, value: RootMeta) void {
        const ref = self.reservation.directories[root][region_start[6]];
        std.debug.assert(self.page(ref).references == 1);
        const page_bytes = self.mutableBytes(root, region_start[6]) catch unreachable;
        writeValue(RootMeta, page_bytes, value);
    }

    pub fn terrainAt(self: *const Storage, root: u1, local: u16) types.Failure!types.Terrain {
        const count: u32 = @as(u32, 6) * self.config.side * self.config.side;
        if (local >= count) return error.InvalidCell;
        const page_bytes = try self.bytes(root, local / page_size);
        return @fromBackingInt(page_bytes[local % page_size]);
    }

    // Genesis calls this after spending a unit, before any page is shared.
    pub fn setTerrainAt(self: *Storage, root: u1, local: u16, terrain: types.Terrain) types.Failure!void {
        const count: u32 = @as(u32, 6) * self.config.side * self.config.side;
        if (local >= count) return error.InvalidCell;
        const page_index: u16 = local / page_size;
        const ref = self.reservation.directories[root][page_index];
        if (self.page(ref).references != 1) return error.InvariantViolation;
        const page_bytes = try self.mutableBytes(root, page_index);
        page_bytes[local % page_size] = @backingInt(terrain);
    }

    fn page(self: *Storage, ref: u64) *PageSlot {
        return &self.reservation.pool[@as(usize, @intCast(ref - 1))];
    }

    pub fn allocatePage(self: *Storage) types.Failure!u64 {
        const head = self.free_head;
        if (head == 0) return error.InvariantViolation;
        const slot = &self.reservation.pool[head - 1];
        self.free_head = slot.next_free;
        slot.references = 1;
        self.live_pages += 1;
        self.peak_pages = @max(self.peak_pages, self.live_pages);
        return head;
    }

    fn retain(self: *Storage, ref: u64) types.Failure!void {
        if (ref == 0 or ref > pool_size) return error.InvariantViolation;
        const slot = self.page(ref);
        if (slot.references == 0 or slot.references == std.math.maxInt(u16)) return error.InvariantViolation;
        slot.references += 1;
    }

    pub fn release(self: *Storage, ref: u64) types.Failure!void {
        if (ref == 0 or ref > pool_size) return error.InvariantViolation;
        const slot = self.page(ref);
        if (slot.references == 0) return error.InvariantViolation;
        slot.references -= 1;
        if (slot.references == 0) {
            slot.next_free = self.free_head;
            self.free_head = @intCast(ref);
            self.live_pages -= 1;
        }
    }

    fn bytes(self: *const Storage, root: u1, page_index: u16) types.Failure![]const u8 {
        if (page_index >= addressed_pages) return error.InvariantViolation;
        const ref = self.reservation.directories[root][page_index];
        if (ref == 0 or ref > pool_size) return error.InvariantViolation;
        return &self.reservation.pool[@as(usize, @intCast(ref - 1))].bytes;
    }

    fn mutableBytes(self: *Storage, root: u1, page_index: u16) types.Failure![]u8 {
        if (page_index >= addressed_pages) return error.InvariantViolation;
        const ref = self.reservation.directories[root][page_index];
        if (ref == 0 or ref > pool_size) return error.InvariantViolation;
        return &self.page(ref).bytes;
    }

    pub fn readRecord(self: *const Storage, root: u1, kind: RecordKind, index: u16) types.Failure!Record {
        const location = try locate(self.config, kind, index, null);
        const page_bytes = try self.bytes(root, location.page);
        const slot = page_bytes[location.offset..][0..location.stride];
        return switch (kind) {
            .claim => blk: {
                const raw = readValue(ClaimSlot, slot);
                break :blk .{ .claim = .{
                    .settlement = if (raw.settlement == 0) null else @fromBackingInt(raw.settlement),
                    .building = if (raw.building == 0) null else @fromBackingInt(raw.building),
                    .is_hall = raw.flags & 1 != 0,
                } };
            },
            .settlement => .{ .settlement = readValue(Settlement, slot) },
            .building => .{ .building = readValue(Building, slot) },
            .shipment => .{ .shipment = readValue(Shipment, slot) },
            .route_cell => error.InvalidCell,
        };
    }

    pub fn routeCell(self: *const Storage, root: u1, shipment_slot: u16, index: u16) types.Failure!ids.CellId {
        const location = try locate(self.config, .route_cell, shipment_slot, index);
        const slot = (try self.bytes(root, location.page))[location.offset..][0..8];
        const local = std.mem.readInt(u64, slot[0..8], .little);
        if (local >= @as(u64, 6) * self.config.side * self.config.side) return error.InvariantViolation;
        return .{ .planet = self.context.planet, .local = local };
    }
};

const Location = struct { page: u16, offset: usize, stride: usize };
fn locate(config: types.Config, kind: RecordKind, index: u16, route_index: ?u16) types.Failure!Location {
    const region: usize, const max_index: usize, const stride: usize = switch (kind) {
        .claim => .{ 1, @as(usize, 6) * config.side * config.side, 32 },
        .settlement => .{ 2, config.capacities.settlements, 256 },
        .building => .{ 3, config.capacities.buildings, 128 },
        .shipment => .{ 4, config.capacities.shipments, 128 },
        .route_cell => .{ 5, config.capacities.shipments, 768 },
    };
    if (index >= max_index) return error.NotFound;
    const byte_offset: usize = if (kind == .route_cell) blk: {
        const sub = route_index orelse return error.InvalidCell;
        if (sub >= 65) return error.InvalidCell;
        break :blk @as(usize, index) * stride + @as(usize, sub) * 8;
    } else @as(usize, index) * stride;
    return .{ .page = @intCast(region_start[region] + byte_offset / page_size), .offset = byte_offset % page_size, .stride = if (kind == .route_cell) 8 else stride };
}

fn readValue(comptime T: type, bytes: []const u8) T {
    var value: T = std.mem.zeroes(T);
    @memcpy(std.mem.asBytes(&value), bytes[0..@sizeOf(T)]);
    return value;
}

fn writeValue(comptime T: type, bytes: []u8, value: T) void {
    @memset(bytes, 0);
    @memcpy(bytes[0..@sizeOf(T)], std.mem.asBytes(&value));
}

pub const Candidate = struct {
    storage: *Storage,
    root: u1,
    prepare_index: u16 = 0,
    prepare_retained: bool = false,
    discard_index: u16 = 0,
    discard_released: bool = false,
    prepared: bool = false,
    pending_new_ref: u64 = 0,
    pending_old_ref: u64 = 0,
    published: bool = false,
    obsolete_root: u1 = 0,
    discard_finalized: bool = false,

    pub fn begin(storage: *Storage) Candidate {
        storage.staged_count = 0;
        return .{ .storage = storage, .root = storage.spareRoot() };
    }

    pub fn advancePrepare(self: *Candidate, budget: *types.WorkBudget) types.Failure!types.Step {
        if (self.prepared) return .done;
        if (budget.remaining > 4096) return error.InvalidBudget;
        while (self.prepare_index < directory_size) {
            if (budget.remaining == 0) return .pending;
            const ref = self.storage.reservation.directories[self.storage.active][self.prepare_index];
            if (!self.prepare_retained and ref != 0) {
                spend(self.storage, budget);
                try self.storage.retain(ref);
                self.prepare_retained = true;
                continue;
            }
            spend(self.storage, budget);
            self.storage.reservation.directories[self.root][self.prepare_index] = ref;
            self.prepare_index += 1;
            self.prepare_retained = false;
        }
        self.prepared = true;
        return .done;
    }

    pub fn advanceDiscard(self: *Candidate, budget: *types.WorkBudget) types.Failure!types.Step {
        if (self.discard_finalized) return .done;
        if (budget.remaining > 4096) return error.InvalidBudget;
        const discarded_root = if (self.published) self.obsolete_root else self.root;
        if (self.pending_new_ref != 0) {
            if (budget.remaining == 0) return .pending;
            spend(self.storage, budget);
            try self.storage.release(self.pending_new_ref);
            self.pending_new_ref = 0;
        }
        if (self.pending_old_ref != 0) {
            if (budget.remaining == 0) return .pending;
            spend(self.storage, budget);
            try self.storage.release(self.pending_old_ref);
            self.pending_old_ref = 0;
        }
        if (self.prepare_retained) {
            if (budget.remaining == 0) return .pending;
            spend(self.storage, budget);
            try self.storage.release(self.storage.reservation.directories[self.storage.active][self.prepare_index]);
            self.prepare_retained = false;
        }
        while (self.discard_index < self.prepare_index) {
            if (budget.remaining == 0) return .pending;
            const ref = self.storage.reservation.directories[discarded_root][self.discard_index];
            if (ref != 0 and !self.discard_released) {
                spend(self.storage, budget);
                try self.storage.release(ref);
                self.discard_released = true;
                continue;
            }
            spend(self.storage, budget);
            self.storage.reservation.directories[discarded_root][self.discard_index] = 0;
            self.discard_index += 1;
            self.discard_released = false;
        }
        if (budget.remaining == 0) return .pending;
        spend(self.storage, budget);
        self.storage.staged_count = 0;
        self.discard_finalized = true;
        return .done;
    }

    pub fn publish(self: *Candidate) void {
        std.debug.assert(self.prepared);
        std.debug.assert(self.pending_new_ref == 0 and self.pending_old_ref == 0);
        self.obsolete_root = self.storage.active;
        self.storage.active = self.root;
        self.published = true;
    }

    pub fn readRecord(self: *const Candidate, kind: RecordKind, index: u16) types.Failure!Record {
        if (!self.prepared) return error.NotReady;
        return self.storage.readRecord(self.root, kind, index);
    }

    pub fn meta(self: *const Candidate) RootMeta {
        std.debug.assert(self.prepared);
        return self.storage.rootMeta(self.root);
    }

    pub fn routeCell(self: *const Candidate, shipment_slot: u16, index: u16) types.Failure!ids.CellId {
        if (!self.prepared) return error.NotReady;
        return self.storage.routeCell(self.root, shipment_slot, index);
    }

    pub fn stage(self: *Candidate, receipt: types.Receipt, budget: *types.WorkBudget) types.Failure!types.Step {
        if (!self.prepared) return error.NotReady;
        if (budget.remaining > 4096) return error.InvalidBudget;
        if (budget.remaining == 0) return .pending;
        if (self.storage.staged_count >= 1024) return error.HistoryFull;
        spend(self.storage, budget);
        self.storage.logicalReceipt(self.storage.staged_count).* = receipt;
        self.storage.staged_count += 1;
        return .done;
    }
};

pub const RecordWriter = struct {
    index: u16,
    value: Record,
    step: enum { start, allocate, copy, swap, release, write, done } = .start,
    location: Location = undefined,
    old_ref: u64 = 0,
    new_ref: u64 = 0,

    pub fn init(index: u16, value: Record) RecordWriter {
        return .{ .index = index, .value = value };
    }

    pub fn advance(self: *RecordWriter, candidate: *Candidate, budget: *types.WorkBudget) types.Failure!types.Step {
        if (!candidate.prepared) return error.NotReady;
        if (budget.remaining > 4096) return error.InvalidBudget;
        while (self.step != .done) {
            if (budget.remaining == 0) return .pending;
            switch (self.step) {
                .start => {
                    const kind = std.meta.activeTag(self.value);
                    self.location = try locate(candidate.storage.config, kind, self.index, if (kind == .route_cell) self.value.route_cell.index else null);
                    if (kind == .route_cell) {
                        const cell = self.value.route_cell.cell;
                        if (cell.planet != candidate.storage.context.planet) return error.InvalidPlanet;
                        if (cell.local >= @as(u64, 6) * candidate.storage.config.side * candidate.storage.config.side) return error.InvalidCell;
                    }
                    self.old_ref = candidate.storage.reservation.directories[candidate.root][self.location.page];
                    if (self.old_ref == 0) return error.InvariantViolation;
                    spend(candidate.storage, budget);
                    self.step = if (candidate.storage.page(self.old_ref).references > 1) .allocate else .write;
                },
                .allocate => {
                    spend(candidate.storage, budget);
                    self.new_ref = try candidate.storage.allocatePage();
                    candidate.pending_new_ref = self.new_ref;
                    self.step = .copy;
                },
                .copy => {
                    spend(candidate.storage, budget);
                    @memcpy(&candidate.storage.page(self.new_ref).bytes, &candidate.storage.page(self.old_ref).bytes);
                    candidate.storage.pages_copied += 1;
                    self.step = .swap;
                },
                .swap => {
                    spend(candidate.storage, budget);
                    candidate.storage.reservation.directories[candidate.root][self.location.page] = self.new_ref;
                    candidate.pending_new_ref = 0;
                    candidate.pending_old_ref = self.old_ref;
                    self.step = .release;
                },
                .release => {
                    spend(candidate.storage, budget);
                    try candidate.storage.release(self.old_ref);
                    candidate.pending_old_ref = 0;
                    self.step = .write;
                },
                .write => {
                    spend(candidate.storage, budget);
                    const bytes = (try candidate.storage.mutableBytes(candidate.root, self.location.page))[self.location.offset..][0..self.location.stride];
                    switch (self.value) {
                        .claim => |value| writeValue(ClaimSlot, bytes, .{
                            .settlement = if (value.settlement) |id| @backingInt(id) else 0,
                            .building = if (value.building) |id| @backingInt(id) else 0,
                            .flags = if (value.is_hall) 1 else 0,
                            .padding = std.mem.zeroes([15]u8),
                        }),
                        .settlement => |value| writeValue(Settlement, bytes, value),
                        .building => |value| writeValue(Building, bytes, value),
                        .shipment => |value| writeValue(Shipment, bytes, value),
                        .route_cell => |value| {
                            if (value.cell.planet != candidate.storage.context.planet) return error.InvalidPlanet;
                            std.mem.writeInt(u64, bytes[0..8], value.cell.local, .little);
                        },
                    }
                    self.step = .done;
                },
                .done => unreachable,
            }
        }
        return .done;
    }
};

pub const MetaWriter = struct {
    value: RootMeta,
    step: enum { start, allocate, copy, swap, release, write, done } = .start,
    old_ref: u64 = 0,
    new_ref: u64 = 0,

    pub fn init(value: RootMeta) MetaWriter {
        return .{ .value = value };
    }

    pub fn advance(self: *MetaWriter, candidate: *Candidate, budget: *types.WorkBudget) types.Failure!types.Step {
        if (!candidate.prepared) return error.NotReady;
        if (budget.remaining > 4096) return error.InvalidBudget;
        while (self.step != .done) {
            if (budget.remaining == 0) return .pending;
            switch (self.step) {
                .start => {
                    self.old_ref = candidate.storage.reservation.directories[candidate.root][region_start[6]];
                    if (self.old_ref == 0) return error.InvariantViolation;
                    spend(candidate.storage, budget);
                    self.step = if (candidate.storage.page(self.old_ref).references > 1) .allocate else .write;
                },
                .allocate => {
                    spend(candidate.storage, budget);
                    self.new_ref = try candidate.storage.allocatePage();
                    candidate.pending_new_ref = self.new_ref;
                    self.step = .copy;
                },
                .copy => {
                    spend(candidate.storage, budget);
                    @memcpy(&candidate.storage.page(self.new_ref).bytes, &candidate.storage.page(self.old_ref).bytes);
                    candidate.storage.pages_copied += 1;
                    self.step = .swap;
                },
                .swap => {
                    spend(candidate.storage, budget);
                    candidate.storage.reservation.directories[candidate.root][region_start[6]] = self.new_ref;
                    candidate.pending_new_ref = 0;
                    candidate.pending_old_ref = self.old_ref;
                    self.step = .release;
                },
                .release => {
                    spend(candidate.storage, budget);
                    try candidate.storage.release(self.old_ref);
                    candidate.pending_old_ref = 0;
                    self.step = .write;
                },
                .write => {
                    spend(candidate.storage, budget);
                    candidate.storage.writeRootMeta(candidate.root, self.value);
                    self.step = .done;
                },
                .done => unreachable,
            }
        }
        return .done;
    }
};

pub fn spend(storage: *Storage, budget: *types.WorkBudget) void {
    std.debug.assert(budget.remaining > 0);
    budget.remaining -= 1;
    storage.work_units += 1;
}
