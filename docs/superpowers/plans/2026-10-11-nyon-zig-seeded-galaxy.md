# NYON Zig Seeded Strategic Galaxy Implementation Plan

<!-- BEGIN 2026-10-19 rewrite decision -->
> **Canceled on 2026-10-19:** Donald canceled further Zig rewrite work. Unfinished Zig tasks and continuation instructions in this document are superseded; retain its historical evidence and existing source. See [the current implementation decision](../../../REWRITE-DECISION-2026-10-19.md).
<!-- END 2026-10-19 rewrite decision -->

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Deliver a deterministic, bounded strategic genesis manifest with owned decoding, copied inspection, policy admission, an honest coordinated-v0 config bridge, and a standalone CLI acceptance journey.

**Architecture:** A new `zig/src/seeded_galaxy/` subsystem owns immutable strategic metadata and a single incremental setup lane. The lane generates or parses records, recomputes their invariants, encodes canonical bytes and charges SHA256 compression individually. It imports existing role/config types without importing the unfinished coordinated runtime; final shared-build integration is a separately owned handoff.

**Tech Stack:** Zig `0.18.0-dev.146+35accc06e`, standard library, optional Python standard-library fixture preparation; no Rust and no new production dependency.

**Spec:** `docs/superpowers/specs/2026-10-11-nyon-zig-seeded-galaxy-design.md`. Written specification is controller-approved under delegated authority after the independent F1/F2 revision review; read the current approved revision before execution. Approved spec SHA256: `5577e8886f80a48df656fa1eb53ff3421a74c9deaecb5d2e7307001f43f4320d`. This plan does not supersede the specification.

**Status:** Proposed implementation plan, 2026-10-11. Donald delegated game design and approval decisions at 08:19–08:20 EDT; see `docs/qualification/2026-10-11-delegated-game-design-authority.md`. The controller may approve this exact artifact after independent review without another human approval loop. Planning, approval, source implementation, focused qualification and integrated delivery are separate statuses.

## Global Constraints

- Compiler is exactly `0.18.0-dev.146+35accc06e`; use `zig/.zigversion` and verify `zig version`.
- Identity `nyon.seeded-galaxy.v0`; generator version:u16 exactly1; wire version:u16 exactly1.
- Systems1..64, planets1..512, civilizations1..16, side4..32; systems and civilizations each <=planets; default12/24/4/8.
- Seeds:u64; all entity IDs one-based contiguous enum(u32); system coordinates:i32 integer strategic units; distance:u64, never ticks or duration.
- Frozen Classic `zig/src/generator.zig` remains untouched. Terrain derivation remains `seed XOR ((p-1)*%0x9E3779B97F4A7C15)`.
- No settlements, resources, assets, automatic founding grants, fleets, travel orders, AI, diplomacy, research or persistence in this subsystem.
- One setup lane, no queue; poll budget:u16 in0..4096, default256; budget0 observes without mutation.
- One output and one scratch allocation per candidate, each <=65,536 bytes; candidate reservation including controls/alignment <=131,072 bytes. Retained current output is accounted separately; replacement peak <=196,608 bytes.
- No poll allocation. A generated/validated/encoded record costs one unit; maximum record256 bytes. SHA compression costs one unit per64-byte block including padding; final digest extraction costs one unit.
- Queries copy at most64 records per call with no allocation, hashing or escaped interior pointer. Decoder input ceiling16,384 bytes; owned snapshot established before begin returns.
- No commits, pushes, new worktrees, instruction-file edits, new production dependencies or old-profile access. Preserve all existing dirty files.
- Sequential fresh implementer and independent reviewer per task; review the exact changed source bytes, tests and spec revision, not `HEAD` alone. Stop dependent dispatch on unresolved review findings.
- Active coordinator owner retains all `zig/src/coordinated_galaxy/*`, existing aggregate/root/main/build/gate files and its ledger. New strategic files only are writable under this plan.

## Review Focus

1. Token replay across jobs/controllers and shutdown: reject stale tokens before touching a live candidate; test in Task2.
2. Taking a result while retaining the previous manifest: account both owners, reopen the lane only at the specified terminal boundary, and survive either destruction order; test in Task2.
3. Caller mutation of decode bytes and simultaneous malformed header/count/length conditions: preserve snapshot and deterministic error precedence; test in Task3.
4. A failed policy/bridge/export operation: preserve current manifest bytes and every supplied v0 capacity; test in Task4.
5. Tiny/zero/random budgets near two-block SHA padding, cancellation at every charged unit, and broken CLI output: retain bounded work and release exactly owned memory; test in Tasks2,3,5.

---

## Ownership and file map

All paths below are new unless explicitly described as read-only. Do not broaden ownership to repair unrelated coordinator failures.

| Path | Responsibility |
| --- | --- |
| `zig/src/seeded_galaxy/types.zig` | IDs, records, bounded config, errors, progress and accounting values |
| `zig/src/seeded_galaxy/generate.zig` | Pure keyed mixing, individual record formulas, checked distance |
| `zig/src/seeded_galaxy/storage.zig` | Exact output/scratch/control layouts and reservation proofs |
| `zig/src/seeded_galaxy/setup.zig` | Persistent setup lane, tokens, ownership and charged phase cursor |
| `zig/src/seeded_galaxy/wire.zig` | Incremental record encoder/parser, exact schema |
| `zig/src/seeded_galaxy/hash.zig` | One-compression-per-step SHA256 adapter |
| `zig/src/seeded_galaxy/manifest.zig` | Owning immutable result and copied query pages |
| `zig/src/seeded_galaxy/policy.zig` | Existing role admission plus selected civilization presence |
| `zig/src/seeded_galaxy/bridge.zig` | Exact supplied-capacity conversion to v0 Config |
| `zig/src/seeded_galaxy/root.zig` | Public strategic facade |
| `zig/src/seeded_galaxy/{contracts,generation,lifecycle,wire,policy_bridge,qualification}_tests.zig` | Corresponding regression boundaries |
| `zig/src/seeded_galaxy/fixtures.zig` | Hand-reviewed independent literal bytes/digests |
| `zig/src/seeded_galaxy_tests.zig` | Standalone aggregate, importing only strategic test modules |
| `zig/src/seeded_galaxy_cli.zig` | Writer-based presentation and argument validation |
| `zig/src/seeded_galaxy_cli_main.zig` | Standalone executable entry point |
| `zig/src/seeded_galaxy_cli_tests.zig` | Public CLI journey and failing-output cleanup |
| `zig/tools/seeded-galaxy-reference.py` | Optional independent fixture verification, never product dependency |
| `docs/qualification/2026-10-11-seeded-galaxy-implementation.md` | Source hashes, commands, counters, timing and proof gaps |

