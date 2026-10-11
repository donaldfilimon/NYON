const std = @import("std");
const nyon = @import("nyon");
const m = nyon.model;
const Simulation = nyon.simulation.Simulation;
const store = nyon.classic_store;

pub fn main(init: std.process.Init) !void {
    const args = try init.minimal.args.toSlice(init.arena.allocator());
    var buffer: [8192]u8 = undefined;
    var output = std.Io.File.stdout().writerStreaming(init.io, &buffer);
    const out = &output.interface;
    if (args.len == 1 or (args.len == 2 and std.mem.eql(u8, args[1], "help"))) {
        try out.writeAll("NYON Zig authority\n  trace SEED_HEX TICKS [scripted]\n  play [SEED_HEX]\n  scenario-store-load PROFILE_DIR\n  scenario-store-save PROFILE_DIR SCENARIO_JSON\n  preferences-store-load PROFILE_DIR\n  preferences-store-save PROFILE_DIR PREFERENCES_JSON\n\nPROFILE_DIR must be nonempty and absolute; store probes are explicit replacement requests.\nPlay commands: state, step [TICKS], launch SOURCE TARGET,\n  tune WORLD atmosphere|hydrosphere|topology increase|decrease, quit\n");
        try out.writeAll("  surface-neighbors SIDE CELL_ID (east west north south IDs)\n");
        try out.writeAll("  planet-economy-demo [--seed 2] [--side 4] [--budget 1..4096] [--story combined|creator|ruler-start|policy-replay]\n");
    } else if (std.mem.eql(u8, args[1], "surface-neighbors")) {
        if (args.len != 4) return error.InvalidArguments;
        const topology = try nyon.surface.Topology.init(try std.fmt.parseInt(u16, args[2], 10));
        const cell = try topology.fromId(try std.fmt.parseInt(u64, args[3], 10));
        try out.print("{d}\t{d}\t{d}\t{d}\n", .{
            try topology.cellId(try topology.neighbor(cell, .east)),
            try topology.cellId(try topology.neighbor(cell, .west)),
            try topology.cellId(try topology.neighbor(cell, .north)),
            try topology.cellId(try topology.neighbor(cell, .south)),
        });
    } else if (std.mem.eql(u8, args[1], "planet-economy-demo")) {
        nyon.planet_economy_cli.run(init.gpa, args[2..], out) catch |failure| {
            try out.flush();
            return failure;
        };
    } else if (isStoreProbe(args[1])) {
        storeProbe(init.io, init.gpa, out, args[1..]) catch |failure| {
            if (failure == error.OutOfMemory) try out.writeAll("ERR\tRESOURCE\tOutOfMemory\n");
            try out.flush();
            return failure;
        };
    } else if (std.mem.eql(u8, args[1], "scenario-default") and args.len == 3) {
        const seed = try std.fmt.parseInt(u64, args[2], 16);
        const validated = try nyon.scenario.generatedValidated(seed, .{});
        try writeScenario(init.gpa, out, &validated);
    } else if (std.mem.eql(u8, args[1], "scenario-json") and args.len == 3) {
        const validated = nyon.scenario.decode(init.gpa, args[2]) catch |failure| {
            if (failure == error.OutOfMemory) return failure;
            try out.print("ERR\t{s}\n", .{@errorName(failure)});
            try out.flush();
            return;
        };
        try writeScenario(init.gpa, out, &validated);
    } else if (std.mem.eql(u8, args[1], "preferences-default") and args.len == 2) {
        try writePreferences(init.gpa, out, .{});
    } else if (std.mem.eql(u8, args[1], "preferences-json") and args.len == 3) {
        const decoded = nyon.preferences.decode(init.gpa, args[2]) catch |failure| {
            if (failure == error.OutOfMemory) return failure;
            try out.print("ERR\t{s}\n", .{@errorName(failure)});
            try out.flush();
            return;
        };
        try writePreferences(init.gpa, out, decoded);
    } else if (std.mem.eql(u8, args[1], "trace")) {
        if (args.len < 4 or args.len > 5) return error.InvalidArguments;
        const seed = try std.fmt.parseInt(u64, args[2], 16);
        const ticks = try std.fmt.parseInt(u32, args[3], 10);
        if (ticks > 100_000) return error.InvalidArguments;
        const scripted = args.len == 5;
        if (scripted and !std.mem.eql(u8, args[4], "scripted")) return error.InvalidArguments;
        var simulation = Simulation.generated(init.gpa, seed, .{});
        defer simulation.deinit();
        try out.print("{d}\t{X:0>16}\t-\t-\n", .{ simulation.state.next_tick, simulation.canonicalFingerprint() });
        for (0..ticks) |iteration| {
            if (scripted) try enqueueScript(&simulation, iteration);
            var report = try simulation.step();
            defer report.deinit(init.gpa);
            try writeTrace(out, &simulation, &report);
        }
    } else if (std.mem.eql(u8, args[1], "play")) {
        if (args.len > 3) return error.InvalidArguments;
        const seed = if (args.len == 3) try std.fmt.parseInt(u64, args[2], 16) else m.DEFAULT_SEED;
        try play(init.io, init.gpa, seed, out);
    } else return error.InvalidArguments;
    try out.flush();
}

