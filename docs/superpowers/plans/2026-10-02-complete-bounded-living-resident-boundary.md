# Complete the bounded Living resident boundary

Status: approved for implementation by Donald, 2026-10-02 11:31 EDT.

## Summary
Replace the resident executor synchronous suffix with a complete resumable boundary. Preserve V2 gameplay, canonical bytes, identities, receipts, fault precedence and accepted-command order. Reuse reviewed forecast, intent-input, economic/fleet enumeration, dispatch, copy/hash, validator and state-retirement components. Current resident integration is the starting point. V3, new gameplay and unrelated creator preview optimization are excluded.

## Global constraints
Canonical dirty checkout; preserve every existing change. No new dependency, wire migration, branch, worktree, commit, push or PR. Sequential implementers with independent spec and quality review. Reconcile other active chat ownership before source writes. Core remains pure, forbid unsafe. Accepted V2 rules and independent vectors are authoritative. Every poll accepts 1..=1024 actual visits; invalid budgets consume zero work. Variable payload encoding chunks are at most 256 bytes/visit. One prospective boundary plus one retirement task; retirement drains before starting a new boundary. At most twenty outstanding requests/publications per update. One shared 1024-visit update budget covers retirement, boundary and cache preparation. Effective pace is actual publications over rolling one-second elapsed time, excluding zero-duration denominator entries. No resident synchronous helper calls. Record boundedness does not prove time or allocator guarantees.

## Task 1: Establish ownership and bounded retirement
Adopt other chat reviewed state helper and validator. Add catalog retirement and private handoff adapters for active copy/hash/validation/phase/forecast/intent/enumeration owners and indexes. Add opaque retirement jobs for completed publications, abandoned candidates, rejected preparations and active children. Count nested records/storage transitions; drain uniquely owned payloads before releasing final owner. Shared release must never race an ordinary final destructor; retain publication anchors and exact deferred owners where required. Runtime retains retirement across pause, leave, history navigation and replacement. Split this broad task into sequential catalog, child-adapter, and publication/history checkpoints with focused regressions and independent review before later source release.

## Task 2: Complete creator and intent preparation
Replace every delegated running creator operation, cascade and recovery scan with resumable work, preserving local IDs, operation/event/error order. Compose immutable intent inputs, economic/fleet enumeration and transactional reservation under remaining budget. Apply each proposal to private trial state, validate incrementally, accept or retire before next trial. Preserve actor/category order, fallback alternatives, proposal ordinals, reservations and identity consumption. Due intents precede dispatch; remove marker only when every nested path is counted. Split creator and reservation/composition checkpoints sequentially.

## Task 3: Complete validation, sealing and journals
Set prospective tick, incremental validate and hash. Build applied revision ordering, ordinary/event-facts/decision receipts, recovery/corridor bindings and journals incrementally, counting nested records and <=256-byte encoding chunks. Canonical bytes/domains unchanged. Preallocate sealed owners before commit; commit only freshness-checks and moves owners. Add terminal Ready, borrowed take_preparation returning existing opaque success/failure exactly once, private read-only sealed state/receipt access, and retirement handoffs. Incomplete extraction preserves owners; cancel prevents extraction; repeated terminal poll zero visits. Fault sticky and only authentic current failure token latches authority. Legacy progress variants remain source-compatible but complete executor never emits them.

## Task 4: Unify execution and replay
Authority step drains complete executor and uses same commit. Running archive/replay jobs poll it with remaining budget, preserving recorded command modes and original branch IDs. Retain synchronous algorithm only as test oracle. Explicit noninteractive helpers remain synchronous adapters. Resident never calls them. Keep premature/repeated extraction and stale tokens safe with complete owner handoff.

## Task 5: Finish resident scheduling and presentation
Prepare Chronicle, decisions, guidance and audio cues privately before commit. Publish prepared caches only on current successful authority commit, no fallible cache work afterward; stale/cancelled work emits no row/cue. One shared budget across cleanup/preparation/projection and all completed/new ticks, max twenty publications. Keep integer debt, 1/4/20 multipliers, discard excess, exactly-one Step. Pause/holds/suspension/history/replacement/leave cancel and clear timing/requests while accepted commands remain authority-owned. Runtime cleanup survives old Session replacement. Actual publication rolling-one-second pace only; zero elapsed entries do not enter denominator.

