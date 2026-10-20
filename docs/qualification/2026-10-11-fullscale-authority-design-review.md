# Full-scale authority design: independent architectural review

Date: 2026-10-11. Verdict: **Revisions required before implementation approval**.
Three Important contract gaps; no Critical defect established. This is a review
of a Proposed architecture, not an implementation or performance qualification.

Reviewed full-scale specification:
`docs/superpowers/specs/2026-10-11-nyon-zig-fullscale-authority-design.md`, SHA256
`a09fb2da02b18df95fabb1319399289b92d9d14ee76e55bed152ca17442f8e69`.
Line references below refer to that exact revision unless another file is named.

## F1 — Important: define ownership during resumable index movement

**Locations:** specification lines 114–129, 150–157, 241–247, 365–373.

The final-state pool envelope is feasible, but it does not yet establish the
ownership invariant at each interruption point. The proposed index is compact
and sorted, insert/delete/move is resumable, and cleanup traverses its references.
There is no definition of which row slots own references during a partial shift,
or how a partial shipment child traversal is recorded independently of its parent.

For example, inserting before `[A,B,C]` by backward copies first writes C into
slot 3, then B into slot 2. The physical array is now `[A,B,B,C]`. Traversing the
old length misses C and releases B twice; traversing the extended length still
releases B twice. Increasing page refcounts for temporary duplicate rows instead
breaks the stated two-root reference bound. This is a concrete interruption state
the contract must distinguish from a valid candidate index; the arithmetic alone
does not select a safe algorithm. A move/hole cursor or ownership-preserving swap
algorithm can resolve it within the reservation.

Similarly, copying a shared shipment parent must not retain its route children
again merely because a new parent page exists: child ownership is per root.
Partial root preparation must remember which children were actually retained;
abort must not decrement unretained children or lose retained children whose
parent row has not yet become reachable.

**Fix direction:** specify the index insertion/removal state machine and its
ownership intervals, including logical count, transient hole/duplicate slots,
pending row/parent/child references, and exact cleanup enumeration. State whether
the compact/sorted invariant applies only at stable boundaries. Keep all such
cursors inside the 2 MiB control allowance and explicitly prove at most two
detached physical pages throughout parent/child changes. Define which actions
charge separate units and how zero budget preserves those states. Require abort
after every move, retain, link, unlink and release, including maximum-size roots.

The current dense implementation provides a useful precedent, not an automatic
proof for this replacement: `zig/src/planet_economy/state.zig:295–355` explicitly
tracks `prepare_retained`, `pending_new_ref`, `pending_old_ref` and release progress;
lines 413–467 distinguish allocation/copy/swap/release. Sparse index movement and
child ownership add states that those dense-directory cursors do not cover.

## F2 — Important: freeze the bounded history-provider ownership protocol

**Locations:** specification lines 171–186, 277–314, 345–352, 371–373.

The resident cache limit and host-owned durable stream are sensible boundaries.
However, the stated acknowledgment and source-provider facts are not yet a
complete pure-authority interface. The draft requires validating a returned
segment's identity/digest after eviction without specifying where the trusted
expected identity/digest comes from, or how its authenticated metadata remains
bounded. Retaining every segment digest inside authority would violate the fixed
control reservation: at the admitted 32-segment/tick envelope, 100,000 ticks give
3,200,000 segments and 102,400,000 bytes of digests alone. This is a worst-envelope
calculation, not a prediction of the required endurance fixture's actual history.
Accepting a digest returned alongside arbitrary bytes only validates those bytes
against that supplied digest; it does not bind them to the leased history.

The full-history lease pins only a current segment window. The missing contract
must also ensure that branch deletion, replacement, undo or session shutdown
cannot release an evicted segment before that lease next requests it. The phrase
"the durable session owns lifetime" does not specify acquisition, transfer,
release or cancellation of the reference that establishes that lifetime. The
same gap affects fault shutdown's requirement to wait for providers: a stale
completion must not retain or write into a freed authority, and an unavailable
provider needs a bounded abandonment/drain protocol.

