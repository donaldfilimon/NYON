# NYON Rust Tactical Ground Combat Implementation Plan

> **For agentic workers:** Use superpowers:executing-plans task-by-task. The user's no-test/no-commit/canonical-checkout constraints override generic execution steps. Do not dispatch parallel writers.

**Goal:** Implement deterministic paid troops, cube-seam movement, simultaneous combat, retreat and occupation integrated with cities.

**Architecture:** Run four internal tactical rounds inside the common authoritative hour. Freeze movement/fire snapshots, apply summed damage, and publish city/combat effects with the same global descriptor. Exact cube graph distance and transported cardinal rays serve different rules.

**Tech Stack:** Rust stable1.99.1, edition2024, existing Cargo workspace/dependencies and retained Rust platform.

**Spec:** `docs/superpowers/specs/2026-10-19-nyon-rust-tactical-ground-combat-design.md`. Read the spec and pinned historical input sections before each task.

## Global constraints

- Rust stable 1.99.1, edition 2024; preserve `forbid(unsafe_code)` and pure-core dependencies.
- Existing source, dirty work, save identities, independent fixtures and historical evidence remain intact.
- Compilation only while Donald's test/CI pause applies. Test scenarios below are future acceptance, not executed or green.
- No commits, pushes, deployment, new production dependencies, automatic profile migration or Zig implementation work.
- Current evidence is in `docs/qualification/2026-10-19-rust-stable-compilation.md`; source revisions are pinned in `docs/qualification/2026-10-19-rust-recovery-inputs.md`.

## Review focus

- Preserve active state across rejected/canceled/stale work; no partial publication.
- Treat malformed/oversized input as errors before unbounded allocation.
- Keep canonical bytes and old save identities stable; no replacement goldens.
- Meter cleanup, hashing and publication preparation with the common allowance.
- Distinguish compilation, future behavioral tests and actual native/browser evidence.

### Task 1: Freeze composition and feasibility witnesses

**Files:** Rust tactical/city composition docs; core recovery/tactical/{types,distance}.rs. `core` means `crates/nyon-workshop-core/src/`; exact nested file boundaries follow the responsibility names above. Register every new core module in `tests/living_wire.rs` and its declaration count.

**Interfaces:** Typed UnitId/StructureId, qualified targets, copied grants/results and common phases.

- [ ] Read the spec's pinned source inputs and compare existing dirty work before editing these owned files.
- [ ] Write regression fixtures for: Exact-distance independent reference, native/wasm layout/largest encoding, maximum-load complete-hour measurement. Compile them without executing during the pause.
- [ ] Reconcile CCM01..09 author responses with the corrected city base and preserve open CCM06/07. Define the exact class table and codec by source section/hash.
- [ ] Compile affected Rust targets with explicit +1.99.1 and --locked; use ignored root target directories for separate tools. Capture exact exit status and warnings. Do not invoke test/CI wrappers.
- [ ] Review the full task diff for wire/ownership/policy errors; record Current/Partial/Proposed evidence and unresolved findings. No commit.
- [ ] Deferred acceptance, after Donald lifts the pause: execute the named regression scenarios and appropriate repository/platform gates. Compilation alone does not check this box.

### Task 2: Movement and simultaneous resolution

**Files:** core recovery/tactical/{movement,fire,round,wire}.rs; tests/recovery_tactical.rs. `core` means `crates/nyon-workshop-core/src/`; exact nested file boundaries follow the responsibility names above. Register every new core module in `tests/living_wire.rs` and its declaration count.

**Interfaces:** One tactical phase job consumes city grants/hostility and emits the sole typed result stream.

- [ ] Read the spec's pinned source inputs and compare existing dirty work before editing these owned files.
- [ ] Write regression fixtures for: Seam rays, occupied-before-credit, contention, mutual kill, stale target, abort/cancel and repeatable bytes. Compile them without executing during the pause.
- [ ] Implement four fixed rounds, snapshots, cardinal rays and summed effects under the shared allowance. Keep all mutations private until global publication.
- [ ] Compile affected Rust targets with explicit +1.99.1 and --locked; use ignored root target directories for separate tools. Capture exact exit status and warnings. Do not invoke test/CI wrappers.
- [ ] Review the full task diff for wire/ownership/policy errors; record Current/Partial/Proposed evidence and unresolved findings. No commit.
- [ ] Deferred acceptance, after Donald lifts the pause: execute the named regression scenarios and appropriate repository/platform gates. Compilation alone does not check this box.

### Task 3: City integration and ruler controls

**Files:** core recovery/city/; src/ui/ and application tactical controls. `core` means `crates/nyon-workshop-core/src/`; exact nested file boundaries follow the responsibility names above. Register every new core module in `tests/living_wire.rs` and its declaration count.

**Interfaces:** City phase35 alone resolves capture; tactical phase12 stages only.

- [ ] Read the spec's pinned source inputs and compare existing dirty work before editing these owned files.
- [ ] Write regression fixtures for: Paid troop journey through movement/fire/retreat/occupation and exact Continue; corrupt input and stale actions preserve active state. Compile them without executing during the pause.
- [ ] Route paid training, reserve/capture effects and final occupation through their sole city owner; expose orders, range overlays and typed failure reasons.
- [ ] Compile affected Rust targets with explicit +1.99.1 and --locked; use ignored root target directories for separate tools. Capture exact exit status and warnings. Do not invoke test/CI wrappers.
- [ ] Review the full task diff for wire/ownership/policy errors; record Current/Partial/Proposed evidence and unresolved findings. No commit.
- [ ] Deferred acceptance, after Donald lifts the pause: execute the named regression scenarios and appropriate repository/platform gates. Compilation alone does not check this box.

## Compilation commands

Use `cargo +1.99.1 check --locked --workspace --all-targets --all-features` for native root/core changes. For each affected browser backend use `cargo +1.99.1 check --locked --target wasm32-unknown-unknown --lib --no-default-features --features webgpu-backend` and the same command with `webgl-backend`. Also compile core tests for wasm with `cargo +1.99.1 check --locked -p nyon-workshop-core --target wasm32-unknown-unknown --all-targets` when appropriate. Separate tools use explicit `--manifest-path` and `--target-dir target/rust-recovery/<tool>`.

## Plan self-review and handoff

Reviewed against the paired Rust design and preserved input revisions. Known source-review gaps are tasks/prerequisites, not waived acceptance. These plans preserve the complete requested game; later strategic subsystems explicitly require their own design/interface freeze rather than implementation guesses. No task authorizes tests during the pause, changing CI/deploy configuration, removing Zig source or claiming a completed full game.
