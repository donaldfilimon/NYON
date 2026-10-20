## Design Document Review: Authority/session Task 0 binding — fix 1

### Summary

**The reconciliation addendum passes this scoped technical rereview of the four original gaps; actual joint Task 0 acceptance still needs revision/adoption.** No new Important/Critical technical defect was found in those four fixes or their direct interactions, but the exact external S/D documents have not adopted the refinement, so this is not an implementation approval or a Task 0 PASS.

Date: 2026-10-11. Scope is the four findings in `2026-10-11-authority-session-contract-review.md` and direct regressions only. The original review is preserved. City mechanics, the full session feature, catalog/ledger changes outside these interfaces and a fresh whole-authority review are out of scope.

### Exact revisions

Paths in this table are relative to `docs/superpowers/specs/`. All five hashes matched the assigned revisions at initial read and the pre-write recheck. Final drift status is recorded below.

| Name | Document | SHA-256 |
| --- | --- | --- |
| B | `2026-10-11-nyon-zig-authority-session-bridge-design.md` | `ae8f3b3f39c03dc2bb867ef39f274af56a44ac3b73fecf8f4a378fb780c9eff3` |
| D | `2026-10-11-nyon-zig-durable-authority-bridge-design.md` | `acf3592befd3c3f28febcf18a5ce4ed78427e0fef181f335c1622c22ec032959` |
| S | `2026-10-11-nyon-zig-expanded-durable-sessions-design.md` | `a3d5b5e04204ecf16fe729a12306bfe868723af6195be48f4692e2ab7b147637` |
| A | `2026-10-11-nyon-zig-fullscale-authority-design.md` | `a8bc0d41767722b3215e2e6c334d06d5cf5cacc3107f28f83b3b4219ba6ce2f5` |
| C | `2026-10-11-nyon-zig-fullscale-authority-codec.md` | `c116655859eae999ee1529180521254a575570ba471099fc0ca2e00c8778b6b9` |

B and D were read in full; S's dependency, provider, stream/publication, budget and acceptance sections were checked against them. A/C are the unchanged exact documents read during the original review. B explicitly refines the existing `nyon.fullscale-durable-adapter.v1` execution profile; it introduces no competing gameplay, archive or storage profile. C's six literal/cancellation fixture sections remain byte-for-byte untouched because its full-file hash is unchanged; no fixture was executed or regenerated in this rereview.

### Issue 1: The refinement has not yet been adopted by the owning S/D contract

- **Severity**: major (Important; blocks actual Task 0 compatibility acceptance, not the scoped technical acceptance of B)
- **Section**: B “Inputs, concurrent revision and authority,” lines 35-42, and “Required amendments by the other owner and controller,” lines 758-778; S dependency lines 86-98, provider lines 612-645 and admission lines 659-668; D lines 239-245, 324-341 and 395-410.
- **Description**: B says its precedence takes effect “only when the controller records a jointly reviewed hash tuple including all five files” and requires the external S/D owner to adopt its concrete bindings. The pinned S instead names only D's current hash. D still calls the four-disposition handoff “unchanged” at lines 397-399 and does not contain B's pending-pin tagged states/nine-position broker ledger. D's acceptance says the parent is absent “only at genesis” and its prepared-source grammar admits initialization sources only for initialization; B adds the necessary cold `replay_existing` tick-zero exception at lines 433-443. S's generic pre-begin node/insertion reservation also has not acquired B's explicit zero-node/full-graph replay exemption. B resolves these matters as a proposed higher-precedence refinement, but an implementer dispatched against the currently adopted S/D tuple would still receive different instructions. Their absence is not proof that B's solution is unsound; it is an outstanding ownership/integration action already acknowledged by B.
- **Suggestion**: The **S/D owner** adopts B's explicit full bindings, cache-only window contract, purpose-specific cold replay, pending-request disposition grammar and bounded broker inventory. This can be an explicit normative adoption with precise superseded clauses; it need not duplicate all declaration text. The **controller** then pins A/C, the resulting S/D revisions and B in the plan/integration receipt, names the consuming owners, and requests a focused verification of that final tuple. Keep B's four amendment bullets as the exact checklist. The separately approved A/C need not be edited merely to erase historical standalone wording; the composed-profile precedence must be unambiguous. Do not dispatch a source writer or mark Task 0 PASS on this rereview alone.
- **Status**: open

