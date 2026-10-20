const std = @import("std");
const sg = @import("root.zig");
const f = @import("fixtures.zig");
const h = @import("test_support.zig");
test "owned ready take reopens lane and invalidates token" {
    var setup = try sg.Setup.init(std.testing.allocator, 1);
    defer setup.deinit();
    const token = try setup.beginCreate(f.small_config);
    try std.testing.expectError(error.Busy, setup.beginCreate(f.small_config));
    try std.testing.expectError(error.NotReady, setup.take(token));
    const before = try setup.stats(token);
    _ = try setup.poll(token, 0);
    try std.testing.expectEqualDeep(before, try setup.stats(token));
    try h.finish(&setup, token, 1);
    var manifest = try setup.take(token);
    defer manifest.deinit();
    try std.testing.expectError(error.StaleJob, setup.take(token));
    try std.testing.expectEqualSlices(u8, &f.small_digest, &(try manifest.digest()));
    const replacement = try setup.beginCreate(f.small_config);
    try std.testing.expect(replacement.id > token.id);
    try std.testing.expectError(error.StaleJob, setup.poll(token, 1));
    try setup.cancel(replacement);
    while (true) switch (try setup.poll(replacement, 1)) {
        .canceled => break,
        .pending => {},
        else => return error.InvariantViolation,
    };
    try setup.release(replacement);
    try std.testing.expectEqualSlices(u8, &f.small_digest, &(try manifest.digest()));
}

test "canonical creation budgets literal bytes and maximum witnesses" {
    for ([_]u16{ 1, 7, 256, 4096 }) |budget| {
        var setup = try sg.Setup.init(std.testing.allocator, budget);
        defer setup.deinit();
        for ([_]sg.Config{ f.small_config, .{ .systems = 64, .planets = 512, .civilizations = 16 }, .{ .seed = std.math.maxInt(u64), .systems = 64, .planets = 512, .civilizations = 16 } }, 0..) |config, index| {
            const token = try setup.beginCreate(config);
            try h.finish(&setup, token, budget);
            try std.testing.expectEqual(@as(u64, if (index == 0) 30 else 1959), (try setup.stats(token)).total_units);
            var manifest = try setup.take(token);
            defer manifest.deinit();
            var bytes: [16384]u8 = undefined;
            const len = try h.collectWire(&manifest, &bytes);
            const expected = switch (index) {
                0 => f.small_digest,
                1 => f.maximum_zero_digest,
                else => f.maximum_max_digest,
            };
            try std.testing.expectEqualSlices(u8, &expected, &(try manifest.digest()));
            try std.testing.expectEqualDeep(config, try manifest.config());
            if (index == 0) try std.testing.expectEqualSlices(u8, &f.small_wire, bytes[0..len]) else try std.testing.expectEqual(@as(u16, 11455), len);
        }
    }
}

test "hash charges complete padded blocks including two block padding" {
    const hash = @import("hash.zig");
    var bytes: [177]u8 = undefined;
    for (&bytes, 0..) |*b, i| b.* = @truncate(i);
    for ([_]usize{ 0, 1, 55, 56, 63, 64, 65, 119, 120, 177 }) |len| {
        var scratch: sg.storage.Scratch = undefined;
        scratch.sha = std.crypto.hash.sha2.Sha256.init(.{});
        scratch.block_cursor = 0;
        const blocks = (len + 9 + 63) / 64;
        for (0..blocks) |i| {
            try std.testing.expectEqual(i + 1 == blocks, try hash.advance(&scratch, bytes[0..len]));
            try std.testing.expectEqual(i + 1, scratch.block_cursor);
            try std.testing.expectEqual(@as(u8, 0), scratch.sha.buf_len);
        }
        var expected: [32]u8 = undefined;
        std.crypto.hash.sha2.Sha256.hash(bytes[0..len], &expected, .{});
        try std.testing.expectEqualSlices(u8, &expected, &hash.finish(&scratch));
        try std.testing.expectError(error.InvariantViolation, hash.advance(&scratch, bytes[0..len]));
    }
    for ([_][]const u8{ "", "abc" }, [_][32]u8{
        comptime f.decodeHex("e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"),
        comptime f.decodeHex("ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"),
    }) |message, expected| {
        var scratch: sg.storage.Scratch = undefined;
        scratch.sha = std.crypto.hash.sha2.Sha256.init(.{});
        scratch.block_cursor = 0;
        try std.testing.expect(try hash.advance(&scratch, message));
        try std.testing.expectEqualSlices(u8, &expected, &hash.finish(&scratch));
    }
}

