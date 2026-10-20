# NYON Zig Full-scale Authority Implementation Plan

<!-- BEGIN 2026-10-19 rewrite decision -->
> **Canceled on 2026-10-19:** Donald canceled further Zig rewrite work. Unfinished Zig tasks and continuation instructions in this document are superseded; retain its historical evidence and existing source. See [the current implementation decision](../../../REWRITE-DECISION-2026-10-19.md).
<!-- END 2026-10-19 rewrite decision -->

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking. Use one authority writer, followed by a fresh specification reviewer and a fresh code-quality reviewer before starting the next task.

**Goal:** Implement the reviewed initial full-scale local-economy authority with shared sparse storage, bounded work, atomic all-world publication, authenticated complete history and independent replay.

**Architecture:** One owner manages seven preallocated blocks, two root indexes and descriptor-owned transport contexts. All enrolled worlds share one command lane, budget, clock, quota ledger and publication point. New modules live under `zig/src/fullscale_authority/`; existing planet-economy, coordinated-galaxy and seeded identities remain unchanged.

**Tech Stack:** Exactly Zig `0.18.0-dev.146+35accc06e`, standard library only; Python standard library for independent reference fixtures; no Rust dependency.

**Spec:**

- `docs/superpowers/specs/2026-10-11-nyon-zig-fullscale-authority-design.md`
  - Approved-for-planning SHA256: `a8bc0d41767722b3215e2e6c334d06d5cf5cacc3107f28f83b3b4219ba6ce2f5`.
- `docs/superpowers/specs/2026-10-11-nyon-zig-fullscale-authority-codec.md`
  - Approved-for-planning SHA256: `c116655859eae999ee1529180521254a575570ba471099fc0ca2e00c8778b6b9`.
- Review chain:
  - `docs/qualification/2026-10-11-fullscale-authority-design-review.md`
  - `docs/qualification/2026-10-11-fullscale-authority-design-fix1-review.md`
  - `docs/qualification/2026-10-11-fullscale-authority-design-fix2-review.md`

**Status:** Proposed implementation plan. Specification approval for planning is not implementation approval. Task 0 compatibility acceptance and Task 1 compiler layout witness are hard prerequisites for product implementation.

## Global Constraints

- Compiler: exactly `0.18.0-dev.146+35accc06e`.
- Rules identity: `nyon.fullscale-galaxy.v1`.
- Storage profile: `nyon.fullscale-storage.v1`.
- Systems/worlds/civilizations: `1..64 / 1..512 / 1..16`.
- Surface side: `4..32`.
- Settlements: `4,096` global, `64/world`.
- Buildings/projects: `16,384` global, `256/world`.
- Occupied claims: `36,864` global, `576/world`, additionally bounded by actual cells.
- Local freight: `4,096` global, `64/world`; `65` path cells/shipment.
- Reserved strategic capacities: `2,048` industries, `2,048` routes, `4,096` strategic freight, `256` fleets, `2,048` hulls and `2,048` extension records. These remain storage envelopes; the initial identity rejects their records and commands.
- Public commands: `1,024/tick`, `1,024` body bytes, `64/world`.
- Leases: `64`; one mutation lane through cleanup; one replay target/session.
- One hour/tick; horizon `1,000,000`; allowance `0..4,096`, default `256`.
- Index: two arrays of `131,072` rows; explicit `24-byte` row layout.
- Pool: `262,144` slots, `272 bytes/slot`; `256-byte` payload.
- Each root owns at most `131,071` pages; at most two detached transition slots.
- Exactly seven allocation blocks totaling `188,809,216` requested bytes.
- Source plus target: `377,618,432` requested bytes.
- Committed history cache: `67,108,864` bytes; candidate stream: `33,554,432` bytes.
- Scratch: `8,388,608` bytes; controls: `2,097,152` bytes; manifest: `65,536` bytes.
- Exactly two transport context slots, `4,096` bytes each, within controls.
- Initial tick: at most `47,106` records and `4,057,265` framed bytes.
- Receipt body at most `2,048` bytes, before its four-byte frame.
- Begin’s sole synchronous bulk-copy exception: at most `1,052,672` command/route bytes.
- One shared allowance covers comparisons, shifts, ownership changes, initialization, phases, serialization, hashing, provider ingestion, publication and cleanup.
- One unit permits at most a `256-byte` copy/zero, one `64-byte` hash block, one index comparison/move, one reference/free-list change, one topology edge or one bounded record operation.
- Zero-budget polling changes no state, including terminal-delivery flags and diagnostic counters.
- Publication performs one prepared descriptor switch; it allocates, hashes, validates and traverses nothing.
- Poll target: at least `10,000` observations, budget `256`, p95 `<=2 ms`, maximum `<=8 ms`.
- Full-step target: p95 `<=5 ms`; measure complete steps separately from polls.
- No edits to old v0 modules, shared `zig/src/root.zig`, `zig/src/main.zig`, `zig/build.zig`, repository gates, instructions, dependencies or other owners’ documents.
- No commits, pushes or PRs under this plan.
- Current city and tactical drafts are moving Proposed dependencies. Their mechanics cannot enter this initial codec through reserved rows, opaque payloads or an unreviewed tag addition.

## Review Focus

1. **Interrupted ownership transfer:** stale bytes in an index hole must never become a second owner. Test cancellation after every preparation, shift, parent-COW and child-COW action, including cancellation while already cleaning up. Tasks 4–5.
2. **Committed transport rollback:** seven sealed leaves plus a partial tail must survive candidate carries, eviction and cancellation while the provider is unavailable. Task 6.
3. **Provider lifetime mismatch:** stale tickets and delayed completion after namespace closure must never copy into authority memory or release another owner’s pin. Task 8.
4. **Whole-tick preservation:** valid command bursts, split records and unavailable durability must backpressure before admission rather than truncate events or fabricate gameplay rejection. Tasks 7, 10–12.
5. **False qualification:** synthetic storage envelopes, native-only tests, short polls and a quota fragment must not be reported as strategic gameplay, Wasm execution, full-step performance or complete replay. Tasks 13–15.

## Current source and ownership map

These are source observations, not implementation claims for full-scale authority:

| Existing source | Reuse decision |
|---|---|
| `zig/src/planet_economy/rules.zig` | Reuse pure `AlgorithmContext`, terrain and footprint formulas through a new adapter. |
| `zig/src/surface.zig` | Reuse qualified cube topology and edge behavior. |
| `zig/src/planet_economy/command.zig` | Behavioral reference. Its cursor requires old `state.Candidate`; do not instantiate an island reservation per world or alter this cursor. |
| `zig/src/planet_economy/phases.zig` | Behavioral reference for arrivals, construction, production, supply and demography. Implement new full-scale cursors. |
| `zig/src/coordinated_galaxy/types.zig` | Reference for existing public distinctions and policy behavior; retain v0 wire/types untouched. |
| `zig/src/seeded_galaxy/generate.zig` | Reuse stateless seeded formulas after that owner’s interface is qualified. |
| `zig/src/seeded_galaxy/wire.zig` | Read-only reference/reuse for frozen manifest fields. |
| `zig/src/seeded_galaxy/manifest.zig` | Existing manifest is an owning allocation. Do not retain a builder or manifest allocation as hidden eighth-block ownership. |
| `zig/tools/check.sh` | Current Rust-free migration gate; integration owner runs and extends it after explicit handoff. |
| `zig/build.zig` | Current shared build. No full-scale edits from this writer. |