### Original Issue 1: Joint publication and complete ownership binding

**Resolved in B, subject to external adoption.** B:78-180 declares owner-bound authority/resident/reservation/seal values and keeps large records in fixed private registries. B:251-318 provides the missing integration owner and binding checks: branch/cursor/tip and graph generations belong to the session, authority validates its own frozen bytes and independently hashes the session envelope using the bound parent, and the bridge compares both before sealing. The local fence prevents generation/selection changes during final validation; foreign stale offers cannot abort another job, while a matching mismatch drains its moved graph once.

`publishSealedNoFail` consumes prepared state by one paired selector update; its primitive contains no lookup, comparison, increment, allocation, hashing, callback, I/O or failure. The wrapper resolves the token before invoking it under the same fence and without a yield. Unselected pair metadata and window ownership are already prepared. Public authority and graph views resolve that same selector; there is no intermediate authority-only publication. CAS conflict is correctly a later durable-save result that leaves the resident pair committed and dirty (B:320-322). The fixed copy/hash/ref operations and terminal delivery remain charged through the shared allowance (B:200-249, 711-715).

### Original Issue 2: Stream access, cache lease transfer and cancellation

**Resolved in B, subject to external adoption.** B:324-408 distinguishes private frozen candidate bytes, committed history windows, genesis object and genesis stream. Candidate cursors are bound to a frozen job, range and checked pass/epoch; repeats require close/reopen and cancellation/publication invalidates them. The old partial tail is acquired before begin and read from cache, while a prepared window covers the tail plus at most 32 MiB of new bytes. All per-slot refcount work finishes before sealing; publication changes only prepared visibility/ownership flags.

The eight role-limited windows are per authority and pin cache slots independently of gameplay roots. Releasing a PairLease after taking independent graph/window ownership therefore permits root retirement while an unsaved transcript remains readable. Windows cannot grow beyond resident cache coverage; role exhaustion or unavailable bytes is explicit backpressure. The contract does not promise that two maximum-size unsaved ticks always fit simultaneously. The authority's candidate/cache payloads remain the backing storage; postpublication staging rereads those windows through the existing segment/batch arenas rather than keeping all changed leaf payloads.

B:383-391 and the stage table assign abort/retirement ownership for insertion pages, manifest/path scratch, old-tail/prepared masks and authority roots/context. A completed broker batch remains pinned until reconciliation. Cursor/window generations prevent reads through reused slots. These clauses close the original missing lifecycle; actual refcounts, union layouts, cancellation behavior and absence of extra allocations remain implementation-test obligations.

### Original Issue 3: Live append versus replay/genesis

**Resolved in B, subject to external adoption.** B:410-513 makes purpose registry-owned and restricts `replay_existing` to the detached domain. It reserves zero new logical nodes, insertion pages, unsaved descriptors or output archive batches, so replay at the 131,072-node graph cap does not fail live append admission. Cold replay explicitly bootstraps tick zero against the existing graph using E, zero parent and the initial stream; it does not incorrectly require a new empty graph. Subsequent replay steps compare complete regenerated bytes and the expected session hash, derive their own shorter Prefix, and authenticate a whole source leaf before exposing only the permitted target range.

New genesis has its separate initialized-root/empty-graph path. Final installation uses affine target/install handles and a session-issued resident disposition proof, checks stale generation before transfer, moves the target into the resident unselected pair and then uses one selector. Old resident leases retain old ownership until retirement; no third authority is created. The bounded fake host must use the same purpose-specific registries and authenticating checks. Target-local publication/cancellation never mutates resident dirty state.

### Original Issue 4: Pending pins, stale completions and shutdown

