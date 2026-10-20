const std = @import("std");
const cli = @import("seeded_galaxy_cli.zig");
pub fn main(init: std.process.Init) !void {
    const args = try init.minimal.args.toSlice(init.arena.allocator());
    var buffer: [8192]u8 = undefined;
    var output = std.Io.File.stdout().writerStreaming(init.io, &buffer);
    cli.run(init.gpa, args[1..], &output.interface) catch |failure| {
        try output.interface.flush();
        return failure;
    };
    try output.interface.flush();
}
