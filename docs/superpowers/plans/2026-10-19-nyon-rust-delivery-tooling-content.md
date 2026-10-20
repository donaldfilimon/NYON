# NYON Rust Platform Delivery, Tooling and Content Implementation Plan

> **For agentic workers:** Use superpowers:executing-plans task-by-task. The user's no-test/no-commit/canonical-checkout constraints override generic execution steps. Do not dispatch parallel writers.

**Goal:** Integrate recovered gameplay with retained Rust hosts and tooling, then qualify the complete game on required platforms.

**Architecture:** Keep winit/wgpu, existing Tokio native acquisition, wasm spawn_local generations, Rust fontdue/resvg atlas builder and existing storage adapters. Integrate missing semantics without introducing a second host or renderer. Content is versioned input to the same authority.

**Tech Stack:** Rust stable1.99.1, edition2024, existing Cargo workspace/dependencies and retained Rust platform.

**Spec:** `docs/superpowers/specs/2026-10-19-nyon-rust-delivery-tooling-content-design.md`. Read the spec and pinned historical input sections before each task.

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

### Task 1: Runtime and store integration

**Files:** src/app/; src/workshop/store/; src/platform/; core recovery projections. `core` means `crates/nyon-workshop-core/src/`; exact nested file boundaries follow the responsibility names above. Register every new core module in `tests/living_wire.rs` and its declaration count.

**Interfaces:** Copied projections and typed authority/store outcomes; no gameplay commands originate in advisory/editor code.

- [ ] Read the spec's pinned source inputs and compare existing dirty work before editing these owned files.
- [ ] Write regression fixtures for: New game/production/diplomacy/combat/save/quit/Continue, failed load/save, stale completion, offline and abandoned mutation. Compile them without executing during the pause.
- [ ] Wire recovered authority into retained application lifecycle and new-version storage. Preserve policy ownership, recovery behavior and generation checks.
- [ ] Compile affected Rust targets with explicit +1.99.1 and --locked; use ignored root target directories for separate tools. Capture exact exit status and warnings. Do not invoke test/CI wrappers.
- [ ] Review the full task diff for wire/ownership/policy errors; record Current/Partial/Proposed evidence and unresolved findings. No commit.
- [ ] Deferred acceptance, after Donald lifts the pause: execute the named regression scenarios and appropriate repository/platform gates. Compilation alone does not check this box.

### Task 2: Atlas and platform contracts

**Files:** tools/ui-atlas/; assets/ui/ only when an approved enhancement requires changes; renderer contracts. `core` means `crates/nyon-workshop-core/src/`; exact nested file boundaries follow the responsibility names above. Register every new core module in `tests/living_wire.rs` and its declaration count.

**Interfaces:** Input manifest + authenticated source closure; evidence keys include host/target/backend/viewport/scale/profile.

- [ ] Read the spec's pinned source inputs and compare existing dirty work before editing these owned files.
- [ ] Write regression fixtures for: Missing/duplicate hardware rows, candidate-only outcome, metric/pixel checks, 36-byte packing and logical-pixel hit testing. Compile them without executing during the pause.
- [ ] Retain current Rust atlas pipeline. For any new receipt/candidate enhancement specify source authority, witness identity and typed candidate/release outcomes before code; preserve assets until independently qualified.
- [ ] Compile affected Rust targets with explicit +1.99.1 and --locked; use ignored root target directories for separate tools. Capture exact exit status and warnings. Do not invoke test/CI wrappers.
- [ ] Review the full task diff for wire/ownership/policy errors; record Current/Partial/Proposed evidence and unresolved findings. No commit.
- [ ] Deferred acceptance, after Donald lifts the pause: execute the named regression scenarios and appropriate repository/platform gates. Compilation alone does not check this box.

### Task 3: Content and acceptance ledger

**Files:** docs/PLAYER-MANUAL.md; versioned content; platform qualification records. `core` means `crates/nyon-workshop-core/src/`; exact nested file boundaries follow the responsibility names above. Register every new core module in `tests/living_wire.rs` and its declaration count.

**Interfaces:** No content bypasses pure command admission; rollback disables new-version entry without rewriting saves.

- [ ] Read the spec's pinned source inputs and compare existing dirty work before editing these owned files.
- [ ] Write regression fixtures for: Native/browser live accessibility, audio/input, device loss, fallback, real IndexedDB and named-hardware full-hour/frame/input timings. Compile them without executing during the pause.
- [ ] Deliver templates/tutorials/demos/difficulty/help/campaigns after their mechanics exist; record each required journey/platform/performance row independently.
- [ ] Compile affected Rust targets with explicit +1.99.1 and --locked; use ignored root target directories for separate tools. Capture exact exit status and warnings. Do not invoke test/CI wrappers.
- [ ] Review the full task diff for wire/ownership/policy errors; record Current/Partial/Proposed evidence and unresolved findings. No commit.
- [ ] Deferred acceptance, after Donald lifts the pause: execute the named regression scenarios and appropriate repository/platform gates. Compilation alone does not check this box.

## Compilation commands

Use `cargo +1.99.1 check --locked --workspace --all-targets --all-features` for native root/core changes. For each affected browser backend use `cargo +1.99.1 check --locked --target wasm32-unknown-unknown --lib --no-default-features --features webgpu-backend` and the same command with `webgl-backend`. Also compile core tests for wasm with `cargo +1.99.1 check --locked -p nyon-workshop-core --target wasm32-unknown-unknown --all-targets` when appropriate. Separate tools use explicit `--manifest-path` and `--target-dir target/rust-recovery/<tool>`.

## Plan self-review and handoff

Reviewed against the paired Rust design and preserved input revisions. Known source-review gaps are tasks/prerequisites, not waived acceptance. These plans preserve the complete requested game; later strategic subsystems explicitly require their own design/interface freeze rather than implementation guesses. No task authorizes tests during the pause, changing CI/deploy configuration, removing Zig source or claiming a completed full game.
