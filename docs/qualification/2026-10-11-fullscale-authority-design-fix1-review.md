# Full-scale authority design: scoped fix 1 review

Date: 2026-10-11. Verdict: **F1 and F3 addressed; F2 partially addressed with one
Important transaction-state gap remaining. Implementation approval remains open.**
M1 is addressed as a dated dependency correction. No Critical finding established.

Reviewed exact revisions:

- `docs/superpowers/specs/2026-10-11-nyon-zig-fullscale-authority-design.md`
  SHA256 `f32ae2ea88a582bcb72e72c7b7b0ba1adfb1ab43af0b06cda74c4fcacc79b125`.
- `docs/superpowers/specs/2026-10-11-nyon-zig-fullscale-authority-codec.md`
  SHA256 `453fb362910d214f9e2e634b890c983b89853d875c7d304b34e172f5f64e8284`.

This reviews the original F1/F2/F3/M1 and directly connected regressions. The raw
review `2026-10-11-fullscale-authority-design-review.md` remains unchanged.

## F1 — Addressed at the design-contract level

Design lines 159–225 now define which physical index slots own references during
insertion and deletion. A move transfers ownership into the hole and changes the
hole position in the same bounded step; stale bytes in the new hole own nothing.
Pending rows count toward the candidate root ceiling, and cleanup enumerates the
extent excluding the hole, pending row, and separately owned transition handles.
This resolves the original interrupted `[A,B,B,C]` ambiguity.

Preparation separately tracks the retained parent and the three child-retain
bits. Cancellation releases only retained children. Parent COW transfers the
candidate's existing child ownership without retaining children again; child COW
waits until the parent transition drains. These rules preserve one reference per
owning root and the two-root-plus-two-transition-slot argument. Updating masks
with each reference operation prevents a yield between release and bookkeeping.

The 2 MiB control fit, seven-block layout, maximum-root cut points and zero-budget
behavior still require the stated compiler witness and implementation tests.
This review establishes that the stated algorithm has an ownership model, not
that those implementation checks have passed.

## F2 — Partially addressed; Important transport-state rollback gap remains

**Remaining locations:** design lines 353–359, 388–398, 310–316 and 539–547.

The new interface resolves the original unbounded-digest/authentication problem:
the session owns the durable index, authority retains a fixed-depth frontier and
verification window, and segment proofs bind to a locally retained Prefix root.
The exact-prefix pin owns the complete closure, including future cursor windows;
branch deletion and GC cannot invalidate a live source. Bounded Tickets, copied
completion values, explicit broker disposal and shutdown dispositions prevent
late provider writes through freed authority pointers. Replay output comparison
and source-pin reuse avoid a second unaccounted archive. These are substantive
fixes to the original finding.

However, the newly introduced **single append frontier and current-tail hash
state have no committed/candidate ownership or rollback rule**. Lines 388–398
state that sealed appends update the one frontier. A candidate must derive its
transport root before the failure-free descriptor switch. Cancellation then
discards invisible history tails, but no rule preserves or restores the previous
frontier occupancy, peak hashes, sealed-segment count or tail hash context.

Concrete interruption: the committed stream has seven sealed segments and a
partial tail. A candidate seals that tail and another segment. The first append
merges occupied frontier peaks; the second can overwrite a lower-level peak that
the committed seven-segment prefix still needs. Cancel before publication. The
committed Prefix digest is insufficient to invert that hash update, and earlier
segments may already have left the resident cache. Discarding candidate bytes
does not restore the committed append state. Rebuilding from arbitrarily old
durable history would introduce an unspecified provider dependency and work path
into ordinary cancellation. Updating the live frontier after publication instead
also fails to satisfy the already-prepared/failure-free publication contract.

**Required fix:** assign bounded committed and candidate frontier/tail contexts
to the two descriptor/root generations, or specify an equivalent bounded undo or
delta algorithm. Include leaf count, frontier occupancy/peaks, partial-tail hash
state and tail length, and distinguish committed versus candidate segment sealing
and eviction eligibility. Prepare candidate transport state incrementally; the
same descriptor switch must select it. Abort must discard only candidate context
and must not need evicted bytes or storage availability. Fit the contexts inside
the existing blocks, retain the stated requested-byte totals, and keep all copy,
hash, rollback and retirement work charged. Require interruption witnesses after
each append/carry/tail-seal/fold and cancellation followed by an identical retry,
including a previously evicted prefix and a zero-budget poll.

This is a directly connected regression introduced by the authentication fix,
not a request to design the physical store in this document. A second fixed
frontier or bounded rollback ledger is small; neither is currently specified by
the normative statement that there is one frontier. Until its lifecycle is fixed,
F2 cannot be closed or the full authority implementation approved.