Read-only dependencies: `zig/src/play_role.zig`, `zig/src/coordinated_galaxy/types.zig`, `zig/src/coordinated_galaxy/wire_vectors.zig`, and pinned standard-library `crypto/sha2.zig`. The bridge does not import `coordinated_galaxy/root.zig`, `state.zig` or `planet.zig`.

## Shared value interfaces

Task1 defines these names; subsequent tasks consume them unchanged. Do not expose ABI layout as canonical bytes.

```zig
pub const SystemId = enum(u32) { _ };
pub const PlanetId = enum(u32) { _ };
pub const CivilizationId = @import("../play_role.zig").CivilizationId;
pub const Config = struct {
    seed: u64 = 0,
    systems: u16 = 12,
    planets: u16 = 24,
    civilizations: u16 = 4,
    side: u16 = 8,
    version: u16 = 1,
};
pub const System = struct { id: SystemId, x: i32, y: i32, z: i32 };
pub const Planet = struct {
    id: PlanetId, system: SystemId, orbit_index: u16,
    terrain_seed: u64, side: u16,
};
pub const Civilization = struct {
    id: CivilizationId, founding_planet: PlanetId,
    kind: enum(u8) { uninitialized = 0 },
};
pub const Failure = error{
    InvalidVersion, CapacityUnsupported, InvalidConfig, InvalidPolicy,
    OutOfMemory, InvalidBudget, Busy, StaleJob, NotReady, Shutdown,
    ArithmeticOverflow, InvariantViolation, IdExhausted,
    InvalidWire, UnsupportedWireVersion,
};
pub const Phase = enum(u8) {
    validate_config, parse_header, systems, planets, civilizations,
    validate_records, encode_header, encode_records, hash_blocks,
    finalize_digest, ready, canceled, failed, taken,
};
pub const Stats = struct {
    phase: Phase, cursor: u16, units_used: u16, total_units: u64,
    requested_bytes: usize, admission_copy_bytes: u16,
};
pub const Progress = union(enum) {
    pending: Stats, ready: Stats, canceled, failed: Failure,
};
pub const Page = struct { written: u8, next: ?u16 };
pub const WireRecord = struct { len: u16, bytes: [254]u8 };
pub fn validateConfig(config: Config) Failure!void;
```

Page cursors are zero-based record offsets. `start == count` returns an empty terminal page; `start > count`, zero-capacity output, or output capacity>64 returns `InvalidConfig` without writes. Partial final pages set `next=null`. Wire record0 is the47-byte header, followed by system, planet and civilization records in that order. Header/record lengths are47/16/20/9; maximum wire is11,455 bytes, not the16,384-byte admitted-input ceiling.

Pure generation interfaces in `generate.zig`:

```zig
pub fn mix(value: u64) u64;
pub fn draw(seed: u64, domain: u8, id: u32, field: u16) u64;
pub fn system(config: Config, id: SystemId) Failure!System;
pub fn planet(config: Config, id: PlanetId) Failure!Planet;
pub fn civilization(config: Config, id: CivilizationId) Failure!Civilization;
pub fn distance(a: System, b: System) Failure!u64;
```

Only SplitMix additions/multiplications and approved terrain multiplication wrap. Public helpers check IDs before subtracting1. Distance uses signed i128 differences, checked u128 squared sum, integer floor square root and checked u64 conversion; no float, host entropy or mutable PRNG.

### Task 1: Freeze independent witnesses, typed records and resource layout

**Files:** Create `types.zig`, `generate.zig`, `storage.zig`, `fixtures.zig`, `contracts_tests.zig`, `generation_tests.zig`, `root.zig`, standalone aggregate, optional reference script. No setup or coordinator implementation changes.

**Interfaces:** Produce the shared value and generation signatures above, plus `storage.Output`, `storage.Scratch`, `storage.candidateReservation() usize`; facade exports types, `generate` and `storage`.

- [ ] **Step1: Record baseline and exact owner scope.** Read this plan/spec, home/repo guides, `zig/.zigversion`; run `git --no-optional-locks status --short --branch` and `zig version`. Record existing dirty paths without staging. Require the exact compiler before interpreting compile errors.
- [ ] **Step2: Add independent literal witnesses and failing tests.** Fixture preparation must import no NYON encoder/generator. Copy the spec's177-byte full wire and digest verbatim into `fixtures.zig` using a compile-time hex decoder that rejects odd length/nonhex. Define `small_config`, `small_wire`, `small_digest`, `maximum_zero_digest`, `maximum_max_digest` and `decodeHex(comptime hex: []const u8) [hex.len/2]u8`.

```zig
const std = @import("std");
const sg = @import("root.zig");
const f = @import("fixtures.zig");
test "independent seed zero records and mixing" {
    try std.testing.expectEqual(@as(u64, 0xE220A8397B1DCDAF), sg.generate.mix(0));
    const c = sg.Config{ .seed = 0, .systems = 2, .planets = 4,
        .civilizations = 2, .side = 8 };
    const a = try sg.generate.system(c, @fromBackingInt(1));
    try std.testing.expectEqual(@as(i32, 889), a.x);
    try std.testing.expectEqual(@as(i32, 369), a.y);
    const b = try sg.generate.system(c, @fromBackingInt(2));
    try std.testing.expectEqual(@as(i32, 4399), b.x);
    try std.testing.expectEqual(@as(i32, 764), b.y);
    const p = try sg.generate.planet(c, @fromBackingInt(4));
    try std.testing.expectEqual(@as(u32, 2), @backingInt(p.system));
    try std.testing.expectEqual(@as(u16, 1), p.orbit_index);
    try std.testing.expectEqual(@as(u64, 0xDAA66D2C7DDF743F), p.terrain_seed);
    try std.testing.expectEqual(@as(usize, 177), f.small_wire.len);
}
test "config precedence and reservation" {
    try std.testing.expectError(error.InvalidVersion,
        sg.validateConfig(.{ .version = 2, .systems = 0, .planets = 0 }));
    try std.testing.expectError(error.CapacityUnsupported,
        sg.validateConfig(.{ .systems = 65, .planets = 1 }));
    try std.testing.expectError(error.InvalidConfig,
        sg.validateConfig(.{ .systems = 2, .planets = 1, .civilizations = 1 }));
    try std.testing.expect(sg.storage.candidateReservation() <= 131_072);
}
```

