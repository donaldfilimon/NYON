//! RulesV1 behavior regressions translated from the independent Rust fixtures.
//! Fixture worlds suppress income so combat and command assertions expose the
//! phase boundary rather than relying on rounded economy output.
const std = @import("std");
const m = @import("model.zig");
const generator = @import("generator.zig");
const s = @import("simulation.zig");
const expect = std.testing.expect;
const expectEqual = std.testing.expectEqual;
const expectEqualDeep = std.testing.expectEqualDeep;
const allocator = std.testing.allocator;

fn quietRules() m.RulesV1 {
    return .{ .ai_period_ticks = 0, .hazard_first_tick = std.math.maxInt(u64) };
}

fn fixture(rules: m.RulesV1) m.Campaign {
    var campaign = generator.generate(m.DEFAULT_SEED, rules);
    for (&campaign.worlds, 0..) |*world, i| {
        world.position = .{ .x = @as(i32, @intCast(i)) * 100, .y = 0 };
        world.owner = switch (i) {
            0 => .union_faction,
            1 => .helix,
            2 => .choir,
            else => null,
        };
        world.defense = 1000;
        world.energy = 0;
        world.base_output_per_second = 0;
        world.base_regeneration_per_second = 0;
        world.fields = .init(0, 0, 0);
        world.production_remainder = 0;
        world.regeneration_remainder = 0;
    }
    return campaign;
}

fn arriving(id: u64, faction: m.Faction, destination: m.WorldId, strength: u64) m.Fleet {
    return .{
        .id = id,
        .faction = faction,
        .source = 6,
        .destination = destination,
        .strength = strength,
        .route_length = 1,
        .progress = 1,
        .speed_per_second = 0,
        .hydrosphere_level = 0,
        .movement_remainder = 0,
    };
}

fn inbound(id: u64, faction: m.Faction, destination: m.WorldId, strength: u64, remaining: u64) m.Fleet {
    var fleet = arriving(id, faction, destination, strength);
    fleet.route_length = remaining;
    fleet.progress = 0;
    return fleet;
}

fn fleetFor(simulation: *const s.Simulation, faction: m.Faction) !m.Fleet {
    for (simulation.state.fleets.items) |fleet| {
        if (fleet.faction == faction) return fleet;
    }
    return error.ExpectedFactionFleet;
}

test "RulesV1 exact opposing force preserves incumbent at one defense" {
    const rules = quietRules();
    var campaign = fixture(rules);
    campaign.worlds[0].defense = 20_000;
    campaign.worlds[0].energy = 50_000;
    try campaign.fleets.append(allocator, arriving(0, .helix, 0, 20_000));
    campaign.next_fleet_id = 1;
    var simulation = s.Simulation.fromCampaign(allocator, campaign, rules);
    defer simulation.deinit();
    var report = try simulation.step();
    defer report.deinit(allocator);
    try expectEqual(@as(?m.Faction, .union_faction), simulation.state.worlds[0].owner);
    try expectEqual(@as(u64, 1), simulation.state.worlds[0].defense);
    try expectEqual(@as(u64, 50_000), simulation.state.worlds[0].energy);
    try expectEqual(@as(usize, 0), simulation.state.fleets.items.len);
    try expectEqual(@as(usize, 0), report.events.items.len);
}

test "RulesV1 tied attackers neutralize and clear the former owner's energy" {
    const rules = quietRules();
    var campaign = fixture(rules);
    campaign.worlds[0].defense = 1;
    campaign.worlds[0].energy = 50_000;
    try campaign.fleets.appendSlice(allocator, &.{ arriving(0, .helix, 0, 20_000), arriving(1, .choir, 0, 20_000) });
    campaign.next_fleet_id = 2;
    var simulation = s.Simulation.fromCampaign(allocator, campaign, rules);
    defer simulation.deinit();
    var report = try simulation.step();
    defer report.deinit(allocator);
    try expectEqual(@as(?m.Faction, null), simulation.state.worlds[0].owner);
    try expectEqual(@as(u64, 1), simulation.state.worlds[0].defense);
    try expectEqual(@as(u64, 0), simulation.state.worlds[0].energy);
    try expectEqualDeep(s.GameEvent{ .world_captured = .{ .world = 0, .owner = null } }, report.events.items[0]);
}

