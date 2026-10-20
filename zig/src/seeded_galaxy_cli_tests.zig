const std = @import("std");
const cli = @import("seeded_galaxy_cli.zig");
const h = @import("seeded_galaxy/test_support.zig");

fn contains(text: []const u8, row: []const u8) !void {
    try std.testing.expect(std.mem.indexOf(u8, text, row) != null);
}
test "standalone journey reports large bridge refusal and decode" {
    var out = std.Io.Writer.Allocating.init(std.testing.allocator);
    defer out.deinit();
    try cli.run(std.testing.allocator, &.{ "--seed", "0", "--systems", "64", "--planets", "512", "--civilizations", "16", "--budget", "7" }, &out.writer);
    try contains(out.written(), "DECODE\tverified\tbytes=11455\n");
    try contains(out.written(), "BRIDGE\tCapacityUnsupported\n");
    try contains(out.written(), "DIGEST\t9a851413f0c528643d4360e2219ed67d340af083b2f3828a368551cd3e1e5852\n");
    try std.testing.expect(std.mem.endsWith(u8, out.written(), "SCOPE\tstrategic-metadata-only\n"));
}
test "standalone small ruler literal witness creator combined and max seed" {
    const roles = [_][]const u8{ "creator", "ruler", "combined" };
    for (roles, 0..) |role, index| {
        var out = std.Io.Writer.Allocating.init(std.testing.allocator);
        defer out.deinit();
        const args = [_][]const u8{ "--seed", "0", "--systems", "2", "--planets", "4", "--civilizations", "2", "--budget", "1", "--role", role, "--civilization", if (index == 1) "2" else "1" };
        try cli.run(std.testing.allocator, args[0..if (index == 0) 12 else 14], &out.writer);
        try contains(out.written(), "DECODE\tverified\tbytes=177\n");
        try contains(out.written(), "DIGEST\td2453927e7ba740f64ade02f4a63175fcf45d343b32817df54bbba86e9ec72b1\n");
        try contains(out.written(), "SYSTEM\t1\tSystem 1\tx=889\ty=369\tz=0\n");
        try contains(out.written(), "CIVILIZATION\t2\tCivilization 2\tfounding_planet=2\tkind=uninitialized\n");
        try contains(out.written(), "BRIDGE\tconfig-validated\tlocal=64,256,64,4096\tglobal=4096\n");
        try contains(out.written(), switch (index) {
            0 => "POLICY\tcreator\tcivilization=none\n",
            1 => "POLICY\truler\tcivilization=2\n",
            else => "POLICY\tcombined\tcivilization=1\n",
        });
    }
    var out = std.Io.Writer.Allocating.init(std.testing.allocator);
    defer out.deinit();
    try cli.run(std.testing.allocator, &.{ "--seed", "18446744073709551615", "--systems", "64", "--planets", "512", "--civilizations", "16" }, &out.writer);
    try contains(out.written(), "DIGEST\t8064e196c4258200079e25ff644ada6d241401f0ac5b79d093247bed89f4ef30\n");
}
test "standalone rejects malformed arguments and policies with one ERR row" {
    const bad = [_][]const []const u8{
        &.{"--seed"},                             &.{ "--unknown", "1" },                                           &.{ "--seed", "0", "--seed", "0" },
        &.{ "--seed", "-1" },                     &.{ "--seed", "+1" },                                             &.{ "--seed", "18446744073709551616" },
        &.{ "--seed", "" },                       &.{ "--seed", "1_0" },                                            &.{ "--seed", " 1" },
        &.{ "--seed", "0x10" },                   &.{ "--budget", "0" },                                            &.{ "--budget", "4097" },
        &.{ "--side", "3" },                      &.{ "--side", "33" },                                             &.{ "--systems", "0" },
        &.{ "--systems", "65" },                  &.{ "--planets", "513" },                                         &.{ "--civilizations", "17" },
        &.{ "--civilization", "0" },              &.{ "--civilization", "17" },                                     &.{ "--role", "bad" },
        &.{ "--systems", "3", "--planets", "2" }, &.{ "--planets", "1", "--systems", "1", "--civilizations", "2" },
    };
    for (bad) |args| {
        var out = std.Io.Writer.Allocating.init(std.testing.allocator);
        defer out.deinit();
        try std.testing.expectError(error.InvalidArguments, cli.run(std.testing.allocator, args, &out.writer));
        try std.testing.expectEqualStrings("ERR\tInvalidArguments\n", out.written());
    }
    for ([_][]const []const u8{ &.{ "--role", "ruler" }, &.{ "--role", "combined" }, &.{ "--civilization", "5" } }) |args| {
        var out = std.Io.Writer.Allocating.init(std.testing.allocator);
        defer out.deinit();
        try std.testing.expectError(error.InvalidPolicy, cli.run(std.testing.allocator, args, &out.writer));
        try std.testing.expectEqualStrings("ERR\tInvalidPolicy\n", out.written());
    }
}
test "standalone every byte boundary writer failure releases all ownership" {
    const args = [_][]const u8{ "--systems", "2", "--planets", "4", "--civilizations", "2" };
    var complete = std.Io.Writer.Allocating.init(std.testing.allocator);
    defer complete.deinit();
    try cli.run(std.testing.allocator, &args, &complete.writer);
    var bytes: [4096]u8 = undefined;
    try std.testing.expect(complete.written().len < bytes.len);
    for (0..complete.written().len) |cut| {
        var counter = h.Counter.init(std.testing.allocator);
        var out = std.Io.Writer.fixed(bytes[0..cut]);
        try std.testing.expectError(error.WriteFailed, cli.run(counter.allocator(), &args, &out));
        try std.testing.expectEqual(@as(usize, 0), counter.live);
    }
}

