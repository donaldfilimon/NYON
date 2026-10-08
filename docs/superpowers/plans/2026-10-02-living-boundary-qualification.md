# Living Boundary Qualification Implementation Plan

> **For agentic workers:** Use superpowers:subagent-driven-development, one fresh implementer per task and independent task review before the next source release. No commits or worktrees: Donald explicitly selected the canonical dirty checkout.

**Goal:** Qualify the exact complete source and paired artifacts with automated, live and measured evidence.

**Architecture:** Freeze the final candidate after review. Keep each proof class separate and preserve raw logs/samples with source/backend/hardware identities.

**Tech Stack:** Rust edition 2024, nightly-2026-09-01; existing pure nyon-workshop-core and native/wasm resident runtime.

**Spec:** Donald's approved revised boundary requirements of 2026-10-02 21:28 EDT, recorded in `2026-10-02-complete-bounded-living-resident-boundary.md` under Revised approved contract.

## Global Constraints

Preserve every existing uncommitted change. No dependency, schema migration, version/identity-formula change, branch, worktree, commit, push or PR. One product/build writer at a time. Verify adopted source identities and exact first-touch snapshots before release. Poll budgets are 1..=1024 actual visits; invalid budgets use zero work. Variable encoding visits emit at most 256 bytes. Reclamation retains original nested owners. Resident updates share 1024 visits and publish at most twenty boundaries; pending requests are capped at twenty. Complete corrected version-2 rules in place; preserve old files and safely reject affected archives that no longer replay. Record parity exceptions only for approved rule corrections. Historical evidence does not qualify changed source. Automated, live and measured performance evidence remain separate.

## Review Focus

- Stale success/failure must not mutate authority or publish presentation.
- Failed or cancelled partial work must retain original nested allocations until counted retirement.
- Children must charge failed work once and use only the parent's actual remaining budget.
- Corrected rules must have independently derived public-path expectations, not shared-cursor parity alone.
- Historical replay rejection must leave files, active sessions and accepted authority commands intact.

## Execution evidence

The existing complete-boundary SDD ledger remains the recovery map. Keep reviewed baselines, raw gates and complete task diffs; do not repeat completed prerequisite work. Each task follows focused regression/implementation/frozen covering gates/self-review/fresh independent review. Preserve development failures without claiming behavioral RED from compilation errors. Final whole-candidate review and fresh platform gates are mandatory.

### Task 1: Full regression and final review

**Files/responsibility:** Test core/root parity, ownership, correction, archive and resident suites; preserve independent vectors.

**Interfaces:** Budgets1/64/1024/mixed; every stage cancellation/stale; full state/three receipts/archive/queue/journal/allocator/fault comparisons.

- [ ] Capture ownership and first-touch identity scope; add focused named regression assertions for the stated behaviors.
- [ ] Implement the smallest complete child-and-parent slice: Cover competing actors, fallbacks, replacement, hidden observations, creator cascades, repeated forks, arithmetic/receipt limits and safe affected-old-archive rejection. Final complete diff review, covering fixes and one scoped fix review; freeze source and require stable identities across final gates.
- [ ] Run the focused tests, preserve actual failures, then freeze and run covering documented gates.
- [ ] Self-review full first-touch diff and exhaustive owner/count map; write report/logs/identities and stop writes for fresh independent review.

### Task 2: Documented native/fuzz/wasm/vector/artifact gates

**Files/responsibility:** Use repository AGENTS exact commands and existing gate ledger scripts.

**Interfaces:** Both backend builds and fresh artifacts; unchanged identity corpus verified before any vector edit; independent corrected-transition expectations.

