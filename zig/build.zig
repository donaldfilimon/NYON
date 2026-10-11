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
    const planet_tests = b.addTest(.{ .root_module = b.createModule(.{
        .root_source_file = b.path("src/planet_economy_tests.zig"),
        .target = target,
        .optimize = optimize,
    }) });
    const planet_run = b.addRunArtifact(planet_tests);
    planet_run.stdio = .inherit;
    b.step("test-planet", "Run the isolated planet economy suite").dependOn(&planet_run.step);

    const checker = b.addExecutable(.{ .name = "check-planet-cli", .root_module = b.createModule(.{
        .root_source_file = b.path("tools/check-planet-cli.zig"),
        .target = target,
        .optimize = optimize,
    }) });
    const check_cli = b.addRunArtifact(checker);
    check_cli.addArtifactArg(exe);
    b.step("check-planet-cli", "Check native planet economy stories and errors").dependOn(&check_cli.step);

    const wasm = b.addObject(.{ .name = "planet-economy-wasm-probe", .root_module = b.createModule(.{
        .root_source_file = b.path("src/planet_economy_wasm.zig"),
        .target = b.resolveTargetQuery(.{ .cpu_arch = .wasm32, .os_tag = .freestanding }),
        .optimize = .ReleaseSafe,
    }) });
    b.step("check-planet-wasm", "Compile the freestanding planet economy API probe").dependOn(&wasm.step);
}