## Task 6: Verification and acceptance
Reference parity full state/all receipts/archive/queue/journals/high-water/faults at 1/64/1024 and mixed budgets. Due boundary, competing actors/fallback/capacity/hidden-stale intelligence, all creator cascades, repeated forks, arithmetic and receipt limits. Cancel/stale at every stage, including sealing/projection/retirement, accepted tails, stale failures. Shared nested visit accounting/zero terminal visits/bounded retained ownership/reclamation outside commit.
Run native fmt/strict workspace all-target/all-feature Clippy/workspace all-target tests/release, separate locked fuzz fmt/lint, paired web builds/root+core wasm checks/wasm Clippy/artifact checks, independent original/all additive vectors and vertex checks. Freeze source+artifact identities. Independent final complete-change review. Native/WebGPU/WebGL2 Step/Run/pause/save-leave/replacement/history journeys at declared capacity. Existing performance targets: total authority boundary p95 <=5ms, native/WebGPU frame p95 <=16.7ms, WebGL2 Low <=33.3ms, input p95 <=100ms. Separate total boundary/poll/frame/input timing; unmet/unverified acceptance remains Partial. No fabricated live, GPU, accessibility or endurance proof.

## Assumptions
Complete boundary selected by Donald. Existing V2 rules and independent vectors win. Historical receipts qualify only exact matching source. Future changes supersede checkpoints. Do not reset or revert others work; no completion claim until required gates and acceptance are verified.


## Current checkpoint and remaining execution plan

Updated 2026-10-02 at Donald's request to write the plan. The complete scope above remains binding. Overall status is **Partial**. This update records a continuation plan; it does not release another product writer or qualify the final executor.

Reviewed prerequisites are catalog retirement, copy/hash ownership, phase and Rules retirement, publication and archive retirement, bounded storage/insertion, receipt append, and lifecycle preparation. Task2C lifecycle passed independent review with one minor mixed-budget test correction; that correction passed its single scoped re-review with no remaining findings. Its current covering evidence is 17 lifecycle tests, 677 core tests across 27 suites, strict core Clippy and workspace formatting. The earlier 35-test wire run and all original checkpoint evidence retain their original source identities. These results do not qualify the remaining executor, runtime or current wasm artifacts.

The source and evidence ledger is `.superpowers/sdd/2026-10-02-complete-bounded-living-resident-boundary/progress.md`; public qualification is `docs/LIVING-BOUNDARY-EVIDENCE-2026-10-02.md`. Preserve exact first-touch snapshots, full diffs, raw logs and source identities. Reconcile live ownership before every implementation release. One product/build writer at a time; independent review follows each complete slice. A passing prerequisite is not permission to skip its owning parent or resident adoption.

### Checkpoint A: Complete recipe, repair and scheduled producers

Implement recipe first in `living/step_job.rs`, its parent in `living/production_job.rs`, and narrow synchronous compatibility/test reference handling in `living/rules.rs`. Extend `living/simulation.rs` only with the named cfg(test) capacity-propagation tests. Exact four-file baselines are already captured in `baseline-2d1`; verify they still match before writing.

Preserve the existing inline recipe owner and original production/Step Boxes. Prepare sorted reason output from the known reason count, acquire and commit reasons separately, and retire the original exhausted map before Ready. Preserve all checks before payment, duplicate-key replacement, malformed-input fault precedence and input/deposit/output application order. Recipe emits no events. Retain shared IO Arc handles through child cleanup; release or defer them through the existing typed lane.

Use closed copied recipe and production causes, with exact public diagnostic nesting. Production's reachable causes are Invariant, Arithmetic, KeyedOrder and explained-event Facts/Receipt TooManyEvents/Wire Json/Capacity. Preparation capacity remains a separate noncanonical latch. Route both child poll and borrowed extraction failures through the parent latch without advancing RecipeCommit or dropping the child. Do not classify a broad recursive fault envelope as scalar.

Preserve the old recipe implementation as an independent cfg(test) oracle: the existing Rules reference delegates to the same cursor and cannot establish independent parity. Compare full recipe outcomes and integrated production state/event/facts bytes. Derive work and retirement ledgers from stage/field ownership before measurements; prove them at budgets 1/64/1024/mixed and every cancellation cut.

Then implement repair as a separate reviewed slice: known-count fleet and hull outputs, retained selected rows, original exhausted hull iterators reclaimed before replacement, bounded world/catalog indexing and closed diagnostics. Preserve current V2 behavior. Finally migrate the remaining scheduled producers—arrival/path search, conflict/combat, occupation, production, ecology and their nested event/output preparation—using the adopted storage, append and retirement components. Release each producer together with its owning parent handoff and error path. No producer is complete while it hides source-sized work or destroys a delivered child synchronously.

Exit: every scheduled phase and nested child uses actual remaining budget; every success/failure/cancellation retains or retires its original owners; independent state/event/fault parity passes. General production is not qualified merely because recipe and repair delegation pass.

### Checkpoint B: Complete accepted creator operations

Replace all remaining delegated accepted running operations with cursors for the full 46-operation surface. Implement direct changes and additions before removal cascades, then recovery scans and complete revision handoff. Preserve accepted sequence, local identity consumption, operation order and event/error precedence.

