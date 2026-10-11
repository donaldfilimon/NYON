const std = @import("std");
const ids = @import("ids.zig");
const types = @import("types.zig");
const state = @import("state.zig");

pub const Cursor = struct {
    phase: types.Phase,
    target: ids.Tick,
    step: enum { start, scan, arrival_destination, arrival_meta, construction_house, drain, done } = .start,
    index: u16 = 0,
    building_index: u16 = 0,
    count: u16 = 0,
    settlement_count: u16 = 0,
    last_shipment_id: u64 = 0,
    best_slot: ?u16 = null,
    best_shipment: state.Shipment = undefined,
    current_building: state.Building = undefined,
    settlement: state.Settlement = undefined,
    free_workers: u8 = 0,
    writer_a: ?state.RecordWriter = null,
    writer_b: ?state.RecordWriter = null,
    meta_writer: ?state.MetaWriter = null,
    receipt_a: ?types.Receipt = null,
    receipt_b: ?types.Receipt = null,

    pub fn init(phase: types.Phase, target: ids.Tick) Cursor {
        return .{ .phase = phase, .target = target };
    }

    fn receipt(self: *const Cursor, candidate: *const state.Candidate, event: types.ReceiptEvent) types.Receipt {
        return .{ .header = .{ .tick = self.target, .ordinal = candidate.storage.staged_count, .provenance = .autonomous, .sequence = null }, .event = event };
    }

    fn drain(self: *Cursor, candidate: *state.Candidate, budget: *types.WorkBudget) types.Failure!void {
        if (self.writer_a) |*writer| {
            if (try writer.advance(candidate, budget) == .pending) return;
            self.writer_a = null;
        }
        if (self.writer_b) |*writer| {
            if (try writer.advance(candidate, budget) == .pending) return;
            self.writer_b = null;
        }
        if (self.meta_writer) |*writer| {
            if (try writer.advance(candidate, budget) == .pending) return;
            self.meta_writer = null;
        }
        if (self.receipt_a) |event_receipt| {
            if (try candidate.stage(event_receipt, budget) == .pending) return;
            self.receipt_a = null;
        }
        if (self.receipt_b) |*event_receipt| {
            event_receipt.header.ordinal = candidate.storage.staged_count;
            if (try candidate.stage(event_receipt.*, budget) == .pending) return;
            self.receipt_b = null;
        }
        self.step = .scan;
    }

    pub fn advance(self: *Cursor, candidate: *state.Candidate, budget: *types.WorkBudget) types.Failure!types.Step {
        if (!candidate.prepared) return error.NotReady;
        if (budget.remaining > 4096) return error.InvalidBudget;
        if (self.phase != .arrivals and self.phase != .construction and self.phase != .production and self.phase != .population) return error.InvariantViolation;
        while (self.step != .done) {
            if (budget.remaining == 0) return .pending;
            switch (self.step) {
                .start => {
                    state.spend(candidate.storage, budget);
                    const meta = candidate.meta();
                    self.count = switch (self.phase) {
                        .arrivals => candidate.storage.config.capacities.shipments,
                        .construction, .production => meta.buildings,
                        .population => meta.settlements,
                        else => unreachable,
                    };
                    if (self.phase == .production) self.settlement_count = meta.settlements;
                    self.step = .scan;
                },
                .scan => switch (self.phase) {
                    .arrivals => try self.scanArrival(candidate, budget),
                    .construction => try self.scanConstruction(candidate, budget),
                    .production => try self.scanProduction(candidate, budget),
                    .population => try self.scanPopulation(candidate, budget),
                    else => unreachable,
                },
                .arrival_destination => try self.arrivalDestination(candidate, budget),
                .arrival_meta => {
                    state.spend(candidate.storage, budget);
                    var meta = candidate.meta();
                    if (meta.shipments == 0) return error.InvariantViolation;
                    meta.shipments -= 1;
                    self.meta_writer = state.MetaWriter.init(meta);
                    self.receipt_a = self.receipt(candidate, .{ .arrived = .{
                        .shipment = self.best_shipment.id,
                        .source = self.best_shipment.source,
                        .destination = self.best_shipment.destination,
                        .resource = self.best_shipment.resource,
                        .quantity = self.best_shipment.quantity,
                        .destination_stock_after = if (self.best_shipment.resource == .food) self.settlement.food else self.settlement.material,
                        .destination_reservation_after = if (self.best_shipment.resource == .food) self.settlement.incoming_food else self.settlement.incoming_material,
                    } });
                    self.last_shipment_id = @backingInt(self.best_shipment.id);
                    self.index = 0;
                    self.best_slot = null;
                    self.step = .drain;
                },
                .construction_house => try self.completeHouse(candidate, budget),
                .drain => try self.drain(candidate, budget),
                .done => unreachable,
            }
        }
        return .done;
    }

    fn scanArrival(self: *Cursor, candidate: *state.Candidate, budget: *types.WorkBudget) types.Failure!void {
        if (self.index == self.count) {
            self.step = if (self.best_slot == null) .done else .arrival_destination;
            return;
        }
        state.spend(candidate.storage, budget);
        const shipment = (try candidate.readRecord(.shipment, self.index)).shipment;
        if (@backingInt(shipment.id) > self.last_shipment_id and @backingInt(shipment.arrival_tick) == @backingInt(self.target)) {
            if (self.best_slot == null or @backingInt(shipment.id) < @backingInt(self.best_shipment.id)) {
                self.best_slot = self.index;
                self.best_shipment = shipment;
            }
        }
        self.index += 1;
    }

    fn arrivalDestination(self: *Cursor, candidate: *state.Candidate, budget: *types.WorkBudget) types.Failure!void {
        state.spend(candidate.storage, budget);
        const raw = @backingInt(self.best_shipment.destination);
        if (raw == 0 or raw > candidate.storage.config.capacities.settlements) return error.InvariantViolation;
        const slot: u16 = @intCast(raw - 1);
        self.settlement = (try candidate.readRecord(.settlement, slot)).settlement;
        if (self.settlement.id != self.best_shipment.destination) return error.InvariantViolation;
        const amount: u32 = self.best_shipment.quantity;
        switch (self.best_shipment.resource) {
            .food => {
                if (self.settlement.incoming_food < amount or self.settlement.food + amount > 65_535) return error.InvariantViolation;
                self.settlement.incoming_food -= amount;
                self.settlement.food += amount;
            },
            .material => {
                if (self.settlement.incoming_material < amount or self.settlement.material + amount > 65_535) return error.InvariantViolation;
                self.settlement.incoming_material -= amount;
                self.settlement.material += amount;
            },
        }
        self.writer_a = state.RecordWriter.init(slot, .{ .settlement = self.settlement });
        self.writer_b = state.RecordWriter.init(self.best_slot.?, .{ .shipment = std.mem.zeroes(state.Shipment) });
        self.step = .arrival_meta;
    }

    fn scanConstruction(self: *Cursor, candidate: *state.Candidate, budget: *types.WorkBudget) types.Failure!void {
        if (self.index == self.count) {
            self.step = .done;
            return;
        }
        state.spend(candidate.storage, budget);
        const slot = self.index;
        self.index += 1;
        self.current_building = (try candidate.readRecord(.building, slot)).building;
        if (@backingInt(self.current_building.id) == 0 or self.current_building.remaining_ticks == 0) return;
        self.current_building.remaining_ticks -= 1;
        self.writer_a = state.RecordWriter.init(slot, .{ .building = self.current_building });
        if (self.current_building.remaining_ticks == 0) {
            if (self.current_building.kind == .house) {
                self.step = .construction_house;
                return;
            }
            self.receipt_a = self.receipt(candidate, .{ .completed = .{
                .building = self.current_building.id,
                .settlement = self.current_building.settlement,
                .kind = self.current_building.kind,
            } });
        }
        self.step = .drain;
    }

    fn completeHouse(self: *Cursor, candidate: *state.Candidate, budget: *types.WorkBudget) types.Failure!void {
        state.spend(candidate.storage, budget);
        const raw = @backingInt(self.current_building.settlement);
        if (raw == 0 or raw > candidate.storage.config.capacities.settlements) return error.InvariantViolation;
        const slot: u16 = @intCast(raw - 1);
        var value = (try candidate.readRecord(.settlement, slot)).settlement;
        if (value.id != self.current_building.settlement) return error.InvariantViolation;
        value.housing = @min(@as(u8, 32), value.housing + 4);
        self.writer_b = state.RecordWriter.init(slot, .{ .settlement = value });
        self.receipt_a = self.receipt(candidate, .{ .completed = .{
            .building = self.current_building.id,
            .settlement = self.current_building.settlement,
            .kind = self.current_building.kind,
        } });
        self.step = .drain;
    }

    fn scanProduction(self: *Cursor, candidate: *state.Candidate, budget: *types.WorkBudget) types.Failure!void {
        if (self.index == self.settlement_count) {
            self.step = .done;
            return;
        }
        if (self.building_index == 0) {
            state.spend(candidate.storage, budget);
            self.settlement = (try candidate.readRecord(.settlement, self.index)).settlement;
            self.free_workers = self.settlement.population;
            self.building_index = 1;
            return;
        }
        if (self.building_index > self.count) {
            self.index += 1;
            self.building_index = 0;
            return;
        }
        state.spend(candidate.storage, budget);
        const slot = self.building_index - 1;
        self.building_index += 1;
        const item = (try candidate.readRecord(.building, slot)).building;
        if (item.settlement != self.settlement.id or item.remaining_ticks != 0 or item.kind == .house) return;
        const resource: types.Resource = if (item.kind == .farm) .food else .material;
        const quantity: u32 = if (item.kind == .farm) 6 else 2;
        const stock: u32 = if (resource == .food) self.settlement.food else self.settlement.material;
        const incoming: u32 = if (resource == .food) self.settlement.incoming_food else self.settlement.incoming_material;
        if (self.free_workers < 2 or stock + incoming + quantity > 65_535) {
            self.receipt_a = self.receipt(candidate, .{ .idle = .{
                .building = item.id,
                .settlement = item.settlement,
                .reason = if (self.free_workers < 2) .workforce else .output_capacity,
                .workers_needed = 2,
                .workers_available = self.free_workers,
                .resource = resource,
                .quantity = quantity,
                .stock_before = stock,
                .incoming_reserved = incoming,
            } });
        } else {
            self.free_workers -= 2;
            if (resource == .food) self.settlement.food += quantity else self.settlement.material += quantity;
            self.writer_a = state.RecordWriter.init(self.index, .{ .settlement = self.settlement });
            self.receipt_a = self.receipt(candidate, .{ .produced = .{
                .building = item.id,
                .settlement = item.settlement,
                .workers = 2,
                .resource = resource,
                .quantity = quantity,
                .stock_after = stock + quantity,
            } });
        }
        self.step = .drain;
    }

    fn scanPopulation(self: *Cursor, candidate: *state.Candidate, budget: *types.WorkBudget) types.Failure!void {
        if (self.index == self.count) {
            self.step = .done;
            return;
        }
        state.spend(candidate.storage, budget);
        const slot = self.index;
        self.index += 1;
        var value = (try candidate.readRecord(.settlement, slot)).settlement;
        const before = value.population;
        const consumed: u8 = @intCast(@min(value.food, @as(u32, before)));
        const shortage = before - consumed;
        value.food -= consumed;
        self.receipt_a = self.receipt(candidate, .{ .supplied = .{
            .settlement = value.id,
            .needed = before,
            .consumed = consumed,
            .shortage = shortage,
            .food_after = value.food,
        } });
        var demographic: ?types.DemographyKind = null;
        if (before == 0) {
            value.hunger = 0;
            value.fed_streak = 0;
        } else if (shortage == 0) {
            value.hunger = 0;
            value.fed_streak += 1;
            if (value.fed_streak == 8) {
                value.fed_streak = 0;
                if (value.population < value.housing and value.population < 32) {
                    value.population += 1;
                    demographic = .growth;
                } else demographic = .housing_blocked;
            }
        } else {
            value.fed_streak = 0;
            value.hunger += 1;
            if (value.hunger == 3) {
                value.population -= 1;
                value.hunger = 0;
                demographic = .starvation;
            }
        }
        if (demographic) |kind| self.receipt_b = self.receipt(candidate, .{ .demography = .{
            .settlement = value.id,
            .kind = kind,
            .population_before = before,
            .population_after = value.population,
            .housing = value.housing,
            .hunger_after = value.hunger,
            .fed_streak_after = value.fed_streak,
        } });
        self.writer_a = state.RecordWriter.init(slot, .{ .settlement = value });
        self.step = .drain;
    }
};
