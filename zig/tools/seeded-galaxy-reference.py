"""Independent, read-only strategic wire fixture preparation.

Optional developer verification only; no final runtime or tool pipeline dependency.
Imports no NYON product code and writes no replacement fixture files.
"""

import hashlib
import struct

MASK = (1 << 64) - 1


def mix(value):
    value = (value + 0x9E3779B97F4A7C15) & MASK
    value = ((value ^ (value >> 30)) * 0xBF58476D1CE4E5B9) & MASK
    value = ((value ^ (value >> 27)) * 0x94D049BB133111EB) & MASK
    return value ^ (value >> 31)


def wire(seed, systems, planets, civilizations):
    result = b"NYSG" + struct.pack("<HH", 1, 21) + b"nyon.seeded-galaxy.v0"
    result += struct.pack("<HQHHHH", 1, seed, systems, planets, civilizations, 8)
    for i in range(1, systems + 1):
        x = ((i - 1) % 8) * 4096 + mix(seed ^ (1 << 56) ^ (i << 16)) % 1024
        y = ((i - 1) // 8) * 4096 + mix(seed ^ (1 << 56) ^ (i << 16) ^ 1) % 1024
        result += struct.pack("<Iiii", i, x, y, 0)
    for p in range(1, planets + 1):
        result += struct.pack("<IIHQH", p, 1 + (p - 1) % systems,
                              (p - 1) // systems,
                              seed ^ (((p - 1) * 0x9E3779B97F4A7C15) & MASK), 8)
    for c in range(1, civilizations + 1):
        result += struct.pack("<IIB", c, c, 0)
    return result


SMALL_LITERAL = bytes.fromhex(
    "4e595347010015006e796f6e2e7365656465642d67616c6178792e7630010000000000000000000200040002000800"
    "01000000790300007101000000000000020000002f110000fc02000000000000"
    "010000000100000000000000000000000000080002000000020000000000157c4a7fb979379e0800"
    "030000000100000001002af894fe72f36e3c0800040000000200000001003f74df7d2c6da6da0800"
    "010000000100000000020000000200000000"
)

assert mix(0) == 0xE220A8397B1DCDAF
assert wire(0, 2, 4, 2) == SMALL_LITERAL
assert hashlib.sha256(SMALL_LITERAL).hexdigest() == "d2453927e7ba740f64ade02f4a63175fcf45d343b32817df54bbba86e9ec72b1"
assert hashlib.sha256(wire(0, 64, 512, 16)).hexdigest() == "9a851413f0c528643d4360e2219ed67d340af083b2f3828a368551cd3e1e5852"
assert hashlib.sha256(wire(MASK, 64, 512, 16)).hexdigest() == "8064e196c4258200079e25ff644ada6d241401f0ac5b79d093247bed89f4ef30"
assert len(SMALL_LITERAL) == 177
assert len(wire(0, 64, 512, 16)) == 11455
