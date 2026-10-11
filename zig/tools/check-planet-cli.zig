const std = @import("std");

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

fn checkRows(output: []const u8, expected: []const []const u8) !void {
    var lines = std.mem.splitScalar(u8, output, '\n');
    var index: usize = 0;
    while (lines.next()) |line| {
        if (!std.mem.startsWith(u8, line, "TICK\t")) continue;
        try require(index < expected.len, "unexpected tick row");
        const marker = std.mem.indexOf(u8, line, "\tchain=") orelse return error.MissingChain;
        try require(std.mem.eql(u8, expected[index], line[0..marker]), "tick row order or value");
        const chain = line[marker + "\tchain=".len ..];
        try require(chain.len == 64, "chain length");
        for (chain) |digit| try require(std.ascii.isHex(digit), "chain hex");
        index += 1;
    }
    try require(index == expected.len, "missing tick row");
}

pub fn main(init: std.process.Init) !void {
    const args = try init.minimal.args.toSlice(init.arena.allocator());
    if (args.len != 2) return error.InvalidArguments;
    const exe = args[1];
    const cases = [_]struct { name: []const u8, rows: []const []const u8 }{
        .{ .name = "combined", .rows = &combined_rows },
        .{ .name = "creator", .rows = &creator_rows },
        .{ .name = "ruler-start", .rows = &ruler_rows },
        .{ .name = "policy-replay", .rows = &replay_rows },
    };
    for (cases) |case| {
        const argv = [_][]const u8{ exe, "planet-economy-demo", "--seed", "2", "--side", "4", "--budget", "256", "--story", case.name };
        const result = try std.process.run(init.gpa, init.io, .{ .argv = &argv });
        defer init.gpa.free(result.stdout);
        defer init.gpa.free(result.stderr);
        try require(result.term.success(), case.name);
        try require(std.mem.indexOf(u8, result.stdout, "IDENTITY\tnyon.planet-economy.v0\n") != null, case.name);
        try require(std.mem.indexOf(u8, result.stdout, "CONFIG\tseed=2\tside=4\tbudget=256") != null, case.name);
        try require(std.mem.indexOf(u8, result.stdout, "RECEIPT\tcase=0\ttick=1\tindex=") != null, case.name);
        try require(std.mem.indexOf(u8, result.stdout, "START\t") != null, case.name);
        try checkRows(result.stdout, case.rows);
        try require(std.mem.indexOf(u8, result.stdout, "PASS\tstory=") != null, case.name);
        if (std.mem.eql(u8, case.name, "ruler-start") or std.mem.eql(u8, case.name, "policy-replay"))
            try require(std.mem.indexOf(u8, result.stdout, "START\truler-start.v0\tcreator-authored tick 1") != null, case.name);
    }
    const defaults = [_][]const u8{ exe, "planet-economy-demo" };
    const default_result = try std.process.run(init.gpa, init.io, .{ .argv = &defaults });
    defer init.gpa.free(default_result.stdout);
    defer init.gpa.free(default_result.stderr);
    try require(default_result.term.success() and std.mem.indexOf(u8, default_result.stdout, "story=combined") != null, "defaults");
    try checkRows(default_result.stdout, &combined_rows);

    const rejected = [_][]const []const u8{
        &.{ "--seed", "3" },                &.{ "--side", "5" },        &.{ "--budget", "0" },
        &.{ "--budget", "4097" },           &.{"--seed"},               &.{ "--strange", "1" },
        &.{ "--seed", "2", "--seed", "2" }, &.{ "--story", "missing" },
    };
    for (rejected) |suffix| {
        var argv: [10][]const u8 = undefined;
        argv[0] = exe;
        argv[1] = "planet-economy-demo";
        @memcpy(argv[2 .. 2 + suffix.len], suffix);
        const result = try std.process.run(init.gpa, init.io, .{ .argv = argv[0 .. 2 + suffix.len] });
        defer init.gpa.free(result.stdout);
        defer init.gpa.free(result.stderr);
        try require(!result.term.success(), "rejected status");
        try require(std.mem.indexOf(u8, result.stdout, "ERR\tARGUMENT\tInvalidArguments\n") != null, "rejected diagnostic");
        try require(std.mem.indexOf(u8, result.stdout, "PASS\t") == null, "rejected pass");
    }
    std.debug.print("planet CLI: four stories, default, eight invalid inputs passed\n", .{});
}

fn require(ok: bool, context: []const u8) !void {
    if (ok) return;
    std.log.err("planet CLI check failed: {s}", .{context});
    return error.CheckFailed;
}
