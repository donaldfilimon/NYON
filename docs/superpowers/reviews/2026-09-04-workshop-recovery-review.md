# Workshop V1 Recovery Repair Review — 2026-09-04

## Verdict

**REQUEST CHANGES.** The repair closes the successful-path mechanics behind baseline Findings 3 and 4: the client/session now preserve the loaded generation separately from the CAS head, an accepted fallback is initially not Continue-ready, and recovery promotion retains the exact generation that supplied authoritative-valid bytes. One P1 failure-path bug remains, however: a failed recovery promotion is never retried automatically and makes the unpromoted recovered session replaceable. The repair therefore does not yet satisfy the accepted persistence requirement that recovery remain durable-safe before the resident session can be discarded.

## Scope and evidence boundary

- Reviewed only `src/workshop/store.rs`, `src/workshop/store/web.rs`, `src/workshop/session.rs`, `src/app/client_runtime.rs`, `tests/workshop_recovery.rs`, and the single `WorkshopStoreSnapshot` fixture adjustment in `tests/workshop_ui.rs`.
- Compared the repair with Findings 3 and 4 in `docs/superpowers/reviews/2026-09-04-workshop-v1-baseline-review.md` and the accepted persistence contracts at `docs/superpowers/specs/2026-09-02-nyon-v2-design.md:390-429`.
- The controller reported focused, host, and wasm gates green. This review did not independently rerun them and does not treat wasm compilation or the host-only IndexedDB conformance model as live IndexedDB recovery evidence.
- No product file was edited, staged, or committed.

## Findings

### 1. [P1] A failed recovery promotion becomes neither retryable nor replacement-blocking

- **Severity:** P1
- **File:line:** `src/workshop/session.rs:214-223`, `src/workshop/session.rs:245-268`, `src/workshop/session.rs:793-807`, `src/workshop/session.rs:893-898`
- **Description:** `from_loaded` correctly represents an accepted previous-generation fallback as clean authority with `save_requested = true` and no selected Continue generation. If `PromoteRecoveredSlot` subsequently fails, `poll_commit` calls `defer_autosave_retry`. That helper clears `save_requested` and arms the dirty debounce, but the recovered session remains `dirty == false`. The update loop advances the retry timer only under `self.dirty && self.dirty_debounce_armed`, so no retry can ever be scheduled. At that point `continue_ready()` is correctly false, but `replacement_blocked()` is also false because there is no dirty state, queued save intent, or in-flight job. A New Workshop or Classic route may therefore replace the user-accepted recovered state while the corrupt head remains authoritative, and ordinary background updates can never repair it. This violates the intended Finding 3 invariant that accepted recovery stays non-discardable until the known-good bytes are durably promoted and selected.
- **Suggestion:** Model recovery promotion as an explicit persistence obligation rather than borrowing the ordinary dirty-autosave predicate. Keep that obligation replacement-blocking across start, poll, CAS, quota, abort, and I/O failures; apply bounded backoff without requiring authority dirtiness; and clear it only after promotion commits the new head and the exact generation is selected for Continue. Add a deterministic test that accepts a previous generation, injects one promotion failure, proves the session is not replaceable or Continue-ready, advances through retry, and verifies reopen no longer enters recovery.
- **Status:** Open — commit blocker for claiming baseline Finding 3 closed.

### 2. [P2] The new regression suite does not exercise the native authority-invalid-head path required by Finding 4

- **Severity:** P2
- **File:line:** `tests/workshop_recovery.rs:170-229`, `tests/workshop_recovery.rs:237-290`
- **Description:** The shared memory/model test uses integrity-valid JSON (`{}`) whose authoritative archive replay fails, which is the precise scenario behind Finding 4. The native test instead corrupts the generation file so its recorded length/hash fails storage validation; native `LoadSlot` therefore skips the head before the client attempts authoritative replay. That proves physical-corruption fallback and promotion, but not the native sequence in which the structurally valid head is returned, authoritative decode rejects it, `LoadPreviousGeneration` is requested against the observed head, and `PromoteRecoveredSlot` retains that exact predecessor. Finding 4 explicitly required equivalent memory, native, and IndexedDB coverage for the integrity-valid/authority-invalid case.
- **Suggestion:** Add a native test with a persisted `{}` head whose descriptor remains intact, drive the full client recovery/promotion flow, corrupt the promoted head afterward, and prove the retained recovered generation still replays to the expected digest and branch graph. Keep the existing physical-corruption test because it covers a distinct store-level fallback path.
- **Status:** Open — required to close the native portion of baseline Finding 4.

### 3. [P2] Browser recovery-promotion behavior is represented only by a host-side model

- **Severity:** P2
- **File:line:** `src/workshop/store/web.rs:43-79`, `src/workshop/store/web.rs:506-582`, `tests/workshop_recovery.rs:226-229`
- **Description:** `indexeddb_model_recovery_promotes_authoritative_bytes_and_retains_the_known_good_predecessor` runs `IndexedDbTransactionModel`, whose mutation is delegated to `MemoryWorkshopStore`. It does not execute the wasm-only `promote_recovered_slot` implementation, its pre-transaction generation read, its multi-store read-write transaction, or transaction commit/abort behavior. The wasm compile gate establishes type correctness only. The accepted design requires browser archive/ref replacement in one IndexedDB transaction and recoverable handling of quota, abort, schema, unavailable storage, conflict, and eviction; the new promotion request adds a distinct browser transaction path with no runtime regression.
- **Suggestion:** Add a browser IndexedDB integration test for authority-invalid head -> prior-generation replay -> promotion -> reopen, plus an aborted/quota-denied promotion case proving the prior reference and archive remain intact. Assert that a later corrupt promoted head falls back to the exact retained authoritative-valid generation.
- **Status:** Open — browser release-evidence gap; required before claiming the IndexedDB part of baseline Finding 4 closed.

## Requirements coverage

| Requirement | Evidence in repair | Status |
| --- | --- | --- |
| Preserve loaded generation separately from durable head | `LoadedSlot`, `PendingDecode`, `ResidentSlot`, session snapshots, and `ClientRuntime::continue_selected_workshop` carry both values | Covered |
| Accepted fallback is not immediately Continue-ready | Recovered sessions clear the selected-generation latch, request promotion, and the happy-path test asserts Continue unavailable before promotion | Covered on successful promotion path |
| Promote with CAS against the corrupt/current head | `PromoteRecoveredSlot` checks `expected_head_generation` in memory, native, and wasm IndexedDB implementations | Covered by code; failure recovery is broken by Finding 1 |
| Retain the exact authoritative-valid predecessor | Promotion selects `recovered_generation` explicitly rather than rescanning for a structurally valid generation | Covered on successful path |
| Reopen cleanly after promotion and survive a second corrupt head | Memory/model and native tests exercise successful promotion and retained fallback | Partially covered; adapter-specific gaps in Findings 2 and 3 |
| Preserve active state on failure | Store candidates/transactions remain atomic, but session replacement safety is lost after a promotion failure | Not covered; Finding 1 |

## Commit recommendation

Do **not** commit this repair as closure of baseline Findings 3 and 4 yet. Fix Finding 1 and add its injected-failure regression first. Add the native authority-invalid-head test and real browser IndexedDB transaction coverage before marking Finding 4 fully closed or making browser recovery claims. After those changes, rerun the focused recovery suite, the mandatory workspace format/clippy/test gates, wasm clippy/compile, and a live browser IndexedDB recovery journey as separate evidence layers.
