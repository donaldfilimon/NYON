const std = @import("std");
const m = @import("model.zig");
const generator = @import("generator.zig");
const economy = @import("economy.zig");
const navigation = @import("navigation.zig");
const scenario = @import("scenario.zig");

pub const CommandRejection = error{
    WrongTick,
    DuplicateSequence,
    CampaignFinished,
    InvalidWorld,
    SameSourceAndTarget,
    SourceNotOwnedByIssuer,
    InsufficientEnergy,
    FieldAtLimit,
};
pub const CommandResult = union(enum) {
    accepted: m.CommandSequence,
    rejected: struct { sequence: m.CommandSequence, reason: CommandRejection },
};
pub const CommandPreview = struct {
    command: m.GameCommand,
    source: m.WorldId,
    destination: ?m.WorldId,
    launch_strength: ?m.Strength,
};
pub const GameEvent = union(enum) {
    fleet_launched: m.FleetId,
    world_captured: struct { world: m.WorldId, owner: ?m.Faction },
    hazard_started: m.ActiveHazard,
    hazard_ended: m.HazardKind,
    campaign_finished: m.Outcome,
};
pub const TickReport = struct {
    tick: m.Tick,
    command_results: std.ArrayList(CommandResult) = .empty,
    events: std.ArrayList(GameEvent) = .empty,

    pub fn deinit(self: *TickReport, allocator: std.mem.Allocator) void {
        self.command_results.deinit(allocator);
        self.events.deinit(allocator);
        self.* = undefined;
    }
};
const AiIntent = struct { faction: m.Faction, source: m.WorldId, destination: m.WorldId };
const LaunchPlan = struct {
    source_index: usize,
    launch_strength: u64,
    route_length: u64,
    speed_per_second: u64,
    hydrosphere_level: u8,
};
const TunePlan = struct { world_index: usize, field: m.FieldKind, level: u8, energy: u64 };

