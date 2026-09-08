# NYON Galaxy Workshop Qualification Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Repair independently found defects and produce honest source, artifact, runtime, accessibility, recovery, offline, performance, and cross-platform evidence for NYON Galaxy Workshop V1.

**Architecture:** Qualification consumes a feature-complete candidate without adding product scope. Automated gates establish source and artifact evidence; separate live runs exercise the same canonical pack, seed, recorded history, save/archive journey, accessibility paths, and digest across named native and browser hosts.

**Tech Stack:** Cargo gates, native macOS/Windows/Linux builds, WebGPU and WebGL2 bundles, browser automation where available, manual accessibility inspection, fault injection, and recorded p95 timing.

**Spec:** `docs/superpowers/specs/2026-09-02-nyon-v2-design.md`

## Checklist status (recorded 2026-09-08)

**Unlike the core and client plans, this one is genuinely unexecuted.** 0 of 30
boxes are checked *and* the Task 1 deliverable does not exist: there is no
`docs/qualification/` directory and no
`docs/qualification/2026-09-02-galaxy-workshop-evidence.md`. No live-matrix,
performance, or accessibility qualification artifact has ever been produced for
Workshop V1. Treat every runtime, accessibility, recovery, offline, performance
and cross-platform claim about Workshop V1 as unevidenced until this plan runs.

## Global Constraints

- Begin only after the core and client child plans are source-clean.
- Do not add features during qualification. A product gap returns to the owning child plan.
- Every defect gets a failing focused regression test before repair where the behavior is automatable.
- Do not equate compilation, bundle creation, screenshots, CI definitions, adapter skips, or one host with live matrix evidence.
- Record exact commit, artifact hash, platform, GPU/driver, browser, requested backend, selected backend, seed, catalog hash, archive hash, revision, and digest.
- Run the same Two-System Forge command history on every authority comparison.
- Keep imported content and archive payloads out of logs.
- Missing Windows or Linux hosts remain explicit blockers to runtime-qualified and release-qualified status.

---

### Task 1: Freeze the release candidate and run automated source gates

**Files:**
- Create: `docs/qualification/2026-09-02-galaxy-workshop-evidence.md`
- Modify: only files required by regression-tested findings

**Interfaces:**
- Consumes: intended clean feature diff and all child-plan test suites.
- Produces: candidate commit/hash inventory and complete automated gate results.

- [ ] **Step 1: Capture candidate provenance**

Record:

~~~bash
git rev-parse HEAD
git status --short --branch
git diff --check
rustc --version --verbose
cargo metadata --format-version 1 --no-deps
~~~

The evidence report lists unrelated untracked paths separately and excludes them from the release diff.

- [ ] **Step 2: Run the complete source gate from the canonical checkout**

~~~bash
cargo fmt --all --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-targets
cargo check -p nyon-workshop-core --target wasm32-unknown-unknown
git diff --check
~~~

Record each command, exit status, and test count. A rerun after repair replaces neither the failed observation nor its diagnosis; retain both in the evidence report.

- [ ] **Step 3: Run deterministic compatibility subsets**

Run the RulesV1 façade, campaign, scenario, shader, source-contract, pack, creator, simulation, Two-System Forge, history, archive, store, client, presentation, web, and accessibility suites individually. Record exact names and counts.

- [ ] **Step 4: Build and hash artifacts**

~~~bash
cargo build --release
./tools/build-web-webgpu.sh
./tools/build-web-webgl.sh
./tools/check-workshop.sh
shasum -a 256 dist/webgpu/nyon.js dist/webgpu/nyon_bg.wasm dist/webgl/nyon.js dist/webgl/nyon_bg.wasm
~~~

Label this `artifact-qualified` evidence only.

### Task 2: Perform independent domain, persistence, security, and UI reviews

**Files:**
- Modify: exact owning source and focused tests for accepted findings
- Update: `docs/qualification/2026-09-02-galaxy-workshop-evidence.md`

**Interfaces:**
- Consumes: frozen candidate diff, design, and child plans.
- Produces: finding ledger with severity, reproduction, test, repair commit, and rerun evidence.

- [ ] **Step 1: Review deterministic authority**

Inspect for unordered traversal, float or wall-clock state, unchecked arithmetic, partial mutation, inconsistent encoding, digest exclusions, local-reference type confusion, ID collision handling, phase-order drift, and RulesV1 coupling.