- [ ] **Step3: Run RED.** From repository root: `zig test zig/src/seeded_galaxy_tests.zig --test-filter 'independent seed zero'`. Expected missing new facade functions, then incorrect-value assertion after declarations exist. Once declarations compile, retain at least one behavior-level RED before implementation; a broken import alone is not a regression witness.
- [ ] **Step4: Implement formulas and independent reference script.** Use the exact mix code and fixture algorithm below; reference script uses its own loops and `struct` packing rather than calling product code.

```zig
pub fn mix(input: u64) u64 {
    var value = input +% 0x9E3779B97F4A7C15;
    value = (value ^ (value >> 30)) *% 0xBF58476D1CE4E5B9;
    value = (value ^ (value >> 27)) *% 0x94D049BB133111EB;
    return value ^ (value >> 31);
}
pub fn draw(seed: u64, domain: u8, id: u32, field: u16) u64 {
    return mix(seed ^ (@as(u64, domain) << 56) ^ (@as(u64, id) << 16) ^ field);
}
```

The optional checked-in oracle contains this independent generation path and runs assertions before any source generator is implemented. It prints no replacement fixture files:

```python
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
            (p - 1) // systems, seed ^ (((p - 1) * 0x9E3779B97F4A7C15) & MASK), 8)
    for c in range(1, civilizations + 1):
        result += struct.pack("<IIB", c, c, 0)
    return result
assert mix(0) == 0xE220A8397B1DCDAF
assert hashlib.sha256(wire(0, 2, 4, 2)).hexdigest() == "d2453927e7ba740f64ade02f4a63175fcf45d343b32817df54bbba86e9ec72b1"
assert hashlib.sha256(wire(0, 64, 512, 16)).hexdigest() == "9a851413f0c528643d4360e2219ed67d340af083b2f3828a368551cd3e1e5852"
assert hashlib.sha256(wire(MASK, 64, 512, 16)).hexdigest() == "8064e196c4258200079e25ff644ada6d241401f0ac5b79d093247bed89f4ef30"
assert len(wire(0, 2, 4, 2)) == 177
assert len(wire(0, 64, 512, 16)) == 11455
```

Run `python3 zig/tools/seeded-galaxy-reference.py` before implementing formulas, then compare the entire small wire against the spec literal. Python is optional developer verification; required delivery/gates remain Zig-only. The literal witnesses still pin source tests when Python is absent.

Independent maximum witnesses, recomputed during planning using masked Python arithmetic/`struct.pack`/`hashlib.sha256`:

| Seed, config64/512/16/side8 | Wire bytes | SHA256 | Last system x,y |
| --- | --- | --- | --- |
| 0 | 11455 | `9a851413f0c528643d4360e2219ed67d340af083b2f3828a368551cd3e1e5852` |29603,29224|
|18446744073709551615|11455|`8064e196c4258200079e25ff644ada6d241401f0ac5b79d093247bed89f4ef30`|29066,29521|

For each record ID, calculate the spec formula directly. Add all valid side endpoints4/32, minimum1/1/1, seedmax, invalid zero/max IDs, repeated generation, every max-capacity membership/orbit/home, pairwise position uniqueness, and distance symmetry/zero/3-4-5/extreme-i32 tests. Side does not alter coordinate draws; there are no ecology fields.

- [ ] **Step5: Implement measured storage shape and fail-closed size assertions.** Store typed arrays once and canonical output bytes once; scratch owns the decoder snapshot and fixed hashing/cursor state. Task2 allocates these exact types using `allocator.create(Output)` and `allocator.create(Scratch)` with `errdefer` cleanup; Task1 implements only the layout/reservation proof. Task2 initializes bounded control/header fields synchronously and each record in its charged phase, with no eager whole-array zeroing during poll.

```zig
pub const Output = struct {
    config: Config, systems: [64]System, planets: [512]Planet,
    civilizations: [16]Civilization, wire: [16384]u8,
    digest: [32]u8, wire_len: u16,
};
pub const Scratch = struct {
    input: [16384]u8, sha: std.crypto.hash.sha2.Sha256,
    block: [64]u8, input_len: u16, parsed_cursor: u16,
    record_cursor: u16, block_cursor: u16,
    phase: u8, mode: u8, total_units: u64,
};
comptime {
    if (@sizeOf(Output) > 65_536 or @sizeOf(Scratch) > 65_536)
        @compileError("strategic allocation ceiling exceeded");
    if (@sizeOf(System) > 256 or @sizeOf(Planet) > 256 or @sizeOf(Civilization) > 256)
        @compileError("strategic record work bound exceeded");
}
```

Measured on pinned compiler during planning: Config24/align8, System16/4, Planet24/8, Civilization12/4, Output29,952/8, Scratch16,592/16. Product tests must verify actual layouts again after controller fields exist. `candidateReservation` includes Output+Scratch+controller+returned handles+worst alignment padding; do not report only46,544 heap bytes as the aggregate. Reserve a fixed1,024-byte control envelope and two worst alignment pads while asserting actual controls fit it: upper bound47,590 bytes at this layout (46,544 + 1,024 + 7 + 15). Detached output reservation includes its owning handle and padding, conservatively<=65,536. Replacement peak is the sum, with no hidden exclusion of the previous resident output.

- [ ] **Step6: GREEN and independent review.** Run `zig fmt --check zig/src/seeded_galaxy zig/src/seeded_galaxy_tests.zig`, `zig test zig/src/seeded_galaxy_tests.zig`, and the same test with `-O ReleaseSafe`. Reviewer checks literal provenance, formula arithmetic, exact layouts, role ID compatibility and no Classic edits. Record exact file SHA256s and exit statuses; do not commit.

### Task 2: Complete incremental creation, ownership, wire export and charged hashing