/// Classic RulesV1 authority. The caller owns reports and the simulation.
/// All fallible tick allocation happens before the first authority mutation.
pub const Simulation = struct {
    state: m.Campaign,
    rules: m.RulesV1,
    allocator: std.mem.Allocator,
    pending: std.ArrayList(m.CommandEnvelope) = .empty,
    scenario_fingerprint: ?scenario.ScenarioFingerprint = null,

    pub fn generated(allocator: std.mem.Allocator, seed: u64, rules: m.RulesV1) Simulation {
        var result = fromCampaign(allocator, generator.generate(seed, rules), rules);
        if (scenario.generatedValidated(seed, rules)) |validated| {
            result.scenario_fingerprint = validated.fingerprint;
        } else |_| {}
        return result;
    }

    /// Transfers ownership of the campaign's fleet allocation.
    pub fn fromCampaign(allocator: std.mem.Allocator, state: m.Campaign, rules: m.RulesV1) Simulation {
        return .{ .allocator = allocator, .state = state, .rules = rules };
    }

    pub fn fromScenario(allocator: std.mem.Allocator, candidate: *const scenario.ScenarioV1) scenario.AdmissionError!Simulation {
        const validated = try scenario.admit(candidate);
        return .{
            .allocator = allocator,
            .state = m.Campaign.fromWorlds(validated.seed, validated.worlds),
            .rules = validated.rules,
            .scenario_fingerprint = validated.fingerprint,
        };
    }

    pub fn replayIdentity(self: *const Simulation) ?scenario.ReplayIdentity {
        const fingerprint = self.scenario_fingerprint orelse return null;
        return .{ .scenario = fingerprint, .state = self.canonicalFingerprint() };
    }

    pub fn deinit(self: *Simulation) void {
        self.state.deinit(self.allocator);
        self.pending.deinit(self.allocator);
        self.* = undefined;
    }

    pub fn enqueue(self: *Simulation, envelope: m.CommandEnvelope) (CommandRejection || std.mem.Allocator.Error)!void {
        if (self.state.phase == .finished) return error.CampaignFinished;
        if (envelope.tick != self.state.next_tick) return error.WrongTick;
        for (self.pending.items) |queued| {
            if (queued.sequence == envelope.sequence) return error.DuplicateSequence;
        }
        try self.pending.append(self.allocator, envelope);
    }

    pub fn enqueueNext(self: *Simulation, sequence: m.CommandSequence, command: m.GameCommand) (CommandRejection || std.mem.Allocator.Error)!void {
        try self.enqueue(.{ .tick = self.state.next_tick, .sequence = sequence, .command = command });
    }

    pub fn previewCommand(self: *const Simulation, command: m.GameCommand) CommandRejection!CommandPreview {
        if (self.state.phase == .finished) return error.CampaignFinished;
        return switch (command) {
            .launch => |launch| blk: {
                const plan = try validateLaunch(&self.state, self.rules, .union_faction, launch.source, launch.destination);
                break :blk .{ .command = command, .source = launch.source, .destination = launch.destination, .launch_strength = plan.launch_strength };
            },
            .tune_field => |tune| blk: {
                _ = try validateTune(&self.state, self.rules, .union_faction, tune.world, tune.field, tune.adjustment);
                break :blk .{ .command = command, .source = tune.world, .destination = null, .launch_strength = null };
            },
        };
    }

    pub fn step(self: *Simulation) std.mem.Allocator.Error!TickReport {
        const tick = self.state.next_tick;
        var report: TickReport = .{ .tick = tick };
        if (self.state.phase == .finished) return report;
        errdefer report.deinit(self.allocator);
        const launch_capacity = std.math.add(usize, self.pending.items.len, 2) catch return error.OutOfMemory;
        const event_capacity = std.math.add(usize, self.pending.items.len, 12) catch return error.OutOfMemory;
        try report.command_results.ensureTotalCapacity(self.allocator, self.pending.items.len);
        try report.events.ensureTotalCapacity(self.allocator, event_capacity);
        var launched: std.ArrayList(m.FleetId) = .empty;
        defer launched.deinit(self.allocator);
        try launched.ensureTotalCapacity(self.allocator, launch_capacity);
        try self.state.fleets.ensureUnusedCapacity(self.allocator, launch_capacity);

        self.updateHazard(tick, &report.events);
        // Both factions see the same start-of-tick state, before player commands.
        const ai_due = tick > 0 and self.rules.ai_period_ticks > 0 and tick % self.rules.ai_period_ticks == 0;
        const helix = if (ai_due) self.aiIntent(.helix) else null;
        const choir = if (ai_due) self.aiIntent(.choir) else null;
        std.mem.sort(m.CommandEnvelope, self.pending.items, {}, struct {
            fn less(_: void, a: m.CommandEnvelope, b: m.CommandEnvelope) bool {
                return a.sequence < b.sequence;
            }
        }.less);
        for (self.pending.items) |envelope| {
            const event = self.applyPlayerCommand(envelope.command) catch |reason| {
                report.command_results.appendAssumeCapacity(.{ .rejected = .{ .sequence = envelope.sequence, .reason = reason } });
                continue;
            };
            if (event) |id| {
                launched.appendAssumeCapacity(id);
                report.events.appendAssumeCapacity(.{ .fleet_launched = id });
            }
            report.command_results.appendAssumeCapacity(.{ .accepted = envelope.sequence });
        }
        self.pending.clearRetainingCapacity();
        for ([_]?AiIntent{ helix, choir }) |optional| {
            if (optional) |intent| {
                const id = self.launchForFaction(intent.faction, intent.source, intent.destination) catch continue;
                launched.appendAssumeCapacity(id);
                report.events.appendAssumeCapacity(.{ .fleet_launched = id });
            }
        }
        const affected = if (self.state.active_hazard) |h| h.affected_field else null;
        for (&self.state.worlds) |*world| economy.accrueWorld(world, self.rules, affected);
        self.resolveArrivals(self.advanceFleets(launched.items), &report.events);
        self.resolveOutcome(tick, &report.events);
        self.state.next_tick = tick +| 1;
        return report;
    }

    pub fn stepN(self: *Simulation, count: u32) std.mem.Allocator.Error!void {
        for (0..count) |_| {
            var report = try self.step();
            report.deinit(self.allocator);
        }
    }

    fn updateHazard(self: *Simulation, tick: m.Tick, events: *std.ArrayList(GameEvent)) void {
        if (self.state.active_hazard) |hazard| {
            if (hazard.end == tick) {
                self.state.active_hazard = null;
                events.appendAssumeCapacity(.{ .hazard_ended = hazard.kind });
            }
        }
        const index = self.state.next_hazard_index;
        const start = self.rules.hazard_first_tick +| (index *| self.rules.hazard_period_ticks);
        if (tick == start) {
            const hazard: m.ActiveHazard = .{
                .event_index = index,
                .kind = if (index % 2 == 0) .ion_storm else .gravity_tide,
                .affected_field = switch (generator.splitmix64Once(self.state.seed ^ index) % 3) {
                    0 => .atmosphere,
                    1 => .hydrosphere,
                    else => .topology,
                },
                .start = tick,
                .end = start +| self.rules.hazard_duration_ticks,
            };
            self.state.active_hazard = hazard;
            self.state.next_hazard_index = index +| 1;
            events.appendAssumeCapacity(.{ .hazard_started = hazard });
        }
    }

    fn applyPlayerCommand(self: *Simulation, command: m.GameCommand) CommandRejection!?m.FleetId {
        switch (command) {
            .launch => |launch| return try self.launchForFaction(.union_faction, launch.source, launch.destination),
            .tune_field => |tune| {
                const plan = try validateTune(&self.state, self.rules, .union_faction, tune.world, tune.field, tune.adjustment);
                const world = &self.state.worlds[plan.world_index];
                world.energy = plan.energy;
                world.fields.set(plan.field, plan.level);
                return null;
            },
        }
    }

    fn launchForFaction(self: *Simulation, faction: m.Faction, source: m.WorldId, destination: m.WorldId) CommandRejection!m.FleetId {
        const plan = try validateLaunch(&self.state, self.rules, faction, source, destination);
        const id = self.state.next_fleet_id;
        self.state.next_fleet_id = id +| 1;
        self.state.worlds[plan.source_index].energy -|= plan.launch_strength;
        self.state.fleets.appendAssumeCapacity(.{
            .id = id,
            .faction = faction,
            .source = source,
            .destination = destination,
            .strength = plan.launch_strength,
            .route_length = plan.route_length,
            .progress = 0,
            .speed_per_second = plan.speed_per_second,
            .hydrosphere_level = plan.hydrosphere_level,
            .movement_remainder = 0,
        });
        sortFleets(self.state.fleets.items);
        return id;
    }

    const Arrivals = struct {
        forces: [256][3]u64 = @splat(@splat(0)),
        present: [256]bool = @splat(false),
    };

    fn advanceFleets(self: *Simulation, launched: []const m.FleetId) Arrivals {
        var arrivals: Arrivals = .{};
        const hazardous = if (self.state.active_hazard) |h| h.affected_field == .hydrosphere else false;
        var retained: usize = 0;
        for (self.state.fleets.items) |original| {
            var fleet = original;
            const is_new = std.mem.indexOfScalar(m.FleetId, launched, fleet.id) != null;
            if (!is_new) navigation.advanceFleet(&fleet, self.rules, hazardous);
            if (!is_new and fleet.progress >= fleet.route_length) {
                arrivals.present[fleet.destination] = true;
                arrivals.forces[fleet.destination][factionIndex(fleet.faction)] +|= fleet.strength;
            } else {
                self.state.fleets.items[retained] = fleet;
                retained += 1;
            }
        }
        self.state.fleets.items.len = retained;
        sortFleets(self.state.fleets.items);
        return arrivals;
    }

    fn resolveArrivals(self: *Simulation, arrivals: Arrivals, events: *std.ArrayList(GameEvent)) void {
        for (arrivals.present, 0..) |present, destination| {
            if (!present) continue;
            const index = worldIndex(&self.state, @intCast(destination)) orelse continue;
            const world = &self.state.worlds[index];
            const previous_owner = world.owner;
            var forces: [3]u128 = undefined;
            for (&forces, arrivals.forces[destination]) |*force, arriving| force.* = arriving;
            var neutral: u128 = 0;
            if (previous_owner) |owner| forces[factionIndex(owner)] += world.defense else neutral = world.defense;
            const total = forces[0] + forces[1] + forces[2];
            var owner: ?m.Faction = null;
            var defense: u64 = 1;
            var strict_winner = false;
            for (forces, 0..) |force, i| {
                const opposition = neutral + total - force;
                if (force > opposition) {
                    if (self.rules.maximum_defense == 0) @panic("RulesV1.maximum_defense must be nonzero for combat");
                    owner = indexFaction(i);
                    defense = @intCast(@min(@max(force - opposition, 1), self.rules.maximum_defense));
                    strict_winner = true;
                    break;
                }
            }
            if (!strict_winner) {
                const incumbent = if (previous_owner) |f| forces[factionIndex(f)] else neutral;
                var holds = true;
                for (forces, 0..) |force, i| {
                    if (previous_owner == null or factionIndex(previous_owner.?) != i) {
                        if (incumbent < force) holds = false;
                    }
                }
                if (holds) owner = previous_owner;
            }
            world.owner = owner;
            world.defense = defense;
            if (owner != previous_owner) {
                world.energy = 0;
                events.appendAssumeCapacity(.{ .world_captured = .{ .world = @intCast(destination), .owner = owner } });
            }
        }
    }

    fn resolveOutcome(self: *Simulation, tick: m.Tick, events: *std.ArrayList(GameEvent)) void {
        var outcome: ?m.Outcome = null;
        for ([_]m.Faction{ .union_faction, .helix, .choir }) |faction| {
            if (self.state.controlledCount(faction) >= self.rules.win_world_count) {
                outcome = .{ .faction_victory = .{ .winner = faction } };
                break;
            }
        }
        if (outcome == null and self.state.controlledCount(.union_faction) == 0 and self.state.fleetCount(.union_faction) == 0) outcome = .player_eliminated;
        if (outcome) |value| {
            self.state.phase = .{ .finished = .{ .at = tick, .outcome = value } };
            events.appendAssumeCapacity(.{ .campaign_finished = value });
        }
    }

    fn aiIntent(self: *const Simulation, faction: m.Faction) ?AiIntent {
        var threatened: ?*const m.World = null;
        var greatest_deficit: u128 = 0;
        var earliest_remaining: u64 = std.math.maxInt(u64);
        for (&self.state.worlds) |*world| {
            if (world.owner != faction) continue;
            var friendly: u128 = 0;
            var hostile: u128 = 0;
            var remaining: u64 = std.math.maxInt(u64);
            for (self.state.fleets.items) |fleet| {
                if (fleet.destination != world.id) continue;
                if (fleet.faction == faction) friendly += fleet.strength else {
                    hostile += fleet.strength;
                    remaining = @min(remaining, fleet.route_length -| fleet.progress);
                }
            }
            const deficit = hostile -| (world.defense + friendly);
            if (deficit == 0) continue;
            if (threatened == null or deficit > greatest_deficit or
                (deficit == greatest_deficit and (remaining < earliest_remaining or
                    (remaining == earliest_remaining and world.id < threatened.?.id))))
            {
                threatened = world;
                greatest_deficit = deficit;
                earliest_remaining = remaining;
            }
        }
        if (threatened) |target| {
            var source: ?*const m.World = null;
            for (&self.state.worlds) |*world| {
                if (world.owner != faction or world.id == target.id or world.energy / 2 < self.rules.minimum_launch) continue;
                if (source == null or betterSource(world, source.?, target, true)) source = world;
            }
            if (source) |world| return .{ .faction = faction, .source = world.id, .destination = target.id };
        }
        var target: ?*const m.World = null;
        var least_force: u64 = std.math.maxInt(u64);
        for (&self.state.worlds) |*world| {
            if (world.owner == faction) continue;
            var inbound: u64 = 0;
            if (world.owner) |owner| {
                for (self.state.fleets.items) |fleet| {
                    if (fleet.destination == world.id and fleet.faction == owner) inbound +|= fleet.strength;
                }
            }
            const force = world.defense +| inbound;
            if (target == null or force < least_force or (force == least_force and world.id < target.?.id)) {
                target = world;
                least_force = force;
            }
        }
        const destination = target orelse return null;
        var source: ?*const m.World = null;
        for (&self.state.worlds) |*world| {
            if (world.owner != faction or world.energy / 2 < self.rules.minimum_launch) continue;
            if (source == null or betterSource(world, source.?, destination, false)) source = world;
        }
        const origin = source orelse return null;
        return .{ .faction = faction, .source = origin.id, .destination = destination.id };
    }

    pub fn canonicalFingerprint(self: *const Simulation) u64 {
        var hash: Fnv1a64 = .{};
        hash.integer(u32, self.state.rules_version);
        hash.integer(u32, self.state.generator_version);
        hash.integer(u64, self.state.seed);
        hash.integer(u64, self.state.next_tick);
        switch (self.state.phase) {
            .running => hash.integer(u8, 0),
            .finished => |finished| switch (finished.outcome) {
                .faction_victory => |victory| {
                    hash.integer(u8, 1);
                    hash.integer(u8, factionTag(victory.winner));
                },
                .player_eliminated => hash.integer(u8, 2),
            },
        }
        hash.integer(u64, self.state.next_fleet_id);
        hash.integer(u64, self.state.next_hazard_index);
        if (self.state.active_hazard) |hazard| {
            hash.integer(u8, 1);
            hash.integer(u64, hazard.event_index);
            hash.integer(u8, switch (hazard.kind) {
                .ion_storm => 0,
                .gravity_tide => 1,
            });
            hash.integer(u8, switch (hazard.affected_field) {
                .atmosphere => 0,
                .hydrosphere => 1,
                .topology => 2,
            });
            hash.integer(u64, hazard.start);
            hash.integer(u64, hazard.end);
        } else hash.integer(u8, 0);
        var worlds: [m.WORLD_COUNT]*const m.World = undefined;
        for (&worlds, &self.state.worlds) |*pointer, *world| pointer.* = world;
        std.mem.sort(*const m.World, &worlds, {}, struct {
            fn less(_: void, a: *const m.World, b: *const m.World) bool {
                return a.id < b.id;
            }
        }.less);
        for (worlds) |world| {
            hash.integer(u8, world.id);
            hash.integer(i32, world.position.x);
            hash.integer(i32, world.position.y);
            hash.integer(u8, if (world.owner) |owner| factionTag(owner) else 0);
            hash.integer(u64, world.defense);
            hash.integer(u64, world.energy);
            hash.integer(u64, world.base_output_per_second);
            hash.integer(u64, world.base_regeneration_per_second);
            hash.integer(u8, world.fields.atmosphere);
            hash.integer(u8, world.fields.hydrosphere);
            hash.integer(u8, world.fields.topology);
            hash.integer(u64, world.production_remainder);
            hash.integer(u64, world.regeneration_remainder);
        }
        hash.integer(u64, @intCast(self.state.fleets.items.len));
        const fleets = self.state.fleets.items;
        var ordered = true;
        for (fleets, 0..) |fleet, i| {
            if (i > 0 and fleets[i - 1].id > fleet.id) {
                ordered = false;
                break;
            }
        }
        if (ordered) {
            for (fleets) |fleet| hash.fleet(fleet);
        } else {
            // Fixture callers may supply unordered fleets. Stable ID order,
            // without allocation or mutation of the authoritative campaign.
            var previous: ?usize = null;
            for (0..fleets.len) |_| {
                var next: ?usize = null;
                for (fleets, 0..) |fleet, i| {
                    if (previous) |p| {
                        if (fleet.id < fleets[p].id or (fleet.id == fleets[p].id and i <= p)) continue;
                    }
                    if (next == null or fleet.id < fleets[next.?].id or (fleet.id == fleets[next.?].id and i < next.?)) next = i;
                }
                hash.fleet(fleets[next.?]);
                previous = next;
            }
        }
        return hash.value;
    }
};