test "RulesV1 capture retains fields and only the strict surviving force" {
    const rules = quietRules();
    var campaign = fixture(rules);
    campaign.worlds[0].energy = 50_000;
    campaign.worlds[0].fields = .init(2, 4, 0);
    try campaign.fleets.append(allocator, arriving(0, .helix, 0, 5000));
    campaign.next_fleet_id = 1;
    var simulation = s.Simulation.fromCampaign(allocator, campaign, rules);
    defer simulation.deinit();
    var report = try simulation.step();
    defer report.deinit(allocator);
    try expectEqual(@as(?m.Faction, .helix), simulation.state.worlds[0].owner);
    try expectEqual(@as(u64, 4000), simulation.state.worlds[0].defense);
    try expectEqual(@as(u64, 0), simulation.state.worlds[0].energy);
    try expectEqualDeep(m.FieldLevels.init(2, 4, 0), simulation.state.worlds[0].fields);
    try expectEqualDeep(s.GameEvent{ .world_captured = .{ .world = 0, .owner = .helix } }, report.events.items[0]);
}

test "RulesV1 simultaneous allied and hostile arrivals are independent of storage order" {
    const rules = quietRules();
    var left = fixture(rules);
    left.worlds[0].energy = 30_000;
    try left.fleets.appendSlice(allocator, &.{ arriving(0, .helix, 0, 9000), arriving(1, .helix, 0, 7000), arriving(2, .choir, 0, 4000), arriving(3, .union_faction, 0, 3000) });
    left.next_fleet_id = 4;
    const right = try left.clone(allocator);
    std.mem.reverse(m.Fleet, right.fleets.items);
    var first = s.Simulation.fromCampaign(allocator, left, rules);
    defer first.deinit();
    var second = s.Simulation.fromCampaign(allocator, right, rules);
    defer second.deinit();
    try expectEqual(first.canonicalFingerprint(), second.canonicalFingerprint());
    var report = try first.step();
    defer report.deinit(allocator);
    var reordered = try second.step();
    defer reordered.deinit(allocator);
    try expectEqual(@as(?m.Faction, .helix), first.state.worlds[0].owner);
    try expectEqual(@as(u64, 8000), first.state.worlds[0].defense);
    try expectEqual(first.canonicalFingerprint(), second.canonicalFingerprint());
    try expectEqualDeep(report.events.items, reordered.events.items);
}

test "RulesV1 five worlds finishes for every faction and the finished state absorbs commands and time" {
    for ([_]m.Faction{ .union_faction, .helix, .choir }) |winner| {
        const rules = quietRules();
        var campaign = fixture(rules);
        for (&campaign.worlds, 0..) |*world, i| world.owner = if (i < 5) winner else null;
        var simulation = s.Simulation.fromCampaign(allocator, campaign, rules);
        defer simulation.deinit();
        var report = try simulation.step();
        defer report.deinit(allocator);
        const outcome: m.Outcome = .{ .faction_victory = .{ .winner = winner } };
        try expectEqualDeep(m.Phase{ .finished = .{ .at = 0, .outcome = outcome } }, simulation.state.phase);
        try expectEqualDeep(s.GameEvent{ .campaign_finished = outcome }, report.events.items[0]);
        const digest = simulation.canonicalFingerprint();
        const launch: m.GameCommand = .{ .launch = .{ .source = 0, .destination = 1 } };
        try std.testing.expectError(error.CampaignFinished, simulation.enqueueNext(0, launch));
        try std.testing.expectError(error.CampaignFinished, simulation.previewCommand(launch));
        try simulation.stepN(600);
        try expectEqual(digest, simulation.canonicalFingerprint());
        try expectEqual(@as(u64, 1), simulation.state.next_tick);
        var absorbed = try simulation.step();
        defer absorbed.deinit(allocator);
        try expectEqual(@as(usize, 0), absorbed.events.items.len);
        try expectEqual(@as(usize, 0), absorbed.command_results.items.len);
    }
}