**Resolved in B, subject to external adoption.** B:515-647 names four positions per authority domain, including `pending_pin` before any RetentionToken exists, and makes the source read's Ticket distinct from its token's acquisition Ticket. Reserving four broker positions for each of two authorities plus one shared abandoned position gives nine bounded ledger records without authorizing nine physical I/O operations. Two physical requests and one global source read remain the limits; unresolved predecessor positions block replacement admission.

Shutdown transfers tagged pending requests as well as successful tokens. Busy preserves local ownership; acceptance moves the already-registered complete batch once, and an acknowledgment retry observes the recorded transfer. After namespace closure, late pin success becomes broker disposal work, identical duplicate success cannot release twice, and contradictory success tokens fault/reconcile rather than guess. Source request references outlive cancellation until physical disposal. The explicit header → proof → body → verification/consumption grammar fixes lengths, contiguous offsets, stages and end handling; stale tickets reject before copies. Provider callbacks remain pointer-free, zero budget consumes no ingress part, and no unverified byte reaches replay.

### Arithmetic, bounded work and remaining proof

An independent Python standard-library check summed the declared rows and recomputed input and segment bounds. The command completed with actual exit **0**:

| Check | Verified arithmetic |
| --- | ---: |
| Authority bridge control rows | 3,584 bytes; 512 spare inside 4,096 |
| Store bridge control rows | 24,672 bytes; 8,096 spare inside 32,768 |
| Two transport contexts plus authority bridge | 12,288 bytes; 2,084,864 left in 2 MiB controls |
| Store control remainder after bridge | 1,015,808 bytes |
| 1,024 staged envelopes × (4 + 2 + 1,024) | 1,054,720 bytes |
| Staged stream plus 1,024 × 16-byte indexes | 1,071,104 bytes; within 2 MiB |
| Minimum maximum-stream copy units | 4,120 at 256 bytes/unit, before parsing/encoding/control |
| Append 32 MiB at old end 7 MiB + 17 bytes | 33 affected cache segments |
| Source + target + entire 128 MiB store cap | 511,836,160 bytes |

The proposed four-item shutdown staging is included in store controls; it is not another authority allocation. Large binding/seal/batch records use handles across interfaces and charged field copies, and the fixed table includes copied output windows. The unchanged 8 MiB authority scratch partitions as 2 MiB owned input, 1 MiB authenticated provider input and 5 MiB other scratch. All consumers share the outer remaining allowance; the standalone synchronous bulk copy is inaccessible in this composed profile. Callbacks enqueue values only.

These are **arithmetic checks of selected envelopes**, not an actual `@sizeOf`/alignment witness, allocator trace or proof that every simultaneous runtime object fits. B explicitly preserves Task 1 as the pinned-compiler/native/Wasm layout gate and prohibits overflow allocations. No compiler-layout, build, product, provider, native/browser, failure-injection, performance or endurance test ran. The six codec fixtures, 100,000-tick workload and full-step target remain mandatory future evidence; this review does not reclassify them as passed.

### Strengths

- One paired selector and an immutable seal registry give both owners a concrete visibility and ownership boundary.
- Cache-only windows preserve transcript backing without unnecessarily holding an obsolete gameplay root.
- Cold replay, full-graph replay and fresh genesis have explicit and different admission rules.
- Pending pin ownership and late physical disposal now fit a named bounded ledger and tagged transfer protocol.
- The addendum states its external adoption checklist and refuses to turn specification arithmetic into compiler or runtime evidence.

### Final applicability and disposition

End-of-review hash check: **all five exact document hashes above remained unchanged; verification exit 0.** Only this new review file was written. The original review and all specifications, source, plans, ledgers, instructions and fixture files remain untouched by this reviewer.

**Scoped B fix review: PASS for the four original technical gaps. Joint Task 0 acceptance: not yet PASS because Issue 1 (external S/D adoption and final tuple acceptance) remains open.** Following that adoption, rereview the changed integration clauses and any direct changes to B; do not rerun unrelated design work or waive the separate layout/runtime gates.