The plan’s scoped tests use a new root in `zig/src/`, allowing imports of both new modules and existing pure siblings without modifying shared roots.

## Proposed file boundaries

All listed full-scale paths are new unless a prior task created them.

| Files | Responsibility |
|---|---|
| `zig/src/fullscale_authority_tests.zig` | Isolated test root. |
| `zig/src/fullscale_authority/layout.zig` | Exact block partitions and complete runtime layout witness. |
| `zig/src/fullscale_authority/types.zig` | Value contracts, typed identifiers, errors, budgets and frozen capacities. |
| `zig/src/fullscale_authority/storage.zig` | Seven allocation owners and initialization/shutdown. |
| `zig/src/fullscale_authority/pool.zig` | Page ownership and free-list operations. |
| `zig/src/fullscale_authority/records.zig` | Physical payload fields and path-page access. |
| `zig/src/fullscale_authority/index.zig` | Charged lookup, insertion, deletion and root preparation. |
| `zig/src/fullscale_authority/cow.zig` | Parent/child transitions and candidate cleanup. |
| `zig/src/fullscale_authority/manifest.zig` | Owned fixed-block manifest validation and terrain binding. |
| `zig/src/fullscale_authority/codec.zig` | Initial canonical grammar and incremental encoding/decoding. |
| `zig/src/fullscale_authority/hash.zig` | Metered domain-separated SHA256 work. |
| `zig/src/fullscale_authority/transport.zig` | Descriptor-owned append contexts and authenticated tree. |
| `zig/src/fullscale_authority/history.zig` | Framing, cache slots, candidate bytes and visibility. |
| `zig/src/fullscale_authority/provider.zig` | Pure ticket/pin/source protocol and dispositions. |
| `zig/src/fullscale_authority/view.zig` | Leases and copied, bounded queries. |
| `zig/src/fullscale_authority/local_rules.zig` | Pure-rule adaptation without v0 storage. |
| `zig/src/fullscale_authority/commands.zig` | Snapshot, structural routing, preflight and command execution. |
| `zig/src/fullscale_authority/phases.zig` | All-world economy phases. |
| `zig/src/fullscale_authority/validation.zig` | Incremental canonical/reference/quota checks. |
| `zig/src/fullscale_authority/job.zig` | Lifecycle, prepared barrier, publication and retirement. |
| `zig/src/fullscale_authority/replay.zig` | One-target authenticated replay and transfer. |
| `zig/src/fullscale_authority/root.zig` | New facade only. |
| `zig/src/fullscale_authority/test_support.zig` | Counting allocator, cut-point runner, explicit test fixture owners. |
| `zig/src/fullscale_authority/*_tests.zig` | Behavioral tests owned by the corresponding task. |
| `zig/src/fullscale_authority/fixtures/` | Independently derived checked-in literals and expected values. |
| `zig/tools/fullscale-authority-reference.py` | Independent standard-library reference calculator. |
| `zig/src/fullscale_authority_cli.zig` | Standalone qualification command implementation. |
| `zig/src/fullscale_authority_qualification.zig` | Standalone native executable entry point. |
| `zig/src/fullscale_authority_wasm.zig` | Standalone exported deterministic qualification entry point. |
| `zig/tools/check-fullscale-authority.py` | Output and host qualification runner. |

`session_bridge.zig` is deliberately absent from the implementation map until Task 0 freezes the jointly reviewed bridge. Its eventual file ownership and declarations must be inserted into this plan before product work.

## Shared interface contract

The following are Proposed names and signatures for this plan. Task 1 must instantiate their concrete backing layouts. They are not claims that these declarations exist.

```zig
pub const Digest = [32]u8;
pub const PlanetId = enum(u32) { _ };
pub const CivilizationId = enum(u32) { _ };
pub const DescriptorSlot = enum(u1) { a, b };
pub const PageRef = enum(u32) { none = 0, _ };

pub const WorkBudget = struct { remaining: u16 };
pub const Step = enum { pending, done };

pub const Key = struct {
    realm: u32,
    kind: u16,
    local: u64,
};

pub const Prefix = struct {
    genesis_digest: Digest,
    tick: u64,
    byte_count: u64,
    history_head: Digest,
    transport_root: Digest,
};

pub const Ticket = struct {
    owner_namespace: u64,
    owner_generation: u64,
    job_id: u64,
    request_id: u64,
};

pub const RetentionToken = struct {
    ticket: Ticket,
    prefix_identity: Digest,
    host_pin_id: u64,
};

pub const ViewToken = struct {
    owner_namespace: u64,
    owner_generation: u64,
    descriptor_slot: DescriptorSlot,
    descriptor_generation: u64,
    lease_slot: u8,
    lease_generation: u64,
};

pub const RoutedEnvelope = struct {
    planet: PlanetId,
    body: []const u8,
};

pub const ByteChunk = struct {
    len: u16,
    bytes: [256]u8,
};

pub const PageResult = struct {
    written: u8,
    complete: bool,
};

pub const CancelResult = enum {
    requested,
    already_published,
    stale_job,
    faulted,
};
```

`types.zig` defines `Failure` as an explicit error set covering admission, structural input, stale identity, quota-independent invariant failure, backpressure, provider failure, cleanup fault and shutdown. Gameplay rejection reasons are a separate explicit `enum(u16)` matching codec codes 1–39. Never convert a transport failure into a gameplay rejection.

Types whose full payload fields are fixed by the codec—policy, command union, canonical record union and receipt union—must reproduce the companion’s tables exactly. Do not alias the v0 event union: tags, ordinal width, wrappers and global event ordering differ.

Internal cursors live in witnessed control/scratch ranges. Public callers hold small value tokens, not copied cursors containing hidden scratch.

### Test command conventions

Run from the repository root. Set once per execution shell:

```sh
FS_ZIG=/Users/donaldfilimon/.zvm/bin/zig
"$FS_ZIG" version
```

Require output exactly `0.18.0-dev.146+35accc06e`. A mismatch is an environment blocker, not a failing behavior test.

Every focused task supplies an exact test-name prefix:

```sh
"$FS_ZIG" test zig/src/fullscale_authority_tests.zig -O Debug --test-filter 'FS01 '
"$FS_ZIG" test zig/src/fullscale_authority_tests.zig -O ReleaseSafe --test-filter 'FS01 '
```