fn betterSource(a: *const m.World, b: *const m.World, target: *const m.World, defensive: bool) bool {
    const a_route = navigation.routeLength(a.position, target.position);
    const b_route = navigation.routeLength(b.position, target.position);
    const a_strength = a.energy / 2;
    const b_strength = b.energy / 2;
    if (defensive) {
        if (a_route != b_route) return a_route < b_route;
        if (a_strength != b_strength) return a_strength > b_strength;
    } else {
        if (a_strength != b_strength) return a_strength > b_strength;
        if (a_route != b_route) return a_route < b_route;
    }
    return a.id < b.id;
}
fn worldIndex(campaign: *const m.Campaign, id: m.WorldId) ?usize {
    for (campaign.worlds, 0..) |world, i| if (world.id == id) return i;
    return null;
}
fn validateLaunch(campaign: *const m.Campaign, rules: m.RulesV1, faction: m.Faction, source: m.WorldId, destination: m.WorldId) CommandRejection!LaunchPlan {
    const source_index = worldIndex(campaign, source) orelse return error.InvalidWorld;
    const destination_index = worldIndex(campaign, destination) orelse return error.InvalidWorld;
    if (source == destination) return error.SameSourceAndTarget;
    const world = campaign.worlds[source_index];
    if (world.owner != faction) return error.SourceNotOwnedByIssuer;
    const strength = world.energy / 2;
    if (strength < rules.minimum_launch) return error.InsufficientEnergy;
    return .{ .source_index = source_index, .launch_strength = strength, .route_length = navigation.routeLength(world.position, campaign.worlds[destination_index].position), .speed_per_second = navigation.fleetSpeed(rules.base_fleet_speed, world.fields.hydrosphere, false), .hydrosphere_level = world.fields.hydrosphere };
}
fn validateTune(campaign: *const m.Campaign, rules: m.RulesV1, faction: m.Faction, id: m.WorldId, field: m.FieldKind, adjustment: m.FieldAdjustment) CommandRejection!TunePlan {
    const index = worldIndex(campaign, id) orelse return error.InvalidWorld;
    const world = campaign.worlds[index];
    if (world.owner != faction) return error.SourceNotOwnedByIssuer;
    const level = world.fields.get(field);
    return switch (adjustment) {
        .increase => blk: {
            if (level >= 10) return error.FieldAtLimit;
            if (world.energy < rules.field_raise_cost) return error.InsufficientEnergy;
            break :blk .{ .world_index = index, .field = field, .level = level + 1, .energy = world.energy - rules.field_raise_cost };
        },
        .decrease => blk: {
            if (level == 0) return error.FieldAtLimit;
            break :blk .{ .world_index = index, .field = field, .level = level - 1, .energy = @min(world.energy +| rules.field_lower_refund, rules.maximum_energy) };
        },
    };
}
fn factionIndex(faction: m.Faction) usize {
    return switch (faction) {
        .union_faction => 0,
        .helix => 1,
        .choir => 2,
    };
}
fn indexFaction(index: usize) m.Faction {
    return switch (index) {
        0 => .union_faction,
        1 => .helix,
        else => .choir,
    };
}
fn factionTag(faction: m.Faction) u8 {
    return @intCast(factionIndex(faction) + 1);
}
fn sortFleets(fleets: []m.Fleet) void {
    std.mem.sort(m.Fleet, fleets, {}, struct {
        fn less(_: void, a: m.Fleet, b: m.Fleet) bool {
            return a.id < b.id;
        }
    }.less);
}
const Fnv1a64 = struct {
    value: u64 = 0xCBF2_9CE4_8422_2325,
    fn integer(self: *Fnv1a64, comptime T: type, value: T) void {
        var bytes: [@sizeOf(T)]u8 = undefined;
        std.mem.writeInt(T, &bytes, value, .little);
        for (bytes) |byte| self.value = (self.value ^ byte) *% 0x0000_0100_0000_01B3;
    }
    fn fleet(self: *Fnv1a64, value: m.Fleet) void {
        self.integer(u64, value.id);
        self.integer(u8, factionTag(value.faction));
        self.integer(u8, value.source);
        self.integer(u8, value.destination);
        self.integer(u64, value.strength);
        self.integer(u64, value.route_length);
        self.integer(u64, value.progress);
        self.integer(u64, value.speed_per_second);
        self.integer(u8, value.hydrosphere_level);
        self.integer(u64, value.movement_remainder);
    }
};