test "copied pages bounds terminal shutdown and no query allocations" {
    var counter = h.Counter.init(std.testing.allocator);
    var setup = try sg.Setup.init(counter.allocator(), 1);
    defer setup.deinit();
    const token = try setup.beginCreate(f.small_config);
    try h.finish(&setup, token, 4096);
    var manifest = try setup.take(token);
    defer manifest.deinit();
    const attempts = counter.attempts;
    var systems: [65]sg.System = undefined;
    var planets: [4]sg.Planet = undefined;
    var civs: [2]sg.Civilization = undefined;
    var wires: [10]sg.WireRecord = undefined;
    try std.testing.expectEqualDeep(sg.Page{ .written = 1, .next = 1 }, try manifest.systems(0, systems[0..1]));
    try std.testing.expectEqualDeep(sg.Page{ .written = 1, .next = null }, try manifest.systems(1, systems[0..1]));
    try std.testing.expectEqualDeep(sg.Page{ .written = 0, .next = null }, try manifest.systems(2, systems[0..1]));
    try std.testing.expectError(error.InvalidConfig, manifest.systems(3, systems[0..1]));
    try std.testing.expectError(error.InvalidConfig, manifest.systems(0, systems[0..0]));
    try std.testing.expectError(error.InvalidConfig, manifest.systems(0, &systems));
    _ = try manifest.systems(0, systems[0..2]);
    _ = try manifest.planets(0, &planets);
    _ = try manifest.civilizations(0, &civs);
    _ = try manifest.wireRecords(0, &wires);
    try std.testing.expectEqual(@as(u16, 47), wires[0].len);
    try std.testing.expectEqual(@as(u16, 16), wires[1].len);
    try std.testing.expectEqual(@as(u16, 20), wires[3].len);
    try std.testing.expectEqual(@as(u16, 9), wires[7].len);
    systems[0].x = 0;
    planets[0].side = 0;
    civs[0].founding_planet = @fromBackingInt(0);
    wires[0].bytes[0] = 0;
    try std.testing.expectEqualSlices(u8, &f.small_digest, &(try manifest.digest()));
    _ = try manifest.systems(0, systems[0..1]);
    try std.testing.expectEqual(@as(i32, 889), systems[0].x);
    try std.testing.expectEqual(attempts, counter.attempts);
    manifest.deinit();
    manifest.deinit();
    try std.testing.expectError(error.Shutdown, manifest.config());
    try std.testing.expectError(error.Shutdown, manifest.digest());
    try std.testing.expectError(error.Shutdown, manifest.systems(999, systems[0..0]));
    try std.testing.expectError(error.Shutdown, manifest.planets(0, &planets));
    try std.testing.expectError(error.Shutdown, manifest.civilizations(0, &civs));
    try std.testing.expectError(error.Shutdown, manifest.wireRecords(0, &wires));
}