fn isStoreProbe(mode: []const u8) bool {
    return std.mem.eql(u8, mode, "scenario-store-load") or std.mem.eql(u8, mode, "scenario-store-save") or
        std.mem.eql(u8, mode, "preferences-store-load") or std.mem.eql(u8, mode, "preferences-store-save");
}

fn storeProbe(io: std.Io, allocator: std.mem.Allocator, out: *std.Io.Writer, args: []const []const u8) !void {
    const save = std.mem.endsWith(u8, args[0], "-save");
    if (args.len != (if (save) @as(usize, 3) else 2)) {
        try out.writeAll("ERR\tARGUMENT\tInvalidArguments\n");
        return error.InvalidArguments;
    }
    const record: store.Record = if (std.mem.startsWith(u8, args[0], "scenario-")) .scenario else .preferences;
    const platform: store.NativePlatform = switch (@import("builtin").os.tag) {
        .macos => .macos,
        .windows => .windows,
        else => .linux,
    };
    // Construct first, then borrow raw(); never move native while raw is live.
    var native = store.NativeStore.fromProfile(allocator, io, record, platform, args[1]) catch |failure| {
        if (failure == error.OutOfMemory) return failure;
        try out.writeAll("ERR\tPROFILE\tInvalidProfileDirectory\n");
        return failure;
    };
    defer native.deinit();
    if (record == .scenario) {
        if (!save) {
            var loaded = try nyon.classic_store_load.load(allocator, native.raw());
            defer loaded.deinit(allocator);
            switch (loaded) {
                .absent => try out.writeAll("ABSENT\n"),
                .candidate => |*value| try writeScenario(allocator, out, &value.scenario),
                .failure => |failure| {
                    switch (failure) {
                        .store => |value| try writeStoreFailure(out, "ERR", value),
                        .codec => |value| try writeScenarioFailure(out, value),
                        .admission => |value| try out.print("ERR\tADMISSION\t{s}\n", .{@errorName(value)}),
                    }
                    return error.StoreProbeFailed;
                },
            }
        } else {
            const decoded = switch (try nyon.scenario.decodeDetailed(allocator, args[2])) {
                .scenario => |value| value,
                .failure => |failure| {
                    try writeScenarioFailure(out, failure);
                    return error.StoreProbeFailed;
                },
            };
            var candidate = Simulation.fromScenario(allocator, &decoded) catch |failure| {
                try out.print("ERR\tADMISSION\t{s}\n", .{@errorName(failure)});
                return error.StoreProbeFailed;
            };
            defer candidate.deinit();
            const encoded = try nyon.scenario.encode(allocator, &decoded);
            defer allocator.free(encoded);
            var saved = try native.raw().save(allocator, encoded);
            defer saved.deinit(allocator);
            if (saved == .failure) {
                try writeStoreFailure(out, "ERR", saved.failure);
                return error.StoreProbeFailed;
            }
            try out.print("OK\t{X:0>16}\t{s}\n", .{ decoded.fingerprint, encoded });
        }
    } else {
        var canonical_preferences: ?[]u8 = null;
        defer if (canonical_preferences) |bytes| allocator.free(bytes);
        var outcome = if (save) blk: {
            const decoded = switch (try nyon.preferences.decodeDetailed(allocator, args[2])) {
                .preferences => |value| value,
                .failure => |failure| {
                    try writePreferencesFailure(out, "ERR", failure);
                    return error.StoreProbeFailed;
                },
            };
            // Prepare success delivery before the only durable mutation.
            canonical_preferences = try nyon.preferences.encode(allocator, decoded);
            break :blk try store.saveOrDefault(allocator, native.raw(), decoded);
        } else try store.loadOrDefault(allocator, native.raw());
        defer outcome.deinit(allocator);
        if (outcome.failure) |failure| {
            const status = if (save) "ERR" else "DEFAULT";
            switch (failure) {
                .store => |value| try writeStoreFailure(out, status, value),
                .codec => |value| try writePreferencesFailure(out, status, value),
            }
            if (save) return error.StoreProbeFailed;
        }
        if (canonical_preferences) |bytes| {
            try out.print("OK\t{s}\n", .{bytes});
        } else try writePreferences(allocator, out, outcome.preferences);
    }
}