Tests and CLI code below are future execution instructions. None were run to produce this plan.

For each task:

1. Add the concrete behavior test first.
2. Compile the smallest legal placeholder implementation if necessary.
3. Observe a behavior/assertion failure. An unresolved import alone does not satisfy RED.
4. Implement only the owned task.
5. Run focused Debug and ReleaseSafe tests.
6. Run the accumulated isolated suite once.
7. Obtain a fresh specification review, then a fresh code-quality review.
8. Fix findings and rerun affected tests before the next task.
9. Do not commit.

## Task 0: Freeze the joint session contract and execution ownership

**Owner:** Controller plus the separate session owner; no product writer starts yet.

**Files:** Read approved full-scale documents and the current session specification. No edits by the full-scale implementation writer.

**Current observation:** The live session draft read during planning had SHA256:

`d448fa0479445bdb748dc03de23c17374692845fb08c92b2fe992b0337798992`.

It proposes `32 MiB/tick`, `5 TiB` portable containers, `32 TiB` namespace accounting and `128 MiB` store/transfer transient memory. This is a moving Proposed document.

The authority specification’s older M1 snapshot, SHA256 `0014afd69b1f33bc83842d46148361ad5e8a421a240250b7dc10dc7131942fda`, had `1 MiB/tick`. That older limit conflicts with the initial `4,057,265`-byte bound. It is historical dependency evidence, not a description of the current draft.

**Material unresolved interface:** The current session draft adds `SessionAppendReservation`, `PreparedTranscriptCursor`, `GraphPrepared`, and one owner-thread publication spanning the authority descriptor and resident session graph. Those declarations and this publication coupling are absent from the approved authority revision. Matching byte ceilings alone does not reconcile them.

- [ ] Rehash the authority, codec, seeded and current session documents.
- [ ] Have the separate owners produce joint acceptance for:
  - exact initial grammar and `4,057,265` framed bytes;
  - complete future transactions through `33,554,432` bytes;
  - ticket, Prefix, retention-token and shutdown ownership;
  - stream reservation and graph-preparation ordering;
  - one failure-free publication point without callbacks;
  - candidate transcript access restricted to the session bridge;
  - source/target plus store memory, with no complete extra tick copy;
  - graph/portable/namespace arithmetic and 100,000-tick retention;
  - cancellation before/after publication and delayed disposal.
- [ ] Pin the accepted revisions and exact bridge declarations in an addendum to this plan under controller ownership.
- [ ] Name the bridge writer and consuming session owner. The authority writer must not edit session code.
- [ ] Confirm seeded interfaces used by Task 3 are independently qualified, or retain the work as blocked.
- [ ] Confirm the writer owns only this plan’s new paths and no other agent is editing them.
- [ ] Obtain independent review of the merged contract.

**Exit condition:** Explicit written compatibility acceptance for exact revisions. Until then, this is a design prerequisite, not an instruction to silently adopt the latest session text.

## Task 1: Prove all concrete layouts before product approval

**Files:**

- Create `zig/src/fullscale_authority/layout.zig`.
- Create `zig/src/fullscale_authority/types.zig`.
- Create `zig/src/fullscale_authority/layout_tests.zig`.
- Create `zig/src/fullscale_authority_tests.zig`.

**Boundary:** This is the isolated compiler witness required by the specification. The controller must explicitly admit witness-only work after Task 0. Its successful review is required before approving Tasks 2–15. Do not treat this plan’s prose arithmetic as the compiler witness.

**Interfaces:**

- Produces `layout.PageSlot`, `layout.IndexRow`, `layout.Controls`, `layout.Scratch`.
- Produces `layout.requested_bytes: u64 = 188_809_216`.
- Produces `layout.block_bytes: [7]usize`.
- Produces `layout.assertFits() void`, using compile-time checks.
- All later modules consume these definitions; later cursor growth must re-run this witness.

The block sizes are:

```zig
pub const block_bytes = [_]usize{
    71_303_168,
    6_291_456,
    67_108_864,
    33_554_432,
    8_388_608,
    2_097_152,
    65_536,
};

pub const PageSlot = extern struct {
    payload: [256]u8,
    references: u32,
    next_free: u32,
    flags: u32,
    reserved: u32,
};

pub const IndexRow = extern struct {
    realm: u32,
    kind: u16,
    reserved_key: u16,
    local: u64,
    page: u32,
    reserved_page: u32,
};
```

- [ ] Write `FS01 exact seven blocks and two transport slots`:

```zig
try std.testing.expectEqual(@as(usize, 272), @sizeOf(layout.PageSlot));
try std.testing.expectEqual(@as(usize, 24), @sizeOf(layout.IndexRow));
try std.testing.expectEqual(@as(usize, 8), @offsetOf(layout.IndexRow, "local"));
try std.testing.expectEqual(@as(usize, 16), @offsetOf(layout.IndexRow, "page"));
try std.testing.expectEqual(@as(u64, 188_809_216), layout.requested_bytes);
try std.testing.expect(@sizeOf(layout.Controls) <= 2_097_152);
try std.testing.expect(@sizeOf(layout.Scratch) <= 8_388_608);
```

- [ ] Instantiate the complete cursor inventory: initialization, index preparation/search/shift, child-retain masks, parent/child COW, cleanup, command parsing/sorting/preflight, each local phase, state validation, canonical encoder/decoder, history chain hashing, transport copy/carry/fold, provider verification, query, replay, retirement and accepted session bridge.
- [ ] Put allocation owner bits, block addresses, descriptors, per-world quotas/counters, command lengths/offsets, 64 leases, 64 resident segment identities, provider dispositions, transport contexts and metrics inside the witness.
- [ ] Assert two context ranges of exactly `4,096` bytes; tail SHA state `<=256` bytes; carry/fold/copy workspace `<=1,024` bytes; copy cursor outside the copied destination.
- [ ] Include peak simultaneously live temporary state. Mutually exclusive union storage is legal only where lifecycle states prove exclusion.
- [ ] Assert every offset’s alignment and end bound using checked arithmetic. A byte array called “controls” without its complete instantiated types is insufficient.
- [ ] Compile on native and Wasm:

```sh
"$FS_ZIG" test zig/src/fullscale_authority_tests.zig -O Debug --test-filter 'FS01 '
"$FS_ZIG" test zig/src/fullscale_authority_tests.zig -O ReleaseSafe --test-filter 'FS01 '
"$FS_ZIG" test zig/src/fullscale_authority_tests.zig -target wasm32-freestanding -O ReleaseSafe -fno-emit-bin --test-filter 'FS01 '
```

- [ ] Exercise a deliberate local over-limit type in the witness test helper and confirm its fit predicate rejects it. Restore the valid representation before review.
- [ ] Record actual sizes, offsets, compiler and target. Compile-only Wasm is layout evidence only.
- [ ] Obtain fresh specification and quality review; controller then decides implementation approval.