Count repeated dependent-member and fixed-point scans. Preserve exact Reviewed/Listed/legacy cascade policies, duplicate handling, refund-before-cancellation order, split/merge/relocation semantics and operation-specific status refresh. Prepare recovery candidates from the original pre-boundary dormant state, then bind only qualifying final active actors and their minimum owned worlds. Retire rejected/abandoned partial revisions and temporary indexes through the same ownership route.

Exit: all accepted running operations are resumable, no delegated synchronous fallback remains, and cascade/recovery parity plus every-stage cancellation and stale-capture tests pass. Accepted queue tails remain intact.

### Checkpoint C: Compose transactional due intents before dispatch

Reuse the current forecast, immutable intent inputs, economic enumeration and fleet enumeration. Poll each child with the parent's remaining budget, including output assembly, ranking, replacement, trial validation and cleanup.

For each proposal, prepare a private trial candidate, apply the existing reservation policy, validate with the existing incremental validator, then accept or retire it before considering the next alternative. Preserve actor/category order, fallback alternatives, proposal ordinals, reservations, identity consumption and source/observation tick semantics. Retain failed trials and their temporary indexes; never start another trial while its retirement obligation remains.

Complete the entire due-intent stage before dispatch. Remove the due-intent synchronous marker only after all nested paths are counted. Test competing actors, hidden/stale observations, fallback reservations, capacity replacement and arithmetic failures against the independent reference.

Exit: due intents and dispatch share actual remaining work; complete output/decisions and rejected-trial retirement have exact parity at fixed and mixed budgets.

### Checkpoint D: Seal the boundary and preallocate publication

Set prospective tick, run the existing incremental state validator and hash the candidate. Preserve the reference order of applied-ID sorting, ordinary receipt, event facts, creator recovery, corridor checks, decisions and journal preparation. Keep ordinary applied IDs sorted while journals retain accepted command order.

Build each nested receipt/journal record incrementally. Encode variable payloads in chunks no larger than 256 bytes per encoding visit, retaining original canonical field order and hash domains. Preserve event-facts retained-prefix/omission behavior, payload-before-semantic fault precedence, recovery ordering and corridor witness order. Preallocate all publication owners before Ready.

Add terminal Ready and borrowed `take_preparation(&mut self)` once-only extraction. Incomplete, cancelled or stale extraction preserves owners and returns no usable token; sticky deterministic failures latch authority only through a current opaque failure token. Repeated terminal polls consume zero visits. Add read-only sealed state and three-receipt access on opaque successful preparations for private presentation work. Constructors and mutable fields remain private.

Exit: commit checks capture freshness and moves owners only. Full state, three receipts, queues, journals, allocator marks and faults match the independent oracle; no sealing or journal synchronous suffix remains.

### Checkpoint E: Use one executor for authority and replay

Make public authority step drain the complete executor and use the same commit route. Keep the old running algorithm exclusively as the independent test oracle; retain explicit synchronous compatibility adapters only for noninteractive callers.

Integrate every replay/history forward path: Advance and gaps, recorded Running and Paused boundaries, Fork, final validation/hash/integrity/encoding, comparison children, bookmarks and saved-present handling. Preserve recorded command modes, original branch identities, parent chains, high-water marks and historical sidecar-clearing behavior. Poll children with actual remaining budget and retire original output/child shells before successors.

Cover repeated forks, cancellation at every archive stage and the previously deferred authentic comparison-finalization failure. Avoid recursive fault cloning, whole-state comparison copies and final capacity-sized destruction on interactive paths.

Exit: authority and replay produce byte-identical archives and receipts through the same complete executor; all replay paths have bounded work and reclamation.

### Checkpoint F: Adopt durable runtime retirement and private presentation

Put the sole retirement owner in ClientRuntime outside SandboxSession. It must survive hidden screens, pause, leave, history navigation, replacement and absence of a resident Session. Include rejected incoming sessions/authorities, stale tokens, active child jobs and retired publications. Drain retirement before admitting another prospective boundary; do not accumulate a queue of cancelled full candidates.

Use one 1,024-visit budget across retirement, history/replay, boundary and presentation preparation per update, with at most twenty publications and twenty pending requests. Prepare Chronicle, decision rows, guidance and audio cues privately. Move prepared cache owners only after the authority capture fence succeeds; stale/cancelled work yields no row or cue. Count deep labels, nested comparisons and cache evictions.

Preserve integer timing at 1x/4x/20x, dropped excess debt and exactly-one-boundary Step. Pause, holds, suspension, history, replacement and leave cancel prospective work and clear timing/request debt while preserving accepted authority commands. Compute pace from actual publications over a rolling one-second elapsed-time window; exclude zero-duration samples from its denominator.