test "RulesV1 outstanding Union fleet delays elimination until its final loss" {
    const rules = quietRules();
    var campaign = fixture(rules);
    campaign.worlds[0].owner = null;
    campaign.worlds[4].defense = 30_000;
    try campaign.fleets.append(allocator, inbound(0, .union_faction, 4, 20_000, 1000));
    campaign.next_fleet_id = 1;
    var simulation = s.Simulation.fromCampaign(allocator, campaign, rules);
    defer simulation.deinit();
    var waiting = try simulation.step();
    defer waiting.deinit(allocator);
    try expectEqualDeep(m.Phase.running, simulation.state.phase);
    try expectEqual(@as(usize, 1), simulation.state.fleetCount(.union_faction));
    simulation.state.fleets.items[0].progress = 1000;
    var loss = try simulation.step();
    defer loss.deinit(allocator);
    try expectEqualDeep(m.Phase{ .finished = .{ .at = 1, .outcome = .player_eliminated } }, simulation.state.phase);
    try expectEqual(@as(usize, 0), simulation.state.fleets.items.len);
    const digest = simulation.canonicalFingerprint();
    try simulation.stepN(20);
    try expectEqual(digest, simulation.canonicalFingerprint());
}

test "RulesV1 final Union fleet can establish a world instead of being eliminated" {
    const rules = quietRules();
    var campaign = fixture(rules);
    campaign.worlds[0].owner = null;
    try campaign.fleets.append(allocator, arriving(0, .union_faction, 4, 2000));
    campaign.next_fleet_id = 1;
    var simulation = s.Simulation.fromCampaign(allocator, campaign, rules);
    defer simulation.deinit();
    var report = try simulation.step();
    defer report.deinit(allocator);
    try expectEqual(@as(?m.Faction, .union_faction), simulation.state.worlds[4].owner);
    try expectEqualDeep(m.Phase.running, simulation.state.phase);
    try expectEqual(@as(usize, 0), simulation.state.fleets.items.len);
}

test "RulesV1 AI only first acts at the inclusive tick sixty boundary" {
    var simulation = s.Simulation.generated(allocator, m.DEFAULT_SEED, .{});
    defer simulation.deinit();
    try simulation.stepN(60);
    try expectEqual(@as(usize, 0), simulation.state.fleets.items.len);
    var report = try simulation.step();
    defer report.deinit(allocator);
    try expectEqual(@as(u64, 60), report.tick);
    try expectEqual(@as(usize, 2), simulation.state.fleets.items.len);
    try expectEqual(m.Faction.helix, simulation.state.fleets.items[0].faction);
    try expectEqual(m.Faction.choir, simulation.state.fleets.items[1].faction);
    for (simulation.state.fleets.items) |fleet| try expectEqual(@as(u64, 0), fleet.progress);
}

test "RulesV1 both AI factions use the state before player commands and other launches" {
    const rules: m.RulesV1 = .{};
    var campaign = fixture(rules);
    campaign.next_tick = 60;
    for (&campaign.worlds) |*world| world.defense = 90_000;
    campaign.worlds[0].defense = 2000;
    campaign.worlds[0].energy = 80_000;
    campaign.worlds[1].defense = 3000;
    campaign.worlds[1].energy = 80_000;
    campaign.worlds[2].defense = 1000;
    campaign.worlds[2].energy = 80_000;
    campaign.worlds[3].owner = .choir;
    campaign.worlds[3].energy = 80_000;
    campaign.worlds[4].owner = .union_faction;
    campaign.worlds[4].energy = 80_000;
    var simulation = s.Simulation.fromCampaign(allocator, campaign, rules);
    defer simulation.deinit();
    // A current-tick Union reinforcement must not change the Choir target.
    try simulation.enqueueNext(0, .{ .launch = .{ .source = 4, .destination = 0 } });
    var report = try simulation.step();
    defer report.deinit(allocator);
    try expectEqualDeep(s.CommandResult{ .accepted = 0 }, report.command_results.items[0]);
    const player = try fleetFor(&simulation, .union_faction);
    try expectEqual(@as(u8, 4), player.source);
    try expectEqual(@as(u8, 0), player.destination);
    const helix = try fleetFor(&simulation, .helix);
    const choir = try fleetFor(&simulation, .choir);
    try expectEqual(@as(u8, 2), helix.destination);
    try expectEqual(@as(u8, 2), choir.source);
    try expectEqual(@as(u8, 0), choir.destination);
}

