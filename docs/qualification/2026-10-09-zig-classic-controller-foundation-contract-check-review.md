# Task3I1 terminal contract clarification independent review

2026-10-09. Preparation review only. Specification compliance: PASS. Quality: PASS within this scope. No Critical, Important or Minor findings identified. No implementation, fatal probe, compiler, test or gate was run; the active parent guide gate was not touched.

## Reviewed clarification and findings

- Severity: None; no new finding.
- File:line: `task-3i1-foundation-contract-check-addendum.md:3`; accepted correction `task-3i-execution-slices-lifetime-fix.md:23`, `:31`, `:43`; stable precedent `zig/src/retained_semantics.zig:7`, `:30`, `:193`.
- Description: Requiring explicit non-free-entry detection and unconditional panic in Debug/ReleaseSafe/ReleaseFast makes the accepted terminal contract assertion precise. H1 already uses an explicit conditional `@panic`, including token-budget and preparation-budget deinit occupancy checks. The clarification neither changes public signatures nor turns invalid-ticket/closed-reserve typed failures into panics. It does not reset generations, relax teardown order or introduce arbitrary reused-address protection.
- Suggestion: Implement deinit by checking every entry state before committing terminal closed state, with an explicit panic branch rather than an elidable debug assertion. Preserve existing checked lifecycle error behavior. Qualify the contract through separate processes, as described below.
- Status: Acceptable at specification level; implementation and all-mode behavior untested.

## Caller obligations and implementability

The registry can inspect its own entries and detect both reserved and owned occupancy. It cannot detect an unregistered external borrow, copied ticket or outstanding consumer holding a raw pointer after entries are free. The clock likewise cannot enumerate external CurrentViews or queued/published revision values. The addendum correctly leaves those shutdown proofs with the caller instead of promising a detector (`task-3i1-foundation-contract-check-addendum.md:3`). This matches the accepted lifetime correction's stable-address/no-copy/no-reset/nonescape protocol and I3 integration boundary. Hidden-view detection is not required of the foundation.

No implementability conflict identified. A private explicit contract helper or direct panic branch suffices; no dependency, process-global discriminator, owning lifetime token or new storage population is needed. The instruction does not require exporting mutable registry state. Reserved occupancy is reachable through the proposed public reserve API; owned occupancy is reachable through normal ownership admission and markOwned. Internal test seams, if needed, must remain checked and test-only.

## Fatal-probe fixture boundary

The proposed standalone fatal probe is appropriate because a real panic cannot be treated as an ordinary successful in-process unit test. Later qualification should use separately launched probes for reserved and owned occupancy in every requested mode, plus a valid all-free shutdown control. Capture the actual process outcome and expected contract-panic diagnostic; an arbitrary nonzero result from failed compilation, missing executable or unrelated crash is not occupied-deinit evidence. The all-free control should complete normally and exercise deterministic closed reserve/invalid-ticket rejection while registry storage still exists.

Use real API admission and legitimate synthetic ownership, not out-of-range pointers, freed storage or deliberate same-address reinit. Do not expect successful cleanup after the intentional fatal contract violation. Probe/harness code remains outside the shipped API, without mutable-state exports or production switches to disable checks. These are future verification recommendations, not executed receipts or additional host-policy approval.

## Evidence and remaining scope

Read the full addendum and accepted lifetime review, reused the original slices/lifetime correction already read in this scoped review chain, and checked stable H1 contract methods. Read-only SHA256 verification exited0 and confirmed:

|Evidence|SHA256|
|---|---|
|task-3i1-foundation-contract-check-addendum.md|731a9ee39fe9c0a473d7e598181881a68d62cbcc35d7f77e90b7ba5767df12bb|
|task-3i-execution-slices.md|db78dd944b41ff15777ff67411199d076c2b93ca2659d5836e776e59c1297b60|
|task-3i-execution-slices-lifetime-fix.md|f33e008d82c5bf0c8eab4b857203385b3253370a4af4b19bc13b5fcd29b0e42f|
|task-3i-execution-slices-lifetime-review.md|39bee6b96457f5610257def71c057839c16a254288abd333db4bd2808820144d|
|zig/src/retained_semantics.zig|7e68e9b83d5b5da7e145a02bb8d2c04968558e22c307535b637cade597a735cb|

Only this new ignored review was written. No product/Git/dependencies/assets/subagents or transient guide/UI/root/README was read or changed. This verdict does not resolve numeric caps, host adherence, producer delivery, I3 ordered teardown or any evolving guide integration. Actual target implementation, independent probes and parent gate evidence remain separate prerequisites.
