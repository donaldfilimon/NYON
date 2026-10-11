//! Fixed public-input acceptance stories. Observation is value-only; the host
//! owns paging/formatting and explicitly acknowledges before the next tick.
const std = @import("std");
const types = @import("types.zig");
const ids = @import("ids.zig");
const job = @import("job.zig");
const command = @import("command.zig");
const replay = @import("replay.zig");
const state = @import("state.zig");

pub const Story = enum { combined, creator, ruler_start, policy_replay };
pub const StoryResult = struct { ticks: u64, chain_head: [32]u8, assertions: u32 };
pub const StoryTick = struct { case_index: u8, summary: types.TickSummary, a: ?types.SettlementView, b: ?types.SettlementView, freight: types.FreightView };
pub const StoryProgress = union(enum) { pending, published: StoryTick, awaiting_observer, complete: StoryResult };
const Mode = enum { create, prepare, begin, drive, observe, waiting, replay, failed, complete };

pub const Runner = struct {
    story: Story,
    builders: [4]?job.Builder = @splat(null),
    authorities: [4]?job.Authority = @splat(null),
    count: u8,
    creating: u8 = 0,
    case_index: u8 = 0,
    mode: Mode = .create,
    after_observation: Mode = .drive,
    command_index: u16 = 0,
    batch_count: u16 = 0,
    lengths: [7]u16 = undefined,
    current_job: ?ids.JobId = null,
    replay_cursor: ?replay.Cursor = null,
    pending_summary: ?types.TickSummary = null,
    observation_index: u16 = 0,
    observation_a: ?types.SettlementView = null,
    observation_b: ?types.SettlementView = null,
    observation_step: enum { settlements, shipments, position, destination, finish } = .settlements,
    observation_freight: types.FreightView = .none,
    shipment_index: u16 = 0,
    observed_shipment: ?state.Shipment = null,
    arrival: ?struct { tick: ids.Tick, cell: ids.CellId } = null,
    attempts: enum { normal, old_sequence, cancel_sequence, finished } = .normal,
    expected_failure: bool = false,
    failure: ?types.Failure = null,
    published_ticks: u64 = 0,
    assertions: u32 = 0,

    pub fn init(allocator: types.Allocator, story: Story) types.Failure!Runner {
        var self = Runner{ .story = story, .count = if (story == .policy_replay) 4 else 1 };
        errdefer for (&self.builders) |*entry| {
            // Setup has not started polling or acquired shared page references.
            if (entry.*) |*builder| builder.storage.?.deinit();
        };
        for (0..self.count) |index| self.builders[index] = try job.beginCreate(allocator, .{ .seed = 2, .side = 4, .startup = if (story == .ruler_start or story == .policy_replay) .ruler_start else null });
        return self;
    }

    pub fn deinit(self: *Runner) void {
        std.debug.assert(self.mode == .complete or self.mode == .failed);
        for (&self.builders) |*entry| if (entry.*) |*builder| builder.deinit();
        for (&self.authorities) |*entry| if (entry.*) |*instance| instance.deinit();
        self.* = undefined;
    }

    fn authority(self: *Runner) *job.Authority {
        return &self.authorities[self.case_index].?;
    }

    pub fn receipts(self: *const Runner, revision: ids.Revision, cursor: types.ReceiptCursor, out: []types.ReceiptView) types.Failure!types.ReceiptPage {
        if (self.mode != .waiting) return error.NotReady;
        return self.authorities[self.case_index].?.receipts(revision, cursor, out);
    }

    pub fn acknowledge(self: *Runner, revision: ids.Revision) types.Failure!void {
        if (self.mode != .waiting) return error.NotReady;
        if (revision != self.pending_summary.?.revision) return error.StaleView;
        self.pending_summary = null;
        self.observation_index = 0;
        self.observation_a = null;
        self.observation_b = null;
        self.observation_step = .settlements;
        self.observation_freight = .none;
        self.shipment_index = 0;
        self.observed_shipment = null;
        self.mode = self.after_observation;
    }

    pub fn poll(self: *Runner, budget: *types.WorkBudget) types.Failure!StoryProgress {
        if (budget.remaining > 4096) return error.InvalidBudget;
        if (self.mode == .waiting) return .awaiting_observer;
        if (self.mode == .complete) return .{ .complete = self.result() };
        if (budget.remaining == 0) return .pending;
        if (self.mode == .failed) return self.drain(budget);
        return self.advance(budget) catch |failure| {
            self.failure = failure;
            self.mode = .failed;
            return self.drain(budget);
        };
    }

    fn drain(self: *Runner, budget: *types.WorkBudget) types.Failure!StoryProgress {
        for (&self.builders) |*entry| {
            if (entry.*) |*builder| {
                if (builder.storage == null) continue;
                job.cancelCreate(builder);
                switch (job.pollCreate(builder, budget)) {
                    .canceled, .failed, .idle => {
                        builder.deinit();
                        entry.* = null;
                    },
                    .ready => {
                        self.failure = error.InvariantViolation;
                        return error.InvariantViolation;
                    },
                    .pending => return .pending,
                    .invalid_request => |err| return err,
                }
            }
        }
        for (&self.authorities) |*entry| {
            if (entry.*) |*instance| {
                if (instance.lane) |lane| {
                    _ = instance.cancel(lane.id);
                    switch (instance.poll(lane.id, budget)) {
                        .idle => {},
                        else => return .pending,
                    }
                }
            }
        }
        return self.failure.?;
    }

    fn result(self: *const Runner) StoryResult {
        return .{ .ticks = self.published_ticks, .chain_head = self.authorities[self.case_index].?.summary().chain_head, .assertions = self.assertions };
    }

    fn advance(self: *Runner, budget: *types.WorkBudget) types.Failure!StoryProgress {
        while (budget.remaining > 0) {
            if (self.mode == .create) {
                const builder = &self.builders[self.creating].?;
                switch (job.pollCreate(builder, budget)) {
                    .ready => {
                        self.authorities[self.creating] = try job.takeAuthority(builder);
                        self.builders[self.creating] = null;
                        self.creating += 1;
                        if (self.creating == self.count) self.mode = .prepare;
                    },
                    .pending => return .pending,
                    .failed, .invalid_request => |failure| return failure,
                    else => return error.InvariantViolation,
                }
                continue;
            }
            const instance = self.authority();
            switch (self.mode) {
                .prepare => {
                    state.spend(&instance.storage, budget);
                    const tick = @backingInt(instance.summary().tick) + 1;
                    self.batch_count = if (self.attempts == .old_sequence or self.attempts == .cancel_sequence) 1 else self.inputCount(tick);
                    if (self.command_index == self.batch_count) {
                        self.mode = .begin;
                        continue;
                    }
                    const fixture = self.input(tick, self.command_index);
                    const encoded = try command.encodeEnvelope(fixture.header, fixture.command, &instance.storage.reservation.envelopes[self.command_index]);
                    self.lengths[self.command_index] = @intCast(encoded.bytes.len);
                    self.command_index += 1;
                },
                .begin => {
                    state.spend(&instance.storage, budget);
                    const tick = @backingInt(instance.summary().tick) + 1;
                    self.current_job = try instance.beginOwned(instance.summary().revision, self.policy(tick), self.lengths[0..self.batch_count]);
                    if (self.attempts == .cancel_sequence) _ = instance.cancel(self.current_job.?);
                    self.mode = .drive;
                },
                .drive => switch (instance.poll(self.current_job.?, budget)) {
                    .pending => return .pending,
                    .published => |summary| {
                        if (self.attempts != .normal) return error.InvariantViolation;
                        self.pending_summary = summary;
                        self.after_observation = .drive;
                        self.mode = .observe;
                    },
                    .failed => |err| {
                        if (self.attempts != .old_sequence or err != error.SequenceAlreadyPublished) return err;
                        self.expected_failure = true;
                    },
                    .invalid_request => |err| return err,
                    .canceled => {
                        if (self.attempts != .cancel_sequence) return error.InvariantViolation;
                        self.expected_failure = true;
                    },
                    .idle => {
                        self.current_job = null;
                        self.command_index = 0;
                        if (self.attempts == .old_sequence or self.attempts == .cancel_sequence) {
                            if (!self.expected_failure) return error.InvariantViolation;
                            self.expected_failure = false;
                            if (self.attempts == .old_sequence) {
                                self.attempts = .cancel_sequence;
                                self.mode = .prepare;
                            } else {
                                self.attempts = .finished;
                                try self.nextCase();
                            }
                        } else if (@backingInt(instance.summary().tick) == self.tickLimit()) {
                            if (self.story == .policy_replay) {
                                self.attempts = .old_sequence;
                                self.mode = .prepare;
                            } else self.mode = .complete;
                        } else self.mode = .prepare;
                    },
                },
                .observe => {
                    state.spend(&instance.storage, budget);
                    switch (self.observation_step) {
                        .settlements => {
                            if (self.observation_index == instance.storage.config.capacities.settlements) return error.NotFound;
                            const record = (try instance.storage.readRecord(instance.storage.active, .settlement, self.observation_index)).settlement;
                            self.observation_index += 1;
                            if (record.id == @as(ids.SettlementId, @fromBackingInt(1))) self.observation_a = observedSettlement(self.pending_summary.?.revision, record);
                            if (record.id == @as(ids.SettlementId, @fromBackingInt(2))) self.observation_b = observedSettlement(self.pending_summary.?.revision, record);
                            const needs_b = self.story == .combined or self.story == .creator;
                            if (self.observation_a != null and (!needs_b or self.observation_b != null)) self.observation_step = .shipments;
                        },
                        .shipments => {
                            if (self.shipment_index == instance.storage.config.capacities.shipments) {
                                if (self.arrival) |arrival| {
                                    if (arrival.tick == self.pending_summary.?.tick) self.observation_freight = .{ .delivered = arrival.cell };
                                }
                                self.arrival = null;
                                self.observation_step = .finish;
                                continue;
                            }
                            const record = (try instance.storage.readRecord(instance.storage.active, .shipment, self.shipment_index)).shipment;
                            if (@backingInt(record.id) == 0) {
                                self.shipment_index += 1;
                                continue;
                            }
                            self.observed_shipment = record;
                            self.observation_step = .position;
                        },
                        .position => {
                            const record = self.observed_shipment.?;
                            const elapsed = @backingInt(self.pending_summary.?.tick) - @backingInt(record.departure_tick);
                            const index: u16 = @intCast(@min(elapsed, record.route_len - 1));
                            const position = try instance.storage.routeCell(instance.storage.active, self.shipment_index, index);
                            self.observation_freight = .{ .transit = position };
                            self.observation_step = .destination;
                        },
                        .destination => {
                            const record = self.observed_shipment.?;
                            const position = try instance.storage.routeCell(instance.storage.active, self.shipment_index, record.route_len - 1);
                            self.arrival = .{ .tick = record.arrival_tick, .cell = position };
                            self.observation_step = .finish;
                        },
                        .finish => {
                            try self.verify(self.observation_a.?, self.observation_b);
                            self.published_ticks += 1;
                            self.assertions += 1;
                            self.mode = .waiting;
                            return .{ .published = .{ .case_index = self.case_index, .summary = self.pending_summary.?, .a = self.observation_a, .b = self.observation_b, .freight = self.observation_freight } };
                        },
                    }
                },
                .replay => {
                    const previous = instance.summary().revision;
                    const done = try self.replay_cursor.?.advance(instance, budget);
                    if (instance.summary().revision != previous) {
                        self.pending_summary = instance.summary();
                        self.after_observation = .replay;
                        self.mode = .observe;
                    } else if (done == .done) try self.nextCase() else return .pending;
                },
                .complete => return .{ .complete = self.result() },
                .waiting => return .awaiting_observer,
                .create, .failed => unreachable,
            }
        }
        return .pending;
    }

    fn nextCase(self: *Runner) types.Failure!void {
        if (self.case_index == 3) {
            self.mode = .complete;
            return;
        }
        self.case_index += 1;
        self.arrival = null;
        self.attempts = .normal;
        if (self.case_index < 2) self.mode = .prepare else {
            self.replay_cursor = replay.Cursor.init(&self.authorities[self.case_index - 2].?);
            self.mode = .replay;
        }
    }

    fn tickLimit(self: *const Runner) u64 {
        return switch (self.story) {
            .combined => 8,
            .creator => 4,
            .ruler_start, .policy_replay => 3,
        };
    }
    fn policy(self: *const Runner, tick: u64) types.SessionPolicy {
        if (self.attempts == .old_sequence or self.attempts == .cancel_sequence) return if (self.case_index == 0) .{ .role = .ruler, .civilization = ids.civilization_id } else .{ .role = .creator, .civilization = null };
        if (self.story == .combined) return .{ .role = .combined, .civilization = ids.civilization_id };
        if (self.story == .creator or tick == 1 or (self.story == .policy_replay and self.case_index % 2 == 0)) return .{ .role = .creator, .civilization = null };
        return .{ .role = .ruler, .civilization = ids.civilization_id };
    }
    fn inputCount(self: *const Runner, tick: u64) u16 {
        if (tick == 1) return switch (self.story) {
            .combined => 7,
            .creator => 6,
            .ruler_start, .policy_replay => 1,
        };
        if (tick == 2 and (self.story == .ruler_start or self.story == .policy_replay)) return 1;
        return 0;
    }

    fn input(self: *const Runner, tick: u64, index: u16) Input {
        if (self.attempts != .normal) return creator(if (self.attempts == .old_sequence) 2 else 3, grant(true));
        if (tick == 2) return if (self.story == .ruler_start) ruler(2, .{ .build = build(.farm, 71, .east) }) else creator(2, grant(true));
        if (index == 0) return creator(1, .{ .author_settlement = .{ .owner = ids.civilization_id, .anchor = cell(70), .population = 4, .food = if (self.story == .creator) 0 else 12, .material = if (self.story == .creator) 0 else 20, .economic_bypass = true } });
        if (index == 1) return creator(2, .{ .author_settlement = .{ .owner = ids.civilization_id, .anchor = cell(5), .population = 2, .food = 6, .material = 0, .economic_bypass = true } });
        if (self.story == .creator) return switch (index) {
            2 => creator(3, .{ .author_build = authoredBuild(.farm, 71, .east) }),
            3 => creator(4, .{ .author_build = authoredBuild(.quarry, 74, null) }),
            4 => creator(5, .{ .author_build = authoredBuild(.house, 69, null) }),
            5 => creator(6, .{ .author_ship = .{ .source = @fromBackingInt(1), .destination = @fromBackingInt(2), .resource = .food, .amount = 4, .path = &route, .cargo_source = .mint, .economic_bypass = true } }),
            else => unreachable,
        };
        return switch (index) {
            2 => ruler(3, .{ .build = build(.farm, 71, .east) }),
            3 => ruler(4, .{ .build = build(.quarry, 74, null) }),
            4 => ruler(5, .{ .ship = .{ .source = @fromBackingInt(1), .destination = @fromBackingInt(2), .resource = .food, .amount = 4, .path = &route } }),
            5 => ruler(6, .{ .build = build(.house, 71, null) }),
            6 => creator(7, .{ .grant = .{ .settlement = @fromBackingInt(2), .food = 1, .material = 0, .population = 0, .economic_bypass = false } }),
            else => unreachable,
        };
    }

    fn verify(self: *const Runner, a: types.SettlementView, b: ?types.SettlementView) types.Failure!void {
        const tick = @backingInt(self.pending_summary.?.tick);
        if (tick == 0 or tick > self.tickLimit()) return error.InvariantViolation;
        const row: usize = @intCast(tick - 1);
        if (a.population != 4) return error.InvariantViolation;
        switch (self.story) {
            .combined => {
                const tuples = [_][5]u32{ .{ 4, 10, 2, 4, 0 }, .{ 6, 12, 2, 2, 0 }, .{ 8, 14, 2, 0, 0 }, .{ 10, 16, 2, 2, 0 }, .{ 12, 18, 2, 0, 0 }, .{ 14, 20, 2, 0, 1 }, .{ 16, 22, 2, 0, 2 }, .{ 18, 24, 1, 0, 0 } };
                if (!std.meta.eql(tuples[row], [5]u32{ a.food, a.material, b.?.population, b.?.food, b.?.hunger })) return error.InvariantViolation;
            },
            .creator => {
                const tuples = [_][3]u32{ .{ 2, 2, 4 }, .{ 4, 4, 2 }, .{ 6, 6, 0 }, .{ 8, 8, 2 } };
                if (a.housing != 8 or !std.meta.eql(tuples[row], [3]u32{ a.food, a.material, b.?.food })) return error.InvariantViolation;
            },
            .ruler_start => {
                const tuples = [_][2]u32{ .{ 8, 20 }, .{ 4, 14 }, .{ 6, 14 } };
                if (a.fed_streak != tick or !std.meta.eql(tuples[row], [2]u32{ a.food, a.material })) return error.InvariantViolation;
            },
            .policy_replay => {
                const food = if (self.case_index % 2 == 0) [3]u32{ 8, 5, 1 } else [3]u32{ 8, 4, 0 };
                if (a.food != food[row]) return error.InvariantViolation;
            },
        }
    }
};

