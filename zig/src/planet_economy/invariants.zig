//! Final semantic barrier over COW-dirty pages. Every record, reference lookup,
//! route comparison and topology edge has its own prepaid cursor step. This
//! protects candidate publication, not arbitrary corruption of authority memory.
const std = @import("std");
const ids = @import("ids.zig");
const types = @import("types.zig");
const state = @import("state.zig");
const rules = @import("rules.zig");
const surface = @import("../surface.zig");
const Phase = enum { metadata, census_settlements, census_buildings, census_shipments, page, record, absent, settlement, hall, houses, reservations, claim_settlement, claim_building, building_geometry, building_settlement, footprint, connected_edge, connected_claim, shipment_source, shipment_destination, route, route_duplicate, route_edge, route_claim, route_owner, done };

pub const Cursor = struct {
    phase: Phase = .metadata,
    page: u16 = state.region_start[1],
    index: u16 = 0,
    end: u16 = 0,
    kind: state.RecordKind = .claim,
    scan: u16 = 0,
    meta: state.RootMeta = undefined,
    present: u16 = 0,
    route_owner: ids.SettlementId = undefined,
    record: state.Record = undefined,
    settlement: state.Settlement = undefined,
    after_settlement: Phase = .record,
    housing: u16 = 4,
    buildings: u16 = 0,
    incoming_food: u32 = 0,
    incoming_material: u32 = 0,
    part: u16 = 0,
    other: u16 = 0,
    direction: u8 = 0,
    connected: bool = false,
    adjacent: ids.CellId = undefined,
    current: ids.CellId = undefined,
    previous: ids.CellId = undefined,
    source_hall: ids.CellId = undefined,
    source_owner: types.CivilizationId = undefined,
    destination_hall: ids.CellId = undefined,

    pub fn advance(self: *Cursor, candidate: *state.Candidate, budget: *types.WorkBudget) types.Failure!types.Step {
        return self.advanceContext(candidate, budget, .{});
    }

    pub fn advanceContext(self: *Cursor, candidate: *state.Candidate, budget: *types.WorkBudget, context: rules.AlgorithmContext) types.Failure!types.Step {
        if (budget.remaining > 4096) return error.InvalidBudget;
        const config = candidate.storage.config;
        const topology: surface.Topology = .{ .side = config.side };
        while (self.phase != .done) {
            if (budget.remaining == 0) return .pending;
            state.spend(candidate.storage, budget);
            switch (self.phase) {
                .metadata => {
                    const meta = candidate.meta();
                    if (meta.settlements > config.capacities.settlements or meta.buildings > config.capacities.buildings or meta.shipments > config.capacities.shipments or meta.history_used > config.capacities.history_receipts or @backingInt(meta.tick) > 1_000_000 or meta.next_settlement == 0 or meta.next_building == 0 or meta.next_shipment == 0) return error.InvariantViolation;
                    self.meta = meta;
                    self.phase = .census_settlements;
                },
                .census_settlements, .census_buildings, .census_shipments => {
                    const kind: state.RecordKind, const limit: u16, const count: u16, const next: Phase = switch (self.phase) {
                        .census_settlements => .{ .settlement, config.capacities.settlements, self.meta.settlements, .census_buildings },
                        .census_buildings => .{ .building, config.capacities.buildings, self.meta.buildings, .census_shipments },
                        .census_shipments => .{ .shipment, config.capacities.shipments, self.meta.shipments, .page },
                        else => unreachable,
                    };
                    if (self.scan == limit) {
                        if (self.present != count) return error.InvariantViolation;
                        self.scan = 0;
                        self.present = 0;
                        self.phase = next;
                        continue;
                    }
                    const record = try candidate.readRecord(kind, self.scan);
                    const id = switch (record) {
                        .settlement => |r| @backingInt(r.id),
                        .building => |r| @backingInt(r.id),
                        .shipment => |r| @backingInt(r.id),
                        else => unreachable,
                    };
                    if (id != 0) self.present += 1;
                    self.scan += 1;
                },
                .page => {
                    if (self.page == state.region_start[6]) {
                        self.phase = .done;
                        continue;
                    }
                    const page = self.page;
                    self.page += 1;
                    if (candidate.storage.reservation.directories[candidate.root][page] == candidate.storage.reservation.directories[candidate.storage.active][page]) continue;
                    const start: u16, const stride: u16, const limit: u16 = if (page < state.region_start[2]) blk: {
                        self.kind = .claim;
                        break :blk .{ state.region_start[1], 8, 6 * config.side * config.side };
                    } else if (page < state.region_start[3]) blk: {
                        self.kind = .settlement;
                        break :blk .{ state.region_start[2], 1, config.capacities.settlements };
                    } else if (page < state.region_start[4]) blk: {
                        self.kind = .building;
                        break :blk .{ state.region_start[3], 2, config.capacities.buildings };
                    } else if (page < state.region_start[5]) blk: {
                        self.kind = .shipment;
                        break :blk .{ state.region_start[4], 2, config.capacities.shipments };
                    } else {
                        // Any changed route page requires checking its header and
                        // whole used route; unused cells have no semantic meaning.
                        self.kind = .shipment;
                        self.index = (page - state.region_start[5]) / 3;
                        self.end = @min(self.index + 1, config.capacities.shipments);
                        self.phase = .record;
                        continue;
                    };
                    self.index = (page - start) * stride;
                    self.end = @min(self.index + stride, limit);
                    self.phase = .record;
                },
                .record => {
                    if (self.index >= self.end) {
                        self.phase = .page;
                        continue;
                    }
                    self.record = try candidate.readRecord(self.kind, self.index);
                    self.index += 1;
                    self.scan = 0;
                    switch (self.record) {
                        .settlement => |record| {
                            if (@backingInt(record.id) == 0) {
                                self.phase = .absent;
                                continue;
                            }
                            self.startSettlement(record, .record);
                        },
                        .claim => |record| {
                            if (record.settlement == null) {
                                if (record.building != null or record.is_hall) return error.InvariantViolation;
                                self.phase = .absent;
                            } else {
                                if (record.is_hall == (record.building != null)) return error.InvariantViolation;
                                self.phase = .claim_settlement;
                            }
                        },
                        .building => |record| {
                            if (@backingInt(record.id) == 0) {
                                self.phase = .absent;
                                continue;
                            }
                            if (@backingInt(record.id) >= self.meta.next_building or record.remaining_ticks > 1) return error.InvariantViolation;
                            self.phase = .building_geometry;
                        },
                        .shipment => |record| {
                            if (@backingInt(record.id) == 0) continue;
                            const departure = @backingInt(record.departure_tick);
                            const arrival = @backingInt(record.arrival_tick);
                            const tick = @backingInt(self.meta.tick);
                            if (@backingInt(record.id) >= self.meta.next_shipment or record.source == record.destination or record.quantity == 0 or record.quantity > 256 or record.route_len < 2 or record.route_len > 65 or departure > tick or arrival <= tick or arrival > 1_000_000 or arrival - departure != record.route_len - 1) return error.InvariantViolation;
                            self.phase = .shipment_source;
                        },
                        .route_cell => unreachable,
                    }
                },
                .absent => {
                    // This identity has no claim/settlement/building deletion.
                    // A separate paid read distinguishes untouched empty slots
                    // sharing a dirty page from an illegally removed record.
                    const prior = try candidate.storage.readRecord(candidate.storage.active, self.kind, self.index - 1);
                    const present = switch (prior) {
                        .claim => |r| r.settlement != null or r.building != null or r.is_hall,
                        .settlement => |r| @backingInt(r.id) != 0,
                        .building => |r| @backingInt(r.id) != 0,
                        else => unreachable,
                    };
                    if (present) return error.InvariantViolation;
                    self.phase = .record;
                },
                .building_geometry => {
                    const record = self.record.building;
                    const expected = rules.footprintContext(context, topology, record.kind, record.anchor, record.facing) catch return error.InvariantViolation;
                    if (record.footprint.len != expected.len) return error.InvariantViolation;
                    // At most two inline cells; no record reads or edge walk.
                    for (expected.cells[0..expected.len], record.footprint.cells[0..expected.len]) |a, b| if (!equalCell(a, b)) return error.InvariantViolation;
                    const terrain = rules.terrain(config.seed, record.anchor);
                    if ((record.kind == .farm and terrain != .fertile) or (record.kind == .quarry and terrain != .rocky)) return error.InvariantViolation;
                    self.phase = .building_settlement;
                },
                .settlement => {
                    const record = self.settlement;
                    if (@backingInt(record.id) == 0 or @backingInt(record.id) >= self.meta.next_settlement or !context.validOwner(record.owner) or !validCell(context, config, record.hall) or record.population > 32 or record.population > record.housing or record.housing < 4 or record.housing > 32 or record.housing % 4 != 0 or record.hunger >= 3 or record.fed_streak >= 8 or (record.hunger != 0 and record.fed_streak != 0) or (record.population == 0 and (record.hunger != 0 or record.fed_streak != 0)) or @as(u64, record.food) + record.incoming_food > 65_535 or @as(u64, record.material) + record.incoming_material > 65_535) return error.InvariantViolation;
                    self.phase = .hall;
                },
                .hall => {
                    const claim = (try candidate.readRecord(.claim, @intCast(self.settlement.hall.local))).claim;
                    if (claim.settlement != self.settlement.id or !claim.is_hall or claim.building != null) return error.InvariantViolation;
                    self.phase = .houses;
                },
                .houses => {
                    if (self.scan == config.capacities.buildings) {
                        if (self.buildings > 32 or self.settlement.housing != @min(@as(u16, 32), self.housing)) return error.InvariantViolation;
                        self.scan = 0;
                        self.phase = .reservations;
                        continue;
                    }
                    const record = (try candidate.readRecord(.building, self.scan)).building;
                    self.scan += 1;
                    if (@backingInt(record.id) != 0 and record.settlement == self.settlement.id) {
                        self.buildings += 1;
                        if (record.kind == .house and record.remaining_ticks == 0) self.housing += 4;
                    }
                },
                .reservations => {
                    if (self.scan == config.capacities.shipments) {
                        if (self.incoming_food != self.settlement.incoming_food or self.incoming_material != self.settlement.incoming_material) return error.InvariantViolation;
                        self.scan = 0;
                        self.phase = self.after_settlement;
                        continue;
                    }
                    const record = (try candidate.readRecord(.shipment, self.scan)).shipment;
                    self.scan += 1;
                    if (@backingInt(record.id) != 0 and record.destination == self.settlement.id) {
                        if (record.resource == .food) self.incoming_food += record.quantity else self.incoming_material += record.quantity;
                    }
                },
                .claim_settlement, .building_settlement, .shipment_source, .shipment_destination => {
                    if (self.scan == config.capacities.settlements) return error.InvariantViolation;
                    const found = (try candidate.readRecord(.settlement, self.scan)).settlement;
                    self.scan += 1;
                    const wanted = switch (self.phase) {
                        .claim_settlement => self.record.claim.settlement.?,
                        .building_settlement => self.record.building.settlement,
                        .shipment_source => self.record.shipment.source,
                        .shipment_destination => self.record.shipment.destination,
                        else => unreachable,
                    };
                    if (@backingInt(wanted) == 0) return error.InvariantViolation;
                    if (found.id != wanted) continue;
                    const next: Phase = switch (self.phase) {
                        .claim_settlement => blk: {
                            if (self.record.claim.is_hall and (!validCell(context, config, found.hall) or found.hall.local != self.index - 1)) return error.InvariantViolation;
                            break :blk if (self.record.claim.building != null) .claim_building else .record;
                        },
                        .building_settlement => blk: {
                            self.part = 0;
                            self.direction = 0;
                            self.connected = false;
                            break :blk .footprint;
                        },
                        .shipment_source => blk: {
                            self.source_hall = found.hall;
                            self.source_owner = found.owner;
                            break :blk .shipment_destination;
                        },
                        .shipment_destination => blk: {
                            if (found.owner != self.source_owner) return error.InvariantViolation;
                            self.destination_hall = found.hall;
                            self.part = 0;
                            break :blk .route;
                        },
                        else => unreachable,
                    };
                    if (self.phase == .claim_settlement) {
                        // Claims do not change stock/housing. Resolve the owner
                        // here; its dirty settlement is checked separately.
                        if (!context.validOwner(found.owner)) return error.InvariantViolation;
                        self.scan = 0;
                        self.phase = next;
                    } else self.startSettlement(found, next);
                },
                .claim_building => {
                    if (self.scan == config.capacities.buildings) return error.InvariantViolation;
                    const found = (try candidate.readRecord(.building, self.scan)).building;
                    self.scan += 1;
                    if (found.id != self.record.claim.building.?) continue;
                    if (found.settlement != self.record.claim.settlement.? or found.footprint.len < 1 or found.footprint.len > 2) return error.InvariantViolation;
                    var contains = false;
                    for (found.footprint.cells[0..found.footprint.len]) |cell| if (cell.planet == context.planet and cell.local == self.index - 1) {
                        contains = true;
                    };
                    if (!contains) return error.InvariantViolation;
                    self.phase = .record;
                },
                .footprint => {
                    const building = self.record.building;
                    if (self.part == building.footprint.len) {
                        if (!self.connected) return error.InvariantViolation;
                        self.phase = .record;
                        continue;
                    }
                    const cell = building.footprint.cells[self.part];
                    const claim = (try candidate.readRecord(.claim, @intCast(cell.local))).claim;
                    if (claim.settlement != building.settlement or claim.building != building.id or claim.is_hall) return error.InvariantViolation;
                    self.phase = .connected_edge;
                },
                .connected_edge => {
                    const cell = self.record.building.footprint.cells[self.part];
                    self.adjacent = try neighbor(topology, cell, self.direction);
                    self.phase = .connected_claim;
                },
                .connected_claim => {
                    const building = self.record.building;
                    const claim = (try candidate.readRecord(.claim, @intCast(self.adjacent.local))).claim;
                    if (claim.settlement == building.settlement and claim.building != building.id) self.connected = true;
                    self.direction += 1;
                    if (self.direction == 4) {
                        self.direction = 0;
                        self.part += 1;
                        self.phase = .footprint;
                    } else self.phase = .connected_edge;
                },
                .route => {
                    const shipment = self.record.shipment;
                    if (self.part == shipment.route_len) {
                        self.phase = .record;
                        continue;
                    }
                    self.current = try candidate.routeCell(self.index - 1, self.part);
                    if (!validCell(context, config, self.current) or (self.part == 0 and !equalCell(self.current, self.source_hall)) or (self.part == shipment.route_len - 1 and !equalCell(self.current, self.destination_hall))) return error.InvariantViolation;
                    self.other = 0;
                    self.direction = 0;
                    self.phase = .route_duplicate;
                },
                .route_duplicate => {
                    if (self.other == self.part) {
                        self.phase = if (self.part == 0) .route_claim else .route_edge;
                        continue;
                    }
                    const prior = try candidate.routeCell(self.index - 1, self.other);
                    if (equalCell(prior, self.current)) return error.InvariantViolation;
                    self.other += 1;
                },
                .route_edge => {
                    const adjacent = try neighbor(topology, self.previous, self.direction);
                    if (equalCell(adjacent, self.current)) {
                        self.phase = .route_claim;
                        continue;
                    }
                    self.direction += 1;
                    if (self.direction == 4) return error.InvariantViolation;
                },
                .route_claim => {
                    // Claimed path cells must resolve to the shipment owner.
                    const claim = (try candidate.readRecord(.claim, @intCast(self.current.local))).claim;
                    if (claim.settlement) |settlement_id| {
                        if (@backingInt(settlement_id) == 0 or @backingInt(settlement_id) >= self.meta.next_settlement) return error.InvariantViolation;
                        self.route_owner = settlement_id;
                        self.scan = 0;
                        self.phase = .route_owner;
                        continue;
                    }
                    self.previous = self.current;
                    self.part += 1;
                    self.phase = .route;
                },
                .route_owner => {
                    if (self.scan == config.capacities.settlements) return error.InvariantViolation;
                    const found = (try candidate.readRecord(.settlement, self.scan)).settlement;
                    self.scan += 1;
                    if (found.id != self.route_owner) continue;
                    if (!context.validOwner(found.owner) or found.owner != self.source_owner) return error.InvariantViolation;
                    self.previous = self.current;
                    self.part += 1;
                    self.phase = .route;
                },
                .done => unreachable,
            }
        }
        return .done;
    }

    fn startSettlement(self: *Cursor, record: state.Settlement, next: Phase) void {
        self.settlement = record;
        self.after_settlement = next;
        self.scan = 0;
        self.housing = 4;
        self.buildings = 0;
        self.incoming_food = 0;
        self.incoming_material = 0;
        self.phase = .settlement;
    }
};

fn validCell(context: rules.AlgorithmContext, config: types.Config, cell: ids.CellId) bool {
    return cell.planet == context.planet and cell.local < @as(u64, 6) * config.side * config.side;
}
fn equalCell(a: ids.CellId, b: ids.CellId) bool {
    return a.planet == b.planet and a.local == b.local;
}
fn neighbor(topology: surface.Topology, cell: ids.CellId, direction: u8) types.Failure!ids.CellId {
    const directions = [_]surface.Direction{ .east, .west, .north, .south };
    const at = topology.fromId(cell.local) catch return error.InvariantViolation;
    const adjacent = topology.neighbor(at, directions[direction]) catch return error.InvariantViolation;
    return .{ .planet = cell.planet, .local = topology.cellId(adjacent) catch return error.InvariantViolation };
}
