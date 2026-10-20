# NYON Rust Implemented Feature Parity Implementation Plan

> **For agentic workers:** Use superpowers:executing-plans task-by-task. The user's no-test/no-commit/canonical-checkout constraints override generic execution steps. Do not dispatch parallel writers.

**Goal:** Recover the implemented experimental capabilities in Rust while keeping the retained game usable and its existing identities unchanged.

**Architecture:** Use `nyon_workshop_core::recovery` as an isolated versioned API. Port semantics and canonical fields, not Zig memory representations. Reuse `src/game/simulation.rs` for existing integer navigation; no second Classic authority. Core exposes copied projections and typed commands; runtime owns policy and installation.

**Tech Stack:** Rust stable1.99.1, edition2024, existing Cargo workspace/dependencies and retained Rust platform.

**Spec:** `docs/superpowers/specs/2026-10-19-nyon-rust-implemented-feature-parity-design.md`. Read the spec and pinned historical input sections before each task.

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

### Task 1: Topology and admission

**Files:** core recovery/{mod,roles,surface}.rs; core lib.rs; tests/recovery_foundations.rs; tests/living_wire.rs. `core` means `crates/nyon-workshop-core/src/`; exact nested file boundaries follow the responsibility names above. Register every new core module in `tests/living_wire.rs` and its declaration count.

**Interfaces:** `Topology::new(u16) -> Result<Topology, SurfaceError>`, `cell_id(Cell) -> Result<u64, SurfaceError>`, `from_id(u64) -> Result<Cell, SurfaceError>`, `neighbor(Cell, Direction) -> Result<Cell, SurfaceError>`; `SessionPolicy::new(Role, Option<CivilizationId>)` and `admit(Provenance, Option<CivilizationId>)`.

- [ ] Read the spec's pinned source inputs and compare existing dirty work before editing these owned files.
- [ ] Write regression fixtures for: Maximum side/invalid ID, positive-Z seam literals, reciprocal corners, role escalation, invalid actor and autonomous injection. Compile them without executing during the pause.
- [ ] Port checked SessionPolicy and integer Topology, then add source-bound independent witnesses and register every new core module in CRATE_SOURCES.
- [ ] Compile affected Rust targets with explicit +1.99.1 and --locked; use ignored root target directories for separate tools. Capture exact exit status and warnings. Do not invoke test/CI wrappers.
- [ ] Review the full task diff for wire/ownership/policy errors; record Current/Partial/Proposed evidence and unresolved findings. No commit.
- [ ] Deferred acceptance, after Donald lifts the pause: execute the named regression scenarios and appropriate repository/platform gates. Compilation alone does not check this box.

### Task 2: Planet authority

**Files:** core recovery/planet_economy/{mod,types,state,command,phases,history,job,replay,view}.rs; tests/recovery_planet.rs. `core` means `crates/nyon-workshop-core/src/`; exact nested file boundaries follow the responsibility names above. Register every new core module in `tests/living_wire.rs` and its declaration count.

**Interfaces:** Versioned PlanetId/CellId/SettlementId/BuildingId/ShipmentId, Envelope, WorkBudget, Progress, ViewToken; Planet begin/poll/cancel/view and replay APIs. Exact field/tag contracts come from the preserved source inventory.

- [ ] Read the spec's pinned source inputs and compare existing dirty work before editing these owned files.
- [ ] Write regression fixtures for: All seven command families, path seams, reservation conservation, starvation/growth, publication boundary, history-full, OOM/rollback and exact receipt bytes. Compile them without executing during the pause.
- [ ] Translate types and wire grammar first, then safe typed COW pages, candidate admission/phase jobs, invariant checks, canonical receipts and replay. A canceled job drains its candidate and retains the published root.
- [ ] Compile affected Rust targets with explicit +1.99.1 and --locked; use ignored root target directories for separate tools. Capture exact exit status and warnings. Do not invoke test/CI wrappers.
- [ ] Review the full task diff for wire/ownership/policy errors; record Current/Partial/Proposed evidence and unresolved findings. No commit.
- [ ] Deferred acceptance, after Donald lifts the pause: execute the named regression scenarios and appropriate repository/platform gates. Compilation alone does not check this box.

### Task 3: Coordinated authority

**Files:** core recovery/coordinated_galaxy/; tests/recovery_coordinated.rs. `core` means `crates/nyon-workshop-core/src/`; exact nested file boundaries follow the responsibility names above. Register every new core module in `tests/living_wire.rs` and its declaration count.

**Interfaces:** Galaxy owns Planet backends; RoutedEnvelope, global progress and leased copied views use checked owner/generation IDs. No backend pointer crosses to callbacks.