fn observedSettlement(revision: ids.Revision, record: state.Settlement) types.SettlementView {
    return .{
        .revision = revision,
        .id = record.id,
        .owner = record.owner,
        .hall = record.hall,
        .population = record.population,
        .food = record.food,
        .material = record.material,
        .incoming_food = record.incoming_food,
        .incoming_material = record.incoming_material,
        .housing = record.housing,
        .hunger = record.hunger,
        .fed_streak = record.fed_streak,
    };
}

const Input = struct { header: types.CommandHeader, command: types.Command };
fn creator(sequence: u64, value: types.Command) Input {
    return .{ .header = .{ .sequence = @fromBackingInt(sequence), .provenance = .creator, .actor = null }, .command = value };
}
fn ruler(sequence: u64, value: types.Command) Input {
    return .{ .header = .{ .sequence = @fromBackingInt(sequence), .provenance = .ruler, .actor = ids.civilization_id }, .command = value };
}
fn cell(local: u64) ids.CellId {
    return .{ .planet = ids.planet_id, .local = local };
}
const route = [_]ids.CellId{ cell(70), cell(71), cell(4), cell(5) };
fn grant(bypass: bool) types.Command {
    return .{ .grant = .{ .settlement = @fromBackingInt(1), .food = 1, .material = 0, .population = 0, .economic_bypass = bypass } };
}
fn build(kind: types.BuildingKind, local: u64, facing: ?types.Direction) @FieldType(types.Command, "build") {
    return .{ .settlement = @fromBackingInt(1), .kind = kind, .anchor = cell(local), .facing = facing };
}
fn authoredBuild(kind: types.BuildingKind, local: u64, facing: ?types.Direction) @FieldType(types.Command, "author_build") {
    return .{ .settlement = @fromBackingInt(1), .kind = kind, .anchor = cell(local), .facing = facing, .economic_bypass = true };
}

pub fn run(allocator: types.Allocator, story: Story, units: u16) types.Failure!StoryResult {
    if (units == 0 or units > 4096) return error.InvalidBudget;
    var runner = try Runner.init(allocator, story);
    defer runner.deinit();
    while (true) {
        var budget = types.WorkBudget{ .remaining = units };
        switch (try runner.poll(&budget)) {
            .pending => {},
            .published => |value| try runner.acknowledge(value.summary.revision),
            .awaiting_observer => return error.InvariantViolation,
            .complete => |result| return result,
        }
    }
}