test "token terminal table admission precedence and zero budget immutability" {
    try std.testing.expectError(error.InvalidConfig, sg.Setup.init(std.testing.allocator, 0));
    var setup = try sg.Setup.init(std.testing.allocator, 10);
    defer setup.deinit();
    try std.testing.expectError(error.InvalidVersion, setup.beginCreate(.{ .version = 2, .systems = 0 }));
    const token = try setup.beginCreate(f.small_config);
    try std.testing.expectEqual(@as(u64, 1), token.id);
    try std.testing.expectEqual(@as(u64, 1), token.generation);
    for ([_]sg.Token{
        .{ .owner_namespace = 11, .id = token.id, .generation = token.generation },
        .{ .owner_namespace = 10, .id = token.id + 1, .generation = token.generation },
        .{ .owner_namespace = 10, .id = token.id, .generation = token.generation + 1 },
    }) |stale| {
        const before = try setup.stats(token);
        try std.testing.expectError(error.StaleJob, setup.poll(stale, 4097));
        try std.testing.expectError(error.StaleJob, setup.cancel(stale));
        try std.testing.expectError(error.StaleJob, setup.take(stale));
        try std.testing.expectError(error.StaleJob, setup.release(stale));
        try std.testing.expectError(error.StaleJob, setup.stats(stale));
        try std.testing.expectEqualDeep(before, try setup.stats(token));
    }
    for (0..30) |_| {
        const before = try setup.stats(token);
        _ = try setup.poll(token, 0);
        try std.testing.expectError(error.InvalidBudget, setup.poll(token, 4097));
        try std.testing.expectError(error.Busy, setup.beginCreate(.{ .version = 2 }));
        try std.testing.expectError(error.NotReady, setup.release(token));
        try std.testing.expectEqualDeep(before, try setup.stats(token));
        _ = try setup.poll(token, 1);
    }
    const ready = try setup.stats(token);
    _ = try setup.poll(token, 0);
    try std.testing.expectEqualDeep(ready, try setup.stats(token));
    try std.testing.expectError(error.Busy, setup.beginCreate(f.small_config));
    try setup.release(token);
    try std.testing.expectError(error.StaleJob, setup.take(token));
    try std.testing.expectError(error.StaleJob, setup.release(token));
    try std.testing.expectError(error.StaleJob, setup.cancel(token));
    const next = try setup.beginCreate(f.small_config);
    try std.testing.expectEqual(token.id + 1, next.id);
    setup.deinit();
    setup.deinit();
    try std.testing.expectError(error.Shutdown, setup.beginCreate(.{ .version = 2 }));
    try std.testing.expectError(error.Shutdown, setup.poll(token, 4097));
    try std.testing.expectError(error.Shutdown, setup.cancel(next));
    try std.testing.expectError(error.Shutdown, setup.take(next));
    try std.testing.expectError(error.Shutdown, setup.release(next));
    try std.testing.expectError(error.Shutdown, setup.stats(next));
    setup = try sg.Setup.init(std.testing.allocator, 11);
    const fresh = try setup.beginCreate(f.small_config);
    try std.testing.expectError(error.StaleJob, setup.poll(token, 1));
    try std.testing.expectError(error.StaleJob, setup.poll(next, 1));
    try setup.cancel(fresh);
}

test "all maximum charged cancellation cuts retire whole allocations preserve resident" {
    var failing = std.testing.FailingAllocator.init(std.testing.allocator, .{});
    var counter = h.Counter.init(failing.allocator());
    var resident_setup = try sg.Setup.init(counter.allocator(), 1);
    defer resident_setup.deinit();
    const resident_token = try resident_setup.beginCreate(f.small_config);
    try h.finish(&resident_setup, resident_token, 4096);
    var resident = try resident_setup.take(resident_token);
    defer resident.deinit();
    const expected = try resident.digest();
    const resident_bytes = @sizeOf(sg.storage.Output);
    for (0..1960) |cut| {
        var setup = try sg.Setup.init(counter.allocator(), cut + 2);
        defer setup.deinit();
        const token = try setup.beginCreate(.{ .systems = 64, .planets = 512, .civilizations = 16 });
        for (0..cut) |_| _ = try h.pollChecked(&setup, token, 1, &counter);
        try std.testing.expectEqual(@as(u64, @intCast(cut)), (try setup.stats(token)).total_units);
        try setup.cancel(token);
        try setup.cancel(token);
        try std.testing.expectError(error.NotReady, setup.release(token));
        try std.testing.expectError(error.NotReady, setup.take(token));
        const before = try setup.stats(token);
        const frees = counter.frees;
        _ = try h.pollChecked(&setup, token, 0, &counter);
        try std.testing.expectEqualDeep(before, try setup.stats(token));
        try std.testing.expectEqual(frees, counter.frees);
        try std.testing.expectEqual(.pending, std.meta.activeTag(try h.pollChecked(&setup, token, 1, &counter)));
        try std.testing.expectEqual(frees + 1, counter.frees);
        try setup.cancel(token);
        _ = try h.pollChecked(&setup, token, 0, &counter);
        try std.testing.expectError(error.NotReady, setup.release(token));
        try std.testing.expectEqual(.canceled, std.meta.activeTag(try h.pollChecked(&setup, token, 1, &counter)));
        try std.testing.expectEqual(frees + 2, counter.frees);
        try std.testing.expectEqual(resident_bytes, counter.live);
        try setup.cancel(token);
        try std.testing.expectEqual(.canceled, std.meta.activeTag(try h.pollChecked(&setup, token, 1, &counter)));
        try std.testing.expectEqual(frees + 2, counter.frees);
        try setup.release(token);
        try std.testing.expectError(error.StaleJob, setup.poll(token, 1));
        const attempts = counter.attempts;
        try std.testing.expectEqualSlices(u8, &expected, &(try resident.digest()));
        try std.testing.expectEqual(attempts, counter.attempts);
        try std.testing.expectEqual(@as(usize, 2 * @sizeOf(sg.storage.Output) + @sizeOf(sg.storage.Scratch)), counter.peak);
        const accounted_peak = counter.peak + 2 * sg.storage.control_envelope_bytes + 2 * sg.storage.output_alignment_pad + sg.storage.scratch_alignment_pad;
        try std.testing.expectEqual(sg.storage.replacementPeakReservation(), accounted_peak);
    }
    resident.deinit();
    try std.testing.expectEqual(@as(usize, 0), counter.live);
    try std.testing.expectEqual(failing.allocations, failing.deallocations);
}