test "standalone defaults duplicate flags all admissions and aggregate reservation" {
    var counter = h.Counter.init(std.testing.allocator);
    var out = std.Io.Writer.Allocating.init(std.testing.allocator);
    defer out.deinit();
    try cli.run(counter.allocator(), &.{}, &out.writer);
    try contains(out.written(), "CONFIG\tseed=0\tsystems=12\tplanets=24\tcivilizations=4\tside=8\tbudget=256\n");
    try contains(out.written(), "POLICY\tcreator\tcivilization=none\n");
    try std.testing.expectEqual(@as(usize, 0), counter.live);
    try std.testing.expectEqual(@as(usize, 4), counter.attempts);
    // One retained output plus one replacement candidate, excluding caller buffers.
    try std.testing.expectEqual(@as(usize, 76496), counter.peak);
    const flags = [_][]const u8{ "--seed", "--systems", "--planets", "--civilizations", "--side", "--budget", "--role", "--civilization" };
    for (flags) |flag| {
        var sink = std.Io.Writer.Allocating.init(std.testing.allocator);
        defer sink.deinit();
        const value = if (std.mem.eql(u8, flag, "--role")) "creator" else "4";
        try std.testing.expectError(error.InvalidArguments, cli.run(std.testing.allocator, &.{ flag, value, flag, value }, &sink.writer));
        try std.testing.expectEqualStrings("ERR\tInvalidArguments\n", sink.written());
    }
    for (0..5) |cut| {
        var failing = std.testing.FailingAllocator.init(std.testing.allocator, .{ .fail_index = cut });
        var tracking = h.Counter.init(failing.allocator());
        var bytes: [8192]u8 = undefined;
        var sink = std.Io.Writer.fixed(&bytes);
        if (cut < 4) {
            try std.testing.expectError(error.OutOfMemory, cli.run(tracking.allocator(), &.{}, &sink));
            try std.testing.expectEqualStrings("ERR\tOutOfMemory\n", sink.buffered());
        } else try cli.run(tracking.allocator(), &.{}, &sink);
        try std.testing.expectEqual(@as(usize, 0), tracking.live);
        try std.testing.expectEqual(failing.allocations, failing.deallocations);
    }
}

test "standalone maximum rows and each row writer failure preserve allocation cleanup" {
    const args = [_][]const u8{ "--systems", "64", "--planets", "512", "--civilizations", "16" };
    var complete = std.Io.Writer.Allocating.init(std.testing.allocator);
    defer complete.deinit();
    try cli.run(std.testing.allocator, &args, &complete.writer);
    const text = complete.written();
    var counts: [10]usize = @splat(0);
    const headings = [_][]const u8{ "IDENTITY", "CONFIG", "SYSTEM", "PLANET", "CIVILIZATION", "DIGEST", "DECODE", "POLICY", "BRIDGE", "SCOPE" };
    var rows = std.mem.tokenizeScalar(u8, text, '\n');
    while (rows.next()) |row| {
        const tab = std.mem.indexOfScalar(u8, row, '\t') orelse return error.MissingHeading;
        var found = false;
        for (headings, 0..) |heading, index| if (std.mem.eql(u8, row[0..tab], heading)) {
            counts[index] += 1;
            found = true;
            break;
        };
        try std.testing.expect(found);
    }
    try std.testing.expectEqualSlices(usize, &.{ 1, 1, 64, 512, 16, 1, 1, 1, 1, 1 }, &counts);
    var storage: [65536]u8 = undefined;
    try std.testing.expect(text.len < storage.len);
    for (text, 0..) |byte, cut| {
        if (byte != '\n') continue;
        var counter = h.Counter.init(std.testing.allocator);
        var writer = std.Io.Writer.fixed(storage[0..cut]);
        try std.testing.expectError(error.WriteFailed, cli.run(counter.allocator(), &args, &writer));
        try std.testing.expectEqual(@as(usize, 0), counter.live);
    }
}