fn writeStoreFailure(out: *std.Io.Writer, status: []const u8, failure: store.StoreFailure) !void {
    try out.print("{s}\tSTORE\t{s}\trecord={s}\tcause={s}\tmax_bytes={d}\tcleanup={s}\tmessage={s}\n", .{
        status,                                                @tagName(failure.kind), @tagName(failure.record),
        if (failure.cause) |value| @errorName(value) else "-", failure.max_bytes,      if (failure.cleanup_error) |value| @errorName(value) else "-",
        failure.message orelse "-",
    });
}

fn writeScenarioFailure(out: *std.Io.Writer, failure: nyon.scenario.CodecFailure) !void {
    try out.print("ERR\tCODEC\t{s}", .{@errorName(failure.compactError())});
    switch (failure) {
        .json => |cause| try out.print("\tcause={s}", .{@errorName(cause)}),
        .canonical_number => |value| try out.print("\tfield={s}\tkind={s}", .{ value.field, value.kind }),
        .validation => |report| for (report.issues()) |issue| {
            try out.print("\t{s}:{s}:{s}:{s}", .{ @tagName(issue.severity), @tagName(issue.code), issue.field.asSlice(), issue.message.asSlice() });
        },
        .too_large => try out.print("\tmax_bytes={d}", .{nyon.scenario.MAX_JSON_BYTES}),
        .version => {},
    }
    try out.writeByte('\n');
}

fn writePreferencesFailure(out: *std.Io.Writer, status: []const u8, failure: nyon.preferences.CodecFailure) !void {
    try out.print("{s}\tCODEC\t", .{status});
    switch (failure) {
        .too_large => try out.print("TooLarge\tmax_bytes={d}", .{nyon.preferences.MAX_JSON_BYTES}),
        .json => |cause| try out.print("Json\tcause={s}", .{@errorName(cause)}),
        .version => try out.writeAll("Version"),
        .ui_scale => |value| try out.print("UiScale\tvalue={d}", .{value}),
    }
    try out.writeByte('\n');
}

fn writeScenario(allocator: std.mem.Allocator, out: *std.Io.Writer, validated: *const nyon.scenario.ScenarioV1) !void {
    const encoded = try nyon.scenario.encode(allocator, validated);
    defer allocator.free(encoded);
    try out.print("OK\t{X:0>16}\t{s}\n", .{ validated.fingerprint, encoded });
}

fn writePreferences(allocator: std.mem.Allocator, out: *std.Io.Writer, preferences: nyon.preferences.UserPreferencesV1) !void {
    const encoded = try nyon.preferences.encode(allocator, preferences);
    defer allocator.free(encoded);
    try out.print("OK\t{s}\n", .{encoded});
}

fn enqueueScript(sim: *Simulation, iteration: usize) !void {
    if (sim.state.phase == .finished) return;
    if (iteration % 173 == 0) {
        // Submit out of order to exercise the canonical sequence ordering.
        try sim.enqueueNext(2, .{ .launch = .{ .source = 0, .destination = @intCast((iteration / 173) % 6 + 1) } });
        try sim.enqueueNext(1, .{ .tune_field = .{ .world = 0, .field = .atmosphere, .adjustment = .increase } });
        try sim.enqueueNext(0, .{ .launch = .{ .source = 0, .destination = 0 } });
    }
    if (iteration % 257 == 0) {
        try sim.enqueueNext(3, .{ .tune_field = .{ .world = 0, .field = .topology, .adjustment = .decrease } });
    }
}

fn writeTrace(out: *std.Io.Writer, sim: *const Simulation, report: *const nyon.simulation.TickReport) !void {
    try out.print("{d}\t{X:0>16}\t", .{ sim.state.next_tick, sim.canonicalFingerprint() });
    if (report.command_results.items.len == 0) try out.writeByte('-');
    for (report.command_results.items) |result| switch (result) {
        .accepted => |sequence| try out.print("A{d},", .{sequence}),
        .rejected => |rejected| try out.print("R{d}:{s},", .{ rejected.sequence, @errorName(rejected.reason) }),
    };
    try out.writeByte('\t');
    if (report.events.items.len == 0) try out.writeByte('-');
    for (report.events.items) |event| switch (event) {
        .fleet_launched => |id| try out.print("L{d},", .{id}),
        .world_captured => |capture| try out.print("C{d}:{d},", .{ capture.world, ownerTag(capture.owner) }),
        .hazard_started => |hazard| try out.print("H{d}:{d}:{d}:{d}:{d},", .{ hazard.event_index, @backingInt(hazard.kind), @backingInt(hazard.affected_field), hazard.start, hazard.end }),
        .hazard_ended => |kind| try out.print("E{d},", .{@backingInt(kind)}),
        .campaign_finished => |outcome| switch (outcome) {
            .faction_victory => |victory| try out.print("V{d},", .{ownerTag(victory.winner)}),
            .player_eliminated => try out.writeAll("P,"),
        },
    };
    try out.writeByte('\n');
}