## Task 2: Allocate seven blocks and establish exact pool ownership

**Files:** Create `storage.zig`, `pool.zig`, `storage_tests.zig`, `pool_tests.zig`, `test_support.zig`; extend the isolated test root.

**Interfaces:**

```zig
// storage.zig
pub fn create(allocator: std.mem.Allocator, namespace: u64, generation: u64) Failure!*Storage;
pub fn pollInitialize(self: *Storage, budget: *WorkBudget) Failure!Step;
pub fn destroyUnpublished(self: *Storage) void;
pub fn requestedBytes(self: *const Storage) u64;

// pool.zig
pub fn allocate(self: *Storage, budget: *WorkBudget) Failure!?PageRef;
pub fn retain(self: *Storage, page: PageRef, budget: *WorkBudget) Failure!Step;
pub fn release(self: *Storage, page: PageRef, budget: *WorkBudget) Failure!Step;
```

A `null` allocation result means insufficient work allowance, never physical exhaustion. Physical exhaustion under legal root bounds is `InvariantViolation`.

- [ ] Write `FS02 failure at each allocation frees exact owners`: fail allocation ordinals 1–7 and assert no published authority and zero outstanding bytes.
- [ ] Write `FS02 initialization and pool changes consume shared units`: budget zero preserves a normalized control snapshot and payload bytes; budget one changes at most one permitted operation.
- [ ] Write `FS02 no allocator calls after setup`: use a counting allocator that refuses any post-creation allocation while initialization, pool use and destruction complete.
- [ ] Implement control-block-first bootstrap. Store the independent seven-block ownership ledger inside controls; record each successful allocation before the next request; free controls last.
- [ ] Initialize payload/free-list/control bytes through `<=256-byte` writes. Never sweep all slots in one poll.
- [ ] Implement one-based handles, checked reference increments and free-list operations. Reference count above two, double release, invalid handle or inconsistent ownership faults admission.
- [ ] Confirm one authority peaks at `188,809,216` requested bytes and source plus target at `377,618,432`.
- [ ] Run `FS02 ` Debug/ReleaseSafe and review.

Test harness allocations are separate named test owners. They must not be included in, or mistaken for, authority-requested bytes.

## Task 3: Bind the manifest and freeze independent canonical fixtures

**Files:** Create `manifest.zig`, `records.zig`, `codec.zig`, `hash.zig`, `codec_tests.zig`, `manifest_tests.zig`, fixture files and `zig/tools/fullscale-authority-reference.py`.

**Interfaces:**

```zig
pub fn beginGenesis(storage: *Storage, genesis: []const u8) Failure!void;
pub fn pollGenesis(storage: *Storage, budget: *WorkBudget) Failure!Step;

pub fn beginEncodeRecord(storage: *Storage, record: CanonicalRecord) Failure!void;
pub fn pollEncodeRecord(storage: *Storage, out: *ByteChunk, budget: *WorkBudget) Failure!Step;

pub fn beginDecodeCommand(storage: *Storage, planet: PlanetId, body: []const u8) Failure!void;
pub fn pollDecodeCommand(storage: *Storage, budget: *WorkBudget) Failure!Step;

pub fn pollHash(storage: *Storage, budget: *WorkBudget) Failure!Step;
```

`CanonicalRecord` and decoded command storage are concrete witnessed unions, matching the companion. Slices supplied to `beginGenesis` are explicitly borrowed through initialization, or copied incrementally into the owned manifest block before release; callers cannot destroy a borrowed source early. No seeded builder remains retained after readiness.

- [ ] Independently implement the printed little-endian grammar and masked SplitMix64 formulas in Python, importing no NYON implementation.
- [ ] Copy the six reviewed literal byte strings into fixture files and assert these independent lengths/hashes:

| Fixture | Bytes | Raw SHA256 |
|---|---:|---|
| Genesis | 415 | `f69fd1fbdc56143b27f32fca03624febc2a57d70e22759c60b0e755ddbe7dc1c` |
| Genesis receipt | 136 | `74f3bff61d13c0b1dc82b857ed2c5c5db2bd059fff48882a0a629bd1a15fde8a` |
| Empty first tick | 167 | `7acf5b31d277556770ea892182fedc9a37878dcf830cceb6b4de66d44ffd7cf1` |
| Policy change | 173 | `8170557ad03b537c889cbea78fb35cbfc742c1b9cde461b2501665de48610314` |
| Same local ID | 583 | `ce79a77ea3ff06dc6744a1d95d20c92d91efd8624c4eee18ecba8124c59f2f92` |
| Quota fragment | 296 | `1f5fc797204fcadd25d3038ab405cde15bf44a73ccae3cad609345f8ea28cbb2` |

- [ ] Write `FS03 exact initial grammar and independent literals`: compare every encoded byte, not only hashes.
- [ ] Write `FS03 rejects changed quota vector and strategic kind`: mutate each of 34 quota values, identity, count and unknown tag separately; require structural rejection.
- [ ] Write `FS03 malformed bool optional lengths and trailing bytes`: exercise each scalar/container boundary and checked overflow.
- [ ] Implement `NYFG`, frozen `NYSG`, all 34 exact quotas, initial absent policy, initial world rows, command fields, state rows and receipt tags.
- [ ] Encode physical parent/path fields separately from canonical records. Zero unused physical payload bytes. Never canonicalize raw structs or private handles.
- [ ] Meter SHA compression and padding at `<=64 bytes/unit`; record state updates with their work consumption.
- [ ] Independently assert:
  - `2 + 2*1024 + 4096 + 2*16384 + 2*4096 = 47_106`;
  - framed upper bound `4_057_265`;
  - receipt/body and frame ceilings remain distinct.
- [ ] Run:

```sh
python3 zig/tools/fullscale-authority-reference.py --verify
"$FS_ZIG" test zig/src/fullscale_authority_tests.zig -O Debug --test-filter 'FS03 '
"$FS_ZIG" test zig/src/fullscale_authority_tests.zig -O ReleaseSafe --test-filter 'FS03 '
```

The 296-byte fixture remains labeled a fragment. It does not establish a complete capacity replay.

## Task 4: Implement resumable index operations and root preparation

**Files:** Create `index.zig`, `index_tests.zig`; extend witnessed cursor definitions as already reserved.

**Interfaces:**

```zig
pub fn beginFind(storage: *Storage, root: DescriptorSlot, key: Key) Failure!void;
pub fn pollFind(storage: *Storage, budget: *WorkBudget) Failure!Step;
pub fn foundRow(storage: *const Storage) Failure!?u32;

pub fn beginInsert(storage: *Storage, key: Key, parent: PageRef) Failure!void;
pub fn beginDelete(storage: *Storage, key: Key) Failure!void;
pub fn pollMutation(storage: *Storage, budget: *WorkBudget) Failure!Step;

pub fn beginPrepareRoot(storage: *Storage) Failure!void;
pub fn pollPrepareRoot(storage: *Storage, budget: *WorkBudget) Failure!Step;
```

