# NYON Rust Rich Cities and Strategic Systems Implementation Plan

> **For agentic workers:** Use superpowers:executing-plans task-by-task. The user's no-test/no-commit/canonical-checkout constraints override generic execution steps. Do not dispatch parallel writers.

**Goal:** Deliver a productive ruler capital, city economy and strategic expansion under one deterministic galaxy clock.

**Architecture:** Use rich-city as the sole city base and a new composed identity. Keep economy, combat, strategic hostility and durable graphs under one authority transaction. UI consumes copied stock/workforce/service/custody projections.

**Tech Stack:** Rust stable1.99.1, edition2024, existing Cargo workspace/dependencies and retained Rust platform.

**Spec:** `docs/superpowers/specs/2026-10-19-nyon-rust-rich-cities-strategy-design.md`. Read the spec and pinned historical input sections before each task.

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

### Task 1: Freeze city composition

**Files:** Rust rich-city, tactical and durable design manifests; core recovery/city/types.rs. `core` means `crates/nyon-workshop-core/src/`; exact nested file boundaries follow the responsibility names above. Register every new core module in `tests/living_wire.rs` and its declaration count.

**Interfaces:** Qualified city/structure IDs, u8 crew, typed city effects/results/grants and common phase registry.

- [ ] Read the spec's pinned source inputs and compare existing dirty work before editing these owned files.
- [ ] Write regression fixtures for: Fourteen capital default bytes, accepted tags0..17 custody vectors, roads/solar/captured freight contradictions. Compile them without executing during the pause.
- [ ] Repin the corrected city base; reconcile default bytes, stock/cargo/reservation custody and unique emitters in one composition before writing phases.
- [ ] Compile affected Rust targets with explicit +1.99.1 and --locked; use ignored root target directories for separate tools. Capture exact exit status and warnings. Do not invoke test/CI wrappers.
- [ ] Review the full task diff for wire/ownership/policy errors; record Current/Partial/Proposed evidence and unresolved findings. No commit.
- [ ] Deferred acceptance, after Donald lifts the pause: execute the named regression scenarios and appropriate repository/platform gates. Compilation alone does not check this box.

### Task 2: City loop and capital

**Files:** core recovery/city/{command,phases,genesis,wire}.rs; tests/recovery_city.rs. `core` means `crates/nyon-workshop-core/src/`; exact nested file boundaries follow the responsibility names above. Register every new core module in `tests/living_wire.rs` and its declaration count.

**Interfaces:** Pure city command/phase jobs share coordinator budget and publish only through the composed transaction.

- [ ] Read the spec's pinned source inputs and compare existing dirty work before editing these owned files.
- [ ] Write regression fixtures for: Housing/workforce deficits, power/water starvation, ruined/captured services, growth cap, paid roads and capital byte recomputation. Compile them without executing during the pause.
- [ ] Implement eight-commodity city loops and the receipted capital using exact pinned recipes and next-hour activation; preserve old rules identities.
- [ ] Compile affected Rust targets with explicit +1.99.1 and --locked; use ignored root target directories for separate tools. Capture exact exit status and warnings. Do not invoke test/CI wrappers.
- [ ] Review the full task diff for wire/ownership/policy errors; record Current/Partial/Proposed evidence and unresolved findings. No commit.
- [ ] Deferred acceptance, after Donald lifts the pause: execute the named regression scenarios and appropriate repository/platform gates. Compilation alone does not check this box.

### Task 3: Strategy and playable UI

**Files:** core recovery/strategy/; src/app/ and src/ui/ city integration. `core` means `crates/nyon-workshop-core/src/`; exact nested file boundaries follow the responsibility names above. Register every new core module in `tests/living_wire.rs` and its declaration count.

**Interfaces:** Strategic hostility producer emits the common bounded input; application owns navigation and session policy.

- [ ] Read the spec's pinned source inputs and compare existing dirty work before editing these owned files.
- [ ] Write regression fixtures for: Ruler expansion/diplomacy journey, scarce resources, stale UI actions, canonical saves and cross-world simultaneous publication. Compile them without executing during the pause.
- [ ] Write independent designs/plans for logistics/research/ecology/diplomacy/fleets before implementing each; expose copied projections and typed rejections through retained Rust UI.
- [ ] Compile affected Rust targets with explicit +1.99.1 and --locked; use ignored root target directories for separate tools. Capture exact exit status and warnings. Do not invoke test/CI wrappers.
- [ ] Review the full task diff for wire/ownership/policy errors; record Current/Partial/Proposed evidence and unresolved findings. No commit.
- [ ] Deferred acceptance, after Donald lifts the pause: execute the named regression scenarios and appropriate repository/platform gates. Compilation alone does not check this box.

## Compilation commands

Use `cargo +1.99.1 check --locked --workspace --all-targets --all-features` for native root/core changes. For each affected browser backend use `cargo +1.99.1 check --locked --target wasm32-unknown-unknown --lib --no-default-features --features webgpu-backend` and the same command with `webgl-backend`. Also compile core tests for wasm with `cargo +1.99.1 check --locked -p nyon-workshop-core --target wasm32-unknown-unknown --all-targets` when appropriate. Separate tools use explicit `--manifest-path` and `--target-dir target/rust-recovery/<tool>`.

## Plan self-review and handoff

Reviewed against the paired Rust design and preserved input revisions. Known source-review gaps are tasks/prerequisites, not waived acceptance. These plans preserve the complete requested game; later strategic subsystems explicitly require their own design/interface freeze rather than implementation guesses. No task authorizes tests during the pause, changing CI/deploy configuration, removing Zig source or claiming a completed full game.
