# NYON bounded executor continuation plan

Date: 2026-10-02. Status: **Proposed execution sequence; NYON remains Partial.**

This plan continues [the completion and expansion program](2026-10-01-sandbox-completion-and-expansion.md) from its first unfinished slice. It records completed components rather than restarting intent preparation. Donald's latest request is to write the plan; new implementation is on hold. Earlier implementation changes remain preserved in the canonical dirty checkout. This document does not imply that the tooling session is in Plan mode.

## Current checkpoint

The state-retirement checkpoint passed native formatting, strict workspace Clippy, all-target workspace tests, release build, separate locked fuzz formatting/Clippy, both browser backend builds, root/core wasm checks, wasm Clippy, Workshop artifact checks, vertex layout, original and additive V2 vectors, and the static cross-build fingerprint. Workspace tests: **1,426 passed, 0 failed, 0 ignored, 101 suites**. The driver reported unchanged source contents and paths across the run. Logs and source identities are retained in `.superpowers/sdd/2026-10-01-sandbox-completion-and-expansion/checkpoints/state-retirement/`.

| Component | Current scope | Remaining limit |
| --- | --- | --- |
| Owned forecasts and intent inputs | Immutable snapshot, normalized hash copy, Active actor forecasts and ordered demand requests; independently reviewed | Not a complete boundary executor |
| Economic and fleet enumeration | Budgeted candidates and exact reference ordering/paths; independently reviewed | Central reservations and complete ownership retirement remain |
| Incremental validation | Reference error precedence, retained owners and temporary-index retirement; independently reviewed | Whole publication lifecycle remains |
| State retirement | Move-only cursor over all 20 state collections and nested owners; independently reviewed | Catalog, job outputs and eventual last-owner routing remain |
| Resident stepping | Existing Session polls a transitional job and preserves fresh publication | Explicit synchronous completion tail remains; responsiveness is Partial |
| V3 specifications | Eight coordinated Proposed specifications and reviewed subset vectors | Open expiry-order defect, current full corpus and acceptance remain |

These gates establish automated source/build evidence. They do not establish live WebGPU/WebGL2 parity, real IndexedDB recovery, accessibility, frame timing or endurance. Fuzz compilation is not fuzz execution.

## Execution and ownership

Use one V2 product implementer at a time, followed by independent specification review and code review. Assign exact files before release; reviewers do not modify product source. Preserve existing dirty work, public commands, canonical bytes, identities and gameplay. No commits, pushes, publication, new dependencies, branches or worktrees are included.

A separate authorized task, “Integrate Living step job,” has requested ownership of further V2 creator/intents/sealing/resident work. Before implementation resumes, reconcile its current writers and source hashes with this ledger and assign a single owner. Retain the completed qualification; do not release overlapping writers. This plan does not itself transfer ownership or send a cross-task reply.

Every poll accepts 1..=1024 visits, shares one remaining budget through nested children, and reports actual consumption. Constructors, transitions, Ready polls, cancellation, faults and premature finish must preserve the same bound. No partial result may publish. Capacity-sized owners must remain retained or pass to a counted retirement cursor; releasing a shared Arc handle does not prove its payload was drained.

## 1. Catalog retirement — next implementation slice

**Owner:** one core implementer; `living/catalog.rs` private cursor/tests and minimal `living/step_job.rs` adapter. **Depends on:** reviewed state retirement.

- [ ] Drain the seven catalog vectors incrementally; visit recipe inputs/outputs and policy bonuses explicitly.
- [ ] Count collection, row, nested buffer, empty allocated buffer, document/root and canonical-byte releases. Preserve private fields, catalog hash and canonical behavior.
- [ ] Reuse exact Arc release/defer behavior: successful unique extraction drains the payload; failed extraction retains the original Arc ticket and pointer identity. Do not substitute strong-count checks or clone fallback.
- [ ] Cover owned/boxed/shared inputs, empty capacious owners, deferred uniqueness, cancellation at every stage, invalid budgets, premature finish and repeated zero-work Ready polls.

**Acceptance:** independent specification/code approval, focused parity/lifecycle tests and the current full automated gate. General catalog cleanup must not introduce a synchronous drop hidden in a poll.

## 2. Copy/hash handoff and fault ownership

**Owner:** one core implementer; `living/step_job.rs` and scoped tests. **Depends on:** catalog retirement.