These are internal lane methods. No caller may run two overlapping index mutations.

- [ ] Write `FS04 numeric ordering differs from little endian`: include local IDs 1, 255, 256 and 65,536 across multiple realms/kinds.
- [ ] Write `FS04 insertion first middle last interrupted after each move`.
- [ ] Write `FS04 deletion child release and hole shift interrupted`.
- [ ] Write `FS04 preparation retains only selected child mask`: cover parents with 0, 1, 2 and 3 children and stop after each retain.
- [ ] Implement `extent`, optional `hole`, optional `pending_row`, child ownership masks and logical count publication exactly as F1 specifies.
- [ ] Make each row move a 24-byte ownership transfer plus hole update within one charged action:

```text
row[hole] := row[source]
hole := source
```

The old bytes at the new hole are non-owning. Do not retain or release for this move.

- [ ] Prepare roots by retaining the parent, then individual children, then transferring the pending row into the prepared prefix. The untouched source suffix remains unowned by the candidate.
- [ ] Test full index length separately from page capacity. At legal page capacity, a missing page reserve must be reported before mutation; index maximum does not authorize a 131,072nd root-owned page.
- [ ] Run `FS04 ` Debug/ReleaseSafe and review.

## Task 5: Implement COW, interruption-safe cleanup and fault destruction

**Files:** Create `cow.zig`, `cow_tests.zig`; update `storage.zig` only for the planned lifecycle methods.

**Interfaces:**

```zig
pub fn beginParentWrite(storage: *Storage, key: Key) Failure!void;
pub fn beginChildWrite(storage: *Storage, key: Key, child: u8) Failure!void;
pub fn pollWrite(storage: *Storage, budget: *WorkBudget) Failure!Step;

pub fn beginDiscard(storage: *Storage) Failure!void;
pub fn pollDiscard(storage: *Storage, budget: *WorkBudget) Failure!Step;
pub fn pollRetire(storage: *Storage, budget: *WorkBudget) Failure!Step;
```

The writer’s bounded update operation consumes owned record data from scratch; it does not return an escaping mutable pool slice.

- [ ] Write `FS05 parent COW transfers child ownership without retain`.
- [ ] Write `FS05 child COW waits for drained parent transition`.
- [ ] Write `FS05 every cancellation cut point returns to original ownership`.
- [ ] Write `FS05 abort while aborting preserves release cursor`.
- [ ] Write `FS05 two full roots plus two detached slots never allocate overflow`.
- [ ] Implement `new_ref`/`old_ref` and their ownership bits. Drain both before the next allocation.
- [ ] Enforce `131,071` candidate pages including pending-row parents/children.
- [ ] Clean up extent minus hole, pending-row mask and transition handles. Update each release mask/cursor in the same action as the decrement.
- [ ] Inject invalid reference metadata during cleanup. Require `CleanupFault`, readable committed state and permanently closed mutation admission.
- [ ] Demonstrate fault destruction uses the allocation ledger, waits for local leases and transfers external dispositions without traversing corrupt pages.
- [ ] Run `FS05 ` Debug/ReleaseSafe and review.

Full-root fixtures use storage-only records and are labeled ownership-envelope tests. They cannot be serialized as invented strategic gameplay.

## Task 6: Implement descriptor-owned transport contexts

**Files:** Create `transport.zig`, `transport_tests.zig`.

**Interfaces:**

```zig
pub fn beginCopy(storage: *Storage, source: DescriptorSlot, target: DescriptorSlot) Failure!void;
pub fn pollCopy(storage: *Storage, budget: *WorkBudget) Failure!Step;

pub fn beginAppend(storage: *Storage, chunk: ByteChunk) Failure!void;
pub fn pollAppend(storage: *Storage, budget: *WorkBudget) Failure!Step;
pub fn beginFold(storage: *Storage) Failure!void;
pub fn pollFold(storage: *Storage, budget: *WorkBudget) Failure!Step;
pub fn preparedRoot(storage: *const Storage) Failure!Digest;

pub fn pollDiscardContext(storage: *Storage, budget: *WorkBudget) Failure!Step;
```

Append consumes the candidate’s bounded buffered chunk; it never borrows provider memory. Transport work and canonical history hashing share the caller budget.

- [ ] Write `FS06 zero budget preserves both contexts and copy flags`.
- [ ] Write `FS06 tail finalization leaves appendable SHA state unchanged`.
- [ ] Write `FS06 seven sealed leaves tail cancel offline retry exact`.
- [ ] Independently compute empty leaves, leaf domains, 32-level paths and roots in the Python reference.
- [ ] Implement 16 or fewer copy chunks, then a separately charged generation-binding transition. No append may observe a partial copy.
- [ ] Keep all carries, peak changes, tail resets and folds candidate-only.
- [ ] Use a cached height-32 root for exactly `2^32` leaves; reject an additional leaf or partial tail without indexing peak 32.
- [ ] Build the mandatory witness:
  1. Commit seven sealed segments plus a partial tail.
  2. Evict prior sealed data under a valid retained prefix.
  3. Candidate seals the tail and another full segment.
  4. Cancel after each copy chunk, seal, carry overwrite, fold and ready transition.
  5. Make the provider unavailable throughout cleanup.
  6. Assert committed context bytes remain identical.
  7. Retry and compare transport root and canonical stream with uninterrupted execution.
- [ ] Repeat interruption after publication and require only obsolete context retirement.
- [ ] Run `FS06 ` Debug/ReleaseSafe and review.

## Task 7: Implement framed history, cache admission and eviction ownership

**Files:** Create `history.zig`, `history_tests.zig`.

**Interfaces:**

```zig
pub fn reserveTick(storage: *Storage, maximum_bytes: u32) Failure!void;
pub fn beginReceipt(storage: *Storage, receipt: Receipt) Failure!void;
pub fn pollReceipt(storage: *Storage, budget: *WorkBudget) Failure!Step;
pub fn pollCacheCopy(storage: *Storage, budget: *WorkBudget) Failure!Step;
pub fn acknowledgeSegment(storage: *Storage, ack: SegmentAck) Failure!void;
pub fn pollEviction(storage: *Storage, budget: *WorkBudget) Failure!Step;
```

`SegmentAck` contains exact genesis identity, ordinal, length and digest plus the owned exact-prefix retention-token binding. A bare digest acknowledgment cannot authorize eviction.

