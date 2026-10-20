# Full-scale authority design: scoped fix 2 review

Date: 2026-10-11. Verdict: **Remaining F2 addressed. No Important or Critical
regression found in the scoped changes. Proceed to detailed planning.**

Reviewed exact revisions:

- `docs/superpowers/specs/2026-10-11-nyon-zig-fullscale-authority-design.md`
  SHA256 `a8bc0d41767722b3215e2e6c334d06d5cf5cacc3107f28f83b3b4219ba6ce2f5`.
- `docs/superpowers/specs/2026-10-11-nyon-zig-fullscale-authority-codec.md`
  SHA256 `c116655859eae999ee1529180521254a575570ba471099fc0ca2e00c8778b6b9`.

Scope is the remaining F2 transport-state transaction lifecycle and directly
connected regressions. The original and fix 1 reports remain unchanged. Their
resolved F1/F3/M1 findings are not reopened by this scoped review.

## F2 — Addressed at the design-contract level

Design lines 388–399 and 405–429 now assign separate transport contexts to the
two descriptor/root slots. Each includes frontier peaks and occupancy, sealed
leaf count, tail ordinal/length and incremental hash state, byte count, prepared
root, prefix metadata and bounded operation cursors. The committed context stays
immutable while the candidate executes. A cached height-32 root explicitly covers
the full-tree case without indexing beyond the 32 frontier peaks.

Lines 431–460 define charged preparation and publication. The destination cannot
be reused before obsolete ownership retires. Context copying takes at most 16
256-byte charged chunks, with the copy cursor outside the copied range and no
candidate append/hash operation using a partial copy. Generation binding and
candidate readiness occur at explicit bounded transitions. Tail digest evaluation
finalizes a temporary copy of the unfinalized appendable SHA context, preserving
the ability to append to a committed partial tail.

Sealing, binary carry and folding mutate only the candidate context. All hash
blocks and peak/occupancy transitions are metered before the barrier. The existing
single descriptor-index switch selects the prepared gameplay root and transport
context together; it performs no transport copy, fold, hash or fallible operation.
This resolves the publication-side ambiguity without adding a second commit point.

Lines 462–485 also protect the shared physical tail: candidate bytes may extend
its invisible suffix, but the committed length/hash remains unchanged. Candidate
sealing cannot authorize prepublication eviction or reuse. Abort discards only
candidate cache ownership and the copied/candidate context range in charged
chunks. It neither reconstructs the committed frontier nor reads evicted history.
Retry starts from the unchanged committed context. The seven-sealed-leaf example
from fix 1 therefore no longer destroys the lower-level committed peaks when the
candidate carries and overwrites its own peaks.

Lines 487–504 retain old contexts for old leases after publication and make only
obsolete-context retirement legal afterward. They require interruption coverage
for partial copy, seal, carry overwrite, fold, ready transition, zero budget and
postpublication cancellation. Cancellation with an unavailable provider and an
already evicted prefix is explicitly required. These are mandatory future tests,
not execution results established by this review.

## Memory, work and connected wording checks

The new partition is arithmetically consistent: two 4,096-byte slots consume
8,192 bytes inside the existing 2,097,152-byte control allocation, leaving
2,088,960 bytes for the remaining controls. Copying or zeroing one complete slot
takes 16 chunks of at most 256 bytes, plus separately charged state transitions.
No third context, allocation or history rescan is authorized. The seven-block
totals remain 180.0625 MiB per authority and 360.125 MiB for source plus target.

The actual pinned-compiler SHA context size, all cursor/control fields, alignment
and offsets still require the specified layout witness. This report verifies
the selected partition arithmetic, not a measured Zig layout or allocator result.

The prior minor wording conflicts are corrected:

- Design lines 647–650 require local lease drainage and disposition transfer;
  delayed provider I/O drains under broker ownership without an authority pointer.
- Design line 336 applies the 2,048-byte ceiling to receipt bytes before the
  four-byte frame, matching companion line 135.
- Companion line 160 now names `routed_input` in the command-provenance rule.

As a direct regression check, all six literal fixture strings retain the lengths
and raw SHA256 values independently verified in fix 1: 415, 136, 167, 173, 583 and
296 bytes. A disposable standard-library calculation checked those bytes and
hashes again. No complete semantic fixture reconstruction was repeated in this
scope; the independent reconstruction and 47,106-record / 4,057,265-byte bound
remain the evidence recorded in the fix 1 report.

## Approval and evidence boundary

The remaining architectural review blocker is closed for these exact revisions.
Controller specification approval under the existing delegation and detailed
planning can proceed. Implementation approval still requires the stated compiler
layout witness and reconciliation of the separately owned Proposed session
contract; design lines 569–590 explicitly retain that dependency. This review
does not approve the incompatible session proposal or authorize a silent limit
change. The full-step p95 target of 5 ms remains required separately from poll
latency and must be measured on named hardware.

Only this new report was written. No author specification, codec, product source,
shared ledger, instruction file or earlier report was edited. Reviewed hashes
were checked from disk. No builds, product tests, agents or commits were run.
Runtime, allocation-failure, maximum-capacity, replay, persistence, browser and
hardware qualification remain unverified; no full-game acceptance is implied.
