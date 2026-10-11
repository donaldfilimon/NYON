//! Independently constructed receipt witnesses from the frozen wire table.
pub const Vector = struct { bytes: []const u8, chain: [32]u8 };

fn digit(comptime c: u8) u8 {
    return switch (c) {
        '0'...'9' => c - '0',
        'a'...'f' => c - 'a' + 10,
        else => @compileError("invalid witness hex digit"),
    };
}

fn decode(comptime text: []const u8) [text.len / 2]u8 {
    if (text.len % 2 != 0) @compileError("odd witness hex length");
    var bytes: [text.len / 2]u8 = undefined;
    for (&bytes, 0..) |*byte, index| byte.* = digit(text[2 * index]) * 16 + digit(text[2 * index + 1]);
    return bytes;
}

pub const h0 = decode("5e9de0d8a394dfd0a508c3d2c78f8ea0e3a0ca284eda7e8535734e686c68b977");
const g0_bytes = decode("4e594f4e2d5045302d524543454950540000000000000000000000020016006e796f6e2e706c616e65742d65636f6e6f6d792e7630020000000000000004004000000140000010000100000007000000");
const i1_bytes = decode("4e594f4e2d5045302d5245434549505401010000000000000000000200010000000000000000000000000000000000000100");
const a2_bytes = decode("4e594f4e2d5045302d524543454950540202000000000000000100000102000000000000001d000200000000000000000001010000000000000001000000000000000001010000000000000000");
const r2_bytes = decode("4e594f4e2d5045302d524543454950540302000000000000000100000102000000000000001d000200000000000000000001010000000000000001000000000000000001010000");

pub const G0 = Vector{ .bytes = &g0_bytes, .chain = decode("897ea224b676ada48723c7b9c1e65d9de04f341663c910fc432c351207c05aff") };
pub const I1 = Vector{ .bytes = &i1_bytes, .chain = decode("4968d2be58816a0f24de6369e8af666a29a6d233fb20d40428ae99343013f2c1") };
pub const A2 = Vector{ .bytes = &a2_bytes, .chain = decode("7a555e0ffecee7e2d35664d0442e5d8737720192a045063cf3feb08678d42c9e") };
pub const R2 = Vector{ .bytes = &r2_bytes, .chain = decode("906ee30eab6a2c2785c98018c496b3560dac85448763ee55be5c175d19153a99") };
