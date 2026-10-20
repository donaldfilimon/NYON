//! Pinned to Zig 0.18.0-dev.146+35accc06e's Sha256 chaining state.
//! Each advance compresses exactly one complete manually padded block.
const std = @import("std");
const storage = @import("storage.zig");
const t = @import("types.zig");
pub fn advance(scratch: *storage.Scratch, wire: []const u8) t.Failure!bool {
    if (wire.len > 16384) return error.InvalidWire;
    const blocks = (wire.len + 9 + 63) / 64;
    if (scratch.block_cursor >= blocks or scratch.sha.buf_len != 0) return error.InvariantViolation;
    const base = @as(usize, scratch.block_cursor) * 64;
    const length_start = blocks * 64 - 8;
    const bits: u64 = @as(u64, @intCast(wire.len)) * 8;
    for (&scratch.block, 0..) |*byte, index| {
        const offset = base + index;
        byte.* = if (offset < wire.len) wire[offset] else if (offset == wire.len) 0x80 else if (offset >= length_start)
            @truncate(bits >> @as(u6, @intCast((blocks * 64 - 1 - offset) * 8)))
        else
            0;
    }
    scratch.sha.update(&scratch.block);
    scratch.block_cursor += 1;
    return scratch.block_cursor == blocks;
}
pub fn finish(scratch: *const storage.Scratch) [32]u8 {
    var digest: [32]u8 = undefined;
    for (scratch.sha.s, 0..) |word, index|
        std.mem.writeInt(u32, digest[index * 4 ..][0..4], word, .big);
    return digest;
}