test "RulesV1 AI weakest target ties prefer lowest world ID even with permuted storage" {
    const rules: m.RulesV1 = .{};
    var campaign = fixture(rules);
    campaign.next_tick = 60;
    for (&campaign.worlds) |*world| world.defense = 10_000;
    campaign.worlds[1].energy = 80_000;
    var reversed = try campaign.clone(allocator);
    std.mem.reverse(m.World, &reversed.worlds);
    var first = s.Simulation.fromCampaign(allocator, campaign, rules);
    defer first.deinit();
    var second = s.Simulation.fromCampaign(allocator, reversed, rules);
    defer second.deinit();
    var report = try first.step();
    defer report.deinit(allocator);
    var other = try second.step();
    defer other.deinit(allocator);
    try expectEqual(@as(u8, 0), (try fleetFor(&first, .helix)).destination);
    try expectEqual(first.canonicalFingerprint(), second.canonicalFingerprint());
}

test "RulesV1 defensive AI ranks deficit then arrival distance then ID and subtracts friendly support" {
    const Case = struct { hostile: [2]u64, remaining: [2]u64, friendly_first: u64 = 0, target: u8 };
    for ([_]Case{
        .{ .hostile = .{ 5000, 7000 }, .remaining = .{ 100, 900 }, .target = 2 },
        .{ .hostile = .{ 6000, 6000 }, .remaining = .{ 900, 100 }, .target = 2 },
        .{ .hostile = .{ 6000, 6000 }, .remaining = .{ 100, 100 }, .target = 1 },
        .{ .hostile = .{ 7000, 6000 }, .remaining = .{ 100, 100 }, .friendly_first = 3000, .target = 2 },
    }) |case| {
        const rules: m.RulesV1 = .{};
        var campaign = fixture(rules);
        campaign.next_tick = 60;
        campaign.worlds[2].owner = .helix;
        campaign.worlds[3].owner = .helix;
        campaign.worlds[3].energy = 40_000;
        campaign.worlds[4].owner = .helix;
        campaign.worlds[4].energy = 120_000;
        try campaign.fleets.appendSlice(allocator, &.{ inbound(0, .union_faction, 1, case.hostile[0], case.remaining[0]), inbound(1, .union_faction, 2, case.hostile[1], case.remaining[1]) });
        if (case.friendly_first > 0) try campaign.fleets.append(allocator, inbound(2, .helix, 1, case.friendly_first, 200));
        campaign.next_fleet_id = 3;
        var simulation = s.Simulation.fromCampaign(allocator, campaign, rules);
        defer simulation.deinit();
        var report = try simulation.step();
        defer report.deinit(allocator);
        var reinforcement: ?m.Fleet = null;
        for (simulation.state.fleets.items) |fleet| if (fleet.id == 3) {
            reinforcement = fleet;
        };
        try expect(reinforcement != null);
        try expectEqual(case.target, reinforcement.?.destination);
        // Defensive source proximity outranks the farther source's larger force.
        try expectEqual(@as(u8, 3), reinforcement.?.source);
        try expectEqual(@as(u64, 20_000), reinforcement.?.strength);
    }
}

test "RulesV1 a co-located new launch waits until the next tick before combat" {
    const rules = quietRules();
    var campaign = fixture(rules);
    campaign.worlds[0].energy = 80_000;
    campaign.worlds[3].position = campaign.worlds[0].position;
    campaign.worlds[3].defense = 40_000;
    var simulation = s.Simulation.fromCampaign(allocator, campaign, rules);
    defer simulation.deinit();
    try simulation.enqueueNext(0, .{ .launch = .{ .source = 0, .destination = 3 } });
    var launch = try simulation.step();
    defer launch.deinit(allocator);
    try expectEqual(@as(usize, 1), simulation.state.fleets.items.len);
    try expectEqual(@as(u64, 0), simulation.state.fleets.items[0].progress);
    try expectEqual(@as(u64, 40_000), simulation.state.worlds[3].defense);
    var arrival = try simulation.step();
    defer arrival.deinit(allocator);
    try expectEqual(@as(usize, 0), simulation.state.fleets.items.len);
    try expectEqual(@as(?m.Faction, null), simulation.state.worlds[3].owner);
    try expectEqual(@as(u64, 1), simulation.state.worlds[3].defense);
}