fn ownerTag(owner: ?m.Faction) u8 {
    return if (owner) |value| switch (value) {
        .union_faction => 1,
        .helix => 2,
        .choir => 3,
    } else 0;
}

fn play(io: std.Io, allocator: std.mem.Allocator, seed: u64, out: *std.Io.Writer) !void {
    var sim = Simulation.generated(allocator, seed, .{});
    defer sim.deinit();
    var input_buffer: [4096]u8 = undefined;
    var input = std.Io.File.stdin().readerStreaming(io, &input_buffer);
    var sequence: u64 = 0;
    try out.writeAll("NYON Classic RulesV1 — type help for commands.\n");
    try writeState(out, &sim);
    while (true) {
        try out.writeAll("> ");
        try out.flush();
        const line = input.interface.takeDelimiter('\n') catch |err| switch (err) {
            error.StreamTooLong => {
                try out.writeAll("Command exceeds 4096 bytes.\n");
                try out.flush();
                return error.InvalidArguments;
            },
            else => return err,
        } orelse break;
        var tokens = std.mem.tokenizeAny(u8, line, " \t\r");
        const verb = tokens.next() orelse continue;
        if (std.mem.eql(u8, verb, "quit")) break;
        if (std.mem.eql(u8, verb, "state")) {
            if (tokens.next() != null) {
                try out.writeAll("state takes no arguments.\n");
                continue;
            }
            try writeState(out, &sim);
        } else if (std.mem.eql(u8, verb, "help")) {
            try out.writeAll("state | step [TICKS] | launch SOURCE TARGET | tune WORLD FIELD increase|decrease | quit\n");
        } else if (std.mem.eql(u8, verb, "step")) {
            const count = if (tokens.next()) |value| std.fmt.parseInt(u32, value, 10) catch {
                try out.writeAll("Invalid tick count.\n");
                continue;
            } else 1;
            if (tokens.next() != null or count > 100_000) {
                try out.writeAll("step accepts 0 through 100000 ticks.\n");
                continue;
            }
            for (0..count) |_| {
                var report = try sim.step();
                defer report.deinit(allocator);
                if (report.command_results.items.len > 0 or report.events.items.len > 0) try writeTrace(out, &sim, &report);
            }
            try writeState(out, &sim);
        } else if (std.mem.eql(u8, verb, "launch") or std.mem.eql(u8, verb, "tune")) {
            const command = parseCommand(verb, &tokens) catch {
                try out.writeAll("Invalid command. Use help for syntax.\n");
                continue;
            };
            _ = sim.previewCommand(command) catch |reason| {
                try out.print("Rejected: {s}\n", .{@errorName(reason)});
                continue;
            };
            sim.enqueueNext(sequence, command) catch |reason| {
                try out.print("Cannot queue: {s}\n", .{@errorName(reason)});
                continue;
            };
            try out.print("Queued command {d} at tick {d}.\n", .{ sequence, sim.state.next_tick });
            sequence +|= 1;
        } else try out.writeAll("Unknown command. Use help for syntax.\n");
    }
}

fn parseCommand(verb: []const u8, tokens: *std.mem.TokenIterator(u8, .any)) !m.GameCommand {
    const world = try std.fmt.parseInt(u8, tokens.next() orelse return error.InvalidArguments, 10);
    if (std.mem.eql(u8, verb, "launch")) {
        const destination = try std.fmt.parseInt(u8, tokens.next() orelse return error.InvalidArguments, 10);
        if (tokens.next() != null) return error.InvalidArguments;
        return .{ .launch = .{ .source = world, .destination = destination } };
    }
    const field = std.meta.stringToEnum(m.FieldKind, tokens.next() orelse return error.InvalidArguments) orelse return error.InvalidArguments;
    const adjustment = std.meta.stringToEnum(m.FieldAdjustment, tokens.next() orelse return error.InvalidArguments) orelse return error.InvalidArguments;
    if (tokens.next() != null) return error.InvalidArguments;
    return .{ .tune_field = .{ .world = world, .field = field, .adjustment = adjustment } };
}

fn writeState(out: *std.Io.Writer, sim: *const Simulation) !void {
    try out.print("Tick {d}, digest {X:0>16}, {d} fleets, {d} queued commands\n", .{ sim.state.next_tick, sim.canonicalFingerprint(), sim.state.fleets.items.len, sim.pending.items.len });
    for (&sim.state.worlds) |*world| {
        try out.print("{d} {s}: owner={d} energy={d} defense={d} fields={d}/{d}/{d}\n", .{ world.id, world.name.asSlice(), ownerTag(world.owner), world.energy, world.defense, world.fields.atmosphere, world.fields.hydrosphere, world.fields.topology });
    }
    if (sim.state.phase == .finished) try out.writeAll("Campaign finished.\n");
}