- [ ] **Step 2: Review parser and archive attack surfaces**

Inspect byte caps before parsing, duplicate keys, depth, allocation amplification, unknown fields, float handling, ID and name validation, reference graphs, hash domains, archive DAG cycles, path/URL/script absence, error redaction, and invalid-import atomicity.

- [ ] **Step 3: Review storage failure ordering**

Verify native temporary-file locality, file flush, read-back hash, pointer replacement, directory sync, prior-generation retention, and generation conflict. Verify IndexedDB schema, immutable generation keys, single-transaction ref updates, abort handling, quota handling, and unload non-success.

- [ ] **Step 4: Review UI and accessibility**

Exercise focus order, modal trap/restore, disabled state, branch-head versus cursor presentation, recovery choices, conflict status, keyboard-only tools, semantic names/values/actions, reduced motion, high contrast, and non-color status cues.

- [ ] **Step 5: Repair every reproducible finding**

For each finding:

1. Add the smallest failing regression test.
2. Run it and record the expected failure.
3. Implement the narrow repair.
4. Run the focused test.
5. Run all affected suites.
6. Repeat the full source and artifact gates.
7. Record the repair commit and evidence.

- [ ] **Step 6: Commit repaired findings**

Use one or more scoped commits whose messages reflect actual fixes. If the final review sweep is one coherent UI/accessibility batch, use:

~~~bash
git commit -m "fix(workshop): close acceptance findings"
~~~

Do not create an empty ceremonial commit.

### Task 3: Run native/browser storage, recovery, and offline acceptance

**Files:**
- Update: `docs/qualification/2026-09-02-galaxy-workshop-evidence.md`

**Interfaces:**
- Consumes: repaired candidate, valid pack, malformed pack corpus, valid archive, corrupt archive corpus, and fault-injected stores.
- Produces: durable-save, recovery, import/export, and network-disabled observations.

- [ ] **Step 1: Execute the canonical Two-System Forge journey**

Use one recorded creator history and exact genesis seed. Complete all 18 design steps and record catalog hash, revision graph summary, selected branch, tick, state digest, archive hash, and resulting alloy inventory.

- [ ] **Step 2: Repeat by keyboard-only interaction**

Perform every creator, timeline, branch, save, import/export, and recovery action without spatial canvas clicking. Record focus defects and semantic announcements as findings, not caveats.

- [ ] **Step 3: Verify native durability**

With a disposable application-data root:

- Save generation 1 and generation 2.
- Interrupt before pointer replacement and recover generation 2.
- Interrupt after replacement and recover generation 3.
- Corrupt the newest generation and offer the prior generation.
- Submit a stale generation and receive conflict without losing either side.
- Quit cleanly, relaunch, and `CONTINUE` the explicitly selected branch.

- [ ] **Step 4: Verify IndexedDB durability**

In both WebGPU and forced WebGL2 artifacts:

- Complete a normal transaction and reload.
- Abort before commit and retain the old ref.
- Deny quota and retain the active session.
- Simulate unavailable IndexedDB and expose a recoverable error.
- Corrupt the current archive and offer only the prior valid generation.
- Confirm unload without transaction completion is not reported as saved.

- [ ] **Step 5: Verify pack and archive safety**

Import the valid core pack and archive, then every invalid parser/archive corpus case. Each failure must identify an error code and field path where safe, preserve the active session, avoid raw-content logging, and leave stored generations unchanged.

- [ ] **Step 6: Repeat with networking disabled**

With all network interfaces or browser requests blocked, exercise new Workshop, continue, pack import/export, archive import/export, replay, branch switching, and Classic Sector. Any network request required for these paths is a release defect.

### Task 4: Measure deterministic and presentation performance

**Files:**
- Create or Modify: focused benchmark harnesses that do not alter production authority
- Update: `docs/qualification/2026-09-02-galaxy-workshop-evidence.md`

**Interfaces:**
- Consumes: declared-capacity deterministic fixture and Two-System Forge scene.
- Produces: raw samples, p50/p95/p99 summaries, host metadata, and pass/fail against fixed budgets.

- [ ] **Step 1: Build the maximum-capacity authority fixture**

Populate exactly 16 factions, 64 systems, 128 stars, 512 worlds, 256 lanes, 1,024 deposits, 2,048 industries, 2,048 routes, 4,096 shipments, and 128 hazards through validated state-construction fixtures. Verify its canonical hash before measurement.