Exit: resident updates never call synchronous compatibility helpers, every lifecycle journey uses durable bounded cleanup, and publication/cue/cache freshness and timing tests pass.

### Checkpoint G: Freeze and qualify the complete candidate

Run focused parity/cancellation/ownership suites throughout the earlier checkpoints. After all slices pass review, read the complete final diff, fix findings, freeze source identities and execute the documented final gates sequentially with nightly-2026-09-01:

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
python3 tools/living-v2-vectors.py verify
```

Run all documented additive vector modes, `living_wire`, vertex-layout and artifact/fingerprint checks from the current acceptance ledger. Preserve the independent vector corpus; expected values cannot come from the implementation under test. Freeze paired WebGPU/WebGL2 artifact identities and verify source freshness. Any source change invalidates affected qualification and artifact freshness.

Exercise declared-capacity native, WebGPU and WebGL2 Step/Run, pause, save-and-leave, replacement and history journeys using isolated disposable storage. Record actual backend, quality, hardware, capacity and source/artifact identities. A build, skipped GPU test or host-store model is not live browser/native evidence.

Measure total authority-boundary cost across all polls and commit separately from individual polls, retirement, presentation, frames and input response. Required p95: boundary ≤5 ms; native/WebGPU frame ≤16.7 ms; WebGL2 Low frame ≤33.3 ms; input response ≤100 ms. Preserve raw samples and warm-up policy. Unmet or unmeasured targets remain Partial.

## Explicit conformance risks

Source inspection found existing shared-reference discrepancies around battery repair, overdue repair clocks, repair during combat, and returning freight after a lane-distance edit. These are source observations, not executed counterexamples. Add authentic public-genesis/command transition characterizations and controls in the relevant repair/arrival slices before using reference parity as a conformance claim.

Preserve existing V2 state/archive/reference behavior during this boundedness implementation; do not silently change gameplay, normalize clocks, rewrite rules or replace independent goldens. Existing rules and independent vectors remain authoritative. If authentic tests establish a rule/reference conflict, record the exact conflict and keep rule conformance Partial pending an explicit compatibility/version decision. Passing new-vs-old parity does not settle that conflict.

## Completion criteria

The plan is complete only when the full executor, replay and resident paths pass exact parity, cancellation/stale safety, bounded ownership/work checks, independent final review and fresh documented gates. Automated, live platform and measured performance evidence remain separate. Preserve all uncommitted work and evidence; do not commit, push, branch or open a PR.


## Revised approved contract — 2026-10-02 21:28 EDT

Donald explicitly approved four sequential linked plans: preparation and rule corrections; sealing and replay; resident runtime; qualification. This supersedes earlier preservation-only conformance rulings and their no-gameplay-correction wording for the named confirmed defects. Correct version2 in place, retain wire shape/version/identity formulas, preserve all existing files, accept safe replay rejection of affected old archives. Repair eligible hulls and batteries in one combined entity-ID order, block all repair at a world on any boundary resolving combat there including after retreat, advance unfinished due attempts from stored due+10, at most one attempt per asset/boundary. Freight return preserves original outbound duration captured before overwriting ticks. All other behavior remains exact-reference parity. No automatic migration or partial installation. Existing approved prerequisite work remains adopted after source identity verification; overall acceptance remains Partial. The four new linked plan documents carry the sequential work; no commits or parallel source writers.


### 2026-10-03 Task2E.3 adopted after complete and scoped review

Task2E.3 six existing diplomacy/order operations adopted on all68 `22b011614dd431cf41fcab74c33b18add55e5d7889943a9c1e7c93818ddb88a8`; five-file lease/all63 unleased/HEAD/index preserved. Sole complete independent review `358e6b7a5a871de909d93670656c90f130a56d05b3284aadf25635fdd3942147` SpecCompliant/QualityApproved found no product defect and one Minor artifact-count wording issue. Original-author reportv2 `b26eb5f9dd6c50b5a61fd884217ba3183e15fc08920079c84b3a4b6181624ea4`, append-only root erratum `ccd423d8eb30deb0b30edbb5e10b7d10488b6465a6b4cf0581510b7b3ccf32bb`, and fresh scoped review `901c603cccfe9295fe5dddaa932210f13be84f22801fc3f2d03e25db34884a89` closeM1 with no new findings. Correct count19 generated files per directory(8CSV+8TXT+3JSON); original20 wording/pins retained and superseded only as count metadata. All1,501 natural cuts/18real-child fault controls run fixed1/64/1024/mixed. Final exact-source829core/27summaries(481unit),35wire,strictcoreClippy/workspacefmt passed. Scope33of46 creator kinds; next sole slice is five cancellation/route/hazard operations after fresh ownership and first-touch. All full plans remain Partial; no workspace/fuzz/wasm/live/performance qualification claimed.
