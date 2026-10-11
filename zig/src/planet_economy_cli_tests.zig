const std = @import("std");
const cli = @import("planet_economy_cli.zig");

fn invoke(args: []const []const u8) ![]u8 {
    var output = std.Io.Writer.Allocating.init(std.testing.allocator);
    defer output.deinit();
    try cli.run(std.testing.allocator, args, &output.writer);
    return try std.testing.allocator.dupe(u8, output.written());
}

const combined_rows = [_][]const u8{
    "TICK\tcase=0\ttick=1\tA=4,4,10\tB=2,4,0\tFreight=transit@70\trevision=1\taccepted=5\trejected=2",
    "TICK\tcase=0\ttick=2\tA=4,6,12\tB=2,2,0\tFreight=transit@71\trevision=2\taccepted=0\trejected=0",
    "TICK\tcase=0\ttick=3\tA=4,8,14\tB=2,0,0\tFreight=transit@4\trevision=3\taccepted=0\trejected=0",
    "TICK\tcase=0\ttick=4\tA=4,10,16\tB=2,2,0\tFreight=delivered@5\trevision=4\taccepted=0\trejected=0",
    "TICK\tcase=0\ttick=5\tA=4,12,18\tB=2,0,0\tFreight=none\trevision=5\taccepted=0\trejected=0",
    "TICK\tcase=0\ttick=6\tA=4,14,20\tB=2,0,1\tFreight=none\trevision=6\taccepted=0\trejected=0",
    "TICK\tcase=0\ttick=7\tA=4,16,22\tB=2,0,2\tFreight=none\trevision=7\taccepted=0\trejected=0",
    "TICK\tcase=0\ttick=8\tA=4,18,24\tB=1,0,0\tFreight=none\trevision=8\taccepted=0\trejected=0",
};
const creator_rows = [_][]const u8{
    "TICK\tcase=0\ttick=1\tA=4,2,2\tB=2,4,0\tFreight=transit@70\trevision=1\taccepted=6\trejected=0",
    "TICK\tcase=0\ttick=2\tA=4,4,4\tB=2,2,0\tFreight=transit@71\trevision=2\taccepted=0\trejected=0",
    "TICK\tcase=0\ttick=3\tA=4,6,6\tB=2,0,0\tFreight=transit@4\trevision=3\taccepted=0\trejected=0",
    "TICK\tcase=0\ttick=4\tA=4,8,8\tB=2,2,0\tFreight=delivered@5\trevision=4\taccepted=0\trejected=0",
};
const ruler_rows = [_][]const u8{
    "TICK\tcase=0\ttick=1\tA=4,8,20\tB=-\tFreight=none\trevision=1\taccepted=1\trejected=0",
    "TICK\tcase=0\ttick=2\tA=4,4,14\tB=-\tFreight=none\trevision=2\taccepted=1\trejected=0",
    "TICK\tcase=0\ttick=3\tA=4,6,14\tB=-\tFreight=none\trevision=3\taccepted=0\trejected=0",
};
const replay_rows = [_][]const u8{
    "TICK\tcase=0\ttick=1\tA=4,8,20\tB=-\tFreight=none\trevision=1\taccepted=1\trejected=0",
    "TICK\tcase=0\ttick=2\tA=4,5,20\tB=-\tFreight=none\trevision=2\taccepted=1\trejected=0",
    "TICK\tcase=0\ttick=3\tA=4,1,20\tB=-\tFreight=none\trevision=3\taccepted=0\trejected=0",
    "TICK\tcase=1\ttick=1\tA=4,8,20\tB=-\tFreight=none\trevision=1\taccepted=1\trejected=0",
    "TICK\tcase=1\ttick=2\tA=4,4,20\tB=-\tFreight=none\trevision=2\taccepted=0\trejected=1",
    "TICK\tcase=1\ttick=3\tA=4,0,20\tB=-\tFreight=none\trevision=3\taccepted=0\trejected=0",
    "TICK\tcase=2\ttick=1\tA=4,8,20\tB=-\tFreight=none\trevision=1\taccepted=1\trejected=0",
    "TICK\tcase=2\ttick=2\tA=4,5,20\tB=-\tFreight=none\trevision=2\taccepted=1\trejected=0",
    "TICK\tcase=2\ttick=3\tA=4,1,20\tB=-\tFreight=none\trevision=3\taccepted=0\trejected=0",
    "TICK\tcase=3\ttick=1\tA=4,8,20\tB=-\tFreight=none\trevision=1\taccepted=1\trejected=0",
    "TICK\tcase=3\ttick=2\tA=4,4,20\tB=-\tFreight=none\trevision=2\taccepted=0\trejected=1",
    "TICK\tcase=3\ttick=3\tA=4,0,20\tB=-\tFreight=none\trevision=3\taccepted=0\trejected=0",
};

