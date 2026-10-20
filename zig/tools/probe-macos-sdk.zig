//! Read-only host prerequisite probe; never creates a window or opens profiles.
//! Build on macOS with -lc -lobjc -framework AppKit -framework Metal.
const std = @import("std");
const builtin = @import("builtin");

extern "objc" fn objc_getClass(name: [*:0]const u8) ?*anyopaque;
extern "objc" fn sel_registerName(name: [*:0]const u8) *anyopaque;
extern "objc" fn objc_msgSend() callconv(.c) void;
extern "Metal" fn MTLCreateSystemDefaultDevice() ?*anyopaque;

comptime {
    if (builtin.os.tag != .macos) @compileError("macOS SDK probe requires macOS");
    if (!std.mem.eql(u8, builtin.zig_version_string, "0.18.0-dev.146+35accc06e"))
        @compileError("NYON requires its exact pinned Zig compiler");
}

pub fn main() !void {
    if (objc_getClass("NSApplication") == null) return error.AppKitApplicationUnavailable;
    if (objc_getClass("NSWindow") == null) return error.AppKitWindowUnavailable;
    const device = MTLCreateSystemDefaultDevice() orelse return error.MetalDeviceUnavailable;
    const send_release: *const fn (*anyopaque, *anyopaque) callconv(.c) void = @ptrCast(&objc_msgSend);
    defer send_release(device, sel_registerName("release"));
    std.debug.print("macos-sdk-probe: AppKit classes and retained Metal device available\n", .{});
}