- [ ] Write `FS07 exact initial reserve and one slot short`.
- [ ] Write `FS07 split record preserves framing and chain`.
- [ ] Write `FS07 candidate seals committed tail cannot evict before publication`.
- [ ] Write `FS07 no pin no eviction despite matching segment digest`.
- [ ] Reserve the initial fixed bound before mutation and determine required free slots with checked arithmetic, accounting for the resident partial tail.
- [ ] Keep separate committed/candidate reachability and visible lengths where they share one physical tail.
- [ ] Copy candidate history into cache incrementally; never expose it through public views before publication.
- [ ] Reject begin with `HistoryBackpressure` before changing clock, policy, sequences, IDs or state when exact admission fails.
- [ ] On cancellation discard only invisible ownership; never edit old visible bytes or lengths.
- [ ] Exercise the full 32 MiB transport envelope with synthetic opaque transport bytes only. Label this transport capacity, not accepted gameplay under the initial codec.
- [ ] Run `FS07 ` Debug/ReleaseSafe and review.

## Task 8: Implement pure provider requests, pinning and shutdown dispositions

**Files:** Create `provider.zig`, `provider_tests.zig`.

**Interfaces:**

```zig
pub fn requestPin(storage: *Storage, prefix: Prefix) Failure!Ticket;
pub fn completePin(storage: *Storage, ticket: Ticket, token: RetentionToken) Failure!void;

pub fn requestSegment(
    storage: *Storage,
    ticket: Ticket,
    pin: RetentionToken,
    prefix: Prefix,
    ordinal: u64,
) Failure!void;

pub fn ingest(
    storage: *Storage,
    ticket: Ticket,
    part: SourcePart,
    chunk: ByteChunk,
    budget: *WorkBudget,
) Failure!Step;

pub fn cancelSource(storage: *Storage, ticket: Ticket) Failure!void;
pub fn takeShutdownDispositions(storage: *Storage, out: *[4]Disposition) Failure!u8;
```

`SourcePart = enum { header, proof, bytes }`. `Disposition` is a tagged value for descriptor-pin release, replay-pin release or active-request cancellation; it carries copied tickets/tokens, never pointers.

- [ ] Write `FS08 stale ticket rejected before copy or metering mutation`.
- [ ] Write `FS08 self consistent forged segment fails local root proof`.
- [ ] Write `FS08 partial final segment expected length`.
- [ ] Write `FS08 cancel pending source broker unavailable closes replacement`.
- [ ] Write `FS08 shutdown transfers four dispositions exactly once`.
- [ ] Keep one 1 MiB source window, exactly 32 sibling hashes, two bounded request/header values and one active lane in the existing blocks.
- [ ] Validate ordinal/length from locally owned Prefix before ingest.
- [ ] Hash bytes and path incrementally; expose no source bytes to replay until full authentication succeeds.
- [ ] Reject wrong owner/generation/job/request, prefix, ordinal, length, sibling count, path direction and digest.
- [ ] Model the fake broker as a separate named test owner. Deliver completion after cancellation and after authority destruction; verify it disposes buffers and cannot dereference closed authority memory.
- [ ] Require broker acceptance before disposition transfer. `Busy` preserves authority ownership; unavailable release must not be reported as completed.
- [ ] Test pin closure across branch deletion, replacement and future source windows using the protocol fake. Real durable closure requires the separate session gate.
- [ ] Run `FS08 ` Debug/ReleaseSafe and review.

## Task 9: Implement immutable leases and bounded queries

**Files:** Create `view.zig`, `view_tests.zig`, initial new `root.zig` facade.

**Interfaces:**

```zig
pub fn acquireView(storage: *Storage) Failure!ViewToken;
pub fn releaseView(storage: *Storage, token: ViewToken) Failure!void;
pub fn summary(storage: *const Storage, token: ViewToken) Failure!Summary;

pub fn beginQuery(storage: *Storage, token: ViewToken, query: Query) Failure!u64;
pub fn pollQuery(
    storage: *Storage,
    query_id: u64,
    out: []RecordValue,
    budget: *WorkBudget,
) Failure!PageResult;
```

`Query` is a tagged value for key lookup, numeric entity range, qualified cell or filtered receipt range. `RecordValue` is copied bounded data; query pages contain at most 64 values. Long filtered scans remain cursor-driven.

- [ ] Write `FS09 old lease sees complete old root after switch`.
- [ ] Write `FS09 old leases block retirement new leases do not`.
- [ ] Write `FS09 65th lease and exhausted generation never reuse token`.
- [ ] Write `FS09 invalid realm cell and stale owner fail without candidate read`.
- [ ] Bind every token to owner namespace/generation and descriptor/lease generations.
- [ ] Derive terrain from the manifest’s planet seed and validated cell. Missing claim means unoccupied.
- [ ] Use metered lookup/range work; do not hide an entire index scan behind a value query.
- [ ] Keep leases as root pins, not extra per-page references.
- [ ] Run `FS09 ` Debug/ReleaseSafe and review.

## Task 10: Implement shared command routing and atomic preflight

**Files:** Create `local_rules.zig`, `commands.zig`, `commands_tests.zig`.

**Interfaces:**

```zig
pub fn snapshot(
    storage: *Storage,
    policy: Policy,
    envelopes: []const RoutedEnvelope,
) Failure!void;

pub fn pollStructural(storage: *Storage, budget: *WorkBudget) Failure!Step;
pub fn pollCommands(storage: *Storage, budget: *WorkBudget) Failure!Step;
```

`local_rules.zig` adapts full-scale typed IDs to existing pure `AlgorithmContext`, topology and terrain functions. It does not allocate or call island state/job APIs.

- [ ] Write `FS10 snapshot survives caller mutation after begin`.
- [ ] Write `FS10 route sequence and malformed body fail whole tick`.
- [ ] Write `FS10 valid forbidden and unknown targets emit rejection`.
- [ ] Write `FS10 world1 sequence2 wins last quota before world2 sequence1`.
- [ ] Write `FS10 rejection consumes no resources IDs claims or reservations`.
- [ ] Copy only the bounded input snapshot synchronously. Record body lengths and routing metadata in witnessed ranges.
- [ ] Incrementally sort global sequence order and validate uniqueness, watermark, routing and local counts before gameplay.
- [ ] Emit routed inputs globally by sequence; execute commands by world then sequence.
- [ ] Preserve role/existence/ownership/numeric/topology precedence.
- [ ] In the capacity portion, test in this exact order:
  1. per-world entity;
  2. global entity;
  3. per-world claims;
  4. global claims;
  5. original remaining local capacity/resources.
- [ ] Implement all seven initial commands against new index/COW operations. Preflight every effect before any command mutation.
- [ ] Test shipment paths 0, 1, 2, 65 and 66; foreign cells, repeated cells, seam edges and wrong endpoints; distinguish structural overlength from gameplay-invalid path.
- [ ] Test every quota at exact capacity and one over, both independently and with competing worlds. Verify codes 38/39 and `quota_kind`.
- [ ] Run `FS10 ` Debug/ReleaseSafe and review.

## Task 11: Implement all-world economy phases and canonical validation