fn expectRows(output: []const u8, expected: []const []const u8) !void {
    var lines = std.mem.splitScalar(u8, output, '\n');
    var index: usize = 0;
    while (lines.next()) |line| {
        if (!std.mem.startsWith(u8, line, "TICK\t")) continue;
        try std.testing.expect(index < expected.len);
        const marker = std.mem.indexOf(u8, line, "\tchain=") orelse return error.MissingChain;
        try std.testing.expectEqualStrings(expected[index], line[0..marker]);
        const chain = line[marker + "\tchain=".len ..];
        try std.testing.expectEqual(@as(usize, 64), chain.len);
        for (chain) |digit| try std.testing.expect(std.ascii.isHex(digit));
        index += 1;
    }
    try std.testing.expectEqual(expected.len, index);
}

test "pe/cli/stories-and-errors" {
    const stories = [_][]const u8{ "combined", "creator", "ruler-start", "policy-replay" };
    for (stories, 0..) |story, story_index| {
        const args = [_][]const u8{ "--seed", "2", "--side", "4", "--budget", "256", "--story", story };
        const actual = try invoke(&args);
        defer std.testing.allocator.free(actual);
        try std.testing.expect(std.mem.indexOf(u8, actual, "IDENTITY\tnyon.planet-economy.v0\n") != null);
        try std.testing.expect(std.mem.indexOf(u8, actual, "CONFIG\tseed=2\tside=4\tbudget=256") != null);
        try std.testing.expect(std.mem.indexOf(u8, actual, "RECEIPT\tcase=0\ttick=1\t") != null);
        try std.testing.expect(std.mem.indexOf(u8, actual, "TICK\tcase=0\ttick=1\t") != null);
        try std.testing.expect(std.mem.indexOf(u8, actual, "PASS\t") != null);
        try std.testing.expect(std.mem.indexOf(u8, actual, "START\t") != null);
        const expected: []const []const u8 = switch (story_index) {
            0 => &combined_rows,
            1 => &creator_rows,
            2 => &ruler_rows,
            3 => &replay_rows,
            else => unreachable,
        };
        try expectRows(actual, expected);
        if (std.mem.eql(u8, story, "ruler-start") or std.mem.eql(u8, story, "policy-replay"))
            try std.testing.expect(std.mem.indexOf(u8, actual, "START\truler-start.v0\tcreator-authored tick 1") != null);
    }
    const defaults = try invoke(&.{});
    defer std.testing.allocator.free(defaults);
    try std.testing.expect(std.mem.indexOf(u8, defaults, "story=combined") != null);
    try expectRows(defaults, &combined_rows);

    const bad = [_][]const []const u8{
        &.{ "--seed", "3" },                &.{ "--side", "5" },        &.{ "--budget", "0" },
        &.{ "--budget", "4097" },           &.{"--seed"},               &.{ "--strange", "1" },
        &.{ "--seed", "2", "--seed", "2" }, &.{ "--story", "missing" },
    };
    for (bad) |args| {
        var output = std.Io.Writer.Allocating.init(std.testing.allocator);
        defer output.deinit();
        try std.testing.expectError(error.InvalidArguments, cli.run(std.testing.allocator, args, &output.writer));
    }
}

test "pe/cli/writer-failure drains live runner" {
    var bytes: [160]u8 = undefined;
    var writer = std.Io.Writer.fixed(&bytes);
    try std.testing.expectError(error.WriteFailed, cli.run(std.testing.allocator, &.{}, &writer));
}
