## Design Document Review: NYON Zig Full-scale Authority Implementation Plan

### Summary

**PASS for the proposed task decomposition; no new Important/major or Critical plan blocker found. Product execution remains blocked:** Task 0 joint session-contract acceptance and Task 1 complete pinned-compiler layout witness are unfulfilled prerequisites, and the resulting exact revisions require fresh review before execution approval.

This review covers initial local-economy authority plan readiness. It does not approve integrated city/tactical mechanics, the moving session proposal, implementation correctness, or full-game acceptance.

### Reviewed revisions and drift

Read the complete plan, authority design and codec, the retained fix-2 review, and the source seams listed below on 2026-10-11. SHA256 values checked directly from disk:

| Document | SHA256 |
| --- | --- |
| `docs/superpowers/plans/2026-10-11-nyon-zig-fullscale-authority.md` | `0125c407f3500c221993df1345ab436cad729a7a726410463ba6e81c5a7aa5a6` |
| `docs/superpowers/specs/2026-10-11-nyon-zig-fullscale-authority-design.md` | `a8bc0d41767722b3215e2e6c334d06d5cf5cacc3107f28f83b3b4219ba6ce2f5` |
| `docs/superpowers/specs/2026-10-11-nyon-zig-fullscale-authority-codec.md` | `c116655859eae999ee1529180521254a575570ba471099fc0ca2e00c8778b6b9` |
| `docs/superpowers/specs/2026-10-11-nyon-zig-seeded-galaxy-design.md` | `5577e8886f80a48df656fa1eb53ff3421a74c9deaecb5d2e7307001f43f4320d` |
| `docs/superpowers/specs/2026-10-11-nyon-zig-expanded-durable-sessions-design.md` | `ce8f58f9a4427a741b1fa72586975cee534ce18977669370af35edc78970b4f5` |

The first three match the assigned review revisions. The session document has changed from the plan's explicitly dated observation `d448fa0479445bdb748dc03de23c17374692845fb08c92b2fe992b0337798992`. Task 0 lines 243–270 already treats that document as moving Proposed evidence and requires rehashing and written acceptance; this drift does not silently update the approved authority contract or constitute a new plan defect. The codec's seeded-spec hash still matches.

### Open issues

No additional critical, major, minor or nit correction is required for this scoped planning review. The following are retained execution prerequisites, not newly discovered defects and not satisfied by this PASS.

### Retained prerequisite: Joint publication and session contract

Plan Task 0 lines 251–270 correctly identifies that `SessionAppendReservation`, `PreparedTranscriptCursor`, `GraphPrepared` and coupled resident-session publication are absent from the approved authority revision. The live session proposal still contains those requirements in its “Stream reservations and publication” section, including preparation before its one owner-thread switch. Raising the tick ceiling from the historical 1 MiB snapshot to 32 MiB does not reconcile those interfaces.

Task 0 requires accepted exact revisions, bridge declarations, ownership, bounded candidate access, memory accounting, cancellation, and independent merged-contract review. Line 124 prevents an unspecified bridge file from entering implementation ownership; Task 12 lines 721 and 739 explicitly consume only that accepted bridge. Retain this gate. Record the accepted authority/session amendment and update the plan's exact declarations before witness admission; do not treat approval of this task sequence as approval of the session proposal.

### Retained prerequisite: Complete compiler layout witness

Task 1 lines 281–349 requires witness-only admission after Task 0 and review before Tasks 2–15. Its requirement to instantiate the complete cursor inventory, simultaneous temporary state, owner handles, accepted bridge, descriptors, quotas, source/proof windows and two transport slots is appropriate. In particular, line 338 says: “A byte array called ‘controls’ without its complete instantiated types is insufficient.”

The seven-block arithmetic and proposed `PageSlot`/`IndexRow` definitions are not a layout result. A passing assertion over an empty reservation or cursor placeholders would not satisfy this prerequisite. The actual witness must include every concrete backing type and legal overlap, on native and Wasm, with checked offsets/alignment; later cursor growth must rerun it. The control-block-first allocation ledger in Task 2 lines 375–378 places the owner inside the reservation rather than allocating an eighth owner object.

### Verified plan coverage