- [ ] **Step 2: Measure authoritative steps**

Use a release build, discard the first 100 warmup steps, then record at least 10,000 steps per native host. p95 must be at or below 5 ms. Record CPU, power mode, build hash, sample count, and raw-result artifact.

- [ ] **Step 3: Measure native and WebGPU presentation**

At 1920 by 1080, run Two-System Forge for at least 120 measured seconds after 10 seconds warmup. Native and WebGPU p95 frame time must be at or below 16.7 ms. Record dropped frames and GPU/driver.

- [ ] **Step 4: Measure WebGL2 Low**

Force `?backend=webgl2`, use the same view and duration, and require p95 at or below 33.3 ms. Confirm selected backend is `WEBGL2 LOW` rather than inferred from the URL alone.

- [ ] **Step 5: Handle failures without data loss**

A budget miss is a release finding. Optimize only with unchanged core bytes and digest; rerun determinism, archive, presentation-order, and recovery gates after every performance repair.

### Task 5: Complete the live platform matrix

**Files:**
- Update: `docs/qualification/2026-09-02-galaxy-workshop-evidence.md`

**Interfaces:**
- Consumes: identical catalog, seed, creator history, expected final revision, and expected digest.
- Produces: one complete observation per required native/browser/backend row.

- [ ] **Step 1: Qualify current macOS**

Run:

- Native Metal.
- Safari natural selection.
- Safari forced WebGL2.
- Chrome natural selection.
- Chrome forced WebGL2.

- [ ] **Step 2: Qualify Windows 11**

Run:

- Native DX12.
- Edge natural selection.
- Edge forced WebGL2.
- Chrome natural selection.
- Chrome forced WebGL2.

- [ ] **Step 3: Qualify Ubuntu 24.04**

Run:

- Native Vulkan.
- Native forced GL fallback.
- Firefox natural selection.
- Firefox forced WebGL2.

- [ ] **Step 4: Record every observation**

For each row record:

- NYON commit and artifact SHA-256.
- OS and kernel/build version.
- Browser version.
- GPU and driver.
- Requested and actual selected backend.
- Core catalog hash and genesis seed.
- Imported/exported archive hash.
- Final revision and state digest.
- Save/reload and recovery result.
- Natural or forced fallback result.
- Keyboard-only result.
- Network-disabled result.
- p95 timing and raw-sample path.

- [ ] **Step 5: Compare authority outputs**

The catalog hash, seed, recorded history, final authoritative revision, and state digest must match across every row. Presentation images and frame timings may differ; authority bytes may not.

### Task 6: Assign status and publish only proved evidence

**Files:**
- Finalize: `docs/qualification/2026-09-02-galaxy-workshop-evidence.md`

**Interfaces:**
- Consumes: all automated, review, recovery, offline, performance, and live observations.
- Produces: one honest completion label and explicit pending rows.

- [ ] **Step 1: Classify the candidate**

Use exactly:

1. `Implemented` when source behavior and focused/full tests pass.
2. `Artifact-qualified` when native release and both browser artifacts also pass static gates.
3. `Runtime-qualified` when every live matrix row and Two-System Forge pass.
4. `Release-qualified` when runtime, accessibility, performance, recovery, offline, documentation, provenance, and clean diff all pass.

- [ ] **Step 2: Leave missing evidence pending**

An unavailable Windows/Linux host, failed browser path, adapter skip, unexecuted keyboard path, missing recovery injection, performance miss, or open review finding is a pending or failed row. It is never `complete with blockers`.

- [ ] **Step 3: Re-run final gates on the exact evidence commit parent**

~~~bash
cargo fmt --all --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-targets
cargo check -p nyon-workshop-core --target wasm32-unknown-unknown
./tools/build-web-webgpu.sh
./tools/build-web-webgl.sh
./tools/check-workshop.sh
git diff --check
~~~

- [ ] **Step 4: Commit evidence only if observations are populated**

~~~bash
git add docs/qualification/2026-09-02-galaxy-workshop-evidence.md
git commit -m "docs: record Galaxy Workshop qualification"
~~~

If host rows remain unavailable, the report may be committed as an artifact-qualified checkpoint only when it labels those rows pending and makes no release claim.

## Qualification Completion

Release qualification requires every automated gate, independent review, durability and recovery scenario, offline path, accessibility path, performance budget, and named live matrix row to pass on the recorded candidate. Anything less receives the strongest lower evidence label it actually proves.