**Files:** Create `phases.zig`, `validation.zig`, `phases_tests.zig`, `validation_tests.zig`.

**Interfaces:**

```zig
pub fn beginPhase(storage: *Storage, phase: Phase) Failure!void;
pub fn pollPhase(storage: *Storage, budget: *WorkBudget) Failure!Step;
pub fn beginValidation(storage: *Storage) Failure!void;
pub fn pollValidation(storage: *Storage, budget: *WorkBudget) Failure!Step;
```

`Phase` includes prepare, commands, arrivals, construction, production, population, validation/history, prepared, publication and retirement, with separate wire phase tags from the companion.

- [ ] Write `FS11 512 worlds enter every phase including empty unselected`.
- [ ] Write `FS11 arrival construction production supply order`.
- [ ] Write `FS11 production settlement then building numeric order`.
- [ ] Write `FS11 duplicate key dangling claim path and count mismatch rejected`.
- [ ] Implement one charged empty-world entry and a barrier across all worlds before the next phase.
- [ ] Reproduce existing economy semantics using sparse records and new storage. Preserve arrivals and reservations, completion/housing effects, labor order, output-capacity idling, supply and demography.
- [ ] Incrementally validate rows, claims, footprints, paths, world/global counts, resource reservations and next-ID bounds.
- [ ] Feed canonical state bytes to the incremental state hash; do not include transport, cache or scheduler state.
- [ ] Build reachable legal maximum local fixtures using public commands and declared per-world capacities. Keep synthetic invalid-record tests separate.
- [ ] Check emitted family counts and framed bytes against the conservative bound; do not require mutually exclusive maxima to occur together.
- [ ] Run `FS11 ` Debug/ReleaseSafe and review.

## Task 12: Integrate job lifecycle, prepared publication and retirement

**Files:** Create `job.zig`, `job_tests.zig`; extend the new facade. Add only the exact session-bridge files/declarations accepted in Task 0.

**Interfaces:**

```zig
pub fn beginTick(
    storage: *Storage,
    expected_revision: u64,
    policy: Policy,
    envelopes: []const RoutedEnvelope,
) Failure!u64;

pub fn pollTick(storage: *Storage, job_id: u64, budget: *WorkBudget) Failure!Progress;
pub fn cancel(storage: *Storage, job_id: u64) CancelResult;
```

`Progress` is `pending`, `published`, `canceled`, `failed`, `invalid_request` or `idle`, with copied metadata. Terminal-delivery transitions consume work and cannot occur under zero allowance.

The session bridge is a Task 0 dependency, not an implementation choice in this task. A standalone in-memory test host uses the accepted pure bridge with a bounded fake; it must not bypass required publication preparation.

- [ ] Write `FS12 one publication step contains no fallible work`.
- [ ] Write `FS12 canceled first tick preserves absent policy and genesis`.
- [ ] Write `FS12 cancellation after switch reports already published`.
- [ ] Write `FS12 session graph preparation mismatch aborts before switch`.
- [ ] Write `FS12 same shared allowance reaches cleanup without replenishment`.
- [ ] Connect preparation, routing, commands, all-world phases, state validation, history/cache preparation, transport readiness and accepted session graph preparation.
- [ ] Require the prepared barrier to verify counts, references, quotas, byte counts, global digests, generations and no unfinished hash/carry/copy.
- [ ] Switch the committed descriptor and accepted resident-session descriptor ownership in the reviewed owner-thread operation without callback, allocation, hash or yield.
- [ ] Keep the lane Busy until old ownership and contexts drain. Old leases may delay retirement and next begin, never the completed publication.
- [ ] Exercise checked exhaustion separately:
  - entity IDs reject without consumption;
  - job/revision/descriptor exhaustion rejects admission;
  - sequence `maxInt(u64)` publishes once; later empty ticks remain legal;
  - horizon rejects before mutation;
  - lease generation exhaustion never wraps.
- [ ] Run `FS12 ` Debug/ReleaseSafe and review.

## Task 13: Implement authenticated replay and exact target transfer

**Files:** Create `replay.zig`, `replay_tests.zig`.

**Interfaces:**

```zig
pub fn beginReplay(source: *Storage, prefix: Prefix, allocator: std.mem.Allocator) Failure!u64;
pub fn pollReplay(source: *Storage, replay_id: u64, budget: *WorkBudget) Failure!ReplayProgress;
pub fn cancelReplay(source: *Storage, replay_id: u64) Failure!void;
pub fn takeReplay(source: *Storage, replay_id: u64) Failure!*Storage;
```

`ReplayProgress` is `pending`, `complete`, `canceled` or `failed`. One target allocation set is allowed. Transfer moves target owner and source-history pin together; no cloned owner handle remains live.

- [ ] Write `FS13 literal replay compares every event not supplied digest only`.
- [ ] Write `FS13 target inside source segment authenticates full exposes prefix`.
- [ ] Write `FS13 different page allocation order produces identical bytes`.
- [ ] Write `FS13 failed target leaves source leases and state intact`.
- [ ] Write `FS13 successful take transfers pin and forbids third authority`.
- [ ] Pin the exact complete source Prefix before consuming evicted source bytes.
- [ ] Reconstruct genesis and commands; re-execute all phases and compare every framed receipt, commit state digest and logical record.
- [ ] Discard verified target cache bytes only against still-owned source backing; never archive replay output a second time.
- [ ] Track verified byte prefix and target published prefix separately. A larger source pin does not authorize exposing later target bytes.
- [ ] Require a new exact-prefix pin before a taken replay target forks into fresh simulation.
- [ ] Replay the complete 64/512/16 last-quota setup history. Use an independent reference implementation or independently reviewed fixture derivation; the existing four-record fragment alone is insufficient.
- [ ] Run budgets `0/1/7/256/4096`, deterministic randomized schedules, provider chunk sizes `1/7/64/255/256`, and varied cache eviction schedules. Canonical results must match.
- [ ] Run `FS13 ` Debug/ReleaseSafe and review.

## Task 14: Complete capacity, lifecycle and cross-target qualification

**Files:** Create `capacity_tests.zig`, `schedule_tests.zig`, `robustness_tests.zig`, `zig/src/fullscale_authority_wasm.zig`; extend isolated root and independent reference fixtures.

**Interfaces:** Consumes the completed new facade; introduces no new gameplay/API semantics.

- [ ] Add named tests:
  - `FS14 all fixed quotas exact and one over`;
  - `FS14 64 systems 512 worlds 16 civilizations`;
  - `FS14 full roots leases dirty candidate cancellation`;
  - `FS14 complete 47106 record bound accounting`;
  - `FS14 initialization allocation and cleanup fault cut points`;
  - `FS14 namespace generation and stale token matrix`;
  - `FS14 native wasm canonical equality`.