test "RulesV1 offensive AI selects largest force then closest source then lowest ID" {
    const Case = struct { energy: [2]u64, position: [2]i32, source: u8 };
    for ([_]Case{
        .{ .energy = .{ 40_000, 80_000 }, .position = .{ 100, 900 }, .source = 3 },
        .{ .energy = .{ 80_000, 80_000 }, .position = .{ 900, 100 }, .source = 3 },
        .{ .energy = .{ 80_000, 80_000 }, .position = .{ 100, 100 }, .source = 1 },
    }) |case| {
        const rules: m.RulesV1 = .{};
        var campaign = fixture(rules);
        campaign.next_tick = 60;
        campaign.worlds[0].defense = 1;
        campaign.worlds[1].energy = case.energy[0];
        campaign.worlds[1].position.x = case.position[0];
        campaign.worlds[3].owner = .helix;
        campaign.worlds[3].energy = case.energy[1];
        campaign.worlds[3].position.x = case.position[1];
        var simulation = s.Simulation.fromCampaign(allocator, campaign, rules);
        defer simulation.deinit();
        var report = try simulation.step();
        defer report.deinit(allocator);
        const fleet = try fleetFor(&simulation, .helix);
        try expectEqual(@as(u8, 0), fleet.destination);
        try expectEqual(case.source, fleet.source);
        try expectEqual(@as(u64, 40_000), fleet.strength);
    }
}

test "RulesV1 defensive source ties prefer larger force and finally lower ID" {
    const Case = struct { energy: [2]u64, source: u8 };
    for ([_]Case{
        .{ .energy = .{ 40_000, 80_000 }, .source = 4 },
        .{ .energy = .{ 80_000, 80_000 }, .source = 3 },
    }) |case| {
        const rules: m.RulesV1 = .{};
        var campaign = fixture(rules);
        campaign.next_tick = 60;
        campaign.worlds[3].owner = .helix;
        campaign.worlds[3].energy = case.energy[0];
        campaign.worlds[4].owner = .helix;
        campaign.worlds[4].energy = case.energy[1];
        campaign.worlds[4].position = campaign.worlds[3].position;
        try campaign.fleets.append(allocator, inbound(0, .union_faction, 1, 5000, 200));
        campaign.next_fleet_id = 1;
        var simulation = s.Simulation.fromCampaign(allocator, campaign, rules);
        defer simulation.deinit();
        var report = try simulation.step();
        defer report.deinit(allocator);
        const fleet = try fleetFor(&simulation, .helix);
        try expectEqual(@as(u8, 1), fleet.destination);
        try expectEqual(case.source, fleet.source);
        try expectEqual(@as(u64, 40_000), fleet.strength);
    }
}

test "RulesV1 event stream orders hazard transition then commands AI and simultaneous captures" {
    var rules: m.RulesV1 = .{};
    rules.hazard_first_tick = 0;
    rules.hazard_period_ticks = 60;
    var campaign = fixture(rules);
    campaign.next_tick = 60;
    campaign.active_hazard = .{ .event_index = 0, .kind = .ion_storm, .affected_field = .atmosphere, .start = 0, .end = 60 };
    campaign.next_hazard_index = 1;
    campaign.worlds[0].energy = 80_000;
    campaign.worlds[1].energy = 80_000;
    campaign.worlds[2].energy = 80_000;
    campaign.worlds[4].owner = .union_faction;
    try campaign.fleets.appendSlice(allocator, &.{ arriving(1, .choir, 1, 5000), arriving(0, .helix, 0, 5000) });
    campaign.next_fleet_id = 2;
    var simulation = s.Simulation.fromCampaign(allocator, campaign, rules);
    defer simulation.deinit();
    try simulation.enqueueNext(1, .{ .launch = .{ .source = 0, .destination = 4 } });
    try simulation.enqueueNext(0, .{ .launch = .{ .source = 0, .destination = 5 } });
    var report = try simulation.step();
    defer report.deinit(allocator);
    try expectEqual(@as(usize, 8), report.events.items.len);
    try expectEqualDeep(s.GameEvent{ .hazard_ended = .ion_storm }, report.events.items[0]);
    try expectEqualDeep(s.GameEvent{ .hazard_started = simulation.state.active_hazard.? }, report.events.items[1]);
    for ([_]u64{ 2, 3, 4, 5 }, 2..) |id, index| {
        try expectEqualDeep(s.GameEvent{ .fleet_launched = id }, report.events.items[index]);
    }
    try expectEqualDeep(s.GameEvent{ .world_captured = .{ .world = 0, .owner = .helix } }, report.events.items[6]);
    try expectEqualDeep(s.GameEvent{ .world_captured = .{ .world = 1, .owner = .choir } }, report.events.items[7]);
    try expectEqual(m.Faction.helix, simulation.state.fleets.items[2].faction);
    try expectEqual(m.Faction.choir, simulation.state.fleets.items[3].faction);
}

