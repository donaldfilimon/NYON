# Workshop V1 Recovery Repair Review — 2026-09-04

## Verdict

**APPROVE FOR SOURCE COMMIT.** No P0-P2 source defect remains in the reviewed recovery slice. The client/session preserve the loaded generation separately from the CAS head, retain an explicit recovery-persistence obligation through promotion and Continue selection, retry failed promotion after bounded backoff, and apply the canonical replacement invariant to top-level routes plus in-session load/import. Memory, transaction-model, and native regressions retain the exact authoritative-valid predecessor. Live IndexedDB recovery remains an acceptance-evidence gap, not a source-review defect.

## Scope and evidence boundary

- Reviewed only `src/workshop/store.rs`, `src/workshop/store/web.rs`, `src/workshop/session.rs`, `src/app/client_runtime.rs`, `tests/workshop_recovery.rs`, and the single `WorkshopStoreSnapshot` fixture adjustment in `tests/workshop_ui.rs`.
- Compared the repair with Findings 3 and 4 in `docs/superpowers/reviews/2026-09-04-workshop-v1-baseline-review.md` and the accepted persistence contracts at `docs/superpowers/specs/2026-09-02-nyon-v2-design.md:390-429`.
- `cargo test --test workshop_recovery --test workshop_session` passed all 21 focused tests in this final re-review: 7 recovery tests and 14 session tests. The controller previously reported host and wasm gates green; this review did not independently rerun those broader gates and does not treat wasm compilation or the host-only IndexedDB conformance model as live IndexedDB recovery evidence.
- No product file was edited, staged, or committed.

## Findings

### 1. [Resolved] In-session load/import now use the canonical recovery-aware replacement invariant

- **Severity:** Resolved (formerly P1)
- **File:line:** `src/workshop/session.rs:268-290`, `src/workshop/session.rs:465-487`, `src/workshop/session.rs:508-533`, `src/workshop/session.rs:624-633`, `tests/workshop_recovery.rs:354-419`
- **Description:** `replacement_request_blocked()` now delegates to the canonical `replacement_blocked()` predicate, so the same `recovery_persistence` obligation guards product routes, `RequestLoad`, and `RequestImport`. It temporarily removes only actions that are ordered after the currently drained replacement request; those actions are restored before dispatch continues and observe any replacement job started by the current action. The two new regressions inject a failed promotion, prove load/import cannot replace the recovered digest or start replacement work during backoff, complete promotion and Continue selection, then prove the same actions become eligible. This closes the session-action bypass without creating a second persistence invariant.
- **Suggestion:** Keep the canonical delegation and both regressions together; future authority-replacement actions must use the same invariant.
- **Status:** Resolved.

### 2. [Resolved] Native authority-invalid-head recovery now retains the exact recovered predecessor

- **Severity:** Resolved (formerly P2)
- **File:line:** `tests/workshop_recovery.rs:427-481`, `tests/workshop_recovery.rs:483-536`, `src/workshop/store.rs:1101-1157`
- **Description:** The repair preserves the original physical-corruption test and adds a native integrity-valid JSON (`{}`) head whose authoritative replay fails. The new path drives client recovery and promotion, verifies the promoted archive and retained predecessor bytes exactly, corrupts the promoted head, and proves native fallback returns the original known-good generation. This exercises the native sequence missing from the first review and confirms the exact-generation retention behavior in `PromoteRecoveredSlot`.
- **Suggestion:** Keep both tests: they cover distinct storage-integrity and authority-validation recovery paths.
- **Status:** Resolved for native source behavior and host-test evidence.

### 3. [Evidence gap] Live browser recovery-promotion behavior is not yet qualified

- **Severity:** Evidence only — not a source defect
- **File:line:** `src/workshop/store/web.rs:43-79`, `src/workshop/store/web.rs:506-582`, `tests/workshop_recovery.rs:298-301`
- **Description:** `indexeddb_model_recovery_promotes_authoritative_bytes_and_retains_the_known_good_predecessor` runs `IndexedDbTransactionModel`, whose mutation is delegated to `MemoryWorkshopStore`. It does not execute the wasm-only `promote_recovered_slot` implementation, its pre-transaction generation read, its multi-store read-write transaction, or transaction commit/abort behavior. The wasm compile gate establishes type correctness only. The accepted design requires browser archive/ref replacement in one IndexedDB transaction and recoverable handling of quota, abort, schema, unavailable storage, conflict, and eviction; the new promotion request adds a distinct browser transaction path with no runtime regression.
- **Suggestion:** Add a browser IndexedDB integration test for authority-invalid head -> prior-generation replay -> promotion -> reopen, plus an aborted/quota-denied promotion case proving the prior reference and archive remain intact. Assert that a later corrupt promoted head falls back to the exact retained authoritative-valid generation.
- **Status:** Open acceptance evidence — required before claiming live IndexedDB recovery or browser release qualification, but not a blocker to committing the reviewed source repair.

## Requirements coverage

| Requirement | Evidence in repair | Status |
| --- | --- | --- |
| Preserve loaded generation separately from durable head | `LoadedSlot`, `PendingDecode`, `ResidentSlot`, session snapshots, and `ClientRuntime::continue_selected_workshop` carry both values | Covered |
| Accepted fallback is not immediately Continue-ready | Recovered sessions clear the selected-generation latch and retain an explicit obligation through promotion and Continue selection | Covered |
| Promote with CAS against the corrupt/current head | `PromoteRecoveredSlot` checks `expected_head_generation` in memory, native, and wasm IndexedDB implementations | Covered by source and host tests |
| Retain the exact authoritative-valid predecessor | Promotion selects `recovered_generation` explicitly rather than rescanning for a structurally valid generation | Covered on successful path |
| Reopen cleanly after promotion and survive a second corrupt head | Memory/model and both native recovery modes exercise successful promotion and retained fallback | Covered for host implementations; live IndexedDB remains Finding 3 |
| Retry failed recovery persistence without permitting replacement | Explicit obligation and five-second backoff cover top-level routes, `RequestLoad`, and `RequestImport`; focused regressions verify all three paths | Covered |

## Commit recommendation

The reviewed recovery source is **approved for a focused source commit** closing baseline Findings 3 and 4 at the source/host-test layer. Keep the commit limited to the reviewed recovery slice and its tests. This is not browser release approval: real IndexedDB recovery, transaction abort/quota behavior, reopen, and second-corrupt-head fallback still require a live browser journey. Preserve the controller's workspace and wasm gates as separate evidence from that future runtime acceptance.