- [ ] Capture ownership and first-touch identity scope; add focused named regression assertions for the stated behaviors.
- [ ] Implement the smallest complete child-and-parent slice: Run native fmt/strict workspace all-target/all-feature Clippy/workspace all-target tests/release; separate locked fuzz fmt/Clippy; paired web builds/root+core wasm checks/wasm Clippy/check-workshop; original/additive vectors, wire, vertex/fingerprint/artifacts. Retain exact exit/count/log/identity records; changed source invalidates affected evidence.
- [ ] Run the focused tests, preserve actual failures, then freeze and run covering documented gates.
- [ ] Self-review full first-touch diff and exhaustive owner/count map; write report/logs/identities and stop writes for fresh independent review.

### Task 3: Live capacity journeys and performance

**Files/responsibility:** Use isolated disposable native/browser storage; do not open or rewrite Donald saved profiles.

**Interfaces:** Declared capacity native/WebGPU/WebGL2 Step/Run/pause/save-leave/replacement/history; raw warmup/samples/backend/hardware/quality/source/artifact identity.

- [ ] Capture ownership and first-touch identity scope; add focused named regression assertions for the stated behaviors.
- [ ] Implement the smallest complete child-and-parent slice: Measure total boundary across all polls+commit separately from maximum poll, reclamation/presentation/frame/input. p95 boundary<=5ms, native/WebGPU frame<=16.7ms, WebGL2 Low<=33.3ms, input<=100ms. Include safe affected-history rejection. Compilation/skipped GPU/host store model is not live proof. Missing/unmet evidence remains Partial.
- [ ] Run the focused tests, preserve actual failures, then freeze and run covering documented gates.
- [ ] Self-review full first-touch diff and exhaustive owner/count map; write report/logs/identities and stop writes for fresh independent review.


## Exact final commands

Execute sequentially after all source writers stop. Capture source/artifact manifests and raw logs. --locked preserves the existing lockfiles. Expected every command exit0; test summaries must be clean with no unexplained skips or warnings. A skipped GPU test is not parity proof.

```sh
cargo fmt --all --check
cargo clippy --locked --workspace --all-targets --all-features -- -D warnings
cargo test --locked --workspace --all-targets
cargo build --locked --release
cargo fmt --manifest-path crates/nyon-workshop-core/fuzz/Cargo.toml --all --check
cargo clippy --locked --manifest-path crates/nyon-workshop-core/fuzz/Cargo.toml --all-targets -- -D warnings
./tools/build-web.sh
cargo check --locked --target wasm32-unknown-unknown --lib
cargo check --locked -p nyon-workshop-core --target wasm32-unknown-unknown
cargo clippy --locked --target wasm32-unknown-unknown --lib -- -D warnings
./tools/check-workshop.sh
cargo test --locked -p nyon-workshop-core --test living_wire
python3 tools/validate-vertex-layout.py
python3 tools/living-v2-vectors.py verify
python3 tools/living-v2-additions-vectors.py verify
python3 tools/living-v2-additions-vectors.py verify-creator
python3 tools/living-v2-additions-vectors.py verify-decision-route
python3 tools/living-v2-additions-vectors.py verify-decision-route-adjust
python3 tools/living-v2-additions-vectors.py verify-decision-scrap
python3 tools/living-v2-additions-vectors.py verify-empty-state
python3 tools/living-v2-additions-vectors.py verify-event-facts
python3 tools/living-v2-additions-vectors.py verify-genesis-forks
python3 tools/living-v2-additions-vectors.py verify-notebook
python3 tools/living-v2-additions-vectors.py verify-notebook-corridor-attempts
python3 tools/living-v2-additions-vectors.py verify-notebook-evidence
python3 tools/living-v2-additions-vectors.py verify-observations
python3 tools/living-v2-additions-vectors.py verify-omitted-facts
python3 tools/living-v2-additions-vectors.py verify-production-facts
python3 tools/living-v2-additions-vectors.py verify-shipments
python3 tools/living-v2-additions-vectors.py verify-trade-facts
python3 tools/living-v2-additions-vectors.py verify-trade-facts-unknown
python3 tools/cross-build-fingerprint.py --skip-tests
```