**Files:** Create `setup.zig`, `manifest.zig`, `wire.zig`, `hash.zig`, `lifecycle_tests.zig`, `test_support.zig`; extend new-only storage/types/facade/aggregate. Encoder belongs here; decoder entry point is introduced in Task3.

**Interfaces:** Consume Task1 records/formulas. Produce the following; `Setup` is a stable, caller-owned value with allocator, namespace, checked next counters, optional live token, output/scratch pointers, status/cancellation/error and stats inline. It contains no pointer to its own stack storage. Values holding ownership must not be copied by assignment; transfer only through `take`.

```zig
pub const Token = struct { owner_namespace: u64, id: u64, generation: u64 };
// Setup methods:
pub fn init(allocator: std.mem.Allocator, owner_namespace: u64) Failure!Setup;
pub fn beginCreate(self: *Setup, config: Config) Failure!Token;
pub fn poll(self: *Setup, token: Token, budget: u16) Failure!Progress;
pub fn cancel(self: *Setup, token: Token) Failure!void;
pub fn take(self: *Setup, token: Token) Failure!Manifest;
pub fn release(self: *Setup, token: Token) Failure!void;
pub fn stats(self: *const Setup, token: Token) Failure!Stats;
pub fn deinit(self: *Setup) void;
// Manifest methods; owner fields are allocator plus nullable Output pointer:
pub fn config(self: *const Manifest) Failure!Config;
pub fn digest(self: *const Manifest) Failure![32]u8;
pub fn systems(self: *const Manifest, start: u16, out: []System) Failure!Page;
pub fn planets(self: *const Manifest, start: u16, out: []Planet) Failure!Page;
pub fn civilizations(self: *const Manifest, start: u16, out: []Civilization) Failure!Page;
pub fn wireRecords(self: *const Manifest, start: u16, out: []WireRecord) Failure!Page;
pub fn deinit(self: *Manifest) void;
// wire.zig; all writes target fixed owned storage, no allocation:
pub fn encodeHeader(config: Config, out: []u8) Failure!u16;
pub fn encodeSystem(value: System, out: []u8) Failure!u16;
pub fn encodePlanet(value: Planet, out: []u8) Failure!u16;
pub fn encodeCivilization(value: Civilization, out: []u8) Failure!u16;
// hash.zig; true means all padded blocks have now been compressed:
pub fn advance(scratch: *storage.Scratch, wire: []const u8) Failure!bool;
pub fn finish(scratch: *const storage.Scratch) [32]u8;
```

Namespace0 returns `InvalidConfig`. Host/session supplies fresh monotonic namespaces; there is no global mutable counter. Test fixtures may explicitly set counters near exhaustion within module-local tests; do not expose counter-reset APIs. Closed Setup operations return `Shutdown`; destroyed Manifest queries return `Shutdown` before accessing its null pointer. Admission order is **Shutdown → Busy → config validation (version, dimensions, relationships) → IdExhausted → allocation**. Decode later substitutes byte-length admission for config validation. Length>16,384 returns `InvalidWire`. Token methods check Shutdown→StaleJob→request validation. `release` while generation/cleanup remains pending returns `NotReady`.

- [ ] **Step1: Add lifecycle behavior RED and bounded polling helper.** `test_support.zig` defines `finish(setup:*Setup, token:Token, budget:u16) Failure!void`, shown below, plus `collectWire(manifest:*const Manifest, out:[]u8) Failure!u16`, which pages WireRecord values, checks output length, and copies the bounded record bytes without borrowing manifest storage. Helpers are test-only; the production path never drains a job secretly.

```zig
pub fn collectWire(manifest: *const sg.Manifest, out: []u8) sg.Failure!u16 {
    var row: [1]sg.WireRecord = undefined;
    var cursor: u16 = 0;
    var written: usize = 0;
    while (true) {
        const page = try manifest.wireRecords(cursor, &row);
        if (page.written == 1) {
            const len: usize = row[0].len;
            if (written > out.len or len > out.len - written) return error.InvalidConfig;
            @memcpy(out[written..][0..len], row[0].bytes[0..len]);
            written += len;
        }
        cursor = page.next orelse return @intCast(written);
    }
}
pub fn finish(setup: *sg.Setup, token: sg.Token, budget: u16) sg.Failure!void {
    if (budget == 0) return error.InvalidBudget;
    for (0..10_000) |_| switch (try setup.poll(token, budget)) {
        .pending => |stats| if (stats.units_used > budget) return error.InvariantViolation,
        .ready => return,
        .failed => |cause| return cause,
        .canceled => return error.NotReady,
    };
    return error.InvariantViolation;
}
test "owned ready take reopens lane and invalidates token" {
    var setup = try sg.Setup.init(std.testing.allocator, 1);
    defer setup.deinit();
    const token = try setup.beginCreate(f.small_config);
    try std.testing.expectError(error.Busy, setup.beginCreate(f.small_config));
    try std.testing.expectError(error.NotReady, setup.take(token));
    const before = try setup.stats(token);
    _ = try setup.poll(token, 0);
    try std.testing.expectEqualDeep(before, try setup.stats(token));
    try h.finish(&setup, token, 1);
    var manifest = try setup.take(token);
    defer manifest.deinit();
    try std.testing.expectError(error.StaleJob, setup.take(token));
    try std.testing.expectEqualSlices(u8, &f.small_digest, &(try manifest.digest()));
    const replacement = try setup.beginCreate(f.small_config);
    try std.testing.expect(replacement.id > token.id);
    try std.testing.expectError(error.StaleJob, setup.poll(token, 1));
    try setup.cancel(replacement);
    while (true) switch (try setup.poll(replacement, 1)) {
        .canceled => break,
        .pending => {},
        else => return error.InvariantViolation,
    };
    try setup.release(replacement);
    try std.testing.expectEqualSlices(u8, &f.small_digest, &(try manifest.digest()));
}
```

Run RED: `zig test zig/src/seeded_galaxy_tests.zig --test-filter 'owned ready take'`. Retain failing behavioral output after API stubs compile.

- [ ] **Step2: Implement the single checked lane and terminal table.** Reserve ID+generation before allocation only after idle/input checks; either both checked increments succeed or neither changes. Allocation failure may consume the reserved pair and must free the first allocation if the second fails. Ready holds the lane. `take` moves output+allocator, frees scratch, clears active token and reopens the lane. `release` frees ready output/scratch or releases an already cleaned failed/canceled outcome. `deinit` frees at most two allocations, clears token and closes the controller. Repeated deinit does nothing. Repeated cancel stays pending/canceled; cancel ready enters cleanup and cannot publish. Cancel/take/release after token invalidation returns `StaleJob`.

