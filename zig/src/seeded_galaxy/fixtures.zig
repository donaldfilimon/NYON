//! Literal independent witnesses from the approved strategic specification.
pub const small_config = @import("types.zig").Config{
    .seed = 0,
    .systems = 2,
    .planets = 4,
    .civilizations = 2,
    .side = 8,
};

fn nibble(comptime digit: u8) u8 {
    return switch (digit) {
        '0'...'9' => digit - '0',
        'a'...'f' => digit - 'a' + 10,
        'A'...'F' => digit - 'A' + 10,
        else => @compileError("nonhex fixture digit"),
    };
}

pub fn decodeHex(comptime hex: []const u8) [hex.len / 2]u8 {
    if (hex.len % 2 != 0) @compileError("odd fixture hex length");
    var bytes: [hex.len / 2]u8 = undefined;
    for (0..bytes.len) |index| {
        bytes[index] = (nibble(hex[index * 2]) << 4) | nibble(hex[index * 2 + 1]);
    }
    return bytes;
}

pub const small_wire = decodeHex("4e595347010015006e796f6e2e7365656465642d67616c6178792e7630010000000000000000000200040002000800" ++
    "01000000790300007101000000000000020000002f110000fc02000000000000" ++
    "010000000100000000000000000000000000080002000000020000000000157c4a7fb979379e0800" ++
    "030000000100000001002af894fe72f36e3c0800040000000200000001003f74df7d2c6da6da0800" ++
    "010000000100000000020000000200000000");
pub const small_digest = decodeHex("d2453927e7ba740f64ade02f4a63175fcf45d343b32817df54bbba86e9ec72b1");
pub const maximum_zero_digest = decodeHex("9a851413f0c528643d4360e2219ed67d340af083b2f3828a368551cd3e1e5852");
pub const maximum_max_digest = decodeHex("8064e196c4258200079e25ff644ada6d241401f0ac5b79d093247bed89f4ef30");