- [ ] Verify initial-family capacities simultaneously where reachable, including concentrated worlds and legal global sharing.
- [ ] Retain strategic storage-envelope witnesses separately. Do not create canonical strategic records under the initial identity.
- [ ] Sweep interruption points for every action class, including initial/free-list setup, path child links, index shifts, COW, context copy, SHA compression, provider verification, publication and cleanup.
- [ ] Define the exported Wasm deterministic fixture entry point to accept a fixture ID and schedule and return copied digest/result bytes in a bounded caller buffer. It must exercise the same authority functions as native.
- [ ] Compile and execute native fixtures; compile Wasm now and execute it with the integration owner’s approved existing Wasm runner. If no runner is available, report runtime equality as unverified.
- [ ] Run:

```sh
"$FS_ZIG" test zig/src/fullscale_authority_tests.zig -O Debug
"$FS_ZIG" test zig/src/fullscale_authority_tests.zig -O ReleaseSafe
"$FS_ZIG" build-obj zig/src/fullscale_authority_wasm.zig -target wasm32-freestanding -O ReleaseSafe -fno-emit-bin
python3 zig/tools/fullscale-authority-reference.py --verify
```

- [ ] Re-run the complete layout witness after final cursor composition.
- [ ] Obtain independent specification and code-quality reviews.

Do not run simultaneous full-capacity test processes merely to shorten qualification; source plus target already reserves 360.125 MiB before harness and host memory.

## Task 15: Add standalone CLI qualification and integration handoff

**Files:** Create `zig/src/fullscale_authority_cli.zig`, `zig/src/fullscale_authority_qualification.zig`, `zig/src/fullscale_authority/cli_tests.zig`, `zig/tools/check-fullscale-authority.py`.

**Interfaces:**

```zig
pub fn run(
    allocator: std.mem.Allocator,
    args: []const []const u8,
    out: *std.Io.Writer,
) !void;
```

Use existing native CLI conventions. The new standalone entry point imports the new CLI directly; it does not modify the product main.

**CLI contract:**

```text
--case empty|same-id|quota|cancel|replay|capacity|timing
--budget 1..4096
--seed <u64>
--ticks <u64>
```

Reject duplicate, unknown and missing flags. Explicitly reject budget zero for a run-to-completion CLI; zero-budget behavior remains covered by API tests.

- [ ] Write `FS15 invalid flags allocate no authority`.
- [ ] Write `FS15 output failure drains owners and external dispositions`.
- [ ] Write `FS15 machine output separates requested bytes RSS and timing`.
- [ ] Emit rules/storage identity, manifest digest, fixture, actual capacities, committed tick/revision, state/history/transport digests, requested bytes, page peaks, provider state and qualification result.
- [ ] Build/run standalone:

```sh
"$FS_ZIG" build-exe zig/src/fullscale_authority_qualification.zig -O ReleaseSafe -femit-bin=/tmp/nyon-fullscale-authority-qualification
python3 zig/tools/check-fullscale-authority.py /tmp/nyon-fullscale-authority-qualification
```

Use a task-owned temporary output path if this fixed disposable binary already belongs to another live session.

- [ ] Measure at least 10,000 budget-256 polls on named hardware. Report p50/p95/max, warmup policy and sample count.
- [ ] Independently time complete full steps from admitted begin through publication, recording retirement separately and also reporting begin-to-lane-idle cost. Include request snapshot, index/preparation, hashing, phase and cleanup costs.
- [ ] Include maximum-world population/insertion workloads. Do not relabel short polls as a passing full-step p95.
- [ ] If full-step p95 exceeds 5 ms, stop qualification and request a reviewed index/algorithm revision preserving canonical semantics and ownership. Do not lower capacities or waive the target.
- [ ] Measure allocator-requested peak and host RSS separately; identify test-provider and harness owners.
- [ ] Hand off the following exact integration work to the controller/integration owner:
  - import/export the new facade in shared root;
  - wire new product CLI entry points;
  - add isolated/native/Wasm build targets;
  - extend the Rust-free gate;
  - connect the independently qualified durable session bridge;
  - run existing v0 fixtures unchanged;
  - run the current full Rust-free repository gate.
- [ ] The integration owner runs the existing gate with the pinned compiler first in PATH:

```sh
PATH=/Users/donaldfilimon/.zvm/bin:$PATH ./zig/tools/check.sh
```

- [ ] The session owner supplies crash/CAS/delete/GC/export/import/replay evidence and real 36,000 mixed/100,000 endurance retention fixtures under the reconciled contract.
- [ ] Record native, Wasm compile, Wasm runtime, browser, durable-session, timing, RSS and manual acceptance independently.

Full strategic-capacity performance and later city/tactical gameplay remain blocked on their reviewed versioned contracts. This initial authority must not claim those acceptance items from placeholder rows or an empty campaign.

## Review and handoff protocol

For every bounded task, the controller dispatches one writer with only that task’s path ownership. The writer is told that other agents are active and must not revert their changes. A fresh specification reviewer then checks the exact diff against the pinned contracts. A different fresh quality reviewer checks ownership, accounting, failure behavior, tests and maintainability.

A reviewer may reject a task independently of its neighbors. No task advances with unresolved Important/Critical findings. Shared interfaces may change only through a concrete update reviewed before their consumers proceed.

The task report must include:

```text
Task:
Owned paths:
Input specification hashes:
Behavioral RED command and observed assertion:
GREEN commands, exit codes and test counts:
Requested-byte/allocation evidence:
Independent fixture evidence:
Specification review:
Quality review:
Remaining proof gaps:
```

No product tests or compiler witnesses have been executed by the planning agent. Do not prefill results.

## Plan self-review

- **F1 ownership:** Tasks 4–5 explicitly cover holes, pending rows, retained-child masks, parent/child COW and abort-during-abort.
- **F2 provider/context ownership:** Tasks 6–8 cover two context slots, seven-sealed-plus-tail cancellation, exact-prefix pins, stale tickets and bounded shutdown transfer.
- **F3 codec:** Task 3 binds all 34 quotas, exact initial grammar, independent literals and conservative record/byte bounds; Task 13 adds complete semantic replay.
- **M1 dependency:** Task 0 distinguishes the historical 1 MiB snapshot from the moving 32 MiB draft and requires merged bridge review.
- **Shared budgets:** Every variable operation consumes the caller’s same `WorkBudget`; no new per-world allowance is introduced.
- **Memory:** Layout, allocation, test-provider and source/target owners remain distinct; no extra builder, transcript, comparison authority or replay archive is admitted.
- **Scope:** All implementation paths are isolated new full-scale files; shared root/main/build/gate and other-owner integration remain explicit handoffs.
- **Qualification:** Full-step latency, full-capacity behavior, Wasm execution and durable endurance retain separate evidence gates.
- **Outstanding prerequisites:** Joint session contract acceptance and actual pinned-compiler layout witness remain unfulfilled. This plan does not claim implementation readiness until they pass.