Add explicit `retire_output`/`retire_scratch` phases. Each cleanup free costs one poll unit, so budget0 never frees; terminal canceled/failed is published only after owned pointers are null. Error cause lives in Setup control after scratch retires. Internal arithmetic/invariant failures enter the same cleanup path. Request errors (invalid budget, stale token, busy admission) preserve the active phase, cursor and allocation counters.

- [ ] **Step3: Implement charged creation and exact encoder.** One unit validates bounded config, one generates each record, one validates each record independently against its ID/count/formula, one encodes header, one encodes each record, one compresses each padded SHA block, one extracts digest and marks ready. Check budget before mutation and yield immediately when exhausted; transitions through a fixed number of empty phases do no scans. All integer writes use `std.mem.writeInt` with explicit little endian widths; coordinates are bitcast to signed representation as required. No `std.mem.asBytes` hashing, enum-memory serialization or whole-message hashing.

For R=systems+planets+civilizations and L=47+16*systems+20*planets+9*civilizations, successful creation charges exactly `3*R + 3 + ceil((L+9)/64)` units. The small fixture charges30; maximum metadata charges1,959. Keep zero-budget reads from resetting even diagnostic `units_used`.

- [ ] **Step4: Hash one complete padded block per call.** Use the pinned standard library's `Sha256.init(.{})`, `update` and chaining words `s`. `update` receives exactly64 bytes with `buf_len==0`; prepare each block with at most64 byte operations and append the original bit length big-endian in the last8 bytes. Do not call `final`, `finalResult`, `peek` or whole-message `hash` in production: they hide one/two compression blocks. Read `s` only in the separately charged extraction step. This relies on the pinned standard-library layout and must be reviewed when the compiler changes.

```zig
// In hash.advance, after constructing the current canonical padded block:
if (scratch.sha.buf_len != 0) return error.InvariantViolation;
scratch.sha.update(&scratch.block); // exactly one compression of 64 bytes
scratch.block_cursor += 1;
// hash.finish performs only these eight fixed writes:
var digest: [32]u8 = undefined;
for (scratch.sha.s, 0..) |word, index|
    std.mem.writeInt(u32, digest[index * 4 ..][0..4], word, .big);
return digest;
```

Test messages of lengths0,1,55,56,63,64,65,119,120 and the fixture length against an independent standard-library whole-message hash **in tests only**, verifying compression counts. Add literal known SHA256 empty and `abc` vectors. Every hash test also drives budget1 through both padding blocks when required.

- [ ] **Step5: Exercise lifecycle failures and accounting.** Table-drive foreign namespaces, changed token generation/ID, token from released outcome, shutdown/reinit with a new namespace, ID/generation exhaustion, cancellation before first unit/each phase/ready, repeated cancel/deinit, ready release and both allocation failures. `std.testing.FailingAllocator` records allocation/free counts; wrap its allocator with a test counter to assert every `poll` and copied query makes zero allocation attempts. Iterate cancel cuts0..1,959 for maximum generation at budget1; each cut creates a fresh isolated setup and compares a retained resident's copied digest before/after. Retain the old resident while taking replacement, then destroy old-first and new-first in separate tests. Include handles/control/alignment in peak values and assert no more than one resident plus one candidate in the session fixture.

- [ ] **Step6: GREEN and reviewer.** Run focused aggregate Debug and ReleaseSafe plus format check. Require literal177-byte wire equality, small/max digests, exact30/1,959 units, cleanup free counts and reservation assertions. Independent reviewer checks actual source hashes, no escaped slices, SHA block charging and the complete token terminal table before Task3.

### Task 3: Own and validate decoded wire incrementally

**Files:** Modify only new `setup.zig`, `wire.zig`, storage/types/facade; create `wire_tests.zig`; extend new aggregate.

**Interfaces:** Consume Task2 lifecycle/hash/encoder and Task1 formulas. Add `Setup.beginDecode(self:*Setup, input:[]const u8) Failure!Token`. Wire parser returns copied records and consumed lengths without allocating:

```zig
pub const ParsedHeader = struct { config: Config, consumed: u16 };
pub fn parseHeader(input: []const u8) Failure!ParsedHeader;
pub fn parseSystem(input: []const u8) Failure!System;
pub fn parsePlanet(input: []const u8) Failure!Planet;
pub fn parseCivilization(input: []const u8) Failure!Civilization;
```

Header parse is bounded to47 canonical bytes; validate magic, wire version, identity length/exact identity, then generator version/counts. Short header is `InvalidWire`, wrong wire version is `UnsupportedWireVersion`, wrong generator version is `InvalidVersion`; bounded invalid dimensions are `CapacityUnsupported`, invalid count relationships `InvalidConfig`. Header parse checks enough bytes before each read and never indexes a user-controlled identity length. Compute expected total length with checked integers after config validation; shorter/longer input is `InvalidWire` before parsing entity records. Record parsers accept exact16/20/9-byte slices, reject nonzero z/kind and never read padding.

- [ ] **Step1: Add RED for the owned-input boundary and roundtrip.** Define imports `sg`, `f`, `h`, `std` as in earlier tests.

```zig
test "decode owns caller snapshot and recomputes canonical bytes" {
    var setup = try sg.Setup.init(std.testing.allocator, 7);
    defer setup.deinit();
    var caller = f.small_wire;
    const token = try setup.beginDecode(&caller);
    @memset(&caller, 0xAA);
    try h.finish(&setup, token, 1);
    var manifest = try setup.take(token);
    defer manifest.deinit();
    var copied: [16384]u8 = undefined;
    const len = try h.collectWire(&manifest, &copied);
    try std.testing.expectEqualSlices(u8, &f.small_wire, copied[0..len]);
    try std.testing.expectEqualSlices(u8, &f.small_digest, &(try manifest.digest()));
}
test "oversize is synchronous but malformed header is polled" {
    var setup = try sg.Setup.init(std.testing.allocator, 8);
    defer setup.deinit();
    const oversized: [16385]u8 = @splat(0);
    try std.testing.expectError(error.InvalidWire, setup.beginDecode(&oversized));
    const token = try setup.beginDecode("NY");
    try std.testing.expectError(error.InvalidWire, h.finish(&setup, token, 1));
    try setup.release(token);
}
```