**Nonblocking consistency edit:** design lines 545–546 still say fault shutdown
waits for providers to release, while lines 428–447 deliberately transfer delayed
release ownership to the broker so authority destruction need not wait for I/O.
Clarify that local leases and ownership-disposition transfer must finish, while
provider I/O may drain under broker ownership. The detailed new protocol supplies
the intended safe behavior; the older summary should match it.

## F3 — Addressed; independent literal and bound verification passed

The companion now defines all 34 fixed quota fields, initial counters/policy,
the separate immutable genesis object, manifest binding, canonical state records,
command bodies, receipt tags/widths, sequence/provenance fields and event order.
It separates globally ordered routed inputs from world/sequence-ordered outcomes,
retains exact prior policy, and makes canceled work absent from canonical history.
The genesis object resolves the earlier large-manifest/2,048-byte receipt ambiguity.
Strategic rows remain forbidden in this initial identity.

A disposable independent Python calculation used the printed scalar grammar,
explicit masked SplitMix64 arithmetic and standard-library SHA256. It imported
no NYON code or author calculation script. It regenerated the following complete
literal byte strings and matched every byte and stated raw SHA256:

| Fixture | Bytes | Result |
| --- | ---: | --- |
| Genesis object | 415 | Exact match |
| Framed genesis receipt | 136 | Exact match |
| Empty first tick | 167 | Exact match |
| Policy change on empty second tick | 173 | Exact match |
| Same local settlement ID on two worlds | 583 | Exact match |
| Cross-world last-quota ordered fragment | 296 | Exact match |

The 121-byte seeded manifest, its digest, domain-separated G and all four stated
state/history digest pairs also matched independently. The same-ID state includes
separate world metadata, hall claims and settlement rows; food 8 becomes 6,
fed streak becomes 1 and housing remains 4 after the supply phase. The quota
fragment matches the two input orders and accept/reject bytes under its separate
full-profile precondition. It remains a fragment, not evidence of executing a
complete setup history or the capacity fixture. Canceled-first-tick behavior
remains an implementation lifecycle obligation rather than an executed test.

The source emit sites in `zig/src/planet_economy/command.zig:1145–1164` and
`zig/src/planet_economy/phases.zig:35–55,86–104,155–195,198–249,252–305`
support the stated conservative family counts: one command outcome, one arrival
per shipment, up to one completion and one production/idle event per building,
and one supply plus at most one demography event per settlement. Independent
field-width sums match every row of the companion's event-size table.

Independent totals:

- `2 + 2*1024 + 4096 + 2*16384 + 2*4096 = 47,106` records.
- `71 + 1024*(1074+88) + 4096*75 + 16384*(57+72) + 4096*(55+54) + 106`
  `= 4,057,265` framed bytes.
- Margin below 4 MiB: 137,039 bytes; below 32 MiB: 29,497,167 bytes.

This is a conservative encoded-byte upper bound; combining independent maxima
does not assert that all maxima occur in one reachable tick. Pinned-compiler
encoding, maximum fixtures, deterministic replay and cancellation tests remain
required. The independent calculator exited successfully; no product test or
build was run.

**Nonblocking wording precision:** companion lines 160–162 should name
`routed_input`, not generic "input," in the command-provenance rule. The exact
tag table and literals already give `tick_input` autonomous provenance and no
sequence, so the intended bytes are clear. Similarly, design line 336 calls the
2,048-byte bound a framed-record bound whereas companion line 135 applies it to
receipt bytes before the four-byte frame. Initial maximum records are well below
both limits; align the wording before future extensions use that ceiling.

## M1 — Addressed as an explicit moving dependency

Design lines 467–488 identify the separately owned Proposed session snapshot,
state the current 1 MiB/tick incompatibility, and require coordinated revision
before implementation approval. They retain complete 32 MiB transaction support
as an extension requirement and require recalculation of graph, portable and
namespace bounds. No decoder or shipped format is silently widened. This review
does not approve the separate session proposal or claim the incompatibility has
already been resolved by that owner.

## Retained boundaries and report hygiene

No new Important F1/F3 regression was established. The one remaining Important
finding is the transport-state transaction lifecycle above. The requested-memory
figures remain Proposed allocation budgets: 180.0625 MiB per authority and
360.125 MiB for source plus target. No layout, allocator or RSS claim is inferred
from the codec calculation. The 5 ms full-step target remains mandatory alongside
the distinct poll target; metering proves neither hardware result.

Only this new follow-up report was written. No author specification, codec,
product source, shared ledger, instruction file or raw review was changed. No
agents, builds, product tests or commits were run. The review does not authorize
implementation or claim full-game, persistence, browser or hardware acceptance.