test "replacement take retains old owner and permits either destruction order" {
    for ([_]bool{ false, true }) |old_first| {
        var counter = h.Counter.init(std.testing.allocator);
        var setup = try sg.Setup.init(counter.allocator(), 1);
        defer setup.deinit();
        const first = try setup.beginCreate(f.small_config);
        try h.finish(&setup, first, 256);
        var old = try setup.take(first);
        defer old.deinit();
        const second = try setup.beginCreate(.{ .systems = 64, .planets = 512, .civilizations = 16 });
        try h.finish(&setup, second, 256);
        var new = try setup.take(second);
        defer new.deinit();
        try std.testing.expectEqual(@as(usize, 2 * @sizeOf(sg.storage.Output)), counter.live);
        setup.deinit();
        if (old_first) {
            old.deinit();
            try std.testing.expectEqualSlices(u8, &f.maximum_zero_digest, &(try new.digest()));
            new.deinit();
        } else {
            new.deinit();
            try std.testing.expectEqualSlices(u8, &f.small_digest, &(try old.digest()));
            old.deinit();
        }
        try std.testing.expectEqual(@as(usize, 0), counter.live);
        try std.testing.expectEqual(@as(usize, 4), counter.frees);
    }
}

test "both admission allocation failures clean up and consume reserved pair" {
    for (0..2) |fail_index| {
        var failing = std.testing.FailingAllocator.init(std.testing.allocator, .{ .fail_index = fail_index });
        var counter = h.Counter.init(failing.allocator());
        var setup = try sg.Setup.init(counter.allocator(), 1);
        defer setup.deinit();
        try std.testing.expectError(error.OutOfMemory, setup.beginCreate(f.small_config));
        try std.testing.expectEqual(fail_index + 1, counter.attempts);
        try std.testing.expectEqual(@as(usize, 0), counter.live);
        try std.testing.expectEqual(fail_index, counter.frees);
        try std.testing.expectEqual(failing.allocations, failing.deallocations);
        failing.fail_index = std.math.maxInt(usize);
        const token = try setup.beginCreate(f.small_config);
        try std.testing.expectEqual(@as(u64, 2), token.id);
        try std.testing.expectEqual(@as(u64, 2), token.generation);
        try h.finish(&setup, token, 4096);
        const frees = counter.frees;
        try setup.release(token);
        try std.testing.expectEqual(frees + 2, counter.frees);
        try std.testing.expectEqual(@as(usize, 0), counter.live);
    }
}

test "retained resident survives either replacement allocation failure" {
    for ([_]usize{ 2, 3 }) |fail_index| {
        var failing = std.testing.FailingAllocator.init(std.testing.allocator, .{ .fail_index = fail_index });
        var counter = h.Counter.init(failing.allocator());
        var setup = try sg.Setup.init(counter.allocator(), 1);
        defer setup.deinit();
        const token = try setup.beginCreate(f.small_config);
        try h.finish(&setup, token, 4096);
        var resident = try setup.take(token);
        defer resident.deinit();
        try std.testing.expectError(error.OutOfMemory, setup.beginCreate(f.small_config));
        try std.testing.expectEqualSlices(u8, &f.small_digest, &(try resident.digest()));
        try std.testing.expectEqual(@as(usize, @sizeOf(sg.storage.Output)), counter.live);
        setup.deinit();
        try std.testing.expectEqualSlices(u8, &f.small_digest, &(try resident.digest()));
        resident.deinit();
        try std.testing.expectEqual(failing.allocations, failing.deallocations);
    }
}