Run RED: `zig test zig/src/seeded_galaxy_tests.zig --test-filter 'decode owns'` and `--test-filter 'oversize'` separately.

- [ ] **Step2: Implement bounded admission and charged decode.** Admission precedence is Shutdown→Busy→oversized input→IdExhausted→allocation. Snapshot-copy0..16,384 bytes synchronously into scratch; count `admission_copy_bytes` separately from poll units. Do not parse/hash/recompute in begin. A truncated empty input is admitted then fails during poll. Header parse costs1; config validation costs1; each record parse costs1; each full recomputation/compare costs1; header encoding costs1; each record encoding costs1; SHA and finalization use Task2 rules. Decode charges `3*R + 4 + ceil((L+9)/64)` successful units (31 small,1,960 max). Its single output array holds parsed values; compare with one stack-local generated record, never allocate another manifest. Re-encoding canonical parsed records into output wire provides the independently checked exact digest.

- [ ] **Step3: Enumerate rejection classes and precedence.** For every byte truncation of177-byte fixture, one trailing byte, magic/identity length/identity corruption, generator/wire version, counts0/outside capacities/inconsistent, side3/33, reordered/duplicated/zero IDs, changed membership/orbit/terrain seed/home, nonzero coordinate z/kind and one x/y bit flip, assert exact terminal failure and no ready/take. Wrong-coordinate wire accompanied by its correctly recomputed SHA still fails generated-value comparison: digest authenticity is not schema validity. Mutate seed without regenerating records to prove recomputation.

Add overlapping failures: busy+oversized returns Busy without counter reservation; closed+oversized returns Shutdown; oversized+exhausted returns InvalidWire; creation wrong version+exhausted returns InvalidVersion; admitted wrong-version bytes+exhausted returns IdExhausted before parse. Snapshot allocation failures consume no live lane and leak nothing. Invalid poll budget4097 and foreign token preserve the valid snapshot/cursor. Cancel at all1,960 maximum decode cuts; verify resident manifest unchanged and zero poll allocations.

- [ ] **Step4: GREEN and reviewer.** Run aggregate Debug/ReleaseSafe and format. Replay small/max literals under budgets1,7,256,4096 and deterministic schedules containing0; compare wire/digest/total units exactly. Record begin-copy latency separately from poll timing. Reviewer checks every input read is bounded, exact error precedence, snapshot ownership, no double-output allocation and no whole-manifest validation scan.

### Task 4: Copy queries, enforce role selection and bridge exact v0 configuration

**Files:** Create `policy.zig`, `bridge.zig`, `policy_bridge_tests.zig`; extend new manifest/facade/aggregate only. Existing role and coordinated types remain read-only.

**Interfaces:** Consume ready Manifest, `play_role.SessionPolicy/Provenance/CivilizationId` and unmodified `coordinated_galaxy/types.zig` as `cg`. Produce:

```zig
pub fn validate(manifest: *const Manifest, policy: SessionPolicy) Failure!void;
pub fn admit(manifest: *const Manifest, policy: SessionPolicy,
    provenance: Provenance, actor: ?CivilizationId) Failure!void;
pub const BridgeOptions = struct {
    planet_capacities: []const cg.PlanetCapacities,
    global_history_capacity: u16,
};
pub fn toCoordinatedConfig(manifest: *const Manifest, options: BridgeOptions) Failure!cg.Config;
```

`policy.validate` calls existing `SessionPolicy.init` role requirements first, then verifies any selection lies within1..civilizations; map admission errors to `InvalidPolicy`. `policy.admit` calls existing policy.admit first, then policy.validate/presence, mapping each refusal to `InvalidPolicy`. Creator may have null/present selection; ruler/combined require present selection; autonomous always refused. Neither function changes manifest/session or grants assets. Policy is external to manifest digest.

Bridge checks live manifest first, strategic counts<=4 second (`CapacityUnsupported`), then capacity slice length exactly planet_count (`InvalidConfig`), then copies every supplied capacity/global_history value and runs unmodified `cg.validateConfig`; its invalid local/global capacity returns `InvalidConfig`. No allocation or truncation. IDs translate by exact backing integer, seed/counts/side/membership are copied, not regenerated with new choices. Unused v0 array slots keep normal Config defaults and are not treated as supplied active planets.

- [ ] **Step1: Add RED for full metadata rejection and exact bridge values.** Use the existing test imports and helper `h.finish`.

```zig
test "maximum metadata cannot masquerade as four world simulation" {
    var setup = try sg.Setup.init(std.testing.allocator, 20);
    defer setup.deinit();
    const token = try setup.beginCreate(.{ .systems = 64, .planets = 512,
        .civilizations = 16, .side = 8 });
    try h.finish(&setup, token, 256);
    var manifest = try setup.take(token);
    defer manifest.deinit();
    const before = try manifest.digest();
    const caps: [4]cg.PlanetCapacities = @splat(.{});
    try std.testing.expectError(error.CapacityUnsupported,
        sg.bridge.toCoordinatedConfig(&manifest, .{
            .planet_capacities = &caps, .global_history_capacity = 4096 }));
    try std.testing.expectEqualSlices(u8, &before, &(try manifest.digest()));
}
test "bridge preserves explicitly supplied capacities" {
    var setup = try sg.Setup.init(std.testing.allocator, 21);
    defer setup.deinit();
    const token = try setup.beginCreate(f.small_config);
    try h.finish(&setup, token, 7);
    var manifest = try setup.take(token);
    defer manifest.deinit();
    const caps = [_]cg.PlanetCapacities{
        .{ .settlements = 1, .buildings = 2, .freight = 3, .history_receipts = 4 },
        .{ .settlements = 5, .buildings = 6, .freight = 7, .history_receipts = 8 },
        .{ .settlements = 9, .buildings = 10, .freight = 11, .history_receipts = 12 },
        .{ .settlements = 13, .buildings = 14, .freight = 15, .history_receipts = 16 },
    };
    const config = try sg.bridge.toCoordinatedConfig(&manifest, .{
        .planet_capacities = &caps, .global_history_capacity = 17 });
    try std.testing.expectEqual(@as(u16, 17), config.global_history_capacity);
    try std.testing.expectEqual(@as(u8, 4), config.planet_count);
    for (caps, 0..) |cap, index| {
        try std.testing.expectEqualDeep(cap, config.planets[index].capacities);
        try std.testing.expectEqual(@as(u32, 1 + @as(u32, @intCast(index % 2))),
            @backingInt(config.planets[index].system));
        try std.testing.expectEqual(@as(u16, 8), config.planets[index].side);
    }
    try cg.validateConfig(config);
}
```