- [ ] Add private borrowed once-only Ready extraction, retaining owners on premature finish. Preserve consuming public compatibility APIs.
- [ ] Retire partial copied state and active nested boxed rows without inserting malformed partial rows into ordered collections. Release the Box allocation through counted work.
- [ ] Preflight an outer sorted-key insertion error before taking a completed nested copy. Duplicate/unsorted malformed input must retain the nested owner on failure.
- [ ] Make hash `TooLarge.actual` agree with the reference's full serialized size. After the limit is exceeded, continue budgeted measurement to the end without accumulating bytes or publishing a partial digest; preserve JSON-error precedence.
- [ ] Retire hash fragments, literal/encoded buffers, queues and retained source/catalog owners explicitly.

**Acceptance:** full error parity at and around the size limit, long trailing content, malformed nested keys, normalized application tick, and cancellation during over-limit measurement at budgets 1, 64, 1024 and mixed budgets.

## 3. Rules-owned cleanup and central reservations

**Owner:** one authority implementer; `living/rules.rs`, required private adapters and focused tests. **Depends on:** steps 1–2.

- [ ] Route snapshots, forecasts, graphs, candidate batches, explanations, reasons, action bytes and deltas into bounded retirement; avoid accumulating full-state graveyards.
- [ ] Visit actors in stable ID order, economic category before fleet category, candidates in exact reference rank order. Preserve one winner per category, fallback after rejected candidates and unattempted lower-rank disclosure.
- [ ] Trial each proposal against the current reserved state while applying deltas relative to the original immutable snapshot.
- [ ] Preserve changed/inserted upserts before deletions across worlds, hull jobs, construction jobs, facilities, fleets and routes, including full nested equality and replacement semantics.
- [ ] Keep exact economic costs, refunds, jobs, route timing/cadence/reserve behavior, allocator marks and action/search ordinals.
- [ ] Classify reservation conflicts and queue-capacity rejections exactly; other validator faults fail the entire unpublished job. Publish successful reservations atomically.

**Acceptance:** retained-reference comparisons under scarcity, simultaneous proposals, full slots/replacements, Dormant actors, ownership loss, hidden/stale intelligence, route/shipment fallback and arithmetic faults. Compare ordered reasons and all state/queue identities, not only final stock totals.

## 4. Complete due-intent dispatch

**Owner:** same sequential authority lane; `rules.rs`, `step_job.rs`, dispatch tests. **Depends on:** step 3.

- [ ] Compose inputs, economic/fleet enumeration, reservations, dispatch and cleanup under one remaining budget, reusing the existing dispatch cursor.
- [ ] Preserve phase completion explicitly; do not infer it from nonempty event output.
- [ ] Keep the synchronous completion marker until sealing and publication are fully prepared.
- [ ] Compare boundary-50 and subsequent complete state, all three receipt families, queue IDs, allocator marks and history against the reference at all four budget patterns.

**Acceptance:** cancellation, stale queued commands, deterministic faults and premature finish publish nothing and retain cleanup ownership.

## 5. Receipt sealing, journal preparation and all creator cascades

**Owner:** one core boundary implementer; simulation/step/receipt/history/creator modules and scoped tests. **Depends on:** step 4.

- [ ] Preserve exact ordering: tick, validation, state digest, sorted revisions, ordinary events, facts, recovery, corridor, decisions, journal preparation, authenticated token, fresh commit, retirement.
- [ ] Bound payload encoding/hashing, identity derivation and receipt verification. Preserve facts-envelope prefix/omission selection and error precedence; preserve decision payload-size versus full-record validation distinctions.
- [ ] Copy nested journal commands incrementally; retain unchanged history ownership when no queue work requires replacement.
- [ ] Authenticate success and failure captures against state/catalog/queue/history/mode identities, seed, branch/head and high-water marks. Stale outcomes must not rerun against or consume a new queue tail.
- [ ] Complete every accepted creator operation and cascade. Existing counted simple operations are a checkpoint; remaining synchronous creator markers are unfinished work. Cover all 46 tools without changing accepted commands or identity rules.
- [ ] Route every eventual last owner through retirement, including displaced resident state/history and abandoned successful outputs. Deferred owners stay retained until properly handed off.

**Acceptance:** full boundary/reference parity, all creator/failure/cancellation cases, independently reviewed publication atomicity and ownership. No accepted path may retain an explicit synchronous fallback before step 6 completes.

## 6. Install the complete executor in the existing Session path

**Owner:** one coordinated core/runtime implementer; `sandbox_session.rs`, ClientRuntime and required simulation adapters. **Depends on:** steps 1–5 and writer reconciliation.

- [ ] Replace the transitional synchronous tail with the complete executor; preserve Step, 10 Hz pacing, holds, pause/resume, speed, fault display and fresh cache publication.
- [ ] Make synchronous compatibility stepping drain that same executor rather than maintain a second algorithm.
- [ ] Add resident-wrapper late-phase cancellation and authentic core-fault coverage; verify suspend/history replacement and stale-token behavior.
- [ ] Measure per-update work and latency independently of visit-budget correctness.