test "RulesV1 enqueue rejects wrong ticks and duplicate sequences without losing the accepted command" {
    const rules = quietRules();
    var campaign = fixture(rules);
    campaign.worlds[0].energy = 80_000;
    var simulation = s.Simulation.fromCampaign(allocator, campaign, rules);
    defer simulation.deinit();
    const launch: m.GameCommand = .{ .launch = .{ .source = 0, .destination = 3 } };
    const before = simulation.canonicalFingerprint();
    try std.testing.expectError(error.WrongTick, simulation.enqueue(.{ .tick = 1, .sequence = 7, .command = launch }));
    try expectEqual(@as(usize, 0), simulation.pending.items.len);
    try simulation.enqueueNext(7, launch);
    try std.testing.expectError(error.DuplicateSequence, simulation.enqueueNext(7, .{ .launch = .{ .source = 0, .destination = 4 } }));
    try expectEqual(before, simulation.canonicalFingerprint());
    try expectEqual(@as(usize, 1), simulation.pending.items.len);
    var report = try simulation.step();
    defer report.deinit(allocator);
    try expectEqualDeep(s.CommandResult{ .accepted = 7 }, report.command_results.items[0]);
    try expectEqual(@as(u8, 3), simulation.state.fleets.items[0].destination);
    try std.testing.expectError(error.WrongTick, simulation.enqueue(.{ .tick = 0, .sequence = 8, .command = launch }));
}

test "RulesV1 invalid commands match preview failures and leave the tick identical to a control" {
    const Case = struct { command: m.GameCommand, reason: s.CommandRejection, atmosphere: u8 = 0 };
    for ([_]Case{
        .{ .command = .{ .launch = .{ .source = 7, .destination = 0 } }, .reason = error.InvalidWorld },
        .{ .command = .{ .launch = .{ .source = 0, .destination = 7 } }, .reason = error.InvalidWorld },
        .{ .command = .{ .launch = .{ .source = 0, .destination = 0 } }, .reason = error.SameSourceAndTarget },
        .{ .command = .{ .launch = .{ .source = 1, .destination = 0 } }, .reason = error.SourceNotOwnedByIssuer },
        .{ .command = .{ .launch = .{ .source = 0, .destination = 3 } }, .reason = error.InsufficientEnergy },
        .{ .command = .{ .tune_field = .{ .world = 7, .field = .atmosphere, .adjustment = .increase } }, .reason = error.InvalidWorld },
        .{ .command = .{ .tune_field = .{ .world = 1, .field = .atmosphere, .adjustment = .increase } }, .reason = error.SourceNotOwnedByIssuer },
        .{ .command = .{ .tune_field = .{ .world = 0, .field = .atmosphere, .adjustment = .increase } }, .reason = error.InsufficientEnergy },
        .{ .command = .{ .tune_field = .{ .world = 0, .field = .atmosphere, .adjustment = .decrease } }, .reason = error.FieldAtLimit },
        .{ .command = .{ .tune_field = .{ .world = 0, .field = .atmosphere, .adjustment = .increase } }, .reason = error.FieldAtLimit, .atmosphere = 10 },
    }) |case| {
        const rules = quietRules();
        var campaign = fixture(rules);
        campaign.worlds[0].fields.atmosphere = case.atmosphere;
        var rejected = s.Simulation.fromCampaign(allocator, campaign, rules);
        defer rejected.deinit();
        var control = s.Simulation.fromCampaign(allocator, campaign, rules);
        defer control.deinit();
        const before = rejected.canonicalFingerprint();
        try std.testing.expectError(case.reason, rejected.previewCommand(case.command));
        try expectEqual(before, rejected.canonicalFingerprint());
        try rejected.enqueueNext(0, case.command);
        var report = try rejected.step();
        defer report.deinit(allocator);
        var unchanged = try control.step();
        defer unchanged.deinit(allocator);
        try expectEqualDeep(s.CommandResult{ .rejected = .{ .sequence = 0, .reason = case.reason } }, report.command_results.items[0]);
        try expectEqual(rejected.canonicalFingerprint(), control.canonicalFingerprint());
    }
}

