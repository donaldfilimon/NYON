# Tactical distance probe qualification — 2026-10-11

Current: independently reviewed isolated native feasibility experiment. This qualifies the probe, not a production movement consumer or complete city/combat tick.

## Identity and verification

- Plan SHA256: `ff7102605a50c2456d4c6a38576a4cceb801399307c1a981194f874e737fbadb`.
- `zig/tools/probe-tactical-distance.zig`: `493c2531fe407b29d8bb4433c8160c8e9e7f1a2166274f933bed1aed87156026`.
- Unchanged `zig/src/surface.zig`: `7dfd507054ba36cce761ca023230a96ad8465136808e85f6e5dbd680403ee40e`.
- Final independent review: APPROVED, no open findings; SHA256 `2ffe6a318609b31d20dbd80c0250bc8416bd890fd96b74466bef1f7f685fe125`.
- Exact compiler: `0.18.0-dev.146+35accc06e`; observed host Apple M5, Mac17,2, arm64.

From `zig/`, observed Debug and ReleaseSafe tests each passed 2/2:

```sh
zig test -O Debug --dep surface -Mroot=tools/probe-tactical-distance.zig -Msurface=src/surface.zig
zig test -O ReleaseSafe --dep surface -Mroot=tools/probe-tactical-distance.zig -Msurface=src/surface.zig
zig build-exe -O ReleaseSafe --dep surface -Mroot=tools/probe-tactical-distance.zig -Msurface=src/surface.zig -femit-bin=/tmp/nyon-tactical-distance-probe
```

Build, fmt and exhaustive executable verification returned 0. The preceding behavioral RED failed as intended when adjacent distance returned 255 instead of 1. Final reviewer independently checked identity, diff reconstruction, count arithmetic and corpus serialization; she accepted the existing Zig receipts without rerunning them. No broad repository gate was run for this qualification.

## Observed results

All sides 4..32 passed transform bijection, inverse and neighbor preservation: 3,290,688 transformed cells and 13,162,752 directed neighbor checks. All source/goal pairs on sides 4, 5, 8 and 32 passed ordinary BFS distance and original-ID next-cell comparisons: 37,927,908 pairs. The oracle shares the topology and BFS helper; its independence is from compressed lookup. Maximum complete-field distance was 95.

Two separately owned 6 MiB partitions each use 6,205,340 bytes and retain 86,116 bytes slack. Native alignment is 8; both allocation addresses had remainder zero. Explicit requested heap peak, including the 98,304-byte corpus, is arithmetically 12,681,216 bytes. Allocator overhead, runtime allocation, complete compiled stack and whole-process peak were not measured. Reported stack arrays are selected source-declared subtotals.

The deterministic side-32 corpus contains 16,384 intents and covers all 6,144 IDs in each role. Its literal LE serialization SHA256 is `9384ab1124339e221f176cc67a42fc487b4989abce21e27df220eb9a14867cd9`. Across 100 identical batches, cached-query min/p95/max were 2,262,083 / 2,860,666 / 3,614,500 ns. Setup and exhaustive verification were outside that measurement. The single ordinary-BFS baseline measures distance only, so it establishes neither comparable movement speedup nor baseline p95.

## Remaining qualification

Production authority setup/cancel/free integration, compiled native and Wasm aggregate layouts, browser execution, allocation failure injection, complete stack/process peaks, and maximum-load city/combat/history/hash/cleanup timing remain unverified. This result does not establish the 5 ms complete-tick target. The two partitions must fit existing tactical reservations; no extra cache allocation is authorized by this receipt.

Detailed run receipts and independent reviews are retained in `.superpowers/sdd/2026-10-11-tactical-distance-feasibility/`. The workspace remains because commits were not authorized and it contains unique evidence.