test "shutdown retires only remaining allocations in each lifecycle phase" {
    for (0..33) |cut| {
        var counter = h.Counter.init(std.testing.allocator);
        var setup = try sg.Setup.init(counter.allocator(), 1);
        const token = try setup.beginCreate(f.small_config);
        for (0..@min(cut, 30)) |_| _ = try h.pollChecked(&setup, token, 1, &counter);
        if (cut >= 31) {
            try setup.cancel(token);
            _ = try h.pollChecked(&setup, token, @intCast(cut - 31), &counter);
        }
        setup.deinit();
        setup.deinit();
        try std.testing.expectEqual(@as(usize, 2), counter.frees);
        try std.testing.expectEqual(@as(usize, 0), counter.live);
        try std.testing.expectError(error.Shutdown, setup.poll(token, 0));
    }
}

test "mixed zero and random bounded cadence preserves exact manifest" {
    var counter = h.Counter.init(std.testing.allocator);
    var setup = try sg.Setup.init(counter.allocator(), 1);
    defer setup.deinit();
    const token = try setup.beginCreate(.{ .seed = std.math.maxInt(u64), .systems = 64, .planets = 512, .civilizations = 16 });
    var schedule: u64 = 91;
    var done = false;
    for (0..10000) |_| {
        const before = try setup.stats(token);
        _ = try h.pollChecked(&setup, token, 0, &counter);
        try std.testing.expectEqualDeep(before, try setup.stats(token));
        schedule = sg.generate.mix(schedule);
        const budget: u16 = @intCast(1 + schedule % 37);
        const progress = try h.pollChecked(&setup, token, budget, &counter);
        try std.testing.expect((try setup.stats(token)).units_used <= budget);
        switch (progress) {
            .ready => {
                done = true;
                break;
            },
            .pending => {},
            else => return error.InvariantViolation,
        }
    }
    try std.testing.expect(done);
    try std.testing.expectEqual(@as(u64, 1959), (try setup.stats(token)).total_units);
    var manifest = try setup.take(token);
    defer manifest.deinit();
    try std.testing.expectEqualSlices(u8, &f.maximum_max_digest, &(try manifest.digest()));
}

test "encoder short destinations remain untouched and signed fields use twos complement" {
    const wire = @import("wire.zig");
    var dest: [48]u8 = @splat(0xaa);
    const system: sg.System = .{ .id = @fromBackingInt(1), .x = -1, .y = std.math.minInt(i32), .z = 0 };
    const planet: sg.Planet = .{ .id = @fromBackingInt(1), .system = @fromBackingInt(1), .orbit_index = 0, .terrain_seed = 0, .side = 8 };
    const civ: sg.Civilization = .{ .id = @fromBackingInt(1), .founding_planet = @fromBackingInt(1), .kind = .uninitialized };
    try std.testing.expectError(error.InvalidConfig, wire.encodeHeader(f.small_config, dest[0..46]));
    try std.testing.expectError(error.InvalidConfig, wire.encodeSystem(system, dest[0..15]));
    try std.testing.expectError(error.InvalidConfig, wire.encodePlanet(planet, dest[0..19]));
    try std.testing.expectError(error.InvalidConfig, wire.encodeCivilization(civ, dest[0..8]));
    try std.testing.expectEqualSlices(u8, &@as([48]u8, @splat(0xaa)), &dest);
    try std.testing.expectEqual(@as(u16, 16), try wire.encodeSystem(system, &dest));
    const expected = comptime f.decodeHex("01000000ffffffff0000008000000000");
    try std.testing.expectEqualSlices(u8, &expected, dest[0..16]);
    try std.testing.expectEqual(@as(u8, 0xaa), dest[16]);
}

test "concrete controls and owning handles fit reserved envelope" {
    const Handles = struct { token: sg.Token, manifest: sg.Manifest, progress: sg.Progress };
    sg.storage.assertControlsFit(sg.Setup, Handles);
    try std.testing.expect(@sizeOf(sg.Setup) + @sizeOf(Handles) <= 1024);
    std.debug.print("\nSetup={d} Handles={d} Manifest={d} Output={d} Scratch={d} candidate={d} detached={d} replacement={d}\n", .{
        @sizeOf(sg.Setup),                 @sizeOf(Handles),                 @sizeOf(sg.Manifest),                    @sizeOf(sg.storage.Output), @sizeOf(sg.storage.Scratch),
        sg.storage.candidateReservation(), sg.storage.detachedReservation(), sg.storage.replacementPeakReservation(),
    });
}