**Acceptance:** complete resident stepping with no accepted synchronous fallback; reference parity and current gates pass. Responsiveness remains Partial until measured live evidence meets the contract.

## 7. Replay, comparison, persistence and complete V2 journeys

**Owner:** sequential history/archive/runtime/storage and experience slices, each with exact file ownership. **Depends on:** complete boundary publication.

- [ ] Bound archive decode, graph scheduling, branch replay, comparison outputs and cancellation cleanup at record granularity.
- [ ] Complete paused/running creator journeys, all seven experiments, recorded evidence and exact historical views.
- [ ] Verify save/Continue/export/import, fork/compare and portable reproduction against authentic archives and all receipts.
- [ ] Exercise actual browser storage recovery, interrupted writes, quota/CAS outcomes and offline operation. Host transaction tests alone do not qualify IndexedDB.

**Acceptance:** failed/cancelled operations preserve the active session; reproduced state and evidence match; each journey has attributable runtime evidence.

## 8. Platform qualification and completion decision

**Owner:** verification lane; source remains frozen while evidence is collected.

- [ ] Run native, WebGPU and forced WebGL2 journeys with adapter/build/source identity recorded.
- [ ] Qualify logical-pixel/Retina geometry, input and text/modal ownership, keyboard/accessibility and offline recovery.
- [ ] Run contracted 36,000/100,000-tick endurance and memory checks. Measure p95 authority 5 ms, native/WebGPU 16.7 ms frames, WebGL2 Low 33.3 ms and 100 ms input response against the accepted performance contract.
- [ ] Reconcile every acceptance row to Current, Partial, Blocked or unknown evidence. Missing hosts or manual evidence do not become passing automated results.

**Acceptance:** current evidence for every required V2 surface. Only then mark the V2 completion program complete.

## 9. Resolve and accept V3 before implementation

V3 remains **Proposed / implementation not ready**. Strict imminent Food/energy recovery priority is the selected requirement; additional numerical and visibility choices still require written acceptance.

- [ ] Close the integrated review's remaining event-order defect: schema filters expiry before choosing the newest event ID, while scoring chooses identity before expiry. Freeze one no-resurrection order across both documents; derive moved/shortened-end fixtures and distinguish retained evidence from source eviction/refresh.
- [ ] Obtain fresh independent integrated review of all eight specifications.
- [ ] Re-derive current independent vectors, including full nonempty grammar, history/replay/collision and mechanics coverage. The existing wire subset is bound to its older specification baseline and cannot establish current full coverage.
- [ ] Close all six documented specification gaps: complete schemas, identities, scoring, diplomacy lifecycle, history bounds and independent vectors. Present concrete unresolved/new choices for acceptance before code.
- [ ] Write subsystem plans, then implement isolated versioned society/research, ecology and diplomacy/conflict vertical slices with creator/history/storage/experience integration. Repeat the qualification matrix for the new version.

**Acceptance:** decision-complete reviewed and accepted rules, exact caps/error order and independent corpus. Preserve V1/V2 interpretation and original vectors; no implicit V2-to-V3 identity conversion or read-time migration.

## Verification required after each source slice

Capture baseline and final source identities; run focused reference/lifecycle regressions first, then independent specification and code review. Correct attributable failures and repeat affected checks. The repository gate remains:

```sh
cargo fmt --all --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-targets
cargo build --release
cargo fmt --manifest-path crates/nyon-workshop-core/fuzz/Cargo.toml --all --check
cargo clippy --locked --manifest-path crates/nyon-workshop-core/fuzz/Cargo.toml --all-targets -- -D warnings
./tools/build-web.sh
cargo check --target wasm32-unknown-unknown --lib
cargo check -p nyon-workshop-core --target wasm32-unknown-unknown
cargo clippy --target wasm32-unknown-unknown --lib -- -D warnings
./tools/check-workshop.sh
python3 tools/validate-vertex-layout.py
python3 tools/living-v2-vectors.py verify
python3 tools/cross-build-fingerprint.py --skip-tests
```

Also run all 17 additive vector verifiers enumerated in the checked-in gate driver. The driver `.superpowers/sdd/2026-10-01-sandbox-completion-and-expansion/workspace-gate.py` records exact commands, source manifests and logs for a named checkpoint. If another writer changes inputs during qualification, preserve their edits, mark that checkpoint stale and repeat against a reconciled freeze. A documentation-only plan does not require repeating already successful compilation gates.
