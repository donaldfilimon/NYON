# Tactical Distance Feasibility Implementation Plan

<!-- BEGIN 2026-10-19 rewrite decision -->
> **Historical; Zig continuation canceled 2026-10-19.** Preserve this source contract and evidence. Active recovery targets Rust stable1.99.1; see the 2026-10-19 Rust designs/plans. Historical review labels do not approve new Rust behavior. Tests and CI gates remain paused.
<!-- END 2026-10-19 rewrite decision -->


> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Validate and measure an equivalent cube-symmetry distance lookup as an
isolated feasibility probe, without implementing or qualifying tactical gameplay.

**Architecture:** A project-owned Zig probe builds immutable canonical distance
fields and compact goal transforms using current surface topology. It compares
their answers with ordinary BFS and reports actual memory and timing. Results
inform the pending city/combat review; no production consumer is added.

**Tech Stack:** Exact Zig `0.18.0-dev.146+35accc06e`, std allocation/clock and
the existing `surface` module. No dependencies, assets or shared build edits.

**Spec:** [candidate investigation](../../qualification/2026-10-11-tactical-distance-candidate.md)
and the movement/distance contract in [tactical proposal](../specs/2026-10-11-nyon-zig-tactical-ground-combat-design.md).

**Status:** Proposed experiment plan; independent plan review precedes dispatch.
Controller may approve it under verified design delegation. This does not approve
production movement, full city/combat composition or a performance requirement change.

## Global Constraints

- Own only new `zig/tools/probe-tactical-distance.zig` and probe evidence/report;
  do not edit surface, gameplay, shared build/root, profiles or generated assets.
- Compare every integer side4..32; use original local IDs for movement ties.
- All cell distances fit u8 only after observed complete BFS maxima prove it;
  reserve255 as unseen. Checked queue capacity6,144 and exact reachable counts.
- The selected composition reserves a 6 MiB cache partition per authority, with
  separate retained source and replay copies inside existing tactical scratch.
  Build and measure that layout: mappings,29 side offsets,1,627 field offsets,
  48 matrices and alignment. Partition overflow is failed feasibility; no third
  cache or external allocation is authorized. Count construction scratch and
  actual peak requested bytes separately from retained partition bytes.
- Pin compiler before commands. No commits, pushes, installs or new dependencies.

## Review Focus

1. Odd sides and central symmetry duplicates must map to the same correct distances.
2. Edge/corner goals and reflected cube bases must preserve undirected neighbors.
3. Equal-distance neighbors must retain original-world numeric-ID tie-breaking.
4. Incomplete BFS initialization and invalid side/cell inputs must fail explicitly.
5. Timing must exclude setup only when named, retain checksums, and avoid claiming
   a microbenchmark measures city/combat/history/cleanup or browser execution.

### Task 1: Verified distance table and movement microbenchmark

**Files:** Create `zig/tools/probe-tactical-distance.zig`.

**Interfaces:** Internal `Mapping=struct{transform:u8,orbit:u8}` has exactly2 bytes.
`DistanceCache.init(a:Allocator) !DistanceCache` builds all side4..32 tables;
`deinit(*DistanceCache,Allocator) void` releases exact owned blocks;
`distance(*const DistanceCache,side:u16,goal:u16,source:u16) !u8` and
`nextCell(*const DistanceCache,side:u16,goal:u16,source:u16) !?u16` reject invalid
inputs, return no proposal at source==goal, and otherwise choose the original
neighbor minimizing `(distance,local_cell)`. This is route intent only, before
occupancy/credit resolution. `verify` and `measure` remain tool-private.

- [ ] Add tests for side0/3/33 rejection, out-of-range cell IDs, goal equality,
  odd/even orbit counts, known reciprocal seams and equal-distance original-ID
  selection. Record a compiled behavioral RED before implementing lookup.
- [ ] Implement signed axis permutations: transform both face normal and integer
  position using the written six bases; identify target face by normal, then
  recover coordinates through that face's basis. Enumerate48 transforms, select
  first positive-Z image with `x<=y<ceil(side/2)`, and index orbit as
  `y*(y+1)/2+x`. No renderer coordinates or floating point.
- [ ] Build BFS reference fields using actual `surface.Topology.neighbor`.
  Reject unreachable/capacity/distance-overflow results. Store u8 distances and
  two-byte goal mappings; report distance6,061,056 and mapping137,112 separately.
  The prescribed offsets/matrices yield6,205,340 bytes before alignment. Verify
  the compiled end offset fits6,291,456 and instantiate two distinct retained
  partitions; report their exact ownership, alignment and construction/peak
  requested bytes. Existing authority reservations already include these two
  partitions; do not add them again to394,395,648 or claim this probe measures
  full authority allocation. Native/Wasm layout,setup/cancel/free integration
  remain separate qualification gates.
- [ ] Verify every cell under all48 transforms for each side4..32 preserves its
  four undirected neighbors. Assert each transform is a bijection with an inverse
  over all local cells; every goal mapping lands in its indexed positive-Z orbit,
  and every expected canonical field is populated. Verify all source/goal pairs for sides4,5,8,32
  against an uncompressed BFS oracle, including chosen next-cell original-ID ties.
  The oracle must not call compressed lookup to derive expected values.
- [ ] Run Debug and ReleaseSafe tests, then the ReleaseSafe executable verification.
  Commands from `zig/`, after exact compiler preflight:
  `zig test -O Debug --dep surface -Mroot=tools/probe-tactical-distance.zig -Msurface=src/surface.zig`
  and same with `-O ReleaseSafe`;
  `zig build-exe -O ReleaseSafe --dep surface -Mroot=tools/probe-tactical-distance.zig -Msurface=src/surface.zig -femit-bin=/tmp/nyon-tactical-distance-probe`.
  Adjust only compiler-validated option ordering, recording actual commands.
- [ ] Measure100 cached batches of16,384 intents at side32 after setup/verification,
  with the same deterministic corpus in each batch: for j0..16383,
  goal=j%6144 and source=(37*j+19)%6144. Both ID sets cover all6,144 cells.
  Serialize NYONDC01,u32 count,then each(side:u16,goal:u16,source:u16),all LE,
  and report its SHA256 plus an observable output checksum. Prepare corpus
  outside timed queries but include its storage in probe peak accounting.
  Report sorted p95
  (nearest-rank95th sample), min/max, initialization time, verification time and
  named host/build mode. Also measure one ordinary-BFS batch separately if bounded;
  one sample is not baseline p95. No5ms full-tick pass is claimed.
- [ ] Run `zig fmt --check tools/probe-tactical-distance.zig`, freeze source,
  report actual tests/counts/checksum/memory/timing and request independent source
  review. Any incomplete comparison or absent host measurement remains Unverified.

## Self-review

All five review-focus classes have checks above. This probe implements only the
candidate investigation's geometry and measurement questions. Gameplay scratch
lifetimes, hostile targeting, complete tick cost and browser runtime remain
separate review/qualification work; no cap or rule is weakened to fit this probe.
