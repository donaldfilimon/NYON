//! Native presentation of the fixed public-input acceptance stories.
const std = @import("std");
const pe = @import("planet_economy/root.zig");

pub fn run(allocator: std.mem.Allocator, args: []const []const u8, out: *std.Io.Writer) !void {
    var story: pe.stories.Story = .combined;
    var seed: u64 = 2;
    var side: u16 = 4;
    var units: u16 = 256;
    var seen: u4 = 0;
    var index: usize = 0;
    while (index < args.len) : (index += 2) {
        if (index + 1 == args.len) return invalid(out);
        const flag = args[index];
        const value = args[index + 1];
        const bit: u4 = if (std.mem.eql(u8, flag, "--seed")) 1 else if (std.mem.eql(u8, flag, "--side")) 2 else if (std.mem.eql(u8, flag, "--budget")) 4 else if (std.mem.eql(u8, flag, "--story")) 8 else return invalid(out);
        if (seen & bit != 0) return invalid(out);
        seen |= bit;
        switch (bit) {
            1 => seed = std.fmt.parseInt(u64, value, 10) catch return invalid(out),
            2 => side = std.fmt.parseInt(u16, value, 10) catch return invalid(out),
            4 => units = std.fmt.parseInt(u16, value, 10) catch return invalid(out),
            8 => story = if (std.mem.eql(u8, value, "combined")) .combined else if (std.mem.eql(u8, value, "creator")) .creator else if (std.mem.eql(u8, value, "ruler-start")) .ruler_start else if (std.mem.eql(u8, value, "policy-replay")) .policy_replay else return invalid(out),
            else => unreachable,
        }
    }
    if (seed != 2 or side != 4 or units == 0 or units > 4096) return invalid(out);

    try out.print("IDENTITY\t{s}\nCONFIG\tseed={d}\tside={d}\tbudget={d}\tstory={s}\n", .{ pe.rules_identity, seed, side, units, storyName(story) });
    switch (story) {
        .combined => try out.writeAll("START\tcombined policy\tcreator and ruler inputs\n"),
        .creator => try out.writeAll("START\tcreator policy\tcreator-authored inputs\n"),
        .ruler_start, .policy_replay => try out.writeAll("START\truler-start.v0\tcreator-authored tick 1; ruler handoff recorded\n"),
    }
    var runner = try pe.stories.Runner.init(allocator, story);
    defer runner.deinit();
    errdefer finishRunner(&runner);
    var last_case: u8 = 0;
    var cursor: pe.ReceiptCursor = 0;
    while (true) {
        var budget = pe.WorkBudget{ .remaining = units };
        switch (try runner.poll(&budget)) {
            .pending => {},
            .awaiting_observer => return error.InvariantViolation,
            .published => |tick| {
                if (tick.case_index != last_case) {
                    last_case = tick.case_index;
                    cursor = 0;
                }
                try printReceipts(&runner, tick, &cursor, out);
                const a = tick.a orelse return error.InvariantViolation;
                try out.print("TICK\tcase={d}\ttick={d}\tA={d},{d},{d}\t", .{
                    tick.case_index, @backingInt(tick.summary.tick), a.population, a.food, a.material,
                });
                if (tick.b) |b| try out.print("B={d},{d},{d}", .{ b.population, b.food, b.hunger }) else try out.writeAll("B=-");
                switch (tick.freight) {
                    .none => try out.writeAll("\tFreight=none"),
                    .transit => |cell| try out.print("\tFreight=transit@{d}", .{cell.local}),
                    .delivered => |cell| try out.print("\tFreight=delivered@{d}", .{cell.local}),
                }
                try out.print("\trevision={d}\taccepted={d}\trejected={d}\tchain=", .{
                    @backingInt(tick.summary.revision), tick.summary.accepted, tick.summary.rejected,
                });
                try printHex(out, &tick.summary.chain_head);
                try out.writeByte('\n');
                try runner.acknowledge(tick.summary.revision);
            },
            .complete => |result| {
                try out.print("PASS\tstory={s}\tticks={d}\tassertions={d}\tchain=", .{ storyName(story), result.ticks, result.assertions });
                try printHex(out, &result.chain_head);
                try out.writeByte('\n');
                return;
            },
        }
    }
}

// A failed output write can happen after publication but before acknowledgment.
// Finish the fixed acceptance script so the runner releases every live lane.
fn finishRunner(runner: *pe.stories.Runner) void {
    while (true) {
        var budget = pe.WorkBudget{ .remaining = 4096 };
        const progress = runner.poll(&budget) catch |failure| {
            // Runner reports its original failure only after draining owned work.
            std.log.err("planet economy runner failed while draining: {s}", .{@errorName(failure)});
            return;
        };
        switch (progress) {
            .pending => {},
            .published => |value| runner.acknowledge(value.summary.revision) catch unreachable,
            .awaiting_observer => runner.acknowledge(runner.pending_summary.?.revision) catch unreachable,
            .complete => return,
        }
    }
}

fn printReceipts(runner: *const pe.stories.Runner, tick: pe.stories.StoryTick, cursor: *pe.ReceiptCursor, out: *std.Io.Writer) !void {
    var rows: [64]pe.ReceiptView = undefined;
    while (true) {
        const page = try runner.receipts(tick.summary.revision, cursor.*, &rows);
        for (rows[0..page.written]) |row| {
            try out.print("RECEIPT\tcase={d}\ttick={d}\tindex={d}\tbytes=", .{ tick.case_index, @backingInt(tick.summary.tick), cursor.* });
            try printHex(out, row.bytes[0..row.len]);
            try out.writeByte('\n');
            cursor.* += 1;
        }
        if (page.next == null) return;
    }
}

fn printHex(out: *std.Io.Writer, bytes: []const u8) !void {
    for (bytes) |byte| try out.print("{x:0>2}", .{byte});
}

fn storyName(story: pe.stories.Story) []const u8 {
    return switch (story) {
        .combined => "combined",
        .creator => "creator",
        .ruler_start => "ruler-start",
        .policy_replay => "policy-replay",
    };
}

fn invalid(out: *std.Io.Writer) !void {
    try out.writeAll("ERR\tARGUMENT\tInvalidArguments\n");
    return error.InvalidArguments;
}