Run RED: `zig test zig/src/seeded_galaxy_tests.zig --test-filter 'bridge preserves'` and `--test-filter 'maximum metadata'`.

- [ ] **Step2: Implement pure bridge and policy.** Copy at most4 planet records into stack values through copied queries, set explicit Config fields, validate and return by value. Do not begin an authority job. Use exact role admission code already present, wrapping typed errors, with no duplicated role enum or permissive fallback. Add role matrix across3 roles×3 provenances×actor null/selected/other and selected null/0/present/absent; assert expected InvalidPolicy/acceptance and invariant manifest digest after every case. Require selected civilization2 only when present, reject selection17 even for creator, and never convert ruler selection failure into creator mode.

- [ ] **Step3: Close query ownership and bounds coverage.** Exercise page capacities1,64,0,65; start0,last,count,count+1; every512 planet record exactly once; wire header/entity boundaries; independent copies survive another query and Setup.deinit. Manifest.deinit invalidates its own future queries safely. Returned Page uses written:u8<=64 and next:u16 sufficient for593 wire records. Verify all founding values have kind uninitialized and no API permits a settlement or grant. Names are derived in presentation only and do not affect digest.

- [ ] **Step4: Verify bridge boundary with no resident mutations.** Test every count dimension5 while others fit, local side4/32, history1/4096/0/4097, each local capacity0/upper/upper+1 and capacity slice length mismatch. Track allocator count unchanged throughout bridge/policy. Compare returned Config field-by-field with a separately authored v0 Config including different planet capacities. Record **Current: exact Config equivalence and unmodified v0 validation**. Record **Pending integration: exact local/global v0 genesis bytes/head equivalence**, because importing active coordinator runtime here would violate ownership and create a false dependency. No strategic digest is appended to v0 genesis.

- [ ] **Step5: GREEN and reviewer.** Run focused aggregate Debug/ReleaseSafe and format. Reviewer checks count-first rejection, every supplied capacity, policy precedence, copied-value boundaries, retained-resident witness and explicit distinction between metadata digest and v0 history head.

### Task 5: Standalone public journey, exhaustive qualification and coordinated handoff

**Files:** Create `zig/src/seeded_galaxy_cli.zig`, `zig/src/seeded_galaxy_cli_main.zig`, `zig/src/seeded_galaxy_cli_tests.zig`, `qualification_tests.zig`, qualification report; extend only strategic aggregate. Shared root/main/build/check and coordinator source stay outside ownership until their current owner explicitly hands them over.

**Interfaces:** `cli.run(allocator:std.mem.Allocator, args:[]const []const u8, out:*std.Io.Writer) !void`. CLI arguments: `--seed DECIMAL_U64`, `--systems 1..64`, `--planets 1..512`, `--civilizations 1..16`, `--side 4..32`, `--budget 1..4096`, `--role creator|ruler|combined`, `--civilization 1..16`, with a space between flag and value. Defaults config12/24/4/8, budget256, explicit default creator policy with no civilization. Unknown/duplicate/missing/overflow/zero-budget arguments return `InvalidArguments`, write one ERR row and exit nonzero. Decimal seed accepts0/max and rejects negative/overflow. No filesystem or profile arguments exist.

The successful CLI journey generates via Setup, takes Manifest, copies all three record tables and wire records, independently decodes that owned export via the same controller, checks bytes/digest equality, validates policy, attempts the bridge using explicitly specified fixed local capacities64/256/64/4096 and global4096, then prints deterministic summary. Bridge CapacityUnsupported is the expected large-metadata outcome and is printed as a refusal, not success at full simulation. Invalid policy fails. Exact headings: `IDENTITY`, `CONFIG`, `SYSTEM`, `PLANET`, `CIVILIZATION`, `DIGEST`, `DECODE`, `POLICY`, `BRIDGE`, `SCOPE`. Last row: `SCOPE\tstrategic-metadata-only`. Names `System N`/`Civilization N` remain presentation values. No tick or economy-success row is printed.

The validation roundtrip keeps at most two detached outputs; destroy the decoded verification output before any new begin. Its16,384-byte export buffer and CLI output buffer are caller scratch outside the setup reservation and must appear separately in process/CLI accounting; do not disguise them as zero cost. The session replacement cap still governs setup+resident owners. `test_support.collectWire` uses one WireRecord stack value per query rather than a64-record temporary; qualification reports all caller scratch separately. Use `defer` immediately for Setup and each taken Manifest so writer failure at any row cleans up without draining hidden gameplay work.

- [ ] **Step1: Add CLI RED using an allocating test writer.** The pinned standard library provides `std.Io.Writer.Allocating.init(allocator)`, `.writer`, `.written()` and `.deinit()`; use these directly. Tests import the CLI directly, not the shared root module.

```zig
const std = @import("std");
const cli = @import("seeded_galaxy_cli.zig");
test "standalone journey reports large bridge refusal and decode" {
    var out = std.Io.Writer.Allocating.init(std.testing.allocator);
    defer out.deinit();
    try cli.run(std.testing.allocator, &.{ "--seed", "0", "--systems", "64",
        "--planets", "512", "--civilizations", "16", "--budget", "7" }, &out.writer);
    const text = out.written();
    try std.testing.expect(std.mem.indexOf(u8, text, "DECODE\tverified") != null);
    try std.testing.expect(std.mem.indexOf(u8, text, "BRIDGE\tCapacityUnsupported") != null);
    try std.testing.expect(std.mem.indexOf(u8, text, "SCOPE\tstrategic-metadata-only") != null);
}
```

Run RED: `zig test zig/src/seeded_galaxy_cli_tests.zig --test-filter 'standalone journey'`.

- [ ] **Step2: Implement writer-based flow and standalone main.** Use the existing repository's pinned `std.process.Init`/`writerStreaming` pattern without importing shared `main.zig` or adding a build target.

