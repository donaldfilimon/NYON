const std = @import("std");

pub fn build(b: *std.Build) void {
    const expected = std.mem.trim(u8, @embedFile(".zigversion"), "\r\n");
    if (!std.mem.eql(u8, @import("builtin").zig_version_string, expected)) {
        std.debug.panic("NYON requires Zig {s}; found {s}", .{ expected, @import("builtin").zig_version_string });
    }
    const target = b.standardTargetOptions(.{});
    const optimize = b.standardOptimizeOption(.{});
    const core = b.addModule("nyon", .{
        .root_source_file = b.path("src/root.zig"),
        .target = target,
        .optimize = optimize,
    });
    const exe = b.addExecutable(.{
        .name = "nyon-zig",
        .root_module = b.createModule(.{
            .root_source_file = b.path("src/main.zig"),
            .target = target,
            .optimize = optimize,
            .imports = &.{.{ .name = "nyon", .module = core }},
        }),
    });
    b.installArtifact(exe);
    const run = b.addRunArtifact(exe);
    run.addPassthruArgs();
    b.step("run", "Run the Zig authority CLI").dependOn(&run.step);
    const tests = b.addTest(.{ .root_module = core });
    b.step("test", "Run authority tests with allocation leak checks").dependOn(&b.addRunArtifact(tests).step);
    b.step("test-bin", "Install the test executable for an uncached test count").dependOn(&b.addInstallArtifact(tests, .{ .dest_sub_path = "nyon-zig-tests" }).step);
}