Replay targets also regenerate more than 64 MiB over long histories. The draft
does not choose whether they archive their output, compare and reuse authenticated
source segments, or obtain another explicitly owned retention reference before
eviction. Without that choice, a replay can stop on target history backpressure
despite a fully available durable source, or acquire unaccounted output ownership.

**Fix direction:** freeze a storage-independent request/completion protocol with
owner/job generations, bounded input/output buffers and outstanding lanes, exact
source-prefix identity, authenticated segment lookup, immutable retention-token
lifetime, and stale/canceled completion disposal. Assign the unbounded durable
index to the session explicitly; authority retains only a bounded authenticated
window or another specified bounded verification mechanism. Specify replay target
history disposition and retention transfer on installation. Keep all hashing,
copying, segment lookup verification and completion ingestion charged to the
shared allowance; use typed values without importing store/OS/browser modules.
Tie undo/redo/branch/source pin operations to this contract without requiring a
third authority. This interface can be frozen before the separate store schema,
but cannot be deferred until delivery after dependent authority implementation.

## F3 — Important: new canonical identity still lacks a reviewed byte contract

**Locations:** specification lines 63–64, 224–239, 259–275, 319–352.

The proposal selects a new rules identity and hash domains, but does not freeze
its genesis grammar, receipt tags/headers, input/output order, length widths or
canonical state field grammar. "Enums explicit tags" and "length-prefixed arrays"
do not choose tags or widths. Nor is it stated whether routed inputs are emitted
in global sequence order while outcomes follow world/sequence execution order,
or exactly where local autonomous events occur relative to input/commit records.
Different implementations can satisfy the prose and compute different canonical
bytes under the same `nyon.fullscale-galaxy.v1` identity.

This is observable in global last-quota contention: a higher sequence on world 1
executes before a lower sequence on world 2, but both the replay inputs and their
outcomes need one frozen transcript order. Genesis must include every admitted
configuration value and initial counter/policy distinction, including how the
manifest wire is supplied/bound. The maximum manifest exceeds the 2,048-byte
record ceiling, so binding its digest with separate immutable genesis data versus
fragmenting its wire must be an explicit choice. Neither choice requires changing
the seeded manifest identity.

**Fix direction:** provide a reviewed normative codec appendix or explicitly
required codec-contract companion before implementation-plan approval. Specify
every field/tag/width/order, genesis configuration and initial state, domain
framing, malformed-input rejection and event ordinal/reset semantics. Include
independent literal fixtures for empty ticks, canceled first ticks, policy changes,
same local IDs across worlds and cross-world last-quota contention. Derive the
maximum-event and byte bound from that fixed grammar plus current emit sites.
The existing plan-approval obligation at lines 266–275 correctly refuses to
truncate or reject valid full ticks, but cannot be discharged using unspecified
record bytes. New strategy/city/combat schemas may remain separate reviewed
extensions; this finding concerns the initial local-economy full-scale identity.

The approved coordinated contract is more precise here:
`docs/superpowers/specs/2026-10-11-nyon-zig-coordinated-galaxy-design.md:377–425`
fixes widths, tags and both routed-input and prepared/commit order. Current local
event families appear in `zig/src/planet_economy/types.zig:152–188`; its encoder
is `zig/src/planet_economy/history.zig`. Neither existing codec automatically
defines the new full-scale wire.

## M1 — Minor: update the moving Proposed session dependency references

Lines 305–308 describe an earlier 32 MiB / 65,536-node session draft. The separately
edited Proposed session design snapshot inspected during this review, SHA256
`b876852106404496b27b935509e119e6ef157aa16dcd6f85070a7faa72161ddb`, now states:

- `docs/superpowers/specs/2026-10-11-nyon-zig-expanded-durable-sessions-design.md:260–270`:
  streamed immutable transaction objects, 1 MiB per tick, 131,072 nodes, genesis
  at most 1 MiB and portable container at most 144 GiB.
- Its lines 403–405 reserve a 1 MiB complete transcript before each authority tick;
  line 528 sets the 128 MiB store/transfer transient allowance.

This remains a moving Proposed dependency, not an approved implementation fact.
The 1 MiB per-tick contract still does not admit this proposal's entire 32 MiB
envelope. Update the dated citation and coordinate the compatible reviewed
contract, rather than claiming the original limits are current or silently
widening a decoder. A new version versus revision of an unimplemented Proposed
version is a controller decision that must be recorded. The draft already marks
this integration as pending, so the stale citation is not another independent
Important finding.

## Independent arithmetic and supported design conclusions

Computed with a disposable Python integer calculation, importing no NYON source:

| Quantity | Independently recomputed result |
| --- | ---: |
| Sum of declared record maxima, including reserved families | 74,512 rows |
| 4,096 paths × ceil(65 × 8 / 256) | 12,288 additional pages |
| Declared occupied-page envelope | 86,800 pages |
| 131,071 minus 86,800 | 44,271 pages of reviewed-extension headroom |
| Two maximum roots plus two detached slots | 2 × 131,071 + 2 = 262,144 slots |
| Pool | 71,303,168 bytes |
| Two indexes | 6,291,456 bytes |
| Seven allocation totals | 188,809,216 bytes = 180.0625 MiB |
| Source plus one replacement/replay target | 377,618,432 bytes = 360.125 MiB |
| Margin under 384 MiB | 25,034,752 bytes |
| Two authorities plus 128 MiB store allowance | 511,836,160 bytes |
| 512 current island reservations | 4,424,990,720 bytes |
| Current coordinator plus four islands | 47,153,152 bytes |

The pool inequality is conditional on F1's ownership invariants; it proves the
selected numeric envelope, not the absence of transient leaks or duplicate refs.
The index's 131,072 physical slots do not authorize that many one-page logical
records when the stronger per-root page ceiling is 131,071. Future multi-page
strategic schemas cannot spend the reserved extension envelope without review.

The global descriptor switch preserves a feasible atomic-publication design:
all-world phases finish before publication, old leases retain the complete old
root, and those leases block retirement/the next mutation. The fixed one-hour
clock, ascending-world execution and explicit charged empty-world steps preserve
the approved participation contract. No third root, independently ticking world
or visibility-dependent simulation is proposed. Command preflight and explicit
global/per-world rejection retain deterministic quota behavior.

Seven-block allocation accounting, polled initialization, independent allocation
ownership for fault destruction, no poll allocation, zero-budget immutability and
checked counter exhaustion are useful specified obligations. A control/layout
witness and failure-cut-point tests remain required before approval. They were
not run here, and this review does not claim the proposed Zig structs exist.

## Scope, evidence and completion boundary

Read home/repository guidance, the approved coordinator specification, expanded
roadmap, reviewed seeded design and fix review, the delegated-authority record,
current local storage/history source and the separate Proposed session draft.
The seeded design hash matches its reviewed revision:
`5577e8886f80a48df656fa1eb53ff3421a74c9deaecb5d2e7307001f43f4320d`.

Current evidence is source/document inspection and independent integer arithmetic.
Implementation correctness, pinned-compiler layouts, allocation failures, maximum
fixtures, event bounds, native/Wasm equality, live persistence, host memory and
performance remain unverified. In particular, metered flat-index work does not
prove the retained 5 ms full-step target; that target remains required alongside
the separate poll target. No gameplay, endurance or platform requirement is waived.

Only this new report was written. No source, author specification, instructions,
ledger or other review was edited; no builds, tests, agents or commits were run.
Controller approval under delegated authority remains available after the exact
revised contracts are independently reviewed. No further human approval pause
is requested by this report.