```zig
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
```

Add a small2/4/2 journey with ruler selection2 and exact expected177-byte digest, creator null, combined selection1, bad selection and max wire cases. A writer that returns `WriteFailed` at each possible output boundary must leave the tracking allocator at zero outstanding allocations. Buffer all output only in tests; product writes incrementally.

- [ ] **Step3: Run deterministic public journeys and isolation gates.** From canonical repository root, save full command output to bounded task-specific log files, then inspect exit code and final4,000 bytes; a log pipeline's status must not replace the command's status.

```sh
zig fmt --check zig/src/seeded_galaxy zig/src/seeded_galaxy_tests.zig zig/src/seeded_galaxy_cli.zig zig/src/seeded_galaxy_cli_main.zig zig/src/seeded_galaxy_cli_tests.zig
zig test zig/src/seeded_galaxy_tests.zig
zig test zig/src/seeded_galaxy_tests.zig -O ReleaseSafe
zig test zig/src/seeded_galaxy_cli_tests.zig
zig test zig/src/seeded_galaxy_cli_tests.zig -O ReleaseSafe
zig run zig/src/seeded_galaxy_cli_main.zig -O ReleaseSafe -- --seed 0 --systems 2 --planets 4 --civilizations 2 --budget 1 --role ruler --civilization 2
zig run zig/src/seeded_galaxy_cli_main.zig -O ReleaseSafe -- --seed 18446744073709551615 --systems 64 --planets 512 --civilizations 16 --budget 256
```

Add deterministic schedule tests driven by an independent test-only integer recurrence, including0,1,7,256,4096 and repeated0 before progress. Compare all wire/digest outputs to independent literals and all total units to the formula, never just one implementation run to another. Cancellation/retry uses new tokens and produces the same bytes without consuming gameplay time. Exhaust allocation failures through all begin paths and verify poll/query allocation attempt count is zero.

- [ ] **Step4: Measure maximum metadata, admission and poll bounds.** On named hardware, record OS, architecture, compiler, optimization and exact source hashes. Use `std.Io.Clock.awake.now(init.io)` before and after each call; compute `before.durationTo(after).raw.toNanoseconds()` with the pinned duration API for monotonic timing. Timing instrumentation is outside authority bytes and not part of deterministic state. At least100 maximum-create and100 maximum-decode runs, budget256; report sample count, p95 and maximum per nonzero poll, beginDecode16,384-byte copy time separately, observed work units, requested heap and aggregate reservation, detached bytes, CLI buffers and allocation attempts. Target p95<=2ms/max<=8ms is performance evidence, not a substitute for work-unit proof. If unmet, record measured result and investigate within owned code; never weaken bounds or relabel the target passed.

- [ ] **Step5: Execute the repository gate without claiming integration it does not cover.** The current Zig gate is `./tools/check.sh` from `zig/`; it blocks Rust subprocesses itself. Run it after focused tests. Preserve active-owner failures and classify exact source ownership; do not change coordinator/build/root to make the global gate green. A focused strategic PASS remains Current if the shared gate is independently failing, but integrated acceptance is Partial with the exact failure recorded. The aggregate may not yet be imported by `zig/src/root.zig`, so its focused test evidence remains mandatory even when this existing gate passes.

- [ ] **Step6: Complete independent source review and document concrete shared-owner handoff.** Reviewer checks every newly owned file/diff, exact source hashes, spec/plan, all literal witnesses, lifecycle accounting, parser rejection corpus and standalone output. Record tests and proof gaps in the qualification report. No claims of a playable full galaxy, browser delivery, persistence, native UI or512 resident economies follow from this slice.

The later shared integration owner must explicitly acquire ownership of `zig/src/root.zig`, `zig/src/main.zig`, `zig/build.zig`, `zig/tools/check.sh` and any coordinator acceptance aggregate before edits. That handoff must: import the strategic aggregate into the root test closure; expose the facade and CLI command; add standalone CLI/build qualification to the required gate; compare identical bridged/manual v0 Config through the **finished, unmodified** v0 creation path and require exact local/global genesis bytes and heads; rerun Debug/ReleaseSafe/full Zig gate; and preserve strategic digest as a separate association. Read the current coordinator API at handoff, rather than inventing a future method name in this isolated plan. If no finished v0 creation/replay surface exists yet, exact-genesis qualification remains a recorded dependency and no Config-only assertion is promoted to byte equivalence.

This plan's standalone slice is complete only with focused gates, source review and truthful handoff evidence. Shared integration is complete only after the explicit handoff obligations pass. Full NYON delivery additionally needs the expanded roadmap's storage/coordinator scale, founding, fleets, civilization strategy, saves and native/browser gameplay. Rust-source removal is authorized by Donald's08:23 EDT instruction and follows equivalent Zig replacement/qualification; it is outside these seeded file owners and never deletes old user data.

## Self-review and executor evidence checklist

- [ ] The final spec revision and this plan's exact hash are recorded before dispatch; all independent review findings have explicit dispositions.
- [ ] Every spec section maps to a task: generator/capacity/layout1; lifecycle/canonical encode/hash2; decoder3; roles/bridge/query4; independent schedules/cancellation/timing/CLI and handoff5.
- [ ] Review Focus1–2 map to Task2,3 to Task3,4 to Task4,5 to Tasks2/3/5; exact failure assertions and allocation witnesses accompany each.
- [ ] Shared signatures match across tasks; all helpers used in tests are defined above or implemented in explicitly owned test-support files.
- [ ] Independently supplied byte/digest fixtures never come from product encoder output; handwritten mix and seedmax witnesses remain literal.
- [ ] Task implementer and reviewer evidence includes uncommitted source hashes; no HEAD-only completion claim.
- [ ] Report separates Current focused code/tests, measured timing, Partial shared integration, and remaining full-game delivery; no commits or unauthorized shared-file changes.


Author self-review completed2026-10-11: checked all specification sections against the five tasks, copied-interface consistency, no undefined external runtime dependency, SHA padding charging, exact error precedence, active owner exclusions, independent literals and Review Focus coverage. The plan deliberately leaves shared integration as an explicit owner handoff; it does not claim that handoff or any product test has run. Planning evidence: pinned compiler layout probe passed1 test; independent Python recomputation matched the small and two maximum literal digests. These are planning checks, not implementation qualification.
