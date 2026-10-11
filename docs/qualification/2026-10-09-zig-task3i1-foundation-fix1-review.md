# Task3I1 FIX1 scoped independent re-review

2026-10-09. Verdict: M1 ADDRESSED. No new Critical, Important or Minor findings. This re-review covers only the canceled-slot reuse correction and breakage introduced by that correction. The original full implementation review remains immutable. Parent aggregate qualification remains Partial; no host/controller acceptance follows.

## Evidence freeze

|Evidence|SHA256|
|---|---|
|FIX1 report|d6b8325622a20cf5fdd21c291337b9e2bbfb3205fbe18e7ec980712dbff0fd5f|
|FIX1 final-source receipt|b0c84b73774ffcef77022f25ff865af6a29c5ce26be9e8a8ec10380be76d351b|
|FIX1 artifact manifest|78e9e763147ba610dbe93c5285a8d33868c0bebf0002c91a16b57db1b8b1d711|
|FIX1 baseline-source receipt|6b00bec3ac4a79bb13394ab4863b1ac5caccf053da6c2d064049c6691ca467da|
|FIX1 full-context baseline patch|f5231d7e3d85c94d5fa3ac8ab5383458f5b9b651180b093018b02fde92f9c230|
|Amended classic_input_tests.zig|947f782e16cf12b8aff8bc542cc3d66a1c8710644cad82898af626cb6869f434|
|Original independent full review|810bf7e4c28b690d3a6949f8d9dd0f8100690ca6bb0e6479f43e8ab32e042cf3|

Independent read-only verification confirms all 398 FIX1 manifest artifacts and all 4,227 original artifacts match SHA256 and byte lengths. All 73 protected current identities match. FIX1 baseline equals the original final-source receipt; all four baseline copies match it. All four FIX1 final copies match current bytes and final receipt. The original 24,194-byte paired test is an exact byte prefix of the new 25,516-byte test: only the appended 24-line, 1,322-byte fixture changed. Production foundation, root and README retain original hashes. The full-context FIX1 patch independently reconstructs the entire baseline and amended paired test exactly. Original full review SHA256 is unchanged.

Read the complete appended test, its full diff, FIX1 report and archival runner, all five complete outputs/receipts and their pre-run source identities. Independently verified all 375 pre-run source snapshot identities (75 each); no mismatches. Every receipt has the FIX1 final four-path hashes and bytes, canonical cwd, exact argv, UTC start/finish and terminal child status. The runner snapshots before subprocess execution and writes to the separate FIX1 evidence directory.

## M1 disposition and correction behavior

Location: zig/src/classic_input_tests.zig:493–516. Original finding: task-3i1-foundation-independent-review.md, Minor M1; original report line 46.

The fixture reserves and cancels a ticket, keeps its stale copy, then re-reserves in the same still-live pinned registry (495–498). It explicitly checks the same index and a strictly greater generation (499–500), with exactly one occupied entry (501). Stage 0 tests the fresh reserved state; stage 1 legitimately marks the fresh ticket owned (502–503). Each stage snapshots the full fresh entry, then independently calls old cancel, old markOwned and old release. Each operation must return InvalidTicket and preserve the complete new entry, including state, purpose and generation (504–510). Occupancy remains one after each stage (511); successful fresh release restores zero and permits terminal shutdown (513–515).

This directly exercises the missing canceled-reservation reuse analog in both relevant new states. Old cancellation cannot clear the new reservation, old markOwned cannot steal it, and old release cannot retire the new owner. Checking full entry identity immediately after each failure plus final occupancy prevents falsely passing via a later repair. The fixture uses legitimate registry methods and a single admitted lifetime; it adds no arbitrary address-reuse or forbidden reinitialization claim.

FIX1 report explicitly acknowledges the original coverage overstatement and supersedes only that statement with new evidence. Historical documents and RED remain unchanged. M1 is therefore addressed in both fixture coverage and provenance.

## Focused receipts and new-breakage assessment

Debug (2026-10-09T23:29:44.745452+00:00 to 23:29:51.191387+00:00), ReleaseSafe (23:29:51.283358 to 23:30:03.979248 UTC) and ReleaseFast (23:30:04.038701 to 23:30:14.322272 UTC) each child exit 0 and all 22 tests passed, explicitly including the new fixture. These are 15 foundation plus seven imported H1 tests per mode, not an aggregate count of distinct tests. fmt-write and fmt-check each exit 0; check covers both foundation files and root. Full pre-run identities match final current source.

The change adds only one local synthetic registry test. There is no production API/storage/lifecycle mutation, added dependency or changed host behavior. New test state is fully quiescent at exit and does not affect later tests. Existing focused size/request witnesses remain identical. Prior all-mode fatal probes continue to apply to byte-identical production foundation; this test-only amendment does not require fabricated new fatal evidence. No new fix-induced breakage found.

No compiler, test, formatter, gate, product mutation, subagent or commit was performed in this re-review. Only this new ignored review was written. Parent remains responsible for the aggregate gate and every previously stated caller-lifetime/I3/host proof gap.
