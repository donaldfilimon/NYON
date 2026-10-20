# NYON Rust Full-scale Authority and Durable Sessions Implementation Plan

> **For agentic workers:** Use superpowers:executing-plans task-by-task. The user's no-test/no-commit/canonical-checkout constraints override generic execution steps. Do not dispatch parallel writers.

**Goal:** Define one Rust authority/session publication domain with bounded work and durable history, preserving old profiles.

**Architecture:** Pure authority and pure session graph share value contracts through an integration bridge; neither imports a platform store. One selected pair publishes the authority root and graph prefix together. Native and browser brokers own I/O and report values tied to owner/generation/request tickets.

**Tech Stack:** Rust stable1.99.1, edition2024, existing Cargo workspace/dependencies and retained Rust platform.

**Spec:** `docs/superpowers/specs/2026-10-19-nyon-rust-authority-durable-sessions-design.md`. Read the spec and pinned historical input sections before each task.

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

### Task 1: Freeze shared contract

**Files:** core recovery/session_types.rs; Rust authority/session design and input receipts. `core` means `crates/nyon-workshop-core/src/`; exact nested file boundaries follow the responsibility names above. Register every new core module in `tests/living_wire.rs` and its declaration count.

**Interfaces:** Only value handles cross owners; PairController owns authority/graph visibility.

- [ ] Read the spec's pinned source inputs and compare existing dirty work before editing these owned files.
- [ ] Write regression fixtures for: Cross-owner ticket, stale offer, full-graph replay, cold tick-zero and late reply lifetime cases. Compile them without executing during the pause.
- [ ] Adopt all five source documents into one precedence table; bind exact hashes and identify superseded clauses. Translate owner-bound tickets, pin/request dispositions and purpose grammar without weakening them.
- [ ] Compile affected Rust targets with explicit +1.99.1 and --locked; use ignored root target directories for separate tools. Capture exact exit status and warnings. Do not invoke test/CI wrappers.
- [ ] Review the full task diff for wire/ownership/policy errors; record Current/Partial/Proposed evidence and unresolved findings. No commit.
- [ ] Deferred acceptance, after Donald lifts the pause: execute the named regression scenarios and appropriate repository/platform gates. Compilation alone does not check this box.

### Task 2: Concrete storage and history

**Files:** core recovery/fullscale/; core recovery/durable/. `core` means `crates/nyon-workshop-core/src/`; exact nested file boundaries follow the responsibility names above. Register every new core module in `tests/living_wire.rs` and its declaration count.

**Interfaces:** Prepared roots and authenticated prefix windows are owned Rust values with explicit retirement cursors.

- [ ] Read the spec's pinned source inputs and compare existing dirty work before editing these owned files.
- [ ] Write regression fixtures for: Allocation failure at every cut, overlapping COW lifetimes, partial suffix, canceled cleanup and window exhaustion. Compile them without executing during the pause.
- [ ] Implement checked typed page/index storage and bounded transcript/provider jobs; measure actual layouts on native/wasm before adopting the storage profile.
- [ ] Compile affected Rust targets with explicit +1.99.1 and --locked; use ignored root target directories for separate tools. Capture exact exit status and warnings. Do not invoke test/CI wrappers.
- [ ] Review the full task diff for wire/ownership/policy errors; record Current/Partial/Proposed evidence and unresolved findings. No commit.
- [ ] Deferred acceptance, after Donald lifts the pause: execute the named regression scenarios and appropriate repository/platform gates. Compilation alone does not check this box.

### Task 3: Paired publication and durable adapters

**Files:** core recovery/session_bridge.rs; src/workshop/store/{native.rs,web/wasm.rs}. `core` means `crates/nyon-workshop-core/src/`; exact nested file boundaries follow the responsibility names above. Register every new core module in `tests/living_wire.rs` and its declaration count.

**Interfaces:** Resident publication and durable commit have separate typed outcomes; save conflict does not roll back the resident pair.

- [ ] Read the spec's pinned source inputs and compare existing dirty work before editing these owned files.
- [ ] Write regression fixtures for: Crash before/after publication, dirty resident on CAS conflict, abandoned mutation/re-list, exact selected-branch Continue and read-only replay at capacity. Compile them without executing during the pause.
- [ ] Prepare and validate both owners before a single selector switch; persist using generation CAS, native flush/rename or one IndexedDB transaction.
- [ ] Compile affected Rust targets with explicit +1.99.1 and --locked; use ignored root target directories for separate tools. Capture exact exit status and warnings. Do not invoke test/CI wrappers.
- [ ] Review the full task diff for wire/ownership/policy errors; record Current/Partial/Proposed evidence and unresolved findings. No commit.
- [ ] Deferred acceptance, after Donald lifts the pause: execute the named regression scenarios and appropriate repository/platform gates. Compilation alone does not check this box.

## Compilation commands

Use `cargo +1.99.1 check --locked --workspace --all-targets --all-features` for native root/core changes. For each affected browser backend use `cargo +1.99.1 check --locked --target wasm32-unknown-unknown --lib --no-default-features --features webgpu-backend` and the same command with `webgl-backend`. Also compile core tests for wasm with `cargo +1.99.1 check --locked -p nyon-workshop-core --target wasm32-unknown-unknown --all-targets` when appropriate. Separate tools use explicit `--manifest-path` and `--target-dir target/rust-recovery/<tool>`.

## Plan self-review and handoff

Reviewed against the paired Rust design and preserved input revisions. Known source-review gaps are tasks/prerequisites, not waived acceptance. These plans preserve the complete requested game; later strategic subsystems explicitly require their own design/interface freeze rather than implementation guesses. No task authorizes tests during the pause, changing CI/deploy configuration, removing Zig source or claiming a completed full game.