test "RulesV1 lowering and raising a field cannot create free energy" {
    const rules = quietRules();
    var campaign = fixture(rules);
    campaign.worlds[0].fields.topology = 3;
    campaign.worlds[0].energy = 80_000;
    var tuned = s.Simulation.fromCampaign(allocator, campaign, rules);
    defer tuned.deinit();
    var control = s.Simulation.fromCampaign(allocator, campaign, rules);
    defer control.deinit();
    try tuned.enqueueNext(0, .{ .tune_field = .{ .world = 0, .field = .topology, .adjustment = .decrease } });
    try tuned.enqueueNext(1, .{ .tune_field = .{ .world = 0, .field = .topology, .adjustment = .increase } });
    var report = try tuned.step();
    defer report.deinit(allocator);
    var unchanged = try control.step();
    defer unchanged.deinit(allocator);
    try expectEqual(control.state.worlds[0].energy - 10_000, tuned.state.worlds[0].energy);
    try expectEqualDeep(control.state.worlds[0].fields, tuned.state.worlds[0].fields);
    try expectEqual(control.state.worlds[0].defense, tuned.state.worlds[0].defense);
    try expectEqual(@as(usize, 2), report.command_results.items.len);
}

test "RulesV1 sequence order determines spend and rejected intermediate commands consume no fleet ID" {
    const rules = quietRules();
    var campaign = fixture(rules);
    campaign.worlds[0].energy = 80_000;
    var simulation = s.Simulation.fromCampaign(allocator, campaign, rules);
    defer simulation.deinit();
    try simulation.enqueueNext(2, .{ .launch = .{ .source = 0, .destination = 3 } });
    try simulation.enqueueNext(1, .{ .launch = .{ .source = 0, .destination = 0 } });
    try simulation.enqueueNext(0, .{ .launch = .{ .source = 0, .destination = 4 } });
    var report = try simulation.step();
    defer report.deinit(allocator);
    try expectEqualDeep(s.CommandResult{ .accepted = 0 }, report.command_results.items[0]);
    try expectEqualDeep(s.CommandResult{ .rejected = .{ .sequence = 1, .reason = error.SameSourceAndTarget } }, report.command_results.items[1]);
    try expectEqualDeep(s.CommandResult{ .accepted = 2 }, report.command_results.items[2]);
    try expectEqual(@as(u8, 4), simulation.state.fleets.items[0].destination);
    try expectEqual(@as(u64, 40_000), simulation.state.fleets.items[0].strength);
    try expectEqual(@as(u64, 0), simulation.state.fleets.items[0].id);
    try expectEqual(@as(u8, 3), simulation.state.fleets.items[1].destination);
    try expectEqual(@as(u64, 20_000), simulation.state.fleets.items[1].strength);
    try expectEqual(@as(u64, 1), simulation.state.fleets.items[1].id);
    try expectEqual(@as(u64, 20_000), simulation.state.worlds[0].energy);
}

