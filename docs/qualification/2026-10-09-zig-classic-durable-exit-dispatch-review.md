# Task 3I4 independent preparation review

2026-10-09. **PASS for concrete preparation. Critical 0; Important 0; Minor 0 findings. PROPOSED, NOT DISPATCHED.** This accepts the specification and proposed test design for later parent-controlled admission; it does not accept an implemented module or authorize a compiler run.

## Exact reviewed identities

- Binding SHA-256: `fefc671bd68a8df152f486e71922ac5fb5ba46a34b443f00918924db1b1afe65`.
- Binding report SHA-256: `5f5faf71ed0994e938085b2442a6345e60ef0319d2630102f5f76a7b7f7c5539`.
- Rust durable exit SHA-256: `aa3d6156bf7400bbdf11f3a8b661ba7d977f9fcfd4a3758ea36b68465397f3b8`.
- Rust app caller SHA-256: `67b489723ec3886b38873f12ac6a21e763d0f6fc6d21c709b85fd27730eaaea2`.
- Rust input router SHA-256: `966a9618f3dc44dc25feae1e09f79ad5ee4f83b24189df2601b477e3a49cc68a`.
- Accepted routing brief SHA-256: `d2a5702b16fbbc3bf8145d94dbb011ab770ade972c9376483222f6ef99481e7e`.
- FIX1 routing review SHA-256: `2dab0d66b560a5adf8440c8fe2ca9522eb2916485fffcae11961ca16b6e13816`.

Read the full binding/report, actual Rust module and relevant caller ranges, corrected routing brief and FIX1 review, the execution-slice ownership map and gate script. Both future durable-exit source/test paths were absent at review. The review output is Git-ignored. Evolving classic_input, root.zig and README were not inspected or adopted as integration baselines.

## Independent spec and quality conclusions

The proposed API preserves the exact first branch of Rust request: request(false) returns ExitNow without reading or modifying pending. Therefore the pending-true corner remains true and a subsequent cancel returns true. request(true) first marks pending and requests preparation, then returns Wait on repeat. Observation absent from pending returns Wait for either readiness; pending/not-ready waits; pending/ready clears exactly once. cancel returns prior pending and resets it. The complete literal matrix and explicit request(true)/request(false)/cancel and ready-after-preserved-pending sequences cover these corners without obtaining expected values from the proposed implementation. [src/app/durable_exit.rs:21; src/app/durable_exit.rs:32; src/app/durable_exit.rs:44]

The source callers substantiate the scope boundary: Living save and its separate pending flag precede this machine; Workshop readiness is supplied by the caller; preparation rejection leaves pending; polling retries caller preparation; actual quit is a caller flag. Escape closes guide first and otherwise consumes successful cancellation. The binding neither invents Classic autosave nor equates cancellation with aborting a store mutation or pure ExitNow with completed host closure/archive work. [src/app.rs:1938; src/app.rs:1962; src/app.rs:1985; src/app/input_router.rs:466]

The sole bool state, value copy and pointer mutation API require no allocator, retained resource or deinit. Size/alignment assertions are target-relative to bool and paired with source inspection; copy-independence and comptime lifecycle witnesses are appropriate supplementary checks. No OOM/error fixture is warranted for this total boolean API. Concrete implementation must still compile the proposed syntax, measure actual target size/alignment, and inspect the complete module: a comptime witness alone would not prove absence of arbitrary runtime-only branches. The binding explicitly requires source inspection and real lifecycle tests, so its aggregate evidence requirement is sufficient. [src/app/durable_exit.rs:4; src/app/durable_exit.rs:16; task-3i4-durable-exit-dispatch-binding.md, Independent literal matrix and paired tests]

The proposed behavioral mutant yields a real preserved-pending assertion failure, with syntax/import failure explicitly disallowed as RED. The same witness must become GREEN. Focused Debug/ReleaseSafe/ReleaseFast and parent integrated gate receipts remain future requirements, with hashes, executable identity, argv, target, exits and bounded logs. No compile/test success is implied by this review. [task-3i4-durable-exit-dispatch-binding.md, Fresh admission, RED/GREEN, coordinated integration]

Ownership remains two NEW product files for the future worker, fresh absence/identity/owner checks before dispatch, and parent-only additive root/test imports after foundation release. Root drift is explicitly recorded in the preparation report, not treated as a stale future baseline. I4 independence is consistent with the accepted bounded execution map; sequential parent dispatch and complete implementation review still apply. No source, dependency, instruction, ledger, asset or host integration changes are admitted by this review. [docs/qualification/2026-10-09-zig-classic-controller-execution-slices.md:17; task-3i4-durable-exit-dispatch-binding-report.md, Read-only postwrite verification]

## Limits

Wrote only this new ignored review. No product edits, compiler, formatter, gate, commits, dependency changes or subagents. Runtime, browser/native, graphics, storage completion and whole-game acceptance remain unverified and outside this pure preparation review. Shared-memory routing hints were consulted only to establish that old NYON observations cannot replace live qualification; verdict rests on current inspected sources.