- [ ] Read the spec's pinned source inputs and compare existing dirty work before editing these owned files.
- [ ] Write regression fixtures for: Two-world atomicity, shared-budget accounting, held lease/release, stale completion, fault shutdown and source-bound replay. Compile them without executing during the pause.
- [ ] Port qualified routing, shared lanes/allowance, lease ownership and histories before tick/replay. Prepare every planet then switch one descriptor; cleanup cannot publish part of a tick.
- [ ] Compile affected Rust targets with explicit +1.99.1 and --locked; use ignored root target directories for separate tools. Capture exact exit status and warnings. Do not invoke test/CI wrappers.
- [ ] Review the full task diff for wire/ownership/policy errors; record Current/Partial/Proposed evidence and unresolved findings. No commit.
- [ ] Deferred acceptance, after Donald lifts the pause: execute the named regression scenarios and appropriate repository/platform gates. Compilation alone does not check this box.

### Task 4: Seeded manifests

**Files:** core recovery/seeded_galaxy/; Rust seeded-galaxy CLI; tests/recovery_seeded.rs. `core` means `crates/nyon-workshop-core/src/`; exact nested file boundaries follow the responsibility names above. Register every new core module in `tests/living_wire.rs` and its declaration count.

**Interfaces:** Config/Manifest/Setup/Token/Progress and pure system/planet/civilization/distance functions. Systems1..64, planets1..512, civilizations1..16, side4..32.

- [ ] Read the spec's pinned source inputs and compare existing dirty work before editing these owned files.
- [ ] Write regression fixtures for: Small literal wire/digest d2453927e7ba740f64ade02f4a63175fcf45d343b32817df54bbba86e9ec72b1; maximum witnesses, malformed/noncanonical input, budget/cancellation and coordinated genesis equivalence. Compile them without executing during the pause.
- [ ] Port exact stateless formulas, NYSG codecs, staged Setup and owning Manifest. CLI uses this core, not a separate generator. Wire decoding recomputes every record against config.
- [ ] Compile affected Rust targets with explicit +1.99.1 and --locked; use ignored root target directories for separate tools. Capture exact exit status and warnings. Do not invoke test/CI wrappers.
- [ ] Review the full task diff for wire/ownership/policy errors; record Current/Partial/Proposed evidence and unresolved findings. No commit.
- [ ] Deferred acceptance, after Donald lifts the pause: execute the named regression scenarios and appropriate repository/platform gates. Compilation alone does not check this box.

### Task 5: Catalog and runtime integration

**Files:** core recovery/expanded_session/; src/workshop/store/; src/app/; tests/recovery_catalog.rs. `core` means `crates/nyon-workshop-core/src/`; exact nested file boundaries follow the responsibility names above. Register every new core module in `tests/living_wire.rs` and its declaration count.

**Interfaces:** Catalog, Selection, LedgerRecord, AllocationKind, Chain, LedgerBinding and detached MetadataMutation proposals; store owns durability and generation CAS.

- [ ] Read the spec's pinned source inputs and compare existing dirty work before editing these owned files.
- [ ] Write regression fixtures for: 128-byte labels, invalid UTF-8/control bytes, duplicate branch IDs, exact journal EOF, forged bindings, ID exhaustion, conflicting save and failed import preservation. Compile them without executing during the pause.
- [ ] Port catalog/ledger/metadata codecs first; add native/browser adapters and application installation only after authority parity. Use a new versioned namespace and keep active state on any failed load.
- [ ] Compile affected Rust targets with explicit +1.99.1 and --locked; use ignored root target directories for separate tools. Capture exact exit status and warnings. Do not invoke test/CI wrappers.
- [ ] Review the full task diff for wire/ownership/policy errors; record Current/Partial/Proposed evidence and unresolved findings. No commit.
- [ ] Deferred acceptance, after Donald lifts the pause: execute the named regression scenarios and appropriate repository/platform gates. Compilation alone does not check this box.

## Compilation commands

Use `cargo +1.99.1 check --locked --workspace --all-targets --all-features` for native root/core changes. For each affected browser backend use `cargo +1.99.1 check --locked --target wasm32-unknown-unknown --lib --no-default-features --features webgpu-backend` and the same command with `webgl-backend`. Also compile core tests for wasm with `cargo +1.99.1 check --locked -p nyon-workshop-core --target wasm32-unknown-unknown --all-targets` when appropriate. Separate tools use explicit `--manifest-path` and `--target-dir target/rust-recovery/<tool>`.

## Active execution record

- Decision/goals reconciliation and source inventory written.
- Initial topology/admission ports and regression targets written; native compilation passed, behavioral acceptance deferred.
- Remaining planet/coordinator/seeded/session/runtime ports are Pending.
- Donald requested subagent-driven execution and a game/UI/UX brainstorming pass on2026-10-19. New product decisions remain in discovery; this does not erase the parity-first objective.

## Plan self-review and handoff

Reviewed against the paired Rust design and preserved input revisions. Known source-review gaps are tasks/prerequisites, not waived acceptance. These plans preserve the complete requested game; later strategic subsystems explicitly require their own design/interface freeze rather than implementation guesses. No task authorizes tests during the pause, changing CI/deploy configuration, removing Zig source or claiming a completed full game.