test "RulesV1 hazard duration is half open and expiry precedes a coincident next start" {
    var rules = quietRules();
    rules.hazard_first_tick = 3;
    rules.hazard_period_ticks = 3;
    rules.hazard_duration_ticks = 3;
    var simulation = s.Simulation.fromCampaign(allocator, fixture(rules), rules);
    defer simulation.deinit();
    try simulation.stepN(3);
    try expect(simulation.state.active_hazard == null);
    var start = try simulation.step();
    defer start.deinit(allocator);
    const first = simulation.state.active_hazard.?;
    try expectEqual(@as(u64, 3), first.start);
    try expectEqual(@as(u64, 6), first.end);
    try expectEqual(m.HazardKind.ion_storm, first.kind);
    try expectEqual(@as(usize, 1), start.events.items.len);
    try simulation.stepN(2);
    try expectEqualDeep(first, simulation.state.active_hazard.?);
    var transition = try simulation.step();
    defer transition.deinit(allocator);
    try expectEqual(@as(usize, 2), transition.events.items.len);
    try expectEqualDeep(s.GameEvent{ .hazard_ended = .ion_storm }, transition.events.items[0]);
    const next = simulation.state.active_hazard.?;
    try expectEqualDeep(s.GameEvent{ .hazard_started = next }, transition.events.items[1]);
    try expectEqual(m.HazardKind.gravity_tide, next.kind);
    try expectEqual(@as(u64, 6), next.start);
    try expectEqual(@as(u64, 9), next.end);
    try expectEqual(@as(u64, 2), simulation.state.next_hazard_index);
}

test "RulesV1 hazard has no presence in the gap after its exact end boundary" {
    var rules = quietRules();
    rules.hazard_first_tick = 3;
    rules.hazard_period_ticks = 11;
    rules.hazard_duration_ticks = 3;
    var simulation = s.Simulation.fromCampaign(allocator, fixture(rules), rules);
    defer simulation.deinit();
    try simulation.stepN(6);
    try expect(simulation.state.active_hazard != null);
    var end = try simulation.step();
    defer end.deinit(allocator);
    try expectEqualDeep(s.GameEvent{ .hazard_ended = .ion_storm }, end.events.items[0]);
    try expect(simulation.state.active_hazard == null);
    try simulation.stepN(7);
    try expectEqual(@as(u64, 14), simulation.state.next_tick);
    try expect(simulation.state.active_hazard == null);
    var repeat = try simulation.step();
    defer repeat.deinit(allocator);
    try expectEqual(m.HazardKind.gravity_tide, simulation.state.active_hazard.?.kind);
}

fn allocationFailureFixture(failing: std.mem.Allocator) !void {
    var rules = quietRules();
    rules.hazard_first_tick = 0;
    var campaign = fixture(rules);
    campaign.worlds[0].energy = 80_000;
    var simulation = s.Simulation.fromCampaign(failing, campaign, rules);
    defer simulation.deinit();
    const before = simulation.canonicalFingerprint();
    const first: m.CommandEnvelope = .{ .tick = 0, .sequence = 2, .command = .{ .launch = .{ .source = 0, .destination = 3 } } };
    simulation.enqueue(first) catch |err| {
        try expectEqual(before, simulation.canonicalFingerprint());
        try expectEqual(@as(usize, 0), simulation.pending.items.len);
        return err;
    };
    const second: m.CommandEnvelope = .{ .tick = 0, .sequence = 1, .command = .{ .tune_field = .{ .world = 0, .field = .topology, .adjustment = .increase } } };
    simulation.enqueue(second) catch |err| {
        try expectEqual(before, simulation.canonicalFingerprint());
        try expectEqualDeep(&[_]m.CommandEnvelope{first}, simulation.pending.items);
        return err;
    };
    var report = simulation.step() catch |err| {
        try expectEqual(before, simulation.canonicalFingerprint());
        try expectEqualDeep(&[_]m.CommandEnvelope{ first, second }, simulation.pending.items);
        try expectEqual(@as(u64, 0), simulation.state.next_tick);
        try expectEqual(@as(u64, 0), simulation.state.next_fleet_id);
        try expect(simulation.state.active_hazard == null);
        return err;
    };
    defer report.deinit(failing);
    try expectEqual(@as(u64, 1), simulation.state.next_tick);
    try expectEqual(@as(usize, 0), simulation.pending.items.len);
    try expectEqual(@as(usize, 1), simulation.state.fleets.items.len);
    try expectEqual(@as(u64, 30_000), simulation.state.fleets.items[0].strength);
    try expect(simulation.state.active_hazard != null);
}

test "RulesV1 every enqueue and tick allocation failure preserves authority and pending queue without leaks" {
    try std.testing.checkAllAllocationFailures(allocator, allocationFailureFixture, .{});
}