- **Task dependencies and ownership:** Tasks 0–1 precede product work; storage and codec precede index/COW, transport/history/provider, commands/phases, lifecycle, replay and qualification. Earlier transport/storage tests can exercise their owned layers without claiming that the later integrated job/session path has passed. Task 12 supplies that integration and Tasks 14–15 retain end-to-end qualification. Shared root/main/build/gate edits remain an explicit integration-owner handoff, not permission for the full-scale writer. Each task has path ownership, behavior tests and fresh specification/quality review.
- **Interrupted ownership:** Tasks 4–5 preserve the design's extent/hole/pending-row model, child masks and separate parent/child COW ownership. Cancellation during cleanup must resume the existing release cursor. The full-root fixture is explicitly a storage-envelope test, with no invented canonical strategic records.
- **Committed transport immutability:** Task 6 lines 528–543 covers partial context copy, unfinalized tail SHA state, seven sealed leaves plus a tail, carry overwrite, unavailable provider, cancellation, retry and postpublication retirement. Task 7 lines 565–571 prevents candidate sealing from making committed-tail storage evictable. This retains the substantive fix-2 contract rather than copying only its nominal byte sizes.
- **Work accounting and publication:** The common `WorkBudget`, zero-budget no-change rule, metered hash/copy/index/ownership operations and command-snapshot exception remain explicit. Task 12 lines 737–749 requires charged terminal delivery, complete preparation, one failure-free owner-thread publication and retirement before reuse. Neither a per-world allowance reset nor fallible work inside publication is introduced.
- **Initial codec and replay:** Task 3 binds all 34 quota values, seven commands, complete state/receipt grammar, independent literals, physical/canonical encoding separation and the conservative 47,106-record / 4,057,265-byte bound. Tasks 10–11 preserve structural-error versus gameplay-rejection distinctions, global input ordering versus world execution ordering, all-world barriers, and semantic validation. Task 13 compares regenerated events and logical records, requires the complete quota setup history, and keeps the 296-byte fragment from becoming a claimed replay golden.
- **Provider and target lifetime:** Task 8 uses copied tickets, locally owned prefix roots, bounded authenticated windows and explicit broker disposition acceptance. Stale/delayed completions cannot retain authority pointers. Task 13 transfers the replay owner and backing pin together, forbids a third authority, distinguishes verified bytes from published target bytes, and requires a new exact-prefix pin before a fork.
- **Qualification and operability:** Tasks 14–15 retain allocator cut points, complete capacity fixtures, native/Wasm distinction, requested bytes versus RSS, diagnostic output and the existing repository gate. Lines 860–863 measure complete steps separately from polls and require a reviewed algorithm revision on full-step p95 above 5 ms. Durable crash/CAS/delete/GC/export/import and real endurance evidence remain the session owner's separate obligations.
- **Future mechanics:** Global Constraints, Tasks 3 and 14, and line 882 prohibit using reserved strategic rows, placeholder payloads or empty campaigns to claim city/tactical or strategic acceptance. Those moving Proposed mechanics require a reviewed versioned contract before admission.

### Source verification

The plan's reuse boundaries match the inspected code:

- `zig/src/planet_economy/rules.zig:8` defines pure `AlgorithmContext`; `:30` derives terrain and `:38` qualifies footprint behavior. This supports an adapter without constructing island storage.
- `zig/src/planet_economy/command.zig:453` defines the existing cursor; its `advanceContext` at `:1171` takes `*state.Candidate`. `zig/src/planet_economy/phases.zig:7` likewise owns island-specific writers and receipts. These are behavioral references, not directly reusable full-scale cursors.
- `zig/src/surface.zig:30`–`:47` supplies coordinate conversion and adjacency. Its tests cover reciprocity, seams and coordinate validation; no test execution is asserted here.
- `zig/src/coordinated_galaxy/types.zig:26` retains the v0 identity and its configuration at `:38` stores four planet configurations. This does not establish 512-world authority capacity.
- `zig/src/seeded_galaxy/generate.zig:5` provides stateless keyed formulas; `wire.zig:5` writes explicit canonical fields. `manifest.zig:1` documents a unique owning handle, retains an allocated output pointer and frees it in `deinit`. The plan is correct to prohibit retaining that ownership as an uncounted eighth block. Source presence does not establish the independent qualification required by Task 0.
- `zig/.zigversion` contains the plan's exact compiler version. `zig/build.zig:3` checks it and currently routes shared roots/build targets. `zig/tools/check.sh:1` identifies the implemented migration gate and deliberately rejects Rust subprocesses. Its existing Wasm target is compilation evidence, not runtime equality.

### Strengths

- The plan translates the reviewed failure cases into named interruption, rollback and lifetime tests with explicit owners.
- Publication, complete replay and durable retention remain separate proof obligations rather than being inferred from codec hashes or short polls.
- The exact memory reservation includes owner/control state and requires concrete compiler evidence before implementation approval.
- The retained 5 ms full-step target can force algorithm revision; neither smaller capacities nor short poll latency can waive it.

### Evidence limits and handoff

Only this review file was written. No source, specification, plan, ledger, instructions, build configuration or earlier review was changed. No compiler witness, build, product test, gate, remote operation or commit was run; there is no build/test exit code to report. Read and hash commands completed successfully. Existing dirty work was preserved.

Retain the planning PASS. Complete and independently review Task 0, admit only Task 1 witness work, then obtain fresh review of the exact accepted contracts, updated plan and measured witness before authorizing Tasks 2–15. Runtime allocation/lifecycle behavior, replay, persistence, browser/Wasm execution, latency and RSS remain unverified.
